#!/usr/bin/env python3
"""check_unknown_fields.py — strictness checker for JSON field
vocabulary (W42, gap-fix F3).

The loaders never reject an unknown KEY. src/scenefile.rs and
src/film.rs read the fields they know via Value::get and ignore
everything else, so a typo'd or dead key renders exactly like an
absent one (verified by the external gap analysis: a root
"bogus": 123 renders fine). The vocabulary of VALUES is closed —
unknown node types, geometry kinds, texture generators and
transform ops are hard loader errors — but the vocabulary of KEYS
was, until this tool, written down nowhere and checked by nothing.

This checker makes unknown keys visible, REPORT-ONLY: it changes
no loader behaviour. It walks a JSON file, classifies it by shape,
and reports every key outside the known set for its object context,
by JSON path. Three classes of key exist, and the distinction is
the point:

  KNOWN     read by a Rust loader (src/scenefile.rs for scene
            files, src/film.rs load_film_data for film data) —
            the sets below are transcribed from that source.
  TOLERATED not read by any Rust loader, but owned elsewhere:
            consumed by a named Python tool, or a deliberate
            record field documented in scenes/film/ASSUMPTIONS.md
            / scenes/sets/README.md. Every tolerated key carries
            its citation in the ALLOWLIST below and is LISTED in
            the report (check 5), never silently passed. Keys
            starting with "_" are the corpus's in-file
            documentation convention (atmospheres.json's _cite /
            _note prose records; check_lights.py skips
            "_"-prefixed keys when extracting vocabularies) and
            are tolerated on that citation everywhere.
  UNKNOWN   nobody reads it and nothing documents it. Reported
            as a finding (checks 2-4) and the run FAILs.

Checks (numbered like tools/film/validate_film.py):

  1. parse + classify  every file parses; each file is classified
                       SCENE / FILM-DATA / MANIFEST / PLATES /
                       ATMOSPHERES / OVERLAY-RECORD / UNCLASSIFIED
                       by shape. Unreadable or non-JSON input is
                       an input error (exit 2), not a finding.
  2. scene vocabulary  SCENE files: every key of every
                       loader-owned object (root, camera + keys,
                       lighting, lights rig, textures, nodes
                       recursively — geometry per kind, material,
                       colour objects, texture refs, character
                       look/state/holds/attachments, scatter
                       placement, transform ops, post, fog) is in
                       the src/scenefile.rs set.
  3. film-data vocabulary  FILM-DATA files (the --film driver's
                       film.json / onoda.json): root, lines[] and
                       captions[] keys are in the src/film.rs
                       load_film_data set. `anchors` is a free-form
                       word->time map: its keys are data, never
                       fields, and are not descended into.
  4. manifest vocabulary  MANIFEST files (scenes/film/film.json):
                       root, resolution, audio and segments[]
                       keys are in the set the Python tooling
                       (validate_film.py, check_assets.py,
                       rerender_plan.py) actually reads. No Rust
                       loader reads the manifest.
  5. tolerated register  every TOLERATED key encountered is listed
                       per file with its allowlist citation.
                       PLATES (a free-form name->[x,y] map read by
                       src/film.rs load_assets), ATMOSPHERES
                       (tooling data; presets / filmLooks are
                       free-form maps), OVERLAY-RECORD and
                       UNCLASSIFIED files have no loader key
                       vocabulary: their root keys are listed
                       here informationally and they cannot FAIL.

Descent rules: loader-owned objects are descended into;
TOLERATED keys are reported at the key and their subtrees are NOT
descended (the staging records under `shot`, `captions`,
`channels`, `overlays` belong to their own tools' validators);
free-form maps (anchors, presets, filmLooks, plates) are not
descended. Geometry and texture-entry objects are checked against
PER-KIND sets when their kind / generator names a known one, and
against the union set otherwise. This checker judges KEYS only:
requiredness, types and value ranges are the other validators'
and the loaders' business.

Usage: python3 tools/format/check_unknown_fields.py FILE [FILE ...]
Exit code: 0 = no unknown keys in any classified loader/tooling
file, 1 = at least one unknown key (listed by path), 2 = input
error (no files, a file unreadable / not JSON — one-line ERROR on
stderr, no report). Output is deterministic (command-line order;
findings aggregated by path pattern with occurrence counts and
the first exact path).
"""

