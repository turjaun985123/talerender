//! Character animation channels (W12, Phase 4) — deterministic,
//! pure-function-of-time evaluators that FEED the character state
//! the painters already consume. This module defines no parallel
//! rig: every output is a value (or delta) for a field of
//! `toonfull::FullState` (src/toonfull.rs:140-161) — `blink`,
//! `look`, `head_tilt`, `head_turn`, `bob`, `lean`, `squash`,
//! `mood` — under the same names, units and sign conventions the
//! painter uses. It is deliberately NOT wired into film.rs /
//! scenefile.rs yet; the lead integrates it (see
//! docs/integration/CHARANIM-W12.md for the mapping table).
//!
//! Channels:
//!   (a) BLINK — smooth eyelid closure events, seeded by character
//!       id. Superset of film.rs's binary `Blinker`
//!       (src/film.rs:176-210, 0/1 for 0.12 s on a fixed interval
//!       table): same cadence family, but a shaped 0.12 s down /
//!       0.12 s up curve, because the painter already renders
//!       partial blink — eye height scales by `1.0 - blink * 0.6`
//!       and only `blink > 0.6` draws the closed-eye line
//!       (src/toonfull.rs:593,609).
//!   (b) IDLE — breathing / sway micro-motion. Life, not motion:
//!       amplitudes are a few stage pixels / fractions of a degree.
//!   (c) GAZE — head-turn + pupil targets toward a world point,
//!       clamped to the rig's authored ranges (below).
//!   (d) EXPRESSION — crossfade between two named expressions
//!       (the 17 `toonfull::Mood` names) over the continuous face
//!       parameters the painter derives from the mood.
//!
//! Determinism contract (per channel, detailed at each function):
//! every function is pure — no RNG state, no wall clock, no
//! globals; the only inputs are its arguments. Blink and
//! Expression use integer hashing + polynomial easing only and
//! are bit-exact on any platform. Idle and Gaze use `sin` /
//! `atan2`, so they are bit-repeatable for a given platform and
//! binary but may differ in the last ulp across libm versions —
//! exactly the caveat the existing film evaluators already carry
//! (film.rs `settle` / `talk` use sin/exp, src/film.rs:214-235).
//!
//! Rig ranges (no clamp constants exist in the painter — it is
//! linear in these fields — so the limits are the ranges the
//! production film actually authors, cited in CHARANIM-W12.md):
//!   head_turn  in [-0.5, 0.5]   (unitless; painter: face shift
//!               turn * 30 stage px, eye spacing scaled by
//!               1 - 0.18 * |turn|, src/toonfull.rs:507-545)
//!   head_tilt  in [-10, 10] deg for channel-driven values
//!               (staging authors -5..=+6 deg statically; bows go
//!               further and are staging's business, not a
//!               channel's; painter: to_radians about the neck
//!               pivot, src/toonfull.rs:330-341)
//!   look       in [-1, 1] per component (pupil offset
//!               look * [9, 8] stage px, src/toonfull.rs:603)
//!   blink      in [0, 1]

// ---------------------------------------------------------------------
// Deterministic seeding: splitmix64 over integer inputs only.
// ---------------------------------------------------------------------

/// splitmix64 finalizer — pure integer arithmetic, identical on
/// every platform and Rust version. This is the ONLY source of
/// variation in the seeded channels; there is no RNG state.
fn splitmix64(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^ (x >> 31)
}

/// A deterministic value in [0, 1) hashed from (id, index, salt).
/// Distinct salts keep the channels' hash streams independent, so
/// e.g. a character's blink pattern and idle phases never correlate
/// by construction.
fn seeded_unit(id: u64, index: u64, salt: u64) -> f64 {
    let mixed = splitmix64(
        id ^ splitmix64(index.wrapping_mul(0xD1B5_4A32_D192_ED03) ^ salt),
    );
    // Top 53 bits / 2^53: an exactly representable f64 in [0, 1).
    ((mixed >> 11) as f64) * (1.0 / 9_007_199_254_740_992.0)
}

const SALT_BLINK_FIRST: u64 = 0xB111_0001;
const SALT_BLINK_GAP: u64 = 0xB111_0002;
const SALT_IDLE_PHASE: u64 = 0x1D1E_0001;
const SALT_IDLE_PERIOD: u64 = 0x1D1E_0002;

/// The engine's house easing, smootherstep u^3(u(6u-15)+10),
/// transcribed from film.rs:130-133 (also scenefile.rs:765-767).
/// Polynomial only — bit-exact everywhere.
fn smootherstep(u: f64) -> f64 {
    let u = if u < 0.0 {
        0.0
    } else if u > 1.0 {
        1.0
    } else {
        u
    };
    u * u * u * (u * (u * 6.0 - 15.0) + 10.0)
}

fn clampf(v: f32, lo: f32, hi: f32) -> f32 {
    if v < lo {
        lo
    } else if v > hi {
        hi
    } else {
        v
    }
}

