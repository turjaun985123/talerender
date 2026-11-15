//! DELIVERY NOTE — TALERENDER V2, Batch 1, Seat 8: WP8 scene and
//! asset streaming (`src/assetstream.rs`).
//!
//! FILES CREATED
//! - `src/assetstream.rs` (this file, the seat's only file). No
//!   existing file was edited; no dependency was added.
//!
//! CONTRACT POINTS IMPLEMENTED
//! - V2-CONTRACTS §5 Batch 1 seat 8 / master prompt §12 (WP8):
//!   asset manifest, texture deduplication, resource residency
//!   tracking, safe unloading (metadata level), missing-asset
//!   diagnostics. The manifest consumes the C4 interchange asset
//!   shape (V2-CONTRACTS §3 C4): the top-level `"assets"` array of
//!   `{ "id", "path", "kind" }` objects, plus an optional `"bytes"`
//!   size field this module defines (see LIMITATIONS).
//! - `AssetEntry { id, path, kind, bytes: Option<u64> }`;
//!   `Manifest::from_value(&scene::Value) -> Result<Manifest,
//!   String>` and the convenience
//!   `Manifest::from_project_json(text)` over
//!   `scene::parse_json` — the in-tree JSON only (§1 zero-new-deps
//!   law: std + wgpu/pollster/image, nothing else).
//! - Deduplication: `duplicates()` reports `(kept_id, dup_id)`
//!   pairs, `dedup()` returns the deduplicated manifest,
//!   `unique_paths()` counts distinct paths. Two entries are
//!   duplicates iff they name the same `path` AND their `bytes`
//!   agree whenever both are known (see the honesty note in the
//!   module docs: there is NO content hashing).
//! - Residency: `mark_resident`, `mark_all`,
//!   `evict_unreferenced(keep) -> Vec<String>` (evicted ids,
//!   sorted), `resident_bytes()`, `is_resident(id)`.
//! - `diagnostics(root) -> Vec<String>`: EVERY missing asset under
//!   `root`, plus every size mismatch — never just the first
//!   (mirrors C4's "a diagnostic listing EVERY missing asset").
//!
//! WIRING THE LEAD MUST DO
//! - Add one line to `src/main.rs` in the `mod` block, between
//!   `mod anim;` and `mod atlas;` (alphabetical position):
//!   `mod assetstream;`
//! - No call site is required in this wave: nothing in the render
//!   path consumes the manifest yet, and none may — this module
//!   manages residency METADATA only (module docs). The intended
//!   future call site is wherever the lead lands WP5's
//!   `projectfile.rs` interchange load: build the manifest with
//!   `Manifest::from_value(&root)`, print `diagnostics(root_dir)`
//!   before rendering, and drive `mark_*` / `evict_unreferenced`
//!   from shot asset references when residency becomes real.
//!
//! TESTS INCLUDED (in-module, `#[cfg(test)]`)
//! - Manifest parse: valid C4 document (via both `from_value` and
//!   `from_project_json`), missing-`"assets"` error, duplicate-id
//!   error, `..` path-escape error, malformed `bytes` error.
//! - Dedup: duplicate pairs (known-equal bytes, one-unknown bytes,
//!   same-path/different-bytes NOT a duplicate), `unique_paths`,
//!   `dedup()` keeping the first entry.
//! - Residency: eviction keeps referenced ids and returns the
//!   evicted ids sorted; `resident_bytes` sums known bytes only.
//! - Diagnostics: temp dir with one present + one missing file
//!   reports exactly the missing one; a present file with a wrong
//!   declared size reports a size mismatch.
//!
//! KNOWN LIMITATIONS
//! - Residency is bookkeeping only: `mark_resident` does not load
//!   a file and `evict_unreferenced` does not free a texture — no
//!   GPU or file I/O happens on the residency path. Wiring it to
//!   real loads/unloads is a later wave's work.
//! - Deduplication is by path + declared byte size ONLY. The
//!   zero-new-dependencies law forbids adding a hashing crate and
//!   this module does not read file contents, so two DIFFERENT
//!   files that share a path spelling and a declared size are
//!   reported as duplicates, and the same bytes under two
//!   different paths are NOT detected as duplicates. The report
//!   is a manifest-level dedup, not a content dedup.
//! - `bytes` is an optional manifest field this seat defines: C4's
//!   published asset shape is `{ id, path, kind }` with no size.
//!   When `bytes` is absent the entry's size is unknown (`None`):
//!   it contributes 0 to `resident_bytes()` (a stated undercount,
//!   never an estimate) and no size-mismatch diagnostic is
//!   possible for it.
//! - `kind` defaults to `"data"` when absent (C4 lists `image |
//!   audio | data`; the value is carried verbatim, not validated
//!   against that list, so future kinds do not break old files).
//! - Paths are validated at parse time per C4: relative, non-empty,
//!   no `..` component, not absolute. `scenefile.rs`'s own texture
//!   resolution (`scene_dir.join(file)`) is untouched and
//!   uncontradicted — this module never loads a texture.

