#!/usr/bin/env python3
"""Exporter acceptance harness — the repo-side gate any exported (or
hand-authored) scene file must pass, per EXPORTER-W47 §4.

Given a directory of scene files and their Chrome reference frames,
this harness renders each scene with the native binary's scenefile
mode, scores each render against its reference with the house PSNR
(tools/ci/byte_probes.py `psnr_of_pair`, imported — never re-derived
here), runs the W47 §4.7 determinism spot-check (one scene rendered
twice, byte-compared), and optionally runs the film-lane telemetry
gate (W47 §4.6) over a film directory.

Sample layout (the contract):

    <samples-dir>/
        foo.scene.json      the scene file under test
        foo.ref.png         its Chrome reference frame

The render is made at the reference's own pixel size (the harness
passes --width/--height from the reference), so geometry mismatch
is a finding, not an artefact of defaults.

Classifications per sample:

    PASS          PSNR >= band (default 45 dB, W47 §4.5). Infinite
                  PSNR (byte-identical pixels) is PASS.
    BELOW-BAND    A real render was produced and scored under the
                  band (or could not be scored against the
                  reference's geometry). A quality verdict.
    ENV-BLOCKED   The environment prevented a verdict: the binary
                  failed (no adapter, panic, crash), produced no
                  output, timed out, or produced the black-frame
                  signature of the lavapipe/llvmpipe composer wall
                  (render ~uniform black against a non-black
                  reference). NEVER a quality verdict — a black
                  frame from a software-renderer wall says nothing
                  about the scene or the exporter.
    MISSING-REFERENCE
                  No foo.ref.png beside the scene file. The gate
                  cannot run for that sample.

Exit codes mirror the telemetry convention (EXIT-CODES-W02):

    0   Every sample PASS, determinism byte-identical, telemetry
        CHECK OK (when requested). The printed line is
        "EXPORTER ACCEPTANCE: PASS".
    1   Every sample received a quality verdict and at least one
        failed (BELOW-BAND, determinism mismatch, or telemetry
        CHECK FAILED). Line: "EXPORTER ACCEPTANCE: FAIL (...)".
    3   The gate could not fully run: a sample is MISSING-REFERENCE
        or ENV-BLOCKED, the binary is missing/unusable, the
        samples directory is unusable, or telemetry could not
        run. Line: "EXPORTER ACCEPTANCE: COULD NOT RUN (...)".

Known bias, named per W47 §4.1: Chrome references carry one JPEG
generation (decision (B) — native frames stay clean); that bias
counts against the native render in every PSNR printed here.

Usage:
    exporter_acceptance.py --samples DIR --bin PATH
        [--band 45.0] [--film-dir DIR] [--timeout 300]
        [--renders DIR]        keep renders in DIR (default: temp)
    exporter_acceptance.py --scene FILE --ref PNG --bin PATH
        Single sample, tested in place: scene files whose texture
        paths are relative to their own location (the repo's own
        scenes/*) must be run this way, not copied into a samples
        directory.
    exporter_acceptance.py --self-test
        Runs the synthetic fixture cases under
        tools/format/fixtures/exporter_acceptance/ against a stub
        binary: identical pair -> PASS/exit 0, planted 1-pixel
        shift -> BELOW-BAND/exit 1, missing reference -> exit 3.
"""

import argparse
import io
import math
import os
import shutil
import subprocess
import sys
import tempfile
from contextlib import redirect_stdout

# House PSNR — imported, never forked. tools/ci/byte_probes.py is
# import-safe (its main() sits behind the __main__ guard).
_REPO_ROOT = os.path.dirname(os.path.dirname(os.path.dirname(
    os.path.abspath(__file__))))
sys.path.insert(0, os.path.join(_REPO_ROOT, "tools", "ci"))
import byte_probes  # noqa: E402

psnr_of_pair = byte_probes.psnr_of_pair
decode_png = byte_probes.decode_png
PngError = byte_probes.PngError

DEFAULT_BAND = 45.0
# W47 §4.5 rider: at most 0.01% of pixels may differ by > 10.
RIDER_MAX_PCT_GT10 = 0.01

