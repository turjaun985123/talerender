// =====================================================================
// DELIVERY NOTE — TALERENDER V2, Batch 1, Seat 5: WP5 evaluator
// generalisation (interchange loader). Contract C4
// (docs/integration/V2-CONTRACTS.md §3), master prompt §9 (WP5).
//
// FILES CREATED
//   src/projectfile.rs — this file, the ONLY file created. No
//   existing file was edited, no git command was run, and no new
//   dependency was added: JSON is the in-tree `scene::parse_json` /
//   `scene::Value` API and errors are the existing
//   `errors::TaleError` `data_load` family (exit code 2), per §1.
//
// CONTRACT POINTS IMPLEMENTED (C4)
//   * Interchange document shape: top-level `format`, `meta`
//     { title, fps, width, height }, `assets[]` { id, path, kind },
//     and the counted sections `characters` / `scenes` / `shots` /
//     `captions` / `audio` (see `load_project`).
//   * Version gate: `format` must be exactly
//     "talerender-project/1". A different `talerender-project/N`
//     major (N a parsed integer != 1) is a HARD COMPATIBILITY
//     error whose message names BOTH the found format string /
//     major and the supported one. Any other `format` value
//     (including a missing or non-string `format`) is a
//     `data_load` error naming the expected string.
//   * Meta validation: `meta` must be an object; `title` must be
//     a string; `fps` / `width` / `height` must be positive
//     integers (> 0, integer-valued, fitting u32) — zero,
//     negative, fractional, non-numeric and missing values are
//     all `data_load` errors naming the field.
//   * Assets: `kind` is the closed set image | audio | data
//     (`AssetKind::{Image, Audio, Data}`); an unknown kind is a
//     hard error naming the asset and the offending kind — never
//     a silent default. Absent `assets` = empty manifest;
//     present-but-not-an-array, non-object entries, and missing /
//     empty / non-string `id` or `path` are errors.
//   * Asset path rules (C4: relative, never escaping the project
//     root): absolute paths are rejected (leading `/` or `\`,
//     a Windows drive prefix such as `C:\`, or
//     `Path::is_absolute()`), and ANY `..` path component is
//     rejected with a named "path escape" error naming the asset
//     id and the path. Both `/` and `\` are treated as separators
//     for the `..` scan, so a Windows-spelled escape cannot slip
//     past a Unix loader (and vice versa).
//   * Section counts: `SectionCounts` records the entry count of
//     each of characters / scenes / shots / captions / audio.
//     An ABSENT section counts 0; a PRESENT section that is not
//     an array is a hard error naming the section — a mistyped
//     section is never silently counted as empty.
//   * Missing-asset reporting (C4: EVERY missing asset, never a
//     silent skip): `ProjectFile::validate_assets(root)` checks
//     every manifest entry against `root` and returns the
//     authored relative path of EVERY missing asset, in document
//     order; an empty Vec means all assets are present. It never
//     stops at the first miss.
//
// HOW THIS COMPOSES WITH THE §15 STEP-2 LOADER WORK (the packs)
//   FORMAT15-DRAFT-W31 (§15.10 adoption order) and
//   FORMAT-EXT-CLIPS-W15 (§7 adoption steps) define step 2 as
//   LOADER CONSUMPTION inside the scene format: the §15.1
//   reservation guard, §15.5 prefab expansion ahead of the node
//   walk, and clips loading / evaluator lanes in `scenefile.rs`
//   (and `film.rs`'s own walk). This module is the foundation
//   that work stands on, not a competing plan, and it stays in
//   its own lane:
//   * This file validates the PROJECT interchange ENVELOPE —
//     version compatibility, metadata, the asset manifest and
//     section presence/counts — before any per-scene document is
//     handed on.
//   * The counted sections are OPAQUE here: their entries are
//     counted, not deep-validated or evaluated. Deep shape
//     validation of scene/shot/clip/prefab data remains the
//     §15 step-2 loader's job (`scenefile.rs`), exactly as the
//     packs specify; nothing in this file expands prefabs,
//     evaluates clips, or duplicates the §15.1 guard.
//   * The matching producer, `tools/export/export_project.mjs`
//     (C4's exporter half), is Batch 2 seat B2-5's file, not
//     this seat's; this loader is its Rust-side counterpart —
//     a document the exporter emits must pass `load_project`,
//     and a document `load_project` rejects must never be
//     treated as renderable input.
//   * The s12 code retirement rides WP5 only as its pack
//     (S12-ADOPT-W83) specifies and is lead-applied / byte-probe
//     gated; this seat touches no s12 code.
//
// WIRING THE LEAD MUST DO
//   * Add the module line to src/main.rs, next to the existing
//     scene loader (anchor: the line `mod scenefile;`):
//         mod projectfile;
//     No other existing-file change is needed for this module
//     to land: it is self-contained and its tests run under
//     `cargo test` once the mod line exists.
//   * Call sites (when the lead wires a project-load CLI path):
//         let pf = projectfile::load_project(&text)?;
//         let missing = pf.validate_assets(project_root);
//         // non-empty `missing` => report ALL of them to the
//         // user as one diagnostic and refuse to render.
//     `load_project` takes the document TEXT (it does not read
//     files), so the caller owns I/O and can attach the real
//     file path via `TaleError::data_load(path, ..)` if wanted.
//
// TESTS INCLUDED (#[cfg(test)], all pure / hermetic)
//   minimal_valid_document_loads · wrong_format_string_is_
//   data_load_error · version_2_rejection_names_found_and_
//   supported · dotdot_escape_rejected · absolute_path_rejected
//   · missing_sections_count_zero · wrong_type_section_is_error
//   · unknown_asset_kind_is_hard_error · nonpositive_meta_field_
//   rejected · malformed_json_is_data_load_error ·
//   validate_assets_lists_every_missing_asset (temp-dir fixture:
//   one present file, two missing, order preserved; and the
//   all-present case returns an empty Vec).
//
// KNOWN LIMITATIONS
//   * Section ENTRIES are not validated (counts only — see the
//     §15 composition note above); a section full of malformed
//     entries loads here and fails later, in its real loader.
//   * Duplicate asset `id`s are not rejected: C4 does not ask
//     for it in this seat's charter, and inventing the rule
//     could reject exporter output the Batch 2 seat considers
//     legal. Flagged for the lead if a uniqueness rule is wanted.
//   * `meta.title` must be a string but MAY be empty (no
//     non-emptiness rule in C4).
//   * `validate_assets` tests existence only (`Path::exists`):
//     a directory at an asset path counts as present, and file
//     readability / decodability is the consuming loader's job.
//   * A `talerender-project/1.x` spelling is NOT accepted as
//     major 1: only the exact string "talerender-project/1" is
//     supported, and every other spelling errors (the /1.x form
//     via the generic unsupported-format error, not the
//     compatibility error, since its major IS 1).
// =====================================================================

