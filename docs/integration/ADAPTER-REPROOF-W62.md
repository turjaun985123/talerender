# ADAPTER-REPROOF-W62 — independent re-proof of the adapter's dual telemetry convention

**Seat:** TaleRender Wave 2, Batch 7, W62 (independent re-proof).
**Date:** 2026-11-14 (run at 18:11:55Z). **Verdict: RE-PROOF PASS** — all six
generation × outcome cells behave per the W02 spec, and the strip-mismatch
fixture is refused, in both modes tested.

**Why this seat exists.** WAVE2-RECORD's original "adapter re-proof" row was
corrected at `b6ee9fb`: the run behind it was SIGKILLed mid-telemetry under VM
contention (its rc 1 came from the adapter's no-verdict-line path, not from an
exit-1 verdict), and its fixture paired 66-chunk data with the 73-era strip
set. The lead's corrected real-binary re-proofs are separate rows (OK
direction recorded PASS; FAILED direction re-running). This seat trusts none
of that: it re-proves the **adapter's convention logic** from scratch, with
fixtures and stubs built by this seat, against the live work-copy adapter.

## 1. Unit under test

- Adapter: `~/workspace/talerender-skill-wiring/template/tools/capture_native.mjs`
  — copied **fresh from the work copy at run time** into each cell's temp
  project (the work copy was read, never edited).
- Adapter sha256 at run time:
  `a66153856ddddac30e2b8612a8da20f0c01daa42fbd8cbfb0bc0a9a61bf4d6b2`
- Node: `v24.22.1` (`/usr/bin/node`); `node --check` passed before any cell ran.
- Spec: `docs/integration/EXIT-CODES-W02.md` §4 (adapter behaviour table) and
  the work-copy stub suite `tools/test_adapter_exit_codes.sh` (read; its
  recorded 10/10 was **not** taken as evidence — this seat built its own
  fixtures instead of re-running that suite).

## 2. Method and resource discipline

The original false PASS was a SIGKILL under contention, so the method is the
point:

- Cells run **strictly sequentially** — one adapter process at a time, no
  parallelism anywhere in the runner.
- Every adapter invocation runs under `timeout 120` (each cell's real work is
  sub-second; the ceiling exists to catch a hang, not to fit the work).
- `/proc/loadavg` is recorded **before every cell**; wall time is recorded
  per cell. Conditions during the run were hostile and are reported, not
  hidden: load (1/5/15 min) started at **12.93 / 17.10 / 11.66 on 4 CPUs** and
  ended at **13.40 / 17.06 / 11.71**; memory available 2,766 MB of 7,934 MB at
  start. 
- Retry policy, fixed in advance: **one** disclosed retry per cell, only if
  the attempt was killed (rc 124/137 or the adapter reports a signal death),
  after a 20 s pause. A cell with no clean attempt is reported **UNPROVEN**,
  never passed by assertion.
- **Outcome: every cell completed cleanly on attempt 1** (walls 0.48–0.99 s).
  Zero retries were used; no cell is UNPROVEN.

Runner: `tools/ci/fixtures/adapter_reproof/run_reproof.sh` (new, beside the
fixtures; re-runnable — it re-copies the adapter from the work copy, so it
always tests the adapter as it currently stands).

## 3. Fixtures (all new, under `tools/ci/fixtures/adapter_reproof/`)

**Stub binaries** (`stubs/`, sh scripts answering the `--film-telemetry`
argv). Each stub was first run **directly** to prove the fixture side of its
cell independently of the adapter — verbatim:

```
stub gen-old-ok    -> rc=0 out=FILM TELEMETRY: CHECK OK
stub gen-old-fail  -> rc=0 out=FILM TELEMETRY: CHECK FAILED (0 overflow groups, 0 continuity violations, 7 caption failures)
stub gen-old-norun -> rc=0 out=film telemetry failed: stub: film data unreadable (simulated pre-aa10b13 could-not-run)
stub gen-new-ok    -> rc=0 out=FILM TELEMETRY: CHECK OK
stub gen-new-fail  -> rc=1 out=FILM TELEMETRY: CHECK FAILED (0 overflow groups, 0 continuity violations, 7 caption failures)
stub gen-new-norun -> rc=3 out=film telemetry failed: stub: film data unreadable (simulated post-aa10b13 could-not-run)
```

