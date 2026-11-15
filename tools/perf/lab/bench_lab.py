#!/usr/bin/env python3
"""bench_lab.py — TaleRender V2 WP6 performance-laboratory runner.

====================================================================
DELIVERY NOTE — V2 Batch 1, Seat 6 (WP6 perf-lab infrastructure)
====================================================================
Files created: ONLY this file,
  tools/perf/lab/bench_lab.py
No existing file was edited (bench_report.py, vram_report.py,
tools/ci/byte_probes.py and .github/workflows/bench.yml are
untouched and NOT duplicated: bench_report.py still owns the
PERF-shaped multi-run tables, byte_probes.py still owns
byte-identity verdicts; this tool owns the single-run,
machine-readable bench RECORD that Batch 2's WP6 reporting
engine, tools/perf/lab/report.py, will consume).

Contract: docs/integration/V2-CONTRACTS.md §5 seat 6 and the
master prompt §10 (WP6): build the tooling, do NOT run the
hardware benchmark campaign, do NOT fabricate or prepopulate
results. No real benchmark was run for this delivery (no
binary exists locally); the synthetic --self-test is the proof.
GPU rows stay PENDING-PC: this tool labels a run from its own
`renderer:` line and never fills a hardware row from a
software run.

RECORD SCHEMA — "talerender-bench/1" (one JSON object):
  schema          the literal string "talerender-bench/1"
  label           the --label name (also the file stem)
  binary_path     the --binary path as given
  binary_sha256   sha256 hex of the binary file
  git_head        `git rev-parse HEAD` resolved at run time
                  from the working directory / binary directory,
                  or null when unresolvable (never guessed)
  adapter         {renderer_line, adapter_class}:
                    renderer_line — the first `renderer:` (or
                      `adapter:`) line of the run log, verbatim,
                      or null when the log carries none
                    adapter_class — HONESTY RULE: exactly
                      "lavapipe-software" when renderer_line
                      mentions llvmpipe or lavapipe (any case),
                      else "hardware-or-other" for any other
                      present renderer line, else "unknown"
                      when there is no renderer line. A record
                      may never claim hardware from a software
                      renderer line, and "hardware-or-other" is
                      a label, not a validation claim.
  frames          {count, range, sha256}:
                    count — number of frame PNGs actually found
                      in the run's frames directory (never the
                      requested range length by assumption)
                    range — [A, B] from --frames A..B, the film
                      driver's [--from, --to) range
                    sha256 — map frame filename -> sha256 hex,
                      one entry per produced frame PNG
  timing          {total_s, frame_ms, per_frame_ms, samples}:
                    total_s — wall time in seconds: GNU
                      `time -v` elapsed when the wrapper ran,
                      else this tool's own monotonic wall clock
                    frame_ms — {mean, p50, p95, p99} over the
                      per-frame timing lines, each null when
                      there are no timing lines (never invented)
                    per_frame_ms — map frame index (string) ->
                      milliseconds, as parsed from the log
                    samples — number of per-frame timing lines
  peak_rss_kb     peak resident set size in KiB from
                  /usr/bin/time -v ("Maximum resident set size
                  (kbytes)"); fallback: the completion line's
                  runlog `peak RSS <MiB> MB` x 1024; else null
  paint_cache     the LAST `paint-cache:` stats line of the
                  log, verbatim, or null when absent
  paint_cache_arg the --paint-cache value (the
                  TALERENDER_PAINT_CACHE env the run used),
                  or null for the binary's default
  exit_code       the render process exit code, or null when
                  the record was assembled without a process

Percentiles are NEAREST-RANK, documented here as the schema's
definition: sort the samples ascending; the p-th percentile is
the sample at rank ceil(p/100 * N) (1-based, clamped to
[1, N]). The mean is the plain arithmetic mean. An empty
sample set yields null for mean and every percentile.

PER-FRAME TIMING LINES. Frame times come from the log lines
the binary emits under TALERENDER_FRAME_TIMING=1
(src/runlog.rs `frame_timing_enabled`). At this HEAD the
switch exists but its emission into the film loop is still
the lead's wiring (RUNLOG-W09.md §2e: "the line format is
the lead's call"; STAGE1-SCOPE item 4). The parser therefore
accepts the plausible documented shapes and ignores
everything else — in particular the 25-frame progress lines
(`  frame N (t = T s) — D rendered, S skipped · ...`) and
the completion line are NOT per-frame times:
  frame 123: 45.67 ms        frame 123 — 45.67 ms
  frame 123 45.67 ms         frame 123: 45.67ms
  frame-time: frame=123 ms=45.67
  frame_time frame 123 45.67 ms
If a binary emits none of these, timing.frame_ms is all null
and the summary says so; total_s / peak_rss_kb still record.

WIRING (lead, later, in .github/workflows/bench.yml — this
seat does not edit the workflow): after the existing Build
step, add one step per leg, mirroring the existing legs, and
upload ONLY the text record files (quota law — never the
frame PNGs; their sha256 map in the .json is the provenance):

  - name: Bench lab leg A — cap default
    run: |
      python3 tools/perf/lab/bench_lab.py run \
        --binary ./target/release/talerender \
        --out bench-out/lab --label ci-lavapipe-default \
        --frames 0..100
  - name: Bench lab leg B — paint cache 24
    run: |
      python3 tools/perf/lab/bench_lab.py run \
        --binary ./target/release/talerender \
        --out bench-out/lab --label ci-lavapipe-cache24 \
        --frames 0..100 --paint-cache 24
  # artifact paths: bench-out/lab/*.json, *.md, *.log, *.time

On the user's PC the same `run` command with the RADV binary
produces the hardware record; until then every GPU row in
every downstream report stays PENDING-PC.

USAGE
  bench_lab.py run --binary PATH --out DIR --label NAME
                   --frames A..B [--paint-cache N]
                   [--film DIR]      (default "film", the
                                      bench.yml repo-root shape)
  bench_lab.py record RECORD.json [--out REPORT.md]
                   re-render the .md from an existing .json
                   record (JSON only; nothing is re-measured)
  bench_lab.py --self-test
                   synthetic fake run: record assembly,
                   percentile math, lavapipe labelling

`run` executes:
  /usr/bin/time -v -o <out>/<label>.time \
    <binary> --film <film> --out <out>/<label>.frames \
      --from A --to B
with TALERENDER_FRAME_TIMING=1 (and TALERENDER_PAINT_CACHE=N
when --paint-cache is given), captures the combined output to
<out>/<label>.log, hashes every frame_*.png produced, and
writes <out>/<label>.json + <out>/<label>.md. The frames
directory must be fresh: if it already holds frame PNGs the
driver would resume-skip them and the timing would be
invalid (PERF §3), so the run is refused instead. When
/usr/bin/time is unavailable the binary runs unwrapped:
total_s falls back to the monotonic wall clock and
peak_rss_kb to the completion-line self-report or null.

LIMITATIONS
  * Tooling only — no benchmark campaign, no Chrome legs, no
    cross-renderer comparison, no history/aggregation (that
    is Batch 2 WP6 report.py, consuming this schema).
  * Peak RSS / wall provenance is GNU time -v on Linux;
    off that path the fallbacks above apply and are what the
    record can honestly carry.
  * git_head is best-effort via the git CLI at run time;
    unresolvable -> null. The --self-test never invokes git,
    a binary, or the network: it assembles a record from a
    synthetic fake run directory + timing log in a temp dir.
  * A renderer line naming neither llvmpipe nor lavapipe is
    labelled "hardware-or-other" per the schema rule above;
    that label alone is not hardware validation (the
    validation wave / PC session owns that verdict).

Exit codes (house convention): 0 = run recorded / record
re-rendered / self-test passed; 1 = the render exited
non-zero (the record is still written, with exit_code set)
or the self-test failed; 2 = usage / input error (one
`ERROR:` line on stderr, no record).
Stdlib only.
"""

