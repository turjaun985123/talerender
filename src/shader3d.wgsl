// TaleRender Milestone 1 geometry shader: per-vertex colour with a
// single fixed directional light (diffuse + ambient). The light is a
// compile-time constant on purpose at this stage — real light objects,
// shadow mapping and the composer chain are later milestones, and this
// file must not pretend otherwise.

struct Uniforms {
    mvp: mat4x4f,
    model: mat4x4f,
};

@group(0) @binding(0) var<uniform> u: Uniforms;

struct Vin {
    @location(0) pos: vec3f,
    @location(1) nrm: vec3f,
    @location(2) col: vec3f,
};

struct Vout {
    @builtin(position) clip: vec4f,
    @location(0) col: vec3f,
    @location(1) nrm: vec3f,
};

@vertex
fn vs_main(v: Vin) -> Vout {
    var o: Vout;
    o.clip = u.mvp * vec4f(v.pos, 1.0);
    o.nrm = normalize((u.model * vec4f(v.nrm, 0.0)).xyz);
    o.col = v.col;
    return o;
}

@fragment
fn fs_main(in: Vout) -> @location(0) vec4f {
    let light_dir = normalize(vec3f(0.5, 0.8, 0.6));
    let diffuse = max(dot(normalize(in.nrm), light_dir), 0.0);
    let ambient = 0.35;
    return vec4f(in.col * (ambient + (1.0 - ambient) * diffuse), 1.0);
}