//! WP5 evaluator generalisation: the `talerender-project/1`
//! interchange loader (contract C4). See the DELIVERY NOTE above.

use crate::errors::{TaleError, TaleResult};
use crate::scene::{self, Value};
use std::path::{Component, Path};

/// The one format string this loader accepts.
pub(crate) const SUPPORTED_FORMAT: &str = "talerender-project/1";

/// Prefix shared by every version of the project format; a
/// document whose `format` starts here but is not
/// [`SUPPORTED_FORMAT`] is a version problem, not random data.
const FORMAT_PREFIX: &str = "talerender-project/";

/// The closed asset-kind vocabulary of C4. An unknown `kind`
/// string in a document is a hard error, never a default.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AssetKind {
    Image,
    Audio,
    Data,
}

impl AssetKind {
    /// The document spelling of this kind (`"image"` etc.).
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            AssetKind::Image => "image",
            AssetKind::Audio => "audio",
            AssetKind::Data => "data",
        }
    }
}

/// One asset-manifest entry: a project-relative file the
/// document depends on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Asset {
    pub(crate) id: String,
    pub(crate) path: String,
    pub(crate) kind: AssetKind,
}

/// Entry counts of the document's content sections. An absent
/// section counts 0 (see `load_project`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct SectionCounts {
    pub(crate) characters: usize,
    pub(crate) scenes: usize,
    pub(crate) shots: usize,
    pub(crate) captions: usize,
    pub(crate) audio: usize,
}

impl SectionCounts {
    /// Total entries across all counted sections.
    pub(crate) fn total(&self) -> usize {
        self.characters + self.scenes + self.shots + self.captions + self.audio
    }
}

