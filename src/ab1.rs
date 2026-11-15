//! §28 A/B reproduction scene: Onoda S01 (HOOK) at t = 1.50 s from the
//! delivered project ~/faceless-shorts/the-soldier-who-didnt-know
//! ("THE SOLDIER WHO DIDN'T KNOW THE WAR WAS OVER", v4-era Tall Tale).
//!
//! Every staging value in this file was extracted from the project's
//! own sources (file:line citations live in
//! docs/ab/onoda-s01-t150/README.md). Since Milestone 5 the scene
//! renders through the full production chain: PBR standard materials
//! under the engine's real light rig (key with PCFSoft shadow map,
//! rim, hemisphere, captured PMREM IBL, fire point light) into
//! HalfFloat MSAA-4x, then Bokeh -> UnrealBloom -> Output -> GRADE,
//! with the caption composited last, in sRGB space, as production
//! does. The A/B evidence lives in docs/ab/composer/. Where the real
//! engine still has something this renderer lacks (canvas-painted
//! planks/log textures, icosahedron canopies, geometry tessellation
//! deltas), the closest honest primitive stands in and the
//! substitution is logged in docs/ab/onoda-s01-t150/gaps.md — nothing
//! is silently approximated.
//!
//! Renderer of record for this scene: wgpu on lavapipe (software
//! Vulkan) — correctness evidence only, per contract §6.2.

use crate::math::{self, Mat4};
use std::borrow::Cow;

use crate::composer::{self, BloomParams, GradeParams, PostChain};
use crate::scene::{
    box_geo, cone_geo, cross3, cyl_geo, disc_geo, flat_quad_geo, ground_disc_geo, hex_lin, norm3,
    paint_blob, paint_cloud, paint_grass, paint_sky, quad_geo, rnd, shade_lin, sky_sphere_geo,
    sphere_geo, sphere_map_geo, sub3, ColourDraw, Geo, MapGeo, TexDraw, TexGeo,
};

// ---------------------------------------------------------------------
// The Onoda card: drawToon re-implemented for exactly the state S01
// produces at t = 1.50 (see README.md for the extraction). Stage space
// is 512x1024 with the figure's feet at stage y = 1000; the canvas is
// 1280x1408 with the stage at offset (384, 256), painted at res 2.
// ---------------------------------------------------------------------

/// The Onoda card: scene::paint_toon with exactly the look/state
/// S01 produces at t = 1.50 (camPitchOf(jgOnoda, camPos) = 0.28747,
/// poseTrack sweep headTurn = -0.025, lean 1 — see README.md).
pub(crate) fn paint_onoda() -> (Vec<u8>, u32, u32) {
    crate::scene::paint_toon(&crate::scene::ToonSpec {
        body: "#5C6644",
        head: "#F7F7FA",
        shade: Some("#878E6E"),
        cap: Some("#4C5638"),
        mood: crate::scene::ToonMood::Determined,
        arm_l: (10.0, 6.0),
        arm_r: (16.0, -6.0),
        lean_deg: 1.0,
        head_turn: -0.025,
        cam_pitch: 0.28747,
    })
}

// ---------------------------------------------------------------------
// Scene assembly: the JG jungle camp at t = 1.50, re-authored from
// shots.ts (see README.md for the line-by-line extraction).
// ---------------------------------------------------------------------


const TEX_SKY: usize = 0;
const TEX_GRASS: usize = 1;
const TEX_CLOUD: usize = 2;
const TEX_BLOB: usize = 3;
const TEX_CARD: usize = 4;
const TEX_HILLGRASS: usize = 5;
// The caption strip is uploaded separately (raw Unorm — it composites
// after GRADE); this index only labels the draw.
const TEX_CAPTION: usize = 6;

// Camera at t = 1.50 (camK smootherstep k = 0.05228 through the S01
// push-in; fov 44 from G.jungle spread in the shoot() call).
pub(crate) const EYE: [f32; 3] = [0.83432, 2.64354, 5.28499];
pub(crate) const TGT: [f32; 3] = [0.35, 1.11477, -0.55];

fn t3(x: f32, y: f32, z: f32) -> Mat4 {
    math::translate(x, y, z)
}

/// A MeshStandardMaterial's scalar parameters (the colour itself is
/// the geometry's vertex colour, as in the §28 build). Emissive is
/// linear colour * emissiveIntensity, folded like three does.
pub(crate) struct StdMat {
    pub rough: f32,
    pub metal: f32,
    pub emissive: [f32; 3],
}

fn std_mat(rough: f32, metal: f32) -> StdMat {
    StdMat { rough, metal, emissive: [0.0, 0.0, 0.0] }
}

/// One standard-material draw with its shadow flags, mirroring one
/// mesh of the original scene graph (castShadow / receiveShadow are
/// per-object in three.js).
pub(crate) struct StdDraw {
    pub geo: Geo,
    pub model: Mat4,
    pub mat: StdMat,
    pub cast: bool,
    pub receive: bool,
}

/// A mapped standard-material draw (diffuse = colour x map).
pub(crate) struct MapDraw {
    pub geo: MapGeo,
    pub model: Mat4,
    pub mat: StdMat,
    pub cast: bool,
    pub receive: bool,
}

struct Scene {
    std: Vec<StdDraw>,
    mapped: Vec<MapDraw>,
    sky: TexDraw,
    grass: TexDraw,
    blob: TexDraw,
    clouds: Vec<TexDraw>,
    card: TexDraw,
    caption: TexDraw,
}

