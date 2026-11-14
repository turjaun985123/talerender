# GATE2-W80 — Batch-8 gate

**Seat:** W80 (Batch-8 gate). **Gate HEAD at open:** `33c5b2a`;
**at close:** `9ad2177` — the lead landed four commits inside
the gate window (record fold-in 2 `cca4e6e`, F4 record
correction `4af54c5`, knobs phase 1 `ea98458`, knobs phase 2
`9ad2177`). Where a seat's snapshot and the tree now differ,
this gate says which is which. One file written (this one);
nothing committed; no existing file edited.

## Verdict: PASS-WITH-NOTES

Per-seat: **W71 ACCEPT · W72 ACCEPT · W73 ACCEPT · W74 ACCEPT ·
W75 ACCEPT · W76 ACCEPT · W77 ACCEPT-WITH-NOTES ·
W78 ACCEPT · W79 ACCEPT-WITH-NOTES · W68 (Batch 7) NOT GATED —
file still absent.**

Notes, in order of weight: (1) two headline corrections from
this batch were verified first-hand by the gate and were
already actioned by the lead inside the gate window — W73's
phase-1 record error (the record now corrected at `4af54c5`;
phases 1–2 then landed per the packs) and W74's ratchet
correction (the lowering is 196→**24**, not the briefed 22 —
`fps`/`frames` have hard Python readers). (2) W77's note never
landed; its tool + fixtures are gated first-hand below, its
prose claims are not. (3) W68's PC dry-run file is absent for
the second consecutive batch close. (4) Several W79/W72 marks
are snapshots of `b2e46d2` overtaken by lead landings during
the window — accurate when taken, flagged where superseded.

---

## 1. W71 — fold-in addendum: ACCEPT (gate duty 1, first-hand)

Gate re-ran the acceptance invocation at the current tree:

```
$ python3 tools/format/check_foldin.py
…
OVERALL: PASS (75/75 FOUND, 0 ALTERED-SUSPECT, 0 MISSING)   exit 0
```

The fresh run's tail items sit at the same target lines the
addendum's recorded `b2e46d2` run reports (R-euler target line
1215, R-adopt 1232), and the OVERALL line is byte-identical to
the bar the addendum restores (`OVERALL: PASS (75/75 FOUND, 0
ALTERED-SUSPECT, 0 MISSING)`). HEAD moved past `b2e46d2` since
the seat's run (record edits, knobs landings — none touching
§15 or the draft), and the census still passes: **the
addendum's recorded output matches a fresh run.** The §4 text
itself (bar, exit readings incl. "exit 2 is not a verdict",
re-run triggers, reading rule) is complete and correctly
provenance-tagged; the splice into FOLDIN-W65.md remains the
lead's act (§9 packet).

## 2. W72 — VR register update: ACCEPT (gate duty 5, first-hand half)

Gate fetched run `43625139818`'s artifact via `gh run
download` and ran the **current** tool (old register) on the
real report:

```
Counts: 3 OK · 2 KNOWN-DEBT · 13 NEW-SIGNAL (of 18 rows).   exit 1
s28-onoda-t150  DRIFT 21.65  KNOWN-DEBT
+ both spurious "Register note: … debt appears already cleared"
  lines (ab1-composer, film-s08b) present
