# PERF.md — TaleRender performance measurement protocol (user's PC)

**Purpose.** Produce the first real performance numbers for TaleRender —
on the machine that matters: the user's PC (Ryzen 5 PRO 5650G, Radeon
Vega 7 iGPU, RADV Vulkan, Ubuntu, ~6.7 GiB RAM). Every number this
project currently has was measured on a GPU-less VM under lavapipe, or
not measured at all; **no figure in this document is a result** — the
tables are blanks for the PC run to fill.

**Rules of the protocol.**

- Run every step on the PC, on an idle machine (nothing else rendering,
  compiling, or capturing). Note anything else that was running.
- Record the environment (§1) **before** any timing. If the binary's
  own `renderer:` / `adapter:` line names a software device
  (llvmpipe / lavapipe, device type `Cpu`), **stop**: those numbers
  would be software-render numbers. Pin the RADV ICD per
  [USER-PC.md](USER-PC.md) §2 and re-check the line.
- Report raw numbers (every run, not just the median). A median is a
  summary you add, never a replacement for the runs.
- Judge film telemetry by its verdict **line**, never its exit code:
  the binary exits 0 on pass, fail, and error alike (§7).
- All commands run from the repo root with the CI binary at
  `./talerender-bin/talerender` (see USER-PC.md §1). The binary has
  `-h`/`--help` and `-V`/`--version` (added after v1.0.0); unknown
  arguments print `error: …` and exit 2.

**Return package.** When done, send back: the §1 environment block,
the filled tables (§2–§6), the §8 summary, and the full logs
(`perf/*.log`, `perf/*.time`). That is the whole result — no prose
verdict needed.

---

## §1 Environment capture (before any timing)

```bash
mkdir -p perf
{
  echo "== date ==";        date -Is
  echo "== kernel ==";      uname -r
  echo "== ram ==";         free -h
  echo "== icds ==";        ls /usr/share/vulkan/icd.d/
  echo "== vulkan ==";      vulkaninfo --summary 2>/dev/null | sed -n '/Devices:/,$p'
  echo "== binary ==";      sha256sum ./talerender-bin/talerender
  echo "== ci run ==";      echo "<the run id the artifact was downloaded from>"
} > perf/env.txt 2>&1
```

Then confirm the adapter the binary itself selects — the first lines
of any scene or film log (§2 run 1 is fine) must contain one of:

```
adapter: <name> | backend: Vulkan | device type: <type> | driver: <driver> <info>     (scene modes)
renderer: <name> | backend: Vulkan | device type: <type> | driver: <driver> <info>    (--film mode)
```

Ground truth for a valid run: `<name>` names the AMD/RADV device
(e.g. contains `RADV`), and `<type>` is **not** `Cpu`. Copy that line
into `perf/env.txt` too. A run whose line names llvmpipe is a
configuration result, not a performance result — fix the ICD first.

---

## §2 Scene-file render timing (single stills)

Two stills: the milestone scene (full composer chain) and one film
segment's scene file (the loader path, at the file's staged instant —
a pipeline-cost sample, not a film frame).

```bash
cd ~/workspace/talerender   # repo root; the ab1 caption path is relative

# A. ab1 — five runs, same command each time
for i in 1 2 3 4 5; do
  /usr/bin/time -f "%e %M" -o perf/ab1-$i.time \
    ./talerender-bin/talerender --scene ab1 --out perf/ab1-$i.png \
    > perf/ab1-$i.log 2>&1
done

# B. one film segment still — five runs
for i in 1 2 3 4 5; do
  /usr/bin/time -f "%e %M" -o perf/s01-$i.time \
    ./talerender-bin/talerender --scene-file scenes/film/s01.scene.json \
      --out perf/s01-$i.png \
    > perf/s01-$i.log 2>&1
done
```

Each `.time` file holds `<elapsed seconds> <peak RSS in KiB>`.
Run 1 is the **cold** run (first process start: shader compilation,
pipeline creation, cold OS caches). Runs 2–5 are warm. Report run 1
separately; the warm figure is the median of runs 2–5 (mean of the
2nd and 3rd values when sorted).

| still | run 1 (cold) s | runs 2–5 s (all four) | warm median s | peak RSS (warm runs) KiB |
|---|---|---|---|---|
| ab1 (`--scene ab1`) | | | | |
| S01 (`--scene-file scenes/film/s01.scene.json`) | | | | |

