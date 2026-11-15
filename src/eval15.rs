//! eval15 — scene-format §15 step-2 consumption (Chrome-replacement
//! wave, Batch 1 seat 1): prefab expansion (§15.2–15.5) and clip
//! evaluation (§15.6–15.8) over `crate::scene::Value` documents.
//!
//! This module is the loader-side counterpart of the reference
//! validators `tools/format/check_prefabs.py`,
//! `tools/format/check_clips.py`, `tools/format/stage_clips.py`
//! and `tools/format/check_format15.py`. Its validation semantics
//! mirror theirs check-for-check (same failure classes, same
//! closed vocabularies); where a Python checker and this module
//! could disagree, the Python checker is the record and this file
//! says so at the site.
//!
//! Shape of the work (spec §15.5 / §15.10 step 2):
//!
//! - `process_scene` takes a parsed v2 document and returns a
//!   fully processed ordinary v2 document: prefabs expanded,
//!   clips folded into the targets' static fields at the file's
//!   `time`, both top-level blocks consumed and dropped. The
//!   existing loader (`scenefile::load`) then proceeds exactly as
//!   if the processed document had been authored — no renderer,
//!   telemetry or film-driver code ever sees a `prefab`, `pid`,
//!   `$param`, `clip` or `clipTime` key (asserted on exit).
//! - The typed sampling API (`ClipSet`, `Sampled`) exists for
//!   consumers that evaluate per frame instead of folding once
//!   (the film driver; see docs/integration/CR-B1-01-EVAL15.md
//!   for the s12 retirement that uses it).
//!
//! Float law: clip sampling computes in **f64 and casts to f32 at
//! the boundary**, per §15.8 ("computed in f64, cast to f32 — the
//! film's motion formulas are f64 throughout"). This is a
//! deliberate divergence from `crate::anim`'s f32-native tracks:
//! the two can differ by ~1 ulp, which is why §15.10 A/B-gates
//! evaluator adoption per site. The §15.9 ease remap is
//! internalised here: format keys carry ease on the ARRIVING key
//! (§15.6), and this sampler reads it from the arriving key
//! directly, so no remap is needed at this boundary. If the lead
//! later re-bases sampling on `anim::Track` (easing-to-next), the
//! §15.9 remap (format `key[i+1].ease` -> anim `key[i].ease`)
//! must be applied in `ClipSet` construction.
//!
//! Colour law (§15.9): colour keys are authored `#RRGGBB`,
//! converted to linear BEFORE the track carries them, sampled
//! componentwise in linear, and delivered as `Sampled::ColorLin`
//! — a colour track never holds an sRGB value. Step channels
//! (`mood`, `legs`) ride the `Sampled::Step` side channel and
//! never enter a numeric lane.
//!
//! Clip events (CR-B1-05 R5/R6 — an EXTENSION beyond §15 as
//! written; §15.6 defines clips as tracks only): a clip may carry
//! `events: [{t, cue, gain?, dur?}]` with clip-local `t`,
//! mirroring the engine's `ClipDef.events` (`core/clips.ts`).
//! Events are NOT staging: they are never folded into the
//! document by `evaluate_clips`/`process_scene`. They leave the
//! evaluator only through the dedicated emission surface
//! (`ClipSet::events_in_window`, `ClipSet::collect_events`,
//! `scene_events`), in the R5 shape at absolute film times, and
//! feed the `clip-events.json` sidecar consumed by
//! `audio_recipes.build_cues` — never `cues.json` (event names
//! are not mix.py FX types; SKILL.md §5.7). The conflicts this
//! extension has with §15 as written are listed in
//! docs/integration/CR-B1-01-EVAL15.md §5 — read them before
//! relying on events in production data.
//!
//! Dependencies: `crate::scene::Value` and std only. No wgpu, no
//! I/O — documents in, documents/values out, so the module is
//! unit-testable standalone and callable from any loader path.

use crate::scene::Value;
use std::collections::{HashMap, HashSet};

// =====================================================================
// Small Value helpers (scene::Value exposes get/as_arr/as_str/as_f64
// only; mutation and construction live here).
// =====================================================================

fn is_obj(v: &Value) -> bool {
    matches!(v, Value::Obj(_))
}

fn obj_get<'a>(v: &'a Value, key: &str) -> Option<&'a Value> {
    v.get(key)
}

fn obj_set(v: &mut Value, key: &str, val: Value) {
    if let Value::Obj(pairs) = v {
        for p in pairs.iter_mut() {
            if p.0 == key {
                p.1 = val;
                return;
            }
        }
        pairs.push((key.to_string(), val));
    }
}

fn obj_remove(v: &mut Value, key: &str) {
    if let Value::Obj(pairs) = v {
        pairs.retain(|p| p.0 != key);
    }
}

fn obj_has(v: &Value, key: &str) -> bool {
    v.get(key).is_some()
}

/// A slot is an object with exactly one key, `$param`
/// (check_prefabs.py `is_slot`).
fn is_slot(v: &Value) -> bool {
    match v {
        Value::Obj(pairs) => pairs.len() == 1 && pairs[0].0 == "$param",
        _ => false,
    }
}

fn contains_slot(v: &Value) -> bool {
    if is_slot(v) {
        return true;
    }
    match v {
        Value::Obj(pairs) => pairs.iter().any(|(_, x)| contains_slot(x)),
        Value::Arr(items) => items.iter().any(contains_slot),
        _ => false,
    }
}

/// An instance is any object carrying a `prefab` key
/// (check_prefabs.py `is_instance`).
fn is_instance(v: &Value) -> bool {
    is_obj(v) && obj_has(v, "prefab")
}

fn is_node_like(v: &Value) -> bool {
    is_obj(v) && (obj_has(v, "type") || obj_has(v, "prefab"))
}

fn is_number(v: &Value) -> bool {
    matches!(v, Value::Num(_))
}

fn num_of(v: &Value) -> Option<f64> {
    v.as_f64()
}

/// Duplicate keys inside one JSON object. The crate parser keeps
/// every pair (first wins on `get`); the Python checkers report
/// duplicates as a defect because parsers disagree on the winner.
/// Only called on the §15 path — base documents keep the loader's
/// existing behaviour.
fn find_duplicate_keys(v: &Value, path: &str, out: &mut Vec<String>) {
    match v {
        Value::Obj(pairs) => {
            let mut seen: HashSet<&str> = HashSet::new();
            for (k, x) in pairs {
                if !seen.insert(k.as_str()) {
                    out.push(format!("{path}: duplicate JSON key '{k}'"));
                }
                find_duplicate_keys(x, &format!("{path}.{k}"), out);
            }
        }
        Value::Arr(items) => {
            for (i, x) in items.iter().enumerate() {
                find_duplicate_keys(x, &format!("{path}[{i}]"), out);
            }
        }
        _ => {}
    }
}

const HEX_DIGITS: &[u8] = b"0123456789abcdefABCDEF";

fn is_color_str(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 7 && b[0] == b'#' && b[1..].iter().all(|c| HEX_DIGITS.contains(c))
}

// =====================================================================
// Prefabs (§15.2–15.5)
// =====================================================================

const PARAM_TYPES: [&str; 9] = [
    "number", "string", "color", "boolean", "vec3", "array", "object", "node", "nodes",
];
const DECL_KEYS: [&str; 4] = ["type", "required", "default", "description"];
const DEF_KEYS: [&str; 3] = ["params", "node", "description"];
const INSTANCE_KEYS: [&str; 5] = ["prefab", "params", "overrides", "transform", "name"];
/// §15.4 forbidden override keys, at any depth.
const FORBIDDEN_OVERRIDE_KEYS: [&str; 7] =
    ["type", "prefab", "pid", "id", "kind", "generator", "transform"];
const MAX_EXPANSION_DEPTH: usize = 64;

#[derive(Clone)]
struct ParamDecl {
    ty: String,
    required: bool,
    default: Option<Value>,
}

#[derive(Clone)]
struct PrefabDef {
    /// Declared params in document order.
    params: Vec<(String, ParamDecl)>,
    node: Value,
    source: String,
}

impl PrefabDef {
    fn decl(&self, name: &str) -> Option<&ParamDecl> {
        self.params.iter().find(|(n, _)| n == name).map(|(_, d)| d)
    }
}

type Scope = HashMap<String, PrefabDef>;

/// Does JSON value `v` satisfy declared param type `pty`?
/// (check_prefabs.py `type_ok`, verbatim semantics.)
fn type_ok(v: &Value, pty: &str) -> bool {
    match pty {
        "number" => is_number(v),
        "string" => matches!(v, Value::Str(_)),
        "color" => matches!(v, Value::Str(s) if is_color_str(s)),
        "boolean" => matches!(v, Value::Bool(_)),
        "vec3" => match v.as_arr() {
            Some(a) => a.len() == 3 && a.iter().all(is_number),
            None => false,
        },
        "array" => matches!(v, Value::Arr(_)),
        "object" => is_obj(v) && !is_slot(v) && !obj_has(v, "prefab"),
        "node" => is_node_like(v),
        "nodes" => match v.as_arr() {
            Some(a) => a.iter().all(is_node_like),
            None => false,
        },
        _ => false,
    }
}

/// Collect every slot under `v`: (slot-name-or-None, parent key).
fn collect_slots(v: &Value, parent_key: Option<&str>, out: &mut Vec<(Option<String>, Option<String>)>) {
    if is_slot(v) {
        let name = match v {
            Value::Obj(pairs) => pairs[0].1.as_str().map(|s| s.to_string()),
            _ => None,
        };
        out.push((name, parent_key.map(|s| s.to_string())));
        return;
    }
    match v {
        Value::Obj(pairs) => {
            for (k, x) in pairs {
                collect_slots(x, Some(k.as_str()), out);
            }
        }
        Value::Arr(items) => {
            for x in items {
                collect_slots(x, parent_key, out);
            }
        }
        _ => {}
    }
}

/// Direct-scope pid table (§15.2): descend plain nodes, never a
/// nested instance's interior (the nested instance node itself
/// counts). Returns pid -> occurrence count.
fn pid_counts(template: &Value, out: &mut HashMap<String, usize>) {
    if let Value::Obj(_) = template {
        if let Some(pid) = template.get("pid").and_then(Value::as_str) {
            *out.entry(pid.to_string()).or_insert(0) += 1;
        }
        if is_instance(template) {
            return;
        }
        if let Value::Obj(pairs) = template {
            for (_, x) in pairs {
                pid_counts(x, out);
            }
        }
    } else if let Value::Arr(items) = template {
        for x in items {
            pid_counts(x, out);
        }
    }
}

/// The direct-scope node carrying `pid` (depth-first), or None.
/// Mirrors check_prefabs.py `find_pid_node`.
fn find_pid_node<'a>(template: &'a Value, pid: &str) -> Option<&'a Value> {
    match template {
        Value::Obj(_) => {
            if template.get("pid").and_then(Value::as_str) == Some(pid) {
                return Some(template);
            }
            if is_instance(template) {
                return None;
            }
            if let Value::Obj(pairs) = template {
                for (_, x) in pairs {
                    if let Some(r) = find_pid_node(x, pid) {
                        return Some(r);
                    }
                }
            }
            None
        }
        Value::Arr(items) => {
            for x in items {
                if let Some(r) = find_pid_node(x, pid) {
                    return Some(r);
                }
            }
            None
        }
        _ => None,
    }
}

fn find_pid_node_mut<'a>(template: &'a mut Value, pid: &str) -> Option<&'a mut Value> {
    // Two-phase to keep the borrow checker simple: locate by path.
    fn locate(v: &Value, pid: &str, path: &mut Vec<usize>) -> bool {
        match v {
            Value::Obj(pairs) => {
                if v.get("pid").and_then(Value::as_str) == Some(pid) {
                    return true;
                }
                if is_instance(v) {
                    return false;
                }
                for (i, (_, x)) in pairs.iter().enumerate() {
                    path.push(i);
                    if locate(x, pid, path) {
                        return true;
                    }
                    path.pop();
                }
                false
            }
            Value::Arr(items) => {
                for (i, x) in items.iter().enumerate() {
                    path.push(i);
                    if locate(x, pid, path) {
                        return true;
                    }
                    path.pop();
                }
                false
            }
            _ => false,
        }
    }
    let mut path: Vec<usize> = Vec::new();
    if !locate(template, pid, &mut path) {
        return None;
    }
    let mut cur = template;
    for i in path {
        cur = match cur {
            Value::Obj(pairs) => &mut pairs[i].1,
            Value::Arr(items) => &mut items[i],
            _ => return None,
        };
    }
    Some(cur)
}

/// Every instance target name referenced under `v` (the static
/// reference graph's edges; §15.5 step 3).
fn collect_instance_targets(v: &Value, out: &mut Vec<String>) {
    if is_instance(v) {
        if let Some(t) = v.get("prefab").and_then(Value::as_str) {
            out.push(t.to_string());
        }
    }
    match v {
        Value::Obj(pairs) => {
            for (_, x) in pairs {
                collect_instance_targets(x, out);
            }
        }
        Value::Arr(items) => {
            for x in items {
                collect_instance_targets(x, out);
            }
        }
        _ => {}
    }
}

fn collect_instances(v: &Value, out: &mut Vec<Value>) {
    if is_instance(v) {
        out.push(v.clone());
    }
    match v {
        Value::Obj(pairs) => {
            for (_, x) in pairs {
                collect_instances(x, out);
            }
        }
        Value::Arr(items) => {
            for x in items {
                collect_instances(x, out);
            }
        }
        _ => {}
    }
}

/// §15.5 step 1: assemble the scope from library documents (each
/// a parsed root Value carrying a `prefabs` block) plus the
/// scene's inline block. A name defined twice is a hard error
/// naming both sources — no shadowing.
fn build_scope(root: &Value, libraries: &[Value]) -> Result<Scope, String> {
    let mut scope: Scope = HashMap::new();
    let mut add_block = |block: &Value, source: &str, scope: &mut Scope| -> Result<(), String> {
        let pairs = match block {
            Value::Obj(p) => p,
            _ => return Err(format!("{source}: 'prefabs' must be an object")),
        };
        for (name, defn) in pairs {
            if let Some(prev) = scope.get(name) {
                return Err(format!(
                    "prefab '{name}' defined by both {} and {source} — duplicate definitions are an error, never shadowing (spec §15.2)",
                    prev.source
                ));
            }
            scope.insert(
                name.clone(),
                PrefabDef {
                    params: Vec::new(),
                    node: defn.get("node").cloned().unwrap_or(Value::Null),
                    source: source.to_string(),
                },
            );
            // Stash the raw definition for the validation pass.
            scope.get_mut(name).unwrap().node = defn.clone();
        }
        Ok(())
    };
    for (i, lib) in libraries.iter().enumerate() {
        if let Some(block) = lib.get("prefabs") {
            add_block(block, &format!("library[{i}]"), &mut scope)?;
        }
    }
    if let Some(block) = root.get("prefabs") {
        add_block(block, "scene (inline)", &mut scope)?;
    }
    // Parse each definition's params now that sources are known;
    // `node` currently holds the whole definition — split it.
    let names: Vec<String> = scope.keys().cloned().collect();
    for name in names {
        let defn = scope.get(&name).unwrap().node.clone();
        let entry = scope.get_mut(&name).unwrap();
        entry.node = defn.get("node").cloned().unwrap_or(Value::Null);
        // Raw def kept reachable for validation via a side table
        // is unnecessary: validate_definitions re-reads from the
        // original documents. See validate_definitions.
        let _ = entry;
    }
    Ok(scope)
}

/// §15.5 step 2: validate every definition eagerly, used or not.
/// Reads definitions from the source documents (inline + libs).
fn validate_definitions(root: &Value, libraries: &[Value], scope: &Scope) -> Result<(), String> {
    let mut defs: Vec<(String, Value, String)> = Vec::new();
    for (i, lib) in libraries.iter().enumerate() {
        if let Some(Value::Obj(pairs)) = lib.get("prefabs") {
            for (name, defn) in pairs {
                defs.push((name.clone(), defn.clone(), format!("library[{i}]")));
            }
        }
    }
    if let Some(Value::Obj(pairs)) = root.get("prefabs") {
        for (name, defn) in pairs {
            defs.push((name.clone(), defn.clone(), "scene (inline)".to_string()));
        }
    }
    for (name, defn, source) in &defs {
        let base = format!("prefabs.{name} ({source})");
        if !is_obj(defn) {
            return Err(format!("{base}: definition must be an object"));
        }
        if let Value::Obj(pairs) = defn {
            for (k, _) in pairs {
                if !DEF_KEYS.contains(&k.as_str()) {
                    return Err(format!("{base}: unknown definition key '{k}'"));
                }
            }
        }
        let node = defn
            .get("node")
            .ok_or_else(|| format!("{base}: definition has no 'node' template"))?;
        if !is_node_like(node) {
            return Err(format!("{base}.node: template root must be a node object (with 'type' or 'prefab')"));
        }
        // Param declarations.
        let mut declared: HashMap<String, ParamDecl> = HashMap::new();
        if let Some(params) = defn.get("params") {
            let ppairs = match params {
                Value::Obj(p) => p,
                _ => return Err(format!("{base}.params: must be an object")),
            };
            for (pname, decl) in ppairs {
                let dp = format!("{base}.params.{pname}");
                if !is_obj(decl) {
                    return Err(format!("{dp}: declaration must be an object"));
                }
                if let Value::Obj(dpairs) = decl {
                    for (k, _) in dpairs {
                        if !DECL_KEYS.contains(&k.as_str()) {
                            return Err(format!("{dp}: unknown declaration key '{k}'"));
                        }
                    }
                }
                let ty = decl
                    .get("type")
                    .and_then(Value::as_str)
                    .ok_or_else(|| format!("{dp}: 'type' is required"))?;
                if !PARAM_TYPES.contains(&ty) {
                    return Err(format!("{dp}: type '{ty}' is not one of {}", PARAM_TYPES.join(", ")));
                }
                let required = matches!(decl.get("required"), Some(Value::Bool(true)));
                if let Some(r) = decl.get("required") {
                    if !matches!(r, Value::Bool(_)) {
                        return Err(format!("{dp}: 'required' must be a boolean"));
                    }
                }
                let default = decl.get("default").cloned();
                if required && default.is_some() {
                    return Err(format!("{dp}: a required param may not carry a default (spec §15.2)"));
                }
                if !required && default.is_none() {
                    return Err(format!(
                        "{dp}: param is neither required nor carries a default — every param is one or the other (spec §15.2)"
                    ));
                }
                if let Some(dv) = &default {
                    if contains_slot(dv) {
                        return Err(format!("{dp}: default contains a $param slot — defaults must be literal (spec §15.2)"));
                    }
                    if !type_ok(dv, ty) {
                        return Err(format!("{dp}: default does not satisfy its declared type '{ty}'"));
                    }
                }
                declared.insert(
                    pname.clone(),
                    ParamDecl { ty: ty.to_string(), required, default },
                );
            }
        }
        // Slots inside the template name this prefab's own params.
        let mut slots = Vec::new();
        collect_slots(node, None, &mut slots);
        for (sname, parent_key) in &slots {
            match sname {
                None => return Err(format!("{base}.node: $param value must be a param name string")),
                Some(sn) => {
                    if !declared.contains_key(sn) {
                        return Err(format!(
                            "{base}.node: $param '{sn}' is not declared by prefab '{name}'"
                        ));
                    }
                }
            }
            if parent_key.as_deref() == Some("prefab") {
                return Err(format!(
                    "{base}.node: a slot may not name a prefab — instantiation targets are literal so the reference graph is static (spec §15.2)"
                ));
            }
        }
        // Pids unique per direct scope.
        let mut counts: HashMap<String, usize> = HashMap::new();
        pid_counts(node, &mut counts);
        for (pid, n) in &counts {
            if *n > 1 {
                return Err(format!(
                    "{base}.node: pid '{pid}' declared {n} times in the template's direct scope — pids are unique per prefab (spec §15.2)"
                ));
            }
        }
        // Fill the scope entry's parsed params.
        let _ = scope;
        // (Parsed params are rebuilt in parse_declared below.)
        let _ = declared;
    }
    Ok(())
}

