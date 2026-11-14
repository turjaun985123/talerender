#!/usr/bin/env python3
"""check_prefabs.py — validator for the prefab/template extension to
the TaleRender scene format (docs/integration/FORMAT-EXT-PREFABS-W14.md).

The extension is a load-time macro over format v2: a scene file may
carry a top-level `prefabs` block (name -> {params, node}) and node
positions may hold instances {"prefab": <name>, "params": {...},
"overrides": {...}, "transform": [...]}. A conforming loader expands
every instance before any node is walked, so the renderer only ever
sees plain v2 nodes. This tool is the reference implementation of
that expansion plus the legality checks around it. It renders
nothing and validates only the prefab layer — base-v2 validation
(camera, geometry, textures, the §11 f32 law) stays with the loader
and tools/film/validate_film.py.

Checks:
  1. parse      every input parses; prefab use requires version 2;
                duplicate JSON keys are reported (parsers disagree
                on last-wins vs first-wins — a duplicate is a defect)
  2. library    every prefab definition (inline + --library files)
                is well-formed: declaration grammar, defaults
                type-check and are literal, every $param slot names
                a declared param, slots never name a prefab, pids
                are unique within their prefab's direct scope
  3. instances  every instance (in the scene and inside templates):
                known prefab, closed key set, required params
                present, no undeclared params, values type-check
                (a slot forwarded from an enclosing template must
                have a compatible declared type); no $param slot
                may appear in a scene outside a template
  4. overrides  every override addresses a declared pid of the
                target prefab's direct scope, merges legally (no
                forbidden identity keys — type/prefab/pid/id/kind/
                generator — no transform, no null deletion, never
                through a $param slot)
  5. cycles     the prefab reference graph (definition -> prefab
                names its template instantiates) is acyclic
  6. expansion  the scene expands fully with the spec's algorithm;
                the expanded tree contains no prefab/pid/$param
                residue (skipped when an earlier check failed)

Usage: python3 tools/format/check_prefabs.py [--library LIB.json ...]
           SCENE.scene.json [SCENE ...]
Exit code: 0 = all checks PASS for every scene, 1 = any check FAIL,
2 = input error (an input file is unreadable, invalid JSON, or not
a JSON object — one-line ERROR on stderr, no report). Output is
deterministic (definition order as written, failures in document
order within each check).
"""

import argparse
import copy
import json
import re
import sys

PARAM_TYPES = ("number", "string", "color", "boolean", "vec3",
               "array", "object", "node", "nodes")
DECL_KEYS = {"type", "required", "default", "description"}
DEF_KEYS = {"params", "node", "description"}
INSTANCE_KEYS = {"prefab", "params", "overrides", "transform", "name"}
# Identity / discriminating keys an override may never touch (spec §4):
# changing any of these changes WHAT the node is, not how it is staged.
FORBIDDEN_OVERRIDE_KEYS = {"type", "prefab", "pid", "id", "kind",
                           "generator", "transform"}
COLOR_RE = re.compile(r"^#[0-9A-Fa-f]{6}$")
MAX_EXPANSION_DEPTH = 64


class InputError(Exception):
    """Bad input data (unreadable / invalid / wrong-shaped files).

    Distinct from a check FAIL: the run cannot start, so main()
    reports one named line on stderr and exits 2 instead of
    printing a validation report.
    """


class ExpansionError(Exception):
    """The reference expansion could not complete."""


def load_json_input(path, what):
    """Parse a JSON file, recording duplicate object keys.

    Returns (data, [duplicate key names in encounter order]).
    Raises InputError on unreadable/invalid/non-object input.
    """
    dups = []

    def hook(pairs):
        obj = {}
        for k, v in pairs:
            if k in obj:
                dups.append(k)
            obj[k] = v
        return obj

    try:
        with open(path) as f:
            data = json.load(f, object_pairs_hook=hook)
    except OSError as e:
        raise InputError("%s: cannot read %s (%s)"
                         % (what, path, e.strerror or e))
    except ValueError as e:  # JSONDecodeError + UnicodeDecodeError
        raise InputError("%s: invalid JSON in %s (%s)" % (what, path, e))
    if not isinstance(data, dict):
        raise InputError("%s: %s top level must be a JSON object"
                         % (what, path))
    return data, dups


