# TRACE2-W97 — traceability re-run at final HEAD (Wave 2, Batch 10)

**Seat:** W97, Wave 2 Batch 10 (final batch). **Input:** Batch 5's
`GAPFIX-MATRIX-W49.md` (the traceability matrix for the user's
2026-11-14 23:24 +06 gap-fix order), re-run against the current tree,
plus the rows the wave itself created. **Snapshot:** HEAD `2f5c708`
("Fix tables beat test range"), taken while Batch-10 sibling seats
(W93/W94/W95/W98/W99/W100) and the lead were still landing work —
untracked sibling files in the working tree at snapshot time are
named where relied on. **CI at snapshot (checked first-hand via
`gh`):** CI run `43630186620` **SUCCESS** and Tools CI run
`43630186614` **SUCCESS**, both at `2f5c708`. The immediately prior
head `9380b01` had CI **FAILURE** (run `43629817118`, one unit test —
see row W-14); `2f5c708` is its fix and is green.

**Method (the standing law, applied):** every row was re-verified
against primary evidence — source at HEAD (file:line greps quoted),
git log, workflow/run state via `gh`, or a tool's own output file
read directly. The wave record (`WAVE2-RECORD.md`) and W49 itself
were treated as claims, not evidence: this wave's record has been
wrong before (row W-01), and two of W49's "still open" items turned
out to have landed since (D3, D4 — both fixed by `d967931`, which
neither W49 nor the Batch-8 remainder list anticipated being the
last word; they are verified in source below, not from the commit
message alone).

**Status vocabulary (as commissioned):**
- **LANDED** — in the tree at HEAD, commit cited, substance verified
  in source/runs.
- **STAGED** — a pack/spec/staging exists and is named; the landing
  itself has not happened.
- **DECISION-OWED** — blocked on a named decider (user / lead).
- **REJECTED** — deliberately not done; the standing reason is cited.
- **STALE-CLAIM** — the audited claim was not true of the tree at
  any audited head; the disproof is cited.
- **UNRESOLVED** — cannot be closed from available evidence; the
  exact missing evidence is named.

**Counting convention (read before the count table):** counts are
of *matrix rows*, not of distinct work items. W49's own structure
deliberately overlaps (F-rows, G-rows and R-rows re-ask the same
work from three angles), and this re-run preserves W49's row
identity so the two matrices diff cleanly. Where W49 split a row
into parts with different dispositions, the parts are lettered
sub-rows and **each sub-row counts once**. A sub-row that merely
points at another row is marked `— (cross-ref)`, carries no status,
and is **not counted**. Every total in §10 is the sum of the
statuses printed in §§1–9 under this convention.

---

## 1. External audit — stale-item register (WAVE2-RECORD §3, S1–S8)

The audit of `talerender-main.zip` was written against an archive
predating several landed fixes. The wave's first duty was not to
redo these. Re-verified:

| Row | Status | Evidence at HEAD `2f5c708` |
|---|---|---|
| S1 "Resolve MP4 BT.709 metadata" (audit Phase 1) | **LANDED** (pre-wave) | `6ca97fc`; verified in source: `film/finish.sh:29–30` passes all three `-color_* bt709` flags **plus** the x264 VUI params (`-x264-params colorprim/transfer/colormatrix=bt709`); the f021991 proof MP4 probes all-bt709 (MP4-EVIDENCE). Not redone. |
| S2 "Re-render the complete film at the latest source revision" (Phase 1) | **LANDED** (pre-wave) | Full 1,811-frame re-render at `f021991`; A/B vs Chrome mean 23.02 dB / min 19.36 (lead completion report, `~/workspace/film-final/ab-out4/`). Not redone; Wave 2's later changes were gated by byte-probes against that set instead (record §7 passim). |
| S3 Determinism "root cause not conclusively established" (audit §A/§E) | **LANDED** (pre-wave) | Root cause found and fixed pre-wave: `state_key` quantised continuous channels to 2 dp → cache-key collisions; keys are now exact f32 bit patterns (`cabcc30`, E0600 fallout fixed in `f021991`). The freeze still holds at HEAD: Batch 9's W82/W90 re-extracted `state_key` at film.rs:2246, 25 lines, md5 `95918b9941313f895dbee7bf35c8d14`, byte-identical across five heads. |
| S4a "Formal determinism PASS outstanding" — CI lavapipe leg (§E) | **LANDED** | Determinism contract (row P1.3) + `determinism.yml`; E6 dispatch run `43617370962` SUCCESS at `3a36f1f`; re-dispatches `43620084786` and `43624016907` also SUCCESS (record §7). |
| S4b "Formal determinism PASS outstanding" — RADV leg (§E) | **UNRESOLVED** | Missing evidence: any RADV determinism artifact. No RADV execution has ever occurred (this VM has no GPU; every native number is CI/VM lavapipe). The discharge route is prepared (W36 runbook §1 = the contract's §7 procedure; W78's G1/G2 paste-ins landed `108f33f`) but the run itself is owed by the user's-PC session. See also E-b, G18. |
| S5 "Seven caption failures remain unresolved… no ruling applied" (§C) | **STALE-CLAIM** | A ruling existed before the audit was written: lead ruling ACCEPT WITH WAIVER, `9790553` (`docs/ab/film/CAPTION-DECISION-W92.md`). The claim "unresolved, unruled" was never true; the *engineering* question was reopened by the user's Wave-2 order and is traced at P1.1/C. |
| S6 "Error handling: no --help, panics on bad input" (§D, CLI half) | **LANDED** (pre-wave) | Professional CLI at `d7bebdd`: full `-h/--help`, `-V/--version`, CLI errors print `error: …` + help hint, exit 2. Verified at HEAD: `Cargo.toml` version still `1.0.0`, `--version` derives at main.rs (W89 census: main.rs:274→280 across landings). The runtime-path half is P2.3/D, not this row. |
| S7 "112 Rust test functions… cannot claim they passed" (§7) | **LANDED** | The audit's environment had no cargo; this project's verification route is CI, and the suite runs on every push (ci.yml Test step). Current scale: 180 tests (179 passed / 1 failed at `9380b01` — the failure is row W-14, fixed at `2f5c708`, CI SUCCESS there). |
| S8 Risk register "still describes early milestones as unmitigated" (§7) | **LANDED** | Refreshed in Batch 1 (`3a36f1f`, "risk register refreshed" — `git log -- docs/risk-register.md` confirms `3a36f1f` as the last touch, inside Wave 2). Also counted at P5.6 (same work, phase angle). |

**§1 subtotal: 9 rows — LANDED 7 · STALE-CLAIM 1 · UNRESOLVED 1.**

---

## 2. External audit — findings A–E

The audit's five lettered findings, as cited through the record
(§A determinism, §B duplication table, §C captions, §D error
handling, §E formal determinism):

