#!/usr/bin/env python3
"""Generate every caption strip for the Onoda film, natively (PIL).

Chunk timing is a faithful port of the project's own caption builder,
`src/core/captions.ts` `buildCaptions()` (project:
~/faceless-shorts/the-soldier-who-didnt-know):

  * words come from data/words.json, line ends + film duration from
    data/lines.json (the timing module's T.lines / T.dur);
  * a script line's authored `caps` field ("a | b | c") chooses the
    chunks: chunk j starts at the first word after the previous
    chunk's start whose normalised form equals the chunk's first
    normalised word (chunk 0 starts at word 0); chunk start
    s = word.s - 0.03;
  * chunk end: the next chunk's start, except across a line gap
    (next.s - lineEnd > 0.6 and next chunk on another line), where
    e = lineEnd + 0.25; the final chunk ends at T.dur;
  * lines without `caps` fall back to autoChunks() (short words ride
    along with the next word) — ported too, unused by this film.

Widths + fit follow the caption audit exactly (skill template
tools/capture.ts, printCaptionAudit; CAP from captions.ts):

  rawW      = measure of the chunk text in '900 88px Inter'
  fit       = min(1, CAP.maxW / rawW),        CAP.maxW = 960
  renderedW = rawW * fit
  WIDTH violation iff renderedW > 850;  FIT violation iff fit < 0.68

MEASUREMENT LIMIT (declared, not hidden): the audit's rawW is
Skia's measureText inside Chrome. Here rawW is PIL/FreeType's
getlength() against the SAME font file (the project's own
fonts/Inter-Black.otf, the 900 face). Chunk text and timing are
exact; only rawW — and therefore the fit of over-wide chunks — can
differ by a small factor between the two measurers. The run's
README quantifies the difference against the M6 first-party
metrics (docs/ab/telemetry/ab1-captions.json).

Strips mirror the proven §28 recipe
(docs/ab/onoda-s01-t150/scripts/make_caption.py), which reproduces
captionText()'s three passes: shadow silhouette rgba(0,0,0,0.45)
at (+3, +9), strokeText lineWidth 15 in #0B0B10 with round joins,
fillText white — text centred in a 1080x300 strip whose middle
lands on frame y = CAP.y (1340) when composited post-grade. The
strip carries the SETTLED caption (scale = fit); the 0.1 s pop-in
scale is a draw-time factor in drawCaption(), not part of the
strip. Rendered at 3x supersample, LANCZOS downscale.

Deterministic: same inputs -> same manifest, same PNG bytes.
No randomness, no clock reads.

Usage:
  gen_caption_strips.py [--project DIR] [--out DIR] [--font FILE]
Defaults: project ~/faceless-shorts/the-soldier-who-didnt-know,
out <repo>/assets/captions, font <project>/fonts/Inter-Black.otf.
Exit 2 with a one-line ERROR on stderr when the project data, font,
or output dir is unreadable, invalid, or missing a required field.
"""
import argparse
import json
import re
import sys
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

# CAP metrics — captions.ts: export const CAP = { y: 1340, size: 88, maxW: 960 }
CAP_Y, CAP_SIZE, CAP_MAXW = 1340, 88, 960
# Audit floors — template tools/capture.ts printCaptionAudit
WIDTH_FLOOR, FIT_FLOOR = 850.0, 0.68
# Strip geometry — §28 make_caption.py (strip middle lands on CAP_Y)
FRAME_W, STRIP_H = 1080, 300
SS = 3  # supersample factor for antialiasing

SHORT = re.compile(
    r"^(a|an|the|to|of|in|on|at|by|and|but|or|so|is|it|he|she|we|i|his|her|my|no|up|for|was)$",
    re.IGNORECASE,
)


def norm(w: str) -> str:
    """timing.ts/captions.ts nm(): lowercase, strip [^a-z0-9'] runs."""
    return re.sub(r"[^a-z0-9']+", "", str(w).lower())


def auto_chunks(ws):
    """captions.ts autoChunks(): [a, b] word-index spans per chunk."""
    out = []
    i = 0
    while i < len(ws):
        w = ws[i]
        n = ws[i + 1] if i + 1 < len(ws) else None
        bare = re.sub(r"[^A-Za-z']", "", w)
        if (
            n is not None
            and SHORT.match(bare)
            and not re.search(r"[,.!?;:]$", w)
            and len(w + n) <= 11
        ):
            out.append((i, i + 1))
            i += 2
        else:
            out.append((i, i))
            i += 1
    return out


