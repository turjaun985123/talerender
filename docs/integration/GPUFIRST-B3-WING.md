# GPUFIRST BATCH 3 — SEAT PORT-WING RECORD (Spec 1 wing painter port)

**Branch:** `b3/wing` on base `01e3ea8`.
**Commits:** `786875a` (the port) + `731b25a` (test-only sample fix,
see §5) + this record.
**Scope:** Part 3a only — the wing evaluator + panel painter.
Ears / hair / flip / bodyPitch are other seats; film path,
telemetry and `state_key` untouched; dizzy (D-5F) untouched.

## 1. What landed

### CPU painter (`src/toonfull.rs`)

- **Evaluator** — a port of `secondary.ts` `resolveWing` /
  `wingFlap` / `wingChain` / `wingPose` plus the spring kernels
  it builds on (`makeSpring`, `springFromAppendage`,
  `springStepAt`, `springRampAt`, `springTrack`,
  `springResponse`, `shapeChain`), in **f64** end to end so the
  TS proof values reproduce at TS tolerances. API:
  `WingMotion` (speed / phase driver closures), `WingParams`
  (all-optional; resolved defaults fold 1, foldBase 1.25,
  spreadBase 1.5, foldBend 2.0, spreadBend 0.18, flapAmp 0.5,
  flapLag 0.6, speedRef 1.6, speedSpread 0.65, idleAmp 0.03,
  idleFreq 0.25, maxBend 2.2; spring = explicit (freq, decay),
  else the def's (stiffness, damping) via
  `springFromAppendage`, else house 2.2 Hz / 3.5), `WingFold`
  (value, or function-of-t spring-tracked), `wing_flap` /
  `wing_chain` / `wing_pose`. It is a staging API in the
  `charanim` posture: the painter consumes evaluated state
  (`wing_segs` / `wing_a`) like every other channel; no film
  staging site calls the evaluator yet.
- **Geometry** — `wing_chain_geom` (toon.ts `wingChainGeom`),
  shared by the anchor and both painters: segment count =
  `wing_segs.len()` when the state carries it (any declared
  count), else the scalar fallback family
  `[1.25 + wing_a, 1.25 + wing_a − 2.0]` over 2 segments;
  `wing_segs` wins over `wing_a`. Lengths: `wing_lens[k]`, else
  58 then −12/segment floor 24; widths 46 then −12 floor 20.
  Near root `(bx + 46, top + 96)`; far root `(bx − 38, top + 84)`;
  joints advance `(sin a · len, cos a · len)`; the far chain
  mirrors angles (`a → −a`) and scales lens/widths ×0.88.
- **Painter** — `paint_wing_chain` (toon.ts `paintWingChain`):
  one filled ellipse per segment per side, centred on the
  segment midpoint, long axis along the segment, radii
  `(len/2 + 7, width/2)`, ink stroke lw 7 near / 6 far. Near
  fill = wing colour; far fill = `shadeOf(col, 0.72)`. Near
  only: feather line per segment (`shadeOf(col, 0.75)`, lw 4,
  quadratic through the offset midpoint) and joint line per
  internal joint (`shadeOf(col, 0.7)`, lw 3). Colour resolution:
  `wingCol || body` (an empty `wingCol` string falls back too,
  as in JS). `shadeOf` maps through this painter's established
  `shade_rgba` — the same mapping the bean-shade / stripes /
  scarf ports use.
- **Composition vs the body (draw order):** legs → **far wing**
  → body bean + pattern + shade + neckwear → **near wing** →
  arms (+ held plates) → head. The far wing is therefore
  occluded by the body; the near wing paints over the body and
  under the arms — the TS paint sites (far inside the body
  block just before the bean; near after neckwear, before the
  arms). TS paints both wings inside the bodyPitch sandwich;
  this painter has no pitch transform yet (other seat), so the
  wings ride the plain stage→canvas map (`px`: lean + canvas
  anchor) like every other body-space prim here. Panel rotation
  is computed from the mapped segment endpoints, so lean
  composes exactly.
