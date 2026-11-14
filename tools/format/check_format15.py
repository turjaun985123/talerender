#!/usr/bin/env python3
"""check_format15.py — conformance checker for scene-format §15
(docs/integration/FORMAT15-DRAFT-W31.md): the prefabs + clips
extensions, the §15.1 reservation guard, and the §15.9
reconciliation rulings, in one gate.

The two Batch-2 reference validators remain the semantic
authorities for their halves — check_prefabs.py (prefab grammar,
expansion) and check_clips.py (clip/track grammar) — and this
tool DELEGATES to them by importing both modules; it does not
fork their fixture semantics. What it adds is everything neither
of them can see alone:

  1. guard       §15.1 the reservation guard as a check: a file
                 carrying reserved keys — a top-level `prefabs` or
                 `clips` block, a `prefab` key, a `clip`/`clipTime`
                 field, a `pid`, a `$param` slot — in a pre-§15
                 shape or position (wrong format version, a block
                 that is not an object, a pid or slot outside a
                 prefab template, a clipTime with no clip, a clip
                 field inside a clip definition) is REPORTED here.
                 It is never silently passed, which is the failure
                 §15.1 exists to forbid: today's loader silently
                 ignores a top-level `prefabs` block (W14 §7,
                 measured). A file with no reserved keys passes
                 this check trivially ("no §15 constructs").
  2. prefabs     §15.2–15.5, delegated to check_prefabs.py
                 (imported; its 6 sub-checks are summarised).
                 SKIPPED when the file carries no prefab
                 constructs and no --library was given.
  3. clips       §15.6–15.8, delegated to check_clips.py
                 (imported; its 5 sub-checks are summarised).
                 SKIPPED when the file carries no clip constructs
                 — where check_clips.py alone exits 2 on such a
                 file ("nothing to validate"), §15.10 step 3 wants
                 the validators run over production data with
                 exit 0 = pass, so conformance of a clip-free file
                 is trivially satisfied here.
  4. colour lane §15.9: colour values destined for the evaluator's
                 `Color` lane are AUTHORED as "#RRGGBB" strings
                 (§15.6/§15.7); the loader converts each to linear
                 [f32; 3] before the track carries it, so the
                 [f32; 3] form exists only inside the evaluator.
                 A colour-channel key whose value is an array (the
                 lane shape) or a {hex, shade} object (the base
                 format's node colour-spec) fails here, citing the
                 conversion law. (Hex validity itself is
                 check_clips check 2's; this check fires only on
                 the two lane-confusion shapes.)
  5. step        §15.7/§15.9: step channels (`mood`, `legs`) are
                 declared only on character nodes, every key uses
                 ease "step", and every value is an enum member —
                 step values ride a side channel keyed like the
                 numeric tracks and never enter a numeric track.
                 (Overlaps check_clips checks 2/3 by design; this
                 is the §15.7-table lens on the same defects.)
  6. placement   §15.9's placement rulings, which no other tool
                 checks: `duration` lives on the clip only (never
                 per-track); wrap lives on the instance only
                 (`clipTime.loop` — never a `loop`/`wrap` key on a
                 clip or a track); clip keys carry no poseTrack
                 residue (`move`/`moveSecs` — §15.8's conversion
                 re-spells them as a hold key + an arrival key).
                 Non-failing NOTEs: a track's first key carrying an
                 `ease` (§15.6: it has no incoming segment and is
                 ignored — dead data), and a track targeting the
                 name of a prefab instance (the name binds only
                 after §15.5 expansion sets it on the expanded
                 root).

Usage: python3 tools/format/check_format15.py [--library LIB.json ...]
           FILE [FILE ...]
Exit code: 0 = every non-skipped check PASS for every file,
1 = any check FAIL, 2 = input error (an input file is unreadable,
invalid JSON, or not a JSON object — one-line ERROR on stderr, no
report). Output is deterministic (command-line file order, checks
in the order above, failures in document order within each check).
"""

