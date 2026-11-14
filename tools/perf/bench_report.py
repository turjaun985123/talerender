#!/usr/bin/env python3
"""bench_report.py — turn a TaleRender bench log directory into
PERF.md-shaped markdown tables.

Usage:
    python3 tools/perf/bench_report.py <log-dir> [--out report.md]

Accepted log directories (both are flat directories of log files):

  * A CI bench artifact (from .github/workflows/bench.yml):
    env.txt, bench-default.{log,time,exit}, bench-cache24.{...},
    frames-*.sizes.

  * A PC run following docs/runbook/PERF.md / USER-PC.md:
    env.txt, ab1-<n>.{log,time}, s01-<n>.{log,time},
    chunk-cold / chunk-warm / chunk-s06-*.{log,time},
    telemetry-default / telemetry-24.{log,time,err},
    chunk-warm/frame_*.png, and optionally chrome.time
    (GNU time "%e %M" format) for the §6 comparison row.

Protocol laws implemented here (PERF.md is the spec):

  * Every table is headed by the renderer line taken verbatim
    from the logs (`renderer:` in film mode, `adapter:` in scene
    modes). A table without its renderer named is a defect, so a
    directory whose logs carry NO renderer line is refused.
  * Logs from different renderers are NEVER merged into one
    table: if the directory contains more than one distinct
    renderer, the tool refuses (exit 2) and names them.
  * If the renderer is a software device (llvmpipe / lavapipe /
    device type Cpu), the report carries the honesty banner:
    these numbers are indicative only, NOT hardware performance.
  * A film run whose completion line reports skipped frames or
    a rendered count short of its range is flagged INVALID
    (PERF §3/§7: a short render invalidates the timing).
  * The full-film extrapolation (§3) is computed ONLY from runs
    named *warm* — the only arithmetic PERF sanctions. CI bench
    legs are single startup-inclusive runs and get no
    extrapolation.
  * Totals prefer the caller's /usr/bin/time measurement (wall
    clock of the whole process); the binary's own completion
    line (post-W09: `elapsed MM:SS · peak RSS <MiB> MB`) is the
    fallback and is labelled as render-loop-only when used.

Both progress-line generations parse: the pre-W09 form
(`  frame 1051 (t = 35.03s) — 25 rendered, 0 skipped`) and the
post-W09 form, which appends ` · elapsed 00:07 · eta 00:18`
(elapsed is MM:SS with total minutes, per RUNLOG-W09). Absent
elapsed is tolerated — per-range times then report as
unavailable instead of being invented.

Exit codes: 0 = report written; 2 = refusal / usage error.
Stdlib only.
"""

import glob
import os
import re
import statistics
import sys

FULL_FILM_FRAMES = 1811

RENDERER_RE = re.compile(r"^\s*(renderer|adapter):\s*(.+?)\s*$", re.M)
PROGRESS_RE = re.compile(
    r"frame\s+(\d+)\s+\(t\s*=\s*([\d.]+)s\)\s*—\s*(\d+)\s+rendered,"
    r"\s*(\d+)\s+skipped(.*)$", re.M)
PROGRESS_BARE_RE = re.compile(
    r"frame\s+(\d+)/(\d+)\s+\([\d.]+%\)\s+elapsed\s+(\S+)", re.M)
ELAPSED_RE = re.compile(
    r"elapsed\s+([0-9]+:[0-9]{2}(?::[0-9]{2})?|[0-9]+(?:\.[0-9]+)?\s*s)")
COMPLETE_RE = re.compile(
    r"film render complete:\s*(\d+)\s+rendered,\s*(\d+)\s+skipped"
    r"\s*\(resume\),\s*range\s+(\d+)\.\.(\d+)(.*)$", re.M)
PEAK_RSS_RE = re.compile(r"peak RSS\s+([\d.]+)\s*MB")
VERDICT_RE = re.compile(r"FILM TELEMETRY:\s*(CHECK[^\n]*)")
TIME_V_ELAPSED_RE = re.compile(
    r"Elapsed \(wall clock\) time \(h:mm:ss or m:ss\):\s*([0-9:.]+)")
