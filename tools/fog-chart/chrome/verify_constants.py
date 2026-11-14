#!/usr/bin/env python3
"""Static verification of the fog-chart staging, three ways:

  A. the native scene file (scenes/fog-chart.scene.json) against the shared
     chart spec, re-derived here in f64 from the same constants shots.ts
     uses, with stored values required to be the exact f32 cast (the v2
     data law, spec section 11);
  B. the no-fog control (scenes/fog-chart-nofog.scene.json) deep-equals the
     fog file except `name` and the omitted `fog` block;
  C. the Chrome staging (chrome/shots.ts) constants and construction,
     asserted against the same spec (source-text assertions), plus the
     engine camera defaults it relies on (template src/core/engine.ts:73);
  D. expected.json internal consistency: fog factors are three's
     smoothstep(4.5, 20, d), centre pixels are the NDC centres at
     1080x1920, base colours match the spec.

Prints a markdown table per section and a verdict line per check.
Exit 0 iff every check passes. Run from anywhere:

  python3 tools/fog-chart/chrome/verify_constants.py
"""
import json
import math
import re
import struct
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent            # tools/fog-chart/chrome
REPO = HERE.parents[2]                            # repo root
SKILL_TEMPLATE = Path.home() / "workspace/custom-skills/tall-tale-shorts/template"

# ---- the shared chart spec (one source, both engines) -----------------------
FOV = 40.0
ASPECT = 1080 / 1920
FOG_COLOR = "#C9CFC2"
FOG_NEAR, FOG_FAR = 4.5, 20.0
ROWS = [(3, 0.62), (6, 0.31), (10, 0.0), (15, -0.31), (25, -0.62)]
BASIC_NDC_X, STD_NDC_X = -0.45, 0.45
PANEL_NDC_W, PANEL_NDC_H = 0.62, 0.30
BASIC_COL = {3: "#D94F30", 6: "#3F6FD9", 10: "#7A8B99", 15: "#2FA89B", 25: "#8C8C46"}
STD_COL = {3: "#30D94F", 6: "#D9B83F", 10: "#8E44AD", 15: "#C94F7C", 25: "#466B8C"}
GROUND_COL = "#2E4A34"
LIGHT_DIR = (0.5, 0.8, 0.6)
AMBIENT, DIFFUSE = 0.35, 0.65

results = []  # (section, check, spec, staged, verdict)


def f32(x):
    """The exact f32 value of an f64, as a Python float."""
    return struct.unpack("f", struct.pack("f", float(x)))[0]


def is_f32(stored, derived):
    return stored == f32(derived)


def rec(section, check, spec, staged, ok):
    results.append((section, check, str(spec), str(staged), "PASS" if ok else "FAIL"))


def vec_eq_f32(stored, derived):
    return len(stored) == len(derived) and all(is_f32(s, d) for s, d in zip(stored, derived))


# ---- load inputs -------------------------------------------------------------
scene = json.loads((REPO / "scenes/fog-chart.scene.json").read_text())
nofog = json.loads((REPO / "scenes/fog-chart-nofog.scene.json").read_text())
expected = json.loads((HERE.parent / "expected.json").read_text())
shots_src = (HERE / "shots.ts").read_text()

# ---- A. native scene file vs spec --------------------------------------------
cam = scene["camera"]
rec("A scene", "camera.fovY", FOV, cam["fovY"], cam["fovY"] == FOV)
rec("A scene", "camera.near = f32(0.05)", f32(0.05), cam["near"], is_f32(cam["near"], 0.05))
rec("A scene", "camera.far", 800.0, cam["far"], cam["far"] == 800.0)
rec("A scene", "camera.eye", [0, 0, 0], cam["eye"], cam["eye"] == [0, 0, 0])
rec("A scene", "camera.target", [0, 0, -1], cam["target"], cam["target"] == [0, 0, -1])
rec("A scene", "camera.up", [0, 1, 0], cam["up"], cam["up"] == [0, 1, 0])
rec("A scene", "clear = fog colour", FOG_COLOR, scene["clear"], scene["clear"] == FOG_COLOR)

fog = scene.get("fog", {})
rec("A scene", "fog.color", FOG_COLOR, fog.get("color"), fog.get("color") == FOG_COLOR)
rec("A scene", "fog.near", FOG_NEAR, fog.get("near"), fog.get("near") == FOG_NEAR)
rec("A scene", "fog.far", FOG_FAR, fog.get("far"), fog.get("far") == FOG_FAR)

