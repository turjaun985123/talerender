# tools/ab — A/B measurement for film frames

`measure_ab.py` measures native TaleRender frames against the Chrome
reference frames of the same staged instants, using the house metric
set (the same definitions as `docs/ab/scene-format/scripts/ab_metrics.py`
and every M2–M6 evidence package):

| metric | definition |
|---|---|
| PSNR (dB) | `10·log10(255² / MSE)`, MSE over all channels, float64 |
| mean \|Δ\| | mean absolute per-channel delta |
| %px > 10 | % of pixels whose **max-channel** \|Δ\| > 10 |
| %px > 0, max Δ | recorded alongside, for the full picture |

It complements the lead seat's `film/ab_compare.py` (a fixed-schedule,
PSNR-only proof for this one film, with the contact-sheet step). This
tool is the reusable version: manifest-driven, full metric set, usable
for any project that has a `film.json` and a review-frame directory.

## Usage

Film mode — the normal run, once native frames exist:

```
python3 tools/ab/measure_ab.py \
  --native-dir <frames-dir> \
  --review-dir <project>/review/frames \
  --film scenes/film/film.json \
  --out docs/ab/<package-dir>
```

- **Review frames** are discovered as `t<seconds>.jpg` (the project
  writes `t0.50.jpg` … `t59.50.jpg`, one per second). For every review
  timestamp inside a segment of `film.json`, the native frame staged
  at the same time is paired with it (`n = round(t·fps)`; frame `n`
  is staged at `t = n/fps`). Every segment gets ≥ 1 pair on the
  half-second grid (shortest segment, s08b at 1.372 s, gets 2; s05b
  gets exactly 1).
- **Native frames** may be named by frame number — `frame_00042.png`
  (what the film driver writes), `frame-00042.png`, `f00042.png`,
  `00042.png` — or by time — `t1.50.png`, `1.50.png`.
- **Outputs** in `--out`: `ab-report.json` + `ab-report.md` (per-shot
  summary, per-pair table, explicit missing/no-data list) and, for
  the three lowest-PSNR pairs, `worst-<k>_<seg>_t<T>_side-by-side.png`
  (left = reference/Chrome, right = native/talerender) and
  `..._diff.png` (|Δ| amplified ×4). Every image carries a label band
  naming the renderer on each side.
- **Resolution:** pairs must match. The expected case is native
  1080×1920 vs review 1080×1920. If a native frame arrives at
  another size, it is resized to the reference size (LANCZOS) and
  the fact is recorded per-pair in the report — never silently.
