#!/usr/bin/env python3
"""Assemble the stroke-battery evidence package.

Composites the transparent painter cards over white (the toon
canvas's effective backdrop for these cards) and writes, into the
package directory:
  original.png / native.png          prims card, Chrome/Skia vs CPU painter
  side-by-side.png                   original | native, half width each
  diff.png                           |original - native| amplified x4
  original-toon.png / native-toon.png
  original-face-<mood>.png / native-face-<mood>.png
  metrics.json                       battery metrics (from
                                     measure_battery.py) merged with
                                     the regression block

Usage: make_package.py --ref <dir> --native <dir> --battery-metrics <json>
       --regression <json> --out <package-dir>
The regression JSON is a flat object of measured values produced by
the regression runs (ab1 / dof-chart / scene-format), kept as data
so this script invents no numbers.
"""
import argparse
import json

import numpy as np
from PIL import Image

AMP = 4
MOODS = ["neutral", "determined", "angry", "laugh", "shock"]


def composite(path):
    im = Image.open(path).convert("RGBA")
    bg = Image.new("RGBA", im.size, (255, 255, 255, 255))
    bg.alpha_composite(im)
    return bg.convert("RGB")


def save_pair(ref_path, nat_path, out_dir, stem):
    a = composite(ref_path)
    b = composite(nat_path)
    a.save(f"{out_dir}/original-{stem}.png")
    b.save(f"{out_dir}/native-{stem}.png")
    return a, b


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--ref", required=True)
    ap.add_argument("--native", required=True)
    ap.add_argument("--battery-metrics", required=True)
    ap.add_argument("--regression", required=True)
    ap.add_argument("--out", required=True)
    args = ap.parse_args()

    # prims is the headline pair: original.png / native.png
    a = composite(f"{args.ref}/prims.png")
    b = composite(f"{args.native}/prims.png")
    a.save(f"{args.out}/original.png")
    b.save(f"{args.out}/native.png")

    w, h = a.size
    sbs = Image.new("RGB", (w, h))
    sbs.paste(a.resize((w // 2, h)), (0, 0))
    sbs.paste(b.resize((w // 2, h)), (w // 2, 0))
    sbs.save(f"{args.out}/side-by-side.png")
    diff = np.abs(np.asarray(a, dtype=np.int32) - np.asarray(b, dtype=np.int32))
    Image.fromarray(np.clip(diff * AMP, 0, 255).astype(np.uint8)).save(f"{args.out}/diff.png")

    save_pair(f"{args.ref}/toon.png", f"{args.native}/toon.png", args.out, "toon")
    for mood in MOODS:
        save_pair(f"{args.ref}/face-{mood}.png", f"{args.native}/face-{mood}.png", args.out, f"face-{mood}")

    metrics = json.load(open(args.battery_metrics))
    metrics["regression"] = json.load(open(args.regression))
    with open(f"{args.out}/metrics.json", "w") as f:
        json.dump(metrics, f, indent=2)
    print("package written to", args.out)


if __name__ == "__main__":
    main()
