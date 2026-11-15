# CR-B3-04 — Failure-mode tests of the shipped native adapter + doctor

Chrome-replacement wave, Batch 3, Seat 04. The shipped Tall Tale Shorts
skill must fail **loudly and usefully** when things are wrong. This note
records what the shipped code actually does in each failure mode, with
the verbatim stderr and exit code of every run.

- **Code under test (shipped, unmodified):**
  - adapter `~/workspace/custom-skills/tall-tale-shorts/template/tools/capture_native.mjs`
    (sha256 `5d7c350024cd94fdbc53216b14c38d83f5c94919e1e8ac08bb2c17fe64e1ed97`,
    verified identical in every fixture that carried it)
  - doctor `~/workspace/custom-skills/tall-tale-shorts/template/tools/project_doctor.py`
    (run from the skill path; the project is its argument — it is read-only)
  - validator `tools/integration/validate_native_project.py` (this repo, read-only)
  - binary `~/talerender-bin/talerender` — `talerender 1.0.1`
- **Harness:** `~/workspace/chrome-replacement/batch3/B3-04/run_failmodes.sh`
  (re-runnable; rebuilds all fixtures from source each run).
- **Full outputs:** `~/workspace/chrome-replacement/batch3/B3-04/outputs/` —
  per case: `.cmd` (exact command), `.stdout`, `.stderr`, `.exit`, plus
  `SUMMARY.txt`, `GREP-AUDIT.txt`, `0-00-fixtures.txt`.
- **Fixtures (all scratch copies under `…/B3-04/work/`):**
  - `bridged/` — copy of the B2-08 retrofitted Onoda copy (current-generation
    bridge) + fresh shipped-tools overlay.
  - `stale/` — full copy of the ORIGINAL project
    `~/faceless-shorts/the-soldier-who-didnt-know` (packaged bridge keys:
    `anchors, captions, duration, film, fps, frames, lines, project` — no
    F2 keys) + fresh shipped-tools overlay.
  - `corrupt-field/` — `bridged/` with the `captions` field deleted from
    `native/film/onoda.json`.
  - `corrupt-trunc/` — `bridged/` with `native/film/assets/cap_000.png`
    truncated from 7370 → 16 bytes.
  - `scaffold-parent/b3-04-unbridged/` — fresh `scripts/new_video.sh`
    scaffold; its `native/BRIDGE-STATUS.txt` reads `UNBRIDGED` (captured
    in `outputs/3-00-bridge-status.txt`).
- **Non-mutation proof:** sha256 snapshots of the original project
  (262 files) and the retrofitted copy (273 files) taken before and
  after the whole run are **byte-identical**. Doctor/validator/binary
  probes against the original were read-only. Nothing was committed.

**Verdict rule (charter):** PASS = the error is loud, names the remedy,
and exits non-zero with the documented code (adapter: 1 = capture
failed / prerequisite missing, 2 = usage/selection, 3 = `check`
telemetry could not run — `tools/CAPTURE-NATIVE.md` "Exit codes";
doctor: 2 = a diagnosis failed; validator: 2 = validation failed;
binary: 3 = telemetry could not run, EXIT-CODES-W02). Anything else is
FAIL, with the verbatim output in the case file.

**Result: 25 PASS / 6 FAIL of 31 runs** (plus 2 healthy controls, both
PASS, counted in the 31). The six FAILs cluster into three real gaps,
detailed after the table: (i) `check` mode never prints a remedy when
telemetry cannot run; (ii) the adapter's gate mis-directs a stale
*bridge* toward the binary-provisioning remedy; (iii) two by-design
behaviours that contradict a strict reading of the charter — doctor
`INCONCLUSIVE` exits 0, and `castsheet` succeeds on an unbridged
project.

## Verdict table