import argparse
import json
import re
import sys


class InputError(Exception):
    pass


def load_json_input(path, what):
    try:
        with open(path, "r", encoding="utf-8") as fh:
            return json.load(fh)
    except (OSError, ValueError) as e:
        raise InputError("%s: cannot read %s (%s)" % (what, path, e))


# ---------------------------------------------------------------------
# KNOWN sets — transcribed from the loaders' source.
# src/scenefile.rs: load() (root, camera, lighting, lights,
# textures, post, fog), walk_nodes() (nodes), colour_geo /
# tex_geo / scatter_geo (geometry + placement), material_colour,
# resolve_texture, eval_camera_keys, compose_transform.
# ---------------------------------------------------------------------

SCENE_ROOT = {"format", "version", "time", "name", "camera",
              "clear", "lighting", "lights", "textures", "nodes",
              "post", "fog"}
CAMERA = {"eye", "target", "fovY", "near", "far", "up", "keys"}
CAMERA_KEY = {"t", "eye", "target", "fovY", "ease"}
LIGHTING = {"directional"}
LIGHTING_DIR = {"direction", "ambient"}
LIGHTS = {"key", "rim", "hemisphere", "points", "envIntensity"}
LIGHT_KEY = {"color", "intensity", "position", "target", "shadow",
             "shadowBox", "far", "near", "bias", "normalBias"}
LIGHT_RIM = {"color", "intensity", "position"}
LIGHT_HEMI = {"sky", "ground", "intensity"}
LIGHT_POINT = {"color", "intensity", "position", "distance",
               "decay"}
NODE = {"type", "transform", "children", "geometry", "material",
        "texture", "id", "position", "height", "perspective",
        "look", "state", "attachments", "count", "placement"}
TRANSFORM_OPS = {"translate", "rotate_x", "rotate_y", "rotate_z",
                 "euler_xyz", "scale"}
GEOMETRY_BY_KIND = {
    "box": {"kind", "size"},
    "cylinder": {"kind", "radiusTop", "radiusBottom", "height",
                 "segments"},
    "sphere": {"kind", "radius", "widthSegments",
               "heightSegments"},
    "cone": {"kind", "radius", "height", "segments"},
    "disc": {"kind", "radius", "segments"},
    "quad": {"kind", "width", "height", "y0"},
    "flat-quad": {"kind", "width", "depth"},
    "ground-disc": {"kind", "radius", "segments", "uvRepeat"},
    "sky-sphere": {"kind", "radius", "widthSegments",
                   "heightSegments"},
    "billboard": {"kind", "center", "width", "height"},
}
GEOMETRY_UNION = set().union(*GEOMETRY_BY_KIND.values())
MATERIAL = {"color", "roughness"}
COLOR_OBJ = {"hex", "shade", "shadeHash"}
SHADEHASH = {"index", "base", "span"}
TEXTURE_REF = {"id", "wrap"}
PLACEMENT = {"kind", "angleHash", "radiusHash", "radiusBase",
             "radiusSpan", "angleScale", "center"}
LOOK = {"body", "head", "shade", "pattern", "patCol", "hat",
        "hatCol", "band", "neck", "neckCol", "mustacheCol",
        "extra", "kid"}
STATE = {"mood", "look", "blink", "talk", "sweat", "tears",
         "armL", "armR", "legs", "phase", "lean", "bob", "squash",
         "headTilt", "headTurn", "camPitch", "sitOn", "holdL",
         "holdR"}
HOLD = {"plate", "x0", "y0"}
ATTACHMENT = {"kind", "hand", "offset"}
TEXTURE_BY_GENERATOR = {
    "grass": {"base"},
    "planks": {"base"},
    "wallpaper": {"base", "motif"},
    "felt": {"base"},
    "parcel": {"label"},
    "picture": set(),
    "sign": {"canvas", "bg", "fg"},
    "solid": {"color"},
    "sky-gradient": {"top", "horizon", "ground"},
    "cloud": set(),
    "blob-shadow": set(),
    "onoda-card": set(),
    "dof-chart-pattern": {"size"},
}
TEXTURE_ENTRY = ({"id", "file", "generator"}
                 | set().union(*TEXTURE_BY_GENERATOR.values()))