TIME_V_RSS_RE = re.compile(r"Maximum resident set size \(kbytes\):\s*(\d+)")
SHA_RE = re.compile(r"^([0-9a-f]{64})\s+(\S+)", re.M)


def fail(msg):
    print(f"bench_report: ERROR: {msg}", file=sys.stderr)
    sys.exit(2)


def parse_clock(tok):
    """'MM:SS' (total minutes, W09 format), 'H:MM:SS', or seconds."""
    tok = tok.strip().rstrip("s")
    if ":" in tok:
        parts = [float(p) for p in tok.split(":")]
        total = 0.0
        for p in parts:
            total = total * 60 + p
        return total
    return float(tok)


def find_elapsed(text):
    m = ELAPSED_RE.search(text)
    return parse_clock(m.group(1)) if m else None


def fmt(x, digits=2):
    return "—" if x is None else f"{x:.{digits}f}"


class Run:
    def __init__(self, stem):
        self.stem = stem
        self.kind = "film"          # film | still | telemetry | chrome
        self.still_name = None
        self.still_index = None
        self.log_text = ""
        self.elapsed = None         # seconds (time file preferred)
        self.elapsed_source = None  # 'time -v' | 'time %e %M' | 'completion line'
        self.rss_kib = None
        self.rss_source = None
        self.exit_code = None
        self.progress = []          # dicts: frame, rendered, skipped, elapsed
        self.completion = None      # dict: rendered, skipped, frm, to, elapsed, rss_kib
        self.verdict = None


def parse_time_file(path):
    """Return (elapsed_s, rss_kib, kind) from a GNU time output file,
    sniffing the -v form vs the '-f %e %M' form. (None, None, None)
    if neither parses."""
    try:
        text = open(path, encoding="utf-8", errors="replace").read()
    except OSError:
        return None, None, None
    m_rss = TIME_V_RSS_RE.search(text)
    if m_rss:
        m_el = TIME_V_ELAPSED_RE.search(text)
        elapsed = parse_clock(m_el.group(1)) if m_el else None
        return elapsed, int(m_rss.group(1)), "time -v"
    for line in text.splitlines():
        parts = line.split()
        if len(parts) == 2:
            try:
                return float(parts[0]), int(float(parts[1])), "time %e %M"
            except ValueError:
                continue
    return None, None, None


def parse_log(text):
    out = {"progress": [], "completion": None, "verdict": None}
    for m in PROGRESS_RE.finditer(text):
        out["progress"].append({
            "frame": int(m.group(1)),
            "rendered": int(m.group(3)),
            "skipped": int(m.group(4)),
            "elapsed": find_elapsed(m.group(5)),
        })
    if not out["progress"]:
        for m in PROGRESS_BARE_RE.finditer(text):
            out["progress"].append({
                "frame": int(m.group(1)),
                "rendered": None,
                "skipped": None,
                "elapsed": parse_clock(m.group(3)),
            })
    m = COMPLETE_RE.search(text)
    if m:
        rest = m.group(5)
        rss = None
        mr = PEAK_RSS_RE.search(rest)
        if mr:
            rss = round(float(mr.group(1)) * 1024)  # runlog MB = MiB → KiB
        out["completion"] = {
            "rendered": int(m.group(1)), "skipped": int(m.group(2)),
            "frm": int(m.group(3)), "to": int(m.group(4)),
            "elapsed": find_elapsed(rest), "rss_kib": rss,
        }
    mv = VERDICT_RE.search(text)
    if mv:
        out["verdict"] = mv.group(1).strip()
    return out


