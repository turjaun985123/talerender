//! Production integration: the complete Onoda film, rendered
//! natively end to end.
//!
//! The bridge (docs/integration/README.md): the project's staging
//! (build/src/shots.ts) is ported formula-for-formula into the
//! evaluator below; everything the staging reads from project DATA
//! (line timings, word anchors, caption chunks) is generated into
//! film/onoda.json by film/gen_film_data.py, and every 2D plate
//! (held props, caption strips, calendar cards) is rendered by
//! film/gen_assets.py from the project's own fonts. Text enters
//! as data; the renderer typesets nothing.
//!
//! Determinism law (contract A4): a frame is a pure function of
//! its timestamp — the evaluator reads no wall clock and no RNG
//! state beyond the project's own sin-hash (scene::rnd). Chunked
//! rendering with resume therefore cannot change any frame; the
//! driver skips frames whose PNG already exists.
//!
//! The GPU chain is the M5 production chain (shadow -> bokeh
//! depth prepass -> PBR MSAA HDR -> bokeh -> bloom -> output ->
//! GRADE), structured like ab1::run_ab1 but parameterised per
//! frame. Calendar, film fade and captions composite on the CPU
//! after readback, in main.ts's order (shot draw incl. calendar,
//! then fade, then caption) — the same sRGB-space overlay point
//! ab1 used for its caption.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::ab1::{MapDraw, StdDraw, StdMat};
use crate::composer::{BloomParams, GradeParams, PostChain};
use crate::math::{self, Mat4};
use crate::scene::{
    box_geo, cone_geo, cyl_geo, flat_quad_geo, ground_disc_geo, hex_lin, paint_blob, paint_cloud,
    paint_grass, paint_sky, quad_geo, rounded_rect_pts, rnd, shade_lin, sky_sphere_geo, sphere_geo,
    sphere_map_geo, Geo, MapGeo, Painter, TexDraw, TexGeo, Value,
};
use crate::toonfull::{paint_toon_full, FullLook, FullState, Legs, Mood, Plate};

// ---------------------------------------------------------------------
// Project data (film/onoda.json)
// ---------------------------------------------------------------------

pub(crate) struct FilmData {
    pub duration: f64,
    pub anchors: HashMap<String, f64>,
    pub captions: Vec<Caption>,
    pub lines: Vec<(f64, f64)>, // (start, end) per line index
}

pub(crate) struct Caption {
    pub s: f64,
    pub e: f64,
    pub text: String,
    pub fit: f64,
    pub strip: String,
}

fn jnum(v: &Value, ctx: &str) -> Result<f64, String> {
    v.as_f64().ok_or_else(|| format!("{ctx}: expected number"))
}

pub(crate) fn load_film_data(dir: &Path) -> Result<FilmData, String> {
    let text = std::fs::read_to_string(dir.join("onoda.json"))
        .map_err(|e| format!("cannot read film data: {e}"))?;
    let root = crate::scene::parse_json(&text)?;
    let g = |k: &str| root.get(k).ok_or_else(|| format!("film data: missing {k}"));
    let mut anchors = HashMap::new();
    if let Value::Obj(m) = g("anchors")? {
        for (k, v) in m {
            anchors.insert(k.clone(), jnum(v, "anchor")?);
        }
    }
    let mut lines = Vec::new();
    if let Value::Arr(a) = g("lines")? {
        for l in a {
            lines.push((jnum(l.get("start").ok_or("line.start")?, "line.start")?,
                        jnum(l.get("end").ok_or("line.end")?, "line.end")?));
        }
    }
    let mut captions = Vec::new();
    if let Value::Arr(a) = g("captions")? {
        for c in a {
            captions.push(Caption {
                s: jnum(c.get("s").ok_or("cap.s")?, "cap.s")?,
                e: jnum(c.get("e").ok_or("cap.e")?, "cap.e")?,
                text: c.get("text").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                fit: jnum(c.get("fit").ok_or("cap.fit")?, "cap.fit")?,
                strip: c.get("strip").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            });
        }
    }
    Ok(FilmData {
        duration: jnum(g("duration")?, "duration")?,
        anchors,
        captions,
        lines,
    })
}

// ---------------------------------------------------------------------
// Motion formulas — core/lib.ts, core/moves.ts, core/poses.ts,
// transcribed 1:1 (f64, like the JS originals).
// ---------------------------------------------------------------------

pub(crate) fn clamp(x: f64, a: f64, b: f64) -> f64 {
    x.max(a).min(b)
}
pub(crate) fn lerp(a: f64, b: f64, k: f64) -> f64 {
    a + (b - a) * k
}
pub(crate) fn seg(t: f64, a: f64, b: f64) -> f64 {
    clamp((t - a) / if b - a == 0.0 { 1e-6 } else { b - a }, 0.0, 1.0)
}
pub(crate) fn eo(u: f64) -> f64 {
    1.0 - (1.0 - u).powi(3)
}
pub(crate) fn eio(u: f64) -> f64 {
    if u < 0.5 {
        4.0 * u * u * u
    } else {
        1.0 - (-2.0 * u + 2.0).powi(3) / 2.0
    }
}
pub(crate) fn eob(u: f64) -> f64 {
    let c1 = 1.70158;
    let c3 = c1 + 1.0;
    1.0 + c3 * (u - 1.0).powi(3) + c1 * (u - 1.0).powi(2)
}
pub(crate) fn smoother(u: f64) -> f64 {
    let u = clamp(u, 0.0, 1.0);
    u * u * u * (u * (u * 6.0 - 15.0) + 10.0)
}
/// camK(lt, d, hold): smootherstep over the shot with hold-in/out.
pub(crate) fn cam_k(lt: f64, d: f64, hold: f64) -> f64 {
    let h = clamp(hold, 0.0, 0.45);
    smoother((lt / d - h) / (1.0 - 2.0 * h))
}
/// One poseTrack channel key: (time, value, moveSecs).
pub(crate) type Key = (f64, f64, f64);
pub(crate) fn key(t: f64, v: f64) -> Key {
    (t, v, 0.35)
}
pub(crate) fn keym(t: f64, v: f64, m: f64) -> Key {
    (t, v, m)
}
/// poseTrack(keys, t): hold each value until the next key's
/// (time - moveSecs), then lerp with that key's curve (the film
/// uses the default smootherstep throughout).
pub(crate) fn pose_track(keys: &[Key], t: f64) -> f64 {
    if keys.is_empty() {
        return 0.0;
    }
    let mut cur = keys[0].1;
    for w in keys.windows(2) {
        let (t0, v0, _) = w[0];
        let (t1, v1, m1) = w[1];
        let _ = t0;
        if t < t1 - m1 {
            return cur;
        }
        if t < t1 {
            return lerp(v0, v1, smoother((t - (t1 - m1)) / m1));
        }
        cur = v1;
    }
    cur
}
pub(crate) fn walk_phase(dist: f64, stride: f64) -> f64 {
    dist / stride * std::f64::consts::PI
}
pub(crate) fn walk_bob(phase: f64, amp: f64) -> f64 {
    amp * (1.0 - phase.sin().abs())
}
pub(crate) struct Blinker {
    pub seed: f64,
}
impl Blinker {
    /// blinkDirector(seed), transcribed from motion.ts: the
    /// interval pattern [2.7, 3.9, 3.1, 4.4, 2.9, 3.6] is rotated
    /// by floor(seed*7) mod 6, the clock offset by seed*1.37, and
    /// the eyes are shut for the first 0.12 s of each interval.
    pub(crate) fn at(&self, t: f64) -> f64 {
        const IV: [f64; 6] = [2.7, 3.9, 3.1, 4.4, 2.9, 3.6];
        let s = self.seed;
        let x = t + s * 1.37;
        if !(x >= 0.0) {
            return 0.0;
        }
        let start = (((s * 7.0).floor() as i64 % 6) + 6) % 6;
        let mut acc = 0.0f64;
        let mut i = start;
        while x > acc + IV[(i % 6) as usize] {
            acc += IV[(i % 6) as usize];
            i += 1;
        }
        if x - acc < 0.12 {
            1.0
        } else {
            0.0
        }
    }
}
/// blinkAt(t, beats), transcribed: 1 while 0 <= t - beat < 0.12.
pub(crate) fn blink_at(t: f64, beats: &[f64]) -> f64 {
    if beats.iter().any(|b| t - b >= 0.0 && t - b < 0.12) {
        1.0
    } else {
        0.0
    }
}
pub(crate) fn blink_max(a: f64, b: f64) -> f64 {
    a.max(b)
}
pub(crate) fn settle(t: f64, t0: f64, freq: f64, decay: f64) -> f64 {
    let dt = t - t0;
    if dt > 0.0 {
        (dt * freq * std::f64::consts::TAU).sin() * (-dt * decay).exp()
    } else {
        0.0
    }
}
pub(crate) fn talk(t: f64, on: bool) -> f64 {
    if on {
        0.25 + 0.75 * (13.0 * t).sin().abs() * (0.6 + 0.4 * (5.3 * t).sin())
    } else {
        0.0
    }
}
pub(crate) fn mix3(a: [f64; 3], b: [f64; 3], k: f64) -> [f32; 3] {
    [
        lerp(a[0], b[0], k) as f32,
        lerp(a[1], b[1], k) as f32,
        lerp(a[2], b[2], k) as f32,
    ]
}

// ---------------------------------------------------------------------
// Staging output: one evaluated frame.
// ---------------------------------------------------------------------

#[derive(Clone)]
pub(crate) struct PointState {
    pub pos: [f32; 3],
    pub col: [f32; 3], // folded linear (colour x intensity)
    pub dist: f32,
    pub decay: f32,
}
impl PointState {
    pub(crate) fn off() -> Self {
        PointState { pos: [0.0; 3], col: [0.0; 3], dist: 0.0, decay: 0.0 }
    }
}

#[derive(Clone)]
pub(crate) struct LightState {
    pub key_pos: [f32; 3],
    pub key_tgt: [f32; 3],
    pub key_col: [f32; 3], // folded linear
    pub key_box: f32,
    pub key_near: f32,
    pub key_far: f32,
    pub key_bias: f32,
    pub rim_col: [f32; 3], // folded linear
    pub rim_dir: [f32; 3], // direction TO the rim light
    pub hemi_sky: [f32; 3], // folded linear
    pub hemi_ground: [f32; 3],
    pub points: [PointState; 3],
    pub env: f32,
}

#[derive(Clone, Copy)]
pub(crate) struct PostState {
    pub bloom: f32,
    pub aperture: f32,
    pub maxblur: f32,
    pub sat: f32,
    pub warm: f32,
    pub vig: f32,
    pub grain: f32,
    pub exposure: f32,
}

#[derive(Clone)]
pub(crate) enum TexSrc {
    S(usize),   // static texture registry index
    Dyn(usize), // per-frame uploaded texture slot (character cards)
}

pub(crate) struct BasicDraw {
    pub geo: TexGeo,
    pub model: Mat4,
    pub tex: TexSrc,
    pub tint: [f32; 3],
    pub opacity: f32,
    pub alpha_test: f32,
    pub depth_write: bool,
    pub fog: bool,
    pub prepass: bool,
    pub repeat: bool,
}

/// A textured PBR draw on the ab1 "ground" pattern (TexGeo +
/// constant normal + has_map), generalised to any texture.
pub(crate) struct TexMapDraw {
    pub geo: TexGeo,
    pub model: Mat4,
    pub rough: f32,
    pub receive: bool,
    pub tex: usize,
    pub repeat: bool,
    pub nrm: [f32; 3], // constant local-space normal (vs_tex pattern)
}

pub(crate) struct CharDraw {
    pub look: usize, // index into the look registry
    pub pos: [f32; 3],
    pub state: FullState,
    pub yaw: f32,   // face() model yaw, set by finish_chars
    pub pitch: f32, // face() model pitch (perspective instances)
    pub tint: [f32; 3], // matchEnv multiplier
    pub rifle: bool,    // rifle attached to hand R
    pub hold_l: Option<&'static str>, // plate name, resolved by the driver
    pub hold_r: Option<&'static str>,
}

#[derive(Clone)]
pub(crate) struct CalendarState {
    pub year: usize, // 0..4 -> 1945/1955/1965/1974
    pub sy: f32,     // vertical flip scale 0.1..1
    pub dark: f32,   // flip shading 0..1
    pub alpha: f32,
}

pub(crate) struct FrameSpec {
    pub std: Vec<StdDraw>,
    pub mapped: Vec<MapDraw>,
    pub map_tex: Vec<usize>,
    pub texmapped: Vec<TexMapDraw>,
    pub basics: Vec<BasicDraw>,
    pub chars: Vec<CharDraw>,
    pub lights: LightState,
    pub fog: Option<([f32; 3], f32, f32)>, // linear colour, near, far
    pub eye: [f32; 3],
    pub target: [f32; 3],
    pub fov: f32,
    pub focus: f32,
    pub post: PostState,
    pub caption: Option<usize>,
    pub caption_scale: f32,
    pub calendar: Option<CalendarState>,
    pub fade: f32, // 0 = clear, 1 = full black
    pub clear: [f32; 3], // scene.background (linear); BK's #1A1418
}

impl FrameSpec {
    fn new(lights: LightState, post: PostState) -> Self {
        FrameSpec {
            std: Vec::new(),
            mapped: Vec::new(),
            map_tex: Vec::new(),
            texmapped: Vec::new(),
            basics: Vec::new(),
            chars: Vec::new(),
            lights,
            fog: None,
            eye: [0.0; 3],
            target: [0.0; 3],
            fov: 44.0,
            focus: 6.0,
            post,
            caption: None,
            caption_scale: 1.0,
            calendar: None,
            fade: 0.0,
            clear: [0.0; 3],
        }
    }
}

// Static texture registry indices.
pub(crate) const TEX_SKY_JG: usize = 0;
pub(crate) const TEX_SKY_SH: usize = 1;
pub(crate) const TEX_GRASS: usize = 2;
pub(crate) const TEX_HILLGRASS: usize = 3;
pub(crate) const TEX_CLOUD: usize = 4;
pub(crate) const TEX_BLOB: usize = 5;
pub(crate) const TEX_PLANKS_DARK: usize = 6; // #6B3E26 (desk wood, wainscot)
pub(crate) const TEX_PLANKS_FLOOR: usize = 7; // #7A5230 (BK floor is planks texture, white material)
pub(crate) const TEX_WALLPAPER: usize = 8; // #5A4632 / motif #6B5638
pub(crate) const TEX_PARCEL: usize = 9;
pub(crate) const TEX_SIGN: usize = 10;
pub(crate) const TEX_PICTURE: usize = 11;
pub(crate) const TEX_FELT_DARK: usize = 12; // #2E5A44 desk top
pub(crate) const TEX_FELT_COUNTER: usize = 13; // #1F5A45 counter top
pub(crate) const TEX_VIEWPLANE: usize = 14; // 1x1 #BFE4F7
pub(crate) const TEX_PLANKS_CRATE: usize = 15; // #B07A48 rep baked in uv
pub(crate) const TEX_WALLPAPER_CEIL: usize = 16; // #3A2E22 / shade 1.25
pub(crate) const TEX_PLANKS_COUNTER: usize = 17; // #7A3E24

// ---------------------------------------------------------------------
// Cast looks (shots.ts LOOK table).
// ---------------------------------------------------------------------

fn fl(body: &str, hat: &str, hat_col: Option<&str>) -> FullLook {
    FullLook {
        body: body.to_string(),
        head: "#F7F7FA".to_string(),
        shade: None,
        pattern: "plain".to_string(),
        pat_col: None,
        hat: hat.to_string(),
        hat_col: hat_col.map(|s| s.to_string()),
        band: None,
        neck: "none".to_string(),
        neck_col: None,
        glasses: false,
        mustache: false,
        mustache_col: None,
        kid: false,
    }
}

pub(crate) fn look_registry() -> Vec<(FullLook, bool, f64)> {
    // (look, perspective, blink seed) per instance index:
    // 0 jgOnoda, 1 jgCompA, 2 jgCompB, 3 jgSuzuki, 4 jgTani,
    // 5 shOnoda, 6 shCompA, 7 shCompB, 8 shTani, 9 inTani,
    // 10 jgOnoda2 (the dusk instance swap)
    let onoda_jg = || {
        let mut l = fl("#5C6644", "cap", Some("#4C5638"));
        l.shade = Some("#878E6E".to_string());
        (l, true, 3.0)
    };
    let onoda_sh = || (fl("#5C6644", "cap", Some("#4C5638")), false, 3.0);
    let comp_a = || (fl("#68744E", "cap", Some("#525C3C")), false, 7.0);
    let comp_b = || {
        let mut l = fl("#59613F", "cap", Some("#59613F"));
        l.neck = "scarf".to_string();
        l.neck_col = Some("#8A7A52".to_string());
        (l, false, 11.0)
    };
    let suzuki = || {
        let mut l = fl("#C96A42", "cap", Some("#E8B83A"));
        l.glasses = true;
        (l, false, 5.0)
    };
    let tani = || {
        let mut l = fl("#4A4560", "bowler", Some("#2E2A3A"));
        l.pattern = "vest".to_string();
        l.pat_col = Some("#33304A".to_string());
        l.neck = "tie".to_string();
        l.neck_col = Some("#7A2E3A".to_string());
        l.glasses = true;
        l.mustache = true;
        l.mustache_col = Some("#C9C9D2".to_string());
        (l, false, 9.0)
    };
    vec![
        onoda_jg(),
        comp_a(),
        comp_b(),
        suzuki(),
        tani(),
        onoda_sh(),
        comp_a(),
        comp_b(),
        tani(),
        tani(),
        onoda_jg(),
    ]
}

// ---------------------------------------------------------------------
// Texture painters the ab1 set did not need (world.ts recipes).
// ---------------------------------------------------------------------

fn rgb_of(hex: &str) -> (u8, u8, u8) {
    crate::scene::hex_rgb(hex)
}
fn shade_of(hex: &str, k: f32) -> (u8, u8, u8) {
    crate::scene::shade_rgb(hex, k)
}

/// T.planks(col): 512x512, 8 rows of hash-shaded boards with grain
/// beziers, seams and nail dots (world.ts:58-68).
pub(crate) fn paint_planks(col: &str) -> (Vec<u8>, u32, u32) {
    let (w, h) = (512usize, 512usize);
    let mut p = Painter::new(w, h);
    let n = 8.0f32;
    let ph = h as f32 / n;
    for i in 0..8 {
        let fi = i as f64;
        let sh = shade_of(col, (0.86 + rnd(fi * 3.0) * 0.24) as f32);
        p.fill_rect(0.0, i as f32 * ph, w as f32, ph, (sh.0, sh.1, sh.2, 255));
        let seam = shade_of(col, 0.7);
        for k in 0..4 {
            let y = i as f32 * ph + 8.0 + (rnd(fi * 7.0 + k as f64) * (ph as f64 - 16.0)) as f32;
            let mut pts = Vec::new();
            for s in 0..=16 {
                let u = s as f32 / 16.0;
                let bx = u * w as f32;
                let by = (1.0 - u).powi(2) * y
                    + 2.0 * (1.0 - u) * u * (y + if s % 2 == 0 { 6.0 } else { -6.0 })
                    + u * u * (y + 2.0);
                pts.push((bx, by));
            }
            // canvas strokes at globalAlpha 0.35
            p.stroke_polyline(&pts, 2.0, (seam.0, seam.1, seam.2, 89));
        }
        let dark = shade_of(col, 0.45);
        p.fill_rect(0.0, i as f32 * ph, w as f32, 4.0, (dark.0, dark.1, dark.2, 255));
        let jx = (rnd(fi * 11.0) * w as f64) as f32;
        p.fill_rect(jx, i as f32 * ph, 4.0, ph, (dark.0, dark.1, dark.2, 255));
        let nail = shade_of(col, 0.5);
        for nx in [jx - 14.0, jx + 18.0] {
            p.fill_ellipse(nx, i as f32 * ph + ph / 2.0, 3.0, 3.0, 0.0, (nail.0, nail.1, nail.2, 255));
        }
    }
    (p.px, w as u32, h as u32)
}

/// T.wallpaper(col, motif): 256x256, fleur motifs at (64,64) and
/// (192,192) at scale 0.9 + alpha-0.5 dots (world.ts:69-74).
pub(crate) fn paint_wallpaper(col: &str, motif: &str) -> (Vec<u8>, u32, u32) {
    let (w, h) = (256usize, 256usize);
    let mut p = Painter::new(w, h);
    let c = rgb_of(col);
    p.fill_rect(0.0, 0.0, w as f32, h as f32, (c.0, c.1, c.2, 255));
    let m = rgb_of(motif);
    let fleur = |p: &mut Painter, cx: f32, cy: f32| {
        // bezier petal (0,-40)->(0,14) via (18,-20),(18,0)/(-18,0),(-18,-20)
        let mut pts = Vec::new();
        for s in 0..=12 {
            let u = s as f32 / 12.0;
            let (x, y) = if u < 0.5 {
                let v = u * 2.0;
                (
                    (1.0 - v).powi(3) * 0.0 + 3.0 * (1.0 - v).powi(2) * v * 18.0 + 3.0 * (1.0 - v) * v * v * 18.0 + v.powi(3) * 0.0,
                    (1.0 - v).powi(3) * -40.0 + 3.0 * (1.0 - v).powi(2) * v * -20.0 + 3.0 * (1.0 - v) * v * v * 0.0 + v.powi(3) * 14.0,
                )
            } else {
                let v = (u - 0.5) * 2.0;
                (
                    (1.0 - v).powi(3) * 0.0 + 3.0 * (1.0 - v).powi(2) * v * -18.0 + 3.0 * (1.0 - v) * v * v * -18.0 + v.powi(3) * 0.0,
                    (1.0 - v).powi(3) * 14.0 + 3.0 * (1.0 - v).powi(2) * v * 0.0 + 3.0 * (1.0 - v) * v * v * -20.0 + v.powi(3) * -40.0,
                )
            };
            pts.push((cx + x * 0.9, cy + y * 0.9));
        }
        p.fill_polygon(&pts, (m.0, m.1, m.2, 255));
        p.fill_ellipse(cx - 22.0 * 0.9, cy + 4.0 * 0.9, 12.0 * 0.9, 12.0 * 0.9, 0.0, (m.0, m.1, m.2, 255));
        p.fill_ellipse(cx + 22.0 * 0.9, cy + 4.0 * 0.9, 12.0 * 0.9, 12.0 * 0.9, 0.0, (m.0, m.1, m.2, 255));
        p.fill_rect(cx - 3.0 * 0.9, cy + 10.0 * 0.9, 6.0 * 0.9, 26.0 * 0.9, (m.0, m.1, m.2, 255));
    };
    fleur(&mut p, 64.0, 64.0);
    fleur(&mut p, 192.0, 192.0);
    for i in 0..4 {
        let (x, y) = if i % 2 == 1 { (192.0, 192.0) } else { (64.0, 64.0) };
        let _ = (x, y);
        let (dx, dy) = match i {
            0 => (64.0, 192.0),
            1 => (192.0, 192.0),
            2 => (64.0, 64.0),
            _ => (192.0, 64.0),
        };
        p.fill_ellipse(dx, dy, 6.0, 6.0, 0.0, (m.0, m.1, m.2, 128));
    }
    (p.px, w as u32, h as u32)
}

