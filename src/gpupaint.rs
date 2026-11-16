//! GPU-first Phase 0 harness CLI: `--gpu-paint <SUBCOMMAND>`.
//!
//! Compiled ONLY under the cargo feature `gpupaint` (default OFF);
//! with the feature off the flag is an ordinary unknown argument,
//! exactly as before this seat.
//!
//! Record side (this seat): WORKING end-to-end, pure CPU —
//!   record-battery <outdir>   the M4 stroke battery cards, each
//!                             painted inside a trace window;
//!                             writes the CPU card PNGs (via the
//!                             existing battery entry) plus one
//!                             display-list JSON per card.
//!   record-states <outdir>    paint each (name, look, state)
//!                             from `state_spread()` inside a
//!                             trace window; writes <name>.png +
//!                             <name>.json per state. The STATES
//!                             seat supplies the spread via
//!                             `gpupaint_states::state_spread()`,
//!                             to which `state_spread()` here
//!                             delegates.
//! Replay side (GPU executor seat): WORKING —
//!   replay <list.json> <out.png>   wgpu compute replay of the
//!                             list (src/gpupaint.wgsl; design +
//!                             exactness argument in
//!                             docs/integration/GPUFIRST-GPUX.md);
//!                             writes the canvas PNG and prints
//!                             the pixel sha256, per-prim dispatch
//!                             counts and total GPU wall time.
//!   compare <a.png> <b.png>        [stub] not yet integrated
//!   time <list.json>               [stub] not yet integrated
//!
//! Exit codes follow the CLI's audit table: 0 success, 2 bad
//! usage / unreadable input, 3 not-yet-integrated executor (a
//! render-path failure), 4 image I/O failure (via TaleError).

use crate::gputrace;
use crate::toonfull::{FullLook, FullState};

const USAGE: &str = "\
usage: talerender --gpu-paint <SUBCOMMAND>
  record-battery <outdir>     record the stroke-battery cards (PNG + display-list JSON per card)
  record-states <outdir>      record the state spread (PNG + display-list JSON per state)
  replay <list.json> <out.png>  GPU-replay a display list (writes PNG; prints sha256 + dispatch counts + GPU wall time)
  compare <a.png> <b.png>       [stub] byte/metric comparison of two PNGs
  time <list.json>              [stub] replay timing
";

/// The state spread for `record-states`: (card name, look, state)
/// triples painted with `toonfull::paint_toon_full` on the
/// production 2560x2816 canvas. Supplied by the STATES seat
/// (src/gpupaint_states.rs, docs/integration/GPUFIRST-STATES.md);
/// this is a delegation so `gpupaint::state_spread()` IS the
/// chartered spread.
pub(crate) fn state_spread() -> Vec<(String, FullLook, FullState)> {
    crate::gpupaint_states::state_spread()
}

/// Battery-card trace finish (called from battery.rs under the
/// same feature): close the card's trace window and write its
/// display-list JSON next to the card PNG. No-op when the card
/// was painted without an armed window (production battery mode
/// in a `gpupaint` build).
pub(crate) fn finish_card(out_dir: &str, name: &str) {
    if !gputrace::active() {
        return;
    }
    let list = gputrace::take();
    let path = format!("{out_dir}/{name}.json");
    match std::fs::write(&path, list.to_json()) {
        Ok(()) => println!(
            "wrote {path} ({} prims, {} plates)",
            list.prims.len(),
            list.plates.len()
        ),
        Err(e) => eprintln!("warning: could not write {path}: {e}"),
    }
}

fn save_png(px: &[u8], w: u32, h: u32, path: &str) -> crate::errors::TaleResult<()> {
    let img = image::RgbaImage::from_raw(w, h, px.to_vec()).expect("pixel buffer size mismatch");
    img.save(path).map_err(|e| crate::errors::TaleError::image_io(path, e))?;
    println!("wrote {path} ({w}x{h})");
    Ok(())
}

fn record_battery(out_dir: &str) -> i32 {
    gputrace::set_armed(true);
    let r = crate::battery::run_battery(out_dir);
    gputrace::set_armed(false);
    match r {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("error: {e}");
            e.exit_code()
        }
    }
}

fn record_states(out_dir: &str) -> i32 {
    if let Err(e) = std::fs::create_dir_all(out_dir) {
        eprintln!("error: could not create {out_dir}: {e}");
        return 4;
    }
    gputrace::set_armed(true);
    let states = state_spread();
    println!(
        "record-states: {} state(s) in the spread (toonfull::paint_toon_full, 2560x2816)",
        states.len()
    );
    let mut code = 0;
    for (name, look, state) in &states {
        if name.is_empty() || name.contains('/') || name.contains('\\') || name.contains("..") {
            eprintln!("error: bad state card name {name:?} (must be a plain file stem)");
            code = 2;
            continue;
        }
        // paint_toon_full's canvas is fixed (1280x1408 @ res 2),
        // the same constants battery.rs uses for its toon cards.
        gputrace::begin(2560, 2816);
        let (px, w, h, _, _) = crate::toonfull::paint_toon_full(look, state);
        let list = gputrace::take();
        let png = format!("{out_dir}/{name}.png");
        if let Err(e) = save_png(&px, w, h, &png) {
            eprintln!("error: {e}");
            code = e.exit_code();
            continue;
        }
        let path = format!("{out_dir}/{name}.json");
        match std::fs::write(&path, list.to_json()) {
            Ok(()) => println!(
                "wrote {path} ({} prims, {} plates)",
                list.prims.len(),
                list.plates.len()
            ),
            Err(e) => {
                eprintln!("error: could not write {path}: {e}");
                code = 4;
            }
        }
    }
    gputrace::set_armed(false);
    code
}

