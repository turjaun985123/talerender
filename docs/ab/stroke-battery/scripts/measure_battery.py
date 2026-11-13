#!/usr/bin/env python3
"""M4 stroke battery measurement (compatibility contract SS2).

Compares the Chrome/Skia reference cards against the native CPU
painter cards, per primitive class. Silhouette classes are measured
on the ALPHA channel (coverage). Interior face features — ink
painted inside the opaque head fill, where alpha is 1.0 everywhere —
are measured on ink darkness (composited-over-white luminance),
which is the coverage signal for those strokes.

TOLERANCES — stated before any measurement was taken (see the package
README, committed with this script before the first run):

  width / edge position at half coverage: |native - ref| <= 0.5
      device px (res-2 px = 0.25 stage units). The character plane is
      minified roughly 2-4x on screen, so 0.5 device px is ~0.15
      screen px — below visibility; a systematic 1 device px error
      would start to read as a different line weight.
  AA ramp width (10% -> 90% coverage): |native - ref| <= 0.5 device px.
  Coverage RMSE across the edge profile band: <= 0.10.
  Crop classes (joins, crossings, faces): RMSE within the edge band
      (either side's alpha strictly fractional, dilated 2 px) <= 0.12,
      and <= 5% of band pixels may disagree by more than 0.25.

A binary (point-sampled) rasteriser is expected to FAIL these: its
band RMSE is ~0.25-0.3 and its ramp is a sub-pixel step. That failure
is the baseline this battery exists to quantify.

Usage: measure_battery.py --ref <dir> --native <dir> --spec <spec.json>
                          [--out <metrics.json>]
Cards: prims.png, toon.png, face-<mood>.png in both dirs.
"""
import argparse
import json
import math

import numpy as np
from PIL import Image

TOL = {
    "width_abs_px": 0.5,
    "edge_abs_px": 0.5,
    "ramp_abs_px": 0.5,
    "profile_rmse": 0.10,
    "crop_band_rmse": 0.12,
    "crop_disagree_frac": 0.05,
}

STEP = 0.1  # profile sampling step, device px


def load_alpha(path):
    return np.asarray(Image.open(path).convert("RGBA"), dtype=np.float64)[:, :, 3] / 255.0


def load_grey(path):
    """RGBA composited over white, as luminance 0..1.

    Interior face features (brows, eyes, mouth) are ink painted
    INSIDE the opaque head fill: their alpha is 1.0 everywhere, so
    alpha cannot see them. On the composited grey, ink coverage is
    the darkness signal — the measurable surface for those classes.
    """
    rgba = np.asarray(Image.open(path).convert("RGBA"), dtype=np.float64) / 255.0
    a = rgba[:, :, 3:4]
    rgb = rgba[:, :, :3] * a + (1.0 - a)
    return 0.299 * rgb[:, :, 0] + 0.587 * rgb[:, :, 1] + 0.114 * rgb[:, :, 2]


def grey_crop_metrics(g_ref, g_nat, a_ref, a_nat, box):
    """Band = fractional alpha OR a strong grey gradient (ink edge
    inside an opaque region), dilated 2 px. Metrics on grey diff."""
    x0, y0, x1, y1 = box
    r = g_ref[y0:y1, x0:x1]
    n = g_nat[y0:y1, x0:x1]
    ar = a_ref[y0:y1, x0:x1]
    an = a_nat[y0:y1, x0:x1]

    def grad(g):
        gx = np.abs(np.diff(g, axis=1, prepend=g[:, :1]))
        gy = np.abs(np.diff(g, axis=0, prepend=g[:1, :]))
        return np.maximum(gx, gy)

    band = dilate(
        ((ar > 0.002) & (ar < 0.998)) | ((an > 0.002) & (an < 0.998)) | (grad(r) > 0.25) | (grad(n) > 0.25),
        2,
    )
    diff = np.abs(r - n)
    return {
        "band_px": int(band.sum()),
        "band_rmse": round(float(np.sqrt(np.mean(diff[band] ** 2))), 4) if band.sum() else 0.0,
        "disagree_frac": round(float((diff[band] > 0.25).mean()), 4) if band.sum() else 0.0,
        "crop_rmse": round(float(np.sqrt(np.mean(diff**2))), 4),
    }