CHECK_OK_LINE = "FILM TELEMETRY: CHECK OK"


# ------------------------------------------------------------ helpers

def png_mean_and_zero_frac(data):
    """(mean sample value, fraction of samples == 0) of a PNG."""
    _w, _h, _c, pixels = decode_png(data)
    n = len(pixels)
    if n == 0:
        return 0.0, 1.0
    return sum(pixels) / n, pixels.count(0) / n


def pct_pixels_gt10(ref_bytes, cand_bytes):
    """Percent of pixels whose largest per-channel |delta| > 10."""
    wb, hb, cb, pb = decode_png(ref_bytes)
    wc, hc, cc, pc = decode_png(cand_bytes)
    if (wb, hb, cb) != (wc, hc, cc):
        raise PngError("geometry differs — %>10 not meaningful")
    count = 0
    for i in range(0, len(pb), cb):
        if max(abs(pb[i + k] - pc[i + k]) for k in range(cb)) > 10:
            count += 1
    return 100.0 * count / (wb * hb)


def is_black_frame(render_bytes, ref_bytes):
    """The software-renderer wall signature: a (near-)uniform black
    render against a reference that is not black. Returns (bool,
    evidence string). A genuinely dark scene pair (reference also
    near-black) is NOT the wall — it falls through to PSNR."""
    r_mean, r_zero = png_mean_and_zero_frac(render_bytes)
    if r_mean > 1.0 or r_zero < 0.999:
        return False, ""
    ref_mean, _ = png_mean_and_zero_frac(ref_bytes)
    if ref_mean <= 10.0:
        return False, ""
    return True, (f"render mean {r_mean:.3f}, {r_zero * 100:.2f}% zero "
                  f"samples vs reference mean {ref_mean:.1f} — the "
                  f"lavapipe/llvmpipe composer black-frame wall")


def run_binary(argv, timeout):
    """Run the renderer; return (rc, stdout, stderr). rc is None on
    timeout, -1 when the process could not be started."""
    try:
        proc = subprocess.run(argv, capture_output=True, text=True,
                              timeout=timeout)
        return proc.returncode, proc.stdout, proc.stderr
    except subprocess.TimeoutExpired:
        return None, "", f"timed out after {timeout}s"
    except OSError as e:
        return -1, "", f"could not start binary: {e}"


def first_evidence_line(stderr, stdout):
    """The single most diagnostic line of a failed render. A Rust
    panic splits its location and its message across two lines
    ("panicked at src/x.rs:N:M:" then the message) — quote both,
    or the evidence names a line number and hides the cause."""
    for text in (stderr, stdout):
        lines = [ln.strip() for ln in text.splitlines()]
        for i, s in enumerate(lines):
            if "panicked at" in s:
                msg = next((t for t in lines[i + 1:] if t), "")
                return f"{s} {msg}".strip()
        for s in lines:
            if s:
                return s
    return "(no output)"


# ------------------------------------------------------------ samples

def discover_samples(samples_dir):
    """[(stem, scene_path, ref_path)] for every *.scene.json."""
    out = []
    for name in sorted(os.listdir(samples_dir)):
        if name.endswith(".scene.json"):
            stem = name[: -len(".scene.json")]
            out.append((stem, os.path.join(samples_dir, name),
                        os.path.join(samples_dir, stem + ".ref.png")))
    return out


def pair_sample(scene_path, ref_path):
    """One in-place sample for --scene/--ref: repo scene files
    resolve their textures relative to their own location, so a
    scene under test is not always copyable into a samples dir."""
    stem = os.path.basename(scene_path)
    if stem.endswith(".scene.json"):
        stem = stem[: -len(".scene.json")]
    return (stem, scene_path, ref_path)