def classify(stem):
    """Map a log stem to (kind, label)."""
    m = re.fullmatch(r"(ab1|s01)-(\d+)", stem)
    if m:
        name = "ab1 (`--scene ab1`)" if m.group(1) == "ab1" else \
            "S01 (`--scene-file scenes/film/s01.scene.json`)"
        return "still", (m.group(1), int(m.group(2)), name)
    if stem.startswith("telemetry"):
        cap = "24" if stem.endswith("24") else "160 (default)"
        return "telemetry", f"telemetry, cap {cap}"
    if stem == "chunk-cold":
        return "film", "chunk 1027..1117 — cold"
    if stem == "chunk-warm":
        return "film", "chunk 1027..1117 — warm"
    if stem.startswith("chunk-s06"):
        warm = "warm" if "warm" in stem else "cold"
        return "film", f"optional chunk 784..874 (S06, fog) — {warm}"
    if stem == "bench-default":
        return "film", "film 0..100 (CI bench, cap 160)"
    if stem == "bench-cache24":
        return "film", "film 0..100 (CI bench, cap 24)"
    return "film", stem


def load_runs(logdir):
    runs = {}
    for name in sorted(os.listdir(logdir)):
        if not name.endswith(".log"):
            continue
        stem = name[:-4]
        run = Run(stem)
        path = os.path.join(logdir, name)
        run.log_text = open(path, encoding="utf-8", errors="replace").read()
        parsed = parse_log(run.log_text)
        run.progress = parsed["progress"]
        run.completion = parsed["completion"]
        run.verdict = parsed["verdict"]
        kind, info = classify(stem)
        run.kind = kind
        if kind == "still":
            run.still_name, run.still_index, run.label = info
        else:
            run.label = info
        tpath = os.path.join(logdir, stem + ".time")
        if os.path.exists(tpath):
            el, rss, kind_t = parse_time_file(tpath)
            if el is not None or rss is not None:
                run.elapsed, run.rss_kib = el, rss
                run.elapsed_source = run.rss_source = kind_t
        xpath = os.path.join(logdir, stem + ".exit")
        if os.path.exists(xpath):
            try:
                run.exit_code = int(open(xpath).read().strip())
            except ValueError:
                pass
        # Fall back to the completion line's own signals (post-W09).
        if run.completion:
            if run.elapsed is None and run.completion["elapsed"] is not None:
                run.elapsed = run.completion["elapsed"]
                run.elapsed_source = "completion line (render loop only)"
            if run.rss_kib is None and run.completion["rss_kib"] is not None:
                run.rss_kib = run.completion["rss_kib"]
                run.rss_source = "completion line (binary self-report)"
        runs[stem] = run
    # Chrome timing (PERF §6): a bare time file, no log of its own.
    cpath = os.path.join(logdir, "chrome.time")
    if os.path.exists(cpath):
        el, rss, _ = parse_time_file(cpath)
        if el is not None:
            run = Run("chrome")
            run.kind, run.label, run.elapsed = "chrome", "chrome", el
            runs["chrome"] = run
    return runs


def renderer_header(renderer_line):
    return f"**Renderer:** `{renderer_line}`"


def run_warnings(run):
    w = []
    if run.exit_code not in (None, 0):
        w.append(f"`{run.stem}` exited {run.exit_code} — its numbers are "
                 "partial; keep the log (PERF §7).")
    c = run.completion
    if c and (c["skipped"] > 0 or c["rendered"] != c["to"] - c["frm"]):
        w.append(f"`{run.stem}` completion line reports {c['rendered']} "
                 f"rendered, {c['skipped']} skipped over range "
                 f"{c['frm']}..{c['to']} — **INVALID timing** per PERF §3 "
                 "(fresh-directory rule); the numbers are shown but must "
                 "not be quoted.")
    return w


def spf_fph(elapsed, frames):
    if not elapsed or not frames:
        return None, None
    spf = elapsed / frames
    return spf, 3600.0 / spf


