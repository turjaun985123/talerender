# TaleRender V2 — Performance laboratory (WP6)

Scope law (master prompt §10): build the tooling, **do not run
the hardware benchmark campaign in this wave, do not fabricate
or prepopulate results**. The existing PERF tables stay blank
until genuine measurements exist. This page documents the
tooling that exists, the record schema, and the labelling law.

## 1. What exists

| Tool | State |
|---|---|
| `tools/perf/lab/bench_lab.py` (986 lines) | **Exists** (Batch 1 Seat 6). Single-run, machine-readable bench record: runner + record re-renderer + synthetic self-test. |
| `tools/perf/lab/report.py` (670 lines) | **Landing in this wave** (Batch 2 Seat 9, WP6 reporting engine; untracked seat file). Consumes `talerender-bench/1` records (§3) and emits one Markdown comparison report (stdout or `--out`): runs table, hash agreement across records, timing/RSS comparison. It takes each record's `adapter_class` **verbatim** — it never re-derives or upgrades a class from the renderer line — and renders missing/null fields as "—" (never invented, interpolated, or carried over from another run). It does not fill `docs/runbook/PERF.md`'s tables. Its detailed output shape is defined in its own header; this seat read the header and schema contract, and did not audit the full body — **not verified** beyond that. |
| `tools/perf/bench_report.py` | Pre-existing. Owns the PERF-shaped multi-run tables; not duplicated by `bench_lab.py`. |
| `tools/perf/vram_report.py` | Pre-existing. Parses `gpu-mem:` lines from run logs (the line grammar in `GPU.md` §3). |
| `tools/ci/byte_probes.py` | Pre-existing. Owns byte-identity verdicts (the determinism probes cited in `MIGRATION.md`). |
| `.github/workflows/bench.yml` | Pre-existing workflow; `bench_lab.py`'s delivery note specifies the lead's future steps (one leg per configuration, upload **only** the text records — artifact-quota law, never frame PNGs; their sha256 map in the JSON is the provenance). This seat did not verify the workflow's current contents — **not verified**. |

No benchmark campaign was run for the V2 delivery: the
`bench_lab.py` delivery note states no real benchmark was run
(no binary exists locally on the VM; CI is the only build), and
the synthetic `--self-test` is the tooling proof.

## 2. `bench_lab.py` usage

```
bench_lab.py run --binary PATH --out DIR --label NAME
                 --frames A..B [--paint-cache N] [--film DIR]
bench_lab.py record RECORD.json [--out REPORT.md]
bench_lab.py --self-test
```

- `run` executes, under GNU `time -v` when available:
  `<binary> --film <film> --out <out>/<label>.frames --from A
  --to B`, with `TALERENDER_FRAME_TIMING=1` (and
  `TALERENDER_PAINT_CACHE=N` when `--paint-cache` is given;
  `--film` defaults to `film`, the bench.yml repo-root shape).
  It captures combined output to `<label>.log`, hashes every
  `frame_*.png` produced, and writes `<label>.json` +
  `<label>.md` (+ `<label>.time`). The frames directory must be
  fresh: if it already holds frame PNGs the driver would
  resume-skip them and the timing would be invalid, so the run
  is **refused** instead. When `/usr/bin/time` is unavailable
  the binary runs unwrapped: `total_s` falls back to the tool's
  monotonic wall clock and `peak_rss_kb` to the completion
  line's runlog self-report or `null`.
- `record` re-renders the `.md` from an existing `.json` record
  alone (JSON only; nothing is re-measured).
- `--self-test` assembles a record from a synthetic fake run in
  a temp dir — it never invokes git, a binary, or the network.
- Exit codes: 0 = run recorded / record re-rendered / self-test
  passed; 1 = the render exited non-zero (the record is still
  written, with `exit_code` set) or the self-test failed;
  2 = usage/input error (one `ERROR:` line on stderr, no record).

## 3. Record schema — `talerender-bench/1` (one JSON object)