Also verify each log's `adapter:` line (§1) and that all five PNGs of
a set are byte-identical (`md5sum perf/ab1-*.png`) — determinism is
part of what is being confirmed; a differing hash is a defect report,
not a timing footnote.

---

## §3 Film render throughput (90-frame chunk)

**Chunk.** Frames **1027..1117** — the first 90 frames of segment
S08 (JG noon, the film's longest segment, mid-film, so the paint
cache is in a representative working state). Frame *n* is staged at
t = n/30 s, i.e. this chunk covers t = 34.23…37.20 s.

**Fresh-directory rule.** The driver resumes by skipping any frame
file that already exists — a re-run over existing frames measures
nothing. Each timed run uses a fresh `--out` directory.

```bash
# Cold run (first film invocation: asset load + pipeline build included)
rm -rf perf/chunk-cold
/usr/bin/time -f "%e %M" -o perf/chunk-cold.time \
  ./talerender-bin/talerender --film film --out perf/chunk-cold \
    --from 1027 --to 1117 \
  > perf/chunk-cold.log 2>&1

# Warm run (identical range, fresh directory)
rm -rf perf/chunk-warm
/usr/bin/time -f "%e %M" -o perf/chunk-warm.time \
  ./talerender-bin/talerender --film film --out perf/chunk-warm \
    --from 1027 --to 1117 \
  > perf/chunk-warm.log 2>&1
```

The log's progress lines land every 25 rendered frames
(`  frame 1051 (t = 35.03s) — 25 rendered, 0 skipped`, …) and end
with `film render complete: 90 rendered, 0 skipped (resume), range
1027..1117`. Anything else in the final line (fewer rendered, any
skipped) invalidates the timing — say so and keep the log.

**Arithmetic (fill in with the warm run's own numbers — this is the
only sanctioned way to quote a full-film figure):**

```
elapsed_s        = <warm .time elapsed>
s_per_frame      = elapsed_s / 90
frames_per_hour  = 3600 / s_per_frame
full_film_hours  = 1811 / frames_per_hour        (= s_per_frame × 1811 / 3600)
```

| chunk 1027..1117 | elapsed s | peak RSS KiB | s/frame | frames/hour | extrapolated 1,811 frames |
|---|---|---|---|---|---|
| cold | | | | | — |
| warm | | | | | ____ h (arithmetic above) |

**What the extrapolation does and does not claim.** It assumes the
measured chunk is representative. Two known distortions, both stated
with the result: (1) a mid-segment chunk under-weights the
paint-cache misses at each of the 14 segment starts; (2) shot costs
differ — S06 (fog + fire) is plausibly the heaviest. Optional second
chunk, same procedure, frames **784..874** (inside S06):

| optional chunk 784..874 (S06, fog) | elapsed s | s/frame | frames/hour |
|---|---|---|---|
| warm | | | |

If both chunks are measured, quote the full-film estimate as the
range between the two extrapolations, never a single number.

---

## §4 Memory (peak RSS)

Measured with `/usr/bin/time -v` (`Maximum resident set size`). Three
configurations:

```bash
# A. Film telemetry, default paint cache (cap 160)
/usr/bin/time -v -o perf/telemetry-default.time \
  ./talerender-bin/talerender --film-telemetry film \
  > perf/telemetry-default.log 2> perf/telemetry-default.err

# B. Film telemetry, small cache (the memory-tight setting)
TALERENDER_PAINT_CACHE=24 /usr/bin/time -v -o perf/telemetry-24.time \
  ./talerender-bin/talerender --film-telemetry film \
  > perf/telemetry-24.log 2> perf/telemetry-24.err

# C. The §3 warm chunk already carries peak RSS in perf/chunk-warm.time
#    (that file's second field). No separate run needed.
grep "Maximum resident" perf/telemetry-default.time perf/telemetry-24.time
```

**The cache math the numbers are read against.** One painted
character canvas is 2560×2816 RGBA ≈ **29 MB**. The paint cache is
FIFO-capped: default **160 entries → ~4.6 GB worst case**; cap 24 →
**~0.7 GB**. Eviction is pixel-neutral (painting is a pure function
of the state), so a smaller cap costs time, never correctness. Total
RSS also includes the renderer (GPU-mapped buffers, textures, frame
targets), the loaded film assets, and the binary itself — so RSS
above the cache ceiling alone is expected; RSS far above
ceiling + ~1 GB is a finding. On this PC's ~6.7 GiB RAM, the default
cap's worst case is a genuine OOM risk (§7), which is exactly what
configuration A tests: telemetry touches every character state in
the film at 0.25 s sampling.

