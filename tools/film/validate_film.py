#!/usr/bin/env python3
"""validate_film.py — structural validation of the Onoda film data set.

Validates the 14 segment scene files listed in scenes/film/film.json
(the manifest) against the format-v2 laws and the cross-file
consistency rules the wave works under:

  1. parse          every manifest segment file exists and parses
  2. version        format == 'talerender-scene' and version == 2
  3. f32            every number under `lights`, `camera`, and each
                    character node (nodes[] with type == 'character')
                    is checked against the §11 f32 data law in two
                    classes: a number that round-trips exactly
                    through f32 passes outright; a number that does
                    not must be a SOURCE LITERAL — a short decimal
                    (<= 6 significant digits) that appears as a
                    literal in the project's source tree
                    (<project>/src, grep by float value) — anything
                    else is a COMPUTED value written rounded and
                    FAILs, listed verbatim
  4. attachments    §14.5 R2: attachments live ON character nodes;
                    a top-level `attachments` key is an error
  5. captions       caption chunk ids across all present segment
                    files are contiguous and cover 0..65 exactly
                    once (id taken from `index`, else `cap_NNN` id)
  6. durations      segment durations x fps round to the manifest
                    totalFrames, and the manifest frame ranges tile
                    [0, totalFrames) with no gaps or overlaps
  7. atmosphere     ruling R5: scene files name atmosphere PRESETS,
                    not filmLooks keys. The preset each file declares
                    (top-level `atmosphere`, else `shot.atmosphere`)
                    must equal the manifest's preset for the segment,
                    and the (set, preset) pair must resolve to a
                    filmLook via scenes/atmospheres.json — each
                    filmLooks entry lists the shots it covers, so the
                    segment's shot id ('S' + manifest id minus its
                    leading 's') must appear in exactly the look the
                    manifest names

Usage: python3 tools/film/validate_film.py [--root DIR]
Exit code: 0 = all checks PASS, 1 = any check FAIL.
Output is deterministic (manifest segment order, sorted failures
by file then path within each check).
"""

import argparse
import json
import math
import os
import re
import struct
import sys

EXPECTED_CHUNKS = list(range(66))  # film/onoda.json carries 66 chunks


def f32(v):
    return struct.unpack("f", struct.pack("f", v))[0]


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


def chunk_id(entry):
    if isinstance(entry, dict):
        if "index" in entry:
            return entry["index"]
        if "id" in entry:
            m = re.search(r"(\d+)$", str(entry["id"]))
            if m:
                return int(m.group(1))
    return None


NUM_TOKEN = re.compile(r"-?\d+\.\d+(?:[eE][+-]?\d+)?"
                       r"|-?\.\d+(?:[eE][+-]?\d+)?"
                       r"|-?\d+(?:[eE][+-]?\d+)?")


def source_literal_values(src_dir):
    """Float values of every numeric token in the project's source tree.

    Grep semantics, as specified for the f32 check's SOURCE LITERAL
    class: tokens are matched wherever they appear (code, strings,
    comments). Returns (set_of_floats, files_scanned).
    """
    vals = set()
    nfiles = 0
    for dirpath, _dirs, files in os.walk(src_dir):
        for fn in sorted(files):
            try:
                text = open(os.path.join(dirpath, fn),
                            encoding="utf-8", errors="ignore").read()
            except OSError:
                continue
            nfiles += 1
            for tok in NUM_TOKEN.findall(text):
                try:
                    vals.add(float(tok))
                except ValueError:
                    pass
    return vals, nfiles


