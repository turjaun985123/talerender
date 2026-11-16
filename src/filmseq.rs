//! Generic scene-sequence film driver (REMEDIATION-CONTRACTS §2,
//! Package 1; seat R1-01).
//!
//! The legacy driver (`film::run_film`) stages ONE film from
//! hand-written shot functions in code. This driver stages ANY
//! film whose manifest selects it (`"driver": "scene-sequence"`,
//! contracts §1): the manifest's segments each name a scene-format
//! file, and frame `n` is that segment's scene evaluated at the
//! segment-local time `lt = n/fps - segment.start_s` through the
//! proven generic scene path (`scenefile::load_root` +
//! `scenefile::render_loaded_pixels`, the L0 API). The only
//! film-level overlay the scene path does not own is the caption
//! strip, applied post-render from `FilmData` exactly as the
//! legacy path applies it (`caption_at` below is `eval_frame`'s
//! selection + scale rule, extracted verbatim). NO calendar
//! overlay, NO film fade: if a fade or grade exists it is carried
//! by the scene data itself (contracts §2).
//!
//! Determinism: each segment's scene file is parsed ONCE per run
//! and the parsed root is cloned per frame; the clone's `time`
//! is set to the f32 nearest `lt` (the scenefile loader reads
//! `time` as f32). Same root + same `lt` -> same `Loaded` -> same
//! pixels (the scene path's existing property). The driver reads
//! no wall clock into any pixel (the run clock below is log-only,
//! exactly as in `run_film`) and iterates no HashMap into draw
//! order — segments are a `Vec` in manifest order, frames ascend.
//!
//! PNG / resume semantics mirror `film::run_film` byte-for-byte
//! in shape: `schedwire::resolve_range` + `schedwire::fs_frame_iter`
//! (lazy exists test per frame) + `outwire::frame_path`, the same
//! progress / completion line shapes, the same per-frame timing
//! line gate (`runlog::frame_timing_enabled`).
//!
//! Streaming (`OutputMode::Stream`) FAILS LOUDLY with a named
//! error — see `run_film_seq`'s first statement and NOTE.md
//! (R1-01) §Stream for the evidence. Silence was not an option;
//! a named limitation is (contracts §2).

use std::path::{Path, PathBuf};

use crate::scene::Value;

// ---------------------------------------------------------------------
// Manifest (contracts §0/§1): `<native>/film.json` in the project
// layout, `scenes/film/film.json` in the repo layout. Data, not
// code: title / fps / durationS / totalFrames / segments[] with
// id, scene, lines, startS/endS, startFrame/endFrame and the
// captionChunks {first,last,count} range.
// ---------------------------------------------------------------------

/// One manifest segment: a scene file owning the frame range
/// `[start_frame, end_frame)`.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Segment {
    /// Segment id as authored (`"s01"`, `"s05a"`, ...).
    pub id: String,
    /// The scene path AS WRITTEN in the manifest (kept verbatim
    /// for error messages and telemetry analogues).
    pub scene: String,
    /// `scene` resolved to an existing file at discovery time
    /// (see `discover_manifest` for the resolution order). A
    /// segment that resolves nowhere never reaches this struct —
    /// discovery fails closed naming it.
    pub scene_path: PathBuf,
    /// Narration line indices this segment covers, in order.
    pub lines: Vec<usize>,
    pub start_s: f64,
    pub end_s: f64,
    pub start_frame: u32,
    pub end_frame: u32,
    /// First caption-chunk index of the segment's
    /// `captionChunks` range, if the manifest declares one.
    pub caption_first: Option<usize>,
    /// Last caption-chunk index (inclusive), ditto.
    pub caption_last: Option<usize>,
    /// The segment's set code as authored (`"JG"`, `"BCH"`, …),
    /// for the telemetry shot table; empty when undeclared.
    pub set: String,
    /// The segment's atmosphere preset name as authored;
    /// empty when undeclared.
    pub atmosphere: String,
}

impl Segment {
    /// Does this segment own frame `n`? (Half-open range, the
    /// manifest's own `frameRule`.)
    pub(crate) fn owns(&self, n: u32) -> bool {
        self.start_frame <= n && n < self.end_frame
    }
}

/// A parsed film manifest plus the two directories every later
/// resolution is relative to.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct FilmManifest {
    pub title: String,
    /// Frames per second as authored (30 in both real manifests;
    /// kept f64 — `t = n / fps` is an f64 computation in both
    /// drivers, and a fractional rate must not be rounded away).
    pub fps: f64,
    pub duration_s: f64,
    pub total_frames: u32,
    /// The manifest's `"driver"` field, verbatim. The §1 fork
    /// routes here iff this is `Some("scene-sequence")`.
    pub driver: Option<String>,
    /// Segments in manifest order. Discovery validates that
    /// their frame ranges tile `[0, total_frames)` exactly.
    pub segments: Vec<Segment>,
    /// Directory containing the discovered manifest file.
    pub manifest_dir: PathBuf,
    /// The `--film` argument directory `D`, as passed.
    pub film_dir: PathBuf,
    /// The discovered manifest file itself (for error messages).
    pub manifest_path: PathBuf,
}

impl FilmManifest {
    /// Index of the segment owning frame `n`, if any. Linear
    /// scan over the manifest-ordered `Vec` — deterministic,
    /// and segment counts are tens, not thousands.
    pub(crate) fn segment_index_at(&self, frame: u32) -> Option<usize> {
        self.segments.iter().position(|s| s.owns(frame))
    }

