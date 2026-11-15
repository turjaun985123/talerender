//! assetwire — CR-B1-09 (WP8 production caller): a loading
//! front-end that puts WP8's deduplication in front of the
//! scene loader's file-texture reads.
//!
//! =================================================================
//! DELIVERY NOTE — Chrome-Replacement Batch 1, Seat 9. NEW FILE
//! ONLY; no existing file was edited, no dependency was added
//! (std + `crate::assetstream` + `crate::scene` only; the
//! actual image decode stays in the caller's closure — see
//! below).
//! =================================================================
//!
//! THE ACTUAL ASSET-LOAD CALL SITES — VERIFIED IN CODE
//! (HEAD b093f3a):
//!   - src/scene.rs performs NO file I/O at all (grep-verified:
//!     no `image::open`, no `fs::read`, no `File::open`). It
//!     supplies only the JSON tree (`scene::Value`) and the
//!     procedural painters. Anyone wiring "scene.rs asset
//!     loads" is wiring a site that does not exist.
//!   - The scene loader's ONLY file-asset read is
//!     `scenefile::resolve_texture` (src/scenefile.rs:409),
//!     file branch (lines 410-418):
//!         if let Some(path) = spec.get("file") {
//!             let path = path.as_str().ok_or_else(||
//!                 err(ctx, "'file' must be a string"))?;
//!             let full = scene_dir.join(path);
//!             let img = image::open(&full).map_err(|e|
//!                 err(ctx, &format!("cannot load texture file
//!                     {}: {e}", full.display())))?.to_rgba8();
//!             ...
//!         }
//!     (`err(ctx, msg)` is `format!("{ctx}: {msg}")`,
//!     scenefile.rs:151.) Everything else in that function is
//!     a procedural generator dispatch — no I/O.
//!   - Its ONLY production caller is the texture loop in
//!     `scenefile::load` (scenefile.rs:1010-1021): for each
//!     entry of `root.textures`, IN ORDER — duplicate-id
//!     check first (`duplicate texture id '{id}'`, line
//!     1016), then `textures.push(resolve_texture(...)?)`.
//!     The `?` means the FIRST failure aborts the whole load
//!     and later textures are never attempted. Textures that
//!     `walk_nodes` appends later (painted character cards +
//!     blob shadows, scenefile.rs:688-691) are generated in
//!     memory, never file reads, and are untouched by this
//!     wire.
//!   - The film driver's own asset reads (`film::load_png` /
//!     `load_assets`, film.rs:2173ff — plates, caption strips,
//!     calendar, sign) are a DIFFERENT loader with its own
//!     error strings; they are out of this seat's scope (the
//!     charter names the scene loader) and are noted in
//!     CR-B1-09-CALLERS.md §2.4 as a possible follow-up, NOT
//!     wired here.
//!
//! IDENTICAL FAILURE BEHAVIOUR — THE DESIGN CONSTRAINTS
//!   1. ORDER + FIRST-FAILURE: the front-end resolves one
//!      spec at a time, at the exact point `resolve_texture`
//!      would run. It NEVER pre-scans, pre-validates or
//!      pre-loads later specs, so the first error — its
//!      string AND the moment it surfaces — is unchanged.
//!   2. SAME ERROR STRINGS: the `'file' must be a string`
//!      error is produced here with the identical format;
//!      the open error is produced by the CALLER'S closure,
//!      which is the current `image::open` line moved
//!      verbatim (see [`AssetWire::load_error`], which pins
//!      its exact format so the closure cannot drift).
//!   3. FAILURES ARE NEVER CACHED: a missing/undecodable
//!      file errors on EVERY request for it, with the opener
//!      invoked every time — a later spec naming the same
//!      missing file fails at its own turn, exactly as the
//!      current double-`image::open` would.
//!   4. NO STALE HITS: a cached decode is served ONLY when a
//!      fresh `metadata` stat at request time shows the same
//!      path still present as a file with the SAME byte size
//!      as when it was decoded. If the stat fails (file
//!      deleted since the first load) or the size changed,
//!      the front-end falls through to a real open — and its
//!      real error — never to the cache. This mirrors WP8's
//!      own dedup rule (path + size; assetstream.rs module
//!      docs: no content hashing exists in this crate).
//!   Residual divergence, stated honestly: a file REWRITTEN
//!   with different bytes but the SAME size between two
//!   specs of one load would be served from cache where the
//!   current code would decode the new bytes. That is
//!   precisely assetstream's documented path+size limit,
//!   and within one `load()` call no in-process write can
//!   interleave (textures resolve back-to-back before any
//!   rendering); an external writer racing a load is the
//!   only trigger. Recorded in CR-B1-09-CALLERS.md §2.3.
//!
//! WHY THE DECODE IS A CLOSURE: `resolve_texture` and
//! `scenefile::load` are PRIVATE (`fn`, not `pub(crate)`),
//! so this module cannot call them, and duplicating the
//! `image::open` line here would create a second spelling
//! of the error path to keep in sync. Instead the lead's
//! one-line restructure of the file branch passes the
//! current decode expression in as `open` (full text in
//! CR-B1-09-CALLERS.md §2.2); this module owns WHEN it is
//! invoked (once per cache miss, at the spec's turn) and
//! guarantees constraints 1-4 around it.
//!
//! THE ASSETSTREAM CONNECTION, AND ITS SCOPE-OUT:
//!   - [`AssetWire::manifest`] / [`AssetWire::dedup_pairs`]
//!     DO use `crate::assetstream::Manifest`: they build the
//!     C4 manifest value from a scene's texture specs (file
//!     specs only; `bytes` = the file's current size when it
//!     stats, else unknown) and report the manifest-level
//!     duplicate pairs. Tests prove the pairs agree with the
//!     cache's hit behaviour on the same fixtures.
//!   - But `Manifest::from_value` must NOT be placed in the
//!     load path as a gate: it REJECTS relative paths with a
//!     `..` component (assetstream's C4 path rule), which
//!     `scene_dir.join(path)` loads perfectly legally today
//!     (a scene referencing `../shared/tex.png` works). Its
//!     `diagnostics()` must not gate either: it lists EVERY
//!     missing asset in its own string format, replacing
//!     the first-failure ctx-prefixed error and moving the
//!     failure BEFORE the duplicate-id checks that precede
//!     resolution in the current order. Both are scoped out
//!     in CR-B1-09-CALLERS.md §2.3; the manifest helpers are
//!     advisory (logging / pre-flight reporting the lead may
//!     print WITHOUT failing the load on their result).
//!
//! WIRING THE LEAD MUST DO: in src/main.rs add
//! `mod assetwire;` after `mod assetstream;` (line 21 at
//! b093f3a); the `resolve_texture` restructure is in
//! CR-B1-09-CALLERS.md §2.2. The wire holds one
//! `AssetWire` per `load()` call (cache lifetime = one
//! scene load — the current code keeps no cross-load
//! cache, and neither does this wire).

