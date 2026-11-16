@group(0) @binding(0) var tex: texture_2d<f32>;
@group(0) @binding(1) var samp: sampler;
struct Vout { @builtin(position) clip: vec4f, @location(0) uv: vec2f };
@vertex fn vs_main(@builtin(vertex_index) vi: u32) -> Vout {
    var o: Vout;
    let x = f32(i32(vi & 1u)) * 4.0 - 1.0;
    let y = f32(i32((vi >> 1u) & 1u)) * 4.0 - 1.0;
    o.clip = vec4f(x, y, 0.0, 1.0);
    o.uv = vec2f((x + 1.0) * 0.5, 1.0 - (y + 1.0) * 0.5);
    return o;
}
@fragment fn fs_main(in: Vout) -> @location(0) vec4f {
    return textureSample(tex, samp, in.uv);
}
