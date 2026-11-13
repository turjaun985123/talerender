# M6 — native QA telemetry: the cutouts() equivalent, proven against Chrome

**Milestone 6 (compatibility contract §3).** The Chrome capture
gate does not judge pixels; it judges semantic telemetry —
`OP.cutouts()` (per-toon canvas-overflow counts, screen-space
boxes, look signatures, pose summaries) fed through capture.ts's
audit: CANVAS OVERFLOW, EDGE TOUCH, CONTINUITY, composition
advisories, the caption audit, and one verdict expression. This
package implements the same telemetry natively
(`src/telemetry.rs`, CLI `--telemetry ab1|ab1-file|ab1-variant`)
and proves it field-by-field against the **real** `OP.cutouts()`
running in Chrome on the same staging.

## Verdict — AGREEMENT IS EXACT on every field, both stagings

Renderers, labelled: Chrome side = headless Chrome 157,
**ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero)),
SwiftShader driver)** for both Chrome runs (the real project page
and the harness page; `provenance.json`, `chrome-meta.json`).
Native side = pure CPU arithmetic (the painter canvas + a
three.js-faithful f64 projection); no GPU is involved in the
telemetry itself, so no lavapipe figure appears anywhere in this
package and no performance claim is made.

### Base staging — S01 @ t = 1.50 (the §28 frame)

Four columns: the real project page's `OP.cutouts()` after
`OP.draw(1.5)` (`project/drive.mjs` → `chrome-cutouts.json`); the
harness page staging the identical numbers through the project's
own modules (`project/run-harness.sh` → `base-cutouts.json`); the
native hand-authored telemetry (`native-ab1.json`); the native
scene-file telemetry (`native-ab1-file.json`). The staged onoda
entry, field by field (`project/compare.py` reproduces this
table from the committed payloads):

| field | value (identical in all four columns) |
|---|---|
| id | `onoda` |
| desc | `hat=cap body=#5C6644 pattern=plain` |
| pose | `mood=determined armL=[10,6] armR=[16,-6] legs=stand head=0/-0.024999999999999967` |
| overflow | `{top: 0, right: 0, bottom: 0, left: 0}` |
| box | `{x0: 50, y0: 528, x1: 1030, y1: 1452}` |
| sig | `{"body":"#5C6644","pattern":null,"patCol":null,"hat":"cap","hatCol":"#4C5638","band":null,"neck":null,"neckCol":null,"head":null,"kid":false,"extra":[]}` |

Audit outcome for the sample: native prints `CANVAS OVERFLOW:
none`, `CONTINUITY: clean`, `COMPOSITION: 0 findings`, `CHECK
OK` (exit 0). Every gate term derivable from the Chrome payload
agrees: its overflow map is all-zero, and the composition
advisories computed from Chrome's own box by the transcribed
rules are also zero (figure 392×672 px inside the frame, face
centre (540, 813) — no rail, no caption-zone overlap, no edge
touch). The pose string deserves a note: S01's poseTrack sweep
at t = 1.50 is the f64 `-0.024999999999999967`, not the rounded
`-0.025` earlier extractions printed; the native staging carries
the exact value, which is why the strings match byte-for-byte.

Caption audit (both sides judge the same 66 chunks,
`ab1-captions.json` — metrics measured in the harness with the
project's own caption-engine measurer; see gaps.md §1): native
headline `CAPTION AUDIT: 66 chunks, 12/12 lines authored, min
fit 0.92, max rendered width 960 px`, plus 7 width WARNs for
chunks rendering over the 850 px threshold (lines 2, 5, 7, 8, 9,
10, 11). WARNs do not fail the gate under the default policy;
nothing in the sample fails.

Continuity scope differs by construction and is recorded in
gaps.md §3: the page tracks the film's whole cast (5 distinct
signature ids: onoda, compA, compB, suzuki, taniguchi); the
native single-scene sample tracks its one staged character. The
onoda signature **value** — the field the gate compares — is
identical.

### The deliberately-overflowing variant — both gates fail it

