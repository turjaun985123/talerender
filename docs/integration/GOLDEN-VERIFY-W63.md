# GOLDEN-VERIFY-W63 — golden pin verification + post-pin VR triage

Wave 2, Batch 7, seat W63. Independent verification of the two golden
pin acts (Act 1 `916a109`, Act 2 `166873c`) and triage of the
visual-regression dispatches around them. Everything below is from
this seat's own runs of the tools (`make_golden.py`, `vr_triage.py`,
`gh`, `git show`) unless a figure is explicitly labelled as the
lead's. Nothing committed by this seat; no existing file edited.

**HEAD movement during this session (material to §1/§4):** the
session began at `4dd6530` (the task's stated `3a3a427` or later).
While verification was running, the lead landed `2755ff3` — the
disclosed CORRECTIVE re-pin of Act 1 — which is HEAD at note time.
Both states were verified; the distinction is kept throughout.

## 1. Golden pack verification — my runs

Command (per `docs/golden/NATIVE-GOLDEN.md`): `python3 tools/golden/make_golden.py`.

- At `4dd6530` (original Act 1 + Act 2 pins): **18 OK / 0 STALE / 0
  MISSING**, exit 0.
- At `2755ff3` (after the corrective re-pin): **18 OK / 0 STALE / 0
  MISSING**, exit 0. Corrected pin values spot-checked against the
  registry file: film-s01 `1608ead9…`, film-s08b `fb4e2ba4…`,
  film-s12 `dfabc09a…` — all equal to GOLDEN-RESTAGE-W03 §2's
  f021991-candidate column (see §4).

**Scope caveat, proven by §4:** `make_golden` verifies that each
registry hash equals the staged file on disk. It proves internal
consistency of the pack, *not* that a pin describes current
production output. The original Act 1 passed this check 18/18 while
11 of its film pins described superseded bytes. The control that
caught it was the VR canary (§5), not the pack check.

## 2. The two acts — diffs verified first-hand

**Act 1 (`916a109`), `tools/golden/golden-hashes.json` diff:**
exactly 14 film lines added (film-s01…s12 incl. s05a/s05b/s08b);
the ab1-composer line changed only by gaining a trailing JSON comma
(value `d7bd71a0…` unmoved); no milestone hash (s28, dof-chart,
stroke-toon) touched. File stat: the JSON plus exactly 3 frame PNGs
replaced — s07, s08, s11 (the in-window mids, whose new values
`7f8e1555` / `3e8e703f` / `7bed0da5` match the aadc193 window renders
named in the commit message). The other 11 film PNGs were *not*
replaced by this commit — the fact §4 turns on.

**Act 2 (`166873c`), `golden-hashes.json` diff:** exactly one line —
`ab1-composer`: `d7bd71a02a3dfc96…` → `60c32c1af021cd0c…`. The
commit also replaces the pinned frame file itself
(`docs/ab/composer/native.png`, 3,110,294 → 2,994,196 bytes) and
updates the repin-packet docs. No other pin moved. ✓ as recorded.

## 3. Act 2 provenance chain — one citation is wrong

The commit message states the pinned frame is "a FRESH render at the
integration head (CI run 43621168728 at 9f497c7, CI lavapipe)".

Verified via `gh run view 43621168728`: the run exists — "Render
(dispatch)", event `workflow_dispatch`, **completed/success**,
created 2026-11-14T17:27:04Z. **But its head is
`dc0e555cbe207525fe45da69c47c6895222ac042` (`dc0e555`), not
`9f497c7`.** `dc0e555` is the immediate child of `9f497c7` in the
gpucore migration series (site 3 ab1.rs = `9f497c7`, site 4 main.rs =
`dc0e555`), so the substance — a fresh render at a same-evening
integration head, after the ab1 path itself was migrated — stands.
The recorded cite is nonetheless off by one commit, and a provenance
chain whose purpose is exactness should be corrected in the record
(recommended fold-in fix; this seat edits nothing).

The act's measured figures — fresh vs Chrome **23.98 dB**, fresh vs
the `c00897d` candidate 53.50 dB — are **the lead's figures from the
commit message; this seat did not re-render ab1 or re-measure them.**

Independent corroboration that the Act 2 pin describes current
output: both VR runs triaged below (§5, §6) rendered ab1-composer
**byte-identical to the pin** (MATCH, rendered sha `60c32c1af021` ==
pinned sha) at two different heads (`b6ee9fb`, `2755ff3`).

## 4. HEADLINE — the original Act 1 pinned superseded bytes; the canary caught it; the corrective is verified

The original Act 1 (at `916a109`, and still the state at `4dd6530`
when this session began) pinned, for the 11 out-of-window film mids,
the **381ff47-era staged bytes** — not the f021991 candidates the act
intended. Evidence chain, all first-hand:

- GOLDEN-RESTAGE-W03 §2 tabulates, per film mid, the staged hash
  (@381ff47) vs the f021991 candidate hash. The original pins equal
  the *staged* column for all 11 (e.g. film-s01 pin `ebcfeba1…` =
  staged; candidate was `1608ead9…`; film-s08b pin `7764821c…` =
  staged; candidate `fb4e2ba4…`). W03's procedure copies
  `docs/golden/frames-f021991/` over the staging *first*; that copy
  was missed, and `916a109`'s stat (§2) confirms only the 3 in-window
  PNGs changed.
- The post-pin VR run (§5) then rendered film-s08b as `fb4e2ba4…` —
  byte-identical to the f021991 candidate and to frames4 — against
  the `7764821c…` pin: DRIFT 24.85 dB. The renderer was right; the
  pin was wrong. W03 §2 already named the staged-vs-candidate cause
  for this frame: atmosphere inheritance (`1fc6fab`).
- The lead's corrective commit `2755ff3` discloses exactly this.
  **Verified first-hand:** its stat is 10 frame PNGs (s01–s06, s08b,
  s09, s12) + `golden-hashes.json` with exactly 20 changed film lines
  = the claimed 10 pin lines; film-s10's line and bytes are unmoved
  (identical in both eras — the control case); the 3 in-window mids
  keep their Act-1 window-render bytes. New pins equal the W03
  candidate column (§1). Pack re-verifies 18/18 (§1).

This is the one case where re-pinning was the correct remedy for a
DRIFT: the pin's provenance was broken, with the candidate bytes
independently on record (W03's table, frames4) before the act. It is
not a precedent for re-pinning over a renderer change.

## 5. VR triage — run 43624398371 (head `b6ee9fb`, pre-corrective pins)

Run state: completed/**failure**, created 2026-11-14T18:02:56Z.
Runner verdict line: `VISUAL REGRESSION: FAIL (4 pins, 2 drift)`.
Counts: 2 MATCH · 2 DRIFT · 14 NOT-RENDERED (of 18).

| pin | result (run's own numbers) | triage |
|---|---|---|
| ab1-composer | MATCH (byte-identical, `60c32c1a`) | ✓ expected post-Act-2 |
| dof-chart | MATCH (byte-identical) | ✓ clean control |
| s28-onoda-t150 | DRIFT 21.65 dB, mean \|d\| 16.19157, 79.39583 %px > 10, max d 206 | KNOWN-DEBT (W37) — §7 |
| film-s08b | DRIFT 24.85 dB, mean \|d\| 12.29211, 80.96291 %px > 10, max d 58, rendered `fb4e2ba4…` vs pin `7764821c…` | deviation from the naive "film MATCH" expectation — explained and dispositioned by §4: the pin, not the renderer |
| film-s01…s12 (other 13) + stroke-toon | NOT-RENDERED | by design — see below |

The 14 NOT-RENDERED rows are the workflow's designed state, not a
missing render set: `.github/workflows/visual-regression.yml`
renders exactly 3 frames — ab1 (whose file is supplied as the
candidate for *both* ab1-composer and s28), dof-chart, and film frame
1231 (the s08b mid, "the highest-value film canary per frame
rendered"); "The full film (1,811 frames) is NOT rendered here."
stroke-toon is a future act per the W37 register (KNOWN-DEBT).

`vr_triage.py` on this report (my run): 2 OK · 2 KNOWN-DEBT · 14
NEW-SIGNAL, exit 1. The 14 NEW-SIGNAL labels are a **register
staleness artifact**: W37's register predates the pin acts and
expects the film items in UNPINNED state, so every post-pin film row
lands in "an unregistered state". The tool itself flags the same
staleness for ab1 ("its registered debt appears already cleared").
The register needs a post-pin update (§8) — the classifications in
the table above are this seat's, from the workflow file and §4's
evidence, where they differ from the tool's raw output.

## 6. VR triage — run 43625139818 (head `2755ff3`, post-corrective)

Dispatched during this session (created 2026-11-14T18:10:53Z); this
seat waited for completion. Run state: completed/**failure**.
Runner verdict: `VISUAL REGRESSION: FAIL (4 pins, 1 drift)`.
Counts: **3 MATCH · 1 DRIFT · 14 NOT-RENDERED**.

- film-s08b: **MATCH** — rendered `fb4e2ba44ad8` == pin, byte-identical.
  The corrective (§4) is confirmed live by the canary itself.
- ab1-composer: MATCH (byte-identical). dof-chart: MATCH.
- s28-onoda-t150: DRIFT 21.65 dB — numerically identical to §5 (same
  candidate bytes `60c32c1a`, same pin): the **sole remaining drift**.
- `vr_triage.py` on this report (my run): s28 → KNOWN-DEBT;
  film-s08b → OK (with the tool's register note that s08b's
  registered debt "appears already cleared"); the 13 designed
  NOT-RENDERED film rows still print as NEW-SIGNAL, solely from the
  pre-pin register (§5).

**Steady state reached:** the VR job is now red on s28 alone,
exactly the end-state W37 predicted for the post-pin era.

## 7. The s28 pin — precise state

- **Pin:** `ff68fad3bbcc6155e8d7781325a7ad57dafa391ca4fc883a0e33dd28df60bf62`
  for item `s28-onoda-t150`, file `docs/ab/onoda-s01-t150/native.png`.
- **What it is** (NATIVE-GOLDEN.md): "§28 first-light frame: Onoda
  S01 @ t = 1.50, the first real scene reproduced natively" — the
  Wave-1 M0/§28 milestone artefact.
- **Provenance:** the historical prototype render; it predates the
  shade root-fix `c00897d` and the entire Wave-1 fix chain (workflow
  header and W37 register agree). **Untouched by both pin acts and by
  the corrective** — verified in all three diffs (§2, §4).
- **How VR measures it:** the candidate supplied is the ab1 render
  file itself ("One render, both pins" — the manifest names mode=ab1
  as s28's current-chain equivalent). Its DRIFT (21.65 dB, §5/§6)
  therefore measures current production output against the historical
  prototype, not a like-for-like re-render of a §28 lane.
- **What a disposition would change** — exactly one of:
  (a) *Re-pin* s28 to the current ab1 output (`60c32c1a…`): VR goes
  fully green, but s28 becomes a byte-duplicate of ab1-composer and
  the historical artefact leaves the gated pack. The W03 repin
  packet recommends **against** re-pinning this pin.
  (b) *Retire* the item from the registry: VR green; the frame stays
  in `docs/ab/` as history, out of the gated pack.
  (c) *Leave it*: VR stays red on this row alone — the current,
  documented steady state (§6).
  The choice is a human act under NATIVE-GOLDEN.md's pinning law;
  neither act was authorised to make it, and this seat does not make
  it.

## 8. Open items for the lead / record

1. **Provenance cite correction (§3):** Act 2's record says render
   run 43621168728 was "at 9f497c7"; GitHub records its head as
   `dc0e555`. One-line record fix.
2. **vr_triage REGISTER update:** the register still describes the
   pre-pin world (film UNPINNED, ab1 in debt, s08b undebted). Post-pin
   rows will keep printing as NEW-SIGNAL until it is reconciled —
   the tool's own register notes in both triages above ask for this.
3. **s28 disposition (§7):** human decision, unchanged by anything
   in this note. Until it lands, expect every VR dispatch to read
   `FAIL (4 pins, 1 drift)` with s28 the only DRIFT row — that is the
   verified steady state, not a new failure.
