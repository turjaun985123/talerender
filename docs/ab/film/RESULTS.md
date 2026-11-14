# RESULTS — whole-film A/B, first measured fill-in (W77)

**Recorded:** 2026-11-14T08:31Z · **Seat:** W77 (film A/B fill-in)
**Branch taken:** the frames EXISTED — the lead's full film
re-render at head `381ff47` was harvested from CI artifacts and
measured. This file records the CI state found, the artifact
inventory, the measured headline, and the disposition of every
`EXPECTED.md` hypothesis. The full 60-pair table is
`ab-report.md` / `ab-report.json` beside this file (tool output,
verbatim).

**Renderer pair (every number below):**
- Native: **talerender (CI lavapipe, head `381ff47`)** — the
  "Render (dispatch)" workflow, mode `film`, ubuntu-22.04 /
  Mesa lavapipe, the milestone-proof environment.
- Reference: **Chrome (SwiftShader) project review stills** —
  `~/faceless-shorts/the-soldier-who-didnt-know/review/frames/`,
  `t0.50.jpg` … `t59.50.jpg`, one q0.93 JPEG generation.

Per decision (B) and the package charter, these A/B numbers are
**reported, never gated as equality**: the references carry the
JPEG generation, so every figure understates native accuracy
(README.md, "The JPEG floor": detailed-content ceiling ≈ 42 dB).

## CI state found (2026-11-14, ~08:11–08:31Z, read-only `gh`)

Ten "Render (dispatch)" runs, **all at head `381ff47`, all
success**, created 2026-11-14T08:00:43Z–08:01:19Z — the chunked
full-film render the register commit announced ("FULL FILM
RE-RENDER follows from this commit"). Push CI for the same head
(run `43573712742`, workflow CI) also succeeded. (A Tools CI run
for this head, `43573712817`, is recorded as failure in the run
list; its cause was not investigated by this seat.)

| run id | mode / range | artifact id | artifact name | zip bytes |
|---|---|---|---|---|
| 43573740838 | film 0–200 | 14105427628 | render-film-0-200-43573740838 | 482,520,197 |
| 43573745992 | film 200–400 | 14105254886 | render-film-200-400-43573745992 | 438,125,212 |
| 43573751946 | film 400–600 | 14105771166 | render-film-400-600-43573751946 | 462,537,012 |
| 43573757052 | film 600–800 | 14105537032 | render-film-600-800-43573757052 | 465,391,955 |
| 43573764962 | film 800–1000 | 14105244456 | render-film-800-1000-43573764962 | 442,085,596 |
| 43573771631 | film 1000–1200 | 14105955706 | render-film-1000-1200-43573771631 | 471,841,325 |
| 43573777284 | film 1200–1400 | 14105174323 | render-film-1200-1400-43573777284 | 481,399,231 |
| 43573784026 | film 1400–1600 | 14105801859 | render-film-1400-1600-43573784026 | 464,895,760 |
| 43573789628 | film 1600–1811 | 14105970333 | render-film-1600-1811-43573789628 | 498,550,179 |
| 43573794678 | film 1195–1205 (probe) | 14104984297 | render-film-1195-1205-43573794678 | 24,078,871 |

Artifacts carry the workflow's 7-day retention (uploaded
2026-11-14 ~08:10Z → expire ~2026-11-21).

## Artifact inventory (zip central directories, read first-hand)

All entries are named `frames/frame_%05d.png` (deflate). The
nine chunk artifacts tile **[0, 1811) exactly, no gaps**:

| range | entries | first | last | uncompressed bytes |
|---|---|---|---|---|
| 0–200 | 200 | frame_00000.png | frame_00199.png | 592,006,604 |
| 200–400 | 200 | frame_00200.png | frame_00399.png | 565,312,103 |
| 400–600 | 200 | frame_00400.png | frame_00599.png | 578,067,483 |
| 600–800 | 200 | frame_00600.png | frame_00799.png | 580,473,561 |
| 800–1000 | 200 | frame_00800.png | frame_00999.png | 562,913,555 |
| 1000–1200 | 200 | frame_01000.png | frame_01199.png | 582,111,556 |
| 1200–1400 | 200 | frame_01200.png | frame_01399.png | 590,998,804 |
| 1400–1600 | 200 | frame_01400.png | frame_01599.png | 576,515,545 |
| 1600–1811 | 211 | frame_01600.png | frame_01810.png | 605,550,305 |
| **total** | **1,811** | | | **5,233,949,516** |

