#!/usr/bin/env python3
"""Render the film's 2D plates with PIL: held-prop plates (paper /
book / leaflet / coconut — composited into the character card by
src/toonfull.rs), caption strips, and the S07 calendar cards.

All geometry mirrors the source prop functions (toon.ts PROPS +
shots.ts leafletProp / coconutProp / drawCalendar) at their own
numbers. Text is drawn with the project's own fonts — text enters
the native film as data, never typeset by the renderer.

Plate convention (matches toonfull::Plate): the plate is stage
units at res 2; plate pixel (0,0) is stage-local (x0, y0)
relative to the hand point the PropFn receives (before the
prop's internal translate/rotate, which are baked into the
plate). For rotated props the sub-image is centred exactly on
the prop's translate origin, so PIL's centre rotation is the
canvas's rotation about that same point.

Usage: python3 film/gen_assets.py [project-dir]
Reads film/onoda.json (run gen_film_data.py first).
"""
import json
import math
import sys
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

HERE = Path(__file__).parent
PROJ = Path(sys.argv[1]) if len(sys.argv) > 1 else Path.home() / "faceless-shorts/the-soldier-who-didnt-know"
ASSETS = HERE / "assets"
ASSETS.mkdir(exist_ok=True)
DATA = json.loads((HERE / "onoda.json").read_text())

BLACK = str(PROJ / "fonts/Inter-Black.otf")
XBD = str(PROJ / "fonts/Inter-ExtraBold.otf")
INK = (31, 28, 51, 255)
RES = 2.0


def F(path, size):
    return ImageFont.truetype(path, int(size * RES))


def canvas_rotate(img, a):
    """PIL rotate(phi) == canvas rotate(-phi); we want canvas
    rotate(a), so phi = -a (degrees)."""
    return img.rotate(-math.degrees(a), resample=Image.BILINEAR, expand=False)


# ------------------------------------------------------------------
# paper('MAP') — translate(0,-70), rotate(-0.08), rect(-70,-90,
# 140,180), 'MAP' 900 30px at (0,-50), 6 grey lines (i==2 shorter).
# Sub centred on the translate origin (0,-70): 400x400 units.
# ------------------------------------------------------------------
def paper_plate():
    sub = Image.new("RGBA", (800, 800), (0, 0, 0, 0))
    d = ImageDraw.Draw(sub)
    cx, cy = 400.0, 400.0  # the translate origin, in px
    d.rectangle([cx - 70 * RES, cy - 90 * RES, cx + 70 * RES, cy + 90 * RES],
                fill=(242, 237, 224, 255), outline=INK, width=int(5 * RES))
    d.text((cx, cy - 50 * RES), "MAP", font=F(BLACK, 30), fill=INK, anchor="mm")
    for i in range(6):
        wdt = 60 if i == 2 else 112
        y = cy + (-26 + i * 18) * RES
        d.rectangle([cx - 56 * RES, y, cx + (-56 + wdt) * RES, y + 7 * RES], fill=(155, 149, 138, 255))
    sub = canvas_rotate(sub, -0.08)
    # plate top-left in hand frame: centre (0,-70) - (200,200)
    return sub, -200.0, -270.0


# ------------------------------------------------------------------
# leafletProp — translate(0,-58), rotate(-0.1), rect(-62,-82,124,
# 164), 'WAR IS' (0,-38) / 'OVER' (0,-6) 900 25px, 4 grey lines.
# ------------------------------------------------------------------
def leaflet_plate():
    sub = Image.new("RGBA", (800, 800), (0, 0, 0, 0))
    d = ImageDraw.Draw(sub)
    cx, cy = 400.0, 400.0
    d.rectangle([cx - 62 * RES, cy - 82 * RES, cx + 62 * RES, cy + 82 * RES],
                fill=(242, 237, 224, 255), outline=INK, width=int(5 * RES))
    f = F(BLACK, 25)
    d.text((cx, cy - 38 * RES), "WAR IS", font=f, fill=INK, anchor="mm")
    d.text((cx, cy - 6 * RES), "OVER", font=f, fill=INK, anchor="mm")
    for i in range(4):
        y = cy + (18 + i * 17) * RES
        d.rectangle([cx - 46 * RES, y, cx + 46 * RES, y + 6 * RES], fill=(155, 149, 138, 255))
    sub = canvas_rotate(sub, -0.1)
    return sub, -200.0, -258.0


