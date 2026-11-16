#!/usr/bin/env python3
"""GPU-first Phase 0 — stroke-battery GPU-vs-CPU parity comparison.

Reference = CPU Painter cards. Candidate = GPU replay of the SAME
recorded primitive streams (TRACE seat's `--gpu-paint record-battery`
output replayed by GPUX seat's `replay`). This tool knows nothing
about either producer: it compares two directories of battery cards.

    CPU dir / GPU dir layout (same names as src/battery.rs and
    docs/ab/stroke-battery/scripts/measure_battery.py):
        prims.png, toon.png, face-<mood>.png   (mood from spec.json)

Per class (the battery's 46 classes; geometry/regions REUSED from
docs/ab/stroke-battery/scripts/measure_battery.py — same rows, same
join/crossing crop boxes, same face crop, same brow lifts/apex, same
toon head centre and leg line; each profile class is measured over
the axis-aligned device-px box enclosing its profile geometry,
expanded by the stroke half-width + 8 device px):

  identical_pct   100 * pixels with ALL RGBA channels equal / region px
  differing_px    pixels with ANY channel different
  max_delta       max per-channel |CPU - GPU|, reported per channel
  alpha_psnr_db   alpha-only PSNR (the M4-native metric), alpha 0..255:
                      PSNR = 10 * log10(255^2 / MSE_alpha),
                      MSE_alpha = mean((A_cpu - A_gpu)^2) over the region
                  MSE = 0 is reported as "inf".
  rgba_psnr_db    RGBA PSNR, same formula with MSE over all four
                  channels (mean over pixels AND channels). Both bases
                  are reported, labelled, in every table (coordinator
                  ruling, GPU-first Phase 0).
  localisation    every differing pixel is classified by its Chebyshev
                  distance d to the coverage-boundary set, defined
                  (canonical, coordinator ruling) as the fractional-α
                  pixels of the CPU reference ONLY — the boundary set
                  itself is NOT dilated; dilation enters only through
                  the Chebyshev distance:
                  B = {p : 0 < A_cpu(p) < 255}:
                      EDGE     d <= 1  (the pixel is fractional itself,
                               or a 4-/8-neighbour of a fractional pixel)
                      FRINGE   d == 2
                      INTERIOR d  > 2
                  edge + fringe + interior == differing_px, always.
                  NOTE: the boundary is ALPHA coverage only, per spec.
                  Ink drawn inside an opaque fill (face brows/eyes/
                  mouth) has no alpha boundary, so differing pixels
                  there classify INTERIOR by construction — that is
                  the definition, not a smoothing of the result.

Whole-card summaries (prims, toon, face-<mood> x5) use the same
metrics over the full frame.

PNG decode: Python + Pillow (PIL 10.2.0) + numpy (1.26.4), both
present in this environment — no Rust `image`-crate shim needed.
Cards are converted to RGBA; an RGB card reads alpha = 255.

Usage:
  gpupaint-compare.py --cpu <dir> --gpu <dir> --spec <spec.json>
                      [--out results.json] [--md results.md]
  gpupaint-compare.py --cpu-card <png> --gpu-card <png>
                      --card prims --spec <spec.json> [--out ...]
                      # single-card mode: only that card's classes
No tolerances, no verdicts, no smoothing: raw parity numbers only.
The M4 Chrome/Skia tolerances are a different question (see
docs/ab/stroke-battery/) and are NOT applied here.
"""
import argparse
import json
import math
import sys

import numpy as np
from PIL import Image

# ---------------------------------------------------------------- regions
# All geometry below is copied from measure_battery.py (see its
# docstring/comments); res-scaled with D() exactly as there.