fn build_scene() -> Scene {
    let mut std: Vec<StdDraw> = Vec::new();
    let mut mapped: Vec<MapDraw> = Vec::new();
    let add = |draws: &mut Vec<StdDraw>, geo: Geo, model: Mat4, mat: StdMat, cast: bool, receive: bool| {
        draws.push(StdDraw { geo, model, mat, cast, receive });
    };
    // The world's mat() factory default (world.ts:44-51).
    let def = || std_mat(0.85, 0.0);

    // ---- dirt patch (world.ts: CircleGeometry r 4.8, flat '#86714C',
    // rough 1.0 — shots.ts dirt material; receiveShadow default from
    // the CircleGeometry mesh = receive only)
    add(&mut std, disc_geo(4.8, 28, hex_lin("#86714C")), t3(0.0, 0.012, -0.9), std_mat(1.0, 0.0), false, true);

    // ---- hills (z -65..-85) with their small trees ----
    let hills: [(f32, f32, f32, f32); 4] = [
        (-40.0, -75.0, 34.0, 12.0),
        (10.0, -85.0, 46.0, 16.0),
        (60.0, -71.0, 36.0, 11.0),
        (-90.0, -65.0, 40.0, 10.0),
    ];
    for (i, &(hx, hz, r, h)) in hills.iter().enumerate() {
        let col = shade_lin("#5E8F52", (0.92 + rnd(i as f64) * 0.16) as f32);
        let model = math::mul(&t3(hx, -h * 0.25, hz), &math::scale(r, h, r * 0.6));
        // world.ts hills(): mat({ color: shade(col, ...), map:
        // T.grass(col, [6, 3]) }) — colour x grass map, repeat [6, 3]
        // baked into the uvs. Hill meshes never touch shadows
        // (neither flag is set on the sphere). SphereGeometry in
        // three is 32x16 (the §28 build used 24x16 flat — the mapped
        // build follows three's tessellation).
        mapped.push(MapDraw {
            geo: sphere_map_geo(1.0, 32, 16, col, [6.0, 3.0]),
            model,
            mat: def(),
            cast: false,
            receive: false,
        });
        // 6 small trees per hill (hills() forces castShadow = false on
        // the whole subtree; receiveShadow keeps its builder value).
        for k in 0..6 {
            let a = rnd(i as f64 * 9.0 + k as f64) * 2.0 - 1.0;
            let s = (0.5 + rnd(k as f64) * 0.4) as f32 * 2.2;
            let pos = [
                hx + (a as f32) * r * 0.6,
                (h as f64 * (0.75 - a.abs() * 0.45) - 1.0) as f32,
                hz + r * 0.25,
            ];
            add_tree(&mut std, pos, 0.0, s, "#3E7A34", true, true);
        }
    }

    // ---- tree ring (19 trees, rotation.y = i*1.7) ----
    let ring: [(f32, f32, f32, bool); 19] = [
        (-6.5, -6.0, 1.5, true),
        (-9.0, -2.0, 1.3, false),
        (-7.5, -11.0, 1.7, true),
        (-12.0, -7.0, 1.5, true),
        (-13.0, -14.0, 1.8, false),
        (6.8, -6.5, 1.6, true),
        (9.5, -2.5, 1.35, false),
        (8.0, -12.0, 1.75, true),
        (12.5, -8.0, 1.5, true),
        (13.5, -15.0, 1.9, false),
        (-4.5, -13.0, 1.6, true),
        (4.6, -14.0, 1.55, true),
        (0.0, -17.0, 1.9, true),
        (-16.0, -4.0, 1.6, true),
        (16.5, -5.0, 1.7, true),
        (-5.5, 4.5, 1.3, true),
        (6.2, 5.0, 1.25, false),
        (-11.0, 3.0, 1.5, true),
        (11.5, 3.5, 1.45, true),
    ];
    for (i, &(x, z, s, round)) in ring.iter().enumerate() {
        let col = if i % 3 != 0 { "#3E7A34" } else { "#356B2C" };
        add_tree(&mut std, [x, 0.0, z], i as f32 * 1.7, s, col, round, false);
    }

    // ---- palms ----
    add_palm(&mut std, [4.6, 0.0, -4.6], 1.05, 0.14, 0.0);
    add_palm(&mut std, [-5.2, 0.0, -5.4], 0.9, -0.2, 1.2);
    add_palm(&mut std, [10.5, 0.0, -11.0], 1.15, 0.1, 0.0);

    // ---- grass tufts: 900 instanced cones in the original; here the
    // same hash-placed cones merged into one geometry ----
    {
        let mut merged = Geo::new();
        let cone = cone_geo(0.05, 0.34, 4, hex_lin("#4C8A3E"));
        for i in 0..900 {
            let fi = i as f64;
            let a = rnd(fi * 1.31) * 6.283185307;
            let r = 2.6 + rnd(fi * 2.7) * 20.0;
            let (x, z) = ((a.cos() * r) as f32, (a.sin() * r) as f32 - 3.0);
            let mut inst = Geo { verts: cone.verts.clone(), idx: cone.idx.clone() };
            for v in inst.verts.chunks_exact_mut(9) {
                v[0] += x;
                v[1] += 0.15;
                v[2] += z;
            }
            merged.append(inst);
        }
        // InstancedMesh: castShadow/receiveShadow both default false.
        add(&mut std, merged, math::identity(), def(), false, false);
    }

    // ---- camp (fire pit at FX=-1.35, FZ=-2.45) ----
    // The camp builders are shots.ts box()/cyl() helpers: castShadow
    // and receiveShadow both true, EXCEPT the fire stones, which are
    // raw meshes with only castShadow set.
    let (fx, fz) = (-1.35f32, -2.45f32);
    let stone = hex_lin("#8B8B84");
    for i in 0..8 {
        let a = (i as f32 / 8.0) * std::f32::consts::TAU;
        let r = (0.09 + rnd(i as f64) * 0.04) as f32;
        add(
            &mut std,
            sphere_geo(r, 8, 6, stone),
            t3(fx + a.cos() * 0.44, 0.05, fz + a.sin() * 0.44),
            std_mat(0.95, 0.0),
            true,
            false,
        );
    }
    let log_col = hex_lin("#7A5A38");
    let log_m = || std_mat(0.95, 0.0); // mat({ map: logTex }) rough 0.95
    for (i, &(dx, dz)) in [(0.25f32, 0.3f32), (-0.2, -0.28), (0.05, 0.02)].iter().enumerate() {
        let model = math::mul(
            &t3(fx + dx, 0.09, fz + dz),
            &math::euler_xyz(0.0, i as f32 * 0.7, std::f32::consts::FRAC_PI_2),
        );
        add(&mut std, cyl_geo(0.05, 0.05, 0.72, 7, log_col), model, log_m(), true, true);
    }
    // Flames at t = 1.50: fireLevel scaleY 1.13146 / 0.86854. Real
    // materials now (M5): dark diffuse + strong emissive (shots.ts
    // fire(): emissive #FF7A1A x2.4 / #FFC93C x2.6, roughness 1).
    let flame_model = math::mul(&t3(fx, 0.34, fz), &math::scale(1.0, 1.13146, 1.0));
    let mut flame1 = std_mat(1.0, 0.0);
    flame1.emissive = hex_lin("#FF7A1A").map(|c| c * 2.4);
    add(&mut std, cone_geo(0.17, 0.52, 8, hex_lin("#3A1C08")), flame_model, flame1, false, false);
    let flame2_model = math::mul(&t3(fx + 0.03, 0.30, fz + 0.04), &math::scale(1.0, 0.86854, 1.0));
    let mut flame2 = std_mat(1.0, 0.0);
    flame2.emissive = hex_lin("#FFC93C").map(|c| c * 2.6);
    add(&mut std, cone_geo(0.09, 0.34, 8, hex_lin("#3A2A08")), flame2_model, flame2, false, false);
    // Seat log.
    let seat_model = math::mul(
        &t3(-0.5, 0.17, -1.8),
        &math::euler_xyz(0.0, 0.12, std::f32::consts::FRAC_PI_2),
    );
    add(&mut std, cyl_geo(0.17, 0.17, 1.15, 9, log_col), seat_model, log_m(), true, true);
    // Lean-to: two poles + plank roof (flat colour — planks texture
    // not reproduced, gaps.md).
    for x in [-3.9f32, -2.7] {
        add(&mut std, cyl_geo(0.035, 0.035, 1.5, 6, log_col), t3(x, 0.75, -2.7), log_m(), true, true);
    }
    let roof_model = math::mul(&t3(-3.3, 1.52, -2.95), &math::rotate_x(0.42));
    add(&mut std, box_geo(1.9, 0.05, 1.5, hex_lin("#8A6B42")), roof_model, std_mat(0.95, 0.0), true, true);
    // Crates + barrel + bedroll (flat colours; planks maps dropped).
    add(&mut std, box_geo(0.55, 0.55, 0.55, hex_lin("#B07A48")), t3(-2.35, 0.275, -3.15), def(), true, true);
    let crate2_model = math::mul(&t3(-2.3, 0.55 + 0.21, -3.12), &math::rotate_y(0.4));
    add(&mut std, box_geo(0.42, 0.42, 0.42, hex_lin("#B07A48")), crate2_model, def(), true, true);
    add(&mut std, cyl_geo(0.33, 0.33, 0.8, 16, hex_lin("#6E4525")), t3(-3.05, 0.4, -2.2), def(), true, true);
    let band_col = hex_lin("#3A3A3F");
    for y in [0.12f32, 0.68] {
        // Barrel bands: MeshStandardMaterial metalness 0.4 (shots.ts).
        add(&mut std, cyl_geo(0.345, 0.345, 0.05, 16, band_col), t3(-3.05, y, -2.2), std_mat(0.85, 0.4), true, true);
    }
    let bed_model = math::mul(&t3(-3.3, 0.05, -2.1), &math::rotate_y(0.2));
    add(&mut std, box_geo(1.5, 0.09, 0.55, hex_lin("#77805E")), bed_model, std_mat(1.0, 0.0), true, true);


    // ---- Onoda card + rifle: built by onoda_card_and_rifle()
    // (shared with the scene-format loader's code-ref nodes) ----
    let (card, rifle_draws) = onoda_card_and_rifle();
    // Rifle part materials by construction order (shots.ts makeRifle):
    // wood stock, steel receiver/barrel/sight (metalness 0.55,
    // roughness 0.5), wood grip. Rifle parts are box()/cyl() meshes:
    // cast + receive.
    let rifle_mats = [
        std_mat(0.85, 0.0),
        std_mat(0.5, 0.55),
        std_mat(0.5, 0.55),
        std_mat(0.5, 0.55),
        std_mat(0.85, 0.0),
    ];
    for (d, m) in rifle_draws.into_iter().zip(rifle_mats) {
        std.push(StdDraw { geo: d.geo, model: d.model, mat: m, cast: true, receive: true });
    }
    let h_t = 1.75f32 / 0.96;

    // ---- sky dome + grass ground + blob + clouds + caption ----
    let sky = TexDraw { geo: sky_sphere_geo(380.0, 32, 16), model: math::identity(), tex: TEX_SKY, repeat: false };
    let grass = TexDraw { geo: ground_disc_geo(80.0, 48, 48.0), model: math::identity(), tex: TEX_GRASS, repeat: true };
    let blob = TexDraw {
        geo: flat_quad_geo((h_t / 2.0) * 0.9, (h_t / 2.0) * 0.34),
        model: t3(0.35, 0.02, -0.55),
        tex: TEX_BLOB,
        repeat: false,
    };
    // Clouds: three sprites, billboarding toward the camera.
    let fwd = norm3(sub3(TGT, EYE));
    let right = norm3(cross3(fwd, [0.0, 1.0, 0.0]));
    let upv = cross3(right, fwd);
    let mut clouds = Vec::new();
    for (cx, cy, cz, s) in [
        (-28.0f32, 23.0f32, -85.0f32, 13.0f32),
        (26.0, 29.0, -95.0, 16.0),
        (58.0, 19.0, -75.0, 10.0),
    ] {
        let (hw, hh) = (s, s / 2.0); // sprite scale (2s, s)
        let corner = |sx: f32, sy: f32| -> [f32; 3] {
            [
                cx + right[0] * hw * sx + upv[0] * hh * sy,
                cy + right[1] * hw * sx + upv[1] * hh * sy,
                cz + right[2] * hw * sx + upv[2] * hh * sy,
            ]
        };
        let mut g = TexGeo::new();
        g.vert(corner(-1.0, -1.0), [0.0, 1.0]);
        g.vert(corner(1.0, -1.0), [1.0, 1.0]);
        g.vert(corner(1.0, 1.0), [1.0, 0.0]);
        g.vert(corner(-1.0, 1.0), [0.0, 0.0]);
        g.tri(0, 1, 2);
        g.tri(0, 2, 3);
        clouds.push(TexDraw { geo: g, model: math::identity(), tex: TEX_CLOUD, repeat: false });
    }
    // Caption strip: "fighting" at (540, 1340) in a 1080x1920 frame,
    // as a camera-facing quad 2 m ahead (see README.md).
    let caption = build_caption_draw();

    Scene { std, mapped, sky, grass, blob, clouds, card, caption }
}

