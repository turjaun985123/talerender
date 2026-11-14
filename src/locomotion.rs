//! Procedural locomotion (W24, Phase 4) — gait evaluation as pure
//! functions of phase, feeding the character channels the painters
//! already consume. No parallel rig: the outputs are values for
//! `toonfull::FullState` fields (`phase`, `bob`, `arm_l`, `arm_r`,
//! plus the `legs` Walk/Run choice) under the same names, units and
//! conventions as the existing staging — see the mapping below and
//! docs/integration/LOCOMOTION-W24.md. NOT wired into film.rs /
//! scenefile.rs yet; the lead integrates it.
//!
//! WHY THIS MODULE EXISTS — the backwards-walk lesson.
//! The sister project (Tall Tale engine) proved arithmetically in
//! 2026-11 that a hand-staged walk cycle can be BACKWARDS: the
//! planted foot drifted toward the direction of travel, the arms
//! swung in-phase with the same-side leg, and the bob was inverted.
//! Nothing about a wrong cycle looks wrong in a single still; only
//! the arithmetic shows it. This module therefore defines gait in a
//! travel-axis frame with the invariants stated as laws, and its
//! tests assert them numerically:
//!
//!   L1  A planted foot is world-stationary: during stance the
//!       foot moves backward relative to the body at exactly the
//!       ground speed, so body velocity + foot-relative velocity
//!       = 0. (A sinusoidal foot path violates this — it slides
//!       except at one instant. Stance here is LINEAR in phase.)
//!   L2  The legs are exactly anti-phase: right = left + PI.
//!   L3  Each arm is counter-phase to the SAME-side leg (and so
//!       in-phase with the opposite leg): shoulder swing is a
//!       negative multiple of the same-side foot's fore-aft
//!       position.
//!   L4  The body bob has exactly two height minima per cycle,
//!       located at double support. This is DERIVED below from the
//!       support geometry, not assumed.
//!
//! SURVEY — what the engine does today (cited, not invented):
//!   * Phase comes from travelled distance: `walk_phase(dist,
//!     stride) = dist / stride * PI` (src/film.rs:169-171), so PI
//!     of phase per step of `stride` metres (film call sites use
//!     stride 0.58-0.62: film.rs:1423,1625,1702,1737,1754,1842,
//!     1908). This module's `phase_for_distance` keeps that law
//!     exactly; only the phase ORIGIN differs (see below).
//!   * Bob today: `walk_bob(phase, amp) = amp * (1 - |sin phase|)`
//!     (film.rs:172-174), amp 4.0-5.0 stage px, added DOWNWARD into
//!     the body Y (toonfull.rs:324: `by = ... + st.bob`, stage Y
//!     is down). Its maxima sit at the painter's passing pose.
//!     This module's bob is derived from support geometry and its
//!     maxima sit at double support instead — a quarter-step
//!     shift. Adopting it CHANGES shipped pixels; it is a staging
//!     decision for the lead/user with an A/B, not a silent swap
//!     (LOCOMOTION-W24.md §3).
//!   * The painter's in-card cycle (toonfull.rs:358-364):
//!     foot offset `a = sin(phase + (PI if right)) * amp` (amp
//!     0.55 Walk / 1.0 Run), lift `max(0, -cos(phase + ...)) *
//!     (34 Walk / 70 Run)` stage px. The airborne foot in card
//!     space moves from its forward extreme to its backward
//!     extreme, so the card cycle reads correctly for travel
//!     toward -x_stage and reversed for +x_stage: facing is a
//!     staging sign, exactly the sister project's bug class. This
//!     module works in the travel axis (+x = direction of travel)
//!     where the laws hold unconditionally; staging flips by
//!     facing, as it already owns facing via the card yaw.
//!   * Arms during the film's walks are STATIC pairs (e.g.
//!     arm_r (16,-6), arm_l (10,6) at film.rs:1438-1439,1759-1760,
//!     1913-1914 — degrees (shoulder, elbow), painter convention
//!     toonfull.rs:482-491). No phase-driven arm swing exists in
//!     the film today; L3 is new behaviour this module adds.
//!   * `FullState.phase: f32` (radians), `bob: f32` (stage px),
//!     `arm_l/arm_r: (f32, f32)` (degrees), `legs: Legs::{Stand,
//!     Walk, Run, Sit, Jump}` — toonfull.rs:104-110,147-152;
//!     scene JSON keys `phase`, `bob`, `armL`, `armR`, `legs`
//!     (scenefile.rs:647-669), which are also W15's clip channels
//!     (`phase`, `bob`, `armL.0/.1`, `armR.0/.1`, `legs` —
//!     FORMAT-EXT-CLIPS-W15.md §3). W15 §5.3 rules gait drivers
//!     must NOT migrate into clip key data ("the gait derivation
//!     stays code") — this module is that code.
//!
//! PHASE CONVENTION (module): phase 0 = LEFT foot strike (the
//! start of left stance). One full cycle (2 PI) = two steps.
//! Leg-local cycle fraction w in [0,1): left w = frac(phase/2PI),
//! right w = frac(phase/2PI + 0.5) — L2 by construction.
//! The painter's phase origin is its passing pose; to drive
//! `FullState.phase` from module phase, subtract PI * duty_factor
//! (module left mid-stance lands on painter 0). See the note.
//!
//! GAIT GEOMETRY (the derivation L1/L4 rest on):
//! Let step_length = L (m/step), cycle length S = 2L, ground
//! speed v, duty factor beta (fraction of the cycle each foot is
//! in stance; walk ~0.62, run ~0.36; beta < 0.5 <=> a flight
//! interval exists). During one leg's stance the body advances
//! beta * S, so the foot sweeps from +A to -A relative to the
//! hip with A = beta * L, at constant relative velocity
//! -2A / (beta * T) = -v exactly (T = cycle period) — L1.
//! During swing the foot returns from -A to +A (smootherstep
//! profile, zero end slopes) while lifting on a sin^2 bump of
//! height step_height.
//! BOB: model the support leg as straight, length Lleg, foot at
//! fore-aft offset x from the hip: hip height = sqrt(Lleg^2 - x^2)
//! ~ Lleg (1 - x^2 / 2 Lleg^2) — the hip is highest as it passes
//! over the planted foot (x = 0, mid-stance) and drops as |x|
//! grows. With two feet down, the hip rides the support leg
//! nearer vertical (the smaller |x| dominates: the other leg is
//! already reaching/lifting). So drop(phase) = (min over stance
//! legs of |x| / A)^2, in units of bob_amp. In a walk (beta >
//! 0.5) that function peaks twice per cycle, inside the two
//! double-support windows, and is 0 at each mid-stance: two
//! height minima per cycle at double support — derived. During
//! a run's flight interval neither foot supports; the body is
//! ballistic, so drop continues on a parabola from the toe-off
//! value (1) through -flight_rise at mid-flight back to 1 at the
//! next strike. (Honest limit: this carries the walking
//! inverted-pendulum law across the blend; the spring-mass
//! inversion of true running — COM lowest at mid-stance — is NOT
//! modelled, matching the painter, whose Run is an amplitude
//! variant of the same cycle, toonfull.rs:359-363.)
//!
//! DETERMINISM: pure functions of their arguments — no state, no
//! RNG, no clock. `sin` appears only in the swing lift, the same
//! last-ulp caveat class as film.rs's existing sin evaluators.
//! Internal math is f64 (the film's motion formulas are f64
//! throughout); outputs are cast to f32 at the FullState
//! boundary, like film.rs's staging does (`ph as f32`).

