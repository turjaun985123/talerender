<!--
DELIVERY NOTE — Chrome Replacement, Batch 2, Seat B2-03 (native selftest).

Files created (this seat, NEW files only; nothing existing edited,
nothing committed in either repo):
  ~/workspace/chrome-replacement/batch2/B2-03/selftest.sh
      (staged replacement for the skill's scripts/selftest.sh —
      the lead swaps it in at integration)
  ~/workspace/chrome-replacement/batch2/B2-03/selftest-run1.out
  ~/workspace/chrome-replacement/batch2/B2-03/selftest-run2.out
      (captured runs on this VM; see §3)
  docs/integration/CR-B2-03-SELFTEST.md          (this report)

Contracts honoured: CHROME-REPLACEMENT-CONTRACTS.md §1 row 11
(selftest: "the native path becomes the test's subject end-to-end;
the RENDERER guard is inverted/removed") and row 16 (the opt-in
native smoke is absorbed — its legs are folded in as leg 3), §3
gates (LIBRARY OK 479 asserted as a leg), §4 item 4 (failure modes
fail loudly with remedies: unbridged project, missing binary,
era/timeout classifications). No quality reduction: the video leg
uses the adapter's own draft profile (x264 veryfast CRF 23,
CAPTURE-NATIVE.md) exactly as the old selftest used capture.mjs's
--draft; the master profile is untouched and is not a selftest
subject.
-->

# CR-B2-03 — Native selftest

## 1. Leg audit of the existing selftests

Classification key: **CHROME** = launches/drives a browser;
**NATIVE** = TaleRender binary path; **CPU** = no browser, no GPU
(backend-agnostic tooling).

### 1a. `scripts/selftest.sh` (48 lines, the v6 selftest)

| # | Leg | Class | Evidence |
|---|-----|-------|----------|
| 1 | Typecheck gate: `tsc -p template/tsconfig.json`, hard exit 1 if missing/failing | CPU | `scripts/selftest.sh:11-13` |
| 2 | Scaffold: `new_video.sh tall-tale-selftest` | CPU | `:14` |
| 3 | Build: `bun run build`, fallback to the template's shipped prebuilt `build/` | CPU | `:16-18` |
| 4 | Voice: `voice.py` + `words.py` (or `estimate.py` under `--no-voice`) | CPU (Kokoro/Whisper; never a browser) | `:22` |
| 5 | `capture.mjs check` + `grep "CHECK OK"` — boots headless Chrome, evaluates `OP.*` in the page | **CHROME** | `:29` (capture.mjs `openPage()` spawns Chrome) |
| 6 | `capture.mjs cues` → `tools/mix.py` — cues extracted by page evaluation; mix itself is numpy | **CHROME** (cues) + CPU (mix) | `:30` |
| 7 | `capture.mjs sheet review/all.jpg 1 8` | **CHROME** | `:31` |
| 8 | `capture.mjs video renders/selftest.mp4 2 6 --draft` | **CHROME** | `:32` |
| 9 | Native smoke hook: runs `tools/selftest_native.sh` **only if `RENDERER=native`**, immediately before the verdict; exit 1 fails the selftest, exit 3 is noted and waived | NATIVE (opt-in, guarded) | `:34-41` |
| 10 | Clip loudness proxy: ffmpeg `volumedetect` mean volume > −50 dB on the Chrome-rendered clip → `SELFTEST PASSED` | CPU (on a CHROME product) | `:43-47` |

Net: 4 of 10 legs are Chrome-bound, and they are the pipeline's
core (check/cues/sheet/video). The native path appears only as a
guarded afterthought (leg 9).

### 1b. `tools/selftest_native.sh` (228 lines, the opt-in smoke)

