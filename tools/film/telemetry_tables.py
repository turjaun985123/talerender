#!/usr/bin/env python3
"""telemetry_tables.py — PROTOTYPE (Wave 2 seat W75, G7): the three
review tables the film telemetry verdict does not print.

Reads the real film data (film data file + scenes/film/film.json +
the per-shot scene files) and prints, as fixed-width plain text:

  1. SHOT TABLE    — shot, frame range, time range, duration, set,
                     atmosphere, characters present.
  2. CAPTION TABLE — every caption chunk: index, line, shot, window,
                     fit, rendered width, raw width, audit flag.
  3. CAST LIST     — every character identity: shots present, frame
                     count, authored blink beats.

This is a design prototype for TELEMETRY-TABLES-W75.md. It is NOT
wired into the binary and changes no verdict: the binary's
`FILM TELEMETRY: CHECK OK / CHECK FAILED` line stays the only gate.

Caption width parity with the binary (the load-bearing claim):
run_film_telemetry (src/film.rs) loads each caption's strip PNG via
load_png (image crate -> RGBA8) and measures the INK BOUNDING BOX —
columns containing any pixel with alpha > 32; rendered width =
x1 - x0 + 1 in strip pixels; raw width = rendered / fit
(src/film.rs, caption audit block). print_caption_audit
(src/telemetry.rs) fails a chunk when rendered_w > 850.0 (WIDTH) or
fit < 0.68 (FIT), strict mode. PNG is lossless and the strips are
8-bit RGBA, so the stdlib decoder below reproduces the binary's
pixels byte-for-byte; the only approximation is the one the binary
itself declares (ink bbox of the pre-rendered strip, "PIL metrics —
the declared approximation of docs/integration/README.md").

Authored blink beats are NOT in the data files as bindings: the
beat TIMES are word anchors in the film data's `anchors`, but the
anchor -> (character, shot) binding lives in the driver's staging
functions. BEAT_BINDINGS below transcribes those bindings from
src/film.rs `blink_of` call sites (cited per row); the tool
verifies each beat lands inside its shot's window and on a
character present in that shot, and fails reconciliation if not.

Exit codes: 0 = tables printed, all reconciliation checks passed;
1 = reconciliation failure (tables still printed, failures listed);
2 = usage / IO / parse error.
"""

import json
import struct
import sys
import zlib
from pathlib import Path

WIDTH_BAR = 850.0   # src/telemetry.rs print_caption_audit: rendered_w > 850.0
FIT_BAR = 0.68      # src/telemetry.rs print_caption_audit: fit < 0.68
ALPHA_MIN = 32      # src/film.rs caption audit: ink iff alpha > 32

# anchor -> (character id, shot id, src/film.rs citation).
# Beat times are read live from the film data's `anchors`; only the
# binding is transcribed. Seeds at the call sites match the
# look_registry blink seeds (onoda 3, suzuki 5, tani 9).
BEAT_BINDINGS = {
    "years0":      ("onoda",  "S01",  "film.rs:1459 (s01, seed 3)"),
    "never2":      ("onoda",  "S03",  "film.rs:1528 via w_never=anchors[never2] film.rs:1511 (s03, seed 3)"),
    "message3":    ("onoda",  "S04",  "film.rs:1565 (s04, seed 3)"),
    "found7":      ("suzuki", "S08",  "film.rs:1803 via w_found=anchors[found7] film.rs:1785 (s08, seed 5)"),
    "commander8":  ("onoda",  "S09",  "film.rs:1870 via w_comm=anchors[commander8] film.rs:1855 (s09, seed 3)"),
    "bookseller9": ("tani",   "S10",  "film.rs:1911 via w_books=anchors[bookseller9] film.rs:1890 (s10, seed 9)"),
    "loyal11":     ("onoda",  "S12",  "film.rs:2028 via w_loyal=anchors[loyal11] film.rs:2005 (s12, seed 3)"),
}

