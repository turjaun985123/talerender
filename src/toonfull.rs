//! Full drawToon port (production integration): the complete
//! character painter from the source engine's toon.ts — all 19
//! moods, walk/run/sit/jump legs, patterns (vest, stripes, apron),
//! neckwear (tie, scarf, collar), hats (cap, bowler, boater,
//! beanie, visor, tophat, bun, flower, bow, mailcap), extras
//! (glasses, mustache, blush, freckles, pencil, monocle), blink /
//! talk / look / sweat / tears, headTilt / headTurn / camPitch /
//! squash, the wing appendage channel (RUST-PARITY-SPECS Spec 1:
//! evaluator + panel painter), and held-prop plates — where
//! scene::paint_toon (M4) is
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
    pub hair_col: Option<String>, // 'hairCol' (bow hat's side locks; hair strands are a later port)
    // GPUFIRST Batch 2 CARRY plumbing (DESIGN-CARRY fields):
    // the wing / ears / hair look channels of RUST-PARITY-SPECS
    // Spec 1 / §5b / §5c. The FIELDS exist so no format break
    // stands between here and the painter ports. GPUFIRST
    // Batch 3 landed the WING painter (Spec 1, below), so the
    // wing fields ARE read now; ears / hair painters are still
    // deferred and their fields remain unread. Every pre-existing
    // look (all defaults below) paints byte-identically.
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
    // GPUFIRST Batch 2 CARRY plumbing (DESIGN-CARRY fields):
    // the wing / ears / hair / flip / bodyPitch state channels of
    // RUST-PARITY-SPECS Spec 1 / §5b / §5c / §5d / Spec 3a. They
    // are keyed in film.rs state_key per the §3b coverage law, so
    // a port cannot silently recreate the F-1 defect on these
    // channels. GPUFIRST Batch 3 landed the WING painter (Spec
    // 1), so wing_segs / wing_a ARE read now; the ears / hair /
    // flip / bodyPitch painters are other seats' ports and their
    // fields remain unread here.
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

/// Composite a held-prop plate at a stage-space hand point,
/// inverse-mapping each canvas pixel through the lean rotation
/// (stage -> canvas is lean about (256, 1000), then the canvas
/// re-anchor, then +(384, 256), then x res). Bilinear sampling
/// of the plate.
fn composite_plate(p: &mut Painter, plate: &Plate, hand: (f32, f32), lean_rad: f32, anchor: (f32, f32)) {
    #[cfg(feature = "gpupaint")]
    crate::gputrace::trace_composite_plate(plate, hand, lean_rad, anchor);
    let (s, c) = lean_rad.sin_cos();
    let (ax, ay) = anchor;
    // Canvas-space corners of the plate rect (forward map).
    let fwd = |lx: f32, ly: f32| -> (f32, f32) {
        let (dx, dy) = (lx - 256.0, ly - 1000.0);
        let (rx, ry) = (256.0 + dx * c - dy * s, 1000.0 + dx * s + dy * c);
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
            // canvas -> stage (inverse lean + anchor).
            let (ux, uy) = (x as f32 / RES - 384.0 - ax, y as f32 / RES - 256.0 - ay);
            let (dx, dy) = (ux - 256.0, uy - 1000.0);
            let sx = 256.0 + dx * c + dy * s;
            let sy = 1000.0 - dx * s + dy * c;
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

// ---------------------------------------------------------------------
// Wing appendage (RUST-PARITY-SPECS Spec 1; GPUFIRST Batch 3,
// seat PORT-WING): the evaluator (a port of secondary.ts
// resolveWing / wingFlap / wingChain / wingPose and the spring
// kernels it builds on), the shared chain geometry (toon.ts
// wingChainGeom) and the panel painter (toon.ts paintWingChain).
//
// The evaluator runs in f64 end to end, mirroring the TS
// doubles, so the Spec 1 acceptance values reproduce at TS
// tolerances (±1e-12 on the chain/flap identities). The painter
// consumes evaluated state (FullState.wing_segs / wing_a, f32)
// exactly as it consumes every other evaluated channel — shot
// code / staging runs the evaluator, the painter paints state.
// ---------------------------------------------------------------------

/// The house spring in secondary.ts form: (freq, decay) plus the
/// derived oscillator constants (makeSpring's clamps applied).
#[derive(Clone, Copy)]
struct WingSpring {
    freq: f64,
    decay: f64,
    omega_d: f64,
    omega: f64,
}

/// secondary.ts makeSpring: freq clamped to [0.05, 15] Hz, decay
/// to [0.3, 60] /s, then decay trimmed so zeta <= 0.98
/// (d <= 4.924 * w, w = 2π·freq).
fn wing_make_spring(freq: f64, decay: f64) -> WingSpring {
    let f = freq.clamp(0.05, 15.0);
    let w = std::f64::consts::TAU * f;
    let d = decay.clamp(0.3, 60.0).min(4.924 * w);
    WingSpring { freq: f, decay: d, omega_d: w, omega: (w * w + d * d).sqrt() }
}

/// secondary.ts springFromAppendage: an AppendageDef's
/// (stiffness, damping) as house (freq, decay) — decay =
/// damping/2, freq = sqrt(stiffness − damping²/4)/(2π), freq
/// floored at 0.05 Hz (the overdamped-boundary clamp).
fn wing_spring_from_appendage(stiffness: f64, damping: f64) -> (f64, f64) {
    let decay = damping.max(0.0) / 2.0;
    let wd2 = stiffness - decay * decay;
    ((wd2.max(0.0).sqrt() / std::f64::consts::TAU).max(0.05), decay)
}

/// motion.ts springStep via secondary.ts springStepAt: the unit
/// step response of the house spring, exactly 0 for tau <= 0.
fn wing_spring_step_at(tau: f64, s: &WingSpring) -> f64 {
    if tau <= 0.0 {
        return 0.0;
    }
    let w = std::f64::consts::TAU * s.freq;
    1.0 - (-s.decay * tau).exp() * ((w * tau).cos() + (s.decay / w) * (w * tau).sin())
}

/// secondary.ts springRampAt: the unit RAMP response (the
/// integral of the step response), exactly 0 for tau <= 0.
fn wing_spring_ramp_at(tau: f64, s: &WingSpring) -> f64 {
    if tau <= 0.0 {
        return 0.0;
    }
    let (d, w) = (s.decay, s.omega_d);
    let dd = s.omega * s.omega;
    let e = (-d * tau).exp();
    tau - (2.0 * d - e * (2.0 * d * (w * tau).cos() + ((d * d - w * w) / w) * (w * tau).sin())) / dd
}

/// secondary.ts springTrack: the EXACT tracker response to a
/// piecewise-linear target given as (t, v) keys — the signal
/// holds the first key's value before it, runs linearly between
/// keys, holds the last after it; duplicate key times are
/// instantaneous jumps (step responses). Empty keys -> 0.
fn wing_spring_track(keys_in: &[(f64, f64)], t: f64, s: &WingSpring) -> f64 {
    if keys_in.is_empty() {
        return 0.0;
    }
    let mut ks: Vec<(f64, f64)> = keys_in.to_vec();
    ks.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    let mut x = ks[0].1;
    for i in 1..ks.len() {
        let (t0, v0) = ks[i - 1];
        let (t1, v1) = ks[i];
        let dv = v1 - v0;
        if dv == 0.0 {
            continue;
        }
        let dt = t1 - t0;
        if dt <= 0.0 {
            x += dv * wing_spring_step_at(t - t1, s);
        } else {
            let m = dv / dt;
            x += m * wing_spring_ramp_at(t - t0, s) - m * wing_spring_ramp_at(t - t1, s);
        }
    }
    if x.is_finite() {
        x
    } else {
        ks[0].1
    }
}

/// secondary.ts springResponse: the tracker response to an
/// arbitrary driver FUNCTION of t, history-free — the driver is
/// sampled onto an absolute time grid (anchored at 0, so seeks
/// agree bit-for-bit) over the finite-memory window
/// [t − H, t], H = min(9/decay, 30) s, sampling step
/// clamp(1/(16·freq), 0.008, 0.1) s, and the samples run through
/// the exact springTrack. Non-finite samples read as 0.
fn wing_spring_response(driver: &dyn Fn(f64) -> f64, t: f64, s: &WingSpring) -> f64 {
    let h = (9.0 / s.decay).min(30.0);
    let dt = (1.0 / (16.0 * s.freq)).clamp(0.008, 0.1);
    let start = t - h;
    let mut keys: Vec<(f64, f64)> = Vec::new();
    let mut push = |keys: &mut Vec<(f64, f64)>, tt: f64| {
        let v = driver(tt);
        keys.push((tt, if v.is_finite() { v } else { 0.0 }));
    };
    push(&mut keys, start);
    let k0 = (start / dt).ceil() as i64;
    let k1 = (t / dt).floor() as i64;
    for k in k0..=k1 {
        let tt = k as f64 * dt;
        if tt > start && tt < t {
            push(&mut keys, tt);
        }
    }
    if t > start {
        push(&mut keys, t);
    }
    wing_spring_track(&keys, t, s)
}

/// secondary.ts shapeChain: progressive chain shaping — each
/// segment's bend relative to its parent clamped to ±max_bend,
/// the absolute angle to ±2.8 rad, non-finite raw angles read as
/// 0. Clamping runs root→tip.
fn wing_shape_chain(raw: &[f64], max_bend: f64) -> Vec<f64> {
    let mut out: Vec<f64> = Vec::with_capacity(raw.len());
    for &rv in raw {
        let mut a = if rv.is_finite() { rv } else { 0.0 };
        a = a.clamp(-2.8, 2.8);
        if let Some(&prev) = out.last() {
            a = a.clamp(prev - max_bend, prev + max_bend);
        }
        out.push(a);
    }
    out
}

/// A character's motion as pure functions of t — the driver
/// bundle the wing evaluator consumes (secondary.ts
/// AnchorMotion's speed / phase; a missing field is a still
/// anchor for that channel).
#[allow(dead_code)] // staging API: exercised by wing_tests; film staging adopts it with the evaluator wiring
pub(crate) struct WingMotion<'a> {
    /// Signed speed along facing (engine units/s).
    pub speed: Option<&'a (dyn Fn(f64) -> f64 + 'a)>,
    /// Gait / flap phase (radians).
    pub phase: Option<&'a (dyn Fn(f64) -> f64 + 'a)>,
}

