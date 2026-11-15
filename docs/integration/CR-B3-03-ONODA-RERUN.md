<!--
DELIVERY NOTE — Chrome Replacement, Batch 3, Seat 3 (CR-B3-03):
Onoda re-run through the shipped skill's native path.

Files created (this seat, NEW files only; nothing existing edited
in either tree, nothing committed):
  ~/workspace/chrome-replacement/batch3/B3-03/
    run-legs.sh / rerun-check.sh          (the exact run harnesses)
    logs/                                 (raw outputs, hash snapshots)
    work/onoda-rerun/                     (work copy; fixture untouched)
    ci-render-plan.md                     (CI render plan, legs 0–5)
    proof-onoda.yml                       (standalone workflow fragment)
    proof-onoda-second-job.yml            (B3-02 proof.yml adaptation)
    parity-statement.md                   (which comparisons hold / ride CI)
  docs/integration/CR-B3-03-ONODA-RERUN.md  (this note)

Laws honoured: no commits; the retrofitted fixture
(batch2/B2-08/onoda-copy/) and the original project
(~/faceless-shorts/the-soldier-who-didnt-know) were never mutated —
full-tree sha256 snapshots before/after are byte-identical for both;
all work ran on a copy. No fabricated results: the one failed leg is
reported as what it was (an externally killed run, §3).
-->

# CR-B3-03 — Onoda re-run through the shipped skill's native path

## 1. Verdict

**The canonical film re-runs end-to-end through the SHIPPED skill's
native path, CPU-side: every CPU leg passes with the shipped tool
generation.** Adapter check `CHECK OK` exit 0 with audit numbers
identical to the Batch-1 parity reference; cues byte-identical;
doctor `DOCTOR: OK` (structural and probed); the post-fix repo
validator `RESULT: OK (0 flagged)` — cleaner than any previous run
of it against this bridge. Render legs remain env-blocked on this
VM (measured mechanism: llvmpipe ICD found, PBR shader JIT segfault,
§4) and are staged as a precise CI plan plus a ready second job for
B3-02's proof harness (§5).

Method note: B2-08 proved the fixture with the retrofit-era tool
copies installed in the project. This seat re-proved it with the
**shipped skill tree** (`custom-skills/tall-tale-shorts/template/
tools/`) overlaid byte-for-byte onto a fresh work copy
(`logs/overlay-hashes.txt`). The overlay deltas vs the retrofit-era
copies are small and named: adapter wording is native-first (the
Batch-2 default flip), `native_modes/` and `cues_native.py` were
already identical, `castsheet_native.py` gained the B2-10 plate
origins, and `project_doctor.py` is new to the project.

## 2. CPU legs — real outputs (logs in the seat folder)

Binary for all legs: provisioned `~/talerender-bin/talerender`,
sha256 `b331f6704ebc8836…` (v1.0.1), pinned via `TALERENDER_BIN` /
`--talerender-bin` — the repo-checkout binary is the known stale
trap (`3e955092…`, predates `--film-telemetry`).

- **Adapter check** (`logs/adapter-check-rerun.log`), shipped
  adapter, `RENDERER=native … node tools/capture_native.mjs check`,
  re-run 18:57→19:09 UTC: banner + film header (60.36 s, 1,811
  frames, 1080×1920), 240 telemetry samples, 14-shot table, 73
  caption lines, `CAST: onoda, compA, compB, tani, suzuki`,
  `CANVAS OVERFLOW: none`, `CONTINUITY: clean (5 named characters
  tracked)`, `COMPOSITION: 31 findings (caption-zone 0, safe-area 8,
  thirds 3, framing 2, edge-touch 18)`, `CAPTION AUDIT: 73 chunks,
  12/12 lines authored, min fit 1.00, max rendered width 844 px`,
  **`CHECK OK`, exit 0**. Every number matches CR-B1-04's reference
  run on the same data.
- **Cues parity** (`logs/cues.log`, `logs/cues-sha.txt`): shipped
  `cues_native.py` output vs `data/cues.json` — **byte-identical**,
  sha256 `efbeb79e01ead1df…` both sides, 9 cues, duration 60.357 s,
  exit 0.
