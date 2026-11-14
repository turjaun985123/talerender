# RUNBOOK-VERIFY — W95 verification record

Seat: W95-finisher (the first W95 seat landed runbook fixes but ended
before writing this record; this finisher audited those fixes and ran
the remaining verification pass).

Scope: `docs/runbook/USER-PC.md`, `docs/runbook/CI.md`,
`docs/runbook/PERF.md` — every command and workflow claim checked
against `src/main.rs` at HEAD and `.github/workflows/` at HEAD.
Provisioned binary `~/talerender-bin/talerender` probed only where a
claim was doubtful (one probe, §2 row 10). Environment claims carried
from the project record (llvmpipe JIT segfault on the dev VM, RADV
behaviour on the user's PC) are not re-verified here — they are not
command/workflow claims.

## 1. Audit of the landed W95 diffs

### CI.md — tools-ci.yml paragraph: **CORRECT**

Checked against `.github/workflows/tools-ci.yml` and
`tools/CI-WORKFLOW.md`:

- "Three workflows" — the directory holds exactly `ci.yml`,
  `render.yml`, `tools-ci.yml`. ✓
- "the Python toolchain gates — film validator, captions, golden,
  bridge" — tools-ci.yml's steps are exactly: Film data validator
  (7/7), Caption strips determinism, Native golden pack verify,
  Bridge packager self-verification, plus a Scene JSON parse sweep
  the runbook's one-line summary omits (a summary omission, not an
  error). ✓
- "on pushes/PRs touching the data trees" — the workflow triggers
  on push + pull_request path-filtered to `tools/**`, `scenes/**`,
  `assets/captions/**`, `film/**`, `docs/golden/**`, `docs/ab/**`,
  plus `workflow_dispatch`. ✓
- "it builds no Rust and renders nothing" — no cargo step exists in
  the workflow; its own header states the same division of labour. ✓
- "contract in `tools/CI-WORKFLOW.md`" — the file exists and
  documents this workflow as its subject. ✓

### PERF.md — §5 budget wording: **CORRECT**

`USER-PC.md` §5 reads verbatim: "Disk: budget roughly **2–6 GB**
for the full PNG set." The corrected PERF.md text — "USER-PC.md
budgets roughly 2–6 GB for the full PNG set" — matches it exactly.
The replaced text (1.2–3.1 MB per frame / ≈2.2–5.6 GB) appears
nowhere in USER-PC.md, so the correction was warranted.

## 2. Per-command verification

Flag ground truth: `src/main.rs` parses exactly 14 flags —
`--width --height --out --scene --scene-file --caption
--post-stage --telemetry --captions --cutouts-out --film-telemetry
--film --from --to` (match arms, main.rs:58–113).

