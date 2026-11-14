# LOCOMOTION-W24 — Procedural locomotion module (Phase 4, seat W24)

**Seat:** TaleRender Wave 2, W24 (procedural locomotion).
**Date:** 2026-11-14. **Status:** module DONE, self-contained,
tests green (below). **Nothing else in `src/` was touched, nothing
committed** — `main.rs` does not yet declare the module; wiring is
the lead's adoption step (§3).

**Delivered:** `src/locomotion.rs` (new file only) + this note.

**Verification (standalone, exactly as the module header claims):**

```bash
rustc --edition 2021 --test src/locomotion.rs -o /tmp/loco_test && /tmp/loco_test
→ 9 passed; 0 failed   (also: --crate-type lib build, clean)
```

---

## 1. Why this exists

The sister project (Tall Tale engine) proved arithmetically in
2026-11 that a hand-staged walk cycle can be **backwards**: the
planted foot drifted toward the direction of travel, the arms
swung in-phase with the same-side leg, and the bob was inverted.
None of that is visible in a single still — only the arithmetic
shows it. TaleRender's film walks are staged the same hand-rolled
way today (§2), so this module encodes the correct gait math as
pure evaluators with the invariants stated as laws and asserted
numerically by tests (§5), so the mistake cannot be re-authored.

## 2. Survey — how walking works today (cited)

| Piece | Today | Citation |
|---|---|---|
| Phase | `walk_phase(dist, stride) = dist / stride * PI` — PI of phase per step, driven by **travelled distance**, not time | film.rs:169-171; call sites use stride 0.58–0.62 |
| Bob | `walk_bob(phase, amp) = amp * (1 - \|sin phase\|)`, amp 3.0–5.0 stage px, added **downward** into body Y | film.rs:172-174; toonfull.rs:324 |
| Legs (painter) | In-card sinusoid: offset `sin(phase + (PI if right)) * amp` (0.55 Walk / 1.0 Run), lift `max(0, -cos(...)) * (34 / 70)` px | toonfull.rs:358-364 |
| Arms while walking | **Static pairs** — e.g. arm_r (16,-6), arm_l (10,6); no phase-driven swing exists in the film | film.rs:1438-1439, 1759-1760, 1913-1914 |
| State channels | `FullState { phase: f32 (rad), bob: f32 (stage px), arm_l/arm_r: (f32, f32) (deg: shoulder, elbow), legs: Legs::{Stand, Walk, Run, Sit, Jump} }` | toonfull.rs:104-110, 147-152 |
| Scene JSON | state keys `phase`, `bob`, `armL`, `armR`, `legs` — the same names are W15's clip channels (`phase`, `bob`, `armL.0/.1`, `armR.0/.1`, `legs`) | scenefile.rs:647-669; FORMAT-EXT-CLIPS-W15.md §3 |

Two survey findings the adoption must respect:

1. **The painter's in-card cycle is facing-dependent.** In card
   space the airborne foot travels from its forward extreme to
   its backward extreme (lift is positive exactly while
   `sin(phase)` falls through its negative half). That reads as
   a correct swing for travel toward −x_stage and as a
   **backwards walk** for +x_stage. Facing is currently owned by
   staging via the card yaw; the module therefore works in a
   travel axis (+x = direction of travel) where its laws hold
   unconditionally, and staging keeps owning the sign.
2. **W15 §5.3 already ruled gait out of clip data:** `walkPhase` /
   `walkBob` drivers "are functions of another channel's
   integral, not independent key data… the gait derivation stays
   code." This module is that code; §4's authoring surface is a
   procedural-driver parameter block (the `blinkDirector(seed)`
   precedent), **not** clip tracks.

## 3. The module

