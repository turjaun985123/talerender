#!/usr/bin/env python3
"""castsheet_native.py — native (TaleRender) castsheet scene generator.

CHROME-REPLACEMENT WAVE, Batch 1, Seat 3 (CR-B1-03). Stdlib only.

The Chrome castsheet (template/src/castsheet.ts, staged by
tools/capture.mjs `castsheet`) draws ONE library item per frame on a
plain set with core/toon's drawToon and saves <kind>-<id>.jpg per item
plus a labelled castsheet.jpg. This generator emits the native
equivalent AS DATA: one TaleRender scene-format v2 file per item
(--scene-file is the binary's per-still render surface) plus a cast
manifest that tools/native_modes/castsheet_mode.mjs renders and tiles.

Two item sources:

  catalog (default)  the exact CAST_ITEMS enumeration of castsheet.ts:
                     POSE_NAMES x POSES, MOODS, LOOK_PRESETS keys,
                     PROPS keys (kind 'held'). 77 items.
  project cast       --project <dir> reads the project's cast
                     definitions (the `const LOOK: Record<string,
                     ToonLook>` literal in src/shots.ts — the
                     registerCast source in the Onoda project) or
                     --cast-json <file> ({"characters": [{"name": ...,
                     "look": {...}, "height": 1.75}]}) and emits one
                     kind='cast' item per character, neutral standing.
                     These are NOT library previews; they are the
                     project's own cast sheet.

Staging (transcribed from castsheet.ts `stage()`; see
docs/integration/CR-B1-03-CASTSHEET.md in the TaleRender repo for the
full spec and the native mapping):

  canvas 1080x1920; backdrop #EFE7D6; floor band #DCCBA6 from
  y=1560 (FLOOR_Y) down; soft ellipse shadow under the feet;
  figure centred, feet on FLOOR_Y, drawn at scale
  1.35 * (heightM / 1.75) of the 1024-unit stage (figure ~72% of
  frame height at the default 1.75 m); BASE_LOOK body #7A5FBF,
  hat 'none'.

  Native mapping: scene `clear` is unused for the backdrop (two
  solid-texture meshes carry the exact hexes, unlit); the floor is a
  flat-quad at y=0; the character node stands at the origin; the
  node's own blob shadow replaces the painted ellipse; the camera
  (eye/target [0, 0.78, 3.2] -> [0, 0.78, 0], fovY 42) puts the feet
  at ~0.83 of frame height and a 1.75 m figure at ~71% of frame
  height — the Chrome proportions within a percent. perspective is
  OFF and camPitch is 0: the Chrome castsheet is a flat 2D staging.

Held props: the native painter has NO PropFn painters — a held prop
enters a scene only as a pre-rendered RGBA plate texture
(scene-format §14.3; src/toonfull.rs Plate). A held item therefore
carries a holdR plate ONLY when a plate PNG exists in --plates-dir
(default <out>/plates). Items without a plate are emitted with
"plateMissing": true in the manifest and NO holdR — the mode reports
them as skipped, never silently drops them. Plate origins (x0, y0)
for the plates the film bridge already proved are in PLATE_ORIGINS
(source: talerender/film/gen_assets.py); any other plate defaults to
(-200, -200) and is flagged "plateOrigin": "assumed".

Usage:
  python3 tools/castsheet_native.py --out DIR [--kind pose|mood|look|held]
  python3 tools/castsheet_native.py --out DIR --project ~/faceless-shorts/<proj>
  python3 tools/castsheet_native.py --out DIR --cast-json cast.json
"""
import argparse
import json
import re
import sys
from pathlib import Path

# ---------------------------------------------------------------------------
# Transcribed tables. Every table names its source file; if the TS registry
# changes, this file must change with it (the manifest records the counts so
# drift is visible in the mode's summary line).
# ---------------------------------------------------------------------------

# Source: template/src/core/toon.ts — BASE_LOOK in template/src/castsheet.ts.
BASE_LOOK = {"body": "#7A5FBF", "hat": "none"}

