# PC-EVIDENCE-W36 — the one-visit RADV evidence pack

**Seat:** TaleRender Wave 2, W36. **What this is:** the consolidated
runbook for a single sitting on the user's PC that collects *every*
piece of RADV evidence the project still owes, in the order the
sitting should run it, with exact commands. It sequences three
existing documents — it does not replace any of them:

- [USER-PC.md](USER-PC.md) — binary provisioning, adapter checks,
  production render + finish (the operator runbook).
- [PERF.md](PERF.md) — the performance measurement protocol; its
  tables are the ones this visit fills.
- [DETERMINISM-CONTRACT-W04.md](../integration/DETERMINISM-CONTRACT-W04.md)
  §7 — the RADV determinism leg (evidence item **E7**).

**Read this first — the honesty rule.** Every native number this
project currently has was measured on CI's **lavapipe** (software
Vulkan on a GPU-less runner) or on a GPU-less VM. They are
correctness evidence, not hardware evidence. **No RADV number
exists yet.** This pack exists to change that. Accordingly:

- No table in this document is pre-filled, and no result is
  promised anywhere in it. Where a duration or size is discussed,
  it is either a *formula* (fill it in from your own earlier leg)
  or a figure cited from the source documents and labelled as
  what it is (a lavapipe bound, a prior expectation, cache byte
  math) — never a PC result.
- Every leg's output names its renderer. A leg whose logs do not
  prove RADV ran is not evidence — it is a configuration result
  (PERF.md's rule, enforced mechanically by the report generator,
  §2.4 below).

**The machine:** AMD Ryzen 5 PRO 5650G (6C/12T), Radeon Vega 7
iGPU (gfx90c), RADV Vulkan driver, Ubuntu, **~6.7 GiB usable
RAM**. The RAM figure is the session's binding constraint; each
leg flags where it bites.

## Session map — why this order

| # | Leg | Hands-on? | Produces |
|---|---|---|---|
| 0 | Setup + adapter proof | yes | a binary + checkout whose renderer line is proven RADV; `perf/env.txt` |
| 1 | Determinism E7 (RADV pack) | mostly waiting | `determinism-results.txt` — the formal RADV determinism verdict |
| 2 | Bench legs (PERF protocol) | yes, machine must be idle | `perf/` logs → PERF-shaped tables (first RADV performance numbers) |
| 3 | Full film render + telemetry verdict | start it, then unattended | all 1,811 frames + the film gate verdict on this machine |
| 4 | Golden verification (verify ≠ pin) | yes | per-segment PSNR of RADV frames vs the f021991 (lavapipe) candidates |
| 5 | Send-back | yes | the exact file package in §5 |

The order is load-bearing: §1 is short and gates trust in
everything after it (if this renderer cannot reproduce its own
bytes, timings and goldens measured on it are built on sand);
§2 needs an idle machine, so it runs before the long render;
§3 is the only leg that produces the frames §4 verifies, so §4
cannot start until it finishes; §5 is bookkeeping, done while
the filenames are in front of you.

---

## §0 Setup — checkout, binary, adapter proof

### 0.1 Checkout (the data side of every claim)

Determinism's "same data" term means *the same repo commit*
(DETERMINISM-CONTRACT §1). Get a checkout and record its head:

```bash
# clone if absent, pull if present — either way, end at a known head
git clone https://github.com/turjaun985123/talerender.git ~/workspace/talerender
#   — or, in an existing checkout:  git pull
cd ~/workspace/talerender
git rev-parse HEAD        # record this; every result below is cited against it
```

### 0.2 Binary — from CI, not a local build

USER-PC.md §1 is the provisioning route and this pack follows
it exactly: **the binary comes from a green CI run's
`talerender-binary` artifact.** CI builds on `ubuntu-22.04`
(glibc 2.35) on purpose so the artifact runs on this PC as-is;
no local cargo build is part of the route. Use a run at or
after the CLI release (the professional `--help`/`--version`
surface) — the determinism pack refuses stale binaries, and
the results files record the binary's sha256 either way.