- **Doctor** (`logs/doctor-noprobe.log`, `logs/doctor-probe.log`):
  shipped `project_doctor.py` → `BRIDGE: current generation
  (manifest + F2 film data + manifest-shape plates)`; with probe:
  `PROBE: FILM TELEMETRY: CHECK OK`; both **`DOCTOR: OK`, exit 0**.
- **Validator** (`logs/validator-fixture.log`,
  `logs/validator-work.log`): repo `validate_native_project.py`
  post-fix (0774a31) → `RESULT: OK (0 flagged)`, exit 0, on the
  pristine fixture and the work copy alike. This closes B2-08's
  F4 on this fixture: the pre-fix validator's 3 phantom plates are
  gone, and the bridge passes with nothing flagged.

## 3. Incident — the first adapter-check run was killed externally

First attempt (`logs/adapter-check.log`): telemetry died after
sample 40 with no verdict line; adapter printed `CHECK FAILED`,
exit 3 (check_mode's "telemetry could not run" code — not a film
verdict). Lead notice during the seat confirmed the cause: at
~18:25 UTC seat B3-01, freeing CPU, killed processes by the name
`talerender` and caught this run's telemetry child (PID 11663).
No files were touched. The re-run (§2) passed in full, and the
doctor's probe — spawned 18:25 UTC, after the sweep — independently
completed with `FILM TELEMETRY: CHECK OK`. Both attempts are kept
in the logs; nothing is hidden.

## 4. Render legs — env-blocked here, measured

Probe (`logs/render-probe.log`): the binary finds this VM's
llvmpipe ICD (`/usr/share/vulkan/icd.d/lvp_icd.json`, Mesa 25.2.8),
prints `renderer: llvmpipe (LLVM 20.1.2, 256 bits) | backend:
Vulkan`, starts the film render (frame 0), then **segfaults,
exit 139**, writing no frame — the documented llvmpipe JIT failure
on the PBR shader path (`render.yml` header, `SELFTEST-NATIVE.md`).
So the block on this VM is a driver crash, not a missing ICD;
frames / sheet / castsheet / video all ride CI or the user's
RADV PC, as the contracts already state for the wave.

## 5. CI render plan (staged, not run)

Seat folder: `ci-render-plan.md` (legs 0–5 with exact commands and
expected outputs), `proof-onoda.yml` (standalone fragment in the
`render.yml`/`census.yml` pattern), and — because B3-02's
`proof.yml` landed while this seat ran —
`proof-onoda-second-job.yml`: a `proof-onoda` job to append to
B3-02's workflow (new file only; their harness untouched). It
reuses their Chrome scrub + exec-log shim and adapter bundle, runs
check → frames → sheet (defaults) → castsheet (look) → video
draft, and adds three Onoda PARITY steps: sheet at the CR-B1-04
configuration (step 2, cols 6 → exactly 1316×1936), castsheet
kind=cast (5 project characters → castsheet.jpg exactly 1100×972),
and a full census diff against `census-post-s12-8c0f850.txt` read
from the video leg's frame cache. Fixture staging ask for the
lead: `proof/fixture/onoda/` in B3-02's FLAC + PCM-md5 discipline,
plus the census file beside it. No artifacts, no commits — text
evidence in the job log, per the quota law.

## 6. Parity statement (summary; full text in the seat folder)

Holds locally (CPU): adapter check vs the B1 parity reference
(identical numbers), cues byte-parity, doctor and validator
verdicts, telemetry CHECK OK, film identity across all legs.
Rides CI: the sheet PSNR comparison (22.47 dB reference, CR-B1-04;
geometry already proven, s12 tiles may legitimately move vs the
pre-retirement B1 native sheet), the census (1,811/1,811 identical
is the stated expectation — argued from the additive-only mood
port caf18c0 and a scene grep showing Onoda uses neither new mood;
CR-B2-09 explicitly left its render-level census riding CI, and
the CI leg is the first render of this fixture by a binary
containing the 8c0f850 clip-data evaluator), castsheet kind=cast
(production proof; no local reference render exists), and the
video draft (CRF 18 master is a size/loudness reference only).

## 7. Scope note (B3-01 finding, via lead)

The `--film` driver/telemetry is hardcoded to the Onoda film.
This seat's re-run is exactly that film, so no verdict here is
weakened — but the proof generalises to the canonical film only;
new-film generality rides B3-01/B3-02's line.