/// canopies are approximated by spheres (gaps.md).
/// The Onoda character card and its attached rifle, built from the
/// extracted S01 state (see build_scene). Shared with the
/// scene-format loader, whose code-ref nodes splice these draws in
/// until the M4 painter can express them as data
/// (docs/scene-format.md). The card's `tex` field carries TEX_CARD,
/// an index into this module's texture table; the loader remaps it
/// onto its own "onoda-card" texture.
pub(crate) fn onoda_card_and_rifle() -> (TexDraw, Vec<ColourDraw>) {
    // Toon H = 1.75 / 0.96; plane 1.25H x 1.375H; geometry centre sits
    // 552 stage units above the feet (toon.ts plane derivation).
    let h_t = 1.75f32 / 0.96;
    let su = h_t / 1024.0;
    let plane_w = h_t * 1280.0 / 1024.0;
    let plane_h = h_t * 1408.0 / 1024.0;
    let centre_y = 552.0 * su;
    // face(): yaw atan2(cam.x - p.x, cam.z - p.z) and perspective
    // pitch clamp(pitchAng * 0.45) about the pivot (order YXZ).
    let card_model = math::mul(
        &t3(0.35, 0.0, -0.55),
        &math::mul(&math::rotate_y(0.08282), &math::rotate_x(0.11642)),
    );
    let card = TexDraw {
        geo: quad_geo(plane_w, plane_h, centre_y - plane_h / 2.0),
        model: card_model,
        tex: TEX_CARD,
        repeat: false,
    };
    // Rifle (attach3D, hand R): anchor from the painter's hand
    // position through syncAttachments' formula, offset [0, .03, .07].
    let mut rifle: Vec<ColourDraw> = Vec::new();
    {
        let (hx, hy) = (373.98f32, 599.73f32); // stage-space hand R
        let w = [(384.0 + hx - 640.0) * su, (1256.0 - (256.0 + hy)) * su, 0.0];
        let rifle_model = math::mul(
            &card_model,
            &t3(w[0] + 0.0, w[1] + 0.03, w[2] + 0.07),
        );
        let wood = hex_lin("#6E4F30");
        let steel = hex_lin("#3A3D42");
        let parts: [(Geo, Mat4); 5] = [
            (box_geo(0.075, 0.52, 0.13, wood), t3(0.0, -0.17, 0.0)),
            (box_geo(0.06, 0.1, 0.09, steel), t3(0.0, 0.13, 0.0)),
            (cyl_geo(0.016, 0.016, 0.55, 8, steel), t3(0.0, 0.42, 0.0)),
            (box_geo(0.02, 0.05, 0.02, steel), t3(0.0, 0.68, 0.0)),
            (box_geo(0.05, 0.05, 0.16, wood), t3(0.0, -0.02, 0.05)),
        ];
        for (geo, local) in parts {
            rifle.push(ColourDraw { geo, model: math::mul(&rifle_model, &local) });
        }
    }
    (card, rifle)
}

fn add_tree(draws: &mut Vec<StdDraw>, pos: [f32; 3], rot_y: f32, s: f32, col: &str, round: bool, on_hill: bool) {
    // world.ts tree(): the cyl() helper sets cast+receive on the trunk;
    // canopy spheres set castShadow only. hills() then forces
    // castShadow = false across the subtree (receive keeps its value).
    let base = math::mul(&t3(pos[0], pos[1], pos[2]), &math::rotate_y(rot_y));
    let trunk = cyl_geo(0.12 * s, 0.18 * s, 1.6 * s, 7, hex_lin("#7A5234"));
    draws.push(StdDraw {
        geo: trunk,
        model: math::mul(&base, &t3(0.0, 0.8 * s, 0.0)),
        mat: std_mat(0.85, 0.0),
        cast: !on_hill,
        receive: true,
    });
    if round {
        let blobs: [(f32, f32, f32, f32); 5] = [
            (0.0, 2.3, 0.0, 1.0),
            (0.6, 2.0, 0.2, 0.75),
            (-0.6, 2.0, -0.1, 0.8),
            (0.1, 2.9, -0.2, 0.75),
            (0.2, 2.2, 0.6, 0.7),
        ];
        for (i, &(ox, oy, oz, f)) in blobs.iter().enumerate() {
            let c = shade_lin(col, (0.9 + rnd(i as f64) * 0.2) as f32);
            draws.push(StdDraw {
                geo: sphere_geo(f * s, 12, 9, c),
                model: math::mul(&base, &t3(ox * s, oy * s, oz * s)),
                mat: std_mat(0.85, 0.0),
                cast: !on_hill,
                receive: false,
            });
        }
    } else {
        let model = math::mul(
            &math::mul(&base, &t3(0.0, 3.2 * s, 0.0)),
            &math::scale(0.7 * s, 2.6 * s, 0.7 * s),
        );
        draws.push(StdDraw {
            geo: sphere_geo(1.0, 16, 12, hex_lin(col)),
            model,
            mat: std_mat(0.85, 0.0),
            cast: !on_hill,
            receive: false,
        });
    }
}

/// One palm (shots.ts palm()): leaning trunk, 7 fronds with tips,
/// 3 coconuts. Frond spheres approximate the original's flattened
/// sphere fronds with the same scale/placement.
fn add_palm(draws: &mut Vec<StdDraw>, pos: [f32; 3], s: f32, lean: f32, rot_y: f32) {
    // shots.ts palm(): trunk via cyl() (cast+receive, bark rough 0.95);
    // fronds/tips/nuts are raw spheres with castShadow only
    // (leaves/nuts rough 0.9).
    let base = math::mul(
        &t3(pos[0], pos[1], pos[2]),
        &math::mul(&math::rotate_y(rot_y), &math::mul(&math::rotate_z(lean), &math::scale(s, s, s))),
    );
    draws.push(StdDraw {
        geo: cyl_geo(0.085, 0.13, 4.4, 8, hex_lin("#7A6248")),
        model: math::mul(&base, &t3(0.0, 2.2, 0.0)),
        mat: std_mat(0.95, 0.0),
        cast: true,
        receive: true,
    });
    let top_y = 4.35f32;
    let leaf = hex_lin("#3F7D33");
    let leaf2 = hex_lin("#4C8A3A");
    for i in 0..7 {
        let a = (i as f32 / 7.0) * std::f32::consts::TAU + 0.4;
        let (sa, ca) = a.sin_cos();
        let col = if i % 2 == 0 { leaf } else { leaf2 };
        let frond_model = math::mul(
            &math::mul(&base, &t3(sa * 1.0, top_y + 0.1 - (i % 3) as f32 * 0.07, ca * 1.0)),
            &math::mul(&math::rotate_y(a), &math::scale(0.17, 0.05, 1.2)),
        );
        draws.push(StdDraw { geo: sphere_geo(1.0, 10, 6, col), model: frond_model, mat: std_mat(0.9, 0.0), cast: true, receive: false });
        let tip_model = math::mul(
            &math::mul(&base, &t3(sa * 1.85, top_y - 0.28 - (i % 3) as f32 * 0.06, ca * 1.85)),
            &math::mul(&math::euler_xyz(ca * 0.5, a, -sa * 0.5), &math::scale(0.13, 0.04, 0.8)),
        );
        draws.push(StdDraw { geo: sphere_geo(1.0, 10, 6, col), model: tip_model, mat: std_mat(0.9, 0.0), cast: true, receive: false });
    }
    let nut = hex_lin("#6B4A2E");
    for &(nx, nz) in [(0.14f32, 0.0f32), (-0.12, 0.1), (0.0, -0.14)].iter() {
        draws.push(StdDraw {
            geo: sphere_geo(0.13, 10, 8, nut),
            model: math::mul(&base, &t3(nx, top_y - 0.2, nz)),
            mat: std_mat(0.9, 0.0),
            cast: true,
            receive: false,
        });
    }
}

/// The caption as a textured quad floating 2 m in front of the
/// camera, centred on the caption's NDC position (0, -0.3958) and
/// spanning the full frame width, 300/1920 of the frame height.
fn build_caption_draw() -> TexDraw {
    let d = 2.0f32;
    let tan_h = (22.0f32).to_radians().tan(); // fov 44 / 2
    let aspect = 1080.0f32 / 1920.0;
    let cy = -0.395833f32 * tan_h * d;
    let hw = aspect * tan_h * d;
    let hh = (300.0f32 / 1920.0) * tan_h * d;
    let fwd = norm3(sub3(TGT, EYE));
    let right = norm3(cross3(fwd, [0.0, 1.0, 0.0]));
    let upv = cross3(right, fwd);
    let corner = |sx: f32, sy: f32| -> [f32; 3] {
        [
            EYE[0] + right[0] * hw * sx + upv[0] * (cy + hh * sy) + fwd[0] * d,
            EYE[1] + right[1] * hw * sx + upv[1] * (cy + hh * sy) + fwd[1] * d,
            EYE[2] + right[2] * hw * sx + upv[2] * (cy + hh * sy) + fwd[2] * d,
        ]
    };
    let mut g = TexGeo::new();
    g.vert(corner(-1.0, -1.0), [0.0, 1.0]);
    g.vert(corner(1.0, -1.0), [1.0, 1.0]);
    g.vert(corner(1.0, 1.0), [1.0, 0.0]);
    g.vert(corner(-1.0, 1.0), [0.0, 0.0]);
    g.tri(0, 1, 2);
    g.tri(0, 2, 3);
    TexDraw { geo: g, model: math::identity(), tex: TEX_CAPTION, repeat: false }
}

// ---------------------------------------------------------------------
// Rendering (M5): the full production chain — shadow pass, bokeh
// depth prepass, the PBR scene pass into HalfFloat MSAA-4x, bokeh,
// bloom, output, GRADE, and the caption composited last in sRGB
// space. Textures still carry no mipmaps (three.js builds them;
// logged in gaps.md).
// ---------------------------------------------------------------------

struct Uniforms {
    mvp: Mat4,
    model: Mat4,
}

fn mat_bytes(m: &Mat4) -> [u8; 64] {
    let mut out = [0u8; 64];
    let mut k = 0;
    for col in 0..4 {
        for row in 0..4 {
            out[k..k + 4].copy_from_slice(&m[col][row].to_ne_bytes());
            k += 4;
        }
    }
    out
}

