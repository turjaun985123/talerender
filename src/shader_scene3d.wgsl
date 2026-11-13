// TaleRender scene-format colour shader: shader3d.wgsl with the fixed
// house light promoted to a uniform, so a scene file's "lighting"
// block is real data the loader passes in, not a compile-time
// constant. With the house values (direction 0.5/0.8/0.6, ambient
// 0.35 — the defaults every hand-authored scene ships) the fragment
// arithmetic is operation-identical to shader3d.wgsl: same normalize,
// same diffuse term, same ambient mix. The hand-authored scenes keep
// the constant version; nothing about their rendering changes.

struct Uniforms {
    mvp: mat4x4f,
    model: mat4x4f,
    // xyz: light direction (unnormalised; the shader normalises,
    // exactly as shader3d.wgsl normalises its constant). w: ambient.
    light: vec4f,
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
    let light_dir = normalize(u.light.xyz);
    let diffuse = max(dot(normalize(in.nrm), light_dir), 0.0);
    let ambient = u.light.w;
    return vec4f(in.col * (ambient + (1.0 - ambient) * diffuse), 1.0);
}
