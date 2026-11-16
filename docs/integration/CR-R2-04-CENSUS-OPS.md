# R2-04 — Census operations (Onoda leg of the §5 acceptance gate)

Seat R2-04, Chrome-Replacement REMEDIATION wave, Batch 2.
Gate: `docs/integration/REMEDIATION-CONTRACTS.md` §5(a) — post-remediation
census at the final talerender HEAD must match the pre-remediation census
**1,811/1,811**; any differing frame is listed by number and must be
defect-attributed before anyone proceeds.

## Deliverables (this directory)

| File | What it is |
|---|---|
| `run_post_census.sh` | Deliverable A — dispatch + wait + step verdicts + hash harvest |
| `diff_census.py` | Deliverable B — census differ (PASS/FAIL + every differing frame) |
| `PROOF-DIFF.txt` | Recorded diff proofs (self-diff PASS; one-hash mutation FAIL) |
| `PROOF-DRYRUN.txt` | Recorded `--dry-run` of the runner at HEAD `01a941a` (nothing dispatched) |
| `PROOF-HARVEST.txt` | Recorded harvest-pipeline proof against the real pre-run log |
| `NOTE.md` | This file |

Nothing was dispatched by this seat. No commits. All `gh` use was read-only.

## 1. The census mechanism, as found (run 43797684898)

- **Workflow:** `Census (dispatch)` — file `.github/workflows/census.yml`,
  workflow id `419041820`, state `active`. Added in commit `e3c4a2a`
  ("CI: full-film frame-hash census workflow (dispatch, log-text evidence
  only)") for the state_key sweat/tears proof; **byte-unchanged between
  `9743850` and current HEAD `01a941a`** (`git diff` = 0 lines), and it is
  the only commit that ever touched the file.
- **Trigger/inputs:** `on: workflow_dispatch:` with **no inputs at all**.
  The only dispatch parameter is the ref.
- **Pre-remediation run 43797684898** (verified via `gh run view --json`):
  event `workflow_dispatch`, branch `main`, **headSha
  `9743850143922d1e2b4cb5a4b43cfdb9c6aea679`**, conclusion **success**,
  single job `census` (id 133070135038), **artifacts: 0** (API-verified).
  Steps, all success: Set up job → checkout → rust-toolchain → rust-cache
  → Install mold + lavapipe → Build (~29 s) → Film telemetry (~3 m 02 s,
  printed `FILM TELEMETRY: CHECK OK`) → Full-film census (~39 m 17 s).
- **How the census works:** builds the release binary, runs
  `--film-telemetry`, then renders the Onoda film in 8 chunks
  (0–250 … 1750–1811) with `--film film --from/--to`, printing
  `sha256sum` of every frame to the job log and deleting each chunk's
  frames after hashing; ends with `CENSUS TOTAL FRAMES: 1811 (expected 1811)`.
- **How hashes were harvested (pre file):** from the job **log text** —
  there is no artifact. Raw log lines look like
  `<hash>  census-0-250/frame_00000.png`; the saved reference
  `~/workspace/talerender-evidence/census-pre-remediation-9743850.txt`
  strips the `census-<from>-<to>/` prefix, leaving
  `<hash>  frame_NNNNN.png`. I verified the saved file: 1,811 lines,
  strict sha256sum shape, frames `frame_00000.png`…`frame_01810.png`,
  sorted, unique. I also re-harvested run 43797684898's log with the
  runner's exact pipeline: the result is **byte-identical** to the saved
  reference (`PROOF-HARVEST.txt`).

## 2. The exact commands the lead runs at gate time

The final HEAD must be **pushed to `main` first** (see discrepancy 3).
Then, from this directory:

```bash
./run_post_census.sh <final-HEAD-full-40-char-sha>
python3 diff_census.py \
  ~/workspace/talerender-evidence/census-pre-remediation-9743850.txt \
  ~/workspace/talerender-evidence/census-post-<final-short-sha>.txt
```

`run_post_census.sh` refuses to dispatch unless the SHA is the current
`main` tip, polls for the new run with that headSha, waits for completion,
prints every step verdict, harvests to
`~/workspace/talerender-evidence/census-post-<short-sha>.txt`, and
validates 1,811 frames 00000–01810 + telemetry CHECK OK + CENSUS TOTAL
line before printing the run id. The raw dispatch it performs is exactly:

