<!--
DELIVERY NOTE — Chrome Replacement, Batch 2, Seat 6 (CR-B2-06):
doctor/validation native update.

Files created (this seat, NEW files only; nothing existing edited
in place, nothing committed in either repo):
  ~/workspace/chrome-replacement/batch2/B2-06/
    validate_native_project.patch        (repo validator fix)
    project_state.patch                  (skill stage-table fix)
    project_doctor.patch                 (NEW skill doctor, as patch)
    staged/repo-tools-integration/validate_native_project.py
    staged/skill-template-tools/project_state.py
    staged/skill-template-tools/project_doctor.py   (NEW file)
    tests/test_b2_06.py + tests/test-output.txt
  docs/integration/CR-B2-06-DOCTOR.md    (this note)

Wiring the lead must do:
  - repo:  git apply validate_native_project.patch
           (tools/integration/validate_native_project.py)
  - skill: git apply project_state.patch + project_doctor.patch
           (template/tools/). All three patches were verified to
           apply cleanly and reproduce the staged files byte-for-byte.
  - NOTE for the build_bridge leg (b): build_bridge.py's verify
    leg (b) invokes the repo validator — once the lead applies the
    validator patch, a build_bridge run on a current-gen project
    goes from "VERIFICATION FAILED (1 leg)" to all-legs-pass
    (legs a/c unchanged). No build_bridge change is needed.
-->

# CR-B2-06 — Doctor/validation native update

## 1. The validator drift, proven on real data (not asserted)

The B1-06 finding was reproduced end-to-end before fixing:

* A **current-generation** bridge was built with the repo's own
  `tools/bridge/build_bridge.py` from the Onoda project's real
  timing data (`script.json`, `data/lines.json`, `data/words.json`,
  `data/mix.wav`) + the repo's transcribed scenes
  (`scenes/film/*.scene.json`) + repo film data/assets
  (`--film-data film/onoda.json`). Build leg (a) (P4 data-driven
  adapter semantics) **PASS**; leg (b) — the repo validator —
  **FAIL**, and leg (c) skipped with `--no-telemetry` (printed).
  The old validator's three problems were, verbatim:

  ```
  - film data: plate format.png missing (declared in plates.json)
  - film data: plate plates.png missing (declared in plates.json)
  - film data: plate version.png missing (declared in plates.json)
  ```

  Cause, from the old code: it iterated the plates.json **dict
  keys** as plate names — correct for the pre-F2 flat shape
  (`{"<name>": [x, y]}`), nonsense for the current
  `talerender-plates` manifest shape
  (`{format, version, plates: [{name, file, origin}]}`), whose keys
  are literally `format` / `plates` / `version`.

* The **stale packaged Onoda bridge** (the project's own
  `native/`: flat plates.json, film data with NO
  `calendarYears`/`calendarDark`/`sign` keys, lines carrying the
  old `i`/`text` bookkeeping) was passed **RESULT: OK** by the old
  validator — while the provisioned binary
  (`~/talerender-bin/talerender`, 1.0.1) refuses that exact data:

  ```
  $ talerender --film-telemetry <onoda-project>/native/film
  film telemetry failed: film data: missing calendarYears
  ```

  The old validator also carried a hardcoded Onoda asset list
  (`ADAPTER_FIXED_ASSETS`: calendar-1945/1955/1965/1974,
  calendar-dark, sign) regardless of what a project's data
  declares — failing any non-Onoda bridge for lacking Onoda props
  and never checking the F2 *declarations* the binary demands.

## 2. The fixed validator (staged replacement + patch)

Source of truth: `tools/bridge/build_bridge.py` output shape +
`template/tools/native_modes/filmdata_validate.mjs`
(`requiredAssets()` derivation, self-test 4/4). Changes, all in
the staged file's amendment header:

