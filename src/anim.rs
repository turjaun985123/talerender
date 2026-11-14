//! anim — reusable keyframe animation evaluation (Wave 2, seat W11;
//! the animation-layer foundation the external audit's Phase 4 asks
//! for). OPT-IN: nothing in the renderer calls this module yet. The
//! lead registers it with a one-line `mod anim;` in main.rs when
//! adoption starts; docs/integration/ANIM-W11.md is the adoption map.
//!
//! Design law: a frame is a pure function of time. `sample(t)`
//! depends only on the track's keys and on `t` — no wall clock, no
//! RNG, no mutable state, no dependence on previously sampled
//! frames or on any cache. Tracks are immutable after construction,
//! so any frame can be evaluated in any order, in parallel, or
//! resumed mid-film, with bit-identical results. That is the
//! engine's frame(t) contract (the film driver already renders by
//! random access; this module gives that contract a reusable home).
//!
//! # Float discipline (why sampling is bit-stable)
//!
//! - f32 everywhere. The film/scene sources transcribe the TS
//!   originals in f64 and cast at the boundary; this layer is the
//!   f32-native home for scene-format data, whose stored values are
//!   already exact f32 (scene-format spec §11 data law).
//! - Easing curves are polynomials written with explicit
//!   multiply/add in a fixed Horner order, copied from the forms in
//!   film.rs / scenefile.rs. No `powi`/`powf`, no transcendentals
//!   anywhere in a sampling path. The single root in the module is
//!   `f32::sqrt` in Quat normalisation — IEEE-754 correctly
//!   rounded, hence deterministic.
//! - Interpolation is always `a + (b - a) * u` (film.rs `lerp`'s
//!   exact form), never `a * (1 - u) + b * u`.
//! - Wrap modes use `f32::rem_euclid`; the IEEE remainder is
//!   exact, so wrapping introduces no rounding of its own.
//! - Key sorting uses `f32::total_cmp` (a total order), so even
//!   duplicate key times order deterministically. Non-finite key
//!   times or values are rejected at construction.
//! - No hash maps in any sampling path: clip lookup is a linear
//!   scan over insertion-ordered Vecs.
//!
//! Scope of the bit-stability claim: same build, same target — the
//! same framing as the project's determinism contract (bit-exact
//! repeatability is claimed per renderer/hardware, never across
//! GPU vendors). CPU f32 `+ - * /` and `sqrt` are IEEE operations
//! and Rust does not contract them into FMAs, so a track sampled
//! today and re-sampled after a resume produces identical bits.
//!
//! # Relationship to math.rs
//!
//! Value types mirror math.rs conventions exactly: scalars are
//! f32, vectors are `[f32; 3]` (the arrays math::look_at and
//! friends consume), rotations use the `Quat` newtype over
//! `[f32; 4]`. The module deliberately does NOT `use crate::math`:
//! math.rs pulls in wgpu (its `padded_row_bytes`), and staying
//! dependency-free lets this file be verified standalone with
//! `rustc --edition 2021 --test src/anim.rs`. Interop is free
//! because the representations are identical.

// ---------------------------------------------------------------------
// Easing
// ---------------------------------------------------------------------

/// Easing applied to the segment LEAVING a key (the easing stored
/// on key `a` shapes the interpolation from `a` to the next key).
///
/// Every variant mirrors a curve the film/scene code already uses —
/// no new curves are invented here:
///
/// - `Linear` — scenefile.rs camera keys with `"ease": "linear"`;
///   film.rs `lerp` with the raw segment parameter.
/// - `Step` — film.rs `pose_track`'s hold phase (a value held
///   until the move begins), expressed as an easing: 0 until the
///   segment ends.
/// - `Smootherstep` — THE house default: film.rs `smoother`,
///   scenefile.rs `smootherstep` (motion.ts smootherstep, the camK
///   easing). Note the engine has no cubic smoothstep anywhere —
///   its "smooth" is this quintic — so no divergent cubic variant
///   is offered under that name. Measured f32 caveat: in the flat
///   tail (u > 0.98) the Horner form's evaluation noise is up to
///   ~1.8e-6, so sample-monotonicity holds within that band, not
///   bit-strictly (EaseInOutCubic IS bit-strictly monotonic).
/// - `EaseOutCubic` — film.rs `eo`.
/// - `EaseInOutCubic` — film.rs `eio`.
/// - `EaseOutBack` — film.rs `eob` (overshoots above 1 mid-segment
///   by design; NOT monotonic).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Easing {
    Linear,
    Step,
    Smootherstep,
    EaseOutCubic,
    EaseInOutCubic,
    EaseOutBack,
}

impl Easing {
    /// The house default for newly authored keys (the camK /
    /// poseTrack / scene-format camera default).
    pub const DEFAULT: Easing = Easing::Smootherstep;