| Case | Fault → surface | Exit (documented) | Verdict |
|---|---|---|---|
| 0a-doctor-bridged-noprobe | control: healthy bridged fixture → doctor `--no-probe` | 0 (0) | PASS — `BRIDGE: current generation`, `DOCTOR: OK` |
| 0b-validator-bridged | control → validator | 0 (0) | PASS — `RESULT: OK (0 flagged)` |
| 1a-missing-explicit-adapter-check | `TALERENDER_BIN=/nonexistent/…` → adapter `check` | 1 (1) | PASS |
| 1b-missing-discovery-adapter-check | no binary discoverable (fake `HOME`, PATH=`/usr/bin:/bin`) → adapter `check` | 1 (1) | PASS |
| 1c-missing-doctor | same isolation → doctor | 2 (2) | PASS |
| 2a-fake-versiononly-adapter-check | fake answers `--version`, no `--film-telemetry` → adapter `check` | 3 (3) | **FAIL** — loud, right code, **no remedy printed** |
| 2b-fake-versiononly-adapter-frames | same fake → adapter `frames` (gate) | 1 (1) | PASS |
| 2c-fake-unknownarg-adapter-check | fake exits 2 `unknown argument: --film-telemetry` → adapter `check` | 3 (3) | **FAIL** — symptom shown verbatim, **no remedy printed** |
| 2d-fake-unknownarg-adapter-frames | same fake → adapter `frames` (gate) | 1 (1) | PASS — era-mismatch symptom + full provisioning remedy |
| 2e-fake-unknownarg-doctor | same fake → doctor probe | 2 (2) | PASS — `PROBE: STALE BINARY` + provisioning remedy |
| 2f-real-stale-repo-adapter-frames | REAL stale binary `~/workspace/talerender/talerender` → adapter `frames` | 1 (1) | PASS — identical era-hint surface as 2d |
| 2g-fake-versiononly-doctor | fake-version-only → doctor probe | **0** (2 expected) | **FAIL** — `PROBE: INCONCLUSIVE` … `DOCTOR: OK` |
| 3a-unbridged-check | fresh UNBRIDGED scaffold → adapter `check` | 1 (1) | PASS |
| 3b-unbridged-frames | → adapter `frames` | 1 (1) | PASS |
| 3c-unbridged-sheet | → adapter `sheet` | 1 (1) | PASS |
| 3d-unbridged-video | → adapter `video` | 1 (1) | PASS |
| 3e-unbridged-castsheet-look | → adapter `castsheet review/cast look` | **0** | **FAIL** — no error at all; sheet rendered (by design, see Finding 3) |
| 3f-unbridged-doctor | → doctor `--no-probe` | 2 (2) | PASS — `BRIDGE: UNBRIDGED` + bridge-builder remedy |
| 4a-binary-direct-original | provisioned binary `--film-telemetry` on ORIGINAL film dir (read-only) | 3 (3) | PASS |
| 4b-stale-adapter-check | stale bridge (scratch copy) → adapter `check` | 3 (3) | **FAIL** — refusal named (`missing calendarYears`), **no re-bridge remedy printed** |
| 4c-stale-adapter-frames | stale bridge → adapter `frames` (gate) | 1 (1) | **FAIL** — gate refuses, but the printed remedy is the *binary-provisioning* one, not re-bridge |
| 4d-stale-doctor-original | stale bridge → doctor on ORIGINAL (read-only, with probe) | 2 (2) | PASS |
| 4e-stale-doctor-scratch | stale bridge → doctor on scratch copy | 2 (2) | PASS — identical to 4d except the project path line |
| 4f-stale-validator-original | stale bridge → validator on ORIGINAL (read-only) | 2 (2) | PASS |
| 4g-stale-validator-scratch | stale bridge → validator on scratch copy | 2 (2) | PASS — identical to 4f except path + the adapter-presence FLAGGED line |
| 5a-renderer-chrome-check | `RENDERER=chrome` → adapter `check` | 2 (2) | PASS |
| 5b-renderer-chromium-frames | `RENDERER=chromium` → adapter `frames` | 2 (2) | PASS |
| 5c-renderer-native-control | `RENDERER=native` + bad `TALERENDER_BIN` → adapter `check` | 1 (1) | PASS — guard passed; the binary error behind it fired |
| 5d-renderer-unset-control | `RENDERER` unset + bad `TALERENDER_BIN` → adapter `check` | 1 (1) | PASS — same as 5c |
| 6a1-corrupt-field-adapter-check | `captions` field deleted → adapter `check` | 1 (1) | PASS — `onoda.json: captions[] missing/empty` + "Regenerate the bridge data" |
| 6a2-corrupt-field-validator | → validator | 2 (2) | PASS — field named + MANIFEST-SHA drift named, re-bridge remedy |
| 6a3-corrupt-field-doctor | → doctor `--no-probe` | 2 (2) | PASS — SHA drift on `film/onoda.json` named + re-bridge remedy (doctor names the drift, not the field) |
| 6b1-corrupt-trunc-adapter-check | `cap_000.png` truncated to 16 B → adapter `check` | 3 (3) | **FAIL** — binary names file + `unexpected end of file`, `CHECK FAILED`, right code, **no remedy printed** |
| 6b2-corrupt-trunc-validator | → validator | 2 (2) | PASS — caught via MANIFEST-SHA (`film/assets/cap_000.png (hash differs)`), re-bridge remedy; see note below |
| 6b3-corrupt-trunc-doctor | → doctor `--no-probe` | 2 (2) | PASS — SHA drift on the exact file named + re-bridge remedy |

