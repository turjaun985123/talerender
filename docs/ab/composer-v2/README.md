# Composer A/B — v2: the ab1 full chain after the shade root-fix

**Seat W57 — re-baseline measurement pack.** This package measures the
SAME staging as the M5 pack (`docs/ab/composer/`: Onoda S01 t = 1.50
through the full production chain — Render → Bokeh → Bloom → Output →
GRADE) rendered by a binary that includes the shade root-fix
`c00897d`. It is the measurement half of the re-baseline that commit
declared. **Nothing in the golden pack was re-pinned by this seat** —
see the recommendation at the end.

## Provenance of the native frame

- Source: GitHub Actions **Render (dispatch)** run **43569670339**,
  dispatched against `main` at headSha
  **`c00897d32bf86bd9a081b7152eaa2bd0fe3f1e2b`** — the shade-fix
  commit itself (the CI *push* run for `c00897d` was cancelled by the
  concurrency rule when `e5860b4` landed; this dispatch run is the
  post-fix render of record for the commit).
- Artifact: `render-ab1-0-0-43569670339` (mode `ab1`, created
  2026-11-14T07:10:03Z), containing a single `ab1.png` (1080×1920).
- Renderer: talerender (wgpu, **CI lavapipe**) — correctness only, no
  performance claim. Reference side is unchanged: the production
  engine (three.js r170) in headless Chrome,
  `docs/ab/composer/original.png`.
- Staged frame: `native.png` in this directory, sha256
  `a415ead80a1a17d2b190c03c237bd2f8129057a8688ebbfa31cd5d26ffceeee9`
  (byte-identical to the artifact's `ab1.png`).

## The measurement

Tool: `tools/ab/measure_ab.py --pair` (house metric definitions,
identical to the M5/M6 packages). Control first: re-measuring the
committed M5 pair (`original.png` vs `docs/ab/composer/native.png`)
reproduces the pinned numbers **exactly** — 23.850 dB /
5.20056 / 13.53318 — so the tool run behind this pack is verified
against the pin before the new frame is measured.

| metric (vs the Chrome reference) | pre-fix pin (M5) | post-fix (`c00897d`) | Δ |
|---|---|---|---|
| PSNR | 23.850 dB | **23.96 dB** | +0.11 dB |
| mean \|Δ\| per channel | 5.20056 | **4.82505** | −0.376 |
| %px max-channel Δ > 10 | 13.53318 | **11.44483** | −2.09 pts |
| %px any-channel Δ > 0 | 97.02947 | **96.26264** | −0.77 pts |
| max \|Δ\| | 255 | 255 | — |

`metrics.json` in this directory is the tool's verbatim report for
the post-fix pair (its `native.path` names the /tmp download path;
the staged `native.png` beside it is that file, hash-verified above).
`pair_side-by-side.png` / `pair_diff.png` are the tool's pair images.

**How far the fix itself moved the frame** (pre-fix native vs
post-fix native, `metrics-native-vs-native.json`): PSNR 41.351 dB,
mean |Δ| 0.81309, %px > 10 = 2.08131, %px > 0 = 36.96291,
max |Δ| 30. About 37% of pixels changed, almost all by small
amounts — the signature of a shade-arithmetic correction spread over
the grass, hills and felt surfaces, not a staging change.

## What moved and why

`c00897d` replaced `shade_rgb`'s sRGB **byte multiply** (25–40 LSB
too dark on derived shades) with `shade_lin`: THREE.Color's
linear-space multiply — hex → linear, multiply, → sRGB — the
arithmetic the source engine itself performs, routed through every
texture painter. Its corpus pins reproduce exactly under the new
math (`#3A2E22` ×1.25 = `#413427`; `#3A2A2A` ×0.7 = `#302222`).
The ab1 frame's dominant surfaces are shade-derived end to end
(`paint_grass` blades, per-face hill shades, tufts), so the frame
moves — and moves **toward** the Chrome reference on every metric,
which is the expected direction: the native shades now agree with
the engine's own derived shades instead of sitting systematically
dark of them.

The JPEG floor caveat from the tool applies unchanged: the
reference carries one q0.93 JPEG generation, so these numbers
understate native accuracy one-directionally (a pixel-perfect
native frame could not exceed ≈42 dB against this reference).
+0.11 dB is a small, correctly-signed move on a metric whose
remaining distance is dominated by other, already-named residuals
(grain hash vendor sensitivity, PMREM/trunk deltas from M5) — it is
evidence the fix landed as intended, not a new fidelity claim.

## Golden consequence — RECOMMENDATION (not performed here)

`tools/golden/RUNNER.md` predicted, before this render existed:
*"`ab1-composer` — EXPECTED STALE on re-render (certain)… A
current-binary `mode=ab1` render cannot reproduce pin `d7bd71a0…`."*
Confirmed by measurement: the golden pin for `ab1-composer`
(`tools/golden/golden-hashes.json`) is
`d7bd71a02a3dfc96232957123bfb82766bf7a4e96676f96e72db4e6c7adaf4f3`
— the sha256 of the pre-fix `docs/ab/composer/native.png`. The
post-fix frame hashes to `a415ead8…` ≠ the pin, so any future
golden verify against a regenerated ab1 reference will report
`ab1-composer` **STALE**. That STALE is the *deliberate-change*
class in RUNNER.md's re-baseline protocol, with its cause measured
and disclosed in this pack — not drift.

**Recommendation:** re-pin the `ab1-composer` golden from a
post-fix render, using this pack's `native.png` (sha256
`a415ead8…`, CI lavapipe, headSha `c00897d`) as the candidate
frame, following RUNNER.md §"re-baseline protocol" (measure →
stage → name the cause → pin → scope stays visible). The pin
itself (`make_golden.py --pin`) is a human act and was **not**
performed by this seat; the committed M5 package in
`docs/ab/composer/` is likewise left untouched as the pre-fix
record.

## Reproduce

```sh
gh run download 43569670339 --repo turjaun985123/talerender \
  -n render-ab1-0-0-43569670339 -D /tmp/ab1-postfix   # artifact expires 2026-11-21
python3 tools/ab/measure_ab.py \
  --pair docs/ab/composer/original.png /tmp/ab1-postfix/ab1.png \
  --out /tmp/measure-postfix
python3 tools/ab/measure_ab.py \
  --pair docs/ab/composer/native.png /tmp/ab1-postfix/ab1.png \
  --out /tmp/measure-nvn
```
