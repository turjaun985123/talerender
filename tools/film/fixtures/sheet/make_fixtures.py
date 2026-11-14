#!/usr/bin/env python3
"""make_fixtures.py — synthetic frames for make_sheet.py tests.

Writes frames/frame_00000.png .. frame_00089.png (90 frames = 3.00 s
at the driver's 30 fps) next to this script. No binary renders: each
PNG is written with zlib/struct only. Frames are 108x192 solid
colour with the frame index encoded twice:

  * colour: R = n, G = 255 - n, B = (2n) mod 256 — a tile-centre
    pixel readback recovers n from R alone (JPEG survives within
    the run_tests tolerance);
  * pattern: a white 8x8 block at (10 * (n mod 10), 0), so tiles
    are also distinguishable by eye in the sheet.

Deterministic: re-running produces byte-identical frames.
Usage: python3 tools/film/fixtures/sheet/make_fixtures.py
"""

import os
import struct
import sys
import zlib

W, H, COUNT = 108, 192, 90
HERE = os.path.dirname(os.path.abspath(__file__))


def png_bytes(w, h, pixel):
    def chunk(tag, data):
        c = struct.pack(">I", len(data)) + tag + data
        return c + struct.pack(">I", zlib.crc32(tag + data) & 0xFFFFFFFF)

    raw = bytearray()
    for y in range(h):
        raw.append(0)                       # filter: none
        for x in range(w):
            raw += bytes(pixel(x, y))
    ihdr = struct.pack(">IIBBBBB", w, h, 8, 2, 0, 0, 0)
    return (b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", ihdr)
            + chunk(b"IDAT", zlib.compress(bytes(raw), 9))
            + chunk(b"IEND", b""))


def frame_pixel(n):
    base = (n, 255 - n, (2 * n) % 256)
    mx = 10 * (n % 10)

    def px(x, y):
        if y < 8 and mx <= x < mx + 8:
            return (255, 255, 255)
        return base
    return px


def main():
    outdir = os.path.join(HERE, "frames")
    os.makedirs(outdir, exist_ok=True)
    for n in range(COUNT):
        path = os.path.join(outdir, f"frame_{n:05d}.png")
        with open(path, "wb") as f:
            f.write(png_bytes(W, H, frame_pixel(n)))
    print(f"wrote {COUNT} frames ({W}x{H}) to {outdir}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
