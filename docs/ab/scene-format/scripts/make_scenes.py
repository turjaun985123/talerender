#!/usr/bin/env python3
"""Generate scenes/ab1.scene.json and scenes/dof-chart.scene.json.

The two hand-authored scenes re-expressed in the v1 scene format
(docs/scene-format.md). This script mirrors the Rust scene builders
(src/ab1.rs build_scene/add_tree/add_palm, src/dof.rs panel spec)
formula for formula:

- values the Rust code computes in f32 are computed here through an
  f32 rounding helper at the same points;
- values it computes in f64 (the sin-hash laws) are computed in f64 —
  Python's math.sin calls the same libm as Rust's f64::sin on this
  platform, so the baked numbers agree bit for bit in practice;
- everything the loader can evaluate itself at load time (shadeHash
  material colours, the grass-tuft scatter) is NOT baked — it is
  written as format data and evaluated by the loader's Rust code,
  exactly like the hand-authored path evaluates it.

Run from the repo root:  python3 docs/ab/scene-format/scripts/make_scenes.py
Writes scenes/ab1.scene.json and scenes/dof-chart.scene.json.
"""
import json
import math
import struct


def f32(x):
    return struct.unpack("f", struct.pack("f", float(x)))[0]


def rnd(i):
    """The engine's sin-hash (lib.ts/world.ts): frac(sin(i*127.1+311.7)*43758.5453), f64."""
    x = math.sin(i * 127.1 + 311.7) * 43758.5453
    return x - math.floor(x)


TAU_F32 = f32(2.0 * math.pi)          # std::f32::consts::TAU
FRAC_PI_2 = f32(math.pi / 2.0)        # std::f32::consts::FRAC_PI_2

# ---------------------------------------------------------------------
# Node shorthands
# ---------------------------------------------------------------------

def mesh(geo, color=None, tex=None, transform=None, name=None):
    n = {"type": "mesh", "geometry": geo}
    if name:
        n["name"] = name
    if tex is not None:
        n["texture"] = tex
    else:
        n["material"] = {"color": color} if isinstance(color, str) else color
    if transform:
        n["transform"] = transform
    return n


def group(children, transform=None, name=None):
    n = {"type": "group", "children": children}
    if name:
        n["name"] = name
    if transform:
        n["transform"] = transform
    return n


def T(x, y, z):
    return {"translate": [x, y, z]}


def shade_hash(hex_, index, base, span):
    return {"color": {"hex": hex_, "shadeHash": {"index": index, "base": base, "span": span}}}


# ---------------------------------------------------------------------
# Trees / palms (src/ab1.rs add_tree / add_palm)
# ---------------------------------------------------------------------

BLOBS = [
    (0.0, 2.3, 0.0, 1.0),
    (0.6, 2.0, 0.2, 0.75),
    (-0.6, 2.0, -0.1, 0.8),
    (0.1, 2.9, -0.2, 0.75),
    (0.2, 2.2, 0.6, 0.7),
]


def tree_children(s, colour, round_):
    kids = [
        mesh(
            {"kind": "cylinder", "radiusTop": f32(0.12 * s), "radiusBottom": f32(0.18 * s),
             "height": f32(1.6 * s), "segments": 7},
            "#7A5234",
            transform=[T(0.0, f32(0.8 * s), 0.0)],
            name="trunk",
        )
    ]
    if round_:
        for bi, (ox, oy, oz, f) in enumerate(BLOBS):
            kids.append(mesh(
                {"kind": "sphere", "radius": f32(f * s), "widthSegments": 12, "heightSegments": 9},
                shade_hash(colour, bi, 0.9, 0.2),
                transform=[T(f32(ox * s), f32(oy * s), f32(oz * s))],
                name=f"canopy blob {bi}",
            ))
    else:
        kids.append(mesh(
            {"kind": "sphere", "radius": 1.0, "widthSegments": 16, "heightSegments": 12},
            colour,
            transform=[T(0.0, f32(3.2 * s), 0.0),
                       {"scale": [f32(0.7 * s), f32(2.6 * s), f32(0.7 * s)]}],
            name="cypress canopy",
        ))
    return kids


