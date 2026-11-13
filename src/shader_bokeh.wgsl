// TaleRender bokeh composite — a line-for-line WGSL transcription of the
// three.js r170 BokehShader fragment shader
// (template/vendor/addons/shaders/BokehShader.js in the Tall Tale skill,
// three.js is MIT-licensed; the shader itself is credited there as
// "ported from GLSL shader by Martins Upitis").
//
// Compatibility contract §1.1: the arithmetic is TRANSCRIBED, not
// improved. The circle-of-confusion is `factor = focus + viewZ` (viewZ is
// negative), scaled by aperture and clamped to ±maxblur; the blur is a
// fixed 41-tap gather whose offsets are the subject's OWN depth only —
// there is no per-tap depth test, which is exactly the shader's
// well-known foreground/background bleed behaviour the house look is
// art-directed around.
//
// TRANSCRIPTION MAP (GLSL -> this file):
//   getDepth()      : unpackRGBAToDepth(texture2D(tDepth, uv)) with
//                     DEPTH_PACKING == 1 -> unpack_rgba_to_depth() below,
//                     dot(v, UnpackFactors4), UnpackFactors4 =
//                     (UnpackDownscale / PackFactors.rgb, 1/PackFactors.a),
//                     UnpackDownscale = 255/256, PackFactors =
//                     (1, 256, 65536, 16777216)  [packing.glsl.js, r170]
//   getViewZ()      : perspectiveDepthToViewZ(depth, near, far) with
//                     PERSPECTIVE_CAMERA == 1 ->
//                     (near * far) / ((far - near) * depth - far)
//   aspectcorrect   : vec2(1.0, aspect) — note aspect = width/height
//   dofblur{,9,7,4} : clamp(factor * aperture, -maxblur, maxblur) and the
//                     0.9 / 0.7 / 0.4 ring scales
//   taps            : centre + 16 @ 1.0 + 8 @ 0.9 + 8 @ 0.7 + 8 @ 0.4,
//                     same offsets, same order, divisor 41.0
//   output          : col / 41.0, alpha forced to 1.0
//
// Orientation note: three's vUv has (0,0) at the image's bottom-left;
// wgpu texture coordinates have (0,0) at the top-left texel. The same
// uv is used for the colour and depth textures and the output lands on
// the same pixel, so the composite is internally consistent in either
// convention; and the 41-tap set is symmetric under u -> -u and
// v -> -v (every offset (x, y) appears with (x, -y), (-x, y) and
// (-x, -y)), so the mirrored vertical axis cannot change the result.

struct BokehUniforms {
    focus: f32,
    aspect: f32,
    aperture: f32,
    maxblur: f32,
    near_clip: f32,
    far_clip: f32,
    _pad0: f32,
    _pad1: f32,
};

@group(0) @binding(0) var<uniform> u: BokehUniforms;
@group(0) @binding(1) var t_color: texture_2d<f32>;
@group(0) @binding(2) var s_color: sampler;
@group(0) @binding(3) var t_depth: texture_2d<f32>;
@group(0) @binding(4) var s_depth: sampler;

struct Vout {
    @builtin(position) clip: vec4f,
    @location(0) uv: vec2f,
};

@vertex
fn vs_main(@builtin(vertex_index) i: u32) -> Vout {
    var p = vec2f(-1.0, -1.0);
    if (i == 1u) {
        p = vec2f(3.0, -1.0);
    }
    if (i == 2u) {
        p = vec2f(-1.0, 3.0);
    }
    var o: Vout;
    o.clip = vec4f(p, 0.0, 1.0);
    o.uv = vec2f(p.x * 0.5 + 0.5, 0.5 - p.y * 0.5);
    return o;
}

fn unpack_rgba_to_depth(v: vec4f) -> f32 {
    // UnpackFactors4, expanded from the packing chunk constants.
    let unpack_downscale = 255.0 / 256.0;
    let factors = vec4f(
        unpack_downscale / 1.0,
        unpack_downscale / 256.0,
        unpack_downscale / 65536.0,
        1.0 / 16777216.0,
    );
    return dot(v, factors);
}

fn get_depth(uv: vec2f) -> f32 {
    return unpack_rgba_to_depth(textureSample(t_depth, s_depth, uv));
}

fn get_view_z(depth: f32) -> f32 {
    // perspectiveDepthToViewZ( depth, nearClip, farClip )
    return (u.near_clip * u.far_clip) / ((u.far_clip - u.near_clip) * depth - u.far_clip);
}