import argparse
import contextlib
import importlib.util
import io
import json
import os
import re
import sys


sys.dont_write_bytecode = True  # a validator must not dirty the tree


def _load_sibling(name):
    here = os.path.dirname(os.path.abspath(__file__))
    spec = importlib.util.spec_from_file_location(
        name, os.path.join(here, name + ".py"))
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


cp = _load_sibling("check_prefabs")
cc = _load_sibling("check_clips")

RESERVED_BLOCKS = ("prefabs", "clips")
STEP_CHANNELS = {name: spec for name, spec in cc.CHARACTER_CHANNELS.items()
                 if cc.kind_of(spec) == cc.STEP}
POSETRACK_RESIDUE_KEYS = ("move", "moveSecs", "move_secs")


class InputError(Exception):
    """Bad input file — the run cannot start (exit 2, no report)."""


def load_doc(path):
    try:
        with open(path) as fh:
            doc = json.load(fh)
    except OSError as e:
        raise InputError("cannot read %s (%s)" % (path, e.strerror or e))
    except ValueError as e:
        raise InputError("invalid JSON in %s (%s)" % (path, e))
    if not isinstance(doc, dict):
        raise InputError("%s: top level is not a JSON object" % path)
    return doc


def is_slot(v):
    return isinstance(v, dict) and set(v.keys()) == {"$param"}


# ---------------------------------------------------------------------------
# Reserved-key census — one walk that knows where it is.
# ---------------------------------------------------------------------------

def census(doc):
    """Locate every §15 reserved key and classify its position.

    Returns a dict of lists. Position flags: in_prefabs (under the
    top-level `prefabs` block — the only place templates live),
    in_clips (under the top-level `clips` block — definitions),
    in_nodes (under the top-level `nodes` array), in_params (inside
    the `params` value of a prefab instance).
    """
    c = {
        "prefab_nodes": [],      # paths of dicts carrying "prefab"
        "prefab_misplaced": [],  # ... outside nodes/templates/params
        "clip_fields": [],       # (path, value) of "clip" outside clips
        "clip_in_clips": [],     # "clip" keys inside clip definitions
        "clip_bad_value": [],    # "clip" with a non-string value
        "cliptime_ok": [],       # clipTime beside a string clip
        "cliptime_stray": [],    # clipTime with no clip, or in clips
        "pid_legal": [],         # pid inside a prefab template
        "pid_params": [],        # pid inside an instance's params
        "pid_stray": [],         # pid anywhere else
        "slots_outside": [],     # $param slots outside templates
    }

    def walk(v, path, in_prefabs, in_clips, in_nodes, in_params):
        if isinstance(v, dict):
            if "prefab" in v:
                if in_prefabs or in_nodes or in_params:
                    c["prefab_nodes"].append(path or "<root>")
                else:
                    c["prefab_misplaced"].append(path or "<root>")
            if "clip" in v:
                if in_clips:
                    c["clip_in_clips"].append(path or "<root>")
                elif isinstance(v["clip"], str):
                    c["clip_fields"].append((path or "<root>", v["clip"]))
                else:
                    c["clip_bad_value"].append(path or "<root>")
            if "clipTime" in v:
                if in_clips or not isinstance(v.get("clip"), str):
                    c["cliptime_stray"].append(path or "<root>")
                else:
                    c["cliptime_ok"].append(path or "<root>")
            if "pid" in v:
                if in_prefabs:
                    c["pid_legal"].append(path or "<root>")
                elif in_params:
                    c["pid_params"].append(path or "<root>")
                else:
                    c["pid_stray"].append(path or "<root>")
            if is_slot(v) and not in_prefabs:
                c["slots_outside"].append(path or "<root>")
            for k, val in v.items():
                p = "%s.%s" % (path, k) if path else k
                walk(val, p,
                     in_prefabs or (path == "" and k == "prefabs"),
                     in_clips or (path == "" and k == "clips"),
                     in_nodes or (path == "" and k == "nodes"),
                     in_params or (k == "params" and "prefab" in v))
        elif isinstance(v, list):
            for i, x in enumerate(v):
                walk(x, "%s[%d]" % (path, i),
                     in_prefabs, in_clips, in_nodes, in_params)

    walk(doc, "", False, False, False, False)
    c["has_prefabs_block"] = "prefabs" in doc
    c["has_clips_block"] = "clips" in doc
    c["prefabs_block_ok"] = isinstance(doc.get("prefabs"), dict)
    c["clips_block_ok"] = isinstance(doc.get("clips"), dict)
    c["has_prefab_constructs"] = bool(
        c["has_prefabs_block"] or c["prefab_nodes"] or c["prefab_misplaced"]
        or c["pid_legal"] or c["pid_params"] or c["pid_stray"]
        or c["slots_outside"])
    c["has_clip_constructs"] = bool(
        c["has_clips_block"] or c["clip_fields"] or c["clip_in_clips"]
        or c["clip_bad_value"] or c["cliptime_ok"] or c["cliptime_stray"])
    c["any_reserved"] = c["has_prefab_constructs"] or c["has_clip_constructs"]
    return c


