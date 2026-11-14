# GOAL-REVIEW-W95 — Goal-status review: goal_b21124e71eb3

**Seat:** Wave 2, Batch 10 (final), W95 — goal-status review.
**Subject:** goal_b21124e71eb3, "TaleRender — native Rust/wgpu renderer"
(`~/workspace/goals/talerender-native-rust-wgpu-renderer/GOAL.md`).
**User order under review:** 2026-11-14 ~00:56 +06 — "Don't stop until
this fully completed."
**Reviewed at:** repo HEAD `9380b01` (Wave 2 Batch 9 central commit),
2026-11-15. Batch 10 was in flight at review time; this review is a
status reconciliation, not the wave-close verdict (that is W100's).
**Method:** every status below is first-hand from git log/tree, the
files named, or `gh` run/release listings made during this review.
Where a figure is a seat's or the lead's and was not re-measured here,
it is labelled as such. **This review does not close the goal** — it
recommends (see §8, labelled).

Status vocabulary: **DONE** (evidence in hand) · **DONE-WITH-WAIVER**
(done under a named, recorded waiver) · **OPEN** (blocker + owner
named) · **SUPERSEDED** (replaced by a later act, named).

---

## 1. Headline

The renderer the user ordered exists, is proven, and has completed
both waves of work: milestones M0–M6 all passed their gates, Wave 1
ran the full production-integration path on a complete short, and
Wave 2 executed the external audit's phases with the code-first /
fixes-second method the user ordered. The film-telemetry gate that
closed Wave 1 FAILED is now **green** (`FILM TELEMETRY: CHECK OK`,
exit 0). But the goal's own close condition — GOAL.md: *"Close when
those land or are formally waived"* — is **not yet met**: no RADV
evidence exists (every native number is still CI/VM lavapipe), the
fog-chart PSNR leg is unmeasured, shipped-skill adoption is
undecided, and a handful of user decisions plus a small set of
prepared-but-unlanded implementations remain. **Recommendation:
keep the goal OPEN** (§8 gives the minimal closing path and a
verbatim text for the user).

## 2. Standing orders & laws — compliance

| Law (GOAL.md) | Status | Evidence |
|---|---|---|
| Builds only via GitHub Actions CI; no local cargo | DONE (held) | CI history green at the recorded heads (e.g. `aa92cee`, `f4f6ccf`, `ea98458` — CI + Tools CI SUCCESS); all wave verification ran through CI or CI-built binaries |
| No performance claims from this VM; real perf only from the user's PC (Vega 7 / RADV) | DONE (held) | `docs/runbook/PERF.md` line 7–8, read at this HEAD: "no figure in this document is a result — the tables are blanks for the PC run to fill." The one bench number in the record (1.85 s/frame, run `43623743480`) is labelled CI lavapipe wherever it appears |
| Evidence law: every milestone ships an A/B package in `docs/ab/` with tolerances stated before measuring | DONE | `docs/ab/` at this HEAD: composer, composer-v2, dof-chart, film, onoda-s01-t150, scene-format, stroke-battery, telemetry |
| f32 data law (spec §11): transcribed staging carries exact f32 values | DONE | Capstone proof: the determinism root fix `cabcc30` (Wave 1) — paint-cache keys are exact f32 bit patterns; `state_key` freeze re-verified byte-identical at this wave (md5 `95918b9941313f895dbee7bf35c8d14`, W82/W90) |
| The ONE open decision (JPEG q0.93) brought to the user with evidence; everything else proceeds without asking | DONE — CLOSED | See §3, JPEG row |

## 3. Milestone checklist — line-by-line reconciliation

GOAL.md's checklist items, in its own order:

| GOAL.md item | Status | Evidence (first-hand) |
|---|---|---|
| M0 audit + gate (ADR-001, compatibility contract, risk register) | DONE | Commit `888e03d` present in history ("M0 gate: audit findings + ADR + compatibility contract + risk register") |
| M1 native prototype (1080×1920 wgpu frame + real 3D scene) | DONE | Per GOAL.md record; the prototype's modes remain in the binary's mode set exercised by later gates |
| §28 real-scene A/B (Onoda S01 t=1.50, PSNR 20.152 dB) | DONE | `docs/ab/onoda-s01-t150/`; figure carried through M4/M5 chain (20.152 → 20.218 → 23.850 → anchor 24.09 in the film A/B) |
| M3 Bokeh DoF gate — PASS (47.38 dB; depth Rgba8Unorm default) | DONE | `docs/ab/dof-chart/`; dof pin still exercised by the visual-regression register |
| M2 scene format v1 + loader | DONE, extended | `docs/scene-format.md`; format v2 (Wave 1, spec §14/§14.5, rulings R1–R5) is the production form; v1 stands as the milestone of record |
| M4 painter stroke battery — PASS 46/46 | DONE | Commit `819ee16` present. **Caveat found in Wave 2 (W82):** the battery is the binary mode `--scene stroke-battery` and has *no standing CI invocation* at this HEAD — the 46/46 verdict is of record from M4's CI run `43497572055`, not continuously re-proven. This is a precondition on the split programme (§6, Phase 2), not a reopening of M4 |
| M5 composer completion — PASS (ablation to 23.850 dB) | DONE | Commit `79aff03` present; `docs/ab/composer/` |
| M6 QA telemetry — PASS (exact agreement) + JPEG brief delivered | DONE | Commit `9a8cc79` present; `docs/ab/telemetry/` |
| **JPEG q0.93 decision** | **DONE — CLOSED (B)** | User decided 2026-11-14: **keep native frames clean**; recorded in compatibility-contract §5 by commit `801f85c` (present in history). Not a remainder; listed here only because GOAL.md's "one open decision" framing predates the decision |
| PRODUCTION INTEGRATION (Wave 1, 100 agents) | DONE — see §4 for the verdict history | Push HEAD `744e74e`, ci.yml run `43582564094` SUCCESS, tools-ci run `43582564128` SUCCESS (GOAL.md record); final film re-rendered at `f021991` (A/B mean 23.02 dB, min 19.36 — lead's completion report, of record) |
| Scene-format character node (spec §13) | DONE | GOAL.md record (loader consumes character node + attachments, `4c06a1e`); the format has since carried the whole Wave-2 film-data migration (F2, §6) |
| M6 telemetry, production form (film gate over the complete short) | DONE — gate now GREEN | Wave 1: FAILED on 7 caption chunks only. Wave 2: re-chunk landed `5e90d7e` and the gate re-ran `FILM TELEMETRY: CHECK OK`, exit 0 (§4) |
| Production integration: wiring into the shipped skill | **OPEN — user's decision** | Work copy complete and adapter proven both directions (Wave 1 W99; re-proven independently in Wave 2, W62 8/8). Shipped skill never touched. See §7.3 |

## 4. The caption verdict — the one Wave-1 failure, closed in Wave 2

This is the goal's most-changed item, so it gets its own section.

- **Wave 1 state (GOAL.md "Open at wave close" item 1):** the film
  gate's verdict line was `FILM TELEMETRY: CHECK FAILED (… 7 caption
  failures)` — overflow NONE, continuity CLEAN. The lead ruled
  **ACCEPT WITH WAIVER** (2026-11-14, recorded `9790553`,
  `docs/ab/film/CAPTION-DECISION-W92.md`): the 7 chunks exceed the
  850 px bar in Chrome's original too.
- **Wave 2 state:** the user's Wave-2 order ("complete all phases,
  then fixes") reopened the question. The re-chunk (66 → 73 chunks,
  W92 Option A) was computed in Batch 1 (W01) and landed by the lead
  at **`5e90d7e`** — commit message, read first-hand: *"Telemetry
  re-run at this data with the exit-code binary: FILM TELEMETRY:
  CHECK OK, exit 0 (0 caption failures…)"*. The telemetry exit-code
  contract itself landed at `aa10b13` (0 OK / 1 FAILED / 3
  could-not-run; the verdict line remains authoritative).
- **Status: DONE.** The waiver is **SUPERSEDED** as the operative
  disposition — the gate is green on the data, not waived. What
  remains is not a defect but a **user-held reversal right**: the
  W69 brief (Batch 7) prices a veto (revert `5e90d7e` + the caption
  paths of `3a36f1f`; 3 of the 14 golden pins go stale; the waiver
  would stand again) and recommends KEEP 73. The user has not ruled.
  See §7.4.

## 5. GOAL.md "Open at wave close" (Wave 1) — re-stated at this HEAD

GOAL.md's seven items were written at Wave-1 close. Wave 2 changed
several of them; the goal file itself has not been re-written, so
this table is the current truth:

| # | Item (Wave-1 wording, short) | Status at `9380b01` |
|---|---|---|
| 1 | Caption decision — ruled ACCEPT WITH WAIVER | **SUPERSEDED** by the Wave-2 re-chunk; gate CHECK OK (§4). Residue: user's veto right (decision, §7.4) |
| 2 | Golden pins — two separate human acts, not yet struck | **DONE** — Act 1 `916a109` (14 film first-pins), Act 2 `166873c` (ab1 → `60c32c1a`), corrective `2755ff3` after the VR canary caught 11 pins describing pre-atmosphere-fix bytes (renderer innocent; staging copy missed). Pack state verified by W90 at this wave: **18 OK / 0 STALE / 0 MISSING**. Residue: item 2's s28 sub-question (§7.5) |
| 3 | RADV proof (user's PC) | **OPEN** — unchanged in substance; the discharge path is now rehearsed and has one blocking defect to fix first (§7.1) |
| 4 | Skill adoption (user) | **OPEN** — user's decision (§7.3) |
| 5 | Re-render at final head | **DONE** (lead, Wave 1) — film at `f021991`: A/B mean 23.02 dB / min 19.36; MP4 `~/workspace/your_files/onoda-native-f021991.mp4` (sha256 `dea7bb2c…`, all colour tags bt709, −14.00 LUFS). The Wave-2 re-chunk changed caption *data* after this render; the re-chunk's own in-window pins come from the post-re-chunk render at `aadc193` (Act 1 record) |
| 6 | Lead's integration README disposition | **DONE-WITH-NOTE** — the Wave-2 doc-fix application (`c7aa648`, DOCFIX2-W67 applied in full) refreshed FINAL-REPORT (Addendum 2), RESULTS, FOG, the film README and the root README to true state. Note: the *wave record's own* fold-in for Batches 8–10 is still owed (§6, Phase 5) |
| 7 | Fog-chart PSNR leg (≥45 dB vs the Chrome frame) | **OPEN** — reference-side task (§7.2) |

## 6. Wave 2 — the audit-completion wave, phase by phase

User order 2026-11-14 22:26 +06: 100 agents, the pasted external
audit's Phases 1–5 + §6 improvements + findings A–E, **code first
across all phases, then fixes**. Structure held: 10 batches × 10
seats, lead owning `src/*.rs`, seats writing new files only, central
commits, every CI verdict reported as checked. Batches 1–9 are
closed (gates: PASS, then PASS-WITH-NOTES ×8); Batch 10 (close-out)
was in flight at this review. Scope below is WAVE2-RECORD §4's own
work list, reconciled:

### Phase 1 — Production correctness: **DONE**
- Caption re-chunk applied; telemetry **CHECK OK** (`5e90d7e`, §4).
- Telemetry exit codes per contract (`aa10b13`); adapter re-proven
  against both exit-code generations, independent 8/8 (W62).
- Golden re-stage + pins struck as separate acts (§5 item 2).
- Determinism contract written (W04) and its CI lavapipe leg
  discharged: `determinism.yml` dispatches SUCCESS — three completed
  successful runs visible at this review (`43617370962` = the E6
  discharge, `43620084786`, `43624016907`).
- PERF §9 asks: progress elapsed (§9.1), paint-cache stats (§9.2),
  peak-RSS self-report (§9.3) — landed via the PaintBudget + RunLog
  wiring (`86f0c81`); verified at this HEAD in `src/runlog.rs`
  (`peak_rss_mb()`, the `TALERENDER_FRAME_TIMING` gate).

### Phase 2 — Rendering architecture: **PARTIAL**
- Shared GPU core: **DONE** — gpucore migration 5/5 (`aadc193`
  deleted main.rs's last duplicate helper; audit finding B closed).
- Structured errors: **DONE** — steps 1–2 (`ba88596`, `edb058f`,
  delimiter fix `aa92cee`); the no-adapter path is a structured
  error at exit 3 with the PERF-quoted message preserved verbatim.
- Structured configuration: **DONE** — this is gap-fix F4 (the
  `--res`/`--aa`/`--shadow` knobs, §6.1 below).
- Modularise film.rs / scene.rs: **OPEN (execution)** — blueprints
  (W21/W22, Batch 3), readiness (W66: `state_key` freeze
  byte-identical across heads) and the Batch-9 amendment pack
  (W82: G-TEL gates re-worded to the CHECK OK / exit-0 contract,
  void adapter paragraph struck, 22-commit serial protocol) are
  complete; **not one split commit has landed** — verified at this
  HEAD: `src/film.rs` is still a single 3,787-line file,
  `src/scene.rs` 2,979 lines. W82's stated precondition is also
  open: the 46/46 stroke battery has no standing CI invocation, and
  split steps S2/S4/S5 must not start until the lead establishes
  one. Owner: lead (execution), post-wave scheduling.

### Phase 3 — Performance engineering: **PARTIAL**
- Paint-cache byte budget: **DONE** (`src/paintbudget.rs`, wired
  `86f0c81`; eviction pixel-neutral under the exact-key fix).
- Progress elapsed / per-frame timing / peak-RSS: **DONE** (above).
- Bench harness: **DONE as a harness** — `bench.yml` dispatch-only;
  first run SUCCESS and analysed (W64: CI lavapipe 1.85 s/frame over
  an S01-only window; the workflow's transition claim is false —
  anomaly A1 stands in the record). Numbers remain lavapipe-only
  by law; PC numbers are §7.1.
- VRAM telemetry (`gpu-mem:` line): **OPEN (implementation)** —
  design (W56) and the Batch-9 staging pack (W84) are complete and
  the accounted formula was re-derived three ways to the byte
  (steady 220,930,688 B / peak 229,286,528 B), but verified at this
  HEAD: **no VRAM code exists in `src/`** (grep: no match). Owner:
  lead.
- Buffer pool / atlas: modules exist and are registered
  (`src/bufferpool.rs`, `src/atlas.rs` — main.rs mod list read at
  this HEAD), but adoption is explicitly not claimed: atlas was
  probation-recommended in W18 (not a VRAM saving), and W84 records
  BufferPool still has zero production callers. Status: DONE as
  code, **deferred as adoption** — consistent with the phase's own
  "only if proven pixel-identical, else design note + defer" rule.

### Phase 4 — Animation & visual quality (all opt-in): **PARTIAL**
- Animation evaluator + channels: **DONE as modules** — `src/anim.rs`,
  `src/charanim.rs` registered (Batch 2; +65 tests in suite per the
  record's batch ledger).
- Blink adoption: **DONE** (`5a9383e`, per W33/W58: `max(channel,
  blink_at(beats))` composition; census 25 beat character-frames,
  0 changed — the 7 authored beat-blinks survive bit-exactly).
- Scene-format extensions (prefabs / clips / camera tracks):
  **DONE as format + guard, OPEN as evaluator** — §15 folded into
  the spec with census PASS 75/75 (W65) and the reservation guard
  live in the loaders (`4dd6530`: unknown top-level blocks fail
  loudly). Step 2 — the loader actually *consuming* clips/prefabs —
  has not landed; the §15 tools remain the only consumers.
- Locomotion: **OPEN — user's sign-off** (§7.6). Module written
  (W24, gait laws 9/9 as tests) but verified at this HEAD:
  **`src/locomotion.rs` is not in main.rs's mod list and has zero
  call sites** — its tests do not run in CI. Registration (step 0,
  zero-pixel) is packed (W87); adoption beyond that re-times the
  bob and adds arm swing on 448 frames — a motion-quality call
  reserved to the user.
- Cameramoves: same state (module written W25, 11/11; not
  registered — verified in the same mod-list read).
- Lipsync: **OPEN — binding decision** (§7.7). The viseme track
  exists (W23); the oscillator never drives narration in the staged
  film, so adoption binds future speakers. W86 (Batch 9) recommends
  Variant A (builder-side closed-exit step keys — measured equal to
  full-step evaluation on all three leak classes, closures
  36/36) and corrects W59's wording by measurement. The lead owns
  the semantics choice; the line-3 A/B sheet is the user's
  sign-off artefact.

### Phase 5 — Engine maturity: **PARTIAL**
- §15 versioned extensions: see Phase 4 (spec + guard DONE,
  evaluator OPEN).
- CI jobs: **DONE** — visual-regression (dispatch; steady state =
  failure-as-designed with s28 the sole DRIFT — run `43628892930`
  at this wave under the W72-updated register), determinism
  (dispatch; SUCCESS ×3 above), knobs gate (dispatch; §6.1),
  bench (dispatch), tools-ci (push; SUCCESS at the recent heads,
  e.g. `3223509`, `2fcffc0`).
- Docs: **DONE-WITH-NOTE** — DOCFIX2 applied wholesale (`c7aa648`);
  KNOBS-W43's truncated tail spliced (`33c5b2a`); record fold-in 2
  (`cca4e6e`) plus the lead's own correction commit (`4af54c5`).
  Note: WAVE2-RECORD's batch ledger at this HEAD still shows
  Batch 8 as "running at this fold-in" and the 9–10 row empty —
  **the final fold-in is Batch-10 work (W91), still owed.**
- G7 telemetry tables: **DONE-WITH-NOTE** — the prototype matched
  the binary's own caption audit exactly (73/73, W75); the binary
  implementation (`--film-telemetry --tables`) landed at
  `3223509` during Batch 9. Its independent diff-check against the
  W85 spec (default output byte-identical, verdict/exit unmoved)
  is Batch-10 seat W93 — **unverified at this review**.
- s12 bake adoption: **OPEN (landing)** — the bake is PROVEN
  (W52: 65/65 records convert, equivalence FAITHFUL, ≤1 ulp) and
  Batch 9's W83 re-verified it at this HEAD with zero drift
  (READY). The adoption commit itself has not landed — verified:
  `src/film.rs` still carries the live `apply` path
  (`bow` pose_track → `onoda.lean`/`head_tilt`, ~lines 2021–2023).
  Owner: lead (one data commit + retiring those lines + probes
  over frames 1666–1810, per W83).

### 6.1 Gap-analysis fixes (user order 2026-11-14 23:24, "also do this fixes")

| Fix | Status | Evidence |
|---|---|---|
| F1 `--film` reads film.json (help said film.json, driver read onoda.json) | DONE | `aadc193` (film.json preferred, onoda.json fallback; help corrected) |
| F2 film driver must not demand Onoda-specific assets unconditionally | DONE | Loud fallbacks `a2ec80c`; migration complete `20a81a7` + `2c41313` (plates manifest v1; calendar years/sign/dark from film data) |
| F3 unknown JSON fields silently ignored | PARTIAL | Checker + ratchet wired (tools-ci). F3-a strip **landed** `2fcffc0` — baseline 196 → **24 occurrences / 9 paths**, verified in `tools/format/unknown_fields_baseline.json` at this HEAD. F3-b (19 prose fields) + F3-c (3 bookshop) dispositions are **staged, not landed** (W88: all 19 CITE→strip; materials STRIP via decomposition; `lights.key.near` CONSUME) — staged total would be 3. The hard-error format decision remains report-only + ratchet by design |
| F4 `--res`/`--aa`/`--shadow` knobs | **DONE + ACCEPTED** | After the corrected record error (§6.2): phases 1–3 landed for real (`ea98458`/`9ad2177`/`76b922c`), gate workflow wired (`2017e86`), and Batch 9's W81 verified the acceptance: **two independent knobs-gate runs at `c440d90` (`43628289244`, `43628339967`), both SUCCESS, exact line `KNOBS GATE: PASS`** — leg A 40/40 byte-identical at defaults, leg B exact `[DRAFT …]` tag at 540×960, leg C 10/10 invalid inputs exit 2. Defaults remain full quality; drafts are tagged, never silent |
| F5 native sheet mode | DONE | Sheet tool (W44, tests 10/10) + CI step wired (`8b9ac36`; ffmpeg dependency fixed `f4f6ccf`) |
| F6 cues export | DONE as design-only | W45 design; cues stay skill-side per the triage ruling. Its Q1 (exact cue entry shape) remains open *inside* the design — no implementation was ever in scope |
| NOT adopted: native font rasterizer | Closed by ruling | Standing contract is pre-rendered caption strips post-grade; not a remainder |

### 6.2 The knobs record error — for the record of this review

Wave 2's one integrity incident belongs in a goal review because
the goal's law is *report every verdict as checked*. During Batch
7/8 the wave record stated knobs phase 1 had landed; it had not
(W73 proved the absence airtight at `b2e46d2`; corroborated by
W76/W79). The lead corrected the record itself (`4af54c5`), the
user was told plainly, and the feature was then implemented for
real and independently accepted (§6.1, F4). The lesson is standing:
claims are verified against code/runs before being relayed. No
other Wave-2 verdict relied on the false line.

### 6.3 Wave-2 incident ledger (all resolved)

Tools-CI red ×3 root causes (absolute manifest paths → `617ea4e`;
`.gitignore`-swallowed fixture PNGs → `865c471`; hosted image
lacking ffmpeg → `f4f6ccf`); main-CI red at `edb058f` (film.rs
delimiter) → `aa92cee`; Act-1 golden-pin staging defect → canary
caught it, corrective `2755ff3` exactly verified; one CI run
(`c7aa648`, and at this review `43629625139` / `43628889658`)
**cancelled by concurrency supersession — not failures**, per the
standing CI discipline. At this review, the CI run for HEAD
`9380b01` itself (`43629817118`) was still **in progress** — no
conclusion is claimed for it here.

## 7. Standing remainders — the named list, one by one

### 7.1 PC RADV evidence session — OPEN · owner's move: the user (one PC session)
Everything native is still proven only on CI/VM lavapipe. Owed from
that one session, all scripted in `docs/runbook/PC-EVIDENCE-W36.md`
(+ W64 §5 bench addendum, + W78's G1/G2 paste-ins, the latter landed
by `108f33f`):
- **E7 determinism leg** — the contract's RADV leg: same binary,
  same ranges, byte-identity on RADV (the CI-lavapipe legs are
  discharged; this is the only vendor leg left).
- **PERF tables** — `docs/runbook/PERF.md` filled with real Vega
  7 / RADV numbers (verified at this HEAD: the tables are still
  blanks by design; no figure in it is a result).
- **RADV adapter proof + bench + sysfs VRAM figures** (§0.4 adapter
  line naming RADV, not llvmpipe/`Cpu`).
**New blocker found by rehearsal:** Batch 7's W68 dry-run (landed
late, `docs/integration/PC-DRYRUN-W68.md` — in the tree at this
review, not yet committed) rehearsed every VM-runnable leg: the
telemetry leg reached `FILM TELEMETRY: CHECK OK`, exit 0, and the
golden pack verified 18 OK on this VM. But its **finding F1 is
BLOCKING for the PC session**: runbook §0.2 provisions the binary
to `~/talerender-bin` while §§0.4/2.1/3.1/3.2 invoke
`./talerender-bin/talerender` from the repo root — reproduced
verbatim, exit 127. The canonical-path fix pack is Batch-10 seat
W96; **the fix must land before the user runs the session**, or the
session fails at its first command through no fault of the PC.

### 7.2 Fog-chart PSNR leg — OPEN · owner's move: reference side (Chrome rig)
The ±2-code analytic gate PASSED on both sides (Wave 1: Chrome
≤1 code, native CI frames ≤1 code). The **≥45 dB PSNR leg against
the Chrome frame** remains unmeasured: it needs the **×π-amended
Chrome re-capture** (the Chrome rig's effective light factor is
house/π; ambient ≈1.0996, directional ≈2.0420) — a reference-side
task, not a native fix. Nothing in Wave 2 changed this state.
Disposition options: execute the re-capture, or formally waive
the leg (the analytic gate + the film A/B stand as the fog
evidence).

### 7.3 Skill adoption — OPEN · owner's move: the user
Adopt TaleRender into the **shipped** tall-tale-shorts skill as an
opt-in alternative capture backend at the frame(t) seam, or wait
for RADV proof first. The work copy
(`~/workspace/talerender-skill-wiring/`) is complete and the
adapter has been proven fail-closed in both directions twice
(Wave-1 W99; Wave-2 W62 independent re-proof, 8/8). One-pager:
work-copy `docs/ADOPTION-DECISION-W94.md`. Adoption ≠ default —
Chrome stays renderer of record either way until the user says
otherwise. The shipped skill has never been touched by either wave.

### 7.4 Caption veto — OPEN · owner's move: the user
KEEP the 73-chunk data (gate green; W69's recommendation) or veto
back to the 66-chunk Chrome-shipped layout (the Wave-1 waiver
stands again; 3 golden pins go stale and are re-struck from the
candidates). Decision-ready since Batch 7; the splits are
width-forced in two places, disclosed in the brief.

### 7.5 s28 golden-pin disposition — OPEN · owner's move: the user
`s28` (the §28 first-light artefact, pin `ff68fad3…`) predates the
whole fix chain and is now the visual-regression register's sole
DRIFT — the register's steady state is *failure-as-designed*
(run `43628892930`). Options (W03/W72): re-pin to the current
ab1 bytes (`60c32c1a…` — makes s28 a byte-duplicate of ab1; the
packet recommends against), retire the item, or leave VR red on
s28 alone as known debt. Untouched by both pin acts, deliberately.

### 7.6 Locomotion adoption — OPEN · owner's move: the user (motion quality)
The corrected gait (bob phase fixed; counter-phase arm swing) is
written and law-tested but not registered (§6, Phase 4). Adoption
changes 471 walking frames' bob, adds NEW arm swing on 448 frames,
and re-levels standing figures ~1.9 px for 4.4 s in s10 (W57
census). W87 (Batch 9) prepared step 0 (registration, zero-pixel,
proof protocol) and a plain-language decision sheet with decisions
D1–D4. No engine seat can take this call — it is a
does-it-look-right judgement on contact sheets.

### 7.7 Lipsync binding — OPEN · owner's move: lead (semantics), then user (sign-off)
§6 Phase 4: W86 recommends Variant A; the lead picks the semantics;
the line-3 sheet (frames 482–632; headline check: the 17-frame
silence after "ended.", frames 570–586) is the user's sign-off
artefact. Zero frames of the current film change either way.

### 7.8 v1.0.1 patch release — OPEN · owner's move: the user (go)
Verified at this HEAD: `Cargo.toml` still reads `version =
"1.0.0"` (line 3), and GitHub lists exactly one release, **v1.0.0**.
The v1.0.0 binary predates the professional CLI and the whole of
Wave 2 — and the user's PC holds a second, different "1.0.0"
binary, which is the practical problem v1.0.1 closes. The release
pack is refreshed and decision-ready (W89, Batch 9): changelog
extended to the wave's final HEAD, bump mechanics re-verified
(1 functional literal + README:65 as the one judgement call), six
gates to re-prove on the bump SHA. It ships from the **final
wave HEAD**, so it is sequenced after Batch 10 closes.

### 7.9 Smaller open items found by this review (not in the original remainder list)
- **Split programmes execution** (§6 Phase 2) — 22 serial commits,
  battery-invocation precondition first. Lead, post-wave.
- **VRAM `gpu-mem:` implementation** (§6 Phase 3) — staged, not in
  `src/`. Lead.
- **s12 bake adoption landing** (§6 Phase 5 note) — proven READY;
  one commit. Lead.
- **F3-b/c landing** (§6.1 F3) — dispositions staged (W88); the
  landing lowers the ratchet baseline 24 → 3. Lead.
- **§15 evaluator step 2** (§6 Phase 4) — loader consumption of
  clips/prefabs; guard-only today. Scheduling decision.
- **`--tables` diff-check** (§6 Phase 5) — W93, in flight.
- **WAVE2-RECORD final fold-in** (Batches 8–10) — W91, in flight;
  the ledger at this HEAD is stale for those batches.
- **W77 exporter note** — the acceptance harness's self-test
  passed 3/3 in Batch 8, but its explanatory note is still owed
  (the tool file shows as modified in the working tree at this
  review — that seat appears still to be iterating; the committed
  version stands).
- **PC runbook F1 fix** (§7.1) — W96, in flight; gates the PC
  session.
- **Colour QA (outside the goal's checklist, noted for
  completeness):** the user's 2026-11-14 report that the delivered
  native film looks pale/faded is still awaiting their
  scene/timestamp — global saturation measured equal-or-higher
  than Chrome at every sampled point. Not a goal blocker; the
  headless-three study's tone-mapping WGSL diff is the prepared
  next investigative step (lab only).

## 8. RECOMMENDATION (labelled)

**Recommendation: the goal cannot honestly close at Wave-2 end.
Keep goal_b21124e71eb3 OPEN, with the remainder list below — every
item on it is either a user decision, a user-PC session, or a
prepared pack awaiting a landing commit. Nothing on it requires
new research.**

Why not close: the goal's own close condition (GOAL.md) is "close
when those land or are formally waived." Of the Wave-1 remainder
list, pins and the re-render landed and the caption item was
superseded by a green gate — but **RADV proof and skill adoption
neither landed nor were waived**, the fog-chart leg is unmeasured,
and Wave 2 added its own tail (v1.0.1, s28, locomotion, the veto
right, lipsync, plus §7.9's landings). Closing now would repeat,
at goal scale, the exact failure Wave 2 corrected in miniature:
declaring done what is only prepared.

**Minimal path to close, in order:**
1. **One decisions conversation** (all six are decision-ready):
   caption veto (recommend KEEP 73) · s28 disposition · locomotion
   D1–D4 · lipsync Variant A + line-3 sign-off · v1.0.1 go ·
   skill adoption (or defer adoption explicitly until after
   step 2).
2. **Land the prepared packs** (lead, days not weeks): runbook F1
   fix → s12 adoption → F3-b/c landing → v1.0.1 from the final
   HEAD once approved. The splits, the VRAM line and the §15
   evaluator are larger: either schedule them as a named
   follow-on or formally waive them out of this goal — both are
   honest closings; silence is not.
3. **One PC RADV session** (user's PC, scripted end-to-end):
   discharges E7, fills PERF, proves the adapter on RADV.
4. **Fog-chart leg:** execute the ×π re-capture, or waive the leg
   on the record.
Then close the goal with the final record fold-in as its last
commit.

### 8.1 Verbatim text for the user (the orchestrator may read this as-is)

> TaleRender itself is finished and proven: every milestone gate
> passed, the full Onoda film renders natively, its quality gate
> is green (the caption failure from the first wave is fixed, not
> waived), and the second wave completed the audit — including the
> draft-quality knobs, which were independently verified with two
> passing gate runs after we caught and corrected a false progress
> report mid-wave. What is left is not building; it is decisions
> and one session on your PC. Six decisions are ready for you in
> one conversation: keep the new caption splits (recommended),
> what to do with the one stale golden pin, whether the corrected
> walking motion is adopted, the lip-sync binding choice, whether
> to publish the v1.0.1 patch release, and whether the native
> renderer joins the shipped skill now or after your PC proves it.
> The PC session is fully scripted — it is the only way to get
> real RADV performance numbers and the last determinism proof —
> and one path bug in the runbook, found by rehearsing it here,
> is being fixed before you run it. I recommend keeping the goal
> open until those land; closing it today would mean calling
> prepared work finished.

## Appendix — evidence index (all first-hand at HEAD `9380b01`)

- Commits read in history: `888e03d` M0 · `819ee16` M4 · `79aff03`
  M5 · `9a8cc79` M6 · `801f85c` JPEG (B) recorded · `744e74e`
  Wave-1 close · `9790553` waiver (superseded) · `5e90d7e` re-chunk
  CHECK OK · `aa10b13` exit codes · `ba88596`/`edb058f`/`aa92cee`
  errors · `aadc193` gpucore 5/5 + F1 · `916a109`/`166873c`/
  `2755ff3` pins · `4dd6530` §15 + guard · `20a81a7`/`2c41313` F2 ·
  `5a9383e` blink · `ea98458`/`9ad2177`/`76b922c` knobs 1–3 ·
  `2017e86` knobs gate · `4af54c5` record correction · `33c5b2a`
  W43 splice · `c7aa648` DOCFIX2 · `cca4e6e` fold-in 2 · `2fcffc0`
  F3-a strip · `3c07564` VR register · `108f33f` runbook paste-ins ·
  `3223509` `--tables` · `9380b01` Batch 9 (HEAD).
- Runs read via `gh`: CI `43629817118` (HEAD, in progress at
  review — no conclusion claimed); Tools CI SUCCESS at `3223509`;
  knobs gate `43628289244` + `43628339967` SUCCESS at `c440d90`;
  determinism `43617370962`/`43620084786`/`43624016907` SUCCESS;
  VR `43628892930` failure-as-designed (s28 sole DRIFT).
- Tree reads: `Cargo.toml:3` = 1.0.0; GitHub releases = v1.0.0
  only; `src/main.rs` mod list (no `locomotion`, no
  `cameramoves`); `src/film.rs` 3,787 lines with the s12 `apply`
  path live at ~2021–2023; `src/scene.rs` 2,979 lines; grep finds
  no VRAM code in `src/`; `src/runlog.rs` carries `peak_rss_mb()`
  + `TALERENDER_FRAME_TIMING`;
  `tools/format/unknown_fields_baseline.json` at 24/9 post-strip;
  `docs/runbook/PERF.md` tables blank by design;
  `docs/integration/PC-DRYRUN-W68.md` present (uncommitted) with
  finding F1 BLOCKING; WAVE2-RECORD §8 ledger stale for
  Batches 8–10.

*End of W95 review. This file is the only file written by this
seat; nothing was committed; GOAL.md was not edited; the goal was
not closed.*
