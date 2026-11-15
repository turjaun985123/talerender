#!/usr/bin/env python3
"""Static WGSL shader checker — structural cross-checks between the
WGSL shaders and the Rust code that binds them.

WHAT THIS IS
    A stdlib-only static checker (tools/ci/check_shaders.py, seat
    W29). For every .wgsl file in the repo, plus the two shader
    sources embedded in src/composer.rs (the COPY_WGSL const and
    the blur_wgsl() format!-generated template), it verifies:

      1. Delimiter balance (braces/parens/brackets) — a smoke
         layer only. This is NOT a WGSL parser.
      2. Entry points: at least one declared; names are sane
         identifiers and not WGSL reserved words; no duplicates.
      3. @group/@binding pairs: every binding attribute is paired
         with a group attribute; (group, binding) is unique per
         file; the type of a var<uniform> names a struct defined
         in the same file.
      4. Shader <-> Rust layout agreement. The shader->site map is
         built from source evidence (include_str! sites, const
         indirection, module() closure calls) — never from
         filename similarity. For each pipeline creation site the
         checker resolves the pipeline layout's bind-group layouts
         (literal entry lists, plus anchor-verified interpreters
         for the helper shapes this codebase actually uses:
         tex_bgl / ubo_layout / uniform_bgl / composer *_entry
         helpers) and compares binding numbers and binding KINDS
         (uniform / texture / sampler / storage) per group, plus
         the pipeline's vertex/fragment entry-point names, plus a
         stage-visibility coverage check (bindings the shader uses
         in a stage must be visible in that stage; visibility is
         tracked through the shader's own call graph).
      5. Uniform struct pairs: for the uniform structs listed in
         PAIRS below, the WGSL struct's field sequence and its
         size under the WGSL uniform layout rules are compared
         against the Rust side that fills the buffer — a Rust
         struct definition, or a hand-packed f32 assembly site
         whose byte count the checker reconstructs from the
         assembly's own operations (mat extends, vec4 groups,
         zero fills, pack_f32 arrays). For packed-array sites the
         checker verifies slot COUNT and total SIZE; field ORDER
         there is positional and is audited by citation in
         docs/integration/SHADERS-W29.md, not by this tool.

    Anything the extractor cannot resolve from source is reported
    UNRESOLVED with a reason. UNRESOLVED is a failing verdict for
    CI purposes: an unverifiable site is not a pass.

WHAT THIS IS NOT (see docs/integration/SHADERS-W29.md for the full
limits list)
    Not a WGSL compiler: type checking of expressions, name
    resolution, const-evaluation, texture/sampler compatibility
    rules beyond binding kind, vertex input @location matching,
    and backend codegen are naga/tint/wgpu territory and are NOT
    checked here. A PASS means "the structural contract between
    shader text and Rust binding code holds", nothing more.

EXIT CODES
    0  every shader PASS
    1  at least one FAIL or UNRESOLVED
    2  the checker itself could not complete discovery

USAGE
    python3 tools/ci/check_shaders.py [--repo PATH] [--json] [-v]
"""

import argparse
import json
import re
import sys
from pathlib import Path

# ----------------------------------------------------------------------
# Length-preserving text views.
#
# Two views of each source file, offset-aligned:
#   struct view: comments blanked AND string-literal contents blanked
#                (safe for brace matching / structural regexes)
#   text view:   comments blanked, strings intact (for include_str!,
#                entry_point literals, labels)
# Blanking replaces characters with spaces but keeps newlines, so
# line numbers computed on either view are true source lines.
# ----------------------------------------------------------------------


def _blank_range(chars, i, j):
    for k in range(i, j):
        if chars[k] != "\n":
            chars[k] = " "


def make_views(text, rust):
    chars = list(text)
    n = len(text)
    i = 0
    # string spans we discover while scanning (for the struct view)
    string_spans = []
    while i < n:
        c = text[i]
        nxt = text[i + 1] if i + 1 < n else ""
        if c == "/" and nxt == "/":
            j = text.find("\n", i)
            j = n if j < 0 else j
            _blank_range(chars, i, j)
            i = j
            continue
        if c == "/" and nxt == "*":
            if rust:  # Rust block comments nest
                depth = 0
                j = i
                while j < n:
                    if text.startswith("/*", j):
                        depth += 1
                        j += 2
                    elif text.startswith("*/", j):
                        depth -= 1
                        j += 2
                        if depth == 0:
                            break
                    else:
                        j += 1
            else:  # WGSL block comments do not nest
                j = text.find("*/", i + 2)
                j = n if j < 0 else j + 2
            _blank_range(chars, i, j)
            i = j
            continue
        if rust and c == "r" and nxt in ('"', "#"):
            # raw string r"..." or r#"..."#
            m = re.match(r'r(#+)"', text[i:])
            if m:
                hashes = m.group(1)
                close = '"' + hashes
                j = text.find(close, i + len(m.group(0)))
                j = n if j < 0 else j + len(close)
                string_spans.append((i, j))
                i = j
                continue
            if nxt == '"':
                j = text.find('"', i + 2)
                j = n if j < 0 else j + 1
                string_spans.append((i, j))
                i = j
                continue
        if rust and c == '"':
            j = i + 1
            while j < n:
                if text[j] == "\\":
                    j += 2
                    continue
                if text[j] == '"':
                    j += 1
                    break
                j += 1
            string_spans.append((i, j))
            i = j
            continue
        if rust and c == "'" and i + 2 < n and text[i + 2] == "'" and text[i + 1] != "'":
            # char literal like 'a' (lifetimes 'a are left alone)
            string_spans.append((i, i + 3))
            i += 3
            continue
        i += 1
    text_view = "".join(chars)
    if rust:
        for (a, b) in string_spans:
            _blank_range(chars, a, b)
    struct_view = "".join(chars)
    return struct_view, text_view


def line_of(text, pos):
    return text.count("\n", 0, pos) + 1


def brace_match(view, open_pos):
    """Index of the delimiter closing the one at open_pos, or -1."""
    pairs = {"{": "}", "(": ")", "[": "]"}
    open_c = view[open_pos]
    close_c = pairs[open_c]
    depth = 0
    i = open_pos
    n = len(view)
    while i < n:
        c = view[i]
        if c == open_c:
            depth += 1
        elif c == close_c:
            depth -= 1
            if depth == 0:
                return i
        i += 1
    return -1


def split_top_level(s, sep=","):
    """Split on sep at delimiter depth 0."""
    out = []
    depth = 0
    cur = []
    for c in s:
        if c in "([{":
            depth += 1
        elif c in ")]}":
            depth -= 1
        if c == sep and depth == 0:
            out.append("".join(cur))
            cur = []
        else:
            cur.append(c)
    if cur:
        out.append("".join(cur))
    return [p.strip() for p in out if p.strip()]


# ----------------------------------------------------------------------
# WGSL model
# ----------------------------------------------------------------------

WGSL_RESERVED = {
    "array", "atomic", "bool", "f16", "f32", "i32", "u32", "mat2x2f",
    "mat3x3f", "mat4x4f", "ptr", "sampler", "struct", "texture_2d",
    "var", "vec2f", "vec3f", "vec4f", "vertex", "fragment", "compute",
    "const", "fn", "let", "return", "if", "else", "for", "loop",
    "while", "switch", "case", "default", "break", "continue",
    "discard", "enable", "requires", "override", "storage", "uniform",
    "private", "workgroup", "function",
}

IDENT_RE = re.compile(r"^[A-Za-z_][A-Za-z0-9_]*$")


