# GPU-FIRST TALERENDER — PHASE 0 / SEAT AUDIT (independent verification)

**Status of this document: ROUND 1 — PREP + PARTIAL (items 1, 5-baseline,
6-partial). Items 2, 3, 4 and the final form of 5 are UNPROVEN and are
itemised in §7 with exactly what is needed to close them.**

**Why partial:** at audit time the phase-0 branch had not landed.
`git rev-parse gpufirst-phase0` = `bc12a5c2bbb33d93416f25eaebbe8fec8c4eb678`
(= mission-start HEAD, = `main`), `git diff bc12a5c..gpufirst-phase0`
is empty (0 lines), and `~/workspace/talerender-evidence/gpufirst-phase0/`
contained no harness outputs — only three empty directories created by
another seat (`dryrun-cpu/`, `dryrun-gpu/`, `smoke-chrome/`). There are
therefore no BATT/STATES tables, no GPU replay outputs and no CI runs
to re-measure yet. Nothing in §§2–4 below is inferred from other seats'
claims; each is marked UNPROVEN rather than filled in.

Auditor's own artefacts (not committed, per seat orders):
`~/workspace/talerender-evidence/gpufirst-phase0/audit_compare.py`,
`audit-selftest.log`, `smoke-onoda-s01.json`, `smoke-battery-prims.json`.

---

## 1. Independent checker — CONFIRMED (built, self-tested, cross-validated), with one DEFINITION DISCREPANCY flagged

### 1.1 The checker

`audit_compare.py` was written from scratch for this seat. It does not
import, call or copy the BATT seat's `tools/gpupaint-compare.py`, the
M4 `measure_battery.py`, or any `ab_metrics.py`. Given two PNGs
(CPU first, GPU second) it outputs: identical Y/N, differing-px count
and %, max |Δ| per channel (R,G,B,A), mean |Δ| per channel, PSNR, and
the EDGE/FRINGE/INTERIOR split, plus per-split max Δ (labelled extra).

- **Decode disclosure (as required by the brief):** the default path
  decodes PNG via Pillow 10.2.0 (libpng), decode only. That reuse is
  acceptable because PNG is lossless — any conforming decoder yields
  byte-identical pixels, so the decoder cannot hide or manufacture a
  parity difference; every measurement (differing test, PSNR, boundary
  mask, Chebyshev distance, split) is original code in the file.
- **Independence guard:** the file also contains a pure-stdlib PNG
  decoder (zlib + hand-written unfiltering, no Pillow/numpy).
  `--crosscheck-decode` verified stdlib == Pillow byte-for-byte on
  real cards: `docs/ab/onoda-s01-t150/original.png` (1080×1920) and
  `docs/ab/stroke-battery/native-face-angry.png` (2560×2816), plus
  both synthetic self-test files. `--stdlib-decode` runs the full
  comparison through the stdlib path (verified end-to-end on
  `docs/ab/scene-format/ab1/` original-vs-native).

### 1.2 Known-answer self-test — ALL PASS (`audit-selftest.log`)

- T1 identical pair → identical, differing 0, PSNR inf.
- T2 one pixel changed by +10 in R on an all-opaque canvas (no
  fractional alpha anywhere → no boundary exists) → differing 1,
  max Δ = [10,0,0,0], and the pixel classifies INTERIOR (distance to a
  non-existent boundary is infinite, not 0 — a sentinel case the first
  draft got wrong and the self-test caught before any real data ran).
- T3 fractional-alpha column (α=128 at x=5) with diffs at x=5, x=7,
  x=9 → split EDGE 2 / FRINGE 0 / INTERIOR 1, matching the hand
  computation under the brief's literal definition (see §1.4).
- T4 stdlib decoder == Pillow on both synthetic files.
- Every run asserts EDGE+FRINGE+INTERIOR == differing_px.

### 1.3 External cross-validation of the PSNR math