| Row | Status | Evidence |
|---|---|---|
| A — Determinism root cause unknown (§A) | **LANDED** | Same disposition as S3: `cabcc30` (exact f32 bit-pattern keys); the eviction theory was falsified (pattern repeated digit-for-digit across independent fleets). Contract + CI proof at P1.3/S4a. |
| B — Duplication: instance descriptor ×5, device request ×5, readback ×7 (§B table) | **LANDED** | gpucore migration 5/5: dof `df89ad6`, scenefile, ab1 `9f497c7`, main `dc0e555`, film+F1 `aadc193` (which also deleted main.rs's duplicate `request_adapter`). Per-site byte-identity proofs in record §7 (dof chart `957d5573…`, fx scene `6ef2e39b…`, film probes 30/30 vs `f021991` bytes). At HEAD: `mod gpucore` registered (main.rs:27); the sole `request_adapter` implementation is `gpucore::request_adapter` (verified by W82's grep census). |
| C — Seven caption failures (§C) | **LANDED** | Re-chunk 66→73 chunks (computed splits, W01 data) landed data-side `3a36f1f` and driver-side `5e90d7e`; telemetry gate re-run prints `FILM TELEMETRY: CHECK OK`, exit 0, 850 px threshold untouched. Verified at HEAD first-hand: `film/onoda.json` carries exactly **73 captions** (counted), and W68's dry-run independently observed `CHECK OK` / exit 0 from the CI binary. The Wave-1 waiver remains the documented fallback if vetoed (C-veto). |
| C-veto — caption re-chunk: confirm or veto | **DECISION-OWED (user)** | Decision-ready brief: CAPTION-DECISION-W69 (all 7 splits verbatim with film times; veto mechanics priced — revert `5e90d7e` + caption paths of `3a36f1f`, 3 in-window golden pins go stale; recommendation on record: KEEP 73). Assembled into the wave-close decisions brief (Batch-10 W94). |
| D — Error handling: panics, no exit-code scheme (§D) | **LANDED** | Exit-code contract implemented (X1); structured errors step 1 `ba88596` + step 2 `edb058f` (fix `aa92cee`); no-adapter proof: CI binary with `VK_ICD_FILENAMES` pointed nowhere → `error: GPU initialisation failed …`, **exit 3, no panic** (record §7). **Residue, disclosed:** `gpucore::save_png` (gpucore.rs:352–353) still carries two `expect()`s ("pixel buffer size mismatch", "failed to save PNG") — but grep at HEAD shows **zero call sites** outside gpucore.rs, so it is a latent helper, not a live panic path; the live save paths spell the image crate directly. Converting it belongs to the next errors pass; it does not block this row's disposition. |
| E-a — Formal determinism PASS, CI lavapipe leg (§E) | **LANDED** | = S4a (contract + E6 SUCCESS + two re-dispatches SUCCESS). |
| E-b — Formal determinism PASS, RADV leg (§E) | **UNRESOLVED** | = S4b. Missing evidence: the PC-session RADV pack output (adapter proof + determinism pack + PERF numbers). Prepared, never run; the wave claims no RADV number (record law 7). |

**§2 subtotal: 7 rows — LANDED 5 · DECISION-OWED 1 · UNRESOLVED 1.**

---

## 3. External audit — Phases 1–5 (record §4 work list)

### Phase 1 — Production correctness

| Row | Status | Evidence |
|---|---|---|
| P1.1 Caption re-chunk applied | **LANDED** | = finding C. Data `3a36f1f` + driver leg `5e90d7e`; gate `CHECK OK` exit 0; 73 captions counted in `film/onoda.json` at HEAD. |
| P1.2 Golden re-stage from `f021991`; pins as separate deliberate acts | **LANDED** | Act 1 `916a109` (14 film first-pins) and Act 2 `166873c` (ab1 → `60c32c1a…`, fresh render 23.98 dB) struck as separate commits, per W03's prescription. The Act-1 defect the re-stage itself introduced is row W-02 (corrective `2755ff3`, independently verified by W63). Verify state: 18 OK / 0 STALE / 0 MISSING (W63; re-confirmed by W68's dry-run at the corrected state and by W90). |
| P1.2s s28 golden-pin disposition | **DECISION-OWED (user)** | s28's pin (`ff68fad3…`) still describes the M2-era §28 artefact; post-corrective VR run `43625139818` and the `3c07564` run `43628892930` both read FAIL with s28 the **sole DRIFT** — the designed steady state. Options priced in GOLDEN-RESTAGE-W03 / W63 §7 (re-pin — recommended against, it would byte-duplicate ab1 — retire the item, or leave red). |
| P1.3 Determinism contract + CI lavapipe leg | **LANDED** | DETERMINISM-CONTRACT-W04.md + `determinism.yml` (dispatch-only); E6 run `43617370962` SUCCESS; contract's RADV leg is S4b/E-b. |

### Phase 2 — Rendering architecture

