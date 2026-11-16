#!/usr/bin/env python3
# =====================================================================
# AMENDMENT — Chrome Replacement, Batch 2, Seat 6 (CR-B2-06).
# This staged replacement fixes the validator's OWN drift found by
# CR-B1-06: the original assumed the pre-F2 FLAT plates.json shape
# ({"<name>": [x, y]}) and a hardcoded Onoda asset list, so it FAILED
# every current-generation bridge (it demanded format.png /
# plates.png / version.png from the manifest-shape plates.json) and
# PASSED the stale packaged Onoda bridge the current binary refuses.
# The source of truth is now the current bridge generation, exactly
# as defined by tools/bridge/build_bridge.py and the data-driven
# template/tools/native_modes/filmdata_validate.mjs:
#   - required assets are DERIVED from the data (plates.json entries
#     in the talerender-plates manifest shape; the film data's own
#     calendarYears / calendarDark / sign declarations; caption
#     strips) — no hardcoded film's names remain;
#   - a bridge in the OLD generation (flat plates.json) is
#     reported as STALE with the re-bridge remedy, never passed;
#     [R1-03 AMENDMENT, REMEDIATION-CONTRACTS §4: film data
#     missing the F2 keys is NO LONGER a stale marker — the
#     decoupled binary loader treats calendarYears /
#     calendarDark / sign as optional declarations (absence =
#     empty calendar / no sign). Staleness is MANIFEST-SHA /
#     generation drift only: the flat plates.json shape and
#     SHA mismatch below. A PRESENT-but-malformed F2 key still
#     fails by name, mirroring the loader.]
#   - MANIFEST-SHA, when present, is verified — a bridge whose files
#     drifted from its packaged hashes is stale by definition;
#   - an unbridged project names the bridge step
#     (tools/bridge/build_bridge.py) as the remedy. No diagnostic in
#     this tool routes anyone to Chrome; there is no Chrome fallback.
# Everything not listed above is the original validator, unchanged.
# =====================================================================
# DELIVERY NOTE — TaleRender V2, Batch 2, Seat 6 (WP9 production
# integration, repo side)
#
# Files created (this seat, NEW files only):
#   tools/integration/validate_native_project.py   (this file)
#   docs/integration/V2-WP9-INTEGRATION.md         (integration guide;
#                                                   carries its own
#                                                   copy of this note
#                                                   at the top)
#
# Contract points implemented:
#   - V2-CONTRACTS §5 Batch 2 seat 6 / §6: WP9 is repo-side only.
#     No skill-tree file was read for writing or modified; the
#     skill-side surface (capture_native.mjs, selftest hook) is
#     integrated WITH, from the repo side, and skill-side changes
#     appear only as PROPOSALS in the companion doc.
#   - Master prompt §13: native stays opt-in, Chrome stays the
#     default, Chrome compatibility untouched. This validator
#     changes no default and invokes no renderer.
#   - The checks mirror the adapter's own pre-validation
#     (capture_native.mjs loadManifest + validateFilmData) so a
#     project that passes here does not fail there for a reason
#     this tool could have named first — plus the generic Tall
#     Tale project shape (script.json / data/* / src/shots.ts)
#     the adapter itself does not look at.
#
# Wiring the lead must do: NONE. This is a standalone stdlib-only
# CLI; no mod line, no call site, no Cargo change. Optional
# follow-up (lead/user decision, not done here): mention the tool
# in an index doc or CI Tools job.
#
# Tests included: `--self-test` builds three fixture projects in a
# temp dir (valid / frame-numbering gap / missing required file),
# runs this CLI on each as a subprocess, and prints PASS/FAIL per
# case. Run by the seat: see the companion doc for the result.
#
# Known limitations:
#   - (CR-B2-06: RESOLVED — the adapter's validator is now the
#     data-driven filmdata_validate.mjs and this tool derives its
#     asset list from the data the same way; the hardcoded list is
#     gone from both.)
#   - PNG validation is structural (signature / IHDR dimensions /
#     closing IEND), the same integrity bar as the adapter's
#     pngCheck; pixel content is not decoded.
#   - A missing data/mix.wav is FLAGGED, not failed: capture's
#     video mode legitimately renders SILENT without one, so the
#     validator reports it loudly but does not block on it.
# =====================================================================
"""Validate a Tall Tale project for the TaleRender native backend.

Usage:
    validate_native_project.py <project-dir> [--frames <dir>]
                               [--expect-frames N]
    validate_native_project.py --self-test

Exit codes:
    0  project is valid (flagged warnings may still be printed)
    2  validation failed -- the FULL problem list is printed, never
       just the first problem (usage errors also exit 2)
    1  only from --self-test: a fixture case did not behave as
       specified (the tool itself is broken)

Python standard library only. Read-only: this tool writes nothing
outside the temp fixtures its --self-test creates (and removes).
"""

import argparse
import binascii
import json
import math
import os
import re
import struct
import subprocess
import sys
import tempfile
import wave
import zlib

# The remedy every bridge problem names. There is no other renderer
# and no fallback: an unbridged or stale project is bridged (again).
BRIDGE_REMEDY = ("bridge it with tools/bridge/build_bridge.py "
                 "--project <project> --scenes <transcribed "
                 "*.scene.json dir> [--film-data <prior film data>] "
                 "(see docs/integration/CR-B1-06-BRIDGE.md)")

# F2 film-data keys (film.rs load_film_data). PRE-decoupling the
# loader demanded them BY NAME (verified 2026-11-15: the binary
# refused the packaged Onoda data with "film data: missing
# calendarYears"), and absence was a previous-generation marker.
# REMEDIATION-CONTRACTS §4 (loader decoupling) makes all three
# OPTIONAL: absence = empty calendar / no sign, and a fresh
# scene-sequence bridge legitimately carries none of them. Only
# a present-but-wrong-type value still fails by name.
F2_KEYS = ("calendarYears", "calendarDark", "sign")

PNG_SIG = b"\x89PNG\r\n\x1a\n"
FRAME_RE = re.compile(r"^frame_(\d+)\.png$")
STAGED_RE = re.compile(r"^t(\d+(?:\.\d+)?)\.png$")


