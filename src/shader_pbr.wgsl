// TaleRender PBR scene shader — Milestone 5 (composer).
//
// A transcription of three.js r170's MeshStandardMaterial lighting for
// the exact light rig the Tall Tale engine builds (template/src/core/
// engine.ts lightRig() + world.ts sunlight() + shots.ts fire), over
// the material vocabulary the shipped scenes use: no maps except an
// optional diffuse map, no normal/ao/light maps, no clearcoat/sheen/
// iridescence/transmission, FrontSide only.
//
// three.js is MIT-licensed (LICENSE-three.txt in the skill's vendor
// tree). The functions below are transcribed from the vendored
// three.module.js (r170) ShaderChunks, by name:
//   lights_physical_fragment   (material setup, geometryRoughness)
//   lights_physical_pars_fragment (BRDF_GGX, F_Schlick, V/D_GGX,
//     DFGApprox, computeMultiscattering, RE_Direct/Indirect_Physical)
//   lights_fragment_begin      (light loops, irradiance assembly)
//   lights_fragment_maps       (IBL irradiance/radiance wiring)
//   bsdfs                      (BRDF_Lambert)
//   lights_pars_begin          (getDirectional/Point/Hemisphere infos,
//     getDistanceAttenuation, punctual falloff)
//   envmap_physical_pars_fragment (getIBLIrradiance / getIBLRadiance)
//   cube_uv_reflection_fragment (getFace, getUV, bilinearCubeUV,
//     roughnessToMip, textureCubeUV) — sampling the captured PMREM
//     texture assets/room-env-pmrem.f16 (768x1024, CUBEUV_MAX_MIP 8)
//   shadowmap_pars_fragment    (getShadow, SHADOWMAP_TYPE_PCF_SOFT
//     branch, texture2DCompare) + packing (unpackRGBAToDepth)
//   shadowmap_vertex / worldpos_vertex (shadowWorldPosition with
//     shadowNormalBias applied to the un-normalised world normal)
//
// Light-rig uniforms arrive PRE-FOLDED the way WebGLLights folds them:
// directional/point/hemisphere colours are colour*intensity in linear
// space; the fire point light carries three's distance/decay pair.
// The PMREM environment effect enters through env_intensity
// (= scene.environmentIntensity 0.25 x material envMapIntensity 1).

struct FrameU {
    vp: mat4x4f,
    shadow_vp: mat4x4f, // bias * light-proj * light-view
    cam_pos: vec4f,
    key_color: vec4f,
    key_dir: vec4f, // direction TO the key light (normalised)
    rim_color: vec4f,
    rim_dir: vec4f,
    hemi_sky: vec4f,
    hemi_ground: vec4f,
    point_pos: vec4f,
    point_color: vec4f,
    // x: point distance (cutoff), y: point decay, z: env_intensity,
    // w: shadow bias
    point_misc: vec4f,
    // x: shadow texel size (1/2048), y: shadow map size (2048)
    shadow_misc: vec4f,
    // Integration extension (film mode): two more point slots in
    // the same folded form; a slot with cutoff (misc.x) <= 0 is
    // skipped entirely. Zero-filled by the ab1 path, which therefore
    // renders exactly as before.
    point2_pos: vec4f,
    point2_color: vec4f,
    point2_misc: vec4f, // x: distance cutoff, y: decay
    point3_pos: vec4f,
    point3_color: vec4f,
    point3_misc: vec4f,
    // three.js Fog (fog_fragment): colour (linear) + near; far +
    // enable flag; camera forward for the view-depth fog factor.
    fog: vec4f, // rgb: fog colour, w: near
    fog2: vec4f, // x: far, y: enabled (0/1)
    cam_fwd: vec4f,
};

struct DrawU {
    model: mat4x4f,
    nmat: mat4x4f,
    // x: roughness, y: metalness, z: receive_shadow (0/1), w: has_map
    mat_params: vec4f,
    emissive: vec4f, // linear, colour * emissiveIntensity
    const_nrm: vec4f, // textured variant: uniform surface normal
};

