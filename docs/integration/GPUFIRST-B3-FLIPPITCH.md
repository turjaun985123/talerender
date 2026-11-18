# GPU-FIRST BATCH 3 — SEAT FLIPPITCH: flip mirror + bodyPitch hip-pivot seam + B3-CARRY-1

**Branch:** `b3/flippitch` on base `01e3ea8`.
**Code commit:** `b204972` (this record is a follow-up docs commit).
**Gate:** ci.yml run **44244980651** — SUCCESS (build + test:
574 passed / 0 failed, incl. all 10 new `flippitch_tests`);
Tools CI run **44244980381** — SUCCESS (shader/Params pairing
checker accepts the WGSL change). No local cargo was run.

## 1. Composition order as implemented

Outermost first, exactly the TS order (Specs §5d + 3):

```
anchor ∘ flip ∘ lean ∘ pitch ∘ headTilt(effective)
```

Code path (`src/toonfull.rs::paint_toon_full`):

- `canvas_anchor(look, st)` is solved FIRST and its translation
  is applied OUTERMOST (inside `px`, added after the mirror).
- `px` (the leg map): lean about the feet (256, 1000) → flip
  mirror `x -> 512 - x` (only when `st.flip`) → `+ anchor +
  (384, 256)`, × RES.
- `pxu` (upper assembly: bean, pattern, neckwear, arms):
  `pitch_pt` about the hip `(bx, hip_y)` first, then `px`.
- `hpt` (head): squash about the head centre → effective tilt
  about the neck pivot → `pitch_pt` about the hip → `px`.
- Legs paint through `px` only — never pitched (Spec 3).

Ellipse rotation angles (passed explicitly by this painter,
unlike the TS canvas transform) follow the same stack: a stage
angle is negated under the mirror (`sgn = -1` when flipped) and
pitch adds to upper angles — `th_leg = sgn·lean`,
`th_up = sgn·(lean + pitch)`,
`head_rot = sgn·(lean + tilt_eff + pitch)`; intrinsic angles
(highlight 0.2, cap −0.05 / +0.05, mailcap 0.04, bow-lock
∓0.2) are multiplied by `sgn` at their call sites.

## 2. Flip (§5d)

- Mirror in the point maps at the composition point above;
  at `flip == false` the branch is skipped and the maps are the
  pre-port maps bit-for-bit.
- **canvas_anchor interaction (the §5d obligation):** the
  anchor's envelope points get the SAME final mirror
  (`flip_x(qx) = 512 - qx` in `push!` / `push_leg!` /
  `push_head!`), i.e. the anchor is solved in mirrored space,
  and the painter then applies that translation outside the
  mirror — the TS arrangement (`translate(anchor)` before
  `scale(-1, 1)`). Measured consequence, unit-tested:
  flipping the anchor-engaging lying-lean state negates `ax`
  (|fx + ax| < 1e-3) and keeps `ay` (canvas bounds are
  symmetric about stage x=256: [−380, 892]).
- Returned hands are NOT mirrored (TS returns pre-flip stage
  hands; only pitch is folded into them — §3 below).

## 3. bodyPitch (Spec 3)

- Host-side transform, per the binding B2-CARRY design note —
  no per-prim painter change: the seam is the `pxu` / `hpt`
  point maps plus `pitch_pt` (the TS `rotPt`) and
  `effective_head_tilt` helpers.
- Counter-pitch: the head painter receives
  `tilt = headTilt − bodyPitch · bodyPitchCounter`
  (counter default 1.0, TS `?? 1`, plumbed in Batch 2 and
  verified, not re-plumbed). Net head world tilt
  `headTilt + pitch·(1 − counter)`.
- `canvas_anchor` models the pitch: upper/head envelope
  points rotate about `(bx, hip_y)` between the effective-tilt
  and lean steps; leg points never rotate.
- Returned hands are the pitched stage positions
  (`pitch_pt` of the un-pitched hand); the hip pivot is
  pitch-invariant.
- At pitch 0 `pitch_pt` returns its input exactly and the
  effective tilt equals `head_tilt`, so the seam emits no
  transform — byte-identity at 0 (below).

## 4. B3-CARRY-1 — CompositePlate inverse map

- Prim (`src/gputrace.rs`): `CompositePlate` gains
  `flip: bool`, `pitch_rad: f32`, `hip: (f32, f32)`. JSON is
  additive (`flip` / `pitch_rad` / `hip` keys); pre-Batch-3
  lists without them parse at the identity defaults
  (false / 0 / rest hip (256, 574)) — unit-tested.
- CPU (`toonfull::composite_plate`): forward map pitches the
  plate rect about the hip, leans, mirrors; the per-pixel
  inverse undoes anchor → flip → lean → pitch. `hand` stays
  the UN-pitched stage hand (the plate's figure-local origin,
  as TS held-prop callbacks receive it). Both added steps are
  flag-guarded, so default sampling is the old arithmetic.
