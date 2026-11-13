# Stroke battery A/B — the Milestone 4 painter gate (compatibility contract §2)

**Question.** Tall Tale characters are stroked paths — legs 15, arms
14, body/head outlines 9–10, brows 12 units, all round caps/joins
(a3 §5.1, risk R2). Can TaleRender's CPU painter reproduce Skia's
stroke rasterisation — the perceived line weight and corner softness —
at production resolution, or does every character silently change
drawing style?

**Verdict: PASS — 46/46 classes within tolerance.** The §28
painter (point-sampled, binary coverage) failed 14 of 46 classes;
the painter that ships with this package — signed-distance
coverage, footprint-area supersampling in the AA band, and
Skia-flattened round caps — passes all of them, most by an order
of magnitude. The full character card scores **47.9 dB** alpha
PSNR against the real `drawToon` (band RMSE 0.058 vs the 0.12
tolerance). Two production quirks the battery exposed are
reproduced faithfully, not "fixed": the laugh mouth's missing
outline (see Results) and the cap crown's stroked chord.
Regression: the §28 A/B *improved* to **20.218 dB** (baseline
20.152); the M3 dof gate and both M2 scene-format proofs
reproduce exactly. Details below; per-class numbers in
`metrics.json`.

## The pair

| file | what it is | renderer |
|---|---|---|
| `original.png` | the battery's prims card drawn by the **real Canvas2D painter** in Chrome: raw ctx calls in drawToon's own forms for the primitives, and the template's actual `drawToon` for the character/face cards (`project/battery.ts`) | Chrome 157 headless on this VM, Canvas2D/Skia (UA + WebGL probe string in `provenance.json` in this package; the cards are 2D canvas output, extracted losslessly via `toBlob`) |
| `native.png` | `talerender --scene stroke-battery` — the same spec through the CPU painter in `src/scene.rs` | TaleRender CPU painter — no GPU involved; the cards are raw painter output |

Both sides consume **one shared spec** (`spec.json` in this package):
the Rust binary embeds it at compile time (`include_str!`), the
Chrome page fetches the identical bytes. Geometry, widths and colours
cannot drift between sides.

**Resolution.** The production toon canvas is 1280×1408 logical at
`res = 2` → **2560×2816 device px** (a3 §2; the painter's `res`
default). The battery runs at exactly that size: it is the size the
painter ships, and stroke AA lives at device-pixel scale, so a
smaller card would measure a different rasterisation problem.

## The battery

- **Segments** — for each production width (15, 14, 12, 10, 9): a
  horizontal, a vertical, a ~28° diagonal straight segment, and a
  quadratic curve (the limb shape) at that width.
- **Arcs & ellipses** — the head outline as shipped (stroked ellipse
  138 × 0.97·138, width 10), a stroked circle (width 9), filled foot
  (20×10) and pupil (13×17) ellipses, the shock eye (white filled
  ellipse + width-6 ring), a filled disc, and the happy-eye arc
  stroke (r 20, width 10).
- **Joins & caps** — a width-15 zigzag (round joins), a width-14
  U-turn, two crossing width-12 strokes (separate paths: inter-stroke
  blending), a width-10 acute spike, and a width-15 cap-dominant
  short segment.
- **Face set** — full figures drawn by the real `drawToon` code path
  in five moods (neutral, determined, angry, laugh, shock): brows at
  tilts 0/16/22, pupil+lid eyes, arc eyes, shock eyes, line/quad/open
  mouths with clipped tongue and teeth. Measured on the head crop +
  a brow cross-profile per mood.
- **Full character** — the §28 Onoda look/state (cap, determined),
  drawn by `drawToon` on the reference side and `paint_toon` on the
  native side: global alpha metrics + an in-situ leg-width profile.
  This card doubles as the painter regression surface for §28.

## Tolerances (stated before measuring)

Recorded here and in `scripts/measure_battery.py` in the same commit
that introduced the battery, before the first measurement ran:

