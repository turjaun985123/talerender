// TaleRender shadow-map material for the character card — Milestone 5.
//
// The engine gives the card plane a customDepthMaterial
// (toon.ts: `card.customDepthMaterial = new MeshDepthMaterial({
// depthPacking: RGBADepthPacking, map, alphaTest: 0.5 })`), so the
// card casts its cut-out silhouette into the key light's shadow map
// instead of the whole quad. This shader is that material: same
// packed-depth fragment as shader_depth.wgsl (three.js r170
// packing.glsl.js, MIT — see that file for provenance), gated by the
// map's alpha at the production alphaTest 0.5 (note: stricter than
// the card colour material's 0.04).

struct Uniforms {
    mvp: mat4x4f,
    model: mat4x4f,
};

@group(0) @binding(0) var<uniform> u: Uniforms;
@group(0) @binding(1) var tex: texture_2d<f32>;
@group(0) @binding(2) var smp: sampler;

struct Vout {
    @builtin(position) clip: vec4f,
    @location(0) uv: vec2f,
};

@vertex
fn vs_main(@location(0) pos: vec3f, @location(1) uv: vec2f) -> Vout {
    var o: Vout;
    o.clip = u.mvp * vec4f(pos, 1.0);
    o.uv = uv;
    return o;
}

fn modf_pos(x: f32, whole: ptr<function, f32>) -> f32 {
    let w = floor(x);
    *whole = w;
    return x - w;
}

fn pack_depth_to_rgba(v: f32) -> vec4f {
    if (v <= 0.0) {
        return vec4f(0.0, 0.0, 0.0, 0.0);
    }
    if (v >= 1.0) {
        return vec4f(1.0, 1.0, 1.0, 1.0);
    }
    var vuf: f32;
    let af = modf_pos(v * 16777216.0, &vuf);
    let bf = modf_pos(vuf * (1.0 / 256.0), &vuf);
    let gf = modf_pos(vuf * (1.0 / 256.0), &vuf);
    return vec4f(vuf * (1.0 / 255.0), gf * (256.0 / 255.0), bf * (256.0 / 255.0), af);
}

@fragment
fn fs_main(in: Vout) -> @location(0) vec4f {
    if (textureSample(tex, smp, in.uv).a < 0.5) {
        discard;
    }
    return pack_depth_to_rgba(in.clip.z);
}