## Verbatim evidence (key cases; all 31 in `outputs/`)

### 1 — Missing binary

1a stderr (exit 1):

```
capture-native: TALERENDER_BIN="/nonexistent/b3-04/talerender" is not an executable file. An explicit TALERENDER_BIN is never second-guessed — fix the path or unset it to use discovery.
```

1b stderr (exit 1) — discovery exhausted with a fake HOME and a clean PATH:

```
capture-native: talerender binary not found. Tried: …/work/bridged/native/talerender, …/work/fakehome/workspace/talerender/talerender, then PATH.
Set TALERENDER_BIN=/path/to/talerender — the binary is the "talerender-binary" artifact from a green CI run of turjaun985123/talerender (there is no local build).
```

1c doctor (exit 2) prints `BINARY: MISSING — no talerender binary found
(tried: …)` followed by the full provisioning remedy (`gh run list` /
`gh run download … -n talerender-binary -D ~/talerender-bin` /
`export TALERENDER_BIN=~/talerender-bin/talerender`) and the
repo-checkout-slot warning, ending `DOCTOR: FAILED (1 diagnosis/diagnoses)`.

### 2 — Stale binary

2d stderr (exit 1) — the gate at its best; 2f (the **real** stale repo
binary) produces the identical text with its own path:

```
unknown argument: --film-telemetry
capture-native: film telemetry produced no verdict line (binary exit 2) — treated as a FAILED gate.
Symptom: era mismatch — the talerender binary (via TALERENDER_BIN) predates the film gate: it does not recognise --film-telemetry (a current binary answers the telemetry call with a "FILM TELEMETRY: CHECK OK / CHECK FAILED" line; this one produced none). The binary and the project's bridge data must come from compatible repo states; a stale binary renders nothing rather than rendering wrong frames.
Remedy: Provision a current binary from a green CI run of turjaun985123/talerender and point TALERENDER_BIN at it:
  gh run list --repo turjaun985123/talerender --workflow CI --limit 5
  gh run download <run-id> --repo turjaun985123/talerender -n talerender-binary -D ~/talerender-bin
  chmod +x ~/talerender-bin/talerender
  export TALERENDER_BIN=~/talerender-bin/talerender
Do not rely on the repo-checkout discovery slot (~/workspace/talerender/talerender): it holds whatever build happens to sit in the checkout and was a stale pre-gate build in every local run of the integration wave.
Binary used for this run: …/work/fakebin/fake-unknown-arg
Then re-run the same command. There is no flag to render past a failed gate.
```

2b (fake-version-only, gate) also PASSes: the generic era-hint branch
hedges correctly — "One other named cause is an era mismatch… Verify
currency before re-running" — and still prints the provisioning remedy.

**Finding 1 — `check` mode prints no remedy when telemetry cannot run
(2a, 2c FAIL).** Both check runs are loud and exit 3 (the documented
code), and stdout ends `CHECK FAILED`, but the entire stderr is:

2a: `check_mode: film telemetry produced no verdict line — treated as a FAILED gate (see stderr above)`
(the fake's own line went to the binary's *stdout*, which `check`
parses and does not echo on the no-verdict path — "see stderr above"
points at a stream that does not contain it).

2c: `unknown argument: --film-telemetry` (the binary's own stderr,
forwarded) + the same `check_mode:` line. The symptom is visible; the
`TALERENDER_BIN` provisioning remedy that `frames`/`video` print via
`era_hint.mjs` is never printed by `check`
(`native_modes/check_mode.mjs` no-verdict path). An operator whose
first contact is `check` — the documented QA gate — gets no next step.

**Finding 2 — doctor `INCONCLUSIVE` exits 0 (2g FAIL).** Against the
fake that answers `--version` but produces no verdict and exit 0,
doctor prints (stdout, exit **0**):

```
  PROBE: INCONCLUSIVE — no verdict line (exit 0): fake-version-only: telemetry not implemented — a stale binary is one named possibility, not asserted; if the binary predates --film-telemetry the remedy is: provision a current binary (see BINARY remedy)
DOCTOR: OK
```

The hedging is principled (stale is asserted only on positive
evidence — the era-hint doctrine), but the *verdict* is wrong for a
doctor: a binary that cannot produce a telemetry verdict at all is
not OK, and exit 0 will pass any scripted `doctor && capture` chain.
The adapter never makes this mistake (2a/2b fail closed on the same
binary). Note the probe text also says "(see BINARY remedy)" although
no BINARY remedy block is printed in this run.

