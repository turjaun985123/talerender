#!/usr/bin/env python3
"""check_clips.py — validation of scene-format `clips` blocks (W15).

Validates the clip/track data model specced in
docs/integration/FORMAT-EXT-CLIPS-W15.md: named clips carrying
keyframe tracks over the channels the engine actually animates,
plus the instances that bind clips to nodes / camera / lights /
post / fog. This is a DATA gate only — no loader or evaluator in
src/ consumes clips yet; adoption is the lead's step.

Checks (numbered like tools/film/validate_film.py):

  1. structure     every named clip is an object with `duration`
                   (a number > 0) and a non-empty `tracks` array;
                   every track is { target: {node, channel},
                   keys: [...] } with at least one key; every key
                   is an object carrying `t` (a number) and `value`
                   (the field name W11's Key struct and the film's
                   existing object-form keys both use).
  2. channels      every track's target resolves — the reserved
                   targets `camera`, `lights`, `post`, `fog`, or a
                   node `name` present in the file (recursively) —
                   and its channel is in that target's vocabulary
                   (the channels src/scenefile.rs and src/film.rs
                   actually animate; see the spec for the cited
                   table). Key values match the channel kind
                   (scalar / vec3 / colour hex / step string with
                   enum membership). One clip may not carry two
                   tracks for the same target+channel. A track on
                   `post.grade.time` fails by name: R3 pins the
                   GRADE grain time to 0 for the whole film
                   (docs/scene-format.md §14.5), so it is not an
                   animatable channel. A node whose name collides
                   with a reserved target is also a failure (it
                   would be unaddressable).
  3. keys          within a track, key times are strictly
                   increasing (unsorted and duplicate times fail,
                   naming the offending pair), every key time lies
                   in [0, clip duration], every `ease` names a
                   curve the engine has — exactly the spellings
                   W11's `Easing::from_name` accepts (linear /
                   smoother / smootherstep / step / easeOutCubic /
                   eo / easeInOutCubic / eio / easeOutBack / eob) —
                   and a discrete (step) channel's keys must all
                   use ease "step".
  4. references    every instance — a `clip` field on a node
                   (recursively), `camera`, `lights`, `post` or
                   `fog` — names a defined clip; `clipTime`
                   mapping fields are sane (offset a number,
                   rate a number > 0, loop one of clamp / loop /
                   ping-pong); `camera` may not carry both `clip`
                   and `keys`; the clips' `uses` layering graph
                   references only defined clips and is acyclic
                   (self-reference counts); layered clips (a clip
                   plus everything it transitively uses) may not
                   define the same target+channel twice.
  5. f32           every number under `clips` and every `clipTime`
                   block obeys the §11 f32 data law in the
                   validate_film.py classes: a number that
                   round-trips exactly through f32 passes
                   outright; a short decimal (<= 6 significant
                   digits) passes as an authored literal; anything
                   else is a computed value written rounded and
                   FAILs, listed verbatim.

Usage: python3 tools/format/check_clips.py FILE [FILE ...]
Exit code: 0 = all files PASS, 1 = any check FAIL, 2 = input
error (a file is unreadable, invalid JSON, not a JSON object, or
carries neither a `clips` block nor any clip instance — one-line
ERROR on stderr, no report). Output is deterministic (command
line file order, checks in the order above, failures in document
order within each check).
"""

import argparse
import json
import re
import struct
import sys

# ------------------------------------------------------------------
# Channel vocabulary — the channels the engine actually animates.
# Sources: src/scenefile.rs (camera keys §14.4, lights §14.2,
# character state §14.3, post §9), src/film.rs (LightState /
# PostState per-frame evaluation), src/toonfull.rs (Mood / Legs).
# Kinds: "scalar" (number), "vec3" ([n, n, n]), "color" (#RRGGBB),
# "step" (string from an enum; no interpolation).
# ------------------------------------------------------------------

SCALAR, VEC3, COLOR, STEP = "scalar", "vec3", "color", "step"

MOODS = ["neutral", "smug", "happy", "laugh", "shock", "worried",
         "angry", "sad", "sly", "proud", "tired", "dizzy",
         "determined", "embarrassed", "awe", "suspicious", "relieved"]
LEG_STATES = ["stand", "walk", "run", "sit", "jump"]