//! Scene and asset streaming: the asset manifest, deduplication,
//! residency tracking and missing-asset diagnostics (WP8).
//!
//! SCOPE — READ FIRST. In this wave the module manages residency
//! METADATA only: it parses the interchange manifest (contract C4),
//! reports which declared assets are the same file, tracks which
//! asset ids a caller says are resident, and checks the manifest
//! against the files on disk. It loads nothing, uploads nothing
//! and frees nothing, and it changes NO visual default, resolution,
//! quality setting or render behaviour anywhere (master prompt
//! §12: "None of it may alter the default visual appearance").
//! The renderer's existing asset behaviour lives in
//! `scenefile.rs` (`resolve_texture`: `image::open` of
//! `scene_dir.join(file)`, procedural generators for file-less
//! textures); this module does not replace or wrap it.
//!
//! DEDUPLICATION HONESTY. The crate's dependency surface is fixed
//! (wgpu / pollster / image — V2-CONTRACTS §1), so there is no
//! hashing crate and this module deliberately does not invent
//! one or read file contents to hash them itself. Deduplication
//! is therefore BY PATH + DECLARED BYTE SIZE ONLY: two entries
//! are duplicates iff `path` is identical AND `bytes` agrees
//! whenever both entries declare it (if either side is unknown,
//! the path alone decides — the same path names the same file on
//! disk however much size metadata is missing). The limits of
//! that rule are stated in the DELIVERY NOTE above and on
//! `Manifest::duplicates`.

use crate::scene::{self, Value};
use std::collections::HashSet;
use std::path::{Component, Path};

/// One declared asset from the C4 interchange `"assets"` array.
///
/// `bytes` is the declared file size when the manifest carries
/// the optional `"bytes"` field; `None` means the size is unknown
/// (never estimated — see the module docs).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AssetEntry {
    pub(crate) id: String,
    pub(crate) path: String,
    pub(crate) kind: String,
    pub(crate) bytes: Option<u64>,
}

/// An asset manifest plus its residency set.
///
/// `entries` preserves manifest order (the order diagnostics and
/// duplicate reports follow). `resident` holds ids only — an id
/// can be resident only if it names a manifest entry, so phantom
/// residency is impossible by construction.
#[derive(Debug, Clone, Default)]
pub(crate) struct Manifest {
    entries: Vec<AssetEntry>,
    resident: HashSet<String>,
}