ld = scene["lighting"]["directional"]
rec("A scene", "lighting.direction = f32(0.5, 0.8, 0.6)", [f32(v) for v in LIGHT_DIR],
    ld["direction"], vec_eq_f32(ld["direction"], LIGHT_DIR))
rec("A scene", "lighting.ambient = f32(0.35)", f32(AMBIENT), ld["ambient"],
    is_f32(ld["ambient"], AMBIENT))

post = scene["post"]
bk = post["bokeh"]
rec("A scene", "post keys (bokeh only; no bloom/grade)", ["bokeh"], sorted(post.keys()),
    sorted(post.keys()) == ["bokeh"])
rec("A scene", "post.bokeh.focus", 10.0, bk["focus"], bk["focus"] == 10.0)
rec("A scene", "post.bokeh.aperture (bokeh disabled)", 0, bk["aperture"], bk["aperture"] == 0)
rec("A scene", "post.bokeh.maxblur = f32(0.006)", f32(0.006), bk["maxblur"],
    is_f32(bk["maxblur"], 0.006))

tex = {t["id"]: t for t in scene["textures"]}
for d, _ny in ROWS:
    t = tex.get(f"panel-basic-{d}m", {})
    rec("A scene", f"texture panel-basic-{d}m", f"solid {BASIC_COL[d]}",
        f"{t.get('generator')} {t.get('color')}",
        t.get("generator") == "solid" and t.get("color") == BASIC_COL[d])

nodes = {n["name"]: n for n in scene["nodes"]}
tanH = math.tan(math.radians(FOV / 2))
for d, ny in ROWS:
    hh = d * tanH
    hw = hh * ASPECT
    w, h, cy = PANEL_NDC_W * hw, PANEL_NDC_H * hh, ny * hh
    b = next(n for n in scene["nodes"] if n["name"].startswith(f"basic panel {d} "))
    g = b["geometry"]
    rec("A scene", f"basic {d} m geometry", f"quad f32({w:.6g}) x f32({h:.6g}), y0 f32({-h / 2:.6g})",
        f"{g['kind']} {g['width']} x {g['height']}, y0 {g['y0']}",
        g["kind"] == "quad" and is_f32(g["width"], w) and is_f32(g["height"], h)
        and is_f32(g["y0"], -h / 2))
    bt = b["transform"][0]["translate"]
    rec("A scene", f"basic {d} m translate", [f32(v) for v in (BASIC_NDC_X * hw, cy, -d)], bt,
        vec_eq_f32(bt, (BASIC_NDC_X * hw, cy, -float(d))))
    rec("A scene", f"basic {d} m texture ref", f"panel-basic-{d}m", b["texture"],
        b["texture"] == f"panel-basic-{d}m")
    s = next(n for n in scene["nodes"] if n["name"].startswith(f"standard panel {d} "))
    sg = s["geometry"]
    rec("A scene", f"standard {d} m geometry", f"box f32({w:.6g}) x f32({h:.6g}) x f32(0.1)",
        f"{sg['kind']} {sg['size']}",
        sg["kind"] == "box" and vec_eq_f32(sg["size"], (w, h, 0.1)))
    rec("A scene", f"standard {d} m material", f"{STD_COL[d]} rough 1.0",
        f"{s['material']['color']} rough {s['material']['roughness']}",
        s["material"]["color"] == STD_COL[d] and s["material"]["roughness"] == 1.0)
    st = s["transform"][0]["translate"]
    rec("A scene", f"standard {d} m translate", [f32(v) for v in (STD_NDC_X * hw, cy, -(d + 0.05))], st,
        vec_eq_f32(st, (STD_NDC_X * hw, cy, -(d + 0.05))))
    front = st[2] + 0.05
    rec("A scene", f"standard {d} m front face at exactly {d} m", float(-d), front,
        abs(front - (-d)) < 1e-6)

gr = next(n for n in scene["nodes"] if n["name"].startswith("ground"))
rec("A scene", "ground geometry", "box [80, f32(0.1), 72]", gr["geometry"]["size"],
    gr["geometry"]["kind"] == "box" and vec_eq_f32(gr["geometry"]["size"], (80, 0.1, 72)))
rec("A scene", "ground material", f"{GROUND_COL} rough 1.0",
    f"{gr['material']['color']} rough {gr['material']['roughness']}",
    gr["material"]["color"] == GROUND_COL and gr["material"]["roughness"] == 1.0)
