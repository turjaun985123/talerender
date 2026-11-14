# PC-DRYRUN-W68 — PC evidence runbook, VM dry-run

**Seat:** TaleRender Wave 2, Batch 7, W68. **Subject:**
[PC-EVIDENCE-W36.md](../runbook/PC-EVIDENCE-W36.md) (609 lines) —
the one-visit RADV evidence runbook — executed here, on this
GPU-less VM, for every leg that is meaningful off the PC, so the
user's single PC sitting does not burn time on a broken step.

**Environment of this dry run:** repo checkout
`~/workspace/talerender`. The tree moved under the dry run —
HEAD went `4dd6530` → `2755ff3` (golden Act-1 corrective
re-pin) → `9380b01` (F2 migration landings) — because the
Wave-2 lead works in the same checkout; each leg below is
cited against the state it actually ran on (the golden verify
at `2755ff3`; the telemetry verdicts on the film data as of
the F2 Phase-D landing, unchanged by the later commits). On
the PC the head will be whatever §0.1 records. Renderer here is **llvmpipe** (software Vulkan); no
RADV exists on this machine, and **no number in this note is a
RADV number**. Scratch (binary, logs, renders) lives in
`~/workspace/.w68-dryrun/`, not `/tmp`.

**Verdict summary:** the runbook's machinery is sound — every
command shape, refusal, and verdict line that could be
exercised here behaved as documented, including the §3.1
headline: `FILM TELEMETRY: CHECK OK`, exit 0, under both
paint-cache configurations. **One blocking defect**
(F1: the binary path in the render/telemetry commands does not
resolve from the repo root the runbook itself puts you in) and
five lesser findings (F2–F6). F1 costs the PC operator one
failed command and a confused minute if unaddressed; it should
be fixed in the runbook (and in USER-PC.md, which has the same
defect) before the visit.

---

## §0.2 Binary provisioning — RUN, works (with F5 noted)

Followed the runbook's route exactly: green CI run →
`talerender-binary` artifact → version + sha256 recorded.

```bash
gh run list --repo turjaun985123/talerender --workflow CI --limit 5   # runs, exit 0
gh run download 43624139322 --repo turjaun985123/talerender \
    -n talerender-binary -D ~/workspace/.w68-dryrun/talerender-bin
chmod +x ~/workspace/.w68-dryrun/talerender-bin/talerender
```

(Runbook downloads to `~/talerender-bin`; the dry run used a
scratch dir instead — same route, different `-D`. The
pre-existing `~/talerender-bin/talerender` on this VM is an
older CLI-proof copy and was not used.)

Verbatim results:

```
-rwxr-x--- 1 root nogroup 19544384 … talerender
$ talerender --version
talerender 1.0.0
version-exit:0
$ sha256sum talerender
6a6977cc0219c302129b385e075d6e8b761e011d319f2d428c40fe01870b6b09
```

Run `43624139322` is the green CI run at `b6ee9fb` — at/after
`aa92cee`, per the runbook's "at or after the CLI release"
rule. The newest green run at dry-run time (`43624652280` at
`4dd6530`) was checked separately and **also carries**
`talerender-binary` (5,877,340 B) + `talerender-frame`, so
"latest green" provisioning works as written.

CLI surface (the runbook §0.2/§2.1 claims):

```
$ talerender --help            # exit 0, 85 lines
TaleRender — native Rust + wgpu renderer for Tall Tale Shorts.
…
EXIT CODES:
    0   Success.
    1   --film-telemetry QA verdict: CHECK FAILED (the printed
        line carries the counts and stays authoritative).
    2   Command-line error (unknown flag, bad or missing value).
    3   --film-telemetry could not run (film data unreadable;
        the reason is printed on stderr, no verdict line).

$ talerender --bogus-flag
error: unknown argument: --bogus-flag
Try 'talerender --help' for usage.
badflag-exit:2
```