class WgslShader:
    def __init__(self, shader_id, source, origin):
        self.id = shader_id          # e.g. 'src/shader_pbr.wgsl' or 'embedded:blur_wgsl'
        self.origin = origin         # human description of where the source lives
        self.source = source
        self.view, _ = make_views(source, rust=False)
        self.structs = {}            # name -> [(field, type)] in order
        self.struct_lines = {}       # name -> line
        self.globals = []            # dicts: group, binding, name, kind, type, line
        self.entries = []            # (stage, name, line)
        self.fns = {}                # fn name -> body text (struct view)
        self.problems = []           # (level, message) level in FAIL/INFO
        self.parse()

    def fail(self, msg):
        self.problems.append(("FAIL", msg))

    def info(self, msg):
        self.problems.append(("INFO", msg))

    def parse(self):
        v = self.view
        # delimiter balance smoke layer
        for open_c, close_c in (("{", "}"), ("(", ")"), ("[", "]")):
            if v.count(open_c) != v.count(close_c):
                self.fail(
                    f"unbalanced delimiters: {v.count(open_c)}x '{open_c}' vs "
                    f"{v.count(close_c)}x '{close_c}' (smoke check, not a parse)")
        # structs
        for m in re.finditer(r"\bstruct\s+(\w+)\s*\{", v):
            name = m.group(1)
            if name in self.structs:
                self.fail(f"duplicate struct '{name}' (line {line_of(v, m.start())})")
            close = brace_match(v, m.end() - 1)
            if close < 0:
                self.fail(f"struct '{name}' body does not close")
                continue
            body = v[m.end():close]
            fields = []
            for fm in re.finditer(r"(\w+)\s*:\s*([^,;]+?)\s*[,;]", body):
                fields.append((fm.group(1), fm.group(2).strip()))
            self.structs[name] = fields
            self.struct_lines[name] = line_of(v, m.start())
        # globals with @group/@binding
        paired = 0
        for m in re.finditer(
            r"@group\(\s*(\d+)\s*\)\s*@binding\(\s*(\d+)\s*\)\s*"
            r"var(?:<([^>]+)>)?\s+(\w+)\s*:\s*([^;]+);", v):
            paired += 1
            group, binding = int(m.group(1)), int(m.group(2))
            space, name, typ = m.group(3), m.group(4), m.group(5).strip()
            kind = self._kind(space, typ)
            self.globals.append({
                "group": group, "binding": binding, "name": name,
                "kind": kind, "type": typ, "line": line_of(v, m.start()),
            })
        n_group = len(re.findall(r"@group\s*\(", v))
        n_binding = len(re.findall(r"@binding\s*\(", v))
        if n_group != paired or n_binding != paired:
            self.fail(
                f"@group/@binding pairing broken: {n_group} @group, "
                f"{n_binding} @binding, only {paired} paired resource declarations")
        seen = {}
        for g in self.globals:
            key = (g["group"], g["binding"])
            if key in seen:
                self.fail(
                    f"duplicate (group {key[0]}, binding {key[1]}): "
                    f"'{seen[key]}' and '{g['name']}' (line {g['line']})")
            seen[key] = g["name"]
            if not IDENT_RE.match(g["name"]):
                self.fail(f"resource name '{g['name']}' is not a sane identifier")
            if g["kind"] == "uniform":
                tname = g["type"]
                if tname not in self.structs:
                    self.fail(
                        f"var<uniform> '{g['name']}' has type '{tname}' which is "
                        f"not a struct defined in this file (line {g['line']})")
            if g["kind"] == "unknown":
                self.fail(
                    f"resource '{g['name']}' has unrecognised kind "
                    f"(space={g['space'] if 'space' in g else ''}, type '{g['type']}')")
        # entry points
        for m in re.finditer(
            r"@(vertex|fragment|compute)\s+(?:@\w+(?:\([^)]*\))?\s+)*fn\s+(\w+)", v):
            stage, name = m.group(1), m.group(2)
            self.entries.append((stage, name, line_of(v, m.start())))
            if not IDENT_RE.match(name) or name in WGSL_RESERVED:
                self.fail(f"entry point name '{name}' is not sane / is reserved")
        names = [e[1] for e in self.entries]
        if not self.entries:
            self.fail("no entry points declared")
        if len(names) != len(set(names)):
            self.fail(f"duplicate entry point names: {names}")
        # function bodies (for the stage-usage call graph)
        for m in re.finditer(r"\bfn\s+(\w+)\s*\(", v):
            close_paren = brace_match(v, m.end() - 1)
            if close_paren < 0:
                continue
            brace = v.find("{", close_paren)
            if brace < 0:
                continue
            close = brace_match(v, brace)
            if close >= 0:
                self.fns[m.group(1)] = v[brace:close + 1]

    @staticmethod
    def _kind(space, typ):
        if space and "uniform" in space:
            return "uniform"
        if space and "storage" in space:
            return "storage"
        if typ.startswith("texture_"):
            return "texture"
        if typ.startswith("sampler"):
            return "sampler"
        if space is None and not typ.startswith("texture_"):
            return "unknown"
        return "unknown"

    def group_map(self):
        out = {}
        for g in self.globals:
            out.setdefault(g["group"], {})[g["binding"]] = g
        return out

    def stage_usage(self):
        """var name -> set of stages that (transitively) reference it."""
        calls = {}
        for name, body in self.fns.items():
            calls[name] = set(re.findall(r"\b(\w+)\s*\(", body)) & set(self.fns)
        usage = {}
        for stage, ename, _line in self.entries:
            seen = set()
            stack = [ename]
            while stack:
                f = stack.pop()
                if f in seen:
                    continue
                seen.add(f)
                stack.extend(calls.get(f, ()))
            body_text = "\n".join(self.fns.get(f, "") for f in seen)
            for g in self.globals:
                if re.search(r"\b" + re.escape(g["name"]) + r"\b", body_text):
                    usage.setdefault(g["name"], set()).add(stage)
        return usage


# WGSL uniform-address-space layout (subset: scalars, vectors,
# matrices, arrays, structs — the types this engine uses).
def _round_up(x, a):
    return (x + a - 1) // a * a


def wgsl_type_layout(t, structs, uniform=True, depth=0):
    """(size, align) of a WGSL type, or None if unrecognised."""
    t = t.strip()
    if t in ("f32", "i32", "u32", "bool"):
        return (4, 4)
    m = re.match(r"vec(\d)<(.+)>$", t) or re.match(r"vec(\d)([fiub])$", t)
    if m:
        n = int(m.group(1))
        return ((8, 8) if n == 2 else (12, 16) if n == 3 else (16, 16))
    m = re.match(r"mat(\d)x(\d)<(.+)>$", t) or re.match(r"mat(\d)x(\d)([fiub])$", t)
    if m:
        cols, rows = int(m.group(1)), int(m.group(2))
        col = _round_up(4 * rows, 8 if rows == 2 else 16)
        return (cols * col, 8 if rows == 2 else 16)
    m = re.match(r"array<(.+),\s*(\d+)>$", t)
    if m and depth < 8:
        inner = wgsl_type_layout(m.group(1), structs, uniform, depth + 1)
        if inner is None:
            return None
        stride = _round_up(inner[0], inner[1])
        if uniform:
            stride = _round_up(stride, 16)
        return (stride * int(m.group(2)), _round_up(inner[1], 16) if uniform else inner[1])
    if t in structs and depth < 8:
        return struct_layout(structs[t], structs, uniform, depth + 1)
    return None


def struct_layout(fields, structs, uniform=True, depth=0):
    offset = 0
    max_align = 1
    for _name, t in fields:
        lay = wgsl_type_layout(t, structs, uniform, depth)
        if lay is None:
            return None
        size, align = lay
        if uniform and (t.startswith("array<") or t in structs):
            align = _round_up(align, 16)
        offset = _round_up(offset, align) + size
        max_align = max(max_align, align)
    if uniform:
        max_align = _round_up(max_align, 16)
    return (_round_up(offset, max_align), max_align)


# ----------------------------------------------------------------------
# Rust model
#
# Association rule (documented, auditable): a pipeline is matched to
# the layout / bind-group-layout definitions of the names it uses,
# resolved to the NEAREST PRECEDING definition of that name in the
# same file. Every match prints its evidence lines, so a human can
# audit each association. Shapes the extractor does not recognise
# resolve to UNRESOLVED, never to a guess.
# ----------------------------------------------------------------------

STAGE_OF = {"VERTEX": "vertex", "FRAGMENT": "fragment", "COMPUTE": "compute"}


def kind_of_entry(obj_text):
    if "BufferBindingType::Uniform" in obj_text:
        return "uniform"
    if "BufferBindingType::Storage" in obj_text:
        return "storage"
    if "BindingType::Texture" in obj_text:
        return "texture"
    if "BindingType::Sampler" in obj_text:
        return "sampler"
    return "unknown"


def stages_of_visibility(obj_text):
    m = re.search(r"visibility:\s*(.+?),\s*ty:", obj_text, re.S)
    if not m:
        m = re.search(r"visibility:\s*(.+?)\}", obj_text, re.S)
    if not m:
        return None
    expr = m.group(1)
    stages = {STAGE_OF[s] for s in re.findall(r"ShaderStages::(VERTEX|FRAGMENT|COMPUTE)", expr)}
    return stages if stages else None


