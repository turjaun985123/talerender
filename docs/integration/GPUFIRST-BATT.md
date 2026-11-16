# GPUFIRST-BATT — GPU-vs-CPU stroke-battery parity (Phase 0, SEAT BATT)

**Status at delivery: TOOLING COMPLETE + VALIDATED; GPU MEASUREMENT PENDING
THE GPU REPLAY.** Repo HEAD is `bc12a5c2bbb33d93416f25eaebbe8fec8c4eb678`
(verified by this seat). During this seat's run the TRACE seat's harness
landed in the working tree (uncommitted, coordinator to integrate):
`src/gpupaint.rs` provides `--gpu-paint record-battery <outdir>`
(feature `gpupaint`, default OFF) — record side WORKING, pure CPU,
writes the CPU card PNGs plus one display-list JSON per card. The GPUX
`replay <list.json> <out.png>` subcommand is present but is a **stub
that fails loudly (exit 3, "GPU executor not yet integrated")** — so no
GPU cards exist yet and no GPU parity measurement is possible yet.
This document therefore delivers (1) the comparison tool, (2) its
CPU-vs-CPU dry-run validation table (must be — and is — 100%
identical), and (3) the exact commands the coordinator runs to produce
the real GPU table the moment `replay` is integrated and run on CI.
No GPU parity numbers are claimed anywhere in this document; none
exist yet.

**Reference law.** The CPU Painter's cards are the reference. The
Chrome/Skia originals and the M4 tolerances (`docs/ab/stroke-battery/`)
are a different question and are NOT used here. No tolerances, no
verdicts, no smoothing: the tool reports raw parity numbers only.

## Tool

`tools/gpupaint-compare.py` (standalone; identical copy in the evidence
dir). Works on any two card directories with the battery layout
(`prims.png`, `toon.png`, `face-<mood>.png` — the names `src/battery.rs`
and `measure_battery.py` already use), plus a single-card mode
(`--cpu-card/--gpu-card --card <stem>`).

**PNG decode choice: Python + Pillow (PIL 10.2.0) + numpy 1.26.4** —
both present in this environment (checked), so the Rust `image`-crate
shim the brief allowed for was NOT needed. Cards are read as RGBA.

**Class regions** are reused from
`docs/ab/stroke-battery/scripts/measure_battery.py`: same segment rows
(w15/14/12/10/9 × h/v/diag + quad), same ellipse/arc/fill geometry, same
join/crossing crop boxes, same face crop `(470,260)-(830,600)` logical,
same brow lifts and apex formula, same in-situ toon head centre
(1308.7, 868.3) device px and leg line. Each profile class is measured
over the device-px box enclosing its profile geometry, expanded by the
stroke half-width + 8 device px. Two stated consequences:
`cap-end`/`cap-seg` share one pixel region (the same short segment read
axially vs across in M4), so their pixel numbers are identical by
construction; `toon-card` is the whole frame and overlaps
`toon-head-outline`/`toon-leg`.

**Metrics per class** (and per whole card):
- identical-pixel % = 100 × pixels with all RGBA channels equal / region px
- differing-pixel count = pixels with any channel different
- max per-channel |Δ| (R, G, B, A separately)
- PSNR, **both bases, labelled, in every table** (coordinator ruling):
  alpha-only PSNR (the M4-native metric) = **10·log10(255² / MSE_alpha)**,
  MSE_alpha = mean((A_cpu − A_gpu)²) over the region, alpha in 0..255;
  and RGBA PSNR = 10·log10(255² / MSE_rgba), MSE over region pixels AND
  all four channels. MSE 0 → `inf` on either basis. Headline per case
  stays: byte-identical Y/N (identical % / differing count), differing-px
  count, max per-channel |Δ|.