# Source: template/src/core/poses.ts — POSES (arm convention [shoulder, elbow]
# degrees, 0 = hanging down). POSE_NAMES = Object.keys(POSES), same order.
POSES = {
    "point":     {"mood": "neutral", "armR": [92, -6], "armL": [12, 8], "lean": 2, "headTurn": 0.15},
    "wave":      {"mood": "happy", "armR": [158, 12], "armL": [10, 6]},
    "shrug":     {"mood": "neutral", "armL": [75, -60], "armR": [75, -60], "bob": -6},
    "think":     {"mood": "neutral", "armR": [48, -118], "armL": [20, -30], "headTilt": 6, "headTurn": 0.3, "look": [0.4, -0.3]},
    "celebrate": {"mood": "laugh", "armL": [150, 14], "armR": [150, 14], "bob": -8},
    "mourn":     {"mood": "sad", "armL": [4, 4], "armR": [4, 4], "lean": 5, "headTilt": -4, "bob": 4},
    "explain":   {"mood": "happy", "armR": [68, -42], "armL": [28, -24], "look": [0.3, 0]},
    "listen":    {"mood": "neutral", "armR": [118, -105], "armL": [10, 8], "lean": 3, "headTilt": -5, "headTurn": -0.2},
    "react":     {"mood": "shock", "armL": [34, -26], "armR": [34, -26], "lean": -3, "squash": 0.03},
    "facepalm":  {"mood": "embarrassed", "armR": [120, 78], "armL": [10, 6], "headTilt": -6, "lean": 4, "bob": 2},
    "sneak":     {"mood": "sly", "armL": [35, -55], "armR": [35, -55], "lean": 8, "bob": 2, "look": [0.5, 0]},
    "exhausted": {"mood": "tired", "armL": [4, 2], "armR": [4, 2], "lean": 9, "headTilt": -7, "bob": 5, "sweat": 1},
    "dazed":     {"mood": "dizzy", "armL": [70, -15], "armR": [105, -40], "lean": -4, "headTilt": 9, "squash": 0.02, "look": [0.3, 0.2]},
    "boast":     {"mood": "proud", "armR": [25, -105], "armL": [38, -110], "lean": -3, "bob": -2},
    "salute":    {"mood": "determined", "armR": [146, 30], "armL": [8, 6], "lean": 0, "headTilt": 0},
    "read":      {"mood": "neutral", "armL": [42, -48], "armR": [42, -48], "look": [0, 0.5], "headTilt": 4},
    "elderStoop": {"mood": "neutral", "armL": [12, 8], "armR": [20, -30], "lean": 7, "headTilt": -5, "bob": 3},
}
POSE_NAMES = list(POSES.keys())

# Source: template/src/core/toon.ts — MOODS (union order). NOTE: the native
# painter (talerender/src/toonfull.rs Mood::from_str) has 17 variants;
# 'puzzled' and 'deadpan' are NOT among them and fall back to neutral there —
# a named gap (CR-B1-03 §gaps), not something this generator hides.
MOODS = ["neutral", "smug", "happy", "laugh", "shock", "worried", "angry",
         "sad", "sly", "proud", "tired", "dizzy", "determined", "embarrassed",
         "awe", "suspicious", "relieved", "puzzled", "deadpan"]

# Source: template/src/core/looks.ts — LOOK_PRESETS (look / height / pose).
LOOK_PRESETS = {
    "kid":    {"look": {"kid": True}, "height": 1.35},
    "adult":  {"look": {}, "height": 1.75},
    "tall":   {"look": {}, "height": 1.95},
    "elder":  {"look": {}, "height": 1.68, "pose": "elderStoop"},
    "stocky": {"look": {}, "height": 1.66},
}

# Source: template/src/core/toon.ts — PROPS keys (PropsMap / PROPS object),
# in declaration order. castsheet.ts iterates Object.keys(PROPS) ONLY:
# the props2.ts items (nest, bandage, dropper, splint, berrySprig) and
# butterfly.ts's butterfly are library 'held' items but are NOT in
# CAST_ITEMS and are therefore NOT in the Chrome castsheet either.
# Factories (declared with <= 2 params in TS, invoked with no arguments by
# castsheet.ts propFn): book -> book('RULES', 0) closed; cash -> cash(1);
# paper -> paper('NEWS'); sign -> sign('SALE'); map -> map(undefined).
PROP_IDS = ["parcels", "book", "ticket", "purse", "letters", "rubberStamp",
            "stamp", "cash", "bill", "stack", "paper", "mirror", "cup", "key",
            "sign", "axe", "hammer", "wrench", "shovel", "apple", "bottle",
            "sandwich", "fish", "flower", "egg", "feather", "ball", "trophy",
            "phone", "camera", "radio", "umbrella", "lantern", "map", "rope",
            "medal"]
