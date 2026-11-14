#!/usr/bin/env python3
"""Byte-probe pack — same-renderer byte-identity gate for renderer changes.

WHAT THIS IS
    A stdlib-only comparison tool (tools/ci/byte_probes.py, seat
    W34). It compares two frame directories — a BASELINE rendered
    before a change and a CANDIDATE rendered after it — frame by
    frame, byte for byte, over a set of probe ranges, and prints a
    verdict. It is the standard gate the lead runs around each
    gpucore per-site migration (GPU-CORE-W07.md, migration order
    dof -> scenefile -> ab1 -> main -> film) and around any future
    renderer change (paint-cache wiring, readback unification,
    target-helper changes): the migration is allowed to proceed
    only when this tool prints PASS.

    Default ranges are the project's standing determinism probes
    (DETERMINISM-CONTRACT-W04.md §5, evidence E4):

        195-204    10 frames — the v1 probe range
        795-814    20 frames — mid-film range
        1195-1204  10 frames — straddles the old 160-entry
                   paint-cache cap, where the quantised state_key
                   defect (contract E2) measured 72.34 dB on
                   frames 1195-1199 before fix cabcc30

    Ranges are inclusive at both ends, matching how the evidence
    register counts them (1195-1204 = 10 frames).

USAGE
    byte_probes.py BASELINE_DIR CANDIDATE_DIR [options]

      --ranges A-B,C-D,...   probe ranges, inclusive (default
                             195-204,795-814,1195-1204); a bare
                             number probes that single frame
      --pattern PATTERN      frame filename pattern, Python
                             str.format style with {n} (default
                             frame_{n:05d}.png — the film driver's
                             naming). A pattern with no {n} probes
                             that one literal file in each dir
                             (the dof-chart / ab1-still case).
      --psnr-fallback        for every differing frame, also
                             decode both PNGs and report PSNR
                             (diagnostic only — never the gate)
      --self-test            run the synthetic self-test and exit

EXIT CODES (house convention, same as tools/ab/measure_ab.py)
    0  PASS — every probed frame byte-identical
    1  FAIL — at least one probed frame differs or is missing
    2  input error / refusal — bad arguments, a missing dir, or a
       side that contains NONE of the probed frames (a crashed or
       misdirected render is no evidence, never a soft pass; the
       determinism pack refuses the same way). One-line ERROR on
       stderr, no traceback, no verdict line.

THE DISCIPLINE THIS TOOL SERVES
    * Same renderer only. Baseline and candidate must come from
      the same binary lineage on the same renderer (both CI
      lavapipe, or both PC RADV). Cross-renderer byte-identity is
      physically not promised (contract §2: the GRADE grain's
      fp32 sin() and fp16/JIT rounding are vendor territory), so
      a lavapipe-vs-RADV FAIL is expected physics, not a defect.
      Cross-renderer questions belong to tools/ab/measure_ab.py
      and its PSNR bands, never to this tool.
    * A FAIL is a defect report, not a nuisance: it names the
      first differing frame and byte offset. Never answer it by
      widening the comparison (contract §8).
    * Probes prove NO CHANGE, not correctness. A byte-identical
      wrong frame passes. Correctness is owned by the milestone
      A/B evidence and the telemetry gate, not by this tool.

PSNR FALLBACK — SCOPE AND LIMITS
    --psnr-fallback decodes PNGs with a minimal stdlib decoder
    (zlib + unfiltering; bit depth 8; colour types 0/2/4/6;
    non-interlaced only — the subset the image crate writes).
    PSNR follows the house definition: 10*log10(255^2 / MSE),
    MSE over all decoded channels, float64. Anything outside
    the decoder's subset (interlaced, 16-bit, palette) reports
    "PSNR unavailable" with the reason and falls back to the
    byte-diff data — it never guesses and never crashes the run.
    Exit code and verdict are decided by the byte comparison
    alone; PSNR is there to size a difference the lead has
    already decided to investigate (the E2 defect measured
    72.34 dB: near-invisible, still a FAIL).
"""

import argparse
import hashlib
import math
import os
import struct
import sys
import tempfile
import zlib

DEFAULT_RANGES = "195-204,795-814,1195-1204"
DEFAULT_PATTERN = "frame_{n:05d}.png"


class InputError(Exception):
    """Bad input. Reported as one named line on stderr, exit 2."""