| # | Source | Command / claim | Verdict | Evidence |
|---|---|---|---|---|
| 1 | USER-PC §1 | `gh run list --workflow CI` | VERIFIED | ci.yml `name: CI` |
| 2 | USER-PC §1, CI.md | `gh run download … -n talerender-binary` | VERIFIED | ci.yml uploads artifact `talerender-binary` (`target/release/talerender`); `talerender-frame` is the smoke `frame.png`, as USER-PC §1 states |
| 3 | USER-PC §2 | `vulkaninfo --summary`; `ls /usr/share/vulkan/icd.d/` | VERIFIED | System tools; the ICD naming matches the workflows' own `lvp_icd*.json` glob |
| 4 | USER-PC §2 | `VK_ICD_FILENAMES` ICD pin | VERIFIED | Same variable ci.yml/render.yml set for lavapipe |
| 5 | USER-PC §3 | `--scene-file scenes/character-v2.scene.json --out /tmp/scene.png` | VERIFIED | Flag at main.rs:74; the scene file exists; scene-file mode prints the `adapter:` line (scenefile.rs:1142), as §2 claims |
| 6 | USER-PC §4 | `--film-telemetry film`; verdict lines `FILM TELEMETRY: CHECK OK` / `CHECK FAILED (…)` | VERIFIED | Flag at main.rs:98, dispatched main.rs:131; format strings verbatim at film.rs:3657/3660 |
| 7 | USER-PC §5 | `--film film --out frames/`; `--from N --to M`; `--to 0` = through the end; resume skips existing frames | VERIFIED | Flags main.rs:102–113; frames written as `frame_{n:05}.png` (film.rs:3451); completion line `film render complete: {done} rendered, {skipped} skipped (resume), range {from}..{to}` (film.rs:3477) |
| 8 | USER-PC §6 | `film/finish.sh FRAMES_DIR MIX_WAV OUT_MP4`; `ffprobe` check | VERIFIED | finish.sh usage line is exactly `FRAMES_DIR MIX_WAV OUT_MP4`; header confirms x264 slow CRF 18, BT.709, `.part` discipline; it consumes `frame_%05d.png` |
| 9 | USER-PC §7 | Env vars `TALERENDER_DOF_DEPTH_FORMAT=halffloat`, `CAP_CHECK=strict`, `TALERENDER_PAINT_CACHE=<n>` (default 160) | VERIFIED | dof.rs:191 (+ scenefile.rs:1608, ab1.rs:835, film.rs:2507); telemetry.rs:1380; film.rs:2113 with `unwrap_or(160)`, FIFO eviction via `pop_front` |
| 10 | USER-PC intro | "There is no `--help` — an unknown argument prints `unknown argument: …` and exits with code 2" | VERIFIED (probed) | `~/talerender-bin/talerender --help` → stderr `unknown argument: --help`, exit 2 |
| 11 | USER-PC §8 | The closing full flag list | VERIFIED | Set-identical to the 14 main.rs match arms |
| 12 | CI.md | `gh workflow run render.yml -f mode=film -f from=0 -f to=120`; `gh run watch` | VERIFIED | render.yml declares inputs `mode`/`from`/`to`/`scene`; `gh run watch` is a gh built-in |
| 13 | CI.md | render.yml input table (mode choice + default `ab1`; from/to strings default `0`; scene default `scenes/character-v2.scene.json`) | VERIFIED | Matches render.yml `workflow_dispatch.inputs` line for line |
| 14 | CI.md | Mode command table (ab1 / dof-chart / film / scene-file invocations) | VERIFIED | Identical to render.yml's Render step `case` block |
| 15 | CI.md | Artifact `render-<mode>-<from>-<to>-<run_id>`, 7-day retention, 120-minute timeout | VERIFIED | render.yml upload step `name: render-${{ inputs.mode }}-${{ inputs.from }}-${{ inputs.to }}-${{ github.run_id }}`, `retention-days: 7`, `timeout-minutes: 120` |
| 16 | CI.md | ci.yml steps: rust-cache `cache-on-failure: true`, key segment `ubuntu2204`, `CARGO_INCREMENTAL=0`, mold via env `RUSTFLAGS`, install mold + mesa-vulkan-drivers + vulkan-tools, `cargo build --release` → `cargo test --release`, smoke render `--width 1080 --height 1920 --out frame.png` with globbed `lvp_icd*.json` | VERIFIED | Every element present in ci.yml as described |
| 17 | PERF §1 | Env-capture block; `adapter:` (scene modes) / `renderer:` (--film) line formats | VERIFIED | Line formats at scenefile.rs:1142 / ab1.rs:593 / main.rs:344 (`adapter: {} \| backend: {:?} \| device type: {:?} \| driver: {} {}`) and film.rs:2343 (`renderer: …`, same shape) |
| 18 | PERF §2 | `--scene ab1 --out …` (×5 runs) | VERIFIED | `ab1` is a `--scene` dispatch value in main.rs |
| 19 | PERF §2 | `--scene-file scenes/film/s01.scene.json --out …` | VERIFIED | Flag main.rs:74; the scene file exists |
| 20 | PERF §3 | `--film film --out … --from 1027 --to 1117`; progress every 25 rendered frames; final line format | VERIFIED | film.rs:3472 `if done % 25 == 0` prints `  frame {n} (t = {t:.2}s) — {done} rendered, {skipped} skipped` — PERF's worked example (frame 1051, t = 35.03 s, 25 rendered) is arithmetically exact for the 1027-start chunk; final line at film.rs:3477 matches verbatim |
| 21 | PERF §4 | `--film-telemetry film` under `/usr/bin/time -v`, default cap and `TALERENDER_PAINT_CACHE=24` | VERIFIED | Flag + env var as rows 6/9 |
| 22 | PERF §5 | Python disk script globbing `frame_*.png` | VERIFIED | Matches the driver's `frame_{n:05}.png` naming (film.rs:3451) |
| 23 | PERF §7 | Bad-invocation signature: exit 2, `unknown argument:` / `unknown scene:` / `unknown --post-stage:` | VERIFIED | All three strings are main.rs error paths; exit 2 probed (row 10) |
| 24 | PERF §7 | Telemetry-FAILED signature: exit 0 + `FILM TELEMETRY: CHECK FAILED (<n> overflow groups, <n> continuity violations, <n> caption failures)` | VERIFIED | Format string at film.rs:3660; exit-0 behaviour at main.rs:131–139 (see §3.4) |
| 25 | CI.md | Dev-VM / CI division: ci.yml and render.yml both pin `ubuntu-22.04`, CI renders on lavapipe (Mesa 23.2) | VERIFIED | `runs-on: ubuntu-22.04` in both; render.yml header states Mesa 23.2 lavapipe |
| 26 | USER-PC §2 | "The binary prints its adapter before every render", example `renderer: AMD Radeon … \| backend: Vulkan \| device type: IntegratedGpu \| driver: radv …` | VERIFIED | Format at film.rs:2343 produces exactly this shape; field order and separators match |
| 27 | PERF §6 | Chrome-side comparison uses the skill's `capture.mjs` with `CHROME_FLAGS=…` | VERIFIED (as protocol text) | Not a repo command — it describes the skill-side route and is labelled as such; nothing in the repo contradicts it |

