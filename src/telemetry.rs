//! M6 — native QA telemetry: the `cutouts()` channel of the Chrome
//! capture gate, reimplemented natively (compatibility contract §3;
//! audit a2 §3.1–3.3). The gate is semantic telemetry, not pixels:
//! per character, per sample —
//!
//! - `overflow`: border-strip pixel counts of the character's own
//!   painted canvas, computed inside the painter
//!   (`scanCanvasOverflow`, toon.ts: alpha > 32 in strips of
//!   `max(1, round(1 * res))` px; corners counted in both strips).
//! - `box`: the toon plane's screen-space box — the plane geometry's
//!   bounding-box corners projected through the last-rendered camera
//!   (main.ts `OP.cutouts`), NDC -> px against 1080x1920, clamped
//!   only to ±10000, with three.js `behind` semantics (any corner
//!   view-space z > 0, NDC z > 1, or non-finite projection).
//! - `sig`: the look signature string (`lookSignature`, cast.ts) —
//!   the continuity fingerprint.
//!
//! The host side is a line-level transcription of the v5 template's
//! audit (template/tools/capture.ts: makeAudit / auditSample /
//! printAudit / printContinuity / printComposition /
//! printCaptionAudit + the check verdict) — same dedupe keys, same
//! fig/face stage fractions, same advisory thresholds, same output
//! line formats, same verdict expression:
//! `failed = bad>0 || logs>0 || overflow>0 || violations>0 ||
//! strict-caption fails>0`. Composition findings are advisory and
//! never enter the verdict, exactly as in capture.
//!
//! Projection arithmetic is three.js-faithful and done in f64 (JS
//! numbers): the box definition belongs to three's projection
//! (NDC z in [-1, 1]), not to the renderer's wgpu clip space. The
//! staging inputs are the native scene graph's own values (the card
//! draw's model matrix; the scene's camera), per the work order:
//! bounds come from the scene graph, not from pixels.

use std::collections::HashSet;

// ---------------------------------------------------------------------
// JS number formatting (the audit's printed lines embed JS
// Number->String and toFixed(2) results; these helpers mirror them
// for every value this module prints).
// ---------------------------------------------------------------------

/// JS `String(number)` for the magnitudes the audit prints: integral
/// values print without a fraction; -0 prints as "0"; anything else
/// uses the shortest round-trip decimal (Rust's Display for f64 is
/// also a shortest round-trip representation).
pub(crate) fn js_num(v: f64) -> String {
    if v == 0.0 {
        return "0".to_string();
    }
    if v.fract() == 0.0 && v.abs() < 1e15 {
        return format!("{}", v as i64);
    }
    format!("{v}")
}

/// JS `Math.round` (half toward +infinity).
pub(crate) fn js_round(v: f64) -> f64 {
    (v + 0.5).floor()
}

/// JS `toFixed(2)`.
fn fixed2(v: f64) -> String {
    format!("{v:.2}")
}

/// JSON.stringify escaping for the strings this module emits
/// (colours, names, signature values — plain ASCII in practice, but
/// the escaping rules are implemented, not assumed).
fn js_json_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0c}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

// ---------------------------------------------------------------------
// Page-side definitions (toon.ts / cast.ts / main.ts transcriptions)
// ---------------------------------------------------------------------

/// A character look, restricted to the fields the telemetry reads.
/// Field order in `look_signature` is cast.ts's, not this struct's.
#[derive(Clone, Default)]
pub(crate) struct Look {
    pub body: Option<&'static str>,
    pub pattern: Option<&'static str>,
    pub pat_col: Option<&'static str>,
    pub hat: Option<&'static str>,
    pub hat_col: Option<&'static str>,
    pub band: Option<&'static str>,
    pub neck: Option<&'static str>,
    pub neck_col: Option<&'static str>,
    pub head: Option<&'static str>,
    pub kid: bool,
    pub extra: Vec<&'static str>,
}

/// `lookSignature(look)` (cast.ts:34-48): JSON.stringify of the
/// identity fields in fixed key order, missing fields as null,
/// `extra` sorted (JS default string sort — byte order for the
/// ASCII feature names used by the cast).
pub(crate) fn look_signature(l: &Look) -> String {
    let opt = |v: Option<&str>| -> String {
        match v {
            Some(s) => js_json_string(s),
            None => "null".to_string(),
        }
    };
    let mut extra: Vec<&str> = l.extra.clone();
    extra.sort();
    let extra_json = extra
        .iter()
        .map(|s| js_json_string(s))
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{{\"body\":{},\"pattern\":{},\"patCol\":{},\"hat\":{},\"hatCol\":{},\"band\":{},\"neck\":{},\"neckCol\":{},\"head\":{},\"kid\":{},\"extra\":[{}]}}",
        opt(l.body),
        opt(l.pattern),
        opt(l.pat_col),
        opt(l.hat),
        opt(l.hat_col),
        opt(l.band),
        opt(l.neck),
        opt(l.neck_col),
        opt(l.head),
        if l.kid { "true" } else { "false" },
        extra_json
    )
}

/// `Toon#describe()` (toon.ts:614-617) — reads the raw look, with
/// drawToon's defaults for absent fields.
pub(crate) fn describe(l: &Look) -> String {
    format!(
        "hat={} body={} pattern={}",
        l.hat.unwrap_or("none"),
        l.body.unwrap_or("#E8734A"),
        l.pattern.unwrap_or("plain")
    )
}

/// The painter state fields `poseSummary()` reads (toon.ts:619-625).
#[derive(Clone)]
pub(crate) struct PoseState {
    pub mood: &'static str,
    pub arm_l: (f64, f64),
    pub arm_r: (f64, f64),
    pub legs: &'static str,
    pub head_tilt: f64,
    pub head_turn: f64,
}

/// `Toon#poseSummary()`: `mood=<m> armL=[x,y] armR=[x,y] legs=<l>`,
/// plus ` head=<tilt>/<turn>` iff either head field is non-zero.
/// Array formatting is JS `join(',')` = String(number) per element.
pub(crate) fn pose_summary(s: &PoseState) -> String {
    let base = format!(
        "mood={} armL=[{},{}] armR=[{},{}] legs={}",
        s.mood,
        js_num(s.arm_l.0),
        js_num(s.arm_l.1),
        js_num(s.arm_r.0),
        js_num(s.arm_r.1),
        s.legs
    );
    if s.head_tilt != 0.0 || s.head_turn != 0.0 {
        format!("{} head={}/{}", base, js_num(s.head_tilt), js_num(s.head_turn))
    } else {
        base
    }
}

#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub(crate) struct OverflowCounts {
    pub top: u64,
    pub right: u64,
    pub bottom: u64,
    pub left: u64,
}

/// `scanCanvasOverflow(canvas, res, strip = 1)` (toon.ts:147-166):
/// counts pixels with alpha > 32 in the four border strips of the
/// painted canvas. Strip thickness `s = max(1, round(strip * res))`
/// device px; top/bottom strips span the full width, left/right the
/// full height, so corners are counted in two strips — as in the
/// original's four independent getImageData scans.
pub(crate) fn scan_canvas_overflow(px: &[u8], w: usize, h: usize, res: f64) -> OverflowCounts {
    let s = ((1.0 * res).round() as usize).max(1);
    let scan = |x0: usize, y0: usize, sw: usize, sh: usize| -> u64 {
        let mut n = 0u64;
        for y in y0..(y0 + sh).min(h) {
            for x in x0..(x0 + sw).min(w) {
                if px[(y * w + x) * 4 + 3] > 32 {
                    n += 1;
                }
            }
        }
        n
    };
    let st = s.min(h);
    let sl = s.min(w);
    OverflowCounts {
        top: scan(0, 0, w, st),
        bottom: scan(0, h - st, w, st),
        left: scan(0, 0, sl, h),
        right: scan(w - sl, 0, sl, h),
    }
}

