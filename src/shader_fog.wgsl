// TaleRender scene-file fog pass (format v2 `fog` block):
// three.js fog_fragment, linear Fog branch, applied to the
// resolved HDR scene BEFORE bokeh/output — the same point in
// the chain where the source's material shaders apply it
// (pre-tonemap, linear space):
//   factor = smoothstep(near, far, axisDepth)
//   rgb    = mix(litLinear, fogColorLinear, factor)
// axisDepth = -viewZ, recovered from the packed depth prepass
// (clip.z) via three's perspectiveDepthToViewZ. Pixels the
// prepass cleared (unpacked depth > 1) yield a negative axis
// depth and take factor 0 — the clear/background stays unfogged
// (the fog chart clears TO the fog colour, so both readings of
// the background agree there by construction).

struct U {
    fog_color: vec4f, // rgb: fog colour (linear), w: fog near
    params: vec4f,    // x: fog far, y: camera near, z: camera far
};

@group(0) @binding(0) var<uniform> u: U;
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
    o.uv = p * vec2f(0.5, -0.5) + vec2f(0.5);
    return o;
}

fn unpack_rgba_to_depth(v: vec4f) -> f32 {
    // Identical to shader_bokeh.wgsl's unpack (the packed-depth
    // prepass format is shared).
    let unpack_downscale = 255.0 / 256.0;
    let factors = vec4f(
        unpack_downscale / 1.0,
        unpack_downscale / 256.0,
        unpack_downscale / 65536.0,
        1.0 / 16777216.0,
    );
    return dot(v, factors);
}

@fragment
fn fs_main(in: Vout) -> @location(0) vec4f {
    let col = textureSample(t_color, s_color, in.uv);
    let depth = unpack_rgba_to_depth(textureSample(t_depth, s_depth, in.uv));
    let near = u.params.y;
    let far = u.params.z;
    // three.js perspectiveDepthToViewZ (reversed depth off).
    let view_z = (near * far) / ((far - near) * depth - far);
    let axis_depth = -view_z;
    let f = smoothstep(u.fog_color.w, u.params.x, axis_depth);
    return vec4f(mix(col.rgb, u.fog_color.rgb, f), col.a);
}
