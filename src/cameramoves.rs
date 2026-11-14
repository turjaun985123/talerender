//! Camera-move PRESETS (Wave 2, seat W25) — named, parameterised
//! camera moves that GENERATE key sets in the scene format's
//! existing camera-key shape. The renderers already evaluate keys;
//! this module only authors them.
//!
//! # The shape being generated (surveyed, not invented)
//!
//! Scene format §14.4 (`docs/scene-format.md`) puts keyframed
//! cameras in the scene file's `camera` block:
//!
//! ```jsonc
//! "camera": { "fovY": 44, "eye": [...], "target": [...],
//!   "keys": [ { "t": 0.0, "eye": [...], "target": [...], "fovY": 44 },
//!             { "t": 5.91, "eye": [...], "target": [...],
//!               "ease": "linear" } ] }
//! ```
//!
//! `src/scenefile.rs::eval_camera_keys` consumes exactly that: each
//! key is `{ t, eye: [f32; 3], target: [f32; 3], fovY?, ease? }`;
//! `fovY` falls back to the camera block's base `fovY` when absent;
//! before the first / after the last key the end key holds. One law
//! matters for generators: the renderer eases a segment with
//! **smootherstep by default**, and with linear interpolation only
//! when the DESTINATION key carries `"ease": "linear"` (the ease
//! field sits on the key the segment arrives at — the mirror image
//! of `anim::Key`, whose ease leaves its key; see the note in
//! `src/anim.rs` §"Tracks"). `CameraKey` below mirrors the JSON
//! fields one-for-one and `CameraKey::to_json` renders the exact
//! object the loader parses.
//!
//! # How a preset's easing reaches the screen
//!
//! The renderer's segment easing offers only two curves
//! (smootherstep, linear), so a preset sampled naively at its two
//! endpoints could express nothing else. The generators therefore
//! work in one of two modes, chosen per call:
//!
//! - **Native smootherstep** — profile `Ease::Smootherstep` with
//!   `samples <= 2`: exactly two keys, neither marked linear, and
//!   the renderer's own smootherstep reproduces the profile
//!   bit-for-bit. The default house move.
//! - **Sampled** — any other profile, or `samples > 2`: the eased
//!   curve is sampled at `samples` points and every key after the
//!   first is marked `linear`, so the renderer walks the samples
//!   piecewise-linearly. More samples = closer to the true curve;
//!   endpoints are exact at any count (see below). Orbit moves are
//!   always sampled (a 2-key orbit would chord across the circle).
//!
//! In both modes the first and last emitted keys are the requested
//! start/end framings VERBATIM — assigned, never interpolated — so
//! endpoints are bit-exact regardless of easing arithmetic (the
//! easing polynomials also satisfy f(0)=0, f(1)=1, but the
//! generators do not rely on that for the keys they emit).
//!
//! # Easing set and the `crate::anim` dependency
//!
//! `Ease` below mirrors `src/anim.rs`'s `Easing` variant-for-variant
//! with IDENTICAL polynomial forms (copied from `anim.rs::Easing::
//! eval`, which itself transcribes film.rs `eo`/`eio`/`eob` and the
//! scenefile smootherstep). At the time of writing `anim` is NOT yet
//! wired into `src/main.rs`'s module list, so this module stays
//! self-contained (std only) and compiles/tests standalone. When
//! the lead wires it, the one-line change is:
//!
//! ```ignore
//! pub use crate::anim::Easing as Ease; // replaces the local enum;
//!                                       // eval() bodies are identical
//! ```
//!
//! (plus deleting the local `Ease`/`impl Ease` block). No call site
//! changes: variant names and `eval` semantics already match.
//!
//! All functions are pure; time is in seconds; f32 throughout
//! (matching the loader, which evaluates keys in f32 after reading
//! `t` as f64 — preset times should stay in the range where f32
//! seconds are frame-exact, i.e. ordinary shot lengths).

// ---------------------------------------------------------------------
// The key shape
// ---------------------------------------------------------------------

/// One camera key, field-for-field the §14.4 JSON object:
/// `{ t, eye, target, fovY, ease? }`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CameraKey {
    /// Key time in seconds (shot-local, as the format's `time`).
    pub t: f32,
    /// Camera position, world metres.
    pub eye: [f32; 3],
    /// Look-at point, world metres.
    pub target: [f32; 3],
    /// Vertical field of view, degrees (the format's `fovY`).
    pub fov_y: f32,
    /// Emit `"ease": "linear"` on this key: the segment ARRIVING at
    /// this key interpolates linearly (scenefile law). Meaningless
    /// on the first key of a set (no segment arrives at it).
    pub linear: bool,
}

