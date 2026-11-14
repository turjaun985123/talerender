# BENCH-W17 — the PERF bench harness (Phase 3 measurement)

**Seat:** Wave 2 W17. **Delivered (new files only, uncommitted):**
`.github/workflows/bench.yml`, `tools/perf/bench_report.py`, this
note. **Not touched:** `src/`, `docs/runbook/PERF.md` (the protocol
there is the spec implemented against), `render.yml`.

PERF.md's tables are blank by design: every number so far is CI
lavapipe (correctness evidence), and the real numbers must come
from the user's RADV PC. This seat builds the harness that makes
both future runs mechanical — a CI dispatch that times a bounded
film chunk on lavapipe, and a report generator that turns a log
directory (CI artifact *or* a PC `perf/` directory) into
PERF-shaped tables.

---

## 1. `bench.yml` — the CI dispatch

Dispatch-only, no inputs (a fixed protocol; inputs would invite
unbounded ranges). Environment mirrors `render.yml` exactly
(ubuntu-22.04, mold via `RUSTFLAGS`, `CARGO_INCREMENTAL=0`,
rust-cache with cache-on-failure, lavapipe ICD pinned via
`VK_ICD_FILENAMES`); the one addition is the `time` package for
`/usr/bin/time -v`.

**Legs** — the two paint-cache configurations of PERF §4, each
rendering the same bounded chunk into a *fresh* output directory
(the driver resumes by skipping existing frames; a re-run over
existing frames measures nothing):

| leg | command shape | wrap |
|---|---|---|
| A `bench-default` | `--film film --out bench-out/frames-default --from 0 --to 100` | `/usr/bin/time -v` → `bench-default.time`, exit code → `bench-default.exit` |
| B `bench-cache24` | same, `TALERENDER_PAINT_CACHE=24`, fresh `frames-cache24` | same, `bench-cache24.*` |

Plus an environment capture (`bench-out/env.txt`: date, kernel,
RAM, ICD list, binary sha256 + `--version`) and per-frame PNG
sizes (`frames-*.sizes`, `stat -c '%s %n'`) for the report's §5
table.

**The bound — frames 0..100, justified from render.yml's known
timings.** The Wave-1 full-film proof rendered all 1,811 frames
as 10 render.yml dispatch jobs tiling [0, 1811) — ≈181 frames
per job — every job inside this workflow family's 120-minute
limit on this same lavapipe image. 100 frames is ~55% of that
proven single-job chunk; two legs (200 frames) plus the release
build still fit one job. 100 is also an exact multiple of the
driver's 25-frame progress cadence → exactly 4 progress
boundaries per leg, so the report gets real per-range rows.
Starting at 0 is deliberate and differs from PERF §3's mid-film
chunk (1027..1117): the CI bench measures *startup-inclusive*
cost (process start, asset load, S01 and the S01→S02 segment
transition); the PC protocol measures steady state. The report
labels which is which; the two are never merged (and the
renderer-refusal below makes accidental merging impossible
across machines).

**Honesty rule, implemented twice.** The job summary
(`$GITHUB_STEP_SUMMARY`) opens with: *"⚠ SOFTWARE RENDERER —
INDICATIVE ONLY. This job ran on lavapipe (llvmpipe, CPU). These
timings are not hardware performance and must never be quoted
as such (PERF.md honesty rule)."* — followed by a per-leg table
(renderer line, completion line, wall, peak RSS, exit) grepped
from the raw logs. The report generator prints the same banner
whenever a log set names a software device (§2).

**Artifacts.** One artifact `bench-<run_id>` with the raw logs
only: `*.log`, `*.time`, `*.exit`, `env.txt`, `*.sizes`
(retention 7 days, mirroring render.yml). The 100 PNG masters
per leg are **not** uploaded (~100–300 MB of masters, not logs);
their sizes are. A final verdict step fails the job if either
leg exited non-zero — *after* the upload, so a failed bench is
still a downloadable result (PERF §7 spirit).

*Verification status:* the YAML parses (checked with a YAML
parser) and mirrors render.yml's proven steps; there is no
workflow runner on this VM, so the first real dispatch is the
workflow's verification. Nothing in this seat claims a bench
run has happened.

## 2. `bench_report.py` — logs → PERF-shaped tables

```
python3 tools/perf/bench_report.py <log-dir> [--out report.md]
```

Stdlib only. Accepts either layout, flat directory of logs:

- **CI artifact:** `env.txt`, `bench-default.*`, `bench-cache24.*`,
  `frames-*.sizes`.