| Field | Content |
|---|---|
| `schema` | The literal string `talerender-bench/1`. |
| `label` | The `--label` name (also the file stem). |
| `binary_path` | The `--binary` path as given. |
| `binary_sha256` | sha256 hex of the binary file. |
| `git_head` | `git rev-parse HEAD` resolved at run time, or `null` when unresolvable (never guessed). |
| `adapter` | `{renderer_line, adapter_class}` — `renderer_line` is the first `renderer:` (or `adapter:`) line of the run log, verbatim, or `null`; `adapter_class` per §4. |
| `frames` | `{count, range, sha256}` — count = frame PNGs *actually found* (never the requested range length by assumption); range = `[A, B]` from `--frames A..B`; sha256 = map of frame filename → sha256 hex. |
| `timing` | `{total_s, frame_ms, per_frame_ms, samples}` — `total_s` = GNU `time -v` elapsed when the wrapper ran, else the tool's wall clock; `frame_ms` = `{mean, p50, p95, p99}` over the per-frame timing lines, each `null` when there are none (never invented); `per_frame_ms` = map frame index (string) → ms; `samples` = number of timing lines. |
| `peak_rss_kb` | From `time -v` "Maximum resident set size (kbytes)"; fallback: the completion line's runlog `peak RSS <MiB> MB` × 1024; else `null`. |
| `paint_cache` | The **last** `paint-cache:` stats line of the log, verbatim, or `null`. |
| `paint_cache_arg` | The `--paint-cache` value used, or `null` for the binary's default. |
| `exit_code` | The render process exit code, or `null` when the record was assembled without a process. |

Percentiles are **nearest-rank** (the schema's definition):
sort ascending; the p-th percentile is the sample at rank
`ceil(p/100 × N)` (1-based, clamped to [1, N]); the mean is the
plain arithmetic mean; an empty sample set yields `null` for
mean and every percentile.

Per-frame timing lines: the binary emits them under
`TALERENDER_FRAME_TIMING=1` (`runlog::frame_timing_enabled`).
At this HEAD the switch exists but its emission into the film
loop is still the lead's wiring, so the parser accepts the
documented plausible shapes (`frame 123: 45.67 ms`,
`frame 123 — 45.67 ms`, `frame 123 45.67 ms`,
`frame 123: 45.67ms`, `frame-time: frame=123 ms=45.67`,
`frame_time frame 123 45.67 ms`) and ignores everything else —
in particular the 25-frame progress lines and the completion
line are **not** per-frame times. If a binary emits none of
these, `timing.frame_ms` is all `null` and the summary says so;
`total_s` / `peak_rss_kb` still record.

## 4. The lavapipe labelling law

- `adapter_class` is exactly **`lavapipe-software`** when the
  renderer line mentions `llvmpipe` or `lavapipe` (any case);
  **`hardware-or-other`** for any other present renderer line;
  **`unknown`** when there is no renderer line.
- A record may never claim hardware from a software renderer
  line. `hardware-or-other` is a *label*, not a validation
  claim.
- The Markdown summary of a lavapipe record carries the
  software warning and marks GPU validation **PENDING-PC**;
  a record with no renderer line carries the
  "NO RENDERER LINE" warning instead.
- The four comparison configurations of master prompt §10 —
  (A) Chrome GPU, (B) Chrome software, (C) TaleRender GPU,
  (D) TaleRender software — are the campaign design.
  `bench_lab.py` covers single TaleRender runs only: no Chrome
  legs. Cross-record comparison is `report.py`'s territory
  (landed during this wave, §1) — comparison of records is not
  a benchmark campaign, and no records exist yet to compare.

## 5. PENDING-PC

Every GPU/hardware row remains **PENDING-PC**: all rendering
evidence in this repository is CI lavapipe (software Vulkan).
The same `bench_lab.py run` command on the user's PC
(Ryzen 5 PRO 5650G, Vega 7, RADV) with the RADV binary produces
the hardware record; until the validation wave runs that PC
session, no number in any downstream report may be presented
as a hardware result, and no PERF table cell should be filled
from a lavapipe record.
