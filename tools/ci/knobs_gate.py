#!/usr/bin/env python3
"""Knobs gate — acceptance driver for the F4 quality knobs
(--res / --aa / --shadow / --draft; spec KNOBS-W43, patch pack
KNOBS-PATCH-W54). Stdlib only.

WHAT IT GATES
    The defaults identity law (KNOBS-W43 §3.2): a run with the
    knobs given explicitly at their defaults (--res 1 --aa 4
    --shadow 2048) must produce byte-identical frames to a run
    with no knob flags at all, and neither run may print a DRAFT
    tag. And the draft law (§3.4): a --draft run must differ
    from the full-quality frames, must come out at the scaled
    size, and must carry the exact tag

        [DRAFT res=0.5 aa=0 shadow=512 — NOT FOR DELIVERY]

    in its film header line, its progress lines, and its
    completion line — a draft must never be able to pass as a
    final.

LEGS
    A  identity    the three standing probe ranges
                   (byte_probes.DEFAULT_RANGES =
                   195-204, 795-814, 1195-1204) rendered twice —
                   no flags vs explicit defaults — compared with
                   byte_probes.run_probe (imported, not forked).
                   PASS requires BYTE PROBES: PASS (40/40).
    B  draft       frames 195-219 rendered with --draft (25
                   frames, so exactly one progress line prints).
                   Asserts: exit 0; the exact draft tag in the
                   header line and the completion line, a DRAFT
                   tag in the progress line; frames 195-204
                   DIFFER from leg A's plain frames (byte_probes
                   must FAIL them — identical output would mean
                   the knobs are not plumbed); the draft frame
                   size equals the spec's effective-size
                   arithmetic applied to the plain frame size
                   (1080x1920 -> 540x960).
    C  validation  bad knob values are refused before any GPU
                   work: exit 2, `error:` on stderr, and NOT the
                   `unknown argument` refusal (that would mean
                   the flags do not exist at all).

USAGE (CI shape, mirroring determinism.yml's pack discipline)
    knobs_gate.py BINARY [--film-dir DIR] [--work DIR]
    knobs_gate.py --self-test [--frames4 DIR]

    BINARY is the CI-built ./target/release/talerender; --film-dir
    defaults to `film` (the repo film layout, as render.yml uses);
    --work defaults to a fresh temp dir. The report is written to
    <work>/knobs-results.txt and echoed to stdout; the workflow
    greps the results file for the exact verdict line, the same
    way determinism.yml treats `DETERMINISM: PASS`.

EXIT CODES (house convention)
    0  KNOBS GATE: PASS
    1  KNOBS GATE: FAIL (a leg failed; the report says which)
    2  refusal — no verdict (missing binary/film dir, a run dir
       already populated, byte_probes unimportable). No verdict
       line is printed.

WHICH LEGS RUN WHERE
    Legs A/B/C drive a real binary through the film path, so
    they are CI-only until the lead lands the KNOBS-PATCH-W54
    implementation: no knobs binary exists before that, and this
    development VM cannot render the film's PBR path at all
    (its llvmpipe segfaults compiling shader_pbr.wgsl — see the
    render.yml header). The film legs run where film renders
    run: CI lavapipe (ubuntu-22.04) or the user's RADV PC.
    --self-test exercises only the comparison/tag/size logic
    against the on-disk frames4 corpus (the f021991 film frames
    in ~/workspace/film-final/frames4) and runs anywhere.

COST NOTE (no timing claims, just the shape)
    Leg A renders the 40 probe frames twice; leg B renders 25.
    Fresh output dirs are mandatory: the film driver skips
    frames that already exist (resume), which would silently
    poison a comparison — this gate refuses a populated run dir
    instead of cleaning it.
"""

import argparse
import importlib.util
import io
import os
import shutil
import struct
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))

# The exact draft tag for the --draft preset (KNOBS-W43 §2/§3.4:
# the tag lists the effective values; --draft = res 0.5, aa 0,
# shadow 512). The dash is U+2014, as specced.
DRAFT_TAG = "[DRAFT res=0.5 aa=0 shadow=512 — NOT FOR DELIVERY]"