impl Default for WingMotion<'_> {
    fn default() -> Self {
        WingMotion { speed: None, phase: None }
    }
}

/// The authored fold: a number taken as-is, or a function of t
/// spring-tracked through the wing's spring (secondary.ts
/// WingParams.fold).
#[allow(dead_code)]
pub(crate) enum WingFold<'a> {
    Value(f64),
    Func(&'a (dyn Fn(f64) -> f64 + 'a)),
}

/// secondary.ts WingParams — every field optional; the resolved
/// defaults are fold 1, foldBase 1.25, spreadBase 1.5, foldBend
/// 2.0, spreadBend 0.18, flapAmp 0.5, flapLag 0.6, speedRef 1.6,
/// speedSpread 0.65, idleAmp 0.03, idleFreq 0.25, maxBend 2.2,
/// spring = the def's (via springFromAppendage) else the house
/// default (2.2 Hz, 3.5 /s).
#[allow(dead_code)]
pub(crate) struct WingParams<'a> {
    /// The appendage's spring as (freq, decay); wins over `def`.
    pub spring: Option<(f64, f64)>,
    /// The AppendageDef's (stiffness, damping) — supplies the
    /// spring via springFromAppendage when `spring` is absent.
    pub def: Option<(f64, f64)>,
    /// Authored fold amount 0..1 (1 = fully folded). Default 1.
    pub fold: Option<WingFold<'a>>,
    pub fold_base: Option<f64>,
    pub spread_base: Option<f64>,
    pub fold_bend: Option<f64>,
    pub spread_bend: Option<f64>,
    pub flap_amp: Option<f64>,
    pub flap_lag: Option<f64>,
    pub speed_ref: Option<f64>,
    pub speed_spread: Option<f64>,
    pub idle_amp: Option<f64>,
    pub idle_freq: Option<f64>,
    pub max_bend: Option<f64>,
}

impl Default for WingParams<'_> {
    fn default() -> Self {
        WingParams {
            spring: None,
            def: None,
            fold: None,
            fold_base: None,
            spread_base: None,
            fold_bend: None,
            spread_bend: None,
            flap_amp: None,
            flap_lag: None,
            speed_ref: None,
            speed_spread: None,
            idle_amp: None,
            idle_freq: None,
            max_bend: None,
        }
    }
}

/// secondary.ts WingPose: the evaluated wing at t.
#[allow(dead_code)]
pub(crate) struct WingPose {
    /// Effective fold 0..1 (authored fold after the speed response).
    pub fold: f64,
    /// 1 − fold.
    pub spread: f64,
    /// Base-segment flap angle at t (wing_flap's value).
    pub flap: f64,
    /// Segment 0's absolute angle (= segs[0] when segments > 0).
    pub base: f64,
    /// The per-joint bend applied (rad, signed).
    pub bend: f64,
    /// Per-segment absolute angles, root first.
    pub segs: Vec<f64>,
}

struct WingResolved {
    fold_base: f64,
    spread_base: f64,
    fold_bend: f64,
    spread_bend: f64,
    flap_amp: f64,
    flap_lag: f64,
    idle_amp: f64,
    idle_freq: f64,
    max_bend: f64,
    eff_fold: f64,
    speed_resp: f64,
}

/// secondary.ts resolveWing + chainSpring: the spring is
/// params.spring, else the def's via springFromAppendage, else
/// the house default. Speed spreads the wing: effFold =
/// clamp(fold · (1 − speedSpread · speedResp), 0, 1), where
/// speedResp is the spring response of speed/speedRef.
fn wing_resolve(motion: &WingMotion, params: &WingParams, t: f64) -> WingResolved {
    let s = if let Some((f, d)) = params.spring {
        wing_make_spring(f, d)
    } else if let Some((k, c)) = params.def {
        let (f, d) = wing_spring_from_appendage(k, c);
        wing_make_spring(f, d)
    } else {
        wing_make_spring(2.2, 3.5)
    };
    let speed_ref = params.speed_ref.unwrap_or(1.6);
    let speed_resp = match motion.speed {
        Some(f) => wing_spring_response(&|tt: f64| f(tt) / speed_ref, t, &s).clamp(0.0, 1.0),
        None => 0.0,
    };
    let authored = match &params.fold {
        Some(WingFold::Func(f)) => wing_spring_response(f, t, &s),
        Some(WingFold::Value(v)) => *v,
        None => 1.0,
    };
    let speed_spread = params.speed_spread.unwrap_or(0.65);
    let eff_fold = ((if authored.is_finite() { authored } else { 1.0 })
        * (1.0 - speed_spread * speed_resp))
        .clamp(0.0, 1.0);
    WingResolved {
        fold_base: params.fold_base.unwrap_or(1.25),
        spread_base: params.spread_base.unwrap_or(1.5),
        fold_bend: params.fold_bend.unwrap_or(2.0),
        spread_bend: params.spread_bend.unwrap_or(0.18),
        flap_amp: params.flap_amp.unwrap_or(0.5),
        flap_lag: params.flap_lag.unwrap_or(0.6),
        idle_amp: params.idle_amp.unwrap_or(0.03),
        idle_freq: params.idle_freq.unwrap_or(0.25),
        max_bend: params.max_bend.unwrap_or(2.2),
        eff_fold,
        speed_resp,
    }
}

