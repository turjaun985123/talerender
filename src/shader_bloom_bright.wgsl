// TaleRender bloom bright pass — Milestone 5.
//
// Transcription of three.js r170's LuminosityHighPassShader (vendored
// addons/postprocessing/UnrealBloomPass.js + shaders/
// LuminosityHighPassShader.js, MIT), the first stage of
// UnrealBloomPass: alpha = smoothstep(threshold, threshold +
// smoothWidth, luminance(colour)); out = mix(black, texel, alpha).
// smoothWidth is 0.01 in the shipped shader; luminance() is the
// WebGLProgram prefix function over the working (linear) space,
// weights vec3(0.2126, 0.7152, 0.0722).

struct U {
    threshold: vec4f, // x: threshold
};

@group(0) @binding(0) var<uniform> u: U;
@group(0) @binding(1) var tex: texture_2d<f32>;
@group(0) @binding(2) var smp: sampler;

struct Vout {
    @builtin(position) clip: vec4f,
    @location(0) uv: vec2f,
};

@vertex
fn vs_main(@builtin(vertex_index) i: u32) -> Vout {
    var o: Vout;
    let x = f32((i << 1u) & 2u);
    let y = f32(i & 2u);
    o.uv = vec2f(x, y);
    o.clip = vec4f(x * 2.0 - 1.0, 1.0 - y * 2.0, 0.0, 1.0);
    return o;
}

fn luminance(c: vec3f) -> f32 {
    return dot(vec3f(0.2126, 0.7152, 0.0722), c);
}

@fragment
fn fs_main(in: Vout) -> @location(0) vec4f {
    let texel = textureSample(tex, smp, in.uv);
    let alpha = smoothstep(u.threshold.x, u.threshold.x + 0.01, luminance(texel.rgb));
    return mix(vec4f(0.0), texel, alpha);
}