# ---------------------------------------------------------------- ranges

def parse_ranges(text):
    """'195-204,42,795-814' -> [(195, 204), (42, 42), (795, 814)]."""
    ranges = []
    for part in text.split(","):
        part = part.strip()
        if not part:
            raise InputError(f"empty item in --ranges {text!r}")
        if "-" in part:
            lo_s, hi_s = part.split("-", 1)
        else:
            lo_s = hi_s = part
        try:
            lo, hi = int(lo_s), int(hi_s)
        except ValueError:
            raise InputError(f"unparseable range item {part!r} in --ranges")
        if lo < 0 or hi < lo:
            raise InputError(f"invalid range {part!r} (need 0 <= lo <= hi)")
        ranges.append((lo, hi))
    if not ranges:
        raise InputError("--ranges names no frames")
    return ranges


def frame_name(pattern, n):
    if "{n" in pattern:
        try:
            return pattern.format(n=n)
        except (ValueError, KeyError, IndexError) as e:
            raise InputError(f"bad --pattern {pattern!r}: {e}")
    return pattern


def build_targets(pattern, ranges):
    """[(range_label, frame_number_or_None, relpath), ...]"""
    if "{n" not in pattern:
        return [("file", None, pattern)]
    targets = []
    for lo, hi in ranges:
        label = f"{lo}-{hi}" if lo != hi else f"{lo}"
        for n in range(lo, hi + 1):
            targets.append((label, n, frame_name(pattern, n)))
    return targets


# ---------------------------------------------------------------- compare

def first_diff(a, b):
    """(first differing offset, number of differing bytes).
    Offset is None when the byte strings are equal. A length
    difference counts every trailing byte as differing, and the
    first offset is the first index where they disagree (the
    shorter length when one is a prefix of the other)."""
    n = min(len(a), len(b))
    off = None
    count = abs(len(a) - len(b))
    for i in range(n):
        if a[i] != b[i]:
            if off is None:
                off = i
            count += 1
    if off is None and len(a) != len(b):
        off = n
    return off, count


def sha256_hex(data):
    return hashlib.sha256(data).hexdigest()


# ---------------------------------------------------------------- PNG decode (stdlib, for --psnr-fallback)

class PngError(Exception):
    """PNG outside the fallback decoder's subset; reason is named."""


def decode_png(data):
    """Minimal PNG decoder -> (width, height, channels, sample bytes).
    Bit depth 8, colour types 0/2/4/6, non-interlaced only."""
    if len(data) < 8 or data[:8] != b"\x89PNG\r\n\x1a\n":
        raise PngError("not a PNG file")
    pos = 8
    ihdr = None
    idat = bytearray()
    while pos + 8 <= len(data):
        length = struct.unpack(">I", data[pos:pos + 4])[0]
        ctype = data[pos + 4:pos + 8]
        chunk = data[pos + 8:pos + 8 + length]
        if ctype == b"IHDR":
            ihdr = struct.unpack(">IIBBBBB", chunk[:13])
        elif ctype == b"IDAT":
            idat.extend(chunk)
        elif ctype == b"IEND":
            break
        pos += 12 + length
    if ihdr is None:
        raise PngError("no IHDR chunk")
    width, height, depth, colour, _comp, _filt, interlace = ihdr
    if interlace != 0:
        raise PngError("interlaced PNG (Adam7) — outside decoder subset")
    if depth != 8:
        raise PngError(f"bit depth {depth} — outside decoder subset (8 only)")
    channels = {0: 1, 2: 3, 3: 1, 4: 2, 6: 4}.get(colour)
    if colour == 3:
        raise PngError("palette PNG — outside decoder subset")
    if channels is None:
        raise PngError(f"unknown colour type {colour}")
    try:
        raw = zlib.decompress(bytes(idat))
    except zlib.error as e:
        raise PngError(f"IDAT inflate failed: {e}")
    stride = width * channels
    if len(raw) < (stride + 1) * height:
        raise PngError("truncated image data")
    out = bytearray(stride * height)
    prev = bytearray(stride)
    p = 0
    for y in range(height):
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
                left = line[i - channels] if i >= channels else 0
                line[i] = (line[i] + ((left + prev[i]) >> 1)) & 0xFF
        elif f == 4:    # Paeth
            for i in range(stride):
                a = line[i - channels] if i >= channels else 0
                b = prev[i]
                c = prev[i - channels] if i >= channels else 0
                pa, pb, pc = abs(b - c), abs(a - c), abs(a + b - 2 * c)
                pred = a if (pa <= pb and pa <= pc) else (b if pb <= pc else c)
                line[i] = (line[i] + pred) & 0xFF
        elif f != 0:
            raise PngError(f"unknown row filter {f}")
        out[y * stride:(y + 1) * stride] = line
        prev = line
    return width, height, channels, bytes(out)