/// T.felt(col): 256x256 hash speckle (world.ts:93).
pub(crate) fn paint_felt(col: &str) -> (Vec<u8>, u32, u32) {
    let (w, h) = (256usize, 256usize);
    let mut p = Painter::new(w, h);
    let c = rgb_of(col);
    p.fill_rect(0.0, 0.0, w as f32, h as f32, (c.0, c.1, c.2, 255));
    for i in 0..3000 {
        let fi = i as f64;
        let sh = shade_of(col, (0.85 + rnd(fi) * 0.3) as f32);
        p.fill_rect(
            (rnd(fi * 3.0) * w as f64) as f32,
            (rnd(fi * 7.0) * h as f64) as f32,
            2.0,
            2.0,
            (sh.0, sh.1, sh.2, 255),
        );
    }
    (p.px, w as u32, h as u32)
}

/// PR.parcel texture (label = false for this film): 256x256
/// (props.ts:159).
pub(crate) fn paint_parcel() -> (Vec<u8>, u32, u32) {
    let (w, h) = (256usize, 256usize);
    let mut p = Painter::new(w, h);
    let c = rgb_of("#C8935A");
    p.fill_rect(0.0, 0.0, w as f32, h as f32, (c.0, c.1, c.2, 255));
    let sp = rgb_of("#B07A48");
    for i in 0..30 {
        let fi = i as f64;
        p.fill_rect((rnd(fi) * 256.0) as f32, (rnd(fi * 3.0) * 256.0) as f32, 40.0, 2.0, (sp.0, sp.1, sp.2, 255));
    }
    let tape = rgb_of("#E9D7A5");
    p.fill_rect(120.0, 0.0, 16.0, 256.0, (tape.0, tape.1, tape.2, 255));
    p.fill_rect(0.0, 120.0, 256.0, 16.0, (tape.0, tape.1, tape.2, 255));
    (p.px, w as u32, h as u32)
}

/// interior 'pic' canvas (world.ts:319): hills line + sun disc.
pub(crate) fn paint_picture() -> (Vec<u8>, u32, u32) {
    let (w, h) = (256usize, 200usize);
    let mut p = Painter::new(w, h);
    let c = rgb_of("#E9DCC0");
    p.fill_rect(0.0, 0.0, w as f32, h as f32, (c.0, c.1, c.2, 255));
    let ink = rgb_of("#3A2A22");
    let mut pts = Vec::new();
    for s in 0..=20 {
        let u = s as f32 / 20.0;
        let (x, y) = if u < 0.5 {
            let v = u * 2.0;
            (
                (1.0 - v).powi(2) * 40.0 + 2.0 * (1.0 - v) * v * 90.0 + v * v * 140.0,
                (1.0 - v).powi(2) * 140.0 + 2.0 * (1.0 - v) * v * 60.0 + v * v * 120.0,
            )
        } else {
            let v = (u - 0.5) * 2.0;
            (
                (1.0 - v).powi(2) * 140.0 + 2.0 * (1.0 - v) * v * 170.0 + v * v * 220.0,
                (1.0 - v).powi(2) * 120.0 + 2.0 * (1.0 - v) * v * 80.0 + v * v * 140.0,
            )
        };
        pts.push((x, y));
    }
    p.stroke_polyline(&pts, 6.0, (ink.0, ink.1, ink.2, 255));
    let sun = rgb_of("#C9603F");
    p.fill_ellipse(190.0, 60.0, 22.0, 22.0, 0.0, (sun.0, sun.1, sun.2, 255));
    (p.px, w as u32, h as u32)
}

// ---------------------------------------------------------------------
// Geometry / draw helpers (mirroring ab1.rs's local conventions).
// ---------------------------------------------------------------------

pub(crate) fn t3(x: f32, y: f32, z: f32) -> Mat4 {
    [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [x, y, z, 1.0],
    ]
}
pub(crate) fn sm(rough: f32, metal: f32) -> StdMat {
    StdMat { rough, metal, emissive: [0.0; 3] }
}
pub(crate) fn sme(rough: f32, metal: f32, emissive: [f32; 3]) -> StdMat {
    StdMat { rough, metal, emissive }
}
pub(crate) fn add(draws: &mut Vec<StdDraw>, geo: Geo, model: Mat4, mat: StdMat, cast: bool, receive: bool) {
    draws.push(StdDraw { geo, model, mat, cast, receive });
}
fn def() -> StdMat {
    sm(0.85, 0.0)
}

fn add_tree(draws: &mut Vec<StdDraw>, pos: [f32; 3], rot_y: f32, s: f32, col: &str, round: bool, on_hill: bool) {
    // world.ts tree(), as transcribed in ab1.rs (canopy spheres
    // approximate the original's icosahedra — gaps.md).
    let base = math::mul(&t3(pos[0], pos[1], pos[2]), &math::rotate_y(rot_y));
    add(
        draws,
        cyl_geo(0.12 * s, 0.18 * s, 1.6 * s, 7, hex_lin("#7A5234")),
        math::mul(&base, &t3(0.0, 0.8 * s, 0.0)),
        sm(0.85, 0.0),
        !on_hill,
        true,
    );
    if round {
        let blobs: [(f32, f32, f32, f32); 5] = [
            (0.0, 2.3, 0.0, 1.0),
            (0.6, 2.0, 0.2, 0.75),
            (-0.6, 2.0, -0.1, 0.8),
            (0.1, 2.9, -0.2, 0.75),
            (0.2, 2.2, 0.6, 0.7),
        ];
        for (i, &(ox, oy, oz, f)) in blobs.iter().enumerate() {
            let c = shade_lin(col, (0.9 + rnd(i as f64) * 0.2) as f32);
            add(draws, sphere_geo(f * s, 12, 9, c), math::mul(&base, &t3(ox * s, oy * s, oz * s)), sm(0.85, 0.0), !on_hill, false);
        }
    } else {
        let model = math::mul(
            &math::mul(&base, &t3(0.0, 3.2 * s, 0.0)),
            &math::scale(0.7 * s, 2.6 * s, 0.7 * s),
        );
        add(draws, sphere_geo(1.0, 16, 12, hex_lin(col)), model, sm(0.85, 0.0), !on_hill, false);
    }
}

fn add_palm(draws: &mut Vec<StdDraw>, pos: [f32; 3], s: f32, lean: f32, rot_y: f32) {
    // shots.ts palm(), as transcribed in ab1.rs.
    let base = math::mul(
        &t3(pos[0], pos[1], pos[2]),
        &math::mul(&math::rotate_y(rot_y), &math::mul(&math::rotate_z(lean), &math::scale(s, s, s))),
    );
    add(draws, cyl_geo(0.085, 0.13, 4.4, 8, hex_lin("#7A6248")), math::mul(&base, &t3(0.0, 2.2, 0.0)), sm(0.95, 0.0), true, true);
    let top_y = 4.35f32;
    let leaf = hex_lin("#3F7D33");
    let leaf2 = hex_lin("#4C8A3A");
    for i in 0..7 {
        let a = (i as f32 / 7.0) * std::f32::consts::TAU + 0.4;
        let (sa, ca) = a.sin_cos();
        let col = if i % 2 == 0 { leaf } else { leaf2 };
        let frond_model = math::mul(
            &math::mul(&base, &t3(sa * 1.0, top_y + 0.1 - (i % 3) as f32 * 0.07, ca * 1.0)),
            &math::mul(&math::rotate_y(a), &math::scale(0.17, 0.05, 1.2)),
        );
        add(draws, sphere_geo(1.0, 10, 6, col), frond_model, sm(0.9, 0.0), true, false);
        let tip_model = math::mul(
            &math::mul(&base, &t3(sa * 1.85, top_y - 0.28 - (i % 3) as f32 * 0.06, ca * 1.85)),
            &math::mul(&math::euler_xyz(ca * 0.5, a, -sa * 0.5), &math::scale(0.13, 0.04, 0.8)),
        );
        add(draws, sphere_geo(1.0, 10, 6, col), tip_model, sm(0.9, 0.0), true, false);
    }
    let nut = hex_lin("#6B4A2E");
    for &(nx, nz) in [(0.14f32, 0.0f32), (-0.12, 0.1), (0.0, -0.14)].iter() {
        add(draws, sphere_geo(0.13, 10, 8, nut), math::mul(&base, &t3(nx, top_y - 0.2, nz)), sm(0.9, 0.0), true, false);
    }
}

/// Box with a diffuse map on every face (MapGeo, 44-float stride):
/// corners are ordered top-left/top-right/bottom-right/bottom-left
/// as seen from outside each face, uv (0,0) at the top-left (the
/// Painter-image convention the PBR shader samples), scaled by
/// `uv_scale` (the texture repeat, baked in like the ground's).
pub(crate) fn box_map_geo(w: f32, h: f32, d: f32, col: [f32; 3], uv_scale: [f32; 2]) -> MapGeo {
    let (hw, hh, hd) = (w / 2.0, h / 2.0, d / 2.0);
    let mut g = MapGeo { verts: Vec::new(), idx: Vec::new() };
    let faces: [([f32; 3], [[f32; 3]; 4]); 6] = [
        ([0.0, 0.0, 1.0], [[-hw, hh, hd], [hw, hh, hd], [hw, -hh, hd], [-hw, -hh, hd]]),
        ([0.0, 0.0, -1.0], [[hw, hh, -hd], [-hw, hh, -hd], [-hw, -hh, -hd], [hw, -hh, -hd]]),
        ([1.0, 0.0, 0.0], [[hw, hh, hd], [hw, hh, -hd], [hw, -hh, -hd], [hw, -hh, hd]]),
        ([-1.0, 0.0, 0.0], [[-hw, hh, -hd], [-hw, hh, hd], [-hw, -hh, hd], [-hw, -hh, -hd]]),
        ([0.0, 1.0, 0.0], [[-hw, hh, -hd], [hw, hh, -hd], [hw, hh, hd], [-hw, hh, hd]]),
        ([0.0, -1.0, 0.0], [[-hw, -hh, hd], [hw, -hh, hd], [hw, -hh, -hd], [-hw, -hh, -hd]]),
    ];
    let uvs = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
    for (n, corners) in faces {
        let base = (g.verts.len() / 11) as u32;
        for (c, uv) in corners.iter().zip(uvs) {
            g.verts.extend_from_slice(c);
            g.verts.extend_from_slice(&n);
            g.verts.extend_from_slice(&col);
            g.verts.extend_from_slice(&[uv[0] * uv_scale[0], uv[1] * uv_scale[1]]);
        }
        g.idx.extend_from_slice(&[base, base + 3, base + 2, base, base + 2, base + 1]);
    }
    g
}

fn add_mapped(f: &mut FrameSpec, geo: MapGeo, model: Mat4, tex: usize, mat: StdMat, cast: bool, receive: bool) {
    f.mapped.push(MapDraw { geo, model, mat, cast, receive });
    f.map_tex.push(tex);
}