def class_regions(spec):
    """Ordered {class_name: (card_stem, (x0, y0, x1, y1) device px)}.

    card_stem: "prims", "toon", or "face-<mood>".
    """
    res = spec["canvas"]["res"]
    W, H = spec["canvas"]["w"] * res, spec["canvas"]["h"] * res
    D = lambda x: x * res
    M = 8  # device-px margin beyond stroke half-width / shape extent
    R = {}

    def box_pts(pts, hw):
        xs = [p[0] for p in pts]
        ys = [p[1] for p in pts]
        e = hw + M
        return (
            max(0, int(math.floor(min(xs) - e))),
            max(0, int(math.floor(min(ys) - e))),
            min(W, int(math.ceil(max(xs) + e))),
            min(H, int(math.ceil(max(ys) + e))),
        )

    def box_c(c, rx, ry, hw=0):
        e = hw + M
        return (
            max(0, int(math.floor(c[0] - rx - e))),
            max(0, int(math.floor(c[1] - ry - e))),
            min(W, int(math.ceil(c[0] + rx + e))),
            min(H, int(math.ceil(c[1] + ry + e))),
        )

    # ---- straight segments + quads, measure_battery rows ----
    rows = [(15, 110), (14, 280), (12, 450), (10, 620), (9, 790)]
    for w, y in rows:
        hw = D(w) / 2.0
        R[f"seg-h-{w}"] = ("prims", box_pts([(D(70), D(y)), (D(400), D(y))], hw))
        R[f"seg-v-{w}"] = ("prims", box_pts([(D(500), D(y - 60)), (D(500), D(y + 60))], hw))
        R[f"seg-diag-{w}"] = ("prims", box_pts([(D(600), D(y + 43)), (D(760), D(y - 43))], hw))
        # quad P0/Pc/P2 as in spec prims for this row; sample the curve
        p0, pc, p2 = (860, y + 55), (1035, y - 60), (1210, y + 55)
        curve = []
        for i in range(33):
            t = i / 32.0
            curve.append(
                (
                    D((1 - t) ** 2 * p0[0] + 2 * (1 - t) * t * pc[0] + t * t * p2[0]),
                    D((1 - t) ** 2 * p0[1] + 2 * (1 - t) * t * pc[1] + t * t * p2[1]),
                )
            )
        R[f"quad-{w}"] = ("prims", box_pts(curve, hw))

    # ---- row 6: ellipses, arcs, fills (measure_battery values) ----
    R["head-outline"] = ("prims", box_c((D(250), D(960)), D(138), D(133.86), D(10) / 2))
    R["circle-stroke"] = ("prims", box_c((D(560), D(960)), D(60), D(60), D(9) / 2))
    R["arc-stroke"] = ("prims", box_c((D(1210), D(960)), D(20), D(20), D(10) / 2))
    R["foot-fill"] = ("prims", box_c((D(760), D(960)), D(20), D(10)))
    R["pupil-fill"] = ("prims", box_c((D(830), D(960)), D(13), D(17)))
    R["bun-fill"] = ("prims", box_c((D(1090), D(960)), D(46), D(46)))
    # eye: white fill ellipse 26x30 + width-6 ring -> outer extent +3
    R["eye-ring"] = ("prims", box_c((D(960), D(960)), D(26) + D(3), D(30) + D(3)))
    # cap classes: both profiles live on the one short segment
    # (520,1332)-(580,1332) w15; cap-end reads it axially, cap-seg
    # across it — the pixel region is the same, so their pixel
    # numbers are identical by construction (stated, not hidden).
    cap_box = box_pts([(D(520), D(1332)), (D(580), D(1332))], D(15) / 2)
    R["cap-end"] = ("prims", cap_box)
    R["cap-seg"] = ("prims", cap_box)

    # ---- join / crossing crops (logical boxes, measure_battery) ----
    for name, (x0, y0, x1, y1) in {
        "join-zigzag": (150, 1050, 310, 1210),
        "join-uturn": (920, 1050, 1090, 1210),
        "cross-x": (120, 1240, 300, 1400),
        "join-spike": (700, 1190, 880, 1360),
    }.items():
        R[name] = ("prims", (D(x0), D(y0), D(x1), D(y1)))

    # ---- faces: head crop + brow box per mood ----
    lifts = {"neutral": 4, "determined": -4, "angry": -10, "laugh": 10, "shock": 24}
    for mood in spec["faces"]["moods"]:
        R[f"face-{mood}"] = (f"face-{mood}", (D(470), D(260), D(830), D(600)))
        brow_cx = (384 + 302) * res
        brow_cy = (256 + (182 - 44 - lifts[mood]) - 3.5) * res
        # profile in measure_battery spans cy +/- 26 device px along x=cx;
        # box widens x by 64 device px to cover the brow stroke's extent
        R[f"brow-{mood}"] = (
            f"face-{mood}",
            (
                max(0, int(math.floor(brow_cx - 64))),
                max(0, int(math.floor(brow_cy - 32))),
                min(W, int(math.ceil(brow_cx + 64))),
                min(H, int(math.ceil(brow_cy + 32))),
            ),
        )

    # ---- toon card ----
    R["toon-card"] = ("toon", (0, 0, W, H))
    head_c = (1308.7, 868.3)  # device px, measure_battery in-situ centre
    rp_dev = 138 * (1 + 0.04 * 0.28747) * res
    R["toon-head-outline"] = (
        "toon",
        box_c(head_c, rp_dev, rp_dev * 0.97, D(10) / 2),
    )
    # leg: horizontal profile at device y = D(1043), x in D(560)..D(680)
    R["toon-leg"] = (
        "toon",
        box_pts([(D(560), D(1043)), (D(680), D(1043))], D(15) / 2),
    )
    return R


