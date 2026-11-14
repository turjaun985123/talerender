#!/usr/bin/env python3
"""Generate film/onoda.json from the Onoda project's own data files.

Everything here is a transcription of the project's timing layer
(src/core/timing.ts + src/core/captions.ts buildCaptions) over its
data/lines.json, data/words.json and script.json — no hand-typed
timings. Caption widths are measured with PIL against the
project's own fonts/Inter-Black.otf (the font file the Chrome
canvas measures with; PIL metrics are not Skia's — the fit of an
over-wide chunk can differ by a small factor, declared in
docs/integration/README.md).

Usage: python3 film/gen_film_data.py [project-dir] [out.json]
"""
import json
import re
import sys
from pathlib import Path

from PIL import ImageFont

PROJ = Path(sys.argv[1]) if len(sys.argv) > 1 else Path.home() / "faceless-shorts/the-soldier-who-didnt-know"
OUT = Path(sys.argv[2]) if len(sys.argv) > 2 else Path(__file__).parent / "onoda.json"

lines_data = json.loads((PROJ / "data/lines.json").read_text())
words = json.loads((PROJ / "data/words.json").read_text())
script = json.loads((PROJ / "script.json").read_text())
T_DUR = lines_data["duration"]
LINES = lines_data["lines"]


def norm(w: str) -> str:
    return re.sub(r"[^a-z0-9']+", "", str(w).lower())


def wd(word: str, line: int) -> float:
    """timing.ts Wd(): start of the first word in `line` whose
    normalised form starts with the normalised query."""
    n = norm(word)
    for w in words:
        if w["line"] == line and norm(w["w"]).startswith(n):
            return w["s"]
    raise KeyError(f"word {word!r} not in line {line}")


ANCHOR_QUERIES = [
    ("years", 0), ("never", 2), ("surrender", 2), ("message", 3),
    ("thought", 4), ("trick", 4), ("decades", 6), ("coconuts", 6),
    ("found", 7), ("refused", 7), ("only", 8), ("commander", 8),
    ("bookseller", 9), ("flew", 9), ("order", 10), ("surrendered", 10),
    ("loyal", 11),
]
anchors = {f"{q}{l}": wd(q, l) for q, l in ANCHOR_QUERIES}

# ---- captions: buildCaptions() transcribed ----
font = ImageFont.truetype(str(PROJ / "fonts/Inter-Black.otf"), 88)
caps = []
for i, ln in enumerate(script["lines"]):
    ws = [w for w in words if w["line"] == i]
    if not ws:
        continue
    if ln.get("caps"):
        frm = 0
        for j, c in enumerate([c.strip() for c in ln["caps"].split("|") if c.strip()]):
            first = norm(c.split()[0])
            if j == 0:
                idx = 0
            else:
                idx = next((k for k, w in enumerate(ws) if k > frm and norm(w["w"]) == first), None)
                if idx is None:
                    idx = min(len(ws) - 1, frm + 1)
            frm = idx
            caps.append({"line": i, "s": ws[idx]["s"] - 0.03, "e": 0.0, "text": c.replace("*", "")})
    else:
        raise SystemExit("this film's script always sets caps; autoChunks not transcribed")
for k, c in enumerate(caps):
    nxt = caps[k + 1] if k + 1 < len(caps) else None
    line_end = LINES[c["line"]]["end"]
    if nxt is None:
        c["e"] = T_DUR
    elif nxt["s"] - line_end > 0.6 and nxt["line"] != c["line"]:
        c["e"] = line_end + 0.25
    else:
        c["e"] = nxt["s"]
    w = font.getlength(c["text"])
    c["width"] = round(w, 2)
    c["fit"] = round(min(1.0, 960.0 / w), 5)
    c["strip"] = f"cap_{k:03d}.png"

out = {
    "film": "THE SOLDIER WHO DIDN'T KNOW THE WAR WAS OVER",
    "project": str(PROJ),
    "duration": T_DUR,
    "fps": 30,
    "frames": int(T_DUR * 30) + 1,
    "lines": [{"i": l["i"], "start": l["start"], "end": l["end"], "text": l["text"]} for l in LINES],
    "anchors": anchors,
    "captions": caps,
    # F2 migration (W53 s1.2): the calendar set, dark plate and
    # sign are declared by the film data, not hardcoded in the
    # driver. Values mirror film/gen_assets.py (its calendar
    # loop, sign and dark-plate writers).
    "calendarYears": ["1945", "1955", "1965", "1974"],
    "calendarDark": "calendar-dark.png",
    "sign": "sign.png",
}
OUT.write_text(json.dumps(out, indent=1))
print(f"wrote {OUT}: {len(caps)} caption chunks, {len(anchors)} anchors, duration {T_DUR}")