/// Parse a definition's param declarations (shape already
/// validated by `validate_definitions`).
fn parse_declared(defn: &Value) -> Vec<(String, ParamDecl)> {
    let mut out = Vec::new();
    if let Some(Value::Obj(ppairs)) = defn.get("params") {
        for (pname, decl) in ppairs {
            let ty = decl.get("type").and_then(Value::as_str).unwrap_or("").to_string();
            let required = matches!(decl.get("required"), Some(Value::Bool(true)));
            let default = decl.get("default").cloned();
            out.push((pname.clone(), ParamDecl { ty, required, default }));
        }
    }
    out
}

/// §15.5 step 3: the static reference graph must be acyclic; the
/// error prints the cycle path.
fn check_cycles(scope: &Scope, raw_defs: &HashMap<String, Value>) -> Result<(), String> {
    let mut graph: HashMap<String, Vec<String>> = HashMap::new();
    for name in scope.keys() {
        let mut edges = Vec::new();
        if let Some(defn) = raw_defs.get(name) {
            if let Some(node) = defn.get("node") {
                let mut targets = Vec::new();
                collect_instance_targets(node, &mut targets);
                for t in targets {
                    if scope.contains_key(&t) && !edges.contains(&t) {
                        edges.push(t);
                    }
                }
            }
        }
        edges.sort();
        graph.insert(name.clone(), edges);
    }
    #[derive(Clone, Copy, PartialEq)]
    enum Colour {
        White,
        Grey,
        Black,
    }
    fn dfs(
        u: &str,
        graph: &HashMap<String, Vec<String>>,
        colour: &mut HashMap<String, Colour>,
        stack: &mut Vec<String>,
    ) -> Result<(), String> {
        colour.insert(u.to_string(), Colour::Grey);
        stack.push(u.to_string());
        for w in graph.get(u).cloned().unwrap_or_default() {
            match colour.get(&w).copied().unwrap_or(Colour::White) {
                Colour::Grey => {
                    let start = stack.iter().position(|x| x == &w).unwrap();
                    let mut cyc: Vec<String> = stack[start..].to_vec();
                    cyc.push(w.clone());
                    return Err(format!(
                        "prefab cycle: {} — expansion would not terminate (spec §15.5)",
                        cyc.join(" -> ")
                    ));
                }
                Colour::White => dfs(&w, graph, colour, stack)?,
                Colour::Black => {}
            }
        }
        stack.pop();
        colour.insert(u.to_string(), Colour::Black);
        Ok(())
    }
    let mut colour: HashMap<String, Colour> = HashMap::new();
    let mut names: Vec<String> = graph.keys().cloned().collect();
    names.sort();
    for n in names {
        if colour.get(&n).copied().unwrap_or(Colour::White) == Colour::White {
            dfs(&n, &graph, &mut colour, &mut Vec::new())?;
        }
    }
    Ok(())
}

/// Deep-scan an override partial for forbidden keys / nulls at
/// any depth (check_prefabs.py `scan_forbidden`).
fn scan_override_forbidden(v: &Value, path: &str) -> Result<(), String> {
    match v {
        Value::Obj(pairs) => {
            for (k, x) in pairs {
                if FORBIDDEN_OVERRIDE_KEYS.contains(&k.as_str()) {
                    return Err(format!(
                        "{path}.{k}: override touches forbidden key '{k}' (identity keys and transforms may not be overridden — spec §15.4)"
                    ));
                }
                scan_override_forbidden(x, &format!("{path}.{k}"))?;
            }
        }
        Value::Arr(items) => {
            for (i, x) in items.iter().enumerate() {
                scan_override_forbidden(x, &format!("{path}[{i}]"))?;
            }
        }
        Value::Null => {
            return Err(format!(
                "{path}: override contains null — deletion by override is not defined (spec §15.4)"
            ));
        }
        _ => {}
    }
    Ok(())
}

/// Merge-legality: an override may never write through a `$param`
/// slot position (check_prefabs.py `merge_legality`).
fn merge_legality(target: &Value, partial: &Value, path: &str) -> Result<(), String> {
    let (Value::Obj(tpairs), Value::Obj(ppairs)) = (target, partial) else {
        return Ok(());
    };
    let tmap: HashMap<&str, &Value> = tpairs.iter().map(|(k, v)| (k.as_str(), v)).collect();
    for (k, v) in ppairs {
        if FORBIDDEN_OVERRIDE_KEYS.contains(&k.as_str()) || matches!(v, Value::Null) || is_slot(v) {
            continue; // forbidden/null: scan's job; slot value: data
        }
        if let Some(tv) = tmap.get(k.as_str()) {
            if is_slot(tv) {
                return Err(format!(
                    "{path}.{k}: override targets a $param slot — use the params channel, not overrides (spec §15.4)"
                ));
            }
            if is_obj(tv) && is_obj(v) {
                merge_legality(tv, v, &format!("{path}.{k}"))?;
            }
        }
    }
    Ok(())
}

/// The §15.4 deep merge itself (arrays/scalars replace wholesale).
fn deep_merge(dst: &mut Value, src: &Value) {
    let (Value::Obj(dpairs), Value::Obj(spairs)) = (&mut *dst, src) else {
        return;
    };
    // Index dst keys for replacement.
    for (k, v) in spairs {
        let existing = dpairs.iter_mut().find(|(dk, _)| dk == k);
        match existing {
            Some((_, dv))
                if is_obj(dv) && is_obj(v) && !is_slot(dv) && !is_slot(v) =>
            {
                deep_merge(dv, v);
            }
            Some((_, dv)) => {
                *dv = v.clone();
            }
            None => {
                dpairs.push((k.clone(), v.clone()));
            }
        }
    }
}

/// Validate one instance (check_prefabs.py checks 3+4).
/// `enclosing` is the prefab whose template contains the instance
/// (None at scene level); `outer` maps that prefab's param names
/// to declared types, for forwarding checks.
fn validate_instance(
    inst: &Value,
    scope: &Scope,
    enclosing: Option<&str>,
    outer: &HashMap<String, String>,
    path: &str,
) -> Result<(), String> {
    if let Value::Obj(pairs) = inst {
        for (k, _) in pairs {
            if !INSTANCE_KEYS.contains(&k.as_str()) {
                return Err(format!(
                    "{path}: unknown instance key '{k}' (closed grammar — allowed: {})",
                    INSTANCE_KEYS.join(", ")
                ));
            }
        }
    }
    let target = match inst.get("prefab") {
        Some(Value::Str(s)) => s.clone(),
        _ => {
            return Err(format!(
                "{path}: 'prefab' must be a literal prefab name string (slots may not name a prefab)"
            ))
        }
    };
    let def = scope
        .get(&target)
        .ok_or_else(|| format!("{path}: unknown prefab '{target}'"))?;
    if let Some(p) = inst.get("params") {
        if !is_obj(p) {
            return Err(format!("{path}.params: must be an object"));
        }
    }
    if let Some(tr) = inst.get("transform") {
        if !matches!(tr, Value::Arr(_)) {
            return Err(format!("{path}.transform: must be an array of transform ops (base format §5)"));
        }
    }
    if let Some(ov) = inst.get("overrides") {
        if !is_obj(ov) {
            return Err(format!("{path}.overrides: must be an object keyed by pid"));
        }
    }
    if let Some(n) = inst.get("name") {
        if !matches!(n, Value::Str(_)) {
            return Err(format!("{path}.name: must be a string"));
        }
    }
    let supplied: Vec<(String, Value)> = match inst.get("params") {
        Some(Value::Obj(p)) => p.clone(),
        _ => Vec::new(),
    };
    for (pname, decl) in &def.params {
        if decl.required && !supplied.iter().any(|(n, _)| n == pname) {
            return Err(format!("{path}: missing required param '{pname}' for prefab '{target}'"));
        }
    }
    for (pname, v) in &supplied {
        let decl = def
            .decl(pname)
            .ok_or_else(|| format!("{path}.params.{pname}: prefab '{target}' declares no param '{pname}'"))?;
        if is_slot(v) {
            let sname = match v {
                Value::Obj(pairs) => pairs[0].1.as_str().unwrap_or("").to_string(),
                _ => String::new(),
            };
            match enclosing {
                None => {
                    return Err(format!(
                        "{path}.params.{pname}: slot '{sname}' in a scene instance — scenes have no params to forward (spec §15.2)"
                    ));
                }
                Some(enc) => {
                    if let Some(got) = outer.get(&sname) {
                        if got != &decl.ty && !(got == "color" && decl.ty == "string") {
                            return Err(format!(
                                "{path}.params.{pname}: forwarded slot '{sname}' is declared '{got}' by prefab '{enc}', but prefab '{target}' declares '{pname}' as '{}'",
                                decl.ty
                            ));
                        }
                    }
                    // Undeclared slot names are caught by the
                    // enclosing template's own slot validation.
                }
            }
        } else if !type_ok(v, &decl.ty) {
            return Err(format!(
                "{path}.params.{pname}: value does not satisfy declared type '{}' of prefab '{target}'",
                decl.ty
            ));
        }
    }
    // Overrides: pid must exist in the target's direct scope,
    // merge must be legal.
    if let Some(Value::Obj(ovpairs)) = inst.get("overrides") {
        let template = raw_template(scope, &target);
        let mut counts: HashMap<String, usize> = HashMap::new();
        if let Some(t) = &template {
            pid_counts(t, &mut counts);
        }
        for (pid, partial) in ovpairs {
            let op = format!("{path}.overrides.{pid}");
            if !counts.contains_key(pid) {
                return Err(format!("{op}: prefab '{target}' has no pid '{pid}' in its direct scope"));
            }
            if !is_obj(partial) {
                return Err(format!("{op}: override must be an object"));
            }
            scan_override_forbidden(partial, &op)?;
            if let Some(t) = &template {
                if let Some(node) = find_pid_node(t, pid) {
                    let node = node.clone();
                    merge_legality(&node, partial, &op)?;
                }
            }
        }
    }
    Ok(())
}

/// The raw template node for a prefab (stored on the scope entry
/// during `build_scope_with_defs`).
fn raw_template(scope: &Scope, name: &str) -> Option<Value> {
    scope.get(name).map(|d| d.node.clone())
}

/// Substitute every slot in `v` with a deep copy of its effective
/// param value (§15.5 step 5d).
fn substitute(v: &Value, eff: &HashMap<String, Value>) -> Result<Value, String> {
    if is_slot(v) {
        let name = match v {
            Value::Obj(pairs) => pairs[0].1.as_str().unwrap_or("").to_string(),
            _ => String::new(),
        };
        return eff
            .get(&name)
            .cloned()
            .ok_or_else(|| format!("slot '{name}' has no effective value"));
    }
    match v {
        Value::Obj(pairs) => {
            let mut out = Vec::with_capacity(pairs.len());
            for (k, x) in pairs {
                out.push((k.clone(), substitute(x, eff)?));
            }
            Ok(Value::Obj(out))
        }
        Value::Arr(items) => {
            let mut out = Vec::with_capacity(items.len());
            for x in items {
                out.push(substitute(x, eff)?);
            }
            Ok(Value::Arr(out))
        }
        other => Ok(other.clone()),
    }
}

fn strip_pids(v: &mut Value) {
    match v {
        Value::Obj(pairs) => {
            pairs.retain(|(k, _)| k != "pid");
            for (_, x) in pairs.iter_mut() {
                strip_pids(x);
            }
        }
        Value::Arr(items) => {
            for x in items.iter_mut() {
                strip_pids(x);
            }
        }
        _ => {}
    }
}

/// Expand one instance (§15.5 step 5, sub-steps a–g in order).
fn expand_instance(
    inst: &Value,
    scope: &Scope,
    raw_defs: &HashMap<String, Value>,
    depth: usize,
) -> Result<Value, String> {
    if depth > MAX_EXPANSION_DEPTH {
        return Err(format!(
            "expansion depth exceeds {MAX_EXPANSION_DEPTH} — the cycle check should have caught this"
        ));
    }
    let target = inst.get("prefab").and_then(Value::as_str).unwrap_or("");
    let def = scope
        .get(target)
        .ok_or_else(|| format!("unknown prefab '{target}'"))?;
    let template = raw_defs
        .get(target)
        .and_then(|d| d.get("node"))
        .ok_or_else(|| format!("prefab '{target}': definition has no 'node' template"))?;
    // (a) effective params: defaults overridden wholesale.
    let mut eff: HashMap<String, Value> = HashMap::new();
    for (pname, decl) in &def.params {
        if let Some(dv) = &decl.default {
            eff.insert(pname.clone(), dv.clone());
        }
    }
    if let Some(Value::Obj(spairs)) = inst.get("params") {
        for (k, v) in spairs {
            eff.insert(k.clone(), v.clone());
        }
    }
    // (b) deep-copy the template root.
    let mut tree = template.clone();
    // (c) overrides by pid, before substitution.
    if let Some(Value::Obj(ovpairs)) = inst.get("overrides") {
        for (pid, partial) in ovpairs {
            let node = find_pid_node_mut(&mut tree, pid)
                .ok_or_else(|| format!("override pid '{pid}' not found in prefab '{target}'"))?;
            deep_merge(node, partial);
        }
    }
    // (d) substitute slots.
    let mut tree = substitute(&tree, &eff)?;
    // (e) recursively expand instances in the resulting subtree.
    tree = expand_any(&tree, scope, raw_defs, depth + 1)?;
    // (f) strip every pid.
    strip_pids(&mut tree);
    // (g) wrap for the instance transform; set `name` outermost.
    if let Some(tr) = inst.get("transform") {
        tree = Value::Obj(vec![
            ("type".to_string(), Value::Str("group".to_string())),
            ("transform".to_string(), tr.clone()),
            ("children".to_string(), Value::Arr(vec![tree])),
        ]);
    }
    if let Some(n) = inst.get("name") {
        obj_set(&mut tree, "name", n.clone());
    }
    Ok(tree)
}

fn expand_any(
    v: &Value,
    scope: &Scope,
    raw_defs: &HashMap<String, Value>,
    depth: usize,
) -> Result<Value, String> {
    if is_instance(v) {
        return expand_instance(v, scope, raw_defs, depth);
    }
    match v {
        Value::Obj(pairs) => {
            let mut out = Vec::with_capacity(pairs.len());
            for (k, x) in pairs {
                out.push((k.clone(), expand_any(x, scope, raw_defs, depth)?));
            }
            Ok(Value::Obj(out))
        }
        Value::Arr(items) => {
            let mut out = Vec::with_capacity(items.len());
            for x in items {
                out.push(expand_any(x, scope, raw_defs, depth)?);
            }
            Ok(Value::Arr(out))
        }
        other => Ok(other.clone()),
    }
}

fn assert_no_prefab_residue(v: &Value, path: &str) -> Result<(), String> {
    match v {
        Value::Obj(pairs) => {
            for (k, x) in pairs {
                if k == "prefab" || k == "pid" || k == "$param" {
                    return Err(format!("{path}: residual key '{k}' after expansion"));
                }
                assert_no_prefab_residue(x, &format!("{path}.{k}"))?;
            }
        }
        Value::Arr(items) => {
            for (i, x) in items.iter().enumerate() {
                assert_no_prefab_residue(x, &format!("{path}[{i}]"))?;
            }
        }
        _ => {}
    }
    if is_slot(v) {
        return Err(format!("{path}: residual $param slot after expansion"));
    }
    Ok(())
}

/// Expand all prefab instances in the document's `nodes` array
/// (§15.5 steps 1–6). `libraries` are parsed library documents
/// (roots carrying a `prefabs` block). On success the top-level
/// `prefabs` block is consumed and dropped.
pub fn expand_prefabs(root: &mut Value, libraries: &[Value]) -> Result<(), String> {
    // Scope assembly needs the raw definitions; keep a side map.
    let mut raw_defs: HashMap<String, Value> = HashMap::new();
    let mut collect = |block: &Value, raw_defs: &mut HashMap<String, Value>| {
        if let Value::Obj(pairs) = block {
            for (name, defn) in pairs {
                raw_defs.entry(name.clone()).or_insert_with(|| defn.clone());
            }
        }
    };
    for lib in libraries {
        if let Some(b) = lib.get("prefabs") {
            collect(b, &mut raw_defs);
        }
    }
    if let Some(b) = root.get("prefabs") {
        collect(b, &mut raw_defs);
    }
    let mut scope = build_scope(root, libraries)?;
    // Attach parsed params + template nodes to the scope.
    for (name, defn) in &raw_defs {
        if let Some(entry) = scope.get_mut(name) {
            entry.params = parse_declared(defn);
            entry.node = defn.get("node").cloned().unwrap_or(Value::Null);
        }
    }
    let has_constructs = root.get("prefabs").is_some()
        || !scope.is_empty()
        || root
            .get("nodes")
            .map(|n| {
                let mut insts = Vec::new();
                collect_instances(n, &mut insts);
                !insts.is_empty()
            })
            .unwrap_or(false);
    if !has_constructs {
        // Still enforce: slots may not appear in scene nodes.
        if let Some(nodes) = root.get("nodes") {
            let mut slots = Vec::new();
            collect_slots(nodes, None, &mut slots);
            if !slots.is_empty() {
                return Err("nodes: $param slot appears in the scene outside any prefab template — slots are lexically scoped to the prefab that declares them (spec §15.2)".to_string());
            }
        }
        return Ok(());
    }
    // Prefab use rides format version 2 (check_prefabs check 1).
    let version = root.get("version").and_then(Value::as_f64);
    if version != Some(2.0) {
        return Err(format!(
            "prefab extension rides format version 2, file declares version {version:?} (spec §15)"
        ));
    }
    validate_definitions(root, libraries, &scope)?;
    // Re-attach parsed params (validate_definitions re-read the
    // documents; the scope entries were filled above already —
    // keep them in sync if a definition appeared twice, which
    // build_scope has already rejected).
    check_cycles(&scope, &raw_defs)?;
    // §15.5 step 4: validate every instance — in the scene, and
    // inside templates (with the enclosing prefab's param types
    // for forwarding checks). Slots in scene nodes are errors.
    if let Some(nodes) = root.get("nodes") {
        let mut slots = Vec::new();
        collect_slots(nodes, None, &mut slots);
        if !slots.is_empty() {
            return Err("nodes: $param slot appears in the scene outside any prefab template — slots are lexically scoped to the prefab that declares them (spec §15.2)".to_string());
        }
        let mut insts = Vec::new();
        collect_instances(nodes, &mut insts);
        for inst in &insts {
            validate_instance(inst, &scope, None, &HashMap::new(), "nodes")?;
        }
    }
    for (name, defn) in &raw_defs {
        if let Some(node) = defn.get("node") {
            let outer: HashMap<String, String> = scope
                .get(name)
                .map(|d| d.params.iter().map(|(n, dc)| (n.clone(), dc.ty.clone())).collect())
                .unwrap_or_default();
            let mut insts = Vec::new();
            collect_instances(node, &mut insts);
            for inst in &insts {
                validate_instance(inst, &scope, Some(name), &outer, &format!("prefabs.{name}.node"))?;
            }
        }
    }
    // §15.5 step 5: expand the scene's nodes in place, in order.
    if let Some(nodes) = root.get("nodes").cloned() {
        let expanded = expand_any(&nodes, &scope, &raw_defs, 0)?;
        assert_no_prefab_residue(&expanded, "nodes")?;
        obj_set(root, "nodes", expanded);
    }
    // Step 6: the prefabs block is consumed and dropped.
    obj_remove(root, "prefabs");
    Ok(())
}

// =====================================================================
// Clips (§15.6–15.8)
// =====================================================================

/// The 17 mood strings of `Mood::from_str` (src/toonfull.rs),
/// copied as data — clip validation FAILS an unknown mood instead
/// of falling back to neutral (§15.7 strictness note).
pub const MOODS: [&str; 17] = [
    "neutral", "smug", "happy", "laugh", "shock", "worried", "angry", "sad", "sly", "proud",
    "tired", "dizzy", "determined", "embarrassed", "awe", "suspicious", "relieved",
];
pub const LEG_STATES: [&str; 5] = ["stand", "walk", "run", "sit", "jump"];
pub const RESERVED_TARGETS: [&str; 4] = ["camera", "lights", "post", "fog"];