1. **Plates are data-driven.** Manifest shape → each
   `plates[]` entry requires its `file` (default `<name>.png`)
   and a valid `[x, y]` origin; `format` must be
   `talerender-plates`. Legacy flat shape → **STALE BRIDGE**
   problem (the current binary's `load_assets` rejects that
   shape), files still checked by name.
2. **Calendar/sign assets derive from the film data's own F2
   fields** (`calendarYears[]` → `calendar-<year>.png`,
   `calendarDark`, `sign`) — the filmdata_validate.mjs rule. The
   hardcoded list is deleted.
3. **Stale-generation detection.** Film data missing any F2 key
   (`calendarYears` / `calendarDark` / `sign`) is a named
   **STALE BRIDGE** problem per key, quoting the binary's
   by-name demand, with the re-bridge remedy. (This is stricter
   than the adapter-level JS validator by design: it mirrors
   build_bridge's leg (c), the binary's own verdict, which is
   the generation contract for packaged projects.)
4. **MANIFEST-SHA verified** when present (both bridge builders
   package one): any missing/hash-drifted file → STALE BRIDGE
   with the re-bridge (`--force`) remedy; absence → FLAGGED.
5. **Current-generation film-data shapes checked:** lines are
   `{start, end}` per line; caption chunks carry numeric `s`/`e`
   plus their strip.
6. **No Chrome routing anywhere.** Unbridged / missing film data
   → remedy is the bridge step
   (`tools/bridge/build_bridge.py --project … --scenes …`, doc
   CR-B1-06-BRIDGE.md). The old "capture with Chrome" problem
   text and the "Chrome adapter is unaffected" flag are gone;
   the capture_native flag now names the P1 retrofit.
7. `--self-test` extended 3 → **6 cases**: valid current-gen
   (manifest plates + F2) passes; frame gap fails; missing
   script.json fails; **stale previous-generation fixture
   (flat plates + no F2) fails as STALE with the remedy and no
   "Chrome" in output**; missing data-declared plate named;
   unbridged → bridge remedy, no Chrome.

Real-data verdicts after the fix:

```
$ validate_native_project.py <build_bridge-built project>
  info: … 1811 frames - 60.357s - 1080x1920 - 14 segments/shots …
RESULT: OK (1 flagged)                      # was: FAILED (3 bogus)

$ validate_native_project.py <real Onoda project, stale bridge>
  - STALE BRIDGE: onoda.json has no 'calendarYears' declaration …
  - STALE BRIDGE: onoda.json has no 'calendarDark' declaration …
  - STALE BRIDGE: onoda.json has no 'sign' declaration …
  - STALE BRIDGE: assets/plates.json is the legacy flat shape …
RESULT: FAILED (4 problem(s), 1 flagged)    # was: RESULT: OK
```

## 3. Skill-side doctor/validation

There is **no file named project_doctor** in the skill today
(grep-verified); the skill's diagnostic surface is
`template/tools/project_state.py` (status/verify/resume — the
ledger doctor) plus per-tool checks. Both halves are addressed:

* **NEW `template/tools/project_doctor.py`** (staged; delivered
  also as a new-file patch). Three diagnoses, each native-truthful:
  - **unbridged** → `BRIDGE: UNBRIDGED` + the named bridge step
    (build_bridge route). Never a Chrome suggestion.
  - **stale bridge** → `BRIDGE: STALE` with each reason named
    (missing F2 key / flat plates.json / MANIFEST-SHA drift) +
    the re-bridge remedy. With the binary probe on, the real
    stale Onoda project is additionally refused BY NAME by the
    provisioned binary (`film telemetry failed: film data:
    missing calendarYears`) and classified **data-refused**,
    not stale-binary — the probe follows era_hint.mjs semantics
    (verdict line required; stale asserted only on positive
    evidence: exit 2 or the binary's own
    `unknown argument: --film-telemetry`; the exit-0
    `film telemetry failed:` surface is a data refusal).
  - **missing/stale binary** → adapter discovery order
    (TALERENDER_BIN → provisioned → repo checkout → PATH;
    explicit TALERENDER_BIN never second-guessed) + the
    **era-hint remedy** verbatim in shape from
    `native_modes/era_hint.mjs` / CAPTURE-NATIVE.md Prerequisite 1
    (CI artifact download commands + the repo-checkout-slot
    stale warning).
