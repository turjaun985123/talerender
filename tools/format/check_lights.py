#!/usr/bin/env python3
"""check_lights.py — validation of scene-format light rigs and grades (W26).

Makes mechanical the format-v2 laws that Wave 1's bugs lived under
(docs/scene-format.md §14.2, §14.5): a `saturation` grade key the
loader silently ignored, and `grade.time` drifting off 0 in seven
scene files — both caught by hand. This gate catches them by name.

Checks (numbered like tools/film/validate_film.py):

  1. parse         every file parses as a JSON object and every
                   number under `lights` and `post.grade` is finite.
                   Python's json accepts the literals NaN / Infinity /
                   -Infinity, which strict JSON (and the Rust loader)
                   does not intend — a non-finite value FAILs here,
                   listed by path.
  2. grade keys    `post.grade` keys must be a subset of the keys
                   src/scenefile.rs's GradeCfg actually reads —
                   {vignette, grain, warm, sat, exposure, time}. The
                   §14.5 addendum names sat / warm / exposure / time
                   canonical (exposure feeds the OutputPass uniform;
                   `saturation` is NOT a key). vignette and grain are
                   in the set because the loader consumes them —
                   failing them would false-fail every real scene.
                   Anything outside the loader's set is silently
                   ignored at render time, so it FAILs BY NAME.
  3. grade time    R3 (§14.5): the GRADE grain time is pinned to 0
                   for the whole film — no source path ever sets it.
                   `post.grade.time`, when present, must be exactly
                   0 (absent is fine: the loader defaults it to 0).
  4. atmosphere    R5 (§14.5): where a file declares an atmosphere —
                   top-level `atmosphere` or `shot.atmosphere` — the
                   value must be a string naming a PRESET from
                   scenes/atmospheres.json `presets` (dawn / dusk /
                   fog / golden / night / noon / storm). An inline
                   rig object FAILs (the driver resolves set x
                   preset; a baked rig is the pre-R5 form), and so
                   does a `filmLooks` key (bk-noon-shop, jg-noon,
                   ...) — that table is a derived validation index,
                   not the scene-file vocabulary. The corpus field
                   `atmospherePreset` is deliberately NOT a naming
                   field here: in the film scenes it is a record of
                   resolved preset values (sky colours, cloudTint,
                   tint — scenes/film/ASSUMPTIONS.md), alongside a
                   proper `atmosphere` preset name; no loader reads
                   it, and R5 constrains the naming fields.
  5. light rig     §14.2 as parsed by src/scenefile.rs: the rig's
                   vocabulary is closed — lights keys subset of
                   {key, rim, hemisphere, points, envIntensity};
                   key {color, intensity, position, target, shadow,
                   shadowBox, far, bias, normalBias}; rim {color,
                   intensity, position}; hemisphere {sky, ground,
                   intensity}; point {color, intensity, position,
                   distance, decay, channel}. Unknown keys are the
                   same silent-ignore class as check 2 and FAIL by
                   name. Values: intensities / envIntensity /
                   distance / decay / normalBias are MULTIPLIERS and
                   must be numbers >= 0 (negative FAILs); shadowBox
                   and far must be > 0; bias may be negative (the
                   loader default is -0.0004) and is only required
                   finite; position / target are vec3 of finite
                   numbers. COLOURS: in this format a light colour
                   is a hex STRING (scenefile.rs KeyCfg.color is a
                   String, linearised by hex_lin) — `color`, `sky`
                   and `ground` must match #RGB / #RRGGBB /
                   #RRGGBBAA. If a colour is given as an ARRAY of
                   numbers instead, it can only be raw components,
                   and then every channel must lie in [0, 1] — a
                   channel outside [0, 1] is not a colour (the
                   multiplier role belongs to `intensity`, a
                   separate field). That distinction is the format's
                   own, not a judgement call.
  6. fire basis    R1 (§14.5) — WARNING class only, never a FAIL.
                   The fire point light records its RENDERED basis
                   4.6 (fireLevel overwrites the intensity every
                   frame as (4.6 + 0.7 sin 17t + 0.5 sin 7.3t) x
                   level); the constructor's 5.0 is never rendered.
                   A fire light is identified with high confidence
                   when a point carries channel.kind == "fireLevel"
                   (tier A: expect intensity exactly 4.6), or with
                   medium confidence when colour #FF9A3C, distance 7
                   and decay 1.6 coincide and the position matches
                   a declared fire pit (shot.fire.pit / fire.pit)
                   (tier B: only an intensity of exactly 5.0 is
                   flagged). Any other intensity on a tier-B point
                   is NOT flagged: a file may legitimately record an
                   evaluated instant — s06's fire point carries
                   0.6333... with the fire dying and no channel
                   marker — so suspicion can never be proof. That
                   is why this whole check is warnings: 5.0-basis
                   suspicion, reported with its tier.

Usage: python3 tools/format/check_lights.py FILE [FILE ...]
       python3 tools/format/check_lights.py --film DIR
         (DIR holds film.json; every manifest segment scene is
         checked — segment paths are repo-root-relative, resolved
         against DIR's repo root, then DIR itself)
Options: --atmospheres PATH  atmospheres.json to take the preset
         vocabulary from (default: scenes/atmospheres.json at the
         repo root this tool lives in).
Exit code: 0 = all files PASS (warnings allowed), 1 = any check
FAIL, 2 = input error (no files, a file unreadable / not a JSON
object, the film manifest or atmospheres file unreadable — one-line
ERROR on stderr, no report). Output is deterministic (command-line
file order, then manifest order in --film mode; findings sorted by
file then path within each check).
"""