/// Clip-event vocabulary (CR-B1-05 R7, closed): the 11 names of
/// `audio_recipes.EVENT_VOCABULARY` (template/tools/
/// audio_recipes.py:71-74), mirrored from the TS producers
/// (`core/actions.ts`, `core/cliplib.ts`, `core/interact.ts`).
/// `pop` and `inhale` are in this list AND in `FX_NAMES`.
pub const EVENT_NAMES: [&str; 11] = [
    "takeoff", "land", "fall-impact", "trip", "slip", "recover", "lose-balance", "roll",
    "push-up", "pop", "inhale",
];

/// mix.py's 21 FX types (template/tools/mix.py FX table, via
/// CR-B1-05-CUES.md §2). An event whose `cue` is an FX name (and
/// not an event name) is a LITERAL placement in build_cues terms;
/// any other name fails loudly (R7).
pub const FX_NAMES: [&str; 21] = [
    "ding", "chirp", "chug", "stamp", "inhale", "heartbeat", "whoosh", "slide", "pop", "plop",
    "gulp", "boing", "bubbles", "sizzle", "sparkle", "thud", "slam", "sting", "tock", "zip",
    "alarm",
];

/// R7 gate for a clip-event `cue`: event vocabulary or FX name.
fn event_cue_ok(cue: &str) -> bool {
    EVENT_NAMES.contains(&cue) || FX_NAMES.contains(&cue)
}
/// Easing spellings — exactly `anim::Easing::from_name`'s set
/// (check_clips.py `EASINGS`).
pub const EASING_NAMES: [&str; 10] = [
    "linear", "smoother", "smootherstep", "step", "easeOutCubic", "eo", "easeInOutCubic", "eio",
    "easeOutBack", "eob",
];
pub const DEFAULT_EASE: &str = "smoother";

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum ChanKind {
    Scalar,
    Vec3,
    Color,
    StepMood,
    StepLegs,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum NodeKind {
    Character,
    Plain,
}

/// Channel vocabulary for a target (§15.7 tables, copied from
/// check_clips.py). `points[i].*` is parsed structurally.
fn channel_kind(
    target: &str,
    channel: &str,
    node_kinds: &HashMap<String, NodeKind>,
) -> Result<ChanKind, String> {
    use ChanKind::*;
    let scalar_table: &[&str] = &[];
    let _ = scalar_table;
    match target {
        "camera" => match channel {
            "eye" | "target" => Ok(Vec3),
            "fovY" => Ok(Scalar),
            _ => Err(format!("unknown channel 'camera.{channel}' — camera channels are: eye, fovY, target")),
        },
        "lights" => {
            if let Some(rest) = channel.strip_prefix("points[") {
                if let Some(close) = rest.find(']') {
                    let (idx, sub) = (&rest[..close], &rest[close + 1..]);
                    if idx.chars().all(|c| c.is_ascii_digit()) && sub.starts_with('.') {
                        return match &sub[1..] {
                            "intensity" | "distance" | "decay" => Ok(Scalar),
                            "color" => Ok(Color),
                            "position" => Ok(Vec3),
                            _ => Err(format!(
                                "unknown channel 'lights.{channel}' — point-light channels are points[i].color/decay/distance/intensity/position"
                            )),
                        };
                    }
                }
            }
            match channel {
                "key.intensity" | "rim.intensity" | "hemisphere.intensity" | "envIntensity" => {
                    Ok(Scalar)
                }
                "key.color" | "rim.color" | "hemisphere.sky" | "hemisphere.ground" => Ok(Color),
                "key.position" | "key.target" | "rim.position" => Ok(Vec3),
                _ => Err(format!("unknown channel 'lights.{channel}'")),
            }
        }
        "post" => {
            if channel == "grade.time" {
                return Err(
                    "channel 'post.grade.time' is not animatable: R3 pins the GRADE grain time to 0 for the whole film (docs/scene-format.md §14.5 R3)"
                        .to_string(),
                );
            }
            match channel {
                "bokeh.focus" | "bokeh.aperture" | "bokeh.maxblur" | "bloom.threshold"
                | "bloom.strength" | "bloom.radius" | "grade.sat" | "grade.warm"
                | "grade.exposure" | "grade.vignette" | "grade.grain" => Ok(Scalar),
                _ => Err(format!("unknown channel 'post.{channel}'")),
            }
        }
        "fog" => match channel {
            "near" | "far" => Ok(Scalar),
            "color" => Ok(Color),
            _ => Err(format!("unknown channel 'fog.{channel}'")),
        },
        _ => match node_kinds.get(target) {
            Some(NodeKind::Character) => match channel {
                "position" => Ok(Vec3),
                "mood" => Ok(StepMood),
                "legs" => Ok(StepLegs),
                "armL.0" | "armL.1" | "armR.0" | "armR.1" | "look.0" | "look.1" | "blink"
                | "talk" | "phase" | "lean" | "bob" | "squash" | "headTilt" | "headTurn"
                | "camPitch" | "sitOn" | "height" => Ok(Scalar),
                _ => Err(format!("unknown channel '{target}.{channel}' — character channels are the §15.7 table")),
            },
            Some(NodeKind::Plain) => match channel {
                "translate" | "scale" => Ok(Vec3),
                "rotate.x" | "rotate.y" | "rotate.z" => Ok(Scalar),
                _ => Err(format!("unknown channel '{target}.{channel}' — node channels are: rotate.x, rotate.y, rotate.z, scale, translate")),
            },
            None => Err(format!(
                "unknown target node '{target}' — not a reserved target ({}) and no node in this file has that name",
                RESERVED_TARGETS.join(", ")
            )),
        },
    }
}

fn key_value_ok(kind: ChanKind, v: &Value) -> bool {
    match kind {
        ChanKind::Scalar => is_number(v),
        ChanKind::Vec3 => match v.as_arr() {
            Some(a) => a.len() == 3 && a.iter().all(is_number),
            None => false,
        },
        ChanKind::Color => matches!(v, Value::Str(s) if is_color_str(s)),
        ChanKind::StepMood => matches!(v, Value::Str(s) if MOODS.contains(&s.as_str())),
        ChanKind::StepLegs => matches!(v, Value::Str(s) if LEG_STATES.contains(&s.as_str())),
    }
}

/// name -> kind over the document's nodes, recursively
/// (check_clips.py `collect_nodes`: first occurrence wins; a name
/// colliding with a reserved target is a failure).
fn collect_node_kinds(root: &Value) -> Result<HashMap<String, NodeKind>, String> {
    fn walk(nodes: &Value, kinds: &mut HashMap<String, NodeKind>) -> Result<(), String> {
        if let Some(items) = nodes.as_arr() {
            for n in items {
                if is_obj(n) {
                    if let Some(name) = n.get("name").and_then(Value::as_str) {
                        if RESERVED_TARGETS.contains(&name) {
                            return Err(format!(
                                "node named '{name}' collides with the reserved clip target '{name}' — rename the node; its tracks would be unaddressable"
                            ));
                        }
                        kinds.entry(name.to_string()).or_insert_with(|| {
                            if n.get("type").and_then(Value::as_str) == Some("character") {
                                NodeKind::Character
                            } else {
                                NodeKind::Plain
                            }
                        });
                    }
                    if let Some(children) = n.get("children") {
                        walk(children, kinds)?;
                    }
                }
            }
        }
        Ok(())
    }
    let mut kinds = HashMap::new();
    if let Some(nodes) = root.get("nodes") {
        walk(nodes, &mut kinds)?;
    }
    Ok(kinds)
}

// ---------------------------------------------------------------------
// Easing + sampling (f64 law, §15.8)
// ---------------------------------------------------------------------

/// Evaluate an easing curve at `u` in f64 — the film.rs forms
/// (`eo`/`eio`/`eob`/`smoother`), transcribed.
pub fn ease_eval(name: &str, u: f64) -> f64 {
    let u = u.clamp(0.0, 1.0);
    match name {
        "linear" => u,
        "step" => {
            if u >= 1.0 {
                1.0
            } else {
                0.0
            }
        }
        "easeOutCubic" | "eo" => 1.0 - (1.0 - u).powi(3),
        "easeInOutCubic" | "eio" => {
            if u < 0.5 {
                4.0 * u * u * u
            } else {
                1.0 - (-2.0 * u + 2.0).powi(3) / 2.0
            }
        }
        "easeOutBack" | "eob" => {
            let c1 = 1.70158f64;
            let c3 = c1 + 1.0;
            1.0 + c3 * (u - 1.0).powi(3) + c1 * (u - 1.0).powi(2)
        }
        // "smoother" | "smootherstep" — the house default.
        _ => u * u * u * (u * (u * 6.0 - 15.0) + 10.0),
    }
}

/// One format key for a scalar track: ease names the segment
/// ENDING at this key (§15.6).
#[derive(Clone, Debug, PartialEq)]
pub struct ScalarKey {
    pub t: f64,
    pub value: f64,
    pub ease: String,
}

/// Sample a scalar key list at `tau` (§15.8): hold outside the
/// key range; inside, `u` is eased by the ARRIVING key's ease.
pub fn sample_scalar_keys(keys: &[ScalarKey], tau: f64) -> f64 {
    if keys.is_empty() {
        return 0.0;
    }
    if tau <= keys[0].t || keys.len() == 1 {
        return keys[0].value;
    }
    for w in keys.windows(2) {
        let (a, b) = (&w[0], &w[1]);
        if tau < b.t {
            let span = b.t - a.t;
            let u = if span > 0.0 { (tau - a.t) / span } else { 1.0 };
            let e = ease_eval(&b.ease, u);
            return a.value + (b.value - a.value) * e;
        }
    }
    keys[keys.len() - 1].value
}

/// The §15.8 poseTrack conversion (exact, mechanical): each
/// poseTrack key `[t, v, m]` becomes a hold key at `t - m`
/// carrying the previous value (unless one already sits there)
/// plus the arrival key `{t, v, ease: "smoother"}`; `m = 0`
/// arrives with ease `"step"`; a missing `m` uses the film.rs
/// `key()` default 0.35. Mirrors stage_clips.py
/// `convert_posetrack`, including its overlap failure.
pub fn pose_track_to_clip_keys(
    keys: &[(f64, f64, Option<f64>)],
) -> Result<Vec<ScalarKey>, String> {
    if keys.is_empty() {
        return Err("poseTrack conversion: key table is empty".to_string());
    }
    let mut out: Vec<ScalarKey> = vec![ScalarKey {
        t: keys[0].0,
        value: keys[0].1,
        ease: DEFAULT_EASE.to_string(),
    }];
    let mut prev_v = keys[0].1;
    for &(t, v, m) in &keys[1..] {
        let mv = m.unwrap_or(0.35);
        if mv == 0.0 {
            out.push(ScalarKey { t, value: v, ease: "step".to_string() });
        } else {
            let hold_t = t - mv;
            let last_t = out[out.len() - 1].t;
            if hold_t < last_t && (hold_t - last_t).abs() > 1e-9 {
                return Err(format!(
                    "poseTrack conversion: key at t={t} moves over {mv}s, so its hold would start at {hold_t} — before the previous key at {last_t}; pose_track() never holds the previous value here and the clip grammar cannot express the overlap"
                ));
            }
            if hold_t > last_t {
                out.push(ScalarKey { t: hold_t, value: prev_v, ease: DEFAULT_EASE.to_string() });
            }
            out.push(ScalarKey { t, value: v, ease: DEFAULT_EASE.to_string() });
        }
        prev_v = v;
    }
    Ok(out)
}

// ---------------------------------------------------------------------
// Clip data model
// ---------------------------------------------------------------------

#[derive(Clone, Debug)]
enum KeyVal {
    Num(f64),
    Vec3([f64; 3]),
    Color(String),
    Step(String),
}

#[derive(Clone, Debug)]
struct TrackDef {
    node: String,
    channel: String,
    kind: ChanKind,
    /// (t, value, ease-name-on-arriving-key) in authored order
    /// (validated strictly increasing).
    keys: Vec<(f64, KeyVal, String)>,
}

/// One authored clip event (CR-B1-05 R5/R6 extension): clip-local
/// `t`, closed key set {t, cue, gain, dur}. Mirrors TS
/// `ClipEvent {t, cue, gain?}` (core/clips.ts:140) plus `dur`,
/// which the TS type lacks but the R5/build_cues event shape
/// carries — accepted and passed through verbatim (see the
/// design note §5 for the discrepancy).
#[derive(Clone, Debug)]
struct ClipEventDef {
    t: f64,
    cue: String,
    gain: Option<f64>,
    dur: Option<f64>,
}

#[derive(Clone, Debug)]
struct ClipDef {
    duration: f64,
    tracks: Vec<TrackDef>,
    uses: Vec<String>,
    events: Vec<ClipEventDef>,
}

/// A validated `clips` block (§15.6), ready to sample.
#[derive(Clone, Debug)]
pub struct ClipSet {
    clips: Vec<(String, ClipDef)>,
}

impl ClipSet {
    fn get(&self, name: &str) -> Option<&ClipDef> {
        self.clips.iter().find(|(n, _)| n == name).map(|(_, c)| c)
    }

    pub fn clip_names(&self) -> Vec<&str> {
        self.clips.iter().map(|(n, _)| n.as_str()).collect()
    }

    /// Build + validate from a document root (checks 1–4 of
    /// check_clips.py; the §11 f32 check 5 stays with
    /// validate_film.py over the processed document, per §15.3).
    /// `root` must be the EXPANDED document (targets resolve
    /// against post-expansion node names, §15.5).
    pub fn from_root(root: &Value) -> Result<ClipSet, String> {
        let node_kinds = collect_node_kinds(root)?;
        Self::from_root_with_kinds(root, &node_kinds)
    }

    fn from_root_with_kinds(
        root: &Value,
        node_kinds: &HashMap<String, NodeKind>,
    ) -> Result<ClipSet, String> {
        let empty = Value::Obj(Vec::new());
        let clips_v = root.get("clips").unwrap_or(&empty);
        let clip_pairs = match clips_v {
            Value::Obj(p) => p.clone(),
            _ => return Err("'clips' is not an object".to_string()),
        };
        let mut clips: Vec<(String, ClipDef)> = Vec::new();
        for (name, clip) in &clip_pairs {
            let cpath = format!("clips.{name}");
            if !is_obj(clip) {
                return Err(format!("{cpath}: clip is not an object"));
            }
            let duration = match clip.get("duration").and_then(Value::as_f64) {
                Some(d) if d > 0.0 => d,
                Some(d) => return Err(format!("{cpath}.duration: must be > 0, got {d}")),
                None => return Err(format!("{cpath}.duration: missing or not a number")),
            };
            let tracks_v = match clip.get("tracks") {
                Some(Value::Arr(a)) if !a.is_empty() => a.clone(),
                _ => return Err(format!("{cpath}.tracks: missing, not an array, or empty")),
            };
            let mut tracks: Vec<TrackDef> = Vec::new();
            let mut seen: HashSet<(String, String)> = HashSet::new();
            for (ti, track) in tracks_v.iter().enumerate() {
                let tpath = format!("{cpath}.tracks[{ti}]");
                if !is_obj(track) {
                    return Err(format!("{tpath}: track is not an object"));
                }
                let tgt = track.get("target").cloned().unwrap_or(Value::Null);
                let (tnode, tchan) = (
                    tgt.get("node").and_then(Value::as_str).map(|s| s.to_string()),
                    tgt.get("channel").and_then(Value::as_str).map(|s| s.to_string()),
                );
                let (tnode, tchan) = match (tnode, tchan) {
                    (Some(a), Some(b)) => (a, b),
                    _ => {
                        return Err(format!(
                            "{tpath}.target: must be an object with string 'node' and 'channel'"
                        ))
                    }
                };
                if !seen.insert((tnode.clone(), tchan.clone())) {
                    return Err(format!(
                        "{tpath}: duplicate track for '{tnode}.{tchan}' — one clip, one track per channel"
                    ));
                }
                let kind = channel_kind(&tnode, &tchan, node_kinds)
                    .map_err(|e| format!("{tpath}: {e}"))?;
                let keys_v = match track.get("keys") {
                    Some(Value::Arr(a)) if !a.is_empty() => a.clone(),
                    _ => return Err(format!("{tpath}.keys: missing, not an array, or empty")),
                };
                let mut keys: Vec<(f64, KeyVal, String)> = Vec::new();
                let mut prev_t: Option<f64> = None;
                for (ki, key) in keys_v.iter().enumerate() {
                    let kpath = format!("{tpath}.keys[{ki}]");
                    if !is_obj(key) {
                        return Err(format!("{kpath}: key is not an object"));
                    }
                    let t = key
                        .get("t")
                        .and_then(Value::as_f64)
                        .ok_or_else(|| format!("{kpath}.t: missing or not a number"))?;
                    if t < 0.0 {
                        return Err(format!("{kpath}.t: {t} is negative — clip-local times start at 0"));
                    }
                    if t > duration {
                        return Err(format!(
                            "{kpath}.t: {t} is beyond the clip duration {duration} (clip '{name}')"
                        ));
                    }
                    if let Some(p) = prev_t {
                        if t == p {
                            return Err(format!(
                                "{tpath}.keys[{ki}].t: duplicate key time {t} — two values at one instant is ambiguous"
                            ));
                        }
                        if t < p {
                            return Err(format!(
                                "{tpath}.keys[{ki}].t: key times out of order — {t} follows {p}; times must be strictly increasing"
                            ));
                        }
                    }
                    prev_t = Some(t);
                    let value = key
                        .get("value")
                        .ok_or_else(|| format!("{kpath}.value: missing"))?;
                    if !key_value_ok(kind, value) {
                        return Err(format!(
                            "{kpath}.value: value does not match the kind of channel '{tnode}.{tchan}'"
                        ));
                    }
                    let ease = match key.get("ease") {
                        None => DEFAULT_EASE.to_string(),
                        Some(Value::Str(s)) => {
                            if !EASING_NAMES.contains(&s.as_str()) {
                                return Err(format!(
                                    "{kpath}.ease: unknown easing '{s}' — the engine's curves are: {}",
                                    EASING_NAMES.join(", ")
                                ));
                            }
                            s.clone()
                        }
                        Some(_) => return Err(format!("{kpath}.ease: not a string")),
                    };
                    if matches!(kind, ChanKind::StepMood | ChanKind::StepLegs) && ease != "step" {
                        return Err(format!(
                            "{kpath}.ease: channel '{tnode}.{tchan}' is discrete; its keys must use ease 'step', got '{ease}'"
                        ));
                    }
                    let kv = match kind {
                        ChanKind::Scalar => KeyVal::Num(value.as_f64().unwrap()),
                        ChanKind::Vec3 => {
                            let a = value.as_arr().unwrap();
                            KeyVal::Vec3([
                                a[0].as_f64().unwrap(),
                                a[1].as_f64().unwrap(),
                                a[2].as_f64().unwrap(),
                            ])
                        }
                        ChanKind::Color => KeyVal::Color(value.as_str().unwrap().to_string()),
                        ChanKind::StepMood | ChanKind::StepLegs => {
                            KeyVal::Step(value.as_str().unwrap().to_string())
                        }
                    };
                    keys.push((t, kv, ease));
                }
                tracks.push(TrackDef { node: tnode, channel: tchan, kind, keys });
            }
            let uses: Vec<String> = match clip.get("uses") {
                None => Vec::new(),
                Some(Value::Arr(a)) => {
                    let mut out = Vec::new();
                    for u in a {
                        match u.as_str() {
                            Some(s) => out.push(s.to_string()),
                            None => {
                                return Err(format!("{cpath}.uses: must be an array of clip names"))
                            }
                        }
                    }
                    out
                }
                Some(_) => return Err(format!("{cpath}.uses: must be an array of clip names")),
            };
            // Events (R5/R6 extension — NOT part of §15.6 as
            // written; check_clips.py does not look at this field
            // at all, so this validation is eval15-only strictness,
            // in the §15.1 posture: a consumer that reads a field
            // validates it). Closed entry keys {t, cue, gain, dur};
            // t clip-local within [0, duration]; cue in the closed
            // R7 vocabulary (event names or FX names).
            let events: Vec<ClipEventDef> = match clip.get("events") {
                None => Vec::new(),
                Some(Value::Arr(a)) => {
                    let mut out = Vec::new();
                    for (ei, ev) in a.iter().enumerate() {
                        let epath = format!("{cpath}.events[{ei}]");
                        if !is_obj(ev) {
                            return Err(format!("{epath}: event is not an object"));
                        }
                        if let Value::Obj(pairs) = ev {
                            for (k, _) in pairs {
                                if !["t", "cue", "gain", "dur"].contains(&k.as_str()) {
                                    return Err(format!(
                                        "{epath}: unknown key '{k}' — event keys are exactly t, cue, gain, dur"
                                    ));
                                }
                            }
                        }
                        let et = ev
                            .get("t")
                            .and_then(Value::as_f64)
                            .ok_or_else(|| format!("{epath}.t: missing or not a number"))?;
                        if et < 0.0 || et > duration {
                            return Err(format!(
                                "{epath}.t: {et} is outside the clip duration [0, {duration}]"
                            ));
                        }
                        let cue = ev
                            .get("cue")
                            .and_then(Value::as_str)
                            .ok_or_else(|| format!("{epath}.cue: missing or not a string"))?
                            .to_string();
                        if !event_cue_ok(&cue) {
                            return Err(format!(
                                "{epath}.cue: '{cue}' is not an event name or an FX type — the vocabularies are closed (CR-B1-05 R7): events [{}]; FX [{}]",
                                EVENT_NAMES.join(", "),
                                FX_NAMES.join(", ")
                            ));
                        }
                        let gain = match ev.get("gain") {
                            None => None,
                            Some(v) => Some(v.as_f64().ok_or_else(|| {
                                format!("{epath}.gain: not a number")
                            })?),
                        };
                        let dur = match ev.get("dur") {
                            None => None,
                            Some(v) => Some(v.as_f64().ok_or_else(|| {
                                format!("{epath}.dur: not a number")
                            })?),
                        };
                        out.push(ClipEventDef { t: et, cue, gain, dur });
                    }
                    out
                }
                Some(_) => return Err(format!("{cpath}.events: must be an array of event objects")),
            };
            clips.push((name.clone(), ClipDef { duration, tracks, uses, events }));
        }
        let set = ClipSet { clips };
        // References: every `uses` name resolves; the uses graph
        // is acyclic (self-use counts); layered clips may not
        // define the same target+channel twice.
        for (name, clip) in &set.clips {
            for u in &clip.uses {
                if set.get(u).is_none() {
                    return Err(format!("clips.{name}.uses: unknown clip '{u}'"));
                }
            }
        }
        // Cycle check with path (iterative DFS).
        fn dfs(
            set: &ClipSet,
            u: &str,
            state: &mut HashMap<String, u8>,
            stack: &mut Vec<String>,
        ) -> Result<(), String> {
            state.insert(u.to_string(), 1);
            stack.push(u.to_string());
            if let Some(clip) = set.get(u) {
                for w in &clip.uses {
                    match state.get(w).copied().unwrap_or(0) {
                        1 => {
                            let start = stack.iter().position(|x| x == w).unwrap();
                            let mut cyc: Vec<String> = stack[start..].to_vec();
                            cyc.push(w.clone());
                            return Err(format!("clips: reference cycle: {}", cyc.join(" -> ")));
                        }
                        0 => dfs(set, w, state, stack)?,
                        _ => {}
                    }
                }
            }
            stack.pop();
            state.insert(u.to_string(), 2);
            Ok(())
        }
        let mut state: HashMap<String, u8> = HashMap::new();
        let mut names: Vec<String> = set.clips.iter().map(|(n, _)| n.clone()).collect();
        names.sort();
        for n in names {
            if state.get(&n).copied().unwrap_or(0) == 0 {
                dfs(&set, &n, &mut state, &mut Vec::new())?;
            }
        }
        for (name, _) in &set.clips {
            let mut seen: HashMap<(String, String), String> = HashMap::new();
            for (tnode, tchan, src) in set.layered_idents(name) {
                if let Some(prev) = seen.get(&(tnode.clone(), tchan.clone())) {
                    if prev != &src {
                        return Err(format!(
                            "clips.{name}: channel '{tnode}.{tchan}' is defined by both '{prev}' and layered clip '{src}' — layered clips may not fight over one channel"
                        ));
                    }
                }
                seen.insert((tnode, tchan), src);
            }
        }
        Ok(set)
    }

    /// (node, channel, defining-clip) for a clip plus everything
    /// it transitively uses, own tracks first.
    fn layered_idents(&self, name: &str) -> Vec<(String, String, String)> {
        let mut out = Vec::new();
        let mut visited: HashSet<String> = HashSet::new();
        let mut stack: Vec<String> = vec![name.to_string()];
        // Own clip first, then uses in order (DFS pre-order).
        fn go(
            set: &ClipSet,
            n: &str,
            visited: &mut HashSet<String>,
            out: &mut Vec<(String, String, String)>,
        ) {
            if !visited.insert(n.to_string()) {
                return;
            }
            if let Some(clip) = set.get(n) {
                for tr in &clip.tracks {
                    out.push((tr.node.clone(), tr.channel.clone(), n.to_string()));
                }
                for u in &clip.uses {
                    go(set, u, visited, out);
                }
            }
        }
        let _ = &mut stack;
        go(self, name, &mut visited, &mut out);
        out
    }

    fn layered_tracks(&self, name: &str) -> Vec<TrackDef> {
        let mut out = Vec::new();
        let mut visited: HashSet<String> = HashSet::new();
        fn go(set: &ClipSet, n: &str, visited: &mut HashSet<String>, out: &mut Vec<TrackDef>) {
            if !visited.insert(n.to_string()) {
                return;
            }
            if let Some(clip) = set.get(n) {
                for tr in &clip.tracks {
                    out.push(tr.clone());
                }
                for u in &clip.uses {
                    go(set, u, visited, out);
                }
            }
        }
        go(self, name, &mut visited, &mut out);
        out
    }

    pub fn duration(&self, clip: &str) -> Option<f64> {
        self.get(clip).map(|c| c.duration)
    }

    /// Sample one instance: layered tracks at the mapped time.
    /// Returns (target-node, channel, value) in track order.
    pub fn sample(
        &self,
        clip: &str,
        ct: &ClipTime,
        scene_t: f64,
    ) -> Result<Vec<(String, String, Sampled)>, String> {
        let def = self
            .get(clip)
            .ok_or_else(|| format!("instance references unknown clip '{clip}'"))?;
        let tau = map_time(ct, scene_t, def.duration);
        let mut out = Vec::new();
        for tr in self.layered_tracks(clip) {
            out.push((tr.node.clone(), tr.channel.clone(), sample_track_f64(&tr, tau).into_sampled()));
        }
        Ok(out)
    }

    /// Events of a clip plus everything it transitively uses
    /// (own events first, then uses in order — the same layering
    /// as tracks; §15.6 is silent on events, see design note §5).
    fn layered_events(&self, name: &str) -> Vec<ClipEventDef> {
        let mut out = Vec::new();
        let mut visited: HashSet<String> = HashSet::new();
        fn go(set: &ClipSet, n: &str, visited: &mut HashSet<String>, out: &mut Vec<ClipEventDef>) {
            if !visited.insert(n.to_string()) {
                return;
            }
            if let Some(clip) = set.get(n) {
                for ev in &clip.events {
                    out.push(ev.clone());
                }
                for u in &clip.uses {
                    go(set, u, visited, out);
                }
            }
        }
        go(self, name, &mut visited, &mut out);
        out
    }

    /// True if the clip (or anything it layers) carries events.
    pub fn has_events(&self, clip: &str) -> bool {
        self.get(clip).map(|_| !self.layered_events(clip).is_empty()).unwrap_or(false)
    }

    /// R6 emission for ONE instance over a film-time window.
    ///
    /// Time law: an event at clip-local `te` sits at scene-local
    /// `(te - ct.offset) / ct.rate` — the exact inverse of
    /// §15.8's `raw = t·rate + offset` — and at absolute film
    /// time `segment_start + scene-local`. With the default
    /// ClipTime (offset 0, rate 1) this reduces to R5's formula
    /// `startS + ClipEvent.t` (R5's "instance offset within the
    /// shot" is 0 for a §15 instance: instances are whole-scene
    /// bindings, placement lives in `segment_start`).
    ///
    /// The window is HALF-OPEN `[w0, w1)`: a driver stepping
    /// frames with contiguous windows `[n/fps, (n+1)/fps)` gets
    /// every event exactly once. Events whose scene-local time
    /// is negative (clip-local `t` before the instance's entry
    /// offset) never fire. Looping instances (loop OR ping-pong)
    /// emit NOTHING — R5/cliplib law: looping clips carry no
    /// events; the law is applied per-instance because §15 puts
    /// the wrap on the instance, not the clip.
    pub fn events_in_window(
        &self,
        clip: &str,
        ct: &ClipTime,
        segment_start: f64,
        w0: f64,
        w1: f64,
    ) -> Result<Vec<EmittedEvent>, String> {
        if self.get(clip).is_none() {
            return Err(format!("instance references unknown clip '{clip}'"));
        }
        if ct.rate <= 0.0 {
            return Err(format!("clipTime.rate: must be > 0, got {}", ct.rate));
        }
        if ct.wrap != WrapMode::Clamp {
            return Ok(Vec::new());
        }
        let mut out = Vec::new();
        for ev in self.layered_events(clip) {
            let scene_t = (ev.t - ct.offset) / ct.rate;
            if scene_t < 0.0 {
                continue;
            }
            let abs = segment_start + scene_t;
            if abs >= w0 && abs < w1 {
                out.push(EmittedEvent { t: abs, cue: ev.cue.clone(), gain: ev.gain, dur: ev.dur });
            }
        }
        out.sort_by(|a, b| a.t.partial_cmp(&b.t).unwrap_or(std::cmp::Ordering::Equal));
        Ok(out)
    }

    /// R6 emission merged across instances (document order breaks
    /// time ties), sorted by absolute `t` — the collection a
    /// driver hands to the clip-events.json sidecar writer.
    pub fn collect_events(
        &self,
        instances: &[(&str, ClipTime)],
        segment_start: f64,
        w0: f64,
        w1: f64,
    ) -> Result<Vec<EmittedEvent>, String> {
        let mut out = Vec::new();
        for (clip, ct) in instances {
            out.extend(self.events_in_window(clip, ct, segment_start, w0, w1)?);
        }
        out.sort_by(|a, b| a.t.partial_cmp(&b.t).unwrap_or(std::cmp::Ordering::Equal));
        Ok(out)
    }
}

/// One emitted clip event in the CR-B1-05 R5 shape
/// `{t, cue, gain?, dur?}` at ABSOLUTE film time — the
/// `build_cues` events-input shape. `gain`/`dur` are `None` when
/// the authored event omits them (the key is then absent from
/// the sidecar, exactly as authored cues omit absent opts).
#[derive(Clone, Debug, PartialEq)]
pub struct EmittedEvent {
    pub t: f64,
    pub cue: String,
    pub gain: Option<f64>,
    pub dur: Option<f64>,
}

impl EmittedEvent {
    /// The R5 JSON shape as a `Value`: keys in the order
    /// t, cue, gain, dur; absent opts omitted.
    pub fn to_value(&self) -> Value {
        let mut pairs = vec![
            ("t".to_string(), Value::Num(self.t)),
            ("cue".to_string(), Value::Str(self.cue.clone())),
        ];
        if let Some(g) = self.gain {
            pairs.push(("gain".to_string(), Value::Num(g)));
        }
        if let Some(d) = self.dur {
            pairs.push(("dur".to_string(), Value::Num(d)));
        }
        Value::Obj(pairs)
    }
}

/// An instance's time mapping (§15.6 `clipTime`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ClipTime {
    pub offset: f64,
    pub rate: f64,
    pub wrap: WrapMode,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WrapMode {
    Clamp,
    Loop,
    PingPong,
}

impl Default for ClipTime {
    fn default() -> Self {
        ClipTime { offset: 0.0, rate: 1.0, wrap: WrapMode::Clamp }
    }
}

impl ClipTime {
    /// Parse a `clipTime` value (None = all defaults). Every
    /// field defaults; `rate` must be > 0; `loop` is one of the
    /// three kebab spellings (check_clips.py check 4).
    pub fn parse(v: Option<&Value>, path: &str) -> Result<ClipTime, String> {
        let mut ct = ClipTime::default();
        let Some(v) = v else { return Ok(ct) };
        if !is_obj(v) {
            return Err(format!("{path}.clipTime: not an object"));
        }
        if let Some(o) = v.get("offset") {
            ct.offset = o
                .as_f64()
                .ok_or_else(|| format!("{path}.clipTime.offset: not a number"))?;
        }
        if let Some(r) = v.get("rate") {
            let r = r
                .as_f64()
                .ok_or_else(|| format!("{path}.clipTime.rate: not a number"))?;
            if r <= 0.0 {
                return Err(format!("{path}.clipTime.rate: must be > 0, got {r}"));
            }
            ct.rate = r;
        }
        if let Some(l) = v.get("loop") {
            ct.wrap = match l.as_str() {
                Some("clamp") => WrapMode::Clamp,
                Some("loop") => WrapMode::Loop,
                Some("ping-pong") => WrapMode::PingPong,
                _ => {
                    return Err(format!(
                        "{path}.clipTime.loop: unknown loop mode — modes are: clamp, loop, ping-pong"
                    ))
                }
            };
        }
        Ok(ct)
    }
}

/// §15.8 time mapping: raw = t·rate + offset, then wrap into
/// [0, D] per the instance's loop mode.
pub fn map_time(ct: &ClipTime, t: f64, duration: f64) -> f64 {
    let raw = t * ct.rate + ct.offset;
    match ct.wrap {
        WrapMode::Clamp => raw.clamp(0.0, duration),
        WrapMode::Loop => raw.rem_euclid(duration),
        WrapMode::PingPong => {
            let m = raw.rem_euclid(2.0 * duration);
            if m <= duration {
                m
            } else {
                2.0 * duration - m
            }
        }
    }
}

/// A sampled channel value. Colour is LINEAR `[f32; 3]` (§15.9);
/// step values ride this side channel and never a numeric lane.
#[derive(Clone, Debug, PartialEq)]
pub enum Sampled {
    Scalar(f32),
    Vec3([f32; 3]),
    ColorLin([f32; 3]),
    Step(String),
}

enum SampledF64 {
    Num(f64),
    Vec3([f64; 3]),
    ColorLin([f64; 3]),
    Step(String),
}

impl SampledF64 {
    fn into_sampled(self) -> Sampled {
        match self {
            SampledF64::Num(x) => Sampled::Scalar(x as f32),
            SampledF64::Vec3(v) => Sampled::Vec3([v[0] as f32, v[1] as f32, v[2] as f32]),
            SampledF64::ColorLin(v) => {
                Sampled::ColorLin([v[0] as f32, v[1] as f32, v[2] as f32])
            }
            SampledF64::Step(s) => Sampled::Step(s),
        }
    }
}

/// sRGB hex -> linear [f64; 3] (the §4.1 law, in f64).
fn hex_to_lin_f64(hex: &str) -> [f64; 3] {
    let b = hex.as_bytes();
    let byte = |i: usize| -> f64 {
        let hi = (b[i] as char).to_digit(16).unwrap_or(0) as f64;
        let lo = (b[i + 1] as char).to_digit(16).unwrap_or(0) as f64;
        (hi * 16.0 + lo) / 255.0
    };
    let lin = |c: f64| {
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    [lin(byte(1)), lin(byte(3)), lin(byte(5))]
}

/// Linear [f64; 3] -> `#RRGGBB` (nearest sRGB byte per channel).
/// Exact round-trip for authored hex values (they ARE bytes).
fn lin_to_hex(lin: [f64; 3]) -> String {
    let enc = |c: f64| -> u8 {
        let s = if c <= 0.0031308 {
            c * 12.92
        } else {
            1.055 * c.powf(1.0 / 2.4) - 0.055
        };
        (s.clamp(0.0, 1.0) * 255.0).round() as u8
    };
    format!("#{:02X}{:02X}{:02X}", enc(lin[0]), enc(lin[1]), enc(lin[2]))
}

fn sample_track_f64(tr: &TrackDef, tau: f64) -> SampledF64 {
    match tr.kind {
        ChanKind::Scalar => {
            let keys: Vec<ScalarKey> = tr
                .keys
                .iter()
                .map(|(t, v, e)| ScalarKey {
                    t: *t,
                    value: match v {
                        KeyVal::Num(x) => *x,
                        _ => 0.0,
                    },
                    ease: e.clone(),
                })
                .collect();
            SampledF64::Num(sample_scalar_keys(&keys, tau))
        }
        ChanKind::Vec3 => {
            let mut out = [0.0f64; 3];
            for c in 0..3 {
                let keys: Vec<ScalarKey> = tr
                    .keys
                    .iter()
                    .map(|(t, v, e)| ScalarKey {
                        t: *t,
                        value: match v {
                            KeyVal::Vec3(a) => a[c],
                            _ => 0.0,
                        },
                        ease: e.clone(),
                    })
                    .collect();
                out[c] = sample_scalar_keys(&keys, tau);
            }
            SampledF64::Vec3(out)
        }
        ChanKind::Color => {
            let mut out = [0.0f64; 3];
            for c in 0..3 {
                let keys: Vec<ScalarKey> = tr
                    .keys
                    .iter()
                    .map(|(t, v, e)| ScalarKey {
                        t: *t,
                        value: match v {
                            KeyVal::Color(h) => hex_to_lin_f64(h)[c],
                            _ => 0.0,
                        },
                        ease: e.clone(),
                    })
                    .collect();
                out[c] = sample_scalar_keys(&keys, tau);
            }
            SampledF64::ColorLin(out)
        }
        ChanKind::StepMood | ChanKind::StepLegs => {
            // Latest key with t <= tau; before the first, hold it.
            let mut val = match &tr.keys[0].1 {
                KeyVal::Step(s) => s.clone(),
                _ => String::new(),
            };
            for (t, v, _) in &tr.keys {
                if *t <= tau {
                    if let KeyVal::Step(s) = v {
                        val = s.clone();
                    }
                } else {
                    break;
                }
            }
            SampledF64::Step(val)
        }
    }
}

// ---------------------------------------------------------------------
// Folding sampled values into a document (§15.6: the clip's tracks
// REPLACE the target's static value at evaluation time)
// ---------------------------------------------------------------------

fn set_num_field(obj: &mut Value, key: &str, x: f64) {
    obj_set(obj, key, Value::Num(x));
}

fn set_vec3_field(obj: &mut Value, key: &str, v: [f64; 3]) {
    obj_set(
        obj,
        key,
        Value::Arr(vec![Value::Num(v[0]), Value::Num(v[1]), Value::Num(v[2])]),
    );
}

fn ensure_obj<'a>(parent: &'a mut Value, key: &str) -> &'a mut Value {
    if !matches!(parent.get(key), Some(Value::Obj(_))) {
        obj_set(parent, key, Value::Obj(Vec::new()));
    }
    match parent {
        Value::Obj(pairs) => &mut pairs.iter_mut().find(|(k, _)| k == key).unwrap().1,
        _ => unreachable!(),
    }
}

