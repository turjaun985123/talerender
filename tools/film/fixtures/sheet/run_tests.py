#!/usr/bin/env python3
"""run_tests.py — verification for tools/film/make_sheet.py.

Cases (all against the synthetic 90-frame fixture dir; no binary
renders anywhere):

  A  --step 0.5 --cols 5 --scale 54x96: samples t=0.5..2.5 ->
     frames 15/30/45/60/75, an exact 5x1 tile fill. Dimensions via
     ffprobe; tile ORDER and identity via an ffmpeg rawvideo
     decode: each tile's centre pixel must match its source
     frame's encoded colour (R = frame index) within JPEG
     tolerance.
  B  defaults (step 0.5, cols 9, scale 216x384): same 5 samples
     in a 9x1 grid, 4 padded cells; dims via ffprobe.
  C  missing sampled frame (frame_00030.png removed from a copy):
     exit 1, the frame is NAMED, no sheet file is left behind.
  D  ffmpeg absent from PATH: exit 2, one-line ERROR on stderr.
  E  --from 1 --to 2 --step 0.5 --cols 2: samples t=1.0/1.5 ->
     frames 30/45 only; dims + tile identities checked as in A.

Tile geometry (ffmpeg tile= as the skill invokes it: padding=4,
margin unset): `padding` is the INNER gap between tiles and there
is no outer border, so sheet W = cols*tileW + (cols-1)*4, H =
rows*tileH + (rows-1)*4; cell i's centre is at
(i*(tileW+4) + tileW/2, tileH/2) for row 0. (Verified against
ffprobe on this machine's ffmpeg 8.1.2 — an earlier draft of this
test assumed an outer border and failed 286x96 vs 278x104; the
filter string itself is the skill's, unchanged.)

Exit code: 0 = all PASS, 1 = any FAIL.
Usage: python3 tools/film/fixtures/sheet/run_tests.py
"""

import json
import os
import shutil
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
FRAMES = os.path.join(HERE, "frames")
TOOL = os.path.abspath(os.path.join(HERE, "..", "..", "make_sheet.py"))
TOL = 28                      # JPEG q:v 3 on solid colour: generous
RESULTS = []


def check(name, ok, detail=""):
    RESULTS.append((name, ok))
    print(f"{'PASS' if ok else 'FAIL'}  {name}  {detail}")


def run_tool(args, env=None):
    return subprocess.run([sys.executable, TOOL] + args,
                          capture_output=True, text=True, env=env)


def probe_dims(path):
    r = subprocess.run(
        ["ffprobe", "-v", "error", "-select_streams", "v:0",
         "-show_entries", "stream=width,height", "-of", "json", path],
        capture_output=True, text=True)
    s = json.loads(r.stdout)["streams"][0]
    return s["width"], s["height"]


def tile_centres(path, w, h, cols, tile_w, tile_h):
    r = subprocess.run(
        ["ffmpeg", "-v", "error", "-i", path, "-f", "rawvideo",
         "-pix_fmt", "rgb24", "-"], capture_output=True)
    raw = r.stdout
    out = []
    for i in range(cols):
        cx = i * (tile_w + 4) + tile_w // 2
        cy = tile_h // 2
        off = (cy * w + cx) * 3
        out.append(tuple(raw[off:off + 3]))
    return out


def frame_colour(n):
    return (n, 255 - n, (2 * n) % 256)


def colour_close(got, want):
    return all(abs(g - w) <= TOL for g, w in zip(got, want))


def main():
    if not os.path.isdir(FRAMES) or len(os.listdir(FRAMES)) < 90:
        subprocess.run([sys.executable,
                        os.path.join(HERE, "make_fixtures.py")],
                       check=True, capture_output=True)

    with tempfile.TemporaryDirectory(prefix="sheet_tests_") as tmp:
        # --- A: exact-fill sheet, dims + tile identity/order
        out_a = os.path.join(tmp, "a.jpg")
        r = run_tool([FRAMES, out_a, "--step", "0.5", "--cols", "5",
                      "--scale", "54x96"])
        check("A exit 0", r.returncode == 0, f"(rc={r.returncode})")
        if r.returncode == 0:
            dims = probe_dims(out_a)
            check("A dims 286x96", dims == (286, 96), f"(got {dims})")
            centres = tile_centres(out_a, 286, 96, 5, 54, 96)
            want = [frame_colour(n) for n in (15, 30, 45, 60, 75)]
            ok = all(colour_close(g, w) for g, w in zip(centres, want))
            check("A tiles are frames 15/30/45/60/75 in order", ok,
                  f"(centres R={[c[0] for c in centres]})")
            check("A success line", "SHEET OK" in r.stdout
                  and "(5 frames, every 0.5 s)" in r.stdout)

        # --- B: defaults
        out_b = os.path.join(tmp, "b.jpg")
        r = run_tool([FRAMES, out_b])
        dims = probe_dims(out_b) if r.returncode == 0 else None
        check("B defaults exit 0, dims 1976x384",
              r.returncode == 0 and dims == (1976, 384),
              f"(rc={r.returncode}, dims={dims})")

        # --- C: missing sampled frame fails, named, no sheet
        copy_dir = os.path.join(tmp, "frames_copy")
        shutil.copytree(FRAMES, copy_dir)
        os.unlink(os.path.join(copy_dir, "frame_00030.png"))
        out_c = os.path.join(tmp, "c.jpg")
        r = run_tool([copy_dir, out_c, "--step", "1", "--cols", "3",
                      "--scale", "54x96"])
        check("C missing frame exit 1", r.returncode == 1,
              f"(rc={r.returncode})")
        check("C frame named", "frame_00030.png" in r.stdout)
        check("C no sheet left", not os.path.exists(out_c))

        # --- D: ffmpeg absent -> exit 2, one-line ERROR
        env = dict(os.environ)
        env["PATH"] = "/nonexistent"
        r = run_tool([FRAMES, os.path.join(tmp, "d.jpg")], env=env)
        check("D no-ffmpeg exit 2 + ERROR line",
              r.returncode == 2
              and "ERROR: ffmpeg not found" in r.stderr,
              f"(rc={r.returncode})")

        # --- E: --from/--to window
        out_e = os.path.join(tmp, "e.jpg")
        r = run_tool([FRAMES, out_e, "--from", "1", "--to", "2",
                      "--step", "0.5", "--cols", "2",
                      "--scale", "54x96"])
        dims = probe_dims(out_e) if r.returncode == 0 else None
        ok = r.returncode == 0 and dims == (112, 96)
        if ok:
            centres = tile_centres(out_e, 112, 96, 2, 54, 96)
            ok = all(colour_close(g, w) for g, w in
                     zip(centres, [frame_colour(30), frame_colour(45)]))
        check("E window frames 30/45 only, dims 112x96", ok,
              f"(rc={r.returncode}, dims={dims})")

    failed = [n for n, ok in RESULTS if not ok]
    print(f"OVERALL: {'PASS' if not failed else 'FAIL'} "
          f"({len(RESULTS) - len(failed)}/{len(RESULTS)} checks)")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
