# PC-SESSION-W98 — the one RADV session, consolidated

**Seat:** TaleRender Wave 2, Batch 10 (final), W98. **What this
is:** the single document the operator runs in one sitting on
the user's PC. It sequences the four source documents into one
script — it does not replace any of them, and every command
below is copied from its source (source named per step) and
was verified against the tree at HEAD `9380b01` where
checkable:

- [PC-EVIDENCE-W36.md](../runbook/PC-EVIDENCE-W36.md) — the
  one-visit runbook (the spine; cited as **W36 §n**).
- [PC-DRYRUN-W68.md](PC-DRYRUN-W68.md) — the VM dry-run of that
  runbook; findings **F1–F6** (cited as **W68 Fn**).
- [E7-READINESS-W78.md](E7-READINESS-W78.md) — the E7 audit;
  its gap texts **G1/G2/G4/G5** were paste-ins at audit time.
  **Status check by this seat: G1, G2, G4 and G5 have all
  LANDED in W36** (commit `108f33f`, "W78 runbook paste-ins"):
  G1 is in W36 §5.1, G2 is W36 §1's final subsection, G4 is
  W36 §1's optional support step, G5 is W36 §2.4. G3 was never
  a runbook edit (it is the orchestrator's contract-register
  append — still owed, see §11). This pack therefore cites
  the landed runbook text, not the audit's drafts.
- [BENCH-FIRST-W64.md](BENCH-FIRST-W64.md) **§5** — the bench
  addendum (cited as **W64 §5**).
- [VRAM-IMPL-W84.md](VRAM-IMPL-W84.md) — the VRAM staging
  pack; its sysfs probe spec is **§5.4** (cited as **W84 §n**).

**The machine:** AMD Ryzen 5 PRO 5650G (6C/12T), Radeon Vega 7
iGPU (gfx90c), RADV Vulkan driver, Ubuntu, **~6.7 GiB usable
RAM** — the session's binding constraint.

**The honesty rule (W36, carried verbatim in substance):**
every native number this project currently has was measured
on CI's **lavapipe** (software Vulkan) or on a GPU-less VM.
They are correctness evidence, not hardware evidence. **No
RADV number exists yet.** This session exists to change that.
No result is promised anywhere below; every leg's output names
its renderer, and a leg whose logs do not prove RADV ran is a
configuration result, not evidence.

