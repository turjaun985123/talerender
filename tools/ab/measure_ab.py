#!/usr/bin/env python3
"""A/B measurement for TaleRender film frames vs Chrome review frames.

Compares native frames (TaleRender, clean PNG — contract decision B)
against the project's Chrome review frames (JPEG, one q0.93 generation
from the canvas) at matched timestamps, using the house metric set
(identical definitions to docs/ab/scene-format/scripts/ab_metrics.py
and the M5/M6 packages):

  PSNR (dB)   10*log10(255^2 / MSE), MSE over all channels (float64)
  mean |d|    mean absolute delta per channel
  %px > 10    % of pixels whose MAX-channel |delta| > 10
  (plus %px > 0 and max |delta| for the record)

Film mode (the normal use):

  measure_ab.py --native-dir FRAMES --review-dir REVIEW_FRAMES \
      --film scenes/film/film.json --out OUT_DIR [--fps 30]

  * Review frames are discovered as t<seconds>.jpg/.png in the
    review dir (the project writes t0.50.jpg .. t59.50.jpg).
  * Native frames may be named by frame number (frame_00042.png,
    frame-00042.png, f00042.png, 00042.png — the film driver writes
    frame_{n:05}.png) or by time (t1.50.png, 1.50.png).
  * For every review timestamp inside a film segment (film.json),
    the native frame at the same staged time (n = round(t*fps)) is
    paired with it. Every segment is guaranteed a row in the report;
    segments with no matched pair are reported as NO DATA with the
    reason — never silently dropped.
  * Writes ab-report.json + ab-report.md into OUT_DIR, plus
    side-by-side and |diff|x4 images (labelled with the renderer
    on each side) for the three lowest-PSNR pairs.

Pair mode (ad-hoc / tool proof):

  measure_ab.py --pair REFERENCE.png NATIVE.png --out OUT_DIR

Renderer labels are part of every output. Override with
--reference-label / --native-label when the defaults do not describe
the actual renders (e.g. native frames rendered on RADV vs lavapipe).

JPEG FLOOR (stated in every report): the reference carries one JPEG
generation (Chromium toDataURL image/jpeg 0.93, measured in
docs/ab/telemetry/jpeg-decision.md): flat/step content floors at
50.78-57.71 dB; a detailed frame re-encoded once measures 42.31 dB
against its own clean source. A perfect native frame cannot score
above that floor against these references, and the bias is
one-directional: reference JPEG error counts as native error, so
these numbers can only UNDERSTATE native accuracy.
"""
import argparse
import json
import math
import os
import re
import sys

import numpy as np
from PIL import Image, ImageDraw, ImageFont

DIFF_AMP = 4
LABEL_H = 30

DEFAULT_REFERENCE_LABEL = (
    "REFERENCE — production engine (three.js r170) in headless Chrome, "
    "review JPEG q~0.92 (one JPEG generation)"
)
DEFAULT_NATIVE_LABEL = (
    "NATIVE — talerender (wgpu), clean PNG (no JPEG generation, decision B)"
)

JPEG_FLOOR_NOTE = (
    "JPEG floor: the reference frames are JPEG (q~0.92, one generation "
    "from the Chrome canvas via Chromium toDataURL image/jpeg 0.93). "
    "Measured floor (docs/ab/telemetry/jpeg-decision.md): on flat/step "
    "content two otherwise-identical images agree no better than "
    "50.78-57.71 dB; on a detailed frame, one JPEG generation alone "
    "measures 42.31 dB against the clean source (mean |d| 1.225, "
    "94.47% of pixels changed, max d 48). A pixel-perfect native frame "
    "therefore cannot exceed ~42 dB (detailed) / ~51-58 dB (flat) "
    "against these references. The bias is one-directional: JPEG error "
    "in the reference is counted as native error, so every number "
    "here UNDERSTATES native accuracy; it can never overstate it."
)


# ---------------------------------------------------------------- metrics

def pair_metrics(ref: np.ndarray, nat: np.ndarray) -> dict:
    """House metric set; ref/nat are float64 RGB arrays of equal shape."""
    diff = np.abs(ref - nat)
    mse = float(np.mean((ref - nat) ** 2))
    psnr = float("inf") if mse == 0 else float(10 * np.log10(255.0**2 / mse))
    return {
        "psnr_db": psnr,
        "mean_abs_delta_per_channel": float(diff.mean()),
        "pct_pixels_max_channel_delta_gt_10": float((diff.max(axis=2) > 10).mean() * 100),
        "pct_pixels_any_channel_delta_gt_0": float((diff.max(axis=2) > 0).mean() * 100),
        "max_abs_delta": int(diff.max()),
    }


