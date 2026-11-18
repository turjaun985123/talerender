# GPUFIRST-B2-RECORD — GPU-first Batch 2 close record

Batch 2 of the GPU-first TaleRender programme (user order
2026-11-16 23:59 +06: 30 agents, code first; Batch 1 closed with
parity run #1 — 77/82 byte-identical, 6 diff px, determinism
14/14). Batch 2 scope: per-class GPU paint with measured parity,
root-cause the five run-#1 diffs, and disposition the five
RUST-PARITY-SPECS handed over from the v7 programme (2026-11-17).

**Staffing (actual): coordinator + 8 seats + 1 continuation seat
(VERIFY-2, after the original VERIFY session ended post-Phase-1)
= 10 agents.** Programme total so far: Batch 1 ≈ 13 + Batch 2 10
= ≈ 23 of the 30 ordered; Batch 3 inherits the remainder (≈ 7–8).

**Heads:** base `f4b358d` (Batch 1 close) → seat branches merged
in dependency order (NUMERICS, EDGE, CARRY, BROW, SWEAT,
SHAPE-BATT, STROKE-BATT) → integration `6aa2142` → module-gate fix
`3939698`. Production default UNCHANGED: CPU painter; gpupaint
remains behind the `gpupaint` cargo feature (empty, off by
default). Zero new dependencies (manifest still exactly 3 direct
deps). No rendered images in this repo; evidence lives in
`~/workspace/talerender-evidence/gpufirst-b2/`.

## Closing battery — parity run #2 (CI 44025547389 @ 6aa2142)

**221/221 cases byte-identical, 0 differing pixels, determinism
21/21, FRINGE 0 everywhere.** Baseline run #1 was 77/82, 6 px,
14/14 — improved on every axis, no regressions. Full table:
`GPUFIRST-PARITY.md` § "CI results — run #2".

### The five run-#1 diffs

All five (legs-walk-p3, toon-head-outline, toon-card,
xf-campitch-s01, xf-headtilt-pos6-think) are byte-identical in
run #2. Root cause, bit-proven by seat NUMERICS: `acos` in the
cap-fan `fan_sd` (lavapipe acos deviates up to 3,861 ulp from
Rust libm; the neighbouring sqrt/hypot/dot/cos are bit-exact).
Fix: one guarded Newton step on cos(φ) = cosang in
`gpupaint.wgsl` (+21/−1 lines, shader only). Seat EDGE's measured
CONDITIONAL bound for legs-walk-p3 (≤ 2 px, alpha-only, ΔA ≤ 16,
ΔRGB = 0, EDGE zone; `GPUFIRST-B2-EDGE.md`) is superseded by the
fix. The new batteries' local fan-family findings (F-STROKE-1/2/3:
7 px; F-SHAPE-1/2/3: 5 px, all against the stock shader) resolved
under the same fix — 0 diff px across all 132 new states on CI.

## Per-class parity table (run #2, CI lavapipe)

| class | battery | identical/total | worst case |
|---|---|---:|---|
| Path fills (fill_polygon, fill_rect) | prims + shapes fills 10 | all cases | none — 0 diff px |
| Strokes (round caps/joins; stroke_polyline 2,371 instances) | STROKE battery 61 (laugh-quirk grid, 8-phase gait sweeps, sit legs, arm FK extremes, mood sweep, ellipse rings) | 61/61 | none — 0 diff px (local fan findings F-STROKE-1/2/3 resolved by the acos fix) |
| Ellipses / arcs (fill_ellipse 1,675 instances) | shapes ellipses-arcs 12 + prims | all cases | none |
| Mood features (brows/mouths/eyes) | mood-features 34 + brow 5 + stroke mood sweep 14 | all cases | none |
| Sweat / tears (fx) | fx states incl. new fx-sweat-050 / fx-sweat-099 | all cases | none |
| Shade + masks (head_shade 220, masked prims) | shapes shade+masks 39 (head_shade 27-state grid, mask-edge sweep, masked mouths) | 39/39 | none — the inventory's flagged binary-mask-edge risk did not materialise (6/6 mask edges, 6/6 masked mouths identical) |
| Held plates (composite_plate 13/13, masked_blend 13/13) | plates 3 + shapes plates+anchor 10 | 13/13 cases | none |
| canvas_anchor interplay | anchor 2 + plates+anchor 10 (lean ±82, sit_drop 900, anchor engagement asserted per state) | 12/12 | none |
| Gradients | — | n/a | class formally closed: this painter has NO gradient prim; shading surface = head_shade + masked shade/highlight ellipses (at parity above) |

No class is declared GPU-ready beyond these numbers: parity is
proven for the recorded-state replay surface on CI lavapipe.
Production integration (film path) is Batch 3, behind the flag;
the production default stays CPU until a separate flip decision.

## Parity-spec dispositions (RUST-PARITY-SPECS.md numbering)

The specs describe TS-painter behaviour the Rust CPU painter
itself lacked (2,152-cell FACE_TABLE replay: 99.71% of relations
held). Binding constraint honoured: the gpupaint input surface
now expresses every spec channel — no gap is baked in.

