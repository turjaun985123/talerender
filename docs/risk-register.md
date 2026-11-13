# Risk Register

Ranked by expected damage to parity × likelihood, from the Milestone 0
audit evidence ([docs/audit/](audit/)). Each risk names its mitigation
and where it is enforced ([ADR-001](ADR-001-talerender-architecture.md),
[compatibility-contract.md](compatibility-contract.md)). Status is as of
the M0 gate (2026-11-13): no mitigations are complete yet; several tests
are scheduled as milestone gates.

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

## R6 — q0.93 JPEG master-generation drift · MEDIUM-HIGH

**Evidence:** a2 §4.1, §1.4. **Status:** open decision (contract §5).
Existing masters carry a q0.93 JPEG generation in their noise profile
and QA baselines were set against them. A native renderer emitting
clean frames drifts from every existing reference — or, if the JPEG
step is reproduced unthinkingly, bakes a browser artefact into the
native pipeline forever.
**Mitigation:** explicit open decision with both options and their
consequences recorded (contract §5); must be taken before M6 masters.

## R7 — PCFSoft shadow sampling fingerprint · MEDIUM-HIGH

**Evidence:** a1 Risk #3. **Status:** deferred to M5 with chart tests.
Three's PCFSoft kernel has a specific softness pattern; `shadow.radius`
is set but ignored by PCFSoft in three — honouring it naively produces
*softer* shadows than production. Texel density varies by set (±3 m to
±16–18 m boxes at the same 2048²), and bias values were tuned against
WebGL depth distribution, not wgpu's [0,1] clip-z.
**Mitigation:** shadow charts per production box size; bias re-derived
in native depth units and the mapping recorded (ADR deferred table).

## R8 — Composer chain as a system · MEDIUM

**Evidence:** a1 Risk #4 (§2.1–2.5). **Status:** architecture frozen.
Individually portable passes compose into a fingerprint: bloom
thresholds evaluated on linear HDR, tone mapping mid-chain, grading on
sRGB data, MSAA resolve on a HalfFloat target, shadow map re-rendered
every frame. Any "helpful" reordering or precision change shifts
highlights and edges film-wide.
**Mitigation:** pass order and formats frozen in ADR §5; stage-by-stage
diffing (not just final-frame diffing) during M5 bring-up.

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

## Watchlist

- **Points/stars:** wgpu has no point-size primitive; star fields need
  quad expansion (APPROXIMATE) — a1 §3.4. Decision at M2.
- **Colour working-space semantics:** three r170 converts hex colours
  sRGB→linear on set; a naive port treating hex as raw floats shifts
  every derived shade — a1 §3.7. Deterministic once replicated; covered
  by the `shade()` truth-table test in a1 §3.7.
- **Grain pattern:** the GRADE grain hash's `sin()` precision is
  GPU-vendor-sensitive; grain can only be compared statistically, never
  per-pixel — a1 §4.1.
- **Held-prop closures:** props are function values in state and cannot
  cross a data-only scene boundary; resolved by design (named prop IDs)
  in ADR §4 — flagged here so scene-format work does not rediscover it
  (a3 §6).
