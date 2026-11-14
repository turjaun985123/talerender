#!/usr/bin/env python3
"""make_sheet.py — contact sheet for a native frame directory.

The skill's most-used QA surface is the contact sheet (`capture sheet`:
frames sampled across the film, scaled down, tiled into one JPG by
ffmpeg's tile= filter). Native renders are loose PNGs
(frame_%05d.png, the finish.sh input), so this tool gives them the
same surface: pick the sampled frames out of a frame dir and tile
them with the skill's own ffmpeg recipe.

WHAT IT DOES NOT DO: the Chrome sheet re-runs the overflow /
composition audit at its sample times and burns a time label into
each tile; this tool tiles native pixels only — no overflow, edge,
continuity or composition audit, no labels, no guides. Its only QA
act is completeness: a sampled frame that is absent FAILS the run
and is named (the analogue of the sheet audit's INCOMPLETE rule —
an incomplete sheet must never look complete). Non-sampled holes in
the dir are reported as NOTEs, never silently skipped either.

FRAME SELECTION — the driver's own arithmetic, shared with
rerender_plan.py:

  * fps is 30 (src/film.rs hardcodes 30; frame n is evaluated at
    t = n / 30.0 and is on screen for [n/30, (n+1)/30)).
  * Sample times are t = step, 2*step, 3*step, ... while t <
    duration, where duration = total_frames / 30 and total_frames
    is one past the highest frame index present in the dir.
    (The Chrome sheet samples mid-step, t = step/2 + k*step; this
    tool samples on-step, matching the film tooling's convention —
    the offset difference is stated, not hidden.)
  * The frame at time t is n = floor(t * 30), snapping to the
    nearest integer when t * 30 is within 1e-6 of it (the planner's
    boundary-exact tolerance: a 1-ulp float difference must not
    flip a boundary frame). Duplicate n from a sub-frame step are
    de-duplicated, in time order.
  * --from / --to (seconds) restrict the window: samples with
    from <= t < to are kept (to defaults to duration).

TILING — the skill's recipe (capture.ts sheet mode, via
docs/audit/a2-capture.md §3.4): sampled frames are staged as an
f%04d.png sequence in a temp dir, then
    ffmpeg -framerate 1 -i f%04d.png -vf
      "scale=<W>:<H>,tile=<cols>x<rows>:padding=4:color=black"
      -frames:v 1 -q:v 3 <out>
rows = ceil(samples / cols); the tile filter pads an incomplete
final row with the padding colour, exactly as in Chrome mode.
ffmpeg spawn failure (not on PATH / cannot exec) and ffmpeg exit
failure are checked separately and reported separately, as the
skill's spawnSync code does.

Usage:
    python3 tools/film/make_sheet.py FRAMES_DIR OUT.jpg
        [--step 0.5] [--cols 9] [--scale 216x384]
        [--from 0] [--to SECONDS]

Exit code: 0 = sheet written, 1 = a sampled frame is missing or
ffmpeg failed (no sheet, or a partial sheet is removed), 2 = input
error (bad arguments, unreadable dir, no frames, ffmpeg absent —
one-line ERROR on stderr).
"""

import argparse
import math
import os
import re
import shutil
import struct
import subprocess
import sys
import tempfile

FPS = 30                      # src/film.rs hardcodes 30
FRAME_RE = re.compile(r"^frame_(\d+)\.png$")
BOUNDARY_EPS = 1e-6           # rerender_plan.py's boundary tolerance
DEFAULT_STEP = 0.5            # capture.ts sheet default
DEFAULT_COLS = 9              # capture.ts sheet default
DEFAULT_SCALE = "216x384"     # capture.ts sheet tile size
TILE_PADDING = 4              # capture.ts tile= padding, colour black


def fail2(msg):
    print(f"ERROR: {msg}", file=sys.stderr)
    return 2


def frame_at(t):
    """Frame on screen at time t, by the driver's arithmetic."""
    x = t * FPS
    r = round(x)
    if abs(x - r) < BOUNDARY_EPS:
        return int(r)
    return int(math.floor(x))


def png_size(path):
    """(w, h) from a PNG IHDR, stdlib only; None if not a PNG."""
    try:
        with open(path, "rb") as f:
            head = f.read(24)
    except OSError:
        return None
    if len(head) < 24 or head[:8] != b"\x89PNG\r\n\x1a\n":
        return None
    return struct.unpack(">II", head[16:24])