    /// Parse a format easing name. Only `"linear"` occurs in
    /// current scene files (scenefile.rs); the other names
    /// future-proof a format v3 without changing this module.
    pub fn from_name(name: &str) -> Option<Easing> {
        match name {
            "linear" => Some(Easing::Linear),
            "step" => Some(Easing::Step),
            "smootherstep" | "smoother" => Some(Easing::Smootherstep),
            "easeOutCubic" | "eo" => Some(Easing::EaseOutCubic),
            "easeInOutCubic" | "eio" => Some(Easing::EaseInOutCubic),
            "easeOutBack" | "eob" => Some(Easing::EaseOutBack),
            _ => None,
        }
    }

    /// Evaluate the curve at `u`. Inputs outside [0, 1] are clamped
    /// first (film.rs `smoother` applies the same clamp). All forms
    /// are polynomial with explicit multiplies — see the module's
    /// float discipline. Endpoints: f(0) = 0 and f(1) = 1
    /// bit-exactly for every variant except `EaseOutBack`'s f(0),
    /// which carries the same ~1-ulp residue as the f64 source
    /// (c3 = c1 + 1 does not cancel c1 exactly); sampling never
    /// relies on it (key instants return key values directly).
    pub fn eval(self, u: f32) -> f32 {
        let u = u.clamp(0.0, 1.0);
        match self {
            Easing::Linear => u,
            Easing::Step => {
                if u >= 1.0 {
                    1.0
                } else {
                    0.0
                }
            }
            // u^3 (u (6u - 15) + 10), scenefile.rs's exact form.
            Easing::Smootherstep => u * u * u * (u * (u * 6.0 - 15.0) + 10.0),
            // 1 - (1 - u)^3, film.rs `eo` with powi(3) expanded.
            Easing::EaseOutCubic => {
                let v = 1.0 - u;
                1.0 - v * v * v
            }
            // film.rs `eio` with powi(3) expanded.
            Easing::EaseInOutCubic => {
                if u < 0.5 {
                    4.0 * u * u * u
                } else {
                    let w = -2.0 * u + 2.0;
                    1.0 - (w * w * w) / 2.0
                }
            }
            // film.rs `eob`: 1 + c3 (u-1)^3 + c1 (u-1)^2.
            Easing::EaseOutBack => {
                const C1: f32 = 1.70158;
                const C3: f32 = C1 + 1.0;
                let w = u - 1.0;
                1.0 + C3 * w * w * w + C1 * w * w
            }
        }
    }
}

// ---------------------------------------------------------------------
// Extrapolation / wrap modes
// ---------------------------------------------------------------------

/// What `sample(t)` does outside the key range. The wrap period is
/// the track's duration (see `Track::duration`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Wrap {
    /// Hold the end keys' values outside the range. This is the
    /// current behaviour of scenefile.rs camera keys and of
    /// film.rs `pose_track`.
    Clamp,
    /// Wrap t into [start, start + duration): the track repeats.
    /// `sample(t) == sample(t + duration)` bit-exactly whenever
    /// `t + duration` is itself exact in f32 (the remainder
    /// operation adds no rounding).
    Loop,
    /// Reflect t into the range with period 2 * duration: forward,
    /// then backward, then forward…
    PingPong,
}

// ---------------------------------------------------------------------
// Values
// ---------------------------------------------------------------------

/// A value that can live on a keyframe track.
pub trait AnimValue: Copy {
    /// Interpolate at eased parameter `u` in [0, 1]. Called only
    /// for 0 < u < 1 by `Track::sample` (segment endpoints return
    /// the key values directly, bit-exactly).
    fn interpolate(a: Self, b: Self, u: f32) -> Self;
    /// Construction-time validation: all components finite.
    fn is_finite(self) -> bool;
}

impl AnimValue for f32 {
    fn interpolate(a: Self, b: Self, u: f32) -> Self {
        // film.rs `lerp`'s exact form.
        a + (b - a) * u
    }
    fn is_finite(self) -> bool {
        f32::is_finite(self)
    }
}

impl AnimValue for [f32; 3] {
    fn interpolate(a: Self, b: Self, u: f32) -> Self {
        // Componentwise `a + (b - a) * u` — the same form as the
        // mix3 closure in scenefile.rs `eval_camera_keys`.
        [
            a[0] + (b[0] - a[0]) * u,
            a[1] + (b[1] - a[1]) * u,
            a[2] + (b[2] - a[2]) * u,
        ]
    }
    fn is_finite(self) -> bool {
        self.iter().all(|c| c.is_finite())
    }
}

/// A quaternion, `[x, y, z, w]` — the "quat-lite" of this module.
/// math.rs has no quaternion type (its rotation path is Euler
/// angles -> matrices via `euler_xyz`); this newtype is the minimal
/// addition needed to keyframe rotations.
///
/// Interpolation is hemisphere-corrected NLERP: shortest path,
/// exact at the endpoints, polynomial + one `sqrt`. It is NOT
/// constant-angular-velocity slerp — slerp needs transcendentals
/// and is excluded by the module's float discipline. For the wide
/// arcs where nlerp's mid-segment ease would be visible, author
/// an intermediate key.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Quat(pub [f32; 4]);