/// secondary.ts wingFlap: flapAmp · spread · sin(phase(t)) —
/// exactly 0 with no phase driver, and exactly 0 at full fold
/// for any phase.
#[allow(dead_code)]
pub(crate) fn wing_flap(motion: &WingMotion, params: &WingParams, t: f64) -> f64 {
    let r = wing_resolve(motion, params, t);
    let phase_fn = match motion.phase {
        Some(f) => f,
        None => return 0.0,
    };
    let v = r.flap_amp * (1.0 - r.eff_fold) * phase_fn(t).sin();
    if v.is_finite() {
        v
    } else {
        0.0
    }
}

/// secondary.ts wingChain: the per-segment ABSOLUTE angles at t
/// (chain convention: 0 = straight down, + toward +x; root
/// first) for a chain of `segments` segments. Per segment k:
///   a_k = base + bend·k
///       + flapAmp · spread · sin(phase − k·flapLag)   (if phased)
///       + idleAmp · sin(2π·idleFreq·t − k·flapLag) · (1 − speedResp)
/// then shapeChain's bend/absolute clamps. At rest (fold 1, no
/// drivers, idleAmp 0) the chain is exactly [1.25, −0.75, …].
#[allow(dead_code)]
pub(crate) fn wing_chain(motion: &WingMotion, segments: usize, params: &WingParams, t: f64) -> Vec<f64> {
    let r = wing_resolve(motion, params, t);
    let spread = 1.0 - r.eff_fold;
    let base = r.fold_base + (r.spread_base - r.fold_base) * spread;
    let bend = -r.fold_bend + (r.fold_bend + r.spread_bend) * spread;
    let mut raw: Vec<f64> = Vec::with_capacity(segments);
    for k in 0..segments {
        let kf = k as f64;
        let mut a = base + bend * kf;
        if let Some(phase_fn) = motion.phase {
            a += r.flap_amp * spread * (phase_fn(t) - kf * r.flap_lag).sin();
        }
        a += r.idle_amp * (std::f64::consts::TAU * r.idle_freq * t - kf * r.flap_lag).sin() * (1.0 - r.speed_resp);
        raw.push(a);
    }
    wing_shape_chain(&raw, r.max_bend)
}

/// secondary.ts wingPose: the chain plus the fold/spread/flap
/// summary.
#[allow(dead_code)]
pub(crate) fn wing_pose(motion: &WingMotion, segments: usize, params: &WingParams, t: f64) -> WingPose {
    let r = wing_resolve(motion, params, t);
    let spread = 1.0 - r.eff_fold;
    let segs = wing_chain(motion, segments, params, t);
    let bend = -r.fold_bend + (r.fold_bend + r.spread_bend) * spread;
    let base = if segs.is_empty() {
        r.fold_base + (r.spread_base - r.fold_base) * spread
    } else {
        segs[0]
    };
    WingPose { fold: r.eff_fold, spread, flap: wing_flap(motion, params, t), base, bend, segs }
}

/// toon.ts WingGeom: the shared wing chain geometry — pts are
/// the root + one point per segment end (stage space), lens /
/// widths per segment (far side scaled ×0.88).
pub(crate) struct WingGeom {
    pub pts: Vec<(f32, f32)>,
    pub lens: Vec<f32>,
    pub widths: Vec<f32>,
}

/// toon.ts wingChainGeom: ONE geometry source for the wing,
/// shared by canvas_anchor's envelope and both wing painters.
/// Segment count = wing_segs' length when the state carries it
/// (any declared count — 1, 2, 3, 5 all honoured), else the
/// scalar fallback family over 2 segments:
/// [1.25 + wing_a, 1.25 + wing_a − 2.0]. wing_segs wins over
/// wing_a when both are set. Segment lengths: look wing_lens[k],
/// else the BIRD_RIG pattern 58, then −12 per segment, floor 24.
/// Widths: 46, then −12 per segment, floor 20. Near root at
/// (bx + 46, top + 96); far root at (bx − 38, top + 84); each
/// joint advances by (sin a · len, cos a · len); the far chain
/// mirrors the angles (a → −a).
pub(crate) fn wing_chain_geom(look: &FullLook, st: &FullState, bx: f32, top: f32, far: bool) -> WingGeom {
    let has_segs = !st.wing_segs.is_empty();
    let n = if has_segs { st.wing_segs.len() } else { 2 };
    let scale = if far { 0.88 } else { 1.0 };
    let mut lens: Vec<f32> = Vec::with_capacity(n);
    for k in 0..n {
        let pat = if k == 0 { 58.0 } else { (58.0 - 12.0 * k as f32).max(24.0) };
        lens.push(look.wing_lens.get(k).copied().unwrap_or(pat) * scale);
    }
    let mut widths: Vec<f32> = Vec::with_capacity(n);
    for k in 0..n {
        widths.push((46.0 - 12.0 * k as f32).max(20.0) * scale);
    }
    let (mut x, mut y) = (bx + if far { -38.0 } else { 46.0 }, top + if far { 84.0 } else { 96.0 });
    let mut pts: Vec<(f32, f32)> = Vec::with_capacity(n + 1);
    pts.push((x, y));
    for k in 0..n {
        let a0 = if has_segs {
            st.wing_segs[k]
        } else if k == 0 {
            1.25 + st.wing_a
        } else {
            1.25 + st.wing_a - 2.0
        };
        let a = if far { -a0 } else { a0 };
        x += a.sin() * lens[k];
        y += a.cos() * lens[k];
        pts.push((x, y));
    }
    WingGeom { pts, lens, widths }
}

