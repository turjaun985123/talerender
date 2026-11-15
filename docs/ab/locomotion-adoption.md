# Locomotion adoption — A/B evidence note (2026-11-15)

Decision 3 of DECISIONS-W94 (user ruling 2026-11-15 07:21 +06):
adopt the locomotion module at the seven film walk sites, with
D1 (full replacement incl. standing frames), D2 (companions keep
the (8,6)/(8,6) resting base), D3 (arm swing only on advancing
frames), D4 (s11 authored arms untouched; adopted render becomes
the reference of record). Wiring commit: `56348ac`. Step 0
(registration, `b6d1fd0`) proof: CI test count 180 → 189; telemetry
CHECK OK; and the two CI renders below differ in **0 of 1,811
frames** between the pre-step-0 head (`bf1b27c`) and the step-0-era
head — the standing-range probes are 40/40 byte-identical, as is
the whole film.

## Method

Full 1,811-frame CI renders (render.yml, lavapipe), per
LOCOMOTION-AB-W57 §5 with the baseline re-struck at the
pre-adoption head as §5.1 sanctions:

- Baseline (pre-adoption): head `06d2081` (src/ identical to the
  step-0 head `b6d1fd0`), run `43664830922`, SUCCESS.
- Post-adoption: head `56348ac`, run `43665874388`, SUCCESS.
- Pre-step-0 reference: head `bf1b27c`, run `43665150684`, SUCCESS.

Frames were compared by SHA-256 over all 1,811 pairs. Contact
sheets and render frames are rendered photos: under the user's
2026-11-15 rule they are NOT in git — they are held locally in the
evidence folder and delivered to the user as a zip. This note
carries the hashes/metrics as text.

## Classification (W57 §5 ranges)

- Changed: **858 frames**, ALL inside the gait footprint.
- Footprint (888 frames): 858 changed. Per range: s02 178-391:
  214/214 · s06 784-911: 128/128 · s07 912-1026: 101/115 ·
  s08 1027-1210: 184/184 · s10 1361-1519: 159/159 ·
  s11 1520-1607: 72/88. Changed runs: 178-391; 784-911;
  926-1210; 1361-1519; 1536-1607.
- The 30 unchanged footprint frames are the pre-driver rest frames
  of s07 (912-925) and s11 (1520-1535): the walk driver has not
  started (wk = 0) and the module's rest bob coincides with the
  staged bob at phase 0, so they render identically.
- Control ranges (0-177, 392-783, 1211-1360, 1608-1810):
  **0 of 923 changed** — containment is exact.
- Standing probe ranges (195-204, 795-814, 1195-1204): **40/40
  changed** — the expected signature (D1: bob is module-staged on
  standing frames too; identity there would mean the wiring did
  not take).

## Census vs W57 expectation

W57's staging census expected 471 walking / 888 bob-differing /
448 swing frames. Pixel-measured: 858 frames differ. The 888
bob-differing staging frames map to 858 pixel-changed frames; the
30-frame delta is exactly the pre-driver rest frames above, where
the bob value at phase 0 renders identically. Walking/swing are
staging-level subsets of the same changed set and are not
separately pixel-countable; no changed frame falls outside the
bob footprint, which is the census's containment claim.

## PSNR sample (baseline vs adopted, RGB)

frame 200: 30.07 dB · 285: 26.67 · 350: 21.43 · 848: 49.04 ·
969: 50.02 · 1119: 34.71 · 1200: 42.57 · 1440: 34.86 ·
1593: 53.43 · 1600: 53.32 — min 21.43, mean 39.61, all finite
(gait-scale pose differences; no corruption signature).

## Determinism spot-check

Range 918-967 re-rendered at `56348ac` (run `43676113546`):
**50/50 frames byte-identical** to the full render.

## Gates at the wiring head

CI SUCCESS · Tools CI SUCCESS (189 tests) · film telemetry
`FILM TELEMETRY: CHECK OK`, exit 0 (CI binary of `56348ac`).
Golden impact: the six gait-segment mids re-pinned in `0d5f86b`
(act + disclosures in `docs/golden/NATIVE-GOLDEN.md`, including
the pre-existing blink-era staleness of the film-s02/film-s03
pins — s03 not re-pinned, not gait-changed).

## Contact sheets (local only — hashes)

Built per W57 §5.6 (pre left / post right, 320×180 thumbs):
`s02` 29 samples (5.907-13.061 s, step 0.25), `s06` 18
(26.129-30.390, 0.25), `s07` 16 (30.390-34.209, 0.25), `s08` 25
(34.209-40.360, 0.25), `s10` 11 (49.60-50.648, 0.10), `s11` 10
(50.648-51.60, 0.10).

- s02-ab-sheet.png `7b3b1b92b91131e4f14ddb73f2b2769cadd72c76999f85e448c8a2ed09a8b665`
- s06-ab-sheet.png `3e93900d36b259afddaf795a3efe4593a2dc797bf1b56c3b9fac2137c67e39db`
- s07-ab-sheet.png `bbf67ca453812c0b00c163ec3fdfacd6442a1c677bfb65ae095d991daab5dd28`
- s08-ab-sheet.png `db90ef68b06ed525d52a7163323dcd854424eb262c1e920255c0846eb3e55131`
- s10-ab-sheet.png `50258debd1e061e4233ea0e43857285ecc54b49e1bb6bf0c4c240d05a08888c9`
- s11-ab-sheet.png `1786464b797ddae9f81574990a123251ba7530b94b34c6eeecc469f7867bbdd5`
