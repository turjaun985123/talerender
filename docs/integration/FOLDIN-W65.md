# FOLDIN-W65 — the §15 fold-in diff-checker

**Seat:** TaleRender Wave 2, W65 (Batch 7, replacement run — the
first run died on a transport error before producing anything).
**Date:** 2026-11-14. **Status:** tool + fixtures DONE and proven
(§2–§4). Nothing committed; no existing file edited.

**Deliverables**

| Piece | Path |
|---|---|
| Fold-in diff-checker | `tools/format/check_foldin.py` |
| Self-test fixtures (mini draft, folded-full, folded-altered, runner) | `tools/format/fixtures/foldin/` |
| This note | `docs/integration/FOLDIN-W65.md` |

**Timing, stated plainly:** the brief was written against HEAD
`3a3a427`, when the fold-in was still the lead's future work. By
the time this replacement run executed, the lead had already
landed it — commit `4dd6530` ("scene-format s15 fold-in (W31
draft, paste-ready text verbatim) + s15.10 adoption step 1").
So this note records **both** runs the checker exists to make:
the pre-fold baseline (§3, reconstructed exactly from git at
`3a3a427`) and the run against the current document (§4), which
is therefore not a rehearsal but the **actual post-fold
acceptance run** for the `4dd6530` fold-in. It passes §4's bar.

---

## 1. What the tool checks — and what it does not

`check_foldin.py` judges the **presence of normative content**,
nothing else. Not prose quality, not style, not whether a rule
is a good rule. It answers one question: *did every clause of
the draft survive the fold?*

**Census** (from the draft's paste text — the region between
its two standalone `---` rules, draft lines 36–735; a draft
without such a pair is read whole):

| Class | What | Count for the W31 draft |
|---|---|---|
| H | every numbered heading (`## 15.`, `### 15.1` … `### 15.10`) | 11 |
| N | every normative sentence — containing a keyword from the set MUST / MUST-NOT (`must not`, `may not`) / NEVER / ALWAYS / FORBIDDEN / HARD-ERROR / REQUIRED, deduplicated | 46 |
| R | every named rule from the tool's curated table, reconciled with FORMAT15-CHECK-W51's disagreements D1–D10 (draft wins where they bite — e.g. R-colour checks the draft's authored `#RRGGBB` spelling plus the evaluator-side linear `[f32; 3]` lane, per D5) | 18 |
| | **Total census** | **75** |

The 18 named rules: reservation guard; guard reaching
`film.rs`; no shadowing; pid stripped at expansion; verbatim
substitution; overrides deep-merge / params replace wholesale;
transforms compose, never replace; cycles hard-error printing
the path; the 64 depth guard; `grade.time` rejected by name
(R3); the poseTrack two-key conversion; the fire-flicker R1
basis 4.6 exclusion; the `Color` lane; the step side channel;
the ease remap (`key[i+1].ease → key[i].ease`); duration on the
clip / wrap on the instance; euler-only rotation; the §15.10
adoption order.

**Matching rule** (the tool's whole contract; full text in its
docstring):

- Comparison text is normalised only for matching: lowercased,
  markdown stripped, tokenised, stopwords removed. Reported
  line numbers are the target's real lines (start line of the
  paragraph / list item / table row a sentence came from).
- **Headings:** FOUND if the target has the same number with
  ≥ 0.6 title-token containment; ALTERED-SUSPECT if the number
  exists with a divergent title; MISSING if the number is
  absent.
- **Normative sentences:** containment = share of the source
  sentence's content tokens present in the best-matching target
  sentence (same-section candidates win ties within 0.02 —
  reorganisation across sections is tolerated, and the reported
  line shows where the clause landed). ≥ 0.75 with no strong
  keyword lost → FOUND; ≥ 0.75 with a strong keyword (NEVER /
  ALWAYS / FORBIDDEN / HARD-ERROR / MUST-NOT) lost →
  ALTERED-SUSPECT (a weakening, flagged as such); between the
  floor and 0.75 → ALTERED-SUSPECT; below the floor → MISSING.
  The floor is 0.40, raised to **0.50 for sentences of ≤ 6
  content tokens** — see the self-test note in §2 for why.