impl Quat {
    pub const IDENTITY: Quat = Quat([0.0, 0.0, 0.0, 1.0]);

    /// Raw constructor: components are stored as given (NOT
    /// normalised). Interpolation normalises its result.
    pub fn new(x: f32, y: f32, z: f32, w: f32) -> Quat {
        Quat([x, y, z, w])
    }

    pub fn dot(self, other: Quat) -> f32 {
        self.0[0] * other.0[0]
            + self.0[1] * other.0[1]
            + self.0[2] * other.0[2]
            + self.0[3] * other.0[3]
    }

    /// Unit-length copy; a zero or non-finite quaternion maps to
    /// IDENTITY rather than producing NaNs.
    pub fn normalized(self) -> Quat {
        let len = self.dot(self).sqrt();
        if len > 0.0 && len.is_finite() {
            Quat([
                self.0[0] / len,
                self.0[1] / len,
                self.0[2] / len,
                self.0[3] / len,
            ])
        } else {
            Quat::IDENTITY
        }
    }
}

impl AnimValue for Quat {
    fn interpolate(a: Self, b: Self, u: f32) -> Self {
        // Shortest path: flip b into a's hemisphere first (the
        // negation is exact, so a track keyed with -b samples
        // bit-identically to the same track keyed with b).
        let b = if a.dot(b) < 0.0 {
            Quat([-b.0[0], -b.0[1], -b.0[2], -b.0[3]])
        } else {
            b
        };
        let r = Quat([
            a.0[0] + (b.0[0] - a.0[0]) * u,
            a.0[1] + (b.0[1] - a.0[1]) * u,
            a.0[2] + (b.0[2] - a.0[2]) * u,
            a.0[3] + (b.0[3] - a.0[3]) * u,
        ]);
        let len = r.dot(r).sqrt();
        if len > 0.0 && len.is_finite() {
            Quat([r.0[0] / len, r.0[1] / len, r.0[2] / len, r.0[3] / len])
        } else {
            a
        }
    }
    fn is_finite(self) -> bool {
        self.0.iter().all(|c| c.is_finite())
    }
}

// ---------------------------------------------------------------------
// Keys, tracks, clips
// ---------------------------------------------------------------------

/// One keyframe: a value at time `t`, plus the easing applied to
/// the segment LEAVING this key toward the next one.
///
/// Note the direction: the scene format's camera keys put `ease`
/// on the key the segment ARRIVES at (scenefile.rs reads the
/// target key's flag). A loader adopting this module maps the
/// format's key[i+1].ease onto anim's key[i].ease; the last key's
/// easing is never used. See ANIM-W11.md.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Key<V> {
    pub t: f32,
    pub value: V,
    pub ease: Easing,
}

impl<V> Key<V> {
    /// A key with the house-default easing (Smootherstep).
    pub fn new(t: f32, value: V) -> Key<V> {
        Key {
            t,
            value,
            ease: Easing::DEFAULT,
        }
    }

    /// A key with an explicit easing-to-next.
    pub fn eased(t: f32, value: V, ease: Easing) -> Key<V> {
        Key { t, value, ease }
    }
}

/// Why a track could not be built. Construction is the only place
/// this module can fail; sampling is infallible.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrackError {
    /// No keys at all — there is no value to sample.
    Empty,
    /// A key time or key value was NaN or infinite, or the key
    /// span overflowed f32.
    NonFinite,
    /// An explicit duration shorter than the key span, or not
    /// finite.
    BadDuration,
}

impl std::fmt::Display for TrackError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TrackError::Empty => write!(f, "animation track has no keys"),
            TrackError::NonFinite => {
                write!(f, "animation track has a non-finite key time or value")
            }
            TrackError::BadDuration => {
                write!(f, "animation track duration is shorter than its key span")
            }
        }
    }
}

impl std::error::Error for TrackError {}

/// A sorted keyframe track over one value type. Immutable after
/// construction; `sample` is a pure function of `t`.
#[derive(Clone, Debug)]
pub struct Track<V: AnimValue> {
    keys: Vec<Key<V>>,
    wrap: Wrap,
    /// Effective wrap period: max(explicit duration, key span).
    duration: f32,
}

impl<V: AnimValue> Track<V> {
    /// Build a track: keys are sorted by time (stable, total
    /// order); the wrap duration defaults to the key span
    /// (last.t - first.t).
    pub fn new(keys: Vec<Key<V>>, wrap: Wrap) -> Result<Track<V>, TrackError> {
        Self::build(keys, wrap, None)
    }

    /// Build a track with an explicit wrap duration (must be >=
    /// the key span). In Loop mode the tail between the last key
    /// and start + duration holds the last key's value, then
    /// wraps; in Clamp mode duration only matters via `end()`.
    pub fn with_duration(
        keys: Vec<Key<V>>,
        wrap: Wrap,
        duration: f32,
    ) -> Result<Track<V>, TrackError> {
        Self::build(keys, wrap, Some(duration))
    }