fn s3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] { [a[0] - b[0], a[1] - b[1], a[2] - b[2]] }
fn x3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}
fn n3(a: [f32; 3]) -> [f32; 3] {
    let l = (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt();
    if l > 0.0 { [a[0] / l, a[1] / l, a[2] / l] } else { a }
}

/// Cloud sprites for a camera at `eye` looking at `target`
/// (three Sprite billboarding; world.ts clouds()).
fn add_clouds(f: &mut FrameSpec, list: &[([f32; 3], f32)], eye: [f32; 3], target: [f32; 3]) {
    let fwd = n3(s3(target, eye));
    let right = n3(x3(fwd, [0.0, 1.0, 0.0]));
    let upv = x3(right, fwd);
    for &(c, s) in list {
        let (hw, hh) = (s, s / 2.0);
        let corner = |sx: f32, sy: f32| -> [f32; 3] {
            [
                c[0] + right[0] * hw * sx + upv[0] * hh * sy,
                c[1] + right[1] * hw * sx + upv[1] * hh * sy,
                c[2] + right[2] * hw * sx + upv[2] * hh * sy,
            ]
        };
        let mut g = TexGeo::new();
        g.vert(corner(-1.0, -1.0), [0.0, 1.0]);
        g.vert(corner(1.0, -1.0), [1.0, 1.0]);
        g.vert(corner(1.0, 1.0), [1.0, 0.0]);
        g.vert(corner(-1.0, 1.0), [0.0, 0.0]);
        g.tri(0, 1, 2);
        g.tri(0, 2, 3);
        f.basics.push(BasicDraw {
            geo: g,
            model: math::identity(),
            tex: TexSrc::S(TEX_CLOUD),
            tint: [1.0, 1.0, 1.0],
            opacity: 1.0,
            alpha_test: 0.0,
            depth_write: false,
            fog: false,
            prepass: false,
            repeat: false,
        });
    }
}

fn add_sky(f: &mut FrameSpec, tex: usize) {
    f.basics.push(BasicDraw {
        geo: sky_sphere_geo(380.0, 32, 16),
        model: math::identity(),
        tex: TexSrc::S(tex),
        tint: [1.0, 1.0, 1.0],
        opacity: 1.0,
        alpha_test: 0.0,
        depth_write: false,
        fog: false,
        prepass: true,
        repeat: false,
    });
}

fn hills(f: &mut FrameSpec, z: f32) {
    let list: [(f32, f32, f32, f32); 4] = [
        (-40.0, z, 34.0, 12.0),
        (10.0, z - 10.0, 46.0, 16.0),
        (60.0, z + 4.0, 36.0, 11.0),
        (-90.0, z + 10.0, 40.0, 10.0),
    ];
    for (i, &(hx, hz, r, h)) in list.iter().enumerate() {
        let col = shade_lin("#5E8F52", (0.92 + rnd(i as f64) * 0.16) as f32);
        let model = math::mul(&t3(hx, -h * 0.25, hz), &math::scale(r, h, r * 0.6));
        add_mapped(f, sphere_map_geo(1.0, 32, 16, col, [6.0, 3.0]), model, TEX_HILLGRASS, def(), false, false);
        for k in 0..6 {
            let a = rnd(i as f64 * 9.0 + k as f64) * 2.0 - 1.0;
            let s = (0.5 + rnd(k as f64) * 0.4) as f32 * 2.2;
            let pos = [hx + (a as f32) * r * 0.6, (h as f64 * (0.75 - a.abs() * 0.45) - 1.0) as f32, hz + r * 0.25];
            add_tree(&mut f.std, pos, 0.0, s, "#3E7A34", true, true);
        }
    }
}

// ---------------------------------------------------------------------
// JG — the jungle camp set (shots.ts makeJungle).
// ---------------------------------------------------------------------

pub(crate) struct JgDyn {
    pub fire: f64, // fireLevel 0..1
    pub t: f64,    // film time (flame flicker)
    pub extra_std: Vec<StdDraw>, // plane + leaflets + drop leaf + fallen rifle
}

pub(crate) fn build_jg(f: &mut FrameSpec, jd: JgDyn) {
    add_sky(f, TEX_SKY_JG);
    // grass ground (textured, the ab1 ground pattern)
    f.texmapped.push(TexMapDraw {
        geo: ground_disc_geo(80.0, 48, 48.0),
        model: math::identity(),
        rough: 0.85,
        receive: true,
        tex: TEX_GRASS,
        repeat: true,
        nrm: [0.0, 1.0, 0.0],
    });
    add(&mut f.std, crate::scene::disc_geo(4.8, 28, hex_lin("#86714C")), t3(0.0, 0.012, -0.9), sm(1.0, 0.0), false, true);
    hills(f, -75.0);
    let ring: [(f32, f32, f32, bool); 19] = [
        (-6.5, -6.0, 1.5, true), (-9.0, -2.0, 1.3, false), (-7.5, -11.0, 1.7, true),
        (-12.0, -7.0, 1.5, true), (-13.0, -14.0, 1.8, false), (6.8, -6.5, 1.6, true),
        (9.5, -2.5, 1.35, false), (8.0, -12.0, 1.75, true), (12.5, -8.0, 1.5, true),
        (13.5, -15.0, 1.9, false), (-4.5, -13.0, 1.6, true), (4.6, -14.0, 1.55, true),
        (0.0, -17.0, 1.9, true), (-16.0, -4.0, 1.6, true), (16.5, -5.0, 1.7, true),
        (-5.5, 4.5, 1.3, true), (6.2, 5.0, 1.25, false), (-11.0, 3.0, 1.5, true),
        (11.5, 3.5, 1.45, true),
    ];
    for (i, &(x, z, s, round)) in ring.iter().enumerate() {
        let col = if i % 3 != 0 { "#3E7A34" } else { "#356B2C" };
        add_tree(&mut f.std, [x, 0.0, z], i as f32 * 1.7, s, col, round, false);
    }
    add_palm(&mut f.std, [4.6, 0.0, -4.6], 1.05, 0.14, 0.0);
    add_palm(&mut f.std, [-5.2, 0.0, -5.4], 0.9, -0.2, 1.2);
    add_palm(&mut f.std, [10.5, 0.0, -11.0], 1.15, 0.1, 0.0);
    // grass tufts (900 merged cones, hash placement)
    {
        let mut merged = Geo::new();
        let cone = cone_geo(0.05, 0.34, 4, hex_lin("#4C8A3E"));
        for i in 0..900 {
            let fi = i as f64;
            let a = rnd(fi * 1.31) * 6.283185307;
            let r = 2.6 + rnd(fi * 2.7) * 20.0;
            let (x, z) = ((a.cos() * r) as f32, (a.sin() * r) as f32 - 3.0);
            let mut inst = Geo { verts: cone.verts.clone(), idx: cone.idx.clone() };
            for v in inst.verts.chunks_exact_mut(9) {
                v[0] += x;
                v[1] += 0.15;
                v[2] += z;
            }
            merged.append(inst);
        }
        add(&mut f.std, merged, math::identity(), def(), false, false);
    }
    // camp
    let (fx, fz) = (-1.35f32, -2.45f32);
    let stone = hex_lin("#8B8B84");
    for i in 0..8 {
        let a = (i as f32 / 8.0) * std::f32::consts::TAU;
        let r = (0.09 + rnd(i as f64) * 0.04) as f32;
        add(&mut f.std, sphere_geo(r, 8, 6, stone), t3(fx + a.cos() * 0.44, 0.05, fz + a.sin() * 0.44), sm(0.95, 0.0), true, false);
    }
    let log_col = hex_lin("#7A5A38");
    for (i, &(dx, dz)) in [(0.25f32, 0.3f32), (-0.2, -0.28), (0.05, 0.02)].iter().enumerate() {
        let model = math::mul(&t3(fx + dx, 0.09, fz + dz), &math::euler_xyz(0.0, i as f32 * 0.7, std::f32::consts::FRAC_PI_2));
        add(&mut f.std, cyl_geo(0.05, 0.05, 0.72, 7, log_col), model, sm(0.95, 0.0), true, true);
    }
    if jd.fire > 0.02 {
        let fl = jd.fire as f32;
        let ff = (1.0 + 0.16 * (13.0 * jd.t).sin() + 0.07 * (29.0 * jd.t + 1.0).sin()) as f32;
        let m1 = math::mul(&t3(fx, 0.34, fz), &math::scale(fl, ff * fl, fl));
        add(&mut f.std, cone_geo(0.17, 0.52, 8, hex_lin("#3A1C08")), m1, sme(1.0, 0.0, hex_lin("#FF7A1A").map(|c| c * 2.4)), false, false);
        let m2 = math::mul(&t3(fx + 0.03, 0.30, fz + 0.04), &math::scale(fl, (2.0 - ff) * fl, fl));
        add(&mut f.std, cone_geo(0.09, 0.34, 8, hex_lin("#3A2A08")), m2, sme(1.0, 0.0, hex_lin("#FFC93C").map(|c| c * 2.6)), false, false);
    }
    let seat_model = math::mul(&t3(-0.5, 0.17, -1.8), &math::euler_xyz(0.0, 0.12, std::f32::consts::FRAC_PI_2));
    add(&mut f.std, cyl_geo(0.17, 0.17, 1.15, 9, log_col), seat_model, sm(0.95, 0.0), true, true);
    for x in [-3.9f32, -2.7] {
        add(&mut f.std, cyl_geo(0.035, 0.035, 1.5, 6, log_col), t3(x, 0.75, -2.7), sm(0.95, 0.0), true, true);
    }
    let roof_model = math::mul(&t3(-3.3, 1.52, -2.95), &math::rotate_x(0.42));
    add(&mut f.std, box_geo(1.9, 0.05, 1.5, hex_lin("#8A6B42")), roof_model, sm(0.95, 0.0), true, true);
    add(&mut f.std, box_geo(0.55, 0.55, 0.55, hex_lin("#B07A48")), t3(-2.35, 0.275, -3.15), def(), true, true);
    let crate2_model = math::mul(&t3(-2.3, 0.55 + 0.21, -3.12), &math::rotate_y(0.4));
    add(&mut f.std, box_geo(0.42, 0.42, 0.42, hex_lin("#B07A48")), crate2_model, def(), true, true);
    add(&mut f.std, cyl_geo(0.33, 0.33, 0.8, 16, hex_lin("#6E4525")), t3(-3.05, 0.4, -2.2), def(), true, true);
    let band_col = hex_lin("#3A3A3F");
    for y in [0.12f32, 0.68] {
        add(&mut f.std, cyl_geo(0.345, 0.345, 0.05, 16, band_col), t3(-3.05, y, -2.2), sm(0.85, 0.4), true, true);
    }
    let bed_model = math::mul(&t3(-3.3, 0.05, -2.1), &math::rotate_y(0.2));
    add(&mut f.std, box_geo(1.5, 0.09, 0.55, hex_lin("#77805E")), bed_model, sm(1.0, 0.0), true, true);
    f.std.extend(jd.extra_std);
}

// ---------------------------------------------------------------------
// SH — the shore set (shots.ts makeShore).
// ---------------------------------------------------------------------

pub(crate) fn build_sh(f: &mut FrameSpec) {
    add_sky(f, TEX_SKY_SH);
    add(&mut f.std, crate::scene::disc_geo(30.0, 48, hex_lin("#DEC795")), t3(0.0, 0.0, 0.0), sm(1.0, 0.0), false, true);
    // sea + foam: flat boxes standing in for the rotated planes
    add(&mut f.std, box_geo(190.0, 0.02, 70.0, hex_lin("#4E86AC")), t3(0.0, -0.06, -58.0), sm(0.35, 0.08), false, false);
    add(&mut f.std, box_geo(190.0, 0.02, 1.6, hex_lin("#EDF2EA")), t3(0.0, -0.02, -29.2), def(), false, false);
    hills(f, 55.0);
    let trees: [(f32, f32, f32); 7] = [
        (-7.0, 7.0, 1.5), (7.5, 8.0, 1.4), (-11.0, 10.0, 1.7), (12.0, 11.0, 1.6),
        (0.0, 13.0, 1.8), (-4.0, 11.0, 1.3), (4.5, 12.0, 1.35),
    ];
    for (i, &(x, z, s)) in trees.iter().enumerate() {
        add_tree(&mut f.std, [x, 0.0, z], i as f32 * 1.7, s, "#3E7A34", i % 2 == 1, false);
    }
    add_palm(&mut f.std, [-6.2, 0.0, 4.5], 1.0, 0.18, 0.0);
    add_palm(&mut f.std, [6.8, 0.0, 5.5], 0.92, -0.14, 0.0);
    // boat at (-4.6, 0, -7.5) rotY 1.15
    let boat = math::mul(&t3(-4.6, 0.0, -7.5), &math::rotate_y(1.15));
    let hull = hex_lin("#6E5136");
    add(&mut f.std, box_geo(1.5, 0.5, 4.4, hull), math::mul(&boat, &t3(0.0, 0.32, 0.0)), def(), true, true);
    let bow = math::mul(&math::mul(&boat, &t3(0.0, 0.36, 2.35)), &math::rotate_x(-0.5));
    add(&mut f.std, box_geo(1.1, 0.42, 0.5, hull), bow, def(), true, true);
    for z in [-0.8f32, 0.9] {
        add(&mut f.std, box_geo(1.3, 0.08, 0.5, hull), math::mul(&boat, &t3(0.0, 0.5, z)), def(), true, true);
    }
    add(&mut f.std, box_geo(0.6, 0.6, 0.6, hex_lin("#B07A48")), t3(2.7, 0.3, -1.9), def(), true, true);
    let c2 = math::mul(&t3(3.3, 0.225, -2.5), &math::rotate_y(0.5));
    add(&mut f.std, box_geo(0.45, 0.45, 0.45, hex_lin("#B07A48")), c2, def(), true, true);
    add(&mut f.std, cyl_geo(0.33, 0.33, 0.8, 16, hex_lin("#6E4525")), t3(2.4, 0.4, -2.95), def(), true, true);
    let band_col = hex_lin("#3A3A3F");
    for y in [0.12f32, 0.68] {
        add(&mut f.std, cyl_geo(0.345, 0.345, 0.05, 16, band_col), t3(2.4, y, -2.95), sm(0.85, 0.4), true, true);
    }
}

// ---------------------------------------------------------------------
// BK — the bookshop interior (world.ts interior() with the film's
// options + shots.ts shelves / desk / parcels).
// ---------------------------------------------------------------------

fn shelf_unit(f: &mut FrameSpec, w: f32, h: f32, seed: f64, model: Mat4) {
    let frame = hex_lin("#5A4630");
    let put = |f: &mut FrameSpec, geo: Geo, local: Mat4| {
        add(&mut f.std, geo, math::mul(&model, &local), sm(0.85, 0.0), true, true);
    };
    put(f, box_geo(0.06, h, 0.34, frame), t3(-(w / 2.0 - 0.03), h / 2.0, 0.0));
    put(f, box_geo(0.06, h, 0.34, frame), t3(w / 2.0 - 0.03, h / 2.0, 0.0));
    put(f, box_geo(w, 0.06, 0.34, frame), t3(0.0, h - 0.03, 0.0));
    put(f, box_geo(w, 0.06, 0.34, frame), t3(0.0, 0.03, 0.0));
    let cols = ["#7A4A3A", "#4A5A6A", "#6A6248", "#54455E", "#77603A", "#3F5A50", "#83584A", "#5E5142"];
    for r in 0..4u32 {
        let by = 0.12 + r as f32 * 0.58;
        put(f, box_geo(w - 0.12, 0.04, 0.3, frame), t3(0.0, by, 0.0));
        let mut x = -w / 2.0 + 0.12;
        let mut i = 0u32;
        while x < w / 2.0 - 0.14 {
            let bw = 0.055 + rnd(seed * 31.0 + r as f64 * 17.0 + i as f64) as f32 * 0.05;
            if rnd(seed * 91.0 + r as f64 * 13.0 + i as f64) <= 0.86 {
                let bh = 0.34 + rnd(seed * 47.0 + r as f64 * 7.0 + i as f64 * 3.0) as f32 * 0.13;
                let col = cols[((i + r * 3 + seed as u32) % 8) as usize];
                let tilt = rnd(i as f64 * 7.0 + r as f64 + seed) > 0.93;
                let mut local = t3(x + bw / 2.0, by + 0.02 + bh / 2.0, 0.02);
                if tilt {
                    local = math::mul(&local, &math::rotate_z(0.1));
                }
                add(&mut f.std, box_geo(bw, bh, 0.2, hex_lin(col)), math::mul(&model, &local), sm(0.85, 0.0), true, true);
            }
            x += bw + 0.005;
            i += 1;
        }
    }
}

pub(crate) fn build_bk(f: &mut FrameSpec) {
    let white = [1.0f32, 1.0, 1.0];
    // floor: planks plane (texture rotation pi/2 emulated by the
    // swapped repeat), thin box standing in for the plane
    add_mapped(f, box_map_geo(8.0, 0.02, 9.0, white, [3.6, 3.2]), t3(0.0, -0.01, -2.5), TEX_PLANKS_FLOOR, sm(0.85, 0.0), false, true);
    // back wall (z = -7): side boxes + below/above window
    add_mapped(f, box_map_geo(2.4, 4.2, 0.2, white, [2.4 / 1.4, 3.0]), t3(-2.8, 2.1, -7.0), TEX_WALLPAPER, def(), true, true);
    add_mapped(f, box_map_geo(2.4, 4.2, 0.2, white, [2.4 / 1.4, 3.0]), t3(2.8, 2.1, -7.0), TEX_WALLPAPER, def(), true, true);
    add_mapped(f, box_map_geo(3.2, 1.1, 0.2, white, [1.6, 0.5]), t3(0.0, 0.55, -7.0), TEX_PLANKS_DARK, def(), true, true);
    add_mapped(f, box_map_geo(3.2, 1.2, 0.2, white, [3.2 / 1.4, 3.0]), t3(0.0, 3.6, -7.0), TEX_WALLPAPER, def(), true, true);
    // back wainscots
    add_mapped(f, box_map_geo(2.4, 1.0, 0.24, white, [1.0, 0.5]), t3(-2.8, 0.5, -6.98), TEX_PLANKS_DARK, def(), true, true);
    add_mapped(f, box_map_geo(2.4, 1.0, 0.24, white, [1.0, 0.5]), t3(2.8, 0.5, -6.98), TEX_PLANKS_DARK, def(), true, true);
    // side walls + wainscots
    for s in [-1.0f32, 1.0] {
        add_mapped(f, box_map_geo(0.2, 4.2, 9.0, white, [9.0 / 1.4, 3.0]), t3(s * 4.0, 2.1, -2.5), TEX_WALLPAPER, def(), true, true);
        add_mapped(f, box_map_geo(0.24, 1.0, 9.0, white, [4.5, 0.5]), t3(s * 3.98, 0.5, -2.5), TEX_PLANKS_DARK, def(), true, true);
    }
    // window frame #2F5C52
    let fm2 = hex_lin("#2F5C52");
    add(&mut f.std, box_geo(3.4, 0.14, 0.3, fm2), t3(0.0, 1.1, -6.95), def(), true, true);
    add(&mut f.std, box_geo(3.4, 0.14, 0.3, fm2), t3(0.0, 3.0, -6.95), def(), true, true);
    add(&mut f.std, box_geo(0.12, 1.9, 0.3, fm2), t3(-1.6, 2.05, -6.95), def(), true, true);
    add(&mut f.std, box_geo(0.12, 1.9, 0.3, fm2), t3(1.6, 2.05, -6.95), def(), true, true);
    add(&mut f.std, box_geo(0.06, 1.9, 0.1, fm2), t3(0.0, 2.05, -6.95), def(), true, true);
    // view plane (MeshBasic #BFE4F7)
    f.basics.push(BasicDraw {
        geo: quad_geo(8.32, 4.94, -2.47),
        model: t3(0.0, 2.05, -9.5),
        tex: TexSrc::S(TEX_VIEWPLANE),
        tint: [1.0, 1.0, 1.0],
        opacity: 1.0,
        alpha_test: 0.0,
        depth_write: true,
        fog: false,
        prepass: true,
        repeat: false,
    });
    // ceiling + cornice
    add_mapped(f, box_map_geo(8.0, 0.2, 9.0, white, [8.0 / 1.2, 9.0 / 1.2]), t3(0.0, 4.3, -2.5), TEX_WALLPAPER_CEIL, def(), true, true);
    add(&mut f.std, box_geo(8.0, 0.16, 0.3, hex_lin("#E8DCC0")), t3(0.0, 4.12, -6.9), def(), true, true);
    // counter (group at (2.7, 0, -2.9))
    add_mapped(f, box_map_geo(1.0, 1.1, 5.5, white, [1.0, 2.0]), t3(2.7, 0.55, -2.9), TEX_PLANKS_COUNTER, def(), true, true);
    add_mapped(f, box_map_geo(1.2, 0.08, 5.7, white, [1.0, 1.0]), t3(2.7, 1.14, -2.9), TEX_FELT_COUNTER, def(), true, true);
    add(&mut f.std, box_geo(0.1, 0.06, 5.7, hex_lin("#C9A13B")), t3(2.08, 1.12, -2.9), sm(0.3, 0.6), true, true);
    // picture: frame box + picture plane on its room-facing side
    let pic_model = math::mul(&t3(-3.87, 2.4, -3.5), &math::rotate_y(std::f32::consts::FRAC_PI_2));
    add(&mut f.std, box_geo(0.9, 0.7, 0.05, hex_lin("#C9A13B")), pic_model, def(), true, true);
    f.texmapped.push(TexMapDraw {
        geo: quad_geo(0.9, 0.7, -0.35),
        model: math::mul(&t3(-3.87 + 0.028, 2.4, -3.5), &math::rotate_y(std::f32::consts::FRAC_PI_2)),
        rough: 0.85,
        receive: false,
        tex: TEX_PICTURE,
        repeat: false,
        nrm: [0.0, 0.0, 1.0],
    });
    // sign: board + face plane (text enters as the sign texture)
    let sign_model = math::mul(&t3(3.87, 2.9, -2.9), &math::rotate_y(-std::f32::consts::FRAC_PI_2));
    add(&mut f.std, box_geo(2.2, 0.4, 0.05, shade_lin("#3A2A2A", 0.7)), sign_model, def(), true, true);
    f.texmapped.push(TexMapDraw {
        geo: quad_geo(2.2, 0.4, -0.2),
        model: math::mul(&t3(3.87 - 0.028, 2.9, -2.9), &math::rotate_y(-std::f32::consts::FRAC_PI_2)),
        rough: 0.85,
        receive: false,
        tex: TEX_SIGN,
        repeat: false,
        nrm: [0.0, 0.0, 1.0],
    });
    // bulbs: cord (basic-material brass, as emissive-only) + bulb
    for (x, z) in [(-1.5f32, -2.2f32), (1.6, -4.5)] {
        add(&mut f.std, cyl_geo(0.008, 0.008, 1.0, 4, [0.0, 0.0, 0.0]), t3(x, 3.7, z), sme(1.0, 0.0, hex_lin("#C9A13B")), false, false);
        add(&mut f.std, sphere_geo(0.1, 12, 8, hex_lin("#FFF6DA")), t3(x, 3.15, z), sme(1.0, 0.0, hex_lin("#FFE7A0").map(|c| c * 2.2)), false, false);
    }
    // shelves
    shelf_unit(f, 3.4, 2.5, 1.0, t3(-1.8, 0.0, -6.55));
    shelf_unit(f, 2.4, 2.5, 2.0, t3(1.9, 0.0, -6.55));
    shelf_unit(f, 3.2, 2.5, 3.0, math::mul(&t3(-3.82, 0.0, -3.6), &math::rotate_y(std::f32::consts::FRAC_PI_2)));
    // desk (group at (-1.5, 0, -3.4) rotY 0.15)
    let desk = math::mul(&t3(-1.5, 0.0, -3.4), &math::rotate_y(0.15));
    let dm = |local: Mat4| math::mul(&desk, &local);
    add_mapped(f, box_map_geo(1.6, 0.08, 1.0, white, [1.0, 1.0]), dm(t3(0.0, 0.84, 0.0)), TEX_FELT_DARK, def(), true, true);
    add_mapped(f, box_map_geo(1.7, 0.06, 1.1, white, [1.0, 1.0]), dm(t3(0.0, 0.78, 0.0)), TEX_PLANKS_DARK, def(), true, true);
    add_mapped(f, box_map_geo(0.5, 0.76, 0.95, white, [1.0, 1.0]), dm(t3(-0.5, 0.38, 0.0)), TEX_PLANKS_DARK, def(), true, true);
    add_mapped(f, box_map_geo(0.5, 0.76, 0.95, white, [1.0, 1.0]), dm(t3(0.5, 0.38, 0.0)), TEX_PLANKS_DARK, def(), true, true);
    add_mapped(f, box_map_geo(0.6, 0.6, 0.06, white, [1.0, 1.0]), dm(t3(0.0, 0.45, -0.45)), TEX_PLANKS_DARK, def(), true, true);
    // parcels on the desk
    let p1 = math::mul(&t3(-1.4, 0.88, -3.35), &math::rotate_y(0.4));
    add_mapped(f, box_map_geo(0.5, 0.16, 0.36, white, [1.0, 1.0]), p1, TEX_PARCEL, def(), true, true);
    let p2 = math::mul(&t3(-1.35, 1.015, -3.3), &math::rotate_y(0.15));
    add_mapped(f, box_map_geo(0.44, 0.13, 0.32, white, [1.0, 1.0]), p2, TEX_PARCEL, def(), true, true);
}

// ---------------------------------------------------------------------
// Evaluator: shots.ts transcribed. Small vector helpers first.
// ---------------------------------------------------------------------

pub(crate) fn sub3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
pub(crate) fn cross3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}
pub(crate) fn norm3(a: [f32; 3]) -> [f32; 3] {
    let l = (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt();
    if l > 0.0 {
        [a[0] / l, a[1] / l, a[2] / l]
    } else {
        a
    }
}
fn fold(hex: &str, i: f32) -> [f32; 3] {
    hex_lin(hex).map(|c| c * i)
}
fn folded_point(hex: &str, i: f64, pos: [f32; 3], dist: f32, decay: f32) -> PointState {
    PointState { pos, col: fold(hex, i as f32), dist, decay }
}

/// matchEnv tint (toon.ts): the per-atmosphere multiplier a
/// character card's material colour takes.
fn env_tint(sat: f32, warm: f32, exp: f32, fog_hex: &str, fog_mix: f32) -> [f32; 3] {
    let mut c = [1.0 + warm * 0.6, 1.0, 1.0 - warm * 0.6];
    if sat < 1.0 {
        let k = (1.0 - sat) * 0.25;
        for v in c.iter_mut() {
            *v = *v + (0.92 - *v) * k;
        }
    }
    let e = exp.max(0.0).sqrt().clamp(0.0, 1.15);
    for v in c.iter_mut() {
        *v *= e;
    }
    if fog_mix > 0.0 {
        let f = hex_lin(fog_hex);
        for i in 0..3 {
            c[i] = c[i] + (f[i] - c[i]) * fog_mix;
        }
    }
    c
}

fn g_jungle() -> PostState {
    PostState { bloom: 0.16, aperture: 0.0012, maxblur: 0.007, sat: 0.8, warm: 0.04, vig: 0.24, grain: 0.012, exposure: 1.0 }
}
fn g_shore() -> PostState {
    PostState { sat: 0.84, warm: 0.05, vig: 0.22, ..g_jungle() }
}
fn g_shop() -> PostState {
    PostState { bloom: 0.3, aperture: 0.001, maxblur: 0.008, sat: 0.78, warm: 0.06, vig: 0.26, grain: 0.012, exposure: 1.0 }
}

/// The JG/SH sunlight shadow rig under an atmosphere. `key_pos`
/// and `key_tgt` are the set's; the atmosphere overrides colour /
/// intensity / position per env.ts.
fn rig_noon(key_pos: [f32; 3], key_tgt: [f32; 3], fire: PointState) -> LightState {
    LightState {
        key_pos,
        key_tgt,
        key_col: fold("#FFF0D6", 2.6),
        key_box: 17.0,
        key_near: 0.1,
        key_far: 80.0,
        key_bias: -0.0006,
        rim_col: fold("#FFFFFF", 0.4),
        rim_dir: norm3([-3.0, 3.0, -3.0]),
        hemi_sky: fold("#CFE6FF", 1.25),
        hemi_ground: fold("#C48A5A", 1.25),
        points: [fire, PointState::off(), PointState::off()],
        env: 0.25,
    }
}
fn rig_golden(fire: PointState) -> LightState {
    LightState {
        key_pos: [-8.0, 4.5, 5.0],
        key_tgt: [0.0, 0.0, -2.0],
        key_col: fold("#FFB35C", 2.3),
        rim_col: fold("#FFFFFF", 0.6),
        hemi_sky: fold("#FFE0B0", 1.0),
        hemi_ground: fold("#B07A48", 1.0),
        ..rig_noon([7.0, 12.0, 5.0], [0.0, 0.0, -2.0], fire)
    }
}
fn rig_dusk(fire: PointState) -> LightState {
    LightState {
        key_pos: [-8.0, 5.0, -14.0],
        key_tgt: [0.0, 0.0, -2.0],
        key_col: fold("#FFB27A", 2.0),
        rim_col: fold("#FFFFFF", 0.5),
        hemi_sky: fold("#B9A2D8", 0.9),
        hemi_ground: fold("#7A4A3A", 0.9),
        ..rig_noon([7.0, 12.0, 5.0], [0.0, 0.0, -2.0], fire)
    }
}
fn rig_fog(fire: PointState) -> LightState {
    LightState {
        key_col: fold("#EAF0F2", 0.7),
        rim_col: fold("#FFFFFF", 0.15),
        hemi_sky: fold("#E6E9EA", 0.9),
        hemi_ground: fold("#B8BCBE", 0.9),
        ..rig_noon([7.0, 12.0, 5.0], [0.0, 0.0, -2.0], fire)
    }
}
fn fire_light(level: f64, t: f64) -> PointState {
    let i = if level > 0.02 {
        (4.6 + 0.7 * (17.0 * t).sin() + 0.5 * (7.3 * t).sin()) * level
    } else {
        0.0
    };
    folded_point("#FF9A3C", i, [-1.35, 0.6, -2.45], 7.0, 1.6)
}
fn rig_bk() -> LightState {
    LightState {
        key_pos: [-2.0, 6.0, -13.0],
        key_tgt: [0.5, 0.0, -1.0],
        key_col: fold("#FFE4B8", 2.4),
        key_box: 8.0,
        key_near: 0.5,
        key_far: 30.0,
        key_bias: -0.0006,
        rim_col: [0.0; 3],
        rim_dir: norm3([-3.0, 3.0, -3.0]),
        hemi_sky: fold("#FFE9D2", 1.1),
        hemi_ground: fold("#5A3A2A", 1.1),
        points: [
            folded_point("#FFD9A8", 6.0, [0.0, 3.2, -1.0], 12.0, 1.4),
            folded_point("#FFD9A0", 3.0, [-1.5, 3.05, -2.2], 6.0, 1.6),
            folded_point("#FFD9A0", 3.0, [1.6, 3.05, -4.5], 6.0, 1.6),
        ],
        env: 0.35,
    }
}

const TINT_NOON: [f32; 3] = [1.0, 1.0, 1.0];
fn tint_golden() -> [f32; 3] {
    env_tint(1.02, 0.08, 1.0, "#FFD98A", 0.06)
}
fn tint_dusk() -> [f32; 3] {
    env_tint(0.97, 0.06, 0.96, "#F59C6B", 0.08)
}
fn tint_fog() -> [f32; 3] {
    env_tint(0.82, 0.0, 0.94, "#E6E9EA", 0.35)
}

// Instance indices into look_registry().
const ONODA: usize = 0;
const COMPA: usize = 1;
const COMPB: usize = 2;
const SUZUKI: usize = 3;
const TANI: usize = 4;
const SH_ONODA: usize = 5;
const SH_COMPA: usize = 6;
const SH_COMPB: usize = 7;
const BK_TANI: usize = 9;
const ONODA2: usize = 10;

fn fs() -> FullState {
    FullState::default()
}
fn pose_explain(st: &mut FullState) {
    st.mood = Mood::Happy;
    st.arm_r = (68.0, -42.0);
    st.arm_l = (28.0, -24.0);
    st.look = [0.3, 0.0];
}
fn pose_celebrate(st: &mut FullState) {
    st.mood = Mood::Laugh;
    st.arm_l = (150.0, 14.0);
    st.arm_r = (150.0, 14.0);
    st.bob = -8.0;
}
fn pose_mourn(st: &mut FullState) {
    st.mood = Mood::Sad;
    st.arm_l = (4.0, 4.0);
    st.arm_r = (4.0, 4.0);
    st.lean = 5.0;
    st.head_tilt = -4.0;
    st.bob = 4.0;
}
fn pose_listen(st: &mut FullState) {
    st.arm_r = (118.0, -105.0);
    st.arm_l = (10.0, 8.0);
    st.lean = 3.0;
    st.head_tilt = -5.0;
    st.head_turn = -0.2;
}
fn pose_think(st: &mut FullState) {
    st.arm_r = (48.0, -118.0);
    st.arm_l = (20.0, -30.0);
    st.head_tilt = 6.0;
    st.head_turn = 0.3;
    st.look = [0.4, -0.3];
}

/// shoot(): camera from/to mixed by k, target at/at2 likewise;
/// focus = distance from the mixed position to the mixed target.
fn shoot(
    f: &mut FrameSpec,
    from: [f64; 3],
    to: [f64; 3],
    at: [f64; 3],
    at2: Option<[f64; 3]>,
    k: f64,
    fov: f32,
) {
    let eye = mix3(from, to, k);
    let tgt = mix3(at, at2.unwrap_or(at), k);
    f.eye = eye;
    f.target = tgt;
    f.fov = fov;
    f.focus = ((eye[0] - tgt[0]).powi(2) + (eye[1] - tgt[1]).powi(2) + (eye[2] - tgt[2]).powi(2)).sqrt();
}

/// face() for every character in the frame + the drawing-side
/// camPitch where the staging reads it from the camera: yaw always,
/// model pitch only for perspective instances.
fn finish_chars(f: &mut FrameSpec, looks: &[(FullLook, bool, f64)]) {
    let eye = f.eye;
    for c in f.chars.iter_mut() {
        let (_, perspective, _) = &looks[c.look];
        let yaw = (eye[0] - c.pos[0]).atan2(eye[2] - c.pos[2]);
        c.yaw = yaw;
        if *perspective {
            let h = 1.75f32;
            let dy = eye[1] - (c.pos[1] + h * 0.6);
            let dh = (eye[0] - c.pos[0]).hypot(eye[2] - c.pos[2]);
            let pitch_ang = if dh < 1e-4 { 0.0 } else { dy.atan2(dh) };
            c.pitch = (pitch_ang * 0.45).clamp(-0.38, 0.38);
            c.state.cam_pitch = cam_pitch_of(c.pos, eye);
        } else {
            c.pitch = 0.0;
        }
    }
}

/// camPitchOf (toon.ts): clamp(pitchAng / 0.9, -1, 1) for a toon
/// whose group sits at `pos`, camera at `eye` (H = 1.75 world).
fn cam_pitch_of(pos: [f32; 3], eye: [f32; 3]) -> f32 {
    let dy = eye[1] - (pos[1] + 1.75 * 0.6);
    let dh = (eye[0] - pos[0]).hypot(eye[2] - pos[2]);
    let pitch_ang = if dh < 1e-4 { 0.0 } else { dy.atan2(dh) };
    (pitch_ang / 0.9).clamp(-1.0, 1.0)
}

const SH_TANI: usize = 8;

fn char(f: &mut FrameSpec, inst: usize, pos: [f32; 3], st: FullState, tint: [f32; 3], rifle: bool) {
    f.chars.push(CharDraw { look: inst, pos, state: st, yaw: 0.0, pitch: 0.0, tint, rifle, hold_l: None, hold_r: None });
}

// ---------------------------------------------------------------------
// The twelve shots (fourteen segments). lt = shot-local time,
// d = segment duration, t = film time, st = segment start.
// ---------------------------------------------------------------------

fn blink_of(seed: f64, t: f64, beats: &[f64]) -> f32 {
    blink_max(Blinker { seed }.at(t), blink_at(t, beats)) as f32
}

fn s01(data: &FilmData, t: f64, lt: f64, d: f64) -> FrameSpec {
    let mut f = FrameSpec::new(rig_noon([7.0, 12.0, 5.0], [0.0, 0.0, -2.0], fire_light(1.0, t)), g_jungle());
    build_jg(&mut f, JgDyn { fire: 1.0, t, extra_std: Vec::new() });
    let k = cam_k(lt, d, 0.1);
    shoot(&mut f, [0.85, 2.7, 5.4], [0.55, 1.62, 3.2], [0.35, 1.12, -0.55], Some([0.35, 1.02, -0.55]), k, 44.0);
    let mut st = fs();
    st.mood = Mood::Determined;
    st.arm_r = (16.0, -6.0);
    st.arm_l = (10.0, 6.0);
    st.lean = 1.0;
    st.head_turn = pose_track(&[key(0.0, -0.45), keym(2.6, 0.4, 2.2), keym(5.2, -0.2, 2.2)], lt) as f32;
    st.cam_pitch = cam_pitch_of([0.35, 0.0, -0.55], f.eye);
    st.blink = blink_of(3.0, t, &[data.anchors["years0"]]);
    char(&mut f, ONODA, [0.35, 0.0, -0.55], st, TINT_NOON, true);
    { let (e, tg) = (f.eye, f.target); add_clouds(&mut f, &[([-28.0, 23.0, -85.0], 13.0), ([26.0, 29.0, -95.0], 16.0), ([58.0, 19.0, -75.0], 10.0)], e, tg); }
    f
}

fn s02(data: &FilmData, t: f64, lt: f64, d: f64) -> FrameSpec {
    let _ = data;
    let mut f = FrameSpec::new(rig_noon([-8.0, 11.0, 7.0], [0.0, 0.0, -4.0], PointState::off()), g_shore());
    f.post.aperture = 0.0007;
    build_sh(&mut f);
    let k = cam_k(lt, d, 0.03);
    let kk = if k > 0.97 { 0.97 } else { k };
    let travelled = kk * 6.8;
    let ph = walk_phase(travelled, 0.62);
    let bob = walk_bob(ph, 5.0);
    let legs = if k > 0.97 { Legs::Stand } else { Legs::Walk };
    // MO.crane: pos/look lerped on k, shot with the frame directly.
    f.eye = mix3([2.0, 3.1, 2.4], [0.9, 1.85, 0.6], k);
    f.target = mix3([-0.1, 1.25, -5.2], [0.0, 1.15, -4.2], k);
    f.fov = 46.0;
    f.focus = ((f.eye[0] - f.target[0]).powi(2) + (f.eye[1] - f.target[1]).powi(2) + (f.eye[2] - f.target[2]).powi(2)).sqrt();
    let z = -11.0 + travelled;
    let mut so = fs();
    so.mood = Mood::Determined;
    so.arm_r = (16.0, -6.0);
    so.arm_l = (10.0, 6.0);
    so.legs = legs;
    so.phase = ph as f32;
    so.bob = bob as f32;
    so.blink = Blinker { seed: 3.0 }.at(t) as f32;
    char(&mut f, SH_ONODA, [0.0, 0.0, z as f32], so, TINT_NOON, true);
    let mut sa = fs();
    sa.legs = legs;
    sa.phase = ph as f32;
    sa.bob = bob as f32;
    sa.blink = Blinker { seed: 7.0 }.at(t) as f32;
    char(&mut f, SH_COMPA, [-1.4, 0.0, (z - 0.9) as f32], sa, TINT_NOON, false);
    let mut sb = fs();
    sb.legs = legs;
    sb.phase = ph as f32;
    sb.bob = bob as f32;
    sb.blink = Blinker { seed: 11.0 }.at(t) as f32;
    char(&mut f, SH_COMPB, [1.4, 0.0, (z - 1.5) as f32], sb, TINT_NOON, false);
    { let (e, tg) = (f.eye, f.target); add_clouds(&mut f, &[([-24.0, 22.0, -85.0], 12.0), ([30.0, 27.0, -90.0], 15.0)], e, tg); }
    f
}

fn s03(data: &FilmData, t: f64, lt: f64, d: f64) -> FrameSpec {
    let st0 = 13.061;
    let w_never = data.anchors["never2"];
    let w_surr = data.anchors["surrender2"];
    let mut f = FrameSpec::new(rig_noon([-8.0, 11.0, 7.0], [0.0, 0.0, -4.0], PointState::off()), g_shore());
    build_sh(&mut f);
    let k = cam_k(lt, d, 0.1);
    shoot(&mut f, [0.2, 2.0, 8.4], [0.05, 1.85, 7.2], [0.0, 1.22, 0.6], None, k, 54.0);
    let mut tani = fs();
    pose_explain(&mut tani);
    tani.mood = Mood::Determined;
    tani.arm_r = (pose_track(&[key(0.0, 68.0), keym(w_never - st0 - 0.13, 96.0, 0.3)], lt) as f32, -6.0);
    tani.talk = talk(t, lt < w_surr - st0 + 0.35) as f32;
    tani.blink = Blinker { seed: 9.0 }.at(t) as f32;
    char(&mut f, SH_TANI, [-1.05, 0.0, 1.0], tani, TINT_NOON, false);
    let mut onoda = fs();
    onoda.mood = Mood::Determined;
    onoda.arm_r = (5.0, 3.0);
    onoda.arm_l = (5.0, 3.0);
    onoda.blink = blink_of(3.0, t, &[w_never]);
    char(&mut f, SH_ONODA, [1.0, 0.0, 0.75], onoda, TINT_NOON, true);
    let mut ca = fs();
    ca.arm_r = (5.0, 3.0);
    ca.arm_l = (5.0, 3.0);
    ca.blink = Blinker { seed: 7.0 }.at(t) as f32;
    char(&mut f, SH_COMPA, [1.85, 0.0, -0.85], ca, TINT_NOON, false);
    let mut cb = fs();
    cb.arm_r = (5.0, 3.0);
    cb.arm_l = (5.0, 3.0);
    cb.blink = Blinker { seed: 11.0 }.at(t) as f32;
    char(&mut f, SH_COMPB, [-1.9, 0.0, -0.65], cb, TINT_NOON, false);
    { let (e, tg) = (f.eye, f.target); add_clouds(&mut f, &[([-24.0, 22.0, -85.0], 12.0), ([30.0, 27.0, -90.0], 15.0)], e, tg); }
    f
}

fn s04(data: &FilmData, t: f64, lt: f64, d: f64) -> FrameSpec {
    let mut f = FrameSpec::new(rig_noon([7.0, 12.0, 5.0], [0.0, 0.0, -2.0], fire_light(1.0, t)), g_jungle());
    build_jg(&mut f, JgDyn { fire: 1.0, t, extra_std: Vec::new() });
    let k = cam_k(lt, d, 0.1);
    shoot(&mut f, [1.0, 1.8, 7.6], [0.85, 1.65, 6.6], [0.8, 1.1, -0.6], None, k, 50.0);
    let mut ca = fs();
    pose_celebrate(&mut ca);
    ca.bob = (-4.5 * (4.6 * t).sin().abs()) as f32;
    ca.blink = Blinker { seed: 7.0 }.at(t) as f32;
    char(&mut f, COMPA, [-0.75, 0.0, -2.65], ca, TINT_NOON, false);
    let mut cb = fs();
    pose_celebrate(&mut cb);
    cb.mood = Mood::Happy;
    cb.bob = (-4.5 * (4.6 * t + 1.2).sin().abs()) as f32;
    cb.blink = Blinker { seed: 11.0 }.at(t) as f32;
    char(&mut f, COMPB, [0.85, 0.0, -2.95], cb, TINT_NOON, false);
    let mut onoda = fs();
    onoda.mood = Mood::Suspicious;
    onoda.arm_r = (16.0, -6.0);
    onoda.arm_l = (10.0, 6.0);
    onoda.head_turn = pose_track(&[key(0.0, 0.5), keym(2.2, -0.35, 1.9), keym(4.6, 0.3, 1.9)], lt) as f32;
    onoda.blink = blink_of(3.0, t, &[data.anchors["message3"]]);
    char(&mut f, ONODA, [1.8, 0.0, 1.55], onoda, TINT_NOON, true);
    { let (e, tg) = (f.eye, f.target); add_clouds(&mut f, &[([-28.0, 23.0, -85.0], 13.0), ([26.0, 29.0, -95.0], 16.0), ([58.0, 19.0, -75.0], 10.0)], e, tg); }
    f
}

fn plane_extras(t: f64, lt: f64, d: f64) -> Vec<StdDraw> {
    let mut out = Vec::new();
    let mat = || sm(0.6, 0.25);
    let col = hex_lin("#67705F");
    let px = lerp(-13.0, 11.0, seg(lt, 0.05, d - 0.05));
    let group = math::mul(&t3(px as f32, 9.6, -13.0), &math::rotate_y(std::f32::consts::FRAC_PI_2));
    let put = |out: &mut Vec<StdDraw>, geo: Geo, local: Mat4| {
        add(out, geo, math::mul(&group, &local), mat(), false, false);
    };
    put(&mut out, cyl_geo(0.17, 0.23, 2.7, 10, col), math::rotate_x(std::f32::consts::FRAC_PI_2));
    let nose = math::mul(&t3(0.0, 0.0, 1.55), &math::rotate_x(std::f32::consts::FRAC_PI_2));
    put(&mut out, cone_geo(0.17, 0.5, 10, col), nose);
    put(&mut out, box_geo(3.6, 0.05, 0.72, col), t3(0.0, 0.02, 0.15));
    put(&mut out, box_geo(1.1, 0.05, 0.4, col), t3(0.0, 0.05, -1.25));
    put(&mut out, box_geo(0.05, 0.55, 0.35, col), t3(0.0, 0.28, -1.25));
    let prop = math::mul(&t3(0.0, 0.0, 1.86), &math::rotate_z((35.0 * t) as f32));
    let pcol = hex_lin("#2E2E33");
    add(&mut out, box_geo(0.09, 1.05, 0.03, pcol), math::mul(&group, &prop), mat(), false, false);
    let prop2 = math::mul(&prop, &math::rotate_z(std::f32::consts::FRAC_PI_2));
    add(&mut out, box_geo(0.09, 1.05, 0.03, pcol), math::mul(&group, &prop2), mat(), false, false);
    // 16 leaflets
    let leaf_col = hex_lin("#F2EDE0");
    for i in 0..16 {
        let fi = i as f64;
        let delay = 0.4 + 0.13 * fi;
        let ft = (lt - delay) / 3.1;
        if ft <= 0.0 || ft >= 1.12 {
            continue;
        }
        let sx = lerp(-13.0, 11.0, seg(delay + 0.3, 0.05, d - 0.05));
        let x = sx + (4.0 * ft + 1.7 * fi).sin() * 0.9 + 1.2 * ft;
        let y = 9.4 - ft.powf(1.12) * 8.9;
        let z = -13.0 + (rnd(3.0 * fi) - 0.5) * 3.0 + 3.6 * ft;
        let model = math::mul(
            &t3(x as f32, y as f32, z as f32),
            &math::euler_xyz((1.7 * t + fi) as f32, (1.3 * fi) as f32, (1.1 * t + 2.0 * fi) as f32),
        );
        add(&mut out, box_geo(0.34, 0.45, 0.005, leaf_col), model, sm(0.9, 0.0), false, false);
    }
    out
}

fn s05a(data: &FilmData, t: f64, lt: f64, d: f64) -> FrameSpec {
    let _ = data;
    let mut f = FrameSpec::new(rig_noon([7.0, 12.0, 5.0], [0.0, 0.0, -2.0], PointState::off()), g_jungle());
    build_jg(&mut f, JgDyn { fire: 0.0, t, extra_std: plane_extras(t, lt, d) });
    let k = cam_k(lt, d, 0.1);
    shoot(&mut f, [2.4, 1.0, 4.2], [2.0, 1.2, 3.6], [-4.5, 6.4, -11.0], Some([0.5, 6.0, -11.0]), k, 46.0);
    { let (e, tg) = (f.eye, f.target); add_clouds(&mut f, &[([-28.0, 23.0, -85.0], 13.0), ([26.0, 29.0, -95.0], 16.0), ([58.0, 19.0, -75.0], 10.0)], e, tg); }
    f
}

fn s05b(data: &FilmData, t: f64, lt: f64, d: f64) -> FrameSpec {
    let st0 = 24.560;
    let w_trick = data.anchors["trick4"];
    let drop_t = 26.150f64;
    let mut extra = Vec::new();
    if t >= drop_t {
        let u = seg(t, drop_t, drop_t + 0.85);
        let x = -0.25 + 0.14 * u;
        let y = lerp(1.05, 0.03, u * u);
        let z = -0.3 + 0.1 * u;
        let model = math::mul(&t3(x as f32, y as f32, z as f32), &math::euler_xyz((2.4 * u) as f32, 0.4, (1.8 * u) as f32));
        add(&mut extra, box_geo(0.24, 0.32, 0.005, hex_lin("#F2EDE0")), model, sm(0.9, 0.0), false, false);
    }
    let mut f = FrameSpec::new(rig_noon([7.0, 12.0, 5.0], [0.0, 0.0, -2.0], fire_light(1.0, t)), g_jungle());
    build_jg(&mut f, JgDyn { fire: 1.0, t, extra_std: extra });
    let k = cam_k(lt, d, 0.1);
    shoot(&mut f, [0.95, 1.75, 2.6], [0.7, 1.6, 1.8], [0.3, 1.3, -0.55], None, k, 42.0);
    let mut onoda = fs();
    onoda.mood = Mood::Suspicious;
    onoda.arm_l = (pose_track(&[key(0.0, 58.0), keym(drop_t - st0, 16.0, 0.45)], lt) as f32, -24.0);
    onoda.arm_r = (16.0, -6.0);
    onoda.look = [0.1, 0.3];
    onoda.head_turn = if t > w_trick { (settle(t, w_trick, 6.5, 3.2) * 0.38) as f32 } else { 0.04 };
    onoda.blink = Blinker { seed: 3.0 }.at(t) as f32;
    let idx = {
        char(&mut f, ONODA, [0.35, 0.0, -0.55], onoda, TINT_NOON, true);
        f.chars.len() - 1
    };
    if t < drop_t {
        f.chars[idx].hold_l = Some("leaflet");
    }
    { let (e, tg) = (f.eye, f.target); add_clouds(&mut f, &[([-28.0, 23.0, -85.0], 13.0), ([26.0, 29.0, -95.0], 16.0), ([58.0, 19.0, -75.0], 10.0)], e, tg); }
    f
}

fn s06(data: &FilmData, t: f64, lt: f64, d: f64) -> FrameSpec {
    let _ = data;
    let sit_at = 3.55f64;
    let fire = clamp(1.0 - seg(lt, sit_at - 0.6, sit_at + 0.15), 0.0, 1.0);
    let mut post = g_jungle();
    post.sat = 0.52;
    post.warm = 0.0;
    post.exposure = 0.96;
    let mut f = FrameSpec::new(rig_fog(fire_light(fire, t)), post);
    f.fog = Some((hex_lin("#C9CFC2"), 4.5, 20.0));
    build_jg(&mut f, JgDyn { fire, t, extra_std: Vec::new() });
    let k = cam_k(lt, d, 0.1);
    shoot(&mut f, [1.7, 1.9, 4.4], [1.15, 1.68, 2.9], [-0.3, 1.05, -3.2], None, k, 46.0);
    let wk = seg(lt, 0.1, sit_at);
    let mk_walk = |dist: f64, mood: Mood| -> FullState {
        let mut st = fs();
        let ph = walk_phase(dist, 0.62);
        st.phase = ph as f32;
        st.bob = walk_bob(ph, 5.0) as f32;
        st.legs = if wk >= 1.0 { Legs::Stand } else { Legs::Walk };
        st.mood = mood;
        st.head_tilt = -4.0;
        st
    };
    let mut ca = mk_walk(wk * 5.96, Mood::Sad);
    ca.blink = Blinker { seed: 7.0 }.at(t) as f32;
    char(&mut f, COMPA, [(-1.7 - 2.3 * wk) as f32, 0.0, (-5.0 - 5.5 * wk) as f32], ca, tint_fog(), false);
    let mut cb = mk_walk(wk * 6.1, Mood::Tired);
    cb.blink = Blinker { seed: 11.0 }.at(t) as f32;
    char(&mut f, COMPB, [(0.2 - 2.8 * wk) as f32, 0.0, (-5.6 - 5.3 * wk) as f32], cb, tint_fog(), false);
    let oz = lerp(-1.48, -1.76, seg(lt, sit_at - 0.15, sit_at + 0.25));
    // shots.ts:419: POSES.mourn is spread in BOTH branches (lean 5,
    // headTilt -4, bob 4 survive sitting); the set() then overrides
    // the arms in both branches; sitting only adds legs/sitOn and
    // the settle squash.
    let mut onoda = fs();
    pose_mourn(&mut onoda);
    onoda.arm_r = (16.0, -6.0);
    onoda.arm_l = (8.0, 5.0);
    if lt >= sit_at {
        onoda.legs = Legs::Sit;
        onoda.sit_drop = Some((0.34 * 1024.0 / (1.75 / 0.96)) as f32);
        onoda.squash = (-0.04 * (seg(lt, sit_at, sit_at + 0.3) * std::f64::consts::PI).sin()) as f32;
    }
    onoda.blink = Blinker { seed: 3.0 }.at(t) as f32;
    char(&mut f, ONODA, [-0.5, 0.0, oz as f32], onoda, tint_fog(), true);
    { let (e, tg) = (f.eye, f.target); add_clouds(&mut f, &[([-28.0, 23.0, -85.0], 13.0), ([26.0, 29.0, -95.0], 16.0), ([58.0, 19.0, -75.0], 10.0)], e, tg); }
    f
}

fn calendar_state(lt_cal: f64) -> Option<CalendarState> {
    if !(0.0..=4.4).contains(&lt_cal) {
        return None;
    }
    let years = 4;
    let cyc = 1.04f64;
    let idx = ((lt_cal / cyc).floor() as usize).min(years - 1);
    let local = lt_cal - idx as f64 * cyc;
    let (mut year, mut sy, mut dark) = (idx, 1.0f32, 0.0f32);
    if idx < years - 1 && local > 0.62 {
        let u = clamp((local - 0.62) / 0.42, 0.0, 1.0);
        sy = ((u * std::f64::consts::PI).cos().abs()).max(0.1) as f32;
        dark = 1.0 - sy;
        if u >= 0.5 {
            year = idx + 1;
        }
    }
    let alpha = clamp(lt_cal / 0.18, 0.0, 1.0) * clamp((4.4 - lt_cal) / 0.35, 0.0, 1.0);
    Some(CalendarState { year, sy, dark, alpha: alpha as f32 })
}

fn s07(data: &FilmData, t: f64, lt: f64, d: f64) -> FrameSpec {
    let st0 = 30.390;
    let w_coco = data.anchors["coconuts6"];
    let mut post = g_jungle();
    post.sat = 0.85;
    post.warm = 0.06;
    post.exposure = 1.02;
    let mut f = FrameSpec::new(rig_golden(fire_light(0.0, t)), post);
    f.fog = Some((hex_lin("#FFD98A"), 30.0, 140.0));
    build_jg(&mut f, JgDyn { fire: 0.0, t, extra_std: Vec::new() });
    let k = cam_k(lt, d, 0.1);
    // MO.crane frame fed to shoot with k = 1 (positions already mixed)
    f.eye = mix3([1.4, 1.6, 3.4], [3.6, 5.4, 7.2], k);
    f.target = mix3([1.4, 1.0, -1.8], [1.8, 0.8, -2.4], k);
    f.fov = 46.0;
    f.focus = ((f.eye[0] - f.target[0]).powi(2) + (f.eye[1] - f.target[1]).powi(2) + (f.eye[2] - f.target[2]).powi(2)).sqrt();
    let wk = seg(lt, 0.2, 1.85);
    let ox = lerp(-0.5, 2.2, wk);
    let oz = lerp(-1.76, -2.3, wk);
    let mut onoda = fs();
    let ph = walk_phase(wk * 2.0, 0.6);
    onoda.phase = ph as f32;
    onoda.bob = walk_bob(ph, 4.0) as f32;
    onoda.legs = if wk >= 1.0 { Legs::Stand } else { Legs::Walk };
    onoda.mood = if lt < 1.2 { Mood::Tired } else { Mood::Neutral };
    if lt < 1.2 {
        onoda.head_tilt = -3.0;
    }
    onoda.arm_r = (pose_track(&[key(0.0, 12.0), keym(w_coco - st0 - 0.1, 136.0, 0.3), keym(w_coco - st0 + 0.85, 34.0, 0.5)], lt) as f32, -10.0);
    onoda.arm_l = (12.0, 6.0);
    onoda.look = [0.2, -0.1];
    onoda.cam_pitch = cam_pitch_of([ox as f32, 0.0, oz as f32], f.eye);
    onoda.blink = Blinker { seed: 3.0 }.at(t) as f32;
    let idx = {
        char(&mut f, ONODA, [ox as f32, 0.0, oz as f32], onoda, tint_golden(), true);
        f.chars.len() - 1
    };
    if t >= w_coco - 0.05 {
        f.chars[idx].hold_l = Some("coconut");
    }
    f.calendar = calendar_state(lt - 0.16);
    { let (e, tg) = (f.eye, f.target); add_clouds(&mut f, &[([-28.0, 23.0, -85.0], 13.0), ([26.0, 29.0, -95.0], 16.0), ([58.0, 19.0, -75.0], 10.0)], e, tg); }
    f
}

fn s08(data: &FilmData, t: f64, lt: f64, d: f64) -> FrameSpec {
    let w_found = data.anchors["found7"];
    let mut f = FrameSpec::new(rig_noon([7.0, 12.0, 5.0], [0.0, 0.0, -2.0], fire_light(1.0, t)), g_jungle());
    build_jg(&mut f, JgDyn { fire: 1.0, t, extra_std: Vec::new() });
    let k = cam_k(lt, d, 0.1);
    shoot(&mut f, [1.0, 2.0, 6.2], [0.8, 1.8, 5.0], [0.35, 1.1, -1.2], None, k, 62.0);
    let wk_s = seg(lt, 0.1, 3.3);
    let mut suz = fs();
    let ph = walk_phase(wk_s * 3.28, 0.6);
    suz.phase = ph as f32;
    suz.bob = walk_bob(ph, 5.0) as f32;
    suz.legs = if wk_s >= 1.0 { Legs::Stand } else { Legs::Walk };
    suz.arm_l = (38.0, -52.0);
    suz.arm_r = (12.0, 8.0);
    if t >= w_found {
        suz.mood = Mood::Shock;
    }
    suz.blink = blink_of(5.0, t, &[w_found]);
    let sidx = {
        char(&mut f, SUZUKI, [lerp(3.6, 1.55, wk_s) as f32, 0.0, lerp(-3.2, -0.8, wk_s) as f32], suz, TINT_NOON, false);
        f.chars.len() - 1
    };
    f.chars[sidx].hold_l = Some("paper-map");
    let wk_o = seg(lt, 0.35, 1.95);
    let mut onoda = fs();
    let ph2 = walk_phase(wk_o * 2.25, 0.6);
    onoda.phase = ph2 as f32;
    onoda.bob = walk_bob(ph2, 4.0) as f32;
    onoda.legs = if wk_o >= 1.0 { Legs::Stand } else { Legs::Walk };
    onoda.mood = Mood::Determined;
    onoda.arm_r = (16.0, -6.0);
    onoda.arm_l = (10.0, 6.0);
    onoda.blink = Blinker { seed: 3.0 }.at(t) as f32;
    char(&mut f, ONODA, [lerp(-2.9, -1.1, wk_o) as f32, 0.0, lerp(-3.1, -1.5, wk_o) as f32], onoda, TINT_NOON, true);
    { let (e, tg) = (f.eye, f.target); add_clouds(&mut f, &[([-28.0, 23.0, -85.0], 13.0), ([26.0, 29.0, -95.0], 16.0), ([58.0, 19.0, -75.0], 10.0)], e, tg); }
    f
}

fn s08b(data: &FilmData, t: f64, lt: f64, d: f64) -> FrameSpec {
    let w_ref = data.anchors["refused7"];
    let mut f = FrameSpec::new(rig_noon([7.0, 12.0, 5.0], [0.0, 0.0, -2.0], fire_light(1.0, t)), g_jungle());
    build_jg(&mut f, JgDyn { fire: 1.0, t, extra_std: Vec::new() });
    let k = cam_k(lt, d, 0.1);
    shoot(&mut f, [0.3, 1.78, 5.1], [0.25, 1.7, 4.55], [0.25, 1.2, -1.15], None, k, 54.0);
    let mut onoda = fs();
    onoda.mood = Mood::Determined;
    onoda.arm_r = (16.0, -6.0);
    onoda.arm_l = (12.0, 4.0);
    onoda.lean = 1.0;
    onoda.head_turn = if t > w_ref { (settle(t, w_ref, 5.5, 3.4) * 0.42) as f32 } else { 0.0 };
    onoda.blink = Blinker { seed: 3.0 }.at(t) as f32;
    char(&mut f, ONODA, [-0.95, 0.0, -1.5], onoda, TINT_NOON, true);
    let mut suz = fs();
    pose_listen(&mut suz);
    suz.mood = Mood::Worried;
    suz.blink = Blinker { seed: 5.0 }.at(t) as f32;
    let sidx = {
        char(&mut f, SUZUKI, [1.55, 0.0, -0.8], suz, TINT_NOON, false);
        f.chars.len() - 1
    };
    f.chars[sidx].hold_l = Some("paper-map");
    { let (e, tg) = (f.eye, f.target); add_clouds(&mut f, &[([-28.0, 23.0, -85.0], 13.0), ([26.0, 29.0, -95.0], 16.0), ([58.0, 19.0, -75.0], 10.0)], e, tg); }
    f
}

fn s09(data: &FilmData, t: f64, lt: f64, d: f64) -> FrameSpec {
    let st0 = 41.732;
    let w_comm = data.anchors["commander8"];
    let w_only = data.anchors["only8"];
    let mut f = FrameSpec::new(rig_noon([7.0, 12.0, 5.0], [0.0, 0.0, -2.0], fire_light(1.0, t)), g_jungle());
    build_jg(&mut f, JgDyn { fire: 1.0, t, extra_std: Vec::new() });
    let k = cam_k(lt, d, 0.1);
    shoot(&mut f, [0.3, 1.75, 4.9], [0.25, 1.68, 4.35], [0.25, 1.2, -1.15], None, k, 54.0);
    let mut onoda = fs();
    onoda.mood = Mood::Determined;
    onoda.arm_r = (
        pose_track(&[key(0.0, 12.0), keym(w_comm - st0 - 0.12, 56.0, 0.3), keym(w_comm - st0 + 1.1, 14.0, 0.45)], lt) as f32,
        pose_track(&[key(0.0, 4.0), keym(w_comm - st0 - 0.12, -116.0, 0.3), keym(w_comm - st0 + 1.1, 6.0, 0.45)], lt) as f32,
    );
    onoda.arm_l = (12.0, 4.0);
    onoda.blink = blink_of(3.0, t, &[w_comm]);
    char(&mut f, ONODA, [-0.95, 0.0, -1.5], onoda, TINT_NOON, true);
    let mut suz = fs();
    if t > w_only + 0.3 {
        pose_think(&mut suz);
    } else {
        pose_listen(&mut suz);
    }
    suz.blink = Blinker { seed: 5.0 }.at(t) as f32;
    let sidx = {
        char(&mut f, SUZUKI, [1.55, 0.0, -0.8], suz, TINT_NOON, false);
        f.chars.len() - 1
    };
    f.chars[sidx].hold_l = Some("paper-map");
    { let (e, tg) = (f.eye, f.target); add_clouds(&mut f, &[([-28.0, 23.0, -85.0], 13.0), ([26.0, 29.0, -95.0], 16.0), ([58.0, 19.0, -75.0], 10.0)], e, tg); }
    f
}

fn s10(data: &FilmData, t: f64, lt: f64, d: f64) -> FrameSpec {
    let st0 = 45.365;
    let w_books = data.anchors["bookseller9"];
    let w_flew = data.anchors["flew9"];
    let mut f = FrameSpec::new(rig_bk(), g_shop());
    f.clear = hex_lin("#1A1418");
    build_bk(&mut f);
    let k = cam_k(lt, d, 0.1);
    shoot(&mut f, [0.7, 1.95, 1.6], [0.35, 1.85, 0.4], [-1.5, 1.45, -5.3], None, k, 46.0);
    let wk = seg(t, w_flew + 0.5, w_flew + 1.7);
    let mut tani = fs();
    let ph = walk_phase(wk * 0.55, 0.58);
    tani.phase = ph as f32;
    tani.bob = walk_bob(ph, 3.0) as f32;
    tani.legs = if wk >= 1.0 { Legs::Stand } else { Legs::Walk };
    tani.arm_r = (pose_track(&[key(0.0, 42.0), keym(w_flew - st0, 17.0, 0.5)], lt) as f32, -28.0);
    tani.arm_l = (14.0, 8.0);
    if t >= w_flew {
        tani.mood = Mood::Determined;
    } else if t >= w_books {
        tani.mood = Mood::Awe;
        tani.head_turn = 0.3;
    }
    tani.blink = blink_of(9.0, t, &[w_books]);
    let idx = {
        char(&mut f, BK_TANI, [-1.7, 0.0, lerp(-5.5, -4.95, wk) as f32], tani, TINT_NOON, false);
        f.chars.len() - 1
    };
    f.chars[idx].hold_r = Some(if t >= w_flew { "book-closed" } else { "book-open" });
    f
}

fn rifle_down_draws(t: f64, drop_t: f64) -> Vec<StdDraw> {
    let mut out = Vec::new();
    let u = seg(t, drop_t, drop_t + 0.3);
    let e = eo(u);
    let y = lerp(0.32, 0.07, e);
    let rz = std::f32::consts::FRAC_PI_2 - 0.1 * (1.0 - e) as f32;
    let base = math::mul(&t3(0.62, y as f32, -0.1), &math::euler_xyz(0.0, -0.5, rz));
    let wood = hex_lin("#6E4F30");
    let steel = hex_lin("#3A3D42");
    let parts: [(Geo, Mat4, StdMat); 5] = [
        (box_geo(0.075, 0.52, 0.13, wood), t3(0.0, -0.17, 0.0), sm(0.85, 0.0)),
        (box_geo(0.06, 0.1, 0.09, steel), t3(0.0, 0.13, 0.0), sm(0.5, 0.55)),
        (cyl_geo(0.016, 0.016, 0.55, 8, steel), t3(0.0, 0.42, 0.0), sm(0.5, 0.55)),
        (box_geo(0.02, 0.05, 0.02, steel), t3(0.0, 0.68, 0.0), sm(0.5, 0.55)),
        (box_geo(0.05, 0.05, 0.16, wood), t3(0.0, -0.02, 0.05), sm(0.85, 0.0)),
    ];
    for (geo, local, m) in parts {
        add(&mut out, geo, math::mul(&base, &local), m, true, true);
    }
    out
}

fn s11(data: &FilmData, t: f64, lt: f64, d: f64) -> FrameSpec {
    let st0 = 50.648;
    let w_order = data.anchors["order10"];
    let w_surr = data.anchors["surrendered10"];
    let drop_t = 53.570f64;
    let mut post = g_jungle();
    post.sat = 1.08;
    post.warm = 0.06;
    post.exposure = 0.97;
    let mut extra = Vec::new();
    if t >= drop_t {
        extra = rifle_down_draws(t, drop_t);
    }
    let mut f = FrameSpec::new(rig_dusk(fire_light(1.0, t)), post);
    f.fog = Some((hex_lin("#F59C6B"), 28.0, 120.0));
    build_jg(&mut f, JgDyn { fire: 1.0, t, extra_std: extra });
    let k = cam_k(lt, d, 0.1);
    shoot(&mut f, [0.45, 1.95, 5.0], [0.3, 1.7, 3.6], [0.45, 1.15, -1.1], None, k, 48.0);
    if t < drop_t {
        let wk = seg(lt, 0.1, (w_order - st0 - 0.25).max(0.4));
        let mut onoda = fs();
        let ph = walk_phase(wk * 2.5, 0.6);
        onoda.phase = ph as f32;
        onoda.bob = walk_bob(ph, 4.0) as f32;
        onoda.legs = if wk >= 1.0 { Legs::Stand } else { Legs::Walk };
        onoda.mood = if t >= w_order { Mood::Awe } else { Mood::Determined };
        onoda.lean = 1.0;
        onoda.blink = Blinker { seed: 3.0 }.at(t) as f32;
        char(&mut f, ONODA, [lerp(-2.65, -0.15, wk) as f32, 0.0, lerp(-0.45, -0.95, wk) as f32], onoda, tint_dusk(), true);
    } else {
        let mut onoda = fs();
        onoda.arm_r = (
            pose_track(&[key(0.0, 34.0), keym(drop_t - st0 + 0.55, 7.0, 0.45)], lt) as f32,
            pose_track(&[key(0.0, 0.0), keym(drop_t - st0 + 0.55, 3.0, 0.45)], lt) as f32,
        );
        onoda.mood = if t >= drop_t + 0.85 { Mood::Relieved } else { Mood::Awe };
        onoda.lean = pose_track(&[key(0.0, 1.0), keym(drop_t - st0 + 0.55, 0.0, 0.45)], lt) as f32;
        onoda.blink = Blinker { seed: 3.0 }.at(t) as f32;
        char(&mut f, ONODA2, [-0.15, 0.0, -0.95], onoda, tint_dusk(), false);
    }
    let mut tani = fs();
    tani.arm_r = (pose_track(&[key(0.0, 12.0), keym(w_order - st0 - 0.13, 112.0, 0.32), keym(w_surr - st0 + 0.7, 14.0, 0.5)], lt) as f32, -8.0);
    tani.arm_l = (12.0, 6.0);
    let bow = pose_track(&[key(0.0, 0.0), keym(drop_t - st0 + 1.15, 7.0, 0.5)], lt);
    tani.lean = bow as f32;
    tani.head_tilt = (-0.8 * bow) as f32;
    tani.talk = talk(t, t > w_order - 0.1 && t < w_order + 0.75) as f32;
    tani.mood = Mood::Determined;
    tani.blink = Blinker { seed: 9.0 }.at(t) as f32;
    char(&mut f, TANI, [1.3, 0.0, -1.3], tani, tint_dusk(), false);
    { let (e, tg) = (f.eye, f.target); add_clouds(&mut f, &[([-28.0, 23.0, -85.0], 13.0), ([26.0, 29.0, -95.0], 16.0), ([58.0, 19.0, -75.0], 10.0)], e, tg); }
    f
}

fn s12(data: &FilmData, t: f64, lt: f64, d: f64) -> FrameSpec {
    let st0 = 55.520;
    let w_loyal = data.anchors["loyal11"];
    let mut post = g_jungle();
    post.sat = 1.08;
    post.warm = 0.06;
    post.exposure = 0.97;
    let mut f = FrameSpec::new(rig_dusk(fire_light(1.0, t)), post);
    f.fog = Some((hex_lin("#F59C6B"), 28.0, 120.0));
    build_jg(&mut f, JgDyn { fire: 1.0, t, extra_std: rifle_down_draws(1.0, 0.0) });
    let k = cam_k(lt, d, 0.1);
    f.eye = mix3([0.4, 1.8, 4.6], [1.6, 4.6, 9.8], k);
    f.target = mix3([0.05, 1.0, -1.15], [0.05, 0.8, -1.35], k);
    f.fov = 46.0;
    f.focus = ((f.eye[0] - f.target[0]).powi(2) + (f.eye[1] - f.target[1]).powi(2) + (f.eye[2] - f.target[2]).powi(2)).sqrt();
    let mut onoda = fs();
    let bow = pose_track(&[key(0.0, 0.0), keym(w_loyal - st0 - 0.1, 3.5, 0.45)], lt);
    onoda.lean = (0.6 * bow) as f32;
    onoda.head_tilt = (-bow) as f32;
    onoda.arm_r = (9.0, 5.0);
    onoda.arm_l = (9.0, 5.0);
    onoda.look = [0.25, -0.15];
    onoda.mood = if t >= w_loyal { Mood::Neutral } else { Mood::Relieved };
    onoda.blink = blink_of(3.0, t, &[w_loyal]);
    char(&mut f, ONODA2, [-0.15, 0.0, -0.95], onoda, tint_dusk(), false);
    let mut tani = fs();
    tani.arm_r = (10.0, 6.0);
    tani.arm_l = (10.0, 6.0);
    tani.head_tilt = -3.0;
    tani.blink = Blinker { seed: 9.0 }.at(t) as f32;
    char(&mut f, TANI, [2.3, 0.0, -2.6], tani, tint_dusk(), false);
    { let (e, tg) = (f.eye, f.target); add_clouds(&mut f, &[([-28.0, 23.0, -85.0], 13.0), ([26.0, 29.0, -95.0], 16.0), ([58.0, 19.0, -75.0], 10.0)], e, tg); }
    f
}

/// Shot table: (segment start, evaluator). Starts per main.ts:
/// cut(line) = lineStart - 0.06, with the three word-anchored
/// cuts (S05b/S07/S08b) at their Wd() offsets.
pub(crate) fn eval_frame(data: &FilmData, looks: &[(FullLook, bool, f64)], t: f64) -> FrameSpec {
    let a = &data.anchors;
    let l = |i: usize| data.lines[i].0 - 0.06;
    let shots: [(f64, fn(&FilmData, f64, f64, f64) -> FrameSpec); 14] = [
        (0.0, s01),
        (l(1), s02),
        (l(2), s03),
        (l(3), s04),
        (l(4), s05a),
        (a["thought4"] - 0.12, s05b),
        (l(5), s06),
        (a["decades6"] - 0.25, s07),
        (l(7), s08),
        (a["refused7"] - 0.1, s08b),
        (l(8), s09),
        (l(9), s10),
        (l(10), s11),
        (l(11), s12),
    ];
    let mut idx = 0;
    for (i, (at, _)) in shots.iter().enumerate() {
        if t >= *at {
            idx = i;
        }
    }
    let (at, fnh) = shots[idx];
    let end = if idx + 1 < shots.len() { shots[idx + 1].0 } else { data.duration };
    let mut f = fnh(data, t, t - at, end - at);
    finish_chars(&mut f, looks);
    // film fade + caption (main.ts draw order: shot, fade, caption)
    let fade_factor = clamp(t / 0.25, 0.0, 1.0).min(clamp((data.duration - t) / 0.4, 0.0, 1.0));
    f.fade = (1.0 - fade_factor) as f32;
    if let Some((ci, c)) = data.captions.iter().enumerate().find(|(_, c)| t >= c.s && t < c.e) {
        let lt = t - c.s;
        let sc = if lt < 0.1 { lerp(0.86, 1.0, eob(seg(lt, 0.0, 0.1))) } else { 1.0 };
        f.caption = Some(ci);
        f.caption_scale = (sc * c.fit) as f32;
    }
    f
}

// ---------------------------------------------------------------------
// Assets (film/assets, from gen_assets.py) + character resolution.
// ---------------------------------------------------------------------

pub(crate) struct Assets {
    pub plates: HashMap<String, Plate>,
    pub strips: Vec<(Vec<u8>, u32, u32)>,
    pub cal: Vec<(Vec<u8>, u32, u32)>,
    pub cal_dark: (Vec<u8>, u32, u32),
    pub sign: (Vec<u8>, u32, u32),
}

fn load_png(path: &Path) -> Result<(Vec<u8>, u32, u32), String> {
    let img = image::open(path).map_err(|e| format!("cannot load {}: {e}", path.display()))?.to_rgba8();
    let (w, h) = img.dimensions();
    Ok((img.into_raw(), w, h))
}

pub(crate) fn load_assets(dir: &Path, data: &FilmData) -> Result<Assets, String> {
    let adir = dir.join("assets");
    let origins_text = std::fs::read_to_string(adir.join("plates.json")).map_err(|e| format!("plates.json: {e}"))?;
    let origins = crate::scene::parse_json(&origins_text)?;
    let mut plate_map = HashMap::new();
    if let Value::Obj(m) = &origins {
        for name in ["paper-map", "leaflet", "book-open", "book-closed", "coconut"] {
            let (px, w, h) = load_png(&adir.join(format!("{name}.png")))?;
            let (mut x0, mut y0) = (0.0f32, 0.0f32);
            let entry = m.iter().find(|(k, _)| k == name).map(|(_, v)| v);
            if let Some(Value::Arr(a)) = entry {
                if let (Some(a0), Some(a1)) = (a.first(), a.get(1)) {
                    x0 = a0.as_f64().unwrap_or(0.0) as f32;
                    y0 = a1.as_f64().unwrap_or(0.0) as f32;
                }
            }
            plate_map.insert(name.to_string(), Plate { px, w, h, x0, y0 });
        }
    }
    let mut strips = Vec::new();
    for c in &data.captions {
        strips.push(load_png(&adir.join(&c.strip))?);
    }
    let mut cal = Vec::new();
    for y in ["1945", "1955", "1965", "1974"] {
        cal.push(load_png(&adir.join(format!("calendar-{y}.png")))?);
    }
    Ok(Assets {
        plates: plate_map,
        strips,
        cal,
        cal_dark: load_png(&adir.join("calendar-dark.png"))?,
        sign: load_png(&adir.join("sign.png"))?,
    })
}

type Painted = (Vec<u8>, u32, u32, (f32, f32), (f32, f32));

/// Per-character render record for the telemetry audit: the card's
/// world model, the painted canvas, and the pose-summary fields.
pub(crate) struct CharInfo {
    pub look: usize,
    pub model: Mat4,
    pub painted: std::rc::Rc<Painted>,
    pub mood: Mood,
    pub arm_l: (f32, f32),
    pub arm_r: (f32, f32),
    pub legs: Legs,
    pub head_tilt: f32,
    pub head_turn: f32,
}
type PaintCache = HashMap<String, std::rc::Rc<Painted>>;

fn state_key(look: usize, st: &FullState, hold_l: Option<&str>, hold_r: Option<&str>) -> String {
    let q = |v: f32| (v * 100.0).round() as i32;
    format!(
        "{look}|{:?}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
        st.mood,
        q(st.look[0]), q(st.look[1]), q(st.blink), q(st.talk),
        q(st.arm_l.0), q(st.arm_l.1), q(st.arm_r.0), q(st.arm_r.1),
        q(st.phase), q(st.lean), q(st.bob), q(st.squash),
        match st.legs { Legs::Stand => "stand", Legs::Walk => "walk", Legs::Run => "run", Legs::Sit => "sit", Legs::Jump => "jump" },
        q(st.head_tilt), q(st.head_turn), q(st.cam_pitch),
        st.sit_drop.map(q).unwrap_or(-1),
        hold_l.unwrap_or("-"),
        hold_r.unwrap_or("-"),
    )
}

/// Paint every character (cache by the Toon's own 2-dp-rounded
/// repaint key), then append blob + card draws and held rifles to
/// the frame. Returns the per-frame dynamic textures (card images)
/// in TexSrc::Dyn order.
fn resolve_chars(
    f: &mut FrameSpec,
    looks: &[(FullLook, bool, f64)],
    assets: &Assets,
    cache: &mut PaintCache,
) -> (Vec<std::rc::Rc<Painted>>, Vec<CharInfo>) {
    let mut dyn_tex: Vec<std::rc::Rc<Painted>> = Vec::new();
    let mut infos: Vec<CharInfo> = Vec::new();
    let chars = std::mem::take(&mut f.chars);
    for c in &chars {
        let (look, _, _) = &looks[c.look];
        let mut st = FullState {
            mood: c.state.mood,
            look: c.state.look,
            blink: c.state.blink,
            talk: c.state.talk,
            sweat: c.state.sweat,
            tears: c.state.tears,
            arm_l: c.state.arm_l,
            arm_r: c.state.arm_r,
            legs: c.state.legs,
            phase: c.state.phase,
            lean: c.state.lean,
            bob: c.state.bob,
            squash: c.state.squash,
            head_tilt: c.state.head_tilt,
            head_turn: c.state.head_turn,
            cam_pitch: c.state.cam_pitch,
            sit_drop: c.state.sit_drop,
            hold_l: None,
            hold_r: None,
        };
        let plate_of = |name: Option<&str>| -> Option<Plate> {
            name.and_then(|n| assets.plates.get(n)).map(|p| Plate { px: p.px.clone(), w: p.w, h: p.h, x0: p.x0, y0: p.y0 })
        };
        st.hold_l = plate_of(c.hold_l);
        st.hold_r = plate_of(c.hold_r);
        let key = state_key(c.look, &st, c.hold_l, c.hold_r);
        let entry = cache
            .entry(key)
            .or_insert_with(|| std::rc::Rc::new(paint_toon_full(look, &st)))
            .clone();
        let hand_r = entry.4;
        let h_t = 1.75f32 / 0.96;
        let su = h_t / 1024.0;
        let plane_w = h_t * 1280.0 / 1024.0;
        let plane_h = h_t * 1408.0 / 1024.0;
        // blob shadow at the feet
        f.basics.push(BasicDraw {
            geo: flat_quad_geo((h_t / 2.0) * 0.9, (h_t / 2.0) * 0.34),
            model: t3(c.pos[0], c.pos[1] + 0.01, c.pos[2]),
            tex: TexSrc::S(TEX_BLOB),
            tint: [1.0, 1.0, 1.0],
            opacity: 0.55,
            alpha_test: 0.0,
            depth_write: false,
            fog: true,
            prepass: true,
            repeat: false,
        });
        let card_model = math::mul(
            &t3(c.pos[0], c.pos[1], c.pos[2]),
            &math::mul(&math::rotate_y(c.yaw), &math::rotate_x(c.pitch)),
        );
        infos.push(CharInfo {
            look: c.look,
            model: card_model,
            painted: entry.clone(),
            mood: st.mood,
            arm_l: st.arm_l,
            arm_r: st.arm_r,
            legs: st.legs,
            head_tilt: st.head_tilt,
            head_turn: st.head_turn,
        });
        let slot = dyn_tex.len();
        dyn_tex.push(entry);
        f.basics.push(BasicDraw {
            geo: quad_geo(plane_w, plane_h, 552.0 * su - plane_h / 2.0),
            model: card_model,
            tex: TexSrc::Dyn(slot),
            tint: c.tint,
            opacity: 1.0,
            alpha_test: 0.04,
            depth_write: true,
            fog: true,
            prepass: true,
            repeat: false,
        });
        if c.rifle {
            // syncAttachments: hand R (stage coords) -> card local,
            // offset [0, 0.03, 0.07] in card space.
            let (hx, hy) = hand_r;
            let local = [(384.0 + hx - 640.0) * su, (1256.0 - (256.0 + hy)) * su, 0.0];
            let rifle_model = math::mul(&card_model, &t3(local[0], local[1] + 0.03, local[2] + 0.07));
            let wood = hex_lin("#6E4F30");
            let steel = hex_lin("#3A3D42");
            let parts: [(Geo, Mat4, StdMat); 5] = [
                (box_geo(0.075, 0.52, 0.13, wood), t3(0.0, -0.17, 0.0), sm(0.85, 0.0)),
                (box_geo(0.06, 0.1, 0.09, steel), t3(0.0, 0.13, 0.0), sm(0.5, 0.55)),
                (cyl_geo(0.016, 0.016, 0.55, 8, steel), t3(0.0, 0.42, 0.0), sm(0.5, 0.55)),
                (box_geo(0.02, 0.05, 0.02, steel), t3(0.0, 0.68, 0.0), sm(0.5, 0.55)),
                (box_geo(0.05, 0.05, 0.16, wood), t3(0.0, -0.02, 0.05), sm(0.85, 0.0)),
            ];
            for (geo, plocal, m) in parts {
                add(&mut f.std, geo, math::mul(&rifle_model, &plocal), m, true, true);
            }
        }
    }
    (dyn_tex, infos)
}

// ---------------------------------------------------------------------
// The renderer: ab1::run_ab1's production chain, parameterised per
// frame. Pipelines and targets are built once; per frame it builds
// fresh vertex/uniform buffers from the evaluated FrameSpec.
// ---------------------------------------------------------------------

fn mb(m: &Mat4) -> [u8; 64] {
    let mut out = [0u8; 64];
    let mut k = 0;
    for col in 0..4 {
        for row in 0..4 {
            out[k..k + 4].copy_from_slice(&m[col][row].to_ne_bytes());
            k += 4;
        }
    }
    out
}
fn pack_f32(data: &[f32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len() * 4);
    for v in data {
        out.extend_from_slice(&v.to_ne_bytes());
    }
    out
}
fn pack_u32(data: &[u32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len() * 4);
    for v in data {
        out.extend_from_slice(&v.to_ne_bytes());
    }
    out
}
fn depth_bytes(mvp: &Mat4, model: &Mat4) -> Vec<u8> {
    let mut out = Vec::with_capacity(128);
    out.extend_from_slice(&mb(mvp));
    out.extend_from_slice(&mb(model));
    out
}

pub(crate) struct Renderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    width: u32,
    height: u32,
    bgl_unif: wgpu::BindGroupLayout,
    bgl_basic: wgpu::BindGroupLayout,
    bgl_shadow_card: wgpu::BindGroupLayout,
    pbr_bgl0: wgpu::BindGroupLayout,
    pbr_bgl1: wgpu::BindGroupLayout,
    pbr_pipeline: wgpu::RenderPipeline,
    pbr_tex_pipeline: wgpu::RenderPipeline,
    pbr_map_pipeline: wgpu::RenderPipeline,
    basic_write: wgpu::RenderPipeline,
    basic_nowrite: wgpu::RenderPipeline,
    prepass_geo: wgpu::RenderPipeline,
    prepass_map: wgpu::RenderPipeline,
    prepass_tex: wgpu::RenderPipeline,
    shadow_geo: wgpu::RenderPipeline,
    shadow_map44: wgpu::RenderPipeline,
    shadow_card: wgpu::RenderPipeline,
    bokeh_bgl: wgpu::BindGroupLayout,
    bokeh_pipeline: wgpu::RenderPipeline,
    post: PostChain,
    shadow_map_view: wgpu::TextureView,
    shadow_depth_view: wgpu::TextureView,
    packed_view: wgpu::TextureView,
    prepass_depth_view: wgpu::TextureView,
    hdr_msaa_view: wgpu::TextureView,
    hdr_view: wgpu::TextureView,
    hdr_depth_view: wgpu::TextureView,
    bokeh_view: wgpu::TextureView,
    srgb_view: wgpu::TextureView,
    final_tex: wgpu::Texture,
    final_view: wgpu::TextureView,
    tex_views: Vec<wgpu::TextureView>,
    _tex_keep: Vec<wgpu::Texture>,
    env_view: wgpu::TextureView,
    dummy_view: wgpu::TextureView,
    sampler_clamp: wgpu::Sampler,
    sampler_repeat: wgpu::Sampler,
    sampler_nearest: wgpu::Sampler,
}

