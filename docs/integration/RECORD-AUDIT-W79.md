# RECORD AUDIT — W79 (Wave 2, Batch 8)

**Seat:** W79 — WAVE2-RECORD completeness audit + wave-close skeleton.
**Audited at HEAD:** `b2e46d2` (Batch 7 central commit).
**Object of audit:** `docs/integration/WAVE2-RECORD.md` (389 lines;
last fold-in `b6ee9fb`, plus `a2ec80c`-era intentions — see below).
**Method:** the checklist is built from `git log 6f00400^..HEAD`
(**37 commits**) and nothing else. Every commit is marked
**PRESENT** (the record carries it), **ABSENT** (no trace in the
record — verified by grepping the record for each post-fold-in
commit hash: `a2ec80c`, `4dd6530`, `2755ff3`, `8b9ac36`, `5a9383e`,
`20a81a7`, `2c41313`, `f4f6ccf`, `3a3a427`, `b2e46d2` each return
**0 hits**), or **STALE** (present but superseded; the record's
own line is quoted for every STALE mark). This audit does not use
conversation history as evidence; where a seat note is cited, it
is a committed file in the tree.

## Headline

- **27 / 37 commits PRESENT, 10 / 37 ABSENT.** Every absent commit
  is at or after `3a3a427` — the record's fold-in (`b6ee9fb`)
  predates the Batch-6 central commit itself, so **the entire
  post-Batch-5 history of the wave is missing from its own
  ledger**: Batch 6's close, Batch 7 entire, the F2 migration,
  the §15 fold-in + reservation guard, the Act-1 corrective,
  blink adoption, tools-ci pack 2, and the ffmpeg incident/fix.
- **3 of the 27 PRESENT entries carry STALE elements** (Act-1
  row, Act-2 provenance cite, plus record-level stale lines in
  §6/§8/§10 itemised in §2 of this audit).
- Of the wave's **8 corrections/retractions** (§3), exactly
  **one** — the adapter re-proof retraction — is recorded in the
  ledger. The other seven live only in commit messages and seat
  notes; a reader of the record alone is misinformed on the
  Act-1 pins (the record asserts as fact a composition the
  corrective commit discloses was not what was pinned).

---

## 1. Commit checklist (from `git log 6f00400^..HEAD`)

### 1a. Batch central commits

| Commit | Subject (short) | Record status |
|---|---|---|
| `3a36f1f` | Batch 1 (gate PASS) | **PRESENT** — §7 "B1 push" row; §8 row 1 |
| `1a62161` | Batch 2 (gate PASS) | **PRESENT** — §7 "B2 push" row; §8 row 2 |
| `ff52b3a` | Batch 3 (gate PASS-WITH-NOTES) | **PRESENT** — §7 "B3 push" row; §8 row 3 |
| `f6616aa` | Batch 4 (gate PASS-WITH-NOTES) | **PRESENT** — §7 "B4 push" row; §8 row 4 |
| `d3def63` | Batch 5 (gate PASS-WITH-NOTES) | **PRESENT** — §7 "B5 push" row; §8 row 5 |
| `3a3a427` | Batch 6 (gate PASS-WITH-NOTES) | **ABSENT** — §8 row 6 still reads "running at this fold-in" (see STALE S-1); no §7 verdict row for the push |
| `b2e46d2` | Batch 7 (gate PASS-WITH-NOTES; W68 outstanding) | **ABSENT** — the string "Batch 7" does not occur in the record; §8 still shows the placeholder row "7–10 \| — \| per orchestrator boards" |

### 1b. Lead landings