use std::f64::consts::PI;

/// The film's smootherstep (film.rs:130 `smoother`), restated so
/// this module stays self-contained: 6u^5 - 15u^4 + 10u^3 on
/// clamped u, zero slope at both ends.
fn smoother(u: f64) -> f64 {
    let u = u.clamp(0.0, 1.0);
    u * u * u * (u * (u * 6.0 - 15.0) + 10.0)
}

fn lerp(a: f64, b: f64, u: f64) -> f64 {
    a + (b - a) * u
}

/// Gait authoring parameters. Distances in metres, angles in
/// degrees, `bob_amp` in stage px (FullState.bob units — the film
/// authors 4.0-5.0). Scene/clip-facing names are given in
/// LOCOMOTION-W24.md §4 (camelCase, W15 style).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GaitParams {
    /// Ground speed, m/s. Cadence is derived: steps/s =
    /// speed / step_length; phase rate = PI * speed / step_length.
    pub speed: f64,
    /// Metres per step — the `stride` argument of film.rs's
    /// `walk_phase` (film call sites: 0.58-0.62).
    pub step_length: f64,
    /// Duty factor beta: fraction of the cycle a foot spends in
    /// stance. Walk ~0.62, run ~0.36. beta < 0.5 <=> flight.
    pub duty_factor: f64,
    /// Peak swing clearance, metres.
    pub step_height: f64,
    /// Bob amplitude, stage px (FullState.bob; film: 4.0-5.0).
    pub bob_amp: f64,
    /// Shoulder swing amplitude, degrees (arm.0 channel).
    pub arm_amp: f64,
    /// Resting arm pose the swing oscillates around, as
    /// ((armL.0, armL.1), (armR.0, armR.1)) degrees — defaults are
    /// the film's repeated walk arms (10,6) / (16,-6).
    pub arm_base: ((f64, f64), (f64, f64)),
    /// How far the drop fraction goes below 0 at mid-flight
    /// (run only; body rises above its neutral line). 0 = walk.
    pub flight_rise: f64,
}