# Scene files name the bookshop/shore character "taniguchi" (id and
# display name); the binary's continuity identity for the same
# person is "tani" (src/film.rs INSTANCE_IDS + look_registry
# instances jgTani/shTani/inTani). Canonicalise to the binary's id.
ID_ALIASES = {"taniguchi": "tani"}
CANONICAL_IDS = ["onoda", "compA", "compB", "suzuki", "tani"]


# ---------------------------------------------------------------------
# Stdlib PNG decoding (8-bit, non-interlaced; colour types 0/2/3/4/6)
# ---------------------------------------------------------------------

def decode_png_rgba(path):
    """Return (rgba_bytes, width, height) for an 8-bit PNG.

    Mirrors load_png's contract (any PNG -> RGBA8). Raises ValueError
    on anything outside the supported subset (16-bit, interlaced)
    rather than mismeasuring silently.
    """
    data = Path(path).read_bytes()
    if data[:8] != b"\x89PNG\r\n\x1a\n":
        raise ValueError(f"{path}: not a PNG")
    pos = 8
    idat = bytearray()
    palette = []
    trns = b""
    ihdr = None
    while pos < len(data):
        (length,) = struct.unpack(">I", data[pos:pos + 4])
        ctype = data[pos + 4:pos + 8]
        chunk = data[pos + 8:pos + 8 + length]
        if ctype == b"IHDR":
            ihdr = struct.unpack(">IIBBBBB", chunk)
        elif ctype == b"PLTE":
            palette = [tuple(chunk[i:i + 3]) for i in range(0, len(chunk), 3)]
        elif ctype == b"tRNS":
            trns = chunk
        elif ctype == b"IDAT":
            idat += chunk
        elif ctype == b"IEND":
            break
        pos += 12 + length
    if ihdr is None:
        raise ValueError(f"{path}: missing IHDR")
    w, h, depth, color, _comp, _filt, interlace = ihdr
    if depth != 8:
        raise ValueError(f"{path}: bit depth {depth} unsupported (strips are 8-bit)")
    if interlace != 0:
        raise ValueError(f"{path}: interlaced PNG unsupported by this prototype")
    channels = {0: 1, 2: 3, 3: 1, 4: 2, 6: 4}[color]
    raw = zlib.decompress(bytes(idat))
    stride = w * channels
    out = bytearray(w * h * 4)
    prev = bytearray(stride)
    p = 0
    for y in range(h):
        f = raw[p]
        p += 1
        line = bytearray(raw[p:p + stride])
        p += stride
        if f == 1:      # Sub
            for i in range(channels, stride):
                line[i] = (line[i] + line[i - channels]) & 0xFF
        elif f == 2:    # Up
            for i in range(stride):
                line[i] = (line[i] + prev[i]) & 0xFF
        elif f == 3:    # Average
            for i in range(stride):
                a = line[i - channels] if i >= channels else 0
                line[i] = (line[i] + ((a + prev[i]) >> 1)) & 0xFF
        elif f == 4:    # Paeth
            for i in range(stride):
                a = line[i - channels] if i >= channels else 0
                b = prev[i]
                c = prev[i - channels] if i >= channels else 0
                pa, pb, pc = abs(b - c), abs(a - c), abs(a + b - 2 * c)
                pr = a if (pa <= pb and pa <= pc) else (b if pb <= pc else c)
                line[i] = (line[i] + pr) & 0xFF
        elif f != 0:    # None
            raise ValueError(f"{path}: unknown filter {f}")
        prev = line
        base = y * w * 4
        if color == 6:
            out[base:base + w * 4] = line
        elif color == 2:
            for x in range(w):
                o, i = base + x * 4, x * 3
                out[o:o + 3] = line[i:i + 3]
                out[o + 3] = 255
        elif color == 4:
            for x in range(w):
                o, i = base + x * 4, x * 2
                out[o] = out[o + 1] = out[o + 2] = line[i]
                out[o + 3] = line[i + 1]
        elif color == 0:
            for x in range(w):
                o = base + x * 4
                out[o] = out[o + 1] = out[o + 2] = line[x]
                out[o + 3] = 255
        elif color == 3:
            for x in range(w):
                o = base + x * 4
                r, g, b = palette[line[x]]
                out[o], out[o + 1], out[o + 2] = r, g, b
                out[o + 3] = trns[line[x]] if line[x] < len(trns) else 255
    return bytes(out), w, h