# Per-generator records the loader does not read for THAT
# generator (the key may be a loader field for another one).
TEXTURE_TOLERATED_BY_GENERATOR = {
    "parcel": {
        "base": "set-dialect record: parcel entries carry the "
                "source's base colour (scenes/sets/README.md "
                "B8: the texture entries declare the source's "
                "base colours, canvas sizes and repeats); "
                "src/scenefile.rs's parcel painter reads only "
                "`label`",
    },
}
POST = {"bokeh", "bloom", "grade"}
BOKEH = {"focus", "aperture", "maxblur"}
BLOOM = {"threshold", "strength", "radius"}
GRADE = {"vignette", "grain", "warm", "sat", "exposure", "time"}
FOG = {"color", "near", "far"}

# src/film.rs load_film_data: the --film driver's data file.
FILM_ROOT = {"anchors", "lines", "captions", "duration",
             # F2 migration (W53 T2): declared by the film data,
             # consumed by load_film_data since the migration.
             "calendarYears", "calendarDark", "sign"}
FILM_LINE = {"start", "end"}
FILM_CAPTION = {"s", "e", "text", "fit", "strip"}

# scenes/film/film.json — read by the Python tooling only:
# validate_film.py (segments, scene, id, filmLook, set,
# atmosphere), check_assets.py (segments[].scene, sourceData),
# rerender_plan.py (segment timing fields, captionChunks).
MANIFEST_ROOT = {"version", "format", "title", "project",
                 "resolution", "fps", "durationS", "totalFrames",
                 "frameRule", "boundaryRule", "sourceData",
                 "audio", "segments"}
MANIFEST_RESOLUTION = {"width", "height"}
MANIFEST_AUDIO = {"file", "durationS", "loudnessLUFS",
                  "treatment", "projectPath"}
MANIFEST_SEGMENT = {"id", "scene", "set", "atmosphere",
                    "filmLook", "lines", "startS", "endS",
                    "durationS", "startFrame", "endFrame",
                    "frames", "captionChunks"}

# ---------------------------------------------------------------------
# ALLOWLIST — TOLERATED keys: not read by any Rust loader, owned
# elsewhere. (context, key) -> citation. A key with no citation
# does not belong here; that is what keeps the list honest.
# ---------------------------------------------------------------------