- **Width / edge position at half coverage:** |native − ref| ≤
  **0.5 device px** per class. (0.5 device px = 0.25 stage units; the
  character plane is minified ~2–4× on screen, so this is ≈0.15
  screen px — a systematic 1 device px error would already read as a
  different line weight.)
- **AA ramp width (10%→90% coverage):** |native − ref| ≤ **0.5
  device px**.
- **Coverage RMSE** across each profile's edge band: ≤ **0.10**.
- **Crop classes** (joins, crossings, faces, full character): RMSE
  within the edge band (either side's alpha fractional, dilated
  2 px) ≤ **0.12**, and ≤ **5%** of band pixels may disagree by
  > 0.25 coverage.

Measurement is on the **alpha channel** (coverage) throughout: it is
the signal both rasterisers produce at stroke edges, and it is immune
to the premultiplication rounding in the reference's PNG export.
Profiles are sampled bilinearly at 0.1 device px steps, averaged over
stations away from caps/joins unless the class is the cap/join.

## Results

### The fix loop (every round measured against the same tolerances)

| painter | PASS | what changed |
|---|---|---|
| §28 binary (point-sampled) | 32/46 | baseline: joins/crossings band RMSE ~0.16, ~17% band disagreement; faces ~0.12; laugh 0.284 |
| + signed-distance coverage | 41/46 | `cov = clamp(0.5 − sd)` everywhere; joins 0.052–0.060, faces ~0.047 |
| + 2×2 mean-of-coverage supersample | 20/46 | **falsified**: convolving the ramp with itself widens even straight edges (seg rows 1.41 px vs Skia 0.8) — reverted |
| + 4×4 binary inside-test supersample | 45/46 | the footprint integral estimated directly; straight edges stay exact, curved boundaries get their true area ramp |
| + flattened-cap model (CapFan) | **46/46** | round caps as Skia's chord-flattened arcs (below) |

Two measurement-surface corrections were made mid-loop, both
documented in `scripts/measure_battery.py`, neither touching the
tolerances: (1) interior face features are invisible in alpha
(ink on an opaque head), so face crops and brow profiles are
measured on **ink darkness** (composited luminance); (2) the
in-situ head outline is judged as an **edge** — the head is
filled, so alpha carries only the outline's silhouette edge and
a two-crossing stroke reading is degenerate there by
construction (the isolated prims class covers the full band);
the radial profile also uses only cap-unobstructed rays, since
the upward rays measure the cap's silhouette in both renderers.
One honest caveat from the baseline: axis-aligned integer-placed
edges coincide with Skia trivially (both sides measure the
0.8 px interpolation floor of a perfect step) — the diagonals,
curves, joins, caps and in-situ classes are the discriminating
evidence, and they are the ones that failed at baseline.

### Final per-class results (tolerances: Δwidth/edge ≤ 0.5 px, Δramp ≤ 0.5 px, profile RMSE ≤ 0.10; crops: band RMSE ≤ 0.12, disagreement ≤ 5%)

| class group | worst Δwidth/edge | worst Δramp | worst RMSE / band |
|---|---|---|---|
| segments + quads, w15 | 0.035 | 0.068 | 0.0046 |
| segments + quads, w14 | 0.008 | 0.050 | 0.0029 |
| segments + quads, w12 | 0.005 | 0.022 | 0.0025 |
| segments + quads, w10 | 0.072 | 0.081 | 0.0096 |
| segments + quads, w9 | 0.046 | 0.073 | 0.0072 |
| head outline (isolated) | 0.022 | 0.055 | 0.0032 |
| circle stroke / happy-eye arc | 0.048 | 0.007 | 0.0044 |
| fills (foot, pupil, bun, eye ring) | 0.000 | 0.000 | 0.0000 |
| **cap end** | **0.000** | **0.000** | **0.0000** |
| joins (zigzag, U-turn, spike) | — | — | band 0.054–0.067, disagree ≤ 2.0% |
| crossing strokes | — | — | band 0.057, disagree 1.2% |
| faces ×5 (head crop) | — | — | band 0.047–0.081, disagree ≤ 1.0% |
| brows ×5 | ≤ 0.494 | ≤ 0.120 | ≤ 0.0347 |
| full character (toon card) | — | — | alpha PSNR 47.876 dB; band 0.058; grey band 0.044 |
| head outline in situ (edge) | 0.037 | 0.220 | 0.0095 |
| leg in situ | 0.000 | 0.000 | 0.0000 |