@group(0) @binding(0) var<uniform> frame: FrameU;
@group(0) @binding(1) var<uniform> draw: DrawU;
@group(1) @binding(0) var map_tex: texture_2d<f32>;
@group(1) @binding(1) var map_smp: sampler;
@group(1) @binding(2) var shadow_tex: texture_2d<f32>;
@group(1) @binding(3) var env_tex: texture_2d<f32>;
@group(1) @binding(4) var env_smp: sampler;

const PI: f32 = 3.141592653589793;
const RECIPROCAL_PI: f32 = 0.3183098861837907;

struct Vout {
    @builtin(position) clip: vec4f,
    @location(0) world_pos: vec3f,
    @location(1) world_nrm: vec3f, // un-normalised (three parity)
    @location(2) albedo: vec3f, // vertex colour (linear)
    @location(3) uv: vec2f,
    @location(4) shadow_coord: vec4f,
};

fn vertex_common(pos: vec3f, nrm: vec3f, col: vec3f, uv: vec2f) -> Vout {
    var o: Vout;
    let world = draw.model * vec4f(pos, 1.0);
    let wn = (draw.nmat * vec4f(nrm, 0.0)).xyz;
    o.clip = frame.vp * world;
    o.world_pos = world.xyz;
    o.world_nrm = wn;
    o.albedo = col;
    o.uv = uv;
    // worldpos_vertex: shadowWorldPosition = worldPosition +
    // shadowWorldNormal * shadowNormalBias (0.02 for the key light).
    let shadow_world = world.xyz + wn * 0.02;
    o.shadow_coord = frame.shadow_vp * vec4f(shadow_world, 1.0);
    return o;
}

@vertex
fn vs_colour(@location(0) pos: vec3f, @location(1) nrm: vec3f, @location(2) col: vec3f) -> Vout {
    return vertex_common(pos, nrm, col, vec2f(0.0));
}

@vertex
fn vs_tex(@location(0) pos: vec3f, @location(1) uv: vec2f) -> Vout {
    return vertex_common(pos, draw.const_nrm.xyz, vec3f(1.0), uv);
}

@vertex
fn vs_map(@location(0) pos: vec3f, @location(1) nrm: vec3f, @location(2) col: vec3f, @location(3) uv: vec2f) -> Vout {
    return vertex_common(pos, nrm, col, uv);
}

// ---------------------------------------------------------------------
// packing.glsl.js: unpackRGBAToDepth over the sampled (nearest) texel.
// ---------------------------------------------------------------------

fn unpack_rgba_to_depth(v: vec4f) -> f32 {
    let downscale: f32 = 255.0 / 256.0;
    return v.x * (downscale / 1.0)
        + v.y * (downscale / 256.0)
        + v.z * (downscale / 65536.0)
        + v.w * (1.0 / 16777216.0);
}

fn texture2d_compare(uv: vec2f, compare: f32) -> f32 {
    // textureLoad at the exact texel = three's NearestFilter sample.
    // Row flip: the varying carries three's shadow uv (v measured
    // from the bottom of the light's image), but wgpu's rasterizer
    // wrote NDC y = +1 into memory row 0 — so the row index is
    // mirrored. (The PMREM texture needs no such flip: it was
    // captured in GL memory order and uploaded verbatim.)
    let dims = vec2f(textureDimensions(shadow_tex));
    let fx = clamp(floor(uv.x * dims.x), 0.0, dims.x - 1.0);
    let fy = clamp(dims.y - 1.0 - floor(uv.y * dims.y), 0.0, dims.y - 1.0);
    let depth = unpack_rgba_to_depth(textureLoad(shadow_tex, vec2<i32>(i32(fx), i32(fy)), 0));
    return step(compare, depth);
}