impl GaitParams {
    /// The film's walk shape at a given ground speed: stride
    /// 0.62 (film.rs:1423,1625), bob amp 5.0 (film.rs:1424,1628,
    /// 1740), duty 0.62 (textbook walking duty; the film does not
    /// author one today — its cycle is the painter's sinusoid).
    pub fn walk(speed: f64) -> GaitParams {
        GaitParams {
            speed,
            step_length: 0.62,
            duty_factor: 0.62,
            step_height: 0.055,
            bob_amp: 5.0,
            arm_amp: 16.0,
            arm_base: ((10.0, 6.0), (16.0, -6.0)),
            flight_rise: 0.0,
        }
    }

    /// Run shape: longer stride, duty below 0.5 (flight exists),
    /// higher clearance and arm swing, ballistic rise.
    pub fn run(speed: f64) -> GaitParams {
        GaitParams {
            speed,
            step_length: 0.85,
            duty_factor: 0.36,
            step_height: 0.13,
            bob_amp: 6.0,
            arm_amp: 26.0,
            arm_base: ((10.0, 6.0), (16.0, -6.0)),
            flight_rise: 0.55,
        }
    }

    /// Walk<->run blend: every shape field lerps walk->run by
    /// `run_factor` in [0,1] at this params' speed. The duty
    /// factor crossing 0.5 is where flight appears and where
    /// staging should flip `Legs::Walk` -> `Legs::Run`
    /// (`is_run`). Blending start/stop/turn INTO a gait is the
    /// staging's job (crossfade params over the ease window);
    /// this is the steady-state shape at one blend point.
    pub fn blended(&self, run_factor: f64) -> GaitParams {
        let k = run_factor.clamp(0.0, 1.0);
        let w = GaitParams::walk(self.speed);
        let r = GaitParams::run(self.speed);
        GaitParams {
            speed: self.speed,
            step_length: lerp(w.step_length, r.step_length, k),
            duty_factor: lerp(w.duty_factor, r.duty_factor, k),
            step_height: lerp(w.step_height, r.step_height, k),
            bob_amp: lerp(w.bob_amp, r.bob_amp, k),
            arm_amp: lerp(w.arm_amp, r.arm_amp, k),
            arm_base: w.arm_base,
            flight_rise: lerp(w.flight_rise, r.flight_rise, k),
        }
    }