/// toon.ts paintWingChain: one filled ellipse per segment,
/// centred on the segment midpoint, long axis along the segment,
/// radii (len/2 + 7, width/2), ink-stroked (lw 7 near / 6 far).
/// Near fill = the wing colour; far fill = shadeOf(col, 0.72)
/// (this painter's shade_rgba is its shadeOf mapping — the same
/// mapping the bean shade / stripes / scarf ports use). Near
/// side only: a shade feather line along each segment
/// (shadeOf(col, 0.75), lw 4, quadratic through the offset
/// midpoint) and a shade joint line across each internal joint
/// (shadeOf(col, 0.7), lw 3). The far wing is shade-filled and
/// featherless. `px` is paint_toon_full's stage→canvas map (lean
/// + canvas anchor, host-side), so every prim below is recorded
/// in device space like every other painter prim: panels trace
/// as FillEllipse + the flattened StrokePolyline of
/// stroke_ellipse, feather lines as the flattened
/// StrokePolyline of stroke_quad, joint lines as
/// StrokePolyline — the existing Prim kinds of
/// GPUFIRST-B2-CARRY §2 (wing verdict: EXPRESSIBLE, no gap).
fn paint_wing_chain(
    p: &mut Painter,
    geom: &WingGeom,
    col: &str,
    far: bool,
    px: &dyn Fn(f32, f32) -> (f32, f32),
) {
    let ink = rgba(INK);
    let n = geom.lens.len();
    for k in 0..n {
        let (x0, y0) = geom.pts[k];
        let (x1, y1) = geom.pts[k + 1];
        let (cx, cy) = px((x0 + x1) / 2.0, (y0 + y1) / 2.0);
        let (ax, ay) = px(x0, y0);
        let (bxp, byp) = px(x1, y1);
        let rot = (byp - ay).atan2(bxp - ax);
        let (rx, ry) = ((geom.lens[k] / 2.0 + 7.0) * RES, (geom.widths[k] / 2.0) * RES);
        let fill = if far { shade_rgba(col, 0.72) } else { rgba(col) };
        p.fill_ellipse(cx, cy, rx, ry, rot, fill);
        p.stroke_ellipse(cx, cy, rx, ry, rot, if far { 6.0 * RES } else { 7.0 * RES }, ink);
    }
    if far {
        return;
    }
    // Feather lines (near only).
    let feather = shade_rgba(col, 0.75);
    for k in 0..n {
        let (x0, y0) = geom.pts[k];
        let (x1, y1) = geom.pts[k + 1];
        let (dx, dy) = (x1 - x0, y1 - y0);
        let len = dx.hypot(dy);
        let len = if len == 0.0 { 1.0 } else { len };
        let (ux, uy) = (dx / len, dy / len);
        let (ox, oy) = (uy * -geom.widths[k] * 0.18, ux * geom.widths[k] * 0.18);
        p.stroke_quad(
            px(x0 + ux * 8.0 + ox, y0 + uy * 8.0 + oy),
            px((x0 + x1) / 2.0 + ox * 1.6, (y0 + y1) / 2.0 + oy * 1.6),
            px(x1 - ux * 8.0 + ox, y1 - uy * 8.0 + oy),
            4.0 * RES,
            feather,
        );
    }
    // Joint lines (near only).
    let joint = shade_rgba(col, 0.7);
    for k in 1..n {
        let (x0, y0) = geom.pts[k - 1];
        let (x1, y1) = geom.pts[k];
        let (dx, dy) = (x1 - x0, y1 - y0);
        let len = dx.hypot(dy);
        let len = if len == 0.0 { 1.0 } else { len };
        let (pxn, pyn) = (-dy / len, dx / len);
        let hw = geom.widths[k] * 0.42;
        p.stroke_polyline(
            &[px(x1 - pxn * hw, y1 - pyn * hw), px(x1 + pxn * hw, y1 + pyn * hw)],
            3.0 * RES,
            joint,
        );
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
/// CARRY note (GPUFIRST Batch 2, updated Batch 3): the ear
/// envelope points the TS anchor models (§5b: both ear
/// triangles) are deliberately NOT added here — the ear painter
/// does not exist yet and the anchor models painted geometry
/// only. The WING envelope points (Spec 1: both wing chains)
/// ARE added below, with the wing painter port (Batch 3, seat
/// PORT-WING), per the port-time obligation recorded in
/// docs/integration/GPUFIRST-B2-CARRY.md §4.
///
/// Port notes (where this painter's geometry differs from the
/// TS painter's, the envelope follows THIS painter): the sit
/// drop is the painter's own (426 - sitDrop)/0.98 with its fixed
/// sit-leg polyline (no two-bone sit solve, no kneel, no
/// tail/scarf-streamer/fox-ear primitives exist here); the
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
    // Body-space push: lean about the feet only (bean, arms, legs,
    // neckwear — the painter maps them through `px`).
    macro_rules! push {
        ($x:expr, $y:expr, $rad:expr) => {{
            let (qx, qy) = rot_pt($x, $y, 256.0, 1000.0, lean);
            pts.push((qx, qy, $rad));
        }};
    }
    // Head-space push: the painter's `hpt` stack in stage space —
    // squash about the head centre (the painter's own factors),
    // headTilt about the neck pivot, then lean about the feet.
    macro_rules! push_head {
        ($x:expr, $y:expr, $rad:expr) => {{
            let (sx2, sy2) = (
                hx + ($x - hx) * (1.0 - 0.3 * st.squash),
                hy + ($y - hy) * (1.0 + st.squash),
            );
            let (tx2, ty2) = rot_pt(sx2, sy2, 256.0, pivot_y, st.head_tilt);
            let (qx, qy) = rot_pt(tx2, ty2, 256.0, 1000.0, lean);
            pts.push((qx, qy, $rad));
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
    // Wing chains (near + far) — Spec 1 port-time obligation
    // (GPUFIRST-B2-CARRY §4): the shared wing_chain_geom points,
    // radii = segment half-widths + the ink stroke (3.5), as the
    // TS anchor models them (toon.ts solveCanvasAnchor wing
    // block). The TS anchor pushes these as upper-assembly
    // (pitched) points; this painter has no bodyPitch painter
    // yet, so they ride the same lean-only `push!` as every
    // other body point here.
    if look.wing {
        for far in [true, false] {
            let geom = wing_chain_geom(look, st, bx, top, far);
            for k in 0..geom.pts.len() {
                let w = if k < geom.widths.len() {
                    geom.widths[k]
                } else {
                    geom.widths.last().copied().unwrap_or(0.0)
                };
                push!(geom.pts[k].0, geom.pts[k].1, w / 2.0 + 3.5);
            }
        }
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
            push!(sx, hip_y, 7.5);
            push!(kx, ky, 7.5);
            push!(fx, fy, 7.5);
            push!(fx + side * 10.0, fy - 4.0, 20.0);
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
            push!(x, y, 7.5);
        }
        push!(fx + side * 10.0, fy - 4.0, 20.0);
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
    // Stage -> canvas: lean about the feet (256, 1000), then the
    // canvas offset, then res (as paint_toon / Toon#set).
    let px = |x: f32, y: f32| -> (f32, f32) {
        let (dx, dy) = (x - 256.0, y - 1000.0);
        (
            (256.0 + dx * cth - dy * sth + anx + 384.0) * RES,
            (1000.0 + dx * sth + dy * cth + any + 256.0) * RES,
        )
    };
    // Head-part point map: squash about (hx, hy), then headTilt
    // about (256, pivotY), then the lean map.
    let sit = st.legs == Legs::Sit;
    let jump = st.legs == Legs::Jump;
    let sit_s = st.sit_drop.unwrap_or(220.2);
    let by = (if sit { (426.0 - sit_s) / 0.98 } else { 0.0 })
        + if look.kid { 190.0 } else { 0.0 }
        + st.bob
        + if jump { -60.0 } else { 0.0 };
    let (bx, top, bot, bw) = (256.0f32, 330.0 + by, 600.0 + by * 0.98, 92.0f32);
    let hip_y = bot - 26.0;
    let (hx, hy, r) = (256.0f32, 178.0 + by * 0.99, 138.0f32);
    let pivot_y = 318.0 + by * 0.99;
    let tilt = st.head_tilt.to_radians();
    let (sti, cti) = tilt.sin_cos();
    let sq = st.squash;
    let hpt = |x: f32, y: f32| -> (f32, f32) {
        // squash about the head centre
        let (sx, sy) = (hx + (x - hx) * (1.0 - 0.3 * sq), hy + (y - hy) * (1.0 + sq));
        // tilt about the neck pivot
        let (dx, dy) = (sx - 256.0, sy - pivot_y);
        let (rx, ry) = (256.0 + dx * cti - dy * sti, pivot_y + dx * sti + dy * cti);
        px(rx, ry)
    };
    let head_rot = th + tilt;

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
            p.fill_ellipse(ex, ey, 20.0 * RES, 10.0 * RES, th, ink);
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
        p.fill_ellipse(ex, ey, 20.0 * RES, 10.0 * RES, th, ink);
    }

    // ---- appendage: far wing (behind the body) — Spec 1. The
    // same chain as the near wing, mirrored to the body's far
    // side and shade-filled, painted BEFORE the body bean so the
    // body occludes it (the TS paint site is inside the pitched
    // body block just before the bean; this painter has no
    // bodyPitch transform yet, so the wing rides the plain `px`
    // map like every other body-space prim here). Colour:
    // look wingCol, falling back to the body colour (TS:
    // L.wingCol || L.body — an empty string is falsy in JS and
    // falls back too).
    if look.wing {
        let col = look.wing_col.as_deref().filter(|s| !s.is_empty()).unwrap_or(&look.body);
        let geom = wing_chain_geom(look, st, bx, top, true);
        paint_wing_chain(&mut p, &geom, col, true, &px);
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
    seg(flatten_cubic(px(bx - 58.0, top + 10.0), px(bx - 80.0, top + 40.0), px(bx - bw - 6.0, bot - 120.0), px(bx - bw, bot - 50.0), 28));
    seg(flatten_cubic(px(bx - bw, bot - 50.0), px(bx - bw + 4.0, bot + 6.0), px(bx + bw - 4.0, bot + 6.0), px(bx + bw, bot - 50.0), 28));
    seg(flatten_cubic(px(bx + bw, bot - 50.0), px(bx + bw + 6.0, bot - 120.0), px(bx + 80.0, top + 40.0), px(bx + 58.0, top + 10.0), 28));
    seg(flatten_cubic(px(bx + 58.0, top + 10.0), px(bx + 30.0, top - 8.0), px(bx - 30.0, top - 8.0), px(bx - 58.0, top + 10.0), 28));
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
                px(sx, top - 20.0),
                px(sx + 14.0, top - 20.0),
                px(sx + 14.0, bot + 20.0),
                px(sx, bot + 20.0),
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
                .map(|&(x, y)| px(x, y))
                .collect();
        fill_polygon_masked(&mut p, &body_pts, &mask, rgba(pc_hex));
        let pocket: Vec<(f32, f32)> =
            rounded_rect_pts_radii(bx - 40.0, top + 190.0, 80.0, 60.0, [8.0; 4], 6)
                .iter()
                .map(|&(x, y)| px(x, y))
                .collect();
        let mut ring = pocket.clone();
        ring.push(pocket[0]);
        p.stroke_polyline(&ring, lw(4.0), shade_rgba(pc_hex, 0.8));
    }
    if look.pattern == "vest" {
        let pc = look.pat_col.as_deref().unwrap_or("#3B3346");
        for s in [-1.0f32, 1.0] {
            let panel = [
                px(bx + s * 140.0, top - 10.0),
                px(bx + s * 8.0, top + 120.0),
                px(bx + s * 8.0, bot + 20.0),
                px(bx + s * 140.0, bot + 20.0),
            ];
            fill_polygon_masked(&mut p, &panel, &mask, rgba(pc));
        }
        for dy in [150.0f32, 195.0, 240.0] {
            let (cxp, cyp) = px(bx + 26.0, top + dy);
            p.fill_ellipse(cxp, cyp, 6.0 * RES, 6.0 * RES, th, rgba("#E9C46A"));
        }
    }
    // Shade + highlight, clipped.
    let (shx, shy) = px(bx + bw + 40.0, (top + bot) / 2.0 + 20.0);
    let shade_col = match &look.shade {
        Some(s) => rgba(s.as_str()),
        None => shade_rgba(&look.body, 0.8),
    };
    p.fill_ellipse_masked(shx, shy, 70.0 * RES, 260.0 * RES, th, (shade_col.0, shade_col.1, shade_col.2, (0.85 * 255.0) as u8), &mask);
    let (hix, hiy) = px(bx - 50.0, top + 90.0);
    p.fill_ellipse_masked(hix, hiy, 16.0 * RES, 60.0 * RES, 0.2 + th, (255, 255, 255, (0.25 * 255.0) as u8), &mask);
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
                px(bx - 14.0, top + 10.0),
                px(bx + 14.0, top + 10.0),
                px(bx + 20.0, top + 140.0),
                px(bx, top + 170.0),
                px(bx - 20.0, top + 140.0),
            ];
            p.fill_polygon(&poly, rgba(c));
            let mut ring = poly.to_vec();
            ring.push(poly[0]);
            p.stroke_polyline(&ring, lw(6.0), ink);
        }
        "scarf" => {
            let c = look.neck_col.as_deref().unwrap_or("#D8343F");
            let tail = [
                px(bx + 20.0, top + 20.0),
                px(bx + 58.0, top + 130.0),
                px(bx + 24.0, top + 138.0),
                px(bx + 6.0, top + 26.0),
            ];
            p.fill_polygon(&tail, rgba(c));
            let mut ring = tail.to_vec();
            ring.push(tail[0]);
            p.stroke_polyline(&ring, lw(6.0), ink);
            // Wrap: two quads forming the band, then its shade line.
            let mut band: Vec<(f32, f32)> = Vec::new();
            band.extend_from_slice(&flatten_quad(px(bx - 70.0, top + 4.0), px(bx, top + 44.0), px(bx + 70.0, top + 4.0), 16));
            let lower = flatten_quad(px(bx + 70.0, top + 4.0), px(bx + 66.0, top + 34.0), px(bx, top + 76.0), 16);
            band.extend_from_slice(&lower[1..]);
            let lower2 = flatten_quad(px(bx, top + 76.0), px(bx - 66.0, top + 34.0), px(bx - 70.0, top + 4.0), 16);
            band.extend_from_slice(&lower2[1..]);
            p.fill_polygon(&band, rgba(c));
            let mut bring = band.clone();
            bring.push(band[0]);
            p.stroke_polyline(&bring, lw(6.0), ink);
            let shade = shade_rgba(c, 0.75);
            p.stroke_quad(
                px(bx - 50.0, top + 26.0),
                px(bx, top + 56.0),
                px(bx + 50.0, top + 26.0),
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
                    px(bx, top + 18.0),
                    px(bx + s * 56.0, top + 2.0),
                    px(bx + s * 40.0, top + 46.0),
                ];
                p.fill_polygon(&tri, rgba("#FFFFFF"));
                let mut ring = tri.to_vec();
                ring.push(tri[0]);
                p.stroke_polyline(&ring, lw(6.0), ink);
            }
        }
        _ => {}
    }

    // ---- appendage: near wing (over the body, under the arms)
    // — Spec 1: the chain of flat segment panels honouring the
    // declared segment count (state wing_segs' length; 2 at the
    // scalar wing_a fallback), painted after the body +
    // neckwear and before the arms, as in TS.
    if look.wing {
        let col = look.wing_col.as_deref().filter(|s| !s.is_empty()).unwrap_or(&look.body);
        let geom = wing_chain_geom(look, st, bx, top, false);
        paint_wing_chain(&mut p, &geom, col, false, &px);
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
        let pts = flatten_quad(px(sx, sy), px(ex, ey), px(hx2, hy2), 20);
        p.stroke_polyline(&pts, lw(14.0), ink);
        if let Some(plate) = hold {
            composite_plate(&mut p, plate, (hx2, hy2), th, (anx, any));
        }
        let (hxp, hyp) = px(hx2, hy2);
        p.fill_ellipse(hxp, hyp, 15.0 * RES, 17.0 * RES, th, ink);
        if side < 0.0 {
            hands.0 = (hx2, hy2);
        } else {
            hands.1 = (hx2, hy2);
        }
    }

    // ---- head ----
    let turn = st.head_turn;
    let rp = r * (1.0 + 0.04 * st.cam_pitch);
    let fx = hx + turn * 30.0;
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
                let (rx, ry, rot) = (rp * 0.98 * (1.0 - 0.3 * sq) * RES, 62.0 * (1.0 + sq) * RES, -0.05 + head_rot);
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
                let (rx, ry, rot) = (rp * 0.78 * (1.0 - 0.3 * sq) * RES, 22.0 * (1.0 + sq) * RES, 0.05 + head_rot);
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
                let lrot = head_rot - s * 0.2;
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
                let (rx, ry, rot) = (rp * 0.82 * (1.0 - 0.3 * sq) * RES, 18.0 * (1.0 + sq) * RES, 0.04 + head_rot);
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
mod wing_tests {
    use super::*;
    use std::collections::HashSet;

