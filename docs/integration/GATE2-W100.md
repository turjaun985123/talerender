# GATE2-W100 — Batch-10 gate + WAVE VERDICT (Wave 2, final seat)

**Seat:** W100 — Batch-10 gate and the wave verdict.
**Gate HEAD:** `cc38b03` ("W77 + W68 notes land (late seats)…"),
the current `main` HEAD at gate time.
**Method:** every duty below was executed first-hand by this seat —
`gh run list/view --log-failed`, `git log/show/diff`, source reads,
a local golden-pack verify, a caption count, a baseline-file read,
an arithmetic reproduction, and a re-run of the exporter harness's
self-test. Anything this seat did **not** re-measure is labelled
"the seat's" where relied on. This file is the only file written
by this seat; nothing was committed; no existing file was edited.

---

## §1 Duty 1 — W92 close-out: five load-bearing claims spot-verified

W92's draft (`WAVE2-CLOSEOUT-W92.md`, written at `9380b01`) was
checked against primary evidence, not against the wave record:

1. **Caption gate line (§2, Phase 1: "prints `FILM TELEMETRY:
   CHECK OK` and exits 0, commit `5e90d7e`").** Verified in its
   parts: commit `5e90d7e` exists and its message records the
   telemetry re-run at the 73-chunk data with that exact verdict
   line and exit 0 (the lead's claim, in the commit); this seat
   counted **`film/onoda.json` = exactly 73 captions** first-hand
   at the gate HEAD; the verdict line + exit 0 were independently
   *observed from the CI binary* by W68's dry-run (the seat's
   evidence, PC-DRYRUN-W68 §3.1, corroborated by W99 §8). The
   threshold (850 px) is untouched in the audit code per W97's
   row C. **Claim stands.**
2. **Corrections register, entry 1 (the knobs false claim).**
   Verified first-hand: `4af54c5` exists — the record-correction
   commit whose message quotes W73's airtight disproof (no Knobs
   struct / no parse arms / all four flags rejected at `b2e46d2`);
   the real implementation commits `ea98458` / `9ad2177` /
   `76b922c` all exist with substantive messages ("the landing
   whose absence W73 proved" … "F4 IMPLEMENTATION COMPLETE");
   and the acceptance — knobs-gate runs `43628289244` +
   `43628339967` at `c440d90`, **both success** — was read from
   `gh` by this seat. **The register's worst entry is accurate,
   including its resolution.**
3. **Golden pack (§6: "18 OK / 0 STALE / 0 MISSING").** This seat
   re-ran `tools/golden/make_golden.py` against the gate-HEAD
   tree: **`18 OK / 0 STALE / 0 MISSING of 18 golden items`**.
   First-hand. **Claim stands.**
4. **F3 ratchet (§2: strip landed `2fcffc0`, 196 → 24).** This
   seat read the committed baseline file
   `tools/format/unknown_fields_baseline.json` at HEAD:
   `totalOccurrences: 24`, `distinctPaths: 9`,
   `toleratedOccurrences: 7043`. Commit `2fcffc0` exists with the
   strip in its subject. **Claim stands.**
5. **v1.0.1 no-go (§2/§6: "no-go yet… waits on your go, the wave
   freeze, and those gates").** Verified first-hand:
   `Cargo.toml:3` still reads `version = "1.0.0"`, and
   `gh release list` shows exactly one release, **v1.0.0**.
   **Claim stands** — the release genuinely has not happened and
   nothing code-side misrepresents it.

**Supersessions since W92's snapshot (not W92 errors — the draft
says what was true at `9380b01` and flags its own precondition):**
its §6 "HEAD is CI-red… fixing it is a precondition for calling
the wave closed-green" has been **discharged** — the fix landed
test-only at `2f5c708`, CI is green there and at the gate HEAD
(§3 below), so the draft's own closing precondition is now met;
its "last head with both CI and Tools CI SUCCESS is `2fcffc0`" is
superseded by `2f5c708` and `cc38b03`; its §4 "W77's exporter
note is still owed" is superseded by the note's landing at
`cc38b03` (§2 below). Finalising the draft at the true closing
HEAD is a closing act (§9), not a defect in the draft.

## §2 Duty 2 — W97 recount + the W77 note gated

**Recount.** This seat re-tallied TRACE2-W97's tables row by row:
§§1–8 = 108 counted rows (LANDED 61 · STAGED 18 · DECISION-OWED 8 ·
REJECTED 7 · STALE-CLAIM 7 · UNRESOLVED 7); §9 = 17 rows (LANDED 13 ·
STAGED 1 · UNRESOLVED 3, matching its printed subtotal). Grand
total **125 rows — LANDED 74 · STAGED 19 · DECISION-OWED 8 ·
REJECTED 7 · STALE-CLAIM 7 · UNRESOLVED 10**, and
74+19+8+7+7+10 = 125. **W97's count table reproduces exactly**
from its own tables under its stated convention (lettered
sub-rows count once; 18 cross-ref sub-rows uncounted).

**W77 note — gated by this seat (W97's postscript condition).**
`EXPORTER-ACCEPT-W77.md` landed at `cc38b03` with the final
harness. This seat read the note in full and re-ran its central
claim: **`python3 tools/format/exporter_acceptance.py
--self-test` → `SELF-TEST: PASS (3/3 cases)`** on the committed
final code, first-hand. The note's classifications are honest
throughout: the local run's overall verdict is COULD NOT RUN
(exit 3), the ab1 20.30 dB is explicitly read as calibration
against the recorded 20.152 dB §28 anchor — not as an exporter
failure (no exporter exists) — the binary's pre-Wave-2 vintage
is disclosed, and its three deviations from brief are declared
in its §5. **The note is accepted.** W97 row W-15 therefore
closes as its postscript prescribes: **adjusted wave totals —
LANDED 75 · UNRESOLVED 9** (of 125). What remains owed under
this item is only the CI real-binary acceptance run (the note's
§4 dispatch shape) — carried into the waivers (§8, W-8).

## §3 Duty 3 — W99's §9 action list vs the gate HEAD

W99's sweep was taken at frozen HEAD `9380b01`. Re-run by this
seat (`gh run list`, gate time):

| W99 §9 item | State at gate HEAD `cc38b03` |
|---|---|
| 1. CI red at `9380b01` (1 test) | **CLOSED.** Fix `2f5c708` (test-only, §4); CI `43630186620` @ `2f5c708` **success**; CI `43630790697` @ `cc38b03` **success**. `git diff 2f5c708..cc38b03 -- src/` is **empty** — both green runs judge the same source tree. |
| 2. No binary artifact at HEAD | **CLOSED.** Run `43630790697` carries `talerender-binary` + `talerender-frame` artifacts (read from `gh run view`, first-hand). |
| 3. Tools CI unrun at HEAD | **CLOSED.** Tools CI `43630186614` @ `2f5c708` success; `43630790659` @ `cc38b03` success (the commit touched `tools/**`). |
| 4. Knobs unrun at HEAD | **Open, as listed.** No dispatch after the `c440d90` pair; latest evidence remains the two PASS runs there. |
| 5. Determinism unrun at HEAD | **Open, as listed.** Latest: `43624016907` @ `166873c`, success, `DETERMINISM: PASS` (the seat's log reading, W99 §4). |
| 6. VR unrun at HEAD | **Open, as listed.** Latest: `43628892930` @ `3c07564`, failure-as-designed, sole DRIFT s28 (W99 §5, log-verified there). |
| 7. Bench unrun at HEAD | **Open, as listed.** Latest: `43623743480` @ `aa92cee`, success. |
| 8. Telemetry unobserved at a `--tables` head | **Open.** The binaries now exist (§2 of W93's §F: runs `43629313479` + `43630186620`), but the §3 byte-stability sequence and §6.2 acceptance numbers remain **unexecuted** — carried into the waivers (§8, W-7). |

**One run W99 predates — a NEW red at the gate HEAD.** CI for
`c9b4d86` (`43630693741`) was **cancelled** (concurrency
supersession by the `cc38b03` push — not a failure; its tree's
`src/` is identical to the green `cc38b03` tree, per the empty
diff above). But the **stroke-battery dispatch `43630694752`**
— the first ever run of the `battery.yml` wired at `c9b4d86` —
completed **failure**: the battery ran (Build ✓, battery step ✓)
and its verdict step reported **`BATTERY: FAIL (7/46 cards)`**
("cards written: 7"; log read first-hand by this seat). The
split programme's precondition therefore stands refined: the
standing invocation now **exists** (committed at `c9b4d86`),
but it has **not yet reported 46/46** — its first verdict is
7/46, cause not yet diagnosed (workflow counting vs battery
output shape is the lead's to determine). Split steps S2/S4/S5
remain blocked, now on a red leg rather than a missing one.

## §4 Duty 4 — W93 seam-4 arithmetic, independently confirmed

This seat re-derived W93's §A from the source at the gate HEAD:

- `blink_at` (src/film.rs:257–258): `t - b >= 0.0 && t - b < 0.12`.
  `beat_frames` (src/telemetry.rs:1231–1238): counts `n` with
  `d = n/30 − beat`, `(0.0..0.12).contains(&d)`. **The predicates
  are identical.**
- This seat's own arithmetic (IEEE doubles): beat **4.20** →
  frames {126, 127, 128, 129} = **4** in both windows; beat
  **14.78** → **0 over 0..200** (the window ends at t ≈ 6.63 s;
  the beat's first frame is 444) and **3 over 0..1811**
  ({444, 445, 446}). Exactly W93's §A.3.
- `git show 2f5c708`: **test-only** (one file, +5/−2 — the range
  0..200 → 0..600 in the test plus a comment naming this root
  cause); the implementation is untouched; CI at that head is
  green at 180/0 (§3).

**Confirmed: the test deviated — inheriting the spec sentence's
impossible window/expectation pair (TABLES-IMPL-W85 §6.1 item 4);
the implementation is an exact transcription of `blink_at` and
the tie the seam exists to prove holds at every window.** The
§6.1 erratum remains owed (waivers, §8 W-7).

## §5 Duty 5 — W91: an ABSENT commit, verified

W91 marks the whole F4 implementation series and the fixes-pass
landings ABSENT from WAVE2-RECORD.md. This seat verified three of
its absence claims first-hand against the record at the gate HEAD:

- `grep "3223509" WAVE2-RECORD.md` → **0 hits**; the commit
  exists in `git log` ("G7 tables in the binary"). ABSENT ✓.
- `grep "2fcffc0" WAVE2-RECORD.md` → **0 hits**; the commit
  exists ("F3-a strip… baseline 196 -> 24/9"). ABSENT ✓.
- The five F4-series hashes together (`ea98458`, `9ad2177`,
  `76b922c`, `2017e86`, `c440d90`) → **0 hits in total**, and the
  record's §10 F4 block still reads (line ~398, wrapped)
  "SPEC + PATCH PACKS COMPLETE; IMPLEMENTATION NOT STARTED
  (CORRECTION, 2026-11-15)" — W91's NS-1, confirmed in substance
  at the gate HEAD: the correction of the wave's worst record
  error now itself states the opposite of the landed truth, and
  only W91's paste-ready close fold-in (§6 of its note: 13
  amendments + 18 §7 rows + corrections C-1…C-17) repairs it.

W91's headline (3/16 PRESENT, 1 PARTIAL, 12 ABSENT over
`b2e46d2..9380b01`; the record accurate through `b2e46d2`) is
consistent with everything this seat sampled. The final fold-in
is a closing act (§9), still owed at gate time.

## §6 Duty 6 — Hygiene + final seat count

- **Working tree at gate time:** the only untracked items are
  the nine Batch-10 seat files awaiting the central Batch-10
  commit (RECORD-AUDIT2-W91, WAVE2-CLOSEOUT-W92,
  TABLES-VERIFY-W93, DECISIONS-W94, GOAL-REVIEW-W95,
  DRYRUN-FIX-W96, TRACE2-W97, PC-SESSION-W98, GATES-SWEEP-W99)
  plus `__pycache__` noise. No tracked file is modified. HEAD
  `cc38b03` is the pushed `main` head (the orchestrator's push
  of that commit is in this gate's evidence).
- **Batch gates:** `GATE2-W10`, `W20`, `W30`, `W40`, `W50`,
  `W60`, `W70`, `W80`, `W90` all exist in `docs/integration/`;
  this file is the tenth.
- **Seat count: 100 / 100.** Ten batches × ten seats; every
  seat's deliverable exists in the tree, including the two late
  landings — **W68** (PC-DRYRUN-W68.md) and **W77**
  (EXPORTER-ACCEPT-W77.md + the final harness) — both committed
  at `cc38b03`, closing the wave's only two cross-batch debts.

## §7 Per-seat verdicts — Batch 10 (+ the two late seats)

| Seat | Verdict | Basis (first-hand items in bold) |
|---|---|---|
| W91 record audit 2 | **ACCEPT** | **Absence claims re-verified (0-hit greps ×3, §10 line read)**; fold-in spec is paste-ready |
| W92 close-out draft | **ACCEPT-WITH-NOTES** | **Five load-bearing claims verified (§1)**; its §6 red-CI statement was true at its snapshot and is **superseded** at the gate HEAD — the draft must be finalised at `cc38b03` (§9), which its own precondition now permits |
| W93 tables verify | **ACCEPT** | **Seam-4 arithmetic + source + test-only diff independently confirmed (§4)**; its UNVERIFIED items (§3/§6.2 runs) are carried into the waivers, not hidden |
| W94 decisions brief | **ACCEPT** | Six decisions, each self-contained with labelled recommendations + an answer sheet (read first-hand); ordering note (1 & 2 before 5) is sound |
| W95 goal review | **ACCEPT** | Line-by-line reconciliation; its one time-sensitive caveat (CI `43629817118` "in progress, no conclusion claimed") has since resolved as §3 records; its recommendation — goal stays OPEN — is adopted in §8 |
| W96 dry-run fix pack | **ACCEPT-WITH-NOTES** | **F1 defect lines verified verbatim at HEAD** (runbook :88 provisions `~/talerender-bin`, :130 invokes `./talerender-bin/talerender`); canonical side (`~/talerender-bin` + `TALERENDER_BIN`) justified against all path-touching docs. Note: its §8 W77 **waiver is MOOT** — the note landed (§2); the waiver text must **not** be folded into the record |
| W97 traceability 2 | **ACCEPT** | **Count table reproduced exactly by re-tally (§2)**; postscript adjustment verified and applied (75/9) |
| W98 PC session pack | **ACCEPT-WITH-NOTES** | Structure, stop rules and closure scope verified against its sources; its two-tier VRAM step honestly reflects the unlanded implementation. Execution still depends on W96's F1 patches landing first (or the pack's inline workaround) — the runbook defect is **live at the gate HEAD** |
| W99 gates sweep | **ACCEPT-WITH-NOTES** | Accurate at its frozen HEAD; §9 items 1–3 are now **closed** (§3), items 4–8 stand, and one new row postdates it (battery, §3) |
| W68 PC dry-run (late, Batch 7) | **ACCEPT** | Legs corroborated by W99 §§7–8 and by **this seat's own golden re-run (18 OK)**; its F1 finding **re-verified present at HEAD** |
| W77 exporter note (late, Batch 8) | **ACCEPT** | **Self-test re-run by this gate: PASS 3/3** on the committed final harness; classifications and deviations honest (§2). The CI real-binary acceptance run remains owed (waiver W-8) |

No seat is rejected. No Batch-10 seat edited an existing file
or committed anything.

## §8 THE WAVE VERDICT — **COMPLETE-WITH-WAIVERS**

Wave 2 (100 seats, 10 batches, the external audit's Phases 1–5 +
§6 + findings A–E + gap-fixes F1–F6, code first then fixes) is
**complete**: every seat delivered, every batch gate exists, the
traceability matrix accounts for all 125 rows with nothing
unexplained, and CI + Tools CI are **green at the closing HEAD**
(`cc38b03`; CI `43630790697`, Tools CI `43630790659`, binary
artifact present). It is not *unconditionally* complete: the
items below were prepared to landing shape but not landed, or
proven in part but not in full. They are waived **by name** —
deferred to the lead's post-wave queue or to the user's
decisions — not silently dropped.

**The four-way distinction the verdict rests on:**

- **Code delivered.** Phase 1 in full (caption re-chunk with the
  gate green on data, telemetry exit codes, both golden-pin acts
  + verified corrective, determinism contract + CI leg);
  gpucore 5/5 (finding B closed); structured errors both steps;
  §15 fold-in + reservation guard (75/75 census); F1, F2
  (complete, zero frame impact), F3-a strip (196 → 24), F4 knobs
  (independently accepted, two PASS gate runs), F5 sheet tool;
  blink adoption; G7 `--tables` in the binary (diff-checked,
  §4); paint byte budget + run log; the module set (anim,
  charanim, rendergraph, atlas, bufferpool, locomotion,
  cameramoves, lipsync track, s12 bake). W97 adjusted totals:
  **75 LANDED / 19 STAGED / 8 DECISION-OWED / 7 REJECTED /
  7 STALE-CLAIM / 9 UNRESOLVED of 125 rows.**
- **Evidence delivered.** Every landing above carries its gate
  or CI proof in the batch gates W10–W100; the closing evidence
  table (W99, updated in §3) shows each workflow's latest verdict
  at its own head, labelled by head; the corrections register
  (W92 §3, W97 §9) records the wave's own errors — the knobs
  false claim first — with their disproofs and fixes.
- **Decisions owed — the user's.** Six, assembled in
  DECISIONS-W94 with labelled recommendations: (1) caption veto
  (KEEP 73 recommended); (2) s28 disposition (do-not-re-pin is
  the packs' firm position; retire vs leave-red); (3) locomotion
  D1–D4 + adoption; (4) lipsync Variant A + line-3 sign-off;
  (5) v1.0.1 go; (6) skill adoption (now, or after RADV proof).
  A seventh, smaller user item rides along: F6's Q1 (the exact
  cue entry shape). Two further decisions are the lead's/format's
  (F3 general hard-error; G1c data-driven driver).
- **The goal.** Per W95's review, adopted here:
  **goal_b21124e71eb3 stays OPEN past the wave.** Its close
  condition — the remainders land or are formally waived — is
  not met: no RADV evidence exists, the fog-chart leg is
  unmeasured, adoption is undecided. The minimal closing path
  is W95 §8: the decisions conversation → the prepared landings
  → one scripted PC RADV session → the fog-chart leg executed
  or waived on the record.

**Waivers (each names what is deferred and where its pack lives):**

- **W-1 Splits execution.** The 22-commit film.rs/scene.rs split
  programmes are amended and ready (blueprints applied at
  `c9b4d86`), but **not one split commit has landed**. The
  battery precondition's mechanism landed with them, yet its
  first dispatch is **red (7/46 cards, §3)** — S2/S4/S5 stay
  blocked until a dispatch reports 46/46 on the pre-split tree.
- **W-2 VRAM implementation.** Formula proven byte-exact three
  ways (220,930,688 B steady / 229,286,528 B peak); the
  `gpu-mem:` line is **not in `src/`** (VRAM-IMPL-W84 staged,
  with its two lead rulings pending).
- **W-3 s12 bake adoption.** PROVEN + READY, zero drift
  (S12-ADOPT-W83); the one data commit + film.rs:2021–2023
  retirement + probes 1666–1810 have **not landed**.
- **W-4 F3-b/c landings.** Dispositions staged (F3BC-W88:
  19 CITE→strip; materials STRIP-via-decomposition; `near`
  CONSUME); the landing that lowers the baseline 24 → 3 has
  **not landed**.
- **W-5 §15 evaluator step 2.** Spec + reservation guard are
  live; the loader still does not consume clips/prefabs
  (guard-only), by the guard's own source comment.
- **W-6 Lipsync binding.** Variant A recommended and measured
  (LIPSYNC-BIND-W86); the binding itself is unlanded, pending
  the lead's semantics choice and the user's line-3 sign-off.
- **W-7 `--tables` proof tail.** §3 byte-stability + §6.2
  acceptance runs **never executed** (both binaries sit in CI
  artifacts — runs `43629313479` + `43630186620` — so this is
  an afternoon's execution, not research); plus the owed
  erratum to TABLES-IMPL-W85 §6.1 item 4 (the sentence whose
  impossible window caused the wave's last CI red, §4).
- **W-8 Exporter acceptance, real-binary leg.** Harness +
  fixtures + note landed and gated (§2); the CI real-binary
  acceptance run (the note's §4 dispatch shape) has **never
  been executed**, and the exporter itself remains unbuilt
  (EXPORTER-W47 spec + harness are the delivered artefacts).
- **W-9 Runbook F1 fix.** W96's paste-ready patches are
  prepared, but the defect is **live at the closing HEAD**
  (verified §7/W96 row): the PC session must not be run from
  the unpatched runbook — W98's inline workaround or the patch
  landing must come first.
- **W-10 Record + close-out finalisation.** WAVE2-RECORD is
  stale for everything after `4af54c5` (§5); W91's paste-ready
  fold-in and W92's finalisation at the closing HEAD are the
  closing acts in §9. Until they land, this gate + the batch
  gates are the record of Batches 8–10.
- **W-11 RADV / goal remainders.** The PC RADV session (E7 leg,
  PERF tables, adapter proof, sysfs VRAM numerics), the
  fog-chart ≥45 dB leg (needs the ×π-amended Chrome
  re-capture), and the skill-adoption decision — all per W95
  §7; the goal stays open on them (§8, goal bullet).
- **W-12 Residual UNRESOLVED rows (W97, adjusted).** Beyond
  W-11's RADV rows: the auto-chunk transcription has no owner
  (G4b — `gen_film_data.py:75` still refuses); the literal
  caption-isolated ≥40 dB test was never run (R3c); no native
  headless-chicken render exists (R5b); no `Cargo.lock` /
  two-build identity proof exists (D5); the one-off painter
  panic was never reproduced (W-16). Each names its missing
  evidence in TRACE2-W97; none blocks the wave verdict, and
  none may be quoted as done.

## §9 Closing acts owed (lead / orchestrator, post-gate)

1. Apply W91 §6's fold-in to WAVE2-RECORD.md (13 amendments +
   18 §7 rows + the C-1…C-17 corrections section), with the
   final verdict block filled at the true closing HEAD — and
   **without** W96 §8's W77 waiver text, which events have
   overtaken (§2).
2. Finalise WAVE2-CLOSEOUT-W92.md at the closing HEAD (its §6
   updated per §1's supersessions) and lift its DRAFT mark.
3. Central Batch-10 commit: the nine seat files + this gate.
4. Diagnose the battery leg's 7/46 (W-1) before any split step
   is scheduled.

*End of GATE2-W100. Wave 2: 100/100 seats, 10/10 batch gates,
verdict COMPLETE-WITH-WAIVERS (W-1…W-12). Goal
goal_b21124e71eb3 remains OPEN per W95 §8.*