# ------------------------------------------------------------------
# book — translate(0,-70), no rotation. Plate 340x360 units,
# top-left hand frame (-170,-180); content drawn in hand frame
# (the -70 lift applied to the geometry below).
# ------------------------------------------------------------------
def book_plate(open_book: bool):
    img = Image.new("RGBA", (680, 720), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    ox, oy = 170 * RES, 180 * RES  # hand point in px
    if open_book:
        # pages rect(-110,-70,220,140) in the translated frame
        d.rectangle([ox - 110 * RES, oy - 140 * RES, ox + 110 * RES, oy + 0 * RES],
                    fill=(244, 239, 228, 255), outline=INK, width=int(6 * RES))
        d.line([ox, oy - 140 * RES, ox, oy], fill=INK, width=int(6 * RES))
        for i in range(6):
            y = oy + (-120 + i * 20) * RES
            d.rectangle([ox - 96 * RES, y, ox - 14 * RES, y + 6 * RES], fill=(155, 149, 138, 255))
            d.rectangle([ox + 14 * RES, y, ox + 96 * RES, y + 6 * RES], fill=(155, 149, 138, 255))
    else:
        # roundRect(-80,-100,160,200,8) in the translated frame
        d.rounded_rectangle([ox - 80 * RES, oy - 170 * RES, ox + 80 * RES, oy + 30 * RES],
                            radius=8 * RES, fill=(138, 46, 46, 255), outline=INK, width=int(6 * RES))
        d.text((ox, oy - 90 * RES), "LEDGER", font=F(BLACK, 30), fill=(226, 179, 60, 255), anchor="mm")
        d.rectangle([ox - 50 * RES, oy - 60 * RES, ox + 50 * RES, oy - 54 * RES], fill=(226, 179, 60, 255))
    return img, -170.0, -180.0


# ------------------------------------------------------------------
# coconutProp — translate(0,-8): circle r36 at (0,-8) hand frame,
# dots, stem quad. Plate 200x200 units, top-left (-100,-108).
# ------------------------------------------------------------------
def coconut_plate():
    img = Image.new("RGBA", (400, 400), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    ox, oy = 100 * RES, 100 * RES  # the circle centre (hand + (0,-8))
    d.ellipse([ox - 36 * RES, oy - 36 * RES, ox + 36 * RES, oy + 36 * RES],
              fill=(107, 74, 46, 255), outline=INK, width=int(6 * RES))
    for dx, dy in [(-11, -12), (10, -14), (0, -2)]:
        d.ellipse([ox + (dx - 5) * RES, oy + (dy - 5) * RES, ox + (dx + 5) * RES, oy + (dy + 5) * RES],
                  fill=(84, 56, 32, 255))
    pts = []
    for i in range(21):
        u = i / 20
        x = (1 - u) ** 2 * 2 + 2 * (1 - u) * u * 16 + u * u * 34
        y = (1 - u) ** 2 * -34 + 2 * (1 - u) * u * -52 + u * u * -50
        pts.append((ox + x * RES, oy + y * RES))
    d.line(pts, fill=(135, 168, 96, 255), width=int(7 * RES))
    return img, -100.0, -108.0


PLATES = {}
for name, (img, x0, y0) in {
    "paper-map": paper_plate(),
    "leaflet": leaflet_plate(),
    "book-open": book_plate(True),
    "book-closed": book_plate(False),
    "coconut": coconut_plate(),
}.items():
    img.save(ASSETS / f"{name}.png")
    PLATES[name] = [x0, y0]
    print(f"plate {name}: {img.width}x{img.height} origin ({x0}, {y0})")
(ASSETS / "plates.json").write_text(json.dumps(PLATES, indent=1))

# ------------------------------------------------------------------
# Caption strips: 1080x300 frame-space, text centred (540, 150),
# shadow (3,9) rgba(0,0,0,0.45) fill-only, stroke #0B0B10,
# fill white — captions.ts captionText at scale 1 (the fit and
# the pop-in scale are applied at composite time).
# ------------------------------------------------------------------
capfont = ImageFont.truetype(BLACK, 88)
for c in DATA["captions"]:
    img = Image.new("RGBA", (1080, 300), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    d.text((543, 159), c["text"], font=capfont, fill=(0, 0, 0, 115), anchor="mm")
    d.text((540, 150), c["text"], font=capfont, fill=(255, 255, 255, 255),
           stroke_width=8, stroke_fill=(11, 11, 16, 255), anchor="mm")
    img.save(ASSETS / c["strip"])
print(f"caption strips: {len(DATA['captions'])}")

# ------------------------------------------------------------------
# Calendar cards (S07): drawCalendar geometry at 2x. Local frame
# (the ctx transform's local space): card roundRect(-160,-200,
# 320,400,14) + shadow roundRect(-152,-190,320,400,14).
# Plate covers local x [-180,180], y [-220,220] at 2x.
# ------------------------------------------------------------------
def calendar_plate(year: str):
    S = 2.0
    img = Image.new("RGBA", (720, 880), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    ox, oy = 180 * S, 220 * S
    d.rounded_rectangle([ox - 152 * S, oy - 190 * S, ox + 168 * S, oy + 210 * S], radius=14 * S, fill=(20, 16, 10, 89))
    d.rounded_rectangle([ox - 160 * S, oy - 200 * S, ox + 160 * S, oy + 200 * S], radius=14 * S,
                        fill=(244, 235, 214, 255), outline=INK, width=int(7 * S))
    d.rounded_rectangle([ox - 160 * S, oy - 200 * S, ox + 160 * S, oy - 134 * S], radius=14 * S, fill=(138, 48, 56, 255))
    d.rectangle([ox - 160 * S, oy - 148 * S, ox + 160 * S, oy - 134 * S], fill=(138, 48, 56, 255))
    d.text((ox, oy - 166 * S), "C A L E N D A R", font=ImageFont.truetype(XBD, int(30 * S)), fill=(244, 235, 214, 255), anchor="mm")
    d.text((ox, oy + 8 * S), year, font=ImageFont.truetype(BLACK, int(118 * S)), fill=INK, anchor="mm")
    d.text((ox, oy + 118 * S), "LUBANG ISLAND", font=ImageFont.truetype(XBD, int(25 * S)), fill=(139, 131, 120, 255), anchor="mm")
    d.rectangle([ox - 110 * S, oy + 158 * S, ox + 110 * S, oy + 163 * S], fill=(139, 131, 120, 255))  # D19: source rule colour #8B8378 (drawCalendar), not INK
    return img


for y in ["1945", "1955", "1965", "1974"]:
    calendar_plate(y).save(ASSETS / f"calendar-{y}.png")

# ------------------------------------------------------------------
# BK sign board texture (world.ts signBoard): 200 px per unit,
# 2.2x0.4 units -> 440x80 px; bg #3A2A2A, border fg #F2E6CF
# inset 10 lw 6, 'TANIGUCHI BOOKS' in 700 Caladea fitted to 86%
# width (the canvas measure loop, with PIL metrics).
# ------------------------------------------------------------------
CALADEA = str(PROJ / "fonts/Caladea-Bold.ttf")
img = Image.new("RGBA", (440, 80), (58, 42, 42, 255))
d = ImageDraw.Draw(img)
d.rectangle([10, 10, 430, 70], outline=(242, 230, 207, 255), width=6)
fs = 40
while fs > 8:
    fnt = ImageFont.truetype(CALADEA, fs)
    if d.textlength("TANIGUCHI BOOKS", font=fnt) <= 440 * 0.86:
        break
    fs -= 2
d.text((220, 43), "TANIGUCHI BOOKS", font=fnt, fill=(242, 230, 207, 255), anchor="mm")
img.save(ASSETS / "sign.png")
print(f"sign texture: font size {fs}")
img = Image.new("RGBA", (720, 880), (0, 0, 0, 0))
d = ImageDraw.Draw(img)
d.rounded_rectangle([40, 40, 680, 840], radius=28, fill=(30, 25, 18, 255))
img.save(ASSETS / "calendar-dark.png")
print("calendar plates: 4 years + dark")
