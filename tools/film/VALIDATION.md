# Film validation — run record (seat W27, validator v2)

- **Date:** 2026-11-14 (Asia/Dhaka)
- **Validator:** `tools/film/validate_film.py` v2 (exit code 1)
- **Manifest:** `scenes/film/film.json` — 14 segments, 30 fps, 1811 frames,
  frame ranges tile [0, 1811) exactly; caption ranges assign each chunk
  to the segment containing its start.
- **Segment files present at run time (14/14):**
  s01, s02, s03, s04, s05a, s05b, s06, s07, s08, s08b, s09, s10, s11, s12
- **Segment files missing at run time:** none — the six files missing in
  the W20 run (s08, s08b, s09, s10, s11, s12) have all landed.
- **What changed since the W20 run (was 3/7 PASS):**
  - Check 3 (f32) refined into two classes: exact f32 round-trip, or
    SOURCE LITERAL (≤ 6 significant digits and the float value appears
    as a numeric token in the project source tree — 15 files grepped
    under `/home/hatch/faceless-shorts/the-soldier-who-didnt-know/src`).
    Of 1389 numbers checked: 814 exact, 506 source literals (pass),
    69 computed values written rounded (fail, listed verbatim below).
  - Check 7 (atmosphere) now implements ruling R5 — scene files name
    presets, and (set, preset) resolves to a filmLook via
    `scenes/atmospheres.json`: PASS (14/14 presets match the manifest,
    14/14 resolve). The W20 strict filmLooks-key reading is retired.
  - Caption ownership fixed: chunk 31 removed from `s06.scene.json`
    (starts 25.57, owned by s05b) and chunk 39 removed from
    `s07.scene.json` (starts 30.29, owned by s06); bleed notes recorded
    as a top-level `captionsNote` in both files and in
    `scenes/film/ASSUMPTIONS.md` (seat W27 section).
- **Headline:** 5/7 checks PASS — parse (14/14 present), version,
  attachments (0 top-level violations under §14.5 R2; 10 attachments on
  character nodes), durations (frame tiling exact, 60.357 s -> 1811
  frames), atmosphere (R5). Remaining failures, every one listed
  verbatim in the output below:
  - **f32:** 69 computed values do not round-trip as f32 and are not
    source literals — mostly camera key times, channel key times, and
    evaluated state values written rounded.
  - **captions:** chunk 46 listed 2 times (in s08, s08b) and chunk 56
    listed 2 times (in s10, s11) — the same start-time ownership
    defect as chunks 31/39, in files outside seat W27's edit scope.
    Chunk 46 starts at 39.95 (owned by s08); chunk 56 starts at 49.63
    (owned by s10). Flagged for a follow-up seat. All 66 chunks are
    covered; 0 uncovered.

## Validator output (verbatim)