- differing-pixel localisation — **canonical definition (coordinator
  ruling, stated verbatim so the tables are self-describing): the
  boundary set is the fractional-α pixels of the CPU reference ONLY
  (no 4-neighbour dilation of the boundary set itself); EDGE =
  differing px at Chebyshev distance ≤ 1 from that set; FRINGE =
  Chebyshev distance 2; INTERIOR = beyond 2.** In symbols,
  B = {p : 0 < A_cpu(p) < 255}, d = Chebyshev distance to B:
  EDGE d ≤ 1, FRINGE d = 2, INTERIOR d > 2. (Cross-validation of the
  measurement chain: the AUDIT seat's independent checker measured
  RGB PSNR 20.1521 dB on the legacy §28 pair against the recorded
  20.152 dB baseline — agreement to 0.0001 dB.)
  EDGE+FRINGE+INTERIOR = differing count (asserted in the tool).
  Caveat, by definition: ink inside an opaque fill (face brows/eyes/
  mouth) has no alpha boundary, so differing pixels there classify
  INTERIOR — that is the specified alpha-boundary definition, not a
  defect of the count.

**⚠ Coordinator check before trusting the real run's alpha columns:**
the committed M4 package cards are **RGB** (alpha flattened to 255), so
on those files alpha PSNR is trivially `inf` and localisation is
all-INTERIOR. The record/replay harness must emit **RGBA** cards (the
Painter buffer is RGBA; `src/battery.rs::save` writes `RgbaImage`) —
verify `prims.png` mode is RGBA in the harvested CI output, or the
alpha-PSNR and EDGE/FRINGE columns of the real table are vacuous.

## Dry-run validation — CPU vs CPU (the table below)

Both sides = the committed CPU cards (`docs/ab/stroke-battery/native*.png`
mapped to canonical names; symlinks in
`~/workspace/talerender-evidence/gpufirst-phase0/dryrun-cpu/` and
`dryrun-gpu/`). Result: **46/46 classes at 100.0000% identical, 0
differing pixels, max Δ 0 on every channel, alpha PSNR inf, localisation
0/0/0; all 7 whole-card summaries identical.** The tool validates.