    /// Full-frame FNV-1a 64 hash of a paint (the house test
    /// instrument; no new dependency).
    fn fnv(px: &[u8]) -> u64 {
        let mut h: u64 = 0xcbf29ce484222325;
        for &b in px {
            h ^= b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        h
    }

    /// The reference bird's look as the bridge derives it from
    /// the BIRD_RIG wing declaration (characters.ts: segments 2,
    /// segLen 58, palette '#7FB6D9' -> wingCol '#7FB6D9',
    /// wingLens [58, 46]) on the scene-format default body.
    fn pip_look() -> FullLook {
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
            wing: true,
            wing_col: Some("#7FB6D9".into()),
            wing_lens: vec![58.0, 46.0],
            ears: false,
            hair: false,
        }
    }

    fn paint_hash(look: &FullLook, st: &FullState) -> u64 {
        let (px, w, h, _, _) = paint_toon_full(look, st);
        assert_eq!((w, h), (2560, 2816));
        fnv(&px)
    }

    fn count_col(px: &[u8], col: Rgba) -> usize {
        px.chunks_exact(4)
            .filter(|c| c[0] == col.0 && c[1] == col.1 && c[2] == col.2 && c[3] == col.3)
            .count()
    }

    /// Sample the canvas pixel at a stage point. Only valid for
    /// states whose canvas anchor is (0, 0) — asserted here, so a
    /// sample can never silently read a re-anchored frame.
    fn sample_stage(look: &FullLook, st: &FullState, sx: f32, sy: f32) -> Rgba {
        assert_eq!(canvas_anchor(look, st), (0.0, 0.0), "sample state must not engage the anchor");
        let (px, w, _, _, _) = paint_toon_full(look, st);
        let cx = ((sx + 384.0) * RES) as usize;
        let cy = ((sy + 256.0) * RES) as usize;
        let k = (cy * w as usize + cx) * 4;
        (px[k], px[k + 1], px[k + 2], px[k + 3])
    }