impl CameraKey {
    pub fn new(t: f32, eye: [f32; 3], target: [f32; 3], fov_y: f32) -> CameraKey {
        CameraKey { t, eye, target, fov_y, linear: false }
    }

    /// Render this key as the exact JSON object
    /// `eval_camera_keys` parses (numbers via f32's shortest
    /// round-trip formatting, which the loader's f64 read accepts).
    pub fn to_json(&self) -> String {
        let e = self.eye;
        let g = self.target;
        let ease = if self.linear { ", \"ease\": \"linear\"" } else { "" };
        format!(
            "{{\"t\": {}, \"eye\": [{}, {}, {}], \"target\": [{}, {}, {}], \"fovY\": {}{}}}",
            fnum(self.t),
            fnum(e[0]),
            fnum(e[1]),
            fnum(e[2]),
            fnum(g[0]),
            fnum(g[1]),
            fnum(g[2]),
            fnum(self.fov_y),
            ease
        )
    }
}

/// Render a whole key set as a JSON array for a scene file's
/// `camera.keys` (or a staging tool's intermediate file).
pub fn keys_to_json(keys: &[CameraKey]) -> String {
    let parts: Vec<String> = keys.iter().map(CameraKey::to_json).collect();
    format!("[{}]", parts.join(", "))
}

/// Shortest round-trip f32 formatting (Rust's `{}` for f32).
fn fnum(v: f32) -> String {
    format!("{v}")
}

/// A single framing: the value a move interpolates between.
/// Two `Framing`s are the complete authoring input for most
/// presets (start here, end there).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Framing {
    pub eye: [f32; 3],
    pub target: [f32; 3],
    pub fov_y: f32,
}

impl Framing {
    pub fn new(eye: [f32; 3], target: [f32; 3], fov_y: f32) -> Framing {
        Framing { eye, target, fov_y }
    }

    fn at(self, t: f32, linear: bool) -> CameraKey {
        CameraKey { t, eye: self.eye, target: self.target, fov_y: self.fov_y, linear }
    }

    /// Component-wise mix, `self + (other - self) * u` — the same
    /// mix form `eval_camera_keys` uses, so sampled keys sit on the
    /// same line the renderer would draw.
    fn mix(self, other: Framing, u: f32) -> Framing {
        let m = |a: [f32; 3], b: [f32; 3]| {
            [a[0] + (b[0] - a[0]) * u, a[1] + (b[1] - a[1]) * u, a[2] + (b[2] - a[2]) * u]
        };
        Framing {
            eye: m(self.eye, other.eye),
            target: m(self.target, other.target),
            fov_y: self.fov_y + (other.fov_y - self.fov_y) * u,
        }
    }
}

// ---------------------------------------------------------------------
// Easing (mirrors crate::anim::Easing — see the module docs)
// ---------------------------------------------------------------------

/// Easing profile for a move's parameter curve. Variant names and
/// polynomial forms are `anim::Easing`'s exactly; see that module
/// for the provenance of each curve (film.rs / motion.ts sources).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ease {
    Linear,
    Step,
    Smootherstep,
    EaseOutCubic,
    EaseInOutCubic,
    EaseOutBack,
}

impl Ease {
    /// The house default (the camK / scene-format default).
    pub const DEFAULT: Ease = Ease::Smootherstep;

    /// Evaluate at `u` in [0, 1] (clamped). Identical arithmetic
    /// to `anim::Easing::eval`.
    pub fn eval(self, u: f32) -> f32 {
        let u = u.clamp(0.0, 1.0);
        match self {
            Ease::Linear => u,
            Ease::Step => {
                if u >= 1.0 {
                    1.0
                } else {
                    0.0
                }
            }
            Ease::Smootherstep => u * u * u * (u * (u * 6.0 - 15.0) + 10.0),
            Ease::EaseOutCubic => {
                let v = 1.0 - u;
                1.0 - v * v * v
            }
            Ease::EaseInOutCubic => {
                if u < 0.5 {
                    4.0 * u * u * u
                } else {
                    let w = -2.0 * u + 2.0;
                    1.0 - (w * w * w) / 2.0
                }
            }
            Ease::EaseOutBack => {
                const C1: f32 = 1.70158;
                const C3: f32 = C1 + 1.0;
                let w = u - 1.0;
                1.0 + C3 * w * w * w + C1 * w * w
            }
        }
    }
}