| configuration | verdict line (telemetry) | elapsed s | peak RSS |
|---|---|---|---|
| A. telemetry, cap 160 (default) | CHECK OK / FAILED (from .log) | | |
| B. telemetry, `TALERENDER_PAINT_CACHE=24` | CHECK OK / FAILED (from .log) | | |
| C. film chunk 1027..1117 warm, cap 160 | — (from §3) | | |

A and B must reach the **same verdict line**. Different verdicts at
different caps would be a determinism defect — report it as such,
with both logs.

---

## §5 Disk (frame sizes → full-film budget)

Run after §3, on the warm chunk's 90 PNGs (clean PNG masters,
decision B — there is no JPEG generation in these files):

```bash
python3 - <<'EOF'
import glob, statistics
sizes = sorted(len(open(p,'rb').read()) for p in glob.glob('perf/chunk-warm/frame_*.png'))
n = len(sizes); mean = sum(sizes)/n
print(f"frames measured : {n}")
print(f"min / median / mean / max : {sizes[0]/1e6:.2f} / {statistics.median(sizes)/1e6:.2f} / {mean/1e6:.2f} / {sizes[-1]/1e6:.2f} MB")
print(f"chunk total     : {sum(sizes)/1e9:.3f} GB")
print(f"full film @mean : {mean*1811/1e9:.2f} GB   (mean x 1811)")
print(f"full film @max  : {sizes[-1]*1811/1e9:.2f} GB   (worst-frame bound)")
EOF
```

| quantity | value |
|---|---|
| frames measured | 90 |
| min / median / mean / max PNG size | |
| full-film budget @ mean × 1811 | GB |
| full-film bound @ max × 1811 | GB |

Prior expectation to confirm or replace: USER-PC.md budgets
roughly 2–6 GB for the full PNG set. The measured distribution
supersedes it. Note the frames
directory must hold the full 1,811 PNGs at once during a production
render — the budget is a disk requirement, not a transfer size.

---

## §6 The comparison that matters: native vs Chrome capture (same PC)

Same PC, same 90 frames' worth of film (t = 34.23…37.20 s), same
1080×1920 output size. The Chrome side is the skill's patched
capture route on the Onoda project — `capture.mjs` with
`CHROME_FLAGS="--no-sandbox --use-gl=angle --use-angle=gl"` (the
GPU path from the two-bombs project; the stock capture without
those flags falls back to SwiftShader and is not the comparison).
Capture the same time window, wall-clock the capture step only.

**Read this before quoting the table.** The two routes do not
produce the same artifact, and the table must never be quoted
without that fact:

- **Native** writes clean PNG masters (decision B: no JPEG
  generation, ever). PNG encoding cost is inside the native time.
- **Chrome** frames come through the browser pipeline; the legacy
  pipeline's masters carry a deliberate q0.93 JPEG generation (the
  M0 compatibility contract documents it). Browser capture cost is
  inside the Chrome time.
- Neither number includes the finishing step (ffmpeg encode/mux),
  which is shared downstream tooling.
- Prior context, to confirm or refute: the patched Chrome route is
  on record at ~0.15 s/frame for stills on this GPU at full
  quality. That figure predates this protocol and is not a result.

| route | frames | wall time s | s/frame | frames/hour | output artifact |
|---|---|---|---|---|---|
| TaleRender native (`--film`, §3 warm) | 90 | | | | clean PNG |
| Chrome patched capture (same window) | 90 | | | | browser-pipeline frames |

---

## §7 Failure protocol

A failure during measurement is a result. Record the exit code
(`echo $?` immediately after), keep **all** output (stdout and
stderr as separate files), and send the items listed. Do not retry
with different flags and report only the success.

