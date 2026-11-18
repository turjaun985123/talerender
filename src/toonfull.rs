//! Full drawToon port (production integration): the complete
//! character painter from the source engine's toon.ts — all 19
//! moods, walk/run/sit/jump legs, patterns (vest, stripes, apron),
//! neckwear (tie, scarf, collar), hats (cap, bowler, boater,
//! beanie, visor, tophat, bun, flower, bow, mailcap), extras
//! (glasses, mustache, blush, freckles, pencil, monocle), fox
//! ears + hair strands behind the head disc (Spec §5b / §5c),
//! blink /
//! talk / look / sweat / tears, headTilt / headTurn / camPitch /
//! squash, and held-prop plates — where scene::paint_toon (M4) is
//! the five-mood stand subset the §28 scene needs.
//!
//! The raster model is the M4 one (signed-distance coverage via
//! scene::Painter), and the transform convention matches
//! paint_toon: geometry is computed in stage space (512x1024, feet
//! at y = 1000) and mapped through the lean rotation into the
//! 1280x1408 canvas at res 2. Head-part geometry additionally
//! passes the squash (about the head centre) and headTilt (about
//! the neck pivot) stage-space transforms, in the canvas's own
//! nesting order (lean outermost, then tilt, then squash).
//!
//! Held props arrive as pre-rendered plates (RGBA, stage units at
//! res 2, rendered by film/gen_assets.py from the same prop
//! geometry — including the Inter text, which enters as data).
//! Every prop this film uses ignores its `angle` argument (paper
//! rotates a fixed -0.08, the leaflet -0.1, book/coconut none), so
//! a plate composites by translation to the hand point alone,
//! inverse-mapped through the lean rotation.

use crate::scene::{
    ellipse_sd, flatten_cubic, flatten_quad, hex_rgb, shade_rgb, Painter, Rgba,
};

pub(crate) const INK: &str = "#1F1C33";
const RES: f32 = 2.0;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Mood {
    Neutral,
    Smug,
    Happy,
    Laugh,
    Shock,
    Worried,
    Angry,
    Sad,
    Sly,
    Proud,
    Tired,
    Dizzy,
    Determined,
    Embarrassed,
    Awe,
    Suspicious,
    Relieved,
    Puzzled,
    Deadpan,
}