def is_number(v):
    return isinstance(v, (int, float)) and not isinstance(v, bool)


def is_slot(v):
    return isinstance(v, dict) and set(v.keys()) == {"$param"}


def is_instance(v):
    return isinstance(v, dict) and "prefab" in v


def is_node_like(v):
    return isinstance(v, dict) and ("type" in v or "prefab" in v)


def contains_slot(v):
    if is_slot(v):
        return True
    if isinstance(v, dict):
        return any(contains_slot(x) for x in v.values())
    if isinstance(v, list):
        return any(contains_slot(x) for x in v)
    return False


def type_name(v):
    if isinstance(v, bool):
        return "boolean"
    if is_number(v):
        return "number"
    if isinstance(v, str):
        return "string"
    if isinstance(v, list):
        return "array"
    if isinstance(v, dict):
        return "object"
    return "null"


def type_ok(v, ptype):
    """Does JSON value v satisfy declared param type ptype?"""
    if ptype == "number":
        return is_number(v)
    if ptype == "string":
        return isinstance(v, str)
    if ptype == "color":
        return isinstance(v, str) and bool(COLOR_RE.match(v))
    if ptype == "boolean":
        return isinstance(v, bool)
    if ptype == "vec3":
        return (isinstance(v, list) and len(v) == 3
                and all(is_number(x) for x in v))
    if ptype == "array":
        return isinstance(v, list)
    if ptype == "object":
        return isinstance(v, dict) and not is_slot(v) and "prefab" not in v
    if ptype == "node":
        return is_node_like(v)
    if ptype == "nodes":
        return isinstance(v, list) and all(is_node_like(x) for x in v)
    return False


def walk(v, path, parent_key=None):
    """Yield (value, path, parent_key) for v and every descendant."""
    yield v, path, parent_key
    if isinstance(v, dict):
        for k in v:
            yield from walk(v[k], "%s.%s" % (path, k) if path else k, k)
    elif isinstance(v, list):
        for i, x in enumerate(v):
            yield from walk(x, "%s[%d]" % (path, i), parent_key)


def find_instances(root, root_path):
    """All instance objects under root: [(instance, path)]."""
    return [(v, p) for v, p, _ in walk(root, root_path) if is_instance(v)]


def find_slots(root, root_path):
    """All $param slots under root: [(slot, path, parent_key)]."""
    return [(v, p, pk) for v, p, pk in walk(root, root_path) if is_slot(v)]


def pid_table(template):
    """Map pid -> node for a template's DIRECT scope.

    Descends through plain nodes (groups, meshes, ...) but not into
    a nested instance's interior: that interior belongs to the inner
    prefab, whose instances expose only their own params (spec §4).
    The nested instance node itself IS in scope (its pid counts).
    """
    table = {}

    def visit(v):
        if isinstance(v, dict):
            pid = v.get("pid")
            if isinstance(pid, str):
                table.setdefault(pid, []).append(v)
            if is_instance(v):
                return  # do not descend into a nested instance
            for x in v.values():
                visit(x)
        elif isinstance(v, list):
            for x in v:
                visit(x)

    visit(template)
    return table


def find_pid_node(template, pid):
    """The direct-scope node carrying pid, or None (depth-first)."""
    if isinstance(template, dict):
        if template.get("pid") == pid:
            return template
        if is_instance(template):
            return None
        for x in template.values():
            r = find_pid_node(x, pid)
            if r is not None:
                return r
    elif isinstance(template, list):
        for x in template:
            r = find_pid_node(x, pid)
            if r is not None:
                return r
    return None