def section_stills(runs, renderer_line, notes):
    stills = {}
    for run in runs.values():
        if run.kind == "still":
            stills.setdefault(run.still_name, []).append(run)
    if not stills:
        notes.append("§2 stills: no ab1-*/s01-* logs in this directory.")
        return ""
    lines = ["## §2 Scene-file render timing (single stills)", "",
             renderer_header(renderer_line), "",
             "| still | run 1 (cold) s | runs 2–5 s (all four) | "
             "warm median s | peak RSS (warm runs) KiB |",
             "|---|---|---|---|---|"]
    for key in sorted(stills):
        rs = sorted(stills[key], key=lambda r: r.still_index)
        elapsed = [r.elapsed for r in rs]
        cold = elapsed[0] if elapsed else None
        warm = [e for e in elapsed[1:] if e is not None]
        warm_s = ", ".join(fmt(e) for e in warm) if warm else "—"
        median = statistics.median(warm) if warm else None
        rss = [r.rss_kib for r in rs[1:] if r.rss_kib is not None]
        lines.append(
            f"| {rs[0].label} | {fmt(cold)} | {warm_s} | {fmt(median)} | "
            f"{max(rss) if rss else '—'} |")
        if len(rs) < 5:
            lines.append(f"")
            lines.append(f"Note: only {len(rs)} run(s) present for "
                         f"{rs[0].label}; PERF §2 prescribes five.")
    lines.append("")
    lines.append("Peak RSS (warm runs) is the maximum of the warm runs' "
                 "peak RSS values. Warm median per PERF §2 (median of "
                 "runs 2–5).")
    lines.append("")
    return "\n".join(lines)


def section_film(runs, renderer_line, notes, warnings):
    films = [r for r in runs.values() if r.kind == "film"]
    if not films:
        notes.append("§3 film: no film-run logs in this directory.")
        return ""
    order = {"chunk-cold": 0, "chunk-warm": 1, "chunk-s06-cold": 2,
             "chunk-s06-warm": 3, "bench-default": 4, "bench-cache24": 5}
    films.sort(key=lambda r: (order.get(r.stem, 9), r.stem))
    lines = ["## §3 Film render throughput", "",
             renderer_header(renderer_line), "",
             "| chunk | elapsed s | peak RSS KiB | s/frame | "
             "frames/hour | extrapolated 1,811 frames |",
             "|---|---|---|---|---|---|"]
    for run in films:
        frames = run.completion["rendered"] if run.completion else None
        spf, fph = spf_fph(run.elapsed, frames)
        extrap = "—"
        if spf is not None and "warm" in run.stem:
            extrap = f"{spf * FULL_FILM_FRAMES / 3600:.2f} h"
        lines.append(
            f"| {run.label} | {fmt(run.elapsed)} | "
            f"{run.rss_kib if run.rss_kib is not None else '—'} | "
            f"{fmt(spf, 3)} | {f'{fph:,.0f}' if fph else '—'} | {extrap} |")
    lines.append("")
    lines.append("Extrapolation is computed only for warm runs — the "
                 "only arithmetic PERF §3 sanctions. CI bench legs are "
                 "single startup-inclusive runs and are not extrapolated.")
    lines.append("")
    # Per-range detail, from the progress lines.
    for run in films:
        if not run.progress:
            continue
        lines.append(f"### Per-range frame times — {run.label}")
        lines.append("")
        lines.append(renderer_header(renderer_line))
        lines.append("")
        has_elapsed = any(p["elapsed"] is not None for p in run.progress)
        if not has_elapsed:
            lines.append("Progress lines carry no elapsed (pre-W09 "
                         "binary): per-range times are unavailable and "
                         "are not invented; totals above use the whole-"
                         "run wall clock.")
            lines.append("")
            continue
        lines.append("| range (frames) | rendered in range | elapsed s | "
                     "s/frame |")
        lines.append("|---|---|---|---|")
        prev_frame = (run.completion["frm"] - 1) if run.completion else None
        prev_rendered, prev_elapsed = 0, 0.0
        for p in run.progress:
            if p["rendered"] is not None:
                n_in = p["rendered"] - prev_rendered
                prev_rendered = p["rendered"]
            else:
                n_in = p["frame"] - (prev_frame if prev_frame is not None
                                     else p["frame"])
            start = (prev_frame + 1) if prev_frame is not None \
                else p["frame"] - n_in + 1
            el = None
            if p["elapsed"] is not None:
                el = p["elapsed"] - prev_elapsed
                prev_elapsed = p["elapsed"]
            spf, _ = spf_fph(el, n_in)
            lines.append(f"| {start}..{p['frame']} | {n_in} | {fmt(el)} | "
                         f"{fmt(spf, 3)} |")
            prev_frame = p["frame"]
        c = run.completion
        if c and prev_rendered and c["rendered"] > prev_rendered:
            el = None
            if c["elapsed"] is not None and prev_elapsed:
                el = c["elapsed"] - prev_elapsed
            n_in = c["rendered"] - prev_rendered
            spf, _ = spf_fph(el, n_in)
            lines.append(f"| {prev_frame + 1}..{c['to'] - 1} | {n_in} | "
                         f"{fmt(el)} | {fmt(spf, 3)} |")
        lines.append("")
    return "\n".join(lines)