    /// True when the duty factor leaves a flight interval —
    /// staging maps this to `Legs::Run`, else `Legs::Walk`.
    pub fn is_run(&self) -> bool {
        self.duty_factor < 0.5
    }

    /// Cycle length S = 2 steps, metres.
    pub fn cycle_length(&self) -> f64 {
        2.0 * self.step_length
    }

    /// Steps per second at the authored speed.
    pub fn cadence(&self) -> f64 {
        self.speed / self.step_length
    }

    /// Half-sweep A: the foot strikes A ahead of the hip and
    /// toes off A behind it, A = duty_factor * step_length.
    fn sweep(&self) -> f64 {
        self.duty_factor.clamp(0.05, 0.95) * self.step_length
    }

    fn duty(&self) -> f64 {
        self.duty_factor.clamp(0.05, 0.95)
    }
}

/// One foot's state at an instant, in the BODY frame: `x` is the
/// fore-aft offset from the hip in metres along the travel axis
/// (+x = direction of travel), `height` is clearance above the
/// ground in metres (0 while planted), `stance` is the support
/// flag. World foot position = body position + x (travel axis);
/// during stance that sum is constant — law L1.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FootState {
    pub x: f32,
    pub height: f32,
    pub stance: bool,
}

/// Everything one frame of staging needs from the gait, in
/// FullState channel units. `arm_l`/`arm_r` are (shoulder, elbow)
/// degree pairs for `FullState.arm_l`/`arm_r` (W15 channels
/// `armL.0/.1`, `armR.0/.1`); `bob` is stage px for
/// `FullState.bob`; convert `phase` per the header note before
/// assigning `FullState.phase`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GaitState {
    /// Module phase, radians in [0, 2 PI). 0 = left strike.
    pub phase: f32,
    pub foot_l: FootState,
    pub foot_r: FootState,
    /// Downward body offset, stage px (painter adds it to body Y,
    /// toonfull.rs:324). Negative only in run flight (body risen).
    pub bob: f32,
    /// The raw support-drop fraction bob was scaled from
    /// (diagnostics / tests; 1 = full drop, 0 = passing over the
    /// support foot, < 0 = airborne rise).
    pub support_drop: f32,
    pub arm_l: (f32, f32),
    pub arm_r: (f32, f32),
}

/// Leg-local cycle fraction w in [0,1) for a leg at module
/// phase. Right leg is offset by exactly half a cycle — law L2.
fn leg_w(phase: f64, right: bool) -> f64 {
    let u = (phase / (2.0 * PI)).rem_euclid(1.0);
    if right {
        (u + 0.5).rem_euclid(1.0)
    } else {
        u
    }
}

/// Foot trajectory for one leg (f64 core). Stance is LINEAR
/// (law L1); swing returns on the smootherstep profile with a
/// sin^2 lift bump — positions and lift are continuous at both
/// boundaries, lift slope is 0 at strike and toe-off.
fn foot_f64(p: &GaitParams, phase: f64, right: bool) -> (f64, f64, bool) {
    let beta = p.duty();
    let a = p.sweep();
    let w = leg_w(phase, right);
    if w < beta {
        // Stance: +A -> -A at constant rate. Relative velocity
        // = -2A / (beta * T) = -speed exactly (see header), so
        // the planted foot's world velocity is 0.
        (a * (1.0 - 2.0 * w / beta), 0.0, true)
    } else {
        let s = (w - beta) / (1.0 - beta);
        let x = a * (-1.0 + 2.0 * smoother(s));
        let lift = p.step_height * (PI * s).sin().powi(2);
        (x, lift, false)
    }
}

/// One foot at `phase` (module convention), body frame.
pub fn foot_at(p: &GaitParams, phase: f64, right: bool) -> FootState {
    let (x, h, stance) = foot_f64(p, phase, right);
    FootState {
        x: x as f32,
        height: h as f32,
        stance,
    }
}

