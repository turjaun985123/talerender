#!/usr/bin/env python3
"""report.py — TaleRender V2 WP6 performance reporting engine.

====================================================================
DELIVERY NOTE — V2 Batch 2, Seat 9 (WP6 performance reporting)
====================================================================
Files created: ONLY this file,
  tools/perf/lab/report.py
No existing file was edited. In particular
tools/perf/lab/bench_lab.py is read-only INPUT to this tool
(its record schema is the contract below, unchanged),
tools/perf/bench_report.py still owns the PERF-shaped
log-directory tables, and docs/runbook/PERF.md's tables stay
exactly as they are until genuine measurements exist — this
report complements them, it does not fill them.

INPUT SCHEMA — "talerender-bench/1" (bench_lab.py's output,
one JSON object per run; see that file's header for the full
definition). This tool reads, per record:
  schema          must be the literal "talerender-bench/1"
  label           run name
  binary_sha256   sha256 hex of the binary (rendered as a
                  12-character prefix in the runs table)
  git_head        commit the run was made at, or null
  adapter         {renderer_line, adapter_class} — the class
                  is taken VERBATIM from the record
                  ("lavapipe-software" / "hardware-or-other" /
                  "unknown"); this tool never re-derives or
                  upgrades a class from the renderer line
  frames          {count, range [A, B], sha256 {filename:
                  sha256 hex}} — count/range for the runs
                  table, the sha256 map for hash agreement
  timing          {total_s, frame_ms {mean, p50, p95, p99},
                  per_frame_ms, samples}
  peak_rss_kb     peak RSS in KiB, or null
Missing or null fields render as "—" (the master prompt §10
missing-data rule); no value is ever invented, interpolated,
or carried over from another run.

OUTPUT — one Markdown comparison report (stdout, or --out):
  * an HONESTY BANNER (see render_report / banner_lines):
    if any run is "lavapipe-software" the header states these
    are software results, not hardware; a line reading
    "GPU rows: PENDING-PC" appears UNLESS some record's
    adapter_class is "hardware-or-other", in which case the
    GPU line instead reads "GPU rows: unverified hardware
    label" — a record label, not hardware validation. The
    banner lists the adapter classes present, verbatim.
  * a runs table: label, adapter_class, frames, total_s,
    frame_ms mean/p50/p95/p99, peak_rss_kb, binary sha prefix,
    git head — one row per input record, in argument order.
  * a per-frame-hash agreement section: for every pair of
    runs over the SAME frames.range, the frames compared
    (filenames present in both sha256 maps), how many are
    identical, and the differing frame indices — the lab's
    determinism view. Pairs are only ever formed inside one
    range; runs over different ranges are never compared.

WIRING (lead, later, in .github/workflows/bench.yml — this
seat does not edit the workflow): after the bench_lab legs
have written their records, add ONE reporting step and
upload ONLY the text outputs (quota law — never frame PNGs;
their sha256 maps in the .json records are the provenance):

  - name: Bench lab report
    run: |
      python3 tools/perf/lab/report.py \
        bench-out/lab/*.json \
        --out bench-out/lab/report.md
  # artifact paths: bench-out/lab/*.json, report.md

On the user's PC the same command over the RADV bench_lab
records produces the first report whose GPU line can read
"unverified hardware label"; until a hardware-or-other
record exists in the set, every report carries PENDING-PC.

USAGE
  report.py RECORD.json [RECORD.json ...] [--out REPORT.md]
  report.py --self-test

LIMITATIONS
  * No history store: the report is a pure function of the
    records named on the command line. Nothing is persisted,
    accumulated, or compared against runs not passed in;
    historical storage / trending is future work.
  * No Chrome legs data yet: the four WP6 configurations
    (Chrome GPU / Chrome software / TaleRender GPU /
    TaleRender software) are only comparable here once
    someone produces bench records for those legs — records
    for those legs arrive when someone runs them. This tool
    neither fabricates Chrome rows nor reshapes Chrome logs;
    bench_report.py remains the PERF §6 log-directory tool.
  * Hash agreement is over frame FILENAMES present in both
    records of a pair; frames only in one record are counted
    as not compared (and noted), never as differing.
  * "hardware-or-other" is bench_lab's label for a renderer
    line naming neither llvmpipe nor lavapipe. This report
    repeats it verbatim and never treats it as validation:
    hardware validation belongs to the PC session / the
    validation wave.

Exit codes (house convention): 0 = report written / self-test
passed; 1 = self-test failed; 2 = usage / input error (one
`ERROR:` line on stderr, no report).
Stdlib only.
"""

