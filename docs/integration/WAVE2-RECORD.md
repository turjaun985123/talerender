# TaleRender WAVE 2 — Lead/Integrator Record

> **Mission (user order, 2026-11-14 22:26 +06):** assign 100 agents to
> complete **all phases** of the external audit roadmap — write all
> code first across all phases, then fixes. The audit (of
> `talerender-main.zip`, pasted by the user) is the scope document,
> but parts of it are **stale** — it was written against an archive
> that predates several landed fixes. §3 of this record is the
> stale-item register: what the audit asks for that is already done
> must NOT be redone.
>
> **Model:** as Wave 1. 10 batches of 10 seats. Wave seats write
> NEW files only (including new `src/` modules) and wave-owned
> docs/tools/scenes; they never commit. The orchestrator validates
> and commits wave work centrally between batches. The lead (this
> record's keeper) alone edits existing `src/*.rs`, `Cargo.toml`,
> `docs/scene-format.md`, `docs/integration/README.md` +
> `INVENTORY.md`, and commits its own integration work with
> explicit paths only (never `git add -A`).
>
> **Launch HEAD:** `d7bebdd` (professional CLI: `--help`/`--version`,
> version 1.0.0; CI green at launch).

---

## 1. Laws (carried from Wave 1, restated for Wave 2)

1. **No quality reduction, ever.** No resolution/FPS/CRF/bit-depth
   reduction as a fix for anything.
2. **f32 data law** (scene-format spec §11): transcribed staging
   carries exact f32 values; keys/values that feed rendering must
   not be quantised (the `cabcc30` lesson).
3. **Film output must not change** except where a seat's change is
   gated and measured — A/B before/after, numbers in this record.
4. **The telemetry verdict LINE stays verbatim and authoritative.**
   Exit codes are being ADDED alongside it (Phase 1); the line's
   text, position (stdout), and meaning do not change. The skill
   adapter parses the line and must keep working — coordinate with
   seat W02 before any exit-code behaviour lands.
5. **Zero new dependencies.** The hand-written JSON parser stays
   (the audit's serde suggestion is REJECTED — zero-deps + f32
   exactness laws stand).
6. **Builds only via GitHub Actions CI.** Every CI verdict checked
   is reported in §7 the moment it is checked. No cargo on this VM.
7. **RADV is impossible from this VM (no GPU).** The wave prepares
   everything for the user's PC (bench harness, determinism RADV
   leg, runbooks) and never claims a RADV number.
8. **Renderer labelling.** Every image/number names its renderer
   (CI lavapipe / VM llvmpipe / PC RADV). No performance claims
   from this VM.

## 2. Ownership

| Surface | Owner |
|---|---|
| Existing `src/*.rs`, `src/*.wgsl` edits | Lead only |
| New `src/` modules | Wave seats author; lead integrates (mod declaration, call sites) |
| `Cargo.toml` | Lead only |
| `docs/scene-format.md` | Lead only |
| `docs/integration/README.md`, `INVENTORY.md`, this record | Lead only |
| New docs / tools / scenes / fixtures / workflows | Wave seats (orchestrator commits) |
| Skill work copy (`~/workspace/talerender-skill-wiring/`) | Wave seats per assignment (W02 adapter) |

## 3. Stale-item register — what the audit asks for that is ALREADY DONE

Do not redo these. Each entry: audit claim → actual state → evidence.

| # | Audit claim | Actual state | Evidence |
|---|---|---|---|
| S1 | "Resolve MP4 BT.709 metadata" (Phase 1) | **DONE.** finish.sh passes all three BT.709 flags AND x264 VUI params; the f021991 MP4 probes `color_space/transfer/primaries` all bt709. | commit `6ca97fc`; MP4-EVIDENCE (f021991 delivery, sha256 `dea7bb2c…`) |
| S2 | "Re-render the complete film at the latest source revision" (Phase 1) | **DONE.** Full 1,811-frame re-render at `f021991`; A/B vs Chrome **mean 23.02 dB, min 19.36** (progression 21.44 → 21.84 → 22.43 → 23.02 across the fix chain); S11 recovered 18.52 → 21.91. | Lead completion report; `~/workspace/film-final/ab-out4/` |
| S3 | Determinism "root cause is not conclusively established" (§A, §E) | **DONE — root-caused and fixed.** `state_key` quantised continuous channels to 2 dp → cache-key collisions (NOT eviction; the eviction theory was falsified — the pattern repeated digit-for-digit across independent fleets). Keys are now exact f32 bit patterns. | commit `cabcc30` (+ E0600 fix `f021991`) |
| S4 | "Formal determinism PASS outstanding" (§E) | **PROVEN on CI lavapipe** at the final head: probes 1195–1204 **10/10** and 795–814 **20/20** byte-identical across independent runs. Still genuinely open: the pack's formal PASS per renderer incl. the **RADV leg** (impossible from this VM — prepare, don't claim) and the written determinism contract (§4, Phase 1/5). | Lead completion report |
| S5 | "Seven caption failures remain unresolved… has not yet been applied" (§C) | **Partly stale — see §5, the caption tension.** A ruling WAS made (lead: ACCEPT WITH WAIVER, recorded `9790553`). The user's Wave-2 order reopens it; the wave implements the re-chunk. | `docs/ab/film/CAPTION-DECISION-W92.md` ruling addendum |
| S6 | "Error handling: no --help, panics on bad input" (§D, partial) | **Partly done.** The professional CLI landed at `d7bebdd`: `-h/--help` full reference, `-V/--version`, CLI errors print `error: …` + help hint, exit 2, no panics on bad *input*. Remaining: runtime-path panics (`expect()` in adapter/device/readback/load paths) and the exit-code scheme (§6). | commit `d7bebdd`, CI run 43614468709 |
| S7 | "112 Rust test functions… cannot claim they passed" (§7) | Their environment had no cargo; ours verifies via CI: the cargo suite runs green on every push (71/71 at Wave-1 close). Wave 2 re-verifies each batch. | ci.yml runs |
| S8 | Risk register "still describes early milestones as unmitigated" (§7) | **True and in scope** — documentation refresh, Phase 5 (§4). Not stale; listed here so it isn't double-counted as an audit error. | docs/risk-register.md |

## 4. Open scope by phase (the real work list)

### Phase 1 — Production correctness
- **Caption re-chunk applied** (W92 Option A, computed splits): project
  `caps` edit → regenerate scenes-side strips + manifest (66 → 73,
  renumbering from each insertion) → regenerate driver-side
  `film/onoda.json` + `film/assets/cap_*.png` to the same 73-chunk
  layout → validator re-pass → telemetry gate re-run (target:
  `FILM TELEMETRY: CHECK OK`) → bridge repackage (73 strips) →
  A/B footnote for the ≈10/60 pairs whose caption content changes.
  See §5 for the waiver tension.
- **Telemetry exit codes** per the §6 contract (film-telemetry:
  0 = CHECK OK, 1 = CHECK FAILED, input/run error keeps a
  distinct non-zero). Verdict line unchanged. Adapter coordination:
  W02 (work copy) must keep passing both directions.
- **Golden re-stage from f021991** frames; pins struck as separate
  deliberate commits per decision (14 film first-pins ≠ ab1
  re-pin — one `--pin` run must never strike both, W87 warning).
  s28: packet recommends against re-pinning (kept as-is).
- **Determinism contract** document (same-renderer bit-identity
  scope, cross-vendor tolerance rules — no cross-vendor bit-identity
  promise) + the pack's CI lavapipe leg discharged via CI job.
- **PERF §9 asks**: progress lines carry elapsed seconds
  (§9.1); paint-cache stats line (§9.2, with Phase 3); peak-RSS
  self-report (§9.3); telemetry exit codes (§9.4 = above).

### Phase 2 — Rendering architecture
- **Shared GPU core**: one module owning instance/adapter/device
  creation + readback, consumed by main.rs, ab1.rs, dof.rs,
  scenefile.rs, film.rs (audit §B duplication table). Pixel-neutral
  by construction; A/B spot-proof after wiring.
- **Modularise film.rs / scene.rs**: split along the seams the
  shared core exposes; no behaviour change; cargo tests + A/B
  spot-proof gate each extraction.
- **Structured errors**: new errors module (seat-authored);
  runtime paths return structured errors with context instead of
  `expect()`/`panic!()` (adapter, device, buffer map, PNG save,
  film/scene load). CLI errors already conform (exit 2).
- **Structured configuration**: consolidate env knobs
  (`TALERENDER_PAINT_CACHE`, new byte budget, frame timing) into
  one documented surface, `--help` kept in step.