impl Renderer {
    pub(crate) async fn new(width: u32, height: u32, sign: &(Vec<u8>, u32, u32)) -> Renderer {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN,
            ..Default::default()
        });
        let adapter = crate::request_adapter(&instance).await;
        let info = adapter.get_info();
        println!(
            "renderer: {} | backend: {:?} | device type: {:?} | driver: {} {}",
            info.name, info.backend, info.device_type, info.driver, info.driver_info
        );
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("talerender film device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                },
                None,
            )
            .await
            .expect("request_device failed");
        let ceil_motif = {
            let (r, g, b) = crate::scene::shade_rgb("#3A2E22", 1.25);
            format!("#{r:02X}{g:02X}{b:02X}")
        };
        let textures_px: Vec<(Vec<u8>, u32, u32)> = vec![
            paint_sky("#6E9CC0", "#D8E8DA"),
            paint_sky("#6E9CC0", "#E4EEDC"),
            paint_grass("#5B9A48"),
            paint_grass("#5E8F52"),
            paint_cloud(),
            paint_blob(),
            paint_planks("#6B3E26"),
            paint_planks("#7A5230"),
            paint_wallpaper("#5A4632", "#6B5638"),
            paint_parcel(),
            sign.clone(),
            paint_picture(),
            paint_felt("#2E5A44"),
            paint_felt("#1F5A45"),
            (vec![191, 228, 247, 255], 1, 1),
            paint_planks("#B07A48"),
            paint_wallpaper("#3A2E22", &ceil_motif),
            paint_planks("#7A3E24"),
        ];
        let upload = |device: &wgpu::Device, queue: &wgpu::Queue, px: &[u8], w: u32, h: u32, format: wgpu::TextureFormat| -> wgpu::Texture {
            let tex = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("film texture"),
                size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
            queue.write_texture(
                wgpu::ImageCopyTexture { texture: &tex, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
                px,
                wgpu::ImageDataLayout { offset: 0, bytes_per_row: Some(w * 4), rows_per_image: Some(h) },
                wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
            );
            tex
        };
        let mut tex_views = Vec::new();
        let mut tex_keep = Vec::new();
        for (px, w, h) in &textures_px {
            let t = upload(&device, &queue, px, *w, *h, wgpu::TextureFormat::Rgba8UnormSrgb);
            tex_views.push(t.create_view(&wgpu::TextureViewDescriptor::default()));
            tex_keep.push(t);
        }
        let env_tex = crate::composer::load_room_env(&device, &queue);
        let env_view = env_tex.create_view(&wgpu::TextureViewDescriptor::default());
        let dummy_tex = upload(&device, &queue, &[255u8, 255, 255, 255], 1, 1, wgpu::TextureFormat::Rgba8UnormSrgb);
        let dummy_view = dummy_tex.create_view(&wgpu::TextureViewDescriptor::default());
        tex_keep.push(env_tex);
        tex_keep.push(dummy_tex);
        let sampler_clamp = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("clamp sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let sampler_repeat = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("repeat sampler"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let sampler_nearest = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("nearest sampler"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        let ubo_layout = |label: &str, vis: wgpu::ShaderStages| {
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some(label),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: vis,
                    ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None },
                    count: None,
                }],
            })
        };
        let bgl_unif = ubo_layout("uniform layout", wgpu::ShaderStages::VERTEX);
        let tex_bgl = |with_uniform: bool| {
            let mut entries = Vec::new();
            if with_uniform {
                entries.push(wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None },
                    count: None,
                });
            }
            let base = if with_uniform { 1 } else { 0 };
            entries.push(wgpu::BindGroupLayoutEntry {
                binding: base,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: true }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false },
                count: None,
            });
            entries.push(wgpu::BindGroupLayoutEntry {
                binding: base + 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            });
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor { label: Some("tex layout"), entries: &entries })
        };
        let bgl_basic = tex_bgl(true);
        let bgl_shadow_card = tex_bgl(true);
        let pbr_bgl0 = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("pbr uniforms layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None }, count: None },
            ],
        });
        let pbr_bgl1 = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("pbr textures layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: true }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering), count: None },
                wgpu::BindGroupLayoutEntry { binding: 2, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: true }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 3, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: true }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 4, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering), count: None },
            ],
        });
        let shadow_map = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("key shadow map"),
            size: wgpu::Extent3d { width: 2048, height: 2048, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let shadow_map_view = shadow_map.create_view(&wgpu::TextureViewDescriptor::default());
        tex_keep.push(shadow_map);
        let shadow_depth = crate::composer::make_rt(&device, 2048, 2048, wgpu::TextureFormat::Depth32Float, "shadow depth");
        let shadow_depth_view = shadow_depth.create_view(&wgpu::TextureViewDescriptor::default());
        tex_keep.push(shadow_depth);
        let depth_format = if std::env::var("TALERENDER_DOF_DEPTH_FORMAT").is_ok_and(|v| v == "halffloat") {
            wgpu::TextureFormat::Rgba16Float
        } else {
            wgpu::TextureFormat::Rgba8Unorm
        };
        let packed_depth = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("packed depth"),
            size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: depth_format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let packed_view = packed_depth.create_view(&wgpu::TextureViewDescriptor::default());
        tex_keep.push(packed_depth);
        let prepass_depth = crate::composer::make_rt(&device, width, height, wgpu::TextureFormat::Depth32Float, "prepass depth");
        let prepass_depth_view = prepass_depth.create_view(&wgpu::TextureViewDescriptor::default());
        tex_keep.push(prepass_depth);
        let hdr_msaa = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("hdr msaa"),
            size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 4,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let hdr_msaa_view = hdr_msaa.create_view(&wgpu::TextureViewDescriptor::default());
        tex_keep.push(hdr_msaa);
        let hdr_tex = crate::composer::make_rt(&device, width, height, wgpu::TextureFormat::Rgba16Float, "hdr resolved");
        let hdr_view = hdr_tex.create_view(&wgpu::TextureViewDescriptor::default());
        tex_keep.push(hdr_tex);
        let hdr_depth = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("hdr depth"),
            size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 4,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let hdr_depth_view = hdr_depth.create_view(&wgpu::TextureViewDescriptor::default());
        tex_keep.push(hdr_depth);
        let bokeh_tex = crate::composer::make_rt(&device, width, height, wgpu::TextureFormat::Rgba16Float, "bokeh out");
        let bokeh_view = bokeh_tex.create_view(&wgpu::TextureViewDescriptor::default());
        tex_keep.push(bokeh_tex);
        let srgb_tex = crate::composer::make_rt(&device, width, height, wgpu::TextureFormat::Rgba8Unorm, "output srgb");
        let srgb_view = srgb_tex.create_view(&wgpu::TextureViewDescriptor::default());
        tex_keep.push(srgb_tex);
        let final_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("final"),
            size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let final_view = final_tex.create_view(&wgpu::TextureViewDescriptor::default());
        let module = |label: &str, src: &str| {
            device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(label),
                source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(src)),
            })
        };
        let colour_attrs = [
            wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x3, offset: 0, shader_location: 0 },
            wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x3, offset: 12, shader_location: 1 },
            wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x3, offset: 24, shader_location: 2 },
        ];
        let tex_attrs = [
            wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x3, offset: 0, shader_location: 0 },
            wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x2, offset: 12, shader_location: 1 },
        ];
        let pos36_attrs = [wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x3, offset: 0, shader_location: 0 }];
        let depth_state = |write: bool| {
            Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: write,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            })
        };
        let shader_pbr = module("pbr shader", include_str!("shader_pbr.wgsl"));
        let pbr_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("pbr layout"),
            bind_group_layouts: &[&pbr_bgl0, &pbr_bgl1],
            push_constant_ranges: &[],
        });
        let make_pbr = |entry: &str, buffers: &[wgpu::VertexBufferLayout]| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("pbr pipeline"),
                layout: Some(&pbr_layout),
                vertex: wgpu::VertexState { module: &shader_pbr, entry_point: entry, buffers, compilation_options: Default::default() },
                fragment: Some(wgpu::FragmentState {
                    module: &shader_pbr,
                    entry_point: "fs_main",
                    targets: &[Some(wgpu::ColorTargetState { format: wgpu::TextureFormat::Rgba16Float, blend: Some(wgpu::BlendState::REPLACE), write_mask: wgpu::ColorWrites::ALL })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState { cull_mode: Some(wgpu::Face::Back), ..Default::default() },
                depth_stencil: depth_state(true),
                multisample: wgpu::MultisampleState { count: 4, ..Default::default() },
                multiview: None,
            })
        };
        let pbr_pipeline = make_pbr("vs_colour", &[wgpu::VertexBufferLayout { array_stride: 36, step_mode: wgpu::VertexStepMode::Vertex, attributes: &colour_attrs }]);
        let pbr_tex_pipeline = make_pbr("vs_tex", &[wgpu::VertexBufferLayout { array_stride: 20, step_mode: wgpu::VertexStepMode::Vertex, attributes: &tex_attrs }]);
        let map_attrs = [
            wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x3, offset: 0, shader_location: 0 },
            wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x3, offset: 12, shader_location: 1 },
            wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x3, offset: 24, shader_location: 2 },
            wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x2, offset: 36, shader_location: 3 },
        ];
        let pbr_map_pipeline = make_pbr("vs_map", &[wgpu::VertexBufferLayout { array_stride: 44, step_mode: wgpu::VertexStepMode::Vertex, attributes: &map_attrs }]);
        let shader_basic = module("basic shader", include_str!("shader_basic.wgsl"));
        let basic_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("basic layout"),
            bind_group_layouts: &[&bgl_basic],
            push_constant_ranges: &[],
        });
        let make_basic = |format: wgpu::TextureFormat, depth: Option<wgpu::DepthStencilState>, samples: u32| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("basic pipeline"),
                layout: Some(&basic_layout),
                vertex: wgpu::VertexState {
                    module: &shader_basic,
                    entry_point: "vs_main",
                    buffers: &[wgpu::VertexBufferLayout { array_stride: 20, step_mode: wgpu::VertexStepMode::Vertex, attributes: &tex_attrs }],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader_basic,
                    entry_point: "fs_main",
                    targets: &[Some(wgpu::ColorTargetState { format, blend: Some(wgpu::BlendState::ALPHA_BLENDING), write_mask: wgpu::ColorWrites::ALL })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState { cull_mode: Some(wgpu::Face::Back), ..Default::default() },
                depth_stencil: depth,
                multisample: wgpu::MultisampleState { count: samples, ..Default::default() },
                multiview: None,
            })
        };
        let basic_write = make_basic(wgpu::TextureFormat::Rgba16Float, depth_state(true), 4);
        let basic_nowrite = make_basic(wgpu::TextureFormat::Rgba16Float, depth_state(false), 4);
        let shader_depth_pos = module("depth pos shader", include_str!("shader_depth_pos.wgsl"));
        let shader_depth = module("depth shader", include_str!("shader_depth.wgsl"));
        let shader_shadow_card = module("shadow card shader", include_str!("shader_shadow_card.wgsl"));
        let depth_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("depth layout"),
            bind_group_layouts: &[&bgl_unif],
            push_constant_ranges: &[],
        });
        let make_depth = |shader: &wgpu::ShaderModule, buffers: &[wgpu::VertexBufferLayout], format: wgpu::TextureFormat, layout: &wgpu::PipelineLayout| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("depth pipeline"),
                layout: Some(layout),
                vertex: wgpu::VertexState { module: shader, entry_point: "vs_main", buffers, compilation_options: Default::default() },
                fragment: Some(wgpu::FragmentState {
                    module: shader,
                    entry_point: "fs_main",
                    targets: &[Some(wgpu::ColorTargetState { format, blend: Some(wgpu::BlendState::REPLACE), write_mask: wgpu::ColorWrites::ALL })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState { cull_mode: Some(wgpu::Face::Back), ..Default::default() },
                depth_stencil: depth_state(true),
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
            })
        };
        let prepass_geo = make_depth(&shader_depth_pos, &[wgpu::VertexBufferLayout { array_stride: 36, step_mode: wgpu::VertexStepMode::Vertex, attributes: &pos36_attrs }], depth_format, &depth_layout);
        let prepass_map = make_depth(&shader_depth_pos, &[wgpu::VertexBufferLayout { array_stride: 44, step_mode: wgpu::VertexStepMode::Vertex, attributes: &pos36_attrs }], depth_format, &depth_layout);
        let prepass_tex = make_depth(&shader_depth, &[wgpu::VertexBufferLayout { array_stride: 20, step_mode: wgpu::VertexStepMode::Vertex, attributes: &tex_attrs }], depth_format, &depth_layout);
        let shadow_geo = make_depth(&shader_depth_pos, &[wgpu::VertexBufferLayout { array_stride: 36, step_mode: wgpu::VertexStepMode::Vertex, attributes: &pos36_attrs }], wgpu::TextureFormat::Rgba8Unorm, &depth_layout);
        let shadow_map44 = make_depth(&shader_depth_pos, &[wgpu::VertexBufferLayout { array_stride: 44, step_mode: wgpu::VertexStepMode::Vertex, attributes: &pos36_attrs }], wgpu::TextureFormat::Rgba8Unorm, &depth_layout);
        let shadow_card_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("shadow card layout"),
            bind_group_layouts: &[&bgl_shadow_card],
            push_constant_ranges: &[],
        });
        let shadow_card = make_depth(&shader_shadow_card, &[wgpu::VertexBufferLayout { array_stride: 20, step_mode: wgpu::VertexStepMode::Vertex, attributes: &tex_attrs }], wgpu::TextureFormat::Rgba8Unorm, &shadow_card_layout);
        // Bokeh pipeline (per-frame bind group: focus/aperture vary).
        let shader_bokeh = module("bokeh shader", include_str!("shader_bokeh.wgsl"));
        let bokeh_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("bokeh layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: true }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 2, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering), count: None },
                wgpu::BindGroupLayoutEntry { binding: 3, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: true }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 4, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering), count: None },
            ],
        });
        let bokeh_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("bokeh pipeline"),
            layout: Some(&device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("bokeh layout"),
                bind_group_layouts: &[&bokeh_bgl],
                push_constant_ranges: &[],
            })),
            vertex: wgpu::VertexState { module: &shader_bokeh, entry_point: "vs_main", buffers: &[], compilation_options: Default::default() },
            fragment: Some(wgpu::FragmentState {
                module: &shader_bokeh,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState { format: wgpu::TextureFormat::Rgba16Float, blend: Some(wgpu::BlendState::REPLACE), write_mask: wgpu::ColorWrites::ALL })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });
        let post = PostChain::new(&device);
        Renderer {
            device,
            queue,
            width,
            height,
            bgl_unif,
            bgl_basic,
            bgl_shadow_card,
            pbr_bgl0,
            pbr_bgl1,
            pbr_pipeline,
            pbr_tex_pipeline,
            pbr_map_pipeline,
            basic_write,
            basic_nowrite,
            prepass_geo,
            prepass_map,
            prepass_tex,
            shadow_geo,
            shadow_map44,
            shadow_card,
            bokeh_bgl,
            bokeh_pipeline,
            post,
            shadow_map_view,
            shadow_depth_view,
            packed_view,
            prepass_depth_view,
            hdr_msaa_view,
            hdr_view,
            hdr_depth_view,
            bokeh_view,
            srgb_view,
            final_tex,
            final_view,
            tex_views,
            _tex_keep: tex_keep,
            env_view,
            dummy_view,
            sampler_clamp,
            sampler_repeat,
            sampler_nearest,
        }
    }
}

