#!/usr/bin/env python3
"""check_assets.py — asset-reference integrity for TaleRender scene
and film data (W28; spec: docs/integration/ASSETS-W28.md).

Scene and film JSON reference files outside themselves: texture
images, node data files, caption strips, plate PNGs, the film
manifest's segment scenes and source data, and one external
project tree. Until now nothing verified those references as a
set: a missing asset surfaces as a render-time failure deep in a
load, and two loader behaviours are silent (a plate missing from
plates.json gets origin (0, 0); a non-object plates.json loads no
plates at all — src/film.rs load_assets). This tool walks every
reference the loaders actually read and checks it against the
LOADER's own resolution rule — never against a guessed one:

  scene textures[].file   scene file's directory
                          (src/scenefile.rs:974 scene_dir, :411 join;
                          docs/scene-format.md §"file")
  scene node "ref"        scene file's directory (data-layer
                          pointer; the Rust loader does not read
                          "ref" today — painters/calendar tooling
                          does — checked anyway: the data names it)
  film onoda.json         <film dir>/onoda.json (src/film.rs:64)
  film assets             <film dir>/assets/ + the reference:
                          plates.json, five plate PNGs by hardcoded
                          name, captions[].strip, four calendar
                          years by hardcoded name, calendar-dark.png,
                          sign.png (src/film.rs:2045-2077)
  manifest segments[].scene, sourceData
                          --root (default: the current directory) —
                          the validate_film.py convention
                          (tools/film/validate_film.py:231, :189);
                          CWD-dependent, always flagged in check 4
  manifest project, audio.projectPath
                          absolute paths, used verbatim; existence
                          checked, portability flagged in check 4
  manifest audio.file     under manifest project

Checks:
  1. parse        the target parses / the directory layout is a
                  recognised kind (driver film dir with onoda.json,
                  or manifest film dir with film.json); every
                  referenced JSON file parses
  2. references   every reference the loaders read resolves to an
                  existing file under its own resolution rule.
                  References rooted in an absolute machine path
                  (the manifest's `project` and everything resolved
                  under it) are the exception: they can only exist
                  on the authoring machine, so check 2 does not
                  existence-check them — check 4 classifies them
                  as portability warnings instead. A manifest that
                  only resolves at home must not fail CI on any
                  other machine, while a missing RELATIVE
                  reference still fails here (W28 F2; wave-2
                  urgent fix, see scenes/fixtures/assets/abs-path)
  3. kinds        referenced files are the kind their name and the
                  loader imply: PNG magic bytes for .png (the
                  loaders hand them to image::open), RIFF/WAVE for
                  .wav, font magic for .otf/.ttf
  4. portability  WARNINGS ONLY (never fail): references whose
                  resolution depends on the caller's CWD / --root,
                  and absolute machine paths
  5. silent-fallback guards
                  the loader's silent behaviours, made loud:
                  plates.json must be an object carrying all five
                  plate names with [x, y] origins; a scene's
                  texture ids must be unique (the loader's
                  tex_index insert overwrites silently,
                  src/scenefile.rs:985)
  6. orphans      REPORT ONLY (never fail): asset files in scope
                  (a driver dir's assets/, a manifest dir's own
                  files) that no reference names

This tool checks that referenced files EXIST and are the right
KIND. Whether their CONTENT is right (pixels, timings, origins)
is the A/B + telemetry gates' job, not this tool's.

Usage: python3 tools/format/check_assets.py [--root DIR]
           TARGET [TARGET ...]
       TARGET = a scene .json file, a driver film dir (holds
       onoda.json), or a manifest film dir (holds film.json).
Exit code: 0 = all checks PASS for every target, 1 = any check
FAIL, 2 = input error (a target is unreadable, invalid JSON, or
an unrecognised layout — one-line ERROR on stderr, no report).
Warnings (check 4) and orphans (check 6) never change the exit
code. Output is deterministic (command-line target order, checks
in the order above, failures in document order within a check).
"""

import argparse
import json
import os
import sys

# The film driver's hardcoded asset names (src/film.rs:2051,
# 2069-2077). Not data — the loader reads exactly these.
PLATE_NAMES = ["paper-map", "leaflet", "book-open", "book-closed",
               "coconut"]
CALENDAR_YEARS = ["1945", "1955", "1965", "1974"]
FIXED_ASSETS = (["plates.json", "calendar-dark.png", "sign.png"]
                + ["calendar-%s.png" % y for y in CALENDAR_YEARS])