# ---------------------------------------------------------------------
# Small helpers
# ---------------------------------------------------------------------

def load_json(path, label, problems):
    """Parse a JSON file; append a problem and return None on failure."""
    try:
        with open(path, "r", encoding="utf-8") as fh:
            return json.load(fh)
    except (OSError, ValueError) as exc:
        problems.append("%s: cannot read/parse %s: %s" % (label, path, exc))
        return None


def is_pos_number(value):
    return isinstance(value, (int, float)) and not isinstance(value, bool) \
        and math.isfinite(value) and value > 0


def png_problem(path, want_w=None, want_h=None):
    """Structural PNG check mirroring capture_native.mjs pngCheck().

    Returns None when the frame is usable, else a short reason.
    Checks: regular file, >= 45 bytes, PNG signature, IHDR first
    chunk, dimensions (when expected), closing IEND chunk.
    """
    try:
        size = os.path.getsize(path)
    except OSError:
        return "missing"
    if not os.path.isfile(path):
        return "not a regular file"
    if size < 45:
        return "only %d bytes -- truncated" % size
    try:
        with open(path, "rb") as fh:
            head = fh.read(24)
            fh.seek(size - 12)
            tail = fh.read(12)
    except OSError as exc:
        return "unreadable: %s" % exc
    if len(head) < 24 or head[:8] != PNG_SIG:
        return "bad PNG signature"
    if head[12:16] != b"IHDR":
        return "first chunk is not IHDR"
    width, height = struct.unpack(">II", head[16:24])
    if want_w is not None and (width != want_w or height != want_h):
        return "dimensions %dx%d, expected %dx%d" % (width, height, want_w, want_h)
    if len(tail) < 12 or struct.unpack(">I", tail[0:4])[0] != 0 \
            or tail[4:8] != b"IEND":
        return "no closing IEND chunk -- truncated"
    return None


# ---------------------------------------------------------------------
# Project validation
# ---------------------------------------------------------------------