impl Manifest {
    /// Parse the C4 `"assets"` array out of an interchange
    /// document already parsed as a `scene::Value`.
    ///
    /// Errors (all `String`, in the loaders' house style of
    /// `context: message`): the root is not an object or has no
    /// `"assets"` field; `"assets"` is not an array; an entry is
    /// not an object; `id` / `path` are missing or not strings;
    /// `kind` is present but not a string; `bytes` is present but
    /// not a non-negative integer; an `id` appears twice (the
    /// same loud failure `scenefile.rs` makes for duplicate
    /// texture ids — never a silent overwrite); a `path` is
    /// empty, absolute, or escapes the project root via `..`
    /// (the C4 path rule).
    pub(crate) fn from_value(root: &Value) -> Result<Manifest, String> {
        let list = root
            .get("assets")
            .ok_or_else(|| "assets: missing required field 'assets'".to_string())?
            .as_arr()
            .ok_or_else(|| "assets: must be an array".to_string())?;
        let mut entries: Vec<AssetEntry> = Vec::with_capacity(list.len());
        let mut seen_ids: HashSet<String> = HashSet::new();
        for (i, item) in list.iter().enumerate() {
            let ctx = format!("assets[{i}]");
            if !matches!(item, Value::Obj(_)) {
                return Err(format!("{ctx}: asset entry must be an object"));
            }
            let id = item
                .get("id")
                .and_then(Value::as_str)
                .ok_or_else(|| format!("{ctx}: 'id' must be a string"))?
                .to_string();
            let path = item
                .get("path")
                .and_then(Value::as_str)
                .ok_or_else(|| format!("{ctx}: 'path' must be a string"))?
                .to_string();
            let kind = match item.get("kind") {
                Some(v) => v
                    .as_str()
                    .ok_or_else(|| format!("{ctx}: 'kind' must be a string"))?
                    .to_string(),
                None => "data".to_string(),
            };
            let bytes = match item.get("bytes") {
                Some(v) => {
                    let n = v
                        .as_f64()
                        .ok_or_else(|| format!("{ctx}: 'bytes' must be a number"))?;
                    if n < 0.0 || n.fract() != 0.0 {
                        return Err(format!(
                            "{ctx}: 'bytes' must be a non-negative integer"
                        ));
                    }
                    Some(n as u64)
                }
                None => None,
            };
            validate_relative_path(&path).map_err(|e| format!("{ctx}: {e}"))?;
            if !seen_ids.insert(id.clone()) {
                return Err(format!("{ctx}: duplicate asset id '{id}'"));
            }
            entries.push(AssetEntry { id, path, kind, bytes });
        }
        Ok(Manifest { entries, resident: HashSet::new() })
    }

    /// Convenience: parse interchange JSON text with the in-tree
    /// parser (`scene::parse_json`) and build the manifest.
    pub(crate) fn from_project_json(text: &str) -> Result<Manifest, String> {
        let root = scene::parse_json(text)?;
        Self::from_value(&root)
    }

    /// The manifest entries, in manifest order.
    pub(crate) fn entries(&self) -> &[AssetEntry] {
        &self.entries
    }

    /// Look up one entry by id.
    pub(crate) fn get(&self, id: &str) -> Option<&AssetEntry> {
        self.entries.iter().find(|e| e.id == id)
    }