`src/locomotion.rs` — pure functions of `(phase, GaitParams)`,
f64 internally (the film's motion convention), f32 at the
FullState boundary. Phase convention: **0 = left foot strike**,
2 PI = two steps; right leg offset exactly PI.

**Laws (all test-asserted, §5):**

- **L1 — planted foot is world-stationary.** Stance is *linear*
  in phase: the foot sweeps +A → −A relative to the hip,
  A = duty · step_length, at relative velocity exactly −speed.
  (A sinusoidal foot path — the painter's current shape — can
  satisfy this at only one instant; it slides everywhere else.)
- **L2 — legs exactly anti-phase** by construction
  (half-cycle offset in the leg-local phase).
- **L3 — arms counter-phase to the same-side leg:** shoulder
  swing = −arm_amp · (same-side foot fore-aft fraction). Each
  arm is then in-phase with the opposite leg automatically.
- **L4 — bob derived, two minima per cycle at double support.**
  Straight-support-leg geometry: hip height = sqrt(L² − x²)
  over the planted foot's offset x; the hip rides the stance
  leg nearer vertical, so `drop = (min over stance legs
  |x|/A)²`. For duty > 0.5 this peaks exactly twice per cycle,
  inside the double-support windows, and is 0 at each
  mid-stance. In run flight the drop continues ballistically
  (parabola through −flight_rise at mid-flight).

**Walk↔run** is one parameter family: `GaitParams::walk(speed)`,
`::run(speed)`, `::blended(run_factor)` (duty 0.62 → 0.36,
stride 0.62 → 0.85, clearance, arm amp and flight rise lerp;
`is_run()` ⟺ duty < 0.5 ⟺ a flight interval exists — staging
maps it to `Legs::Run` vs `Legs::Walk`).

**Outputs** (`GaitState`): per-foot `(x, height, stance)` in the
body frame (travel axis, metres), `bob` (stage px, FullState
units), `support_drop` (raw fraction), and `arm_l`/`arm_r`
degree pairs ready for `FullState.arm_l`/`arm_r`. Elbow gets a
light −0.35× echo of the shoulder swing; staging may ignore the
elbow component and keep authored elbows.

## 4. Adoption — how film.rs's walk staging would use it

`mod locomotion;` in main.rs (lead), then each existing
`(walk_phase, walk_bob)` pair is replaced by one eval. The film
drives phase by **distance** (integrals of its position drivers —
that is what keeps shared-driver groups in lockstep, e.g. s02's
three marchers on one `travelled`), so adoption keeps
distance-driving via `phase_for_distance` (film's law, unchanged;
only the origin shifts — convert with `painter_phase`, which
puts module left mid-stance on the painter's phase 0):

```rust
let gait = GaitParams::walk(SPEED);            // or ::blended(run_factor)
let g = locomotion::eval(&gait, locomotion::phase_for_distance(&gait, dist));
st.phase = locomotion::painter_phase(&gait, g.phase as f64) as f32;
st.bob = g.bob;
st.legs = if walking && gait.is_run() { Legs::Run }
          else if walking { Legs::Walk } else { Legs::Stand };
st.arm_l = g.arm_l;   // ONLY where the shot does not author the arm (below)
st.arm_r = g.arm_r;
```

Call sites (all of them — grep `walk_phase(` / `walk_bob(`):

| Site | Today | Adoption note |
|---|---|---|
| s02, film.rs:1423-1424 | `walk_phase(travelled, 0.62)`, bob 5.0, 3 chars share ph/bob, arms static (16,-6)/(10,6) | One shared eval from `travelled`; the gait arm defaults ARE these static pairs, so enabling swing is a pure addition of motion — a look decision |
| s06, film.rs:1625-1627 (`mk_walk` closure) | dist = wk·5.96 / wk·6.1, stride 0.62, bob 5.0 | Closure body becomes the eval; per-character dist already separate |
| s07, film.rs:1702-1704 | wk·2.0, stride 0.6, bob 4.0 | arm_r is a poseTrack **story beat** (coconut) — do NOT overwrite with gait arm; arm_l (12,6) is free |
| s08 Suzuki, film.rs:1737-1739 | wk_s·3.28, stride 0.6, bob 5.0 | arm_l (38,-52) holds the paper-map plate — no swing on that arm; arm_r (12,8) free |
| s08 Onoda, film.rs:1754-1756 | wk_o·2.25, stride 0.6, bob 4.0, arms static | Both arms free |
| s10 Tani, film.rs:1842-1844 | wk·0.55, stride 0.58, bob 3.0 | Set `step_length: 0.58, bob_amp: 3.0`; arm_r is a poseTrack beat — keep |
| s11, film.rs:1908-1910 | wk·2.5, stride 0.6, bob 4.0, arms (16,-6)/(10,5) "D15 source walk-in arms" | Arms are source-authored for this shot — gait swing off unless the user re-stages |

**Two adoption decisions change shipped pixels — flagged, not
taken here:**

1. **Bob phase.** Module bob peaks (body lowest) at double
   support; film's `walk_bob` peaks at the passing pose — a
   quarter-step shift on every walking frame. Physically the
   module is right (inverted pendulum), but the film's current
   frames are the fidelity record: adopt behind an A/B + user
   sign-off, like every other deliberate re-baseline.
2. **Arm swing is new motion.** Where arms are static today,
   L3 swing adds life; where arms carry story beats or props
   (table above), gait must not touch them, or must be blended
   out around the beat (staging owns the blend factor).

**scenefile lane:** a character node whose state carries the §4
`gait` block would be evaluated at the file's `time` by the
loader the same way (distance accumulated from the node's
position track, or time via `phase_at_time` for constant-speed
shots). No scene file carries one today; this is new authoring
surface for the lead/W15 validator to ratify, not data this seat
created.

## 5. Tests — the studio lessons as assertions

`rustc --edition 2021 --test src/locomotion.rs` → **9 passed**.

| Test | Lesson asserted |
|---|---|
| `legs_exactly_antiphase` | L2: right(φ) ≡ left(φ + PI), bit-exact |
| `planted_foot_world_velocity_zero` | L1: numeric — across both feet's full stance windows, speed + d(foot.x)/dt ≈ 0 (< 1e-3 m/s) |
| `stance_sweep_matches_ground_speed` | L1 closed form: stance sweep = speed × stance duration; strike at exactly +A |
| `arm_counterphase_same_side_leg` | L3: Pearson(arm, same-side leg) < −0.99 (it is exactly −1 by construction); opposite side > +0.8 — *not* ~1, correctly: with duty ≠ 0.5 the position waveform isn't half-cycle antisymmetric even though the phase offset is exact (measured +0.836 at duty 0.62) |
| `bob_two_minima_per_cycle_at_double_support` | L4: drop is exactly PI-periodic (2× stride frequency), 0 at mid-stance, exactly 2 maxima per cycle, both inside double-support windows |
| `swing_continuous_and_lifted` | No teleports: position continuous at strike/toe-off, height 0 at both, peaks at step_height mid-swing, never below ground |
| `walk_run_blend_duty_factor` | Duty falls monotonically across the blend; flight ⟺ duty < 0.5; walk always has a foot down and has double support; run has flight and no double support |
| `phase_law_matches_film_walk_phase` | phase_for_distance ≡ film.rs:169-171; cadence advances PI per step |
| `painter_phase_origin` | Module left mid-stance maps to painter phase 0 |

## 6. Authoring parameters (scene / clip vocabulary)

Outputs land on W15's existing character channels — `phase`,
`bob`, `armL.0`, `armL.1`, `armR.0`, `armR.1`, `legs` — no new
channel names. The **inputs** are a procedural-driver block
(W15 §5.3: gait stays code; the `blinkDirector` seed is the
precedent for driver parameters living beside state). Proposed
shape for the lead to ratify into the format doc + validator:

```jsonc
"gait": {
  "speed": 1.4,          // m/s ground speed
  "stepLength": 0.62,    // m/step — film.rs walk_phase's `stride`
  "dutyFactor": 0.62,    // stance fraction; < 0.5 = run (flight)
  "stepHeight": 0.055,   // m, swing clearance
  "bobAmp": 5.0,         // stage px — the film's walk_bob amp values
  "armAmp": 16.0,        // deg of shoulder swing (arm.0 channels)
  "armBaseL": [10, 6],   // resting (shoulder, elbow) deg = armL pair
  "armBaseR": [16, -6],  // = armR pair (the film's walk defaults)
  "runFactor": 0.0,      // 0..1 walk<->run blend (overrides the
                         // duty/stepHeight/armAmp trio when present)
  "flightRise": 0.0      // run ballistic rise, drop-fraction units
}
```

CamelCase follows the state block's existing spellings (`armL`,
`headTilt`, `stepLength` mirrors §14's `stride` renaming pattern).
A W15-style validator extension (ranges: duty in (0,1),
stepLength > 0, runFactor in [0,1]) is future work for the
format seat — not built here.

## 7. Honest limits

- **No IK, and the painter doesn't consume foot positions.**
  `toonfull` draws its two-segment leg from `phase` alone
  (toonfull.rs:358-364). The module's per-foot trajectories are
  the travel-axis ground truth and the channel source
  (phase/bob/arms); making the painter draw *from* the foot
  positions (true planted-foot rendering) is a painter change,
  out of this seat's scope.
- **Flat ground only.** Feet assume ground at the character's
  stage floor line, exactly as the current engine does (feet
  land on the 1000 stage line, toonfull.rs:363). No terrain
  height sampling, no slope adaptation, no foot roll.
- **No turning, starting, or stopping.** The evaluators are
  steady-state cycles. Starts/stops/turns are *blends the
  staging owns*: crossfade `GaitParams` (or a gait weight)
  over an ease window, and — the rule that prevents foot
  slide — **integrate phase from travelled distance**
  (`phase_for_distance` over an accumulating distance), never
  restart phase at a blend, or L1 breaks at the seam.
- **Run bob keeps the walking law.** The blend changes duty
  factor, clearance, and adds the ballistic flight arc, but the
  support drop remains the inverted-pendulum form; the
  spring-mass inversion of real running (COM lowest at
  mid-stance) is not modelled — consistent with the painter,
  whose Run is an amplitude variant of the same cycle.
- **Idealised cycle.** Positions and lift are continuous
  everywhere; foot *velocity* is discontinuous at strike/toe-off
  (the piecewise stance/swing join), as in any keyframed cycle.
  Stance itself — the interval L1 governs — is exact.
- **Determinism** matches the engine's class: pure functions,
  no state/RNG/clock; `sin` only in the swing lift (last-ulp
  libm caveat, same as film.rs's existing sin evaluators).