- GPU prep (`gpupaint.rs`): the spare slots Batch 2 reserved —
  `p3 = (sin pitch, cos pitch, hip.x, hip.y)`,
  `u0.z = flip flag`, `u0.w = pitched flag`. The one existing
  golden-params case was re-pinned for the p3 change (its prim
  is flip=false / pitch=0, so only p3 moved: (0, 1, 256, 574));
  a new prep test asserts the encoding + mirrored bbox for a
  flip+pitch plate.
- WGSL (`gpupaint.wgsl::main_plate`): inverse undoes the flip
  (`ux = 512 - ux`) and the pitch (rotate by −pitch about the
  hip from p3) behind the u0 flags, so default lists compute
  exactly the pre-Batch-3 values.
- **Proving battery cases** (in the appended spread, §5):
  `flip-plate-holdr-label`, `pitch-plate-holdr-p60`,
  `flippitch-plate-holdr-p60` (+ `flip-plate-holdl-lean20`).
  CPU-side direct proofs (default build): a synthetic
  red/blue half plate samples the OPPOSITE half at the same
  stage point under flip, and lands at the pitch-rotated
  stage points (230, 568) / (230, 580) under pitch +90° —
  `flippitch_tests::plate_sampling_mirrors_under_flip` /
  `plate_sampling_rotates_under_pitch`, both PASS in CI.
  CPU-vs-GPU replay of these cases is the post-integration
  parity run's job (gpufirst-parity.yml triggers on the
  integration branch, per its own header).

## 5. Battery states added

New module `src/gpupaint_states_flippitch.rs`, appended LAST
in `state_spread()` — the 221-state Batch-2 baseline is
undisturbed, in order. 16 new states (237 total), rollup
class `flip-pitch`:

`flip-neutral`, `flip-happy-bob14`, `flip-neutral-lean9`,
`flip-suspicious-slit`, `flip-anchor-lying82`,
`pitch-60-c100`, `pitch-60-c050`, `pitch-60-c000`,
`pitch-neg30`, `pitch-crawl62-walk`, `pitch-kid-60`,
`flippitch-60-lean9`, `flip-plate-holdr-label`,
`flip-plate-holdl-lean20`, `pitch-plate-holdr-p60`,
`flippitch-plate-holdr-p60`.

gpupaint expressibility: flip/pitch are host transforms —
recorded prim geometry arrives already mirrored / pitched,
so the executor receives the same transformed geometry it
always did; the only executor-visible change is the plate
prim's new params (§4). No new prim kinds, no format break.

## 6. Acceptance numbers (CI run 44244980651, all PASS)

| Spec test | Result |
|---|---|
| Defaults byte-identity: neutral paint == pre-port pinned hash `d413603252323a7c` under flip=false, pitch 0, pitch 0 + counter {1, 0, 0.37} | PASS (`defaults_reproduce_pre_port_hash`); the 5 carry-plumbing pre-plumbing hashes + brow/sweat pins also still PASS |
| §5d S12: {neutral, happy} × flip × bob {0, 14} × lean {0, 9} | **16 / 16 distinct classes** (pre-port 8) |
| Flip mirror measurement: flipped paint vs device-mirror of unflipped (asymmetric Suspicious state) | PASS inside the asserted bound **< 7,200 differing px (< 0.1% of 7,208,960)** — the exact count is in the test's captured stdout (cargo hides passing-test stdout in CI logs); the bound is the parity statement: a missed mirror on any part would exceed it by orders of magnitude |
| Spec 3 counter-tilt: headTilt 10, pitch 60 | effective tilt **−50° @ counter 1, −20° @ 0.5, +10° @ 0** — exact |
| Spec 3 hands | returned hands == `pitch_pt` of un-pitched hands (< 1e-3); hip pivot invariant |
| Spec 3 legs @ pitch 60 | bottom strip (device rows ≥ 2400) **byte-identical** to pitch 0; full frames differ |
| Spec 3 envelope | crawl range 55–70°: anchor == (0, 0) at adult + kid; sweep −90..180 step 15: finite everywhere |
| Flip anchor | ax negates (< 1e-3), ay unchanged |
| B3-CARRY-1 CPU plate proofs | flip: opposite-half sampling PASS; pitch 90°: halves at the rotated points PASS |

**Parity differences: none found.** Every default-state pin
reproduces byte-identically; no case is left CPU-only by this
seat. The one quantified non-identity is the flip-mirror
measurement above (mirrored flattening rounds differently —
an intra-CPU rasterisation artefact, bounded < 0.1%, not a
CPU-vs-GPU divergence).

## 7. Not touched (per brief)

Wing / ears / hair painters, film-path wiring, telemetry,
`state_key` hold keying (F-V1), and dizzy (SUSPENDED — the
Mood::Dizzy arm is untouched; no dizzy-specific work).
Zero new dependencies. No rendered images committed.