# ---------------------------------------------------------------------------
# Check 1 — the reservation guard (§15.1)
# ---------------------------------------------------------------------------

def check_guard(doc, cen):
    fails, notes = [], []
    if not cen["any_reserved"]:
        return True, "no §15 constructs — no reserved keys anywhere", fails, notes
    if doc.get("version") != 2:
        fails.append(
            "file declares version=%r but carries §15 reserved keys — "
            "the extensions ride format version 2, and a v1 file "
            "carrying `prefabs` or `clips` is rejected, not "
            "reinterpreted (§15.1)" % (doc.get("version"),))
    if cen["has_prefabs_block"] and not cen["prefabs_block_ok"]:
        fails.append(
            "top-level 'prefabs' is %s, not an object of definitions "
            "— a reserved block in a pre-§15 shape is reported, never "
            "silently passed (§15.1)" % type(doc["prefabs"]).__name__)
    if cen["has_clips_block"] and not cen["clips_block_ok"]:
        fails.append(
            "top-level 'clips' is %s, not an object of clip "
            "definitions — a reserved block in a pre-§15 shape is "
            "reported, never silently passed (§15.1)"
            % type(doc["clips"]).__name__)
    for p in cen["prefab_misplaced"]:
        fails.append(
            "%s: 'prefab' key outside a node position (not under "
            "`nodes`, a template, or an instance's params) — "
            "instances may appear only where the base format takes "
            "nodes (§15.2)" % p)
    for p in cen["clip_in_clips"]:
        fails.append(
            "%s: 'clip' field inside a clip definition — instances "
            "live on nodes and on the camera/lights/post/fog blocks, "
            "never inside the `clips` block (§15.6)" % p)
    for p in cen["clip_bad_value"]:
        fails.append(
            "%s: 'clip' value is not a clip-name string — a malformed "
            "instance is reported, not ignored (§15.1, §15.6)" % p)
    for p in cen["cliptime_stray"]:
        fails.append(
            "%s: 'clipTime' with no sibling 'clip' (or inside a clip "
            "definition) — a time mapping for nothing is a reserved "
            "key in a pre-§15 shape (§15.1, §15.6)" % p)
    for p in cen["pid_stray"]:
        fails.append(
            "%s: 'pid' outside a prefab template — pids exist only "
            "inside templates, are scoped to them, and are stripped "
            "at expansion; one in scene data is reserved-key residue "
            "(§15.2)" % p)
    for p in cen["slots_outside"]:
        fails.append(
            "%s: $param slot outside a prefab template — slots are "
            "lexically scoped to the prefab that declares them "
            "(§15.2)" % p)
    for p in cen["pid_params"]:
        notes.append(
            "%s: 'pid' inside an instance's params — pids there are "
            "stripped at expansion but not addressable by overrides; "
            "the draft carries this as an open question (W31 flag "
            "3b), so it is noted, not failed" % p)
    n = (len(cen["prefab_nodes"]) + len(cen["clip_fields"])
         + len(cen["pid_legal"]))
    headline = ("reserved keys present and in §15 positions "
                "(%d instance/pid sites)" % n) if not fails else \
               ("%d guard violation(s)" % len(fails))
    return not fails, headline, fails, notes