class RustFile:
    def __init__(self, path, root):
        self.path = path
        self.rel = str(path.relative_to(root))
        raw = path.read_text()
        self.sview, self.tview = make_views(raw, rust=True)
        self.regions = []        # (name, start, end, params or None)
        self.const_shaders = {}  # const name -> shader id
        self.module_vars = {}    # var name -> (shader id, line)
        self.module_def_lines = []  # lines of direct create_shader_module defs
        self.bgl_defs = []       # dicts: var, line, entries or None, dynamic reason
        self.layout_defs = []    # dicts: var or None, line, [bgl var names], pos
        self.pipelines = []      # resolved pipeline dicts
        self.unresolved = []     # (what, reason, line)
        self._parse_regions()
        self._parse_consts()
        self._parse_bgl_defs()
        self._parse_layout_defs()
        self._parse_modules()
        self._parse_pipelines()

    # -- regions ------------------------------------------------------
    def _parse_regions(self):
        v = self.sview
        for m in re.finditer(r"\bfn\s+(\w+)\s*\(", v):
            close_paren = brace_match(v, m.end() - 1)
            if close_paren < 0:
                continue
            brace = v.find("{", close_paren)
            if brace < 0:
                continue
            close = brace_match(v, brace)
            if close >= 0:
                params = split_top_level(v[m.end():close_paren])
                pnames = [p.split(":")[0].strip().lstrip("&").strip()
                          for p in params]
                self.regions.append((m.group(1), m.start(), close, pnames))
        for m in re.finditer(r"\blet\s+(?:mut\s+)?(\w+)\s*(?::[^=]+)?=\s*\|", v):
            bar = v.find("|", m.end())
            if bar < 0:
                continue
            brace = v.find("{", bar)
            if brace < 0:
                continue
            close = brace_match(v, brace)
            if close >= 0:
                params = split_top_level(v[m.end():bar])
                pnames = [p.split(":")[0].strip().lstrip("&").strip()
                          for p in params]
                self.regions.append((m.group(1), m.start(), close, pnames))
        self.regions.sort(key=lambda r: (r[1], -(r[2] - r[1])))

    def owner(self, pos):
        best = None
        for r in self.regions:
            if r[1] <= pos <= r[2]:
                if best is None or (r[2] - r[1]) < (best[2] - best[1]):
                    best = r
        return best  # (name, start, end, params) or None

    def region_by_name(self, name):
        for r in self.regions:
            if r[0] == name:
                return r
        return None

    # -- consts --------------------------------------------------------
    def _parse_consts(self):
        for m in re.finditer(
            r"\bconst\s+(\w+)\s*:\s*&str\s*=\s*include_str!\(\s*\"([^\"]+\.wgsl)\"\s*\)",
            self.tview):
            self.const_shaders[m.group(1)] = "src/" + Path(m.group(2)).name

    def _shader_ref_in(self, text):
        """Resolve a shader-source expression to a shader id."""
        m = re.search(r"include_str!\(\s*\"([^\"]+\.wgsl)\"\s*\)", text)
        if m:
            return "src/" + Path(m.group(1)).name
        if re.search(r"\bblur_wgsl\s*\(", text):
            return "embedded:blur_wgsl"
        if re.search(r"\bCOPY_WGSL\b", text):
            return "embedded:COPY_WGSL"
        for name, sid in self.const_shaders.items():
            if re.search(r"\b" + re.escape(name) + r"\b", text):
                return sid
        return None

    # -- bind group layouts -------------------------------------------
    def _assigned_var_before(self, pos, window=300):
        seg = self.sview[max(0, pos - window):pos]
        m = None
        for m in re.finditer(r"\blet\s+(?:mut\s+)?(\w+)\s*(?::[^=;]+)?=\s*(?:device\s*\.\s*)?$", seg):
            pass
        return m.group(1) if m else None

    def _parse_bgl_defs(self):
        v = self.sview
        for m in re.finditer(r"create_bind_group_layout\s*\(", v):
            open_paren = v.find("(", m.start())
            close = brace_match(v, open_paren)
            if close < 0:
                continue
            block = v[m.start():close + 1]
            var = self._assigned_var_before(m.start())
            owner = self.owner(m.start())
            if var is None and owner is not None:
                var = owner[0]  # tail expression of a helper fn
            entries = None
            reason = None
            em = re.search(r"entries:\s*&\s*\[", block)
            if em:
                bopen = block.find("[", em.start())
                bclose = brace_match(block, bopen)
                if bclose >= 0:
                    entries = self._parse_entry_slice(block[bopen:bclose + 1])
                else:
                    reason = "entries slice does not close"
            elif re.search(r"entries:\s*&?\s*entries\b", block):
                reason = "dynamic entries (built by pushes)"
            elif re.search(r"entries\s*\}", block) or re.search(r"entries\s*\)", block):
                reason = "entries passed through (helper parameter)"
            else:
                reason = "entries expression not recognised"
            self.bgl_defs.append({
                "var": var, "line": line_of(v, m.start()),
                "entries": entries, "reason": reason, "pos": m.start(),
            })

    def _parse_entry_slice(self, slice_text):
        entries = {}
        for m in re.finditer(r"BindGroupLayoutEntry\s*\{", slice_text):
            close = brace_match(slice_text, m.end() - 1)
            if close < 0:
                continue
            obj = slice_text[m.start():close + 1]
            bm = re.search(r"binding:\s*(\d+)", obj)
            if not bm:
                return None  # non-literal binding: bail to UNRESOLVED
            entries[int(bm.group(1))] = {
                "kind": kind_of_entry(obj),
                "stages": stages_of_visibility(obj),
            }
        return entries

    def bgl_def_for(self, var, before_pos):
        """Nearest preceding literal BGL def for var, or (None, reason)."""
        cands = [d for d in self.bgl_defs if d["var"] == var and d["pos"] < before_pos]
        if not cands:
            return None, f"no create_bind_group_layout definition found for '{var}'"
        d = cands[-1]
        if d["entries"] is None:
            return None, f"'{var}' (line {d['line']}) is not a literal layout: {d['reason']}"
        return d, None

    # -- helper interpreters (anchor-verified) -------------------------
    def _helper_entries(self, var, before_pos):
        """Resolve BGL entries for layouts built by this codebase's
        helper shapes. Returns (entries, evidence) or (None, reason).
        Each interpreter first verifies the helper's definition still
        has the shape it interprets; a changed shape is UNRESOLVED."""
        t = self.tview
        # assignment via helper call: let VAR = helper(args);
        pat = re.compile(
            r"\blet\s+(?:mut\s+)?" + re.escape(var) +
            r"\s*(?::[^=;]+)?=\s*(\w+)\s*\(")
        found = None
        for m in pat.finditer(t):
            if m.start() < before_pos:
                found = m
        if not found:
            return None, f"no helper-call assignment found for '{var}'"
        helper = found.group(1)
        open_paren = t.find("(", found.start())
        close = brace_match(self.sview, open_paren)
        args = split_top_level(t[open_paren + 1:close])
        line = line_of(t, found.start())

        if helper in ("tex_bgl", "ubo_layout", "uniform_bgl"):
            region = self.region_by_name(helper)
            if region is None:
                return None, f"helper '{helper}' not defined in this file"
            body = self.sview[region[1]:region[2] + 1]
            if helper == "ubo_layout":
                if not (re.search(r"binding:\s*0", body)
                        and "BufferBindingType::Uniform" in body):
                    return None, "ubo_layout definition changed shape"
                stages = {STAGE_OF[s] for s in
                          re.findall(r"ShaderStages::(VERTEX|FRAGMENT|COMPUTE)",
                                     args[1] if len(args) > 1 else "")}
                return ({0: {"kind": "uniform", "stages": stages or None}},
                        f"{self.rel}:{line} ubo_layout(...) anchor-verified "
                        f"against definition at line {line_of(self.sview, region[1])}")
            if helper == "uniform_bgl":
                if not (re.search(r"binding:\s*0", body)
                        and "BufferBindingType::Uniform" in body):
                    return None, "uniform_bgl definition changed shape"
                frag = len(args) > 1 and args[1] == "true"
                stages = {"vertex", "fragment"} if frag else {"vertex"}
                return ({0: {"kind": "uniform", "stages": stages}},
                        f"{self.rel}:{line} uniform_bgl(fragment={frag}) "
                        f"anchor-verified against definition")
            # tex_bgl: two definitions exist in the tree (a fn in
            # scenefile.rs with the uniform fixed in, a closure in
            # ab1.rs/film.rs parameterised by with_uniform).
            has_param = "with_uniform" in body
            if not ("BindingType::Texture" in body and "BindingType::Sampler" in body
                    and "BufferBindingType::Uniform" in body):
                return None, "tex_bgl definition changed shape"
            with_uniform = True
            if has_param:
                with_uniform = bool(args) and args[0] == "true"
            # the uniform entry's visibility differs between the two
            # definitions in the tree — read it from the entry object
            # itself: the span from the object's start to its
            # BufferBindingType::Uniform token
            ustages = {"vertex"}
            upos = body.find("BufferBindingType::Uniform")
            if upos >= 0:
                ostart = body.rfind("BindGroupLayoutEntry", 0, upos)
                seg = body[ostart if ostart >= 0 else 0:upos]
                vm = re.findall(r"ShaderStages::(VERTEX|FRAGMENT|COMPUTE)", seg)
                if vm:
                    ustages = {STAGE_OF[s] for s in vm}
            if with_uniform:
                ent = {0: {"kind": "uniform", "stages": ustages},
                       1: {"kind": "texture", "stages": {"fragment"}},
                       2: {"kind": "sampler", "stages": {"fragment"}}}
            else:
                ent = {0: {"kind": "texture", "stages": {"fragment"}},
                       1: {"kind": "sampler", "stages": {"fragment"}}}
            return (ent, f"{self.rel}:{line} tex_bgl(with_uniform={with_uniform}) "
                         f"anchor-verified against definition")
        if helper == "bgl":
            # composer.rs: bgl("label", &[ubo_entry(0), tex_entry(1), ...])
            kinds = {}
            kind_of_helper = {"ubo_entry": "uniform", "tex_entry": "texture",
                              "smp_entry": "sampler"}
            for hname, want in (("ubo_entry", "BufferBindingType::Uniform"),
                                ("tex_entry", "BindingType::Texture"),
                                ("smp_entry", "BindingType::Sampler")):
                region = self.region_by_name(hname)
                if region is None:
                    return None, f"composer helper '{hname}' not defined"
                body = self.sview[region[1]:region[2] + 1]
                if want not in body:
                    return None, f"composer helper '{hname}' changed shape"
                stages = {STAGE_OF[s] for s in re.findall(
                    r"ShaderStages::(VERTEX|FRAGMENT|COMPUTE)", body)}
                kinds[hname] = (kind_of_helper[hname], stages)
            if len(args) < 2:
                return None, "bgl(...) call has no entries argument"
            ent = {}
            for call in re.finditer(r"(\w+)_entry\s*\(\s*(\d+)\s*\)", args[1]):
                hname = call.group(1) + "_entry"
                if hname not in kinds:
                    return None, f"unknown entry helper '{hname}'"
                kind, stages = kinds[hname]
                ent[int(call.group(2))] = {"kind": kind, "stages": stages}
            if not ent:
                return None, "bgl(...) entries argument held no *_entry calls"
            return (ent, f"{self.rel}:{line} bgl(...) with *_entry helpers "
                         f"anchor-verified")
        return None, f"helper '{helper}' has no verified interpreter"

    def resolve_bgl(self, var, before_pos):
        """(entries dict, evidence str) or (None, reason)."""
        d, reason = self.bgl_def_for(var, before_pos)
        if d is not None:
            return d["entries"], f"{self.rel}:{d['line']} literal layout '{var}'"
        ent, ev = self._helper_entries(var, before_pos)
        if ent is not None:
            return ent, ev
        return None, reason + "; " + ev

    # -- pipeline layouts ------------------------------------------------
    def _parse_layout_defs(self):
        v = self.sview
        for m in re.finditer(r"create_pipeline_layout\s*\(", v):
            open_paren = v.find("(", m.start())
            close = brace_match(v, open_paren)
            if close < 0:
                continue
            block = v[m.start():close + 1]
            bm = re.search(r"bind_group_layouts:\s*&\s*\[([^\]]*)\]", block)
            if not bm:
                continue
            # wgpu 30 wraps each layout in Some(..) — unwrap
            # before resolving the variable name.
            def _unwrap(x):
                x = x.strip()
                if x.startswith("Some(") and x.endswith(")"):
                    x = x[5:-1].strip()
                return x.lstrip("&").strip()
            names = [_unwrap(x)
                     for x in bm.group(1).split(",") if x.strip()]
            var = self._assigned_var_before(m.start())
            self.layout_defs.append({
                "var": var, "line": line_of(v, m.start()),
                "groups": names, "pos": m.start(),
            })

    def layout_groups_for(self, var, before_pos):
        cands = [d for d in self.layout_defs
                 if d["var"] == var and d["pos"] < before_pos]
        if not cands:
            return None, f"no create_pipeline_layout definition found for '{var}'"
        d = cands[-1]
        return d["groups"], f"{self.rel}:{d['line']} pipeline layout '{var}'"

    # -- shader modules --------------------------------------------------
    def _parse_modules(self):
        t = self.tview
        for m in re.finditer(r"\blet\s+(?:mut\s+)?(\w+)\s*(?::[^=;]+)?=\s*device\s*\.\s*create_shader_module", t):
            sid = self._shader_ref_in(t[m.start():m.start() + 500])
            if sid:
                self.module_vars[m.group(1)] = (sid, line_of(t, m.start()))
                self.module_def_lines.append(line_of(t, m.start()))
        for m in re.finditer(r"\blet\s+(?:mut\s+)?(\w+)\s*(?::[^=;]+)?=\s*module\s*\(", t):
            open_paren = t.find("(", m.start())
            close = brace_match(self.sview, open_paren)
            if close < 0:
                continue
            args = split_top_level(t[open_paren + 1:close])
            if len(args) >= 2:
                sid = self._shader_ref_in(args[1])
                if sid:
                    self.module_vars[m.group(1)] = (sid, line_of(t, m.start()))

    # -- pipelines ---------------------------------------------------------
    def _call_sites(self, name, exclude_pos):
        """Arg lists (text view) of calls to name in this file."""
        out = []
        for m in re.finditer(r"(?<![\w.:])" + re.escape(name) + r"\s*\(", self.tview):
            if abs(m.start() - exclude_pos) < 4:
                continue
            open_paren = self.tview.find("(", m.start())
            close = brace_match(self.sview, open_paren)
            if close < 0:
                continue
            out.append((m.start(), split_top_level(self.tview[open_paren + 1:close])))
        return out

    def _parse_pipelines(self):
        v, t = self.sview, self.tview
        for m in re.finditer(r"create_render_pipeline\s*\(", v):
            open_paren = v.find("(", m.start())
            close = brace_match(v, open_paren)
            if close < 0:
                continue
            block_s = v[m.start():close + 1]
            block_t = t[m.start():close + 1]
            owner = self.owner(m.start())
            entry_seq = []
            for em in re.finditer(r"entry_point:\s*(?:Some\(\s*)?(?:\"(\w+)\"|(\w+))", block_t):
                if em.group(1) is not None:
                    entry_seq.append(("lit", em.group(1)))
                else:
                    entry_seq.append(("expr", em.group(2)))
            pipe = {
                "file": self.rel, "line": line_of(v, m.start()),
                "owner": owner[0] if owner else "<top level>",
                "module_exprs": re.findall(r"module:\s*&?\s*(\w+)", block_s),
                "entry_seq": entry_seq,
                "layout_kind": None, "layout_var": None, "layout_pos": None,
                "pos": m.start(),
            }
            if re.search(r"layout:\s*Some\(\s*&?\s*device\s*\.\s*create_pipeline_layout", block_s):
                pipe["layout_kind"] = "inline"
                inner = [d for d in self.layout_defs
                         if m.start() < d["pos"] < close]
                if inner:
                    pipe["layout_pos"] = inner[0]["pos"]
                    pipe["layout_groups"] = inner[0]["groups"]
                    pipe["layout_evidence"] = (
                        f"{self.rel}:{inner[0]['line']} inline pipeline layout")
            else:
                lm = re.search(r"layout:\s*Some\(\s*&?\s*(\w+)\s*\)", block_s)
                if lm:
                    pipe["layout_var"] = lm.group(1)
                    # is it a local var or an owner param?
                    if owner and owner[3] and lm.group(1) in owner[3]:
                        pipe["layout_kind"] = "param"
                    else:
                        pipe["layout_kind"] = "var"
                elif re.search(r"layout:\s*Some\(", block_s):
                    pipe["layout_kind"] = "unresolved"
            self.pipelines.append(pipe)
        # fs_pipeline-style fn templates are handled by call sites in
        # resolve_sites(): their single inner pipeline uses params.