# ---------------------------------------------------------------- metrics


def load_rgba(path):
    return np.asarray(Image.open(path).convert("RGBA"), dtype=np.int16)


def dilate3(mask):
    """One 3x3 (Chebyshev-1) binary dilation via shifts."""
    m = mask.copy()
    m[1:, :] |= mask[:-1, :]
    m[:-1, :] |= mask[1:, :]
    m[:, 1:] |= mask[:, :-1]
    m[:, :-1] |= mask[:, 1:]
    m[1:, 1:] |= mask[:-1, :-1]
    m[1:, :-1] |= mask[:-1, 1:]
    m[:-1, 1:] |= mask[1:, :-1]
    m[:-1, :-1] |= mask[1:, 1:]
    return m


def region_metrics(cpu, gpu, box):
    """cpu/gpu: int16 RGBA arrays (full card). box: device-px crop."""
    x0, y0, x1, y1 = (int(v) for v in box)
    a = cpu[y0:y1, x0:x1].astype(np.int32)
    b = gpu[y0:y1, x0:x1].astype(np.int32)
    if a.shape != b.shape:
        raise ValueError(f"card size mismatch in region {box}: {a.shape} vs {b.shape}")
    diff = a - b
    adiff = np.abs(diff)
    differing = np.any(adiff > 0, axis=2)
    n = differing.size
    ndiff = int(differing.sum())
    mse_a = float(np.mean(diff[:, :, 3].astype(np.float64) ** 2))
    psnr = float("inf") if mse_a == 0.0 else 10.0 * math.log10(255.0**2 / mse_a)
    mse_rgba = float(np.mean(diff.astype(np.float64) ** 2))
    psnr_rgba = float("inf") if mse_rgba == 0.0 else 10.0 * math.log10(255.0**2 / mse_rgba)
    # localisation vs CPU alpha coverage boundary (full-card mask,
    # cropped — a differing pixel near the crop edge still sees
    # boundary pixels just outside the region)
    alpha = cpu[:, :, 3]
    boundary = (alpha > 0) & (alpha < 255)
    near1 = dilate3(boundary)
    near2 = dilate3(near1)
    n1 = near1[y0:y1, x0:x1]
    n2 = near2[y0:y1, x0:x1]
    edge = int((differing & n1).sum())
    fringe = int((differing & ~n1 & n2).sum())
    interior = int((differing & ~n2).sum())
    assert edge + fringe + interior == ndiff
    return {
        "region_px": [x0, y0, x1, y1],
        "pixels": n,
        "identical_pct": round(100.0 * (n - ndiff) / n, 6),
        "differing_px": ndiff,
        "max_delta": {
            ch: int(adiff[:, :, i].max()) for i, ch in enumerate("RGBA")
        },
        "alpha_psnr_db": "inf" if math.isinf(psnr) else round(psnr, 3),
        "rgba_psnr_db": "inf" if math.isinf(psnr_rgba) else round(psnr_rgba, 3),
        "localisation": {"edge": edge, "fringe": fringe, "interior": interior},
    }