fn stub(what: &str) -> i32 {
    eprintln!("error: {what}: GPU executor not yet integrated (replay side is a later seat)");
    3
}

/// `--gpu-paint` entry: `args` is everything after the flag.
/// Returns the process exit code.
pub(crate) fn run(args: &[String]) -> i32 {
    let Some(sub) = args.first().map(|s| s.as_str()) else {
        eprint!("{USAGE}");
        return 2;
    };
    match sub {
        "record-battery" if args.len() == 2 => record_battery(&args[1]),
        "record-states" if args.len() == 2 => record_states(&args[1]),
        "replay" if args.len() == 3 => replay(&args[1], &args[2]),
        "compare" if args.len() == 3 => stub("compare"),
        "time" if args.len() == 2 => stub("time"),
        _ => {
            eprintln!("error: bad --gpu-paint invocation");
            eprint!("{USAGE}");
            2
        }
    }
}

// =====================================================================
// GPU executor (seat GPUX) — `replay`.
//
// Replays a recorded DisplayList on the GPU: ONE compute dispatch
// per prim over the prim's CPU bounding box, in list order, into a
// storage-buffer canvas of packed u32 RGBA8. The per-pixel math is
// src/gpupaint.wgsl, a textual port of the CPU painter per the
// normative INV-A raster spec. Production shape: the canvas is
// GPU-resident; the readback below exists ONLY for this harness
// (PNG + sha256 for the parity seats).
//
// Masks are BBOX-SCOPED (INV-B scoping rule): each mask prim gets
// a buffer sized to its own geometry bbox, never canvas-sized;
// consumers translate global -> mask-local (see mask_at in the
// WGSL) and out-of-scope reads are false, exactly as the CPU's
// full-canvas mask is false outside the mask prim's bbox.
// =====================================================================

use crate::gputrace::Prim;
use std::collections::HashMap;

const WGSL_SRC: &str = include_str!("gpupaint.wgsl");
/// Workgroup size, both axes (8x8 = 64 invocations) — stated for
/// the PERF seat; the dispatch counts printed by `replay` are in
/// these workgroups.
const WGSZ: u32 = 8;
/// toonfull.rs's plate resolution (`const RES: f32 = 2.0`, private
/// to that module; composite_plate's math is written in it).
const PLATE_RES: f32 = 2.0;

/// The 144-byte uniform block; byte layout mirrors the WGSL
/// `Params` struct field-for-field (WGSL alignment: the vec4
/// members land on 16-byte boundaries at 32/48/64/80/96/112/128).
struct PBuf {
    b: [u8; 144],
}

impl PBuf {
    fn new(canvas_w: u32, canvas_h: u32, bbox: (i32, i32, i32, i32)) -> Self {
        let mut p = PBuf { b: [0u8; 144] };
        p.u32at(0, canvas_w);
        p.u32at(4, canvas_h);
        p.i32at(8, bbox.0);
        p.i32at(12, bbox.1);
        p.i32at(16, bbox.2);
        p.i32at(20, bbox.3);
        p
    }
    fn u32at(&mut self, off: usize, v: u32) {
        self.b[off..off + 4].copy_from_slice(&v.to_ne_bytes());
    }
    fn i32at(&mut self, off: usize, v: i32) {
        self.b[off..off + 4].copy_from_slice(&v.to_ne_bytes());
    }
    fn f32at(&mut self, off: usize, v: f32) {
        self.b[off..off + 4].copy_from_slice(&v.to_ne_bytes());
    }
    fn col(&mut self, c: crate::scene::Rgba) {
        self.u32at(32, c.0 as u32);
        self.u32at(36, c.1 as u32);
        self.u32at(40, c.2 as u32);
        self.u32at(44, c.3 as u32);
    }
    /// p0..p3 at offsets 48/64/80/96.
    fn p(&mut self, idx: usize, v: [f32; 4]) {
        for (i, x) in v.iter().enumerate() {
            self.f32at(48 + idx * 16 + i * 4, *x);
        }
    }
    fn u0(&mut self, v: [u32; 4]) {
        for (i, x) in v.iter().enumerate() {
            self.u32at(112 + i * 4, *x);
        }
    }
    fn mk(&mut self, v: [i32; 4]) {
        for (i, x) in v.iter().enumerate() {
            self.i32at(128 + i * 4, *x);
        }
    }
}