import hashlib
import json
import math
import os
import re
import subprocess
import sys
import tempfile
import time
from pathlib import Path

SCHEMA = "talerender-bench/1"

# --- line grammars ---------------------------------------------------------

RENDERER_RE = re.compile(r"^\s*(?:renderer|adapter):\s*.+$")
PAINT_CACHE_RE = re.compile(r"paint-cache:.*$")
TIME_V_ELAPSED_RE = re.compile(
    r"Elapsed \(wall clock\) time \(h:mm:ss or m:ss\):\s*([0-9:.]+)")
TIME_V_RSS_RE = re.compile(r"Maximum resident set size \(kbytes\):\s*(\d+)")
COMPLETION_RSS_RE = re.compile(r"peak RSS\s+([\d.]+)\s*MB")

# Per-frame timing shapes (see the header note). Each pattern
# captures (frame_index, milliseconds). Progress lines carry
# "(t = " and "rendered," and are excluded before these run.
TIMING_RES = [
    re.compile(r"frame[-_ ]time:\s*frame=(\d+)\s+ms=([\d.]+)"),
    re.compile(r"frame[-_ ]time\s+frame\s+(\d+)\s+([\d.]+)\s*ms\b"),
    re.compile(r"^\s*frame\s+(\d+)\s*:\s*([\d.]+)\s*ms\b"),
    re.compile(r"^\s*frame\s+(\d+)\s*[—–-]\s*([\d.]+)\s*ms\b"),
    re.compile(r"^\s*frame\s+(\d+)\s+([\d.]+)\s*ms\b"),
]

LABEL_RE = re.compile(r"^[A-Za-z0-9][A-Za-z0-9._-]*$")
FRAMES_RE = re.compile(r"^(\d+)\.\.(\d+)$")


class InputError(Exception):
    """Bad input. Reported as one named line on stderr, exit 2."""


# --- pure helpers ----------------------------------------------------------