```bash
gh run list --repo turjaun985123/talerender --workflow CI --limit 5
gh run download <run-id> --repo turjaun985123/talerender \
    -n talerender-binary -D ~/talerender-bin
chmod +x ~/talerender-bin/talerender
~/talerender-bin/talerender --version    # sanity: 1.0.0 or later
sha256sum ~/talerender-bin/talerender    # record it (legs below do too)
```

### 0.3 Environment capture (before any timing)

Run PERF.md §1 verbatim — it writes `perf/env.txt` (date,
kernel, RAM, ICD list, Vulkan summary, binary sha256, CI run
id):

```bash
mkdir -p perf
# … the full block is PERF.md §1; run it exactly as written there …
```

Plus a free-disk snapshot — §3's full PNG set must exist on
disk all at once (PERF §5 measures the real budget; USER-PC's
prior expectation is roughly 2–6 GB, to be confirmed or
replaced by that measurement):

```bash
free -h; df -h . >> perf/env.txt
```

### 0.4 Adapter proof — the renderer line is the ground truth

The binary prints its adapter before every render. Scene modes
prefix it `adapter:`, `--film` mode prefixes it `renderer:` —
same line, same meaning (PERF §1):

```
adapter: <name> | backend: Vulkan | device type: <type> | driver: <driver> <info>
```

A valid leg requires: `<name>` names the AMD/RADV device,
`<type>` is **IntegratedGpu**, the driver names **radv** — and
it must **not** name llvmpipe / Lavapipe / device type `Cpu`.
Smoke-render one scene file and check the line (USER-PC §3):

```bash
./talerender-bin/talerender --scene-file scenes/character-v2.scene.json --out /tmp/scene.png
```

If the line names the software device, pin the RADV ICD and
re-check before running anything else (USER-PC §2):

```bash
vulkaninfo --summary        # look for the Vega 7 / RADV device
export VK_ICD_FILENAMES=$(ls /usr/share/vulkan/icd.d/radeon_icd*.json | head -n 1)
```

Copy the proven adapter line into `perf/env.txt` (PERF §1
requires it there; §2.4's report generator refuses a log
directory with no renderer line at all, and refuses — loudly —
a directory whose logs name two different renderers).

**Leave `TALERENDER_DOF_DEPTH_FORMAT` unset** (USER-PC §7):
the default Rgba8Unorm depth packing is the proven path; RADV
fp16 rounding is unverified, and verifying it is not part of
this visit.

---

## §1 Determinism — the E7 RADV leg

**Status going in:** the CI-lavapipe leg (E6) is discharged —
determinism dispatch run `43617370962` at `3a36f1f`, verdict
`DETERMINISM: PASS` (recorded in
[WAVE2-RECORD.md](../integration/WAVE2-RECORD.md)). The
contract's discharge rule (§4) is *one pack PASS per renderer
of record*; the PC RADV leg (E7) is the remaining one. This leg
is that run, following DETERMINISM-CONTRACT §7 verbatim.

```bash
cd ~/workspace/talerender
# Fresh output root — the pack refuses non-fresh dirs (a reused
# dir could "pass" by comparing a run against itself).
tools/determinism/determinism_check.sh \
    ~/talerender-bin/talerender ~/det-run-onoda
echo "pack exit: $?"     # 0 = PASS, 1 = FAIL, 2 = refusal (no verdict)

cat ~/det-run-onoda/determinism-results.txt
```

What the pack does (contract §4): renders film frames
**[100, 130)** at 1080×1920 four ways — A: one invocation,
default cache (the reference bytes); B: [100, 115) then
[115, 130) into the **same** directory (chunk boundary +
resume purity); C: [100, 130) at `TALERENDER_PAINT_CACHE=24`
(cache-cap pixel-neutrality) — then sha256-compares every
frame, A vs B and A vs C. 90 frames rendered in total.

**Checks before trusting the result** (contract §7):

- The results file's `renderer (run A):` line must name RADV
  (`device type: IntegratedGpu`, `driver: radv`). If it names
  llvmpipe, pin the ICD (§0.4) and re-run into a **fresh**
  output root.