/// A mask prim's GPU scope: buffer sized to this bbox.
#[derive(Clone, Copy)]
struct MaskAlloc {
    ox: i32,
    oy: i32,
    w: u32,
    h: u32,
}

struct PrepPrim {
    kind: &'static str,
    entry: &'static str,
    params: [u8; 144],
    pts: Vec<f32>,
    plate: Option<Vec<u32>>,
    /// Prim index of the mask buffer to bind (writer or consumer).
    mask_prim: Option<u32>,
    bbox: (i32, i32, i32, i32),
    groups: (u32, u32),
}

/// scene.rs Painter::bbox, verbatim (floor/ceil, canvas clamp,
/// inclusive result).
fn cpu_bbox(w: u32, h: u32, x0: f32, y0: f32, x1: f32, y1: f32) -> (i32, i32, i32, i32) {
    (
        (x0.floor() as i32).max(0),
        (y0.floor() as i32).max(0),
        (x1.ceil() as i32).min(w as i32 - 1),
        (y1.ceil() as i32).min(h as i32 - 1),
    )
}

fn groups_of(bbox: (i32, i32, i32, i32)) -> (u32, u32) {
    let w = bbox.2 - bbox.0 + 1;
    let h = bbox.3 - bbox.1 + 1;
    if w <= 0 || h <= 0 {
        (0, 0)
    } else {
        ((w as u32).div_ceil(WGSZ), (h as u32).div_ceil(WGSZ))
    }
}

/// Rotated-ellipse AABB half-extents, as fill_ellipse /
/// ellipse_mask compute them (separate .cos()/.sin() calls —
/// INV-A spec §3.1: reproduce with CPU-precomputed extents).
fn ellipse_extents(rx: f32, ry: f32, rot: f32) -> (f32, f32) {
    (
        (rx * rot.cos()).abs() + (ry * rot.sin()).abs(),
        (rx * rot.sin()).abs() + (ry * rot.cos()).abs(),
    )
}

fn pts_flat(pts: &[(f32, f32)]) -> Vec<f32> {
    let mut v = Vec::with_capacity(pts.len() * 2);
    for &(x, y) in pts {
        v.push(x);
        v.push(y);
    }
    v
}

fn aabb(pts: &[(f32, f32)]) -> (f32, f32, f32, f32) {
    let (mut mnx, mut mny, mut mxx, mut mxy) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    for &(x, y) in pts {
        mnx = mnx.min(x);
        mny = mny.min(y);
        mxx = mxx.max(x);
        mxy = mxy.max(y);
    }
    (mnx, mny, mxx, mxy)
}

/// One cap fan's host-computed parameters: (centre, axis, r, n,
/// cos_half). scene.rs CapFan's fields are module-private, so
/// cap_fan's formula is replicated textually here — same Rust
/// libm calls in the same order, hence identical values
/// (INV-A spec §1.8: fans MUST be CPU-built; a GPU-side acos
/// near a ceil boundary could change n and the whole cap).
struct Fan {
    c: (f32, f32),
    axis: (f32, f32),
    r: f32,
    n: u32,
    cos_half: f32,
}

fn cap_fan_params(c: (f32, f32), axis: (f32, f32), r: f32) -> Fan {
    let cos_max = (1.0f32 - 0.25 / r).clamp(-1.0, 1.0);
    let n = ((std::f32::consts::PI / (2.0 * cos_max.acos())).ceil() as usize).max(1);
    Fan {
        c,
        axis,
        r,
        n: n as u32,
        cos_half: (std::f32::consts::PI / (2.0 * n as f32)).cos(),
    }
}

/// scene.rs stroke_polyline's mk_fan, verbatim.
fn mk_fan(pts: &[(f32, f32)], vi: usize, ni: usize, hw: f32, closed: bool) -> Option<Fan> {
    if closed {
        return None;
    }
    let (ax, ay) = pts[vi];
    let (bx, by) = pts[ni];
    let (dx, dy) = (ax - bx, ay - by);
    let l = dx.hypot(dy);
    if l < 1e-6 {
        None
    } else {
        Some(cap_fan_params((ax, ay), (dx / l, dy / l), hw))
    }
}