fn uniform_bytes(u: &Uniforms) -> Vec<u8> {
    let mut out = Vec::with_capacity(128);
    out.extend_from_slice(&mat_bytes(&u.mvp));
    out.extend_from_slice(&mat_bytes(&u.model));
    out
}

fn pack_f32(data: &[f32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len() * 4);
    for v in data {
        out.extend_from_slice(&v.to_ne_bytes());
    }
    out
}

fn pack_u32(data: &[u32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len() * 4);
    for v in data {
        out.extend_from_slice(&v.to_ne_bytes());
    }
    out
}

pub async fn run_ab1(width: u32, height: u32, out: &str, caption_path: &str, post_stage: u8, knobs: crate::Knobs) -> crate::errors::TaleResult<()> {
    let tag = if knobs.is_draft() {
        format!(" {}", knobs.tag())
    } else {
        String::new()
    };
    let samples = knobs.samples();
    let shadow_size = knobs.shadow;
    let instance = crate::gpucore::new_instance();
    let adapter = crate::gpucore::request_adapter(&instance).await?;
    let info = adapter.get_info();
    println!(
        "adapter: {} | backend: {:?} | device type: {:?} | driver: {} {}",
        info.name, info.backend, info.device_type, info.driver, info.driver_info
    );
    let stage_name = match post_stage {
        1 => "lighting+shadows (+bokeh/output)",
        2 => "+ bloom",
        _ => "+ GRADE (full production chain)",
    };
    println!("scene: ab1 — Onoda S01 @ t=1.50 (jungle camp JG), fov 44 | M5 composer stage: {stage_name}");
    let (env_max, env_mean) = composer::room_env_stats();
    println!("room env PMREM (captured, assets/room-env-pmrem.f16): max {env_max:.4}, meanG {env_mean:.5}");

    // NOT downlevel limits: those cap textures at 2048, but the
    // real character canvas is 2560x2816 (res 2). Standard WebGPU
    // limits (8192) are what the scene needs; lavapipe admits
    // them — request_device_default's law (gpucore).
    let (device, queue) =
        crate::gpucore::request_device_default(&adapter, "talerender device").await?;

    // ---- camera: S01's extracted frame ----
    let aspect = width as f32 / height as f32;
    let proj = math::perspective(44f32.to_radians(), aspect, 0.05, 800.0);
    let view = math::look_at(EYE, TGT, [0.0, 1.0, 0.0]);
    let vp = math::mul(&proj, &view);

    // ---- the production light rig (final state after the noonJG
    // atmosphere; engine.ts lightRig + env.ts 'noon' preset) ----
    // Key: #FFF0D6 x2.6 at (7,12,5), target (0,0,-2); shadow camera
    // ortho +-17, near 0.1, far 80; bias -0.0006, normalBias 0.02.
    let key_pos = [7.0f32, 12.0, 5.0];
    let key_tgt = [0.0f32, 0.0, -2.0];
    let light_view = math::look_at(key_pos, key_tgt, [0.0, 1.0, 0.0]);
    let light_proj = math::orthographic(-17.0, 17.0, -17.0, 17.0, 0.1, 80.0);
    let bias_mat: Mat4 = [
        [0.5, 0.0, 0.0, 0.0],
        [0.0, 0.5, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.5, 0.5, 0.0, 1.0],
    ];
    let shadow_vp = math::mul(&bias_mat, &math::mul(&light_proj, &light_view));
    let key_dir = norm3(sub3(key_pos, key_tgt));
    let rim_dir = norm3([-3.0, 3.0, -3.0]); // rim at (-3,3,-3), target origin
    // Fire PointLight at t = 1.50: shots.ts fireLevel — base 4.6 with
    // two sine wobbles; constructor (color #FF9A3C, 5, distance 7,
    // decay 1.6), position (-1.35, 0.60, -2.45).
    let fire_i = (4.6 + 0.7 * (17.0f64 * 1.5).sin() + 0.5 * (7.3f64 * 1.5).sin()) as f32;
    let scale3 = |c: [f32; 3], k: f32| [c[0] * k, c[1] * k, c[2] * k];
    let v4 = |c: [f32; 3]| [c[0], c[1], c[2], 0.0];
    let mut frame_bytes: Vec<u8> = Vec::with_capacity(304);
    frame_bytes.extend_from_slice(&mat_bytes(&vp));
    frame_bytes.extend_from_slice(&mat_bytes(&shadow_vp));
    for v in [
        [EYE[0], EYE[1], EYE[2], 1.0],
        v4(scale3(hex_lin("#FFF0D6"), 2.6)),
        v4(key_dir),
        v4(scale3(hex_lin("#FFFFFF"), 0.4)),
        v4(rim_dir),
        v4(scale3(hex_lin("#CFE6FF"), 1.25)),
        v4(scale3(hex_lin("#C48A5A"), 1.25)),
        [-1.35, 0.60, -2.45, 1.0],
        v4(scale3(hex_lin("#FF9A3C"), fire_i)),
        [7.0, 1.6, 0.25, -0.0006], // distance, decay, env intensity, shadow bias
        [1.0 / shadow_size as f32, shadow_size as f32, 0.0, 0.0],
        // Integration extension fields (points 2/3, fog, camera
        // forward): all zero — point slots skipped (cutoff 0),
        // fog disabled — so this path renders exactly as before
        // the extension. See shader_pbr.wgsl FrameU.
        [0.0; 4], [0.0; 4], [0.0; 4], [0.0; 4], [0.0; 4], [0.0; 4],
        [0.0; 4], [0.0; 4], [0.0; 4],
    ] {
        for x in v {
            frame_bytes.extend_from_slice(&x.to_ne_bytes());
        }
    }
    let frame_ub = {
        let buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("frame uniforms"),
            size: frame_bytes.len() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        crate::gpucore::mem_alloc_buffer(crate::gpucore::MemClass::G, frame_bytes.len() as u64); // VRAM ledger (W84 §5.2)
        queue.write_buffer(&buf, 0, &frame_bytes);
        buf
    };

    // ---- textures ----
    let caption_img = image::open(caption_path)
        .map_err(|e| crate::errors::TaleError::data_load(caption_path, e))?
        .to_rgba8();
    let (cap_w, cap_h) = caption_img.dimensions();
    let textures_px: [(Vec<u8>, u32, u32); 6] = [
        paint_sky("#6E9CC0", "#D8E8DA", "#BFE4F7"),
        paint_grass("#5B9A48"),
        paint_cloud(),
        paint_blob(),
        paint_onoda(),
        // The hills' map: T.grass('#5E8F52', [6, 3]) — the grass
        // painter with the hills' own base colour.
        paint_grass("#5E8F52"),
    ];
    let upload = |px: &[u8], w: u32, h: u32, format: wgpu::TextureFormat| -> wgpu::Texture {
        let tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("scene texture"),
            size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        // VRAM ledger (W84 §5.2): class E upload — every ab1
        // scene texture flows through this closure.
        crate::gpucore::mem_alloc_texture(crate::gpucore::MemClass::E, crate::gpucore::tex_bytes(w, h, format, 1));
        queue.write_texture(
            wgpu::TexelCopyTextureInfo { texture: &tex, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
            px,
            wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(w * 4), rows_per_image: Some(h) },
            wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        );
        tex
    };
    let mut tex_views: Vec<wgpu::TextureView> = Vec::new();
    let mut tex_keep: Vec<wgpu::Texture> = Vec::new();
    for (px, w, h) in &textures_px {
        let t = upload(px, *w, *h, wgpu::TextureFormat::Rgba8UnormSrgb);
        tex_views.push(t.create_view(&wgpu::TextureViewDescriptor::default()));
        tex_keep.push(t);
    }
    // The caption is composited AFTER the grade pass, in sRGB space:
    // its texture is sampled raw (Unorm), never decoded.
    let caption_tex = upload(&caption_img.into_raw(), cap_w, cap_h, wgpu::TextureFormat::Rgba8Unorm);
    let caption_view = caption_tex.create_view(&wgpu::TextureViewDescriptor::default());
    let env_tex = composer::load_room_env(&device, &queue);
    let env_view = env_tex.create_view(&wgpu::TextureViewDescriptor::default());
    // 1x1 white dummy for the PBR map slot of untextured draws.
    let dummy_tex = upload(&[255u8, 255, 255, 255], 1, 1, wgpu::TextureFormat::Rgba8UnormSrgb);
    let dummy_view = dummy_tex.create_view(&wgpu::TextureViewDescriptor::default());

    let sampler_clamp = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("clamp sampler"),
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        ..Default::default()
    });
    let sampler_repeat = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("repeat sampler"),
        address_mode_u: wgpu::AddressMode::Repeat,
        address_mode_v: wgpu::AddressMode::Repeat,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        ..Default::default()
    });
    let sampler_nearest = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("nearest sampler"),
        mag_filter: wgpu::FilterMode::Nearest,
        min_filter: wgpu::FilterMode::Nearest,
        ..Default::default()
    });

    // ---- shared layouts ----
    let ubo_layout = |label: &str, vis: wgpu::ShaderStages| {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some(label),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: vis,
                ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None },
                count: None,
            }],
        })
    };
    let bgl_unif = ubo_layout("uniform layout", wgpu::ShaderStages::VERTEX);
    let tex_bgl = |with_uniform: bool| {
        let mut entries = Vec::new();
        if with_uniform {
            entries.push(wgpu::BindGroupLayoutEntry {
                binding: 0,
                // shader_basic reads its uniform in both stages (mvp in
                // the vertex, tint/alphaTest in the fragment).
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None },
                count: None,
            });
        }
        let base = if with_uniform { 1 } else { 0 };
        entries.push(wgpu::BindGroupLayoutEntry {
            binding: base,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: true }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false },
            count: None,
        });
        entries.push(wgpu::BindGroupLayoutEntry {
            binding: base + 1,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
            count: None,
        });
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor { label: Some("tex layout"), entries: &entries })
    };
    let bgl_basic = tex_bgl(true); // shader_basic: ubo 0, tex 1, smp 2
    let bgl_shadow_card = tex_bgl(true);

    let pbr_bgl0 = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("pbr uniforms layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None }, count: None },
            wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None }, count: None },
        ],
    });
    let pbr_bgl1 = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("pbr textures layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: true }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }, count: None },
            wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering), count: None },
            wgpu::BindGroupLayoutEntry { binding: 2, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: true }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }, count: None },
            wgpu::BindGroupLayoutEntry { binding: 3, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: true }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }, count: None },
            wgpu::BindGroupLayoutEntry { binding: 4, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering), count: None },
        ],
    });

    // ---- targets ----
    let shadow_map = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("key shadow map"),
        size: wgpu::Extent3d { width: shadow_size, height: shadow_size, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    crate::gpucore::mem_alloc_texture(crate::gpucore::MemClass::B, crate::gpucore::tex_bytes(shadow_size, shadow_size, wgpu::TextureFormat::Rgba8Unorm, 1)); // VRAM ledger (W84 §5.2)
    let shadow_map_view = shadow_map.create_view(&wgpu::TextureViewDescriptor::default());
    let shadow_depth = composer::make_rt(&device, shadow_size, shadow_size, wgpu::TextureFormat::Depth32Float, "shadow depth");
    crate::gpucore::mem_alloc_texture(crate::gpucore::MemClass::B, crate::gpucore::tex_bytes(shadow_size, shadow_size, wgpu::TextureFormat::Depth32Float, 1)); // VRAM ledger (W84 §5.2)
    let shadow_depth_view = shadow_depth.create_view(&wgpu::TextureViewDescriptor::default());

    let depth_format = if std::env::var("TALERENDER_DOF_DEPTH_FORMAT").is_ok_and(|v| v == "halffloat") {
        wgpu::TextureFormat::Rgba16Float
    } else {
        wgpu::TextureFormat::Rgba8Unorm
    };
    let packed_depth = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("packed depth"),
        size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: depth_format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    crate::gpucore::mem_alloc_texture(crate::gpucore::MemClass::A, crate::gpucore::tex_bytes(width, height, depth_format, 1)); // VRAM ledger (W84 §5.2)
    let packed_view = packed_depth.create_view(&wgpu::TextureViewDescriptor::default());
    let prepass_depth = composer::make_rt(&device, width, height, wgpu::TextureFormat::Depth32Float, "prepass depth");
    crate::gpucore::mem_alloc_texture(crate::gpucore::MemClass::A, crate::gpucore::tex_bytes(width, height, wgpu::TextureFormat::Depth32Float, 1)); // VRAM ledger (W84 §5.2)
    let prepass_depth_view = prepass_depth.create_view(&wgpu::TextureViewDescriptor::default());

    let hdr_msaa = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("hdr msaa"),
        size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: samples,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba16Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    crate::gpucore::mem_alloc_texture(crate::gpucore::MemClass::A, crate::gpucore::tex_bytes(width, height, wgpu::TextureFormat::Rgba16Float, samples)); // VRAM ledger (W84 §5.2)
    let hdr_msaa_view = hdr_msaa.create_view(&wgpu::TextureViewDescriptor::default());
    let hdr_tex = composer::make_rt(&device, width, height, wgpu::TextureFormat::Rgba16Float, "hdr resolved");
    crate::gpucore::mem_alloc_texture(crate::gpucore::MemClass::A, crate::gpucore::tex_bytes(width, height, wgpu::TextureFormat::Rgba16Float, 1)); // VRAM ledger (W84 §5.2)
    let hdr_view = hdr_tex.create_view(&wgpu::TextureViewDescriptor::default());
    let hdr_depth = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("hdr depth"),
        size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: samples,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    crate::gpucore::mem_alloc_texture(crate::gpucore::MemClass::A, crate::gpucore::tex_bytes(width, height, wgpu::TextureFormat::Depth32Float, samples)); // VRAM ledger (W84 §5.2)
    let hdr_depth_view = hdr_depth.create_view(&wgpu::TextureViewDescriptor::default());
    let bokeh_tex = composer::make_rt(&device, width, height, wgpu::TextureFormat::Rgba16Float, "bokeh out");
    crate::gpucore::mem_alloc_texture(crate::gpucore::MemClass::A, crate::gpucore::tex_bytes(width, height, wgpu::TextureFormat::Rgba16Float, 1)); // VRAM ledger (W84 §5.2)
    let bokeh_view = bokeh_tex.create_view(&wgpu::TextureViewDescriptor::default());
    let srgb_tex = composer::make_rt(&device, width, height, wgpu::TextureFormat::Rgba8Unorm, "output srgb");
    crate::gpucore::mem_alloc_texture(crate::gpucore::MemClass::A, crate::gpucore::tex_bytes(width, height, wgpu::TextureFormat::Rgba8Unorm, 1)); // VRAM ledger (W84 §5.2)
    let srgb_view = srgb_tex.create_view(&wgpu::TextureViewDescriptor::default());
    let (final_tex, final_view) = crate::gpucore::make_target_viewed(
        &device,
        width,
        height,
        wgpu::TextureFormat::Rgba8Unorm,
        1,
        wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        "final",
    );
    crate::gpucore::mem_alloc_texture(crate::gpucore::MemClass::A, crate::gpucore::tex_bytes(width, height, wgpu::TextureFormat::Rgba8Unorm, 1)); // VRAM ledger (W84 §5.2)

    // ---- pipelines ----
    let module = |label: &str, src: &str| {
        device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(label),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(src)),
        })
    };
    let colour_attrs = [
        wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x3, offset: 0, shader_location: 0 },
        wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x3, offset: 12, shader_location: 1 },
        wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x3, offset: 24, shader_location: 2 },
    ];
    let tex_attrs = [
        wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x3, offset: 0, shader_location: 0 },
        wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x2, offset: 12, shader_location: 1 },
    ];
    let pos36_attrs = [wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x3, offset: 0, shader_location: 0 }];
    let depth_state = |write: bool| {
        Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: Some(write),
            depth_compare: Some(wgpu::CompareFunction::Less),
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        })
    };

    // PBR pipelines (colour + textured variants).
    let shader_pbr = module("pbr shader", include_str!("shader_pbr.wgsl"));
    let pbr_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("pbr layout"),
        bind_group_layouts: &[Some(&pbr_bgl0), Some(&pbr_bgl1)],
        immediate_size: 0,
    });
    let make_pbr = |entry: &str, buffers: &[Option<wgpu::VertexBufferLayout>]| {
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("pbr pipeline"),
            layout: Some(&pbr_layout),
            vertex: wgpu::VertexState { module: &shader_pbr, entry_point: Some(entry), buffers, compilation_options: Default::default() },
            fragment: Some(wgpu::FragmentState {
                module: &shader_pbr,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState { format: wgpu::TextureFormat::Rgba16Float, blend: Some(wgpu::BlendState::REPLACE), write_mask: wgpu::ColorWrites::ALL })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState { cull_mode: Some(wgpu::Face::Back), ..Default::default() },
            depth_stencil: depth_state(true),
            multisample: wgpu::MultisampleState { count: samples, ..Default::default() },
            multiview_mask: None,
        })
    };
    let pbr_pipeline = make_pbr("vs_colour", &[Some(wgpu::VertexBufferLayout { array_stride: 36, step_mode: wgpu::VertexStepMode::Vertex, attributes: &colour_attrs })]);
    let pbr_tex_pipeline = make_pbr("vs_tex", &[Some(wgpu::VertexBufferLayout { array_stride: 20, step_mode: wgpu::VertexStepMode::Vertex, attributes: &tex_attrs })]);
    let map_attrs = [
        wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x3, offset: 0, shader_location: 0 },
        wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x3, offset: 12, shader_location: 1 },
        wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x3, offset: 24, shader_location: 2 },
        wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x2, offset: 36, shader_location: 3 },
    ];
    let pbr_map_pipeline = make_pbr("vs_map", &[Some(wgpu::VertexBufferLayout { array_stride: 44, step_mode: wgpu::VertexStepMode::Vertex, attributes: &map_attrs })]);

    // Basic (unlit) pipelines.
    let shader_basic = module("basic shader", include_str!("shader_basic.wgsl"));
    let basic_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("basic layout"),
        bind_group_layouts: &[Some(&bgl_basic)],
        immediate_size: 0,
    });
    let make_basic = |format: wgpu::TextureFormat, depth: Option<wgpu::DepthStencilState>, samples: u32| {
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("basic pipeline"),
            layout: Some(&basic_layout),
            vertex: wgpu::VertexState {
                module: &shader_basic,
                entry_point: Some("vs_main"),
                buffers: &[Some(wgpu::VertexBufferLayout { array_stride: 20, step_mode: wgpu::VertexStepMode::Vertex, attributes: &tex_attrs })],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader_basic,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState { format, blend: Some(wgpu::BlendState::ALPHA_BLENDING), write_mask: wgpu::ColorWrites::ALL })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState { cull_mode: Some(wgpu::Face::Back), ..Default::default() },
            depth_stencil: depth,
            multisample: wgpu::MultisampleState { count: samples, ..Default::default() },
            multiview_mask: None,
        })
    };
    let basic_write = make_basic(wgpu::TextureFormat::Rgba16Float, depth_state(true), samples);
    let basic_nowrite = make_basic(wgpu::TextureFormat::Rgba16Float, depth_state(false), samples);
    let basic_overlay = make_basic(wgpu::TextureFormat::Rgba8Unorm, None, 1);

    // Depth pipelines: prepass (camera) + shadow (light).
    let shader_depth_pos = module("depth pos shader", include_str!("shader_depth_pos.wgsl"));
    let shader_depth = module("depth shader", include_str!("shader_depth.wgsl"));
    let shader_shadow_card = module("shadow card shader", include_str!("shader_shadow_card.wgsl"));
    let depth_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("depth layout"),
        bind_group_layouts: &[Some(&bgl_unif)],
        immediate_size: 0,
    });
    let make_depth = |shader: &wgpu::ShaderModule, buffers: &[Option<wgpu::VertexBufferLayout>], format: wgpu::TextureFormat, layout: &wgpu::PipelineLayout| {
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("depth pipeline"),
            layout: Some(layout),
            vertex: wgpu::VertexState { module: shader, entry_point: Some("vs_main"), buffers, compilation_options: Default::default() },
            fragment: Some(wgpu::FragmentState {
                module: shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState { format, blend: Some(wgpu::BlendState::REPLACE), write_mask: wgpu::ColorWrites::ALL })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState { cull_mode: Some(wgpu::Face::Back), ..Default::default() },
            depth_stencil: depth_state(true),
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
        })
    };
    let prepass_geo = make_depth(&shader_depth_pos, &[Some(wgpu::VertexBufferLayout { array_stride: 36, step_mode: wgpu::VertexStepMode::Vertex, attributes: &pos36_attrs })], depth_format, &depth_layout);
    let prepass_map = make_depth(&shader_depth_pos, &[Some(wgpu::VertexBufferLayout { array_stride: 44, step_mode: wgpu::VertexStepMode::Vertex, attributes: &pos36_attrs })], depth_format, &depth_layout);
    let prepass_tex = make_depth(&shader_depth, &[Some(wgpu::VertexBufferLayout { array_stride: 20, step_mode: wgpu::VertexStepMode::Vertex, attributes: &tex_attrs })], depth_format, &depth_layout);
    let shadow_geo = make_depth(&shader_depth_pos, &[Some(wgpu::VertexBufferLayout { array_stride: 36, step_mode: wgpu::VertexStepMode::Vertex, attributes: &pos36_attrs })], wgpu::TextureFormat::Rgba8Unorm, &depth_layout);
    let shadow_card_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("shadow card layout"),
        bind_group_layouts: &[Some(&bgl_shadow_card)],
        immediate_size: 0,
    });
    let shadow_card = make_depth(&shader_shadow_card, &[Some(wgpu::VertexBufferLayout { array_stride: 20, step_mode: wgpu::VertexStepMode::Vertex, attributes: &tex_attrs })], wgpu::TextureFormat::Rgba8Unorm, &shadow_card_layout);

    // Bokeh pipeline (M3's shader + uniform layout, focus = S01's
    // racked 6.051 m, G.jungle aperture/maxblur).
    let bokeh_uniform_data: [f32; 8] = [6.051, aspect, 0.0012, 0.007, 0.05, 800.0, 0.0, 0.0];
    let bokeh_ub = {
        let buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("bokeh uniforms"),
            size: 32,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        crate::gpucore::mem_alloc_buffer(crate::gpucore::MemClass::G, 32); // VRAM ledger (W84 §5.2)
        queue.write_buffer(&buf, 0, &pack_f32(&bokeh_uniform_data));
        buf
    };
    let shader_bokeh = module("bokeh shader", include_str!("shader_bokeh.wgsl"));
    let bokeh_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("bokeh layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None }, count: None },
            wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: true }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }, count: None },
            wgpu::BindGroupLayoutEntry { binding: 2, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering), count: None },
            wgpu::BindGroupLayoutEntry { binding: 3, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: true }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }, count: None },
            wgpu::BindGroupLayoutEntry { binding: 4, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering), count: None },
        ],
    });
    let bokeh_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("bokeh bind group"),
        layout: &bokeh_bgl,
        entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: bokeh_ub.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&hdr_view) },
            wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&sampler_clamp) },
            wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::TextureView(&packed_view) },
            wgpu::BindGroupEntry { binding: 4, resource: wgpu::BindingResource::Sampler(&sampler_nearest) },
        ],
    });
    let bokeh_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("bokeh pipeline"),
        layout: Some(&device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("bokeh layout"),
            bind_group_layouts: &[Some(&bokeh_bgl)],
            immediate_size: 0,
        })),
        vertex: wgpu::VertexState { module: &shader_bokeh, entry_point: Some("vs_main"), buffers: &[], compilation_options: Default::default() },
        fragment: Some(wgpu::FragmentState {
            module: &shader_bokeh,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState { format: wgpu::TextureFormat::Rgba16Float, blend: Some(wgpu::BlendState::REPLACE), write_mask: wgpu::ColorWrites::ALL })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
    });

    let post = PostChain::new(&device);

    // ---- geometry preparation ----
    let scene = build_scene();
    let make_vb = |verts: &[f32], idx: &[u32]| -> (wgpu::Buffer, wgpu::Buffer, u32) {
        let vb = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("vertex buffer"),
            size: (verts.len() * 4) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        crate::gpucore::mem_alloc_buffer(crate::gpucore::MemClass::G, (verts.len() * 4) as u64); // VRAM ledger (W84 §5.2)
        queue.write_buffer(&vb, 0, &pack_f32(verts));
        let ib = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("index buffer"),
            size: (idx.len() * 4) as u64,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        crate::gpucore::mem_alloc_buffer(crate::gpucore::MemClass::G, (idx.len() * 4) as u64); // VRAM ledger (W84 §5.2)
        queue.write_buffer(&ib, 0, &pack_u32(idx));
        (vb, ib, idx.len() as u32)
    };
    let make_ub = |bytes: &[u8], label: &str| -> wgpu::Buffer {
        let buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size: bytes.len() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        crate::gpucore::mem_alloc_buffer(crate::gpucore::MemClass::G, bytes.len() as u64); // VRAM ledger (W84 §5.2)
        queue.write_buffer(&buf, 0, bytes);
        buf
    };
    let depth_ub = |mvp: &Mat4, model: &Mat4| -> wgpu::Buffer {
        make_ub(&uniform_bytes(&Uniforms { mvp: *mvp, model: *model }), "depth uniforms")
    };

    // Shared PBR texture bind groups (group 1): untextured draws bind
    // the dummy map; the ground binds the grass texture (repeat).
    let pbr_tex_bg = |map: &wgpu::TextureView, smp: &wgpu::Sampler| {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("pbr textures"),
            layout: &pbr_bgl1,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(map) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(smp) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(&shadow_map_view) },
                wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::TextureView(&env_view) },
                wgpu::BindGroupEntry { binding: 4, resource: wgpu::BindingResource::Sampler(&sampler_clamp) },
            ],
        })
    };
    let pbr_bg_std = pbr_tex_bg(&dummy_view, &sampler_clamp);
    let pbr_bg_ground = pbr_tex_bg(&tex_views[TEX_GRASS], &sampler_repeat);
    let pbr_bg_hill = pbr_tex_bg(&tex_views[TEX_HILLGRASS], &sampler_repeat);

    struct PreparedStd {
        vb: wgpu::Buffer,
        ib: wgpu::Buffer,
        n: u32,
        bg0: wgpu::BindGroup,
        prepass_bg: wgpu::BindGroup,
        shadow_bg: Option<wgpu::BindGroup>,
        _ubs: Vec<wgpu::Buffer>,
    }
    let mut std_prepared: Vec<PreparedStd> = Vec::new();
    for d in &scene.std {
        let (vb, ib, n) = make_vb(&d.geo.verts, &d.geo.idx);
        let mut draw_bytes: Vec<u8> = Vec::with_capacity(176);
        draw_bytes.extend_from_slice(&mat_bytes(&d.model));
        draw_bytes.extend_from_slice(&mat_bytes(&math::normal_matrix(&d.model)));
        for v in [
            [d.mat.rough, d.mat.metal, if d.receive { 1.0 } else { 0.0 }, 0.0],
            [d.mat.emissive[0], d.mat.emissive[1], d.mat.emissive[2], 0.0],
            [0.0, 1.0, 0.0, 0.0],
        ] {
            for x in v {
                draw_bytes.extend_from_slice(&x.to_ne_bytes());
            }
        }
        let draw_ub = make_ub(&draw_bytes, "pbr draw uniforms");
        let bg0 = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("pbr uniforms bind group"),
            layout: &pbr_bgl0,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: frame_ub.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: draw_ub.as_entire_binding() },
            ],
        });
        let pre_ub = depth_ub(&math::mul(&vp, &d.model), &d.model);
        let prepass_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("prepass bind group"),
            layout: &bgl_unif,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: pre_ub.as_entire_binding() }],
        });
        let mut ubs = vec![draw_ub, pre_ub];
        let shadow_bg = if d.cast {
            let sh_ub = depth_ub(&math::mul(&shadow_vp, &d.model), &d.model);
            let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("shadow bind group"),
                layout: &bgl_unif,
                entries: &[wgpu::BindGroupEntry { binding: 0, resource: sh_ub.as_entire_binding() }],
            });
            ubs.push(sh_ub);
            Some(bg)
        } else {
            None
        };
        std_prepared.push(PreparedStd { vb, ib, n, bg0, prepass_bg, shadow_bg, _ubs: ubs });
    }

    // Mapped draws (the hills): same uniform shape, has_map = 1.
    struct PreparedMap {
        vb: wgpu::Buffer,
        ib: wgpu::Buffer,
        n: u32,
        bg0: wgpu::BindGroup,
        prepass_bg: wgpu::BindGroup,
        _ubs: Vec<wgpu::Buffer>,
    }
    let mut mapped_prepared: Vec<PreparedMap> = Vec::new();
    for d in &scene.mapped {
        let (vb, ib, n) = make_vb(&d.geo.verts, &d.geo.idx);
        let mut draw_bytes: Vec<u8> = Vec::with_capacity(176);
        draw_bytes.extend_from_slice(&mat_bytes(&d.model));
        draw_bytes.extend_from_slice(&mat_bytes(&math::normal_matrix(&d.model)));
        for v in [
            [d.mat.rough, d.mat.metal, if d.receive { 1.0 } else { 0.0 }, 1.0],
            [d.mat.emissive[0], d.mat.emissive[1], d.mat.emissive[2], 0.0],
            [0.0, 1.0, 0.0, 0.0],
        ] {
            for x in v {
                draw_bytes.extend_from_slice(&x.to_ne_bytes());
            }
        }
        let draw_ub = make_ub(&draw_bytes, "mapped draw uniforms");
        let bg0 = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("mapped uniforms bind group"),
            layout: &pbr_bgl0,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: frame_ub.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: draw_ub.as_entire_binding() },
            ],
        });
        let pre_ub = depth_ub(&math::mul(&vp, &d.model), &d.model);
        let prepass_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("mapped prepass bind group"),
            layout: &bgl_unif,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: pre_ub.as_entire_binding() }],
        });
        mapped_prepared.push(PreparedMap { vb, ib, n, bg0, prepass_bg, _ubs: vec![draw_ub, pre_ub] });
    }

    // Ground: the one textured standard surface (grass map, +Y
    // normal, receive-only).
    struct PreparedGround {
        vb: wgpu::Buffer,
        ib: wgpu::Buffer,
        n: u32,
        bg0: wgpu::BindGroup,
        prepass_bg: wgpu::BindGroup,
        _ubs: Vec<wgpu::Buffer>,
    }
    let ground_prepared = {
        let d = &scene.grass;
        let (vb, ib, n) = make_vb(&d.geo.verts, &d.geo.idx);
        let mut draw_bytes: Vec<u8> = Vec::with_capacity(176);
        draw_bytes.extend_from_slice(&mat_bytes(&d.model));
        draw_bytes.extend_from_slice(&mat_bytes(&math::normal_matrix(&d.model)));
        for v in [
            [0.85f32, 0.0, 1.0, 1.0], // roughness, metalness, receive, has_map
            [0.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0], // const normal +Y
        ] {
            for x in v {
                draw_bytes.extend_from_slice(&x.to_ne_bytes());
            }
        }
        let draw_ub = make_ub(&draw_bytes, "ground uniforms");
        let bg0 = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ground uniforms bind group"),
            layout: &pbr_bgl0,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: frame_ub.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: draw_ub.as_entire_binding() },
            ],
        });
        let pre_ub = depth_ub(&math::mul(&vp, &d.model), &d.model);
        let prepass_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ground prepass bind group"),
            layout: &bgl_unif,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: pre_ub.as_entire_binding() }],
        });
        PreparedGround { vb, ib, n, bg0, prepass_bg, _ubs: vec![draw_ub, pre_ub] }
    };

    // Basic draws (sky, clouds, blob, card, caption).
    struct PreparedBasic {
        vb: wgpu::Buffer,
        ib: wgpu::Buffer,
        n: u32,
        bg: wgpu::BindGroup,
        prepass_bg: Option<wgpu::BindGroup>,
        _ubs: Vec<wgpu::Buffer>,
    }
    let prepare_basic = |d: &TexDraw, tint: [f32; 3], opacity: f32, alpha_test: f32, tex_view: &wgpu::TextureView, prepass: bool| -> PreparedBasic {
        let (vb, ib, n) = make_vb(&d.geo.verts, &d.geo.idx);
        let mut bytes: Vec<u8> = Vec::with_capacity(192);
        bytes.extend_from_slice(&mat_bytes(&math::mul(&vp, &d.model)));
        for v in [[tint[0], tint[1], tint[2], opacity], [alpha_test, 0.0, 0.0, 0.0]] {
            for x in v {
                bytes.extend_from_slice(&x.to_ne_bytes());
            }
        }
        // Integration extension: model-view + fog fields, fog
        // disabled on this path (zeros) — see shader_basic.wgsl.
        bytes.extend_from_slice(&mat_bytes(&math::mul(&view, &d.model)));
        bytes.extend_from_slice(&[0u8; 32]);
        let ub = make_ub(&bytes, "basic uniforms");
        let sampler = if d.repeat { &sampler_repeat } else { &sampler_clamp };
        let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("basic bind group"),
            layout: &bgl_basic,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: ub.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(tex_view) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(sampler) },
            ],
        });
        let mut ubs = vec![ub];
        let prepass_bg = if prepass {
            let pre_ub = depth_ub(&math::mul(&vp, &d.model), &d.model);
            let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("basic prepass bind group"),
                layout: &bgl_unif,
                entries: &[wgpu::BindGroupEntry { binding: 0, resource: pre_ub.as_entire_binding() }],
            });
            ubs.push(pre_ub);
            Some(bg)
        } else {
            None
        };
        PreparedBasic { vb, ib, n, bg, prepass_bg, _ubs: ubs }
    };
    let sky_p = prepare_basic(&scene.sky, [1.0, 1.0, 1.0], 1.0, 0.0, &tex_views[TEX_SKY], true);
    let clouds_p: Vec<PreparedBasic> = scene
        .clouds
        .iter()
        .map(|c| prepare_basic(c, hex_lin("#EFF4EA"), 1.0, 0.0, &tex_views[TEX_CLOUD], false))
        .collect();
    let blob_p = prepare_basic(&scene.blob, [1.0, 1.0, 1.0], 0.55, 0.0, &tex_views[TEX_BLOB], true);
    let card_p = prepare_basic(&scene.card, [1.0, 1.0, 1.0], 1.0, 0.04, &tex_views[TEX_CARD], true);
    let caption_p = prepare_basic(&scene.caption, [1.0, 1.0, 1.0], 1.0, 0.0, &caption_view, false);
    // The card's shadow uses its customDepthMaterial (alphaTest 0.5).
    let card_shadow_bg = {
        let ub = depth_ub(&math::mul(&shadow_vp, &scene.card.model), &scene.card.model);
        let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("card shadow bind group"),
            layout: &bgl_shadow_card,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: ub.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&tex_views[TEX_CARD]) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&sampler_clamp) },
            ],
        });
        (bg, ub)
    };

    // ---- passes ----
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("frame encoder"),
    });
    // 1. Key-light shadow map (casters only; cleared to packed far).
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("shadow pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment { depth_slice: None,
                view: &shadow_map_view,
                resolve_target: None,
                ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color { r: 1.0, g: 1.0, b: 1.0, a: 1.0 }), store: wgpu::StoreOp::Store },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &shadow_depth_view,
                depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(1.0), store: wgpu::StoreOp::Store }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&shadow_geo);
        for p in std_prepared.iter().filter(|p| p.shadow_bg.is_some()) {
            pass.set_bind_group(0, p.shadow_bg.as_ref().unwrap(), &[]);
            pass.set_vertex_buffer(0, p.vb.slice(..));
            pass.set_index_buffer(p.ib.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..p.n, 0, 0..1);
        }
        pass.set_pipeline(&shadow_card);
        pass.set_bind_group(0, &card_shadow_bg.0, &[]);
        pass.set_vertex_buffer(0, card_p.vb.slice(..));
        pass.set_index_buffer(card_p.ib.slice(..), wgpu::IndexFormat::Uint32);
        pass.draw_indexed(0..card_p.n, 0, 0..1);
    }
    // 2. Bokeh depth prepass: three's override material draws every
    // Mesh (the card as its full quad, the blob, the sky) and skips
    // Sprites (the clouds). Cleared to packed white (far).
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("depth prepass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment { depth_slice: None,
                view: &packed_view,
                resolve_target: None,
                ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color { r: 1.0, g: 1.0, b: 1.0, a: 1.0 }), store: wgpu::StoreOp::Store },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &prepass_depth_view,
                depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(1.0), store: wgpu::StoreOp::Store }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&prepass_geo);
        for p in &std_prepared {
            pass.set_bind_group(0, &p.prepass_bg, &[]);
            pass.set_vertex_buffer(0, p.vb.slice(..));
            pass.set_index_buffer(p.ib.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..p.n, 0, 0..1);
        }
        pass.set_pipeline(&prepass_map);
        for p in &mapped_prepared {
            pass.set_bind_group(0, &p.prepass_bg, &[]);
            pass.set_vertex_buffer(0, p.vb.slice(..));
            pass.set_index_buffer(p.ib.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..p.n, 0, 0..1);
        }
        pass.set_pipeline(&prepass_tex);
        pass.set_bind_group(0, &ground_prepared.prepass_bg, &[]);
        pass.set_vertex_buffer(0, ground_prepared.vb.slice(..));
        pass.set_index_buffer(ground_prepared.ib.slice(..), wgpu::IndexFormat::Uint32);
        pass.draw_indexed(0..ground_prepared.n, 0, 0..1);
        for b in [&sky_p, &blob_p, &card_p] {
            pass.set_bind_group(0, b.prepass_bg.as_ref().unwrap(), &[]);
            pass.set_vertex_buffer(0, b.vb.slice(..));
            pass.set_index_buffer(b.ib.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..b.n, 0, 0..1);
        }
    }
    // 3. HDR scene pass (MSAA 4x, resolved): sky -> standard draws ->
    // transparent basics back-to-front (clouds, blob, card).
    {
        // Phase 2 (W54 s4 shape): view/resolve chosen together;
        // field-for-field the pre-knob attachment at samples > 1.
        let scene_attachment = wgpu::RenderPassColorAttachment { depth_slice: None,
            view: if samples > 1 { &hdr_msaa_view } else { &hdr_view },
            resolve_target: if samples > 1 { Some(&hdr_view) } else { None },
            ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 }), store: wgpu::StoreOp::Store },
        };
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("ab1 scene pass"),
            color_attachments: &[Some(scene_attachment)],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &hdr_depth_view,
                depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(1.0), store: wgpu::StoreOp::Store }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&basic_nowrite);
        pass.set_bind_group(0, &sky_p.bg, &[]);
        pass.set_vertex_buffer(0, sky_p.vb.slice(..));
        pass.set_index_buffer(sky_p.ib.slice(..), wgpu::IndexFormat::Uint32);
        pass.draw_indexed(0..sky_p.n, 0, 0..1);
        pass.set_pipeline(&pbr_tex_pipeline);
        pass.set_bind_group(0, &ground_prepared.bg0, &[]);
        pass.set_bind_group(1, &pbr_bg_ground, &[]);
        pass.set_vertex_buffer(0, ground_prepared.vb.slice(..));
        pass.set_index_buffer(ground_prepared.ib.slice(..), wgpu::IndexFormat::Uint32);
        pass.draw_indexed(0..ground_prepared.n, 0, 0..1);
        pass.set_pipeline(&pbr_pipeline);
        pass.set_bind_group(1, &pbr_bg_std, &[]);
        for p in &std_prepared {
            pass.set_bind_group(0, &p.bg0, &[]);
            pass.set_vertex_buffer(0, p.vb.slice(..));
            pass.set_index_buffer(p.ib.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..p.n, 0, 0..1);
        }
        pass.set_pipeline(&pbr_map_pipeline);
        pass.set_bind_group(1, &pbr_bg_hill, &[]);
        for p in &mapped_prepared {
            pass.set_bind_group(0, &p.bg0, &[]);
            pass.set_vertex_buffer(0, p.vb.slice(..));
            pass.set_index_buffer(p.ib.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..p.n, 0, 0..1);
        }
        pass.set_pipeline(&basic_nowrite);
        for p in clouds_p.iter().chain([&blob_p]) {
            pass.set_bind_group(0, &p.bg, &[]);
            pass.set_vertex_buffer(0, p.vb.slice(..));
            pass.set_index_buffer(p.ib.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..p.n, 0, 0..1);
        }
        pass.set_pipeline(&basic_write);
        pass.set_bind_group(0, &card_p.bg, &[]);
        pass.set_vertex_buffer(0, card_p.vb.slice(..));
        pass.set_index_buffer(card_p.ib.slice(..), wgpu::IndexFormat::Uint32);
        pass.draw_indexed(0..card_p.n, 0, 0..1);
    }
    // 4. Bokeh.
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("bokeh pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment { depth_slice: None,
                view: &bokeh_view,
                resolve_target: None,
                ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT), store: wgpu::StoreOp::Store },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&bokeh_pipeline);
        pass.set_bind_group(0, &bokeh_bg, &[]);
        pass.draw(0..3, 0..1);
    }
    // 5. Bloom (stage >= 2): G.jungle bloom 0.16, threshold default
    // 0.85, radius default 0.6 (engine.ts per-frame look application).
    if post_stage >= 2 {
        post.bloom(
            &device,
            &queue,
            &mut encoder,
            &bokeh_view,
            &bokeh_view,
            width,
            height,
            &BloomParams { threshold: 0.85, strength: 0.16, radius: 0.6 },
        );
    }
    // 6. OutputPass (exposure 1.0 — the look does not override it).
    // With GRADE (stage 3) it lands in the grade input; without, it
    // is already the final image.
    let output_target = if post_stage >= 3 { &srgb_view } else { &final_view };
    post.output(&device, &queue, &mut encoder, &bokeh_view, output_target, 1.0);
    // 7. GRADE (stage 3): G.jungle sat 0.8 / warm 0.04 / vignette
    // 0.24 / grain 0.012 at uTime = t = 1.50.
    if post_stage >= 3 {
        post.grade(
            &device,
            &queue,
            &mut encoder,
            &srgb_view,
            &final_view,
            &GradeParams { vignette: 0.24, grain: 0.012, warm: 0.04, sat: 0.8, time: 1.5 },
        );
    }
    // 8. Caption overlay: in production the caption is drawn on the
    // 2D canvas over the composited frame (main.ts), i.e. after the
    // whole chain, in sRGB space — so it composites onto the final
    // target with raw sampling and no depth.
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("caption overlay"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment { depth_slice: None,
                view: &final_view,
                resolve_target: None,
                ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&basic_overlay);
        pass.set_bind_group(0, &caption_p.bg, &[]);
        pass.set_vertex_buffer(0, caption_p.vb.slice(..));
        pass.set_index_buffer(caption_p.ib.slice(..), wgpu::IndexFormat::Uint32);
        pass.draw_indexed(0..caption_p.n, 0, 0..1);
    }

    // ---- readback ----
    let rb = crate::gpucore::Readback::record(&device, &mut encoder, &final_tex, width, height, "readback buffer");
    queue.submit(Some(encoder.finish()));
    let pixels = rb.collect(&device)?;

    let img = image::RgbaImage::from_raw(width, height, pixels).expect("pixel buffer size mismatch");
    img.save(out).map_err(|e| crate::errors::TaleError::image_io(out, e))?;
    println!("wrote {out} ({width}x{height}){tag}");
    // VRAM ledger (W84 §5.3): its own line, after the wrote line.
    println!("{}", crate::gpucore::mem_stats_line());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tri_normal_dot_centroid(verts: &[f32], idx: &[u32]) -> f32 {
        let p = |k: usize| -> [f32; 3] {
            let o = (idx[k] as usize) * 9;
            [verts[o], verts[o + 1], verts[o + 2]]
        };
        let (a, b, c) = (p(0), p(1), p(2));
        let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        let v = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
        let n = [
            u[1] * v[2] - u[2] * v[1],
            u[2] * v[0] - u[0] * v[2],
            u[0] * v[1] - u[1] * v[0],
        ];
        let cen = [(a[0] + b[0] + c[0]) / 3.0, (a[1] + b[1] + c[1]) / 3.0, (a[2] + b[2] + c[2]) / 3.0];
        n[0] * cen[0] + n[1] * cen[1] + n[2] * cen[2]
    }

    #[test]
    fn generated_shapes_face_outward() {
        // The colour pipeline culls back faces: a wrongly wound shape
        // would vanish. Scan for the first non-degenerate triangle
        // (sphere poles produce degenerate ones) and check its normal
        // points away from the centroid.
        let check = |g: &Geo, name: &str| {
            let mut found = false;
            for t in g.idx.chunks_exact(3) {
                let d = tri_normal_dot_centroid(&g.verts, t);
                if d.abs() > 1e-4 {
                    assert!(d > 0.0, "{name}: first solid triangle faces inward");
                    found = true;
                    break;
                }
            }
            assert!(found, "{name}: no non-degenerate triangle found");
        };
        check(&sphere_geo(1.0, 8, 6, [0.0; 3]), "sphere");
        check(&cyl_geo(0.3, 0.4, 1.0, 8, [0.0; 3]), "cylinder");
        check(&cone_geo(0.4, 1.0, 8, [0.0; 3]), "cone");
        check(&box_geo(1.0, 2.0, 0.5, [0.0; 3]), "box");
    }

    #[test]
    fn sky_sphere_faces_inward() {
        // The dome is viewed from inside; its triangles must face the
        // centre or the back-face cull removes the whole sky.
        let g = sky_sphere_geo(10.0, 8, 6);
        let mut found = false;
        for t in g.idx.chunks_exact(3) {
            let p = |k: usize| -> [f32; 3] {
                let o = (t[k] as usize) * 5;
                [g.verts[o], g.verts[o + 1], g.verts[o + 2]]
            };
            let (a, b, c) = (p(0), p(1), p(2));
            let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
            let v = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
            let n = [
                u[1] * v[2] - u[2] * v[1],
                u[2] * v[0] - u[0] * v[2],
                u[0] * v[1] - u[1] * v[0],
            ];
            let cen = [(a[0] + b[0] + c[0]) / 3.0, (a[1] + b[1] + c[1]) / 3.0, (a[2] + b[2] + c[2]) / 3.0];
            let d = n[0] * cen[0] + n[1] * cen[1] + n[2] * cen[2];
            if d.abs() > 1e-3 {
                assert!(d < 0.0, "sky triangle faces outward");
                found = true;
                break;
            }
        }
        assert!(found);
    }

    #[test]
    fn rnd_matches_the_js_sin_hash() {
        // rnd(i) = frac(sin(i*127.1 + 311.7) * 43758.5453)
        assert!((rnd(0.0) - 0.81931).abs() < 0.001, "rnd(0) = {}", rnd(0.0));
        for i in 0..100 {
            let v = rnd(i as f64);
            assert!((0.0..1.0).contains(&v), "rnd({i}) = {v}");
        }
    }

    #[test]
    fn onoda_card_paints_a_figure() {
        let (px, w, h) = paint_onoda();
        assert_eq!((w, h), (2560, 2816));
        // The head centre (stage 256, 178 -> canvas px) must be the
        // head colour, and a corner must stay transparent.
        let cx = ((256.0 + 384.0) * 2.0) as usize;
        let cy = ((178.0 + 256.0) * 2.0) as usize;
        let k = (cy * w as usize + cx) * 4;
        assert_eq!(&px[k..k + 3], &[247, 247, 250], "head centre should be #F7F7FA");
        assert_eq!(px[3], 0, "canvas corner should be transparent");
    }
}