    /// The segment owning frame `n`, if any.
    pub(crate) fn segment_at(&self, frame: u32) -> Option<&Segment> {
        self.segment_index_at(frame).map(|i| &self.segments[i])
    }
}

// ---------------------------------------------------------------------
// Manifest discovery + parsing (contracts §1 steps 1-2)
// ---------------------------------------------------------------------

fn mnum(root: &Value, key: &str, ctx: &str) -> Result<f64, String> {
    root.get(key)
        .and_then(Value::as_f64)
        .ok_or_else(|| format!("{ctx}: missing or non-numeric '{key}'"))
}

fn mu32(v: f64, ctx: &str) -> Result<u32, String> {
    if v.is_finite() && v >= 0.0 && v.fract() == 0.0 && v <= u32::MAX as f64 {
        Ok(v as u32)
    } else {
        Err(format!("{ctx}: expected a non-negative integer, got {v}"))
    }
}

fn musize(v: f64, ctx: &str) -> Result<usize, String> {
    if v.is_finite() && v >= 0.0 && v.fract() == 0.0 && v <= usize::MAX as f64 {
        Ok(v as usize)
    } else {
        Err(format!("{ctx}: expected a non-negative integer, got {v}"))
    }
}

/// Resolve one segment's scene path against, in order
/// (contracts §1 step 2): the manifest's directory, `D/..`,
/// `D/../..`. First existing file wins; a segment that resolves
/// nowhere is a named load error (fail closed).
fn resolve_scene(
    manifest_dir: &Path,
    film_dir: &Path,
    scene: &str,
    ctx: &str,
) -> Result<PathBuf, String> {
    let bases: [PathBuf; 3] = [
        manifest_dir.to_path_buf(),
        film_dir.join(".."),
        film_dir.join("../.."),
    ];
    for base in &bases {
        let cand = base.join(scene);
        if cand.is_file() {
            return Ok(cand);
        }
    }
    let tried: Vec<String> = bases.iter().map(|b| b.join(scene).display().to_string()).collect();
    Err(format!(
        "{ctx}: scene {scene:?} resolved nowhere (tried {})",
        tried.join(", ")
    ))
}

fn parse_manifest(
    text: &str,
    manifest_path: &Path,
    film_dir: &Path,
) -> Result<FilmManifest, String> {
    let ctx = format!("film manifest {}", manifest_path.display());
    let root = crate::scene::parse_json(text).map_err(|e| format!("{ctx}: {e}"))?;
    if !matches!(root, Value::Obj(_)) {
        return Err(format!("{ctx}: top level must be a JSON object"));
    }
    let title = root
        .get("title")
        .and_then(Value::as_str)
        .ok_or_else(|| format!("{ctx}: missing or non-string 'title'"))?
        .to_string();
    let fps = mnum(&root, "fps", &ctx)?;
    if !(fps > 0.0) || !fps.is_finite() {
        return Err(format!("{ctx}: 'fps' must be a positive finite number, got {fps}"));
    }
    let duration_s = mnum(&root, "durationS", &ctx)?;
    if !duration_s.is_finite() || duration_s < 0.0 {
        return Err(format!("{ctx}: 'durationS' must be a non-negative finite number, got {duration_s}"));
    }
    let total_frames = mu32(mnum(&root, "totalFrames", &ctx)?, &format!("{ctx}.totalFrames"))?;
    if total_frames == 0 {
        return Err(format!("{ctx}: 'totalFrames' is 0 — a film of no frames"));
    }
    let driver = match root.get("driver") {
        None => None,
        Some(v) => Some(
            v.as_str()
                .ok_or_else(|| format!("{ctx}: 'driver' must be a string"))?
                .to_string(),
        ),
    };
    let manifest_dir = manifest_path
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));
    let seg_values = root
        .get("segments")
        .and_then(Value::as_arr)
        .ok_or_else(|| format!("{ctx}: missing or non-array 'segments'"))?;
    if seg_values.is_empty() {
        return Err(format!("{ctx}: 'segments' is empty"));
    }
    let mut segments: Vec<Segment> = Vec::with_capacity(seg_values.len());
    for (i, sv) in seg_values.iter().enumerate() {
        let sctx = format!("{ctx}.segments[{i}]");
        let id = sv
            .get("id")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("{sctx}: missing or non-string 'id'"))?
            .to_string();
        let sctx = format!("{ctx}.segments[{i}] ({id})");
        let scene = sv
            .get("scene")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("{sctx}: missing or non-string 'scene'"))?
            .to_string();
        let scene_path = resolve_scene(&manifest_dir, film_dir, &scene, &sctx)?;
        let mut lines: Vec<usize> = Vec::new();
        if let Some(lv) = sv.get("lines") {
            let arr = lv
                .as_arr()
                .ok_or_else(|| format!("{sctx}: 'lines' must be an array"))?;
            for (j, l) in arr.iter().enumerate() {
                let n = l
                    .as_f64()
                    .ok_or_else(|| format!("{sctx}.lines[{j}]: expected a number"))?;
                lines.push(musize(n, &format!("{sctx}.lines[{j}]"))?);
            }
        }
        let start_s = mnum(sv, "startS", &sctx)?;
        let end_s = mnum(sv, "endS", &sctx)?;
        if !(end_s > start_s) {
            return Err(format!("{sctx}: endS ({end_s}) must exceed startS ({start_s})"));
        }
        let start_frame = mu32(mnum(sv, "startFrame", &sctx)?, &format!("{sctx}.startFrame"))?;
        let end_frame = mu32(mnum(sv, "endFrame", &sctx)?, &format!("{sctx}.endFrame"))?;
        if end_frame <= start_frame {
            return Err(format!(
                "{sctx}: endFrame ({end_frame}) must exceed startFrame ({start_frame})"
            ));
        }
        let (caption_first, caption_last) = match sv.get("captionChunks") {
            None => (None, None),
            Some(cc) => {
                let first = mu32(mnum(cc, "first", &sctx)?, &format!("{sctx}.captionChunks.first"))? as usize;
                let last = mu32(mnum(cc, "last", &sctx)?, &format!("{sctx}.captionChunks.last"))? as usize;
                if last < first {
                    return Err(format!(
                        "{sctx}: captionChunks.last ({last}) precedes first ({first})"
                    ));
                }
                (Some(first), Some(last))
            }
        };
        let set = sv
            .get("set")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let atmosphere = sv
            .get("atmosphere")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        segments.push(Segment {
            id,
            scene,
            scene_path,
            lines,
            start_s,
            end_s,
            start_frame,
            end_frame,
            caption_first,
            caption_last,
            set,
            atmosphere,
        });
    }
    // Tiling law (contracts §2: "manifest frame ranges tile
    // [0, totalFrames)"). Validated once here so the driver's
    // per-frame segment lookup can fail closed on a range the
    // manifest never claimed instead of guessing.
    if segments[0].start_frame != 0 {
        return Err(format!(
            "{ctx}: segments do not tile [0, {total_frames}): first segment {} starts at frame {}",
            segments[0].id, segments[0].start_frame
        ));
    }
    for w in segments.windows(2) {
        if w[1].start_frame != w[0].end_frame {
            return Err(format!(
                "{ctx}: segments do not tile [0, {total_frames}): segment {} ends at frame {} but segment {} starts at frame {}",
                w[0].id, w[0].end_frame, w[1].id, w[1].start_frame
            ));
        }
    }
    let last = &segments[segments.len() - 1];
    if last.end_frame != total_frames {
        return Err(format!(
            "{ctx}: segments do not tile [0, {total_frames}): last segment {} ends at frame {}",
            last.id, last.end_frame
        ));
    }
    Ok(FilmManifest {
        title,
        fps,
        duration_s,
        total_frames,
        driver,
        segments,
        manifest_dir,
        film_dir: film_dir.to_path_buf(),
        manifest_path: manifest_path.to_path_buf(),
    })
}

