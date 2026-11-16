# R1-10 — Caption-fit measurement report (Package 4, F4/F8)

Tool: `measure_caption_fit.py` (this directory). Data: `measurement.json`
(machine-readable rows). Font: `Inter-Black.otf` (Inter 900), md5
`c26b4e77dc103fd3a1e3b426549296ed` — byte-identical in the Onoda
project, the Lighthouse Cat project, and the skill template, so the
font measured here IS the production caption font file; only the
measurer differs from Chrome (PIL/FreeType `getlength()` here vs
Skia `measureText` in Chrome — quantified below, max Δ 0.1 px).

## Headline

**Lighthouse Cat (43 chunks, all authored `fit: 1.0` assumed):**
- **1 chunk's fit value changes** when measured: idx 3
  `"the lighthouse burning."` — PIL rawW **1039.7 px**, measured fit
  **0.9233** (not 1.0). Min fit is **0.9233**, not the 1.00 the data
  claims. No chunk crosses the 0.68 FIT floor either way.
- **0 chunks change pass/fail verdict.** 3 chunks are WIDTH fails
  under every reading (authored/native, measured/font,
  measured/native): idx 0 `"For thirty-one years,"` (rawW 917.9,
  ink 922), idx 3 (rawW 1039.7, ink 1049), idx 40
  `"Biscuit finally slept,"` (rawW 882.6, ink 887). The native
  audit's WIDTH test uses strip ink, which does not involve `fit`,
  so the assumed 1.0 never hid these — what it hid is in NOTE.md
  (vacuous FIT floor, wrong on-screen scale for idx 3, understated
  audit raw width). These 3 chunks need re-chunking (split), not a
  fit fix; that is an authoring action for the re-bridge (R2-03),
  surfaced here, not fixed here.

**Onoda (73 chunks, authored fits all 1.0 — see NOTE.md for why
that is not the same defect):**
- **0 chunks' fit differs from measurement; 0 fails under any
  reading.** Max PIL rawW 834.8 px, max strip ink 844 px — the
  current chunking is genuinely inside both floors.
- **Measurer validation: PASS, strongly.** 59 of the 73 chunks
  match the Chrome/Skia first-party metrics
  (`docs/ab/telemetry/ab1-captions.json`, matched by text + start
  time): max |PIL rawW − Skia rawW| = **0.1 px**, and the derived
  fit agrees on **59/59**. The other 14 chunks are re-chunked
  splits of ab1-era longer chunks (ab1 has 66 chunks, including 5
  with fit < 1 — e.g. `"was a Japanese soldier"`, Skia rawW 1023.9,
  fit 0.938 — now split as `"was a Japanese"` + `"soldier"`), so
  they have no ab1 counterpart. Independent corroboration:
  `tools/captions/gen_caption_strips.py`'s shipped manifest
  (PIL, same font) also reports 73 chunks, min fit 1.0.
- Conclusion: Onoda's authored fits match measurement because
  `film/gen_film_data.py` MEASURED them with this same PIL
  measurer when the data was generated. The bridge builder is the
  only producer that assumes. Where PIL and Skia could disagree
  (rawW near the 960 cap), they do not on any matched chunk —
  the declared "small factor" caveat in gen_caption_strips.py's
  header is bounded by evidence here at ≤0.1 px on 59 chunks,
  including chunks up to 834.7 px raw. No matched chunk sits in
  the 850–1040 px band where a measurer delta could flip a fit,
  so the residual risk is named, not quantified away: a future
  chunk with Skia rawW in (960, ~961] could round differently;
  nothing in either film is near that knife-edge (nearest:
  Lighthouse idx 3 at 1039.7, fit 0.9233 under both readings of
  the evidence — its ab1-era analogues at 989.9–1040.3 Skia rawW
  vs PIL values within 0.1 px on matched neighbours).

**Strip-ink vs font advance (the delta the task asked to surface):**
the binary audit's rendered width is the strip ink bbox, which runs
**+4.1 to +15.0 px** above PIL rawW on these 116 chunks (stroke
width 8 centred on the glyph edge + the (+3,+9) shadow silhouette,
per film/gen_assets.py). It is a consistent positive bias, not
noise: a chunk at rawW 845 would read ~855 ink and WIDTH-fail the
native audit while passing the font-semantics audit. Neither film
has a chunk in that 835–850 rawW band today (Onoda max 834.8 /
ink 844 — 6 px of headroom, the closest call in either film).

