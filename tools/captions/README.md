# Caption strips — Onoda film (W02)

`gen_caption_strips.py` builds every caption chunk for
"THE SOLDIER WHO DIDN'T KNOW THE WAR WAS OVER" and renders each
chunk's strip PNG natively with PIL. Output lives in
`assets/captions/`: `manifest.json` + 66 strips (`cap-000.png` …
`cap-065.png`), each 1080×300 RGBA, text centred so the strip's
middle lands on frame y = 1340 (CAP.y) when composited post-grade,
exactly as the §28/M5 ab1 caption was composited. Re-running the
generator reproduces all 67 files **byte-identically** (verified).

## Method

- **Chunks — a port, not a reinterpretation.** Chunk text and
  timing are computed by a line-for-line port of the project's own
  `buildCaptions()` (`src/core/captions.ts` in
  `~/faceless-shorts/the-soldier-who-didnt-know`), fed by
  `script.json`'s authored `caps` fields, `data/words.json`
  (word timings) and `data/lines.json` (line ends, film duration
  60.357 s). Chunk start = first-word start − 0.03 s; chunk end =
  next chunk's start, or line end + 0.25 s across a > 0.6 s line
  gap; the last chunk ends at the film duration. All 12 lines
  carry authored `caps`, so the auto-chunk fallback (also ported)
  is unused. Cross-check: text, line and s/e of all 66 chunks are
  identical to the M6 first-party chunk list
  (`docs/ab/telemetry/ab1-captions.json`).
- **Style — the house caption.** CAP metrics from `captions.ts`:
  size 88, maxW 960, y 1340, font `900 Inter` — the project's own
  `fonts/Inter-Black.otf`. Each strip reproduces `captionText()`'s
  three passes (the proven §28 recipe,
  `docs/ab/onoda-s01-t150/scripts/make_caption.py`): shadow
  silhouette rgba(0,0,0,0.45) at (+3, +9); stroke lineWidth 15 in
  #0B0B10; white fill — all at the chunk's settled scale (`fit`);
  the 0.1 s pop-in is a draw-time scale in the source and is not
  baked into the strip. Consistency proof: the generated
  `cap-003.png` ("fighting") is **pixel-identical** to the §28
  strip that the M5 composer A/B was measured with.
- **Fit — the audit's own arithmetic** (skill template
  `tools/capture.ts`, caption audit): `rawW` = text width at
  88 px, `fit = min(1, 960 / rawW)`, `renderedW = rawW × fit`;
  WIDTH violation iff renderedW > 850, FIT violation iff
  fit < 0.68. Per-chunk `rawW`, `renderedW` and `fit` are in the
  manifest.

## Fit table — headline

Audit-format headline for this film's captions:

```
CAPTION AUDIT: 66 chunks, 12/12 lines authored, min fit 0.92, max rendered width 960 px
```

- **Chunks:** 66 (per line: 13, 5, 3, 5, 6, 5, 5, 6, 4, 5, 4, 5).
- **Min fit:** 0.9227 — "Planes dropped leaflets" (line 5,
  rawW 1040.44 px, rendered at the 960 px cap).
- **Max renderedW:** 960 px (the maxW cap; four chunks sit on it).
- **Over the 850 px rendered-width floor: 7 chunks** — the same
  seven the Chrome audit flags (WARN in the default gate;
  they are authored chunks, reported here, not re-split —
  splitting is an authoring decision, not a renderer's):
  "was a Japanese soldier" (960 px, t=6.69), "Planes dropped
  leaflets" (960 px, t=21.61), "surviving on coconuts"
  (960 px, t=31.55), "named Norio Suzuki" (884 px, t=37.13),
  "commander ordered it." (960 px, t=44.01), "his old
  commander," (867 px, t=46.39), "and Onoda surrendered"
  (960 px, t=52.07).
- **Under the 0.68 fit floor: none** (min fit 0.9227).

## Declared limit — PIL measures, Skia typesets

Widths in the manifest are measured with **PIL/FreeType**
(`getlength` at 88 px) against the project's Inter-Black.otf.
The production audit measures with **Skia's `measureText`**
inside Chrome against the same font file. PIL's metrics are not
Skia's, so `rawW` — and, for over-wide chunks, `fit` — can differ
by a small factor between this manifest and the Chrome audit.
Chunk **text and timing are exact** (pure data, ported); only
the width measurement carries this limit (the M6 "measurement
enters as data" limit, carried per the integration inventory).

Measured on this run against the M6 first-party metrics
(`docs/ab/telemetry/ab1-captions.json`, all 66 chunks):

| Quantity | PIL vs Skia |
|---|---|
| rawW absolute difference | max 0.15 px, mean 0.054 px |
| rawW relative difference | max 0.039% |
| fit difference | max 0.0005 |
| Width-floor verdicts (renderedW > 850) | identical — same 7 chunks |
| Fit-floor verdicts (fit < 0.68) | identical — none on either side |

So for this film the declared limit is real but immaterial: no
audit verdict flips between the two measurers. On another film a
chunk sitting within ~0.2 px of the 850 floor could classify
differently under PIL than under Skia; the manifest's numbers
should be read with that tolerance.