- Runs A/B use the driver's **default paint-cache cap by
  design** — that is part of what is being tested. **RAM flag:**
  one painted canvas is ~29 MB and the default cap's worst
  case is ≈4.6 GB on this ~6.7 GiB machine (PERF §4). The pack
  renders only 30-frame ranges, but watch the run; do **not**
  pre-shrink the cap — a PASS at a shrunken cap is not the E7
  verdict. Run C is the tight-cap run.
- Exit 2 is a **refusal**, not a FAIL: read its `REFUSAL:`
  line, fix the named cause, re-run into a fresh root. A
  refusal is *no evidence*, never a soft pass.
- A FAIL names every differing frame. Keep the whole
  `~/det-run-onoda/` tree (results file, `logs/`, and the
  `A-single/` / `B-chunked/` / `C-cache24/` frame dirs) until
  the result is recorded — the kept frames plus
  `tools/ab/measure_ab.py` are the diagnostic path (contract
  §8). Never answer a FAIL by widening the comparison.

**Duration:** no PC timing exists — that is the point of the
visit. The only grounded anchor is a lavapipe bound: the
Wave-1 full film rendered on CI as 10 dispatch jobs of ~181
frames, each inside that workflow family's 120-minute limit
(BENCH-W17 §1). 90 frames is a fraction of one such job, on a
*software* renderer; expect the RADV leg to be far inside it,
but treat that as orientation, not a promise.

---

### Optional support step — RADV byte-probes (W78 G4)

**Optional — RADV byte-probe support (does NOT discharge
anything).** The pack's window [100, 130) never touches the
old 160-entry cache-cap boundary where the quantised-key
defect measured pre-fix (contract E2). While the proven
binary is provisioned, the E4 probe ranges
(`tools/ci/byte_probes.py` defaults: 195–204, 795–814,
1195–1204) can be re-rendered twice on RADV and byte-compared
— supporting evidence on this renderer, in the E4 class:

```bash
export TALERENDER_BIN=~/talerender-bin/talerender
cd ~/workspace/talerender
for dir in probe-a probe-b; do
  for r in "195 205" "795 815" "1195 1205"; do
    set -- $r
    "$TALERENDER_BIN" --film film --out $dir --from $1 --to $2
  done
done
python3 tools/ci/byte_probes.py probe-a probe-b
```

Both dirs fresh, default cache cap, same binary, back to
back. Expected shape: `BYTE PROBES: PASS (40/40 frames
identical across 3 range(s))`. Any FAIL here alongside a
pack PASS is still a headline — send both outputs; per
contract §4 the pack verdict stands as the E7 record and
the probe result opens a defect thread, it does not amend
the verdict either way.

### If the verdict is not PASS (W78 G2)

- **REFUSAL (exit 2): stop the session here.** Fix the named
  cause and re-run §1 into a fresh output root before any
  other leg. A refusal is *no evidence*; the remaining legs
  all cite this leg's binary/adapter proof chain, and the
  re-run costs minutes — never defer E7 past §3.
- **FAIL (exit 1): preserve, send, then decide.** Freeze the
  whole `~/det-run-onoda/` tree (do not re-run into it, do
  not delete from it), complete §5.1 with the G1 additions,
  and send `determinism-results.txt` + logs before continuing.
  §2 (bench) may proceed — its tables claim timings, not
  determinism — and §4's cross-vendor PSNR bands remain
  meaningful. **§3 proceeds only with the defect flagged in
  its send-back note:** the full render's resume/chunking
  story rests on the same purity the pack just failed, so
  its frames are evidence *about* the defect, not a
  production claim. Do not attempt any fix on the PC; a FAIL
  is a defect report (contract §8), and the kept frame dirs
  + `tools/ab/measure_ab.py` are its diagnostic path.
- **PASS:** proceed; §5.2's discharge consequence applies.

## §2 Bench legs — the PERF protocol, run locally

### 2.0 Reconciliation: what runs where

Two different things are both called "bench" in this repo;
they are never merged:

