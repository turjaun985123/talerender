#!/usr/bin/env python3
"""Render the S01 caption strip for the native A/B frame.

The original frame's caption is drawn by the project's captions.ts
(captionText): font '900 88px Inter', centred at (540, 1340) in the
1080x1920 frame, fit = min(1, 960 / measureText(text)), three passes
— shadow silhouette rgba(0,0,0,0.45) at (+3, +9), strokeText lineWidth
15 in '#0B0B10' with round joins, fillText white. At t = 1.50 the
active chunk of line 0 is "fighting" (chunks from the line's explicit
caps field; chunk window 1.35-1.83 s, so the pop-in scale has settled
to exactly `fit`).

This script reproduces that layout with PIL/FreeType and the
project's own Inter-Black.otf (the 900 face). The rasteriser is NOT
Skia — that difference is contract §2.4 material and is logged in
gaps.md. The strip is 1080x300, centred so its middle lands on
frame y = 1340 when composited by the native renderer.

Usage: make_caption.py <project-dir> <out.png>
"""
import sys

from PIL import Image, ImageDraw, ImageFont

TEXT = "fighting"
FRAME_W, STRIP_H = 1080, 300
CAP_Y, CAP_SIZE, CAP_MAXW = 1340, 88, 960
SS = 3  # supersample for antialiasing


def main() -> None:
    project, out = sys.argv[1], sys.argv[2]
    font_path = f"{project}/fonts/Inter-Black.otf"

    measure_font = ImageFont.truetype(font_path, CAP_SIZE)
    width = measure_font.getlength(TEXT)
    fit = min(1.0, CAP_MAXW / width)
    print(f"measureText({TEXT!r}) at 88px = {width:.2f}px -> fit = {fit:.4f}")

    img = Image.new("RGBA", (FRAME_W * SS, STRIP_H * SS), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    font = ImageFont.truetype(font_path, round(CAP_SIZE * SS * fit))
    cx, cy = (FRAME_W // 2) * SS, (STRIP_H // 2) * SS
    # Pass 1: shadow silhouette at (+3, +9) in (already fit-scaled) units.
    d.text(
        (cx + 3 * SS * fit, cy + 9 * SS * fit),
        TEXT,
        font=font,
        fill=(0, 0, 0, int(0.45 * 255)),
        anchor="mm",
    )
    # Passes 2+3: ink stroke (lineWidth 15) under a white fill.
    d.text(
        (cx, cy),
        TEXT,
        font=font,
        fill=(255, 255, 255, 255),
        stroke_width=round(15 * SS * fit),
        stroke_fill=(0x0B, 0x0B, 0x10, 255),
        anchor="mm",
    )
    img = img.resize((FRAME_W, STRIP_H), Image.LANCZOS)
    img.save(out)
    print(f"wrote {out} ({FRAME_W}x{STRIP_H}), caption centre lands on frame y={CAP_Y}")


if __name__ == "__main__":
    main()