// shadowmap_pars_fragment getShadow, SHADOWMAP_TYPE_PCF_SOFT branch,
// shadowIntensity = 1 (the three.js default, never overridden here).
fn get_shadow(coord_in: vec4f) -> f32 {
    var shadow_coord = coord_in;
    shadow_coord = vec4f(shadow_coord.xyz / shadow_coord.w, shadow_coord.w);
    shadow_coord.z += frame.point_misc.w; // shadowBias
    let in_frustum = shadow_coord.x >= 0.0 && shadow_coord.x <= 1.0
        && shadow_coord.y >= 0.0 && shadow_coord.y <= 1.0;
    let frustum_test = in_frustum && shadow_coord.z <= 1.0;
    var shadow = 1.0;
    if (frustum_test) {
        let shadow_map_size = vec2f(frame.shadow_misc.y);
        let texel_size = vec2f(frame.shadow_misc.x);
        let dx = texel_size.x;
        let dy = texel_size.y;
        var uv = shadow_coord.xy;
        let f = fract(uv * shadow_map_size + 0.5);
        uv -= f * texel_size;
        let z = shadow_coord.z;
        shadow = (
            texture2d_compare(uv, z)
            + texture2d_compare(uv + vec2f(dx, 0.0), z)
            + texture2d_compare(uv + vec2f(0.0, dy), z)
            + texture2d_compare(uv + texel_size, z)
            + mix(texture2d_compare(uv + vec2f(-dx, 0.0), z),
                  texture2d_compare(uv + vec2f(2.0 * dx, 0.0), z), f.x)
            + mix(texture2d_compare(uv + vec2f(-dx, dy), z),
                  texture2d_compare(uv + vec2f(2.0 * dx, dy), z), f.x)
            + mix(texture2d_compare(uv + vec2f(0.0, -dy), z),
                  texture2d_compare(uv + vec2f(0.0, 2.0 * dy), z), f.y)
            + mix(texture2d_compare(uv + vec2f(dx, -dy), z),
                  texture2d_compare(uv + vec2f(dx, 2.0 * dy), z), f.y)
            + mix(mix(texture2d_compare(uv + vec2f(-dx, -dy), z),
                      texture2d_compare(uv + vec2f(2.0 * dx, -dy), z), f.x),
                  mix(texture2d_compare(uv + vec2f(-dx, 2.0 * dy), z),
                      texture2d_compare(uv + vec2f(2.0 * dx, 2.0 * dy), z), f.x),
                  f.y)
        ) * (1.0 / 9.0);
    }
    return shadow; // mix(1.0, shadow, shadowIntensity = 1)
}

// ---------------------------------------------------------------------
// cube_uv_reflection_fragment, verbatim constants for the captured
// PMREM texture (768x1024, CUBEUV_MAX_MIP 8 — see room-env-pmrem.json).
// ---------------------------------------------------------------------

const CUBEUV_MAX_MIP: f32 = 8.0;
const CUBEUV_TEXEL_WIDTH: f32 = 1.0 / 768.0;
const CUBEUV_TEXEL_HEIGHT: f32 = 1.0 / 1024.0;

fn get_face(direction: vec3f) -> f32 {
    let ad = abs(direction);
    var face = -1.0;
    if (ad.x > ad.z) {
        if (ad.x > ad.y) {
            face = select(3.0, 0.0, direction.x > 0.0);
        } else {
            face = select(4.0, 1.0, direction.y > 0.0);
        }
    } else {
        if (ad.z > ad.y) {
            face = select(5.0, 2.0, direction.z > 0.0);
        } else {
            face = select(4.0, 1.0, direction.y > 0.0);
        }
    }
    return face;
}

fn get_uv(direction: vec3f, face: f32) -> vec2f {
    var uv: vec2f;
    if (face == 0.0) {
        uv = vec2f(direction.z, direction.y) / abs(direction.x);
    } else if (face == 1.0) {
        uv = vec2f(-direction.x, -direction.z) / abs(direction.y);
    } else if (face == 2.0) {
        uv = vec2f(-direction.x, direction.y) / abs(direction.z);
    } else if (face == 3.0) {
        uv = vec2f(-direction.z, direction.y) / abs(direction.x);
    } else if (face == 4.0) {
        uv = vec2f(-direction.x, direction.z) / abs(direction.y);
    } else {
        uv = vec2f(direction.x, direction.y) / abs(direction.z);
    }
    return 0.5 * (uv + 1.0);
}