// ---------------------------------------------------------------------
// (a) BLINK
// ---------------------------------------------------------------------

/// Seconds the lid takes to close, and to reopen. The existing
/// binary Blinker holds the eyes shut for 0.12 s total
/// (src/film.rs:184-202); here 0.12 s is the DOWN time and 0.12 s
/// the UP time, so a full event lasts 0.24 s (~7 frames at 30 fps)
/// and passes through the partial-blink range the painter renders.
pub const BLINK_DOWN_S: f64 = 0.12;
pub const BLINK_UP_S: f64 = 0.12;
pub const BLINK_EVENT_S: f64 = BLINK_DOWN_S + BLINK_UP_S;

/// Start time of blink `n` for character `id`. Blink 0 starts at
/// 0.6 + 1.4*u in [0.6, 2.0) s; each later start follows the
/// previous by a gap of 2.6 + 1.8*u in [2.6, 4.4) s — the cadence
/// family of motion.ts's interval table [2.7, 3.9, 3.1, 4.4, 2.9,
/// 3.6] (film.rs:179-183) with per-blink hash jitter instead of a
/// fixed rotation, so two characters never fall into lockstep.
/// Accumulation is a fixed sequence of f64 additions — the same
/// additions in the same order on every call — so the schedule is
/// reproducible exactly, for any t, with no stored state.
fn blink_start(id: u64, n: u64) -> f64 {
    let mut s = 0.6 + 1.4 * seeded_unit(id, 0, SALT_BLINK_FIRST);
    let mut k = 0u64;
    while k < n {
        s += 2.6 + 1.8 * seeded_unit(id, k, SALT_BLINK_GAP);
        k += 1;
    }
    s
}

/// Eyelid closure for character `id` at time `t` (seconds), in
/// [0, 1]: 0 = eyes open, 1 = fully closed. Pure function of
/// (id, t): walks the deterministic blink schedule from t = 0
/// (O(number of blinks before t) — under 200 iterations for any
/// film-length t). The curve is smootherstep down over
/// BLINK_DOWN_S and smootherstep up over BLINK_UP_S, peaking at
/// exactly 1.0 at the event midpoint.
///
/// Determinism: integer hash + f64 add/mul + the smootherstep
/// polynomial. No transcendentals. Bit-exact across platforms.
///
/// Feeds `FullState.blink` (src/toonfull.rs:143) directly. The
/// film's per-character blink seeds (look registry, film.rs:434-
/// 435: seeds 3, 5, 7, 9, 11) map to ids by `seed as u64`.
pub fn blink_amount(id: u64, t: f64) -> f32 {
    if !(t >= 0.0) {
        return 0.0;
    }
    let mut n = 0u64;
    loop {
        let s = blink_start(id, n);
        if t < s {
            return 0.0;
        }
        if t <= s + BLINK_EVENT_S {
            let p = (t - s) / BLINK_EVENT_S;
            let v = if p < 0.5 {
                smootherstep(p * 2.0)
            } else {
                smootherstep((1.0 - p) * 2.0)
            };
            return v as f32;
        }
        n += 1;
        if n > 1_000_000 {
            return 0.0; // unreachable for any real t; loop guard
        }
    }
}

/// `blink_amount` quantized to `steps` levels (e.g. 8 → multiples
/// of 0.125). The film's paint cache keys on the exact blink value
/// (state_key, film.rs:2132-2151), so a smooth blink would paint
/// ~7 distinct cards per event; quantizing bounds that to at most
/// `steps` distinct values per event at a small fidelity cost.
/// The integrator chooses whether to pay it; the unquantized
/// channel above remains the source of truth.
pub fn blink_amount_q(id: u64, t: f64, steps: u32) -> f32 {
    if steps == 0 {
        return blink_amount(id, t);
    }
    let s = steps as f32;
    clampf((blink_amount(id, t) * s).round() / s, 0.0, 1.0)
}

// ---------------------------------------------------------------------
// (b) IDLE — breathing and sway. Amplitudes are stated in the
// units of the FullState fields they feed, and are deliberately
// SUBTLE: the stage is 512x1024 units (toonfull.rs:10-11), so a
// 1.5-unit bob is ~0.15% of the figure height.
// ---------------------------------------------------------------------

/// Bob amplitude, stage units (feeds `FullState.bob`, which the
/// painter adds to the body/head Y, toonfull.rs:320-328).
pub const IDLE_BOB_AMP: f32 = 1.5;
/// Lean (whole-figure sway) amplitude, degrees (feeds
/// `FullState.lean`, painter: to_radians, toonfull.rs:306).
pub const IDLE_LEAN_AMP_DEG: f32 = 0.35;
/// Head-tilt micro-drift amplitude, degrees (feeds
/// `FullState.head_tilt`).
pub const IDLE_TILT_AMP_DEG: f32 = 0.5;
/// Head-turn micro-drift amplitude, unitless (feeds
/// `FullState.head_turn`).
pub const IDLE_TURN_AMP: f32 = 0.012;
/// Squash breathing amplitude, unitless (feeds `FullState.squash`;
/// the painter scales head height by 1 + squash, toonfull.rs:334).
pub const IDLE_SQUASH_AMP: f32 = 0.004;