import hashlib
import itertools
import json
import re
import sys
from pathlib import Path

SCHEMA = "talerender-bench/1"
SHA_PREFIX_LEN = 12
MISSING = "—"

_FRAME_INDEX_RE = re.compile(r"(\d+)")


class InputError(Exception):
    """Bad input. Reported as one named line on stderr, exit 2."""


# --- loading ---------------------------------------------------------------

def load_record(path):
    """Load and schema-check one talerender-bench/1 record.
    Same refusal rule as bench_lab.load_record."""
    try:
        with open(path, encoding="utf-8") as fh:
            record = json.load(fh)
    except OSError as e:
        raise InputError(f"cannot read record {path} ({e})")
    except json.JSONDecodeError as e:
        raise InputError(f"not a JSON record: {path} ({e})")
    if not isinstance(record, dict) or record.get("schema") != SCHEMA:
        got = record.get("schema") if isinstance(record, dict) else None
        raise InputError(
            f"{path} is not a {SCHEMA} record (schema field: {got!r})")
    return record


# --- formatting (missing/null -> "—", never invented) ----------------------

def _fmt(x, digits=2):
    """Float with `digits` decimals, or "—" when missing."""
    if x is None:
        return MISSING
    try:
        return f"{float(x):.{digits}f}"
    except (TypeError, ValueError):
        return MISSING


def _fmt_int(x):
    """Plain integer (no thousands separators, so values stay
    machine-greppable), or "—" when missing."""
    if x is None:
        return MISSING
    try:
        return str(int(x))
    except (TypeError, ValueError):
        return MISSING


def _cell(value):
    """A verbatim string cell, or "—" when missing/empty.
    Pipes and newlines are neutralised so one record cannot
    break the table it is rendered into."""
    if value is None:
        return MISSING
    text = str(value)
    if not text.strip():
        return MISSING
    return text.replace("|", "\\|").replace("\n", " ")


def _sha_prefix(value):
    """First SHA_PREFIX_LEN characters of a sha256 hex string,
    in backticks, or "—" when missing."""
    if value is None:
        return MISSING
    text = str(value).strip()
    if not text:
        return MISSING
    return f"`{text[:SHA_PREFIX_LEN]}`"


def _adapter_class(record):
    """The record's adapter_class, verbatim, or None. Never
    re-derived from the renderer line (see DELIVERY NOTE)."""
    adapter = record.get("adapter")
    if isinstance(adapter, dict):
        value = adapter.get("adapter_class")
        if isinstance(value, str) and value.strip():
            return value.strip()
    return None


def _frame_range(record):
    """frames.range as an (A, B) tuple of ints, or None when
    absent or malformed (such a run forms no agreement pair)."""
    frames = record.get("frames")
    if isinstance(frames, dict):
        rng = frames.get("range")
        if (isinstance(rng, (list, tuple)) and len(rng) == 2
                and all(isinstance(v, int) and not isinstance(v, bool)
                        for v in rng)):
            return (rng[0], rng[1])
    return None


def _frame_hashes(record):
    """frames.sha256 as a {filename: hash} dict ({} if absent)."""
    frames = record.get("frames")
    if isinstance(frames, dict):
        hashes = frames.get("sha256")
        if isinstance(hashes, dict):
            return hashes
    return {}


# --- honesty banner ----------------------------------------------------------