/// Support-drop fraction at `phase` — the derivation in the
/// header, in code: the stance leg nearer vertical (smaller
/// |x|/A) sets the hip height, drop = (|x|/A)^2 of that leg.
/// If neither foot is in stance (run flight), the ballistic
/// parabola through the flight interval.
fn support_drop(p: &GaitParams, phase: f64) -> f64 {
    let beta = p.duty();
    let a = p.sweep();
    let mut best: Option<f64> = None;
    for right in [false, true] {
        let (x, _, stance) = foot_f64(p, phase, right);
        if stance {
            let d = (x / a).powi(2);
            best = Some(best.map_or(d, |b: f64| b.min(d)));
        }
    }
    if let Some(d) = best {
        return d;
    }
    // Flight: cycle fraction lies in (beta, 0.5) or
    // (beta + 0.5, 1). Parabola: 1 at both ends (strike /
    // toe-off drop), -flight_rise at mid-flight.
    let u = (phase / (2.0 * PI)).rem_euclid(1.0);
    let s = if u < 0.5 {
        (u - beta) / (0.5 - beta)
    } else {
        (u - beta - 0.5) / (0.5 - beta)
    };
    let q = 1.0 - 2.0 * s.clamp(0.0, 1.0);
    q * q - p.flight_rise * (1.0 - q * q)
}

/// Shoulder swing (degrees) for one arm: counter-phase to the
/// SAME-side leg (law L3). The leg's fore-aft fraction x/A is
/// the swing signal — foot fully forward (+1) => arm fully back
/// (-arm_amp), and vice versa. Because the legs are anti-phase,
/// each arm automatically ends up in-phase with the opposite
/// leg, as in a real gait.
fn arm_swing(p: &GaitParams, phase: f64, right: bool) -> f64 {
    let (x, _, _) = foot_f64(p, phase, right);
    -p.arm_amp * (x / p.sweep())
}

/// Evaluate the full gait at module `phase` (radians, any real —
/// wrapped internally). Pure: same inputs, same outputs.
pub fn eval(p: &GaitParams, phase: f64) -> GaitState {
    let drop = support_drop(p, phase);
    let sw_l = arm_swing(p, phase, false);
    let sw_r = arm_swing(p, phase, true);
    // Elbow coupling: the forearm echoes the shoulder swing at
    // reduced gain and opposite sign (the elbow opens as the arm
    // comes forward). Small and linear; staging may ignore the
    // elbow component and keep its authored elbow value.
    let elbow = |sw: f64| -0.35 * sw;
    GaitState {
        phase: phase.rem_euclid(2.0 * PI) as f32,
        foot_l: foot_at(p, phase, false),
        foot_r: foot_at(p, phase, true),
        bob: (p.bob_amp * drop) as f32,
        support_drop: drop as f32,
        arm_l: (
            (p.arm_base.0 .0 + sw_l) as f32,
            (p.arm_base.0 .1 + elbow(sw_l)) as f32,
        ),
        arm_r: (
            (p.arm_base.1 .0 + sw_r) as f32,
            (p.arm_base.1 .1 + elbow(sw_r)) as f32,
        ),
    }
}

/// Phase at time `t` (seconds) for steady travel at the params'
/// speed, phase 0 at t = 0 (left strike at the origin).
pub fn phase_at_time(p: &GaitParams, t: f64) -> f64 {
    (PI * p.cadence() * t).rem_euclid(2.0 * PI)
}

/// Evaluate at time `t` — `eval(params, phase_at_time(params, t))`.
pub fn eval_at_time(p: &GaitParams, t: f64) -> GaitState {
    eval(p, phase_at_time(p, t))
}

/// Phase for a travelled distance — film.rs's law, kept exactly:
/// `walk_phase(dist, stride) = dist / stride * PI`
/// (film.rs:169-171). Only the origin differs from film's: here
/// dist = 0 is a left foot strike; film's origin is the painter's
/// passing pose (offset by PI * duty_factor, see header).
pub fn phase_for_distance(p: &GaitParams, dist: f64) -> f64 {
    (dist / p.step_length * PI).rem_euclid(2.0 * PI)
}