| class | card | identical % | differing px | max ΔR | max ΔG | max ΔB | max ΔA | alpha PSNR (dB) | RGBA PSNR (dB) | EDGE | FRINGE | INTERIOR |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| seg-h-15 | prims | 100.0000 | 0 | 0 | 0 | 0 | 0 | inf | inf | 0 | 0 | 0 |
| seg-v-15 | prims | 100.0000 | 0 | 0 | 0 | 0 | 0 | inf | inf | 0 | 0 | 0 |
| seg-diag-15 | prims | 100.0000 | 0 | 0 | 0 | 0 | 0 | inf | inf | 0 | 0 | 0 |
| quad-15 | prims | 100.0000 | 0 | 0 | 0 | 0 | 0 | inf | inf | 0 | 0 | 0 |
| seg-h-14 | prims | 100.0000 | 0 | 0 | 0 | 0 | 0 | inf | inf | 0 | 0 | 0 |
| seg-v-14 | prims | 100.0000 | 0 | 0 | 0 | 0 | 0 | inf | inf | 0 | 0 | 0 |
| seg-diag-14 | prims | 100.0000 | 0 | 0 | 0 | 0 | 0 | inf | inf | 0 | 0 | 0 |
| quad-14 | prims | 100.0000 | 0 | 0 | 0 | 0 | 0 | inf | inf | 0 | 0 | 0 |
| seg-h-12 | prims | 100.0000 | 0 | 0 | 0 | 0 | 0 | inf | inf | 0 | 0 | 0 |
| seg-v-12 | prims | 100.0000 | 0 | 0 | 0 | 0 | 0 | inf | inf | 0 | 0 | 0 |
| seg-diag-12 | prims | 100.0000 | 0 | 0 | 0 | 0 | 0 | inf | inf | 0 | 0 | 0 |
| quad-12 | prims | 100.0000 | 0 | 0 | 0 | 0 | 0 | inf | inf | 0 | 0 | 0 |
| seg-h-10 | prims | 100.0000 | 0 | 0 | 0 | 0 | 0 | inf | inf | 0 | 0 | 0 |
| seg-v-10 | prims | 100.0000 | 0 | 0 | 0 | 0 | 0 | inf | inf | 0 | 0 | 0 |
| seg-diag-10 | prims | 100.0000 | 0 | 0 | 0 | 0 | 0 | inf | inf | 0 | 0 | 0 |
| quad-10 | prims | 100.0000 | 0 | 0 | 0 | 0 | 0 | inf | inf | 0 | 0 | 0 |
| seg-h-9 | prims | 100.0000 | 0 | 0 | 0 | 0 | 0 | inf | inf | 0 | 0 | 0 |
| seg-v-9 | prims | 100.0000 | 0 | 0 | 0 | 0 | 0 | inf | inf | 0 | 0 | 0 |
| seg-diag-9 | prims | 100.0000 | 0 | 0 | 0 | 0 | 0 | inf | inf | 0 | 0 | 0 |
| quad-9 | prims | 100.0000 | 0 | 0 | 0 | 0 | 0 | inf | inf | 0 | 0 | 0 |
| head-outline | prims | 100.0000 | 0 | 0 | 0 | 0 | 0 | inf | inf | 0 | 0 | 0 |
| circle-stroke | prims | 100.0000 | 0 | 0 | 0 | 0 | 0 | inf | inf | 0 | 0 | 0 |
| arc-stroke | prims | 100.0000 | 0 | 0 | 0 | 0 | 0 | inf | inf | 0 | 0 | 0 |
| foot-fill | prims | 100.0000 | 0 | 0 | 0 | 0 | 0 | inf | inf | 0 | 0 | 0 |
| pupil-fill | prims | 100.0000 | 0 | 0 | 0 | 0 | 0 | inf | inf | 0 | 0 | 0 |
| bun-fill | prims | 100.0000 | 0 | 0 | 0 | 0 | 0 | inf | inf | 0 | 0 | 0 |
| eye-ring | prims | 100.0000 | 0 | 0 | 0 | 0 | 0 | inf | inf | 0 | 0 | 0 |
| cap-end | prims | 100.0000 | 0 | 0 | 0 | 0 | 0 | inf | inf | 0 | 0 | 0 |
| cap-seg | prims | 100.0000 | 0 | 0 | 0 | 0 | 0 | inf | inf | 0 | 0 | 0 |
| join-zigzag | prims | 100.0000 | 0 | 0 | 0 | 0 | 0 | inf | inf | 0 | 0 | 0 |
| join-uturn | prims | 100.0000 | 0 | 0 | 0 | 0 | 0 | inf | inf | 0 | 0 | 0 |
| cross-x | prims | 100.0000 | 0 | 0 | 0 | 0 | 0 | inf | inf | 0 | 0 | 0 |
| join-spike | prims | 100.0000 | 0 | 0 | 0 | 0 | 0 | inf | inf | 0 | 0 | 0 |
| toon-card | toon | 100.0000 | 0 | 0 | 0 | 0 | 0 | inf | inf | 0 | 0 | 0 |
| toon-head-outline | toon | 100.0000 | 0 | 0 | 0 | 0 | 0 | inf | inf | 0 | 0 | 0 |
| toon-leg | toon | 100.0000 | 0 | 0 | 0 | 0 | 0 | inf | inf | 0 | 0 | 0 |
| face-neutral | face-neutral | 100.0000 | 0 | 0 | 0 | 0 | 0 | inf | inf | 0 | 0 | 0 |
| brow-neutral | face-neutral | 100.0000 | 0 | 0 | 0 | 0 | 0 | inf | inf | 0 | 0 | 0 |
| face-determined | face-determined | 100.0000 | 0 | 0 | 0 | 0 | 0 | inf | inf | 0 | 0 | 0 |
| brow-determined | face-determined | 100.0000 | 0 | 0 | 0 | 0 | 0 | inf | inf | 0 | 0 | 0 |
| face-angry | face-angry | 100.0000 | 0 | 0 | 0 | 0 | 0 | inf | inf | 0 | 0 | 0 |
| brow-angry | face-angry | 100.0000 | 0 | 0 | 0 | 0 | 0 | inf | inf | 0 | 0 | 0 |
| face-laugh | face-laugh | 100.0000 | 0 | 0 | 0 | 0 | 0 | inf | inf | 0 | 0 | 0 |
| brow-laugh | face-laugh | 100.0000 | 0 | 0 | 0 | 0 | 0 | inf | inf | 0 | 0 | 0 |
| face-shock | face-shock | 100.0000 | 0 | 0 | 0 | 0 | 0 | inf | inf | 0 | 0 | 0 |
| brow-shock | face-shock | 100.0000 | 0 | 0 | 0 | 0 | 0 | inf | inf | 0 | 0 | 0 |

