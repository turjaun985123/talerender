# Risk Register

Ranked by expected damage to parity × likelihood, from the Milestone 0
audit evidence ([docs/audit/](audit/)). Each risk names its mitigation
and where it is enforced ([ADR-001](ADR-001-talerender-architecture.md),
[compatibility-contract.md](compatibility-contract.md)). Status is as of
the M0 gate (2026-11-13): no mitigations are complete yet; several tests
are scheduled as milestone gates.

**Refresh — 2026-11-14 (wave 2, seat W08).** Every status below
was re-verified against the repo record (milestone packages in
docs/ab/, docs/integration/FINAL-REPORT.md, git log). The M0 text
is preserved verbatim; each risk carries a dated refresh line.
Summary: **closed** — R1, R2, R4, R6, R8. **Mitigated with named
residuals** — R3, R7, R9. **Partially mitigated, still open** —
R5. **Still open** — R10. Watchlist: colour working space and
held-prop closures closed; grain and points/stars remain open.
Full claim-by-claim record: docs/integration/DOC-SWEEP-W08.md.

## R1 — BokehPass depth-of-field parity · CRITICAL

**Evidence:** a1 §2.3, Risk #1. **Status:** open; mitigation scheduled.
The DoF is one specific, physically incorrect fullscreen shader whose
artefacts the house look is art-directed around; it runs by default on
essentially every frame; its blur depends on a float depth texture
reconstructed across a 0.05–800 camera range. The earlier headless-gl
spike's only *structural* A/B failures were in exactly this pass.
A "better" DoF fails parity by definition.
**Mitigation:** transcribe the r170 BokehShader arithmetic into WGSL
(contract §1.1); DoF chart test **before** the first real scene
(contract §1.2, M3 gate).
**Refresh 2026-11-14: CLOSED.** The M3 gate ran as specified and
PASSED (commit b0de44e; docs/ab/dof-chart/README.md): step-edge
blur widths within 0.05 px (checkerboard median 0.12 px) at all
ten chart depths against a tolerance of max(0.75 px, 15%);
full-frame PSNR 47.38 dB. The one divergence found was in
packed-depth *storage*, not the shader arithmetic: the shipped
default stores depth packing in Rgba8Unorm (lossless); the
HalfFloat variant's RADV rounding leg remains open under R10.

## R2 — Ink stroke rasterisation changes the characters · CRITICAL

**Evidence:** a3 §5.1 (supported by §1.1–1.2, §5.3–5.4). **Status:** open.
Every character is stroked paths with round caps/joins at fixed widths
(legs 15, arms 14, outlines 9–10, brows 12 units). A rasterizer that
flattens curves or anti-aliases stroke edges differently changes
perceived line weight on every limb, face and silhouette in every
frame — it reads instantly as a different drawing style and cannot be
fixed per-character.
**Mitigation:** painter treated as the fidelity-critical component
(ADR §4); stroke battery test gates M4 (contract §2.2); ellipse, clip +
even-odd shading, and premultiplied-alpha clauses in contract §2.3.
**Refresh 2026-11-14: CLOSED.** The M4 stroke battery PASSED
46/46 classes (commit 819ee16; docs/ab/stroke-battery/README.md):
full character alpha PSNR 47.88 dB. The decisive discovery was
that Skia's round cap is a flattened-chord fan, not a semicircle;
the native painter reproduces it. Residual items in
docs/ab/stroke-battery/gaps.md are source-engine quirks reproduced
verbatim (e.g. the laugh-mouth outline the source itself never
strokes), not rasterisation divergences.

## R3 — MeshStandardMaterial + PMREM IBL shifts every surface · HIGH