# Probe ranges come from byte_probes (the standing determinism
# probes) — parsed with its own parser, compared with its own
# run_probe. Inclusive at both ends, like the evidence register.
COMPARE_RANGES_TEXT = "195-204"        # leg B comparison subset
DRAFT_FROM, DRAFT_TO = 195, 220        # [from, to) — 25 frames


class Refusal(Exception):
    """Exit-2 condition: the gate cannot run; no verdict."""


# ------------------------------------------------- byte_probes, imported

def load_byte_probes():
    """Import the sibling byte_probes.py by path (the house
    pattern, same as frame_hash_compare.py)."""
    path = os.path.join(HERE, "byte_probes.py")
    spec = importlib.util.spec_from_file_location("byte_probes", path)
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


# ------------------------------------------------- pure helpers

def spec_effective(value, res):
    """The spec's effective-size arithmetic for one dimension
    (KNOBS-W43 §3.1, as prescribed in KNOBS-PATCH-W54): res == 1
    returns the base size exactly; otherwise round(value * res),
    floor 2, then round down to even (encoder needs even)."""
    if res == 1.0:
        return value
    raw = int(round(value * res))
    return max(2, raw) & ~1


def png_size(path):
    """(width, height) from a PNG's IHDR — first 24 bytes only."""
    with open(path, "rb") as fh:
        head = fh.read(24)
    if len(head) < 24 or head[:8] != b"\x89PNG\r\n\x1a\n" \
            or head[12:16] != b"IHDR":
        raise Refusal(f"{path} is not a PNG (no IHDR in first 24 bytes)")
    return struct.unpack(">II", head[16:24])


def stdout_checks(stdout, draft):
    """The labelling law, as (ok, detail) checks over one run's
    stdout. For a draft run: the header line (`film: ...`) and
    the completion line (`film render complete: ...`) carry the
    exact tag, and every progress line (the `rendered,` lines
    that are not the completion line) carries a DRAFT tag. For a
    defaults run: no DRAFT anywhere. Returns a list of
    (name, ok, detail)."""
    lines = stdout.splitlines()
    header = next((l for l in lines if l.startswith("film: ")), None)
    completion = next(
        (l for l in lines if l.startswith("film render complete:")), None)
    progress = [l for l in lines
                if "rendered," in l and not l.startswith("film render")]
    checks = []
    if not draft:
        checks.append(("no DRAFT tag in defaults stdout",
                       "DRAFT" not in stdout,
                       "a defaults run must print exactly the pre-knob "
                       "lines (KNOBS-W43 §3.2)"))
        return checks
    checks.append(("header line present", header is not None,
                   "no line starts with 'film: '"))
    if header is not None:
        checks.append(("header carries the exact draft tag",
                       DRAFT_TAG in header, header.strip()))
    checks.append(("completion line present", completion is not None,
                   "no line starts with 'film render complete:'"))
    if completion is not None:
        checks.append(("completion repeats the draft tag",
                       DRAFT_TAG in completion, completion.strip()))
    checks.append(("at least one progress line printed",
                   len(progress) >= 1,
                   "the draft range is sized to force one "
                   "(25 frames, progress every 25)"))
    for l in progress:
        checks.append(("progress line carries a DRAFT tag",
                       "DRAFT" in l and "NOT FOR DELIVERY" in l,
                       l.strip()))
    return checks


# ------------------------------------------------- render driving

def fresh_dir(path):
    if os.path.isdir(path) and os.listdir(path):
        raise Refusal(
            f"run dir {path} is already populated — the film "
            f"driver would resume-skip those frames and poison "
            f"the comparison; refusing (pass a fresh --work)")
    os.makedirs(path, exist_ok=True)
    return path


def run_render(binary, film_dir, out_dir, frm, to, extra, report):
    cmd = [binary, "--film", film_dir, "--out", out_dir,
           "--from", str(frm), "--to", str(to)] + extra
    report(f"  $ {' '.join(cmd)}")
    proc = subprocess.run(cmd, capture_output=True, text=True)
    if proc.returncode != 0:
        tail = "\n".join((proc.stdout + proc.stderr).splitlines()[-5:])
        raise GateFail(f"render exited {proc.returncode}:\n{tail}")
    return proc.stdout


