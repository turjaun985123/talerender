# CR-R2-07 — Failure-mode re-tests (remediation era)

Chrome-replacement REMEDIATION wave, Batch 2, Seat 07. Every broken setup
must produce a clear, named error — never a panic, never a silent fallback
to Chrome. Re-runs the CR-B3-04 cases (31 runs, 6 FAILs, all lead-fixed)
plus the new remediation-era cases against the current code.

- **Binary under test:** `~/workspace/talerender/target/release/talerender`
  — `talerender 1.0.1`, built @ `67fd9ef` (repo HEAD `01a941a`).
- **Adapter under test:** shipped skill adapter
  `~/workspace/custom-skills/tall-tale-shorts/template/tools/capture_native.mjs`
  (sha256 `5d7c3500…`, byte-identical to the B3-04-tested adapter file —
  the remediation fixes live in `native_modes/check_mode.mjs` +
  `native_modes/era_hint.mjs`, which DO differ from the LC project's
  in-tree copies; see defect D2) + `template/tools/project_doctor.py`.
- **Mis-bridged specimen:** `~/faceless-shorts/the-lighthouse-cats-last-shift`
  (LC) — pre-remediation bridge: `native/film.json` has NO `driver` field,
  so the binary routes it to the legacy driver, whose Onoda anchor set the
  LC anchors do not satisfy. Probed read-only; never written.
- **Stale binaries found locally (both real):**
  `~/talerender-bin/talerender` (v1.0.1, pre-guard — PANICS on LC data,
  case 2f) and `~/workspace/talerender/talerender` (pre-gate checkout
  build — `unknown argument: --film-telemetry`, case 2e). No simulation
  was needed for case 2.
- **Harness:** `run_r2_07.sh` in this dir (re-runnable; rebuilds all
  fixtures under `work/`). Full verbatim outputs in `outputs/` —
  per case `.cmd` / `.stdout` / `.stderr` / `.exit`.
- **Chrome-launch proof:** every adapter run executed with PATH shims
  for `chrome`, `chromium`, `chromium-browser`, `google-chrome`,
  `google-chrome-stable`, `brave`, `brave-browser`, `firefox` that log
  to `work/chrome-launch.log` and exit 99. **Final log size: 0 bytes —
  no browser launch was attempted in any case.** No output suggested
  Chrome/Chromium as a remedy anywhere (the only `chrome` strings are
  the operator's own `RENDERER=chrome` value echoed in 7d and the
  workspace path component `chrome-replacement`).
- **Environment:** this VM has NO Vulkan ICD. All cases are CPU/CLI
  paths. The one render leg reached (3e castsheet) failed at GPU init
  with a named error — see ENV-BLOCKED.

**Verdict rule:** PASS = clear named error + correct remedy text +
documented exit code + no Chrome launch. Result: **28 PASS / 2 FAIL
(defects D1, D3 — both minor/misleading-text class) / 1 observed-stale-copy
note (D2) / 1 ENV-BLOCKED leg (3e render success path).** All six B3-04
FAILs are verified FIXED (see §7).

## Results table