**Evidence:** a1 Risk #2 (§3.4, §2.2). **Status:** open.
One PBR material lit substantially by a PMREM'd RoomEnvironment lights
practically the entire world. Small errors in GGX/env-sampling/PMREM
generation shift every material's brightness and colour balance a few
percent, scene-dependently — "the same film, slightly off", everywhere.
A stock wgpu PBR shader will visibly diverge.
**Mitigation:** PMREM generation and env sampling ported as a
first-class component with a roughness-sweep material chart before any
set is ported (ADR §2).
**Refresh 2026-11-14: MITIGATED — closed for parity at M5, in a
different form than planned.** The composer gate PASSED (commit
79aff03; docs/ab/composer/README.md): PMREM ships as *captured*
real CubeUV data (`room-env-pmrem.f16`) sampled through a verbatim
textureCubeUV port, instead of a native generator. Measured
residual on the most IBL-sensitive surfaces: region ratios
1.002–1.048, no IBL-shaped error (docs/ab/composer/gaps.md item
1); stage ablation reached 23.850 dB. A native PMREM generator
remains future work, explicitly off the critical path; the
largest remaining regional bias in that frame (trunk sides
6–9 % bright) is a geometry/shadow interaction recorded under
R7, not an env-sampling error.

## R4 — A frames-only renderer silently deletes the QA gate · HIGH

**Evidence:** a2 §3.1–3.3, §5. **Status:** open; contract written.
`check`'s verdict rests on painter-internal telemetry (overflow counts,
continuity signatures, projected plane boxes) and threshold lattices —
not on pixels. A renderer that reproduces only frames removes the gate
without any visible failure, and the pipeline loses its overflow,
continuity and caption checks while appearing to work.
**Mitigation:** the telemetry channel is part of the runtime contract
(contract §3); painter exposes overflow counts from its first version
(ADR §5, deferred table).
**Refresh 2026-11-14: CLOSED.** M6 PASSED with *exact* agreement
(commit 9a8cc79; docs/ab/telemetry/README.md): the native
cutouts() transcription matches Chrome on every field in both
stagings. The production form of the gate also exists and has
run over a complete short: `--film-telemetry` (commit df79162)
produced the whole-film verdict recorded in
docs/ab/film/TELEMETRY-W85.md and FINAL-REPORT §4.2 — the gate
was not deleted by the frames path; it caught the 7 caption
failures the frames alone would have hidden.

## R5 — Texture-upload cost and facade texture memory · HIGH (performance)

**Evidence:** a3 §1.0, §2 (full 2560×2816 canvas re-upload on every
state-key change; no dirty-region upload); a5 §6 (street facades ≈
40–90 MB of unique CanvasTextures per scene). **Status:** open.
The source design repaints and re-uploads whole character canvases
whenever the rounded state key changes — at 30 fps with an animated
cast this is the dominant per-frame transfer cost, and on the target's
~6.7 GiB shared-memory APU it competes with the render targets
themselves. Facade memory is a second, static pressure on the same
budget.
**Mitigation:** repaint-key model preserved for behavioural parity
(ADR §4) but upload strategy is a native design point (dirty-region or
atlas uploads are permitted *below* the contract — pixels, not
transfers, are contracted); memory budget enforced in the runtime;
facade streaming decision deferred to M7 (ADR deferred table).
**Refresh 2026-11-14: PARTIALLY MITIGATED — still open.** Landed:
the paint cache is FIFO-capped at 160 entries (commit 1475b66 —
paints are pure, so eviction is pixel-neutral), and the cap is
overridable via `TALERENDER_PAINT_CACHE` (commit e5860b4;
telemetry runs at 24 entries ≈ 700 MB instead of the 4.6 GB
worst case). The repaint trigger itself was hardened: cache keys
are now exact f32 bit patterns (commit cabcc30), replacing the
2-dp-quantised keys whose collisions caused cross-invocation
frame differences. **Not landed:** byte-denominated budget
enforcement (no memory-budget mechanism exists in src/ — the
cap counts entries, not bytes), dirty-region or atlas uploads,
and facade streaming (the ADR's "M7" never existed as a phase;
production scenes paint their textures natively instead). The
core exposure — whole-canvas re-upload on a key change — remains
the design; the cap bounds its memory cost on the target APU
but does not remove the transfer cost.

## R6 — q0.93 JPEG master-generation drift · MEDIUM-HIGH

