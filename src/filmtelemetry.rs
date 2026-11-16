//! Generic film telemetry (REMEDIATION-CONTRACTS §3, seat R1-02):
//! the M6 audit semantics of `film::run_film_telemetry`, rebuilt
//! from the film's OWN data — the scene-sequence manifest
//! (`crate::filmseq::FilmManifest`) plus `film::FilmData` — so any
//! bridged film can be gated, not only the Onoda staging.
//!
//! What is shared with the legacy gate, by construction:
//! - the audit engine itself (`telemetry::Audit`, `print_audit` /
//!   `print_continuity` / `print_composition`), fed `Cutout`s built
//!   from the same painter (`toonfull::paint_toon_full`), the same
//!   canvas-overflow scan (`telemetry::scan_canvas_overflow`,
//!   res 2.0), the same pose summary and look signature, and the
//!   same plane projection (`telemetry::project_box`);
//! - the strict caption audit (`telemetry::print_caption_audit`,
//!   called with `strict = true` exactly as film.rs calls it), with
//!   rendered widths measured by the legacy estimator: the ink
//!   bounding box (alpha > 32) of the film's own rendered caption
//!   strip PNGs, `raw_w = rendered_w / fit`;
//! - the sampling law (0.25 s over the whole film), the verdict
//!   expression (overflow groups + continuity violations + caption
//!   failures) and the verdict line's exact wording;
//! - the `--tables` report printed AFTER the verdict, in the
//!   legacy `print_film_tables` line shapes, so
//!   `check_mode.mjs`'s SHOT_ROW / CAP_ROW / CAST_ROW parsing and
//!   the adapter's verdict scan work unchanged.
//!
//! What differs, and why (full defence in the seat note,
//! docs/integration/CR-R1-02 — staging NOTE.md):
//! - Shots come from manifest segments (id, startS/endS, frame
//!   ranges, set, atmosphere) — no anchor names, no hardcoded ids,
//!   no transcribed dressing table.
//! - Per sample, the owning segment's scene root is cloned, its
//!   `time` is set to the segment-local time `lt` (f32 law, as in
//!   contracts §2), validated through `scenefile::load_root`
//!   (fail closed: a scene that does not load at `lt` is a named
//!   `Err`, never a skipped sample), folded by
//!   `eval15::process_scene` (the same fold `load_root` performs),
//!   and its `character` nodes are read from the folded root.
//!   `Loaded` exposes no fields, so the folded root — the loader's
//!   own input — is the only data surface a generic consumer can
//!   read; the cutout construction mirrors `scenefile::walk_nodes`'
//!   character branch field for field.
//! - The projection height is each character node's declared
//!   `height` (default 1.75) instead of the legacy literal 1.75,
//!   which is Onoda's height transcribed; for a 1.75 character the
//!   two agree exactly.
//! - Held plates are resolved from the scene's texture table only
//!   when the texture is a `file` PNG beside the scene (the audit
//!   has no GPU and does not run the texture painters); a plate
//!   that cannot be resolved this way is painted WITHOUT its
//!   pixels and counted, and the count is reported on a NOTE line
//!   before the verdict — never silently.
//! - No `paint-cache:` line is printed: the generic path paints
//!   uncached (scenefile parity), so there is no cache to report.
//!   `check_mode.mjs` only filters such lines out; their absence
//!   changes nothing it parses.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::scene::Value;
use crate::telemetry::{
    describe, look_signature, note_presence, pose_summary, print_caption_audit, project_box,
    scan_canvas_overflow, shot_at, Audit, CaptionInfo, Cutout, PoseState, PresenceRow, ProjCamera,
    ShotInfo,
};
use crate::toonfull::{FullLook, FullState, Legs, Mood, Plate};

// ---------------------------------------------------------------------
// Small value helpers (mirrors of scenefile.rs's private accessors,
// same defaults and error posture; the scene has already been
// validated by load_root before any of these run).
// ---------------------------------------------------------------------

fn truthy(v: Option<&Value>) -> bool {
    matches!(v, Some(Value::Bool(true))) || v.and_then(Value::as_f64).unwrap_or(0.0) != 0.0
}

fn opt_str(v: &Value, key: &str) -> Option<String> {
    v.get(key).and_then(Value::as_str).map(|s| s.to_string())
}

fn f32_or(v: &Value, key: &str, default: f32) -> f32 {
    v.get(key).and_then(Value::as_f64).map(|n| n as f32).unwrap_or(default)
}

fn vec3_or(v: &Value, key: &str, default: [f32; 3]) -> [f32; 3] {
    match v.get(key).and_then(Value::as_arr) {
        Some(a) if a.len() == 3 => [
            a[0].as_f64().unwrap_or(default[0] as f64) as f32,
            a[1].as_f64().unwrap_or(default[1] as f64) as f32,
            a[2].as_f64().unwrap_or(default[2] as f64) as f32,
        ],
        _ => default,
    }
}

fn pair_or(v: &Value, key: &str, default: (f32, f32)) -> (f32, f32) {
    match v.get(key).and_then(Value::as_arr) {
        Some(a) if a.len() == 2 => (
            a[0].as_f64().unwrap_or(default.0 as f64) as f32,
            a[1].as_f64().unwrap_or(default.1 as f64) as f32,
        ),
        _ => default,
    }
}

/// Set (or insert) the root's `time` field — the scene-sequence
/// driver's staging law (contracts §2): the value written is the
/// f32 nearest the segment-local time.
fn set_root_time(root: &mut Value, lt: f64) -> Result<(), String> {
    let v = (lt as f32) as f64;
    match root {
        Value::Obj(pairs) => {
            if let Some(p) = pairs.iter_mut().find(|(k, _)| k == "time") {
                p.1 = Value::Num(v);
            } else {
                pairs.push(("time".to_string(), Value::Num(v)));
            }
            Ok(())
        }
        _ => Err("scene root is not an object".to_string()),
    }
}

/// `&'static str` interner for telemetry `Look` fields (the legacy
/// path leaks per registry look in `film::tel_look`; the generic
/// path meets unbounded authored strings, so each distinct string
/// is leaked exactly once per telemetry run).
#[derive(Default)]
struct Interner {
    map: HashMap<String, &'static str>,
}

impl Interner {
    fn get(&mut self, s: &str) -> &'static str {
        if let Some(v) = self.map.get(s) {
            return v;
        }
        let leaked: &'static str = Box::leak(s.to_string().into_boxed_str());
        self.map.insert(s.to_string(), leaked);
        leaked
    }
    fn opt(&mut self, s: &Option<String>) -> Option<&'static str> {
        s.as_ref().map(|v| self.get(v))
    }
}

// ---------------------------------------------------------------------
// Manifest helpers
// ---------------------------------------------------------------------