/// One instant of idle micro-motion: DELTAS to add to the staged
/// `FullState` fields of the same names.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IdleDelta {
    pub bob: f32,
    pub lean: f32,
    pub head_tilt: f32,
    pub head_turn: f32,
    pub squash: f32,
}

/// Idle micro-motion for character `id` at time `t`. Periods and
/// phases are per-character (hashed from the id): breathing
/// 3.6-4.8 s, sway 5.9-8.2 s, tilt drift 7.7-10.6 s, turn drift
/// 9.4-12.5 s — incommensurate families, so the combined motion
/// does not visibly loop on film timescales. The breath drives
/// bob and squash in phase (chest rises → head squash breathes),
/// as one sine, not two.
///
/// Determinism: pure in (id, t); uses `sin`, so same-platform
/// bit-repeatable, last-ulp variation across libm versions is
/// possible (the caveat film.rs's own evaluators carry).
pub fn idle_at(id: u64, t: f64) -> IdleDelta {
    let ph = |slot: u64| seeded_unit(id, slot, SALT_IDLE_PHASE) * std::f64::consts::TAU;
    let per = |slot: u64, lo: f64, span: f64| lo + span * seeded_unit(id, slot, SALT_IDLE_PERIOD);
    let breath = (std::f64::consts::TAU * t / per(0, 3.6, 1.2) + ph(0)).sin();
    let sway = (std::f64::consts::TAU * t / per(1, 5.9, 2.3) + ph(1)).sin();
    let tilt = (std::f64::consts::TAU * t / per(2, 7.7, 2.9) + ph(2)).sin();
    let turn = (std::f64::consts::TAU * t / per(3, 9.4, 3.1) + ph(3)).sin();
    IdleDelta {
        bob: (IDLE_BOB_AMP as f64 * breath) as f32,
        lean: (IDLE_LEAN_AMP_DEG as f64 * sway) as f32,
        head_tilt: (IDLE_TILT_AMP_DEG as f64 * tilt) as f32,
        head_turn: (IDLE_TURN_AMP as f64 * turn) as f32,
        squash: (IDLE_SQUASH_AMP as f64 * breath) as f32,
    }
}

// ---------------------------------------------------------------------
// (c) GAZE
// ---------------------------------------------------------------------

/// Authored-range limits (see the module header for evidence).
pub const HEAD_TURN_LIMIT: f32 = 0.5;
pub const HEAD_TILT_LIMIT_DEG: f32 = 10.0;
pub const LOOK_LIMIT: f32 = 1.0;

/// Angular scale of the gaze mapping: a target 60 degrees off the
/// facing direction asks for head_turn 1.0 before clamping, so
/// the ±0.5 limit corresponds to ±30 degrees of head yaw — the
/// range the film authors by hand (pose_track keys reach
/// -0.45..=+0.5, film.rs:1405,1512). Pupils saturate sooner
/// (±40 deg horizontal, ±25 deg vertical): eyes lead, head
/// follows, as in the staged poses (look [0.4, -0.3] with
/// head_turn 0.3, film.rs:1322-1327).
pub const GAZE_TURN_DEG_PER_UNIT: f32 = 60.0;
pub const GAZE_LOOK_X_DEG_PER_UNIT: f32 = 40.0;
pub const GAZE_LOOK_Y_DEG_PER_UNIT: f32 = 25.0;

/// Absolute gaze targets for the `FullState` fields of the same
/// names (NOT deltas — subtract the current values, or use
/// `gaze_delta`, to get deltas).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GazeTarget {
    pub head_turn: f32,
    pub head_tilt: f32,
    pub look: [f32; 2],
}