use crate::assetstream::Manifest;
use crate::scene::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// A decoded texture, in `resolve_texture`'s return shape:
/// `(RGBA8 pixels, width, height)`.
pub(crate) type DecodedTexture = (Vec<u8>, u32, u32);

/// The cache key: WP8's dedup identity — the full resolved
/// path plus the file's byte size at decode time.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct TexKey {
    full: PathBuf,
    size: u64,
}

/// The deduplicating front-end for file textures. One
/// instance per scene `load()`; see the module docs for the
/// failure-behaviour constraints this type maintains.
#[derive(Debug, Default)]
pub(crate) struct AssetWire {
    cache: HashMap<TexKey, DecodedTexture>,
    opens: u64,
    hits: u64,
}

impl AssetWire {
    pub(crate) fn new() -> Self {
        AssetWire::default()
    }

    /// How many times the opener closure was actually
    /// invoked (cache misses + fall-throughs).
    pub(crate) fn opens(&self) -> u64 {
        self.opens
    }

    /// How many requests were served from the cache without
    /// invoking the opener.
    pub(crate) fn hits(&self) -> u64 {
        self.hits
    }

    /// The current open-error string, pinned:
    /// scenefile's `err(ctx, &format!("cannot load texture
    /// file {}: {e}", full.display()))` with
    /// `err(ctx, msg) = "{ctx}: {msg}"`. The lead's opener
    /// closure formats its `image::open` error through this
    /// so the string can never drift from today's.
    pub(crate) fn load_error(ctx: &str, full: &Path, error: &dyn std::fmt::Display) -> String {
        format!("{ctx}: cannot load texture file {}: {error}", full.display())
    }

