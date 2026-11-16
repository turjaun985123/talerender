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

/// WGSL `vec4<u32>` with the WGSL uniform alignment (16 bytes).
/// The align(16) wrapper is what makes the repr(C) `Params`
/// layout below land every vec4 member on a 16-byte boundary,
/// exactly as the WGSL uniform layout rules place them.
#[repr(C, align(16))]
#[derive(Clone, Copy, Default)]
struct Vec4u([u32; 4]);

/// WGSL `vec4<f32>` with the WGSL uniform alignment (16 bytes).
#[repr(C, align(16))]
#[derive(Clone, Copy, Default)]
struct Vec4f([f32; 4]);

/// WGSL `vec4<i32>` with the WGSL uniform alignment (16 bytes).
#[repr(C, align(16))]
#[derive(Clone, Copy, Default)]
struct Vec4i([i32; 4]);

/// The 144-byte uniform block: a #[repr(C)] Rust mirror of the
/// WGSL `Params` struct in src/gpupaint.wgsl, field-for-field
/// and in field order (statically verified by
/// tools/ci/check_shaders.py, pair src/gpupaint.wgsl:Params,
/// rust_struct dialect). Layout: six scalars (24 B), implicit
/// padding to 32 (the first vec4's align(16)), then six vec4
/// members at 32/48/64/80/96/112/128; total size 144 B — the
/// offsets and size are asserted in exec_tests, and the golden
/// params bytes there pin the produced bytes per prim kind.
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct Params {
    canvas_w: u32,
    canvas_h: u32,
    bx0: i32,
    by0: i32,
    bx1: i32,
    by1: i32,
    col: Vec4u,
    p0: Vec4f,
    p1: Vec4f,
    p2: Vec4f,
    p3: Vec4f,
    u0: Vec4u,
    mk: Vec4i,
}