def palm(pos, s, lean, rot):
    kids = [
        mesh(
            {"kind": "cylinder", "radiusTop": 0.085, "radiusBottom": 0.13, "height": 4.4, "segments": 8},
            "#7A6248",
            transform=[T(0.0, 2.2, 0.0)],
            name="trunk",
        )
    ]
    top_y = 4.35
    for i in range(7):
        a = f32(f32(f32(i / 7.0) * TAU_F32) + f32(0.4))
        sa, ca = f32(math.sin(a)), f32(math.cos(a))
        col = "#3F7D33" if i % 2 == 0 else "#4C8A3A"
        kids.append(mesh(
            {"kind": "sphere", "radius": 1.0, "widthSegments": 10, "heightSegments": 6},
            col,
            transform=[
                T(sa, f32(f32(top_y + 0.1) - f32(f32(i % 3) * f32(0.07))), ca),
                {"rotate_y": a},
                {"scale": [0.17, 0.05, 1.2]},
            ],
            name=f"frond {i}",
        ))
        kids.append(mesh(
            {"kind": "sphere", "radius": 1.0, "widthSegments": 10, "heightSegments": 6},
            col,
            transform=[
                T(f32(sa * 1.85), f32(f32(top_y - 0.28) - f32(f32(i % 3) * f32(0.06))), f32(ca * 1.85)),
                {"euler_xyz": [f32(ca * 0.5), a, f32(-sa * 0.5)]},
                {"scale": [0.13, 0.04, 0.8]},
            ],
            name=f"frond tip {i}",
        ))
    for nx, nz in [(0.14, 0.0), (-0.12, 0.1), (0.0, -0.14)]:
        kids.append(mesh(
            {"kind": "sphere", "radius": 0.13, "widthSegments": 10, "heightSegments": 8},
            "#6B4A2E",
            transform=[T(nx, f32(top_y - 0.2), nz)],
            name="coconut",
        ))
    return group(
        kids,
        transform=[T(*pos), {"rotate_y": rot}, {"rotate_z": lean}, {"scale": [s, s, s]}],
        name="palm",
    )


# ---------------------------------------------------------------------
# ab1 — Onoda S01 @ t=1.50
# ---------------------------------------------------------------------