**Order note — one forced deviation from W36's session map,
named here, not hidden.** W36 orders: setup → E7 → bench →
full film → golden verification → send-back, because its §4
verifies frames that only its §3 render produces. This pack
keeps every dependency but splits the golden leg in two:
**pack verify** (CPU-only precondition — needs no RADV frames)
runs as Step 5, in the slot this pack's commission assigns to
the golden leg; the **PSNR measurement** (needs the film
render's frames) runs as Step 7b, immediately after the render
and before finish. Nothing else is reordered.

---

## 1. Pre-flight checklist

Work through this list before Step 1. Items are checkable in
minutes; F1's item is the one that would otherwise burn the
session's first commands.

- [ ] **Machine idle.** Steps 3–4 are timing legs; nothing
  else heavy may run during them (W36 §2: "machine must be
  idle"). Close browsers/encoders before starting.
- [ ] **GitHub CLI authenticated** against an account that can
  read `turjaun985123/talerender`:
  `gh auth status`. Provisioning (Step 1) downloads a CI
  artifact; a logged-out `gh` fails there, not earlier.
- [ ] **Checkout present** at `~/workspace/talerender`
  (cloned or pulled in Step 1); you will record its HEAD and
  cite every result against it (W36 §0.1 — determinism's
  "same data" means *the same repo commit*).
- [ ] **`vulkaninfo` present (W68 F4).** Neither W36 nor
  USER-PC lists `vulkan-tools` as a prerequisite, but W36
  §0.4's wrong-adapter fallback starts with
  `vulkaninfo --summary`, and PERF §1's env block embeds it —
  with its stderr discarded, so its absence is invisible
  (empty `== vulkan ==` section, exit 0). Check now:
  `command -v vulkaninfo || sudo apt install vulkan-tools`.
  The adapter proof itself does not depend on it (the binary
  prints the line); only the fallback branch does.
- [ ] **F1 RESOLVED — binary path (W68 F1, BLOCKING).** The
  contradiction, stated plainly: W36 §0.2 provisions the
  binary to the *home* directory (`-D ~/talerender-bin`),
  while W36 §§0.4/2.1/3.1/3.2, PERF.md §§1–4 and W64 §5
  invoke it repo-relatively (`./talerender-bin/talerender`)
  from the repo root — where no `talerender-bin/` exists.
  W68 reproduced the failure verbatim: exit 127,
  `No such file or directory`, on the §3.1 gate command
  itself. **Resolution adopted by this pack (source: W68
  F1's analysis): the session's one canonical binary path is
  the absolute `~/talerender-bin/talerender`.** It is the
  path §0.2 already provisions to, the path §1's determinism
  pack already takes as its argument, and the path the
  project's PC convention names (`TALERENDER_BIN`, W46) —
  and being absolute, it resolves from any directory. To
  make the repo-relative command blocks (PERF, W64 §5) work
  *verbatim, unedited*, add one symlink from the repo root
  once, in Step 1:

  ```bash
  cd ~/workspace/talerender
  ln -sfn ~/talerender-bin talerender-bin   # ./talerender-bin/talerender now resolves
  ```

  **Flag:** the permanent fix is Batch 10's W96 pack,
  [DRYRUN-FIX-W96.md](DRYRUN-FIX-W96.md): paste-ready patches
  for the lead (13 invocation lines across 3 files, zero
  provisioning steps changed), **not yet landed** in the
  runbook files at this pack's HEAD. Its canonical-side
  decision is the same as this pack's — `~/talerender-bin`
  wins, selected via `TALERENDER_BIN` — so the two agree on
  the path and differ only in mechanism (its patches rewrite
  the invocations; this pack's symlink makes the unedited
  blocks resolve). If the lead lands W96 before the session,
  the runbook is self-consistent and the symlink is
  unnecessary; it is harmless either way. Do not edit the
  runbook yourself mid-session.
- [ ] **`TALERENDER_DOF_DEPTH_FORMAT` unset** (W36 §0.4):
  the default Rgba8Unorm depth packing is the proven path;
  RADV fp16 rounding is unverified and verifying it is not
  part of this visit. `echo "$TALERENDER_DOF_DEPTH_FORMAT"`
  must print empty.
- [ ] **Fresh output roots.** `~/det-run-onoda` must not
  already contain frames (the determinism pack refuses
  non-fresh roots — W68 verified the refusal fires before
  any render). `perf/`, `perf-radv/`, `radv-mids/`,
  `radv-golden/` start absent or empty. `radv-frames/` is
  the one resumable directory (Step 7) — absent for a first
  run.
- [ ] **Disk headroom.** The full PNG set must exist on disk
  all at once. Orientation only: USER-PC's prior expectation
  is ~2–6 GB for 1,811 frames (W36 §0.3), and W64 §1.2
  measured S01 frames plateauing at ~3.0 MB on lavapipe —
  the same order. PERF §5 (Step 3) replaces the expectation
  with this machine's measurement. Step 1 appends
  `df -h .` to `perf/env.txt`; check the free figure before
  Step 7, not during it.
- [ ] **`radeontop` (optional).** W84 §6.5's PC acceptance
  cross-checks the sysfs counters against `radeontop`:
  `command -v radeontop`. Absent is fine — Step 6 degrades
  to the raw counters alone, recorded as such.

---

## 2. Step 1 — checkout, binary, adapter proof

Source: W36 §0, with the F5 clause (W68) applied to §0.2 and
the §1 symlink from §1 above. **Produces:** a checkout at a
recorded HEAD, a provisioned binary with recorded sha256,
`perf/env.txt`, and a proven RADV adapter line. **This step
gates everything: no later step's output means anything if
the renderer line here names the software device.**

### 2.1 Checkout (W36 §0.1, verbatim)

```bash
# clone if absent, pull if present — either way, end at a known head
git clone https://github.com/turjaun985123/talerender.git ~/workspace/talerender
#   — or, in an existing checkout:  git pull
cd ~/workspace/talerender
git rev-parse HEAD        # record this; every result below is cited against it
```

### 2.2 Binary — from CI, not a local build (W36 §0.2, verbatim + F5 clause)

The binary comes from a green CI run's `talerender-binary`
artifact; CI builds on `ubuntu-22.04` (glibc 2.35) so the
artifact runs on this PC as-is. No local cargo build is part
of the route. Use a run at or after the CLI release.

```bash
gh run list --repo turjaun985123/talerender --workflow CI --limit 5
gh run download <run-id> --repo turjaun985123/talerender \
    -n talerender-binary -D ~/talerender-bin
chmod +x ~/talerender-bin/talerender
~/talerender-bin/talerender --version    # sanity: 1.0.0 or later
sha256sum ~/talerender-bin/talerender    # record it (legs below do too)
```

**F5 clause (W68 F5, applied):** pick a **`completed /
success`** row from the list. The block as written in W36
never repeats that constraint, and the top rows are routinely
in-progress or cancelled (W68 observed, of the top 5: 1 in
progress, 2 cancelled, 2 success) — an in-progress run has no
artifact yet, and a cancelled run's artifact, if any, predates
its head's verdict. This pack's recommendation, labelled as
such: prefer the success run whose head is the checkout's
HEAD from §2.1, so binary and data cite one commit.

Then the F1 symlink (§1 of this pack):

```bash
cd ~/workspace/talerender
ln -sfn ~/talerender-bin talerender-bin
./talerender-bin/talerender --version    # must now resolve from the repo root
```

### 2.3 Environment capture (W36 §0.3)

Run PERF.md §1's block verbatim — it writes `perf/env.txt`
(date, kernel, RAM, ICD list, Vulkan summary, binary sha256,
CI run id). Its `sha256sum ./talerender-bin/talerender` line
is one of the F1-affected commands; the §2.2 symlink makes it
work as written.

```bash
mkdir -p perf
# … the full block is PERF.md §1; run it exactly as written there …
```

Plus W36 §0.3's free-disk snapshot:

```bash
free -h; df -h . >> perf/env.txt
```

### 2.4 Adapter proof — the renderer line is the ground truth (W36 §0.4, verbatim)

The binary prints its adapter before every render. Scene
modes prefix it `adapter:`, `--film` mode prefixes it
`renderer:` — same line, same meaning (PERF §1):

```
adapter: <name> | backend: Vulkan | device type: <type> | driver: <driver> <info>
```

A valid leg requires: `<name>` names the AMD/RADV device,
`<type>` is **IntegratedGpu**, the driver names **radv** —
and it must **not** name llvmpipe / Lavapipe / device type
`Cpu`. Smoke-render one scene file and check the line
(USER-PC §3):

```bash
./talerender-bin/talerender --scene-file scenes/character-v2.scene.json --out /tmp/scene.png
```

(W68 verified this command's shape and the line's format on
the VM — where it correctly produced the *reject* signature,
llvmpipe / `Cpu`. The RADV-positive branch first occurs
here, on the PC.)

**Stop rule:** if the line names the software device, stop.
Pin the RADV ICD and re-check before running anything else
(USER-PC §2, via W36 §0.4):

```bash
vulkaninfo --summary        # look for the Vega 7 / RADV device
export VK_ICD_FILENAMES=$(ls /usr/share/vulkan/icd.d/radeon_icd*.json | head -n 1)
```

Copy the proven adapter line into `perf/env.txt` (PERF §1
requires it there; the report generator in Step 3 refuses a
log directory with no renderer line at all, and refuses —
loudly — a directory whose logs name two different
renderers).

**Send back from this step:** `perf/env.txt` (part of the
§9 package).

---

## 3. Step 2 — determinism E7 pack (the formal RADV verdict)

Source: W36 §1, including its landed G2 subsection and G4
support step. **Status going in:** the CI-lavapipe leg (E6)
is discharged — determinism dispatch run `43617370962` at
`3a36f1f`, verdict `DETERMINISM: PASS` (recorded in
WAVE2-RECORD.md). The contract's discharge rule (§4) is *one
pack PASS per renderer of record*; this step is the
remaining one (E7).

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

**Expected verdict:** the results file ends with exactly one
line, `DETERMINISM: PASS` (exit 0) — or `DETERMINISM: FAIL
(n differ)` (exit 1). Exit 2 is a **refusal**, not a FAIL:
read its `REFUSAL:` line, fix the named cause, re-run into a
fresh root. A refusal is *no evidence*, never a soft pass.
(W68 exercised both refusal paths on the VM — missing binary
and non-fresh root both produce the documented `REFUSAL:`
line + exit 2, the latter before any render.)

**Checks before trusting the result** (contract §7, via
W36 §1):

- The results file's `renderer (run A):` line must name RADV
  (`device type: IntegratedGpu`, `driver: radv`). If it names
  llvmpipe, pin the ICD (§2.4) and re-run into a **fresh**
  output root.
- Runs A/B use the driver's **default paint-cache cap by
  design** — that is part of what is being tested. **RAM
  flag:** W36 quotes one painted canvas at ~29 MB and the
  default cap's worst case at ≈4.6 GB on this ~6.7 GiB
  machine (PERF §4). Watch the run; do **not** pre-shrink
  the cap — a PASS at a shrunken cap is not the E7 verdict.
  Run C is the tight-cap run. (See §10/F6 for the byte-budget
  correction to the 4.6 GB figure — it applies to the
  *telemetry* leg's observed behaviour; the pack's discipline
  is unchanged: default cap, watched.)
- A FAIL names every differing frame. Keep the whole
  `~/det-run-onoda/` tree until the result is recorded —
  see the stop rule.

**Stop rule — G2, as landed in W36 §1 (operative text):**

- **REFUSAL (exit 2): stop the session here.** Fix the named
  cause and re-run this step into a fresh output root before
  any other leg. The remaining legs all cite this step's
  binary/adapter proof chain, and the re-run costs minutes —
  never defer E7 past the film render.
- **FAIL (exit 1): preserve, send, then decide.** Freeze the
  whole `~/det-run-onoda/` tree (do not re-run into it, do
  not delete from it), complete §9 with the G1 additions
  (§9.2), and send `determinism-results.txt` + logs before
  continuing. Step 3 (bench/PERF) may proceed — its tables
  claim timings, not determinism — and Step 7b's cross-vendor
  PSNR bands remain meaningful. **The film render (Step 7)
  proceeds only with the defect flagged in its send-back
  note:** its resume/chunking story rests on the same purity
  the pack just failed, so its frames are evidence *about*
  the defect, not a production claim. Do not attempt any fix
  on the PC; a FAIL is a defect report (contract §8), and
  the kept frame dirs + `tools/ab/measure_ab.py` are its
  diagnostic path. Never answer a FAIL by widening the
  comparison.
- **PASS:** proceed; §11's discharge consequence applies.

**Optional support step — G4 RADV byte-probes (W36 §1;
does NOT discharge anything).** The pack's window [100, 130)
never touches the old 160-entry cache-cap boundary where the
quantised-key defect measured pre-fix (contract E2). While
the proven binary is provisioned, the E4 probe ranges
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
contract §4 the pack verdict stands as the E7 record and the
probe result opens a defect thread, it does not amend the
verdict either way.

**Send back from this step:** `~/det-run-onoda/determinism-results.txt`;
`~/det-run-onoda/logs/*` (required if FAIL or REFUSAL); the
G4 output if run. On FAIL, the G1 frame evidence (§9.2).

---

## 4. Step 3 — PERF tables (the PERF protocol, run locally)

Source: W36 §2 (which runs PERF.md §§1–5 locally). **Two
different things are both called "bench" in this repo; they
are never merged** (W36 §2.0): `bench.yml` is CI-lavapipe,
dispatch-only, harness checks only — **you never dispatch it
as part of this visit, and its numbers never enter a PC
table.** This step is PERF.md run on RADV; Step 4 is the
W64 §5 addendum that repeats the CI bench's *shape* on RADV
so the two can be compared side by side by a human.

Run PERF.md §§2–5 exactly as written — commands,
fresh-directory rule, and all. The legs, in PERF's order
(W36 §2.1):

1. **§2 stills** — ab1 (`--scene ab1`) and S01
   (`--scene-file scenes/film/s01.scene.json`), five runs
   each, `/usr/bin/time -f "%e %M"` wrapped, logs + `.time`
   files into `perf/`. Includes the byte-identity check
   across each set of five PNGs (`md5sum`) — determinism is
   part of what is being confirmed; a differing hash is a
   defect report, not a timing footnote. Run 1 is cold;
   the warm figure is the median of runs 2–5 (PERF §2).
2. **§3 film chunk** — frames **1027..1117** (first 90 of
   S08, mid-film, representative paint-cache state), cold
   then warm, each into a **fresh** `--out` directory:

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

   **Validity (PERF §3):** the log ends with the completion
   line — quoted there as `film render complete: 90
   rendered, 0 skipped (resume), range 1027..1117`. **F2
   applies (W68 F2): match it as a prefix.** The real format
   (film.rs, quoted by W68 from `src/film.rs:3467` at its
   HEAD) appends two `·`-separated fields — elapsed/RSS and
   the paint-cache statistics. Anything else in the counts
   (fewer rendered, any skipped) invalidates the timing —
   say so and keep the log. Keep the *whole* line: Step 4
   needs the completion lines' cache statistics and RSS.
   **This chunk is the session's only sanctioned source of
   a full-film time estimate** (PERF §3 arithmetic, §10).
3. **§4 memory** — film telemetry under both cache
   configurations, `/usr/bin/time -v` wrapped (PERF §4,
   verbatim):

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
   grep "Maximum resident" perf/telemetry-default.time perf/telemetry-24.time
   ```

   A and B must reach the **same verdict line** —
   `FILM TELEMETRY: CHECK OK` (W68 observed exactly this
   line, as the last line of stdout, exit 0, under both
   configurations on the VM). Different verdicts at
   different caps are a determinism defect — report as such,
   with both logs. An advisory `CAPTIONS: estimate-grade …`
   line above the verdict is expected at this head (W68
   observed it); it is not part of the verdict and does not
   disturb it. **Exit codes (W36 §2.1's dated note):** PERF
   §4/§7/§9 describe `--film-telemetry` as always exiting 0;
   the current binary's `--help` documents the newer
   contract — **0** success, **1** on `CHECK FAILED`, **3**
   when the gate could not run (CLI errors are 2). Record
   **both** the printed verdict line and `echo $?` for every
   telemetry run in this visit; the printed line stays
   authoritative, and any line/code disagreement is itself a
   finding to send back. (Optional, non-gating: at HEAD
   `3223509` the binary also accepts `--film-telemetry film
   --tables`, printing shot/caption/cast tables after the
   verdict. The tables change no verdict and no exit code;
   if you run it, send the output as a bonus file — its
   diff-check against spec was still owed at this pack's
   HEAD, so treat the tables as unverified garnish, never
   as evidence.)
   **RAM flag (W36 §2.1, with the F6 correction):** W36
   calls §4A "the sharp edge" — telemetry at the default cap
   touches every character state in the film at 0.25 s
   sampling, against a quoted worst case of ≈4.6 GB. That
   math predates the paint cache's byte budget (see §10/F6):
   W68's default-cap telemetry run on the VM printed
   `peak_bytes=2162688000` against `budget_bytes=2147483648`
   with only 74 entries live — the 2 GiB byte budget, not
   the 160-entry cap, was the binding constraint *for the
   telemetry leg*. PERF §4 at HEAD already carries the
   budget correction in its own text. Run §4 on an otherwise
   idle machine, and run it *before* Step 7 so a memory
   finding is not discovered mid-render. If §4A dies with
   exit 137 (`Killed`), follow PERF §7: dmesg excerpt,
   re-run at cap 24, record that the mitigation was
   required — that fact is part of the result.
4. **§5 disk** — the size distribution over §3's warm-chunk
   PNGs (script in PERF §5) → the full-film disk budget.
   This measurement supersedes the §1 orientation figures.
5. **§6 Chrome comparison (optional, manual)** — the same
   90-frame window (t = 34.23…37.20 s) through the skill's
   patched Chrome capture, timed with one
   `/usr/bin/time -f "%e %M" -o perf/chrome.time` wrap around
   the capture command only (BENCH-W17 §4, via W36 §2.1).
   Two cautions travel with this row wherever it is quoted
   (PERF §6): the native side writes clean PNG masters, the
   Chrome side's masters carry the legacy q0.93 JPEG
   generation, and neither side includes finishing. PERF §6
   also records a prior figure of ~0.15 s/frame for the
   patched route on this GPU — labelled there as prior
   context to confirm or refute, **not a result**; this leg
   is what confirms or refutes it.

Then the report (W36 §2.3, verbatim):

```bash
python3 tools/perf/bench_report.py perf --out perf/REPORT.md
```

`perf/REPORT.md` is the filled §2–§6 tables with every table
headed by its verbatim renderer line — that file, plus the
raw logs, is what the lead works from to fill PERF.md. PERF
§8's summary prose stays a human fill-in; the generator
deliberately does not fabricate it. (W68 verified the
generator's refusal leg: a directory with no renderer line →
exit 2 naming the cause. A refusal here means a leg's log
lost its renderer line — find which, do not paper over it.)

**Stop rules for this step:** any log whose renderer line
names llvmpipe → that leg is a configuration result; stop,
pin the ICD (§2.4), redo the leg into fresh dirs (PERF §1).
A/B telemetry verdict disagreement → determinism defect,
send both logs. OOM → PERF §7 protocol (above). Standing
rule (PERF §7, via W36's appendix): *a failure during
measurement is a result — record the exit code, keep all
output, do not retry with different flags and report only
the success.*

**Send back from this step:** all of `perf/` — `env.txt`,
`REPORT.md`, every `*.log` / `*.time` / `*.err` (PERF's
return package, W36 §5.1), plus the hand-filled PERF §8
summary.

---

## 5. Step 4 — bench full-film window (W64 §5 addendum) + G5

Source: W64 §5 (verbatim block below) + W36 §2.4 (the landed
G5). **Purpose:** the bench.yml leg shapes repeated on RADV
so the PC numbers land commensurable with CI run
`43623743480` — same frames (0..100), same legs, same file
names, same tool. They do **not** replace any Step-3 leg
(PERF's chunk stays the only source of a full-film
extrapolation). Prerequisite per W64 §5: run from the repo
root, machine idle, binary provisioned, **after** PERF §1's
environment capture, and only if the binary's `renderer:`
line names the RADV device (device type not `Cpu`) — PERF
§1's stop rule. (The block's `./talerender-bin/talerender`
paths are F1-affected; the §2.2 symlink makes them work as
written. Its header calls that path "canonical … per W36
§1 / USER-PC" — that is the commands' side of the F1
contradiction speaking; §1 of this pack records the
resolution.)

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

**Validity gates (W64 §5, identical to the CI run's):** each
completion line must read `100 rendered, 0 skipped (resume),
range 0..100` — as a prefix, per F2 (the full line appends
elapsed/RSS + cache statistics); both `.exit` files must be
`0`; the `renderer:` line in both logs must name the same
RADV device (if it names llvmpipe, stop — that is a
configuration result, not a performance result, per PERF
§1). **Keep the completion lines' paint-cache statistics and
the `time -v` peak RSS for both legs together** (W64 §5):
their pairing computes RADV's resident cost per canvas
(Δ peak RSS ÷ Δ peak entries between the legs) — the number
that decides whether PERF §4's OOM concern is real on this
machine. No extra invocation; just don't strip the logs.

**G5 — cache-cap neutrality support (W36 §2.4, landed;
zero extra renders).** The two legs above are the same shape
as pack section C over a 100-frame window. While both dirs
exist:

```bash
python3 tools/ci/byte_probes.py --ranges 0-99 \
    perf-radv/frames-default perf-radv/frames-cache24
```

Record the output beside `perf-radv/REPORT.md`. Expected: a
`BYTE PROBES: PASS` line over the 0–99 range. A FAIL here
with a pack PASS is reported, not reconciled on the spot.
(Cross-reference, per the landed text: the E7 verdict itself
is Step 2's pack; this is not it.)

**Send back from this step:** the whole `perf-radv/`
directory (env.txt, both legs' `.log`/`.time`/`.exit`, both
`.sizes`, `REPORT.md`, the G5 output) alongside Step 3's
`perf/` package (W64 §5's return-package addition).

---

## 6. Step 5 — golden leg, part 1: pack verify (precondition)

Source: W36 §4's framing + W68 §4's exercised procedure.
**This session verifies goldens. It does not pin them**
(W36 §4.1). A pin is a deliberate human act — two separate
acts, two separate commits, per GOLDEN-RESTAGE-W03 §5.
Nothing in this session runs `--pin`, stages candidate files
onto the canonical paths, or edits `golden-hashes.json`.

The precondition the whole visit's golden talk rests on —
the pinned set intact on this checkout:

```bash
python3 tools/golden/make_golden.py
```

**Expected (W68 observed exactly this at `2755ff3`):**
`18 OK / 0 STALE / 0 MISSING of 18 golden items`, including
the 14 film segments, ab1-composer (pin `60c32c1a…`), dof,
stroke, and s28 (`ff68fad3…` — present, deliberately not
re-pinned; its disposition is a separate open decision, not
this session's business, §11).

**Stop rule:** any STALE or MISSING → the checkout is not
the tree the pins describe. Stop before Step 7, record the
verify output, and report it — do not re-pin, do not
"refresh" anything. (A wrong HEAD from §2.1 is the likely
cause; if `git rev-parse HEAD` is not the head the session
intended, that is the finding.)

The comparison set for part 2 is the **f021991 candidate
set**: `docs/golden/frames-f021991/` — 14 segment mid frames
rendered on CI lavapipe at the Wave-1 final head, provenance
in that directory's `RENDERER.txt` (`renderer: CI lavapipe`).
W68 verified the set intact at its paths, spot hashes
matching RENDERER.txt (s01 `1608ead9…`, s10 `59bdb4f2…`).

**Send back from this step:** the verify output (paste it
into the session's notes file, §9).

---

## 7. Step 6 — sysfs VRAM reading (numeric path: PC TO-VERIFY)

Source: W84 §5.4 (probe spec) + §6.5 (PC acceptance). **Read
this first — the state of the code, verified by this seat at
HEAD `9380b01`:** the VRAM ledger has **not landed** in
`src/` yet. A tree-wide grep finds `gpu-mem` only in the
parser (`tools/perf/vram_report.py`) — **the current binary
prints no `gpu-mem:` line**, and W84 is a staging pack for an
implementation the lead has not landed. So this step has two
tiers, and which one applies is decided by the provisioned
binary, not by preference:

**Tier 1 — only if the binary was built from a head where
the lead has landed W84's implementation** (decided by
whether the CI run §2.2 downloaded from post-dates that
landing; if unsure, Step 7's log tail settles it — the line
is either there or it isn't, and its absence in a Tier-2
binary is expected, not a failure). Then the film render's
log (Step 7) carries the line immediately after the
completion line:

```
gpu-mem: accounted_bytes=<u64> peak_accounted_bytes=<u64> textures=<u64> buffers=<u64> readback_bytes=<u64> sysfs_gtt_used_bytes=<u64|unavailable> sysfs_vram_used_bytes=<u64|unavailable>
```

(grammar pinned by the parser — W84 §5.3; do not expect any
other spelling.) Acceptance on this machine (W84 §6.5,
carrying W56 §6.5): sysfs fields **numeric** on RADV,
cross-checked against `radeontop` (pre-flight); `readback_bytes`
= **8,355,840** exactly; `accounted_bytes` ≥ **220,930,688**
with the excess equal to the ledger's class E + G subtotals
(a real film run exceeds the formula total — the parser
fixture's 41/18 counts are synthetic; do not "fix" anything
to match them). Telemetry prints **no** gpu-mem line by
design (W84 decision D1 — telemetry never creates a device;
a zero line would fabricate a measurement).

**Tier 2 — the expected tier at this pack's HEAD: capture
the counters directly.** W84 §5.4 specifies the probe's
paths; the commands below are this pack's assembly from that
spec (the probe iterates `card0..card8` under
`/sys/class/drm/` and reads the first card whose
`device/mem_info_gtt_used` exists). Run them now, at idle,
to establish that the numeric path exists on this machine:

```bash
ls /sys/class/drm/
CARD=$(for n in 0 1 2 3 4 5 6 7 8; do
  [ -f /sys/class/drm/card$n/device/mem_info_gtt_used ] && { echo card$n; break; }
done)
echo "card: $CARD"
cat /sys/class/drm/$CARD/device/mem_info_gtt_used
cat /sys/class/drm/$CARD/device/mem_info_vram_used
```

**Expected:** a `cardN` is found (this is the amdgpu device
— the Vega 7), and both files print bare integers (bytes).
Record all three outputs into the session notes. W68
verified the opposite environment: on this VM
`/sys/class/drm/` contains only `version` — no `cardN` at
all — so the `unavailable` path is the only locally/CI-
testable one, and the numeric path stays **TO-VERIFY until
this moment on the PC** (W84 §5.4).

**In-flight readings belong to Step 7** (a counter read at
idle proves the path exists, not the render's footprint):
while the film render runs, sample both files from a second
terminal (§8.2). Two interpretation rules, from W84 §5.4,
that travel with any number captured: the counters are
**device-wide** (all processes), and on the 5650G the VRAM
figure is a **carve-out of the same system RAM** — neither
is this process's share. And W84 §7.1: the paint cache's
bytes are *host* memory, never device; the gpu-mem ledger
never ingests them, and no one sums the two lines into one
number outside `vram_report.py`'s labelled reference sum.

**Stop rule:** no `cardN` with `mem_info_gtt_used` on this
PC → the numeric path does not exist as specified; record
the `ls` output and stop this step (only this step — it
blocks nothing else). Non-numeric or unreadable files →
same: record verbatim, proceed.

**Send back from this step:** the card name + both idle
readings (session notes), the Step-7 in-flight samples
(§8.2), and — Tier 1 only — the render log's `gpu-mem:`
line plus the `radeontop` cross-check note.

---

## 8. Step 7 — telemetry gate, full film render, finish

Source: W36 §3. This is the session's long pole; everything
else is minutes-to-short next to it, and it runs unattended
(W36 §3.2). If Step 2's verdict was FAIL, this step proceeds
only with the defect flagged in its send-back note (G2,
§3 of this pack).

### 8.1 Telemetry verdict first (CPU-only, gates the render)

No point rendering 1,811 frames over data the gate rejects.
USER-PC §4's command via W36 §3.1, with the exit code
captured per the §4 dated note in Step 3:

```bash
./talerender-bin/talerender --film-telemetry film \
    > perf/film-telemetry-radv.log 2> perf/film-telemetry-radv.err
echo "exit: $?"
tail -n 1 perf/film-telemetry-radv.log
```

Expected line (the verdict is the line; the counts in a
FAILED line are the report):

```
FILM TELEMETRY: CHECK OK
```

Telemetry is CPU-only — the verdict does not depend on the
renderer, and it must agree with the CI-side verdict for the
same data (currently `CHECK OK` at the re-chunked head).
**Stop rule:** a different verdict here is a finding — send
the full stdout (PERF §7) and do not render until it is
understood. (The `CAPTIONS: estimate-grade …` advisory above
the verdict is expected; see Step 3.)

### 8.2 The render (W36 §3.2, verbatim)

```bash
./talerender-bin/talerender --film film --out radv-frames/ \
    > perf/film-full-radv.log 2> perf/film-full-radv.err
echo "exit: $?"
```

- 1,811 frames, 1080×1920 @ 30 fps,
  `radv-frames/frame_00000.png` … `frame_01810.png` (frame
  *n* is t = n/30 s). Progress prints every 25 frames; the
  final line begins `film render complete: 1811 rendered, 0
  skipped (resume), range 0..1811` — **prefix, per F2**: two
  `·`-separated fields follow (elapsed/RSS, paint-cache
  statistics). Keep the whole line; Step 4's RSS pairing and
  §10's accounting read from these logs.
- **Tier-1 VRAM check:** if the Step-6 Tier-1 condition
  applied, the `gpu-mem:` line stands immediately after the
  completion line in `perf/film-full-radv.log`. Check it
  against §7's acceptance before anything else reads the
  log. Its absence in a Tier-2 binary is expected, not a
  failure.
- **Sysfs in-flight samples (Step 6, Tier 2):** while the
  render runs, from a second terminal, sample the counters
  a few times and note the wall clock beside each pair:

  ```bash
  date -Is; cat /sys/class/drm/$CARD/device/mem_info_gtt_used /sys/class/drm/$CARD/device/mem_info_vram_used
  ```

  (with `$CARD` from Step 6). Device-wide counters, system-
  RAM carve-out — §7's interpretation rules apply.
- **Resume is automatic** — existing frame files are
  skipped, and frame(t) is a pure function, so chunking
  (`--from N --to M`) and interruption change nothing about
  the bytes. If the §10 extrapolation says the film will not
  fit the sitting, render it in `--from/--to` chunks across
  sittings into the same directory.
- **RAM flag:** W36 quotes the default cap's (160) paint
  cache worst case at ≈4.6 GB (PERF §4 math: ~29 MB × 160)
  on a ~6.7 GiB machine, and USER-PC §7's guidance to prefer
  `TALERENDER_PAINT_CACHE=24` (≈700 MB bound) for long
  renders if the machine is doing anything else. The F6
  correction (§10) applies here too, with its limit stated
  honestly: the byte budget bound the *telemetry* leg at
  ~2.16 GB in W68's observation; the full *render's* peak
  has never been measured anywhere — this run is that
  measurement (PERF §4C reads the warm chunk's RSS; this
  log's completion line carries the render's own). Either
  cap is safe for §8.3's purposes — the cap is pixel-neutral
  by construction and by pack section C (Step 2) — but say
  which cap you used in the send-back note, and if an OOM
  kill happens (exit 137) follow PERF §7: dmesg excerpt,
  re-run at 24, record that the mitigation was required.
- **Corrupt-frame trap (USER-PC §8, via W36 §3.2):** a PNG
  being written when a process dies can exist but be
  truncated, and resume skips by existence. After any hard
  kill, delete the highest-numbered frame file before
  re-running.
- Disk: the full set must coexist — Step 3's §5 measurement
  is the real budget (prior expectation ≈2–6 GB).

**Expected verdict:** exit 0, the completion line as above,
and exactly 1,811 frame files. The completion line
(`1811 rendered, 0 skipped`) is what makes §8.3's frames
citable (W36 §5.2). On a resumed/chunked completion the
counts differ by construction — the send-back note must say
the render was chunked, and the union of the chunk logs is
the evidence.

### 8.3 Golden leg, part 2: PSNR measurement (W36 §4.2–4.4)

What is compared: this step's RADV frames against the
**f021991 candidate set** (Step 5). Same binary lineage,
same data (subject to §2.1's recorded head), different
renderer — exactly the comparison the determinism contract
governs. The 14 mids (W36 §4.2's table): s01:89, s02:285,
s03:435, s04:560, s05a:689, s05b:760, s06:848, s07:969,
s08:1119, s08b:1231, s09:1306, s10:1440, s11:1593, s12:1738.

```bash
mkdir -p radv-mids radv-golden
# extract the 14 mids from the §8.2 render (frame numbers per the table)
for spec in s01:89 s02:285 s03:435 s04:560 s05a:689 s05b:760 s06:848 \
            s07:969 s08:1119 s08b:1231 s09:1306 s10:1440 s11:1593 s12:1738; do
  seg=${spec%%:*}; n=${spec##*:}
  cp "$(printf 'radv-frames/frame_%05d.png' "$n")" "radv-mids/$seg.png"
done

# 1) byte-identity check — decisive where it happens, not required anywhere
sha256sum docs/golden/frames-f021991/*.png radv-mids/*.png
```

Step 2 of W36 §4.3 measures PSNR per mid — **with the F3
correction applied (W68 F3).** W36 writes
`--out "radv-golden/$seg-ab.txt"`, but `measure_ab.py`
defines `--out` as an output *directory*: run as written it
creates 14 directories named `*.txt`, each holding
`ab-report.json` (the PSNR lives in its `psnr_db` field),
`ab-report.md`, `pair_diff.png`, `pair_side-by-side.png` —
and W36 §5.1's send-back row ("`radv-golden/*.txt`") then
describes nothing the operator actually has. This pack's
corrected shape (the deviation is F3's, named here, not
silent):

```bash
# 2) PSNR per mid, house tool (float64 MSE over all channels)
for seg in s01 s02 s03 s04 s05a s05b s06 s07 s08 s08b s09 s10 s11 s12; do
  python3 tools/ab/measure_ab.py \
      --pair "docs/golden/frames-f021991/$seg.png" "radv-mids/$seg.png" \
      --reference-label "f021991 candidate (CI lavapipe)" \
      --native-label "PC RADV" \
      --out "radv-golden/$seg"
done
```

Fill W36 §4.3's table from the `ab-report.json` outputs
(segment | frame | sha256 identical? | PSNR dB | mean |Δ|) —
blank until measured; the table itself travels in the
send-back (§9).

**How to read the numbers (W36 §4.4, contract §2
discipline):**

- **Cross-vendor bit-identity is NOT promised** — a physical
  limit, not a backlog item. The guarantee is *per renderer*
  (same data + binary + renderer ⇒ byte-identical); across
  renderers, comparisons are made in **PSNR bands — measured
  and reported, never gated as equality**, and each renderer
  of record keeps its own goldens. A mid that is not
  byte-identical is not, by itself, a failure of anything.
- Named mechanisms for any difference: the GRADE grain hash
  evaluates fp32 `sin()` on the GPU (term bounded by
  construction — amplitude ±0.006 of full scale, ≈49 dB PSNR
  ceiling *if it decorrelated entirely*), and floating-point
  rounding differs across shader JITs and fp16 paths. (The
  grain's *input* is pinned — `post.grade.time` is 0 for the
  whole film, ruling R3 — so only its evaluation is vendor
  territory.)
- Differences consistent with those mechanisms are small and
  diffuse. A mid measuring in the **teens of dB** would be a
  different class of signal — that band is the *cross-engine*
  reference point on record (15.8–19.3 dB, headless-gl
  spike) — and is reported as a defect candidate with the
  frame pair kept, not averaged away.
- Control case on the lavapipe side: film-s10 was
  byte-identical across the entire Wave-1 fix chain (W03
  §2). No promise is made that it — or any mid — is
  byte-identical across renderers; the sha256 column records
  what happened.

**Send back from this step:** the filled table +
`radv-golden/*/ab-report.json` (+ `.md`) + `radv-mids/*.png`
(14 files), so any number can be re-measured (W36 §5.1, as
corrected by F3).

### 8.4 Optional closing step — finish (W36 §3.3 / USER-PC §6)

Assembles the frames into the viewable proof with the
project's own mix:

```bash
film/finish.sh radv-frames/ ~/faceless-shorts/the-soldier-who-didnt-know/data/mix.wav onoda-native-radv.mp4
ffprobe onoda-native-radv.mp4   # 1080x1920, 30 fps, h264 + AAC, ≈60.46 s
```

W68 verified: `finish.sh` exists, is executable, its usage
is `film/finish.sh FRAMES_DIR MIX_WAV OUT_MP4` (the argument
order above), the mix path exists, and the ffprobe
expectations match the existing proof this same script
produced from the lavapipe frames (h264 1080×1920@30 + AAC,
60.457 s). **One stale word in W36 §3.3, named:** it
describes the encode as "AAC 192k". `film/finish.sh` at HEAD
carries `-b:a 256k` (line 31) — the W76 micro-fix landed
(`d967931`) after W36 was written. The file at HEAD is the
truth: **x264 slow CRF 18, BT.709 tags, mix muxed unchanged
as AAC 256k, faststart; loudness should measure ≈ −14.0 LUFS
— the project's master, not a remix.** Optional for the
evidence package; the frames + logs are the evidence. The
MP4 is the human-checkable artifact.

---

## 9. Send-back — the exact package, and the FAIL-evidence package

Assemble from the repo checkout on the PC (W36 §5.1, with
the F3 correction, plus Steps 4–7's additions). Keep a
`session-notes.txt` as you go: HEAD, binary sha256 + CI run
id, the proven adapter line, every step's verdict line +
exit code, the Step-5 verify output, the Step-6 sysfs
readings, which paint-cache cap Step 7 used, and anything
that deviated.

### 9.1 The package

| File(s) | From |
|---|---|
| `session-notes.txt` | this pack, §9 preamble |
| `~/det-run-onoda/determinism-results.txt` | Step 2 — the citable E7 record (binary sha256 + `renderer:` line + both section lines + verdict) |
| `~/det-run-onoda/logs/*` | Step 2 — required if the verdict is FAIL or REFUSAL; keep the frame dirs (`A-single/`, `B-chunked/`, `C-cache24/`) until the result is recorded |
| G4 byte-probe output, if run | Step 2 (optional support) |
| `perf/env.txt` | Step 1 — environment block incl. the proven adapter line, HEAD, free/disk snapshot |
| `perf/REPORT.md` | Step 3 — the generated PERF-shaped tables |
| `perf/*.log`, `perf/*.time`, `perf/*.err` | Steps 3 + 7 — the full raw logs (PERF's return package), incl. `film-telemetry-radv.log/.err` and `film-full-radv.log/.err` |
| PERF §8 summary, filled by hand | Step 3 — prose over the generated tables; nobody fabricates this for you |
| the whole `perf-radv/` directory | Step 4 — env.txt, both legs' `.log`/`.time`/`.exit`, both `.sizes`, `REPORT.md`, the G5 output |
| Step-5 golden verify output | Step 5 — in `session-notes.txt` |
| Step-6 sysfs readings (idle + in-flight) | Steps 6 + 8.2 — in `session-notes.txt`; Tier 1: the `gpu-mem:` line is inside `film-full-radv.log`, plus the `radeontop` cross-check note |
| the §8.3 table (filled) + `radv-golden/*/ab-report.json` (+ `.md`) | Step 8.3 — per-mid PSNR outputs (F3-corrected paths) |
| `radv-mids/*.png` (14 files) | Step 8.3 — the RADV mids themselves, so any number can be re-measured |
| `onoda-native-radv.mp4` (optional) | Step 8.4 — the assembled proof, if the finish step ran |

### 9.2 On a FAIL verdict — the G1 additions (landed, W36 §5.1)

The results file names every differing frame, but the
diagnosis happens from the sent package, and the frames are
the diagnosis. Before sending:

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
manifests are optional (the results file is the record); the
tree is still kept until recording is confirmed.

### 9.3 On any other failure — PERF §7's checklist

W36's appendix points at PERF.md §7, which owns failure
handling; its classes, one line each (W36's summary):
**segfault** (exit 139 — on RADV a real defect signal;
delete the highest-numbered frame before any resume, send
log + range + reproducibility), **OOM kill** (exit 137 —
dmesg excerpt, re-run at `TALERENDER_PAINT_CACHE=24`, record
the mitigation), **no adapter** (ICD configuration — USER-PC
§2), **device failure** (`request_device failed` after a
valid adapter line — send the full log), **bad invocation**
(exit 2 — `talerender --help`, correct, re-run, nothing to
send), **telemetry FAILED** (exit per the §4 dated note; the
printed line's counts are the report), **short render**
(timing invalid; keep the log). For whichever class hit, the
evidence to capture is PERF §7's send-back checklist for
that class: **exact command, exit code, stdout + stderr,
frame range and last progress line; dmesg excerpt for OOM.**
That, plus the step's full log files (already in §9.1), is
what makes a failure diagnosable from the other side — a
failure reported without its log is a rumour.

---

## 10. Time and RAM expectations — stated honestly

**No RADV timing exists. Nothing below is a RADV prediction.**

**The one measured anchor, and why it does not transfer.**
CI bench run `43623743480` (lavapipe, frames 0..100,
`/usr/bin/time -v`, W64 §1.1): leg A (default cache) wall
185.32 s → **1.853 s/frame**, peak RSS 1,551,904 KiB
(1515.5 MiB); leg B (cap 24) 183.78 s → **1.838 s/frame**,
peak RSS 1,401,472 KiB (1368.6 MiB) — the tight cap saved
~147 MiB at no time cost *in that window*. Three confounds,
all stated by W64 itself: the renderer is software (llvmpipe
on a shared CI runner — a different machine *and* a different
renderer from the PC); the window is **S01-only** (frames
0..100 lie entirely inside segment S01 — no segment
transition, anomaly A1); and the figures are
startup-inclusive process wall, where PERF's PC chunk
protocol is the steady-state one. RADV on a real iGPU could
be much faster per frame; nothing measured bounds it from
either side. Anyone quoting "1.85 s/frame" as a PC
expectation is misquoting a lavapipe bound as hardware
evidence — the exact category error this session exists to
retire.

**Per-step orientation (labelled as orientation, from W36
and W78 §4):**

- **Step 2 (E7 pack):** 90 frames in 4 invocations — 1/20th
  of Step 7's 1,811 frames at the same resolution. *If* RADV
  merely matched the lavapipe pace, ~3 minutes against ~56
  for the film. W78's words: an orientation bound from a
  different renderer, **not** an estimate; W36 §1's "no PC
  timing exists" stands. What can be said: the pack's cost
  is a small fraction of the visit under any plausible RADV
  speed — which is why G2 makes a refusal an immediate
  fix-and-rerun.
- **Steps 3–4:** stills are seconds-to-short each on any
  renderer; the chunks are 90–100 frames each at the
  unknown RADV rate — plan them from the first chunk's own
  progress lines, not from this document.
- **Step 7 (the film):** the only sanctioned full-film
  figure is PERF §3's arithmetic on Step 3's *own warm
  chunk*: `s_per_frame = elapsed / 90`;
  `full_film_hours = s_per_frame × 1811 / 3600` — quoted
  that way, or as the range between the S08 and S06
  extrapolations if both chunks were measured (PERF §3),
  with its two stated distortions attached (segment starts
  under-weighted; shot costs differ, S06 plausibly
  heaviest). At the lavapipe anchor the film would be ~56
  minutes; treat that as the planning ceiling-shape, run
  the render unattended, and chunk across sittings if the
  measured rate says so (§8.2 — resume is byte-safe).

**RAM (~6.7 GiB usable — the binding constraint):**

- **F6 (W68 F6) — the ≈4.6 GB worst case is pre-budget
  math, for the telemetry leg at least.** W36 §§1/2.1/3.2
  quote the default paint cache's worst case as ≈4.6 GB
  (~29 MB × the 160-entry cap). That predates the paint
  cache's byte budget (Wave 2, `86f0c81`). W68's default-cap
  telemetry run printed its own accounting —
  `entries=74 … peak_bytes=2162688000 budget_bytes=2147483648`
  — i.e. the **2 GiB byte budget**, not the 160-entry cap,
  was the binding constraint, and the whole film's telemetry
  touched only 74 distinct canvases. PERF §4 at HEAD already
  states the budget in its own cache math
  (`min(cap × canvas, 2048 MiB)`); W36's prose has not
  caught up — this pack follows PERF/W68. Scoped honestly:
  the observation covers the *telemetry* leg; the full
  *render's* peak was not measured by W68 and is not known
  until Step 7 measures it.
- CI bench peak RSS (1,515.5 / 1,368.6 MiB, frames 0..100)
  is a *lavapipe process* figure; a RADV process additionally
  drives a real GPU stack whose allocations, on this APU,
  come out of the same physical RAM (Step 6's GTT counter is
  the device-side witness). RADV total footprint: unknown
  until measured — Steps 3–4 measure it in pieces, Step 7
  in full.
- The OOM protocol is PERF §7's (exit 137 → dmesg excerpt,
  re-run at cap 24, record the mitigation as part of the
  result). An OOM at the default cap that cap-24 mitigates
  is a *result*, recorded as such — it sizes the production
  default for this machine (W36 §5.2).

**Unknowns, listed rather than smoothed:** RADV s/frame
(every leg); RADV peak RSS (telemetry, chunk, full render);
whether the default cap completes the film inside 6.7 GiB;
the sysfs counters' in-flight values; every §8.3 PSNR. The
session *is* the measurement of each.

---

## 11. What this session closes — and what it does not

**Closes (on the results landing as specified):**

- **E7 — the determinism contract's RADV leg.** A pack PASS
  in Step 2, with E6 already on record, **discharges the
  contract's TBD-PROOF item** (contract §4: one pack PASS
  per renderer of record). The orchestrator appends the E7
  row to the contract's §5 register (date, renderer, binary
  sha256, both section lines) from the sent results file —
  that append is not the operator's to make (W36 §5.2). A
  FAIL is a defect report, not a discharge; a refusal is no
  evidence (contract §4/§8).
- **PERF.md's tables.** Steps 3–4 produce the project's
  first RADV performance numbers: `perf/REPORT.md` +
  `perf-radv/REPORT.md` are what the lead fills PERF.md's
  blank tables from; the §3-chunk extrapolation becomes the
  first honest full-film time estimate for this hardware;
  PERF §6 (if run) settles native-vs-Chrome capture cost on
  the same machine, with its artifact caveats attached.
- **The goal's RADV remainder** (goal `goal_b21124e71eb3`):
  every native number to date is CI/VM lavapipe — no PC
  render, no PERF numbers, no determinism RADV leg. This
  session is the whole of that remainder: E7, PERF, the W64
  bench repeat, the cross-vendor golden baseline (§8.3),
  the full-film RADV render + finish proof (§8), and the
  sysfs numeric path (Step 6 — discharged as *verified* if
  the counters read numeric and, Tier 1, the gpu-mem line
  parses with numeric fields; otherwise kept open honestly
  with the `ls`/readings as the record, per W84 §6.5).
- **The §8.3 PSNR table** establishes the RADV baseline
  against the lavapipe candidates under the contract's
  per-vendor-goldens discipline. It authorises no pin by
  itself; it is the evidence a future pin decision — two
  separate human acts, per W03 §5 — would be made from.

**Does not close:**

- **The fog-chart ≥45 dB PSNR leg.** The analytic ±2-code
  gate has passed on both sides (Chrome ≤1 code, W66;
  native CI frames ≤1 code at every panel, W86 — FINAL-REPORT
  / FOG-STATUS-W74 §5). The remaining leg — native-vs-Chrome
  full-chart PSNR ≥ 45 dB against the Chrome frame — is
  **reference-side**: the Chrome harness's rig folds 1/π into
  its physical light intensities, so its effective front-face
  factor is house/π = 0.222429… against the native house
  factor 0.6988266044899671, and an unadjusted comparison
  sags on the standard panels with nothing wrong with fog.
  Closing it needs the **×π-amended Chrome re-capture**
  (VERIFY.md's rig amendment: ambient 0.35π ≈ 1.0996,
  directional 0.65π ≈ 2.0420) — a Chrome-side capture task
  in the skill's environment, **not** a native fix and not
  any step of this session. No command in this pack touches
  it; do not improvise one at the PC.
- **The contract register's E6 row (G3).** The contract's
  §5 register still lists E6 as "Pending: workflow written;
  awaiting commit + first dispatch" and its header still
  says "not yet run" — stale since run `43617370962`
  discharged E6 at `3a36f1f` (W78 G3, verified first-hand by
  that seat; W36 §1 states the discharged truth). An
  operator reading the contract alone would think both legs
  are open. The append is the orchestrator's act (contract
  §6) — flagged here so the staleness is not mistaken for a
  session task.
- **Decisions that are not evidence:** skill adoption of
  the native backend (the user's call; the work copy exists,
  adoption ≠ default), the s28 golden disposition, the
  v1.0.1 release go, the caption-veto and locomotion
  sign-offs. This session informs none of them directly and
  must not be read as settling any.
- **The goal stays open** after this session on those
  decision items (and on the fog-chart leg above) — the
  session closes its *evidence* remainder, not the goal.

---

## 12. Contradictions register — everything the sources disagree on

Named, per this pack's laws; only F1 is resolved (in §1),
the rest are flagged with the side this pack follows.

| # | Contradiction | This pack follows |
|---|---|---|
| F1 | Binary path: provisioning (W36 §0.2, pack §1, `TALERENDER_BIN` convention) puts the binary at `~/talerender-bin/`; the run commands (W36 §§0.4/2.1/3.1/3.2, PERF §§1–4, W64 §5, USER-PC §§3–5) invoke `./talerender-bin/talerender` from the repo root. W64 §5 even calls the repo-relative path "canonical … per W36 §1 / USER-PC" — W36 §1 is in fact the one place the absolute path is used. Reproduced by W68: exit 127. | **Resolved in §1:** canonical = absolute `~/talerender-bin/talerender` + one repo-root symlink so verbatim blocks run. Permanent doc fix = W96's paste-ready pack (DRYRUN-FIX-W96.md — same canonical side), not yet landed by the lead. |
| F2 | Completion line: W36 §3.2 and PERF §3 quote it ending at `… range 1027..1117` / `0..1811`; the film.rs format appends two `·`-separated fields (elapsed/RSS, cache stats) after the range (W68 F2). | Match as a **prefix** everywhere; keep whole lines in logs. |
| F3 | `measure_ab.py --out`: W36 §4.3 passes `"radv-golden/$seg-ab.txt"` and §5.1 lists `radv-golden/*.txt`; the tool treats `--out` as a directory (W68 F3 — verified by running it). | Corrected shape in §8.3 (`--out "radv-golden/$seg"`; send `ab-report.json`/`.md`). |
| F4 | `vulkaninfo`: required by W36 §0.4's fallback and embedded in PERF §1's env block; listed as a prerequisite nowhere; its absence is silent (W68 F4). | Pre-flight check in §1. |
| F5 | W36 §0.2's prose says "a green CI run" but its command block never repeats the completed/success constraint (W68 F5). | Clause applied in §2.2. |
| F6 | RAM worst case: W36 §§1/2.1/3.2 quote ≈4.6 GB (pre-budget entry-cap math); PERF §4 at HEAD and W68's observed telemetry accounting (`peak_bytes` ≈ 2.16 GB vs the 2 GiB `budget_bytes`) show the byte budget binding — for the telemetry leg. The render leg's peak is unmeasured either way. | PERF/W68 for telemetry (§10); W36's caution retained for the render, labelled unmeasured. |
| — | Finish audio rate: W36 §3.3 says "AAC 192k"; `film/finish.sh` at HEAD encodes `-b:a 256k` (W76 micro-fix, `d967931`, after W36 was written). | The file at HEAD: **256k** (§8.4). |
| — | Telemetry exit codes: PERF §4/§7/§9 describe `--film-telemetry` as always exiting 0; the binary's `--help` documents 0/1/3 (W36 §2.1's dated note; W68 verified the help text matches the note). | The binary's contract; record line **and** code (Step 3). |
| — | Contract register: DETERMINISM-CONTRACT §5 lists E6 "Pending" and the header "not yet run"; the E6 dispatch in fact succeeded at `3a36f1f` (run `43617370962`, WAVE2-RECORD; W78 G3). | The record of the run (§11); register append is the orchestrator's. |
| — | Session order: this pack's golden leg is split (verify in Step 5, PSNR in §8.3) against W36's single §4-after-§3 placement. | The dependency, not the section number, is honoured (§0 order note). |
| — | G5's cross-reference: W36 §2.4's landed text speaks of "the §5 RADV bench legs" — that §5 is W64's addendum section, not a W36 section. | Read as W64 §5; the dirs it names are Step 4's (§5 of this pack). |

---

*Evidence basis (all read first-hand by this seat at HEAD
`9380b01`): PC-EVIDENCE-W36.md (701 lines, incl. the landed
G1/G2/G4/G5 text — commit `108f33f` verified in git log),
PC-DRYRUN-W68.md (F1–F6 + every verbatim observation cited),
E7-READINESS-W78.md (R1–R10, G1–G5, §4), BENCH-FIRST-W64.md
§§1/5 (run `43623743480` figures), VRAM-IMPL-W84.md (§§3,
5.3–5.4, 6, 7), PERF.md §§1–5 (command blocks + the budget
correction in §4), DETERMINISM-CONTRACT-W04.md §5 register,
FOG-STATUS-W74.md (×π caveat + closure note),
DRYRUN-FIX-W96.md (F1 canonical-side decision + patch-pack
status, landed as a file during this seat's run), and tree
checks: `grep` for `gpu-mem`/`mem_info` over `src/` (absent —
parser only), `film/finish.sh` line 31 (`-b:a 256k`), and
the existence of every tool/scene/film path the commands
name. No existing file was edited; nothing was committed.*