Probe artifact: 10 entries, `frame_01195.png` … `frame_01204.png`
— an overlap render straddling the 1200 chunk boundary (see the
consistency check below).

**Harvest method.** This VM's `/tmp` is a 512 MB tmpfs, so the
frames were harvested into `~/workspace/.w77-scratch/` (home
volume, outside the repo) by HTTP **range requests** against the
artifact zips: only each zip's central directory plus the 82
needed frame entries (60 pair frames + 12 further golden mids +
10 overlap frames) were fetched, not the 4.2 GB of zips. The
extractor was validated before use: range-extracted frames
1195/1204 are byte-identical (sha256) to a full `gh run
download` of the probe artifact. The scratch tree (`frames/`,
`probe/`, `measure-out/`) is left in place for the lead and can
be deleted once the golden staging is done or the artifacts
expire.

## Headline measurement

`tools/ab/measure_ab.py` film mode (exit 0 = measured):

```
pairs measured: 60; missing: 0; overall PSNR min 18.378 / mean 21.836
```

Per-shot summary (verbatim from `ab-report.md`; the full
60-pair table and worst-3 images are there):

| segment | set | atmosphere | pairs | PSNR min (dB) | PSNR mean (dB) | mean \|d\| | %px > 10 (mean) |
|---|---|---|---|---|---|---|---|
| s01 | JG | noon | 6 | 24.094 | 25.005 | 4.86742 | 12.21615 |
| s02 | SH | noon | 7 | 21.034 | 22.623 | 5.04955 | 10.0241 |
| s03 | SH | noon | 3 | 20.492 | 20.776 | 9.54173 | 36.01273 |
| s04 | JG | noon | 5 | 22.558 | 23.0 | 6.46579 | 25.84367 |
| s05a | JG | noon | 4 | 20.588 | 21.466 | 11.79834 | 62.83133 |
| s05b | JG | noon | 1 | 19.363 | 19.363 | 9.15144 | 21.23563 |
| s06 | JG | fog | 4 | 23.878 | 24.77 | 4.83212 | 17.93833 |
| s07 | JG | golden | 4 | 21.424 | 22.191 | 6.86003 | 36.1841 |
| s08 | JG | noon | 6 | 19.332 | 20.347 | 17.5514 | 87.2398 |
| s08b | JG | noon | 2 | 18.952 | 19.401 | 19.39113 | 82.95816 |
| s09 | JG | noon | 3 | 18.902 | 19.302 | 19.66499 | 82.57612 |
| s10 | BK | noon | 6 | 21.238 | 21.816 | 7.66262 | 23.16945 |
| s11 | JG | dusk | 5 | 18.378 | 18.523 | 14.62819 | 61.1964 |
| s12 | JG | dusk | 4 | 19.695 | 22.275 | 9.78313 | 57.61859 |

Worst-3 pairs — all s11: t 52.50 (18.378 dB), t 53.50
(18.402 dB), t 55.50 (18.574 dB); side-by-side + |Δ|×4 images
in this directory.

## Disposition of the EXPECTED.md hypotheses

- **s01 anchor — HOLDS.** The milestone pair t = 1.50 (frame
  45) measures **24.094 dB** in film mode, against the M5
  full-chain milestone's 23.850 dB for the same staged instant
  (§28 20.152 → M4 20.218 → M5 23.850). The film path reproduces
  the milestone chain's agreement level; the segment is the
  film's best (mean 25.005 dB, %px > 10 ≈ 12.2%).
- **s06 fog — the declared residual risk does NOT materialize.**
  Mean 24.77 dB (second-best segment), mean |Δ| 4.83, and the
  four pairs are flat-to-improving (23.878 → 25.343 → 24.817 →
  25.044) with no depth-growing broad-delta signature. The film
  path's smoothstep fog agrees with Chrome on this evidence.
