# Stroke battery A/B — the Milestone 4 painter gate (compatibility contract §2)

**Question.** Tall Tale characters are stroked paths — legs 15, arms
14, body/head outlines 9–10, brows 12 units, all round caps/joins
(a3 §5.1, risk R2). Can TaleRender's CPU painter reproduce Skia's
stroke rasterisation — the perceived line weight and corner softness —
at production resolution, or does every character silently change
drawing style?

**Verdict: _(filled in after measurement)_**

## The pair

| file | what it is | renderer |
|---|---|---|
| `original.png` | the battery's prims card drawn by the **real Canvas2D painter** in Chrome: raw ctx calls in drawToon's own forms for the primitives, and the template's actual `drawToon` for the character/face cards (`project/battery.ts`) | Chrome 157 headless on this VM, Canvas2D/Skia (WebGL probe string + UA in `project/` run notes; the cards are 2D canvas output, extracted losslessly via `toBlob`) |
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

_(baseline + final tables filled in after measurement)_

## Regression

_(ab1 / dof-chart / scene-format outcomes filled in after the final
binary is measured)_

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