def section_memory(runs, renderer_line, notes):
    teles = [r for r in runs.values() if r.kind == "telemetry"]
    films = [r for r in runs.values() if r.kind == "film"]
    warm = next((r for r in films if r.stem == "chunk-warm"), None)
    if not teles and not films:
        notes.append("§4 memory: no telemetry or film logs present.")
        return ""
    lines = ["## §4 Memory (peak RSS)", "",
             renderer_header(renderer_line), "",
             "| configuration | verdict line (telemetry) | elapsed s | "
             "peak RSS |",
             "|---|---|---|---|"]
    label = {"telemetry-default": "A. telemetry, cap 160 (default)",
             "telemetry-24": "B. telemetry, `TALERENDER_PAINT_CACHE=24`"}
    for run in sorted(teles, key=lambda r: r.stem):
        verdict = f"`FILM TELEMETRY: {run.verdict}`" if run.verdict else "—"
        rss = f"{run.rss_kib:,} KiB" if run.rss_kib is not None else "—"
        lines.append(f"| {label.get(run.stem, run.label)} | {verdict} | "
                     f"{fmt(run.elapsed)} | {rss} |")
    if teles:
        caps = {r.verdict for r in teles if r.verdict}
        if len(caps) > 1:
            lines.append("")
            lines.append("⚠ **The telemetry configurations disagree on "
                         "the verdict line** — per PERF §4 that is a "
                         "determinism defect, reported as such, with "
                         "both logs as evidence.")
    if warm is not None:
        rss = f"{warm.rss_kib:,} KiB" if warm.rss_kib is not None else "—"
        lines.append(f"| C. film chunk 1027..1117 warm, cap 160 | "
                     f"— (from §3) | {fmt(warm.elapsed)} | {rss} |")
    if not teles:
        for run in films:
            rss = f"{run.rss_kib:,} KiB" if run.rss_kib is not None else "—"
            lines.append(f"| {run.label} | — (film run, not telemetry) | "
                         f"{fmt(run.elapsed)} | {rss} |")
    lines.append("")
    return "\n".join(lines)


def disk_sets(logdir):
    sets = []
    for path in sorted(glob.glob(os.path.join(logdir, "*.sizes"))):
        sizes = []
        for line in open(path, encoding="utf-8", errors="replace"):
            parts = line.split()
            if parts:
                try:
                    sizes.append(int(parts[0]))
                except ValueError:
                    continue
        if sizes:
            sets.append((os.path.basename(path), sizes))
    for sub in sorted(os.listdir(logdir)):
        full = os.path.join(logdir, sub)
        if os.path.isdir(full):
            pngs = glob.glob(os.path.join(full, "frame_*.png"))
            if pngs:
                sets.append((f"{sub}/frame_*.png",
                             [os.path.getsize(p) for p in pngs]))
    return sets


