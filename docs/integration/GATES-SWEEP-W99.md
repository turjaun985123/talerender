# GATES SWEEP — Wave 2 closing evidence table (W99, Batch 10)

**Frozen HEAD observed:** `9380b01285f0bca4be6d103b1af0fc451724c627`
("Wave 2 Batch 9: knobs-gate acceptance (F4 ACCEPTED, two runs PASS), …;
gate W90 PASS-WITH-NOTES", 2026-11-15 01:01:32 +0600).
**Observed:** 2026-11-14 19:01–19:05 UTC.
**Method:** first-hand `gh run list/view --log` output for every workflow row;
`git merge-base --is-ancestor` verified for every cited head (all are ancestors
of the frozen HEAD — each run below is evidence about *its own* head, labelled
as such, never silently about the frozen HEAD). Golden verify was run locally
by this seat against the frozen-HEAD tree (read-only pack check). Telemetry's
row is from its documented local evidence, as commissioned. This seat
dispatched nothing and edited nothing else.

## The closing table

| Gate / workflow | Latest completed run | Head | Conclusion | Verdict line / evidence |
|---|---|---|---|---|
| **CI** (`ci.yml`) | `43629817118` | **`9380b01` = frozen HEAD** | **failure** (Test step) | 179 passed / **1 failed**: `telemetry::tests::tables_beat_frames_tie_to_blink_at` — see §1 |
| **Tools CI** (`tools-ci.yml`) | `43629625142` | `3223509` | success | **No run at the frozen HEAD** — path-filtered trigger, the Batch-9 commit touched no watched path (§2) |
| **Knobs gate** (`knobs.yml`, dispatch) | `43628339967` (+ twin `43628289244`) | `c440d90` | success ×2 | `KNOBS GATE: PASS` (log read first-hand, §3) |
| **Determinism** (`determinism.yml`, dispatch) | `43624016907` | `166873c` | success | `DETERMINISM: PASS` (log read first-hand, §4) |
| **Visual Regression** (`visual-regression.yml`, dispatch) | `43628892930` | `3c07564` | **failure — as designed** | 3 MATCH · **1 DRIFT (s28 only)** · 14 NOT-RENDERED, verified from the run's own table (§5) |
| **Bench** (`bench.yml`, dispatch) | `43623743480` | `aa92cee` | success | No verdict line exists; documented reading in BENCH-FIRST-W64 (§6) |
| **Golden verify** (`tools/golden/make_golden.py`) | local run by this seat, at observation | frozen-HEAD tree | — | **18 OK / 0 STALE / 0 MISSING of 18 golden items** (§7) |
| **Film telemetry** (binary CPU mode) | W68 local run, binary of CI run `43624139322` | binary head `b6ee9fb` | — | **`FILM TELEMETRY: CHECK OK`, exit 0** (PC-DRYRUN-W68 §3.1) (§8) |

**Headline for the close-out:** every gate is green at *some* ancestor head, and
the golden pack is green at the frozen HEAD itself — but **CI is RED at the
frozen HEAD** on exactly one unit test in the G7 `--tables` code (§1), and five
gates have no run *at* the frozen HEAD (§9 lists the dispatch/action that
closes each). W100's verdict must weigh both halves of that sentence.

---

## §1 CI — RED at the frozen HEAD (one test)

Run `43629817118` @ `9380b01` was `in_progress` when this seat began and
completed **failure** during the sweep; this section reports its final state.

Anatomy (from the run's job/step view and log, first-hand):

- Step 6 **Build: success**. Step 7 **Test: failure**. Steps 8–10 (M1 smoke
  render, frame artifact, binary artifact) **skipped** — so **no binary
  artifact exists for the frozen HEAD**.
- The failure is exactly one test of 180:

  ```
  test telemetry::tests::tables_beat_frames_tie_to_blink_at ... FAILED
  thread 'telemetry::tests::tables_beat_frames_tie_to_blink_at' panicked at src/telemetry.rs:2062:13:
  assertion `left == right` failed
    left: 0
   right: 3
  test result: FAILED. 179 passed; 1 failed; 0 ignored
  ```

- Provenance: `git log -S 'tables_beat_frames_tie_to_blink_at'` (this seat)
  shows the test was introduced by `3223509` — the lead's G7 `--tables`
  landing ("G7 tables in the binary (TABLES-IMPL-W85)"). That commit's own CI
  run (`43629625139`) was **cancelled** (see below), so the frozen HEAD's run
  is the *first completed CI* to exercise the `--tables` code — and it caught
  this. The failing test is one of W85 §6's specified unit seams (the
  beat-frame counter tied to `blink_at`); the diff-check of that landing is
  Batch-10 seat W93's brief, and the fix is the lead's. This seat reports the
  evidence only.

**Cancelled ≠ failed.** Two CI runs in the window are `cancelled`, both by the
workflow's concurrency cancel-in-progress being superseded by the next push —
neither says anything about its head's code:

- `43629625139` @ `3223509` — cancelled when `9380b01` was pushed.
- `43628889658` @ `3c07564` — cancelled when `2fcffc0` was pushed.

Previous completed CI runs: `43629313479` @ `108f33f` **success**,
`43629008511` @ `2fcffc0` **success**, `43628681545` @ `d967931` **success**.

## §2 Tools CI — no run at the frozen HEAD (trigger design, not failure)

`tools-ci.yml` triggers only on pushes touching `tools/**`, `scenes/**`,
`src/**`, `assets/captions/**`, `film/**`, `docs/golden/**`, `docs/ab/**`, or
itself (its `on:` block, read first-hand). The frozen HEAD commit touched only
`docs/integration/**` (+ `docs/integration/f3bc/`), so **no Tools CI run exists
at `9380b01`** — nothing failed; nothing was invoked. The workflow has no
`workflow_dispatch` trigger as written, so this closes on the next push that
touches a watched path.

Latest completed: `43629625142` @ `3223509` **success** — the tail of an
unbroken success streak across the observed window (`c440d90`, `d967931`,
`3c07564`, `2fcffc0`, `3223509`, five consecutive completed runs, all success).

## §3 Knobs gate — PASS at `c440d90` (two runs)

Latest completed: `43628339967` @ `c440d90`, **success**; twin run
`43628289244` at the same head also success (W81's acceptance pair). This seat
read `43628339967`'s log first-hand:

- Leg A: `BYTE PROBES: PASS (40/40 frames identical across 3 range(s))`
- Leg B: `BYTE PROBES: FAIL (10 frame(s) differ in 1 of 1 range(s))` — the
  *required* draft-difference leg, not a gate failure
- Final line: `KNOBS GATE: PASS`; the verdict step's `grep -qx "KNOBS GATE:
  PASS"` confirmed.

Dispatch-only: **no knobs run at the frozen HEAD.** Closing dispatch:
`gh workflow run knobs.yml` (it builds its own binary; independent of §1).

## §4 Determinism — PASS at `166873c`

Latest completed: `43624016907` @ `166873c`, **success**. Log line read
first-hand: `DETERMINISM: PASS`, followed by `DETERMINISM: PASS confirmed
(pack exit 0 + PASS line present)`. Prior runs also success:
`43620084786` @ `fbf37c0`; E6 `43617370962` @ `3a36f1f` — three consecutive.

Dispatch-only: **no determinism run at the frozen HEAD.** Closing dispatch:
`gh workflow run determinism.yml`.

## §5 Visual Regression — failure as designed, verified from the log

Latest completed: `43628892930` @ `3c07564`, conclusion **failure**. That head
*is* the W72 register-patch commit ("W72 VR register update (GATE2-W80
§12.1.2): post-pin register + fixtures, one commit"), so this is the first
post-register-patch VR run. The run's own per-pin table (log, first-hand):

- **3 MATCH** (byte-identical): `dof-chart` (`957d5573…`), `ab1-composer`
  (`60c32c1a…`), `film-s08b` (`fb4e2ba4…`)
- **1 DRIFT**: `s28-onoda-t150` — 21.65 dB, pin `ff68fad3…` vs rendered
  `60c32c1a…`
- **14 NOT-RENDERED**: `stroke-toon` + `film-s01…s07, s08, s09…s12` (pins
  present; this dispatch supplied no frames for them)

Runner exit code 1 → workflow failure. Under the new register (W72 pack;
re-verified by gate W90) this exact pattern classifies **AS EXPECTED**: the
sole DRIFT is s28's known debt (`{DRIFT, WITHIN-BAND}`), and no pin produced a
new signal. The expected state — failure-as-designed with sole DRIFT s28 — is
therefore **confirmed from the log**, not assumed.

One reader warning: the workflow's static `::error::` annotation text still
carries pre-Act-2 boilerplate naming `ab1-composer` as expected-stale. The
run's table governs: ab1-composer is MATCH (it was re-pinned in Act 2). The
annotation wording is stale; the register verdict is not.

**No VR run at the frozen HEAD.** Closing dispatch:
`gh workflow run visual-regression.yml` — expected to reproduce the same
as-designed failure until the s28 disposition (a user decision) lands.

## §6 Bench — success at `aa92cee` (only run of the wave)

`43623743480` @ `aa92cee`, **success** — the bench workflow's only dispatch to
date. The workflow emits no single verdict line (its numbers live in the run
artifact); the documented reading of this run is BENCH-FIRST-W64.md: CI
lavapipe 1.85 s/frame over frames 0–100; the cache=24 leg saves ~147 MiB peak
RSS at no time cost in-window; anomalies A1 (the bench window is S01-only) and
A2 (accounted-vs-resident cache bytes ~9× divergent) stand as recorded there.

**No bench run at the frozen HEAD.** Closing dispatch:
`gh workflow run bench.yml`.

## §7 Golden verify — green at the frozen HEAD (run by this seat)

`python3 tools/golden/make_golden.py` (pack verify, read-only) run by this
seat against the frozen-HEAD tree at observation time:

```
18 OK / 0 STALE / 0 MISSING of 18 golden items
```

Every pin's on-disk frame hashes to its pinned value — including s28's
`ff68fad3…`. Note the division of labour: this verify proves the *pack* is
intact; §5's DRIFT is a separate comparison (fresh render vs that same pin)
and is unaffected. Corroborating evidence: Tools CI @ `3223509` success
includes the golden-pack step per tools-ci's contract header, and W68
rehearsed the same verify at pack state `2755ff3` (18 OK, PC-DRYRUN-W68 §4).
No dispatch is needed to close this row — it is closed at the frozen HEAD.

## §8 Film telemetry — last observed CHECK OK (binary of `b6ee9fb`)

Telemetry is a CPU binary mode, not a workflow (render.yml's dispatch modes
are `ab1 / dof-chart / film / scene-file` only). Its last observed verdict:

- **`FILM TELEMETRY: CHECK OK`, exit 0** — W68's local dry-run
  (PC-DRYRUN-W68.md §3.1), executed with the binary artifact of CI run
  `43624139322` (CI **success**, head **`b6ee9fb`** — head and conclusion
  gh-verified by this seat). The PERF §4 cap-24 leg in the same note reached
  the identical verdict line (its own exit code was not observed — W68
  discloses its wrapper was killed; §3.1's main leg has both the line and
  exit 0 observed).

**No telemetry observation exists at any head after `b6ee9fb`, and none at
the frozen HEAD.** This matters more than usual: the `--tables` landing
(`3223509`) changed telemetry's code, and the frozen HEAD's one CI failure
(§1) is a unit test *inside* that new code — the tables implementation has
not yet had a full film-telemetry run at any head containing it. Closing
action (lead/orchestrator): fix the §1 test → green CI at HEAD → download
the `talerender-binary` artifact → run `--film-telemetry` (and `--tables`)
locally.

## §9 What closes each gap at the frozen HEAD (for the lead / orchestrator)

| # | Gap | Closing action |
|---|---|---|
| 1 | CI red at `9380b01` — 1 test (§1) | Fix `tables_beat_frames_tie_to_blink_at` (lead; W93 diff-check informs); re-push → CI completes |
| 2 | No binary artifact at HEAD (§1 consequence) | Follows #1 (artifact steps were skipped behind the Test failure) |
| 3 | Tools CI unrun at HEAD (§2) | Next push touching a watched path (`tools/**`, `src/**`, …); no dispatch trigger exists |
| 4 | Knobs unrun at HEAD (§3) | `gh workflow run knobs.yml` |
| 5 | Determinism unrun at HEAD (§4) | `gh workflow run determinism.yml` |
| 6 | VR unrun at HEAD (§5) | `gh workflow run visual-regression.yml` — as-designed failure expected until the s28 decision |
| 7 | Bench unrun at HEAD (§6) | `gh workflow run bench.yml` |
| 8 | Telemetry unobserved at HEAD (§8) | After #1: HEAD binary artifact → local `--film-telemetry` run |

## §10 Hygiene

Working tree at observation: `docs/integration/PC-DRYRUN-W68.md` untracked
(W68's landed note, awaiting the central Batch-10 commit) and
`tools/format/exporter_acceptance.py` modified-uncommitted (W77's seat still
iterating, per gate W90). Neither affects any run evidence above; HEAD itself
is the Batch-9 commit `9380b01`.