/// Discover the manifest for film dir `D` (the `--film`
/// argument), in the contracts §1 order:
///
/// 1. `D/../film.json`            (project layout: `<native>/film.json`,
///                                 `D = <native>/film`)
/// 2. `D/film.json`               (manifest inside the film dir itself)
/// 3. `D/../scenes/film/film.json` (repo layout: `scenes/film/film.json`,
///                                 `D = <repo>/film`)
///
/// `Ok(None)` = no manifest at any candidate (the §1 fork then
/// takes the legacy path). `Err` = a manifest WAS found but is
/// unreadable, unparseable, malformed, fails the tiling law, or
/// names a scene that resolves nowhere — era pairing fails
/// closed, never silently falls back to the legacy driver.
pub(crate) fn discover_manifest(film_dir: &Path) -> Result<Option<FilmManifest>, String> {
    let candidates: [PathBuf; 3] = [
        film_dir.join("../film.json"),
        film_dir.join("film.json"),
        film_dir.join("../scenes/film/film.json"),
    ];
    for cand in &candidates {
        if cand.is_file() {
            let text = std::fs::read_to_string(cand)
                .map_err(|e| format!("cannot read film manifest {}: {e}", cand.display()))?;
            let manifest = parse_manifest(&text, cand, film_dir)?;
            return Ok(Some(manifest));
        }
    }
    Ok(None)
}

// ---------------------------------------------------------------------
// Caption selection (extracted from film::eval_frame)
// ---------------------------------------------------------------------

/// The caption active at film time `t`, and its overlay scale —
/// `film::eval_frame`'s rule (film.rs, the `f.caption` block at
/// the end of `eval_frame`), extracted verbatim:
///
/// ```text
/// if let Some((ci, c)) = data.captions.iter().enumerate().find(|(_, c)| t >= c.s && t < c.e) {
///     let lt = t - c.s;
///     let sc = if lt < 0.1 { lerp(0.86, 1.0, eob(seg(lt, 0.0, 0.1))) } else { 1.0 };
///     f.caption = Some(ci);
///     f.caption_scale = (sc * c.fit) as f32;
/// }
/// ```
///
/// So: the FIRST caption with `s <= t < e` wins (captions are
/// stored in time order; the half-open interval means a `t`
/// exactly at a caption's `e` belongs to the next caption, or
/// to none). The scale `sc` is an ease-out-back pop-in over the
/// caption's first 0.1 s, from 0.86 to 1.0 (overshooting above
/// 1.0 mid-way — `eob` is easeOutBack), and exactly 1.0 from
/// `lt = 0.1` on. The returned scale is `caption_scale` itself:
/// `sc * fit`, with `fit` the caption's own declared fit,
/// cast to f32 exactly where `eval_frame` casts it.
pub(crate) fn caption_at(data: &crate::film::FilmData, t: f64) -> Option<(usize, f32)> {
    let (ci, c) = data
        .captions
        .iter()
        .enumerate()
        .find(|(_, c)| t >= c.s && t < c.e)?;
    let lt = t - c.s;
    let sc = if lt < 0.1 {
        crate::film::lerp(0.86, 1.0, crate::film::eob(crate::film::seg(lt, 0.0, 0.1)))
    } else {
        1.0
    };
    Some((ci, (sc * c.fit) as f32))
}