def card_path(directory, stem):
    return f"{directory}/{stem}.png"


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--cpu", help="dir of CPU (reference) cards")
    ap.add_argument("--gpu", help="dir of GPU (candidate) cards")
    ap.add_argument("--cpu-card", help="single CPU card PNG (with --card)")
    ap.add_argument("--gpu-card", help="single GPU card PNG (with --card)")
    ap.add_argument("--card", help="card stem for single-card mode, e.g. prims")
    ap.add_argument("--spec", required=True)
    ap.add_argument("--out")
    ap.add_argument("--md")
    args = ap.parse_args()
    if bool(args.cpu) == bool(args.cpu_card):
        ap.error("give exactly one of --cpu <dir> / --cpu-card <png>")

    spec = json.load(open(args.spec))
    regions = class_regions(spec)
    moods = spec["faces"]["moods"]
    stems = ["prims", "toon"] + [f"face-{m}" for m in moods]
    if args.card:
        stems = [args.card]
        regions = {k: v for k, v in regions.items() if v[0] == args.card}

    cards = {}
    classes = {}
    for stem in stems:
        cpu_p = args.cpu_card if args.card else card_path(args.cpu, stem)
        gpu_p = args.gpu_card if args.card else card_path(args.gpu, stem)
        try:
            cpu = load_rgba(cpu_p)
            gpu = load_rgba(gpu_p)
        except FileNotFoundError as e:
            print(f"ERROR: missing card: {e.filename}", file=sys.stderr)
            return 2
        if cpu.shape != gpu.shape:
            print(f"ERROR: {stem}: size mismatch {cpu.shape} vs {gpu.shape}", file=sys.stderr)
            return 2
        h, w = cpu.shape[:2]
        cards[stem] = {"card": stem, **region_metrics(cpu, gpu, (0, 0, w, h))}
        for name, (cstem, box) in regions.items():
            if cstem == stem:
                classes[name] = {"card": stem, **region_metrics(cpu, gpu, box)}

    report = {
        "reference": "CPU Painter cards (TaleRender src/scene.rs via battery harness)",
        "candidate": "GPU replay of the same recorded primitive streams",
        "alpha_psnr_formula": "alpha-only PSNR (M4-native): 10*log10(255^2 / MSE_alpha), MSE over region, alpha 0..255; inf when MSE=0",
        "rgba_psnr_formula": "RGBA PSNR: 10*log10(255^2 / MSE_rgba), MSE over region pixels AND all 4 channels, values 0..255; inf when MSE=0",
        "localisation": "Chebyshev distance to CPU alpha boundary B={0<A_cpu<255}: edge<=1, fringe==2, interior>2",
        "classes": classes,
        "cards": cards,
    }
    if args.out:
        with open(args.out, "w") as f:
            json.dump(report, f, indent=2)
        print("wrote", args.out)
    if args.md:
        with open(args.md, "w") as f:
            f.write(render_md(report))
        print("wrote", args.md)
    for stem, c in cards.items():
        print(
            f"{stem:18s} identical {c['identical_pct']:.4f}%  differing {c['differing_px']}"
            f"  alpha PSNR {c['alpha_psnr_db']}  RGBA PSNR {c['rgba_psnr_db']}  maxΔ {c['max_delta']}"
            f"  E/F/I {c['localisation']['edge']}/{c['localisation']['fringe']}/{c['localisation']['interior']}"
        )
    worst = sorted(classes.items(), key=lambda kv: kv[1]["identical_pct"])[:5]
    print("worst classes:", ", ".join(f"{k} ({v['identical_pct']}%)" for k, v in worst))
    return 0


def render_md(report):
    lines = [
        "| class | card | identical % | differing px | max ΔR | max ΔG | max ΔB | max ΔA | alpha PSNR (dB) | RGBA PSNR (dB) | EDGE | FRINGE | INTERIOR |",
        "|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|",
    ]
    for name, m in report["classes"].items():
        md = m["max_delta"]
        loc = m["localisation"]
        lines.append(
            f"| {name} | {m['card']} | {m['identical_pct']:.4f} | {m['differing_px']} "
            f"| {md['R']} | {md['G']} | {md['B']} | {md['A']} | {m['alpha_psnr_db']} | {m['rgba_psnr_db']} "
            f"| {loc['edge']} | {loc['fringe']} | {loc['interior']} |"
        )
    lines += [
        "",
        "| card (whole) | identical % | differing px | max ΔR | max ΔG | max ΔB | max ΔA | alpha PSNR (dB) | RGBA PSNR (dB) | EDGE | FRINGE | INTERIOR |",
        "|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|",
    ]
    for stem, m in report["cards"].items():
        md = m["max_delta"]
        loc = m["localisation"]
        lines.append(
            f"| {stem} | {m['identical_pct']:.4f} | {m['differing_px']} "
            f"| {md['R']} | {md['G']} | {md['B']} | {md['A']} | {m['alpha_psnr_db']} | {m['rgba_psnr_db']} "
            f"| {loc['edge']} | {loc['fringe']} | {loc['interior']} |"
        )
    return "\n".join(lines) + "\n"


if __name__ == "__main__":
    sys.exit(main())
