// TaleRender GRADE pass — Milestone 5.
//
// Verbatim transcription of the engine's custom grade shader
// (template/src/core/engine.ts, the ShaderPass after OutputPass):
//
//   uniform float uVig, uGrain, uTime, uWarm, uSat;
//   float h(vec2 p){ return fract(sin(dot(p, vec2(12.9898, 78.233))
//                                + uTime * 7.13) * 43758.5453); }
//   void main(){
//     vec4 c = texture2D(tDiffuse, vUv);
//     vec2 d = vUv - 0.5; d.x *= 0.5625;
//     c.rgb *= 1.0 - uVig * smoothstep(0.25, 0.75, length(d) * 1.4);
//     c.rgb += (h(vUv * 1000.0) - 0.5) * uGrain;
//     c.r += uWarm * 0.5; c.b -= uWarm * 0.5;
//     float l = dot(c.rgb, vec3(0.299, 0.587, 0.114));
//     c.rgb = mix(vec3(l), c.rgb, uSat);
//     gl_FragColor = c;
//   }
//
// It runs on the sRGB-encoded output of OutputPass (the engine feeds
// it the composer's final buffer), so this pass samples and writes
// raw Rgba8Unorm values — no colour-space conversion either side.
// The grain hash is a function of (uv, uTime) only: uTime is the
// shot time t, so a given frame index yields the same grain field on
// every run (the sin() itself may round differently across GPU
// vendors — the compatibility contract flags this; amplitude here is
// uGrain/2, i.e. sub-LSB for production settings).

struct U {
    // x: uVig, y: uGrain, z: uWarm, w: uSat
    params: vec4f,
    // x: uTime
    time: vec4f,
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

fn h(p: vec2f) -> f32 {
    return fract(sin(dot(p, vec2f(12.9898, 78.233)) + u.time.x * 7.13) * 43758.5453);
}

@fragment
fn fs_main(in: Vout) -> @location(0) vec4f {
    var c = textureSample(tex, smp, in.uv);
    var d = in.uv - 0.5;
    d.x *= 0.5625;
    c = vec4f(c.rgb * (1.0 - u.params.x * smoothstep(0.25, 0.75, length(d) * 1.4)), c.a);
    c = vec4f(c.rgb + (h(in.uv * 1000.0) - 0.5) * u.params.y, c.a);
    c = vec4f(c.r + u.params.z * 0.5, c.g, c.b - u.params.z * 0.5, c.a);
    let l = dot(c.rgb, vec3f(0.299, 0.587, 0.114));
    c = vec4f(mix(vec3f(l), c.rgb, u.params.w), c.a);
    return c;
}
