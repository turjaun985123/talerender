# Visual regression CI — W16 (Phase 5)

**Deliverables (this seat, new files only):**

- `.github/workflows/visual-regression.yml` — dispatch-only job.
- `tools/ci/visual_regression.py` — the per-pin drift runner.
- This note.

**Status: written and locally validated; NOT yet run in CI.**
A workflow is proven only by a real dispatch — none has happened.
Nothing below claims a CI verdict; §4 records exactly what was
exercised locally and how, and §5 states what the first dispatch
is *expected* to show and why.

## 1. What was missing, and what this adds

The golden pack (`tools/golden/make_golden.py`) verifies that the
reference files *on disk* still hash to their pins. It cannot see
the renderer: a code change that moves every pixel leaves the
pack green until a human happens to re-render a reference by hand
and notice the STALE. This job closes that half — it renders the
pinned references fresh and measures the fresh frames against the
pins, per pin, with a verdict.

## 2. The law the gate obeys

- **Same-renderer drift is gated; cross-renderer equality never
  is.** Native-vs-Chrome A/B and lavapipe-vs-RADV comparisons stay
  *reported, never gated* (the Chrome side sits behind its q0.93
  JPEG generation; the GRADE grain's fp32 `sin()` is vendor
  territory — determinism contract §2, NATIVE-GOLDEN.md). The
  workflow therefore runs **only** on CI lavapipe, against the CI
  lavapipe pins, in an environment copied line-for-line from
  `render.yml`/`determinism.yml` (ubuntu-22.04, Mesa 23.2 lavapipe,
  `VK_ICD_FILENAMES` pinning, mold, rust-cache). RADV frames must
  never be fed to this runner; RADV keeps its own future pins.
- **The post-`cabcc30` same-renderer promise is byte-identity**
  (determinism contract §1), so drift past a tight band on the
  *same* renderer is a legitimate regression signal — the only
  kind this job raises.
- **Verdict discipline mirrors `determinism.yml`:** the runner
  prints `VISUAL REGRESSION: PASS (n pins, 0 drift)` (exit 0) or
  `VISUAL REGRESSION: FAIL (n pins, m drift)` (exit 1); input
  error is exit 2 with an ERROR line and **no verdict line**. The
  workflow's verdict step requires exit 0 *and* the PASS line.
  The markdown table is uploaded as an artifact either way.

## 3. The runner

`tools/ci/visual_regression.py --render ID=PATH … [--band 80]
[--out report.md]`. Item registry and reference paths are
**imported from `tools/golden/make_golden.py`** (ITEMS), and the
metrics are **imported from `tools/ab/measure_ab.py`**
(`pair_metrics()` — the house set: PSNR = 10·log10(255²/MSE) over
all channels, float64 RGB, plus mean |Δ|, %px > 10, max Δ), so
neither the registry nor the math can drift from the house tools.
One deliberate difference from the A/B tool: a size mismatch is
DRIFT, never a silent LANCZOS resize — in a regression check the
changed size *is* the drift.

Per-item classification:

| status | meaning | fails? |
|---|---|---|
| `MATCH` | rendered sha256 == pin (byte-identical) | no |
| `WITHIN-BAND` | bytes differ, PSNR ≥ band (incl. pixel-identical re-encodes, PSNR inf) | no — but wants an explanation; byte-identity is the expectation |
| `DRIFT` | PSNR < band, or size mismatch | **yes** |
| `MISSING-RENDER` | rendered path supplied, file absent | **yes** |
| `REFERENCE-MISSING` | pinned item's reference file gone from the repo | **yes** |
| `UNPINNED` | in the manifest, no pin in `golden-hashes.json` | no — reported only |
| `NOT-RENDERED` | pinned, not in this run's render set | no — reported only |

In the verdict, *n* = pinned items actually measured; *m* =
DRIFT + MISSING-RENDER + REFERENCE-MISSING.

**The band: 80 dB, from the record.** (i) After `cabcc30`
(exact f32 paint-cache keys; E0600 follow-up `f021991`), same-
renderer re-renders are byte-identical — the lead's CI lavapipe
probes at the final head: frames 1195–1204 10/10, 795–814 20/20,
195–204 10/10 (determinism contract §5, E4). (ii) The one same-
renderer defect on record — the pre-fix cache-key collision —
measured **72.34 dB** (E2); a band at or below that would wave
the exact defect class through. (iii) 80 dB in absolute terms is
MSE ≈ 6.5·10⁻⁴, ~0.065 % of channel samples wrong by a full LSB;
a single wrong pixel in a 1080×1920 frame sits at ~116 dB. The
band is > 7 dB above the worst defect on record and > 35 dB
below single-pixel noise: tight, but not a byte-compare in
disguise. `--band` exists; changing it is a policy act for a
commit message, not a run-time knob to reach for.

**Current golden state, handled honestly:** `golden-hashes.json`
pins exactly 4 items (`s28-onoda-t150`, `dof-chart`,
`stroke-toon`, `ab1-composer`). The 14 film mids are staged
candidates (`docs/golden/frames/`, `frames-f021991/`) pending
human first-pins. The runner reports them `UNPINNED` — no PSNR
is computed against a candidate, because a staging is not a
reference — and they never fail the run. The day a human pins
them, the workflow gates them with no edit.

## 4. The workflow — 3 frames, chosen by cost

| render | command (after the shared build) | pins checked |
|---|---|---|
| ab1 still | `--scene ab1` | **two**: `ab1-composer`, and `s28-onoda-t150` whose manifest entry names mode=ab1 as its current-chain equivalent |
| dof-chart still | `--scene dof-chart` | `dof-chart` |
| film chunk [1231, 1232) | `--film film --from 1231 --to 1232` | `film-s08b` (today: UNPINNED report) |