| | `bench.yml` (W17) | This leg (the PC run) |
|---|---|---|
| Where | GitHub Actions, dispatch-only | **locally on this PC** |
| Renderer | CI lavapipe — its job summary and report banner say so verbatim: *"SOFTWARE RENDERER — INDICATIVE ONLY… must never be quoted as such"* | RADV (proven per §0.4) |
| Shape | fixed: frames 0..100, two legs — default cache + `TALERENDER_PAINT_CACHE=24` — each `/usr/bin/time -v`-wrapped into a fresh dir | PERF.md §§1–5 as written: stills, the mid-film chunk, the two telemetry cache configurations, disk |
| Purpose | harness checks only — proves the measurement pipeline runs | **the** performance numbers PERF.md's tables wait for |

The bridge is `tools/perf/bench_report.py`: it consumes
*either* a CI `bench-<run_id>` artifact *or* a PC `perf/`
directory and emits PERF-shaped tables — and it enforces the
separation mechanically (no renderer line → refusal, exit 2;
mixed renderers in one directory → refusal naming each). So:
**on the PC you run PERF.md, then the report generator; the
lead fills PERF.md's tables from the generated report.** You
never dispatch `bench.yml` as part of this visit, and its
lavapipe numbers never enter a PC table.

### 2.1 The legs, in PERF's order

Run PERF.md §§2–5 exactly as written — commands, fresh-directory
rule, and all. In outline, so the sitting can be planned:

1. **§2 stills** — ab1 (`--scene ab1`) and S01
   (`--scene-file scenes/film/s01.scene.json`), five runs each,
   `/usr/bin/time -f "%e %M"` wrapped, logs + `.time` files into
   `perf/`. Includes the byte-identity check across each set of
   five PNGs (`md5sum`) — determinism is part of what is being
   confirmed.
2. **§3 film chunk** — frames **1027..1117** (first 90 of S08,
   mid-film, representative paint-cache state), cold then warm,
   each into a **fresh** `--out` directory. Optional second
   chunk **784..874** (S06, fog + fire — plausibly the
   heaviest) by the same procedure.
3. **§4 memory** — film telemetry under both cache
   configurations, `/usr/bin/time -v` wrapped:
   - **A. default cap (160):**
     `./talerender-bin/talerender --film-telemetry film`
   - **B. tight cap:**
     `TALERENDER_PAINT_CACHE=24 ./talerender-bin/talerender --film-telemetry film`
   
   (full command lines with their `.log`/`.err`/`.time` files
   are PERF §4 verbatim.) A and B must reach the **same verdict
   line**; different verdicts at different caps are a
   determinism defect — report as such, with both logs.
4. **§5 disk** — the size distribution over §3's warm-chunk
   PNGs (script in PERF §5) → the full-film disk budget.
5. **§6 Chrome comparison (optional, manual)** — the same
   90-frame window (t = 34.23…37.20 s) through the skill's
   patched Chrome capture, timed with one
   `/usr/bin/time -f "%e %M" -o perf/chrome.time` wrap around
   the capture command only (BENCH-W17 §4). Two cautions travel
   with this row wherever it is quoted (PERF §6): the native
   side writes clean PNG masters, the Chrome side's masters
   carry the legacy q0.93 JPEG generation, and neither side
   includes finishing. PERF §6 also records a prior figure of
   ~0.15 s/frame for the patched route on this GPU — labelled
   there as prior context to confirm or refute, **not a
   result**; this leg is what confirms or refutes it.

**RAM flag (§4A is the sharp edge):** telemetry at the default
cap touches every character state in the film at 0.25 s
sampling; the cache's worst case (≈4.6 GB) plus renderer and
system on ~6.7 GiB is a genuine OOM risk — PERF §4 says so and
§7 gives the failure protocol (exit 137, `Killed`, dmesg
excerpt; re-run at cap 24 and record that the mitigation was
required — that fact is part of the result). Run §4 on an
otherwise idle machine, and run it *before* §3 so a memory
finding is not discovered mid-render.