PROP_FACTORY_DEFAULTS = {"book": "RULES (closed)", "cash": "1 bill",
                         "paper": "NEWS", "sign": "SALE", "map": "default"}

# Source: template/src/castsheet.ts — the held staging state, verbatim.
HELD_STATE = {"armR": [72, -38], "armL": [14, 8]}

# Plate origins for the shipped prop-plate library (tools/prop_plates/,
# generated by tools/prop_plates.py — CR-B2-10: all 36 PropFn recipes ported
# and probe-proven). Origins pair with THOSE plates. The film bridge's own
# plate origins (paper 'MAP', book) live in talerender/film/gen_assets.py.
PLATE_ORIGINS = {
    "parcels": (-120.0, -229.0),
    "book": (-85.0, -175.0),
    "ticket": (-70.5, -97.0),
    "purse": (-48.5, -68.5),
    "letters": (-71.5, -95.5),
    "rubberStamp": (-44.5, -124.5),
    "stamp": (-28.0, -74.0),
    "cash": (-46.0, -65.0),
    "bill": (-34.0, -119.0),
    "stack": (-64.5, -124.5),
    "paper": (-82.0, -170.5),
    "mirror": (-59.0, -169.0),
    "cup": (-35.0, -75.0),
    "key": (-26.5, -86.5),
    "sign": (-105.0, -255.0),
    "axe": (-50.0, -154.5),
    "hammer": (-57.0, -133.0),
    "wrench": (-24.0, -128.0),
    "shovel": (-31.0, -173.5),
    "apple": (-29.0, -62.0),
    "bottle": (-22.0, -117.0),
    "sandwich": (-45.0, -45.0),
    "fish": (-58.5, -80.5),
    "flower": (-31.0, -115.0),
    "egg": (-26.0, -60.0),
    "feather": (-13.0, -102.0),
    "ball": (-39.0, -75.0),
    "trophy": (-50.0, -100.0),
    "phone": (-26.0, -83.0),
    "camera": (-51.0, -97.0),
    "radio": (-57.0, -150.0),
    "umbrella": (-105.0, -118.0),
    "lantern": (-29.0, -116.5),
    "map": (-82.5, -121.5),
    "rope": (-46.5, -73.5),
    "medal": (-27.5, -4.5),
}

# Staging constants. Source: template/src/castsheet.ts (FLOOR_Y, colours,
# scale) + castsheet.html (canvas 1080x1920); native camera derived in
# CR-B1-03-CASTSHEET.md §2.
STAGING = {
    "width": 1080, "height": 1920,
    "backdrop": "#EFE7D6", "floor": "#DCCBA6",
    "chromeFloorY": 1560, "chromeScaleAt175": 1.35,
    "camera": {"eye": [0, 0.78, 3.2], "target": [0, 0.78, 0], "fovY": 42},
    "baseLook": BASE_LOOK,
}

# Look keys the v2 character node consumes (src/scenefile.rs character
# branch). Anything else in a cast look is carried into the manifest for
# audit but does NOT reach the painter — see NATIVE_LOOK_DROPPED.
NATIVE_LOOK_KEYS = ["body", "head", "shade", "pattern", "patCol", "hat",
                    "hatCol", "band", "neck", "neckCol", "extra", "kid",
                    "mustacheCol"]
NATIVE_LOOK_DROPPED = ["patCol2", "dots", "stamps", "hairCol", "name",
                       "ears", "tail", "tailCol", "tailLens", "wing",
                       "hair", "scarf", "scarfCol"]


def scene_state(raw):
    """ToonState dict -> scene-format state dict. sweat/tears are numbers
    in TS (drawFace thresholds) but booleans in the scene loader
    (scenefile.rs truthy()); convert here, keep everything else verbatim."""
    st = {}
    for k, v in raw.items():
        if k in ("sweat", "tears"):
            st[k] = bool(v)
        else:
            st[k] = v
    return st


