# DRYRUN-FIX-W96 — W68 findings disposition: paste-ready runbook patches + the W77 waiver

**Seat:** TaleRender Wave 2, Batch 10 (final), W96.
**HEAD at writing:** `9380b01`. All line cites below are first-hand
at this HEAD (the tree moved under W68's dry run; where W68's
cites and this note's differ, this note's govern).
**Inputs:** `docs/integration/PC-DRYRUN-W68.md` (findings F1
BLOCKING + F2–F6 against the PC runbook) and the W77 note
question (Batch 9's gate saw the W77 tool file modified in the
working tree; §8 below settles it).
**Law of this seat:** this file is the only file written.
Nothing edited in place, nothing committed. Every patch below
is paste-ready for the lead: exact old text → exact new text,
with the file and line it lands on.

**Disposition summary:**

| Finding | Verdict | Where |
|---|---|---|
| F1 (binary path contradiction, BLOCKING) | Canonical side picked: `~/talerender-bin` + `TALERENDER_BIN`. 13 invocation lines patched across 3 files; **zero** provisioning steps change | §1, §2 |
| F2 (§3.2 completion line is a prefix) | Patch: quote the full format, mark prefix-match | §3 |
| F3 (§4.3 `--out` makes directories) | Patch: `--out "radv-golden/$seg"` + §5.1 row corrected | §4 |
| F4 (`vulkaninfo` unlisted prerequisite) | Patch: package named at both use sites | §5 |
| F5 (§0.2 doesn't say pick a green run) | Patch: one comment line in both provisioning blocks | §6 |
| F6 (RAM flags quote the pre-budget worst case) | Numbers ruling + replacement text at 5 stale sites; PERF §4 needs **no** patch (already corrected) | §7 |
| W77 note | **Absent — formally recorded; waiver text supplied.** Tool stands gated 3/3 by W80 | §8 |

---

## §1 The canonical-path decision (the ground F1's patches stand on)

**Decision: the binary lives at `~/talerender-bin/talerender`,
selected by `export TALERENDER_BIN=~/talerender-bin/talerender`.
The repo-relative `./talerender-bin/talerender` invocations are
the defect; every provisioning step already downloads to the
home directory and stays untouched.**

Justification, document by document — every file that touches
the path, checked first-hand:

- **The release packs already declared this canonical.**
  `docs/integration/RELEASE-101B-W89.md:441-442`: "Canonical
  path: `~/talerender-bin/talerender`, selected by
  `export TALERENDER_BIN=~/talerender-bin/talerender`".
  `docs/integration/RELEASE-101-W46.md:473-474` says the same.
  Picking the other side would contradict the wave's own
  release record in two places.
- **The skill adapter can only speak this convention.**
  The wiring work copy's `template/tools/capture_native.mjs:15`
  documents "TALERENDER_BIN=/path/to/talerender wins", and its
  `findBinary()` honours the env var first
  (capture_native.mjs:69-72: an explicit TALERENDER_BIN "is
  never second-guessed"). A repo-relative path convention
  cannot be selected by the adapter at all; the home path +
  env var is the one convention every consumer shares.
- **PC-EVIDENCE is internally split, and its majority is
  already canonical.** Provisioning §0.2
  (`docs/runbook/PC-EVIDENCE-W36.md:88-91`) downloads to
  `~/talerender-bin` and invokes it absolutely; §1's pack
  command passes the absolute path (`:168`); §2's bench helper
  already does `export TALERENDER_BIN=~/talerender-bin/talerender`
  (`:227`) and invokes `"$TALERENDER_BIN"` (`:232`). Only five
  invocation lines are repo-relative: `:130` (§0.4), `:311` and
  `:313` (§2.1 item 3), `:413` (§3.1), `:435` (§3.2). The patch
  makes the document agree with itself.
- **USER-PC is split the same way.** §1 provisions to
  `~/talerender-bin` (`docs/runbook/USER-PC.md:25-26`); §§3/4/5
  invoke `./talerender-bin/talerender` (`:66`, `:76`, `:93`).
- **PERF's premise about its own source is false.** PERF.md's
  header (`docs/runbook/PERF.md:25-26`) declares "the CI binary
  at `./talerender-bin/talerender` (see USER-PC.md §1)" — but
  USER-PC §1 provisions to `~/talerender-bin`. PERF's seven
  command sites (`:46`, `:78`, `:85`, `:124`, `:131`, `:181`,
  `:186`) all follow the header's wrong premise. PERF is the
  only document that is repo-relative *throughout*, and it is
  repo-relative by citation of a document that says the
  opposite.
- **Majority practice across the wave's evidence notes** is
  home + absolute: `DETERMINISM-CONTRACT-W04.md:239-248`,
  `E7-READINESS-W78.md:103,166-167,249`,
  `docs/ab/film/TELEMETRY-W85.md:33,47`,
  `LOADER-EVIDENCE-W86.md:17`, `DETERMINISM-RUN-W96.md:192-196`,
  the golden repin packets (`docs/golden/repin-packets/stroke-toon.md:45-51`).
- **Cost asymmetry.** The canonical pick changes 0 provisioning
  steps and 13 invocation lines in 3 files. The repo-relative
  pick would change every provisioning step in every document,
  both release packs' declared canonical path, and the
  adapter's selection convention — to defend a directory inside
  the checkout that `git status` would forever show as clutter
  and one `git clean` would delete mid-session.

**Deliberately NOT patched:** `BENCH-FIRST-W64.md:303,318-334`
uses `./talerender-bin/talerender` — it is a dated evidence
record of a run as executed (its bench window used a repo-local
scratch layout), not operator instruction; rewriting it would
falsify the record. The same applies to scratch downloads in
evidence notes (`docs/ab/onoda-s01-t150/README.md:225`,
TELEMETRY-W85's scratch route). The operator surface is the
three runbooks; that is the whole patch surface.

---

## §2 F1 — the path patches (paste-ready)

The substitution is uniform: `./talerender-bin/talerender` →
`~/talerender-bin/talerender`. Sites are listed individually so
the landing can be verified line by line.

### 2.1 `docs/runbook/PC-EVIDENCE-W36.md` — 5 sites

Line 130 (§0.4):

```
-old: ./talerender-bin/talerender --scene-file scenes/character-v2.scene.json --out /tmp/scene.png
+new: ~/talerender-bin/talerender --scene-file scenes/character-v2.scene.json --out /tmp/scene.png
```

Line 311 (§2.1 item 3, leg A):

```
-old:      `./talerender-bin/talerender --film-telemetry film`
+new:      `~/talerender-bin/talerender --film-telemetry film`
```

Line 313 (§2.1 item 3, leg B):

```
-old:      `TALERENDER_PAINT_CACHE=24 ./talerender-bin/talerender --film-telemetry film`
+new:      `TALERENDER_PAINT_CACHE=24 ~/talerender-bin/talerender --film-telemetry film`
```

Line 413 (§3.1):

```
-old: ./talerender-bin/talerender --film-telemetry film \
+new: ~/talerender-bin/talerender --film-telemetry film \
```

Line 435 (§3.2):

```
-old: ./talerender-bin/talerender --film film --out radv-frames/ \
+new: ~/talerender-bin/talerender --film film --out radv-frames/ \
```

No other line in the file changes: §0.2 (`:88-91`), §1 (`:168`)
and §2's helper (`:227,232`) are already canonical — W68's
"determinism pack is immune" observation (`PC-DRYRUN-W68.md`
F1) is exactly this.

### 2.2 `docs/runbook/USER-PC.md` — §1 block + 3 sites

§1's block (lines 22-27) gains the export line, so USER-PC
speaks the release packs' "selected by" language (this block
also carries F5's comment — see §6.2; the two edits are one
paste):

Old:

```bash
gh run list --repo turjaun985123/talerender --workflow CI --limit 5
gh run download <run-id> --repo turjaun985123/talerender \
    -n talerender-binary -D ~/talerender-bin
chmod +x ~/talerender-bin/talerender
```

New:

```bash
gh run list --repo turjaun985123/talerender --workflow CI --limit 5
# pick a completed / success row — an in-progress run has no artifact
# yet, and a cancelled run's artifact may predate its head's verdict
gh run download <run-id> --repo turjaun985123/talerender \
    -n talerender-binary -D ~/talerender-bin
chmod +x ~/talerender-bin/talerender
export TALERENDER_BIN=~/talerender-bin/talerender
```

Line 66 (§3):

```
-old: ./talerender-bin/talerender --scene-file scenes/character-v2.scene.json --out /tmp/scene.png
+new: ~/talerender-bin/talerender --scene-file scenes/character-v2.scene.json --out /tmp/scene.png
```

Line 76 (§4):

```
-old: ./talerender-bin/talerender --film-telemetry film
+new: ~/talerender-bin/talerender --film-telemetry film
```

Line 93 (§5):

```
-old: ./talerender-bin/talerender --film film --out frames/
+new: ~/talerender-bin/talerender --film film --out frames/
```

### 2.3 `docs/runbook/PERF.md` — header + 7 sites

Header, lines 25-26. Old:

```
- All commands run from the repo root with the CI binary at
  `./talerender-bin/talerender` (see USER-PC.md §1). The binary has
```

New:

```
- All commands run from the repo root with the CI binary at
  `~/talerender-bin/talerender` (provisioned by USER-PC.md §1;
  `TALERENDER_BIN` names the same path). The binary has
```

Command sites — the uniform substitution, one line each:
`:46` (`sha256sum` in the §1 env block), `:78` and `:85` (§2
stills), `:124` and `:131` (§3 chunks), `:181` and `:186` (§4
telemetry A/B). After the substitution PERF contains no
repo-relative binary path, and its header's citation of
USER-PC §1 becomes true.

---

## §3 F2 — §3.2's completion line is a prefix of the real line

Source at this HEAD, `src/film.rs:3557-3561` (W68 cited :3467;
the file has grown):

```rust
println!(
    "film render complete: {done} rendered, {skipped} skipped (resume), range {from}..{to} · {} · {}{tag}",
    clock.summary_suffix(),
    cache.stats_line()
);
```

The two trailing fields are the timing summary and the
paint-cache stats line; `{tag}` appends the DRAFT tag on a
knob run. (The cache-stats field is load-bearing for §7 below:
it is where the render's own `peak_bytes=` / `budget_bytes=`
appear.)

Patch — `docs/runbook/PC-EVIDENCE-W36.md`, §3.2 bullet at
lines 442-443. Old:

```
  prints every 25 frames; the final line reads
  `film render complete: 1811 rendered, 0 skipped (resume), range 0..1811`.
```

New:

```
  prints every 25 frames; the final line begins
  `film render complete: 1811 rendered, 0 skipped (resume), range 0..1811`
  and continues ` · <timing summary> · <paint-cache stats>`
  (format: `src/film.rs:3557-3561`; a draft/knob run appends
  its DRAFT tag). Match it as a prefix — the quoted part is
  not the whole line.
```

---

## §4 F3 — §4.3's `--out` produces directories, not .txt files

Tool truth, first-hand: `tools/ab/measure_ab.py:514` defines
`--out` with help "output directory for the report + images".
W68 ran the runbook's exact shape and observed a *directory*
named `s01-ab.txt` containing `ab-report.json` (PSNR in its
`psnr_db` field), `ab-report.md`, `pair_diff.png`,
`pair_side-by-side.png` (`PC-DRYRUN-W68.md` F3).

Patch 1 — `docs/runbook/PC-EVIDENCE-W36.md` §4.3 loop, line 555.
Old:

```
      --out "radv-golden/$seg-ab.txt"
```

New:

```
      --out "radv-golden/$seg"
```

Patch 2 — same section, insert after the loop's closing
`` ``` `` and before "Fill from the outputs" (the table at
`:559`):

```
Each `--out` is a directory (`tools/ab/measure_ab.py:514`):
`radv-golden/$seg/` holds `ab-report.json` (the PSNR is its
`psnr_db` field; mean |Δ| is `mean_abs_delta_per_channel`),
`ab-report.md`, `pair_diff.png` and `pair_side-by-side.png`.
Fill the table from the `ab-report.json` files.
```

Patch 3 — §5.1 send-back row, line 622. Old:

```
| the §4 table (filled) + `radv-golden/*.txt` | §4.3 — per-mid PSNR outputs |
```

New:

```
| the §4 table (filled) + `radv-golden/*/ab-report.json` (+ `ab-report.md`) | §4.3 — per-mid PSNR outputs |
```

---

## §5 F4 — `vulkaninfo` is an unlisted prerequisite

The dependency, first-hand: PERF §1's env block embeds
`vulkaninfo --summary 2>/dev/null | …`
(`docs/runbook/PERF.md:45`); PC-EVIDENCE §0.4's wrong-adapter
fallback starts with it (`PC-EVIDENCE-W36.md:137`); USER-PC §2
instructs it (`USER-PC.md:49`) and §8's troubleshooting
assumes it (`USER-PC.md:155`). No document names the package.
W68 observed the failure mode on this VM: tool absent → the
env block still exits 0 with an empty `== vulkan ==` section,
invisible until the fallback is needed (`PC-DRYRUN-W68.md`
F4). The adapter proof itself does not depend on vulkaninfo
(the binary prints the line) — only the fallback branch and
the env record do.

Patch 1 — `docs/runbook/PC-EVIDENCE-W36.md` §0.3, lines 96-99.
Old:

```
Run PERF.md §1 verbatim — it writes `perf/env.txt` (date,
kernel, RAM, ICD list, Vulkan summary, binary sha256, CI run
id):
```

New:

```
Run PERF.md §1 verbatim — it writes `perf/env.txt` (date,
kernel, RAM, ICD list, Vulkan summary, binary sha256, CI run
id). One prerequisite: the Vulkan summary comes from
`vulkaninfo`, package **`vulkan-tools`** — if the tool is
absent the `== vulkan ==` section is silently empty (the
block still exits 0), so install it first
(`sudo apt install vulkan-tools`). The §0.4 adapter proof
does not depend on it; §0.4's wrong-adapter fallback and
this section do:
```

Patch 2 — `docs/runbook/USER-PC.md` §2, insert immediately
after the code block ending at line 51
(`ls /usr/share/vulkan/icd.d/ # radeon_icd*.json is the RADV ICD`):

```
(`vulkaninfo` is in the `vulkan-tools` package —
`sudo apt install vulkan-tools` if the command is missing.
§8's troubleshooting assumes it is installed.)
```

No PERF patch: PERF §1 is the block being run, not the place
an operator reads prerequisites; the two patches above put
the package name on both reading paths that lead to it.

---

## §6 F5 — §0.2's command block must say "pick a green run"

W68 observed the top-5 run list as 1 in-progress, 2 cancelled,
2 success (`PC-DRYRUN-W68.md` F5): an in-progress run has no
artifact yet, and a cancelled run's artifact, if any, predates
its head's verdict. PC-EVIDENCE §0.2's prose already says
"green CI run" (`:78`) but the block doesn't repeat it;
USER-PC §1's prose says "latest green CI run" (`USER-PC.md:19-20`)
with the same gap in its block.

Patch 1 — `docs/runbook/PC-EVIDENCE-W36.md` §0.2 block,
between lines 86 and 87. Old:

```bash
gh run list --repo turjaun985123/talerender --workflow CI --limit 5
gh run download <run-id> --repo turjaun985123/talerender \
```

New:

```bash
gh run list --repo turjaun985123/talerender --workflow CI --limit 5
# pick a completed / success row — an in-progress run has no artifact
# yet, and a cancelled run's artifact may predate its head's verdict
gh run download <run-id> --repo turjaun985123/talerender \
```

Patch 2 — USER-PC §1: the identical comment is already folded
into the §2.2 replacement block above (it lands with the
`TALERENDER_BIN` export line as one paste). No separate edit.

---

## §7 F6 — the RAM numbers: what the runbook should say now

### 7.1 The three numbers, kept separate

The confusion in the current text is that three different
quantities are being quoted as one "worst case". They are:

1. **The entry-cap term: ≈4.6 GB.** One painted canvas is
   2560×2816 RGBA = 28,835,840 B (~29 MB); the default entry
   cap is 160 (`src/paintbudget.rs:38`,
   `DEFAULT_ENTRY_CAP: usize = 160`). 160 × 28,835,840 =
   4,613,734,400 ≈ 4.6 GB. This was the live bound **before**
   the byte budget landed (`86f0c81`). It is again the live
   bound only if the byte budget is disabled or raised —
   `TALERENDER_PAINT_BUDGET_MB=0` restores "entry cap alone,
   legacy behaviour" (`src/paintbudget.rs:40-54` doc).
2. **The live default bound: 2 GiB.** Since `86f0c81` the
   cache is also bound by a byte budget:
   `default_budget_bytes = min(entry_cap × canvas,
   BUDGET_CEILING_BYTES)` with the ceiling at 2048 MiB
   (`src/paintbudget.rs:54`, `.min(…)` at `:71`). At the
   default cap: min(4.6 GB, 2 GiB) = **2,147,483,648 B** —
   ~74 canvases, not 160. At cap 24 the entry term is smaller
   and sets the budget: 24 × 28,835,840 = 692,060,160 B
   (~0.69 GB).
3. **W68's observed telemetry peak: 2,162,688,000 B.**
   The default-cap *telemetry* leg printed
   `entries=74 bytes=2133852160 … peak_bytes=2162688000
   budget_bytes=2147483648` (`PC-DRYRUN-W68.md` §3.1). The
   arithmetic closes exactly: 74 live entries × 28,835,840 =
   2,133,852,160 (the printed `bytes=`), peak = 75 canvases
   transiently, budget = the 2 GiB ceiling. The leg was
   **budget-bound, not entry-bound**, and the whole film's
   telemetry touched only 74 distinct canvases.

### 7.2 The scope rule (do not conflate)

Number 3 is a **telemetry** peak: a CPU-only audit sampling
the film at 0.25 s. The **render** peak was not measured in
the dry run (W68 §3.2: NOT-RUN-HERE) and this note does not
predict it. What can be said of the render: its paint cache
answers to the same budget, so the cache component is
bounded by the same 2 GiB; total render RSS additionally
includes the renderer (GPU-mapped buffers, textures, frame
targets), the loaded film assets and the binary — PERF §4's
existing wording, which is correct. No runbook text may
print "the render peaks at ~2.16 GB"; the first render peak
is measured by the PC visit's §3.2, and the completion line
itself reports it (the stats field, §3 above). The honest
risk statement that survives: 2 GiB of cache + renderer +
system on a ~6.7 GiB machine is still the session's binding
constraint, cap 24 remains the recommended setting for long
renders on a busy machine, and the exit-137 OOM protocol is
unchanged.

### 7.3 PERF §4 needs no patch

PERF already states the corrected math
(`docs/runbook/PERF.md:195-200`): "cached canvas bytes also
capped by PaintBudget at **min(cap × canvas, 2048 MiB)**
since `86f0c81` (the pre-budget worst case was ~4.6 GB);
cap 24 → **~0.7 GB**", with the RSS-above-ceiling guidance
at `:200-205`. The stale flags are the four sites that
*quote* PERF §4 without its correction: PC-EVIDENCE §1
(`:189-192`), §2.2 (`:334-337`), §3.2 (`:450-452`), and
USER-PC §7 (`:141-143`) + §8 (`:162-164`). Patches:

### 7.4 Patch — PC-EVIDENCE §1 RAM flag (lines 189-192)

Old:

```
  design** — that is part of what is being tested. **RAM flag:**
  one painted canvas is ~29 MB and the default cap's worst
  case is ≈4.6 GB on this ~6.7 GiB machine (PERF §4). The pack
```

(The bullet begins "Runs A/B use the driver's **default
paint-cache cap by" at `:188-189`.)

New:

```
  design** — that is part of what is being tested. **RAM flag:**
  one painted canvas is ~29 MB; the entry cap alone (160)
  would allow ≈4.6 GB, but since `86f0c81` a byte budget of
  min(cap × canvas, 2048 MiB) (`src/paintbudget.rs:54`) makes
  the live bound at the default cap **2 GiB** (~74 canvases)
  on this ~6.7 GiB machine (PERF §4). The binary prints the
  live bound itself — `budget_bytes=` in the paint-cache
  stats; check it in the run's log. The pack
```

### 7.5 Patch — PC-EVIDENCE §2.2 RAM flag (lines 334-337)

Old:

```
**RAM flag (§4A is the sharp edge):** telemetry at the default
cap touches every character state in the film at 0.25 s
sampling; the cache's worst case (≈4.6 GB) plus renderer and
system on ~6.7 GiB is a genuine OOM risk — PERF §4 says so and
```

New:

```
**RAM flag (§4A is the sharp edge):** telemetry at the default
cap touches every character state in the film at 0.25 s
sampling; the cache's live bound at the default cap is the
2 GiB byte budget (PERF §4 — the ≈4.6 GB figure is the
pre-budget entry-cap term), and 2 GiB of cache plus renderer
and system on ~6.7 GiB is still a genuine OOM risk — PERF §4
says so and
```

### 7.6 Patch — PC-EVIDENCE §3.2 RAM flag (lines 450-452)

Old:

```
- **RAM flag:** at the default cap (160) the paint cache's
  worst case is ≈4.6 GB (PERF §4 math: ~29 MB × 160) on a
  ~6.7 GiB machine. USER-PC §7's guidance: prefer
```

New:

```
- **RAM flag:** at the default cap (160) the entry-cap term
  is ≈4.6 GB (~29 MB × 160), but the byte budget (`86f0c81`;
  PERF §4) clamps the live bound to min(cap × canvas,
  2048 MiB) = **2 GiB**. Read the actual bound from this
  render's own completion line — its last field is the
  paint-cache stats (`peak_bytes=` / `budget_bytes=`). The
  VM dry run's *telemetry* peak was 2,162,688,000 B against
  that budget (PC-DRYRUN-W68 §3.1); the *render* peak is
  measured by this run, not predicted by the telemetry
  number. On a ~6.7 GiB machine, 2 GiB of cache plus
  renderer and system is still the binding constraint.
  USER-PC §7's guidance: prefer
```

### 7.7 Patch — USER-PC §7 (lines 141-143)

Old:

```
  (~6.7 GiB usable RAM)**: each canvas is ~29 MB, so the
  default cap's worst case is ≈ **4.6 GB**, while
  `TALERENDER_PAINT_CACHE=24` bounds the cache to
  ≈ **700 MB** — the setting telemetry runs use on
```

New:

```
  (~6.7 GiB usable RAM)**: each canvas is ~29 MB, so the
  entry cap alone would allow ≈ **4.6 GB** — but since
  `86f0c81` a byte budget clamps the cache to
  min(cap × canvas, 2048 MiB), making the default cap's
  live bound **2 GiB** (~74 canvases; the binary prints it
  as `budget_bytes=` in its paint-cache stats), while
  `TALERENDER_PAINT_CACHE=24` bounds the cache to
  ≈ **700 MB** — the setting telemetry runs use on
```

### 7.8 Patch — USER-PC §8 (lines 162-164)

Old:

```
  character canvases are ~29 MB each and the paint cache is
  FIFO-capped at 160 entries (worst ≈4.6 GB, but a single
  shot's live working set is far smaller; eviction costs time,
```

New:

```
  character canvases are ~29 MB each and the paint cache is
  FIFO-capped at 160 entries and byte-budgeted at
  min(cap × canvas, 2048 MiB) — a 2 GiB live bound at the
  default cap (the ≈4.6 GB entry-cap figure predates the
  budget); a single shot's live working set is far smaller,
  and eviction costs time,
```

---

## §8 W77 — the note does not exist; formal absence + waiver

**First-hand state at HEAD `9380b01`:**

- `docs/integration/EXPORTER-ACCEPT-W77.md` **does not exist**
  (`ls` by this seat). It has now been absent at three
  consecutive batch closes (W80, W90, and this one).
- What exists is the tool: `tools/format/exporter_acceptance.py`
  + fixtures (3 cases + stub binary), gated first-hand by
  GATE2-W80 §7 — self-test **PASS 3/3** (A: identical pair →
  PASS, exit 0; B: planted 1-px shift → BELOW-BAND 19.48 dB,
  exit 1; C: missing reference → exit 3, COULD NOT RUN),
  verdict ACCEPT-WITH-NOTES covering **the tool and its
  self-test only**. W80 §7 states the limit plainly: with no
  note, "the seat's real-binary / ENV-BLOCKED evidence … is
  **ungated**", and "the CI real-binary acceptance run belongs
  in the Batch-9 definition". No Batch-9 seat executed that
  run; it remains owed.
- The working tree at this HEAD carries an **uncommitted
  modification** to the tool: `git status` shows
  ` M tools/format/exporter_acceptance.py`, `git diff --stat`
  +100/−25 vs HEAD — consistent with GATE2-W90's observation
  that the seat appeared still to be iterating. No gate has
  seen this version of the tool. It is not part of any commit
  and must not silently become the tool of record.

**Waiver text — paste-ready for `docs/integration/WAVE2-RECORD.md`:**

> **W77 note — WAIVED at wave close** (recorded by W96,
> DRYRUN-FIX-W96.md §8). `docs/integration/EXPORTER-ACCEPT-W77.md`
> never landed (absent at the W80, W90 and W96 closes). The
> deliverable that exists, `tools/format/exporter_acceptance.py`
> + fixtures, was gated first-hand by GATE2-W80 §7 (self-test
> PASS 3/3) and stands ACCEPT-WITH-NOTES on that evidence.
> Waived is only the seat's *note* — the write-up of its
> real-binary / ENV-BLOCKED evidence. That evidence is **not**
> closed by this waiver: (a) the CI real-binary acceptance run
> assigned by GATE2-W80 §7 remains an open post-wave item;
> (b) the uncommitted working-tree modification to
> `tools/format/exporter_acceptance.py` present at HEAD
> `9380b01` (+100/−25 vs HEAD, ungated) must be landed with
> its own gate or reverted by the lead — the wave does not
> close over an ungated tool change.

---

## §9 Landing notes

- All patches in §§2-7 are documentation-only (three runbook
  files); no code, no CI surface. Suggested landing: one
  commit per runbook file, or one combined runbook commit —
  the lead's call. They should land **before the PC visit is
  scheduled**: F1 alone guarantees the visit's §3.1 gate
  command fails on first contact until §2.1 is applied.
- Verification after landing: `grep -rn '\./talerender-bin'
  docs/runbook/` returns nothing; `grep -n '4.6' docs/runbook/
  PC-EVIDENCE-W36.md docs/runbook/USER-PC.md` returns only
  the historical framings introduced by §7's patches.
- This seat verified every old-text string against the files
  at `9380b01` before quoting it. If the lead's landing order
  shifts line numbers, the strings — not the numbers — are
  the anchors.