/// A loaded, envelope-validated `talerender-project/1` document.
/// Section contents are deliberately not carried: this type is
/// the interchange envelope (metadata + manifest + counts) the
/// §15 step-2 loader consumption builds on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProjectFile {
    pub(crate) title: String,
    pub(crate) fps: u32,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) assets: Vec<Asset>,
    pub(crate) section_counts: SectionCounts,
}

impl ProjectFile {
    /// Check every manifest asset against the project `root`
    /// and return the authored relative path of EVERY missing
    /// asset, in document order. Never stops at the first miss
    /// and never silently skips an entry; an empty Vec means
    /// all assets are present.
    pub(crate) fn validate_assets(&self, root: &Path) -> Vec<String> {
        let mut missing = Vec::new();
        for asset in &self.assets {
            if !root.join(&asset.path).exists() {
                missing.push(asset.path.clone());
            }
        }
        missing
    }
}

/// Every failure in this module is a `data_load` error (invalid
/// input, exit code 2) — the format/version/path problems of an
/// interchange document are the caller's data, not a render or
/// output failure.
fn err(msg: String) -> TaleError {
    TaleError::data_load_msg(msg)
}

/// Short JSON type name for error messages.
fn type_name(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "a boolean",
        Value::Num(_) => "a number",
        Value::Str(_) => "a string",
        Value::Arr(_) => "an array",
        Value::Obj(_) => "an object",
    }
}

/// Parse and envelope-validate a `talerender-project/1`
/// document from its text. Pure: no file I/O (the caller reads
/// the file), so every rule is unit-testable in-module.
pub(crate) fn load_project(text: &str) -> TaleResult<ProjectFile> {
    let root = scene::parse_json(text)
        .map_err(|e| err(format!("project: invalid JSON: {e}")))?;
    if !matches!(root, Value::Obj(_)) {
        return Err(err(format!(
            "project: top-level value must be a JSON object, found {}",
            type_name(&root)
        )));
    }

    check_format(&root)?;

    // ---- meta -------------------------------------------------
    let meta = root.get("meta").ok_or_else(|| {
        err("project: missing required field 'meta' \
             (object with title, fps, width, height)"
            .to_string())
    })?;
    if !matches!(meta, Value::Obj(_)) {
        return Err(err(format!(
            "project: 'meta' must be an object, found {}",
            type_name(meta)
        )));
    }
    let title = meta
        .get("title")
        .and_then(Value::as_str)
        .ok_or_else(|| err("project: 'meta.title' must be a string".to_string()))?
        .to_string();
    let fps = meta_u32(meta, "fps")?;
    let width = meta_u32(meta, "width")?;
    let height = meta_u32(meta, "height")?;

    // ---- assets -----------------------------------------------
    let assets = parse_assets(&root)?;

    // ---- section counts ---------------------------------------
    let section_counts = SectionCounts {
        characters: count_section(&root, "characters")?,
        scenes: count_section(&root, "scenes")?,
        shots: count_section(&root, "shots")?,
        captions: count_section(&root, "captions")?,
        audio: count_section(&root, "audio")?,
    };

    Ok(ProjectFile {
        title,
        fps,
        width,
        height,
        assets,
        section_counts,
    })
}

/// The C4 version gate. Exact match loads; a different
/// `talerender-project/N` major is a hard compatibility error
/// naming found vs supported; anything else is a data_load
/// error naming the expected format.
fn check_format(root: &Value) -> TaleResult<()> {
    let format = root
        .get("format")
        .ok_or_else(|| {
            err(format!(
                "project: missing required field 'format' (expected \"{SUPPORTED_FORMAT}\")"
            ))
        })?
        .as_str()
        .ok_or_else(|| err("project: 'format' must be a string".to_string()))?;

    if format == SUPPORTED_FORMAT {
        return Ok(());
    }
    if let Some(rest) = format.strip_prefix(FORMAT_PREFIX) {
        // A versioned sibling: "2", "2.0", "1.5", "x", ...
        let major_str = rest.split('.').next().unwrap_or("");
        if let Ok(major) = major_str.parse::<u64>() {
            if major != 1 {
                return Err(err(format!(
                    "project: incompatible format version: found '{format}' \
                     (major {major}), supported is '{SUPPORTED_FORMAT}' (major 1) \
                     — this loader cannot read talerender-project/{major} documents"
                )));
            }
        }
    }
    Err(err(format!(
        "project: unsupported format '{format}': expected '{SUPPORTED_FORMAT}'"
    )))
}

