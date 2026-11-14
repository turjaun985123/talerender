# ANIM-W11 — animation evaluator core: keyframe tracks, clips, bit-stable sampling

Wave 2, seat W11 (audit Phase 4 foundation: the reusable animation
evaluator). Deliverables: `src/anim.rs` (new module, ~1,050 lines
including tests) + this note. This seat edited no existing file and
committed nothing. Line numbers are against the tree as surveyed
(HEAD after d7bebdd, the professional-CLI commit; sibling seats'
untracked modules present but untouched).

## 0. Headline

The engine already evaluates every frame as a pure function of
time — but the machinery is hand-rolled at each call site, in f64,
in three different shapes (format-v2 camera keys, camK + `shoot`,
`pose_track`). `src/anim.rs` is that machinery extracted into one
reusable, f32-native, bit-stable layer: keyframe tracks with
easing-to-next, clamp/loop/ping-pong extrapolation, and named
multi-track clips. It is OPT-IN — nothing calls it yet. Adoption
is per call site, gated, and mapped in §4 below.

## 1. What was built

- `Easing` — the codebase's own curves, nothing invented (§3).
- `Wrap::{Clamp, Loop, PingPong}` — extrapolation modes. Clamp is
  today's behaviour everywhere; Loop/PingPong are new capability
  for cycles (walk loops, cloud drift, blink-free idles).
- `Key<V> { t, value, ease }` — `ease` shapes the segment LEAVING
  the key (easing-to-next; see the direction warning in §4.1).
- `Track<V>` — sorted at construction (stable sort, `total_cmp`
  total order), immutable afterwards, `sample(t)` infallible and
  pure. Construction validates: empty / non-finite / bad-duration
  tracks are `TrackError`s, never panics. An explicit duration
  longer than the key span is supported (Loop tail holds the last
  value, then wraps). Duplicate key times step: the later key
  wins at that instant.
- `AnimValue` — implemented for `f32`, `[f32; 3]`, and `Quat`.
- `Quat([f32; 4])` — quat-lite. math.rs has NO quaternion type
  (its rotation path is Euler -> `euler_xyz` matrices), so this
  newtype is the minimal addition for keyframed rotation.
  Interpolation is hemisphere-corrected nlerp: shortest path,
  exact at endpoints, polynomial + one sqrt. Slerp is deliberately
  excluded (transcendentals break the float discipline); nlerp's
  non-constant angular velocity on wide arcs is documented in the
  module, with "author an intermediate key" as the remedy.
- `Clip` — a named set of tracks (one Vec lane per value type)
  with a duration; `sample_f32 / sample_vec3 / sample_quat` by
  track name. This is the unit a shot's camera or a character's
  channels would be authored as.

Sampling guarantees, all test-backed:

- Sampling exactly at a key time returns the key value
  bit-exactly, for EVERY easing including EaseOutBack (segment
  endpoints never route through easing arithmetic).
- `Loop`: `sample(t) == sample(t + duration)` bit-exactly
  whenever `t + duration` is exact in f32.
- Sampling is order-independent (interleaved sampling test):
  no state, no cache, no history.

## 2. Dependency note for the lead

**anim.rs depends on nothing — not even math.rs.** Deliberate:
math.rs pulls in wgpu (`padded_row_bytes` uses
`wgpu::COPY_BYTES_PER_ROW_ALIGNMENT`), and a zero-dependency
module is standalone-testable (§5). The value types are math.rs's
exact representations — f32, `[f32; 3]`, and `[f32; 4]` inside
`Quat` — so sampled values feed `math::look_at`, `mix3`-style
consumers, and the scene state with zero conversion. Wiring is
one line: `mod anim;` in main.rs.

## 3. Easing map — mirrored, not invented

| anim variant | codebase source | where it rules today |
| --- | --- | --- |
| `Linear` | scenefile.rs `"ease": "linear"`; film.rs:109 `lerp` raw factor | camera keys opting out of smooth |
| `Step` | film.rs:150 `pose_track` hold phase | pose channels hold, then move |
| `Smootherstep` | film.rs:130 `smoother`; scenefile.rs:767 `smootherstep` | THE default: camK, poseTrack, camera keys |
| `EaseOutCubic` | film.rs:115 `eo` | moves.ts easings |
| `EaseInOutCubic` | film.rs:118 `eio` | moves.ts easings |
| `EaseOutBack` | film.rs:125 `eob` | moves.ts easings (overshoots; not monotonic) |

Two naming decisions the lead should ratify or veto:

1. **There is no cubic smoothstep in this engine.** What the
   brief called "smoothstep" is the quintic smootherstep
   (motion.ts), so the module offers `Smootherstep` under its
   real name and does NOT add a divergent cubic curve.
2. `powi(3)` in the f64 sources is written as explicit multiplies
   in f32 (identical polynomial, guaranteed evaluation form —
   part of the float discipline, §4 of the module docs).

`Easing::from_name` accepts the format's `"linear"` plus the
Rust-side names, future-proofing a format v3; only `"linear"`
occurs in current scene files.

## 4. Adoption map (later, per-site, gated)

### 4.1 scenefile.rs camera keys — the direct replacement