```bash
gh workflow run census.yml --repo turjaun985123/talerender --ref main
```

Gate verdict = `diff_census.py` printing `IDENTICAL: 1811/1811` and
`CENSUS DIFF: PASS` (exit 0). Any FAIL prints one `DIFF frame NNNNN`
line per differing frame with both hashes — that list is the defect-
attribution input; per §5(a) the gate stops there until each is explained.
Fallback if a run was already dispatched by hand:
`./run_post_census.sh --from-run <run-id> <sha>` harvests it without
re-dispatching (headSha is asserted).

## 3. Expected duration

Run 43797684898: **43 m 40 s end-to-end** (job 43 m 36 s; census step
39 m 17 s; telemetry ~3 m; build ~30 s). But earlier census runs on the
same workflow took **1 h 02 m – 1 h 23 m** (43710059527, 43703307203,
43726047689, 43731826714), so budget **up to ~1.5 h**; the workflow's own
timeout is 180 min. The runner polls every 60 s and never times out early.

## 4. Artifact-size discipline

`census.yml` uploads **nothing** — by design, after the 2026-11-15
artifact-storage quota incident: evidence is log text only, frames are
deleted per chunk (disk stays at one ~250-frame chunk). The runner
preserves this: it downloads only the job log (`gh run view --log`),
writes only the 1,811-line hash file (~145 KB) into
`~/workspace/talerender-evidence/`, and creates no artifacts. Do not add
an upload step to "help" harvesting — that is exactly the quota failure
mode the workflow was built to avoid.

## 5. Diff-half proof (recorded in PROOF-DIFF.txt)

- Pre vs itself: `IDENTICAL: 1811/1811`, `CENSUS DIFF: PASS`, exit 0.
- Pre vs a copy with frame 00999's hash mutated (first hex digit flipped):
  `DIFF frame 00999 (frame_00999.png): pre=b6f6ffc5… post=06f6ffc5…`,
  `IDENTICAL: 1810/1811`, `CENSUS DIFF: FAIL`, exit 1.

## 6. Discrepancies between the record/charter and the workflow as found

1. **Quota-red upload steps cannot occur on a census run.** The charter
   asked the runner to "distinguish quota-red upload steps from real
   failures" — but `census.yml` contains **no upload-artifact step at
   all** (and run 43797684898 has 0 artifacts). The quota-red pattern
   belongs to the *other* workflows (ci.yml etc., cf. STATE-KEY-FIX.md).
   Consequence: on a census run, **any failed step is a real failure**.
   The runner still classifies upload-named steps separately and reports
   both counts explicitly, so the distinction is visible if the workflow
   ever gains such a step.
2. **Saved hashes ≠ raw log lines.** The contract (§0) says hashes live
   "in the job log; saved locally" without noting the shape change: log
   lines carry the `census-<from>-<to>/` directory prefix, the saved
   reference does not. A naive harvest diffed naively would false-fail
   all 1,811 frames. The runner normalises (proven byte-identical, §1);
   `diff_census.py` additionally keys by frame number, so prefix or
   ordering differences can never masquerade as frame differences.
3. **"Dispatch at that ref" needs a branch, not a bare SHA.** The
   workflow_dispatch API/`gh workflow run --ref` takes a branch or tag;
   there is no way to dispatch at an arbitrary detached SHA. The runner
   therefore requires the target SHA to be the `main` tip and asserts
   the resulting run's headSha — the gate's "final HEAD" must be pushed
   to main before dispatch (which matches how 43797684898 itself ran:
   branch `main`, headSha `9743850…`).
4. **Duration in the record understates variance.** The pre-run's
   43 m 40 s is the fastest census on record; the four earlier census
   runs took 62–83 min. Plan the gate window for ~1.5 h, not ~45 min.
5. No discrepancy in the census contract itself: workflow name, 1,811
   frames, chunking, telemetry-first order, and the no-artifacts law all
   match REMEDIATION-CONTRACTS.md §0/§5(a) and STATE-KEY-FIX.md exactly,
   and the workflow is unchanged since before the pre-remediation run.