def validate_project(project):
    """Return (problems, flagged, info) for a project directory."""
    problems = []
    flagged = []
    info = {}

    if not os.path.isdir(project):
        return ["project dir does not exist or is not a directory: %s"
                % project], flagged, info

    def rel(*parts):
        return os.path.join(project, *parts)

    # -- Generic Tall Tale project shape ------------------------------
    script_path = rel("script.json")
    if not os.path.isfile(script_path):
        problems.append("required file missing: script.json")
        script = None
    else:
        script = load_json(script_path, "script.json", problems)

    script_lines = None
    if isinstance(script, dict):
        lines = script.get("lines")
        if not isinstance(lines, list) or not lines:
            problems.append("script.json: lines[] missing/empty -- "
                            "no narration lines, nothing to stage")
        else:
            script_lines = lines
            for i, line in enumerate(lines):
                if not isinstance(line, dict) \
                        or not isinstance(line.get("text"), str) \
                        or not line["text"].strip():
                    problems.append(
                        "script.json: line %d has no non-empty text" % i)
    elif script is not None:
        problems.append("script.json: top level is not a JSON object")

    lines_path = rel("data", "lines.json")
    if not os.path.isfile(lines_path):
        problems.append("required file missing: data/lines.json "
                        "(timed narration lines; the bridge is generated "
                        "from it)")
        lines_data = None
    else:
        lines_data = load_json(lines_path, "data/lines.json", problems)

    timed_lines = None
    lines_duration = None
    if isinstance(lines_data, dict):
        timed_lines = lines_data.get("lines")
        if not isinstance(timed_lines, list) or not timed_lines:
            problems.append("data/lines.json: lines[] missing/empty")
            timed_lines = None
        else:
            prev_start = None
            for i, line in enumerate(timed_lines):
                if not isinstance(line, dict):
                    problems.append("data/lines.json: line %d is not an "
                                    "object" % i)
                    continue
                start, end = line.get("start"), line.get("end")
                if not (isinstance(start, (int, float))
                        and isinstance(end, (int, float))):
                    problems.append("data/lines.json: line %d lacks "
                                    "numeric start/end" % i)
                    continue
                if not end > start:
                    problems.append("data/lines.json: line %d end (%s) "
                                    "is not after start (%s)"
                                    % (i, end, start))
                if prev_start is not None and start < prev_start:
                    problems.append("data/lines.json: line %d starts "
                                    "(%s) before the previous line (%s) "
                                    "-- timings not monotonic"
                                    % (i, start, prev_start))
                prev_start = start
        if is_pos_number(lines_data.get("duration")):
            lines_duration = lines_data["duration"]
    elif lines_data is not None:
        problems.append("data/lines.json: top level is not a JSON object")

    if script_lines is not None and timed_lines is not None \
            and len(script_lines) != len(timed_lines):
        problems.append("line count mismatch: script.json has %d lines, "
                        "data/lines.json has %d -- script and timing "
                        "data have drifted apart"
                        % (len(script_lines), len(timed_lines)))

    words_path = rel("data", "words.json")
    if not os.path.isfile(words_path):
        problems.append("required file missing: data/words.json "
                        "(word timings; the bridge's anchors/captions "
                        "are generated from it)")
    else:
        words = load_json(words_path, "data/words.json", problems)
        if isinstance(words, list):
            if not words:
                problems.append("data/words.json: empty word list")
            else:
                for i, word in enumerate(words[:100000]):
                    if isinstance(word, dict) and "s" in word and "e" in word:
                        if not word["e"] >= word["s"]:
                            problems.append(
                                "data/words.json: word %d ends (%s) "
                                "before it starts (%s)"
                                % (i, word["e"], word["s"]))
                            break
        elif isinstance(words, dict):
            if not words:
                problems.append("data/words.json: empty object")
        elif words is not None:
            problems.append("data/words.json: expected a list of word "
                            "timings (or an object), got %s"
                            % type(words).__name__)

    if not os.path.isfile(rel("src", "shots.ts")):
        problems.append("required file missing: src/shots.ts "
                        "(the staging the bridge transcribes)")

    # Audio mix: present-or-flagged. capture_native.mjs video mode
    # renders SILENT when data/mix.wav is absent, so absence is a
    # loud flag, not a hard failure -- a silent master shipped by
    # accident is exactly what the flag exists to prevent.
    mix_path = rel("data", "mix.wav")
    if not os.path.isfile(mix_path):
        flagged.append("AUDIO: data/mix.wav missing -- the native "
                       "finish would mux SILENCE (adapter prints "
                       "SILENT). Run the mix step before a video "
                       "capture meant to be a master.")
    elif os.path.getsize(mix_path) < 44:
        problems.append("data/mix.wav is only %d bytes -- not a "
                        "plausible WAV master" % os.path.getsize(mix_path))

    if not os.path.isfile(rel("tools", "capture_native.mjs")):
        flagged.append("tools/capture_native.mjs not present in this "
                       "project -- the native adapter lives in the "
                       "skill template; copy the template's native "
                       "tools into this project (the P1 retrofit) "
                       "before invoking the native path from this "
                       "folder.")

    # -- Native bridge: manifest --------------------------------------
    native_dir = rel("native")
    manifest_path = None
    for cand in (os.path.join(native_dir, "film.json"),
                 os.path.join(native_dir, "scenes", "film", "film.json")):
        if os.path.isfile(cand):
            manifest_path = cand
            break
    if manifest_path is None:
        problems.append("required file missing: native film manifest "
                        "(native/film.json or "
                        "native/scenes/film/film.json) -- project is "
                        "not bridged; the native backend refuses "
                        "unbridged projects. Remedy: " + BRIDGE_REMEDY)
        manifest = None
    else:
        manifest = load_json(manifest_path, "film manifest", problems)

    fps = None
    total_frames = None
    duration_s = None
    width = height = None
    segments = None
    if isinstance(manifest, dict):
        if manifest.get("format") != "talerender-film":
            problems.append('film manifest: format is %r, expected '
                            '"talerender-film"' % manifest.get("format"))
        if manifest.get("version") != 1:
            problems.append("film manifest: version %r not supported "
                            "(expected 1)" % manifest.get("version"))
        if is_pos_number(manifest.get("fps")) \
                and float(manifest["fps"]).is_integer():
            fps = int(manifest["fps"])
        else:
            problems.append("film manifest: fps missing/invalid (%r)"
                            % manifest.get("fps"))
        env_fps = os.environ.get("FPS")
        if env_fps and fps is not None:
            try:
                if int(float(env_fps)) != fps:
                    problems.append(
                        "FPS env is %s but the manifest is %d fps -- "
                        "capture_native.mjs would refuse (the binary "
                        "renders at the manifest rate)" % (env_fps, fps))
            except ValueError:
                problems.append("FPS env %r is not a number" % env_fps)
        res = manifest.get("resolution") or {}
        if isinstance(res, dict) and is_pos_number(res.get("width")) \
                and is_pos_number(res.get("height")):
            width, height = int(res["width"]), int(res["height"])
        else:
            problems.append("film manifest: resolution missing/invalid")
        if is_pos_number(manifest.get("durationS")):
            duration_s = float(manifest["durationS"])
        else:
            problems.append("film manifest: durationS missing/invalid")
        if isinstance(manifest.get("totalFrames"), int) \
                and manifest["totalFrames"] > 0:
            total_frames = manifest["totalFrames"]
        else:
            problems.append("film manifest: totalFrames missing/invalid")
        if fps is not None and duration_s is not None \
                and total_frames is not None:
            want = math.ceil(duration_s * fps)
            if want != total_frames:
                problems.append(
                    "film manifest: totalFrames %d != ceil(durationS "
                    "%s * fps %d) = %d -- manifest disagrees with the "
                    "binary's frame rule" % (total_frames, duration_s,
                                             fps, want))
        segments = manifest.get("segments")
        if not isinstance(segments, list) or not segments:
            problems.append("film manifest: segments[] missing/empty -- "
                            "no shots to render")
            segments = None
        else:
            expect = 0
            for seg in segments:
                if not isinstance(seg, dict):
                    problems.append("film manifest: a segment is not "
                                    "an object")
                    continue
                sid = seg.get("id", "?")
                # The generic parser (filmseq.rs) demands startS /
                # endS by name (mnum) — a manifest without them is
                # refused by name at load, so the validator must
                # refuse it here first (R2-10 defect D3).
                start_s, end_s = seg.get("startS"), seg.get("endS")
                if not (isinstance(start_s, (int, float))
                        and isinstance(end_s, (int, float))):
                    problems.append("film manifest: segment %s lacks "
                                    "numeric startS/endS — the generic "
                                    "driver/telemetry parser demands "
                                    "them by name" % sid)
                    continue
                if not end_s > start_s:
                    problems.append("film manifest: segment %s endS "
                                    "(%s) must exceed startS (%s)"
                                    % (sid, end_s, start_s))
                    continue
                start, end = seg.get("startFrame"), seg.get("endFrame")
                if not (isinstance(start, int) and isinstance(end, int)):
                    problems.append("film manifest: segment %s lacks "
                                    "integer startFrame/endFrame" % sid)
                    continue
                if start != expect or not end > start:
                    problems.append(
                        "film manifest: segment %s frame range %d..%d "
                        "does not tile [0, %s) (expected start %d)"
                        % (sid, start, end, total_frames, expect))
                expect = end
                if isinstance(seg.get("frames"), int) \
                        and seg["frames"] != end - start:
                    problems.append(
                        "film manifest: segment %s declares frames=%d "
                        "but its range spans %d"
                        % (sid, seg["frames"], end - start))
                scene = seg.get("scene")
                if not scene:
                    problems.append("film manifest: segment %s names "
                                    "no scene file" % sid)
                elif not os.path.isfile(os.path.join(native_dir, scene)):
                    problems.append(
                        "film manifest: segment %s scene file missing: "
                        "%s" % (sid, os.path.join(native_dir, scene)))
                else:
                    load_json(os.path.join(native_dir, scene),
                              "scene file (segment %s)" % sid, problems)
            if total_frames is not None and expect != total_frames:
                problems.append(
                    "film manifest: segments tile to frame %d, "
                    "totalFrames is %d" % (expect, total_frames))
        info.update(title=manifest.get("title"), fps=fps,
                    total_frames=total_frames, duration_s=duration_s,
                    width=width, height=height,
                    segments=len(segments) if segments else 0)
    elif manifest is not None:
        problems.append("film manifest: top level is not a JSON object")

    if lines_duration is not None and duration_s is not None \
            and abs(lines_duration - duration_s) > 0.25:
        problems.append("duration mismatch: data/lines.json says "
                        "%.3fs, the film manifest says %.3fs -- the "
                        "bridge was not regenerated from the current "
                        "timing data" % (lines_duration, duration_s))

    # -- Native bridge: binary film data (onoda.json + assets) --------
    film_dir = None
    for cand in (os.path.join(native_dir, "film"), native_dir):
        if os.path.isfile(os.path.join(cand, "onoda.json")):
            film_dir = cand
            break
    if film_dir is None:
        problems.append("required file missing: film data onoda.json "
                        "(looked in native/film/ and native/) -- the "
                        "adapter never improvises film data. Remedy: "
                        + BRIDGE_REMEDY)
        data = None
    else:
        data = load_json(os.path.join(film_dir, "onoda.json"),
                         "film data", problems)

    caption_strips = []
    if isinstance(data, dict):
        dur = data.get("duration")
        if not is_pos_number(dur):
            problems.append("onoda.json: duration missing/invalid")
            dur = None
        if fps is not None and dur is not None \
                and total_frames is not None \
                and math.ceil(dur * fps) != total_frames:
            problems.append(
                "onoda.json: duration %ss implies %d frames, manifest "
                "totalFrames is %d -- the bridge data and the "
                "manifest disagree; regenerate them together"
                % (dur, math.ceil(dur * fps), total_frames))
        if isinstance(data.get("frames"), int) \
                and total_frames is not None \
                and data["frames"] != total_frames:
            problems.append("onoda.json: frames=%d but manifest "
                            "totalFrames is %d"
                            % (data["frames"], total_frames))
        if isinstance(data.get("fps"), (int, float)) \
                and fps is not None and int(data["fps"]) != fps:
            problems.append("onoda.json: fps %r but manifest fps is %d"
                            % (data["fps"], fps))
        dlines = data.get("lines")
        if not isinstance(dlines, list) or not dlines:
            problems.append("onoda.json: lines[] missing/empty")
        else:
            # Current-generation shape (build_bridge.py): each line
            # is {start, end} — the binary loader's shape.
            for i, ln in enumerate(dlines):
                if not isinstance(ln, dict) \
                        or not (isinstance(ln.get("start"), (int, float))
                                and isinstance(ln.get("end"), (int, float))):
                    problems.append("onoda.json: line %d lacks numeric "
                                    "start/end (current-generation "
                                    "film data carries {start, end} "
                                    "per line)" % i)
                    break
        if not isinstance(data.get("anchors"), dict):
            problems.append("onoda.json: anchors{} missing")
        # F2 declarations are OPTIONAL (REMEDIATION-CONTRACTS §4,
        # loader decoupling): film data with NO calendarYears /
        # calendarDark / sign is a current-generation
        # scene-sequence bridge, NOT a stale one — the decoupled
        # loader reads absence as an empty calendar / no sign,
        # and staleness is MANIFEST-SHA / generation drift only.
        # What still fails BY NAME, mirroring the loader, is a
        # declaration that is present but malformed.
        if "calendarYears" in data and not (
                isinstance(data["calendarYears"], list)
                and all(isinstance(y, str)
                        for y in data["calendarYears"])):
            problems.append(
                "onoda.json: calendarYears is present but is not "
                "a list of strings -- the binary's film loader "
                "fails it by name (absence would be legal; a "
                "malformed declaration is not)")
        for key in ("calendarDark", "sign"):
            if key in data and not isinstance(data[key], str):
                problems.append(
                    "onoda.json: %s is present but is not a "
                    "string -- the binary's film loader fails it "
                    "by name (absence would be legal; a malformed "
                    "declaration is not)" % key)
        captions = data.get("captions")
        if not isinstance(captions, list) or not captions:
            problems.append("onoda.json: captions[] missing/empty")
        else:
            for i, cap in enumerate(captions):
                if not isinstance(cap, dict) \
                        or not isinstance(cap.get("strip"), str) \
                        or not cap["strip"]:
                    problems.append("onoda.json: caption chunk %d has "
                                    "no strip filename" % i)
                else:
                    caption_strips.append(cap["strip"])
                if isinstance(cap, dict) \
                        and not (isinstance(cap.get("s"), (int, float))
                                 and isinstance(cap.get("e"), (int, float))):
                    problems.append("onoda.json: caption chunk %d "
                                    "lacks numeric s/e" % i)
        if segments:
            n_lines = len(dlines) if isinstance(dlines, list) else 0
            for seg in segments:
                if isinstance(seg, dict) \
                        and isinstance(seg.get("lines"), list):
                    for li in seg["lines"]:
                        if isinstance(li, int) and not 0 <= li < n_lines:
                            problems.append(
                                "film manifest: segment %s references "
                                "line %d but onoda.json has %d lines"
                                % (seg.get("id", "?"), li, n_lines))
    elif data is not None:
        problems.append("onoda.json: top level is not a JSON object")

    if film_dir is not None:
        assets_dir = os.path.join(film_dir, "assets")
        if not os.path.isdir(assets_dir):
            problems.append("film data: assets/ dir missing in %s"
                            % film_dir)
        else:
            plates_path = os.path.join(assets_dir, "plates.json")
            if not os.path.isfile(plates_path):
                problems.append("film data: assets/plates.json missing")
            else:
                plates = load_json(plates_path, "assets/plates.json",
                                   problems)
                # Data-driven derivation, mirroring
                # filmdata_validate.mjs requiredAssets() exactly:
                # the required plate files come from plates.json's
                # own declarations, in whichever shape it carries.
                if isinstance(plates, dict) \
                        and isinstance(plates.get("plates"), list):
                    # Current generation: the talerender-plates
                    # manifest shape the binary's load_assets reads.
                    if plates.get("format") != "talerender-plates":
                        problems.append(
                            "assets/plates.json: format is %r, "
                            "expected 'talerender-plates' (the "
                            "current binary's load_assets contract)"
                            % plates.get("format"))
                    for entry in plates["plates"]:
                        if not isinstance(entry, dict) \
                                or not isinstance(entry.get("name"), str) \
                                or not entry.get("name"):
                            problems.append(
                                "assets/plates.json: a plates[] entry "
                                "has no name")
                            continue
                        fname = entry.get("file") \
                            if isinstance(entry.get("file"), str) \
                            and entry.get("file") \
                            else "%s.png" % entry["name"]
                        origin = entry.get("origin")
                        if not (isinstance(origin, list)
                                and len(origin) == 2
                                and all(isinstance(v, (int, float))
                                        and not isinstance(v, bool)
                                        for v in origin)):
                            problems.append(
                                "assets/plates.json: plate %s has no "
                                "[x, y] origin -- the binary requires "
                                "an origin per plate" % entry["name"])
                        if not os.path.isfile(
                                os.path.join(assets_dir, fname)):
                            problems.append(
                                "film data: plate %s missing "
                                "(declared in plates.json)" % fname)
                elif isinstance(plates, dict):
                    # Legacy flat shape {"<name>": [x, y]}: a
                    # previous bridge generation. The current
                    # binary's load_assets rejects this shape; the
                    # bridge is STALE even if every PNG is present.
                    problems.append(
                        "STALE BRIDGE: assets/plates.json is the "
                        "legacy flat shape ({\"<name>\": [x, y]}) -- "
                        "the current binary's load_assets requires "
                        "the talerender-plates manifest shape "
                        "({format, version, plates: [{name, file, "
                        "origin}]}). Remedy: re-bridge -- "
                        + BRIDGE_REMEDY)
                    for name in sorted(plates):
                        if name in ("format", "version"):
                            continue
                        png = os.path.join(assets_dir, "%s.png" % name)
                        if not os.path.isfile(png):
                            problems.append(
                                "film data: plate %s.png missing "
                                "(declared in plates.json)" % name)
                elif plates is not None:
                    problems.append("assets/plates.json: top level is "
                                    "not a JSON object")
            # Calendar / sign assets are required ONLY as the film
            # data itself declares them (F2 fields) — the
            # filmdata_validate.mjs derivation. No hardcoded years,
            # no hardcoded filenames.
            if isinstance(data, dict):
                derived = []
                years = data.get("calendarYears")
                if isinstance(years, list):
                    for y in years:
                        derived.append("calendar-%s.png" % y)
                elif "calendarYears" in data:
                    problems.append("onoda.json: calendarYears is "
                                    "not a list of years")
                for key in ("calendarDark", "sign"):
                    if isinstance(data.get(key), str) and data[key]:
                        derived.append(data[key])
                for fname in sorted(set(derived)):
                    if not os.path.isfile(os.path.join(assets_dir,
                                                       fname)):
                        problems.append(
                            "film data: assets/%s missing (declared "
                            "by onoda.json's F2 fields)" % fname)
            for strip in sorted(set(caption_strips)):
                if not os.path.isfile(os.path.join(assets_dir, strip)):
                    problems.append(
                        "film data: caption strip assets/%s missing "
                        "(named by onoda.json captions[])" % strip)

    # -- Native bridge: MANIFEST-SHA integrity -------------------------
    # Both bridge builders (package_bridge.py, build_bridge.py)
    # package a MANIFEST-SHA of every bridged file. A bridge whose
    # files no longer match it was edited or partially regenerated
    # after packaging — stale; the remedy is a clean re-bridge.
    sha_path = rel("native", "MANIFEST-SHA")
    if manifest_path is not None and os.path.isfile(sha_path):
        import hashlib as _hashlib
        try:
            entries = {}
            with open(sha_path, "r", encoding="utf-8") as fh:
                for raw in fh:
                    raw = raw.strip()
                    if not raw:
                        continue
                    digest, _, relpath = raw.partition("  ")
                    entries[relpath.strip()] = digest.strip()
            drift = []
            for relpath, digest in sorted(entries.items()):
                p = os.path.join(native_dir, relpath)
                if not os.path.isfile(p):
                    drift.append("%s (missing)" % relpath)
                    continue
                h = _hashlib.sha256()
                with open(p, "rb") as fh:
                    for chunk in iter(lambda: fh.read(1 << 20), b""):
                        h.update(chunk)
                if h.hexdigest() != digest:
                    drift.append("%s (hash differs)" % relpath)
            if drift:
                problems.append(
                    "STALE BRIDGE: native/MANIFEST-SHA does not "
                    "match the bridge files (%s) -- the bridge was "
                    "modified after packaging. Remedy: re-bridge "
                    "with --force -- %s"
                    % ("; ".join(drift[:8]), BRIDGE_REMEDY))
        except OSError as exc:
            flagged.append("native/MANIFEST-SHA unreadable (%s) -- "
                           "integrity not verified" % exc)
    elif manifest_path is not None:
        flagged.append("native/MANIFEST-SHA absent -- bridge "
                       "integrity not verifiable (both bridge "
                       "builders package one; its absence suggests "
                       "a hand-assembled bridge)")

    info["lines"] = len(timed_lines) if timed_lines else 0
    info["captions"] = len(caption_strips)
    return problems, flagged, info