    // ---------------------------------------------------------
    // Spec 1 acceptance 1 — evaluator unit values (the TS proof
    // numbers of tests/14/wing.test.mjs, quoted in the spec).
    // ---------------------------------------------------------

    #[test]
    fn wing_evaluator_rest_chain_matches_ts_proof() {
        let motion = WingMotion::default();
        let rest = WingParams { idle_amp: Some(0.0), ..Default::default() };
        // At rest (fold 1, no drivers, idleAmp 0) the chain is
        // exactly [1.25, -0.75, -2.75] (a_k = 1.25 - 2.0*k,
        // before clamps; the clamps do not move these values).
        let folded = wing_chain(&motion, 3, &rest, 3.7);
        assert_eq!(folded, vec![1.25, -0.75, -2.75]);
        // Spread rest: [spreadBase, spreadBase + spreadBend].
        let spread_params = WingParams { idle_amp: Some(0.0), fold: Some(WingFold::Value(0.0)), ..Default::default() };
        let spread = wing_chain(&motion, 2, &spread_params, 3.7);
        assert!((spread[0] - 1.5).abs() <= 1e-12, "spread base {}", spread[0]);
        assert!((spread[1] - 1.68).abs() <= 1e-12, "spread seg1 {}", spread[1]);
        // Half-fold: base is the midpoint of the bases, bend the
        // midpoint of the bends.
        let mid_params = WingParams { idle_amp: Some(0.0), fold: Some(WingFold::Value(0.5)), ..Default::default() };
        let mid = wing_chain(&motion, 2, &mid_params, 3.7);
        assert!((mid[0] - 1.375).abs() <= 1e-12, "half-fold base {}", mid[0]);
        assert!((mid[1] - mid[0] + 0.91).abs() <= 1e-12, "half-fold bend {}", mid[1] - mid[0]);
        // Folded vs spread silhouettes differ structurally: with
        // unit segment lengths, tip reach is 1.08 folded vs 1.99
        // spread, and the per-joint bend differs by 2.18 rad.
        let reach = |segs: &[f64]| -> f64 {
            let (mut x, mut y) = (0.0f64, 0.0f64);
            for &a in segs {
                x += a.sin();
                y += a.cos();
            }
            x.hypot(y)
        };
        let folded2 = wing_chain(&motion, 2, &rest, 3.7);
        let (rf, rs) = (reach(&folded2), reach(&spread));
        assert!((rf - 1.08).abs() < 0.01, "folded tip reach {rf} (TS proof: 1.08)");
        assert!((rs - 1.99).abs() < 0.01, "spread tip reach {rs} (TS proof: 1.99)");
        assert!(rf < 0.6 * rs, "folded reach must be < 60% of spread reach");
        let bend_diff = ((folded2[1] - folded2[0]) - (spread[1] - spread[0])).abs();
        assert!((bend_diff - 2.18).abs() <= 1e-12, "per-joint bend differs by {bend_diff} (TS proof: 2.18)");
        // Any declared count is honoured.
        for n in [1usize, 2, 3, 5] {
            let p = WingParams { idle_amp: Some(0.0), fold: Some(WingFold::Value(0.4)), ..Default::default() };
            let segs = wing_chain(&motion, n, &p, 2.0);
            assert_eq!(segs.len(), n, "wing_chain must honour segments={n}");
            assert!(segs.iter().all(|a| a.is_finite()));
        }
    }