import argparse
import json
import math
import os
import re
import sys

GRADE_KEYS = {"vignette", "grain", "warm", "sat", "exposure", "time"}
LIGHTS_KEYS = {"key", "rim", "hemisphere", "points", "envIntensity"}
KEY_KEYS = {"color", "intensity", "position", "target", "shadow",
            "shadowBox", "far", "bias", "normalBias"}
RIM_KEYS = {"color", "intensity", "position"}
HEMI_KEYS = {"sky", "ground", "intensity"}
POINT_KEYS = {"color", "intensity", "position", "distance", "decay",
              "channel"}
HEX_RE = re.compile(r"^#(?:[0-9A-Fa-f]{3}|[0-9A-Fa-f]{6}|[0-9A-Fa-f]{8})$")
FIRE_BASIS = 4.6
FIRE_CONSTRUCTOR = 5.0
FIRE_COLOR = "#FF9A3C"
FIRE_DISTANCE = 7.0
FIRE_DECAY = 1.6


class InputError(Exception):
    """Bad input (unreadable / invalid / wrong-shaped files).

    Distinct from a check FAIL: the run cannot start, so main()
    reports one named line on stderr and exits 2 instead of
    printing a validation report.
    """


def load_json_input(path, what):
    try:
        with open(path) as f:
            return json.load(f)
    except OSError as e:
        raise InputError("%s: cannot read %s (%s)"
                         % (what, path, e.strerror or e))
    except ValueError as e:  # JSONDecodeError + UnicodeDecodeError
        raise InputError("%s: invalid JSON in %s (%s)" % (what, path, e))


def require(cond, msg):
    if not cond:
        raise InputError(msg)


def is_number(v):
    return isinstance(v, (int, float)) and not isinstance(v, bool)


def numbers(obj, path=""):
    """Yield (path, value) for every JSON number under obj (bools excluded)."""
    if isinstance(obj, dict):
        for k in obj:
            yield from numbers(obj[k], path + "." + k if path else k)
    elif isinstance(obj, list):
        for i, v in enumerate(obj):
            yield from numbers(v, "%s[%d]" % (path, i))
    elif isinstance(obj, (int, float)) and not isinstance(obj, bool):
        yield path, obj


def grade_of(scene):
    post = scene.get("post")
    if isinstance(post, dict):
        g = post.get("grade")
        if isinstance(g, dict):
            return g
    return None


def fire_pits(scene):
    """Declared fire pit positions: shot.fire.pit, else top-level fire.pit."""
    pits = []
    for holder in (scene.get("shot"), scene):
        if isinstance(holder, dict):
            fire = holder.get("fire")
            if isinstance(fire, dict) and isinstance(fire.get("pit"), list):
                pits.append(fire["pit"])
    return pits