def build_captions(script, lines_data, words):
    """Port of buildCaptions(). Returns [{line, s, e, text}]."""
    line_end = {ln["i"]: ln["end"] for ln in lines_data["lines"]}
    dur = lines_data["duration"]
    by_line = {}
    for w in words:
        by_line.setdefault(w["line"], []).append(w)

    caps = []
    for i, ln in enumerate(script["lines"]):
        ws = by_line.get(i, [])
        if not ws:
            continue
        text_words = ln["text"].split()
        spoken = text_words if len(ws) == len(text_words) else [w["w"] for w in ws]
        authored = ln.get("caps")
        if authored:
            frm = 0
            segs = [c.strip() for c in authored.split("|")]
            segs = [c for c in segs if c]
            for j, c in enumerate(segs):
                first = norm(c.split()[0])
                if j == 0:
                    idx = 0
                else:
                    idx = next(
                        (k for k in range(frm + 1, len(ws)) if norm(ws[k]["w"]) == first),
                        -1,
                    )
                    if idx < 0:
                        idx = min(len(ws) - 1, frm + 1)
                frm = idx
                caps.append(
                    {"line": i, "s": ws[idx]["s"] - 0.03, "e": 0.0, "text": c.replace("*", "")}
                )
        else:
            for a, b in auto_chunks(spoken):
                caps.append(
                    {
                        "line": i,
                        "s": ws[a]["s"] - 0.03,
                        "e": 0.0,
                        "text": " ".join(spoken[a : b + 1]),
                    }
                )
    for k, c in enumerate(caps):
        nxt = caps[k + 1] if k + 1 < len(caps) else None
        if nxt is None:
            c["e"] = dur
        elif nxt["s"] - line_end[c["line"]] > 0.6 and nxt["line"] != c["line"]:
            c["e"] = line_end[c["line"]] + 0.25
        else:
            c["e"] = nxt["s"]
    return caps


