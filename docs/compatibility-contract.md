# Compatibility Contract

The testable terms under which TaleRender output may be called compatible
with the Tall Tale Shorts pipeline it replaces. Every clause cites the
audit evidence it rests on ([docs/audit/](audit/)). A clause is satisfied
by its stated test, not by inspection or intention. Where the evidence
supports a question but not an answer, the contract says so and leaves
the decision open.

## 1. Depth of field — transcription before scenes (a1)

1.1. The r170 **BokehShader arithmetic is transcribed into WGSL** — same
circle-of-confusion formula (`factor = depth − focus`), same
`aperture × −focus` scaling, same fixed tap pattern — over a native
depth prepass. A physically better DoF is a different look and **fails**
this contract (a1 §2.3, Risk #1). The house look has been art-directed
around this shader's specific incorrectness, including the fgLeaves
foreground device that exists to be blurred by it (a1 Risk #1).

1.2. **The DoF chart test runs before any real scene is ported (M3
gate).** Chart: a sharp subject at `focus`, textured planes at known
distances in front of and behind the focal plane, high-contrast edges
crossing it. Pass condition: blur radius per distance and edge behaviour
match the Chrome/r170 reference within the tolerance recorded with the
chart; foreground-bleed behaviour is compared explicitly, since that is
where the shader is wrong in a load-bearing way.

1.3. Production defaults are the **effective** ones, not the constructor
values: aperture 0.0015, maxblur 0.006, focus from the shot's look
(a1 §2.3). DoF is enabled by default on essentially every frame.

## 2. Character identity — stroke rasterisation (a3)

2.1. Characters are stroked paths: limbs are single quadratic strokes
(legs 15, arms 14 units), body/head outlines 9–10, brows 12, all with
`lineCap`/`lineJoin = 'round'` (a3 §5.1). The native rasterizer must
reproduce the **perceived line weight and corner softness** of the
Chrome/Skia reference at production resolution. Flat interior fills are
expected to match exactly; the contract binds at stroke edges, curve
flattening and joins, where rasterizers differ.

2.2. **Stroke test (gates M4):** a fixed battery of production poses and
moods rendered by the reference painter and the native painter at res 2;
edge-profile comparison on limbs, head outline and brows. A mismatch
that "reads as a different drawing style" is a failure regardless of
aggregate pixel scores — this clause is judged on the flagged features,
not only on global metrics.

2.3. Supporting identity clauses, same battery: ellipse tessellation of
the head (radii r × 0.97r) and pupils (a3 §5.3); the clip + even-odd
head-shade crescent and body-pattern clip (a3 §5.4); premultiplied-alpha
canvas output with the dual silhouette thresholds (alphaTest 0.04
colour / 0.5 shadow) (a3 §5.5).

2.4. **Text metrics are part of identity.** Caption fit is
`min(1, 960 / measured width)` under Inter 900 (a3 §3.2), and the same
measurer serves renderer and audit (a3 §3.1). The native font stack must
produce the same fit values for the production caption corpus; the
capture audit must never re-measure with a second engine (a2 §3.3).

## 3. Semantic QA telemetry — the gate is not pixels (a2)

3.1. The native runtime reimplements the `cutouts()` telemetry channel
(a2 §3.2): per character, per sample — border-strip **overflow pixel
counts** per side, computed inside the painter; the toon plane's
screen-space **box** projected through the last-rendered camera
(unclamped, with behind-camera semantics); and the **look signature**
string.

3.2. The `check` verdict is reproduced exactly (a2 §3.1): sampled at
0.25 s with draw + telemetry atomic per sample; the verdict fails on
**any** of — bad samples, page-equivalent errors, canvas overflow
entries (deduped by id + pose + sides), continuity violations
(signature diff with named changed fields), strict-caption failures.
Output line formats are preserved; downstream QA reads them.

3.3. Advisory thresholds are reproduced with their constants (a2 §3.2):
figure/face boxes derived by the fixed stage fractions (figure: x
384/1280–896/1280, y 256/1408–1280/1408 of the plane box; face: x
118/512–394/512, y 40/1024–316/1024); caption zone y 1340–1550 at > 5%
face overlap; right rail x > 920; bottom band y > 1620; small figure
< 288 px; the dead-centre aggregate (≥ 4 focal samples, ≥ 80% presence,
≥ 90% centred).

3.4. Caption audit (a2 §3.3): rendered width ≤ 850 px and fit ≥ 0.68,
WARN by default, FAIL under the strict flag; auto-chunked lines reported
as estimate-grade; the ≥ 0.33 s/word time floor is **not** enforced
(ruling R5 — the width floor wins).

3.5. **Incomplete-audit honesty:** lost samples are reported as
`unknown (audit incomplete)`, never as `none`, and fail sheet mode
(a2 §3.4). An audit that lost samples must never certify a pass.

## 4. Determinism law (a3, a4)

4.1. `frame(t)` is a pure function of (t, project data, seeds). The
sources contain no `Math.random`, no wall-clock reads, no integrators
(a4 §3; a3 §4.3); the only pseudo-random source is the explicit sin-hash
`rnd(i)`, ported in f64. State-key quantisation in the painter rounds to
2 decimals (a3 §1.0) and is preserved, so repaint behaviour matches.

4.2. Pass condition: repeated and out-of-order evaluation of any
timestamp yields pixel-identical frames within one implementation.
Cross-implementation (Chrome vs native) frame equality is judged by the
parity tests of §1–§2, **not** by byte equality — last-ulp transcendental
differences are documented as acceptable (a4 §3).

## 5. OPEN DECISION — the q0.93 JPEG master generation (a2)

**Status: open. This contract does not decide it.**

Evidence (a2 §4.1): on the default path, every master frame reaches
libx264 only after a canvas JPEG encode at quality 0.93 — a real,
lossy generation baked into every existing master's noise profile and
into the QA baselines those masters were checked against. A native
renderer naturally produces clean frames; doing so silently would drift
output away from all existing references. The lossless raw path exists
in the current pipeline but is explicitly experimental and unvalidated
(a2 §1.4).

The decision, required **before M6 masters are produced**, is one of:

- (a) reproduce the q0.93 JPEG generation in the native path, preserving
  continuity with existing masters and baselines; or
- (b) adopt clean frames as a deliberate, documented quality change,
  re-baselining QA references, with the change disclosed wherever
  masters are compared.

Either is defensible; choosing by accident is not. Draft (0.8), frames
(0.92) and sheet (0.8) qualities are unaffected — they are QA surfaces,
not masters (a2 §4.1).

## 6. Evidence rules — where claims may come from

6.1. **Renderer identity is always printed.** Every mode that renders
prints a `renderer: <identity>` line (adapter name, backend, device
type) with a loud warning when a software adapter is in use — the
native equivalent of the current probe (a2 §1.5). The warning is
advisory; it never silently passes and never fails a run by itself.

6.2. **lavapipe is correctness evidence only.** CI runs on lavapipe
(software Vulkan). Frames from CI prove the pipeline works; they prove
nothing about GPU behaviour or speed. **Performance, GPU-feature and
timing claims are admissible only when measured on the target PC**
(Ryzen 5 PRO 5650G, Radeon Vega 7 iGPU, RADV) — the same rule the
project README states.

6.3. **Failure honesty is contractual** (a2 §4.4–4.5): outputs are
written to a `.part` sibling and renamed only after the encoder exits 0;
any failure deletes the partial file; "encoder failed to start" is
distinguished from "encoder exited non-zero"; invalid configuration
(e.g. FPS ≤ 0) fails before any work begins.

6.4. No quality reduction is ever applied silently. Any configuration
that lowers output quality relative to this contract is an explicit,
reported choice (project README rule; a2 §5 checklist).
