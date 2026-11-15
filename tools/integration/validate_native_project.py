#!/usr/bin/env python3
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
#   - The adapter's current fixed asset list (calendar plates +
#     sign, hardcoded in capture_native.mjs validateFilmData) is
#     Onoda-bridge-shaped; this validator checks it because the
#     adapter requires it today, and labels those checks as
#     adapter-required. A generalised bridge will need both the
#     adapter and this list revisited together.
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

# Assets capture_native.mjs validateFilmData() requires by name, on
# top of whatever plates.json / caption strips declare. The hold
# plates come from plates.json keys (checked generically); these
# calendar / sign plates are the adapter's hardcoded remainder.
ADAPTER_FIXED_ASSETS = [
    "calendar-1945.png",
    "calendar-1955.png",
    "calendar-1965.png",
    "calendar-1974.png",
    "calendar-dark.png",
    "sign.png",
]

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
                       "skill template; copy the template tools (or "
                       "re-derive the project) before invoking the "
                       "native path from this folder. The Chrome "
                       "adapter (tools/capture.mjs) is unaffected.")

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
                        "unbridged projects, capture with Chrome")
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
                        "adapter never improvises film data")
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
        if not isinstance(data.get("anchors"), dict):
            problems.append("onoda.json: anchors{} missing")
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
                if isinstance(plates, dict):
                    for name in sorted(plates):
                        png = os.path.join(assets_dir, "%s.png" % name)
                        if not os.path.isfile(png):
                            problems.append(
                                "film data: plate %s.png missing "
                                "(declared in plates.json)" % name)
                elif plates is not None:
                    problems.append("assets/plates.json: top level is "
                                    "not a JSON object")
            for name in ADAPTER_FIXED_ASSETS:
                if not os.path.isfile(os.path.join(assets_dir, name)):
                    problems.append(
                        "film data: assets/%s missing "
                        "(adapter-required: capture_native.mjs "
                        "validateFilmData checks this name today)"
                        % name)
            for strip in sorted(set(caption_strips)):
                if not os.path.isfile(os.path.join(assets_dir, strip)):
                    problems.append(
                        "film data: caption strip assets/%s missing "
                        "(named by onoda.json captions[])" % strip)

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


def build_fixture(root, omit=None, frame_indices=(0, 1, 2)):
    """A minimal but complete bridged project + rendered frames.

    30 fps, 0.1 s, 3 frames, one segment, 64x64 (fixture-scale
    dimensions keep the temp files tiny; the manifest and the PNG
    headers agree, which is what the validator checks).
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
    put(os.path.join("native", "film.json"), json.dumps({
        "format": "talerender-film", "version": 1, "title": "Fixture",
        "fps": 30, "durationS": 0.1, "totalFrames": 3,
        "resolution": {"width": 64, "height": 64},
        "segments": [{"id": "s01", "scene": "scenes/film/s01.scene.json",
                      "lines": [0], "startFrame": 0, "endFrame": 3,
                      "frames": 3}]}))
    put(os.path.join("native", "scenes", "film", "s01.scene.json"),
        json.dumps({"format": "talerender-scene", "version": 2,
                    "name": "s01", "nodes": []}))
    put(os.path.join("native", "film", "onoda.json"), json.dumps({
        "duration": 0.1, "fps": 30, "frames": 3,
        "lines": [{"i": 0, "start": 0.0, "end": 0.09,
                   "text": "One line."}],
        "anchors": {},
        "captions": [{"line": 0, "s": 0.0, "e": 0.09, "text": "One",
                      "strip": "cap_000.png"}]}))
    put(os.path.join("native", "film", "assets", "plates.json"),
        json.dumps({"paper-map": [0, 0], "leaflet": [0, 0],
                    "book-open": [0, 0], "book-closed": [0, 0],
                    "coconut": [0, 0]}))
    png = _png_bytes(64, 64)
    for name in ("paper-map", "leaflet", "book-open", "book-closed",
                 "coconut", "calendar-1945", "calendar-1955",
                 "calendar-1965", "calendar-1974", "calendar-dark",
                 "sign", "cap_000"):
        put(os.path.join("native", "film", "assets", name + ".png"),
            png, binary=True)
    for n in frame_indices:
        _write(os.path.join(frames, "frame_%05d.png" % n), png,
               binary=True)
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