The help's exit-code table matches the runbook §2.1 dated
note (0/1/3 for telemetry, 2 for CLI errors) exactly.

## §0.3 Environment capture — RUN (in scratch), block is sound

PERF.md §1's block was run verbatim in a scratch dir with
`talerender-bin` symlinked to the provisioned binary: it
completed, exit 0, and produced a well-formed `env.txt`
(date / kernel / RAM / ICD list / binary sha256 / CI run id).
One silent gap: `vulkaninfo` is **absent on this VM**, so the
`== vulkan ==` section came out empty — the block's
`2>/dev/null` swallows that without a trace (see F4).

## §0.4 Adapter proof — NOT-RUN-HERE (RADV), command shape verified

The adapter *proof* is PC-only by definition; what could be
verified here is the smoke command and the line format.
Run (binary path substituted per F1; `--out` redirected to
scratch instead of the runbook's `/tmp/scene.png`):

```
$ talerender --scene-file scenes/character-v2.scene.json --out scene.png
adapter: llvmpipe (LLVM 20.1.2, 256 bits) | backend: Vulkan | device type: Cpu | driver: llvmpipe Mesa 25.2.8-0ubuntu0.24.04.4 (LLVM 20.1.2)
scene: file — format v2 proof — character node, camera keys, light rig (Onoda S01 staging at t=1.50) (…)
wrote …/scene.png (1080x1920)
scene-exit:0
```

Findings from this run:

- The printed line matches the runbook's §0.4 template
  verbatim (`adapter: <name> | backend: Vulkan | device
  type: <type> | driver: <driver> <info>`), and on this
  machine it names llvmpipe / `Cpu` — i.e. this dry run
  produces exactly the "invalid leg" signature §0.4 tells the
  operator to reject. The rejection logic is exercisable and
  correct; the RADV-positive branch is **NOT-RUN-HERE** — it
  first occurs at PC-EVIDENCE §0.4 on the PC (and gates §1's
  results-file renderer check, PERF §§2–5, and §3).
- `scenes/character-v2.scene.json` exists at the runbook's
  path. ✓
- The ICD fallback's glob is well-formed:
  `ls /usr/share/vulkan/icd.d/radeon_icd*.json` →
  `/usr/share/vulkan/icd.d/radeon_icd.json` on this VM. ✓
  (Its first command, `vulkaninfo --summary`, is the F4 gap.)

## §1 Determinism E7 — NOT-RUN-HERE (RADV pack); refusal legs RUN

The pack itself renders film frames four ways on the renderer
of record; on llvmpipe that is the CI leg's territory (E6,
already discharged), not E7. **NOT-RUN-HERE:** the full pack
run — PC-EVIDENCE §1 on the PC, after §0.4's adapter proof.

What was exercised here — the pack's two refusal paths
(`tools/determinism/determinism_check.sh`, which exists at
the runbook's path and defaults its film dir to the repo's
`film/`):

```
$ determinism_check.sh /nonexistent/talerender <fresh-dir>
REFUSAL: binary not found or not executable: /nonexistent/talerender
missing-binary-exit:2

$ determinism_check.sh <provisioned binary> <root with A-single/ non-empty>
REFUSAL: output dir not fresh: …/A-single already contains files. The driver skips existing frames, so a reused dir could fake a pass. Remove it or choose a new output root.
nonfresh-exit:2
```

Both match §1's contract (exit 2 = refusal, `REFUSAL:` line
names the cause, no verdict claimed), and the non-fresh
refusal fires *before* any render — the capability probe that
precedes it is a CPU-only data-load failure against an empty
dir. The script also verifies the film dir carries
`onoda.json`; the repo's `film/` does (it holds `onoda.json`,
no `film.json` — the loader's generic-entry/fallback pair
both remain exercised by every `film`-arg command in this
note).

## §2 Bench legs — NOT-RUN-HERE (renders), report refusal + telemetry legs RUN