| Commit | Subject (short) | Record status |
|---|---|---|
| `6f00400` | Record created (mission, laws, stale register) | **PRESENT** — it is the record |
| `aa10b13` | Telemetry exit codes | **PRESENT** — §7 row |
| `5e90d7e` | Caption driver leg, 73 chunks, CHECK OK | **PRESENT** — §5 "CLOSED BY DATA"; §7 headline row |
| `ba88596` | Errors step 1 + `mod errors` | **PRESENT** — §7 row |
| `2328e6f` | Module registration (94 tests) | **PRESENT** — §7 row (incl. the 174/175 failure) |
| `38930fa` | gpucore strip-padding test fix | **PRESENT** — §7 registration row "FAILURE → fixed `38930fa`"; wiring row "175/175 incl. the fixed gpucore test" |
| `86f0c81` | PaintBudget + RunLog wiring | **PRESENT** — §7 row |
| `fbf37c0` | render.yml `paint_budget_mb` input | **PRESENT** — watch-items flag ("render.yml lead edit… flagged for the orchestrator's ratification") |
| `11a7a2e` | Record fold-in (verdicts, B1/B2 ledger) | **PRESENT** — embodied in the record |
| `df89ad6` | gpucore site 1/5 (dof) | **PRESENT** — §7 row |
| `c420d72` | gpucore site 2/5 (scenefile) | **PRESENT** — §7 row |
| `9f497c7` | gpucore site 3/5 (ab1) | **PRESENT** — §7 row (but see STALE S-4 on the run cite) |
| `dc0e555` | gpucore site 4/5 (main) | **PRESENT** — §7 row |
| `aadc193` | gpucore site 5/5 (film) + F1 | **PRESENT** — §7 row ("migration COMPLETE (5/5)… finding B CLOSED"); §10 F1 "LANDED" |
| `630d82e` | tools-ci wiring per W39 | **PRESENT** — §7 row |
| `edb058f` | Errors step 2 | **PRESENT** — §7 row (incl. its own CI failure, next row) |
| `916a109` | GOLDEN ACT 1 (14 film pins) | **PRESENT but STALE** — §7 row asserts the intended composition; the committed pins were not that composition (STALE S-3) |
| `166873c` | GOLDEN ACT 2 (ab1 re-pin) | **PRESENT but STALE cite** — §7 rows 239/248 tie its render run to `9f497c7`; the run's head is `dc0e555` (STALE S-4) |
| `b6ee9fb` | Record fold-in (batches 3–6, acts, incidents) | **PRESENT** — embodied in the record; it is the last fold-in |
| `a2ec80c` | F2 first landing (loud fallbacks) | **ABSENT** — §10 F2 still reads "landing scoped" (STALE S-6) |
| `4dd6530` | §15 fold-in + reservation guard (adoption step 1) | **ABSENT** — §9/§10 still speak of the fold-in as future ("Recorded in §15's fold-in"; "resolve at the §15 fold-in") |
| `2755ff3` | GOLDEN ACT 1 CORRECTIVE re-pin | **ABSENT** — no corrective row; §7's Act-1 row uncorrected (STALE S-3) |
| `8b9ac36` | tools-ci pack 2 (W55) + §15 conformance steps | **ABSENT** — §8 row 6 still lists it as "queued" (STALE S-1) |
| `5a9383e` | Blink adoption (W58 pack) | **ABSENT** — §8 row 6 still lists blink as "queued" (STALE S-1) |
| `20a81a7` | F2 migration Phase D (data declares calendar/sign) | **ABSENT** |
| `2c41313` | F2 migration Phases L+T (manifest loader + tools) | **ABSENT** — with `a2ec80c` + `20a81a7`, F2 is COMPLETE in git; §10 says "landing scoped" (STALE S-6) |

### 1c. Fixes / incident commits

