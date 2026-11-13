// TaleRender depth prepass — transcription of three.js r170's
// MeshDepthMaterial fragment output with RGBADepthPacking (3201), as used
// by BokehPass (template/vendor/addons/postprocessing/BokehPass.js):
// the scene is rendered a second time with an override material that
// writes packDepthToRGBA(fragCoordZ) instead of shading.
//
// three.js is MIT-licensed (LICENSE-three.txt in the skill's vendor
// tree); the packing arithmetic below is transcribed from
// ShaderChunk/packing.glsl.js in the vendored three.module.js (r170),
// constants and all:
//
//   const float PackUpscale    = 256. / 255.;
//   const float ShiftRight8    = 1. / 256.;
//   const float Inv255         = 1. / 255.;
//   const vec4  PackFactors    = vec4( 1.0, 256.0, 65536.0, 16777216.0 );
//
//   vec4 packDepthToRGBA( const in float v ) {
//       if( v <= 0.0 ) return vec4( 0., 0., 0., 0. );
//       if( v >= 1.0 ) return vec4( 1., 1., 1., 1. );
//       float vuf;
//       float af = modf( v * PackFactors.a, vuf );
//       float bf = modf( vuf * ShiftRight8, vuf );
//       float gf = modf( vuf * ShiftRight8, vuf );
//       return vec4( vuf * Inv255, gf * PackUpscale, bf * PackUpscale, af );
//   }
//
// fragCoordZ in three is 0.5 * vHighPrecisionZW[0] / vHighPrecisionZW[1] + 0.5
// where the ratio is WebGL NDC z in [-1, 1]. wgpu's @builtin(position).z is
// already the [0, 1] depth, i.e. exactly fragCoordZ. GLSL modf(x, i) for
// x >= 0 returns x - floor(x) and sets i = floor(x); every value fed here
// is >= 0, so the helper below is the same function on this domain.

struct Uniforms {
    mvp: mat4x4f,
    model: mat4x4f,
};

@group(0) @binding(0) var<uniform> u: Uniforms;

struct Vin {
    @location(0) pos: vec3f,
    @location(1) uv: vec2f,
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