PNG_MAGIC = b"\x89PNG\r\n\x1a\n"
IMAGE_MAGICS = (PNG_MAGIC, b"\xff\xd8\xff", b"GIF87a", b"GIF89a",
                b"BM")
ASSET_EXTS = {".png", ".json", ".wav", ".otf", ".ttf", ".f16"}


class InputError(Exception):
    """Bad target — the run cannot start (exit 2, no report)."""


def load_json_input(path, what):
    try:
        with open(path) as f:
            return json.load(f)
    except OSError as e:
        raise InputError("%s: cannot read %s (%s)"
                         % (what, path, e.strerror or e))
    except ValueError as e:
        raise InputError("%s: invalid JSON in %s (%s)" % (what, path, e))


def is_number(v):
    return isinstance(v, (int, float)) and not isinstance(v, bool)


class Ref:
    """One asset reference: where it is written, the raw string,
    how the loader resolves it, and the kind it must be."""

    def __init__(self, where, raw, base_desc, resolved, kind,
                 cwd_dependent=False, absolute=False, want_dir=False):
        self.where = where          # e.g. "textures[0].file"
        self.raw = raw              # the string as written
        self.base_desc = base_desc  # human name of the base
        self.resolved = resolved    # path the loader would open
        self.kind = kind            # "png" | "json" | "wav" | "any"
        self.cwd_dependent = cwd_dependent
        self.absolute = absolute
        # the manifest's `project` names a directory, not a file
        self.exists = (os.path.isdir(resolved) if want_dir
                       else os.path.isfile(resolved))


def kind_failure(ref):
    """None when the resolved file's bytes match its implied kind,
    else a failure line. Existence is check 2's job; absent files
    are not re-reported here."""
    if not ref.exists or ref.kind == "any":
        return None
    try:
        with open(ref.resolved, "rb") as f:
            head = f.read(16)
    except OSError as e:
        return "%s: cannot read %s (%s)" % (ref.where, ref.resolved,
                                            e.strerror or e)
    if ref.kind == "png":
        if not head.startswith(PNG_MAGIC):
            return ("%s: %s is named .png but does not start with "
                    "the PNG magic bytes (first bytes: %s) — the "
                    "loader hands it to image::open as an image"
                    % (ref.where, ref.resolved,
                       head[:8].hex(" ") or "empty file"))
    elif ref.kind == "image":
        if not any(head.startswith(m) for m in IMAGE_MAGICS) \
                and not (head.startswith(b"RIFF")
                         and head[8:12] == b"WEBP"):
            return ("%s: %s carries no recognised image header "
                    "(PNG/JPEG/GIF/BMP/WebP) — image::open would "
                    "reject it" % (ref.where, ref.resolved))
    elif ref.kind == "wav":
        if not (head.startswith(b"RIFF") and head[8:12] == b"WAVE"):
            return ("%s: %s is named .wav but has no RIFF/WAVE "
                    "header" % (ref.where, ref.resolved))
    elif ref.kind == "font":
        if not (head[:4] in (b"OTTO", b"\x00\x01\x00\x00", b"true",
                             b"ttcf")):
            return ("%s: %s carries no font magic (OTTO / TrueType)"
                    % (ref.where, ref.resolved))
    elif ref.kind == "json":
        try:
            with open(ref.resolved) as f:
                json.load(f)
        except ValueError as e:
            return ("%s: %s does not parse as JSON (%s)"
                    % (ref.where, ref.resolved, e))
        except OSError as e:
            return ("%s: cannot read %s (%s)"
                    % (ref.where, ref.resolved, e.strerror or e))
    return None


def walk_node_refs(nodes, path, out):
    """Collect (path, ref-string) for every "ref" field anywhere
    in a scene document (under nodes, under the top-level
    "overlays" block — s07's calendar node lives there — or any
    other nesting), recursively."""
    if isinstance(nodes, list):
        for i, n in enumerate(nodes):
            walk_node_refs(n, "%s[%d]" % (path, i), out)
    elif isinstance(nodes, dict):
        if isinstance(nodes.get("ref"), str):
            out.append(("%s.ref" % path, nodes["ref"]))
        for k, v in nodes.items():
            if isinstance(v, (dict, list)):
                walk_node_refs(v, "%s.%s" % (path, k), out)