# ----------------------------------------------------------------------
# Site resolution: pipelines -> (shader, groups, entry points)
# ----------------------------------------------------------------------

STAGE_NAMES = ("vertex", "fragment", "compute")


def resolve_sites(rf):
    """Return (sites, unresolved) for one RustFile.
    site: dict(shader, where, line, groups=[entries,...], vent, fent)
    """
    sites = []
    unresolved = []

    def finish(pipe, module_var, groups_spec, vent, fent, via, ref_pos):
        sid, _line = rf.module_vars.get(module_var, (None, None))
        where = f"{rf.rel}:{pipe['line']} ({pipe['owner']}{via})"
        if sid is None:
            unresolved.append((where, f"module variable '{module_var}' not "
                                      f"resolved to a shader source"))
            return
        groups = []
        for var in groups_spec:
            ent, ev = rf.resolve_bgl(var, ref_pos)
            if ent is None:
                unresolved.append((where, f"bind group layout '{var}': {ev}"))
                return
            groups.append((ent, ev))
        sites.append({
            "shader": sid, "where": where, "line": pipe["line"],
            "groups": groups, "vent": vent, "fent": fent,
        })

    def groups_for_layout(pipe, mapping, ref_pos):
        """Ordered BGL var names for a pipeline, or (None, reason)."""
        if pipe["layout_kind"] == "inline":
            names = []
            for g in pipe.get("layout_groups", []):
                if mapping and g in mapping:
                    a = mapping[g].strip().lstrip("&").strip()
                    names.append(a)
                else:
                    names.append(g)
            return names, pipe.get("layout_evidence", "inline layout")
        if pipe["layout_kind"] == "var":
            names, ev = rf.layout_groups_for(pipe["layout_var"], ref_pos)
            if names is None:
                return None, ev
            return names, ev
        if pipe["layout_kind"] == "param":
            var = pipe["layout_var"]
            if mapping and var in mapping:
                lvar = mapping[var].strip().lstrip("&").strip()
                names, ev = rf.layout_groups_for(lvar, ref_pos)
                if names is None:
                    return None, ev
                return names, ev
            return None, f"layout parameter '{var}' has no call-site mapping"
        return None, "pipeline layout expression not recognised"

    for pipe in rf.pipelines:
        owner = rf.owner(pipe["pos"])
        params = owner[3] if owner else []
        entry_seq = pipe["entry_seq"]
        exprs = [e[1] for e in entry_seq if e[0] == "expr"]
        needs_calls = (
            any(e in params for e in pipe["module_exprs"])
            or pipe["layout_kind"] == "param"
            or any(e in params for e in exprs)
            or (not pipe["module_exprs"] and "module" in params))
        if not needs_calls:
            module_var = pipe["module_exprs"][0] if pipe["module_exprs"] else None
            if module_var is None:
                unresolved.append((f"{rf.rel}:{pipe['line']}",
                                   "pipeline has no recognisable module reference"))
                continue
            names, ev = groups_for_layout(pipe, None, pipe["pos"])
            if names is None:
                unresolved.append((f"{rf.rel}:{pipe['line']} ({pipe['owner']})", ev))
                continue
            vent = entry_seq[0][1] if len(entry_seq) > 0 else None
            fent = entry_seq[1][1] if len(entry_seq) > 1 else None
            finish(pipe, module_var, names, vent, fent, "", pipe["pos"])
            continue
        # template pipeline: resolve per call site of the owner
        if owner is None:
            unresolved.append((f"{rf.rel}:{pipe['line']}", "template owner unknown"))
            continue
        calls = rf._call_sites(owner[0], owner[1])
        calls = [(p, a) for (p, a) in calls
                 if not (owner[1] <= p <= owner[2])
                 and "fn " not in rf.tview[max(0, p - 4):p]]
        if not calls:
            unresolved.append((f"{rf.rel}:{pipe['line']} ({owner[0]})",
                               "template pipeline has no resolvable call sites"))
            continue
        for cpos, args in calls:
            mapping = {}
            for i, pname in enumerate(params):
                if i < len(args):
                    mapping[pname] = args[i]
            if pipe["module_exprs"]:
                mexpr = pipe["module_exprs"][0]
                module_var = (mapping[mexpr].strip().lstrip("&").strip()
                              if mexpr in mapping else mexpr)
            elif "module" in mapping:
                module_var = mapping["module"].strip().lstrip("&").strip()
            else:
                unresolved.append((f"{rf.rel}:{line_of(rf.tview, cpos)}",
                                   f"call site of {owner[0]} has no module argument"))
                continue
            names, ev = groups_for_layout(pipe, mapping, cpos)
            if names is None:
                unresolved.append((f"{rf.rel}:{line_of(rf.tview, cpos)} "
                                   f"(call of {owner[0]})", ev))
                continue
            seq = []
            for kind, val in entry_seq:
                if kind == "lit":
                    seq.append(val)
                elif val in mapping:
                    seq.append(mapping[val].strip().strip('"'))
                else:
                    seq.append(None)
            vent = seq[0] if len(seq) > 0 else None
            fent = seq[1] if len(seq) > 1 else None
            finish(pipe, module_var, names, vent, fent,
                   f" via {owner[0]} call at line {line_of(rf.tview, cpos)}",
                   cpos)
    return sites, unresolved