- **NOT-RUN-HERE:** PERF §§2/3/5 (ab1 + S01 stills, the
  1027..1117 film chunk, disk sizing) and §6 (Chrome
  comparison) — all are render legs whose VM numbers would
  be lavapipe timings, which PERF.md itself forbids quoting;
  they first occur at PC-EVIDENCE §2.1 items 1/2/4/5 on the
  PC. Command shapes were source-checked instead:
  `scenes/film/s01.scene.json` exists ✓; `--scene ab1` is a
  documented mode in `--help` ✓.
- **RUN — the report generator's refusal** (§2.3's command,
  pointed at an empty directory):

  ```
  $ python3 tools/perf/bench_report.py <empty-dir> --out REPORT.md
  bench_report: ERROR: no `renderer:`/`adapter:` line found in any log or env.txt — PERF.md requires every table to name its renderer, so this directory cannot be reported. (On the PC, copy the adapter line into perf/env.txt per PERF §1.)
  bench-report-exit:2
  ```

  Exactly the mechanical enforcement §2.0/§2.3 describe
  (no renderer line → refusal, exit 2). The CLI shape
  (`<dir> --out <file>`) is as the runbook writes it.
- **RUN — PERF §4's two telemetry legs** (the §2.1 item-3
  local analogue; CPU-only, renderer-independent): see §3.1
  below — default cap and `TALERENDER_PAINT_CACHE=24` must
  reach the same verdict line.

## §3.1 Telemetry verdict — RUN (this is also §2's PERF §4 analogue)

First, the runbook's command **as written**, from the repo
root §0.1 leaves you in:

```
$ ./talerender-bin/talerender --film-telemetry film
/bin/bash: line 1: ./talerender-bin/talerender: No such file or directory
as-written-exit:127
```

That is F1 (below). With the provisioned binary's path used
directly, from the repo root, output redirected to scratch
files in the runbook's own `perf/`-style arrangement:

```
$ talerender --film-telemetry film            # default cap
… progress on stderr: film-telemetry: sample 20 (t=4.75) … sample 240 (t=59.75)
paint-cache: entries=74 bytes=2133852160 hits=101 misses=381 evictions=307 rerenders=0 peak_bytes=2162688000 budget_bytes=2147483648
FILM TELEMETRY — THE SOLDIER WHO DIDN'T KNOW THE WAR WAS OVER (native, 0.25 s sampling)
CANVAS OVERFLOW: none
CONTINUITY: clean (5 named characters tracked)
CAPTIONS: estimate-grade (auto-chunked) on lines 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12 — author caps in script.json for a master
FILM TELEMETRY: CHECK OK
default-exit:0

$ TALERENDER_PAINT_CACHE=24 talerender --film-telemetry film
paint-cache: entries=24 bytes=692060160 hits=101 misses=381 evictions=357 rerenders=0 peak_bytes=720896000 budget_bytes=692060160
CANVAS OVERFLOW: none
CONTINUITY: clean (5 named characters tracked)
FILM TELEMETRY: CHECK OK
```

The verdict line is exactly the runbook's expected line, it
is the **last** line of stdout (so §3.1's `tail -n 1`
procedure returns it), and the exit code is **0** — line and
code agree, per §2.1's dated note. The PERF §4 A/B pair
(default vs cap 24) reached the **same verdict line**, as
required; the two 39-line reports are identical except the
paint-cache statistics. One dry-run casualty, disclosed: the
cap-24 run's own exit code was not captured — its detached
wrapper was killed in a process-management error by this
seat after a VM reboot (its binary ran on orphaned and
completed; the verdict line above is from its log). By the
binary's documented contract (`--help`: 1 only on CHECK
FAILED, 3 only when the gate could not run) a completed
CHECK OK run exits 0 — stated here as the contract's
implication, not as an observation. The default-cap leg's
exit 0 *was* observed directly. (The CAPTIONS line above the
verdict is an advisory about caption provenance; it is not
part of the verdict and does not disturb it — the runbook
does not mention it, and an operator meeting it for the
first time should know it is expected at this head.)