gt = gr["transform"][0]["translate"]
rec("A scene", "ground translate", [0.0, f32(-8.05), -29.0], gt, vec_eq_f32(gt, (0, -8.05, -29)))

# ---- B. no-fog control ---------------------------------------------------------
a, b = dict(scene), dict(nofog)
a.pop("fog", None)
name_a, name_b = a.pop("name"), b.pop("name")
rec("B control", "nofog == fog file minus fog block + name", "deep-equal", "deep-equal" if a == b else "DIFFERS",
    a == b)
rec("B control", "nofog has no fog block", "absent", "absent" if "fog" not in nofog else "PRESENT",
    "fog" not in nofog)
rec("B control", "names differ (fog vs NO-FOG control)", "different", f"{name_a[:20]}... / {name_b[:20]}...",
    name_a != name_b)

# ---- C. Chrome staging (shots.ts source-text assertions) -----------------------
def has(pattern, label, spec_desc, staged_desc=None):
    ok = re.search(pattern, shots_src, re.S) is not None
    rec("C chrome", label, spec_desc, staged_desc or ("present" if ok else "ABSENT"), ok)


has(r"export const FOV = 40;", "FOV const", "40")
has(r"FOG_COLOR = 0xC9CFC2, FOG_NEAR = 4\.5, FOG_FAR = 20", "fog consts", "0xC9CFC2 / 4.5 / 20")
has(r"\[3, 0\.62\], \[6, 0\.31\], \[10, 0\], \[15, -0\.31\], \[25, -0\.62\]", "ROWS", "[d, ndcY] x5")
has(r"BASIC_NDC_X = -0\.45, STD_NDC_X = 0\.45", "panel NDC x", "-0.45 / +0.45")
has(r"PANEL_NDC_W = 0\.62, PANEL_NDC_H = 0\.30", "panel NDC size", "0.62 x 0.30")
for d, _ny in ROWS:
    has(rf"{d}: '{BASIC_COL[d]}'", f"BASIC_COL[{d}]", BASIC_COL[d])
    has(rf"{d}: '{STD_COL[d]}'", f"STD_COL[{d}]", STD_COL[d])
has(r"GROUND_COL = '#2E4A34'", "GROUND_COL", GROUND_COL)
has(r"scene\.fog = new THREE\.Fog\(FOG_COLOR, FOG_NEAR, FOG_FAR\)", "scene.fog set", "THREE.Fog(FOG_COLOR, 4.5, 20)")
has(r"scene\.background = new THREE\.Color\(FOG_COLOR\)", "background = fog colour", "Color(FOG_COLOR)")
has(r"camera\(FOV\)", "camera(FOV)", "engine camera(40)")
has(r"cam\.position\.set\(0, 0, 0\)", "camera at origin", "(0, 0, 0)")
has(r"cam\.lookAt\(0, 0, -1\)", "camera looks down -Z", "lookAt(0, 0, -1)")
has(r"AmbientLight\(0xffffff, 0\.35\)", "ambient light", "0.35 white")
has(r"DirectionalLight\(0xffffff, 0\.65\)", "directional light", "0.65 white")
has(r"dir\.position\.set\(10, 16, 12\)", "light dir = 20 x (0.5, 0.8, 0.6)", "(10, 16, 12)")
has(r"BoxGeometry\(80, 0\.1, 72\)", "ground box", "80 x 0.1 x 72")
has(r"ground\.position\.set\(0, -8\.05, -29\)", "ground position", "(0, -8.05, -29)")
has(r"MeshStandardMaterial\(\{ color: GROUND_COL, roughness: 1, metalness: 0 \}\)",
    "ground material", "GROUND_COL rough 1 metal 0")
has(r"hw = d \* tanH \* \(1080 / 1920\), hh = d \* tanH", "frustum halves", "hw = d*tanH*aspect, hh = d*tanH")
has(r"w = PANEL_NDC_W \* hw, h = PANEL_NDC_H \* hh, cy = ny \* hh", "panel size/centre", "0.62*hw, 0.30*hh, ny*hh")
has(r"basic\.position\.set\(BASIC_NDC_X \* hw, cy, -d\)", "basic position", "(-0.45*hw, cy, -d)")
has(r"BoxGeometry\(w, h, 0\.1\)", "standard box depth", "0.1")
has(r"std\.position\.set\(STD_NDC_X \* hw, cy, -\(d \+ 0\.05\)\)", "standard position", "(+0.45*hw, cy, -(d+0.05))")
has(r"MeshStandardMaterial\(\{ color: STD_COL\[d\], roughness: 1, metalness: 0 \}\)",
    "standard material", "STD_COL[d] rough 1 metal 0")