/// The phase to hand the painter (`FullState.phase`) for a
/// module phase: shift so module left mid-stance (phase =
/// PI * duty_factor) lands on the painter's passing pose
/// (phase 0 in toonfull.rs:358-364).
pub fn painter_phase(p: &GaitParams, phase: f64) -> f64 {
    phase - PI * p.duty()
}

#[cfg(test)]
mod tests {
    use super::*;

    const TAU: f64 = 2.0 * PI;

    fn walk() -> GaitParams {
        GaitParams::walk(1.4)
    }

    /// Sample n phases across one full cycle.
    fn cycle(n: usize) -> Vec<f64> {
        (0..n).map(|i| i as f64 / n as f64 * TAU).collect()
    }

    // L2: legs exactly anti-phase — the right foot at phase is
    // the left foot at phase + PI, bit for bit (same code path).
    #[test]
    fn legs_exactly_antiphase() {
        let p = walk();
        for ph in cycle(720) {
            let l = foot_at(&p, ph, false);
            let r = foot_at(&p, ph, true);
            let l_shifted = foot_at(&p, ph + PI, false);
            assert_eq!(r, l_shifted, "phase {ph}");
            // And the two feet are never in the same fore-aft
            // position mid-cycle (they only coincide at x=0
            // crossings of measure zero in this sampling).
            let _ = l;
        }
    }

    // L1: the planted foot is world-stationary. Numeric check
    // across the whole stance window of BOTH feet: world
    // velocity = speed + d(foot.x)/dt must be ~0 (central
    // difference in time; stance is linear, so this is exact
    // up to float error).
    #[test]
    fn planted_foot_world_velocity_zero() {
        let p = walk();
        let h = 1e-4; // seconds
        let mut stance_samples = 0;
        for i in 0..4000 {
            let t = i as f64 * 0.002;
            let st = eval_at_time(&p, t);
            for (foot, right) in [(st.foot_l, false), (st.foot_r, true)] {
                if !foot.stance {
                    continue;
                }
                stance_samples += 1;
                let xp = foot_at(&p, phase_at_time(&p, t + h), right).x as f64;
                let xm = foot_at(&p, phase_at_time(&p, t - h), right).x as f64;
                // Stay inside the same stance window: if the
                // stencil straddles a boundary the derivative is
                // meaningless; skip those samples.
                let fp = foot_at(&p, phase_at_time(&p, t + h), right);
                let fm = foot_at(&p, phase_at_time(&p, t - h), right);
                if !(fp.stance && fm.stance) {
                    continue;
                }
                let rel_vel = (xp - xm) / (2.0 * h);
                let world_vel = p.speed + rel_vel;
                assert!(
                    world_vel.abs() < 1e-3,
                    "t={t} right={right}: planted foot world velocity {world_vel} m/s"
                );
            }
        }
        assert!(stance_samples > 1000, "test must actually sample stance");
    }

    // L1, closed-form cross-check: stance sweep rate is exactly
    // the ground speed. Over one full stance of the left foot
    // the foot's relative travel equals speed * stance duration.
    #[test]
    fn stance_sweep_matches_ground_speed() {
        let p = walk();
        let beta = p.duty_factor;
        let t_cycle = p.cycle_length() / p.speed;
        let stance_dur = beta * t_cycle;
        let x0 = foot_at(&p, 0.0, false).x as f64; // strike: +A
        let x1 = foot_at(&p, beta * TAU - 1e-9, false).x as f64; // toe-off: -A
        let swept = x0 - x1;
        assert!((swept - p.speed * stance_dur).abs() < 1e-6);
        assert!((x0 - p.sweep()).abs() < 1e-6);
    }