/// Index of the segment owning film time `t` (segments tile the
/// film by frame range; time ownership is start_s <= t < end_s,
/// with the last segment also owning the endpoint neighbourhood
/// and any float dust past the end).
fn segment_index_at(manifest: &crate::filmseq::FilmManifest, t: f64) -> usize {
    for (i, seg) in manifest.segments.iter().enumerate() {
        if t >= seg.start_s && t < seg.end_s {
            return i;
        }
    }
    if t < manifest.segments[0].start_s {
        0
    } else {
        manifest.segments.len() - 1
    }
}

/// Fail-closed manifest sanity: a declared caption range that
/// points outside the film's caption list is a corrupt bridge,
/// named here instead of misreported downstream. A segment whose
/// range is empty (first > last, the bridge's no-chunks marker)
/// is not checked.
fn check_caption_ranges(
    manifest: &crate::filmseq::FilmManifest,
    captions: usize,
) -> Result<(), String> {
    for seg in &manifest.segments {
        if let (Some(first), Some(last)) = (seg.caption_first, seg.caption_last) {
            if first <= last && last >= captions {
                return Err(format!(
                    "filmtelemetry: segment {} declares caption chunks {}..={} but the film data carries {} captions",
                    seg.id, first, last, captions
                ));
            }
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------
// Caption widths — the legacy estimator.
//
// film.rs's telemetry measures each caption's rendered width as
// the ink bounding box of the film's own rendered strip PNG
// (alpha > 32), and raw width as rendered / fit. The generic path
// measures the SAME strips; it loads only the strips (from
// <film_dir>/assets/<strip>), not the whole asset set, because a
// decoupled film (contracts §4) may legitimately declare no
// calendar or sign assets for load_assets to demand.
// ---------------------------------------------------------------------

fn load_strip(film_dir: &Path, strip: &str, idx: usize) -> Result<(Vec<u8>, u32, u32), String> {
    if strip.is_empty() {
        return Err(format!("filmtelemetry: captions[{idx}].strip: missing or empty"));
    }
    let path = film_dir.join("assets").join(strip);
    let img = image::open(&path)
        .map_err(|e| format!("filmtelemetry: cannot load {}: {e}", path.display()))?
        .to_rgba8();
    let (w, h) = img.dimensions();
    Ok((img.into_raw(), w, h))
}

/// Ink width (alpha > 32) of one strip — film.rs's loop, verbatim
/// arithmetic: the span from the first to the last inked column.
fn strip_ink_width(px: &[u8], w: u32, h: u32) -> f64 {
    let (mut x0, mut x1) = (i64::MAX, -1i64);
    for y in 0..h as i64 {
        for x in 0..w as i64 {
            let a = px[((y * w as i64 + x) as usize) * 4 + 3];
            if a > 32 {
                x0 = x0.min(x);
                x1 = x1.max(x);
            }
        }
    }
    if x1 >= 0 {
        (x1 - x0 + 1) as f64
    } else {
        0.0
    }
}

/// Build the audit's CaptionInfo list from measured (rendered,
/// raw) widths — the line attribution is film.rs's own rule (the
/// last line whose start is <= caption start + 0.05), and `auto`
/// is true for every film caption, exactly as the legacy film
/// path constructs them (film data carries no authored flag;
/// check_mode.mjs corrects the authored count from script.json).
fn caption_infos_from_widths(
    data: &crate::film::FilmData,
    widths: &[(f64, f64)],
) -> Vec<CaptionInfo> {
    data.captions
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let line = data
                .lines
                .iter()
                .rposition(|&(s, _)| s <= c.s + 0.05)
                .unwrap_or(0) as i64;
            CaptionInfo {
                line,
                s: c.s,
                e: c.e,
                text: c.text.clone(),
                auto: true,
                raw_w: widths[i].1,
                fit: c.fit,
                rendered_w: widths[i].0,
            }
        })
        .collect()
}

fn caption_infos(
    manifest: &crate::filmseq::FilmManifest,
    data: &crate::film::FilmData,
) -> Result<Vec<CaptionInfo>, String> {
    let mut widths = Vec::with_capacity(data.captions.len());
    for (i, c) in data.captions.iter().enumerate() {
        let (px, w, h) = load_strip(&manifest.film_dir, &c.strip, i)?;
        let rendered = strip_ink_width(&px, w, h);
        let raw = if c.fit > 0.0 { rendered / c.fit } else { rendered };
        widths.push((rendered, raw));
    }
    Ok(caption_infos_from_widths(data, &widths))
}

/// Display-time advisory (never verdict-bearing): the legacy
/// audit's per-word time floor is advisory by ruling R5
/// (telemetry.rs, print_caption_audit: "the ≥0.33 s/word time
/// floor is NOT enforced — the width floor wins"). The legacy
/// film gate prints no per-chunk time lines; the generic gate
/// surfaces the same floor as explicit advisory lines so a
/// too-fast chunk is visible in the report without ever entering
/// the verdict counts.
fn caption_time_advisories(caps: &[CaptionInfo]) -> Vec<String> {
    let mut out = Vec::new();
    for c in caps {
        let words = c.text.split_whitespace().count();
        if words == 0 {
            continue;
        }
        let per_word = (c.e - c.s) / words as f64;
        if per_word < 0.33 {
            out.push(format!(
                "CAPTION TIME (advisory): line {} chunk \"{}\" shows {:.2} s for {} words ({:.2} s/word < 0.33) at t={:.2}s — advisory only (ruling R5); not counted in the verdict",
                c.line + 1,
                c.text,
                c.e - c.s,
                words,
                per_word,
                c.s
            ));
        }
    }
    out
}

// ---------------------------------------------------------------------
// Scene-side cutouts: character nodes of the folded scene root.
//
// The construction mirrors scenefile::walk_nodes' "character"
// branch (the loader's own interpretation of the node) field for
// field: same look/state defaults, same painter, same faced model
// (parent transform accumulated through groups, transform ops
// composed by the same law as scenefile::compose_transform).
// ---------------------------------------------------------------------

/// smootherstep (scenefile::smootherstep / motion.ts camK easing).
fn smootherstep(u: f64) -> f64 {
    let u = u.clamp(0.0, 1.0);
    u * u * u * (u * (u * 6.0 - 15.0) + 10.0)
}

struct SeqCamera {
    proj: ProjCamera,
    eye: [f32; 3],
}

/// One camera key read against the static base values (a key's
/// missing eye/target/fovY falls back to the base, as in
/// scenefile's eval_camera_keys): (eye, target, fovY, linear).
fn cam_key(
    k: &Value,
    base_eye: [f32; 3],
    base_target: [f32; 3],
    base_fov: f32,
) -> ([f64; 3], [f64; 3], f32, bool) {
    let e = vec3_or(k, "eye", base_eye);
    let t = vec3_or(k, "target", base_target);
    (
        [e[0] as f64, e[1] as f64, e[2] as f64],
        [t[0] as f64, t[1] as f64, t[2] as f64],
        f32_or(k, "fovY", base_fov),
        k.get("ease").and_then(Value::as_str) == Some("linear"),
    )
}

/// The scene's camera at the root's time: static eye/target/fovY
/// with format-v2 keys evaluated exactly as scenefile's
/// eval_camera_keys (smootherstep between keys, "linear" ease
/// honoured, end keys held outside the range).
fn scene_camera(root: &Value, time: f64) -> Result<SeqCamera, String> {
    let cam = root
        .get("camera")
        .ok_or_else(|| "filmtelemetry: scene has no camera block".to_string())?;
    let mut eye = vec3_or(cam, "eye", [0.0, 0.0, 0.0]);
    let mut target = vec3_or(cam, "target", [0.0, 0.0, 0.0]);
    let mut fov = f32_or(cam, "fovY", 44.0);
    let up = vec3_or(cam, "up", [0.0, 1.0, 0.0]);
    let near = f32_or(cam, "near", 0.05) as f64;
    let far = f32_or(cam, "far", 800.0) as f64;
    if let Some(keys) = cam.get("keys").and_then(Value::as_arr) {
        if !keys.is_empty() {
            // Base values are captured by value (the key reader
            // is a free fn, not a closure — eye/target/fov are
            // reassigned below while keys are being read).
            let (base_eye, base_target, base_fov) = (eye, target, fov);
            let first_t = keys[0].get("t").and_then(Value::as_f64).unwrap_or(0.0);
            if time <= first_t || keys.len() == 1 {
                let (e, t, f, _) = cam_key(&keys[0], base_eye, base_target, base_fov);
                eye = [e[0] as f32, e[1] as f32, e[2] as f32];
                target = [t[0] as f32, t[1] as f32, t[2] as f32];
                fov = f;
            } else {
                let mut done = false;
                for w in keys.windows(2) {
                    let at = w[0].get("t").and_then(Value::as_f64).unwrap_or(0.0);
                    let bt = w[1].get("t").and_then(Value::as_f64).unwrap_or(0.0);
                    if time <= bt {
                        let (ae, atg, af, _) = cam_key(&w[0], base_eye, base_target, base_fov);
                        let (be, btg, bf, linear) = cam_key(&w[1], base_eye, base_target, base_fov);
                        let span = bt - at;
                        let raw = if span > 0.0 { (time - at) / span } else { 1.0 };
                        let u = (if linear { raw.clamp(0.0, 1.0) } else { smootherstep(raw) }) as f32;
                        eye = [
                            ae[0] as f32 + (be[0] as f32 - ae[0] as f32) * u,
                            ae[1] as f32 + (be[1] as f32 - ae[1] as f32) * u,
                            ae[2] as f32 + (be[2] as f32 - ae[2] as f32) * u,
                        ];
                        target = [
                            atg[0] as f32 + (btg[0] as f32 - atg[0] as f32) * u,
                            atg[1] as f32 + (btg[1] as f32 - atg[1] as f32) * u,
                            atg[2] as f32 + (btg[2] as f32 - atg[2] as f32) * u,
                        ];
                        fov = af + (bf - af) * u;
                        done = true;
                        break;
                    }
                }
                if !done {
                    let (e, t, f, _) = cam_key(&keys[keys.len() - 1], base_eye, base_target, base_fov);
                    eye = [e[0] as f32, e[1] as f32, e[2] as f32];
                    target = [t[0] as f32, t[1] as f32, t[2] as f32];
                    fov = f;
                }
            }
        }
    }
    Ok(SeqCamera {
        proj: ProjCamera {
            eye: [eye[0] as f64, eye[1] as f64, eye[2] as f64],
            target: [target[0] as f64, target[1] as f64, target[2] as f64],
            up: [up[0] as f64, up[1] as f64, up[2] as f64],
            fov_deg: fov as f64,
            aspect: 1080.0 / 1920.0,
            near,
            far,
        },
        eye,
    })
}

/// scenefile::compose_transform, mirrored op for op (m = m * op,
/// left to right) over crate::math — group transforms accumulate
/// down the tree exactly as the loader accumulates them.
fn compose_transform(spec: Option<&Value>) -> Result<crate::math::Mat4, String> {
    use crate::math;
    let mut m = math::identity();
    let Some(spec) = spec else { return Ok(m) };
    let ops = spec
        .as_arr()
        .ok_or_else(|| "filmtelemetry: 'transform' must be an array of ops".to_string())?;
    for op in ops {
        let Value::Obj(pairs) = op else {
            return Err("filmtelemetry: transform op must be a single-key object".to_string());
        };
        if pairs.len() != 1 {
            return Err("filmtelemetry: transform op must have exactly one key".to_string());
        }
        let (key, val) = &pairs[0];
        let v3 = |v: &Value| -> [f32; 3] {
            match v.as_arr() {
                Some(a) if a.len() == 3 => [
                    a[0].as_f64().unwrap_or(0.0) as f32,
                    a[1].as_f64().unwrap_or(0.0) as f32,
                    a[2].as_f64().unwrap_or(0.0) as f32,
                ],
                _ => [0.0, 0.0, 0.0],
            }
        };
        let opm = match key.as_str() {
            "translate" => {
                let t = v3(val);
                math::translate(t[0], t[1], t[2])
            }
            "rotate_x" => math::rotate_x(val.as_f64().unwrap_or(0.0) as f32),
            "rotate_y" => math::rotate_y(val.as_f64().unwrap_or(0.0) as f32),
            "rotate_z" => math::rotate_z(val.as_f64().unwrap_or(0.0) as f32),
            "euler_xyz" => {
                let e = v3(val);
                math::euler_xyz(e[0], e[1], e[2])
            }
            "scale" => {
                let s = v3(val);
                math::scale(s[0], s[1], s[2])
            }
            other => return Err(format!("filmtelemetry: unknown transform op '{other}'")),
        };
        m = math::mul(&m, &opm);
    }
    Ok(m)
}

/// film.rs's mood_name / legs_name, copied (both are private to
/// film.rs): the pose summary's vocabulary is the painter's.
fn mood_name(m: Mood) -> &'static str {
    match m {
        Mood::Neutral => "neutral",
        Mood::Smug => "smug",
        Mood::Happy => "happy",
        Mood::Laugh => "laugh",
        Mood::Shock => "shock",
        Mood::Worried => "worried",
        Mood::Angry => "angry",
        Mood::Sad => "sad",
        Mood::Sly => "sly",
        Mood::Proud => "proud",
        Mood::Tired => "tired",
        Mood::Dizzy => "dizzy",
        Mood::Determined => "determined",
        Mood::Embarrassed => "embarrassed",
        Mood::Awe => "awe",
        Mood::Suspicious => "suspicious",
        Mood::Relieved => "relieved",
        Mood::Puzzled => "puzzled",
        Mood::Deadpan => "deadpan",
    }
}

