# R2-10 — Selftest + doctor regression (Chrome-Replacement REMEDIATION, Batch 2)

Seat: R2-10. Date: 2026-11-16. Skill: `~/workspace/custom-skills/tall-tale-shorts`
(post-Batch-1 landings). TaleRender HEAD during testing:
`01a941adda6cbb0c3dbb5fd06f663117fa0a4166` (hygiene commit on top of
Batch-1 close `67fd9ef`). Current binary used:
`~/workspace/talerender/target/release/talerender` (23 MB, built
2026-11-16 04:22, `talerender 1.0.1`, post-Batch-1).
Scratch: this directory (`logs/` holds the raw output of every run
quoted below; `scratch/` holds the fixtures). No shipped tree and no
live project was edited; nothing committed. Live projects were only
read; the one adapter run that could write was done on a scratch
copy (`scratch/lc-copy/`).

**Environment (this VM):** NO Vulkan ICD on any standard path
(`/usr/share/vulkan/icd.d` absent, `/etc/vulkan/icd.d` empty),
`/dev/dri` absent. Every render attempt dies with the exact error
`no wgpu adapter found — a Vulkan driver (hardware or lavapipe) is
required` (current binary prefixes it: `error: GPU initialisation
failed (request_adapter (gpucore)):`). There is no llvmpipe here at
all, so the historical llvmpipe-segfault class does not even arise:
render legs are **ENV-BLOCKED (no adapter)**, never PASS/FAIL.
CPU: ~1.5 shared vCPU — full-film `--film-telemetry` is a
tens-of-minutes operation here (selftest.sh's own header measurement:
the 60 s fixture's telemetry "did not complete within 360 s
standalone"); CI (lavapipe) is the reference environment for those
legs, per leg in §5.

---

## 0. Headline verdicts

| Question | Verdict |
|---|---|
| Fresh bridge mislabelled STALE by the **validator** (R1-03 fix)? | **NO — re-proven.** Self-test 9/9; fresh no-F2 fixture → `RESULT: OK`, no STALE. |
| Fresh bridge mislabelled STALE by **project_doctor**? | **YES — DEFECT D1.** Doctor → `BRIDGE: STALE` on the identical fresh fixture, on F2-absence grounds R1-03 abolished. Doctor was never given the decoupling semantics. |
| LIBRARY OK (479) after Batch 1 landings? | **YES** — `LIBRARY OK (479 items)`, exit 0. (Strict-previews still FAIL 238 pending — the honest pre-R2-09 state, not a regression.) |
| Onoda retrofitted project clean/OK? | **NO — see §3.** The live project's *packaged* bridge is the stale generation (flat `plates.json`) per BOTH tools. The current-generation Onoda data lives in the repo `film/`, not in the project's `native/`. Charter expectation not met by the live project as it stands. |
| LC current bridge label? | Validator: **OK**. Doctor (structural): **"current generation" / DOCTOR OK**. Both labels are *structurally* correct; neither tool sees that the bridge has no `driver` field and therefore routes to the legacy driver (binary guard — §4 probe legs). **Dependency: R2-03's re-bridge is still in flight** (its staging dir exists, no delivery yet); the LC bridge tested here is the pre-remediation B3-01-era bridge, byte-for-byte as found. |
| Unbridged scratch → named bridge remedy, not an error? | **YES, both tools** — `build_bridge.py` named, no Chrome routing, no traceback (§3). |
| Any selftest leg FAIL for a real (non-env) reason? | No leg *printed* FAIL — but **leg 6's ENV-BLOCKED is a misclassified real defect (D4)**: the bridge builder deterministically refuses the scaffolded project and the script misreports it as a provisioning block with a wrong remedy. It would ENV-BLOCK identically on CI/RADV. Plus D1 (doctor STALE mislabel), D2 (driver blind spot), D3 (validator startS gap). |

---

## 1. Per-leg table — `tools/selftest_native.sh` (legacy smoke, still shipped)

| Leg | Command | Verdict | Evidence (log) |
|---|---|---|---|
| Gate SKIP-1 | `env -u RENDERER -u TALERENDER_BIN bash tools/selftest_native.sh` | PASS (SKIP, exit 0) | `SELFTEST-NATIVE SKIP: RENDERER is '<unset>' (need 'native')` — `logs/09` |
| Gate SKIP-2 | `RENDERER=native`, `TALERENDER_BIN` unset | PASS (SKIP, exit 0) | `SELFTEST-NATIVE SKIP: TALERENDER_BIN is not set` — `logs/09` |
| Gate SKIP-3 | `RENDERER=chrome` + binary set | PASS (SKIP, exit 0) | `SELFTEST-NATIVE SKIP: RENDERER is 'chrome'` — `logs/09` |
| Steps 1–3 telemetry, stale binary (repo-checkout slot, 13 Nov, pre-telemetry) | `RENDERER=native TALERENDER_BIN=~/workspace/talerender/talerender bash tools/selftest_native.sh` | ENV-BLOCKED (exit 3), correctly **not** FAIL | Steps 1–3: `binary cannot run this telemetry mode here (exit 2: unknown argument: --telemetry)` — `logs/09` |
| Step 4 render, stale binary | (same run) | ENV-BLOCKED | `non-zero exit 101 … no wgpu adapter found — a Vulkan driver (hardware or lavapipe) is required` — `logs/09` |
| Steps 1–3 telemetry, **current** binary | `RENDERER=native TALERENDER_BIN=~/workspace/talerender/target/release/talerender bash tools/selftest_native.sh` | **PASS ×3** | `ok: step 1 … exit 0, 'CHECK OK'`; `ok: step 2 … exit 1, 'CHECK FAILED'` (gate proven in the failing direction); `ok: step 3 … exit 0, 'CHECK OK'` — `logs/10` |
| Step 4 render, current binary | (same run) | ENV-BLOCKED (exit 3 overall) | `error: GPU initialisation failed (request_adapter (gpucore)): no wgpu adapter found — a Vulkan driver (hardware or lavapipe) is required` — `logs/10` |
| Steps 1–3 telemetry, provisioned binary (`~/talerender-bin`, 15 Nov, v1.0.1-era) | same, `TALERENDER_BIN=~/talerender-bin/talerender` | **PASS ×3** | Identical three `ok:` lines — the M6 telemetry CLI predates Batch 1, so the pre-Batch-1 provisioned binary still passes the CPU legs — `logs/10` |
| Step 4 render, provisioned binary | (same run) | ENV-BLOCKED | Same no-adapter error — `logs/10` |

## 2. Per-leg table — `scripts/selftest.sh` (native end-to-end, B2-03 replacement)

The script itself was run end-to-end in `--cpu-only` mode:
`TALERENDER_BIN=<current binary> SELFTEST_BRIDGE_TIMEOUT=420 SELFTEST_RENDER_TIMEOUT=420 bash scripts/selftest.sh --cpu-only --keep`
→ **`SELFTEST ENV-BLOCKED`, exit 3** (39 s wall; `logs/13`, script
log at `~/faceless-shorts/_selftest-native/selftest.log`, project
kept). Per-leg results from that run, plus the standalone
equivalents run separately:

| Leg | Verdict (script run) | Evidence |
|---|---|---|
| 0 env probe | PASS (binary = current target/release, era probe OK; canary skipped by `--cpu-only` as designed) | `leg 0: binary=…/target/release/talerender (talerender 1.0.1)` — `logs/13`. Components independently verified: node v24.22.1, ffmpeg 8.1.2, numpy/scipy OK, bun `/opt/hatch-image/bin/bun`, fixture files present — `logs/00`, `logs/11` |
| 0 render canary (full-run mode only) | ENV-BLOCKED | No frame; `no wgpu adapter found` (proven via selftest_native step 4 with two binaries, `logs/09`+`logs/10`) |
| 1 typecheck | **PASS** | `ok: leg 1: typecheck clean` — `logs/13`; standalone rerun exit 0 in 3.8 s — `logs/11` |
| 2 library | **PASS** | `ok: leg 2: LIBRARY OK (479 items)` — `logs/13`, `logs/03` (§6) |
| 3 telemetry ×3 | **PASS ×3** | `ok: leg 3: telemetry ab1 (exit 0, 'CHECK OK')`, `ab1-variant (exit 1, 'CHECK FAILED')`, `ab1-file (exit 0, 'CHECK OK')` — `logs/13`, `logs/10` |
| 4 scaffold + build + fixture overlay | **PASS** | `ok: leg 4: scaffold + build OK`, `ok: leg 4: fixture overlay OK (… from the-soldier-who-didnt-know)` — `logs/13` |
| 5 cues negative | **PASS** | `ok: leg 5: unbridged cues correctly refused (exit 2, bridge step named; no Chrome fallback offered)` — `logs/13`; standalone equivalent on scratch with the real emitter: exit 2, `not bridged … no Chrome fallback` — `logs/12` |
| 6 bridge build | **ENV-BLOCKED as printed — actually DEFECT D4** | Script: `SELFTEST ENV-BLOCKED: leg 6: bridge built (data legs OK) but its binary telemetry verification did not run — remedy: provision a binary (leg 0) and re-run`. Script log line 77 gives the true cause: `build-bridge: …/tall-tale-selftest/native already contains scenes, film — refusing to overwrite; re-run with --force`. No bridge was built (kept project's `native/` has no `film.json`, only the template skeleton + `film.json.example`). See D4. |
| 7 native check | ENV-BLOCKED (cascade from leg 6) | `leg 7: native check not run — bridge not verified (leg 6)` — `logs/13`. Not independently blocked: the check gate's own semantics were exercised via doctor probes + direct telemetry (§4). |
| 8 cues (8a routing + 8b emitter) | ENV-BLOCKED (cascade: `bridge data not verified (leg 6a)`) | `logs/13`. Standalone equivalents PASS on the LC scratch copy: 8a routing exit 1 naming `cues_native.py` with correct cues-currency report — `logs/12`. |
| 9–12 frames/sheet/castsheet/video | SKIPPED-ENV (`--cpu-only`) | `logs/13`. In a full run they are canary-gated → ENV-BLOCKED (no adapter, §5). |
| 13 mix | ENV-BLOCKED (cascade from legs 6a/8) | `leg 13: mix not run — needs data-verified bridge data + data/cues.json` — `logs/13` |
| 14 metadata | **PASS** | `ok: leg 14: metadata — METADATA: PASS (shelf preview emitted at 180x320 and dimension-verified …)`; two-pass gate output in script log (pass 1 FAIL = shelf attestation only, as designed) — `logs/13` |
| Adapter usage (extra) | PASS | `node tools/capture_native.mjs` (no mode) on LC copy: modes line, exit 2 usage — `logs/12` |
| **Overall** | **ENV-BLOCKED (exit 3)** | `SELFTEST ENV-BLOCKED (all CPU legs that could run passed; …)` — the "could run" is doing real work in that sentence: leg 6 could not run *as scripted* on any machine (D4). |

## 3. Doctor + validator matrix (charter item 2)

Tools: doctor = skill template `template/tools/project_doctor.py`
(B2-06 + CR-B3-04 inconclusive amendment; LC's own project copy is
the same file minus that amendment — 14-line diff, same bridge
logic). Validator = repo `tools/integration/validate_native_project.py`
(R1-03-patched). Doctor rows are `--no-probe` (structural) unless
marked probe; probe rows in §4. Full output: `logs/05` (doctor),
`logs/06` (validator).

| Target | Validator | Doctor (structural) | Label correct per R1-03 semantics? |
|---|---|---|---|
| (a) Onoda project `~/faceless-shorts/the-soldier-who-didnt-know` | **FAILED (exit 2), 1 problem: `STALE BRIDGE: assets/plates.json is the legacy flat shape`** + re-bridge remedy; 1 FLAGGED (no `tools/capture_native.mjs` in project — P1 retrofit not applied). F2 absence **not** flagged. | **STALE (exit 2)**: flat-plates reason + 3× F2-absence reasons; remedy = re-bridge via `build_bridge.py`. Binary found via provisioned slot. | Validator: **YES** — staleness rests solely on generation drift (flat plates), exactly R1-03's definition. Doctor: verdict YES, **rationale ¾ wrong** — the 3 F2 reasons assert a loader demand that no longer exists (D1). |
| (b) LC project `~/faceless-shorts/the-lighthouse-cats-last-shift` (current = pre-remediation bridge) | **RESULT: OK (0 flagged), exit 0** — 39.049 s, 1172 frames, 11 segments, 43 caption strips. | **BRIDGE: current generation … DOCTOR: OK (exit 0)**. | **Structurally yes, with a named blind spot (D2):** the bridge *is* manifest-shape + F2-declaring + SHA-true (both SHA verifications: 68 entries, no drift — `logs/01`). But it carries **no `driver` field**, so the binary routes it to the legacy driver and refuses at the guard (§4) — a fact neither structural tool can see. R2-03's re-bridge (driver field, de-invented calendar/sign, measured fit, plate audit) is the fix; **this seat tested the bridge as found and flags the dependency.** Also noted: `native/BRIDGE-STATUS.txt` still claims `UNBRIDGED` — stale status text, already logged as WAVE-STATE Discrepancy 1. |
| (c1) Unbridged scratch, full project shape (`scratch/unbridged-full/proj`) | FAILED (exit 2) with exactly the 2 bridge problems, **both naming `build_bridge.py`**, no Chrome | **UNBRIDGED** (exit 2) + named `build_bridge.py` remedy, no Chrome | **YES** — named remedy, not an error/traceback, both tools. |
| (c2) Unbridged scratch, bare empty dir (`scratch/unbridged-bare`) | FAILED (exit 2), full problem list (script/lines/words/shots missing) + bridge remedy named twice, 2 FLAGGED, no traceback | **UNBRIDGED** (exit 2) + named remedy | **YES** — exit 2 is the tools' designed validation-failed code; the remedy is named, nothing crashes. |
| (d) **Fresh scene-sequence bridge** (`scratch/fresh/proj`: no F2 keys, `driver: "scene-sequence"`, manifest plates, MANIFEST-SHA written then unmodified — validator self-test case 7's fixture) | **RESULT: OK, exit 0, no STALE anywhere** | **BRIDGE: STALE (exit 2)** — sole grounds: the 3 F2-absence reasons | Validator: **YES**. Doctor: **NO — DEFECT D1** (the exact mislabel this seat exists to re-prove absent). |

Bridge anatomy behind the matrix (`logs/01`, `logs/02`): Onoda
project `onoda.json` has **no top-level F2 keys** (its `film` key is
the title string) and flat `plates.json` (5 plates); MANIFEST-SHA
101 entries, **no drift**. Repo `film/onoda.json` (the data the
binary actually censuses) is the opposite on both counts:
top-level F2 present, manifest-shape plates. LC `onoda.json` has
the B3-01 **invented** F2 values (`calendarYears 1893/1904/1915/1924`,
`calendar-dark.png`, `sign.png`), manifest plates (2: lens-cloth,
oil-can), SHA 68 entries no drift, **no `driver` field** in either
project's `film.json`.

## 4. Binary probe legs (doctor probe + direct `--film-telemetry`)

Doctor probes (`logs/08`) — all fast: every refusal happens at
load/guard, before the audit, so none hit the 120 s probe timeout.

| Probe | Result | Assessment |
|---|---|---|
| LC + doctor **default discovery** (provisioned `~/talerender-bin`, 15 Nov, v1.0.1-era) | `PROBE: INCONCLUSIVE — no verdict line (exit 101)` → `DOCTOR: INCONCLUSIVE`, exit 3, project NOT certified | **Doctor correct.** The old binary *panics* (exit 101) on the LC data where the current binary prints the named guard. The doctor asserts STALE only on positive evidence, so a verdict-less panic is honestly INCONCLUSIVE. This is the era-pairing hazard in the wild: the provisioned slot holds a pre-Batch-1 binary. (R2-02's charter covers the exit-101 surface.) |
| LC + current binary (`--talerender-bin target/release`) | `PROBE: binary refused the film data — film telemetry failed: film data is not staged by the legacy driver (missing anchors: bookseller9, coconuts6, commander8, decades6, flew9, found7, loyal11, message3, never2, only8, order10, refused7, surrender2, surrendered10, thought4, trick4, years0); bridge it with driver: scene-sequence (see REMEDIATION-CONTRACTS §1)` → remedy re-bridge → `DOCTOR: FAILED`, exit 2 | **Correct in full.** With a current binary the probe catches exactly what the structural legs cannot (D2): no `driver` field → legacy routing → named guard, right remedy. 0.3 s. |
| Onoda project + current binary | `PROBE: binary refused the film data — film telemetry failed: plates.json: not a plates manifest (expected {"format": "talerender-plates", "plates": [...]})` → `DOCTOR: FAILED (2 diagnoses)`, exit 2 | **Correct.** The binary's own refusal confirms the flat-plates STALE is real — and note the binary complains about *plates only*, never about the absent F2 keys (further D1 evidence). |
| Fresh fixture + current binary | Structural STALE (D1) **plus** probe refusal: `film manifest …/film.json.segments[0] (s01): missing or non-numeric 'startS'` → `DOCTOR: FAILED (2 diagnoses)` | Probe refusal is a **fixture artifact, not a doctor/binary defect**: the validator's minimal fixture manifest carries `startFrame/endFrame` only, while real `build_bridge.py` manifests (both live projects) also carry `startS/endS`, which the generic telemetry requires. See D3 for the validator-side gap this exposes. The D1 mislabel stands independently on the `--no-probe` structural leg. |
| Missing binary (`--talerender-bin /nonexistent/talerender`, `--no-probe`) | `BINARY: MISSING — no talerender binary found (tried: /nonexistent/talerender)` + full provisioning remedy (`gh run list/download … -n talerender-binary`, `TALERENDER_BIN` export, warning against the repo-checkout slot) → exit 2 | **PASS** — the era-hint provisioning remedy, verbatim in shape from CAPTURE-NATIVE.md. |

Direct binary `--film-telemetry` runs (current binary; `logs/14`,
exit 3 each, all sub-second — refusals precede the audit):

| Target | Binary verdict |
|---|---|
| LC project `native/film` | `film telemetry failed: film data is not staged by the legacy driver (missing anchors: bookseller9, … years0 — 17 named); bridge it with driver: scene-sequence (see REMEDIATION-CONTRACTS §1)` |
| Onoda project `native/film` | `film telemetry failed: plates.json: not a plates manifest (expected {"format": "talerender-plates", "plates": [...]})` |
| Fresh fixture `native/film` | `film telemetry failed: film manifest … segments[0] (s01): missing or non-numeric 'startS'` (fixture artifact — D3) |
| Repo `film/` (Onoda current-gen data) | **No verdict obtained on this VM.** Run reached `sample 40 (t=9.75 of 60.357; ~242 samples)` in ~4 min wall under 3-way cross-seat telemetry contention (B2-08 + R2-03 audits concurrent) on 1.5 shared vCPU, projected completion at/over its 1500 s timeout boundary; terminated by this seat to release CPU (`logs/07`). Prior proof stands: Batch-1 lead recorded `Onoda --film-telemetry CHECK OK` with the Batch-1 binary, and the acceptance census is R2-04's CI dispatch. Classified ENV-BLOCKED (VM CPU class), §5. |

## 5. ENV-BLOCKED list — every leg that rendered no verdict here, and the CI route that covers it

| Leg | Exact block evidence | Covering route |
|---|---|---|
| selftest_native step 4 / selftest.sh leg 0 canary + legs 9–12 (frames/sheet/castsheet/video) | `error: GPU initialisation failed (request_adapter (gpucore)): no wgpu adapter found — a Vulkan driver (hardware or lavapipe) is required` (current binary); stale binary: exit 101, same `no wgpu adapter found` text. No ICD installed; `/dev/dri` absent. | TaleRender CI render workflow (lavapipe, GitHub Actions) — the reference environment named in `tools/SELFTEST-NATIVE.md`; the Batch-2 acceptance CI proof (R2-05, mirroring run 43750140530) covers LC check/frames/sheet/video + Onoda check under scrub+shim. RADV on the user's PC is the other proving ground. |
| Full-film `--film-telemetry` audit on a *passing* bridge (repo Onoda data; selftest.sh legs 6c/7 class) | Audit progresses (`sample 40, t=9.75/60.357` in ~4 min wall, `logs/07`) but cannot complete in reasonable time on 1.5 shared vCPU — compounded by 3 concurrent seats' audits. All *refusal* paths (guard/load) complete in <1 s and were fully exercised (§4); only the passing audit is time-blocked. Doctor probes at the default 120 s were unaffected (they refused fast or, for the stale binary, panicked fast). | TaleRender CI (lavapipe runners) — R2-05's proof legs (LC check + Onoda check) and R2-04's post-remediation census; or a RADV machine. |
| selftest.sh leg 6 + cascades (7, 8, 13) | **Not env-blocked — DEFECT D4.** The builder refusal is deterministic and machine-independent; CI would hit it identically. Listed here so the cascade is not mistaken for an environment effect. | No CI route covers it until D4 is fixed; after the fix, R2-05's CI proof + a full selftest run on CI/RADV. |

## 6. Library (charter item 3)

`scripts/py scripts/library_check.py` → **`LIBRARY OK (479 items)`**,
exit 0 (`logs/03`). Census line unchanged from Batch-1 close:
`114 rendered / 74 kind-exempt / 53 exempt-with-reason / 238 pending`.
`--strict-previews` → `LIBRARY FAIL (1 problems, 479 items)`, exit 1,
the 1 problem being the 238 pending previews (`logs/03b`) — the
honest, already-recorded pre-generator state (R2-09's charter), not
a Batch-1 regression.

## 7. Defects

### D1 — project_doctor still mislabels fresh bridges STALE (R1-03 not propagated) — REAL, with repro
`template/tools/project_doctor.py::bridge_state` treats each absent
F2 key as a stale reason: *"film data has no 'calendarYears'
declaration — the current binary's loader demands it by name and
refuses this data"* (×3). Post-decoupling (Batch-1 `film.rs` patch,
REMEDIATION-CONTRACTS §4) the loader demands nothing of the sort:
absence = empty calendar / no sign. R1-03 patched the validator,
`film.rs`, and `build_bridge.py` — the doctor was not in its patch
set, and still speaks pre-decoupling law.
**Repro:** build the validator's own case-7 fixture (fresh
scene-sequence bridge, no F2 keys, matching MANIFEST-SHA — builder
script preserved in this seat's transcript; fixture at
`scratch/fresh/proj`), then
`python3 template/tools/project_doctor.py scratch/fresh/proj --no-probe`
→ `BRIDGE: STALE`, exit 2, while
`python3 tools/integration/validate_native_project.py scratch/fresh/proj`
→ `RESULT: OK`, exit 0. The same wrong rationale supplies 3 of the
4 reasons in the doctor's (coincidentally correct) Onoda STALE
verdict. **Fix shape (for the lead; this seat edits nothing):**
mirror the validator's R1-03 semantics in `bridge_state` — drop the
F2-absence reasons; keep flat-plates + SHA-drift as the stale
markers; optionally fail by name on present-but-malformed F2
values, as the validator does. LC's project copy of the doctor has
the identical defect (same `bridge_state`; it additionally lacks
the CR-B3-04 inconclusive-probe amendment).

### D2 — structural blind spot: no `driver`-field check (LC passes both structural tools; only a probed current binary catches it)
Neither structural tool inspects `film.json`'s `driver` field, so a
pre-remediation bridge with no driver (LC, as found) certifies
clean structurally while the current binary routes it to the legacy
driver and refuses it at the anchor guard. The doctor's probe leg
*does* catch it — with a current binary, in 0.3 s (§4) — but the
doctor's default discovery resolves the stale provisioned binary,
whose probe ends INCONCLUSIVE (exit-101 panic, §4), so an operator
running the doctor as-shipped on this machine gets "current
generation + not certified", never the named routing defect.
Suggested: a structural `driver` line in both tools (absent =
legacy-routed, named as such — a routing fact, not STALE), so the
LC class of bridge reads truthfully without a probe. R2-03's
re-bridge makes this moot for LC itself; the blind spot remains
for any other pre-remediation bridge.

### D3 — validator does not check `segments[].startS/endS`, which the generic telemetry requires (minor)
The R1-03 validator validates segment tiling on
`startFrame/endFrame` only. The binary's generic (scene-sequence)
telemetry additionally requires numeric `startS` per segment and
refuses by name without it (§4, fresh-fixture probe). Real
`build_bridge.py` manifests always carry `startS/endS`, so only a
hand-assembled bridge can pass the validator and still be refused
by the binary's check — the exact class of surprise the validator
exists to pre-empt ("a project that passes here does not fail
there for a reason this tool could have named first", per its own
header). Suggested: when a manifest declares
`driver: "scene-sequence"`, require numeric `startS/endS` per
segment in the validator.

### D4 — selftest.sh leg 6 can never pass: builder refuses the scaffolded skeleton, script misclassifies it ENV-BLOCKED with a wrong remedy — REAL, machine-independent
Chain, all from this seat's kept run (`logs/13` + script log):
(1) `new_video.sh` scaffolds the template, whose `native/` is a
*skeleton* (`BRIDGE-STATUS.txt`, `film/`, `scenes/`, `renders/`,
`film.json.example` — the template's own `template/native/`).
(2) Leg 6 invokes `build_bridge.py --project $P --scenes …`
**without `--force`**. (3) The builder refuses:
`build-bridge: …/native already contains scenes, film — refusing
to overwrite; re-run with --force` (script log line 77), exits
before building or verifying anything — the kept project has no
`native/film.json`. (4) With no `verify (a)/(c)` lines in the
output, the script's parsers read VA/VC as empty and land in the
`VC SKIPPED/empty` branch, printing
`ENV-BLOCKED: leg 6: bridge built (data legs OK) but its binary
telemetry verification did not run — remedy: provision a binary
(leg 0) and re-run` — every clause wrong for this outcome: nothing
was built, no data leg ran, and a binary *was* provisioned (legs
0/3 had just passed with it). (5) Legs 7, 8, 13 cascade to
ENV-BLOCKED on the unverified bridge. Net effect: **at current
HEADs of both trees, `scripts/selftest.sh` cannot print PASSED on
any machine** — the refusal is not environmental, so CI (R2-05's
proof, if it drives this script) and RADV hit it identically.
**Fix shapes (lead's choice; this seat edits nothing):** pass
`--force` in leg 6 (the project is a throwaway scaffold — force is
safe there), *or* teach the builder to treat a skeleton-only
`native/` (no `film.json`) as overwritable, *or* scaffold without
the skeleton. Independently, the script's VC-empty branch should
surface the builder's own last line before claiming ENV-BLOCKED,
so an early builder refusal can never again masquerade as a
provisioning block.

## 8. NOT RUN list

- `scripts/selftest.sh` full (non-`--cpu-only`) run: legs 9–12 are
  canary-gated ENV-BLOCKED on this VM by the script's own design,
  and leg 6 blocks earlier for a non-env reason (D4); the
  `--cpu-only` run + per-leg equivalents above cover every CPU leg
  the script can reach.
- Repo-film full `--film-telemetry` to a verdict: ENV-BLOCKED
  (VM CPU class), §4/§5 — not "not run" but "run, no verdict
  obtainable here"; the CI route is named in §5.
- Voice/words legs: none exist in the native selftest by design
  (NOT-CHROME pipeline, unchanged) — not a gap.

## 9. Log index

`logs/00` env probe · `01`+`02` bridge anatomy (+ SHA re-verify) ·
`03` library check · `03b` strict-previews · `04` validator
self-test 9/9 · `05` doctor `--no-probe` matrix · `06` validator
matrix · `07` repo-film telemetry (terminated, progress) · `08`
doctor probes · `09` selftest_native SKIP + stale binary · `10`
selftest_native current + provisioned binaries · `11` typecheck /
leg-0 components · `12` adapter pre-flight (LC scratch copy) ·
`13` selftest.sh `--cpu-only` run · `14` direct film-telemetry
refusals. Fixtures: `scratch/fresh/` (case-7 fresh bridge),
`scratch/unbridged-full/`, `scratch/unbridged-bare/`,
`scratch/lc-copy/`. Kept selftest project + full script log:
`~/faceless-shorts/_selftest-native/` (the script's own work dir).