    // L3: arm swing is counter-phase to the same-side leg and
    // in-phase with the opposite leg. Pearson correlation over a
    // full cycle between shoulder swing and foot fore-aft
    // fraction: same side ~ -1, opposite side ~ +1.
    #[test]
    fn arm_counterphase_same_side_leg() {
        let p = walk();
        let phs = cycle(1440);
        let corr = |a: &[f64], b: &[f64]| -> f64 {
            let ma = a.iter().sum::<f64>() / a.len() as f64;
            let mb = b.iter().sum::<f64>() / b.len() as f64;
            let (mut cov, mut va, mut vb) = (0.0, 0.0, 0.0);
            for i in 0..a.len() {
                cov += (a[i] - ma) * (b[i] - mb);
                va += (a[i] - ma).powi(2);
                vb += (b[i] - mb).powi(2);
            }
            cov / (va.sqrt() * vb.sqrt())
        };
        let a = p.sweep();
        let leg_l: Vec<f64> = phs.iter().map(|&ph| foot_at(&p, ph, false).x as f64 / a).collect();
        let leg_r: Vec<f64> = phs.iter().map(|&ph| foot_at(&p, ph, true).x as f64 / a).collect();
        let arm_l: Vec<f64> = phs.iter().map(|&ph| arm_swing(&p, ph, false)).collect();
        let arm_r: Vec<f64> = phs.iter().map(|&ph| arm_swing(&p, ph, true)).collect();
        assert!(corr(&arm_l, &leg_l) < -0.99, "left arm vs left leg");
        assert!(corr(&arm_r, &leg_r) < -0.99, "right arm vs right leg");
        // Opposite side: strongly positive but NOT ~1, and that
        // is correct gait, not an error: the PHASE offset between
        // the legs is exactly PI (legs_exactly_antiphase), but
        // the foot-position waveform is not half-cycle
        // antisymmetric when duty != 0.5 (stance is longer than
        // swing, so x(w + 0.5) != -x(w)). Measured: +0.836 at
        // duty 0.62. The studio law is the same-side sign; the
        // opposite side follows from L2 + L3, not from position
        // symmetry.
        assert!(corr(&arm_l, &leg_r) > 0.8, "left arm vs right leg");
        assert!(corr(&arm_r, &leg_l) > 0.8, "right arm vs left leg");
    }

    // L4: bob frequency is exactly 2x the stride frequency —
    // the drop function is PI-periodic (steps are symmetric) —
    // and its two per-cycle maxima sit inside the double-support
    // windows while mid-stance is at zero drop (body highest).
    #[test]
    fn bob_two_minima_per_cycle_at_double_support() {
        let p = walk();
        // Exact PI periodicity.
        for ph in cycle(1440) {
            let d0 = support_drop(&p, ph);
            let d1 = support_drop(&p, ph + PI);
            assert!((d0 - d1).abs() < 1e-12, "phase {ph}: {d0} vs {d1}");
        }
        // Mid-stance of the left leg (w = beta/2): support foot
        // under the hip -> no drop.
        let mid_stance = p.duty_factor * 0.5 * TAU;
        assert!(support_drop(&p, mid_stance).abs() < 1e-9);
        // Count local maxima of drop over one cycle: exactly 2.
        let n = 7200;
        let drops: Vec<f64> = (0..n)
            .map(|i| support_drop(&p, i as f64 / n as f64 * TAU))
            .collect();
        let mut maxima = Vec::new();
        for i in 0..n {
            let prev = drops[(i + n - 1) % n];
            let next = drops[(i + 1) % n];
            if drops[i] > prev && drops[i] >= next {
                maxima.push(i as f64 / n as f64); // cycle fraction
            }
        }
        assert_eq!(maxima.len(), 2, "drop maxima at {maxima:?}");
        // Each maximum lies in a double-support window:
        // cycle fractions [0, beta-0.5) and [0.5, beta).
        let beta = p.duty_factor;
        for m in &maxima {
            let in_ds = *m < beta - 0.5 || (*m >= 0.5 && *m < beta);
            assert!(in_ds, "bob maximum at cycle fraction {m} not in double support");
        }
        // And the maxima are real drops, well above mid-stance.
        for m in &maxima {
            assert!(support_drop(&p, m * TAU) > 0.5);
        }
    }