/// Prepare a stroke over an already-flattened point list
/// (stroke_polyline / stroke_ellipse / stroke_quad all land here,
/// exactly as on the CPU).
fn prep_stroke(
    kind: &'static str,
    pts: &[(f32, f32)],
    width: f32,
    col: crate::scene::Rgba,
    cw: u32,
    ch: u32,
) -> PrepPrim {
    let base = |bbox| {
        let mut p = PBuf::new(cw, ch, bbox);
        p.col(col);
        p
    };
    if pts.len() < 2 {
        // CPU returns without painting; keep a zero-dispatch
        // record so per-prim indices in the log still line up.
        let bbox = (0, 0, -1, -1);
        return PrepPrim {
            kind,
            entry: "main_stroke",
            params: base(bbox).b,
            pts: Vec::new(),
            plate: None,
            mask_prim: None,
            bbox,
            groups: (0, 0),
        };
    }
    let hw = width / 2.0;
    let (mnx, mny, mxx, mxy) = aabb(pts);
    let bbox = cpu_bbox(cw, ch, mnx - hw - 1.0, mny - hw - 1.0, mxx + hw + 1.0, mxy + hw + 1.0);
    let nseg = pts.len() - 1;
    let closed = pts.len() > 2 && (pts[0].0 - pts[nseg].0).hypot(pts[0].1 - pts[nseg].1) < 0.05;
    let fan_start = mk_fan(pts, 0, 1, hw, closed);
    let fan_end = mk_fan(pts, nseg, nseg - 1, hw, closed);
    let mut p = base(bbox);
    p.p(0, [hw, 0.0, 0.0, 0.0]);
    let mut flags = 0u32;
    if let Some(f) = &fan_start {
        p.p(1, [f.c.0, f.c.1, f.axis.0, f.axis.1]);
        p.f32at(80, f.r);
        p.f32at(84, f.cos_half);
        flags |= 1;
    }
    if let Some(f) = &fan_end {
        p.f32at(88, f.c.0);
        p.f32at(92, f.c.1);
        p.p(3, [f.axis.0, f.axis.1, f.r, f.cos_half]);
        flags |= 2;
    }
    p.u0([
        pts.len() as u32,
        fan_start.as_ref().map_or(0, |f| f.n),
        fan_end.as_ref().map_or(0, |f| f.n),
        flags,
    ]);
    PrepPrim {
        kind,
        entry: "main_stroke",
        params: p.b,
        pts: pts_flat(pts),
        plate: None,
        mask_prim: None,
        bbox,
        groups: groups_of(bbox),
    }
}