The film frame is segment s08b's mid (frame 1231; s08b is the
shortest segment, 41 frames / 1.37 s), immediately past the
historic 1195–1199 paint-cache-boundary defect zone — the
highest-value film canary per frame rendered. The **full film is
deliberately not rendered**: 1,811 frames at CI render rates is a
production dispatch (`render.yml` mode=film), not a routine
regression job; three frames keep this in `determinism.yml`'s
cost class. `stroke-toon` is not rendered: the battery writes a
card *set* into a directory (a different output contract from
the still modes) and carries the same re-pin debt as ab1; it can
be added when the founding pins are re-baselined.

**Expected first dispatch — FAIL, and that is the debt, not a
bug in this job.** The `ab1-composer` and `s28-onoda-t150` pins
predate the shade root-fix `c00897d`; the golden pack's own
analysis (`tools/golden/RUNNER.md`) records both as EXPECTED
STALE on re-render — "certain" for ab1-composer. Until a human
re-pins them under the re-baseline protocol, a correct run of
this job fails on those two pins. `dof-chart` carries no such
expectation (its chart texture never passes through `shade_rgb`)
and is the job's clean control: a DRIFT there is real drift
until explained. Nobody widens the band or whitelists a pin to
make this job green — the workflow's own error annotation says
so at the point of failure.

## 5. Local validation (what was actually exercised)

The workflow YAML parses (`yaml.safe_load`: dispatch-only
trigger, one job, `runs-on: ubuntu-22.04`). The runner was
exercised against synthetic pairs fabricated in `/tmp/vr-w16`
(320×240 gradient/block images; a fabricated pins file via
`--pins-file` + reference overrides via `--reference`, so the
real golden state was never touched), plus one identity run
against the real repo state. Outputs, trimmed to the rows under
test:

**Run 1 — byte-identical renders (`cp` copies) → MATCH, PASS, exit 0**

```
| dof-chart    | MATCH | inf | … | 20f8d94f37c1 | 20f8d94f37c1 | sha256 == pin (byte-identical) |
| ab1-composer | MATCH | inf | … | 7189bb1d23c9 | 7189bb1d23c9 | sha256 == pin (byte-identical) |
| film-s01     | UNPINNED | — | … | - | - | no pin in golden-hashes.json — staged candidates are not references … |
Counts: 2 MATCH · 16 UNPINNED (of 18 golden items).
VISUAL REGRESSION: PASS (2 pins, 0 drift)   EXIT=0
```

(16 UNPINNED here because the fabricated pins file pins only 2
items; against the real pins file the film count is 14 — Run 5.)

**Run 2 — 50 channel samples off by 1 LSB → WITHIN-BAND, PASS, exit 0**

```
| dof-chart | WITHIN-BAND | 84.77 | 0.00022 | 0.00000 | 1 | … | bytes differ, PSNR >= band 80 dB … |
VISUAL REGRESSION: PASS (2 pins, 0 drift)   EXIT=0
```

(84.77 dB observed; the closed-form prediction for 50 unit
errors in 320×240×3 samples is 84.8 dB.)

**Run 3 — Gaussian noise σ=10 → DRIFT, FAIL, exit 1**

```
| dof-chart | DRIFT | 28.13 | 7.97170 | 64.69792 | 49 | … | PSNR < band 80 dB — same-renderer drift |
VISUAL REGRESSION: FAIL (2 pins, 1 drift)   EXIT=1
```

(28.13 dB observed; σ=10 noise predicts ≈28.1 dB.)

**Run 4 — rendered file absent → MISSING-RENDER, FAIL, exit 1**

```
| dof-chart | MISSING-RENDER | — | … | rendered file not found: /tmp/vr-w16/absent.png |
VISUAL REGRESSION: FAIL (1 pins, 1 drift)   EXIT=1
```

**Run 5 — real repo state, each reference fed as its own render
(identity sanity only — NOT render evidence) → MATCH, exit 0**

```
| dof-chart    | MATCH | inf | … | 957d5573497e | 957d5573497e | sha256 == pin |
| ab1-composer | MATCH | inf | … | d7bd71a02a3d | d7bd71a02a3d | sha256 == pin |
Counts: 2 MATCH · 14 UNPINNED · 2 NOT-RENDERED (of 18 golden items).
VISUAL REGRESSION: PASS (2 pins, 0 drift)   EXIT=0
```

**Run 6 — refusal discipline:** unknown item id and
no-`--render` invocations each print one ERROR line on stderr,
print no verdict, and exit 2.

**Incidental finding, kept because it proves a classification:**
an early fabrication round re-saved the "identical" pair through
PIL instead of copying bytes — the runner classified it
`WITHIN-BAND` with PSNR `inf` ("pixels identical, file bytes
differ (re-encode/metadata — not pixel drift)"). That is the
designed behaviour: MATCH is a *byte* claim, pixel-identity
without byte-identity is reported, not silently promoted.

## 6. What is NOT proven

- **CI execution.** No dispatch has run. The render commands are
  copied from `render.yml`'s proven modes and the verdict step
  from `determinism.yml`'s pattern, but the job as a whole is
  unproven until its first dispatch — at which point the
  expected outcome is §4's documented FAIL on the two debt pins,
  with `dof-chart` as the control that distinguishes "the job
  works" from "the job is broken".
- **RADV.** Out of scope by law (§2). A RADV visual-regression
  job becomes possible only after RADV pins exist.
- **The film pins.** They gate nothing until a human strikes
  the first pins; this job merely keeps the exercise warm and
  the UNPINNED state visible in every report.

*Seat W16 — files written, nothing committed, no other file
touched (`src/` and the existing workflows untouched).*