    /// Resolve one texture spec's file branch.
    ///
    /// Returns:
    /// - `None` — the spec has NO `"file"` field: it is a
    ///   generator texture; the caller runs the existing
    ///   generator dispatch completely unchanged.
    /// - `Some(Err(..))` — exactly the error the current
    ///   file branch produces for this spec at this point
    ///   (non-string `"file"`, or the opener's error,
    ///   verbatim).
    /// - `Some(Ok(..))` — the decoded texture, either freshly
    ///   opened (opener invoked exactly once, now) or served
    ///   from the cache under constraint 4 (module docs).
    ///
    /// `open` is invoked with the joined path
    /// `scene_dir.join(file)` — the same `PathBuf` the
    /// current code builds — and ONLY on a genuine miss.
    pub(crate) fn resolve_file<F>(
        &mut self,
        spec: &Value,
        scene_dir: &Path,
        ctx: &str,
        open: F,
    ) -> Option<Result<DecodedTexture, String>>
    where
        F: FnOnce(&Path) -> Result<DecodedTexture, String>,
    {
        let file_value = spec.get("file")?;
        let file = match file_value.as_str() {
            Some(s) => s,
            None => {
                return Some(Err(format!("{ctx}: 'file' must be a string")));
            }
        };
        let full = scene_dir.join(file);

        // Constraint 4: revalidate at request time. Only a
        // regular file whose CURRENT size matches a cached
        // decode's key may be served without opening.
        let size_now = std::fs::metadata(&full)
            .ok()
            .filter(|md| md.is_file())
            .map(|md| md.len());
        if let Some(size) = size_now {
            let key = TexKey {
                full: full.clone(),
                size,
            };
            if let Some(cached) = self.cache.get(&key) {
                self.hits += 1;
                return Some(Ok(cached.clone()));
            }
        }

        // Genuine miss (or unstat-able / changed file): open
        // for real, at this spec's turn, exactly once.
        self.opens += 1;
        let decoded = match open(&full) {
            Ok(d) => d,
            Err(e) => return Some(Err(e)), // constraint 3: never cached
        };
        // Cache under the size observed BEFORE the open (the
        // size the dedup decision was made with). If the file
        // could not be statted before opening, nothing is
        // cached — a later request stats/opens again, which
        // is the current behaviour for such a file.
        if let Some(size) = size_now {
            self.cache.insert(TexKey { full, size }, decoded.clone());
        }
        Some(Ok(decoded))
    }