    fn build(
        mut keys: Vec<Key<V>>,
        wrap: Wrap,
        duration: Option<f32>,
    ) -> Result<Track<V>, TrackError> {
        if keys.is_empty() {
            return Err(TrackError::Empty);
        }
        for k in &keys {
            if !k.t.is_finite() || !k.value.is_finite() {
                return Err(TrackError::NonFinite);
            }
        }
        // Stable sort under a total order: deterministic even for
        // duplicate times (insertion order survives among equals,
        // and sampling lets the later of two same-time keys win at
        // that instant — see `sample`).
        keys.sort_by(|a, b| a.t.total_cmp(&b.t));
        let span = keys[keys.len() - 1].t - keys[0].t;
        if !span.is_finite() {
            return Err(TrackError::NonFinite);
        }
        let duration = match duration {
            None => span,
            Some(d) => {
                if !d.is_finite() || d < span {
                    return Err(TrackError::BadDuration);
                }
                d
            }
        };
        Ok(Track {
            keys,
            wrap,
            duration,
        })
    }

    /// A one-key track: `value` at every t, in every mode.
    pub fn constant(value: V) -> Track<V> {
        Track {
            keys: vec![Key::new(0.0, value)],
            wrap: Wrap::Clamp,
            duration: 0.0,
        }
    }

    pub fn keys(&self) -> &[Key<V>] {
        &self.keys
    }

    pub fn wrap(&self) -> Wrap {
        self.wrap
    }

    /// Time of the first key; the wrap period starts here.
    pub fn start(&self) -> f32 {
        self.keys[0].t
    }

    /// The effective wrap period (see `with_duration`).
    pub fn duration(&self) -> f32 {
        self.duration
    }

    /// start + duration: the instant the track wraps (Loop) or
    /// reflects (PingPong).
    pub fn end(&self) -> f32 {
        self.keys[0].t + self.duration
    }

    /// Sample the track at time `t`. Pure: the result depends only
    /// on the keys, the wrap mode, and `t`.
    ///
    /// Guarantees:
    /// - Sampling exactly at a key time returns that key's value
    ///   bit-exactly, for every easing (endpoints are never routed
    ///   through the easing/interpolation arithmetic). With
    ///   duplicate key times, the later key wins at that instant.
    /// - Between keys a < b: u = (t - a.t) / (b.t - a.t), eased by
    ///   a.ease (the easing-to-next), then interpolated.
    /// - Outside the range, the wrap mode applies (see `Wrap`).
    pub fn sample(&self, t: f32) -> V {
        let keys = &self.keys;
        if keys.len() == 1 {
            return keys[0].value;
        }
        let t = self.wrap_time(t);
        if t < keys[0].t {
            return keys[0].value;
        }
        for w in keys.windows(2) {
            let (a, b) = (&w[0], &w[1]);
            if t < b.t {
                // Loop invariant: a.t <= t < b.t, so u is in
                // [0, 1) and a.t < b.t (span > 0 here).
                let u = (t - a.t) / (b.t - a.t);
                if u <= 0.0 {
                    return a.value;
                }
                return V::interpolate(a.value, b.value, a.ease.eval(u));
            }
        }
        keys[keys.len() - 1].value
    }

    /// Map an arbitrary t into the track's range per the wrap
    /// mode. Clamp leaves t alone (the sampler's end-key fallbacks
    /// do the holding). rem_euclid is exact, so Loop/PingPong
    /// mapping adds no rounding beyond the caller's own t.
    fn wrap_time(&self, t: f32) -> f32 {
        let start = self.keys[0].t;
        match self.wrap {
            Wrap::Clamp => t,
            Wrap::Loop => {
                if self.duration > 0.0 {
                    start + (t - start).rem_euclid(self.duration)
                } else {
                    t
                }
            }
            Wrap::PingPong => {
                if self.duration > 0.0 {
                    let period = 2.0 * self.duration;
                    let p = (t - start).rem_euclid(period);
                    start
                        + if p > self.duration {
                            period - p
                        } else {
                            p
                        }
                } else {
                    t
                }
            }
        }
    }
}

/// A named set of tracks with a duration — the unit a shot or a
/// scene would author (e.g. a "camera" clip holding eye / target /
/// fovY tracks). Tracks are stored per value type in
/// insertion-ordered Vecs; lookup is a linear scan (few tracks per
/// clip, and no hash-order anywhere near a sampling path).
#[derive(Clone, Debug)]
pub struct Clip {
    name: String,
    duration: f32,
    f32_tracks: Vec<(String, Track<f32>)>,
    vec3_tracks: Vec<(String, Track<[f32; 3]>)>,
    quat_tracks: Vec<(String, Track<Quat>)>,
}