def render_strip(text, fit, font_path, out_path):
    """captionText()'s three passes, settled scale (= fit), PIL version."""
    font = ImageFont.truetype(font_path, round(CAP_SIZE * SS * fit))
    img = Image.new("RGBA", (FRAME_W * SS, STRIP_H * SS), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    cx, cy = (FRAME_W // 2) * SS, (STRIP_H // 2) * SS
    # Pass 1: shadow silhouette rgba(0,0,0,0.45) at (+3, +9), fit-scaled units.
    d.text(
        (cx + 3 * SS * fit, cy + 9 * SS * fit),
        text,
        font=font,
        fill=(0, 0, 0, int(0.45 * 255)),
        anchor="mm",
    )
    # Passes 2+3: ink stroke (lineWidth 15, #0B0B10) under a white fill.
    d.text(
        (cx, cy),
        text,
        font=font,
        fill=(255, 255, 255, 255),
        stroke_width=round(15 * SS * fit),
        stroke_fill=(0x0B, 0x0B, 0x10, 255),
        anchor="mm",
    )
    img = img.resize((FRAME_W, STRIP_H), Image.LANCZOS)
    img.save(out_path)


class InputError(Exception):
    """Bad input data (unreadable / invalid / wrong-shaped project
    files, unusable font or output dir). Reported as one named line
    on stderr with exit 2 instead of a traceback."""


def load_json_input(path: Path, what: str):
    try:
        return json.loads(path.read_text())
    except OSError as e:
        raise InputError(f"cannot read {what} {path} ({e.strerror or e})")
    except ValueError as e:  # JSONDecodeError + UnicodeDecodeError
        raise InputError(f"invalid JSON in {what} {path} ({e})")


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    repo = Path(__file__).resolve().parents[2]
    ap.add_argument(
        "--project",
        default=str(Path.home() / "faceless-shorts" / "the-soldier-who-didnt-know"),
    )
    ap.add_argument("--out", default=str(repo / "assets" / "captions"))
    ap.add_argument("--font", default=None, help="default: <project>/fonts/Inter-Black.otf")
    args = ap.parse_args()

    project = Path(args.project)
    out_dir = Path(args.out)
    font_path = args.font or str(project / "fonts" / "Inter-Black.otf")
    try:
        out_dir.mkdir(parents=True, exist_ok=True)
    except OSError as e:
        raise InputError(f"cannot create output dir {out_dir} "
                         f"({e.strerror or e})")

    script = load_json_input(project / "script.json", "script")
    lines_data = load_json_input(project / "data" / "lines.json", "lines data")
    words = load_json_input(project / "data" / "words.json", "words data")
    if not isinstance(script, dict) or not isinstance(script.get("lines"), list):
        raise InputError(f"script {project / 'script.json'}: "
                         f"'lines' must be a list")
    if not isinstance(lines_data, dict) \
            or not isinstance(lines_data.get("lines"), list) \
            or "duration" not in lines_data:
        raise InputError(f"lines data {project / 'data' / 'lines.json'}: "
                         f"requires a 'lines' list and a 'duration'")
    if not isinstance(words, list):
        raise InputError(f"words data {project / 'data' / 'words.json'}: "
                         f"top level must be a list")

    try:
        chunks = build_captions(script, lines_data, words)
    except (KeyError, IndexError, TypeError) as e:
        raise InputError(f"project data is missing or mistypes a "
                         f"required field ({e!r})")
    if not chunks:
        raise InputError("no caption chunks built — data/words.json "
                         "yields no chunks for script.json's lines")
    try:
        measure_font = ImageFont.truetype(font_path, CAP_SIZE)
    except OSError as e:
        raise InputError(f"cannot open font {font_path} ({e})")

    entries = []
    for k, c in enumerate(chunks):
        raw_w = measure_font.getlength(c["text"])
        fit = min(1.0, CAP_MAXW / raw_w)
        rendered_w = raw_w * fit
        name = f"cap-{k:03d}.png"
        render_strip(c["text"], fit, font_path, out_dir / name)
        entries.append(
            {
                "id": f"cap-{k:03d}",
                "text": c["text"],
                "s": round(c["s"], 3),
                "e": round(c["e"], 3),
                "line": c["line"],
                "rawW": round(raw_w, 2),
                "renderedW": round(rendered_w, 2),
                "fit": round(fit, 4),
                "file": name,
            }
        )

    manifest = {
        "film": script.get("title", ""),
        "source": {
            "script": "script.json (caps fields)",
            "words": "data/words.json",
            "lines": "data/lines.json",
            "font": "fonts/Inter-Black.otf (project's own 900 face)",
            "measurer": "PIL/FreeType getlength at 88px — NOT Skia measureText; see README",
        },
        "cap": {"y": CAP_Y, "size": CAP_SIZE, "maxW": CAP_MAXW},
        "floors": {"renderedW": WIDTH_FLOOR, "fit": FIT_FLOOR},
        "count": len(entries),
        "chunks": entries,
    }
    (out_dir / "manifest.json").write_text(json.dumps(manifest, indent=1) + "\n")

    min_fit = min(e["fit"] for e in entries)
    max_rw = max(e["renderedW"] for e in entries)
    wide = [e for e in entries if e["renderedW"] > WIDTH_FLOOR]
    low = [e for e in entries if e["fit"] < FIT_FLOOR]
    print(
        f"CAPTION STRIPS: {len(entries)} chunks, min fit {min_fit:.2f}, "
        f"max rendered width {max_rw:.0f} px, {len(wide)} over the 850 px floor, "
        f"{len(low)} under the 0.68 fit floor"
    )
    for e in wide:
        print(
            f"  WIDTH: line {e['line'] + 1} chunk \"{e['text']}\" renders "
            f"{e['renderedW']:.0f} px (> 850) at t={e['s']:.2f}s"
        )
    for e in low:
        print(
            f"  FIT: line {e['line'] + 1} chunk \"{e['text']}\" fit {e['fit']:.2f} "
            f"(< 0.68; raw width {e['rawW']:.0f} px) at t={e['s']:.2f}s"
        )
    print(f"wrote {out_dir / 'manifest.json'} + {len(entries)} strips to {out_dir}")
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except InputError as e:
        print(f"ERROR: gen_caption_strips.py: {e}", file=sys.stderr)
        sys.exit(2)