### Phase 3 — Performance engineering
- **Paint-cache byte budget**: account bytes, not just entries —
  budget-based eviction alongside (or replacing) the 160-entry
  count cap; **eviction must stay pixel-neutral** (paints are pure;
  the exact-key fix is what makes re-paint deterministic). Stats:
  hits / misses / evictions / bytes / peak bytes, printed end of
  run in film + telemetry modes (PERF §9.2).
- **Progress-line elapsed** (PERF §9.1) + optional per-frame ms
  behind `TALERENDER_FRAME_TIMING=1`.
- **Peak-RSS self-report** on exit, stderr (PERF §9.3).
- Buffer pooling / pipeline cache where the shared core makes it
  safe; texture atlas / dirty-region updates **only if proven
  pixel-identical** (A/B gated), else design note + defer.
- **Bench harness for the PC**: PERF protocol automation prepared;
  zero numbers claimed from this VM.

### Phase 4 — Animation & visual quality (all opt-in)
- **Animation evaluator + channels**: a pure `evaluate(t)` surface
  for character state (pose/expression/gaze/blink channels),
  data-driven; the film path uses it only behind a gate with
  before/after A/B — default film output unchanged.
- Expression/gaze/blink as scene-format channels (additive).
- Camera tracks as a versioned format extension.
- NO new effects for their own sake; each addition carries a
  quality + performance note (audit §4.2 principle, adopted).

### Phase 5 — Engine maturity
- **Scene-format versioned extensions**: prefabs, animation clips,
  camera tracks — additive extensions to the existing format and
  the existing hand-written parser (no serde swap, §3/S5 laws).
- **CI jobs**: visual-regression (golden verify on push) +
  determinism (pack CI leg) wired into workflows.
- **Docs**: risk-register refresh (S8), architecture doc matching
  the post-refactor tree, this record's close-out.
- Skill adoption remains the user's decision (not wave scope);
  the adapter must merely keep working (§6/W02).

## 5. The caption tension (recorded, per mission)

Wave 1 closed with the lead's ruling **ACCEPT WITH WAIVER**: the
seven chunks exceed the 850 px bar in Chrome's original too
(worst chunk 953 px ink in Chrome's own frame), they are the
project's auto-chunks reproduced faithfully, and re-chunking
desyncs the native film from its fidelity target
(`9790553`, CAPTION-DECISION-W92.md ruling addendum).

The user's Wave-2 order — "complete ALL phases", audit Phase 1
first item "resolve seven caption failures" — **reopens the
question and outranks the waiver**. Wave 2 therefore implements
W92's Option A re-chunk (66 → 73 chunks, splits already computed
and priced in the decision pack) and re-runs the telemetry gate.
The waiver stays in the record as the prior disposition; if the
re-chunk lands and the gate goes green, the film's caption data
changes and the ≈10/60 A/B pairs inside the seven windows change
caption content — that cost is known and accepted by the order.
**The user can veto at wave close** and revert to the waiver
state (the 66-chunk data remains in git history).

> **CLOSED BY DATA (lead, 2026-11-14):** the re-chunk's driver
> leg landed as `5e90d7e` (film/onoda.json + film/assets
> regenerated to 73 chunks; bridge repackaged, packager's own
> verification 21/21). The telemetry gate re-run at the
> regenerated data prints **`FILM TELEMETRY: CHECK OK`** with
> exit 0 — the seven failures are resolved at the data level,
> the 850 px threshold untouched. The waiver above remains the
> documented fallback if the user vetoes at wave close.

## 6. Exit-code contract (lead design, Phase 1)

Audit §D's table, adopted with one clarification (film-telemetry
run errors):

| Code | Meaning | Where |
|---|---|---|
| 0 | Success / all checks passed | all modes |
| 1 | QA validation failed | `--film-telemetry` on CHECK FAILED; `--telemetry` already returns 1 on gate fail |
| 2 | Invalid arguments or input | CLI parse errors (already), unreadable/invalid film or scene input to a gate |
| 3 | GPU initialisation / rendering failure | render modes when adapter/device creation fails (replaces the `expect()` panics as the errors module lands) |
| 4 | Output / filesystem failure | PNG/frame write failures |

Binding constraints:
- The verdict **line is printed verbatim to stdout in all cases**
  and remains the authoritative signal; exit codes are additive.
- The skill adapter (work copy, W02) gates on the line today and
  treats non-zero how? — **RESOLVED (W02 + W62):** the
  work-copy adapter implements the dual convention (accepts
  FAILED at exit 0 or 1, OK only at 0, fails closed on any
  disagreement or missing line; stub-proven 10/10 by W02; the
  lead's real-binary re-proof claim was retracted (§7
  CORRECTED row) and the convention was then re-proven
  independently by W62, 8/8 cells across both exit-code
  generations (§7, B7/W62 row)).
- `main.rs` dispatch today: `run_film_telemetry` returns
  `Result<(), String>`; the lead will change it to surface the
  verdict (`Ok(passed: bool)`) so main can exit 0/1, with `Err`
  mapping to 2. The line itself is printed inside film.rs and
  does not move.

## 7. Verdicts log (CI + gates, as checked)

