# RECORD AUDIT 2 — W91 (Wave 2, Batch 10 — FINAL)

**Seat:** W91 — final WAVE2-RECORD completeness audit.
**Audited at HEAD:** `9380b01` (Batch 9 central commit).
**Object of audit:** `docs/integration/WAVE2-RECORD.md` (448 lines;
last fold-in `cca4e6e`, plus the correction edits `4af54c5` and
`d967931` — the only three in-range commits that touch the file,
verified by `git log b2e46d2..HEAD -- WAVE2-RECORD.md`).
**Method:** the checklist is built from `git log b2e46d2..HEAD`
(**16 commits**) and nothing else — the exact range W79's audit
closed at. Every commit is marked **PRESENT** (the record carries
its substance), **PARTIAL** (part carried, part not), or
**ABSENT** (no trace — verified by grepping the record for each
of the 16 hashes: **all 16 return 0 hits**, so PRESENT marks rest
on substance-matching against the record's text, cited by line).
Corrections were swept from the Batch 6–9 seat notes, the four
gate notes (GATE2-W60/W70/W80/W90), and every in-range commit
message, per W79 §6's standing instruction. This audit does not
use conversation history as evidence; every cited seat/gate note
is a file in the tree.

## Headline

- **3 / 16 commits PRESENT, 1 PARTIAL, 12 / 16 ABSENT.** The
  record is accurate and complete **through `b2e46d2`**: fold-in 2
  (`cca4e6e`) plus the two correction edits closed nearly every
  gap W79 found (scorecard in §4). Everything after `4af54c5` is
  missing: **the entire fixes pass, Batch 8's close, Batch 9
  entire, and the F4 implementation + acceptance** — the wave's
  largest landing is the record's largest absence.
- **The record's single most misleading line is now §10's F4
  disposition (line 398): "IMPLEMENTATION NOT STARTED."** That
  sentence was the *correction* of the wave's worst record error
  (a landing claimed that had not happened). It has since become
  false in the opposite direction: F4 is implemented
  (`ea98458`/`9ad2177`/`76b922c`), gated (`2017e86`), and
  **accepted** (W81: two independent dispatches at `c440d90`,
  both `KNOBS GATE: PASS`). Left unamended at close, the closed
  record would commit the mirror image of the error it corrects.
- **Corrections register (§3): 17 corrections.** Of W79's
  C-1…C-8, seven are now findable in the record (C-2 with a
  row-level residue). Of the nine new corrections C-9…C-17,
  exactly one — C-9, the knobs P6 retraction itself — is in the
  record, because it was applied as a record edit. **W79's
  pattern repeats exactly: corrections the lead folded in reached
  the record; everything since the last fold-in did not, because
  no fold-in has run since `cca4e6e`.**

---

## 1. Commit checklist (from `git log b2e46d2..HEAD`)

### 1a. Batch central commits

| Commit | Subject (short) | Record status |
|---|---|---|
| `021ca01` | Batch 8 (gate PASS-WITH-NOTES; W77 note + W68 dry-run still owed) | **ABSENT** — no §7 B8 row (the §7 log ends at the B7 push row, line 263); §8 row 8 still reads "running at this fold-in" (STALE NS-2) |
| `9380b01` | Batch 9 (gate W90 PASS-WITH-NOTES; F4 ACCEPTED) | **ABSENT** — §8 rows 9–10 are still the placeholder "per orchestrator boards" (line 325) |

### 1b. Lead landings — record-keeping commits

| Commit | Subject (short) | Record status |
|---|---|---|
| `cca4e6e` | WAVE2-RECORD fold-in 2 (Batch 7 + post-B6 landings) | **PRESENT** — embodied: it is the record's current §6–§8/§10 state. Its compliance with W79's skeleton is scored in §4 |
| `4af54c5` | F4 RECORD CORRECTION (W73 register P6) | **PRESENT** — embodied: §10 F4 CORRECTION block (line 398 ff.) + §8 row 8's F4 sentence. Its present-tense state claim is itself now superseded (STALE NS-1) |
| `33c5b2a` | KNOBS-W43 tail splice (W61 tail appended; T2 included) | **PRESENT in substance** — §10 F4 (line 412): "truncation repaired by the lead's W61-tail splice, T2 included". No §7 row; §8 row 7 still says "splice queued" (STALE NS-4) |

### 1c. Lead landings — F4 implementation series

| Commit | Subject (short) | Record status |
|---|---|---|
| `ea98458` | Knobs PHASE 1 (`Knobs{res,aa,shadow}`, `is_draft()` born with the full three-knob condition) | **ABSENT** — no §7 row; §10 says the opposite (NS-1) |
| `9ad2177` | Knobs PHASE 2 (--aa plumbing; ab1 trap line held) | **ABSENT** — and its build break is disclosed only in `c440d90`'s message (correction C-11) |
| `76b922c` | Knobs PHASE 3 (--shadow lockstep) — "F4 IMPLEMENTATION COMPLETE" | **ABSENT** |
| `2017e86` | Knobs gate workflow (dispatch; legs A/B/C; exact-line verdict) | **ABSENT** — the gate's wiring *and* its acceptance (two PASS runs at `c440d90`, KNOBS-GATE-W81 + GATE2-W90) are both unrecorded |
| `c440d90` | Fix ab1 scene-pass delimiter (knobs phase-2 fallout; four probe dispatches failed at Build) | **ABSENT** |

### 1d. Fixes-pass landings (GATE2-W80 §12.1 order) + Batch-9-window landings

| Commit | Subject (short) | Record status |
|---|---|---|
| `c7aa648` | DOCFIX2-W67 applied in its §10 order (supersedes W38 wholesale) | **ABSENT** — §8 row 7 still says "application queued" (NS-4); §10's preamble still credits W38's pack (NS-7) |
| `d967931` | W76 micro-fixes (finish.sh AAC 192k→256k; HELP exit table + scenes clause) + the F4 correction's Batch-8 row | **PARTIAL** — its 1-line record edit is embodied (§8 row 8); the micro-fix landings themselves have no §7 row (correction C-16) |
| `3c07564` | W72 VR register update (post-pin register + fixtures, one commit) | **ABSENT** — the register's first dispatch verdict at that head (failure as designed: 3 MATCH / 1 DRIFT = s28 / 14 NOT-RENDERED, per GATE2-W90) is likewise unrecorded |
| `2fcffc0` | F3-a strip (W74 pack §6): baseline 196 → 24/9 | **ABSENT** — §10 F3-a still reads as a future disposition (NS-12); the landing used W74's corrected figure (C-10) |
| `108f33f` | W71 §4 splice into FOLDIN-W65.md + W78 runbook paste-ins | **ABSENT** — discharges correction C-7's repair and the E7 runbook gaps; neither is recorded |
| `3223509` | G7 tables in the binary (`--film-telemetry --tables`; telemetry.rs +464) | **ABSENT** — landed inside the Batch-9 window on the W85 spec; its diff-check against that spec is owed (Batch-10 seat W93), so even its verdict is not yet statable |

---

## 2. STALE register — record lines superseded at HEAD

Line numbers are the record's at audit time. Items NS-6…NS-11
are **carried over from W79's register**: fold-in 2 did not apply
the corresponding skeleton amendments (§4), so they are re-issued
here with their current state.

- **NS-1 — §10, F4 (lines 398–399 + the "TRUE state" paragraph):**
  "SPEC + PATCH PACKS COMPLETE; IMPLEMENTATION NOT STARTED
  (CORRECTION, 2026-11-15)."
  **Superseded:** implementation landed `ea98458` (phase 1),
  `9ad2177` (phase 2), `76b922c` (phase 3 — its message declares
  "F4 IMPLEMENTATION COMPLETE"); the gate workflow landed
  `2017e86`; Batch 9's W81 verified the acceptance independently
  — two dispatches at `c440d90` (43628289244 the lead's,
  43628339967 W81's), both success, exact line
  `KNOBS GATE: PASS` (leg A 40/40 byte-identical, no DRAFT; leg B
  exact DRAFT tag at 540×960; leg C 10/10 exit 2). GATE2-W90
  re-read the logs first-hand and confirmed. **The correction
  block must be preserved verbatim as history** — it is the
  record of a retraction — but a closing paragraph must follow
  it (§6, A-11), or the closed record ends on a false present
  tense.
- **NS-2 — §8, Batch 8 row (line 324):** "running at this
  fold-in" … "still queued: knobs implementation (W73 basis),
  VRAM line, s12 corpus".
  **Superseded:** Batch 8 closed at `021ca01` (gate
  PASS-WITH-NOTES, GATE2-W80). Of the three queued items: knobs
  **landed + accepted** (NS-1); VRAM implementation is **staged,
  not landed** (VRAM-IMPL-W84, Batch 9 — formula reproduced
  byte-exact, 220,930,688 steady / 229,286,528 peak); s12
  adoption is **staged-ready, not landed** (S12-ADOPT-W83 —
  zero drift since W52; retirement lines film.rs:2021–2023 +
  probes 1666–1810 prescribed).
- **NS-3 — §8, rows 9–10 (line 325):** "per orchestrator boards".
  **Superseded:** Batch 9 closed at `9380b01` (gate
  PASS-WITH-NOTES, GATE2-W90; all seats W81–W89 ACCEPT). Batch
  10 is the close batch (this audit's cohort).
- **NS-4 — §8, Batch 7 row (line 323):** three queue/running
  claims, all discharged in-range: "W61 tail = splice queued
  (§12.3b)" → landed `33c5b2a`; "W67 DOCFIX2 = application
  queued (§12.3a)" → applied `c7aa648`; "W68 PC dry-run STILL
  RUNNING at batch close — folds in on landing" → **landed**
  (PC-DRYRUN-W68.md; see NS-5).
- **NS-5 — §7, B7 push row (line 263) + line 323:** "W68 PC
  dry-run still running, folds in on landing."
  **Superseded:** W68's note landed after the Batch-9 gate (its
  third consecutive gate-close absence, recorded in GATE2-W90).
  At audit HEAD the file sits **untracked** in the tree
  (`git status`: `?? docs/integration/PC-DRYRUN-W68.md`) — it
  must be committed (Batch-10 central commit) before the record
  may cite it as landed. Its legs rehearsed green on this VM
  (telemetry `CHECK OK` exit 0 observed; golden verify 18 OK;
  refusals as documented) and it carries one BLOCKING finding
  (correction register, W68-F1 below).
- **NS-6 — §10, F2 (line 377):** "AUDITED (W41), landing
  scoped." *(W79 S-6; skeleton amendment 5 not applied.)*
  §7 rows 259–260 record F2's first landing and completion;
  the §10 disposition was never re-marked. Still stale.
- **NS-7 — §10, preamble (lines 367–370):** "errors
  mid-flight" and "W38's doc pack + the v1.0.1 release close
  the stale-binary side." *(W79 S-7; amendment 6 not applied.)*
  Errors step 2 landed at `edb058f`; **nothing from W38 was
  ever applied** (DOCFIX2-W67 verified it); its supersession
  DOCFIX2 **was** applied at `c7aa648`. The preamble credits
  the wrong pack and the wrong state, twice over.
- **NS-8 — §10, F3-c (lines 395–397):** "resolve at the §15
  fold-in — either the loader consumes it or the data stops
  carrying it, never both." *(W79 S-8; amendment 7 not
  applied.)* The fold-in (`4dd6530`) landed guard-only; the
  either/or did not occur there. **New state:** Batch 9's W88
  staged the resolution **asymmetrically** — `nodes[].materials`
  → STRIP via decomposition (staged bookshop file in
  `docs/integration/f3bc/`); `lights.key.near` → CONSUME (one
  loader line; film.rs already carries `key_near`). Staged
  only — not landed, and the record still points at the
  fold-in as the resolving event.
- **NS-9 — §6, exit-code contract (lines 209–212):** "…
  re-proven by the lead with the REAL binary at the exit-code
  head — FAILED direction §7; OK/old-binary directions appended
  there as they land." *(W79 S-5; amendment 1 not applied.)*
  §7 now carries both the CORRECTED row retracting that proof
  and W62's closing row (line 255, 8/8 PASS) — so §6's sentence
  is contradicted by the record's own verdicts log and still
  unamended.
- **NS-10 — §7, GOLDEN ACT 1 row (line 247):** "14 film
  first-pins struck — 11 mids from the `f021991` candidates
  (planner-proven pixel-identical outside the split windows)…"
  *(W79 S-3; amendment 2 not applied.)* Fold-in 2 added the
  separate CORRECTIVE row (line 252) instead of amending this
  row. The record as a whole is no longer misleading — the
  correction is three rows below — but the row itself still
  asserts the intended composition as the committed one.
- **NS-11 — §8, Batch 1 row (line 317):** "pins pending
  (post-re-render sequencing, GOLDEN-RESTAGE-W03 §4.3)."
  *(W79 S-2; amendment 4 not applied.)* Both acts struck
  (`916a109`, `166873c`), Act 1 corrected (`2755ff3`); s28
  unpinned by design. Still stale.
- **NS-12 — §10, F3-a (line 389):** "174 occurrences are dead
  provenance written by `gen_film_data.py` (strip at the
  generator; data-only change, gated on telemetry CHECK OK)".
  **Superseded:** the strip **landed** at `2fcffc0` — generator
  patched, `film/onoda.json` stripped, baseline lowered
  **196 → 24 (9 paths)** per W74's corrected pack (172 stripped;
  root `fps`/`frames` re-classed F3-a-kept — they have HARD
  readers in the Python tools). The disposition still reads as
  future work.