fn prepare(list: &gputrace::DisplayList) -> Result<(Vec<PrepPrim>, HashMap<u32, MaskAlloc>), String> {
    let (cw, ch) = (list.canvas_w, list.canvas_h);
    let mut allocs: HashMap<u32, MaskAlloc> = HashMap::new();
    let mut out: Vec<PrepPrim> = Vec::with_capacity(list.prims.len());

    // Masked consumers must reference an EARLIER mask prim (the
    // recorder's resolve_mask only ever yields those); NO_MASK or
    // anything else fails loudly, per GPUFIRST-TRACE.md.
    let mask_scope = |allocs: &HashMap<u32, MaskAlloc>, id: u32, at: usize| -> Result<MaskAlloc, String> {
        if id == gputrace::NO_MASK {
            Err(format!("prim {at}: masked prim references NO_MASK (mask was never recorded)"))
        } else if !matches!(
            list.prims.get(id as usize),
            Some(Prim::PolygonMask { .. }) | Some(Prim::EllipseMask { .. })
        ) {
            Err(format!("prim {at}: mask id {id} does not name a mask prim"))
        } else if let Some(a) = allocs.get(&id) {
            Ok(*a)
        } else {
            Err(format!("prim {at}: mask prim {id} is recorded after its consumer"))
        }
    };

    for (i, prim) in list.prims.iter().enumerate() {
        let prep = match prim {
            Prim::FillRect { x, y, w, h, col } => {
                let bbox = cpu_bbox(cw, ch, x - 1.0, y - 1.0, x + w + 1.0, y + h + 1.0);
                let mut p = PBuf::new(cw, ch, bbox);
                p.col(*col);
                p.p(0, [x + w / 2.0, y + h / 2.0, w / 2.0, h / 2.0]);
                PrepPrim { kind: "fill_rect", entry: "main_rect", params: p.b, pts: Vec::new(), plate: None, mask_prim: None, bbox, groups: groups_of(bbox) }
            }
            Prim::FillEllipse { cx, cy, rx, ry, rot, col }
            | Prim::FillEllipseMasked { cx, cy, rx, ry, rot, col, .. } => {
                let masked = matches!(prim, Prim::FillEllipseMasked { .. });
                let (ex, ey) = ellipse_extents(*rx, *ry, *rot);
                let bbox = cpu_bbox(cw, ch, cx - ex - 1.0, cy - ey - 1.0, cx + ex + 1.0, cy + ey + 1.0);
                let mut p = PBuf::new(cw, ch, bbox);
                p.col(*col);
                p.p(0, [*cx, *cy, *rx, *ry]);
                let (s, c) = rot.sin_cos();
                p.p(1, [s, c, 0.0, 0.0]);
                let (mask_prim, kind) = if masked {
                    let Prim::FillEllipseMasked { mask, .. } = prim else { unreachable!() };
                    let a = mask_scope(&allocs, *mask, i)?;
                    p.mk([a.ox, a.oy, a.w as i32, a.h as i32]);
                    p.u0([0, 0, 0, 1]);
                    (Some(*mask), "fill_ellipse_masked")
                } else {
                    (None, "fill_ellipse")
                };
                PrepPrim { kind, entry: "main_ellipse", params: p.b, pts: Vec::new(), plate: None, mask_prim, bbox, groups: groups_of(bbox) }
            }
            Prim::StrokePolyline { pts, width, col } => {
                prep_stroke("stroke_polyline", pts, *width, *col, cw, ch)
            }
            Prim::StrokeQuad { p0, p1, p2, width, col } => {
                // CPU: stroke_polyline(flatten_quad(p0, p1, p2, 24)).
                let pts = crate::scene::flatten_quad(*p0, *p1, *p2, 24);
                prep_stroke("stroke_quad", &pts, *width, *col, cw, ch)
            }
            Prim::StrokeEllipse { cx, cy, rx, ry, rot, col, width } => {
                // CPU: flatten the perimeter (n = 144, host-side
                // f32 sin/cos per point — replicated verbatim so
                // the points are bit-identical), then stroke.
                let n = 144usize;
                let (s, c) = rot.sin_cos();
                let pts: Vec<(f32, f32)> = (0..=n)
                    .map(|k| {
                        let a = (k as f32 / n as f32) * std::f32::consts::TAU;
                        let (lx, ly) = (rx * a.cos(), ry * a.sin());
                        (cx + lx * c - ly * s, cy + lx * s + ly * c)
                    })
                    .collect();
                prep_stroke("stroke_ellipse", &pts, *width, *col, cw, ch)
            }
            Prim::FillPolygon { pts, col } => {
                let (mnx, mny, mxx, mxy) = aabb(pts);
                let bbox = cpu_bbox(cw, ch, mnx - 1.0, mny - 1.0, mxx + 1.0, mxy + 1.0);
                let mut p = PBuf::new(cw, ch, bbox);
                p.col(*col);
                p.u0([pts.len() as u32, 0, 0, 0]);
                PrepPrim { kind: "fill_polygon", entry: "main_fillpoly", params: p.b, pts: pts_flat(pts), plate: None, mask_prim: None, bbox, groups: groups_of(bbox) }
            }
            Prim::PolygonMask { pts } => {
                let (mnx, mny, mxx, mxy) = aabb(pts);
                let bbox = cpu_bbox(cw, ch, mnx, mny, mxx, mxy);
                let alloc = MaskAlloc {
                    ox: bbox.0,
                    oy: bbox.1,
                    w: (bbox.2 - bbox.0 + 1).max(0) as u32,
                    h: (bbox.3 - bbox.1 + 1).max(0) as u32,
                };
                allocs.insert(i as u32, alloc);
                let mut p = PBuf::new(cw, ch, bbox);
                p.u0([pts.len() as u32, 0, 0, 0]);
                p.mk([alloc.ox, alloc.oy, alloc.w as i32, alloc.h as i32]);
                PrepPrim { kind: "polygon_mask", entry: "main_mask_poly", params: p.b, pts: pts_flat(pts), plate: None, mask_prim: Some(i as u32), bbox, groups: groups_of(bbox) }
            }
            Prim::EllipseMask { cx, cy, rx, ry, rot } => {
                let (ex, ey) = ellipse_extents(*rx, *ry, *rot);
                let bbox = cpu_bbox(cw, ch, cx - ex, cy - ey, cx + ex, cy + ey);
                let alloc = MaskAlloc {
                    ox: bbox.0,
                    oy: bbox.1,
                    w: (bbox.2 - bbox.0 + 1).max(0) as u32,
                    h: (bbox.3 - bbox.1 + 1).max(0) as u32,
                };
                allocs.insert(i as u32, alloc);
                let mut p = PBuf::new(cw, ch, bbox);
                p.p(0, [*cx, *cy, *rx, *ry]);
                let (s, c) = rot.sin_cos();
                p.p(1, [s, c, 0.0, 0.0]);
                p.mk([alloc.ox, alloc.oy, alloc.w as i32, alloc.h as i32]);
                PrepPrim { kind: "ellipse_mask", entry: "main_mask_ellipse", params: p.b, pts: Vec::new(), plate: None, mask_prim: Some(i as u32), bbox, groups: groups_of(bbox) }
            }
            Prim::FillPolygonMasked { pts, col, mask } => {
                let a = mask_scope(&allocs, *mask, i)?;
                let (mnx, mny, mxx, mxy) = aabb(pts);
                let bbox = cpu_bbox(cw, ch, mnx, mny, mxx, mxy);
                let mut p = PBuf::new(cw, ch, bbox);
                p.col(*col);
                p.u0([pts.len() as u32, 0, 0, 0]);
                p.mk([a.ox, a.oy, a.w as i32, a.h as i32]);
                PrepPrim { kind: "fill_polygon_masked", entry: "main_fillpoly_masked", params: p.b, pts: pts_flat(pts), plate: None, mask_prim: Some(*mask), bbox, groups: groups_of(bbox) }
            }
            Prim::MaskedBlend { rect, mask, col } => {
                let a = mask_scope(&allocs, *mask, i)?;
                // The rect is the CPU loop's own integer bbox;
                // blend() bounds-checks per pixel on the CPU, so
                // clamping the dispatch to the canvas is exact.
                let bbox = (
                    rect.0.max(0),
                    rect.1.max(0),
                    rect.2.min(cw as i32 - 1),
                    rect.3.min(ch as i32 - 1),
                );
                let mut p = PBuf::new(cw, ch, bbox);
                p.col(*col);
                p.mk([a.ox, a.oy, a.w as i32, a.h as i32]);
                PrepPrim { kind: "masked_blend", entry: "main_masked_blend", params: p.b, pts: Vec::new(), plate: None, mask_prim: Some(*mask), bbox, groups: groups_of(bbox) }
            }
            Prim::HeadShade { form, head_c, head_r, circ_c, circ_r, rot, rad, col } => {
                let bbox = cpu_bbox(cw, ch, head_c.0 - rad, head_c.1 - rad, head_c.0 + rad, head_c.1 + rad);
                let mut p = PBuf::new(cw, ch, bbox);
                p.col(*col);
                p.p(0, [head_c.0, head_c.1, head_r.0, head_r.1]);
                let (s, c) = rot.sin_cos();
                p.p(1, [s, c, 0.0, 0.0]);
                p.p(2, [circ_c.0, circ_c.1, circ_r.0, circ_r.1]);
                p.u0([*form as u32, 0, 0, 0]);
                PrepPrim { kind: "head_shade", entry: "main_headshade", params: p.b, pts: Vec::new(), plate: None, mask_prim: None, bbox, groups: groups_of(bbox) }
            }
            Prim::CompositePlate { plate, hand, lean_rad, anchor } => {
                let pd = list
                    .plates
                    .get(*plate)
                    .ok_or_else(|| format!("prim {i}: plate index {plate} out of range"))?;
                if pd.px.len() != (pd.w as usize) * (pd.h as usize) * 4 {
                    return Err(format!("prim {i}: plate pixel data size mismatch"));
                }
                let (s, c) = lean_rad.sin_cos();
                let (ax, ay) = *anchor;
                // toonfull::composite_plate's forward map, verbatim.
                let fwd = |lx: f32, ly: f32| -> (f32, f32) {
                    let (dx, dy) = (lx - 256.0, ly - 1000.0);
                    let (rx, ry) = (256.0 + dx * c - dy * s, 1000.0 + dx * s + dy * c);
                    ((rx + ax + 384.0) * PLATE_RES, (ry + ay + 256.0) * PLATE_RES)
                };
                let corners = [
                    fwd(hand.0 + pd.x0, hand.1 + pd.y0),
                    fwd(hand.0 + pd.x0 + pd.w as f32 / PLATE_RES, hand.1 + pd.y0),
                    fwd(hand.0 + pd.x0, hand.1 + pd.y0 + pd.h as f32 / PLATE_RES),
                    fwd(hand.0 + pd.x0 + pd.w as f32 / PLATE_RES, hand.1 + pd.y0 + pd.h as f32 / PLATE_RES),
                ];
                let (mut mnx, mut mny, mut mxx, mut mxy) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
                for &(x, y) in &corners {
                    mnx = mnx.min(x);
                    mny = mny.min(y);
                    mxx = mxx.max(x);
                    mxy = mxy.max(y);
                }
                let bbox = cpu_bbox(cw, ch, mnx, mny, mxx, mxy);
                let mut p = PBuf::new(cw, ch, bbox);
                p.p(0, [hand.0, hand.1, pd.x0, pd.y0]);
                p.p(1, [s, c, ax, ay]);
                p.p(2, [pd.w as f32, pd.h as f32, PLATE_RES, 0.0]);
                p.u0([pd.w, pd.h, 0, 0]);
                let packed: Vec<u32> = pd
                    .px
                    .chunks_exact(4)
                    .map(|q| u32::from_le_bytes([q[0], q[1], q[2], q[3]]))
                    .collect();
                PrepPrim { kind: "composite_plate", entry: "main_plate", params: p.b, pts: Vec::new(), plate: Some(packed), mask_prim: None, bbox, groups: groups_of(bbox) }
            }
        };
        out.push(prep);
    }
    Ok((out, allocs))
}