    #[test]
    fn wing_evaluator_flap_matches_ts_proof() {
        let phase = |tt: f64| tt * 4.0;
        let motion = WingMotion { speed: None, phase: Some(&phase) };
        // wingFlap at spread = flapAmp * sin(phase).
        let p_spread = WingParams { fold: Some(WingFold::Value(0.0)), ..Default::default() };
        let flap = wing_flap(&motion, &p_spread, 0.3);
        assert!((flap - 0.5 * 1.2f64.sin()).abs() <= 1e-12, "flap {flap}");
        // Exactly 0 at full fold, and exactly 0 with no phase driver.
        let p_fold = WingParams { fold: Some(WingFold::Value(1.0)), ..Default::default() };
        assert_eq!(wing_flap(&motion, &p_fold, 0.3), 0.0);
        assert_eq!(wing_flap(&WingMotion::default(), &p_spread, 0.3), 0.0);
        // wingPose.flap equals wingFlap to ±1e-12 (spec wording).
        let pose = wing_pose(&motion, 2, &WingParams { fold: Some(WingFold::Value(0.0)), idle_amp: Some(0.0), ..Default::default() }, 0.3);
        assert!((pose.flap - flap).abs() <= 1e-12, "pose.flap {} vs wingFlap {flap}", pose.flap);
        assert!((pose.bend - 0.18).abs() <= 1e-12);
        assert_eq!(pose.base, pose.segs[0]);
        // Per-segment flap lag: seg0 carries the full flap, seg1
        // trails by flapLag.
        let segs = wing_chain(&motion, 2, &WingParams { fold: Some(WingFold::Value(0.0)), idle_amp: Some(0.0), ..Default::default() }, 0.3);
        assert!((segs[0] - (1.5 + 0.5 * 1.2f64.sin())).abs() <= 1e-12, "seg0 {}", segs[0]);
        assert!((segs[1] - (1.68 + 0.5 * 0.6f64.sin())).abs() <= 1e-12, "seg1 {}", segs[1]);
    }

    #[test]
    fn wing_evaluator_speed_and_spring_fold_match_ts_proof() {
        // Constant full speed settles the speed response to
        // exactly 1, so an authored fold of 1 opens to exactly
        // 0.35 at speed = speedRef (spec: ±1e-9).
        let speed = |_tt: f64| 1.6;
        let motion = WingMotion { speed: Some(&speed), phase: None };
        let pose = wing_pose(&motion, 2, &WingParams { idle_amp: Some(0.0), ..Default::default() }, 30.0);
        assert!((pose.fold - 0.35).abs() <= 1e-9, "effFold at speedRef {} (TS proof: 0.35)", pose.fold);
        assert!((pose.spread - (1.0 - pose.fold)).abs() <= 1e-12);
        // The AppendageDef spring (BIRD_RIG wing: stiffness 0.6,
        // damping 0.35) settles to the same effFold.
        let via_def = wing_pose(
            &motion,
            2,
            &WingParams { idle_amp: Some(0.0), def: Some((0.6, 0.35)), ..Default::default() },
            60.0,
        );
        assert!((via_def.fold - 0.35).abs() <= 1e-6, "def-spring effFold {}", via_def.fold);
        // A fold FUNCTION is spring-tracked: a 1->0 step at t=1
        // is mid-flight shortly after, settled long after.
        let step = |tt: f64| if tt < 1.0 { 1.0 } else { 0.0 };
        let p_step = WingParams { idle_amp: Some(0.0), fold: Some(WingFold::Func(&step)), ..Default::default() };
        let mid = wing_pose(&WingMotion::default(), 2, &p_step, 1.1);
        assert!(mid.fold > 0.05 && mid.fold < 0.95, "spring-tracked fold mid-flight: {}", mid.fold);
        let settled = wing_pose(&WingMotion::default(), 2, &p_step, 30.0);
        assert!(settled.fold.abs() <= 1e-6, "spring-tracked fold settled: {}", settled.fold);
        // Bounded + deterministic under wild drivers (the TS
        // boundedness proof): finite, |a| <= 2.8, and shuffled
        // evaluation is bit-identical to in-order evaluation.
        let wild_speed = |tt: f64| 40.0 * (tt * 1.7).sin();
        let wild_phase = |tt: f64| tt * 5.1;
        let wild = WingMotion { speed: Some(&wild_speed), phase: Some(&wild_phase) };
        for tt in [-10.0, 0.0, 0.37, 5.0, 100.0, 10000.0] {
            for a in wing_chain(&wild, 4, &WingParams::default(), tt) {
                assert!(a.is_finite() && a.abs() <= 2.8, "wild chain angle {a} at t={tt}");
            }
            assert!(wing_flap(&wild, &WingParams::default(), tt).is_finite());
        }
        let det_speed = |tt: f64| if tt < 2.0 { 0.0 } else { 1.2 };
        let det_phase = |tt: f64| tt * 3.3;
        let det_fold = |tt: f64| if tt < 4.0 { 1.0 } else { 0.2 };
        let det_motion = WingMotion { speed: Some(&det_speed), phase: Some(&det_phase) };
        let det_params = WingParams { fold: Some(WingFold::Func(&det_fold)), ..Default::default() };
        let times = [7.3, 0.2, 100.0, 3.3, 1.05];
        let in_order: Vec<Vec<f64>> = times.iter().map(|&tt| wing_pose(&det_motion, 3, &det_params, tt).segs).collect();
        let mut shuffled: Vec<Vec<f64>> = vec![Vec::new(); times.len()];
        for i in [3usize, 0, 4, 1, 2] {
            shuffled[i] = wing_pose(&det_motion, 3, &det_params, times[i]).segs;
        }
        assert_eq!(in_order, shuffled, "seek determinism: shuffled evaluation must be bit-identical");
    }

    // ---------------------------------------------------------
    // Spec 1 — shared geometry (toon.ts wingChainGeom proofs).
    // ---------------------------------------------------------

    #[test]
    fn wing_geom_matches_ts_proof() {
        let look = pip_look();
        let st = FullState::default();
        // Fallback is the folded 2-segment family at BIRD
        // lengths: root (bx+46, top+96) for bx=256, top=520.
        let geom = wing_chain_geom(&look, &st, 256.0, 520.0, false);
        assert_eq!(geom.pts.len(), 3);
        assert_eq!(geom.pts[0], (302.0, 616.0));
        assert_eq!(geom.lens, vec![58.0, 46.0]);
        assert_eq!(geom.widths, vec![46.0, 34.0]);
        // Far chain: root (bx-38, top+84), lengths/widths x0.88.
        let far = wing_chain_geom(&look, &st, 256.0, 520.0, true);
        assert_eq!(far.pts[0], (218.0, 604.0));
        assert!((far.lens[0] - 58.0 * 0.88).abs() < 1e-3 && (far.lens[1] - 46.0 * 0.88).abs() < 1e-3);
        assert!((far.widths[0] - 46.0 * 0.88).abs() < 1e-3 && (far.widths[1] - 34.0 * 0.88).abs() < 1e-3);
        // Declared wingLens wins over the pattern, per segment.
        let mut look3 = pip_look();
        look3.wing_lens = vec![70.0, 55.5, 41.0];
        let mut st3 = FullState::default();
        st3.wing_segs = vec![1.5, 1.68, 1.86];
        let geom3 = wing_chain_geom(&look3, &st3, 256.0, 520.0, false);
        assert_eq!(geom3.lens, vec![70.0, 55.5, 41.0]);
        assert_eq!(geom3.pts.len(), 4);
        // wingSegs wins over wingA when both are set (geometry
        // is identical for different wingA).
        let mut sa = FullState::default();
        sa.wing_segs = vec![0.7, -1.3];
        sa.wing_a = 0.0;
        let mut sb = FullState::default();
        sb.wing_segs = vec![0.7, -1.3];
        sb.wing_a = 0.9;
        assert_eq!(
            wing_chain_geom(&look, &sa, 256.0, 520.0, false).pts,
            wing_chain_geom(&look, &sb, 256.0, 520.0, false).pts
        );
        // Declared counts 1, 2, 3, 5: pts = root + n per side.
        for n in [1usize, 2, 3, 5] {
            let mut s = FullState::default();
            s.wing_segs = vec![1.5; n];
            assert_eq!(wing_chain_geom(&look, &s, 256.0, 330.0, false).pts.len(), n + 1);
            assert_eq!(wing_chain_geom(&look, &s, 256.0, 330.0, true).lens.len(), n);
        }
    }

