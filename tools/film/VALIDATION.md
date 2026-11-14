# Film validation — run record (seat W31, validator v2)

- **Date:** 2026-11-14 (Asia/Dhaka)
- **Validator:** `tools/film/validate_film.py` v2 (exit code 0)
- **Manifest:** `scenes/film/film.json` — 14 segments, 30 fps, 1811 frames,
  frame ranges tile [0, 1811) exactly; caption ranges assign each chunk
  to the segment containing its start.
- **Segment files present at run time (14/14):**
  s01, s02, s03, s04, s05a, s05b, s06, s07, s08, s08b, s09, s10, s11, s12
- **Segment files missing at run time:** none.
- **What changed since the W27 run (was 5/7 PASS):**
  - Check 3 (f32) now PASSes. All 69 computed values listed by W27 were
    recomputed in f64 from their documented sources — cut times from
    `film/onoda.json` line starts via cut(i) = lines[i].start − 0.06;
    channel/beat times from the `anchors` table and each shot's
    ASSUMPTIONS formulas; evaluated states from poseTrack / camK / bob /
    fireLevel / camPitchOf transcribed from the project sources — and
    rewritten as the exact f32 of the recomputed value, in the files'
    shortest-round-trip spelling. 883 exact f32 + 506 source literals,
    0 computed failures. Every replacement differs from the rounded
    decimal it supersedes by less than 5×10⁻⁵ (the authoring seats'
    own rounding; max 4.84×10⁻⁵); the one full-precision value in the
    set (s09 fire `atTime`) recomputed bit-identical to the seat's f64.
    Method and per-file counts: `scenes/film/ASSUMPTIONS.md`, seat W31
    section.
  - Check 5 (captions) now PASSes. Chunk 46 removed from
    `s08b.scene.json` (starts 39.95, owned by s08) and chunk 56 removed
    from `s11.scene.json` (starts 49.63, owned by s10) — the same
    start-time ownership fix W27 applied to chunks 31/39, with the
    bleed recorded as a top-level `captionsNote` in both files.
    66 listings, 66 unique, 0 duplicates, 0 uncovered.
  - Node indices in W27's listing had shifted before this run (seats
    W26/W32 regrouped crate/barrel nodes in the SH/JG files); the 69
    values themselves were unchanged and were re-located from a fresh
    validator run.
- **Headline:** 7/7 checks PASS — parse (14/14 present), version, f32
  (883 exact + 506 source literals, 0 computed failures), attachments
  (0 top-level violations under §14.5 R2; 10 attachments on character
  nodes), captions (66/66 unique, 0 duplicates, 0 uncovered), durations
  (frame tiling exact, 60.357 s -> 1811 frames), atmosphere (R5).

## Validator output (verbatim)

```
validate_film.py — THE SOLDIER WHO DIDN'T KNOW THE WAR WAS OVER (14 segments, 30 fps, 1811 frames)

[1 parse] PASS — 14 present, 0 missing, 0 unparsable
    present: 14/14 segment files (s01, s02, s03, s04, s05a, s05b, s06, s07, s08, s08b, s09, s10, s11, s12)

[2 version] PASS — 14 files checked, 0 wrong

[3 f32] PASS — 1389 numbers checked: 883 exact f32, 506 source literals (pass), 0 computed values not round-tripping (fail); literals grepped from 15 source files under /home/hatch/faceless-shorts/the-soldier-who-didnt-know/src

[4 attachments] PASS — 0 top-level violations, 10 attachments on character nodes

[5 captions] PASS — 66 chunk listings, 66 unique chunks covered of 66, 0 duplicates, 0 uncovered

[6 durations] PASS — sum(durations) = 60.357 s -> 1811 frames (total 1811); tiling exact

[7 atmosphere] PASS — R5: declared presets match the manifest for 14/14 files; (set, preset) -> filmLook resolves via atmospheres.json for 14/14 segments
    -- derived mapping (context) --
    s01: set JG x preset 'noon' -> filmLook 'jg-noon' (atmospheres.json shots list)
    s02: set SH x preset 'noon' -> filmLook 'sh-noon' (atmospheres.json shots list)
    s03: set SH x preset 'noon' -> filmLook 'sh-noon' (atmospheres.json shots list)
    s04: set JG x preset 'noon' -> filmLook 'jg-noon' (atmospheres.json shots list)
    s05a: set JG x preset 'noon' -> filmLook 'jg-noon' (atmospheres.json shots list)
    s05b: set JG x preset 'noon' -> filmLook 'jg-noon' (atmospheres.json shots list)
    s06: set JG x preset 'fog' -> filmLook 'jg-fog-s06' (atmospheres.json shots list)
    s07: set JG x preset 'golden' -> filmLook 'jg-golden-s07' (atmospheres.json shots list)
    s08: set JG x preset 'noon' -> filmLook 'jg-noon' (atmospheres.json shots list)
    s08b: set JG x preset 'noon' -> filmLook 'jg-noon' (atmospheres.json shots list)
    s09: set JG x preset 'noon' -> filmLook 'jg-noon' (atmospheres.json shots list)
    s10: set BK x preset 'noon' -> filmLook 'bk-noon-shop' (atmospheres.json shots list)
    s11: set JG x preset 'dusk' -> filmLook 'jg-dusk' (atmospheres.json shots list)
    s12: set JG x preset 'dusk' -> filmLook 'jg-dusk' (atmospheres.json shots list)

OVERALL: PASS (7/7 checks passed)
```