- **NS-13 — §10, closing append-promises (lines ~437–447):**
  "verdicts to be appended to §7 when checked" / "verdicts
  appended when checked; plus the no-adapter failure proof".
  **Discharged in substance, unmarked in text:** §7 now carries
  the bench row (line 254, run 43623743480 SUCCESS), the VR
  steady-state row (line 253, run 43625139818), and the
  errors-step-2 gates row incl. the no-adapter exit-3 proof
  (line 251). Two residues: the promise texts still read as
  outstanding; and the pre-pin VR run **43623748951** still has
  no §7 row of its own (its id occurs only inside the promise
  text itself) — the defect-exposing run 43624398371 is carried
  inside the CORRECTIVE row (line 252). Also noted at cite
  level: §10 promised fx run 43624007505; §7's appended verdict
  is from the re-run 43624248449 "(correct fixture scene)" —
  the substitution is visible in §7's parenthetical but nowhere
  explained.

### Not stale (checked, still accurate)

- §5 caption text + "CLOSED BY DATA" — unchanged and still the
  standing disposition; the veto path is live (W69 brief). ✓
- Watch item "painter panic observed once, not reproduced" —
  no recurrence anywhere in the range's commits or notes. ✓
- Watch item on `frames4` baseline scope — nothing in range
  contradicts it. ✓
