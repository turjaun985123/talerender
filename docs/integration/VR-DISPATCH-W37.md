# Visual-regression first-dispatch triage pack — W37 (Phase 5)

**Deliverables (this seat, new files only):**

- `tools/ci/vr_triage.py` — the triage tool (stdlib only).
- `tools/ci/fixtures/vr_triage/` — three synthetic runner reports
  in `visual_regression.py`'s exact output format.
- This note.

**Status: written and locally validated; the workflow itself has
still never been dispatched.** W16's caveat stands untouched —
`visual-regression.yml` is proven only by a real dispatch, and
the first one's expected result is the documented FAIL on the
re-pin debt. This pack exists so that when that red job lands,
nobody has to re-derive which rows are the known debt and which
rows are news. Nothing here edits the workflow or the runner,
widens the 80 dB band, or whitelists a pin.

## 1. The problem this solves

A correct first dispatch produces a **red job**: the runner
prints `VISUAL REGRESSION: FAIL (3 pins, 2 drift)` and exits 1.
That red is the re-pin debt surfacing, not a renderer regression
(VISUAL-REGRESSION-W16.md §4; the workflow's own error annotation
says so). But "expected to fail" is not the same as "any failure
is expected": a first dispatch could also contain real drift, a
broken render step, or a missing reference, and those must not
hide inside the debt's shadow. The triage tool reads the
runner's uploaded report (`visual-regression-<run_id>` artifact →
`visual-regression.md`) and classifies **every non-MATCH row** as
`KNOWN-DEBT` (registered below, with the act that clears it) or
`NEW-SIGNAL` (anything else — the only rows needing human
attention), ending in one verdict line:

- `TRIAGE VERDICT: first-dispatch result is AS EXPECTED`
  (tool exit 0)
- `TRIAGE VERDICT: contains NEW SIGNALS: <id> (<status>), …`
  (tool exit 1)
- unparseable input: one `ERROR:` line on stderr, **no verdict**,
  exit 2 — the runner's own refusal discipline.

## 2. The expected first dispatch (the register)

Repo state the register describes: `golden-hashes.json` pins
exactly 4 items; the 14 film mids are staged candidates,
unpinned. The workflow renders `ab1-composer` and
`s28-onoda-t150` (both from the one ab1 render), `dof-chart`,
and `film-s08b`.

| pin | expected | why | clears with |
|---|---|---|---|
| `ab1-composer` | DRIFT (or WITHIN-BAND — same debt, milder) | pin predates the shade root-fix `c00897d`; EXPECTED STALE, "certain" (`tools/golden/RUNNER.md`) | **GOLDEN-RESTAGE-W03 Act 2** — human re-pin per `docs/golden/repin-packets/ab1-composer.md`, its own commit, diff scoped to the one pin line |
| `s28-onoda-t150` | DRIFT (or WITHIN-BAND) | same pre-`c00897d` pin vintage | **neither W03 act** — see the warning below |
| `dof-chart` | **MATCH** | the clean control: its chart texture never passes through `shade_rgb` | — (any non-MATCH is a NEW-SIGNAL) |
| `stroke-toon` | NOT-RENDERED | the workflow renders no battery card set (different output contract) | a future deliberate act: re-baseline the founding pin + add a battery render to the workflow (W16 §4) |
| `film-s01`…`film-s12` (14) | UNPINNED | staged candidates are not references; reported only, never failed, no PSNR computed | **GOLDEN-RESTAGE-W03 Act 1** — human first-pins from `docs/golden/frames-f021991/` (one commit, exactly 14 pin lines added, the 4 milestone pins unmoved) |

Expected totals: runner `FAIL (3 pins, 2 drift)`; counts
`1 MATCH · 2 DRIFT · 14 UNPINNED · 1 NOT-RENDERED (of 18)`;
triage `1 OK · 17 KNOWN-DEBT · 0 NEW-SIGNAL` → **AS EXPECTED**.