    // Swing shape: foot leaves and lands at height 0, peaks near
    // step_height mid-swing, and position is continuous across
    // the stance/swing boundaries (no teleports).
    #[test]
    fn swing_continuous_and_lifted() {
        let p = walk();
        let beta = p.duty_factor;
        let eps = 1e-6;
        for right in [false, true] {
            let off = if right { PI } else { 0.0 };
            // Toe-off boundary (leg-local w = beta).
            let before = foot_at(&p, off + beta * TAU - eps, right);
            let after = foot_at(&p, off + beta * TAU + eps, right);
            assert!((before.x - after.x).abs() < 1e-3);
            assert!(before.stance && !after.stance);
            // Strike boundary (w wraps to 0).
            let before = foot_at(&p, off + TAU - eps, right);
            let after = foot_at(&p, off + eps, right);
            assert!((before.x - after.x).abs() < 1e-3);
            assert!(!before.stance && after.stance);
            // Mid-swing peak lift.
            let mid = foot_at(&p, off + (beta + 1.0) / 2.0 * TAU, right);
            assert!((mid.height as f64 - p.step_height).abs() < 1e-5);
            assert_eq!(mid.height, mid.height.max(0.0));
        }
        // No foot ever goes below the ground.
        for ph in cycle(1440) {
            assert!(foot_at(&p, ph, false).height >= 0.0);
            assert!(foot_at(&p, ph, true).height >= 0.0);
        }
    }

    // Walk<->run blend: duty factor falls monotonically, flight
    // exists only once duty < 0.5, walk always has a foot down,
    // run clearance exceeds walk clearance.
    #[test]
    fn walk_run_blend_duty_factor() {
        let base = GaitParams::walk(2.8);
        let mut prev_duty = f64::INFINITY;
        for i in 0..=10 {
            let g = base.blended(i as f64 / 10.0);
            assert!(g.duty_factor < prev_duty, "duty must fall monotonically");
            prev_duty = g.duty_factor;
            assert_eq!(g.is_run(), g.duty_factor < 0.5);
            // Support coverage across the cycle.
            let mut any_flight = false;
            let mut any_double = false;
            for ph in cycle(720) {
                let l = foot_at(&g, ph, false).stance;
                let r = foot_at(&g, ph, true).stance;
                if !l && !r {
                    any_flight = true;
                }
                if l && r {
                    any_double = true;
                }
            }
            if g.is_run() {
                assert!(any_flight, "run must have a flight interval");
                assert!(!any_double, "run must not have double support");
            } else {
                assert!(!any_flight, "walk must always have a foot down");
                assert!(any_double, "walk must have double support");
            }
        }
        assert!(GaitParams::run(2.8).step_height > GaitParams::walk(1.4).step_height);
        assert!(GaitParams::run(2.8).duty_factor < GaitParams::walk(1.4).duty_factor);
        // Endpoints of the blend are the pure shapes.
        assert_eq!(base.blended(0.0).duty_factor, GaitParams::walk(2.8).duty_factor);
        assert_eq!(base.blended(1.0).duty_factor, GaitParams::run(2.8).duty_factor);
    }

    // The phase law matches film.rs's walk_phase exactly:
    // PI of phase per step_length travelled.
    #[test]
    fn phase_law_matches_film_walk_phase() {
        let p = walk();
        for dist in [0.0, 0.31, 0.62, 1.24, 3.7, 5.96] {
            let expect = (dist / p.step_length * PI).rem_euclid(TAU);
            let got = phase_for_distance(&p, dist);
            assert!((expect - got).abs() < 1e-12);
        }
        // Cadence consistency: phase_at_time advances PI per step
        // duration (step_length / speed seconds).
        let t_step = p.step_length / p.speed;
        let adv = phase_at_time(&p, t_step);
        assert!((adv - PI).abs() < 1e-9);
    }

    // Painter phase conversion: module left mid-stance maps to
    // painter phase 0.
    #[test]
    fn painter_phase_origin() {
        let p = walk();
        let mid_stance = PI * p.duty();
        assert!(painter_phase(&p, mid_stance).abs() < 1e-12);
    }
}