The old generation's could-not-run stub exits **0** with no verdict line, per
the documented old-binary contract (adapter header + CAPTURE-NATIVE.md: old
binaries exit 0 on pass, on fail, *and* on error). The FAILED line's counts
text is the canonical one used by the work-copy suite.

**Matched project** (`project/`): manifest `talerender-film` v1 (30 fps,
1080×1920, durationS 2.0, totalFrames 60, one segment tiling [0,60)); film
data whose captions name strips `cap_001.png` + `cap_002.png`, **both present**
in `assets/` alongside `plates.json` and the eleven fixed asset names the
adapter's `validateFilmData` requires.

**Strip-mismatch project** (`project-strip-mismatch/`): identical data —
captions name `cap_001.png` + `cap_002.png` — but `assets/` **omits
`cap_002.png`**. The binary stub paired with it prints CHECK OK and exits 0:
if the adapter passed this fixture, the refusal machinery would be broken in
exactly the direction the corrected record warns about.

## 4. Expectations (from EXIT-CODES-W02 §4)

| Cell | Binary behaviour | Adapter must |
|---|---|---|
| old × OK | line OK, exit 0 | accept — rc 0, terminal `CHECK OK` |
| old × FAILED | line FAILED, exit 0 | gate fails — rc 1, terminal `CHECK FAILED` |
| old × could-not-run | no line, stderr, exit 0 | fail closed — rc 1, no-verdict message naming exit 0 |
| new × OK | line OK, exit 0 | accept — rc 0, terminal `CHECK OK` |
| new × FAILED | line FAILED, exit 1 | gate fails — rc 1, terminal `CHECK FAILED` |
| new × could-not-run | no line, stderr, exit 3 | fail closed — rc 1, no-verdict message naming exit 3 |
| mismatch × check | (stub: OK, exit 0) | **refuse** at pre-validation, rc 1, naming the missing strip; telemetry never reached |
| mismatch × frames | (stub: OK, exit 0) | **refuse**, rc 1, zero artefacts (no `renders/`, no `review/`) |

## 5. Results — verbatim per cell