- PERF.md watch item ("§1 dated correction owed"): the DOCFIX2
  application (`c7aa648`) did land PERF corrections (preamble /
  FAILED-row / cache-figure, plus the §9 discharge note already
  present in PERF itself), but **this audit did not verify the
  §1 sentence line-by-line**; the record's watch item was never
  updated either way. The close fold-in must verify and either
  close or explicitly carry it (§6, A-13) — do not silently
  drop it.

---

## 3. Corrections register — complete, Batches 1–9

A correction is *findable* if a reader of **the record alone**
can discover that an earlier claim was wrong. C-1…C-8 are
W79's register, re-scored after fold-in 2. C-9…C-17 are new
(Batches 8–9 + in-range commit messages + gates). W68's
findings are scored separately below the table.

| # | Correction | Where the correction lives | In the record? |
|---|---|---|---|
| C-1 | Adapter real-binary re-proof PASS retracted (run SIGKILLed; fixture strip-mismatched) | Record §7 CORRECTED row; closed by W62's independent 8/8 RE-PROOF PASS (`b2e46d2`), now §7 line 255 | **YES — recorded and CLOSED.** Residue: §6's text never amended (NS-9) |
| C-2 | Act-1 pins described superseded bytes (11 mids = staged `381ff47`-era bytes; canary-caught) | `2755ff3` message; GOLDEN-VERIFY-W63; record §7 CORRECTIVE row (line 252, added by fold-in 2) | **YES** — via the corrective row. Residue: the Act-1 row itself unamended (NS-10) |
| C-3 | W55's assumption that the hosted CI image ships ffmpeg | `f4f6ccf` message; record §7 Incident 3 row (line 258, added by fold-in 2) | **YES** |
| C-4 | Bench-window claim false (window is S01-only; transition cost unmeasured) + accounted-vs-resident ~9× divergence | BENCH-FIRST-W64 anomalies A1/A2; record §7 bench row (line 254) names both | **YES** |
| C-5 | Act-2 provenance cite: run 43621168728's head is `dc0e555`, not `9f497c7` | GOLDEN-VERIFY-W63 §3; record §7 line 239 corrected in place by fold-in 2, with an explicit "[Cite corrected per W63 §3…]" bracket — the correction of record | **YES** |
| C-6 | KNOBS-W43.md truncated on disk at line 264; tail restored provenance-tagged by W61 (6 items UNRECOVERABLE) | KNOBS-W43-TAIL-W61; splice landed `33c5b2a`; record §10 F4 (line 412) states "truncation repaired by the lead's W61-tail splice, T2 included" | **YES, in substance** — findable in §10; no §7 row for the splice commit |
| C-7 | FOLDIN-W65.md §4 itself transport-truncated (second occurrence of the class) | `b2e46d2` message; GATE2-W70; addendum FOLDIN-ADDENDUM-W71; **repair landed `108f33f`** (W71 §4 splice, FOLDIN-W65.md +222 lines) | **NO** — neither the truncation nor its landed repair is in the record |
| C-8 | Split blueprints' G-TEL gate texts quote the pre-re-chunk `CHECK FAILED` line + exit 1 (healthy truth: `CHECK OK`, exit 0) | SPLIT-READY-W66; record watch item carries the hazard; **Batch 9's SPLIT-AMEND-W82 re-words the texts** (amendment pack) | **NO** — the hazard is recorded; W82's amendment and its new precondition (below) are not |
| C-9 | **Knobs phase-1 landing claim was a RECORD ERROR** (W73 register item P6, KNOBS-P23-W73.md:240: "record error — correct WAVE2-RECORD / the fold-in"): no Knobs struct / `effective_size` / parse arms / DRAFT string existed at `b2e46d2`; all four flags rejected as unknown arguments; the attached 540×960 / 22.35 dB figure is void (render.yml has no size input). W76 + W79 independently confirmed; GATE2-W80 lists it as headline correction (1) | Corrected **in the record itself** at `4af54c5` (§10 F4 CORRECTION block + §8 row 8; row refined at `d967931`) | **YES** — the model in-record correction of this range. **Follow-on owed:** its present-tense "NOT STARTED" is now false (NS-1) |
| C-10 | F3-a lowering figure corrected: **196 → 24, not 22** — root `fps`/`frames` have HARD readers (`blink_cost`, `blink_census`, `evaluate_track` exits 2 without `frames`) and are re-classed F3-a-kept; strip = 172 occurrences | F3A-STRIP-W74.md:25,48; `021ca01` message; GATE2-W80 headline correction (2); strip landed at the corrected figure in `2fcffc0` | **NO** — §10 F3-a carries neither the corrected figure nor the landing (NS-12) |
| C-11 | Knobs phase 2 as landed **broke the build**: the attachment replacement in ab1 left the pre-knob closing `})],` dangling (rustc: unexpected closing delimiter, ab1.rs:1438); **all four phase-2 probe dispatches failed at Build**. The film/scenefile/dof replacements consumed their closers; ab1's cut one line short. One line removed, no logic change | `c440d90` commit message only (lead self-disclosure) | **NO** — neither the phase-2 landing nor its break/fix is in the record |
| C-12 | W66's split note "calendar_state's hardcoded `years = 4` moves as-is" is **dead**: the F2 migration (`2c41313`) landed after W66's run; at HEAD `calendar_state(lt_cal, years)` takes the count from `data.calendar_years.len()` | SPLIT-AMEND-W82.md §3 (announced at its lines 16–17) | **NO** — the record's split-hazard watch item still presents W66's readiness notes unamended |
| C-13 | W59's lipsync option (a) **as worded cannot work**: step-after-*silence* keys cannot fix the `pp` closures — the leaking segment is governed by the key *after* the closure, whose predecessor is 0.02, not 0.0. The measured-correct rule is step on keys whose **predecessor is closed** (closed-exit rule): closures 36/36, out-of-line leak 0, identical to full-step on all painted classes (LIPSYNC-BIND-W86 re-derived every W59 figure first, then measured) | LIPSYNC-BIND-W86.md (line 40: "One correction to W59 along the way") | **NO** |
| C-14 | The F3-c either/or resolves **asymmetrically**, not as one branch: `nodes[].materials` → STRIP (via decomposition into s10's form, values preserved); `lights.key.near` → CONSUME (scalar completing an existing rig; film.rs `LightState` already carries `key_near`, hardcoded 0.5 = the data's value) | F3BC-W88.md (STRIP recommendation §, staged files in `docs/integration/f3bc/`) | **NO** — staged only, not landed; §10 still frames the fold-in as the resolving event (NS-8) |
| C-15 | W88's "pristine 196/15 control" was **already historical when the Batch-9 gate re-ran it**: the lead landed `2fcffc0` inside the gate window, so the live tree sat at 24/9; W88's staged states (→ 3 unknown / 3 paths) are unaffected. Labelled by the gate itself as a method note, **not a seat error** | GATE2-W90.md (lines 197–200) | **NO** — the record has no F3 landing at all (C-10), so the note has nothing to attach to until §6 lands |
| C-16 | The binary's HELP exit-code table **omitted exit 4 (ImageIo) entirely and misdescribed 3**; and finish.sh's `-b:a 192k` was the skill's DRAFT audio rate paired with master video settings. Found by W76 (which verified both micro-fixes STILL-OPEN in Batch 8); **fixed at `d967931`** (256k; HELP table + stale scenes/film clause) | MICROFIX-W76.md; `d967931` (finish.sh + src/main.rs) | **NO** — the find/fix pair has no §7 row (commit is PARTIAL in §1 for exactly this reason) |
| C-17 | Cite drift (minor class): RELEASE-101B-W89 cited the `--version` derivation at main.rs:274; at gate-close HEAD it sits at **main.rs:280** (lead landings drifted it). Pack's verdict unaffected | GATE2-W90.md:226 | **NO** — listed for sweep completeness; no record text cites the line, so no record amendment is owed beyond this register |