# ---------------------------------------------------------------------------
# Checks 2/3 — delegation to the Batch-2 reference validators.
# Their reports are parsed, never re-implemented: sub-check verdict
# lines and their failure lines are re-emitted with attribution.
# ---------------------------------------------------------------------------

SUBCHECK_RE = re.compile(r"^\[(\d+) (\w+)\] (PASS|FAIL|SKIPPED) — (.*)$")


def summarise_delegated(out_lines, tool_name):
    """(ok, headline, [failure lines]) from a delegate's report."""
    subs = []  # (label, status, headline, [failures])
    cur = None
    for raw in out_lines:
        line = raw.lstrip("\n")  # check_prefabs embeds \n in its lines
        m = SUBCHECK_RE.match(line)
        if m:
            cur = ("%s %s" % (m.group(1), m.group(2)), m.group(3),
                   m.group(4), [])
            subs.append(cur)
        elif line.startswith("    ") and cur is not None \
                and cur[1] == "FAIL":
            cur[3].append(line.strip())
    fails = []
    n_pass = 0
    n_run = 0
    bad_names = []
    for label, status, _headline, lines in subs:
        if status == "SKIPPED":
            continue
        n_run += 1
        if status == "PASS":
            n_pass += 1
        else:
            bad_names.append(label)
            for ln in lines:
                fails.append("[%s] %s" % (label, ln))
    if not subs:
        return False, "via %s: no sub-check report produced" % tool_name, \
            ["%s produced no parseable report" % tool_name]
    headline = "via %s: %d/%d sub-checks passed" % (tool_name, n_pass, n_run)
    if bad_names:
        headline += " (failing: %s)" % ", ".join(bad_names)
    return not bad_names, headline, fails


def check_prefabs_delegated(path, libraries, lib_dups):
    try:
        scene, scene_dups = cp.load_json_input(path, "scene")
    except cp.InputError as e:
        raise InputError(str(e))
    _ok, lines = cp.check_scene(path, scene, scene_dups,
                                libraries, lib_dups)
    return summarise_delegated(lines, "check_prefabs.py")


def check_clips_delegated(path):
    buf = io.StringIO()
    try:
        with contextlib.redirect_stdout(buf):
            cc.check_file(path)
    except cc.InputError as e:
        return False, "via check_clips.py: input error", [str(e)]
    except Exception as e:  # a delegate crash is a FAIL, never a pass
        return False, "via check_clips.py: validator error", \
            ["check_clips.py raised %s: %s" % (type(e).__name__, e)]
    return summarise_delegated(buf.getvalue().splitlines(),
                               "check_clips.py")


# ---------------------------------------------------------------------------
# Checks 4–6 — the §15.9 / §15.7 lenses over the clips block.
# Channel kinds come from check_clips' imported tables (no fork).
# ---------------------------------------------------------------------------

def iter_tracks(doc):
    clips = doc.get("clips")
    if not isinstance(clips, dict):
        return
    for name in sorted(clips):
        clip = clips[name]
        if not isinstance(clip, dict):
            continue
        tracks = clip.get("tracks")
        if not isinstance(tracks, list):
            continue
        for ti, track in enumerate(tracks):
            if isinstance(track, dict):
                yield name, clip, ti, track