fn bilinear_cube_uv(direction: vec3f, mip_int_in: f32) -> vec3f {
    var face = get_face(direction);
    let filter_int = max(4.0 - mip_int_in, 0.0);
    let mip_int = max(mip_int_in, 4.0);
    let face_size = exp2(mip_int);
    var uv = get_uv(direction, face) * (face_size - 2.0) + 1.0;
    if (face > 2.0) {
        uv.y += face_size;
        face -= 3.0;
    }
    uv.x += face * face_size;
    uv.x += filter_int * 3.0 * 16.0;
    uv.y += 4.0 * (exp2(CUBEUV_MAX_MIP) - face_size);
    uv.x *= CUBEUV_TEXEL_WIDTH;
    uv.y *= CUBEUV_TEXEL_HEIGHT;
    return textureSampleLevel(env_tex, env_smp, uv, 0.0).rgb;
}

fn roughness_to_mip(roughness: f32) -> f32 {
    var mip = 0.0;
    if (roughness >= 0.8) {
        mip = (1.0 - roughness) * (-1.0 - -2.0) / (1.0 - 0.8) + -2.0;
    } else if (roughness >= 0.4) {
        mip = (0.8 - roughness) * (2.0 - -1.0) / (0.8 - 0.4) + -1.0;
    } else if (roughness >= 0.305) {
        mip = (0.4 - roughness) * (3.0 - 2.0) / (0.4 - 0.305) + 2.0;
    } else if (roughness >= 0.21) {
        mip = (0.305 - roughness) * (4.0 - 3.0) / (0.305 - 0.21) + 3.0;
    } else {
        mip = -2.0 * log2(1.16 * roughness);
    }
    return mip;
}

fn texture_cube_uv(sample_dir: vec3f, roughness: f32) -> vec3f {
    let mip = clamp(roughness_to_mip(roughness), -2.0, CUBEUV_MAX_MIP);
    let mip_f = fract(mip);
    let mip_int = floor(mip);
    let color0 = bilinear_cube_uv(sample_dir, mip_int);
    if (mip_f == 0.0) {
        return color0;
    }
    let color1 = bilinear_cube_uv(sample_dir, mip_int + 1.0);
    return mix(color0, color1, mip_f);
}

// envmap_physical_pars_fragment (envMapRotation = identity).
fn get_ibl_irradiance(normal: vec3f) -> vec3f {
    return PI * texture_cube_uv(normal, 1.0) * frame.point_misc.z;
}

fn get_ibl_radiance(view_dir: vec3f, normal: vec3f, roughness: f32) -> vec3f {
    var reflect_vec = reflect(-view_dir, normal);
    reflect_vec = normalize(mix(reflect_vec, normal, roughness * roughness));
    return texture_cube_uv(reflect_vec, roughness) * frame.point_misc.z;
}

// ---------------------------------------------------------------------
// lights_physical_pars_fragment: the BRDF set.
// ---------------------------------------------------------------------

fn pow2(x: f32) -> f32 { return x * x; }

fn f_schlick(f0: vec3f, f90: f32, dot_vh: f32) -> vec3f {
    let fresnel = exp2((-5.55473 * dot_vh - 6.98316) * dot_vh);
    return f0 + (vec3f(f90) - f0) * fresnel;
}

fn v_ggx_smith_correlated(alpha: f32, dot_nl: f32, dot_nv: f32) -> f32 {
    let a2 = pow2(alpha);
    let gv = dot_nl * sqrt(a2 + (1.0 - a2) * pow2(dot_nv));
    let gl = dot_nv * sqrt(a2 + (1.0 - a2) * pow2(dot_nl));
    return 0.5 / max(gv + gl, 1e-6); // EPSILON
}

fn d_ggx(alpha: f32, dot_nh: f32) -> f32 {
    let a2 = pow2(alpha);
    let denom = pow2(dot_nh) * (a2 - 1.0) + 1.0;
    return RECIPROCAL_PI * a2 / pow2(denom);
}

fn brdf_ggx(light_dir: vec3f, view_dir: vec3f, normal: vec3f,
            specular_color: vec3f, specular_f90: f32, roughness: f32) -> vec3f {
    let alpha = pow2(roughness);
    let half_dir = normalize(light_dir + view_dir);
    let dot_nl = clamp(dot(normal, light_dir), 0.0, 1.0);
    let dot_nv = clamp(dot(normal, view_dir), 0.0, 1.0);
    let dot_nh = clamp(dot(normal, half_dir), 0.0, 1.0);
    let dot_vh = clamp(dot(view_dir, half_dir), 0.0, 1.0);
    let f = f_schlick(specular_color, specular_f90, dot_vh);
    let v = v_ggx_smith_correlated(alpha, dot_nl, dot_nv);
    let d = d_ggx(alpha, dot_nh);
    return f * (v * d);
}

