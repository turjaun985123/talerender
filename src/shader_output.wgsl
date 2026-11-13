// TaleRender output stage for the DoF chart — transcriptions of the two
// three.js r170 OutputPass steps (MIT-licensed, vendored
// three.module.js, ShaderChunk/tonemapping.glsl.js +
// ShaderChunk/colorspace.glsl.js), so the chart comparison isolates the
// bokeh pass instead of confounding it with tone mapping:
//
//   1. NeutralToneMapping with toneMappingExposure = 1.0 (the engine's
//      per-frame default when the look sets exposure: 1).
//   2. sRGBTransferOETF (linear -> sRGB), the renderer's output colour
//      space conversion.
//
// In the reference chain these run in the OutputPass; the GRADE pass
// after them is an exact identity for the chart's look (vignette 0,
// grain 0, warm 0, sat 1), and UnrealBloomPass at strength 0 adds
// exactly 0, so neither is reproduced here.

@group(0) @binding(0) var t_color: texture_2d<f32>;
@group(0) @binding(1) var s_color: sampler;

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
    o.uv = vec2f(p.x * 0.5 + 0.5, 0.5 - p.y * 0.5);
    return o;
}

// vec3 NeutralToneMapping( vec3 color ) — transcribed line for line.
fn neutral_tone_mapping(color_in: vec3f) -> vec3f {
    var color = color_in;
    color *= 1.0; // toneMappingExposure (engine look exposure: 1)
    let start_compression = 0.8 - 0.04;
    let desaturation = 0.15;
    let x = min(color.r, min(color.g, color.b));
    var offset = 0.04;
    if (x < 0.08) {
        offset = x - 6.25 * x * x;
    }
    color -= offset;
    let peak = max(color.r, max(color.g, color.b));
    if (peak < start_compression) {
        return color;
    }
    let d = 1.0 - start_compression;
    let new_peak = 1.0 - d * d / (peak + d - start_compression);
    color *= new_peak / peak;
    let g = 1.0 - 1.0 / (desaturation * (peak - new_peak) + 1.0);
    return mix(color, vec3f(new_peak), g);
}

// vec4 sRGBTransferOETF( in vec4 value ) — transcribed line for line
// (note the 0.41666 exponent: it is three's constant, not 1/2.4).
fn srgb_transfer_oetf(value: vec4f) -> vec4f {
    let hi = pow(value.rgb, vec3f(0.41666)) * 1.055 - vec3f(0.055);
    let lo = value.rgb * 12.92;
    return vec4f(select(hi, lo, value.rgb <= vec3f(0.0031308)), value.a);
}

@fragment
fn fs_main(in: Vout) -> @location(0) vec4f {
    var c = textureSample(t_color, s_color, in.uv);
    c = vec4f(neutral_tone_mapping(c.rgb), c.a);
    return srgb_transfer_oetf(c);
}