On the pre-existing §28 pair (`docs/ab/onoda-s01-t150/original.png` vs
`native.png`, 1080×1920) the checker reports **RGB PSNR 20.1521 dB** —
the stroke-battery README records the §28 baseline as **20.152 dB**.
Agreement to the recorded precision, from independent code, on data
neither seat produced for this mission. (Full result:
`smoke-onoda-s01.json`. That pair is an opaque composite — α Δ = 0
everywhere, boundary_px = 0, all 2,067,214 differing px INTERIOR —
which is the correct, if degenerate, application of the definition;
see §1.5.)

### 1.4 ⚠ DEFINITION DISCREPANCY — the brief's boundary vs BATT's boundary

The brief given to this seat defines the split as: *EDGE = differing
px within Chebyshev distance 1 of a CPU coverage boundary — CPU alpha
in (0,255) **or any 4-neighbour alpha in (0,255)***. Read literally,
the boundary set is B′ = {fractional-α px} ∪ {their 4-neighbours},
and EDGE = Chebyshev-≤1 of B′.

The BATT seat's `tools/gpupaint-compare.py` (read, not reused)
implements B = {fractional-α px} only, EDGE = Chebyshev-≤1 of B, and
its docstring calls that "per spec".

These are **not the same set**. Under the brief-literal reading, a
differing px at Chebyshev distance 2 from a fractional px in an axial
position (e.g. offset (2,0) or (2,1)) is EDGE; under BATT's reading it
is FRINGE. Diagonal (2,2) offsets agree (INTERIOR/FRINGE boundary
unaffected there — FRINGE under both). So EDGE/FRINGE cell counts
**will differ between the two readings on any card with real diffs**,
by exactly the count of differing px at Chebyshev distance 2 in
axial/knight positions relative to a fractional-α px.

