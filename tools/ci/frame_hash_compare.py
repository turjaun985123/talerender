#!/usr/bin/env python3
"""Frame-hash comparison — the native selftest leg's frame gate.

WHAT THIS IS
    A stdlib-only comparison tool (tools/ci/frame_hash_compare.py,
    seat W48) that compares two directories of same-named frames —
    e.g. Chrome-path frames vs native frames for the same project
    at the same times — frame by frame by sha256, and classifies
    every frame. It exists for one job: the frame-hash leg of a
    future native selftest (docs/integration/SELFTEST-NATIVE-W48.md
    is the leg spec; the gap analysis's G19/R10 requires the
    skill's selftest to gain a native leg WITH a frame comparison
    against the Chrome path, extending the selftest rather than
    bypassing the Chrome path).

HOW IT DIFFERS FROM ITS NEIGHBOURS
    * byte_probes.py (W34) gates a renderer change on ONE renderer:
      baseline vs candidate frames from the same binary lineage,
      byte-identity or FAIL, probe ranges over a film's frame
      numbers. This tool compares WHOLE directories of named
      frames across two production paths and speaks the
      visual-regression dialect (below), because a cross-path
      comparison usually cannot promise byte-identity.
    * visual_regression.py (W16) compares fresh renders against
      pinned golden references from a manifest. This tool has no
      manifest and no pins: two directories, compared directly.

THE CLASSIFICATION DIALECT (mirror of visual_regression.py)
    MATCH             sha256 identical on both sides.
    WITHIN-BAND       bytes differ but PSNR >= --band. Band mode
                      only. Includes pixel-identical-but-re-encoded
                      files (PSNR inf, bytes differ). A signal to
                      look, not a failure — every WITHIN-BAND row
                      wants an explanation in the run's notes.
    DRIFT             band mode: bytes differ and PSNR < --band,
                      or PSNR is unavailable for a differing pair
                      (an undecodable difference cannot be proven
                      within band, so it fails closed as DRIFT —
                      this tool never guesses). FAILS.
    DIFFERS           strict mode: bytes differ. PSNR is reported
                      where decodable, as a diagnostic only.
                      FAILS.
    MISSING-IN-CANDIDATE / MISSING-IN-REFERENCE
                      the name exists on one side only. These play
                      the role visual_regression's MISSING-RENDER /
                      REFERENCE-MISSING play there: a frame that
                      did not happen must not read as a pass.
                      FAILS in both modes.

MODES
    --strict (the default)  any difference or missing frame fails.
                            This is the same-path regime: native
                            vs native across a change, where the
                            post-cabcc30 norm is byte-identity.
    --band DB               differing frames at or above DB dB
                            pass as WITHIN-BAND; below fail as
                            DRIFT. This is the cross-path regime:
                            Chrome masters carry one q0.93 JPEG
                            generation and native frames are clean
                            PNGs (decision (B), contract SS5), so
                            byte-identity across paths is
                            impossible BY DESIGN and a strict gate
                            there would fail forever. The W16 band
                            is 80 dB; choosing a band is a policy
                            act, stated on the command line, never
                            silently defaulted in band mode.

PSNR — IMPORTED, NOT REINVENTED
    Decoding + PSNR come from byte_probes.py in this directory
    (its stdlib PNG decoder and house PSNR: 10*log10(255^2/MSE),
    MSE over all decoded channels, float64), imported by path so
    the definitions cannot drift apart. The decoder's subset is
    bit depth 8, colour types 0/2/4/6, non-interlaced. A differing
    pair outside the subset (e.g. JPEG frames — the Chrome review
    path writes JPEG) reports "PSNR unavailable" with the reason:
    hash-only in strict mode, DRIFT in band mode.

USAGE
    frame_hash_compare.py REF_DIR CAND_DIR [options]

      --pattern GLOB   frame filename glob matched in both dirs
                       (default frame_*.png); the comparison set
                       is the union of matching names
      --strict         any difference fails (default)
      --band DB        WITHIN-BAND threshold in dB (mutually
                       exclusive with --strict)
      --self-test      run the synthetic fixture proof and exit

EXIT CODES (house convention; the exit code agrees with the
verdict line, as in visual_regression.py)
    0  FRAME HASH: PASS
    1  FRAME HASH: FAIL (DRIFT / DIFFERS / missing frames)
    2  input error / refusal — bad arguments, a missing dir, or a
       side containing NONE of the matching frames (a crashed or
       misdirected render is no evidence, never a soft pass; the
       byte-probe pack refuses the same way). One ERROR line on
       stderr, no verdict line.
"""