| Case | Fault → surface | Exit (expected) | Verdict |
|---|---|---|---|
| 1a-missing-explicit-check | `TALERENDER_BIN=/nonexistent/…` → adapter `check` | 1 (1) | PASS — "is not an executable file. An explicit TALERENDER_BIN is never second-guessed — fix the path or unset it" |
| 1b-missing-discovery-check | fake HOME + shim-only PATH → adapter `check` | 1 (1) | PASS — tried-list + "Set TALERENDER_BIN=… artifact from a green CI run" |
| 2a-fake-versiononly-check | fake answers `--version`, no telemetry → adapter `check` | 3 (3) | PASS — **B3-04 FAIL FIXED**: generic era-hint branch now printed by `check` (verify-currency command + provisioning remedy) |
| 2b-fake-versiononly-frames | same fake → adapter `frames` gate | 1 (1) | PASS — same remedy via gate |
| 2c-fake-unknownarg-check | fake exits 2 `unknown argument` → adapter `check` | 3 (3) | PASS — **B3-04 FAIL FIXED**: "Symptom: era mismatch…" + full provisioning remedy now printed by `check` |
| 2d-fake-unknownarg-frames | same fake → adapter `frames` | 1 (1) | PASS — identical era surface as B3-04 2d |
| 2e-real-repo-stale-frames | REAL stale `~/workspace/talerender/talerender` → adapter `frames` | 1 (1) | PASS — byte-shape identical to 2d with its own path |
| 2f-oldbin-direct-lc | REAL stale `~/talerender-bin/talerender` --film-telemetry on LC bridge (binary direct) | **101** (n/a — stale binary itself) | DOCUMENTED HAZARD — the pre-guard binary still panics (`src/film.rs:3726:55: no entry found for key`). This is the binary the guard was built against; it is not the current binary. Adapter-mediated use is case 2g. |
| 2g-oldbin-adapter-check | same panicking binary via adapter `check` | 3 (3) | PASS — adapter fails closed: panic text forwarded, "binary exit 101 — treated as a FAILED gate", crash named as a cause + provisioning remedy printed. The adapter converts the panic into a named, remedied refusal. |
| 2h-fake-versiononly-doctor | fake-version-only → doctor probe | 3 (3, new) | PASS — **B3-04 FAIL (2g, exit 0) FIXED**: `DOCTOR: INCONCLUSIVE — … NOT certified`, exit 3 (documented in doctor source as the EXIT-CODES "could not run to a verdict" code) |
| 3a-unbridged-check | no `native/film.json` → adapter `check` | 1 (1) | PASS — manifest-not-found + bridge-builder remedy + "There is no other capture backend to fall back to" |
| 3b-unbridged-frames | → adapter `frames` | 1 (1) | PASS — byte-identical stderr to 3a |
| 3c-unbridged-sheet | → adapter `sheet` | 1 (1) | PASS — byte-identical stderr |
| 3d-unbridged-video | → adapter `video` | 1 (1) | PASS — byte-identical stderr |
| 3e-unbridged-castsheet | → adapter `castsheet … look` | 1 (GPU init) | PASS on contract, ENV-BLOCKED on render — **B3-04 FAIL (3e) FIXED**: stderr now leads with "note — this project is UNBRIDGED… Castsheet is exempt by design… Film modes still require the bridge step", then the render leg failed "GPU initialisation failed… no wgpu adapter found — a Vulkan driver (hardware or lavapipe) is required" (exit 3 from binary, adapter exit 1). Success path needs lavapipe/hardware — CI route below. |
| 3f-unbridged-doctor | → doctor `--no-probe` | 2 (2) | PASS — `BRIDGE: UNBRIDGED` + build_bridge remedy |
| 4a-binary-telemetry-lc-live | current binary `--film-telemetry` on LC live bridge | 3 (3) | **PASS — headline case.** Named-anchor guard, NOT 101: `film telemetry failed: film data is not staged by the legacy driver (missing anchors: bookseller9, coconuts6, commander8, decades6, flew9, found7, loyal11, message3, never2, only8, order10, refused7, surrender2, surrendered10, thought4, trick4, years0); bridge it with driver: scene-sequence (see REMEDIATION-CONTRACTS §1)` |
| 5a-binary-film-lc-live | current binary `--film` on LC live bridge (`--from 0 --to 1`, scratch out) | 2 (see note) | PASS on guard — same named-anchor message via `error: failed to load …: film data is not staged by the legacy driver (missing anchors: …); bridge it with driver: scene-sequence…`, no panic, no frames written. Exit is **2, not 3**: the binary's documented `--film` code for "film or scene data that fails to load" is 2 (`--help` EXIT CODES); 3 is the telemetry could-not-run code. The task's "same guard expectation" is met in substance (named guard, no 101); the code difference is per the binary's own published table, recorded here so the lead can rule if a uniform 3 was intended. |
| 5b-adapter-video-lc | LC bridge → adapter `video` (gate) | 1 (1) | PASS — gate refuses pre-render with the binary's anchor message + **re-bridge remedy** ("Provisioning a different binary does not fix refused data") |
| 5c-adapter-check-lc-oldtools | LC bridge → LC project's OWN in-tree tools `check` | 3 (3) | See D2 — exit/remedy behaviour of the stale in-tree copy |
| 6a-binary-malformed-onoda | scratch film dir, `onoda.json` = `not json at all` → binary telemetry | 3 (3) | PASS — `film telemetry failed: json: bad literal at byte 0` |
| 6b-adapter-malformed-manifest | scratch project, `native/film.json` corrupted → adapter `check` | 1 (1) | PASS — `film manifest: cannot read/parse …: Expected property name or '}' in JSON at position 2 (line 1 column 3)` — file + parse position named |
| 6c-binary-nocaptions | LC film copy with `captions` deleted → binary telemetry | 3 (3) | PASS — `film telemetry failed: film data: missing captions` |
| 6d-binary-truncated-png | LC film copy, `cap_000.png` → 16 B → binary telemetry | 3 (3) | PASS-with-masking-note — the anchor guard (4a) fires before caption assets load, so the truncation is not what this run surfaces; the unmasked truncated-PNG behaviour is 6f. |
| 6e-adapter-check-truncated | adapter `check` with `NATIVE_FILM_DIR`=6d dir | 3 (3) | PASS — data-refused classification + re-bridge remedy printed (remedy class proven; the specific refusal line is the anchor guard, per 6d) |
| 6f-binary-truncated-onoda | B2-08 current-gen Onoda copy, `cap_000.png` → 16 B → binary telemetry | 3 (3) | PASS — `film telemetry failed: cannot load …/cap_000.png: unexpected end of file` (exact file named; B3-04 6b1 binary behaviour unchanged). Control on the untruncated copy: `FILM TELEMETRY: CHECK OK`, exit 0. |
| 7a-stalebridge-adapter-check | original Onoda bridge copy → adapter `check`, current binary+tools | 3 (3) | PASS — **B3-04 FAIL (4b) FIXED**: binary refusal `plates.json: not a plates manifest (expected {"format": "talerender-plates", "plates": [...]})` + re-bridge remedy printed by `check` |
| 7b-stalebridge-adapter-frames | same → adapter `frames` gate | 1 (1) | PASS — **B3-04 FAIL (4c) FIXED**: gate now classifies `film telemetry failed:` as data-refused and prints the RE-BRIDGE remedy, not the binary-provisioning one |
| 7c-stalebridge-binary-direct | current binary telemetry on original Onoda film dir (read-only) | 3 (3) | PASS — loader decoupling visible: refusal is now the plates shape, NOT `missing calendarYears` (the B3-04 4a text). Calendar/sign are no longer demanded. |
| 7d-renderer-chrome-check | `RENDERER=chrome` → adapter `check` | 2 (2) | PASS — selection refusal, "never falls back silently", no launch (shim log empty) |
| 7e-stalebridge-doctor-noprobe | original Onoda bridge copy → doctor `--no-probe` | 2 (2) | **FAIL (minor, D3)** — exit + re-bridge remedy correct, but the named causes are stale post-decoupling |