**One dated note on exit codes:** PERF §4/§7/§9 describe
`--film-telemetry` as always exiting 0. The current binary's
`--help` documents the newer contract — **0** success, **1**
on `CHECK FAILED`, **3** when the gate could not run (CLI
errors are 2). Record **both** the printed verdict line and
`echo $?` for every telemetry run in this visit; the printed
line stays authoritative, and any line/code disagreement is
itself a finding to send back.

### 2.2 Durations

Unknown until measured — the protocol *is* the measurement.
Plan the sitting from §3's own arithmetic once the warm chunk
exists (`s_per_frame = elapsed / 90`; `full_film_hours =
s_per_frame × 1811 / 3600`) and quote a full-film figure only
that way, or as the range between the S08 and S06
extrapolations if both chunks were measured (PERF §3).

### 2.3 Where the outputs go

Everything lands in `perf/` at the repo root, under PERF's own
filenames (`env.txt`, `ab1-<n>.log/.time/.png`,
`s01-<n>.log/.time/.png`, `chunk-cold.*`, `chunk-warm.*`,
`chunk-s06-*`, `telemetry-default.*`, `telemetry-24.*`,
optional `chrome.time`). Then:

```bash
python3 tools/perf/bench_report.py perf --out perf/REPORT.md
```

`perf/REPORT.md` is the filled §2–§6 tables with every table
headed by its verbatim renderer line — that file, plus the raw
logs, is what the lead works from to fill PERF.md. PERF §8's
summary prose stays a human fill-in; the generator
deliberately does not fabricate it.

---

### 2.4 Cache-cap neutrality support (W78 G5)

The §5 RADV bench legs render frames 0..100 twice (default
cap, then `TALERENDER_PAINT_CACHE=24`) into two fresh dirs —
the same shape as pack section C over a 100-frame window.
While both dirs exist, one command turns them into RADV
cache-cap-neutrality *support* evidence at zero extra
renders (cross-reference: the E7 verdict itself is W36 §1's
pack; this is not it):

```bash
python3 tools/ci/byte_probes.py --ranges 0-99 \
    perf-radv/frames-default perf-radv/frames-cache24
```

Record the output beside `perf-radv/REPORT.md`. A FAIL here
with a pack PASS is reported, not reconciled on the spot.
---

## §3 Full film render + telemetry verdict capture

The production render (USER-PC §5), plus the gate verdict
captured on this machine alongside it.

### 3.1 Telemetry verdict first (CPU-only, gates the render)

No point rendering 1,811 frames over data the gate rejects.
USER-PC §4's command, with the exit code captured per §2.1's
dated note:

```bash
./talerender-bin/talerender --film-telemetry film \
    > perf/film-telemetry-radv.log 2> perf/film-telemetry-radv.err
echo "exit: $?"
tail -n 1 perf/film-telemetry-radv.log
```

Expected line (the verdict is the line; the counts in a FAILED
line are the report):

```
FILM TELEMETRY: CHECK OK
```

Telemetry is CPU-only — the verdict does not depend on the
renderer, and it must agree with the CI-side verdict for the
same data (currently `CHECK OK` at the re-chunked head). A
different verdict here is a finding: send the full stdout
(PERF §7).

### 3.2 The render

```bash
./talerender-bin/talerender --film film --out radv-frames/ \
    > perf/film-full-radv.log 2> perf/film-full-radv.err
echo "exit: $?"
```

- 1,811 frames, 1080×1920 @ 30 fps, `radv-frames/frame_00000.png`
  … `frame_01810.png` (frame *n* is t = n/30 s). Progress
  prints every 25 frames; the final line reads
  `film render complete: 1811 rendered, 0 skipped (resume), range 0..1811`.
- **Resume is automatic** — existing frame files are skipped,
  and frame(t) is a pure function, so chunking
  (`--from N --to M`) and interruption change nothing about
  the bytes. If the §2.2 extrapolation says the film will not
  fit the sitting, render it in `--from/--to` chunks across
  sittings into the same directory.
