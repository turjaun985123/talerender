# R2-06 — flip package refresh: note (remediation wave, Batch 2)

Staged ONLY in this dir. Nothing applied, nothing committed, the
shipped skill tree untouched (all dry-runs ran in `/tmp` scratch
copies). Originals: `~/workspace/chrome-replacement/batch3/B3-10/flip/`.
Flip contract: `REMEDIATION-CONTRACTS.md` §5. Evidence base:
remediation `WAVE-STATE.md` (Batch 1 landings) + first-hand checks
below, run 2026-11-16 by this seat.

## 1. SKILL.md 7.0 stamp — NO DRIFT (verified, not assumed)

The task premise was that Batch 1 skill landings might have drifted
the shipped SKILL.md away from the B3-10 staged file. They did not:
Batch 1 landed in `library/`, `scripts/` and `template/tools/` —
SKILL.md itself was not touched (mtime 2026-11-15 17:58, pre-wave).

Verbatim dry-run output:

```
$ diff -u current-SKILL.md B3-10/flip/SKILL.md
@@ -1,12 +1,12 @@
-version: 6.0
+version: 7.0
...
-**Version 6.0** — `CHANGELOG.md` has what changed; `README.md` keeps the
+**Version 7.0** — `CHANGELOG.md` has what changed; `README.md` keeps the
(only hunks in the file — exactly two changed lines)

$ git apply --check -v SKILL-7.0.patch   (in a scratch copy)
Checking patch SKILL.md...
apply-check exit=0
apply exit=0
APPLIED == B3-10 FLIP FILE: IDENTICAL
```

Deliverable form CHANGED vs B3-10, deliberately: B3-10 shipped a
full-file copy; this package ships `SKILL-7.0.patch` as the
authoritative artifact plus `SKILL.md` as today's result of it.
Reason: Batch 2 seats R2-01 (§5 bridge-docs text) and R2-02 (§0
symptom-list text) land SKILL.md edits BEFORE the flip. A frozen
full-file copy applied at flip time would silently revert those
landings; a two-line patch either applies on top of them or fails
loudly. The checklist (step 3) now says exactly that.

## 2. Castsheet fallback-clearing patch — RE-PROVED, no drift

The shipped `template/tools/castsheet_native.py` is byte-identical
to the file the B3-10 patch was cut against (same mtime
2026-11-15 17:58; patch context matches). Verbatim:

```
$ patch -p1 --dry-run < castsheet_native.py.patch   (scratch copy)
checking file template/tools/castsheet_native.py
patch dry-run exit=0
patch apply exit=0
PATCHED == B3-10 FLIP FILE: IDENTICAL

$ python3 tools/castsheet_native.py --out out-patched   (patched)
castsheet_native: 77 items (36 plate-missing) -> /tmp/r206-cs/out-patched
items count: 77
fallback-flagged items: 0 []
grep nativeMoodFallback in file: 0

$ python3 tools/castsheet_native.py --out out   (unpatched baseline)
castsheet_native: 77 items (36 plate-missing)
unpatched fallback-flagged: 2 [('mood', 'puzzled'), ('mood', 'deadpan')]
```

So the parent wave's claim (77 items / zero fallback flags) still
holds at the current tree. One packaging fix: the B3-10 `.patch`
carries absolute-path headers
(`/home/hatch/workspace/custom-skills/…`); the refreshed `.patch`
in this package was regenerated with repo-relative labels
(`a/template/tools/castsheet_native.py`) and re-verified end to
end (`REGEN PATCHED == STAGED FILE: IDENTICAL`). Same hunks, same
result file (sha of staged file unchanged from B3-10's:
`8883ea50e0258902241230c53754ed8a5f91669f6efeef819df3ff15ba064ce4`).

Era warning CONFIRMED live, not copied on faith: the currently
provisioned `~/talerender-bin/talerender` hashes to
`b331f6704ebc8836057aa0e1b0f97421cb638c41ce9757abf0da230846903128`
— the v1.0.1 release binary, which PREDATES `caf18c0`. Landing the
castsheet edit before the flip-time binary swap would silence the
only warning for two silently-neutral moods. The refreshed
checklist keeps this as a hard gate with the sha recorded.

## 3. CHANGELOG v7.0 entry — what was filled, what was fixed