## Full per-chunk tables (tool output, verbatim)


## /home/hatch/workspace/talerender/film/onoda.json — 73 chunks
font /home/hatch/workspace/custom-skills/tall-tale-shorts/template/fonts/Inter-Black.otf (md5 c26b4e77dc103fd3a1e3b426549296ed); CAP 900 88px Inter, maxW 960; floors: rendered > 850 = WIDTH, fit < 0.68 = FIT
authored fit values in data: [1.0]; chunks whose measured fit differs: 0; min measured fit 1.0000; max PIL rawW 834.8 px; max strip ink 844 px
fails — authored/native (binary audit as data stands): 0; measured/font (Chrome semantics): 0; measured/native (binary audit, measured fit written): 0; chunks changing verdict vs authored/native: 0
Skia reference: 59 chunks matched by (text, s); max |PIL rawW − Skia rawW| = 0.1 px; fit agrees (≤0.0005) on 59/59

| idx | line | s (s) | text | authored fit | PIL rawW | measured fit | rendered (font) | strip ink | ink−raw | verdict authored/native | verdict measured/font | verdict measured/native |
|---:|---:|---:|---|---:|---:|---:|---:|---:|---:|---|---|---|
| 0 | 1 | 0.27 | This | 1.000 | 195.4 | 1.0000 | 195.4 | 208 | 12.6 | pass | pass | pass |
| 1 | 1 | 0.43 | soldier | 1.000 | 307.2 | 1.0000 | 307.2 | 320 | 12.8 | pass | pass | pass |
| 2 | 1 | 0.93 | kept | 1.000 | 196.8 | 1.0000 | 196.8 | 208 | 11.2 | pass | pass | pass |
| 3 | 1 | 1.35 | fighting | 1.000 | 354.0 | 1.0000 | 354.0 | 366 | 12.0 | pass | pass | pass |
| 4 | 1 | 1.83 | World | 1.000 | 268.3 | 1.0000 | 268.3 | 279 | 10.7 | pass | pass | pass |
| 5 | 1 | 2.15 | War | 1.000 | 180.3 | 1.0000 | 180.3 | 193 | 12.7 | pass | pass | pass |
| 6 | 1 | 2.77 | II | 1.000 | 51.2 | 1.0000 | 51.2 | 60 | 8.8 | pass | pass | pass |
| 7 | 1 | 3.13 | for | 1.000 | 129.0 | 1.0000 | 129.0 | 144 | 15.0 | pass | pass | pass |
| 8 | 1 | 3.65 | 29 | 1.000 | 116.3 | 1.0000 | 116.3 | 127 | 10.7 | pass | pass | pass |
| 9 | 1 | 4.17 | years | 1.000 | 251.1 | 1.0000 | 251.1 | 265 | 13.9 | pass | pass | pass |
| 10 | 1 | 4.67 | after | 1.000 | 211.6 | 1.0000 | 211.6 | 224 | 12.4 | pass | pass | pass |
| 11 | 1 | 5.13 | it | 1.000 | 61.2 | 1.0000 | 61.2 | 72 | 10.8 | pass | pass | pass |
| 12 | 1 | 5.29 | ended. | 1.000 | 310.4 | 1.0000 | 310.4 | 318 | 7.6 | pass | pass | pass |
| 13 | 2 | 5.94 | Hiroo Onoda | 1.000 | 544.7 | 1.0000 | 544.7 | 552 | 7.3 | pass | pass | pass |
| 14 | 2 | 6.69 | was a Japanese | 1.000 | 699.3 | 1.0000 | 699.3 | 713 | 13.7 | pass | pass | pass |
| 15 | 2 | 7.87 | soldier | 1.000 | 307.2 | 1.0000 | 307.2 | 320 | 12.8 | pass | pass | pass |
| 16 | 2 | 8.53 | sent to | 1.000 | 304.2 | 1.0000 | 304.2 | 316 | 11.8 | pass | pass | pass |
| 17 | 2 | 9.31 | a Philippine island | 1.000 | 801.1 | 1.0000 | 801.1 | 811 | 9.9 | pass | pass | pass |
| 18 | 2 | 10.39 | in 1944. | 1.000 | 353.4 | 1.0000 | 353.4 | 359 | 5.6 | pass | pass | pass |
| 19 | 3 | 13.15 | His orders | 1.000 | 451.8 | 1.0000 | 451.8 | 462 | 10.2 | pass | pass | pass |
| 20 | 3 | 13.61 | were simple: | 1.000 | 565.9 | 1.0000 | 565.9 | 575 | 9.1 | pass | pass | pass |
| 21 | 3 | 14.75 | never surrender. | 1.000 | 741.0 | 1.0000 | 741.0 | 746 | 5.0 | pass | pass | pass |
| 22 | 4 | 16.01 | In 1945, | 1.000 | 351.0 | 1.0000 | 351.0 | 355 | 4.0 | pass | pass | pass |
| 23 | 4 | 17.25 | the war ended. | 1.000 | 659.0 | 1.0000 | 659.0 | 668 | 9.0 | pass | pass | pass |
| 24 | 4 | 19.51 | But Onoda | 1.000 | 458.3 | 1.0000 | 458.3 | 466 | 7.7 | pass | pass | pass |
| 25 | 4 | 19.93 | never got | 1.000 | 419.4 | 1.0000 | 419.4 | 430 | 10.6 | pass | pass | pass |
| 26 | 4 | 20.47 | the message. | 1.000 | 599.2 | 1.0000 | 599.2 | 608 | 8.8 | pass | pass | pass |
| 27 | 5 | 21.61 | Planes dropped | 1.000 | 687.1 | 1.0000 | 687.1 | 695 | 7.9 | pass | pass | pass |
| 28 | 5 | 22.09 | leaflets | 1.000 | 335.8 | 1.0000 | 335.8 | 346 | 10.2 | pass | pass | pass |
| 29 | 5 | 22.61 | saying the war | 1.000 | 647.3 | 1.0000 | 647.3 | 660 | 12.7 | pass | pass | pass |
| 30 | 5 | 23.59 | was over. | 1.000 | 425.1 | 1.0000 | 425.1 | 434 | 8.9 | pass | pass | pass |
| 31 | 5 | 24.49 | He thought it | 1.000 | 569.8 | 1.0000 | 569.8 | 581 | 11.2 | pass | pass | pass |
| 32 | 5 | 24.97 | was an enemy | 1.000 | 626.9 | 1.0000 | 626.9 | 643 | 16.1 | pass | pass | pass |
| 33 | 5 | 25.57 | trick. | 1.000 | 238.6 | 1.0000 | 238.6 | 248 | 9.4 | pass | pass | pass |
| 34 | 6 | 26.19 | His friends | 1.000 | 481.7 | 1.0000 | 481.7 | 492 | 10.3 | pass | pass | pass |
| 35 | 6 | 26.67 | gave up | 1.000 | 346.8 | 1.0000 | 346.8 | 358 | 11.2 | pass | pass | pass |
| 36 | 6 | 27.19 | or died, | 1.000 | 336.6 | 1.0000 | 336.6 | 343 | 6.4 | pass | pass | pass |
| 37 | 6 | 27.79 | until he | 1.000 | 329.6 | 1.0000 | 329.6 | 339 | 9.4 | pass | pass | pass |
| 38 | 6 | 28.49 | was alone. | 1.000 | 475.5 | 1.0000 | 475.5 | 485 | 9.5 | pass | pass | pass |
| 39 | 7 | 29.35 | He lived | 1.000 | 353.3 | 1.0000 | 353.3 | 362 | 8.7 | pass | pass | pass |
| 40 | 7 | 29.71 | in the jungle | 1.000 | 541.2 | 1.0000 | 541.2 | 551 | 9.8 | pass | pass | pass |
| 41 | 7 | 30.29 | for decades, | 1.000 | 557.9 | 1.0000 | 557.9 | 566 | 8.1 | pass | pass | pass |
| 42 | 7 | 31.55 | surviving | 1.000 | 425.9 | 1.0000 | 425.9 | 436 | 10.1 | pass | pass | pass |
| 43 | 7 | 32.01 | on coconuts | 1.000 | 546.5 | 1.0000 | 546.5 | 559 | 12.5 | pass | pass | pass |
| 44 | 7 | 32.99 | and bananas. | 1.000 | 598.1 | 1.0000 | 598.1 | 605 | 6.9 | pass | pass | pass |
| 45 | 8 | 34.27 | In 1974, | 1.000 | 336.7 | 1.0000 | 336.7 | 341 | 4.3 | pass | pass | pass |
| 46 | 8 | 35.53 | a young traveler | 1.000 | 715.4 | 1.0000 | 715.4 | 728 | 12.6 | pass | pass | pass |
| 47 | 8 | 37.13 | named Norio | 1.000 | 562.3 | 1.0000 | 562.3 | 572 | 9.7 | pass | pass | pass |
| 48 | 8 | 38.07 | Suzuki | 1.000 | 304.2 | 1.0000 | 304.2 | 314 | 9.8 | pass | pass | pass |
| 49 | 8 | 38.45 | found him. | 1.000 | 478.5 | 1.0000 | 478.5 | 488 | 9.5 | pass | pass | pass |
| 50 | 8 | 39.95 | Onoda still refused | 1.000 | 834.8 | 1.0000 | 834.8 | 844 | 9.2 | pass | pass | pass |
| 51 | 8 | 40.83 | to leave. | 1.000 | 378.3 | 1.0000 | 378.3 | 387 | 8.7 | pass | pass | pass |
| 52 | 9 | 41.76 | He said | 1.000 | 324.0 | 1.0000 | 324.0 | 332 | 8.0 | pass | pass | pass |
| 53 | 9 | 42.17 | he would only stop | 1.000 | 827.8 | 1.0000 | 827.8 | 837 | 9.2 | pass | pass | pass |
| 54 | 9 | 43.13 | if his own | 1.000 | 421.6 | 1.0000 | 421.6 | 429 | 7.4 | pass | pass | pass |
| 55 | 9 | 44.01 | commander | 1.000 | 532.1 | 1.0000 | 532.1 | 545 | 12.9 | pass | pass | pass |
| 56 | 9 | 44.43 | ordered it. | 1.000 | 457.8 | 1.0000 | 457.8 | 465 | 7.2 | pass | pass | pass |
| 57 | 10 | 45.39 | So Japan found | 1.000 | 682.9 | 1.0000 | 682.9 | 693 | 10.1 | pass | pass | pass |
| 58 | 10 | 46.39 | his old | 1.000 | 290.3 | 1.0000 | 290.3 | 299 | 8.7 | pass | pass | pass |
| 59 | 10 | 46.93 | commander, | 1.000 | 559.4 | 1.0000 | 559.4 | 566 | 6.6 | pass | pass | pass |
| 60 | 10 | 47.77 | now a bookseller, | 1.000 | 772.3 | 1.0000 | 772.3 | 776 | 3.7 | pass | pass | pass |
| 61 | 10 | 48.93 | and flew him | 1.000 | 560.6 | 1.0000 | 560.6 | 571 | 10.4 | pass | pass | pass |
| 62 | 10 | 49.63 | to the island. | 1.000 | 573.6 | 1.0000 | 573.6 | 583 | 9.4 | pass | pass | pass |
| 63 | 11 | 50.69 | He finally | 1.000 | 420.5 | 1.0000 | 420.5 | 433 | 12.5 | pass | pass | pass |
| 64 | 11 | 51.19 | gave the order, | 1.000 | 662.6 | 1.0000 | 662.6 | 669 | 6.4 | pass | pass | pass |
| 65 | 11 | 52.07 | and Onoda | 1.000 | 473.7 | 1.0000 | 473.7 | 483 | 9.3 | pass | pass | pass |
| 66 | 11 | 52.99 | surrendered | 1.000 | 548.8 | 1.0000 | 548.8 | 559 | 10.2 | pass | pass | pass |
| 67 | 11 | 53.57 | after 29 years. | 1.000 | 646.9 | 1.0000 | 646.9 | 654 | 7.1 | pass | pass | pass |
| 68 | 12 | 55.55 | He wasn't hiding | 1.000 | 737.3 | 1.0000 | 737.3 | 745 | 7.7 | pass | pass | pass |
| 69 | 12 | 56.21 | from the war. | 1.000 | 586.5 | 1.0000 | 586.5 | 596 | 9.5 | pass | pass | pass |
| 70 | 12 | 57.15 | He was just | 1.000 | 505.5 | 1.0000 | 505.5 | 516 | 10.5 | pass | pass | pass |
| 71 | 12 | 57.73 | still loyal | 1.000 | 395.4 | 1.0000 | 395.4 | 405 | 9.6 | pass | pass | pass |
| 72 | 12 | 58.35 | to it. | 1.000 | 200.9 | 1.0000 | 200.9 | 210 | 9.1 | pass | pass | pass |