impl Params {
    fn new(canvas_w: u32, canvas_h: u32, bbox: (i32, i32, i32, i32)) -> Self {
        Params {
            canvas_w,
            canvas_h,
            bx0: bbox.0,
            by0: bbox.1,
            bx1: bbox.2,
            by1: bbox.3,
            ..Default::default()
        }
    }
    fn col(&mut self, c: crate::scene::Rgba) {
        self.col = Vec4u([c.0 as u32, c.1 as u32, c.2 as u32, c.3 as u32]);
    }
    /// p0..p3, selected by index (the prep code's slot number).
    fn p(&mut self, idx: usize, v: [f32; 4]) {
        match idx {
            0 => self.p0 = Vec4f(v),
            1 => self.p1 = Vec4f(v),
            2 => self.p2 = Vec4f(v),
            _ => self.p3 = Vec4f(v),
        }
    }
    fn u0(&mut self, v: [u32; 4]) {
        self.u0 = Vec4u(v);
    }
    fn mk(&mut self, v: [i32; 4]) {
        self.mk = Vec4i(v);
    }
    /// The uniform bytes: each field serialised at its own
    /// repr(C) offset (std::mem::offset_of!, asserted against
    /// the WGSL offsets in exec_tests) into a zeroed block, so
    /// the padding bytes (24..32) are zero by construction and
    /// the struct definition is the single layout source.
    fn bytes(&self) -> [u8; 144] {
        fn w32(b: &mut [u8; 144], off: usize, v: u32) {
            b[off..off + 4].copy_from_slice(&v.to_ne_bytes());
        }
        fn w32s(b: &mut [u8; 144], off: usize, vs: &[u32; 4]) {
            for (i, x) in vs.iter().enumerate() {
                w32(b, off + i * 4, *x);
            }
        }
        fn wi32s(b: &mut [u8; 144], off: usize, vs: &[i32; 4]) {
            for (i, x) in vs.iter().enumerate() {
                b[off + i * 4..off + i * 4 + 4].copy_from_slice(&x.to_ne_bytes());
            }
        }
        fn wf32s(b: &mut [u8; 144], off: usize, vs: &[f32; 4]) {
            for (i, x) in vs.iter().enumerate() {
                b[off + i * 4..off + i * 4 + 4].copy_from_slice(&x.to_ne_bytes());
            }
        }
        let mut b = [0u8; 144];
        w32(&mut b, std::mem::offset_of!(Params, canvas_w), self.canvas_w);
        w32(&mut b, std::mem::offset_of!(Params, canvas_h), self.canvas_h);
        w32(&mut b, std::mem::offset_of!(Params, bx0), self.bx0 as u32);
        w32(&mut b, std::mem::offset_of!(Params, by0), self.by0 as u32);
        w32(&mut b, std::mem::offset_of!(Params, bx1), self.bx1 as u32);
        w32(&mut b, std::mem::offset_of!(Params, by1), self.by1 as u32);
        w32s(&mut b, std::mem::offset_of!(Params, col), &self.col.0);
        wf32s(&mut b, std::mem::offset_of!(Params, p0), &self.p0.0);
        wf32s(&mut b, std::mem::offset_of!(Params, p1), &self.p1.0);
        wf32s(&mut b, std::mem::offset_of!(Params, p2), &self.p2.0);
        wf32s(&mut b, std::mem::offset_of!(Params, p3), &self.p3.0);
        w32s(&mut b, std::mem::offset_of!(Params, u0), &self.u0.0);
        wi32s(&mut b, std::mem::offset_of!(Params, mk), &self.mk.0);
        b
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
        let mut p = Params::new(cw, ch, bbox);
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
            params: base(bbox).bytes(),
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
    // p2 = (fan_start.r, fan_start.cos_half, fan_end.c) per the
    // WGSL stroke layout; the two fans each own half of the slot.
    if let Some(f) = &fan_start {
        p.p(1, [f.c.0, f.c.1, f.axis.0, f.axis.1]);
        p.p2.0[0] = f.r;
        p.p2.0[1] = f.cos_half;
        flags |= 1;
    }
    if let Some(f) = &fan_end {
        p.p2.0[2] = f.c.0;
        p.p2.0[3] = f.c.1;
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
        params: p.bytes(),
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
                let mut p = Params::new(cw, ch, bbox);
                p.col(*col);
                p.p(0, [x + w / 2.0, y + h / 2.0, w / 2.0, h / 2.0]);
                PrepPrim { kind: "fill_rect", entry: "main_rect", params: p.bytes(), pts: Vec::new(), plate: None, mask_prim: None, bbox, groups: groups_of(bbox) }
            }
            Prim::FillEllipse { cx, cy, rx, ry, rot, col }
            | Prim::FillEllipseMasked { cx, cy, rx, ry, rot, col, .. } => {
                let masked = matches!(prim, Prim::FillEllipseMasked { .. });
                let (ex, ey) = ellipse_extents(*rx, *ry, *rot);
                let bbox = cpu_bbox(cw, ch, cx - ex - 1.0, cy - ey - 1.0, cx + ex + 1.0, cy + ey + 1.0);
                let mut p = Params::new(cw, ch, bbox);
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
                PrepPrim { kind, entry: "main_ellipse", params: p.bytes(), pts: Vec::new(), plate: None, mask_prim, bbox, groups: groups_of(bbox) }
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
                let mut p = Params::new(cw, ch, bbox);
                p.col(*col);
                p.u0([pts.len() as u32, 0, 0, 0]);
                PrepPrim { kind: "fill_polygon", entry: "main_fillpoly", params: p.bytes(), pts: pts_flat(pts), plate: None, mask_prim: None, bbox, groups: groups_of(bbox) }
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
                let mut p = Params::new(cw, ch, bbox);
                p.u0([pts.len() as u32, 0, 0, 0]);
                p.mk([alloc.ox, alloc.oy, alloc.w as i32, alloc.h as i32]);
                PrepPrim { kind: "polygon_mask", entry: "main_mask_poly", params: p.bytes(), pts: pts_flat(pts), plate: None, mask_prim: Some(i as u32), bbox, groups: groups_of(bbox) }
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
                let mut p = Params::new(cw, ch, bbox);
                p.p(0, [*cx, *cy, *rx, *ry]);
                let (s, c) = rot.sin_cos();
                p.p(1, [s, c, 0.0, 0.0]);
                p.mk([alloc.ox, alloc.oy, alloc.w as i32, alloc.h as i32]);
                PrepPrim { kind: "ellipse_mask", entry: "main_mask_ellipse", params: p.bytes(), pts: Vec::new(), plate: None, mask_prim: Some(i as u32), bbox, groups: groups_of(bbox) }
            }
            Prim::FillPolygonMasked { pts, col, mask } => {
                let a = mask_scope(&allocs, *mask, i)?;
                let (mnx, mny, mxx, mxy) = aabb(pts);
                let bbox = cpu_bbox(cw, ch, mnx, mny, mxx, mxy);
                let mut p = Params::new(cw, ch, bbox);
                p.col(*col);
                p.u0([pts.len() as u32, 0, 0, 0]);
                p.mk([a.ox, a.oy, a.w as i32, a.h as i32]);
                PrepPrim { kind: "fill_polygon_masked", entry: "main_fillpoly_masked", params: p.bytes(), pts: pts_flat(pts), plate: None, mask_prim: Some(*mask), bbox, groups: groups_of(bbox) }
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
                let mut p = Params::new(cw, ch, bbox);
                p.col(*col);
                p.mk([a.ox, a.oy, a.w as i32, a.h as i32]);
                PrepPrim { kind: "masked_blend", entry: "main_masked_blend", params: p.bytes(), pts: Vec::new(), plate: None, mask_prim: Some(*mask), bbox, groups: groups_of(bbox) }
            }
            Prim::HeadShade { form, head_c, head_r, circ_c, circ_r, rot, rad, col } => {
                let bbox = cpu_bbox(cw, ch, head_c.0 - rad, head_c.1 - rad, head_c.0 + rad, head_c.1 + rad);
                let mut p = Params::new(cw, ch, bbox);
                p.col(*col);
                p.p(0, [head_c.0, head_c.1, head_r.0, head_r.1]);
                let (s, c) = rot.sin_cos();
                p.p(1, [s, c, 0.0, 0.0]);
                p.p(2, [circ_c.0, circ_c.1, circ_r.0, circ_r.1]);
                p.u0([*form as u32, 0, 0, 0]);
                PrepPrim { kind: "head_shade", entry: "main_headshade", params: p.bytes(), pts: Vec::new(), plate: None, mask_prim: None, bbox, groups: groups_of(bbox) }
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
                let mut p = Params::new(cw, ch, bbox);
                p.p(0, [hand.0, hand.1, pd.x0, pd.y0]);
                p.p(1, [s, c, ax, ay]);
                p.p(2, [pd.w as f32, pd.h as f32, PLATE_RES, 0.0]);
                p.u0([pd.w, pd.h, 0, 0]);
                let packed: Vec<u32> = pd
                    .px
                    .chunks_exact(4)
                    .map(|q| u32::from_le_bytes([q[0], q[1], q[2], q[3]]))
                    .collect();
                PrepPrim { kind: "composite_plate", entry: "main_plate", params: p.bytes(), pts: Vec::new(), plate: Some(packed), mask_prim: None, bbox, groups: groups_of(bbox) }
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
    // Literal entry list (the shape tools/ci/check_shaders.py
    // models): binding 0 canvas storage rw, 1 Params uniform,
    // 2 pts storage ro, 3 mask storage rw, 4 plate storage ro —
    // the same five entries the previous closure+mutation code
    // assembled, in the same order with the same flags.
    let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("gpupaint-bgl"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 3,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 4,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("gpupaint-layout"),
        bind_group_layouts: &[Some(&bgl)],
        immediate_size: 0,
    });
    let mut pipelines: HashMap<&'static str, wgpu::ComputePipeline> = HashMap::new();

    let canvas = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("gpupaint canvas"),
        size: (npix * 4) as u64,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false, // wgpu zero-initialises: the CPU Painter::new zero start
    });
    crate::gpucore::mem_alloc_buffer(crate::gpucore::MemClass::G, (npix * 4) as u64); // VRAM ledger (W84 §5.2)
    let dummy = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("gpupaint dummy"),
        size: 4,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    crate::gpucore::mem_alloc_buffer(crate::gpucore::MemClass::G, 4); // VRAM ledger (W84 §5.2)
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
            crate::gpucore::mem_alloc_buffer(crate::gpucore::MemClass::G, ((a.w as usize) * (a.h as usize) * 4).max(4) as u64); // VRAM ledger (W84 §5.2)
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
        crate::gpucore::mem_alloc_buffer(crate::gpucore::MemClass::G, data.len() as u64); // VRAM ledger (W84 §5.2)
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
        pipeline_for(&mut pipelines, &device, &layout, &module, prep.entry);
        let params_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("gpupaint params"),
            size: prep.params.len() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        crate::gpucore::mem_alloc_buffer(crate::gpucore::MemClass::G, prep.params.len() as u64); // VRAM ledger (W84 §5.2)
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
    crate::gpucore::mem_note_readback((npix * 4) as u64); // VRAM ledger (W84 §5.2)
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