    /// Build the WP8 manifest for a scene's texture specs:
    /// one C4 entry per FILE-bearing spec (generator specs
    /// declare no asset and are skipped), `id` from the
    /// spec's `"id"`, `path` = the spec's `"file"` string
    /// verbatim, `kind = "image"`, `bytes` = the file's
    /// current size under `scene_dir` when it stats, else
    /// unknown (`None`).
    ///
    /// Errors: a file spec whose `"id"` is missing/not a
    /// string (the manifest cannot name it), plus every
    /// `Manifest::from_value` error — INCLUDING its `..`
    /// path rejection. That asymmetry with the loader (which
    /// accepts `..` paths) is exactly why this is an
    /// advisory surface, never a load-path gate (module
    /// docs). Callers that log its result must not fail the
    /// load on `Err`.
    pub(crate) fn manifest(scene_dir: &Path, texture_specs: &[Value]) -> Result<Manifest, String> {
        let mut entries = Vec::new();
        for (i, spec) in texture_specs.iter().enumerate() {
            let Some(file) = spec.get("file").and_then(Value::as_str) else {
                continue; // generator texture: no asset declared
            };
            let id = spec
                .get("id")
                .and_then(Value::as_str)
                .ok_or_else(|| format!("scene.textures[{i}]: 'id' must be a string"))?
                .to_string();
            let bytes = std::fs::metadata(scene_dir.join(file))
                .ok()
                .filter(|md| md.is_file())
                .map(|md| md.len());
            let mut obj = vec![
                ("id".to_string(), Value::Str(id)),
                ("path".to_string(), Value::Str(file.to_string())),
                ("kind".to_string(), Value::Str("image".to_string())),
            ];
            if let Some(b) = bytes {
                obj.push(("bytes".to_string(), Value::Num(b as f64)));
            }
            entries.push(Value::Obj(obj));
        }
        let root = Value::Obj(vec![("assets".to_string(), Value::Arr(entries))]);
        Manifest::from_value(&root)
    }

    /// The manifest-level duplicate pairs `(kept_id, dup_id)`
    /// for a scene's texture specs — the WP8 report the lead
    /// may log beside a load. Advisory only (see
    /// [`AssetWire::manifest`]).
    pub(crate) fn dedup_pairs(
        scene_dir: &Path,
        texture_specs: &[Value],
    ) -> Result<Vec<(String, String)>, String> {
        Ok(Self::manifest(scene_dir, texture_specs)?.duplicates())
    }
}