def check_colour_lane(doc, node_kinds):
    fails = []
    n_colour = 0
    for name, _clip, ti, track in iter_tracks(doc):
        tgt = track.get("target") or {}
        node, channel = tgt.get("node"), tgt.get("channel")
        if not isinstance(node, str) or not isinstance(channel, str):
            continue  # structure is check_clips check 1's report
        spec, _err = cc.channel_spec(node, channel, node_kinds)
        if spec is None or cc.kind_of(spec) != cc.COLOR:
            continue
        n_colour += 1
        keys = track.get("keys")
        if not isinstance(keys, list):
            continue
        for ki, key in enumerate(keys):
            if not isinstance(key, dict) or "value" not in key:
                continue
            v = key["value"]
            kp = "clips.%s.tracks[%d].keys[%d].value" % (name, ti, ki)
            if isinstance(v, list):
                fails.append(
                    "%s: %r is an [f32; 3]-shaped array on colour "
                    "channel '%s.%s' — files author colour as "
                    "\"#RRGGBB\" strings (§15.6/§15.7); the linear "
                    "[f32; 3] form is the evaluator's `Color` lane "
                    "shape and exists only after the loader converts "
                    "the hex value (§15.9)" % (kp, v, node, channel))
            elif isinstance(v, dict):
                fails.append(
                    "%s: a {hex, shade} object on colour channel "
                    "'%s.%s' — that is the base format's node "
                    "colour-spec spelling; a clip colour key is a "
                    "\"#RRGGBB\" string, converted to the lane's "
                    "linear [f32; 3] by the loader (§15.6, §15.9)"
                    % (kp, node, channel))
    if n_colour == 0:
        return None, "no colour channels in the clips block", fails
    return not fails, "%d colour-channel track(s) authored as hex" \
        % n_colour if not fails else \
        "%d colour-lane violation(s)" % len(fails), fails


def check_step_channels(doc, node_kinds):
    fails = []
    n_step = 0
    for name, _clip, ti, track in iter_tracks(doc):
        tgt = track.get("target") or {}
        node, channel = tgt.get("node"), tgt.get("channel")
        if not isinstance(node, str) or not isinstance(channel, str):
            continue
        tpath = "clips.%s.tracks[%d]" % (name, ti)
        if channel in STEP_CHANNELS:
            spec = STEP_CHANNELS[channel]
            n_step += 1
            if node in cc.RESERVED_TARGETS:
                fails.append(
                    "%s: step channel '%s' declared on reserved "
                    "target '%s' — step channels exist only on "
                    "character nodes (§15.7)" % (tpath, channel, node))
            elif node in node_kinds and node_kinds[node] != "character":
                fails.append(
                    "%s: step channel '%s' declared on node '%s', "
                    "which is not a character node — `mood`/`legs` "
                    "are character `state` fields (§15.7)"
                    % (tpath, channel, node))
            keys = track.get("keys")
            if not isinstance(keys, list):
                continue
            for ki, key in enumerate(keys):
                if not isinstance(key, dict):
                    continue
                kp = "%s.keys[%d]" % (tpath, ki)
                ease = key.get("ease", cc.DEFAULT_EASE)
                if ease != "step":
                    fails.append(
                        "%s.ease: step channel '%s.%s' must use ease "
                        "'step' — its values ride the step side "
                        "channel and are never interpolated (§15.7, "
                        "§15.9); got %r" % (kp, node, channel, ease))
                if "value" in key and not cc.value_ok(spec, key["value"]):
                    fails.append(
                        "%s.value: %r is not a member of the '%s' "
                        "enum (%s) — an unknown step value fails "
                        "here; it is never routed through a numeric "
                        "lane (§15.7, §15.9)"
                        % (kp, key["value"], channel,
                           ", ".join(spec[1])))
        elif node in cc.RESERVED_TARGETS or node in node_kinds:
            # a mood/legs-shaped declaration under any other name is
            # check_clips' unknown-channel report; nothing to add.
            pass
    if n_step == 0:
        return None, "no step channels declared", fails
    return not fails, "%d step-channel track(s) declared legally" \
        % n_step if not fails else \
        "%d step-channel violation(s)" % len(fails), fails