**Pattern, restated for the close:** W79 found 1 of 8
corrections in the record; fold-in 2 raised that to 7 of 8
(C-7/C-8 outstanding). The nine corrections issued *since*
fold-in 2 stand at 1 of 9 (C-9 — itself a record edit). The
defect is not editorial quality; it is that **the record only
learns what a fold-in carries, and no fold-in has run since
`cca4e6e`.** The close fold-in (§6) is therefore specified to
sweep seat notes, gate notes, *and commit messages* — C-11
exists only in a commit message.

### W68 findings F1–F6 — should the record carry them?

W68's dry-run (PC-DRYRUN-W68.md, landed late, untracked at
audit HEAD) issued six findings against the PC runbook. Scored
individually, as commissioned:

- **F1 — BLOCKING (PC-DRYRUN-W68.md:349): YES, the record must
  carry it.** Runbook §0.2 provisions the binary to
  `~/talerender-bin`, but §§0.4/2.1/3.1/3.2 invoke
  `./talerender-bin/talerender` from the repo root —
  reproduced verbatim: `No such file or directory`, exit 127.
  The same defect is in USER-PC.md §§3–5. This blocks the PC
  session that the record's own §3/S4 and watch items track as
  the wave's outstanding RADV leg: a reader planning that
  session from the record would walk into exit 127 in the
  first minutes. One watch-item line + the fix pointer (the
  canonical-path fix pack is Batch-10 seat W96's board) is the
  right weight — §6, A-13.
- **F2–F6 — NO individual rows.** §3.2's completion line is a
  prefix of film.rs's real format (F2); §4.3's `--out` creates
  directories, not .txt files (F3); `vulkaninfo` is an
  unlisted prerequisite (F4); §0.2 doesn't say the listed run
  must be green (F5); the RAM flags predate the paint byte
  budget (F6, observed telemetry peak 2,162,688,000 B against
  the 2 GiB budget). These are runbook errata; their home is
  PC-EVIDENCE-W36.md's next revision, not the ledger. The
  record should carry **one line** (§6, R2-13) stating the
  dry-run landed, its legs rehearsed green, and that F1 is
  carried as a watch item while F2–F6 are owned by the
  runbook's revision.

---

## 4. Scorecard — fold-in 2 vs W79's skeleton (§6 of RECORD-AUDIT-W79)

W79's §6 was the explicit instruction set for the close
fold-in. Fold-in 2 (`cca4e6e`) + `4af54c5` + `d967931` are
scored against it so the close fold-in (§6 below) only carries
what is actually left.

### 4.1 Amendments to existing text (W79 §6.1, items 1–8)

| # | Amendment | Applied? |
|---|---|---|
| 1 | §6 adapter bullet: strike the lead re-proof sentence | **NO** → NS-9 |
| 2 | §7 Act-1 row: append the composition caveat | **NO** (corrective row added instead) → NS-10 |
| 3 | §7 ab1 rows: correct `9f497c7` → `dc0e555` | **YES** — line 239, with correction-of-record bracket (C-5) |
| 4 | §8 Batch-1 row: "pins pending" → struck/corrected | **NO** → NS-11 |
| 5 | §10 F2: re-mark LANDED | **NO** → NS-6 |
| 6 | §10 preamble: errors state + W38 sentence | **NO** → NS-7 |
| 7 | §10 F3-c: record the missed resolution, re-queue | **NO** → NS-8 |
| 8 | §10 append-promises: append the verdicts or strike the promises | **IN SUBSTANCE** — verdicts appended (§7 lines 251/253/254); promise texts unmarked; run 43623748951 still row-less → NS-13 |

**Score: 1 of 8 applied verbatim, 1 in substance, 6 not
applied.** The six survivors are all one-line-class edits;
none requires new evidence. They are carried into §6 verbatim
so the close fold-in does not have to re-derive them.

### 4.2 Pre-drafted §7 rows (W79 §6.2, R-1…R-14)

All fourteen are present in §7 (R-1 F2 first landing, line 259;
R-3 §15 fold-in, line 256; R-4 Act-1 CORRECTIVE, line 252; R-5
tools-ci pack 2, line 257; R-6 blink adoption, line 261; R-7/R-8
F2 migration COMPLETE, line 260; R-9 Incident 3, line 258; R-11
adapter re-proof resolved, line 255; R-12 bench, line 254; R-14
errors-step-2 gates + no-adapter proof, line 251) — **except
R-13's first element**: the pre-pin VR run 43623748951 has no
row (NS-13). R-2/R-10's CI cells were filled (B6 row: "CI +
tools-ci SUCCESS", no run ids; B7 row, line 263: run ids
43626618994 / 43626619083, both SUCCESS).