def banner_lines(records):
    """The honesty banner as a list of Markdown lines. Pure
    function of the adapter classes present, verbatim:

    * any "lavapipe-software" -> the header states software
      results, not hardware;
    * no "hardware-or-other" -> a "GPU rows: PENDING-PC" line;
    * some "hardware-or-other" -> the GPU line instead reads
      "GPU rows: unverified hardware label" (and the exact
      PENDING-PC line does not appear);
    * any "unknown" (or missing class) -> a line stating the
      renderer cannot be named for that run/those runs.
    """
    classes = [_adapter_class(r) for r in records]
    present = sorted({c for c in classes if c is not None})
    lines = []
    if "lavapipe-software" in classes:
        lines += [
            "> ⚠ **SOFTWARE RESULTS — NOT HARDWARE.** At least one "
            "run in this report is `lavapipe-software` (lavapipe / "
            "llvmpipe, a CPU software Vulkan device). Its numbers "
            "are software results and must never be quoted as "
            "hardware performance (PERF.md honesty rule).", ""]
    if present:
        listed = ", ".join(f"`{c}`" for c in present)
        lines += [f"**Adapter classes present:** {listed}", ""]
    else:
        lines += ["**Adapter classes present:** —", ""]
    if "hardware-or-other" in classes:
        lines += [
            "**GPU rows: unverified hardware label** — at least one "
            "record is labelled `hardware-or-other`. That is the "
            "record's own label, repeated verbatim; it is **not** "
            "hardware validation, which belongs to the PC session "
            "/ validation wave. Any `lavapipe-software` run in this "
            "set remains a software result.", ""]
    else:
        lines += [
            "**GPU rows: PENDING-PC** — no record in this set is "
            "labelled `hardware-or-other`; every GPU row stays "
            "pending a run on the user's PC (RADV).", ""]
    if any(c is None or c == "unknown" for c in classes):
        lines += [
            "> ⚠ **UNNAMED RENDERER.** At least one record carries "
            "adapter class `unknown` (or no class): its renderer "
            "cannot be named from the record, so its numbers must "
            "not be quoted until the run is repeated with the "
            "renderer line captured.", ""]
    return lines


# --- per-frame hash agreement ------------------------------------------------

def frame_index(filename):
    """The frame index from a frame filename: the LAST run of
    digits before the extension (frame_00037.png -> 37), as an
    int; the filename itself when it carries no digits."""
    stem = str(filename).rsplit(".", 1)[0]
    matches = _FRAME_INDEX_RE.findall(stem)
    if matches:
        return int(matches[-1])
    return str(filename)


def compare_pair(a, b):
    """Agreement between two records' frame-hash maps.
    Compared = filenames present in BOTH maps; identical =
    compared filenames whose hashes are equal; differing =
    compared filenames whose hashes differ, as sorted frame
    indices. Filenames only in one map are not compared."""
    hashes_a = _frame_hashes(a)
    hashes_b = _frame_hashes(b)
    common = sorted(set(hashes_a) & set(hashes_b))
    differing = [name for name in common if hashes_a[name] != hashes_b[name]]
    differing_indices = sorted(
        (frame_index(name) for name in differing),
        key=lambda v: (0, v) if isinstance(v, int) else (1, str(v)))
    return {
        "label_a": a.get("label"),
        "label_b": b.get("label"),
        "range": _frame_range(a),
        "compared": len(common),
        "identical": len(common) - len(differing),
        "differing": len(differing),
        "differing_indices": differing_indices,
        "only_a": len(set(hashes_a) - set(hashes_b)),
        "only_b": len(set(hashes_b) - set(hashes_a)),
    }


def agreement_pairs(records):
    """All compare_pair results: pairs formed ONLY between
    records sharing the same frames.range, in argument order."""
    by_range = {}
    for idx, record in enumerate(records):
        rng = _frame_range(record)
        if rng is not None:
            by_range.setdefault(rng, []).append(idx)
    pairs = []
    for rng in sorted(by_range):
        for i, j in itertools.combinations(by_range[rng], 2):
            pairs.append(compare_pair(records[i], records[j]))
    return pairs


# --- rendering ---------------------------------------------------------------

