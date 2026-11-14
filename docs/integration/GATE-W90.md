# GATE-W90 — Batch-9 gate + pre-close audit

Seat W90, TaleRender 100-agent wave. Runs executed 2026-11-14 (UTC, timestamps per section).

## Tree state at gate start

- HEAD: `743d248` (Wave batch 8 commit) — measured 2026-11-14T08:36:09Z.
- `git status --porcelain` at gate start: **clean** (no uncommitted changes).
  No Batch-9 sibling seat (W81 rename, W82 staging reconcile, W83 tools fix,
  W87 golden staging) had landed in the working tree at gate start; any
  landing during the run is noted where it affects a read.
- Tree state at gate end: HEAD advanced to `0f6c033` (lead, pushed —
  `origin/main` equal); working tree carries the uncommitted Batch-9
  landings + this file. Full account in "Tree movement during the
  gate" below.

## JOB 1 — Consolidated gate

### 1. Film validator — `python3 tools/film/validate_film.py`
Run 2026-11-14T08:36:24Z. **PASS.**

Verbatim: `OVERALL: PASS (7/7 checks passed)`, exit 0. Check 3: 883 exact
f32 + 506 source literals + 0 computed failures (1,389 numbers); captions
66/66 chunks, 0 duplicates, 0 uncovered; durations sum 60.357 s → 1,811
frames, tiling exact. W81's `saturation`→`sat` rename had not landed
at run time (working tree clean at `743d248`); this run therefore
measures the pre-rename committed data. **Addendum (08:46:28Z):** the
rename landed in the working tree during the gate (see "Tree movement"
below); re-running the validator on the renamed tree (with W83's
working-tree validator) again gives `OVERALL: PASS (7/7 checks
passed)`, identical numbers (883 + 506 + 0) — the rename keeps 7/7,
as required.

### 2. Caption determinism — regenerate + `diff -r` vs committed assets
**PASS.** Regenerated with `tools/captions/gen_caption_strips.py
--project ~/faceless-shorts/the-soldier-who-didnt-know --out
/tmp/gate90-captions`; generator headline: 66 chunks, min fit 0.92,
max rendered width 960 px, 7 over the 850 px floor, 0 under the 0.68
floor. `diff -r` against the committed `assets/captions/` (repo root;
66 strips + manifest.json = 67 files): **clean — byte-identical**.

### 3. Golden verify — `python3 tools/golden/make_golden.py`
**PASS** (verify mode). Verbatim: `4 OK / 0 STALE / 14 MISSING of 18 golden items`,
exit 0. Gloss, as in GATE-W62/W80: OK = each pin matches its own
(pre-shade-fix) bytes; STALE is a property of re-renders, not of this
verify; the 14 film segment mid-frames are MISSING — W87's staging has
not landed in this tree (no files under `docs/golden/frames/`, tree
clean at gate start). **Addendum:** W87's staging landed during the
gate — `docs/golden/frames/` now holds the 14 segment mid-frames +
`RENDERER.txt` (untracked). Re-running the verify gives the same
headline (`4 OK / 0 STALE / 14 MISSING of 18`) with a changed meaning
for the film rows: a current hash is now shown for each staged frame
(e.g. film-s11 `eb7b23f4a4be2834`) but the pin column is still empty —
staged, not pinned. The pin remains the outstanding human act.

### 4. Bridge packager — fresh /tmp target
Run against `/tmp/gate90-bridge` (fresh). **PASS.**

Verbatim: `package-bridge: VERIFICATION PASSED — 21/21 checks`, exit 0;
94 files packaged, MANIFEST-SHA verified over all 94.

### 5. JSON sweep — `scenes/**/*.json` parse
**PASS.** 32 files under `scenes/`, 0 invalid.

### 6. Painter harnesses — integrated (src/scene.rs) + staging (painters/)
**PASS, both forms** (standalone rustc 1.99.1, TEST-RUN.md recipe with
the documented wgpu-stub constant; scratch in /tmp, repo untouched).

