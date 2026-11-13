// TaleRender bloom composite — Milestone 5.
//
// Transcription of three.js r170's UnrealBloomPass composite material
// (vendored addons/postprocessing/UnrealBloomPass.js, MIT):
//   bloom = strength * sum_i( lerpBloomFactor(bloomFactors[i], radius)
//                              * bloomTintColors[i] * blurTexture_i )
// with bloomFactors = [1.0, 0.8, 0.6, 0.4, 0.2], bloomTintColors all
// (1,1,1), and lerpBloomFactor(f) = mix(f, 1.2 - f, radius). The pass
// renders this into renderTargetsHorizontal[0] in three; its output is
// then drawn over the read buffer with AdditiveBlending (CopyShader)
// by the caller — this module computes only the composite value.

struct U {
    // x: strength, y: radius
    params: vec4f,
};

@group(0) @binding(0) var<uniform> u: U;
@group(0) @binding(1) var tex0: texture_2d<f32>;
@group(0) @binding(2) var tex1: texture_2d<f32>;
@group(0) @binding(3) var tex2: texture_2d<f32>;
@group(0) @binding(4) var tex3: texture_2d<f32>;
@group(0) @binding(5) var tex4: texture_2d<f32>;
@group(0) @binding(6) var smp: sampler;

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

fn lerp_bloom_factor(factor: f32) -> f32 {
    return mix(factor, 1.2 - factor, u.params.y);
}

@fragment
fn fs_main(in: Vout) -> @location(0) vec4f {
    // The GLSL is vec4 arithmetic throughout: the alpha channel ends
    // up as strength * sum(factor_i * blurAlpha_i), and three's
    // AdditiveBlending (blendSrc = SrcAlpha) makes that alpha scale
    // the rgb contribution of the blend pass — replicated, not trimmed.
    var bloom = vec4f(0.0);
    bloom += textureSample(tex0, smp, in.uv) * lerp_bloom_factor(1.0);
    bloom += textureSample(tex1, smp, in.uv) * lerp_bloom_factor(0.8);
    bloom += textureSample(tex2, smp, in.uv) * lerp_bloom_factor(0.6);
    bloom += textureSample(tex3, smp, in.uv) * lerp_bloom_factor(0.4);
    bloom += textureSample(tex4, smp, in.uv) * lerp_bloom_factor(0.2);
    return bloom * u.params.x;
}