| card (whole) | identical % | differing px | max ΔR | max ΔG | max ΔB | max ΔA | alpha PSNR (dB) | RGBA PSNR (dB) | EDGE | FRINGE | INTERIOR |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| prims | 100.0000 | 0 | 0 | 0 | 0 | 0 | inf | inf | 0 | 0 | 0 |
| toon | 100.0000 | 0 | 0 | 0 | 0 | 0 | inf | inf | 0 | 0 | 0 |
| face-neutral | 100.0000 | 0 | 0 | 0 | 0 | 0 | inf | inf | 0 | 0 | 0 |
| face-determined | 100.0000 | 0 | 0 | 0 | 0 | 0 | inf | inf | 0 | 0 | 0 |
| face-angry | 100.0000 | 0 | 0 | 0 | 0 | 0 | inf | inf | 0 | 0 | 0 |
| face-laugh | 100.0000 | 0 | 0 | 0 | 0 | 0 | inf | inf | 0 | 0 | 0 |
| face-shock | 100.0000 | 0 | 0 | 0 | 0 | 0 | inf | inf | 0 | 0 | 0 |

## Worst 5 classes

**Dry run: there are none** — all 46 classes tie at exactly 0 differing
pixels; naming a "worst 5" from the dry run would be fabrication. The
five classes below are the **predicted** worst for the real GPU run,
named now (before any GPU number exists) with the raster-model
hypothesis the real table will confirm or refute:

1. **cap-end / cap-seg** — the CPU painter models Skia's cap as a
   chord-flattened fan (`CapFan`: 9 chords at R15, tip at R·cos10°,
   derived from the 0.25-device-px flattening tolerance). A GPU replay
   that tessellates caps as true semicircles (or with a different chord
   count) puts its boundary up to ~0.3 device px outside the CPU fan
   near the tip — expect EDGE-concentrated differing pixels and the
   battery's largest per-class max ΔA.
2. **head-outline / toon-head-outline** — ellipse coverage on the CPU
   side comes from a signed-distance approximation evaluated per pixel
   (plus 4×4 footprint supersampling in the AA band). A GPU SDF that
   uses the common scaled-gradient ellipse approximation has its worst
   error exactly where curvature is highest (the ±x ends of the
   138×133.86 head ellipse) — expect a thin EDGE band of small ΔA,
   growing with eccentricity; the in-situ class adds lean/camPitch
   fractional placement on top.
3. **cross-x** — two width-12 strokes drawn as separate paths: coverage
   at the crossing is two sequential alpha blends. Any GPU path that
   fuses the union before blending, or blends in a different precision,
   stacks quantisation differently in the overlap diamond — expect
   INTERIOR differing pixels (inside opaque coverage, Δ from blend
   rounding, not from geometry), the one join class whose error is
   arithmetic rather than geometric.
4. **join-zigzag / join-uturn / join-spike** — round joins are fans on
   the CPU side; GPU tessellation density at the outer arc decides the
   boundary. The acute spike is the stress case: a coarse fan chords
   across the tip. Expect EDGE pixels clustered at join apices.
5. **brow-* (×5)** — the thinnest discriminating strokes in the face
   set (M4's closest call was brow-determined, Δwidth 0.494 of 0.5 px
   against Skia). Small SDF or flattening differences move a brow's
   coverage proportionally more than a w15 limb's. Note these differing
   pixels will read INTERIOR under the alpha-boundary localisation
   (brows are ink on the opaque head fill) — read their differing count
   and max Δ, not their EDGE count.