has(r"MeshBasicMaterial\(\{ map: flatTexture\(BASIC_COL\[d\]\) \}\)", "basic material", "flat texture, unlit")
has(r"const S = 64;.*cv\.width = S; cv\.height = S;", "flat texture size", "64x64 canvas")
has(r"tex\.colorSpace = THREE\.SRGBColorSpace", "flat texture colour space", "SRGBColorSpace")
has(r"generateMipmaps = false", "flat texture mipmaps off", "single level, linear")
has(r"focus: 10, aperture: 0, maxblur: 0\.006,\s*bloom: 0, exposure: 1, vignette: 0, grain: 0, warm: 0, sat: 1",
    "draw look (neutral, aperture 0)", "focus 10, aperture 0, maxblur 0.006, bloom 0, exposure 1, vignette 0, grain 0, warm 0, sat 1")

engine_src = (SKILL_TEMPLATE / "src/core/engine.ts").read_text()
ok = "new THREE.PerspectiveCamera(fov, W / H, 0.05, 800)" in engine_src
rec("C chrome", "engine camera() defaults (engine.ts)", "PerspectiveCamera(fov, W/H, 0.05, 800)",
    "present" if ok else "ABSENT", ok)

# ---- D. expected.json consistency ----------------------------------------------
def smoothstep(near, far, x):
    t = min(max((x - near) / (far - near), 0.0), 1.0)
    return t * t * (3 - 2 * t)


panels = {p["id"]: p for p in expected["panels"]}
for d, ny in ROWS:
    for kind, col, nx in (("basic", BASIC_COL[d], BASIC_NDC_X), ("standard", STD_COL[d], STD_NDC_X)):
        p = panels[f"{kind}-{d}m"]
        want_f = smoothstep(FOG_NEAR, FOG_FAR, d)
        rec("D expected", f"{kind}-{d}m fogFactor", want_f, p["fogFactor"],
            abs(p["fogFactor"] - want_f) < 1e-12)
        rec("D expected", f"{kind}-{d}m baseColor", col, p["baseColor"], p["baseColor"] == col)
        ex = round((nx + 1) / 2 * 1080)
        ey = math.floor((1 - ny) / 2 * 1920 + 0.5)  # round half up
        got = (p["centrePixel"]["x"], p["centrePixel"]["y"])
        rec("D expected", f"{kind}-{d}m centrePixel", (ex, ey), got, got == (ex, ey))
        rec("D expected", f"{kind}-{d}m centreNdc", [nx, ny], p["centreNdc"], p["centreNdc"] == [nx, ny])

ef = expected["fog"]
rec("D expected", "fog block", f"{FOG_COLOR} {FOG_NEAR}/{FOG_FAR}",
    f"{ef['color']} {ef['near']}/{ef['far']}",
    ef["color"] == FOG_COLOR and ef["near"] == FOG_NEAR and ef["far"] == FOG_FAR)
hl = expected["houseLight"]
rec("D expected", "houseLight dir/ambient", f"{list(LIGHT_DIR)} / {AMBIENT}",
    f"{hl['direction']} / {hl['ambient']}", hl["direction"] == list(LIGHT_DIR) and hl["ambient"] == AMBIENT)
ln = math.sqrt(sum(v * v for v in LIGHT_DIR))
want_factor = AMBIENT + DIFFUSE * (LIGHT_DIR[2] / ln)
rec("D expected", "houseLight front-face factor", want_factor, hl["factor"],
    abs(hl["factor"] - want_factor) < 1e-12)

# ---- report ----------------------------------------------------------------------
fails = [r for r in results if r[4] == "FAIL"]
cur = None
for section, check, spec, staged, verdict in results:
    if section != cur:
        cur = section
        print(f"\n### {section}\n")
        print("| check | spec (derived) | staged | verdict |")
        print("|---|---|---|---|")
    print(f"| {check} | {spec} | {staged} | {verdict} |")
print(f"\n**{len(results) - len(fails)}/{len(results)} checks PASS, {len(fails)} FAIL**")
sys.exit(1 if fails else 0)