def fmt_psnr(v: float) -> str:
    return "inf" if math.isinf(v) else f"{v:.3f}"


def json_psnr(v: float):
    return "inf" if math.isinf(v) else round(v, 3)


# ---------------------------------------------------------------- images

def _font(size: int = 19):
    for path in (
        "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
    ):
        if os.path.exists(path):
            return ImageFont.truetype(path, size)
    return ImageFont.load_default()


def _label_band(width: int, left_text: str, right_text: str | None) -> Image.Image:
    band = Image.new("RGB", (width, LABEL_H), (16, 16, 20))
    d = ImageDraw.Draw(band)
    f = _font()
    d.text((8, 5), left_text, fill=(255, 255, 255), font=f)
    if right_text:
        tw = d.textlength(right_text, font=f)
        d.text((width - tw - 8, 5), right_text, fill=(255, 255, 255), font=f)
    return band


def side_by_side(ref_img: Image.Image, nat_img: Image.Image,
                 ref_label: str, nat_label: str) -> Image.Image:
    """House layout (each side half width, full height) + label band."""
    w, h = ref_img.size
    half = Image.new("RGB", (w, h))
    half.paste(ref_img.resize((w // 2, h)), (0, 0))
    half.paste(nat_img.resize((w - w // 2, h)), (w // 2, 0))
    band = _label_band(w, ref_label, nat_label)
    out = Image.new("RGB", (w, h + LABEL_H))
    out.paste(band, (0, 0))
    out.paste(half, (0, LABEL_H))
    return out


def diff_image(ref: np.ndarray, nat: np.ndarray, title: str) -> Image.Image:
    amplified = np.clip(np.abs(ref - nat) * DIFF_AMP, 0, 255).astype(np.uint8)
    img = Image.fromarray(amplified)
    band = _label_band(img.size[0], title, f"|delta| x{DIFF_AMP}")
    out = Image.new("RGB", (img.size[0], img.size[1] + LABEL_H))
    out.paste(band, (0, 0))
    out.paste(img, (0, LABEL_H))
    return out


# ---------------------------------------------------------------- discovery

_FRAME_RE = re.compile(r"^(?:frame[-_]?|f)?(\d+)$")
_TIME_RE = re.compile(r"^t?(\d+\.\d+)$")


def discover_native(frames_dir: str, fps: float) -> dict[int, str]:
    """frame index -> path, from frame-number or time based PNG names."""
    found: dict[int, str] = {}
    for name in sorted(os.listdir(frames_dir)):
        if not name.lower().endswith(".png"):
            continue
        stem = name[:-4]
        m = _TIME_RE.match(stem)
        if m:
            idx = int(round(float(m.group(1)) * fps))
        else:
            m = _FRAME_RE.match(stem)
            if not m:
                continue
            idx = int(m.group(1))
        found.setdefault(idx, os.path.join(frames_dir, name))
    return found


def discover_review(review_dir: str) -> dict[float, str]:
    """timestamp seconds -> path, from t<seconds>.jpg/.png names."""
    found: dict[float, str] = {}
    for name in sorted(os.listdir(review_dir)):
        low = name.lower()
        if not (low.endswith(".jpg") or low.endswith(".jpeg") or low.endswith(".png")):
            continue
        stem = low.rsplit(".", 1)[0]
        m = _TIME_RE.match(stem)
        if m:
            found.setdefault(float(m.group(1)), os.path.join(review_dir, name))
    return found


def load_rgb(path: str) -> Image.Image:
    return Image.open(path).convert("RGB")


# ---------------------------------------------------------------- core run

def measure_pair(ref_path: str, nat_path: str):
    """Returns (metrics, ref_img, nat_img, ref_arr, nat_arr, resize_note)."""
    ref_img = load_rgb(ref_path)
    nat_img = load_rgb(nat_path)
    resize_note = None
    if nat_img.size != ref_img.size:
        resize_note = f"native {nat_img.size[0]}x{nat_img.size[1]} resized to reference {ref_img.size[0]}x{ref_img.size[1]} (LANCZOS)"
        nat_img = nat_img.resize(ref_img.size, Image.LANCZOS)
    ref = np.asarray(ref_img, dtype=np.float64)
    nat = np.asarray(nat_img, dtype=np.float64)
    return pair_metrics(ref, nat), ref_img, nat_img, ref, nat, resize_note


def write_pair_images(out_dir: str, tag: str, ref_img, nat_img, ref, nat,
                      ref_label: str, nat_label: str) -> list[str]:
    sbs = side_by_side(ref_img, nat_img, "REFERENCE (Chrome, JPEG)", "NATIVE (talerender, PNG)")
    sbs_path = os.path.join(out_dir, f"{tag}_side-by-side.png")
    sbs.save(sbs_path)
    dif = diff_image(ref, nat, f"{tag} — abs diff, native vs reference")
    dif_path = os.path.join(out_dir, f"{tag}_diff.png")
    dif.save(dif_path)
    return [sbs_path, dif_path]


def report_header_md(title: str, ref_label: str, nat_label: str) -> list[str]:
    return [
        f"# {title}",
        "",
        f"- {ref_label}",
        f"- {nat_label}",
        "",
        f"> {JPEG_FLOOR_NOTE}",
        "",
    ]


def run_pair_mode(args) -> int:
    ref_path, nat_path = args.pair
    os.makedirs(args.out, exist_ok=True)
    m, ref_img, nat_img, ref, nat, resize_note = measure_pair(ref_path, nat_path)
    tag = "pair"
    images = write_pair_images(args.out, tag, ref_img, nat_img, ref, nat,
                               args.reference_label, args.native_label)
    record = {
        "mode": "pair",
        "reference": {"path": ref_path, "label": args.reference_label},
        "native": {"path": nat_path, "label": args.native_label},
        "size": [ref_img.size[0], ref_img.size[1]],
        "resize_note": resize_note,
        "metrics": {
            "psnr_db": json_psnr(m["psnr_db"]),
            "mean_abs_delta_per_channel": round(m["mean_abs_delta_per_channel"], 5),
            "pct_pixels_max_channel_delta_gt_10": round(m["pct_pixels_max_channel_delta_gt_10"], 5),
            "pct_pixels_any_channel_delta_gt_0": round(m["pct_pixels_any_channel_delta_gt_0"], 5),
            "max_abs_delta": m["max_abs_delta"],
        },
        "images": [os.path.basename(p) for p in images],
        "jpeg_floor_note": JPEG_FLOOR_NOTE,
    }
    with open(os.path.join(args.out, "ab-report.json"), "w") as f:
        json.dump(record, f, indent=2)
    lines = report_header_md("A/B — single pair", args.reference_label, args.native_label)
    lines += [
        f"- reference file: `{ref_path}`",
        f"- native file: `{nat_path}`",
        f"- size: {ref_img.size[0]}x{ref_img.size[1]}",
    ]
    if resize_note:
        lines.append(f"- NOTE: {resize_note}")
    lines += [
        "",
        "| PSNR (dB) | mean \\|d\\| | %px > 10 | %px > 0 | max d |",
        "|---|---|---|---|---|",
        f"| {fmt_psnr(m['psnr_db'])} | {m['mean_abs_delta_per_channel']:.5f} | "
        f"{m['pct_pixels_max_channel_delta_gt_10']:.5f} | "
        f"{m['pct_pixels_any_channel_delta_gt_0']:.5f} | {m['max_abs_delta']} |",
        "",
        "Images: " + ", ".join(f"`{os.path.basename(p)}`" for p in images)
        + " (left = reference, right = native; diff amplified x4).",
    ]
    with open(os.path.join(args.out, "ab-report.md"), "w") as f:
        f.write("\n".join(lines) + "\n")
    print(json.dumps(record["metrics"], indent=2))
    print(f"wrote ab-report.json + ab-report.md + images in {args.out}")
    return 0


def run_film_mode(args) -> int:
    with open(args.film) as f:
        film = json.load(f)
    fps = args.fps if args.fps else float(film.get("fps", 30))
    os.makedirs(args.out, exist_ok=True)

    native = discover_native(args.native_dir, fps)
    review = discover_review(args.review_dir)
    if not review:
        print(f"ERROR: no t<seconds> review frames found in {args.review_dir}", file=sys.stderr)
        return 2

    pairs: list[dict] = []
    missing: list[dict] = []
    per_segment: dict[str, list[dict]] = {}

    # Pass 1: metrics only. Pixel arrays are dropped after each pair —
    # a 1080x1920 float64 pair is ~100 MB, and holding a whole film's
    # worth in memory gets the process OOM-killed on small machines.
    for seg in film["segments"]:
        sid = seg["id"]
        per_segment[sid] = []
        for t in sorted(review):
            if not (seg["startS"] <= t < seg["endS"]):
                continue
            idx = int(round(t * fps))
            nat_path = native.get(idx)
            if nat_path is None:
                missing.append({"segment": sid, "t": t, "frame": idx,
                                "reason": f"no native frame {idx} in {args.native_dir}"})
                continue
            m, _ri, _ni, _ra, _na, resize_note = measure_pair(review[t], nat_path)
            rec = {
                "segment": sid, "t": t, "frame": idx,
                "reference_path": review[t], "native_path": nat_path,
                "metrics": m, "resize_note": resize_note,
            }
            pairs.append(rec)
            per_segment[sid].append(rec)

    # Pass 2: reload only the three worst pairs to build their images.
    worst = sorted(pairs, key=lambda r: r["metrics"]["psnr_db"])[:3]
    worst_tags: dict[int, str] = {}
    for rank, rec in enumerate(worst, 1):
        tag = f"worst-{rank}_{rec['segment']}_t{rec['t']:.2f}"
        _m, ref_img, nat_img, ref, nat, _rn = measure_pair(
            rec["reference_path"], rec["native_path"])
        write_pair_images(args.out, tag, ref_img, nat_img, ref, nat,
                          args.reference_label, args.native_label)
        worst_tags[rec["frame"]] = tag

    def pub(rec: dict) -> dict:
        m = rec["metrics"]
        return {
            "segment": rec["segment"], "t": rec["t"], "frame": rec["frame"],
            "reference": os.path.basename(rec["reference_path"]),
            "native": os.path.basename(rec["native_path"]),
            "psnr_db": json_psnr(m["psnr_db"]),
            "mean_abs_delta_per_channel": round(m["mean_abs_delta_per_channel"], 5),
            "pct_pixels_max_channel_delta_gt_10": round(m["pct_pixels_max_channel_delta_gt_10"], 5),
            "pct_pixels_any_channel_delta_gt_0": round(m["pct_pixels_any_channel_delta_gt_0"], 5),
            "max_abs_delta": m["max_abs_delta"],
            "resize_note": rec["resize_note"],
            "worst_tag": worst_tags.get(rec["frame"]),
        }

    seg_summary = []
    for seg in film["segments"]:
        sid = seg["id"]
        recs = per_segment[sid]
        if recs:
            psnrs = [r["metrics"]["psnr_db"] for r in recs]
            seg_summary.append({
                "segment": sid, "set": seg.get("set"), "atmosphere": seg.get("atmosphere"),
                "pairs": len(recs),
                "psnr_min_db": json_psnr(min(psnrs)),
                "psnr_mean_db": round(sum(psnrs) / len(psnrs), 3) if all(math.isfinite(p) for p in psnrs) else "inf",
                "mean_abs_delta_mean": round(sum(r["metrics"]["mean_abs_delta_per_channel"] for r in recs) / len(recs), 5),
                "pct_px_gt10_mean": round(sum(r["metrics"]["pct_pixels_max_channel_delta_gt_10"] for r in recs) / len(recs), 5),
            })
        else:
            seg_summary.append({
                "segment": sid, "set": seg.get("set"), "atmosphere": seg.get("atmosphere"),
                "pairs": 0, "status": "NO DATA — no matched native/review pair in this segment",
            })

    all_psnr = [r["metrics"]["psnr_db"] for r in pairs]
    record = {
        "mode": "film",
        "film": film.get("title"),
        "film_manifest": args.film,
        "fps": fps,
        "reference": {"dir": args.review_dir, "label": args.reference_label,
                      "frames_found": len(review)},
        "native": {"dir": args.native_dir, "label": args.native_label,
                   "frames_found": len(native)},
        "jpeg_floor_note": JPEG_FLOOR_NOTE,
        "pairs_measured": len(pairs),
        "pairs_missing": missing,
        "overall": {
            "psnr_min_db": json_psnr(min(all_psnr)) if all_psnr else None,
            "psnr_mean_db": (round(sum(all_psnr) / len(all_psnr), 3)
                             if all_psnr and all(math.isfinite(p) for p in all_psnr)
                             else ("inf" if all_psnr else None)),
        },
        "per_segment": seg_summary,
        "pairs": [pub(r) for r in sorted(pairs, key=lambda r: (r["t"]))],
    }
    with open(os.path.join(args.out, "ab-report.json"), "w") as f:
        json.dump(record, f, indent=2)

    lines = report_header_md(f"A/B — {film.get('title')} (native vs Chrome review frames)",
                             args.reference_label, args.native_label)
    lines += [
        f"- film manifest: `{args.film}` ({fps:g} fps, segments own frames by staged time t = n/fps)",
        f"- review frames found: {len(review)}; native frames found: {len(native)}; pairs measured: {len(pairs)}",
        "",
        "## Per-shot summary",
        "",
        "| segment | set | atmosphere | pairs | PSNR min (dB) | PSNR mean (dB) | mean \\|d\\| | %px > 10 (mean) |",
        "|---|---|---|---|---|---|---|---|",
    ]
    for s in seg_summary:
        if s["pairs"]:
            lines.append(
                f"| {s['segment']} | {s['set']} | {s['atmosphere']} | {s['pairs']} | "
                f"{s['psnr_min_db']} | {s['psnr_mean_db']} | {s['mean_abs_delta_mean']} | {s['pct_px_gt10_mean']} |")
        else:
            lines.append(f"| {s['segment']} | {s['set']} | {s['atmosphere']} | 0 | — | — | — | — |")
    lines += ["", "## Per-pair measurements", "",
              "| segment | t (s) | frame | PSNR (dB) | mean \\|d\\| | %px > 10 | %px > 0 | max d |",
              "|---|---|---|---|---|---|---|---|"]
    for r in record["pairs"]:
        lines.append(
            f"| {r['segment']} | {r['t']:.2f} | {r['frame']} | {r['psnr_db']} | "
            f"{r['mean_abs_delta_per_channel']} | {r['pct_pixels_max_channel_delta_gt_10']} | "
            f"{r['pct_pixels_any_channel_delta_gt_0']} | {r['max_abs_delta']} |")
    nodata = [s for s in seg_summary if not s["pairs"]]
    if nodata or missing:
        lines += ["", "## Missing / no-data (reported, not hidden)", ""]
        for s in nodata:
            lines.append(f"- segment {s['segment']}: {s['status']}")
        for mm in missing:
            lines.append(f"- t={mm['t']:.2f} (segment {mm['segment']}, frame {mm['frame']}): {mm['reason']}")
    if worst:
        lines += ["", "## Three lowest-PSNR pairs (images)", ""]
        for rank, rec in enumerate(worst, 1):
            tag = worst_tags[rec["frame"]]
            lines.append(
                f"- {tag}: PSNR {fmt_psnr(rec['metrics']['psnr_db'])} dB — "
                f"`{tag}_side-by-side.png` (left = reference/Chrome, right = native/talerender), "
                f"`{tag}_diff.png` (|delta| x{DIFF_AMP})")
    resized = [r for r in record["pairs"] if r["resize_note"]]
    if resized:
        lines += ["", "## Resolution notes", ""]
        for r in resized:
            lines.append(f"- frame {r['frame']} (t={r['t']:.2f}): {r['resize_note']}")
    with open(os.path.join(args.out, "ab-report.md"), "w") as f:
        f.write("\n".join(lines) + "\n")

    print(f"pairs measured: {len(pairs)}; missing: {len(missing)}; "
          f"overall PSNR min {record['overall']['psnr_min_db']} / mean {record['overall']['psnr_mean_db']}")
    for s in seg_summary:
        print(f"  {s['segment']}: pairs={s['pairs']}" + (f" psnr_mean={s['psnr_mean_db']}" if s["pairs"] else " NO DATA"))
    print(f"wrote ab-report.json + ab-report.md" + (f" + worst-3 images" if worst else "") + f" in {args.out}")
    if not pairs:
        print("ERROR: no pairs could be measured — see report for the missing list", file=sys.stderr)
        return 2
    return 0


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--pair", nargs=2, metavar=("REFERENCE", "NATIVE"),
                    help="pair mode: measure one reference/native image pair")
    ap.add_argument("--native-dir", help="film mode: dir of native PNG frames")
    ap.add_argument("--review-dir", help="film mode: dir of t<seconds>.jpg review frames")
    ap.add_argument("--film", default="scenes/film/film.json",
                    help="film mode: film manifest (default scenes/film/film.json)")
    ap.add_argument("--fps", type=float, default=None,
                    help="override fps (default: from the film manifest)")
    ap.add_argument("--out", required=True, help="output directory for the report + images")
    ap.add_argument("--reference-label", default=DEFAULT_REFERENCE_LABEL)
    ap.add_argument("--native-label", default=DEFAULT_NATIVE_LABEL)
    args = ap.parse_args()

    if args.pair:
        return run_pair_mode(args)
    if args.native_dir and args.review_dir:
        return run_film_mode(args)
    ap.error("give either --pair REFERENCE NATIVE or --native-dir + --review-dir")
    return 2


if __name__ == "__main__":
    sys.exit(main())