- **Staging form** (painters/*.rs + scene.rs/math.rs from
  `c74cf0c^`): `test result: ok. 52 passed; 0 failed` (164 s).
  Tree-state precision: the painters inputs were copied from the
  working tree at ~08:39Z, minutes after W82's reconcile landed
  there (uncommitted) — `painters/felt.rs`
  `shade_is_the_three_linear_multiply` now asserts agreement
  (`assert_eq!` vs `crate::scene::shade_rgb`, comment credits the
  reconciliation to W82). So the staging read is 52/52, not the
  51/52 that GATE-W80 measured pre-reconcile; W70's refutation is
  closed in both copies now. (At committed HEAD the staging copy
  still carries the superseded assertion — the reconcile awaits
  the Batch-9 central commit like the other landings below.)
- **Integrated form** (`src/scene.rs` + `src/math.rs` as of
  committed HEAD `743d248`, copied ~08:42Z):
  `test result: ok. 45 passed; 0 failed` (83 s) — 34 pasted painter
  tests + 2 json + 9 math, felt agreement assertion included. Note:
  the lead's `0f6c033` (08:44:46Z, sky dome) changed `src/scene.rs`
  after this copy; the 45/45 read is of the `743d248` scene.rs, not
  of current HEAD.

### 7. Tools-CI validator fix (W83 source-absent mode)
**PENDING — W83 has not landed in this tree.** Evidence: working tree
clean at `743d248`; `validate_film.py` at HEAD exposes only `--root`
(no source-absent flag/mode), and `tools-ci.yml` invokes it bare
(`--root .`). HEAD behaviour reproduced first-hand in a /tmp copy with
the manifest's project path pointed at a nonexistent dir (the hosted-
runner condition): `[3 f32] FAIL — … 0 source literals (pass), 506
computed values not round-tripping (fail); literals grepped from 0
source files`, `OVERALL: FAIL (6/7 checks passed)`, exit 1 — the exact
signature of the Tools CI failure at `381ff47` (run 43573712817). The
fix (exact-f32 still enforced; literals reported UNVERIFIED, not
failed) remains to land; until it does, Tools CI on hosted runners
stays red on the validator step for an environmental reason, not a
data reason.

**Addendum — the fix landed during the gate and is verified.**
W83's changes are in the working tree (uncommitted):
`tools/film/validate_film.py` gains a `--no-source` flag plus an
auto mode when the source tree is absent; `tools-ci.yml` and
`tools/CI-WORKFLOW.md` are updated alongside. Verified first-hand
with the working-tree validator in the sandbox (project path →
nonexistent):

- Auto mode (no flag): `[3 f32] PASS — 1389 numbers checked: 883
  exact f32 (pass), 506 source literals UNVERIFIED (source tree
  absent), 0 computed values not round-tripping (fail)`, exit 0 —
  each short decimal is individually labelled UNVERIFIED, never
  silently passed.
- `--no-source`: same verdict plus an explicit `COVERAGE: REDUCED`
  line stating the SOURCE LITERAL class was not checked, that the
  exact-f32 round-trip was enforced on all 1,389 numbers, and that
  >6-significant-digit non-round-trippers still FAIL;
  `OVERALL: PASS (7/7 checks passed)`, exit 0.

So item 7's status at gate end: **landed (working tree) and behaving
exactly as specified** — PASS-with-UNVERIFIED in source-absent
environments, full 7/7 where the project exists (re-verified at
08:46:28Z on this VM). It takes effect in CI once the Batch-9
central commit lands and pushes.

## Tree movement during the gate

The tree moved substantially while the gate ran (~08:36–08:50Z) —
all of it sibling/lead activity, recorded here so every read above
can be attributed to the state it measured:

- **08:36:09Z** — gate start: clean tree at `743d248` (Batch-8
  commit). Items 1, 3 (first read), 4, 5 measure this state.
- **~08:38Z** — Batch-9 seats land in the working tree
  (uncommitted): W81 (rename in 12 scene files + ASSUMPTIONS note),
  W82 (`painters/felt.rs` reconcile + `TEST-RUN.md`), W83
  (`validate_film.py` `--no-source`, `tools-ci.yml`,
  `tools/CI-WORKFLOW.md`), W87 (`docs/golden/frames/` staged +
  `GOLDEN-STAGING-W87.md`), W86/W88 evidence docs
  (`LOADER-EVIDENCE-W86.md`, `README-PROOF-PATCH-W88.md`), W89
  (`FINAL-REPORT.md` + `FOG-STATUS-W74.md` edits; FINAL-REPORT
  mtime 08:38:54Z, unchanged since — the JOB 2 audit below reads
  exactly this version).
- **08:44:46Z** — lead commit `0f6c033` ("Sky dome corrected to the
  source shader + atmosphere re-dressing wired"; touches
  `src/ab1.rs`, `src/film.rs`, `src/scene.rs`, `src/scenefile.rs`)
  lands and is pushed: HEAD and `origin/main` both at `0f6c033`,
  22 commits since base `34d7882`. Nothing committed is unpushed;
  only the working-tree Batch-9 landings await the central commit.

## JOB 2 — Pre-close audit of docs/integration/FINAL-REPORT.md

Snapshot audited: the working-tree version, mtime
2026-11-14T08:38:54Z (W75 pass + W89 addenda; uncommitted, like the
other Batch-9 landings). 12 claim groups checked against first-hand
sources (git log, the audit docs, `docs/ab/film/ab-report.json`,
source greps, re-runs above). **6 MATCH · 6 STALE** — every STALE
is a timing/concurrency artefact (a sibling or the lead moved the
truth after the text was written), itemised so the close-out seat
can reconcile mechanically. One is also an internal inconsistency:
§4.5's W89 correction (21 commits) and §5.1 (18 commits) disagree
inside the same snapshot.

| # | Claim (FINAL-REPORT) | Verification | Verdict |
|---|---|---|---|
| 1 | §5.1 census: 11 lead commits (list ends `d8fbe83`), 7 wave, 18 total since `34d7882` | `git log 34d7882..HEAD` at audit close: **14 lead** (adds `69952ab`, `5b0889d`, `0f6c033`), **8 wave** (adds `743d248`), **22 total** | **STALE** — report says 11/7/18; tree says 14/8/22 |
| 2 | §4.5 (W89): 21 commits since base on local main, 20 on origin; origin at `5b0889d`, local ahead by `743d248` | Correct when written; at audit close local = origin = `0f6c033`, **22/22** | **STALE** (by the lead's `0f6c033`, landed + pushed 08:44:46Z) |
| 3 | §2.7 audit totals: 395 sampled / 387 match / 8 mismatch (W40 90/87/3, W42 165/162/3, W61 140/138/2) | Arithmetic exact; AUDIT-W61.md headline "138 MATCH, 2 MISMATCH" confirmed in the doc; W40/W42 figures match their docs | **MATCH** |
| 4 | Film A/B (docs/ab/film package the report's §4.2 points to): 60 pairs, mean 21.836 dB, s01 anchor 24.094 dB | Recomputed from `ab-report.json`: 60 pairs; mean **21.837** from the stored 3-dp pair values (report's 21.836 presumably from unrounded values — a 0.001 rounding artefact, not a data difference); s01 t=1.50 pair = **24.094** exact; overall min 18.378 exact; worst-3 all s11 | **MATCH** (rounding note) |
| 5 | Telemetry: whole-film verdict PENDING; no `FILM TELEMETRY: CHECK OK` in the record (§3, §4.2, §5.2) | Tree-wide grep: every occurrence of the string is procedural (how to read a verdict), none is a recorded verdict; RENDER-READINESS.md:389 concurs | **MATCH** |
| 6 | Fog: loader wiring CLOSED by `69952ab`, verified `5b0889d`, chart ≤1 code at every panel (§2.3 W89 note, §4.1) | Both commits in `git log` first-hand with those messages (fog block parse + `shader_fog.wgsl`; spec §14.5 addendum recording the ≤1-code verification). The ≤1-code figure itself is the lead's CI evidence as cited — the report cites it the same way | **MATCH** |
| 7 | Saturation (W89 update, §2.7/§4.1): rename landed in working tree; all 14 files carry `sat` (0.52–1.08), zero `saturation` keys; validator 7/7 on the renamed tree; pending Batch-9 central commit | Working-tree grep: 0 files with `"saturation"`; `sat` present in all 14 (values 0.52–1.08); one-line diff per file (`"saturation": 0.8` → `"sat": 0.8` etc.); validator re-run at 08:46:28Z on the renamed tree: 7/7 PASS. Still uncommitted at audit close, exactly as stated | **MATCH** |
| 8 | §2.7 red-team residue: "the wave's staging copy `painters/felt.rs` still carries the superseded `assert_ne!`… a fresh run of the wave's scratch harness over `painters/` still reads 51/52" (repeated in §5.2) | W82's reconcile is in the working-tree `painters/felt.rs` (agreement assertion, comment credits W82); this gate's staging-harness run over the current `painters/` reads **52/52** (JOB 1 item 6) | **STALE** — residue closed in the working tree during the same window the W89 pass was written; closed at committed HEAD only once the Batch-9 commit lands |
| 9 | §4.2 proof run: all five items outstanding (render, telemetry, A/B table, golden staging, MP4); §3 tail: the re-render "is in progress" | The `381ff47`-provenance render completed in CI and the A/B table is filled in `docs/ab/film/` (RESULTS.md + ab-report.json, committed in `743d248`); W87 has since staged the golden mid-frames (working tree). Still open: telemetry verdict, golden pins (human), MP4 evidence. Caveat in the other direction: the lead's `0f6c033` sky-dome correction may supersede the `381ff47` frames' provenance — a further re-render is the lead's call | **STALE** (items 1 + 3 done in-tree; §3's "in progress" overtaken) |
| 10 | §2.7 sea-metalness disposition: W73 fix "is in the working tree at this writing, pending the Batch-8 central commit" | Substance verified (s03 — unmodified since — carries `metalness: 0.08`; `743d248`'s record includes the restore for s02/s03); the Batch-8 central commit has landed, so the "pending" framing is one commit behind events | **STALE** (framing only; substance MATCH) |
| 11 | §2.1 caption width: max Δ 0.15 px, mean 0.054 px, source `tools/captions/README.md` (W89 located it after W80 could not) | README.md:86: "rawW absolute difference \| max 0.15 px, mean 0.054 px" — the citation is real | **MATCH** (W80's one unverifiable figure is now sourced) |
| 12 | §4.4: ADOPTION-REVIEW restoration "is assigned to seat W72" (present tense) | The restoration completed in the Batch-7/8 window (work copy; verified by W72/W75 records) — the tense is behind events | **STALE** (minor, tense only) |

Not counted as claims: §5.1's Batch 8–10 rows and §5.3 are tagged
[TO COMPLETE] by design — Batch 8's row is fillable now (10 seats,
commit `743d248`); the rest belongs to the close-out pass.

## Summary

**JOB 1 — gate: 6/6 executable checks PASS**, each on a stated tree
state: validator 7/7 (pre-rename at 08:36Z and post-rename at
08:46Z), caption determinism byte-identical (67 files), golden
verify 4 OK / 0 STALE / 14 MISSING (film frames now staged,
unpinned), bridge 21/21 over 94 files, JSON sweep 32/32, painter
harnesses 52/52 (staging, post-W82 working tree) and 45/45
(integrated, `743d248` scene.rs). Item 7 (status check): the W83
tools-CI validator fix was **absent at HEAD at gate start**
(failure reproduced: source-absent → check 3 FAIL, exit 1) and
**landed + verified during the gate** (PASS-with-UNVERIFIED in both
auto and `--no-source` modes; exact-f32 enforcement intact).

**JOB 2 — pre-close audit: 12 claim groups, 6 MATCH / 6 STALE.**
No claim was found wrong when written; all six STALE entries are
concurrency artefacts of a tree that moved during the audit
itself (Batch-9 landings + the lead's `0f6c033`). The two the
close-out seat must not miss: the census (§5.1 says 18 total /
§4.5 says 21 / the tree says **22**, split **14 lead / 8 wave**,
local = origin = `0f6c033`) and §4.2's proof-run framing (render
+ A/B are done in-tree; telemetry, pins, and MP4 are the real
remainder — with the caveat that `0f6c033` may re-open the render's
provenance).