def bilinear(a, x, y):
    """Sample alpha image a at float device coords (x, y)."""
    h, w = a.shape
    x = min(max(x, 0.0), w - 1.001)
    y = min(max(y, 0.0), h - 1.001)
    x0, y0 = int(x), int(y)
    fx, fy = x - x0, y - y0
    return (
        a[y0, x0] * (1 - fx) * (1 - fy)
        + a[y0, x0 + 1] * fx * (1 - fy)
        + a[y0 + 1, x0] * (1 - fx) * fy
        + a[y0 + 1, x0 + 1] * fx * fy
    )


def crossings(d, v, level):
    """All positions where profile v crosses `level`, linearly interp."""
    out = []
    for i in range(len(d) - 1):
        if (v[i] - level) * (v[i + 1] - level) < 0 or (v[i] == level and v[i + 1] != level):
            t = (level - v[i]) / (v[i + 1] - v[i]) if v[i + 1] != v[i] else 0.0
            out.append(d[i] + t * (d[i + 1] - d[i]))
    return out


def stroke_metrics(d, v):
    """Width at half coverage + mean 10-90 ramp over both edges."""
    c50 = crossings(d, v, 0.5)
    c10 = crossings(d, v, 0.1)
    c90 = crossings(d, v, 0.9)
    if len(c50) < 2 or len(c10) < 2 or len(c90) < 2:
        return None
    width = c50[-1] - c50[0]
    ramp_l = abs(c90[0] - c10[0])
    ramp_r = abs(c90[-1] - c10[-1])
    return {"width": width, "ramp": (ramp_l + ramp_r) / 2.0}


def profile_rmse(v_ref, v_nat):
    return float(np.sqrt(np.mean((np.asarray(v_ref) - np.asarray(v_nat)) ** 2)))


def seg_profile(a, p0, p1, hw, stations=(0.35, 0.5, 0.65)):
    """Mean coverage profile across a straight segment.

    d = signed distance from the centreline (device px), sampled at
    several stations along the segment, away from the caps.
    """
    (x0, y0), (x1, y1) = p0, p1
    ux, uy = x1 - x0, y1 - y0
    ln = math.hypot(ux, uy)
    ux, uy = ux / ln, uy / ln
    nx, ny = -uy, ux
    band = hw + 4.0
    d = np.arange(-band, band + STEP, STEP)
    prof = np.zeros(len(d))
    for t in stations:
        cx, cy = x0 + ux * ln * t, y0 + uy * ln * t
        for i, dd in enumerate(d):
            prof[i] += bilinear(a, cx + nx * dd, cy + ny * dd)
    return d, prof / len(stations)


def radial_profile(a, c, r_of_theta, hw, angles):
    """Mean coverage profile vs (r - r_edge) along rays from c."""
    band = hw + 4.0
    d = np.arange(-band, band + STEP, STEP)
    prof = np.zeros(len(d))
    for th in angles:
        re = r_of_theta(th)
        dx, dy = math.cos(th), math.sin(th)
        for i, dd in enumerate(d):
            r = re + dd
            prof[i] += bilinear(a, c[0] + dx * r, c[1] + dy * r)
    return d, prof / len(angles)


def edge_profile(a, c, direction, r_edge, band=6.0):
    """Coverage profile across a fill edge, along `direction` from c."""
    d = np.arange(-band, band + STEP, STEP)
    v = np.array([bilinear(a, c[0] + direction[0] * (r_edge + dd), c[1] + direction[1] * (r_edge + dd)) for dd in d])
    return d, v


def fill_metrics(d, v):
    c50 = crossings(d, v, 0.5)
    c10 = crossings(d, v, 0.1)
    c90 = crossings(d, v, 0.9)
    if not c50 or not c10 or not c90:
        return None
    return {"edge": c50[0], "ramp": abs(c90[0] - c10[0])}