# ---------------------------------------------------------------------
# Rendered-frames validation
# ---------------------------------------------------------------------

def validate_frames(frames_dir, expect_frames, info):
    """Return (problems, flagged) for a rendered output directory."""
    problems = []
    flagged = []
    if not os.path.isdir(frames_dir):
        return ["frames dir does not exist or is not a directory: %s"
                % frames_dir], flagged

    want_w = info.get("width")
    want_h = info.get("height")
    indexed = {}   # frame_<n>.png (the adapter's cache naming)
    staged = {}    # t<time>.png (frames-mode staged copies)
    for name in sorted(os.listdir(frames_dir)):
        path = os.path.join(frames_dir, name)
        if not os.path.isfile(path):
            continue
        match = FRAME_RE.match(name)
        if match:
            indexed[int(match.group(1))] = path
            if not re.match(r"^frame_\d{5}\.png$", name):
                flagged.append("frames: %s is not zero-padded to 5 "
                               "digits (frame_%%05d.png) -- ffmpeg's "
                               "image2 sequence may not pick it up"
                               % name)
            continue
        match = STAGED_RE.match(name)
        if match:
            staged[float(match.group(1))] = path
            continue
        if name.lower().endswith(".png"):
            flagged.append("frames: %s does not match either frame "
                           "naming scheme (frame_%%05d.png / "
                           "t<time>.png) -- ignored by sequence "
                           "consumers" % name)
        else:
            flagged.append("frames: unexpected non-PNG file %s" % name)

    if not indexed and not staged:
        problems.append("frames: no PNG frames found in %s (expected "
                        "frame_%%05d.png cache frames or t<time>.png "
                        "staged frames)" % frames_dir)
        return problems, flagged

    if indexed:
        indices = sorted(indexed)
        expected = expect_frames
        if expected is None:
            expected = info.get("total_frames")
        if expected is not None and len(indices) != expected:
            problems.append(
                "frames: found %d frame_<n>.png files, expected %d "
                "(%s) -- a partial/ranged render must be validated "
                "with --expect-frames set to its range length"
                % (len(indices), expected,
                   "--expect-frames" if expect_frames is not None
                   else "manifest totalFrames"))
        lo, hi = indices[0], indices[-1]
        present = set(indices)
        missing = [n for n in range(lo, hi + 1) if n not in present]
        if missing:
            shown = ", ".join(str(n) for n in missing[:50])
            problems.append(
                "frames: gap in frame numbering -- %d frame(s) "
                "missing between frame_%05d.png and frame_%05d.png: "
                "%s%s" % (len(missing), lo, hi, shown,
                          " ..." if len(missing) > 50 else ""))
        total = info.get("total_frames")
        if total is not None:
            over = [n for n in indices if n >= total]
            if over:
                problems.append(
                    "frames: %d frame(s) at index >= manifest "
                    "totalFrames (%d): first is frame_%05d.png -- "
                    "stale frames from a longer cut?"
                    % (len(over), total, over[0]))
        for n in indices:
            bad = png_problem(indexed[n], want_w, want_h)
            if bad:
                problems.append("frames: frame_%05d.png is %s"
                                % (n, bad))
    if staged:
        for when in sorted(staged):
            bad = png_problem(staged[when], want_w, want_h)
            if bad:
                problems.append("frames: t%.2f.png is %s" % (when, bad))
    return problems, flagged


