#!/usr/bin/env python3
"""rerender_plan.py — incremental re-render planner for film data changes.

Given two snapshots of the film data (OLD = as rendered, NEW = as
modified), compute the MINIMAL set of frame ranges that must be
deleted and re-rendered, with a per-range reason, plus the ranges
that must NOT be touched. The driver already skips existing frames
(resume), but until now nothing computed WHICH frames a data change
affects — ranges were guessed by hand (Wave 2's caption re-chunk
priced its A/B impact manually; this tool reproduces that analysis
mechanically and exactly).

The tool PRINTS the plan. It never deletes or renders anything.
`--print-rm DIR` emits the `rm` commands for a human/lead to review
and run; deletion + a driver resume run then re-render exactly the
planned frames.

Usage:
    python3 tools/film/rerender_plan.py OLD_DIR NEW_DIR [--json]
    python3 tools/film/rerender_plan.py OLD_DIR NEW_DIR --print-rm FRAMES_DIR
    python3 tools/film/rerender_plan.py OLD_DIR NEW_DIR --inherit s07:s08,s08b,s09

A snapshot dir may use the repo layout (scenes/film/film.json +
scenes/atmospheres.json + film/onoda.json + film/assets/) or the
bare forms (film.json / onoda.json / assets/ at the top). Two git
refs can be compared by materialising them first, e.g.
    git archive <ref> scenes film | tar -x -C /tmp/snap-old

Exit code: 0 = plan computed (possibly empty), 2 = input error
(a snapshot is unreadable, invalid, or holds no film data at all —
one-line ERROR on stderr, no plan).

FRAME ARITHMETIC — the driver's own, not an approximation:

  * fps comes from the data (30 for this film; src/film.rs hardcodes
    30 and eval_frame evaluates frame n at t = n / 30.0 exactly).
  * A caption chunk is on screen for frame n iff  s <= n/fps < e
    (src/film.rs eval_frame: `t >= c.s && t < c.e`, first match).
    Chunk windows below are computed by that comparison itself, in
    float arithmetic, so boundary frames are exact by construction.
    A chunk whose s*fps or e*fps lands within 1e-6 of an integer is
    flagged boundary-exact in the notes (a 1-ulp parse difference
    between the driver's hand-rolled JSON parser and Python's could
    flip that one frame; verify by telemetry if it matters).
  * Caption windows are GLOBAL (film time), never clipped to the
    owning segment: chunks bleed across cuts (chunk 51 ends 0.03 s
    into S10; chunk 41 bleeds 1.16 s into S07) and the driver does
    not clip either.
  * Segment ranges come from the manifest (scenes/film/film.json
    startFrame/endFrame — the validator's tiling of record). Caveat,
    stated once here and in RERENDER-W27.md: the driver's in-code
    shot cuts for s05b/s07/s08b sit at anchor-derived times up to
    0.25 s BEFORE the manifest boundary, so a segment-range plan can
    under-cover by up to ~8 frames at those three cuts; caption
    windows are unaffected (global arithmetic).

CHANGE RULES (what maps to which frames):

  * Caption chunk changed (text / s / e / fit differ; chunks are
    matched by CONTENT, never by index — a pure renumbering after a
    split is pixel-identical and yields no frames) -> the union of
    the old and new chunk windows.
  * Scene file changed outside `captions`/`captionsNote` -> the
    segment's full range. Pixel dependency inside a segment is
    all-or-nothing: the planner does not guess sub-segment regions.
    `captionsNote` is prose (nothing in src/ reads it) and is
    reported as a note only, as is a manifest `captionChunks`
    re-tile that moves no chunk.
  * Manifest timing change for a segment (startS/endS/startFrame/
    endFrame differ at all — motion is continuous in t, so even a
    sub-frame shift re-evaluates every frame) -> union of the old
    and new ranges. A shorter new film adds a delete-only stale
    tail [newTotal, oldTotal).
  * Manifest dressing change (set / atmosphere / filmLook), a
    filmLook definition change in atmospheres.json, or a scene
    dressing change -> the segment's full range, EXPANDED through
    the atmosphere-inheritance graph (below).
  * Driver data (film/onoda.json) changed outside `captions` ->
    the full film. Shot boundaries and staging derive from
    lines/anchors in code; that is not decomposable from data, and
    the planner says so instead of guessing.
  * Asset changed (bytes differ; added/removed count) -> the
    segments whose scene data references the asset (deep string
    scan — scene files name their plate files), or the referencing
    chunk's window for caption strips... caption strips (cap_*.png)
    are EXCLUDED from byte diffing on purpose: a strip is a pure
    function of its chunk's (text, fit), both already in the chunk
    key, and the generator is byte-deterministic (W01 record), so
    name-based strip diffs after a renumbering would report ~59
    false changes. A changed plates.json resolves per plate.
    An asset no segment references is not derivable -> full film,
    flagged conservative.

ATMOSPHERE INHERITANCE (encoded facts, film.rs at the wave-2 head):

  s08, s08b and s09 inherit S07's golden dressing: each of their
  shot functions in src/film.rs sets `sky_jg = TEX_SKY_GOLDEN` with
  the comment "inherits S07's golden dressing: noon passes
  sky:false and never restores dome, clouds, or the key position
  (golden keyPos [-8, 4.5, 5] persists...)". A dressing change to
  s07 therefore expands to s08, s08b, s09. The graph is data: the
  built-in table below, overlaid by `inheritance.json` sitting next
  to the manifest (or at the snapshot root) when present, overlaid
  by --inherit. Only dressing causes expand (scene non-caption
  change, manifest dressing change, filmLook change) — timing and
  caption changes do not, because the inherited state is dressing,
  not time or text.
"""