_INK_TABLE = bytes(1 if b > ALPHA_MIN else 0 for b in range(256))


def ink_width(rgba, w, h):
    """Ink bbox width exactly as src/film.rs: columns holding any
    pixel with alpha > 32; width = x1 - x0 + 1; 0 when no ink.

    The alpha plane is thresholded with a translate table (C speed)
    and each row's first/last ink column found with bytes.find —
    the result is identical to the per-pixel scan."""
    alpha = rgba[3::4]
    bits = alpha.translate(_INK_TABLE)
    x0, x1 = None, -1
    for y in range(h):
        row = bits[y * w:(y + 1) * w]
        f = row.find(b"\x01")
        if f >= 0:
            if x0 is None or f < x0:
                x0 = f
            r = row.rfind(b"\x01")
            if r > x1:
                x1 = r
    return (x1 - x0 + 1) if x1 >= 0 else 0


# ---------------------------------------------------------------------
# Data loading
# ---------------------------------------------------------------------

def canonical_id(node):
    """Character identity for a scene-file character node: explicit
    `id` wins; otherwise the display name minus any parenthetical,
    matched case-insensitively against the known registry; then the
    alias map (taniguchi -> tani). Returns None when unrecognised."""
    raw = node.get("id")
    if not raw:
        name = node.get("name", "").split("(")[0].strip()
        raw = name
        for cand in CANONICAL_IDS + list(ID_ALIASES):
            if name.lower() == cand.lower():
                raw = cand
                break
    return ID_ALIASES.get(raw, raw)


def load_all(film_dir, film_json_path):
    film_dir = Path(film_dir)
    # F1 rule, same as load_film_data: film.json wins, onoda.json is
    # the fallback name.
    data_path = film_dir / "film.json"
    if not data_path.exists():
        data_path = film_dir / "onoda.json"
    data = json.loads(data_path.read_text())
    film = json.loads(Path(film_json_path).read_text())
    repo_root = Path(film_json_path).resolve().parent.parent.parent
    segments = []
    for seg in film["segments"]:
        scene = json.loads((repo_root / seg["scene"]).read_text())
        chars = []
        instances = {}
        for node in scene.get("nodes", []):
            if node.get("type") != "character":
                continue
            cid = canonical_id(node)
            instances[cid] = instances.get(cid, 0) + 1
            if cid not in chars:
                chars.append(cid)
        segments.append({**seg, "chars": chars, "instances": instances})
    return data, film, segments


# ---------------------------------------------------------------------
# Tables
# ---------------------------------------------------------------------

def fmt_t(x):
    return f"{x:.2f}"


def shot_table(film, segments):
    lines = []
    lines.append(f"SHOT TABLE — {len(segments)} shots, {film['totalFrames']} frames "
                 f"@ {film['fps']} fps, {fmt_t(film['durationS'])} s")
    lines.append(f"{'shot':<6}{'frames':<13}{'time (s)':<16}{'dur':>6}  "
                 f"{'set':<5}{'atmosphere':<11}characters")
    for seg in segments:
        fr = f"{seg['startFrame']}-{seg['endFrame'] - 1}"
        tm = f"{fmt_t(seg['startS'])}-{fmt_t(seg['endS'])}"
        chars = []
        for cid in seg["chars"]:
            n = seg["instances"].get(cid, 1)
            chars.append(cid if n == 1 else f"{cid} x{n}")
        lines.append(f"{seg['id'].upper():<6}{fr:<13}{tm:<16}"
                     f"{seg['durationS']:>6.2f}  {seg['set']:<5}"
                     f"{seg['atmosphere']:<11}{', '.join(chars) if chars else '(none)'}")
    return lines