def compare_site(shader, site):
    """Compare one resolved site against the shader. Returns
    (verdict, notes) with verdict in PASS/FAIL."""
    notes = []
    fails = []
    gmap = shader.group_map()
    usage = shader.stage_usage()
    layout_groups = site["groups"]
    if gmap and max(gmap) >= len(layout_groups):
        fails.append(
            f"shader uses group {max(gmap)} but the pipeline layout has "
            f"only {len(layout_groups)} group(s)")
    for gidx, wgsl_bindings in sorted(gmap.items()):
        if gidx >= len(layout_groups):
            break
        entries, ev = layout_groups[gidx]
        for b, g in sorted(wgsl_bindings.items()):
            rent = entries.get(b)
            if rent is None:
                fails.append(
                    f"group {gidx} binding {b} ('{g['name']}', {g['kind']}) "
                    f"missing from Rust layout [{ev}]")
            elif rent["kind"] != g["kind"]:
                fails.append(
                    f"group {gidx} binding {b}: shader kind {g['kind']} "
                    f"vs Rust kind {rent['kind']} [{ev}]")
            else:
                used = usage.get(g["name"], set())
                vis = rent.get("stages")
                if vis is not None and used and not used <= vis:
                    fails.append(
                        f"group {gidx} binding {b} ('{g['name']}') is used in "
                        f"stage(s) {sorted(used)} but the Rust layout grants "
                        f"visibility {sorted(vis)} [{ev}]")
                elif vis is not None and used and used < vis:
                    notes.append(
                        f"group {gidx} binding {b}: layout visibility "
                        f"{sorted(vis)} is a superset of shader use "
                        f"{sorted(used)} (legal)")
        for b in sorted(set(entries) - set(wgsl_bindings)):
            notes.append(
                f"group {gidx} binding {b} exists in Rust layout but is "
                f"unused by the shader (legal)")
    declared = {stage: name for stage, name, _ in shader.entries}
    if site["vent"] is not None:
        if declared.get("vertex") != site["vent"] and \
                site["vent"] not in [n for s, n, _ in shader.entries if s == "vertex"]:
            fails.append(
                f"pipeline vertex entry '{site['vent']}' is not a @vertex "
                f"entry point of the shader (declared: "
                f"{[n for s, n, _ in shader.entries if s == 'vertex']})")
    if site["fent"] is not None:
        if site["fent"] not in [n for s, n, _ in shader.entries if s == "fragment"]:
            fails.append(
                f"pipeline fragment entry '{site['fent']}' is not a "
                f"@fragment entry point of the shader (declared: "
                f"{[n for s, n, _ in shader.entries if s == 'fragment']})")
    return ("FAIL" if fails else "PASS"), fails, notes


# ----------------------------------------------------------------------
# Uniform struct pairs
#
# Each site entry is (rust file, dialect, anchor[, end anchor]).
# Dialects:
#   rust_struct   parse `struct Name { ... }` in the file; compare
#                 field names + mapped types with the WGSL struct.
#   extends       reconstruct bytes from a hand assembly: mat
#                 extends x64, `for v in [...]` vec4 groups x16,
#                 [0u8; N] zero fills, Vec<[f32; 4]> accumulators.
#   extends_all   like extends, but the anchor must match several
#                 assemblies and every one must reconstruct.
#   pack_array    count top-level elements of the f32 array literal
#                 following the anchor (x4 bytes).
#   fn_mats       inside the named fn: mat extends x64 plus a
#                 [f32; 4] parameter x16 when present.
# A site whose anchor matches zero or several places is UNRESOLVED.
# ----------------------------------------------------------------------