fn legs_name(l: Legs) -> &'static str {
    match l {
        Legs::Stand => "stand",
        Legs::Walk => "walk",
        Legs::Run => "run",
        Legs::Sit => "sit",
        Legs::Jump => "jump",
    }
}

/// Resolve a held plate the audit's way: only a `file` texture
/// declared in the scene's texture table, read from beside the
/// scene file. Generator-declared plates and undeclared ids
/// return None and are counted by the caller (the loader itself
/// fails closed on undeclared ids before this walk runs, so a
/// None here in production means a generator plate).
fn resolve_plate(
    root: &Value,
    scene_dir: &Path,
    hold: &Value,
) -> Option<Plate> {
    let id = hold.get("plate").and_then(Value::as_str)?;
    let spec = root
        .get("textures")
        .and_then(Value::as_arr)?
        .iter()
        .find(|t| t.get("id").and_then(Value::as_str) == Some(id))?;
    let file = spec.get("file").and_then(Value::as_str)?;
    let img = image::open(scene_dir.join(file)).ok()?.to_rgba8();
    let (w, h) = img.dimensions();
    Some(Plate {
        px: img.into_raw(),
        w,
        h,
        x0: f32_or(hold, "x0", 0.0),
        y0: f32_or(hold, "y0", 0.0),
    })
}

