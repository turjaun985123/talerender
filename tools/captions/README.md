# Caption strips — Onoda film (W02)

`gen_caption_strips.py` builds every caption chunk for
"THE SOLDIER WHO DIDN'T KNOW THE WAR WAS OVER" and renders each
chunk's strip PNG natively with PIL. Output lives in
`assets/captions/`: `manifest.json` + 73 strips (`cap-000.png` …
`cap-072.png`), each 1080×300 RGBA, text centred so the strip's
middle lands on frame y = 1340 (CAP.y) when composited post-grade,
exactly as the §28/M5 ab1 caption was composited. Re-running the
generator reproduces all 74 files **byte-identically** (verified;
re-verified at the re-chunk, W01).

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
  is unused. Cross-check: text, line and s/e of all chunks are
  identical to the M6 first-party chunk list
  (`docs/ab/telemetry/ab1-captions.json`) — with one dated
  exception: the **re-chunk (W01, 2026-11-14)** split the 7
  over-floor chunks at word boundaries in the project's `caps`
  (66 → 73 chunks), superseding the M6 list at those 7 chunks
  only; every other chunk still matches it exactly. Record:
  `docs/ab/film/RECHUNK-W01.md`.
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
CAPTION AUDIT: 73 chunks, 12/12 lines authored, min fit 1.00, max rendered width 835 px
```

- **Chunks:** 73 (per line: 13, 6, 3, 5, 7, 5, 6, 7, 5, 6, 5, 5).
- **Min fit:** 1.00 — no chunk is fit-scaled any more (the
  widest rawW, 834.80 px, is under the 960 px maxW cap).
- **Max renderedW:** 835 px — "Onoda still refused" (line 8,
  rawW 834.80 px, fit 1.0).
- **Over the 850 px rendered-width floor: none.** The 7 chunks
  that used to exceed it ("was a Japanese soldier",
  "Planes dropped leaflets", "surviving on coconuts",
  "named Norio Suzuki", "commander ordered it.", "his old
  commander,", "and Onoda surrendered" — the same seven the
  Chrome audit flagged as WARN) were split at word boundaries
  by the re-chunk (W01, 2026-11-14; the user's Wave-2 order
  superseded the earlier waiver ruling — see
  `docs/ab/film/RECHUNK-W01.md`). Widest split piece:
  "was a Japanese" (rawW 699.34 px).
- **Under the 0.68 fit floor: none** (min fit 1.00).

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

Measured against the M6 first-party metrics
(`docs/ab/telemetry/ab1-captions.json`, all 66 chunks of the
pre-re-chunk chunking; the comparison still holds per-chunk for
the 59 unsplit chunks, and the re-chunk's split points were
themselves computed with PIL against this same font — W92):

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