def scene_refs(doc, scene_path):
    """(refs, guard_failures) for one parsed scene document.
    Resolution base: the scene file's own directory
    (src/scenefile.rs:974, :411)."""
    scene_dir = os.path.dirname(os.path.abspath(scene_path))
    refs, guards = [], []
    textures = doc.get("textures")
    seen_ids = {}
    if isinstance(textures, list):
        for i, spec in enumerate(textures):
            where = "textures[%d]" % i
            if not isinstance(spec, dict):
                guards.append("%s: texture entry is not an object — "
                              "the loader rejects it" % where)
                continue
            tid = spec.get("id")
            if isinstance(tid, str):
                if tid in seen_ids:
                    guards.append(
                        "%s.id: duplicate texture id %r (first at "
                        "textures[%d]) — the loader's tex_index "
                        "insert overwrites silently "
                        "(src/scenefile.rs:985); the first texture "
                        "is unreachable" % (where, tid, seen_ids[tid]))
                else:
                    seen_ids[tid] = i
            if "file" in spec:
                raw = spec["file"]
                if not isinstance(raw, str):
                    guards.append("%s.file: not a string" % where)
                    continue
                kind = "png" if raw.lower().endswith(".png") else "image"
                refs.append(Ref("%s.file" % where, raw,
                                "scene file's directory",
                                os.path.normpath(
                                    os.path.join(scene_dir, raw)),
                                kind))
    node_refs = []
    walk_node_refs(doc, "", node_refs)
    for where, raw in node_refs:
        where = where.lstrip(".")
        kind = "json" if raw.lower().endswith(".json") else "any"
        refs.append(Ref(where, raw, "scene file's directory",
                        os.path.normpath(os.path.join(scene_dir, raw)),
                        kind))
    return refs, guards


def report(title, results):
    """results: [(name, ok-or-None, headline, [lines])]; None =
    SKIPPED. Returns all_ok."""
    lines = [title]
    all_ok = True
    n_pass = n_total = 0
    for name, ok, headline, failures in results:
        if ok is None:
            lines.append("\n[%s] SKIPPED — %s" % (name, headline))
            continue
        n_total += 1
        n_pass += 1 if ok else 0
        all_ok = all_ok and ok
        lines.append("\n[%s] %s — %s"
                     % (name, "PASS" if ok else "FAIL", headline))
        for line in failures:
            lines.append("    " + line)
    lines.append("\nOVERALL: %s (%d/%d checks passed)"
                 % ("PASS" if all_ok else "FAIL", n_pass, n_total))
    print("\n".join(lines))
    return all_ok


def evaluate(refs, guards, orphan_files, orphan_scope, mode_note):
    """Shared checks 2-6 over a collected reference set."""
    results = []
    # Absolute machine paths are check 4's classification, not
    # check 2 failures: they name the authoring machine's tree
    # and cannot exist anywhere else (W28 F2). Everything else
    # that is missing fails here, relative references included.
    missing = ["%s: %r (%s) -> %s — not found"
               % (r.where, r.raw, r.base_desc, r.resolved)
               for r in refs if not r.exists and not r.absolute]
    n_machine = sum(1 for r in refs if r.absolute)
    headline2 = "%d references walked (%s)" % (len(refs), mode_note)
    if n_machine:
        headline2 += ("; %d rooted in absolute machine paths — "
                      "classified in check 4, not existence-checked"
                      % n_machine)
    results.append(("2 references", not missing, headline2, missing))
    bad_kinds = [k for k in (kind_failure(r) for r in refs) if k]
    results.append(("3 kinds", not bad_kinds,
                    "magic-byte / parse sanity of every referenced "
                    "file", bad_kinds))
    warns = []
    for r in refs:
        if r.cwd_dependent:
            warns.append("WARN: %s: %r resolves against %s — its "
                         "resolution depends on the caller's working "
                         "directory / --root; run from elsewhere "
                         "without --root and it breaks"
                         % (r.where, r.raw, r.base_desc))
        if r.absolute:
            warns.append("WARN: %s: %r is an absolute machine path "
                         "— portable only to the machine that "
                         "authored it" % (r.where, r.raw))
    results.append(("4 portability", True,
                    "%d warning(s), report-only" % len(warns), warns))
    results.append(("5 silent-fallback guards",
                    not guards if guards is not None else None,
                    "loader-silent behaviours, made loud"
                    if guards is not None else
                    "no loader-silent fallback surface in this mode",
                    guards or []))
    orphans = ["orphan: %s — present in %s, referenced by nothing"
               % (p, orphan_scope) for p in orphan_files]
    results.append(("6 orphans", True,
                    "%d orphan(s), report-only" % len(orphans),
                    orphans))
    return results