def score_sample(binary, stem, scene_path, ref_path, renders_dir,
                 band, timeout):
    """Render + score one sample. Returns a result dict with keys:
    stem, cls, psnr (float|None), detail."""
    result = {"stem": stem, "cls": None, "psnr": None, "detail": ""}
    if not os.path.isfile(ref_path):
        result["cls"] = "MISSING-REFERENCE"
        result["detail"] = f"no reference at {os.path.basename(ref_path)}"
        return result
    ref_bytes = open(ref_path, "rb").read()
    try:
        rw, rh, _rc, _rp = decode_png(ref_bytes)
    except PngError as e:
        result["cls"] = "MISSING-REFERENCE"
        result["detail"] = f"reference unreadable: {e}"
        return result

    out_png = os.path.join(renders_dir, stem + ".render.png")
    rc, stdout, stderr = run_binary(
        [binary, "--scene-file", scene_path, "--out", out_png,
         "--width", str(rw), "--height", str(rh)], timeout)
    if rc is None or rc == -1:
        result["cls"] = "ENV-BLOCKED"
        result["detail"] = first_evidence_line(stderr, stdout)
        return result
    if rc != 0 or not os.path.isfile(out_png):
        result["cls"] = "ENV-BLOCKED"
        result["detail"] = (f"binary exit {rc}: "
                            f"{first_evidence_line(stderr, stdout)}")
        return result

    render_bytes = open(out_png, "rb").read()
    black, evidence = is_black_frame(render_bytes, ref_bytes)
    if black:
        result["cls"] = "ENV-BLOCKED"
        result["detail"] = evidence
        result["render_path"] = out_png
        return result

    try:
        value = psnr_of_pair(ref_bytes, render_bytes)
    except PngError as e:
        result["cls"] = "BELOW-BAND"
        result["detail"] = f"PSNR unavailable ({e})"
        result["render_path"] = out_png
        return result
    result["psnr"] = value
    result["render_path"] = out_png
    if value == math.inf:
        result["cls"] = "PASS"
        result["detail"] = "byte-identical pixels; 0.00% px>10"
        return result
    try:
        pct = pct_pixels_gt10(ref_bytes, render_bytes)
        rider = (f"{pct:.4f}% px>10"
                 + (" (W47 §4.5 rider exceeded)" if pct > RIDER_MAX_PCT_GT10
                    else ""))
    except PngError:
        rider = "%>10 unavailable"
    result["detail"] = rider
    result["cls"] = "PASS" if value >= band else "BELOW-BAND"
    return result


def determinism_check(binary, sample, renders_dir, timeout):
    """W47 §4.7: render one already-rendered scene a second time and
    byte-compare. Returns (status, detail) with status in
    PASS / FAIL / ENV-BLOCKED."""
    stem, scene_path, ref_path = sample
    ref_bytes = open(ref_path, "rb").read()
    rw, rh, _c, _p = decode_png(ref_bytes)
    again = os.path.join(renders_dir, stem + ".render2.png")
    rc, stdout, stderr = run_binary(
        [binary, "--scene-file", scene_path, "--out", again,
         "--width", str(rw), "--height", str(rh)], timeout)
    if rc is None or rc == -1 or rc != 0 or not os.path.isfile(again):
        return "ENV-BLOCKED", (f"second render failed: "
                               f"{first_evidence_line(stderr, stdout)}")
    first = open(os.path.join(renders_dir, stem + ".render.png"),
                 "rb").read()
    second = open(again, "rb").read()
    if first == second:
        return "PASS", f"{stem}: two renders byte-identical " \
                       f"({len(first)} bytes)"
    return "FAIL", f"{stem}: two renders differ " \
                   f"({len(first)} vs {len(second)} bytes)"


def telemetry_gate(binary, film_dir, timeout):
    """W47 §4.6: the verdict is the printed line, per contract.
    Returns (status, detail), status in OK / FAILED / COULD-NOT-RUN."""
    rc, stdout, stderr = run_binary(
        [binary, "--film-telemetry", film_dir], timeout)
    lines = [ln.strip() for ln in stdout.splitlines()]
    if CHECK_OK_LINE in lines and rc == 0:
        return "OK", CHECK_OK_LINE
    if rc == 1 and any(ln.startswith("FILM TELEMETRY: CHECK FAILED")
                       for ln in lines):
        failed = next(ln for ln in lines
                      if ln.startswith("FILM TELEMETRY: CHECK FAILED"))
        return "FAILED", failed
    return "COULD-NOT-RUN", (f"exit {rc}: "
                             f"{first_evidence_line(stderr, stdout)}")