fn brdf_lambert(diffuse_color: vec3f) -> vec3f {
    return RECIPROCAL_PI * diffuse_color;
}

fn dfg_approx(normal: vec3f, view_dir: vec3f, roughness: f32) -> vec2f {
    let dot_nv = clamp(dot(normal, view_dir), 0.0, 1.0);
    let c0 = vec4f(-1.0, -0.0275, -0.572, 0.022);
    let c1 = vec4f(1.0, 0.0425, 1.04, -0.04);
    let r = roughness * c0 + c1;
    let a004 = min(r.x * r.x, exp2(-9.28 * dot_nv)) * r.x + r.y;
    return vec2f(-1.04, 1.04) * a004 + r.zw;
}

// Point-light distance falloff (lights_pars_begin getDistanceAttenuation,
// physicallyCorrectLights is the only mode since r155).
fn get_distance_attenuation(light_distance: f32, cutoff_distance: f32, decay_exponent: f32) -> f32 {
    var distance_falloff = 1.0 / max(pow(light_distance, decay_exponent), 0.01);
    if (cutoff_distance > 0.0) {
        let t = clamp(1.0 - pow(light_distance / cutoff_distance, 4.0), 0.0, 1.0);
        distance_falloff *= t * t;
    }
    return distance_falloff;
}