Adapter output is quoted verbatim (the two header lines naming temp paths are
included once, for cell 1; in later cells the decisive lines are quoted —
full transcripts were captured from the runner's stdout).

**Cell old-ok** — loadavg 12.93 17.10 11.66, attempt 1, **rc=0, wall 0.81 s**:

```
capture-native: backend=native (TaleRender) · binary .../stubs/gen-old-ok.sh (via TALERENDER_BIN) · manifest /tmp/w62-reproof.06bGIL/old-ok/proj/native/film.json
film: ADAPTER REPROOF FIXTURE (strips matched) — duration 2.00 s · 60 frames @ 30 fps · 1080x1920 · film data /tmp/w62-reproof.06bGIL/old-ok/proj/native/film
FILM TELEMETRY: CHECK OK
CHECK OK
```
**MATCH** — accepted, rc 0.

**Cell old-fail** — attempt 1, **rc=1, wall 0.96 s**:

```
FILM TELEMETRY: CHECK FAILED (0 overflow groups, 0 continuity violations, 7 caption failures)
CHECK FAILED
```
**MATCH** — the old binary's exit 0 did not leak the failure through; the
gate read the line and refused, rc 1.

**Cell old-norun** — attempt 1, **rc=1, wall 0.66 s**:

```
film telemetry failed: stub: film data unreadable (simulated pre-aa10b13 could-not-run)
capture-native: film telemetry produced no verdict line (binary exit 0) — treated as a FAILED gate. Exit 3 means the film data could not be read (the reason is on stderr, above); exit 2 usually means the binary predates --film-telemetry — use a current CI artifact.
```
**MATCH** — fail closed on the no-line/exit-0 combination (the same path that
produced the original record's misleading rc 1 — here it is the *expected*
behaviour, on a fixture where no verdict exists to relay).

**Cell new-ok** — attempt 1, **rc=0, wall 0.91 s**:

```
FILM TELEMETRY: CHECK OK
CHECK OK
```
**MATCH** — accepted, rc 0.

**Cell new-fail** — attempt 1, **rc=1, wall 0.99 s**:

```
FILM TELEMETRY: CHECK FAILED (0 overflow groups, 0 continuity violations, 7 caption failures)
CHECK FAILED
```
**MATCH** — gate refuses, rc 1, line and code in agreement.

**Cell new-norun** — attempt 1, **rc=1, wall 0.48 s**:

```
film telemetry failed: stub: film data unreadable (simulated post-aa10b13 could-not-run)
capture-native: film telemetry produced no verdict line (binary exit 3) — treated as a FAILED gate. Exit 3 means the film data could not be read (the reason is on stderr, above); exit 2 usually means the binary predates --film-telemetry — use a current CI artifact.
```
**MATCH** — fail closed, and the message names exit 3 as the could-not-run
code, per spec.

**Cell mismatch-check** (mismatch fixture × gen-new-ok stub, check mode) —
attempt 1, **rc=1, wall 0.83 s**, complete output:

```
capture-native: film data in /tmp/w62-reproof.06bGIL/mismatch-check/proj/native/film is incomplete — the binary would fail on:
  assets/cap_002.png
Regenerate the bridge data; the adapter does not improvise it.
```
**MATCH** — refused at pre-validation, naming the missing strip; no
`FILM TELEMETRY` line appears anywhere in the output, i.e. the gate was never
reached and the stub's CHECK OK was never even requested.

**Cell mismatch-frames** (mismatch fixture × gen-new-ok stub,
`frames review/frames 1.0`) — attempt 1, **rc=1, wall 0.78 s**, output
identical to mismatch-check above; the runner additionally verified
**zero artefacts**: no `renders/` and no `review/` in the cell's project.
**MATCH.**

Runner summary line, verbatim:
`== summary: 8 matched, 0 mismatched, 0 unproven (of 8 cells) ==` (runner exit 0).

## 6. Verdict

| Cell | Verdict |
|---|---|
| old × OK | **PASS** (accepted, rc 0) |
| old × FAILED | **PASS** (gate refuses, rc 1) |
| old × could-not-run | **PASS** (fail closed, rc 1) |
| new × OK | **PASS** (accepted, rc 0) |
| new × FAILED | **PASS** (gate refuses, rc 1) |
| new × could-not-run | **PASS** (fail closed, rc 1, exit 3 named) |
| strip mismatch × check | **PASS** (refused pre-gate, strip named) |
| strip mismatch × frames | **PASS** (refused, zero artefacts) |

**Overall: RE-PROOF PASS.** The adapter gates correctly on both telemetry
exit-code generations across all three outcomes, fails closed when there is
no verdict line under either generation's could-not-run behaviour, and
refuses strip-mismatched film data before the gate — under sustained load
(1-min loadavg 12.9–13.4 on 4 CPUs) with every cell clean on its first
attempt.

## 7. Scope and limits — what this proof does NOT establish

- **Stubs, not real binaries.** This seat proves the adapter's convention
  logic against faithful stub behaviour. That real binaries actually emit
  these lines/codes is the complementary proof: the lead's real-binary rows
  (OK direction PASS on the `ba88596` build; the corrected FAILED-direction
  re-runs in flight per WAVE2-RECORD). Neither proof substitutes for the
  other; together they cover the contract.
- **The old could-not-run stub rests on the documented contract** (old
  binaries exit 0 on error as well as pass/fail — adapter header,
  CAPTURE-NATIVE.md). No pre-aa10b13 binary was executed by this seat.
- **Mismatch detection boundary.** The adapter checks that every strip the
  data *names* exists in `assets/` (named strips ⊆ assets). An era-mix whose
  strip set still contains every named strip (a superset mix) passes this
  check by construction; catching that class belongs to the binary's
  telemetry audit over the matched data, not to the adapter's existence
  check. This seat's cell proves the refusal for the missing-strip case —
  the case where the adapter is the last line of defence.

## 8. Files written (all new; nothing committed; work copy untouched)

- `docs/integration/ADAPTER-REPROOF-W62.md` (this note)
- `tools/ci/fixtures/adapter_reproof/run_reproof.sh`
- `tools/ci/fixtures/adapter_reproof/stubs/gen-{old,new}-{ok,fail,norun}.sh` (6)
- `tools/ci/fixtures/adapter_reproof/project/` (matched fixture project)
- `tools/ci/fixtures/adapter_reproof/project-strip-mismatch/` (mismatch fixture project)