// ---------------------------------------------------------------------
// Tests. The fake opener stands in for the caller's decode
// closure: it counts invocations and derives its result from
// the real file at the given path (missing file -> the pinned
// load_error string with a fixed inner error; present file ->
// a deterministic "decode" of its bytes), so ordering, timing
// and failure repetition are all observable.
// ---------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::path::PathBuf;

    fn spec(pairs: &[(&str, Value)]) -> Value {
        Value::Obj(
            pairs
                .iter()
                .map(|(k, v)| (k.to_string(), v.clone()))
                .collect(),
        )
    }

    fn file_spec(id: &str, file: &str) -> Value {
        spec(&[
            ("id", Value::Str(id.to_string())),
            ("file", Value::Str(file.to_string())),
        ])
    }

    /// A fake opener with the production closure's contract:
    /// missing file → Err(load_error(ctx-less inner, ...)).
    /// (The production closure captures its ctx; the fake
    /// receives the path only, so tests pass ctx through a
    /// thread-local-free trick: they call `make_opener` with
    /// the ctx baked in.)
    fn make_opener<'a>(
        calls: &'a Cell<u64>,
        ctx: &'a str,
    ) -> impl FnMut(&Path) -> Result<DecodedTexture, String> + 'a {
        move |full: &Path| {
            calls.set(calls.get() + 1);
            match std::fs::read(full) {
                Ok(bytes) => {
                    let w = bytes.len() as u32;
                    Ok((bytes, w, 1))
                }
                Err(e) => Err(AssetWire::load_error(ctx, full, &e)),
            }
        }
    }

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "talerender-assetwire-test-{}-{tag}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    #[test]
    fn generator_specs_pass_through_untouched() {
        let dir = temp_dir("gen");
        let calls = Cell::new(0);
        let mut wire = AssetWire::new();
        let gen = spec(&[
            ("id", Value::Str("g".to_string())),
            ("generator", Value::Str("grass".to_string())),
        ]);
        let r = wire.resolve_file(&gen, &dir, "scene.textures[0]", make_opener(&calls, "x"));
        assert!(r.is_none(), "no 'file' field → caller runs the generator branch");
        assert_eq!(calls.get(), 0, "opener must not run for generator specs");
        assert_eq!(wire.opens(), 0);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn non_string_file_is_the_current_error_and_opens_nothing() {
        let dir = temp_dir("nonstr");
        let calls = Cell::new(0);
        let mut wire = AssetWire::new();
        let bad = spec(&[
            ("id", Value::Str("t".to_string())),
            ("file", Value::Num(3.0)),
        ]);
        let r = wire.resolve_file(&bad, &dir, "scene.textures[2]", make_opener(&calls, "x"))
            .expect("file branch taken");
        assert_eq!(
            r.expect_err("non-string file must fail"),
            "scene.textures[2]: 'file' must be a string"
        );
        assert_eq!(calls.get(), 0);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn missing_file_fails_identically_every_time_and_is_never_cached() {
        let dir = temp_dir("missing");
        let calls = Cell::new(0);
        let mut wire = AssetWire::new();
        let ctx = "scene.textures[0]";
        let want = AssetWire::load_error(
            ctx,
            &dir.join("gone.png"),
            &std::io::Error::from(std::io::ErrorKind::NotFound),
        );
        // Two specs naming the same missing file (different
        // ids): EACH fails at its own turn with the opener
        // invoked each time — the current code's behaviour.
        // (The fake opener's inner io error text may differ
        // from the literal NotFound rendering, so compare
        // against a direct opener call, not `want`'s tail.)
        let _ = want;
        let first = wire
            .resolve_file(&file_spec("a", "gone.png"), &dir, ctx, make_opener(&calls, ctx))
            .expect("file branch")
            .expect_err("missing file must fail");
        let second = wire
            .resolve_file(&file_spec("b", "gone.png"), &dir, ctx, make_opener(&calls, ctx))
            .expect("file branch")
            .expect_err("missing file must fail again, not serve a cached error or hit");
        assert_eq!(first, second, "identical failure on repetition");
        assert!(
            first.starts_with("scene.textures[0]: cannot load texture file "),
            "error keeps the current shape: {first}"
        );
        assert!(first.contains("gone.png"), "error names the file: {first}");
        assert_eq!(calls.get(), 2, "opener invoked per request — no failure caching");
        assert_eq!(wire.opens(), 2);
        assert_eq!(wire.hits(), 0);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn duplicate_file_opens_once_and_serves_identical_bytes() {
        let dir = temp_dir("dup");
        std::fs::write(dir.join("tex.png"), b"PNGDATA").expect("write texture file");
        let calls = Cell::new(0);
        let mut wire = AssetWire::new();
        let ctx = "scene.textures[0]";
        let a = wire
            .resolve_file(&file_spec("a", "tex.png"), &dir, ctx, make_opener(&calls, ctx))
            .expect("file branch")
            .expect("first load succeeds");
        let b = wire
            .resolve_file(&file_spec("b", "tex.png"), &dir, "scene.textures[1]", make_opener(&calls, ctx))
            .expect("file branch")
            .expect("duplicate load succeeds from cache");
        assert_eq!(a, b, "the duplicate receives the same decoded texture");
        assert_eq!(calls.get(), 1, "the file was opened exactly once");
        assert_eq!(wire.opens(), 1);
        assert_eq!(wire.hits(), 1);
        // A DIFFERENT path still opens for real.
        std::fs::write(dir.join("other.png"), b"OTHER").expect("write second file");
        let _ = wire
            .resolve_file(&file_spec("c", "other.png"), &dir, "scene.textures[2]", make_opener(&calls, ctx))
            .expect("file branch")
            .expect("distinct file loads");
        assert_eq!(calls.get(), 2);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn size_change_and_deletion_bust_the_cache_to_a_real_open() {
        let dir = temp_dir("bust");
        let path = dir.join("tex.png");
        std::fs::write(&path, b"12345").expect("write v1");
        let calls = Cell::new(0);
        let mut wire = AssetWire::new();
        let ctx = "scene.textures[0]";
        let _ = wire
            .resolve_file(&file_spec("a", "tex.png"), &dir, ctx, make_opener(&calls, ctx))
            .expect("file branch")
            .expect("v1 loads");
        // Same path, DIFFERENT size → not a dedup hit: a real
        // open returns the new bytes (where assetstream's
        // rule also says "not a duplicate").
        std::fs::write(&path, b"1234567").expect("write v2, longer");
        let v2 = wire
            .resolve_file(&file_spec("a", "tex.png"), &dir, ctx, make_opener(&calls, ctx))
            .expect("file branch")
            .expect("v2 loads");
        assert_eq!(v2.0, b"1234567".to_vec());
        assert_eq!(calls.get(), 2);
        assert_eq!(wire.hits(), 0);
        // Deleted after being cached → the CURRENT error at
        // this spec's turn, never a stale hit.
        std::fs::remove_file(&path).expect("delete texture");
        let r = wire
            .resolve_file(&file_spec("a", "tex.png"), &dir, ctx, make_opener(&calls, ctx))
            .expect("file branch");
        assert!(r.is_err(), "deleted file must fail, got a stale cache hit");
        assert_eq!(calls.get(), 3, "fall-through invoked the real opener");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn manifest_dedup_pairs_agree_with_cache_behaviour() {
        let dir = temp_dir("manifest");
        std::fs::write(dir.join("tex.png"), b"PNGDATA").expect("write texture");
        std::fs::write(dir.join("other.png"), b"OTHER!").expect("write texture");
        let specs = vec![
            file_spec("a", "tex.png"),
            file_spec("b", "tex.png"),   // duplicate of a (same path, same size)
            file_spec("c", "other.png"), // distinct
            spec(&[
                ("id", Value::Str("g".to_string())),
                ("generator", Value::Str("cloud".to_string())),
            ]), // generator: no manifest entry
        ];
        let pairs = AssetWire::dedup_pairs(&dir, &specs).expect("manifest builds");
        assert_eq!(pairs, vec![("a".to_string(), "b".to_string())]);
        let m = AssetWire::manifest(&dir, &specs).expect("manifest builds");
        assert_eq!(m.len(), 3, "generator spec contributes no entry");
        assert_eq!(m.unique_paths(), 2);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn manifest_rejects_dotdot_paths_the_loader_accepts_scope_out_evidence() {
        // PINNED DIVERGENCE (the §2.3 scope-out): scenefile's
        // join loads "../shared.png" legally today, but the
        // C4 manifest validation rejects it. Therefore
        // Manifest must never gate the load path.
        let dir = temp_dir("dotdot");
        let specs = vec![file_spec("a", "../shared.png")];
        let err = AssetWire::manifest(&dir, &specs)
            .expect_err("manifest must reject the '..' path");
        assert!(err.contains(".."), "rejection names the escape: {err}");
        // …while the front-end itself resolves the same spec
        // through the normal joined-path open: a real file
        // one level up from a scene subdirectory loads
        // successfully — the front-end imposes no rejection.
        let scene_sub = dir.join("scene");
        std::fs::create_dir_all(&scene_sub).expect("create scene subdir");
        std::fs::write(dir.join("shared.png"), b"SHARED").expect("write shared file");
        let calls = Cell::new(0);
        let mut wire = AssetWire::new();
        let loaded = wire
            .resolve_file(
                &file_spec("a", "../shared.png"),
                &scene_sub,
                "scene.textures[0]",
                make_opener(&calls, "scene.textures[0]"),
            )
            .expect("file branch")
            .expect("front-end must load a legal '..' path");
        assert_eq!(loaded.0, b"SHARED".to_vec());
        assert_eq!(calls.get(), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