```
validate_film.py — THE SOLDIER WHO DIDN'T KNOW THE WAR WAS OVER (14 segments, 30 fps, 1811 frames)

[1 parse] PASS — 14 present, 0 missing, 0 unparsable
    present: 14/14 segment files (s01, s02, s03, s04, s05a, s05b, s06, s07, s08, s08b, s09, s10, s11, s12)

[2 version] PASS — 14 files checked, 0 wrong

[3 f32] FAIL — 1389 numbers checked: 814 exact f32, 506 source literals (pass), 69 computed values not round-tripping (fail); literals grepped from 15 source files under /home/hatch/faceless-shorts/the-soldier-who-didnt-know/src
    scenes/film/s02.scene.json: camera.keys[0].t = 5.907 -> f32 5.9070000648498535 (computed value, not a source literal)
    scenes/film/s02.scene.json: camera.keys[1].t = 13.061 -> f32 13.060999870300293 (computed value, not a source literal)
    scenes/film/s02.scene.json: nodes[49] (character 'onoda').position[2] = -7.6 -> f32 -7.599999904632568 (computed value, not a source literal)
    scenes/film/s02.scene.json: nodes[51] (character 'compB').position[2] = -9.1 -> f32 -9.100000381469727 (computed value, not a source literal)
    scenes/film/s03.scene.json: camera.keys[1].t = 0.2889 -> f32 0.2888999879360199 (computed value, not a source literal)
    scenes/film/s03.scene.json: camera.keys[2].t = 2.6001 -> f32 2.600100040435791 (computed value, not a source literal)
    scenes/film/s03.scene.json: camera.keys[3].t = 2.889 -> f32 2.8889999389648438 (computed value, not a source literal)
    scenes/film/s03.scene.json: nodes[49] (character 'Taniguchi (shore)').channels.armR.keys[1][0] = 1.589 -> f32 1.5889999866485596 (computed value, not a source literal)
    scenes/film/s03.scene.json: nodes[50] (character 'Onoda (shore)').channels.blink.at[0].t = 14.78 -> f32 14.779999732971191 (computed value, not a source literal)
    scenes/film/s04.scene.json: camera.keys[0].t = 15.95 -> f32 15.949999809265137 (computed value, not a source literal)
    scenes/film/s04.scene.json: camera.keys[1].t = 21.384 -> f32 21.384000778198242 (computed value, not a source literal)
    scenes/film/s04.scene.json: nodes[80] (character 'compA').state.bob = -3.9234 -> f32 -3.9233999252319336 (computed value, not a source literal)
    scenes/film/s04.scene.json: nodes[81] (character 'compB').state.bob = -3.4758 -> f32 -3.475800037384033 (computed value, not a source literal)
    scenes/film/s04.scene.json: nodes[82] (character 'onoda').channels.blink.maxOf[1].blinkAt.beats[0] = 20.7 -> f32 20.700000762939453 (computed value, not a source literal)
    scenes/film/s04.scene.json: nodes[82] (character 'onoda').state.headTurn = -0.349993 -> f32 -0.3499929904937744 (computed value, not a source literal)
    scenes/film/s05a.scene.json: camera.keys[0].t = 21.384 -> f32 21.384000778198242 (computed value, not a source literal)
    scenes/film/s05a.scene.json: camera.keys[1].t = 24.56 -> f32 24.559999465942383 (computed value, not a source literal)
    scenes/film/s05b.scene.json: camera.keys[0].t = 24.56 -> f32 24.559999465942383 (computed value, not a source literal)
    scenes/film/s05b.scene.json: camera.keys[1].t = 26.129 -> f32 26.128999710083008 (computed value, not a source literal)
    scenes/film/s05b.scene.json: lights.points[0].intensity = 4.968072 -> f32 4.968071937561035 (computed value, not a source literal)
    scenes/film/s05b.scene.json: nodes[80] (character 'onoda').channels.armL.poseTrack.keys[1][0] = 1.59 -> f32 1.590000033378601 (computed value, not a source literal)
    scenes/film/s06.scene.json: nodes[81] (character 'compB').channels.position.keys[1][1][2] = -10.9 -> f32 -10.899999618530273 (computed value, not a source literal)
    scenes/film/s06.scene.json: nodes[82] (character 'onoda').channels.position.keys[2][0] = 3.8 -> f32 3.799999952316284 (computed value, not a source literal)
    scenes/film/s07.scene.json: camera.keys[1].t = 3.819 -> f32 3.819000005722046 (computed value, not a source literal)
    scenes/film/s07.scene.json: nodes[78] (character 'Onoda').channels.armR.keys[1][0] = 1.91 -> f32 1.909999966621399 (computed value, not a source literal)
    scenes/film/s07.scene.json: nodes[78] (character 'Onoda').channels.armR.keys[2][0] = 2.86 -> f32 2.859999895095825 (computed value, not a source literal)
    scenes/film/s07.scene.json: nodes[78] (character 'Onoda').channels.camPitch.atEnd = 0.4726 -> f32 0.4726000130176544 (computed value, not a source literal)
    scenes/film/s07.scene.json: nodes[78] (character 'Onoda').channels.camPitch.atStart = 0.1108 -> f32 0.11079999804496765 (computed value, not a source literal)
    scenes/film/s07.scene.json: nodes[78] (character 'Onoda').channels.holdL.atFilm = 32.35 -> f32 32.349998474121094 (computed value, not a source literal)
    scenes/film/s07.scene.json: nodes[78] (character 'Onoda').state.camPitch = 0.1108 -> f32 0.11079999804496765 (computed value, not a source literal)
    scenes/film/s08.scene.json: camera.keys[0].t = 34.208999999999996 -> f32 34.20899963378906 (computed value, not a source literal)
    scenes/film/s08.scene.json: camera.keys[1].t = 40.36 -> f32 40.36000061035156 (computed value, not a source literal)
    scenes/film/s08.scene.json: nodes[81] (character 'suzuki').channels.blink.maxOf[1].blinkAt[0] = 38.48 -> f32 38.47999954223633 (computed value, not a source literal)
    scenes/film/s08.scene.json: nodes[81] (character 'suzuki').channels.mood.at = 38.48 -> f32 38.47999954223633 (computed value, not a source literal)
    scenes/film/s08b.scene.json: camera.eye[0] = 0.275 -> f32 0.2750000059604645 (computed value, not a source literal)
    scenes/film/s08b.scene.json: camera.eye[1] = 1.74 -> f32 1.7400000095367432 (computed value, not a source literal)
    scenes/film/s08b.scene.json: camera.eye[2] = 4.824999999999999 -> f32 4.824999809265137 (computed value, not a source literal)
    scenes/film/s09.scene.json: camera.keys[0].t = 41.732 -> f32 41.731998443603516 (computed value, not a source literal)
    scenes/film/s09.scene.json: camera.keys[1].t = 45.365 -> f32 45.3650016784668 (computed value, not a source literal)
    scenes/film/s09.scene.json: lights.points[0].channel.atTime = 4.076015727691372 -> f32 4.076015949249268 (computed value, not a source literal)
    scenes/film/s09.scene.json: nodes[80] (character 'onoda').channels.armR.elbowKeys[1][0] = 2.188 -> f32 2.187999963760376 (computed value, not a source literal)
    scenes/film/s09.scene.json: nodes[80] (character 'onoda').channels.armR.elbowKeys[2][0] = 3.408 -> f32 3.4079999923706055 (computed value, not a source literal)
    scenes/film/s09.scene.json: nodes[80] (character 'onoda').channels.armR.keys[1][0] = 2.188 -> f32 2.187999963760376 (computed value, not a source literal)
    scenes/film/s09.scene.json: nodes[80] (character 'onoda').channels.armR.keys[2][0] = 3.408 -> f32 3.4079999923706055 (computed value, not a source literal)
    scenes/film/s09.scene.json: nodes[80] (character 'onoda').channels.blink.beat.at = 44.04 -> f32 44.040000915527344 (computed value, not a source literal)
    scenes/film/s09.scene.json: nodes[81] (character 'suzuki').channels.pose.switchAt = 42.82 -> f32 42.81999969482422 (computed value, not a source literal)
    scenes/film/s09.scene.json: nodes[81] (character 'suzuki').channels.pose.switchAtLocal = 1.088 -> f32 1.0880000591278076 (computed value, not a source literal)
    scenes/film/s10.scene.json: camera.keys[0].t = 45.365 -> f32 45.3650016784668 (computed value, not a source literal)
    scenes/film/s10.scene.json: camera.keys[1].t = 50.648 -> f32 50.64799880981445 (computed value, not a source literal)
    scenes/film/s10.scene.json: nodes[23] (character 'Taniguchi (bookshop)').channels.blink.beats[0] = 48.16 -> f32 48.15999984741211 (computed value, not a source literal)
    scenes/film/s10.scene.json: nodes[23] (character 'Taniguchi (bookshop)').channels.holdR.at = 49.22 -> f32 49.220001220703125 (computed value, not a source literal)
    scenes/film/s10.scene.json: nodes[23] (character 'Taniguchi (bookshop)').channels.mood.steps[0][0] = 45.365 -> f32 45.3650016784668 (computed value, not a source literal)
    scenes/film/s10.scene.json: nodes[23] (character 'Taniguchi (bookshop)').channels.mood.steps[1][0] = 48.16 -> f32 48.15999984741211 (computed value, not a source literal)
    scenes/film/s10.scene.json: nodes[23] (character 'Taniguchi (bookshop)').channels.mood.steps[2][0] = 49.22 -> f32 49.220001220703125 (computed value, not a source literal)
    scenes/film/s11.scene.json: camera.keys[1].t = 4.872 -> f32 4.872000217437744 (computed value, not a source literal)
    scenes/film/s11.scene.json: nodes[80] (character 'Onoda').channels.mood.keys[1][0] = 1.132000000000005 -> f32 1.1319999694824219 (computed value, not a source literal)
    scenes/film/s11.scene.json: nodes[80] (character 'Onoda').channels.position.segment[1] = 0.882 -> f32 0.8820000290870667 (computed value, not a source literal)
    scenes/film/s11.scene.json: nodes[80] (character 'Onoda').channels.visible.window[1] = 2.922000000000004 -> f32 2.921999931335449 (computed value, not a source literal)
    scenes/film/s11.scene.json: nodes[81] (character 'Onoda (second instance)').channels.armR.elbow.keys[1][0] = 3.4720000000000013 -> f32 3.4719998836517334 (computed value, not a source literal)
    scenes/film/s11.scene.json: nodes[81] (character 'Onoda (second instance)').channels.armR.keys[1][0] = 3.4720000000000013 -> f32 3.4719998836517334 (computed value, not a source literal)
    scenes/film/s11.scene.json: nodes[81] (character 'Onoda (second instance)').channels.blink.at[0].t = 53.02 -> f32 53.02000045776367 (computed value, not a source literal)
    scenes/film/s11.scene.json: nodes[81] (character 'Onoda (second instance)').channels.lean.keys[1][0] = 3.4720000000000013 -> f32 3.4719998836517334 (computed value, not a source literal)
    scenes/film/s11.scene.json: nodes[81] (character 'Onoda (second instance)').channels.mood.keys[1][0] = 3.7720000000000056 -> f32 3.7720000743865967 (computed value, not a source literal)
    scenes/film/s11.scene.json: nodes[81] (character 'Onoda (second instance)').channels.visible.window[0] = 2.922000000000004 -> f32 2.921999931335449 (computed value, not a source literal)
    scenes/film/s11.scene.json: nodes[82] (character 'Taniguchi').channels.armR.keys[1][0] = 1.002000000000005 -> f32 1.0019999742507935 (computed value, not a source literal)
    scenes/film/s11.scene.json: nodes[82] (character 'Taniguchi').channels.armR.keys[2][0] = 3.072000000000007 -> f32 3.072000026702881 (computed value, not a source literal)
    scenes/film/s11.scene.json: nodes[82] (character 'Taniguchi').channels.lean.keys[1][0] = 4.0720000000000045 -> f32 4.072000026702881 (computed value, not a source literal)
    scenes/film/s12.scene.json: camera.keys[1].t = 4.837 -> f32 4.836999893188477 (computed value, not a source literal)
    scenes/film/s12.scene.json: nodes[81] (character 'Onoda (JG onoda2)').channels.blink.beats[0] = 58.12 -> f32 58.119998931884766 (computed value, not a source literal)

[4 attachments] PASS — 0 top-level violations, 10 attachments on character nodes

[5 captions] FAIL — 68 chunk listings, 66 unique chunks covered of 66, 2 duplicates, 0 uncovered
    chunk 46 listed 2 times (in s08, s08b)
    chunk 56 listed 2 times (in s10, s11)

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

OVERALL: FAIL (5/7 checks passed)
```