impl Clip {
    /// `duration` is the authored clip length; `Clip::duration`
    /// reports the max of it and every track's end, so a clip is
    /// never shorter than its content.
    pub fn new(name: &str, duration: f32) -> Clip {
        Clip {
            name: name.to_string(),
            duration,
            f32_tracks: Vec::new(),
            vec3_tracks: Vec::new(),
            quat_tracks: Vec::new(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn duration(&self) -> f32 {
        let mut d = self.duration;
        for (_, t) in &self.f32_tracks {
            d = d.max(t.end());
        }
        for (_, t) in &self.vec3_tracks {
            d = d.max(t.end());
        }
        for (_, t) in &self.quat_tracks {
            d = d.max(t.end());
        }
        d
    }

    /// Add (or replace, if the name already exists in this type
    /// lane) an f32 track.
    pub fn add_f32(&mut self, name: &str, track: Track<f32>) {
        self.f32_tracks.retain(|(n, _)| n != name);
        self.f32_tracks.push((name.to_string(), track));
    }

    /// Add (or replace) a [f32; 3] track.
    pub fn add_vec3(&mut self, name: &str, track: Track<[f32; 3]>) {
        self.vec3_tracks.retain(|(n, _)| n != name);
        self.vec3_tracks.push((name.to_string(), track));
    }

    /// Add (or replace) a Quat track.
    pub fn add_quat(&mut self, name: &str, track: Track<Quat>) {
        self.quat_tracks.retain(|(n, _)| n != name);
        self.quat_tracks.push((name.to_string(), track));
    }

    pub fn sample_f32(&self, name: &str, t: f32) -> Option<f32> {
        self.f32_tracks
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, tr)| tr.sample(t))
    }

    pub fn sample_vec3(&self, name: &str, t: f32) -> Option<[f32; 3]> {
        self.vec3_tracks
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, tr)| tr.sample(t))
    }

    pub fn sample_quat(&self, name: &str, t: f32) -> Option<Quat> {
        self.quat_tracks
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, tr)| tr.sample(t))
    }

    pub fn track_count(&self) -> usize {
        self.f32_tracks.len() + self.vec3_tracks.len() + self.quat_tracks.len()
    }
}