- **s08 + s08b + s09 — a broad-delta cluster (partly a NEW
  FINDING).** Three consecutive JG/noon segments (t 34.50–44.50)
  share a signature no other noon stretch has: mean |Δ|
  17.55 / 19.39 / 19.66, %px > 10 87.2 / 83.0 / 82.6%, %px > 0
  ≈ 99.5% — a whole-frame-scale delta, not a localised error.
  EXPECTED pre-registered the fire channel (R1 basis 4.6) as
  s08's expected delta centre, and all three segments are the
  fire-camp stretch of the film; the observed pattern is
  consistent in kind with a fire-light contribution but **much
  broader than "flame and fire-lit regions"** as predicted.
  For **s09** EXPECTED registered *no* hypothesis and said a low
  number would be a new finding — it is the film's highest
  mean |Δ| (19.66) and is recorded here as one. The cluster is
  referred to the lead as-is; this package reports, and the
  telemetry gate plus visual diff inspection are the diagnosis
  path. (No cluster pair is in the worst-3: PSNR ranks by
  largest errors, and the cluster's delta is broad-but-moderate
  where s11's is concentrated.)
- **s11 — lowest segment, NOT explained away.** Mean
  18.523 dB, min 18.378 dB, all three worst pairs. Its named
  risks were dusk scene.fog and the D14 lean / D15 arms items —
  but D14/D15 were closed in `381ff47`, the very commit these
  frames were rendered from, so the register fixes are *in* the
  measured frames and the gap persists: %px > 10 ≈ 61% with
  mean |Δ| 14.63 again indicates a broad component (dusk
  fog/grade) on top of any local delta. The worst-3
  side-by-sides and diffs in this directory are the inspection
  material.
- **s05a — contrary to expectation.** EXPECTED placed this
  sky/leaflet segment near the flat-content ceiling; measured
  mean is 21.466 dB with %px > 10 = 62.8% and mean |Δ| 11.80 —
  a broad delta on supposedly flat content. Recorded as an
  observation for the lead; no mechanism asserted from here.
- **s10 — post-fix behaviour confirmed.** Mean 21.816 dB,
  %px > 10 ≈ 23.2%, trending down pair-over-pair (25.9 → ≈21).
  None of the pre-fix shade signature (broad drift, stubborn
  %px > 10) EXPECTED described for a pre-`c00897d` binary; the
  rendering head `381ff47` descends from `c00897d`, and the
  numbers are consistent with that.
- **s07 calendar — as predicted in kind.** %px > 10 is highest
  on the 1945-steady and first-flip pairs (46.7% at t 30.50,
  42.4% at t 31.50) and eases to 26.9% by t 33.50 — the card
  region dominating %px > 10 is the pre-registered expectation.
- **s02 — trend noted, no contradiction.** %px > 10 climbs
  monotonically across the segment (6.5% → 14.3%) while PSNR
  holds 21.0–24.2 dB. D6 (march clamp) was closed in the
  rendering commit itself; the residual rise is recorded, not
  attributed.
- **s12 — read mean-|Δ|-first, as EXPECTED instructed:** 9.78
  segment mean, improving monotonically across the segment
  (12.76 → 10.26 → 8.28 → 7.84); PSNR mean 22.275 dB. The
  dark-outro concern does not materialize.
- **s03 / s04 / s05b — reported without incident.** s04
  (23.0 dB mean) is consistent with its exact-f32 recomputed
  states being right; s03 (20.776 dB, %px > 10 36.0%) carries
  the rifle as predicted to need watching but produces no
  worst-3 pair; s05b's single pair (19.363 dB) characterizes
  the segment neither way, per EXPECTED's own warning.

## Chunk-overlap consistency check (probe vs chunks)

The probe render `[1195, 1205)` (run `43573794678`) overlaps
two chunk artifacts, giving the package its only
same-head, cross-invocation comparison:

- Frames **1200–1204** (chunk [1200,1400) vs probe):
  **byte-identical** (sha256 match, pair-mode PSNR = ∞).
- Frames **1195–1199** (chunk [1000,1200) vs probe): differ,
  but at **PSNR 72.34 dB** (1195 / 1197 / 1199 measured):
  1,787 of 2,073,600 px (0.086%), 86% of them at |Δ| = 1,
  max |Δ| 28 (4 px ≥ 17), clustered at x 1–597, y 467–1270
  (the lower-left foliage region of the s08 frame).

Leading hypothesis (evidence, not proof): a paint re-rendered
after FIFO paint-cache eviction is not bit-identical to its
first render — the chunk invocation had rendered 195 frames
when it reached 1195, the probe 0–4, and the cache cap is 160
entries (commit `1475b66`, which records the *assumption* that
eviction is pixel-neutral). The effect is ~30 dB below the
references' own JPEG noise and immaterial to every number in
this package: **no review pair lands in 1195–1199**, and no
pair frame sits at a chunk-tail position with a measured
deviation. It is recorded because the golden staging consumes
chunk frames: four golden mids sit at chunk-invocation offsets
≥ 160 — frames **560 (offset 160), 760 (160), 969 (169),
1593 (193)** — and may carry the same ±1-class deviation
relative to a fresh render. Pinning is a human act; weigh this
before pinning those four from these artifacts.

## Golden mid frames (staged, NOT pinned)

The 14 segment mid frames were harvested with the pairs. Their
sha256 (native, CI lavapipe, head `381ff47`) — staging via
`tools/golden/run_golden.sh` and any pinning remain the
lead/human step; this seat staged hashes only:

| seg | mid frame | sha256 |
|---|---|---|
| s01 | 89 | ebcfeba172be55b8ec8a8a56d2b50bef9a8641543d645fa8fd50106729cbc295 |
| s02 | 285 | 9be3879c94db211aaf0d2668ad874dd26e9c804aab7217f0a520334f6fa1b73c |
| s03 | 435 | 61c54a42dbde95c814e80bb0416a63f95503869ca712c2ba9b691db407495857 |
| s04 | 560 | 61e2ff32da99814610b695fdccdcdec1d2b1338925e3a594e114c1aaaa85af2f |
| s05a | 689 | bcb161f11cf418814c83e3875c1fb9c5732ed75e0f7c660c88a332c4e12f2bde |
| s05b | 760 | e932ed856dbac45f299211456dbe4b8c259a1190c56cf84e13099a2a8a1b215c |
| s06 | 848 | d41bd58e3918fe7018d45ce32e1f8ee3c90b749db921a5ea7aba5f3fe6777289 |
| s07 | 969 | 9eca38032479cfdba282358d67cb313b68781f68e65edbfff5a45b21fe6122a1 |
| s08 | 1119 | 7ab95e9d4c94fb0b09e1450214f14649ce551f2eea2dfe8316ba1326f398746a |
| s08b | 1231 | 7764821c30e1094e27626770857e048528fbf4fcc3f8a3c5a512a18379426f5a |
| s09 | 1306 | ebe45d9c7f322e22a858b9fe11e057bed848afa7d23690a3d578ee95f40290a3 |
| s10 | 1440 | 59bdb4f2558fa37f34569482f1c650669b96eb8a8fb225c8d2825386b93ebaad |
| s11 | 1593 | eb7b23f4a4be2834b842e30924fe29d2928df5022688f3a66fa05fd9a2a84f30 |
| s12 | 1738 | 1978d6e603bcfc83e0764de509116a1e448332671fa6342d57b640c04a91bfe7 |

## Still pending for this package

- **FILM TELEMETRY verdict line** for head `381ff47` — not run
  by this seat (CPU-side mode; runs locally or in CI). The
  README's gate-first ordering is noted: the A/B above measures
  the frames that exist; the gate verdict is the lead's to
  record alongside.
- **Golden staging + verify output** for the 14 mids (hashes
  above; pinning is a human act, with the offset caveat).
- **MP4 finish + `MP4-EVIDENCE.md`** — no finished MP4 exists
  yet; the template's procedure stands (`film/finish.sh` over
  the 1,811 frames + the project's `data/mix.wav`; counted
  frames must be 1811; mux loudness must match the mix
  ≈ −14.0 LUFS).
- **PC RADV evidence** — none here; all native numbers are CI
  lavapipe and are labelled as such. Grain is vendor-sensitive;
  a RADV run is separate evidence, not a replacement.