* **`project_state.py` stage table made native** (staged
  replacement + patch): new `bridge` stage
  (build_bridge.py; outputs `native/`; every native stage
  depends on it); `cues` → `python3 tools/cues_native.py .`;
  `check`/`sheet`/`video` → `RENDERER=native node
  tools/capture_native.mjs …` with `native/` as an input and
  `capture.mjs` removed from every run command and tool stamp.
  The `build` stage remains (transcription/skeleton export reads
  `src/`) but no native stage depends on it.

## 4. Tests — run for real, 17/17 PASS

`tests/test_b2_06.py` (output saved verbatim in
`tests/test-output.txt`). Fixtures are real where it matters:
the current-gen bridge is built by build_bridge.py itself; the
stale fixture is the real Onoda project (read-only); the binary
refusal is the provisioned binary's own output.

```
case fixture: build_bridge.py builds a current-gen bridge       PASS (build exit 1 — leg (b) is the OLD validator; §2 wiring note)
case validator ACCEPTS current-generation bridged project       PASS (exit 0)
case old validator fails it naming format/plates/version.png (drift proof) PASS (exit 2)
case validator REJECTS real stale Onoda bridge as STALE         PASS (exit 2)
case old validator passes the stale bridge (drift proof)        PASS (exit 0)
case provisioned binary refuses stale data (missing calendarYears) PASS
case validator REJECTS bridge missing data-declared plate       PASS (exit 2)   [negative: coconut.png deleted]
case validator REJECTS tampered bridge (MANIFEST-SHA drift)     PASS (exit 2)   [negative: anchor added post-package]
case validator REJECTS unbridged project, names build_bridge    PASS (exit 2)   [negative: native/ removed]
case validator --self-test                                      PASS (SELF-TEST: PASS (6/6))
case doctor OK on current-gen bridged project                   PASS (exit 0)
case doctor names stale Onoda bridge + re-bridge remedy         PASS (exit 2)
case doctor names unbridged project + bridge step               PASS (exit 2)
case doctor names missing binary + TALERENDER_BIN remedy        PASS (exit 2)   [negative: TALERENDER_BIN=/nonexistent]
case doctor names stale binary + era-hint remedy                PASS (exit 2)   [negative: fake binary, "unknown argument" + exit 2]
case doctor probe: real binary refuses stale data by name       PASS (exit 2)
case project_state stage table is native-truthful               PASS
TESTS: PASS (17/17)
```

Negative cases tested, by name: frame-numbering gap; missing
`script.json`; stale previous-generation bridge (flat plates +
no F2); missing data-declared plate (`coconut.png`); tampered
bridge (MANIFEST-SHA drift); unbridged project; missing binary;
stale binary (fake, positive-evidence marker). One honest
non-result: `--film-telemetry` against the repo's *valid* film
dir did not return within 120 s on this VM (telemetry over the
full film is heavy here), so no valid-data probe verdict is
claimed; the doctor's probe therefore has a timeout and reports
INCONCLUSIVE rather than hanging or guessing.

## 5. Grep audit — doctor/validation text routing to Chrome

Scope: validator/doctor/validation surfaces in both trees
(2026-11-15, grep for `chrome`/`capture.mjs` over
`tools/integration/`, `tools/bridge/`, skill
`template/tools/{project_state,project_doctor,capture_native}.mjs/py`,
skill `scripts/{metadata_check,library_check}.py`,
`template/tools/{qa_visual,script_lint}.py`, plus the wider
skill validation scripts). Dispositions: FIXED-HERE (this seat's
deliverables) / OTHER-SEAT (named owner in the Batch-2 plan) /
HISTORY (true historical statement, no routing) / CLEAN.