// ---------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn bits(x: f32) -> u32 {
        x.to_bits()
    }

    fn track_f32(keys: Vec<Key<f32>>, wrap: Wrap) -> Track<f32> {
        Track::new(keys, wrap).unwrap()
    }

    // --- Easing endpoints -------------------------------------------

    #[test]
    fn easing_endpoints_are_bit_exact_where_the_math_allows() {
        // Every variant except EaseOutBack hits both endpoints
        // bit-exactly (see Easing::eval's doc for the eob residue).
        for e in [
            Easing::Linear,
            Easing::Step,
            Easing::Smootherstep,
            Easing::EaseOutCubic,
            Easing::EaseInOutCubic,
        ] {
            assert_eq!(bits(e.eval(0.0)), bits(0.0), "{e:?} at 0");
            assert_eq!(bits(e.eval(1.0)), bits(1.0), "{e:?} at 1");
        }
        // EaseOutBack: f(1) is exact; f(0) carries the c1/c3
        // rounding residue (~4e-8), as in the f64 source.
        assert_eq!(bits(Easing::EaseOutBack.eval(1.0)), bits(1.0));
        assert!(Easing::EaseOutBack.eval(0.0).abs() < 1e-6);
        // Inputs clamp like film.rs `smoother` does.
        assert_eq!(bits(Easing::Linear.eval(-0.5)), bits(0.0));
        assert_eq!(bits(Easing::Linear.eval(1.5)), bits(1.0));
    }

    #[test]
    fn easing_midpoints_and_shape() {
        // EaseInOutCubic is symmetric: f(0.5) = 0.5 exactly, by
        // both branches (4 * 0.5^3 and 1 - 1^3 / 2).
        assert_eq!(bits(Easing::EaseInOutCubic.eval(0.5)), bits(0.5));
        // Smootherstep is symmetric too: 0.5^3 * (0.5 * (3 - 15) + 10)
        // = 0.125 * 4 = 0.5, all exact in f32.
        assert_eq!(bits(Easing::Smootherstep.eval(0.5)), bits(0.5));
        // EaseOutCubic runs ahead of linear mid-segment; Step holds.
        assert!(Easing::EaseOutCubic.eval(0.5) > 0.5);
        assert_eq!(bits(Easing::Step.eval(0.999)), bits(0.0));
        // EaseOutBack overshoots above 1 mid-segment (by design).
        assert!(Easing::EaseOutBack.eval(0.8) > 1.0);
    }

    #[test]
    fn easing_from_name_mirrors_the_format() {
        assert_eq!(Easing::from_name("linear"), Some(Easing::Linear));
        assert_eq!(Easing::from_name("smootherstep"), Some(Easing::Smootherstep));
        assert_eq!(Easing::from_name("eio"), Some(Easing::EaseInOutCubic));
        assert_eq!(Easing::from_name("bogus"), None);
    }

    // --- Boundary sampling per wrap mode ----------------------------

    #[test]
    fn clamp_holds_end_values_outside_the_range() {
        let tr = track_f32(
            vec![Key::new(1.0, 10.0), Key::new(3.0, 30.0)],
            Wrap::Clamp,
        );
        assert_eq!(bits(tr.sample(-100.0)), bits(10.0));
        assert_eq!(bits(tr.sample(0.999)), bits(10.0));
        assert_eq!(bits(tr.sample(100.0)), bits(30.0));
        assert_eq!(bits(tr.sample(3.0)), bits(30.0));
    }

    #[test]
    fn loop_wraps_both_directions() {
        let tr = track_f32(
            vec![
                Key::eased(0.0, 0.0, Easing::Linear),
                Key::new(2.0, 4.0),
            ],
            Wrap::Loop,
        );
        // Before the first key: wraps to the equivalent in-range t.
        assert_eq!(bits(tr.sample(-0.5)), bits(tr.sample(1.5)));
        // After the last key: wraps likewise.
        assert_eq!(bits(tr.sample(2.5)), bits(tr.sample(0.5)));
        assert_eq!(bits(tr.sample(102.0)), bits(tr.sample(0.0)));
    }

    #[test]
    fn pingpong_reflects_both_directions() {
        let tr = track_f32(
            vec![
                Key::eased(0.0, 0.0, Easing::Linear),
                Key::new(2.0, 4.0),
            ],
            Wrap::PingPong,
        );
        // Forward leg then reflected leg: 2.5 is 1.5 on the way back.
        assert_eq!(bits(tr.sample(2.5)), bits(tr.sample(1.5)));
        assert_eq!(bits(tr.sample(3.0)), bits(tr.sample(1.0)));
        // Full period (2 * duration) returns to the start phase.
        assert_eq!(bits(tr.sample(4.5)), bits(tr.sample(0.5)));
        assert_eq!(bits(tr.sample(100.0)), bits(tr.sample(0.0)));
        // Before the start reflects too.
        assert_eq!(bits(tr.sample(-0.5)), bits(tr.sample(0.5)));
    }

    // --- Loop wrap equivalence (bit-exact) --------------------------

    #[test]
    fn loop_sample_is_bit_identical_one_period_later() {
        let tr = track_f32(
            vec![
                Key::eased(0.0, -1.0, Easing::EaseInOutCubic),
                Key::eased(1.0, 2.0, Easing::Smootherstep),
                Key::new(2.0, 0.5),
            ],
            Wrap::Loop,
        );
        let d = tr.duration();
        assert_eq!(bits(d), bits(2.0));
        // All probe values and t +/- d are exact in f32, so the
        // exact remainder makes the wrapped times bit-identical.
        for t in [0.0f32, 0.25, 0.5, 1.25, 1.75] {
            assert_eq!(bits(tr.sample(t)), bits(tr.sample(t + d)), "t={t} +d");
            assert_eq!(bits(tr.sample(t)), bits(tr.sample(t + 2.0 * d)), "t={t} +2d");
            assert_eq!(bits(tr.sample(t)), bits(tr.sample(t - d)), "t={t} -d");
        }
    }

    #[test]
    fn loop_with_explicit_duration_holds_the_tail_then_wraps() {
        let tr = Track::with_duration(
            vec![
                Key::eased(0.0, 0.0, Easing::Linear),
                Key::new(2.0, 4.0),
            ],
            Wrap::Loop,
            3.0,
        )
        .unwrap();
        // Tail (2, 3): last value holds…
        assert_eq!(bits(tr.sample(2.5)), bits(4.0));
        // …and the period is 3, not the 2-second key span.
        assert_eq!(bits(tr.sample(3.5)), bits(tr.sample(0.5)));
        assert_eq!(bits(tr.sample(-0.5)), bits(tr.sample(2.5)));
    }

    // --- Keyframe instants, sorting, duplicates ---------------------

    #[test]
    fn key_instants_return_key_values_bit_exactly_for_every_easing() {
        for e in [
            Easing::Linear,
            Easing::Step,
            Easing::Smootherstep,
            Easing::EaseOutCubic,
            Easing::EaseInOutCubic,
            Easing::EaseOutBack, // even the overshooting curve
        ] {
            let tr = track_f32(
                vec![
                    Key::eased(0.0, 3.25, e),
                    Key::eased(1.0, -7.5, e),
                    Key::new(2.0, 1.0),
                ],
                Wrap::Clamp,
            );
            assert_eq!(bits(tr.sample(0.0)), bits(3.25), "{e:?}");
            assert_eq!(bits(tr.sample(1.0)), bits(-7.5), "{e:?}");
            assert_eq!(bits(tr.sample(2.0)), bits(1.0), "{e:?}");
        }
    }

    #[test]
    fn keys_are_sorted_at_construction_and_duplicates_step() {
        // Deliberately unsorted, with a duplicate time at t = 1:
        // the later of the two wins at that instant (a step).
        let tr = track_f32(
            vec![
                Key::eased(2.0, 30.0, Easing::Linear),
                Key::eased(0.0, 0.0, Easing::Linear),
                Key::eased(1.0, 10.0, Easing::Linear),
                Key::eased(1.0, 20.0, Easing::Linear),
            ],
            Wrap::Clamp,
        );
        assert_eq!(tr.keys()[0].t, 0.0);
        assert_eq!(tr.keys()[3].t, 2.0);
        assert_eq!(bits(tr.sample(0.5)), bits(5.0));
        assert_eq!(bits(tr.sample(1.0)), bits(20.0));
        assert_eq!(bits(tr.sample(1.5)), bits(25.0));
    }

    #[test]
    fn step_easing_holds_until_the_segment_ends() {
        let tr = track_f32(
            vec![
                Key::eased(0.0, 1.0, Easing::Step),
                Key::new(1.0, 5.0),
            ],
            Wrap::Clamp,
        );
        assert_eq!(bits(tr.sample(0.0)), bits(1.0));
        assert_eq!(bits(tr.sample(0.999)), bits(1.0));
        assert_eq!(bits(tr.sample(1.0)), bits(5.0));
    }

    #[test]
    fn construction_rejects_bad_tracks() {
        assert_eq!(
            Track::<f32>::new(vec![], Wrap::Clamp).unwrap_err(),
            TrackError::Empty
        );
        assert_eq!(
            Track::new(vec![Key::new(f32::NAN, 1.0)], Wrap::Clamp).unwrap_err(),
            TrackError::NonFinite
        );
        assert_eq!(
            Track::new(vec![Key::new(0.0, f32::INFINITY)], Wrap::Clamp).unwrap_err(),
            TrackError::NonFinite
        );
        assert_eq!(
            Track::with_duration(
                vec![Key::new(0.0, 0.0), Key::new(2.0, 1.0)],
                Wrap::Loop,
                1.0,
            )
            .unwrap_err(),
            TrackError::BadDuration
        );
        // A constant track samples its value everywhere, every mode.
        let c = Track::constant(7.0f32);
        assert_eq!(bits(c.sample(-1e6)), bits(7.0));
        assert_eq!(bits(c.sample(1e6)), bits(7.0));
    }

    // --- Monotonicity over 10k samples ------------------------------

    #[test]
    fn ease_in_out_track_is_monotonic_over_10k_samples() {
        let tr = track_f32(
            vec![
                Key::eased(0.0, 0.0, Easing::EaseInOutCubic),
                Key::new(1.0, 10.0),
            ],
            Wrap::Clamp,
        );
        let mut prev = tr.sample(0.0);
        for i in 1..=10_000u32 {
            let t = i as f32 / 10_000.0;
            let v = tr.sample(t);
            assert!(v >= prev, "decrease at i={i}: {prev} -> {v}");
            prev = v;
        }
        assert_eq!(bits(prev), bits(10.0));
    }

    #[test]
    fn smootherstep_track_is_monotonic_within_f32_evaluation_noise() {
        // The house default, with an honest caveat: in f32 the
        // Horner form's flat tail (u > 0.98, where the true slope
        // is ~0) carries evaluation noise — measured worst dip
        // 1.79e-6 over 100k samples at u = 0.9955 (probe in
        // ANIM-W11.md). EaseInOutCubic above is exactly monotonic;
        // smootherstep is monotonic within a 2e-6 band. The f64
        // sources have the same effect at ~1e-16, invisible there.
        let tr = track_f32(vec![Key::new(0.0, 0.0), Key::new(1.0, 1.0)], Wrap::Clamp);
        let mut prev = tr.sample(0.0);
        let mut worst_dip = 0.0f32;
        for i in 1..=10_000u32 {
            let t = i as f32 / 10_000.0;
            let v = tr.sample(t);
            if v < prev {
                worst_dip = worst_dip.max(prev - v);
            }
            prev = v;
        }
        assert!(worst_dip <= 2e-6, "dip {worst_dip:e} exceeds the noise band");
        assert_eq!(bits(prev), bits(1.0));
    }

    // --- Purity ------------------------------------------------------

    #[test]
    fn sampling_is_pure_interleaved_order_changes_nothing() {
        let tr = track_f32(
            vec![
                Key::eased(0.0, 1.0, Easing::EaseInOutCubic),
                Key::eased(0.7, -2.0, Easing::Smootherstep),
                Key::new(1.3, 4.0),
            ],
            Wrap::Clamp,
        );
        let first = tr.sample(0.3);
        let _ = tr.sample(0.9);
        let _ = tr.sample(-5.0);
        let _ = tr.sample(1.2);
        assert_eq!(bits(tr.sample(0.3)), bits(first));
    }

    // --- Vec3 and Quat tracks ----------------------------------------

    #[test]
    fn vec3_track_interpolates_componentwise() {
        let tr = Track::new(
            vec![
                Key::eased(0.0, [0.0, 0.0, 0.0], Easing::Linear),
                Key::new(2.0, [2.0, 4.0, 6.0]),
            ],
            Wrap::Clamp,
        )
        .unwrap();
        // u = 0.5 with exact operands: bit-exact midpoint.
        assert_eq!(tr.sample(1.0), [1.0, 2.0, 3.0]);
        assert_eq!(tr.sample(-1.0), [0.0, 0.0, 0.0]);
        assert_eq!(tr.sample(9.0), [2.0, 4.0, 6.0]);
    }

    #[test]
    fn quat_track_nlerps_the_shortest_path() {
        // Identity -> 180 degrees about Y; the midpoint is the
        // 90-degree quaternion (0, sqrt(1/2), 0, sqrt(1/2)).
        let tr = Track::new(
            vec![
                Key::eased(0.0, Quat::IDENTITY, Easing::Linear),
                Key::new(2.0, Quat::new(0.0, 1.0, 0.0, 0.0)),
            ],
            Wrap::Clamp,
        )
        .unwrap();
        let mid = tr.sample(1.0);
        let s = std::f32::consts::FRAC_1_SQRT_2;
        assert!((mid.0[1] - s).abs() < 1e-6 && (mid.0[3] - s).abs() < 1e-6, "{mid:?}");
        assert_eq!(mid.0[0], 0.0);
        assert_eq!(mid.0[2], 0.0);
        // Endpoints are the authored keys, bit-exact.
        assert_eq!(tr.sample(0.0), Quat::IDENTITY);
        assert_eq!(tr.sample(2.0), Quat::new(0.0, 1.0, 0.0, 0.0));
    }

    #[test]
    fn quat_hemisphere_flip_makes_sign_of_key_irrelevant() {
        let half = std::f32::consts::FRAC_1_SQRT_2;
        let pos = Quat::new(0.0, half, 0.0, half); // +90 deg about Y
        let neg = Quat::new(0.0, -half, 0.0, -half); // same rotation
        let a = Track::new(
            vec![
                Key::eased(0.0, Quat::IDENTITY, Easing::Linear),
                Key::new(1.0, pos),
            ],
            Wrap::Clamp,
        )
        .unwrap();
        let b = Track::new(
            vec![
                Key::eased(0.0, Quat::IDENTITY, Easing::Linear),
                Key::new(1.0, neg),
            ],
            Wrap::Clamp,
        )
        .unwrap();
        // Negation is exact, so mid-segment samples are bit-equal.
        for t in [0.25f32, 0.5, 0.75] {
            assert_eq!(a.sample(t), b.sample(t), "t={t}");
        }
    }

    #[test]
    fn accessors_report_the_tracks_shape() {
        let tr = Track::with_duration(
            vec![
                Key::eased(0.5, 1.0, Easing::Linear),
                Key::new(2.5, 3.0),
            ],
            Wrap::PingPong,
            4.0,
        )
        .unwrap();
        assert_eq!(tr.wrap(), Wrap::PingPong);
        assert_eq!(bits(tr.start()), bits(0.5));
        assert_eq!(bits(tr.duration()), bits(4.0));
        assert_eq!(bits(tr.end()), bits(4.5));
        assert_eq!(tr.keys().len(), 2);
        // Quat::normalized rescales and rescues the zero quaternion.
        let q = Quat::new(0.0, 2.0, 0.0, 0.0).normalized();
        assert_eq!(q, Quat::new(0.0, 1.0, 0.0, 0.0));
        assert_eq!(Quat::new(0.0, 0.0, 0.0, 0.0).normalized(), Quat::IDENTITY);
    }

    // --- Clips --------------------------------------------------------

    #[test]
    fn clip_samples_named_tracks_and_reports_duration() {
        let mut clip = Clip::new("camera", 0.0);
        clip.add_vec3(
            "eye",
            Track::new(
                vec![
                    Key::eased(0.0, [0.0, 2.0, 5.0], Easing::Linear),
                    Key::new(2.0, [0.0, 1.0, 3.0]),
                ],
                Wrap::Clamp,
            )
            .unwrap(),
        );
        clip.add_f32(
            "fovY",
            Track::new(
                vec![Key::eased(0.0, 44.0, Easing::Linear), Key::new(2.0, 36.0)],
                Wrap::Clamp,
            )
            .unwrap(),
        );
        assert_eq!(clip.track_count(), 2);
        assert_eq!(clip.sample_vec3("eye", 1.0), Some([0.0, 1.5, 4.0]));
        assert_eq!(clip.sample_f32("fovY", 1.0), Some(40.0));
        assert_eq!(clip.sample_f32("eye", 1.0), None); // wrong lane
        assert_eq!(clip.sample_vec3("missing", 1.0), None);
        // Duration grows to cover the tracks even when authored 0.
        assert_eq!(bits(clip.duration()), bits(2.0));
        // Re-adding a name replaces the track.
        clip.add_f32("fovY", Track::constant(50.0));
        assert_eq!(clip.sample_f32("fovY", 1.0), Some(50.0));
        assert_eq!(clip.track_count(), 2);
    }
}