@fragment
fn fs_main(in: Vout) -> @location(0) vec4f {
    // normal_fragment_begin: FrontSide only, no normal maps, so the
    // shading normal is the normalised interpolated world normal.
    let normal = normalize(in.world_nrm);
    let view_dir = normalize(frame.cam_pos.xyz - in.world_pos);

    // lights_physical_fragment: material setup.
    // map_fragment: diffuseColor *= texture — the vertex colour is
    // the material colour, so mapped surfaces multiply the two.
    var diffuse = in.albedo;
    if (draw.mat_params.w > 0.5) {
        diffuse *= textureSample(map_tex, map_smp, in.uv).rgb;
    }
    let metalness = draw.mat_params.y;
    let diffuse_color = diffuse * (1.0 - metalness);
    let specular_color = mix(vec3f(0.04), diffuse, metalness);
    let specular_f90 = 1.0;
    var roughness = max(draw.mat_params.x, 0.0525);
    // geometryRoughness from the non-perturbed normal's derivatives:
    // max component of the componentwise max of |dFdx| and |dFdy|.
    let dxy = max(abs(dpdx(normal)), abs(dpdy(normal)));
    let geo_rough = max(max(dxy.x, dxy.y), dxy.z);
    roughness = min(roughness + geo_rough, 1.0);

    var direct_diffuse = vec3f(0.0);
    var direct_specular = vec3f(0.0);
    var indirect_diffuse = vec3f(0.0);
    var indirect_specular = vec3f(0.0);

    // ---- direct lights (lights_fragment_begin) ----
    // Key directional, shadow-mapped.
    {
        var light_color = frame.key_color.rgb;
        if (draw.mat_params.z > 0.5) {
            light_color *= get_shadow(in.shadow_coord);
        }
        let l = frame.key_dir.xyz;
        let dot_nl = clamp(dot(normal, l), 0.0, 1.0);
        let irradiance = dot_nl * light_color;
        direct_specular += irradiance * brdf_ggx(l, view_dir, normal, specular_color, specular_f90, roughness);
        direct_diffuse += irradiance * brdf_lambert(diffuse_color);
    }
    // Rim directional, unshadowed.
    {
        let l = frame.rim_dir.xyz;
        let dot_nl = clamp(dot(normal, l), 0.0, 1.0);
        let irradiance = dot_nl * frame.rim_color.rgb;
        direct_specular += irradiance * brdf_ggx(l, view_dir, normal, specular_color, specular_f90, roughness);
        direct_diffuse += irradiance * brdf_lambert(diffuse_color);
    }
    // Fire point light.
    {
        let lvec = frame.point_pos.xyz - in.world_pos;
        let dist = length(lvec);
        let l = lvec / dist;
        let att = get_distance_attenuation(dist, frame.point_misc.x, frame.point_misc.y);
        let light_color = frame.point_color.rgb * att;
        let dot_nl = clamp(dot(normal, l), 0.0, 1.0);
        let irradiance = dot_nl * light_color;
        direct_specular += irradiance * brdf_ggx(l, view_dir, normal, specular_color, specular_f90, roughness);
        direct_diffuse += irradiance * brdf_lambert(diffuse_color);
    }

    // Extra point lights (film mode; skipped when cutoff <= 0,
    // which is what the ab1 path's zero padding produces).
    if (frame.point2_misc.x > 0.0) {
        let lvec = frame.point2_pos.xyz - in.world_pos;
        let dist = length(lvec);
        let l = lvec / dist;
        let att = get_distance_attenuation(dist, frame.point2_misc.x, frame.point2_misc.y);
        let light_color = frame.point2_color.rgb * att;
        let dot_nl = clamp(dot(normal, l), 0.0, 1.0);
        let irradiance = dot_nl * light_color;
        direct_specular += irradiance * brdf_ggx(l, view_dir, normal, specular_color, specular_f90, roughness);
        direct_diffuse += irradiance * brdf_lambert(diffuse_color);
    }
    if (frame.point3_misc.x > 0.0) {
        let lvec = frame.point3_pos.xyz - in.world_pos;
        let dist = length(lvec);
        let l = lvec / dist;
        let att = get_distance_attenuation(dist, frame.point3_misc.x, frame.point3_misc.y);
        let light_color = frame.point3_color.rgb * att;
        let dot_nl = clamp(dot(normal, l), 0.0, 1.0);
        let irradiance = dot_nl * light_color;
        direct_specular += irradiance * brdf_ggx(l, view_dir, normal, specular_color, specular_f90, roughness);
        direct_diffuse += irradiance * brdf_lambert(diffuse_color);
    }

    // ---- indirect diffuse: hemisphere (no ambient light in the rig) ----
    {
        let hemi = mix(frame.hemi_ground.rgb, frame.hemi_sky.rgb, 0.5 * normal.y + 0.5);
        indirect_diffuse += hemi * brdf_lambert(diffuse_color);
    }

    // ---- IBL (lights_fragment_maps + RE_IndirectSpecular_Physical) ----
    {
        let ibl_irradiance = get_ibl_irradiance(normal);
        let radiance = get_ibl_radiance(view_dir, normal, roughness);
        let fab = dfg_approx(normal, view_dir, roughness);
        let fss_ess = specular_color * fab.x + specular_f90 * fab.y;
        let ess = fab.x + fab.y;
        let ems = 1.0 - ess;
        let favg = specular_color + (1.0 - specular_color) * 0.047619;
        let fms = fss_ess * favg / (1.0 - ems * favg);
        let single = fss_ess;
        let multi = fms * ems;
        let total_scattering = single + multi;
        let cosine_irradiance = ibl_irradiance * RECIPROCAL_PI;
        indirect_specular += radiance * single + multi * cosine_irradiance;
        let diff = diffuse_color * (1.0 - max(total_scattering.r, max(total_scattering.g, total_scattering.b)));
        indirect_diffuse += diff * cosine_irradiance;
    }

    // lights_fragment_end + opaque_fragment (no tonemapping in the
    // composer chain — OutputPass owns it).
    let outgoing = direct_diffuse + indirect_diffuse + direct_specular + indirect_specular
        + draw.emissive.rgb;
    // fog_fragment: factor over view depth (three's vFogDepth is
    // -mvPosition.z == distance along the camera forward axis).
    var out_col = outgoing;
    if (frame.fog2.y > 0.5) {
        let fdepth = dot(in.world_pos - frame.cam_pos.xyz, frame.cam_fwd.xyz);
        let ffactor = smoothstep(frame.fog.w, frame.fog2.x, fdepth);
        out_col = mix(out_col, frame.fog.rgb, ffactor);
    }
    return vec4f(out_col, 1.0);
}