Filled with real remediation evidence (Batch 1): talerender
commits `093f40d` (filmseq + filmtelemetry + decoupling + roll +
guard + flat-quad fix), `5bc1b30` (bridge driver field + measured
caption fit + validator fix), `9469b04` (seat notes), `67fd9ef`
(R-W5 painters), `01a941a` (hygiene); CI 43804026408 step-level
SUCCESS + Tools CI 43804026272 SUCCESS at `67fd9ef`; skill-side
landings R1-06/R1-07/R1-08/R1-09 with their install-time counts;
pre-wave census reference (run 43797684898). Six clearly marked
`[TO-FILL-AT-FLIP: …]` slots remain: Batch 2 landings, census run
id/result, LC proof run id/result, flip-time binary (HEAD + CI run
+ sha), flip commit sha, sync result. The v6.0-and-older tail is
byte-verbatim from the CURRENT shipped CHANGELOG (verified
programmatically: tail identical = True).

Wrong in the B3-10 entry, now corrected:
- **Castshet bullet claimed `puzzled`/`deadpan` "have no native
  painter variant yet (flagged in the staging manifest — B2-09)".**
  Stale twice over: `caf18c0` ported both moods in the PARENT wave,
  and the flip's own castsheet patch removes the flag the sentence
  describes. The refreshed entry states the closed state + the era
  gate.
- **Bridge bullet hardcoded the Onoda shape** (`film/onoda.json` as
  THE data file). Post-decoupling the data file is the film's own;
  the refreshed entry describes the generic layout + the
  `"driver": "scene-sequence"` manifest field.
- **Verification block pointed at the wrong proof.** B3-10's slots
  asked for the parent wave's Batch 3 proof (Onoda-only). That
  proof PASSED for Onoda and the wave STILL failed acceptance —
  the remediation gate is different (census 1,811/1,811 + LC
  generic-telemetry proof, contracts §5) and the refreshed entry
  names both legs explicitly.
- **No remediation content at all.** The B3-10 entry predates the
  failure it would have shipped over: no filmseq, no generic
  telemetry, no decoupling, no roll, no painters, no measured fit,
  no exemptions. The refreshed entry leads with why v7.0 did not
  ship in the parent wave — a changelog that hid that would be a
  false record.

## 4. FLIP-CHECKLIST — differences vs B3-10

- Gates rewritten to contracts §5: census leg (a) + LC proof leg
  (b) replace B3-10's single "Batch 3 proof passed" gate; "Batch 2
  fully integrated + landings invalidate earlier gate runs" added.
- Step 1 rewritten: B3-10 swapped in its staged binary
  `talerender-63cc816`. That binary is stale by construction now
  (contracts §5, verbatim). The refreshed step provisions from the
  flip-time HEAD's CI artifact and re-runs era checks against it,
  including a NEW generic-path telemetry smoke (the path whose
  absence sank the parent wave).
- Step 2 marker count corrected by construction: B3-10 said 5
  `[TO FILL FROM CR-B3-*]` markers (true — 5 occurrences on 4
  lines); the refreshed package has 6 `[TO-FILL-AT-FLIP: …]` slots
  and the checklist counts 6.
- Step 3 now applies the SKILL patch (see §1 above) instead of
  copying a frozen file; castsheet verify gains the verbatim
  generator line + the unpatched baseline (2 flagged) for contrast.
- Step 6 keeps the `gh api user` == `turjaun985123` check BEFORE
  the sync, and notes R2-06 observed `turjaun985123` at refresh
  time without discharging the flip-time check (accounts flip on
  this machine; the sync script also self-aborts post-commit).
- Step 7 previews exclusion re-verified at refresh time: mirror
  `.gitignore` still contains `library/previews/` and
  `style-sheet.png`.
- Failure section extended: patch-no-longer-applies and failed
  census leg are now named stop conditions.

## 5. What else in the old package is now wrong (summary)

1. Staged binary `talerender-63cc816` + its seat-note era record —
   superseded; never provision from it.
2. B3-10's gate definition (Batch 3 proof) — superseded by
   contracts §5's two legs.
3. B3-10 CHANGELOG castsheet + bridge bullets — see §3.
4. B3-10's full-file SKILL.md delivery form — unsafe across Batch 2
   SKILL.md landings; replaced by the patch form (§1).
5. NOT wrong, re-verified live: the two-line stamp itself, the
   castsheet patch hunks + result, the 77-item count, the era-gate
   law, the README one-line stamp (current README line 3 still
   reads `**Current version: 6.0**`), the sync script path and its
   account guard, the previews exclusion.

## 6. Open at flip time (owned by the lead, not this seat)

Batch 2 landings (R2-01/R2-02/R2-03/R2-07/R2-08/R2-09/R2-10) had
not landed when this refresh was cut — their evidence slots in the
CHANGELOG are open by design. Census + LC proof run at the final
HEAD, after ALL landings. This package asserts nothing about their
outcomes.