/// One compute pipeline for `entry`, a @compute entry point of
/// gpupaint.wgsl. Kept as a plain fn template whose call sites
/// all pass a literal entry name (see pipeline_for below), so
/// tools/ci/check_shaders.py resolves one binding-checked site
/// per entry point.
fn compute_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    module: &wgpu::ShaderModule,
    entry: &'static str,
) -> wgpu::ComputePipeline {
    device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some(entry),
        layout: Some(layout),
        module: module,
        entry_point: Some(entry),
        compilation_options: Default::default(),
        cache: None,
    })
}

/// Lazily create (once per entry) the pipeline for `entry` in
/// `pipelines` — the same lazy HashMap cache the previous
/// closure in replay_gpu maintained. The match arms are the
/// ONLY call sites of compute_pipeline and each one passes its
/// entry name literally; `prepare` only ever emits these ten
/// entry names, so the panic arm is unreachable from a prepared
/// list (it replaces wgpu's own validation error for a bogus
/// entry with a louder, earlier one).
fn pipeline_for(
    pipelines: &mut HashMap<&'static str, wgpu::ComputePipeline>,
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    module: &wgpu::ShaderModule,
    entry: &'static str,
) {
    if pipelines.contains_key(entry) {
        return;
    }
    let p = match entry {
        "main_rect" => compute_pipeline(device, layout, module, "main_rect"),
        "main_ellipse" => compute_pipeline(device, layout, module, "main_ellipse"),
        "main_stroke" => compute_pipeline(device, layout, module, "main_stroke"),
        "main_fillpoly" => compute_pipeline(device, layout, module, "main_fillpoly"),
        "main_mask_poly" => compute_pipeline(device, layout, module, "main_mask_poly"),
        "main_mask_ellipse" => compute_pipeline(device, layout, module, "main_mask_ellipse"),
        "main_fillpoly_masked" => {
            compute_pipeline(device, layout, module, "main_fillpoly_masked")
        }
        "main_masked_blend" => compute_pipeline(device, layout, module, "main_masked_blend"),
        "main_headshade" => compute_pipeline(device, layout, module, "main_headshade"),
        "main_plate" => compute_pipeline(device, layout, module, "main_plate"),
        _ => panic!("gpupaint: unknown entry point {entry}"),
    };
    pipelines.insert(entry, p);
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
        // The expected value here was previously corrupted past
        // the "...5dae" prefix; the implementation's output was
        // verified byte-for-byte against system sha256sum /
        // hashlib across empty, padding-boundary (55/56/57),
        // block-boundary (64/65) and multi-block inputs
        // (incl. the NIST 1,000,000 x 'a' vector) — the
        // implementation was correct, this constant was not.
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        // 56-byte message: exercises the two-block padding path.
        assert_eq!(
            sha256_hex(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"),
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
        );
        // Multi-block: 1,000 x 'a' (16 blocks), cross-checked
        // against system sha256sum on the same bytes.
        assert_eq!(
            sha256_hex(&vec![b'a'; 1000]),
            "41edece42d63e8d9bf515a9ba6932e1c20cbc9f5a5d134645adb5db1b9737ea3"
        );
    }

    /// Golden params bytes (seat FIX-SHADER-GATE): the hex
    /// table below was captured from the PRE-refactor custom
    /// byte-builder (the `p.b` offset writer this seat replaced)
    /// for exactly the lists constructed here; the #[repr(C)]
    /// Params-struct packing must reproduce every byte, for
    /// every prim kind. Also pins the struct layout against the
    /// WGSL offsets (144 B; vec4 slots at 32/48/.../128).
    #[test]
    fn params_bytes_golden() {
        assert_eq!(std::mem::size_of::<Params>(), 144);
        assert_eq!(std::mem::align_of::<Params>(), 16);
        assert_eq!(std::mem::offset_of!(Params, canvas_w), 0);
        assert_eq!(std::mem::offset_of!(Params, bx0), 8);
        assert_eq!(std::mem::offset_of!(Params, col), 32);
        assert_eq!(std::mem::offset_of!(Params, p0), 48);
        assert_eq!(std::mem::offset_of!(Params, p1), 64);
        assert_eq!(std::mem::offset_of!(Params, p2), 80);
        assert_eq!(std::mem::offset_of!(Params, p3), 96);
        assert_eq!(std::mem::offset_of!(Params, u0), 112);
        assert_eq!(std::mem::offset_of!(Params, mk), 128);
        let hex = |b: &[u8]| b.iter().map(|x| format!("{x:02x}")).collect::<String>();
        let one = |prims: Vec<Prim>, plates: Vec<gputrace::PlateData>| {
            let list = gputrace::DisplayList { canvas_w: 64, canvas_h: 64, plates, prims };
            prepare(&list).expect("prepare").0
        };
        let cases: Vec<(&str, Vec<PrepPrim>)> = vec![
            ("fill_rect", one(vec![Prim::FillRect { x: 10.5, y: 12.25, w: 30.0, h: 20.0, col: (200, 30, 40, 255) }], vec![])),
            ("fill_ellipse", one(vec![Prim::FillEllipse { cx: 32.0, cy: 30.0, rx: 12.5, ry: 8.25, rot: 0.7, col: (1, 2, 3, 200) }], vec![])),
            ("ellipse_mask", one(vec![Prim::EllipseMask { cx: 28.0, cy: 26.0, rx: 10.0, ry: 14.0, rot: -0.3 }], vec![])),
            ("masked_blend", one(vec![
                Prim::EllipseMask { cx: 28.0, cy: 26.0, rx: 10.0, ry: 14.0, rot: -0.3 },
                Prim::MaskedBlend { rect: (5, 6, 40, 44), mask: 0, col: (9, 8, 7, 255) },
            ], vec![])),
            ("head_shade0", one(vec![Prim::HeadShade { form: 0, head_c: (32.0, 32.0), head_r: (18.0, 20.0), circ_c: (40.0, 24.0), circ_r: (12.0, 9.0), rot: 0.2, rad: 22.5, col: (50, 60, 70, 128) }], vec![])),
            ("head_shade1", one(vec![Prim::HeadShade { form: 1, head_c: (32.0, 32.0), head_r: (18.0, 20.0), circ_c: (40.0, 24.0), circ_r: (15.0, 15.0), rot: 0.2, rad: 22.5, col: (50, 60, 70, 128) }], vec![])),
            ("composite_plate", one(vec![Prim::CompositePlate { plate: 0, hand: (100.0, 200.0), lean_rad: 0.35, anchor: (12.0, -8.0) }],
                vec![gputrace::PlateData { px: (0..48u8).collect(), w: 4, h: 3, x0: 1.5, y0: -2.0 }])),
            ("stroke_polyline", one(vec![Prim::StrokePolyline { pts: vec![(5.0, 5.0), (20.0, 15.0), (35.0, 10.0), (50.0, 30.0)], width: 6.5, col: (255, 128, 0, 255) }], vec![])),
            ("fill_ellipse_masked", one(vec![
                Prim::PolygonMask { pts: vec![(10.0, 10.0), (50.0, 12.0), (44.0, 50.0)] },
                Prim::FillEllipseMasked { cx: 30.0, cy: 28.0, rx: 11.0, ry: 9.0, rot: 0.4, col: (77, 88, 99, 210), mask: 0 },
            ], vec![])),
            ("fill_polygon", one(vec![Prim::FillPolygon { pts: vec![(8.0, 9.0), (40.0, 7.0), (30.0, 45.0), (12.0, 38.0)], col: (3, 4, 5, 6) }], vec![])),
            ("stroke_quad", one(vec![Prim::StrokeQuad { p0: (4.0, 50.0), p1: (30.0, 4.0), p2: (58.0, 44.0), width: 4.25, col: (11, 22, 33, 44) }], vec![])),
            ("stroke_ellipse", one(vec![Prim::StrokeEllipse { cx: 32.0, cy: 32.0, rx: 20.0, ry: 13.0, rot: 0.9, col: (90, 80, 70, 60), width: 3.5 }], vec![])),
            ("stroke_short", one(vec![Prim::StrokePolyline { pts: vec![(7.0, 7.0)], width: 5.0, col: (1, 1, 1, 1) }], vec![])),
        ];
        let golden: &[(&str, usize, &str, &str)] = &[
            ("fill_rect", 0, "fill_rect", "4000000040000000090000000b0000002a000000220000000000000000000000c80000001e00000028000000ff0000000000cc410000b24100007041000020410000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000"),
            ("fill_ellipse", 0, "fill_ellipse", "4000000040000000100000000e000000300000002e0000000000000000000000010000000200000003000000c8000000000000420000f041000048410000044173eb243fb3cc433f000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000"),
            ("ellipse_mask", 0, "ellipse_mask", "40000000400000000e000000090000002a0000002b0000000000000000000000000000000000000000000000000000000000e0410000d04100002041000060416d4e97beef90743f00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000e000000090000001d00000023000000"),
            ("masked_blend", 0, "ellipse_mask", "40000000400000000e000000090000002a0000002b0000000000000000000000000000000000000000000000000000000000e0410000d04100002041000060416d4e97beef90743f00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000e000000090000001d00000023000000"),
            ("masked_blend", 1, "masked_blend", "40000000400000000500000006000000280000002c0000000000000000000000090000000800000007000000ff00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000e000000090000001d00000023000000"),
            ("head_shade0", 0, "head_shade", "4000000040000000090000000900000037000000370000000000000000000000320000003c00000046000000800000000000004200000042000090410000a041f96f4b3ea5e57a3f0000000000000000000020420000c0410000404100001041000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000"),
            ("head_shade1", 0, "head_shade", "4000000040000000090000000900000037000000370000000000000000000000320000003c00000046000000800000000000004200000042000090410000a041f96f4b3ea5e57a3f0000000000000000000020420000c0410000704100007041000000000000000000000000000000000100000000000000000000000000000000000000000000000000000000000000"),
            ("composite_plate", 0, "composite_plate", "40000000400000001a060000730300003f0000003f0000000000000000000000000000000000000000000000000000000000c842000048430000c03f000000c04d90af3ebb7a703f00004041000000c100008040000040400000004000000000000000000000000000000000000000000400000003000000000000000000000000000000000000000000000000000000"),
            ("stroke_polyline", 0, "stroke_polyline", "4000000040000000000000000000000037000000230000000000000000000000ff0000008000000000000000ff000000000050400000000000000000000000000000a0400000a040400155bfd5000ebf000050405e836c3f000048420000f0419a99193fcdcc4c3f000050405e836c3f0400000004000000040000000300000000000000000000000000000000000000"),
            ("fill_ellipse_masked", 0, "polygon_mask", "40000000400000000a0000000a000000320000003200000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000030000000000000000000000000000000a0000000a0000002900000029000000"),
            ("fill_ellipse_masked", 1, "fill_ellipse_masked", "40000000400000000f0000000e0000002d0000002a00000000000000000000004d0000005800000063000000d20000000000f0410000e0410000304100001041d761c73ea7ca6b3f00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000010000000a0000000a0000002900000029000000"),
            ("fill_polygon", 0, "fill_polygon", "40000000400000000700000006000000290000002e000000000000000000000003000000040000000500000006000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000400000000000000000000000000000000000000000000000000000000000000"),
            ("stroke_quad", 0, "stroke_quad", "400000004000000000000000160000003e0000003600000000000000000000000b00000016000000210000002c000000000008400000000000000000000000000000804000004842ebee01bf34935c3f000008405e836c3f00006842000030429c2c173fd9984e3f000008405e836c3f1900000004000000040000000300000000000000000000000000000000000000"),
            ("stroke_ellipse", 0, "stroke_ellipse", "40000000400000000d0000000b000000330000003500000000000000000000005a00000050000000460000003c0000000000e03f0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000009100000000000000000000000000000000000000000000000000000000000000"),
            ("stroke_short", 0, "stroke_polyline", "40000000400000000000000000000000ffffffffffffffff000000000000000001000000010000000100000001000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000"),
        ];
        let mut gi = 0;
        for (name, preps) in &cases {
            for (i, p) in preps.iter().enumerate() {
                let (gname, gidx, gkind, ghex) = golden[gi];
                gi += 1;
                assert_eq!((*name, i, p.kind), (gname, gidx, gkind));
                assert_eq!(
                    hex(&p.params),
                    ghex,
                    "params bytes drifted for {name} prim {i} ({})",
                    p.kind
                );
            }
        }
        assert_eq!(gi, golden.len());
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