// ---------------------------------------------------------------------
// Sampling engine (shared by the framing-to-framing presets)
// ---------------------------------------------------------------------

/// Sample the parametric framing curve `f` (parameter = EASED
/// progress in [0, 1]) into a key set over `[t0, t0 + duration]`.
///
/// `force_sampled` (orbit) skips the native-smootherstep shortcut.
/// The first/last keys are `f(0)` / `f(1)` verbatim; interior keys
/// are `f(ease.eval(raw))` marked linear.
fn sample_move(
    t0: f32,
    duration: f32,
    samples: usize,
    ease: Ease,
    force_sampled: bool,
    f: impl Fn(f32) -> Framing,
) -> Vec<CameraKey> {
    let n = samples.max(2);
    let start = f(0.0);
    let end = f(1.0);
    if !force_sampled && ease == Ease::Smootherstep && n == 2 {
        // Native mode: the renderer's own smootherstep IS the
        // profile; two unmarked keys reproduce it exactly.
        return vec![
            start.at(t0, false),
            end.at(t0 + duration, false),
        ];
    }
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let (t, framing) = if i == 0 {
            (t0, start)
        } else if i == n - 1 {
            (t0 + duration, end)
        } else {
            let raw = i as f32 / (n - 1) as f32;
            (t0 + duration * raw, f(ease.eval(raw)))
        };
        out.push(framing.at(t, i > 0));
    }
    out
}

// ---------------------------------------------------------------------
// Presets
// ---------------------------------------------------------------------

/// DOLLY — move between two full framings (eye, target and fovY
/// all interpolate). Direction is data: dolly-in and dolly-out are
/// this same move with the nearer framing last / first. See
/// `dolly_in` / `dolly_out`.
pub fn dolly(
    from: Framing,
    to: Framing,
    t0: f32,
    duration: f32,
    ease: Ease,
    samples: usize,
) -> Vec<CameraKey> {
    sample_move(t0, duration, samples, ease, false, move |u| from.mix(to, u))
}

/// Dolly-in: `from` is the wide framing, `to` the closer one.
pub fn dolly_in(
    from: Framing,
    to: Framing,
    t0: f32,
    duration: f32,
    ease: Ease,
    samples: usize,
) -> Vec<CameraKey> {
    dolly(from, to, t0, duration, ease, samples)
}

/// Dolly-out: `from` is the close framing, `to` the wider one.
pub fn dolly_out(
    from: Framing,
    to: Framing,
    t0: f32,
    duration: f32,
    ease: Ease,
    samples: usize,
) -> Vec<CameraKey> {
    dolly(from, to, t0, duration, ease, samples)
}

/// STATIC WITH DRIFT — the locked-off shot that breathes: eye
/// creeps toward the target by `drift` (a fraction of the
/// eye→target distance; 0.03 ≈ a 3% micro push-in), target and
/// fovY held. `drift` may be negative (micro pull-back). The end
/// eye is the exact f32 point `eye + (target - eye) * drift`.
pub fn static_with_drift(
    base: Framing,
    t0: f32,
    duration: f32,
    drift: f32,
    ease: Ease,
    samples: usize,
) -> Vec<CameraKey> {
    let end_eye = [
        base.eye[0] + (base.target[0] - base.eye[0]) * drift,
        base.eye[1] + (base.target[1] - base.eye[1]) * drift,
        base.eye[2] + (base.target[2] - base.eye[2]) * drift,
    ];
    let to = Framing { eye: end_eye, target: base.target, fov_y: base.fov_y };
    dolly(base, to, t0, duration, ease, samples)
}

/// PAN / TILT — reframe from a fixed position: the eye NEVER moves
/// (every key carries `from.eye` verbatim); only the target (the
/// pan/tilt direction) and fovY interpolate. Pass both framings
/// for readability; `to.eye` is ignored by construction.
pub fn pan_tilt(
    from: Framing,
    to: Framing,
    t0: f32,
    duration: f32,
    ease: Ease,
    samples: usize,
) -> Vec<CameraKey> {
    let to_fixed = Framing { eye: from.eye, target: to.target, fov_y: to.fov_y };
    sample_move(t0, duration, samples, ease, false, move |u| from.mix(to_fixed, u))
}