# ---------------------------------------------------------------------
# Reporting / CLI
# ---------------------------------------------------------------------

def report(project, frames_dir, expect_frames):
    problems, flagged, info = validate_project(project)
    if frames_dir is not None:
        f_problems, f_flagged = validate_frames(frames_dir,
                                                expect_frames, info)
        problems.extend(f_problems)
        flagged.extend(f_flagged)

    print("validate-native: project %s" % os.path.abspath(project))
    if info:
        bits = []
        if info.get("title"):
            bits.append("title %r" % info["title"])
        if info.get("fps"):
            bits.append("%d fps" % info["fps"])
        if info.get("total_frames"):
            bits.append("%d frames" % info["total_frames"])
        if info.get("duration_s"):
            bits.append("%.3fs" % info["duration_s"])
        if info.get("width"):
            bits.append("%dx%d" % (info["width"], info["height"]))
        if info.get("segments"):
            bits.append("%d segments/shots" % info["segments"])
        if info.get("lines"):
            bits.append("%d lines" % info["lines"])
        if info.get("captions"):
            bits.append("%d caption strips referenced"
                        % info["captions"])
        if bits:
            print("  info: " + " - ".join(bits))
    if frames_dir is not None:
        print("  frames dir: %s" % os.path.abspath(frames_dir))
    for msg in flagged:
        print("  FLAGGED: %s" % msg)
    if problems:
        print("problems (%d):" % len(problems))
        for msg in problems:
            print("  - %s" % msg)
        print("RESULT: FAILED (%d problem(s), %d flagged)"
              % (len(problems), len(flagged)))
        return 2
    print("RESULT: OK (%d flagged)" % len(flagged))
    return 0