/// A `meta` dimension field: required, numeric, integer-valued
/// and > 0, fitting u32.
fn meta_u32(meta: &Value, field: &str) -> TaleResult<u32> {
    let v = meta.get(field).ok_or_else(|| {
        err(format!("project: missing required field 'meta.{field}'"))
    })?;
    let n = v.as_f64().ok_or_else(|| {
        err(format!(
            "project: 'meta.{field}' must be a number, found {}",
            type_name(v)
        ))
    })?;
    if n <= 0.0 || n.fract() != 0.0 || n > u32::MAX as f64 {
        return Err(err(format!(
            "project: 'meta.{field}' must be a positive integer (> 0), found {n}"
        )));
    }
    Ok(n as u32)
}

/// The asset manifest. Absent = empty; present-but-wrong-type,
/// malformed entries and unknown kinds are hard errors.
fn parse_assets(root: &Value) -> TaleResult<Vec<Asset>> {
    let items = match root.get("assets") {
        None => return Ok(Vec::new()),
        Some(v) => v.as_arr().ok_or_else(|| {
            err(format!(
                "project: 'assets' must be an array, found {}",
                type_name(v)
            ))
        })?,
    };
    let mut assets = Vec::with_capacity(items.len());
    for (i, item) in items.iter().enumerate() {
        if !matches!(item, Value::Obj(_)) {
            return Err(err(format!(
                "project: assets[{i}] must be an object with id, path and kind, found {}",
                type_name(item)
            )));
        }
        let id = item
            .get("id")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| {
                err(format!(
                    "project: assets[{i}]: 'id' must be a non-empty string"
                ))
            })?
            .to_string();
        let path = item
            .get("path")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| {
                err(format!(
                    "project: assets[{i}] (id '{id}'): 'path' must be a non-empty string"
                ))
            })?
            .to_string();
        let kind_str = item
            .get("kind")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                err(format!(
                    "project: assets[{i}] (id '{id}'): 'kind' must be a string \
                     (\"image\", \"audio\" or \"data\")"
                ))
            })?;
        let kind = match kind_str {
            "image" => AssetKind::Image,
            "audio" => AssetKind::Audio,
            "data" => AssetKind::Data,
            other => {
                return Err(err(format!(
                    "project: assets[{i}] (id '{id}'): unknown asset kind '{other}' \
                     — expected one of \"image\", \"audio\", \"data\""
                )))
            }
        };
        validate_asset_path(&id, &path)?;
        assets.push(Asset { id, path, kind });
    }
    Ok(assets)
}

/// C4 path law for one asset: relative only, and no `..`
/// component may escape the project root. Both spellings of
/// separator are scanned so the check is authoring-platform
/// independent.
fn validate_asset_path(id: &str, path: &str) -> TaleResult<()> {
    let drive_prefixed = {
        let b = path.as_bytes();
        b.len() >= 2 && b[0].is_ascii_alphabetic() && b[1] == b':'
    };
    if path.starts_with('/') || path.starts_with('\\') || drive_prefixed || Path::new(path).is_absolute() {
        return Err(err(format!(
            "project: asset '{id}': absolute path rejected: '{path}' \
             — asset paths must be relative to the project root"
        )));
    }
    let has_parent_component = path
        .split(|c: char| c == '/' || c == '\\')
        .any(|component| component == "..")
        || Path::new(path)
            .components()
            .any(|c| matches!(c, Component::ParentDir));
    if has_parent_component {
        return Err(err(format!(
            "project: asset '{id}': path escape rejected: '{path}' contains a '..' \
             component — asset paths must not escape the project root"
        )));
    }
    Ok(())
}