ALLOWLIST = {
    # scene root — staging records riding alongside the loader data
    ("scene-root", "shot"):
        "staging record; read by tools/film/validate_film.py "
        "(shot.atmosphere fallback) and tools/format/check_lights.py "
        "(shot.fire.pit, shot.atmosphere)",
    ("scene-root", "captions"):
        "per-shot caption staging; read by tools/film/"
        "validate_film.py (caption id census) and tools/film/"
        "rerender_plan.py (chunk matching)",
    ("scene-root", "captionsNote"):
        "caption provenance note; read by tools/film/"
        "rerender_plan.py, documented in tools/film/VALIDATION.md",
    ("scene-root", "duration"):
        "shot-duration record (scenes/film/ASSUMPTIONS.md staging "
        "corpus); the loader's time source is `time` + the manifest",
    ("scene-root", "resolution"):
        "frame-size record (scenes/film/ASSUMPTIONS.md staging "
        "corpus); the loader takes --width/--height",
    ("scene-root", "atmosphere"):
        "preset naming field; read by tools/film/validate_film.py "
        "(declared-atmosphere check) and tools/format/"
        "check_lights.py check 4 (R5)",
    ("scene-root", "atmospherePreset"):
        "record of RESOLVED preset values, read by no loader — "
        "scenes/film/ASSUMPTIONS.md (the s06 record); "
        "check_lights.py deliberately does not check it "
        "(LIGHTS-W26)",
    ("scene-root", "channels"):
        "animation-channel formula records (scenes/film/"
        "ASSUMPTIONS.md); cf. node `channels`, consumed by "
        "tools/format/stage_clips.py",
    ("scene-root", "fire"):
        "fire record; tools/format/check_lights.py check 6 tier B "
        "reads fire.pit (LIGHTS-W26)",
    ("scene-root", "overlays"):
        "overlay records; tools/format/check_assets.py walks "
        "overlays[].ref (ASSETS-W28)",
    ("scene-root", "background"):
        "set record: the interior's scene.background colour "
        "(scenes/sets/README.md)",
    ("scene-root", "bookRows"):
        "set record: 'the file's bookRows block records the full "
        "evaluated result' (scenes/sets/README.md)",
    # nodes
    ("node", "name"):
        "authoring label; read by tools/film/validate_film.py "
        "(character-node diagnostics); present on nearly every "
        "node in the corpus",
    ("node", "channels"):
        "animation-channel records; consumed by tools/format/"
        "stage_clips.py (ANIM-ADOPT-W32), documented in scenes/"
        "film/ASSUMPTIONS.md",
    ("node", "tint"):
        "source sprite tint record (scenes/film/ASSUMPTIONS.md); "
        "the loader's billboard is untinted — the rendered tint "
        "is carried by the film driver's dressing",
    ("node", "envTint"):
        "environment tint record (scenes/film/ASSUMPTIONS.md)",
    ("node", "role"):
        "staging annotation, documented in scenes/film/"
        "ASSUMPTIONS.md ('\"role\": \"walk-target\"')",
    ("node", "rifleNote"):
        "attachment provenance record (scenes/film/"
        "ASSUMPTIONS.md); the loader mechanism is attachments[]",
    # materials — source-material records the loader does not
    # consume (ASSUMPTIONS.md is explicit that they are carried
    # data, recoverable, not loader fields)
    ("material", "map"):
        "source-material texture record (scenes/film/"
        "ASSUMPTIONS.md); the loader's texture mechanism is the "
        "NODE's texture field — a colour-pipeline node cannot "
        "carry a map in format v2",
    ("material", "metalness"):
        "carried material field: 'the loader consumes metalness "
        "nowhere yet, but the value is now recoverable from "
        "scene data' (scenes/film/ASSUMPTIONS.md, W73 correction)",
    ("material", "doubleSide"):
        "carried source-material record (scenes/film/"
        "ASSUMPTIONS.md)",
    ("material", "basic"):
        "source material-kind record: MeshBasicMaterial surfaces "
        "carry '\"basic\": true' (scenes/film/ASSUMPTIONS.md)",
    ("material", "emissive"):
        "recorded additive material field (scenes/film/"
        "ASSUMPTIONS.md, 'Bulb emissive')",
    ("material", "emissiveIntensity"):
        "recorded additive material field (scenes/film/"
        "ASSUMPTIONS.md, 'Bulb emissive')",
    # texture entries
    ("texture", "repeat"):
        "source UV-repeat record (scenes/film/ASSUMPTIONS.md "
        "names the source repeat factors, e.g. T.grass(col, "
        "[6, 3])); the loader's repeat vocabulary is the "
        "texture-ref wrap + ground-disc uvRepeat",
    ("texture", "sourceDrawing"):
        "provenance record (scenes/film/ASSUMPTIONS.md)",
    ("texture", "canvas"):
        "canvas-size record carried by every canvasTex-derived "
        "texture entry (scenes/film/ASSUMPTIONS.md: 'Each entry "
        "carries its full parameters (base/motif/repeat/canvas "
        "size...)'); src/scenefile.rs reads `canvas` only for "
        "the sign generator, where it is KNOWN",
    ("texture", "text"):
        "the sign's words — the recorded signBoard(text, ...) "
        "input (scenes/sets/README.md B9); the native sign "
        "painter draws block glyphs and reads bg/fg/canvas only",
    ("texture", "font"):
        "sign font record (scenes/sets/README.md B9: the "
        "signBoard rule depends on the browser's Caladea "
        "metrics, which cannot be reproduced from source)",
    ("texture", "rotation"):
        "source map.rotation record (scenes/sets/README.md, "
        "bookshop floor row: 'texture rotation PI/2'); the "
        "loader has no texture-rotation field — the rendered "
        "equivalent is baked into the geometry orientation",
    # bookshop set dialect (scenes/sets/README.md)
    ("light-key", "radius"):
        "source literal carried on the lights.key block: 'the "
        "interior key's shadow radius: 4 and mapSize: 2048 are "
        "source literals (world.ts:326) carried on the "
        "lights.key block; the \u00a714.2 loader type-checks a "
        "subset' (scenes/sets/README.md B7)",
    ("light-key", "mapSize"):
        "source literal carried on the lights.key block "
        "(scenes/sets/README.md B7, quoted at `radius`)",
    ("node", "receiveShadow"):
        "source shadow-flag fact recorded on the node "
        "(scenes/sets/README.md B6 / shore G6: castShadow / "
        "receiveShadow are source facts for the record; the "
        "loader has no per-node shadow flags)",
    ("node", "basic"):
        "basic-surface marker on the NODE in the set dialect "
        "(scenes/sets/README.md, the window view plane passage: "
        "'\"basic\": true on the affected nodes plus toneMapped "
        "on...'); cf. material `basic` in the film scenes",
    ("material", "toneMapped"):
        "source material record (scenes/sets/README.md, the "
        "window view plane passage: toneMapped: false — its "
        "#BFE4F7 is a flat colour); the loader's output pass "
        "tone-maps uniformly",
    # post
    ("post", "focusChannel"):
        "focus-driver record: 'post.focusChannel records...' "
        "while post.bokeh.focus carries the value (scenes/film/"
        "ASSUMPTIONS.md)",
    # light points — a two-consumer dialect fact: scenefile.rs
    # does NOT read it; the Python light validator does
    ("light-point", "channel"):
        "fire-channel marker; read by tools/format/"
        "check_lights.py check 6 tier A (channel.kind == "
        "'fireLevel') — LIGHTS-W26. src/scenefile.rs's PointCfg "
        "has no channel field",
}