// SHA-256 (hand-rolled; the crate has no hashing dependency).
// Used only for the printed pixel hash the parity harness
// compares across runs (replay twice -> identical hash).
fn sha256_hex(data: &[u8]) -> String {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
        0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
        0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
        0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
        0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
        0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
        0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
    ];
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
    ];
    let bitlen = (data.len() as u64) * 8;
    let mut msg = data.to_vec();
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bitlen.to_be_bytes());
    for chunk in msg.chunks_exact(64) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([chunk[i * 4], chunk[i * 4 + 1], chunk[i * 4 + 2], chunk[i * 4 + 3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16].wrapping_add(s0).wrapping_add(w[i - 7]).wrapping_add(s1);
        }
        let (mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh) =
            (h[0], h[1], h[2], h[3], h[4], h[5], h[6], h[7]);
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let t1 = hh.wrapping_add(s1).wrapping_add(ch).wrapping_add(K[i]).wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);
            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(hh);
    }
    let mut out = String::with_capacity(64);
    for v in h {
        out.push_str(&format!("{v:08x}"));
    }
    out
}

struct ReplayOutcome {
    px: Vec<u8>,
    gpu_ms: f64,
    adapter_line: String,
    dispatches: u32,
}

async fn replay_gpu(
    list: &gputrace::DisplayList,
    preps: &[PrepPrim],
    allocs: &HashMap<u32, MaskAlloc>,
) -> Result<ReplayOutcome, String> {
    let (cw, ch) = (list.canvas_w, list.canvas_h);
    let npix = (cw as usize) * (ch as usize);
    let instance = crate::gpucore::new_instance();
    let adapter = crate::gpucore::request_adapter(&instance)
        .await
        .map_err(|e| e.to_string())?;
    let adapter_line = crate::adapter::report_line(&crate::adapter::describe(&adapter.get_info()));
    let (device, queue) = crate::gpucore::request_device_default(&adapter, "gpupaint replay")
        .await
        .map_err(|e| e.to_string())?;

    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("gpupaint"),
        source: wgpu::ShaderSource::Wgsl(WGSL_SRC.into()),
    });
    let storage = |read_only: bool| wgpu::BindGroupLayoutEntry {
        binding: 0,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only },
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    };
    let mut e0 = storage(false);
    e0.binding = 0;
    let mut e1 = storage(false);
    e1.binding = 1;
    e1.ty = wgpu::BindingType::Buffer {
        ty: wgpu::BufferBindingType::Uniform,
        has_dynamic_offset: false,
        min_binding_size: None,
    };
    let mut e2 = storage(true);
    e2.binding = 2;
    let mut e3 = storage(false);
    e3.binding = 3;
    let mut e4 = storage(true);
    e4.binding = 4;
    let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("gpupaint-bgl"),
        entries: &[e0, e1, e2, e3, e4],
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("gpupaint-layout"),
        bind_group_layouts: &[Some(&bgl)],
        immediate_size: 0,
    });
    let mut pipelines: HashMap<&'static str, wgpu::ComputePipeline> = HashMap::new();
    let pipeline_for = |pipelines: &mut HashMap<&'static str, wgpu::ComputePipeline>, entry: &'static str| {
        if !pipelines.contains_key(entry) {
            let p = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(entry),
                layout: Some(&layout),
                module: &module,
                entry_point: Some(entry),
                compilation_options: Default::default(),
                cache: None,
            });
            pipelines.insert(entry, p);
        }
    };

    let canvas = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("gpupaint canvas"),
        size: (npix * 4) as u64,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false, // wgpu zero-initialises: the CPU Painter::new zero start
    });
    let dummy = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("gpupaint dummy"),
        size: 4,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    // Bbox-scoped mask buffers, created lazily on first bind
    // (zero-initialised = the CPU mask's false start).
    let mut mask_bufs: HashMap<u32, wgpu::Buffer> = HashMap::new();
    let mask_buf_for = |mask_bufs: &mut HashMap<u32, wgpu::Buffer>, id: u32| {
        if !mask_bufs.contains_key(&id) {
            let a = allocs[&id];
            let b = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("gpupaint mask"),
                size: ((a.w as usize) * (a.h as usize) * 4).max(4) as u64,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            mask_bufs.insert(id, b);
        }
    };

    let upload = |data: &[u8], label: &'static str| -> wgpu::Buffer {
        let b = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size: data.len() as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&b, 0, data);
        b
    };

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("gpupaint replay"),
    });
    let mut dispatches = 0u32;
    for prep in preps {
        if prep.groups == (0, 0) {
            continue;
        }
        pipeline_for(&mut pipelines, prep.entry);
        let params_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("gpupaint params"),
            size: prep.params.len() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&params_buf, 0, &prep.params);
        let pts_buf;
        let pts_ref: &wgpu::Buffer = if prep.pts.is_empty() {
            &dummy
        } else {
            let mut bytes = Vec::with_capacity(prep.pts.len() * 4);
            for f in &prep.pts {
                bytes.extend_from_slice(&f.to_ne_bytes());
            }
            pts_buf = upload(&bytes, "gpupaint pts");
            &pts_buf
        };
        let plate_buf;
        let plate_ref: &wgpu::Buffer = if let Some(packed) = &prep.plate {
            let mut bytes = Vec::with_capacity(packed.len() * 4);
            for v in packed {
                bytes.extend_from_slice(&v.to_ne_bytes());
            }
            plate_buf = upload(&bytes, "gpupaint plate");
            &plate_buf
        } else {
            &dummy
        };
        let mask_ref: &wgpu::Buffer = if let Some(id) = prep.mask_prim {
            mask_buf_for(&mut mask_bufs, id);
            &mask_bufs[&id]
        } else {
            &dummy
        };
        let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("gpupaint bg"),
            layout: &bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: canvas.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: params_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: pts_ref.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 3, resource: mask_ref.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 4, resource: plate_ref.as_entire_binding() },
            ],
        });
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some(prep.entry),
            timestamp_writes: None,
        });
        pass.set_pipeline(&pipelines[prep.entry]);
        pass.set_bind_group(0, &bg, &[]);
        pass.dispatch_workgroups(prep.groups.0, prep.groups.1, 1);
        drop(pass);
        dispatches += 1;
    }

    // GPU wall time: queue submit -> completion (readback excluded).
    let t0 = std::time::Instant::now();
    queue.submit(Some(encoder.finish()));
    device
        .poll(wgpu::PollType::Wait { submission_index: None, timeout: None })
        .map_err(|e| format!("device poll: {e}"))?;
    let gpu_ms = t0.elapsed().as_secs_f64() * 1000.0;

    // Harness-only readback (production canvas stays on the GPU).
    let staging = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("gpupaint readback"),
        size: (npix * 4) as u64,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut enc2 = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("gpupaint readback"),
    });
    enc2.copy_buffer_to_buffer(&canvas, 0, &staging, 0, (npix * 4) as u64);
    queue.submit(Some(enc2.finish()));
    let slice = staging.slice(..);
    let (tx, rx) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |r| {
        let _ = tx.send(r);
    });
    device
        .poll(wgpu::PollType::Wait { submission_index: None, timeout: None })
        .map_err(|e| format!("device poll (readback): {e}"))?;
    rx.recv()
        .map_err(|_| "readback: map channel closed".to_string())?
        .map_err(|e| format!("readback: map failed: {e}"))?;
    let data = slice
        .get_mapped_range()
        .map_err(|e| format!("readback: {e}"))?;
    let px = data.to_vec();
    drop(data);
    staging.unmap();
    // The canvas buffer packs R in the low byte of each u32; its
    // memory image is RGBA on little-endian (all supported
    // targets). Convert explicitly on big-endian hosts.
    #[cfg(target_endian = "big")]
    let px = {
        let mut px = px;
        for q in px.chunks_exact_mut(4) {
            q.reverse();
        }
        px
    };
    Ok(ReplayOutcome { px, gpu_ms, adapter_line, dispatches })
}