def parse_frame_range(text):
    """'A..B' -> (A, B), the film driver's [--from, --to) range."""
    m = FRAMES_RE.match(text.strip())
    if not m:
        raise InputError(f"bad --frames {text!r} (need A..B, e.g. 0..100)")
    a, b = int(m.group(1)), int(m.group(2))
    if b <= a:
        raise InputError(
            f"bad --frames {text!r} (need B > A; the film driver "
            "treats --to 0 as through-the-end, which a bench "
            "record must not leave unbounded)")
    return a, b


def adapter_class(renderer_line):
    """The honesty rule, exactly as the schema states it."""
    if renderer_line is None:
        return "unknown"
    low = renderer_line.lower()
    if "llvmpipe" in low or "lavapipe" in low:
        return "lavapipe-software"
    return "hardware-or-other"


def percentile_nearest_rank(sorted_values, p):
    """Nearest-rank percentile of pre-sorted values: the sample
    at rank ceil(p/100 * N), 1-based, clamped to [1, N].
    None for an empty sample set."""
    if not sorted_values:
        return None
    if p <= 0:
        return sorted_values[0]
    rank = math.ceil(p / 100.0 * len(sorted_values))
    rank = max(1, min(rank, len(sorted_values)))
    return sorted_values[rank - 1]


def frame_stats(values):
    """{mean, p50, p95, p99} over per-frame milliseconds;
    every field null when there are no samples."""
    if not values:
        return {"mean": None, "p50": None, "p95": None, "p99": None}
    ordered = sorted(values)
    return {
        "mean": sum(values) / len(values),
        "p50": percentile_nearest_rank(ordered, 50),
        "p95": percentile_nearest_rank(ordered, 95),
        "p99": percentile_nearest_rank(ordered, 99),
    }


def parse_clock(tok):
    """'M:SS.cc' / 'H:MM:SS.cc' / bare seconds -> float seconds
    (the GNU time -v elapsed shapes; same rule as
    bench_report.py's parse_clock)."""
    tok = tok.strip()
    if ":" in tok:
        total = 0.0
        for part in tok.split(":"):
            total = total * 60 + float(part)
        return total
    return float(tok)


def parse_time_v(text):
    """(elapsed_s, peak_rss_kb) from GNU `time -v` output;
    either field None when absent."""
    elapsed = rss = None
    m = TIME_V_ELAPSED_RE.search(text)
    if m:
        try:
            elapsed = parse_clock(m.group(1))
        except ValueError:
            elapsed = None
    m = TIME_V_RSS_RE.search(text)
    if m:
        rss = int(m.group(1))
    return elapsed, rss


def find_renderer_line(log_text):
    """First `renderer:`/`adapter:` line, verbatim (stripped),
    or None."""
    for line in log_text.splitlines():
        if RENDERER_RE.match(line):
            return line.strip()
    return None


def find_paint_cache_line(log_text):
    """Last `paint-cache:` stats line, verbatim, or None (a log
    may hold several runs; the end-of-run line is what counts —
    the same rule vram_report.py applies)."""
    found = None
    for line in log_text.splitlines():
        m = PAINT_CACHE_RE.search(line)
        if m:
            found = m.group(0).strip()
    return found


def parse_timing_lines(log_text):
    """{frame_index: milliseconds} from the per-frame timing
    lines (header lists the accepted shapes). Progress and
    completion lines are explicitly not timing lines. A frame
    named twice keeps its LAST value (a re-emitted line
    supersedes, as with the end-of-run stats lines)."""
    times = {}
    for line in log_text.splitlines():
        if "(t =" in line and "rendered" in line:
            continue  # 25-frame progress line, not a frame time
        if line.lstrip().startswith("film render complete"):
            continue
        for rx in TIMING_RES:
            m = rx.search(line)
            if m:
                times[int(m.group(1))] = float(m.group(2))
                break
    return times


def completion_rss_kb(log_text):
    """Fallback peak RSS: the completion line's runlog
    `peak RSS <MiB> MB` converted to KiB (x1024), or None."""
    m = COMPLETION_RSS_RE.search(log_text)
    if m:
        return round(float(m.group(1)) * 1024)
    return None