| Commit | Subject (short) | Record status |
|---|---|---|
| `617ea4e` | URGENT FIX: tools-ci absolute-path classification | **PRESENT** — §7 "Incident" row (full root cause + fix + disclosure) |
| `865c471` | Orchestrator fix: gitignored fixture PNGs (55 committed) | **PRESENT** — §7 "Incident 2" row |
| `aa92cee` | Fix errors-step-2 delimiter / Ok-wrap miss | **PRESENT** — §7 row ("CI + Tools CI both SUCCESS") |
| `f4f6ccf` | tools-ci: install ffmpeg before the sheet suite | **ABSENT** — incident 3 (sheet suite red at three prior heads solely because the hosted image lacks ffmpeg; W55's one false assumption) appears nowhere in the record; the word "ffmpeg" does not occur in it |

---

## 2. STALE register — record lines superseded at HEAD

Each entry quotes the record verbatim (line numbers are the
record's at audit time).

- **S-1 — §8, Batch 6 row (line 283):** "running at this
  fold-in" and "Lead landings queued: tools-ci pack 2 (W55),
  knobs (W54 + §10 ruling), F2 staging (W53), blink (W58), §15
  fold-in with the W51 checker".
  **Superseded:** Batch 6 closed and was committed at `3a3a427`
  (gate PASS-WITH-NOTES). Of the five queued landings, four have
  landed — tools-ci pack 2 (`8b9ac36`), blink (`5a9383e`), §15
  fold-in (`4dd6530`), F2 (staging → full migration,
  `a2ec80c`/`20a81a7`/`2c41313`) — and one has **not**: grep of
  `src/main.rs`/`src/film.rs` at HEAD finds no `--res`, `--aa`,
  `--shadow`, or DRAFT-tag code; **no knobs implementation
  commit exists anywhere in the wave range**. The row must not
  survive into the close-out in this form: it simultaneously
  understates four landings and hides the one outstanding item.
- **S-2 — §8, Batch 1 row (line 278):** "pins pending
  (post-re-render sequencing, GOLDEN-RESTAGE-W03 §4.3)".
  **Superseded:** both acts struck (`916a109`, `166873c`) and
  Act 1 corrected (`2755ff3`); pack verifies 18 OK / 0 STALE /
  0 MISSING. Only s28 remains unpinned, by design.
- **S-3 — §7, GOLDEN ACT 1 row (line 247):** "14 film
  first-pins struck — 11 mids from the `f021991` candidates
  (planner-proven pixel-identical outside the split windows)".
  **Superseded by `2755ff3`'s own disclosure:** the first Act-1
  commit in fact pinned the **staged `381ff47`-era bytes** for
  the 11 out-of-window mids (W03's copy step was missed); the
  visual-regression canary caught it (film-s08b DRIFT 24.85 dB,
  rendered `fb4e2ba4` = the f021991 candidate, pin was
  `7764821c`); the corrective re-pinned exactly 10 film lines
  (s10's bytes identical — the control case). The row as written
  describes the *intended* composition as the *committed* one.
  This is the record's single most misleading line at HEAD.
- **S-4 — §7, ab1 rows (lines 239, 248):** "ab1 fresh render
  at `9f497c7` (run 43621168728)" / Act 2 "the FRESH head frame
  (run 43621168728)".
  **Superseded:** GOLDEN-VERIFY-W63 verified via `gh` that run
  43621168728's head is **`dc0e555`**, not `9f497c7` (off by one
  commit in the gpucore series; substance stands — GATE2-W70
  gate-confirmed). The record is also internally inconsistent:
  its own gpucore site-4 row ties the same run id to `dc0e555`
  ("`dc0e555` CI SUCCESS (43621168728 list)").
- **S-5 — §6, exit-code contract (line ~209):** "RESOLVED
  (W02 + lead re-proof): the work-copy adapter implements the
  dual convention (accepts FAILED at exit 0 or 1, OK only at 0,
  fails closed on any disagreement or missing line;
  stub-proven 10/10 by W02 and **re-proven by the lead with
  the REAL binary at the exit-code head — FAILED direction
  §7**; OK/old-binary directions appended there as they land)."
  **Superseded in part:** §7's own CORRECTED row retracts the
  lead real-binary FAILED-direction proof (the run was
  SIGKILLed; the fixture was strip-mismatched) — but §6's
  stronger sentence was never amended, so the record's contract
  section still asserts a proof its verdicts section withdrew.
  The current truth is stronger than either text: Batch 7's W62
  independently re-proved the adapter **8/8 cells** across both
  exit-code generations (RE-PROOF PASS, `b2e46d2`) — also absent
  from the record.
- **S-6 — §10, F2 disposition (line 336):** "F2 — AUDITED
  (W41), landing scoped".
  **Superseded:** F2 landed in three commits — first landing
  `a2ec80c` (silent fallbacks made loud), Phase D `20a81a7`
  (film data declares `calendarYears`/`calendarDark`/`sign`),
  Phases L+T `2c41313` (plates manifest v1 loader, declared
  year count with the zero-years guard, tools re-pointed;
  check_assets real driver 86 refs 6/6). F2 is **complete**;
  the disposition must be re-marked LANDED at close.
- **S-7 — §10, preamble (lines 327–329):** "Stale claims in
  the rev-2 analysis are NOT re-fixed (source in VCS,
  scene-format.md exists, telemetry exits 1/3 since `aa10b13`,
  CLI panic-free exit 2 since `d7bebdd`, **errors mid-flight**)
  — **W38's doc pack + the v1.0.1 release close the
  stale-binary side**."
  **Superseded twice over:** (a) errors is not mid-flight —
  step 2 landed at `edb058f` (and its fix at `aa92cee`); (b)
  DOCFIX2-W67 verified first-hand that **nothing from W38 was
  ever applied** — `git log df89ad6..HEAD` over all seven W38
  target files returns zero commits. The pack does not "close"
  anything until the lead applies DOCFIX2-W67 (Batch 7's
  refresh of it).
- **S-8 — §10, F3-c disposition (line 355):** "resolve at the
  §15 fold-in — either the loader consumes it or the data
  stops carrying it, never both".
  **The promised either/or did not occur.** The fold-in landed
  (`4dd6530`, reservation guard only, per its message). At
  HEAD: grep finds no consumption of `nodes[].materials` in
  `src/scenefile.rs`/`src/scene.rs` (the only "materials" hit
  is a comment in scene.rs:2645), and
  `scenes/sets/bookshop.scene.json` still carries the key. The
  disposition is now overdue, not merely pending.

### Not stale (checked, still accurate)

- §5 caption text + "CLOSED BY DATA" — the waiver is framed as
  the prior disposition and the veto path is still live. ✓
- Watch item "PERF.md §1 dated correction owed" — still owed:
  no commit in the wave range touches `docs/runbook/PERF.md`. ✓
- Watch item "painter panic observed once, not reproduced" —
  no recurrence in git. ✓
- §9 Batch-2 adoption decisions — decisions, not status claims;
  their implementation states are tracked in §4 below. ✓

---

## 3. Corrections register — is each correction findable in the record?

The wave produced eight corrections/retractions. A future
reader of **the record alone** finds exactly one of them.

| # | Correction | Where the correction lives | In the record? |
|---|---|---|---|
| C-1 | Adapter real-binary re-proof PASS retracted (run SIGKILLed under contention; fixture strip-mismatched; corrected fixture rebuilt from `aa10b13`) | Record §7 CORRECTED row; `b6ee9fb` fold-in | **YES** — the model example; explicitly labelled CORRECTION. (Caveat: §6's text was not amended — STALE S-5; and the resolution, W62's 8/8 RE-PROOF PASS at `b2e46d2`, is not yet recorded.) |
| C-2 | Act-1 pins described superseded bytes (11 mids = staged `381ff47`-era bytes, not the f021991 candidates; canary-caught) | `2755ff3` commit message; GOLDEN-VERIFY-W63.md; GATE2-W70.md | **NO** — record's Act-1 row still asserts the intended composition (STALE S-3) |
| C-3 | W55's assumption that the hosted CI image ships ffmpeg (sheet suite rc=2 at three heads) | `f4f6ccf` commit message | **NO** — no sheet-suite or ffmpeg mention anywhere in the record |
| C-4 | Bench-window claim (bench.yml header + BENCH-W17: frames 0..100 include the S01→S02 transition) is false — S01 spans frames 0..178; the window is S01-only, transition cost unmeasured | BENCH-FIRST-W64.md anomaly A1 (`b2e46d2`) | **NO** — the record's only bench mention is the unfulfilled "verdicts to be appended" (§4 of this audit) |
| C-5 | Act-2 provenance cite: run 43621168728 head is `dc0e555`, not `9f497c7` | GOLDEN-VERIFY-W63.md; GATE2-W70.md | **NO** — record lines 239/248 uncorrected and internally inconsistent (STALE S-4) |
| C-6 | KNOBS-W43.md is truncated on disk at line 264 (its §4.3/§5/§6 never landed); tail reconstructed by W54, restoration provenance-tagged by W61 with 6 items UNRECOVERABLE | `3a3a427` + `b2e46d2` commit messages; KNOBS-PATCH-W54.md; KNOBS-W43-TAIL-W61.md | **NO** — §10 F4 says only "SPEC LANDED (W43)" |
| C-7 | FOLDIN-W65.md §4 itself transport-truncated (second occurrence of the class) | `b2e46d2` commit message; GATE2-W70.md | **NO** — record predates Batch 7 entirely |
| C-8 | Split blueprints' G-TEL gate texts are stale (they quote the pre-re-chunk `CHECK FAILED (… 7 caption failures)` + exit 1; standing truth is `CHECK OK`, exit 0 — an unamended gate would fail a healthy tree) | SPLIT-READY-W66.md | **NO** |

**Pattern worth naming at close:** every correction that the
*lead* authored into a fold-in (C-1) reached the record; every
correction authored by a seat or disclosed in a commit message
(C-2…C-8) did not — because no fold-in has run since `b6ee9fb`.
The fix is procedural, not editorial: the close-out fold-in
(§5 skeleton) must sweep commit messages and gate notes, not
only seat headlines.

---

## 4. Verdicts the record promised to append — and never did

§10 closes with two explicit append-promises. Neither was kept;
the underlying verdicts are absent from §7.

1. **"First dispatches at `aa92cee` (orchestrator): bench
   43623743480 + visual-regression 43623748951 — verdicts to be
   appended to §7 when checked"** (lines 375–377). Neither
   verdict is in §7. Current truth (from `b2e46d2`'s message and
   the Batch-7 notes): bench run **SUCCESS** (CI lavapipe,
   1.85 s/frame; cache-24 leg −147 MiB peak RSS at no in-window
   time cost; anomalies A1/A2 per C-4); the pre-pin VR run
   failed on the expected UNPINNED pattern; the **post-pin** VR
   runs — `43624398371` (exposed the Act-1 defect) and
   `43625139818` (post-corrective steady state: **s28 sole
   DRIFT**) — are likewise unrecorded.
2. **"Errors step 2 gate dispatches (lead, at the post-Act-2
   head): dof-chart 43624002645, fx scene-file 43624007505,
   film probe 795–815 43624011865, determinism 43624016907 —
   verdicts appended when checked; plus the no-adapter failure
   proof"** (lines ~380–386). No §7 rows exist for any of the
   four runs or for the no-adapter exit-3 proof — the one
   behaviour errors step 2 exists to change has no verdict in
   the ledger either way.

---

## 5. Open remainders — carried in the record, or not

| Remainder | In record? | State at HEAD (per git) |
|---|---|---|
| RADV leg (determinism E7, bench, PERF tables) | ✓ §3 S4, §7 E6 row | Still owed on the user's PC; nothing claimed. W68 PC dry-run still running at `b2e46d2` (folds in on landing) — unrecorded |
| Caption veto (user, at wave close) | ✓ §5 | Live; W69 brief decision-ready (veto would stale 3 Act-1 pins) — brief unrecorded |
| s28 disposition (user) | ✓ §4 ("packet recommends against re-pinning") | Still open; latest status — sole DRIFT in post-corrective VR — unrecorded |
| v1.0.1 release (user to confirm) | ✓ §8 row 5, §10 | Still NOT releasable until record + gates complete on the bump SHA |
| Skill adoption (user) | ✓ §4 Phase 5 | Open; adapter side now re-proven (W62, unrecorded) |
| Knobs implementation (F4) | △ §8 "queued", §10 pointer | **Not landed** — no implementation commit in range; §15-guard and F2 consumed the fixes-pass capacity instead (STALE S-1) |
| Film/scene split programmes | △ §4 scope, §8 row 3 "noted SERIAL" | Blueprints only; W66 readiness re-check (state_key freeze byte-identical; G-TEL texts stale — C-8) unrecorded; 22 pure-move commits still owed |
| Locomotion adoption | △ §8 row 3 "held for A/B sign-off (Batch 6 W57 pack)" | W57 pack landed in Batch 6 (471 walking frames; arm swing NEW on 448; s10 standing re-level ~1.9 px) — adoption is now a user-eye decision; record doesn't say the pack landed |
| Lipsync adoption | ✗ (scope word only, §8 row 6) | W59 evaluation landed in Batch 6: adoption changes **zero frames** of this film; binding must name step-vs-linear semantics (lead decision) — unrecorded |
| Blink adoption | ✗ | **Landed** `5a9383e` — unrecorded (see §1b) |
| Charanim channels beyond blink; atlas probation; bufferpool under Readback; rendergraph | ✓ §9 decisions | Implementations open |
| VRAM telemetry implementation | ✗ (§8 row 6 scope word "design") | Design only (W56); implementation open |
| G7 telemetry tables (shot/caption/cast) | ✗ | Open, unqueued at record level (Batch 8 W75 in flight) |
| F3-a strip (174 dead-provenance occurrences) | ✓ §10 disposition | Not landed; Batch 8 W74 pack in flight |
| F3-b cite-don't-strip | ✓ §10 ruling | Checker-side landing not evidenced in git subjects; treat as open |
| F3-c materials resolution | ✓ §10 disposition | **Overdue** — neither branch occurred at the fold-in (STALE S-8) |
| Doc-fix application (W38 → DOCFIX2-W67) | △ §10 preamble (STALE S-7) | Nothing applied; DOCFIX2-W67 is the live queue |
| PERF.md §1 dated correction | ✓ watch item | Still owed (no PERF.md commit in range) |
| Painter panic (observed once) | ✓ watch item | Not reproduced |

---

## 6. Wave-close fold-in SKELETON (for the lead's final fold-in)

Section-by-section outline. Pre-drafted rows use **only**
verdicts stated in the batch/lead commit messages themselves —
no new claims. Bracketed items are placeholders the lead fills
from the run list at close; nothing below invents a CI verdict
that git does not state.

### 6.1 Amendments to existing text (apply before appending)

1. **§6 adapter bullet (STALE S-5):** strike "re-proven by the
   lead with the REAL binary at the exit-code head — FAILED
   direction §7" and replace with a pointer to §7's CORRECTED
   row + the W62 row (§6.2, R-11 below).
2. **§7 Act-1 row (STALE S-3):** append "(composition as
   intended; as committed, the 11 mids were the staged
   `381ff47`-era bytes — see CORRECTIVE row below)".
3. **§7 ab1 rows (STALE S-4):** correct "at `9f497c7` (run
   43621168728)" → "at `dc0e555` (run 43621168728; head
   corrected per GOLDEN-VERIFY-W63)".