### 4.3 Batch-ledger rows (W79 §6.3)

Row 6 replaced ✓ (line 322, incl. knobs-not-landed as of its
writing); row 7 added ✓ (line 323); row 8 added ✓ (line 324 —
written as "running", correct at fold-in time, stale now,
NS-2); rows 9–10 left as the placeholder W79 drafted (line
325, NS-3).

---

## 5. Open remainders at HEAD `9380b01`

| Remainder | In record? | State at HEAD (per git + notes) |
|---|---|---|
| RADV leg (determinism E7, bench, PERF tables, VRAM sysfs) | ✓ §3 S4, §7 E6 row | Still owed on the user's PC. W68 dry-run landed (untracked — commit owed) and found the runbook's F1 path defect BLOCKING as written; fix pack = Batch-10 W96 |
| Caption veto (user, at wave close) | ✓ §5 + watch item | Live; W69 brief decision-ready (veto stales 3 in-window pins; recommendation KEEP 73) |
| s28 disposition (user) | ✓ watch item | Open. New state unrecorded: under the W72-updated register (`3c07564`) VR reads failure-as-designed, s28 sole DRIFT |
| v1.0.1 release (user go) | ✓ §8 row 5, §10 | Pack refreshed by W89 (RELEASE-101B-W89): **no-go yet** — waits on user go, wave freeze, the bump, and six gates re-proven on the bump SHA |
| Skill adoption (user) | ✓ §4 Phase 5 | Open |
| Knobs implementation (F4) | ✗ (says NOT STARTED) | **COMPLETE + ACCEPTED** (NS-1) — the fold-in closes it |
| Film/scene split programmes | △ watch item (W66 hazards) | Not executed. W82 amendment pack staged (G-TEL re-worded, adapter paragraph struck, cites refreshed; state_key freeze byte-identical, md5 `95918b99…`, film.rs:2246). **Precondition open:** the 46/46 stroke-battery leg has no standing invocation at HEAD (battery is a binary mode, absent from ci.yml/render.yml/cargo test) — S2/S4/S5 must not start until the lead establishes a dispatch (W82 §5) |
| s12 baked-corpus adoption | ✗ | READY (W83: zero drift, proof chain reproduces; adoption = staged data + retire film.rs:2021–2023 + probes 1666–1810). Not landed |
| VRAM telemetry implementation | ✗ | Staged (W84: census re-derived at HEAD; defaults reproduce 220,930,688 / 229,286,528 exactly; two staging rulings owed by the lead — bloom per-call registration, telemetry must not print the line). Not landed |
| G7 telemetry tables | ✗ | **Implementation LANDED** `3223509` — diff-check vs the W85 spec owed (Batch-10 W93); unverified by GATE2-W90 |
| Locomotion adoption | △ §8 row 3 | Step-0 registration pack ready (W87: one `mod` line + zero-pixel proof protocol; 9 gait tests join CI). Registration not landed; D1–D4 + the plain-language decision sheet are the user's |
| Lipsync binding | △ §8 rows 6–7 (deferred) | Decision pack ready (W86): Variant A (builder-side closed-exit step keys) recommended, measured; line-3 sign-off sheet = user decision. Binding not landed |
| Blink adoption | ✓ §7 line 261 | Landed `5a9383e` (recorded by fold-in 2) |
| F3-a strip | ✗ (§10 future-tense) | **Landed** `2fcffc0` (NS-12) |
| F3-b (19 cite-don't-strip) | ✓ §10 ruling | W88 dispositioned all 19 CITE → strip-with-citation-recorded; staged files in `f3bc/`; not landed |
| F3-c (materials / near) | ✓ §10 (stale framing) | W88 staged asymmetric resolution (C-14); not landed |
| F2 migration | ✓ §7; ✗ §10 | Complete in git; §10 re-mark owed (NS-6) |
| Doc-fix application | △ §10 (wrong pack credited) | DOCFIX2 applied `c7aa648`; W38 never applied (NS-7) |
| Exporter acceptance (W77) | ✗ | Note still absent at HEAD; its tool file shows **modified in the working tree** (seat still iterating). Batch 8's acceptance stands on the committed version (GATE2-W80/W90) |
| W68 note commit | ✗ ("still running") | Landed but **untracked**; commit in the Batch-10 central commit (NS-5) |
| PERF.md §1 dated correction | ✓ watch item | Unresolved as a record item — see "Not stale" note above; close or carry explicitly (A-13) |
| Painter panic (observed once) | ✓ watch item | Not reproduced in range |
| Charanim beyond blink; atlas probation; bufferpool under Readback; rendergraph | ✓ §9 decisions | Implementations open (post-wave scope) |

---

## 6. THE CLOSE FOLD-IN (exact — apply in this order)

This is the fold-in the lead must apply for the record to be
called complete at wave close. Part A amends existing text
(each item: location → strike → replacement, paste-ready).
Part B is new §7 rows, pre-drafted using **only** verdicts
stated in commit messages or gate notes read first-hand by
this audit — bracketed cells are the lead's to fill from the
run list at close; nothing here invents a CI verdict git does
not state. Part C replaces the §8 rows. Part D lists the
sections to append. Apply A before B (B's rows cite A's
amended §10 state).

### Part A — Amendments to existing text

- **A-1 (§6, lines 209–212; discharges NS-9 / W79 amendment 1).**
  Strike: "stub-proven 10/10 by W02 and re-proven by the lead
  with the REAL binary at the exit-code head — FAILED direction
  §7; OK/old-binary directions appended there as they land".
  Replace with: "stub-proven 10/10 by W02; the lead's
  real-binary re-proof claim was retracted (§7 CORRECTED row)
  and the convention was then re-proven independently by W62,
  8/8 cells across both exit-code generations (§7, B7/W62
  row)".
- **A-2 (§7, Act-1 row, line 247; discharges NS-10 / W79
  amendment 2).** Append to the row's What cell: "(Composition
  as intended; as committed, the 11 mids were the staged
  `381ff47`-era bytes — see the CORRECTIVE row below.)"
