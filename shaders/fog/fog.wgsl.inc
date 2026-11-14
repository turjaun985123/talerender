// =====================================================================
// fog.wgsl.inc — three.js scene.fog for TaleRender (seat W28 package)
// =====================================================================
//
// A paste-ready transcription of three.js's fog chunks (vendored
// three.module.js, ShaderChunk; three.js is MIT-licensed):
//
//   fog_pars_fragment:  uniform vec3 fogColor;
//                       uniform float fogNear; uniform float fogFar;
//                       varying float vFogDepth;
//   fog_vertex:         vFogDepth = - mvPosition.z;
//   fog_fragment:       #ifdef FOG_EXP2
//                           float fogFactor = 1.0 - exp( - fogDensity *
//                               fogDensity * vFogDepth * vFogDepth );
//                       #else
//                           float fogFactor = smoothstep( fogNear,
//                               fogFar, vFogDepth );
//                       #endif
//                       gl_FragColor.rgb = mix( gl_FragColor.rgb,
//                           fogColor, fogFactor );
//
// CRITICAL: three's LINEAR fog factor is smoothstep(near, far, depth)
// — NOT the classic fixed-function ramp clamp((far-depth)/(far-near),
// 0, 1). The two curves differ by up to 0.078 in factor over the S06
// range (see FOG.md §2 for the table); a classic-ramp build is
// measurably wrong. WGSL smoothstep is the same Hermite polynomial
// (t = clamp((x-e0)/(e1-e0), 0, 1); t*t*(3-2t)) as GLSL smoothstep.
//
// Depth is the FORWARD-AXIS depth (-mvPosition.z in three), never the
// radial distance to the camera. fogDepth == dot(world - cam, fwd)
// for a lookAt camera, and == -view_pos.z from a view matrix; the two
// helpers below take the depth from whichever the host shader has.
//
// Fog runs at the END of the material fragment: after lighting and
// emissive, on the linear HDR colour, alpha untouched — i.e. before
// tone mapping / sRGB encode (the OutputPass) and before the Bokeh /
// Bloom passes, exactly as in three's composer chain. Do NOT apply
// fog in any post shader.
//
// Uniform contract (identical field names/semantics in both host
// shaders; colours are LINEAR working-space, i.e. sRGB-decoded):
//   fog:  vec4f  — rgb: fog colour (linear), w: near
//   fog2: vec4f  — x: far, y: enabled flag (0/1; 0 = skip entirely)
// The basic/card shader adds a per-draw flag (its material.fog):
//   misc.y > 0.5 means THIS draw honours scene fog (three's
//   material.fog property; sky dome + cloud sprites set it false).
//
// STATUS: semantically identical to the fog blocks the lead already
// wired inline in src/shader_pbr.wgsl (fs_main tail) and
// src/shader_basic.wgsl (fs_main). This include is the shared,
// testable form of that code: replacing the inline blocks with these
// calls must not change a single pixel. See FOG.md for the full
// integration spec, values, and verification plan.
// =====================================================================

// --- the factor ------------------------------------------------------

// Linear Fog branch of fog_fragment (the only branch this film uses:
// every fogged shot builds THREE.Fog; FogExp2 appears nowhere in
// env.ts / shots.ts).
fn tr_fog_factor(view_depth: f32, fog_near: f32, fog_far: f32) -> f32 {
    return smoothstep(fog_near, fog_far, view_depth);
}

// FOG_EXP2 branch of fog_fragment, for completeness only. NOT used by
// the Onoda film; wire it only if a future staging builds FogExp2
// (it would need a density uniform in place of near/far).
fn tr_fog_factor_exp2(view_depth: f32, fog_density: f32) -> f32 {
    return 1.0 - exp(-fog_density * fog_density * view_depth * view_depth);
}

// fog_fragment's mix, rgb only (alpha is never fogged).
fn tr_fog_mix(lit_rgb: vec3f, fog_rgb: vec3f, factor: f32) -> vec3f {
    return mix(lit_rgb, fog_rgb, factor);
}

// --- PBR host form (src/shader_pbr.wgsl) -----------------------------
// Depth from the frame uniforms, as the PBR shader does: the signed
// distance from the camera along its forward axis. Call at the end of
// fs_main, on the fully lit colour (lighting + IBL + emissive):
//
//     var out_col = outgoing;
//     out_col = tr_fog_apply_frame(out_col, in.world_pos);
//     return vec4f(out_col, 1.0);
//
// Requires the host's `frame` uniform to carry fog / fog2 / cam_fwd
// with the contract above. When frame.fog2.y == 0 the colour is
// returned untouched (the no-fog path stays bit-identical).
fn tr_fog_apply_frame(lit_rgb: vec3f, world_pos: vec3f,
                      cam_pos: vec3f, cam_fwd: vec3f,
                      fog: vec4f, fog2: vec4f) -> vec3f {
    if (fog2.y > 0.5) {
        let fdepth = dot(world_pos - cam_pos, cam_fwd);
        let ffactor = tr_fog_factor(fdepth, fog.w, fog2.x);
        return tr_fog_mix(lit_rgb, fog.rgb, ffactor);
    }
    return lit_rgb;
}

// --- basic / card host form (src/shader_basic.wgsl) ------------------
// Depth is -view_pos.z, three's vFogDepth verbatim (the host vertex
// stage already computes view_pos from its model-view matrix). Call
// in fs_main AFTER the alphaTest discard, on the tinted texel:
//
//     rgba = tr_fog_apply_basic(rgba, in.view_pos);
//
// Honours BOTH gates, matching three: the scene must have fog
// (fog2.y, i.e. scene.fog != null) AND this draw's material must not
// opt out (per_draw_fog == material.fog: cards + blob shadow true;
// sky dome, cloud sprites, caption overlays false).
fn tr_fog_apply_basic(rgba: vec4f, view_pos: vec3f,
                      per_draw_fog: bool,
                      fog: vec4f, fog2: vec4f) -> vec4f {
    if (per_draw_fog && fog2.y > 0.5) {
        let ffactor = tr_fog_factor(-view_pos.z, fog.w, fog2.x);
        return vec4f(tr_fog_mix(rgba.rgb, fog.rgb, ffactor), rgba.a);
    }
    return rgba;
}