/// ORBIT — arc around a subject point. The target is `center` on
/// every key; the eye rides the circle of `radius` around it in
/// the XZ plane at height `center.y + height`, from `start_angle`
/// sweeping `sweep` radians (signed; positive = counter-clockwise
/// seen from above, +Y up, angle measured from +Z toward +X:
/// `eye = center + [r·sin(a), height, r·cos(a)]`).
///
/// Always sampled (minimum 8 keys, all linear): two keys would
/// make the renderer chord straight across the circle. The radius
/// is constant up to f32 sin/cos rounding (~1e-7 relative).
pub fn orbit(
    center: [f32; 3],
    radius: f32,
    height: f32,
    start_angle: f32,
    sweep: f32,
    fov_y: f32,
    t0: f32,
    duration: f32,
    ease: Ease,
    samples: usize,
) -> Vec<CameraKey> {
    let n = samples.max(8);
    let at = move |u: f32| {
        let a = start_angle + sweep * u;
        Framing {
            eye: [
                center[0] + radius * a.sin(),
                center[1] + height,
                center[2] + radius * a.cos(),
            ],
            target: center,
            fov_y,
        }
    };
    sample_move(t0, duration, n, ease, true, at)
}

/// TRACKING — the camera follows a moving subject. `subject` is
/// the subject's own key set, `(t, point)` pairs (e.g. a character
/// position track); the camera emits one key per subject key with
/// `target` = the subject point VERBATIM and `eye` = point +
/// `offset` (a constant world-space offset: behind/above/beside
/// the subject). Keys are marked linear so the renderer tracks the
/// subject's piecewise-linear path exactly as authored — easing a
/// follow would make the camera lag and surge against the
/// subject's own motion.
pub fn tracking(
    subject: &[(f32, [f32; 3])],
    offset: [f32; 3],
    fov_y: f32,
) -> Vec<CameraKey> {
    subject
        .iter()
        .enumerate()
        .map(|(i, &(t, p))| CameraKey {
            t,
            eye: [p[0] + offset[0], p[1] + offset[1], p[2] + offset[2]],
            target: p,
            fov_y,
            linear: i > 0,
        })
        .collect()
}

// ---------------------------------------------------------------------
// Seeded handheld
// ---------------------------------------------------------------------

/// Handheld parameters. Defaults are the subtle production values:
/// ±2 cm of eye wander, ±4 cm of target wander (≈ a gentle
/// rotational sway at conversational distances), one noise cell
/// per 0.8 s.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Handheld {
    /// Any u64; the same seed always yields the same move.
    pub seed: u64,
    /// Peak eye displacement, metres.
    pub amp_eye: f32,
    /// Peak target displacement, metres.
    pub amp_target: f32,
    /// Noise lattice spacing, seconds (lower = busier).
    pub cell: f32,
}

impl Default for Handheld {
    fn default() -> Self {
        Handheld { seed: 1, amp_eye: 0.02, amp_target: 0.04, cell: 0.8 }
    }
}