fn find_node_mut<'a>(nodes: &'a mut Value, name: &str) -> Option<&'a mut Value> {
    fn go<'a>(v: &'a mut Value, name: &str) -> Option<&'a mut Value> {
        if let Value::Arr(items) = v {
            // Search this level first (document order), then
            // descend — two passes to avoid holding borrows.
            let mut found_path: Option<Vec<usize>> = None;
            fn locate(nodes: &Value, name: &str, path: &mut Vec<usize>) -> bool {
                if let Value::Arr(items) = nodes {
                    for (i, n) in items.iter().enumerate() {
                        path.push(i);
                        if n.get("name").and_then(Value::as_str) == Some(name) {
                            return true;
                        }
                        if let Some(ch) = n.get("children") {
                            if locate(ch, name, path) {
                                return true;
                            }
                        }
                        path.pop();
                    }
                }
                false
            }
            let mut p: Vec<usize> = Vec::new();
            if locate(v, name, &mut p) {
                found_path = Some(p);
            }
            if let Some(path) = found_path {
                // Path indexes alternate levels: nodes[i], then
                // that node's children[j], and so on.
                let mut cur = v;
                let mut idx_iter = path.iter().peekable();
                while let Some(i) = idx_iter.next() {
                    let node = match cur {
                        Value::Arr(items) => &mut items[*i],
                        _ => return None,
                    };
                    if idx_iter.peek().is_none() {
                        return Some(node);
                    }
                    cur = match node.get_mut_children() {
                        Some(c) => c,
                        None => return None,
                    };
                }
                return None;
            }
        }
        None
    }
    go(nodes, name)
}