/// Count one content section: absent = 0, an array counts its
/// entries, anything else present is a hard error.
fn count_section(root: &Value, name: &str) -> TaleResult<usize> {
    match root.get(name) {
        None => Ok(0),
        Some(Value::Arr(items)) => Ok(items.len()),
        Some(v) => Err(err(format!(
            "project: section '{name}' must be an array, found {}",
            type_name(v)
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// A complete document skeleton: `assets` is a full array
    /// literal, `sections` is raw top-level text already carrying
    /// its leading commas ("" for none).
    fn project_json(assets: &str, sections: &str) -> String {
        format!(
            r#"{{"format":"talerender-project/1","meta":{{"title":"T","fps":30,"width":1080,"height":1920}},"assets":{assets}{sections}}}"#
        )
    }

    #[test]
    fn minimal_valid_document_loads() {
        let text = project_json(
            r#"[{"id":"hero","path":"img/hero.png","kind":"image"},
                {"id":"theme","path":"audio/theme.ogg","kind":"audio"}]"#,
            r#","characters":[{}],"scenes":[{},{}],"shots":[]
               ,"captions":[{}],"audio":[{}]"#,
        );
        let pf = load_project(&text).expect("valid document loads");
        assert_eq!(pf.title, "T");
        assert_eq!(pf.fps, 30);
        assert_eq!(pf.width, 1080);
        assert_eq!(pf.height, 1920);
        assert_eq!(pf.assets.len(), 2);
        assert_eq!(pf.assets[0].id, "hero");
        assert_eq!(pf.assets[0].path, "img/hero.png");
        assert_eq!(pf.assets[0].kind, AssetKind::Image);
        assert_eq!(pf.assets[0].kind.as_str(), "image");
        assert_eq!(pf.assets[1].kind, AssetKind::Audio);
        assert_eq!(
            pf.section_counts,
            SectionCounts {
                characters: 1,
                scenes: 2,
                shots: 0,
                captions: 1,
                audio: 1
            }
        );
        assert_eq!(pf.section_counts.total(), 5);
    }

    #[test]
    fn wrong_format_string_is_data_load_error() {
        let text = r#"{"format":"talerender-scene","meta":{"title":"T","fps":30,"width":1080,"height":1920}}"#;
        let e = load_project(text).unwrap_err();
        assert_eq!(e.category(), "data-load");
        assert_eq!(e.exit_code(), 2);
        let msg = e.to_string();
        assert!(msg.contains("talerender-scene"), "got: {msg}");
        assert!(msg.contains(SUPPORTED_FORMAT), "got: {msg}");
    }

    #[test]
    fn version_2_rejection_names_found_and_supported() {
        let text = r#"{"format":"talerender-project/2","meta":{"title":"T","fps":30,"width":1080,"height":1920}}"#;
        let e = load_project(text).unwrap_err();
        assert_eq!(e.category(), "data-load");
        assert_eq!(e.exit_code(), 2);
        let msg = e.to_string();
        // The compatibility error names BOTH versions.
        assert!(msg.contains("incompatible"), "got: {msg}");
        assert!(msg.contains("talerender-project/2"), "got: {msg}");
        assert!(msg.contains("talerender-project/1"), "got: {msg}");
        assert!(msg.contains("major 2"), "got: {msg}");
        assert!(msg.contains("major 1"), "got: {msg}");
    }

    #[test]
    fn dotdot_escape_rejected() {
        for bad in ["../secret.png", "img/../../secret.png", r"img\..\secret.png"] {
            // JSON-escape the backslashes for the document text;
            // the parsed path the loader sees is `bad` verbatim.
            let json_path = bad.replace('\\', "\\\\");
            let text = project_json(
                &format!(r#"[{{"id":"x","path":"{json_path}","kind":"image"}}]"#),
                "",
            );
            let e = load_project(&text).unwrap_err();
            let msg = e.to_string();
            assert!(msg.contains("path escape"), "path {bad:?}, got: {msg}");
            assert!(msg.contains(".."), "path {bad:?}, got: {msg}");
            assert!(msg.contains(bad), "path {bad:?}, got: {msg}");
        }
    }

    #[test]
    fn absolute_path_rejected() {
        for bad in ["/etc/hero.png", r"C:\art\hero.png", r"\\share\hero.png"] {
            // JSON-escape the backslashes for the document text.
            let json_path = bad.replace('\\', "\\\\");
            let text = project_json(
                &format!(r#"[{{"id":"x","path":"{json_path}","kind":"image"}}]"#),
                "",
            );
            let e = load_project(&text).unwrap_err();
            let msg = e.to_string();
            assert!(msg.contains("absolute path rejected"), "path {bad:?}, got: {msg}");
        }
    }

    #[test]
    fn missing_sections_count_zero() {
        let text = project_json("[]", "");
        let pf = load_project(&text).expect("bare document loads");
        assert_eq!(pf.section_counts, SectionCounts::default());
        assert_eq!(pf.section_counts.characters, 0);
        assert_eq!(pf.section_counts.scenes, 0);
        assert_eq!(pf.section_counts.shots, 0);
        assert_eq!(pf.section_counts.captions, 0);
        assert_eq!(pf.section_counts.audio, 0);
        assert!(pf.assets.is_empty());
    }

    #[test]
    fn wrong_type_section_is_error() {
        for (section, value) in [
            ("scenes", "{}"),
            ("shots", "3"),
            ("characters", "\"many\""),
            ("captions", "null"),
            ("audio", "true"),
        ] {
            let text = project_json("[]", &format!(r#","{section}":{value}"#));
            let e = load_project(&text).unwrap_err();
            let msg = e.to_string();
            assert!(
                msg.contains(&format!("section '{section}'")),
                "section {section}, got: {msg}"
            );
            assert!(msg.contains("must be an array"), "section {section}, got: {msg}");
        }
    }

    #[test]
    fn unknown_asset_kind_is_hard_error() {
        let text = project_json(
            r#"[{"id":"clip","path":"vid/clip.mp4","kind":"video"}]"#,
            "",
        );
        let e = load_project(&text).unwrap_err();
        let msg = e.to_string();
        assert!(msg.contains("unknown asset kind 'video'"), "got: {msg}");
        assert!(msg.contains("clip"), "got: {msg}");
    }

    #[test]
    fn nonpositive_meta_field_rejected() {
        for (field, value) in [("fps", "0"), ("width", "-2"), ("height", "1.5")] {
            // One bad field per document, built directly (a second
            // key would not override: Value::get finds the first).
            let text = match field {
                "fps" => r#"{"format":"talerender-project/1","meta":{"title":"T","fps":0,"width":1080,"height":1920}}"#,
                "width" => r#"{"format":"talerender-project/1","meta":{"title":"T","fps":30,"width":-2,"height":1920}}"#,
                _ => r#"{"format":"talerender-project/1","meta":{"title":"T","fps":30,"width":1080,"height":1.5}}"#,
            };
            let e = load_project(text).unwrap_err();
            let msg = e.to_string();
            assert!(
                msg.contains(&format!("'meta.{field}'")),
                "field {field} (value {value}), got: {msg}"
            );
            assert!(msg.contains("positive integer"), "field {field}, got: {msg}");
        }
    }

    #[test]
    fn malformed_json_is_data_load_error() {
        let e = load_project("{ not json").unwrap_err();
        assert_eq!(e.category(), "data-load");
        assert!(e.to_string().contains("invalid JSON"), "got: {e}");
    }

    #[test]
    fn validate_assets_lists_every_missing_asset() {
        // Hermetic temp fixture: one present file, two missing.
        let dir: PathBuf = std::env::temp_dir()
            .join(format!("talerender-projectfile-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("img")).expect("make fixture dir");
        std::fs::write(dir.join("img/present.png"), b"x").expect("write present asset");

        let text = project_json(
            r#"[{"id":"a","path":"img/present.png","kind":"image"},
                {"id":"b","path":"img/missing-one.png","kind":"image"},
                {"id":"c","path":"audio/missing-two.ogg","kind":"audio"}]"#,
            "",
        );
        let pf = load_project(&text).expect("document loads");
        let missing = pf.validate_assets(&dir);
        // EVERY missing asset, in document order — not just one.
        assert_eq!(
            missing,
            vec![
                "img/missing-one.png".to_string(),
                "audio/missing-two.ogg".to_string()
            ]
        );

        // All present => empty Vec.
        let text = project_json(
            r#"[{"id":"a","path":"img/present.png","kind":"image"}]"#,
            "",
        );
        let pf = load_project(&text).expect("document loads");
        assert!(pf.validate_assets(&dir).is_empty());

        let _ = std::fs::remove_dir_all(&dir);
    }
}