def check_scene_file(path):
    doc = load_json_input(path, "scene")
    if not isinstance(doc, dict):
        raise InputError("scene: %s top level must be a JSON object"
                         % path)
    if doc.get("format") == "talerender-film":
        return check_manifest(os.path.dirname(os.path.abspath(path)),
                              doc, path)
    refs, guards = scene_refs(doc, path)
    results = [("1 parse", True,
                "%s parses as a JSON object; scene-directory "
                "resolution (src/scenefile.rs:974)" % path, [])]
    results += evaluate(refs, guards, [], "", "scene-file mode")
    # orphans are meaningless for a lone scene file: drop check 6
    results[-1] = ("6 orphans", None,
                   "a lone scene file has no assets directory to "
                   "orphan-scan", [])
    return report("check_assets.py — scene %s" % path, results)


def check_driver(d):
    data_path = os.path.join(d, "onoda.json")
    data = load_json_input(data_path, "film data")
    if not isinstance(data, dict):
        raise InputError("film data: %s top level must be a JSON "
                         "object" % data_path)
    adir = os.path.join(d, "assets")
    refs = [Ref("onoda.json (the film data itself)", "onoda.json",
                "film dir", data_path, "json")]
    captions = data.get("captions")
    guards = []
    if isinstance(captions, list):
        for i, c in enumerate(captions):
            strip = c.get("strip") if isinstance(c, dict) else None
            if not isinstance(strip, str) or not strip:
                guards.append(
                    "captions[%d].strip: missing or empty — "
                    "src/film.rs:89 defaults it to \"\" and "
                    "load_png(assets/) then fails without naming "
                    "the caption" % i)
                continue
            refs.append(Ref("captions[%d].strip" % i, strip,
                            "film dir assets/",
                            os.path.join(adir, strip), "png"))
    for name in FIXED_ASSETS:
        kind = "json" if name.endswith(".json") else "png"
        refs.append(Ref("assets/%s (loader-fixed name)" % name, name,
                        "film dir assets/", os.path.join(adir, name),
                        kind))
    for name in PLATE_NAMES:
        refs.append(Ref("assets/%s.png (loader-fixed name)" % name,
                        "%s.png" % name, "film dir assets/",
                        os.path.join(adir, "%s.png" % name), "png"))
    # check 5: plates.json completeness (src/film.rs:2047-2064)
    plates_path = os.path.join(adir, "plates.json")
    if os.path.isfile(plates_path):
        try:
            with open(plates_path) as f:
                plates = json.load(f)
        except ValueError:
            plates = None  # check 3 reports the parse failure
        if plates is not None:
            if not isinstance(plates, dict):
                guards.append(
                    "plates.json: top level is not an object — "
                    "src/film.rs:2050 loads NO plates at all in "
                    "that case, silently (the plate PNGs are never "
                    "even opened)")
            else:
                for name in PLATE_NAMES:
                    v = plates.get(name)
                    if not (isinstance(v, list) and len(v) == 2
                            and all(is_number(x) for x in v)):
                        guards.append(
                            "plates.json: no [x, y] origin for %r — "
                            "src/film.rs:2054-2061 silently defaults "
                            "a missing entry to (0, 0)" % name)
    referenced = {os.path.normpath(r.resolved) for r in refs}
    orphans = []
    if os.path.isdir(adir):
        for fn in sorted(os.listdir(adir)):
            full = os.path.join(adir, fn)
            if os.path.isfile(full) \
                    and os.path.normpath(full) not in referenced:
                orphans.append(full)
    results = [("1 parse", True,
                "%s parses; driver layout (film dir + assets/, "
                "src/film.rs:64, :2046)" % data_path, [])]
    results += evaluate(refs, guards, orphans, "the film assets dir",
                        "driver film-dir mode")
    return report("check_assets.py — film dir %s (driver)" % d,
                  results)