import argparse
import fnmatch
import hashlib
import importlib.util
import math
import os
import sys

DEFAULT_PATTERN = "frame_*.png"


class InputError(Exception):
    """Bad input. Reported as one named line on stderr, exit 2."""


# ------------------------------------------------- PSNR, imported from byte_probes

def _load_byte_probes():
    """Import the sibling byte_probes.py by path (house pattern:
    visual_regression.py imports tools/ab/measure_ab.py the same
    way) so the PNG decoder and PSNR definition live in exactly
    one place."""
    here = os.path.dirname(os.path.abspath(__file__))
    path = os.path.join(here, "byte_probes.py")
    spec = importlib.util.spec_from_file_location("byte_probes", path)
    if spec is None or spec.loader is None:
        raise InputError(f"cannot import PNG/PSNR support from {path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


BP = _load_byte_probes()
PngError = BP.PngError
psnr_of_pair = BP.psnr_of_pair


# ---------------------------------------------------------------- compare

def sha256_hex(data):
    return hashlib.sha256(data).hexdigest()


def list_frames(dirpath, pattern):
    """Sorted names in dirpath matching the glob (files only)."""
    try:
        names = os.listdir(dirpath)
    except OSError as e:
        raise InputError(f"cannot list {dirpath} ({e.strerror or e})")
    return sorted(n for n in names
                  if fnmatch.fnmatchcase(n, pattern)
                  and os.path.isfile(os.path.join(dirpath, n)))


def read_frame(dirpath, name):
    path = os.path.join(dirpath, name)
    try:
        with open(path, "rb") as fh:
            return fh.read()
    except OSError as e:
        raise InputError(f"cannot read {path} ({e.strerror or e})")


def run_compare(ref_dir, cand_dir, pattern, band, out):
    """Runs the comparison, prints the report to `out`, returns
    the exit code (0 PASS / 1 FAIL). Raises InputError (exit 2).
    `band` is None in strict mode, else the WITHIN-BAND threshold
    in dB."""
    for label, d in (("reference", ref_dir), ("candidate", cand_dir)):
        if not os.path.isdir(d):
            raise InputError(
                f"{label} dir {d} does not exist or is not a directory")
    ref_names = list_frames(ref_dir, pattern)
    cand_names = list_frames(cand_dir, pattern)
    # Presence census: a side with NO matching frames at all is a
    # broken/misdirected render — refuse, never a soft pass.
    for label, d, names in (("reference", ref_dir, ref_names),
                            ("candidate", cand_dir, cand_names)):
        if not names:
            raise InputError(
                f"{label} dir {d} contains no frames matching "
                f"{pattern!r} — render missing or wrong dir/pattern; "
                f"refusing, this is no evidence")
    all_names = sorted(set(ref_names) | set(cand_names))
    mode = "strict" if band is None else f"band {band:g} dB"
    out.write(f"frame hash compare: reference={ref_dir} "
              f"candidate={cand_dir} pattern={pattern} mode={mode}\n")

    counts = {"MATCH": 0, "WITHIN-BAND": 0, "DRIFT": 0, "DIFFERS": 0,
              "MISSING-IN-CANDIDATE": 0, "MISSING-IN-REFERENCE": 0}
    for name in all_names:
        in_ref = name in ref_names
        in_cand = name in cand_names
        if not in_cand:
            counts["MISSING-IN-CANDIDATE"] += 1
            out.write(f"  {name}: MISSING-IN-CANDIDATE "
                      f"(present in reference only)\n")
            continue
        if not in_ref:
            counts["MISSING-IN-REFERENCE"] += 1
            out.write(f"  {name}: MISSING-IN-REFERENCE "
                      f"(present in candidate only)\n")
            continue
        ref_bytes = read_frame(ref_dir, name)
        cand_bytes = read_frame(cand_dir, name)
        if ref_bytes == cand_bytes:
            counts["MATCH"] += 1
            out.write(f"  {name}: MATCH (sha256 "
                      f"{sha256_hex(ref_bytes)[:16]}…)\n")
            continue
        # Differing pair: PSNR where decodable, else the reason.
        psnr_text = None
        psnr_value = None
        try:
            psnr_value = psnr_of_pair(ref_bytes, cand_bytes)
            psnr_text = "inf (identical pixels, bytes differ)" \
                if math.isinf(psnr_value) else f"{psnr_value:.2f} dB"
        except PngError as e:
            psnr_text = f"unavailable ({e})"
        hashes = (f"sha256 reference {sha256_hex(ref_bytes)[:16]}… / "
                  f"candidate {sha256_hex(cand_bytes)[:16]}…")
        if band is None:
            counts["DIFFERS"] += 1
            out.write(f"  {name}: DIFFERS (PSNR {psnr_text}; {hashes})\n")
        elif psnr_value is not None and psnr_value >= band:
            counts["WITHIN-BAND"] += 1
            out.write(f"  {name}: WITHIN-BAND (PSNR {psnr_text} >= "
                      f"{band:g} dB; {hashes})\n")
        else:
            counts["DRIFT"] += 1
            if psnr_value is None:
                why = f"PSNR {psnr_text} — cannot prove within band, " \
                      f"fails closed"
            else:
                why = f"PSNR {psnr_text} < {band:g} dB"
            out.write(f"  {name}: DRIFT ({why}; {hashes})\n")

    total = len(all_names)
    failing = (counts["DRIFT"] + counts["DIFFERS"]
               + counts["MISSING-IN-CANDIDATE"]
               + counts["MISSING-IN-REFERENCE"])
    parts = [f"{counts[k]} {k}" for k in
             ("MATCH", "WITHIN-BAND", "DRIFT", "DIFFERS",
              "MISSING-IN-CANDIDATE", "MISSING-IN-REFERENCE")
             if counts[k]]
    summary = ", ".join(parts) if parts else "no frames compared"
    if failing == 0:
        out.write(f"FRAME HASH: PASS ({summary}; {total} frame name(s), "
                  f"mode {mode})\n")
        return 0
    out.write(f"FRAME HASH: FAIL ({summary}; {total} frame name(s), "
              f"mode {mode})\n")
    return 1


# ---------------------------------------------------------------- self-test

class _Capture:
    def __init__(self):
        self.parts = []

    def write(self, s):
        self.parts.append(s)

    def text(self):
        return "".join(self.parts)


def _fixtures_dir():
    here = os.path.dirname(os.path.abspath(__file__))
    return os.path.join(here, "fixtures", "frame_hash")


def run_self_test(out):
    """Synthetic proof against the committed fixtures in
    tools/ci/fixtures/frame_hash/ (see SELFTEST-NATIVE-W48.md for
    how the fixtures were generated):
      A identical dirs      -> strict PASS, exit 0
      B 1-LSB planted frame -> strict FAIL (DIFFERS), exit 1;
                               band 80 PASS (WITHIN-BAND), exit 0
      C heavily planted frame -> band 80 FAIL (DRIFT), exit 1
      D missing + extra frames -> FAIL, both names reported,
                               exit 1
    Prints the transcripts; returns 0 when every expectation
    holds, 1 otherwise."""
    fx = _fixtures_dir()
    chrome = os.path.join(fx, "chrome")
    identical = os.path.join(fx, "native-identical")
    withinband = os.path.join(fx, "native-withinband")
    drift = os.path.join(fx, "native-drift")
    missing = os.path.join(fx, "native-missing")
    for d in (chrome, identical, withinband, drift, missing):
        if not os.path.isdir(d):
            out.write(f"self-test: fixture dir {d} missing — the "
                      f"self-test cannot run\n")
            out.write("SELF-TEST: FAIL (fixtures missing)\n")
            return 1
    failures = []

    def check(what, cond):
        out.write(f"self-test: {what}: {'ok' if cond else 'NOT OK'}\n")
        if not cond:
            failures.append(what)

    out.write("self-test A: chrome vs native-identical, strict "
              "-> expect PASS, exit 0\n")
    cap = _Capture()
    rc = run_compare(chrome, identical, DEFAULT_PATTERN, None, cap)
    out.write(cap.text())
    check("identical dirs exit 0", rc == 0)
    check("identical dirs all MATCH",
          "3 MATCH" in cap.text() and "FRAME HASH: PASS" in cap.text())

    out.write("self-test B1: chrome vs native-withinband, strict "
              "-> expect FAIL (DIFFERS), exit 1\n")
    cap = _Capture()
    rc = run_compare(chrome, withinband, DEFAULT_PATTERN, None, cap)
    out.write(cap.text())
    check("withinband strict exit 1", rc == 1)
    check("planted frame named DIFFERS",
          "frame_00001.png: DIFFERS" in cap.text())

    out.write("self-test B2: chrome vs native-withinband, band 80 "
              "-> expect PASS (WITHIN-BAND), exit 0\n")
    cap = _Capture()
    rc = run_compare(chrome, withinband, DEFAULT_PATTERN, 80.0, cap)
    out.write(cap.text())
    check("withinband band-mode exit 0", rc == 0)
    check("planted frame classified WITHIN-BAND",
          "frame_00001.png: WITHIN-BAND" in cap.text())

    out.write("self-test C: chrome vs native-drift, band 80 "
              "-> expect FAIL (DRIFT), exit 1\n")
    cap = _Capture()
    rc = run_compare(chrome, drift, DEFAULT_PATTERN, 80.0, cap)
    out.write(cap.text())
    check("drift band-mode exit 1", rc == 1)
    check("planted frame classified DRIFT",
          "frame_00001.png: DRIFT" in cap.text())

    out.write("self-test D: chrome vs native-missing, strict "
              "-> expect FAIL, both directions named, exit 1\n")
    cap = _Capture()
    rc = run_compare(chrome, missing, DEFAULT_PATTERN, None, cap)
    out.write(cap.text())
    check("missing-fixture exit 1", rc == 1)
    check("absent frame named MISSING-IN-CANDIDATE",
          "frame_00002.png: MISSING-IN-CANDIDATE" in cap.text())
    check("extra frame named MISSING-IN-REFERENCE",
          "frame_00003.png: MISSING-IN-REFERENCE" in cap.text())

    if failures:
        out.write(f"SELF-TEST: FAIL ({len(failures)} expectation(s) "
                  f"failed)\n")
        return 1
    out.write("SELF-TEST: PASS\n")
    return 0


# ---------------------------------------------------------------- entry

def main(argv=None):
    parser = argparse.ArgumentParser(
        prog="frame_hash_compare.py",
        description="Per-frame sha256 comparison of two frame "
                    "dirs, classified in the visual-regression "
                    "dialect. Exit 0 PASS, 1 FAIL, 2 input error.")
    parser.add_argument("reference", nargs="?",
                        help="reference frame dir (e.g. Chrome path)")
    parser.add_argument("candidate", nargs="?",
                        help="candidate frame dir (e.g. native path)")
    parser.add_argument("--pattern", default=DEFAULT_PATTERN,
                        help=f"frame filename glob (default "
                             f"{DEFAULT_PATTERN})")
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument("--strict", action="store_true",
                      help="any difference fails (default)")
    mode.add_argument("--band", type=float, default=None, metavar="DB",
                      help="WITHIN-BAND threshold in dB: differing "
                           "frames at/above it pass as WITHIN-BAND, "
                           "below fail as DRIFT")
    parser.add_argument("--self-test", action="store_true",
                        help="run the synthetic fixture proof and exit")
    args = parser.parse_args(argv)
    try:
        if args.self_test:
            return run_self_test(sys.stdout)
        if not args.reference or not args.candidate:
            raise InputError("need REF_DIR and CAND_DIR (or --self-test)")
        if args.band is not None and args.band <= 0:
            raise InputError(f"--band must be positive (got {args.band:g})")
        return run_compare(args.reference, args.candidate, args.pattern,
                           args.band, sys.stdout)
    except InputError as e:
        print(f"ERROR: {e}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