- **RAM flag:** at the default cap (160) the paint cache's
  worst case is ≈4.6 GB (PERF §4 math: ~29 MB × 160) on a
  ~6.7 GiB machine. USER-PC §7's guidance: prefer
  `TALERENDER_PAINT_CACHE=24` (≈700 MB bound) for long renders
  if the machine is doing anything else. Either cap is safe
  for §4's purposes — the cap is pixel-neutral by construction
  and by pack section C (§1) — but say which cap you used in
  the log's send-back note, and if an OOM kill happens (exit
  137) follow PERF §7: dmesg excerpt, re-run at 24, record
  that the mitigation was required.
- **Corrupt-frame trap (USER-PC §8):** a PNG being written
  when a process dies can exist but be truncated, and resume
  skips by existence. After any hard kill, delete the
  highest-numbered frame file before re-running.
- Disk: the full set must coexist — the §2 §5 measurement is
  the real budget (prior expectation ≈2–6 GB).

**Duration:** not estimable before §2 — use the §3-chunk
extrapolation from *this* visit (`s_per_frame × 1811 / 3600`
hours), with PERF §3's two stated distortions (segment starts
under-weighted; shot costs differ, S06 plausibly heaviest).
This leg is the session's long pole; everything else is
minutes-to-short next to it, and it runs unattended.

### 3.3 Optional closing step — finish (USER-PC §6)

Assembles the frames into the viewable proof with the
project's own mix:

```bash
film/finish.sh radv-frames/ ~/faceless-shorts/the-soldier-who-didnt-know/data/mix.wav onoda-native-radv.mp4
ffprobe onoda-native-radv.mp4   # 1080x1920, 30 fps, h264 + AAC, ≈60.46 s
```

