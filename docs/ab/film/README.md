# docs/ab/film — the whole-film A/B proof package (SKELETON)

**Status: SKELETON — no native film frames exist yet.** This package
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
   # verdict is the printed line: FILM TELEMETRY: CHECK OK
   # (this mode exits 0 on pass, fail, and error alike)
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

## Results (to be filled by the procedure above)

- Telemetry verdict: _pending_
- Renderer of record for the native frames: _pending_
- Pairs measured: _pending_ / 60 (missing listed in ab-report.md)
- Per-shot summary: _paste from ab-report.md_
- Worst-3 pairs + their risk-note disposition: _pending_
- Golden verify (14 film mids): _pending_
- MP4 evidence: see `MP4-EVIDENCE.md` — _pending_