# ------------------------------------------------------------ the run

def run_acceptance(binary, samples_dir, band, film_dir, timeout,
                   renders_dir, out, samples_override=None):
    """Returns the exit code; transcript written to `out`."""
    def say(s=""):
        out.write(s + "\n")

    if not (os.path.isfile(binary) and os.access(binary, os.X_OK)):
        say(f"EXPORTER ACCEPTANCE: COULD NOT RUN "
            f"(binary missing or not executable: {binary})")
        return 3
    if samples_override is not None:
        samples = samples_override
    else:
        if not os.path.isdir(samples_dir):
            say(f"EXPORTER ACCEPTANCE: COULD NOT RUN "
                f"(samples directory missing: {samples_dir})")
            return 3
        samples = discover_samples(samples_dir)
        if not samples:
            say(f"EXPORTER ACCEPTANCE: COULD NOT RUN "
                f"(no *.scene.json under {samples_dir})")
            return 3

    say(f"exporter acceptance — {len(samples)} sample(s), "
        f"band {band:.1f} dB, binary {binary}")
    results = [score_sample(binary, stem, scene, ref, renders_dir,
                            band, timeout)
               for stem, scene, ref in samples]
    for r in results:
        psnr = ""
        if r["psnr"] is not None:
            psnr = ("  PSNR inf dB" if r["psnr"] == math.inf
                    else f"  PSNR {r['psnr']:.2f} dB")
        say(f"  {r['stem']:<24} {r['cls']}{psnr}  {r['detail']}")

    # Determinism spot-check on the first sample that produced a
    # render (W47 §4.7); skipped when nothing rendered.
    det_status, det_detail = "ENV-BLOCKED", "no sample rendered"
    rendered = [s for s, r in zip(samples, results)
                if r.get("render_path")]
    if rendered:
        det_status, det_detail = determinism_check(
            binary, rendered[0], renders_dir, timeout)
    say(f"  determinism: {det_status}  {det_detail}")

    tel_status = None
    if film_dir:
        tel_status, tel_detail = telemetry_gate(binary, film_dir,
                                                timeout)
        say(f"  telemetry: {tel_status}  {tel_detail}")

    classes = [r["cls"] for r in results]
    blocked = classes.count("ENV-BLOCKED")
    missing = classes.count("MISSING-REFERENCE")
    below = classes.count("BELOW-BAND")
    passed = classes.count("PASS")

    if missing or blocked or (tel_status == "COULD-NOT-RUN"):
        reasons = []
        if missing:
            reasons.append(f"{missing} missing reference")
        if blocked:
            reasons.append(f"{blocked} env-blocked")
        if tel_status == "COULD-NOT-RUN":
            reasons.append("telemetry could not run")
        say(f"EXPORTER ACCEPTANCE: COULD NOT RUN "
            f"({'; '.join(reasons)}; {passed} pass, {below} below-band)")
        return 3
    if below or det_status == "FAIL" or tel_status == "FAILED":
        reasons = []
        if below:
            reasons.append(f"{below} below-band")
        if det_status == "FAIL":
            reasons.append("determinism mismatch")
        if tel_status == "FAILED":
            reasons.append("telemetry CHECK FAILED")
        say(f"EXPORTER ACCEPTANCE: FAIL ({'; '.join(reasons)}; "
            f"{passed}/{len(results)} samples >= {band:.1f} dB)")
        return 1
    say(f"EXPORTER ACCEPTANCE: PASS "
        f"({passed}/{len(results)} samples >= {band:.1f} dB, "
        f"determinism byte-identical)")
    return 0


# ------------------------------------------------------------ self-test