```

**The stale labels reproduce exactly as W72 §1/§6 claim.**
The runner verdict on the report is `VISUAL REGRESSION: FAIL
(4 pins, 1 drift)` — s28 the sole drift.

New register's s28 handling, read in the pack (§2/§3/§4):
s28 keeps `{DRIFT, WITHIN-BAND}` = KNOWN-DEBT with a rewritten
action (sole remaining debt; clears only with the separate
human disposition); the runner stays red on s28 alone. The
anti-whitelist structure is sound and is the point of the
pack: the 13 non-canary film ids are registered for
`{NOT-RENDERED}` **only** — any rendered non-MATCH (DRIFT,
WITHIN-BAND, UNPINNED) on them routes to NEW-SIGNAL;
ab1-composer, film-s08b and the dof-chart control carry **no**
register entry, so they have no cover at all; MISSING-RENDER /
REFERENCE-MISSING are NEW-SIGNAL before the register is
consulted. The stated divergence from W37 §6 (keeping the 13
in the register for NOT-RENDERED instead of removing them) is
justified in §5: the runner reports all 18 items every
dispatch, so removal would manufacture 13 false NEW-SIGNALs
per healthy run — the failure the update exists to end.

Not re-run by this gate (the seat's, labelled): the
scratch-copy verification of the patched register (3 OK · 15
KNOWN-DEBT · 0 NEW-SIGNAL, AS EXPECTED, exit 0) and probes
A–D. The pack is patch-ready (Hunks A+B+C + fixture rebuild,
one commit) with a post-landing re-dispatch prescribed.

## 3. W73 — knobs phases 2–3 + phase-1 verification: ACCEPT

The headline is verified three independent ways inside this
batch alone: W73's census (no `Knobs`/`effective_size`/DRAFT
anywhere in `src/` at `b2e46d2`; all four flags fall into
`unknown argument`), W76's sweep line ("no `--res`/knobs flag
… — HELP omits nothing that exists"), and W79's STALE S-1
(same grep, same result). The lead's own correction commit
`4af54c5` confirms register item P6 and voids the 540×960 /
22.35 dB figure's provenance. **The record error is real,
bounded (one relayed claim), and now corrected in the record
itself.**

During the gate window the lead landed phase 1 (`ea98458`)
and phase 2 (`9ad2177`) explicitly per this seat's packs —
with W73 §1.4's same-commit tag warning discharged by
construction (`is_draft()` born with the full three-knob
condition) and the ab1 trap line held (W73 §2.2, re-verified
in the phase-2 commit message). The gate did not re-audit the
landed code; CI + Tools CI are SUCCESS at `ea98458` (§8), and
the landings' own byte-probe gates are the instruments of
record. Phase 3 (`--shadow`, W73 §3 pack: six sites, lockstep
bit-identity at 2048 re-derived) and the full knobs_gate run
— passable only once all three phases land, per W73 §4's leg
table — remain open (§9 packet).

## 4. W74 — F3-a strip pack: ACCEPT (gate duty 2, first-hand)

Gate rebuilt the stripped corpus in a scratch tree (the
tools-ci corpus command verbatim, `film/onoda.json` replaced
by the staged `f3a-strip/onoda.json`) and re-ran the checker:

```
stripped tree:  OVERALL: FAIL (3/5 checks passed, 24 unknown-key
                occurrence(s) in 9 distinct path(s), 7043 tolerated)
live control:   OVERALL: FAIL (3/5 checks passed, 196 unknown-key
                occurrence(s) in 15 distinct path(s), 7043 tolerated)
```

**196 → 24 in 9 paths — the seat's corrected totals, not the
brief's 22.** (FAIL is the checker's standing state; the
ratchet is the gate, per the workflow's own comment.) The
staged baseline file independently summed by the gate: **9
paths, 24 occurrences** ✓. Staged `onoda.json` structure
verified: root keeps `fps`/`frames` (plus anchors, duration,
calendarYears/Dark/Sign, lines, captions); captions carry
exactly `{s, e, text, fit, strip}`; lines exactly
`{start, end}`; 73 captions ✓.

Spot-check of a stripped key's readers, gate's own grep:
`film/gen_assets.py` — a true consumer of the driver
captions — reads `c["text"]` and `c["strip"]` only (lines
174–177); no `["line"]`/`["width"]` subscript reader of the
driver data exists in `src/` or the film/perf/lipsync tools.
The two apparent hits resolve innocently: W75's prototype
uses `c.get("line")` defensively to *compare* stored vs
recomputed assignment (absent key → check skipped, not a
dependency), and `evaluate_track.py`'s `tok["line"]` is on
words-file tokens. The correction itself — root `fps` and
`frames` are HARD-read (`onoda["fps"]`, the frames
cross-check, evaluate_track's exit-2 requirement) by blink /
lipsync / rerender tooling, so W42's "read by nobody" held
only for the Rust loader — is consistent with every reader
the gate touched. Strip list: 6 paths / 172 occurrences;
`fps`/`frames` re-classed F3-a-kept in the staged baseline.
Landing is one four-file commit in the pack's §6 order
(data, generator patch, baseline, STRICTNESS-W42 register
line); the follow-up checker known-set fix (→ 22/7) is a
separate landing, correctly not folded in.

## 5. W75 — G7 telemetry tables: ACCEPT (gate duty 3, first-hand)

Gate ran the prototype on the real film data:

```
$ python3 tools/film/telemetry_tables.py    exit 0
SHOT TABLE — 14 shots, 1811 frames @ 30 fps, 60.36 s
CAPTION TABLE — 73 chunks (audit bars: rendered > 850 px = WIDTH
                fail, fit < 0.68 = FIT fail; strict)