- **A-3 (§8, Batch-1 row, line 317; discharges NS-11 / W79
  amendment 4).** Strike "pins pending (post-re-render
  sequencing, GOLDEN-RESTAGE-W03 §4.3)"; replace with "pins
  struck `916a109`/`166873c`, Act 1 corrected `2755ff3`; s28
  unpinned by design".
- **A-4 (§8, Batch-7 row, line 323; discharges NS-4).** Strike
  "W61 tail = splice queued (§12.3b); W67 DOCFIX2 = application
  queued (§12.3a)"; replace with "W61 tail splice landed
  `33c5b2a`; DOCFIX2 applied `c7aa648`". Strike "W68 PC dry-run
  STILL RUNNING at batch close — folds in on landing"; replace
  with "W68 PC dry-run landed late (PC-DRYRUN-W68.md; F1
  carried as a watch item)".
- **A-5 (§7, B7 push row, line 263; discharges NS-5).** Strike
  "W68 PC dry-run still running, folds in on landing"; replace
  with "W68 PC dry-run landed after the Batch-9 gate
  (PC-DRYRUN-W68.md)".
- **A-6 (§10, preamble, lines 367–370; discharges NS-7 / W79
  amendment 6).** Strike "errors mid-flight"; replace with
  "errors step 2 landed (`edb058f`, fix `aa92cee`)". Strike
  "W38's doc pack + the v1.0.1 release close the stale-binary
  side"; replace with "DOCFIX2-W67 (which superseded W38
  wholesale — nothing from W38 was ever applied) was applied
  at `c7aa648`; the v1.0.1 release closes the stale-binary
  side".
- **A-7 (§10, F2, line 377; discharges NS-6 / W79 amendment
  5).** Strike "F2 — AUDITED (W41), landing scoped:"; replace
  with "F2 — LANDED (`a2ec80c` loud fallbacks + `20a81a7`
  Phase D + `2c41313` Phases L+T; migration COMPLETE, zero
  frame impact — §7):".
- **A-8 (§10, F3-a, line 389; discharges NS-12).** Append to
  the F3-a disposition: "**LANDED `2fcffc0`** (W74 pack §6):
  172 occurrences stripped at the generator; baseline lowered
  196 → 24 occurrences / 9 paths. Root `fps`/`frames` are NOT
  stripped — they have hard readers in the Python tools and
  are re-classed F3-a-kept (W74's correction of the briefed
  196 → 22 figure)."
- **A-9 (§10, F3-c, lines 395–397; discharges NS-8 / W79
  amendment 7).** Strike "resolve at the §15 fold-in — either
  the loader consumes it or the data stops carrying it, never
  both"; replace with "the §15 fold-in landed guard-only and
  the either/or did not occur there (W79 S-8). Batch 9's W88
  staged the resolution asymmetrically (F3BC-W88 + `f3bc/`):
  `nodes[].materials` → STRIP via decomposition;
  `lights.key.near` → CONSUME (one loader line). Staged, not
  landed — carried as an open remainder".
- **A-10 (§10, closing promises, lines ~437–447; discharges
  NS-13).** Append one line after the second promise
  paragraph: "**Discharged:** bench 43623743480, VR
  steady-state 43625139818, the four errors-step-2 dispatches
  and the no-adapter exit-3 proof are appended in §7 (the fx
  verdict is from the re-run 43624248449 on the correct
  fixture scene, superseding 43624007505)." And add §7 row
  R2-16 (Part B) for the one run still row-less.
- **A-11 (§10, F4, after the CORRECTION block; discharges
  NS-1).** **Do not alter the CORRECTION block** — it is the
  retraction of record. Append immediately after it:
  "**LANDED + ACCEPTED (post-correction):** phase 1 `ea98458`
  (Knobs struct, parsers, `effective_size`, DRAFT tag;
  `is_draft()` born with the full three-knob condition per
  W73's warning), phase 2 `9ad2177` (--aa plumbing; ab1's
  basic_overlay trap line held at literal 1; its build break
  and one-line fix are `c440d90`), phase 3 `76b922c` (--shadow
  lockstep; bit-identical at 2048 by arithmetic) — F4
  IMPLEMENTATION COMPLETE. Gate workflow `2017e86`; acceptance
  verified independently by W81: two dispatches at `c440d90`
  (43628289244, 43628339967), both `KNOBS GATE: PASS` (leg A
  40/40 byte-identical at defaults, no DRAFT; leg B exact
  DRAFT tag, 540×960; leg C 10/10 invalid cases exit 2).
  Defaults remain byte-identical; knobs are draft/preview
  only per the laws above."
