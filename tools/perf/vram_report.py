#!/usr/bin/env python3
"""vram_report.py — accounted GPU-memory report from TaleRender run logs.

Wave 2 seat W56 prototype for the design in
docs/integration/VRAM-TELEMETRY-W56.md (PERF.md §9 item 5, the
GPU-memory blind spot).

What it does: parse the end-of-run stats lines a TaleRender run
prints —

  * the film completion line's runlog suffix  (`peak RSS <f> MB`,
    format pinned in src/runlog.rs `summary_suffix`),
  * `paint-cache: entries=.. bytes=.. hits=.. misses=.. evictions=..
    rerenders=.. peak_bytes=.. budget_bytes=..`
    (src/paintbudget.rs `stats_line`, format pinned by its test),
  * `buffer-pool: retained_bytes=.. free=.. hits=.. misses=..
    evictions=.. peak_retained_bytes=.. budget_bytes=..`
    (src/bufferpool.rs `stats_line`, format pinned by its test),
  * `gpu-mem: accounted_bytes=.. peak_accounted_bytes=.. textures=..
    buffers=.. readback_bytes=.. sysfs_gtt_used_bytes=..
    sysfs_vram_used_bytes=..`
    (PROPOSED by VRAM-TELEMETRY-W56.md §3 — no binary prints this
    yet; the fixture under fixtures/vram/ carries a synthetic one).

— and print the accounted-total report: every component named with
its class (device-accounted vs host) and the sysfs counters when
the log carries them.

What it must never do: invent a total the components do not
support. On the target PC (Ryzen 5 PRO 5650G, Vega 7 iGPU) device
memory is carved out of the same system RAM the process RSS
measures, and whether a given device allocation is also inside
the process's RSS depends on the driver's mapping. The report
therefore prints the components side by side and one REFERENCE
sum, labelled as such — it is an arithmetic convenience for
eyeballing pressure against the machine's ~6.7 GiB, NOT a
measurement. See the design note §5 for what stays unknowable.

House conventions (same as the other tools/ci + tools/perf
tools): stdlib only; exit 0 = report printed / self-test passed,
exit 1 = self-test failure or a required stats line missing,
exit 2 = usage / input error (one `ERROR:` line on stderr, no
report).

Usage:
    python3 tools/perf/vram_report.py LOGFILE
    python3 tools/perf/vram_report.py --self-test
"""

import re
import sys
from pathlib import Path

MIB = 1024 * 1024

# --- line grammars (each mirrors the pinned Rust format) ---------------

PAINT_CACHE_RE = re.compile(
    r"paint-cache: entries=(\d+) bytes=(\d+) hits=(\d+) misses=(\d+) "
    r"evictions=(\d+) rerenders=(\d+) peak_bytes=(\d+) budget_bytes=(\d+)"
)
BUFFER_POOL_RE = re.compile(
    r"buffer-pool: retained_bytes=(\d+) free=(\d+) hits=(\d+) misses=(\d+) "
    r"evictions=(\d+) peak_retained_bytes=(\d+) budget_bytes=(\d+)"
)
GPU_MEM_RE = re.compile(
    r"gpu-mem: accounted_bytes=(\d+) peak_accounted_bytes=(\d+) "
    r"textures=(\d+) buffers=(\d+) readback_bytes=(\d+) "
    r"sysfs_gtt_used_bytes=(\d+|unavailable) "
    r"sysfs_vram_used_bytes=(\d+|unavailable)"
)
PEAK_RSS_RE = re.compile(r"peak RSS (\d+(?:\.\d+)?) MB")


def parse_log(text):
    """Extract the last occurrence of each stats line (a log may
    hold several runs; end-of-run lines are what count)."""
    found = {}
    for line in text.splitlines():
        m = PAINT_CACHE_RE.search(line)
        if m:
            keys = ("entries", "bytes", "hits", "misses", "evictions",
                    "rerenders", "peak_bytes", "budget_bytes")
            found["paint_cache"] = {k: int(v) for k, v in zip(keys, m.groups())}
        m = BUFFER_POOL_RE.search(line)
        if m:
            keys = ("retained_bytes", "free", "hits", "misses", "evictions",
                    "peak_retained_bytes", "budget_bytes")
            found["buffer_pool"] = {k: int(v) for k, v in zip(keys, m.groups())}
        m = GPU_MEM_RE.search(line)
        if m:
            g = m.groups()
            found["gpu_mem"] = {
                "accounted_bytes": int(g[0]),
                "peak_accounted_bytes": int(g[1]),
                "textures": int(g[2]),
                "buffers": int(g[3]),
                "readback_bytes": int(g[4]),
                "sysfs_gtt_used_bytes": None if g[5] == "unavailable" else int(g[5]),
                "sysfs_vram_used_bytes": None if g[6] == "unavailable" else int(g[6]),
            }
        m = PEAK_RSS_RE.search(line)
        if m:
            found["peak_rss_mb"] = float(m.group(1))
    return found