def build_scene(item):
    """One scene-format v2 document for a manifest item."""
    cam = STAGING["camera"]
    textures = [
        {"id": "wall", "generator": "solid", "color": STAGING["backdrop"]},
        {"id": "floor", "generator": "solid", "color": STAGING["floor"]},
    ]
    nodes = [
        {"type": "mesh",
         "geometry": {"kind": "quad", "width": 16, "height": 10, "y0": -3},
         "texture": "wall",
         "transform": [{"translate": [0, 0, -1.5]}]},
        {"type": "mesh",
         "geometry": {"kind": "flat-quad", "width": 16, "depth": 12},
         "texture": "floor",
         "transform": [{"translate": [0, 0, 0.5]}]},
    ]
    look = {k: v for k, v in item["look"].items() if k in NATIVE_LOOK_KEYS}
    state = scene_state(item.get("state") or {})
    if item.get("plate"):
        textures.append({"id": "prop-" + item["id"], "file": item["plate"]["file"]})
        state["holdR"] = {"plate": "prop-" + item["id"],
                          "x0": item["plate"]["x0"], "y0": item["plate"]["y0"]}
    nodes.append({
        "type": "character",
        "position": [0, 0, 0],
        "height": item.get("height", 1.75),
        "perspective": False,
        "look": look,
        "state": state,
    })
    return {
        "format": "talerender-scene",
        "version": 2,
        "name": "castsheet %s-%s (native)" % (item["kind"], item["id"]),
        "camera": {"eye": cam["eye"], "target": cam["target"], "fovY": cam["fovY"]},
        "clear": STAGING["backdrop"],
        "textures": textures,
        "nodes": nodes,
    }


def catalog_items(plates_dir):
    """The CAST_ITEMS enumeration of castsheet.ts, in its order."""
    items = []
    for pid in POSE_NAMES:
        items.append({"kind": "pose", "id": pid, "name": "pose " + pid,
                      "look": dict(BASE_LOOK), "state": dict(POSES[pid]),
                      "height": 1.75})
    for mid in MOODS:
        it = {"kind": "mood", "id": mid, "name": "mood " + mid,
              "look": dict(BASE_LOOK), "state": {"mood": mid},
              "height": 1.75}
        if mid in ("puzzled", "deadpan"):
            # toonfull.rs Mood::from_str has no such variants: the scene
            # renders NEUTRAL. Flagged so no consumer mistakes the frame
            # for the named mood (CR-B1-03 gap G2).
            it["nativeMoodFallback"] = "neutral"
        items.append(it)
    for lid, preset in LOOK_PRESETS.items():
        look = dict(BASE_LOOK)
        look.update(preset["look"])
        state = dict(POSES[preset["pose"]]) if preset.get("pose") else {}
        items.append({"kind": "look", "id": lid, "name": "look " + lid,
                      "look": look, "state": state,
                      "height": preset.get("height", 1.75)})
    for pid in PROP_IDS:
        it = {"kind": "held", "id": pid, "name": "prop " + pid,
              "look": dict(BASE_LOOK), "state": dict(HELD_STATE),
              "height": 1.75}
        plate = plates_dir / (pid + ".png")
        if plate.exists():
            x0, y0 = PLATE_ORIGINS.get(pid, (-200.0, -200.0))
            # texture `file` resolves relative to the SCENE file's dir
            # (scene-format §8), and scenes live in <out>/scenes/.
            it["plate"] = {"file": "../plates/%s.png" % pid, "x0": x0, "y0": y0,
                           "origin": "proven" if pid in PLATE_ORIGINS else "assumed"}
        else:
            it["plateMissing"] = True
        if pid in PROP_FACTORY_DEFAULTS:
            it["factoryDefault"] = PROP_FACTORY_DEFAULTS[pid]
        items.append(it)
    return items


# ---------------------------------------------------------------------------
# Project cast extraction. The Onoda-shaped source is a flat TS literal:
#   const LOOK: Record<string, ToonLook> = { name: { body: '#…', … }, … };
# Values are strings / booleans / string arrays / numbers only (ToonLook's
# flat fields). Anything richer fails loudly rather than parsing halfway.
# ---------------------------------------------------------------------------

def _balanced(text, start):
    depth = 0
    for i in range(start, len(text)):
        if text[i] == "{":
            depth += 1
        elif text[i] == "}":
            depth -= 1
            if depth == 0:
                return text[start:i + 1]
    raise ValueError("unbalanced braces in LOOK literal")


def _parse_value(raw):
    raw = raw.strip()
    if raw.startswith("["):
        return [a if a else b for a, b in
                re.findall(r"'([^']*)'|\"([^\"]*)\"", raw)]
    m = re.match(r"^'([^']*)'$|^\"([^\"]*)\"$", raw)
    if m:
        return m.group(1) if m.group(1) is not None else m.group(2)
    if raw in ("true", "false"):
        return raw == "true"
    try:
        return int(raw)
    except ValueError:
        try:
            return float(raw)
        except ValueError:
            raise ValueError("unsupported LOOK value: %r" % raw)