- **PC `perf/` dir** (PERF.md §1–§6 file names): `ab1-<n>.*`,
  `s01-<n>.*`, `chunk-cold/warm`, `chunk-s06-*`,
  `telemetry-default/24.*`, `chunk-warm/frame_*.png`, optional
  `chrome.time`.

Protocol laws implemented:

- **Every table names its renderer.** Each table block is headed
  by the `renderer:`/`adapter:` line taken *verbatim* from the
  logs. A directory whose logs carry no renderer line at all is
  **refused** (exit 2) — on the PC, PERF §1 already has the
  operator copy the adapter line into `perf/env.txt`, which the
  parser also reads.
- **Mixed renderers are refused, loudly.** If the distinct
  renderer strings (prefix-normalised, so a PC set's `adapter:`
  scene lines and `renderer:` film lines of the *same* device
  unify) number more than one, exit 2 naming each. Lavapipe and
  RADV numbers can never share a table by accident.
- **Software banner.** llvmpipe/lavapipe/`device type: Cpu` →
  the honesty banner at the head of the report.
- **Invalid timings flagged, not hidden.** A completion line
  with skipped frames or a rendered count short of its range
  produces a `⚠ INVALID timing per PERF §3` warning; a non-zero
  `.exit` is likewise surfaced. Numbers are shown, marked
  unquotable — per PERF §7, a failure is a result.
- **Extrapolation discipline (§3).** The 1,811-frame figure is
  computed *only* for runs named `*warm*` — the only arithmetic
  PERF sanctions. CI legs get `—`.
- **Total-time provenance.** Totals prefer the caller's
  `/usr/bin/time` (both forms parse: `-v` and `-f "%e %M"`).
  The binary's own post-W09 completion signals
  (`elapsed MM:SS · peak RSS <MiB> MB`) are the fallback and are
  labelled render-loop-only / self-report; runlog's MiB are
  converted to the tables' KiB (×1024, per RUNLOG-W09 §3).
- **Both progress generations parse.** Pre-W09 lines (no
  elapsed) produce an explicit "per-range times unavailable —
  not invented" note; post-W09 lines (`· elapsed MM:SS ·
  eta MM:SS`, total-minutes clocks per RUNLOG-W09) produce the
  per-range table, including the tail interval up to the
  completion line. The W09 bare form (`frame 1051/1811 (58.0%)
  elapsed 03:12`) is also recognised.
- **Telemetry disagreement = defect (§4).** If cap-160 and
  cap-24 verdict lines differ, the §4 table carries the
  determinism-defect warning PERF prescribes.
- **§6 caveat travels with the table.** The native/Chrome
  comparison always prints PERF §6's artifact-difference note
  (clean PNG vs browser-pipeline q0.93 JPEG generation;
  finishing excluded). The Chrome row fills from `chrome.time`
  (GNU time `%e %M`; frame count from `chrome.frames` if it
  differs from the native row's) and otherwise prints the
  instruction instead of a blank.

## 3. Parser tests — synthetic logs (fabricated numbers!)

No bench run exists yet, so the parser was tested on fabricated
log sets built to the real formats (PERF §2–§4 file shapes,
RUNLOG-W09 line shapes). **Every number below is synthetic.**

### Set A — CI-style lavapipe (post-W09 lines, `time -v`)

§3 table + first per-range table, verbatim from the output:

```
| chunk | elapsed s | peak RSS KiB | s/frame | frames/hour | extrapolated 1,811 frames |
|---|---|---|---|---|---|
| film 0..100 (CI bench, cap 160) | 108.20 | 921600 | 1.082 | 3,327 | — |
| film 0..100 (CI bench, cap 24) | 119.00 | 512000 | 1.190 | 3,025 | — |

| range (frames) | rendered in range | elapsed s | s/frame |
|---|---|---|---|
| 0..24 | 25 | 26.00 | 1.040 |
| 25..49 | 25 | 25.00 | 1.000 |
| 50..74 | 25 | 25.00 | 1.000 |
| 75..99 | 25 | 26.00 | 1.040 |
```

Checks: header carries the llvmpipe line + software banner;
totals come from `time -v` (1:48.20 → 108.20, RSS 921,600 KiB —
not the completion line's render-loop 102 s / 812.4 MB);
per-range rows split on the progress elapsed deltas; no
extrapolation for CI legs; §4 falls back to the two film legs;
§5 computed from both `.sizes` sets (mean ≈2.3 MB → ≈4.1 GB
full-film @ mean); §6 native row filled, Chrome row prints the
`chrome.time` instruction.

### Set B — PC-style RADV (PERF layout; warm chunk pre-W09)

§2 and §6 rows, verbatim:

```
| still | run 1 (cold) s | runs 2–5 s (all four) | warm median s | peak RSS (warm runs) KiB |
|---|---|---|---|---|
| ab1 (`--scene ab1`) | 3.40 | 2.90, 2.85, 2.95, 2.88 | 2.89 | 406000 |
| S01 (`--scene-file scenes/film/s01.scene.json`) | 1.95 | 1.70, 1.68, 1.72, 1.69 | 1.69 | 378200 |