class GateFail(Exception):
    """A leg failed: the gate verdict is FAIL (exit 1)."""


# ------------------------------------------------- the legs

def leg_identity(binary, film_dir, work, bp, report):
    report("leg A — defaults identity (no flags vs "
           "--res 1 --aa 4 --shadow 2048)")
    ranges = bp.parse_ranges(bp.DEFAULT_RANGES)
    # Render spans for the same ranges (--to is exclusive):
    spans = []
    for text in bp.DEFAULT_RANGES.split(","):
        a, b = text.split("-")
        spans.append((int(a), int(b) + 1))  # --to is exclusive
    plain = fresh_dir(os.path.join(work, "plain"))
    defaults = fresh_dir(os.path.join(work, "defaults"))
    for frm, to in spans:
        out_a = run_render(binary, film_dir, plain, frm, to, [], report)
        out_b = run_render(binary, film_dir, defaults, frm, to,
                           ["--res", "1", "--aa", "4",
                            "--shadow", "2048"], report)
        for name, out in (("plain", out_a), ("defaults", out_b)):
            for cname, ok, detail in stdout_checks(out, draft=False):
                if not ok:
                    raise GateFail(f"leg A/{name}: {cname} — {detail}")
    sink = io.StringIO()
    rc = bp.run_probe(plain, defaults, ranges, bp.DEFAULT_PATTERN,
                      False, sink)
    report("  " + "\n  ".join(sink.getvalue().splitlines()))
    if rc != 0:
        raise GateFail("leg A: explicit-defaults frames differ from "
                       "no-flag frames (BYTE PROBES: FAIL)")
    report("leg A: PASS (defaults are byte-identical, stdout clean)")


def leg_draft(binary, film_dir, work, bp, report):
    report("leg B — draft differs, is scaled, and says so")
    plain = os.path.join(work, "plain")
    draft = fresh_dir(os.path.join(work, "draft"))
    out = run_render(binary, film_dir, draft, DRAFT_FROM, DRAFT_TO,
                     ["--draft"], report)
    for cname, ok, detail in stdout_checks(out, draft=True):
        report(f"  {'ok' if ok else 'FAIL'}: {cname}")
        if not ok:
            raise GateFail(f"leg B: {cname} — {detail}")
    sink = io.StringIO()
    try:
        rc = bp.run_probe(plain, draft,
                          bp.parse_ranges(COMPARE_RANGES_TEXT),
                          bp.DEFAULT_PATTERN, False, sink)
    except bp.InputError as e:
        raise GateFail(f"leg B: probe refused — {e}")
    report("  " + "\n  ".join(sink.getvalue().splitlines()))
    if rc == 0:
        raise GateFail("leg B: draft frames are byte-identical to "
                       "full quality — the knobs are not plumbed")
    pw, ph = png_size(os.path.join(
        plain, bp.frame_name(bp.DEFAULT_PATTERN, DRAFT_FROM)))
    dw, dh = png_size(os.path.join(
        draft, bp.frame_name(bp.DEFAULT_PATTERN, DRAFT_FROM)))
    want = (spec_effective(pw, 0.5), spec_effective(ph, 0.5))
    if (dw, dh) != want:
        raise GateFail(f"leg B: draft frame is {dw}x{dh}, spec "
                       f"arithmetic on {pw}x{ph} expects {want[0]}x"
                       f"{want[1]}")
    report(f"leg B: PASS (draft differs; {pw}x{ph} -> {dw}x{dh}; "
           f"tag in header, progress, completion)")