**Evidence:** a2 §4.1, §1.4. **Status:** open decision (contract §5).
Existing masters carry a q0.93 JPEG generation in their noise profile
and QA baselines were set against them. A native renderer emitting
clean frames drifts from every existing reference — or, if the JPEG
step is reproduced unthinkingly, bakes a browser artefact into the
native pipeline forever.
**Mitigation:** explicit open decision with both options and their
consequences recorded (contract §5); must be taken before M6 masters.
**Refresh 2026-11-14: CLOSED by decision.** Contract §5 is
RESOLVED 2026-11-14 (commit 801f85c): option (b) — native frames
stay clean; the JPEG generation is *not* reproduced. The
evidence brief (docs/ab/telemetry/jpeg-decision.md) measured one
q0.93 generation through the pipeline's own encoder at 42.31 dB
from clean, 94.5 % of pixels changed. Standing consequence:
A/B numbers against legacy references are reported, never gated
as equality, and native golden re-baselining is the accepted
cost of the decision.

## R7 — PCFSoft shadow sampling fingerprint · MEDIUM-HIGH

**Evidence:** a1 Risk #3. **Status:** deferred to M5 with chart tests.
Three's PCFSoft kernel has a specific softness pattern; `shadow.radius`
is set but ignored by PCFSoft in three — honouring it naively produces
*softer* shadows than production. Texel density varies by set (±3 m to
±16–18 m boxes at the same 2048²), and bias values were tuned against
WebGL depth distribution, not wgpu's [0,1] clip-z.
**Mitigation:** shadow charts per production box size; bias re-derived
in native depth units and the mapping recorded (ADR deferred table).
**Refresh 2026-11-14: MITIGATED at M5, with a measured residual.**
The composer gate PASSED with the PCF-soft key shadow map
transcribed (commit 79aff03; docs/ab/composer/README.md), and
production carries per-shot shadow configuration as data
(shadowBox / bias / normalBias, spec §14; census in
scenes/fixtures/FIXTURES.md, fx-lights). The standalone
per-box-size chart package named above was not produced as a
separate artefact; the evidence is the composer A/B plus the
whole-film A/B (final head mean 23.02 dB, integration record).
Residual, as measured: trunk sides run 6–9 % bright (region
ratio 1.06–1.09), consistent with the geometry-roughness term
and 12-segment cylinder faceting interacting with PCF-soft
edges — a ~1 dB-class term inside a 23.85 dB frame
(docs/ab/composer/gaps.md item 3). `shadow.radius` is not
honoured, matching the source.

## R8 — Composer chain as a system · MEDIUM

**Evidence:** a1 Risk #4 (§2.1–2.5). **Status:** architecture frozen.
Individually portable passes compose into a fingerprint: bloom
thresholds evaluated on linear HDR, tone mapping mid-chain, grading on
sRGB data, MSAA resolve on a HalfFloat target, shadow map re-rendered
every frame. Any "helpful" reordering or precision change shifts
highlights and edges film-wide.
**Mitigation:** pass order and formats frozen in ADR §5; stage-by-stage
diffing (not just final-frame diffing) during M5 bring-up.
**Refresh 2026-11-14: CLOSED.** M5 PASSED with exactly the
specified discipline (commit 79aff03; docs/ab/composer/README.md):
frozen order Render → Bokeh → Bloom → Output → GRADE over
HalfFloat MSAA-4×, with stage-by-stage ablation — lighting
21.36 dB → +bloom 21.42 → +GRADE 23.850 dB. Later chain changes
followed the same rule (loader fog inserted pre-bokeh as a
separately gated stage, commit 69952ab, fog chart ≤1 code at
every panel).

## R9 — Code-painted texture breadth (incl. text) · MEDIUM

**Evidence:** a1 Risk #5; a5 §1.3. **Status:** open; M2 decision point.
Every environment texture is a Chrome-canvas rasterisation — Skia AA,
gradient interpolation, font shaping and even canvas blur filters are
baked into pixels across every set. Per-texture differences are mostly
sub-pixel and acceptable; text-bearing textures (facades, signs) are
not in that class.
**Mitigation:** per-texture atlas diffs with per-texture thresholds,
strictest for text-bearing textures (a1 §3.6); texture-baking location
decided at M2 (ADR deferred table).
**Refresh 2026-11-14: MITIGATED — one named residual.** The M2
decision went to native painting: textures are rasterised by the
Rust painter, not baked by Chrome (commit 2a7ff89; M4 battery,
819ee16). In production all 13 texture generators named by the
data are armed in the loader, each with pinned painter tests
(commit c74cf0c; loader census, commit 743d248). The per-texture
threshold mechanism landed as those pinned tests rather than a
separate atlas-diff package. Residual in the strictest class:
the sign painter's border ink can sit up to the stroke
half-width (3 px) off the PIL asset's along the border
(painters/picture-sign.md, "Unreproducible / declared gaps"
item 2) — recorded, not silently absorbed.