# ---------------------------------------------------------------------
# Walking. Findings aggregate by (file, path-pattern, key):
# pattern paths use [] for array indices; the first exact path is
# kept as the example.
# ---------------------------------------------------------------------

class Findings:
    def __init__(self):
        self.unknown = {}    # (label, pattern, key, ctx) -> [n, first]
        self.tolerated = {}  # (label, pattern, key, ctx) -> [n, cite]

    def _add(self, table, label, pattern, exact, key, ctx, cite=None):
        k = (label, pattern, key, ctx)
        if k in table:
            table[k][0] += 1
        else:
            table[k] = [1, exact, cite]

    def check_obj(self, label, obj, known, ctx, path,
                  extra_tolerated=None):
        """Check one object's keys against `known` + the allowlist
        for context `ctx`. Returns nothing; records findings.
        Findings aggregate by path PATTERN (array indices folded
        to []) with the first exact key path kept as the example.
        `extra_tolerated` maps individual keys to citations for
        context narrower than `ctx` (per-generator texture
        records). "_"-prefixed keys are the corpus's in-file
        documentation convention and are tolerated everywhere."""
        if not isinstance(obj, dict):
            return
        pattern = re.sub(r"\[\d+\]", "[]", path)
        for key in obj:
            if key in known:
                continue
            exact = "%s.%s" % (path, key)
            if key.startswith("_"):
                cite = ("in-file documentation key (the corpus's "
                        "'_' convention: scenes/atmospheres.json "
                        "carries _cite / _note prose records; "
                        "tools/format/check_lights.py skips "
                        "'_'-prefixed keys when extracting "
                        "vocabularies)")
            elif extra_tolerated and key in extra_tolerated:
                cite = extra_tolerated[key]
            else:
                cite = ALLOWLIST.get((ctx, key))
            if cite is not None:
                self._add(self.tolerated, label, pattern, exact,
                          key, ctx, cite)
            else:
                self._add(self.unknown, label, pattern, exact,
                          key, ctx)