struct Prepared {
    vb: wgpu::Buffer,
    ib: wgpu::Buffer,
    n: u32,
    bg0: Option<wgpu::BindGroup>,
    bg: Option<wgpu::BindGroup>,
    prepass_bg: Option<wgpu::BindGroup>,
    shadow_bg: Option<wgpu::BindGroup>,
    _ubs: Vec<wgpu::Buffer>,
}

impl Renderer {
    fn upload_dyn(&self, px: &[u8], w: u32, h: u32) -> wgpu::Texture {
        let tex = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("card texture"),
            size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        self.queue.write_texture(
            wgpu::ImageCopyTexture { texture: &tex, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
            px,
            wgpu::ImageDataLayout { offset: 0, bytes_per_row: Some(w * 4), rows_per_image: Some(h) },
            wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        );
        tex
    }

    pub(crate) fn render(&mut self, spec: &FrameSpec, dyn_tex: &[std::rc::Rc<Painted>]) -> Vec<u8> {
        let (device, queue) = (&self.device, &self.queue);
        let (width, height) = (self.width, self.height);
        let aspect = width as f32 / height as f32;
        let proj = math::perspective(spec.fov.to_radians(), aspect, 0.05, 800.0);
        let view = math::look_at(spec.eye, spec.target, [0.0, 1.0, 0.0]);
        let vp = math::mul(&proj, &view);
        let l = &spec.lights;
        let light_view = math::look_at(l.key_pos, l.key_tgt, [0.0, 1.0, 0.0]);
        let light_proj = math::orthographic(-l.key_box, l.key_box, -l.key_box, l.key_box, l.key_near, l.key_far);
        let bias_mat: Mat4 = [
            [0.5, 0.0, 0.0, 0.0],
            [0.0, 0.5, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.5, 0.5, 0.0, 1.0],
        ];
        let shadow_vp = math::mul(&bias_mat, &math::mul(&light_proj, &light_view));
        let key_dir = norm3(sub3(l.key_pos, l.key_tgt));
        let cam_fwd = norm3(sub3(spec.target, spec.eye));
        let v4 = |c: [f32; 3]| [c[0], c[1], c[2], 0.0];
        let mut frame_bytes: Vec<u8> = Vec::with_capacity(448);
        frame_bytes.extend_from_slice(&mb(&vp));
        frame_bytes.extend_from_slice(&mb(&shadow_vp));
        let pt = |p: &PointState| -> [[f32; 4]; 3] {
            [
                [p.pos[0], p.pos[1], p.pos[2], 1.0],
                v4(p.col),
                [p.dist, p.decay, 0.0, 0.0],
            ]
        };
        let mut vecs: Vec<[f32; 4]> = vec![
            [spec.eye[0], spec.eye[1], spec.eye[2], 1.0],
            v4(l.key_col),
            v4(key_dir),
            v4(l.rim_col),
            v4(l.rim_dir),
            v4(l.hemi_sky),
            v4(l.hemi_ground),
        ];
        let p1 = pt(&l.points[0]);
        vecs.push(p1[0]);
        vecs.push(p1[1]);
        vecs.push([l.points[0].dist, l.points[0].decay, l.env, l.key_bias]);
        vecs.push([1.0 / 2048.0, 2048.0, 0.0, 0.0]);
        for p in [&l.points[1], &l.points[2]] {
            let pp = pt(p);
            vecs.extend_from_slice(&pp);
        }
        match &spec.fog {
            Some((col, near, far)) => {
                vecs.push([col[0], col[1], col[2], *near]);
                vecs.push([*far, 1.0, 0.0, 0.0]);
            }
            None => {
                vecs.push([0.0; 4]);
                vecs.push([0.0; 4]);
            }
        }
        vecs.push(v4(cam_fwd));
        for v in &vecs {
            for x in v {
                frame_bytes.extend_from_slice(&x.to_ne_bytes());
            }
        }
        debug_assert_eq!(frame_bytes.len(), 448);
        let make_ub = |bytes: &[u8], label: &str| -> wgpu::Buffer {
            let buf = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: bytes.len() as u64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            queue.write_buffer(&buf, 0, bytes);
            buf
        };
        let frame_ub = make_ub(&frame_bytes, "frame uniforms");
        let make_vb = |verts: &[f32], idx: &[u32]| -> (wgpu::Buffer, wgpu::Buffer, u32) {
            let vb = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("vertex buffer"),
                size: (verts.len() * 4) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            queue.write_buffer(&vb, 0, &pack_f32(verts));
            let ib = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("index buffer"),
                size: (idx.len() * 4) as u64,
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            queue.write_buffer(&ib, 0, &pack_u32(idx));
            (vb, ib, idx.len() as u32)
        };
        // PBR texture bind groups (group 1), cached per (tex, repeat).
        let bg1 = |r: &Renderer, tex: Option<usize>, repeat: bool| -> wgpu::BindGroup {
            let bg = {
                    let (view, smp) = match tex {
                        Some(i) => (&r.tex_views[i], if repeat { &r.sampler_repeat } else { &r.sampler_clamp }),
                        None => (&r.dummy_view, &r.sampler_clamp),
                    };
                    device.create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some("pbr textures"),
                        layout: &r.pbr_bgl1,
                        entries: &[
                            wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(view) },
                            wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(smp) },
                            wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(&r.shadow_map_view) },
                            wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::TextureView(&r.env_view) },
                            wgpu::BindGroupEntry { binding: 4, resource: wgpu::BindingResource::Sampler(&r.sampler_clamp) },
                        ],
                    })
            };
            bg
        };
        let pbr_bg = |r: &Renderer, draw_ub: &wgpu::Buffer| -> wgpu::BindGroup {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("pbr uniforms bind group"),
                layout: &r.pbr_bgl0,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: frame_ub.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 1, resource: draw_ub.as_entire_binding() },
                ],
            })
        };
        let depth_ub = |mvp: &Mat4, model: &Mat4| -> wgpu::Buffer { make_ub(&depth_bytes(mvp, model), "depth uniforms") };
        let depth_bg = |r: &Renderer, ub: &wgpu::Buffer| -> wgpu::BindGroup {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("depth bind group"),
                layout: &r.bgl_unif,
                entries: &[wgpu::BindGroupEntry { binding: 0, resource: ub.as_entire_binding() }],
            })
        };
        let draw_bytes = |model: &Mat4, rough: f32, metal: f32, receive: bool, has_map: f32, emissive: [f32; 3], nrm: [f32; 3]| -> Vec<u8> {
            let mut b = Vec::with_capacity(176);
            b.extend_from_slice(&mb(model));
            b.extend_from_slice(&mb(&math::normal_matrix(model)));
            for v in [
                [rough, metal, if receive { 1.0 } else { 0.0 }, has_map],
                [emissive[0], emissive[1], emissive[2], 0.0],
                [nrm[0], nrm[1], nrm[2], 0.0],
            ] {
                for x in v {
                    b.extend_from_slice(&x.to_ne_bytes());
                }
            }
            b
        };
        // ---- prepare std draws ----
        let mut std_p: Vec<Prepared> = Vec::new();
        for d in &spec.std {
            let (vb, ib, n) = make_vb(&d.geo.verts, &d.geo.idx);
            let draw_ub = make_ub(&draw_bytes(&d.model, d.mat.rough, d.mat.metal, d.receive, 0.0, d.mat.emissive, [0.0, 1.0, 0.0]), "pbr draw uniforms");
            let bg0 = pbr_bg(self, &draw_ub);
            let pre_ub = depth_ub(&math::mul(&vp, &d.model), &d.model);
            let prepass_bg = depth_bg(self, &pre_ub);
            let mut ubs = vec![draw_ub, pre_ub];
            let shadow_bg = if d.cast {
                let sh_ub = depth_ub(&math::mul(&shadow_vp, &d.model), &d.model);
                let bg = depth_bg(self, &sh_ub);
                ubs.push(sh_ub);
                Some(bg)
            } else {
                None
            };
            std_p.push(Prepared { vb, ib, n, bg0: Some(bg0), bg: None, prepass_bg: Some(prepass_bg), shadow_bg, _ubs: ubs });
        }
        // ---- prepare mapped draws ----
        let mut map_p: Vec<Prepared> = Vec::new();
        for (i, d) in spec.mapped.iter().enumerate() {
            let (vb, ib, n) = make_vb(&d.geo.verts, &d.geo.idx);
            let draw_ub = make_ub(&draw_bytes(&d.model, d.mat.rough, d.mat.metal, d.receive, 1.0, d.mat.emissive, [0.0, 1.0, 0.0]), "mapped draw uniforms");
            let bg0 = pbr_bg(self, &draw_ub);
            let pre_ub = depth_ub(&math::mul(&vp, &d.model), &d.model);
            let prepass_bg = depth_bg(self, &pre_ub);
            let mut ubs = vec![draw_ub, pre_ub];
            let shadow_bg = if d.cast {
                let sh_ub = depth_ub(&math::mul(&shadow_vp, &d.model), &d.model);
                let bg = depth_bg(self, &sh_ub);
                ubs.push(sh_ub);
                Some(bg)
            } else {
                None
            };
            let _ = i;
            map_p.push(Prepared { vb, ib, n, bg0: Some(bg0), bg: None, prepass_bg: Some(prepass_bg), shadow_bg, _ubs: ubs });
        }
        // ---- prepare texmapped draws ----
        let mut tex_p: Vec<Prepared> = Vec::new();
        for d in &spec.texmapped {
            let (vb, ib, n) = make_vb(&d.geo.verts, &d.geo.idx);
            let draw_ub = make_ub(&draw_bytes(&d.model, d.rough, 0.0, d.receive, 1.0, [0.0; 3], d.nrm), "texmapped draw uniforms");
            let bg0 = pbr_bg(self, &draw_ub);
            let pre_ub = depth_ub(&math::mul(&vp, &d.model), &d.model);
            let prepass_bg = depth_bg(self, &pre_ub);
            tex_p.push(Prepared { vb, ib, n, bg0: Some(bg0), bg: None, prepass_bg: Some(prepass_bg), shadow_bg: None, _ubs: vec![draw_ub, pre_ub] });
        }
        // ---- prepare basics ----
        let mut dyn_keep: Vec<wgpu::Texture> = Vec::new();
        let mut dyn_views: Vec<wgpu::TextureView> = Vec::new();
        for p in dyn_tex {
            let t = self.upload_dyn(&p.0, p.1, p.2);
            dyn_views.push(t.create_view(&wgpu::TextureViewDescriptor::default()));
            dyn_keep.push(t);
        }
        let (fog_col, fog_near, fog_far, fog_on) = match &spec.fog {
            Some((c, n, fa)) => (*c, *n, *fa, 1.0f32),
            None => ([0.0; 3], 0.0, 0.0, 0.0),
        };
        let mut basic_p: Vec<Prepared> = Vec::new();
        let mut card_shadow_bgs: Vec<Option<wgpu::BindGroup>> = Vec::new();
        for d in &spec.basics {
            let (vb, ib, n) = make_vb(&d.geo.verts, &d.geo.idx);
            let mut bytes: Vec<u8> = Vec::with_capacity(192);
            bytes.extend_from_slice(&mb(&math::mul(&vp, &d.model)));
            for v in [
                [d.tint[0], d.tint[1], d.tint[2], d.opacity],
                [d.alpha_test, if d.fog { 1.0 } else { 0.0 }, 0.0, 0.0],
            ] {
                for x in v {
                    bytes.extend_from_slice(&x.to_ne_bytes());
                }
            }
            bytes.extend_from_slice(&mb(&math::mul(&view, &d.model)));
            for v in [[fog_col[0], fog_col[1], fog_col[2], fog_near], [fog_far, fog_on, 0.0, 0.0]] {
                for x in v {
                    bytes.extend_from_slice(&x.to_ne_bytes());
                }
            }
            let ub = make_ub(&bytes, "basic uniforms");
            let tex_view = match &d.tex {
                TexSrc::S(i) => &self.tex_views[*i],
                TexSrc::Dyn(i) => &dyn_views[*i],
            };
            let sampler = if d.repeat { &self.sampler_repeat } else { &self.sampler_clamp };
            let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("basic bind group"),
                layout: &self.bgl_basic,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: ub.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(tex_view) },
                    wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(sampler) },
                ],
            });
            let mut ubs = vec![ub];
            let prepass_bg = if d.prepass {
                let pre_ub = depth_ub(&math::mul(&vp, &d.model), &d.model);
                let b = depth_bg(self, &pre_ub);
                ubs.push(pre_ub);
                Some(b)
            } else {
                None
            };
            // Card shadow (customDepthMaterial pattern, alphaTest 0.5)
            let card_shadow = if matches!(d.tex, TexSrc::Dyn(_)) {
                let sh_ub = make_ub(&depth_bytes(&math::mul(&shadow_vp, &d.model), &d.model), "card shadow uniforms");
                let b = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("card shadow bind group"),
                    layout: &self.bgl_shadow_card,
                    entries: &[
                        wgpu::BindGroupEntry { binding: 0, resource: sh_ub.as_entire_binding() },
                        wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(tex_view) },
                        wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&self.sampler_clamp) },
                    ],
                });
                ubs.push(sh_ub);
                Some(b)
            } else {
                None
            };
            card_shadow_bgs.push(card_shadow);
            basic_p.push(Prepared { vb, ib, n, bg0: None, bg: Some(bg), prepass_bg, shadow_bg: None, _ubs: ubs });
        }
        // Bokeh bind group (per-frame focus).
        let bokeh_ub = make_ub(
            &pack_f32(&[spec.focus, aspect, spec.post.aperture, spec.post.maxblur, 0.05, 800.0, 0.0, 0.0]),
            "bokeh uniforms",
        );
        let bokeh_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bokeh bind group"),
            layout: &self.bokeh_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: bokeh_ub.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&self.hdr_view) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&self.sampler_clamp) },
                wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::TextureView(&self.packed_view) },
                wgpu::BindGroupEntry { binding: 4, resource: wgpu::BindingResource::Sampler(&self.sampler_nearest) },
            ],
        });
        let bg1_std = bg1(self, None, false);
        // ---- passes ----
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("film frame encoder") });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("shadow pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.shadow_map_view,
                    resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color { r: 1.0, g: 1.0, b: 1.0, a: 1.0 }), store: wgpu::StoreOp::Store },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.shadow_depth_view,
                    depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(1.0), store: wgpu::StoreOp::Store }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(&self.shadow_geo);
            for p in std_p.iter().filter(|p| p.shadow_bg.is_some()) {
                pass.set_bind_group(0, p.shadow_bg.as_ref().unwrap(), &[]);
                pass.set_vertex_buffer(0, p.vb.slice(..));
                pass.set_index_buffer(p.ib.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..p.n, 0, 0..1);
            }
            pass.set_pipeline(&self.shadow_map44);
            for p in map_p.iter().filter(|p| p.shadow_bg.is_some()) {
                pass.set_bind_group(0, p.shadow_bg.as_ref().unwrap(), &[]);
                pass.set_vertex_buffer(0, p.vb.slice(..));
                pass.set_index_buffer(p.ib.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..p.n, 0, 0..1);
            }
            pass.set_pipeline(&self.shadow_card);
            for (p, cs) in basic_p.iter().zip(card_shadow_bgs.iter()) {
                if let Some(b) = cs {
                    pass.set_bind_group(0, b, &[]);
                    pass.set_vertex_buffer(0, p.vb.slice(..));
                    pass.set_index_buffer(p.ib.slice(..), wgpu::IndexFormat::Uint32);
                    pass.draw_indexed(0..p.n, 0, 0..1);
                }
            }
        }
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("depth prepass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.packed_view,
                    resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color { r: 1.0, g: 1.0, b: 1.0, a: 1.0 }), store: wgpu::StoreOp::Store },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.prepass_depth_view,
                    depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(1.0), store: wgpu::StoreOp::Store }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(&self.prepass_geo);
            for p in &std_p {
                pass.set_bind_group(0, p.prepass_bg.as_ref().unwrap(), &[]);
                pass.set_vertex_buffer(0, p.vb.slice(..));
                pass.set_index_buffer(p.ib.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..p.n, 0, 0..1);
            }
            pass.set_pipeline(&self.prepass_map);
            for p in &map_p {
                pass.set_bind_group(0, p.prepass_bg.as_ref().unwrap(), &[]);
                pass.set_vertex_buffer(0, p.vb.slice(..));
                pass.set_index_buffer(p.ib.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..p.n, 0, 0..1);
            }
            pass.set_pipeline(&self.prepass_tex);
            for p in tex_p.iter().chain(basic_p.iter().filter(|p| p.prepass_bg.is_some())) {
                pass.set_bind_group(0, p.prepass_bg.as_ref().unwrap(), &[]);
                pass.set_vertex_buffer(0, p.vb.slice(..));
                pass.set_index_buffer(p.ib.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..p.n, 0, 0..1);
            }
        }
        let mut bg1_map: HashMap<(Option<usize>, bool), wgpu::BindGroup> = HashMap::new();
        for d in &spec.texmapped {
            bg1_map.entry((Some(d.tex), d.repeat)).or_insert_with(|| bg1(self, Some(d.tex), d.repeat));
        }
        for (i, _) in spec.mapped.iter().enumerate() {
            bg1_map.entry((Some(spec.map_tex[i]), true)).or_insert_with(|| bg1(self, Some(spec.map_tex[i]), true));
        }
        {
            let clear = wgpu::Color { r: spec.clear[0] as f64, g: spec.clear[1] as f64, b: spec.clear[2] as f64, a: 1.0 };
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("film scene pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.hdr_msaa_view,
                    resolve_target: Some(&self.hdr_view),
                    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(clear), store: wgpu::StoreOp::Store },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.hdr_depth_view,
                    depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(1.0), store: wgpu::StoreOp::Store }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(&self.pbr_tex_pipeline);
            pass.set_bind_group(1, &bg1_std, &[]);
            for (p, d) in tex_p.iter().zip(spec.texmapped.iter()) {
                let bg = &bg1_map[&(Some(d.tex), d.repeat)];
                pass.set_bind_group(0, p.bg0.as_ref().unwrap(), &[]);
                pass.set_bind_group(1, bg, &[]);
                pass.set_vertex_buffer(0, p.vb.slice(..));
                pass.set_index_buffer(p.ib.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..p.n, 0, 0..1);
            }
            pass.set_pipeline(&self.pbr_pipeline);
            pass.set_bind_group(1, &bg1_std, &[]);
            for p in &std_p {
                pass.set_bind_group(0, p.bg0.as_ref().unwrap(), &[]);
                pass.set_vertex_buffer(0, p.vb.slice(..));
                pass.set_index_buffer(p.ib.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..p.n, 0, 0..1);
            }
            pass.set_pipeline(&self.pbr_map_pipeline);
            for (i, p) in map_p.iter().enumerate() {
                let bg = &bg1_map[&(Some(spec.map_tex[i]), true)];
                pass.set_bind_group(0, p.bg0.as_ref().unwrap(), &[]);
                pass.set_bind_group(1, bg, &[]);
                pass.set_vertex_buffer(0, p.vb.slice(..));
                pass.set_index_buffer(p.ib.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..p.n, 0, 0..1);
            }
            for (p, d) in basic_p.iter().zip(spec.basics.iter()) {
                if d.depth_write {
                    pass.set_pipeline(&self.basic_write);
                } else {
                    pass.set_pipeline(&self.basic_nowrite);
                }
                pass.set_bind_group(0, p.bg.as_ref().unwrap(), &[]);
                pass.set_vertex_buffer(0, p.vb.slice(..));
                pass.set_index_buffer(p.ib.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..p.n, 0, 0..1);
            }
        }
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("bokeh pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.bokeh_view,
                    resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT), store: wgpu::StoreOp::Store },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(&self.bokeh_pipeline);
            pass.set_bind_group(0, &bokeh_bg, &[]);
            pass.draw(0..3, 0..1);
        }
        self.post.bloom(
            device,
            queue,
            &mut encoder,
            &self.bokeh_view,
            &self.bokeh_view,
            width,
            height,
            &BloomParams { threshold: 0.85, strength: spec.post.bloom, radius: 0.6 },
        );
        self.post.output(device, queue, &mut encoder, &self.bokeh_view, &self.srgb_view, spec.post.exposure);
        self.post.grade(
            device,
            queue,
            &mut encoder,
            &self.srgb_view,
            &self.final_view,
            &GradeParams { vignette: spec.post.vig, grain: spec.post.grain, warm: spec.post.warm, sat: spec.post.sat, time: 0.0 },
        );
        // ---- readback ----
        let padded = math::padded_row_bytes(width);
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("readback buffer"),
            size: (padded * height) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        encoder.copy_texture_to_buffer(
            wgpu::ImageCopyTexture { texture: &self.final_tex, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
            wgpu::ImageCopyBuffer {
                buffer: &readback,
                layout: wgpu::ImageDataLayout { offset: 0, bytes_per_row: Some(padded), rows_per_image: Some(height) },
            },
            wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
        );
        queue.submit(Some(encoder.finish()));
        let slice = readback.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = tx.send(result);
        });
        device.poll(wgpu::Maintain::Wait);
        rx.recv().expect("map channel closed").expect("buffer map failed");
        let data = slice.get_mapped_range();
        let row = (width * 4) as usize;
        let stride = padded as usize;
        let mut pixels = Vec::with_capacity(row * height as usize);
        for y in 0..height as usize {
            pixels.extend_from_slice(&data[y * stride..y * stride + row]);
        }
        drop(data);
        readback.unmap();
        drop(dyn_keep);
        pixels
    }
}

