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
  treats non-zero how? — **RESOLVED (W02 + lead re-proof):** the
  work-copy adapter implements the dual convention (accepts
  FAILED at exit 0 or 1, OK only at 0, fails closed on any
  disagreement or missing line; stub-proven 10/10 by W02 and
  re-proven by the lead with the REAL binary at the exit-code
  head — FAILED direction §7; OK/old-binary directions appended
  there as they land).
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
| Lead | Adapter re-proof, real binary (`ba88596` build) vs 66-chunk fixture: binary prints the FAILED line (7 caption failures — the pre-rechunk verdict reproduced) and exits **1**; work-copy adapter relays `CHECK FAILED`, rc 1 | PASS (FAILED direction, new generation). OK direction + old-binary direction re-proofs run at the same head — rows appended when they land |
| Lead | Module registration `2328e6f` CI (43619241590): 174 passed / **1 failed** — `gpucore::tests::strip_padding_drops_sentinels` (first-ever execution of W07's CI-pending tests; the test's payload pattern aliased its own 0xEE sentinel at x=238 — a seat test bug, not a module defect) | FAILURE → fixed `38930fa` |
| Lead | PaintBudget + RunLog wiring `86f0c81` CI (43619649089) — suite 175/175 incl. the fixed gpucore test | SUCCESS |

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
  is not lead-owned).
- **render.yml lead edit** (`paint_budget_mb` dispatch input,
  additive, empty = default) — made so the W06 byte-probe's
  forced-eviction leg is dispatchable; flagged for the
  orchestrator's ratification.

## 8. Batch ledger

| Batch | Seats | Scope (short) | Seat handoffs received | Lead integration | CI |
|---|---|---|---|---|---|
| 1 | W01–W10 | Phase 1 + foundations | Gate PASS (GATE2-W10) | Exit codes ✓; caption driver leg ✓ CHECK OK; errors step 1 ✓; paintbudget/runlog/gpucore registered ✓ (wiring: paintbudget ✓, runlog ✓, gpucore per-site in progress); pins pending (post-re-render sequencing, GOLDEN-RESTAGE-W03 §4.3) | green at `86f0c81` |
| 2 | W11–W20 | Phase 4/5 foundations: anim, charanim, rendergraph, format extensions, visual-regression CI, bench, atlas, bufferpool | Gate PASS (GATE2-W20) | All 5 modules registered ✓ (+65 tests in suite); adoption decisions recorded §9; format fold-ins + per-site adoptions in progress | green at `1a62161` |
| 3 | W21–W30 | film/scene modularisation blueprints, lip-sync data, locomotion, camera moves, validators, planners, checkers | seats running at this record's writing | — | — |
| 4–10 | — | per orchestrator boards | — | — | — |

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

*(Filled per batch as the orchestrator hands over.)*
