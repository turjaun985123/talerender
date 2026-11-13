#!/usr/bin/env python3
"""DoF chart measurement — TaleRender M3 gate (contract §1.2).

Reads original.png (Chrome reference, real three r170 composer) and
native.png (talerender --scene dof-chart) from the package directory,
and writes:
  metrics.json     §28-style global metrics + the per-plane blur table
  side-by-side.png original | native, full height, half width each
  diff.png         absolute difference amplified x4 (clipped)

Blur is measured as the 10-90% transition width (px) of averaged edge
profiles: the chart's big step edge at u = 0.79 on every panel
(primary), and the checkerboard's vertical edges (secondary, median).
A third column predicts the width from the transcribed 41-tap kernel
alone (discrete tap offsets convolved with the 1 px bilinear tent), so
the table separates "does the transcription match the reference" from
"does either match the pure kernel" (both renders also carry the
scene pass's own texture filtering).
"""
import json
import math
import os

import numpy as np
from PIL import Image

D = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
W, H = 1080, 1920
FOCUS, APERTURE, MAXBLUR = 10.0, 0.0015, 0.006
PANEL_NDC_W, PANEL_NDC_H, BLEED_NDC_H = 0.56, 0.36, 0.30
PANELS = [  # (ndc_x, ndc_y, depth_m) — the shared chart spec
    (-0.62, 0.66, 30.0), (0.0, 0.66, 14.0), (0.62, 0.66, 11.0),
    (-0.62, 0.0, 12.0), (0.0, 0.0, 10.0), (0.62, 0.0, 8.0),
    (-0.62, -0.66, 7.0), (0.0, -0.66, 5.0), (0.62, -0.66, 3.5),
    (0.0, -0.30, 2.0),
]

# The 41 tap offsets of the r170 BokehShader (x components drive the
# horizontal edge spread; ring scales 1.0 / 0.9 / 0.7 / 0.4 applied).
TAPS_X = (
    [0.0]
    + [0.0, 0.15, 0.29, -0.37, 0.40, 0.37, 0.29, -0.15,
       0.0, -0.15, -0.29, 0.37, -0.4, -0.37, -0.29, 0.15]
    + [v * 0.9 for v in (0.15, -0.37, 0.37, -0.15, -0.15, 0.37, -0.37, 0.15)]
    + [v * 0.7 for v in (0.29, 0.40, 0.29, 0.0, -0.29, -0.4, -0.29, 0.0)]
    + [v * 0.4 for v in (0.29, 0.4, 0.29, 0.0, -0.29, -0.4, -0.29, 0.0)]
)


def kernel_predicted_width(dofblur):
    """10-90 width (px) of the discrete kernel's step response."""
    xs = np.arange(-8.0, 8.01, 0.05)
    prof = np.zeros_like(xs)
    for ox in TAPS_X:
        prof += np.clip(xs - ox * abs(dofblur) * W + 0.5, 0.0, 1.0)
    prof /= 41.0
    return crossing_width(xs, prof, 0.1, 0.9)


def crossing_width(xs, prof, lo_frac, hi_frac):
    lo = prof[0] + lo_frac * (prof[-1] - prof[0])
    hi = prof[0] + hi_frac * (prof[-1] - prof[0])
    def cross(level):
        for i in range(1, len(xs)):
            a, b = prof[i - 1], prof[i]
            if (a - level) * (b - level) <= 0 and a != b:
                return xs[i - 1] + (level - a) * (xs[i] - xs[i - 1]) / (b - a)
        return float("nan")
    return abs(cross(hi) - cross(lo))


def panel_rect(nx, ny, d):
    h_ndc = BLEED_NDC_H if d == 2.0 else PANEL_NDC_H
    x0 = (nx - PANEL_NDC_W / 2 + 1) / 2 * W
    x1 = (nx + PANEL_NDC_W / 2 + 1) / 2 * W
    y0 = (1 - (ny + h_ndc / 2 + 1) / 2) * H
    y1 = (1 - (ny - h_ndc / 2 + 1) / 2) * H
    return x0, x1, y0, y1


def step_width(gray, x_edge, rows, half_win=28):
    prof = gray[rows[0]:rows[1], :].mean(axis=0)
    xi = int(round(x_edge))
    seg = prof[xi - half_win:xi + half_win + 1]
    xs = np.arange(xi - half_win, xi + half_win + 1, dtype=float)
    lo = np.median(seg[:6])
    hi = np.median(seg[-6:])
    if hi - lo < 40:
        return float("nan")
    return crossing_width(xs, seg, 0.1, 0.9)


def checker_width(gray, rect, bands):
    x0, x1, y0, y1 = rect
    pw, ph = x1 - x0, y1 - y0
    widths = []
    for k in bands:
        r0 = int(y0 + (k + 0.15) / 10 * ph)
        r1 = int(y0 + (k + 0.85) / 10 * ph)
        prof = gray[r0:r1, :].mean(axis=0)
        for u in (0.1, 0.2, 0.3, 0.4):
            xi = int(round(x0 + u * pw))
            seg = prof[xi - 11:xi + 12]
            xs = np.arange(xi - 11, xi + 12, dtype=float)
            vmin, vmax = seg.min(), seg.max()
            if vmax - vmin < 40:
                continue
            widths.append(crossing_width(xs, seg, 0.1, 0.9))
    return float(np.median(widths)) if widths else float("nan"), len(widths)