// ---------------------------------------------------------------------
// CPU overlays (main.ts order: calendar inside the shot draw, then
// the film fade, then the caption — all in sRGB space, the same
// overlay point ab1 used for its caption).
// ---------------------------------------------------------------------

fn sample_bilinear(img: &(Vec<u8>, u32, u32), x: f32, y: f32) -> (f32, f32, f32, f32) {
    let (px, w, h) = (&img.0, img.1 as f32, img.2 as f32);
    let x = x.clamp(0.0, w - 1.001);
    let y = y.clamp(0.0, h - 1.001);
    let (x0, y0) = (x.floor() as usize, y.floor() as usize);
    let (fx, fy) = (x - x0 as f32, y - y0 as f32);
    let get = |xx: usize, yy: usize| -> (f32, f32, f32, f32) {
        let k = (yy * img.1 as usize + xx) * 4;
        (px[k] as f32, px[k + 1] as f32, px[k + 2] as f32, px[k + 3] as f32)
    };
    let (a, b, c, d) = (get(x0, y0), get(x0 + 1, y0), get(x0, y0 + 1), get(x0 + 1, y0 + 1));
    let mixv = |i: usize, a: (f32, f32, f32, f32), b: (f32, f32, f32, f32), c: (f32, f32, f32, f32), d: (f32, f32, f32, f32), fx: f32, fy: f32| -> f32 {
        let top = [a.0, a.1, a.2, a.3][i] + ([b.0, b.1, b.2, b.3][i] - [a.0, a.1, a.2, a.3][i]) * fx;
        let bot = [c.0, c.1, c.2, c.3][i] + ([d.0, d.1, d.2, d.3][i] - [c.0, c.1, c.2, c.3][i]) * fx;
        top + (bot - top) * fy
    };
    (
        mixv(0, a, b, c, d, fx, fy),
        mixv(1, a, b, c, d, fx, fy),
        mixv(2, a, b, c, d, fx, fy),
        mixv(3, a, b, c, d, fx, fy),
    )
}