RAM observation for §2.1's flag: the default-cap telemetry
run peaked at `peak_bytes=2162688000` (~2.16 GB) with only
74 cache entries live — the paint cache's byte budget
(`budget_bytes=2147483648`, printed by the binary) bound it
well below the 160-entry worst case the runbook quotes.
See F6.

## §3.2 Full film render — NOT-RUN-HERE

The render itself is the PC session's long pole (RADV
timings; a lavapipe render here proves nothing the CI film
jobs haven't). **NOT-RUN-HERE** — PC-EVIDENCE §3.2 on the PC.
Verified from source instead: the completion line the
runbook quotes is `src/film.rs:3467`'s format —
`film render complete: {done} rendered, {skipped} skipped
(resume), range {from}..{to} · {} · {}` — see F2. The frame
naming (`frame_%05d.png`, resume-skip) matches `--help`'s
`--film` text verbatim.

## §3.3 Finish — NOT-RUN-HERE (needs §3.2's frames); shape verified

- `film/finish.sh` exists, is executable, and its usage is
  `film/finish.sh FRAMES_DIR MIX_WAV OUT_MP4` — the runbook's
  argument order. ✓
- The mix path exists (on this VM, and it is the project's
  own path the PC also holds):
  `~/faceless-shorts/the-soldier-who-didnt-know/data/mix.wav` ✓
- The runbook's ffprobe expectations were checked against
  the existing proof MP4 that this same `finish.sh`
  produced from the lavapipe frames
  (`~/workspace/your_files/onoda-native-f021991.mp4`):

  ```
  codec_name=h264  width=1080  height=1920  r_frame_rate=30/1
  codec_name=aac   duration=60.457000
  ```

  = the runbook's "1080x1920, 30 fps, h264 + AAC, ≈60.46 s" ✓

## §4 Golden verification — the pack verify RUN (18 OK); the RADV comparison NOT-RUN-HERE

The runbook's §4 compares §3.2's RADV mids against the
f021991 candidates — **NOT-RUN-HERE** (no RADV frames exist);
it first occurs at PC-EVIDENCE §4.3 on the PC, after §3.2.
Everything around it was exercised:

- The candidate set is intact at the runbook's paths:
  `docs/golden/frames-f021991/` holds all 14 segment PNGs +
  `RENDERER.txt` (`renderer: CI lavapipe`). Spot-checked
  hashes match the RENDERER.txt table: s01
  `1608ead9a477764a…`, s10 `59bdb4f2558fa37f…`. ✓
- §4.3 step 1's hash command shape works; the candidates now
  also equal the struck pins (the Act-1 corrective re-pin at
  `2755ff3` copied these candidate bytes onto the canonical
  `docs/golden/frames/` paths — the runbook's
  verify-against-candidates framing is unchanged and still
  the conservative side to compare from).