> **⚠ The s28 warning, stated plainly.** W03 §5 explicitly does
> **not** pin `s28-onoda-t150` in either act — the repin packet
> recommends against re-pinning it (historical prototype
> artefact). So after Acts 1 and 2 both land, this job is
> expected to **stay red on s28 alone** until a human makes a
> separate disposition of that pin (a deliberate re-pin anyway,
> or retiring the pin), recorded per `docs/golden/NATIVE-GOLDEN.md`.
> A future reader who sees "FAIL (3 pins, 1 drift), s28 only"
> after the acts is looking at this paragraph, not at a new
> regression. The triage register says the same in the s28 row's
> clearing action.

Everything not in the table is a NEW-SIGNAL by construction:
`MISSING-RENDER` / `REFERENCE-MISSING` on any item (run / repo
breakage is never debt), any non-MATCH on the `dof-chart`
control, a registered item in an unregistered state, an unknown
item id, or a registry item missing from the report (truncated
artifact).

## 3. Pre-dispatch checklist

Before dispatching `visual-regression.yml` for the first time,
confirm — in this order:

1. **The register still describes the repo.** `golden-hashes.json`
   still pins exactly the 4 founding items and neither pin act
   has landed: `python3 tools/golden/make_golden.py` reads the
   on-disk state as 4 OK / 0 STALE / 14 MISSING, and the W03
   candidate staging (`docs/golden/frames-f021991/`) has not been
   copied over `docs/golden/frames/`. If an act *has* landed,
   stop: update `REGISTER` in `vr_triage.py` in the same commit
   as the act *first* (§6), or the triage will manufacture
   NEW-SIGNALs out of cleared debt.
2. **CI is green at the dispatch head** (`ci.yml` + tools-ci on
   the commit being dispatched). Triage compares against pins,
   not against a broken build.
3. **You have read §2's expected totals.** The job going red is
   the designed outcome; the person dispatching should be able
   to say in advance: FAIL (3 pins, 2 drift), dof-chart MATCH.
   If you cannot, re-read VISUAL-REGRESSION-W16.md §4 — dispatch
   is the expensive way to learn it.
4. **Artifact window.** The report artifact
   (`visual-regression-<run_id>`) is retained **7 days**. Plan
   to download `visual-regression.md` and run the triage inside
   that window; an expired artifact leaves the run un-triaged.
5. **Nobody pre-"fixes" the red.** No `--band` change, no
   whitelist, no skipping the failing pins in the workflow. The
   only exits from the debt are the acts in §2's table. If
   someone proposes one of these instead, that proposal is the
   finding.

## 4. Running the triage

```sh
# after downloading the run artifact:
python3 tools/ci/vr_triage.py visual-regression.md   # or: … -
echo $?   # 0 = AS EXPECTED · 1 = NEW SIGNALS · 2 = refused
```

Self-check at any time (also the fastest way to see both verdict
shapes before the real dispatch):

```sh
python3 tools/ci/vr_triage.py --self-test
```

## 5. How to read the outcome — and the follow-up per class

The runner's verdict and the triage verdict answer different
questions: the runner asks "did every pin reproduce?", the
triage asks "is what happened the thing we predicted?".

| runner | triage | meaning | action |
|---|---|---|---|
| FAIL | AS EXPECTED | the job works; it found exactly the registered debt; `dof-chart` MATCH proves the chain end-to-end | proceed to the pin acts (§2); no investigation |
| FAIL | NEW SIGNALS | debt **and** news | investigate the named NEW-SIGNAL rows first (below); the debt rows still clear only via their acts |
| PASS | AS EXPECTED + Register note(s) | the debt items report MATCH — a clearing act has already landed (or the pin state moved) | confirm the act's commit exists and update `REGISTER` in it (§6); then the job is simply green |
| FAIL/PASS | exit 2, no verdict | the file is not a complete runner report (truncated artifact, or the runner itself refused with exit 2 and wrote no report) | fix the artifact/run first; a refused run triages nothing and proves nothing |