def check_manifest(d, manifest, manifest_path, root):
    refs = []
    guards = None  # no manifest-level silent-fallback surface
    segments = manifest.get("segments")
    seg_docs = []
    if isinstance(segments, list):
        for i, seg in enumerate(segments):
            if not isinstance(seg, dict) \
                    or not isinstance(seg.get("scene"), str):
                continue
            raw = seg["scene"]
            resolved = os.path.normpath(os.path.join(root, raw))
            refs.append(Ref("segments[%d].scene" % i, raw,
                            "--root %s" % root, resolved, "json",
                            cwd_dependent=True))
            if os.path.isfile(resolved):
                try:
                    with open(resolved) as f:
                        seg_docs.append((resolved, json.load(f)))
                except ValueError:
                    pass  # check 3 reports the parse failure
    if isinstance(manifest.get("sourceData"), str):
        raw = manifest["sourceData"]
        refs.append(Ref("sourceData", raw, "--root %s" % root,
                        os.path.normpath(os.path.join(root, raw)),
                        "json", cwd_dependent=True))
    project = manifest.get("project")
    audio = manifest.get("audio")
    if isinstance(project, str):
        refs.append(Ref("project", project, "absolute path",
                        project, "any",
                        absolute=os.path.isabs(project),
                        want_dir=True))
    if isinstance(audio, dict):
        if isinstance(audio.get("projectPath"), str):
            raw = audio["projectPath"]
            refs.append(Ref("audio.projectPath", raw, "absolute path",
                            raw, "wav",
                            absolute=os.path.isabs(raw)))
        if isinstance(audio.get("file"), str) \
                and isinstance(project, str):
            raw = audio["file"]
            refs.append(Ref("audio.file", raw,
                            "the manifest's project dir",
                            os.path.normpath(os.path.join(project,
                                                          raw)),
                            "wav",
                            # resolved under `project`: when that
                            # is an absolute machine path, so is
                            # this reference (check 4's class)
                            absolute=os.path.isabs(project)))
    # segment scenes contribute their own references (scene rules)
    for spath, sdoc in seg_docs:
        if isinstance(sdoc, dict):
            srefs, sguards = scene_refs(sdoc, spath)
            for r in srefs:
                r.where = "%s: %s" % (os.path.basename(spath), r.where)
            refs.extend(srefs)
            if sguards:
                guards = (guards or []) + [
                    "%s: %s" % (os.path.basename(spath), g)
                    for g in sguards]
    referenced = {os.path.normpath(r.resolved) for r in refs}
    orphans = []
    for fn in sorted(os.listdir(d)):
        full = os.path.join(d, fn)
        if not os.path.isfile(full):
            continue
        if os.path.splitext(fn)[1].lower() not in ASSET_EXTS:
            continue
        if os.path.normpath(full) == os.path.normpath(manifest_path):
            continue  # the manifest references everything, not itself
        if os.path.normpath(full) not in referenced:
            orphans.append(full)
    results = [("1 parse", True,
                "%s parses; manifest layout (segments resolved "
                "against --root, the validate_film.py convention)"
                % manifest_path, [])]
    results += evaluate(refs, guards, orphans, "the manifest dir",
                        "manifest film-dir mode, root=%s" % root)
    return report("check_assets.py — film dir %s (manifest)" % d,
                  results)


def check_target(target, root):
    if os.path.isfile(target):
        return check_scene_file(target)
    if os.path.isdir(target):
        d = os.path.abspath(target)
        manifest_path = os.path.join(d, "film.json")
        if os.path.isfile(manifest_path):
            manifest = load_json_input(manifest_path, "manifest")
            if not isinstance(manifest, dict):
                raise InputError("manifest: %s top level must be a "
                                 "JSON object" % manifest_path)
            return check_manifest(d, manifest, manifest_path, root)
        if os.path.isfile(os.path.join(d, "onoda.json")):
            return check_driver(d)
        raise InputError("%s: unrecognised film dir — holds "
                         "neither film.json (manifest) nor "
                         "onoda.json (driver data)" % target)
    raise InputError("no such file or directory: %s" % target)


def main():
    ap = argparse.ArgumentParser(
        description="Check asset references in TaleRender scene "
                    "files and film dirs against the loaders' own "
                    "resolution rules (spec: "
                    "docs/integration/ASSETS-W28.md).")
    ap.add_argument("--root", default=None, metavar="DIR",
                    help="repo root for manifest references "
                         "(segments[].scene, sourceData); default: "
                         "the current directory — the "
                         "validate_film.py convention")
    ap.add_argument("targets", nargs="+", metavar="TARGET",
                    help="scene .json file, driver film dir, or "
                         "manifest film dir")
    args = ap.parse_args()
    root = os.path.abspath(args.root) if args.root else os.getcwd()
    all_ok = True
    for i, t in enumerate(args.targets):
        if i:
            print()
        all_ok = check_target(t, root) and all_ok
    return 0 if all_ok else 1


if __name__ == "__main__":
    try:
        sys.exit(main())
    except InputError as e:
        print("ERROR: check_assets.py: %s" % e, file=sys.stderr)
        sys.exit(2)
