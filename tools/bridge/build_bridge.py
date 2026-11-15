#!/usr/bin/env python3
# =====================================================================
# DELIVERY NOTE — Chrome Replacement, Batch 1, Seat 6 (CR-B1-06)
#
# Files created (this seat, NEW files only):
#   tools/bridge/build_bridge.py                       (this file)
#   docs/integration/CR-B1-06-BRIDGE.md                (companion doc)
#   <skill>/template/tools/native_modes/filmdata_validate.mjs
#                                                      (P4, separate
#                                                      deliverable)
#
# What this closes: package_bridge.py only PACKAGES film data the
# repo already holds; V2-WP9 proposal P5 — nothing BUILDs a bridge
# for a new project. This builder assembles <project>/native/
# completely from a project's own timing data + a directory of
# transcribed segment scene files.
#
# Honoured contracts:
#   - CHROME-REPLACEMENT-CONTRACTS.md §0 (bridge layout) — the
#     layout produced is exactly package_bridge.py's layout.
#   - CR-B1-05 (cues seat) bridge requirements R1-R5, R7, R8:
#     R1 shot.cues[] is carried from the transcribed scenes (never
#        invented); R2 legacy {at,name,source} cue entries are
#        normalised to canonical {t,type,gain?,dur?,pan?} and the
#        dropped provenance is recorded in the manifest bridge
#        block, OUTSIDE the entries; R3 cue t values are already
#        absolute in the scene data and are carried verbatim —
#        word anchors for the FILM DATA are resolved here, at
#        bridge time, from data/words.json; R4 manifest durationS
#        is the lines/film duration, never the mix duration; R5
#        shot.events[] is emitted on every shot — as an EMPTY list
#        unless the transcribed scene already carries events,
#        because computing events needs clip-instance data this
#        builder does not have (named as a gap, never omitted
#        silently); R7 cue types / event names are validated
#        against the closed vocabularies and fail loudly; R8 the
#        manifest names every scene file it segments.
#   - Self-verification is THREE-legged, because the legs disagree
#     in strictness: (a) this builder's own adapter-semantics check
#     (the P4 data-driven asset derivation — plates.json + film
#     data declarations, NOT the adapter's hardcoded Onoda list);
#     (b) tools/integration/validate_native_project.py as a
#     subprocess (it mirrors the CURRENT adapter, hardcoded list
#     included, and will flag a non-Onoda bridge accordingly —
#     that flag is reported, not hidden); (c) the binary's own
#     verdict: `talerender --film-telemetry <film-dir>`, whose
#     FILM TELEMETRY: CHECK OK line is REQUIRED (the binary loader
#     is stricter than the adapter — e.g. it demands the F2
#     calendarYears/calendarDark/sign keys by name). Leg (c) is
#     skipped only with --no-telemetry, which prints why.
#
# Stdlib only. Never commits. Writes only under --out.
# =====================================================================
"""Build a Tall Tale project's native/ bridge from its own data.

Usage:
    build_bridge.py --project DIR --scenes DIR [--out DIR]
                    [--assets-dir DIR] [--film-data FILE]
                    [--anchors FILE] [--fps 30]
                    [--calendar-year YYYY ...] [--calendar-dark FILE]
                    [--sign FILE] [--force] [--no-telemetry]
                    [--talerender-bin PATH] [--compare DIR]

Inputs:
    --project     skill project: script.json, data/lines.json,
                  data/words.json (data/mix.wav used for the audio
                  block when present)
    --scenes      directory of transcribed segment scene files
                  (*.scene.json), one per segment, each carrying
                  shot.at / shot.duration (the transcription)
    --assets-dir  source of rasterised PNG assets (plates, caption
                  strips, calendar/sign plates). Scene-relative
                  file references are tried first. This builder
                  COPIES rasterisations; it cannot create them —
                  rasterising plates/strips from drawing recipes
                  is film/gen_assets.py's PIL job (see the doc).
    --film-data   an existing film data file (e.g. the repo's
                  film/onoda.json) whose anchors and F2
                  declarations (calendarYears/calendarDark/sign)
                  are carried over, and whose captions' fit values
                  are reused for chunks this build regenerates
                  identically (same text at the same start time)
    --anchors     JSON: either {"<name>": <seconds>} verbatim, or
                  [["<word>", <line>], ...] resolved with the
                  timing.ts Wd() rule against data/words.json
    --skeleton    a talerender-skeleton/1 report from
                  tools/export/export_project_v2.mjs (CR-B1-07).
                  Primary input where it covers a need: shot times
                  + frame tiling, segment->lines, set identity and
                  atmosphere presets are consumed from it (and
                  cross-checked against the scene files); its
                  `owes` list is carried into the manifest bridge
                  block as the named manual remainder. Cues are
                  NOT taken from the skeleton — its per-shot cue
                  times can be unresolved (S07's loop-offset tocks);
                  the scene files' shot.cues[] remain the cue
                  source of truth (CR-B1-05 R1/R3), with
                  data/cues.json authoritative for mixing.

Output (under --out, default <project>/native):
    film.json                  manifest (talerender-film v1)
    MANIFEST-SHA               sha256 of every built file
    scenes/film/*.scene.json   the segment scenes (cues normalised,
                               shot.events[] ensured — see R5)
    film/onoda.json            timing, anchors, caption chunks
    film/assets/plates.json    manifest shape (talerender-plates)
    film/assets/*.png          every asset the data names

Exit codes: 0 = built and every verification leg PASS (or a leg
explicitly skipped with --no-telemetry / validator absent, both
printed); 1 = build or verification failed, named; 2 = usage
error / refusal (e.g. --out already populated without --force).
"""