## /home/hatch/faceless-shorts/the-lighthouse-cats-last-shift/native/film/onoda.json — 43 chunks
font /home/hatch/workspace/custom-skills/tall-tale-shorts/template/fonts/Inter-Black.otf (md5 c26b4e77dc103fd3a1e3b426549296ed); CAP 900 88px Inter, maxW 960; floors: rendered > 850 = WIDTH, fit < 0.68 = FIT
authored fit values in data: [1.0]; chunks whose measured fit differs: 1; min measured fit 0.9233; max PIL rawW 1039.7 px; max strip ink 1049 px
fails — authored/native (binary audit as data stands): 3; measured/font (Chrome semantics): 3; measured/native (binary audit, measured fit written): 3; chunks changing verdict vs authored/native: 0

| idx | line | s (s) | text | authored fit | PIL rawW | measured fit | rendered (font) | strip ink | ink−raw | verdict authored/native | verdict measured/font | verdict measured/native |
|---:|---:|---:|---|---:|---:|---:|---:|---:|---:|---|---|---|
| 0 | 1 | 0.27 | For thirty-one years, | 1.000 | 917.9 | 1.0000 | 917.9 | 922 | 4.1 | WIDTH | WIDTH | WIDTH |
| 1 | 1 | 1.89 | a cat named | 1.000 | 532.0 | 1.0000 | 532.0 | 542 | 10.0 | pass | pass | pass |
| 2 | 1 | 2.59 | Biscuit kept | 1.000 | 522.3 | 1.0000 | 522.3 | 533 | 10.7 | pass | pass | pass |
| 3 | 1 | 3.25 | the lighthouse burning. | 1.000 | 1039.7 | 0.9233 | 960.0 | 1049 | 9.3 | WIDTH | WIDTH | WIDTH |
| 4 | 2 | 4.67 | Every night | 1.000 | 507.0 | 1.0000 | 507.0 | 518 | 11.0 | pass | pass | pass |
| 5 | 2 | 5.09 | at dusk, she | 1.000 | 531.9 | 1.0000 | 531.9 | 544 | 12.1 | pass | pass | pass |
| 6 | 2 | 6.19 | climbed the | 1.000 | 518.8 | 1.0000 | 518.8 | 530 | 11.2 | pass | pass | pass |
| 7 | 2 | 6.71 | ninety-nine iron | 1.000 | 710.2 | 1.0000 | 710.2 | 718 | 7.8 | pass | pass | pass |
| 8 | 2 | 7.87 | steps alone, | 1.000 | 542.4 | 1.0000 | 542.4 | 549 | 6.6 | pass | pass | pass |
| 9 | 3 | 9.19 | she trimmed | 1.000 | 555.4 | 1.0000 | 555.4 | 566 | 10.6 | pass | pass | pass |
| 10 | 3 | 9.65 | the wick, she | 1.000 | 581.0 | 1.0000 | 581.0 | 595 | 14.0 | pass | pass | pass |
| 11 | 3 | 10.77 | polished the great | 1.000 | 798.5 | 1.0000 | 798.5 | 809 | 10.5 | pass | pass | pass |
| 12 | 3 | 11.69 | glass lens, | 1.000 | 477.7 | 1.0000 | 477.7 | 484 | 6.3 | pass | pass | pass |
| 13 | 4 | 12.91 | and she watched | 1.000 | 747.8 | 1.0000 | 747.8 | 758 | 10.2 | pass | pass | pass |
| 14 | 4 | 13.47 | the fog roll | 1.000 | 471.5 | 1.0000 | 471.5 | 483 | 11.5 | pass | pass | pass |
| 15 | 4 | 14.33 | in like a | 1.000 | 327.7 | 1.0000 | 327.7 | 335 | 7.3 | pass | pass | pass |
| 16 | 4 | 14.99 | slow grey sea. | 1.000 | 635.7 | 1.0000 | 635.7 | 643 | 7.3 | pass | pass | pass |
| 17 | 5 | 16.45 | Then one | 1.000 | 411.2 | 1.0000 | 411.2 | 423 | 11.8 | pass | pass | pass |
| 18 | 5 | 16.83 | October night, | 1.000 | 641.8 | 1.0000 | 641.8 | 647 | 5.2 | pass | pass | pass |
| 19 | 5 | 17.99 | the supply boat | 1.000 | 687.3 | 1.0000 | 687.3 | 702 | 14.7 | pass | pass | pass |
| 20 | 5 | 18.79 | never came. | 1.000 | 548.0 | 1.0000 | 548.0 | 553 | 5.0 | pass | pass | pass |
| 21 | 6 | 19.73 | The oil | 1.000 | 295.6 | 1.0000 | 295.6 | 306 | 10.4 | pass | pass | pass |
| 22 | 6 | 20.11 | ran low, the | 1.000 | 505.5 | 1.0000 | 505.5 | 515 | 9.5 | pass | pass | pass |
| 23 | 6 | 21.21 | wind rose, and | 1.000 | 642.1 | 1.0000 | 642.1 | 654 | 11.9 | pass | pass | pass |
| 24 | 6 | 22.23 | a fishing boat | 1.000 | 601.5 | 1.0000 | 601.5 | 614 | 12.5 | pass | pass | pass |
| 25 | 6 | 22.89 | was lost in | 1.000 | 467.2 | 1.0000 | 467.2 | 479 | 11.8 | pass | pass | pass |
| 26 | 6 | 23.65 | the dark. | 1.000 | 398.7 | 1.0000 | 398.7 | 408 | 9.3 | pass | pass | pass |
| 27 | 7 | 24.35 | So Biscuit did | 1.000 | 596.3 | 1.0000 | 596.3 | 607 | 10.7 | pass | pass | pass |
| 28 | 7 | 25.15 | the one thing | 1.000 | 579.7 | 1.0000 | 579.7 | 592 | 12.3 | pass | pass | pass |
| 29 | 7 | 25.85 | no keeper had | 1.000 | 620.2 | 1.0000 | 620.2 | 628 | 7.8 | pass | pass | pass |
| 30 | 7 | 26.89 | ever taught her. | 1.000 | 703.2 | 1.0000 | 703.2 | 710 | 6.8 | pass | pass | pass |
| 31 | 8 | 27.79 | She dragged the | 1.000 | 719.7 | 1.0000 | 719.7 | 731 | 11.3 | pass | pass | pass |
| 32 | 8 | 28.51 | spare oil up, | 1.000 | 539.7 | 1.0000 | 539.7 | 546 | 6.3 | pass | pass | pass |
| 33 | 8 | 29.81 | one small can | 1.000 | 603.5 | 1.0000 | 603.5 | 613 | 9.5 | pass | pass | pass |
| 34 | 8 | 30.63 | at a time, | 1.000 | 405.7 | 1.0000 | 405.7 | 412 | 6.3 | pass | pass | pass |
| 35 | 9 | 31.41 | The light | 1.000 | 389.9 | 1.0000 | 389.9 | 403 | 13.1 | pass | pass | pass |
| 36 | 9 | 31.75 | never went out. | 1.000 | 692.0 | 1.0000 | 692.0 | 697 | 5.0 | pass | pass | pass |
| 37 | 10 | 32.75 | The boat found | 1.000 | 668.3 | 1.0000 | 668.3 | 679 | 10.7 | pass | pass | pass |
| 38 | 10 | 33.35 | the harbour. | 1.000 | 547.3 | 1.0000 | 547.3 | 556 | 8.7 | pass | pass | pass |
| 39 | 11 | 34.17 | And at dawn, | 1.000 | 583.1 | 1.0000 | 583.1 | 589 | 5.9 | pass | pass | pass |
| 40 | 11 | 34.91 | Biscuit finally slept, | 1.000 | 882.6 | 1.0000 | 882.6 | 887 | 4.4 | WIDTH | WIDTH | WIDTH |
| 41 | 11 | 36.49 | the lighthouse | 1.000 | 639.9 | 1.0000 | 639.9 | 654 | 14.1 | pass | pass | pass |
| 42 | 11 | 36.99 | still burning. | 1.000 | 564.9 | 1.0000 | 564.9 | 572 | 7.1 | pass | pass | pass |

wrote measurement.json