// ---------------------------------------------------------------------
// The driver
// ---------------------------------------------------------------------

/// Set a parsed scene root's top-level `time` to `lt`, with the
/// f32 law of contracts §2: the value written is the f32 nearest
/// `lt` (stored back as f64, exactly the number the loader's
/// `num(...) as f32` read would produce). Mirrors the semantics
/// of `eval15`'s private `obj_set` (replace in place, else
/// append) — key order in the object is otherwise untouched.
fn set_root_time(root: &mut Value, lt: f32) {
    let val = Value::Num(lt as f64);
    if let Value::Obj(pairs) = root {
        for p in pairs.iter_mut() {
            if p.0 == "time" {
                p.1 = val;
                return;
            }
        }
        pairs.push(("time".to_string(), val));
    }
}

/// `fps` for the banner: integral rates print as integers
/// (`30`), as `run_film`'s hardcoded `@30fps` does; a fractional
/// rate prints as authored rather than being rounded in the log.
fn fmt_fps(fps: f64) -> String {
    if fps.fract() == 0.0 && fps.abs() <= 9_007_199_254_740_992.0 {
        format!("{}", fps as i64)
    } else {
        format!("{fps}")
    }
}

/// The generic scene-sequence film driver. Signature and
/// semantics mirror `film::run_film` (same argument order and
/// roles), with the manifest — discovered and routed by the §1
/// fork in `run_film` BEFORE any film data is loaded — in place
/// of a film-dir string, and `data_dir` carrying that string's
/// role (`load_film_data` / `load_assets` read from it, exactly
/// as the legacy path reads from its `film_dir`).
///
/// `adapter_policy` is accepted for signature parity and NOT
/// consumed: the scene path's device construction
/// (`scenefile::make_device`) takes no policy argument today —
/// see NOTE.md (R1-01) §Visibility/consumption. Likewise
/// `queue_capacity` is only meaningful to the stream branch,
/// which this driver refuses (below).
#[allow(clippy::too_many_arguments)]
pub(crate) async fn run_film_seq(
    width: u32,
    height: u32,
    manifest: &FilmManifest,
    data_dir: &Path,
    out_dir: &str,
    from: u32,
    to: u32,
    knobs: crate::Knobs,
    adapter_policy: &str,
    output_mode: crate::streammodes::OutputMode,
    queue_capacity: usize,
) -> crate::errors::TaleResult<()> {
    // Stream mode: named refusal, FIRST, before any data load,
    // scene parse or directory creation — a stream request on
    // this driver can never half-start. Evidence and rationale:
    // NOTE.md (R1-01) §Stream. (An `Auto` that reaches here has
    // already been resolved by main's mode resolution, exactly
    // as in `run_film`; anything that is not `Stream` takes the
    // PNG path, as there.)
    if output_mode == crate::streammodes::OutputMode::Stream {
        return Err(crate::errors::TaleError::render(
            "filmseq stream mode",
            format!(
                "OutputMode::Stream is not supported by the scene-sequence driver \
                 (film {:?}, manifest {}): the streaming session's producer contract \
                 (streamsession::FrameProducer) is synchronous, while the generic \
                 scene renderer (scenefile::render_loaded_pixels) is async and \
                 builds its device per call — the legacy stream branch works only \
                 because film::Renderer::render is synchronous. Use PNG output \
                 mode (the adapter's video mode is disk frames + finish, so no \
                 adapter leg needs stream); queue_capacity {queue_capacity} unused",
                manifest.title,
                manifest.manifest_path.display()
            ),
        ));
    }
    let _ = adapter_policy; // parity only — see the doc comment above.

    let tag = if knobs.is_draft() {
        format!(" {}", knobs.tag())
    } else {
        String::new()
    };
    let data = crate::film::load_film_data(data_dir)
        .map_err(|e| crate::errors::TaleError::data_load(data_dir, e))?;
    let assets = crate::film::load_assets(data_dir, &data)
        .map_err(|e| crate::errors::TaleError::data_load(data_dir, e))?;
    let (from, to) = crate::schedwire::resolve_range(from, to, manifest.total_frames);

    // Parse each segment's scene file ONCE for the whole run.
    // The Vec is index-aligned with `manifest.segments`; the
    // per-frame work below clones one entry, never re-reads or
    // re-parses, and never touches the filesystem again for
    // scene data.
    let mut roots: Vec<Value> = Vec::with_capacity(manifest.segments.len());
    for seg in &manifest.segments {
        let text = std::fs::read_to_string(&seg.scene_path).map_err(|e| {
            crate::errors::TaleError::data_load(
                &seg.scene_path,
                format!("cannot read scene file for segment {}: {e}", seg.id),
            )
        })?;
        let root = crate::scene::parse_json(&text).map_err(|e| {
            crate::errors::TaleError::data_load(
                &seg.scene_path,
                format!("segment {}: {e}", seg.id),
            )
        })?;
        roots.push(root);
    }

    std::fs::create_dir_all(out_dir).map_err(|e| crate::errors::TaleError::image_io(out_dir, e))?;
    println!(
        "film: {} — {} frames @{}fps, rendering {}..{} at {}x{} (clean PNG frames; decision B){tag}",
        manifest.title,
        manifest.total_frames,
        fmt_fps(manifest.fps),
        from,
        to,
        width,
        height
    );
    let clock = crate::runlog::RunClock::new();
    let timing = crate::runlog::frame_timing_enabled();
    // Resume semantics identical to run_film's: the schedwire
    // iterator's exists test runs lazily at each frame's turn,
    // and outwire::frame_path keeps the path spelling.
    let mut frames = crate::schedwire::fs_frame_iter(from, to, out_dir);
    while let Some(step) = frames.next() {
        let n = step.index();
        if let crate::schedwire::Step::Skip(_) = step {
            continue;
        }
        let path = crate::outwire::frame_path(out_dir, n);
        let frame_start = std::time::Instant::now();
        let t = n as f64 / manifest.fps;
        let seg_idx = manifest.segment_index_at(n).ok_or_else(|| {
            crate::errors::TaleError::data_load(
                &manifest.manifest_path,
                format!(
                    "frame {n} (t = {t:.3}s) is not owned by any segment of the manifest \
                     (segments tile [0, {}); the resolved range was {from}..{to})",
                    manifest.total_frames
                ),
            )
        })?;
        let seg = &manifest.segments[seg_idx];
        let lt = t - seg.start_s;
        // Clone the segment's parsed root and evaluate it at the
        // segment-local time. The clone is the ONLY per-frame
        // scene-data cost besides the render itself; the cached
        // root is never mutated.
        let mut root = roots[seg_idx].clone();
        set_root_time(&mut root, lt as f32);
        let label = seg.scene_path.display().to_string();
        let loaded = crate::scenefile::load_root(root, &label)
            .map_err(|e| crate::errors::TaleError::data_load(&seg.scene_path, e))?;
        let mut px = crate::scenefile::render_loaded_pixels(width, height, &loaded, knobs).await?;
        // Post-render film-level overlay, in the film path's
        // order position for captions (after the shot draw):
        // caption only — no calendar overlay, no fade (see the
        // module header).
        if let Some((ci, scale)) = caption_at(&data, t) {
            crate::film::overlay_caption(&mut px, width, height, &assets, ci, scale);
        }
        let img = image::RgbaImage::from_raw(width, height, px).expect("pixel buffer size mismatch");
        img.save(&path).map_err(|e| crate::errors::TaleError::image_io(&path, e))?;
        let progress_due = frames.note_rendered();
        if timing {
            println!(
                "frame-time: frame={n} ms={:.2}",
                frame_start.elapsed().as_secs_f64() * 1000.0
            );
        }
        if progress_due {
            println!(
                "  frame {n} (t = {t:.2}s) — {} rendered, {} skipped · {}{tag}",
                frames.done(),
                frames.skipped(),
                clock.timing_suffix(frames.progress_numerator(), frames.total() as u64)
            );
        }
    }
    // Completion line mirrors run_film's shape minus the two
    // Renderer-owned tails the generic path does not have (the
    // paint-cache stats segment and the GPU mem-report block):
    // the scene renderer owns no PaintCache and no persistent
    // Renderer — see NOTE.md (R1-01) §Deviations.
    println!(
        "film render complete: {} rendered, {} skipped (resume), range {from}..{to} · {}{tag}",
        frames.done(),
        frames.skipped(),
        clock.summary_suffix()
    );
    Ok(())
}