def ab1_scene():
    nodes = []
    # Sky + grass ground (textured, drawn first).
    nodes.append(mesh(
        {"kind": "sky-sphere", "radius": 380.0, "widthSegments": 32, "heightSegments": 16},
        tex="sky", name="sky dome",
    ))
    nodes.append(mesh(
        {"kind": "ground-disc", "radius": 80.0, "segments": 48, "uvRepeat": 48.0},
        tex={"id": "grass", "wrap": "repeat"}, name="grass ground",
    ))
    # Dirt patch.
    nodes.append(mesh(
        {"kind": "disc", "radius": 4.8, "segments": 28},
        "#86714C", transform=[T(0.0, 0.012, -0.9)], name="dirt patch",
    ))
    # Hills with their small trees.
    hills = [(-40.0, -75.0, 34.0, 12.0), (10.0, -85.0, 46.0, 16.0),
             (60.0, -71.0, 36.0, 11.0), (-90.0, -65.0, 40.0, 10.0)]
    for i, (hx, hz, r, h) in enumerate(hills):
        nodes.append(mesh(
            {"kind": "sphere", "radius": 1.0, "widthSegments": 24, "heightSegments": 16},
            shade_hash("#5E8F52", i, 0.92, 0.16),
            transform=[T(hx, f32(-h * 0.25), hz), {"scale": [r, h, f32(r * 0.6)]}],
            name=f"hill {i}",
        ))
        for k in range(6):
            a = rnd(i * 9.0 + k) * 2.0 - 1.0
            s = f32(f32(0.5 + rnd(float(k)) * 0.4) * f32(2.2))
            pos = [
                f32(hx + f32(f32(f32(a) * r) * f32(0.6))),
                f32(h * (0.75 - abs(a) * 0.45) - 1.0),
                f32(hz + f32(r * f32(0.25))),
            ]
            nodes.append(group(tree_children(s, "#3E7A34", True),
                               transform=[T(*pos)], name=f"hill {i} tree {k}"))
    # Tree ring (19 trees).
    ring = [
        (-6.5, -6.0, 1.5, True), (-9.0, -2.0, 1.3, False), (-7.5, -11.0, 1.7, True),
        (-12.0, -7.0, 1.5, True), (-13.0, -14.0, 1.8, False), (6.8, -6.5, 1.6, True),
        (9.5, -2.5, 1.35, False), (8.0, -12.0, 1.75, True), (12.5, -8.0, 1.5, True),
        (13.5, -15.0, 1.9, False), (-4.5, -13.0, 1.6, True), (4.6, -14.0, 1.55, True),
        (0.0, -17.0, 1.9, True), (-16.0, -4.0, 1.6, True), (16.5, -5.0, 1.7, True),
        (-5.5, 4.5, 1.3, True), (6.2, 5.0, 1.25, False), (-11.0, 3.0, 1.5, True),
        (11.5, 3.5, 1.45, True),
    ]
    for i, (x, z, s, round_) in enumerate(ring):
        col = "#3E7A34" if i % 3 != 0 else "#356B2C"
        nodes.append(group(
            tree_children(s, col, round_),
            transform=[T(x, 0.0, z), {"rotate_y": f32(f32(i) * f32(1.7))}],
            name=f"ring tree {i}",
        ))
    # Palms.
    nodes.append(palm([4.6, 0.0, -4.6], 1.05, 0.14, 0.0))
    nodes.append(palm([-5.2, 0.0, -5.4], 0.9, -0.2, 1.2))
    nodes.append(palm([10.5, 0.0, -11.0], 1.15, 0.1, 0.0))
    # Grass tufts: the scatter node — evaluated by the loader under
    # the same sin-hash law the hand-authored code runs.
    nodes.append({
        "type": "scatter", "name": "grass tufts",
        "geometry": {"kind": "cone", "radius": 0.05, "height": 0.34, "segments": 4},
        "material": {"color": "#4C8A3E"},
        "count": 900,
        "placement": {"kind": "annulus", "angleHash": 1.31, "radiusHash": 2.7,
                      "radiusBase": 2.6, "radiusSpan": 20.0,
                      "angleScale": 6.283185307, "center": [0.0, 0.15, -3.0]},
    })
    # Camp (fire pit at (-1.35, -2.45)).
    fx, fz = -1.35, -2.45
    for i in range(8):
        a = f32(f32(i / 8.0) * TAU_F32)
        r_i = f32(0.09 + rnd(float(i)) * 0.04)
        nodes.append(mesh(
            {"kind": "sphere", "radius": r_i, "widthSegments": 8, "heightSegments": 6},
            "#8B8B84",
            transform=[T(f32(fx + f32(f32(math.cos(a)) * f32(0.44))), 0.05,
                         f32(fz + f32(f32(math.sin(a)) * f32(0.44))))],
            name=f"fire stone {i}",
        ))
    for i, (dx, dz) in enumerate([(0.25, 0.3), (-0.2, -0.28), (0.05, 0.02)]):
        nodes.append(mesh(
            {"kind": "cylinder", "radiusTop": 0.05, "radiusBottom": 0.05, "height": 0.72, "segments": 7},
            "#7A5A38",
            transform=[T(f32(fx + dx), 0.09, f32(fz + dz)),
                       {"euler_xyz": [0.0, f32(f32(i) * f32(0.7)), FRAC_PI_2]}],
            name=f"fire log {i}",
        ))
    nodes.append(mesh(
        {"kind": "cone", "radius": 0.17, "height": 0.52, "segments": 8},
        "#FF8A30",
        transform=[T(fx, 0.34, fz), {"scale": [1.0, 1.13146, 1.0]}], name="flame",
    ))
    nodes.append(mesh(
        {"kind": "cone", "radius": 0.09, "height": 0.34, "segments": 8},
        "#FFD23C",
        transform=[T(f32(fx + 0.03), 0.30, f32(fz + 0.04)), {"scale": [1.0, 0.86854, 1.0]}],
        name="flame inner",
    ))
    nodes.append(mesh(
        {"kind": "cylinder", "radiusTop": 0.17, "radiusBottom": 0.17, "height": 1.15, "segments": 9},
        "#7A5A38",
        transform=[T(-0.5, 0.17, -1.8), {"euler_xyz": [0.0, 0.12, FRAC_PI_2]}],
        name="seat log",
    ))
    for x in (-3.9, -2.7):
        nodes.append(mesh(
            {"kind": "cylinder", "radiusTop": 0.035, "radiusBottom": 0.035, "height": 1.5, "segments": 6},
            "#7A5A38", transform=[T(x, 0.75, -2.7)], name="lean-to pole",
        ))
    nodes.append(mesh(
        {"kind": "box", "size": [1.9, 0.05, 1.5]},
        "#8A6B42",
        transform=[T(-3.3, 1.52, -2.95), {"rotate_x": 0.42}], name="lean-to roof",
    ))
    nodes.append(mesh(
        {"kind": "box", "size": [0.55, 0.55, 0.55]},
        "#B07A48", transform=[T(-2.35, 0.275, -3.15)], name="crate",
    ))
    nodes.append(mesh(
        {"kind": "box", "size": [0.42, 0.42, 0.42]},
        "#B07A48",
        transform=[T(-2.3, f32(f32(0.55) + f32(0.21)), -3.12), {"rotate_y": 0.4}],
        name="crate (stacked)",
    ))
    nodes.append(mesh(
        {"kind": "cylinder", "radiusTop": 0.33, "radiusBottom": 0.33, "height": 0.8, "segments": 16},
        "#6E4525", transform=[T(-3.05, 0.4, -2.2)], name="barrel",
    ))
    for y in (0.12, 0.68):
        nodes.append(mesh(
            {"kind": "cylinder", "radiusTop": 0.345, "radiusBottom": 0.345, "height": 0.05, "segments": 16},
            "#3A3A3F", transform=[T(-3.05, y, -2.2)], name="barrel band",
        ))
    nodes.append(mesh(
        {"kind": "box", "size": [1.5, 0.09, 0.55]},
        "#77805E",
        transform=[T(-3.3, 0.05, -2.1), {"rotate_y": 0.2}], name="bedroll",
    ))
    # The rifle is code-ref: its anchor comes from the painter's hand
    # position (M4). Drawn here, in the colour phase, as in the code.
    nodes.append({"type": "code-ref", "id": "ab1-onoda-rifle",
                  "name": "rifle (attach3D, code until M4)"})
    # Blob shadow (a plain textured quad — fully data).
    h_t = f32(f32(1.75) / f32(0.96))
    nodes.append(mesh(
        {"kind": "flat-quad", "width": f32(f32(h_t / 2.0) * f32(0.9)),
         "depth": f32(f32(h_t / 2.0) * f32(0.34))},
        tex="blob", transform=[T(0.35, 0.02, -0.55)], name="blob shadow",
    ))
    # Clouds (billboards; basis derived from the camera by the loader).
    for cx, cy, cz, s in [(-28.0, 23.0, -85.0, 13.0), (26.0, 29.0, -95.0, 16.0),
                          (58.0, 19.0, -75.0, 10.0)]:
        nodes.append(mesh(
            {"kind": "billboard", "center": [cx, cy, cz], "width": 2.0 * s, "height": s},
            tex="cloud", name="cloud sprite",
        ))
    # The character card is code-ref (the painter is M4).
    nodes.append({"type": "code-ref", "id": "ab1-onoda-card",
                  "name": "Onoda card (painter, code until M4)"})
    # Caption: a billboard 2 m ahead of the camera, centred on the
    # caption's NDC position (0, -0.395833), full width, 300/1920 of
    # the frame height (the code builds its corners from the camera
    # basis; the centre/width/height here are that construction
    # evaluated once, in f64).
    eye = (f32(0.83432), f32(2.64354), f32(5.28499))
    tgt = (f32(0.35), f32(1.11477), f32(-0.55))
    fwd = [tgt[i] - eye[i] for i in range(3)]
    fl = math.sqrt(sum(c * c for c in fwd))
    fwd = [c / fl for c in fwd]
    # right = norm(cross(fwd, up=(0,1,0))) = norm((-fwd.z, 0, fwd.x))
    right = [fwd[1] * 0.0 - fwd[2] * 1.0, fwd[2] * 0.0 - fwd[0] * 0.0, fwd[0] * 1.0 - fwd[1] * 0.0]
    rl = math.sqrt(sum(c * c for c in right))
    right = [c / rl for c in right]
    upv = [right[1] * fwd[2] - right[2] * fwd[1],
           right[2] * fwd[0] - right[0] * fwd[2],
           right[0] * fwd[1] - right[1] * fwd[0]]
    d = 2.0
    tan_h = math.tan(math.radians(22.0))
    cy = -0.395833 * tan_h * d
    hw = (1080.0 / 1920.0) * tan_h * d
    hh = (300.0 / 1920.0) * tan_h * d
    centre = [eye[i] + fwd[i] * d + upv[i] * cy for i in range(3)]
    nodes.append(mesh(
        {"kind": "billboard", "center": centre, "width": 2.0 * hw, "height": 2.0 * hh},
        tex="caption", name="caption 'fighting'",
    ))

    return {
        "format": "talerender-scene",
        "version": 1,
        "name": "ab1 — Onoda S01 @ t=1.50 (scene-format re-expression)",
        "camera": {"fovY": 44.0, "near": 0.05, "far": 800.0,
                   "eye": [0.83432, 2.64354, 5.28499], "target": [0.35, 1.11477, -0.55],
                   "up": [0.0, 1.0, 0.0]},
        "clear": "#D8E8DA",
        "lighting": {"directional": {"direction": [0.5, 0.8, 0.6], "ambient": 0.35}},
        "textures": [
            {"id": "sky", "generator": "sky-gradient", "top": "#6E9CC0", "horizon": "#D8E8DA"},
            {"id": "grass", "generator": "grass", "base": "#5B9A48"},
            {"id": "cloud", "generator": "cloud"},
            {"id": "blob", "generator": "blob-shadow"},
            {"id": "onoda-card", "generator": "onoda-card"},
            {"id": "caption", "file": "../docs/ab/onoda-s01-t150/assets/caption.png"},
        ],
        "nodes": nodes,
    }