def psnr_of_pair(base_bytes, cand_bytes):
    """House PSNR: 10*log10(255^2/MSE), MSE over all decoded
    channels, float64. Returns a float (inf for identical pixels)
    or raises PngError with the reason it is unavailable."""
    wb, hb, cb, pb = decode_png(base_bytes)
    wc, hc, cc, pc = decode_png(cand_bytes)
    if (wb, hb, cb) != (wc, hc, cc):
        raise PngError(f"geometry differs (baseline {wb}x{hb}x{cb}, "
                       f"candidate {wc}x{hc}x{cc}) — PSNR not meaningful")
    n = len(pb)
    sse = 0.0
    for i in range(n):
        d = pb[i] - pc[i]
        sse += d * d
    mse = sse / n
    if mse == 0.0:
        return math.inf
    return 10.0 * math.log10(255.0 * 255.0 / mse)


# ---------------------------------------------------------------- the probe

def read_frame(dirpath, rel):
    """Bytes of dirpath/rel, or None when the file is absent.
    An existing-but-unreadable file is an input error."""
    path = os.path.join(dirpath, rel)
    if not os.path.isfile(path):
        return None
    try:
        with open(path, "rb") as fh:
            return fh.read()
    except OSError as e:
        raise InputError(f"cannot read {path} ({e.strerror or e})")


def run_probe(baseline_dir, candidate_dir, ranges, pattern, psnr_fallback,
              out):
    """Runs the comparison, prints the report to `out`, returns
    the exit code (0 PASS / 1 FAIL). Raises InputError (exit 2)."""
    for label, d in (("baseline", baseline_dir), ("candidate", candidate_dir)):
        if not os.path.isdir(d):
            raise InputError(f"{label} dir {d} does not exist or is not a directory")
    targets = build_targets(pattern, ranges)
    # Presence census first: a side with NONE of the probed frames
    # is a broken/misdirected render — refuse, never a soft pass.
    present = {"baseline": 0, "candidate": 0}
    loaded = []
    for label, n, rel in targets:
        b = read_frame(baseline_dir, rel)
        c = read_frame(candidate_dir, rel)
        if b is not None:
            present["baseline"] += 1
        if c is not None:
            present["candidate"] += 1
        loaded.append((label, n, rel, b, c))
    for side in ("baseline", "candidate"):
        if present[side] == 0:
            d = baseline_dir if side == "baseline" else candidate_dir
            raise InputError(
                f"{side} dir {d} contains none of the {len(targets)} probed "
                f"frames (pattern {pattern!r}) — render missing or wrong "
                f"dir/pattern; refusing, this is no evidence")

    out.write(f"byte probes: baseline={baseline_dir} candidate={candidate_dir} "
              f"pattern={pattern}\n")
    range_order = []
    for label, _n, _rel, _b, _c in loaded:
        if label not in range_order:
            range_order.append(label)
    total_frames = 0
    total_differ = 0
    failed_ranges = 0
    for rlabel in range_order:
        rows = [t for t in loaded if t[0] == rlabel]
        differ = []
        for _label, n, rel, b, c in rows:
            if b is None or c is None:
                side = "baseline" if b is None else "candidate"
                differ.append((n, rel, b, c, f"missing in {side}"))
            elif b != c:
                differ.append((n, rel, b, c, None))
        total_frames += len(rows)
        total_differ += len(differ)
        if not differ:
            out.write(f"range {rlabel}: PASS ({len(rows)}/{len(rows)} identical)\n")
            continue
        failed_ranges += 1
        out.write(f"range {rlabel}: FAIL ({len(rows) - len(differ)}/{len(rows)} "
                  f"identical, {len(differ)} differ)\n")
        n0, rel0, b0, c0, why0 = differ[0]
        name0 = f"frame {n0} ({rel0})" if n0 is not None else f"file {rel0}"
        if why0 is not None:
            out.write(f"  first difference: {name0}: {why0}\n")
        else:
            off, count = first_diff(b0, c0)
            noun = "byte differs" if count == 1 else "bytes differ"
            out.write(f"  first difference: {name0} at byte {off} "
                      f"({count} {noun}; sizes {len(b0)} baseline / "
                      f"{len(c0)} candidate)\n")
            out.write(f"    sha256 baseline  {sha256_hex(b0)}\n")
            out.write(f"    sha256 candidate {sha256_hex(c0)}\n")
        if psnr_fallback:
            for n, rel, b, c, why in differ:
                name = f"frame {n} ({rel})" if n is not None else f"file {rel}"
                if why is not None:
                    out.write(f"  {name}: PSNR unavailable ({why})\n")
                    continue
                try:
                    value = psnr_of_pair(b, c)
                    rendered = "inf (identical pixels)" if math.isinf(value) \
                        else f"{value:.2f} dB"
                    out.write(f"  {name}: PSNR {rendered} "
                              f"(--psnr-fallback; diagnostic only)\n")
                except PngError as e:
                    out.write(f"  {name}: PSNR unavailable ({e}) — "
                              f"byte-diff data only\n")
    if total_differ == 0:
        out.write(f"BYTE PROBES: PASS ({total_frames}/{total_frames} frames "
                  f"identical across {len(range_order)} range(s))\n")
        return 0
    out.write(f"BYTE PROBES: FAIL ({total_differ} frame(s) differ in "
              f"{failed_ranges} of {len(range_order)} range(s))\n")
    return 1