Same S01 camera and look; the toon is re-staged at group
(1.9, −0.55, 0.6) with lean 42° so the painted head crosses the
canvas's right strip and the figure breaches the right rail and
the frame's bottom edge. Staged identically on both sides from
shared constants (native `ab1-variant`; harness §2 with the
project's real `Toon`/`set()`/`face()` and `OP.cutouts` copied
verbatim from the project's `main.ts`).

| field | Chrome (harness) | native |
|---|---|---|
| overflow | `{top: 0, right: 1028, bottom: 0, left: 0}` | `{top: 0, right: 1028, bottom: 0, left: 0}` |
| box | `{x0: 662, y0: 867, x1: 1983, y1: 2032}` | `{x0: 662, y0: 867, x1: 1983, y1: 2032}` |
| pose | `… head=0/-0.025` (the literal its set() passes) | identical |
| desc / sig | — | identical to base |

Native audit of the variant (`native-ab1-variant.txt`):
`CANVAS OVERFLOW: onoda (…) right=1028 — pose …`, EDGE TOUCH
right + bottom, one SAFE AREA rail finding, `CHECK FAILED`
(exit 1). The failing term is the same one the v5 verdict
expression fails on — a non-zero canvas-overflow side — and the
advisories derived from Chrome's box by the same rules produce
the same edge-touch and rail findings. The overflow **count**
agreeing to the pixel (1028 = 1028) is stronger than the
contract requires; counts are rasteriser-dependent in general
(gaps.md §4) and are reported here as measured, not as a
guarantee.

## What was implemented

`src/telemetry.rs` transcribes, from the v5 template capture
(the contract's binding source): the cutouts item shape;
`scanCanvasOverflow` (alpha > 32 in res-scaled border strips,
corners double-counted, null when unpainted); the plane-box
projection (geometry bounding-box corners at z = bb.min.z
through matrixWorld, behind/non-finite handling, ±10000 clamp,
JS `Math.round`); the figure/face sub-boxes (capture.ts
fractions); EDGE TOUCH on the figure box; the composition
advisories (caption zone y 1340–1550 at >5 % of face area, rail
x > 920, bottom y > 1620, exceeds-frame, small < 288 px,
dead-centre aggregation); CANVAS OVERFLOW and CONTINUITY dedupe
keys; the caption audit (850 px / 0.68 fit, WARN default, FAIL
under strict); and the verdict expression itself,
`failed = bad>0 || logs>0 || overflow>0 || continuity>0 ||
strict-caption fails>0`. CLI:

```
talerender --telemetry ab1        [--captions ab1-captions.json] [--cutouts-out out.json]
talerender --telemetry ab1-file --scene-file scenes/ab1.scene.json [--captions …]
talerender --telemetry ab1-variant [--captions …]
```

Exit code mirrors the verdict (0 OK / 1 FAILED / 2 usage). The
hand-authored and scene-file forms produce byte-identical
cutout payloads — the telemetry, like the M2 format proof,
loses nothing through the loader.

## Files

- `chrome-cutouts.json`, `chrome-meta.json` — the real page's
  payloads (11 cutout items; the film's cast list; renderer
  string).
- `base-cutouts.json`, `base-debug.json` — harness base staging
  (payload + the mesh's matrixWorld/camera dump used to verify
  the projection against three.js's own arithmetic).
- `variant-cutouts.json` — harness variant payload.
- `ab1-captions.json` — all 66 chunks with measured metrics.
- `native-*.json` / `native-*.txt` — native payloads and full
  audit output for the three modes.
- `project/` — `drive.mjs` (real page), `harness.ts` /
  `harness.html` / `serve.mjs` / `run-harness.sh` (project-
  module harness), `compare.py` (the table above).
- `jpeg-decision.md` — the contract §5 brief (evidence +
  recommendation; the decision is the user's).
- `jpeg-metrics.json`, `jpeg-gen1.jpg` — the brief's direct
  measurement and its encoded artifact.
- `gaps.md` — what does not match exactly, and why.