def sig_digits(v):
    """Significant digits of a JSON number's shortest decimal form."""
    r = repr(abs(float(v)))
    mant = r.split("e")[0].split("E")[0]
    digits = mant.replace(".", "").strip("0")
    return len(digits) if digits else 1


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--root", default=".",
                    help="repo root (default: current directory)")
    args = ap.parse_args()
    root = args.root

    manifest = json.load(open(os.path.join(root, "scenes/film/film.json")))
    atmos = json.load(open(os.path.join(root, "scenes/atmospheres.json")))
    filmlook_keys = sorted(k for k in atmos.get("filmLooks", {})
                           if not k.startswith("_"))
    fps = manifest["fps"]
    total_frames = manifest["totalFrames"]
    segments = manifest["segments"]

    results = []  # (name, ok, headline, [failure lines])

    # ---- load segment files -------------------------------------------------
    docs = {}       # seg id -> parsed doc
    missing = []    # seg ids whose file is absent
    parse_fail = []  # (seg id, error)
    for seg in segments:
        p = os.path.join(root, seg["scene"])
        if not os.path.exists(p):
            missing.append(seg["id"])
            continue
        try:
            docs[seg["id"]] = json.load(open(p))
        except Exception as e:  # noqa: BLE001 - report verbatim
            parse_fail.append((seg["id"], "%s: %s" % (seg["scene"], e)))

    lines = ["present: %d/%d segment files (%s)"
             % (len(docs) + len(parse_fail), len(segments),
                ", ".join(s for s in docs) or "-")]
    if missing:
        lines.append("missing: %s" % ", ".join(missing))
    for sid, err in parse_fail:
        lines.append("PARSE FAILURE %s -> %s" % (sid, err))
    ok = not missing and not parse_fail
    results.append(("1 parse", ok,
                    "%d present, %d missing, %d unparsable"
                    % (len(docs), len(missing), len(parse_fail)), lines))

    # ---- 2 version -----------------------------------------------------------
    bad = []
    for seg in segments:
        d = docs.get(seg["id"])
        if d is None:
            continue
        if d.get("format") != "talerender-scene" or d.get("version") != 2:
            bad.append("%s: format=%r version=%r (want 'talerender-scene', 2)"
                       % (seg["scene"], d.get("format"), d.get("version")))
    results.append(("2 version", not bad,
                    "%d files checked, %d wrong" % (len(docs), len(bad)), bad))

    # ---- 3 f32 data law (two classes) -----------------------------------------
    src_dir = os.path.join(manifest["project"], "src")
    src_literals, src_files = source_literal_values(src_dir)
    total_nums = 0
    n_exact = 0
    n_literal = 0
    bad = []
    for seg in segments:
        d = docs.get(seg["id"])
        if d is None:
            continue
        scopes = []
        if "lights" in d:
            scopes.append(("lights", d["lights"]))
        if "camera" in d:
            scopes.append(("camera", d["camera"]))
        for i, node in enumerate(d.get("nodes", [])):
            if isinstance(node, dict) and node.get("type") == "character":
                scopes.append(("nodes[%d] (character %r)"
                               % (i, node.get("name")), node))
        for name, scope in scopes:
            for path, v in numbers(scope, name):
                total_nums += 1
                fv = f32(float(v))
                if fv == float(v):
                    n_exact += 1
                elif sig_digits(v) <= 6 and float(v) in src_literals:
                    n_literal += 1
                else:
                    bad.append("%s: %s = %r -> f32 %r "
                               "(computed value, not a source literal)"
                               % (seg["scene"], path, v, fv))
    bad.sort()
    lines = list(bad)
    if not src_files:
        lines.append("NOTE: no source files scanned under %s — the "
                     "SOURCE LITERAL class is empty for this run" % src_dir)
    results.append(("3 f32", not bad,
                    "%d numbers checked: %d exact f32, %d source literals "
                    "(pass), %d computed values not round-tripping (fail); "
                    "literals grepped from %d source files under %s"
                    % (total_nums, n_exact, n_literal, len(bad),
                       src_files, src_dir), lines))

    # ---- 4 attachments placement (§14.5 R2) -----------------------------------
    bad = []
    on_node = 0
    for seg in segments:
        d = docs.get(seg["id"])
        if d is None:
            continue
        if "attachments" in d:
            bad.append("%s: top-level 'attachments' key present (%d entries)"
                       " — R2 requires attachments ON character nodes"
                       % (seg["scene"], len(d["attachments"])))
        for node in d.get("nodes", []):
            if isinstance(node, dict) and node.get("type") == "character":
                on_node += len(node.get("attachments", []))
    results.append(("4 attachments", not bad,
                    "%d top-level violations, %d attachments on character nodes"
                    % (len(bad), on_node), bad))

    # ---- 5 captions coverage ---------------------------------------------------
    seen = {}
    bad = []
    for seg in segments:
        d = docs.get(seg["id"])
        if d is None:
            continue
        for entry in d.get("captions", []):
            cid = chunk_id(entry)
            if cid is None:
                bad.append("%s: caption entry without index/id: %r"
                           % (seg["scene"], entry))
                continue
            seen.setdefault(cid, []).append(seg["id"])
    for cid in sorted(seen):
        if len(seen[cid]) > 1:
            bad.append("chunk %d listed %d times (in %s)"
                       % (cid, len(seen[cid]), ", ".join(seen[cid])))
    absent = [c for c in EXPECTED_CHUNKS if c not in seen]
    if absent:
        bad.append("chunks not covered by any present segment file: %s"
                   % ", ".join(str(c) for c in absent))
    extra = [c for c in sorted(seen) if c not in EXPECTED_CHUNKS]
    if extra:
        bad.append("chunk ids outside 0..65: %s"
                   % ", ".join(str(c) for c in extra))
    results.append(("5 captions", not bad,
                    "%d chunk listings, %d unique chunks covered of 66, "
                    "%d duplicates, %d uncovered"
                    % (sum(len(v) for v in seen.values()), len(seen),
                       sum(1 for v in seen.values() if len(v) > 1),
                       len(absent)), bad))

    # ---- 6 durations & frame tiling --------------------------------------------
    bad = []
    dur_sum = sum(s["endS"] - s["startS"] for s in segments)
    rounded = round(dur_sum * fps)
    if rounded != total_frames:
        bad.append("sum(durations) x fps = %r x %d -> %d frames, manifest "
                   "totalFrames = %d" % (dur_sum, fps, rounded, total_frames))
    expect = 0
    for s in segments:
        if s["startFrame"] != expect:
            bad.append("segment %s starts at frame %d, expected %d (gap/overlap)"
                       % (s["id"], s["startFrame"], expect))
        if s["endFrame"] - s["startFrame"] != s["frames"]:
            bad.append("segment %s: endFrame-startFrame = %d != frames = %d"
                       % (s["id"], s["endFrame"] - s["startFrame"], s["frames"]))
        expect = s["endFrame"]
    if expect != total_frames:
        bad.append("segments end at frame %d, manifest totalFrames = %d"
                   % (expect, total_frames))
    # per-segment frame count follows the manifest frame rule
    for s in segments:
        want = math.ceil(s["endS"] * fps - 1e-9) - math.ceil(s["startS"] * fps - 1e-9)
        if s["id"] == segments[-1]["id"]:
            want = total_frames - math.ceil(s["startS"] * fps - 1e-9)
        if want != s["frames"]:
            bad.append("segment %s: frame rule gives %d frames, manifest says %d"
                       % (s["id"], want, s["frames"]))
    results.append(("6 durations", not bad,
                    "sum(durations) = %r s -> %d frames (total %d); tiling %s"
                    % (dur_sum, rounded, total_frames,
                       "exact" if not bad else "broken"), bad))

    # ---- 7 atmosphere (ruling R5: presets resolve via filmLooks) -----------------
    # Each filmLooks entry lists the shots it covers; that list is the
    # (set, preset) resolution table — a segment's shot id must appear
    # in exactly the filmLook the manifest names for it.
    shot_to_look = {}
    for key in filmlook_keys:
        entry = atmos["filmLooks"][key]
        if isinstance(entry, dict):
            for shot in entry.get("shots", []):
                shot_to_look[shot] = key
    bad = []
    info = []
    n_preset_ok = 0
    n_resolved = 0
    for seg in segments:
        shot_id = "S" + seg["id"][1:]
        derived = shot_to_look.get(shot_id)
        if derived is not None and derived == seg["filmLook"]:
            n_resolved += 1
            info.append("%s: set %s x preset %r -> filmLook %r "
                        "(atmospheres.json shots list)"
                        % (seg["id"], seg["set"], seg["atmosphere"], derived))
        else:
            bad.append("segment %s: (set %s, preset %r) resolves to %r via "
                       "atmospheres.json, manifest names %r"
                       % (seg["id"], seg["set"], seg["atmosphere"],
                          derived, seg["filmLook"]))
        d = docs.get(seg["id"])
        if d is None:
            continue
        declared = d.get("atmosphere")
        if declared is None and isinstance(d.get("shot"), dict):
            declared = d["shot"].get("atmosphere")
        if declared == seg["atmosphere"]:
            n_preset_ok += 1
        else:
            bad.append("%s: declared atmosphere preset %r, manifest "
                       "preset is %r (R5: files name presets)"
                       % (seg["scene"], declared, seg["atmosphere"]))
    bad.sort()
    results.append(("7 atmosphere", not bad,
                    "R5: declared presets match the manifest for %d/%d "
                    "files; (set, preset) -> filmLook resolves via "
                    "atmospheres.json for %d/%d segments"
                    % (n_preset_ok, len(docs), n_resolved, len(segments)),
                    bad + ["-- derived mapping (context) --"] + info))

    # ---- report -------------------------------------------------------------------
    print("validate_film.py — %s (%d segments, %d fps, %d frames)"
          % (manifest["title"], len(segments), fps, total_frames))
    all_ok = True
    for name, ok, headline, failures in results:
        all_ok = all_ok and ok
        print("\n[%s] %s — %s" % (name, "PASS" if ok else "FAIL", headline))
        for line in failures:
            print("    " + line)
    print("\nOVERALL: %s (%d/%d checks passed)"
          % ("PASS" if all_ok else "FAIL",
             sum(1 for _, ok, _, _ in results if ok), len(results)))
    return 0 if all_ok else 1


if __name__ == "__main__":
    sys.exit(main())