Follow-ups per class:

- **KNOWN-DEBT `ab1-composer`** → Act 2 (§2). One commit; the
  `golden-hashes.json` diff must be exactly the one
  `ab1-composer` line (`d7bd71a0…` → `a415ead8…` if the packet's
  `c00897d` candidate is used, or a fresh ab1 dispatch's bytes
  per W03 §4 — the packet's measure-and-name-the-cause procedure
  applies either way).
- **KNOWN-DEBT `s28-onoda-t150`** → no scheduled act clears it
  (§2 warning). It needs its own human disposition decision;
  until then it is expected-red and blocks nobody's
  investigation of other rows.
- **KNOWN-DEBT `film-*` UNPINNED** → Act 1 (§2). After it lands,
  `film-s08b` starts *gating* in this workflow (it is the one
  film mid rendered); the other 13 gate only when a production
  dispatch renders them.
- **KNOWN-DEBT `stroke-toon` NOT-RENDERED** → future act (§2);
  nothing to do at first dispatch.
- **NEW-SIGNAL DRIFT on `dof-chart`** → stop. Real same-renderer
  drift until explained: bisect the renderer changes since the
  pin was struck, compare against the determinism probes. Do
  **not** re-pin it (that hides a regression) and do not widen
  the band.
- **NEW-SIGNAL DRIFT / WITHIN-BAND anywhere else unregistered**
  → same discipline: byte-identity is the post-`cabcc30` norm;
  find the cause or the explanation before accepting the run.
- **NEW-SIGNAL MISSING-RENDER** → the render step broke, not
  the renderer. Read the workflow's "Render the pinned
  references" step log and the `visual-out` artifact; fix the
  run and re-dispatch.
- **NEW-SIGNAL REFERENCE-MISSING** → repo breakage: restore the
  pinned reference file or account for its removal in a commit.
  Never re-pin around a missing reference.
- **NEW-SIGNAL unexpected state / unknown id / absent row** →
  the pin state, render set, registry, or report format moved
  since the register was written. Reconcile deliberately, in a
  commit — never by editing the register mid-incident to make
  the output quiet.

## 6. Register maintenance

`REGISTER` in `vr_triage.py` is a snapshot of the pre-pin repo
state, and it is only valid while §3 item 1 holds. When a pin
act lands, update the register **in the same commit**:

- after Act 1: `film-s08b` → expect MATCH; the other film ids
  leave the register as debt (once pinned, a non-MATCH on them
  is a NEW-SIGNAL — which is the gate working as designed);
- after Act 2: `ab1-composer` leaves the register the same way;
- after an s28 disposition: `s28-onoda-t150` leaves the register
  (re-pinned → non-MATCH is a signal; pin retired → the row
  disappears from the report and from `EXPECTED_IDS`).

The tool helps notice a stale register — a debt item reporting
MATCH prints a `Register note:` line (informational; it never
changes the verdict) — but noticing is the tool's job; updating
is a human's, in the act's own commit.

## 7. Validation — both runs verbatim