(x264 slow CRF 18, BT.709 tags, mix muxed unchanged as AAC
192k, faststart; loudness should measure ≈ −14.0 LUFS — the
project's master, not a remix.) Optional for the evidence
package; the frames + logs are the evidence. The MP4 is the
human-checkable artifact.

---

## §4 Golden verification — verify, never pin

### 4.1 The distinction, stated plainly

**This session verifies goldens. It does not pin them.** A pin
is a deliberate human act — `make_golden.py --pin` records the
hash of *every* reference file on disk, which is why
[GOLDEN-RESTAGE-W03.md](../integration/GOLDEN-RESTAGE-W03.md)
§5 prescribes the film first-pins and the ab1 re-pin as **two
separate acts, two separate commits**, with diff-scope checks.
Nothing in this visit runs `--pin`, stages candidate files
onto the canonical paths, or edits `golden-hashes.json`. If
the RADV frames measured here are ever pinned as the RADV
per-vendor goldens, that is a later, separate, human decision
— this visit produces the evidence for it, nothing more.

### 4.2 What is compared

The RADV frames from §3 against the **f021991 candidate set**:
`docs/golden/frames-f021991/` — 14 segment mid frames rendered
on CI lavapipe at the Wave-1 final head, staged as candidates
by W03 with provenance in that directory's `RENDERER.txt`
(`renderer: CI lavapipe`). Same binary lineage, same data
(subject to §0.1's recorded head), different renderer — which
is exactly the comparison the determinism contract governs.

| segment | mid frame | candidate file |
|---|---|---|
| s01 | 89 | `docs/golden/frames-f021991/s01.png` |
| s02 | 285 | `docs/golden/frames-f021991/s02.png` |
| s03 | 435 | `docs/golden/frames-f021991/s03.png` |
| s04 | 560 | `docs/golden/frames-f021991/s04.png` |
| s05a | 689 | `docs/golden/frames-f021991/s05a.png` |
| s05b | 760 | `docs/golden/frames-f021991/s05b.png` |
| s06 | 848 | `docs/golden/frames-f021991/s06.png` |
| s07 | 969 | `docs/golden/frames-f021991/s07.png` |
| s08 | 1119 | `docs/golden/frames-f021991/s08.png` |
| s08b | 1231 | `docs/golden/frames-f021991/s08b.png` |
| s09 | 1306 | `docs/golden/frames-f021991/s09.png` |
| s10 | 1440 | `docs/golden/frames-f021991/s10.png` |
| s11 | 1593 | `docs/golden/frames-f021991/s11.png` |
| s12 | 1738 | `docs/golden/frames-f021991/s12.png` |

### 4.3 Procedure — hash first, then measure

```bash
mkdir -p radv-mids radv-golden
# extract the 14 mids from the §3 render (frame numbers per the table)
for spec in s01:89 s02:285 s03:435 s04:560 s05a:689 s05b:760 s06:848 \
            s07:969 s08:1119 s08b:1231 s09:1306 s10:1440 s11:1593 s12:1738; do
  seg=${spec%%:*}; n=${spec##*:}
  cp "$(printf 'radv-frames/frame_%05d.png' "$n")" "radv-mids/$seg.png"
done

# 1) byte-identity check — decisive where it happens, not required anywhere
sha256sum docs/golden/frames-f021991/*.png radv-mids/*.png

# 2) PSNR per mid, house tool (float64 MSE over all channels)
for seg in s01 s02 s03 s04 s05a s05b s06 s07 s08 s08b s09 s10 s11 s12; do
  python3 tools/ab/measure_ab.py \
      --pair "docs/golden/frames-f021991/$seg.png" "radv-mids/$seg.png" \
      --reference-label "f021991 candidate (CI lavapipe)" \
      --native-label "PC RADV" \
      --out "radv-golden/$seg-ab.txt"
done
```

Fill from the outputs — blank until measured:

| segment | frame | sha256 identical? | PSNR (dB) | mean \|Δ\| |
|---|---|---|---|---|
| s01 | 89 | | | |
| s02 | 285 | | | |
| s03 | 435 | | | |
| s04 | 560 | | | |
| s05a | 689 | | | |
| s05b | 760 | | | |
| s06 | 848 | | | |
| s07 | 969 | | | |
| s08 | 1119 | | | |
| s08b | 1231 | | | |
| s09 | 1306 | | | |
| s10 | 1440 | | | |
| s11 | 1593 | | | |
| s12 | 1738 | | | |

### 4.4 How to read the numbers (contract §2 discipline)

- **Cross-vendor bit-identity is NOT promised** — this is a
  physical limit, not a backlog item. The guarantee is *per
  renderer* (same data + binary + renderer ⇒ byte-identical);
  across renderers, comparisons are made in **PSNR bands —
  measured and reported, never gated as equality**, and each
  renderer of record keeps its own goldens. A mid that is not
  byte-identical is not, by itself, a failure of anything.
- The named mechanisms for any difference: the GRADE grain
  hash evaluates fp32 `sin()` on the GPU (its term is bounded
  by construction — amplitude ±0.006 of full scale, ≈49 dB
  PSNR ceiling *if it decorrelated entirely*), and
  floating-point rounding differs across shader JITs and fp16
  paths. (The grain's *input* is pinned — `post.grade.time`
  is 0 for the whole film, ruling R3 — so only its evaluation
  is vendor territory.)
- Interpretation, without inventing a threshold: differences
  consistent with those mechanisms are small and diffuse.
  A mid measuring in the teens of dB would be a different
  class of signal — that band is the *cross-engine* reference
  point on record (15.8–19.3 dB, headless-gl spike) — and
  should be reported as a defect candidate with the frame
  pair kept, not averaged away.
- Note the control case on the lavapipe side: film-s10 was
  byte-identical across the entire Wave-1 fix chain (W03 §2).
  No promise is made that it — or any mid — is byte-identical
  across renderers; the sha256 column records what happened.

---

## §5 Send-back — the exact package, and what each result means

### 5.1 Files

From the repo checkout on the PC:

| File(s) | From |
|---|---|
| `~/det-run-onoda/determinism-results.txt` | §1 — the citable E7 record (binary sha256 + `renderer:` line + both section lines + verdict) |
| `~/det-run-onoda/logs/*` | §1 — required if the verdict is FAIL or REFUSAL; keep the frame dirs (`A-single/`, `B-chunked/`, `C-cache24/`) until the result is recorded |
| `perf/env.txt` | §0.3 — environment block incl. the proven adapter line, HEAD, free/disk snapshot |
| `perf/REPORT.md` | §2.3 — the generated PERF-shaped tables |
| `perf/*.log`, `perf/*.time`, `perf/*.err` | §§2–3 — the full raw logs (PERF's return package), incl. `film-telemetry-radv.log/.err` and `film-full-radv.log/.err` |
| the §4 table (filled) + `radv-golden/*.txt` | §4.3 — per-mid PSNR outputs |
| `radv-mids/*.png` (14 files) | §4.3 — the RADV mids themselves, so any number can be re-measured |
| `onoda-native-radv.mp4` (optional) | §3.3 — the assembled proof, if the finish step ran |
| PERF §8 summary, filled by hand | §2 — prose over the generated tables; nobody fabricates this for you |

**On a FAIL verdict, add the frame evidence itself** — the
results file names every differing frame, but the diagnosis
happens from the sent package, and the frames are the
diagnosis. Before sending:

```bash
cd ~/det-run-onoda
(cd A-single  && sha256sum frame_*.png) > manifest-A-single.sha256
(cd B-chunked && sha256sum frame_*.png) > manifest-B-chunked.sha256
(cd C-cache24 && sha256sum frame_*.png) > manifest-C-cache24.sha256
```

Send the three manifests plus, for every frame the results
file names as differing, that frame's PNG from **each** of
the three dirs (same frame number, three files). Keep the
full `~/det-run-onoda/` tree on the PC until the orchestrator
confirms the E7 row is recorded. On PASS or REFUSAL the
manifests are optional (the results file is the record);
the tree is still kept until recording is confirmed.

On any failure, add PERF §7's send-back checklist for that
failure class (exact command, exit code, stdout + stderr,
frame range and last progress line; dmesg excerpt for OOM).

### 5.2 What each result means

- **E7 `DETERMINISM: PASS`** → with E6 already on record, the
  determinism contract's TBD-PROOF item is **discharged**
  (contract §4): both renderers of record have a formal pack
  PASS. The orchestrator appends the E7 row to the contract's
  §5 register (date, renderer, binary sha256, both section
  lines) — send the results file, that append is not yours to
  make. A **FAIL** is a defect report: it names every
  differing frame, and the kept frame dirs are the diagnostic
  path. A **refusal** means no evidence was produced; the
  `REFUSAL:` line names the cause.
- **The §2 tables** → the project's first RADV performance
  numbers. The lead fills PERF.md's blank tables from
  `perf/REPORT.md`; the §3-chunk extrapolation becomes the
  first honest full-film time estimate for this hardware, and
  §6 (if run) settles native-vs-Chrome capture cost on the
  same machine, with its artifact caveats attached.
- **The §4 memory legs** → A/B verdict disagreement is a
  determinism defect (send both logs as such). An OOM at the
  default cap that cap-24 mitigates is a *result*, recorded
  as such — it sizes the production default for this machine.
- **The §3 telemetry line** → the data gate's verdict on this
  machine; it must agree with CI's. The full render's
  completion line (`1811 rendered, 0 skipped`) is what makes
  §4's frames citable.
- **The §4 PSNR table** → establishes the RADV baseline
  against the lavapipe candidates under the contract's
  per-vendor-goldens discipline (§4.4). It authorises no pin
  by itself; it is the evidence a future pin decision — two
  separate human acts, per W03 §5 — would be made from.

---

## Appendix — failure protocol (pointers, not a copy)

PERF.md §7 owns failure handling; its classes, one line each:
**segfault** (exit 139 — on RADV a real defect signal; delete
the highest-numbered frame before any resume, send log +
range + reproducibility), **OOM kill** (exit 137 — dmesg
excerpt, re-run at `TALERENDER_PAINT_CACHE=24`, record the
mitigation), **no adapter** (ICD configuration — USER-PC §2),
**device failure** (`request_device failed` after a valid
adapter line — send the full log), **bad invocation** (exit 2
— `talerender --help`, correct, re-run, nothing to send),
**telemetry FAILED** (exit per §2.1's note; the printed line's
counts are the report), **short render** (timing invalid; keep
the log). Standing rule, PERF §7 verbatim in substance: *a
failure during measurement is a result — record the exit code,
keep all output, do not retry with different flags and report
only the success.*