def caption_rows(data, film, segments, film_dir):
    """One row per chunk + the shot assignment cross-check.

    Shot assignment is primarily film.json's captionChunks tiling
    (first..last inclusive per segment); it is verified against the
    rule the tiling itself encodes — the segment containing the
    chunk's START owns it (the driver draws a caption whenever
    t in [s, e), across a cut if the window straddles one; straddles
    are reported as notes, not failures). Line assignment recomputes
    the binary's rule (src/film.rs): the last line whose start
    <= chunk start + 0.05 (0 when none)."""
    caps = data["captions"]
    lines_data = data["lines"]
    owner = {}
    for seg in segments:
        cc = seg["captionChunks"]
        for i in range(cc["first"], cc["last"] + 1):
            owner[i] = seg
    rows = []
    problems = []
    straddles = []
    if sorted(owner) != list(range(len(caps))):
        problems.append(f"captionChunks tiling covers {len(owner)} chunks, "
                        f"data has {len(caps)}")
    for i, c in enumerate(caps):
        seg = owner.get(i)
        if seg is not None:
            if not (seg["startS"] <= c["s"] < seg["endS"] or
                    (seg is segments[-1] and c["s"] <= seg["endS"])):
                problems.append(f"chunk {i}: start {c['s']:.2f} outside "
                                f"{seg['id']} [{seg['startS']}, {seg['endS']}) per tiling")
            if c["e"] > seg["endS"]:
                straddles.append((i, seg["id"].upper(), c["e"] - seg["endS"]))
        line_idx = 0
        for j, ln in enumerate(lines_data):
            if ln["start"] <= c["s"] + 0.05:
                line_idx = j
        if c.get("line") is not None and c["line"] != line_idx:
            problems.append(f"chunk {i}: stored line {c['line']} != recomputed "
                            f"{line_idx} (binary ignores the stored field)")
        strip_path = Path(film_dir) / "assets" / c["strip"]
        rgba, w, h = decode_png_rgba(strip_path)
        rendered = float(ink_width(rgba, w, h))
        raw = rendered / c["fit"] if c["fit"] > 0 else rendered
        flags = []
        if rendered > WIDTH_BAR:
            flags.append("WIDTH")
        if c["fit"] < FIT_BAR:
            flags.append("FIT")
        rows.append({
            "i": i, "line": line_idx + 1,
            "shot": seg["id"].upper() if seg else "??",
            "s": c["s"], "e": c["e"], "fit": c["fit"],
            "rendered": rendered, "raw": raw,
            "flag": "+".join(flags), "text": c["text"],
        })
    return rows, problems, straddles


def caption_table(rows):
    lines = []
    lines.append(f"CAPTION TABLE — {len(rows)} chunks "
                 f"(audit bars: rendered > {WIDTH_BAR:.0f} px = WIDTH fail, "
                 f"fit < {FIT_BAR} = FIT fail; strict)")
    lines.append(f"{'idx':<5}{'line':<6}{'shot':<6}{'window (s)':<17}"
                 f"{'fit':>5}{'rend':>6}{'raw':>7}  {'flag':<10}text")
    for r in rows:
        win = f"{fmt_t(r['s'])}-{fmt_t(r['e'])}"
        lines.append(f"{r['i']:<5}{r['line']:<6}{r['shot']:<6}{win:<17}"
                     f"{r['fit']:>5.2f}{r['rendered']:>6.0f}{r['raw']:>7.0f}  "
                     f"{r['flag']:<10}\"{r['text']}\"")
    return lines