| # | Leg | Class | Evidence |
|---|-----|-------|----------|
| 0 | Gate: no-op SKIP (exit 0) unless `RENDERER=native` **and** `TALERENDER_BIN` set | — | `tools/selftest_native.sh:99-107` |
| 1 | `talerender --telemetry ab1` → exit 0 + exact line `CHECK OK` | NATIVE/CPU (telemetry creates no GPU) | telemetry step table in header, `:37-45` |
| 2 | `--telemetry ab1-variant` → exit 1 + `CHECK FAILED` (gate proven in the failing direction) | NATIVE/CPU | same |
| 3 | `--telemetry ab1-file --scene-file <fixture>` → exit 0 + `CHECK OK` | NATIVE/CPU | same |
| 4 | `--scene-file <fixture> --out ab1.png` → true PNG at exactly 1080×1920 (clean-frame contract, decision (B)) | NATIVE/**RENDER** | header `:46-55`; exit-code table: 0 pass/skip, 1 FAIL, 3 ENV-BLOCKED |

This script is a *binary* smoke (one staging, one fixture, one
frame) — it never touches a skill project, a bridge, cues, mix or
metadata. Under the replacement it is absorbed, not extended:
its legs 1–3 become leg 3 of the new selftest; its leg 4 is
subsumed by the new render canary (leg 0) + frames/sheet/video
legs against a real bridged film.

## 2. The new selftest (staged at `B2-03/selftest.sh`)

Design decisions, each forced by code evidence:

- **No RENDERER guard, no Chrome anywhere.** Every capture runs
  `capture_native.mjs` with `RENDERER=native` set and `CHROME`
  explicitly unset. Exit codes keep the native discipline:
  0 PASS / 1 FAIL / 3 ENV-BLOCKED (a blocked run is never a pass).
- **Fixture.** The template film has **no transcribed scenes**,
  and `build_bridge.py` assembles a bridge only from transcribed
  scenes (it dies: "no *.scene.json files in … — it does not
  transcribe shots.ts itself"). The only completely transcribed
  film is Onoda (repo `scenes/film/`, `film/assets/`,
  `film/onoda.json`; timing/voice in the Onoda project). The
  selftest therefore scaffolds a **fresh template project**
  (`new_video.sh` — engine/tools/fonts are the shipped
  template's), overlays the Onoda film's `script.json` +
  `data/lines.json` + `data/words.json` + `data/narration.wav`
  as fixture data, and bridges it with the repo builder. The
  film is fixture, exactly as the mailed-grandma film was for
  the Chrome selftest; the subject is the skill's native
  tooling. A missing fixture project degrades bridge-dependent
  legs to ENV-BLOCKED (provisioning), never a fake pass.
- **No voice leg.** Timing is an *input* to the native chain;
  the fixture carries recorded timing from a real voiced
  production. Voice/words are NOT-CHROME by design (contracts
  §1 row 20) and unchanged by this migration.
- **Legs** (each fails loudly and names its remedy):
  0 env probe (node/ffmpeg/tsc/python-env hard gates; binary
  discovery + `--telemetry` era probe; **render canary** — one
  `--scene-file` ab1 frame; its failure classifies all render
  legs ENV-BLOCKED up front) · 1 typecheck · 2 `library_check.py`
  asserting the exact line `LIBRARY OK (479 items)` · 3 binary
  telemetry both directions (folded-in smoke) · 4 scaffold +
  build + fixture overlay · 5 cues **negative** leg (unbridged
  project must exit 2 naming the bridge) · 6 bridge build via
  `tools/bridge/build_bridge.py`, its three verification legs
  classified separately (see finding F1) · 7 native `check`
  (`CHECK OK`, exit 3 = ENV-BLOCKED era, timeout = ENV-BLOCKED)
  · 8 cues (adapter routing assertion: `capture_native.mjs cues`
  exits 1 naming `cues_native.py`; then the emitter itself) ·
  9 frames (2 PNGs) · 10 sheet · 11 castsheet (one kind,
  `SELFTEST_CAST_KIND`, default `mood` — the full 77-item
  catalog is the library-preview job, not a selftest) ·
  12 video **draft profile** (default range 2–6 s, the old
  selftest's range; asserts MP4 + ffprobe duration + the P3
  sidecar `<out>.render.json`) · 13 mix (asserts mix.py's own
  `loudness: PASS`) · 14 metadata gate (two-pass shelf loop,
  §3d).
- **Ordering note (leg 12 before 13)** follows the contract
  chain literally: the draft clip is therefore rendered before
  `data/mix.wav` exists and is silent by design ("SILENT: no
  data/mix.wav"); the sound gate lives in leg 13. Production
  order (mix before master) is a SKILL.md matter.
- **Bridge data vs verdict are gated separately.** Legs 8/13
  consume bridge *data* (builder leg (a) + files on disk);
  legs 7/9–12 consume the binary's *verdict* on that data
  (builder leg (c)). A slow machine can thus still prove the
  data chain without faking the verdict chain.
- Per-leg expected durations are documented in the script
  header; render/telemetry legs are timeout-bounded
  (`SELFTEST_BRIDGE_TIMEOUT`, `SELFTEST_RENDER_TIMEOUT`,
  default 1800 s each) with timeout = ENV-BLOCKED.
- `SELFTEST_SKILL` override exists so a staged copy can be
  verified against the real skill tree (used for the runs in
  §3); installed as `scripts/selftest.sh` it defaults correctly.

## 3. What was actually run here (this VM, 2026-11-15)

Environment facts (measured): **no Vulkan ICD at all**
(`/usr/share/vulkan/` does not exist; `/dev/dri` absent);
the binary's own canary error is "no wgpu adapter found — a
Vulkan driver (hardware or lavapipe) is required". Provisioned
binary: `~/talerender-bin/talerender`, **talerender 1.0.1**
(pre-Batch-1; era consequences in §4). Node v24.22.1, Bun
1.4.4, ffmpeg 8.1.2, Python 3.12.3 + skill venv (numpy/scipy OK).

### 3a. Full staged run — `selftest-run2.out` (438 s, exit 3)

Command: `SELFTEST_SKILL=<skill> SELFTEST_BRIDGE_TIMEOUT=360
bash selftest.sh --keep`. Verdict: **SELFTEST ENV-BLOCKED
(exit 3)** — the designed verdict for this machine. Per leg:

| Leg | Result |
|---|---|
| 0 probe | binary found (1.0.1, era probe OK); render canary **ENV-BLOCKED** (no wgpu adapter) |
| 1 typecheck | **PASS** (also standalone: 3.8 s, rc 0) |
| 2 library | **PASS** — `LIBRARY OK (479 items)` (standalone: 2.7 s, rc 0) |
| 3 telemetry ×3 | **PASS** — ab1 `CHECK OK` rc 0; ab1-variant `CHECK FAILED` rc 1; ab1-file `CHECK OK` rc 0 |
| 4 scaffold+build+overlay | **PASS** |
| 5 cues negative | **PASS** — unbridged emitter exit 2, bridge named (also standalone, §3c) |
| 6 bridge | **ENV-BLOCKED** — builder timed out at 360 s in its telemetry verification leg (c); builder leg (a) **PASS** parsed from the unbuffered partial output; bridge data built on disk |
| 7 check | **ENV-BLOCKED** (cascade: no verified bridge verdict) |
| 8a cues routing | **PASS** — adapter exit 1 naming `cues_native.py` |
| 8b cues emitter | **PASS** — `data/cues.json`: **9 cues, duration 60.357** |
| 9–12 frames/sheet/castsheets/video | **ENV-BLOCKED** (no adapter; leg-0 canary is the evidence) |
| 13 mix | **PASS** — `loudness: PASS` ("cues: 9 placed, 9 over voice") on the native cues + fixture narration |
| 14 metadata | **PASS** — `METADATA: PASS` (see §3d) |

Zero FAILs. (`selftest-run1.out` is the same script run from
its staged path without `SELFTEST_SKILL`: it correctly failed
leg 0 against the wrong tree — the staged-location artifact
that motivated the override, kept as evidence.)

### 3b. Bridge builder, standalone (fast legs)

`build_bridge.py --project <fixture> --scenes <repo>/scenes/film
--assets-dir <repo>/film/assets --film-data <repo>/film/onoda.json
--no-telemetry`: completes in **2.8 s**. Leg (a) P4 adapter
semantics **PASS**; leg (b) repo validator **FAIL** — see
finding F1 (3 of its 4 problems are phantom plates; the 4th,
missing `src/shots.ts`, is an artifact of the minimal probe
fixture — the real scaffold has the template's `src/`).
Leg (c) skipped by flag in this run.

### 3c. Cues parity, standalone

Against a bridge built by `build_bridge.py`, `cues_native.py`
writes **9 cues, duration 60.357 s** — the contracts §1 row 5
parity fixture is the Onoda project's Chrome-produced
`data/cues.json`, and the native emitter matches its cue count
(9) and the film duration (60.357, the lines duration, not the
60.46 mix duration). First cues: `{t 0.5, chirp}`, `{t 21.884,
whoosh}`, `{t 31.17, tock}`. Unbridged: exit 2 with the bridge
remedy, verbatim in §1b's successor leg 5.

### 3d. Metadata gate, standalone (the leg-14 mechanics)

Synthetic 1080×1920 test-card cover (ffmpeg `testsrc2`) +
authored fixture metadata: pass 1 → exactly one FAIL (shelf
attestation) and the gate emits the 180×320 shelf preview;
dimension-verify, attest, pass 2 → **`METADATA: PASS`,
rc 0**, all 11 checks green. The selftest's attestation covers
gate mechanics on a synthetic cover — it does not claim a
human shelf read, and says so in its own output line.

### 3e. Film telemetry cost (the leg 6c / leg 7 long pole)

Standalone `talerender --film-telemetry <built bridge>/film`
with the 1.0.1 binary: **did not complete within 280 s**
(84 CPU-seconds consumed; `timeout` rc 124), nor within
360 s inside the builder, nor in ~11 min under contention
(an earlier builder run was SIGTERM'd by the runtime at
714 s with the telemetry leg still running; its buffered
output was lost — the staged selftest therefore runs the
builder with `python3 -u`). The binary's progress lines are
block-buffered when redirected (last flushed line lagged at
"sample 20 (t=4.75)"), so no exact samples/min rate is
claimed. Conclusion: full-film telemetry on this VM class is
a tens-of-minutes operation; CI (where contracts §3 records
`FILM TELEMETRY: CHECK OK` at render-path landings) is the
reference environment. This is a provisioning fact, not a
verdict against the binary.

## 4. What remains UNVERIFIED-ENV here (proving ground: CI lavapipe / RADV PC)

- **Legs 9–12 (frames, sheet, castsheet, video draft)** —
  never executed against a renderer here; statically verified
  only (`bash -n` clean on the staged script; dispatch traced
  through `capture_native.mjs` + `native_modes/mode_dispatch.
  mjs`: castsheet → `castsheet_mode.mjs`, sheet → `sheet_mode.
  mjs`, check → `check_mode.mjs`). The adapter code paths
  themselves are Batch 1's, already landed.
- **Leg 7 (native check)** — not run to a verdict: it needs
  the leg-6(c) telemetry verdict first, and its own audit is
  the same long pole (§3e). Era sub-risk, named not hidden:
  `check_mode.mjs` invokes `--film-telemetry --tables`; the
  1.0.1 binary accepts `--film-telemetry` (its error on the
  stale packaged data was a data error, exit 3 — not "unknown
  argument"), but whether it honours `--tables` could not be
  confirmed without a telemetry run that completes. The
  selftest classifies a check exit 3 as ENV-BLOCKED (era),
  which is where a `--tables` rejection would land.
- **Leg 6(c) verdict** on the freshly built bridge — same
  long pole. The bridge *data* is verified (leg (a) PASS;
  consumed successfully by the cues emitter and mix).
- **Binary era, summary.** The provisioned 1.0.1 binary
  predates Batch 1. It fully supports `--telemetry` (leg 3
  passes both directions) and knows `--film-telemetry`; it
  cannot be the proving binary for Batch 1 adapter behaviour.
  Nothing in the runs above fakes a post-Batch-1 result.

## 5. Findings — behaviour the selftest exposed as broken/divergent

**F1 — `validate_native_project.py` misparses the current
plates.json shape (repo tooling divergence; the bridge builder's
leg (b) can therefore never pass a current bridge).**
`tools/integration/validate_native_project.py:513-519` iterates
the plates.json dict's **keys** as plate names (its comment,
line ~86: "The hold plates come from plates.json keys" — the
legacy flat `{name: [x, y]}` shape). Against the current
manifest `{format, version, plates: [{name, file, origin}]}`
— the shape `build_bridge.py` emits, the repo's own
`film/assets/plates.json` carries, and the binary's loader
demands (CR-B1-06 §1: `film.rs load_assets` rejects the legacy
flat shape by name) — the validator invents phantom plates
`format.png`, `plates.png`, `version.png` and fails the build
(measured, §3b). The validator can only pass **stale**
bridges. The staged selftest handles this explicitly: a leg-(b)
failure whose only problems are the three phantoms is reported
as a NOTE; any other (b) problem is a FAIL. **Remedy (repo
side): teach the validator the current manifest shape (read
`plates[].file`), or retire leg (b).**

**F2 — The packaged Onoda project bridge is stale in the
binary's eyes (corroborates CR-B1-06's lead finding).**
`talerender --film-telemetry ~/faceless-shorts/the-soldier-
who-didnt-know/native/film` (1.0.1 binary) → exit 3,
"film data: missing calendarYears". Its `native/film/
onoda.json` keys lack `calendarYears`/`calendarDark`/`sign`
(the repo's `film/onoda.json` has all three) and its
`plates.json` is the legacy flat shape (F1). The Batch 2/3
P1 retrofit must **rebuild** this project's bridge with
`build_bridge.py`; copying the packaged one forward
propagates both stalenesses.

**F3 — `tools/mix.py` still gives a Chrome-era remedy.**
`template/tools/mix.py` (header/`_load_json` hint, ~line 29):
a missing `data/cues.json` tells the operator to run
``node tools/capture.mjs cues``. In the native flow the route
is `python3 tools/cues_native.py` (the adapter itself prints
exactly that). Minor doc debt in a shipped tool; the selftest
does not depend on the hint, but a fresh operator following it
would resurrect Chrome. **Remedy: update the hint text when
the template is next touched (Batch 2 template seat).**

**F4 — No bridgeable fresh-scaffold film exists.**
`build_bridge.py` requires transcribed scenes; the skeleton
exporter (`tools/export/export_project_v2.mjs`) emits a
`talerender-skeleton/1` report whose `owes` list is precisely
the untranscribed remainder. Until a transcription step lands,
*any* end-to-end native selftest must fixture on a transcribed
film (this seat used Onoda). The Batch 3 fresh-scaffold proof
seat will hit the same wall and should plan for it (its
"complete NEW short" needs scenes authored as data, not only
`shots.ts`).

**F5 — `/tmp` on this VM is reaped mid-work.**
A staged fixture under `/tmp` vanished between commands
during this seat's investigation (one reason an early bridge
run's evidence is incomplete). The staged selftest follows
its predecessor in working under `~/faceless-shorts/`
(persistent); noted here so other seats don't re-learn it.

## 6. Integration notes for the lead

- Swap `B2-03/selftest.sh` → `scripts/selftest.sh` as-is.
  `tools/selftest_native.sh` + `tools/SELFTEST-NATIVE.md` can
  retire with it (their legs live on as leg 3; contracts §1
  row 16 "absorbed").
- First full PASS verdict requires a machine with a Vulkan
  ICD **and** a post-Batch-1 binary: run on the TaleRender CI
  render workflow or the user's RADV PC with
  `TALERENDER_BIN` pointed at a current CI artifact. Expect
  ~15–35 min on lavapipe, a few minutes on RADV (header table).
- On such a machine, also confirm the `--tables` era question
  (§4) resolves to a clean leg 7 — if the current binary
  prints `CHECK OK`, the era note in leg 7's blocked message
  can be dropped at integration.
- `SELFTEST_CAST_KIND=held` is the one castsheet variant that
  can partially skip (held props need pre-rendered plates;
  `castsheet_mode.mjs` names skips) — the default `mood`
  avoids that surface in the selftest.
