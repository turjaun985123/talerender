// TaleRender Milestone 1 textured-quad shader: the stand-in for the
// CanvasTexture character plane. UV-mapped texture sampled with a
// linear sampler; no lighting yet (the real character plane blends
// into the scene through the composer in later milestones).

struct Uniforms {
    mvp: mat4x4f,
    model: mat4x4f,
};

@group(0) @binding(0) var<uniform> u: Uniforms;
@group(0) @binding(1) var tex: texture_2d<f32>;
@group(0) @binding(2) var samp: sampler;

struct Vin {
    @location(0) pos: vec3f,
    @location(1) uv: vec2f,
};

struct Vout {
    @builtin(position) clip: vec4f,
    @location(0) uv: vec2f,
};

@vertex
fn vs_main(v: Vin) -> Vout {
    var o: Vout;
    o.clip = u.mvp * vec4f(v.pos, 1.0);
    o.uv = v.uv;
    return o;
}

@fragment
fn fs_main(in: Vout) -> @location(0) vec4f {
    return textureSample(tex, samp, in.uv);
}