def walk_nodes(find, label, nodes, path):
    if not isinstance(nodes, list):
        return
    for i, node in enumerate(nodes):
        np = "%s[%d]" % (path, i)
        if not isinstance(node, dict):
            continue
        find.check_obj(label, node, NODE, "node", np)
        geo = node.get("geometry")
        if isinstance(geo, dict):
            kind = geo.get("kind")
            known = GEOMETRY_BY_KIND.get(kind, GEOMETRY_UNION)
            find.check_obj(label, geo, known, "geometry",
                           np + ".geometry")
        mat = node.get("material")
        if isinstance(mat, dict):
            find.check_obj(label, mat, MATERIAL, "material",
                           np + ".material")
            col = mat.get("color")
            if isinstance(col, dict):
                find.check_obj(label, col, COLOR_OBJ, "color",
                               np + ".material.color")
                sh = col.get("shadeHash")
                if isinstance(sh, dict):
                    find.check_obj(label, sh, SHADEHASH,
                                   "shadeHash",
                                   np + ".material.color.shadeHash")
        tex = node.get("texture")
        if isinstance(tex, dict):
            find.check_obj(label, tex, TEXTURE_REF, "texture-ref",
                           np + ".texture")
        tr = node.get("transform")
        if isinstance(tr, list):
            for j, op in enumerate(tr):
                if isinstance(op, dict):
                    find.check_obj(label, op, TRANSFORM_OPS,
                                   "transform-op",
                                   "%s.transform[%d]" % (np, j))
        look = node.get("look")
        if isinstance(look, dict):
            find.check_obj(label, look, LOOK, "look", np + ".look")
        state = node.get("state")
        if isinstance(state, dict):
            find.check_obj(label, state, STATE, "state",
                           np + ".state")
            for h in ("holdL", "holdR"):
                hv = state.get(h)
                if isinstance(hv, dict):
                    find.check_obj(label, hv, HOLD, "hold",
                                   np + ".state." + h)
        atts = node.get("attachments")
        if isinstance(atts, list):
            for j, att in enumerate(atts):
                if isinstance(att, dict):
                    find.check_obj(label, att, ATTACHMENT,
                                   "attachment",
                                   "%s.attachments[%d]" % (np, j))
        plc = node.get("placement")
        if isinstance(plc, dict):
            find.check_obj(label, plc, PLACEMENT, "placement",
                           np + ".placement")
        walk_nodes(find, label, node.get("children"),
                   np + ".children")


def walk_scene(find, label, root):
    find.check_obj(label, root, SCENE_ROOT, "scene-root", "$")
    cam = root.get("camera")
    if isinstance(cam, dict):
        find.check_obj(label, cam, CAMERA, "camera", "$.camera")
        keys = cam.get("keys")
        if isinstance(keys, list):
            for i, k in enumerate(keys):
                if isinstance(k, dict):
                    find.check_obj(label, k, CAMERA_KEY,
                                   "camera-key",
                                   "$.camera.keys[%d]" % i)
    lighting = root.get("lighting")
    if isinstance(lighting, dict):
        find.check_obj(label, lighting, LIGHTING, "lighting",
                       "$.lighting")
        d = lighting.get("directional")
        if isinstance(d, dict):
            find.check_obj(label, d, LIGHTING_DIR,
                           "lighting-directional",
                           "$.lighting.directional")
    lights = root.get("lights")
    if isinstance(lights, dict):
        find.check_obj(label, lights, LIGHTS, "lights", "$.lights")
        for sub, known, ctx in (("key", LIGHT_KEY, "light-key"),
                                ("rim", LIGHT_RIM, "light-rim"),
                                ("hemisphere", LIGHT_HEMI,
                                 "light-hemisphere")):
            v = lights.get(sub)
            if isinstance(v, dict):
                find.check_obj(label, v, known, ctx,
                               "$.lights." + sub)
        pts = lights.get("points")
        if isinstance(pts, list):
            for i, p in enumerate(pts):
                if isinstance(p, dict):
                    find.check_obj(label, p, LIGHT_POINT,
                                   "light-point",
                                   "$.lights.points[%d]" % i)
    textures = root.get("textures")
    if isinstance(textures, list):
        for i, t in enumerate(textures):
            if isinstance(t, dict):
                gen = t.get("generator")
                extra = TEXTURE_BY_GENERATOR.get(gen)
                known = (TEXTURE_ENTRY if extra is None
                         else {"id", "file", "generator"} | extra)
                find.check_obj(
                    label, t, known, "texture",
                    "$.textures[%d]" % i,
                    extra_tolerated=TEXTURE_TOLERATED_BY_GENERATOR
                    .get(gen))
    walk_nodes(find, label, root.get("nodes"), "$.nodes")
    post = root.get("post")
    if isinstance(post, dict):
        find.check_obj(label, post, POST, "post", "$.post")
        for sub, known in (("bokeh", BOKEH), ("bloom", BLOOM),
                           ("grade", GRADE)):
            v = post.get(sub)
            if isinstance(v, dict):
                find.check_obj(label, v, known, "post-" + sub,
                               "$.post." + sub)
    fog = root.get("fog")
    if isinstance(fog, dict):
        find.check_obj(label, fog, FOG, "fog", "$.fog")