def dilate(mask, iters=2):
    m = mask.copy()
    for _ in range(iters):
        n = m.copy()
        n[1:, :] |= m[:-1, :]
        n[:-1, :] |= m[1:, :]
        n[:, 1:] |= m[:, :-1]
        n[:, :-1] |= m[:, 1:]
        m = n
    return m


def crop_metrics(a_ref, a_nat, box):
    """box = (x0, y0, x1, y1) device px. Band = either alpha fractional."""
    x0, y0, x1, y1 = box
    r = a_ref[y0:y1, x0:x1]
    n = a_nat[y0:y1, x0:x1]
    band = dilate(((r > 0.002) & (r < 0.998)) | ((n > 0.002) & (n < 0.998)), 2)
    if band.sum() == 0:
        return {"band_px": 0, "band_rmse": 0.0, "disagree_frac": 0.0}
    diff = np.abs(r - n)
    return {
        "band_px": int(band.sum()),
        "band_rmse": round(float(np.sqrt(np.mean(diff[band] ** 2))), 4),
        "disagree_frac": round(float((diff[band] > 0.25).mean()), 4),
        "crop_rmse": round(float(np.sqrt(np.mean(diff**2))), 4),
    }


def judge_profile(name, kind, ref_m, nat_m, rmse):
    if ref_m is None or nat_m is None:
        return {"kind": kind, "note": "degenerate profile (crossings not found)", "verdict": "FAIL"}
    dw = abs(nat_m["width"] - ref_m["width"]) if "width" in ref_m else abs(nat_m["edge"] - ref_m["edge"])
    dr = abs(nat_m["ramp"] - ref_m["ramp"])
    tol_w = TOL["width_abs_px"] if "width" in ref_m else TOL["edge_abs_px"]
    ok = dw <= tol_w and dr <= TOL["ramp_abs_px"] and rmse <= TOL["profile_rmse"]
    return {
        "kind": kind,
        "ref": {k: round(v, 3) for k, v in ref_m.items()},
        "native": {k: round(v, 3) for k, v in nat_m.items()},
        "delta_width_or_edge_px": round(dw, 3),
        "delta_ramp_px": round(dr, 3),
        "profile_rmse": round(rmse, 4),
        "verdict": "PASS" if ok else "FAIL",
    }