def leg_validation(binary, report):
    report("leg C — bad values refused at the CLI (exit 2)")
    cases = [
        ["--res", "1.5"], ["--res", "0"], ["--res", "-0.5"],
        ["--aa", "3"], ["--aa", "16"],
        ["--shadow", "1000"], ["--shadow", "0"],
        ["--res"], ["--aa"], ["--shadow"],
    ]
    for extra in cases:
        cmd = [binary, "--scene", "m1"] + extra
        proc = subprocess.run(cmd, capture_output=True, text=True)
        ok = (proc.returncode == 2
              and "error:" in proc.stderr
              and "unknown argument" not in proc.stderr)
        report(f"  {'ok' if ok else 'FAIL'}: {' '.join(extra)} "
               f"-> exit {proc.returncode}")
        if not ok:
            raise GateFail(
                f"leg C: {' '.join(extra)} gave exit "
                f"{proc.returncode}, stderr: {proc.stderr.strip()!r} "
                f"(want exit 2 with a value error, not an "
                f"unknown-argument refusal — that would mean the "
                f"flag does not exist)")
    report("leg C: PASS")


# ------------------------------------------------- self-test

def run_self_test(frames4):
    out = []

    def check(what, cond, detail=""):
        out.append(f"  {'ok' if cond else 'FAIL'}: {what}"
                   + (f" — {detail}" if detail and not cond else ""))
        return cond

    ok = True
    if not os.path.isdir(frames4):
        print(f"ERROR: frames4 corpus {frames4} not found — "
              f"self-test has no evidence to run on", file=sys.stderr)
        return 2
    bp = load_byte_probes()
    ranges = bp.parse_ranges(bp.DEFAULT_RANGES)

    sink = io.StringIO()
    rc = bp.run_probe(frames4, frames4, ranges, bp.DEFAULT_PATTERN,
                      False, sink)
    ok &= check("identity: frames4 vs itself passes byte_probes",
                rc == 0 and "BYTE PROBES: PASS (40/40" in sink.getvalue(),
                sink.getvalue().strip().splitlines()[-1]
                if sink.getvalue() else "no output")

    tmp = tempfile.mkdtemp(prefix="knobs-gate-selftest-")
    try:
        a_dir = os.path.join(tmp, "a")
        b_dir = os.path.join(tmp, "b")
        os.makedirs(a_dir)
        os.makedirs(b_dir)
        for n in range(195, 205):
            name = bp.frame_name(bp.DEFAULT_PATTERN, n)
            shutil.copy(os.path.join(frames4, name),
                        os.path.join(a_dir, name))
            shutil.copy(os.path.join(frames4, name),
                        os.path.join(b_dir, name))
        victim = os.path.join(b_dir, bp.frame_name(bp.DEFAULT_PATTERN, 200))
        with open(victim, "r+b") as fh:
            fh.seek(4096)
            byte = fh.read(1)
            fh.seek(4096)
            fh.write(bytes([byte[0] ^ 0xFF]))
        sink = io.StringIO()
        rc = bp.run_probe(a_dir, b_dir, bp.parse_ranges("195-204"),
                          bp.DEFAULT_PATTERN, False, sink)
        ok &= check("difference: one flipped byte fails byte_probes",
                    rc == 1 and "BYTE PROBES: FAIL" in sink.getvalue())
    finally:
        shutil.rmtree(tmp, ignore_errors=True)

    size = png_size(os.path.join(
        frames4, bp.frame_name(bp.DEFAULT_PATTERN, 195)))
    ok &= check("png_size reads frames4 frame 195 as 1080x1920",
                size == (1080, 1920), f"got {size}")
    ok &= check("spec arithmetic: 1080x1920 at res 0.5 -> 540x960",
                (spec_effective(1080, 0.5), spec_effective(1920, 0.5))
                == (540, 960))
    ok &= check("spec arithmetic: res 1 returns the base size "
                "exactly (even for an odd base)",
                spec_effective(1081, 1.0) == 1081
                and spec_effective(1081, 0.5) == 540)

    plain_out = (
        "renderer: llvmpipe | backend: Vulkan\n"
        "film: THE SOLDIER — 1811 frames @30fps, rendering 195..205 "
        "at 1080x1920 (clean PNG frames; decision B)\n"
        "film render complete: 10 rendered, 0 skipped (resume), "
        "range 195..205 · elapsed 00:31 · peak RSS 900.0 MB · cache\n")
    draft_out = (
        "renderer: llvmpipe | backend: Vulkan\n"
        "film: THE SOLDIER — 1811 frames @30fps, rendering 195..220 "
        "at 540x960 (clean PNG frames; decision B) "
        + DRAFT_TAG + "\n"
        "  frame 219 (t = 7.30s) — 25 rendered, 0 skipped · "
        "elapsed 00:12 · eta 00:00 " + DRAFT_TAG + "\n"
        "film render complete: 25 rendered, 0 skipped (resume), "
        "range 195..220 · elapsed 00:12 · peak RSS 900.0 MB · "
        "cache " + DRAFT_TAG + "\n")
    ok &= check("tag logic: defaults stdout is clean",
                all(c[1] for c in stdout_checks(plain_out, draft=False)))
    ok &= check("tag logic: drafted stdout passes every check",
                all(c[1] for c in stdout_checks(draft_out, draft=True)))
    ok &= check("tag logic: untagged film stdout fails draft checks",
                not all(c[1] for c in stdout_checks(plain_out, draft=True)))

    for line in out:
        print(line)
    print("SELF-TEST: " + ("PASS" if ok else "FAIL"))
    print("note: legs A/B/C drive a real binary and are CI-only "
          "(see module docstring); the self-test covers the "
          "comparison, size, and tag logic only.")
    return 0 if ok else 1


