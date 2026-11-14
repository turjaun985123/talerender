#!/usr/bin/env python3
"""Visual regression — same-renderer drift check against the native
golden pins (docs/golden/NATIVE-GOLDEN.md is the policy).

WHAT THIS IS
    The golden pack (tools/golden/make_golden.py) verifies that the
    reference files ON DISK still hash to their pins. It cannot see
    the renderer at all. This runner closes the other half: it takes
    FRESHLY RENDERED frames, compares each one against the pinned
    reference for its item, and reports per-pin drift.

THE LAW THIS TOOL OBEYS (and the one it refuses to break)
    * Cross-renderer A/B numbers (native vs Chrome, native lavapipe
      vs native RADV) are REPORTED, never gated as equality — the
      Chrome side sits behind a q0.93 JPEG generation and the GRADE
      grain's fp32 sin() is vendor territory (contract §2 of
      docs/integration/DETERMINISM-CONTRACT-W04.md). Nothing in this
      tool compares across renderers.
    * SAME-renderer drift is different: contract §4 / the
      determinism contract §1 promise that same scene data + same
      binary + same renderer gives byte-identical frames. A fresh
      CI-lavapipe render that no longer matches a CI-lavapipe pin
      is a regression signal, and this runner FAILS on it. The pins
      this gate is valid for are the CI lavapipe pins — feeding it
      RADV renders would be a cross-renderer comparison and is out
      of scope by construction (the workflow that calls this runs
      on lavapipe only; see .github/workflows/visual-regression.yml).

METRICS — the house pair tool's math, imported, not reinvented
    PSNR / mean |d| / %px>10 / max d come from pair_metrics() in
    tools/ab/measure_ab.py (the house metric set: PSNR =
    10*log10(255^2 / MSE), MSE over all channels, float64 RGB —
    identical definitions to docs/ab/scene-format/scripts/
    ab_metrics.py and the M5/M6 packages). This module IMPORTS that
    function so the definitions can never drift apart. One
    deliberate difference from the A/B tool: a size mismatch is
    DRIFT here, never a silent LANCZOS resize — in a regression
    check, a changed frame size IS the drift.

CLASSIFICATION (per pinned item that was rendered)
    MATCH             rendered file's sha256 == pinned sha256
                      (byte-identical — the post-cabcc30 norm).
    WITHIN-BAND       bytes differ but PSNR >= band (default
                      80 dB). Includes pixel-identical-but-
                      re-encoded files (PSNR inf, bytes differ).
                      A signal to look, not a failure: the
                      same-renderer expectation is byte-identity,
                      so any WITHIN-BAND row wants an explanation
                      in the run's notes.
    DRIFT             PSNR < band, or size mismatch. FAILS.
    MISSING-RENDER    a rendered path was supplied but the file is
                      not there. FAILS (a render that did not
                      happen must not read as a pass).
    REFERENCE-MISSING an item is pinned but its reference file is
                      gone from the repo. FAILS (repo breakage).
    UNPINNED          the item exists in the golden manifest but
                      has no pin in golden-hashes.json — today:
                      all 14 film segment mids (staged candidates
                      in docs/golden/frames/ and frames-f021991/
                      are NOT pins; pinning is a human act).
                      Reported, never failed, no PSNR computed —
                      a candidate staging is not a reference.
    NOT-RENDERED      pinned, but no rendered frame was supplied
                      for it in this run. Reported, never failed.

THE BAND — 80 dB, and why (numbers from the project's own record)
    * Post-fix norm is byte-identity: after cabcc30 (exact f32
      paint-cache keys; E0600 follow-up f021991), the lead's CI
      lavapipe probes at the final head re-rendered frames
      1195-1204 10/10, 795-814 20/20, 195-204 10/10 byte-identical
      (DETERMINISM-CONTRACT-W04.md §5, E4). Anything measurably
      short of inf is already abnormal.
    * The one same-renderer defect on record — the pre-fix cache-
      key collision — measured 72.34 dB (frames 1195-1199, E2).
      A band at or below 72.34 dB would wave that exact defect
      class through as "within band". The band must sit above it.
    * 80 dB in absolute terms: MSE = 255^2 / 10^8 ~= 6.5e-4, i.e.
      ~0.065% of channel samples wrong by a full LSB (or far fewer
      wrong by more). A single wrong pixel in a 1080x1920 frame
      sits at ~116 dB; ordinary rounding flutter, if a driver
      ever produced any, lands in the high double digits of dB.
      80 dB is >7 dB above the worst defect on record and >35 dB
      below single-pixel noise: tight, but not a byte-compare in
      disguise. It is a CLI parameter (--band) — changing it is a
      policy act and belongs in a commit message, not a run.

USAGE
    python3 tools/ci/visual_regression.py \
        --render ab1-composer=render-out/ab1.png \
        --render dof-chart=render-out/dof-chart.png \
        --render film-s08b=render-out/frames/frame_01231.png \
        [--band 80] [--out visual-regression.md] [--repo .]

    --render ID=PATH is repeatable; ID is a golden manifest id
    (tools/golden/make_golden.py ITEMS). --reference ID=PATH
    overrides a pinned item's reference path (synthetic testing;
    the CI workflow never uses it). --pins-file PATH overrides
    tools/golden/golden-hashes.json (ditto).

EXIT CODES — the exit code agrees with the verdict line
    0   VISUAL REGRESSION: PASS (n pins, m drift)   [m == 0]
    1   VISUAL REGRESSION: FAIL (n pins, m drift)   [m >= 1]
    2   input error — one ERROR line on stderr, NO verdict line
        printed (a broken run is no evidence, never a soft pass;
        same refusal discipline as tools/determinism).
    In the verdict, n = pinned items actually measured this run
    (MATCH + WITHIN-BAND + DRIFT + MISSING-RENDER +
    REFERENCE-MISSING); m = DRIFT + MISSING-RENDER +
    REFERENCE-MISSING. UNPINNED and NOT-RENDERED items are listed
    in the table and counted in the summary, never in n or m.
"""

