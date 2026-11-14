// TaleRender unlit textured shader — Milestone 5.
//
// The native counterpart of three.js MeshBasicMaterial / SpriteMaterial
// as the composer chain uses them: colour = map * colour, sampled in
// linear space (textures upload through sRGB views), alphaTest discard
// (alphatest_fragment: `if ( diffuseColor.a < alphaTest ) discard;`),
// opacity folded into alpha. In the composer chain materials do NOT
// tone-map (OutputPass owns tone mapping + sRGB encode), so this
// pass writes linear HDR straight through — the character card's
// toneMapped:false is moot here (engine.ts renders it into the same
// HalfFloat target as everything else).
//
// Used for the sky dome (MeshBasicMaterial over the gradient texture),
// the cloud sprites, the blob shadow, the character card, and — with a
// raw (non-sRGB) source texture and no depth — the post-GRADE caption
// overlay, where the strip's stored bytes ARE the final sRGB values.

struct DrawU {
    mvp: mat4x4f,
    tint: vec4f, // rgb linear tint, a: opacity
    misc: vec4f, // x: alpha_test, y: per-draw fog flag (0/1)
    // Integration extension (film mode): model-view matrix (for
    // the view-depth fog factor) + the scene fog state. The ab1
    // path writes fog2.y = 0 (disabled) and renders as before.
    mv: mat4x4f,
    fog: vec4f, // rgb: fog colour (linear), w: near
    fog2: vec4f, // x: far, y: enabled (0/1)
};

@group(0) @binding(0) var<uniform> draw: DrawU;
@group(0) @binding(1) var tex: texture_2d<f32>;
@group(0) @binding(2) var smp: sampler;

struct Vout {
    @builtin(position) clip: vec4f,
    @location(0) uv: vec2f,
    @location(1) view_pos: vec3f,
};

@vertex
fn vs_main(@location(0) pos: vec3f, @location(1) uv: vec2f) -> Vout {
    var o: Vout;
    o.clip = draw.mvp * vec4f(pos, 1.0);
    o.uv = uv;
    o.view_pos = (draw.mv * vec4f(pos, 1.0)).xyz;
    return o;
}

@fragment
fn fs_main(in: Vout) -> @location(0) vec4f {
    let c = textureSample(tex, smp, in.uv);
    // diffuseColor = vec4(diffuse, opacity) * map in three, and the
    // alphaTest comparison runs on that product.
    var rgba = vec4f(c.rgb * draw.tint.rgb, c.a * draw.tint.a);
    if (rgba.a < draw.misc.x) {
        discard;
    }
    // fog_fragment (film mode only; both flags are 0 on the ab1
    // path): factor over view depth, three's smoothstep form.
    if (draw.misc.y > 0.5 && draw.fog2.y > 0.5) {
        let ffactor = smoothstep(draw.fog.w, draw.fog2.x, -in.view_pos.z);
        rgba = vec4f(mix(rgba.rgb, draw.fog.rgb, ffactor), rgba.a);
    }
    return rgba;
}