/// Helper used by find_node_mut's path walk.
trait ChildrenMut {
    fn get_mut_children(&mut self) -> Option<&mut Value>;
}
impl ChildrenMut for Value {
    fn get_mut_children(&mut self) -> Option<&mut Value> {
        if let Value::Obj(pairs) = self {
            for (k, v) in pairs.iter_mut() {
                if k == "children" {
                    return Some(v);
                }
            }
        }
        None
    }
}

/// Fold one sampled value into its target inside `root`.
fn fold_value(
    root: &mut Value,
    target: &str,
    channel: &str,
    val: &SampledF64,
    node_kinds: &HashMap<String, NodeKind>,
) -> Result<(), String> {
    match target {
        "camera" => {
            let cam = ensure_obj(root, "camera");
            match (channel, val) {
                ("eye", SampledF64::Vec3(v)) => set_vec3_field(cam, "eye", *v),
                ("target", SampledF64::Vec3(v)) => set_vec3_field(cam, "target", *v),
                ("fovY", SampledF64::Num(x)) => set_num_field(cam, "fovY", *x),
                _ => return Err(format!("fold: channel 'camera.{channel}' value kind mismatch")),
            }
            Ok(())
        }
        "lights" => {
            let lights = ensure_obj(root, "lights");
            fold_lights(lights, channel, val)
        }
        "post" => {
            let post = ensure_obj(root, "post");
            let (sub, field) = channel
                .split_once('.')
                .ok_or_else(|| format!("fold: bad post channel '{channel}'"))?;
            let block = ensure_obj(post, sub);
            match val {
                SampledF64::Num(x) => set_num_field(block, field, *x),
                _ => return Err(format!("fold: channel 'post.{channel}' value kind mismatch")),
            }
            Ok(())
        }
        "fog" => {
            let fog = ensure_obj(root, "fog");
            match (channel, val) {
                ("near", SampledF64::Num(x)) => set_num_field(fog, "near", *x),
                ("far", SampledF64::Num(x)) => set_num_field(fog, "far", *x),
                ("color", SampledF64::ColorLin(l)) => {
                    obj_set(fog, "color", Value::Str(lin_to_hex(*l)))
                }
                _ => return Err(format!("fold: channel 'fog.{channel}' value kind mismatch")),
            }
            Ok(())
        }
        _ => {
            let kind = node_kinds
                .get(target)
                .copied()
                .ok_or_else(|| format!("fold: target node '{target}' not found"))?;
            let nodes = match root.get_mut_nodes() {
                Some(n) => n,
                None => return Err("fold: document has no 'nodes' array".to_string()),
            };
            let node = find_node_mut(nodes, target)
                .ok_or_else(|| format!("fold: target node '{target}' not found in the node tree"))?;
            match kind {
                NodeKind::Character => fold_character(node, channel, val),
                NodeKind::Plain => fold_plain_node(node, channel, val),
            }
        }
    }
}

trait NodesMut {
    fn get_mut_nodes(&mut self) -> Option<&mut Value>;
}
impl NodesMut for Value {
    fn get_mut_nodes(&mut self) -> Option<&mut Value> {
        if let Value::Obj(pairs) = self {
            for (k, v) in pairs.iter_mut() {
                if k == "nodes" {
                    return Some(v);
                }
            }
        }
        None
    }
}

fn fold_lights(lights: &mut Value, channel: &str, val: &SampledF64) -> Result<(), String> {
    if channel == "envIntensity" {
        return match val {
            SampledF64::Num(x) => {
                set_num_field(lights, "envIntensity", *x);
                Ok(())
            }
            _ => Err("fold: lights.envIntensity value kind mismatch".to_string()),
        };
    }
    if let Some(rest) = channel.strip_prefix("points[") {
        let close = rest.find(']').ok_or_else(|| format!("fold: bad channel '{channel}'"))?;
        let idx: usize = rest[..close]
            .parse()
            .map_err(|_| format!("fold: bad channel '{channel}'"))?;
        let field = &rest[close + 2..]; // skip "]."
        if !matches!(lights.get("points"), Some(Value::Arr(_))) {
            obj_set(lights, "points", Value::Arr(Vec::new()));
        }
        let points = match lights {
            Value::Obj(pairs) => {
                &mut pairs.iter_mut().find(|(k, _)| k == "points").unwrap().1
            }
            _ => unreachable!(),
        };
        if let Value::Arr(items) = points {
            while items.len() <= idx {
                items.push(Value::Obj(Vec::new()));
            }
            let pt = &mut items[idx];
            return fold_light_leaf(pt, field, val, channel);
        }
        return Err("fold: lights.points is not an array".to_string());
    }
    let (sub, field) = channel
        .split_once('.')
        .ok_or_else(|| format!("fold: bad lights channel '{channel}'"))?;
    let block = ensure_obj(lights, sub);
    fold_light_leaf(block, field, val, channel)
}

fn fold_light_leaf(
    obj: &mut Value,
    field: &str,
    val: &SampledF64,
    channel: &str,
) -> Result<(), String> {
    match val {
        SampledF64::Num(x) => set_num_field(obj, field, *x),
        SampledF64::Vec3(v) => set_vec3_field(obj, field, *v),
        SampledF64::ColorLin(l) => obj_set(obj, field, Value::Str(lin_to_hex(*l))),
        SampledF64::Step(_) => {
            return Err(format!("fold: channel 'lights.{channel}' value kind mismatch"))
        }
    }
    Ok(())
}

fn fold_character(node: &mut Value, channel: &str, val: &SampledF64) -> Result<(), String> {
    match (channel, val) {
        ("position", SampledF64::Vec3(v)) => {
            set_vec3_field(node, "position", *v);
            Ok(())
        }
        ("height", SampledF64::Num(x)) => {
            set_num_field(node, "height", *x);
            Ok(())
        }
        ("mood", SampledF64::Step(s)) => {
            let st = ensure_obj(node, "state");
            obj_set(st, "mood", Value::Str(s.clone()));
            Ok(())
        }
        ("legs", SampledF64::Step(s)) => {
            let st = ensure_obj(node, "state");
            obj_set(st, "legs", Value::Str(s.clone()));
            Ok(())
        }
        _ => {
            // Pair components and plain state scalars.
            let st = ensure_obj(node, "state");
            if let Some((pair, comp)) = channel.split_once('.') {
                if matches!(pair, "armL" | "armR" | "look") {
                    let idx: usize = comp
                        .parse()
                        .map_err(|_| format!("fold: bad channel '{channel}'"))?;
                    // scenefile.rs pair_field defaults: armL/armR
                    // (8, 6), look (0, 0) — an absent pair starts
                    // from its loader default, never (0, 0) for arms.
                    let default_pair = if pair == "look" { [0.0, 0.0] } else { [8.0, 6.0] };
                    let mut arr = match st.get(pair) {
                        Some(Value::Arr(a)) if a.len() == 2 => {
                            [a[0].as_f64().unwrap_or(default_pair[0]), a[1].as_f64().unwrap_or(default_pair[1])]
                        }
                        _ => default_pair,
                    };
                    if let SampledF64::Num(x) = val {
                        arr[idx] = *x;
                    } else {
                        return Err(format!("fold: channel '{channel}' value kind mismatch"));
                    }
                    obj_set(
                        st,
                        pair,
                        Value::Arr(vec![Value::Num(arr[0]), Value::Num(arr[1])]),
                    );
                    return Ok(());
                }
            }
            match val {
                SampledF64::Num(x) => {
                    set_num_field(st, channel, *x);
                    Ok(())
                }
                _ => Err(format!("fold: channel '{channel}' value kind mismatch")),
            }
        }
    }
}

fn fold_plain_node(node: &mut Value, channel: &str, val: &SampledF64) -> Result<(), String> {
    // Transform channels fold into the node's transform op list:
    // an existing op of the same kind is replaced in place; a
    // missing op is appended (ops compose left-to-right, §5 —
    // appending is the only order-preserving choice that does not
    // rewrite authored staging; see the design note).
    let (op_key, op_val) = match (channel, val) {
        ("translate", SampledF64::Vec3(v)) => ("translate", Value::Arr(vec![
            Value::Num(v[0]),
            Value::Num(v[1]),
            Value::Num(v[2]),
        ])),
        ("scale", SampledF64::Vec3(v)) => ("scale", Value::Arr(vec![
            Value::Num(v[0]),
            Value::Num(v[1]),
            Value::Num(v[2]),
        ])),
        ("rotate.x", SampledF64::Num(x)) => ("rotate_x", Value::Num(*x)),
        ("rotate.y", SampledF64::Num(x)) => ("rotate_y", Value::Num(*x)),
        ("rotate.z", SampledF64::Num(x)) => ("rotate_z", Value::Num(*x)),
        _ => return Err(format!("fold: channel '{channel}' value kind mismatch")),
    };
    if !matches!(node.get("transform"), Some(Value::Arr(_))) {
        obj_set(node, "transform", Value::Arr(Vec::new()));
    }
    let ops = match node {
        Value::Obj(pairs) => &mut pairs
            .iter_mut()
            .find(|(k, _)| k == "transform")
            .unwrap()
            .1,
        _ => unreachable!(),
    };
    if let Value::Arr(items) = ops {
        for op in items.iter_mut() {
            if is_obj(op) && obj_has(op, op_key) {
                obj_set(op, op_key, op_val);
                return Ok(());
            }
        }
        items.push(Value::Obj(vec![(op_key.to_string(), op_val)]));
    }
    Ok(())
}

fn strip_clip_fields(v: &mut Value) {
    match v {
        Value::Obj(pairs) => {
            pairs.retain(|(k, _)| k != "clip" && k != "clipTime");
            for (_, x) in pairs.iter_mut() {
                strip_clip_fields(x);
            }
        }
        Value::Arr(items) => {
            for x in items.iter_mut() {
                strip_clip_fields(x);
            }
        }
        _ => {}
    }
}

fn assert_no_clip_residue(v: &Value, path: &str, in_clips: bool) -> Result<(), String> {
    match v {
        Value::Obj(pairs) => {
            for (k, x) in pairs {
                let child_in_clips = in_clips || (path.is_empty() && k == "clips");
                if !child_in_clips && (k == "clip" || k == "clipTime") {
                    return Err(format!("{path}: residual key '{k}' after clip evaluation"));
                }
                assert_no_clip_residue(x, &format!("{path}.{k}"), child_in_clips)?;
            }
        }
        Value::Arr(items) => {
            for (i, x) in items.iter().enumerate() {
                assert_no_clip_residue(x, &format!("{path}[{i}]"), in_clips)?;
            }
        }
        _ => {}
    }
    Ok(())
}

/// Evaluate every clip instance in the document at the file's
/// `time` and fold the results into the static fields (§15.6).
/// The `clips` block and every `clip`/`clipTime` field are
/// consumed. Call AFTER `expand_prefabs` (targets bind to
/// post-expansion names).
pub fn evaluate_clips(root: &mut Value) -> Result<(), String> {
    // Discover instances (document order: root blocks in pair
    // order, then nodes recursively).
    struct Inst {
        clip: String,
        ct: ClipTime,
        path: String,
    }
    let mut instances: Vec<Inst> = Vec::new();
    let mut has_constructs = root.get("clips").is_some();
    if let Value::Obj(pairs) = root.clone() {
        for (k, v) in &pairs {
            if RESERVED_TARGETS.contains(&k.as_str()) {
                if let Some(c) = v.get("clip") {
                    has_constructs = true;
                    let name = c
                        .as_str()
                        .ok_or_else(|| format!("{k}.clip: must be a clip name string"))?
                        .to_string();
                    let ct = ClipTime::parse(v.get("clipTime"), k)?;
                    instances.push(Inst { clip: name, ct, path: k.clone() });
                } else if v.get("clipTime").is_some() {
                    return Err(format!("{k}: 'clipTime' with no 'clip' — a time mapping with nothing to map"));
                }
            }
        }
    }
    fn collect_node_instances(v: &Value, path: &str, out: &mut Vec<Inst>) -> Result<(), String> {
        match v {
            Value::Obj(pairs) => {
                if let Some(c) = v.get("clip") {
                    let name = c
                        .as_str()
                        .ok_or_else(|| format!("{path}.clip: must be a clip name string"))?
                        .to_string();
                    let ct = ClipTime::parse(v.get("clipTime"), path)?;
                    out.push(Inst { clip: name, ct, path: path.to_string() });
                } else if v.get("clipTime").is_some() {
                    return Err(format!(
                        "{path}: 'clipTime' with no 'clip' — a time mapping with nothing to map"
                    ));
                }
                for (k, x) in pairs {
                    if k != "clips" {
                        collect_node_instances(x, &format!("{path}.{k}"), out)?;
                    }
                }
            }
            Value::Arr(items) => {
                for (i, x) in items.iter().enumerate() {
                    collect_node_instances(x, &format!("{path}[{i}]"), out)?;
                }
            }
            _ => {}
        }
        Ok(())
    }
    if let Some(nodes) = root.get("nodes").cloned() {
        let before = instances.len();
        collect_node_instances(&nodes, "nodes", &mut instances)?;
        if instances.len() > before {
            has_constructs = true;
        }
    }
    if !has_constructs {
        return Ok(());
    }
    if root.get("version").and_then(Value::as_f64) != Some(2.0) {
        return Err("clips extension rides format version 2 (spec §15)".to_string());
    }
    // Conflict rule: camera may not carry both clip and keys.
    if let Some(cam) = root.get("camera") {
        if cam.get("clip").is_some() && cam.get("keys").is_some() {
            return Err(
                "camera: carries both 'clip' and 'keys' — one camera, one time source; drop one"
                    .to_string(),
            );
        }
    }
    let node_kinds = collect_node_kinds(root)?;
    let set = ClipSet::from_root_with_kinds(root, &node_kinds)?;
    // Every instance must name a defined clip (ClipSet::sample
    // also errors; check up front for a named failure).
    for inst in &instances {
        if set.get(&inst.clip).is_none() {
            return Err(format!("{}: instance references unknown clip '{}'", inst.path, inst.clip));
        }
    }
    let t = root.get("time").and_then(Value::as_f64).unwrap_or(0.0);
    // Fold in document order; where two instances write the same
    // target+channel the later instance wins (the validators are
    // silent on cross-instance conflicts — see the design note).
    for inst in &instances {
        let def = set.get(&inst.clip).unwrap();
        let tau = map_time(&inst.ct, t, def.duration);
        for tr in set.layered_tracks(&inst.clip) {
            let val = sample_track_f64(&tr, tau);
            fold_value(root, &tr.node, &tr.channel, &val, &node_kinds)?;
        }
    }
    // Consume: strip instance fields everywhere, drop the block.
    strip_clip_fields(root);
    obj_remove(root, "clips");
    assert_no_clip_residue(root, "", false)?;
    Ok(())
}

/// R6 emission for a whole scene document: discovers the clip
/// instances exactly as `evaluate_clips` does (root reserved
/// blocks in pair order, then nodes recursively), and returns
/// their events in the R5 shape over the half-open film-time
/// window `[w0, w1)`, given the segment's absolute `segment_start`
/// (manifest `startS`). `root` must be the EXPANDED document
/// (post-`expand_prefabs`) carrying its `clips` block — call this
/// BEFORE `evaluate_clips` consumes the block, or on a fresh
/// parse. A document with no clips yields an empty vec. The
/// events are a SIDE CHANNEL: they are not written into the
/// document, and they must be routed to `clip-events.json`,
/// never merged into `cues.json` (CR-B1-05 §3).
pub fn scene_events(
    root: &Value,
    segment_start: f64,
    w0: f64,
    w1: f64,
) -> Result<Vec<EmittedEvent>, String> {
    if root.get("clips").is_none() {
        return Ok(Vec::new());
    }
    if root.get("version").and_then(Value::as_f64) != Some(2.0) {
        return Err("clips extension rides format version 2 (spec §15)".to_string());
    }
    let mut instances: Vec<(String, ClipTime)> = Vec::new();
    if let Value::Obj(pairs) = root {
        for (k, v) in pairs {
            if RESERVED_TARGETS.contains(&k.as_str()) {
                if let Some(c) = v.get("clip") {
                    let name = c
                        .as_str()
                        .ok_or_else(|| format!("{k}.clip: must be a clip name string"))?
                        .to_string();
                    instances.push((name, ClipTime::parse(v.get("clipTime"), k)?));
                } else if v.get("clipTime").is_some() {
                    return Err(format!(
                        "{k}: 'clipTime' with no 'clip' — a time mapping with nothing to map"
                    ));
                }
            }
        }
    }
    fn collect_node_instances(
        v: &Value,
        path: &str,
        out: &mut Vec<(String, ClipTime)>,
    ) -> Result<(), String> {
        match v {
            Value::Obj(pairs) => {
                if let Some(c) = v.get("clip") {
                    let name = c
                        .as_str()
                        .ok_or_else(|| format!("{path}.clip: must be a clip name string"))?
                        .to_string();
                    out.push((name, ClipTime::parse(v.get("clipTime"), path)?));
                } else if v.get("clipTime").is_some() {
                    return Err(format!(
                        "{path}: 'clipTime' with no 'clip' — a time mapping with nothing to map"
                    ));
                }
                for (k, x) in pairs {
                    if k != "clips" {
                        collect_node_instances(x, &format!("{path}.{k}"), out)?;
                    }
                }
            }
            Value::Arr(items) => {
                for (i, x) in items.iter().enumerate() {
                    collect_node_instances(x, &format!("{path}[{i}]"), out)?;
                }
            }
            _ => {}
        }
        Ok(())
    }
    if let Some(nodes) = root.get("nodes") {
        collect_node_instances(nodes, "nodes", &mut instances)?;
    }
    let set = ClipSet::from_root(root)?;
    let refs: Vec<(&str, ClipTime)> =
        instances.iter().map(|(n, ct)| (n.as_str(), *ct)).collect();
    set.collect_events(&refs, segment_start, w0, w1)
}

// =====================================================================
// Orchestration (§15.10 step 2)
// =====================================================================

/// Process a parsed scene document end to end: duplicate-key
/// scan (only when §15 constructs are present), prefab expansion,
/// then clip evaluation at the file's `time`. Returns the fully
/// processed ordinary v2 document.
pub fn process_scene(root: Value, libraries: &[Value]) -> Result<Value, String> {
    let mut root = root;
    let uses_15 = root.get("prefabs").is_some()
        || root.get("clips").is_some()
        || libraries.iter().any(|l| l.get("prefabs").is_some())
        || root.get("nodes").map(|n| {
            let mut insts = Vec::new();
            collect_instances(n, &mut insts);
            !insts.is_empty()
        }).unwrap_or(false);
    if uses_15 {
        let mut dups = Vec::new();
        find_duplicate_keys(&root, "scene", &mut dups);
        if let Some(first) = dups.into_iter().next() {
            return Err(first);
        }
    }
    expand_prefabs(&mut root, libraries)?;
    evaluate_clips(&mut root)?;
    Ok(root)
}