def merge_legality(target, partial, path, lines):
    """Check a deep-merge of partial onto target without applying it.

    Reports only the slot-targeting violations (spec §4); forbidden
    keys and nulls anywhere in the override are scan_forbidden's job.
    Returns True when the merge never crosses a $param slot.
    """
    ok = True
    for k, v in partial.items():
        p = "%s.%s" % (path, k)
        if k in FORBIDDEN_OVERRIDE_KEYS or v is None or is_slot(v):
            continue  # forbidden/null: scan_forbidden; slot value: data
        if isinstance(target, dict) and k in target:
            tv = target[k]
            if is_slot(tv):
                lines.append("%s: override targets a $param slot "
                             "(%r) — use the params channel, not "
                             "overrides (spec §4)" % (p, tv["$param"]))
                ok = False
                continue
            if isinstance(tv, dict) and isinstance(v, dict):
                if not merge_legality(tv, v, p, lines):
                    ok = False
    return ok


def scan_forbidden(v, path, lines, what):
    """Fail on any forbidden key or null anywhere under an override.

    This is the deep scan: it also catches forbidden keys nested
    inside values the merge would replace wholesale, which the
    merge walk itself never visits.
    """
    for val, p, _ in walk(v, path):
        if isinstance(val, dict):
            for k in val:
                if k in FORBIDDEN_OVERRIDE_KEYS:
                    lines.append("%s.%s: %s touches forbidden key %r "
                                 "(identity keys and transforms may "
                                 "not be overridden — spec §4)"
                                 % (p, k, what, k))
        if val is None:
            lines.append("%s: %s contains null — deletion by override "
                         "is not defined (spec §4)" % (p, what))


# ---------------------------------------------------------------------------
# Reference expansion (spec §5) — a load-time macro; renders nothing.
# ---------------------------------------------------------------------------

def substitute(v, eff):
    if is_slot(v):
        name = v["$param"]
        if name not in eff:
            raise ExpansionError("slot %r has no effective value" % name)
        return copy.deepcopy(eff[name])
    if isinstance(v, dict):
        return {k: substitute(x, eff) for k, x in v.items()}
    if isinstance(v, list):
        return [substitute(x, eff) for x in v]
    return v


def deep_merge(dst, src):
    for k, v in src.items():
        if (k in dst and isinstance(dst[k], dict) and isinstance(v, dict)
                and not is_slot(v) and not is_slot(dst[k])):
            deep_merge(dst[k], v)
        else:
            dst[k] = copy.deepcopy(v)
    return dst


def strip_pids(v):
    if isinstance(v, dict):
        v.pop("pid", None)
        for x in v.values():
            strip_pids(x)
    elif isinstance(v, list):
        for x in v:
            strip_pids(x)


def expand_instance(inst, defs, depth, counter):
    if depth > MAX_EXPANSION_DEPTH:
        raise ExpansionError("expansion depth exceeds %d — the cycle "
                             "check should have caught this"
                             % MAX_EXPANSION_DEPTH)
    name = inst["prefab"]
    defn = defs[name]["def"]
    declared = defn.get("params") or {}
    eff = {}
    for pname, decl in declared.items():
        if isinstance(decl, dict) and "default" in decl:
            eff[pname] = copy.deepcopy(decl["default"])
    for k, v in (inst.get("params") or {}).items():
        eff[k] = copy.deepcopy(v)
    tree = copy.deepcopy(defn["node"])
    for pid, partial in (inst.get("overrides") or {}).items():
        target = find_pid_node(tree, pid)
        if target is None:
            raise ExpansionError("override pid %r not found in prefab "
                                 "%r" % (pid, name))
        deep_merge(target, partial)
    tree = substitute(tree, eff)
    tree = expand_any(tree, defs, depth + 1, counter)
    strip_pids(tree)
    counter[0] += 1
    if "transform" in inst:
        tree = {"type": "group",
                "transform": copy.deepcopy(inst["transform"]),
                "children": [tree]}
    if "name" in inst:
        tree["name"] = inst["name"]
    return tree


def expand_any(v, defs, depth, counter):
    if isinstance(v, dict):
        if is_instance(v):
            return expand_instance(v, defs, depth, counter)
        return {k: expand_any(x, defs, depth, counter)
                for k, x in v.items()}
    if isinstance(v, list):
        return [expand_any(x, defs, depth, counter) for x in v]
    return v


def count_typed_nodes(v):
    n = 0
    if isinstance(v, dict):
        if "type" in v:
            n += 1
        for x in v.values():
            n += count_typed_nodes(x)
    elif isinstance(v, list):
        for x in v:
            n += count_typed_nodes(x)
    return n


