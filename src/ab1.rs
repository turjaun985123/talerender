//! §28 A/B reproduction scene: Onoda S01 (HOOK) at t = 1.50 s from the
//! delivered project ~/faceless-shorts/the-soldier-who-didnt-know
//! ("THE SOLDIER WHO DIDN'T KNOW THE WAR WAS OVER", v4-era Tall Tale).
//!
//! Every staging value in this file was extracted from the project's
//! own sources (file:line citations live in
//! docs/ab/onoda-s01-t150/README.md). The scene is re-authored with
//! TaleRender's existing M1 pipelines: coloured geometry for the 3D
//! set, CPU-painted textures for the character card, grass, clouds,
//! sky gradient and blob shadow. Where the real engine has something
//! this renderer lacks (shadow maps, the composer chain, PMREM IBL,
//! canvas-painted planks textures, icosahedron canopies), the closest
//! honest primitive stands in and the substitution is logged in
//! docs/ab/onoda-s01-t150/gaps.md — nothing is silently approximated.
//!
//! Renderer of record for this scene: wgpu on lavapipe (software
//! Vulkan) — correctness evidence only, per contract §6.2.

use crate::math::{self, Mat4};
use std::borrow::Cow;

use crate::scene::{
    box_geo, cone_geo, cross3, cyl_geo, disc_geo, flat_quad_geo, flatten_cubic, flatten_quad,
    ground_disc_geo, hex_lin, hex_rgb, norm3, paint_blob, paint_cloud, paint_grass, paint_sky,
    quad_geo, rgba_of, rnd, shade_lin, shade_rgb, sky_sphere_geo, sphere_geo, srgb_to_linear, sub3,
    ColourDraw, Geo, Painter, TexDraw, TexGeo,
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
const TEX_CAPTION: usize = 5;

// Camera at t = 1.50 (camK smootherstep k = 0.05228 through the S01
// push-in; fov 44 from G.jungle spread in the shoot() call).
const EYE: [f32; 3] = [0.83432, 2.64354, 5.28499];
const TGT: [f32; 3] = [0.35, 1.11477, -0.55];

fn t3(x: f32, y: f32, z: f32) -> Mat4 {
    math::translate(x, y, z)
}

struct Scene {
    colour: Vec<ColourDraw>,
    sky: TexDraw,
    grass: TexDraw,
    blob: TexDraw,
    clouds: Vec<TexDraw>,
    card: TexDraw,
    caption: TexDraw,
}

fn build_scene() -> Scene {
    let mut colour: Vec<ColourDraw> = Vec::new();
    let mut add = |draws: &mut Vec<ColourDraw>, geo: Geo, model: Mat4| {
        draws.push(ColourDraw { geo, model });
    };

    // ---- dirt patch (world.ts: CircleGeometry r 4.8, flat '#86714C')
    add(&mut colour, disc_geo(4.8, 28, hex_lin("#86714C")), t3(0.0, 0.012, -0.9));

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
        add(&mut colour, sphere_geo(1.0, 24, 16, col), model);
        // 6 small trees per hill (castShadow false in the original).
        for k in 0..6 {
            let a = rnd(i as f64 * 9.0 + k as f64) * 2.0 - 1.0;
            let s = (0.5 + rnd(k as f64) * 0.4) as f32 * 2.2;
            let pos = [
                hx + (a as f32) * r * 0.6,
                (h as f64 * (0.75 - a.abs() * 0.45) - 1.0) as f32,
                hz + r * 0.25,
            ];
            add_tree(&mut colour, pos, 0.0, s, "#3E7A34", true);
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
        add_tree(&mut colour, [x, 0.0, z], i as f32 * 1.7, s, col, round);
    }

    // ---- palms ----
    add_palm(&mut colour, [4.6, 0.0, -4.6], 1.05, 0.14, 0.0);
    add_palm(&mut colour, [-5.2, 0.0, -5.4], 0.9, -0.2, 1.2);
    add_palm(&mut colour, [10.5, 0.0, -11.0], 1.15, 0.1, 0.0);

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
        add(&mut colour, merged, math::identity());
    }

    // ---- camp (fire pit at FX=-1.35, FZ=-2.45) ----
    let (fx, fz) = (-1.35f32, -2.45f32);
    let stone = hex_lin("#8B8B84");
    for i in 0..8 {
        let a = (i as f32 / 8.0) * std::f32::consts::TAU;
        let r = (0.09 + rnd(i as f64) * 0.04) as f32;
        add(
            &mut colour,
            sphere_geo(r, 8, 6, stone),
            t3(fx + a.cos() * 0.44, 0.05, fz + a.sin() * 0.44),
        );
    }
    let log_col = hex_lin("#7A5A38");
    for (i, &(dx, dz)) in [(0.25f32, 0.3f32), (-0.2, -0.28), (0.05, 0.02)].iter().enumerate() {
        let model = math::mul(
            &t3(fx + dx, 0.09, fz + dz),
            &math::euler_xyz(0.0, i as f32 * 0.7, std::f32::consts::FRAC_PI_2),
        );
        add(&mut colour, cyl_geo(0.05, 0.05, 0.72, 7, log_col), model);
    }
    // Flames at t = 1.50: fireLevel scaleY f = 1.13146 / 0.86854.
    // Emissive materials in the original; flat bright colours here
    // (no emissive/bloom/tonemap in this renderer — gaps.md).
    let flame_model = math::mul(&t3(fx, 0.34, fz), &math::scale(1.0, 1.13146, 1.0));
    add(&mut colour, cone_geo(0.17, 0.52, 8, hex_lin("#FF8A30")), flame_model);
    let flame2_model = math::mul(&t3(fx + 0.03, 0.30, fz + 0.04), &math::scale(1.0, 0.86854, 1.0));
    add(&mut colour, cone_geo(0.09, 0.34, 8, hex_lin("#FFD23C")), flame2_model);
    // Seat log.
    let seat_model = math::mul(
        &t3(-0.5, 0.17, -1.8),
        &math::euler_xyz(0.0, 0.12, std::f32::consts::FRAC_PI_2),
    );
    add(&mut colour, cyl_geo(0.17, 0.17, 1.15, 9, log_col), seat_model);
    // Lean-to: two poles + plank roof (flat colour — planks texture
    // not reproduced, gaps.md).
    for x in [-3.9f32, -2.7] {
        add(&mut colour, cyl_geo(0.035, 0.035, 1.5, 6, log_col), t3(x, 0.75, -2.7));
    }
    let roof_model = math::mul(&t3(-3.3, 1.52, -2.95), &math::rotate_x(0.42));
    add(&mut colour, box_geo(1.9, 0.05, 1.5, hex_lin("#8A6B42")), roof_model);
    // Crates + barrel + bedroll (flat colours; planks maps dropped).
    add(&mut colour, box_geo(0.55, 0.55, 0.55, hex_lin("#B07A48")), t3(-2.35, 0.275, -3.15));
    let crate2_model = math::mul(&t3(-2.3, 0.55 + 0.21, -3.12), &math::rotate_y(0.4));
    add(&mut colour, box_geo(0.42, 0.42, 0.42, hex_lin("#B07A48")), crate2_model);
    add(&mut colour, cyl_geo(0.33, 0.33, 0.8, 16, hex_lin("#6E4525")), t3(-3.05, 0.4, -2.2));
    let band_col = hex_lin("#3A3A3F");
    for y in [0.12f32, 0.68] {
        add(&mut colour, cyl_geo(0.345, 0.345, 0.05, 16, band_col), t3(-3.05, y, -2.2));
    }
    let bed_model = math::mul(&t3(-3.3, 0.05, -2.1), &math::rotate_y(0.2));
    add(&mut colour, box_geo(1.5, 0.09, 0.55, hex_lin("#77805E")), bed_model);


    // ---- Onoda card + rifle: built by onoda_card_and_rifle()
    // (shared with the scene-format loader's code-ref nodes) ----
    let (card, rifle_draws) = onoda_card_and_rifle();
    for d in rifle_draws {
        colour.push(d);
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

    Scene { colour, sky, grass, blob, clouds, card, caption }
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

fn add_tree(draws: &mut Vec<ColourDraw>, pos: [f32; 3], rot_y: f32, s: f32, col: &str, round: bool) {
    let base = math::mul(&t3(pos[0], pos[1], pos[2]), &math::rotate_y(rot_y));
    let trunk = cyl_geo(0.12 * s, 0.18 * s, 1.6 * s, 7, hex_lin("#7A5234"));
    draws.push(ColourDraw {
        geo: trunk,
        model: math::mul(&base, &t3(0.0, 0.8 * s, 0.0)),
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
            draws.push(ColourDraw {
                geo: sphere_geo(f * s, 12, 9, c),
                model: math::mul(&base, &t3(ox * s, oy * s, oz * s)),
            });
        }
    } else {
        let model = math::mul(
            &math::mul(&base, &t3(0.0, 3.2 * s, 0.0)),
            &math::scale(0.7 * s, 2.6 * s, 0.7 * s),
        );
        draws.push(ColourDraw { geo: sphere_geo(1.0, 16, 12, hex_lin(col)), model });
    }
}

/// One palm (shots.ts palm()): leaning trunk, 7 fronds with tips,
/// 3 coconuts. Frond spheres approximate the original's flattened
/// sphere fronds with the same scale/placement.
fn add_palm(draws: &mut Vec<ColourDraw>, pos: [f32; 3], s: f32, lean: f32, rot_y: f32) {
    let base = math::mul(
        &t3(pos[0], pos[1], pos[2]),
        &math::mul(&math::rotate_y(rot_y), &math::mul(&math::rotate_z(lean), &math::scale(s, s, s))),
    );
    draws.push(ColourDraw {
        geo: cyl_geo(0.085, 0.13, 4.4, 8, hex_lin("#7A6248")),
        model: math::mul(&base, &t3(0.0, 2.2, 0.0)),
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
        draws.push(ColourDraw { geo: sphere_geo(1.0, 10, 6, col), model: frond_model });
        let tip_model = math::mul(
            &math::mul(&base, &t3(sa * 1.85, top_y - 0.28 - (i % 3) as f32 * 0.06, ca * 1.85)),
            &math::mul(&math::euler_xyz(ca * 0.5, a, -sa * 0.5), &math::scale(0.13, 0.04, 0.8)),
        );
        draws.push(ColourDraw { geo: sphere_geo(1.0, 10, 6, col), model: tip_model });
    }
    let nut = hex_lin("#6B4A2E");
    for &(nx, nz) in [(0.14f32, 0.0f32), (-0.12, 0.1), (0.0, -0.14)].iter() {
        draws.push(ColourDraw {
            geo: sphere_geo(0.13, 10, 8, nut),
            model: math::mul(&base, &t3(nx, top_y - 0.2, nz)),
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
// Rendering. Same device/pipeline structure as the M1 scene in
// main.rs, with two honest differences, both forced by what this
// scene is: the textured pipeline alpha-blends (the character is a
// cut-out with transparent margins, the clouds are sprites — M1's
// card was opaque), and textures carry no mipmaps (three.js builds
// them; logged in gaps.md).
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

pub async fn run_ab1(width: u32, height: u32, out: &str, caption_path: &str) {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::VULKAN,
        ..Default::default()
    });
    let adapter = crate::request_adapter(&instance).await;
    let info = adapter.get_info();
    println!(
        "adapter: {} | backend: {:?} | device type: {:?} | driver: {} {}",
        info.name, info.backend, info.device_type, info.driver, info.driver_info
    );
    println!("scene: ab1 — Onoda S01 @ t=1.50 (jungle camp JG), fov 44");

    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: Some("talerender device"),
                required_features: wgpu::Features::empty(),
                // NOT downlevel_defaults: those cap textures at 2048,
                // but the real character canvas is 2560x2816 (res 2).
                // Standard WebGPU limits (8192) are what the scene
                // needs; lavapipe admits them.
                required_limits: wgpu::Limits::default(),
            },
            None,
        )
        .await
        .expect("request_device failed");

    // ---- camera: S01's extracted frame ----
    let aspect = width as f32 / height as f32;
    let proj = math::perspective(44f32.to_radians(), aspect, 0.05, 800.0);
    let view = math::look_at(EYE, TGT, [0.0, 1.0, 0.0]);
    let vp = math::mul(&proj, &view);

    // ---- pipelines ----
    let shader3d = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("geometry shader"),
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("shader3d.wgsl"))),
    });
    let bgl3d = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("uniform layout"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    });
    let layout3d = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("colour pipeline layout"),
        bind_group_layouts: &[&bgl3d],
        push_constant_ranges: &[],
    });
    let colour_attrs = [
        wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x3, offset: 0, shader_location: 0 },
        wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x3, offset: 12, shader_location: 1 },
        wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x3, offset: 24, shader_location: 2 },
    ];
    let pipeline3d = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("colour pipeline"),
        layout: Some(&layout3d),
        vertex: wgpu::VertexState {
            module: &shader3d,
            entry_point: "vs_main",
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: 36,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &colour_attrs,
            }],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader3d,
            entry_point: "fs_main",
            targets: &[Some(wgpu::ColorTargetState {
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState {
            cull_mode: Some(wgpu::Face::Back),
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Less,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
    });

    let shader_tex = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("textured shader"),
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("shader_tex.wgsl"))),
    });
    let bgl_tex = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("textured layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
    });
    let layout_tex = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("textured pipeline layout"),
        bind_group_layouts: &[&bgl_tex],
        push_constant_ranges: &[],
    });
    let tex_attrs = [
        wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x3, offset: 0, shader_location: 0 },
        wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x2, offset: 12, shader_location: 1 },
    ];
    let pipeline_tex = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("textured pipeline"),
        layout: Some(&layout_tex),
        vertex: wgpu::VertexState {
            module: &shader_tex,
            entry_point: "vs_main",
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: 20,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &tex_attrs,
            }],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader_tex,
            entry_point: "fs_main",
            targets: &[Some(wgpu::ColorTargetState {
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState {
            cull_mode: Some(wgpu::Face::Back),
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Less,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
    });

    // ---- textures ----
    let caption_img = image::open(caption_path)
        .unwrap_or_else(|e| panic!("cannot load caption strip {caption_path}: {e}"))
        .to_rgba8();
    let (cap_w, cap_h) = caption_img.dimensions();
    let textures_px: [(Vec<u8>, u32, u32); 6] = [
        paint_sky("#6E9CC0", "#D8E8DA"),
        paint_grass("#5B9A48"),
        paint_cloud(),
        paint_blob(),
        paint_onoda(),
        (caption_img.into_raw(), cap_w, cap_h),
    ];
    let mut tex_views: Vec<wgpu::TextureView> = Vec::new();
    for (px, w, h) in &textures_px {
        let tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("scene texture"),
            size: wgpu::Extent3d { width: *w, height: *h, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &tex,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            px,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(w * 4),
                rows_per_image: Some(*h),
            },
            wgpu::Extent3d { width: *w, height: *h, depth_or_array_layers: 1 },
        );
        tex_views.push(tex.create_view(&wgpu::TextureViewDescriptor::default()));
    }
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

    // ---- geometry buffers + bind groups ----
    let scene = build_scene();
    let make_uniform = |model: &Mat4| -> wgpu::Buffer {
        let u = Uniforms { mvp: math::mul(&vp, model), model: *model };
        let buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("uniforms"),
            size: 128,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&buf, 0, &uniform_bytes(&u));
        buf
    };
    let make_vb = |verts: &[f32], idx: &[u32]| -> (wgpu::Buffer, wgpu::Buffer, u32) {
        let vb = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("vertex buffer"),
            size: (verts.len() * 4) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&vb, 0, &pack_f32(verts));
        let ib = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("index buffer"),
            size: (idx.len() * 4) as u64,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&ib, 0, &pack_u32(idx));
        (vb, ib, idx.len() as u32)
    };
    struct PreparedColour {
        vb: wgpu::Buffer,
        ib: wgpu::Buffer,
        n: u32,
        bg: wgpu::BindGroup,
        _ub: wgpu::Buffer,
    }
    struct PreparedTex {
        vb: wgpu::Buffer,
        ib: wgpu::Buffer,
        n: u32,
        bg: wgpu::BindGroup,
        _ub: wgpu::Buffer,
    }
    let mut colour_prepared: Vec<PreparedColour> = Vec::new();
    for d in &scene.colour {
        let (vb, ib, n) = make_vb(&d.geo.verts, &d.geo.idx);
        let ub = make_uniform(&d.model);
        let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("colour bind group"),
            layout: &bgl3d,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: ub.as_entire_binding() }],
        });
        colour_prepared.push(PreparedColour { vb, ib, n, bg, _ub: ub });
    }
    let prepare_tex = |d: &TexDraw| -> PreparedTex {
        let (vb, ib, n) = make_vb(&d.geo.verts, &d.geo.idx);
        let ub = make_uniform(&d.model);
        let sampler = if d.repeat { &sampler_repeat } else { &sampler_clamp };
        let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("textured bind group"),
            layout: &bgl_tex,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: ub.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&tex_views[d.tex]) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(sampler) },
            ],
        });
        PreparedTex { vb, ib, n, bg, _ub: ub }
    };
    let sky_p = prepare_tex(&scene.sky);
    let grass_p = prepare_tex(&scene.grass);
    let blob_p = prepare_tex(&scene.blob);
    let clouds_p: Vec<PreparedTex> = scene.clouds.iter().map(&prepare_tex).collect();
    let card_p = prepare_tex(&scene.card);
    let caption_p = prepare_tex(&scene.caption);

    // ---- frame target + readback ----
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("frame target"),
        size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view_tex = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let depth = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("depth target"),
        size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let depth_view = depth.create_view(&wgpu::TextureViewDescriptor::default());
    let padded = math::padded_row_bytes(width);
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("readback buffer"),
        size: (padded * height) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("frame encoder"),
    });
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("ab1 scene pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view_tex,
                resolve_target: None,
                ops: wgpu::Operations {
                    // Horizon colour '#D8E8DA', in linear values (the
                    // sRGB target encodes on write).
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: srgb_to_linear(216.0 / 255.0) as f64,
                        g: srgb_to_linear(232.0 / 255.0) as f64,
                        b: srgb_to_linear(218.0 / 255.0) as f64,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        // Sky + grass (textured, opaque content) first.
        pass.set_pipeline(&pipeline_tex);
        for tp in [&sky_p, &grass_p] {
            pass.set_bind_group(0, &tp.bg, &[]);
            pass.set_vertex_buffer(0, tp.vb.slice(..));
            pass.set_index_buffer(tp.ib.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..tp.n, 0, 0..1);
        }
        // All coloured geometry.
        pass.set_pipeline(&pipeline3d);
        for cp in &colour_prepared {
            pass.set_bind_group(0, &cp.bg, &[]);
            pass.set_vertex_buffer(0, cp.vb.slice(..));
            pass.set_index_buffer(cp.ib.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..cp.n, 0, 0..1);
        }
        // Transparent textured content, back to front.
        pass.set_pipeline(&pipeline_tex);
        for tp in std::iter::once(&blob_p).chain(clouds_p.iter()).chain([&card_p, &caption_p]) {
            pass.set_bind_group(0, &tp.bg, &[]);
            pass.set_vertex_buffer(0, tp.vb.slice(..));
            pass.set_index_buffer(tp.ib.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..tp.n, 0, 0..1);
        }
    }
    encoder.copy_texture_to_buffer(
        wgpu::ImageCopyTexture {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::ImageCopyBuffer {
            buffer: &readback,
            layout: wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(padded),
                rows_per_image: Some(height),
            },
        },
        wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
    );
    queue.submit(Some(encoder.finish()));

    let slice = readback.slice(..);
    let (tx, rx) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |result| {
        let _ = tx.send(result);
    });
    device.poll(wgpu::Maintain::Wait);
    rx.recv().expect("map channel closed").expect("buffer map failed");
    let data = slice.get_mapped_range();
    let row = (width * 4) as usize;
    let stride = padded as usize;
    let mut pixels = Vec::with_capacity(row * height as usize);
    for y in 0..height as usize {
        pixels.extend_from_slice(&data[y * stride..y * stride + row]);
    }
    drop(data);
    readback.unmap();

    let img = image::RgbaImage::from_raw(width, height, pixels).expect("pixel buffer size mismatch");
    img.save(out).expect("failed to save PNG");
    println!("wrote {out} ({width}x{height})");
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