- **Anchor interaction:** `canvas_anchor` now includes both
  wing chains' points (radius = half-width + 3.5) — the Spec 1
  port-time obligation from GPUFIRST-B2-CARRY §4 — pushed
  through the same lean-only map as the other body points.
  Wing states that fit keep anchor (0, 0); an oversized
  declared wing (lens 400) engages the anchor (asserted in
  tests). Wing OFF adds no envelope points.
- **Wing OFF is byte-identical:** the painter and anchor gate
  on `look.wing`; the pre-plumbing pinned hashes (carry ×5,
  brow ×8) are unchanged tests and pass, and a dedicated test
  pins the pre-port neutral hash `0xd413603252323a7c` for a
  look with the wing flag OFF but wing state set.

### gpupaint (`src/gpupaint_states_wing.rs`, wiring in
`gpupaint_states.rs` + `main.rs`)

No new Prim kind — per the B2-CARRY §2 verdict the wing is
expressible in existing kinds, and the trace confirms what
records: panels = `FillEllipse` + the flattened
`StrokePolyline` of `stroke_ellipse`; feather lines = the
flattened `StrokePolyline` of `stroke_quad`; joint lines =
`StrokePolyline`. All geometry is host-side device space, list
order preserves the far/body/near layering.

**Battery additions (APPEND-ONLY; the Batch 2 baseline spread
of 214 states — 77 base + 5 brow + 71 shape + 61 stroke — is
untouched; spread total now 222):**

| ID | Contents |
|---|---|
| `wing-pip-folded-rest` | reference bird (wingCol `#7FB6D9`, lens [58, 46]), scalar fallback at rest |
| `wing-pip-spread` | evaluator spread chain [1.5, 1.68], 2 segments |
| `wing-segs1` | 1 segment, lens [58] |
| `wing-segs3` | 3 segments [1.5, 1.68, 1.86], lens [58, 46, 34] |
| `wing-segs5` | 5 segments [1.5 … 2.22], lens [58, 46, 34, 24, 24] |
| `wing-a-fallback-08` | scalar fallback, wingA 0.8 |
| `wing-kid-folded` | kid look, folded fallback |
| `wing-lean20-spread` | spread chain under lean 20 |

`rollup_class` maps `wing-*` → `wing-panels` (the parity
compare *tool* has its own grouping table and filed these
under `transforms` in run 44246387185's summary — coordinator
may want the tool's table updated; out of this seat's scope).

## 2. Acceptance (Spec 1) — expected vs measured

All as CI-runnable tests (`toonfull::wing_tests`, default
build; run 44245797020, 570 passed / 0 failed):

| Check | TS expected | Rust measured |
|---|---|---|
| Rest chain (fold 1, idleAmp 0) | `[1.25, −0.75, −2.75]` exact | exact (assert_eq) |
| Spread rest | `[1.5, 1.68]` ±1e-12 | within 1e-12 |
| Half-fold base / bend | 1.375 / −0.91 ±1e-12 | within 1e-12 |
| Tip reach, unit lens | 1.08 folded / 1.99 spread | 1.0806 / 1.9919 (asserted ±0.01; ratio < 0.6) |
| Per-joint bend diff | 2.18 rad | within 1e-12 |
| `wingFlap` at spread | `0.5·sin(1.2)` ±1e-12 | within 1e-12 |
| Flap at full fold / no phase | exactly 0 | exactly 0 |
| `wingPose.flap == wingFlap` | ±1e-12 | within 1e-12 |
| Seg flap lag | seg1 = 1.68 + 0.5·sin(0.6) ±1e-12 | within 1e-12 |
| effFold at speed = speedRef | 0.35 ±1e-9 | within 1e-9 (def-spring variant ±1e-6 at t=60) |
| Fold function spring-tracked | mid-flight ∈ (0.05, 0.95) at t=1.1; settled ≈ 0 at t=30 | both hold |
| Counts 1 / 2 / 3 / 5 | exactly n panels per side | geom pts = n+1 per side; paints for 1/2/3/5 pairwise pixel-distinct and ≠ wingless; prim-exact counts in the feature trace test (see §4 caveat) |
| Far wing behind body, 0.72 shade ×0.88 | occluded centre; shade crescent | far panel-0 centre samples the body colour (232,115,74); long-lens look: >100 px of exact `shadeOf(#7FB6D9, 0.72)`, 0 in wingless |
| Near panel centre | wing colour | (127,182,217) sampled at panel-0 centre, pose [−1.2, −1.0] |
| Reference bird at rest (acc. 4) | folded 2-panel wing protruding past body edge + far crescent | protrusion point (348.8, 441.5) = wing colour winged / transparent wingless; near fill > 500 px |
| Wingless non-regression (acc. 3) | byte-identical | pinned hash `0xd413603252323a7c` reproduced with wing state set + flag off; all pre-existing pins (carry/brow/sweat) pass |
| Anchor | both chains enveloped | lens-400 wing engages anchor (ax < 0); same state wingless = (0, 0); ordinary wing states = (0, 0) |