# ---------------------------------------------------------------------------
# Per-scene validation
# ---------------------------------------------------------------------------

def build_scope(scene, scene_label, libraries):
    """name -> {"def":..., "source":...} from inline + library files.

    Returns (defs, dup_lines): a name defined twice is a failure —
    there is no shadowing in this extension (spec §2).
    """
    defs = {}
    dups = []

    def add(block, source):
        if not isinstance(block, dict):
            return
        for name, defn in block.items():
            if name in defs:
                dups.append("prefab %r defined by both %s and %s — "
                            "duplicate definitions are an error, "
                            "never shadowing (spec §2)"
                            % (name, defs[name]["source"], source))
            else:
                defs[name] = {"def": defn, "source": source}

    for lib_data, lib_label in libraries:
        if "prefabs" in lib_data:
            add(lib_data["prefabs"], lib_label)
    if isinstance(scene.get("prefabs"), dict):
        add(scene["prefabs"], scene_label + " (inline)")
    return defs, dups


def check_scene(scene_path, scene, scene_dups, libraries, lib_dups):
    """Run checks 1-6 for one scene. Returns (all_ok, report_lines)."""
    results = []  # (name, ok or None, headline, [lines]); None = SKIPPED
    scene_label = scene_path

    # ---- scope (needed by every check after 1) ------------------------------
    defs, dup_defs = build_scope(scene, scene_label, libraries)
    scene_instances = find_instances(scene.get("nodes", []), "nodes") \
        if isinstance(scene.get("nodes"), list) else []
    uses_prefabs = "prefabs" in scene or bool(scene_instances) \
        or any(is_instance(d["def"].get("node"))
               or find_instances(d["def"].get("node"), "")
               for d in defs.values())

    # ---- 1 parse -------------------------------------------------------------
    bad = []
    for k in scene_dups:
        bad.append("%s: duplicate JSON key %r in the scene file"
                   % (scene_path, k))
    for (lib_data, lib_label), dlist in zip(libraries, lib_dups):
        for k in dlist:
            bad.append("%s: duplicate JSON key %r" % (lib_label, k))
    if "format" in scene and scene["format"] != "talerender-scene":
        bad.append("%s: format=%r (want 'talerender-scene')"
                   % (scene_path, scene["format"]))
    if uses_prefabs and scene.get("version") != 2:
        bad.append("%s: prefab extension rides format version 2, "
                   "file declares version=%r (spec §7)"
                   % (scene_path, scene.get("version")))
    for lib_data, lib_label in libraries:
        if "version" in lib_data and lib_data["version"] != 2:
            bad.append("%s: library declares version=%r, extension "
                       "requires 2 (spec §7)"
                       % (lib_label, lib_data["version"]))
        if "prefabs" in lib_data and not isinstance(lib_data["prefabs"], dict):
            bad.append("%s: 'prefabs' must be an object" % lib_label)
    if "prefabs" in scene and not isinstance(scene["prefabs"], dict):
        bad.append("%s: 'prefabs' must be an object" % scene_path)
    results.append(("1 parse", not bad,
                    "%d prefab definitions in scope (%s), %d scene "
                    "instances, version %r"
                    % (len(defs),
                       ", ".join(sorted(defs)) or "none",
                       len(scene_instances), scene.get("version")), bad))

    # ---- 2 library ------------------------------------------------------------
    bad = list(dup_defs)
    n_slots = 0
    n_pids = 0
    for name in sorted(defs):
        defn = defs[name]["def"]
        src = defs[name]["source"]
        base = "prefabs.%s (%s)" % (name, src)
        if not isinstance(defn, dict):
            bad.append("%s: definition must be an object" % base)
            continue
        for k in defn:
            if k not in DEF_KEYS:
                bad.append("%s: unknown definition key %r (allowed: %s)"
                           % (base, k, ", ".join(sorted(DEF_KEYS))))
        if "node" not in defn:
            bad.append("%s: definition has no 'node' template" % base)
            continue
        if not is_node_like(defn["node"]):
            bad.append("%s.node: template root must be a node object "
                       "(with 'type' or 'prefab')" % base)
        declared = defn.get("params") or {}
        if "params" in defn and not isinstance(defn["params"], dict):
            bad.append("%s.params: must be an object" % base)
            declared = {}
        for pname, decl in declared.items():
            dp = "%s.params.%s" % (base, pname)
            if not isinstance(decl, dict):
                bad.append("%s: declaration must be an object" % dp)
                continue
            for k in decl:
                if k not in DECL_KEYS:
                    bad.append("%s: unknown declaration key %r (allowed: %s)"
                               % (dp, k, ", ".join(sorted(DECL_KEYS))))
            ptype = decl.get("type")
            if ptype not in PARAM_TYPES:
                bad.append("%s: type %r is not one of %s"
                           % (dp, ptype, ", ".join(PARAM_TYPES)))
                continue
            if "required" in decl and not isinstance(decl["required"], bool):
                bad.append("%s: 'required' must be a boolean" % dp)
            if decl.get("required") and "default" in decl:
                bad.append("%s: a required param may not carry a "
                           "default (spec §2)" % dp)
            if not decl.get("required") and "default" not in decl:
                bad.append("%s: param is neither required nor carries "
                           "a default — every param is one or the "
                           "other (spec §2)" % dp)
            if "default" in decl:
                if contains_slot(decl["default"]):
                    bad.append("%s: default contains a $param slot — "
                               "defaults must be literal (spec §2)" % dp)
                elif not type_ok(decl["default"], ptype):
                    bad.append("%s: default %r does not satisfy its "
                               "declared type %r"
                               % (dp, decl["default"], ptype))
        # slots inside the template subtree
        for slot, spath, pkey in find_slots(defn["node"],
                                            "%s.node" % base):
            n_slots += 1
            sname = slot.get("$param")
            if not isinstance(sname, str):
                bad.append("%s: $param value must be a param name "
                           "string, got %r" % (spath, sname))
            elif sname not in declared:
                bad.append("%s: $param %r is not declared by prefab "
                           "%r (declared: %s)"
                           % (spath, sname, name,
                              ", ".join(sorted(declared)) or "none"))
            if pkey == "prefab":
                bad.append("%s: a slot may not name a prefab — "
                           "instantiation targets are literal so the "
                           "reference graph is static (spec §2)" % spath)
        # pids: direct-scope uniqueness
        for pid, nodes_ in pid_table(defn["node"]).items():
            n_pids += 1
            if len(nodes_) > 1:
                bad.append("%s.node: pid %r declared %d times in the "
                           "template's direct scope — pids are unique "
                           "per prefab (spec §4)" % (base, pid, len(nodes_)))
    results.append(("2 library", not bad,
                    "%d definitions checked, %d slots, %d pids"
                    % (len(defs), n_slots, n_pids), bad))

    # ---- 3 instances -----------------------------------------------------------
    # Collect (instance, path, enclosing prefab name or None).
    all_instances = [(inst, p, None) for inst, p in scene_instances]
    for name in sorted(defs):
        defn = defs[name]["def"]
        if isinstance(defn, dict) and isinstance(defn.get("node"), (dict, list)):
            for inst, p in find_instances(defn["node"],
                                          "prefabs.%s.node" % name):
                # the template root itself, if an instance, is found too
                all_instances.append((inst, p, name))
    bad = []
    for slot, spath, _ in (find_slots(scene.get("nodes", []), "nodes")
                           if isinstance(scene.get("nodes"), list) else []):
        bad.append("%s: $param slot %r appears in the scene outside "
                   "any prefab template — slots are lexically scoped "
                   "to the prefab that declares them (spec §2)"
                   % (spath, slot.get("$param")))
    for inst, path, enclosing in all_instances:
        for k in inst:
            if k not in INSTANCE_KEYS:
                bad.append("%s: unknown instance key %r (closed "
                           "grammar — allowed: %s)"
                           % (path, k, ", ".join(sorted(INSTANCE_KEYS))))
        target = inst.get("prefab")
        if not isinstance(target, str):
            bad.append("%s: 'prefab' must be a literal prefab name "
                       "string, got %r" % (path, target))
            continue
        if target not in defs:
            bad.append("%s: unknown prefab %r (in scope: %s)"
                       % (path, target, ", ".join(sorted(defs)) or "none"))
            continue
        declared = defs[target]["def"].get("params") or {}
        supplied = inst.get("params") or {}
        if "params" in inst and not isinstance(inst["params"], dict):
            bad.append("%s.params: must be an object" % path)
            supplied = {}
        if "transform" in inst and not isinstance(inst["transform"], list):
            bad.append("%s.transform: must be an array of transform "
                       "ops (base format §5)" % path)
        if "overrides" in inst and not isinstance(inst["overrides"], dict):
            bad.append("%s.overrides: must be an object keyed by pid" % path)
        if "name" in inst and not isinstance(inst["name"], str):
            bad.append("%s.name: must be a string" % path)
        for pname, decl in declared.items():
            if isinstance(decl, dict) and decl.get("required") \
                    and pname not in supplied:
                bad.append("%s: missing required param %r for prefab "
                           "%r" % (path, pname, target))
        outer_declared = {}
        if enclosing is not None:
            outer_declared = defs[enclosing]["def"].get("params") or {}
        for pname, v in supplied.items():
            if pname not in declared:
                bad.append("%s.params.%s: prefab %r declares no param "
                           "%r (declared: %s)"
                           % (path, pname, target, pname,
                              ", ".join(sorted(declared)) or "none"))
                continue
            want = declared[pname].get("type")
            if is_slot(v):
                # forwarding from the enclosing template: types must
                # be compatible (exact, or color widening to string)
                sname = v.get("$param")
                if enclosing is None:
                    bad.append("%s.params.%s: slot %r in a scene "
                               "instance — scenes have no params to "
                               "forward (spec §2)" % (path, pname, sname))
                elif sname in outer_declared:
                    got = outer_declared[sname].get("type")
                    if got != want and not (got == "color" and want == "string"):
                        bad.append("%s.params.%s: forwarded slot %r "
                                   "is declared %r by prefab %r, but "
                                   "prefab %r declares %r as %r"
                                   % (path, pname, sname, got, enclosing,
                                      target, pname, want))
            elif not type_ok(v, want):
                bad.append("%s.params.%s: value %r does not satisfy "
                           "declared type %r of prefab %r (got %s)"
                           % (path, pname, v, want, target, type_name(v)))
    results.append(("3 instances", not bad,
                    "%d instances checked (%d in the scene, %d in "
                    "templates)"
                    % (len(all_instances), len(scene_instances),
                       len(all_instances) - len(scene_instances)), bad))

    # ---- 4 overrides -----------------------------------------------------------
    bad = []
    n_overrides = 0
    for inst, path, _enc in all_instances:
        ov = inst.get("overrides")
        if not isinstance(ov, dict):
            continue
        target = inst.get("prefab")
        if not isinstance(target, str) or target not in defs:
            continue  # already reported by check 3
        table = pid_table(defs[target]["def"]["node"])
        for pid, partial in ov.items():
            n_overrides += 1
            op = "%s.overrides.%s" % (path, pid)
            if pid not in table:
                bad.append("%s: prefab %r has no pid %r in its direct "
                           "scope (declared: %s)"
                           % (op, target, pid,
                              ", ".join(sorted(table)) or "none"))
                continue
            if not isinstance(partial, dict):
                bad.append("%s: override must be an object" % op)
                continue
            scan_forbidden(partial, op, bad, "override")
            merge_legality(table[pid][0], partial, op, bad)
    results.append(("4 overrides", not bad,
                    "%d override blocks checked" % n_overrides, bad))

    # ---- 5 cycles ---------------------------------------------------------------
    graph = {}
    for name in sorted(defs):
        defn = defs[name]["def"]
        edges = set()
        if isinstance(defn, dict) and "node" in defn:
            for inst, _p in find_instances(defn["node"], ""):
                t = inst.get("prefab")
                if isinstance(t, str) and t in defs:
                    edges.add(t)
        graph[name] = sorted(edges)
    bad = []
    WHITE, GRAY, BLACK = 0, 1, 2
    color = {n: WHITE for n in graph}
    reported = set()

    def dfs(u, stack):
        color[u] = GRAY
        stack.append(u)
        for w in graph[u]:
            if color[w] == GRAY:
                cyc = stack[stack.index(w):] + [w]
                key = tuple(cyc)
                if key not in reported:
                    reported.add(key)
                    bad.append("prefab cycle: %s — expansion would "
                               "not terminate (spec §6)"
                               % " -> ".join(cyc))
            elif color[w] == WHITE:
                dfs(w, stack)
        stack.pop()
        color[u] = BLACK

    for n in sorted(graph):
        if color[n] == WHITE:
            dfs(n, [])
    n_edges = sum(len(v) for v in graph.values())
    results.append(("5 cycles", not bad,
                    "reference graph: %d definitions, %d edges, %s"
                    % (len(graph), n_edges,
                       "acyclic" if not bad else "CYCLE PRESENT"), bad))

    # ---- 6 expansion --------------------------------------------------------------
    earlier_ok = all(ok for _n, ok, _h, _l in results if ok is not None)
    if not earlier_ok:
        results.append(("6 expansion", None,
                        "earlier checks failed; expansion is only "
                        "meaningful over a legal scope", []))
    elif not isinstance(scene.get("nodes"), list):
        results.append(("6 expansion", None,
                        "scene has no 'nodes' array (library-only "
                        "file)", []))
    else:
        bad = []
        counter = [0]
        try:
            expanded = expand_any(copy.deepcopy(scene["nodes"]),
                                  defs, 0, counter)
            residue = []
            for v, p, _ in walk(expanded, "nodes"):
                if isinstance(v, dict):
                    for k in ("prefab", "pid", "$param"):
                        if k in v:
                            residue.append("%s: residual key %r after "
                                           "expansion" % (p, k))
            bad.extend(residue)
            headline = ("%d scene instances expanded (of %d total "
                        "instantiations incl. nested), %d typed nodes "
                        "in the expanded tree, %d residual keys"
                        % (len(scene_instances), counter[0],
                           count_typed_nodes(expanded), len(residue)))
        except ExpansionError as e:
            bad.append("expansion failed: %s" % e)
            headline = "expansion did not complete"
        results.append(("6 expansion", not bad, headline, bad))

    # ---- report ---------------------------------------------------------------------
    lines = ["check_prefabs.py — %s (%d definitions in scope: %s)"
             % (scene_label, len(defs), ", ".join(sorted(defs)) or "none")]
    all_ok = True
    n_pass = 0
    n_total = 0
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
    return all_ok, lines