fn blend_over(px: &mut [u8], w: u32, x: u32, y: u32, src: (f32, f32, f32, f32)) {
    let a = (src.3 / 255.0).clamp(0.0, 1.0);
    if a <= 0.0 {
        return;
    }
    let k = ((y as u32 * w + x as u32) as usize) * 4;
    let rgb = [src.0, src.1, src.2];
    for i in 0..3 {
        px[k + i] = (rgb[i] * a + px[k + i] as f32 * (1.0 - a)).round() as u8;
    }
}

fn overlay_calendar(px: &mut [u8], w: u32, h: u32, assets: &Assets, cal: &CalendarState) {
    // drawCalendar transform: translate(352, 648) rotate(-0.03)
    // scale(0.92, 0.92*sy); plate local frame x [-180,180],
    // y [-220,220] at 2 px per unit.
    let (cx, cy) = (352.0f32, 648.0f32);
    let (rot, sx, sy) = (-0.03f32, 0.92f32, 0.92 * cal.sy);
    let (sr, cr) = rot.sin_cos();
    let fwd = |lx: f32, ly: f32| -> (f32, f32) {
        let (x, y) = (lx * sx, ly * sy);
        (cx + x * cr - y * sr, cy + x * sr + y * cr)
    };
    let corners = [fwd(-180.0, -220.0), fwd(180.0, -220.0), fwd(-180.0, 220.0), fwd(180.0, 220.0)];
    let (mut x0, mut y0, mut x1, mut y1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    for &(x, y) in &corners {
        x0 = x0.min(x);
        y0 = y0.min(y);
        x1 = x1.max(x);
        y1 = y1.max(y);
    }
    let (ix0, iy0) = ((x0.floor() as i32).max(0), (y0.floor() as i32).max(0));
    let (ix1, iy1) = ((x1.ceil() as i32).min(w as i32 - 1), (y1.ceil() as i32).min(h as i32 - 1));
    let plate = &assets.cal[cal.year];
    for y in iy0..=iy1 {
        for x in ix0..=ix1 {
            // inverse: un-translate, un-rotate, un-scale
            let (dx, dy) = (x as f32 - cx, y as f32 - cy);
            let (rx, ry) = (dx * cr + dy * sr, -dx * sr + dy * cr);
            let (lx, ly) = (rx / sx, ry / sy);
            if !(-180.0..=180.0).contains(&lx) || !(-220.0..=220.0).contains(&ly) {
                continue;
            }
            let (pu, pv) = ((lx + 180.0) * 2.0, (ly + 220.0) * 2.0);
            let mut src = sample_bilinear(plate, pu, pv);
            src.3 *= cal.alpha;
            blend_over(px, w, x as u32, y as u32, src);
            if cal.dark > 0.0 {
                let dsrc = sample_bilinear(&assets.cal_dark, pu, pv);
                let a = dsrc.3 * cal.dark * 0.22 * cal.alpha;
                if a > 0.0 {
                    let k = ((y as u32 * w + x as u32) as usize) * 4;
                    for i in 0..3 {
                        let dc = [30.0, 25.0, 18.0][i];
                        px[k + i] = (dc * (a / 255.0) + px[k + i] as f32 * (1.0 - a / 255.0)).round() as u8;
                    }
                }
            }
        }
    }
}

fn overlay_caption(px: &mut [u8], w: u32, h: u32, assets: &Assets, idx: usize, scale: f32) {
    let strip = &assets.strips[idx];
    let (cx, cy) = (540.0f32, 1340.0f32);
    let (hw, hh) = (540.0 * scale, 150.0 * scale);
    let (ix0, iy0) = (((cx - hw).floor() as i32).max(0), ((cy - hh).floor() as i32).max(0));
    let (ix1, iy1) = (((cx + hw).ceil() as i32).min(w as i32 - 1), ((cy + hh).ceil() as i32).min(h as i32 - 1));
    for y in iy0..=iy1 {
        for x in ix0..=ix1 {
            let su = cx + (x as f32 - cx) / scale;
            let sv = cy + (y as f32 - cy) / scale;
            let src_x = su;
            let src_y = sv - (1340.0 - 150.0);
            if src_x < 0.0 || src_x >= strip.1 as f32 || src_y < 0.0 || src_y >= strip.2 as f32 {
                continue;
            }
            let src = sample_bilinear(strip, src_x, src_y);
            blend_over(px, w, x as u32, y as u32, src);
        }
    }
}

fn apply_fade(px: &mut [u8], fade: f32) {
    let keep = 1.0 - fade;
    for chunk in px.chunks_exact_mut(4) {
        for i in 0..3 {
            chunk[i] = (chunk[i] as f32 * keep).round() as u8;
        }
    }
}

// ---------------------------------------------------------------------
// The film driver.
// ---------------------------------------------------------------------

pub(crate) async fn run_film(width: u32, height: u32, film_dir: &str, out_dir: &str, from: u32, to: u32) {
    let dir = Path::new(film_dir);
    let data = load_film_data(dir).unwrap_or_else(|e| panic!("film data: {e}"));
    let assets = load_assets(dir, &data).unwrap_or_else(|e| panic!("film assets: {e}"));
    let looks = look_registry();
    let frames_total = (data.duration * 30.0).ceil() as u32 + 1;
    let to = if to == 0 { frames_total } else { to.min(frames_total) };
    let mut renderer = Renderer::new(width, height, &assets.sign).await;
    std::fs::create_dir_all(out_dir).expect("cannot create frames dir");
    println!(
        "film: {} — {} frames @30fps, rendering {}..{} at {}x{} (clean PNG frames; decision B)",
        "THE SOLDIER WHO DIDN'T KNOW THE WAR WAS OVER", frames_total, from, to, width, height
    );
    let mut cache: PaintCache = HashMap::new();
    let (mut done, mut skipped) = (0u32, 0u32);
    for n in from..to {
        let path = format!("{out_dir}/frame_{n:05}.png");
        if Path::new(&path).exists() {
            skipped += 1;
            continue;
        }
        let t = n as f64 / 30.0;
        let mut spec = eval_frame(&data, &looks, t);
        let (dyn_tex, _) = resolve_chars(&mut spec, &looks, &assets, &mut cache);
        let mut px = renderer.render(&spec, &dyn_tex);
        if let Some(cal) = &spec.calendar {
            overlay_calendar(&mut px, width, height, &assets, cal);
        }
        if spec.fade > 0.0 {
            apply_fade(&mut px, spec.fade);
        }
        if let Some(ci) = spec.caption {
            overlay_caption(&mut px, width, height, &assets, ci, spec.caption_scale);
        }
        let img = image::RgbaImage::from_raw(width, height, px).expect("pixel buffer size mismatch");
        img.save(&path).unwrap_or_else(|e| panic!("cannot save {path}: {e}"));
        done += 1;
        if done % 25 == 0 {
            println!("  frame {n} (t = {t:.2}s) — {done} rendered, {skipped} skipped");
        }
    }
    println!("film render complete: {done} rendered, {skipped} skipped (resume), range {from}..{to}");
}

// ---------------------------------------------------------------------
// Film telemetry: the M6 audit semantics over the whole film —
// 0.25 s sampling of eval_frame, per-character cutouts from the real
// painted canvases + projected card boxes, the Audit's overflow /
// edge / continuity / composition derivations, and the strict
// caption audit. Caption widths are measured from the film's own
// rendered caption strips (ink bounding box; PIL metrics — the
// declared approximation of docs/integration/README.md).
// ---------------------------------------------------------------------

fn tel_look(l: &FullLook) -> crate::telemetry::Look {
    fn lk(s: &str) -> &'static str {
        Box::leak(s.to_string().into_boxed_str())
    }
    fn lko(s: &Option<String>) -> Option<&'static str> {
        s.as_ref().map(|v| lk(v))
    }
    crate::telemetry::Look {
        body: Some(lk(&l.body)),
        pattern: Some(lk(&l.pattern)),
        pat_col: lko(&l.pat_col),
        hat: Some(lk(&l.hat)),
        hat_col: lko(&l.hat_col),
        band: lko(&l.band),
        neck: Some(lk(&l.neck)),
        neck_col: lko(&l.neck_col),
        head: Some(lk(&l.head)),
        kid: l.kid,
        extra: Vec::new(),
    }
}

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