Fixtures are synthetic reports in the runner's exact format
(header, row order from `make_golden.py` ITEMS, note texts from
`check_item()`). Their PSNR / rendered-sha figures are
illustrative — the statuses, counts, and verdict lines are the
load-bearing content. (The planted dof-chart row reuses W16
Run 3's measured σ=10 figures, 28.13 dB.) Full self-test:
`SELF-TEST: PASS (3/3 fixtures triaged as designed)`, exit 0.

**Run A — synthetic replay of the expected first dispatch**
(`expected-first-dispatch.md`) → **AS EXPECTED, exit 0:**

```
# Visual-regression triage — first dispatch

Runner verdict: VISUAL REGRESSION: FAIL (3 pins, 2 drift)
Band: 80 dB — unchanged. Nothing in this triage widens the band or whitelists a pin; KNOWN-DEBT rows clear only through the pin acts named in their follow-ups (docs/integration/VR-DISPATCH-W37.md).

| pin | runner status | PSNR (dB) | triage | clearing action / follow-up |
|---|---|---|---|---|
| s28-onoda-t150 | DRIFT | 41.27 | KNOWN-DEBT | Stale pin: predates c00897d, same debt as ab1-composer — but cleared by NEITHER W03 act (the repin packet recommends against re-pinning this historical prototype artefact). Clears only with a separate human disposition of the s28 pin (deliberate re-pin or pin retirement, recorded per NATIVE-GOLDEN.md). Expect this row to keep failing after Acts 1-2 land. |
| dof-chart | MATCH | inf | OK | none — byte-identical to the pin |
| stroke-toon | NOT-RENDERED | — | KNOWN-DEBT | Expected: the workflow renders no battery card set (different output contract). Clears with a future deliberate act — re-baseline the founding pin and add a battery render to the workflow (VISUAL-REGRESSION-W16 §4). |
| ab1-composer | DRIFT | 38.94 | KNOWN-DEBT | Stale pin: predates the shade root-fix c00897d (EXPECTED STALE, tools/golden/RUNNER.md). Clears with GOLDEN-RESTAGE-W03 Act 2 — human re-pin per docs/golden/repin-packets/ab1-composer.md, its own commit, diff scoped to the one ab1-composer pin line. |
| film-s01 | UNPINNED | — | KNOWN-DEBT | Clears with GOLDEN-RESTAGE-W03 Act 1 — human first-pins of the 14 film mids from docs/golden/frames-f021991/ (one commit, exactly 14 pin lines added, milestone pins unmoved). Until pinned, UNPINNED is reported-only by design and gates nothing. |
| film-s02 | UNPINNED | — | KNOWN-DEBT | Clears with GOLDEN-RESTAGE-W03 Act 1 — human first-pins of the 14 film mids from docs/golden/frames-f021991/ (one commit, exactly 14 pin lines added, milestone pins unmoved). Until pinned, UNPINNED is reported-only by design and gates nothing. |
| film-s03 | UNPINNED | — | KNOWN-DEBT | Clears with GOLDEN-RESTAGE-W03 Act 1 — human first-pins of the 14 film mids from docs/golden/frames-f021991/ (one commit, exactly 14 pin lines added, milestone pins unmoved). Until pinned, UNPINNED is reported-only by design and gates nothing. |
| film-s04 | UNPINNED | — | KNOWN-DEBT | Clears with GOLDEN-RESTAGE-W03 Act 1 — human first-pins of the 14 film mids from docs/golden/frames-f021991/ (one commit, exactly 14 pin lines added, milestone pins unmoved). Until pinned, UNPINNED is reported-only by design and gates nothing. |
| film-s05a | UNPINNED | — | KNOWN-DEBT | Clears with GOLDEN-RESTAGE-W03 Act 1 — human first-pins of the 14 film mids from docs/golden/frames-f021991/ (one commit, exactly 14 pin lines added, milestone pins unmoved). Until pinned, UNPINNED is reported-only by design and gates nothing. |
| film-s05b | UNPINNED | — | KNOWN-DEBT | Clears with GOLDEN-RESTAGE-W03 Act 1 — human first-pins of the 14 film mids from docs/golden/frames-f021991/ (one commit, exactly 14 pin lines added, milestone pins unmoved). Until pinned, UNPINNED is reported-only by design and gates nothing. |
| film-s06 | UNPINNED | — | KNOWN-DEBT | Clears with GOLDEN-RESTAGE-W03 Act 1 — human first-pins of the 14 film mids from docs/golden/frames-f021991/ (one commit, exactly 14 pin lines added, milestone pins unmoved). Until pinned, UNPINNED is reported-only by design and gates nothing. |
| film-s07 | UNPINNED | — | KNOWN-DEBT | Clears with GOLDEN-RESTAGE-W03 Act 1 — human first-pins of the 14 film mids from docs/golden/frames-f021991/ (one commit, exactly 14 pin lines added, milestone pins unmoved). Until pinned, UNPINNED is reported-only by design and gates nothing. |
| film-s08 | UNPINNED | — | KNOWN-DEBT | Clears with GOLDEN-RESTAGE-W03 Act 1 — human first-pins of the 14 film mids from docs/golden/frames-f021991/ (one commit, exactly 14 pin lines added, milestone pins unmoved). Until pinned, UNPINNED is reported-only by design and gates nothing. |
| film-s08b | UNPINNED | — | KNOWN-DEBT | Clears with GOLDEN-RESTAGE-W03 Act 1 — human first-pins of the 14 film mids from docs/golden/frames-f021991/ (one commit, exactly 14 pin lines added, milestone pins unmoved). Until pinned, UNPINNED is reported-only by design and gates nothing. |
| film-s09 | UNPINNED | — | KNOWN-DEBT | Clears with GOLDEN-RESTAGE-W03 Act 1 — human first-pins of the 14 film mids from docs/golden/frames-f021991/ (one commit, exactly 14 pin lines added, milestone pins unmoved). Until pinned, UNPINNED is reported-only by design and gates nothing. |
| film-s10 | UNPINNED | — | KNOWN-DEBT | Clears with GOLDEN-RESTAGE-W03 Act 1 — human first-pins of the 14 film mids from docs/golden/frames-f021991/ (one commit, exactly 14 pin lines added, milestone pins unmoved). Until pinned, UNPINNED is reported-only by design and gates nothing. |
| film-s11 | UNPINNED | — | KNOWN-DEBT | Clears with GOLDEN-RESTAGE-W03 Act 1 — human first-pins of the 14 film mids from docs/golden/frames-f021991/ (one commit, exactly 14 pin lines added, milestone pins unmoved). Until pinned, UNPINNED is reported-only by design and gates nothing. |
| film-s12 | UNPINNED | — | KNOWN-DEBT | Clears with GOLDEN-RESTAGE-W03 Act 1 — human first-pins of the 14 film mids from docs/golden/frames-f021991/ (one commit, exactly 14 pin lines added, milestone pins unmoved). Until pinned, UNPINNED is reported-only by design and gates nothing. |

Counts: 1 OK · 17 KNOWN-DEBT · 0 NEW-SIGNAL (of 18 rows).

TRIAGE VERDICT: first-dispatch result is AS EXPECTED
```

**Run B — same report with planted drift on the control**
(`planted-new-drift.md`: `dof-chart` DRIFT 28.13 dB, runner
`FAIL (3 pins, 3 drift)`) → **NEW-SIGNAL, exit 1:**

```
# Visual-regression triage — first dispatch

Runner verdict: VISUAL REGRESSION: FAIL (3 pins, 3 drift)
Band: 80 dB — unchanged. Nothing in this triage widens the band or whitelists a pin; KNOWN-DEBT rows clear only through the pin acts named in their follow-ups (docs/integration/VR-DISPATCH-W37.md).

| pin | runner status | PSNR (dB) | triage | clearing action / follow-up |
|---|---|---|---|---|
| s28-onoda-t150 | DRIFT | 41.27 | KNOWN-DEBT | Stale pin: predates c00897d, same debt as ab1-composer — but cleared by NEITHER W03 act (the repin packet recommends against re-pinning this historical prototype artefact). Clears only with a separate human disposition of the s28 pin (deliberate re-pin or pin retirement, recorded per NATIVE-GOLDEN.md). Expect this row to keep failing after Acts 1-2 land. |
| dof-chart | DRIFT | 28.13 | NEW-SIGNAL | dof-chart is the clean control (its chart texture never passes through shade_rgb): only MATCH is expected. Investigate before anything else: same-renderer drift on an item carrying no re-pin debt. Bisect the renderer changes since the pin was struck; do NOT re-pin to silence it (that would hide a regression) and do NOT widen the band. |
| stroke-toon | NOT-RENDERED | — | KNOWN-DEBT | Expected: the workflow renders no battery card set (different output contract). Clears with a future deliberate act — re-baseline the founding pin and add a battery render to the workflow (VISUAL-REGRESSION-W16 §4). |
| ab1-composer | DRIFT | 38.94 | KNOWN-DEBT | Stale pin: predates the shade root-fix c00897d (EXPECTED STALE, tools/golden/RUNNER.md). Clears with GOLDEN-RESTAGE-W03 Act 2 — human re-pin per docs/golden/repin-packets/ab1-composer.md, its own commit, diff scoped to the one ab1-composer pin line. |
| film-s01 | UNPINNED | — | KNOWN-DEBT | Clears with GOLDEN-RESTAGE-W03 Act 1 — human first-pins of the 14 film mids from docs/golden/frames-f021991/ (one commit, exactly 14 pin lines added, milestone pins unmoved). Until pinned, UNPINNED is reported-only by design and gates nothing. |
| film-s02 | UNPINNED | — | KNOWN-DEBT | Clears with GOLDEN-RESTAGE-W03 Act 1 — human first-pins of the 14 film mids from docs/golden/frames-f021991/ (one commit, exactly 14 pin lines added, milestone pins unmoved). Until pinned, UNPINNED is reported-only by design and gates nothing. |
| film-s03 | UNPINNED | — | KNOWN-DEBT | Clears with GOLDEN-RESTAGE-W03 Act 1 — human first-pins of the 14 film mids from docs/golden/frames-f021991/ (one commit, exactly 14 pin lines added, milestone pins unmoved). Until pinned, UNPINNED is reported-only by design and gates nothing. |
| film-s04 | UNPINNED | — | KNOWN-DEBT | Clears with GOLDEN-RESTAGE-W03 Act 1 — human first-pins of the 14 film mids from docs/golden/frames-f021991/ (one commit, exactly 14 pin lines added, milestone pins unmoved). Until pinned, UNPINNED is reported-only by design and gates nothing. |
| film-s05a | UNPINNED | — | KNOWN-DEBT | Clears with GOLDEN-RESTAGE-W03 Act 1 — human first-pins of the 14 film mids from docs/golden/frames-f021991/ (one commit, exactly 14 pin lines added, milestone pins unmoved). Until pinned, UNPINNED is reported-only by design and gates nothing. |
| film-s05b | UNPINNED | — | KNOWN-DEBT | Clears with GOLDEN-RESTAGE-W03 Act 1 — human first-pins of the 14 film mids from docs/golden/frames-f021991/ (one commit, exactly 14 pin lines added, milestone pins unmoved). Until pinned, UNPINNED is reported-only by design and gates nothing. |
| film-s06 | UNPINNED | — | KNOWN-DEBT | Clears with GOLDEN-RESTAGE-W03 Act 1 — human first-pins of the 14 film mids from docs/golden/frames-f021991/ (one commit, exactly 14 pin lines added, milestone pins unmoved). Until pinned, UNPINNED is reported-only by design and gates nothing. |
| film-s07 | UNPINNED | — | KNOWN-DEBT | Clears with GOLDEN-RESTAGE-W03 Act 1 — human first-pins of the 14 film mids from docs/golden/frames-f021991/ (one commit, exactly 14 pin lines added, milestone pins unmoved). Until pinned, UNPINNED is reported-only by design and gates nothing. |
| film-s08 | UNPINNED | — | KNOWN-DEBT | Clears with GOLDEN-RESTAGE-W03 Act 1 — human first-pins of the 14 film mids from docs/golden/frames-f021991/ (one commit, exactly 14 pin lines added, milestone pins unmoved). Until pinned, UNPINNED is reported-only by design and gates nothing. |
| film-s08b | UNPINNED | — | KNOWN-DEBT | Clears with GOLDEN-RESTAGE-W03 Act 1 — human first-pins of the 14 film mids from docs/golden/frames-f021991/ (one commit, exactly 14 pin lines added, milestone pins unmoved). Until pinned, UNPINNED is reported-only by design and gates nothing. |
| film-s09 | UNPINNED | — | KNOWN-DEBT | Clears with GOLDEN-RESTAGE-W03 Act 1 — human first-pins of the 14 film mids from docs/golden/frames-f021991/ (one commit, exactly 14 pin lines added, milestone pins unmoved). Until pinned, UNPINNED is reported-only by design and gates nothing. |
| film-s10 | UNPINNED | — | KNOWN-DEBT | Clears with GOLDEN-RESTAGE-W03 Act 1 — human first-pins of the 14 film mids from docs/golden/frames-f021991/ (one commit, exactly 14 pin lines added, milestone pins unmoved). Until pinned, UNPINNED is reported-only by design and gates nothing. |
| film-s11 | UNPINNED | — | KNOWN-DEBT | Clears with GOLDEN-RESTAGE-W03 Act 1 — human first-pins of the 14 film mids from docs/golden/frames-f021991/ (one commit, exactly 14 pin lines added, milestone pins unmoved). Until pinned, UNPINNED is reported-only by design and gates nothing. |
| film-s12 | UNPINNED | — | KNOWN-DEBT | Clears with GOLDEN-RESTAGE-W03 Act 1 — human first-pins of the 14 film mids from docs/golden/frames-f021991/ (one commit, exactly 14 pin lines added, milestone pins unmoved). Until pinned, UNPINNED is reported-only by design and gates nothing. |

Counts: 0 OK · 17 KNOWN-DEBT · 1 NEW-SIGNAL (of 18 rows).

TRIAGE VERDICT: contains NEW SIGNALS: dof-chart (DRIFT)
```

**Run C — planted run breakage** (`planted-missing-render.md`:
`dof-chart` MISSING-RENDER) → **NEW-SIGNAL, exit 1** — proving
infrastructure failure cannot pass as debt:

```
| dof-chart | MISSING-RENDER | — | NEW-SIGNAL | The render step produced no file — run breakage, not drift. Check the workflow's render step log and the visual-out artifact; fix the run, then re-dispatch. |

TRIAGE VERDICT: contains NEW SIGNALS: dof-chart (MISSING-RENDER)
```

**Refusal check** — non-report input on stdin prints
`ERROR: vr_triage.py: no 'VISUAL REGRESSION: PASS/FAIL …' verdict
line found …` on stderr, no verdict line, exit 2.

## 8. Limits and what is NOT proven

- **No real dispatch has happened.** The fixtures are synthetic;
  the first real report may differ in ways no fixture covers
  (e.g. the stale pins' true PSNRs are unmeasured — "EXPECTED
  STALE" is a hash-level claim from the golden pack, and the
  DRIFT-vs-WITHIN-BAND form the debt takes on the day is only
  known when the render exists; the register accepts both for
  exactly that reason).
- **The register is a snapshot, not an oracle.** It encodes the
  pre-pin repo state and the W03 acts as written today. It
  cannot see a re-pin that lands without the §6 register update —
  it will call the resulting MATCH rows out via `Register note:`
  lines, and any *new* pin's first DRIFT will read NEW-SIGNAL,
  which is the safe direction to err in.
- **Triage is not a gate change.** The workflow still passes
  only on runner exit 0 + the PASS line; this pack runs
  afterwards, by hand, on the artifact. A triage of AS EXPECTED
  does not make the red job green — only the pin acts do that.

*Seat W37 — files written, nothing committed, no existing file
touched (`.github/workflows/visual-regression.yml` and
`tools/ci/visual_regression.py` read, never edited).*