- **The golden pack verify** (the precondition the whole
  visit's golden talk rests on), run at HEAD `2755ff3`:

  ```
  $ python3 tools/golden/make_golden.py
  item            status   pinned            current           path
  s28-onoda-t150  OK       ff68fad3bbcc6155  ff68fad3bbcc6155  docs/ab/onoda-s01-t150/native.png
  dof-chart       OK       957d5573497e6269  957d5573497e6269  docs/ab/dof-chart/native.png
  stroke-toon     OK       9ba9ff890402b6cf  9ba9ff890402b6cf  docs/ab/stroke-battery/native-toon.png
  ab1-composer    OK       60c32c1af021cd0c  60c32c1af021cd0c  docs/ab/composer/native.png
  film-s01        OK       1608ead9a477764a  1608ead9a477764a  docs/golden/frames/s01.png
  … (film-s02 … film-s12 all OK, pinned == current)
  18 OK / 0 STALE / 0 MISSING of 18 golden items
  ```

- §4.3 step 2's measure command was run in its exact shape
  (candidate vs itself, the only pair available here):

  ```
  $ python3 tools/ab/measure_ab.py --pair docs/golden/frames-f021991/s01.png docs/golden/frames-f021991/s01.png \
      --reference-label "f021991 candidate (CI lavapipe)" --native-label "PC RADV" --out <dir>
  { "psnr_db": "inf", "mean_abs_delta_per_channel": 0.0,
    "pct_pixels_max_channel_delta_gt_10": 0.0,
    "pct_pixels_any_channel_delta_gt_0": 0.0, "max_abs_delta": 0 }
  measure-exit:0
  ```

  Flags, labels and exit behaviour as documented — but see
  F3 on what `--out` actually produces.

## §5 Send-back — not applicable to a dry run

No package is sent anywhere from here. The §5.1 file list
was checked for producibility: every named file is the
output of a leg above; the two entries affected by findings
are `perf/*.log` (F1's path failure would have left the
§3.1 log empty on the PC) and `radv-golden/*.txt` (F3).

---

## Findings

### F1 — BLOCKING (one wasted command + confusion on the PC): the binary path in the run commands does not resolve from the repo root

Runbook text (§0.2) provisions the binary to the *home*
directory:

> `gh run download <run-id> --repo turjaun985123/talerender -n talerender-binary -D ~/talerender-bin`

…while §0.1 leaves the operator in the repo root
(`cd ~/workspace/talerender`), and §§0.4/2.1/3.1/3.2 then
invoke the binary repo-relatively — §3.1:

> `./talerender-bin/talerender --film-telemetry film \`

Observed truth (this dry run, verbatim above): from the repo
root that command fails — `No such file or directory`, exit
127 — because there is no `talerender-bin/` inside the
checkout (verified: none exists; the binary is at
`~/talerender-bin/`). The same defect is in USER-PC.md, whose
§1 provisions to `~/talerender-bin` and whose §§3–5 use
`./talerender-bin/talerender` "run from the repo root".
PERF.md's header states the convention the commands assume —
"the CI binary at `./talerender-bin/talerender`" — i.e. the
*commands* expect the download to land in
`<repo>/talerender-bin/`, and the *provisioning step* is the
side that disagrees. One-line fix on either side (download
`-D talerender-bin` from the repo root, or invoke
`~/talerender-bin/talerender` throughout); until it is made,
every §-numbered command block after §0.2 fails on first
contact, including the §3.1 gate that is supposed to precede
the long render. (The determinism pack in §1 is immune — it
takes the binary as an argument, and the runbook passes it
the absolute `~/talerender-bin/talerender`.)

### F2 — minor: §3.2's quoted completion line is a prefix of the real line

Runbook (§3.2): "the final line reads `film render
complete: 1811 rendered, 0 skipped (resume), range
0..1811`." Source (`src/film.rs:3467`): the format is
`film render complete: {done} rendered, {skipped} skipped
(resume), range {from}..{to} · {} · {}` — two further
`·`-separated fields follow the range. An operator (or a
script) matching the quoted line exactly will miss; matching
it as a prefix works. Quote the full format or mark the
quote as a prefix.

### F3 — minor: §4.3's `--out "radv-golden/$seg-ab.txt"` produces directories, not .txt files — §5.1's send-back row inherits the mismatch

`tools/ab/measure_ab.py` defines `--out` as an "output
directory for the report + images". Run in the runbook's
exact shape, `--out <path>/s01-ab.txt` created a *directory*
named `s01-ab.txt` containing `ab-report.json` (the PSNR
lives in its `psnr_db` field), `ab-report.md`,
`pair_diff.png`, and `pair_side-by-side.png`. The §4.3 loop
therefore yields 14 directories named `*.txt`, and §5.1's
package row "`radv-golden/*.txt` — per-mid PSNR outputs"
does not describe what the operator will actually send.
Fix by pointing `--out` at `radv-golden/$seg/` and correcting
§5.1 to `radv-golden/*/ab-report.json` (+ `.md`).

### F4 — minor: `vulkaninfo` is an unlisted prerequisite; PERF §1's block hides its absence

Neither PC-EVIDENCE (§0.3 "Run PERF.md §1 verbatim", §0.4's
fallback) nor USER-PC §2 lists `vulkan-tools` as a
prerequisite, but §0.4's wrong-adapter branch starts with
`vulkaninfo --summary`, and PERF §1's env block embeds it.
On this VM `vulkaninfo` is not installed; the env block
still exits 0 with an empty `== vulkan ==` section (the
command's stderr is discarded by the block itself), so the
gap is invisible until the operator actually needs the
fallback. List the package in §0, or note the section may be
empty when the tool is absent. (The adapter proof itself
does not depend on vulkaninfo — the binary prints the line —
so this only weakens the fallback branch.)

### F5 — minor: §0.2's run-list step does not say the listed run must be green/completed

§0.2's prose says the binary "comes from a green CI run's
artifact", but the command block (`gh run list … --limit 5`
then `gh run download <run-id>`) never repeats the
constraint, and the list's top rows are routinely
in-progress or cancelled (observed verbatim in this dry run:
of the top 5 — 1 in progress, 2 cancelled, 2 success; an
in-progress run has no artifact yet and a cancelled run's
artifact, if any, predates its head's verdict). One clause
in the block — "pick a `completed / success` row" — removes
the ambiguity.

### F6 — minor: the RAM flags quote the pre-budget worst case; telemetry's observed peak is budget-bound at ~2.16 GB

§§1/2.1/3.2 flag the default paint cache's worst case as
≈4.6 GB (~29 MB × the 160-entry cap) and §2.1 calls the
default-cap telemetry leg "the sharp edge" for OOM on the
PC's ~6.7 GiB. That math predates the paint cache's byte
budget (Wave 2, `86f0c81`). In this dry run the default-cap
telemetry leg printed its own accounting —
`entries=74 … peak_bytes=2162688000 budget_bytes=2147483648`
— i.e. the 2 GiB byte budget, not the 160-entry cap, was the
binding constraint, and the whole film's telemetry touched
only 74 distinct canvases. Scoped honestly: this observation
covers the *telemetry* leg; the full *render*'s peak was not
measured here (NOT-RUN-HERE, §3.2) and may differ. But for
§2.1 item 3 / §4A the warning as written overstates the
current risk, and the binary already prints the numbers
that correct it — the runbook should cite the budget line
alongside the entry-cap math so the operator can see which
bound is live in their own log.

## What was NOT run here, and where the PC operator hits it

| Leg | Status here | First real occurrence |
|---|---|---|
| Adapter proof naming RADV / IntegratedGpu / radv | NOT-RUN-HERE (llvmpipe line verified as the negative case) | PC-EVIDENCE §0.4 |
| Determinism pack full run (E7) | NOT-RUN-HERE (both refusal paths verified) | PC-EVIDENCE §1 |
| PERF §2 stills, §3 chunks, §5 disk, §6 Chrome | NOT-RUN-HERE (shapes source-checked) | PC-EVIDENCE §2.1 |
| Full film render (1,811 frames) + its timing | NOT-RUN-HERE (line format source-verified, F2) | PC-EVIDENCE §3.2 |
| finish.sh encode to MP4 | NOT-RUN-HERE (usage + mix path + expected ffprobe shape verified against the existing finish.sh-produced proof) | PC-EVIDENCE §3.3 |
| Golden PSNR vs RADV mids | NOT-RUN-HERE (candidates, hashes, pack verify 18 OK, measure tool all verified) | PC-EVIDENCE §4.3 |

**Dry-run caveats.** This VM was heavily contended during
the run (multiple wave seats + a VM reboot that killed the
first telemetry attempt — the legs were re-run detached), so
all elapsed times here are contention artefacts and are
deliberately not reported as numbers. No step of the runbook
was edited; this note is the only file written.