def section_disk(logdir, renderer_line, notes):
    sets = disk_sets(logdir)
    if not sets:
        notes.append("§5 disk: no frame-size data (*.sizes files or "
                     "frame_*.png directories) in this directory.")
        return ""
    out = ["## §5 Disk (frame sizes → full-film budget)", ""]
    for name, sizes in sets:
        sizes = sorted(sizes)
        n = len(sizes)
        mean = sum(sizes) / n
        out += [f"### Frame set: `{name}`", "",
                renderer_header(renderer_line), "",
                "| quantity | value |", "|---|---|",
                f"| frames measured | {n} |",
                f"| min / median / mean / max PNG size | "
                f"{sizes[0]/1e6:.2f} / {statistics.median(sizes)/1e6:.2f} / "
                f"{mean/1e6:.2f} / {sizes[-1]/1e6:.2f} MB |",
                f"| full-film budget @ mean × 1811 | "
                f"{mean * FULL_FILM_FRAMES / 1e9:.2f} GB |",
                f"| full-film bound @ max × 1811 | "
                f"{sizes[-1] * FULL_FILM_FRAMES / 1e9:.2f} GB |",
                ""]
    return "\n".join(out)


def section_compare(runs, logdir, renderer_line, notes):
    films = [r for r in runs.values() if r.kind == "film"]
    native = next((r for r in films if r.stem == "chunk-warm"), None) \
        or next((r for r in films if r.stem == "bench-default"), None) \
        or (films[0] if films else None)
    chrome = runs.get("chrome")
    if native is None and chrome is None:
        notes.append("§6 comparison: no native film run and no "
                     "chrome.time in this directory.")
        return ""
    lines = ["## §6 The comparison that matters: native vs Chrome "
             "capture (same PC)", "",
             renderer_header(renderer_line), "",
             "The two routes do not produce the same artifact, and this "
             "table must never be quoted without that fact: **native** "
             "writes clean PNG masters (decision B — PNG encoding cost "
             "is inside the native time); **Chrome** frames come through "
             "the browser pipeline and the legacy masters carry a "
             "deliberate q0.93 JPEG generation. Neither number includes "
             "the shared ffmpeg finishing step. (PERF §6.)", "",
             "| route | frames | wall time s | s/frame | frames/hour | "
             "output artifact |",
             "|---|---|---|---|---|---|"]
    native_frames = None
    if native is not None:
        native_frames = native.completion["rendered"] \
            if native.completion else None
        spf, fph = spf_fph(native.elapsed, native_frames)
        lines.append(
            f"| TaleRender native (`--film`, {native.label}) | "
            f"{native_frames if native_frames else '—'} | "
            f"{fmt(native.elapsed)} | {fmt(spf, 3)} | "
            f"{f'{fph:,.0f}' if fph else '—'} | clean PNG |")
    if chrome is not None:
        frames = native_frames or 90
        npath = os.path.join(logdir, "chrome.frames")
        if os.path.exists(npath):
            try:
                frames = int(open(npath).read().strip())
            except ValueError:
                pass
        spf, fph = spf_fph(chrome.elapsed, frames)
        lines.append(
            f"| Chrome patched capture (same window) | {frames} | "
            f"{fmt(chrome.elapsed)} | {fmt(spf, 3)} | "
            f"{f'{fph:,.0f}' if fph else '—'} | browser-pipeline frames |")
    else:
        lines.append("| Chrome patched capture (same window) | — | — | "
                     "— | — | browser-pipeline frames |")
        lines.append("")
        lines.append("No Chrome timing in this log set. To fill this "
                     "row, save the capture step's GNU time output "
                     "(`%e %M` format) as `chrome.time` in the log "
                     "directory (and the frame count as `chrome.frames` "
                     "if it differs from the native row's).")
    lines.append("")
    return "\n".join(lines)