| spec | disposition | evidence |
|---|---|---|
| §5a browLift / browTilt (Spec 1, largest family) | **PORT-NOW — DONE** (seat BROW, `74c7192`) | Fields through FullState + Default, painter (offsets applied after per-side mood overrides; no clamp/quantisation), loader keys `browLift`/`browTilt`, filmtelemetry mirror, resolve_chars copy, state_key exact bits. Acceptance: the 304-cell grid (19 moods × lift {0,−8,0.5,14} × tilt {0,−10,0.5,10}) refines to **304/304 distinct pixel classes** (pre-port: 19). 8 pre-change pinned hashes reproduce byte-identically. GPU parity: 5/5 new brow states byte-identical in run #2. |
| §5e sweat gradation (Spec 2) | **PORT-NOW — DONE** (seat SWEAT, `ef6366e`) | `FullState.sweat: bool → Option<f32>` end-to-end (painter trigger + alpha = min(1,k) on fill and stroke; loader: `true`→Some(1.0), number→Some(v), absent/false/null→None; filmtelemetry + charstate mirrors; state_key keys presence + exact bits). Acceptance: the 24-cell S11 grid refines to **exactly the spec's 16 pixel classes** (worried+0 is its own alpha-0 class). 3 pinned hashes byte-identical. GPU parity byte-identical on all fx sweat states incl. 0.50/0.99 (trace alpha 255/252/127). |
| Spec 1 wing painter (file §1) | **DESIGN-CARRY** (seat CARRY, `50e64a6`) | Plumbed: FullLook `wing` (bool), `wing_col` (Option<String>), `wing_lens` (Vec<f32>); FullState `wing_segs` (Vec<f32>), `wing_a`; loader keys + resolve_chars copies + full state_key coverage. Painter deferred: the wing evaluator + panel painter is a standalone port; channels are expressible in existing Prim kinds (see GPUFIRST-B2-CARRY.md). |
| §5b fox ears (Spec 1) | **DESIGN-CARRY** | Plumbed: FullLook `ears` (bool; `"fox"`→true); FullState `ear_a`, `ear_al`/`ear_ar` (Option<f32>); keys + copies + state_key. Painter (ear triangles + anchor envelope) deferred with the port. |
| §5c hair (Spec 1) | **DESIGN-CARRY** | Plumbed: FullLook `hair` (bool; `hair_col` pre-existed); FullState `hair_segs` (Vec<f32>); keys + copies + state_key. Painter deferred; TS gives hair NO canvas envelope and the port must mirror that. |
| §5d flip (Spec 1) | **DESIGN-CARRY** | Plumbed: FullState `flip` (bool) + loader/resolve/state_key. Mirror transform deferred; composition order recorded (after anchor, before lean). Named gap **B3-CARRY-1** (below). |
| §3a bodyPitch (Spec 5) | **DESIGN-CARRY** | Plumbed: FullState `body_pitch`, `body_pitch_counter` (default 1.0, TS `?? 1`); keys + copies + state_key. Painter deferred: hip-pivot seam is the riskiest painter change (host-transform design note in GPUFIRST-B2-CARRY.md is binding for the port). |
| Spec 4 blink layer (ambient cadence, lead frames, swap-masking) | **DEFER** | Evaluator/staging layer, not paint: blink already reaches the painter as a key-covered scalar state, so the gpupaint input surface needs nothing. The port belongs with the staging/evaluator work, not the paint batch. |
| Spec 5 shortest-arc phase blending | **DEFER** | Evaluator layer (`eval15::sample_scalar_keys`); phase is already a FullState field, so the blend is invisible to paint inputs. Same reasoning as blink. |
| §3b F-1 (repaint key exactness) | **NO PORT** | Verified present in Rust: state_key keys exact f32 bits (Batch 2's composed key preserves this for all 30 FullState fields). |
| §5f "dizzy" (D-5F) | **SUSPENDED** | Per the hard rule: no winner declared, not implemented, not approximated. Its 362 EQ pairs / 8 TS classes stay suspended from acceptance bars pending the user's side-by-side ruling. |
| §5g deadpan micro-merges | documented-limitation track | Sub-pixel raster-resolution family; if it surfaces in future batteries it is reported as measured bounds, never chased or silently absorbed. |

CARRY safety evidence: 5 pre-change pinned full-frame hashes
reproduce byte-identically at the new defaults; a key-coverage
test varies each of the 9 CARRY state fields alone and yields
distinct keys; loader tests green under both feature sets.
BROW/SWEAT film-safety: no film staging site assigns the new
fields (argued from code + census below); SWEAT additionally
proved no in-repo scene JSON carries a `"sweat"` key and the one
Worried staging (Suzuki) maps identically under the new trigger.

**Downstream obligation (from the v7 programme's fixture rule):**
the 2,152-cell replay fixtures are pinned against the pre-port
binary. The BROW and SWEAT ports change the CPU painter, so the
v7 side must regenerate the fixtures against the post-Batch-2
binary before the brow/sweat cells can be claimed against the
new painter.

## Other closing gates

- **CI (build + test) @ 3939698: SUCCESS** (run 44029485464,
  watched to completion). At `6aa2142` the Test step had failed
  on the ungated battery modules (E0432/E0433, default-features
  test compile only); `3939698` fixed it and the full suite is
  green at the final HEAD. Tools CI @ 6aa2142: SUCCESS (run
  44025547348). Perf @ 6aa2142 (run 44025547359): failed on the
  same Test-step root cause; Perf is dispatch-only and was not
  re-run at `3939698` — its failing step is the identical
  default-features Test step that CI now passes.
- **Onoda census @ 6aa2142 (run 44025611760): UNVERIFIED —
  blocked by a GitHub API rate limit (HTTP 403, hit 2026-11-17
  06:26 UTC while watching).** Last observed state: the run was
  in its "Full-film census (chunked renders)" step with the film
  telemetry step already ✓. The census verdict (expected
  IDENTICAL 1811/1811 vs the post-port census; BROW/SWEAT/CARRY
  argued film-inert from code, and CI's green suite includes the
  telemetry tests) must be read from the finished run before
  anyone claims it. Follow-up for the parent: read run
  44025611760's conclusion + census comparison step, then update
  this section.
  **[CORRECTION, Batch 3 close 2026-11-18: READ — run
  44025611760 completed SUCCESS; telemetry FILM TELEMETRY:
  CHECK OK; all 1,811 extracted frame hashes IDENTICAL to the
  flip-era census (file sha256 ce08dff5…). The UNVERIFIED /
  UNCONFIRMED wording above and in the VERIFY Phase 2 bullet
  below is superseded by this read; see COVERAGE-MATRIX.md
  Group 9, post-Batch-2 census row.]**
- **Golden pins:** untouched by this batch (no painter change
  reaches any pinned film frame; BROW/SWEAT defaults reproduce
  pinned hashes byte-identically — see dispositions).
- **VERIFY Phase 2 (seat VERIFY-2, report
  `~/workspace/talerender-evidence/gpufirst-b2/verify/PHASE2-INTEGRATED-6aa2142.md`):**
  laws re-check PASS (3 deps; feature empty/off; film.rs and
  scenefile.rs contain ZERO gpupaint/gputrace references;
  production paint is `paint_toon_full` only); **state_key
  coverage PASS — all 30 FullState fields keyed exactly once, no
  missing, no duplicated field; F-V1 NOT weakened**; evidence
  hygiene CLEAN (zero images in the 15-file integration diff);
  parity independently confirmed from the artifact (221/221,
  0 px, determinism 21/21, `missing_named` empty — G1 closed by
  evidence; all five run-#1 cases individually identical). Its
  one BLOCKER, **F-V2-1** — the ungated shapes/strokes modules
  breaking the default-features test build at `6aa2142` (the same
  failure as the CI bullet above; consequence: the suite never
  executed at `6aa2142`, so the brow/sweat/state_key tests' green
  verdict rests on the `3939698` CI run) — was already fixed by
  `3939698` before the report landed, and CI @ `3939698` is
  SUCCESS. Its census read hit the same rate limit: UNCONFIRMED,
  corroborating the bullet above. Phase 1 (baseline @ f4b358d)
  PASSed all laws; F-V1 and gaps G2/G5 carry to Batch 3 (below).

## What Batch 3 inherits

1. **Film-path integration** behind the `gpupaint` flag (the
   batch's main event): wire the GPU replay into the film path,
   measure on the per-frame-sampled pose profile (unique-state
   counts ~an order higher than staged poses), then bring the
   flip decision to the user with numbers. Production default
   stays CPU until that decision.
2. **B3-CARRY-1** (named format gap): `CompositePlate`'s inverse
   map carries lean+anchor only, so flip/bodyPitch cannot reach
   held-plate sampling. Needs a WGSL change that must land WITH
   the painter ports — nothing mis-renders today (no painter
   consumes the channels yet).
3. **DESIGN-CARRY painter ports:** wing (evaluator + panels),
   ears, hair, flip mirror transform, bodyPitch hip-pivot seam —
   each with its recorded canvas_anchor obligations (wing chains,
   ear triangles, pitch rotation, flip mirror; hair: no envelope,
   mirroring TS).
4. **F-V1** (VERIFY Phase 1, low, non-live): hold_l/hold_r are
   keyed by prop NAME, not plate pixels, in state_key. Safe in
   the only cached path today; must not weaken when Batch 3
   touches the key or the cache.
5. **Determinism harness gaps G2/G5** (VERIFY Phase 1): the
   subset records rather than gates, and can silently shrink via
   missing_named. The 21/21 in run #2 was confirmed against the
   artifact rows, not the workflow's exit code alone.
6. **D-5F "dizzy"** — still suspended; needs the user's
   side-by-side ruling before anyone implements either side.
7. **Fixture regeneration** on the v7 side for brow/sweat cells
   (above).
8. RADV (real-hardware) GPU numbers: all parity figures in this
   record are CI lavapipe. A PC session on the user's Vega 7 /
   RADV remains the place for hardware wall-time numbers; none
   are claimed here.