No runbook text required correction in this finisher pass: the two
landed diffs (§1) were the only drift, and both are accurate as
written.

## 3. PERF.md §9 feature requests — implementation state in `src/`

PERF §9 lists five items (four requests + one named blind spot).
State at HEAD, one line each:

1. **Elapsed seconds on progress lines — NOT implemented.** No
   occurrence of `elapsed` or `Instant` anywhere in `src/`; the
   film progress line (film.rs:3473) carries frame, t, and counts
   only.
2. **Paint-cache statistics — NOT implemented.** No hits / misses /
   evictions counters or end-of-run cache line exist; the only
   `*stats*` symbol in `src/` is `composer::room_env_stats`
   (environment-lighting statistics, unrelated).
3. **Peak-RSS self-report — NOT implemented.** No `getrusage` /
   `ru_maxrss` / peak-RSS reporting anywhere in `src/`; memory
   measurement still depends on `/usr/bin/time -v` wrapping, as
   PERF §4 assumes.
4. **Film-telemetry exit codes — NOT implemented.** main.rs:131–139
   runs `film::run_film_telemetry`, prints any error to stderr, and
   returns from `main` — exit 0 on pass, fail, and error alike, so
   §4/§7 must grep the verdict line, exactly as PERF states. (The
   per-shot `--telemetry` modes do exit via
   `std::process::exit(telemetry::run(…))`, consistent with §9.4's
   remark.)
5. **GPU memory figure (named blind spot) — NOT implemented.** The
   `renderer:`/`adapter:` format strings print name, backend,
   device type, and driver only — no memory field exists to print.

## 4. Counts

- Landed diffs audited: **2 — both CORRECT** (0 wrong, 0 fixes
  needed).
- Per-command / per-claim rows: **27 — 27 VERIFIED, 0
  DRIFTED-fixed, 0 UNVERIFIABLE-HERE.** (One row verified by probing
  the provisioned binary; the rest from source/workflow text at
  HEAD.)
- PERF §9 items checked: **5 — 0 implemented, 5 still open** (the
  four requests and the blind spot all stand as PERF.md describes
  them; PERF.md does not claim any of them exists).
- Runbook edits made by this finisher: **none** — the W95 fixes
  already on disk were accurate, and no further error was found.