chunk 50:  rend 844  "Onoda still refused"
RECONCILIATION: 73 chunks (0 flagged), 1811 frames, 14 shots,
                7 authored beats bound
RECONCILIATION: all checks passed (…)
```

Headline counts reconcile against the data — **14 shots / 73
chunks / 1,811 frames** — and the gate diffed its full stdout
against the note's §5 verbatim block programmatically:
**byte-identical.** Caption width parity argument (§4.1:
same ink-bbox measure as the binary's audit, stdlib decoder
cross-checked 73/73 against PIL by the seat) and the labelled
deltas (shot-granularity cast frame counts as an upper bound;
the stored `width` field is dead F3-a provenance drifting up
to 16.11 px from the measured value) are honest and
checkable. Findings the tables surface stand: chunk 41
straddles the S06→S07 cut by 1.16 s across a fog→golden
change; the film's whole width margin is 6 px (chunk 50).
Design recommendation (opt-in `--tables`, stdout after the
verdict line, no exit-code interaction) preserves the
verdict-line contract the adapter gates on.


## 6. W76 — micro-fix pack: ACCEPT (read in full; no re-measurement duty)

Both W49 items verified **STILL-OPEN** at `b2e46d2` with
patch-ready corrections: finish.sh:31 `-b:a 192k` → `256k`
(the skill master rate; 192k is the skill's *draft* rate —
the script pairs master video settings with draft audio, and
the loudness analysis is correct: no gain stage exists in the
flow, the delivered MP4 measured −14.00 LUFS *after* the 192k
encode, so the fix is about the "pipeline's own settings"
claim, not loudness) and the HELP `--film` entry's false
"segment scenes resolve from scenes/film/" clause (no src
reader exists; tools-only convention — replacement text
supplied). The sweep's third find is genuine and belongs in
the same edit: **HELP's EXIT CODES table omits exit 4
(ImageIo) entirely and narrows exits 3 and 2** vs
`errors.rs`'s mapping — incomplete, not wrong about
telemetry; replacement section supplied. The sweep's
remaining rows (7 of 8 named paths ACCURATE, adjacent
behaviour claims ACCURATE) are the seat's readings, except
the no-knobs line, independently confirmed (§3).

## 7. W77 — exporter acceptance harness: ACCEPT-WITH-NOTES (gate duty 4, first-hand half)

The seat's note (`EXPORTER-ACCEPT-W77.md`) **never landed**.
What exists: `tools/format/exporter_acceptance.py` + fixtures
(3 cases + stub binary). Gate ran the self-test first-hand:

```
self-test A: identical pair      -> PASS, exit 0
self-test B: planted 1-px shift  -> BELOW-BAND 19.48 dB, exit 1
             (1.5137% px>10 — the W47 §4.5 rider trips)