def judge_crop(name, m):
    ok = m["band_rmse"] <= TOL["crop_band_rmse"] and m["disagree_frac"] <= TOL["crop_disagree_frac"]
    return {"kind": "crop", **m, "verdict": "PASS" if ok else "FAIL"}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--ref", required=True)
    ap.add_argument("--native", required=True)
    ap.add_argument("--spec", required=True)
    ap.add_argument("--out")
    args = ap.parse_args()

    spec = json.load(open(args.spec))
    res = spec["canvas"]["res"]
    D = lambda x: x * res  # logical -> device
    prims_ref = load_alpha(f"{args.ref}/prims.png")
    prims_nat = load_alpha(f"{args.native}/prims.png")
    classes = {}

    # ---- straight segments + quads, from the spec rows ----
    rows = [(15, 110), (14, 280), (12, 450), (10, 620), (9, 790)]
    for w, y in rows:
        hw = D(w) / 2.0
        segs = {
            f"seg-h-{w}": ((D(70), D(y)), (D(400), D(y))),
            f"seg-v-{w}": ((D(500), D(y - 60)), (D(500), D(y + 60))),
            f"seg-diag-{w}": ((D(600), D(y + 43)), (D(760), D(y - 43))),
        }
        for name, (p0, p1) in segs.items():
            dr_, vr = seg_profile(prims_ref, p0, p1, hw)
            _, vn = seg_profile(prims_nat, p0, p1, hw)
            classes[name] = judge_profile(name, "segment", stroke_metrics(dr_, vr), stroke_metrics(dr_, vn), profile_rmse(vr, vn))
        # quad apex: C = .25 P0 + .5 Pc + .25 P2, tangent horizontal
        # there -> profile along the vertical through the apex (a
        # horizontal sampling segment gives a vertical profile).
        name = f"quad-{w}"
        apex = (D(1035), D(y - 2.5))
        dr_, vr = seg_profile(prims_ref, (apex[0] - 30, apex[1]), (apex[0] + 30, apex[1]), hw, stations=(0.5,))
        _, vn = seg_profile(prims_nat, (apex[0] - 30, apex[1]), (apex[0] + 30, apex[1]), hw, stations=(0.5,))
        classes[name] = judge_profile(name, "quad", stroke_metrics(dr_, vr), stroke_metrics(dr_, vn), profile_rmse(vr, vn))

    # ---- row 6: ellipses, arcs, fills ----
    def ellipse_r(rx, ry):
        return lambda th: 1.0 / math.sqrt((math.cos(th) / rx) ** 2 + (math.sin(th) / ry) ** 2)

    angs = [0.0, math.pi / 4, math.pi / 2, math.pi, 3 * math.pi / 2]
    dr_, vr = radial_profile(prims_ref, (D(250), D(960)), ellipse_r(D(138), D(133.86)), D(10) / 2, angs)
    _, vn = radial_profile(prims_nat, (D(250), D(960)), ellipse_r(D(138), D(133.86)), D(10) / 2, angs)
    classes["head-outline"] = judge_profile("head-outline", "ellipse-stroke", stroke_metrics(dr_, vr), stroke_metrics(dr_, vn), profile_rmse(vr, vn))

    dr_, vr = radial_profile(prims_ref, (D(560), D(960)), lambda th: D(60), D(9) / 2, angs)
    _, vn = radial_profile(prims_nat, (D(560), D(960)), lambda th: D(60), D(9) / 2, angs)
    classes["circle-stroke"] = judge_profile("circle-stroke", "ellipse-stroke", stroke_metrics(dr_, vr), stroke_metrics(dr_, vn), profile_rmse(vr, vn))

    # arc stroke: ray at 270 deg (top of the arc, mid-sweep)
    dr_, vr = radial_profile(prims_ref, (D(1210), D(960)), lambda th: D(20), D(10) / 2, [3 * math.pi / 2])
    _, vn = radial_profile(prims_nat, (D(1210), D(960)), lambda th: D(20), D(10) / 2, [3 * math.pi / 2])
    classes["arc-stroke"] = judge_profile("arc-stroke", "arc-stroke", stroke_metrics(dr_, vr), stroke_metrics(dr_, vn), profile_rmse(vr, vn))

    for name, c, r_edge in [
        ("foot-fill", (D(760), D(960)), D(20)),
        ("pupil-fill", (D(830), D(960)), D(13)),
        ("bun-fill", (D(1090), D(960)), D(46)),
    ]:
        dr_, vr = edge_profile(prims_ref, c, (1.0, 0.0), r_edge)
        _, vn = edge_profile(prims_nat, c, (1.0, 0.0), r_edge)
        classes[name] = judge_profile(name, "fill", fill_metrics(dr_, vr), fill_metrics(dr_, vn), profile_rmse(vr, vn))

    # eye ring: profile along +x from centre, r in [30, 84] device px
    d = np.arange(D(15), D(42) + STEP, STEP)
    vr = np.array([bilinear(prims_ref, D(960) + rr, D(960)) for rr in d])
    vn = np.array([bilinear(prims_nat, D(960) + rr, D(960)) for rr in d])
    # recentre on the ring's outer edge: (rx 26 + w/2 3) logical = 58 device px
    d2 = d - D(29)
    outer_r = fill_metrics(d2, vr)
    outer_n = fill_metrics(d2, vn)
    if outer_r is None or outer_n is None:
        classes["eye-ring"] = {"kind": "ring-outer-edge", "note": "degenerate profile", "verdict": "FAIL"}
    else:
        ring = {
            "kind": "ring-outer-edge",
            "ref": {k: round(v, 3) for k, v in outer_r.items()},
            "native": {k: round(v, 3) for k, v in outer_n.items()},
            "delta_width_or_edge_px": round(abs(outer_n["edge"] - outer_r["edge"]), 3),
            "delta_ramp_px": round(abs(outer_n["ramp"] - outer_r["ramp"]), 3),
            "profile_rmse": round(profile_rmse(vr, vn), 4),
        }
        ring["verdict"] = "PASS" if ring["delta_width_or_edge_px"] <= TOL["edge_abs_px"] and ring["delta_ramp_px"] <= TOL["ramp_abs_px"] and ring["profile_rmse"] <= TOL["profile_rmse"] else "FAIL"
        classes["eye-ring"] = ring

    # cap end: axial profile through the right cap of the short segment
    d = np.arange(-D(10), D(10) + STEP, STEP)
    vr = np.array([bilinear(prims_ref, D(580) + dd, D(1332)) for dd in d])
    vn = np.array([bilinear(prims_nat, D(580) + dd, D(1332)) for dd in d])
    classes["cap-end"] = judge_profile("cap-end", "cap", fill_metrics(d, vr), fill_metrics(d, vn), profile_rmse(vr, vn))
    # cap segment cross profile too
    dr_, vr = seg_profile(prims_ref, (D(520), D(1332)), (D(580), D(1332)), D(15) / 2, stations=(0.5,))
    _, vn = seg_profile(prims_nat, (D(520), D(1332)), (D(580), D(1332)), D(15) / 2, stations=(0.5,))
    classes["cap-seg"] = judge_profile("cap-seg", "segment", stroke_metrics(dr_, vr), stroke_metrics(dr_, vn), profile_rmse(vr, vn))

    # ---- join / crossing crops (logical boxes) ----
    crops = {
        "join-zigzag": (150, 1050, 310, 1210),
        "join-uturn": (920, 1050, 1090, 1210),
        "cross-x": (120, 1240, 300, 1400),
        "join-spike": (700, 1190, 880, 1360),
    }
    for name, (x0, y0, x1, y1) in crops.items():
        classes[name] = judge_crop(name, crop_metrics(prims_ref, prims_nat, (D(x0), D(y0), D(x1), D(y1))))

    # ---- faces: head crop + brow profile per mood ----
    # Interior features are measured on ink darkness (1 - grey): see
    # load_grey. Brow profiles are normalised by their own plateau
    # (ink darkness ~0.88, over a shade-crescent floor on some moods).
    lifts = {"neutral": 4, "determined": -4, "angry": -10, "laugh": 10, "shock": 24}
    for mood in spec["faces"]["moods"]:
        a_ref = load_alpha(f"{args.ref}/face-{mood}.png")
        a_nat = load_alpha(f"{args.native}/face-{mood}.png")
        g_ref = load_grey(f"{args.ref}/face-{mood}.png")
        g_nat = load_grey(f"{args.native}/face-{mood}.png")
        classes[f"face-{mood}"] = judge_crop(f"face-{mood}", grey_crop_metrics(g_ref, g_nat, a_ref, a_nat, (D(470), D(260), D(830), D(600))))
        # brow: vertical profile through the right brow apex.
        # stage: brow base y = (182 - 44 - lift), apex dips 3.5 lower;
        # canvas device x = (384 + 256 + 46) * res, y likewise (+256).
        brow_cx = (384 + 302) * res
        brow_cy = (256 + (182 - 44 - lifts[mood]) - 3.5) * res
        d = np.arange(-26, 26 + STEP, STEP)
        vr = np.array([1.0 - bilinear(g_ref, brow_cx, brow_cy + dd) for dd in d])
        vn = np.array([1.0 - bilinear(g_nat, brow_cx, brow_cy + dd) for dd in d])
        vr = vr / max(vr.max(), 1e-9)
        vn = vn / max(vn.max(), 1e-9)
        classes[f"brow-{mood}"] = judge_profile(f"brow-{mood}", "brow", stroke_metrics(d, vr), stroke_metrics(d, vn), profile_rmse(vr, vn))

    # ---- toon card: global + in-situ leg width ----
    t_ref = load_alpha(f"{args.ref}/toon.png")
    t_nat = load_alpha(f"{args.native}/toon.png")
    tg_ref = load_grey(f"{args.ref}/toon.png")
    tg_nat = load_grey(f"{args.native}/toon.png")
    diff = t_ref - t_nat
    mse = float(np.mean(diff**2))
    band = dilate(((t_ref > 0.002) & (t_ref < 0.998)) | ((t_nat > 0.002) & (t_nat < 0.998)), 2)
    gm = grey_crop_metrics(tg_ref, tg_nat, t_ref, t_nat, (0, 0, t_ref.shape[1], t_ref.shape[0]))
    toon = {
        "kind": "full-character",
        "alpha_psnr_db": round(float(10 * np.log10(1.0 / mse)), 3) if mse > 0 else float("inf"),
        "alpha_rmse": round(float(np.sqrt(mse)), 4),
        "band_px": int(band.sum()),
        "band_rmse": round(float(np.sqrt(np.mean(diff[band] ** 2))), 4) if band.sum() else 0.0,
        "disagree_frac": round(float((np.abs(diff[band]) > 0.25).mean()), 4) if band.sum() else 0.0,
        "grey_band_rmse": gm["band_rmse"],
        "grey_disagree_frac": gm["disagree_frac"],
    }
    toon["verdict"] = "PASS" if toon["band_rmse"] <= TOL["crop_band_rmse"] and toon["disagree_frac"] <= TOL["crop_disagree_frac"] and gm["band_rmse"] <= TOL["crop_band_rmse"] and gm["disagree_frac"] <= TOL["crop_disagree_frac"] else "FAIL"
    classes["toon-card"] = toon
    # in-situ head outline: the toon head is lean-rotated (1 deg) and
    # camPitch-scaled (rp = 138 * 1.0115 stage units), so its edges
    # sit at fractional device coords — the honest curved-stroke
    # case. Centre: stage (256,178) lean-rotated about (256,1000)
    # -> canvas logical (654.35, 434.13) -> device (1308.7, 868.3).
    # The 1-degree ellipse rotation is ignored (cos error ~0.02%).
    head_c = (1308.7, 868.3)
    rp_dev = 138 * (1 + 0.04 * 0.28747) * res
    dr_, vr = radial_profile(t_ref, head_c, ellipse_r(rp_dev, rp_dev * 0.97), D(10) / 2, angs)
    _, vn = radial_profile(t_nat, head_c, ellipse_r(rp_dev, rp_dev * 0.97), D(10) / 2, angs)
    classes["toon-head-outline"] = judge_profile("toon-head-outline", "ellipse-stroke-in-situ", stroke_metrics(dr_, vr), stroke_metrics(dr_, vn), profile_rmse(vr, vn))
    # leg: horizontal profile at stage y=787 across the left leg
    xs = np.arange(D(560), D(680) + 0.5, 0.5)
    vr = np.array([bilinear(t_ref, xx, D(1043)) for xx in xs])
    vn = np.array([bilinear(t_nat, xx, D(1043)) for xx in xs])
    classes["toon-leg"] = judge_profile("toon-leg", "segment-in-situ", stroke_metrics(xs, vr), stroke_metrics(xs, vn), profile_rmse(vr, vn))

    fails = [k for k, v in classes.items() if v["verdict"] == "FAIL"]
    report = {
        "tolerances": TOL,
        "renderer_ref": "Chrome headless (SwiftShader) Canvas2D/Skia — see provenance.json beside the reference cards",
        "renderer_native": "TaleRender CPU painter (src/scene.rs)",
        "classes": classes,
        "summary": {"total": len(classes), "pass": len(classes) - len(fails), "fail": len(fails), "failing": fails},
    }
    print(json.dumps(report["summary"], indent=2))
    for k, v in classes.items():
        print(f"  {k:16s} {v['verdict']:4s} {json.dumps({kk: vv for kk, vv in v.items() if kk not in ('kind', 'verdict')})}")
    if args.out:
        with open(args.out, "w") as f:
            json.dump(report, f, indent=2)
        print("wrote", args.out)


if __name__ == "__main__":
    main()