Closest call: **brow-determined at Δwidth 0.494** of the 0.5 px
tolerance — the tilted, curved brow's vertical-profile width is
the one number that sits at the bar rather than under it; its
ramp (0.120) and RMSE (0.0347) pass with margin.

### Three findings the battery produced

1. **Skia's round cap is not a semicircle.** The reference cap's
   axial profile (edge 14.33 px at R 15, ramp 1.47 px) is
   impossible for a true disc (edge 14.5, one-pixel step): the
   boundary tracks ~0.3 px inside the circle around the tip.
   Skia flattens the cap arc into equal chords within its
   0.25-device-px flattening tolerance — at R 15, 9 chords, tip
   chord at R·cos(10°). The painter now models caps the same way
   (`CapFan` in `src/scene.rs`, chord count derived from the
   tolerance rule, not fitted); the class then matches **exactly**
   (Δ 0.000 / 0.000 / RMSE 0.0000).
2. **The production laugh mouth has no outline — by accident.**
   In `drawFace`'s laugh branch, the final `g.stroke()` runs
   after the tongue's `beginPath()`, so Chrome strokes the
   *tongue ellipse* (lineWidth 8) and the mouth outline is never
   drawn; the tongue carries an ink ring instead. The battery
   caught the port stroking the mouth path (band RMSE 0.284,
   the worst class). The port now reproduces the quirk verbatim;
   face-laugh band RMSE 0.081. (The skill's own templates render
   the same quirk — this is fidelity to production, flagged here
   for the skill owners.)
3. **The cap crown's chord is stroked; the brim's is not.**
   `drawHat`'s crown path is `closePath()`'d before fill+stroke,
   the brim path is not. The port's crown stroke was open; the
   toon card's grey error was concentrated in the cap zone
   (2.0% of pixels > 0.25 off, vs ≤ 0.03% elsewhere). Closed now;
   toon-card grey band RMSE 0.139 → 0.044.

## Regression

All run with the final binary (commit `f501d46`, CI run
43495977996, **green**: build + tests + M1 render):

| proof | before | with the M4 painter | verdict |
|---|---|---|---|
| §28 ab1 A/B vs committed original (`docs/ab/onoda-s01-t150/scripts/ab_metrics.py`) | 20.152 dB, mean \|Δ\| 17.273 | **20.218 dB**, mean \|Δ\| 17.177 | improved (+0.066 dB) |
| M3 dof-chart gate (`measure_dof.py`) | PSNR 47.376 dB, blur within 0.05 px | PSNR 47.376 dB — render **pixel-identical** (regenerated package files byte-identical) | PASS |
| M2 scene-format ab1-as-data | 95.170 dB, max Δ 1 | 95.170 dB, max Δ 1 | PASS, exact |
| M2 scene-format dof-as-data | 84.486 dB, max Δ 1 | 84.486 dB, max Δ 1 | PASS, exact |

The remaining §28 gap is unchanged in kind (composer/lighting,
M5 territory): the painter was gap #3 of 3 there, and this
package closes it. Numbers also in `metrics.json` under
`regression`.

## Reproduce

```bash
# reference (Chrome/Skia):
bash docs/ab/stroke-battery/project/run-reference.sh /tmp/stroke-ref
# native:
talerender --scene stroke-battery --out /tmp/stroke-native   # (CI binary)
# measurement:
python3 docs/ab/stroke-battery/scripts/measure_battery.py \
  --ref /tmp/stroke-ref --native /tmp/stroke-native \
  --spec docs/ab/stroke-battery/spec.json --out metrics.json
```