## Exact commands for the real measurement (coordinator)

```bash
cd ~/workspace/talerender
# 0. Build the harness binary (feature gpupaint, default OFF), on CI:
cargo build --release --features gpupaint   # binary: target/release/talerender
TALERENDER=target/release/talerender
# 1. CPU reference + recorded display lists (TRACE seat, WORKING):
$TALERENDER --gpu-paint record-battery /tmp/gpufirst-cpu
#    -> /tmp/gpufirst-cpu/{prims,toon,face-<mood>}.png + <card>.json each
# 2. GPU replay of the same streams (GPUX seat — STUB at delivery,
#    exit 3 until integrated; replay is per-card by design):
mkdir -p /tmp/gpufirst-gpu
for c in prims toon face-neutral face-determined face-angry face-laugh face-shock; do
  $TALERENDER --gpu-paint replay /tmp/gpufirst-cpu/$c.json /tmp/gpufirst-gpu/$c.png
done
# 3. Verify the cards kept their alpha channel (see warning above):
python3 -c "from PIL import Image; print(Image.open('/tmp/gpufirst-cpu/prims.png').mode)"
#    must print RGBA
# 4. Compare (this seat's tool):
E=~/workspace/talerender-evidence/gpufirst-phase0
python3 tools/gpupaint-compare.py --cpu /tmp/gpufirst-cpu --gpu /tmp/gpufirst-gpu \
  --spec docs/ab/stroke-battery/spec.json \
  --out $E/gpu-vs-cpu.json --md $E/gpu-vs-cpu.md
# 5. Replace the dry-run table in this doc with $E/gpu-vs-cpu.md and
#    re-write "Worst 5" from its sorted identical-% — no other edits.
```

## Appendix — tool discrimination smoke test (NOT GPU parity)

To prove the tool detects real differences (a tool that prints 100% on
everything proves nothing), it was also run CPU-native vs the committed
Chrome/Skia originals. This is the M4 pair inverted, **not** a GPU
measurement, and its numbers must not be quoted as GPU parity.
Whole cards: prims 99.7181% identical (20,324 differing px, max ΔR/G/B
98/100/89), toon 99.5200% (34,606 px). Worst classes: eye-ring
96.9697% (592 px, max Δ 84/85/76), foot-fill 97.0610% (158 px),
brow-angry 97.1069% (237 px), brow-determined 97.2290% (227 px),
brow-neutral/laugh/shock 97.3022% (221 px each), pupil-fill 97.4440%
(146 px). Alpha-only PSNR is `inf` in this smoke run because both committed
card sets are RGB (alpha = 255 everywhere, so MSE_alpha = 0); the
RGBA basis is finite (prims 44.827 dB, toon 49.084 dB, faces
46.233–49.574 dB) because it also averages the differing RGB
channels. Localisation is all-INTERIOR for the same RGB reason —
there is no fractional-α boundary set in flattened cards. See the
coordinator warning above. Full output:
evidence dir, `smoke-cpu-vs-chrome.{json,md}`.

## Evidence (all in ~/workspace/talerender-evidence/gpufirst-phase0/)

- `gpupaint-compare.py` — byte-identical copy of `tools/gpupaint-compare.py`
- `dryrun-cpu-vs-cpu.{json,md}` — the validation run tabulated above
- `smoke-cpu-vs-chrome.{json,md}` — appendix smoke run
- `dryrun-cpu/`, `dryrun-gpu/`, `smoke-chrome/` — card dirs (symlinks
  to the committed package cards)

Nothing in this seat is committed; files are delivered on disk for the
coordinator to integrate.

## Smoke-test table (appendix, full)

