// TaleRender depth prepass, position-only variant for the
// scene-format loader: the fragment arithmetic is IDENTICAL to
// shader_depth.wgsl (the transcription of three.js r170's
// MeshDepthMaterial output with RGBADepthPacking — see that file for
// the provenance and the packing constants). Only the vertex input
// differs: this variant reads positions from a stride-12 buffer so
// one depth pipeline serves both vertex formats the loader builds
// (colour geometry carries normal/colour after the position; textured
// geometry carries uv — neither affects packed depth, which depends
// on clip z alone).

struct Uniforms {
    mvp: mat4x4f,
    model: mat4x4f,
};

@group(0) @binding(0) var<uniform> u: Uniforms;

struct Vin {
    @location(0) pos: vec3f,
};

struct Vout {
    @builtin(position) clip: vec4f,
};

@vertex
fn vs_main(v: Vin) -> Vout {
    var o: Vout;
    o.clip = u.mvp * vec4f(v.pos, 1.0);
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
    return pack_depth_to_rgba(in.clip.z);
}
