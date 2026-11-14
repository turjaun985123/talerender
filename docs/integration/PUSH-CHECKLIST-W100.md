# Pre-Push Checklist — seat W100 (Batch 10)

**Verdict: PUSH-READY — YES.** No blockers. One unpushed commit, clean on every check below.

Stamps (state re-verified twice; the lead did not push mid-check):

- First stamp: 2026-11-14T09:12:34Z — HEAD `8af40eb`, origin/main `ede0e92`, tree clean.
- Final stamp: 2026-11-14T09:13:34Z — HEAD `8af40eb`, origin/main `ede0e92`, range unchanged (see §3 for the one tree change).

## 1. Commit inventory — `origin/main..HEAD`

Exactly **one** unpushed commit:

| Commit | Class | Subject |
|---|---|---|
| `8af40eb9cfe450f18a1a2d9e448937c21e52d615` | **wave** | Wave batch 9 (W81–W90): saturation→sat rename, staging felt reconcile, tools-ci validator source-absent mode, MP4 finish + evidence, telemetry verdict record, loader evidence, golden staging, README proof patch proposal, gate 6/6 |

origin/main sits at `ede0e92` (lead: scene-format canonical grade key). Everything before `8af40eb` — including wave batch 8 (`743d248`) and all lead commits through `ede0e92` — is already on origin.

## 2. Wave-commit hygiene — `src/` ownership rule

Checked by `git show --name-only` (full file lists, not summaries):

- `8af40eb` (unpushed): **zero `src/` paths.** 41 files: docs (integration + ab/film + golden frames), `scenes/film/*.scene.json` + ASSUMPTIONS (data), `painters/felt.rs` + `painters/TEST-RUN.md` (wave-owned staging painters — not `src/`), `tools/` + `.github/workflows/tools-ci.yml` (wave tooling). Clean.
- `743d248` (batch 8, already pushed): **zero `src/` paths.** 21 files. Clean.

No ownership violations in either wave commit. (Lead commits in the pushed history own `src/` as expected — out of scope for this rule.)

## 3. Working tree

At the final stamp the tree holds exactly one untracked file:

- `docs/integration/GATE-W99.md` — 4.0K, text. A sibling Batch-10 deliverable awaiting the central commit. Expected class, no action.

Nothing else: no scratch dirs, no binaries, no frame dumps, no accidental files, nothing over 5 MB in the tree. (This checklist file itself becomes the second untracked Batch-10 doc when the orchestrator picks it up.) The assembled frame sets and scratch trees from W77/W84 live outside the repo (`~/workspace/.w77-scratch/`, `~/workspace/.w84-scratch/`) and are not part of the push.

## 4. Secrets scan — unpushed range

`git log -p origin/main..HEAD` scanned for credential shapes: private-key headers (`BEGIN … PRIVATE KEY`), GitHub token prefixes (`ghp_`, `gho_`, `github_pat_`), AWS access keys (`AKIA…`), Slack tokens, `sk-…` keys, `Bearer …` values, and `key/token/secret/password = value` assignments.

**Result: clean — zero matches.** A loose census for the words `password|secret|token` anywhere in the diff also returns **0**: the range doesn't even discuss credentials. Nothing to redact; no hit locations to report.

## 5. Large files

- Blobs added in `origin/main..HEAD`: **0 over 10 MB.** Largest blob is 3,045,401 bytes.
- The 14 staged golden frames (`docs/golden/frames/s01…s12.png`, in `8af40eb`) measure 2,772,199–3,045,401 bytes each (~2.8–2.9 MB, ~40 MB in total across the set) — each inside the expected band, none near the 10 MB line. They are W87's intentional, hash-verified staging (frames themselves are the evidence; RENDERER.txt records provenance). Repo growth of ~40 MB is a known, deliberate cost of this commit, not an accident.
- **The MP4 is not in the repo:** `git ls-files` shows no `.mp4` tracked anywhere. The finished proof MP4 lives only at `~/workspace/your_files/onoda-native-381ff47.mp4` (29,722,802 bytes).

## 6. CI expectations for this push

Two workflows fire; one does not:

- **ci.yml — fires** (any push). Green = `cargo build --release` + `cargo test --release` pass (the 71/71-class suite incl. the pasted painter tests in scene.rs) + the Milestone 1 smoke frame renders headless on lavapipe; binary + frame artifacts upload. Note its concurrency group cancels superseded runs on the same ref — if the lead pushes again in quick succession, a cancelled run is not a failure.
- **tools-ci.yml — fires** (this push touches `tools/**`, `scenes/**`, `docs/golden/**`, `docs/ab/**`, and the workflow file itself — all in its path filter). Green =
  - Film data validator: PASS with the explicit `--no-source` mode banner and the `COVERAGE: REDUCED` line in check 3 (506 source literals UNVERIFIED on hosted runners by design — W83; exact-f32 still enforced on all 1,389 numbers, >6-digit non-round-trippers still FAIL);
  - Caption determinism: **SKIPPED with `::warning::`** and exit 0 on hosted runners (source project not in the repo; a `::error::` + exit 1 happens only if `CAPTIONS_PROJECT` is set-but-wrong) — a warning here is the expected green state, not a failure;
  - Native golden pack verify: passes (STALE fails; film mids staged-but-unpinned read MISSING, which does not fail);
  - Bridge packager self-verification 21/21;
  - Scene JSON parse sweep clean.
- **render.yml — does NOT fire** (workflow_dispatch only). No render is part of this push's CI.

## Push-ready

**YES.** Push `8af40eb` (and the Batch-10 central commit when it lands) to origin/main. Expected outcome: ci.yml green, tools-ci.yml green with the caption step in its warned-skip state and the validator in declared reduced coverage. Anything redder than that is a real signal, not an expected artefact of this range.