def main():
    orig = Image.open(f"{D}/original.png").convert("RGB")
    nat = Image.open(f"{D}/native.png").convert("RGB")
    assert orig.size == (W, H) and nat.size == (W, H)
    a = np.asarray(orig, dtype=np.float64)
    b = np.asarray(nat, dtype=np.float64)
    ga = a.mean(axis=2)
    gb = b.mean(axis=2)

    diff = np.abs(a - b)
    mse = float((diff ** 2).mean())
    psnr = float("inf") if mse == 0 else float(10 * np.log10(255.0 ** 2 / mse))
    metrics = {
        "pair": "original.png (Chrome/SwiftShader, three r170 real composer) vs native.png (talerender --scene dof-chart, wgpu llvmpipe)",
        "psnr_db": round(psnr, 3),
        "mean_abs_delta_per_channel": round(float(diff.mean()), 3),
        "pct_pixels_max_channel_delta_gt_10": round(float((diff.max(axis=2) > 10).mean() * 100), 3),
        "pct_pixels_any_delta": round(float((diff.max(axis=2) > 0).mean() * 100), 3),
        "diff_amplification": 4,
        "chart": {
            "focus_m": FOCUS, "aperture": APERTURE, "maxblur": MAXBLUR,
            "camera": "fov 30, near 0.05, far 800 (engine defaults)",
            "look_neutralised": "bloom 0, exposure 1, vignette 0, grain 0, warm 0, sat 1",
        },
    }

    focal_rect = panel_rect(0.0, 0.0, 10.0)
    bleed_rect = panel_rect(0.0, -0.30, 2.0)
    table = []
    for (nx, ny, d) in PANELS:
        rect = panel_rect(nx, ny, d)
        x0, x1, y0, y1 = rect
        pw = x1 - x0
        x_edge = x0 + 0.79 * pw
        if d == 2.0:
            rows = (int(focal_rect[3]) + 8, int(y1) - 10)  # free (unoccluded) strip
            bands = list(range(1, 10))
        else:
            rows = (int(y0 + 0.2 * (y1 - y0)), int(y0 + 0.8 * (y1 - y0)))
            bands = list(range(10))
        dofblur = max(-MAXBLUR, min(MAXBLUR, (FOCUS - d) * APERTURE))
        sw_o = step_width(ga, x_edge, rows)
        sw_n = step_width(gb, x_edge, rows)
        cw_o, n_o = checker_width(ga, rect, bands)
        cw_n, n_n = checker_width(gb, rect, bands)
        # PSNR restricted to the smooth step zone (u in 0.55..0.97)
        zx0, zx1 = int(x0 + 0.55 * pw), int(x0 + 0.97 * pw)
        za = a[rows[0]:rows[1], zx0:zx1]
        zb = b[rows[0]:rows[1], zx0:zx1]
        zmse = float(((za - zb) ** 2).mean())
        zpsnr = float("inf") if zmse == 0 else float(10 * np.log10(255.0 ** 2 / zmse))
        table.append({
            "depth_m": d,
            "dofblur": round(dofblur, 6),
            "step_width_px": {"original": round(sw_o, 2), "native": round(sw_n, 2),
                              "delta": round(sw_n - sw_o, 2),
                              "kernel_prediction": round(kernel_predicted_width(dofblur), 2)},
            "checker_width_px_median": {"original": round(cw_o, 2), "native": round(cw_n, 2),
                                        "delta": round(cw_n - cw_o, 2),
                                        "samples": min(n_o, n_n)},
            "step_zone_psnr_db": round(zpsnr, 2),
        })
    metrics["blur_table"] = table

    # Foreground bleed: RMSE (ref vs native) in the focal-panel strip
    # directly above the 2 m panel's top edge, vs a control strip.
    fx0, fx1 = int(focal_rect[0]) + 8, int(focal_rect[1]) - 8
    bleed_top = int(bleed_rect[2])
    strip = (a[bleed_top - 36:bleed_top - 4, fx0:fx1] - b[bleed_top - 36:bleed_top - 4, fx0:fx1])
    ctrl_y = int(focal_rect[2])
    ctrl = (a[ctrl_y + 4:ctrl_y + 36, fx0:fx1] - b[ctrl_y + 4:ctrl_y + 36, fx0:fx1])
    metrics["foreground_bleed"] = {
        "rmse_above_bleed_edge": round(float(np.sqrt((strip ** 2).mean())), 3),
        "rmse_control_strip": round(float(np.sqrt((ctrl ** 2).mean())), 3),
        "note": "both strips are focal-panel (10 m) texels; the first sits within one maxblur radius band above the 2 m panel's silhouette, where the shader's no-depth-test gather spills foreground colour",
    }

    with open(f"{D}/metrics.json", "w") as f:
        json.dump(metrics, f, indent=2)
    half = Image.new("RGB", (W, H))
    half.paste(orig.resize((W // 2, H)), (0, 0))
    half.paste(nat.resize((W // 2, H)), (W // 2, 0))
    half.save(f"{D}/side-by-side.png")
    Image.fromarray(np.clip(diff * 4, 0, 255).astype(np.uint8)).save(f"{D}/diff.png")
    print(f"PSNR {psnr:.3f} dB, mean|d| {diff.mean():.3f}")
    for t in table:
        s = t["step_width_px"]
        c = t["checker_width_px_median"]
        print(f"  d={t['depth_m']:>5} m  dofblur={t['dofblur']:+.5f}  step: ref {s['original']:.2f} / nat {s['native']:.2f} / pred {s['kernel_prediction']:.2f}  checker: ref {c['original']:.2f} / nat {c['native']:.2f}  zonePSNR {t['step_zone_psnr_db']:.1f}")
    print(f"  bleed RMSE {metrics['foreground_bleed']['rmse_above_bleed_edge']} vs control {metrics['foreground_bleed']['rmse_control_strip']}")
    print(f"wrote metrics.json + side-by-side.png + diff.png in {D}")


if __name__ == "__main__":
    main()
