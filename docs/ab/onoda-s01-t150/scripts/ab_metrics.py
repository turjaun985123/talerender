#!/usr/bin/env python3
"""A/B metrics + evidence images for the §28 reproduction.

Compares original.png (the project's own SwiftShader-Chrome frame)
with native.png (TaleRender on lavapipe) and writes:
  metrics.json     PSNR (dB, on RGB), mean |delta| per channel,
                   % of pixels whose max-channel |delta| > 10
  side-by-side.png original | native, full height, half width each
  diff.png         absolute difference amplified x4 (clipped)

Usage: ab_metrics.py <package-dir>
"""
import json
import sys

import numpy as np
from PIL import Image

AMP = 4


def main() -> None:
    d = sys.argv[1]
    a = np.asarray(Image.open(f"{d}/original.png").convert("RGB"), dtype=np.float64)
    b = np.asarray(Image.open(f"{d}/native.png").convert("RGB"), dtype=np.float64)
    assert a.shape == b.shape, f"shape mismatch: {a.shape} vs {b.shape}"

    diff = np.abs(a - b)
    mse = float(np.mean((a - b) ** 2))
    psnr = float("inf") if mse == 0 else float(10 * np.log10(255.0**2 / mse))
    metrics = {
        "size": [int(a.shape[1]), int(a.shape[0])],
        "psnr_db": round(psnr, 3),
        "mean_abs_delta_per_channel": round(float(diff.mean()), 3),
        "pct_pixels_max_channel_delta_gt_10": round(
            float((diff.max(axis=2) > 10).mean() * 100), 3
        ),
        "pct_pixels_any_channel_delta_gt_0": round(
            float((diff.max(axis=2) > 0).mean() * 100), 3
        ),
        "original": "project review frame t1.50.jpg (SwiftShader Chrome, OP.frame JPEG q~0.92), converted losslessly to PNG",
        "native": "TaleRender --scene ab1, wgpu on lavapipe (software Vulkan), PNG direct",
        "diff_amplification": AMP,
    }
    with open(f"{d}/metrics.json", "w") as f:
        json.dump(metrics, f, indent=2)
    print(json.dumps(metrics, indent=2))

    h, w = a.shape[:2]
    half = Image.new("RGB", (w, h))
    half.paste(Image.open(f"{d}/original.png").convert("RGB").resize((w // 2, h)), (0, 0))
    half.paste(Image.open(f"{d}/native.png").convert("RGB").resize((w // 2, h)), (w // 2, 0))
    half.save(f"{d}/side-by-side.png")

    amplified = np.clip(diff * AMP, 0, 255).astype(np.uint8)
    Image.fromarray(amplified).save(f"{d}/diff.png")
    print(f"wrote side-by-side.png + diff.png (x{AMP}) + metrics.json in {d}")


if __name__ == "__main__":
    main()