// ---------------------------------------------------------------------
// Tests. CPU-only: manifest discovery/parsing against the two
// real manifests on disk, segment tiling, and caption_at against
// the two real film-data files. The Lighthouse Cat project lives
// OUTSIDE this repo (a user project dir), so its tests skip —
// loudly, via eprintln — when the path is absent (e.g. on CI),
// instead of failing a checkout that cannot contain it. The
// repo-layout tests never skip: those files ship with the repo.
// ---------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// Repo root as the test process sees it: cargo runs unit
    /// tests with the crate root as the working directory (the
    /// same assumption film/telemetry tests already make, e.g.
    /// `--film film` in the docs).
    fn repo_film_dir() -> PathBuf {
        PathBuf::from("film")
    }

    fn lc_native_dir() -> PathBuf {
        PathBuf::from("/home/hatch/faceless-shorts/the-lighthouse-cats-last-shift/native")
    }

    // ---- discovery: both real layouts -------------------------

    #[test]
    fn discover_repo_layout_manifest() {
        // D = <repo>/film: candidates 1 (../film.json) and 2
        // (film/film.json) do not exist; candidate 3
        // (../scenes/film/film.json) is the repo manifest.
        let m = discover_manifest(&repo_film_dir())
            .expect("repo manifest must parse")
            .expect("repo manifest must be found");
        assert_eq!(m.title, "THE SOLDIER WHO DIDN'T KNOW THE WAR WAS OVER");
        assert_eq!(m.fps, 30.0);
        assert_eq!(m.duration_s, 60.357);
        assert_eq!(m.total_frames, 1811);
        assert_eq!(m.driver, None, "the Onoda manifest carries no driver field (contracts §1.3)");
        assert_eq!(m.segments.len(), 14);
        assert_eq!(m.manifest_dir, PathBuf::from("film/../scenes/film"));
        // Scene resolution: manifest_dir (scenes/film) joined
        // with the repo-relative scene path does NOT exist;
        // D/.. (the repo root) wins — the second base.
        assert_eq!(m.segments[0].id, "s01");
        assert_eq!(m.segments[0].scene, "scenes/film/s01.scene.json");
        assert_eq!(
            m.segments[0].scene_path,
            PathBuf::from("film/../scenes/film/s01.scene.json")
        );
        assert_eq!(m.segments[0].lines, vec![0]);
        assert_eq!(m.segments[0].caption_first, Some(0));
        assert_eq!(m.segments[0].caption_last, Some(12));
        let last = &m.segments[m.segments.len() - 1];
        assert_eq!(last.id, "s12");
        assert_eq!(last.caption_first, Some(68));
        assert_eq!(last.caption_last, Some(72));
        for seg in &m.segments {
            assert!(seg.scene_path.is_file(), "segment {} scene must exist", seg.id);
        }
    }

    #[test]
    fn discover_lighthouse_cat_manifest() {
        let native = lc_native_dir();
        if !native.is_dir() {
            eprintln!("SKIP: Lighthouse Cat project not present at {}", native.display());
            return;
        }
        // D = <native>/film: candidate 1 (D/../film.json =
        // <native>/film.json) is the project manifest.
        let m = discover_manifest(&native.join("film"))
            .expect("LC manifest must parse")
            .expect("LC manifest must be found");
        assert_eq!(m.title, "The Lighthouse Cat's Last Shift");
        assert_eq!(m.fps, 30.0);
        assert_eq!(m.duration_s, 39.049);
        assert_eq!(m.total_frames, 1172);
        assert_eq!(m.driver, None, "the CURRENT (pre-re-bridge) LC manifest has no driver field");
        assert_eq!(m.segments.len(), 11);
        // manifest_dir is the discovered path's parent, kept
        // lexical (no canonicalisation, mirroring run_film's
        // path spellings): <native>/film/.. for candidate 1.
        assert_eq!(m.manifest_dir, native.join("film/.."));
        // Scene resolution: the manifest's own directory wins —
        // <native>/scenes/film/sNN.scene.json via that first
        // base, spelled lexically through film/.. (the OS
        // resolves the `..` on access; the file is the same).
        assert_eq!(
            m.segments[0].scene_path,
            native.join("film/../scenes/film/s01.scene.json")
        );
        assert!(m.segments[0].scene_path.is_file());
        assert_eq!(m.segments[0].caption_first, Some(0));
        assert_eq!(m.segments[0].caption_last, Some(3));
        let last = &m.segments[m.segments.len() - 1];
        assert_eq!(last.id, "s11");
        assert_eq!(last.caption_first, Some(39));
        assert_eq!(last.caption_last, Some(42));
        for seg in &m.segments {
            assert!(seg.scene_path.is_file(), "segment {} scene must exist", seg.id);
        }
    }

    #[test]
    fn discover_no_manifest_is_none() {
        let dir = std::env::temp_dir().join("filmseq-test-empty-2243f259");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        assert_eq!(discover_manifest(&dir).unwrap(), None);
        let _ = fs::remove_dir_all(&dir);
    }

    // ---- discovery: synthetic layouts, order + failure modes --

    struct Scratch {
        root: PathBuf,
    }

    impl Scratch {
        fn new(name: &str) -> Scratch {
            let root = std::env::temp_dir().join(format!("filmseq-test-{name}-2243f259"));
            let _ = fs::remove_dir_all(&root);
            fs::create_dir_all(&root).unwrap();
            Scratch { root }
        }
        fn write(&self, rel: &str, text: &str) -> PathBuf {
            let p = self.root.join(rel);
            fs::create_dir_all(p.parent().unwrap()).unwrap();
            fs::write(&p, text).unwrap();
            p
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    /// A minimal valid manifest: two segments tiling [0, 20)
    /// at 10 fps, scenes at the repo-relative spelling.
    fn manifest_json(title: &str, driver: Option<&str>) -> String {
        let driver_line = match driver {
            Some(d) => format!("\"driver\": \"{d}\","),
            None => String::new(),
        };
        format!(
            r#"{{ "format": "talerender-film", "version": 1, "title": "{title}", {driver_line}
                 "fps": 10, "durationS": 2.0, "totalFrames": 20,
                 "segments": [
                   {{ "id": "s01", "scene": "scenes/film/s01.scene.json", "lines": [0],
                      "startS": 0.0, "endS": 1.0, "startFrame": 0, "endFrame": 10,
                      "captionChunks": {{ "first": 0, "last": 1, "count": 2 }} }},
                   {{ "id": "s02", "scene": "scenes/film/s02.scene.json", "lines": [1],
                      "startS": 1.0, "endS": 2.0, "startFrame": 10, "endFrame": 20,
                      "captionChunks": {{ "first": 2, "last": 3, "count": 2 }} }}
                 ] }}"#
        )
    }

    #[test]
    fn discover_order_first_candidate_wins() {
        let s = Scratch::new("order");
        // D = <root>/proj/film. All three candidates exist,
        // with different titles; §1 order must pick D/../film.json.
        s.write("proj/film/dummy.txt", "");
        s.write("proj/scenes/film/s01.scene.json", "{}");
        s.write("proj/scenes/film/s02.scene.json", "{}");
        s.write("proj/film.json", &manifest_json("FIRST", Some("scene-sequence")));
        s.write("proj/film/film.json", &manifest_json("SECOND", None));
        s.write("proj/scenes/film/film.json", &manifest_json("THIRD", None));
        let m = discover_manifest(&s.root.join("proj/film")).unwrap().unwrap();
        assert_eq!(m.title, "FIRST");
        assert_eq!(m.driver.as_deref(), Some("scene-sequence"));
        // Remove candidate 1: candidate 2 wins. Then 3.
        fs::remove_file(s.root.join("proj/film.json")).unwrap();
        let m = discover_manifest(&s.root.join("proj/film")).unwrap().unwrap();
        assert_eq!(m.title, "SECOND");
        fs::remove_file(s.root.join("proj/film/film.json")).unwrap();
        let m = discover_manifest(&s.root.join("proj/film")).unwrap().unwrap();
        assert_eq!(m.title, "THIRD");
    }

    #[test]
    fn discover_unresolved_scene_is_named_error() {
        let s = Scratch::new("unresolved");
        s.write("proj/film/dummy.txt", "");
        // No scene files anywhere: discovery must fail closed
        // naming the segment and the scene, never return a
        // manifest with a dangling path.
        s.write("proj/film.json", &manifest_json("BROKEN", Some("scene-sequence")));
        let err = discover_manifest(&s.root.join("proj/film")).unwrap_err();
        assert!(err.contains("s01"), "error must name the segment: {err}");
        assert!(err.contains("scenes/film/s01.scene.json"), "error must name the scene: {err}");
        assert!(err.contains("resolved nowhere"), "error must say why: {err}");
    }

    #[test]
    fn discover_non_tiling_manifest_is_named_error() {
        let s = Scratch::new("tiling");
        s.write("proj/film/dummy.txt", "");
        s.write("proj/scenes/film/s01.scene.json", "{}");
        s.write("proj/scenes/film/s02.scene.json", "{}");
        // Second segment starts at 11, not 10: a one-frame gap.
        let bad = manifest_json("GAPPED", None).replace("\"startFrame\": 10", "\"startFrame\": 11");
        s.write("proj/film.json", &bad);
        let err = discover_manifest(&s.root.join("proj/film")).unwrap_err();
        assert!(err.contains("do not tile"), "error must name the tiling law: {err}");
    }

    #[test]
    fn discover_malformed_manifest_is_error_not_none() {
        let s = Scratch::new("malformed");
        s.write("proj/film/dummy.txt", "");
        s.write("proj/film.json", "{ not json");
        // A FOUND but broken manifest must not silently fall
        // back to "no manifest" (era pairing fails closed).
        assert!(discover_manifest(&s.root.join("proj/film")).is_err());
    }

    // ---- segment_at tiling ------------------------------------

    fn assert_tiling(m: &FilmManifest) {
        assert_eq!(m.segment_at(0).unwrap().start_frame, 0);
        for n in 0..m.total_frames {
            let seg = m
                .segment_at(n)
                .unwrap_or_else(|| panic!("frame {n} of {} must be owned", m.total_frames));
            assert!(seg.owns(n));
            // Ownership is by frame range, and consecutive
            // frames inside a segment keep the same owner.
            assert_eq!(m.segment_index_at(n), m.segments.iter().position(|s| s.id == seg.id));
        }
        assert_eq!(m.segment_at(m.total_frames), None);
        // Boundary spot-checks across the whole film: the frame
        // at each segment's start belongs to it; the frame just
        // before belongs to its predecessor.
        for (i, seg) in m.segments.iter().enumerate() {
            assert_eq!(m.segment_at(seg.start_frame).unwrap().id, seg.id);
            assert_eq!(m.segment_at(seg.end_frame - 1).unwrap().id, seg.id);
            if i > 0 {
                assert_eq!(m.segment_at(seg.start_frame - 1).unwrap().id, m.segments[i - 1].id);
            }
        }
    }

    #[test]
    fn segment_at_tiles_repo_film() {
        let m = discover_manifest(&repo_film_dir()).unwrap().unwrap();
        assert_tiling(&m);
        assert_eq!(m.segment_at(0).unwrap().id, "s01");
        assert_eq!(m.segment_at(177).unwrap().id, "s01");
        assert_eq!(m.segment_at(178).unwrap().id, "s02");
        assert_eq!(m.segment_at(1810).unwrap().id, "s12");
    }

    #[test]
    fn segment_at_tiles_lighthouse_cat() {
        let native = lc_native_dir();
        if !native.is_dir() {
            eprintln!("SKIP: Lighthouse Cat project not present at {}", native.display());
            return;
        }
        let m = discover_manifest(&native.join("film")).unwrap().unwrap();
        assert_tiling(&m);
        assert_eq!(m.segment_at(0).unwrap().id, "s01");
        assert_eq!(m.segment_at(137).unwrap().id, "s01");
        assert_eq!(m.segment_at(138).unwrap().id, "s02");
        assert_eq!(m.segment_at(1171).unwrap().id, "s11");
    }

    // ---- caption_at against the real film data ----------------

    #[test]
    fn caption_at_onoda_matches_eval_frame_rule() {
        let data = crate::film::load_film_data(&repo_film_dir()).expect("Onoda film data loads");
        assert_eq!(data.captions.len(), 73);
        // Before the first caption (s = 0.27): nothing.
        assert_eq!(caption_at(&data, 0.0), None);
        assert_eq!(caption_at(&data, 0.26), None);
        // At the caption's first instant: lt = 0, eob(0) = 0,
        // so sc = 0.86 and scale = 0.86 * fit (fit = 1.0).
        let (ci, scale) = caption_at(&data, data.captions[0].s).unwrap();
        assert_eq!(ci, 0);
        assert!((scale - 0.86).abs() < 1e-6, "scale at lt=0: {scale}");
        // Mid pop-in (lt = 0.05): eob(0.5) overshoots —
        // sc = 0.86 + 0.14 * 1.0876975 = 1.01227765 (computed
        // independently of the crate's easing code).
        let (ci, scale) = caption_at(&data, data.captions[0].s + 0.05).unwrap();
        assert_eq!(ci, 0);
        assert!((scale - 1.01227765).abs() < 1e-5, "scale at lt=0.05: {scale}");
        // From lt = 0.1 on, sc = 1.0: scale == fit. (Tolerance,
        // not exact equality: (s + 0.1) - s in f64 may land an
        // ulp either side of the 0.1 branch boundary, and the
        // easing branch is continuous there — eob(1) = 1 — so
        // either side gives fit to within float noise.)
        // Caption 0 ("This", 0.27..0.43) is only 0.16 s long —
        // too short for the +0.1/+0.2 probes — so these run on
        // caption 1 (0.43..0.93). (Lead fix at integration: the
        // seat's original probes used caption 0 throughout and
        // the +0.2 probe lands in caption 1's window.)
        let c1 = &data.captions[1];
        let (ci, scale) = caption_at(&data, c1.s + 0.1).unwrap();
        assert_eq!(ci, 1);
        assert!((scale - c1.fit as f32).abs() < 1e-6, "scale at lt=0.1: {scale}");
        // Well past the pop-in the else branch is exact.
        let (ci, scale) = caption_at(&data, c1.s + 0.2).unwrap();
        assert_eq!(ci, 1);
        assert_eq!(scale, c1.fit as f32);
        // Half-open interval: t == e of the LAST caption is
        // outside it, and there is no next caption.
        let last = &data.captions[data.captions.len() - 1];
        assert_eq!(caption_at(&data, last.e), None);
        assert_eq!(caption_at(&data, last.e + 10.0), None);
        // A later caption, well past its pop-in: index + fit.
        let c10 = &data.captions[10];
        let (ci, scale) = caption_at(&data, c10.s + 0.2).unwrap();
        assert_eq!(ci, 10);
        assert_eq!(scale, c10.fit as f32);
    }

    #[test]
    fn caption_at_onoda_agrees_with_eval_frame_everywhere() {
        // The extraction is only worth anything if it agrees
        // with the original, in place, at every time — sample
        // the whole film at the telemetry cadence (0.25 s) plus
        // every caption boundary neighbourhood, and compare
        // against eval_frame's own caption fields.
        let data = crate::film::load_film_data(&repo_film_dir()).expect("Onoda film data loads");
        let looks = crate::film::look_registry();
        let mut times: Vec<f64> = Vec::new();
        let mut t = 0.0;
        while t < data.duration {
            times.push(t);
            t += 0.25;
        }
        for c in &data.captions {
            for dt in [-0.001, 0.0, 0.05, 0.099, 0.1, 0.101] {
                let tt = c.s + dt;
                if tt >= 0.0 && tt < data.duration {
                    times.push(tt);
                }
            }
            let te = c.e - 0.001;
            if te >= 0.0 && te < data.duration {
                times.push(te);
            }
        }
        for t in times {
            let spec = crate::film::eval_frame(&data, &looks, t);
            match (spec.caption, caption_at(&data, t)) {
                (None, None) => {}
                (Some(ci), Some((cj, scale))) => {
                    assert_eq!(ci, cj, "caption index at t={t}");
                    assert!(
                        (spec.caption_scale - scale).abs() < 1e-6,
                        "caption scale at t={t}: eval_frame {} vs caption_at {scale}",
                        spec.caption_scale
                    );
                }
                (a, b) => panic!("caption presence at t={t}: eval_frame {a:?} vs caption_at {b:?}"),
            }
        }
    }

    #[test]
    fn caption_at_lighthouse_cat() {
        let native = lc_native_dir();
        if !native.is_dir() {
            eprintln!("SKIP: Lighthouse Cat project not present at {}", native.display());
            return;
        }
        let data = crate::film::load_film_data(&native.join("film")).expect("LC film data loads");
        assert_eq!(data.captions.len(), 43);
        assert_eq!(caption_at(&data, 0.0), None);
        let c0 = &data.captions[0];
        let (ci, scale) = caption_at(&data, c0.s).unwrap();
        assert_eq!(ci, 0);
        assert!((scale - 0.86 * c0.fit as f32).abs() < 1e-6, "LC scale at lt=0: {scale}");
        let (ci, scale) = caption_at(&data, c0.s + 0.2).unwrap();
        assert_eq!(ci, 0);
        assert_eq!(scale, c0.fit as f32);
        let last = &data.captions[data.captions.len() - 1];
        assert_eq!(caption_at(&data, last.e), None);
        // Every caption is reachable at its own midpoint.
        for (i, c) in data.captions.iter().enumerate() {
            let mid = (c.s + c.e) / 2.0;
            assert_eq!(caption_at(&data, mid).map(|(ci, _)| ci), Some(i), "caption {i} at its midpoint");
        }
    }

    // ---- root time --------------------------------------------

    #[test]
    fn set_root_time_writes_f32_nearest() {
        let mut root = crate::scene::parse_json(r#"{"format": "talerender-scene", "time": 0.0}"#).unwrap();
        set_root_time(&mut root, 0.1f32);
        // The stored f64 is exactly the f32 value widened —
        // not the f64 nearest 0.1.
        assert_eq!(root.get("time").and_then(Value::as_f64), Some(0.1f32 as f64));
        // A root without a time field gains one (appended).
        let mut root = crate::scene::parse_json(r#"{"format": "talerender-scene"}"#).unwrap();
        set_root_time(&mut root, 2.5f32);
        assert_eq!(root.get("time").and_then(Value::as_f64), Some(2.5));
    }
}