def mib(b):
    return b / MIB


def report(found):
    lines = []
    pc = found.get("paint_cache")
    bp = found.get("buffer_pool")
    gm = found.get("gpu_mem")
    rss_mb = found.get("peak_rss_mb")

    lines.append("TaleRender accounted-memory report")
    lines.append("")

    lines.append("DEVICE (GPU-side, renderer's own ledger):")
    if gm:
        lines.append(
            f"  accounted (end of run) : {gm['accounted_bytes']:>13,} B "
            f"({mib(gm['accounted_bytes']):.1f} MiB)  "
            f"[{gm['textures']} textures, {gm['buffers']} buffers]"
        )
        lines.append(
            f"  accounted (peak)       : {gm['peak_accounted_bytes']:>13,} B "
            f"({mib(gm['peak_accounted_bytes']):.1f} MiB)"
        )
        lines.append(
            f"  of which readback      : {gm['readback_bytes']:>13,} B "
            f"({mib(gm['readback_bytes']):.1f} MiB)  (transient staging, in the peak)"
        )
    else:
        lines.append("  no gpu-mem line in log — device bytes UNACCOUNTED")
        lines.append("  (the gpu-mem line is proposed in VRAM-TELEMETRY-W56.md §3;")
        lines.append("   no binary prints it yet)")
    if bp:
        lines.append(
            f"  buffer-pool peak       : {bp['peak_retained_bytes']:>13,} B "
            f"({mib(bp['peak_retained_bytes']):.1f} MiB)  "
            f"(budget {bp['budget_bytes']:,} B; class of the readback bytes above)"
        )
    lines.append("")

    lines.append("HOST (CPU-side):")
    if rss_mb is not None:
        lines.append(f"  peak RSS (runlog)      : {rss_mb:>13.1f} MB (as printed, MB = MiB)")
    else:
        lines.append("  peak RSS               : not in log")
    if pc:
        lines.append(
            f"  paint-cache peak       : {pc['peak_bytes']:>13,} B "
            f"({mib(pc['peak_bytes']):.1f} MiB)  (budget {pc['budget_bytes']:,} B)"
        )
        lines.append("    ^ host Vec<u8> canvases — already inside peak RSS;")
        lines.append("      listed for the split, never added on top of it.")
    lines.append("")

    lines.append("OS COUNTERS (device-wide, all processes — not this run alone):")
    if gm and (gm["sysfs_gtt_used_bytes"] is not None
               or gm["sysfs_vram_used_bytes"] is not None):
        for key, label in (("sysfs_gtt_used_bytes", "amdgpu GTT used "),
                           ("sysfs_vram_used_bytes", "amdgpu VRAM used")):
            v = gm[key]
            if v is not None:
                lines.append(f"  {label}       : {v:>13,} B ({mib(v):.1f} MiB)")
            else:
                lines.append(f"  {label}       : unavailable")
    else:
        lines.append("  unavailable in this log (no amdgpu sysfs on the run machine,")
        lines.append("  e.g. CI lavapipe — see the design note §2.4; never read as 0)")
    lines.append("")

    if gm and rss_mb is not None:
        ref = gm["peak_accounted_bytes"] + int(rss_mb * MIB)
        lines.append(
            f"REFERENCE SUM (not a measurement): peak accounted device "
            f"{gm['peak_accounted_bytes']:,} B + peak RSS {rss_mb:.1f} MB "
            f"= {ref:,} B ({mib(ref):.1f} MiB)."
        )
        lines.append(
            "  On the shared-memory 5650G some device allocations may already"
        )
        lines.append(
            "  sit inside RSS, so this sum can double-count; driver-internal"
        )
        lines.append(
            "  overhead is in neither component, so it can also under-count."
        )
        lines.append(
            "  Use it only as a pressure eyeball against the PC's ~6.7 GiB."
        )
    return "\n".join(lines)