def _parse_flat_object(body):
    out = {}
    # split top-level commas (values here contain no nested objects)
    parts, depth, cur = [], 0, ""
    for ch in body:
        if ch == "[":
            depth += 1
        elif ch == "]":
            depth -= 1
        if ch == "," and depth == 0:
            parts.append(cur)
            cur = ""
        else:
            cur += ch
    if cur.strip():
        parts.append(cur)
    for part in parts:
        m = re.match(r"\s*([A-Za-z_]\w*)\s*:\s*(.+?)\s*$", part, re.S)
        if not m:
            raise ValueError("unsupported LOOK entry: %r" % part)
        out[m.group(1)] = _parse_value(m.group(2))
    return out


def cast_from_shots_ts(project):
    shots = Path(project) / "src" / "shots.ts"
    if not shots.exists():
        raise FileNotFoundError("no src/shots.ts in %s" % project)
    text = shots.read_text()
    m = re.search(r"const\s+LOOK[^=]*=\s*\{", text)
    if not m:
        raise ValueError("%s: no `const LOOK = {...}` cast literal found — "
                         "use --cast-json for this project's shape" % shots)
    literal = _balanced(text, m.end() - 1)
    inner = literal[1:-1]
    chars, i = [], 0
    entry = re.compile(r"\s*([A-Za-z_]\w*)\s*:\s*\{")
    while True:
        m2 = entry.match(inner, i)
        if not m2:
            break
        body = _balanced(inner, m2.end() - 1)
        chars.append({"name": m2.group(1),
                      "look": _parse_flat_object(body[1:-1]), "height": 1.75})
        i = m2.end() - 1 + len(body)
        comma = re.match(r"\s*,", inner[i:])
        if comma:
            i += comma.end()
    if not chars:
        raise ValueError("%s: LOOK literal parsed to zero characters" % shots)
    return chars


def cast_items(characters):
    items = []
    for c in characters:
        look = dict(c["look"])
        dropped = [k for k in look if k in NATIVE_LOOK_DROPPED]
        items.append({"kind": "cast", "id": c["name"], "name": "cast " + c["name"],
                      "look": look, "state": {},
                      "height": c.get("height", 1.75),
                      "droppedLookFields": dropped})
    return items


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--out", required=True, help="work dir: scenes/ + cast-manifest.json")
    ap.add_argument("--kind", default="", help="catalog filter: pose|mood|look|held")
    ap.add_argument("--project", default="", help="project dir: emit its cast (kind=cast)")
    ap.add_argument("--cast-json", default="", help='{"characters":[{name, look, height?}]}')
    ap.add_argument("--plates-dir", default="",
                    help="dir holding <prop-id>.png plates (default <out>/plates)")
    a = ap.parse_args(argv)

    out = Path(a.out)
    plates_dir = Path(a.plates_dir) if a.plates_dir else out / "plates"
    if a.project:
        items = cast_items(cast_from_shots_ts(a.project))
        source = "project:%s" % a.project
    elif a.cast_json:
        data = json.loads(Path(a.cast_json).read_text())
        items = cast_items(data["characters"])
        source = "cast-json:%s" % a.cast_json
    else:
        items = catalog_items(plates_dir)
        source = "catalog (castsheet.ts CAST_ITEMS)"
        if a.kind:
            items = [it for it in items if it["kind"] == a.kind]
            if not items:
                print("castsheet_native: no catalog items of kind %r" % a.kind,
                      file=sys.stderr)
                return 2

    scenes = out / "scenes"
    scenes.mkdir(parents=True, exist_ok=True)
    for it in items:
        it["scene"] = "scenes/%s-%s.scene.json" % (it["kind"], it["id"])
        (out / it["scene"]).write_text(json.dumps(build_scene(it), indent=1) + "\n")

    manifest = {
        "format": "talerender-castsheet", "version": 1,
        "source": source, "staging": STAGING,
        "counts": {
            "items": len(items),
            "plateMissing": sum(1 for it in items if it.get("plateMissing")),
        },
        "items": items,
    }
    (out / "cast-manifest.json").write_text(json.dumps(manifest, indent=1) + "\n")
    print("castsheet_native: %d items (%d plate-missing) -> %s"
          % (len(items), manifest["counts"]["plateMissing"], out))
    return 0


if __name__ == "__main__":
    sys.exit(main())