    pub(crate) fn len(&self) -> usize {
        self.entries.len()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    // -----------------------------------------------------------------
    // Deduplication (path + declared byte size; no content hashing —
    // see the module docs for the honest limits of the rule).
    // -----------------------------------------------------------------

    /// The duplicate pairs, as `(kept_id, dup_id)` in manifest
    /// order. The FIRST entry naming a path is the kept one; every
    /// later entry that duplicates an earlier kept entry is
    /// reported once, against that first entry. Two entries are
    /// duplicates iff their paths are identical AND their `bytes`
    /// agree whenever both are known (either side unknown → the
    /// path alone decides). Same path with two DIFFERENT known
    /// sizes is NOT a duplicate: the manifest contradicts itself
    /// about one file, which is a defect `diagnostics` surfaces
    /// as a size mismatch against the file on disk, not something
    /// dedup may silently merge.
    pub(crate) fn duplicates(&self) -> Vec<(String, String)> {
        let mut kept: Vec<&AssetEntry> = Vec::new();
        let mut pairs = Vec::new();
        for e in &self.entries {
            match kept.iter().find(|k| entries_duplicate(k, e)) {
                Some(k) => pairs.push((k.id.clone(), e.id.clone())),
                None => kept.push(e),
            }
        }
        pairs
    }

    /// A copy of this manifest with duplicate entries removed
    /// (first occurrence wins, manifest order preserved). The
    /// residency set is carried over, intersected with the
    /// surviving ids — a removed duplicate's residency goes with
    /// it; callers that still need the asset resident re-mark the
    /// KEPT id.
    pub(crate) fn dedup(&self) -> Manifest {
        let dup_ids: HashSet<String> =
            self.duplicates().into_iter().map(|(_, dup)| dup).collect();
        let entries: Vec<AssetEntry> = self
            .entries
            .iter()
            .filter(|e| !dup_ids.contains(&e.id))
            .cloned()
            .collect();
        let resident: HashSet<String> = self
            .resident
            .iter()
            .filter(|id| entries.iter().any(|e| &e.id == *id))
            .cloned()
            .collect();
        Manifest { entries, resident }
    }

    /// The number of DISTINCT paths the manifest names. Note this
    /// counts paths, not unique assets: one path declared with
    /// two different known sizes is one unique path but (per
    /// `duplicates`) two non-duplicate entries.
    pub(crate) fn unique_paths(&self) -> usize {
        self.entries
            .iter()
            .map(|e| e.path.as_str())
            .collect::<HashSet<&str>>()
            .len()
    }

    // -----------------------------------------------------------------
    // Residency tracking (metadata only — see the module docs).
    // -----------------------------------------------------------------

    /// Mark one asset resident. Returns `true` iff `id` names a
    /// manifest entry (unknown ids are a no-op and can never
    /// become resident). Idempotent.
    pub(crate) fn mark_resident(&mut self, id: &str) -> bool {
        if self.get(id).is_some() {
            self.resident.insert(id.to_string());
            true
        } else {
            false
        }
    }

    /// Mark a slice of ids resident; returns how many of them
    /// named manifest entries. Accepts `&[String]`, `&[&str]` and
    /// any other slice of string-like ids.
    pub(crate) fn mark_all<S: AsRef<str>>(&mut self, ids: &[S]) -> usize {
        let mut found = 0;
        for id in ids {
            if self.mark_resident(id.as_ref()) {
                found += 1;
            }
        }
        found
    }

    /// Evict every resident asset NOT in `keep` (the safe-unload
    /// half of residency: only unreferenced assets leave).
    /// Returns the evicted ids, sorted. Entries stay in the
    /// manifest — only their residency flag clears. Ids in `keep`
    /// that are not resident are ignored.
    pub(crate) fn evict_unreferenced(&mut self, keep: &HashSet<String>) -> Vec<String> {
        let mut evicted: Vec<String> = self
            .resident
            .iter()
            .filter(|id| !keep.contains(*id))
            .cloned()
            .collect();
        evicted.sort();
        for id in &evicted {
            self.resident.remove(id);
        }
        evicted
    }

    /// Whether `id` is currently marked resident.
    pub(crate) fn is_resident(&self, id: &str) -> bool {
        self.resident.contains(id)
    }

    /// The resident ids, sorted (for reports and tests).
    pub(crate) fn resident_ids(&self) -> Vec<String> {
        let mut ids: Vec<String> = self.resident.iter().cloned().collect();
        ids.sort();
        ids
    }

    /// Total declared bytes of the resident entries. Entries
    /// whose size is unknown (`bytes: None`) contribute 0, so
    /// this is a floor, never an estimate.
    pub(crate) fn resident_bytes(&self) -> u64 {
        self.entries
            .iter()
            .filter(|e| self.resident.contains(&e.id))
            .fold(0u64, |acc, e| acc.saturating_add(e.bytes.unwrap_or(0)))
    }

    // -----------------------------------------------------------------
    // Diagnostics against the files on disk.
    // -----------------------------------------------------------------

    /// Check every manifest asset against `root` (the project
    /// root the manifest's relative paths resolve under) and
    /// return one diagnostic string per problem, in manifest
    /// order — EVERY missing asset is listed, never just the
    /// first, and checking never stops early:
    ///
    /// - the file does not exist (or its metadata cannot be
    ///   read): `missing asset '<id>': <path> (expected at <full>)`
    /// - the path exists but is not a file:
    ///   `asset '<id>': <path> is not a file (at <full>)`
    /// - `bytes` is known and the file's actual size differs:
    ///   `size mismatch for asset '<id>': <path> declares N bytes
    ///   but the file is M bytes`
    ///
    /// An empty result means every declared asset is present and
    /// every declared size matches. This is the module's only
    /// file-system touch, and it is read-only metadata
    /// (`std::fs::metadata`) — contents are never read.
    pub(crate) fn diagnostics(&self, root: &Path) -> Vec<String> {
        let mut out = Vec::new();
        for e in &self.entries {
            let full = root.join(&e.path);
            match std::fs::metadata(&full) {
                Err(_) => out.push(format!(
                    "missing asset '{}': {} (expected at {})",
                    e.id,
                    e.path,
                    full.display()
                )),
                Ok(md) if !md.is_file() => out.push(format!(
                    "asset '{}': {} is not a file (at {})",
                    e.id,
                    e.path,
                    full.display()
                )),
                Ok(md) => {
                    if let Some(declared) = e.bytes {
                        let actual = md.len();
                        if actual != declared {
                            out.push(format!(
                                "size mismatch for asset '{}': {} declares {} bytes but the file is {} bytes",
                                e.id, e.path, declared, actual
                            ));
                        }
                    }
                }
            }
        }
        out
    }
}

/// The dedup predicate (module docs): same path, and byte sizes
/// that agree whenever both are known.
fn entries_duplicate(a: &AssetEntry, b: &AssetEntry) -> bool {
    if a.path != b.path {
        return false;
    }
    match (a.bytes, b.bytes) {
        (Some(x), Some(y)) => x == y,
        _ => true,
    }
}

/// The C4 path rule: asset paths are relative to the project
/// root and must not escape it — non-empty, not absolute, and no
/// `..` component. (`.` components and ordinary names are fine.)
fn validate_relative_path(path: &str) -> Result<(), String> {
    if path.is_empty() {
        return Err("'path' must not be empty".to_string());
    }
    let p = Path::new(path);
    if p.is_absolute() {
        return Err(format!("'path' must be relative, got '{path}'"));
    }
    for c in p.components() {
        if matches!(c, Component::ParentDir) {
            return Err(format!("'path' must not escape the project root ('..'), got '{path}'"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    const VALID_DOC: &str = r#"{
        "format": "talerender-project/1",
        "meta": { "title": "t", "fps": 30, "width": 1080, "height": 1920 },
        "assets": [
            { "id": "bg",   "path": "img/bg.png",    "kind": "image", "bytes": 100 },
            { "id": "bg2",  "path": "img/bg.png",    "kind": "image", "bytes": 100 },
            { "id": "song", "path": "audio/song.wav", "kind": "audio" },
            { "id": "song2", "path": "audio/song.wav", "kind": "audio", "bytes": 50 },
            { "id": "alt",  "path": "img/bg.png",    "kind": "image", "bytes": 999 }
        ]
    }"#;

    fn valid_manifest() -> Manifest {
        Manifest::from_project_json(VALID_DOC).expect("valid doc parses")
    }

    #[test]
    fn parse_valid_manifest() {
        let m = valid_manifest();
        assert_eq!(m.len(), 5);
        let bg = m.get("bg").expect("bg entry");
        assert_eq!(bg.path, "img/bg.png");
        assert_eq!(bg.kind, "image");
        assert_eq!(bg.bytes, Some(100));
        // Absent "bytes" stays unknown; absent size is not 0.
        assert_eq!(m.get("song").expect("song entry").bytes, None);
        // from_value over a pre-parsed tree agrees.
        let root = scene::parse_json(VALID_DOC).expect("json parses");
        let m2 = Manifest::from_value(&root).expect("from_value parses");
        assert_eq!(m2.entries(), m.entries());
    }

    #[test]
    fn parse_missing_assets_field_is_an_error() {
        let root = scene::parse_json(r#"{ "format": "talerender-project/1" }"#)
            .expect("json parses");
        let err = Manifest::from_value(&root).expect_err("missing assets must fail");
        assert!(err.contains("assets"), "error names the field: {err}");
        // Same failure through the text convenience.
        assert!(Manifest::from_project_json(r#"{ "meta": {} }"#).is_err());
        // An empty assets array is valid, not an error.
        let empty = Manifest::from_project_json(r#"{ "assets": [] }"#)
            .expect("empty assets array parses");
        assert!(empty.is_empty());
    }

    #[test]
    fn parse_rejects_duplicate_ids_escapes_and_bad_bytes() {
        let dup = r#"{ "assets": [
            { "id": "a", "path": "x.png", "kind": "image" },
            { "id": "a", "path": "y.png", "kind": "image" } ] }"#;
        assert!(Manifest::from_project_json(dup)
            .expect_err("duplicate id must fail")
            .contains("duplicate"));
        let escape = r#"{ "assets": [
            { "id": "a", "path": "../secret.png", "kind": "image" } ] }"#;
        assert!(Manifest::from_project_json(escape).is_err());
        let bad_bytes = r#"{ "assets": [
            { "id": "a", "path": "x.png", "kind": "image", "bytes": 1.5 } ] }"#;
        assert!(Manifest::from_project_json(bad_bytes).is_err());
    }

    #[test]
    fn dedup_pairs_and_unique_paths() {
        let m = valid_manifest();
        // bg2 duplicates bg (same path, same known bytes);
        // song2 duplicates song (same path, one side unknown);
        // alt shares the path but declares a DIFFERENT known
        // size, so it is not a duplicate.
        assert_eq!(
            m.duplicates(),
            vec![
                ("bg".to_string(), "bg2".to_string()),
                ("song".to_string(), "song2".to_string()),
            ]
        );
        assert_eq!(m.unique_paths(), 2);
        let d = m.dedup();
        let ids: Vec<&str> = d.entries().iter().map(|e| e.id.as_str()).collect();
        assert_eq!(ids, vec!["bg", "song", "alt"]);
        assert!(d.duplicates().is_empty());
    }

    #[test]
    fn residency_eviction_keeps_referenced() {
        let mut m = valid_manifest();
        assert_eq!(
            m.mark_all(&["bg".to_string(), "song".to_string(), "alt".to_string()]),
            3
        );
        assert!(!m.mark_resident("no-such-id"));
        assert!(m.is_resident("bg"));
        // Known bytes only: bg 100 + song (unknown -> 0) + alt 999.
        assert_eq!(m.resident_bytes(), 1099);
        let keep: HashSet<String> = ["song".to_string()].into_iter().collect();
        let evicted = m.evict_unreferenced(&keep);
        assert_eq!(evicted, vec!["alt".to_string(), "bg".to_string()]);
        assert!(m.is_resident("song"));
        assert!(!m.is_resident("bg"));
        assert_eq!(m.resident_ids(), vec!["song".to_string()]);
        assert_eq!(m.resident_bytes(), 0);
        // Eviction is residency-only: the entries remain.
        assert_eq!(m.len(), 5);
    }

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "talerender-assetstream-{tag}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    #[test]
    fn diagnostics_present_missing_and_size_mismatch() {
        let dir = temp_dir("diag");
        std::fs::write(dir.join("present.bin"), b"12345").expect("write present file");
        let doc = r#"{ "assets": [
            { "id": "ok",      "path": "present.bin", "kind": "data", "bytes": 5 },
            { "id": "gone",    "path": "missing.bin", "kind": "data" },
            { "id": "wrongsz", "path": "present.bin", "kind": "data", "bytes": 42 }
        ] }"#;
        let m = Manifest::from_project_json(doc).expect("doc parses");
        let diags = m.diagnostics(&dir);
        // Exactly two problems, in manifest order: the missing
        // file and the size mismatch. The well-formed entry and
        // the shared path produce nothing.
        assert_eq!(diags.len(), 2, "diagnostics: {diags:?}");
        assert!(diags[0].contains("missing asset 'gone'"), "{diags:?}");
        assert!(diags[1].contains("size mismatch for asset 'wrongsz'"), "{diags:?}");
        assert!(diags[1].contains("42"), "{diags:?}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