def check_file(label, scene, presets, film_looks):
    """Run all six checks on one parsed scene.

    Returns (failures, warnings): failures is a list of
    (check_no, message); warnings likewise (check 6 only).
    """
    fails = []
    warns = []
    lights = scene.get("lights")
    grade = grade_of(scene)

    # ---- 1 parse / finite -------------------------------------------------
    for block_name, block in (("lights", lights), ("post.grade", grade)):
        if block is None:
            continue
        for path, v in numbers(block, block_name):
            if not math.isfinite(v):
                fails.append((1, "%s: %s is %r — not a finite number "
                                 "(NaN/Infinity are not valid here)"
                                 % (label, path, v)))

    # ---- 2 grade keys ------------------------------------------------------
    if grade is not None:
        for k in sorted(grade):
            if k not in GRADE_KEYS:
                hint = " (the canonical key is 'sat')" if k == "saturation" else ""
                fails.append((2, "%s: post.grade key %r is not read by the "
                                 "loader and is silently ignored%s — known "
                                 "keys: %s"
                                 % (label, k, hint,
                                    ", ".join(sorted(GRADE_KEYS)))))

    # ---- 3 grade time (R3) -------------------------------------------------
    if grade is not None and "time" in grade:
        t = grade["time"]
        if not is_number(t) or t != 0:
            fails.append((3, "%s: post.grade.time is %r — R3 pins the GRADE "
                             "grain time to 0 for the whole film"
                             % (label, t)))

    # ---- 4 atmosphere (R5) -------------------------------------------------
    declared = []
    if "atmosphere" in scene:
        declared.append(("atmosphere", scene["atmosphere"]))
    shot = scene.get("shot")
    if isinstance(shot, dict) and "atmosphere" in shot:
        declared.append(("shot.atmosphere", shot["atmosphere"]))
    for field, v in declared:
        if isinstance(v, dict):
            fails.append((4, "%s: %s is an inline rig object — R5: scene "
                             "files name an atmosphere PRESET; the driver "
                             "resolves set x preset" % (label, field)))
        elif not isinstance(v, str):
            fails.append((4, "%s: %s is %r — must be a preset name string"
                             % (label, field, v)))
        elif v in film_looks:
            fails.append((4, "%s: %s names filmLooks key %r — that table is "
                             "a derived (set x preset) index, not the "
                             "scene-file vocabulary (R5); name a preset "
                             "instead" % (label, field, v)))
        elif v not in presets:
            fails.append((4, "%s: %s names %r — not an atmosphere preset "
                             "(presets: %s)"
                             % (label, field, v, ", ".join(sorted(presets)))))

    # ---- 5 light rig --------------------------------------------------------
    if lights is not None:
        if not isinstance(lights, dict):
            fails.append((5, "%s: lights is not an object" % label))
        else:
            for k in sorted(lights):
                if k not in LIGHTS_KEYS:
                    fails.append((5, "%s: lights key %r is not read by the "
                                     "loader and is silently ignored — known "
                                     "keys: %s"
                                     % (label, k, ", ".join(sorted(LIGHTS_KEYS)))))

            def num_field(obj, field, path, lo=None, lo_open=False):
                if field not in obj:
                    return
                v = obj[field]
                if not is_number(v):
                    fails.append((5, "%s: %s is %r — must be a number"
                                     % (label, path, v)))
                elif lo is not None and (v <= lo if lo_open else v < lo):
                    fails.append((5, "%s: %s is %r — must be %s %s"
                                     % (label, path, v,
                                        ">" if lo_open else ">=", lo)))

            def colour_field(obj, field, path):
                if field not in obj:
                    return
                v = obj[field]
                if isinstance(v, str):
                    if not HEX_RE.match(v):
                        fails.append((5, "%s: %s is %r — light colours are "
                                         "hex strings (#RRGGBB)"
                                         % (label, path, v)))
                elif isinstance(v, list):
                    for i, c in enumerate(v):
                        if not is_number(c):
                            fails.append((5, "%s: %s[%d] is %r — colour "
                                             "channels must be numbers"
                                             % (label, path, i, c)))
                        elif not (0.0 <= c <= 1.0):
                            fails.append((5, "%s: %s[%d] is %r — outside "
                                             "[0, 1]; an array colour is raw "
                                             "components, and the multiplier "
                                             "role belongs to 'intensity'"
                                             % (label, path, i, c)))
                else:
                    fails.append((5, "%s: %s is %r — light colours are hex "
                                     "strings in this format"
                                     % (label, path, v)))

            def vec3_field(obj, field, path):
                if field not in obj:
                    return
                v = obj[field]
                if not (isinstance(v, list) and len(v) == 3
                        and all(is_number(c) for c in v)):
                    fails.append((5, "%s: %s is %r — must be a vec3 of "
                                     "numbers" % (label, path, v)))

            def vocab(obj, known, path):
                for k in sorted(obj):
                    if k not in known:
                        fails.append((5, "%s: %s key %r is not read by the "
                                         "loader and is silently ignored — "
                                         "known keys: %s"
                                         % (label, path, k,
                                            ", ".join(sorted(known)))))

            key = lights.get("key")
            if isinstance(key, dict):
                vocab(key, KEY_KEYS, "lights.key")
                colour_field(key, "color", "lights.key.color")
                num_field(key, "intensity", "lights.key.intensity", lo=0)
                vec3_field(key, "position", "lights.key.position")
                vec3_field(key, "target", "lights.key.target")
                num_field(key, "shadowBox", "lights.key.shadowBox",
                          lo=0, lo_open=True)
                num_field(key, "far", "lights.key.far", lo=0, lo_open=True)
                num_field(key, "normalBias", "lights.key.normalBias", lo=0)
            elif key is not None:
                fails.append((5, "%s: lights.key is not an object" % label))
            rim = lights.get("rim")
            if isinstance(rim, dict):
                vocab(rim, RIM_KEYS, "lights.rim")
                colour_field(rim, "color", "lights.rim.color")
                num_field(rim, "intensity", "lights.rim.intensity", lo=0)
                vec3_field(rim, "position", "lights.rim.position")
            elif rim is not None:
                fails.append((5, "%s: lights.rim is not an object" % label))
            hemi = lights.get("hemisphere")
            if isinstance(hemi, dict):
                vocab(hemi, HEMI_KEYS, "lights.hemisphere")
                colour_field(hemi, "sky", "lights.hemisphere.sky")
                colour_field(hemi, "ground", "lights.hemisphere.ground")
                num_field(hemi, "intensity", "lights.hemisphere.intensity",
                          lo=0)
            elif hemi is not None:
                fails.append((5, "%s: lights.hemisphere is not an object"
                                 % label))
            num_field(lights, "envIntensity", "lights.envIntensity", lo=0)
            points = lights.get("points")
            if points is not None:
                if not isinstance(points, list):
                    fails.append((5, "%s: lights.points is not an array"
                                     % label))
                else:
                    for i, p in enumerate(points):
                        pp = "lights.points[%d]" % i
                        if not isinstance(p, dict):
                            fails.append((5, "%s: %s is not an object"
                                             % (label, pp)))
                            continue
                        vocab(p, POINT_KEYS, pp)
                        colour_field(p, "color", pp + ".color")
                        num_field(p, "intensity", pp + ".intensity", lo=0)
                        vec3_field(p, "position", pp + ".position")
                        num_field(p, "distance", pp + ".distance", lo=0)
                        num_field(p, "decay", pp + ".decay", lo=0)

    # ---- 6 fire basis (R1, warnings only) -----------------------------------
    if isinstance(lights, dict) and isinstance(lights.get("points"), list):
        pits = fire_pits(scene)
        for i, p in enumerate(lights["points"]):
            if not isinstance(p, dict) or not is_number(p.get("intensity")):
                continue
            inten = p["intensity"]
            pp = "lights.points[%d]" % i
            chan = p.get("channel")
            if isinstance(chan, dict) and chan.get("kind") == "fireLevel":
                if inten != FIRE_BASIS:
                    warns.append((6, "%s: %s carries channel.kind "
                                     "'fireLevel' but intensity %r != %s — "
                                     "R1: a fire light records its rendered "
                                     "basis 4.6; 5.0 is the constructor value "
                                     "that is never rendered (tier A: explicit "
                                     "channel marker)"
                                     % (label, pp, inten, FIRE_BASIS)))
            elif (isinstance(p.get("color"), str)
                  and p["color"].upper() == FIRE_COLOR
                  and p.get("distance") == FIRE_DISTANCE
                  and p.get("decay") == FIRE_DECAY
                  and any(isinstance(p.get("position"), list)
                          and len(p["position"]) == 3
                          and len(pit) == 3
                          and pit[0] == p["position"][0]
                          and pit[2] == p["position"][2]
                          and p["position"][1] > pit[1]
                          for pit in pits)):
                if inten == FIRE_CONSTRUCTOR:
                    warns.append((6, "%s: %s looks like the fire light "
                                     "(colour %s, distance 7, decay 1.6, at "
                                     "the declared pit) with intensity 5.0 — "
                                     "R1 suspicion: 5.0 is the constructor "
                                     "basis; the rendered basis is 4.6 "
                                     "(tier B: heuristic identification)"
                                     % (label, pp, FIRE_COLOR)))
    return fails, warns