| When | What | Verdict |
|---|---|---|
| Launch | HEAD `d7bebdd` CI (run 43614468709, from Wave-1 record) | SUCCESS |
| B1 push | HEAD `3a36f1f` CI (43617243966) + Tools CI (43617243900) | SUCCESS / SUCCESS |
| E6 | **Determinism (dispatch) run 43617370962** at `3a36f1f` — the pack's CI-lavapipe leg (W04 workflow, W96 route 1) | **SUCCESS — DETERMINISM pack PASS on CI lavapipe.** RADV leg still owed on the user's PC (prepared, not claimed) |
| Lead | Exit codes `aa10b13` CI (43617609513) | SUCCESS |
| Lead | **Caption driver leg `5e90d7e`: telemetry re-run (exit-code binary, local CPU) — `FILM TELEMETRY: CHECK OK`, exit 0** (0 overflow / 0 continuity / **0 caption failures**; 241 samples; max on-screen caption width 928 px). Watch item closed: the tightest unsplit chunk ("Onoda still refused", driver ink 844 pre-regen per GATE2-W10 §2.3) passes the audit's floor in the regenerated set — zero chunks over the bar | **PHASE-1 HEADLINE GATE: GREEN** |
| Lead | Errors step 1 + `mod errors` `ba88596`: CI (43618555381) + Tools CI (43618555345) | SUCCESS / SUCCESS |
| B2 push | HEAD `1a62161` CI (43618849875) + Tools CI (43618849888) | SUCCESS / SUCCESS |
| Lead | Adapter re-proof, real binary (`ba88596` build) vs 66-chunk fixture — **CORRECTED 2026-11-14 (late)**: the run behind this row's original text was in fact **SIGKILLed mid-telemetry** (OOM under VM contention; log ends at sample ~160); the adapter's rc 1 came from its no-verdict-line path ("binary killed by signal SIGKILL — treated as a FAILED gate"), NOT from an exit-1 verdict. A second defect compounded it: the fixture paired 66-chunk data with the 73-era strip set, and on that mixed fixture the OLD (v1.0.0) binary prints `CHECK OK` (max rendered width 844 px) — the genuine Wave-1 failure state needs matched data+assets. Fixture rebuilt from `git archive aa10b13 film/` (66 captions, 78 asset files); the corrected FAILED-direction proofs (new binary, then old binary) are being re-run sequentially and will be appended as their own rows. The OK direction stands as proven (row below) | **CORRECTION — exit-1 direction NOT proven by that run; re-proof in flight** |
| Lead | Adapter re-proof, OK direction: real binary (`ba88596` build) on the 73-chunk fixture — binary prints `FILM TELEMETRY: CHECK OK`, adapter relays `CHECK OK`, **rc 0** | PASS |
| Lead | PaintBudget byte-probes (render dispatches 43620063494 / 43620069128 defaults; 43620074415 / 43620079310 at `TALERENDER_PAINT_BUDGET_MB=100`): budget-forced eviction vs defaults — 795–814 **20/20** + 1195–1204 **10/10 byte-identical**; defaults vs the `f021991` set also 30/30 byte-identical | PASS (PERF §9 wiring discharged) |
| Lead | Determinism re-dispatch at the wiring head (run 43620084786) | SUCCESS |
| B3 push | HEAD `ff52b3a` Tools CI (43620525788) SUCCESS; its CI run was cancelled-by-concurrency by the lead's next push (superseded by green `df89ad6` — a cancellation, not a failure) | SUCCESS (gate PASS-WITH-NOTES) |
| Lead | **gpucore migration site 1/5 dof** `df89ad6` CI (43620553455) SUCCESS; dof-chart dispatch 43620556727: chart sha256 `957d5573…` **byte-identical** to the M3 reference + golden pin | PASS, pin discharged |
| Lead | **gpucore site 2/5 scenefile** (CI 43620746325 cancelled-by-concurrency, covered by the ab1 run); fx-textures dispatch 43620768148: `scene.png` sha256 `6ef2e39b…` **byte-identical** to the Wave-1 pinned hash | PASS, pin discharged |
| Lead | **gpucore site 3/5 ab1** `9f497c7` CI (43620828640) SUCCESS; fresh ab1 dispatch 43621168728 (below) | SUCCESS |
| Lead | **gpucore site 4/5 main** `dc0e555` CI SUCCESS (43621168728 list) | SUCCESS |
| Lead | **gpucore site 5/5 film + F1** `aadc193` CI SUCCESS — migration COMPLETE (5/5); post-migration film probes (runs 43621922784 / 43621929764): 795–814 **20/20** + 1195–1204 **10/10 byte-identical** vs post-wiring defaults (= `f021991` bytes). main.rs's duplicate `request_adapter` deleted — audit finding B CLOSED | PASS, pin discharged |
| Lead | ab1 fresh render at `dc0e555` (run 43621168728): sha256 `60c32c1a…`; vs Chrome **23.98 dB**, vs the `c00897d` candidate `a415ead8…` **53.50 dB**, candidate vs Chrome 23.96 dB (sampler validated against the recorded pair) — the basis for Golden Act 2 (below). **[Cite corrected per W63 §3: this row originally read `9f497c7`; the run's head was `dc0e555` (site 4/5 had already landed). The Act-2 commit message carries the same slip and is immutable; this row is the correction of record.]** | MEASURED |
| Lead | Tools-ci wiring (W39) `630d82e`: validators verified locally pre-wiring (lights 6/6, assets 5/5+6/6, shaders 21/21) | SUCCESS at wiring head |
| B4 push | HEAD `f6616aa` CI (43622186461) | SUCCESS (gate PASS-WITH-NOTES) |
| Incident | **Tools CI RED at `630d82e`/`f6616aa`** (runs 43622033396, 43622186445): the new "Asset references" step existence-failed the film manifest's 3 absolute machine paths (`project`, `audio.projectPath`, derived `audio.file`) on runners — W28 F2 (manifest is machine-bound) colliding with a check-2/check-4 classification inconsistency. Lead fix `617ea4e`: absolute-rooted refs are classified by check 4 (report-only; `audio.file` now flagged when its `project` base is absolute) and skipped by check 2; relative missing refs still fail (missing fixture rc 1 re-verified); new fixture `scenes/fixtures/assets/abs-path` proves the rule (PASS 5/5, exactly 3 check-4 warnings); manifest portability count 17→18 disclosed (audio.file newly classified). Main CI on `617ea4e` SUCCESS; manifest/driver step green in CI from run 43622601972 | FIXED (lead) |
| Incident 2 | Same run 43622601972 then failed one step later: fixture PNGs (`valid-scene/tex.png` etc.) missing on the runner — **`.gitignore` `*.png`** (negations only for `docs/**`, `film/assets/*`) had silently swallowed W28's fixture PNGs since Batch 3; every local reproduction passed because the files exist on this disk. Orchestrator fix `865c471`: negations for `scenes/fixtures/**` + `tools/**/fixtures/**`, 55 fixture PNGs committed. **Tools CI on `865c471` SUCCESS** — fixtures + WGSL steps green in CI for the first time | FIXED (orchestrator) |
| B5 push | HEAD `d3def63` (gap-fix batch; gate PASS-WITH-NOTES, GATE2-W50) | CI green via `aa92cee` (below) |
| Lead | **Errors step 2** `edb058f` — W35's pack as collapsed by the migration: only gpucore's 3 panic sites remained (`request_adapter`, `request_device`, `Readback::collect_padded`); all converted to `TaleResult` (GpuInit/DeviceRequest/BufferMap → exit 3), message in PERF's table preserved verbatim as the GpuInit source; callers converted across dof/scenefile/ab1/main/film (`Renderer::new`/`render` TaleResult ripples). Tools CI at `edb058f` SUCCESS; main CI FAILED at Build — two missing `Ok(...)` wraps at film.rs's construction tails (the delimiter class the orchestrator diagnosed from the log) | RED → fixed next row |
| Lead | Ok-wrap fix `aa92cee` (`Ok(Renderer {…})`, `Ok(pixels)`) | **CI + Tools CI both SUCCESS** |
| Lead | **GOLDEN ACT 1** (separate act): 14 film first-pins struck — 11 mids from the `f021991` candidates (planner-proven pixel-identical outside the split windows) + 3 in-window mids from the post-re-chunk render at `aadc193` (s07 f969 `7f8e1555`, s08 f1119 `3e8e703f`, s11 f1593 `7bed0da5`; runs 43621883744/43621890834/43621914911). Verify: 18 OK / 0 STALE / 0 MISSING; pin diff = exactly the 14 film lines, no milestone hash moved. (Composition as intended; as committed, the 11 out-of-window mids carried the staged `381ff47`-era bytes — see the CORRECTIVE row below.) | **STRUCK** |
| Lead | **GOLDEN ACT 2** (separate act): ab1-composer re-pinned `d7bd71a0…` → `60c32c1a…` — the FRESH head frame (run 43621168728), not the `c00897d` candidate, for the reason recorded in the repin packet's updated State line (candidate predates the exact-key fix chain; a pin describes current production output; A/B vs Chrome unchanged at 23.98 dB). Scope guard: pin diff = exactly the ab1 line; film frames moved aside during the act, restored; pack verifies 18/18 OK | **STRUCK** |
| Lead | Module registration `2328e6f` CI (43619241590): 174 passed / **1 failed** — `gpucore::tests::strip_padding_drops_sentinels` (first-ever execution of W07's CI-pending tests; the test's payload pattern aliased its own 0xEE sentinel at x=238 — a seat test bug, not a module defect) | FAILURE → fixed `38930fa` |
| Lead | PaintBudget + RunLog wiring `86f0c81` CI (43619649089) — suite 175/175 incl. the fixed gpucore test | SUCCESS |
| Lead | Errors step-2 gates at the post-step-2 head: dof-chart dispatch 43624002645 sha `957d5573…` byte-identical; fx-textures dispatch 43624248449 (correct fixture scene) sha `6ef2e39b…` byte-identical; film probe 43624011865 795–814 20/20 byte-identical; determinism dispatch 43624016907 SUCCESS. **No-adapter proof (W35 P6):** the `aa92cee` CI binary with `VK_ICD_FILENAMES` pointed nowhere → stderr `error: GPU initialisation failed (request_adapter (gpucore)): no wgpu adapter found — a Vulkan driver (hardware or lavapipe) is required`, **exit 3**, no panic | ALL PASS |
| Lead | **GOLDEN ACT 1 — CORRECTIVE** `2755ff3`: the first act pinned the staged 381ff47-era bytes for the 11 out-of-window mids (W03's candidate-copy step was missed). The VR canary caught it on its first measured run (dispatch 43624398371: film-s08b DRIFT 24.85 dB; rendered `fb4e2ba4…` = the f021991 candidate = current output). Corrective copies the 11 candidates (s10 identical — control case, line unmoved) and re-pins; diff = exactly 10 film lines; pack 18 OK / 0 STALE / 0 MISSING. W63 (Batch 7) independently verified the corrective exactly | **CORRECTED + VERIFIED** |
| Lead | Visual regression, post-correction dispatch 43625139818: dof-chart MATCH, ab1-composer MATCH, **film-s08b MATCH** (byte-identical to the corrected pin); the ONLY drift is **s28 (21.65 dB, by design** — its reference is the M2-era artifact; disposition is a user decision, W63 §7). The run's FAIL verdict is solely the designed s28 red | STEADY STATE |
| Lead | First bench dispatch 43623743480 at `aa92cee` | SUCCESS (harness verified; W64 analysis: CI lavapipe 1.85 s/f in-window, cache-24 saves ~147 MiB RSS at no time cost; anomalies A1 bench window is S01-only, A2 accounted-vs-resident cache bytes diverge ~9× — both recorded, neither blocks) |
| B7/W62 | **Adapter re-proof, INDEPENDENT (W62, gate re-ran the grid): 8/8 cells PASS** across both telemetry exit-code generations, strip-mismatch refused pre-gate. This closes the CORRECTION row above with an outside verdict: the exit-1 (CHECK FAILED) direction is proven — the lead's own re-run attempts kept dying to VM contention (SIGKILL at ~sample 160 twice; a third died in a service restart), which is itself recorded rather than papered over | **PASS — correction CLOSED** |
| Lead | **§15 fold-in + reservation guard** `4dd6530`: W31's paste-ready §15 appended verbatim to `docs/scene-format.md`; §15.10 step 1 in src (scenefile `load()` + `walk_nodes`, film `load_film_data`) hard-errors the reserved blocks/keys with the §15.1 named messages. W65 (Batch 7) post-fold census: **75/75 clauses FOUND, none weakened**; pre-fold baseline honestly 0/75 | LANDED + CENSUS PASS |
| Lead | Tools-ci pack 2 `8b9ac36` (W55 verbatim + the §15 conformance steps): all steps verified locally pre-commit (ratchet PASS 196/15, sheet 10/10, format15 corpus PASS, planted 9/9) | SUCCESS in CI at the wiring head |
| Incident 3 | **Tools CI RED at `2c41313`** (run 43625940101) — failed ONLY at the new sheet step: every case rc 2 (make_sheet's ffmpeg-absent exit) on the hosted image; the ratchet, the migrated-tree asset steps and everything upstream passed. **Third tools-ci root cause, same family as incidents 1–2: an environment assumption local runs couldn't see** (W55 assumed the ubuntu-22.04 image ships ffmpeg; it does not guarantee it). Lead fix `f4f6ccf`: the workflow installs ffmpeg before the suite (the suite's own ffmpeg-absent case manipulates PATH internally, unaffected). **CI + Tools CI both SUCCESS at `f4f6ccf`** (43626299883 / 43626299876) | FIXED (lead) |
| Lead | F2 first landing `a2ec80c` gates: film probe dispatch 43624533281 — 795–814 **20/20 byte-identical** vs defaults (the loud fallbacks are pixel-neutral for valid data) | PASS |
| Lead | **F2 migration COMPLETE**: Phase D `20a81a7` (data declares calendarYears/calendarDark/sign; regeneration diff = exactly the 3 keys; its tools-ci red was the ratchet window W53 predicted — the checker did not know the keys until T2 — closed in the same series) → Phases L+T `2c41313` (loader demands the keys, plates manifest atomic swap, calendar_state count from data + zero-years guard, check_assets re-pointed 86 refs 6/6, unknown-fields + rerender-plan taught). Gates: F2-isolation probe diff vs the blink-era set = **zero frames**; calendar range (1000–1061) diffs are blink-scale only (mean \|Δ\| ≤ 0.014, ≤ 0.01 % px — no wrong-year signature). Main CI SUCCESS at both heads | **LANDED, ZERO FRAME IMPACT** |
| Lead | **Blink adoption** `5a9383e` (W58 pack, max(channel, beats)) gates — dispatches 43625505071 / 43625513647 / 43625521417: 195–204 changes **only f204**, 795–814 **only f814**, 1195–1204 **none** — the W58 census promise exact (38/40 probe frames assertable, the 2 changed frames are the promised ones). Baseline note: 195–204 was compared against the `aadc193` window renders, NOT `frames4` — the f021991 set's s02 frames differ from the current generation in a caption-band region and are **not a clean baseline for s02**; `.w2-probes` + `.w2-windows` are the baselines of record | **LANDED, GATE EXACT** |
| B6 push | HEAD `3a3a427` (adoption-preparation batch; gate PASS-WITH-NOTES, GATE2-W60 §9 = the landing packet) | CI + tools-ci SUCCESS |
| B7 push | HEAD `b2e46d2` (gate PASS-WITH-NOTES, GATE2-W70 §12 = the ordered packet; W68 PC dry-run landed after Batch 9's gate and is committed in `cc38b03`) | **CI (43626618994) + Tools CI (43626619083) both SUCCESS — tree green at the current head** |
| Lead | **W61 tail splice** `33c5b2a`: KNOBS-W43's truncation markers deleted, W61's provenance-tagged restored tail appended verbatim (§4.2/§4.3/§5/§6; T2 INCLUDED); 6 UNRECOVERABLE items stay registered in W61 §3. Five content spot-checks re-run at the splice head: all found BY CONTENT (one cite drifted film.rs 2545→2630 — drift, not contradiction) | LANDED |
| Lead | **DOCFIX2-W67 applied** `c7aa648` (supersedes W38 wholesale — nothing of W38 was ever applied): FINAL-REPORT Addendum 2 refreshed; RESULTS discharge note; FOG §8 WIRED note; film README's five stale lines; root README CLI pointer + CHECK OK clause + struck-pins clause; PERF §9 discharge note + item-5 status + corrections. Two disclosed adaptations (gate's N2 rule: the adapter paragraph closes with W62's 8/8 PASS; the F2 sentence completed with `20a81a7`/`2c41313`). Plus the lead's PERF §7 step-2 corrections (No-adapter / Device-failure rows are TaleResult errors at exit 3 since `edb058f`, message texts verbatim) | LANDED |
| Lead | **F4 RECORD CORRECTION** `4af54c5` (see §10's F4 block): the phase-1 landing relayed during Batch 7/8 never happened; W73 verified the absence airtight at `b2e46d2`, W79 + W76 independently confirm. The correction stands; the implementation below is the real one | CORRECTED |
| Lead | **Knobs phase 1** `ea98458`: Knobs {res, aa, shadow} + parsers (exit 2 pre-GPU), `effective_size` identity structural at res==1, HELP per W43 §2 verbatim, tags end-appended/empty at defaults, F8 overlay scaling bit-identical at 1080×1920 | CI 43627762918 SUCCESS |
| Lead | **Knobs phase 2** `9ad2177`: `--aa` — one attachment shape in all four scene passes, HDR MSAA colour + depth twin together, ab1's `basic_overlay` trap line held at literal 1. Its four first probes all FAILED at Build (rustc unexpected closing delimiter, ab1.rs:1438 — the lead's replacement left the pre-knob `})],` dangling) | RED → fixed next row |
| Lead | Phase-2 build fix `c440d90` (one line, no logic change): probes 43628254203 / 43628262354 SUCCESS byte-identical; dof 43628270652 `957d5573…`; ab1 43628279059 renders the Act-2 frame | PASS |
| Lead | **Knobs phase 3** `76b922c`: `--shadow` lockstep in film + ab1 via the `shadow_misc` uniform (bit-identical at 2048; `--shadow 4096` fails visibly, never clamped; inert-but-tagging in scenefile/dof/m1); knobs gate workflow `2017e86` (legs A defaults-identity / B draft-smoke / C validation; passes only on the exact `KNOBS GATE: PASS` line) | LANDED |
| B8/W81 | **F4 ACCEPTANCE:** knobs gate dispatches 43628289244 (lead) + 43628339967 (W81, independent) at `c440d90` both report `KNOBS GATE: PASS` | **F4 COMPLETE + ACCEPTED** |
| Lead | **W76 micro-fixes** `d967931`: finish.sh AAC 192k→256k; HELP `--film` entry corrected (scenes/film/ is the tools' manifest copy, never read by `--film`); HELP EXIT CODES completed (exit 4 ImageIo; 2 widened to data-load; 3 widened to GPU/render failures in any mode). The `-ar` question NOT taken (premise unverified — disclosed in the commit) | LANDED |
| Lead | **W72 VR register update** `3c07564` (vr_triage.py hunks A+B+C + fixtures rebuilt on the post-pin steady state; self-test PASS 4/4; planted cases isolated; anti-whitelist fixture names film-s03) | LANDED |
| Lead | VR dispatch 43628892930 at `3c07564`: runner FAIL on s28 alone; triage from the run's own artifact = **3 OK · 15 KNOWN-DEBT · 0 NEW-SIGNAL — AS EXPECTED** | AS EXPECTED |
| Lead | **F3-a strip** `2fcffc0` (W74 pack §6, one atomic four-file commit): generator stops emitting the six dead paths; `film/onoda.json` = the staged strip; baseline 196 → 24/9 (fps/frames F3-a-kept, hard readers in the Python tools); STRICTNESS-W42 §6 disposition line. Pre-commit: regeneration byte-identical to the staged strip; checker reads exactly 24/9 | CI 43629008511 + Tools CI 43629008503 SUCCESS |
| Lead | **W71 §4 splice + W78 paste-ins** `108f33f`: FOLDIN-W65.md completed (§§1–4; §3's other lost rows NOT restored — the gate's §3 re-run stands in). PC-EVIDENCE-W36 gains G1 (after the §5.1 table — the literal anchor would have split it; deviation disclosed in the commit), G2 as §1's final subsection, G4 in §1, G5 as §2.4 | CI 43629313479 SUCCESS |
| Lead | **G7 tables in the binary** `3223509` (TABLES-IMPL-W85): `--film-telemetry --tables` prints shot/caption/cast tables AFTER the verdict line from a separate Vec (default output a strict prefix; exit codes unmoved). telemetry.rs: `shot_at`, `PresenceRow`/`note_presence`, `shot_frame_ranges`, `beat_frames`, `SHOT_DRESSING` + `BEAT_BINDINGS`, `print_film_tables`; film.rs accumulates presence per sample. Disclosed deltas vs the W75 prototype in the commit | LANDED |
| Lead | Tables test red at `9380b01` (CI 43629817118, the first full CI on the tables code — its own run was cancelled): 179 passed / 1 failed — `tables_beat_frames_tie_to_blink_at` (left 0, right 3). **Diagnosis: the test fixture was wrong, the implementation is right** — the test, copied literally from W85 §6.1, counted over frames `0..200` (t ≤ 6.63 s), which never reaches the 14.78 s fixture beat (frame ~443); the expectation 3 is correct over the full range (frames 444–446). `beat_frames` transcribes blink_at's window exactly; W85 §5's cast-table expectations (25 beat-frames) stand. A §6.1 spec erratum is owed (waivers queue) | FAILURE → fixed next row |
| Lead | Test fix `2f5c708` (range `0..600` only): **CI 43630186620 SUCCESS (180/180) + Tools CI 43630186614 SUCCESS**. W93's diff-check (TABLES-VERIFY-W93) independently reached the same conclusion ("test was wrong") and MATCHES the implementation across all spec areas; its §3 strict-prefix proof is the gate-accepted verification — the lead's own §3 step 0 (parent-binary run pair) completed later (A1 == A2, both CHECK OK; see the Close rows), steps 1–5 unexecuted | **GREEN** |
| Lead | **Split amendments + battery leg** `c9b4d86`: SPLIT-AMEND-W82 applied to both blueprints + SPLIT-READY-W66 annotations; `.github/workflows/battery.yml` created | LANDED |
| Lead | Battery first dispatch 43630694752 at `c9b4d86`: `BATTERY: FAIL (7/46)` — **classified as the workflow's expectation defect, not a renderer defect** (run completed cleanly; the 7 written cards are exactly the set `src/battery.rs` defines; 46 is the M4 case count inside the cards). Verdict corrected `6e4fe3c` (`BATTERY: PASS (7/7 cards)` on completion + the full named set); **re-dispatch 43631562001 SUCCESS** — the leg passes only on the exact line, so the corrected pass is reported; the SPLIT-AMEND-W82 §5.3 precondition is discharged | CLASSIFIED + FIXED + PASSING |
| B9 push | HEAD `9380b01` (gate PASS-WITH-NOTES, GATE2-W90; all seats W81–W89 ACCEPT) | CI red was the lead's tables test (above) → green at `2f5c708` |
| B10 push | HEAD `c297692` (gate W100: **COMPLETE-WITH-WAIVERS**, W-1..W-12; 100/100 seats). CI + Tools CI SUCCESS at `cc38b03` (43630790697 / 43630790659); CI at `c297692` (43631412123) **cancelled** — concurrency supersession by the battery-fix push `6e4fe3c`, a cancellation, not a failure; the final fold-in head's CI is the closing verdict (appended below) | WAVE CLOSED |
| Lead | **Final fold-in + close-out finalised** `bf0a3a9` (this record's §11/§12; WAVE2-CLOSEOUT-W92.md DRAFT dropped). Its own CI was cancelled by supersession; CI SUCCESS at its parent `6e4fe3c` (43631559438, docs/workflow-only delta) | LANDED |
| Lead | **Waivers 1/4 — runbook F1–F6 patches** `cb93083` (DRYRUN-FIX-W96 §§2–7; the §8 W77 waiver NOT entered — moot, the note landed in `cc38b03`). Pack's own verification: no `./talerender-bin` remains under `docs/runbook/` | LANDED |
| Lead | **Waivers 2/4 — F3-b/c data + s12 bake (data side)** `52ca55f`: staged files applied (s12 = the composed bake+note file, NOT W88's pre-bake staging); baseline 24→3; sets README B2 amended; ANIM-ADOPT-W32 resolution pointer added. Local: ratchet PASS at exactly 3/3; conversion census **14/14 PASS** (W83 §2.4 item 1). **Tools CI 43632214182 SUCCESS** (ratchet verified in CI at baseline 3) | LANDED + CI-VERIFIED |
| Lead | **Waivers 3/4 — `near` CONSUME** `13ad26b`: scenefile `key_near` + checker LIGHT_KEY + §14.2 example + s10 follow-up; baseline 3→2; local register exactly 2/2. **Tools CI 43632304639 FAILED** — the light-rig validator (`check_lights.py`) carries a second closed key vocabulary the W88 spec did not name, and the s10 follow-up tripped it | RED → fixed next row |
| Lead | Near-consume follow-up `2294f23`: `check_lights.py` KEY_KEYS + header gain `near` (the loader reads it now; the validator's premise updated); lights validator 6/6 locally | FIXED |
| Lead | **Waivers 4/4 — §6.1 spec erratum** `2cbd090` (TABLES-IMPL-W85 item 4: range read as `0..600`; W93's diagnosis) | LANDED |
| Lead | **Tables §3 step 0 — COMPLETED after the fold-in text was written:** the parent-binary run pair finished: A1 == A2 byte-identical, both rc 0, `FILM TELEMETRY: CHECK OK` (pre-change stdout deterministic). §3 steps 1–5 remain unexecuted by the lead; W93's diff-check stands as the gate-accepted verification of the tables landing | STEP 0 PASS |
| Close | **CI 43632390675 + Tools CI 43632390729 both SUCCESS at `2294f23`** — the closing head is green on both workflows (the near-consume src change is CI-verified here; the `13ad26b`/`2cbd090` CI runs were supersession-cancelled). Post-landing telemetry with this head's CI binary: **`FILM TELEMETRY: CHECK OK`, exit 0** (W83 §2.4 item 2 discharged) | **GREEN AT CLOSE** |

**Watch items / flags recorded by the lead:**
- **Painter panic observed once, not reproduced.** The first
  telemetry run at the exit-code binary (66-chunk driver data,
  scenes side already at 73 — the half-migrated tree) panicked at
  `src/painters/cutout_build.rs:44` ("skin paint panicked: fill
  contour start outside painted skin area"), exit 101. The
  post-regen run over the consistent 73/73 tree completed all 241
  samples with CHECK OK. No root cause is claimed; if it recurs
  it is a painter-robustness finding. (Three concurrent local
  telemetry processes from the lead's own attempts also contended
  for this VM's CPUs during that window; the stale two were
  killed.)
- **PERF.md §1 dated correction owed** (W02 §3d): the "binary
  exits 0 on pass, fail, and error alike" sentence is stale
  since `aa10b13` — the line is still the verdict; the exit code
  now agrees with it. For the doc seat / orchestrator (PERF.md
  is not lead-owned). *Fold-in note: the §7 error-table rows
  were corrected at `c7aa648`; the §1 sentence itself remains
  open — carried, not dropped.*
- **render.yml lead edit** (`paint_budget_mb` dispatch input,
  additive, empty = default) — made so the W06 byte-probe's
  forced-eviction leg is dispatchable; flagged for the
  orchestrator's ratification.
- **Split-programme hazards (W66, Batch 7):** W21 §4's adapter
  paragraph is VOID (`aadc193` deleted the helper it preserves)
  — strike it before executing the splits; every G-TEL gate
  text in the split packs expects the OLD `CHECK FAILED` line
  and must be re-worded to `FILM TELEMETRY: CHECK OK` / exit 0,
  or the gates fail a healthy tree. `state_key` freeze verified
  byte-identical at five heads (at film.rs:2246 after the
  Batch-6/7-era landings). Splits run SERIALLY (film, then
  scene) and must not interleave with §15 step 2.
  *Fold-in update: the SPLIT-AMEND-W82 amendments were APPLIED
  at `c9b4d86` (G-TEL defined on CHECK OK/exit 0 in both
  blueprints, adapter paragraph struck, SPLIT-READY-W66 §3c +
  step-7 annotated SUPERSEDED) and the battery leg the
  amendments' §5.3 precondition needs exists (`.github/
  workflows/battery.yml`). The leg's first dispatch
  (43630694752) reported `BATTERY: FAIL (7/46)` — classified
  from the run's own artifact as the workflow's expectation
  defect, not a renderer defect: the run completed cleanly and
  wrote exactly the 7 named cards `src/battery.rs` defines
  (46 is the M4 stroke-case count painted inside the cards,
  never a file count). Verdict corrected to the named set
  (`BATTERY: PASS (7/7 cards)`, `6e4fe3c`); byte-pinning stays
  out (the stroke-toon pin predates the current 2560×2816
  output; the VR register carries it as NOT-RENDERED debt).
  The precondition discharges when a dispatch reports the pass.*
- **Runbook F1 — BLOCKING for the PC session (W68, verified by
  W96 at HEAD):** PC-EVIDENCE-W36 §0.2 provisions the binary to
  `~/talerender-bin` while §§0.4/2.1/3.1/3.2 invoke
  `./talerender-bin/talerender` from the repo root (exit 127
  reproduced by W68); the same defect lives in USER-PC.md
  §§3–5. Fix = DRYRUN-FIX-W96's canonical-path pack
  (`~/talerender-bin` + `TALERENDER_BIN`, 13 invocation lines);
  it lands first in the waivers queue (§12), before the PC
  visit. W68's F2–F6 errata ride the same revision.
- **s28 disposition (user decision, W63 §7):** re-pin to
  current ab1 output / retire from the registry / leave as the
  designed red. W03's standing recommendation is against
  re-pinning. Until ruled, VR reads FAIL with s28 the only
  DRIFT row — steady state, not a new failure.
- **Caption veto pricing (W69):** a veto of the 73-chunk
  re-chunk stales exactly the 3 in-window golden pins
  (s07/s08/s11) and restores the Wave-1 waiver state; the
  brief is decision-ready for the user at wave close
  (recommendation on record: KEEP 73).
- **`frames4` baseline caveat:** the f021991 frame set is not
  uniform for s02 (its 195–204 frames differ from the current
  generation outside any audited change). Byte-probe baselines
  of record are `.w2-probes` (795–814, 1195–1204) and the
  `aadc193` window renders (`.w2-windows`); do not gate new
  work against `frames4` outside the ranges where it was
  verified (795–814 / 1195–1204 / f1231).

## 8. Batch ledger

| Batch | Seats | Scope (short) | Seat handoffs received | Lead integration | CI |
|---|---|---|---|---|---|
| 1 | W01–W10 | Phase 1 + foundations | Gate PASS (GATE2-W10) | Exit codes ✓; caption driver leg ✓ CHECK OK; errors step 1 ✓; paintbudget/runlog/gpucore registered ✓ (wiring: paintbudget ✓, runlog ✓, gpucore per-site in progress); pins struck `916a109`/`166873c`, Act 1 corrected `2755ff3`; s28 unpinned by design | green at `86f0c81` |
| 2 | W11–W20 | Phase 4/5 foundations: anim, charanim, rendergraph, format extensions, visual-regression CI, bench, atlas, bufferpool | Gate PASS (GATE2-W20) | All 5 modules registered ✓ (+65 tests in suite); adoption decisions recorded §9; format fold-ins + per-site adoptions in progress | green at `1a62161` |
| 3 | W21–W30 | film/scene modularisation blueprints, lip-sync data, locomotion, camera moves, validators, planners, checkers | Gate PASS-WITH-NOTES (GATE2-W30) | Validators wired via W39 (below); split programmes noted SERIAL (film.rs then scene.rs); locomotion adoption held for A/B sign-off (Batch 6 W57 pack) | green at `ff52b3a` (tools) |
| 4 | W31–W40 | §15 draft, anim staging tool, blink evidence, gpucore probes, errors step-2 pack, PC runbook, VR triage, doc-fix pack, tools-ci wiring, gate | Gate PASS-WITH-NOTES (GATE2-W40) | W39 wiring landed verbatim (`630d82e`); W35 pack consumed by errors step 2; W33 blink = Batch-6 W58 input; W31 draft = §15 fold-in source; W38 pack queued for application | green at `f6616aa` |
| 5 | W41–W50 | Gap-fix batch (user order 23:24): F1 verify + F2 audit, F3 strictness checker, F4 knobs spec, F5 sheet tool, F6 cues design, v1.0.1 pack, exporter spec, selftest leg, traceability, gate | Gate PASS-WITH-NOTES (GATE2-W50) | F1 verified landed (`aadc193`); F2/F3/F4 dispositions in §10; v1.0.1 NOT releasable at snapshot (procedure ready, user confirms at close) | green via `aa92cee` |
| 6 | W51–W60 | §15 conformance checker, s12 bake pack, F2 migration staging pack, knobs patch pack + gate, tools-ci pack 2, VRAM telemetry design, locomotion A/B pack, blink patch pack, lipsync evaluation, gate | Gate PASS-WITH-NOTES (GATE2-W60) | Tools-ci pack 2 ✓ (`8b9ac36`); §15 fold-in + guard ✓ (`4dd6530`); F2 migration ✓ (D `20a81a7`, L+T `2c41313`); blink ✓ (`5a9383e`, gate exact); lipsync DEFERRED to user sign-off (gate §9.3: A/B sheet on line 3 + the diegetic windows first); locomotion = user-eye decision (W57 census + LOCOMOTION-AB-W57); still queued: knobs implementation (W54 pack), VRAM `gpu-mem:` line (W56), s12 baked-corpus adoption (W52) | green at `3a3a427`; lead landings green through `f4f6ccf` |
| 7 | W61–W70 | W43 tail restoration, adapter re-proof, golden-verify + VR triage, first-bench analysis, §15 fold-in checker, split readiness, docfix refresh, PC dry-run, caption-veto brief, gate | Gate PASS-WITH-NOTES (GATE2-W70) | W62 adapter PASS closes the §7 correction; W63 verified the Act-1 corrective + corrected the Act-2 cite (fixed in §7); W65 census 75/75; W66 split hazards recorded (below); W61 tail splice landed `33c5b2a`; DOCFIX2 applied `c7aa648`; W69 veto brief ready for the user (veto ⇒ 3 in-window pins stale + restore from candidates; recommendation on record: KEEP 73); W68 PC dry-run landed late (PC-DRYRUN-W68.md; its F1 is carried as a watch item) | green at `b2e46d2` (CI + Tools CI SUCCESS) |
| 8 | W71–W80 | Fold-in addendum, VR register update, knobs phases 2–3 staging + phase-1 verification, F3-a strip pack, G7 telemetry tables, micro-fix verification, exporter acceptance harness, E7 RADV readiness audit, record completeness audit + close skeleton, gate | Gate PASS-WITH-NOTES (GATE2-W80; §12 = the ordered fixes-pass packet) | All of §12.1's items patch-ready at close; lead landings in the Batch-9 window: W72 register + proof dispatch ✓ (43628892930 AS EXPECTED), W76 micro-fixes ✓ (`d967931`), F3-a strip ✓ (`2fcffc0`, 196→24/9), W71 splice + W78 paste-ins ✓ (`108f33f`), G7 `--tables` ✓ (`3223509`) | green at `021ca01` |
| 9 | W81–W90 | Knobs-gate verification, split amendments, s12 adoption readiness, VRAM implementation staging, G7 tables impl spec, lipsync binding decision, locomotion step-0 + decision sheet, F3-b/c closures, v1.0.1 pack refresh, gate | Gate PASS-WITH-NOTES (GATE2-W90) | F4 ACCEPTED (gate PASS in two independent runs at `c440d90`); split amendments applied `c9b4d86` + battery leg established; s12/VRAM/F3-b/c/splits staged-not-landed (waivers queue); W68 landed (F1 watch item + W96 fix pack) | green via `2f5c708` (CI 180/180) |
| 10 | W91–W100 | Final record audit, close-out draft, tables verification, six-decisions brief, goal review, runbook dispositions, traceability re-run, PC session pack, gates sweep, wave verdict | Gate W100: **COMPLETE-WITH-WAIVERS** (W-1..W-12); 100/100 seats | Final fold-in (this record, §11/§12) + close-out finalised; waivers queue lands after: runbook F1 (W96), s12 (W83), F3-b/c (W88), §6.1 erratum (W93) | CI + Tools CI SUCCESS at `cc38b03`; final-head CI appended below |

## 9. Lead decisions (Batch-2 adoption, recorded 2026-11-14)

1. **Colour/step anim lane (the batch's one unowned gap) —
   DECIDED:** the evaluator extension is `AnimValue::Color`
   (linear-space `[f32; 3]`, converted from authored hex by the
   loader's existing hex→linear law before the track carries it,
   per W15 §4's colour law) plus a **step sampling mode** on
   tracks (hold the leaving key's value; no interpolation) for
   enum/string channels (`mood`, `legs`). String values ride a
   side channel keyed like the numeric tracks — they never route
   through numeric interpolation. Implementation lands with the
   clips loader adoption; the decision is recorded in
   scene-format §15's fold-in so the format text and the code
   agree from the start.
2. **Ease-direction bridge — RATIFIED:** W11 §4.1's rule applies
   to clips exactly as to §14.4 camera keys (loader remaps
   format key[i+1].ease → anim key[i].ease). Recorded in §15.
3. **W12/W11 layering — RATIFIED as independent/complementary**
   (GATE2-W20 §4.4): charanim's procedural channels stay
   closed-form; authored character channels keyframe through
   W11 tracks at clips adoption. No future seat should merge
   them by assumption.
4. **W18 atlas — adoption rule ACCEPTED as the gate:** wire
   only with the probation admission policy (Dyn first paint,
   atlas slot on second hit, lazy pages to `max_pages`, no
   mid-film eviction) and discharge pixel-neutrality by
   ATLAS-W18 §7's byte-probes post-wiring. Never pack-everything.
5. **W16 first dispatch expectation — NOTED:** FAIL on
   `ab1-composer` + `s28-onoda-t150` (stale pre-shade-fix pins)
   with `dof-chart` as clean control is the expected first red,
   per the workflow's own annotation — not new drift. The 80 dB
   band is not widened; pins move only by the human pin acts.
6. **Wrap/duration placement (W15 §7 flag 1) — RATIFIED:**
   duration on the clip, wrap on the instance (`clipTime`),
   bridged mechanically by the loader; anim's per-track
   duration/wrap remains the evaluator's internal shape.

## 10. Gap-fix packet (user order 2026-11-14 23:24) — dispositions

Stale claims in the rev-2 analysis are NOT re-fixed (source in
VCS, scene-format.md exists, telemetry exits 1/3 since
`aa10b13`, CLI panic-free exit 2 since `d7bebdd`, errors
migration COMPLETE — step 2 `edb058f` + fix `aa92cee`) — the
DOCFIX2-W67 pack (applied `c7aa648`, superseding W38 wholesale)
+ the v1.0.1 release (user decision at close) close the
stale-binary side. True gaps:

- **F1 — LANDED** (`aadc193`): `load_film_data` accepts
  `film.json` as the generic entry, `onoda.json` fallback;
  `--help` agrees with the loader (also closes W28 F4; W28 F3's
  `--caption` CWD-relative default is documented in `--help`).
  W41 verified the landing in Batch 5.
- **F2 — LANDED** (first landing `a2ec80c`; Phase D `20a81a7`;
  Phases L+T `2c41313`; see the §7 F2 rows): of 13 film-driver
  loads, only caption strips were data-referenced; plates /
  calendars / sign loaded unconditionally and two loads had
  silent fallbacks. The migration made the calendar years/dark
  variant and the sign data-declared, the plates load a
  validated manifest (legacy form rejected by name), and
  `calendarYears`/`calendarDark` demanded via `g()`. Gates:
  isolation probe zero-frame diff; calendar-range diffs
  blink-scale only. film.rs's transcribed staging was NOT
  generalised blindly (packet instruction).
- **F3 — CHECKER LANDED (W42), policy = Option A:** real data
  carries 196 unknown-field occurrences in 15 paths / 3
  classes. Recommendation (lead concurs with W42 + GATE2-W50):
  report-only tools-ci ratchet against the 196 baseline;
  hard errors deferred to a format v3 decision (user-visible
  at wave close). Dispositions: F3-a — 174 occurrences are
  dead provenance written by `gen_film_data.py` (strip at the
  generator; data-only change, gated on telemetry CHECK OK) —
  **LANDED `2fcffc0`**: the generator stops emitting the six
  dead paths; baseline 196 → **24 occurrences / 9 paths** (the
  briefed 22 was corrected by W74: `fps`/`frames` have hard
  readers in `blink_cost`/`blink_census` and are re-classed
  F3-a-kept, not stripped); STRICTNESS-W42 §6 disposition line
  added. F3-b — 19 prose `note` fields + s10 `motifLaw`: **lead
  ruling: CITE, don't strip** (they are the staging's
  documentation; the checker learns them as known-optional) —
  discharged by F3BC-W88 §1's per-item audit, which is the
  citation record; the strips ride the F3-b/c landing in the
  waivers queue (§12). F3-c — bookshop `nodes[].materials` is a
  shadow format (sharpest): resolved asymmetrically per
  F3BC-W88 — `materials` is STRIPPED by decomposition into the
  canonical group+box+quad form (the set's README carries the
  amendment), while the ignored lights.key `near` is
  CONSUMED (the checker was right; the writer's intent was the
  loader's default) — both staged (24→3→2), landing in the
  waivers queue (§12).
- **F4 — SPEC + PATCH PACKS COMPLETE; IMPLEMENTATION NOT
  STARTED (CORRECTION, 2026-11-15).** A phase-1 landing
  (`--res` + DRAFT tag) was relayed during Batch 7/8 and is
  **retracted as a record error** (W73 register item P6):
  W73 verified airtight at `b2e46d2` — no Knobs struct, no
  `effective_size`, no parse arms for `--res`/`--aa`/`--shadow`
  in `src/`; the parse loop rejects all four flags as unknown
  arguments; HELP ends OPTIONS at `--to`. W79's audit and
  W76's HELP check independently grep the same absence. The
  540×960 / 22.35 dB figure attached to the claim cannot have
  come through this repo's render.yml (it has no size input);
  W73 §1's divergence register (P1–P6) is the full account,
  and the orchestrator has corrected it with the user. TRUE
  state: spec = KNOBS-W43 (truncation repaired by the lead's
  W61-tail splice, T2 included) + patch packs W54 and
  KNOBS-P23-W73 (phases 2–3 cites refreshed; W73 is now the
  implementation basis). Laws: defaults byte-identical
  (the gate proves it); knobs are draft/preview only, finals
  stay full quality; draft output is TAGGED as draft — and
  per W73's warning, `is_draft()` gains the aa/shadow terms
  in the SAME commits that land those knobs, or untagged
  drafts pass as finals. **Gate
  conflict (3) — lead ruling:** W43's caption-scale behaviour
  vs the strips contract — captions are pre-rendered strips
  composited post-grade (standing contract); at draft
  resolutions the strip scales WITH the frame (it is part of
  the composed image, not an overlay re-render), so no
  re-rendering of strips is introduced; the knob spec's
  scale-with-frame reading stands.
  **LANDED + ACCEPTED (post-correction):** phase 1 `ea98458`
  (Knobs + parsers + `effective_size` identity + HELP; CI
  43627762918 SUCCESS), phase 2 `9ad2177` (`--aa`, one
  attachment shape in all four scene passes; its four first
  probes FAILED at Build on a one-line delimiter defect in
  the lead's edit — fix `c440d90`, no logic change), phase 3
  `76b922c` (`--shadow` lockstep in film + ab1; 4096 fails
  visibly, never clamped), gate workflow `2017e86`.
  Verification at `c440d90`: byte-identity probes 43628254203 /
  43628262354 SUCCESS, dof pin `957d5573…`, ab1 renders the
  Act-2 frame; knobs gate dispatches 43628289244 (lead) and
  43628339967 (W81, independent) both report
  `KNOBS GATE: PASS` — **F4 COMPLETE + ACCEPTED**.
- **F5 — TOOL LANDED (W44):** native sheet tool (seat tool,
  tests 10/10, honest no-audit scope). A `--sheet` mode in the
  binary is NOT duplicated on top of it; the adapter covers
  review at the frame(t) seam.
- **F6 — DESIGN ONLY (W45)** as scoped: emitted-entry shape
  marked PROPOSED; the skill-side Q1 stays open for the user.

**First dispatches at `aa92cee` (orchestrator):** bench
43623743480 + visual-regression 43623748951 — bench SUCCESS
(§7 row); the pre-pin VR run FAILED on the expected UNPINNED
pattern (W37 triage AS-EXPECTED, seat-reported); the post-pin
steady state is §7's 43625139818 row. (Both closing-promise
sentences discharged at fold-in.)

**Errors step 2 gate dispatches (lead, at the post-Act-2
head):** the §7 "Errors step-2 gates" row is the record
(dof-chart 43624002645; fx scene-file 43624248449 — the
correct-fixture re-run superseding the wrong-scene dispatch
43624007505 named here when written; film probe 43624011865;
determinism 43624016907; all PASS) plus the no-adapter proof
(CI binary + `VK_ICD_FILENAMES` pointed nowhere →
`error: GPU initialisation failed …`, exit 3 — W35 P6 gate,
PASS).

*(Filled per batch as the orchestrator hands over.)*

## 11. Corrections register (fold-in, wave close)

Corrections are first-class record content in this wave; each
entry below is one the lead can stand behind from first-hand
evidence (its own rows above cite the underlying runs). The
Batch-10 audit (RECORD-AUDIT2-W91 §3) enumerates the register
at finer grain (C-1..C-17, including commit-message-only
corrections); this section is the lead's verified subset, and
where the two differ in grain the audit governs the count.

- **C-Adapter.** §7's original adapter re-proof PASS (FAILED
  direction) was false: the run was SIGKILLed mid-telemetry
  under VM contention and its fixture mixed 66-chunk data with
  73-era strips. Retracted in §7; re-proven independently by
  W62 (8/8 cells) and closed with an outside verdict.
- **C-Act1.** Golden Act 1 as committed pinned the staged
  `381ff47`-era bytes for the 11 out-of-window mids. Caught by
  the VR canary's first measured run; corrective `2755ff3`
  re-pinned the candidates (diff exactly 10 film lines; W63
  verified independently).
- **C-Act2-cite.** Act 2's provenance cite read `9f497c7`; the
  fresh-render run's head was `dc0e555`. Corrected in §7 (the
  Act-2 commit message carries the slip and is immutable; the
  record row is the correction of record).
- **C-KnobsPhase1.** A knobs phase-1 landing relayed during
  Batch 7/8 never happened (verified airtight at `b2e46d2`:
  no Knobs struct / `effective_size` / parse arms / DRAFT
  string in `src/`). Corrected in `4af54c5` and with the user;
  the 540×960 / 22.35 dB figure's provenance is void (render.yml
  has no size input). The real implementation is §7's
  `ea98458`/`9ad2177`/`c440d90`/`76b922c` arc, accepted by two
  independent gate runs.
- **C-ToolsCI-ffmpeg.** The tools-ci suite's sheet step assumed
  the hosted image ships ffmpeg; it does not. Third tools-ci
  root cause, same family as incidents 1–2 (environment
  assumptions local runs couldn't see). Fixed `f4f6ccf`.
- **C-W43-truncation.** The committed KNOBS-W43.md was
  truncated at line 264 with a literal truncation marker;
  restored by the lead's splice `33c5b2a` from W61's
  provenance-tagged tail (T2 included; 6 items UNRECOVERABLE,
  registered in W61 §3).
- **C-FOLDIN-truncation.** FOLDIN-W65.md was itself
  transport-truncated (§4 never landed); completed by the
  lead's splice in `108f33f` (W71 addendum). §3's other lost
  rows were NOT restored — the gate's §3 re-run stands in.
- **C-GTEL.** Every G-TEL gate text in the split packs
  expected the pre-re-chunk `CHECK FAILED` line; re-worded to
  `CHECK OK` / exit 0 by the SPLIT-AMEND-W82 application
  (`c9b4d86`), together with striking W21 §4's void adapter
  paragraph (`aadc193` deleted the helper it preserved).
- **C-F3a-figure.** The briefed F3-a outcome (196→22) was a
  misprojection: `fps`/`frames` have hard readers in the
  Python tools and are kept (F3-a-kept); the landed baseline
  is 24 occurrences / 9 paths (`2fcffc0`).
- **C-TablesTest.** The one red test of the close
  (`tables_beat_frames_tie_to_blink_at`) was a spec-fixture
  error in TABLES-IMPL-W85 §6.1 (counting range `0..200` cannot
  reach its own 14.78 s fixture beat), not an implementation
  defect; the landed test uses `0..600` (`2f5c708`); a §6.1
  spec erratum is owed (waivers queue). W93 reached the same
  conclusion independently.
- **C-Battery.** The battery leg's founding expectation (46
  card files) was wrong: `src/battery.rs` writes 7 named
  cards; 46 is the M4 stroke-case count inside them. The leg's
  first dispatch failure was the workflow's defect, classified
  from its own artifact; verdict corrected in `6e4fe3c`.
- **C-BenchWindow.** The bench harness's window is S01-only
  (W64 anomaly A1) — bench numbers in this record are
  in-window figures, not film-wide.

## 12. Wave close — dispositions, decisions, carry-overs

**Gap-fix dispositions at close:** F1 LANDED (`aadc193`) ·
F2 LANDED (`a2ec80c`, `20a81a7`, `2c41313`) · F3-a LANDED
(`2fcffc0`, baseline 24/9) · F3-b/c STAGED (F3BC-W88; the data
landing + the `near` consume land first in the waivers queue,
baseline 24→3→2) · F4 COMPLETE + ACCEPTED (two independent
`KNOBS GATE: PASS` runs) · F5 TOOL LANDED (W44; no binary
`--sheet` duplication, per the lead ruling) · F6 DESIGN ONLY
(W45; skill-side Q1 open for the user).

**Six decisions put to the user at close (DECISIONS-W94;
recommendations on record):** (1) caption veto — KEEP the
73-chunk re-chunk (telemetry CHECK OK) or veto to 66 (waiver
state returns; 3 in-window pins stale, restored from
candidates); (2) s28 disposition — retire recommended
(re-pinning makes it a byte-duplicate of the ab1 pin);
(3) locomotion adoption per LOCOMOTION-STEP0-W87's D1–D4
(+ the zero-pixel step-0 registration line, lead-landable
without D1–D4); (4) lipsync Variant A (builder-side step
keys; adoption binds future speakers only — this film changes
zero frames) after the user's visual sign-off on the line-3
sheet; (5) v1.0.1 release GO (RELEASE-101B-W89 pack; all six
Step-3 gates re-proven on the bump SHA; the v1.0.0 proof MP4
is never re-attached — it predates the re-chunk);
(6) skill-backend adoption as opt-in now (work copy drift 0)
or wait for RADV proof.

**Waivers queue (packs ready, landing after this fold-in, each
reported with its CI verdict):** runbook F1 patch (W96) ·
s12 adoption data side (W83; the film.rs code-side retirement
does NOT land standalone — it rides §15 step 2, byte-probe
gated) · F3-b/c (W88) · §6.1 spec erratum (W93) · then, per
the user's decisions: v1.0.1, the split programmes (22 pure-
move commits under the amended blueprints; S2/S4/S5 cite the
battery leg), VRAM implementation (VRAM-IMPL-W84, rulings
D1–D3 recorded), lipsync Variant A, locomotion step-0.

**Goal carry-overs (the goal stays OPEN):** the RADV evidence
session on the user's PC (runbook + PC-SESSION-W98 pack; the
W96 F1 fix lands first); the fog-chart PSNR leg (needs the
×π-amended Chrome re-capture); §15 step 2 + the adoption
remainders (prefab/clips evaluator lanes, W11 anim wiring,
rendergraph steps 2–3, bufferpool wiring, atlas wiring under
the probation rule); the auto-chunk owner, R3c literal test,
chicken-film render, Cargo.lock and painter-panic repro
(UNRESOLVED rows in the final traceability: 125 rows — 75
LANDED / 19 STAGED / 8 DECISION-OWED / 7 REJECTED / 7
STALE-CLAIM / 9 UNRESOLVED, per W97 + GATE2-W100).

**Final verdict block.** Wave 2 closes
**COMPLETE-WITH-WAIVERS** (GATE2-W100; waivers W-1..W-12).
Seat-close head `c297692`; this fold-in lands on top of it.
CI + Tools CI SUCCESS at `cc38b03` (runs 43630790697 /
43630790659); CI SUCCESS at `2f5c708` with the suite at
180/180; battery re-dispatch 43631562001 SUCCESS (corrected
leg passing — §7). CI at `c297692` itself was cancelled by
concurrency (superseded, not failed). The waivers queue then
landed (`cb93083`, `52ca55f`, `13ad26b` + follow-up `2294f23`,
`2cbd090`), and the closing head `2294f23` is green on both
workflows: CI 43632390675 + Tools CI 43632390729, both
SUCCESS (§7, Close row).