## Defects

### D1 — (from 2f) The provisioned stale binary `~/talerender-bin/talerender` still panics on mis-bridged data
Repro: `~/talerender-bin/talerender --film-telemetry ~/faceless-shorts/the-lighthouse-cats-last-shift/native/film`
→ exit 101, `thread 'main' panicked at src/film.rs:3726:55: no entry found for key`.
This is the OLD binary, not the current one — the current binary answers
the identical call with the named guard at exit 3 (4a). Recorded because
`~/talerender-bin/` is the documented provisioning slot: any operator (or
discovery path) still holding that build gets a panic, not an error.
The adapter mitigates it (2g: fails closed, exit 3, remedy printed).
Remedy for the lead: re-provision `~/talerender-bin` from current CI, or
delete the stale build. No code defect in current trees.

### D2 — LC project's in-tree tools are the pre-remediation copies (5c)
The LC project carries its own `tools/capture_native.mjs` +
`tools/native_modes/` from before the B3-04 fixes: its `check_mode.mjs`
has no `era_hint` import and its `era_hint.mjs` has no data-refused
classification. Running the project's own tools against its own bridge
(5c, read-only `check` in the live project dir) exits 3 correctly with
the binary's named-anchor refusal, but prints only
`check_mode: film telemetry produced no verdict line — treated as a FAILED gate (see stderr above)`
— **no remedy** (B3-04 Finding 1, alive in this copy). The shipped skill
template is fixed (2a/2c/7a prove it); the fix reaches projects only via
the tools overlay at retrofit/re-bridge time. Not a shipped-tree defect;
flagged so the LC retrofit seat overlays fresh tools before judging LC
by its in-tree adapter.