| Row | Status | Evidence |
|---|---|---|
| P2.1 Shared GPU core (one module for adapter/device/readback) | **LANDED** | = finding B (gpucore 5/5, `aadc193` completes). |
| P2.2 Modularise film.rs / scene.rs (split programmes) | **STAGED** | Blueprints FILM-SPLIT-W21 (10 steps) + SCENE-SPLIT-W22 (13 steps); readiness SPLIT-READY-W66 (`state_key` freeze md5 `95918b99…`); amendments SPLIT-AMEND-W82 (G-TEL gates re-worded to `CHECK OK`/exit 0, void adapter paragraph struck, 22-commit serial protocol). **Not landed, verified:** `src/film.rs` is 3,787 lines and `src/scene.rs` 2,979 lines at HEAD — both still monoliths. W82's stated precondition (a standing 46/46 stroke-battery invocation) is row W-17. |
| P2.3 Structured errors (runtime paths return structured errors) | **LANDED** | `src/errors.rs` (W05) registered main.rs:26; step 1 `ba88596` (load paths), step 2 `edb058f` + `aa92cee` (gpucore's adapter/device/collect sites → `TaleResult`, exit 3); no-adapter exit-3 proof (record §7). Residue: the zero-call-site `save_png` expects noted at finding D. |
| P2.4 Structured configuration (env knobs consolidated, `--help` in step) | **LANDED** | The env surface is documented in HELP itself (main.rs:124 `TALERENDER_PAINT_CACHE <N>`, :128 `TALERENDER_PAINT_BUDGET_MB <N>`, plus the frame-timing switch) and owned by the modules that consume it: `PaintBudget::from_env` (paintbudget.rs:146), paint-cache cap read at film.rs:2212, `TALERENDER_FRAME_TIMING` at runlog.rs:162. CLI-side configuration is the `Knobs` struct (main.rs:182; F4). |

### Phase 3 — Performance engineering

| Row | Status | Evidence |
|---|---|---|
| P3.1 Paint-cache byte budget (bytes, not just entries; pixel-neutral eviction) | **LANDED** | `src/paintbudget.rs` (W06) wired `86f0c81`; budget-forced-eviction byte-probes PASS (795–814 20/20 + 1195–1204 10/10 byte-identical vs defaults, record §7); W68's dry-run observed `budget_bytes=2147483648` with `peak_bytes=2162688000` in real telemetry output on this VM. |
| P3.2 Per-frame timing behind `TALERENDER_FRAME_TIMING=1` | **LANDED** | runlog.rs:157–162 implements the switch; film progress lines call `clock.timing_suffix(...)` (film.rs:3553). (Progress elapsed itself is X2.) |
| P3.3 Buffer pooling (where the shared core makes it safe) | **STAGED** | `src/bufferpool.rs` (W19) exists and is registered (main.rs:22), with its adoption design in BUFFERPOOL-W19 — but grep at HEAD shows **zero call sites** outside its own file (independently restated by W84: "BufferPool still has zero production callers"). Its bytes enter the VRAM ledger only if the Readback adoption lands (P3.6 pack). |
| P3.4 Texture atlas / dirty-region updates (only if proven pixel-identical) | **STAGED** | `src/atlas.rs` (W18) registered (main.rs:20), zero call sites at HEAD (grep-verified). Adoption rule accepted by the lead (record §9.4: probation admission policy + byte-probe discharge post-wiring) — the wiring itself has not happened, per the audit's own "only if proven" condition. |
| P3.5 Bench harness for the PC (PERF protocol automation; no VM numbers) | **LANDED** | `bench.yml` (dispatch-only, W17) + BENCH-W17.md; first dispatch `43623743480` SUCCESS (CI lavapipe 1.85 s/f in-window; W64's anomalies recorded: bench window is S01-only, accounted-vs-resident cache bytes diverge ~9× — neither blocks). RADV numbers remain G18's missing evidence, not this row's. |
| P3.6 VRAM telemetry (`gpu-mem:` accounted-bytes line) | **STAGED** | Design VRAM-TELEMETRY-W56 (film lane accounted 220,930,688 B steady / 229,286,528 B peak by formula) + implementation staging VRAM-IMPL-W84 (census re-derived at HEAD; formula reproduced byte-exact by W84 and by gate W90). **Not in the binary, verified:** no `accounted_bytes` / `gpu-mem` string exists in `src/` at HEAD. W84's two staging rulings (bloom is per-call; telemetry must not print a fabricated zero) are the lead's to take at landing. |

### Phase 4 — Animation & visual quality (all opt-in)

| Row | Status | Evidence |
|---|---|---|
| P4.1 Animation evaluator + channels (anim/charanim; film path behind a gate) | **STAGED** | Modules landed and registered (`2328e6f`: main.rs:19 `anim`, :23 `charanim`; tests in the CI suite). Adoption into the film path: exactly one channel crossed — blink (P4.2). `crate::anim::` has **zero production call sites** at HEAD (grep-verified; the only hits are doc comments inside the unregistered cameramoves.rs). The clips-loader adoption that would carry authored channels is P5.3. |
| P4.2 Blink channel adoption | **LANDED** | `5a9383e` (W58 pack: `max(channel, blink_at(beats))`); call sites verified at HEAD: film.rs:1444 and :1491 call `crate::charanim::blink_amount`. Gate was exact vs the census promise (only f204 / f814 changed in their probe windows; the 7 authored beat-blinks survive bit-exactly — 25 beat frames, 0 changed). |
| P4.3a Locomotion — registration step 0 | **STAGED** | `src/locomotion.rs` (W24, gait laws 9/9 tests) exists but is **not in main.rs's mod list at HEAD** (verified: mods run ab1…toonfull with no locomotion). Step-0 pack LOCOMOTION-STEP0-W87 specifies the one-line registration + a zero-pixel proof protocol (40/40 byte-probes + telemetry `CHECK OK`). |
| P4.3b Locomotion — adoption (bob phase correction + arm swing) | **DECISION-OWED (user)** | Motion-quality sign-off is the user's by design (W57 census: 471 walking frames; arm swing is NEW motion on 448 frames; standing re-level ~1.9 px in s10). Decisions D1–D4 with recommendations + a plain-language sheet are in LOCOMOTION-STEP0-W87 §3; assembled into the W94 decisions brief. |
| P4.4 Camera moves vocabulary | **STAGED** | `src/cameramoves.rs` (W25, 11/11 tests) exists, unregistered at HEAD (same mod-list check), zero call sites. The interim route — exporter samples rigs into §14.4 camera keys, which the loader does consume — is G11's landed half. |
| P4.5 Lipsync binding (viseme track → film path) | **STAGED** | Track built (W23, 91.1% dictionary coverage); evaluation LIPSYNC-EVAL-W59 (the oscillator never drives narration in this film; step semantics fix closures 6→36/36); binding pack LIPSYNC-BIND-W86 recommends Variant A (builder-side closed-exit step keys — measured equal to full-step on all three leak classes; corrects W59's wording by measurement) and prices Variant B. Binding itself not landed. |
| P4.5s Lipsync — line-3 sign-off sheet | **DECISION-OWED (user)** | The judgement artefact is specified in LIPSYNC-BIND-W86 §5 (line 3, frames 482–632, three bindings via make_sheet; headline check = the 17-frame silence after "ended.", frames 570–586). Renders are CI/PC work; the decision is the user's. |

### Phase 5 — Engine maturity

| Row | Status | Evidence |
|---|---|---|
| P5.1 Scene-format versioned extensions (prefabs, clips, camera tracks) — spec + fold-in | **LANDED** | §15 appended verbatim to `docs/scene-format.md` at `4dd6530`; verified at HEAD: §15 begins at scene-format.md:559 ("Format v2 extensions: prefabs and clips"); the document is now 1,288 lines. Post-fold census 75/75 clauses, none weakened (W65; addendum splice `108f33f`). |
| P5.2 §15 reservation guard (reserved blocks fail loud) | **LANDED** | Verified in source: scenefile.rs:512–516 (`prefab`/`clip`/`clipTime` → `Err("… extension not supported")`), :856–860 (top-level `prefabs`/`clips` → Err), film.rs:82 (film-data side). Unknown top-level blocks fail loudly instead of being silently ignored. |
| P5.3 §15 loader consumption (prefabs expanded, clips evaluated) | **STAGED** | The guard's own source comment states the state: "Until the implementations land (§15.10), check_prefabs.py / check_clips.py are the only consumers of these keys" (scenefile.rs:854–855). The Python conformance checkers exist and pass on the real corpus (W51); the Rust loader implementation is post-wave lead work. This is also F3-c's `materials` either/or context (F3c). |
| P5.4 CI job: visual regression (golden verify) | **LANDED** | `visual-regression.yml` (dispatch-only, W16) + register/fixtures updated `3c07564` (W72 pack). Runs at HEAD-era heads read FAIL **as designed** — sole DRIFT s28 (P1.2s); the register patch's anti-whitelist probes bite (W72/W90 verified a rendered non-canary drift still surfaces as NEW-SIGNAL). |
| P5.5 CI job: determinism | **LANDED** | `determinism.yml`; E6 + re-dispatches SUCCESS (P1.3). |
| P5.6 Docs: risk-register refresh | **LANDED** | = S8 (`3a36f1f`). |
| P5.7 Docs: architecture/state documentation matching the post-refactor tree | **LANDED** | DOC-SWEEP-W08 (Batch 1) + DOCFIX2-W67 applied at `c7aa648` (FINAL-REPORT Addendum 2 refreshed, root README gains the CLI pointer + CHECK OK clause + struck-pins clause, PERF corrections incl. the exit-code rows reworded to the post-`edb058f` truth, film README's five stale lines replaced). ADR-001 remains the M0 decision record, unmaintained by design (ADRs are immutable); the living state documentation is the README/FINAL-REPORT layer the docfix packs refreshed. |
| P5.8a s12 bake (the `apply`-transform blocker on the clips census) | **LANDED** | W52 proved both transforms are pure scalar multiplies; baked staging converts, census completes 65/65 records / 14/14 files, equivalence FAITHFUL (bake contribution ≤ 1 ulp, key-times bit-identical). Re-proven at HEAD by W83 (drift guard PASS, regenerated output byte-identical to the committed staging). |
| P5.8b s12 bake adoption (baked data in, code path retired) | **STAGED** | Adoption pack S12-ADOPT-W83 (verdict READY, zero drift). **Not landed, verified:** film.rs still evaluates the pre-bake form — `let bow = pose_track(…)` with `onoda.lean = (0.6 * bow) as f32;` / `onoda.head_tilt = (-bow) as f32;` present in `fn s12` at HEAD. The pack prescribes the retirement (film.rs:2021–2023) + post-adoption probes over frames 1666–1810, which no standing probe set covers. |

**§3 subtotal: 30 rows — LANDED 17 · STAGED 10 · DECISION-OWED 3.**

---

## 4. External audit — §6 improvements (exit-code contract + PERF §9 asks)

The audit's §6 improvement set, as adopted by the lead (record §6
contract; PERF §9.1–9.4):

| Row | Status | Evidence |
|---|---|---|
| X1 Exit-code contract (0 OK / 1 QA-failed / 2 bad input / 3 GPU / 4 I/O; verdict line stays authoritative) | **LANDED** | `aa10b13` implements the film-telemetry legs; CLI errors exit 2 since `d7bebdd`; GPU failures exit 3 since `edb058f` (no-adapter proof, finding D). Adapter dual convention re-proven independently 8/8 by W62 (row W-12). HELP's table is now complete and accurate: main.rs:136–145 lists exit 4 ("Image I/O failure") — that completeness is D4's fix, `d967931`. |
| X2 PERF §9.1 — progress lines carry elapsed seconds | **LANDED** | RunLog wiring `86f0c81`; film progress lines append `clock.timing_suffix(...)` = `elapsed MM:SS · eta …` (film.rs:3553; runlog.rs:130–133). |
| X3 PERF §9.2 — paint-cache stats line (hits/misses/evictions/bytes/peak) | **LANDED** | PaintBudget wiring `86f0c81` (record §7: suite 175/175 at the wiring head); the completion line's composition contract puts the cache-stats segment last (runlog.rs:135–140); W68 observed the stats lines and budget figures in real output. |
| X4 PERF §9.3 — peak-RSS self-report on exit | **LANDED** | `clock.summary_suffix()` on film's completion line (film.rs:3559) = `elapsed … · peak RSS {mb} MB` via `peak_rss_mb()` (runlog.rs:143–145; "unavailable" off-Linux). |

**§4 subtotal: 4 rows — LANDED 4.**

---

## 5. Gap-fix order — the F-rows (W49 §A, re-verified)

| Row | Status | Evidence |
|---|---|---|
| F1 `--film` reads `onoda.json` while help says `film.json` | **LANDED** | `aadc193`; verified at HEAD: film.rs:69–76 — `load_film_data` prefers `film.json`, falls back to `onoda.json`, with the F1 comment in source. |
| F2 Film driver demands Onoda-specific assets unconditionally | **LANDED** | Loud fallbacks `a2ec80c`; migration complete `20a81a7` + `2c41313`. Verified at HEAD: the loader **demands** `calendarYears` (film.rs:120–132, hard `Err` on wrong shape), `calendarDark` (:145) and `sign` from the data; `load_assets` (film.rs:2102+) treats `plates.json` as a manifest — format-checked, duplicate names rejected, missing `file`/`origin` hard errors (:2111–2145). Record gates: isolation diff zero frames; calendar-range diffs blink-scale only. |
| F3a Unknown JSON fields — checker + report-only ratchet | **LANDED** | `tools/format/check_unknown_fields.py` + `tools/format/unknown_fields_baseline.json` wired into tools-ci (`8b9ac36`). Baseline file verified at HEAD: `totalOccurrences: 24`, `distinctPaths: 9`, `toleratedOccurrences: 7043`. |
| F3b F3-a strip (174 dead-provenance occurrences) | **LANDED** | `2fcffc0` — generator + data in one commit, baseline lowered 196 → 24/9. The strip is 172 occurrences / 6 paths, not 174: root `fps`/`frames` were re-classed **F3-a-kept** after W74 proved hard readers in the Python tools (`tools/perf/blink_cost.py:351,359`, `blink_census.py:169,179` — cited in the baseline file itself). Verified: `film/onoda.json` still carries `fps: 30, frames: 1811` at HEAD, baselined as kept. |
| F3c F3-b (19 prose `note`/`motifLaw`) + F3-c (bookshop `materials`/`near`) dispositions | **STAGED** | F3BC-W88.md + staged tree `docs/integration/f3bc/` (s10/s11/s12/bookshop + lowered baseline — all present at HEAD). Dispositions: all 19 F3-b items CITE → strip-with-citation-recorded (zero KEEP); F3-c `materials` STRIP via decomposition into s10's form; `lights.key.near` CONSUME (one `f32_field_or` line — deliberately asymmetric: `near` completes an existing rig whose other values film.rs already hardcodes). Staged corpus re-run by W88 and gate W90: **3 occurrences / 3 paths / 7,047 tolerated**; survivors `fps`, `frames`, `near` (near exits when the consume landing lands). The landing commits are the lead's. Note the disposition evolved from the record §10's initial "CITE, don't strip" ruling to W88's cite-then-strip-with-citation-recorded; the citations are recorded in F3BC-W88 either way. |
| F3d Unknown fields as hard errors (general) | **DECISION-OWED (lead / format)** | The §15 reservation guard (P5.2) hard-errors the reserved blocks today; a *general* hard error on any unknown field was explicitly deferred to a format-v3 decision (W42 Option A adopted: report-only + ratchet). Not a user decision (W49's format-decision note stands); it is owed by whoever calls format v3. |
| F4 No `--res`/`--aa`/`--shadow` knobs | **LANDED** | Implemented for real after the record error (W-01): phase 1 `ea98458`, phase 2 `9ad2177`, phase 3 `76b922c`; gate workflow `2017e86`. Verified at HEAD: `struct Knobs` main.rs:182, parse arms `--res`/`--aa`/`--shadow`/`--draft` at main.rs:367–379, draft-only by law (defaults are the identity; `is_draft` carries all three terms). **Acceptance (W81, first-hand):** two independent knobs-gate dispatches at `c440d90` — runs `43628289244` and `43628339967`, both SUCCESS, exact line `KNOBS GATE: PASS` (leg A 40/40 byte-identical + no DRAFT tag; leg B exact DRAFT tag at 540×960, frames differ as required; leg C 10/10 invalid cases exit 2). |
| F5 No native sheet mode | **LANDED** | `tools/film/make_sheet.py` (W44; tests 10/10) + tools-ci sheet step wired `8b9ac36` (its red window is incident W-06). |
| F5b A `--sheet` mode inside the binary | **REJECTED** | Lead ruling (record §10): a binary sheet mode is NOT duplicated on top of the W44 tool; the adapter covers review at the frame(t) seam. Standing reason, not an oversight. |
| F6 Cues export (design only, per the order) | **STAGED** | CUES-W45.md — the emitted-entry shape is specified and marked PROPOSED; no native cues emitter was in this wave's scope (the native finish muxes the existing `data/mix.wav`, cues baked in, unchanged). |
| F6q F6 Q1 — the exact cue entry shape | **DECISION-OWED (user)** | Record §10: "the skill-side Q1 stays open for the user." It gates any future native cues implementation, not anything landed here. |
| EXP Exporter architecture (analysis §7: Chrome stays authoring runtime, exporter emits scene/film data) | **STAGED** | Spec EXPORTER-W47.md (acceptance ≥45 dB/sample + telemetry + determinism) + acceptance harness `tools/format/exporter_acceptance.py` (W77; self-test PASS 3/3 per gate W80). The exporter itself is not built; every "generic project" remainder (G1c, R1b, R5b) routes through it. The harness's own note is row W-15. |

**§5 subtotal: 12 rows — LANDED 6 · STAGED 3 · DECISION-OWED 2 · REJECTED 1.**

---

## 6. Gaps G1–G19 (W49 §B, re-verified)

| Row | Status | Evidence |
|---|---|---|
| G1a Generic film input — entry filename | **LANDED** | = F1 (`aadc193`). |
| G1b Generic film input — unconditional assets | **LANDED** | = F2 (migration complete; loader demands the data at HEAD). |
| G1c Generic film input — data-driven shot table / title / fps | **DECISION-OWED (lead, post-exporter)** | Still true at HEAD, verified: the title string is hardcoded (film.rs:3523 and again :3745 in the telemetry header), fps is a literal (`frames_total = (data.duration * 30.0).ceil()` :3517; `t = n as f64 / 30.0` :3534), and the shot table remains 14 code-staged shot functions. W49 flagged this as "a post-W47 lead decision — flagged OPEN as a decision, not lost"; it remains exactly that. A second project still cannot run end-to-end (R1b). |
| G2 Scene format undocumented + hostile | **LANDED** | `docs/scene-format.md` is 1,288 lines at HEAD, §§1–15 (the §15 fold-in is P5.1); load-path errors are `TaleError`/`Result` (P2.3), not exit-101 panics. The residue W49 assigned here (unknown fields) is the F3 family, traced in §5. |
| G3 No text rasterizer | **REJECTED** | Mechanism rejected under the standing strips contract (pre-rendered caption strips composited post-grade; "the renderer typesets nothing" — film driver header). Verified: `Cargo.toml` at HEAD carries exactly three dependencies (wgpu 0.20, pollster 0.3, image 0.25) — no font stack exists or is planned. The outcome the requirement wanted is R3b. |
| G4a Word-timing/chunking parity — reference film | **LANDED** | `film/gen_film_data.py` transcribes `buildCaptions()` over the project's real `script.json`/`lines.json`/`words.json`; the 73-chunk layout gates `CHECK OK` (P1.1). |
| G4b Word-timing/chunking parity — the auto-chunk rule | **UNRESOLVED** | Still true at HEAD, verified: `film/gen_film_data.py:75` still raises `SystemExit("this film's script always sets caps; autoChunks not transcribed")`. W49 recommended folding it into W47's exporter scope; grep of EXPORTER-W47.md finds **no** auto-chunk coverage — the fold-in never happened on paper either. Missing evidence: an owner — either an exporter-scope amendment naming the auto-chunk rule (short function word rides the next, ≤ 11 chars) or a transcription seat. Until then any auto-chunked project (e.g. headless-chicken) cannot be data-generated. = D6. |
| G5 No audio path / no cues in the binary | **REJECTED** | The binary emits PNG frames only, by design: cues are extracted once and baked into `data/mix.wav`, which the native finish muxes unchanged (G16/R8a); `mix.py` is untouched per the analysis's own non-goal. The fully-Chrome-free authoring question is the F6 design (STAGED), not a binary audio path. |
| G6 No shot model / duration source in the binary | **REJECTED** | A native shot model was not adopted; the wave's architecture (following the analysis §7) is an exporter emitting driver data (EXP, STAGED). Duration enters via `film.json`'s `duration` key today. |
| G7 `check` has no equivalent — telemetry tables missing | **LANDED** | The exit-code half was already stale at W49 (`aa10b13`). The tables half landed at `3223509` ("G7 tables in the binary: --film-telemetry --tables", built to TABLES-IMPL-W85's spec): verified at HEAD — `--tables` arm at main.rs:352; `SHOT TABLE` (telemetry.rs:1298), `CAPTION TABLE` (:1342), `CAST LIST` (:1403) all print from the telemetry path, default output unchanged (tables are opt-in, printed after the verdict line, no exit-code interaction per the spec). Disclosed deltas vs the W75 prototype are enumerated in TABLES-IMPL-W85 §4 (audit-attributed frame ranges can differ 1–2 frames from manifest tiling; cast frames are sampled presence ×7.5). The landing's own CI incident is W-14. |
| G8a Cast/continuity source — reference film | **STALE-CLAIM** | The gate computes `sig` itself: `look_signature()` (src/telemetry.rs, transcribed from cast.ts) over the driver's own look registry, tracked per instance id. True at W49's HEAD and at this one; the "no source" claim described the v1.0.0 binary's surface, not the tree. |
| G8b Cast/continuity source — generic projects | **STAGED** | For arbitrary projects the cast/look source is exporter-supplied data → EXP (§5). |
| G9 Characters: "painter exists, nothing feeds it, state static" | **STALE-CLAIM** | Disproven for the film driver at every audited head: per-frame state is evaluated in code (`pose_track`, `walk_phase`/`walk_bob`, `blink_of`, mood/arm/leg channels); the Wave-1 film itself (1,811 frames, A/B 23.02 dB) is the standing counterexample. The honest residues W49 attached (scene-file lane channels unconsumed; locomotion/cameramoves unregistered) are traced at P4.1/P4.3a/P4.4/D7 — not as part of this claim. |
| G10a Set kit / atmospheres — "none exist" | **STALE-CLAIM** | A set vocabulary existed by W49 and persists: `scenes/atmospheres.json` + `scenes/sets/` (both in the strictness corpus at HEAD), format §4.3 scatter / §4.4 code-ref node kinds, preset-named atmospheres (ruling R5). |
| G10b Set kit — TS `world.ts`/`props.ts` exporter | — (cross-ref) | = EXP (§5), STAGED there; not recounted. (Porting the kit natively was the analysis's expensive option and is rejected in favour of export — see G6's reasoning.) |
| G11 Camera rigs / motion vocabulary | **STAGED** | Landed half: format §14.4 camera keys with the house easing law are consumed by `eval_camera_keys` (src/scenefile.rs); W32's staging tool converts 63/65 key-carrying records (65/65 with the s12 bake, P5.8a). Remainder: `src/cameramoves.rs` unconsumed (P4.4) and the exporter route (EXP). |
| G12 Quality knobs fixed | **LANDED** | = F4, including the knobs-gate acceptance (two PASS runs at `c440d90`). |
| G13a Per-shot look "not demonstrated" | **STALE-CLAIM** | Per-shot look exists two ways and did at W49: format-side, each scene file carries its own `post` block (§9/§14 grade keys, enforced by W26's validator 6/6 over all 14 real scenes); driver-side, each shot function sets its own `FrameSpec` post. The ab1 `exposure = 1.0` the analysis cited is the §28 reference scene's fixed staging, not a pipeline limit. |
| G13b Cross-shot look transitions (blended grades) | **REJECTED** | No contract in either engine — Chrome grades per shot discretely too. W49 flagged this "so the row is a decision, not an oversight"; the decision stands. |
| G14 No frame fade | **STALE-CLAIM** | Disproof by direct source read, re-verified at HEAD: film.rs:2073–2074 computes `fade_factor = clamp(t/0.25).min(clamp((duration−t)/0.4))` (fade-in 0.25 s, fade-out 0.4 s — exactly the `main.ts` values) and `apply_fade` (film.rs:3494) is applied after the shot draw, before captions (main.ts's order). The analysis inferred the absence; it was never true of the film driver. = D1. |
| G15a Frames mode | **LANDED** | `--film --from/--to` renders PNG ranges with resume (pre-existing); the skill work-copy adapter exposes frames mode over it. |
| G15b Sheet mode | **LANDED** | = F5 (W44 tool + CI step). |
| G15c Preview | **REJECTED** | No native equivalent, by design: a live preview page is inherently the browser's; the work-copy adapter refuses it as Chrome-only, documented. The analysis itself rated preview "nice-to-have" (§5 row 14). Revisit only if the user asks. |
| G16 No deliverable stage | **LANDED** | `film/finish.sh` wraps the PNGs at the skill's finishing settings — verified at HEAD: `-c:v libx264 -preset slow -crf 18` (finish.sh:28), full BT.709 flags + VUI params (:29–30), AAC (D3), `+faststart`, `.part` discipline; proven end-to-end by the delivered proof MP4 (1,811 frames, 60.457 s, −14.00 LUFS, all-bt709 probe). |
| G17a Operational readiness — "source not in VCS" | **STALE-CLAIM** | The matrix is written inside the repo it claimed wasn't versioned (`turjaun985123/talerender`, private, full history). |
| G17b Operational readiness — CLI/load-path panics | **LANDED** | CLI surface `d7bebdd` (exit 2, no panics on bad input); load paths `ba88596` (errors step 1). |
| G17c Operational readiness — GPU-init/readback `expect()`s | **LANDED** | Collapsed by the gpucore migration to gpucore's own sites, then converted by errors step 2 (`edb058f`): adapter/device/buffer-map now return `TaleResult` → exit 3 (no-adapter proof at finding D). Residue: `save_png`'s two expects, zero call sites (finding D). |
| G17d Operational readiness — `--help` accuracy residue | — (cross-ref) | = D4 (§8), LANDED there; not recounted. |
| G18 GPU portability (RADV evidence) | **UNRESOLVED** | Evidence gap, not a code gap. RADV *execution* is not in doubt (the analysis itself ran the v1.0.0 binary on the user's PC); what has never been measured: RADV perf numbers (PERF.md tables blank by design), the determinism RADV leg (S4b/E-b), a lavapipe-vs-Chrome-SwiftShader timing comparison. Instruments are ready (bench harness P3.5, byte-probe pack W34, runbook PC-EVIDENCE-W36, W68 dry-run of every VM-runnable leg). Missing evidence: the PC session's outputs — it can only run on the user's PC. |
| G19 No self-test for the native path | **STAGED** | W48 delivered the frame-hash comparison tool into this repo (`tools/ci/frame_hash_compare.py` + fixtures — verified present) and the native selftest leg spec (SELFTEST-NATIVE-W48.md). The leg itself targets the shipped skill's `selftest.sh` via the skill-wiring work copy, which this wave deliberately does not touch; until the leg is adopted there, the native path has gate evidence (telemetry, validators, CI) but not selftest evidence. = R10. |

**§6 subtotal: 28 counted rows (G10b, G17d cross-ref, uncounted) — LANDED 11 · STALE-CLAIM 6 · REJECTED 5 · STAGED 3 · DECISION-OWED 1 · UNRESOLVED 2.**

---

## 7. Requirements R0–R10 (W49 §C, re-verified)

| Row | Status | Evidence |
|---|---|---|
| R0a Source and build — source in version control | **LANDED** | Private repo, full history, 100+ wave commits. (= G17a's disproof.) |
| R0b Source and build — one canonical binary, behaviour-changing version (v1.0.1) | **DECISION-OWED (user)** | Release pack refreshed: RELEASE-101B-W89 (changelog `f6616aa`→`c440d90`, bump mechanics re-verified, six gates to re-prove on the bump SHA). Verified at HEAD: `Cargo.toml` still `version = "1.0.0"`; no v1.0.1 exists. W89's go/no-go paragraph reads: no-go yet — nothing code-side blocks it, but the release waits on the user's go, the wave freeze, the bump itself, and the gates on the bump SHA. W49's standing prohibitions carry: do NOT re-attach the v1.0.0 proof MP4 (it predates the re-chunk); canonical PC path `~/talerender-bin/talerender` via `TALERENDER_BIN` closes the two-binaries complaint. |
| R0c Source and build — reproducible builds (build twice → identical sha256) | — (cross-ref) | = D5 (§8), UNRESOLVED there; not recounted. |
| R1a Generic film input — `film.json` entry | — (cross-ref) | = F1/G1a, LANDED there. |
| R1b Generic film input — a second project can run ("no `onoda*` present") | **STAGED** | The route is the exporter (EXP) feeding driver-consumable data; the driver-side decision is G1c. Neither has landed, so the acceptance remains unreachable today — stated plainly, as W49 stated it. |
| R2a Timeline from the skill's data — reference film | **LANDED** | Onoda's timeline is computed from `script.json`/`lines.json`/`words.json` by `gen_film_data.py` — no hand-typed timings (G4a). |
| R2b Timeline — auto-chunked projects | — (cross-ref) | = G4b, UNRESOLVED there. |
| R2c Timeline — telemetry caption table comparable to `capture check` | **LANDED** | The missing half at W49 (no full caption table) landed with G7's `--tables` (`3223509`): the binary now prints the full caption table from its own audit (parity with the W75 prototype was exact, 73/73, at prototype stage; the binary table is a re-print of the audit's own data per TABLES-IMPL-W85 §1). The headless-chicken instance of this acceptance additionally needs R2b. |
| R3a Captions rendered — native rasterizer mechanism | **REJECTED** | = G3 (strips contract). |
| R3b Captions rendered — outcome bars, reference film | **LANDED** | Strip pipeline: chunks measured from the strips' own ink; gate `CHECK OK` at the 73-chunk layout; pop 0.86→1 over 0.1 s in `eval_frame`; fit floor enforced by the audit (split, never shrink). |
| R3c Captions rendered — the literal accept test (frame PSNR ≥ 40 dB vs Chrome at a chunk start) | **UNRESOLVED** | Never run as stated — the film A/B (mean 23.02 dB) is whole-frame, not caption-isolated. A repo-wide search finds the test mentioned only in W49 itself: no seat ever picked it up. Missing evidence: the test run. It is unqueued; commissioning it is the user's call if they want the letter of R3. |
| R4a Characters — reference film | **LANDED** | Native painter + per-frame state proven on Onoda: the telemetry gate's own checks are the evidence (overflow none, continuity clean, `CHECK OK`). |
| R4b Characters — arbitrary two-character scene (±2 px silhouette accept test) | — (cross-ref) | Awaits exporter-fed specs (EXP) + the anim/charanim adoption (P4.1); not recounted. |
| R5a Sets — reference film end-to-end | **LANDED** | `the-soldier-who-didnt-know` renders natively end-to-end (1,811 frames; A/B 23.02 dB mean vs Chrome — the calibration number, headless-gl band 15.8–19.3 dB). |
| R5b Sets — headless-chicken (analysis's ≥ 38 dB sequence bar) | **UNRESOLVED** | Blocked on the same pair as R1b/R2b: the exporter (EXP, STAGED) and the auto-chunk transcription (G4b, UNRESOLVED). Missing evidence: a native chicken render and its sequence PSNR. No chicken frame has ever been rendered natively. |
| R6 QA gate parity | **LANDED** | Exit-code parity (`aa10b13`, X1); overflow/edge/continuity/composition/caption-audit parity (telemetry.rs is a deliberate transcription of capture.ts's printers); the W49 remainder — shot table, full caption table, cast list — landed at `3223509` (G7). Deliberate-break acceptance holds for every class the gate prints. The boot-error class is Chrome-specific; its native analogue (load failure → exit 3, no verdict line) exists. |
| R7a Review loop — draft knobs | — (cross-ref) | = F4, LANDED there. |
| R7b Review loop — sheet | — (cross-ref) | = F5/G15b, LANDED there. |
| R7c Review loop — preview | — (cross-ref) | = G15c, REJECTED there. |
| R8a Audio + deliverable — the deliverable | **LANDED** | = G16 (`film/finish.sh` + the proof MP4's ffprobe shape). |
| R8b Audio + deliverable — cues | — (cross-ref) | = F6, STAGED there (design only, per the order). |
| R9a Ops — meaningful errors | — (cross-ref) | = finding D / P2.3, LANDED there. |
| R9b Ops — unknown fields as errors | — (cross-ref) | = F3d, DECISION-OWED there. |
| R9c Ops — `--help` matches reality | — (cross-ref) | = D4, LANDED there. |
| R9d Ops — canonical binary | — (cross-ref) | = R0b, DECISION-OWED there. |
| R10 Selftest | — (cross-ref) | = G19, STAGED there. |

**§7 subtotal: 13 counted rows (13 cross-ref sub-rows uncounted) — LANDED 8 · DECISION-OWED 1 · STAGED 1 · REJECTED 1 · UNRESOLVED 2.**

---

## 8. W49's own extra finds — D1–D8 (re-verified)

| Row | Status | Evidence |
|---|---|---|
| D1 G14 frame fade already implemented (the triage missed it) | — (cross-ref) | = G14, STALE-CLAIM there (fade verified in source at HEAD). |
| D2 tools-ci RED at W49's HEAD (manifest absolute paths) | **LANDED** | Fixed in the fixes pass — the full incident chain is W-04/W-05/W-06 (§9). Tools CI conclusions checked first-hand at HEAD-era commits: SUCCESS at `d967931`, `2fcffc0`, `3c07564`, `3223509` (Tools CI `43629625142`), and at snapshot HEAD `2f5c708` (Tools CI `43630186614` SUCCESS). |
| D3 AAC bitrate deviation (`finish.sh` 192k vs the skill master's 256k) | **LANDED** | `d967931` (W76 micro-fix landing). Verified in source: `film/finish.sh:31` now reads `-c:a aac -b:a 256k`. **Disclosure carried from the fix commit itself:** the delivered f021991 proof MP4 predates the fix and still carries 192k — the *file* is aligned; the *artifact* is not re-encoded by this row. |
| D4 `--help` over-promises scene resolution (+ exit-code table gaps) | **LANDED** | `d967931`, all three W76 items. Verified in source: the HELP `--film` entry now states the segment shots are staged by the driver in code and `scenes/film/` is the tools' manifest copy, never read by `--film` (main.rs:68); the EXIT CODES table (main.rs:136–145) includes exit 4 (Image I/O) and the widened exit 2/3 wordings matching the errors.rs mapping. |
| D5 No `Cargo.lock` → R0's reproducibility accept test unmet | **UNRESOLVED** | Still true at HEAD, verified: no `Cargo.lock` exists in the repo (CI generates one per run). The fact is surfaced in the release pack (RELEASE-101B-W89.md:191 — "There is still no tracked `Cargo.lock` in this repo"), so the v1.0.1 decision can be taken with it in view — but the accept test itself (two source builds → identical sha256) has never been demonstrated. Missing evidence: a committed lockfile (zero-deps law makes it small) or a two-build identity proof. Owner: lead. |
| D6 Auto-chunk transcription gap | — (cross-ref) | = G4b, UNRESOLVED there. |
| D7 Unregistered/unconsumed animation modules (honesty guard) | **STAGED** | Re-checked at HEAD: `locomotion` and `cameramoves` are **still absent** from main.rs's mod list (the observation stands; their packs are P4.3a/P4.4). The other half of W49's warning has been overtaken: `charanim` now has production call sites (P4.2), so the tree no longer over-claims there; `anim` remains registered-but-uncalled (P4.1). The guard's purpose — the close-out report must not claim an "animation system landed" — is discharged by this row's wording wherever it is cited. |
| D8 Telemetry table outputs absent | — (cross-ref) | = G7, LANDED there (`3223509`). |

**§8 subtotal: 5 counted rows (D1, D6, D8 cross-ref, uncounted) — LANDED 3 · UNRESOLVED 1 · STAGED 1.**

---

## 9. Rows the wave itself created

Wave 2 generated its own defects, corrections and incidents. Each
is traced here with its disposition; none is hidden inside the
rows above.

| Row | Status | Evidence |
|---|---|---|
| W-01 **The knobs record error** — a phase-1 landing ("--res works, 540×960 renders, [DRAFT] tag") was recorded and relayed to the user during Batches 7/8; it had never happened | **LANDED** (corrected + closed) | Disproof was airtight (W73 at `b2e46d2`: no Knobs struct / `effective_size` / DRAFT string anywhere in src; all four flags rejected as unknown arguments; the claimed dispatch was impossible — render.yml has no size input; the 540×960 / 22.35 dB figure's provenance is void). The lead corrected the record itself (`4af54c5`, register item P6); the user was told plainly, with the true state, the same night. The implementation then landed for real (`ea98458`/`9ad2177`/`76b922c`) and was independently accepted (F4/W81). This row is why §0's method treats the record as a claim. |
| W-02 **Act-1 pins defect** — 11 of the 14 film first-pins initially described superseded pre-atmosphere-fix (381ff47-era) bytes; a staging copy step was missed | **LANDED** (corrected + verified) | Exposed by the post-pin VR canary (dispatch `43624398371`: film-s08b DRIFT 24.85 dB; the rendered bytes equalled the `f021991` candidate, i.e. current output — the renderer was innocent). Corrective `2755ff3` (exactly 10 pin lines; s10 identical-bytes control unmoved), independently verified exactly by W63. Post-corrective VR steady state: s28 sole DRIFT (P1.2s). |
| W-03 **Act-2 provenance cite slip** — the Act-2 commit message and record row cited render run head `9f497c7`; the run's actual head was `dc0e555` | **LANDED** (corrected in place) | Found by W63 §3; the record's §7 row was corrected in place at fold-in 2 (`cca4e6e`) with an annotation; the commit message is immutable and is flagged as carrying the slip wherever it is cited. The pin itself (`60c32c1a…`, 23.98 dB) was never in question. |
| W-04 **Tools-CI incident 1** — red at `630d82e`/`f6616aa`: the new asset-check step existence-failed the film manifest's absolute `/home/hatch/...` paths on runners (passed locally, where those paths exist) | **LANDED** | Lead fix `617ea4e` (absolute refs classified report-only by check 4, skipped by check 2; relative missing refs still fail; fixture `scenes/fixtures/assets/abs-path` proves the rule). |
| W-05 **Tools-CI incident 2** — fixture PNGs silently swallowed by `.gitignore` `*.png` since Batch 3; every local reproduction passed because the files exist on this disk | **LANDED** | Orchestrator fix `865c471` (negations for `scenes/fixtures/**` + `tools/**/fixtures/**`; 55 fixture PNGs committed). Standing lesson from this incident: clean-tree (`git archive`) verification is now the standard for wiring packs (applied by W55 onward). |
| W-06 **Tools-CI incident 3** — red at `2c41313` (run `43625940101`): the new sheet step failed every case rc 2 — the hosted image does not guarantee ffmpeg, which W55's pack assumed | **LANDED** | Lead fix `f4f6ccf` (the workflow installs ffmpeg before the suite; the suite's own ffmpeg-absent case manipulates PATH internally and is unaffected). CI + Tools CI both SUCCESS at `f4f6ccf` (`43626299883` / `43626299876`). Third environment-assumption root cause in the same family as W-04/W-05. |
| W-07 **Main CI red at `edb058f`** — errors step 2 missed two `Ok(...)` wraps at film.rs's construction tails (delimiter class) | **LANDED** | Lead fix `aa92cee`; CI + Tools CI both SUCCESS there (`43623461613`/`43623461624`). |
| W-08 **ab1 delimiter break** — knobs phase 2's attachment replacement in ab1 left the pre-knob closing `})],` dangling (rustc: unexpected closing delimiter, ab1.rs:1438); all four phase-2 probe dispatches failed at Build | **LANDED** | Fix `c440d90` (one line removed, no logic change; the film/scenefile/dof replacements had consumed their closers — ab1's old-text cut one line short). The knobs-gate acceptance runs (F4) ran at this fixed head. |
| W-09 **gpucore test failure at module registration** — `2328e6f`'s first CI run: 174 passed / 1 failed (`strip_padding_drops_sentinels` — the seat test's payload pattern aliased its own 0xEE sentinel; a test bug, not a module defect) | **LANDED** | Fix `38930fa`; suite 175/175 at the PaintBudget+RunLog wiring head `86f0c81`. |
| W-10 **KNOBS-W43.md truncation** — the committed F4 spec was transport-truncated at line 264 | **LANDED** (with disclosed loss) | Tail restored provenance-tagged (W61) and spliced by the lead at `33c5b2a` (T2 included, tag intact; five content spot-checks re-run at the splice head). **Six items remain UNRECOVERABLE** and are registered as such in KNOBS-W43-TAIL-W61 §3 — the restoration is substance-level and claims nothing more. |
| W-11 **FOLDIN-W65.md §4 truncation** — the §15 fold-in checker's own note was transport-truncated (same failure class as W-10) | **LANDED** | Addendum FOLDIN-ADDENDUM-W71 restored §4; lead splice `108f33f`; post-splice census re-verified 75/75 at HEAD by W71 and again by gate W90. Standing rule adopted from this pair of incidents: long seat notes are written in append chunks. |
| W-12 **Adapter re-proof retraction** — the lead's "FAILED direction proven" row rested on a run that had in fact been SIGKILLed mid-telemetry (OOM under VM contention; the adapter's rc 1 came from its no-verdict-line path), compounded by a fixture pairing 66-chunk data with 73-era strips | **LANDED** (correction closed by independent proof) | The correction is recorded in record §7 in place. The fixture was rebuilt from `git archive aa10b13 film/` (matched 66-chunk data+assets); Batch 7's W62 then re-proved the adapter **independently, 8/8 cells PASS** across both exit-code generations, strip-mismatch refused pre-gate (gate W70 re-ran the grid). The lead's own re-run attempts kept dying to contention and are recorded as such, not papered over. |
| W-13 **W68 F1 — PC runbook path defect (BLOCKING for the PC session)** — PC-EVIDENCE-W36 §0.2 provisions the binary to `~/talerender-bin`, but §§0.4/2.1/3.1/3.2 invoke `./talerender-bin/talerender` from the repo root | **UNRESOLVED** | Verified present at HEAD first-hand: PC-EVIDENCE-W36.md:88 installs to `~/talerender-bin` (`-D ~/talerender-bin`), while :130 invokes `./talerender-bin/talerender --scene-file …` and :311/:313 use `./talerender-bin/talerender --film-telemetry …` — reproduced by W68's dry-run as `No such file or directory`, exit 127. Same defect in USER-PC.md §§3–5 (per W68). W68's note itself landed three batch-closes late (PC-DRYRUN-W68.md, in tree). Missing evidence: the one-line fix commit (either side — provisioning path or invocation paths) in the runbook + USER-PC.md; a canonical-path fix pack is assigned to Batch-10 seat W96, whose file had not landed at snapshot. Until it lands, the PC session (G18/S4b) would trip on its first leg. |
| W-14 **G7 tables beat-test CI red at `9380b01`** — the `--tables` landing (`3223509`) shipped one failing unit test: `telemetry::tests::tables_beat_frames_tie_to_blink_at` (assertion left 0, right 3, telemetry.rs:2062); 179 passed / 1 failed | **LANDED** | Diagnosed from the failed run's log (`43629817118`) and fixed at `2f5c708` ("Fix tables beat test range"). Re-verified first-hand: CI `43630186620` + Tools CI `43630186614` both **SUCCESS** at `2f5c708` — this matrix's snapshot head is that green head. (Batch-10 seat W93 owns the fuller diff-check of the tables landing against TABLES-IMPL-W85; this row covers the CI defect only.) |
| W-15 **W77 exporter-acceptance note never landed** — the harness tool was gated (self-test 3/3, W80), but the seat's note (real-binary / ENV-BLOCKED evidence) was still owed at Batches 8, 9 and at this snapshot | **UNRESOLVED** | Missing evidence: the note file itself (no `EXPORTER-ACCEPT-W77.md` exists in `docs/integration/` at HEAD — directory listing verified). Corroborating oddity: `tools/format/exporter_acceptance.py` shows as **modified in the working tree** at snapshot — the seat appears to have kept iterating without delivering. The harness's gated self-test stands; its real-binary claims remain ungated. |
| W-16 **Painter panic, observed once, not reproduced** — first telemetry run at the exit-code binary (half-migrated 66/73 tree) panicked at `src/painters/cutout_build.rs:44` ("skin paint panicked: fill contour start outside painted skin area"), exit 101 | **UNRESOLVED** | The post-regen run over the consistent 73/73 tree completed all 241 samples with `CHECK OK`; no root cause is claimed by anyone. Missing evidence: a reproduction (on matched or deliberately mismatched data+assets). Recorded as a watch item in record §7; if it recurs it is a painter-robustness finding. |
| W-17 **Split-programme battery precondition** — W82 established that the 46/46 stroke-battery leg has no standing invocation at HEAD (the battery is the binary mode `--scene stroke-battery`, absent from ci.yml and not a render.yml mode), so split steps S2/S4/S5 must not start until the lead establishes a dispatch reporting 46/46 on the pre-split tree | **STAGED** | At snapshot, `.github/workflows/battery.yml` exists in the working tree **untracked** — the precondition is being established by the lead/sibling seats during the Batch-10 window but is not committed at HEAD. P2.2's STAGED status already assumes this gate. |

**§9 subtotal: 17 rows — LANDED 13 · UNRESOLVED 3 · STAGED 1.**

---

## 10. Count table — the wave's completeness claim

Counted rows only, under §0's convention (lettered sub-rows count
once each; `— (cross-ref)` sub-rows are uncounted and listed in
the right column so the arithmetic can be re-performed from the
tables above):

| Section | Rows counted | LANDED | STAGED | DECISION-OWED | REJECTED | STALE-CLAIM | UNRESOLVED | Cross-ref (uncounted) |
|---|---|---|---|---|---|---|---|---|
| §1 Stale register S1–S8 | 9 | 7 | 0 | 0 | 0 | 1 | 1 | 0 |
| §2 Findings A–E | 7 | 5 | 0 | 1 | 0 | 0 | 1 | 0 |
| §3 Phases 1–5 | 30 | 17 | 10 | 3 | 0 | 0 | 0 | 0 |
| §4 §6 improvements | 4 | 4 | 0 | 0 | 0 | 0 | 0 | 0 |
| §5 Gap order F-rows | 12 | 6 | 3 | 2 | 1 | 0 | 0 | 0 |
| §6 Gaps G1–G19 | 28 | 11 | 3 | 1 | 5 | 6 | 2 | 2 (G10b, G17d) |
| §7 Requirements R0–R10 | 13 | 8 | 1 | 1 | 1 | 0 | 2 | 13 (R0c, R1a, R2b, R4b, R7a–c, R8b, R9a–d, R10) |
| §8 Extra finds D1–D8 | 5 | 3 | 1 | 0 | 0 | 0 | 1 | 3 (D1, D6, D8) |
| §9 Wave-created | 17 | 13 | 1 | 0 | 0 | 0 | 3 | 0 |
| **TOTAL** | **125** | **74** | **19** | **8** | **7** | **7** | **10** | 18 |

Column check: 74 + 19 + 8 + 7 + 7 + 10 = **125** = the counted-row
total. DECISION-OWED splits by decider: **user 6** (C-veto, P1.2s
s28, P4.3b locomotion, P4.5s lipsync sheet, F6q, R0b v1.0.1) and
**lead/format 2** (F3d hard-error, G1c data-driven driver).

### What the totals do and do not claim

- **74 LANDED** rows are each backed above by a commit plus a
  source/run verification performed for this re-run — not by the
  wave record's say-so. Where a landing's own verification is a
  sibling seat's in-flight work (the W93 diff-check of the tables
  landing), the row says so and claims only what was checked here.
- **19 STAGED** rows are real, named, gated packs — the wave's
  second half was explicitly "code first, then fixes", and the
  adoption landings (splits, §15 loader, locomotion/cameramoves
  wiring, atlas/bufferpool adoption, VRAM line, s12 adoption,
  exporter, lipsync binding, F3-b/c landing, selftest leg) were
  prepared to landing shape but not landed. They are the natural
  scope of a Wave 3, in the lead's queue, and none of them blocks
  a user decision except where marked.
- **10 UNRESOLVED** rows name their missing evidence exactly:
  the RADV session's outputs (S4b, E-b, G18 — one event discharges
  all three), the auto-chunk owner (G4b), the caption-isolated
  40 dB test run (R3c), a native chicken render (R5b), a committed
  `Cargo.lock` or two-build proof (D5), the runbook path fix
  (W-13), the W77 note (W-15), and a reproduction of the painter
  panic (W-16).
- **7 REJECTED** rows are decisions with standing reasons (strips
  contract ×2 angles, native shot model, blended-grade
  transitions, preview, binary `--sheet` duplication, native
  rasterizer) — each reversible only by changing the underlying
  contract, not by finding time.
- **7 STALE-CLAIM** rows are the audit/gap analyses' errors,
  corrected with disproofs — the wave's answer to them was
  verification, not work.
- One decision exists outside this matrix's row set and is
  recorded here so the wave-close brief is complete: **skill
  adoption** (adopt the native backend into the shipped skill as
  an opt-in alternative; work copy `~/workspace/talerender-skill-wiring/`
  ready) was explicitly *not* wave scope (record §4, Phase 5) and
  is DECISION-OWED (user) alongside the six in-matrix user
  decisions — seven user decisions in total at wave close.

*End of TRACE2-W97. Snapshot HEAD `2f5c708`; CI + Tools CI SUCCESS
at that head (runs `43630186620` / `43630186614`, checked
first-hand). Nothing in this file was committed by its author.*

---

**Postscript (same session, minutes after the snapshot):** HEAD
moved once more while this file was being written — `cc38b03`
("W77 + W68 notes land (late seats)"). Two rows are affected and
are updated here rather than silently edited above:

- **W-15's missing evidence has arrived:** `cc38b03` commits
  `docs/integration/EXPORTER-ACCEPT-W77.md` (263 lines) together
  with the FINAL version of `tools/format/exporter_acceptance.py`
  (the commit message states the Batch-8 file was a mid-flight
  draft; the note's transcripts are from the final code). W-15's
  UNRESOLVED therefore lapses on existence — but this seat has
  **not** verified the note's contents, and the EXP row's
  "self-test PASS 3/3" figure was the *draft's* gate result; the
  final harness's numbers are the note's own to state and the
  close-out gate's to check. Count-table effect if the gate
  accepts the note: UNRESOLVED 10 → 9, LANDED 74 → 75.
- **W-13 is unchanged in substance:** `cc38b03` also commits the
  W68 dry-run note (PC-DRYRUN-W68.md) itself, so that evidence is
  now in the tree rather than untracked — but the commit does not
  touch `docs/runbook/PC-EVIDENCE-W36.md`, and the path defect
  was spot-rechecked after the commit (provisioning still
  `~/talerender-bin` at :88). The runbook fix is still owed.
- CI conclusions at `cc38b03` were not checked by this seat; the
  snapshot's green claim covers `2f5c708` only.