PAIRS = [
    # shader, wgsl struct, [(file, dialect, anchor, end_anchor)]
    ("src/shader3d.wgsl", "Uniforms",
     [("src/main.rs", "rust_struct", "Uniforms", None)]),
    ("src/shader_tex.wgsl", "Uniforms",
     [("src/main.rs", "rust_struct", "Uniforms", None),
      ("src/dof.rs", "rust_struct", "Uniforms", None),
      ("src/scenefile.rs", "fn_mats", "uniform_bytes_tex", None)]),
    ("src/shader_scene3d.wgsl", "Uniforms",
     [("src/scenefile.rs", "fn_mats", "uniform_bytes_colour", None)]),
    ("src/shader_depth.wgsl", "Uniforms",
     [("src/dof.rs", "rust_struct", "Uniforms", None),
      ("src/ab1.rs", "rust_struct", "Uniforms", None),
      ("src/film.rs", "fn_mats", "depth_bytes", None)]),
    ("src/shader_depth_pos.wgsl", "Uniforms",
     [("src/scenefile.rs", "fn_mats", "uniform_bytes_tex", None),
      ("src/ab1.rs", "rust_struct", "Uniforms", None),
      ("src/film.rs", "fn_mats", "depth_bytes", None)]),
    ("src/shader_shadow_card.wgsl", "Uniforms",
     [("src/ab1.rs", "rust_struct", "Uniforms", None),
      ("src/film.rs", "fn_mats", "depth_bytes", None)]),
    ("src/shader_pbr.wgsl", "FrameU",
     [("src/ab1.rs", "extends", "let mut frame_bytes",
       "size: frame_bytes.len()"),
      ("src/film.rs", "extends", "let mut frame_bytes",
       "debug_assert_eq!(frame_bytes.len()")]),
    ("src/shader_pbr.wgsl", "DrawU",
     [("src/ab1.rs", "extends_all", "let mut draw_bytes", "make_ub(&draw_bytes"),
      ("src/film.rs", "extends", "let draw_bytes = |", "pbr draw uniforms")]),
    ("src/shader_basic.wgsl", "DrawU",
     [("src/ab1.rs", "extends", "let mut bytes: Vec<u8> = Vec::with_capacity(192)",
       "make_ub(&bytes"),
      ("src/film.rs", "extends", "Vec::with_capacity(192)", "make_ub(&bytes")]),
    ("src/shader_bokeh.wgsl", "BokehUniforms",
     [("src/dof.rs", "pack_array", "let bokeh_uniform_data", None),
      ("src/ab1.rs", "pack_array", "let bokeh_uniform_data", None),
      ("src/scenefile.rs", "pack_array", "let bokeh_uniform_data", None),
      ("src/film.rs", "pack_array", "pack_f32(&[spec.focus", None)]),
    ("src/shader_bloom_bright.wgsl", "U",
     [("src/composer.rs", "pack_array", '"bloom bright uniforms"', None)]),
    ("src/shader_bloom_composite.wgsl", "U",
     [("src/composer.rs", "pack_array", '"bloom composite uniforms"', None)]),
    ("src/shader_grade.wgsl", "U",
     [("src/composer.rs", "pack_array", "let data: [f32; 8]", None)]),
    ("src/shader_output_exp.wgsl", "U",
     [("src/composer.rs", "pack_array", '"output uniforms"', None),
      ("src/scenefile.rs", "pack_array", '"output uniforms"', None)]),
    ("src/shader_fog.wgsl", "U",
     [("src/scenefile.rs", "pack_array", '"fog uniforms"', None)]),
    ("embedded:blur_wgsl", "U",
     [("src/composer.rs", "pack_array", '"bloom blur uniforms"', None)]),
]

RUST_TYPE_MAP = {
    "Mat4": "mat4x4f", "math::Mat4": "mat4x4f",
    "[f32; 4]": "vec4f", "[f32; 3]": "vec3f", "[f32; 2]": "vec2f",
    "f32": "f32",
}


def _find_unique(text, anchor):
    idxs = [m.start() for m in re.finditer(re.escape(anchor), text)]
    return idxs[0] if len(idxs) == 1 else None


def _match_arm_spans(window):
    """Arm body spans of each match block in the window:
    [(match_start, match_end, [arm_span, ...])]. Light-weight:
    arms are the balanced {...} blocks following top-level '=>'.
    """
    out = []
    for m in re.finditer(r"\bmatch\b[^{]*\{", window):
        bopen = window.find("{", m.start())
        bclose = brace_match(window, bopen)
        if bclose < 0:
            continue
        arms = []
        i = bopen + 1
        depth = 1
        while i < bclose:
            c = window[i]
            if c == "{":
                depth += 1
            elif c == "}":
                depth -= 1
            elif depth == 1 and window.startswith("=>", i):
                j = i + 2
                while j < bclose and window[j] not in "{,":
                    j += 1
                if j < bclose and window[j] == "{":
                    aclose = brace_match(window, j)
                    if aclose >= 0:
                        arms.append((j, aclose))
                        i = aclose
            i += 1
        out.append((m.start(), bclose, arms))
    return out


def _extends_total(rf, pos, end_anchor):
    """Reconstruct the byte total of a hand assembly starting at
    pos. Returns (total, detail) or (None, reason). Counts only
    shapes it understands; anything else is UNRESOLVED, never
    silently skipped:
      * mat_bytes(/mb( extends                       -> 64 B each
      * for x in [ [..], [..], ... ] vec4 groups     -> 16 B each
      * [0u8; N] zero fills                          -> N B
      * Vec<[f32; 4]> accumulators: vec![...] init, .push(...)
        (match arms counted per-arm: all arms must agree),
        .extend_from_slice(&v) where v is the result of a local
        helper returning [[f32; 4]; K] (loop multipliers from
        literal iteration arrays are applied).
    """
    end = len(rf.tview)
    if end_anchor:
        epos = rf.tview.find(end_anchor, pos)
        if epos < 0:
            return None, f"end anchor '{end_anchor}' not found"
        end = epos
    w = rf.sview[pos:end]
    total = 0
    detail = []
    n_mats = len(re.findall(r"\bmat_bytes\(|\bmb\(", w))
    if n_mats:
        total += 64 * n_mats
        detail.append(f"{n_mats} mat extends = {64 * n_mats} B")
    # vec4-returning helper closures whose calls may appear as
    # elements of the group arrays below: let v4 = |c: [f32; 3]|
    # [c[0], c[1], c[2], 0.0];  (body is one 4-element array literal).
    # Scanned over the whole file: the helper is commonly defined a
    # few lines before the assembly it serves.
    vec4_helpers = set()
    for m in re.finditer(r"let\s+(\w+)\s*=\s*\|", rf.sview):
        bar2 = rf.sview.find("|", m.end())
        if bar2 < 0:
            continue
        bopen = rf.sview.find("[", bar2)
        if bopen < 0:
            continue
        bclose = brace_match(rf.sview, bopen)
        if bclose >= 0 and len(split_top_level(rf.sview[bopen + 1:bclose])) == 4:
            vec4_helpers.add(m.group(1))
    # literal iteration arrays (for multiplier + vec4-group rules)
    loops = []  # (body_start, body_end, n_elems, elems_are_vec4)
    for m in re.finditer(r"for\s+\w+\s+in\s*&?\s*\[", w):
        bopen = w.find("[", m.start())
        bclose = brace_match(w, bopen)
        if bclose < 0:
            return None, "iteration array does not close"
        elems = split_top_level(w[bopen + 1:bclose])
        brace = w.find("{", bclose)
        bend = brace_match(w, brace) if brace >= 0 else bclose
        are_vec4 = bool(elems) and all(
            e.startswith("[") or
            any(re.match(h + r"\s*\(", e) for h in vec4_helpers)
            for e in elems)
        loops.append((brace, bend, len(elems), are_vec4))
        if are_vec4:
            total += 16 * len(elems)
            detail.append(f"{len(elems)} vec4 groups = {16 * len(elems)} B")
    for m in re.finditer(r"\[\s*0u8;\s*(\d+)\s*\]", w):
        total += int(m.group(1))
        detail.append(f"zero fill {m.group(1)} B")
    # vec4-returning helper closures: let pt = |..| -> [[f32; 4]; K]
    helpers = {}
    for m in re.finditer(
        r"let\s+(\w+)\s*=\s*\|[^|]*\|\s*->\s*\[\[\s*f32;\s*4\s*\];\s*(\d+)\s*\]", w):
        helpers[m.group(1)] = int(m.group(2))
    matches = _match_arm_spans(w)

    def arm_key(p):
        for mi, (_ms, _me, arms) in enumerate(matches):
            for ai, (a, b) in enumerate(arms):
                if a <= p <= b:
                    return (mi, ai)
        return None

    def loop_mult(p):
        mult = 1
        for (bs, be, n, _v) in loops:
            if bs <= p <= be:
                mult = n
        return mult

    for m in re.finditer(
        r"let\s+(?:mut\s+)?(\w+)\s*:\s*Vec<\s*\[\s*f32;\s*4\s*\]\s*>\s*=\s*vec!\s*\[", w):
        name = m.group(1)
        vm = re.search(r"vec!\s*\[", w[m.start():])
        bopen = m.start() + vm.end() - 1
        bclose = brace_match(w, bopen)
        if bclose < 0:
            return None, "vec! accumulator literal does not close"
        elems = split_top_level(w[bopen + 1:bclose])
        total += 16 * len(elems)
        detail.append(f"vec! init {len(elems)} vec4 = {16 * len(elems)} B")
        # contributions per execution path: straight-line items and
        # per-match-arm items (all arms of a match must agree)
        straight = 0
        per_match = {}
        events = []  # (pos, n_vec4)
        for pm in re.finditer(re.escape(name) + r"\.push\s*\(", w):
            events.append((pm.start(), 1))
        for em in re.finditer(
            re.escape(name) + r"\.extend_from_slice\s*\(\s*&\s*(\w+)\s*\)", w):
            src = em.group(1)
            dm = re.search(r"let\s+" + re.escape(src) + r"\s*=\s*(\w+)\s*\(", w)
            if not dm or dm.group(1) not in helpers:
                return None, (f"extend_from_slice source '{src}' is not a "
                              f"known [[f32; 4]; K] helper result")
            events.append((em.start(), helpers[dm.group(1)]))
        for p, nvec in events:
            nvec *= loop_mult(p)
            key = arm_key(p)
            if key is None:
                straight += nvec
            else:
                per_match[key] = per_match.get(key, 0) + nvec
        total += 16 * straight
        if straight:
            detail.append(f"{straight} pushed/extended vec4 = {16 * straight} B")
        by_match = {}
        for (mi, ai), nvec in per_match.items():
            by_match.setdefault(mi, {})[ai] = nvec
        for mi, arms in by_match.items():
            counts = set(arms.values())
            if len(counts) != 1:
                return None, (f"match arms contribute different vec4 counts "
                              f"{sorted(arms.values())}: total is path-dependent")
            nvec = counts.pop()
            total += 16 * nvec
            detail.append(f"match arms agree on {nvec} vec4 = {16 * nvec} B")
    return total, "; ".join(detail)