/// Registry instance -> cutout identity (the same person keeps one
/// id across sets; continuity = the look signature staying stable
/// under that id).
const INSTANCE_IDS: [&str; 11] = [
    "onoda", "compA", "compB", "suzuki", "tani", "onoda", "compA", "compB", "tani", "tani", "onoda",
];

pub(crate) fn run_film_telemetry(dir: &Path) -> Result<(), String> {
    use crate::telemetry::*;
    let data = load_film_data(dir)?;
    let looks = look_registry();
    let assets = load_assets(dir, &data)?;
    let tels: Vec<Look> = looks.iter().map(|(l, _, _)| tel_look(l)).collect();
    let l0 = |i: usize| data.lines[i].0 - 0.06;
    let shots = vec![
        ShotInfo { id: "S01".into(), at: l0(0) },
        ShotInfo { id: "S02".into(), at: l0(1) },
        ShotInfo { id: "S03".into(), at: l0(2) },
        ShotInfo { id: "S04".into(), at: l0(3) },
        ShotInfo { id: "S05a".into(), at: l0(4) },
        ShotInfo { id: "S05b".into(), at: data.anchors["thought4"] - 0.12 },
        ShotInfo { id: "S06".into(), at: l0(5) },
        ShotInfo { id: "S07".into(), at: data.anchors["decades6"] - 0.25 },
        ShotInfo { id: "S08".into(), at: l0(7) },
        ShotInfo { id: "S08b".into(), at: data.anchors["refused7"] - 0.1 },
        ShotInfo { id: "S09".into(), at: l0(8) },
        ShotInfo { id: "S10".into(), at: l0(9) },
        ShotInfo { id: "S11".into(), at: l0(10) },
        ShotInfo { id: "S12".into(), at: l0(11) },
    ];
    let mut audit = Audit::new(shots);
    let mut cache: PaintCache = HashMap::new();
    let mut t = 0.0f64;
    while t < data.duration {
        let mut spec = eval_frame(&data, &looks, t);
        let cam = ProjCamera {
            eye: [spec.eye[0] as f64, spec.eye[1] as f64, spec.eye[2] as f64],
            target: [spec.target[0] as f64, spec.target[1] as f64, spec.target[2] as f64],
            up: [0.0, 1.0, 0.0],
            fov_deg: spec.fov as f64,
            aspect: 1080.0 / 1920.0,
            near: 0.05,
            far: 800.0,
        };
        let (_, infos) = resolve_chars(&mut spec, &looks, &assets, &mut cache);
        let cuts: Vec<Cutout> = infos
            .iter()
            .map(|ci| {
                let tl = &tels[ci.look];
                Cutout {
                    id: INSTANCE_IDS[ci.look].to_string(),
                    desc: describe(tl),
                    pose: pose_summary(&PoseState {
                        mood: mood_name(ci.mood),
                        arm_l: (ci.arm_l.0 as f64, ci.arm_l.1 as f64),
                        arm_r: (ci.arm_r.0 as f64, ci.arm_r.1 as f64),
                        legs: legs_name(ci.legs),
                        head_tilt: ci.head_tilt as f64,
                        head_turn: ci.head_turn as f64,
                    }),
                    overflow: Some(scan_canvas_overflow(&ci.painted.0, ci.painted.1 as usize, ci.painted.2 as usize, 2.0)),
                    box_: Some(project_box(&model_from_f32(&ci.model), &cam, 1.75)),
                    sig: look_signature(tl),
                }
            })
            .collect();
        audit.sample(t, &cuts);
        t += 0.25;
    }
    // Caption audit: rendered width = ink bbox of the film's own
    // strip; raw width = rendered / fit.
    let mut caps: Vec<CaptionInfo> = Vec::new();
    for (i, c) in data.captions.iter().enumerate() {
        let strip = &assets.strips[i];
        let (mut x0, mut x1) = (i64::MAX, -1i64);
        for y in 0..strip.2 as i64 {
            for x in 0..strip.1 as i64 {
                let a = strip.0[((y * strip.1 as i64 + x) as usize) * 4 + 3];
                if a > 32 {
                    x0 = x0.min(x);
                    x1 = x1.max(x);
                }
            }
        }
        let rendered = if x1 >= 0 { (x1 - x0 + 1) as f64 } else { 0.0 };
        let line = data
            .lines
            .iter()
            .rposition(|&(s, _)| s <= c.s + 0.05)
            .unwrap_or(0) as i64;
        caps.push(CaptionInfo {
            line,
            s: c.s,
            e: c.e,
            text: c.text.clone(),
            auto: true,
            raw_w: if c.fit > 0.0 { rendered / c.fit } else { rendered },
            fit: c.fit,
            rendered_w: rendered,
        });
    }
    let mut out: Vec<String> = Vec::new();
    out.push("FILM TELEMETRY — THE SOLDIER WHO DIDN'T KNOW THE WAR WAS OVER (native, 0.25 s sampling)".to_string());
    audit.print_audit(false, &mut out);
    audit.print_continuity(&mut out);
    audit.print_composition(&mut out);
    let cap_fails = print_caption_audit(&caps, true, &mut out);
    let bad = audit.overflow_len() + audit.violations_len() + cap_fails;
    out.push(if bad == 0 {
        "FILM TELEMETRY: CHECK OK".to_string()
    } else {
        format!(
            "FILM TELEMETRY: CHECK FAILED ({} overflow groups, {} continuity violations, {} caption failures)",
            audit.overflow_len(),
            audit.violations_len(),
            cap_fails
        )
    });
    for line in &out {
        println!("{line}");
    }
    Ok(())
}