# ---------------------------------------------------------------------
# --self-test fixtures
# ---------------------------------------------------------------------

def _png_bytes(width, height):
    """A minimal structurally valid PNG (stdlib zlib/struct only)."""
    def chunk(tag, payload):
        return struct.pack(">I", len(payload)) + tag + payload \
            + struct.pack(">I", binascii.crc32(tag + payload)
                          & 0xFFFFFFFF)
    ihdr = struct.pack(">IIBBBBB", width, height, 8, 2, 0, 0, 0)
    raw = b"".join(b"\x00" + b"\x00\x00\x00" * width
                   for _ in range(height))
    return PNG_SIG + chunk(b"IHDR", ihdr) \
        + chunk(b"IDAT", zlib.compress(raw)) + chunk(b"IEND", b"")


def _write(path, content, binary=False):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    mode = "wb" if binary else "w"
    with open(path, mode) as fh:
        fh.write(content)


def build_fixture(root, omit=None, frame_indices=(0, 1, 2),
                  plates_shape="manifest", f2=True, drop_asset=None,
                  manifest_driver=None, write_sha=False,
                  f2_wrong_type=False):
    """A minimal but complete CURRENT-GENERATION bridged project +
    rendered frames (CR-B2-06: manifest-shape plates.json, F2 film
    data, {start, end} lines — the build_bridge.py output shape).

    30 fps, 0.1 s, 3 frames, one segment, 64x64 (fixture-scale
    dimensions keep the temp files tiny; the manifest and the PNG
    headers agree, which is what the validator checks).

    plates_shape="flat" + f2=False reproduces the PREVIOUS bridge
    generation (the stale packaged Onoda shape) for the stale case.
    """
    proj = os.path.join(root, "proj")
    frames = os.path.join(root, "frames")

    def put(relpath, content, binary=False):
        if omit is not None and relpath == omit:
            return
        _write(os.path.join(proj, relpath), content, binary)

    put("script.json", json.dumps({
        "title": "Fixture", "voice": "test",
        "lines": [{"text": "One line.", "caps": "One | line."}]}))
    put(os.path.join("data", "lines.json"), json.dumps({
        "duration": 0.1, "voice": "test", "speed": 1.0,
        "lines": [{"i": 0, "text": "One line.", "start": 0.0,
                   "end": 0.09}]}))
    put(os.path.join("data", "words.json"), json.dumps(
        [{"w": "One", "line": 0, "s": 0.0, "e": 0.05},
         {"w": "line.", "line": 0, "s": 0.05, "e": 0.09}]))
    put(os.path.join("src", "shots.ts"),
        "// fixture staging\nexport const SHOTS = [];\n")
    put(os.path.join("tools", "capture_native.mjs"),
        "// fixture placeholder for the template adapter\n")
    # data/mix.wav -- a real (silent) WAV via the stdlib wave module.
    if omit != os.path.join("data", "mix.wav"):
        mix = os.path.join(proj, "data", "mix.wav")
        os.makedirs(os.path.dirname(mix), exist_ok=True)
        with wave.open(mix, "wb") as wav:
            wav.setnchannels(1)
            wav.setsampwidth(2)
            wav.setframerate(48000)
            wav.writeframes(b"\x00\x00" * 4800)
    manifest_doc = {
        "format": "talerender-film", "version": 1, "title": "Fixture",
        "fps": 30, "durationS": 0.1, "totalFrames": 3,
        "resolution": {"width": 64, "height": 64},
        "segments": [{"id": "s01", "scene": "scenes/film/s01.scene.json",
                      "lines": [0], "startS": 0.0, "endS": 0.1,
                      "startFrame": 0, "endFrame": 3,
                      "frames": 3}]}
    if manifest_driver is not None:
        manifest_doc["driver"] = manifest_driver
    put(os.path.join("native", "film.json"), json.dumps(manifest_doc))
    put(os.path.join("native", "scenes", "film", "s01.scene.json"),
        json.dumps({"format": "talerender-scene", "version": 2,
                    "name": "s01", "nodes": []}))
    film_data = {
        "duration": 0.1, "fps": 30, "frames": 3,
        "lines": [{"start": 0.0, "end": 0.09}],
        "anchors": {},
        "captions": [{"s": 0.0, "e": 0.09, "text": "One", "fit": 1.0,
                      "strip": "cap_000.png"}]}
    if f2:
        film_data.update({
            "calendarYears": ["1945", "1955", "1965", "1974"],
            "calendarDark": "calendar-dark.png", "sign": "sign.png"})
    if f2_wrong_type:
        # Present but malformed: a string where the loader
        # demands an array of strings (R1-03 self-test case 9).
        film_data["calendarYears"] = "1945"
    put(os.path.join("native", "film", "onoda.json"),
        json.dumps(film_data))
    plate_names = ("paper-map", "leaflet", "book-open", "book-closed",
                   "coconut")
    if plates_shape == "manifest":
        plates_doc = {"format": "talerender-plates", "version": 1,
                      "plates": [{"name": n, "file": "%s.png" % n,
                                  "origin": [0, 0]}
                                 for n in plate_names]}
    else:
        plates_doc = {n: [0, 0] for n in plate_names}
    put(os.path.join("native", "film", "assets", "plates.json"),
        json.dumps(plates_doc))
    png = _png_bytes(64, 64)
    for name in plate_names + ("calendar-1945", "calendar-1955",
                               "calendar-1965", "calendar-1974",
                               "calendar-dark", "sign", "cap_000"):
        if drop_asset is not None and name == drop_asset:
            continue
        put(os.path.join("native", "film", "assets", name + ".png"),
            png, binary=True)
    for n in frame_indices:
        _write(os.path.join(frames, "frame_%05d.png" % n), png,
               binary=True)
    if write_sha:
        # MANIFEST-SHA in build_bridge.py's spelling: sha256 of
        # every file under native/ except MANIFEST-SHA itself,
        # "<digest>  <relative path>", sorted, trailing newline.
        import hashlib as _hashlib
        native_root = os.path.join(proj, "native")
        entries = []
        for dirpath, _dirs, files in os.walk(native_root):
            for fn in sorted(files):
                if fn == "MANIFEST-SHA":
                    continue
                full = os.path.join(dirpath, fn)
                relp = os.path.relpath(full, native_root) \
                    .replace(os.sep, "/")
                h = _hashlib.sha256()
                with open(full, "rb") as fh:
                    h.update(fh.read())
                entries.append("%s  %s" % (h.hexdigest(), relp))
        _write(os.path.join(native_root, "MANIFEST-SHA"),
               "\n".join(sorted(entries)) + "\n")
    return proj, frames