### 3 — Unbridged project

3a–3d PASS with **byte-identical** stderr (md5
`2d19b3cb19ecf76a123f8b09447e3b42` for all four modes), exit 1:

```
capture-native: native film manifest not found. Tried: …/native/film.json and …/native/scenes/film/film.json.
The native backend captures only projects bridged to film data; a project without it is not bridged yet — bridge it first, then re-run. Build the bridge from the project's own data with the bridge builder (tools/bridge/build_bridge.py in the talerender repo); for film data the repo already holds, package it instead (tools/bridge/package_bridge.py, same repo). See tools/CAPTURE-NATIVE.md for the layout. There is no other capture backend to fall back to.
film data dir not found: no onoda.json in …/native/film or …/native. The binary's film data (onoda.json + assets/) is generated by the bridge, never improvised at capture time. Bridge the project first — build the bridge from the project's own data with tools/bridge/build_bridge.py, or package film data the repo already holds with tools/bridge/package_bridge.py (both in the talerender repo) — then re-run. See tools/CAPTURE-NATIVE.md for the layout.
```

Doctor (3f, exit 2): `BRIDGE: UNBRIDGED — no native film manifest
(native/film.json). The native backend refuses unbridged projects.` +
the `build_bridge.py` remedy.

**Finding 3 — `castsheet` on an unbridged project succeeds (3e FAIL
against the charter's letter).** Verbatim stdout (exit 0):

```
capture-native: castsheet via /home/hatch/talerender-bin/talerender (via argument)
castsheet_native: 5 items (0 plate-missing) -> …/b3-04-unbridged/review/cast/.native-castsheet
look-kid.jpg
look-adult.jpg
look-tall.jpg
look-elder.jpg
look-stocky.jpg
…/b3-04-unbridged/review/cast/castsheet.jpg (5 items, kind=look)
```

This is **by design**, not a bug in the mode: `castsheet` is dispatched
before binary/manifest validation (`mode_dispatch.mjs`) because it
stages its own scene data and never reads the film bridge — the
contract in `tools/CAPTURE-NATIVE.md` says exactly that. But the
charter for this seat demanded every mode's error name the bridge
remedy on an unbridged project, and castsheet produces no error.
Recorded as FAIL so the wave lead can rule: either the charter
exempts castsheet explicitly, or castsheet should print a
"project is UNBRIDGED — castsheet does not use the bridge" notice.
Bounded kind `look` (5 items) was used so the run stays finite; the
dispatch path is kind-independent.

### 4 — Stale bridge (original Onoda packaged bridge)

4a, the binary itself on the original film dir (read-only), exit 3,
entire stderr:

```
film telemetry failed: film data: missing calendarYears
```

Doctor (4d = 4e, exit 2) is exemplary — structural verdict *and* a
live probe, each with the re-bridge remedy:

```
  BRIDGE: STALE — a previous bridge generation; the current binary refuses it:
    - film data has no 'calendarYears' declaration — the current binary's loader demands it by name and refuses this data
    - film data has no 'calendarDark' declaration — the current binary's loader demands it by name and refuses this data
    - film data has no 'sign' declaration — the current binary's loader demands it by name and refuses this data
    - assets/plates.json is the legacy flat shape — the current binary's load_assets requires the talerender-plates manifest shape
  REMEDY: re-bridge — bridge it with the TaleRender repo's bridge builder: …
  PROBE: binary refused the film data — film telemetry failed: film data: missing calendarYears
  REMEDY: the data, not the binary, is the problem: re-bridge — …
DOCTOR: FAILED (2 diagnosis/diagnoses)
```

Validator (4f = 4g, exit 2): the same four items as `STALE BRIDGE:`
problems, each ending `Remedy: re-bridge -- bridge it with
tools/bridge/build_bridge.py …`, closing `RESULT: FAILED (4 problem(s),
1 flagged)` on the original (the 1 flagged is the honest note that the
original project folder itself carries no `capture_native.mjs` — the
P1 retrofit item).

**Finding 4 — the adapter itself mis-directs the stale-bridge remedy
(4b, 4c FAIL).** 4b (`check`, exit 3) forwards the binary's exact
refusal and prints `CHECK FAILED`, but no remedy. 4c (`frames`,
exit 1) refuses at the gate — good — but because the no-verdict
classifier sees binary exit 3, the era-hint generic branch prints the
*binary-currency* remedy (verify `TALERENDER_BIN`, re-provision from
CI). On this run the binary is current (v1.0.1, via `TALERENDER_BIN`)
and the data is stale; following the printed remedy changes nothing.
The re-bridge remedy — the one doctor and the validator both print —
appears nowhere in the adapter's stale-bridge output. The gate's
classification keys on exit code alone here; a `film telemetry
failed:` line on stderr is positive evidence of a *data* refusal
(the doctor's probe already classifies exactly that as
`data-refused`) and could route the gate to the re-bridge remedy the
same way.

Adapter pre-validation does **not** catch the stale bridge either, by
design of the data-driven validator (absent F2 fields declare
nothing; the legacy flat plates shape is honoured) — the failure
correctly surfaces at the binary, fast (<2 s), never mid-render.

### 5 — `RENDERER=<other>` selection refusal

5a stderr (exit 2; 5b identical with `"chromium"`):

```
capture-native: RENDERER="chrome" does not select the native backend, and this adapter is the native backend only.
  Native:               RENDERER=native node tools/capture_native.mjs <mode>
Selection is explicit; the adapter never falls back silently, and this template ships no other capture backend.
```

Controls 5c/5d prove the guard is selection-shaped, not a blanket
refusal: `RENDERER=native` and unset both pass it and reach the next
check (the deliberately bad `TALERENDER_BIN` error, exit 1).

### 6 — Corrupted film data

6a (field deleted) is the cleanest row in the suite. Adapter (exit 1):

```
capture-native: film data in …/corrupt-field/native/film is incomplete — the binary would fail on:
  onoda.json: captions[] missing/empty
Regenerate the bridge data; the adapter does not improvise it.
```

Validator (exit 2) names the field *and* the packaged-hash drift;
doctor (exit 2) names the drift on `film/onoda.json` with the
re-bridge remedy (doctor does not inspect `captions` — recorded as a
PASS because its named remedy is correct, with the field-level
diagnosis supplied by the other two surfaces).

6b (truncated asset): the adapter's pre-validation checks asset
*existence* only, so the corruption surfaces at the binary — loudly,
naming the exact file (6b1 stderr, exit 3):

```
film telemetry failed: cannot load …/corrupt-trunc/native/film/assets/cap_000.png: unexpected end of file
check_mode: film telemetry produced no verdict line — treated as a FAILED gate (see stderr above)
```

— but again with **no remedy printed** (same Finding-1 gap), hence
FAIL. Doctor and validator (6b3/6b2, exit 2) both catch it — note
*how*: via `MANIFEST-SHA … film/assets/cap_000.png (hash differs)`,
not via a PNG-structure check. The validator's structural PNG check
did not independently flag the 16-byte file in this run; on a bridge
packaged **without** a MANIFEST-SHA, doctor and validator would both
miss this corruption and only the binary's load error (6b1) would
catch it. Worth a follow-up if SHA-less bridges are a real case.

## Chrome-suggestion grep audit

Over every captured `.stdout`/`.stderr` (`outputs/GREP-AUDIT.txt`):

- `capture\.mjs` — **zero hits.** No output names the retired file.
- `chrome` (case-insensitive) — hits in exactly three classes, none a
  suggested remedy:
  1. the literal path component `chrome-replacement` in this wave's
     own workspace directory, printed as part of absolute fixture
     paths (the majority of hits);
  2. 5a's stderr echoing the operator's own value back:
     `RENDERER="chrome" does not select the native backend…`;
  3. 5b's `chromium` hit is the same echo for `RENDERER=chromium`.
- No output suggests Chrome, Chromium, or any browser as a remedy,
  fallback, or next step. **Audit: PASS.**

## Bottom line

The failure surface is strong where the wave concentrated —
binary discovery, era pairing at the gate, unbridged refusal,
RENDERER selection, doctor, and validator all fail loudly with the
correct native remedy and the documented exit code. The remaining
work is concentrated in `check` mode and the gate's remedy routing:

1. `check` should print the era-hint / remedy text on the no-verdict
   path, exactly as `frames`/`video` do (fixes 2a, 2c, 4b, 6b1).
2. The gate should classify a stderr `film telemetry failed:` line
   as a data refusal and print the re-bridge remedy, as the doctor's
   probe already does (fixes 4c).
3. Doctor `INCONCLUSIVE` should not exit 0 when the probed binary
   produced no verdict at all (fixes 2g).
4. Rule on castsheet-vs-unbridged: exempt it in the contract, or
   have it print an UNBRIDGED notice while proceeding (3e).