import argparse
import hashlib
import json
import math
import os
import re
import sys

BUILTIN_INHERITANCE = {"s07": ["s08", "s08b", "s09"]}
METADATA_KEYS_SCENE = {"captionsNote"}          # prose; src/ never reads it
ATMOS_METADATA_KEYS = {"format", "version", "name", "provenance",
                       "sourceProject", "_note"}
STRIP_RE = re.compile(r"^cap_.*\.png$")


class InputError(Exception):
    """A snapshot cannot be read or holds no film data (exit 2)."""


# ---------------------------------------------------------------------------
# snapshot loading
# ---------------------------------------------------------------------------

def load_json(path, what):
    try:
        with open(path) as f:
            return json.load(f)
    except OSError as e:
        raise InputError("%s: cannot read %s (%s)" % (what, path, e.strerror or e))
    except ValueError as e:
        raise InputError("%s: invalid JSON in %s (%s)" % (what, path, e))


def first_existing(*paths):
    for p in paths:
        if p and os.path.isfile(p):
            return p
    return None


def file_hash(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for block in iter(lambda: f.read(1 << 16), b""):
            h.update(block)
    return h.hexdigest()


class Snapshot:
    def __init__(self, root, label):
        self.root = root
        self.label = label
        self.manifest = None          # scenes/film/film.json dict
        self.manifest_dir = None
        self.segments = {}            # id -> manifest segment entry
        self.scene_docs = {}          # id -> parsed scene JSON
        self.sidecars = {}            # id -> {filename: parsed JSON}
        self.atmospheres = None       # scenes/atmospheres.json dict
        self.driver = None            # film/onoda.json dict
        self.assets = {}              # basename -> sha256 (non-strip incl.)
        man = first_existing(os.path.join(root, "scenes", "film", "film.json"),
                             os.path.join(root, "film.json"))
        if man:
            self.manifest = load_json(man, "%s manifest" % label)
            if not isinstance(self.manifest, dict) \
                    or not isinstance(self.manifest.get("segments"), list):
                raise InputError("%s manifest: %s has no 'segments' list"
                                 % (label, man))
            self.manifest_dir = os.path.dirname(man)
            for seg in self.manifest["segments"]:
                if isinstance(seg, dict) and isinstance(seg.get("id"), str):
                    self.segments[seg["id"]] = seg
            self._load_scenes()
            atm = first_existing(
                os.path.join(root, "scenes", "atmospheres.json"),
                os.path.join(os.path.dirname(self.manifest_dir),
                             "atmospheres.json"),
                os.path.join(root, "atmospheres.json"))
            if atm:
                self.atmospheres = load_json(atm, "%s atmospheres" % label)
        drv = first_existing(os.path.join(root, "film", "onoda.json"),
                             os.path.join(root, "onoda.json"))
        if drv:
            self.driver = load_json(drv, "%s driver data" % label)
        for cand in (os.path.join(root, "film", "assets"),
                     os.path.join(root, "assets"),
                     os.path.join(root, "assets", "captions")):
            if os.path.isdir(cand):
                for fn in sorted(os.listdir(cand)):
                    p = os.path.join(cand, fn)
                    if os.path.isfile(p) and fn not in self.assets:
                        self.assets[fn] = file_hash(p)
        if self.manifest is None and self.driver is None:
            raise InputError("%s: no film data under %s (looked for "
                             "scenes/film/film.json, film.json, "
                             "film/onoda.json, onoda.json)" % (label, root))

    def _resolve_scene(self, seg):
        rel = seg.get("scene")
        if not isinstance(rel, str):
            return None
        for cand in (os.path.join(self.root, rel),
                     os.path.join(self.manifest_dir, os.path.basename(rel)),
                     os.path.join(self.manifest_dir, rel)):
            if os.path.isfile(cand):
                return cand
        return None

    def _load_scenes(self):
        for sid, seg in self.segments.items():
            p = self._resolve_scene(seg)
            if p:
                doc = load_json(p, "%s scene %s" % (self.label, sid))
                if isinstance(doc, dict):
                    self.scene_docs[sid] = doc
            # sidecars: <segid>-*.json next to the manifest (s07-calendar.json)
            if self.manifest_dir and os.path.isdir(self.manifest_dir):
                for fn in sorted(os.listdir(self.manifest_dir)):
                    if fn.startswith(sid + "-") and fn.endswith(".json"):
                        doc = load_json(os.path.join(self.manifest_dir, fn),
                                        "%s sidecar %s" % (self.label, fn))
                        self.sidecars.setdefault(sid, {})[fn] = doc

    # -- derived facts --------------------------------------------------------

    @property
    def fps(self):
        for src in (self.manifest, self.driver):
            if src and isinstance(src.get("fps"), (int, float)):
                return src["fps"]
        return 30

    @property
    def total_frames(self):
        if self.manifest and isinstance(self.manifest.get("totalFrames"),
                                        (int, float)):
            return int(self.manifest["totalFrames"])
        if self.driver and isinstance(self.driver.get("duration"),
                                       (int, float)):
            return math.ceil(self.driver["duration"] * self.fps)
        if self.driver and isinstance(self.driver.get("frames"), (int, float)):
            return int(self.driver["frames"])
        return 0

    def seg_range(self, sid):
        """(startFrame, endFrame) from the manifest; derived via the
        validator's frame rule when the manifest omits them."""
        seg = self.segments.get(sid)
        if seg is None:
            return None
        if isinstance(seg.get("startFrame"), (int, float)) \
                and isinstance(seg.get("endFrame"), (int, float)):
            return int(seg["startFrame"]), int(seg["endFrame"])
        fps = self.fps
        start = math.ceil(seg["startS"] * fps - 1e-9)
        segs = list(self.segments.values())
        if seg is segs[-1]:
            return start, self.total_frames
        return start, math.ceil(seg["endS"] * fps - 1e-9)

    def chunks(self):
        """Global caption chunk list, scenes side first, driver side
        as fallback: entries (text, s, e, fit)."""
        out = []
        if self.scene_docs:
            for sid in self.segments:
                doc = self.scene_docs.get(sid) or {}
                for c in doc.get("captions", []) or []:
                    if isinstance(c, dict) and "s" in c and "e" in c:
                        out.append((c.get("text", ""), c["s"], c["e"],
                                    c.get("fit")))
            if out:
                return out, "scenes"
        if self.driver:
            for c in self.driver.get("captions", []) or []:
                if isinstance(c, dict) and "s" in c and "e" in c:
                    out.append((c.get("text", ""), c["s"], c["e"],
                                c.get("fit")))
            return out, "driver"
        return out, "none"

    def driver_chunks(self):
        out = []
        if self.driver:
            for c in self.driver.get("captions", []) or []:
                if isinstance(c, dict) and "s" in c and "e" in c:
                    out.append((c.get("text", ""), c["s"], c["e"],
                                c.get("fit")))
        return out

    def asset_referrers(self, basename):
        """Segment ids whose scene data (or sidecars) mention the
        asset basename anywhere in a string value."""
        hits = []

        def walk(obj):
            if isinstance(obj, str):
                return basename in obj
            if isinstance(obj, dict):
                return any(walk(v) for v in obj.values())
            if isinstance(obj, list):
                return any(walk(v) for v in obj)
            return False

        for sid in self.segments:
            doc = self.scene_docs.get(sid)
            if doc is not None and walk(doc):
                hits.append(sid)
            elif any(walk(d) for d in (self.sidecars.get(sid) or {}).values()):
                hits.append(sid)
        return hits


# ---------------------------------------------------------------------------
# frame arithmetic — the driver's own comparisons (see module docstring)
# ---------------------------------------------------------------------------

def chunk_window(s, e, fps):
    """[lo, hi) frame range where chunk [s, e) is on screen, computed
    with the driver's comparison s <= n/fps < e in float arithmetic."""
    if e <= s:
        return None
    lo = int(s * fps)
    while lo > 0 and (lo - 1) / fps >= s:
        lo -= 1
    while lo / fps < s:
        lo += 1
    hi = int(e * fps)
    while hi > 0 and (hi - 1) / fps >= e:
        hi -= 1
    while hi / fps < e:
        hi += 1
    return lo, hi


def boundary_exact(s, e, fps):
    return any(abs(v * fps - round(v * fps)) < 1e-6 for v in (s, e))


# ---------------------------------------------------------------------------
# plan construction
# ---------------------------------------------------------------------------

class Plan:
    def __init__(self):
        self.parts = []   # (start, end, reason, delete_only)
        self.notes = []

    def add(self, start, end, reason, delete_only=False):
        if end > start:
            self.parts.append((int(start), int(end), reason, delete_only))

    def merged(self):
        out = []
        for start, end, reason, d_only in sorted(self.parts):
            if out and start <= out[-1][1]:
                o = out[-1]
                o[1] = max(o[1], end)
                if reason not in o[2]:
                    o[2].append(reason)
                o[3] = o[3] and d_only
            else:
                out.append([start, end, [reason], d_only])
        return out


def load_inheritance(old, new, cli_spec):
    graph = {k: list(v) for k, v in BUILTIN_INHERITANCE.items()}
    for snap in (old, new):
        for cand in (os.path.join(snap.manifest_dir or "", "inheritance.json"),
                     os.path.join(snap.root, "inheritance.json")):
            if cand and os.path.isfile(cand):
                data = load_json(cand, "inheritance")
                if isinstance(data, dict):
                    for k, v in data.items():
                        if isinstance(v, list):
                            graph[k] = [x for x in v if isinstance(x, str)]
    if cli_spec:
        for group in cli_spec.split(";"):
            if ":" in group:
                src, dsts = group.split(":", 1)
                graph[src.strip()] = [d.strip() for d in dsts.split(",")
                                      if d.strip()]
    return graph


def expand_inheritance(plan, graph, src, new, old, cause):
    """Add full ranges for every (transitive) inheritor of src."""
    seen, queue = set(), list(graph.get(src, []))
    while queue:
        dst = queue.pop(0)
        if dst in seen:
            continue
        seen.add(dst)
        queue.extend(graph.get(dst, []))
        for snap in (new, old):
            r = snap.seg_range(dst)
            if r:
                plan.add(r[0], r[1],
                         "segment %s inherits dressing from %s (%s) — "
                         "expanded via the atmosphere-inheritance graph"
                         % (dst, src, cause))
                break


def diff_chunks(plan, old, new, fps, total, side_label):
    """Chunk-content diff for one data side. Returns True if any
    chunk content moved (used to spot renumber-only manifest tiles)."""
    def key(c):
        return (c[0], c[1], c[2], c[3])

    old_map, new_map = {}, {}
    for c in old:
        old_map.setdefault(key(c), []).append(c)
    for c in new:
        new_map.setdefault(key(c), []).append(c)
    moved = False
    for k in sorted(set(old_map) | set(new_map), key=lambda k: (k[1], k[0])):
        o, n = old_map.get(k, []), new_map.get(k, [])
        for i in range(max(len(o), len(n))):
            if i < len(o) and i < len(n):
                continue  # identical chunk, same multiplicity
            moved = True
            c = o[i] if i < len(o) else n[i]
            verb = "removed or retimed" if i < len(o) else "added or retimed"
            w = chunk_window(c[1], c[2], fps)
            if boundary_exact(c[1], c[2], fps):
                plan.notes.append(
                    "%s caption chunk '%s' [%.3f, %.3f) starts/ends within "
                    "1e-6 of a frame boundary — its edge frame is "
                    "parser-ulp sensitive; verify by telemetry"
                    % (side_label, c[0], c[1], c[2]))
            if w:
                plan.add(max(w[0], 0), min(w[1], total),
                         "%s caption chunk %s: '%s' [%.3f, %.3f)"
                         % (side_label, verb, c[0], c[1], c[2]))
    return moved


def build_plan(old, new, inherit_spec):
    plan = Plan()
    graph = load_inheritance(old, new, inherit_spec)
    fps_old, fps_new = old.fps, new.fps
    total_old, total_new = old.total_frames, new.total_frames
    total = max(total_old, total_new)

    if fps_old != fps_new:
        plan.add(0, total, "fps changed (%r -> %r): every frame's t "
                 "changes — full film" % (fps_old, fps_new))
        return plan, total_new

    fps = fps_new
    dressing_hits = []   # segment ids whose dressing changed (expand later)

    # ---- manifest segment diff -------------------------------------------
    if old.manifest is not None and new.manifest is not None:
        for sid in sorted(set(old.segments) | set(new.segments)):
            o, n = old.segments.get(sid), new.segments.get(sid)
            if o is None:
                r = new.seg_range(sid)
                plan.add(r[0], r[1], "segment %s added" % sid)
                continue
            if n is None:
                r = old.seg_range(sid)
                plan.add(r[0], r[1], "segment %s removed (its frames "
                         "hold stale content)" % sid)
                continue
            timing_keys = ("startS", "endS", "startFrame", "endFrame")
            if any(o.get(k) != n.get(k) for k in timing_keys):
                ro, rn = old.seg_range(sid), new.seg_range(sid)
                plan.add(min(ro[0], rn[0]), max(ro[1], rn[1]),
                         "segment %s timing changed (%r-%r s -> %r-%r s): "
                         "union of old/new ranges"
                         % (sid, o.get("startS"), o.get("endS"),
                            n.get("startS"), n.get("endS")))
            dress_keys = [k for k in ("set", "atmosphere", "filmLook")
                          if o.get(k) != n.get(k)]
            if dress_keys:
                r = new.seg_range(sid)
                plan.add(r[0], r[1], "segment %s dressing changed in the "
                         "manifest (%s)" % (sid, ", ".join(dress_keys)))
                dressing_hits.append(sid)
            if o.get("captionChunks") != n.get("captionChunks"):
                plan.notes.append("segment %s: manifest captionChunks "
                                  "re-tiled (%r -> %r) — derived field; "
                                  "pixel impact, if any, comes from the "
                                  "chunk diff itself"
                                  % (sid, o.get("captionChunks"),
                                     n.get("captionChunks")))
    elif (old.manifest is None) != (new.manifest is None):
        plan.add(0, total, "manifest appeared/disappeared between "
                 "snapshots — full film (conservative)")
        return plan, total_new

    # ---- scene file diff ---------------------------------------------------
    captions_touched = False
    if old.manifest is not None and new.manifest is not None:
        for sid in sorted(set(old.segments) & set(new.segments)):
            od, nd = old.scene_docs.get(sid), new.scene_docs.get(sid)
            if (od is None) != (nd is None):
                r = new.seg_range(sid) or old.seg_range(sid)
                plan.add(r[0], r[1], "segment %s scene file appeared/"
                         "disappeared" % sid)
                dressing_hits.append(sid)
                continue
            if od is None or od == nd:
                if (old.sidecars.get(sid) or {}) != (new.sidecars.get(sid)
                                                      or {}):
                    r = new.seg_range(sid)
                    plan.add(r[0], r[1], "segment %s sidecar data changed "
                             "(%s)" % (sid, ", ".join(sorted(
                                 (new.sidecars.get(sid) or {}).keys()))))
                continue
            changed = sorted(k for k in set(od) | set(nd)
                             if od.get(k) != nd.get(k))
            render_keys = [k for k in changed
                           if k not in METADATA_KEYS_SCENE]
            if "captions" in render_keys:
                captions_touched = True
                render_keys.remove("captions")
            if not render_keys:
                if changed:
                    plan.notes.append("segment %s: only metadata/caption "
                                      "fields changed (%s) — no scene "
                                      "range; caption impact, if any, "
                                      "comes from the chunk diff"
                                      % (sid, ", ".join(changed)))
            else:
                r = new.seg_range(sid)
                plan.add(r[0], r[1], "segment %s scene data changed "
                         "(keys: %s) — full segment range (sub-segment "
                         "pixel dependency is all-or-nothing)"
                         % (sid, ", ".join(render_keys)))
                dressing_hits.append(sid)

    # ---- caption chunk diff (global; scenes side, driver side) -------------
    old_chunks, old_src = old.chunks()
    new_chunks, _ = new.chunks()
    if old_chunks or new_chunks:
        moved = diff_chunks(plan, old_chunks, new_chunks, fps,
                            total_new or total, old_src)
        if captions_touched and not moved:
            plan.notes.append("scene caption arrays differ only by "
                              "renumbering (index/id/strip fields): chunk "
                              "contents are identical, so no frames move")
    if old.manifest is not None and old.driver and new.driver:
        # both sides carry a driver file: the scenes chunk diff above
        # already used the scenes chunks; diff the driver's own list
        # too (they are regenerated together and must agree — a
        # driver-only drift is still a pixel change in the film the
        # driver renders).
        diff_chunks(plan, old.driver_chunks(), new.driver_chunks(), fps,
                    total_new or total, "driver")

    # ---- atmospheres.json ---------------------------------------------------
    if old.atmospheres is not None and new.atmospheres is not None \
            and old.atmospheres != new.atmospheres:
        oa, na = old.atmospheres, new.atmospheres
        olooks = oa.get("filmLooks", {}) or {}
        nlooks = na.get("filmLooks", {}) or {}
        for look in sorted(set(olooks) | set(nlooks)):
            if look.startswith("_"):
                continue
            if olooks.get(look) != nlooks.get(look):
                for snap in (new, old):
                    for sid, seg in snap.segments.items():
                        if seg.get("filmLook") == look:
                            r = snap.seg_range(sid)
                            plan.add(r[0], r[1], "filmLook '%s' definition "
                                     "changed in atmospheres.json "
                                     "(segment %s)" % (look, sid))
                            if sid not in dressing_hits:
                                dressing_hits.append(sid)
        other = [k for k in set(oa) | set(na)
                 if k != "filmLooks" and oa.get(k) != na.get(k)
                 and k not in ATMOS_METADATA_KEYS]
        for k in sorted(other):
            plan.add(0, total, "atmospheres.json '%s' changed — its "
                     "resolution into segments is not data-decomposable; "
                     "full film (conservative)" % k)

    # ---- driver data outside captions ---------------------------------------
    if (old.driver is None) != (new.driver is None):
        plan.add(0, total, "driver data (onoda.json) appeared/disappeared "
                 "— full film (conservative)")
    elif old.driver is not None and new.driver is not None:
        changed = [k for k in set(old.driver) | set(new.driver)
                   if k != "captions"
                   and old.driver.get(k) != new.driver.get(k)]
        if changed:
            plan.add(0, total, "driver data changed outside captions "
                     "(keys: %s): shot boundaries/staging derive from "
                     "lines+anchors in code — full film"
                     % ", ".join(sorted(changed)))

    # ---- assets ---------------------------------------------------------------
    for name in sorted(set(old.assets) | set(new.assets)):
        if STRIP_RE.match(name) or name == "manifest.json":
            continue  # caption strips + their census: derived from chunks
        if old.assets.get(name) == new.assets.get(name):
            continue
        if name == "plates.json":
            _diff_plates(plan, old, new)
            continue
        referrers = sorted(set(new.asset_referrers(name))
                           | set(old.asset_referrers(name)))
        if referrers:
            for sid in referrers:
                r = new.seg_range(sid) or old.seg_range(sid)
                plan.add(r[0], r[1], "asset %s changed; segment %s "
                         "references it" % (name, sid))
        else:
            plan.add(0, total, "asset %s changed; no segment's scene "
                     "data references it — dependents not derivable "
                     "from data; full film (conservative)" % name)

    # ---- inheritance expansion --------------------------------------------------
    for sid in dressing_hits:
        if graph.get(sid):
            expand_inheritance(plan, graph, sid, new, old,
                               "film.rs: the inheriting shots set "
                               "sky_jg = TEX_SKY_GOLDEN, 'inherits "
                               "S07's golden dressing'")

    # ---- stale tail --------------------------------------------------------------
    if total_new < total_old:
        plan.add(total_new, total_old,
                 "frames at/after the new totalFrames (%d) are a stale "
                 "tail — delete only" % total_new, delete_only=True)

    return plan, total_new


def _diff_plates(plan, old, new):
    """plates.json: resolve per changed plate via scene references."""
    def plates(snap):
        for cand in (os.path.join(snap.root, "film", "assets",
                                  "plates.json"),
                     os.path.join(snap.root, "assets", "plates.json")):
            if os.path.isfile(cand):
                d = load_json(cand, "plates.json")
                if not isinstance(d, dict):
                    return {}
                # F2 migration (W53 T3): the manifest entries
                # form — resolve origin per plate name. The
                # legacy bare {name: [x, y]} object still reads
                # (pre-migration snapshots stay diffable).
                entries = d.get("plates")
                if isinstance(entries, list):
                    return {e.get("name"): e.get("origin")
                            for e in entries
                            if isinstance(e, dict) and e.get("name")}
                return d
        return {}

    op, np_ = plates(old), plates(new)
    for name in sorted(set(op) | set(np_)):
        if op.get(name) == np_.get(name):
            continue
        png = "%s.png" % name
        referrers = sorted(set(new.asset_referrers(png))
                           | set(old.asset_referrers(png)))
        if referrers:
            for sid in referrers:
                r = new.seg_range(sid) or old.seg_range(sid)
                plan.add(r[0], r[1], "plates.json entry '%s' changed; "
                         "segment %s references %s" % (name, sid, png))
        else:
            plan.add(0, max(old.total_frames, new.total_frames),
                     "plates.json entry '%s' changed; no segment "
                     "references %s — full film (conservative)"
                     % (name, png))


# ---------------------------------------------------------------------------
# reporting
# ---------------------------------------------------------------------------

def complement(merged, total):
    out, cur = [], 0
    for start, end, _r, _d in merged:
        if start > cur:
            out.append((cur, start))
        cur = max(cur, end)
    if cur < total:
        out.append((cur, total))
    return out


def fmt_range(start, end):
    return "[%d, %d)  %d frames" % (start, end, end - start)


def main():
    ap = argparse.ArgumentParser(
        description="Compute the minimal re-render plan between two "
                    "film-data snapshots. Prints the plan; never "
                    "deletes or renders anything.")
    ap.add_argument("old", help="OLD snapshot dir (as rendered)")
    ap.add_argument("new", help="NEW snapshot dir (as modified)")
    ap.add_argument("--json", action="store_true",
                    help="machine-readable plan on stdout")
    ap.add_argument("--print-rm", metavar="FRAMES_DIR",
                    help="print ONLY the rm commands that delete the "
                         "planned frames from FRAMES_DIR (for a human "
                         "to review and run); the plan itself goes to "
                         "stderr")
    ap.add_argument("--inherit", metavar="SPEC",
                    help="inheritance graph overlay, e.g. "
                         "'s07:s08,s08b,s09' (';'-separated groups)")
    args = ap.parse_args()

    old = Snapshot(args.old, "OLD")
    new = Snapshot(args.new, "NEW")
    plan, total_new = build_plan(old, new, args.inherit)
    merged = plan.merged()
    untouched = complement(merged, total_new)
    n_rerender = sum(e - s for s, e, _r, _d in merged)

    if args.print_rm:
        lines = []
        for start, end, _r, _d in merged:
            files = ["%s/frame_%05d.png" % (args.print_rm.rstrip("/"), n)
                     for n in range(start, end)]
            for i in range(0, len(files), 40):
                lines.append("rm -f " + " ".join(files[i:i + 40]))
        print("\n".join(lines))
        print("rerender_plan: %d frames in %d ranges (see stderr-free "
              "plan by re-running without --print-rm)"
              % (n_rerender, len(merged)), file=sys.stderr)
        return 0

    if args.json:
        print(json.dumps({
            "fps": new.fps,
            "old_total": old.total_frames,
            "new_total": total_new,
            "ranges": [{"start": s, "end": e, "frames": e - s,
                        "delete_only": d, "reasons": r}
                       for s, e, r, d in merged],
            "total_rerender": n_rerender,
            "untouched": [{"start": s, "end": e} for s, e in untouched],
            "notes": plan.notes,
        }, indent=1))
        return 0

    print("rerender_plan.py — %s (OLD) vs %s (NEW)" % (args.old, args.new))
    print("fps %r · total frames %d -> %d" % (new.fps, old.total_frames,
                                             total_new))
    print("\nRE-RENDER (delete these frames, then let the driver "
          "resume):")
    if merged:
        for start, end, reasons, d_only in merged:
            print("  %s%s" % (fmt_range(start, end),
                              "  [delete only]" if d_only else ""))
            for r in reasons:
                print("      - %s" % r)
        pct = (100.0 * n_rerender / total_new) if total_new else 0.0
        print("  TOTAL: %d of %d frames (%.1f%%)"
              % (n_rerender, total_new, pct))
    else:
        print("  none — the snapshots are pixel-equivalent under the "
              "planner's rules")
    print("\nDO NOT TOUCH:")
    if untouched:
        for s, e in untouched:
            print("  %s" % fmt_range(s, e))
    else:
        print("  (nothing — the whole film re-renders)")
    if plan.notes:
        print("\nNOTES:")
        for n in plan.notes:
            print("  - %s" % n)
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except InputError as e:
        print("ERROR: rerender_plan.py: %s" % e, file=sys.stderr)
        sys.exit(2)