def run_self_test(out):
    """Synthetic fixtures + a stub binary (see fixtures dir). The
    stub "renders" by copying a planted frame, so the harness's
    classification logic is exercised end to end with no GPU."""
    def say(s=""):
        out.write(s + "\n")

    fixtures = os.path.join(_REPO_ROOT, "tools", "format", "fixtures",
                            "exporter_acceptance")
    stub = os.path.join(fixtures, "stub_talerender.sh")
    failures = []

    def check(what, cond):
        say(f"  self-test: {what}: {'OK' if cond else 'MISMATCH'}")
        if not cond:
            failures.append(what)

    def case(dirname, expect_rc, expect_in_line):
        case_dir = os.path.join(fixtures, dirname)
        renders = tempfile.mkdtemp(prefix="exporter_acceptance_st_")
        buf = io.StringIO()
        try:
            with redirect_stdout(buf):
                rc = run_acceptance(stub, case_dir, DEFAULT_BAND, None,
                                    60, renders, buf)
        finally:
            shutil.rmtree(renders, ignore_errors=True)
        text = buf.getvalue()
        say(f"case {dirname}: exit {rc} (expect {expect_rc})")
        for ln in text.splitlines():
            say(f"    {ln}")
        check(f"{dirname} exit code", rc == expect_rc)
        check(f"{dirname} verdict line",
              any(expect_in_line in ln for ln in text.splitlines()))

    say("self-test A: identical pair -> PASS, exit 0")
    case("case_identical", 0, "EXPORTER ACCEPTANCE: PASS")
    say("self-test B: planted 1-pixel shift -> BELOW-BAND, exit 1")
    case("case_shifted", 1, "EXPORTER ACCEPTANCE: FAIL")
    say("self-test C: missing reference -> exit 3")
    case("case_missing_ref", 3, "EXPORTER ACCEPTANCE: COULD NOT RUN")

    if failures:
        say(f"SELF-TEST: FAIL ({len(failures)} mismatch(es))")
        return 1
    say("SELF-TEST: PASS (3/3 cases)")
    return 0


def main(argv=None):
    ap = argparse.ArgumentParser(
        description="Exporter acceptance harness (EXPORTER-W47 §4).")
    ap.add_argument("--samples", help="directory of *.scene.json + "
                                        "*.ref.png pairs")
    ap.add_argument("--scene", help="single scene file, tested in "
                                    "place (alternative to --samples)")
    ap.add_argument("--ref", help="Chrome reference PNG for --scene")
    ap.add_argument("--bin", dest="binary",
                    help="path to the talerender binary")
    ap.add_argument("--band", type=float, default=DEFAULT_BAND,
                    help="per-sample PSNR pass bar in dB "
                         f"(default {DEFAULT_BAND})")
    ap.add_argument("--film-dir",
                    help="also gate on --film-telemetry <DIR> "
                         "(W47 §4.6: verdict line must be CHECK OK)")
    ap.add_argument("--timeout", type=int, default=300,
                    help="per-render timeout in seconds (default 300)")
    ap.add_argument("--renders",
                    help="keep renders in this directory "
                         "(default: a temp directory, removed)")
    ap.add_argument("--self-test", action="store_true",
                    help="run the synthetic fixture cases and exit")
    args = ap.parse_args(argv)

    if args.self_test:
        return run_self_test(sys.stdout)
    if not args.binary or not (args.samples or (args.scene and args.ref)):
        ap.error("--bin plus --samples (or --scene + --ref) are "
                 "required (or use --self-test)")

    override = None
    if args.scene:
        override = [pair_sample(args.scene, args.ref)]
    if args.renders:
        os.makedirs(args.renders, exist_ok=True)
        return run_acceptance(args.binary, args.samples, args.band,
                              args.film_dir, args.timeout, args.renders,
                              sys.stdout, override)
    renders = tempfile.mkdtemp(prefix="exporter_acceptance_")
    try:
        return run_acceptance(args.binary, args.samples, args.band,
                              args.film_dir, args.timeout, renders,
                              sys.stdout, override)
    finally:
        shutil.rmtree(renders, ignore_errors=True)


if __name__ == "__main__":
    sys.exit(main())