def check_pair_site(rf_map, shader, struct_name, site):
    """Returns (status, detail) status in PASS/FAIL/UNRESOLVED."""
    rel, dialect, anchor, end_anchor = site
    rf = rf_map.get(rel)
    if rf is None:
        return "UNRESOLVED", f"{rel} not loaded"
    fields = shader.structs.get(struct_name)
    if fields is None:
        return "FAIL", f"WGSL struct '{struct_name}' not found in {shader.id}"
    lay = struct_layout(fields, shader.structs, uniform=True)
    if lay is None:
        return "UNRESOLVED", f"WGSL struct '{struct_name}' has an unmodelled type"
    wgsl_size = lay[0]
    where = f"{rel} [{dialect} '{anchor}']"

    if dialect == "rust_struct":
        pos = _find_unique(rf.sview, f"struct {anchor}")
        if pos is None:
            return "UNRESOLVED", f"{where}: struct anchor not unique/found"
        brace = rf.sview.find("{", pos)
        close = brace_match(rf.sview, brace)
        body = rf.sview[brace:close + 1]
        rfields = [(m.group(1), m.group(2).strip())
                   for m in re.finditer(r"(\w+)\s*:\s*([^,;]+?)\s*[,;]", body)]
        mapped = [(n, RUST_TYPE_MAP.get(t, f"<{t}>")) for n, t in rfields]
        if mapped != fields:
            return "FAIL", (f"{where}: Rust fields {mapped} != WGSL fields "
                            f"{fields} (line {line_of(rf.sview, pos)})")
        return "PASS", (f"{where}: field sequence identical "
                        f"({len(fields)} fields, {wgsl_size} B; Rust struct "
                        f"line {line_of(rf.sview, pos)})")

    if dialect == "fn_mats":
        region = rf.region_by_name(anchor)
        if region is None:
            return "UNRESOLVED", f"{where}: fn not found"
        body = rf.sview[region[1]:region[2] + 1]
        n_mats = len(re.findall(r"\bmat_bytes\(|\bmb\(|\bmat_bytes_colour\(", body))
        total = 64 * n_mats
        brace = body.find("{")
        sig = rf.tview[region[1]:region[1] + (brace if brace >= 0 else 0)]
        if "[f32; 4]" in sig:
            total += 16
        if total != wgsl_size:
            return "FAIL", (f"{where}: fn assembles {total} B "
                            f"({n_mats} matrices) vs WGSL {wgsl_size} B")
        return "PASS", (f"{where}: fn assembles {total} B == WGSL "
                        f"{wgsl_size} B (line {line_of(rf.sview, region[1])})")

    if dialect in ("extends", "extends_all"):
        if dialect == "extends":
            pos = _find_unique(rf.tview, anchor)
            if pos is None:
                return "UNRESOLVED", f"{where}: anchor not unique/found"
            positions = [pos]
        else:
            positions = [m.start() for m in re.finditer(re.escape(anchor), rf.tview)]
            if not positions:
                return "UNRESOLVED", f"{where}: anchor not found"
        results = []
        for pos in positions:
            total, detail = _extends_total(rf, pos, end_anchor)
            if total is None:
                return "UNRESOLVED", (f"{where}: assembly at line "
                                      f"{line_of(rf.tview, pos)}: {detail}")
            results.append((pos, total, detail))
        bad = [(p, t) for p, t, _d in results if t != wgsl_size]
        if bad:
            p, t = bad[0]
            return "FAIL", (f"{where}: assembly at line "
                            f"{line_of(rf.tview, p)} reconstructs {t} B vs "
                            f"WGSL {wgsl_size} B")
        lines = ", ".join(str(line_of(rf.tview, p)) for p, _t, _d in results)
        noun = "assemblies" if len(results) > 1 else "assembly"
        verb = "reconstruct" if len(results) > 1 else "reconstructs"
        return "PASS", (f"{where}: {len(results)} {noun} {verb} "
                        f"{wgsl_size} B == WGSL {wgsl_size} B "
                        f"({results[0][2]}; anchor line(s) {lines}); "
                        f"slot order positional")

    if dialect == "pack_array":
        pos = _find_unique(rf.tview, anchor)
        if pos is None:
            return "UNRESOLVED", f"{where}: anchor not unique/found"
        seg = rf.tview[pos:pos + 2000]
        m = re.search(r"\[", seg)
        if not m:
            return "UNRESOLVED", f"{where}: no array literal after anchor"
        bopen = pos + m.start()
        # skip a leading [f32; N] type ascription: take the LAST '['
        # before the first '=' ... in practice the literal follows
        # ' = [' or '(&['; find the assignment bracket:
        m2 = re.search(r"=\s*\[|\(\s*&?\s*\[", seg)
        if m2:
            bopen = pos + m2.end() - 1
        bclose = brace_match(rf.sview, bopen)
        if bclose < 0:
            return "UNRESOLVED", f"{where}: array literal does not close"
        elems = split_top_level(rf.sview[bopen + 1:bclose])
        total = 4 * len(elems)
        if total != wgsl_size:
            return "FAIL", (f"{where}: {len(elems)} f32 slots = {total} B vs "
                            f"WGSL {wgsl_size} B (anchor line "
                            f"{line_of(rf.tview, pos)})")
        # stronger form when the WGSL struct is all-scalar: the
        # field COUNT must equal the slot count (a dropped pad
        # field would otherwise hide inside the padded struct size)
        if all(t == "f32" for _n, t in fields) and len(elems) != len(fields):
            return "FAIL", (f"{where}: {len(elems)} f32 slots vs "
                            f"{len(fields)} scalar WGSL fields "
                            f"(anchor line {line_of(rf.tview, pos)})")
        return "PASS", (f"{where}: {len(elems)} f32 slots = {total} B == "
                        f"WGSL {wgsl_size} B (anchor line "
                        f"{line_of(rf.tview, pos)}); slot order positional")

    return "UNRESOLVED", f"{where}: unknown dialect '{dialect}'"


