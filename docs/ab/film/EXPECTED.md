# EXPECTED — the whole-film A/B expectation table

Written **before** any native film frames exist, from
`scenes/film/film.json` (frame ranges, lines) and the project's
review set, so the measured table in `ab-report.md` lands next to
its hypotheses. Risk notes name the known open items per segment
(from the wave's divergence register and audits); they are
hypotheses to check, not excuses pre-registered — a segment whose
numbers are low *and* whose risk note predicts the observed delta
pattern has an explained result; a low number with a different
pattern is a new finding.

## Reference set — verified first-hand (2026-11-14)

`~/faceless-shorts/the-soldier-who-didnt-know/review/frames/`:
**exactly 60 files, `t0.50.jpg` … `t59.50.jpg`, one per second at
.50 offsets, no gaps** (1080×1920, Chrome SwiftShader stills, one
q0.93 JPEG generation — see the floor discussion in README.md).
Pairing (measure_ab film mode): every review timestamp inside a
segment pairs with native frame `n = round(t·30)`; on the .50
grid, frame n is staged at exactly t = n/30. The tool's plumbing
self-check against this set: **60/60 pairs**, per-segment counts
6/7/3/5/4/1/4/4/6/2/3/6/5/4 — matching the table below.

## Segment table

Mid = golden staging frame (`startFrame + frames//2`, the
`run_golden.sh` computation). Lines are `film/onoda.json` line
indices with text.

| seg | frames [start, end) | n | mid | review pairs (t → native frame) | line(s) | narration |
|---|---|---|---|---|---|---|
| s01 | [0, 178) | 178 | 89 | 0.50→15, 1.50→45, 2.50→75, 3.50→105, 4.50→135, 5.50→165 | 0 | "This soldier kept fighting World War II for 29 years after it ended." |
| s02 | [178, 392) | 214 | 285 | 6.50→195, 7.50→225, 8.50→255, 9.50→285, 10.50→315, 11.50→345, 12.50→375 | 1 | "Hiroo Onoda was a Japanese soldier sent to a Philippine island in 1944." |
| s03 | [392, 479) | 87 | 435 | 13.50→405, 14.50→435, 15.50→465 | 2 | "His orders were simple: never surrender." |
| s04 | [479, 642) | 163 | 560 | 16.50→495, 17.50→525, 18.50→555, 19.50→585, 20.50→615 | 3 | "In 1945, the war ended. But Onoda never got the message." |
| s05a | [642, 737) | 95 | 689 | 21.50→645, 22.50→675, 23.50→705, 24.50→735 | 4 | "Planes dropped leaflets saying the war was over. He thought it was an enemy trick." |
| s05b | [737, 784) | 47 | 760 | 25.50→765 | 4 (cont.) | (same line, second half) |
| s06 | [784, 912) | 128 | 848 | 26.50→795, 27.50→825, 28.50→855, 29.50→885 | 5, 6 | "His friends gave up or died, until he was alone." / line 6 begins inside this segment |
| s07 | [912, 1027) | 115 | 969 | 30.50→915, 31.50→945, 32.50→975, 33.50→1005 | 6 | "He lived in the jungle for decades, surviving on coconuts and bananas." |
| s08 | [1027, 1211) | 184 | 1119 | 34.50→1035, 35.50→1065, 36.50→1095, 37.50→1125, 38.50→1155, 39.50→1185 | 7 | "In 1974, a young traveler named Norio Suzuki found him. Onoda still refused to leave." |
| s08b | [1211, 1252) | 41 | 1231 | 40.50→1215, 41.50→1245 | 7 (cont.) | (same line, second half) |
| s09 | [1252, 1361) | 109 | 1306 | 42.50→1275, 43.50→1305, 44.50→1335 | 8 | "He said he would only stop if his own commander ordered it." |
| s10 | [1361, 1520) | 159 | 1440 | 45.50→1365, 46.50→1395, 47.50→1425, 48.50→1455, 49.50→1485, 50.50→1515 | 9 | "So Japan found his old commander, now a bookseller, and flew him to the island." |
| s11 | [1520, 1666) | 146 | 1593 | 51.50→1545, 52.50→1575, 53.50→1605, 54.50→1635, 55.50→1665 | 10 | "He finally gave the order, and Onoda surrendered after 29 years." |
| s12 | [1666, 1811) | 145 | 1738 | 56.50→1695, 57.50→1725, 58.50→1755, 59.50→1785 | 11 | "He wasn't hiding from the war. He was just still loyal to it." |

Totals: 1,811 frames tiling [0, 1811); 60 pairs; 14 mids.

## Risk notes per segment

- **s01 — the anchor.** t = 1.50 (frame 45) is the milestone
  pair: §28 20.152 dB → M4 20.218 dB → **M5 full chain
  23.850 dB** (%px > 10 = 13.5 %). Detailed-foliage content
  class; the film-mode number for this pair should land in
  that neighbourhood, and a large deviation from it is the
  first thing to investigate. (The flames' flicker bake was
  corrected to f(0) in the data during the wave — C9.)
- **s02 — march clamp.** The film evaluator clamps the march
  driver (travelled stops at k = 0.97) where the source keeps
  travelling; the data records the source formulas. Late pairs
  (t 11.50 / 12.50) may show a character-position delta — an
  evaluator-semantics difference, not rasterisation error.
- **s03 — short + rifle.** Only 3 pairs over 87 frames. The
  rifle is drawn by the film path (the data carries it as an
  on-node attachment); watch the rifle region in the diffs.
- **s04 — evaluated states.** Bob/headTurn values in this file
  were among the 69 recomputed at exact f32 (validator 7/7);
  agreement here is evidence the recomputation is right.
- **s05a — flatter content.** Sky/leaflet content sits nearer
  the flat-content floor (≈ 51–58 dB ceiling) than the
  detailed-frame ceiling (≈ 42 dB); read it against its own
  class, not against s01.
- **s05b — one pair only.** t 25.50 is the segment's entire
  film-mode evidence. Neither a good nor a bad single number
  characterises the segment; the golden mid (frame 760) is a
  second, hash-level check.
- **s06 — FOG (the declared residual risk).** Effective fog
  #C9CFC2, near 4.5, far 20, three.js **smoothstep** factor
  (the classic linear ramp misses fog-chart panel centres by
  up to 27 sRGB codes — `tools/fog-chart/expected.json`).
  Signature of a fog-formula error: broad low-amplitude delta
  growing with scene depth across all four pairs. Run the
  fog-chart verification before attributing a low s06 to
  staging.
- **s07 — CALENDAR.** The card's glyphs are hand-authored
  vector approximations natively vs Inter plates in Chrome;
  expect the card region to dominate %px > 10 regardless of
  the rest of the frame. Pair phases: t 30.50 = year 1945
  steady; t 31.50 and 32.50 fall inside the flip cycle (year
  flips at film t 31.38 / 32.42 / 33.46); t 33.50 = 1974
  steady. Note which render path produced the frames: this
  file's grade uses the `saturation`/`exposure` spellings the
  still loader does not read (the film path is unaffected).
- **s08 — firelight.** Fire intensity is an evaluated channel
  (post-atmosphere basis 4.6, ruling R1); flame and fire-lit
  regions are the expected delta centre.
- **s08b — shortest segment.** 1.372 s / 41 frames, 2 pairs;
  thin evidence by construction, like s05b.
- **s09 — quiet segment.** No register items; its fire-channel
  atTime value was recomputed bit-identical in the validator
  pass. A low number here has no pre-registered hypothesis —
  treat as a new finding.
- **s10 — bookshop shades.** Interior derived shades are the
  linear-shade corpus (#413427 family). The shade root-fix
  (commit c00897d) aligned the native helper with Chrome's
  arithmetic, so a post-fix binary should agree; a pre-fix
  binary reads ~25–40 LSB dark on derived shades — signature:
  broad drift with falling mean |Δ| but stubborn %px > 10.
  310 books from the source's own shelf arithmetic; the
  interior is not atmosphere-dressed (own key #FFE4B8 ×2.4).
  **Record the binary's commit in the results** — it decides
  which expectation applies.
- **s11 — dusk fog + rifle drop.** Fog here is real
  scene.fog (dusk preset #F59C6B, near 28, far 120), not
  grade-only. The rifle drop's end state was derived
  independently by two seats with exact agreement (position
  [0.62, 0.07, −0.1], euler [0, −0.5, π/2]) and s12's ground
  rifle continues from it — the late pairs test that chain.
  An eased-vs-step lean difference in the film path is on the
  open register; mid-segment pose deltas are its signature.
- **s12 — dark outro.** The Chrome original is itself very
  dark here (a known property of the source film, carried
  deliberately). On near-black content, tiny absolute errors
  swing PSNR widely — read **mean |Δ| and %px > 10 first**,
  PSNR second, before concluding anything.

## Cross-cutting expectations

- **Renderer matters:** GRADE grain hash is GPU-vendor
  sensitive (M5). A CI-lavapipe run and a PC-RADV run of the
  same frames may differ slightly in grain regions; both are
  valid evidence **when labelled** — never merge them into
  one table.
- **Ceilings:** no pair can exceed ≈ 42 dB (detailed) or
  ≈ 51–58 dB (flat) against these JPEG references, however
  perfect the native frame (README.md, JPEG floor).
- **The floor of concern:** the headless-gl band
  (15.8–19.3 dB) is what a different renderer looks like.
  Segment means near or below it mean the staging — not the
  rasteriser — needs the next look, via the telemetry gate.