- **A-12 (§8, Batch-8 row, line 324).** Replace the entire row
  with Part C's row 8.
- **A-13 (Watch items; discharges NS-5's follow-on + the PERF
  item + C-8/C-12's record side).**
  1. Add: "**W68 F1 — PC runbook binary path is BLOCKING as
     written:** §0.2 provisions `~/talerender-bin` while
     §§0.4/2.1/3.1/3.2 invoke `./talerender-bin/talerender`
     from the repo root (exit 127 reproduced; same defect in
     USER-PC.md §§3–5). Fix before the PC session (canonical-
     path pack, Batch-10 W96). F2–F6 are runbook errata owned
     by PC-EVIDENCE-W36's next revision (PC-DRYRUN-W68.md)."
  2. PERF.md §1 item: verify the §1 sentence against
     `aa10b13`'s behaviour; if the DOCFIX2 application
     (`c7aa648`) already corrected it, strike the watch item
     with the cite; if not, carry it into the goal's open list
     by name. Do not drop it silently.
  3. Split-hazard item: append "Amended by SPLIT-AMEND-W82
     (Batch 9): G-TEL texts re-worded to `CHECK OK` / exit 0,
     W21 §4's adapter paragraph struck, cites refreshed
     (state_key freeze byte-identical, film.rs:2246, md5
     `95918b99…`); W66's calendar_state note is superseded
     (the count is data-driven since `2c41313`). Precondition
     before S2/S4/S5: establish a standing 46/46 stroke-
     battery dispatch — none exists at HEAD."

### Part B — New §7 rows (paste-ready; append after the B7 push row)

| When | What | Verdict |
|---|---|---|
| R2-1 B8 push | HEAD `021ca01` (Batch 8, gate PASS-WITH-NOTES, GATE2-W80 §12 = the fixes-pass packet; W77 note still owed, W68 landed later) | [CI + Tools CI conclusions from the run list at close] |
| R2-2 Lead | **DOCFIX2-W67 applied** `c7aa648` — supersedes W38 wholesale (nothing of W38 had ever been applied): FINAL-REPORT Addendum 2 refreshed, RESULTS discharge note, FOG §8 WIRED note, film README + root README corrections, PERF discharge/status/corrections; two disclosed adaptations (adapter paragraph closes with W62's 8/8 PASS; F2 sentence completed with `20a81a7`/`2c41313`) | LANDED |
| R2-3 Lead | **KNOBS-W43 tail splice** `33c5b2a` (W61 tail appended verbatim; truncation markers deleted; T2 included with provenance tag; six UNRECOVERABLE items stay registered in W61 §3) | LANDED |
| R2-4 Lead | **F4 RECORD CORRECTION** `4af54c5` (W73 register P6): the Batch 7/8-relayed phase-1 landing did not happen; §10 F4 + §8 corrected in place. The 540×960 / 22.35 dB figure's provenance is void | CORRECTED |
| R2-5 Lead | **Knobs PHASE 1** `ea98458` (W54 §§1–2 + W73 P1–P5): Knobs + parsers (exit 2 pre-GPU), `effective_size` identity structural, `is_draft()` born with the full three-knob condition, tag sites end-appended (empty at defaults); disclosed behaviour change: CPU overlays scale to frame at non-default --width/--height without --res | LANDED |
| R2-6 Lead | **Knobs PHASE 2** `9ad2177` (--aa plumbing): one attachment shape in all four scene passes; HDR MSAA colour + depth twin take samples together; **the trap line held** — ab1's basic_overlay keeps literal 1 (re-verified per W73) | LANDED |
| R2-7 Lead | **Knobs phase-2 fallout fix** `c440d90`: ab1's replacement left the pre-knob closing `})],` dangling (rustc unexpected closing delimiter, ab1.rs:1438); all four phase-2 probe dispatches failed at Build. One line removed; no logic change | FIXED (lead) |
| R2-8 Lead | **Knobs PHASE 3** `76b922c` (--shadow lockstep: colour+depth targets take shadow_size together; shadow_misc = [1/size, size, 0, 0] — bit-identical at 2048 by arithmetic; --shadow 4096 fails visibly on downlevel limits, never clamped) | **F4 IMPLEMENTATION COMPLETE** |
| R2-9 Lead | **Knobs gate workflow** `2017e86` (dispatch-only; legs A identity / B draft smoke / C validation via `tools/ci/knobs_gate.py`; passes only on the exact `KNOBS GATE: PASS` line) | WIRED |
| R2-10 B9/W81 | **F4 ACCEPTANCE (independent):** two knobs-gate dispatches at `c440d90` — 43628289244 (lead) + 43628339967 (W81) — both success; verdict line `KNOBS GATE: PASS` in both. Leg A byte-probes 40/40 identical, no DRAFT in stdout; leg B exact tag `[DRAFT res=0.5 aa=0 shadow=512 — NOT FOR DELIVERY]`, IHDR 1080×1920 → 540×960, draft frames differ as required; leg C all ten invalid cases exit 2 with `error:`, no `unknown argument`. GATE2-W90 re-read the logs first-hand | **F4 ACCEPTED** |
| R2-11 Lead | **W76 micro-fixes** `d967931` (GATE2-W80 §12.1.1): finish.sh AAC 192k → 256k (192k was the skill's DRAFT rate paired with master video settings; loudness unaffected); HELP exit-code table completed (exit 4 ImageIo was omitted entirely; 3 misdescribed) + stale scenes/film clause corrected | LANDED |
| R2-12 Lead | **W72 VR register update** `3c07564` (§12.1.2, one commit: register + fixtures rebuilt). First dispatch under the new register (at `3c07564`, per GATE2-W90): 3 MATCH / 1 DRIFT / 14 NOT-RENDERED, sole DRIFT = s28, verdict AS EXPECTED (old register reproduced 13 false NEW-SIGNAL labels on the same evidence) | LANDED — steady state under new register |
| R2-13 Lead | **F3-a strip** `2fcffc0` (W74 pack §6, one four-file commit): generator patched, `film/onoda.json` stripped, baseline 196 → 24/9 (fps/frames kept — hard readers; C-10) | LANDED [CI + Tools CI SUCCESS at this head per GATE2-W90 — confirm run ids at close] |
| R2-14 Lead | **W71 §4 splice + W78 runbook paste-ins** `108f33f` (§12.1 items 4+5): FOLDIN-W65.md §4 restored from FOLDIN-ADDENDUM-W71 (independently re-verified 75/75); PC-EVIDENCE-W36 gains the E7 FAIL-evidence package + stop/continue rules (W78 G1/G2) | LANDED [CI SUCCESS at this head per GATE2-W90] |
| R2-15 Lead | **G7 tables in the binary** `3223509` (TABLES-IMPL-W85): `--film-telemetry --tables` — shot/caption/cast tables printed after the verdict line from a separate buffer; default output byte-identical, exit codes unmoved by construction | LANDED — [diff-check vs W85 spec (W93) verdict to append at close] |
| R2-16 Dispatch | **Visual-regression, pre-pin run 43623748951** at `aa92cee` (disposition of §10's first append-promise, the one run §7 never carried): FAILED on the expected UNPINNED pattern (W37 triage: AS-EXPECTED; s28 red by design) | AS EXPECTED |
| R2-17 B9 push | HEAD `9380b01` (Batch 9, gate PASS-WITH-NOTES, GATE2-W90; all seats W81–W89 ACCEPT; W68 note landed after the gate, W77 note still absent) | [CI + Tools CI conclusions from the run list at close] |
| R2-18 Lead | **W68 PC dry-run landed** (PC-DRYRUN-W68.md, committed [in the Batch-10 central commit]): every VM-runnable runbook leg rehearsed — telemetry `CHECK OK` exit 0 observed, golden verify 18 OK / 0 STALE / 0 MISSING, determinism + bench refusal paths as documented. Findings F1 (BLOCKING, runbook binary path — watch items) + F2–F6 (runbook errata, owned by the runbook's revision) | LANDED (VM legs only; no RADV claim) |

### Part C — §8 batch-ledger rows (replace row 8; replace rows 9–10)

| Batch | Seats | Scope (short) | Seat handoffs received | Lead integration | CI |
|---|---|---|---|---|---|
| 8 | W71–W80 | Fold-in addendum, VR register update, knobs phases 2–3 staging + phase-1 verification, F3-a strip pack, G7 telemetry tables, micro-fix verification, exporter acceptance harness, E7 RADV readiness audit, record completeness audit + close skeleton, gate | Gate PASS-WITH-NOTES (GATE2-W80) | W73 proved knobs phase 1 ABSENT → record corrected `4af54c5`, implementation re-based on W73; fixes pass per GATE2-W80 §12.1: micro-fixes ✓ (`d967931`), VR register ✓ (`3c07564`), F3-a strip ✓ (`2fcffc0`), W71 splice + W78 paste-ins ✓ (`108f33f`), DOCFIX2 ✓ (`c7aa648`), W61 splice ✓ (`33c5b2a`); W77 note NOT GATED (absent; tool modified in tree — acceptance stands on the committed harness); W68 landed after Batch 9's gate | committed `021ca01` [CI to fill] |
| 9 | W81–W90 | Knobs-gate acceptance, split amendment pack, s12 adoption readiness, VRAM implementation staging, G7 tables impl spec, lipsync binding pack, locomotion step-0 + decision sheet, F3-b/c closures, v1.0.1 pack refresh, gate | Gate PASS-WITH-NOTES (GATE2-W90); all seats ACCEPT | F4 ACCEPTED (W81, two PASS runs); --tables implemented from W85 (`3223509`, diff-check owed W93); s12 adoption READY (W83), not landed; VRAM staged (W84), not landed; splits amended (W82), not executed; F3-b/c staged (W88), not landed; v1.0.1 pack refreshed (W89) — no-go pending user + bump-SHA gates | committed `9380b01` [CI to fill] |
| 10 | W91–W100 | Wave close: final record audit (RECORD-AUDIT2-W91), close-out report, --tables diff-check, six-decisions brief, goal review, traceability re-run, W68 fix pack, PC session pack, final gates sweep, wave verdict | [gate file + verdict at close] | [this fold-in] | [final HEAD + conclusions at close] |

### Part D — Sections the fold-in must append

1. **Corrections register** — append §3 of this audit
   (C-1…C-17 + the W68 F1–F6 scoring) as a record section,
   so the closed ledger carries its own retractions, not
   only its claims. (W79 §6.4 item 2, carried forward: fold-in
   2 did not add it.)
2. **§10 addendum — dispositions at close:** F1 LANDED
   (`aadc193`); F2 LANDED (A-7); F3-a LANDED (`2fcffc0`);
   F3-b/F3-c STAGED (W88), carried open; F4 **COMPLETE +
   ACCEPTED** (A-11); F5 tool landed (W44), unchanged; F6
   design only, Q1 still open for the user.
3. **User decisions owed at close** (assembled by GATE2-W90
   §11 for one conversation): caption veto (W69 brief;
   recommendation KEEP 73 — a veto stales the 3 in-window
   pins); s28 disposition (re-pin / retire / leave as the
   designed red; W03 recommends against re-pinning);
   locomotion D1–D4 (W87 decision sheet); lipsync Variant A +
   line-3 sign-off sheet (W86); v1.0.1 go (W89 pack — gates
   re-proven on the bump SHA); skill adoption.
4. **Open remainders carried into the goal** (goal
   `goal_b21124e71eb3` stays OPEN past the wave): the PC RADV
   session (E7 leg, PERF tables, bench, VRAM sysfs numeric
   path) — with the W68-F1 runbook fix landed first; the
   fog-chart PSNR leg (Chrome re-capture with the ×π light
   amendment); skill adoption; plus every "staged, not landed"
   item in §5 of this audit, each with its owner.
5. **Final verdict block:** final HEAD; CI + Tools CI +
   knobs-gate + determinism conclusions at that HEAD; VR
   steady state under the new register (s28 sole DRIFT, as
   designed); golden verify count (18 OK / 0 STALE / 0
   MISSING at last check); telemetry `CHECK OK` / exit 0; and
   the wave's seat accounting — 100 seats ordered, actual runs
   including re-runs stated exactly, per the user's counting
   rule (the orchestrator supplies the count; this audit does
   not invent it).

---

*Audit ends. One file written (this one); WAVE2-RECORD.md and
every other existing file untouched; nothing committed.*