fn replay(list_path: &str, out_path: &str) -> i32 {
    let text = match std::fs::read_to_string(list_path) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("error: could not read {list_path}: {e}");
            return 2;
        }
    };
    let list = match gputrace::DisplayList::from_json(&text) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("error: {list_path}: {e}");
            return 2;
        }
    };
    if list.canvas_w == 0 || list.canvas_h == 0 {
        eprintln!("error: {list_path}: canvas size is zero");
        return 2;
    }
    let (preps, allocs) = match prepare(&list) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("error: replay: {e}");
            return 3;
        }
    };
    match pollster::block_on(replay_gpu(&list, &preps, &allocs)) {
        Ok(outcome) => {
            println!("{}", outcome.adapter_line);
            for (i, p) in preps.iter().enumerate() {
                println!(
                    "prim {i} {}: dispatch {}x{} workgroups ({}x{}), bbox ({},{})-({},{})",
                    p.kind,
                    p.groups.0,
                    p.groups.1,
                    WGSZ,
                    WGSZ,
                    p.bbox.0,
                    p.bbox.1,
                    p.bbox.2,
                    p.bbox.3
                );
            }
            println!(
                "replay: {} prims, {} dispatches, canvas {}x{}",
                preps.len(),
                outcome.dispatches,
                list.canvas_w,
                list.canvas_h
            );
            println!("gpu_wall_ms: {:.3}", outcome.gpu_ms);
            println!("sha256: {}", sha256_hex(&outcome.px));
            match save_png(&outcome.px, list.canvas_w, list.canvas_h, out_path) {
                Ok(()) => 0,
                Err(e) => {
                    eprintln!("error: {e}");
                    e.exit_code()
                }
            }
        }
        Err(e) => {
            eprintln!("error: replay: {e}");
            3
        }
    }
}