/// splitmix64 finalizer — integer-only, platform-identical.
fn mix64(mut z: u64) -> u64 {
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Hash (seed, axis, lattice cell) to a value in [-1, 1).
/// Uses the top 24 bits, exactly representable in f32.
fn lattice(seed: u64, axis: u64, cell: i64) -> f32 {
    let h = mix64(seed ^ axis.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ (cell as u64).wrapping_mul(0xD1B5_4A32_D192_ED03));
    let unit = ((h >> 40) as f32) / 16_777_216.0; // [0, 1)
    unit * 2.0 - 1.0
}

/// Smooth value noise for one axis at time `t`: smootherstep
/// interpolation between adjacent lattice hashes. Continuous with
/// zero slope at lattice points — no per-frame jitter steps, ever;
/// the value depends only on (seed, axis, t), never on frame
/// history, so any frame rate samples the same underlying move.
fn noise(seed: u64, axis: u64, t: f32, cell: f32) -> f32 {
    let x = t / cell;
    let k = x.floor();
    let frac = x - k;
    let a = lattice(seed, axis, k as i64);
    let b = lattice(seed, axis, k as i64 + 1);
    let u = frac * frac * frac * (frac * (frac * 6.0 - 15.0) + 10.0);
    a + (b - a) * u
}

/// Fade the handheld in and out over the first/last 15% of the
/// span so a shot starts and ends exactly on its authored framing
/// (clean cuts). 1.0 through the middle.
fn envelope(t: f32, t0: f32, duration: f32) -> f32 {
    if duration <= 0.0 {
        return 0.0;
    }
    let ramp = 0.15 * duration;
    let head = ((t - t0) / ramp).clamp(0.0, 1.0);
    let tail = ((t0 + duration - t) / ramp).clamp(0.0, 1.0);
    let u = head.min(tail);
    u * u * u * (u * (u * 6.0 - 15.0) + 10.0)
}

fn offset3(params: &Handheld, axis_base: u64, amp: f32, t: f32, env: f32) -> [f32; 3] {
    [
        noise(params.seed, axis_base, t, params.cell) * amp * env,
        noise(params.seed, axis_base + 1, t, params.cell) * amp * env,
        noise(params.seed, axis_base + 2, t, params.cell) * amp * env,
    ]
}

/// HANDHELD over an existing key set — perturb each key's eye and
/// target by the seeded noise at that key's time (axes 0–2 = eye,
/// 3–5 = target). Times, fovY and linear flags pass through
/// untouched; the envelope spans the key set's own first→last
/// times, so the first and last keys come out bit-identical to the
/// input. Works over any preset's output (handheld dolly, handheld
/// orbit, …) or a hand-authored set.
pub fn handheld_overlay(keys: &[CameraKey], params: Handheld) -> Vec<CameraKey> {
    if keys.is_empty() {
        return Vec::new();
    }
    let t0 = keys[0].t;
    let duration = keys[keys.len() - 1].t - t0;
    keys.iter()
        .map(|k| {
            let env = envelope(k.t, t0, duration);
            if env == 0.0 {
                return *k;
            }
            let de = offset3(&params, 0, params.amp_eye, k.t, env);
            let dt = offset3(&params, 3, params.amp_target, k.t, env);
            CameraKey {
                eye: [k.eye[0] + de[0], k.eye[1] + de[1], k.eye[2] + de[2]],
                target: [k.target[0] + dt[0], k.target[1] + dt[1], k.target[2] + dt[2]],
                ..*k
            }
        })
        .collect()
}

/// HANDHELD on a locked-off shot: a static framing sampled at
/// `samples` keys across `[t0, t0 + duration]`, with the seeded
/// handheld overlaid. Sample at (or above) the frame rate for the
/// smoothest ride; the noise itself is frame-rate independent.
pub fn handheld_static(
    base: Framing,
    t0: f32,
    duration: f32,
    samples: usize,
    params: Handheld,
) -> Vec<CameraKey> {
    let n = samples.max(2);
    let mut keys = Vec::with_capacity(n);
    for i in 0..n {
        let t = if i == 0 {
            t0
        } else if i == n - 1 {
            t0 + duration
        } else {
            t0 + duration * (i as f32 / (n - 1) as f32)
        };
        keys.push(base.at(t, i > 0));
    }
    handheld_overlay(&keys, params)
}

// ---------------------------------------------------------------------
// Tests (standalone: rustc --edition 2021 --test cameramoves.rs)
// ---------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn wide() -> Framing {
        Framing::new([0.0, 2.0, 8.0], [0.0, 1.0, 0.0], 44.0)
    }
    fn close() -> Framing {
        Framing::new([0.5, 1.8, 4.0], [0.0, 1.2, 0.0], 36.0)
    }

    fn assert_framing_eq(k: &CameraKey, f: Framing) {
        assert_eq!(k.eye, f.eye, "eye not bit-exact");
        assert_eq!(k.target, f.target, "target not bit-exact");
        assert_eq!(k.fov_y, f.fov_y, "fovY not bit-exact");
    }

    #[test]
    fn dolly_endpoints_bit_exact_all_profiles() {
        for ease in [
            Ease::Linear,
            Ease::Step,
            Ease::Smootherstep,
            Ease::EaseOutCubic,
            Ease::EaseInOutCubic,
            Ease::EaseOutBack,
        ] {
            for samples in [2usize, 3, 17] {
                let keys = dolly(wide(), close(), 1.5, 4.0, ease, samples);
                assert_eq!(keys.first().unwrap().t, 1.5);
                assert_eq!(keys.last().unwrap().t, 5.5);
                assert_framing_eq(keys.first().unwrap(), wide());
                assert_framing_eq(keys.last().unwrap(), close());
                assert!(keys.windows(2).all(|w| w[1].t > w[0].t), "times strictly increase");
            }
        }
    }

    #[test]
    fn dolly_native_smootherstep_is_two_unmarked_keys() {
        let keys = dolly(wide(), close(), 0.0, 5.91, Ease::Smootherstep, 2);
        assert_eq!(keys.len(), 2);
        assert!(!keys[0].linear && !keys[1].linear);
        // Sampled mode (any other profile) marks arrival keys linear.
        let lin = dolly(wide(), close(), 0.0, 5.91, Ease::Linear, 5);
        assert_eq!(lin.len(), 5);
        assert!(lin[1..].iter().all(|k| k.linear));
    }

    #[test]
    fn drift_moves_eye_toward_target_only() {
        let base = wide();
        let keys = static_with_drift(base, 0.0, 6.0, 0.05, Ease::Smootherstep, 2);
        assert_framing_eq(keys.first().unwrap(), base);
        let last = keys.last().unwrap();
        let want_eye = [
            base.eye[0] + (base.target[0] - base.eye[0]) * 0.05,
            base.eye[1] + (base.target[1] - base.eye[1]) * 0.05,
            base.eye[2] + (base.target[2] - base.eye[2]) * 0.05,
        ];
        assert_eq!(last.eye, want_eye);
        assert_eq!(last.target, base.target);
        assert_eq!(last.fov_y, base.fov_y);
        // And it really is closer to the target than the start.
        let d = |e: [f32; 3]| {
            ((e[0] - base.target[0]).powi(2)
                + (e[1] - base.target[1]).powi(2)
                + (e[2] - base.target[2]).powi(2))
            .sqrt()
        };
        assert!(d(last.eye) < d(base.eye));
    }

    #[test]
    fn pan_tilt_eye_never_moves() {
        let keys = pan_tilt(wide(), close(), 0.0, 3.0, Ease::EaseInOutCubic, 9);
        assert!(keys.iter().all(|k| k.eye == wide().eye), "eye verbatim on every key");
        assert_framing_eq(keys.first().unwrap(), wide());
        let last = keys.last().unwrap();
        assert_eq!(last.target, close().target);
        assert_eq!(last.fov_y, close().fov_y);
    }

    #[test]
    fn orbit_keeps_constant_radius_and_target() {
        let center = [1.0, 1.5, -2.0];
        let radius = 6.0f32;
        let keys = orbit(center, radius, 0.8, 0.3, 1.2, 40.0, 0.0, 4.0, Ease::Smootherstep, 24);
        assert!(keys.len() >= 8);
        for k in &keys {
            assert_eq!(k.target, center);
            assert_eq!(k.fov_y, 40.0);
            let dx = k.eye[0] - center[0];
            let dz = k.eye[2] - center[2];
            let r = (dx * dx + dz * dz).sqrt();
            assert!((r - radius).abs() < 1e-4, "radius {r} drifted from {radius}");
            assert_eq!(k.eye[1], center[1] + 0.8);
        }
        // Endpoints sit at the requested angles.
        let ang = |k: &CameraKey| (k.eye[0] - center[0]).atan2(k.eye[2] - center[2]);
        assert!((ang(keys.first().unwrap()) - 0.3).abs() < 1e-5);
        assert!((ang(keys.last().unwrap()) - 1.5).abs() < 1e-5);
        assert!(keys[1..].iter().all(|k| k.linear), "orbit keys are sampled/linear");
    }

    #[test]
    fn tracking_copies_subject_points_verbatim() {
        let subject = [
            (0.0, [0.0, 1.0, 0.0]),
            (2.0, [3.0, 1.0, -1.0]),
            (4.5, [3.5, 1.2, -4.0]),
        ];
        let offset = [-1.0, 0.6, 2.5];
        let keys = tracking(&subject, offset, 38.0);
        assert_eq!(keys.len(), 3);
        for (k, &(t, p)) in keys.iter().zip(subject.iter()) {
            assert_eq!(k.t, t);
            assert_eq!(k.target, p, "target is the subject point verbatim");
            assert_eq!(k.eye, [p[0] + offset[0], p[1] + offset[1], p[2] + offset[2]]);
            assert_eq!(k.fov_y, 38.0);
        }
    }

    #[test]
    fn handheld_same_seed_identical_different_seed_differs() {
        let p = Handheld { seed: 42, ..Handheld::default() };
        let a = handheld_static(wide(), 0.0, 6.0, 61, p);
        let b = handheld_static(wide(), 0.0, 6.0, 61, p);
        assert_eq!(a, b, "same seed -> identical key set");
        let c = handheld_static(wide(), 0.0, 6.0, 61, Handheld { seed: 43, ..p });
        assert_ne!(a, c, "different seed -> different key set");
        // Interior keys actually move (the move is not a no-op)...
        assert!(a.iter().any(|k| k.eye != wide().eye || k.target != wide().target));
        // ...but the shot starts and ends exactly on the framing.
        assert_framing_eq(a.first().unwrap(), wide());
        assert_framing_eq(a.last().unwrap(), wide());
    }

    #[test]
    fn handheld_amplitude_bounded_and_smooth() {
        let p = Handheld { seed: 7, amp_eye: 0.03, amp_target: 0.05, cell: 0.8 };
        let keys = handheld_static(wide(), 0.0, 6.0, 181, p); // 30 fps sampling
        for k in &keys {
            for i in 0..3 {
                assert!((k.eye[i] - wide().eye[i]).abs() <= p.amp_eye + 1e-6);
                assert!((k.target[i] - wide().target[i]).abs() <= p.amp_target + 1e-6);
            }
        }
        // Smoothness: value noise through a smootherstep lattice has
        // |slope| <= 2 * 1.875 / cell per axis; at 1/30 s spacing the
        // per-key step is bounded well under the amplitude itself.
        // Assert the empirical bound with generous headroom — the
        // point is NO jitter steps (a white-noise handheld would
        // jump by ~2*amp between adjacent keys).
        for w in keys.windows(2) {
            for i in 0..3 {
                assert!(
                    (w[1].eye[i] - w[0].eye[i]).abs() < p.amp_eye * 0.5,
                    "eye axis {i} stepped"
                );
                assert!(
                    (w[1].target[i] - w[0].target[i]).abs() < p.amp_target * 0.5,
                    "target axis {i} stepped"
                );
            }
        }
    }

    #[test]
    fn handheld_overlay_preserves_times_flags_and_endpoints() {
        let base = dolly(wide(), close(), 0.0, 5.0, Ease::EaseOutCubic, 11);
        let over = handheld_overlay(&base, Handheld::default());
        assert_eq!(over.len(), base.len());
        for (a, b) in base.iter().zip(over.iter()) {
            assert_eq!(a.t, b.t);
            assert_eq!(a.fov_y, b.fov_y);
            assert_eq!(a.linear, b.linear);
        }
        assert_eq!(over.first().unwrap().eye, base.first().unwrap().eye);
        assert_eq!(over.last().unwrap().eye, base.last().unwrap().eye);
    }

    #[test]
    fn json_matches_the_14_4_shape() {
        let keys = dolly(wide(), close(), 0.0, 5.91, Ease::Linear, 3);
        let first = keys[0].to_json();
        assert!(first.starts_with("{\"t\": 0, \"eye\": [0, 2, 8], \"target\": [0, 1, 0], \"fovY\": 44}"), "{first}");
        let mid = keys[1].to_json();
        assert!(mid.contains("\"ease\": \"linear\""), "{mid}");
        let arr = keys_to_json(&keys);
        assert!(arr.starts_with('[') && arr.ends_with(']'));
        // Native smootherstep keys carry no ease field at all.
        let native = dolly(wide(), close(), 0.0, 5.91, Ease::Smootherstep, 2);
        assert!(!native[1].to_json().contains("ease"));
    }

    #[test]
    fn ease_endpoints() {
        for e in [Ease::Linear, Ease::Smootherstep, Ease::EaseOutCubic, Ease::EaseInOutCubic] {
            assert_eq!(e.eval(0.0), 0.0);
            assert_eq!(e.eval(1.0), 1.0);
        }
        assert_eq!(Ease::Step.eval(0.5), 0.0);
        assert_eq!(Ease::Step.eval(1.0), 1.0);
    }
}