def render_runs_table(records):
    lines = [
        "## Runs",
        "",
        "| label | adapter_class | frames | total_s | frame_ms mean "
        "| frame_ms p50 | frame_ms p95 | frame_ms p99 | peak_rss_kb "
        "| binary sha256 (prefix) | git head |",
        "|---|---|---|---|---|---|---|---|---|---|---|",
    ]
    for record in records:
        frames = record.get("frames") if isinstance(
            record.get("frames"), dict) else {}
        timing = record.get("timing") if isinstance(
            record.get("timing"), dict) else {}
        fms = timing.get("frame_ms") if isinstance(
            timing.get("frame_ms"), dict) else {}
        lines.append(
            f"| {_cell(record.get('label'))} "
            f"| {_cell(_adapter_class(record))} "
            f"| {_fmt_int(frames.get('count'))} "
            f"| {_fmt(timing.get('total_s'))} "
            f"| {_fmt(fms.get('mean'))} "
            f"| {_fmt(fms.get('p50'))} "
            f"| {_fmt(fms.get('p95'))} "
            f"| {_fmt(fms.get('p99'))} "
            f"| {_fmt_int(record.get('peak_rss_kb'))} "
            f"| {_sha_prefix(record.get('binary_sha256'))} "
            f"| {_cell(record.get('git_head'))} |")
    lines.append("")
    return lines


def render_agreement(records):
    lines = ["## Per-frame hash agreement", ""]
    pairs = agreement_pairs(records)
    if not pairs:
        lines += [
            "No two runs share the same frame range — no agreement "
            "comparison is available, and none is invented. "
            "(Runs are only compared over an identical "
            "`frames.range`.)",
            "",
        ]
        return lines
    lines += [
        "Frames are compared by filename over the sha256 maps of "
        "runs sharing the same `frames.range`: *compared* = "
        "filenames present in both runs, *identical* = compared "
        "frames whose hashes match. This is the lab's determinism "
        "view — a differing hash is a defect report, not a timing "
        "footnote.",
        "",
    ]
    for pair in pairs:
        a, b = pair["range"]
        lines += [
            f"### Range {a}..{b} — `{pair['label_a']}` vs "
            f"`{pair['label_b']}`",
            "",
            "| frames compared | identical | differing | "
            "differing indices |",
            "|---|---|---|---|",
        ]
        if pair["differing_indices"]:
            indices = ", ".join(str(v) for v in pair["differing_indices"])
        elif pair["compared"]:
            indices = "none"
        else:
            indices = MISSING
        lines.append(
            f"| {pair['compared']} | {pair['identical']} | "
            f"{pair['differing']} | {indices} |")
        lines.append("")
        if pair["only_a"] or pair["only_b"]:
            lines += [
                f"Not compared (present in only one run): "
                f"{pair['only_a']} frame(s) only in "
                f"`{pair['label_a']}`, {pair['only_b']} frame(s) "
                f"only in `{pair['label_b']}`. Unshared frames are "
                "never counted as differing.",
                "",
            ]
    return lines


def render_report(records):
    """The full comparison report. A pure function of the
    record dicts, in argument order."""
    lines = ["# TaleRender bench comparison — lab report", ""]
    lines += banner_lines(records)
    lines += render_runs_table(records)
    lines += render_agreement(records)
    lines += [
        "---",
        "",
        f"Provenance: generated by `tools/perf/lab/report.py` from "
        f"{len(records)} `talerender-bench/1` record(s) written by "
        "`tools/perf/lab/bench_lab.py`. Missing or null fields are "
        f"shown as `{MISSING}` and are never invented. This report "
        "complements docs/runbook/PERF.md; it does not fill its "
        "tables. No history is stored — only the records named on "
        "the command line are compared.",
        "",
    ]
    return "\n".join(lines)


# --- self-test ---------------------------------------------------------------

def _synthetic_record(label, adapter_class, renderer_line, hashes,
                      differing=()):
    """One fabricated talerender-bench/1 record over frames
    0..40. `hashes` is the base {index: hash}; `differing` lists
    indices whose hash is replaced with a different value."""
    sha_map = {}
    for n in range(40):
        digest = hashes[n]
        if n in differing:
            digest = hashlib.sha256(
                f"different frame {n}\n".encode()).hexdigest()
        sha_map[f"frame_{n:05d}.png"] = digest
    return {
        "schema": SCHEMA,
        "label": label,
        "binary_path": "synthetic/talerender",
        "binary_sha256": "a" * 64,
        "git_head": "0123456789abcdef0123456789abcdef01234567",
        "adapter": {
            "renderer_line": renderer_line,
            "adapter_class": adapter_class,
        },
        "frames": {"count": 40, "range": [0, 40], "sha256": sha_map},
        "timing": {
            "total_s": 12.5,
            "frame_ms": {"mean": 40.0, "p50": 30.0,
                         "p95": 100.0, "p99": 100.0},
            "per_frame_ms": {},
            "samples": 40,
        },
        "peak_rss_kb": 123456,
        "paint_cache": None,
        "paint_cache_arg": None,
        "exit_code": 0,
    }