def run_self_test():
    cases = []

    def run_cli(*cli_args):
        env = {k: v for k, v in os.environ.items() if k != "FPS"}
        proc = subprocess.run(
            [sys.executable, os.path.abspath(__file__)] + list(cli_args),
            capture_output=True, text=True, env=env)
        return proc.returncode, proc.stdout + proc.stderr

    with tempfile.TemporaryDirectory(prefix="validate-native-") as tmp:
        # Case 1: a valid project + complete frames dir passes.
        proj, frames = build_fixture(os.path.join(tmp, "valid"))
        rc, out = run_cli(proj, "--frames", frames)
        ok = rc == 0 and "RESULT: OK" in out
        cases.append(("valid project + complete frames", ok,
                      "exit %d (want 0)" % rc))
        if not ok:
            print(out)

        # Case 2: a gap in frame numbering fails and names the gap.
        proj, frames = build_fixture(os.path.join(tmp, "gap"),
                                     frame_indices=(0, 1, 3))
        rc, out = run_cli(proj, "--frames", frames)
        ok = rc == 2 and "gap in frame numbering" in out
        cases.append(("gap in frame numbering", ok,
                      "exit %d (want 2, gap reported)" % rc))
        if not ok:
            print(out)

        # Case 3: a missing required file fails and names the file.
        proj, frames = build_fixture(os.path.join(tmp, "missing"),
                                     omit="script.json")
        rc, out = run_cli(proj)
        ok = rc == 2 and "script.json" in out
        cases.append(("missing required file (script.json)", ok,
                      "exit %d (want 2, file named)" % rc))
        if not ok:
            print(out)

        # Case 4 (CR-B2-06, as amended by R1-03): the PREVIOUS
        # bridge generation (flat plates.json — the stale packaged
        # Onoda shape) fails as STALE with the re-bridge remedy,
        # and no output routes to Chrome. The fixture also lacks
        # the F2 keys, but post-decoupling that absence alone is
        # NOT the stale marker (case 7 proves it passes); the
        # flat plates.json generation shape is.
        proj, frames = build_fixture(os.path.join(tmp, "stale"),
                                     plates_shape="flat", f2=False)
        rc, out = run_cli(proj)
        ok = rc == 2 and "STALE BRIDGE" in out \
            and "build_bridge.py" in out and "Chrome" not in out
        cases.append(("stale previous-generation bridge rejected",
                      ok, "exit %d (want 2, STALE + re-bridge remedy, "
                      "no Chrome routing)" % rc))
        if not ok:
            print(out)

        # Case 5 (CR-B2-06): a plate file missing is named from the
        # DATA's declaration (manifest-shape plates.json), proving
        # the derivation is data-driven, not a hardcoded list.
        proj, frames = build_fixture(os.path.join(tmp, "noplate"),
                                     drop_asset="coconut")
        rc, out = run_cli(proj)
        ok = rc == 2 and "coconut.png" in out
        cases.append(("missing data-declared plate (coconut.png)",
                      ok, "exit %d (want 2, plate named)" % rc))
        if not ok:
            print(out)

        # Case 6 (CR-B2-06): an unbridged project fails naming the
        # bridge step as the remedy — never a Chrome suggestion.
        proj, frames = build_fixture(
            os.path.join(tmp, "unbridged"),
            omit=os.path.join("native", "film.json"))
        # Also remove the rest of native/ so the project is truly
        # unbridged (the fixture writes it file by file).
        import shutil as _shutil
        _shutil.rmtree(os.path.join(proj, "native"),
                       ignore_errors=True)
        rc, out = run_cli(proj)
        ok = rc == 2 and "not bridged" in out \
            and "build_bridge.py" in out and "Chrome" not in out
        cases.append(("unbridged project -> bridge remedy", ok,
                      "exit %d (want 2, build_bridge named, no "
                      "Chrome)" % rc))
        if not ok:
            print(out)

        # Case 7 (R1-03, REMEDIATION-CONTRACTS §4): a FRESH
        # scene-sequence bridge with NO calendarYears /
        # calendarDark / sign declarations and a matching
        # MANIFEST-SHA is current-generation — it must PASS and
        # must NOT be labelled STALE. Absence of the F2 keys is
        # a declaration of "no calendar / no sign", not drift.
        proj, frames = build_fixture(
            os.path.join(tmp, "fresh-no-f2"), f2=False,
            manifest_driver="scene-sequence", write_sha=True)
        rc, out = run_cli(proj)
        ok = rc == 0 and "RESULT: OK" in out and "STALE" not in out
        cases.append(("fresh bridge without calendar/sign passes",
                      ok, "exit %d (want 0, OK, no STALE)" % rc))
        if not ok:
            print(out)

        # Case 8 (R1-03): staleness still bites where it should —
        # a bridge whose files drifted from its MANIFEST-SHA is
        # STALE even when it declares no F2 keys. The drift here
        # is a trailing space appended to onoda.json after the
        # SHA was written (JSON stays valid; the hash does not).
        proj, frames = build_fixture(
            os.path.join(tmp, "sha-drift"), f2=False,
            manifest_driver="scene-sequence", write_sha=True)
        with open(os.path.join(proj, "native", "film",
                               "onoda.json"), "a",
                  encoding="utf-8") as fh:
            fh.write(" ")
        rc, out = run_cli(proj)
        ok = rc == 2 and "STALE BRIDGE" in out \
            and "MANIFEST-SHA" in out
        cases.append(("MANIFEST-SHA drift is STALE (no F2 keys)",
                      ok, "exit %d (want 2, STALE + MANIFEST-SHA)"
                      % rc))
        if not ok:
            print(out)

        # Case 9 (R1-03): a PRESENT-but-malformed F2 declaration
        # still fails BY NAME — decoupling made absence legal,
        # not malformed data.
        proj, frames = build_fixture(
            os.path.join(tmp, "f2-wrong-type"), f2=False,
            manifest_driver="scene-sequence", f2_wrong_type=True)
        rc, out = run_cli(proj)
        ok = rc == 2 and "calendarYears" in out
        cases.append(("malformed calendarYears fails by name",
                      ok, "exit %d (want 2, calendarYears named)"
                      % rc))
        if not ok:
            print(out)

    passed = 0
    for name, ok, detail in cases:
        print("self-test case %-38s %s (%s)"
              % (name, "PASS" if ok else "FAIL", detail))
        passed += 1 if ok else 0
    print("SELF-TEST: %s (%d/%d)"
          % ("PASS" if passed == len(cases) else "FAIL",
             passed, len(cases)))
    return 0 if passed == len(cases) else 1


def main(argv=None):
    parser = argparse.ArgumentParser(
        description="Validate a Tall Tale project for the TaleRender "
                    "native backend (read-only, stdlib only).")
    parser.add_argument("project", nargs="?",
                        help="Tall Tale project directory")
    parser.add_argument("--frames", metavar="DIR",
                        help="also validate a rendered output dir "
                             "(frame_<nnnnn>.png cache frames or "
                             "t<time>.png staged frames)")
    parser.add_argument("--expect-frames", metavar="N", type=int,
                        help="expected frame count for --frames "
                             "(default: the manifest's totalFrames)")
    parser.add_argument("--self-test", action="store_true",
                        help="build fixture projects in a temp dir "
                             "and verify the validator against them")
    args = parser.parse_args(argv)

    if args.self_test:
        return run_self_test()
    if not args.project:
        parser.print_usage(sys.stderr)
        print("validate-native: a project directory is required "
              "(or pass --self-test)", file=sys.stderr)
        return 2
    if args.expect_frames is not None and args.expect_frames <= 0:
        print("validate-native: --expect-frames must be positive",
              file=sys.stderr)
        return 2
    return report(args.project, args.frames, args.expect_frames)


if __name__ == "__main__":
    sys.exit(main())