// ---------------------------------------------------------------------
// Box projection — three.js arithmetic in f64 (main.ts:94-113)
// ---------------------------------------------------------------------

type M4 = [[f64; 4]; 4]; // [column][row], as in src/math.rs

fn m4_mul_vec(m: &M4, v: [f64; 4]) -> [f64; 4] {
    let mut out = [0.0; 4];
    for row in 0..4 {
        out[row] = m[0][row] * v[0] + m[1][row] * v[1] + m[2][row] * v[2] + m[3][row] * v[3];
    }
    out
}

fn m4_mul(a: &M4, b: &M4) -> M4 {
    let mut out = [[0.0; 4]; 4];
    for col in 0..4 {
        for row in 0..4 {
            out[col][row] = a[0][row] * b[col][0]
                + a[1][row] * b[col][1]
                + a[2][row] * b[col][2]
                + a[3][row] * b[col][3];
        }
    }
    out
}

fn m4_translate(x: f64, y: f64, z: f64) -> M4 {
    let mut m = [[0.0; 4]; 4];
    for i in 0..4 {
        m[i][i] = 1.0;
    }
    m[3] = [x, y, z, 1.0];
    m
}

fn m4_rot_y(rad: f64) -> M4 {
    let (s, c) = rad.sin_cos();
    let mut m = m4_translate(0.0, 0.0, 0.0);
    m[0] = [c, 0.0, -s, 0.0];
    m[2] = [s, 0.0, c, 0.0];
    m
}

fn m4_rot_x(rad: f64) -> M4 {
    let (s, c) = rad.sin_cos();
    let mut m = m4_translate(0.0, 0.0, 0.0);
    m[1] = [0.0, c, s, 0.0];
    m[2] = [0.0, -s, c, 0.0];
    m
}

/// `Toon#face()` (toon.ts:632-652) for a group with no rotation of
/// its own: pivot yaw = atan2(cam.x - p.x, cam.z - p.z); perspective
/// pitch = clamp(pitchAng * 0.45, -0.38, 0.38) about the same pivot
/// (Euler order YXZ, so the model matrix is T * RotY * RotX).
/// Returns (yaw, pitch). `h` is the toon's H (height / 0.96).
pub(crate) fn face_angles(group_pos: [f64; 3], cam_pos: [f64; 3], h: f64, perspective: bool) -> (f64, f64) {
    let yaw = (cam_pos[0] - group_pos[0]).atan2(cam_pos[2] - group_pos[2]);
    if !perspective {
        return (yaw, 0.0);
    }
    let dy = cam_pos[1] - (group_pos[1] + h * 0.6);
    let dh = (cam_pos[0] - group_pos[0]).hypot(cam_pos[2] - group_pos[2]);
    let pitch_ang = if dh < 1e-4 { 0.0 } else { dy.atan2(dh) };
    (yaw, (pitch_ang * 0.45).clamp(-0.38, 0.38))
}

/// `camPitchOf(toon, camPos)` (toon.ts:720-725): the drawing-side
/// companion of face()'s pitch — clamp(pitchAng / 0.9, -1, 1).
pub(crate) fn cam_pitch_of(group_pos: [f64; 3], cam_pos: [f64; 3], h: f64) -> f64 {
    let dy = cam_pos[1] - (group_pos[1] + h * 0.6);
    let dh = (cam_pos[0] - group_pos[0]).hypot(cam_pos[2] - group_pos[2]);
    let pitch_ang = if dh < 1e-4 { 0.0 } else { dy.atan2(dh) };
    (pitch_ang / 0.9).clamp(-1.0, 1.0)
}

/// Model matrix for a toon group at `group_pos` faced per face():
/// T(group) * RotY(yaw) * RotX(pitch) — three.js group (position
/// only) -> pivot (Euler YXZ) -> mesh (identity).
pub(crate) fn toon_model(group_pos: [f64; 3], yaw: f64, pitch: f64) -> M4 {
    m4_mul(
        &m4_translate(group_pos[0], group_pos[1], group_pos[2]),
        &m4_mul(&m4_rot_y(yaw), &m4_rot_x(pitch)),
    )
}

/// Convert the renderer's f32 model matrix (src/math.rs layout).
pub(crate) fn model_from_f32(m: &crate::math::Mat4) -> M4 {
    let mut out = [[0.0; 4]; 4];
    for col in 0..4 {
        for row in 0..4 {
            out[col][row] = m[col][row] as f64;
        }
    }
    out
}

/// The toon plane's local bounding box (toon.ts:540-549): the plane
/// spans the full 1280x1408 canvas at H/1024 per unit, translated so
/// its centre sits 552 units above the feet. Returns
/// (min_x, min_y, max_x, max_y); z is 0 everywhere (bb.min.z = 0).
pub(crate) fn plane_bbox(height: f64) -> (f64, f64, f64, f64) {
    let h = height / 0.96;
    let plane_w = h * 1280.0 / 1024.0;
    let plane_h = h * 1408.0 / 1024.0;
    let centre_y = h * 552.0 / 1024.0;
    (-plane_w / 2.0, centre_y - plane_h / 2.0, plane_w / 2.0, centre_y + plane_h / 2.0)
}

