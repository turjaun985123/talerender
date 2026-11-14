# Caption re-chunk — Option A applied (W01, TaleRender Wave 2, 2026-11-14)

**What this is.** The 7 caption chunks that failed the film telemetry
gate's 850 px width floor have been split at word boundaries, exactly
as computed in [CAPTION-DECISION-W92.md](CAPTION-DECISION-W92.md) §3
(Option A): **66 → 73 chunks**. The splits were applied as data,
through the repo's own caption tooling — no renderer code was touched
(`src/` untouched), nothing is committed (the orchestrator commits
centrally).

**Authority.** The lead's earlier ruling was ACCEPT WITH WAIVER
(recorded in CAPTION-DECISION-W92.md — **that text is unchanged by
this seat**). The user's Wave-2 order (complete all phases of the
audit; its Phase 1 reads "resolve seven caption failures") reopens
the question and directs the re-chunk. This record therefore states:
**the waiver is superseded by the user's order, pending the user's
final ruling at wave close** — the re-chunked data below is what now
stands in the tree; reverting to the waiver state is a data
regeneration from the pre-split `caps` (kept in the project history
of this record, §2), not a code change.

---

## 1. Verdicts after the re-chunk (all run first-hand, this seat)

Generator (the repo tooling's own audit arithmetic):

```
CAPTION STRIPS: 73 chunks, min fit 1.00, max rendered width 835 px, 0 over the 850 px floor, 0 under the 0.68 fit floor
```

Caption determinism (regenerate to a scratch dir, `diff -r` against
`assets/captions/`): **byte-identical over all 74 files** (73 strips
+ manifest).

Film validator, **full mode** (project source tree present on this
VM; no `--no-source`):

```
[3 f32] PASS — 1389 numbers checked: 883 exact f32, 506 source literals (pass), 0 computed values not round-tripping (fail)
[5 captions] PASS — 73 chunk listings, 73 unique chunks covered of 73, 0 duplicates, 0 uncovered
OVERALL: PASS (7/7 checks passed)   exit 0
```

## 2. The splits (input change)

The generator derives chunks from the project's authored `caps`, so
the splits were applied there — the only input the repo tooling
consumes. File: `~/faceless-shorts/the-soldier-who-didnt-know/script.json`
(the tooling's source project; a pre-edit copy was kept in this
seat's scratch). The diff is exactly 7 lines, one `caps` string per
line, each gaining one `|` boundary:

| Line | Before (segment) | After |
|---|---|---|
| 2 | `… \| was a Japanese soldier \| …` | `… \| was a Japanese \| soldier \| …` |
| 5 | `Planes dropped leaflets \| …` | `Planes dropped \| leaflets \| …` |
| 7 | `… \| surviving on coconuts \| …` | `… \| surviving \| on coconuts \| …` |
| 8 | `… \| named Norio Suzuki \| …` | `… \| named Norio \| Suzuki \| …` |
| 9 | `… \| commander ordered it.` | `… \| commander \| ordered it.` |
| 10 | `So Japan found \| his old commander, \| …` | `So Japan found \| his old \| commander, \| …` |
| 11 | `… \| and Onoda surrendered \| …` | `… \| and Onoda \| surrendered \| …` |

Every resulting piece matches W92's computed table to the digit —
text, timings (piece 2 starts at its first word's start − 0.03 s,
from `data/words.json`), and PIL `rawW`:

| Old chunk | Pieces (new indices) | rawW | s–e (s) |
|---|---|---|---|
| cap-014 "was a Japanese soldier" | cap-014 "was a Japanese" / cap-015 "soldier" | 699.34 / 307.17 | 6.69–7.87 / 7.87–8.53 |
| cap-026 "Planes dropped leaflets" | cap-027 "Planes dropped" / cap-028 "leaflets" | 687.14 / 335.77 | 21.61–22.09 / 22.09–22.61 |
| cap-040 "surviving on coconuts" | cap-042 "surviving" / cap-043 "on coconuts" | 425.88 / 546.55 | 31.55–32.01 / 32.01–32.99 |
| cap-044 "named Norio Suzuki" | cap-047 "named Norio" / cap-048 "Suzuki" | 562.33 / 304.25 | 37.13–38.07 / 38.07–38.45 |
| cap-051 "commander ordered it." | cap-055 "commander" / cap-056 "ordered it." | 532.11 / 457.75 | 44.01–44.43 / 44.43–45.395 |
| cap-053 "his old commander," | cap-058 "his old" / cap-059 "commander," | 290.33 / 559.44 | 46.39–46.93 / 46.93–47.77 |
| cap-059 "and Onoda surrendered" | cap-065 "and Onoda" / cap-066 "surrendered" | 473.73 / 548.77 | 52.07–52.99 / 52.99–53.57 |

Renumbering: every chunk after a split point shifts (+1 after
cap-014, +2 after cap-026, … +7 after cap-059). Piece timings are
W92's, so its advisory 0.33 s/word assessment carries over unchanged.

## 3. Census — before / after

| | Before | After |
|---|---|---|
| Chunks | 66 | **73** |
| Per line | 13, 5, 3, 5, 6, 5, 5, 6, 4, 5, 4, 5 | 13, 6, 3, 5, 7, 5, 6, 7, 5, 6, 5, 5 |
| Min fit | 0.9227 | **1.00** (nothing is fit-scaled now) |
| Max renderedW | 960 px (cap) | **835 px** (cap-050 "Onoda still refused", rawW 834.80) |
| Over the 850 px floor | **7** | **0** |
| Widest split piece (rawW) | — | 699.34 px |

## 4. Widths measured with PIL over the regenerated strips

Ink = bbox of alpha > 32 over `assets/captions/cap-*.png` (the
scenes-side strips; all fits are now 1.0, so these strips are
unscaled). The 14 pieces:

| Chunk | rawW | **ink** | Headroom under 850 |
|---|---|---|---|
| cap-014 "was a Japanese" | 699.34 | **728** | 122 |
| cap-015 "soldier" | 307.17 | 335 | 515 |
| cap-027 "Planes dropped" | 687.14 | 710 | 140 |
| cap-028 "leaflets" | 335.77 | 360 | 490 |
| cap-042 "surviving" | 425.88 | 450 | 400 |
| cap-043 "on coconuts" | 546.55 | 572 | 278 |
| cap-047 "named Norio" | 562.33 | 586 | 264 |
| cap-048 "Suzuki" | 304.25 | 328 | 522 |
| cap-055 "commander" | 532.11 | 559 | 291 |
| cap-056 "ordered it." | 457.75 | 479 | 371 |
| cap-058 "his old" | 290.33 | 313 | 537 |
| cap-059 "commander," | 559.44 | 579 | 271 |
| cap-065 "and Onoda" | 473.73 | 498 | 352 |
| cap-066 "surrendered" | 548.77 | 574 | 276 |

Widest piece ink **728 px** — ≥ 122 px of headroom, better than
W92's worst-case projection (≈714 px ink under the driver recipe's
delta, ≥ 80 px headroom guaranteed). Every piece passes on every
measure: rawW ≤ 699.34, renderedW = rawW (fit 1.0), scenes-recipe
ink ≤ 728.

**Two readings over 850 — investigated, not a regression.** Two
*unsplit* chunks measure ink 859 (cap-050 "Onoda still refused",
rawW 834.80) and 852 (cap-053 "he would only stop", rawW 827.75)
on the scenes-side strips. Three facts close this out:

1. The gate does not compare strip ink: the film telemetry audit
   fails a chunk iff `rendered_w > 850.0` (`src/telemetry.rs:1094`,
   read-only inspection). Both chunks' renderedW are 834.80 /
   827.75 — **pass**, as does every one of the 73 (max 835).
2. On the driver-side strips (the recipe the audit's widths were
   reproduced from), the same two chunks measure ink **844** and
   **837** (measured this seat over the current
   `film/assets/cap_046.png` / `cap_049.png`) — under the floor,
   consistent with the old audit flagging exactly 7.
3. The scenes-side recipe (3× supersample + LANCZOS + the offset
   shadow pass) spreads ink wider than the driver recipe. Proof
   the re-chunk changed nothing here: a standalone re-render of
   the two chunks' (unchanged) text/fit through the scenes
   generator is **byte-identical** to the regenerated
   cap-050/cap-053 strips.

## 5. Files changed

**Tooling input (outside the repo):**

- `~/faceless-shorts/the-soldier-who-didnt-know/script.json` —
  the 7 `caps` splits of §2. Nothing else in the file changed
  (verified by diff).

**Repo — regenerated by the tooling:**

- `assets/captions/manifest.json` — 66 → 73 chunks (the tracked
  census record).
- `assets/captions/cap-000.png` … `cap-072.png` — regenerated on
  disk, byte-stable (§1). Only `manifest.json` and `cap-000.png`
  are git-tracked here; the strips are ignored build outputs, and
  `cap-000.png` is byte-unchanged (chunk 0 is untouched).

**Repo — scenes data (`scenes/film/`):**

- `s02, s05a, s07, s08, s09, s10, s11.scene.json` — the split
  chunk's entry replaced by its two piece entries, in the file's
  own entry shape (keys/order preserved per file; piece `fit`
  1.0; piece `width` = generator rawW where the shape carries
  `width`; outer s/e kept verbatim from the old entry, the inner
  boundary taken from the generator's own arithmetic — rounded
  to 3 dp in files whose entries use short decimals, exact float
  elsewhere). All later entries renumbered (`index` / `id` /
  `strip` fields only; s/e/text/fit/width verbatim — asserted
  against the new manifest for every unsplit chunk).
- `s03, s04, s05b, s06, s08b, s12.scene.json` — renumbering only.
- `s01.scene.json` — **unchanged** (chunks 0–12 precede the first
  split; no field moves).
- `captionsNote` bleed references renumbered with the data (the
  relations are unchanged — boundaries and times did not move):
  s03 17→18, s04 20→21, s05a 25→26, s05b 29→31, s06 31→33,
  s07 39→41, s08 41→44, s08b 46→50, s09 47→51, s11 56→62,
  s12 60→67; s02's reference (chunk 12) precedes all splits and
  is unchanged. **s10's note named the split chunk itself** —
  re-pointed from "chunk 51 ('commander ordered it.',
  44.01-45.395)" to "chunk 56 ('ordered it.', 44.43-45.395)",
  the piece that ends the old span and carries the bleed
  relation (verified: 45.395 − s10.startS 45.365 = 0.03 s, the
  note's figure).
- `scenes/film/film.json` — every segment's `captionChunks`
  re-tiled to the new spans; the 14 ranges tile **[0, 73)**
  exactly: s01 0–12 · s02 13–18 · s03 19–21 · s04 22–26 ·
  s05a 27–31 · s05b 32–33 · s06 34–41 · s07 42–44 · s08 45–50 ·
  s08b 51 · s09 52–56 · s10 57–62 · s11 63–67 · s12 68–72.
  Nothing else in the manifest changed (frame tiling untouched;
  validator check 6 still exact at 1,811 frames).

**Repo — tooling:**

- `tools/film/validate_film.py` — check 5's coverage constant
  `EXPECTED_CHUNKS` 66 → 73, with the two hardcoded census
  strings now derived from it and the check-5 docstring updated.
  *Scope note, stated plainly:* this file sits outside this
  seat's nominal write set, but the constant *is* the caption
  census contract — with 66 hardcoded, the validator the task
  requires at 7/7 fails the correct new data by construction
  (same class of edit as W83's validator fix in Wave 1). The
  constant's comment records that `film/onoda.json` still
  carries 66 until the driver regeneration lands.
- `tools/captions/README.md` — census updated (73 strips,
  headline, per-line counts, the 7-chunk section replaced by the
  resolution, M6 cross-check scoped to the pre-re-chunk
  chunking).

**Deliberately NOT changed:**

- `film/onoda.json` and `film/assets/cap_*.png` — the driver-side
  caption data (still 66 chunks/strips). Regenerating them to
  the 73-chunk layout is the lead's integration step (§6); until
  then the two sides intentionally differ and this record is the
  notice.
- `docs/ab/film/CAPTION-DECISION-W92.md` — the waiver text
  stands as written; this record carries the supersession.
- `scenes/film/ASSUMPTIONS.md` — its caption references are to
  `film/onoda.json` indices (derivation provenance); that file
  has not changed, so neither do they.
- The bridge package — **none exists to update**: the Onoda
  project has no `native/` bridge dir on this VM (Wave-1 proofs
  packaged into /tmp). `tools/bridge/package_bridge.py` copies
  the scene files byte-for-byte, so the next packaging run picks
  the re-chunk up automatically — but it must run *with* the
  driver regeneration, or the package would mix 73-chunk scenes
  with 66-chunk driver data.
- `tools/film/VALIDATION.md` and `tools/CI-WORKFLOW.md` quote
  the old 66-chunk census in past run outputs — historical
  records, left as written; flagged here for the doc sweep.

## 6. What remains — the lead's integration steps

1. **Driver-side regeneration** to this same 73-chunk layout:
   `film/onoda.json` `captions[]` (all 73; the per-chunk
   text/s/e/width/fit already stand in the generator manifest
   and, for their segments, in the scene files) and the driver
   strips `film/assets/cap_000.png` … `cap_072.png`.
2. **Telemetry re-run** against the regenerated driver data.
   Expected outcome on the caption section: **0 caption
   failures** — every chunk's renderedW ≤ 835 < 850 — i.e. the
   gate's verdict line becomes `FILM TELEMETRY: CHECK OK` if
   overflow/continuity stay clean. The re-run is the lead's:
   the verdict counts only when the driver data and the gate
   meet, and that meeting is integration, not data prep.
3. **Bridge repackage** (`package_bridge.py --force`) after
   step 1 — the package then carries 73 strips.
4. **Re-render + the A/B consequence, stated as W92 priced it:**
   captions are composited into the measured frames on both
   sides, so after the re-render ≈ **10 of the 60 A/B pairs**
   (the 1 s-sampled stills at t = 7.50, 8.50, 22.50, 32.50,
   37.50, 44.50, 46.50, 47.50, 52.50, 53.50 — inside the seven
   split windows) will show different caption text on the
   native side. The other ≈ 50 pairs are caption-identical.
   Per decision (B) the numbers are reported, never gated —
   but the A/B table needs its footnote, and the staged golden
   mids in the affected windows predate the re-chunk (pinning
   remains a human act, as before).

---

*Evidence base (first-hand, this seat): the 7-line project diff;
the generator's audit headline; determinism `diff -r` over 74
files; the validator's full-mode 7/7 output; PIL ink bboxes
(alpha > 32) over all 73 regenerated strips and over the two
widest driver strips; byte-identity probes for the two over-850
scenes-recipe readings; the s10 bleed arithmetic
(45.395 − 45.365 = 0.03). Transform assertions (66 → 73 global
sequence, unsplit entries verbatim, manifest cross-match on every
chunk) all passed in the seat's transform run.*