`eval_camera_keys` (scenefile.rs:775) + `smootherstep`
(scenefile.rs:767) are already a track evaluator: keys
{t, eye, target, fovY?, ease?}, end-clamped, smootherstep
default. Adoption: the loader builds a `Clip` named after the
scene with `eye`/`target` `[f32; 3]` tracks and a `fovY` f32
track, and the per-frame call becomes three `sample_*` lookups.

Two translation facts the adopting change must honour:

- **Ease direction flips.** The format stores ease on the key a
  segment ARRIVES at (the target key's flag makes its incoming
  segment linear); anim stores easing-to-next on the key the
  segment LEAVES. Loader mapping: format key[i+1].ease ->
  anim key[i].ease. The last format key's ease is meaningless
  today and stays meaningless.
- **f64 -> f32 evaluation changes bits.** The current code
  evaluates the easing and the segment parameter in f64 and
  casts the result; anim evaluates natively in f32. The stored
  key values are exact f32 either way (spec §11), but the eased
  parameter can differ by ~1 ulp, so rendered frames can differ
  in the last bit on some pixels. This adoption must go through
  the §28 A/B anchor and the film A/B gate like any other
  staging change — it is a behaviour-preserving refactor in
  value space, NOT a bit-identical one, and must not be landed
  silently as "just a refactor".

### 4.2 film.rs camK + shoot — per-shot camera clips

Every shot function computes `let k = cam_k(lt, d, hold)`
(film.rs:135) and calls `shoot(f, from, to, at, at2, k, fov)`
(film.rs:1332), which `mix3`s eye and target by that one scalar.
Adoption: per shot, a two-key `Clip` (eye / target / fov tracks,
keys at 0 and d) replaces the from/to plumbing; camK's hold
becomes key placement — keys at `h*d` (value = from) and
`(1-h)*d` (value = to) with Smootherstep between them reproduces
hold-in/out exactly, since cam_k IS smootherstep over the
sub-interval between the holds. The win is uniformity: shot
cameras become data (diffable, validatable, serialisable into
scene format) instead of per-shot imperative code.

### 4.3 film.rs pose_track — W12's lane, mapping recorded here

`pose_track` (film.rs:150) with its `Key = (t, value, moveSecs)`
(film.rs:140) is the character-channel evaluator — adoption is
W12's (charanim), not this seat's. The exact encoding into anim
tracks, so W12 doesn't have to re-derive it: a poseTrack key
(t1, v1, m1) becomes TWO anim keys — `(t1 - m1, v0, Smootherstep)`
and `(t1, v1, Step)` — and the preceding segment's key gets
`Step` as its easing-to-next. Between keys the value then holds
(via Step on the long segment), moves over exactly
[t1 - m1, t1] under smootherstep, and lands bit-exactly on v1 —
poseTrack's semantics, reproduced in the track model.

## 5. Determinism evidence (standalone run)

Recipe (the module is dependency-free by design):

```
~/.cargo/bin/rustc --edition 2021 --test src/anim.rs -o /tmp/anim_test
/tmp/anim_test        # also re-run with -O: identical results
```

**20/20 tests pass**, debug and `-O`, zero warnings; the file
also compiles clean as an rlib. Coverage: easing endpoints
(bit-exact where the math allows — EaseOutBack's f(0) carries
the documented ~4e-8 c1/c3 residue, as in the f64 source),
boundary sampling before/after the range in all three wrap
modes, loop wrap equivalence (bit-compared at t, t+d, t+2d,
t-d), key-instant exactness for every easing, sorting +
duplicate-time steps, construction errors, vec3 componentwise
sampling, quat nlerp midpoint + hemisphere sign-invariance
(bit-equal), clip lanes + replacement, and the two 10k-sample
monotonicity checks below.

### The one measured caveat (not hidden)

- **EaseInOutCubic is bit-strictly monotonic**: worst dip over
  100k samples = 0. The required 10k-sample track check passes
  with strict `>=`.
- **Smootherstep is monotonic only within 1.8e-6.** Probe
  (100k samples of `Easing::eval`): worst dip 1.79e-6 at
  u = 0.9955. Cause: in the flat tail the true slope is ~0 while
  the Horner form's intermediate magnitudes reach ~15, so f32
  rounding noise (~1e-6) exceeds the per-step rise. The f64
  sources have the identical effect at ~1e-16 — invisible there,
  visible in an f32-native layer. Consequences: none for
  rendering (1.8e-6 of a channel's range is far below a pixel
  or a paint-cache key quantum), but the test asserts the
  measured band (<= 2e-6) instead of strict monotonicity, and
  any future "is this channel monotonic" gate should use the
  band. The evaluation form was NOT changed to chase strict
  monotonicity — mirroring the source polynomial exactly is
  worth more than the last 2e-6.

## 6. Out of scope (explicitly)

- Character channels — expressions, gaze, blink, lip-sync,
  walk/idle procedural motion: W12 (charanim). anim provides
  tracks; what the channels ARE is theirs.
- Scene-format changes: no v3 schema here. `from_name` merely
  stops the format from needing a code change later.
- GPU-side evaluation, render-graph integration (W13's lane).
- Slerp (see §1), cubic-Bezier tracks, and per-key tangent /
  Hermite authoring — the codebase has none of these in its
  animation paths today; adding them is a format decision for
  the lead, not a foundation-layer invention.
- Editing film.rs / scenefile.rs call sites: adoption (§4) is
  the lead's integration step, gated per site.