def walk_film_data(find, label, root):
    find.check_obj(label, root, FILM_ROOT, "film-root", "$")
    lines = root.get("lines")
    if isinstance(lines, list):
        for i, ln in enumerate(lines):
            if isinstance(ln, dict):
                find.check_obj(label, ln, FILM_LINE, "film-line",
                               "$.lines[%d]" % i)
    caps = root.get("captions")
    if isinstance(caps, list):
        for i, c in enumerate(caps):
            if isinstance(c, dict):
                find.check_obj(label, c, FILM_CAPTION,
                               "film-caption", "$.captions[%d]" % i)
    # `anchors` is a free-form word->time map: keys are data.


def walk_manifest(find, label, root):
    find.check_obj(label, root, MANIFEST_ROOT, "manifest-root", "$")
    res = root.get("resolution")
    if isinstance(res, dict):
        find.check_obj(label, res, MANIFEST_RESOLUTION,
                       "manifest-resolution", "$.resolution")
    audio = root.get("audio")
    if isinstance(audio, dict):
        find.check_obj(label, audio, MANIFEST_AUDIO,
                       "manifest-audio", "$.audio")
    segs = root.get("segments")
    if isinstance(segs, list):
        for i, s in enumerate(segs):
            if isinstance(s, dict):
                find.check_obj(label, s, MANIFEST_SEGMENT,
                               "manifest-segment",
                               "$.segments[%d]" % i)


def classify(root, path):
    if not isinstance(root, dict):
        return "UNCLASSIFIED"
    if isinstance(root.get("segments"), list) and "format" in root:
        return "MANIFEST"
    if all(k in root for k in ("anchors", "lines", "captions")):
        return "FILM-DATA"
    if isinstance(root.get("nodes"), list):
        return "SCENE"
    if "presets" in root and "filmLooks" in root:
        return "ATMOSPHERES"
    if root.get("kind") == "overlay":
        return "OVERLAY-RECORD"
    if root and all(isinstance(v, list) and len(v) == 2
                    and all(isinstance(x, (int, float))
                            for x in v)
                    for v in root.values()):
        return "PLATES"
    return "UNCLASSIFIED"