/// Where should the head and pupils point for a character whose
/// head is at `head` (world units), whose card faces `facing_yaw`
/// (radians, the face() convention yaw = atan2(dx, dz),
/// scenefile.rs:682-684), to look at `target` (world units)?
///
/// Mapping, in the painter's own terms:
/// - Horizontal angle → `head_turn` (clamped ±HEAD_TURN_LIMIT)
///   and the pupil `look[0]` (clamped ±1; pupils travel further
///   than the head, painter scale 9 stage px per unit,
///   toonfull.rs:603).
/// - Vertical angle → `look[1]` only (clamped ±1; painter scale
///   8 stage px per unit, sign: +look[1] moves the pupil DOWN in
///   stage space, so a target above gives a negative value).
/// - `head_tilt` is exactly 0.0, always: in this painter
///   head_tilt is an in-plane ROLL about the neck pivot
///   (toonfull.rs:330-341) — it reads as a quizzical head-cock
///   (pose_think, film.rs:1325), not as pitch toward a target.
///   Vertical gaze lives in `look[1]` (and the camera-driven
///   `cam_pitch`, which gaze must not author: it is derived from
///   the camera in finish_chars, film.rs:1364). Returning an
///   invented roll here would be a parallel-rig fiction.
///
/// Determinism: pure in its arguments; uses atan2/sin/cos, so
/// the libm last-ulp caveat of the Idle channel applies.
pub fn gaze_at(head: [f32; 3], facing_yaw: f32, target: [f32; 3]) -> GazeTarget {
    let dx = target[0] - head[0];
    let dy = target[1] - head[1];
    let dz = target[2] - head[2];
    let dist_h = (dx * dx + dz * dz).sqrt();
    // Degenerate case: target at the head's own vertical line —
    // no horizontal direction exists; look straight/down neutrally.
    if dist_h < 1e-6 {
        return GazeTarget { head_turn: 0.0, head_tilt: 0.0, look: [0.0, 0.0] };
    }
    let world_yaw = dx.atan2(dz);
    let mut rel = (world_yaw - facing_yaw) as f64;
    // Wrap to [-pi, pi]: the short way around, always.
    let tau = std::f64::consts::TAU;
    while rel > std::f64::consts::PI {
        rel -= tau;
    }
    while rel < -std::f64::consts::PI {
        rel += tau;
    }
    let rel_deg = rel.to_degrees() as f32;
    let elev_deg = (dy as f64).atan2(dist_h as f64).to_degrees() as f32;
    GazeTarget {
        head_turn: clampf(rel_deg / GAZE_TURN_DEG_PER_UNIT, -HEAD_TURN_LIMIT, HEAD_TURN_LIMIT),
        head_tilt: 0.0,
        look: [
            clampf(rel_deg / GAZE_LOOK_X_DEG_PER_UNIT, -LOOK_LIMIT, LOOK_LIMIT),
            clampf(-elev_deg / GAZE_LOOK_Y_DEG_PER_UNIT, -LOOK_LIMIT, LOOK_LIMIT),
        ],
    }
}

/// Gaze as DELTAS from the character's current field values —
/// the form staging code adds onto a posed `FullState`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GazeDelta {
    pub head_turn: f32,
    pub head_tilt: f32,
    pub look: [f32; 2],
}

pub fn gaze_delta(current_turn: f32, current_tilt: f32, current_look: [f32; 2], want: GazeTarget) -> GazeDelta {
    GazeDelta {
        head_turn: want.head_turn - current_turn,
        head_tilt: want.head_tilt - current_tilt,
        look: [want.look[0] - current_look[0], want.look[1] - current_look[1]],
    }
}

// ---------------------------------------------------------------------
// (d) EXPRESSION TRANSITIONS
// ---------------------------------------------------------------------

/// The expression vocabulary: the 17 moods the full painter
/// implements (`toonfull::Mood`, src/toonfull.rs:38-56), mirrored
/// 1:1 — same names, same strings (`Mood::from_str`,
/// toonfull.rs:59-79), same brow table and heavy-lid set
/// (toonfull.rs:76-102). This mirror exists so the module stays
/// self-contained for standalone testing; the lead's integration
/// converts Expr <-> Mood by name (they cannot drift silently:
/// the tests below pin every name and brow value).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Expr {
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
}

impl Expr {
    /// All 17 expressions, in the painter's declaration order.
    pub const ALL: [Expr; 17] = [
        Expr::Neutral,
        Expr::Smug,
        Expr::Happy,
        Expr::Laugh,
        Expr::Shock,
        Expr::Worried,
        Expr::Angry,
        Expr::Sad,
        Expr::Sly,
        Expr::Proud,
        Expr::Tired,
        Expr::Dizzy,
        Expr::Determined,
        Expr::Embarrassed,
        Expr::Awe,
        Expr::Suspicious,
        Expr::Relieved,
    ];

    /// Same strings as `Mood::from_str`; unknown → Neutral,
    /// exactly as the painter does (toonfull.rs:59-79).
    pub fn from_str(s: &str) -> Expr {
        match s {
            "smug" => Expr::Smug,
            "happy" => Expr::Happy,
            "laugh" => Expr::Laugh,
            "shock" => Expr::Shock,
            "worried" => Expr::Worried,
            "angry" => Expr::Angry,
            "sad" => Expr::Sad,
            "sly" => Expr::Sly,
            "proud" => Expr::Proud,
            "tired" => Expr::Tired,
            "dizzy" => Expr::Dizzy,
            "determined" => Expr::Determined,
            "embarrassed" => Expr::Embarrassed,
            "awe" => Expr::Awe,
            "suspicious" => Expr::Suspicious,
            "relieved" => Expr::Relieved,
            _ => Expr::Neutral,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Expr::Neutral => "neutral",
            Expr::Smug => "smug",
            Expr::Happy => "happy",
            Expr::Laugh => "laugh",
            Expr::Shock => "shock",
            Expr::Worried => "worried",
            Expr::Angry => "angry",
            Expr::Sad => "sad",
            Expr::Sly => "sly",
            Expr::Proud => "proud",
            Expr::Tired => "tired",
            Expr::Dizzy => "dizzy",
            Expr::Determined => "determined",
            Expr::Embarrassed => "embarrassed",
            Expr::Awe => "awe",
            Expr::Suspicious => "suspicious",
            Expr::Relieved => "relieved",
        }
    }

