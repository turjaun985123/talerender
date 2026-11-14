# BENCH-FIRST-W64 — first `bench.yml` run: extraction + analysis

**Seat:** Wave 2 W64 (Batch 7). **Scope:** analysis of the first-ever
dispatch of Batch 2 W17's bench harness. New file only; nothing else
touched, nothing committed. `docs/runbook/PERF.md` is **not** edited —
its tables stay blank for the user's PC (RADV), per its own rule.

**Run of record:** GitHub Actions run **43623743480**, workflow
*Bench (dispatch)*, head **`aa92cee4288c798e4db9b7a40091b71fca567d11`**,
dispatched by the orchestrator. Job started 2026-11-14T17:55:47Z,
run completed **2026-11-14T18:06:31Z, conclusion: success** — every
step green, including both bench legs, the artifact upload, and the
verdict step (both legs' exit files are `0`). Artifact
`bench-43623743480` (logs only, 7-day retention) was downloaded and
analysed for this note; all numbers below are extracted from it.

> ⚠ **SOFTWARE RENDERER — CI LAVAPIPE ONLY.** Every number in this
> note was produced on a GitHub-hosted ubuntu-22.04 runner under
> llvmpipe (CPU). These are **not hardware performance** and must
> never be quoted as such (PERF.md honesty rule). No RADV or PC
> performance claim appears anywhere in this note. What this run
> proves: the harness works end-to-end on its first dispatch, the
> log formats parse through the house tool unmodified, and the two
> paint-cache configurations compare under identical conditions.

**Renderer (verbatim, identical in both legs — the
`bench_report.py` labelling law is satisfied by a single renderer
string for the whole set):**

```
renderer: llvmpipe (LLVM 15.0.7, 256 bits) | backend: Vulkan | device type: Cpu | driver: llvmpipe Mesa 23.2.1-1ubuntu3.1~22.04.4 (LLVM 15.0.7)
```

**Binary (from the artifact's `env.txt`):** sha256
`6a6977cc0219c302129b385e075d6e8b761e011d319f2d428c40fe01870b6b09`
(`./target/release/talerender`), `--version` → `talerender 1.0.0`.
Runner environment: kernel `6.8.0-1070-azure`, 7.8 GiB RAM (the CI
runner's, not the PC's), lavapipe ICD pinned by the workflow.

The extraction was done by running the house tool itself —
`python3 tools/perf/bench_report.py <artifact-dir>` — which exited
0 and applied its own rules (renderer census, software banner,
no extrapolation for CI legs). Tables in §1 are its output; the
paint-cache analysis in §2 reads the completion lines it does not
tabulate.

---

## 1. Per-leg results

Both legs rendered the fixed CI window — film frames **0..100**
(100 frames, fresh output directory per leg) — and both are **valid
timings** by PERF §3's rule: completion lines report
`100 rendered, 0 skipped` over the full range, exit 0.

### 1.1 Totals (elapsed source: `/usr/bin/time -v`, as the tool prefers)

| leg | wall s | user CPU s | CPU % | peak RSS KiB (`time -v`) | s/frame | frames/hour |
|---|---|---|---|---|---|---|
| A — `bench-default`, paint cache default (cap 160) | 185.32 | 286.90 | 156% | 1,551,904 | 1.853 | 1,943 |
| B — `bench-cache24`, `TALERENDER_PAINT_CACHE=24` | 183.78 | 286.61 | 157% | 1,401,472 | 1.838 | 1,959 |

No full-film extrapolation is computed: PERF §3 sanctions that
arithmetic only for warm mid-film PC chunks, and the report tool
correctly leaves the CI legs' extrapolation cells empty.

Cross-check: each binary's own completion line self-reports
`peak RSS 1515.5 MB` (leg A) / `1368.6 MB` (leg B) in MiB — both
match the `time -v` figures exactly (1,551,904 KiB = 1515.5 MiB;
1,401,472 KiB = 1368.6 MiB). The render-loop elapsed on the
completion lines (03:04 / 03:02) sits ~1.5 s under the process
wall in both legs — startup + teardown outside the loop is small
but is *inside* every figure above (the CI bench is deliberately
startup-inclusive; PERF's PC chunk protocol is the steady-state
one, and the two are never merged).

### 1.2 Per-range frame times (the finest grain the binary emits)

There is **no per-frame timing** in the logs — the standing
instrumentation limit (PERF §9 item 1). The finest grain is the
four 25-frame progress ranges per leg (elapsed read off the W09
progress lines, whole-second resolution, ±1 s per boundary):

| range (frames) | leg A s | leg A s/frame | leg B s | leg B s/frame |
|---|---|---|---|---|
| 0..24 | 41 | 1.640 | 39 | 1.560 |
| 25..49 | 47 | 1.880 | 48 | 1.920 |
| 50..74 | 48 | 1.920 | 48 | 1.920 |
| 75..99 | 48 | 1.920 | 47 | 1.880 |

**Shot-boundary cross-reference** (from `scenes/film/film.json` at
this head): segment **S01 spans frames 0..178** (t 0–5.907 s, JG
set, noon; S02 starts at frame 178). The entire bench window lies
**inside S01** — there is no segment transition in frames 0..100
at all (see anomaly A1). Within the window the cost shape is a
single step, not spikes:

- Range 1 (frames 0..24, t 0–0.80 s) is the cheapest in both legs
  *despite carrying all process startup* (asset load, pipeline
  build). The film opens with the 0.25 s fade-in: frame 0's PNG is
  44,758 bytes, frame 1 is 1.33 MB, frame 2 is 2.00 MB, rising to
  the ~3.0 MB plateau — cheap early frames (low entropy → cheap
  paint/composite/PNG-encode) outweigh startup. A bound follows:
  startup + the first 25 frames cost 41/39 s, *less* than 25
  frames at the plateau rate (48 s), so startup alone is well
  under the plateau cost of the range it hides in.
- Ranges 2–4 are flat (47–48 s in both legs, 1.88–1.92 s/frame):
  no frame-time spike is localised anywhere in the window, and
  the two legs agree range-by-range to within the ±1 s progress
  granularity (and the leg-order confound, §3.3).

### 1.3 Disk — frame sizes (§5 shape)

| set | frames | min / median / mean / max PNG | full-film @ mean × 1811 | bound @ max × 1811 |
|---|---|---|---|---|
| `frames-default.sizes` | 100 | 0.04 / 3.00 / 2.94 / 3.05 MB | 5.33 GB | 5.52 GB |
| `frames-cache24.sizes` | 100 | 0.04 / 3.00 / 2.94 / 3.05 MB | 5.33 GB | 5.52 GB |

All 100 frames have **identical sizes in both legs** (size
equality only — byte identity is the determinism workflow's
evidence, not this run's). PNG size is a property of frame
content, not of the renderer, so this distribution is the one
number in the note that transfers to the PC session as a *prior
expectation* (PERF §5's framing): it sits inside USER-PC's
2–6 GB budget range. Caveat: the window is all-S01 (bright JG
noon); later segments (night, S06 fog) may shift the mean — the
PC §5 measurement on the S08 warm chunk will triangulate.

---

## 2. The paint-cache leg comparison

The completion lines carry the W06 paint-budget statistics (this
is PERF §9 item 2's signal, landed since the protocol was
written). Verbatim tails:

```
leg A: paint-cache: entries=74 bytes=2133852160 hits=11 misses=89 evictions=15 rerenders=0 peak_bytes=2162688000 budget_bytes=2147483648
leg B: paint-cache: entries=24 bytes=692060160 hits=11 misses=89 evictions=65 rerenders=0 peak_bytes=720896000 budget_bytes=692060160
```

Arithmetic checks (all exact): one canvas accounts
**28,835,840 B** (2560×2816 RGBA) — leg A ends at 74 entries
(74 × 28,835,840 = 2,133,852,160 ✓), peaks at 75; leg B ends and
peaks at 24/25. Paints − evictions = retained in both legs
(89 − 15 = 74; 89 − 65 = 24 ✓).

**What the comparison shows:**

1. **Identical work.** Both legs: 89 misses (89 canvases painted),
   11 hits, 0 rerenders. Leg B evicted 50 more canvases than leg A
   and paid **zero re-paints** for them — every evicted state was
   dead for the rest of the window. The first 100 frames' reuse
   distance is short: the 11 reuses all fall within a 24-entry
   window. This is a property of *this window*, stated as such —
   it is not evidence about the full film (later segments revisit
   character states across longer distances, e.g. the
   S07→S08/S08b/S09 inherited dressing).
2. **Time: no measurable difference.** Wall 185.32 s vs 183.78 s
   (−0.8% for the small cache) and user CPU 286.90 s vs 286.61 s
   are indistinguishable at single-run precision, and §3.3's
   confound points leg B's way. The expected trade — smaller
   cache → more time, less memory — shows **only its memory
   half** in this window, and the counters explain why (point 1:
   there was no extra paint work to pay for). Nothing here says
   the time half is zero at cap 24 for the whole film.
3. **Memory: direction as expected, magnitude surprising.**
   Peak RSS fell 150,432 KiB (≈147 MiB) from leg A to leg B.
   But see anomaly A2: the accounted cache bytes fell by
   1,441,792,000 B (≈1375 MiB) over the same step. The direction
   confirms the trade exists; the ~9× gap between accounted and
   resident bytes is the finding, not the 147 MiB.

**The default configuration's real ceiling.** Leg A's
`budget_bytes` is exactly 2 GiB (2,147,483,648) — the W06 byte
budget — and it bound the cache at 74–75 entries, far below the
160-entry cap. PERF §4's written worst case ("cap 160 → ~4.6 GB")
is therefore superseded by the wiring that landed after it: at
default settings the byte budget, not the entry cap, is the
binding constraint (~2 GiB accounted). Leg B's `budget_bytes`
(692,060,160 = 24 canvases exactly) shows the entries cap
driving the byte budget in that configuration.

---

## 3. What these numbers can and cannot say

### 3.1 What CI lavapipe establishes

- **The harness itself.** W17's note left the first dispatch as
  the workflow's own verification (no runner exists on this VM).
  That verification has now happened: build, environment capture,
  both legs, summary, upload, and verdict all ran as designed,
  and `bench_report.py` consumed the artifact unmodified (exit 0,
  correct banner, correct refusal to extrapolate).
- **A valid relative comparison** of the two paint-cache
  configurations under identical conditions (same binary, same
  runner, back-to-back): §2's conclusions hold for frames 0..100
  on lavapipe.
- **Content data**: the §5 frame-size distribution (§1.3), and
  the internal consistency of the binary's self-reported signals
  (peak RSS vs `time -v`; cache counters vs their arithmetic).

### 3.2 What it cannot say — the confound at the centre

On lavapipe, the "GPU" is llvmpipe: rasterisation runs on the
same CPU cores as the paint stage, the compositor's compute, and
PNG encoding. Leg A's ~287 s of user CPU over a 185 s wall
(156% CPU) is one undifferentiated sum. **These logs cannot
separate CPU paint cost from raster cost** — the paint-cache
counters bound the paint *count* (89 paints, 11 hits) but not
its share of the 185 s. The 1.85 s/frame figure is a software-
renderer number with no hardware meaning: it must not be
compared against, or averaged with, any RADV figure, and the
report tool's mixed-renderer refusal exists precisely to keep
the two sets from ever sharing a table.

### 3.3 Confounds and precision limits (stated, not smoothed)

- **Leg order is fixed A→B** in the workflow, and the page cache
  remembers it: leg A recorded 443 major page faults and 134,096
  file-system inputs; leg B recorded **0 and 0** (assets and the
  binary were warm). Leg B's −1.54 s wall edge is inside this
  confound. Any future sub-1% A/B claim from this harness is
  below its noise floor.
- **One run per leg.** No repeats, no median; runner neighbours
  are unknown. Range-level agreement between the legs (§1.2) is
  the only replication this run contains.
- **±1 s progress granularity** (§1.2) — per-range s/frame
  carries ~±0.04 s/frame of quantisation.
- **Startup-inclusive by design** (frames 0..100): the CI bench
  and PERF's steady-state PC chunk (1027..1117) measure
  different things and are labelled as such wherever either is
  quoted.

### 3.4 What a RADV run separates

On the user's PC the same legs would decompose: rasterisation
moves to the Vega 7 iGPU (RADV), leaving wall time ≈ CPU paint +
GPU render + encode, with the paint-cache counters attributing
the paint side and `time -v` user-vs-wall exposing how much CPU
the process still needs. The cap-160-vs-24 comparison on RADV is
the pure paint-cost/memory trade (raster cost is cap-invariant),
and the resident-bytes-per-canvas question (A2) gets a real
answer on a device whose memory is shared system RAM. None of
that is inferable from this run — it is what §4's shopping list
buys.

---

## 4. Anomalies

- **A1 — the window contains no segment transition.**
  `bench.yml`'s header (and BENCH-W17 §1) describe frames 0..100
  as covering "S01 and the S01→S02 segment transition". Per
  `scenes/film/film.json`, S01 spans frames 0..178; the window
  is startup + the first ~56% of S01 only. The bench therefore
  says nothing about segment-transition cost (asset/scene swap
  at a boundary). Harness-comment correction is a lead/docs
  matter; flagged here, not edited.
- **A2 — accounted cache bytes exceed total process RSS.**
  Leg A's cache *peak accounting* is 2,162,688,000 B (≈2063 MiB)
  while the whole process's peak RSS is 1515.5 MiB — the cache's
  own ledger is ~547 MiB larger than the process that contains
  it. Marginal resident cost between the legs is
  150,432 KiB / 50 peak canvases ≈ **2.9 MiB per canvas**, vs the
  27.5 MiB accounting unit — a ~9× divergence on lavapipe.
  Hypotheses (labelled as such, not findings): wgpu texture
  backing under llvmpipe is lazily faulted, so untouched canvas
  pages never become resident; and/or stored canvas format is
  smaller than the RGBA accounting assumes. Either way, on this
  evidence the 28,835,840 B figure is an **accounting unit for
  the budget**, not a measured resident cost — and PERF §4's
  OOM reasoning, which reads the accounting as memory, needs
  the RADV measurement (§4, item 6) before it is trusted in
  either direction.
- **A3 — peak accounting overshoots the budget by exactly one
  canvas in both legs** (leg A peak 2,162,688,000 vs budget
  2,147,483,648; leg B 720,896,000 vs 692,060,160 — each exactly
  +28,835,840 B). Consistent with admit-then-evict semantics
  (the just-painted entry is admitted, eviction follows). Benign
  and self-consistent; recorded so the PC session doesn't
  rediscover it as a surprise.

No other anomalies: no skipped frames, no exit-code failures, no
renderer disagreement between legs, no per-range spikes (§1.2).

---

## 5. PC-session shopping list — the RADV legs that pair with this run

W36's `PC-EVIDENCE-W36.md` §2 already runs PERF.md §§1–5 on the
PC (stills, the steady-state chunk 1027..1117, both telemetry
cache configurations, disk). Those remain the performance
protocol of record. The invocations below are an **addendum**:
the bench.yml leg shapes repeated on RADV so the PC numbers land
commensurable with run 43623743480 — same frames, same legs, same
file names, same tool. They do not replace any PERF leg (PERF's
chunk stays the only source of a full-film extrapolation).

Run from the repo root on the PC, machine idle, with the CI-built
binary provisioned per W36 §1 / USER-PC (canonical
`./talerender-bin/talerender`), **after** PERF §1's environment
capture, and only if the binary's `renderer:` line names the
RADV device (device type not `Cpu`) — PERF §1's stop rule.

```bash
mkdir -p perf-radv

# env capture mirroring bench.yml's env.txt fields, so the two
# env.txt files diff field-for-field (plus the ICD you pinned):
{
  echo "== date ==";    date -Is
  echo "== kernel ==";  uname -r
  echo "== ram ==";     free -h
  echo "== icds ==";    ls /usr/share/vulkan/icd.d/
  echo "== icd pinned =="; echo "$VK_ICD_FILENAMES"
  echo "== binary ==";  sha256sum ./talerender-bin/talerender
  echo "== version =="; ./talerender-bin/talerender --version
  echo "== bench ==";   echo "film frames 0..100, legs: cap160 (default), cap24 (TALERENDER_PAINT_CACHE=24) — RADV repeat of CI run 43623743480"
} > perf-radv/env.txt 2>&1

# Leg A — default paint cache (fresh directory; A before B, the
# same fixed order as CI, so the page-cache confound at least
# points the same way):
/usr/bin/time -v -o perf-radv/bench-default.time \
  ./talerender-bin/talerender --film film \
    --out perf-radv/frames-default --from 0 --to 100 \
  > perf-radv/bench-default.log 2>&1
echo $? > perf-radv/bench-default.exit

# Leg B — TALERENDER_PAINT_CACHE=24 (fresh directory):
TALERENDER_PAINT_CACHE=24 /usr/bin/time -v -o perf-radv/bench-cache24.time \
  ./talerender-bin/talerender --film film \
    --out perf-radv/frames-cache24 --from 0 --to 100 \
  > perf-radv/bench-cache24.log 2>&1
echo $? > perf-radv/bench-cache24.exit

# Frame sizes, the workflow's own method:
stat -c '%s %n' perf-radv/frames-default/frame_*.png > perf-radv/frames-default.sizes
stat -c '%s %n' perf-radv/frames-cache24/frame_*.png > perf-radv/frames-cache24.sizes

# Tables (RADV-headed; the tool refuses to merge this set with
# the CI artifact's — the two reports are compared side by side
# by a human, never merged):
python3 tools/perf/bench_report.py perf-radv --out perf-radv/REPORT.md
```

Validity gates, identical to the CI run's: each completion line
must read `100 rendered, 0 skipped (resume), range 0..100`; both
`.exit` files must be `0`; the `renderer:` line in both logs must
name the same RADV device (if it names llvmpipe, stop — that is
a configuration result, not a performance result, per PERF §1).

One extra measurement this CI run argues for, free in the logs:
**keep the completion lines' paint-cache statistics and the
`time -v` peak RSS for both legs together** — their pairing is
what computes RADV's resident cost per canvas
(Δ peak RSS ÷ Δ peak entries between the legs), the number that
decides whether PERF §4's OOM concern is real on the PC's
~6.7 GiB (anomaly A2). No extra invocation; just don't strip the
logs.

Return package addition for the session: the whole `perf-radv/`
directory (env.txt, both legs' `.log`/`.time`/`.exit`, both
`.sizes`, `REPORT.md`) alongside W36's existing `perf/` package.

---

## 6. Provenance and limits of this note

- Source artifact: `bench-43623743480` from run 43623743480
  (head `aa92cee`), downloaded 2026-11-14; the workflow's 7-day
  artifact retention applies to re-download.
- Segment boundaries: `scenes/film/film.json` at the current
  tree (S01 `startFrame` 0 / `endFrame` 178).
- The run completed before this seat began work; no IN-FLIGHT
  section is needed. The `gh run view` status line lagged the
  jobs API by one poll during extraction (showed `in_progress`
  after the job had completed); the jobs API and the downloadable
  artifact are the evidence relied on.
- Nothing in this note fills, or licenses filling, PERF.md's
  tables: those are RADV-only by the protocol's own law, and the
  PC session (§5 + W36 §2) is what fills them.