CAMERA_CHANNELS = {"eye": VEC3, "target": VEC3, "fovY": SCALAR}

LIGHT_CHANNELS = {
    "key.intensity": SCALAR, "key.color": COLOR,
    "key.position": VEC3, "key.target": VEC3,
    "rim.intensity": SCALAR, "rim.color": COLOR, "rim.position": VEC3,
    "hemisphere.intensity": SCALAR, "hemisphere.sky": COLOR,
    "hemisphere.ground": COLOR,
    "envIntensity": SCALAR,
}
POINT_SUBCHANNELS = {"intensity": SCALAR, "color": COLOR,
                     "position": VEC3, "distance": SCALAR,
                     "decay": SCALAR}
POINT_RE = re.compile(r"^points\[(\d+)\]\.(.+)$")

POST_CHANNELS = {
    "bokeh.focus": SCALAR, "bokeh.aperture": SCALAR,
    "bokeh.maxblur": SCALAR,
    "bloom.threshold": SCALAR, "bloom.strength": SCALAR,
    "bloom.radius": SCALAR,
    "grade.sat": SCALAR, "grade.warm": SCALAR,
    "grade.exposure": SCALAR, "grade.vignette": SCALAR,
    "grade.grain": SCALAR,
    # grade.time is deliberately absent — R3 (§14.5).
}

FOG_CHANNELS = {"near": SCALAR, "far": SCALAR, "color": COLOR}

NODE_CHANNELS = {"translate": VEC3, "scale": VEC3,
                 "rotate.x": SCALAR, "rotate.y": SCALAR,
                 "rotate.z": SCALAR}

CHARACTER_CHANNELS = {
    "position": VEC3,
    "armL.0": SCALAR, "armL.1": SCALAR,
    "armR.0": SCALAR, "armR.1": SCALAR,
    "look.0": SCALAR, "look.1": SCALAR,
    "blink": SCALAR, "talk": SCALAR, "phase": SCALAR,
    "lean": SCALAR, "bob": SCALAR, "squash": SCALAR,
    "headTilt": SCALAR, "headTurn": SCALAR, "camPitch": SCALAR,
    "sitOn": SCALAR, "height": SCALAR,
    "mood": (STEP, MOODS), "legs": (STEP, LEG_STATES),
}

RESERVED_TARGETS = ("camera", "lights", "post", "fog")

# Easing names — aligned to W11's evaluator, src/anim.rs
# `Easing::from_name` (landed during this seat): it accepts exactly
# these spellings. Canonical forms are the long names; "smoother"
# and the short "eo"/"eio"/"eob" are evaluator-accepted aliases.
# Curves: smootherstep (film.rs smoother / scenefile smootherstep,
# the default — camK, poseTrack, §14.4 camera keys), linear (the
# one ease name format v2 already spells, §14.4), step (hold —
# poseTrack with moveSecs 0; discrete channels).
EASINGS = ["linear", "smoother", "smootherstep", "step",
           "easeOutCubic", "eo", "easeInOutCubic", "eio",
           "easeOutBack", "eob"]
DEFAULT_EASE = "smoother"

LOOP_MODES = ["clamp", "loop", "ping-pong"]

HEX_RE = re.compile(r"^#[0-9A-Fa-f]{6}$")


class InputError(Exception):
    """Bad input file — the run cannot start (exit 2, no report)."""


def is_number(v):
    return isinstance(v, (int, float)) and not isinstance(v, bool)


def f32(v):
    return struct.unpack("f", struct.pack("f", v))[0]


def sig_digits(v):
    """Significant digits in the shortest round-trip decimal."""
    s = repr(float(v)).lower()
    if "e" in s:
        s = s.split("e")[0]
    s = s.replace("-", "").replace(".", "").lstrip("0")
    return len(s) if s else 1


def kind_of(spec):
    return spec[0] if isinstance(spec, tuple) else spec


def describe_kind(spec):
    k = kind_of(spec)
    if k == SCALAR:
        return "a number"
    if k == VEC3:
        return "a [x, y, z] array of 3 numbers"
    if k == COLOR:
        return "a '#RRGGBB' colour string"
    return "one of: " + ", ".join(spec[1])