impl Mood {
    pub(crate) fn from_str(s: &str) -> Mood {
        match s {
            "smug" => Mood::Smug,
            "happy" => Mood::Happy,
            "laugh" => Mood::Laugh,
            "shock" => Mood::Shock,
            "worried" => Mood::Worried,
            "angry" => Mood::Angry,
            "sad" => Mood::Sad,
            "sly" => Mood::Sly,
            "proud" => Mood::Proud,
            "tired" => Mood::Tired,
            "dizzy" => Mood::Dizzy,
            "determined" => Mood::Determined,
            "embarrassed" => Mood::Embarrassed,
            "awe" => Mood::Awe,
            "suspicious" => Mood::Suspicious,
            "relieved" => Mood::Relieved,
            "puzzled" => Mood::Puzzled,
            "deadpan" => Mood::Deadpan,
            _ => Mood::Neutral,
        }
    }
    /// drawFace brow table: [lift, tilt].
    fn brow(self) -> (f32, f32) {
        match self {
            Mood::Neutral => (4.0, 0.0),
            Mood::Smug => (-8.0, 10.0),
            Mood::Happy => (8.0, 0.0),
            Mood::Laugh => (10.0, 0.0),
            Mood::Shock => (24.0, 0.0),
            Mood::Worried => (10.0, -16.0),
            Mood::Angry => (-10.0, 22.0),
            Mood::Sad => (8.0, -18.0),
            Mood::Sly => (-6.0, 12.0),
            Mood::Proud => (6.0, 4.0),
            Mood::Tired => (-2.0, 0.0),
            Mood::Dizzy => (14.0, 0.0),
            Mood::Determined => (-4.0, 16.0),
            Mood::Embarrassed => (12.0, -10.0),
            Mood::Awe => (26.0, 0.0),
            Mood::Suspicious => (-2.0, 6.0),
            Mood::Relieved => (14.0, -4.0),
            Mood::Puzzled => (6.0, 4.0),
            Mood::Deadpan => (2.0, 0.0),
        }
    }
    fn heavy_lid(self) -> bool {
        matches!(self, Mood::Smug | Mood::Tired | Mood::Sly | Mood::Proud | Mood::Determined | Mood::Deadpan)
    }
    /// drawFace eye() lid offset: the lid rect's bottom edge and
    /// the lid line sit at ey - 2 + lid (lid = 6 tired, 8 deadpan,
    /// 0 for the other heavy-lid moods).
    fn lid_offset(self) -> f32 {
        match self {
            Mood::Tired => 6.0,
            Mood::Deadpan => 8.0,
            _ => 0.0,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Legs {
    Stand,
    Walk,
    Run,
    Sit,
    Jump,
}

/// A held-prop plate: RGBA pixels at res 2 in stage units.
/// (x0, y0) is the stage-local coordinate of the plate's top-left
/// corner relative to the hand point the PropFn receives.
pub(crate) struct Plate {
    pub px: Vec<u8>,
    pub w: u32,
    pub h: u32,
    pub x0: f32,
    pub y0: f32,
}

pub(crate) struct FullLook {
    pub body: String,
    pub head: String,
    pub shade: Option<String>,
    pub pattern: String, // 'plain' | 'vest' | 'stripes' | 'apron'
    pub pat_col: Option<String>,
    pub hat: String, // 'none' | 'cap' | 'bowler' | 'boater' | 'beanie' | 'visor' | 'tophat' | 'bun' | 'flower' | 'bow' | 'mailcap'
    pub hat_col: Option<String>,
    pub band: Option<String>,
    pub neck: String, // 'none' | 'tie' | 'scarf' | 'collar'
    pub neck_col: Option<String>,
    pub glasses: bool,
    pub mustache: bool,
    pub mustache_col: Option<String>,
    pub kid: bool,
    // CR-B3-06 look ports (B2-10 §1.4–§1.8): extra-list flags +
    // hair colour. All default false / None in every pre-existing
    // look, so paints without them are byte-identical. These are
    // FullLook fields, not FullState: the film PaintCache keys a
    // look by registry index (film.rs state_key), and scenefile
    // paints uncached, so no state_key / charstate change applies.
    pub blush: bool,    // extra 'blush' (drawFace: OR'd with the laugh/proud/embarrassed moods)
    pub freckles: bool, // extra 'freckles'
    pub pencil: bool,   // extra 'pencil'
    pub monocle: bool,  // extra 'monocle'
    pub hair_col: Option<String>, // 'hairCol' (bow hat's side locks + §5c hair strands)
    // GPUFIRST Batch 2 CARRY plumbing (DESIGN-CARRY fields):
    // the wing / ears / hair look channels of RUST-PARITY-SPECS
    // Spec 1 / §5b / §5c. The FIELDS exist so no format break
    // stands between here and the painter ports. Batch 3: the
    // ears (§5b) and hair (§5c) painters now READ their fields;
    // the wing fields are still plumbing only (the painter does
    // NOT read them yet). Every pre-existing look carries the
    // defaults below, so paints without these channels are
    // byte-identical.
    pub wing: bool,              // look 'wing' (Spec 1: wing appendage declared)
    pub wing_col: Option<String>, // look 'wingCol' (Spec 1: wing palette)
    pub wing_lens: Vec<f32>,     // look 'wingLens' (Spec 1: per-segment lengths)
    pub ears: bool,              // look 'ears' ("fox" / true; §5b fox ears)
    pub hair: bool,              // look 'hair' (§5c hair strands; colour is hair_col above)
}

pub(crate) struct FullState {
    pub mood: Mood,
    pub look: [f32; 2],
    pub blink: f32,
    pub talk: f32,
    // Spec 5a (RUST-PARITY-SPECS §5a): post-table brow offsets,
    // stage units, continuous f32 with no clamp / quantisation.
    // Default 0.0 / 0.0 is exactly the pre-channel arithmetic.
    pub brow_lift: f32,
    pub brow_tilt: f32,
    pub sweat: Option<f32>, // Spec 5e: numeric sweat (drawFace line 1490); drop also fires on 'worried', k = sweat.unwrap_or(1.0), alpha = min(1, k)
    pub tears: bool, // drawFace: tears also fire on 'sad'
    pub arm_l: (f32, f32),
    pub arm_r: (f32, f32),
    pub legs: Legs,
    pub phase: f32,
    pub lean: f32,
    pub bob: f32,
    pub squash: f32,
    pub head_tilt: f32,
    pub head_turn: f32,
    pub cam_pitch: f32,
    pub sit_drop: Option<f32>,
    pub hold_l: Option<Plate>,
    pub hold_r: Option<Plate>,
    // GPUFIRST Batch 2 CARRY plumbing: the wing / ears / hair /
    // flip / bodyPitch state channels of RUST-PARITY-SPECS
    // Spec 1 / §5b / §5c / §5d / Spec 3a, keyed in film.rs
    // state_key per the §3b coverage law, so a port cannot
    // silently recreate the F-1 defect on these channels.
    // Batch 3 PORTED the painters for all five: wing (seat
    // PORT-WING), ears + hair (seat PORT-EARS-HAIR: ear_a /
    // ear_al / ear_ar, hair_segs), flip + bodyPitch (seat
    // PORT-FLIP-PITCH: flip mirror in the point maps +
    // canvas_anchor; bodyPitch hip-pivot seam as the host-side
    // `pxu` / `hpt` transforms + effective head tilt).
    pub wing_segs: Vec<f32>, // state 'wingSegs' (Spec 1: per-segment absolute angles)
    pub wing_a: f32,         // state 'wingA' (Spec 1: scalar fallback angle)
    pub ear_a: f32,          // state 'earA' (§5b: both-ears angle, radians)
    pub ear_al: Option<f32>, // state 'earAL' (§5b: left-ear angle, wins per side)
    pub ear_ar: Option<f32>, // state 'earAR' (§5b: right-ear angle, wins per side)
    pub hair_segs: Vec<f32>, // state 'hairSegs' (§5c: per-strand angles; length sets strand count)
    pub flip: bool,          // state 'flip' (§5d: whole-figure mirror)
    pub body_pitch: f32,     // state 'bodyPitch' (Spec 3a: degrees, hip-pivot)
    pub body_pitch_counter: f32, // state 'bodyPitchCounter' (Spec 3a: TS resolved default ?? 1)
}

impl Default for FullState {
    fn default() -> Self {
        FullState {
            mood: Mood::Neutral,
            look: [0.0, 0.0],
            blink: 0.0,
            talk: 0.0,
            brow_lift: 0.0,
            brow_tilt: 0.0,
            sweat: None,
            tears: false,
            arm_l: (8.0, 6.0),
            arm_r: (8.0, 6.0),
            legs: Legs::Stand,
            phase: 0.0,
            lean: 0.0,
            bob: 0.0,
            squash: 0.0,
            head_tilt: 0.0,
            head_turn: 0.0,
            cam_pitch: 0.0,
            sit_drop: None,
            hold_l: None,
            hold_r: None,
            wing_segs: Vec::new(),
            wing_a: 0.0,
            ear_a: 0.0,
            ear_al: None,
            ear_ar: None,
            hair_segs: Vec::new(),
            flip: false,
            body_pitch: 0.0,
            body_pitch_counter: 1.0,
        }
    }
}

fn rgba(s: &str) -> Rgba {
    let (r, g, b) = hex_rgb(s);
    (r, g, b, 255)
}

fn shade_rgba(s: &str, k: f32) -> Rgba {
    let (r, g, b) = shade_rgb(s, k);
    (r, g, b, 255)
}

/// Fill a polygon through a mask (the bean clip for patterns):
/// per-pixel centre test against both masks over the bbox.
fn fill_polygon_masked(p: &mut Painter, pts: &[(f32, f32)], mask: &[bool], col: Rgba) {
    // GPU trace hook: MUST stay above the internal polygon_mask
    // call below — that call records the panel's own mask and
    // moves the recorder's last-mask id; resolving this fill's
    // mask (the caller's bean mask) has to happen first. See
    // docs/integration/GPUFIRST-TRACE.md §3.
    #[cfg(feature = "gpupaint")]
    crate::gputrace::trace_fill_polygon_masked(pts, col, mask);
    let own = p.polygon_mask(pts);
    let (mut x0, mut y0, mut x1, mut y1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    for &(x, y) in pts {
        x0 = x0.min(x);
        y0 = y0.min(y);
        x1 = x1.max(x);
        y1 = y1.max(y);
    }
    let (bx0, by0, bx1, by1) = p.bbox(x0, y0, x1, y1);
    for y in by0..=by1 {
        for x in bx0..=bx1 {
            let k = y as usize * p.w + x as usize;
            if mask[k] && own[k] {
                p.blend(x, y, col);
            }
        }
    }
}

/// Rounded-rect points in stage space with per-corner radii
/// [top-left, top-right, bottom-right, bottom-left] — the native
/// stand-in for ctx.roundRect with a radius list (scene's
/// rounded_rect_pts takes a single radius). Corner order and
/// flattening follow rounded_rect_pts (seg segments per corner).
fn rounded_rect_pts_radii(
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    radii: [f32; 4],
    seg: usize,
) -> Vec<(f32, f32)> {
    let [rtl, rtr, rbr, rbl] = radii;
    let corners = [
        (x + w - rtr, y + rtr, rtr, -std::f32::consts::FRAC_PI_2),
        (x + w - rbr, y + h - rbr, rbr, 0.0),
        (x + rbl, y + h - rbl, rbl, std::f32::consts::FRAC_PI_2),
        (x + rtl, y + rtl, rtl, std::f32::consts::PI),
    ];
    let mut pts: Vec<(f32, f32)> = Vec::new();
    for (cx, cy, r, a0) in corners {
        for i in 0..=seg {
            let a = a0 + (i as f32 / seg as f32) * std::f32::consts::FRAC_PI_2;
            pts.push((cx + r * a.cos(), cy + r * a.sin()));
        }
    }
    pts
}

/// Rotate a stage point about the hip pivot `(bx, hip_y)` by
/// `pitch_deg` (bodyPitch, Spec 3) — the TS painter's `rotPt`.
/// At pitch 0 this returns the point EXACTLY (the TS seam emits
/// no transform at pitch 0; byte-identity depends on it).
pub(crate) fn pitch_pt(x: f32, y: f32, bx: f32, hip_y: f32, pitch_deg: f32) -> (f32, f32) {
    if pitch_deg == 0.0 {
        return (x, y);
    }
    let (s, c) = pitch_deg.to_radians().sin_cos();
    let (dx, dy) = (x - bx, y - hip_y);
    (bx + dx * c - dy * s, hip_y + dx * s + dy * c)
}

/// The head's effective tilt under bodyPitch (Spec 3):
/// `headTilt − bodyPitch · bodyPitchCounter` (counter defaults
/// to 1.0, TS `?? 1` — see FullState::default). At pitch 0 this
/// is exactly `head_tilt`.
pub(crate) fn effective_head_tilt(st: &FullState) -> f32 {
    st.head_tilt - st.body_pitch * st.body_pitch_counter
}

/// Composite a held-prop plate at a stage-space hand point,
/// inverse-mapping each canvas pixel through the full figure
/// transform stack (stage -> canvas is: bodyPitch about the hip
/// pivot for the upper assembly the plate rides, then lean about
/// (256, 1000), then the flip mirror x -> 512 - x, then the
/// canvas re-anchor, then +(384, 256), then x res). Bilinear
/// sampling of the plate.
///
/// B3-CARRY-1: the inverse map carries flip + pitch as well as
/// the lean + anchor it always had — before Batch 3 no painter
/// emitted flip/pitch, so the omission was latent; a flipped or
/// pitched hold would have sampled its plate un-mirrored /
/// un-rotated. `hand` is the UN-pitched stage hand (the plate's
/// figure-local origin, exactly as the TS held-prop callbacks
/// receive it inside the pitched frame); `hip` is the pitch
/// pivot `(bx, hip_y)`. At flip=false / pitch=0 every added
/// branch is skipped and the arithmetic is the pre-Batch-3 map,
/// byte for byte.
#[allow(clippy::too_many_arguments)]
fn composite_plate(
    p: &mut Painter,
    plate: &Plate,
    hand: (f32, f32),
    lean_rad: f32,
    anchor: (f32, f32),
    flip: bool,
    pitch_rad: f32,
    hip: (f32, f32),
) {
    #[cfg(feature = "gpupaint")]
    crate::gputrace::trace_composite_plate(plate, hand, lean_rad, anchor, flip, pitch_rad, hip);
    let (s, c) = lean_rad.sin_cos();
    let pitched = pitch_rad != 0.0;
    let (sp, cp) = pitch_rad.sin_cos();
    let (ax, ay) = anchor;
    // Canvas-space corners of the plate rect (forward map).
    let fwd = |lx: f32, ly: f32| -> (f32, f32) {
        let (ux, uy) = if pitched {
            let (dx, dy) = (lx - hip.0, ly - hip.1);
            (hip.0 + dx * cp - dy * sp, hip.1 + dx * sp + dy * cp)
        } else {
            (lx, ly)
        };
        let (dx, dy) = (ux - 256.0, uy - 1000.0);
        let (rx, ry) = (256.0 + dx * c - dy * s, 1000.0 + dx * s + dy * c);
        let rx = if flip { 512.0 - rx } else { rx };
        ((rx + ax + 384.0) * RES, (ry + ay + 256.0) * RES)
    };
    let corners = [
        fwd(hand.0 + plate.x0, hand.1 + plate.y0),
        fwd(hand.0 + plate.x0 + plate.w as f32 / RES, hand.1 + plate.y0),
        fwd(hand.0 + plate.x0, hand.1 + plate.y0 + plate.h as f32 / RES),
        fwd(hand.0 + plate.x0 + plate.w as f32 / RES, hand.1 + plate.y0 + plate.h as f32 / RES),
    ];
    let (mut x0, mut y0, mut x1, mut y1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    for &(x, y) in &corners {
        x0 = x0.min(x);
        y0 = y0.min(y);
        x1 = x1.max(x);
        y1 = y1.max(y);
    }
    let (bx0, by0, bx1, by1) = p.bbox(x0, y0, x1, y1);
    let (pw, ph) = (plate.w as f32, plate.h as f32);
    for y in by0..=by1 {
        for x in bx0..=bx1 {
            // canvas -> stage (inverse anchor, flip, lean, pitch).
            let (ux0, uy) = (x as f32 / RES - 384.0 - ax, y as f32 / RES - 256.0 - ay);
            let ux = if flip { 512.0 - ux0 } else { ux0 };
            let (dx, dy) = (ux - 256.0, uy - 1000.0);
            let sx0 = 256.0 + dx * c + dy * s;
            let sy0 = 1000.0 - dx * s + dy * c;
            let (sx, sy) = if pitched {
                let (dx, dy) = (sx0 - hip.0, sy0 - hip.1);
                (hip.0 + dx * cp + dy * sp, hip.1 - dx * sp + dy * cp)
            } else {
                (sx0, sy0)
            };
            let lx = (sx - hand.0 - plate.x0) * RES;
            let ly = (sy - hand.1 - plate.y0) * RES;
            if lx < -0.5 || ly < -0.5 || lx > pw - 0.5 || ly > ph - 0.5 {
                continue;
            }
            // Bilinear sample (premultiplied blend via blend_cov on
            // the sampled colour with the sampled alpha).
            let fx = lx.clamp(0.0, pw - 1.001);
            let fy = ly.clamp(0.0, ph - 1.001);
            let (ix, iy) = (fx as usize, fy as usize);
            let (tx, ty) = (fx - ix as f32, fy - iy as f32);
            let sample = |xx: usize, yy: usize| -> [f32; 4] {
                let k = (yy * plate.w as usize + xx) * 4;
                [
                    plate.px[k] as f32,
                    plate.px[k + 1] as f32,
                    plate.px[k + 2] as f32,
                    plate.px[k + 3] as f32,
                ]
            };
            let s00 = sample(ix, iy);
            let s10 = sample(ix + 1, iy);
            let s01 = sample(ix, iy + 1);
            let s11 = sample(ix + 1, iy + 1);
            let mut out = [0.0f32; 4];
            for ch in 0..4 {
                out[ch] = (s00[ch] * (1.0 - tx) + s10[ch] * tx) * (1.0 - ty)
                    + (s01[ch] * (1.0 - tx) + s11[ch] * tx) * ty;
            }
            let a = out[3] / 255.0;
            if a > 0.0 {
                p.blend_cov(
                    x,
                    y,
                    (out[0] as u8, out[1] as u8, out[2] as u8, 255),
                    a,
                );
            }
        }
    }
}

/// Canvas re-anchor (B-1: the Rust port of the TS painter's
/// `solveCanvasAnchor`, v7 P2 D4 — tall-tale template/src/core/
/// toon.ts). The cut-out canvas is finite (stage 512x1024 placed
/// at +(384, 256) inside a 1280x1408 canvas), and two universal
/// pose families paint outside it: high-|lean| states (lean
/// rotates the figure about its feet, so at the lying lean the
/// head lands past the side edge) and high seats at small
/// scales. Capping the lean or the seat would rewrite clip
/// contracts, so the painter instead RE-ANCHORS: this function
/// rebuilds the painted envelope from the painter's own geometry
/// (the bean cubics, the head ellipse, the hat/neckwear extremes
/// this painter draws, arm FK, every leg branch this painter
/// implements), runs it through the painter's transform stack
/// (head squash -> headTilt about the neck pivot -> lean about
/// the feet), and returns the smallest whole-figure translation,
/// in stage units, that brings the envelope inside the canvas —
/// inset by 4 so antialiased paint never touches the border
/// strip `scan_canvas_overflow` reads.
///
/// Semantics preserved from the TS original: a pure function of
/// (look, state) — deterministic, ZERO for every state that
/// already fits (previously clean states paint byte-identically),
/// and continuous in the state, so an animated fall slides its
/// anchor instead of popping. `paint_toon_full` applies it
/// inside the stage -> canvas map (plates included — they ride
/// the hands); the card-placement sites (film.rs resolve_chars,
/// scenefile.rs) counter-offset the card model by the opposite
/// world amount, so a re-anchored figure keeps its exact world
/// position — only the transparent canvas margins redistribute.
/// Held plates are NOT modelled in the envelope (their reach is
/// bounded by the prop contract and they ride the hands, which
/// are), exactly as in the TS original.
///
/// CARRY note (GPUFIRST Batch 2, updated Batch 3): the wing
/// envelope points the TS anchor models (Spec 1: both wing
/// chains) are deliberately NOT added here — the wing painter
/// does not exist yet and the anchor models painted geometry
/// only; adding them remains the wing port's obligation. The
/// EAR envelope (§5b: both ear triangles, tips included, radius
/// 3.5 = appendage width 7 / 2, exactly as the TS anchor at
/// toon.ts 683–693) IS modelled below, because the ear painter
/// now exists. Hair (§5c) is deliberately NOT modelled: the TS
/// anchor does not envelope hair strands — mirror that, invent
/// nothing.
///
/// Port notes (where this painter's geometry differs from the
/// TS painter's, the envelope follows THIS painter): the sit
/// drop is the painter's own (426 - sitDrop)/0.98 with its fixed
/// sit-leg polyline (no two-bone sit solve, no kneel, no
/// tail/scarf-streamer/wing/fox-ear primitives exist here); the
/// head envelope uses the painter's own squash factors
/// (1 - 0.3*squash, 1 + squash) rather than the TS anchor's
/// exact volume factors; hat extremes are this painter's drawn
/// shapes (tophat crown, bun, bow locks, visor sheet, beanie,
/// mailcap, the cap/bowler/boater brim family). The anchor
/// consumes only inputs the painter already consumes, so the
/// paint-cache key (film.rs state_key, which covers every
/// FullState field) needs no new component.
pub(crate) fn canvas_anchor(look: &FullLook, st: &FullState) -> (f32, f32) {
    const INSET: f32 = 4.0;
    let sit = st.legs == Legs::Sit;
    let jump = st.legs == Legs::Jump;
    let sit_s = st.sit_drop.unwrap_or(220.2);
    // `by`, top/bot/hip and head geometry exactly as paint_toon_full.
    let by = (if sit { (426.0 - sit_s) / 0.98 } else { 0.0 })
        + if look.kid { 190.0 } else { 0.0 }
        + st.bob
        + if jump { -60.0 } else { 0.0 };
    let (bx, top, bot, bw) = (256.0f32, 330.0 + by, 600.0 + by * 0.98, 92.0f32);
    let hip_y = bot - 26.0;
    let (hx, hy, r) = (256.0f32, 178.0 + by * 0.99, 138.0f32);
    let pivot_y = 318.0 + by * 0.99;
    let rp = r * (1.0 + 0.04 * st.cam_pitch);
    let turn = st.head_turn;
    let hatx = hx + turn * 18.0;
    let htop = hy - rp * 0.97;
    let lean = st.lean;
    // Batch 3 (flip §5d + bodyPitch Spec 3): the envelope runs
    // through the painter's full stack — head points rotate by
    // the EFFECTIVE tilt, upper-assembly points then rotate by
    // the pitch about (bx, hip_y), every point then leans about
    // the feet, and the flip mirror (x -> 512 - x) is applied
    // LAST, exactly as the TS anchor's push() does (toon.ts
    // solveCanvasAnchor). Leg points are never pitched. At
    // pitch 0 / flip false each added step is the identity, so
    // pre-Batch-3 envelopes (and anchors) are unchanged.
    let pitch = st.body_pitch;
    let eff_tilt = effective_head_tilt(st);

    fn rot_pt(x: f32, y: f32, cx: f32, cy: f32, deg: f32) -> (f32, f32) {
        if deg == 0.0 {
            return (x, y);
        }
        let (s, c) = deg.to_radians().sin_cos();
        let (dx, dy) = (x - cx, y - cy);
        (cx + dx * c - dy * s, cy + dx * s + dy * c)
    }
    fn quad_pts(p0: (f32, f32), p1: (f32, f32), p2: (f32, f32), n: usize) -> Vec<(f32, f32)> {
        let mut out = Vec::with_capacity(n + 1);
        for i in 0..=n {
            let u = i as f32 / n as f32;
            let v = 1.0 - u;
            out.push((
                v * v * p0.0 + 2.0 * v * u * p1.0 + u * u * p2.0,
                v * v * p0.1 + 2.0 * v * u * p1.1 + u * u * p2.1,
            ));
        }
        out
    }
    fn cubic_pts(
        p0: (f32, f32),
        p1: (f32, f32),
        p2: (f32, f32),
        p3: (f32, f32),
        n: usize,
    ) -> Vec<(f32, f32)> {
        let mut out = Vec::with_capacity(n + 1);
        for i in 0..=n {
            let u = i as f32 / n as f32;
            let v = 1.0 - u;
            out.push((
                v * v * v * p0.0 + 3.0 * v * v * u * p1.0 + 3.0 * v * u * u * p2.0 + u * u * u * p3.0,
                v * v * v * p0.1 + 3.0 * v * v * u * p1.1 + 3.0 * v * u * u * p2.1 + u * u * u * p3.1,
            ));
        }
        out
    }

    // Envelope points (stage x, stage y, radius), carried through
    // the painter's transform stack.
    let mut pts: Vec<(f32, f32, f32)> = Vec::new();
    // The flip mirror, applied last to every envelope point
    // (TS anchor: `if (S.flip) px = GEO.stage.w - px`).
    let flip_x = |x: f32| if st.flip { 512.0 - x } else { x };
    // Upper-assembly push (bean, arms, neckwear — the painter
    // maps them through its pitched map): pitch about the hip,
    // lean about the feet, flip.
    macro_rules! push {
        ($x:expr, $y:expr, $rad:expr) => {{
            let (ux, uy) = rot_pt($x, $y, bx, hip_y, pitch);
            let (qx, qy) = rot_pt(ux, uy, 256.0, 1000.0, lean);
            pts.push((flip_x(qx), qy, $rad));
        }};
    }
    // Leg push: lean about the feet, then flip — legs are NEVER
    // pitched (Spec 3).
    macro_rules! push_leg {
        ($x:expr, $y:expr, $rad:expr) => {{
            let (qx, qy) = rot_pt($x, $y, 256.0, 1000.0, lean);
            pts.push((flip_x(qx), qy, $rad));
        }};
    }
    // Head-space push: the painter's `hpt` stack in stage space —
    // squash about the head centre (the painter's own factors),
    // effective headTilt about the neck pivot, pitch about the
    // hip, lean about the feet, flip.
    macro_rules! push_head {
        ($x:expr, $y:expr, $rad:expr) => {{
            let (sx2, sy2) = (
                hx + ($x - hx) * (1.0 - 0.3 * st.squash),
                hy + ($y - hy) * (1.0 + st.squash),
            );
            let (tx2, ty2) = rot_pt(sx2, sy2, 256.0, pivot_y, eff_tilt);
            let (ux, uy) = rot_pt(tx2, ty2, bx, hip_y, pitch);
            let (qx, qy) = rot_pt(ux, uy, 256.0, 1000.0, lean);
            pts.push((flip_x(qx), qy, $rad));
        }};
    }

    // Body bean — the painter's four cubics, ink radius 9/2.
    let bean: [Vec<(f32, f32)>; 4] = [
        cubic_pts((bx - 58.0, top + 10.0), (bx - 80.0, top + 40.0), (bx - bw - 6.0, bot - 120.0), (bx - bw, bot - 50.0), 12),
        cubic_pts((bx - bw, bot - 50.0), (bx - bw + 4.0, bot + 6.0), (bx + bw - 4.0, bot + 6.0), (bx + bw, bot - 50.0), 12),
        cubic_pts((bx + bw, bot - 50.0), (bx + bw + 6.0, bot - 120.0), (bx + 80.0, top + 40.0), (bx + 58.0, top + 10.0), 12),
        cubic_pts((bx + 58.0, top + 10.0), (bx + 30.0, top - 8.0), (bx - 30.0, top - 8.0), (bx - 58.0, top + 10.0), 12),
    ];
    for seg in &bean {
        for &(x, y) in seg {
            push!(x, y, 4.5);
        }
    }
    // Head ellipse (the painter draws it at rp with the 0.97
    // vertical factor; outline stroke lw 10 -> radius 5).
    for i in 0..16 {
        let a = (i as f32 / 16.0) * std::f32::consts::TAU;
        push_head!(hx + rp * a.cos(), hy + rp * 0.97 * a.sin(), 5.0);
    }
    // Fox ears (§5b): the painter's triangles, tips included —
    // the TS anchor models both ear triangles (toon.ts 683–693)
    // with radius 3.5 (the appendage stroke width 7 / 2), so a
    // tilted ear cannot paint outside the cleared envelope.
    // The rotation is about the ear anchor, in head space
    // (push_head applies this painter's squash/tilt/lean stack,
    // the same stack the ear painter's points pass through).
    // Hair is NOT modelled here (§5c: the TS anchor gives hair
    // no envelope — mirror that exactly).
    if look.ears {
        for s in [-1.0f32, 1.0] {
            let ea = (if s < 0.0 { st.ear_al } else { st.ear_ar }).unwrap_or(st.ear_a);
            let (exx, eyy) = (hx + s * rp * 0.52, hy - rp * 0.74);
            let (sa, ca) = (s * 0.16 + ea).sin_cos();
            for (lx, ly) in [(-28.0f32, 8.0f32), (0.0, -88.0), (28.0, 8.0)] {
                push_head!(exx + lx * ca - ly * sa, eyy + lx * sa + ly * ca, 3.5);
            }
        }
    }
    // Hat extremes, per this painter's drawHat branches.
    match look.hat.as_str() {
        "tophat" => {
            // Crown roundRect(hatx - rp*0.6, htop - 140, rp*1.2,
            // 164) + brim ellipse rx rp at htop + 24.
            for &(x, y) in &[
                (hatx - rp * 0.6, htop - 140.0),
                (hatx + rp * 0.6, htop - 140.0),
                (hatx - rp * 0.6, htop + 24.0),
                (hatx + rp * 0.6, htop + 24.0),
                (hatx - rp, htop + 24.0),
                (hatx + rp, htop + 24.0),
            ] {
                push_head!(x, y, 4.5);
            }
        }
        "bun" => {
            // Bun disc (hx + 10 + turn*18, hy - rp - 8) r 46,
            // drawn before the head disc.
            push_head!(hx + 10.0 + turn * 18.0, hy - rp - 8.0, 46.0 * (1.0 + st.squash) + 4.5);
        }
        "bow" => {
            // Side locks: ellipses (hatx +/-(rp + 4), hy + 10)
            // rx 22 ry 50; loops reach +/-66 / -46..+40 from
            // (hatx + rp*0.5, htop + 26).
            for s in [-1.0f32, 1.0] {
                push_head!(hatx + s * (rp + 4.0), hy + 10.0, 50.0 * (1.0 + st.squash) + 3.0);
            }
            push_head!(hatx + rp * 0.5, htop + 26.0, 69.0);
        }
        "visor" => {
            // Sheet corners reach hatx +/- rp*1.15 at htop + 98.
            for &(x, y) in &[
                (hatx - rp * 1.15, htop + 98.0),
                (hatx + rp * 1.15, htop + 98.0),
                (hatx - rp, htop + 70.0),
                (hatx + rp, htop + 70.0),
                (hatx, htop + 10.0),
            ] {
                push_head!(x, y, 4.5);
            }
        }
        "beanie" => {
            // Dome top (hatx, htop + 50 - 96), brim corners, pom
            // (hatx, htop - 52) r 22.
            for &(x, y) in &[
                (hatx, htop - 46.0),
                (hatx - rp * 0.98, htop + 30.0),
                (hatx + rp * 0.98, htop + 30.0),
                (hatx - rp * 0.98, htop + 70.0),
                (hatx + rp * 0.98, htop + 70.0),
            ] {
                push_head!(x, y, 4.5);
            }
            push_head!(hatx, htop - 52.0, 22.0 * (1.0 + st.squash) + 4.5);
        }
        "mailcap" => {
            // Crown roundRect(hatx - rp*0.72, htop - 40,
            // rp*1.44, 70); brim half-ellipse rx rp*0.82 at
            // (hatx - 10, htop + 34).
            for &(x, y) in &[
                (hatx - rp * 0.72, htop - 40.0),
                (hatx + rp * 0.72, htop - 40.0),
                (hatx - rp * 0.72, htop + 30.0),
                (hatx + rp * 0.72, htop + 30.0),
                (hatx - 10.0 - rp * 0.82, htop + 34.0),
                (hatx - 10.0 + rp * 0.82, htop + 34.0),
            ] {
                push_head!(x, y, 4.5);
            }
        }
        "flower" => {
            // Petals r 14 at radius 18 around
            // (hatx + rp*0.62, htop + 40).
            push_head!(hatx + rp * 0.62, htop + 40.0, 18.0 + 14.0 * (1.0 + st.squash) + 2.0);
        }
        "cap" | "bowler" | "boater" => {
            // The brim family: brim ellipses reach rx rp*1.3
            // around htop + 26..34; crowns rise to htop - 46 at
            // rp*0.66.
            for &(x, y) in &[
                (hatx + 6.0 - rp * 1.3, htop + 34.0),
                (hatx + 6.0 + rp * 1.3, htop + 34.0),
                (hatx - rp * 0.66, htop - 46.0),
                (hatx + rp * 0.66, htop - 46.0),
                (hatx, htop - 46.0),
            ] {
                push_head!(x, y, 4.5);
            }
        }
        _ => {}
    }
    // Neckwear extremes that can leave the bean's sampled edge:
    // the scarf band (bx +/- 70 at top + 4, knot to top + 76)
    // and the collar triangles (bx +/- 56 at top + 2).
    match look.neck.as_str() {
        "scarf" => {
            push!(bx - 70.0, top + 4.0, 3.0);
            push!(bx + 70.0, top + 4.0, 3.0);
            push!(bx, top + 76.0, 3.0);
        }
        "collar" => {
            push!(bx - 56.0, top + 2.0, 3.0);
            push!(bx + 56.0, top + 2.0, 3.0);
        }
        _ => {}
    }
    // Arms — painter FK, quadratic samples + the hand ellipse
    // (radii 15 x 17; the envelope uses the 17 disc, as TS).
    for (side, (a, e)) in [(-1.0f32, st.arm_l), (1.0f32, st.arm_r)] {
        let (sx, sy) = (bx + side * 66.0, top + 46.0);
        let a1 = (a as f64).to_radians();
        let a2 = ((a + e) as f64).to_radians();
        let ex = sx + side * (a1.sin() as f32) * 118.0;
        let ey = sy + (a1.cos() as f32) * 118.0;
        let hxp = ex + side * (a2.sin() as f32) * 112.0;
        let hyp = ey + (a2.cos() as f32) * 112.0;
        for (x, y) in quad_pts((sx, sy), (ex, ey), (hxp, hyp), 8) {
            push!(x, y, 7.0);
        }
        push!(hxp, hyp, 17.0);
    }
    // Legs — the painter's branches (stroke lw 15 -> radius
    // 7.5; foot ellipse rx 20 at (fx + side*10, fy - 4)).
    for (sx, side) in [(bx - 42.0, -1.0f32), (bx + 42.0, 1.0f32)] {
        if sit {
            let (kx, ky) = (sx + side * 30.0, hip_y + 44.0);
            let (fx, fy) = (sx + side * 34.0, (hip_y + 220.2).min(1000.0));
            push_leg!(sx, hip_y, 7.5);
            push_leg!(kx, ky, 7.5);
            push_leg!(fx, fy, 7.5);
            push_leg!(fx + side * 10.0, fy - 4.0, 20.0);
            continue;
        }
        let (kx, ky, fx, fy) = match st.legs {
            Legs::Walk | Legs::Run => {
                let amp = if st.legs == Legs::Run { 1.0 } else { 0.55 };
                let a = (st.phase + if side > 0.0 { std::f32::consts::PI } else { 0.0 }).sin() * amp;
                let lift = (-(st.phase + if side > 0.0 { std::f32::consts::PI } else { 0.0 }).cos())
                    .max(0.0)
                    * if st.legs == Legs::Run { 70.0 } else { 34.0 };
                (
                    sx + a * 22.0 + side * 6.0,
                    hip_y + 200.0 - lift * 0.6,
                    sx + a * 34.0 + side * 8.0,
                    1000.0 - lift,
                )
            }
            Legs::Jump => (sx + side * 40.0, hip_y + 170.0, sx + side * 18.0, 940.0),
            _ => (sx + side * 4.0, hip_y + 210.0, sx + side * 10.0, 1000.0),
        };
        for (x, y) in quad_pts((sx, hip_y), (kx, ky), (fx, fy), 8) {
            push_leg!(x, y, 7.5);
        }
        push_leg!(fx + side * 10.0, fy - 4.0, 20.0);
    }
    // Excursions against the canvas bounds (stage coords), inset
    // for paint AA — the TS bounds verbatim: the stage sits at
    // +(384, 256) inside the 1280x1408 canvas.
    let (bl, br) = (-384.0 + INSET, 1280.0 - 384.0 - INSET);
    let (bt, bb) = (-256.0 + INSET, 1408.0 - 256.0 - INSET);
    let (mut minx, mut maxx, mut miny, mut maxy) = (f32::MAX, f32::MIN, f32::MAX, f32::MIN);
    for &(x, y, rad) in &pts {
        minx = minx.min(x - rad);
        maxx = maxx.max(x + rad);
        miny = miny.min(y - rad);
        maxy = maxy.max(y + rad);
    }
    let mut ax = 0.0f32;
    let mut ay = 0.0f32;
    if maxx - minx <= br - bl {
        if minx < bl {
            ax = bl - minx;
        } else if maxx > br {
            ax = br - maxx;
        }
    } else {
        ax = (bl + br) / 2.0 - (minx + maxx) / 2.0;
    }
    if maxy - miny <= bb - bt {
        if miny < bt {
            ay = bt - miny;
        } else if maxy > bb {
            ay = bb - maxy;
        }
    } else {
        ay = (bt + bb) / 2.0 - (miny + maxy) / 2.0;
    }
    (ax, ay)
}

/// The full painter. Returns (pixels, w, h, hands) with hands in
/// stage coordinates, exactly as drawToon returns them.
pub(crate) fn paint_toon_full(
    look: &FullLook,
    st: &FullState,
) -> (Vec<u8>, u32, u32, (f32, f32), (f32, f32)) {
    let (cw, ch) = (1280usize * 2, 1408usize * 2);
    let mut p = Painter::new(cw, ch);
    let ink = rgba(INK);
    let lw = |w: f32| w * RES;
    let th = st.lean.to_radians();
    let (sth, cth) = th.sin_cos();
    // Canvas re-anchor (B-1, canvas_anchor): the translation is
    // applied in stage space AFTER the lean rotation — the
    // outermost transform, exactly as drawToonLayer applies it —
    // and only shifts paint when the state's envelope would
    // otherwise leave the canvas.
    let (anx, any) = canvas_anchor(look, st);
    // ---- Batch 3 transform stack (flip §5d + bodyPitch Spec 3) ----
    // Composition, outermost first, exactly as TS drawToonLayer:
    //   anchor ∘ flip ∘ lean ∘ pitch ∘ headTilt(effective)
    // `px` below is the LEG map (lean -> flip -> anchor; legs are
    // never pitched). `pxu` is the UPPER-ASSEMBLY map (pitch about
    // the hip first, then `px`) used by the bean / pattern /
    // neckwear / arms. `hpt` is the head map (squash -> effective
    // tilt -> pitch -> `px`). The flip is a stage-space reflection
    // x -> 512 - x applied AFTER the lean rotation and BEFORE the
    // anchor translation (the anchor was solved in mirrored
    // space — see canvas_anchor). At flip=false / pitch=0 every
    // added step is skipped or the identity and the maps are the
    // pre-Batch-3 maps, byte for byte.
    let flip = st.flip;
    let sgn: f32 = if flip { -1.0 } else { 1.0 };
    let pitch_deg = st.body_pitch;
    let pitch_rad = pitch_deg.to_radians();
    // Stage -> canvas: lean about the feet (256, 1000), then the
    // flip mirror, then the canvas offset, then res (as
    // paint_toon / Toon#set).
    let px = |x: f32, y: f32| -> (f32, f32) {
        let (dx, dy) = (x - 256.0, y - 1000.0);
        let lx = 256.0 + dx * cth - dy * sth;
        (
            ((if flip { 512.0 - lx } else { lx }) + anx + 384.0) * RES,
            (1000.0 + dx * sth + dy * cth + any + 256.0) * RES,
        )
    };
    let sit = st.legs == Legs::Sit;
    let jump = st.legs == Legs::Jump;
    let sit_s = st.sit_drop.unwrap_or(220.2);
    let by = (if sit { (426.0 - sit_s) / 0.98 } else { 0.0 })
        + if look.kid { 190.0 } else { 0.0 }
        + st.bob
        + if jump { -60.0 } else { 0.0 };
    let (bx, top, bot, bw) = (256.0f32, 330.0 + by, 600.0 + by * 0.98, 92.0f32);
    let hip_y = bot - 26.0;
    // Upper-assembly point map: pitch about the hip pivot first
    // (identity at pitch 0 via pitch_pt's exact early return),
    // then the leg map's lean/flip/anchor stack.
    let pxu = |x: f32, y: f32| -> (f32, f32) {
        let (ux, uy) = pitch_pt(x, y, bx, hip_y, pitch_deg);
        px(ux, uy)
    };
    let (hx, hy, r) = (256.0f32, 178.0 + by * 0.99, 138.0f32);
    let pivot_y = 318.0 + by * 0.99;
    // Head counter-tilt (Spec 3): the head painter receives the
    // EFFECTIVE tilt headTilt - bodyPitch*bodyPitchCounter about
    // the neck pivot, inside the pitched frame.
    let tilt = effective_head_tilt(st).to_radians();
    let (sti, cti) = tilt.sin_cos();
    let sq = st.squash;
    let hpt = |x: f32, y: f32| -> (f32, f32) {
        // squash about the head centre
        let (sx, sy) = (hx + (x - hx) * (1.0 - 0.3 * sq), hy + (y - hy) * (1.0 + sq));
        // effective tilt about the neck pivot
        let (dx, dy) = (sx - 256.0, sy - pivot_y);
        let (rx, ry) = (256.0 + dx * cti - dy * sti, pivot_y + dx * sti + dy * cti);
        // pitch about the hip, then lean/flip/anchor via px.
        let (ux, uy) = pitch_pt(rx, ry, bx, hip_y, pitch_deg);
        px(ux, uy)
    };
    // Canvas-space rotation angles for the ellipse calls (the
    // painter passes rotations explicitly, unlike the TS canvas
    // transform): a stage rotation angle is negated by the flip
    // mirror, and the pitch adds to every upper-assembly angle.
    // head_rot's net world tilt is headTilt + pitch*(1-counter)
    // before the mirror, per Spec 3.
    let head_rot = sgn * (th + tilt + pitch_rad);
    let th_up = sgn * (th + pitch_rad); // upper assembly, non-head
    let th_leg = sgn * th; // legs: lean only, mirrored

    // ---- legs ----
    for (sx0, side) in [(bx - 42.0, -1.0f32), (bx + 42.0, 1.0f32)] {
        if sit {
            let (kx, ky) = (sx0 + side * 30.0, hip_y + 44.0);
            let (fx, fy) = (sx0 + side * 34.0, (hip_y + 220.2).min(1000.0));
            p.stroke_polyline(
                &[px(sx0, hip_y), px(kx, ky), px(fx, fy)],
                lw(15.0),
                ink,
            );
            let (ex, ey) = px(fx + side * 10.0, fy - 4.0);
            p.fill_ellipse(ex, ey, 20.0 * RES, 10.0 * RES, th_leg, ink);
            continue;
        }
        let (kx, ky, fx, fy) = match st.legs {
            Legs::Walk | Legs::Run => {
                let amp = if st.legs == Legs::Run { 1.0 } else { 0.55 };
                let a = (st.phase + if side > 0.0 { std::f32::consts::PI } else { 0.0 }).sin() * amp;
                let lift = (-(st.phase + if side > 0.0 { std::f32::consts::PI } else { 0.0 }).cos())
                    .max(0.0)
                    * if st.legs == Legs::Run { 70.0 } else { 34.0 };
                (
                    sx0 + a * 22.0 + side * 6.0,
                    hip_y + 200.0 - lift * 0.6,
                    sx0 + a * 34.0 + side * 8.0,
                    1000.0 - lift,
                )
            }
            Legs::Jump => (sx0 + side * 40.0, hip_y + 170.0, sx0 + side * 18.0, 940.0),
            _ => (sx0 + side * 4.0, hip_y + 210.0, sx0 + side * 10.0, 1000.0),
        };
        let pts = flatten_quad(px(sx0, hip_y), px(kx, ky), px(fx, fy), 20);
        p.stroke_polyline(&pts, lw(15.0), ink);
        let (ex, ey) = px(fx + side * 10.0, fy - 4.0);
        p.fill_ellipse(ex, ey, 20.0 * RES, 10.0 * RES, th_leg, ink);
    }

    // ---- body bean ----
    let mut bean: Vec<(f32, f32)> = Vec::new();
    let mut seg = |pts: Vec<(f32, f32)>| {
        if bean.is_empty() {
            bean.extend_from_slice(&pts);
        } else {
            bean.extend_from_slice(&pts[1..]);
        }
    };
    seg(flatten_cubic(pxu(bx - 58.0, top + 10.0), pxu(bx - 80.0, top + 40.0), pxu(bx - bw - 6.0, bot - 120.0), pxu(bx - bw, bot - 50.0), 28));
    seg(flatten_cubic(pxu(bx - bw, bot - 50.0), pxu(bx - bw + 4.0, bot + 6.0), pxu(bx + bw - 4.0, bot + 6.0), pxu(bx + bw, bot - 50.0), 28));
    seg(flatten_cubic(pxu(bx + bw, bot - 50.0), pxu(bx + bw + 6.0, bot - 120.0), pxu(bx + 80.0, top + 40.0), pxu(bx + 58.0, top + 10.0), 28));
    seg(flatten_cubic(pxu(bx + 58.0, top + 10.0), pxu(bx + 30.0, top - 8.0), pxu(bx - 30.0, top - 8.0), pxu(bx - 58.0, top + 10.0), 28));
    p.fill_polygon(&bean, rgba(&look.body));
    let mask = p.polygon_mask(&bean);
    // Pattern (clipped to the bean, as drawPattern runs inside the
    // body's clip): only 'vest' occurs in this film besides plain.
    if look.pattern == "stripes" {
        // drawPattern 'stripes': fillRect(x, top-20, 14, bot-top+40)
        // for x = bx-130; x < bx+130; x += 36, colour
        // patCol || shadeOf(body, 0.7), full alpha. Masked rects
        // reproduce the bean clip exactly.
        let pc = match &look.pat_col {
            Some(s) => rgba(s.as_str()),
            None => shade_rgba(&look.body, 0.7),
        };
        let mut sx = bx - 130.0;
        while sx < bx + 130.0 {
            let rect = [
                pxu(sx, top - 20.0),
                pxu(sx + 14.0, top - 20.0),
                pxu(sx + 14.0, bot + 20.0),
                pxu(sx, bot + 20.0),
            ];
            fill_polygon_masked(&mut p, &rect, &mask, pc);
            sx += 36.0;
        }
    }
    if look.pattern == "apron" {
        // drawPattern 'apron': roundRect(bx-70, top+90, 140,
        // bot-top, 18) fill patCol || '#F4EFE4'; pocket
        // roundRect(bx-40, top+190, 80, 60, 8) STROKE
        // shadeOf(patCol, 0.8) lw4. The pocket stroke is unmasked:
        // probe-verified wholly inside the bean (min distance from
        // the pocket contour to the bean boundary 3.79 stage units
        // over the by range, vs the stroke's 2.0 half-width — see
        // CR-B3-06), so it is pixel-equal to TS's clipped stroke.
        let pc_hex = look.pat_col.as_deref().unwrap_or("#F4EFE4");
        let body_pts: Vec<(f32, f32)> =
            rounded_rect_pts_radii(bx - 70.0, top + 90.0, 140.0, bot - top, [18.0; 4], 6)
                .iter()
                .map(|&(x, y)| pxu(x, y))
                .collect();
        fill_polygon_masked(&mut p, &body_pts, &mask, rgba(pc_hex));
        let pocket: Vec<(f32, f32)> =
            rounded_rect_pts_radii(bx - 40.0, top + 190.0, 80.0, 60.0, [8.0; 4], 6)
                .iter()
                .map(|&(x, y)| pxu(x, y))
                .collect();
        let mut ring = pocket.clone();
        ring.push(pocket[0]);
        p.stroke_polyline(&ring, lw(4.0), shade_rgba(pc_hex, 0.8));
    }
    if look.pattern == "vest" {
        let pc = look.pat_col.as_deref().unwrap_or("#3B3346");
        for s in [-1.0f32, 1.0] {
            let panel = [
                pxu(bx + s * 140.0, top - 10.0),
                pxu(bx + s * 8.0, top + 120.0),
                pxu(bx + s * 8.0, bot + 20.0),
                pxu(bx + s * 140.0, bot + 20.0),
            ];
            fill_polygon_masked(&mut p, &panel, &mask, rgba(pc));
        }
        for dy in [150.0f32, 195.0, 240.0] {
            let (cxp, cyp) = pxu(bx + 26.0, top + dy);
            p.fill_ellipse(cxp, cyp, 6.0 * RES, 6.0 * RES, th_up, rgba("#E9C46A"));
        }
    }
    // Shade + highlight, clipped.
    let (shx, shy) = pxu(bx + bw + 40.0, (top + bot) / 2.0 + 20.0);
    let shade_col = match &look.shade {
        Some(s) => rgba(s.as_str()),
        None => shade_rgba(&look.body, 0.8),
    };
    p.fill_ellipse_masked(shx, shy, 70.0 * RES, 260.0 * RES, th_up, (shade_col.0, shade_col.1, shade_col.2, (0.85 * 255.0) as u8), &mask);
    let (hix, hiy) = pxu(bx - 50.0, top + 90.0);
    p.fill_ellipse_masked(hix, hiy, 16.0 * RES, 60.0 * RES, sgn * 0.2 + th_up, (255, 255, 255, (0.25 * 255.0) as u8), &mask);
    {
        let mut ring = bean.clone();
        ring.push(bean[0]);
        p.stroke_polyline(&ring, lw(9.0), ink);
    }

    // ---- neckwear ----
    match look.neck.as_str() {
        "tie" => {
            let c = look.neck_col.as_deref().unwrap_or("#D8343F");
            let poly = [
                pxu(bx - 14.0, top + 10.0),
                pxu(bx + 14.0, top + 10.0),
                pxu(bx + 20.0, top + 140.0),
                pxu(bx, top + 170.0),
                pxu(bx - 20.0, top + 140.0),
            ];
            p.fill_polygon(&poly, rgba(c));
            let mut ring = poly.to_vec();
            ring.push(poly[0]);
            p.stroke_polyline(&ring, lw(6.0), ink);
        }
        "scarf" => {
            let c = look.neck_col.as_deref().unwrap_or("#D8343F");
            let tail = [
                pxu(bx + 20.0, top + 20.0),
                pxu(bx + 58.0, top + 130.0),
                pxu(bx + 24.0, top + 138.0),
                pxu(bx + 6.0, top + 26.0),
            ];
            p.fill_polygon(&tail, rgba(c));
            let mut ring = tail.to_vec();
            ring.push(tail[0]);
            p.stroke_polyline(&ring, lw(6.0), ink);
            // Wrap: two quads forming the band, then its shade line.
            let mut band: Vec<(f32, f32)> = Vec::new();
            band.extend_from_slice(&flatten_quad(pxu(bx - 70.0, top + 4.0), pxu(bx, top + 44.0), pxu(bx + 70.0, top + 4.0), 16));
            let lower = flatten_quad(pxu(bx + 70.0, top + 4.0), pxu(bx + 66.0, top + 34.0), pxu(bx, top + 76.0), 16);
            band.extend_from_slice(&lower[1..]);
            let lower2 = flatten_quad(pxu(bx, top + 76.0), pxu(bx - 66.0, top + 34.0), pxu(bx - 70.0, top + 4.0), 16);
            band.extend_from_slice(&lower2[1..]);
            p.fill_polygon(&band, rgba(c));
            let mut bring = band.clone();
            bring.push(band[0]);
            p.stroke_polyline(&bring, lw(6.0), ink);
            let shade = shade_rgba(c, 0.75);
            p.stroke_quad(
                pxu(bx - 50.0, top + 26.0),
                pxu(bx, top + 56.0),
                pxu(bx + 50.0, top + 26.0),
                lw(4.0),
                shade,
            );
        }
        "collar" => {
            // drawNeckwear 'collar': fill '#FFFFFF' (neckCol is
            // IGNORED for collar, as authored), per side the
            // triangle (bx, top+18), (bx±56, top+2), (bx±40,
            // top+46); fill + stroke lw6.
            for s in [-1.0f32, 1.0] {
                let tri = [
                    pxu(bx, top + 18.0),
                    pxu(bx + s * 56.0, top + 2.0),
                    pxu(bx + s * 40.0, top + 46.0),
                ];
                p.fill_polygon(&tri, rgba("#FFFFFF"));
                let mut ring = tri.to_vec();
                ring.push(tri[0]);
                p.stroke_polyline(&ring, lw(6.0), ink);
            }
        }
        _ => {}
    }

    // ---- arms (+ held plates, then the hand) ----
    let mut hands = ((0.0f32, 0.0f32), (0.0f32, 0.0f32));
    for (side, (a, e), hold) in [
        (-1.0f32, st.arm_l, &st.hold_l),
        (1.0f32, st.arm_r, &st.hold_r),
    ] {
        let (sx, sy) = (bx + side * 66.0, top + 46.0);
        let a1 = (a as f64).to_radians();
        let a2 = ((a + e) as f64).to_radians();
        let ex = sx + side * (a1.sin() as f32) * 118.0;
        let ey = sy + (a1.cos() as f32) * 118.0;
        let hx2 = ex + side * (a2.sin() as f32) * 112.0;
        let hy2 = ey + (a2.cos() as f32) * 112.0;
        let pts = flatten_quad(pxu(sx, sy), pxu(ex, ey), pxu(hx2, hy2), 20);
        p.stroke_polyline(&pts, lw(14.0), ink);
        if let Some(plate) = hold {
            composite_plate(&mut p, plate, (hx2, hy2), th, (anx, any), flip, pitch_rad, (bx, hip_y));
        }
        let (hxp, hyp) = pxu(hx2, hy2);
        p.fill_ellipse(hxp, hyp, 15.0 * RES, 17.0 * RES, th_up, ink);
        // Returned hands are the PITCHED stage positions (Spec 3,
        // as TS rotPt) — the plate above keeps the UN-pitched
        // hand as its figure-local origin. Flip does not change
        // the returned stage coordinates (TS returns pre-flip
        // stage hands; only pitch is folded in).
        let hand_ret = pitch_pt(hx2, hy2, bx, hip_y, pitch_deg);
        if side < 0.0 {
            hands.0 = hand_ret;
        } else {
            hands.1 = hand_ret;
        }
    }

    // ---- head ----
    let turn = st.head_turn;
    let rp = r * (1.0 + 0.04 * st.cam_pitch);
    let fx = hx + turn * 30.0;
    // Appendages on the head, BEHIND the head disc so the bases
    // tuck under it (TS drawHeadPart order: ears, then hair,
    // then the bun, then the disc). All geometry is computed in
    // head space and mapped through `hpt`, which applies this
    // painter's squash / headTilt / lean stack exactly as the TS
    // transform nest (tilt sandwich outside, squash inside) does
    // for every other head part.
    if look.ears {
        // Spec §5b fox ears. Per side s: ea = (s<0 ? earAL :
        // earAR) ?? earA (per-ear wins per side; earA default 0);
        // anchor (hx + s*rp*0.52, hy - rp*0.74); rotate by
        // (s*0.16 + ea) about the anchor. Outer triangle
        // (-28, 8), (0, -88), (28, 8): fill = look body colour,
        // stroke INK at the appendage width (default 7). Inner
        // triangle (-14, 2), (0, -52), (14, 2): fill '#F7F1E3',
        // no stroke.
        for s in [-1.0f32, 1.0] {
            let ea = (if s < 0.0 { st.ear_al } else { st.ear_ar }).unwrap_or(st.ear_a);
            let (exx, eyy) = (hx + s * rp * 0.52, hy - rp * 0.74);
            let (sa, ca) = (s * 0.16 + ea).sin_cos();
            let rot = |lx: f32, ly: f32| -> (f32, f32) {
                hpt(exx + lx * ca - ly * sa, eyy + lx * sa + ly * ca)
            };
            let outer = [rot(-28.0, 8.0), rot(0.0, -88.0), rot(28.0, 8.0)];
            p.fill_polygon(&outer, rgba(&look.body));
            let mut ring = outer.to_vec();
            ring.push(outer[0]);
            p.stroke_polyline(&ring, lw(7.0), ink);
            let inner = [rot(-14.0, 2.0), rot(0.0, -52.0), rot(14.0, 2.0)];
            p.fill_polygon(&inner, rgba("#F7F1E3"));
        }
    }
    if look.hair {
        // Spec §5c hair strands. Strand count: the hairSegs
        // array length SETS it; the plumbed representation
        // (Vec, empty = absent) maps absent to the TS default
        // of 6 rest strands. Two passes in the TS order (ALL
        // ink strands first, then ALL colour strands): pass 0
        // INK lw 17, pass 1 hairCol (|| '#6B4A3A') lw 11.
        // NOTE: hair has NO canvas envelope (§5c) — canvas_anchor
        // deliberately does not model these strands.
        let n = if st.hair_segs.is_empty() { 6 } else { st.hair_segs.len() };
        let col = rgba(look.hair_col.as_deref().unwrap_or("#6B4A3A"));
        for pass in 0..2 {
            let (w, c) = if pass == 0 { (17.0, ink) } else { (11.0, col) };
            for k in 0..n {
                let arc_a = if n > 1 {
                    -1.0 + 2.0 * k as f32 / (n - 1) as f32
                } else {
                    0.0
                };
                let root = (
                    hx + arc_a.sin() * rp * 0.98,
                    hy - arc_a.cos() * rp * 0.95,
                );
                let a = st.hair_segs.get(k).copied().unwrap_or(0.0);
                let tip = (root.0 + a.sin() * 74.0, root.1 + a.cos() * 74.0);
                let ctrl = (
                    root.0 + a.sin() * 36.0 + arc_a.cos() * 7.0,
                    root.1 + a.cos() * 36.0,
                );
                p.stroke_quad(hpt(root.0, root.1), hpt(ctrl.0, ctrl.1), hpt(tip.0, tip.1), lw(w), c);
            }
        }
    }
    if look.hat == "bun" {
        // drawHeadPart: the bun is drawn BEFORE the head disc (it
        // is not a drawHat branch): circle (hx + 10 + turn*18,
        // hy - rp - 8) r46, fill hatCol || '#6B4A3A', INK lw LW.
        let bc = rgba(look.hat_col.as_deref().unwrap_or("#6B4A3A"));
        let (bcx, bcy) = hpt(hx + 10.0 + turn * 18.0, hy - rp - 8.0);
        let (brx, bry) = (46.0 * (1.0 - 0.3 * sq) * RES, 46.0 * (1.0 + sq) * RES);
        p.fill_ellipse(bcx, bcy, brx, bry, head_rot, bc);
        p.stroke_ellipse(bcx, bcy, brx, bry, head_rot, lw(9.0), ink);
    }
    // Head outline.
    let (chx, chy) = hpt(hx, hy);
    p.fill_ellipse(chx, chy, rp * (1.0 - 0.3 * sq) * RES, rp * 0.97 * (1.0 + sq) * RES, head_rot, rgba(&look.head));
    // Even-odd shade: inside the head ellipse, outside the offset
    // circle (coverage form, as paint_toon).
    {
        let (ccx, ccy) = hpt(hx + 52.0 + turn * 30.0, hy + 44.0);
        let shade = shade_rgb(&look.head, 0.9);
        let col = (shade.0, shade.1, shade.2, 255u8);
        let (rx_h, ry_h) = (rp * (1.0 - 0.3 * sq) * RES, rp * 0.97 * (1.0 + sq) * RES);
        let (rx_c, ry_c) = (rp * (1.0 - 0.3 * sq) * RES, rp * (1.0 + sq) * RES);
        let rad = rp * RES * 1.05;
        let (x0, y0, x1, y1) = p.bbox(chx - rad, chy - rad, chx + rad, chy + rad);
        #[cfg(feature = "gpupaint")]
        crate::gputrace::trace_head_shade(0, (chx, chy), (rx_h, ry_h), (ccx, ccy), (rx_c, ry_c), head_rot, rad, col);
        for y in y0..=y1 {
            for x in x0..=x1 {
                let (pxx, pyy) = (x as f32 + 0.5, y as f32 + 0.5);
                let cov_head = Painter::cov_of(ellipse_sd(chx, chy, rx_h, ry_h, head_rot, pxx, pyy));
                if cov_head <= 0.0 {
                    continue;
                }
                let d = ellipse_sd(ccx, ccy, rx_c, ry_c, head_rot, pxx, pyy);
                let unshaded = (0.5 - d).clamp(0.0, 1.0);
                let cov = cov_head * unshaded;
                if cov > 0.0 {
                    p.blend_cov(x, y, col, cov);
                }
            }
        }
    }
    p.stroke_ellipse(chx, chy, rp * (1.0 - 0.3 * sq) * RES, rp * 0.97 * (1.0 + sq) * RES, head_rot, lw(10.0), ink);

    // ---- face ----
    let m = st.mood;
    let ex = 46.0 * (1.0 - 0.18 * turn.abs());
    let ey = hy + 4.0 + st.cam_pitch * 8.0;
    let (lift0, tilt0) = m.brow();
    for s in [-1.0f32, 1.0] {
        let mut lift = lift0;
        let mut tilt = tilt0 * s;
        if (m == Mood::Smug || m == Mood::Sly) && s > 0.0 {
            lift = 16.0;
            tilt = -4.0;
        }
        if m == Mood::Suspicious && s < 0.0 {
            lift = 18.0;
            tilt = -6.0;
        }
        if m == Mood::Puzzled {
            // drawFace: one brow up, one brow down — the whole read.
            if s < 0.0 {
                lift = 16.0;
                tilt = -2.0;
            } else {
                lift = -2.0;
                tilt = 8.0;
            }
        }
        // Spec 5a: post-table brow offsets, applied AFTER the
        // per-side row/override resolution above. No clamp, no
        // quantisation (the fixture's 0.5 steps must distinguish).
        lift += st.brow_lift;
        tilt += st.brow_tilt * s;
        let y = ey - 44.0 - lift;
        p.stroke_quad(
            hpt(fx + s * (ex - 26.0), y + tilt * 0.5),
            hpt(fx + s * ex, y - 7.0),
            hpt(fx + s * (ex + 26.0), y - tilt * 0.5),
            lw(12.0),
            ink,
        );
    }
    // Eyes.
    for s in [-1.0f32, 1.0] {
        let x = fx + s * ex;
        let [lx, ly] = st.look;
        if m == Mood::Happy || m == Mood::Laugh || m == Mood::Relieved {
            let pts: Vec<(f32, f32)> = (0..=24)
                .map(|i| {
                    let a = std::f32::consts::PI * 1.15 + (i as f32 / 24.0) * std::f32::consts::PI * 0.7;
                    hpt(x + 20.0 * a.cos(), ey + 10.0 + 20.0 * a.sin())
                })
                .collect();
            p.stroke_polyline(&pts, lw(10.0), ink);
            continue;
        }
        if m == Mood::Shock || m == Mood::Awe {
            let (exp, eyp) = hpt(x, ey);
            p.fill_ellipse(exp, eyp, 26.0 * RES, 30.0 * RES, head_rot, rgba("#FFFFFF"));
            p.stroke_ellipse(exp, eyp, 26.0 * RES, 30.0 * RES, head_rot, lw(6.0), ink);
            let (ppx, ppy) = hpt(x + lx * 8.0, ey + ly * 8.0);
            let pr = if m == Mood::Awe { 9.0 } else { 7.0 };
            p.fill_ellipse(ppx, ppy, pr * RES, pr * RES, head_rot, ink);
            if m == Mood::Awe {
                let (spx, spy) = hpt(x + lx * 8.0 - 3.0, ey + ly * 8.0 - 3.0);
                p.fill_ellipse(spx, spy, 3.0 * RES, 3.0 * RES, head_rot, rgba("#FFFFFF"));
            }
            continue;
        }
        if st.blink > 0.6 {
            p.stroke_quad(
                hpt(x - 14.0, ey + 4.0),
                hpt(x, ey + 10.0),
                hpt(x + 14.0, ey + 4.0),
                lw(8.0),
                ink,
            );
            continue;
        }
        let (ppx, ppy) = hpt(x + lx * 9.0, ey + ly * 8.0);
        let slit = if m == Mood::Suspicious && s < 0.0 { 0.55 } else { 1.0 };
        let deadpan = if m == Mood::Deadpan { 0.8 } else { 1.0 };
        p.fill_ellipse(
            ppx,
            ppy,
            13.0 * RES,
            17.0 * (1.0 - st.blink * 0.6) * slit * deadpan * RES,
            head_rot,
            ink,
        );
        if m.heavy_lid() {
            // Head-colour rect over the top half + the lid line.
            let rect_pts = [
                hpt(x - 24.0, ey - 28.0),
                hpt(x + 24.0, ey - 28.0),
                hpt(x + 24.0, ey - 2.0 + m.lid_offset()),
                hpt(x - 24.0, ey - 2.0 + m.lid_offset()),
            ];
            p.fill_polygon(&rect_pts, rgba(&look.head));
            p.stroke_polyline(
                &[
                    hpt(x - 20.0, ey - 2.0 + m.lid_offset()),
                    hpt(x + 20.0, ey - 2.0 + m.lid_offset()),
                ],
                lw(7.0),
                ink,
            );
        }
        if m == Mood::Sad || m == Mood::Worried {
            let (spx, spy) = hpt(x + lx * 9.0 - 3.0, ey + ly * 8.0 - 5.0);
            p.fill_ellipse(spx, spy, 3.5 * RES, 3.5 * RES, head_rot, rgba("#FFFFFF"));
        }
    }
    if look.glasses {
        for s in [-1.0f32, 1.0] {
            let (gx, gy) = hpt(fx + s * ex, ey);
            p.stroke_ellipse(gx, gy, 30.0 * RES, 30.0 * RES, head_rot, lw(5.0), ink);
        }
        p.stroke_polyline(&[hpt(fx - 16.0, ey - 4.0), hpt(fx + 16.0, ey - 4.0)], lw(5.0), ink);
    }
    if look.monocle {
        // drawFace 'monocle': stroke '#C9A13B' lw5 — circle r30 at
        // the figure's right eye (fx + ex, ey), then the chain
        // quadratic from (fx+ex+22, ey+22) via (fx+ex+40, ey+90)
        // to (fx+ex+10, ey+140).
        let gold = rgba("#C9A13B");
        let (mxp, myp) = hpt(fx + ex, ey);
        p.stroke_ellipse(mxp, myp, 30.0 * RES, 30.0 * RES, head_rot, lw(5.0), gold);
        p.stroke_quad(
            hpt(fx + ex + 22.0, ey + 22.0),
            hpt(fx + ex + 40.0, ey + 90.0),
            hpt(fx + ex + 10.0, ey + 140.0),
            lw(5.0),
            gold,
        );
    }
    if look.blush || m == Mood::Laugh || m == Mood::Proud || m == Mood::Embarrassed {
        // drawFace: extra 'blush' OR mood in {laugh, proud,
        // embarrassed} — fill rgba(240,120,130,0.35), ellipses
        // (fx ± 78, ey + 40) rx22 ry12.
        let blush = (240u8, 120u8, 130u8, (0.35 * 255.0) as u8);
        for s in [-1.0f32, 1.0] {
            let (bxp, byp) = hpt(fx + s * 78.0, ey + 40.0);
            p.fill_ellipse(bxp, byp, 22.0 * RES, 12.0 * RES, head_rot, blush);
        }
    }
    if look.freckles {
        // drawFace 'freckles': fill '#C98A64', per side s three
        // r3.5 discs at (fx + s*(74+dx), ey + 36 + dy) for
        // (dx, dy) in {(0,0), (12,8), (-10,10)} — dx sits INSIDE
        // the side multiplication, as authored.
        for s in [-1.0f32, 1.0] {
            for (dx, dy) in [(0.0f32, 0.0f32), (12.0, 8.0), (-10.0, 10.0)] {
                let (cxp, cyp) = hpt(fx + s * (74.0 + dx), ey + 36.0 + dy);
                p.fill_ellipse(cxp, cyp, 3.5 * RES, 3.5 * RES, head_rot, rgba("#C98A64"));
            }
        }
    }
    // Mouth.
    let my = ey + 62.0;
    let t = st.talk;
    let open_mouth = |p: &mut Painter, w: f32, h: f32, tongue: bool| {
        let (mxp, myp) = hpt(fx, my + h * 0.3);
        p.fill_ellipse(mxp, myp, w * RES, h.max(4.0) * RES, head_rot, rgba("#3A1A2A"));
        p.stroke_ellipse(mxp, myp, w * RES, h.max(4.0) * RES, head_rot, lw(8.0), ink);
        if tongue && h > 12.0 {
            let msk = p.ellipse_mask(mxp, myp, w * RES, h.max(4.0) * RES, head_rot);
            let (txp, typ) = hpt(fx, my + h * 1.1);
            p.fill_ellipse_masked(txp, typ, w * 0.7 * RES, h * 0.6 * RES, head_rot, rgba("#E2697A"), &msk);
        }
    };
    if m == Mood::Shock {
        open_mouth(&mut p, 16.0, 20.0 + t * 6.0, true);
    } else if m == Mood::Laugh {
        // As in drawFace — including its stroke quirk (see
        // paint_toon): the final stroke() lands on the tongue.
        let mut pts: Vec<(f32, f32)> = Vec::new();
        let mut chain = |qs: Vec<(f32, f32)>| {
            if pts.is_empty() {
                pts.extend_from_slice(&qs);
            } else {
                pts.extend_from_slice(&qs[1..]);
            }
        };
        chain(flatten_quad(hpt(fx - 40.0, my - 6.0), hpt(fx, my - 2.0), hpt(fx + 40.0, my - 6.0), 24));
        chain(flatten_quad(hpt(fx + 40.0, my - 6.0), hpt(fx + 34.0, my + 52.0), hpt(fx, my + 52.0), 24));
        chain(flatten_quad(hpt(fx, my + 52.0), hpt(fx - 34.0, my + 52.0), hpt(fx - 40.0, my - 6.0), 24));
        p.fill_polygon(&pts, rgba("#3A1A2A"));
        let msk = p.polygon_mask(&pts);
        let (txp, typ) = hpt(fx, my + 50.0);
        p.fill_ellipse_masked(txp, typ, 24.0 * RES, 18.0 * RES, head_rot, rgba("#E2697A"), &msk);
        let (ax, ay) = hpt(fx - 40.0, my - 10.0);
        let (bxp, byp) = hpt(fx + 40.0, my + 2.0);
        let (x0, y0, x1, y1) = p.bbox(ax.min(bxp), ay.min(byp), ax.max(bxp), ay.max(byp));
        #[cfg(feature = "gpupaint")]
        crate::gputrace::trace_masked_blend((x0, y0, x1, y1), &msk, rgba("#FFFFFF"));
        for yy in y0..=y1 {
            for xx in x0..=x1 {
                if msk[yy as usize * p.w + xx as usize] {
                    p.blend(xx, yy, rgba("#FFFFFF"));
                }
            }
        }
        p.stroke_ellipse(txp, typ, 24.0 * RES, 18.0 * RES, head_rot, lw(8.0), ink);
    } else if t > 0.05 {
        open_mouth(&mut p, 14.0 + t * 10.0, 6.0 + t * 18.0, true);
    } else {
        match m {
            Mood::Happy | Mood::Proud => {
                p.stroke_quad(hpt(fx - 28.0, my), hpt(fx, my + 26.0), hpt(fx + 28.0, my), lw(8.0), ink);
            }
            Mood::Smug | Mood::Sly => {
                p.stroke_quad(hpt(fx - 22.0, my + 6.0), hpt(fx + 4.0, my + 16.0), hpt(fx + 24.0, my - 4.0), lw(8.0), ink);
            }
            Mood::Worried => {
                let pts = flatten_cubic(
                    hpt(fx - 26.0, my + 8.0),
                    hpt(fx - 12.0, my - 4.0),
                    hpt(fx + 8.0, my + 14.0),
                    hpt(fx + 26.0, my + 2.0),
                    24,
                );
                p.stroke_polyline(&pts, lw(8.0), ink);
            }
            Mood::Angry => {
                p.stroke_quad(hpt(fx - 30.0, my + 12.0), hpt(fx, my - 8.0), hpt(fx + 30.0, my + 12.0), lw(8.0), ink);
                let pts: Vec<(f32, f32)> = crate::scene::rounded_rect_pts(fx - 24.0, my - 2.0, 48.0, 14.0, 6.0, 6)
                    .iter()
                    .map(|&(x, y)| hpt(x, y))
                    .collect();
                p.fill_polygon(&pts, rgba("#FFFFFF"));
                let mut ring = pts.clone();
                ring.push(pts[0]);
                p.stroke_polyline(&ring, lw(5.0), ink);
            }
            Mood::Sad => {
                p.stroke_quad(hpt(fx - 24.0, my + 12.0), hpt(fx, my - 4.0), hpt(fx + 24.0, my + 12.0), lw(8.0), ink);
            }
            Mood::Tired | Mood::Dizzy | Mood::Deadpan => {
                p.stroke_polyline(&[hpt(fx - 18.0, my + 6.0), hpt(fx + 18.0, my + 6.0)], lw(8.0), ink);
            }
            Mood::Determined => {
                p.stroke_polyline(&[hpt(fx - 24.0, my + 5.0), hpt(fx + 24.0, my + 1.0)], lw(8.0), ink);
            }
            Mood::Embarrassed => {
                let mut pts = flatten_quad(hpt(fx - 17.0, my + 4.0), hpt(fx - 8.0, my - 2.0), hpt(fx, my + 4.0), 12);
                let q2 = flatten_quad(hpt(fx, my + 4.0), hpt(fx + 8.0, my + 10.0), hpt(fx + 17.0, my + 4.0), 12);
                pts.extend_from_slice(&q2[1..]);
                p.stroke_polyline(&pts, lw(8.0), ink);
            }
            Mood::Awe => {
                open_mouth(&mut p, 11.0, 13.0, false);
            }
            Mood::Suspicious => {
                p.stroke_quad(hpt(fx - 22.0, my), hpt(fx, my + 2.0), hpt(fx + 24.0, my + 9.0), lw(8.0), ink);
            }
            Mood::Puzzled => {
                p.stroke_quad(hpt(fx - 16.0, my + 5.0), hpt(fx, my + 1.0), hpt(fx + 16.0, my - 1.0), lw(8.0), ink);
            }
            Mood::Relieved => {
                p.stroke_quad(hpt(fx - 20.0, my), hpt(fx, my + 18.0), hpt(fx + 20.0, my), lw(8.0), ink);
                let (ox, oy) = hpt(fx, my + 16.0);
                p.fill_ellipse(ox, oy, 5.0 * RES, 3.0 * RES, head_rot, rgba("#3A1A2A"));
            }
            _ => {
                p.stroke_quad(hpt(fx - 20.0, my + 2.0), hpt(fx, my + 14.0), hpt(fx + 20.0, my + 2.0), lw(8.0), ink);
            }
        }
    }
    if look.mustache {
        let mc = look.mustache_col.as_deref().unwrap_or("#3A2A22");
        let mut pts: Vec<(f32, f32)> = Vec::new();
        let mut chain = |qs: Vec<(f32, f32)>| {
            if pts.is_empty() {
                pts.extend_from_slice(&qs);
            } else {
                pts.extend_from_slice(&qs[1..]);
            }
        };
        chain(flatten_cubic(hpt(fx, my - 18.0), hpt(fx - 20.0, my - 32.0), hpt(fx - 60.0, my - 22.0), hpt(fx - 64.0, my - 2.0), 20));
        chain(flatten_cubic(hpt(fx - 64.0, my - 2.0), hpt(fx - 44.0, my - 8.0), hpt(fx - 20.0, my - 4.0), hpt(fx, my - 10.0), 20));
        chain(flatten_cubic(hpt(fx, my - 10.0), hpt(fx + 20.0, my - 4.0), hpt(fx + 44.0, my - 8.0), hpt(fx + 64.0, my - 2.0), 20));
        chain(flatten_cubic(hpt(fx + 64.0, my - 2.0), hpt(fx + 60.0, my - 22.0), hpt(fx + 20.0, my - 32.0), hpt(fx, my - 18.0), 20));
        p.fill_polygon(&pts, rgba(mc));
        let mut ring = pts.clone();
        ring.push(pts[0]);
        p.stroke_polyline(&ring, lw(4.0), ink);
    }
    if look.pencil {
        // drawFace 'pencil' (after mustache): save / translate
        // (fx + 132, hy - 50) / rotate(-0.5): body fillRect
        // (-8, -60, 16, 120) '#F2C230'; eraser fillRect
        // (-8, -70, 16, 14) '#E58A9A'; outline strokeRect
        // (-8, -70, 16, 130) INK lw4. No group-rotation op is
        // needed: the rect corners are rotated by -0.5 about the
        // anchor in stage space (canvas rotate is y-down, so
        // (x, y) -> (x cos - y sin, x sin + y cos) with the angle
        // -0.5), then mapped through hpt, which applies squash /
        // tilt / lean exactly as the TS transform nest does.
        // Anchored to (fx, hy) — the head centre, not ey.
        let (ax, ay) = (fx + 132.0, hy - 50.0);
        let (s5, c5) = (-0.5f32).sin_cos();
        let rot = |lx: f32, ly: f32| -> (f32, f32) {
            hpt(ax + lx * c5 - ly * s5, ay + lx * s5 + ly * c5)
        };
        let rect = |x: f32, y: f32, w: f32, h: f32| -> Vec<(f32, f32)> {
            vec![rot(x, y), rot(x + w, y), rot(x + w, y + h), rot(x, y + h)]
        };
        p.fill_polygon(&rect(-8.0, -60.0, 16.0, 120.0), rgba("#F2C230"));
        p.fill_polygon(&rect(-8.0, -70.0, 16.0, 14.0), rgba("#E58A9A"));
        let mut ring = rect(-8.0, -70.0, 16.0, 130.0);
        ring.push(ring[0]);
        p.stroke_polyline(&ring, lw(4.0), ink);
    }
    // Spec 5e (TS drawFace line 1490): trigger = (sweat present
    // AND != 0.0) OR mood == Worried (the FACE_TABLE forceSweat
    // row); k = sweat.unwrap_or(1.0); alpha = min(1.0, k) applied
    // to BOTH the drop fill and its outline stroke. Worried +
    // sweat 0 therefore emits the drop at alpha 0 (pixel-invisible,
    // matching the TS op stream; that pair is not chased beyond
    // this semantics). Alpha byte follows the house style of the
    // other fractional-alpha paints in this file ((k * 255.0) as u8).
    if st.sweat.map_or(false, |v| v != 0.0) || m == Mood::Worried {
        let k = st.sweat.unwrap_or(1.0);
        let a = (k.min(1.0) * 255.0) as u8;
        let mut drop = rgba("#8FD3F4");
        drop.3 = a;
        let mut ink_a = ink;
        ink_a.3 = a;
        // Sweat drop at (fx + 104, hy - 70): two quads, fill + stroke.
        let (sx, sy) = (fx + 104.0, hy - 70.0);
        let mut pts = flatten_quad(hpt(sx, sy - 26.0), hpt(sx + 18.0, sy), hpt(sx, sy + 12.0), 16);
        let q2 = flatten_quad(hpt(sx, sy + 12.0), hpt(sx - 18.0, sy), hpt(sx, sy - 26.0), 16);
        pts.extend_from_slice(&q2[1..]);
        p.fill_polygon(&pts, drop);
        let mut ring = pts.clone();
        ring.push(pts[0]);
        p.stroke_polyline(&ring, lw(4.0), ink_a);
    }
    if st.tears || m == Mood::Sad {
        for s in [-1.0f32, 1.0] {
            let (txp, typ) = hpt(fx + s * (ex + 6.0), ey + 40.0);
            p.fill_ellipse(txp, typ, 7.0 * RES, 14.0 * RES, head_rot, rgba("#8FD3F4"));
        }
    }

    // ---- hat (centre lags turn*18) ----
    let hatx = hx + turn * 18.0;
    let htop = hy - rp * 0.97;
    match look.hat.as_str() {
        "cap" => {
            let cap_hex = look.hat_col.as_deref().unwrap_or("#7A5236");
            let cap = rgba(cap_hex);
            {
                let (ccx, ccy) = hpt(hatx + 6.0, htop + 34.0);
                let (rx, ry, rot) = (rp * 0.98 * (1.0 - 0.3 * sq) * RES, 62.0 * (1.0 + sq) * RES, -0.05 * sgn + head_rot);
                let (s3, c3) = rot.sin_cos();
                let mut pts: Vec<(f32, f32)> = Vec::new();
                for i in 0..=48 {
                    let a = std::f32::consts::PI + (i as f32 / 48.0) * std::f32::consts::PI;
                    let (lx2, ly2) = (rx * a.cos(), ry * a.sin());
                    pts.push((ccx + lx2 * c3 - ly2 * s3, ccy + lx2 * s3 + ly2 * c3));
                }
                p.fill_polygon(&pts, cap);
                let mut ring = pts.clone();
                ring.push(pts[0]);
                p.stroke_polyline(&ring, lw(9.0), ink);
            }
            {
                let (bcx, bcy) = hpt(hatx - 24.0, htop + 40.0);
                let (rx, ry, rot) = (rp * 0.78 * (1.0 - 0.3 * sq) * RES, 22.0 * (1.0 + sq) * RES, 0.05 * sgn + head_rot);
                let (s3, c3) = rot.sin_cos();
                let mut pts: Vec<(f32, f32)> = Vec::new();
                for i in 0..=48 {
                    let a = (i as f32 / 48.0) * std::f32::consts::PI;
                    let (lx2, ly2) = (rx * a.cos(), ry * a.sin());
                    pts.push((bcx + lx2 * c3 - ly2 * s3, bcy + lx2 * s3 + ly2 * c3));
                }
                p.fill_polygon(&pts, shade_rgba(cap_hex, 0.75));
                p.stroke_polyline(&pts, lw(9.0), ink);
            }
            {
                let (bcx, bcy) = hpt(hatx + 6.0, htop - 26.0);
                p.fill_ellipse(bcx, bcy, 12.0 * RES, 12.0 * RES, 0.0, shade_rgba(cap_hex, 1.15));
                p.stroke_ellipse(bcx, bcy, 12.0 * RES, 12.0 * RES, 0.0, lw(9.0), ink);
            }
        }
        "bowler" => {
            let c_hex = look.hat_col.as_deref().unwrap_or("#7A5236");
            let c = rgba(c_hex);
            // Brim: full ellipse (r*1.02, 22) at top + 26.
            let (bcx, bcy) = hpt(hatx, htop + 26.0);
            p.fill_ellipse(bcx, bcy, rp * 1.02 * (1.0 - 0.3 * sq) * RES, 22.0 * (1.0 + sq) * RES, head_rot, c);
            p.stroke_ellipse(bcx, bcy, rp * 1.02 * (1.0 - 0.3 * sq) * RES, 22.0 * (1.0 + sq) * RES, head_rot, lw(9.0), ink);
            // Dome: upper half ellipse (r*0.66, 80) at top + 14, closed.
            let (dcx, dcy) = hpt(hatx, htop + 14.0);
            let (rx, ry) = (rp * 0.66 * (1.0 - 0.3 * sq) * RES, 80.0 * (1.0 + sq) * RES);
            let (s3, c3) = head_rot.sin_cos();
            let mut pts: Vec<(f32, f32)> = Vec::new();
            for i in 0..=48 {
                let a = std::f32::consts::PI + (i as f32 / 48.0) * std::f32::consts::PI;
                let (lx2, ly2) = (rx * a.cos(), ry * a.sin());
                pts.push((dcx + lx2 * c3 - ly2 * s3, dcy + lx2 * s3 + ly2 * c3));
            }
            p.fill_polygon(&pts, c);
            let mut ring = pts.clone();
            ring.push(pts[0]);
            p.stroke_polyline(&ring, lw(9.0), ink);
            // Band: rect (hx - w, top + 4 - 12, 2w, 24), w = r*0.64,
            // colour = band || shadeOf(c, 0.6), fill + stroke.
            let band_col = look.band.as_deref().map(|b| rgba(b)).unwrap_or_else(|| shade_rgba(c_hex, 0.6));
            let w = rp * 0.64;
            let rect = [
                hpt(hatx - w, htop + 4.0 - 12.0),
                hpt(hatx + w, htop + 4.0 - 12.0),
                hpt(hatx + w, htop + 4.0 + 12.0),
                hpt(hatx - w, htop + 4.0 + 12.0),
            ];
            p.fill_polygon(&rect, band_col);
            let mut ring = rect.to_vec();
            ring.push(rect[0]);
            p.stroke_polyline(&ring, lw(9.0), ink);
        }
        "boater" => {
            // drawHat 'boater': brim ellipse (hx, top+30) rx rp*1.3
            // ry26 fill hatCol; crown roundRect(hx - rp*0.66,
            // top-46, rp*1.32, 76, [16,16,4,4]); band(top+8,
            // rp*0.66, band || '#3C6FB6'); straw hatch shadeOf(c,
            // 0.82) lw3, verticals leaning +6 over the crown,
            // unclipped as authored.
            let c_hex = look.hat_col.as_deref().unwrap_or("#7A5236");
            let c = rgba(c_hex);
            let (bcx, bcy) = hpt(hatx, htop + 30.0);
            let (brx, bry) = (rp * 1.3 * (1.0 - 0.3 * sq) * RES, 26.0 * (1.0 + sq) * RES);
            p.fill_ellipse(bcx, bcy, brx, bry, head_rot, c);
            p.stroke_ellipse(bcx, bcy, brx, bry, head_rot, lw(9.0), ink);
            let crown: Vec<(f32, f32)> =
                rounded_rect_pts_radii(hatx - rp * 0.66, htop - 46.0, rp * 1.32, 76.0, [16.0, 16.0, 4.0, 4.0], 6)
                    .iter()
                    .map(|&(x, y)| hpt(x, y))
                    .collect();
            p.fill_polygon(&crown, c);
            let mut ring = crown.clone();
            ring.push(crown[0]);
            p.stroke_polyline(&ring, lw(9.0), ink);
            let band_col = look.band.as_deref().map(|b| rgba(b)).unwrap_or_else(|| rgba("#3C6FB6"));
            let w = rp * 0.66;
            let rect = [
                hpt(hatx - w, htop + 8.0 - 12.0),
                hpt(hatx + w, htop + 8.0 - 12.0),
                hpt(hatx + w, htop + 8.0 + 12.0),
                hpt(hatx - w, htop + 8.0 + 12.0),
            ];
            p.fill_polygon(&rect, band_col);
            let mut ring = rect.to_vec();
            ring.push(rect[0]);
            p.stroke_polyline(&ring, lw(9.0), ink);
            let hatch = shade_rgba(c_hex, 0.82);
            let mut hoff = -rp * 0.6;
            while hoff < rp * 0.6 {
                p.stroke_polyline(
                    &[hpt(hatx + hoff, htop - 40.0), hpt(hatx + hoff + 6.0, htop - 6.0)],
                    lw(3.0),
                    hatch,
                );
                hoff += 16.0;
            }
        }
        "beanie" => {
            // drawHat 'beanie': dome = upper half-ellipse (hx,
            // top+50) rx rp*0.95 ry96 closed; brim roundRect
            // (hx - rp*0.98, top+30, rp*1.96, 40, 18) fill
            // shadeOf(c, 0.8); pom circle (hx, top-52) r22 fill
            // band || '#F4EFE4'.
            let c_hex = look.hat_col.as_deref().unwrap_or("#7A5236");
            let c = rgba(c_hex);
            {
                let (dcx, dcy) = hpt(hatx, htop + 50.0);
                let (rx, ry) = (rp * 0.95 * (1.0 - 0.3 * sq) * RES, 96.0 * (1.0 + sq) * RES);
                let (s3, c3) = head_rot.sin_cos();
                let mut pts: Vec<(f32, f32)> = Vec::new();
                for i in 0..=48 {
                    let a = std::f32::consts::PI + (i as f32 / 48.0) * std::f32::consts::PI;
                    let (lx2, ly2) = (rx * a.cos(), ry * a.sin());
                    pts.push((dcx + lx2 * c3 - ly2 * s3, dcy + lx2 * s3 + ly2 * c3));
                }
                p.fill_polygon(&pts, c);
                let mut ring = pts.clone();
                ring.push(pts[0]);
                p.stroke_polyline(&ring, lw(9.0), ink);
            }
            let brim: Vec<(f32, f32)> =
                rounded_rect_pts_radii(hatx - rp * 0.98, htop + 30.0, rp * 1.96, 40.0, [18.0; 4], 6)
                    .iter()
                    .map(|&(x, y)| hpt(x, y))
                    .collect();
            p.fill_polygon(&brim, shade_rgba(c_hex, 0.8));
            let mut ring = brim.clone();
            ring.push(brim[0]);
            p.stroke_polyline(&ring, lw(9.0), ink);
            let pom_col = look.band.as_deref().map(|b| rgba(b)).unwrap_or_else(|| rgba("#F4EFE4"));
            let (pcx, pcy) = hpt(hatx, htop - 52.0);
            let (prx, pry) = (22.0 * (1.0 - 0.3 * sq) * RES, 22.0 * (1.0 + sq) * RES);
            p.fill_ellipse(pcx, pcy, prx, pry, head_rot, pom_col);
            p.stroke_ellipse(pcx, pcy, prx, pry, head_rot, lw(9.0), ink);
        }
        "visor" => {
            // drawHat 'visor': translucent green sheet — quad
            // (hx-rp, top+70) via (hx, top+10) to (hx+rp, top+70),
            // line to (hx+rp*1.15, top+98), quad via (hx, top+54)
            // back to (hx-rp*1.15, top+98), close; fill
            // rgba(70,190,120,0.85) + stroke. Then the cap band in
            // hatCol over its top (same construction at
            // ±rp*0.96/0.9, top+64/80, controls top+4/top+24).
            let c = rgba(look.hat_col.as_deref().unwrap_or("#7A5236"));
            let sheet = (70u8, 190u8, 120u8, (0.85 * 255.0) as u8);
            let mut pts = flatten_quad(
                hpt(hatx - rp, htop + 70.0),
                hpt(hatx, htop + 10.0),
                hpt(hatx + rp, htop + 70.0),
                24,
            );
            let q = flatten_quad(
                hpt(hatx + rp * 1.15, htop + 98.0),
                hpt(hatx, htop + 54.0),
                hpt(hatx - rp * 1.15, htop + 98.0),
                24,
            );
            pts.extend_from_slice(&q[1..]);
            p.fill_polygon(&pts, sheet);
            let mut ring = pts.clone();
            ring.push(pts[0]);
            p.stroke_polyline(&ring, lw(9.0), ink);
            let mut pts = flatten_quad(
                hpt(hatx - rp * 0.96, htop + 64.0),
                hpt(hatx, htop + 4.0),
                hpt(hatx + rp * 0.96, htop + 64.0),
                24,
            );
            let q = flatten_quad(
                hpt(hatx + rp * 0.9, htop + 80.0),
                hpt(hatx, htop + 24.0),
                hpt(hatx - rp * 0.9, htop + 80.0),
                24,
            );
            pts.extend_from_slice(&q[1..]);
            p.fill_polygon(&pts, c);
            let mut ring = pts.clone();
            ring.push(pts[0]);
            p.stroke_polyline(&ring, lw(9.0), ink);
        }
        "tophat" => {
            // drawHat 'tophat': brim ellipse (hx, top+24) rx
            // rp*1.0 ry20; crown roundRect(hx - rp*0.6, top-140,
            // rp*1.2, 164, 8); band(top-4, rp*0.6,
            // band || '#B5372E').
            let c_hex = look.hat_col.as_deref().unwrap_or("#7A5236");
            let c = rgba(c_hex);
            let (bcx, bcy) = hpt(hatx, htop + 24.0);
            let (brx, bry) = (rp * (1.0 - 0.3 * sq) * RES, 20.0 * (1.0 + sq) * RES);
            p.fill_ellipse(bcx, bcy, brx, bry, head_rot, c);
            p.stroke_ellipse(bcx, bcy, brx, bry, head_rot, lw(9.0), ink);
            let crown: Vec<(f32, f32)> =
                rounded_rect_pts_radii(hatx - rp * 0.6, htop - 140.0, rp * 1.2, 164.0, [8.0; 4], 6)
                    .iter()
                    .map(|&(x, y)| hpt(x, y))
                    .collect();
            p.fill_polygon(&crown, c);
            let mut ring = crown.clone();
            ring.push(crown[0]);
            p.stroke_polyline(&ring, lw(9.0), ink);
            let band_col = look.band.as_deref().map(|b| rgba(b)).unwrap_or_else(|| rgba("#B5372E"));
            let w = rp * 0.6;
            let rect = [
                hpt(hatx - w, htop - 4.0 - 12.0),
                hpt(hatx + w, htop - 4.0 - 12.0),
                hpt(hatx + w, htop - 4.0 + 12.0),
                hpt(hatx - w, htop - 4.0 + 12.0),
            ];
            p.fill_polygon(&rect, band_col);
            let mut ring = rect.to_vec();
            ring.push(rect[0]);
            p.stroke_polyline(&ring, lw(9.0), ink);
        }
        "bow" => {
            // drawHat 'bow': at (hx + rp*0.5, top+26), colour
            // hatCol || '#D8343F', lw6 — two bezier loops (per
            // side: centre, bezier (±30,-46), (±66,-30) ->
            // (±60,0), bezier (±66,+30), (±30,+40) -> centre),
            // centre circle r15; PLUS side locks: ellipses
            // (hx ± (rp+4), hy+10) rx22 ry50 rot ∓0.2 fill
            // hairCol || '#6B4A3A'.
            let c2 = rgba(look.hat_col.as_deref().unwrap_or("#D8343F"));
            let (bx2, by2) = (hatx + rp * 0.5, htop + 26.0);
            for s in [-1.0f32, 1.0] {
                let mut pts = flatten_cubic(
                    hpt(bx2, by2),
                    hpt(bx2 + s * 30.0, by2 - 46.0),
                    hpt(bx2 + s * 66.0, by2 - 30.0),
                    hpt(bx2 + s * 60.0, by2),
                    20,
                );
                let q = flatten_cubic(
                    hpt(bx2 + s * 60.0, by2),
                    hpt(bx2 + s * 66.0, by2 + 30.0),
                    hpt(bx2 + s * 30.0, by2 + 40.0),
                    hpt(bx2, by2),
                    20,
                );
                pts.extend_from_slice(&q[1..]);
                p.fill_polygon(&pts, c2);
                let mut ring = pts.clone();
                ring.push(pts[0]);
                p.stroke_polyline(&ring, lw(6.0), ink);
            }
            let (ccx, ccy) = hpt(bx2, by2);
            let (crx, cry) = (15.0 * (1.0 - 0.3 * sq) * RES, 15.0 * (1.0 + sq) * RES);
            p.fill_ellipse(ccx, ccy, crx, cry, head_rot, c2);
            p.stroke_ellipse(ccx, ccy, crx, cry, head_rot, lw(6.0), ink);
            let lock_col = rgba(look.hair_col.as_deref().unwrap_or("#6B4A3A"));
            for s in [-1.0f32, 1.0] {
                let (lx2, ly2) = hpt(hatx + s * (rp + 4.0), hy + 10.0);
                let (lrx, lry) = (22.0 * (1.0 - 0.3 * sq) * RES, 50.0 * (1.0 + sq) * RES);
                let lrot = head_rot - sgn * s * 0.2;
                p.fill_ellipse(lx2, ly2, lrx, lry, lrot, lock_col);
                p.stroke_ellipse(lx2, ly2, lrx, lry, lrot, lw(6.0), ink);
            }
        }
        "mailcap" => {
            // drawHat 'mailcap': crown roundRect(hx - rp*0.72,
            // top-40, rp*1.44, 70, [20,20,4,4]) fill hatCol; brim
            // = INK half-ellipse (hx-10, top+34) rx rp*0.82 ry18
            // rot 0.04, arc 0..PI, filled INK (no stroke, as
            // authored); badge circle (hx, top-6) r13 '#E2B33C'
            // fill + lw4 stroke.
            let c = rgba(look.hat_col.as_deref().unwrap_or("#7A5236"));
            let crown: Vec<(f32, f32)> =
                rounded_rect_pts_radii(hatx - rp * 0.72, htop - 40.0, rp * 1.44, 70.0, [20.0, 20.0, 4.0, 4.0], 6)
                    .iter()
                    .map(|&(x, y)| hpt(x, y))
                    .collect();
            p.fill_polygon(&crown, c);
            let mut ring = crown.clone();
            ring.push(crown[0]);
            p.stroke_polyline(&ring, lw(9.0), ink);
            {
                let (bcx, bcy) = hpt(hatx - 10.0, htop + 34.0);
                let (rx, ry, rot) = (rp * 0.82 * (1.0 - 0.3 * sq) * RES, 18.0 * (1.0 + sq) * RES, 0.04 * sgn + head_rot);
                let (s3, c3) = rot.sin_cos();
                let mut pts: Vec<(f32, f32)> = Vec::new();
                for i in 0..=48 {
                    let a = (i as f32 / 48.0) * std::f32::consts::PI;
                    let (lx2, ly2) = (rx * a.cos(), ry * a.sin());
                    pts.push((bcx + lx2 * c3 - ly2 * s3, bcy + lx2 * s3 + ly2 * c3));
                }
                p.fill_polygon(&pts, ink);
            }
            let (bcx2, bcy2) = hpt(hatx, htop - 6.0);
            let (brx, bry) = (13.0 * (1.0 - 0.3 * sq) * RES, 13.0 * (1.0 + sq) * RES);
            p.fill_ellipse(bcx2, bcy2, brx, bry, head_rot, rgba("#E2B33C"));
            p.stroke_ellipse(bcx2, bcy2, brx, bry, head_rot, lw(4.0), ink);
        }
        "flower" => {
            // drawHat 'flower': at (hx + rp*0.62, top+40): 6
            // petals r14 at radius 18, fill hatCol || '#F29BB5',
            // INK lw4; centre r11 '#F6C945'.
            let petal = rgba(look.hat_col.as_deref().unwrap_or("#F29BB5"));
            let (fx2, fy2) = (hatx + rp * 0.62, htop + 40.0);
            for i in 0..6 {
                let a = (i as f32 / 6.0) * std::f32::consts::TAU;
                let (pcx, pcy) = hpt(fx2 + a.cos() * 18.0, fy2 + a.sin() * 18.0);
                let (prx, pry) = (14.0 * (1.0 - 0.3 * sq) * RES, 14.0 * (1.0 + sq) * RES);
                p.fill_ellipse(pcx, pcy, prx, pry, head_rot, petal);
                p.stroke_ellipse(pcx, pcy, prx, pry, head_rot, lw(4.0), ink);
            }
            let (ccx, ccy) = hpt(fx2, fy2);
            let (crx, cry) = (11.0 * (1.0 - 0.3 * sq) * RES, 11.0 * (1.0 + sq) * RES);
            p.fill_ellipse(ccx, ccy, crx, cry, head_rot, rgba("#F6C945"));
            p.stroke_ellipse(ccx, ccy, crx, cry, head_rot, lw(4.0), ink);
        }
        _ => {}
    }

    (p.px, cw as u32, ch as u32, hands.0, hands.1)
}

#[cfg(test)]
mod canvas_anchor_tests {
    use super::*;

    fn finn_look() -> FullLook {
        // ex07's Finn (the B-1 case): hat=none, plain pattern.
        FullLook {
            body: "#DE7A35".into(),
            head: "#F7F7FA".into(),
            shade: None,
            pattern: "plain".into(),
            pat_col: None,
            hat: "none".into(),
            hat_col: None,
            band: None,
            neck: "none".into(),
            neck_col: None,
            glasses: false,
            mustache: false,
            mustache_col: None,
            kid: false,
            blush: false,
            freckles: false,
            pencil: false,
            monocle: false,
            hair_col: None,
            wing: false,
            wing_col: None,
            wing_lens: Vec::new(),
            ears: false,
            hair: false,
        }
    }

    #[test]
    fn rest_and_walk_states_need_no_anchor() {
        let l = finn_look();
        assert_eq!(canvas_anchor(&l, &FullState::default()), (0.0, 0.0));
        let mut st = FullState::default();
        st.legs = Legs::Walk;
        st.phase = 1.1;
        assert_eq!(canvas_anchor(&l, &st), (0.0, 0.0));
    }

    #[test]
    fn lying_lean_anchors_back_into_canvas() {
        // The B-1 case family (ex07's held lie): a high lean
        // rotates the head past the right canvas edge; the
        // anchor must pull the figure back left, with no
        // vertical shift for a pure side overflow. The mirror
        // lean pulls the other way.
        let l = finn_look();
        let mut st = FullState::default();
        st.mood = Mood::Dizzy;
        st.arm_l = (150.0, 20.0);
        st.arm_r = (138.0, -30.0);
        st.head_tilt = 4.0;
        st.lean = 82.0;
        let (ax, ay) = canvas_anchor(&l, &st);
        assert!(ax < -50.0, "expected a strong left pull, got ax={ax}");
        // The raised right hand (138 deg) also rotates past the
        // bottom inset, so the anchor pulls up slightly too —
        // the envelope is honest in both axes, as in TS.
        assert!(ay < 0.0, "expected a slight upward pull, got ay={ay}");
        st.lean = -82.0;
        let (ax2, _) = canvas_anchor(&l, &st);
        assert!(ax2 > 50.0, "expected a strong right pull, got ax={ax2}");
    }

    #[test]
    fn high_seat_anchors_down() {
        // The ex06 family: a high seat (large sitDrop through
        // the painter's own sit formula) lifts the head past
        // the canvas top; the anchor pushes the figure down.
        let l = finn_look();
        let mut st = FullState::default();
        st.legs = Legs::Sit;
        st.sit_drop = Some(900.0);
        let (_, ay) = canvas_anchor(&l, &st);
        assert!(ay > 100.0, "expected a strong downward pull, got ay={ay}");
    }
}

#[cfg(test)]
mod carry_plumbing_tests {
    use super::*;

    fn fnv(px: &[u8]) -> u64 {
        let mut h: u64 = 0xcbf29ce484222325;
        for &b in px {
            h ^= b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        h
    }

    fn look(body: &str) -> FullLook {
        FullLook {
            body: body.into(),
            head: "#F7F7FA".into(),
            shade: None,
            pattern: "plain".into(),
            pat_col: None,
            hat: "none".into(),
            hat_col: None,
            band: None,
            neck: "none".into(),
            neck_col: None,
            glasses: false,
            mustache: false,
            mustache_col: None,
            kid: false,
            blush: false,
            freckles: false,
            pencil: false,
            monocle: false,
            hair_col: None,
            wing: false,
            wing_col: None,
            wing_lens: Vec::new(),
            ears: false,
            hair: false,
        }
    }

    /// Non-regression gate for the CARRY plumbing: the five
    /// full-frame FNV-1a hashes below were pinned at
    /// gpufirst-phase0 f4b358d BEFORE the wing/ears/hair/flip/
    /// bodyPitch fields existed (probe log:
    /// ~/workspace/talerender-evidence/gpufirst-b2/carry/baseline-probe.log).
    /// The states here carry the new fields at their defaults
    /// (FullState::default + the look() defaults above), so
    /// reproducing the hashes byte-identically proves the
    /// plumbing changed no paint.
    #[test]
    fn default_carry_fields_reproduce_pre_plumbing_hashes() {
        let mut cases: Vec<(&str, FullLook, FullState, u64)> = Vec::new();
        cases.push(("A-default", look("#DE7A35"), FullState::default(), 0xcfb4d768fb09fdbf));
        {
            let mut s = FullState::default();
            s.mood = Mood::Happy;
            s.legs = Legs::Walk;
            s.phase = 1.1;
            cases.push(("B-happy-walk", look("#DE7A35"), s, 0x05a781daa65cb22b));
        }
        {
            let mut l = look("#5C6644");
            l.hat = "cap".into();
            l.hat_col = Some("#4C5638".into());
            l.pattern = "vest".into();
            l.pat_col = Some("#33304A".into());
            l.neck = "scarf".into();
            l.neck_col = Some("#8A7A52".into());
            let mut s = FullState::default();
            s.mood = Mood::Determined;
            s.arm_l = (28.0, -24.0);
            s.arm_r = (68.0, -42.0);
            s.look = [0.3, 0.0];
            s.lean = 1.0;
            cases.push(("C-hero-vest", l, s, 0x1ed010bf0e163e77));
        }
        {
            let mut l = look("#E8734A");
            l.kid = true;
            let mut s = FullState::default();
            s.mood = Mood::Sad;
            s.legs = Legs::Sit;
            s.sit_drop = Some(300.0);
            s.tears = true;
            cases.push(("D-kid-sit-sad", l, s, 0xcec92a8d3ebab654));
        }
        {
            let mut l = look("#4A4560");
            l.hat = "bowler".into();
            l.hat_col = Some("#2E2A3A".into());
            l.glasses = true;
            l.mustache = true;
            l.mustache_col = Some("#C9C9D2".into());
            l.blush = true;
            l.freckles = true;
            let mut s = FullState::default();
            s.mood = Mood::Worried;
            s.sweat = Some(1.0); // Batch 2 integration: bool true at pin time == Some(1.0) under Spec 5e
            s.talk = 0.7;
            s.blink = 0.3;
            s.head_tilt = 6.0;
            s.head_turn = 0.3;
            s.squash = 0.1;
            s.cam_pitch = 0.28747;
            cases.push(("E-tani-worried", l, s, 0x2cb164e9c2e44aa6));
        }
        for (name, lk, st, want) in &cases {
            let (px, w, h, _, _) = paint_toon_full(lk, st);
            assert_eq!((w, h), (2560, 2816), "{name}: canvas size changed");
            assert_eq!(fnv(&px), *want, "{name}: default-carry paint drifted from the pre-plumbing hash");
        }
    }

    /// The resolved defaults are the TS defaults (RUST-PARITY-SPECS
    /// Spec 1 / §5b / §5c / §5d / Spec 3a): wing/ears/hair/flip off,
    /// angles 0, arrays empty, per-ear overrides absent — and
    /// body_pitch_counter defaults to 1.0 (TS `?? 1`), NOT 0.0.
    #[test]
    fn carry_defaults_match_ts_resolved_defaults() {
        let st = FullState::default();
        assert!(st.wing_segs.is_empty());
        assert_eq!(st.wing_a.to_bits(), 0.0f32.to_bits());
        assert_eq!(st.ear_a.to_bits(), 0.0f32.to_bits());
        assert_eq!(st.ear_al, None);
        assert_eq!(st.ear_ar, None);
        assert!(st.hair_segs.is_empty());
        assert!(!st.flip);
        assert_eq!(st.body_pitch.to_bits(), 0.0f32.to_bits());
        assert_eq!(st.body_pitch_counter.to_bits(), 1.0f32.to_bits());
        let lk = look("#000000");
        assert!(!lk.wing);
        assert_eq!(lk.wing_col, None);
        assert!(lk.wing_lens.is_empty());
        assert!(!lk.ears);
        assert!(!lk.hair);
    }
}


#[cfg(test)]mod brow_tests {
    use super::*;
    use std::collections::HashSet;

    /// Full-frame FNV-1a 64 hash of a paint (the test's hash
    /// instrument; no new dependency).
    fn fnv1a(px: &[u8]) -> u64 {
        let mut h: u64 = 0xcbf29ce484222325;
        for &b in px {
            h ^= b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        h
    }

    fn brow_look() -> FullLook {
        // The scene-format default look (same values as
        // gpupaint_states::base_look).
        FullLook {
            body: "#E8734A".into(),
            head: "#F7F7FA".into(),
            shade: None,
            pattern: "plain".into(),
            pat_col: None,
            hat: "none".into(),
            hat_col: None,
            band: None,
            neck: "none".into(),
            neck_col: None,
            glasses: false,
            mustache: false,
            mustache_col: None,
            kid: false,
            blush: false,
            freckles: false,
            pencil: false,
            monocle: false,
            hair_col: None,
            wing: false,
            wing_col: None,
            wing_lens: Vec::new(),
            ears: false,
            hair: false,
        }
    }

    fn paint_hash(look: &FullLook, st: &FullState) -> u64 {
        let (px, w, h, _, _) = paint_toon_full(look, st);
        assert_eq!((w, h), (2560, 2816));
        fnv1a(&px)
    }

    /// Non-regression: hashes pinned at HEAD f4b358d BEFORE the
    /// Spec 5a painter change (captured with this same helper /
    /// look / default states on the unmodified painter). Brow-
    /// absent (Default) and explicit brow 0/0 states must
    /// reproduce them byte-identically (hash-identically).
    #[test]
    fn brow_absent_paints_match_pre_port_pinned_hashes() {
        let look = brow_look();
        // (mood, pinned FNV-1a at f4b358d, pre-port painter)
        let pinned: [(Mood, u64); 8] = [
            (Mood::Neutral, 0xd413603252323a7c),
            (Mood::Smug, 0xc77ad1c3ba4db004),
            (Mood::Suspicious, 0x503455d1f3519971),
            (Mood::Puzzled, 0x59c8d9bc324d098b),
            (Mood::Worried, 0xf97d0c5ae8a31394),
            (Mood::Determined, 0x5596a27e0713ff76),
            (Mood::Happy, 0x1e3f39de0803d646),
            (Mood::Sad, 0xe3ce626585f25b3d),
        ];
        for (mood, want) in pinned {
            let mut st = FullState::default();
            st.mood = mood;
            assert_eq!(st.brow_lift, 0.0);
            assert_eq!(st.brow_tilt, 0.0);
            let got = paint_hash(&look, &st);
            assert_eq!(got, want, "{mood:?}: brow-absent paint must be byte-identical to pre-port");
        }
        // Explicit 0/0 is the same state as absent.
        let mut st = FullState::default();
        st.mood = Mood::Neutral;
        st.brow_lift = 0.0;
        st.brow_tilt = 0.0;
        assert_eq!(paint_hash(&look, &st), 0xd413603252323a7c);
    }

    /// Spec 5a acceptance (S3 bar): 19 moods x browLift
    /// {0, -8, 0.5, 14} x browTilt {0, -10, 0.5, 10} = 304 paints
    /// must yield 304 DISTINCT full-frame hashes (pre-port the
    /// painter yields 19). Every 0.5-step pair is distinct by
    /// construction of the count, and is also asserted directly
    /// below for the neutral mood.
    #[test]
    fn brow_offsets_yield_304_distinct_classes() {
        let look = brow_look();
        let moods = [
            Mood::Neutral,
            Mood::Smug,
            Mood::Happy,
            Mood::Laugh,
            Mood::Shock,
            Mood::Worried,
            Mood::Angry,
            Mood::Sad,
            Mood::Sly,
            Mood::Proud,
            Mood::Tired,
            Mood::Dizzy,
            Mood::Determined,
            Mood::Embarrassed,
            Mood::Awe,
            Mood::Suspicious,
            Mood::Relieved,
            Mood::Puzzled,
            Mood::Deadpan,
        ];
        let lifts = [0.0f32, -8.0, 0.5, 14.0];
        let tilts = [0.0f32, -10.0, 0.5, 10.0];
        let mut seen: HashSet<u64> = HashSet::new();
        for mood in moods {
            for lift in lifts {
                for tilt in tilts {
                    let mut st = FullState::default();
                    st.mood = mood;
                    st.brow_lift = lift;
                    st.brow_tilt = tilt;
                    let h = paint_hash(&look, &st);
                    assert!(
                        seen.insert(h),
                        "duplicate paint class: mood={mood:?} browLift={lift} browTilt={tilt}"
                    );
                }
            }
        }
        assert_eq!(seen.len(), 304);
    }
}


#[cfg(test)]
mod sweat_tests {
    use super::*;
    use std::collections::HashSet;

    /// The gpupaint_states base look (body #E8734A, head #F7F7FA,
    /// plain / none / none, no extras) — the look the pre-port
    /// pins below were captured with.
    fn look() -> FullLook {
        FullLook {
            body: "#E8734A".into(),
            head: "#F7F7FA".into(),
            shade: None,
            pattern: "plain".into(),
            pat_col: None,
            hat: "none".into(),
            hat_col: None,
            band: None,
            neck: "none".into(),
            neck_col: None,
            glasses: false,
            mustache: false,
            mustache_col: None,
            kid: false,
            blush: false,
            freckles: false,
            pencil: false,
            monocle: false,
            hair_col: None,
            wing: false,
            wing_col: None,
            wing_lens: Vec::new(),
            ears: false,
            hair: false,
        }
    }

    /// FNV-1a 64 over the full RGBA canvas (no hashing dependency
    /// in this crate; the pin values below are in-crate only).
    fn fnv(px: &[u8]) -> u64 {
        let mut h: u64 = 0xcbf29ce484222325;
        for &b in px {
            h ^= b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        h
    }

    fn paint_hash(mood: Mood, sweat: Option<f32>) -> u64 {
        let mut st = FullState::default();
        st.mood = mood;
        st.sweat = sweat;
        fnv(&paint_toon_full(&look(), &st).0)
    }

    /// Non-regression pins: these FNV hashes were captured BEFORE
    /// the Spec 5e port, at gpufirst-phase0 f4b358d, with the bool
    /// painter (neutral+sweat=true, worried+sweat=false,
    /// neutral+sweat=false). The mapped equivalents after the
    /// port — Some(1.0), None (worried forcing, k = 1), None —
    /// must reproduce them byte-identically (alpha min(1, 1) = 1
    /// is today's unconditional alpha 1; None paints no drop).
    #[test]
    fn sweat_pinned_pre_port_hashes() {
        assert_eq!(paint_hash(Mood::Neutral, Some(1.0)), 0xb90b2423dd32af00, "neutral + sweat Some(1.0) must equal pre-port sweat=true");
        assert_eq!(paint_hash(Mood::Worried, None), 0xf97d0c5ae8a31394, "worried + no sweat must equal pre-port worried (forced, alpha 1)");
        assert_eq!(paint_hash(Mood::Neutral, None), 0xd413603252323a7c, "neutral + no sweat must equal pre-port sweat=false");
    }

    /// Spec 5e acceptance, mirroring S11-sweat: sweat {absent, 0,
    /// 0.5, 0.99, 1, 1.5} x moods {neutral, worried, sad, happy}
    /// = 24 paints -> EXACTLY 16 distinct pixel classes.
    /// Partition (per mood, 4 classes):
    ///   non-forcing (neutral/sad/happy): {absent, 0}, {0.5},
    ///     {0.99}, {1, 1.5}  (saturation EQ at 1, gradation
    ///     distinctness below it);
    ///   worried (forcing): {absent, 1, 1.5}, {0}, {0.5}, {0.99}.
    /// Worried + 0 lands pixel-wise as its OWN class: the drop is
    /// emitted at alpha 0 (invisible), so its pixels are the
    /// worried face with NO visible drop — distinct from
    /// worried-absent (visible full-alpha drop) and from every
    /// other worried cell. That is the pixel-side landing of the
    /// spec's unadjudicable pair (distinct in the TS op stream,
    /// invisible in pixels in both painters).
    #[test]
    fn sweat_s11_sixteen_pixel_classes() {
        let moods = [Mood::Neutral, Mood::Worried, Mood::Sad, Mood::Happy];
        let vals: [Option<f32>; 6] = [None, Some(0.0), Some(0.5), Some(0.99), Some(1.0), Some(1.5)];
        // h[mood][val]
        let h: Vec<Vec<u64>> = moods.iter().map(|&m| vals.iter().map(|&v| paint_hash(m, v)).collect()).collect();
        let distinct: HashSet<u64> = h.iter().flatten().copied().collect();
        assert_eq!(distinct.len(), 16, "S11-sweat must refine to exactly 16 pixel classes, got {}", distinct.len());
        for (mi, &m) in moods.iter().enumerate() {
            let (absent, zero, half, nn, one, over) = (h[mi][0], h[mi][1], h[mi][2], h[mi][3], h[mi][4], h[mi][5]);
            // Representative classes for this mood, per the partition above.
            let reps: [u64; 4] = if m == Mood::Worried {
                assert_eq!(absent, one, "worried: absent == 1 (k = sweat.unwrap_or(1))");
                assert_eq!(one, over, "worried: 1 == 1.5 (saturation)");
                [absent, zero, half, nn]
            } else {
                assert_eq!(absent, zero, "{m:?}: absent == 0 (no drop)");
                assert_eq!(one, over, "{m:?}: 1 == 1.5 (saturation)");
                [absent, half, nn, one]
            };
            for i in 0..4 {
                for j in (i + 1)..4 {
                    assert_ne!(reps[i], reps[j], "{m:?}: representative classes {i} and {j} must be pixel-distinct");
                }
            }
        }
    }
}

#[cfg(test)]
mod flippitch_tests {
    use super::*;
    use std::collections::HashSet;

    fn fnv(px: &[u8]) -> u64 {
        let mut h: u64 = 0xcbf29ce484222325;
        for &b in px {
            h ^= b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        h
    }

    fn look() -> FullLook {
        // The gpupaint_states base look (the look the pre-port
        // pins in brow_tests / sweat_tests were captured with).
        FullLook {
            body: "#E8734A".into(),
            head: "#F7F7FA".into(),
            shade: None,
            pattern: "plain".into(),
            pat_col: None,
            hat: "none".into(),
            hat_col: None,
            band: None,
            neck: "none".into(),
            neck_col: None,
            glasses: false,
            mustache: false,
            mustache_col: None,
            kid: false,
            blush: false,
            freckles: false,
            pencil: false,
            monocle: false,
            hair_col: None,
            wing: false,
            wing_col: None,
            wing_lens: Vec::new(),
            ears: false,
            hair: false,
        }
    }

    fn paint_hash(st: &FullState) -> u64 {
        let (px, w, h, _, _) = paint_toon_full(&look(), st);
        assert_eq!((w, h), (2560, 2816));
        fnv(&px)
    }

    /// Defaults byte-identity (flip §5d acceptance "flip: false
    /// renders byte-identical to today" + Spec 3 acceptance
    /// "byte-identity at 0"): the neutral default paint must
    /// reproduce the PRE-PORT pinned hash (brow_tests pin at
    /// f4b358d, before flip/pitch existed in the painter) under
    /// flip=false explicitly, and under every pitch-0 variant:
    /// absent (Default), 0, 0 + counter 0, 0 + counter 0.37.
    #[test]
    fn defaults_reproduce_pre_port_hash() {
        const PRE_PORT_NEUTRAL: u64 = 0xd413603252323a7c;
        assert_eq!(paint_hash(&FullState::default()), PRE_PORT_NEUTRAL);
        let mut s = FullState::default();
        s.flip = false;
        assert_eq!(paint_hash(&s), PRE_PORT_NEUTRAL, "explicit flip:false drifted");
        for counter in [1.0f32, 0.0, 0.37] {
            let mut s = FullState::default();
            s.body_pitch = 0.0;
            s.body_pitch_counter = counter;
            assert_eq!(
                paint_hash(&s),
                PRE_PORT_NEUTRAL,
                "pitch 0 with counter {counter} must paint identically (Spec 3)"
            );
        }
    }

    /// Flip acceptance (Spec §5d, S12-placement): the fixture
    /// cells {neutral, happy} x flip x bob {0, 14} x lean {0, 9}
    /// = 16 cells must yield 16 DISTINCT paint classes (TS 16;
    /// pre-port Rust merged the 8 flip pairs -> 8).
    #[test]
    fn flip_s12_sixteen_classes() {
        let mut seen: HashSet<u64> = HashSet::new();
        for mood in [Mood::Neutral, Mood::Happy] {
            for flip in [false, true] {
                for bob in [0.0f32, 14.0] {
                    for lean in [0.0f32, 9.0] {
                        let mut s = FullState::default();
                        s.mood = mood;
                        s.flip = flip;
                        s.bob = bob;
                        s.lean = lean;
                        let h = paint_hash(&s);
                        assert!(
                            seen.insert(h),
                            "S12 cell merged: mood={mood:?} flip={flip} bob={bob} lean={lean}"
                        );
                    }
                }
            }
        }
        assert_eq!(seen.len(), 16, "S12 must refine to 16 classes (TS 16, pre-port 8)");
    }

    /// Flip is a whole-figure mirror, measured: the flipped paint
    /// of an asymmetric state (Suspicious slit eye + look offset
    /// + arm asymmetry) against the device-mirror of the
    /// unflipped paint. Exact bit-mirror is not demanded (cubic /
    /// quad flattening rounds differently in mirrored order);
    /// the measured diff is printed and bounded well under 0.1%
    /// of the canvas — a missed mirror on any part (a brow, an
    /// ellipse rotation, the plate) would blow past it by orders
    /// of magnitude.
    #[test]
    fn flip_paint_mirrors_the_unflipped_paint() {
        let mut s = FullState::default();
        s.mood = Mood::Suspicious;
        s.look = [0.4, -0.3];
        s.arm_l = (28.0, -24.0);
        s.arm_r = (68.0, -42.0);
        let (a, w, h, _, _) = paint_toon_full(&look(), &s);
        s.flip = true;
        let (b, _, _, _, _) = paint_toon_full(&look(), &s);
        let w = w as usize;
        let mut diff_px = 0u64;
        let mut max_d = 0u8;
        for y in 0..h as usize {
            for x in 0..w {
                let ka = (y * w + x) * 4;
                let kb = (y * w + (w - 1 - x)) * 4;
                let mut differs = false;
                for ch in 0..4 {
                    let d = (a[ka + ch] as i32 - b[kb + ch] as i32).abs();
                    if d > 0 {
                        differs = true;
                        max_d = max_d.max(d as u8);
                    }
                }
                if differs {
                    diff_px += 1;
                }
            }
        }
        println!("FLIP-MIRROR: diff_px={diff_px} max_delta={max_d} of {} px", w * h as usize);
        assert!(
            diff_px < 7_200,
            "flipped paint is not the mirror of the unflipped paint: {diff_px} differing px (max delta {max_d})"
        );
    }

    /// Flip + anchor: the anchor is solved in mirrored space, so
    /// flipping an anchor-engaging state negates ax and keeps ay
    /// (the canvas bounds are symmetric about the stage centre
    /// x=256: [-380, 892]).
    #[test]
    fn flip_anchor_mirrors_ax() {
        let l = look();
        let mut s = FullState::default();
        s.mood = Mood::Dizzy;
        s.arm_l = (150.0, 20.0);
        s.arm_r = (138.0, -30.0);
        s.head_tilt = 4.0;
        s.lean = 82.0;
        let (ax, ay) = canvas_anchor(&l, &s);
        assert!(ax < -50.0, "unflipped lying lean must engage, got {ax}");
        s.flip = true;
        let (fx, fy) = canvas_anchor(&l, &s);
        assert!((fx + ax).abs() < 1e-3, "flipped ax {fx} must negate unflipped ax {ax}");
        assert!((fy - ay).abs() < 1e-3, "flipped ay {fy} must equal unflipped ay {ay}");
    }

    /// Spec 3 counter-tilt, exact: state headTilt 10, bodyPitch
    /// 60 -> effective head rotation -50 at counter 1, -20 at
    /// 0.5, +10 at 0 (the spec's TS numbers, verbatim).
    #[test]
    fn pitch_counter_tilt_exact() {
        for (counter, want) in [(1.0f32, -50.0f32), (0.5, -20.0), (0.0, 10.0)] {
            let mut s = FullState::default();
            s.head_tilt = 10.0;
            s.body_pitch = 60.0;
            s.body_pitch_counter = counter;
            assert_eq!(effective_head_tilt(&s), want, "counter {counter}");
        }
    }

    /// Spec 3 hands: the returned hand positions are the pitched
    /// stage positions — pitch_pt of the un-pitched hands, to
    /// float precision — and the hip pivot itself is
    /// pitch-invariant.
    #[test]
    fn pitch_hands_are_pitched_positions() {
        let l = look();
        let mut s = FullState::default();
        s.arm_l = (28.0, -24.0);
        s.arm_r = (68.0, -42.0);
        let (_, _, _, hl0, hr0) = paint_toon_full(&l, &s);
        s.body_pitch = 60.0;
        let (_, _, _, hl1, hr1) = paint_toon_full(&l, &s);
        // Rest geometry: by = 0, hip = (256, 600 - 26).
        for (before, after) in [(hl0, hl1), (hr0, hr1)] {
            let want = pitch_pt(before.0, before.1, 256.0, 574.0, 60.0);
            assert!(
                (after.0 - want.0).abs() < 1e-3 && (after.1 - want.1).abs() < 1e-3,
                "hand {before:?} pitched to {after:?}, want {want:?}"
            );
            assert_ne!(after, before, "pitch 60 must move the hands");
        }
        assert_eq!(pitch_pt(256.0, 574.0, 256.0, 574.0, 60.0), (256.0, 574.0));
    }

    /// Spec 3 legs: at pitch 60 the leg strokes are identical to
    /// pitch 0. Pixel-level form: the bottom strip of the canvas
    /// (device rows >= 2400, stage y >= ~919 — feet + lower legs
    /// only; no upper-assembly point of this state reaches it
    /// under a 60-degree hip rotation) must be byte-identical
    /// between the pitch-0 and pitch-60 paints, while the full
    /// frames differ.
    #[test]
    fn pitch_60_legs_paint_identically() {
        let l = look();
        let s0 = FullState::default();
        let (a, w, h, _, _) = paint_toon_full(&l, &s0);
        let mut s1 = FullState::default();
        s1.body_pitch = 60.0;
        let (b, _, _, _, _) = paint_toon_full(&l, &s1);
        assert_ne!(a, b, "pitch 60 must change the full paint");
        let row_bytes = w as usize * 4;
        let strip_a = &a[2400 * row_bytes..];
        let strip_b = &b[2400 * row_bytes..];
        assert_eq!(strip_a.len(), (h as usize - 2400) * row_bytes);
        assert_eq!(strip_a, strip_b, "leg strip (rows >= 2400) differs under pitch 60");
    }

    /// Spec 3 envelope: the crawl working range (55–70°) never
    /// engages the anchor at adult or kid scale, and a full
    /// -90..180 sweep stays finite (anchor is a pure function —
    /// no panic, no NaN).
    #[test]
    fn pitch_anchor_crawl_range_never_engages() {
        for kid in [false, true] {
            let mut l = look();
            l.kid = kid;
            for pitch in [55.0f32, 60.0, 65.0, 70.0] {
                let mut s = FullState::default();
                s.body_pitch = pitch;
                assert_eq!(
                    canvas_anchor(&l, &s),
                    (0.0, 0.0),
                    "crawl pitch {pitch} (kid={kid}) must not engage the anchor"
                );
            }
            let mut p = -90.0f32;
            while p <= 180.0 {
                let mut s = FullState::default();
                s.body_pitch = p;
                let a = canvas_anchor(&l, &s);
                assert!(a.0.is_finite() && a.1.is_finite(), "pitch {p}: anchor {a:?}");
                p += 15.0;
            }
        }
    }

    // ---- B3-CARRY-1: plate sampling under flip / pitch ----
    //
    // A synthetic 40x40 plate (20x20 stage units), left half red,
    // right half blue, fully opaque, centred on the hand at
    // (256, 600) with hip (256, 574). Sampled at interior points
    // 6 stage units either side of the plate centre — far from
    // the half boundary and the plate edge, so bilinear sampling
    // returns the exact half colour.

    fn half_plate() -> Plate {
        let (w, h) = (40u32, 40u32);
        let mut px = vec![0u8; (w * h * 4) as usize];
        for y in 0..h {
            for x in 0..w {
                let k = ((y * w + x) * 4) as usize;
                let (r, b) = if x < 20 { (255u8, 0u8) } else { (0u8, 255u8) };
                px[k] = r;
                px[k + 2] = b;
                px[k + 3] = 255;
            }
        }
        Plate { px, w, h, x0: -10.0, y0: -10.0 }
    }

    fn sample(p: &Painter, stage_x: f32, stage_y: f32) -> (u8, u8, u8, u8) {
        // Stage -> canvas device pixel (lean 0, anchor 0):
        // ((x + 384) * RES, (y + 256) * RES), pixel-centre safe
        // for the integer stage points used here.
        let cx = ((stage_x + 384.0) * RES) as usize;
        let cy = ((stage_y + 256.0) * RES) as usize;
        let k = (cy * p.w + cx) * 4;
        (p.px[k], p.px[k + 1], p.px[k + 2], p.px[k + 3])
    }

    const RED: (u8, u8, u8, u8) = (255, 0, 0, 255);
    const BLUE: (u8, u8, u8, u8) = (0, 0, 255, 255);

    #[test]
    fn plate_sampling_mirrors_under_flip() {
        let plate = half_plate();
        let hand = (256.0, 600.0);
        let hip = (256.0, 574.0);
        // Unflipped: stage (250, 600) samples the plate's left
        // (red) half; (262, 600) the right (blue) half.
        let mut p = Painter::new(2560, 2816);
        composite_plate(&mut p, &plate, hand, 0.0, (0.0, 0.0), false, 0.0, hip);
        assert_eq!(sample(&p, 250.0, 600.0), RED, "unflipped left");
        assert_eq!(sample(&p, 262.0, 600.0), BLUE, "unflipped right");
        // Flipped: the SAME stage points must sample the OPPOSITE
        // halves — the plate mirrors with the figure. Pre-B3-CARRY-1
        // (lean+anchor-only inverse map) this returned RED/BLUE
        // again: the plate was sampled un-mirrored.
        let mut p = Painter::new(2560, 2816);
        composite_plate(&mut p, &plate, hand, 0.0, (0.0, 0.0), true, 0.0, hip);
        assert_eq!(sample(&p, 250.0, 600.0), BLUE, "flipped left must sample the blue half");
        assert_eq!(sample(&p, 262.0, 600.0), RED, "flipped right must sample the red half");
    }

    #[test]
    fn plate_sampling_rotates_under_pitch() {
        let plate = half_plate();
        let hand = (256.0, 600.0);
        let hip = (256.0, 574.0);
        // Pitch +90 about the hip: the plate-local point
        // (250, 600) (red half) rotates to stage (230, 568);
        // (262, 600) (blue half) rotates to (230, 580).
        // (rotPt: rel hip (-6, 26) -> (-26, -6); (6, 26) -> (-26, 6).)
        let mut p = Painter::new(2560, 2816);
        composite_plate(
            &mut p,
            &plate,
            hand,
            0.0,
            (0.0, 0.0),
            false,
            std::f32::consts::FRAC_PI_2,
            hip,
        );
        assert_eq!(sample(&p, 230.0, 568.0), RED, "pitched red half must land at (230, 568)");
        assert_eq!(sample(&p, 230.0, 580.0), BLUE, "pitched blue half must land at (230, 580)");
        // And the un-pitched location no longer carries the plate's
        // red half (the plate has rotated away from it).
        assert_ne!(sample(&p, 250.0, 600.0), RED, "plate must not sample un-pitched under pitch 90");
    }
}
#[cfg(test)]
mod earshair_tests {
    use super::*;
    use std::collections::HashSet;

    /// Full-frame FNV-1a 64 hash of a paint (the house test
    /// instrument; no new dependency).
    fn fnv1a(px: &[u8]) -> u64 {
        let mut h: u64 = 0xcbf29ce484222325;
        for &b in px {
            h ^= b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        h
    }

    fn look(body: &str) -> FullLook {
        FullLook {
            body: body.into(),
            head: "#F7F7FA".into(),
            shade: None,
            pattern: "plain".into(),
            pat_col: None,
            hat: "none".into(),
            hat_col: None,
            band: None,
            neck: "none".into(),
            neck_col: None,
            glasses: false,
            mustache: false,
            mustache_col: None,
            kid: false,
            blush: false,
            freckles: false,
            pencil: false,
            monocle: false,
            hair_col: None,
            wing: false,
            wing_col: None,
            wing_lens: Vec::new(),
            ears: false,
            hair: false,
        }
    }

    fn paint(look: &FullLook, st: &FullState) -> (u64, usize) {
        let (px, w, h, _, _) = paint_toon_full(look, st);
        assert_eq!((w, h), (2560, 2816));
        // Non-background = any pixel with non-zero alpha (the
        // canvas starts transparent; every paint lands alpha).
        let non_bg = px.chunks_exact(4).filter(|p| p[3] != 0).count();
        (fnv1a(&px), non_bg)
    }

    /// The S9 fixture states (tests/face-table/face-fixtures.mjs
    /// 157–159), in fixture order, on look {ears:'fox',
    /// body:'#DE7A35'} at mood neutral.
    fn s9_states() -> Vec<FullState> {
        let mut v = Vec::new();
        v.push(FullState::default());
        let mut s = FullState::default();
        s.ear_a = 0.3;
        v.push(s);
        let mut s = FullState::default();
        s.ear_a = -0.3;
        v.push(s);
        let mut s = FullState::default();
        s.ear_a = 0.3;
        s.ear_al = Some(-0.2);
        v.push(s);
        let mut s = FullState::default();
        s.ear_a = 0.3;
        s.ear_ar = Some(0.25);
        v.push(s);
        let mut s = FullState::default();
        s.ear_al = Some(-0.2);
        s.ear_ar = Some(0.25);
        v.push(s);
        v
    }

    /// Spec §5b acceptance (S9-ears): TS 6 classes -> Rust 6
    /// (pre-port 1: all six merged into the no-ears default).
    /// All 15 within-group pairs must distinguish, and every ear
    /// cell's non-background pixel count must EXCEED the no-ears
    /// default's (pre-port it was equal, 82,260 in the TS-side
    /// replay's stage canvas; the Rust canvas is 2560x2816, so
    /// the comparison here is against this painter's own
    /// no-ears count, measured in the same test).
    #[test]
    fn ears_s9_six_classes_and_more_painted_pixels() {
        let mut lk = look("#DE7A35");
        lk.ears = true;
        let plain = look("#DE7A35"); // ears OFF, same body colour
        let (_, plain_non_bg) = paint(&plain, &FullState::default());
        let mut hashes = Vec::new();
        for st in s9_states() {
            let (h, non_bg) = paint(&lk, &st);
            assert!(
                non_bg > plain_non_bg,
                "ear cell must paint more non-background pixels than the no-ears default ({non_bg} <= {plain_non_bg})"
            );
            hashes.push(h);
        }
        let distinct: HashSet<u64> = hashes.iter().copied().collect();
        assert_eq!(distinct.len(), 6, "S9-ears must refine to 6 classes, got {}", distinct.len());
        for i in 0..hashes.len() {
            for j in (i + 1)..hashes.len() {
                assert_ne!(hashes[i], hashes[j], "S9 cells {i} and {j} must be pixel-distinct");
            }
        }
    }

    /// Spec §5c acceptance (S10-hair): 5 cells -> TS 4 classes
    /// (pre-port Rust 1). The absent == all-zero hairSegs EQ
    /// pair must HOLD (both paint the 6 rest strands); the
    /// mixed-angle 6-strand cell and the 3-strand cell each
    /// distinguish AT PIXEL LEVEL.
    ///
    /// MEASURED DIVERGENCE from the spec's "4 classes" bar, with
    /// root cause (CI run 44244470403, this branch): the
    /// hairCol '#101010' cell is PIXEL-IDENTICAL to the absent
    /// cell in this painter (FNV fb94c92d734f770f for both; 0
    /// differing pixels), so the pixel-level count is 3 classes,
    /// not 4. Root cause: the TS fixture instrument is NOT a
    /// pixel hash — tests/face-table/face-fixtures.mjs defines
    /// streamHash as the sha256 of the recorded 2D OP STREAM
    /// ("the house pixel proxy: identical op stream => identical
    /// figure"), and the hairCol cell distinguishes there
    /// because the recorded strokeStyle value differs. In actual
    /// pixels the colour pass (lw 11, centred inside the lw-17
    /// ink pass) is fully occluded by the head disc + its lw-10
    /// ink ring in BOTH painters: the strand centrelines lie
    /// inside the disc (roots at 0.98/0.95 of the radii), and
    /// every sub-segment that crosses the disc edge (the root
    /// round-caps and the side strands' outer edges) lies within
    /// the ring's +/-5 band around the ellipse. The Rust painter
    /// DOES emit the distinguishing op: the gpupaint trace of
    /// the hairCol cell carries StrokeQuad prims coloured
    /// (16,16,16) vs the default's (107,74,58) — asserted in
    /// gpupaint_states_earshair::tests (feature-gated), which is
    /// the Rust op-stream equivalent of the TS fixture hash.
    /// Painter geometry is unchanged by this finding: the port
    /// mirrors the TS painter exactly, occlusion included.
    #[test]
    fn hair_s10_pixel_classes_with_absent_eq_zeroes() {
        let mut lk = look("#E8734A");
        lk.hair = true;
        let absent = paint(&lk, &FullState::default()).0;
        let mut s = FullState::default();
        s.hair_segs = vec![0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        let zeroes = paint(&lk, &s).0;
        assert_eq!(absent, zeroes, "absent hairSegs == all-zero hairSegs (6 rest strands)");
        let mut s = FullState::default();
        s.hair_segs = vec![0.3, -0.3, 0.15, 0.0, 0.0, -0.15];
        let mixed = paint(&lk, &s).0;
        let mut s = FullState::default();
        s.hair_segs = vec![0.2, 0.0, -0.2];
        let three = paint(&lk, &s).0;
        let mut lk_col = look("#E8734A");
        lk_col.hair = true;
        lk_col.hair_col = Some("#101010".into());
        let col = paint(&lk_col, &FullState::default()).0;
        // Pixel level (see this test's doc): col is pixel-EQUAL
        // to absent (0 differing pixels — the colour pass is
        // occluded in both painters); its distinction is at
        // op-stream level, asserted in the gpupaint battery.
        assert_eq!(col, absent, "hairCol cell is pixel-identical to absent (occluded colour pass; op-stream distinction asserted in gpupaint_states_earshair)");
        let classes: HashSet<u64> = [absent, mixed, three].into_iter().collect();
        assert_eq!(classes.len(), 3, "S10-hair pixel classes: absent/zeroes, mixed, three-strand (hairCol is the 4th TS class, at op-stream level only)");
    }

    /// Non-regression for the ports themselves: ears OFF and
    /// hair OFF (every existing character's default) must paint
    /// byte-identically to the ears/hair channels never having
    /// been set — including when the (now-read) state fields
    /// carry non-default values under an OFF look flag: the TS
    /// painter gates both blocks on the LOOK, so ear/hair state
    /// under ears:false / hair:false paints nothing.
    #[test]
    fn ears_hair_off_look_paints_nothing_extra() {
        let lk = look("#DE7A35"); // ears: false, hair: false
        let base = paint(&lk, &FullState::default()).0;
        let mut s = FullState::default();
        s.ear_a = 0.3;
        s.ear_al = Some(-0.2);
        s.ear_ar = Some(0.25);
        s.hair_segs = vec![0.3, -0.3, 0.15];
        assert_eq!(paint(&lk, &s).0, base, "state under an OFF look flag must not paint");
        // And the standing default pin (carry_plumbing_tests
        // case A) is the byte-identity proof at defaults.
    }

    /// Ears envelope (§5b): with a strong lean the ear tips are
    /// the envelope's extreme; a look WITH ears must anchor at
    /// least as far as the same state without them, and the
    /// earred anchor must differ when the ears are what pushes
    /// the envelope out. Hair envelope (§5c): NONE — the same
    /// state with hair:true vs hair:false must return the
    /// IDENTICAL anchor (the TS anchor does not model hair).
    #[test]
    fn ears_anchor_envelopes_hair_anchor_does_not() {
        // A lean that engages the anchor (mirrors the B-1 lying
        // family, milder so the ear tips are near the edge).
        let mut st = FullState::default();
        st.lean = 82.0;
        st.head_tilt = 4.0;
        let plain = look("#DE7A35");
        let mut earred = look("#DE7A35");
        earred.ears = true;
        let a_plain = canvas_anchor(&plain, &st);
        let a_ears = canvas_anchor(&earred, &st);
        assert_ne!(a_plain, (0.0, 0.0), "test state must engage the anchor");
        assert!(
            a_ears.0.abs() >= a_plain.0.abs() && a_ears.1.abs() >= a_plain.1.abs(),
            "ears envelope must not shrink the anchor: plain {a_plain:?} vs ears {a_ears:?}"
        );
        // Hair: identical anchor, by design (§5c).
        let mut haired = look("#DE7A35");
        haired.hair = true;
        let mut s2 = FullState::default();
        s2.hair_segs = vec![0.3, -0.3, 0.15, 0.0, 0.0, -0.15];
        s2.lean = 82.0;
        assert_eq!(
            canvas_anchor(&haired, &s2),
            canvas_anchor(&plain, &s2),
            "hair must NOT change the canvas anchor (§5c: no hair envelope)"
        );
    }
}
