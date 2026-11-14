#!/usr/bin/env python3
"""package_bridge.py — assemble a skill project's native/ bridge package.

Seat W48 (TaleRender 100-agent wave). Stdlib only.

The native capture backend (the skill's tools/capture_native.mjs) captures
only projects that carry a bridge package under <project>/native/: a film
manifest, the segment scene files, and the binary's film data dir
(onoda.json + assets/). This packager builds that package from the
TaleRender repo's authored data for THIS film (the Onoda short), then
verifies its own output by re-implementing, check for check, the
adapter's pre-validation (findManifest / loadManifest / findFilmDir /
validateFilmData in capture_native.mjs) — plus a MANIFEST-SHA integrity
pass the adapter does not do.

Layout produced (paths relative to <project>/native/):

    film.json                  manifest (format "talerender-film" v1)
    MANIFEST-SHA               sha256 of every packaged file
    scenes/film/*.scene.json   the segment scene files the manifest names
    film/onoda.json            the binary's film data
    film/assets/               plates.json, plate/calendar/sign PNGs,
                               and every caption strip onoda.json names

Usage:
    python3 tools/bridge/package_bridge.py --repo ~/workspace/talerender \
        --project /path/to/skill-project [--force]

Exit codes: 0 = packaged and every verification check PASS;
            1 = packaging or verification failed (the failure is named);
            2 = usage error / refused (e.g. native/ already populated
                and --force not given).

Scope: this bridges THIS film's data (see README.md). It is not a
general project converter.
"""

import argparse
import hashlib
import json
import math
import shutil
import sys
from pathlib import Path

# The adapter compares the manifest fps against its FPS env (default 30);
# a package is verified against that default.
ADAPTER_DEFAULT_FPS = 30

# Asset names the adapter's validateFilmData requires unconditionally
# (capture_native.mjs: need('plates.json'), the two PNG loops).
REQUIRED_PLATES = ["paper-map", "leaflet", "book-open", "book-closed", "coconut"]
REQUIRED_IMAGES = [
    "calendar-1945", "calendar-1955", "calendar-1965",
    "calendar-1974", "calendar-dark", "sign",
]

# Paths inside <project>/native/ this tool manages; --force rebuilds
# exactly these and nothing else (an operator-dropped native/talerender
# binary, which the adapter also discovers, is left untouched).
MANAGED = ["film.json", "MANIFEST-SHA", "scenes", "film"]


def die(msg, code=1):
    print(f"package-bridge: {msg}", file=sys.stderr)
    sys.exit(code)