    // ---------------------------------------------------------
    // Spec 1 acceptance 2 + 4 — the painter: panel counts per
    // side (distinct paints per declared count), far wing behind
    // the body in the 0.72 shade, the reference bird at rest.
    // (Exact prim-level panel counts are asserted under the
    // gpupaint feature in gpupaint_states_wing.rs, where the
    // trace is available.)
    // ---------------------------------------------------------

    #[test]
    fn wing_painter_counts_layering_and_reference_bird() {
        // Distinct paints per declared count 1, 2, 3, 5 — each
        // count paints that many panels per side, so no two
        // counts can produce the same frame; all differ from
        // the wingless frame.
        let mut look = pip_look();
        look.wing_lens = vec![58.0, 46.0, 34.0, 24.0, 24.0];
        let chain = [1.5f32, 1.68, 1.86, 2.04, 2.22];
        let mut hashes: HashSet<u64> = HashSet::new();
        for n in [1usize, 2, 3, 5] {
            let mut st = FullState::default();
            st.wing_segs = chain[..n].to_vec();
            assert!(hashes.insert(paint_hash(&look, &st)), "count {n} paint collides with another count");
        }
        let mut wingless = pip_look();
        wingless.wing = false;
        let wingless_hash = paint_hash(&wingless, &FullState::default());
        assert!(!hashes.contains(&wingless_hash), "winged paints must differ from wingless");

        // Non-regression gate (acceptance 3 at paint level): a
        // look WITHOUT the wing flag paints byte-identically to
        // the pre-port painter even when the state carries wing
        // channels — the pinned neutral hash is the pre-port pin
        // from brow_tests (gpufirst-phase0 f4b358d).
        let mut st = FullState::default();
        st.wing_segs = vec![1.5, 1.68];
        st.wing_a = 0.8;
        assert_eq!(paint_hash(&wingless, &st), 0xd413603252323a7c, "wing state without the look flag must not paint");

        // wingSegs wins over wingA at paint level; the scalar
        // fallback still rotates on wingA.
        let look2 = pip_look();
        let mut s0 = FullState::default();
        s0.wing_segs = vec![0.7, -1.3];
        let mut s1 = FullState::default();
        s1.wing_segs = vec![0.7, -1.3];
        s1.wing_a = 0.9;
        assert_eq!(paint_hash(&look2, &s0), paint_hash(&look2, &s1), "wingSegs must win over wingA");
        let mut f0 = FullState::default();
        let mut f1 = FullState::default();
        f1.wing_a = 0.8;
        assert_ne!(paint_hash(&look2, &f0), paint_hash(&look2, &f1), "wingA fallback must move the wing");

        // Layering, pose segs [-1.2, -1.0] (panels sweep to the
        // figure's left, clear of both arms):
        // - the NEAR panel 0 centre carries the wing colour
        //   (near wing over the body);
        // - the FAR panel 0 centre carries the BODY colour
        //   (the far wing's own centre is occluded by the body:
        //   it paints behind it).
        let mut st = FullState::default();
        st.wing_segs = vec![-1.2, -1.0];
        assert_eq!(sample_stage(&look2, &st, 274.97, 436.5), rgba("#7FB6D9"), "near panel centre must be the wing colour");
        assert_eq!(sample_stage(&look2, &st, 241.79, 423.24), rgba("#E8734A"), "far panel centre must be occluded by the body");

        // Reference bird at rest (acceptance 4): the folded
        // two-panel wing (scalar fallback, wingA 0) —
        // - the near wing protrudes past the body edge: a point
        //   on panel 0 at 85% of the segment (past the bean's
        //   edge + its ink stroke at that height, and clear of
        //   the joint line — which crosses AT the joint, so the
        //   joint itself is shade-toned, not fill-coloured) is
        //   wing-coloured in the winged paint and transparent
        //   in the wingless;
        // - the far wing's shade crescent behind the opposite
        //   shoulder: at the bird's declared lens the crescent
        //   is only a few stage units wide (the TS acceptance
        //   wording is a visual read), so the exact-colour proof
        //   uses a long-lens declared look (wingLens [120, 100],
        //   same folded rest state), where the far panels reach
        //   well past the body edge: shadeOf(wingCol, 0.72)
        //   pixels appear in the winged paint and nowhere in
        //   the wingless paint.
        let rest = FullState::default();
        let (px, _, _, _, _) = paint_toon_full(&look2, &rest);
        assert!(count_col(&px, rgba("#7FB6D9")) > 500, "near wing panels must paint");
        assert_eq!(sample_stage(&look2, &rest, 348.8, 441.5), rgba("#7FB6D9"), "folded wing must protrude past the body edge");
        assert_eq!(sample_stage(&wingless, &rest, 348.8, 441.5), (0, 0, 0, 0), "no wing, no paint past the body edge");
        let mut long_look = pip_look();
        long_look.wing_lens = vec![120.0, 100.0];
        let far_shade = shade_rgba("#7FB6D9", 0.72);
        let (pxl, _, _, _, _) = paint_toon_full(&long_look, &rest);
        assert!(count_col(&pxl, far_shade) > 100, "far wing shade crescent must paint");
        let mut long_wingless = pip_look();
        long_wingless.wing = false;
        long_wingless.wing_lens = vec![120.0, 100.0];
        let (pxlw, _, _, _, _) = paint_toon_full(&long_wingless, &rest);
        assert_eq!(count_col(&pxlw, far_shade), 0, "wingless paint must have no far-shade pixels");
    }

    #[test]
    fn wing_anchor_envelope_includes_both_chains() {
        // An oversized declared wing (lens 400) reaches past
        // the canvas side bound: with the wing flag the anchor
        // must engage and pull the figure back; without it the
        // same state fits and the anchor stays zero.
        let mut look = pip_look();
        look.wing_lens = vec![400.0, 400.0];
        let mut st = FullState::default();
        st.wing_segs = vec![1.5, 1.68];
        let (ax, _) = canvas_anchor(&look, &st);
        assert!(ax < 0.0, "oversized wing must engage the anchor, got ax={ax}");
        let mut wingless = pip_look();
        wingless.wing = false;
        assert_eq!(canvas_anchor(&wingless, &st), (0.0, 0.0));
        // Ordinary wing states fit: no anchor, byte-identical
        // placement to a wingless figure.
        assert_eq!(canvas_anchor(&pip_look(), &FullState::default()), (0.0, 0.0));
    }
}