# ----------------------------------------------------------------------
# Discovery + embedded shaders + driver
# ----------------------------------------------------------------------


def discover_shaders(root):
    shaders = {}
    for p in sorted((root / "src").rglob("*.wgsl")):
        sid = str(p.relative_to(root))
        shaders[sid] = WgslShader(sid, p.read_text(), f"file {sid}")
    comp = root / "src" / "composer.rs"
    if comp.exists():
        _sv, tv = make_views(comp.read_text(), rust=True)
        m = re.search(r"const\s+COPY_WGSL\s*:\s*&str\s*=\s*r#\"(.*?)\"#\s*;", tv, re.S)
        if m:
            shaders["embedded:COPY_WGSL"] = WgslShader(
                "embedded:COPY_WGSL", m.group(1),
                "const COPY_WGSL in src/composer.rs")
        # blur_wgsl template: un-double the format! braces, then
        # instantiate with real parameters from BLOOM_KERNEL_RADII.
        radii = [3]
        rm = re.search(r"BLOOM_KERNEL_RADII[^=]*=\s*\[([^\]]*)\]", tv)
        if rm:
            radii = [int(x) for x in re.findall(r"\d+", rm.group(1))]
        fm = re.search(r"fn\s+blur_wgsl", tv)
        if fm:
            sm = re.search(r"r#\"(.*?)\"#", tv[fm.start():], re.S)
            if sm:
                tpl = sm.group(1).replace("{{", "\x00").replace("}}", "\x01")
                for radius in sorted(set(radii)):
                    coeffs = ", ".join(["0.0"] * radius)
                    inst = (tpl.replace("{radius}", str(radius))
                               .replace("{dir}", "vec2f(1.0, 0.0)")
                               .replace("{coeffs}", coeffs)
                               .replace("\x00", "{").replace("\x01", "}"))
                    sid = f"embedded:blur_wgsl(r={radius})"
                    shaders[sid] = WgslShader(
                        sid, inst,
                        f"blur_wgsl() template in src/composer.rs, "
                        f"instantiated radius={radius} (of {radii}), "
                        f"horizontal; coefficient VALUES are runtime "
                        f"f64 formatting and are substituted with 0.0 — "
                        f"only structure is checked")
    return shaders


def main(argv=None):
    ap = argparse.ArgumentParser(description="Static WGSL/Rust binding checker")
    ap.add_argument("--repo", default=".")
    ap.add_argument("--json", action="store_true")
    ap.add_argument("-v", "--verbose", action="store_true")
    args = ap.parse_args(argv)
    root = Path(args.repo).resolve()
    if not (root / "src").is_dir():
        print("check_shaders: no src/ under repo root", file=sys.stderr)
        return 2

    shaders = discover_shaders(root)
    rf_map = {}
    for p in sorted((root / "src").rglob("*.rs")):
        rf = RustFile(p, root)
        rf_map[rf.rel] = rf

    # site resolution across all rust files
    sites_by_shader = {}
    unresolved_sites = []
    for rf in rf_map.values():
        sites, unres = resolve_sites(rf)
        for s in sites:
            sites_by_shader.setdefault(s["shader"], []).append((rf, s))
        unresolved_sites.extend(unres)
    # the single blur code site instantiates every radius x both
    # directions at runtime; fan its site out to each instance
    if "embedded:blur_wgsl" in sites_by_shader:
        blur_sites = sites_by_shader.pop("embedded:blur_wgsl")
        for sid in [k for k in shaders if k.startswith("embedded:blur_wgsl")]:
            sites_by_shader.setdefault(sid, []).extend(blur_sites)

    # coverage: every create_shader_module must be accounted for
    for rf in rf_map.values():
        for m in re.finditer(r"create_shader_module\s*\(", rf.sview):
            owner = rf.owner(m.start())
            if owner and owner[0] == "module":
                continue  # the module() helper mechanism itself
            line = line_of(rf.sview, m.start())
            if not any(abs(ln - line) <= 2 for ln in rf.module_def_lines):
                unresolved_sites.append(
                    (f"{rf.rel}:{line}",
                     "create_shader_module not mapped to any shader source"))

    # pair checks
    pair_results = {}  # shader id -> [(status, detail)]
    shader_by_id = shaders
    for shader_id, struct_name, sites in PAIRS:
        # blur pairs apply to every instantiated radius
        targets = [shader_id]
        if shader_id == "embedded:blur_wgsl":
            targets = [s for s in shaders if s.startswith("embedded:blur_wgsl")]
        for tid in targets:
            sh = shader_by_id.get(tid)
            if sh is None:
                pair_results.setdefault(tid, []).append(
                    ("UNRESOLVED", f"pair target shader '{tid}' not discovered"))
                continue
            for site in sites:
                status, detail = check_pair_site(rf_map, sh, struct_name, site)
                pair_results.setdefault(tid, []).append((status, detail))

    # verdicts
    report = {"shaders": {}, "unresolved_sites": unresolved_sites}
    any_fail = False
    any_unresolved = bool(unresolved_sites)
    lines = []
    for sid in sorted(shaders):
        sh = shaders[sid]
        verdicts = []
        notes = []
        fails = [msg for lvl, msg in sh.problems if lvl == "FAIL"]
        notes.extend(msg for lvl, msg in sh.problems if lvl == "INFO")
        if fails:
            verdicts.append("FAIL")
        site_rows = []
        for rf, s in sites_by_shader.get(sid, []):
            v, f, n = compare_site(sh, s)
            site_rows.append((v, s["where"], f, n))
            if v == "FAIL":
                verdicts.append("FAIL")
            notes.extend(n)
        pair_rows = pair_results.get(sid, [])
        for status, _d in pair_rows:
            if status == "FAIL":
                verdicts.append("FAIL")
            elif status == "UNRESOLVED":
                verdicts.append("UNRESOLVED")
        if not sites_by_shader.get(sid):
            verdicts.append("UNRESOLVED")
            notes.append("no Rust usage site resolved for this shader")
        verdict = ("FAIL" if "FAIL" in verdicts else
                   "UNRESOLVED" if "UNRESOLVED" in verdicts else "PASS")
        any_fail = any_fail or verdict == "FAIL"
        any_unresolved = any_unresolved or verdict == "UNRESOLVED"
        report["shaders"][sid] = {
            "verdict": verdict, "origin": sh.origin,
            "internal_fails": fails,
            "sites": [{"verdict": v, "where": w, "fails": f, "notes": n}
                      for v, w, f, n in site_rows],
            "pairs": [{"status": st, "detail": d} for st, d in pair_rows],
            "notes": notes,
        }
        lines.append(f"{verdict:10s} {sid}  ({sh.origin})")
        for msg in fails:
            lines.append(f"    FAIL  {msg}")
        for v, w, f, n in site_rows:
            lines.append(f"    site {v}: {w}")
            for msg in f:
                lines.append(f"        FAIL  {msg}")
            if args.verbose:
                for msg in n:
                    lines.append(f"        info  {msg}")
        for st, d in pair_rows:
            lines.append(f"    pair {st}: {d}")
        if args.verbose:
            for msg in notes:
                lines.append(f"    info  {msg}")
    for where, reason in unresolved_sites:
        lines.append(f"UNRESOLVED site at {where}: {reason}")
    n_pass = sum(1 for s in report["shaders"].values() if s["verdict"] == "PASS")
    lines.append("")
    lines.append(f"check_shaders: {n_pass}/{len(shaders)} shaders PASS, "
                 f"{sum(1 for s in report['shaders'].values() if s['verdict'] == 'FAIL')} FAIL, "
                 f"{sum(1 for s in report['shaders'].values() if s['verdict'] == 'UNRESOLVED')} UNRESOLVED, "
                 f"{len(unresolved_sites)} unresolved site(s)")
    if args.json:
        print(json.dumps(report, indent=2))
    else:
        print("\n".join(lines))
    return 1 if (any_fail or any_unresolved) else 0


if __name__ == "__main__":
    sys.exit(main())