#[cfg(test)]
mod exec_tests {
    use super::*;

    #[test]
    fn sha256_known_vectors() {
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae3039dae2007cb0ac9239ac9fb8f95d5cd60a"
        );
        // 56-byte message: exercises the two-block padding path.
        assert_eq!(
            sha256_hex(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"),
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
        );
    }

    #[test]
    fn prepare_rejects_no_mask_and_forward_refs() {
        let mk_list = |prims: Vec<Prim>| gputrace::DisplayList {
            canvas_w: 64,
            canvas_h: 64,
            plates: Vec::new(),
            prims,
        };
        // NO_MASK must fail loudly.
        let list = mk_list(vec![Prim::FillPolygonMasked {
            pts: vec![(1.0, 1.0), (9.0, 1.0), (9.0, 9.0)],
            col: (0, 0, 0, 255),
            mask: gputrace::NO_MASK,
        }]);
        assert!(prepare(&list).is_err());
        // A mask recorded before its consumer prepares fine, and
        // the alloc is the mask's own bbox, not the canvas.
        let list = mk_list(vec![
            Prim::PolygonMask { pts: vec![(10.0, 10.0), (20.0, 10.0), (20.0, 20.0)] },
            Prim::FillPolygonMasked {
                pts: vec![(1.0, 1.0), (9.0, 1.0), (9.0, 9.0)],
                col: (0, 0, 0, 255),
                mask: 0,
            },
        ]);
        let (_, allocs) = prepare(&list).expect("prepare");
        let a = allocs[&0u32];
        assert_eq!((a.ox, a.oy, a.w, a.h), (10, 10, 11, 11));
    }
}