import argparse
import hashlib
import importlib.util
import json
import math
import sys
from pathlib import Path

DEFAULT_BAND_DB = 80.0


class InputError(Exception):
    """Bad input data. Reported as one named line on stderr with
    exit 2 instead of a traceback (house convention)."""


# ------------------------------------------------------------------ pins

def load_golden_state(repo: Path, pins_file: Path | None):
    """Item registry + pins, from the golden tool itself.

    ITEMS is imported from tools/golden/make_golden.py so this
    runner can never disagree with the pack about which items
    exist or where their references live. Pins come from
    golden-hashes.json (or --pins-file) as an id -> sha256 dict,
    validated to the same shape make_golden.load_pins enforces.
    """
    make_golden_path = repo / "tools" / "golden" / "make_golden.py"
    if not make_golden_path.is_file():
        raise InputError(f"golden tool not found: {make_golden_path}")
    spec = importlib.util.spec_from_file_location("make_golden",
                                                  make_golden_path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    items = module.ITEMS

    pins_path = pins_file if pins_file is not None else (
        repo / "tools" / "golden" / "golden-hashes.json")
    if not pins_path.exists():
        pins = {}
    else:
        try:
            pins = json.loads(pins_path.read_text())
        except (OSError, ValueError) as e:
            raise InputError(f"{pins_path}: cannot load pins ({e})")
        if not isinstance(pins, dict) or not all(
                isinstance(k, str) and isinstance(v, str)
                for k, v in pins.items()):
            raise InputError(f"{pins_path}: must be a JSON object "
                             f"mapping item id to sha256 hex string")
    return items, pins


def load_house_metrics(repo: Path):
    """pair_metrics() imported from the house A/B tool.

    tools/ab/measure_ab.py is the single definition of the house
    metric set; importing it (instead of copying the formulas)
    means a future fix to the definition lands here too.
    """
    measure_path = repo / "tools" / "ab" / "measure_ab.py"
    if not measure_path.is_file():
        raise InputError(f"house A/B tool not found: {measure_path}")
    spec = importlib.util.spec_from_file_location("measure_ab",
                                                  measure_path)
    module = importlib.util.module_from_spec(spec)
    try:
        spec.loader.exec_module(module)
    except ImportError as e:
        raise InputError(f"cannot import house A/B tool "
                         f"({measure_path}): {e} — this runner "
                         f"needs numpy + Pillow, the pinned tools "
                         f"stack (see .github/workflows/tools-ci.yml)")
    return module.pair_metrics


# ------------------------------------------------------------------ args

def parse_id_paths(pairs, flag):
    out = {}
    for raw in pairs or []:
        if "=" not in raw:
            raise InputError(f"{flag} expects ID=PATH, got '{raw}'")
        item_id, path = raw.split("=", 1)
        if not item_id or not path:
            raise InputError(f"{flag} expects ID=PATH, got '{raw}'")
        if item_id in out:
            raise InputError(f"{flag}: duplicate id '{item_id}'")
        out[item_id] = Path(path)
    return out


# ------------------------------------------------------------------ core

def sha256_of(path: Path) -> str:
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def load_rgb_array(path: Path, what: str):
    import numpy as np
    from PIL import Image
    try:
        with Image.open(path) as img:
            rgb = img.convert("RGB")
            return np.asarray(rgb, dtype=np.float64), rgb.size
    except (OSError, ValueError) as e:
        raise InputError(f"cannot read {what} image {path} ({e})")


def fmt_psnr(v: float) -> str:
    return "inf" if math.isinf(v) else f"{v:.2f}"


def check_item(item, pin, rendered_path, reference_override, repo,
               band_db, pair_metrics):
    """One manifest item -> one report row (dict).

    Status semantics are in the module docstring. Only DRIFT,
    MISSING-RENDER and REFERENCE-MISSING fail the run.
    """
    item_id = item["id"]
    row = {"id": item_id, "status": None, "psnr": None,
           "mean_abs": None, "pct_gt10": None, "max_delta": None,
           "pin_short": (pin or "-")[:12], "rendered_sha": "-",
           "note": ""}

    if pin is None:
        row["status"] = "UNPINNED"
        row["note"] = ("no pin in golden-hashes.json — staged "
                       "candidates are not references; pinning is "
                       "a human act (NATIVE-GOLDEN.md)")
        return row

    if rendered_path is None:
        row["status"] = "NOT-RENDERED"
        row["note"] = "pinned, but no rendered frame supplied this run"
        return row

    reference_path = reference_override or (repo / item["path"])
    if not reference_path.is_file():
        row["status"] = "REFERENCE-MISSING"
        row["note"] = f"pinned reference not in repo: {reference_path}"
        return row

    if not rendered_path.is_file():
        row["status"] = "MISSING-RENDER"
        row["note"] = f"rendered file not found: {rendered_path}"
        return row

    rendered_sha = sha256_of(rendered_path)
    row["rendered_sha"] = rendered_sha[:12]
    if rendered_sha == pin:
        row["status"] = "MATCH"
        row["psnr"] = float("inf")
        row["note"] = "sha256 == pin (byte-identical)"
        return row

    ref_arr, ref_size = load_rgb_array(reference_path, "reference")
    nat_arr, nat_size = load_rgb_array(rendered_path, "rendered")
    if nat_size != ref_size:
        row["status"] = "DRIFT"
        row["note"] = (f"size mismatch: rendered {nat_size[0]}x"
                       f"{nat_size[1]} vs reference {ref_size[0]}x"
                       f"{ref_size[1]} (never resized away)")
        return row

    m = pair_metrics(ref_arr, nat_arr)
    row["psnr"] = m["psnr_db"]
    row["mean_abs"] = m["mean_abs_delta_per_channel"]
    row["pct_gt10"] = m["pct_pixels_max_channel_delta_gt_10"]
    row["max_delta"] = m["max_abs_delta"]
    if math.isinf(m["psnr_db"]):
        row["status"] = "WITHIN-BAND"
        row["note"] = ("pixels identical, file bytes differ "
                       "(re-encode/metadata — not pixel drift)")
    elif m["psnr_db"] >= band_db:
        row["status"] = "WITHIN-BAND"
        row["note"] = (f"bytes differ, PSNR >= band {band_db:g} dB — "
                       f"same-renderer expectation is byte-identity; "
                       f"this row wants an explanation")
    else:
        row["status"] = "DRIFT"
        row["note"] = f"PSNR < band {band_db:g} dB — same-renderer drift"
    return row


FAILING = {"DRIFT", "MISSING-RENDER", "REFERENCE-MISSING"}
MEASURED = {"MATCH", "WITHIN-BAND"} | FAILING


def render_report(rows, band_db, verdict, n_measured, n_drift):
    lines = [
        "# Visual regression — same-renderer drift vs the native golden pins",
        "",
        "Scope: CI lavapipe renders vs CI lavapipe pins ONLY. "
        "Same-renderer drift is gated; cross-renderer numbers are "
        "reported elsewhere and never gated as equality "
        "(docs/golden/NATIVE-GOLDEN.md; determinism contract §2).",
        f"Band: PSNR >= {band_db:g} dB = WITHIN-BAND; below = DRIFT. "
        "Byte-identity (MATCH) is the post-cabcc30 norm; the band "
        "exists so a sub-defect flutter is classified, not hidden. "
        "See tools/ci/visual_regression.py header for the full "
        "justification (defect on record: 72.34 dB).",
        "",
        "| pin | status | PSNR (dB) | mean \\|d\\| | %px > 10 | max d | pinned sha | rendered sha | note |",
        "|---|---|---|---|---|---|---|---|---|",
    ]
    for r in rows:
        psnr = fmt_psnr(r["psnr"]) if r["psnr"] is not None else "—"
        mean_abs = f"{r['mean_abs']:.5f}" if r["mean_abs"] is not None else "—"
        pct = f"{r['pct_gt10']:.5f}" if r["pct_gt10"] is not None else "—"
        maxd = str(r["max_delta"]) if r["max_delta"] is not None else "—"
        lines.append(
            f"| {r['id']} | {r['status']} | {psnr} | {mean_abs} | "
            f"{pct} | {maxd} | {r['pin_short']} | {r['rendered_sha']} "
            f"| {r['note']} |")
    counts = {}
    for r in rows:
        counts[r["status"]] = counts.get(r["status"], 0) + 1
    summary = " · ".join(f"{counts[s]} {s}" for s in
                         ["MATCH", "WITHIN-BAND", "DRIFT",
                          "MISSING-RENDER", "REFERENCE-MISSING",
                          "UNPINNED", "NOT-RENDERED"] if s in counts)
    lines += ["", f"Counts: {summary} (of {len(rows)} golden items).",
              "", verdict, ""]
    return "\n".join(lines)


def main() -> int:
    ap = argparse.ArgumentParser(
        description="Same-renderer visual regression vs the native "
                    "golden pins. See the module docstring for the "
                    "law, the band justification and exit codes.",
        formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--render", action="append", default=[],
                    metavar="ID=PATH",
                    help="a freshly rendered frame for golden item "
                         "ID (repeatable)")
    ap.add_argument("--reference", action="append", default=[],
                    metavar="ID=PATH",
                    help="override the reference path for item ID "
                         "(synthetic testing; CI never uses this)")
    ap.add_argument("--band", type=float, default=DEFAULT_BAND_DB,
                    help=f"WITHIN-BAND threshold in dB "
                         f"(default {DEFAULT_BAND_DB:g})")
    ap.add_argument("--repo", type=Path,
                    default=Path(__file__).resolve().parents[2],
                    help="repo root (default: this file's repo)")
    ap.add_argument("--pins-file", type=Path, default=None,
                    help="override tools/golden/golden-hashes.json")
    ap.add_argument("--out", type=Path, default=None,
                    help="also write the markdown report here")
    args = ap.parse_args()

    try:
        renders = parse_id_paths(args.render, "--render")
        references = parse_id_paths(args.reference, "--reference")
        items, pins = load_golden_state(args.repo, args.pins_file)
        known = {item["id"] for item in items}
        for flag, mapping in (("--render", renders),
                              ("--reference", references)):
            unknown = sorted(set(mapping) - known)
            if unknown:
                raise InputError(
                    f"{flag}: unknown golden item id(s): "
                    f"{', '.join(unknown)} (registry: "
                    f"tools/golden/make_golden.py ITEMS)")
        if not renders:
            raise InputError("no --render ID=PATH given — nothing "
                             "to measure, no verdict claimed")
        pair_metrics = load_house_metrics(args.repo)

        rows = [check_item(item, pins.get(item["id"]),
                           renders.get(item["id"]),
                           references.get(item["id"]),
                           args.repo, args.band, pair_metrics)
                for item in items]
    except InputError as e:
        print(f"ERROR: visual_regression.py: {e}", file=sys.stderr)
        return 2

    n_measured = sum(1 for r in rows if r["status"] in MEASURED)
    n_drift = sum(1 for r in rows if r["status"] in FAILING)
    if n_measured == 0:
        print("ERROR: visual_regression.py: no pinned item was "
              "rendered — nothing measured, no verdict claimed",
              file=sys.stderr)
        return 2
    verdict_word = "FAIL" if n_drift else "PASS"
    verdict = (f"VISUAL REGRESSION: {verdict_word} "
               f"({n_measured} pins, {n_drift} drift)")
    report = render_report(rows, args.band, verdict,
                           n_measured, n_drift)
    print(report)
    if args.out is not None:
        try:
            args.out.write_text(report)
        except OSError as e:
            print(f"ERROR: visual_regression.py: cannot write "
                  f"--out {args.out} ({e})", file=sys.stderr)
            return 2
    return 1 if n_drift else 0


if __name__ == "__main__":
    sys.exit(main())