def run_self_test(out):
    """Synthetic proof. Fabricates two records over the same
    40-frame range — the first lavapipe-software, the second
    identical except for exactly 3 differing frame hashes
    (indices 7, 19, 33) — and verifies the runs table renders,
    the agreement math is 37 identical of 40 compared with the
    right differing indices, the honesty-banner rules fire in
    both the software-only and hardware-labelled sets, and
    missing/null fields render as "—". Nothing is read from
    disk, no binary or git is invoked."""
    checks = []

    def check(name, ok, detail=""):
        checks.append((name, ok))
        out.write(f"  [{'PASS' if ok else 'FAIL'}] {name}"
                  + (f" — {detail}" if detail and not ok else "")
                  + "\n")

    base_hashes = {
        n: hashlib.sha256(f"frame {n}\n".encode()).hexdigest()
        for n in range(40)
    }
    lavapipe_line = (
        "renderer: llvmpipe (LLVM 15.0.7, 256 bits) | backend: "
        "Vulkan | device type: Cpu | driver: llvmpipe 0.0.1")
    rec_a = _synthetic_record(
        "selftest-lavapipe", "lavapipe-software", lavapipe_line,
        base_hashes)
    rec_b = _synthetic_record(
        "selftest-second", "lavapipe-software", lavapipe_line,
        base_hashes, differing=(7, 19, 33))

    report = render_report([rec_a, rec_b])

    # Runs table renders, with the records' own values.
    check("runs table names both labels",
          "selftest-lavapipe" in report and "selftest-second" in report)
    check("runs table carries adapter_class verbatim",
          "lavapipe-software" in report)
    check("runs table carries frames / total_s / frame_ms stats",
          "| 40 | 12.50 | 40.00 | 30.00 | 100.00 | 100.00 |" in report,
          "expected row fragment not found")
    check("runs table carries peak_rss_kb",
          "123456" in report)
    check("runs table carries binary sha prefix (12 chars)",
          "`aaaaaaaaaaaa`" in report)
    check("runs table carries git head",
          "0123456789abcdef0123456789abcdef01234567" in report)

    # Agreement math: 40 compared, 37 identical, 3 differing.
    pairs = agreement_pairs([rec_a, rec_b])
    check("exactly one agreement pair over the shared range",
          len(pairs) == 1, f"got {len(pairs)}")
    if pairs:
        pair = pairs[0]
        check("agreement compared == 40", pair["compared"] == 40,
              f"got {pair['compared']}")
        check("agreement identical == 37 (40 - 3 differing)",
              pair["identical"] == 37, f"got {pair['identical']}")
        check("agreement differing == 3", pair["differing"] == 3,
              f"got {pair['differing']}")
        check("differing indices are exactly [7, 19, 33]",
              pair["differing_indices"] == [7, 19, 33],
              f"got {pair['differing_indices']}")
    check("agreement section renders the 40 / 37 / 3 counts",
          "| 40 | 37 | 3 | 7, 19, 33 |" in report)
    check("identical records agree 40/40 with no differing indices",
          agreement_pairs([rec_a, rec_a])[0]["identical"] == 40
          and agreement_pairs([rec_a, rec_a])[0]["differing"] == 0)
    other_range = _synthetic_record(
        "selftest-other-range", "lavapipe-software", lavapipe_line,
        base_hashes)
    other_range["frames"]["range"] = [40, 80]
    check("different ranges form no agreement pair",
          agreement_pairs([rec_a, other_range]) == [])

    # Honesty banner: software set.
    check("software banner states software results, not hardware",
          "SOFTWARE RESULTS — NOT HARDWARE" in report)
    check("banner lists adapter classes present, verbatim",
          "**Adapter classes present:** `lavapipe-software`" in report)
    check("PENDING-PC line fires when no hardware-or-other record",
          "**GPU rows: PENDING-PC**" in report)
    check("unverified-hardware line does NOT fire for software set",
          "unverified hardware label" not in report)

    # Honesty banner: hardware-labelled set (a clone of rec_b
    # whose class is hardware-or-other; the pair logic above is
    # unaffected — only the banner is under test here).
    rec_hw = json.loads(json.dumps(rec_b))
    rec_hw["label"] = "selftest-hardware-labelled"
    rec_hw["adapter"] = {
        "renderer_line": (
            "renderer: AMD Radeon Vega 7 (RADV RENOIR) | backend: "
            "Vulkan | device type: IntegratedGpu | driver: radv"),
        "adapter_class": "hardware-or-other",
    }
    hw_report = render_report([rec_a, rec_hw])
    check("hardware-labelled set says 'unverified hardware label'",
          "**GPU rows: unverified hardware label**" in hw_report)
    check("PENDING-PC line does NOT fire when hardware-or-other present",
          "**GPU rows: PENDING-PC**" not in hw_report)
    check("mixed set still states software results, not hardware",
          "SOFTWARE RESULTS — NOT HARDWARE" in hw_report)
    check("mixed banner lists both classes verbatim",
          "`hardware-or-other`" in hw_report
          and "`lavapipe-software`" in hw_report)

    # Missing-data handling: nulls render as "—", never invented.
    bare = _synthetic_record(
        "selftest-bare", "lavapipe-software", lavapipe_line, base_hashes)
    bare["binary_sha256"] = None
    bare["git_head"] = None
    bare["peak_rss_kb"] = None
    bare["timing"] = {
        "total_s": None,
        "frame_ms": {"mean": None, "p50": None,
                     "p95": None, "p99": None},
        "per_frame_ms": {}, "samples": 0,
    }
    bare_report = render_report([bare])
    bare_row = next(
        (ln for ln in bare_report.splitlines()
         if "selftest-bare" in ln), "")
    check("null fields render as — in the runs row",
          bare_row.count(MISSING) >= 6, f"row: {bare_row}")
    check("null binary sha / git head are not invented",
          "aaaaaaaaaaaa" not in bare_row
          and "0123456789abcdef" not in bare_row)

    failed = sum(1 for _name, ok in checks if not ok)
    passed = len(checks) - failed
    if failed:
        out.write(f"SELF-TEST: FAIL ({passed} passed, {failed} failed)\n")
        return 1
    out.write(f"SELF-TEST: PASS ({passed} passed, 0 failed)\n")
    return 0