| route | frames | wall time s | s/frame | frames/hour | output artifact |
|---|---|---|---|---|---|
| TaleRender native (`--film`, chunk 1027..1117 — warm) | 90 | 24.60 | 0.273 | 13,171 | clean PNG |
| Chrome patched capture (same window) | 90 | 13.50 | 0.150 | 24,000 | browser-pipeline frames |
```

Checks: RADV label on every table, **no** software banner;
warm median = PERF's definition (mean of middle two);
warm-chunk extrapolation appears (0.14 h) while the cold row's
stays `—`; the cold chunk's post-W09 per-range table includes
the 15-frame tail interval (1102..1116) from the completion
elapsed; the warm chunk's pre-W09 lines produce the
"unavailable, not invented" note; §4 shows both telemetry
configurations with the verbatim verdict line + row C; §5 read
from the actual `chunk-warm/frame_*.png` files.

### Refusals and the invalid-timing flag, verbatim

Mixed set (one lavapipe log + one RADV log) — exit 2:

```
bench_report: ERROR: logs from DIFFERENT renderers in one directory — refusing to merge them into one table (PERF.md: a table's renderer must be unambiguous):
  - renderer: llvmpipe (LLVM 15.0.7, 256 bits) | backend: Vulkan | device type: Cpu | driver: llvmpipe 0.0.1
  - renderer: AMD Radeon Vega 7 (RADV RENOIR) | backend: Vulkan | device type: IntegratedGpu | driver: radv 23.2.1
Split the directory by renderer and report each set separately.
```

No-renderer set — exit 2:

```
bench_report: ERROR: no `renderer:`/`adapter:` line found in any log or env.txt — PERF.md requires every table to name its renderer, so this directory cannot be reported. (On the PC, copy the adapter line into perf/env.txt per PERF §1.)
```

Short render (completion line mutated to `90 rendered,
10 skipped` over `0..100`) — warning section:

```
- `bench-default` completion line reports 90 rendered, 10 skipped over range 0..100 — **INVALID timing** per PERF §3 (fresh-directory rule); the numbers are shown but must not be quoted.
```

## 4. How the user's PC run uses this

The PC side changes almost nothing — PERF.md remains the
protocol; the report generator just replaces the hand-filled
tables:

```bash
# 1. Run PERF.md §1–§5 exactly as written (produces perf/).
#    §1 already saves the adapter line into perf/env.txt —
#    the parser needs it (or any log's renderer line) to label
#    the tables.

# 2. Optional, for the §6 row: time ONLY the Chrome capture
#    step (patched capture, same t = 34.23…37.20 s window) —
#    e.g. wrap the capture command:
#      /usr/bin/time -f "%e %M" -o perf/chrome.time <capture command>
#    (add the frame count as perf/chrome.frames if it is not 90)

# 3. Generate the filled tables:
python3 tools/perf/bench_report.py perf --out perf/REPORT.md

# 4. Return package per PERF: env.txt, REPORT.md (the §2–§6
#    tables + §8's raw inputs), and the full logs (perf/*.log,
#    perf/*.time). §8's summary prose stays a human fill-in.
```

The CI route, for harness checks only (never for numbers):
dispatch **Bench** in GitHub Actions → download the `bench-*`
artifact → unzip → `python3 tools/perf/bench_report.py
<unzipped-dir>`.

## 5. What remains manual / pending

- **The PC execution itself — pending**, exactly as PERF.md
  states. No RADV number exists yet; nothing in this seat's
  files is a measurement. The harness removes the table-filling
  and labelling labour, not the run.
- **The Chrome-side capture timing** (§6) is a step in the
  skill's Onoda project, outside this repo; it stays manual
  (one `time` wrap, above).
- **PERF §1's environment capture** stays as the protocol
  writes it (the CI workflow does its own equivalent).
- **PERF §8's summary** is prose over the generated tables;
  the generator deliberately does not fabricate it.
- **First bench.yml dispatch** is the workflow's own
  verification (no runner on this VM; YAML parse-checked only).