4. **§8 Batch-1 row (STALE S-2):** "pins pending" → "pins
   struck `916a109`/`166873c`, Act 1 corrected `2755ff3`; s28
   unpinned by design".
5. **§10 F2 (STALE S-6):** re-mark "LANDED (`a2ec80c` +
   `20a81a7` + `2c41313`)".
6. **§10 preamble (STALE S-7):** "errors mid-flight" → "errors
   step 2 landed (`edb058f`)"; replace the W38 sentence with
   "DOCFIX2-W67 is the live doc-fix queue; nothing from W38 was
   applied (verified in DOCFIX2-W67)".
7. **§10 F3-c (STALE S-8):** record that the fold-in landed
   without the resolution and re-queue the either/or as an
   open remainder.
8. **§10 append-promises (§4 of this audit):** either append
   the verdicts (rows R-12/R-13 below) or strike the promises;
   do not carry them into the closed record unfulfilled.

### 6.2 New §7 rows (pre-drafted, paste-ready)

| When | What | Verdict |
|---|---|---|
| R-1 Lead | **F2 first landing** `a2ec80c` — the two silent fallbacks in `load_assets` are now loud: plates.json non-object → Err; missing/malformed plate origin → Err naming the plate; absent `captions[].strip` fails naming the caption index. Byte-neutral for valid data by construction (check_assets check 5 guards exactly this) | LANDED |
| R-2 B6 push | HEAD `3a3a427` (Batch 6, gate PASS-WITH-NOTES, GATE2-W60) — CI [append run ids + conclusions from the run list at close] | [to fill] |
| R-3 Lead | **§15 fold-in + reservation guard** `4dd6530` — W31 draft folded into scene-format.md verbatim; §15.10 adoption step 1: top-level `prefabs`/`clips` blocks, `clip`/`clipTime` on camera/lights/post/fog, and prefab/clip keys in walk_nodes hard-error with the §15.1 named messages in scenefile.rs; same top-level guard in film.rs `load_film_data`. No production data uses the extensions | LANDED |
| R-4 Lead | **GOLDEN ACT 1 — CORRECTIVE** `2755ff3` (disclosed): the first Act-1 commit pinned the STAGED `381ff47`-era bytes for the 11 out-of-window mids (W03's copy step missed). The visual-regression canary caught it on its first measured run: film-s08b DRIFT 24.85 dB, rendered `fb4e2ba4` == W03's f021991 candidate, pin was `7764821c` (cause per W03 §2: atmosphere inheritance `1fc6fab`). Corrective copies the 11 candidates (s10's bytes identical — its line does not move: the control case) and re-pins exactly 10 film lines; the 3 in-window mids keep their post-re-chunk bytes. Pack verifies 18 OK / 0 STALE / 0 MISSING | CORRECTED |
| R-5 Lead | **tools-ci pack 2 + §15 conformance steps** `8b9ac36` (TOOLSCI2-W55 verbatim + GATE2-W60 §9 item 2): strictness fixtures, the unknown-field RATCHET over the real corpus (Option A; growth-only failure vs the 196 baseline), the native sheet suite, check_format15 fixtures + production-corpus trivial pass. Verified locally pre-commit: ratchet PASS (196 = baseline, 15/15 paths), sheet 10/10, format15 corpus PASS, planted 9/9 | LANDED |
| R-6 Lead | **Blink adoption** `5a9383e` (W58 pack, on W33's recommendation) — `blink_of`'s scheduled channel becomes `charanim::blink_amount` under the UNTOUCHED `max(channel, blink_at(beats))` composition; all 21 bare Blinker sites converted; Blinker retained one release cycle as the motion.ts reference. Deliberate pixel change where the census promises it: 386 character-frames / 345 film frames (252 opened, 132 removed, 2 reshaped); census: 25 beat character-frames, 0 changed. Gate: byte-probes on the clean sub-ranges (195–203, 795–813, 1195–1204) | LANDED |
| R-7 Lead | **F2 migration Phase D** `20a81a7` — film/onoda.json gains `calendarYears`/`calendarDark`/`sign`; gen_film_data.py emits them so regeneration cannot drop them. Inert under the then-current loader; regeneration diff verified to be exactly the three keys | LANDED |
| R-8 Lead | **F2 migration Phases L+T** `2c41313` — FilmData demands the three keys (no Onoda defaults in a generic loader); plates.json becomes a manifest (name/file/origin required, duplicates rejected, legacy bare-object shape rejected BY NAME, no dual-read); `calendar_state` takes the declared year count with the zero-years guard. Tools re-pointed (check_assets real driver still 86 refs, 6/6; check_unknown_fields knows the three keys; rerender_plan resolves manifest entries). Zero frame impact by transcription — probes incl. a calendar range gate it | LANDED — F2 COMPLETE |
| R-9 Incident 3 | **Tools CI sheet-suite red at three heads** — every case rc=2 (make_sheet's ffmpeg-absent exit) on the hosted image, while the strictness ratchet, the migrated-tree asset steps and everything upstream passed. W55's assumption that the image ships ffmpeg does not hold. Lead fix `f4f6ccf`: declare the dependency in the workflow (install ffmpeg before the suite). The suite's own ffmpeg-absent negative case manipulates PATH internally and is unaffected | FIXED (lead) |
| R-10 B7 push | HEAD `b2e46d2` (Batch 7, gate PASS-WITH-NOTES, GATE2-W70; W68 PC dry-run still running at the commit, folds in on landing) — CI [append run ids + conclusions from the run list at close] | [to fill] |
| R-11 Lead | **Adapter re-proof — RESOLVED**: Batch 7's W62 independently re-proved the work-copy adapter's dual convention, 8/8 cells across both telemetry exit-code generations (old × OK/FAILED/could-not-run, new × OK/FAILED/could-not-run, strip-mismatch refused pre-gate in both check and frames modes); the gate re-ran the grid. The §7 CORRECTED row's "re-proof in flight" is discharged | RE-PROOF PASS |
| R-12 Dispatch | **Bench first run** 43623743480 at `aa92cee` — SUCCESS (disposition of §10's append-promise): CI lavapipe, frames 0–100, 1.85 s/frame; cache-24 leg saves ~147 MiB peak RSS at no measurable in-window time cost. Anomalies (BENCH-FIRST-W64): A1 the window is S01-only (no segment transition — see corrections); A2 accounted-vs-resident cache bytes diverge ~9× (PERF §4 reasoning awaits the RADV legs) | SUCCESS (CI lavapipe only; no PC claim) |
| R-13 Dispatch | **Visual-regression steady state**: pre-pin dispatch 43623748951 failed on the expected UNPINNED pattern; post-pin run 43624398371 exposed the Act-1 pin defect (→ R-4); post-corrective run 43625139818: 3 MATCH (dof, ab1, s08b byte-identical), sole DRIFT = s28 — the verified steady state; s28's disposition remains the user's | AS DESIGNED (s28 red by design) |
| R-14 [to fill] | Errors-step-2 gate dispatches (dof-chart 43624002645, fx 43624007505, film probe 43624011865, determinism 43624016907) + the no-adapter exit-3 proof — append the checked conclusions or record them as never checked | [to fill] |

### 6.3 §8 batch-ledger rows (replace row 6; add rows 7–10)

| Batch | Seats | Scope (short) | Seat handoffs received | Lead integration | CI |
|---|---|---|---|---|---|
| 6 | W51–W60 | §15 conformance checker, s12 bake pack, F2 migration staging pack, knobs patch pack + gate, tools-ci pack 2, VRAM telemetry design, locomotion A/B pack, blink patch pack, lipsync evaluation, gate | Gate PASS-WITH-NOTES (GATE2-W60) | tools-ci pack 2 ✓ (`8b9ac36`); §15 fold-in + guard ✓ (`4dd6530`); F2 migration ✓ (`a2ec80c` + `20a81a7` + `2c41313`); blink ✓ (`5a9383e`); s12 bake proven (adoption = baked data + code-path retirement, lead); knobs NOT landed (no implementation commit in range); VRAM design only; locomotion pack received (adoption = user sign-off); lipsync: adoption changes zero frames of this film, binding semantics = lead decision | committed `3a3a427` [CI to fill] |
| 7 | W61–W70 | KNOBS-W43 tail restoration, adapter re-proof, golden verify + VR triage, first-bench analysis, fold-in checker, split readiness, docfix refresh, PC dry-run, caption-veto brief, gate | Gate PASS-WITH-NOTES (GATE2-W70); W68 NOT GATED (file absent at gate close; agent still running) | Adapter re-proof PASS (W62, independent); Act-1 corrective verified (W63); §15 fold-in census PASS 75/75 (W65; its note's §4 transport-truncated — addendum owed, Batch 8 W71); splits ready, G-TEL texts stale (W66); DOCFIX2-W67 = live doc-fix queue (nothing from W38 was ever applied); caption-veto brief decision-ready (W69) | committed `b2e46d2` [CI to fill] |
| 8 | W71–W80 | Fold-in addendum, VR register update, knobs phases 2–3 staging, F3-a strip pack, G7 telemetry tables, micro-fix pack, exporter acceptance harness, determinism E7 prep audit, record audit (this file), gate | IN FLIGHT at audit time — no verdicts recorded here | [to fill at Batch-8 close] | — |
| 9 | — | [not yet boarded at audit time] | — | — | — |
| 10 | — | [not yet boarded at audit time] | — | — | — |

### 6.4 Remaining close-out sections the fold-in must add

1. **§10 addendum — dispositions at close:** F2 → LANDED
   (R-1/R-7/R-8); F3-a → [strip landed? Batch-8 W74 outcome];
   F3-c → re-queued (STALE S-8); F4 knobs → [landed in the
   fixes pass? per git at audit time: NOT landed]; F5/F6
   unchanged.
2. **Corrections register** (§3 of this audit, C-1…C-8) —
   append as a record section so the closed ledger carries its
   own retractions, not only its claims.
3. **Open remainders at close** (§5 of this audit) — each
   remainder either lands, is waived by name, or is carried
   into the goal's open list with its owner (user / lead /
   PC session).
4. **User decisions owed at close** (unchanged from the wave's
   standing list): caption veto (W69 brief; recommendation
   KEEP 73); s28 disposition; v1.0.1 go; locomotion sign-off;
   skill adoption.
5. **Final verdict block:** final HEAD, CI + Tools CI
   conclusions at that HEAD, determinism + render dispatch
   conclusions, and the wave's seat accounting (100 seats
   ordered; actual runs incl. re-runs stated, per the user's
   counting rule).

---

*Audit ends. One file written (this one); WAVE2-RECORD.md and
every other existing file untouched; nothing committed.*