CHECK_NAMES = {
    1: "parse",
    2: "grade keys",
    3: "grade time",
    4: "atmosphere",
    5: "light rig",
    6: "fire basis",
}
CHECK_TITLES = {
    1: "files parse; numbers under lights / post.grade finite",
    2: "post.grade keys subset of the loader's set (unknown named)",
    3: "R3: post.grade.time == 0",
    4: "R5: atmosphere declarations name presets, never inline rigs "
       "or filmLooks keys",
    5: "§14.2 light rig: closed vocabulary, sane ranges, hex colours",
    6: "R1: fire point basis 4.6, not the constructor's 5.0 (warnings)",
}


def resolve_inputs(args):
    """Return a list of (label, path) for every scene to check."""
    items = []
    if args.film:
        manifest = load_json_input(os.path.join(args.film, "film.json"),
                                   "film manifest")
        require(isinstance(manifest, dict)
                and isinstance(manifest.get("segments"), list),
                "film manifest: top level must be an object with a "
                "'segments' array")
        repo_root = os.path.dirname(os.path.dirname(
            os.path.abspath(args.film)))
        for seg in manifest["segments"]:
            require(isinstance(seg, dict)
                    and isinstance(seg.get("scene"), str),
                    "film manifest: every segment needs a 'scene' path")
            rel = seg["scene"]
            path = rel
            if not os.path.exists(path):
                cand = os.path.join(repo_root, rel)
                if os.path.exists(cand):
                    path = cand
                else:
                    cand = os.path.join(args.film,
                                        os.path.basename(rel))
                    if os.path.exists(cand):
                        path = cand
            items.append((rel, path))
    for f in args.files:
        items.append((f, f))
    require(items, "no scene files given (pass FILE... or --film DIR)")
    return items


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("files", nargs="*", help="scene files to check")
    ap.add_argument("--film", metavar="DIR",
                    help="check every segment scene of DIR/film.json")
    ap.add_argument("--atmospheres", metavar="PATH",
                    help="atmospheres.json for the preset vocabulary "
                         "(default: scenes/atmospheres.json at the "
                         "repo root this tool lives in)")
    args = ap.parse_args()
    try:
        atmo_path = args.atmospheres or os.path.join(
            os.path.dirname(os.path.dirname(os.path.dirname(
                os.path.abspath(__file__)))),
            "scenes", "atmospheres.json")
        atmos = load_json_input(atmo_path, "atmospheres")
        require(isinstance(atmos, dict)
                and isinstance(atmos.get("presets"), dict)
                and isinstance(atmos.get("filmLooks"), dict),
                "atmospheres: needs 'presets' and 'filmLooks' objects")
        presets = {k for k in atmos["presets"] if not k.startswith("_")}
        film_looks = {k for k in atmos["filmLooks"]
                      if not k.startswith("_")}
        items = resolve_inputs(args)
        scenes = []
        for label, path in items:
            scene = load_json_input(path, "scene file")
            require(isinstance(scene, dict),
                    "scene file: %s top level must be a JSON object" % path)
            scenes.append((label, scene))
    except InputError as e:
        print("ERROR: check_lights.py: %s" % e, file=sys.stderr)
        return 2

    all_fails = []
    all_warns = []
    for label, scene in scenes:
        f, w = check_file(label, scene, presets, film_looks)
        all_fails.extend(f)
        all_warns.extend(w)

    print("check_lights.py — %d file(s)" % len(scenes))
    n_pass = 0
    for no in sorted(CHECK_NAMES):
        f_here = sorted(m for c, m in all_fails if c == no)
        w_here = sorted(m for c, m in all_warns if c == no)
        ok = not f_here
        n_pass += 1 if ok else 0
        headline = CHECK_TITLES[no]
        if w_here:
            headline += " — %d warning(s)" % len(w_here)
        print("\n[%d %s] %s — %s"
              % (no, CHECK_NAMES[no], "PASS" if ok else "FAIL", headline))
        for line in f_here + w_here:
            print("    " + line)
    verdict = "PASS" if not all_fails else "FAIL"
    tail = " (%d/%d checks passed" % (n_pass, len(CHECK_NAMES))
    if all_warns:
        tail += ", %d warning(s)" % len(all_warns)
    tail += ")"
    print("\nOVERALL: %s%s" % (verdict, tail))
    return 0 if not all_fails else 1


if __name__ == "__main__":
    sys.exit(main())