- **Named rules:** each rule is phrase groups; a group is
  satisfied when one alternative phrase's content tokens all
  appear in a single target sentence (same section preferred).
  All groups → FOUND; at least half → ALTERED-SUSPECT; else
  MISSING.

**False-negative behaviour (known and accepted):**

- A normative sentence carrying **none** of the keywords is not
  an N-item. It is covered only if it is also a named rule —
  several are (e.g. "Substitution is verbatim — no arithmetic"
  is keyword-less; it is census item R-subst, not an N-item).
  A keyword-less, unnamed normative sentence in a future draft
  would be invisible to this checker.
- A clause reworded past the containment floor reads MISSING
  even if a human would call it the same rule. The checker is
  a tripwire for review, not a semantic judge.
- In the other direction: a target sentence keeping ≥ 75% of a
  clause's tokens and its strong keywords reads FOUND even if
  a qualifier elsewhere in its paragraph was cut. Paragraph
  context is not compared.

Exit codes (repo convention): **0** every item FOUND; **1**
any ALTERED-SUSPECT or MISSING; **2** input error (`ERROR:`
line on stderr, no report). `--rules=off` skips the R census
(it is §15-specific) — used by the self-test's mini draft.

---

## 2. Self-test — synthetic fixtures (verbatim results)

`tools/format/fixtures/foldin/`: `mini-draft.md` (a §15-shaped
mini draft: 3 headings + 6 normative sentences inside its own
`---` paste region, plus a normative-looking trap sentence
*outside* the region that must not be counted), `folded-full.md`
(all clauses present), `folded-altered.md` (the cycle clause
dropped; "Instances **never** carry" weakened to "usually"),
and `run_tests.py`:

```
  PASS  folded-full: all clauses FOUND, exit 0
  PASS  folded-altered: dropped=MISSING, weakened=ALTERED-SUSPECT, exit 1
  PASS  missing target: exit 2, ERROR line, no report
SELF-TEST: PASS 3/3
```

The altered target's two flags, verbatim from the tool:

```
  ALTERED-SUSPECT §15.1   (NEVER) Instances never carry a `type` key.  (target line 13, containment 0.80) [strong keyword lost: NEVER]
  MISSING         §15.2   (HARD-ERROR) A cycle is a hard error, and the error prints the cycle path.  (—, containment 0.40)
```

**Two refinements the self-test forced** (both in the tool,
both recorded here because they change matching semantics):

1. **Table headers are not clauses.** The first census counted
   48 normative sentences; two were table-header artifacts
   ("Forbidden | Why" and the param table's "type | value must
   be | notes" — column names, not prose). The extractor now
   skips a row immediately followed by a `|---|` separator.
   Census: 46 genuine sentences.
2. **The short-sentence floor.** The dropped cycle clause
   initially scored 0.40 against a *sibling* clause ("A loader
   must hard-error…" shares the stock phrase "hard error" —
   2 of its 5 tokens) and read ALTERED-SUSPECT instead of
   MISSING. Hence the 0.50 floor for ≤ 6-token sentences
   (§1). A dropped clause that shares only a stock phrase with
   surviving text is dropped, and now says so.

---

## 3. Pre-fold baseline — target = `docs/scene-format.md` at `3a3a427`

Run: `git show 3a3a427:docs/scene-format.md` extracted to a
scratch file, then `python3 tools/format/check_foldin.py
<snapshot>`. (That file contains zero occurrences of "15." —
verified before the run.)

```
OVERALL: FAIL (0/75 FOUND, 5 ALTERED-SUSPECT, 70 MISSING)   exit 1
```

This is the honest pre-fold state the brief predicted: every
§15 heading MISSING, 41 of 46 normative sentences MISSING, all
but four named rules MISSING. The **5 ALTERED-SUSPECTs are
partial coincidences, each traceable, none of them a §15
clause present early**:

| Item | Matched pre-existing text | Why it is only a suspect |
|---|---|---|
| N "Duplicate name → hard error." (§15.5) | pre-fold line 195 (containment 0.50) | the base format has its own duplicate-name error; the §15.5 scope-assembly clause is absent |
| R-guard-film | pre-fold line 404 (1/2 groups) | §14 text mentions `film.rs` and node walking; the guard rule itself is absent |
| R-gradetime | pre-fold line 305 (1/2 groups) | §9 already pins GRADE time (the R3 source
...[truncated 3072 chars]