def main():
    ap = argparse.ArgumentParser(
        description="Validate the prefab/template extension layer of "
                    "TaleRender scene files (spec: "
                    "docs/integration/FORMAT-EXT-PREFABS-W14.md).")
    ap.add_argument("--library", action="append", default=[],
                    metavar="LIB.json",
                    help="prefab library file (an object with a "
                         "'prefabs' block); may be given more than "
                         "once. Scenes may also define prefabs "
                         "inline. A name defined twice is an error.")
    ap.add_argument("scenes", nargs="+", metavar="SCENE.json",
                    help="scene file(s) to validate")
    args = ap.parse_args()

    libraries = []
    lib_dups = []
    for p in args.library:
        data, dups = load_json_input(p, "library")
        if "prefabs" in data and not isinstance(data["prefabs"], dict):
            raise InputError("library: %s: 'prefabs' must be an object" % p)
        libraries.append((data, p))
        lib_dups.append(dups)

    all_ok = True
    for i, sp in enumerate(args.scenes):
        scene, scene_dups = load_json_input(sp, "scene")
        ok, lines = check_scene(sp, scene, scene_dups, libraries, lib_dups)
        all_ok = all_ok and ok
        if i:
            print()
        print("\n".join(lines))
    return 0 if all_ok else 1


if __name__ == "__main__":
    try:
        sys.exit(main())
    except InputError as e:
        print("ERROR: check_prefabs.py: %s" % e, file=sys.stderr)
        sys.exit(2)