- **Labels:** `--native-label` / `--reference-label` override the
  defaults. Use them: native frames rendered on lavapipe (this VM /
  CI) and on RADV (the user's PC) are both "native", but the GRADE
  grain hash is GPU-vendor sensitive (M5 gaps.md), so the report
  must say which adapter produced the frames.
- **Exit codes:** 0 = at least one pair measured; 2 = no pairs
  (or no review frames) — the report is still written, with every
  segment marked NO DATA and every missing frame listed.
- **Memory:** the tool holds one pair at a time (~100 MB transient)
  and reloads only the worst three pairs to build images, so a full
  60-pair film run fits comfortably in a small VM.

Pair mode — measure one committed pair (also how the tool is proven):

```
python3 tools/ab/measure_ab.py --pair REFERENCE.png NATIVE.png --out DIR
```

## How to read the numbers

**The JPEG floor comes first.** The review references are JPEGs —
one q0.93 generation from the Chrome canvas (Chromium
`toDataURL('image/jpeg', 0.93)`). The M6 decision brief
(`docs/ab/telemetry/jpeg-decision.md`) measured what that generation
costs: on flat/step content two otherwise-identical images agree no
better than **50.78–57.71 dB**; on a detailed frame, one generation
alone measures **42.31 dB** against its own clean source (mean |Δ|
1.225, 94.47 % of pixels changed, max Δ 48). Consequences:

- A *pixel-perfect* native frame cannot score above ≈ 42 dB
  (detailed content) or ≈ 51–58 dB (flat content) against these
  references. A reading in the high 30s/low 40s on detailed frames
  is near the reference's own noise floor, not a mediocre result.
- The bias is **one-directional**: JPEG error in the reference is
  counted as native error. These numbers can only *understate*
  native accuracy, never overstate it. Every report the tool writes
  states this in its header.

**Calibration context** (same metric set, this project's history):

- The headless-gl port — a lossy WebGL1 re-implementation — scored
  **15.8–19.3 dB** on its film A/B. That band is what "visibly a
  different renderer" looks like in these units.
- TaleRender on the Onoda S01 frame (t = 1.50, detailed foliage
  content): **20.152 dB** at §28 (flat-colour prototype) →
  **20.218 dB** after the M4 painter → **23.850 dB** with the full
  M5 composer chain. Progress is read as movement within a content
  class, against the floor for that class.
- The dof-chart — flat synthetic content — reached 47.38 dB
  full-frame against its JPEG reference, with flat zones at the
  50.78–57.71 dB floor. Do not compare numbers across content
  classes: a detailed jungle frame and a flat test chart live in
  different bands.

**Which metric to look at:** PSNR is the headline fidelity number
and the one the milestones gate on. %px > 10 is the more legible
"how much of the frame visibly differs" measure (M5 full chain:
13.5 % on S01). mean |Δ| is dominated by large near-matching areas
and moves last; a falling mean |Δ| with a flat PSNR usually means
broad low-amplitude drift (light/grade), while a falling %px > 10
means whole regions snapping into agreement.

**What the numbers cannot claim:**

- Nothing about **staging correctness** — a perfectly measured pair
  can still be a faithfully-rendered wrong staging. That is the M6
  telemetry gate's job (`cutouts()` agreement), not this tool's.
- Nothing about **motion** — pairs are single staged instants.
- Nothing about **performance** — no timing is measured, and frames
  rendered on lavapipe carry no performance meaning at all.
- Nothing across **content classes** or across **references with
  different JPEG generations** (see the floor above).

## Worked example — the tool proving itself (2026-11-14)

Pair mode run against three already-committed evidence packages.
In each case the tool's output reproduces the package's committed
`metrics.json` — the strongest available check that the metric
implementation is the house one:

**1. M5 composer package** (`docs/ab/composer/`, full chain vs the
Chrome original, S01 t = 1.50):

```
$ python3 tools/ab/measure_ab.py --pair docs/ab/composer/original.png \
      docs/ab/composer/native.png --out /tmp/ab-proof/composer
{ "psnr_db": 23.85, "mean_abs_delta_per_channel": 5.20056,
  "pct_pixels_max_channel_delta_gt_10": 13.53318,
  "pct_pixels_any_channel_delta_gt_0": 97.02947, "max_abs_delta": 255 }
```
Committed package values: PSNR 23.850 dB, mean |Δ| 5.201, %px > 10
13.533 — agreement to every committed digit.

**2. M2/M6 scene-format ab1 pair** (`docs/ab/scene-format/ab1/`):

```
{ "psnr_db": 21.82, "mean_abs_delta_per_channel": 15.96009,
  "pct_pixels_max_channel_delta_gt_10": 78.7447,
  "pct_pixels_any_channel_delta_gt_0": 98.45809, "max_abs_delta": 157 }
```
Digit-for-digit identical to the committed `metrics.json`, including
max Δ 157. (This pair is the hand-authored composer-path render vs
the v1 scene-file render — the 21.82 dB measures the §12 light-rig
format gap, per that package's MAINTENANCE.md, not a renderer
defect.)

**3. Scene-format dof-chart pair** (`docs/ab/scene-format/dof-chart/`,
near-identical renders — the sensitive end of the range):

```
{ "psnr_db": 84.486, "mean_abs_delta_per_channel": 0.00023,
  "pct_pixels_max_channel_delta_gt_10": 0.0,
  "pct_pixels_any_channel_delta_gt_0": 0.02315, "max_abs_delta": 1 }
```
Again identical to the committed `metrics.json` on every field.

**4. Film-mode plumbing self-check.** The 60 review JPEGs were
decoded and re-saved losslessly as `frame_{n:05}.png` "native"
frames, then measured against the review directory itself. This is
*not* an A/B result — it proves the sampling machinery:

```
pairs measured: 60; missing: 0; overall PSNR min inf / mean inf
  s01: 6   s02: 7   s03: 3   s04: 5   s05a: 4   s05b: 1   s06: 4
  s07: 4   s08: 6   s08b: 2  s09: 3   s10: 6    s11: 5    s12: 4
```
All 60 pairs measure PSNR ∞ / mean |Δ| 0 (identical pixels, as they
must), and the per-segment counts confirm the pairing follows
`film.json` boundaries exactly (s05b = 1 pair at t = 25.50, s08b = 2).
The same run against an **empty** native directory writes the full
report with all 14 segments marked NO DATA, lists all 60 missing
frames, and exits 2.