def sha256_file(path):
    h = hashlib.sha256()
    with open(path, "rb") as fh:
        for chunk in iter(lambda: fh.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def hash_frames(frames_dir):
    """{filename: sha256 hex} for every frame_*.png in the
    directory, sorted by filename. Missing dir -> {}."""
    out = {}
    frames_dir = Path(frames_dir)
    if frames_dir.is_dir():
        for path in sorted(frames_dir.glob("frame_*.png")):
            out[path.name] = sha256_file(path)
    return out


# --- record assembly -------------------------------------------------------

def assemble_record(label, binary_path, binary_sha256, git_head,
                    renderer_line, frame_range, frame_hashes,
                    per_frame_ms, total_s, peak_rss_kb,
                    paint_cache_line, paint_cache_arg, exit_code):
    """Pure assembly of a talerender-bench/1 record dict from
    already-collected evidence. No I/O, no fabrication: every
    input the caller could not collect arrives as None / {}
    and stays null / empty in the record."""
    a, b = frame_range
    ordered_times = {str(n): per_frame_ms[n] for n in sorted(per_frame_ms)}
    return {
        "schema": SCHEMA,
        "label": label,
        "binary_path": binary_path,
        "binary_sha256": binary_sha256,
        "git_head": git_head,
        "adapter": {
            "renderer_line": renderer_line,
            "adapter_class": adapter_class(renderer_line),
        },
        "frames": {
            "count": len(frame_hashes),
            "range": [a, b],
            "sha256": dict(sorted(frame_hashes.items())),
        },
        "timing": {
            "total_s": total_s,
            "frame_ms": frame_stats(list(per_frame_ms.values())),
            "per_frame_ms": ordered_times,
            "samples": len(per_frame_ms),
        },
        "peak_rss_kb": peak_rss_kb,
        "paint_cache": paint_cache_line,
        "paint_cache_arg": paint_cache_arg,
        "exit_code": exit_code,
    }


def collect_record(label, binary, frames_dir, log_text, time_text,
                   frame_range, git_head, paint_cache_arg, exit_code,
                   wall_s=None):
    """Assemble a record from run artifacts: the binary file,
    the frames directory, the captured log text and the GNU
    time -v text (either text may be None). Wall time prefers
    time -v, then the caller's own wall clock; peak RSS
    prefers time -v, then the completion-line self-report."""
    total_s, peak_rss_kb = (None, None)
    if time_text:
        total_s, peak_rss_kb = parse_time_v(time_text)
    if total_s is None:
        total_s = wall_s
    if peak_rss_kb is None:
        peak_rss_kb = completion_rss_kb(log_text)
    return assemble_record(
        label=label,
        binary_path=str(binary),
        binary_sha256=sha256_file(binary),
        git_head=git_head,
        renderer_line=find_renderer_line(log_text),
        frame_range=frame_range,
        frame_hashes=hash_frames(frames_dir),
        per_frame_ms=parse_timing_lines(log_text),
        total_s=total_s,
        peak_rss_kb=peak_rss_kb,
        paint_cache_line=find_paint_cache_line(log_text),
        paint_cache_arg=paint_cache_arg,
        exit_code=exit_code,
    )


def resolve_git_head(start_dirs):
    """Best-effort `git rev-parse HEAD` from the first start
    directory that yields one; None when unresolvable. Run-time
    provenance only — the self-test never calls this."""
    for start in start_dirs:
        try:
            proc = subprocess.run(
                ["git", "-C", str(start), "rev-parse", "HEAD"],
                capture_output=True, text=True, timeout=5)
        except (OSError, subprocess.SubprocessError):
            continue
        head = proc.stdout.strip()
        if proc.returncode == 0 and re.fullmatch(r"[0-9a-f]{7,64}", head):
            return head
    return None


# --- markdown rendering ----------------------------------------------------

def _fmt(x, digits=2):
    return "—" if x is None else f"{x:.{digits}f}"


def render_markdown(record):
    """Human summary of a record. A pure function of the record
    dict, so `bench_lab.py record <label>.json` re-renders the
    identical .md the `run` command wrote."""
    adapter = record.get("adapter") or {}
    renderer_line = adapter.get("renderer_line")
    aclass = adapter.get("adapter_class") or adapter_class(renderer_line)
    frames = record.get("frames") or {}
    rng = frames.get("range") or [None, None]
    timing = record.get("timing") or {}
    fms = timing.get("frame_ms") or {}

    lines = [f"# TaleRender bench record — `{record.get('label')}`", ""]
    if aclass == "lavapipe-software":
        lines += [
            "> ⚠ **SOFTWARE RENDERER — INDICATIVE ONLY.** The "
            "renderer line names a software Vulkan device "
            "(lavapipe / llvmpipe, CPU). These numbers are **not "
            "hardware performance** and must never be quoted as "
            "such (PERF.md honesty rule). GPU rows remain "
            "**PENDING-PC** until the same command runs on the "
            "user's RADV machine.", ""]
    elif aclass == "unknown":
        lines += [
            "> ⚠ **NO RENDERER LINE** in the run log: this record "
            "cannot name its renderer, so its numbers must not be "
            "quoted until the run is repeated with the renderer "
            "line captured.", ""]
    lines += [
        f"**Renderer:** `{renderer_line}`" if renderer_line
        else "**Renderer:** — (no renderer line in log)",
        "",
        "| field | value |",
        "|---|---|",
        f"| schema | `{record.get('schema')}` |",
        f"| label | `{record.get('label')}` |",
        f"| binary | `{record.get('binary_path')}` |",
        f"| binary sha256 | `{record.get('binary_sha256')}` |",
        f"| git head | `{record.get('git_head')}` |"
        if record.get("git_head") else "| git head | — (unresolvable) |",
        f"| adapter class | `{aclass}` |",
        f"| frames range | {rng[0]}..{rng[1]} |",
        f"| frames produced | {frames.get('count')} |",
        f"| wall time (total_s) | {_fmt(timing.get('total_s'))} s |",
        f"| peak RSS | "
        f"{record['peak_rss_kb']:,} KiB |" if record.get("peak_rss_kb")
        is not None else "| peak RSS | — (unavailable) |",
        f"| paint-cache arg | {record.get('paint_cache_arg')} |"
        if record.get("paint_cache_arg") is not None
        else "| paint-cache arg | — (binary default) |",
        f"| exit code | {record.get('exit_code')} |"
        if record.get("exit_code") is not None
        else "| exit code | — |",
        "",
        "## Frame times (per-frame timing lines)",
        "",
    ]
    if timing.get("samples"):
        lines += [
            "| metric | ms |",
            "|---|---|",
            f"| mean | {_fmt(fms.get('mean'))} |",
            f"| p50 | {_fmt(fms.get('p50'))} |",
            f"| p95 | {_fmt(fms.get('p95'))} |",
            f"| p99 | {_fmt(fms.get('p99'))} |",
            "",
            f"Percentiles are nearest-rank over "
            f"{timing['samples']} timed frame(s); see the record "
            "schema in bench_lab.py's header.",
            "",
        ]
    else:
        lines += [
            "No per-frame timing lines in the log: frame-time "
            "statistics are unavailable and are not invented. "
            "(The binary must emit them under "
            "TALERENDER_FRAME_TIMING=1; see the header note on "
            "the pending film-loop wiring.)",
            "",
        ]
    lines += [
        "## Paint cache",
        "",
        f"`{record['paint_cache']}`" if record.get("paint_cache")
        else "No `paint-cache:` stats line in the log.",
        "",
        "## Frame hashes (sha256)",
        "",
    ]
    hashes = frames.get("sha256") or {}
    if hashes:
        lines += ["| frame | sha256 |", "|---|---|"]
        for name in sorted(hashes):
            lines.append(f"| `{name}` | `{hashes[name]}` |")
    else:
        lines.append("No frame PNGs were produced (or none were "
                     "found in the frames directory).")
    lines += [
        "",
        "Provenance: written by `tools/perf/lab/bench_lab.py run`; "
        "re-render this file from the JSON record alone with "
        "`bench_lab.py record <label>.json`. GPU validation "
        "remains PENDING-PC.",
        "",
    ]
    return "\n".join(lines)


def write_record(record, out_dir):
    """Write <out>/<label>.json + <out>/<label>.md; return the
    two paths. JSON is indent-2, key-sorted, newline-ended so
    committed records diff cleanly as text."""
    out_dir = Path(out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    label = record["label"]
    json_path = out_dir / f"{label}.json"
    md_path = out_dir / f"{label}.md"
    with open(json_path, "w", encoding="utf-8") as fh:
        json.dump(record, fh, indent=2, sort_keys=True)
        fh.write("\n")
    with open(md_path, "w", encoding="utf-8") as fh:
        fh.write(render_markdown(record))
    return json_path, md_path


# --- subcommands -----------------------------------------------------------

def cmd_run(argv):
    binary = out_dir = label = frames_arg = film = None
    paint_cache = None
    i = 0
    while i < len(argv):
        arg = argv[i]
        if arg == "--self-test":
            return run_self_test(sys.stdout)
        if arg in ("--binary", "--out", "--label", "--frames",
                   "--paint-cache", "--film"):
            if i + 1 >= len(argv):
                raise InputError(f"{arg} needs a value")
            value = argv[i + 1]
            i += 2
            if arg == "--binary":
                binary = value
            elif arg == "--out":
                out_dir = value
            elif arg == "--label":
                label = value
            elif arg == "--frames":
                frames_arg = value
            elif arg == "--film":
                film = value
            else:
                try:
                    paint_cache = int(value)
                except ValueError:
                    raise InputError(
                        f"--paint-cache needs an integer, got {value!r}")
                if paint_cache < 0:
                    raise InputError("--paint-cache must be >= 0")
            continue
        raise InputError(f"unknown run option {arg!r}")
    missing = [name for name, val in
               (("--binary", binary), ("--out", out_dir),
                ("--label", label), ("--frames", frames_arg))
               if val is None]
    if missing:
        raise InputError(
            "run needs " + ", ".join(missing) +
            " (usage: bench_lab.py run --binary PATH --out DIR "
            "--label NAME --frames A..B [--paint-cache N])")
    if not LABEL_RE.match(label) or label in (".", ".."):
        raise InputError(
            f"bad --label {label!r} (letters/digits/._- only; it "
            "becomes a file stem)")
    frame_range = parse_frame_range(frames_arg)
    binary_path = Path(binary)
    if not binary_path.is_file():
        raise InputError(f"no such binary: {binary}")
    film = film or "film"
    if not Path(film).is_dir():
        raise InputError(
            f"no such film directory: {film} (the binary's "
            "--film argument; pass --film DIR if it is not the "
            "repo-root `film`)")

    out = Path(out_dir)
    out.mkdir(parents=True, exist_ok=True)
    frames_dir = out / f"{label}.frames"
    if frames_dir.is_dir() and any(frames_dir.glob("frame_*.png")):
        raise InputError(
            f"frames directory {frames_dir} already holds frame "
            "PNGs — the driver would resume-skip them and the "
            "timing would be invalid (PERF §3 fresh-directory "
            "rule); remove it or choose a new --label/--out")
    frames_dir.mkdir(parents=True, exist_ok=True)
    log_path = out / f"{label}.log"
    time_path = out / f"{label}.time"

    env = dict(os.environ)
    env["TALERENDER_FRAME_TIMING"] = "1"
    if paint_cache is not None:
        env["TALERENDER_PAINT_CACHE"] = str(paint_cache)

    a, b = frame_range
    render_cmd = [str(binary_path), "--film", film,
                  "--out", str(frames_dir),
                  "--from", str(a), "--to", str(b)]
    time_bin = "/usr/bin/time"
    if os.path.isfile(time_bin) and os.access(time_bin, os.X_OK):
        cmd = [time_bin, "-v", "-o", str(time_path)] + render_cmd
    else:
        cmd = render_cmd  # fallbacks in collect_record apply

    started = time.monotonic()
    proc = subprocess.run(cmd, env=env, capture_output=True,
                          text=True, errors="replace")
    wall_s = time.monotonic() - started
    log_text = (proc.stdout or "") + (proc.stderr or "")
    with open(log_path, "w", encoding="utf-8") as fh:
        fh.write(log_text)
    time_text = None
    if time_path.is_file():
        time_text = time_path.read_text(encoding="utf-8",
                                        errors="replace")

    record = collect_record(
        label=label, binary=binary_path, frames_dir=frames_dir,
        log_text=log_text, time_text=time_text,
        frame_range=frame_range,
        git_head=resolve_git_head([Path.cwd(),
                                   binary_path.resolve().parent]),
        paint_cache_arg=paint_cache, exit_code=proc.returncode,
        wall_s=wall_s)
    json_path, md_path = write_record(record, out)
    print(f"bench_lab: wrote {json_path}")
    print(f"bench_lab: wrote {md_path}")
    print(f"bench_lab: raw log {log_path}; frames in {frames_dir}")
    if proc.returncode != 0:
        print(f"bench_lab: render exited {proc.returncode} — the "
              "record above carries exit_code and whatever "
              "evidence the log holds; its timings are partial",
              file=sys.stderr)
        return 1
    return 0


def load_record(path):
    try:
        with open(path, encoding="utf-8") as fh:
            record = json.load(fh)
    except OSError as e:
        raise InputError(f"cannot read record {path} ({e})")
    except json.JSONDecodeError as e:
        raise InputError(f"not a JSON record: {path} ({e})")
    if not isinstance(record, dict) or \
            record.get("schema") != SCHEMA:
        raise InputError(
            f"{path} is not a {SCHEMA} record "
            f"(schema field: {record.get('schema')!r})"
            if isinstance(record, dict) else
            f"{path} is not a {SCHEMA} record")
    return record


def cmd_record(argv):
    record_path = out_path = None
    i = 0
    while i < len(argv):
        arg = argv[i]
        if arg in ("--out", "--record", "--input"):
            if i + 1 >= len(argv):
                raise InputError(f"{arg} needs a value")
            if arg == "--out":
                out_path = argv[i + 1]
            else:
                record_path = argv[i + 1]
            i += 2
            continue
        if arg.startswith("--"):
            raise InputError(f"unknown record option {arg!r}")
        if record_path is not None:
            raise InputError("record takes exactly one RECORD.json")
        record_path = arg
        i += 1
    if record_path is None:
        raise InputError(
            "usage: bench_lab.py record RECORD.json [--out REPORT.md]")
    record = load_record(record_path)
    if out_path is None:
        out_path = str(Path(record_path).with_suffix(".md"))
    with open(out_path, "w", encoding="utf-8") as fh:
        fh.write(render_markdown(record))
    print(f"bench_lab: wrote {out_path}")
    return 0


# --- self-test -------------------------------------------------------------

# Synthetic evidence, hand-computable: five frames with timing
# samples [10, 20, 30, 40, 100] ms -> mean 40.0; nearest-rank
# p50 = 3rd of 5 = 30, p95 = p99 = 5th of 5 = 100.
FAKE_RENDERER_SOFTWARE = (
    "renderer: llvmpipe (LLVM 15.0.7, 256 bits) | backend: Vulkan "
    "| device type: Cpu | driver: llvmpipe 0.0.1")
FAKE_RENDERER_HARDWARE = (
    "renderer: AMD Radeon Vega 7 (RADV RENOIR) | backend: Vulkan "
    "| device type: IntegratedGpu | driver: radv 23.2.1")
FAKE_PAINT_CACHE = (
    "paint-cache: entries=3 bytes=300 hits=7 misses=3 evictions=1 "
    "rerenders=1 peak_bytes=350 budget_bytes=1000")
FAKE_LOG = "\n".join([
    FAKE_RENDERER_SOFTWARE,
    "film: fake — 5 frames @30fps, rendering 0..5",
    "frame 0: 10.0 ms",
    "frame 1: 20.0 ms",
    "frame-time: frame=2 ms=30.0",
    "frame 3 — 40.0 ms",
    "  frame 1051 (t = 35.03s) — 25 rendered, 0 skipped · "
    "elapsed 00:07 · eta 00:18",  # progress line: NOT a frame time
    "frame 4 100.0 ms",
    "film render complete: 5 rendered, 0 skipped (resume), "
    "range 0..5 · elapsed 00:02 · peak RSS 50.0 MB · "
    + FAKE_PAINT_CACHE,
    "",
])
FAKE_TIME = (
    "\tElapsed (wall clock) time (h:mm:ss or m:ss): 0:02.00\n"
    "\tMaximum resident set size (kbytes): 123456\n")


def run_self_test(out):
    """Synthetic proof: build a fake run directory (fake binary,
    fake frame PNGs) + a fake timing log + a fake time -v file
    in a temp dir, assemble a record through the same
    collect_record path `run` uses, and verify the assembly,
    the percentile math, the lavapipe labelling rule and the
    record -> markdown round-trip. No binary is executed, no
    git is invoked, nothing outside the temp dir is touched."""
    checks = []

    def check(name, ok, detail=""):
        checks.append((name, ok))
        out.write(f"  [{'PASS' if ok else 'FAIL'}] {name}"
                  + (f" — {detail}" if detail and not ok else "")
                  + "\n")

    # Percentile math, hand-computed (nearest-rank).
    hundred = list(range(1, 101))
    check("nearest-rank p50 of 1..100 == 50",
          percentile_nearest_rank(hundred, 50) == 50)
    check("nearest-rank p95 of 1..100 == 95",
          percentile_nearest_rank(hundred, 95) == 95)
    check("nearest-rank p99 of 1..100 == 99",
          percentile_nearest_rank(hundred, 99) == 99)
    check("nearest-rank of a single sample is that sample",
          percentile_nearest_rank([7.5], 50) == 7.5
          and percentile_nearest_rank([7.5], 99) == 7.5)
    check("percentile of no samples is None",
          percentile_nearest_rank([], 95) is None)
    stats = frame_stats([100.0, 10.0, 40.0, 20.0, 30.0])
    check("frame_stats mean/p50/p95/p99 over the fake samples",
          stats == {"mean": 40.0, "p50": 30.0, "p95": 100.0,
                    "p99": 100.0}, f"got {stats}")
    check("frame_stats of no samples is all-None",
          frame_stats([]) == {"mean": None, "p50": None,
                              "p95": None, "p99": None})

    # The labelling rule, both directions + the missing case.
    check("llvmpipe line labels lavapipe-software",
          adapter_class(FAKE_RENDERER_SOFTWARE) == "lavapipe-software")
    check("lavapipe token labels lavapipe-software",
          adapter_class("renderer: lavapipe (Mesa)") ==
          "lavapipe-software")
    check("RADV line labels hardware-or-other",
          adapter_class(FAKE_RENDERER_HARDWARE) == "hardware-or-other")
    check("no renderer line labels unknown (never hardware)",
          adapter_class(None) == "unknown")

    # Timing-log parsing: every accepted shape, progress excluded.
    times = parse_timing_lines(FAKE_LOG)
    check("timing lines parse (all accepted shapes)",
          times == {0: 10.0, 1: 20.0, 2: 30.0, 3: 40.0, 4: 100.0},
          f"got {times}")
    check("progress line is not a frame time",
          1051 not in times)

    with tempfile.TemporaryDirectory(prefix="bench-lab-selftest-") as tmp:
        tmp = Path(tmp)
        fake_binary = tmp / "talerender-fake"
        fake_binary.write_bytes(b"fake talerender binary for bench_lab\n")
        frames_dir = tmp / "fake.frames"
        frames_dir.mkdir()
        expected_hashes = {}
        for n in range(5):
            data = f"fake frame {n}\n".encode()
            name = f"frame_{n:05d}.png"
            (frames_dir / name).write_bytes(data)
            expected_hashes[name] = hashlib.sha256(data).hexdigest()

        record = collect_record(
            label="selftest-fake", binary=fake_binary,
            frames_dir=frames_dir, log_text=FAKE_LOG,
            time_text=FAKE_TIME, frame_range=(0, 5),
            git_head="selftest-head", paint_cache_arg=24,
            exit_code=0)

        check("record schema is talerender-bench/1",
              record["schema"] == "talerender-bench/1")
        check("record label round-trips", record["label"] == "selftest-fake")
        check("binary sha256 matches the fake binary",
              record["binary_sha256"] ==
              hashlib.sha256(fake_binary.read_bytes()).hexdigest())
        check("git head is carried verbatim when resolvable",
              record["git_head"] == "selftest-head")
        check("adapter line verbatim + lavapipe class",
              record["adapter"] == {
                  "renderer_line": FAKE_RENDERER_SOFTWARE,
                  "adapter_class": "lavapipe-software"},
              f"got {record['adapter']}")
        check("frames count is the produced count (5)",
              record["frames"]["count"] == 5)
        check("frames range is [0, 5]", record["frames"]["range"] == [0, 5])
        check("frame sha256 map matches hand-computed hashes",
              record["frames"]["sha256"] == expected_hashes)
        check("total_s prefers the time -v wall clock (2.00 s)",
              record["timing"]["total_s"] == 2.0,
              f"got {record['timing']['total_s']}")
        check("timing stats over the fake samples",
              record["timing"]["frame_ms"] ==
              {"mean": 40.0, "p50": 30.0, "p95": 100.0, "p99": 100.0},
              f"got {record['timing']['frame_ms']}")
        check("peak_rss_kb prefers time -v (123456, not the "
              "completion line's 50.0 MB)",
              record["peak_rss_kb"] == 123456,
              f"got {record['peak_rss_kb']}")
        check("paint-cache line is verbatim",
              record["paint_cache"] == FAKE_PAINT_CACHE,
              f"got {record['paint_cache']}")
        check("paint-cache arg + exit code carried",
              record["paint_cache_arg"] == 24
              and record["exit_code"] == 0)

        # Missing-evidence honesty: no timing lines, no time -v,
        # no paint-cache -> nulls, and the RSS fallback engages.
        bare = collect_record(
            label="selftest-bare", binary=fake_binary,
            frames_dir=frames_dir,
            log_text=FAKE_RENDERER_HARDWARE + "\n"
                     "film render complete: 5 rendered, 0 skipped "
                     "(resume), range 0..5 · peak RSS 50.0 MB\n",
            time_text=None, frame_range=(0, 5), git_head=None,
            paint_cache_arg=None, exit_code=0, wall_s=3.5)
        check("no timing lines -> frame_ms all null, samples 0",
              bare["timing"]["frame_ms"] ==
              {"mean": None, "p50": None, "p95": None, "p99": None}
              and bare["timing"]["samples"] == 0)
        check("no time -v -> total falls back to wall, RSS to "
              "the completion line (50.0 MB = 51200 KiB)",
              bare["timing"]["total_s"] == 3.5
              and bare["peak_rss_kb"] == 51200,
              f"got total={bare['timing']['total_s']} "
              f"rss={bare['peak_rss_kb']}")
        check("no paint-cache line -> null, git head null",
              bare["paint_cache"] is None and bare["git_head"] is None)
        check("RADV record labels hardware-or-other",
              bare["adapter"]["adapter_class"] == "hardware-or-other")

        # Write + record-subcommand round-trip: the re-rendered
        # .md must be byte-identical to the one `run` wrote.
        json_path, md_path = write_record(record, tmp / "out")
        md_before = md_path.read_text(encoding="utf-8")
        rc = cmd_record([str(json_path)])
        md_after = md_path.read_text(encoding="utf-8")
        check("record subcommand re-renders an identical .md",
              rc == 0 and md_after == md_before)
        check("software banner + PENDING-PC in the summary",
              "SOFTWARE RENDERER" in md_before
              and "PENDING-PC" in md_before)
        check("summary names the renderer verbatim",
              FAKE_RENDERER_SOFTWARE in md_before)
        loaded = load_record(json_path)
        check("written JSON reloads as the same record",
              loaded == record)

    failed = sum(1 for _name, ok in checks if not ok)
    passed = len(checks) - failed
    if failed:
        out.write(f"SELF-TEST: FAIL ({passed} passed, {failed} failed)\n")
        return 1
    out.write(f"SELF-TEST: PASS ({passed} passed, 0 failed)\n")
    return 0


# --- entry -----------------------------------------------------------------

USAGE = (
    "usage:\n"
    "  bench_lab.py run --binary PATH --out DIR --label NAME "
    "--frames A..B [--paint-cache N] [--film DIR]\n"
    "  bench_lab.py record RECORD.json [--out REPORT.md]\n"
    "  bench_lab.py --self-test")


def main(argv=None):
    argv = list(sys.argv[1:] if argv is None else argv)
    try:
        if not argv:
            raise InputError("no subcommand\n" + USAGE)
        if argv[0] in ("-h", "--help"):
            print(USAGE)
            return 0
        if argv[0] in ("--self-test", "self-test"):
            return run_self_test(sys.stdout)
        if argv[0] == "run":
            return cmd_run(argv[1:])
        if argv[0] == "record":
            return cmd_record(argv[1:])
        raise InputError(f"unknown subcommand {argv[0]!r}\n" + USAGE)
    except InputError as e:
        print(f"ERROR: {e}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