CHECK_NAMES = {
    1: "parse + classify",
    2: "scene vocabulary",
    3: "film-data vocabulary",
    4: "manifest vocabulary",
    5: "tolerated register",
}
CHECK_TITLES = {
    1: "every file parses and is classified by shape",
    2: "scene files: keys subset of the src/scenefile.rs sets "
       "(unknown named by path)",
    3: "film data: keys subset of the src/film.rs load_film_data "
       "set (unknown named by path)",
    4: "film manifest: keys subset of the Python tooling's set "
       "(unknown named by path)",
    5: "allowlisted keys listed with citations; vocabulary-less "
       "files listed informationally (never a FAIL)",
}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("files", nargs="*",
                    help="JSON files to check")
    args = ap.parse_args()
    if not args.files:
        print("ERROR: check_unknown_fields.py: no files given",
              file=sys.stderr)
        return 2
    try:
        docs = []
        for path in args.files:
            docs.append((path, load_json_input(path, "input file")))
    except InputError as e:
        print("ERROR: check_unknown_fields.py: %s" % e,
              file=sys.stderr)
        return 2

    find = Findings()
    kinds = {}
    freeform_notes = []
    per_check_fail = {2: [], 3: [], 4: []}
    for label, root in docs:
        kind = classify(root, label)
        kinds[label] = kind
        if kind == "SCENE":
            walk_scene(find, label, root)
        elif kind == "FILM-DATA":
            walk_film_data(find, label, root)
        elif kind == "MANIFEST":
            walk_manifest(find, label, root)
        else:
            if kind == "PLATES":
                freeform_notes.append(
                    "%s: PLATES — free-form plate-origin map "
                    "(%d entries), read by src/film.rs "
                    "load_assets; keys are plate names (data), "
                    "not fields" % (label, len(root)))
            elif kind == "ATMOSPHERES":
                freeform_notes.append(
                    "%s: ATMOSPHERES — tooling data file; "
                    "presets / filmLooks are free-form maps; the "
                    "remaining root keys are the file's embedded "
                    "documentation records: %s"
                    % (label, ", ".join(sorted(
                        k for k in root
                        if k not in ("presets", "filmLooks")))))
            elif kind == "OVERLAY-RECORD":
                freeform_notes.append(
                    "%s: OVERLAY-RECORD — single-purpose record "
                    "file with no loader vocabulary; root keys: "
                    "%s" % (label, ", ".join(sorted(root))))
            else:
                freeform_notes.append(
                    "%s: UNCLASSIFIED — matches no known shape; "
                    "root keys: %s"
                    % (label, ", ".join(sorted(root))
                       if isinstance(root, dict)
                       else type(root).__name__))
    # Render findings grouped by the check that owns the file kind.
    kind_check = {"SCENE": 2, "FILM-DATA": 3, "MANIFEST": 4}
    for (label, pattern, key, ctx), (n, first, _c) in \
            sorted(find.unknown.items()):
        no = kind_check.get(kinds.get(label), 2)
        line = ("%s: %s — unknown key '%s' in %s object "
                "(%d occurrence(s); first at %s)"
                % (label, pattern, key, ctx, n, first))
        per_check_fail[no].append(line)

    print("check_unknown_fields.py — %d file(s)" % len(docs))
    for label, _root in docs:
        print("    %s: %s" % (label, kinds[label]))
    n_pass = 0
    total_checks = 5
    # check 1 passes by construction here (input errors exited 2).
    print("\n[1 %s] PASS — %s" % (CHECK_NAMES[1], CHECK_TITLES[1]))
    n_pass += 1
    for no in (2, 3, 4):
        lines = sorted(per_check_fail[no])
        ok = not lines
        n_pass += 1 if ok else 0
        print("\n[%d %s] %s — %s"
              % (no, CHECK_NAMES[no], "PASS" if ok else "FAIL",
                 CHECK_TITLES[no]))
        for line in lines:
            print("    " + line)
    print("\n[5 %s] PASS — %s" % (CHECK_NAMES[5], CHECK_TITLES[5]))
    n_pass += 1
    for (label, pattern, key, ctx), (n, first, cite) in \
            sorted(find.tolerated.items()):
        print("    %s: %s — tolerated key '%s' (%s) x%d — %s"
              % (label, pattern, key, ctx, n, cite))
    for note in freeform_notes:
        print("    " + note)

    n_unknown = sum(v[0] for v in find.unknown.values())
    verdict = "PASS" if not find.unknown else "FAIL"
    print("\nOVERALL: %s (%d/%d checks passed, %d unknown-key "
          "occurrence(s) in %d distinct path(s), %d tolerated "
          "occurrence(s))"
          % (verdict, n_pass, total_checks, n_unknown,
             len(find.unknown),
             sum(v[0] for v in find.tolerated.values())))
    return 0 if not find.unknown else 1


if __name__ == "__main__":
    sys.exit(main())