/// A three.js-style camera for the projection: eye/target/up plus
/// PerspectiveCamera(fov, aspect, near, far) parameters
/// (engine.ts `camera()`: near 0.05, far 800).
#[derive(Clone, Copy)]
pub(crate) struct ProjCamera {
    pub eye: [f64; 3],
    pub target: [f64; 3],
    pub up: [f64; 3],
    pub fov_deg: f64,
    pub aspect: f64,
    pub near: f64,
    pub far: f64,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct ScreenBox {
    pub x0: i64,
    pub y0: i64,
    pub x1: i64,
    pub y1: i64,
    pub behind: bool,
}

/// The `box` computation of OP.cutouts (main.ts:86-113), transcribed
/// operation for operation: corners (min.x, min.y), (min.x, max.y),
/// (max.x, min.y), (max.x, max.y) at z = bb.min.z, transformed by
/// the mesh's world matrix; per corner the view-space z through the
/// camera's inverse world matrix; three.js `Vector3.project`
/// (perspective divide included); non-finite components replaced
/// (x->0, y->0, z->2) and flagged behind; NDC z > 1 flagged behind;
/// px = ((ndc.x + 1) / 2 * 1080, (1 - ndc.y) / 2 * 1920), clamped
/// to ±10000; the box is the rounded min/max.
pub(crate) fn project_box(model: &M4, cam: &ProjCamera, height: f64) -> ScreenBox {
    let (min_x, min_y, max_x, max_y) = plane_bbox(height);
    let corners = [
        [min_x, min_y],
        [min_x, max_y],
        [max_x, min_y],
        [max_x, max_y],
    ];
    // Camera basis (three.js Matrix4.lookAt): the camera looks down
    // its -Z, so zaxis = normalize(eye - target).
    let sub = |a: [f64; 3], b: [f64; 3]| [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
    let cross = |a: [f64; 3], b: [f64; 3]| {
        [
            a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0],
        ]
    };
    let norm = |a: [f64; 3]| {
        let l = (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt();
        [a[0] / l, a[1] / l, a[2] / l]
    };
    let zaxis = norm(sub(cam.eye, cam.target));
    let xaxis = norm(cross(cam.up, zaxis));
    let yaxis = cross(zaxis, xaxis);
    // PerspectiveCamera.updateProjectionMatrix (symmetric frustum).
    let f = 1.0 / (0.5 * cam.fov_deg * std::f64::consts::PI / 180.0).tan();
    let (near, far) = (cam.near, cam.far);
    let pz = (far + near) / (near - far);
    let pw = 2.0 * far * near / (near - far);
    let clamp_px = |n: f64| n.max(-10000.0).min(10000.0);
    let mut behind = false;
    let mut xs: Vec<f64> = Vec::with_capacity(4);
    let mut ys: Vec<f64> = Vec::with_capacity(4);
    for [cx, cy] in corners {
        let world = m4_mul_vec(model, [cx, cy, 0.0, 1.0]);
        // View space via cam.matrixWorldInverse (rigid inverse:
        // transpose of the lookAt basis, eye as origin).
        let d = sub([world[0], world[1], world[2]], cam.eye);
        let view = [
            xaxis[0] * d[0] + xaxis[1] * d[1] + xaxis[2] * d[2],
            yaxis[0] * d[0] + yaxis[1] * d[1] + yaxis[2] * d[2],
            zaxis[0] * d[0] + zaxis[1] * d[1] + zaxis[2] * d[2],
        ];
        if view[2] > 0.0 {
            behind = true;
        }
        // Vector3.project: clip = P * view, then divide by clip.w.
        let clip = [
            (f / cam.aspect) * view[0],
            f * view[1],
            pz * view[2] + pw,
            -view[2],
        ];
        let w = clip[3];
        let mut ndc = [clip[0] / w, clip[1] / w, clip[2] / w];
        if !ndc[0].is_finite() {
            behind = true;
            ndc[0] = 0.0;
        }
        if !ndc[1].is_finite() {
            behind = true;
            ndc[1] = 0.0;
        }
        if !ndc[2].is_finite() {
            behind = true;
            ndc[2] = 2.0;
        }
        if ndc[2] > 1.0 {
            behind = true;
        }
        xs.push(clamp_px((ndc[0] + 1.0) / 2.0 * 1080.0));
        ys.push(clamp_px((1.0 - ndc[1]) / 2.0 * 1920.0));
    }
    let (mut x0, mut x1) = (f64::INFINITY, f64::NEG_INFINITY);
    let (mut y0, mut y1) = (f64::INFINITY, f64::NEG_INFINITY);
    for v in &xs {
        x0 = x0.min(*v);
        x1 = x1.max(*v);
    }
    for v in &ys {
        y0 = y0.min(*v);
        y1 = y1.max(*v);
    }
    ScreenBox {
        x0: js_round(x0) as i64,
        y0: js_round(y0) as i64,
        x1: js_round(x1) as i64,
        y1: js_round(y1) as i64,
        behind,
    }
}

// ---------------------------------------------------------------------
// Cutouts (the OP.cutouts() payload shape)
// ---------------------------------------------------------------------

#[derive(Clone)]
pub(crate) struct Cutout {
    pub id: String,
    pub desc: String,
    pub pose: String,
    pub overflow: Option<OverflowCounts>,
    pub box_: Option<ScreenBox>,
    pub sig: String,
}

impl Cutout {
    /// The OP.cutouts() JSON shape (field order as main.ts builds
    /// it: id, desc, pose, overflow, box, sig; `behind` only when
    /// true, as the original sets the property conditionally).
    pub(crate) fn to_json(&self) -> String {
        let ov = match &self.overflow {
            Some(o) => format!(
                "{{\"top\":{},\"right\":{},\"bottom\":{},\"left\":{}}}",
                o.top, o.right, o.bottom, o.left
            ),
            None => "null".to_string(),
        };
        let bx = match &self.box_ {
            Some(b) => {
                let behind = if b.behind { ",\"behind\":true" } else { "" };
                format!(
                    "{{\"x0\":{},\"y0\":{},\"x1\":{},\"y1\":{}{}}}",
                    b.x0, b.y0, b.x1, b.y1, behind
                )
            }
            None => "null".to_string(),
        };
        format!(
            "{{\"id\":{},\"desc\":{},\"pose\":{},\"overflow\":{},\"box\":{},\"sig\":{}}}",
            js_json_string(&self.id),
            js_json_string(&self.desc),
            js_json_string(&self.pose),
            ov,
            bx,
            js_json_string(&self.sig)
        )
    }
}

// ---------------------------------------------------------------------
// Host audit — capture.ts (v5 template) transcribed
// ---------------------------------------------------------------------

#[derive(Clone)]
pub(crate) struct ShotInfo {
    pub id: String,
    pub at: f64,
}

#[derive(Clone)]
pub(crate) struct CaptionInfo {
    pub line: i64,
    pub s: f64,
    pub e: f64,
    pub text: String,
    pub auto: bool,
    pub raw_w: f64,
    pub fit: f64,
    pub rendered_w: f64,
}

struct OverflowEntry {
    id: String,
    desc: String,
    pose: String,
    sides: Vec<String>,
}

struct EdgeEntry {
    shot: String,
    id: String,
    side: String,
    first: f64,
    n: u64,
}

struct ViolationEntry {
    id: String,
    shot_id: String,
    fields: Vec<String>,
}

struct CompEntry {
    kind: &'static str,
    shot: String,
    id: String,
    first: f64,
    n: u64,
    detail: Option<String>,
}

#[derive(Default)]
struct CompShotAgg {
    samples: u64,
    focal_samples: u64,
    centre_samples: u64,
}

#[derive(Default)]
pub(crate) struct Audit {
    shots: Vec<ShotInfo>,
    overflow: Vec<(String, OverflowEntry)>,
    edge: Vec<(String, EdgeEntry)>,
    sigs: Vec<(String, String)>,
    violations: Vec<ViolationEntry>,
    violation_keys: HashSet<String>,
    comp: Vec<(String, CompEntry)>,
    comp_shots: Vec<(String, CompShotAgg)>,
}

const OVERFLOW_SIDES: [&str; 4] = ["top", "right", "bottom", "left"];

fn side_count(o: &OverflowCounts, side: &str) -> u64 {
    match side {
        "top" => o.top,
        "right" => o.right,
        "bottom" => o.bottom,
        _ => o.left,
    }
}

/// Extract one key's raw JSON value text from a signature string,
/// for sigDiffFields. Understands strings, arrays, objects,
/// numbers, booleans and null at the top level of a flat object —
/// the signature's own shape. Returns None if unparseable.
fn sig_value(sig: &str, key: &str) -> Option<String> {
    let b = sig.as_bytes();
    let pat = format!("\"{key}\"");
    let mut i = sig.find(&pat)? + pat.len();
    while i < b.len() && (b[i] == b':' || b[i] == b' ' || b[i] == b'\t') {
        i += 1;
    }
    let start = i;
    let mut depth = 0i32;
    let mut in_str = false;
    let mut esc = false;
    while i < b.len() {
        let c = b[i] as char;
        if in_str {
            if esc {
                esc = false;
            } else if c == '\\' {
                esc = true;
            } else if c == '"' {
                in_str = false;
            }
        } else {
            match c {
                '"' => in_str = true,
                '[' | '{' => depth += 1,
                ']' | '}' => {
                    if depth == 0 {
                        break;
                    }
                    depth -= 1;
                    if depth == 0 {
                        i += 1;
                        break;
                    }
                }
                ',' if depth == 0 => break,
                _ => {}
            }
        }
        i += 1;
    }
    if i == start {
        return None;
    }
    Some(sig[start..i].trim().to_string())
}

/// Ordered signature keys (cast.ts's lookSignature key order).
const SIG_KEYS: [&str; 11] = [
    "body", "pattern", "patCol", "hat", "hatCol", "band", "neck", "neckCol", "head", "kid",
    "extra",
];

/// `sigDiffFields(a, b)` (capture.ts): JSON-parse both signatures
/// and return the keys whose stringified values differ, in a's key
/// order then b's extras. Unparseable input yields ['sig'].
fn sig_diff_fields(a: &str, b: &str) -> Vec<String> {
    let keys_a: Vec<&str> = SIG_KEYS.iter().filter(|k| sig_value(a, k).is_some()).copied().collect();
    let keys_b: Vec<&str> = SIG_KEYS.iter().filter(|k| sig_value(b, k).is_some()).copied().collect();
    if keys_a.is_empty() && !a.trim_start().starts_with('{') {
        return vec!["sig".to_string()];
    }
    let mut keys: Vec<&str> = keys_a.clone();
    for k in &keys_b {
        if !keys.contains(k) {
            keys.push(k);
        }
    }
    keys.iter()
        .filter(|k| sig_value(a, k) != sig_value(b, k))
        .map(|k| k.to_string())
        .collect()
}

#[derive(Clone, Copy, Default)]
struct FigBoxF {
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
}

/// `figBox` (capture.ts): the drawn figure's box from the plane box
/// by the fixed stage fractions (stage x 384..896 of the 1280-wide
/// canvas, y 256..1280 of the 1408-tall canvas).
fn fig_box(b: &ScreenBox) -> FigBoxF {
    let w = (b.x1 - b.x0) as f64;
    let h = (b.y1 - b.y0) as f64;
    FigBoxF {
        x0: b.x0 as f64 + w * (384.0 / 1280.0),
        y0: b.y0 as f64 + h * (256.0 / 1408.0),
        x1: b.x0 as f64 + w * (896.0 / 1280.0),
        y1: b.y0 as f64 + h * (1280.0 / 1408.0),
    }
}

/// `faceBox` (capture.ts): the standing-adult head disc within the
/// figure (stage x 118..394 of 512, y 40..316 of 1024).
fn face_box(f: &FigBoxF) -> FigBoxF {
    let w = f.x1 - f.x0;
    let h = f.y1 - f.y0;
    FigBoxF {
        x0: f.x0 + w * (118.0 / 512.0),
        y0: f.y0 + h * (40.0 / 1024.0),
        x1: f.x0 + w * (394.0 / 512.0),
        y1: f.y0 + h * (316.0 / 1024.0),
    }
}

impl Audit {
    pub(crate) fn new(shots: Vec<ShotInfo>) -> Audit {
        Audit {
            shots,
            ..Default::default()
        }
    }

    fn shot_id_at(&self, t: f64) -> String {
        let mut id = if self.shots.is_empty() {
            "?".to_string()
        } else {
            self.shots[0].id.clone()
        };
        for s in &self.shots {
            if s.at <= t + 1e-9 {
                id = s.id.clone();
            } else {
                break;
            }
        }
        id
    }

    /// `auditSample(audit, t, cuts)` (capture.ts), transcribed —
    /// including the v5 figBox-based EDGE TOUCH and the advisory
    /// composition derivations from the same box in the same sample.
    pub(crate) fn sample(&mut self, t: f64, cuts: &[Cutout]) {
        let shot_id = self.shot_id_at(t);
        let mut focal: Option<(f64, f64, f64)> = None; // (h, cx, cy)
        for c in cuts {
            if let Some(ov) = &c.overflow {
                let sides: Vec<&str> = OVERFLOW_SIDES
                    .iter()
                    .filter(|s| side_count(ov, s) > 0)
                    .copied()
                    .collect();
                if !sides.is_empty() {
                    let key = format!("{}\0{}\0{}", c.id, c.pose, sides.join(","));
                    if !self.overflow.iter().any(|(k, _)| *k == key) {
                        self.overflow.push((
                            key,
                            OverflowEntry {
                                id: c.id.clone(),
                                desc: c.desc.clone(),
                                pose: c.pose.clone(),
                                sides: sides
                                    .iter()
                                    .map(|s| format!("{}={}", s, side_count(ov, s)))
                                    .collect(),
                            },
                        ));
                    }
                }
            }
            if !c.sig.is_empty() {
                if let Some((_, prev)) = self.sigs.iter().find(|(id, _)| *id == c.id) {
                    if *prev != c.sig {
                        let fields = sig_diff_fields(&prev.clone(), &c.sig);
                        let vkey = format!("{}\0{}\0{}", c.id, shot_id, fields.join(","));
                        if self.violation_keys.insert(vkey) {
                            self.violations.push(ViolationEntry {
                                id: c.id.clone(),
                                shot_id: shot_id.clone(),
                                fields,
                            });
                        }
                    }
                } else {
                    self.sigs.push((c.id.clone(), c.sig.clone()));
                }
            }
            if let Some(b) = &c.box_ {
                let fb = fig_box(b);
                let mut crossed: Vec<&str> = Vec::new();
                if fb.y0 < 0.0 {
                    crossed.push("top");
                }
                if fb.x1 > 1080.0 {
                    crossed.push("right");
                }
                if fb.y1 > 1920.0 {
                    crossed.push("bottom");
                }
                if fb.x0 < 0.0 {
                    crossed.push("left");
                }
                for side in crossed {
                    let key = format!("{}\0{}\0{}", shot_id, c.id, side);
                    if let Some((_, e)) = self.edge.iter_mut().find(|(k, _)| *k == key) {
                        e.n += 1;
                    } else {
                        self.edge.push((
                            key,
                            EdgeEntry {
                                shot: shot_id.clone(),
                                id: c.id.clone(),
                                side: side.to_string(),
                                first: t,
                                n: 1,
                            },
                        ));
                    }
                }
                if !b.behind {
                    let fig = fb;
                    let face = face_box(&fig);
                    let rec = |audit: &mut Audit, kind: &'static str, detail: Option<String>| {
                        let key = format!("{}\0{}\0{}", shot_id, c.id, kind);
                        if let Some((_, e)) = audit.comp.iter_mut().find(|(k, _)| *k == key) {
                            e.n += 1;
                        } else {
                            audit.comp.push((
                                key,
                                CompEntry {
                                    kind,
                                    shot: shot_id.clone(),
                                    id: c.id.clone(),
                                    first: t,
                                    n: 1,
                                    detail,
                                },
                            ));
                        }
                    };
                    let face_area = (face.x1 - face.x0).max(0.0) * (face.y1 - face.y0).max(0.0);
                    let zone_x = (face.x1.min(1080.0) - face.x0.max(0.0)).max(0.0);
                    let zone_y = (face.y1.min(1550.0) - face.y0.max(1340.0)).max(0.0);
                    if face_area > 0.0 && zone_x * zone_y > 0.05 * face_area {
                        rec(self, "zone", None);
                    }
                    let fcx = (face.x0 + face.x1) / 2.0;
                    let fcy = (face.y0 + face.y1) / 2.0;
                    if fcx > 920.0 {
                        rec(self, "rail", None);
                    }
                    if fcy > 1620.0 {
                        rec(self, "bottom", None);
                    }
                    let fig_w = fig.x1 - fig.x0;
                    let fig_h = fig.y1 - fig.y0;
                    if fig_w > 1080.0 || fig_h > 1920.0 {
                        rec(
                            self,
                            "exceeds",
                            Some(format!(
                                "{}×{} px vs the 1080×1920 frame",
                                js_num(js_round(fig_w)),
                                js_num(js_round(fig_h))
                            )),
                        );
                    } else if fig_h > 0.0 && fig_h < 288.0 {
                        rec(
                            self,
                            "small",
                            Some(format!(
                                "{}% of frame height",
                                js_num(js_round(fig_h / 1920.0 * 100.0))
                            )),
                        );
                    }
                    let this = (fig_h, (fig.x0 + fig.x1) / 2.0, (fig.y0 + fig.y1) / 2.0);
                    if focal.map_or(true, |(h, _, _)| fig_h > h) {
                        focal = Some(this);
                    }
                }
            }
        }
        let agg = if let Some((_, a)) = self.comp_shots.iter_mut().find(|(k, _)| *k == shot_id) {
            a
        } else {
            self.comp_shots.push((shot_id.clone(), CompShotAgg::default()));
            &mut self.comp_shots.last_mut().unwrap().1
        };
        agg.samples += 1;
        if let Some((_, cx, cy)) = focal {
            agg.focal_samples += 1;
            if (360.0..=720.0).contains(&cx) && (640.0..=1280.0).contains(&cy) {
                agg.centre_samples += 1;
            }
        }
    }

    /// `printAudit` (capture.ts) — with the v5 incomplete-audit
    /// honesty rule: an audit that lost samples reports `unknown
    /// (audit incomplete)`, never `none`.
    pub(crate) fn print_audit(&self, incomplete: bool, out: &mut Vec<String>) {
        if !self.overflow.is_empty() {
            for (_, e) in &self.overflow {
                out.push(format!(
                    "CANVAS OVERFLOW: {} ({}) {} — pose {}",
                    e.id,
                    e.desc,
                    e.sides.join(" "),
                    e.pose
                ));
            }
        } else {
            out.push(
                if incomplete {
                    "CANVAS OVERFLOW: unknown (audit incomplete)"
                } else {
                    "CANVAS OVERFLOW: none"
                }
                .to_string(),
            );
        }
        if !self.edge.is_empty() {
            for (_, e) in &self.edge {
                out.push(format!(
                    "EDGE TOUCH (warning): {} {} crosses {} (first at t={}s, {} sampled frames)",
                    e.shot,
                    e.id,
                    e.side,
                    fixed2(e.first),
                    e.n
                ));
            }
        } else {
            out.push(
                if incomplete {
                    "EDGE TOUCH: unknown (audit incomplete)"
                } else {
                    "EDGE TOUCH: none"
                }
                .to_string(),
            );
        }
    }

    /// `printContinuity` (capture.ts).
    pub(crate) fn print_continuity(&self, out: &mut Vec<String>) {
        if !self.violations.is_empty() {
            for v in &self.violations {
                out.push(format!(
                    "CONTINUITY VIOLATION: {} in {} — changed: {}",
                    v.id,
                    v.shot_id,
                    v.fields.join(", ")
                ));
            }
        } else {
            out.push(format!(
                "CONTINUITY: clean ({} named characters tracked)",
                self.sigs.len()
            ));
        }
    }

    /// `printComposition` (capture.ts).
    pub(crate) fn print_composition(&self, out: &mut Vec<String>) {
        let (mut zone, mut safe, mut thirds, mut framing) = (0u64, 0u64, 0u64, 0u64);
        for (_, e) in &self.comp {
            match e.kind {
                "zone" => {
                    zone += 1;
                    out.push(format!(
                        "CAPTION ZONE (warning): {} {} face in caption zone (first at t={}s, {} sampled frames)",
                        e.shot,
                        e.id,
                        fixed2(e.first),
                        e.n
                    ));
                }
                "rail" | "bottom" => {
                    safe += 1;
                    out.push(format!(
                        "SAFE AREA (warning): {} {} face centre {} (first at t={}s, {} sampled frames)",
                        e.shot,
                        e.id,
                        if e.kind == "rail" {
                            "under right rail"
                        } else {
                            "in bottom UI band"
                        },
                        fixed2(e.first),
                        e.n
                    ));
                }
                "exceeds" | "small" => {
                    framing += 1;
                    out.push(format!(
                        "FRAMING (warning): {} {} subject {} ({}) (first at t={}s, {} sampled frames)",
                        e.shot,
                        e.id,
                        if e.kind == "exceeds" {
                            "exceeds the frame"
                        } else {
                            "under 15% of frame height"
                        },
                        e.detail.clone().unwrap_or_default(),
                        fixed2(e.first),
                        e.n
                    ));
                }
                _ => {}
            }
        }
        for (shot, a) in &self.comp_shots {
            if a.focal_samples >= 4
                && (a.focal_samples as f64) >= 0.8 * (a.samples as f64)
                && (a.centre_samples as f64) >= 0.9 * (a.focal_samples as f64)
            {
                thirds += 1;
                out.push(format!(
                    "COMPOSITION (advisory): {} subject dead-centre for the whole shot — check the --guides sheet",
                    shot
                ));
            }
        }
        let total = zone + safe + thirds + framing + self.edge.len() as u64;
        out.push(format!(
            "COMPOSITION: {} findings (caption-zone {}, safe-area {}, thirds {}, framing {}, edge-touch {})",
            total,
            zone,
            safe,
            thirds,
            framing,
            self.edge.len()
        ));
    }

    pub(crate) fn overflow_len(&self) -> usize {
        self.overflow.len()
    }

    pub(crate) fn violations_len(&self) -> usize {
        self.violations.len()
    }
}

/// `printCaptionAudit` (capture.ts): WARN by default; under
/// CAP_CHECK=strict the violations print as FAIL and the count is
/// returned for the verdict. The ≥0.33 s/word time floor is NOT
/// enforced (ruling R5 — the width floor wins).
pub(crate) fn print_caption_audit(caps: &[CaptionInfo], strict: bool, out: &mut Vec<String>) -> usize {
    let total = caps.len();
    let mut authored_lines: Vec<i64> = Vec::new();
    let mut all_lines: Vec<i64> = Vec::new();
    for c in caps {
        if !all_lines.contains(&c.line) {
            all_lines.push(c.line);
        }
        if !c.auto && !authored_lines.contains(&c.line) {
            authored_lines.push(c.line);
        }
    }
    // Empty list: JS Math.min() = Infinity and Math.max() =
    // -Infinity, and toFixed/concatenation print those words — the
    // transcription keeps them rather than inventing friendlier
    // numbers (a native run should pass measured caption data; see
    // the M6 gaps file).
    let min_fit = caps.iter().map(|c| c.fit).fold(f64::INFINITY, f64::min);
    let max_w = caps.iter().map(|c| c.rendered_w).fold(f64::NEG_INFINITY, f64::max);
    let min_fit_s = if min_fit.is_infinite() {
        "Infinity".to_string()
    } else {
        fixed2(min_fit)
    };
    let max_w_s = if max_w == f64::NEG_INFINITY {
        "-Infinity".to_string()
    } else {
        js_num(js_round(max_w))
    };
    out.push(format!(
        "CAPTION AUDIT: {} chunks, {}/{} lines authored, min fit {}, max rendered width {} px",
        total,
        authored_lines.len(),
        all_lines.len(),
        min_fit_s,
        max_w_s
    ));
    let mut auto_lines: Vec<i64> = caps
        .iter()
        .filter(|c| c.auto)
        .map(|c| c.line)
        .collect::<Vec<_>>();
    auto_lines.sort();
    auto_lines.dedup();
    if !auto_lines.is_empty() {
        out.push(format!(
            "CAPTIONS: estimate-grade (auto-chunked) on lines {} — author caps in script.json for a master",
            auto_lines
                .iter()
                .map(|i| js_num((i + 1) as f64))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    let mut fails = 0usize;
    let mut seen: HashSet<String> = HashSet::new();
    for c in caps {
        let wide = c.rendered_w > 850.0;
        let low_fit = c.fit < 0.68;
        if !wide && !low_fit {
            continue;
        }
        let key = format!("{}\0{}", c.line, c.text);
        if !seen.insert(key) {
            continue;
        }
        fails += 1;
        let tag = if strict { "FAIL" } else { "WARN" };
        if wide {
            out.push(format!(
                "CAPTION WIDTH {}: line {} chunk \"{}\" renders {} px (> 850) at t={}s — the width floor wins (the per-word time floor is advisory); split the chunk",
                tag,
                c.line + 1,
                c.text,
                js_num(js_round(c.rendered_w)),
                fixed2(c.s)
            ));
        }
        if low_fit {
            out.push(format!(
                "CAPTION FIT {}: line {} chunk \"{}\" fit {} (< 0.68; raw width {} px) at t={}s — split the chunk, never shrink it",
                tag,
                c.line + 1,
                c.text,
                fixed2(c.fit),
                js_num(js_round(c.raw_w)),
                fixed2(c.s)
            ));
        }
    }
    if strict {
        fails
    } else {
        0
    }
}

// ---------------------------------------------------------------------
// Scene assembly — ab1 (Onoda S01 @ t = 1.50) in its three forms
// ---------------------------------------------------------------------

/// Onoda's registered cast look (shots.ts LOOK.onoda, registered
/// via registerCast; the look clone also carries shade '#878E6E',
/// which is not an identity field and never enters the signature).
pub(crate) fn onoda_look() -> Look {
    Look {
        body: Some("#5C6644"),
        hat: Some("cap"),
        hat_col: Some("#4C5638"),
        ..Default::default()
    }
}

/// The painter state S01's draw passes at t = 1.50 (shots.ts S01:
/// mood determined, armR [16,-6], armL [10,6], lean 1, headTurn =
/// the poseTrack sweep; legs unset, hence 'stand'). The sweep's
/// exact f64 at t = 1.50 is -0.024999999999999967 — the value the
/// real page's poseSummary() prints (verified against Chrome,
/// docs/ab/telemetry/) — so the native pose string matches
/// byte-for-byte; the rounded -0.025 would not.
fn onoda_pose() -> PoseState {
    PoseState {
        mood: "determined",
        arm_l: (10.0, 6.0),
        arm_r: (16.0, -6.0),
        legs: "stand",
        head_tilt: 0.0,
        head_turn: -0.024999999999999967,
    }
}

fn ab1_camera() -> ProjCamera {
    ProjCamera {
        eye: [
            crate::ab1::EYE[0] as f64,
            crate::ab1::EYE[1] as f64,
            crate::ab1::EYE[2] as f64,
        ],
        target: [
            crate::ab1::TGT[0] as f64,
            crate::ab1::TGT[1] as f64,
            crate::ab1::TGT[2] as f64,
        ],
        up: [0.0, 1.0, 0.0],
        fov_deg: 44.0,
        aspect: 1080.0 / 1920.0,
        near: 0.05,
        far: 800.0,
    }
}

fn cutout_from(px: &[u8], w: u32, h: u32, model: &M4, cam: &ProjCamera) -> Cutout {
    let look = onoda_look();
    Cutout {
        id: "onoda".to_string(),
        desc: describe(&look),
        pose: pose_summary(&onoda_pose()),
        overflow: Some(scan_canvas_overflow(px, w as usize, h as usize, 2.0)),
        box_: Some(project_box(model, cam, 1.75)),
        sig: look_signature(&look),
    }
}

/// ab1, hand-authored form: the painter canvas from
/// ab1::paint_onoda (the M4 painter at res 2, 2560x2816 — the same
/// canvas size the Chrome toon paints) and the card draw's own
/// model matrix from the scene graph (ab1::onoda_card_and_rifle).
pub(crate) fn ab1_cutout() -> Cutout {
    let (px, w, h) = crate::ab1::paint_onoda();
    let (card, _rifle) = crate::ab1::onoda_card_and_rifle();
    let model = model_from_f32(&card.model);
    cutout_from(&px, w, h, &model, &ab1_camera())
}

/// ab1, scene-file form: the camera parsed from
/// scenes/ab1.scene.json (the loader's own data); the character is
/// the file's `code-ref` node `ab1-onoda-card`, which the loader
/// splices from the same scene-graph code as the hand-authored
/// path, so the painter canvas and card model are shared — what
/// the file form proves is that the data carries the same staging.
pub(crate) fn ab1_file_cutout(path: &str) -> Result<Cutout, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("read {path}: {e}"))?;
    let root = crate::scene::parse_json(&text)?;
    let camv = root.get("camera").ok_or("scene file has no camera block")?;
    let arr3 = |v: Option<&crate::scene::Value>| -> Result<[f64; 3], String> {
        let a = v.and_then(|x| x.as_arr()).ok_or("camera vector missing")?;
        if a.len() != 3 {
            return Err("camera vector must have 3 components".to_string());
        }
        Ok([
            a[0].as_f64().ok_or("bad camera vector")?,
            a[1].as_f64().ok_or("bad camera vector")?,
            a[2].as_f64().ok_or("bad camera vector")?,
        ])
    };
    let cam = ProjCamera {
        eye: arr3(camv.get("eye"))?,
        target: arr3(camv.get("target"))?,
        up: arr3(camv.get("up"))?,
        fov_deg: camv
            .get("fovY")
            .and_then(|v| v.as_f64())
            .ok_or("camera.fovY missing")?,
        aspect: 1080.0 / 1920.0,
        near: camv
            .get("near")
            .and_then(|v| v.as_f64())
            .ok_or("camera.near missing")?,
        far: camv
            .get("far")
            .and_then(|v| v.as_f64())
            .ok_or("camera.far missing")?,
    };
    let (px, w, h) = crate::ab1::paint_onoda();
    let (card, _rifle) = crate::ab1::onoda_card_and_rifle();
    let model = model_from_f32(&card.model);
    Ok(cutout_from(&px, w, h, &model, &cam))
}

/// The deliberately-overflowing variant: the same Onoda, the same
/// S01 camera, restaged — group moved right, down and toward the
/// camera (breaching the right rail and the caption zone), and the
/// drawn pose tipped to lean 42 degrees so the head's paint crosses
/// the character canvas's right edge (canvas overflow). The Chrome
/// harness stages the identical numbers through the real Toon
/// (docs/ab/telemetry/project/), so both gates judge one staging.
pub(crate) const VARIANT_POS: [f64; 3] = [1.9, -0.55, 0.6];
pub(crate) const VARIANT_LEAN_DEG: f32 = 42.0;

pub(crate) fn ab1_variant_cutout() -> Cutout {
    let h_t = 1.75 / 0.96;
    let cam = ab1_camera();
    let (yaw, pitch) = face_angles(VARIANT_POS, cam.eye, h_t, true);
    let cam_pitch = cam_pitch_of(VARIANT_POS, cam.eye, h_t);
    let spec = crate::scene::ToonSpec {
        body: "#5C6644",
        head: "#F7F7FA",
        shade: Some("#878E6E"),
        cap: Some("#4C5638"),
        mood: crate::scene::ToonMood::Determined,
        arm_l: (10.0, 6.0),
        arm_r: (16.0, -6.0),
        lean_deg: VARIANT_LEAN_DEG,
        head_turn: -0.025,
        cam_pitch: cam_pitch as f32,
    };
    let (px, w, h) = crate::scene::paint_toon(&spec);
    let model = toon_model(VARIANT_POS, yaw, pitch);
    cutout_from(&px, w, h, &model, &cam)
}

// ---------------------------------------------------------------------
// Caption metrics input (v5 CaptionInfo shape)
// ---------------------------------------------------------------------

/// Load caption metrics (the OP.captions() v5 field set) from a
/// JSON array — project data measured by the renderer's own
/// measureText (see the M6 package README for provenance; a native
/// Inter-900 measurer is contract §2.4 future work, so until it
/// lands the metrics enter as data, exactly as capture receives
/// them from the page).
pub(crate) fn load_captions(path: &str) -> Result<Vec<CaptionInfo>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("read {path}: {e}"))?;
    let root = crate::scene::parse_json(&text)?;
    let arr = root.as_arr().ok_or("captions file must be a JSON array")?;
    let mut out = Vec::new();
    for (i, c) in arr.iter().enumerate() {
        let num = |k: &str| -> Result<f64, String> {
            c.get(k)
                .and_then(|v| v.as_f64())
                .ok_or_else(|| format!("captions[{i}].{k} missing or not a number"))
        };
        out.push(CaptionInfo {
            line: num("line")? as i64,
            s: num("s")?,
            e: num("e")?,
            text: c
                .get("text")
                .and_then(|v| v.as_str())
                .ok_or_else(|| format!("captions[{i}].text missing"))?
                .to_string(),
            auto: matches!(c.get("auto"), Some(crate::scene::Value::Bool(true))),
            raw_w: num("rawW")?,
            fit: num("fit")?,
            rendered_w: num("renderedW")?,
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------
// The telemetry run — check mode's audit over the native samples
// ---------------------------------------------------------------------

/// Build the audit lines for one sample set, mirroring check
/// mode's print order (printAudit, printContinuity,
/// printComposition, printCaptionAudit, logs, verdict). Returns
/// (lines, failed). The native runtime has no page: `bad` (thrown
/// samples) and `logs` (page errors) have no native counterpart —
/// a native sample is an in-process computation that either
/// produces its cutouts or fails the whole run before the audit
/// starts, so both terms are 0 here by construction.
pub(crate) fn audit_lines(
    shots: Vec<ShotInfo>,
    samples: &[(f64, Vec<Cutout>)],
    captions: &[CaptionInfo],
    strict: bool,
) -> (Vec<String>, bool) {
    let mut audit = Audit::new(shots);
    for (t, cuts) in samples {
        audit.sample(*t, cuts);
    }
    let mut lines: Vec<String> = Vec::new();
    audit.print_audit(false, &mut lines);
    audit.print_continuity(&mut lines);
    audit.print_composition(&mut lines);
    let cap_fails = print_caption_audit(captions, strict, &mut lines);
    let failed = audit.overflow_len() > 0 || audit.violations_len() > 0 || cap_fails > 0;
    lines.push(if failed { "CHECK FAILED" } else { "CHECK OK" }.to_string());
    (lines, failed)
}

/// `--telemetry <mode>`: modes `ab1`, `ab1-file` (scene file path
/// in `scene_file`), `ab1-variant`. Prints capture's audit lines
/// for the native sample(s) and returns the process exit code
/// (0 = CHECK OK, 1 = CHECK FAILED — capture sets exitCode the
/// same way).
pub(crate) fn run(mode: &str, scene_file: Option<&str>, captions_path: Option<&str>, cutouts_out: Option<&str>) -> i32 {
    let captions: Vec<CaptionInfo> = match captions_path {
        Some(p) => match load_captions(p) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("telemetry: {e}");
                return 2;
            }
        },
        None => Vec::new(),
    };
    let strict = std::env::var("CAP_CHECK")
        .map(|v| v.trim().to_lowercase() == "strict")
        .unwrap_or(false);
    let (label, cutout) = match mode {
        "ab1" => ("ab1 (hand-authored staging, S01 @ t=1.50)", ab1_cutout()),
        "ab1-file" => {
            let path = scene_file.unwrap_or("scenes/ab1.scene.json");
            match ab1_file_cutout(path) {
                Ok(c) => ("ab1 (scene-file staging, S01 @ t=1.50)", c),
                Err(e) => {
                    eprintln!("telemetry: {e}");
                    return 2;
                }
            }
        }
        "ab1-variant" => ("ab1-variant (deliberate overflow staging)", ab1_variant_cutout()),
        other => {
            eprintln!("unknown telemetry mode: {other} (expected ab1, ab1-file or ab1-variant)");
            return 2;
        }
    };
    if let Some(p) = cutouts_out {
        let json = format!("[{}]", cutout.to_json());
        if let Err(e) = std::fs::write(p, json) {
            eprintln!("telemetry: write {p}: {e}");
            return 2;
        }
    }
    if std::env::var("TALERENDER_TELEMETRY_DEBUG").is_ok() {
        if let Some(b) = &cutout.box_ {
            let fig = fig_box(b);
            let face = face_box(&fig);
            eprintln!(
                "debug: box=({},{},{},{}) fig=({:.1},{:.1},{:.1},{:.1}) face=({:.1},{:.1},{:.1},{:.1}) face_c=({:.1},{:.1})",
                b.x0, b.y0, b.x1, b.y1, fig.x0, fig.y0, fig.x1, fig.y1, face.x0, face.y0,
                face.x1, face.y1, (face.x0 + face.x1) / 2.0, (face.y0 + face.y1) / 2.0
            );
        }
    }
    println!("TELEMETRY: {label}");
    println!("telemetry source: native painter canvas 2560x2816 (res 2) + scene-graph box projection (three.js arithmetic, f64)");
    let shots = vec![ShotInfo {
        id: "S01".to_string(),
        at: 0.0,
    }];
    let (lines, failed) = audit_lines(shots, &[(1.5, vec![cutout])], &captions, strict);
    for l in lines {
        println!("{l}");
    }
    if failed {
        1
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn js_num_matches_js_string() {
        assert_eq!(js_num(0.0), "0");
        assert_eq!(js_num(-0.0), "0");
        assert_eq!(js_num(10.0), "10");
        assert_eq!(js_num(-6.0), "-6");
        assert_eq!(js_num(-0.025), "-0.025");
        assert_eq!(js_num(0.28747), "0.28747");
        assert_eq!(js_round(2.5), 3.0);
        assert_eq!(js_round(-2.5), -2.0);
        assert_eq!(js_round(-1.4), -1.0);
    }

    #[test]
    fn onoda_signature_and_describe_are_exact() {
        assert_eq!(
            look_signature(&onoda_look()),
            "{\"body\":\"#5C6644\",\"pattern\":null,\"patCol\":null,\"hat\":\"cap\",\"hatCol\":\"#4C5638\",\"band\":null,\"neck\":null,\"neckCol\":null,\"head\":null,\"kid\":false,\"extra\":[]}"
        );
        assert_eq!(describe(&onoda_look()), "hat=cap body=#5C6644 pattern=plain");
    }

    #[test]
    fn onoda_pose_summary_is_exact() {
        assert_eq!(
            pose_summary(&onoda_pose()),
            "mood=determined armL=[10,6] armR=[16,-6] legs=stand head=0/-0.024999999999999967"
        );
        let plain = PoseState {
            mood: "neutral",
            arm_l: (8.0, 6.0),
            arm_r: (8.0, 6.0),
            legs: "stand",
            head_tilt: 0.0,
            head_turn: 0.0,
        };
        assert_eq!(pose_summary(&plain), "mood=neutral armL=[8,6] armR=[8,6] legs=stand");
    }

    #[test]
    fn overflow_scan_counts_strips_with_corners_in_both() {
        // 8x6 canvas, res 1 -> strip 1. Paint: top-left corner,
        // one top-edge pixel, one right-edge pixel, alpha 32
        // (below threshold) at bottom edge, alpha 33 at bottom.
        let (w, h) = (8usize, 6usize);
        let mut px = vec![0u8; w * h * 4];
        let set = |px: &mut Vec<u8>, x: usize, y: usize, a: u8| px[(y * w + x) * 4 + 3] = a;
        set(&mut px, 0, 0, 255); // corner: top + left
        set(&mut px, 3, 0, 255); // top only
        set(&mut px, 7, 2, 255); // right only
        set(&mut px, 2, 5, 32); // bottom, NOT counted (alpha > 32 required)
        set(&mut px, 4, 5, 33); // bottom, counted
        let o = scan_canvas_overflow(&px, w, h, 1.0);
        assert_eq!(o, OverflowCounts { top: 2, right: 1, bottom: 1, left: 1 });
        // res 2 -> strip 2: now the right-edge pixel at x=7 is in a
        // 2-wide strip, and x=6 col counts too (empty); counts for
        // top row pair include y=1 (empty) — same totals here.
        let o2 = scan_canvas_overflow(&px, w, h, 2.0);
        assert_eq!(o2.top, 2);
        assert_eq!(o2.left, 1);
    }

    #[test]
    fn face_angles_reproduce_the_baked_ab1_card() {
        // ab1.rs bakes rotate_y(0.08282) / rotate_x(0.11642) from
        // face() at the S01 camera; camPitchOf = 0.28747 (the
        // painter's cam_pitch). The transcription must land on the
        // same values from the same inputs.
        let eye = [0.83432, 2.64354, 5.28499];
        let pos = [0.35, 0.0, -0.55];
        let h = 1.75 / 0.96;
        let (yaw, pitch) = face_angles(pos, eye, h, true);
        assert!((yaw - 0.08282).abs() < 1e-4, "yaw {yaw}");
        assert!((pitch - 0.11642).abs() < 1e-4, "pitch {pitch}");
        let cp = cam_pitch_of(pos, eye, h);
        assert!((cp - 0.28747).abs() < 1e-4, "camPitch {cp}");
    }

    #[test]
    fn box_projection_centred_and_behind() {
        let cam = ProjCamera {
            eye: [0.0, 0.0, 0.0],
            target: [0.0, 0.0, -1.0],
            up: [0.0, 1.0, 0.0],
            fov_deg: 44.0,
            aspect: 1080.0 / 1920.0,
            near: 0.05,
            far: 800.0,
        };
        // Plane straight ahead at z = -5, no rotation: symmetric in
        // x about 540. Half plane width = (1.75/0.96)*1280/1024/2 =
        // 1.13932; ndc.x = (f/aspect)*1.13932/5 with f = 1/tan(22deg)
        // = 2.47509 -> 1.00264 -> px 1081.4 -> x1 = 1081, x0 = -1.
        let model = m4_translate(0.0, 0.0, -5.0);
        let b = project_box(&model, &cam, 1.75);
        assert!(!b.behind);
        assert_eq!(b.x1, 1081);
        assert_eq!(b.x0, -1);
        assert_eq!(b.x0 + b.x1, 1080);
        // Behind the camera: viewZ > 0 flags behind.
        let model2 = m4_translate(0.0, 0.0, 5.0);
        let b2 = project_box(&model2, &cam, 1.75);
        assert!(b2.behind);
    }

    #[test]
    fn fig_and_face_box_fractions() {
        let b = ScreenBox { x0: 0, y0: 0, x1: 1280, y1: 1408, behind: false };
        let f = fig_box(&b);
        assert_eq!((f.x0, f.y0, f.x1, f.y1), (384.0, 256.0, 896.0, 1280.0));
        let face = face_box(&f);
        assert!((face.x0 - (384.0 + 512.0 * (118.0 / 512.0))).abs() < 1e-9);
        assert!((face.y1 - (256.0 + 1024.0 * (316.0 / 1024.0))).abs() < 1e-9);
    }

    #[test]
    fn sig_diff_names_the_changed_field() {
        let a = look_signature(&onoda_look());
        let mut l2 = onoda_look();
        l2.hat = Some("bowler");
        let b = look_signature(&l2);
        assert_eq!(sig_diff_fields(&a, &b), vec!["hat".to_string()]);
        assert!(sig_diff_fields(&a, &a).is_empty());
    }

    fn clean_cutout() -> Cutout {
        Cutout {
            id: "onoda".to_string(),
            desc: describe(&onoda_look()),
            pose: pose_summary(&onoda_pose()),
            overflow: Some(OverflowCounts::default()),
            box_: Some(ScreenBox { x0: 300, y0: 400, x1: 800, y1: 1500, behind: false }),
            sig: look_signature(&onoda_look()),
        }
    }

    #[test]
    fn audit_verdict_clean_and_overflowing() {
        let shots = vec![ShotInfo { id: "S01".to_string(), at: 0.0 }];
        let (lines, failed) = audit_lines(shots.clone(), &[(1.5, vec![clean_cutout()])], &[], false);
        assert!(!failed);
        assert!(lines.iter().any(|l| l == "CANVAS OVERFLOW: none"));
        assert!(lines
            .iter()
            .any(|l| l == "CONTINUITY: clean (1 named characters tracked)"));
        assert!(lines.iter().any(|l| l == "CHECK OK"));

        let mut bad = clean_cutout();
        bad.overflow = Some(OverflowCounts { top: 0, right: 123, bottom: 0, left: 0 });
        let (lines, failed) = audit_lines(shots, &[(1.5, vec![bad])], &[], false);
        assert!(failed);
        assert!(lines.iter().any(|l| {
            l == "CANVAS OVERFLOW: onoda (hat=cap body=#5C6644 pattern=plain) right=123 — pose mood=determined armL=[10,6] armR=[16,-6] legs=stand head=0/-0.024999999999999967"
        }));
        assert!(lines.iter().any(|l| l == "CHECK FAILED"));
    }

    #[test]
    fn caption_audit_warns_and_strict_fails() {
        let caps = vec![CaptionInfo {
            line: 0,
            s: 1.32,
            e: 1.8,
            text: "fighting".to_string(),
            auto: false,
            raw_w: 900.0,
            fit: 1.0,
            rendered_w: 900.0,
        }];
        let mut lines = Vec::new();
        let fails = print_caption_audit(&caps, false, &mut lines);
        assert_eq!(fails, 0);
        assert!(lines[0].starts_with("CAPTION AUDIT: 1 chunks, 1/1 lines authored"));
        assert!(lines.iter().any(|l| l.contains("CAPTION WIDTH WARN: line 1 chunk \"fighting\" renders 900 px")));
        let mut lines2 = Vec::new();
        let fails2 = print_caption_audit(&caps, true, &mut lines2);
        assert_eq!(fails2, 1);
        assert!(lines2.iter().any(|l| l.contains("CAPTION WIDTH FAIL:")));
    }
}