# --- entry -------------------------------------------------------------------

USAGE = (
    "usage:\n"
    "  report.py RECORD.json [RECORD.json ...] [--out REPORT.md]\n"
    "  report.py --self-test")


def main(argv=None):
    argv = list(sys.argv[1:] if argv is None else argv)
    try:
        if not argv:
            raise InputError("no records given\n" + USAGE)
        if argv[0] in ("-h", "--help"):
            print(USAGE)
            return 0
        if "--self-test" in argv or "self-test" in argv:
            if len(argv) != 1:
                raise InputError("--self-test takes no other arguments")
            return run_self_test(sys.stdout)
        paths = []
        out_path = None
        i = 0
        while i < len(argv):
            arg = argv[i]
            if arg == "--out":
                if i + 1 >= len(argv):
                    raise InputError("--out needs a value")
                out_path = argv[i + 1]
                i += 2
                continue
            if arg.startswith("--"):
                raise InputError(f"unknown option {arg!r}\n" + USAGE)
            paths.append(arg)
            i += 1
        if not paths:
            raise InputError("no records given\n" + USAGE)
        records = [load_record(p) for p in paths]
        report = render_report(records)
        if out_path is not None:
            Path(out_path).write_text(report, encoding="utf-8")
            print(f"report: wrote {out_path}")
        else:
            sys.stdout.write(report)
        return 0
    except InputError as e:
        print(f"ERROR: {e}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