| class | card | identical % | differing px | max ΔR | max ΔG | max ΔB | max ΔA | alpha PSNR (dB) | RGBA PSNR (dB) | EDGE | FRINGE | INTERIOR |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| seg-h-15 | prims | 99.7845 | 70 | 56 | 56 | 51 | 0 | inf | 48.061 | 0 | 0 | 70 |
| seg-v-15 | prims | 99.3767 | 82 | 98 | 99 | 89 | 0 | inf | 39.456 | 0 | 0 | 82 |
| seg-diag-15 | prims | 99.2605 | 590 | 98 | 99 | 89 | 0 | inf | 42.375 | 0 | 0 | 590 |
| quad-15 | prims | 99.0017 | 1199 | 98 | 100 | 89 | 0 | inf | 38.023 | 0 | 0 | 1199 |
| seg-h-14 | prims | 99.7611 | 74 | 56 | 56 | 51 | 0 | inf | 48.583 | 0 | 0 | 74 |
| seg-v-14 | prims | 99.3438 | 82 | 70 | 71 | 64 | 0 | inf | 40.088 | 0 | 0 | 82 |
| seg-diag-14 | prims | 99.2267 | 608 | 70 | 71 | 64 | 0 | inf | 42.26 | 0 | 0 | 608 |
| quad-14 | prims | 98.9983 | 1185 | 98 | 99 | 89 | 0 | inf | 38.278 | 0 | 0 | 1185 |
| seg-h-12 | prims | 99.8286 | 48 | 56 | 56 | 51 | 0 | inf | 48.694 | 0 | 0 | 48 |
| seg-v-12 | prims | 99.5714 | 48 | 56 | 56 | 51 | 0 | inf | 44.715 | 0 | 0 | 48 |
| seg-diag-12 | prims | 99.2505 | 572 | 56 | 57 | 51 | 0 | inf | 43.094 | 0 | 0 | 572 |
| quad-12 | prims | 98.9773 | 1173 | 98 | 99 | 89 | 0 | inf | 38.073 | 0 | 0 | 1173 |
| seg-h-10 | prims | 99.8244 | 44 | 56 | 56 | 51 | 0 | inf | 48.637 | 0 | 0 | 44 |
| seg-v-10 | prims | 99.5572 | 44 | 56 | 56 | 51 | 0 | inf | 44.62 | 0 | 0 | 44 |
| seg-diag-10 | prims | 99.2329 | 568 | 56 | 57 | 51 | 0 | inf | 42.87 | 0 | 0 | 568 |
| quad-10 | prims | 98.9805 | 1133 | 98 | 99 | 89 | 0 | inf | 37.877 | 0 | 0 | 1133 |
| seg-h-9 | prims | 99.7796 | 52 | 42 | 43 | 39 | 0 | inf | 48.823 | 0 | 0 | 52 |
| seg-v-9 | prims | 99.3774 | 58 | 56 | 57 | 51 | 0 | inf | 40.897 | 0 | 0 | 58 |
| seg-diag-9 | prims | 99.2129 | 574 | 56 | 56 | 51 | 0 | inf | 43.275 | 0 | 0 | 574 |
| quad-9 | prims | 98.9393 | 1160 | 98 | 99 | 89 | 0 | inf | 38.092 | 0 | 0 | 1160 |
| head-outline | prims | 99.2085 | 2662 | 84 | 86 | 76 | 0 | inf | 40.368 | 0 | 0 | 2662 |
| circle-stroke | prims | 98.4496 | 1164 | 84 | 86 | 76 | 0 | inf | 36.669 | 0 | 0 | 1164 |
| arc-stroke | prims | 98.5434 | 196 | 84 | 85 | 76 | 0 | inf | 37.418 | 0 | 0 | 196 |
| foot-fill | prims | 97.0610 | 158 | 84 | 85 | 76 | 0 | inf | 34.228 | 0 | 0 | 158 |
| pupil-fill | prims | 97.4440 | 146 | 84 | 85 | 76 | 0 | inf | 33.546 | 0 | 0 | 146 |
| bun-fill | prims | 98.8000 | 480 | 70 | 70 | 64 | 0 | inf | 39.01 | 0 | 0 | 480 |
| eye-ring | prims | 96.9697 | 592 | 84 | 85 | 76 | 0 | inf | 33.79 | 0 | 0 | 592 |
| cap-end | prims | 99.0833 | 70 | 56 | 56 | 51 | 0 | inf | 41.774 | 0 | 0 | 70 |
| cap-seg | prims | 99.0833 | 70 | 56 | 56 | 51 | 0 | inf | 41.774 | 0 | 0 | 70 |
| join-zigzag | prims | 98.9727 | 1052 | 84 | 86 | 76 | 0 | inf | 39.394 | 0 | 0 | 1052 |
| join-uturn | prims | 99.9844 | 17 | 84 | 85 | 76 | 0 | inf | 56.413 | 0 | 0 | 17 |
| cross-x | prims | 99.0087 | 1142 | 56 | 57 | 51 | 0 | inf | 39.868 | 0 | 0 | 1142 |
| join-spike | prims | 99.2337 | 938 | 98 | 99 | 89 | 0 | inf | 42.483 | 0 | 0 | 938 |
| toon-card | toon | 99.5200 | 34606 | 98 | 100 | 89 | 0 | inf | 49.084 | 0 | 0 | 34606 |
| toon-head-outline | toon | 98.4894 | 5204 | 95 | 96 | 87 | 0 | inf | 39.459 | 0 | 0 | 5204 |
| toon-leg | toon | 99.8100 | 25 | 56 | 57 | 51 | 0 | inf | 43.669 | 0 | 0 | 25 |
| face-neutral | face-neutral | 99.0355 | 4722 | 94 | 96 | 87 | 0 | inf | 40.212 | 0 | 0 | 4722 |
| brow-neutral | face-neutral | 97.3022 | 221 | 67 | 69 | 63 | 0 | inf | 33.189 | 0 | 0 | 221 |
| face-determined | face-determined | 99.0574 | 4615 | 84 | 86 | 77 | 0 | inf | 40.701 | 0 | 0 | 4615 |
| brow-determined | face-determined | 97.2290 | 227 | 81 | 82 | 74 | 0 | inf | 34.069 | 0 | 0 | 227 |
| face-angry | face-angry | 99.0229 | 4784 | 94 | 96 | 87 | 0 | inf | 40.346 | 0 | 0 | 4784 |
| brow-angry | face-angry | 97.1069 | 237 | 81 | 82 | 74 | 0 | inf | 34.038 | 0 | 0 | 237 |
| face-laugh | face-laugh | 98.8223 | 5766 | 197 | 229 | 213 | 0 | inf | 35.635 | 0 | 0 | 5766 |
| brow-laugh | face-laugh | 97.3022 | 221 | 67 | 69 | 63 | 0 | inf | 33.189 | 0 | 0 | 221 |
| face-shock | face-shock | 98.7439 | 6150 | 98 | 86 | 77 | 0 | inf | 39.085 | 0 | 0 | 6150 |
| brow-shock | face-shock | 97.3022 | 221 | 67 | 69 | 63 | 0 | inf | 33.189 | 0 | 0 | 221 |

| card (whole) | identical % | differing px | max ΔR | max ΔG | max ΔB | max ΔA | alpha PSNR (dB) | RGBA PSNR (dB) | EDGE | FRINGE | INTERIOR |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| prims | 99.7181 | 20324 | 98 | 100 | 89 | 0 | inf | 44.827 | 0 | 0 | 20324 |
| toon | 99.5200 | 34606 | 98 | 100 | 89 | 0 | inf | 49.084 | 0 | 0 | 34606 |
| face-neutral | 99.5988 | 28919 | 98 | 99 | 89 | 0 | inf | 49.311 | 0 | 0 | 28919 |
| face-determined | 99.6003 | 28812 | 98 | 99 | 89 | 0 | inf | 49.574 | 0 | 0 | 28812 |
| face-angry | 99.5980 | 28981 | 98 | 99 | 89 | 0 | inf | 49.385 | 0 | 0 | 28981 |
| face-laugh | 99.5844 | 29963 | 197 | 229 | 213 | 0 | inf | 46.233 | 0 | 0 | 29963 |
| face-shock | 99.5790 | 30347 | 98 | 99 | 89 | 0 | inf | 48.653 | 0 | 0 | 30347 |