def sha256_of(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def is_number(x):
    return isinstance(x, (int, float)) and not isinstance(x, bool)


def is_pos_int(x):
    return isinstance(x, int) and not isinstance(x, bool) and x > 0


# ---------------------------------------------------------------------
# Packaging
# ---------------------------------------------------------------------

def convert_manifest(src, repo, project):
    """Repo manifest -> packaged manifest.

    Everything the adapter reads (format, version, fps, resolution,
    durationS, totalFrames, segments) is carried verbatim — the repo
    manifest is already format v1 and its segments already tile.
    Only machine-specific metadata changes: the source-VM absolute
    paths are replaced with this package's target, and a bridge
    provenance block is added (the adapter ignores extra fields).
    """
    m = dict(src)
    m["project"] = str(project)
    audio = dict(m.get("audio") or {})
    audio["projectPath"] = str(project / audio.get("file", "data/mix.wav"))
    m["audio"] = audio
    m["bridge"] = {
        "packagedBy": "tools/bridge/package_bridge.py",
        "sourceRepo": str(repo),
        "sourceManifest": "scenes/film/film.json",
        "sourceFilmDir": "film/",
        "note": "Assembled by package_bridge.py; verified against the "
                "capture_native.mjs pre-validation contract. See "
                "tools/bridge/README.md.",
    }
    return m


def package(repo, project, force):
    src_manifest_path = repo / "scenes" / "film" / "film.json"
    src_film_dir = repo / "film"
    if not src_manifest_path.is_file():
        die(f"repo manifest not found: {src_manifest_path}")
    if not (src_film_dir / "onoda.json").is_file():
        die(f"repo film data not found: {src_film_dir}/onoda.json")
    try:
        src_manifest = json.loads(src_manifest_path.read_text(encoding="utf-8"))
    except Exception as e:
        die(f"repo manifest unparseable: {e}")
    if src_manifest.get("format") != "talerender-film":
        die(f"repo manifest format is {src_manifest.get('format')!r}, "
            "expected 'talerender-film' — refusing to package unknown data")

    native = project / "native"
    existing = [n for n in MANAGED if (native / n).exists()]
    if existing and not force:
        die(f"{native} already contains {', '.join(existing)} — "
            "refusing to overwrite; re-run with --force to rebuild the "
            "managed paths", code=2)
    if force:
        for n in MANAGED:
            p = native / n
            if p.is_dir():
                shutil.rmtree(p)
            elif p.exists():
                p.unlink()

    manifest = convert_manifest(src_manifest, repo, project)

    # Manifest (adapter's first discovery path: <native>/film.json).
    native.mkdir(parents=True, exist_ok=True)
    (native / "film.json").write_text(
        json.dumps(manifest, indent=1, ensure_ascii=False) + "\n",
        encoding="utf-8")

    # Segment scene files, exactly the set the manifest names
    # (scene paths are native-relative in the manifest).
    copied_scenes = []
    for seg in manifest["segments"]:
        rel = seg["scene"]
        src = repo / rel
        if not src.is_file():
            die(f"manifest segment {seg.get('id', '?')} names a scene "
                f"file missing from the repo: {src}")
        dst = native / rel
        dst.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(src, dst)
        copied_scenes.append(rel)

    # The binary's film data dir: onoda.json + the whole assets/ tree
    # (the binary's load_assets reads plates, calendars, sign and every
    # caption strip from here; copying the tree wholesale keeps the
    # package complete by construction, not by enumeration).
    dst_film = native / "film"
    dst_film.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(src_film_dir / "onoda.json", dst_film / "onoda.json")
    src_assets = src_film_dir / "assets"
    dst_assets = dst_film / "assets"
    dst_assets.mkdir(parents=True, exist_ok=True)
    copied_assets = []
    for f in sorted(src_assets.iterdir()):
        if f.is_file():
            shutil.copyfile(f, dst_assets / f.name)
            copied_assets.append(f.name)

    # MANIFEST-SHA: sha256 of every packaged file (itself excluded),
    # sha256sum-compatible lines, paths native-relative, sorted.
    packaged = []
    for p in sorted(native.rglob("*")):
        if p.is_file() and p.name != "MANIFEST-SHA":
            packaged.append(p)
    lines = [
        f"{sha256_of(p)}  {p.relative_to(native).as_posix()}"
        for p in packaged
    ]
    (native / "MANIFEST-SHA").write_text("\n".join(lines) + "\n",
                                         encoding="utf-8")

    print(f"packaged {len(packaged)} files into {native}")
    print(f"  manifest : film.json "
          f"({manifest['totalFrames']} frames @ {manifest['fps']} fps, "
          f"{manifest['durationS']} s, "
          f"{manifest['resolution']['width']}x"
          f"{manifest['resolution']['height']})")
    print(f"  scenes   : {len(copied_scenes)} segment scene files")
    print(f"  film data: film/onoda.json + {len(copied_assets)} asset files")
    return native


# ---------------------------------------------------------------------
# Verification — a check-for-check re-implementation of the adapter's
# pre-validation (capture_native.mjs). Each check prints PASS/FAIL;
# any FAIL makes the process exit non-zero.
# ---------------------------------------------------------------------

class Checks:
    def __init__(self):
        self.failures = 0
        self.count = 0

    def check(self, name, ok, detail=""):
        self.count += 1
        if not ok:
            self.failures += 1
        suffix = f" — {detail}" if detail else ""
        print(f"  [{'PASS' if ok else 'FAIL'}] {name}{suffix}")
        return ok


def verify(native):
    c = Checks()
    print(f"verifying {native} against the capture_native.mjs contract:")

    # findManifest: <native>/film.json, else <native>/scenes/film/film.json
    manifest_path = native / "film.json"
    if not manifest_path.is_file():
        manifest_path = native / "scenes" / "film" / "film.json"
    if not c.check("manifest discoverable (film.json or "
                   "scenes/film/film.json)", manifest_path.is_file(),
                   str(manifest_path)):
        return c
    try:
        m = json.loads(manifest_path.read_text(encoding="utf-8"))
    except Exception as e:
        c.check("manifest parses as JSON", False, str(e))
        return c

    # loadManifest
    c.check('manifest format == "talerender-film"',
            m.get("format") == "talerender-film", f"got {m.get('format')!r}")
    c.check("manifest version == 1", m.get("version") == 1,
            f"got {m.get('version')!r}")
    fps = m.get("fps")
    c.check(f"manifest fps == adapter FPS ({ADAPTER_DEFAULT_FPS})",
            fps == ADAPTER_DEFAULT_FPS, f"got {fps!r}")
    res = m.get("resolution") or {}
    c.check("manifest resolution width/height positive integers",
            is_pos_int(res.get("width")) and is_pos_int(res.get("height")),
            f"got {res.get('width')!r}x{res.get('height')!r}")
    segments = m.get("segments")
    c.check("manifest segments non-empty array",
            isinstance(segments, list) and len(segments) > 0,
            f"{len(segments) if isinstance(segments, list) else 'n/a'} segments")
    total = m.get("totalFrames")
    tile_ok, tile_detail = True, f"tile [0, {total})"
    scene_ok, scene_detail = True, "all segment scene files exist"
    expect = 0
    if isinstance(segments, list):
        for s in segments:
            sid = s.get("id", "?")
            sf, ef = s.get("startFrame"), s.get("endFrame")
            if sf != expect or not (is_number(ef) and is_number(sf)
                                    and ef > sf):
                tile_ok = False
                tile_detail = (f"segment {sid} range {sf}..{ef} does not "
                               f"tile (expected start {expect})")
                break
            expect = ef
            scene_rel = s.get("scene")
            if not scene_rel or not (native / scene_rel).is_file():
                scene_ok = False
                scene_detail = (f"segment {sid} scene missing: "
                                f"{native / (scene_rel or '')}")
        else:
            if expect != total:
                tile_ok = False
                tile_detail = (f"segments tile to frame {expect}, "
                               f"totalFrames is {total}")
    c.check("segment frame ranges tile [0, totalFrames)", tile_ok,
            tile_detail)
    c.check("segment scene files exist (native-relative)", scene_ok,
            scene_detail)
    dur_s = m.get("durationS")
    c.check("totalFrames == ceil(durationS * fps)",
            is_number(dur_s) and is_number(fps)
            and math.ceil(dur_s * fps) == total,
            f"ceil({dur_s} * {fps}) = "
            f"{math.ceil(dur_s * fps) if is_number(dur_s) and is_number(fps) else 'n/a'}"
            f" vs totalFrames {total}")

    # findFilmDir: <native>/film with onoda.json, else <native> itself
    film_dir = native / "film"
    if not (film_dir / "onoda.json").is_file():
        film_dir = native
    has_onoda = (film_dir / "onoda.json").is_file()
    if not c.check("film data dir found (onoda.json)", has_onoda,
                   str(film_dir)):
        return c

    # validateFilmData
    try:
        data = json.loads((film_dir / "onoda.json").read_text(
            encoding="utf-8"))
    except Exception as e:
        c.check("onoda.json parses as JSON", False, str(e))
        return c
    dur = data.get("duration")
    c.check("onoda.json duration is a number > 0",
            is_number(dur) and dur > 0, f"got {dur!r}")
    lines = data.get("lines")
    c.check("onoda.json lines[] non-empty",
            isinstance(lines, list) and len(lines) > 0,
            f"{len(lines) if isinstance(lines, list) else 'n/a'} lines")
    c.check("onoda.json anchors{} present",
            isinstance(data.get("anchors"), dict))
    captions = data.get("captions")
    c.check("onoda.json captions[] non-empty",
            isinstance(captions, list) and len(captions) > 0,
            f"{len(captions) if isinstance(captions, list) else 'n/a'} captions")
    if is_number(dur) and is_number(fps):
        c.check("ceil(onoda.duration * fps) == manifest totalFrames",
                math.ceil(dur * fps) == total,
                f"ceil({dur} * {fps}) = {math.ceil(dur * fps)} "
                f"vs {total}")
    adir = film_dir / "assets"
    c.check("assets/plates.json exists", (adir / "plates.json").is_file())
    missing = [f"{n}.png" for n in REQUIRED_PLATES + REQUIRED_IMAGES
               if not (adir / f"{n}.png").is_file()]
    c.check("assets plate/calendar/sign PNGs exist", not missing,
            "missing: " + ", ".join(missing) if missing
            else f"all {len(REQUIRED_PLATES) + len(REQUIRED_IMAGES)} present")
    strip_bad = []
    strip_missing = []
    if isinstance(captions, list):
        for i, cap in enumerate(captions):
            strip = cap.get("strip") if isinstance(cap, dict) else None
            if not (isinstance(strip, str) and strip):
                strip_bad.append(f"caption {i} has no strip filename")
            elif not (adir / strip).is_file():
                strip_missing.append(strip)
    c.check("every caption names a strip file", not strip_bad,
            "; ".join(strip_bad[:3]))
    c.check("every caption strip exists in assets/", not strip_missing,
            "missing: " + ", ".join(strip_missing[:5])
            + (f" (+{len(strip_missing) - 5} more)"
               if len(strip_missing) > 5 else "")
            if strip_missing else
            f"all {len(captions) if isinstance(captions, list) else 0} strips present")

    # MANIFEST-SHA integrity (the packager's own addition — the adapter
    # does not do this; it proves the package on disk is the package
    # that was assembled, byte for byte).
    sha_path = native / "MANIFEST-SHA"
    if c.check("MANIFEST-SHA exists", sha_path.is_file()):
        recorded = {}
        for line in sha_path.read_text(encoding="utf-8").splitlines():
            if not line.strip():
                continue
            digest, _, rel = line.partition("  ")
            recorded[rel] = digest
        on_disk = {
            p.relative_to(native).as_posix(): p
            for p in sorted(native.rglob("*"))
            if p.is_file() and p.name != "MANIFEST-SHA"
        }
        problems = []
        for rel, p in on_disk.items():
            if rel not in recorded:
                problems.append(f"unrecorded: {rel}")
            elif recorded[rel] != sha256_of(p):
                problems.append(f"hash mismatch: {rel}")
        for rel in recorded:
            if rel not in on_disk:
                problems.append(f"recorded but missing: {rel}")
        c.check("MANIFEST-SHA covers every packaged file, hashes match",
                not problems,
                "; ".join(problems[:3]) if problems
                else f"{len(on_disk)} files verified")
    return c


def main():
    ap = argparse.ArgumentParser(
        description="Assemble + verify a native/ bridge package from "
                    "the TaleRender repo (see tools/bridge/README.md).")
    ap.add_argument("--repo", required=True,
                    help="TaleRender repo checkout (source of "
                         "scenes/film/film.json + film/)")
    ap.add_argument("--project", required=True,
                    help="target skill-project dir; <project>/native/ "
                         "is assembled")
    ap.add_argument("--force", action="store_true",
                    help="rebuild the managed paths if native/ is "
                         "already populated")
    args = ap.parse_args()

    repo = Path(args.repo).expanduser().resolve()
    project = Path(args.project).expanduser().resolve()
    if not repo.is_dir():
        die(f"repo dir not found: {repo}", code=2)
    if not project.is_dir():
        die(f"project dir not found: {project} (create it first)", code=2)

    native = package(repo, project, args.force)
    result = verify(native)
    if result.failures:
        print(f"package-bridge: VERIFICATION FAILED — "
              f"{result.failures}/{result.count} checks failed; "
              f"the package in {native} must not be used as-is")
        sys.exit(1)
    print(f"package-bridge: VERIFICATION PASSED — "
          f"{result.count}/{result.count} checks")
    sys.exit(0)


if __name__ == "__main__":
    main()