# ------------------------------------------------- main

def main(argv=None):
    parser = argparse.ArgumentParser(
        prog="knobs_gate.py",
        description="Acceptance gate for the F4 quality knobs "
                    "(defaults byte-identity + draft labelling). "
                    "Exit 0 PASS, 1 FAIL, 2 refusal (no verdict).")
    parser.add_argument("binary", nargs="?",
                        help="path to the built talerender binary")
    parser.add_argument("--film-dir", default="film",
                        help="film data dir (default: film)")
    parser.add_argument("--work", default=None,
                        help="scratch dir for renders + "
                             "knobs-results.txt (default: fresh "
                             "temp dir)")
    parser.add_argument("--self-test", action="store_true",
                        help="run the corpus self-test and exit")
    parser.add_argument("--frames4", default=os.path.expanduser(
        "~/workspace/film-final/frames4"),
        help="frames4 corpus dir for --self-test")
    args = parser.parse_args(argv)

    if args.self_test:
        return run_self_test(args.frames4)

    if not args.binary:
        print("ERROR: need BINARY (or --self-test)", file=sys.stderr)
        return 2
    if not os.path.isfile(args.binary):
        print(f"ERROR: binary {args.binary} not found", file=sys.stderr)
        return 2
    if not os.path.isdir(args.film_dir):
        print(f"ERROR: film dir {args.film_dir} not found",
              file=sys.stderr)
        return 2
    work = args.work or tempfile.mkdtemp(prefix="knobs-gate-")
    os.makedirs(work, exist_ok=True)

    lines = []

    def report(line=""):
        lines.append(line)
        print(line, flush=True)

    verdict = None
    try:
        bp = load_byte_probes()
    except Exception as e:  # import failure = no evidence, refuse
        print(f"ERROR: cannot import byte_probes: {e}", file=sys.stderr)
        return 2
    try:
        report(f"knobs gate: binary={args.binary} "
               f"film_dir={args.film_dir} work={work}")
        leg_identity(args.binary, args.film_dir, work, bp, report)
        leg_draft(args.binary, args.film_dir, work, bp, report)
        leg_validation(args.binary, report)
        verdict = "KNOBS GATE: PASS"
        rc = 0
    except Refusal as e:
        print(f"ERROR: {e}", file=sys.stderr)
        return 2
    except GateFail as e:
        report(f"FAIL: {e}")
        verdict = "KNOBS GATE: FAIL"
        rc = 1
    report(verdict)
    with open(os.path.join(work, "knobs-results.txt"), "w") as fh:
        fh.write("\n".join(lines) + "\n")
    return rc


if __name__ == "__main__":
    sys.exit(main())