def main(argv):
    out_path = None
    pos = []
    i = 1
    while i < len(argv):
        if argv[i] == "--out":
            if i + 1 >= len(argv):
                fail("--out needs a path")
            out_path = argv[i + 1]
            i += 2
        elif argv[i].startswith("--"):
            print(f"bench_report: ERROR: unknown option {argv[i]}",
                  file=sys.stderr)
            return 2
        else:
            pos.append(argv[i])
            i += 1
    if len(pos) != 1:
        print("usage: bench_report.py <log-dir> [--out report.md]",
              file=sys.stderr)
        return 2
    logdir = pos[0]
    if not os.path.isdir(logdir):
        fail(f"not a directory: {logdir}")

    # --- renderer census: the labelling law -------------------------
    renderers = {}   # remainder -> first verbatim full line
    env_text = ""
    env_path = os.path.join(logdir, "env.txt")
    if os.path.exists(env_path):
        env_text = open(env_path, encoding="utf-8",
                        errors="replace").read()
    texts = [env_text] if env_text else []
    for name in sorted(os.listdir(logdir)):
        if name.endswith(".log"):
            texts.append(open(os.path.join(logdir, name),
                              encoding="utf-8", errors="replace").read())
    for text in texts:
        for m in RENDERER_RE.finditer(text):
            remainder = re.sub(r"\s+", " ", m.group(2)).strip()
            renderers.setdefault(remainder, m.group(0).strip())
    if not renderers:
        fail("no `renderer:`/`adapter:` line found in any log or "
             "env.txt — PERF.md requires every table to name its "
             "renderer, so this directory cannot be reported. "
             "(On the PC, copy the adapter line into perf/env.txt "
             "per PERF §1.)")
    if len(renderers) > 1:
        listing = "\n".join(f"  - {line}" for line in renderers.values())
        fail("logs from DIFFERENT renderers in one directory — "
             "refusing to merge them into one table (PERF.md: a "
             f"table's renderer must be unambiguous):\n{listing}\n"
             "Split the directory by renderer and report each set "
             "separately.")
    renderer_line = next(iter(renderers.values()))
    remainder = next(iter(renderers))
    software = ("llvmpipe" in remainder.lower()
                or "lavapipe" in remainder.lower()
                or "device type: Cpu" in remainder)

    runs = load_runs(logdir)
    if not runs:
        fail(f"no recognised bench logs in {logdir}")

    notes, warnings = [], []
    for run in runs.values():
        warnings.extend(run_warnings(run))

    title_dir = os.path.basename(os.path.abspath(logdir))
    head = [f"# TaleRender bench report — `{title_dir}`", "",
            renderer_header(renderer_line), ""]
    m_sha = SHA_RE.search(env_text)
    if m_sha:
        head.append(f"**Binary:** sha256 `{m_sha.group(1)}` "
                    f"({m_sha.group(2)})")
        head.append("")
    if software:
        head += ["> ⚠ **SOFTWARE RENDERER — INDICATIVE ONLY.** The "
                 "logs name a software Vulkan device (lavapipe / "
                 "llvmpipe, CPU). These numbers are **not hardware "
                 "performance** and must never be quoted as such "
                 "(PERF.md honesty rule). The real numbers come "
                 "from the RADV run on the user's PC.", ""]
    sources = sorted({r.elapsed_source for r in runs.values()
                      if r.elapsed_source})
    if sources:
        head.append("Elapsed sources, in preference order as applied "
                    "per run: " + "; ".join(sources) + ".")
        head.append("")

    body = []
    for section in (section_stills, section_film, section_memory):
        text = section(runs, renderer_line, notes) \
            if section is not section_film else \
            section_film(runs, renderer_line, notes, warnings)
        if text:
            body.append(text)
    text = section_disk(logdir, renderer_line, notes)
    if text:
        body.append(text)
    text = section_compare(runs, logdir, renderer_line, notes)
    if text:
        body.append(text)

    tail = []
    if warnings:
        tail += ["## Warnings", ""] + [f"- {w}" for w in warnings] + [""]
    if notes:
        tail += ["## Not in this log set", ""] + \
                [f"- {n}" for n in notes] + [""]
    report = "\n".join(head + body + tail)
    if out_path:
        with open(out_path, "w", encoding="utf-8") as fh:
            fh.write(report)
        print(f"bench_report: wrote {out_path}")
    else:
        print(report)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