struct WalkCtx<'a> {
    root: &'a Value,
    scene_dir: &'a Path,
    seg_id: &'a str,
    cam: &'a SeqCamera,
    intern: &'a mut Interner,
    unresolved_plates: &'a mut usize,
    char_seq: usize,
}

fn walk_characters(
    nodes: &[Value],
    parent: &crate::math::Mat4,
    ctx: &mut WalkCtx,
    out: &mut Vec<Cutout>,
) -> Result<(), String> {
    use crate::math;
    for node in nodes {
        match node.get("type").and_then(Value::as_str) {
            Some("group") => {
                let local = compose_transform(node.get("transform"))?;
                let m = math::mul(parent, &local);
                let children = node
                    .get("children")
                    .and_then(Value::as_arr)
                    .ok_or_else(|| "filmtelemetry: group node without children array".to_string())?;
                walk_characters(children, &m, ctx, out)?;
            }
            Some("character") => {
                ctx.char_seq += 1;
                out.push(character_cutout(node, parent, ctx)?);
            }
            _ => {}
        }
    }
    Ok(())
}

/// One character node -> one Cutout, mirroring scenefile's
/// character branch: FullLook/FullState with the loader's own
/// defaults, painted by paint_toon_full, overflow scanned at
/// res 2.0, box projected from the faced model at the node's
/// declared height.
fn character_cutout(
    node: &Value,
    parent: &crate::math::Mat4,
    ctx: &mut WalkCtx,
) -> Result<Cutout, String> {
    use crate::math;
    let pos = vec3_or(node, "position", [0.0, 0.0, 0.0]);
    let height = f32_or(node, "height", 1.75);
    let h_t = height / 0.96;
    let perspective = truthy(node.get("perspective"));
    let look_spec = node.get("look").cloned().unwrap_or(Value::Null);
    let extra: Vec<String> = look_spec
        .get("extra")
        .and_then(Value::as_arr)
        .map(|a| a.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
        .unwrap_or_default();
    let has_extra = |name: &str| extra.iter().any(|e| e == name);
    // The painter's look — scenefile's construction, verbatim
    // defaults (the loader drops authored extras it does not
    // paint; the signature below keeps the authored list whole).
    let look = FullLook {
        body: opt_str(&look_spec, "body").unwrap_or_else(|| "#E8734A".to_string()),
        head: opt_str(&look_spec, "head").unwrap_or_else(|| "#F7F7FA".to_string()),
        shade: opt_str(&look_spec, "shade"),
        pattern: opt_str(&look_spec, "pattern").unwrap_or_else(|| "plain".to_string()),
        pat_col: opt_str(&look_spec, "patCol"),
        hat: opt_str(&look_spec, "hat").unwrap_or_else(|| "none".to_string()),
        hat_col: opt_str(&look_spec, "hatCol"),
        band: opt_str(&look_spec, "band"),
        neck: opt_str(&look_spec, "neck").unwrap_or_else(|| "none".to_string()),
        neck_col: opt_str(&look_spec, "neckCol"),
        glasses: has_extra("glasses"),
        mustache: has_extra("mustache"),
        mustache_col: opt_str(&look_spec, "mustacheCol"),
        kid: truthy(look_spec.get("kid")),
        blush: has_extra("blush"),
        freckles: has_extra("freckles"),
        pencil: has_extra("pencil"),
        monocle: has_extra("monocle"),
        hair_col: opt_str(&look_spec, "hairCol"),
    };
    let st_spec = node.get("state").cloned().unwrap_or(Value::Null);
    let mut plate_of = |key: &str| -> Option<Plate> {
        let hold = st_spec.get(key)?;
        match resolve_plate(ctx.root, ctx.scene_dir, hold) {
            Some(p) => Some(p),
            None => {
                *ctx.unresolved_plates += 1;
                None
            }
        }
    };
    let state = FullState {
        mood: Mood::from_str(opt_str(&st_spec, "mood").as_deref().unwrap_or("neutral")),
        look: {
            let (a, b) = pair_or(&st_spec, "look", (0.0, 0.0));
            [a, b]
        },
        blink: f32_or(&st_spec, "blink", 0.0),
        talk: f32_or(&st_spec, "talk", 0.0),
        sweat: truthy(st_spec.get("sweat")),
        tears: truthy(st_spec.get("tears")),
        arm_l: pair_or(&st_spec, "armL", (8.0, 6.0)),
        arm_r: pair_or(&st_spec, "armR", (8.0, 6.0)),
        legs: match opt_str(&st_spec, "legs").as_deref() {
            Some("walk") => Legs::Walk,
            Some("run") => Legs::Run,
            Some("sit") => Legs::Sit,
            Some("jump") => Legs::Jump,
            _ => Legs::Stand,
        },
        phase: f32_or(&st_spec, "phase", 0.0),
        lean: f32_or(&st_spec, "lean", 0.0),
        bob: f32_or(&st_spec, "bob", 0.0),
        squash: f32_or(&st_spec, "squash", 0.0),
        head_tilt: f32_or(&st_spec, "headTilt", 0.0),
        head_turn: f32_or(&st_spec, "headTurn", 0.0),
        cam_pitch: f32_or(&st_spec, "camPitch", 0.0),
        sit_drop: match st_spec.get("sitOn") {
            Some(v) => Some((v.as_f64().unwrap_or(0.0) as f32) * 1024.0 / h_t),
            None => None,
        },
        hold_l: plate_of("holdL"),
        hold_r: plate_of("holdR"),
    };
    let (pxbuf, pw, ph, _, _) = crate::toonfull::paint_toon_full(&look, &state);
    // face(): yaw toward the camera, perspective pitch — the
    // loader's f32 arithmetic (scenefile walk_nodes).
    let yaw = (ctx.cam.eye[0] - pos[0]).atan2(ctx.cam.eye[2] - pos[2]);
    let pitch = if perspective {
        let dy = ctx.cam.eye[1] - (pos[1] + h_t * 0.6);
        let dh = ((ctx.cam.eye[0] - pos[0]).powi(2) + (ctx.cam.eye[2] - pos[2]).powi(2)).sqrt();
        if dh < 1e-4 {
            0.0
        } else {
            (dy.atan2(dh) * 0.45).clamp(-0.38, 0.38)
        }
    } else {
        0.0
    };
    let model = math::mul(
        parent,
        &math::mul(
            &math::translate(pos[0], pos[1], pos[2]),
            &math::mul(&math::rotate_y(yaw), &math::rotate_x(pitch)),
        ),
    );
    // The telemetry look: identity fields from the AUTHORED look
    // (cast.ts's lookSignature reads the authored look, extras
    // whole), with drawToon's defaults for absent fields, as in
    // telemetry::describe / film::tel_look.
    let tel = crate::telemetry::Look {
        body: Some(ctx.intern.get(&look.body)),
        pattern: Some(ctx.intern.get(&look.pattern)),
        pat_col: ctx.intern.opt(&look.pat_col),
        hat: Some(ctx.intern.get(&look.hat)),
        hat_col: ctx.intern.opt(&look.hat_col),
        band: ctx.intern.opt(&look.band),
        neck: Some(ctx.intern.get(&look.neck)),
        neck_col: ctx.intern.opt(&look.neck_col),
        head: Some(ctx.intern.get(&look.head)),
        kid: look.kid,
        extra: extra.iter().map(|e| ctx.intern.get(e)).collect(),
    };
    // Identity: the node's authored `name` (the cast registry
    // name in scene data). An unnamed character cannot be bound
    // across segments, so it is tracked per segment occurrence —
    // present in presence/overflow, never falsely continuous.
    let id = match opt_str(node, "name") {
        Some(n) if !n.is_empty() => n,
        _ => format!("{}#char{}", ctx.seg_id, ctx.char_seq),
    };
    Ok(Cutout {
        id,
        desc: describe(&tel),
        pose: pose_summary(&PoseState {
            mood: mood_name(state.mood),
            arm_l: (state.arm_l.0 as f64, state.arm_l.1 as f64),
            arm_r: (state.arm_r.0 as f64, state.arm_r.1 as f64),
            legs: legs_name(state.legs),
            head_tilt: state.head_tilt as f64,
            head_turn: state.head_turn as f64,
        }),
        overflow: Some(scan_canvas_overflow(&pxbuf, pw as usize, ph as usize, 2.0)),
        box_: Some(project_box(
            &crate::telemetry::model_from_f32(&model),
            &ctx.cam.proj,
            height as f64,
        )),
        sig: look_signature(&tel),
    })
}

/// All cutouts of one folded scene root at its own `time`.
fn cutouts_from_root(
    root: &Value,
    scene_dir: &Path,
    seg_id: &str,
    intern: &mut Interner,
    unresolved_plates: &mut usize,
) -> Result<Vec<Cutout>, String> {
    let time = root.get("time").and_then(Value::as_f64).unwrap_or(0.0);
    let cam = scene_camera(root, time)?;
    let nodes = root
        .get("nodes")
        .and_then(Value::as_arr)
        .ok_or_else(|| "filmtelemetry: scene root has no nodes array".to_string())?;
    let mut ctx = WalkCtx {
        root,
        scene_dir,
        seg_id,
        cam: &cam,
        intern,
        unresolved_plates,
        char_seq: 0,
    };
    let mut out = Vec::new();
    walk_characters(nodes, &crate::math::identity(), &mut ctx, &mut out)?;
    Ok(out)
}

// ---------------------------------------------------------------------
// Tables (--tables): the legacy print_film_tables line shapes,
// sourced from the manifest + this run's own audit products.
// ---------------------------------------------------------------------

fn fmt_fps(fps: f64) -> String {
    if fps.fract() == 0.0 && fps.abs() < 1e15 {
        format!("{}", fps as i64)
    } else {
        format!("{fps}")
    }
}

fn shot_table_lines(
    manifest: &crate::filmseq::FilmManifest,
    presence: &[PresenceRow],
) -> Vec<String> {
    let mut out = Vec::new();
    out.push(format!(
        "SHOT TABLE — {} shots, {} frames @ {} fps, {:.2} s",
        manifest.segments.len(),
        manifest.total_frames,
        fmt_fps(manifest.fps),
        manifest.duration_s
    ));
    out.push(format!(
        "{:<6}{:<13}{:<16}{:>6}  {:<5}{:<11}characters",
        "shot", "frames", "time (s)", "dur", "set", "atmosphere"
    ));
    for seg in &manifest.segments {
        let mut chars: Vec<String> = Vec::new();
        for r in presence.iter().filter(|r| r.shot == seg.id) {
            if r.max_simul > 1 {
                chars.push(format!("{} x{}", r.id, r.max_simul));
            } else {
                chars.push(r.id.clone());
            }
        }
        let chars = if chars.is_empty() {
            "(none)".to_string()
        } else {
            chars.join(", ")
        };
        out.push(format!(
            "{:<6}{:<13}{:<16}{:>6.2}  {:<5}{:<11}{}",
            seg.id,
            format!("{}-{}", seg.start_frame, seg.end_frame.saturating_sub(1)),
            format!("{:.2}-{:.2}", seg.start_s, seg.end_s),
            seg.end_s - seg.start_s,
            seg.set,
            seg.atmosphere,
            chars
        ));
    }
    out
}

fn caption_table_lines(
    manifest: &crate::filmseq::FilmManifest,
    shots: &[ShotInfo],
    caps: &[CaptionInfo],
) -> Vec<String> {
    let mut out = Vec::new();
    out.push(format!(
        "CAPTION TABLE — {} chunks (audit bars: rendered > 850 px = WIDTH fail, fit < 0.68 = FIT fail; strict)",
        caps.len()
    ));
    out.push(format!(
        "{:<5}{:<6}{:<6}{:<17}{:>5}{:>6}{:>7}  {:<10}text",
        "idx", "line", "shot", "window (s)", "fit", "rend", "raw", "flag"
    ));
    let mut notes: Vec<String> = Vec::new();
    for (i, c) in caps.iter().enumerate() {
        let owner = shot_at(shots, c.s);
        let owner_idx = shots.iter().position(|s| s.id == owner.id).unwrap_or(0);
        // The flag predicates are print_caption_audit's own
        // literals, as in the legacy tables (W85 §1.2 freeze).
        let wide = c.rendered_w > 850.0;
        let low_fit = c.fit < 0.68;
        let flag = match (wide, low_fit) {
            (true, true) => "WIDTH+FIT",
            (true, false) => "WIDTH",
            (false, true) => "FIT",
            (false, false) => "",
        };
        out.push(format!(
            "{:<5}{:<6}{:<6}{:<17}{:>5.2}{:>6.0}{:>7.0}  {:<10}\"{}\"",
            i,
            c.line + 1,
            owner.id,
            format!("{:.2}-{:.2}", c.s, c.e),
            c.fit,
            c.rendered_w,
            c.raw_w,
            flag,
            c.text
        ));
        let end = if owner_idx + 1 < manifest.segments.len() {
            manifest.segments[owner_idx + 1].start_s
        } else {
            manifest.duration_s
        };
        if c.e > end {
            notes.push(format!(
                "note: chunk {} ({}) straddles the cut — its window runs {:.2} s into the next shot; the driver draws it across the cut (window rule)",
                i,
                owner.id,
                c.e - end
            ));
        }
    }
    out.extend(notes);
    out
}

fn cast_list_lines(
    manifest: &crate::filmseq::FilmManifest,
    shots: &[ShotInfo],
    presence: &[PresenceRow],
) -> Vec<String> {
    // Identity order: first appearance, as in the legacy tables.
    let mut order: Vec<String> = Vec::new();
    for r in presence {
        if !order.contains(&r.id) {
            order.push(r.id.clone());
        }
    }
    order.sort_by(|a, b| {
        let fa = presence.iter().filter(|r| &r.id == a).map(|r| r.first).fold(f64::MAX, f64::min);
        let fb = presence.iter().filter(|r| &r.id == b).map(|r| r.first).fold(f64::MAX, f64::min);
        fa.partial_cmp(&fb).unwrap_or(std::cmp::Ordering::Equal)
    });
    let frames_per_sample = manifest.fps * 0.25;
    let mut out = Vec::new();
    out.push(format!(
        "CAST LIST — {} identities (frame counts are sampled presence: 0.25 s audit samples x {} frames per sample)",
        order.len(),
        fmt_fps(frames_per_sample)
    ));
    out.push(format!(
        "{:<9}{:>6}{:>8}{:>7}  shot list / authored blink beats",
        "id", "shots", "frames", "beats"
    ));
    for id in &order {
        let rows: Vec<&PresenceRow> = presence.iter().filter(|r| &r.id == id).collect();
        let total_samples: u32 = rows.iter().map(|r| r.samples).sum();
        let frames = (total_samples as f64 * frames_per_sample).round() as u32;
        let shot_list: Vec<&str> = shots
            .iter()
            .filter(|s| rows.iter().any(|r| r.shot == s.id))
            .map(|s| s.id.as_str())
            .collect();
        // No authored blink-beat bindings exist in scene data
        // (the legacy table's bindings are transcribed from the
        // Onoda staging code); the count is an honest 0 and the
        // beats line carries the dash the legacy shape prints
        // for an empty entry list.
        out.push(format!(
            "{:<9}{:>6}{:>8}{:>7}  {}",
            id,
            shot_list.len(),
            frames,
            0,
            shot_list.join(", ")
        ));
        out.push(format!(
            "{:<9}{:>6}{:>8}{:>7}  beats: {}",
            "", "", "", "", "-"
        ));
    }
    out
}

fn table_lines(
    manifest: &crate::filmseq::FilmManifest,
    shots: &[ShotInfo],
    caps: &[CaptionInfo],
    presence: &[PresenceRow],
) -> Vec<String> {
    let mut out = vec![String::new()];
    out.extend(shot_table_lines(manifest, presence));
    out.push(String::new());
    out.extend(caption_table_lines(manifest, shots, caps));
    out.push(String::new());
    out.extend(cast_list_lines(manifest, shots, presence));
    out
}

// ---------------------------------------------------------------------
// The verdict line — the consumer contract's exact wording
// (film.rs run_film_telemetry; check_mode.mjs parses the FAILED
// counts back out of it).
// ---------------------------------------------------------------------

fn verdict_line(overflow: usize, violations: usize, cap_fails: usize) -> String {
    if overflow + violations + cap_fails == 0 {
        "FILM TELEMETRY: CHECK OK".to_string()
    } else {
        format!(
            "FILM TELEMETRY: CHECK FAILED ({} overflow groups, {} continuity violations, {} caption failures)",
            overflow, violations, cap_fails
        )
    }
}

// ---------------------------------------------------------------------
// The generic telemetry entry point (contracts §3). Invoked from
// the §1 fork in run_film_telemetry when the manifest selects
// the scene-sequence driver. Ok(true) = CHECK OK, Ok(false) =
// CHECK FAILED — the printed verdict line stays authoritative,
// exactly as in the legacy entry point.
// ---------------------------------------------------------------------

pub(crate) fn run_telemetry_seq(
    manifest: &crate::filmseq::FilmManifest,
    data: &crate::film::FilmData,
    tables: bool,
) -> Result<bool, String> {
    if manifest.segments.is_empty() {
        return Err("filmtelemetry: manifest carries no segments".to_string());
    }
    check_caption_ranges(manifest, data.captions.len())?;

    // Load every segment's scene root once (fail closed: a scene
    // that resolves nowhere or does not parse is a named load
    // error, per contracts §1.2's posture for the driver).
    let mut roots: Vec<Value> = Vec::with_capacity(manifest.segments.len());
    let mut scene_dirs: Vec<PathBuf> = Vec::with_capacity(manifest.segments.len());
    for seg in &manifest.segments {
        let path = seg.scene_path.clone();
        let text = std::fs::read_to_string(&path).map_err(|e| {
            format!(
                "filmtelemetry: segment {} scene {}: cannot read: {e}",
                seg.id,
                path.display()
            )
        })?;
        let root = crate::scene::parse_json(&text).map_err(|e| {
            format!("filmtelemetry: segment {} scene {}: {e}", seg.id, path.display())
        })?;
        scene_dirs.push(
            path.parent().map(|p| p.to_path_buf()).unwrap_or_else(|| PathBuf::from(".")),
        );
        roots.push(root);
    }

    let caps = caption_infos(manifest, data)?;

    let shots: Vec<ShotInfo> = manifest
        .segments
        .iter()
        .map(|seg| ShotInfo { id: seg.id.clone(), at: seg.start_s })
        .collect();
    let shots_keep = shots.clone();
    let mut audit = Audit::new(shots);
    let mut presence: Vec<PresenceRow> = Vec::new();
    let mut intern = Interner::default();
    let mut unresolved_plates = 0usize;

    // 0.25 s sampling, the legacy loop's shape (t from 0 while
    // t < duration, stderr progress every 20 samples).
    let mut t = 0.0f64;
    let mut sample_no = 0u32;
    while t < manifest.duration_s {
        sample_no += 1;
        if sample_no % 20 == 0 {
            eprintln!("film-telemetry: sample {sample_no} (t={t:.2})");
        }
        let si = segment_index_at(manifest, t);
        let seg = &manifest.segments[si];
        let lt = t - seg.start_s;
        let mut root = roots[si].clone();
        set_root_time(&mut root, lt)?;
        let label = format!("segment {} scene at lt={lt:.2}", seg.id);
        // Validation gate: the loader must accept this scene at
        // this local time (era pairing fails closed). Loaded is
        // opaque, so the cutouts are read from the folded root
        // below — the same root, the same fold.
        crate::scenefile::load_root(root.clone(), &label).map_err(|e| {
            format!("filmtelemetry: segment {} at lt={lt:.2}: {e}", seg.id)
        })?;
        let folded = crate::eval15::process_scene(root, &[]).map_err(|e| {
            format!("filmtelemetry: segment {} at lt={lt:.2}: {e}", seg.id)
        })?;
        let cuts = cutouts_from_root(
            &folded,
            &scene_dirs[si],
            &seg.id,
            &mut intern,
            &mut unresolved_plates,
        )?;
        audit.sample(t, &cuts);
        let ids: Vec<&str> = cuts.iter().map(|c| c.id.as_str()).collect();
        note_presence(&mut presence, &seg.id, t, &ids);
        t += 0.25;
    }

    let mut out: Vec<String> = Vec::new();
    out.push(format!(
        "FILM TELEMETRY — {} (native, 0.25 s sampling)",
        manifest.title
    ));
    audit.print_audit(false, &mut out);
    audit.print_continuity(&mut out);
    audit.print_composition(&mut out);
    let cap_fails = print_caption_audit(&caps, true, &mut out);
    out.extend(caption_time_advisories(&caps));
    if unresolved_plates > 0 {
        out.push(format!(
            "GENERIC TELEMETRY NOTE: {} held-plate sample(s) painted without plate pixels (plate texture is generator-declared; the raw-scene cutout path resolves file plates only) — canvas-overflow counts for those samples exclude plate ink",
            unresolved_plates
        ));
    }
    let bad = audit.overflow_len() + audit.violations_len() + cap_fails;
    out.push(verdict_line(
        audit.overflow_len(),
        audit.violations_len(),
        cap_fails,
    ));
    for line in &out {
        println!("{line}");
    }
    if tables {
        // Tables print AFTER the verdict, as in the legacy path:
        // the default output stays a strict prefix of the
        // --tables output and the verdict scan is unaffected.
        for line in table_lines(manifest, &shots_keep, &caps, &presence) {
            println!("{line}");
        }
    }
    Ok(bad == 0)
}

// ---------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a FilmManifest from a project-layout film.json (the
    /// bridge's manifest shape: captionChunks{first,last,count},
    /// set, atmosphere) — the test-side twin of filmseq's parser,
    /// written against the R1-01 API fields this module consumes.
    fn manifest_from_json(
        root: &Value,
        manifest_dir: &Path,
        film_dir: &Path,
    ) -> crate::filmseq::FilmManifest {
        let segs = root
            .get("segments")
            .and_then(Value::as_arr)
            .expect("segments")
            .iter()
            .map(|s| crate::filmseq::Segment {
                id: s.get("id").and_then(Value::as_str).unwrap_or("").to_string(),
                scene: s.get("scene").and_then(Value::as_str).unwrap_or("").to_string(),
                scene_path: manifest_dir.join(
                    s.get("scene").and_then(Value::as_str).unwrap_or(""),
                ),
                lines: s
                    .get("lines")
                    .and_then(Value::as_arr)
                    .map(|a| a.iter().filter_map(|v| v.as_f64().map(|n| n as usize)).collect())
                    .unwrap_or_default(),
                start_s: s.get("startS").and_then(Value::as_f64).unwrap_or(0.0),
                end_s: s.get("endS").and_then(Value::as_f64).unwrap_or(0.0),
                start_frame: s.get("startFrame").and_then(Value::as_f64).unwrap_or(0.0) as u32,
                end_frame: s.get("endFrame").and_then(Value::as_f64).unwrap_or(0.0) as u32,
                caption_first: s
                    .get("captionChunks")
                    .and_then(|c| c.get("first"))
                    .and_then(Value::as_f64)
                    .map(|n| n as usize),
                caption_last: s
                    .get("captionChunks")
                    .and_then(|c| c.get("last"))
                    .and_then(Value::as_f64)
                    .map(|n| n as usize),
                set: s.get("set").and_then(Value::as_str).unwrap_or("").to_string(),
                atmosphere: s
                    .get("atmosphere")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
            })
            .collect();
        crate::filmseq::FilmManifest {
            title: root.get("title").and_then(Value::as_str).unwrap_or("").to_string(),
            fps: root.get("fps").and_then(Value::as_f64).unwrap_or(30.0),
            duration_s: root.get("durationS").and_then(Value::as_f64).unwrap_or(0.0),
            total_frames: root.get("totalFrames").and_then(Value::as_f64).unwrap_or(0.0) as u32,
            driver: root.get("driver").and_then(Value::as_str).map(|s| s.to_string()),
            segments: segs,
            manifest_dir: manifest_dir.to_path_buf(),
            film_dir: film_dir.to_path_buf(),
            manifest_path: manifest_dir.join("film.json"),
        }
    }

    /// Shot-table construction from the real Lighthouse Cat
    /// manifest. The project lives outside the repo, so the test
    /// skips (does not fail) when it is absent — CI has no copy;
    /// the seat verified it locally against the file on disk.
    #[test]
    fn shot_table_from_real_lc_manifest() {
        let native = Path::new(
            "/home/hatch/faceless-shorts/the-lighthouse-cats-last-shift/native",
        );
        let Ok(text) = std::fs::read_to_string(native.join("film.json")) else {
            return; // project not present on this machine — skip
        };
        let root = crate::scene::parse_json(&text).expect("LC manifest parses");
        let manifest = manifest_from_json(&root, native, &native.join("film"));
        assert_eq!(manifest.title, "The Lighthouse Cat's Last Shift");
        assert_eq!(manifest.segments.len(), 11);
        assert_eq!(manifest.total_frames, 1172);

        let lines = shot_table_lines(&manifest, &[]);
        // Header + column header + 11 rows.
        assert_eq!(lines.len(), 13);
        assert!(lines[0].starts_with("SHOT TABLE — 11 shots, 1172 frames @ 30 fps, 39.05 s"));
        // First row carries s01's own data (frames 0..138
        // exclusive -> printed 0-137; 0.00-4.59; dur 4.59; set
        // BCH; atmosphere dusk; no presence yet -> "(none)").
        let row = &lines[2];
        assert!(row.starts_with("s01"), "row: {row}");
        assert!(row.contains("0-137"), "row: {row}");
        assert!(row.contains("0.00-4.59"), "row: {row}");
        assert!(row.contains("4.59"), "row: {row}");
        assert!(row.contains("BCH"), "row: {row}");
        assert!(row.contains("dusk"), "row: {row}");
        assert!(row.contains("(none)"), "row: {row}");
        // Last row: s11, 1024-1171, 34.12-39.05, dawn.
        let last = &lines[12];
        assert!(last.starts_with("s11"), "row: {last}");
        assert!(last.contains("1024-1171"), "row: {last}");
        assert!(last.contains("34.12-39.05"), "row: {last}");
        assert!(last.contains("dawn"), "row: {last}");
        // Caption-range sanity passes against the film's 43
        // captions and fails closed on an out-of-range range.
        check_caption_ranges(&manifest, 43).expect("LC ranges are in bounds");
        assert!(check_caption_ranges(&manifest, 10).is_err());
    }

    fn synthetic_data() -> crate::film::FilmData {
        crate::film::FilmData {
            duration: 12.0,
            anchors: HashMap::new(),
            captions: vec![
                crate::film::Caption {
                    s: 0.5,
                    e: 3.5,
                    text: "a wide chunk of text".to_string(),
                    fit: 1.0,
                    strip: "cap_000.png".to_string(),
                },
                crate::film::Caption {
                    s: 4.0,
                    e: 6.0,
                    text: "shrunk".to_string(),
                    fit: 0.5,
                    strip: "cap_001.png".to_string(),
                },
                crate::film::Caption {
                    s: 6.5,
                    e: 11.5,
                    text: "one two three four five six seven eight nine ten".to_string(),
                    fit: 0.9,
                    strip: "cap_002.png".to_string(),
                },
            ],
            lines: vec![(0.0, 6.0), (6.0, 12.0)],
            calendar_years: Vec::new(),
            calendar_dark: String::new(),
            sign: String::new(),
            s12_clips: None,
        }
    }

    /// Caption audit floors on synthetic captions: width > 850
    /// and fit < 0.68 are strict failures counted for the
    /// verdict; the display-time floor is advisory only.
    #[test]
    fn caption_audit_floors_on_synthetic_captions() {
        let data = synthetic_data();
        // (rendered, raw): chunk 0 renders 900 px wide; chunk 1
        // has fit 0.50; chunk 2 is clean on width/fit.
        let widths = vec![(900.0, 900.0), (300.0, 600.0), (400.0, 444.0)];
        let caps = caption_infos_from_widths(&data, &widths);
        assert_eq!(caps.len(), 3);
        assert_eq!(caps[0].line, 0);
        assert_eq!(caps[2].line, 1);
        let mut out: Vec<String> = Vec::new();
        let fails = print_caption_audit(&caps, true, &mut out);
        assert_eq!(fails, 2, "one WIDTH + one FIT failure: {out:?}");
        assert_eq!(
            out[0],
            "CAPTION AUDIT: 3 chunks, 0/2 lines authored, min fit 0.50, max rendered width 900 px"
        );
        assert!(out.iter().any(|l| l.starts_with("CAPTION WIDTH FAIL:")), "{out:?}");
        assert!(out.iter().any(|l| l.starts_with("CAPTION FIT FAIL:")), "{out:?}");
        // Advisory: shrink chunk 2's window so 10 words show in
        // 2.0 s (0.20 s/word) — advisory fires, verdict unaffected.
        let mut fast_data = synthetic_data();
        fast_data.captions[2].e = fast_data.captions[2].s + 2.0;
        let fast = caption_infos_from_widths(&fast_data, &widths);
        let adv = caption_time_advisories(&fast);
        assert_eq!(adv.len(), 1);
        assert!(adv[0].starts_with("CAPTION TIME (advisory):"));
        assert!(adv[0].contains("not counted in the verdict"));
    }

    /// The verdict line's exact consumer wording, both forms.
    #[test]
    fn verdict_line_formatting() {
        assert_eq!(verdict_line(0, 0, 0), "FILM TELEMETRY: CHECK OK");
        assert_eq!(
            verdict_line(2, 1, 3),
            "FILM TELEMETRY: CHECK FAILED (2 overflow groups, 1 continuity violations, 3 caption failures)"
        );
        assert_eq!(
            verdict_line(0, 0, 1),
            "FILM TELEMETRY: CHECK FAILED (0 overflow groups, 0 continuity violations, 1 caption failures)"
        );
    }

    /// Cutout extraction from a synthetic scene root: one named
    /// character yields one cutout with the authored identity,
    /// a look signature carrying the authored body colour, and
    /// a painted-canvas overflow measurement.
    #[test]
    fn cutout_from_synthetic_scene_root() {
        let text = r##"{
            "format": "talerender-scene", "version": 2, "time": 0.0,
            "camera": {"eye": [0, 2.6, 10.5], "target": [0, 1.2, 0], "fovY": 44},
            "nodes": [
                {"type": "character", "name": "Biscuit",
                 "position": [0, 0, 0], "height": 1.15, "perspective": true,
                 "look": {"body": "#E8913A", "hat": "none", "kid": true,
                          "neck": "collar", "neckCol": "#B5372E"},
                 "state": {"mood": "proud", "armR": [50, -40], "blink": 0}}
            ]
        }"##;
        let root = crate::scene::parse_json(text).expect("scene parses");
        let folded = crate::eval15::process_scene(root, &[]).expect("fold is clean");
        let mut intern = Interner::default();
        let mut unresolved = 0usize;
        let cuts = cutouts_from_root(
            &folded,
            Path::new("."),
            "s01",
            &mut intern,
            &mut unresolved,
        )
        .expect("cutouts build");
        assert_eq!(cuts.len(), 1);
        assert_eq!(cuts[0].id, "Biscuit");
        assert!(cuts[0].sig.contains("\"body\":\"#E8913A\""), "sig: {}", cuts[0].sig);
        assert!(cuts[0].sig.contains("\"kid\":true"), "sig: {}", cuts[0].sig);
        assert!(cuts[0].overflow.is_some());
        assert!(cuts[0].box_.is_some());
        assert!(cuts[0].pose.starts_with("mood=proud"), "pose: {}", cuts[0].pose);
        assert_eq!(unresolved, 0);
    }
}