# --- self-test ------------------------------------------------------------

FIXTURE = Path(__file__).parent / "fixtures" / "vram" / "film-run-tail.log"

# Every expectation below is derivable from the fixture by hand:
# the gpu-mem figures are the design note §4 formula totals, the
# paint-cache peak is 74 canvases x 28,835,840 B (paintbudget.rs
# CANVAS_BYTES), the readback figure is bufferpool.rs's pinned
# FRAME_READBACK_BYTES (4352 x 1920).
EXPECTED = {
    "gpu_accounted": 220_930_688,
    "gpu_peak": 229_286_528,
    "gpu_textures": 41,
    "readback": 8_355_840,
    "paint_peak": 2_133_852_160,
    "paint_budget": 2_147_483_648,
    "pool_peak": 8_355_840,
    "pool_budget": 67_108_864,
    "rss_mb": 2314.6,
}


def self_test():
    checks = []

    def check(name, ok, detail=""):
        checks.append((name, ok, detail))

    if not FIXTURE.is_file():
        print(f"ERROR: fixture missing: {FIXTURE}", file=sys.stderr)
        return 2
    found = parse_log(FIXTURE.read_text())

    gm, pc, bp = found.get("gpu_mem"), found.get("paint_cache"), found.get("buffer_pool")
    check("gpu-mem line parsed", gm is not None)
    check("paint-cache line parsed", pc is not None)
    check("buffer-pool line parsed", bp is not None)
    check("peak RSS parsed", found.get("peak_rss_mb") == EXPECTED["rss_mb"],
          f"got {found.get('peak_rss_mb')}")
    if gm:
        check("accounted == §4 formula total", gm["accounted_bytes"] == EXPECTED["gpu_accounted"],
              f"got {gm['accounted_bytes']}")
        check("peak == accounted + one frame readback",
              gm["peak_accounted_bytes"] == EXPECTED["gpu_peak"]
              and gm["peak_accounted_bytes"] - gm["accounted_bytes"] == EXPECTED["readback"])
        check("textures counted", gm["textures"] == EXPECTED["gpu_textures"])
        check("readback == pinned FRAME_READBACK_BYTES", gm["readback_bytes"] == EXPECTED["readback"])
        check("sysfs fields parse as unavailable (not 0)",
              gm["sysfs_gtt_used_bytes"] is None and gm["sysfs_vram_used_bytes"] is None)
    if pc:
        check("paint peak == 74 x CANVAS_BYTES", pc["peak_bytes"] == EXPECTED["paint_peak"],
              f"got {pc['peak_bytes']}")
        check("paint peak under budget", pc["peak_bytes"] <= pc["budget_bytes"] == EXPECTED["paint_budget"])
    if bp:
        check("pool peak == one readback buffer", bp["peak_retained_bytes"] == EXPECTED["pool_peak"])
        check("pool budget == 64 MiB default", bp["budget_bytes"] == EXPECTED["pool_budget"])

    # A log with no stats lines must yield an empty parse, not a crash
    # and not fabricated zeros.
    empty = parse_log("some unrelated output\nnothing to see\n")
    check("empty log parses to no components", empty == {}, f"got {empty}")

    rep = report(found)
    check("report labels the reference sum as not-a-measurement",
          "REFERENCE SUM (not a measurement)" in rep)
    check("report states the double-count caveat", "double-count" in rep)

    failed = 0
    for name, ok, detail in checks:
        print(f"  [{'PASS' if ok else 'FAIL'}] {name}" + (f" — {detail}" if detail and not ok else ""))
        failed += 0 if ok else 1
    if failed:
        print(f"SELF-TEST: FAIL ({failed} check(s) failed)")
        return 1
    print(f"SELF-TEST: PASS ({len(checks)} checks)")
    print()
    print(report(found))
    return 0


def main(argv):
    if len(argv) == 2 and argv[1] == "--self-test":
        return self_test()
    if len(argv) != 2:
        print("ERROR: usage: vram_report.py LOGFILE | --self-test", file=sys.stderr)
        return 2
    path = Path(argv[1])
    if not path.is_file():
        print(f"ERROR: no such log file: {path}", file=sys.stderr)
        return 2
    found = parse_log(path.read_text(errors="replace"))
    if not found:
        print("ERROR: no recognised stats lines in log "
              "(paint-cache / buffer-pool / gpu-mem / peak RSS)", file=sys.stderr)
        return 1
    print(report(found))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