def value_ok(spec, v):
    k = kind_of(spec)
    if k == SCALAR:
        return is_number(v)
    if k == VEC3:
        return (isinstance(v, list) and len(v) == 3
                and all(is_number(x) for x in v))
    if k == COLOR:
        return isinstance(v, str) and bool(HEX_RE.match(v))
    return isinstance(v, str) and v in spec[1]


def channel_spec(target_node, channel, node_kinds):
    """(spec, None) if the channel exists for the target, else
    (None, failure message). node_kinds: name -> 'character'|'node'."""
    if target_node == "camera":
        table, what = CAMERA_CHANNELS, "camera"
    elif target_node == "lights":
        m = POINT_RE.match(channel)
        if m:
            sub = m.group(2)
            if sub in POINT_SUBCHANNELS:
                return POINT_SUBCHANNELS[sub], None
            return None, ("unknown channel 'lights.%s' — point-light "
                          "channels are points[i].%s"
                          % (channel, "/".join(sorted(POINT_SUBCHANNELS))))
        table, what = LIGHT_CHANNELS, "lights"
    elif target_node == "post":
        if channel == "grade.time":
            return None, ("channel 'post.grade.time' is not animatable: "
                          "R3 pins the GRADE grain time to 0 for the "
                          "whole film (docs/scene-format.md §14.5 R3)")
        table, what = POST_CHANNELS, "post"
    elif target_node == "fog":
        table, what = FOG_CHANNELS, "fog"
    elif target_node in node_kinds:
        if node_kinds[target_node] == "character":
            table, what = CHARACTER_CHANNELS, "character node '%s'" % target_node
        else:
            table, what = NODE_CHANNELS, "node '%s'" % target_node
    else:
        return None, ("unknown target node '%s' — not a reserved target "
                      "(%s) and no node in this file has that name"
                      % (target_node, ", ".join(RESERVED_TARGETS)))
    if channel in table:
        return table[channel], None
    return None, ("unknown channel '%s.%s' — %s channels are: %s"
                  % (target_node, channel, what,
                     ", ".join(sorted(table))))


def collect_nodes(doc):
    """name -> 'character'|'node' over doc['nodes'] recursively,
    plus name-collision failures for check 2."""
    kinds, collisions = {}, []

    def walk(nodes):
        for n in nodes:
            if not isinstance(n, dict):
                continue
            name = n.get("name")
            if isinstance(name, str):
                if name in RESERVED_TARGETS:
                    collisions.append(
                        "node named '%s' collides with the reserved clip "
                        "target '%s' — rename the node; its tracks would "
                        "be unaddressable" % (name, name))
                elif name not in kinds:
                    kinds[name] = ("character" if n.get("type") == "character"
                                   else "node")
            walk(n.get("children") or [])

    walk(doc.get("nodes") or [])
    return kinds, collisions


def find_instances(doc):
    """All clip instances: (path, clip-name, clipTime-or-None) for
    every dict in the document carrying a string `clip` field,
    excluding the clips' own definitions."""
    found = []

    def walk(o, path, in_clips):
        if isinstance(o, dict):
            in_clips = in_clips or path == "clips"
            if not in_clips and isinstance(o.get("clip"), str):
                found.append((path or "<root>", o["clip"],
                              o.get("clipTime")))
            for k, v in o.items():
                walk(v, "%s.%s" % (path, k) if path else k, in_clips)
        elif isinstance(o, list):
            for i, v in enumerate(o):
                walk(v, "%s[%d]" % (path, i), in_clips)

    walk(doc, "", False)
    return found


