#!/usr/bin/env python3
"""A/B proof for the production-integration film.

Compares native frames (TaleRender, renderer labelled in the run
log — lavapipe on this VM) against the project's Chrome review
frames (review/t{n}.50.jpg, one per second), computes PSNR per
review timestamp, groups by shot segment, writes side-by-side
composites for the three most different shots, and builds a
film-level contact comparison (native sheet vs review/all.jpg).

Usage: ab_compare.py FRAMES_DIR REVIEW_DIR OUT_DIR
"""
import math
import os
import sys

import numpy as np
from PIL import Image

SEGMENTS = [  # (id, start, end) — film timing, seconds
    ("S01", 0.000, 5.907), ("S02", 5.907, 13.061), ("S03", 13.061, 15.950),
    ("S04", 15.950, 21.384), ("S05a", 21.384, 24.560), ("S05b", 24.560, 26.129),
    ("S06", 26.129, 30.390), ("S07", 30.390, 34.209), ("S08", 34.209, 40.360),
    ("S08b", 40.360, 41.732), ("S09", 41.732, 45.365), ("S10", 45.365, 50.648),
    ("S11", 50.648, 55.520), ("S12", 55.520, 60.357),
]


def shot_of(t):
    for sid, a, b in SEGMENTS:
        if a <= t < b:
            return sid
    return SEGMENTS[-1][0]


def psnr(a, b):
    d = a.astype(np.float64) - b.astype(np.float64)
    mse = float(np.mean(d * d))
    if mse == 0:
        return float("inf")
    return 10.0 * math.log10(255.0 * 255.0 / mse)


def main():
    frames_dir, review_dir, out_dir = sys.argv[1], sys.argv[2], sys.argv[3]
    os.makedirs(out_dir, exist_ok=True)
    rows = []
    per_shot = {}
    thumbs_native = []
    for n in range(60):
        t = n + 0.5
        ref_path = os.path.join(review_dir, f"t{t:.2f}.jpg")
        idx = int(round(t * 30))
        nat_path = os.path.join(frames_dir, f"frame_{idx:05d}.png")
        if not (os.path.exists(ref_path) and os.path.exists(nat_path)):
            rows.append((t, shot_of(t), None, "missing"))
            continue
        ref = np.asarray(Image.open(ref_path).convert("RGB"))
        nat = np.asarray(Image.open(nat_path).convert("RGB"))
        if ref.shape != nat.shape:
            rows.append((t, shot_of(t), None, f"shape {ref.shape} vs {nat.shape}"))
            continue
        v = psnr(ref, nat)
        rows.append((t, shot_of(t), v, ""))
        per_shot.setdefault(shot_of(t), []).append((t, v))
        thumbs_native.append(Image.open(nat_path).convert("RGB").resize((180, 320)))
    lines = ["# A/B — native (TaleRender) vs Chrome review frames", "",
             "PSNR dB per review timestamp (Chrome side is q-JPEG; the",
             "JPEG generation floor caps attainable PSNR — see README).", "",
             "| t (s) | shot | PSNR dB |", "|---|---|---|"]
    for t, sid, v, note in rows:
        lines.append(f"| {t:.2f} | {sid} | {v:.2f} |" if v is not None else f"| {t:.2f} | {sid} | — {note} |")
    lines += ["", "## Per-shot summary", "", "| shot | n | min | mean |", "|---|---|---|---|"]
    shot_mean = {}
    for sid, vals in sorted(per_shot.items()):
        vs = [v for _, v in vals]
        shot_mean[sid] = sum(vs) / len(vs)
        lines.append(f"| {sid} | {len(vs)} | {min(vs):.2f} | {shot_mean[sid]:.2f} |")
    allv = [v for _, _, v, _ in rows if v is not None]
    if allv:
        lines += ["", f"Overall: n={len(allv)} min={min(allv):.2f} mean={sum(allv)/len(allv):.2f} dB"]
    with open(os.path.join(out_dir, "ab-table.md"), "w") as f:
        f.write("\n".join(lines) + "\n")
    # Side-by-sides for the three most different shots (their
    # lowest-PSNR review timestamp).
    worst = sorted(shot_mean.items(), key=lambda kv: kv[1])[:3]
    for sid, _ in worst:
        t, _v = min(per_shot[sid], key=lambda tv: tv[1])
        idx = int(round(t * 30))
        ref = Image.open(os.path.join(review_dir, f"t{t:.2f}.jpg")).convert("RGB").resize((540, 960))
        nat = Image.open(os.path.join(frames_dir, f"frame_{idx:05d}.png")).convert("RGB").resize((540, 960))
        comp = Image.new("RGB", (1080, 960))
        comp.paste(ref, (0, 0))
        comp.paste(nat, (540, 0))
        comp.save(os.path.join(out_dir, f"side-by-side-{sid}-t{t:.2f}.jpg"), quality=90)
    # Film-level contact comparison: native sheet (6x10) beside the
    # project's review/all.jpg scaled to the same height.
    if thumbs_native:
        cols, rows_n = 6, 10
        sheet = Image.new("RGB", (cols * 180, rows_n * 320))
        for i, th in enumerate(thumbs_native[: cols * rows_n]):
            sheet.paste(th, ((i % cols) * 180, (i // cols) * 320))
        sheet.save(os.path.join(out_dir, "native-sheet.jpg"), quality=88)
        allp = os.path.join(review_dir, "all.jpg")
        if os.path.exists(allp):
            ref_sheet = Image.open(allp).convert("RGB")
            h = sheet.height
            ref_sheet = ref_sheet.resize((int(ref_sheet.width * h / ref_sheet.height), h))
            comp = Image.new("RGB", (sheet.width + ref_sheet.width, h))
            comp.paste(sheet, (0, 0))
            comp.paste(ref_sheet, (sheet.width, 0))
            comp.save(os.path.join(out_dir, "contact-comparison.jpg"), quality=88)
    print("\n".join(lines))


if __name__ == "__main__":
    main()