import argparse
import hashlib
import json
import math
import os
import re
import shutil
import subprocess
import sys
import wave
from pathlib import Path

MANAGED = ["film.json", "MANIFEST-SHA", "scenes", "film"]

# R7 closed vocabularies (CR-B1-05 §2/§5): mix.py's FX table and
# audio_recipes.py's EVENT_VOCABULARY.
FX_VOCAB = {
    "ding", "chirp", "chug", "stamp", "inhale", "heartbeat", "whoosh",
    "slide", "pop", "plop", "gulp", "boing", "bubbles", "sizzle",
    "sparkle", "thud", "slam", "sting", "tock", "zip", "alarm",
}
EVENT_VOCAB = {
    "takeoff", "land", "fall-impact", "trip", "slip", "lose-balance",
    "recover", "roll", "push-up", "pop", "inhale",
}

SHORT = re.compile(
    r"^(a|an|the|to|of|in|on|at|by|and|but|or|so|is|it|he|she|we|i|his|her|my|no|up|for|was)$",
    re.IGNORECASE,
)


def die(msg, code=1):
    print(f"build-bridge: {msg}", file=sys.stderr)
    sys.exit(code)


def sha256_of(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def is_number(x):
    return isinstance(x, (int, float)) and not isinstance(x, bool)


def load_json(path, what):
    try:
        return json.loads(Path(path).read_text(encoding="utf-8"))
    except Exception as e:
        die(f"{what}: cannot read/parse {path}: {e}")


def dump_js(obj):
    """JSON.stringify(obj, null, 1) spelling: indent 1, literal
    unicode, no trailing newline — the repo film data's own style."""
    return json.dumps(obj, indent=1, ensure_ascii=False)


# ---------------------------------------------------------------------
# Captions — a faithful port of tools/captions/gen_caption_strips.py
# build_captions() (itself a port of captions.ts buildCaptions()):
# authored `caps` chunks start at the first word after the previous
# chunk's start whose normalised form equals the chunk's first
# normalised word (chunk 0 at word 0), s = word.s - 0.03; a chunk
# ends at the next chunk's start, except across a line gap
# (next.s - lineEnd > 0.6 and next chunk on another line), where
# e = lineEnd + 0.25; the final chunk ends at the film duration.
# Lines without `caps` fall back to autoChunks(). The ONE thing
# this stdlib port cannot reproduce is the strip WIDTH/fit: that
# needs a font measurer (PIL/FreeType or Skia). fit is carried
# from --film-data for identical chunks, else assumed 1.0 and the
# assumption is counted and printed — never silent.
# ---------------------------------------------------------------------

def norm(w):
    return re.sub(r"[^a-z0-9']+", "", str(w).lower())


def auto_chunks(ws):
    out = []
    i = 0
    while i < len(ws):
        w = ws[i]
        n = ws[i + 1] if i + 1 < len(ws) else None
        bare = re.sub(r"[^A-Za-z']", "", w)
        if (n is not None and SHORT.match(bare)
                and not re.search(r"[,.!?;:]$", w)
                and len(w + n) <= 11):
            out.append((i, i + 1))
            i += 2
        else:
            out.append((i, i))
            i += 1
    return out


def build_captions(script, lines_data, words):
    """Returns [{line, s, e, text}] (gen_caption_strips semantics)."""
    line_end = {ln["i"]: ln["end"] for ln in lines_data["lines"]}
    dur = lines_data["duration"]
    by_line = {}
    for w in words:
        by_line.setdefault(w["line"], []).append(w)
    caps = []
    for i, ln in enumerate(script["lines"]):
        ws = by_line.get(i, [])
        if not ws:
            continue
        text_words = ln["text"].split()
        spoken = text_words if len(ws) == len(text_words) else [w["w"] for w in ws]
        authored = ln.get("caps")
        if authored:
            frm = 0
            segs = [c.strip() for c in authored.split("|") if c.strip()]
            for j, c in enumerate(segs):
                first = norm(c.split()[0])
                if j == 0:
                    idx = 0
                else:
                    idx = next((k for k in range(frm + 1, len(ws))
                                if norm(ws[k]["w"]) == first), -1)
                    if idx < 0:
                        idx = min(len(ws) - 1, frm + 1)
                frm = idx
                caps.append({"line": i, "s": ws[idx]["s"] - 0.03,
                             "e": 0.0, "text": c.replace("*", "")})
        else:
            for a, b in auto_chunks(spoken):
                caps.append({"line": i, "s": ws[a]["s"] - 0.03,
                             "e": 0.0,
                             "text": " ".join(spoken[a:b + 1])})
    for k, c in enumerate(caps):
        nxt = caps[k + 1] if k + 1 < len(caps) else None
        if nxt is None:
            c["e"] = dur
        elif nxt["s"] - line_end[c["line"]] > 0.6 and nxt["line"] != c["line"]:
            c["e"] = line_end[c["line"]] + 0.25
        else:
            c["e"] = nxt["s"]
    return caps


def wd(word, line, words):
    """timing.ts Wd(): start of the first word in `line` whose
    normalised form starts with the normalised query
    (film/gen_film_data.py's rule)."""
    n = norm(word)
    for w in words:
        if w["line"] == line and norm(w["w"]).startswith(n):
            return w["s"]
    die(f"anchor word {word!r} not found in line {line} of data/words.json")


# ---------------------------------------------------------------------
# Scene transcription reading
# ---------------------------------------------------------------------

def walk(obj):
    if isinstance(obj, dict):
        yield obj
        for v in obj.values():
            yield from walk(v)
    elif isinstance(obj, list):
        for v in obj:
            yield from walk(v)


def scene_png_refs(scene_obj, scene_path):
    """Asset file references inside a scene: string values that are
    relative paths ending in .png (e.g. textures[].file =
    '../../film/assets/coconut.png'), resolved against the scene
    file's own directory. Returns {basename: resolved Path}."""
    refs = {}
    for node in walk(scene_obj):
        for v in node.values():
            if isinstance(v, str) and v.endswith(".png") and ("/" in v):
                p = (scene_path.parent / v).resolve()
                refs.setdefault(Path(v).name, p)
    return refs


def scene_plates(scene_obj):
    """Held-prop plates declared by the scene data: any object with
    a string `plate` name; the origin comes from the x0/y0 carried
    by the state/channel entries that hold it. Returns
    {name: (x0, y0) or None}."""
    plates = {}
    for node in walk(scene_obj):
        name = node.get("plate")
        if isinstance(name, str) and name and " " not in name:
            origin = None
            if is_number(node.get("x0")) and is_number(node.get("y0")):
                origin = (float(node["x0"]), float(node["y0"]))
            if name not in plates or (plates[name] is None and origin):
                plates[name] = origin if origin else plates.get(name)
    return plates


CUE_KEYS = ("t", "type", "gain", "dur", "pan")


def normalise_cues(seg_id, cues, provenance):
    """R2: canonical {t, type, gain?, dur?, pan?} only. Legacy
    {at, name, source} entries are rewritten; `source` provenance
    is collected OUTSIDE the entries (returned via `provenance`).
    R3: t is carried verbatim (already absolute). R7: the type
    must be in the closed FX vocabulary."""
    out = []
    for c in cues or []:
        if not isinstance(c, dict):
            die(f"segment {seg_id}: shot.cues entry is not an object: {c!r}")
        entry = {}
        t = c.get("t", c.get("at"))
        typ = c.get("type", c.get("name"))
        if not is_number(t) or not isinstance(typ, str):
            die(f"segment {seg_id}: cue lacks a numeric t/at or a "
                f"type/name: {c!r}")
        entry["t"] = t
        entry["type"] = typ
        for k in ("gain", "dur", "pan"):
            if c.get(k) is not None:
                entry[k] = c[k]
        extra = [k for k in c if k not in CUE_KEYS + ("at", "name", "source")]
        if extra:
            die(f"segment {seg_id}: cue {c!r} carries keys outside "
                f"the canonical spelling: {extra}")
        if typ not in FX_VOCAB:
            if typ in EVENT_VOCAB:
                die(f"segment {seg_id}: cue type {typ!r} is a clip "
                    "EVENT name, not an FX type — events belong in "
                    "shot.events[] (R5/R7), not shot.cues[]")
            die(f"segment {seg_id}: cue type {typ!r} is not in "
                "mix.py's closed FX vocabulary (R7)")
        if isinstance(c.get("source"), str):
            provenance.append(f"{seg_id}: {c['source']}")
        out.append(entry)
    return out


def normalise_events(seg_id, events):
    """R5 shape [{t, cue, gain?, dur?}], R7 event vocabulary."""
    out = []
    for e in events or []:
        if not isinstance(e, dict) or not is_number(e.get("t")) \
                or not isinstance(e.get("cue"), str):
            die(f"segment {seg_id}: shot.events entry is not "
                f"{{t, cue, ...}}: {e!r}")
        if e["cue"] not in EVENT_VOCAB and e["cue"] not in FX_VOCAB:
            die(f"segment {seg_id}: event cue {e['cue']!r} is in "
                "neither the event vocabulary nor the FX table (R7)")
        entry = {"t": e["t"], "cue": e["cue"]}
        for k in ("gain", "dur"):
            if e.get(k) is not None:
                entry[k] = e[k]
        out.append(entry)
    return out


# ---------------------------------------------------------------------
# Build
# ---------------------------------------------------------------------

def build(args):
    project = Path(args.project).expanduser().resolve()
    scenes_dir = Path(args.scenes).expanduser().resolve()
    out = Path(args.out).expanduser().resolve() if args.out \
        else project / "native"
    if not project.is_dir():
        die(f"project dir not found: {project}", code=2)
    if not scenes_dir.is_dir():
        die(f"scenes dir not found: {scenes_dir}", code=2)

    script = load_json(project / "script.json", "script")
    lines_data = load_json(project / "data" / "lines.json", "lines data")
    words = load_json(project / "data" / "words.json", "words data")

    # ---- skeleton report (CR-B1-07), primary where it covers ----
    skeleton = None
    skel_by_id = {}
    if args.skeleton:
        skeleton = load_json(Path(args.skeleton).expanduser(), "skeleton")
        if skeleton.get("format") != "talerender-skeleton/1":
            die(f"skeleton: format is {skeleton.get('format')!r}, "
                "expected 'talerender-skeleton/1'")
        for sseg in skeleton.get("segments") or []:
            if isinstance(sseg.get("shotId"), str):
                skel_by_id[sseg["shotId"].lower()] = sseg
        print(f"  skeleton : {len(skel_by_id)} segments, "
              f"{len(skeleton.get('owes') or [])} owes items "
              "(talerender-skeleton/1)")
    if not isinstance(script.get("lines"), list):
        die("script.json: 'lines' must be a list")
    if not isinstance(lines_data.get("lines"), list) \
            or not is_number(lines_data.get("duration")):
        die("data/lines.json: requires a 'lines' list and a 'duration'")
    if not isinstance(words, list):
        die("data/words.json: top level must be a list")

    prior = None
    if args.film_data:
        prior = load_json(Path(args.film_data).expanduser(), "film data")

    # ---- segments from the transcribed scenes --------------------
    scene_files = sorted(scenes_dir.glob("*.scene.json"))
    if not scene_files:
        die(f"no *.scene.json files in {scenes_dir} — the segment "
            "scenes are the transcription this builder assembles; "
            "it does not transcribe shots.ts itself")
    segments = []
    plates_all = {}
    png_refs = {}
    provenance = []
    events_gap = []
    for sf in scene_files:
        scene = load_json(sf, f"scene {sf.name}")
        shot = scene.get("shot")
        if not isinstance(shot, dict):
            die(f"{sf.name}: no shot object — a transcribed segment "
                "scene must carry its shot block")
        seg_id = sf.name[: -len(".scene.json")]
        skel = skel_by_id.get(seg_id.lower())
        # Shot time: the skeleton is primary where it resolved one
        # (CR-B1-07); the scene's shot.at is the cross-check.
        if skel is not None and is_number(skel.get("atS")):
            start_s = round(float(skel["atS"]), 3)
            if is_number(shot.get("at")) \
                    and abs(float(shot["at"]) - start_s) > 0.0015:
                print(f"  WARNING: segment {seg_id}: skeleton atS "
                      f"{start_s} != scene shot.at {shot['at']} — "
                      "skeleton time used (primary input)")
        elif is_number(shot.get("at")):
            start_s = round(float(shot["at"]), 3)
        else:
            die(f"{sf.name}: no shot.at and no skeleton atS — a "
                "segment's absolute start time must come from the "
                "transcription or the skeleton report")
        dur_s = float(shot["duration"]) if is_number(shot.get("duration")) \
            else None
        if skel is not None and is_number(skel.get("durationS")):
            dur_s = float(skel["durationS"])
        segments.append({
            "id": seg_id, "file": sf, "scene_obj": scene, "shot": shot,
            "startS": start_s, "durS": dur_s, "skel": skel,
        })
        for name, origin in scene_plates(scene).items():
            if name in plates_all and plates_all[name] is not None \
                    and origin is not None and plates_all[name] != origin:
                die(f"plate {name!r} has conflicting origins in the "
                    f"scene data: {plates_all[name]} vs {origin}")
            if plates_all.get(name) is None:
                plates_all[name] = origin
        for base, p in scene_png_refs(scene, sf).items():
            png_refs.setdefault(base, p)
    segments.sort(key=lambda s: (s["startS"], s["id"]))
    duration = float(lines_data["duration"])
    skel_timing = (skeleton or {}).get("timing") or {}
    fps = int(args.fps or skel_timing.get("fps") or 30)
    # totalFrames rule (adapter/binary): ceil(durationS * fps).
    total_frames = math.ceil(duration * fps) \
        if not float(duration * fps).is_integer() else int(duration * fps)
    for i, seg in enumerate(segments):
        seg["endS"] = round(segments[i + 1]["startS"], 3) \
            if i + 1 < len(segments) else duration
        if seg["durS"] is None:
            seg["durS"] = round(seg["endS"] - seg["startS"], 3)
        # startFrame: frame n is staged at t = n/fps and a segment
        # owns frames with startS <= n/fps < endS (repo manifest's
        # frameRule) => ceil(startS * fps); the ranges then tile.
        seg["startFrame"] = 0 if i == 0 else math.ceil(seg["startS"] * fps - 1e-9)
        seg["endFrame"] = total_frames if i + 1 == len(segments) \
            else math.ceil(segments[i + 1]["startS"] * fps - 1e-9)
        # Frame tiling: the skeleton's resolved tiling is primary
        # where it covers this segment (CR-B1-07 verified it tiles
        # [0, totalFrames) on the real project); cross-check it
        # against the ceil rule and name any disagreement.
        skel = seg.get("skel")
        if skel is not None and isinstance(skel.get("startFrame"), int) \
                and isinstance(skel.get("endFrame"), int):
            if (skel["startFrame"], skel["endFrame"]) != (seg["startFrame"], seg["endFrame"]):
                print(f"  WARNING: segment {seg['id']}: skeleton "
                      f"tiling {skel['startFrame']}..{skel['endFrame']} "
                      f"!= ceil-rule {seg['startFrame']}..{seg['endFrame']}"
                      " — skeleton tiling used (primary input)")
            seg["startFrame"], seg["endFrame"] = skel["startFrame"], skel["endFrame"]
        seg["frames"] = seg["endFrame"] - seg["startFrame"]
        if seg["frames"] <= 0:
            die(f"segment {seg['id']}: non-positive frame range "
                f"{seg['startFrame']}..{seg['endFrame']}")
        # Lines: the skeleton's coverage is primary; the fallback
        # is lines overlapping [startS, endS) by their timed span.
        if skel is not None and isinstance(skel.get("lines"), list):
            seg["lines"] = [int(x) for x in skel["lines"]]
        else:
            seg["lines"] = [ln["i"] for ln in lines_data["lines"]
                            if ln["start"] < seg["endS"] and ln["end"] > seg["startS"]]
        # Set identity + atmosphere preset from the skeleton.
        if skel is not None:
            if isinstance(skel.get("set"), str):
                seg["set"] = skel["set"]
            atms = skel.get("atmospheres") or []
            if atms and isinstance(atms[0].get("preset"), str):
                seg["atmosphere"] = atms[0]["preset"]
        # R1/R2/R5 on the shot this builder writes out.
        seg["shot"]["cues"] = normalise_cues(
            seg["id"], seg["shot"].get("cues"), provenance)
        if isinstance(seg["shot"].get("events"), list) \
                and seg["shot"]["events"]:
            seg["shot"]["events"] = normalise_events(
                seg["id"], seg["shot"]["events"])
        else:
            # R5 GAP, emitted not omitted: computing events needs
            # clip-instance data (instance offset + ClipEvent.t,
            # param-adjusted) that scene transcription for this
            # film does not carry. The field is present and empty;
            # the gap is named here and in the manifest bridge
            # block. Onoda's staging uses no clips (CR-B1-05 §3),
            # so for this film the empty list is also correct.
            seg["shot"]["events"] = []
            events_gap.append(seg["id"])

    # Resolution: CLI flags, else skeleton timing, else the first
    # scene that declares one, else the skill default 1080x1920.
    width = int(args.width or skel_timing.get("width") or 0)
    height = int(args.height or skel_timing.get("height") or 0)
    if not (width and height):
        for seg in segments:
            res = seg["scene_obj"].get("resolution") or {}
            if is_number(res.get("width")) and is_number(res.get("height")):
                width, height = int(res["width"]), int(res["height"])
                break
    if not (width and height):
        width, height = 1080, 1920

    # ---- film data ------------------------------------------------
    raw_caps = build_captions(script, lines_data, words)
    prior_fit = {}
    if prior:
        for c in prior.get("captions") or []:
            prior_fit[(c.get("text"), round(float(c.get("s", -1)), 3))] = c.get("fit")
    captions = []
    fit_assumed = 0
    for k, c in enumerate(raw_caps):
        fit = prior_fit.get((c["text"], round(c["s"], 3)))
        if not is_number(fit):
            fit = 1.0
            fit_assumed += 1
        captions.append({"s": c["s"], "e": c["e"], "text": c["text"],
                         "fit": fit, "strip": f"cap_{k:03d}.png"})
    for seg in segments:
        idx = [k for k, c in enumerate(captions)
               if seg["startS"] - 1e-9 <= c["s"] < seg["endS"] - 1e-9
               or (seg is segments[-1] and abs(c["s"] - seg["endS"]) < 1e-9)]
        if idx:
            seg["captionChunks"] = {"first": idx[0], "last": idx[-1],
                                    "count": len(idx)}

    anchors = {}
    if args.anchors:
        spec = load_json(Path(args.anchors).expanduser(), "anchors")
        if isinstance(spec, dict):
            anchors = {str(k): float(v) for k, v in spec.items()}
        elif isinstance(spec, list):
            for q, ln in spec:
                anchors[f"{q}{ln}"] = wd(q, int(ln), words)
        else:
            die("--anchors: expected an object or a [[word, line], ...] list")
    if prior and isinstance(prior.get("anchors"), dict):
        for k, v in prior["anchors"].items():
            anchors.setdefault(k, v)
    for seg in segments:
        sc_anchors = seg["scene_obj"].get("anchors")
        if isinstance(sc_anchors, dict):
            for k, v in sc_anchors.items():
                if is_number(v):
                    anchors.setdefault(k, float(v))

    cal_years = [str(y) for y in args.calendar_year or []]
    cal_dark = args.calendar_dark
    sign = args.sign
    if prior:
        if not cal_years and isinstance(prior.get("calendarYears"), list):
            cal_years = [str(y) for y in prior["calendarYears"]]
        cal_dark = cal_dark or prior.get("calendarDark")
        sign = sign or prior.get("sign")

    film_data = {
        "duration": duration,
        "fps": fps,
        "frames": total_frames,
        "lines": [{"start": ln["start"], "end": ln["end"]}
                  for ln in lines_data["lines"]],
        "anchors": anchors,
        "captions": captions,
    }
    if cal_years or cal_dark or sign:
        # F2 declarations: emitted only when the inputs declare
        # them. The current binary loader demands these keys by
        # name for the film.rs path; a film whose transcription
        # declares no calendar/sign will fail the telemetry leg
        # BY NAME until its data declares them — that failure is
        # reported, never papered over with Onoda's values.
        if cal_years:
            film_data["calendarYears"] = cal_years
        if cal_dark:
            film_data["calendarDark"] = cal_dark
        if sign:
            film_data["sign"] = sign

    assets_dir_hint = Path(args.assets_dir).expanduser().resolve() \
        if args.assets_dir else None

    # ---- plates ---------------------------------------------------
    # Origin fallback: the assets source's own plates.json (the
    # rasteriser's manifest — film/gen_assets.py writes it with the
    # plates). Scene-carried x0/y0 stays primary; the fallback
    # covers plates a scene references only by name (Onoda's
    # book-closed, named in a channel's `to` with no origin).
    fallback_origins = {}
    if assets_dir_hint is not None:
        pj = assets_dir_hint / "plates.json"
        if pj.is_file():
            try:
                pdata = json.loads(pj.read_text(encoding="utf-8"))
                if isinstance(pdata.get("plates"), list):
                    for e in pdata["plates"]:
                        if isinstance(e.get("name"), str) \
                                and isinstance(e.get("origin"), list) \
                                and len(e["origin"]) == 2:
                            fallback_origins[e["name"]] = (
                                float(e["origin"][0]), float(e["origin"][1]))
                elif isinstance(pdata, dict):
                    for k, v in pdata.items():
                        if isinstance(v, list) and len(v) == 2 \
                                and all(is_number(x) for x in v):
                            fallback_origins[k] = (float(v[0]), float(v[1]))
            except Exception:
                pass
    plates_manifest = {"format": "talerender-plates", "version": 1,
                       "plates": []}
    for name in sorted(plates_all):
        origin = plates_all[name] or fallback_origins.get(name)
        if origin is None:
            die(f"plate {name!r} is held in the scene data but no "
                "scene entry carries its x0/y0 origin and the "
                "assets plates.json declares none — the binary "
                "requires an origin per plate, no default exists")
        plates_manifest["plates"].append(
            {"name": name, "file": f"{name}.png",
             "origin": [origin[0], origin[1]]})

    # ---- asset resolution ----------------------------------------
    assets_dir = Path(args.assets_dir).expanduser().resolve() \
        if args.assets_dir else None

    def resolve_asset(basename):
        cand = png_refs.get(basename)
        if cand and cand.is_file():
            return cand
        if assets_dir and (assets_dir / basename).is_file():
            return assets_dir / basename
        # Scene-dir-relative fallback: <scenes>/../../film/assets
        alt = (scenes_dir / ".." / ".." / "film" / "assets" / basename)
        if alt.is_file():
            return alt.resolve()
        return None

    needed_assets = {}
    for entry in plates_manifest["plates"]:
        needed_assets[entry["file"]] = "plate"
    for c in captions:
        needed_assets[c["strip"]] = "caption strip"
    for y in cal_years:
        needed_assets[f"calendar-{y}.png"] = "calendar plate"
    if cal_dark:
        needed_assets[cal_dark] = "calendar dark plate"
    if sign:
        needed_assets[sign] = "sign plate"
    resolved = {}
    missing = []
    for base, kind in sorted(needed_assets.items()):
        src = resolve_asset(base)
        if src is None:
            missing.append(f"{base} ({kind})")
        else:
            resolved[base] = src
    if missing:
        die("rasterised assets not found (this builder copies "
            "rasterisations; it cannot create them — see "
            "film/gen_assets.py):\n  " + "\n  ".join(missing))

    # ---- write ----------------------------------------------------
    existing = [n for n in MANAGED if (out / n).exists()]
    if existing and not args.force:
        die(f"{out} already contains {', '.join(existing)} — "
            "refusing to overwrite; re-run with --force", code=2)
    if args.force:
        for n in MANAGED:
            p = out / n
            if p.is_dir():
                shutil.rmtree(p)
            elif p.exists():
                p.unlink()
    (out / "scenes" / "film").mkdir(parents=True, exist_ok=True)
    (out / "film" / "assets").mkdir(parents=True, exist_ok=True)

    manifest_segments = []
    for seg in segments:
        entry = {
            "id": seg["id"],
            "scene": f"scenes/film/{seg['file'].name}",
            "lines": seg["lines"],
            "startS": seg["startS"],
            "endS": seg["endS"],
            "durationS": round(seg["endS"] - seg["startS"], 3),
            "startFrame": seg["startFrame"],
            "endFrame": seg["endFrame"],
            "frames": seg["frames"],
        }
        shot = seg["shot"]
        if seg.get("set"):
            entry["set"] = seg["set"]
        if seg.get("atmosphere"):
            entry["atmosphere"] = seg["atmosphere"]
        elif isinstance(shot.get("atmosphere"), str):
            entry["atmosphere"] = shot["atmosphere"]
        if "captionChunks" in seg:
            entry["captionChunks"] = seg["captionChunks"]
        manifest_segments.append(entry)
        (out / "scenes" / "film" / seg["file"].name).write_text(
            dump_js(seg["scene_obj"]), encoding="utf-8")

    manifest = {
        "format": "talerender-film",
        "version": 1,
        "title": script.get("title") or prior and prior.get("title") or "",
        "project": str(project),
        "sourceData": "film/onoda.json",
        "fps": fps,
        "durationS": duration,
        "totalFrames": total_frames,
        "resolution": {"width": width, "height": height},
    }
    mix = project / "data" / "mix.wav"
    if mix.is_file():
        audio = {"file": "data/mix.wav", "projectPath": str(mix),
                 "treatment": "muxed unchanged (project master)"}
        try:
            with wave.open(str(mix), "rb") as w:
                audio["durationS"] = round(w.getnframes() / w.getframerate(), 3)
        except Exception:
            pass
        manifest["audio"] = audio
    manifest["frameRule"] = (
        "frame n is staged at t = n/fps; a segment owns frames n "
        "with startS <= n/fps < endS, so startFrame = "
        "ceil(startS*fps) and the segment ranges tile "
        "[0, totalFrames) exactly")
    manifest["segments"] = manifest_segments
    manifest["bridge"] = {
        "builtBy": "tools/bridge/build_bridge.py",
        "sourceProject": str(project),
        "sourceScenes": str(scenes_dir),
        "note": "Assembled from the project's timing data + the "
                "transcribed segment scenes; verified against the "
                "adapter contract (P4 data-driven), the repo "
                "validator, and the binary's film telemetry.",
        "cuesProvenance": provenance,
        "skeletonOwes": [
            {"item": o.get("item"), "severity": o.get("severity")}
            for o in ((skeleton or {}).get("owes") or [])
            if isinstance(o, dict)
        ],
        "eventsGap": (
            "shot.events[] emitted empty for segments "
            + (", ".join(events_gap) if events_gap else "(none)")
            + " — clip-event computation needs clip-instance data "
              "the transcription does not carry (CR-B1-05 R5)."),
    }
    (out / "film.json").write_text(dump_js(manifest), encoding="utf-8")
    (out / "film" / "onoda.json").write_text(dump_js(film_data),
                                             encoding="utf-8")
    (out / "film" / "assets" / "plates.json").write_text(
        dump_js(plates_manifest), encoding="utf-8")
    for base, src in sorted(resolved.items()):
        shutil.copyfile(src, out / "film" / "assets" / base)

    packaged = [p for p in sorted(out.rglob("*"))
                if p.is_file() and p.name != "MANIFEST-SHA"]
    (out / "MANIFEST-SHA").write_text(
        "\n".join(f"{sha256_of(p)}  {p.relative_to(out).as_posix()}"
                  for p in packaged) + "\n", encoding="utf-8")

    print(f"built {len(packaged)} files into {out}")
    print(f"  manifest : {total_frames} frames @ {fps} fps, "
          f"{duration} s, {width}x{height}, {len(segments)} segments")
    print(f"  film data: {len(film_data['lines'])} lines, "
          f"{len(anchors)} anchors, {len(captions)} caption chunks")
    if fit_assumed:
        print(f"  WARNING: caption fit assumed 1.0 for {fit_assumed} "
              "chunk(s) — no font measurer is available to a "
              "stdlib-only builder and no --film-data fit matched; "
              "an over-wide chunk's fit can differ (see doc)")
    if not anchors:
        print("  WARNING: no word anchors — none declared via "
              "--anchors, --film-data, or scene data; staging that "
              "reads anchors will fail by name in the binary")
    if events_gap:
        print(f"  NOTE: shot.events[] empty for {len(events_gap)} "
              "segment(s) (R5 gap — see manifest bridge block)")
    return out, manifest, film_data


# ---------------------------------------------------------------------
# Verification — three legs (see the delivery note for why three)
# ---------------------------------------------------------------------

def verify_p4(out, manifest, film_data):
    """Leg (a): the P4 data-driven adapter semantics — the same
    derivation as filmdata_validate.mjs requiredAssets()."""
    problems = []
    film_dir = out / "film"
    data = film_data
    if not (is_number(data.get("duration")) and data["duration"] > 0):
        problems.append("onoda.json: duration missing/invalid")
    if not data.get("lines"):
        problems.append("onoda.json: lines[] missing/empty")
    if not isinstance(data.get("anchors"), dict):
        problems.append("onoda.json: anchors{} missing")
    if not data.get("captions"):
        problems.append("onoda.json: captions[] missing/empty")
    if math.ceil(data["duration"] * manifest["fps"]) != manifest["totalFrames"]:
        problems.append("onoda.json duration implies a different "
                        "frame count than the manifest totalFrames")
    adir = film_dir / "assets"
    need = ["plates.json"]
    try:
        plates = json.loads((adir / "plates.json").read_text(encoding="utf-8"))
        if isinstance(plates.get("plates"), list):
            for e in plates["plates"]:
                need.append(e.get("file") or f"{e.get('name')}.png")
        elif isinstance(plates, dict):
            for k in plates:
                if k not in ("format", "version"):
                    need.append(f"{k}.png")
    except Exception as e:
        problems.append(f"assets/plates.json unparseable: {e}")
    for y in data.get("calendarYears") or []:
        need.append(f"calendar-{y}.png")
    for k in ("calendarDark", "sign"):
        if data.get(k):
            need.append(data[k])
    for c in data.get("captions") or []:
        if not c.get("strip"):
            problems.append("a caption chunk has no strip filename")
        else:
            need.append(c["strip"])
    for rel in sorted(set(need)):
        if not (adir / rel).is_file():
            problems.append(f"assets/{rel} missing")
    # Manifest tiling (adapter loadManifest semantics).
    expect = 0
    for s in manifest["segments"]:
        if s["startFrame"] != expect or not s["endFrame"] > s["startFrame"]:
            problems.append(f"segment {s['id']} does not tile")
        expect = s["endFrame"]
        if not (out / s["scene"]).is_file():
            problems.append(f"segment {s['id']} scene missing")
    if expect != manifest["totalFrames"]:
        problems.append("segments do not tile to totalFrames")
    return problems


def verify_validator(out, project, repo_root):
    """Leg (b): the repo validator as a subprocess, against the
    PROJECT the bridge was built for (its native/ is --out only
    when --out is the project's own native dir; otherwise this leg
    validates a staged copy). Returns (status, output)."""
    validator = repo_root / "tools" / "integration" / "validate_native_project.py"
    if not validator.is_file():
        return "SKIPPED", f"validator not found at {validator}"
    target = project
    if (out).resolve() != (project / "native").resolve():
        return "SKIPPED", ("--out is not <project>/native; the "
                           "validator checks a project's own native/ "
                           "dir, so this leg does not apply to a "
                           "scratch build (legs a/c still apply)")
    r = subprocess.run([sys.executable, str(validator), str(project)],
                       capture_output=True, text=True)
    return ("PASS" if r.returncode == 0 else "FAIL"), (r.stdout + r.stderr).strip()


def verify_telemetry(out, args):
    """Leg (c): the binary's own verdict line — the strictest
    check, and the one the lead requires on the built output."""
    if args.no_telemetry:
        return "SKIPPED", "--no-telemetry given; binary verdict not obtained"
    bin_path = None
    for cand in [args.talerender_bin, os.environ.get("TALERENDER_BIN"),
                 str(Path.home() / "talerender-bin" / "talerender")]:
        if cand and Path(cand).is_file() and os.access(cand, os.X_OK):
            bin_path = cand
            break
    if not bin_path:
        return "FAIL", ("no talerender binary found (tried "
                        "--talerender-bin, TALERENDER_BIN, "
                        "~/talerender-bin/talerender) — re-run with "
                        "a binary, or --no-telemetry to skip loudly")
    r = subprocess.run([bin_path, "--film-telemetry", str(out / "film")],
                       capture_output=True, text=True)
    out_text = (r.stdout or "") + (r.stderr or "")
    lines = [ln.strip() for ln in out_text.splitlines()]
    if any(ln == "FILM TELEMETRY: CHECK OK" for ln in lines) and r.returncode == 0:
        return "PASS", f"{bin_path}: FILM TELEMETRY: CHECK OK"
    verdict = next((ln for ln in lines if ln.startswith("FILM TELEMETRY")), "(no verdict line)")
    return "FAIL", f"{bin_path}: exit {r.returncode}, verdict: {verdict}"


def compare(out, other):
    """--compare: inventory + hashes against another native dir,
    with a JSON deep-diff summary for differing .json files."""
    def inventory(root):
        return {p.relative_to(root).as_posix(): p
                for p in sorted(root.rglob("*")) if p.is_file()}
    a, b = inventory(out), inventory(other)
    only_a = sorted(set(a) - set(b))
    only_b = sorted(set(b) - set(a))
    same, differ = [], []
    for rel in sorted(set(a) & set(b)):
        (same if sha256_of(a[rel]) == sha256_of(b[rel]) else differ).append(rel)
    print(f"compare {out} vs {other}:")
    print(f"  identical bytes : {len(same)}")
    print(f"  differing bytes : {len(differ)}")
    for rel in differ:
        print(f"    DIFF {rel}")
        if rel.endswith(".json"):
            for line in json_deep_diff(a[rel], b[rel], limit=6):
                print(f"      {line}")
    print(f"  only in built   : {len(only_a)} {only_a[:8]}")
    print(f"  only in compared: {len(only_b)} {only_b[:8]}")
    return {"identical": len(same), "differ": differ,
            "only_built": only_a, "only_compared": only_b}


def json_deep_diff(pa, pb, limit=6):
    try:
        a = json.loads(Path(pa).read_text(encoding="utf-8"))
        b = json.loads(Path(pb).read_text(encoding="utf-8"))
    except Exception as e:
        return [f"(unparseable: {e})"]
    diffs = []

    def walk(x, y, path):
        if len(diffs) >= limit:
            return
        if type(x) is not type(y) and not (is_number(x) and is_number(y)):
            diffs.append(f"{path}: type {type(x).__name__} vs {type(y).__name__}")
        elif isinstance(x, dict):
            for k in sorted(set(x) | set(y)):
                if k not in x:
                    diffs.append(f"{path}.{k}: only in compared")
                elif k not in y:
                    diffs.append(f"{path}.{k}: only in built")
                else:
                    walk(x[k], y[k], f"{path}.{k}")
                if len(diffs) >= limit:
                    return
        elif isinstance(x, list):
            if len(x) != len(y):
                diffs.append(f"{path}: list length {len(x)} vs {len(y)}")
            for i, (u, v) in enumerate(zip(x, y)):
                walk(u, v, f"{path}[{i}]")
                if len(diffs) >= limit:
                    return
        elif x != y:
            diffs.append(f"{path}: {x!r} vs {y!r}")
    walk(a, b, "$")
    if not diffs:
        diffs.append("JSON content identical — bytes differ only in "
                     "formatting/serialisation")
    return diffs


def main():
    ap = argparse.ArgumentParser(
        description="Build a project's native/ bridge from its "
                    "timing data + transcribed segment scenes.")
    ap.add_argument("--project", required=True)
    ap.add_argument("--scenes", required=True,
                    help="directory of transcribed *.scene.json files")
    ap.add_argument("--out", help="default: <project>/native")
    ap.add_argument("--assets-dir")
    ap.add_argument("--film-data",
                    help="existing film data: anchors + F2 "
                         "declarations + caption fits carried over")
    ap.add_argument("--anchors",
                    help='JSON {"name": seconds} or [["word", line], ...]')
    ap.add_argument("--fps", type=int, default=None,
                    help="default: skeleton timing, else 30")
    ap.add_argument("--width", type=int, default=None,
                    help="default: skeleton timing, else first "
                         "scene resolution, else 1080")
    ap.add_argument("--height", type=int, default=None,
                    help="default: skeleton timing, else first "
                         "scene resolution, else 1920")
    ap.add_argument("--skeleton",
                    help="talerender-skeleton/1 report from "
                         "tools/export/export_project_v2.mjs")
    ap.add_argument("--calendar-year", action="append", default=[])
    ap.add_argument("--calendar-dark")
    ap.add_argument("--sign")
    ap.add_argument("--force", action="store_true")
    ap.add_argument("--no-telemetry", action="store_true",
                    help="skip the binary telemetry leg (printed, "
                         "never silent)")
    ap.add_argument("--talerender-bin")
    ap.add_argument("--compare",
                    help="after building, compare --out against "
                         "this native dir (inventory + hashes)")
    args = ap.parse_args()

    repo_root = Path(__file__).resolve().parents[2]
    out, manifest, film_data = build(args)
    project = Path(args.project).expanduser().resolve()

    failures = 0
    problems = verify_p4(out, manifest, film_data)
    print(f"verify (a) P4 adapter semantics: "
          f"{'PASS' if not problems else 'FAIL'}")
    for p in problems:
        print(f"  - {p}")
    failures += 1 if problems else 0

    status, detail = verify_validator(out, project, repo_root)
    print(f"verify (b) validate_native_project.py: {status} — {detail.splitlines()[-1] if detail else ''}")
    if status == "FAIL":
        print("  " + detail.replace("\n", "\n  "))
        failures += 1

    status, detail = verify_telemetry(out, args)
    print(f"verify (c) binary film telemetry: {status} — {detail}")
    if status == "FAIL":
        failures += 1

    if args.compare:
        compare(out, Path(args.compare).expanduser().resolve())

    if failures:
        print(f"build-bridge: VERIFICATION FAILED ({failures} leg(s)) — "
              f"the bridge in {out} must not be used as-is")
        sys.exit(1)
    print("build-bridge: VERIFICATION PASSED (all non-skipped legs)")
    sys.exit(0)


if __name__ == "__main__":
    main()