// =====================================================================
// Tests — the semantics the Python checkers enforce, as loader
// tests (spec §15.10 step 2: "the fixture sets become loader
// tests"). Fixture-shaped documents are built programmatically
// (this crate's Value has no serialiser; the shapes mirror
// scenes/fixtures/prefabs/ and scenes/fixtures/clips/).
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn n(x: f64) -> Value {
        Value::Num(x)
    }
    fn s(x: &str) -> Value {
        Value::Str(x.to_string())
    }
    fn a(items: Vec<Value>) -> Value {
        Value::Arr(items)
    }
    fn o(pairs: Vec<(&str, Value)>) -> Value {
        Value::Obj(pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
    }
    fn slot(name: &str) -> Value {
        o(vec![("$param", s(name))])
    }
    fn base_doc(nodes: Value) -> Value {
        o(vec![
            ("format", s("talerender-scene")),
            ("version", n(2.0)),
            ("name", s("eval15 test")),
            ("camera", o(vec![("fovY", n(44.0)), ("eye", a(vec![n(0.0), n(2.0), n(5.0)])), ("target", a(vec![n(0.0), n(1.0), n(0.0)]))])),
            ("nodes", nodes),
        ])
    }

    /// The marker-post library (scenes/fixtures/prefabs/
    /// library.prefabs.json), rebuilt as a Value.
    fn marker_library() -> Value {
        let post = o(vec![
            ("type", s("mesh")),
            ("geometry", o(vec![("kind", s("box")), ("size", a(vec![n(0.12), slot("height"), n(0.12)]))])),
            ("material", o(vec![("color", slot("postColor"))])),
            ("transform", a(vec![o(vec![("translate", a(vec![n(0.0), slot("postY"), n(0.0)]))])])),
        ]);
        let cap = o(vec![
            ("type", s("mesh")),
            ("pid", s("cap")),
            ("geometry", o(vec![("kind", s("box")), ("size", a(vec![n(0.2), n(0.08), n(0.2)]))])),
            ("material", o(vec![("color", slot("capColor")), ("roughness", n(0.8))])),
            ("transform", a(vec![o(vec![("translate", a(vec![n(0.0), slot("capY"), n(0.0)]))])])),
        ]);
        let marker_post = o(vec![
            ("params", o(vec![
                ("height", o(vec![("type", s("number")), ("required", Value::Bool(true))])),
                ("postY", o(vec![("type", s("number")), ("default", n(0.9))])),
                ("capY", o(vec![("type", s("number")), ("default", n(1.84))])),
                ("postColor", o(vec![("type", s("color")), ("default", s("#8A6B42"))])),
                ("capColor", o(vec![("type", s("color")), ("default", s("#C23B2E"))])),
            ])),
            ("node", o(vec![("type", s("group")), ("children", a(vec![post, cap]))])),
        ]);
        let inner = |x: f64| {
            o(vec![
                ("prefab", s("marker-post")),
                ("params", o(vec![
                    ("height", slot("height")),
                    ("capColor", slot("capColor")),
                ])),
                ("transform", a(vec![o(vec![("translate", a(vec![n(x), n(0.0), n(0.0)]))])])),
            ])
        };
        let post_pair = o(vec![
            ("params", o(vec![
                ("height", o(vec![("type", s("number")), ("required", Value::Bool(true))])),
                ("gap", o(vec![("type", s("number")), ("default", n(2.0))])),
                ("capColor", o(vec![("type", s("color")), ("default", s("#C23B2E"))])),
            ])),
            ("node", o(vec![("type", s("group")), ("children", a(vec![inner(0.0), inner(2.0)]))])),
        ]);
        o(vec![
            ("format", s("talerender-scene")),
            ("version", n(2.0)),
            ("prefabs", o(vec![("marker-post", marker_post), ("post-pair", post_pair)])),
        ])
    }

    // ---------------- prefabs ----------------

    #[test]
    fn prefab_expansion_substitutes_wraps_and_strips() {
        let inst = o(vec![
            ("prefab", s("marker-post")),
            ("params", o(vec![
                ("height", n(1.2)),
                ("postY", n(0.6)),
                ("capY", n(1.24)),
                ("capColor", s("#2E5FC2")),
            ])),
            ("overrides", o(vec![("cap", o(vec![("material", o(vec![("roughness", n(0.35))]))]))])),
            ("transform", a(vec![o(vec![("translate", a(vec![n(2.5), n(0.0), n(-2.0)]))])])),
            ("name", s("east gate post")),
        ]);
        let mut doc = base_doc(a(vec![inst]));
        expand_prefabs(&mut doc, &[marker_library()]).unwrap();
        assert!(doc.get("prefabs").is_none(), "prefabs block consumed");
        let nodes = doc.get("nodes").unwrap().as_arr().unwrap();
        assert_eq!(nodes.len(), 1);
        let wrapper = &nodes[0];
        assert_eq!(wrapper.get("type").and_then(Value::as_str), Some("group"));
        assert_eq!(wrapper.get("name").and_then(Value::as_str), Some("east gate post"));
        let root = &wrapper.get("children").unwrap().as_arr().unwrap()[0];
        let children = root.get("children").unwrap().as_arr().unwrap();
        // Post: slot in the middle of a size vector substituted.
        let post = &children[0];
        let size = post.get("geometry").unwrap().get("size").unwrap().as_arr().unwrap();
        assert_eq!(size[1], n(1.2));
        assert_eq!(
            post.get("material").unwrap().get("color").and_then(Value::as_str),
            Some("#8A6B42"),
            "default postColor applied"
        );
        // Cap: override deep-merged roughness, slot colour intact,
        // pid stripped.
        let cap = &children[1];
        assert!(cap.get("pid").is_none());
        let mat = cap.get("material").unwrap();
        assert_eq!(mat.get("roughness"), Some(&n(0.35)));
        assert_eq!(mat.get("color").and_then(Value::as_str), Some("#2E5FC2"));
    }

    #[test]
    fn prefab_defaults_and_no_wrapper_without_transform() {
        let inst = o(vec![
            ("prefab", s("marker-post")),
            ("params", o(vec![("height", n(0.9)), ("postY", n(0.45)), ("capY", n(0.94))])),
        ]);
        let mut doc = base_doc(a(vec![inst]));
        expand_prefabs(&mut doc, &[marker_library()]).unwrap();
        let nodes = doc.get("nodes").unwrap().as_arr().unwrap();
        // No instance transform -> the template root itself, no
        // wrapper group around it (its type is group already, so
        // assert it has children directly and no nested wrapper).
        assert!(nodes[0].get("children").is_some());
        assert!(nodes[0].get("transform").is_none());
    }

    #[test]
    fn prefab_nested_forwarding_and_widening() {
        // post-pair forwards height + capColor (color -> color)
        // into two marker-posts; the second carries a transform
        // slot? No — transforms are literal; gap is declared but
        // the fixture's inner transforms are literal too. The
        // forwarding under test is params-via-slots.
        let inst = o(vec![
            ("prefab", s("post-pair")),
            ("params", o(vec![("height", n(1.8)), ("gap", n(2.5))])),
        ]);
        let mut doc = base_doc(a(vec![inst]));
        expand_prefabs(&mut doc, &[marker_library()]).unwrap();
        let nodes = doc.get("nodes").unwrap().as_arr().unwrap();
        let pair_children = nodes[0].get("children").unwrap().as_arr().unwrap();
        assert_eq!(pair_children.len(), 2);
        // First inner instance: the fixture's inner instances
        // carry literal transforms, so each expands to a wrapper
        // group holding the marker-post group; the post mesh is
        // that group's first child.
        let first = &pair_children[0];
        let post_group = &first.get("children").unwrap().as_arr().unwrap()[0];
        let post = &post_group.get("children").unwrap().as_arr().unwrap()[0];
        let size = post.get("geometry").unwrap().get("size").unwrap().as_arr().unwrap();
        assert_eq!(size[1], n(1.8), "forwarded height reached the nested template");
    }

    #[test]
    fn prefab_forwarding_type_mismatch_fails() {
        // Outer declares `height` as string, inner wants number.
        let lib = o(vec![
            ("format", s("talerender-scene")),
            ("version", n(2.0)),
            ("prefabs", o(vec![
                ("inner", o(vec![
                    ("params", o(vec![("v", o(vec![("type", s("number")), ("required", Value::Bool(true))]))])),
                    ("node", o(vec![("type", s("mesh")), ("geometry", o(vec![("kind", s("box")), ("size", a(vec![slot("v"), n(1.0), n(1.0)]))]))])),
                ])),
                ("outer", o(vec![
                    ("params", o(vec![("v", o(vec![("type", s("string")), ("required", Value::Bool(true))]))])),
                    ("node", o(vec![
                        ("type", s("group")),
                        ("children", a(vec![o(vec![
                            ("prefab", s("inner")),
                            ("params", o(vec![("v", slot("v"))])),
                        ])])),
                    ])),
                ])),
            ])),
        ]);
        let mut doc = base_doc(a(vec![o(vec![
            ("prefab", s("outer")),
            ("params", o(vec![("v", s("x"))])),
        ])]));
        let e = expand_prefabs(&mut doc, &[lib]).unwrap_err();
        assert!(e.contains("forwarded slot"), "{e}");
    }

    #[test]
    fn prefab_color_widens_to_string_but_not_back() {
        // color -> string is the one widening (§15.3).
        let mk = |outer_ty: &str, inner_ty: &str| {
            o(vec![
                ("format", s("talerender-scene")),
                ("version", n(2.0)),
                ("prefabs", o(vec![
                    ("inner", o(vec![
                        ("params", o(vec![("v", o(vec![("type", s(inner_ty)), ("required", Value::Bool(true))]))])),
                        ("node", o(vec![("type", s("mesh")), ("geometry", o(vec![("kind", s("box")), ("size", a(vec![n(1.0), n(1.0), n(1.0)]))])), ("material", o(vec![("color", slot("v"))]))])),
                    ])),
                    ("outer", o(vec![
                        ("params", o(vec![("v", o(vec![("type", s(outer_ty)), ("required", Value::Bool(true))]))])),
                        ("node", o(vec![
                            ("type", s("group")),
                            ("children", a(vec![o(vec![
                                ("prefab", s("inner")),
                                ("params", o(vec![("v", slot("v"))])),
                            ])])),
                        ])),
                    ])),
                ])),
            ])
        };
        let mut doc = base_doc(a(vec![o(vec![
            ("prefab", s("outer")),
            ("params", o(vec![("v", s("#112233"))])),
        ])]));
        expand_prefabs(&mut doc, &[mk("color", "string")]).unwrap();
        let mut doc2 = base_doc(a(vec![o(vec![
            ("prefab", s("outer")),
            ("params", o(vec![("v", s("hello"))])),
        ])]));
        assert!(expand_prefabs(&mut doc2, &[mk("string", "color")]).is_err());
    }

    #[test]
    fn prefab_cycle_error_prints_path() {
        let lib = o(vec![
            ("format", s("talerender-scene")),
            ("version", n(2.0)),
            ("prefabs", o(vec![
                ("loop-a", o(vec![("node", o(vec![("type", s("group")), ("children", a(vec![o(vec![("prefab", s("loop-b"))])]))]))])),
                ("loop-b", o(vec![("node", o(vec![("type", s("group")), ("children", a(vec![o(vec![("prefab", s("loop-a"))])]))]))])),
            ])),
        ]);
        let mut doc = base_doc(a(vec![o(vec![("prefab", s("loop-a"))])]));
        let e = expand_prefabs(&mut doc, &[lib]).unwrap_err();
        assert!(e.contains("loop-a -> loop-b -> loop-a"), "{e}");
    }

    #[test]
    fn prefab_duplicate_definition_is_an_error() {
        let mut doc = base_doc(a(vec![]));
        obj_set(&mut doc, "prefabs", o(vec![
            ("marker-post", o(vec![("node", o(vec![("type", s("mesh"))]))])),
        ]));
        let e = expand_prefabs(&mut doc, &[marker_library()]).unwrap_err();
        assert!(e.contains("defined by both"), "{e}");
    }

    #[test]
    fn prefab_instance_errors() {
        // missing required
        let mut doc = base_doc(a(vec![o(vec![("prefab", s("marker-post")), ("params", o(vec![]))])]));
        assert!(expand_prefabs(&mut doc, &[marker_library()]).unwrap_err().contains("missing required param 'height'"));
        // undeclared param
        let mut doc = base_doc(a(vec![o(vec![
            ("prefab", s("marker-post")),
            ("params", o(vec![("height", n(1.0)), ("weight", n(3.0))])),
        ])]));
        assert!(expand_prefabs(&mut doc, &[marker_library()]).unwrap_err().contains("declares no param 'weight'"));
        // param type: string for number
        let mut doc = base_doc(a(vec![o(vec![
            ("prefab", s("marker-post")),
            ("params", o(vec![("height", s("tall"))])),
        ])]));
        assert!(expand_prefabs(&mut doc, &[marker_library()]).unwrap_err().contains("declared type 'number'"));
        // unknown prefab
        let mut doc = base_doc(a(vec![o(vec![("prefab", s("ghost-fence"))])]));
        assert!(expand_prefabs(&mut doc, &[marker_library()]).unwrap_err().contains("unknown prefab 'ghost-fence'"));
        // closed key set
        let mut doc = base_doc(a(vec![o(vec![
            ("prefab", s("marker-post")),
            ("params", o(vec![("height", n(1.0))])),
            ("bogus", n(1.0)),
        ])]));
        assert!(expand_prefabs(&mut doc, &[marker_library()]).unwrap_err().contains("unknown instance key 'bogus'"));
        // boolean is not a number
        let mut doc = base_doc(a(vec![o(vec![
            ("prefab", s("marker-post")),
            ("params", o(vec![("height", Value::Bool(true))])),
        ])]));
        assert!(expand_prefabs(&mut doc, &[marker_library()]).is_err());
    }

    #[test]
    fn prefab_definition_errors() {
        let bad_lib = |defn: Value| {
            o(vec![
                ("format", s("talerender-scene")),
                ("version", n(2.0)),
                ("prefabs", o(vec![("p", defn)])),
            ])
        };
        let node = || o(vec![("type", s("mesh")), ("geometry", o(vec![("kind", s("box")), ("size", a(vec![n(1.0), n(1.0), n(1.0)]))]))]);
        // required + default
        let d = o(vec![
            ("params", o(vec![("v", o(vec![("type", s("number")), ("required", Value::Bool(true)), ("default", n(1.0))]))])),
            ("node", node()),
        ]);
        let mut doc = base_doc(a(vec![]));
        assert!(expand_prefabs(&mut doc, &[bad_lib(d)]).unwrap_err().contains("may not carry a default"));
        // neither required nor default
        let d = o(vec![
            ("params", o(vec![("v", o(vec![("type", s("number"))]))])),
            ("node", node()),
        ]);
        let mut doc = base_doc(a(vec![]));
        assert!(expand_prefabs(&mut doc, &[bad_lib(d)]).unwrap_err().contains("neither required nor carries a default"));
        // default containing a slot
        let d = o(vec![
            ("params", o(vec![
                ("v", o(vec![("type", s("number")), ("default", n(1.0))])),
                ("w", o(vec![("type", s("number")), ("default", slot("v"))])),
            ])),
            ("node", node()),
        ]);
        let mut doc = base_doc(a(vec![]));
        assert!(expand_prefabs(&mut doc, &[bad_lib(d)]).unwrap_err().contains("defaults must be literal"));
        // undeclared slot in template
        let d = o(vec![
            ("params", o(vec![("height", o(vec![("type", s("number")), ("required", Value::Bool(true))]))])),
            ("node", o(vec![("type", s("mesh")), ("geometry", o(vec![("kind", s("box")), ("size", a(vec![n(0.12), slot("widht"), n(0.12)]))]))])),
        ]);
        let mut doc = base_doc(a(vec![]));
        assert!(expand_prefabs(&mut doc, &[bad_lib(d)]).unwrap_err().contains("not declared by prefab 'p'"));
        // duplicate pid in direct scope
        let kid = |pid: &str| o(vec![("type", s("mesh")), ("pid", s(pid)), ("geometry", o(vec![("kind", s("box")), ("size", a(vec![n(1.0), n(1.0), n(1.0)]))]))]);
        let d = o(vec![("node", o(vec![("type", s("group")), ("children", a(vec![kid("x"), kid("x")]))]))]);
        let mut doc = base_doc(a(vec![]));
        assert!(expand_prefabs(&mut doc, &[bad_lib(d)]).unwrap_err().contains("pid 'x' declared 2 times"));
    }

    #[test]
    fn prefab_override_errors() {
        let with_override = |ov: Value| {
            base_doc(a(vec![o(vec![
                ("prefab", s("marker-post")),
                ("params", o(vec![("height", n(1.0))])),
                ("overrides", ov),
            ])]))
        };
        // forbidden: type
        let mut doc = with_override(o(vec![("cap", o(vec![("type", s("group"))]))]));
        assert!(expand_prefabs(&mut doc, &[marker_library()]).unwrap_err().contains("forbidden key 'type'"));
        // forbidden: transform
        let mut doc = with_override(o(vec![("cap", o(vec![("transform", a(vec![]))]))]));
        assert!(expand_prefabs(&mut doc, &[marker_library()]).unwrap_err().contains("forbidden key 'transform'"));
        // unknown pid
        let mut doc = with_override(o(vec![("roof", o(vec![("material", o(vec![("roughness", n(0.1))]))]))]));
        assert!(expand_prefabs(&mut doc, &[marker_library()]).unwrap_err().contains("no pid 'roof'"));
        // null deletion
        let mut doc = with_override(o(vec![("cap", o(vec![("material", Value::Null)]))]));
        assert!(expand_prefabs(&mut doc, &[marker_library()]).unwrap_err().contains("null"));
        // through a slot: cap's material.color is a $param slot.
        let mut doc = with_override(o(vec![("cap", o(vec![("material", o(vec![("color", s("#000000"))]))]))]));
        assert!(expand_prefabs(&mut doc, &[marker_library()]).unwrap_err().contains("targets a $param slot"));
    }

    #[test]
    fn prefab_override_arrays_replace_wholesale() {
        // Override a size vector: replaced, never merged by index.
        let lib = o(vec![
            ("format", s("talerender-scene")),
            ("version", n(2.0)),
            ("prefabs", o(vec![("blk", o(vec![
                ("node", o(vec![
                    ("type", s("mesh")),
                    ("pid", s("root")),
                    ("geometry", o(vec![("kind", s("box")), ("size", a(vec![n(1.0), n(2.0), n(3.0)]))])),
                    ("material", o(vec![("color", s("#112233"))])),
                ])),
            ]))])),
        ]);
        let mut doc = base_doc(a(vec![o(vec![
            ("prefab", s("blk")),
            ("overrides", o(vec![("root", o(vec![("geometry", o(vec![("size", a(vec![n(9.0)]))]))]))])),
        ])]));
        expand_prefabs(&mut doc, &[lib]).unwrap();
        let nodes = doc.get("nodes").unwrap().as_arr().unwrap();
        let size = nodes[0].get("geometry").unwrap().get("size").unwrap().as_arr().unwrap();
        assert_eq!(size.len(), 1, "array replaced wholesale");
        assert_eq!(size[0], n(9.0));
    }

    #[test]
    fn prefab_object_param_replaces_wholesale() {
        // A supplied object param is NOT merged into its default.
        let lib = o(vec![
            ("format", s("talerender-scene")),
            ("version", n(2.0)),
            ("prefabs", o(vec![("mat-mesh", o(vec![
                ("params", o(vec![("mat", o(vec![
                    ("type", s("object")),
                    ("default", o(vec![("color", s("#111111")), ("roughness", n(0.8))])),
                ]))])),
                ("node", o(vec![
                    ("type", s("mesh")),
                    ("geometry", o(vec![("kind", s("box")), ("size", a(vec![n(1.0), n(1.0), n(1.0)]))])),
                    ("material", slot("mat")),
                ])),
            ]))])),
        ]);
        let mut doc = base_doc(a(vec![o(vec![
            ("prefab", s("mat-mesh")),
            ("params", o(vec![("mat", o(vec![("color", s("#222222"))]))])),
        ])]));
        expand_prefabs(&mut doc, &[lib]).unwrap();
        let nodes = doc.get("nodes").unwrap().as_arr().unwrap();
        let mat = nodes[0].get("material").unwrap();
        assert_eq!(mat.get("color").and_then(Value::as_str), Some("#222222"));
        assert!(mat.get("roughness").is_none(), "no field-by-field merge of object params");
    }

    #[test]
    fn prefab_node_param_injects_and_expands() {
        // A `node` param carrying an instance: injected, then
        // recursively expanded (§15.5 step 5e).
        let lib = o(vec![
            ("format", s("talerender-scene")),
            ("version", n(2.0)),
            ("prefabs", o(vec![
                ("wrap", o(vec![
                    ("params", o(vec![("kid", o(vec![("type", s("node")), ("required", Value::Bool(true))]))])),
                    ("node", o(vec![("type", s("group")), ("children", a(vec![slot("kid")]))])),
                ])),
                ("dot", o(vec![
                    ("node", o(vec![("type", s("mesh")), ("geometry", o(vec![("kind", s("disc")), ("radius", n(1.0)), ("segments", n(8.0))])), ("material", o(vec![("color", s("#445566"))]))])),
                ])),
            ])),
        ]);
        let mut doc = base_doc(a(vec![o(vec![
            ("prefab", s("wrap")),
            ("params", o(vec![("kid", o(vec![("prefab", s("dot"))]))])),
        ])]));
        expand_prefabs(&mut doc, &[lib]).unwrap();
        let nodes = doc.get("nodes").unwrap().as_arr().unwrap();
        let kid = &nodes[0].get("children").unwrap().as_arr().unwrap()[0];
        assert_eq!(kid.get("type").and_then(Value::as_str), Some("mesh"));
    }

    #[test]
    fn prefab_slot_in_scene_outside_template_fails() {
        let mut doc = base_doc(a(vec![o(vec![
            ("type", s("mesh")),
            ("geometry", o(vec![("kind", s("box")), ("size", a(vec![n(1.0), slot("h"), n(1.0)]))])),
        ])]));
        assert!(expand_prefabs(&mut doc, &[]).is_err());
    }

    #[test]
    fn prefab_depth_guard_fails_closed() {
        // A 70-deep acyclic chain: the graph check passes, the
        // depth guard must fail rather than overflow (§15.5).
        let mut prefabs: Vec<(&str, Value)> = Vec::new();
        let names: Vec<String> = (0..70).map(|i| format!("p{i}")).collect();
        for (i, name) in names.iter().enumerate() {
            let node = if i + 1 < names.len() {
                o(vec![("type", s("group")), ("children", a(vec![o(vec![("prefab", s(&names[i + 1]))])]))])
            } else {
                o(vec![("type", s("mesh")), ("geometry", o(vec![("kind", s("box")), ("size", a(vec![n(1.0), n(1.0), n(1.0)]))]))])
            };
            prefabs.push((Box::leak(name.clone().into_boxed_str()), o(vec![("node", node)])));
        }
        let lib = o(vec![
            ("format", s("talerender-scene")),
            ("version", n(2.0)),
            ("prefabs", Value::Obj(prefabs.into_iter().map(|(k, v)| (k.to_string(), v)).collect())),
        ]);
        let mut doc = base_doc(a(vec![o(vec![("prefab", s("p0"))])]));
        let e = expand_prefabs(&mut doc, &[lib]).unwrap_err();
        assert!(e.contains("expansion depth exceeds"), "{e}");
    }

    // ---------------- clips ----------------

    fn key(t: f64, v: Value) -> Value {
        o(vec![("t", n(t)), ("value", v)])
    }
    fn keye(t: f64, v: Value, e: &str) -> Value {
        o(vec![("t", n(t)), ("value", v), ("ease", s(e))])
    }
    fn track(node: &str, channel: &str, keys: Value) -> Value {
        o(vec![
            ("target", o(vec![("node", s(node)), ("channel", s(channel))])),
            ("keys", keys),
        ])
    }
    fn char_node(name: &str) -> Value {
        o(vec![
            ("type", s("character")),
            ("name", s(name)),
            ("position", a(vec![n(0.0), n(0.0), n(0.0)])),
            ("height", n(1.75)),
            ("state", o(vec![
                ("mood", s("determined")),
                ("armL", a(vec![n(10.0), n(6.0)])),
                ("armR", a(vec![n(16.0), n(-6.0)])),
                ("headTurn", n(0.0)),
                ("lean", n(0.0)),
            ])),
        ])
    }

    #[test]
    fn clip_fold_camera_character_and_consumption() {
        let clips = o(vec![("push-in", o(vec![
            ("duration", n(5.907)),
            ("tracks", a(vec![
                track("camera", "eye", a(vec![key(0.0, a(vec![n(0.0), n(2.0), n(5.0)])), key(5.907, a(vec![n(0.0), n(1.0), n(3.0)]))])),
                track("camera", "fovY", a(vec![key(0.0, n(44.0)), keye(5.907, n(36.0), "linear")])),
            ])),
        ])), ("look", o(vec![
            ("duration", n(5.2)),
            ("tracks", a(vec![
                track("Onoda", "headTurn", a(vec![key(0.0, n(-0.45)), key(2.6, n(0.4)), key(5.2, n(-0.2))])),
                track("Onoda", "mood", a(vec![keye(0.0, s("determined"), "step"), keye(2.6, s("neutral"), "step")])),
            ])),
        ]))]);
        let mut cam = o(vec![
            ("fovY", n(44.0)),
            ("eye", a(vec![n(0.0), n(2.0), n(5.0)])),
            ("target", a(vec![n(0.0), n(1.0), n(0.0)])),
            ("clip", s("push-in")),
        ]);
        let mut onoda = char_node("Onoda");
        obj_set(&mut onoda, "clip", s("look"));
        obj_set(&mut onoda, "clipTime", o(vec![("offset", n(0.0)), ("rate", n(1.0)), ("loop", s("clamp"))]));
        let mut doc = base_doc(a(vec![onoda]));
        obj_set(&mut doc, "camera", {
            let mut c = o(vec![]);
            for (k, v) in [("fovY", n(44.0)), ("eye", a(vec![n(0.0), n(2.0), n(5.0)])), ("target", a(vec![n(0.0), n(1.0), n(0.0)]))] {
                obj_set(&mut c, k, v);
            }
            obj_set(&mut c, "clip", s("push-in"));
            c
        });
        let _ = &mut cam;
        obj_set(&mut doc, "clips", clips);
        obj_set(&mut doc, "time", n(2.6));
        evaluate_clips(&mut doc).unwrap();
        assert!(doc.get("clips").is_none(), "clips block consumed");
        let cam = doc.get("camera").unwrap();
        assert!(cam.get("clip").is_none());
        // eye at tau 2.6: smoother midpoint-ish of the segment.
        let eye = cam.get("eye").unwrap().as_arr().unwrap();
        assert!(eye[1].as_f64().unwrap() > 1.0 && eye[1].as_f64().unwrap() < 2.0);
        // fovY linear: 44 + (36-44) * (2.6/5.907)
        let fov = cam.get("fovY").unwrap().as_f64().unwrap();
        assert!((fov - (44.0 - 8.0 * (2.6 / 5.907))).abs() < 1e-9, "{fov}");
        let node = &doc.get("nodes").unwrap().as_arr().unwrap()[0];
        assert!(node.get("clip").is_none() && node.get("clipTime").is_none());
        let st = node.get("state").unwrap();
        assert_eq!(st.get("headTurn"), Some(&n(0.4)), "at the key instant, the key value");
        assert_eq!(st.get("mood").and_then(Value::as_str), Some("neutral"), "step fired at 2.6");
    }

    #[test]
    fn clip_wrap_modes_and_time_mapping() {
        let clamp = ClipTime::default();
        assert_eq!(map_time(&clamp, 7.0, 5.0), 5.0);
        assert_eq!(map_time(&clamp, -1.0, 5.0), 0.0);
        let lp = ClipTime { offset: 0.0, rate: 1.0, wrap: WrapMode::Loop };
        assert_eq!(map_time(&lp, 7.0, 5.0), 2.0);
        assert_eq!(map_time(&lp, -1.0, 5.0), 4.0);
        let pp = ClipTime { offset: 0.0, rate: 1.0, wrap: WrapMode::PingPong };
        assert_eq!(map_time(&pp, 7.0, 5.0), 3.0);
        assert_eq!(map_time(&pp, 12.0, 5.0), 2.0);
        assert_eq!(map_time(&pp, -1.0, 5.0), 1.0);
        // rate + offset compose before wrapping.
        let ro = ClipTime { offset: 0.5, rate: 2.0, wrap: WrapMode::Clamp };
        assert_eq!(map_time(&ro, 1.0, 5.0), 2.5);
    }

    #[test]
    fn clip_ease_is_on_the_arriving_key() {
        // Segment [0,2]: arriving key says linear. If ease were
        // read from the leaving key (smoother default), the
        // midpoint would still be 0.5 — so probe at u=0.25:
        // linear gives 0.25, smootherstep gives ~0.1035.
        let keys = vec![
            ScalarKey { t: 0.0, value: 0.0, ease: "smoother".to_string() },
            ScalarKey { t: 2.0, value: 1.0, ease: "linear".to_string() },
        ];
        assert!((sample_scalar_keys(&keys, 0.5) - 0.25).abs() < 1e-12);
        // First key's ease is ignored (no incoming segment).
        let keys2 = vec![
            ScalarKey { t: 0.0, value: 0.0, ease: "linear".to_string() },
            ScalarKey { t: 2.0, value: 1.0, ease: "smoother".to_string() },
        ];
        let v = sample_scalar_keys(&keys2, 0.5);
        assert!((v - 0.103515625).abs() < 1e-9, "smootherstep(0.25) = {v}");
    }

    #[test]
    fn clip_colour_lerps_in_linear() {
        let tr = TrackDef {
            node: "lights".to_string(),
            channel: "key.color".to_string(),
            kind: ChanKind::Color,
            keys: vec![
                (0.0, KeyVal::Color("#000000".to_string()), "smoother".to_string()),
                (2.0, KeyVal::Color("#FFFFFF".to_string()), "linear".to_string()),
            ],
        };
        match sample_track_f64(&tr, 1.0) {
            SampledF64::ColorLin(l) => {
                assert!((l[0] - 0.5).abs() < 1e-12, "linear midpoint is 0.5 in linear space");
                assert_eq!(lin_to_hex(l), "#BCBCBC", "0.5 linear == sRGB 188, not 128");
            }
            _ => panic!("colour lane expected"),
        }
        // Hex round-trip is exact for authored values.
        assert_eq!(lin_to_hex(hex_to_lin_f64("#2E5FC2")), "#2E5FC2");
    }

    #[test]
    fn clip_step_holds_and_fires() {
        let tr = TrackDef {
            node: "Onoda".to_string(),
            channel: "mood".to_string(),
            kind: ChanKind::StepMood,
            keys: vec![
                (1.0, KeyVal::Step("sad".to_string()), "step".to_string()),
                (3.0, KeyVal::Step("happy".to_string()), "step".to_string()),
            ],
        };
        let at = |tau: f64| match sample_track_f64(&tr, tau) {
            SampledF64::Step(s) => s,
            _ => panic!("step lane"),
        };
        assert_eq!(at(0.0), "sad", "before the first key, hold it");
        assert_eq!(at(2.999), "sad");
        assert_eq!(at(3.0), "happy");
        assert_eq!(at(99.0), "happy");
    }

    #[test]
    fn clip_validation_errors() {
        let doc_with = |clips: Value, nodes: Value| {
            let mut d = base_doc(nodes);
            obj_set(&mut d, "clips", clips);
            d
        };
        let onoda = || a(vec![char_node("Onoda")]);
        // R3: grade.time rejected by name.
        let clips = o(vec![("c", o(vec![("duration", n(2.0)), ("tracks", a(vec![track("post", "grade.time", a(vec![key(0.0, n(0.0))]))]))]))]);
        let e = ClipSet::from_root(&doc_with(clips, a(vec![]))).unwrap_err();
        assert!(e.contains("R3"), "{e}");
        // unknown channel
        let clips = o(vec![("c", o(vec![("duration", n(2.0)), ("tracks", a(vec![track("camera", "zoom", a(vec![key(0.0, n(44.0))]))]))]))]);
        assert!(ClipSet::from_root(&doc_with(clips, a(vec![]))).unwrap_err().contains("unknown channel 'camera.zoom'"));
        // unknown easing
        let clips = o(vec![("c", o(vec![("duration", n(2.0)), ("tracks", a(vec![track("Onoda", "headTurn", a(vec![key(0.0, n(0.0)), keye(2.0, n(1.0), "bounce")]))]))]))]);
        assert!(ClipSet::from_root(&doc_with(clips, onoda())).unwrap_err().contains("unknown easing 'bounce'"));
        // duplicate key time
        let clips = o(vec![("c", o(vec![("duration", n(5.0)), ("tracks", a(vec![track("Onoda", "headTurn", a(vec![key(0.0, n(0.0)), key(2.6, n(0.4)), key(2.6, n(-0.2))]))]))]))]);
        assert!(ClipSet::from_root(&doc_with(clips, onoda())).unwrap_err().contains("duplicate key time"));
        // unsorted
        let clips = o(vec![("c", o(vec![("duration", n(5.0)), ("tracks", a(vec![track("Onoda", "headTurn", a(vec![key(0.0, n(0.0)), key(2.6, n(0.4)), key(1.3, n(0.1))]))]))]))]);
        assert!(ClipSet::from_root(&doc_with(clips, onoda())).unwrap_err().contains("out of order"));
        // key beyond duration
        let clips = o(vec![("c", o(vec![("duration", n(2.0)), ("tracks", a(vec![track("Onoda", "headTurn", a(vec![key(0.0, n(0.0)), key(2.5, n(0.4))]))]))]))]);
        assert!(ClipSet::from_root(&doc_with(clips, onoda())).unwrap_err().contains("beyond the clip duration"));
        // bad mood value (strict — no neutral fallback)
        let clips = o(vec![("c", o(vec![("duration", n(2.0)), ("tracks", a(vec![track("Onoda", "mood", a(vec![keye(0.0, s("gleeful"), "step")]))]))]))]);
        assert!(ClipSet::from_root(&doc_with(clips, onoda())).is_err());
        // step channel with non-step ease
        let clips = o(vec![("c", o(vec![("duration", n(2.0)), ("tracks", a(vec![track("Onoda", "mood", a(vec![key(0.0, s("sad"))]))]))]))]);
        assert!(ClipSet::from_root(&doc_with(clips, onoda())).unwrap_err().contains("must use ease 'step'"));
        // duplicate track in one clip
        let clips = o(vec![("c", o(vec![("duration", n(2.0)), ("tracks", a(vec![
            track("Onoda", "lean", a(vec![key(0.0, n(0.0))])),
            track("Onoda", "lean", a(vec![key(0.0, n(1.0))])),
        ]))]))]);
        assert!(ClipSet::from_root(&doc_with(clips, onoda())).unwrap_err().contains("duplicate track"));
        // reserved-name collision
        let mut bad_node = char_node("camera");
        let _ = &mut bad_node;
        let clips = o(vec![("c", o(vec![("duration", n(2.0)), ("tracks", a(vec![track("camera", "eye", a(vec![key(0.0, a(vec![n(0.0), n(0.0), n(0.0)]))]))]))]))]);
        assert!(ClipSet::from_root(&doc_with(clips, a(vec![bad_node]))).unwrap_err().contains("collides with the reserved clip target"));
    }

    #[test]
    fn clip_uses_layering_and_cycles() {
        let onoda = || a(vec![char_node("Onoda")]);
        // Layered conflict: same channel via uses.
        let clips = o(vec![
            ("base", o(vec![("duration", n(2.0)), ("tracks", a(vec![track("Onoda", "lean", a(vec![key(0.0, n(0.0)), key(2.0, n(1.0))]))]))])),
            ("top", o(vec![
                ("duration", n(2.0)),
                ("uses", a(vec![s("base")])),
                ("tracks", a(vec![track("Onoda", "lean", a(vec![key(0.0, n(5.0))]))])),
            ])),
        ]);
        let mut d = base_doc(onoda());
        obj_set(&mut d, "clips", clips);
        assert!(ClipSet::from_root(&d).unwrap_err().contains("may not fight over one channel"));
        // uses cycle (self).
        let clips = o(vec![("selfy", o(vec![
            ("duration", n(2.0)),
            ("uses", a(vec![s("selfy")])),
            ("tracks", a(vec![track("Onoda", "lean", a(vec![key(0.0, n(0.0))]))])),
        ]))]);
        let mut d = base_doc(onoda());
        obj_set(&mut d, "clips", clips);
        assert!(ClipSet::from_root(&d).unwrap_err().contains("reference cycle: selfy -> selfy"));
        // unknown uses target.
        let clips = o(vec![("c", o(vec![
            ("duration", n(2.0)),
            ("uses", a(vec![s("nope")])),
            ("tracks", a(vec![track("Onoda", "lean", a(vec![key(0.0, n(0.0))]))])),
        ]))]);
        let mut d = base_doc(onoda());
        obj_set(&mut d, "clips", clips);
        assert!(ClipSet::from_root(&d).unwrap_err().contains("unknown clip 'nope'"));
        // Happy layering samples both clips' tracks.
        let clips = o(vec![
            ("base", o(vec![("duration", n(2.0)), ("tracks", a(vec![track("Onoda", "lean", a(vec![key(0.0, n(0.0)), keye(2.0, n(2.0), "linear")]))]))])),
            ("top", o(vec![
                ("duration", n(2.0)),
                ("uses", a(vec![s("base")])),
                ("tracks", a(vec![track("Onoda", "bob", a(vec![key(0.0, n(1.0))]))])),
            ])),
        ]);
        let mut d = base_doc(onoda());
        obj_set(&mut d, "clips", clips);
        let set = ClipSet::from_root(&d).unwrap();
        let out = set.sample("top", &ClipTime::default(), 1.0).unwrap();
        assert_eq!(out.len(), 2);
        assert!(out.iter().any(|(nd, ch, v)| nd == "Onoda" && ch == "lean" && *v == Sampled::Scalar(1.0)));
    }

    #[test]
    fn clip_instance_errors() {
        // camera clip + keys conflict
        let mut d = base_doc(a(vec![]));
        obj_set(&mut d, "clips", o(vec![("c", o(vec![("duration", n(1.0)), ("tracks", a(vec![track("camera", "fovY", a(vec![key(0.0, n(44.0))]))]))]))]));
        let mut cam = d.get("camera").unwrap().clone();
        obj_set(&mut cam, "clip", s("c"));
        obj_set(&mut cam, "keys", a(vec![key(0.0, n(1.0))]));
        obj_set(&mut d, "camera", cam);
        assert!(evaluate_clips(&mut d).unwrap_err().contains("both 'clip' and 'keys'"));
        // unknown clip instance
        let mut d = base_doc(a(vec![]));
        let mut cam = d.get("camera").unwrap().clone();
        obj_set(&mut cam, "clip", s("ghost"));
        obj_set(&mut d, "camera", cam);
        assert!(evaluate_clips(&mut d).unwrap_err().contains("unknown clip 'ghost'"));
        // clipTime rate must be > 0
        assert!(ClipTime::parse(Some(&o(vec![("rate", n(0.0))])), "x").is_err());
        assert!(ClipTime::parse(Some(&o(vec![("loop", s("bounce"))])), "x").is_err());
        // clipTime without clip
        let mut d = base_doc(a(vec![]));
        let mut cam = d.get("camera").unwrap().clone();
        obj_set(&mut cam, "clipTime", o(vec![("offset", n(1.0))]));
        obj_set(&mut d, "camera", cam);
        assert!(evaluate_clips(&mut d).is_err());
    }

    #[test]
    fn clip_fold_lights_points_post_and_plain_node() {
        let leaflet = o(vec![
            ("type", s("mesh")),
            ("name", s("leaflet")),
            ("geometry", o(vec![("kind", s("quad")), ("width", n(0.62)), ("height", n(0.42))])),
            ("transform", a(vec![o(vec![("translate", a(vec![n(-13.0), n(9.6), n(-13.0)]))]), o(vec![("rotate_y", n(1.5))])])),
        ]);
        let clips = o(vec![("env", o(vec![
            ("duration", n(4.0)),
            ("tracks", a(vec![
                track("lights", "key.intensity", a(vec![key(0.0, n(2.6)), keye(4.0, n(1.15), "easeInOutCubic")])),
                track("lights", "points[0].intensity", a(vec![key(0.0, n(4.6)), key(4.0, n(4.6))])),
                track("post", "bokeh.focus", a(vec![key(0.0, n(6.0)), keye(4.0, n(8.0), "linear")])),
                track("leaflet", "translate", a(vec![key(0.0, a(vec![n(-13.0), n(9.6), n(-13.0)])), keye(4.0, a(vec![n(11.0), n(9.6), n(-13.0)]), "linear")])),
                track("leaflet", "rotate.y", a(vec![key(0.0, n(1.5)), keye(4.0, n(1.5), "linear")])),
            ])),
        ]))]);
        let mut d = base_doc(a(vec![leaflet]));
        obj_set(&mut d, "clips", clips);
        obj_set(&mut d, "lights", o(vec![
            ("clip", s("env")),
            ("key", o(vec![("intensity", n(2.6)), ("color", s("#FFF0D6"))])),
            ("points", a(vec![o(vec![("intensity", n(4.6)), ("position", a(vec![n(-1.35), n(0.6), n(-2.45)]))])])),
        ]));
        obj_set(&mut d, "post", o(vec![("clip", s("env")), ("bokeh", o(vec![("focus", n(6.0)), ("aperture", n(0.0012)), ("maxblur", n(0.007))]))]));
        let mut leaf_inst = d.get("nodes").unwrap().as_arr().unwrap()[0].clone();
        obj_set(&mut leaf_inst, "clip", s("env"));
        obj_set(&mut d, "nodes", a(vec![leaf_inst]));
        obj_set(&mut d, "time", n(4.0));
        evaluate_clips(&mut d).unwrap();
        let lights = d.get("lights").unwrap();
        assert_eq!(lights.get("key").unwrap().get("intensity"), Some(&n(1.15)));
        assert_eq!(lights.get("points").unwrap().as_arr().unwrap()[0].get("intensity"), Some(&n(4.6)));
        assert_eq!(d.get("post").unwrap().get("bokeh").unwrap().get("focus"), Some(&n(8.0)));
        // translate replaced IN PLACE (op order preserved),
        // rotate_y replaced in place too.
        let node = &d.get("nodes").unwrap().as_arr().unwrap()[0];
        let ops = node.get("transform").unwrap().as_arr().unwrap();
        assert_eq!(ops.len(), 2);
        assert_eq!(ops[0].get("translate").unwrap().as_arr().unwrap()[0], n(11.0));
        assert_eq!(ops[1].get("rotate_y"), Some(&n(1.5)));
    }

    #[test]
    fn clip_pair_component_fold_uses_loader_defaults() {
        // armR.1 folds into state.armR[1]; the untouched
        // component keeps its authored value; an absent armL
        // starts from the loader default (8, 6), never (0, 0).
        let clips = o(vec![("c", o(vec![
            ("duration", n(2.0)),
            ("tracks", a(vec![
                track("Onoda", "armR.1", a(vec![key(0.0, n(-6.0)), keye(2.0, n(-10.0), "linear")])),
                track("Onoda", "armL.0", a(vec![key(0.0, n(8.0)), keye(2.0, n(20.0), "linear")])),
            ])),
        ]))]);
        let mut node = char_node("Onoda");
        // Remove armL to exercise the default path.
        let mut st = node.get("state").unwrap().clone();
        obj_remove(&mut st, "armL");
        obj_set(&mut node, "state", st);
        obj_set(&mut node, "clip", s("c"));
        let mut d = base_doc(a(vec![node]));
        obj_set(&mut d, "clips", clips);
        obj_set(&mut d, "time", n(2.0));
        evaluate_clips(&mut d).unwrap();
        let node = &d.get("nodes").unwrap().as_arr().unwrap()[0];
        let st = node.get("state").unwrap();
        assert_eq!(st.get("armR"), Some(&a(vec![n(16.0), n(-10.0)])));
        assert_eq!(st.get("armL"), Some(&a(vec![n(20.0), n(6.0)])));
    }

    // ---------------- poseTrack conversion + s12 ----------------

    fn ref_smoother(u: f64) -> f64 {
        let u = u.clamp(0.0, 1.0);
        u * u * u * (u * (u * 6.0 - 15.0) + 10.0)
    }
    /// film.rs pose_track, transcribed (keys are (t, v, moveSecs)).
    fn ref_pose_track(keys: &[(f64, f64, f64)], t: f64) -> f64 {
        let mut cur = keys[0].1;
        for w in keys.windows(2) {
            let (t1, v1, m1) = (w[1].0, w[1].1, w[1].2);
            if t < t1 - m1 {
                return cur;
            }
            if t < t1 {
                return w[0].1 + (v1 - w[0].1) * ref_smoother((t - (t1 - m1)) / m1);
            }
            cur = v1;
        }
        cur
    }

    #[test]
    fn posetrack_conversion_matches_reference() {
        let keys = [(0.0, 0.0, None), (2.5, 3.5, Some(0.45))];
        let clip_keys = pose_track_to_clip_keys(&keys).unwrap();
        assert_eq!(clip_keys.len(), 3);
        assert_eq!(clip_keys[1].t, 2.5 - 0.45);
        assert_eq!(clip_keys[1].value, 0.0, "hold key carries the previous value");
        assert_eq!(clip_keys[2].ease, "smoother");
        let raw = [(0.0, 0.0, 0.35), (2.5, 3.5, 0.45)];
        for t in [0.0, 1.0, 2.05, 2.2, 2.35, 2.5, 3.0] {
            let a = sample_scalar_keys(&clip_keys, t);
            let b = ref_pose_track(&raw, t);
            assert!((a - b).abs() < 1e-9, "t={t}: converted {a} vs pose_track {b}");
        }
        // moveSecs 0 -> step arrival; default moveSecs is 0.35.
        let k2 = pose_track_to_clip_keys(&[(0.0, 1.0, None), (1.0, 2.0, Some(0.0))]).unwrap();
        assert_eq!(k2[1].ease, "step");
        let k3 = pose_track_to_clip_keys(&[(0.0, 1.0, None), (1.0, 2.0, None)]).unwrap();
        assert_eq!(k3[1].t, 1.0 - 0.35);
        // Overlapping move windows fail like stage_clips.
        assert!(pose_track_to_clip_keys(&[(0.0, 0.0, None), (0.3, 1.0, Some(0.45)), (2.0, 2.0, None)]).is_err());
    }

    /// The s12 evidence test: the baked, stage_clips-converted
    /// tracks (f32-quantised, exactly as stage_clips.py emits
    /// them at HEAD) sampled by this evaluator reproduce film.rs
    /// s12's bow channels — bit-exact at holds and at the arrival
    /// key, within the W52 conversion envelope (<= 3.58e-7)
    /// mid-move, and the mood step exactly everywhere.
    #[test]
    fn s12_baked_tracks_reproduce_the_code_path() {
        // stage_clips.py output for s12-onoda-jg-onoda2 (HEAD).
        let lean = vec![
            ScalarKey { t: 0.0, value: 0.0, ease: "smoother".to_string() },
            ScalarKey { t: 2.049999952316284, value: 0.0, ease: "smoother".to_string() },
            ScalarKey { t: 2.5, value: 2.0999999046325684, ease: "smoother".to_string() },
        ];
        let head = vec![
            ScalarKey { t: 0.0, value: 0.0, ease: "smoother".to_string() },
            ScalarKey { t: 2.049999952316284, value: 0.0, ease: "smoother".to_string() },
            ScalarKey { t: 2.5, value: -3.5, ease: "smoother".to_string() },
        ];
        // The code of record: bow over the pre-bake track, then
        // lean = 0.6*bow, headTilt = -bow (film.rs s12).
        let bow_raw = [(0.0, 0.0, 0.35), (2.5, 3.5, 0.45)];
        // Bit-exactness holds at the holds and at the arrival —
        // probed at each route's own key times. t = 2.05 is the
        // CODE route's hold-key time; the clip route's hold key
        // sits at f32(2.05) = 2.049999952316284, so at 2.05 the
        // clip route is already ~1e-7 into its move segment and
        // is envelope-checked, not bit-checked (W52 §2).
        for t in [0.0, 0.5, 1.0, 2.049999952316284, 2.05, 2.2, 2.35, 2.5, 3.0, 4.837] {
            let bow = ref_pose_track(&bow_raw, t);
            let code_lean = (0.6 * bow) as f32;
            let code_head = (-bow) as f32;
            let clip_lean = sample_scalar_keys(&lean, t) as f32;
            let clip_head = sample_scalar_keys(&head, t) as f32;
            if t < 2.05 || t >= 2.5 {
                assert_eq!(clip_lean.to_bits(), code_lean.to_bits(), "lean bit-exact at t={t}");
                // headTilt zeros differ only in the sign of zero
                // (W52 §3, disclosed): compare values there.
                assert_eq!(clip_head, code_head, "headTilt at t={t}");
                if code_head != 0.0 {
                    assert_eq!(clip_head.to_bits(), code_head.to_bits(), "headTilt bit-exact at t={t}");
                }
            } else {
                assert!((clip_lean - code_lean).abs() <= 4e-7, "lean envelope at t={t}: {clip_lean} vs {code_lean}");
                assert!((clip_head - code_head).abs() <= 4e-7, "headTilt envelope at t={t}: {clip_head} vs {code_head}");
            }
        }
        // Mood step: film.rs `t >= w_loyal` with w_loyal - st0 =
        // 2.6 lt; the converted key is f32(2.6) = 2.5999999046325684.
        let mood = TrackDef {
            node: "Onoda (JG onoda2)".to_string(),
            channel: "mood".to_string(),
            kind: ChanKind::StepMood,
            keys: vec![
                (0.0, KeyVal::Step("relieved".to_string()), "step".to_string()),
                (2.5999999046325684, KeyVal::Step("neutral".to_string()), "step".to_string()),
            ],
        };
        let at = |tau: f64| match sample_track_f64(&mood, tau) {
            SampledF64::Step(v) => v,
            _ => panic!("step"),
        };
        assert_eq!(at(0.0), "relieved");
        assert_eq!(at(2.5), "relieved");
        assert_eq!(at(2.6), "neutral");
        assert_eq!(at(4.837), "neutral");
    }

    #[test]
    fn process_scene_end_to_end_and_duplicate_keys() {
        // Prefab instance + clip instance in one document.
        let mut doc = base_doc(a(vec![o(vec![
            ("prefab", s("marker-post")),
            ("params", o(vec![("height", n(1.0))])),
            ("name", s("gate")),
        ])]));
        obj_set(&mut doc, "clips", o(vec![("c", o(vec![
            ("duration", n(2.0)),
            ("tracks", a(vec![track("camera", "fovY", a(vec![key(0.0, n(44.0)), keye(2.0, n(40.0), "linear")]))])),
        ]))]));
        let mut cam = doc.get("camera").unwrap().clone();
        obj_set(&mut cam, "clip", s("c"));
        obj_set(&mut doc, "camera", cam);
        obj_set(&mut doc, "time", n(2.0));
        let out = process_scene(doc, &[marker_library()]).unwrap();
        assert!(out.get("prefabs").is_none() && out.get("clips").is_none());
        assert_eq!(out.get("camera").unwrap().get("fovY"), Some(&n(40.0)));
        let nodes = out.get("nodes").unwrap().as_arr().unwrap();
        assert_eq!(nodes[0].get("name").and_then(Value::as_str), Some("gate"));
        // Duplicate JSON keys on the §15 path are a defect.
        let mut dup = base_doc(a(vec![]));
        if let Value::Obj(pairs) = &mut dup {
            pairs.push(("clips".to_string(), o(vec![])));
            pairs.push(("clips".to_string(), o(vec![])));
        }
        assert!(process_scene(dup, &[]).is_err());
    }

    #[test]
    fn scalar_sampling_holds_and_constants() {
        let one = vec![ScalarKey { t: 1.0, value: 7.0, ease: "smoother".to_string() }];
        assert_eq!(sample_scalar_keys(&one, -5.0), 7.0);
        assert_eq!(sample_scalar_keys(&one, 99.0), 7.0);
        let two = vec![
            ScalarKey { t: 1.0, value: 10.0, ease: "linear".to_string() },
            ScalarKey { t: 3.0, value: 30.0, ease: "linear".to_string() },
        ];
        assert_eq!(sample_scalar_keys(&two, 0.0), 10.0);
        assert_eq!(sample_scalar_keys(&two, 2.0), 20.0);
        assert_eq!(sample_scalar_keys(&two, 9.0), 30.0);
    }

    // ---- Clip events (CR-B1-05 R5/R6 extension) ----

    fn ev(t: f64, cue: &str) -> Value {
        o(vec![("t", n(t)), ("cue", s(cue))])
    }

    fn clip_with_events(events: Vec<Value>, uses: Option<Vec<Value>>) -> Value {
        let mut pairs = vec![
            ("duration", n(2.0)),
            (
                "tracks",
                a(vec![o(vec![
                    ("target", o(vec![("node", s("camera")), ("channel", s("fovY"))])),
                    ("keys", a(vec![o(vec![("t", n(0.0)), ("value", n(40.0))])])),
                ])]),
            ),
        ];
        if let Some(u) = uses {
            pairs.push(("uses", a(u)));
        }
        if !events.is_empty() {
            pairs.push(("events", a(events)));
        }
        o(pairs)
    }

    fn doc_with_clips(clips: Vec<(&str, Value)>, nodes: Value) -> Value {
        let mut doc = base_doc(nodes);
        obj_set(&mut doc, "clips", o(clips));
        doc
    }

    #[test]
    fn events_emit_in_r5_shape_at_absolute_times() {
        let jump = clip_with_events(
            vec![
                o(vec![("t", n(0.25)), ("cue", s("takeoff")), ("gain", n(0.8))]),
                ev(1.5, "land"),
            ],
            None,
        );
        let doc = doc_with_clips(vec![("jump", jump)], a(vec![]));
        let set = ClipSet::from_root(&doc).unwrap();
        assert!(set.has_events("jump"));
        // R5 reduction: default ClipTime -> startS + ClipEvent.t.
        let out = set
            .events_in_window("jump", &ClipTime::default(), 55.52, 55.52, 60.357)
            .unwrap();
        assert_eq!(out.len(), 2);
        assert!((out[0].t - 55.77).abs() < 1e-9 && out[0].cue == "takeoff");
        assert_eq!(out[0].gain, Some(0.8));
        assert_eq!(out[0].dur, None);
        assert!((out[1].t - 57.02).abs() < 1e-9 && out[1].cue == "land");
        assert_eq!(out[1].gain, None);
        // R5 serialised shape: key order t, cue, gain, dur; absent
        // opts omitted entirely.
        let v0 = out[0].to_value();
        let v1 = out[1].to_value();
        if let Value::Obj(p) = &v0 {
            let keys: Vec<&str> = p.iter().map(|(k, _)| k.as_str()).collect();
            assert_eq!(keys, vec!["t", "cue", "gain"]);
        } else {
            panic!("event value is not an object");
        }
        if let Value::Obj(p) = &v1 {
            assert_eq!(p.len(), 2);
        } else {
            panic!("event value is not an object");
        }
    }

    #[test]
    fn events_respect_entry_offset_and_rate() {
        let jump = clip_with_events(vec![ev(0.25, "takeoff"), ev(1.5, "land")], None);
        let doc = doc_with_clips(vec![("jump", jump)], a(vec![]));
        let set = ClipSet::from_root(&doc).unwrap();
        // offset 0.5, rate 2: land at (1.5-0.5)/2 = 0.5 scene time;
        // takeoff at (0.25-0.5)/2 < 0 — before the instance's entry,
        // never fires.
        let ct = ClipTime { offset: 0.5, rate: 2.0, wrap: WrapMode::Clamp };
        let out = set.events_in_window("jump", &ct, 10.0, 10.0, 20.0).unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].cue, "land");
        assert!((out[0].t - 10.5).abs() < 1e-9);
    }

    #[test]
    fn events_suppressed_on_looping_instances() {
        let jump = clip_with_events(vec![ev(0.25, "takeoff")], None);
        let doc = doc_with_clips(vec![("jump", jump)], a(vec![]));
        let set = ClipSet::from_root(&doc).unwrap();
        for wrap in [WrapMode::Loop, WrapMode::PingPong] {
            let ct = ClipTime { offset: 0.0, rate: 1.0, wrap };
            let out = set.events_in_window("jump", &ct, 0.0, 0.0, 100.0).unwrap();
            assert!(out.is_empty(), "wrap {wrap:?} must emit no events (R5 looping law)");
        }
        assert!(set.has_events("jump"), "the clip still CARRIES the event");
    }

    #[test]
    fn events_window_is_half_open() {
        let jump = clip_with_events(vec![ev(0.5, "takeoff"), ev(1.5, "land")], None);
        let doc = doc_with_clips(vec![("jump", jump)], a(vec![]));
        let set = ClipSet::from_root(&doc).unwrap();
        // Window [10.5, 11.5) at segment start 10.0: the event at
        // exactly w0 fires, the one at exactly w1 does not — a
        // frame-stepping driver sees each event exactly once.
        let out = set
            .events_in_window("jump", &ClipTime::default(), 10.0, 10.5, 11.5)
            .unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].cue, "takeoff");
    }

    #[test]
    fn events_layer_through_uses() {
        // The layered clips must not share a track channel (the
        // layered-conflict rule), so base animates a different
        // channel than the helper's camera fovY.
        let mut base = clip_with_events(vec![ev(0.4, "inhale")], None);
        if let Value::Obj(pairs) = &mut base {
            for (k, v) in pairs.iter_mut() {
                if k == "tracks" {
                    *v = a(vec![o(vec![
                        ("target", o(vec![("node", s("post")), ("channel", s("grade.sat"))])),
                        ("keys", a(vec![o(vec![("t", n(0.0)), ("value", n(1.0))])])),
                    ])]);
                }
            }
        }
        let top = clip_with_events(vec![ev(1.0, "pop")], Some(vec![s("base")]));
        let doc = doc_with_clips(vec![("base", base), ("top", top)], a(vec![]));
        let set = ClipSet::from_root(&doc).unwrap();
        let out = set
            .events_in_window("top", &ClipTime::default(), 0.0, 0.0, 2.0)
            .unwrap();
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].cue, "inhale");
        assert_eq!(out[1].cue, "pop");
    }

    #[test]
    fn events_validation_is_closed_and_loud() {
        let bad = |events: Vec<Value>| {
            let doc = doc_with_clips(vec![("c", clip_with_events(events, None))], a(vec![]));
            ClipSet::from_root(&doc).map(|_| ())
        };
        // Unknown cue name — neither vocabulary (R7).
        assert!(bad(vec![ev(0.5, "kazoo")]).is_err());
        // An FX name is a legal literal placement (R7).
        assert!(bad(vec![ev(0.5, "thud")]).is_ok());
        // t beyond the clip duration.
        assert!(bad(vec![ev(2.5, "land")]).is_err());
        // Closed entry keys: 'pan' is a cues.json field, not an
        // event field.
        assert!(bad(vec![o(vec![
            ("t", n(0.5)),
            ("cue", s("land")),
            ("pan", n(0.5)),
        ])])
        .is_err());
        // gain must be a number when present.
        assert!(bad(vec![o(vec![
            ("t", n(0.5)),
            ("cue", s("land")),
            ("gain", s("loud")),
        ])])
        .is_err());
        // dur passes through when numeric (R5 shape superset of
        // the TS ClipEvent type).
        let doc = doc_with_clips(
            vec![("c", clip_with_events(
                vec![o(vec![("t", n(0.5)), ("cue", s("land")), ("dur", n(0.3))])],
                None,
            ))],
            a(vec![]),
        );
        let set = ClipSet::from_root(&doc).unwrap();
        let out = set.events_in_window("c", &ClipTime::default(), 0.0, 0.0, 2.0).unwrap();
        assert_eq!(out[0].dur, Some(0.3));
    }

    #[test]
    fn scene_events_document_level_and_never_folded() {
        let jump = clip_with_events(vec![ev(0.25, "takeoff")], None);
        let hero = o(vec![
            ("type", s("character")),
            ("name", s("Hero")),
            ("clip", s("jump")),
        ]);
        let doc = doc_with_clips(vec![("jump", jump)], a(vec![hero]));
        let out = scene_events(&doc, 55.52, 55.52, 60.357).unwrap();
        assert_eq!(out.len(), 1);
        assert!((out[0].t - 55.77).abs() < 1e-9);
        // Events are a side channel: processing the scene folds
        // staging only — no event data survives into the document.
        let processed = process_scene(doc, &[]).unwrap();
        assert!(!format!("{processed:?}").contains("takeoff"));
    }
}