    /// drawFace brow table [lift, tilt], transcribed from
    /// `Mood::brow` (toonfull.rs:80-98). These are the painter's
    /// only CONTINUOUS per-expression parameters, which is why
    /// the crossfade below interpolates them.
    pub fn brow(self) -> (f32, f32) {
        match self {
            Expr::Neutral => (4.0, 0.0),
            Expr::Smug => (-8.0, 10.0),
            Expr::Happy => (8.0, 0.0),
            Expr::Laugh => (10.0, 0.0),
            Expr::Shock => (24.0, 0.0),
            Expr::Worried => (10.0, -16.0),
            Expr::Angry => (-10.0, 22.0),
            Expr::Sad => (8.0, -18.0),
            Expr::Sly => (-6.0, 12.0),
            Expr::Proud => (6.0, 4.0),
            Expr::Tired => (-2.0, 0.0),
            Expr::Dizzy => (14.0, 0.0),
            Expr::Determined => (-4.0, 16.0),
            Expr::Embarrassed => (12.0, -10.0),
            Expr::Awe => (26.0, 0.0),
            Expr::Suspicious => (-2.0, 6.0),
            Expr::Relieved => (14.0, -4.0),
        }
    }

    /// `Mood::heavy_lid` (toonfull.rs:100-102): these moods draw
    /// the lid line over the eye.
    pub fn heavy_lid(self) -> bool {
        matches!(self, Expr::Smug | Expr::Tired | Expr::Sly | Expr::Proud | Expr::Determined)
    }
}

/// One instant of an expression crossfade. `FullState.mood` is a
/// single discrete enum — the painter cannot blend two moods in
/// one card — so a transition is honestly represented as:
/// - `mix`: the smootherstepped blend factor in [0, 1], for a
///   composite-level crossfade of the two painted cards, and
/// - the interpolated continuous parameters (`brow_lift`,
///   `brow_tilt`, `heavy_lid` as a weight), for an integrator
///   who adds brow-override fields to the state, and
/// - `mood`: the dominant expression (from while mix < 0.5,
///   else to) for integrators who can only switch discretely.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ExpressionBlend {
    pub from: Expr,
    pub to: Expr,
    pub mix: f32,
    pub brow_lift: f32,
    pub brow_tilt: f32,
    pub heavy_lid: f32,
    pub mood: Expr,
}

/// Expression state at time `t` for a transition from `from` to
/// `to` starting at `t0` (seconds) over `dur` seconds, eased
/// with the house smootherstep. Before/at `t0` the result is
/// EXACTLY `from` (mix 0, its brow values bit-for-bit); at/after
/// `t0 + dur` it is EXACTLY `to` — the endpoints are returned
/// directly, never reconstructed by interpolation, so the
/// endpoint guarantee holds in floating point, not just in
/// exact arithmetic. `dur <= 0` is an instant switch at `t0`.
///
/// Determinism: polynomial easing + table lookups only —
/// bit-exact across platforms, pure in its arguments.
pub fn expression_at(from: Expr, to: Expr, t0: f64, dur: f64, t: f64) -> ExpressionBlend {
    let (fl, ft) = from.brow();
    let (tl, tt) = to.brow();
    let fh = if from.heavy_lid() { 1.0f32 } else { 0.0 };
    let th = if to.heavy_lid() { 1.0f32 } else { 0.0 };
    if dur <= 0.0 {
        return if t < t0 {
            ExpressionBlend { from, to, mix: 0.0, brow_lift: fl, brow_tilt: ft, heavy_lid: fh, mood: from }
        } else {
            ExpressionBlend { from, to, mix: 1.0, brow_lift: tl, brow_tilt: tt, heavy_lid: th, mood: to }
        };
    }
    if t <= t0 {
        return ExpressionBlend { from, to, mix: 0.0, brow_lift: fl, brow_tilt: ft, heavy_lid: fh, mood: from };
    }
    if t >= t0 + dur {
        return ExpressionBlend { from, to, mix: 1.0, brow_lift: tl, brow_tilt: tt, heavy_lid: th, mood: to };
    }
    let k = smootherstep((t - t0) / dur) as f32;
    ExpressionBlend {
        from,
        to,
        mix: k,
        brow_lift: fl + (tl - fl) * k,
        brow_tilt: ft + (tt - ft) * k,
        heavy_lid: fh + (th - fh) * k,
        mood: if k < 0.5 { from } else { to },
    }
}

