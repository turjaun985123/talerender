# docs/ab/film — the whole-film A/B proof package (SKELETON)

**Status: MEASURED — film A/B filled in 2026-11-14 (W77);
telemetry verdict, golden staging, and MP4 evidence have
since landed, and the golden pins have since been struck
(see the Results section; the package's final film state is
the `f021991` re-render, A/B mean 23.02 dB).** The full film was rendered natively in CI at head
`381ff47` (ten dispatch runs, 1,811 frames) and measured:
`ab-report.md` / `ab-report.json` + `RESULTS.md` in this
directory. This package
is the acceptance evidence for TaleRender production integration:
the complete Onoda film ("THE SOLDIER WHO DIDN'T KNOW THE WAR WAS
OVER", 14 segments, 1,811 frames at 30 fps, narration 60.357 s)
rendered natively and measured against the Chrome original, shot by
shot, plus the finished MP4's technical evidence. The structure is
built so that once a film render exists, completing the package is
mechanical: render → measure → paste → finish → record. Nothing in
this directory is a result until the fill-in procedure below has
been run and its outputs committed beside this file.

## What the completed package contains

- `ab-report.json` + `ab-report.md` — the measure_ab film-mode
  output: per-shot summary (PSNR / mean |Δ| / %px > 10), the full
  60-pair table, and the explicit missing/no-data list.
- `worst-<k>_<seg>_t<T>_side-by-side.png` + `..._diff.png` — for
  the three lowest-PSNR pairs: labelled side-by-sides (reference
  left, native right) and |Δ|×4 diffs, produced by the tool.
- `EXPECTED.md` — the expectation table (this package, written in
  advance): segments, frame ranges, which review pairs land where,
  golden mid-frame indices, and each segment's known risk notes,
  so a low number arrives next to its hypothesis.
- `MP4-EVIDENCE.md` — the finished film's technical record
  (streams, frame count, duration, colour tags, loudness),
  filled from the template in that file.
- The native golden staging for the 14 segment mid frames
  (`tools/golden/run_golden.sh` + `make_golden.py` verify output),
  recorded in the results section below.

## Metric definitions (house set)

The same definitions as every M2–M6 package and
`docs/ab/scene-format/scripts/ab_metrics.py`, implemented by
`tools/ab/measure_ab.py` (which reproduces all three committed
milestone packages digit-for-digit):

| metric | definition |
|---|---|
| PSNR (dB) | `10·log10(255² / MSE)`, MSE over all channels, float64 |
| mean \|Δ\| | mean absolute per-channel delta |
| %px > 10 | % of pixels whose **max-channel** \|Δ\| > 10 |
| %px > 0, max Δ | recorded alongside, for the full picture |

PSNR is the headline; %px > 10 is the legible "how much of the
frame visibly differs" measure. Numbers are comparable only
within a content class (detailed foliage ≠ flat chart) and only
against references carrying the same JPEG generation.

## The JPEG floor — read this before any number

The references are the project's Chrome review stills: JPEGs,
**one q0.93 generation** from the Chrome canvas (Chromium
`toDataURL('image/jpeg', 0.93)`). The M6 decision brief
(`docs/ab/telemetry/jpeg-decision.md`) measured what that
generation costs, through the pipeline's own encoder:

| comparison | PSNR | mean \|Δ\| | px changed | max Δ |
|---|---|---|---|---|
| clean → generation 1 (detailed frame) | **42.31 dB** | 1.225 | 94.47 % | 48 |
| flat/step content floor (dof-chart panels) | **50.78–57.71 dB** | — | — | — |
| generation 1 → generation 2 | 56.10 dB | 0.082 | 8.80 % | 14 |

Consequences, also stated in every report the tool writes:

- A *pixel-perfect* native frame cannot score above ≈ 42 dB on
  detailed content (≈ 51–58 dB on flat content) against these
  references. High-30s/low-40s on a detailed frame is near the
  reference's own noise floor.
- The bias is **one-directional**: JPEG error sits in the
  reference but is counted as native error. These numbers can
  only *understate* native accuracy, never overstate it.
- Per the user's decision (B), native frames stay **clean** —
  no q0.93 generation is applied to them to chase the floor.

Calibration (same metric set): the lossy headless-gl port scored
**15.8–19.3 dB** on its film A/B — that band is "visibly a
different renderer". TaleRender on this film's S01 frame
(t = 1.50): **20.152 dB** (§28 flat prototype) → **20.218 dB**
(M4 painter) → **23.850 dB** (M5 full composer chain, %px > 10
13.5 %). The dof-chart reached 47.38 dB full-frame with its flat
zones at the 50.78–57.71 dB floor.

## Renderer labelling law

Every number in the completed package names the renderer that
produced each side:

- **References:** Chrome (SwiftShader) stills from the project's
  review set — `review/frames/t<sec>.jpg`, one per second.
- **Native frames:** labelled **CI lavapipe** (GitHub Actions
  render workflow) or **PC RADV** (the user's Ryzen 5 PRO 5650G /
  Vega 7 machine) via `--native-label` / `--reference-label` and
  the golden runner's mandatory `--renderer`. The GRADE grain
  hash is GPU-vendor sensitive (M5), so lavapipe and RADV frames
  are both "native" but are **not** interchangeable evidence;
  a frames directory of unknown provenance is not measurable
  evidence for this package.

These numbers claim nothing about staging correctness (that is
the M6 telemetry gate's job — `cutouts()` agreement), nothing
about motion (pairs are single staged instants), and nothing
about performance (lavapipe timing is meaningless; RADV timing
lives in `docs/runbook/PERF.md`).

## Fill-in procedure (exact commands)

Prerequisites: a current binary (CI `talerender-binary` artifact
or a user-PC build of HEAD), the film data in this repo, and the
project at `~/faceless-shorts/the-soldier-who-didnt-know`.

1. **Telemetry gate first** — the film must pass before its
   frames mean anything:

   ```
   cd ~/workspace/talerender
   TALERENDER_PAINT_CACHE=24 ./talerender --film-telemetry film
   # exit 0 = CHECK OK, 1 = CHECK FAILED, 3 = could not run;
   # the printed verdict line remains authoritative
   ```

2. **Render the film** (chunked/resumable; on CI dispatch the
   "Render (dispatch)" workflow, mode `film`, instead):

   ```
   ./talerender --film film --out /tmp/onoda-frames
   # writes frame_00000.png … frame_01810.png (1,811 frames)
   ```

3. **Stage the golden mid frames** (never pins — pinning is a
   human act after reviewing this package):

   ```
   bash tools/golden/run_golden.sh /tmp/onoda-frames \
     --renderer "CI lavapipe"        # or "PC RADV"
   python3 tools/golden/make_golden.py   # verify; record output
   ```

4. **Measure** (60 pairs; exit 0 = pairs measured, exit 2 = no
   data — a NO DATA report is a failed proof, not a pass):

   ```
   python3 tools/ab/measure_ab.py \
     --native-dir /tmp/onoda-frames \
     --review-dir ~/faceless-shorts/the-soldier-who-didnt-know/review/frames \
     --film scenes/film/film.json \
     --native-label "talerender (CI lavapipe)" \
     --out docs/ab/film
   ```

5. **Read the results against `EXPECTED.md`** — per segment,
   check the measured pairs against the segment's risk notes
   before concluding anything; copy the per-shot summary table
   into the results section below, verbatim.

6. **Finish the MP4 and record its evidence** per
   `MP4-EVIDENCE.md`:

   ```
   film/finish.sh /tmp/onoda-frames \
     ~/faceless-shorts/the-soldier-who-didnt-know/data/mix.wav \
     /tmp/onoda-native.mp4
   ```

## Results (filled 2026-11-14, W77 — see `RESULTS.md`)

- Telemetry verdict: recorded in `TELEMETRY-W85.md` for head
  `381ff47` (CHECK FAILED on captions only; waiver ruled in
  `9790553`), then CLOSED BY DATA in Wave 2 — after the
  66 → 73 chunk re-chunk (`3a36f1f`) and driver regeneration,
  the re-run prints `FILM TELEMETRY: CHECK OK`, exit 0
  (`5e90d7e`)
- Renderer of record for the native frames: **talerender
  (CI lavapipe, head `381ff47`)** — ten "Render (dispatch)"
  runs, mode `film`; references are the Chrome (SwiftShader)
  project review stills. Run/artifact record: `RESULTS.md`.
- Pairs measured: **60 / 60** (missing: none — `ab-report.md`)
- Per-shot summary (verbatim from `ab-report.md`):

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

  Overall: PSNR min 18.378 / mean 21.836 dB (reported per
  decision (B), never gated — see `RESULTS.md` for the
  per-hypothesis dispositions: anchor holds, s06 fog risk
  refuted, s08–s09 broad-delta cluster and s11 open).
- Worst-3 pairs + their risk-note disposition: all **s11**
  (t 52.50 / 53.50 / 55.50; 18.378–18.574 dB) — images in this
  directory. Disposition: **open** — the segment's register
  items (D14/D15) closed in the rendering commit itself, yet
  the gap persists with a broad (%px > 10 ≈ 61%) component;
  referred to the lead with the diff images (`RESULTS.md`).
- Golden verify (14 film mids): staging DONE and pins
  STRUCK — mids staged hash-verified in `docs/golden/frames/`
  (`381ff47` set), re-staged from the `f021991` frames in
  `docs/golden/frames-f021991/` (W03, `3a36f1f`), and pinned
  in Golden Act 1 (`916a109`; the 3 in-window mids — frames
  969 / 1119 / 1593 — come from the post-re-chunk render at
  `aadc193`, so their struck hashes differ from the staged
  table in `RESULTS.md`). Pack verifies 18 OK / 0 STALE /
  0 MISSING.
- MP4 evidence: see `MP4-EVIDENCE.md` — DONE (1,811 frames
  counted; mux −14.00 LUFS vs mix −13.99; the final proof
  MP4 from the `f021991` frames carries all-bt709 colour
  tags after the finish.sh fix `6ca97fc`)