def instance_names(doc):
    """Names carried by prefab instances under `nodes` (recursive)."""
    names = set()

    def walk(v):
        if isinstance(v, dict):
            if "prefab" in v and isinstance(v.get("name"), str):
                names.add(v["name"])
            for x in v.values():
                walk(x)
        elif isinstance(v, list):
            for x in v:
                walk(x)

    walk(doc.get("nodes") or [])
    return names


def check_placement(doc, cen):
    fails, notes = [], []
    clips = doc.get("clips")
    if not isinstance(clips, dict):
        return None, "no clips block", fails, notes
    inst_names = instance_names(doc)
    for name in sorted(clips):
        clip = clips[name]
        if not isinstance(clip, dict):
            continue
        cpath = "clips.%s" % name
        for k in ("loop", "wrap"):
            if k in clip:
                fails.append(
                    "%s.%s: wrap is spelled on the clip — duration "
                    "lives on the clip, wrap lives on the instance "
                    "(`clipTime.loop`); no format file spells wrap "
                    "per-clip or per-track (§15.9)" % (cpath, k))
        tracks = clip.get("tracks")
        if not isinstance(tracks, list):
            continue
        for ti, track in enumerate(tracks):
            if not isinstance(track, dict):
                continue
            tpath = "%s.tracks[%d]" % (cpath, ti)
            if "duration" in track:
                fails.append(
                    "%s.duration: a per-track duration — the clip's "
                    "`duration` is the one bound for all its tracks "
                    "(§15.9)" % tpath)
            for k in ("loop", "wrap"):
                if k in track:
                    fails.append(
                        "%s.%s: wrap spelled per-track — it lives "
                        "on the instance as `clipTime.loop` (§15.9)"
                        % (tpath, k))
            tgt = track.get("target") or {}
            if isinstance(tgt.get("node"), str) \
                    and tgt["node"] in inst_names:
                notes.append(
                    "%s: target '%s' is the name of a prefab "
                    "instance — the name binds only after expansion "
                    "sets it on the expanded root (§15.5 step 5g); "
                    "legal, but expansion-order dependent"
                    % (tpath, tgt["node"]))
            keys = track.get("keys")
            if not isinstance(keys, list):
                continue
            for ki, key in enumerate(keys):
                if not isinstance(key, dict):
                    continue
                kp = "%s.keys[%d]" % (tpath, ki)
                for rk in POSETRACK_RESIDUE_KEYS:
                    if rk in key:
                        fails.append(
                            "%s.%s: poseTrack residue in a clip key "
                            "— §15.8's conversion re-spells a "
                            "poseTrack key as a hold key plus an "
                            "arrival key carrying the ease; clip keys "
                            "have no move field" % (kp, rk))
                if ki == 0 and "ease" in key:
                    notes.append(
                        "%s.ease: the first key's ease (%r) has no "
                        "incoming segment and is ignored (§15.6) — "
                        "dead data" % (kp, key["ease"]))
    headline = "duration on clips, wrap on instances, no residue" \
        if not fails else "%d placement violation(s)" % len(fails)
    return not fails, headline, fails, notes


# ---------------------------------------------------------------------------
# Per-file run + report (house style: numbered checks, OVERALL).
# ---------------------------------------------------------------------------