## 3. Parity (GPU vs CPU, CI lavapipe)

GPUFIRST Parity run **44246387185** (dispatched on `b3/wing`
@ `731b25a`): **268/268 cases byte-identical, 0 differing
pixels** — 46 battery cards + 222 states. The 8 wing states
individually: identical = Y, diff_px = 0, max |Δ| = 0, PSNR
inf/inf. Per-prim-kind: `fill_ellipse` 229/229 cases,
`stroke_polyline` 229/229. **No parity difference found** —
no wing case stays CPU under the parity law on this evidence
(lavapipe; RADV is the user's-PC check, as always).
Summary artifact copied to
`~/workspace/talerender-evidence/gpufirst-b3/wing/parity-summary-44246387185.md`.

## 4. CI verdicts

- `ci.yml` run **44245108190** @ `786875a`: Build ✓, **Test ✗**
  (569 passed, 1 failed — §5).
- `ci.yml` run **44245797020** @ `731b25a`: **SUCCESS** — all
  steps ✓, **570 passed / 0 failed**.
- GPUFIRST Parity run **44246387185** @ `731b25a`: **SUCCESS**
  (§3).

**Caveat (feature test compile):** no workflow in this repo
runs `cargo test --features gpupaint` (ci.yml and
gpufirst-perf.yml test default features only; the parity
workflow builds the feature without cfg(test)). The battery's
non-test code is proven by the parity run (it recorded and
replayed all 8 wing states); the feature-gated trace test
`wing_trace_panel_counts` (exact prim counts: FillEllipse
delta 2n, near fills = wingCol ×n, far fills = 0.72-shade ×n,
StrokePolyline delta 4n−1 for n ∈ {1, 3, 5}) was audited
line-by-line against the gputrace/scene APIs but has **not
been executed by any CI run available to this seat** — first
execution is the coordinator's integration (or a local
`cargo test --features gpupaint wing_trace`).

## 5. The one red run — root cause

Run 44245108190's single failure was a **test-sample defect,
not a painter defect**: the reference-bird protrusion sample
sat exactly on joint 1, where `paintWingChain` (correctly,
per TS) draws the shade joint line across the panel; the
measured pixel was the joint-shade blend (108, 155, 185) over
the fill (127, 182, 217). Fix `731b25a` moved the sample to
85% along panel 0 (past the bean edge + ink stroke, clear of
the joint/feather lines and the arm). Painter code unchanged
between the red and green runs.

## 6. Representation note

`wing_segs` is plumbed as `Vec<f32>` (Batch 2), so an empty
vec is indistinguishable from absent state: a scene file with
`"wingSegs": []` falls back to the 2-segment scalar family
here, where TS (array present, length 0) would paint no
panels. Same convention the plumbed `hair_segs` carries;
changing it needs a presence bit in the Batch-2 plumbing,
which was out of this seat's scope.