// ---------------------------------------------------------------------
// Combined channel output + application to a mirrored state.
// ---------------------------------------------------------------------

/// Mirror of the channel-relevant `FullState` fields, same names
/// and units (toonfull.rs:140-161). Standalone stand-in so this
/// module compiles and tests without the crate; the lead maps
/// field-for-field (CHARANIM-W12.md).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CharStateMirror {
    pub mood: Expr,
    pub look: [f32; 2],
    pub blink: f32,
    pub talk: f32,
    pub lean: f32,
    pub bob: f32,
    pub squash: f32,
    pub head_tilt: f32,
    pub head_turn: f32,
    pub cam_pitch: f32,
}

impl Default for CharStateMirror {
    fn default() -> Self {
        // Mirrors FullState::default (toonfull.rs:162-185) for the
        // channel-relevant fields (mood Neutral, all channels 0).
        CharStateMirror {
            mood: Expr::Neutral,
            look: [0.0, 0.0],
            blink: 0.0,
            talk: 0.0,
            lean: 0.0,
            bob: 0.0,
            squash: 0.0,
            head_tilt: 0.0,
            head_turn: 0.0,
            cam_pitch: 0.0,
        }
    }
}

/// Everything the time-driven channels (blink + idle) produce
/// for one character at one instant. `blink` is an ABSOLUTE
/// value (eyelid state is not additive); the idle fields are
/// DELTAS added to the staged pose. Gaze and expression are
/// separate entry points above — they need scene context
/// (a target point; a from/to pair), not just (id, t).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CharStateDelta {
    pub blink: f32,
    pub bob: f32,
    pub lean: f32,
    pub head_tilt: f32,
    pub head_turn: f32,
    pub squash: f32,
}

/// Blink + idle for character `id` at time `t`. Pure in (id, t).
pub fn channels_at(id: u64, t: f64) -> CharStateDelta {
    let idle = idle_at(id, t);
    CharStateDelta {
        blink: blink_amount(id, t),
        bob: idle.bob,
        lean: idle.lean,
        head_tilt: idle.head_tilt,
        head_turn: idle.head_turn,
        squash: idle.squash,
    }
}

/// Apply a channel delta to a state: blink replaces, idle fields
/// add, and the channel-driven totals are clamped to the rig
/// ranges in the module header (blink [0,1], look untouched,
/// head_turn ±0.5, head_tilt ±10 deg). Staging that needs values
/// outside the channel ranges (e.g. a deep bow's head_tilt) sets
/// the field itself, exactly as film.rs does today — this helper
/// is only for channel output.
pub fn apply_channels(state: &mut CharStateMirror, d: &CharStateDelta) {
    state.blink = clampf(d.blink, 0.0, 1.0);
    state.bob += d.bob;
    state.lean += d.lean;
    state.head_tilt = clampf(state.head_tilt + d.head_tilt, -HEAD_TILT_LIMIT_DEG, HEAD_TILT_LIMIT_DEG);
    state.head_turn = clampf(state.head_turn + d.head_turn, -HEAD_TURN_LIMIT, HEAD_TURN_LIMIT);
    state.squash += d.squash;
}

/// Apply a gaze target to a state (absolute set of the gaze
/// fields, clamped to the rig ranges).
pub fn apply_gaze(state: &mut CharStateMirror, g: &GazeTarget) {
    state.head_turn = clampf(g.head_turn, -HEAD_TURN_LIMIT, HEAD_TURN_LIMIT);
    state.head_tilt = clampf(g.head_tilt, -HEAD_TILT_LIMIT_DEG, HEAD_TILT_LIMIT_DEG);
    state.look = [
        clampf(g.look[0], -LOOK_LIMIT, LOOK_LIMIT),
        clampf(g.look[1], -LOOK_LIMIT, LOOK_LIMIT),
    ];
}

