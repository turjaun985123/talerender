# KNOBS-GATE-W81 — F4 knobs gate acceptance, independently verified

**Seat:** Wave 2 · Batch 9 · W81. **Scope:** verify the F4 acceptance
run of the knobs gate (`tools/ci/knobs_gate.py` via
`.github/workflows/knobs.yml`) against the prescriptions in
KNOBS-PATCH-W54 §10 and KNOBS-P23-W73 §4. New file only; nothing
else touched, nothing committed.

## Verdict: F4 ACCEPTED

The knobs gate ran on a real CI dispatch at a named head and
passed all three legs, with the exact verdict line the packs
prescribe. Two independent dispatches at the same head agree
frame-for-frame.

## 1. The runs of record

| | run | dispatched (UTC) | head | conclusion |
|---|---|---|---|---|
| First dispatch (the lead's — the F4 acceptance run) | [43628289244](https://github.com/turjaun985123/talerender/actions/runs/43628289244) | 2026-11-14T18:44:36Z | `c440d90c7a01c3e81c90ecc7e4e8b643164de246` | **success** |
| Corroborating dispatch (this seat's) | [43628339967](https://github.com/turjaun985123/talerender/actions/runs/43628339967) | 2026-11-14T18:45:10Z | `c440d90c7a01c3e81c90ecc7e4e8b643164de246` | **success** |

Disclosure: when this seat went to dispatch, no knobs-gate run
existed yet in the run list it first read; the lead's dispatch
landed 34 s before this seat's own, so both ran. Neither was
cancelled; both completed. Head `c440d90` is the lead's
delimiter fix on top of `2017e86` (the workflow wiring) — i.e.
the gate ran against the tree that contains phases 1–3
(`ea98458` / `9ad2177` / `76b922c`) plus that fix.

**Verdict line, verbatim** (final line of `knobs-results.txt`,
both runs; the workflow's Verdict step greps for exactly this
line with `grep -qx` and passed in both):

```
KNOBS GATE: PASS
```

Environment, per the workflow: ubuntu-22.04, Mesa lavapipe (ICD
pinned), mold, rust-cache. A lavapipe PASS discharges the CI
leg; RADV confirmation belongs to the PC evidence session
(workflow header, stated plainly there).

## 2. Leg-by-leg evidence audit

All quotations are from the run logs / the uploaded
`knobs-results.txt` of both runs; the two runs' gate sections
are identical in every figure quoted below.

### Leg A — defaults identity: FULLY evidenced

Six renders (plain vs `--res 1 --aa 4 --shadow 2048`) over the
three standing probe ranges from `byte_probes.DEFAULT_RANGES`,
compared via byte_probes imported (not forked):

```
  range 195-204: PASS (10/10 identical)
  range 795-814: PASS (20/20 identical)
  range 1195-1204: PASS (10/10 identical)
  BYTE PROBES: PASS (40/40 frames identical across 3 range(s))
leg A: PASS (defaults are byte-identical, stdout clean)
```

Matches pack §10 exactly: 40/40 byte-identical, and the
"stdout clean" half is the tool's `stdout_checks(..., draft=False)`
assertion over **both** runs' stdout (no `DRAFT` substring in
either) — asserted in `tools/ci/knobs_gate.py` leg-A code and
reported in its PASS line.

### Leg B — draft smoke: FULLY evidenced, one indirection (§3)

Frames 195–219 with `--draft` (25 frames, one progress line):

```
  ok: header line present
  ok: header carries the exact draft tag
  ok: completion line present
  ok: completion repeats the draft tag
  ok: at least one progress line printed
  ok: progress line carries a DRAFT tag
  range 195-204: FAIL (0/10 identical, 10 differ)
    first difference: frame 195 (frame_00195.png) at byte 18 (2804969 bytes differ; sizes 2814097 baseline / 716845 candidate)
      sha256 baseline  fb25e36b8fd76b14542270e8f78aeadaee528e67645d8878a9eac55734f02f5f
      sha256 candidate 2eaa72e3fdbd6c999020141070a81083544a4d3112b51df36d2605c5c91d52a4
  BYTE PROBES: FAIL (10 frame(s) differ in 1 of 1 range(s))
leg B: PASS (draft differs; 1080x1920 -> 540x960; tag in header, progress, completion)
```

- The "exact draft tag" is the tool's literal constant
  `DRAFT_TAG = "[DRAFT res=0.5 aa=0 shadow=512 — NOT FOR DELIVERY]"`
  (`tools/ci/knobs_gate.py:97`), substring-asserted against the
  header and completion lines; the progress line is asserted to
  contain `DRAFT` and `NOT FOR DELIVERY`. This is the exact
  string pack §10 prescribes.
- The differ assertion is the prescribed one: byte_probes must
  **FAIL** (an identical draft would mean the knobs are
  decorative) — 0/10 identical, first difference quoted above.
- Size: draft frame IHDR measured against spec arithmetic on
  the plain size — 1080x1920 → **540x960**, as the PASS line
  records.
- Cross-run bonus: the frame-195 sha256 pair is identical in
  both independent runs — the plain and draft frames are
  byte-stable across dispatches on lavapipe.

### Leg C — CLI validation: FULLY evidenced, one indirection (§3)

All ten prescribed cases (W73 §4's table, pack §10's "ten
bad/missing values"):

```
  ok: --res 1.5 -> exit 2
  ok: --res 0 -> exit 2
  ok: --res -0.5 -> exit 2
  ok: --aa 3 -> exit 2
  ok: --aa 16 -> exit 2
  ok: --shadow 1000 -> exit 2
  ok: --shadow 0 -> exit 2
  ok: --res -> exit 2
  ok: --aa -> exit 2
  ok: --shadow -> exit 2
leg C: PASS
```

Each `ok` is the tool's three-part check (`knobs_gate.py:290–293`):
exit code 2 **and** `error:` present in stderr **and**
`unknown argument` absent from stderr, run against
`--scene m1` so validation fires in the parse loop before any
GPU work.

## 3. Evidence thinness — named, as required

Two places where the preserved evidence is thinner than the
prescription's spirit, neither of which blocks acceptance:

1. **Leg C's stderr wording is asserted, not preserved.** On
   success the tool echoes only the flag and exit code; the
   actual refusal text per case is not written to the log or to
   `knobs-results.txt` (it is quoted only on failure). The
   message *shape* (exit 2 + `error:` + no `unknown argument`)
   is genuinely checked — verified in the tool source — but a
   later reader cannot re-audit the wording itself from the
   run's artefacts.
2. **Leg B's tagged lines are asserted, not echoed.** The
   header/progress/completion lines carrying the tag are not
   printed into the report; only the assertion results are.
   The comparison string is the spec's literal tag (tool source
   line 97), so the check is exact — but, as with leg C, the
   evidence is the assertion, not the line.

And the standing residual gap, disclosed in both packs and
unchanged by this run: the gate exercises the defaults, the
all-off-default `--draft` combination, and invalid values.
A defect visible only at one intermediate value alone
(`--aa 2` solo, `--shadow 1024` solo) passes this gate; those
spot checks belong to the PC evidence session. This is a
design limit of the gate, not a shortfall of the run.

## 4. Self-test + workflow-shape confirmation (this seat, first-hand)

- `python3 tools/ci/knobs_gate.py --self-test` re-run twice at
  the seat's checkout: **8/8 `ok:` lines + `SELF-TEST: PASS`**
  (identity vs frames4, one-byte-flip difference, IHDR size,
  spec arithmetic incl. res-1 identity on an odd base, tag
  logic ×3). The self-test covers comparison/size/tag logic
  only — it makes no claim about the binary legs, and neither
  does this note on its basis.
- `.github/workflows/knobs.yml` invokes exactly the command
  pack §10 / W73 §4 specify:
  `python3 tools/ci/knobs_gate.py ./target/release/talerender --film-dir film --work "$RUNNER_TEMP/knobs"`,
  followed by the verdict step
  `grep -qx "KNOBS GATE: PASS" "$RUNNER_TEMP/knobs/knobs-results.txt"`.
  No leg selector exists or is needed; the invocation is the
  monolithic one the packs prescribed, verbatim.

## 5. F4 verdict

**ACCEPTED.** The F4 knobs implementation (phases 1–3) passes
its designed acceptance gate on a real CI dispatch at head
`c440d90c7a01c3e81c90ecc7e4e8b643164de246` — run 43628289244
(the lead's first dispatch), corroborated by run 43628339967
(this seat's dispatch, same head, same verdict, identical
frame hashes). The only open F4-adjacent item is the packs'
own disclosed remainder: intermediate-value spot checks on
the user's RADV PC, which were never part of this gate.