def parse_scale(text, src_size):
    """'WxH' or 'W' (height from the source aspect, rounded even)."""
    if "x" in text:
        w, h = text.split("x", 1)
        return int(w), int(h)
    w = int(text)
    if src_size is None:
        raise ValueError("--scale W needs a readable first frame "
                         "to derive the tile height from")
    sw, sh = src_size
    return w, max(2, int(round(w * sh / sw)) // 2 * 2)


def main(argv=None):
    ap = argparse.ArgumentParser(
        description="Tile sampled frames from a native frame dir "
                    "into one contact-sheet JPG (the skill's ffmpeg "
                    "tile recipe).")
    ap.add_argument("frames_dir")
    ap.add_argument("out")
    ap.add_argument("--step", type=float, default=DEFAULT_STEP,
                    help="sample spacing in seconds (default 0.5, "
                         "the capture sheet default)")
    ap.add_argument("--cols", type=int, default=DEFAULT_COLS,
                    help="tile columns (default 9, the capture "
                         "sheet default)")
    ap.add_argument("--scale", default=DEFAULT_SCALE,
                    help="tile size WxH, or W alone to keep the "
                         "source aspect (default 216x384, the "
                         "capture sheet tile size)")
    ap.add_argument("--from", dest="from_s", type=float, default=0.0,
                    help="only sample at t >= FROM seconds")
    ap.add_argument("--to", dest="to_s", type=float, default=None,
                    help="only sample at t < TO seconds "
                         "(default: the film's duration)")
    args = ap.parse_args(argv)

    ffmpeg = shutil.which("ffmpeg")
    if ffmpeg is None:
        return fail2("ffmpeg not found on PATH — the sheet tiles "
                     "with ffmpeg's tile= filter (as capture sheet "
                     "does); install ffmpeg or run where it exists")

    if args.step <= 0:
        return fail2(f"--step must be > 0 (got {args.step})")
    if args.cols < 1:
        return fail2(f"--cols must be >= 1 (got {args.cols})")
    if args.from_s < 0:
        return fail2(f"--from must be >= 0 (got {args.from_s})")
    if args.to_s is not None and args.to_s <= args.from_s:
        return fail2(f"--to ({args.to_s}) must be > --from "
                     f"({args.from_s})")
    if not os.path.isdir(args.frames_dir):
        return fail2(f"frames dir not found: {args.frames_dir}")

    present = {}
    for name in os.listdir(args.frames_dir):
        m = FRAME_RE.match(name)
        if m:
            present[int(m.group(1))] = name
    if not present:
        return fail2(f"no frame_%05d.png frames in {args.frames_dir}")

    total = max(present) + 1
    duration = total / FPS
    holes = [n for n in range(total) if n not in present]

    to_s = duration if args.to_s is None else min(args.to_s, duration)
    samples = []                       # (t, n), time order, n unique
    seen = set()
    k = 1
    while True:
        t = k * args.step
        if t >= to_s:
            break
        k += 1
        if t < args.from_s:
            continue
        n = frame_at(t)
        if n >= total:                 # t < duration forbids this;
            continue                   # float paranoia, never invent
        if n in seen:
            continue
        seen.add(n)
        samples.append((t, n))
    if not samples:
        return fail2(f"no samples in window: step {args.step} over "
                     f"[{args.from_s}, {to_s}) of a {duration:.2f} s "
                     f"film")

    print(f"sheet: {args.frames_dir} — {len(present)} frames present "
          f"(0..{total - 1}), duration {duration:.2f} s")
    missing = [(t, n) for (t, n) in samples if n not in present]
    missing_ns = {n for _t, n in missing}
    other_holes = [n for n in holes if n not in missing_ns]
    if other_holes:
        named = ", ".join(f"frame_{n:05d}.png" for n in other_holes[:10])
        more = (f" (+{len(other_holes) - 10} more)"
                if len(other_holes) > 10 else "")
        print(f"NOTE: {len(other_holes)} non-sampled frame(s) absent "
              f"from the dir: {named}{more}")
    if missing:
        for t, n in missing:
            print(f"MISSING frame_{n:05d}.png (sample at t={t:.2f} s)")
        print(f"SHEET: FAILED ({len(missing)} sampled frame(s) "
              f"missing — no sheet written)")
        return 1

    first = os.path.join(args.frames_dir, present[samples[0][1]])
    try:
        tile_w, tile_h = parse_scale(args.scale, png_size(first))
    except ValueError as e:
        return fail2(str(e))
    if tile_w < 1 or tile_h < 1:
        return fail2(f"--scale must be positive (got {args.scale})")

    rows = (len(samples) + args.cols - 1) // args.cols
    if len(samples) <= 40:
        listing = ", ".join(f"t={t:g}s→frame {n}" for t, n in samples)
        print(f"samples ({len(samples)}): {listing}")
    else:
        print(f"samples: {len(samples)} (t={samples[0][0]:g}s .. "
              f"t={samples[-1][0]:g}s, every {args.step:g} s)")
    print(f"tiling: scale={tile_w}x{tile_h} tile={args.cols}x{rows} "
          f"padding={TILE_PADDING} color=black q:v 3")

    out_abs = os.path.abspath(args.out)
    out_dir = os.path.dirname(out_abs)
    if out_dir:
        os.makedirs(out_dir, exist_ok=True)

    with tempfile.TemporaryDirectory(prefix="make_sheet_") as tmp:
        for i, (_t, n) in enumerate(samples):
            src = os.path.join(args.frames_dir, present[n])
            dst = os.path.join(tmp, f"f{i:04d}.png")
            try:
                os.symlink(os.path.abspath(src), dst)
            except OSError:
                shutil.copyfile(src, dst)
        vf = (f"scale={tile_w}:{tile_h},"
              f"tile={args.cols}x{rows}:padding={TILE_PADDING}:"
              f"color=black")
        cmd = [ffmpeg, "-y", "-v", "error", "-framerate", "1",
               "-i", os.path.join(tmp, "f%04d.png"),
               "-vf", vf, "-frames:v", "1", "-q:v", "3", out_abs]
        try:
            r = subprocess.run(cmd, capture_output=True, text=True)
        except OSError as e:
            print(f"SHEET: FAILED (ffmpeg could not be executed: {e})")
            return 1
        if r.returncode != 0:
            tail = (r.stderr or "").strip().splitlines()
            for line in tail[-3:]:
                print(f"ffmpeg: {line}")
            print("SHEET: FAILED (ffmpeg exited "
                  f"{r.returncode} — no sheet written)")
            if os.path.exists(out_abs):
                os.unlink(out_abs)
            return 1

    if not os.path.exists(out_abs):
        print("SHEET: FAILED (ffmpeg reported success but no "
              "output file exists)")
        return 1
    print(f"{args.out} ({len(samples)} frames, every {args.step:g} s)")
    print(f"SHEET OK: {args.out} ({len(samples)} tiles in "
          f"{args.cols}x{rows}, {tile_w}x{tile_h} each)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