# ---------------------------------------------------------------------
# dof-chart — the M3 gate chart
# ---------------------------------------------------------------------

PANELS = [
    (-0.62, 0.66, 30.0), (0.0, 0.66, 14.0), (0.62, 0.66, 11.0),
    (-0.62, 0.0, 12.0), (0.0, 0.0, 10.0), (0.62, 0.0, 8.0),
    (-0.62, -0.66, 7.0), (0.0, -0.66, 5.0), (0.62, -0.66, 3.5),
    (0.0, -0.30, 2.0),
]


def dof_scene():
    # src/dof.rs panel_geometry, mirrored in f32: panels are quads
    # whose NDC footprint at their depth is 0.56 x 0.36 (0.30 for the
    # 2 m bleed panel), centred on (nx, ny) in NDC at that depth.
    aspect = f32(1080.0 / 1920.0)
    tan_h = f32(math.tan(f32(math.radians(15.0))))
    nodes = []
    for nx, ny, d in PANELS:
        hh = f32(d * tan_h)
        hw = f32(hh * aspect)
        h_ndc = 0.30 if d == 2.0 else 0.36
        w = f32(f32(0.56) * hw)
        h = f32(f32(h_ndc) * hh)
        nodes.append(mesh(
            {"kind": "quad", "width": w, "height": h, "y0": f32(-h / 2.0)},
            tex="chart",
            transform=[T(f32(nx * hw), f32(ny * hh), -d)],
            name=f"panel {d} m",
        ))
    return {
        "format": "talerender-scene",
        "version": 1,
        "name": "dof-chart — the M3 bokeh gate chart (scene-format re-expression)",
        "camera": {"fovY": 30.0, "near": 0.05, "far": 800.0,
                   "eye": [0.0, 0.0, 0.0], "target": [0.0, 0.0, -1.0], "up": [0.0, 1.0, 0.0]},
        "textures": [
            {"id": "chart", "generator": "dof-chart-pattern", "size": 512},
        ],
        "post": {"bokeh": {"focus": 10.0, "aperture": 0.0015, "maxblur": 0.006}},
        "nodes": nodes,
    }


def main():
    import os
    os.makedirs("scenes", exist_ok=True)
    with open("scenes/ab1.scene.json", "w") as f:
        json.dump(ab1_scene(), f, indent=1)
        f.write("\n")
    with open("scenes/dof-chart.scene.json", "w") as f:
        json.dump(dof_scene(), f, indent=1)
        f.write("\n")
    print("wrote scenes/ab1.scene.json + scenes/dof-chart.scene.json")


if __name__ == "__main__":
    main()