@fragment
fn fs_main(in: Vout) -> @location(0) vec4f {
    let aspectcorrect = vec2f(1.0, u.aspect);

    let view_z = get_view_z(get_depth(in.uv));

    let factor = (u.focus + view_z); // viewZ is <= 0, so this is a difference equation

    let dofblur = vec2f(clamp(factor * u.aperture, -u.maxblur, u.maxblur));

    let dofblur9 = dofblur * 0.9;
    let dofblur7 = dofblur * 0.7;
    let dofblur4 = dofblur * 0.4;

    var col = vec4f(0.0);

    col += textureSample(t_color, s_color, in.uv);
    col += textureSample(t_color, s_color, in.uv + (vec2f(0.0, 0.4) * aspectcorrect) * dofblur);
    col += textureSample(t_color, s_color, in.uv + (vec2f(0.15, 0.37) * aspectcorrect) * dofblur);
    col += textureSample(t_color, s_color, in.uv + (vec2f(0.29, 0.29) * aspectcorrect) * dofblur);
    col += textureSample(t_color, s_color, in.uv + (vec2f(-0.37, 0.15) * aspectcorrect) * dofblur);
    col += textureSample(t_color, s_color, in.uv + (vec2f(0.40, 0.0) * aspectcorrect) * dofblur);
    col += textureSample(t_color, s_color, in.uv + (vec2f(0.37, -0.15) * aspectcorrect) * dofblur);
    col += textureSample(t_color, s_color, in.uv + (vec2f(0.29, -0.29) * aspectcorrect) * dofblur);
    col += textureSample(t_color, s_color, in.uv + (vec2f(-0.15, -0.37) * aspectcorrect) * dofblur);
    col += textureSample(t_color, s_color, in.uv + (vec2f(0.0, -0.4) * aspectcorrect) * dofblur);
    col += textureSample(t_color, s_color, in.uv + (vec2f(-0.15, 0.37) * aspectcorrect) * dofblur);
    col += textureSample(t_color, s_color, in.uv + (vec2f(-0.29, 0.29) * aspectcorrect) * dofblur);
    col += textureSample(t_color, s_color, in.uv + (vec2f(0.37, 0.15) * aspectcorrect) * dofblur);
    col += textureSample(t_color, s_color, in.uv + (vec2f(-0.4, 0.0) * aspectcorrect) * dofblur);
    col += textureSample(t_color, s_color, in.uv + (vec2f(-0.37, -0.15) * aspectcorrect) * dofblur);
    col += textureSample(t_color, s_color, in.uv + (vec2f(-0.29, -0.29) * aspectcorrect) * dofblur);
    col += textureSample(t_color, s_color, in.uv + (vec2f(0.15, -0.37) * aspectcorrect) * dofblur);

    col += textureSample(t_color, s_color, in.uv + (vec2f(0.15, 0.37) * aspectcorrect) * dofblur9);
    col += textureSample(t_color, s_color, in.uv + (vec2f(-0.37, 0.15) * aspectcorrect) * dofblur9);
    col += textureSample(t_color, s_color, in.uv + (vec2f(0.37, -0.15) * aspectcorrect) * dofblur9);
    col += textureSample(t_color, s_color, in.uv + (vec2f(-0.15, -0.37) * aspectcorrect) * dofblur9);
    col += textureSample(t_color, s_color, in.uv + (vec2f(-0.15, 0.37) * aspectcorrect) * dofblur9);
    col += textureSample(t_color, s_color, in.uv + (vec2f(0.37, 0.15) * aspectcorrect) * dofblur9);
    col += textureSample(t_color, s_color, in.uv + (vec2f(-0.37, -0.15) * aspectcorrect) * dofblur9);
    col += textureSample(t_color, s_color, in.uv + (vec2f(0.15, -0.37) * aspectcorrect) * dofblur9);

    col += textureSample(t_color, s_color, in.uv + (vec2f(0.29, 0.29) * aspectcorrect) * dofblur7);
    col += textureSample(t_color, s_color, in.uv + (vec2f(0.40, 0.0) * aspectcorrect) * dofblur7);
    col += textureSample(t_color, s_color, in.uv + (vec2f(0.29, -0.29) * aspectcorrect) * dofblur7);
    col += textureSample(t_color, s_color, in.uv + (vec2f(0.0, -0.4) * aspectcorrect) * dofblur7);
    col += textureSample(t_color, s_color, in.uv + (vec2f(-0.29, 0.29) * aspectcorrect) * dofblur7);
    col += textureSample(t_color, s_color, in.uv + (vec2f(-0.4, 0.0) * aspectcorrect) * dofblur7);
    col += textureSample(t_color, s_color, in.uv + (vec2f(-0.29, -0.29) * aspectcorrect) * dofblur7);
    col += textureSample(t_color, s_color, in.uv + (vec2f(0.0, 0.4) * aspectcorrect) * dofblur7);

    col += textureSample(t_color, s_color, in.uv + (vec2f(0.29, 0.29) * aspectcorrect) * dofblur4);
    col += textureSample(t_color, s_color, in.uv + (vec2f(0.4, 0.0) * aspectcorrect) * dofblur4);
    col += textureSample(t_color, s_color, in.uv + (vec2f(0.29, -0.29) * aspectcorrect) * dofblur4);
    col += textureSample(t_color, s_color, in.uv + (vec2f(0.0, -0.4) * aspectcorrect) * dofblur4);
    col += textureSample(t_color, s_color, in.uv + (vec2f(-0.29, 0.29) * aspectcorrect) * dofblur4);
    col += textureSample(t_color, s_color, in.uv + (vec2f(-0.4, 0.0) * aspectcorrect) * dofblur4);
    col += textureSample(t_color, s_color, in.uv + (vec2f(-0.29, -0.29) * aspectcorrect) * dofblur4);
    col += textureSample(t_color, s_color, in.uv + (vec2f(0.0, 0.4) * aspectcorrect) * dofblur4);

    col = col / 41.0;
    col.a = 1.0;
    return col;
}
