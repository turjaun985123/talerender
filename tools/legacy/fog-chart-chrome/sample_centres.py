#!/usr/bin/env python3
"""Sample fog-chart panel centres from a rendered frame and compare
against tools/fog-chart/expected.json.

For each panel: the mean over the +/- sampleBoxPx box at centrePixel,
the analytic expectation (expectedSrgb, fog applied), the no-fog
expectation (expectedSrgbNoFog), and the per-channel deltas against
both. The +/-2-code bar in expected.json gates the NATIVE side against
the analytic column; a Chrome capture is reference (b) — this script
reports its actual deltas, it does not pass/fail them (the package
judges Chrome by the basic panels' centres and, once a native frame
exists, by full-chart PSNR >= 45 dB).

  python3 tools/fog-chart/chrome/sample_centres.py <frame.png|frame.jpg>
"""
import json
import sys
from pathlib import Path

from PIL import Image

HERE = Path(__file__).resolve().parent
exp = json.loads((HERE.parent / "expected.json").read_text())
img = Image.open(sys.argv[1]).convert("RGB")
print(f"frame: {sys.argv[1]} ({img.width}x{img.height})")
print("| panel | sampled mean | expected (fog) | delta | expected (no fog) | delta |")
print("|---|---|---|---|---|---|")
worst_basic = 0
for p in exp["panels"]:
    x, y, b = p["centrePixel"]["x"], p["centrePixel"]["y"], p["sampleBoxPx"]
    box = img.crop((x - b, y - b, x + b + 1, y + b + 1))
    px = list(box.getdata())
    mean = [round(sum(c[i] for c in px) / len(px)) for i in range(3)]
    want = p["expectedSrgb"]
    want_nf = p["expectedSrgbNoFog"]
    d = [m - w for m, w in zip(mean, want)]
    dnf = [m - w for m, w in zip(mean, want_nf)]
    if p["material"] == "basic":
        worst_basic = max(worst_basic, max(abs(v) for v in d))
    print(f"| {p['id']} | {mean} | {want} | {d} | {want_nf} | {dnf} |")
print(f"\nworst |delta| vs expected (fog), basic panels: {worst_basic} codes")