| # | Hit | Disposition |
|---|-----|-------------|
| 1 | repo `tools/integration/validate_native_project.py`: "unbridged projects, **capture with Chrome**" | **FIXED-HERE** — remedy is now the build_bridge step (§2.6) |
| 2 | same file: flag "The Chrome adapter (tools/capture.mjs) is unaffected" | **FIXED-HERE** — flag now names the P1 native-tools retrofit |
| 3 | same file: header "native stays opt-in, Chrome stays the default" (V2 §13 note) | **FIXED-HERE** — amendment header supersedes it in the staged file; the V2 note remains below as history |
| 4 | repo `tools/bridge/README.md:157`: "an unbridged project is captured with Chrome, and the adapter says so itself" | OTHER-SEAT — bridge docs; must be rewritten to the build_bridge remedy when the adapter text (row 6) changes. Flagged for the lead |
| 5 | skill `template/tools/project_state.py`: stage table routed cues/check/sheet/video through `tools/capture.mjs` | **FIXED-HERE** — native stage table (§3) |
| 6 | skill `template/tools/capture_native.mjs:99`: unbridged → "a project without it is **captured with Chrome (tools/capture.mjs)**" | OTHER-SEAT — adapter seat (B1-10 lineage / Batch-2 adapter work); the doctor + validator no longer repeat it, but the adapter's own fail text still routes to Chrome and must get the build_bridge remedy at integration |
| 7 | skill `capture_native.mjs:57`: RENDERER-mismatch error offers "Chrome (the default): node tools/capture.mjs" | OTHER-SEAT — same adapter item; post-flip the mismatch text should name only the native invocation |
| 8 | skill `capture_native.mjs:2,10,11,402` + header: "instead of headless Chrome", "preview retired with Chrome", "no Chrome fallback" | CLEAN — anti-routing statements (they refuse Chrome), not suggestions |
| 9 | skill `scripts/selftest.sh:24-32`: runs `capture.mjs` check/cues/sheet/video with CHROME env passthrough | OTHER-SEAT — contracts §1 row 11 (Batch-2 selftest seat) |
| 10 | skill `scripts/setup.sh:3,31-32`: Chrome is a hard requirement ("chrome MISSING (install Google Chrome…)") | OTHER-SEAT — contracts §1 row 10 (Batch-2 setup seat): requirement becomes binary + Vulkan ICD + ffmpeg |
| 11 | skill `tools/selftest_native.sh:11,15,100,104` + `tools/SELFTEST-NATIVE.md:7,8,66,147`: "Chrome flow unaffected", "the Chrome selftest … byte-identical" framing | OTHER-SEAT — contracts §1 row 16 (absorbed into the native selftest at the flip); text is descriptive of the opt-in era, becomes false at the flip |
| 12 | skill `scripts/metadata_check.py`, `scripts/library_check.py`, `template/tools/qa_visual.py`, `template/tools/script_lint.py` | CLEAN — no Chrome references (grep-verified); backend-agnostic validators |
| 13 | repo `tools/bridge/build_bridge.py` header + delivery note | CLEAN — names the mission ("Chrome Replacement"), routes nothing to Chrome |

## 6. Honest remainders

* The validator's F2-presence rule is generation-strict: a
  hypothetical future film whose data legitimately declares no
  calendar/sign would be flagged STALE by the project validator
  while passing the adapter-level JS validator. That mirrors the
  current binary (its film loader demands the keys by name —
  verified refusal above) and build_bridge's own stance; if the
  binary's loader generalises, this check must generalise with
  it. Named here so it is a decision, not a surprise.
* Doctor probe on valid film data: no verdict claimed (VM
  timeout, §4). The probe's INCONCLUSIVE path is the designed
  behaviour for exactly this case.
* Nothing was committed; no existing file was edited in place.
  The three patches + staged files are the deliverables.