// ---------------------------------------------------------------------
// Tests (standalone: rustc --test charanim.rs)
// ---------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blink_stays_in_unit_interval_dense_sweep() {
        for id in 0..8u64 {
            let mut saw_open = false;
            let mut saw_near_closed = false;
            let mut t = 0.0f64;
            while t < 120.0 {
                let b = blink_amount(id, t);
                assert!(b >= 0.0 && b <= 1.0, "blink {b} out of [0,1] id={id} t={t}");
                if b == 0.0 {
                    saw_open = true;
                }
                if b > 0.99 {
                    saw_near_closed = true;
                }
                t += 0.001;
            }
            assert!(saw_open, "id={id} never fully open in 120 s");
            assert!(saw_near_closed, "id={id} never blinks in 120 s");
        }
    }

    #[test]
    fn blink_repeatable_per_id_and_differs_across_ids() {
        // Repeatable: identical (id, t) → bit-identical value.
        for &(id, t) in &[(3u64, 7.31f64), (5, 0.83), (11, 59.97), (0, 3.3)] {
            assert_eq!(blink_amount(id, t).to_bits(), blink_amount(id, t).to_bits());
        }
        // Differs across ids: the 120 s traces of ids 3 and 7
        // disagree somewhere (they share no schedule by hash).
        let mut differ = false;
        let mut t = 0.0f64;
        while t < 120.0 {
            if blink_amount(3, t) != blink_amount(7, t) {
                differ = true;
                break;
            }
            t += 0.005;
        }
        assert!(differ, "ids 3 and 7 blink identically — hash seeding broken");
    }

    #[test]
    fn blink_cadence_is_human_plausible() {
        // Count blink events (rising crossings of 0.5) in 120 s:
        // gaps are in [2.6, 4.4) s, so expect roughly 27-46 events.
        let mut events = 0u32;
        let mut prev = blink_amount(3, 0.0);
        let mut t = 0.01f64;
        while t < 120.0 {
            let b = blink_amount(3, t);
            if prev <= 0.5 && b > 0.5 {
                events += 1;
            }
            prev = b;
            t += 0.01;
        }
        assert!(events >= 25 && events <= 48, "blink events in 120 s: {events}");
    }

    #[test]
    fn blink_quantized_is_bounded_and_stepwise() {
        let mut t = 0.0f64;
        while t < 30.0 {
            let q = blink_amount_q(9, t, 8);
            assert!(q >= 0.0 && q <= 1.0);
            assert!((q * 8.0 - (q * 8.0).round()).abs() < 1e-6, "q={q} not on the 1/8 grid");
            t += 0.002;
        }
    }

    #[test]
    fn idle_is_subtle_bounded_and_repeatable() {
        let mut t = 0.0f64;
        while t < 300.0 {
            let d = idle_at(42, t);
            assert!(d.bob.abs() <= IDLE_BOB_AMP + 1e-6, "bob {}", d.bob);
            assert!(d.lean.abs() <= IDLE_LEAN_AMP_DEG + 1e-6, "lean {}", d.lean);
            assert!(d.head_tilt.abs() <= IDLE_TILT_AMP_DEG + 1e-6, "tilt {}", d.head_tilt);
            assert!(d.head_turn.abs() <= IDLE_TURN_AMP + 1e-6, "turn {}", d.head_turn);
            assert!(d.squash.abs() <= IDLE_SQUASH_AMP + 1e-7, "squash {}", d.squash);
            t += 0.05;
        }
        assert_eq!(idle_at(42, 12.34), idle_at(42, 12.34));
        assert_ne!(idle_at(42, 12.34), idle_at(43, 12.34));
    }

    #[test]
    fn gaze_straight_ahead_and_above() {
        // Facing +Z (yaw 0), head at origin.
        let g = gaze_at([0.0, 1.6, 0.0], 0.0, [0.0, 1.6, 5.0]);
        assert_eq!(g.head_turn, 0.0);
        assert_eq!(g.look, [0.0, 0.0]);
        assert_eq!(g.head_tilt, 0.0);
        // Target above: pupils up = negative look[1] (stage y down).
        let g = gaze_at([0.0, 1.6, 0.0], 0.0, [0.0, 3.6, 5.0]);
        assert!(g.look[1] < 0.0, "look[1]={} for a target above", g.look[1]);
        // Target to the character's left at 30 deg: turn at limit.
        let g = gaze_at([0.0, 1.6, 0.0], 0.0, [2.5, 1.6, 4.33]);
        assert!(g.head_turn > 0.4 && g.head_turn <= HEAD_TURN_LIMIT, "turn={}", g.head_turn);
    }

    #[test]
    fn gaze_clamps_at_limits() {
        // Target directly behind: wrapped angle is ±180 deg —
        // everything must saturate, never exceed the rig ranges.
        let g = gaze_at([0.0, 1.6, 0.0], 0.0, [0.0, 1.6, -5.0]);
        assert!(g.head_turn.abs() <= HEAD_TURN_LIMIT, "turn={}", g.head_turn);
        assert!(g.look[0].abs() <= LOOK_LIMIT && g.look[1].abs() <= LOOK_LIMIT);
        // Target far below-left, and facing turned 90 deg away.
        let g = gaze_at([1.0, 2.0, 1.0], std::f32::consts::FRAC_PI_2, [-50.0, -30.0, -50.0]);
        assert!(g.head_turn.abs() <= HEAD_TURN_LIMIT, "turn={}", g.head_turn);
        assert!(g.look[0].abs() <= LOOK_LIMIT && g.look[1].abs() <= LOOK_LIMIT, "look={:?}", g.look);
        // Applying any gaze keeps the state inside the ranges.
        let mut st = CharStateMirror::default();
        apply_gaze(&mut st, &g);
        assert!(st.head_turn.abs() <= HEAD_TURN_LIMIT);
        assert!(st.look[0].abs() <= LOOK_LIMIT && st.look[1].abs() <= LOOK_LIMIT);
    }

    #[test]
    fn gaze_delta_is_target_minus_current() {
        let want = gaze_at([0.0, 1.6, 0.0], 0.0, [3.0, 1.6, 4.0]);
        let d = gaze_delta(0.1, 0.0, [0.05, 0.0], want);
        assert!((d.head_turn - (want.head_turn - 0.1)).abs() < 1e-7);
        assert!((d.look[0] - (want.look[0] - 0.05)).abs() < 1e-7);
        assert_eq!(d.head_tilt, 0.0);
    }

    #[test]
    fn expression_endpoints_are_exact() {
        let pairs = [
            (Expr::Neutral, Expr::Angry),
            (Expr::Determined, Expr::Relieved),
            (Expr::Smug, Expr::Shock),
            (Expr::Sad, Expr::Neutral),
        ];
        for &(a, b) in &pairs {
            let at0 = expression_at(a, b, 2.0, 0.8, 2.0);
            assert_eq!(at0.mix, 0.0);
            assert_eq!(at0.brow_lift.to_bits(), a.brow().0.to_bits());
            assert_eq!(at0.brow_tilt.to_bits(), a.brow().1.to_bits());
            assert_eq!(at0.mood, a);
            let before = expression_at(a, b, 2.0, 0.8, 1.0);
            assert_eq!(before.mix, 0.0);
            let at1 = expression_at(a, b, 2.0, 0.8, 2.8);
            assert_eq!(at1.mix, 1.0);
            assert_eq!(at1.brow_lift.to_bits(), b.brow().0.to_bits());
            assert_eq!(at1.brow_tilt.to_bits(), b.brow().1.to_bits());
            assert_eq!(at1.mood, b);
            let after = expression_at(a, b, 2.0, 0.8, 99.0);
            assert_eq!(after.mix, 1.0);
        }
    }

    #[test]
    fn expression_midpoint_and_instant_switch() {
        // smootherstep(0.5) = 0.5 exactly, so the midpoint brow is
        // the arithmetic mean of the endpoint brows.
        let mid = expression_at(Expr::Neutral, Expr::Angry, 0.0, 2.0, 1.0);
        assert_eq!(mid.mix, 0.5);
        assert!((mid.brow_lift - (-3.0)).abs() < 1e-6, "lift={}", mid.brow_lift); // (4 + -10)/2
        assert!((mid.brow_tilt - 11.0).abs() < 1e-6, "tilt={}", mid.brow_tilt); // (0 + 22)/2
        assert_eq!(mid.mood, Expr::Angry); // k >= 0.5 → dominant = to
        // Zero duration: instant switch at t0.
        let sw = expression_at(Expr::Happy, Expr::Sad, 5.0, 0.0, 5.0);
        assert_eq!(sw.mix, 1.0);
        assert_eq!(sw.mood, Expr::Sad);
        let sw = expression_at(Expr::Happy, Expr::Sad, 5.0, 0.0, 4.99);
        assert_eq!(sw.mix, 0.0);
        assert_eq!(sw.mood, Expr::Happy);
    }

    #[test]
    fn expression_vocabulary_matches_the_painter() {
        // The 17 names, in order, round-tripping through strings —
        // the same strings Mood::from_str accepts (toonfull.rs:59-79).
        let names = [
            "neutral", "smug", "happy", "laugh", "shock", "worried", "angry", "sad", "sly",
            "proud", "tired", "dizzy", "determined", "embarrassed", "awe", "suspicious",
            "relieved",
        ];
        assert_eq!(Expr::ALL.len(), 17);
        for (i, e) in Expr::ALL.iter().enumerate() {
            assert_eq!(e.as_str(), names[i]);
            assert_eq!(Expr::from_str(names[i]), *e);
        }
        assert_eq!(Expr::from_str("bogus"), Expr::Neutral);
        // Spot-pin the brow table against toonfull.rs:80-98.
        assert_eq!(Expr::Neutral.brow(), (4.0, 0.0));
        assert_eq!(Expr::Angry.brow(), (-10.0, 22.0));
        assert_eq!(Expr::Awe.brow(), (26.0, 0.0));
        assert_eq!(Expr::Relieved.brow(), (14.0, -4.0));
        assert!(Expr::Determined.heavy_lid());
        assert!(!Expr::Happy.heavy_lid());
    }

    #[test]
    fn apply_channels_keeps_state_in_rig_ranges() {
        let mut st = CharStateMirror { head_turn: 0.49, head_tilt: 9.5, ..Default::default() };
        let mut t = 0.0f64;
        while t < 60.0 {
            apply_channels(&mut st, &channels_at(3, t));
            assert!(st.blink >= 0.0 && st.blink <= 1.0);
            assert!(st.head_turn.abs() <= HEAD_TURN_LIMIT + 1e-6, "turn={}", st.head_turn);
            assert!(st.head_tilt.abs() <= HEAD_TILT_LIMIT_DEG + 1e-6, "tilt={}", st.head_tilt);
            t += 0.033;
        }
    }
}