| symptom | signature | what it means / do |
|---|---|---|
| **Segfault** | exit **139**; shell prints `Segmentation fault (core dumped)`; log stops mid-range | On RADV this is a real defect signal. (A llvmpipe JIT segfault is a known artifact of the GPU-less VM — it must never be used to dismiss a crash on this PC.) The last progress line names the last completed 25-frame boundary. **Before any resume:** delete the highest-numbered `frame_*.png` in the out dir — resume skips by existence, and the last file may be truncated mid-save (see USER-PC.md). Send: full log, exact command, frame range, §1 env block, and whether re-running just the surrounding 25-frame block (`--from <boundary> --to <boundary+25>`) reproduces it. |
| **OOM kill** | exit **137**; shell may print `Killed`; `dmesg \| tail -20` or `journalctl -k -n 50` shows `Out of memory: Killed process … talerender` | The default cache cap's ~4.6 GB worst case (§4) plus renderer and system on ~6.7 GiB RAM. Re-run the failed step with `TALERENDER_PAINT_CACHE=24` and record that the mitigation was required — that fact is part of the result. Send: the dmesg/journal excerpt, `free -h` at idle, which step and how far it got. |
| **No adapter** | panic: `no wgpu adapter found — a Vulkan driver (hardware or lavapipe) is required` | Driver/ICD configuration, not a crash. Follow USER-PC.md §2 (pin `VK_ICD_FILENAMES` to the radeon JSON), re-check `vulkaninfo --summary`, retry. Send the vulkaninfo summary if it persists. |
| **Device failure** | panic: `request_device failed`, after a valid `renderer:`/`adapter:` line | Send the full log including the adapter line — which device failed is the diagnostic. |
| **Bad invocation** | exit **2**, `error: unknown argument: …` (or `unknown scene:` / `unknown --post-stage:`) + `Try 'talerender --help' …` | A typo'd flag. Run `talerender --help` for the flag reference, correct, and re-run; nothing to send. |
| **Telemetry FAILED** | exit **0**, stdout line `FILM TELEMETRY: CHECK FAILED (<n> overflow groups, <n> continuity violations, <n> caption failures)` | A data verdict, not a crash — and not a performance result either way. Send the full telemetry stdout; the counts are the report. |
| **Short render** | final line reports `< 90 rendered` or any `skipped` in a fresh directory | The timing is invalid. Keep the log, note the counts, send both. |

**Send-back checklist (any failure):** §1 env block · the exact
command line · exit code · stdout log · stderr log · the frame
range and last progress line (film) · dmesg excerpt (OOM only) ·
whether a re-run of the same range reproduces it.

---

## §8 Results summary (fill and return with the logs)

| measure | result |
|---|---|
| adapter line (verbatim) | |
| ab1 still — warm median s | |
| S01 still — warm median s | |
| chunk 1027..1117 warm — s/frame / frames per hour | |
| full-film extrapolation (arithmetic from §3) | |
| S06 chunk (if measured) — s/frame | |
| telemetry peak RSS — cap 160 / cap 24 | |
| chunk peak RSS — cap 160 | |
| PNG mean size → full-film disk budget | |
| Chrome comparison — s/frame (with §6 caveats) | |
| failures encountered (§7 class) | |

---

## §9 Missing signals — feature requests for the integrator

The protocol above works, but four measurements are impossible or
indirect because the binary does not emit the signal. These are
requests, recorded here — the workarounds in §2–§4 (whole-run wall
clock, external RSS) are protocol scaffolding, not solutions.

1. **No per-frame timing, and no clock on progress lines.** Film
   progress prints every 25 rendered frames with no elapsed time.
   The per-frame cost distribution, the single slowest frame, and
   intra-run drift (e.g. thermal throttling across 1,811 frames)
   cannot be measured from the binary's own output. *Request:*
   progress lines carry elapsed seconds; optionally a per-frame
   millisecond line behind an env flag (e.g.
   `TALERENDER_FRAME_TIMING=1`).
2. **No paint-cache statistics.** No hits / misses / evictions /
   final size are printed at end of run, so CPU painting cost
   cannot be separated from GPU render cost, and the
   `TALERENDER_PAINT_CACHE` time-for-memory trade (§4) cannot be
   quantified from output — only inferred from total time and RSS.
   *Request:* an end-of-run cache stats line in film and telemetry
   modes.
3. **No peak-RSS self-report.** Memory measurement depends on the
   caller wrapping the binary in `/usr/bin/time -v`; any future
   launcher or wrapper silently loses the signal. *Request:* the
   process reports its own peak RSS on exit (stderr is fine).
4. **Film telemetry always exits 0.** Pass, fail, and error are
   indistinguishable to scripts; §4/§7 must grep the verdict line.
   *Request:* non-zero exit on `CHECK FAILED` (the per-shot
   `--telemetry` modes already return 0/1/2 properly).
5. **Named blind spot — GPU memory.** wgpu exposes no VRAM figure
   here, and on the 5650G the iGPU shares system RAM, so GPU-side
   pressure is invisible in both the binary's output and (partly)
   host RSS. External observation (`radeontop`, if installed) is
   the only route; recorded as a blind spot, not a request — but
   any future adapter-info extension that surfaces it should be
   printed next to the `renderer:` line.