def check_file(path):
    try:
        with open(path) as fh:
            doc = json.load(fh)
    except OSError as e:
        raise InputError("cannot read %s (%s)" % (path, e.strerror or e))
    except ValueError as e:
        raise InputError("invalid JSON in %s (%s)" % (path, e))
    if not isinstance(doc, dict):
        raise InputError("%s: top level is not a JSON object" % path)

    clips = doc.get("clips")
    instances = find_instances(doc)
    if clips is None and not instances:
        raise InputError("%s: no 'clips' block and no clip instances — "
                         "nothing to validate" % path)
    if clips is not None and not isinstance(clips, dict):
        raise InputError("%s: 'clips' is not an object" % path)
    clips = clips or {}
    node_kinds, collisions = collect_nodes(doc)

    s_fail, c_fail, k_fail, r_fail, f_fail = [], [], [], [], []
    c_fail.extend(collisions)

    # ---- 1 structure ------------------------------------------------
    durations = {}
    for name in sorted(clips):
        clip = clips[name]
        cpath = "clips.%s" % name
        if not isinstance(clip, dict):
            s_fail.append("%s: clip is not an object" % cpath)
            continue
        dur = clip.get("duration")
        if not is_number(dur):
            s_fail.append("%s.duration: missing or not a number" % cpath)
        elif dur <= 0:
            s_fail.append("%s.duration: must be > 0, got %r" % (cpath, dur))
        else:
            durations[name] = float(dur)
        tracks = clip.get("tracks")
        if not isinstance(tracks, list) or not tracks:
            s_fail.append("%s.tracks: missing, not an array, or empty"
                          % cpath)
            continue
        for ti, track in enumerate(tracks):
            tpath = "%s.tracks[%d]" % (cpath, ti)
            if not isinstance(track, dict):
                s_fail.append("%s: track is not an object" % tpath)
                continue
            tgt = track.get("target")
            if (not isinstance(tgt, dict)
                    or not isinstance(tgt.get("node"), str)
                    or not isinstance(tgt.get("channel"), str)):
                s_fail.append("%s.target: must be an object with string "
                              "'node' and 'channel'" % tpath)
            keys = track.get("keys")
            if not isinstance(keys, list) or not keys:
                s_fail.append("%s.keys: missing, not an array, or empty"
                              % tpath)
                continue
            for ki, key in enumerate(keys):
                kpath = "%s.keys[%d]" % (tpath, ki)
                if not isinstance(key, dict):
                    s_fail.append("%s: key is not an object" % kpath)
                    continue
                if not is_number(key.get("t")):
                    s_fail.append("%s.t: missing or not a number" % kpath)
                if "value" not in key:
                    s_fail.append("%s.value: missing" % kpath)
                if "ease" in key and not isinstance(key["ease"], str):
                    s_fail.append("%s.ease: not a string" % kpath)

    # ---- 2 channels (+ value shapes, duplicate tracks) --------------
    track_channels = {}  # clip name -> {(node, channel): track path}
    for name in sorted(clips):
        clip = clips[name]
        if not isinstance(clip, dict) or not isinstance(clip.get("tracks"), list):
            continue
        seen = {}
        for ti, track in enumerate(clip["tracks"]):
            if not isinstance(track, dict):
                continue
            tpath = "clips.%s.tracks[%d]" % (name, ti)
            tgt = track.get("target") or {}
            node, channel = tgt.get("node"), tgt.get("channel")
            if not isinstance(node, str) or not isinstance(channel, str):
                continue  # already failed in structure
            ident = (node, channel)
            if ident in seen:
                c_fail.append("%s: duplicate track for '%s.%s' (first at "
                              "%s) — one clip, one track per channel"
                              % (tpath, node, channel, seen[ident]))
            seen[ident] = tpath
            spec, err = channel_spec(node, channel, node_kinds)
            if err:
                c_fail.append("%s: %s" % (tpath, err))
                continue
            keys = track.get("keys")
            if not isinstance(keys, list):
                continue
            for ki, key in enumerate(keys):
                if isinstance(key, dict) and "value" in key \
                        and not value_ok(spec, key["value"]):
                    c_fail.append("%s.keys[%d].value: %r is not %s "
                                  "(channel '%s.%s')"
                                  % (tpath, ki, key["value"],
                                     describe_kind(spec), node, channel))
        track_channels[name] = seen

    # ---- 3 keys ------------------------------------------------------
    for name in sorted(clips):
        clip = clips[name]
        if not isinstance(clip, dict) or not isinstance(clip.get("tracks"), list):
            continue
        dur = durations.get(name)
        for ti, track in enumerate(clip["tracks"]):
            if not isinstance(track, dict):
                continue
            tpath = "clips.%s.tracks[%d]" % (name, ti)
            tgt = track.get("target") or {}
            spec, _ = (channel_spec(tgt.get("node"), tgt.get("channel"),
                                    node_kinds)
                       if isinstance(tgt.get("node"), str)
                       and isinstance(tgt.get("channel"), str)
                       else (None, None))
            keys = track.get("keys")
            if not isinstance(keys, list):
                continue
            times = []
            for ki, key in enumerate(keys):
                if not isinstance(key, dict) or not is_number(key.get("t")):
                    times.append(None)
                    continue
                t = float(key["t"])
                times.append(t)
                kpath = "%s.keys[%d]" % (tpath, ki)
                if t < 0:
                    k_fail.append("%s.t: %r is negative — clip-local "
                                  "times start at 0" % (kpath, key["t"]))
                if dur is not None and t > dur:
                    k_fail.append("%s.t: %r is beyond the clip duration "
                                  "%r (clip '%s')" % (kpath, key["t"],
                                                      dur, name))
                ease = key.get("ease", DEFAULT_EASE)
                if isinstance(ease, str) and ease not in EASINGS:
                    k_fail.append("%s.ease: unknown easing %r — the "
                                  "engine's curves are: %s"
                                  % (kpath, ease, ", ".join(EASINGS)))
                if spec is not None and kind_of(spec) == STEP \
                        and ease != "step":
                    k_fail.append("%s.ease: channel '%s.%s' is discrete; "
                                  "its keys must use ease 'step', got %r"
                                  % (kpath, tgt.get("node"),
                                     tgt.get("channel"), ease))
            clean = [(i, t) for i, t in enumerate(times) if t is not None]
            for (i0, t0), (i1, t1) in zip(clean, clean[1:]):
                if t1 == t0:
                    k_fail.append("%s.keys[%d].t: duplicate key time %r "
                                  "(already at keys[%d]) — two values "
                                  "at one instant is ambiguous"
                                  % (tpath, i1, t1, i0))
                elif t1 < t0:
                    k_fail.append("%s.keys[%d].t: key times out of order "
                                  "— %r follows %r (keys[%d]); times "
                                  "must be strictly increasing"
                                  % (tpath, i1, t1, t0, i0))

    # ---- 4 references -------------------------------------------------
    for ipath, cname, ctime in instances:
        if cname not in clips:
            r_fail.append("%s: instance references unknown clip '%s'"
                          % (ipath, cname))
        if ctime is not None:
            if not isinstance(ctime, dict):
                r_fail.append("%s.clipTime: not an object" % ipath)
            else:
                if "offset" in ctime and not is_number(ctime["offset"]):
                    r_fail.append("%s.clipTime.offset: not a number"
                                  % ipath)
                if "rate" in ctime:
                    if not is_number(ctime["rate"]):
                        r_fail.append("%s.clipTime.rate: not a number"
                                      % ipath)
                    elif ctime["rate"] <= 0:
                        r_fail.append("%s.clipTime.rate: must be > 0, "
                                      "got %r" % (ipath, ctime["rate"]))
                if "loop" in ctime and ctime["loop"] not in LOOP_MODES:
                    r_fail.append("%s.clipTime.loop: unknown loop mode "
                                  "%r — modes are: %s"
                                  % (ipath, ctime["loop"],
                                     ", ".join(LOOP_MODES)))
    cam = doc.get("camera")
    if isinstance(cam, dict) and isinstance(cam.get("clip"), str) \
            and "keys" in cam:
        r_fail.append("camera: carries both 'clip' ('%s') and 'keys' — "
                      "one camera, one time source; drop one"
                      % cam["clip"])
    uses = {}
    for name in sorted(clips):
        clip = clips[name]
        if not isinstance(clip, dict):
            continue
        u = clip.get("uses")
        if u is None:
            uses[name] = []
            continue
        if not isinstance(u, list) or not all(isinstance(x, str) for x in u):
            r_fail.append("clips.%s.uses: must be an array of clip names"
                          % name)
            uses[name] = []
            continue
        uses[name] = u
        for target in u:
            if target not in clips:
                r_fail.append("clips.%s.uses: unknown clip '%s'"
                              % (name, target))
    # cycle detection over the uses graph (iterative DFS, white/grey)
    WHITE, GREY, BLACK = 0, 1, 2
    colour = {n: WHITE for n in uses}

    def dfs(start):
        stack = [(start, iter(uses.get(start, [])))]
        colour[start] = GREY
        path = [start]
        while stack:
            node, it = stack[-1]
            advanced = False
            for nxt in it:
                if nxt not in uses:
                    continue  # unknown — already failed above
                if colour[nxt] == GREY:
                    cyc = path[path.index(nxt):] + [nxt]
                    r_fail.append("clips: reference cycle: %s"
                                  % " -> ".join(cyc))
                elif colour[nxt] == WHITE:
                    colour[nxt] = GREY
                    path.append(nxt)
                    stack.append((nxt, iter(uses.get(nxt, []))))
                    advanced = True
                    break
            if not advanced:
                colour[node] = BLACK
                stack.pop()
                path.pop()

    for n in sorted(uses):
        if colour[n] == WHITE:
            dfs(n)
    # layered-channel conflicts (only over acyclic ancestry)
    def layered(name, seen_clips):
        out = {}
        if name in seen_clips or name not in clips:
            return out
        seen_clips = seen_clips | {name}
        for ident, tpath in track_channels.get(name, {}).items():
            out.setdefault(ident, (name, tpath))
        for u in uses.get(name, []):
            for ident, src in layered(u, seen_clips).items():
                if ident in out and out[ident][0] != src[0]:
                    r_fail.append(
                        "clips.%s: channel '%s.%s' is defined by both "
                        "'%s' (%s) and layered clip '%s' (%s) — layered "
                        "clips may not fight over one channel"
                        % (name, ident[0], ident[1], out[ident][0],
                           out[ident][1], src[0], src[1]))
                out.setdefault(ident, src)
        return out

    for n in sorted(clips):
        layered(n, set())

    # ---- 5 f32 ---------------------------------------------------------
    def walk_numbers(o, path):
        if isinstance(o, bool):
            return
        if is_number(o):
            if f32(o) != float(o) and sig_digits(o) > 6:
                f_fail.append("%s: %r is neither an exact f32 value "
                              "nor a short authored decimal (<= 6 "
                              "significant digits) — a computed value "
                              "written rounded violates the §11 f32 "
                              "data law" % (path, o))
        elif isinstance(o, dict):
            for k, v in o.items():
                walk_numbers(v, "%s.%s" % (path, k))
        elif isinstance(o, list):
            for i, v in enumerate(o):
                walk_numbers(v, "%s[%d]" % (path, i))

    if isinstance(doc.get("clips"), dict):
        walk_numbers(doc["clips"], "clips")

    def walk_cliptimes(o, path, in_clips):
        if isinstance(o, dict):
            in_clips = in_clips or path == "clips"
            if not in_clips and isinstance(o.get("clipTime"), dict):
                walk_numbers(o["clipTime"], "%s.clipTime" % (path or "<root>"))
            for k, v in o.items():
                walk_cliptimes(v, "%s.%s" % (path, k) if path else k,
                               in_clips)
        elif isinstance(o, list):
            for i, v in enumerate(o):
                walk_cliptimes(v, "%s[%d]" % (path, i), in_clips)

    walk_cliptimes(doc, "", False)

    # ---- report ---------------------------------------------------------
    n_tracks = sum(len(c["tracks"]) for c in clips.values()
                   if isinstance(c, dict) and isinstance(c.get("tracks"), list))
    print("check_clips.py — %s (%d clips, %d tracks, %d instances)"
          % (path, len(clips), n_tracks, len(instances)))
    results = [
        ("1 structure", s_fail),
        ("2 channels", c_fail),
        ("3 keys", k_fail),
        ("4 references", r_fail),
        ("5 f32", f_fail),
    ]
    all_ok = True
    for name, failures in results:
        ok = not failures
        all_ok = all_ok and ok
        print("\n[%s] %s — %s"
              % (name, "PASS" if ok else "FAIL",
                 "no failures" if ok else "%d failure(s)" % len(failures)))
        for line in failures:
            print("    " + line)
    print("\nOVERALL: %s (%d/%d checks passed)"
          % ("PASS" if all_ok else "FAIL",
             sum(1 for _, f in results if not f), len(results)))
    return 0 if all_ok else 1


def main():
    ap = argparse.ArgumentParser(
        description="Validate scene-format clips blocks (W15 spec).")
    ap.add_argument("files", nargs="+", help="scene JSON files to check")
    args = ap.parse_args()
    rc = 0
    for path in args.files:
        if rc:
            print()
        rc = check_file(path) or rc
    return rc


if __name__ == "__main__":
    try:
        sys.exit(main())
    except InputError as e:
        print("ERROR: %s" % e, file=sys.stderr)
        sys.exit(2)