### D3 — FAIL (minor): doctor's stale-bridge reasons are pre-decoupling text (7e)
Doctor on the original Onoda bridge (exit 2, remedy correct) still names
as refusal causes: "film data has no 'calendarYears' declaration — the
current binary's loader demands it by name and refuses this data"
(likewise `calendarDark`, `sign`). Post-remediation the loader demands
none of these: the current binary's actual refusal of the same data is
solely the plates shape (7c). An operator following the doctor's text
would hunt calendar fields the binary no longer reads. The verdict and
the re-bridge remedy are right; the cause list misleads. Fix: update
`project_doctor.py`'s structural stale-bridge checks to the decoupled
loader contract (drop or re-word the F2-field items; keep plates shape).

## B3-04 FAIL re-verification summary (case 7 + friends)

| B3-04 FAIL | Cause | Re-test | Now |
|---|---|---|---|
| 2a, 2c | `check` printed no remedy on no-verdict | 2a, 2c | FIXED — classified era-hint remedy printed, exit 3 |
| 2g | doctor INCONCLUSIVE exited 0 | 2h | FIXED — exit 3, "NOT certified" |
| 3e | castsheet silent on unbridged | 3e | FIXED — explicit UNBRIDGED/exempt notice printed before dispatch |
| 4b | stale bridge, `check` no remedy | 7a | FIXED — re-bridge remedy printed |
| 4c | gate printed binary-provisioning remedy for stale data | 7b | FIXED — data-refused classification → re-bridge remedy |
| 6b1 | truncated PNG, `check` no remedy | 6e/6f + 7a pattern | FIXED at adapter level — binary names the file (6f, exit 3); adapter `check` on a refused film dir now prints the re-bridge remedy (6e, same code path as 7a) |

## ENV-BLOCKED

- **3e castsheet render success path** (and any `frames`/`sheet`/`video`
  render leg past the gate): this VM has no Vulkan ICD; the binary
  refuses GPU init with a named error (proven above). The success path
  is covered by the CI route used throughout the wave: the talerender
  GitHub Actions CI job (lavapipe) running the shipped adapter's
  castsheet against an unbridged scaffold — same route as the B3-04 3e
  run and the CR-B3-02 CI proof. Nothing else in this seat's case list
  needs a render leg: case 5's `--film` attempt is refused at data load,
  before any GPU initialisation, on any machine.

## Non-mutation note

All fixtures are scratch copies under `work/`. The live LC project was
only read (binary telemetry probes + one read-only adapter `check`,
5c, which writes nothing). No shipped tree was edited; nothing committed.