# ---------------------------------------------------------------- self-test

def _encode_png_rgba(width, height, pixels):
    """Tiny filter-0 RGBA PNG writer for the self-test (pixels =
    bytes of width*height*4)."""
    def chunk(ctype, payload):
        body = ctype + payload
        return struct.pack(">I", len(payload)) + body + \
            struct.pack(">I", zlib.crc32(body) & 0xFFFFFFFF)
    stride = width * 4
    raw = bytearray()
    for y in range(height):
        raw.append(0)
        raw.extend(pixels[y * stride:(y + 1) * stride])
    ihdr = struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0)
    return b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", ihdr) + \
        chunk(b"IDAT", zlib.compress(bytes(raw))) + chunk(b"IEND", b"")


class _Capture:
    def __init__(self):
        self.parts = []

    def write(self, s):
        self.parts.append(s)

    def text(self):
        return "".join(self.parts)


def run_self_test(out):
    """Synthetic proof, in temp dirs, of the three verdicts:
    identical dirs PASS (exit 0); one planted byte in frame 1197
    FAILs (exit 1) naming that frame and offset, other ranges
    unaffected; a partially missing frame FAILs as missing; a
    side with no probed frames at all REFUSES (InputError).
    Plus the PSNR path on two hand-built PNGs whose expected
    PSNR is computed by hand. Prints the transcript; returns 0
    when every expectation holds, 1 otherwise."""
    failures = []

    def check(what, cond):
        out.write(f"self-test: {what}: {'ok' if cond else 'NOT OK'}\n")
        if not cond:
            failures.append(what)

    stub_len = 8192

    def stub_bytes(n):
        return bytes(((i * 31 + n * 7) & 0xFF) for i in range(stub_len))

    ranges = parse_ranges(DEFAULT_RANGES)
    tmp = tempfile.mkdtemp(prefix="byte-probes-selftest-")
    base = os.path.join(tmp, "base")
    cand = os.path.join(tmp, "cand")
    os.makedirs(base)
    os.makedirs(cand)
    for lo, hi in ranges:
        for n in range(lo, hi + 1):
            data = stub_bytes(n)
            for d in (base, cand):
                with open(os.path.join(d, frame_name(DEFAULT_PATTERN, n)),
                          "wb") as fh:
                    fh.write(data)

    out.write("self-test A: identical dirs -> expect PASS, exit 0\n")
    cap = _Capture()
    rc = run_probe(base, cand, ranges, DEFAULT_PATTERN, False, cap)
    out.write(cap.text())
    check("identical dirs exit 0", rc == 0)
    check("identical dirs verdict PASS",
          "BYTE PROBES: PASS (40/40" in cap.text())

    out.write("self-test B: flip byte 1234 of candidate frame 1197 "
              "-> expect FAIL, exit 1, range 1195-1204 only\n")
    victim = os.path.join(cand, frame_name(DEFAULT_PATTERN, 1197))
    with open(victim, "rb") as fh:
        data = bytearray(fh.read())
    data[1234] ^= 0xFF
    with open(victim, "wb") as fh:
        fh.write(bytes(data))
    cap = _Capture()
    rc = run_probe(base, cand, ranges, DEFAULT_PATTERN, False, cap)
    out.write(cap.text())
    check("planted byte exit 1", rc == 1)
    check("planted byte names frame 1197 at byte 1234",
          "frame 1197 (frame_01197.png) at byte 1234" in cap.text())
    check("unaffected ranges still PASS",
          "range 195-204: PASS" in cap.text()
          and "range 795-814: PASS" in cap.text())
    check("affected range FAILs", "range 1195-1204: FAIL" in cap.text())

    out.write("self-test C: remove candidate frame 801 "
              "-> expect FAIL naming it missing\n")
    with open(victim, "wb") as fh:  # restore B's flip first
        fh.write(stub_bytes(1197))
    os.remove(os.path.join(cand, frame_name(DEFAULT_PATTERN, 801)))
    cap = _Capture()
    rc = run_probe(base, cand, ranges, DEFAULT_PATTERN, False, cap)
    out.write(cap.text())
    check("missing frame exit 1", rc == 1)
    check("missing frame named", "frame 801 (frame_00801.png): "
          "missing in candidate" in cap.text())

    out.write("self-test D: candidate dir with no probed frames "
              "-> expect refusal (InputError, exit 2)\n")
    empty = os.path.join(tmp, "empty")
    os.makedirs(empty)
    try:
        run_probe(base, empty, ranges, DEFAULT_PATTERN, False, _Capture())
        check("empty side refused", False)
    except InputError as e:
        out.write(f"ERROR: {e}\n")
        check("empty side refused", True)

    out.write("self-test E: PSNR path — 4x2 RGBA PNGs, one sample "
              "differs by 16 -> MSE 8, expect PSNR 39.10 dB\n")
    px_a = bytearray(4 * 2 * 4)
    px_b = bytearray(px_a)
    px_b[0] = 16
    png_a = _encode_png_rgba(4, 2, bytes(px_a))
    png_b = _encode_png_rgba(4, 2, bytes(px_b))
    value = psnr_of_pair(png_a, png_b)
    expected = 10.0 * math.log10(255.0 * 255.0 / 8.0)
    out.write(f"self-test: computed PSNR {value:.4f} dB, "
              f"expected {expected:.4f} dB\n")
    check("PSNR matches hand computation", abs(value - expected) < 0.01)
    check("identical PNGs give inf PSNR",
          math.isinf(psnr_of_pair(png_a, png_a)))

    if failures:
        out.write(f"SELF-TEST: FAIL ({len(failures)} expectation(s) failed)\n")
        return 1
    out.write("SELF-TEST: PASS\n")
    return 0