## R10 — All verification so far is software-rendered · MEDIUM

**Evidence:** project README (honest rendering notes); a2 §1.5.
**Status:** standing rule in force.
CI renders on lavapipe; the user's PC (Vega 7 / RADV) is the only GPU
evidence source, and no native frame has been produced on it yet.
GPU-specific behaviour (sampler filtering at grazing angles, grain
hash precision, comparison-sampler borders) and all performance numbers
remain unverified until measured there.
**Mitigation:** contract §6.2 — perf/GPU claims admissible only from
the target PC; renderer-identity line printed on every run (§6.1).
**Refresh 2026-11-14: STILL OPEN — unchanged in substance.**
Every native number in the record is still CI/VM lavapipe
(FINAL-REPORT passim; the PERF protocol's tables in
docs/runbook/PERF.md are blank by design). No frame has been
produced on the user's Vega 7 / RADV yet, so GPU-specific
behaviour (grazing-angle filtering, grain hash precision, fp16
rounding — see the watchlist) and all performance numbers
remain unverified. The §6.1 half of the mitigation is in force:
the binary prints its renderer identity on every run (verified
against src/main.rs in docs/runbook/RUNBOOK-VERIFY-W95.md).
Discharge path is docs/runbook/USER-PC.md on the target PC;
nothing in this refresh relaxes §6.2.

## Watchlist

- **Points/stars:** wgpu has no point-size primitive; star fields need
  quad expansion (APPROXIMATE) — a1 §3.4. Decision at M2.
  **Refresh 2026-11-14: still open.** No points/star primitive
  was implemented and no production scene in the current corpus
  names one (the `points` arrays in the scene data are point
  *lights*, a different thing). The M2 decision point passed
  without a forcing case; the sky was later ported as a dome
  shader (commit 0f6c033) with no star fields. The item stays
  on the watchlist for the first night-sky scene that needs it.
- **Colour working-space semantics:** three r170 converts hex colours
  sRGB→linear on set; a naive port treating hex as raw floats shifts
  every derived shade — a1 §3.7. Deterministic once replicated; covered
  by the `shade()` truth-table test in a1 §3.7.
  **Refresh 2026-11-14: CLOSED — and the predicted failure
  actually occurred and was caught.** The native `shade_rgb` was
  an sRGB byte multiply, 25–40 LSB dark on derived shades —
  exactly this item's failure mode. Root fix (commit c00897d):
  the THREE.Color linear-space multiply at the single helper,
  with corpus pins reproducing exactly (#3A2E22 ×1.25 = #413427,
  #3A2A2A ×0.7 = #302222); the one test that had pinned the old
  divergence was re-pinned to the agreement (commit d8fbe83).
- **Grain pattern:** the GRADE grain hash's `sin()` precision is
  GPU-vendor-sensitive; grain can only be compared statistically, never
  per-pixel — a1 §4.1.
  **Refresh 2026-11-14: still open, bounded.** Confirmed and
  bounded at M5 (docs/ab/composer/gaps.md item 2): the term is
  ±0.006 of full scale, a ≈49 dB PSNR ceiling if it decorrelated
  entirely; it is not the limiting term in any measured frame.
  For the film, grade `time` is pinned to 0 by integration
  ruling (commit 5e5b0a0), so the shipped film carries no
  animated grain divergence; the statistical-only comparison
  rule stands for any grain-bearing render, and the RADV leg
  discharges only under R10.
- **Held-prop closures:** props are function values in state and cannot
  cross a data-only scene boundary; resolved by design (named prop IDs)
  in ADR §4 — flagged here so scene-format work does not rediscover it
  (a3 §6).
  **Refresh 2026-11-14: CLOSED, as designed.** Attachments are
  named kinds carried on the character node (integration ruling
  R2) and the loader consumes them — the rifle is anchored at
  the painted hand (commit 4c06a1e; census in
  scenes/fixtures/FIXTURES.md, fx-attachment). One fidelity note
  survives in a different form: held props ignore their staging
  angle argument, verified in the source (docs/integration/
  README.md, "Honest remaining limits").