This is a definition conflict, not (on present evidence) an error or
a dishonesty instance — the two texts genuinely say different things.
**Coordinator ruling needed on which reading is canonical.** To make
the follow-up re-measurement possible under either ruling, the
checker reports **both** splits in every result
(`split` = brief-literal; `split_batt_reading_boundary_is_fractional_only`
= BATT's reading).

### 1.5 Second definitional divergence — PSNR

The brief to this seat says "RGBA PSNR". BATT's tool reports
**alpha-only PSNR** (`10*log10(255²/MSE_alpha)`), stated openly in its
docstring. The M4 battery's headline (47.9 dB) was also alpha PSNR.
The checker reports all three — RGBA (brief), RGB, and alpha (BATT) —
so any seat's cell can be compared like-for-like, but a headline
"PSNR" quoted without its channel set is not comparable across seats.
Follow-up comparisons in §2 will name the channel set for every cell.

### 1.6 Split degeneracy warning for the follow-up round

On cards whose CPU side is an opaque composite (α ≡ 255, e.g. the
legacy Chrome-exported cards and the M4 battery originals: α Δ = 0 and
boundary_px = 0 on the prims card, `smoke-battery-prims.json`), the
split is INTERIOR-only by construction. The split only discriminates
on cards carrying true painter α. Whether the Phase-0 CPU cards do is
a follow-up check (§7), and any seat table that reports a split on
opaque cards without noting the degeneracy should be flagged in §6.

---

## 2. Re-measurement vs BATT/STATES tables — UNPROVEN

No battery-card pairs, state-spread outputs, or BATT/STATES result
tables existed in the evidence dir (or the repo) at audit time.
0 cells compared; 0 discrepancies found and 0 agreements claimed.
What is ready: the checker (§1) runs on 2560×2816 cards in ~5 s and
is definition-bilingual (§1.4/§1.5), so the cell-by-cell pass is a
pure execution step once outputs land.

## 3. Worst-case check — UNPROVEN

No tables exist to scan. No named worst cases exist yet to confirm
or refute.

## 4. Determinism (independent) — UNPROVEN

No GPU replay harness output exists yet; no CI replay was dispatched
to this seat. Neither the two-run hash comparison nor the harness
self-check confirmation could be performed. (This seat cannot run
the GPU replay locally with any meaning: this VM has no GPU —
`/dev/dri` absent — so local replay would be lavapipe and must not
be reported as RADV evidence.)

## 5. Production invariance — BASELINE RECORDED; final verdict UNPROVEN

At audit time the branch is identical to `bc12a5c`, so the diff audit
is vacuously clean and proves nothing about the final head. Baselines
recorded for the follow-up diff:

- `Cargo.toml` at `bc12a5c`: 14 lines; `[dependencies]` = `wgpu
  30.0.2`, `pollster 1.0.1`, `image 0.25.10`; **no `[features]`
  section exists** — so any features section at the branch head is
  new, and any change to the three dependency lines is a violation
  of the "features only" rule.
- Tracked PNGs at `bc12a5c`: **390** (`git ls-files | grep -c '\.png$'`),
  all outside `docs/ab/` path filtering used in the brief — the
  follow-up must diff the full list, not a count, and classify any
  new PNG against the pre-existing fixture/ab packages (the bulk sit
  under `tools/film/fixtures/`, `proof/`, `film/assets/`).
- Working tree at audit time (uncommitted seat work, not branch
  commits): `docs/integration/GPUFIRST-INVB-INVENTORY.md` and
  `tools/gpupaint-compare.py` untracked; pre-existing `__pycache__`
  noise. The follow-up audit must re-run the diff against the
  **final committed head the coordinator names**, including a check
  that these files' committed forms match what was reviewed.
- CI corroboration: none available — no branch-head CI result or
  census/determinism report was supplied to this seat. What that
  leaves unproven: everything in this item beyond the baseline.

## 6. Honesty sweep — PARTIAL (only two seat artefacts exist)

Read in full: `docs/integration/GPUFIRST-INVB-INVENTORY.md` (INV-B)
and the header/logic of `tools/gpupaint-compare.py` (BATT's tool).
No `GPUFIRST-*.md` records from BATT/STATES/GPUX/TRACE seats exist yet.

- **INV-B inventory: no instances found.** Counts are reconciled
  mechanically against grep with substring collisions separated
  (e.g. `fill_polygon` 31 = 27 pure + 3 masked + 1 definition); line
  numbers are cited throughout; scope limits are stated ("structural
  inventory only"); closure/indirect-dispatch caveats are named
  rather than smoothed over. It makes no parity verdicts, no RADV
  claims and no performance claims, so there is nothing in it
  stronger than its evidence.
- **BATT compare tool: no instances found in the tool itself.** Its
  docstring states its PSNR is alpha-only, states that ink inside an
  opaque fill classifies INTERIOR "by construction — that is the
  definition, not a smoothing of the result", states its decode stack
  (Pillow + numpy, versions), and disclaims verdicts ("No tolerances,
  no verdicts, no smoothing: raw parity numbers only"). One item is
  flagged in §1.4: the docstring presents its boundary set as "per
  spec" while the brief text supplied to this seat defines the
  boundary differently — a definition conflict for the coordinator,
  recorded here so it is not silently resolved by whoever writes the
  headline table.
- Aggregate-only reporting, unlabelled estimates, RADV-from-lavapipe:
  **cannot be assessed** — the records that would contain them do not
  exist yet. This sweep must be re-run in full in the follow-up round.

## 7. Exactly what remains for the follow-up round

1. Coordinator names the final branch head (commit hash) and the CI
   run(s) at that head, including whether a census/determinism run
   exists there.
2. BATT + STATES outputs land in
   `~/workspace/talerender-evidence/gpufirst-phase0/` (card PNG pairs
   + their results tables, machine-readable form preferred).
3. Then, with no further prep needed: (a) run `audit_compare.py` on
   every pair, compare cell-by-cell under both boundary readings and
   all three PSNR channel sets, report every discrepancy with both
   numbers; (b) scan full tables for unnamed worse cases; (c) two
   CI GPU replays (one heavy state + one battery card) with hashes
   compared by this seat + harness self-check confirmation;
   (d) full `git diff bc12a5c..<head>` gating audit + Cargo.toml +
   `git ls-files` PNG diff against §5 baselines + CI corroboration;
   (e) full honesty sweep over all `GPUFIRST-*.md` records.
4. Coordinator ruling on §1.4 (canonical boundary reading) — needed
   before headline EDGE/FRINGE numbers from different seats can be
   placed in one table.