# ---------------------------------------------------------------- entry

def main(argv=None):
    parser = argparse.ArgumentParser(
        prog="byte_probes.py",
        description="Same-renderer byte-identity probes over frame "
                    "ranges. Exit 0 PASS, 1 FAIL, 2 input error.")
    parser.add_argument("baseline", nargs="?", help="baseline frame dir")
    parser.add_argument("candidate", nargs="?", help="candidate frame dir")
    parser.add_argument("--ranges", default=DEFAULT_RANGES,
                        help=f"inclusive frame ranges (default {DEFAULT_RANGES})")
    parser.add_argument("--pattern", default=DEFAULT_PATTERN,
                        help=f"frame filename pattern (default {DEFAULT_PATTERN})")
    parser.add_argument("--psnr-fallback", action="store_true",
                        help="also report PSNR for differing frames "
                             "(diagnostic only)")
    parser.add_argument("--self-test", action="store_true",
                        help="run the synthetic self-test and exit")
    args = parser.parse_args(argv)
    try:
        if args.self_test:
            return run_self_test(sys.stdout)
        if not args.baseline or not args.candidate:
            raise InputError("need BASELINE_DIR and CANDIDATE_DIR "
                             "(or --self-test)")
        ranges = parse_ranges(args.ranges)
        return run_probe(args.baseline, args.candidate, ranges,
                         args.pattern, args.psnr_fallback, sys.stdout)
    except InputError as e:
        print(f"ERROR: {e}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