self-test C: missing reference   -> exit 3 (COULD NOT RUN)
SELF-TEST: PASS (3/3 cases)      exit 0
```

**The harness's fixture behaviour is exactly as designed** —
band 45 dB, determinism leg byte-identical in A/B, the
missing-reference case classified COULD NOT RUN rather than
FAIL. But with no note, the seat's real-binary / ENV-BLOCKED
evidence (the v2-composer local-render wall classification)
is **ungated**: this verdict covers the tool and its
self-test only. The note is owed; the CI real-binary
acceptance run belongs in the Batch-9 definition (§9).

## 8. W78 — E7 readiness audit: ACCEPT (read in full)

Verdict **COVERED with edge gaps** — W36 §1 *is* the
contract's §7 procedure (right instrument, window [100,130),
fresh-root rule, renderer check, exit discipline), so the
one-visit PC session would not miss the RADV leg. The five
gaps are real and correctly scoped: G1 (a FAIL's send-back
lacks the frame evidence/manifests the remote diagnosis
needs), G2 (no operative stop/continue rule after FAIL /
REFUSAL — paste-ready decision text supplied), G3 (the
*contract's* register is stale about E6 — the seat verified
`gh run view 43617370962` = Determinism success @ `3a36f1f`;
the append is the orchestrator's act), G4/G5 (free RADV
support measurements the visit already renders, honestly
labelled as non-discharging per contract R1). Session-order
reasoning (E7 before bench, non-negotiably before the full
film) is sound. The seat's contract/runbook line quotes were
not independently re-verified by this gate (labelled); its
one hard external check (the E6 run) agrees with the record.

## 9. W79 — record audit: ACCEPT-WITH-NOTES (gate duty 6, first-hand samples)

Gate sampled three marks against git and the record:

- **PRESENT — confirmed.** `3a36f1f`: record §7 line 223
  carries the "B1 push" row with both run ids and SUCCESS /
  SUCCESS, as marked.
- **ABSENT — correct at audit HEAD, superseded since.**
  `3a3a427` (Batch-6 central commit): the seat's grep of the
  record returned 0 hits at `b2e46d2`. Inside the gate window
  the lead's fold-in 2 (`cca4e6e`) recorded the post-B6
  history — the gate's grep now returns 2 hits. The mark was
  accurate when taken; the absence it named has been partly
  remedied by the very fold-in the audit's §6 skeleton
  prescribes.
- **STALE — correct at audit HEAD, corrected since.** S-4
  (Act-2 provenance cite): record line 239 **now** reads
  "ab1 fresh render at `dc0e555` (run 43621168728)" with an
  explicit "[Cite corrected per W63 §3 … this row is the
  correction of record]" annotation — fold-in 2 applied the
  audit's §6.1 amendment 3 (and the corrective-row amendment
  for S-3). At audit time the `9f497c7` text stood, as quoted.

Two framing notes. (a) The audit's commit range was 37
commits at `b2e46d2`; at gate close `git log 6f00400^..HEAD`
counts **41** (fold-in 2, record correction, knobs phases
1–2, tail splice) — the audit is a labelled snapshot and
says so. (b) Its structural finding survives the remedy:
of the wave's 8 corrections, only C-1 reached the record
before fold-in 2 because fold-ins sweep seat headlines, not
commit messages and gate notes — the close-out fold-in must
sweep all three (§10 packet). The §5 remainders table and
§6 skeleton remain the close-out's working basis, with the
knobs row updated by §3 of this gate (implementation now
phases 1–2 landed, phase 3 open) and the F2 row already
re-marked LANDED by fold-in 2 / the correction commit.

## 10. W68 (Batch 7, late) — NOT GATED, continued absence

`docs/integration/PC-DRYRUN-W68.md` is absent at this gate's
open, mid-window, and close — the second consecutive batch
close without it. The seat was still running at Batch-7
close; nothing has landed since. PC-session readiness
therefore rests on W36's runbook + W64 §5's bench addendum +
W78's E7 audit (all gated in their own batches) — the
session pack is *audited* but still *unrehearsed*. Record
the absence at wave close; the dry-run remains worth landing
before the PC visit, not before the wave can close.

## 11. Hygiene + CI at gate close

**Hygiene.** Every Batch-8 seat deliverable is a new,
untracked file (9 notes + `f3a-strip/` + 2 tools + W77's
fixtures); no seat edited a tracked file. The one tracked
modification in the tree, `docs/integration/WAVE2-RECORD.md`,
is the lead's own lane (fold-in work, concurrent with its
commits `cca4e6e`/`4af54c5`). Untracked `__pycache__` dirs
under `tools/` are pre-existing noise, not seat artefacts.

**CI (gh run list at close).** HEAD `9ad2177`: CI + Tools CI
+ Render dispatch **in progress**. At `ea98458` (knobs phase
1): CI **SUCCESS**, Tools CI **SUCCESS**. At `4af54c5`: CI
SUCCESS. `c7aa648` shows one CI **cancelled** — concurrency
supersession by the next push, not a failure (the standing
distinction). Bench run `43623743480` SUCCESS (Batch 7);
visual-regression steady state = s28 sole DRIFT (§2). No
workflow is red at any head the gate observed.

---

## 12. Integration packet for the lead — the FINAL phase, ordered

### 12.1 Fixes-pass remainder (all patch-ready, all gated)

1. **W76 micro-fixes** — finish.sh `192k`→`256k`; HELP
   `--film` entry replacement; HELP EXIT CODES replacement
   (adds exit 4, widens 3/2). One commit.
2. **W72 register patch** — vr_triage Hunks A+B+C + the
   fixture rebuild (steady-state + re-isolated planteds +
   the new anti-whitelist fixture) in **one** commit, then
   one VR re-dispatch: expected triage 3 OK · 15 KNOWN-DEBT ·
   0 NEW-SIGNAL / AS EXPECTED over a runner FAIL on s28
   alone.
3. **W74 F3-a strip** — one commit, four files, the pack's
   §6 order. Baseline lowers to **24 / 9 paths** (not 22 —
   §4). Post-landing: tools-ci strictness green at 24/9 +
   regeneration no-op diff. The checker known-set fix for
   `fps`/`frames` (→ 22/7) is a separate later landing.
4. **W71 §4 splice** into FOLDIN-W65.md (delete truncated
   tail, append the addendum's §4; gate-verified, §1).
5. **W78 paste-ins** — G1+G2 into PC-EVIDENCE-W36; G4/G5
   support steps; G3's contract §5 E6 append is the
   orchestrator's act, flagged here.
6. **DOCFIX2-W67 queue** — still the live doc-fix list;
   nothing from W38 was ever applied (W67/W79 agree).

### 12.2 Batch 9–10 definition input

- **Knobs phase 3** (`--shadow`, W73 §3) → then the **full
  knobs_gate** (legs A+B+C become passable only with all
  three phases; W73 §4). F4 closes there.
- **Split programmes**: 22 pure-move commits (W21 9 + W22
  13) under W66's updated cites, G-TEL texts re-worded to
  the standing `CHECK OK` / exit 0 first (W79 C-8).
- **s12 bake adoption**: baked data + code-path retirement
  in one commit (W52).
- **VRAM implementation** (W56 design); **G7 tables in the
  binary** (W75 design + prototype; adapter grid re-run with
  `--tables` on, expecting 8/8 unchanged).
- **Lipsync binding semantics** (lead decision: step vs
  linear, W59) and **locomotion registration step 0** (W57)
  — both pixel-frozen until their sign-offs.
- **F3-b citing pass; F3-c `materials` either/or** — overdue
  (W79 S-8: the §15 fold-in landed without the resolution).
- **Exporter real-binary acceptance in CI** (W77 tool gated;
  its seat note still owed) with the ENV-BLOCKED classes
  exercised on the runner, not this VM.

### 12.3 Wave-close requirements

1. **Final WAVE2-RECORD fold-in** per W79 §6, sweeping
   commit messages + gate notes (not seat headlines only):
   amendments §6.1 (several already applied by fold-in 2 —
   re-verify, don't re-apply), new §7 rows incl. the two
   discharged append-promises (bench + VR verdicts, §12.1.2
   above) and the errors-step-2 gate dispatches (R-14 —
   append checked conclusions or record them as never
   checked), the **corrections register C-1…C-8** as a record
   section, and the final verdict block (final HEAD, CI +
   Tools CI conclusions there, seat accounting stated as
   actual runs incl. re-runs).
2. **v1.0.1 pack refresh** (W46 pack re-based to the final
   HEAD): version bump, changelog extended, all gates
   re-proven on the bump SHA; the v1.0.0 proof MP4 is NOT
   re-attached. Release itself needs the user's go.
3. **User decisions owed at close** (unchanged owners):
   caption veto (W69 brief; recommendation KEEP 73 — a veto
   stales 3 Act-1 pins); **s28 disposition** (re-pin /
   retire / leave red — §2 keeps it red-labelled until
   then); locomotion sign-off (W57 pack, user-eye); skill
   adoption; v1.0.1 go.
4. **Carried past the wave** (goal stays open on these):
   the PC RADV session — E7 pack (W78: covered, G1/G2
   paste-ins pending), bench legs, PERF tables — and W68's
   unrehearsed dry-run (§10).

*Gate ends. First-hand evidence: §1 (fold-in re-run), §2
(old-register triage on the real run-43625139818 report), §4
(stripped-corpus checker re-run + reader spot-check), §5
(telemetry_tables run + byte-diff), §7 (harness self-test),
§9 (three record/git samples), §11 (run list). Everything
else is labelled as the seat's.*