def check_file(path, libraries, lib_dups):
    doc = load_doc(path)
    cen = census(doc)
    node_kinds, _collisions = cc.collect_nodes(doc)
    results = []  # (label, ok or None, headline, [failures], [notes])

    ok, headline, fails, notes = check_guard(doc, cen)
    results.append(("1 guard", ok, headline, fails, notes))

    if cen["has_prefab_constructs"] or libraries:
        ok, headline, fails = check_prefabs_delegated(path, libraries,
                                                      lib_dups)
        results.append(("2 prefabs", ok, headline, fails, []))
    else:
        results.append(("2 prefabs", None,
                        "no prefab constructs in this file", [], []))

    if cen["has_clips_block"] or cen["clip_fields"]:
        ok, headline, fails = check_clips_delegated(path)
        results.append(("3 clips", ok, headline, fails, []))
    elif cen["has_clip_constructs"]:
        results.append(("3 clips", None,
                        "no clips block and no clip instances — the "
                        "reserved-key misuse is check 1's report",
                        [], []))
    else:
        results.append(("3 clips", None,
                        "no clip constructs in this file", [], []))

    if isinstance(doc.get("clips"), dict):
        for label, fn in (("4 colour lane", check_colour_lane),
                          ("5 step channels", check_step_channels)):
            ok, headline, fails = fn(doc, node_kinds)
            results.append((label, ok, headline, fails, []))
        ok, headline, fails, notes = check_placement(doc, cen)
        results.append(("6 placement", ok, headline, fails, notes))
    else:
        for label in ("4 colour lane", "5 step channels", "6 placement"):
            results.append((label, None, "no clips block", [], []))

    census_bits = []
    census_bits.append("prefabs block" if cen["has_prefabs_block"]
                       else "no prefabs block")
    census_bits.append("%d prefab instance(s)" % len(cen["prefab_nodes"]))
    census_bits.append("clips block" if cen["has_clips_block"]
                       else "no clips block")
    census_bits.append("%d clip instance(s)" % len(cen["clip_fields"]))
    print("check_format15.py — %s (§15 census: %s)"
          % (path, ", ".join(census_bits)))
    all_ok = True
    n_pass = 0
    n_total = 0
    for label, ok, headline, fails, notes in results:
        if ok is None:
            print("\n[%s] SKIPPED — %s" % (label, headline))
            continue
        n_total += 1
        n_pass += 1 if ok else 0
        all_ok = all_ok and ok
        print("\n[%s] %s — %s" % (label, "PASS" if ok else "FAIL",
                                  headline))
        for line in fails:
            print("    " + line)
        for line in notes:
            print("    note: " + line)
    print("\nOVERALL: %s (%d/%d checks passed)"
          % ("PASS" if all_ok else "FAIL", n_pass, n_total))
    return 0 if all_ok else 1


def main():
    ap = argparse.ArgumentParser(
        description="Validate scene files against scene-format §15 "
                    "(prefabs + clips + the reservation guard; spec: "
                    "docs/integration/FORMAT15-DRAFT-W31.md).")
    ap.add_argument("--library", action="append", default=[],
                    metavar="LIB.json",
                    help="prefab library file, as in "
                         "check_prefabs.py; may be given more than "
                         "once. Giving one runs the prefabs check "
                         "even on files without inline prefabs.")
    ap.add_argument("files", nargs="+", metavar="FILE.json",
                    help="scene / film-data JSON file(s) to validate")
    args = ap.parse_args()

    libraries = []
    lib_dups = []
    for p in args.library:
        data, dups = cp.load_json_input(p, "library")
        if "prefabs" in data and not isinstance(data["prefabs"], dict):
            raise InputError("library: %s: 'prefabs' must be an "
                             "object" % p)
        libraries.append((data, p))
        lib_dups.append(dups)

    rc = 0
    for i, path in enumerate(args.files):
        if i:
            print()
        rc = check_file(path, libraries, lib_dups) or rc
    return rc


if __name__ == "__main__":
    try:
        sys.exit(main())
    except InputError as e:
        print("ERROR: check_format15.py: %s" % e, file=sys.stderr)
        sys.exit(2)
    except cp.InputError as e:
        print("ERROR: check_format15.py: %s" % e, file=sys.stderr)
        sys.exit(2)