def cast_list(data, segments):
    order = []
    stats = {}
    for seg in segments:
        for cid in seg["chars"]:
            if cid not in stats:
                stats[cid] = {"shots": [], "frames": 0, "beats": []}
                order.append(cid)
            stats[cid]["shots"].append(seg["id"].upper())
            stats[cid]["frames"] += seg["frames"]
    seg_by_id = {s["id"].upper(): s for s in segments}
    problems = []
    for anchor, (cid, shot, _cite) in BEAT_BINDINGS.items():
        if anchor not in data["anchors"]:
            problems.append(f"beat anchor {anchor} missing from film data anchors")
            continue
        t = data["anchors"][anchor]
        seg = seg_by_id[shot]
        if not (seg["startS"] <= t < seg["endS"]):
            problems.append(f"beat {anchor} @ {t} outside {shot} "
                            f"[{seg['startS']}, {seg['endS']})")
        if cid not in seg["chars"]:
            problems.append(f"beat {anchor}: {cid} not present in {shot} per scene data")
        # Frame n is staged at t = n/fps (film.json frameRule); a beat
        # lights the blink while 0 <= t - beat < 0.12 (film.rs blink_at).
        fps = data["fps"]
        frames = [n for n in range(int(t * fps) - 1, int((t + 0.12) * fps) + 2)
                  if 0 <= n / fps - t < 0.12]
        stats[cid]["beats"].append((anchor, t, shot, len(frames)))
    lines = []
    lines.append(f"CAST LIST — {len(order)} identities "
                 f"(frame counts are shot-granularity: full frames of every "
                 f"shot the identity appears in)")
    lines.append(f"{'id':<9}{'shots':>6}{'frames':>8}{'beats':>7}  "
                 f"shot list / authored blink beats")
    for cid in order:
        st = stats[cid]
        beat_s = "; ".join(f"{a} @ {fmt_t(t)} ({s}, {n}f)"
                           for a, t, s, n in st["beats"]) or "-"
        lines.append(f"{cid:<9}{len(st['shots']):>6}{st['frames']:>8}"
                     f"{len(st['beats']):>7}  {', '.join(st['shots'])}")
        lines.append(f"{'':<9}{'':>6}{'':>8}{'':>7}  beats: {beat_s}")
    return lines, stats, problems


# ---------------------------------------------------------------------

def main(argv):
    film_dir = argv[1] if len(argv) > 1 else "film"
    film_json = argv[2] if len(argv) > 2 else "scenes/film/film.json"
    try:
        data, film, segments = load_all(film_dir, film_json)
        rows, cap_problems, straddles = caption_rows(data, film, segments, film_dir)
        cast_lines, _stats, cast_problems = cast_list(data, segments)
    except (OSError, ValueError, KeyError, json.JSONDecodeError) as e:
        print(f"telemetry_tables: {e}", file=sys.stderr)
        return 2

    problems = list(cap_problems) + list(cast_problems)
    total_frames = sum(s["frames"] for s in segments)
    if total_frames != film["totalFrames"] or total_frames != data["frames"]:
        problems.append(f"frame totals disagree: segments {total_frames}, "
                        f"film.json {film['totalFrames']}, data {data['frames']}")
    if len(rows) != len(data["captions"]):
        problems.append("caption row count != data caption count")
    n_flagged = sum(1 for r in rows if r["flag"])

    out = []
    out += shot_table(film, segments)
    out.append("")
    out += caption_table(rows)
    for i, sid, over in straddles:
        out.append(f"note: chunk {i} ({sid}) straddles the cut — its window "
                   f"runs {over:.2f} s into the next shot; the driver draws "
                   f"it across the cut (window rule, film.rs eval_frame)")
    out.append("")
    out += cast_lines
    out.append("")
    out.append(f"RECONCILIATION: {len(rows)} chunks ({n_flagged} flagged), "
               f"{total_frames} frames, {len(segments)} shots, "
               f"{len(BEAT_BINDINGS)} authored beats bound")
    if problems:
        for p in problems:
            out.append(f"RECONCILIATION FAIL: {p}")
    else:
        out.append("RECONCILIATION: all checks passed "
                   "(tiling == start assignment, stored line == recomputed line, "
                   "frame totals agree, beats in-window on present characters)")
    print("\n".join(out))
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
