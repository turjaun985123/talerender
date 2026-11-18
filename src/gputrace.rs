//! GPU-first Phase 0: the display-list trace recorder.
//!
//! Compiled ONLY under the cargo feature `gpupaint` (default OFF).
//! With the feature off this module does not exist and the painter
//! is byte-for-byte the production painter. With the feature on,
//! the recorder is inert until a harness driver arms it and opens
//! a window with `begin(..)`; every `trace_*` hook is then a cheap
//! thread-local check plus, inside a window, one `Prim` push.
//!
//! A display list is the EXACT semantic primitive stream the CPU
//! `Painter` executes between `begin` and `take`, in execution
//! order. Mask producers (`PolygonMask` / `EllipseMask`) are prims
//! too: a mask's id is its index in `prims`, and masked fills
//! reference that index. See docs/integration/GPUFIRST-TRACE.md
//! for the mask-id threading proof, the serialization round-trip
//! argument, and the exhaustiveness argument. The two open-coded
//! per-pixel loops in the character painters (the even-odd head
//! shade and the laugh-mouth teeth blit) are recorded as the
//! dedicated `HeadShade` / `MaskedBlend` prims, not dropped.
//!
//! Serialization choice (stated per the seat brief): every `f32`
//! is written as `{"b": <u32>}` — the raw `f32::to_bits()` value
//! as a decimal integer. Rust's `Display` for `f32` does
//! round-trip finite values (shortest representation), but it
//! prints `NaN` / `inf` / `-inf`, which are not JSON numbers and
//! which `scene::parse_json` cannot read back; the bits form is
//! total over all 2^32 bit patterns (NaN payloads, -0.0,
//! subnormals included) and the u32 itself is exact in JSON
//! (parsed as f64; every u32 is < 2^53). Plate pixels are
//! standard base64 (hand-rolled, no new dependencies).

use crate::scene::{Rgba, Value};
use crate::toonfull::Plate;
use std::cell::RefCell;

/// One recorded primitive, in painter execution order.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Prim {
    FillPolygon { pts: Vec<(f32, f32)>, col: Rgba },
    FillPolygonMasked { pts: Vec<(f32, f32)>, col: Rgba, mask: u32 },
    FillEllipse { cx: f32, cy: f32, rx: f32, ry: f32, rot: f32, col: Rgba },
    FillEllipseMasked { cx: f32, cy: f32, rx: f32, ry: f32, rot: f32, col: Rgba, mask: u32 },
    StrokeEllipse { cx: f32, cy: f32, rx: f32, ry: f32, rot: f32, col: Rgba, width: f32 },
    StrokePolyline { pts: Vec<(f32, f32)>, width: f32, col: Rgba },
    StrokeQuad { p0: (f32, f32), p1: (f32, f32), p2: (f32, f32), width: f32, col: Rgba },
    FillRect { x: f32, y: f32, w: f32, h: f32, col: Rgba },
    PolygonMask { pts: Vec<(f32, f32)> },
    EllipseMask { cx: f32, cy: f32, rx: f32, ry: f32, rot: f32 },
    /// B3-CARRY-1 (Batch 3): `flip` / `pitch_rad` / `hip` extend
    /// the inverse map beyond lean + anchor — the plate rides the
    /// upper assembly, so its sampling must undo the flip mirror
    /// and the bodyPitch hip-pivot rotation as well. `hand` stays
    /// the UN-pitched stage hand (the plate's figure-local origin).
    CompositePlate { plate: usize, hand: (f32, f32), lean_rad: f32, anchor: (f32, f32), flip: bool, pitch_rad: f32, hip: (f32, f32) },
    /// The even-odd head shade: an open-coded per-pixel loop in
    /// both character painters (no Painter method), recorded as
    /// its own prim so the pixels are not silently dropped.
    /// Semantics: iterate the canvas-clamped bbox of
    /// `head_c ± rad`; per pixel centre q,
    /// `cov_head = cov_of(ellipse_sd(head_c, head_r, rot, q))`
    /// (Painter::cov_of ONLY — no cov_area supersample); skip
    /// when cov_head <= 0; `cov = cov_head * unshaded`, then
    /// blend_cov(col, cov) when cov > 0. `unshaded` has TWO
    /// forms, selected by `form` — the two painters genuinely
    /// diverge here (INV-A D7) and the list must not unify them:
    ///   form 0 (toonfull): clamp(0.5 - ellipse_sd(circ_c,
    ///     circ_r, rot, q), 0, 1) — a full ellipse SD;
    ///     rad = rp*RES*1.05.
    ///   form 1 (scene::paint_toon): clamp(hypot(q - circ_c)
    ///     - circ_r.0 + 0.5, 0, 1) — a CIRCLE distance;
    ///     circ_r = (R, R) with R = rp*res (executor uses
    ///     circ_r.0); rad = rp*res + 1.0.
    HeadShade {
        form: u8,
        head_c: (f32, f32),
        head_r: (f32, f32),
        circ_c: (f32, f32),
        circ_r: (f32, f32),
        rot: f32,
        rad: f32,
        col: Rgba,
    },
    /// The laugh-mouth teeth blit: an open-coded loop in both
    /// character painters — for each pixel in `rect` (the loop's
    /// own bbox, from the two mapped corner points, NOT the
    /// mask's bbox), if the mask bitmap is set, blend `col` at
    /// full coverage.
    MaskedBlend { rect: (i32, i32, i32, i32), mask: u32, col: Rgba },
}

/// A held-prop plate's pixels, captured once per distinct plate
/// instance per display list (see `push_plate`).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PlateData {
    pub px: Vec<u8>,
    pub w: u32,
    pub h: u32,
    pub x0: f32,
    pub y0: f32,
}

/// One recorded paint: canvas size, the plates it composites,
/// and the primitive stream.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DisplayList {
    pub canvas_w: u32,
    pub canvas_h: u32,
    pub plates: Vec<PlateData>,
    pub prims: Vec<Prim>,
}

/// Mask-id sentinel for a masked fill whose mask was never
/// recorded inside the window (unreachable in the current call
/// graph — see GPUFIRST-TRACE.md §3 — but representable so the
/// executor can fail loudly instead of guessing).
pub(crate) const NO_MASK: u32 = u32::MAX;

struct Recorder {
    canvas_w: u32,
    canvas_h: u32,
    plates: Vec<PlateData>,
    /// Dedup keys for `push_plate`: (px buffer address, len, w, h,
    /// x0 bits, y0 bits) per stored plate. The address identifies
    /// the plate INSTANCE for the duration of one paint (the
    /// owning `FullState` outlives the paint), which is the
    /// "distinct plate" the brief asks for.
    plate_keys: Vec<(usize, usize, u32, u32, u32, u32)>,
    prims: Vec<Prim>,
    /// Prim index of the most recently recorded mask prim.
    last_mask_id: Option<u32>,
    /// (mask content hash, mask length, mask prim id) for every
    /// mask bitmap a masked fill has resolved. This registry is
    /// what makes mask threading correct where `last_mask_id`
    /// alone is not (repeated masked fills over one bean mask
    /// with other mask prims recorded in between — proof in
    /// GPUFIRST-TRACE.md §3).
    mask_registry: Vec<(u64, usize, u32)>,
}

thread_local! {
    static REC: RefCell<Option<Recorder>> = const { RefCell::new(None) };
    /// Armed by the `--gpu-paint` drivers only. Card paints that
    /// also run in production modes (the stroke battery) open
    /// trace windows through `begin_card`, which is a no-op
    /// unless armed — so a `gpupaint` build running
    /// `--scene stroke-battery` behaves exactly like production.
    static ARMED: RefCell<bool> = const { RefCell::new(false) };
}

pub(crate) fn set_armed(on: bool) {
    ARMED.with(|a| *a.borrow_mut() = on);
}

pub(crate) fn armed() -> bool {
    ARMED.with(|a| *a.borrow())
}

/// Open a trace window for a canvas of `w` x `h`. Any previous
/// window's state (including mask registry and last-mask) is
/// discarded: ids are per-list.
pub(crate) fn begin(w: u32, h: u32) {
    REC.with(|r| {
        *r.borrow_mut() = Some(Recorder {
            canvas_w: w,
            canvas_h: h,
            plates: Vec::new(),
            plate_keys: Vec::new(),
            prims: Vec::new(),
            last_mask_id: None,
            mask_registry: Vec::new(),
        });
    });
}

/// `begin` for card paints shared with production modes: opens a
/// window only when a `--gpu-paint` driver has armed tracing.
pub(crate) fn begin_card(w: u32, h: u32) {
    if armed() {
        begin(w, h);
    }
}

/// Whether a trace window is currently open.
pub(crate) fn active() -> bool {
    REC.with(|r| r.borrow().is_some())
}

/// Append a prim to the open window; returns its prim index, or
/// `None` when no window is open (the hook's no-op path).
pub(crate) fn record(prim: Prim) -> Option<u32> {
    REC.with(|r| {
        let mut g = r.borrow_mut();
        let rec = g.as_mut()?;
        let idx = rec.prims.len() as u32;
        rec.prims.push(prim);
        Some(idx)
    })
}

/// Record a mask prim: appends it, makes it the `last_mask_id`,
/// and returns its prim index (`None` when no window is open).
pub(crate) fn record_mask(prim: Prim) -> Option<u32> {
    REC.with(|r| {
        let mut g = r.borrow_mut();
        let rec = g.as_mut()?;
        let idx = rec.prims.len() as u32;
        rec.prims.push(prim);
        rec.last_mask_id = Some(idx);
        Some(idx)
    })
}

/// The prim index of the most recently recorded mask prim.
pub(crate) fn last_mask_id() -> Option<u32> {
    REC.with(|r| r.borrow().as_ref().and_then(|rec| rec.last_mask_id))
}

/// Store a plate's pixels (once per distinct plate instance per
/// window) and return its index into `DisplayList::plates`.
/// Outside a window this clones nothing and returns
/// `usize::MAX`; callers must only use the index after checking
/// `active()` (the `trace_composite_plate` hook does).
pub(crate) fn push_plate(plate: &Plate) -> usize {
    REC.with(|r| {
        let mut g = r.borrow_mut();
        let Some(rec) = g.as_mut() else { return usize::MAX };
        let key = (
            plate.px.as_ptr() as usize,
            plate.px.len(),
            plate.w,
            plate.h,
            plate.x0.to_bits(),
            plate.y0.to_bits(),
        );
        if let Some(i) = rec.plate_keys.iter().position(|k| *k == key) {
            return i;
        }
        rec.plate_keys.push(key);
        rec.plates.push(PlateData {
            px: plate.px.clone(),
            w: plate.w,
            h: plate.h,
            x0: plate.x0,
            y0: plate.y0,
        });
        rec.plates.len() - 1
    })
}

/// Close the window and return the recorded list. With no window
/// open, returns an empty 0x0 list (drivers check `active()`
/// before calling when the distinction matters).
pub(crate) fn take() -> DisplayList {
    REC.with(|r| {
        let rec = r.borrow_mut().take();
        match rec {
            Some(rec) => DisplayList {
                canvas_w: rec.canvas_w,
                canvas_h: rec.canvas_h,
                plates: rec.plates,
                prims: rec.prims,
            },
            None => DisplayList { canvas_w: 0, canvas_h: 0, plates: Vec::new(), prims: Vec::new() },
        }
    })
}

/// FNV-1a over the mask bitmap (one byte per bool, as stored).
/// Content identity is the right registry key: two bitmaps with
/// identical content are interchangeable for replay, so a hash
/// collision between DIFFERENT contents is the only failure mode
/// (64-bit FNV over the full bitmap; lengths are also keyed).
fn mask_hash(mask: &[bool]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in mask {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

/// Resolve the mask prim id for a masked fill's `mask` bitmap:
/// registry (content) first, then `last_mask_id`, then the
/// `NO_MASK` sentinel. A resolution through `last_mask_id` is
/// registered, so later fills over the same bitmap stay correct
/// after other mask prims have been recorded in between.
fn resolve_mask(mask: &[bool]) -> u32 {
    REC.with(|r| {
        let mut g = r.borrow_mut();
        let Some(rec) = g.as_mut() else { return NO_MASK };
        let hash = mask_hash(mask);
        if let Some(&(_, _, id)) = rec
            .mask_registry
            .iter()
            .find(|&&(hh, len, _)| hh == hash && len == mask.len())
        {
            return id;
        }
        let id = rec.last_mask_id.unwrap_or(NO_MASK);
        if id != NO_MASK {
            rec.mask_registry.push((hash, mask.len(), id));
        }
        id
    })
}

// ---------------------------------------------------------------------
// Hook shims: each painter hook site is ONE cfg-gated call to one
// of these. All are no-ops outside a trace window.
// ---------------------------------------------------------------------

pub(crate) fn trace_fill_ellipse(cx: f32, cy: f32, rx: f32, ry: f32, rot: f32, col: Rgba) {
    record(Prim::FillEllipse { cx, cy, rx, ry, rot, col });
}

pub(crate) fn trace_stroke_polyline(pts: &[(f32, f32)], width: f32, col: Rgba) {
    // Painter::stroke_polyline returns before painting for
    // pts.len() < 2; a recorded no-op would not be an executed
    // primitive, so mirror the guard here.
    if pts.len() < 2 {
        return;
    }
    record(Prim::StrokePolyline { pts: pts.to_vec(), width, col });
}

pub(crate) fn trace_fill_polygon(pts: &[(f32, f32)], col: Rgba) {
    record(Prim::FillPolygon { pts: pts.to_vec(), col });
}

pub(crate) fn trace_fill_rect(x: f32, y: f32, w: f32, h: f32, col: Rgba) {
    record(Prim::FillRect { x, y, w, h, col });
}

pub(crate) fn trace_polygon_mask(pts: &[(f32, f32)]) {
    record_mask(Prim::PolygonMask { pts: pts.to_vec() });
}

pub(crate) fn trace_ellipse_mask(cx: f32, cy: f32, rx: f32, ry: f32, rot: f32) {
    record_mask(Prim::EllipseMask { cx, cy, rx, ry, rot });
}

pub(crate) fn trace_fill_ellipse_masked(
    cx: f32, cy: f32, rx: f32, ry: f32, rot: f32, col: Rgba, mask: &[bool],
) {
    if !active() {
        return;
    }
    let mask_id = resolve_mask(mask);
    record(Prim::FillEllipseMasked { cx, cy, rx, ry, rot, col, mask: mask_id });
}

pub(crate) fn trace_fill_polygon_masked(pts: &[(f32, f32)], col: Rgba, mask: &[bool]) {
    if !active() {
        return;
    }
    // MUST run before fill_polygon_masked's own internal
    // polygon_mask(pts) call records the panel's own mask and
    // moves last_mask_id — the hook sits at the top of the
    // function for exactly this reason (see GPUFIRST-TRACE.md).
    let mask_id = resolve_mask(mask);
    record(Prim::FillPolygonMasked { pts: pts.to_vec(), col, mask: mask_id });
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn trace_head_shade(
    form: u8,
    head_c: (f32, f32),
    head_r: (f32, f32),
    circ_c: (f32, f32),
    circ_r: (f32, f32),
    rot: f32,
    rad: f32,
    col: Rgba,
) {
    record(Prim::HeadShade { form, head_c, head_r, circ_c, circ_r, rot, rad, col });
}

pub(crate) fn trace_masked_blend(rect: (i32, i32, i32, i32), mask: &[bool], col: Rgba) {
    if !active() {
        return;
    }
    // At both call sites the mask is the laugh-mouth polygon
    // mask: it is the most recent mask prim (the tongue's
    // fill_ellipse_masked between the polygon_mask call and this
    // loop records no mask), and resolve_mask's registry holds
    // the same id from the tongue fill's resolution — the two
    // routes agree; see GPUFIRST-TRACE.md §3.
    let mask_id = resolve_mask(mask);
    record(Prim::MaskedBlend { rect, mask: mask_id, col });
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn trace_composite_plate(
    plate: &Plate, hand: (f32, f32), lean_rad: f32, anchor: (f32, f32),
    flip: bool, pitch_rad: f32, hip: (f32, f32),
) {
    if !active() {
        return;
    }
    let idx = push_plate(plate);
    record(Prim::CompositePlate { plate: idx, hand, lean_rad, anchor, flip, pitch_rad, hip });
}

// ---------------------------------------------------------------------
// Serialization: hand-rolled JSON writing (no new dependencies);
// reading reuses scene::parse_json / Value.
// ---------------------------------------------------------------------

fn fb(v: f32) -> String {
    format!("{{\"b\":{}}}", v.to_bits())
}

fn pt_json(p: (f32, f32)) -> String {
    format!("[{},{}]", fb(p.0), fb(p.1))
}

fn pts_json(pts: &[(f32, f32)]) -> String {
    let mut s = String::from("[");
    for (i, p) in pts.iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        s.push_str(&pt_json(*p));
    }
    s.push(']');
    s
}

fn col_json(c: Rgba) -> String {
    format!("[{},{},{},{}]", c.0, c.1, c.2, c.3)
}

const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn b64_encode(data: &[u8]) -> String {
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b = [chunk[0], *chunk.get(1).unwrap_or(&0), *chunk.get(2).unwrap_or(&0)];
        let n = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | b[2] as u32;
        out.push(B64[(n >> 18) as usize & 63] as char);
        out.push(B64[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 { B64[(n >> 6) as usize & 63] as char } else { '=' });
        out.push(if chunk.len() > 2 { B64[n as usize & 63] as char } else { '=' });
    }
    out
}

fn b64_decode(s: &str) -> Result<Vec<u8>, String> {
    let val = |c: u8| -> Result<u32, String> {
        match c {
            b'A'..=b'Z' => Ok((c - b'A') as u32),
            b'a'..=b'z' => Ok((c - b'a' + 26) as u32),
            b'0'..=b'9' => Ok((c - b'0' + 52) as u32),
            b'+' => Ok(62),
            b'/' => Ok(63),
            _ => Err(format!("base64: bad byte {c}")),
        }
    };
    let bytes = s.as_bytes();
    if bytes.len() % 4 != 0 {
        return Err("base64: length not a multiple of 4".into());
    }
    let mut out = Vec::with_capacity(bytes.len() / 4 * 3);
    for chunk in bytes.chunks(4) {
        let pad = chunk.iter().filter(|&&c| c == b'=').count();
        let mut n: u32 = 0;
        for (i, &c) in chunk.iter().enumerate() {
            n |= if c == b'=' { 0 } else { val(c)? } << (18 - 6 * i);
        }
        out.push((n >> 16) as u8);
        if pad < 2 {
            out.push((n >> 8) as u8);
        }
        if pad < 1 {
            out.push(n as u8);
        }
    }
    Ok(out)
}

fn prim_json(p: &Prim) -> String {
    match p {
        Prim::FillPolygon { pts, col } => format!(
            "{{\"t\":\"fill_polygon\",\"pts\":{},\"col\":{}}}", pts_json(pts), col_json(*col)),
        Prim::FillPolygonMasked { pts, col, mask } => format!(
            "{{\"t\":\"fill_polygon_masked\",\"pts\":{},\"col\":{},\"mask\":{}}}",
            pts_json(pts), col_json(*col), mask),
        Prim::FillEllipse { cx, cy, rx, ry, rot, col } => format!(
            "{{\"t\":\"fill_ellipse\",\"cx\":{},\"cy\":{},\"rx\":{},\"ry\":{},\"rot\":{},\"col\":{}}}",
            fb(*cx), fb(*cy), fb(*rx), fb(*ry), fb(*rot), col_json(*col)),
        Prim::FillEllipseMasked { cx, cy, rx, ry, rot, col, mask } => format!(
            "{{\"t\":\"fill_ellipse_masked\",\"cx\":{},\"cy\":{},\"rx\":{},\"ry\":{},\"rot\":{},\"col\":{},\"mask\":{}}}",
            fb(*cx), fb(*cy), fb(*rx), fb(*ry), fb(*rot), col_json(*col), mask),
        Prim::StrokeEllipse { cx, cy, rx, ry, rot, col, width } => format!(
            "{{\"t\":\"stroke_ellipse\",\"cx\":{},\"cy\":{},\"rx\":{},\"ry\":{},\"rot\":{},\"col\":{},\"width\":{}}}",
            fb(*cx), fb(*cy), fb(*rx), fb(*ry), fb(*rot), col_json(*col), fb(*width)),
        Prim::StrokePolyline { pts, width, col } => format!(
            "{{\"t\":\"stroke_polyline\",\"pts\":{},\"width\":{},\"col\":{}}}",
            pts_json(pts), fb(*width), col_json(*col)),
        Prim::StrokeQuad { p0, p1, p2, width, col } => format!(
            "{{\"t\":\"stroke_quad\",\"p0\":{},\"p1\":{},\"p2\":{},\"width\":{},\"col\":{}}}",
            pt_json(*p0), pt_json(*p1), pt_json(*p2), fb(*width), col_json(*col)),
        Prim::FillRect { x, y, w, h, col } => format!(
            "{{\"t\":\"fill_rect\",\"x\":{},\"y\":{},\"w\":{},\"h\":{},\"col\":{}}}",
            fb(*x), fb(*y), fb(*w), fb(*h), col_json(*col)),
        Prim::PolygonMask { pts } => format!(
            "{{\"t\":\"polygon_mask\",\"pts\":{}}}", pts_json(pts)),
        Prim::EllipseMask { cx, cy, rx, ry, rot } => format!(
            "{{\"t\":\"ellipse_mask\",\"cx\":{},\"cy\":{},\"rx\":{},\"ry\":{},\"rot\":{}}}",
            fb(*cx), fb(*cy), fb(*rx), fb(*ry), fb(*rot)),
        Prim::CompositePlate { plate, hand, lean_rad, anchor, flip, pitch_rad, hip } => format!(
            "{{\"t\":\"composite_plate\",\"plate\":{},\"hand\":{},\"lean_rad\":{},\"anchor\":{},\"flip\":{},\"pitch_rad\":{},\"hip\":{}}}",
            plate, pt_json(*hand), fb(*lean_rad), pt_json(*anchor), flip, fb(*pitch_rad), pt_json(*hip)),
        Prim::HeadShade { form, head_c, head_r, circ_c, circ_r, rot, rad, col } => format!(
            "{{\"t\":\"head_shade\",\"form\":{},\"head_c\":{},\"head_r\":{},\"circ_c\":{},\"circ_r\":{},\"rot\":{},\"rad\":{},\"col\":{}}}",
            form,
            pt_json(*head_c),
            pt_json(*head_r),
            pt_json(*circ_c),
            pt_json(*circ_r),
            fb(*rot),
            fb(*rad),
            col_json(*col)),
        Prim::MaskedBlend { rect, mask, col } => format!(
            "{{\"t\":\"masked_blend\",\"rect\":[{},{},{},{}],\"mask\":{},\"col\":{}}}",
            rect.0, rect.1, rect.2, rect.3, mask, col_json(*col)),
    }
}

impl DisplayList {
    pub(crate) fn to_json(&self) -> String {
        let mut s = String::new();
        s.push_str("{\n\"format\":\"talerender-displaylist\",\"version\":1,\n");
        s.push_str(&format!("\"canvas_w\":{},\"canvas_h\":{},\n", self.canvas_w, self.canvas_h));
        s.push_str("\"plates\":[");
        for (i, pl) in self.plates.iter().enumerate() {
            if i > 0 {
                s.push(',');
            }
            s.push_str(&format!(
                "{{\"w\":{},\"h\":{},\"x0\":{},\"y0\":{},\"px_b64\":\"{}\"}}",
                pl.w, pl.h, fb(pl.x0), fb(pl.y0), b64_encode(&pl.px)
            ));
        }
        s.push_str("],\n\"prims\":[\n");
        for (i, p) in self.prims.iter().enumerate() {
            if i > 0 {
                s.push_str(",\n");
            }
            s.push_str(&prim_json(p));
        }
        s.push_str("\n]\n}\n");
        s
    }

    pub(crate) fn from_json(text: &str) -> Result<DisplayList, String> {
        let root = crate::scene::parse_json(text)?;
        if root.get("format").and_then(|v| v.as_str()) != Some("talerender-displaylist") {
            return Err("displaylist: bad format tag".into());
        }
        let u32_of = |v: &Value, what: &str| -> Result<u32, String> {
            let n = v.as_f64().ok_or_else(|| format!("displaylist: {what} not a number"))?;
            if n < 0.0 || n > u32::MAX as f64 || n.fract() != 0.0 {
                return Err(format!("displaylist: {what} not a u32"));
            }
            Ok(n as u32)
        };
        let i32_of = |v: &Value, what: &str| -> Result<i32, String> {
            let n = v.as_f64().ok_or_else(|| format!("displaylist: {what} not a number"))?;
            if n < i32::MIN as f64 || n > i32::MAX as f64 || n.fract() != 0.0 {
                return Err(format!("displaylist: {what} not an i32"));
            }
            Ok(n as i32)
        };
        let f32_of = |v: &Value, what: &str| -> Result<f32, String> {
            let bits = v
                .get("b")
                .ok_or_else(|| format!("displaylist: {what} missing bits form"))?;
            Ok(f32::from_bits(u32_of(bits, what)?))
        };
        fn field<'a>(o: &'a Value, k: &str) -> Result<&'a Value, String> {
            o.get(k).ok_or_else(|| format!("displaylist: missing field {k}"))
        }
        let pt_of = |v: &Value, what: &str| -> Result<(f32, f32), String> {
            let a = v.as_arr().ok_or_else(|| format!("displaylist: {what} not a point"))?;
            if a.len() != 2 {
                return Err(format!("displaylist: {what} point arity"));
            }
            Ok((f32_of(&a[0], what)?, f32_of(&a[1], what)?))
        };
        let pts_of = |v: &Value, what: &str| -> Result<Vec<(f32, f32)>, String> {
            v.as_arr()
                .ok_or_else(|| format!("displaylist: {what} not an array"))?
                .iter()
                .map(|p| pt_of(p, what))
                .collect()
        };
        let col_of = |v: &Value| -> Result<Rgba, String> {
            let a = v.as_arr().ok_or("displaylist: col not an array")?;
            if a.len() != 4 {
                return Err("displaylist: col arity".into());
            }
            let mut c = [0u8; 4];
            for (i, x) in a.iter().enumerate() {
                let n = u32_of(x, "col")?;
                if n > 255 {
                    return Err("displaylist: col channel > 255".into());
                }
                c[i] = n as u8;
            }
            Ok((c[0], c[1], c[2], c[3]))
        };
        let mut plates = Vec::new();
        for pl in field(&root, "plates")?.as_arr().ok_or("displaylist: plates not an array")? {
            plates.push(PlateData {
                px: b64_decode(
                    field(pl, "px_b64")?.as_str().ok_or("displaylist: px_b64 not a string")?,
                )?,
                w: u32_of(field(pl, "w")?, "plate.w")?,
                h: u32_of(field(pl, "h")?, "plate.h")?,
                x0: f32_of(field(pl, "x0")?, "plate.x0")?,
                y0: f32_of(field(pl, "y0")?, "plate.y0")?,
            });
        }
        let mut prims = Vec::new();
        for pv in field(&root, "prims")?.as_arr().ok_or("displaylist: prims not an array")? {
            let t = field(pv, "t")?.as_str().ok_or("displaylist: prim t not a string")?;
            let g = |k: &str| field(pv, k);
            let prim = match t {
                "fill_polygon" => Prim::FillPolygon {
                    pts: pts_of(g("pts")?, "pts")?,
                    col: col_of(g("col")?)?,
                },
                "fill_polygon_masked" => Prim::FillPolygonMasked {
                    pts: pts_of(g("pts")?, "pts")?,
                    col: col_of(g("col")?)?,
                    mask: u32_of(g("mask")?, "mask")?,
                },
                "fill_ellipse" => Prim::FillEllipse {
                    cx: f32_of(g("cx")?, "cx")?,
                    cy: f32_of(g("cy")?, "cy")?,
                    rx: f32_of(g("rx")?, "rx")?,
                    ry: f32_of(g("ry")?, "ry")?,
                    rot: f32_of(g("rot")?, "rot")?,
                    col: col_of(g("col")?)?,
                },
                "fill_ellipse_masked" => Prim::FillEllipseMasked {
                    cx: f32_of(g("cx")?, "cx")?,
                    cy: f32_of(g("cy")?, "cy")?,
                    rx: f32_of(g("rx")?, "rx")?,
                    ry: f32_of(g("ry")?, "ry")?,
                    rot: f32_of(g("rot")?, "rot")?,
                    col: col_of(g("col")?)?,
                    mask: u32_of(g("mask")?, "mask")?,
                },
                "stroke_ellipse" => Prim::StrokeEllipse {
                    cx: f32_of(g("cx")?, "cx")?,
                    cy: f32_of(g("cy")?, "cy")?,
                    rx: f32_of(g("rx")?, "rx")?,
                    ry: f32_of(g("ry")?, "ry")?,
                    rot: f32_of(g("rot")?, "rot")?,
                    col: col_of(g("col")?)?,
                    width: f32_of(g("width")?, "width")?,
                },
                "stroke_polyline" => Prim::StrokePolyline {
                    pts: pts_of(g("pts")?, "pts")?,
                    width: f32_of(g("width")?, "width")?,
                    col: col_of(g("col")?)?,
                },
                "stroke_quad" => Prim::StrokeQuad {
                    p0: pt_of(g("p0")?, "p0")?,
                    p1: pt_of(g("p1")?, "p1")?,
                    p2: pt_of(g("p2")?, "p2")?,
                    width: f32_of(g("width")?, "width")?,
                    col: col_of(g("col")?)?,
                },
                "fill_rect" => Prim::FillRect {
                    x: f32_of(g("x")?, "x")?,
                    y: f32_of(g("y")?, "y")?,
                    w: f32_of(g("w")?, "w")?,
                    h: f32_of(g("h")?, "h")?,
                    col: col_of(g("col")?)?,
                },
                "polygon_mask" => Prim::PolygonMask { pts: pts_of(g("pts")?, "pts")? },
                "ellipse_mask" => Prim::EllipseMask {
                    cx: f32_of(g("cx")?, "cx")?,
                    cy: f32_of(g("cy")?, "cy")?,
                    rx: f32_of(g("rx")?, "rx")?,
                    ry: f32_of(g("ry")?, "ry")?,
                    rot: f32_of(g("rot")?, "rot")?,
                },
                "composite_plate" => Prim::CompositePlate {
                    plate: u32_of(g("plate")?, "plate")? as usize,
                    hand: pt_of(g("hand")?, "hand")?,
                    lean_rad: f32_of(g("lean_rad")?, "lean_rad")?,
                    anchor: pt_of(g("anchor")?, "anchor")?,
                    // B3-CARRY-1 additive fields: pre-Batch-3 lists
                    // (no flip/pitch/hip keys) parse at the identity
                    // defaults, so old recordings replay unchanged.
                    flip: pv.get("flip").map(|v| matches!(v, Value::Bool(true))).unwrap_or(false),
                    pitch_rad: match pv.get("pitch_rad") {
                        Some(v) => f32_of(v, "pitch_rad")?,
                        None => 0.0,
                    },
                    hip: match pv.get("hip") {
                        Some(v) => pt_of(v, "hip")?,
                        None => (256.0, 574.0),
                    },
                },
                "head_shade" => Prim::HeadShade {
                    form: {
                        let n = u32_of(g("form")?, "form")?;
                        if n > 255 {
                            return Err("displaylist: form > 255".into());
                        }
                        n as u8
                    },
                    head_c: pt_of(g("head_c")?, "head_c")?,
                    head_r: pt_of(g("head_r")?, "head_r")?,
                    circ_c: pt_of(g("circ_c")?, "circ_c")?,
                    circ_r: pt_of(g("circ_r")?, "circ_r")?,
                    rot: f32_of(g("rot")?, "rot")?,
                    rad: f32_of(g("rad")?, "rad")?,
                    col: col_of(g("col")?)?,
                },
                "masked_blend" => Prim::MaskedBlend {
                    rect: {
                        let a = g("rect")?
                            .as_arr()
                            .ok_or("displaylist: rect not an array")?;
                        if a.len() != 4 {
                            return Err("displaylist: rect arity".into());
                        }
                        (
                            i32_of(&a[0], "rect")?,
                            i32_of(&a[1], "rect")?,
                            i32_of(&a[2], "rect")?,
                            i32_of(&a[3], "rect")?,
                        )
                    },
                    mask: u32_of(g("mask")?, "mask")?,
                    col: col_of(g("col")?)?,
                },
                other => return Err(format!("displaylist: unknown prim t={other}")),
            };
            prims.push(prim);
        }
        Ok(DisplayList {
            canvas_w: u32_of(field(&root, "canvas_w")?, "canvas_w")?,
            canvas_h: u32_of(field(&root, "canvas_h")?, "canvas_h")?,
            plates,
            prims,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_round_trip_is_bit_exact() {
        // Values chosen to break naive formatting: NaN (a payload
        // pattern), -0.0, +inf, a subnormal, and ordinary values.
        let nan = f32::from_bits(0x7fc0_0042);
        let list = DisplayList {
            canvas_w: 2560,
            canvas_h: 2816,
            plates: vec![PlateData {
                px: vec![0, 1, 2, 250, 251, 252, 253],
                w: 7,
                h: 1,
                x0: -0.0,
                y0: nan,
            }],
            prims: vec![
                Prim::FillEllipse {
                    cx: f32::INFINITY,
                    cy: f32::from_bits(1), // smallest subnormal
                    rx: 12.5,
                    ry: -0.0,
                    rot: nan,
                    col: (1, 2, 3, 4),
                },
                Prim::PolygonMask { pts: vec![(0.0, 0.0), (1.5, -2.25)] },
                Prim::FillPolygonMasked {
                    pts: vec![(3.0, 4.0)],
                    col: (9, 8, 7, 6),
                    mask: 1,
                },
                Prim::StrokeQuad {
                    p0: (0.0, 0.0),
                    p1: (1.0, 2.0),
                    p2: (3.0, 4.0),
                    width: 5.0,
                    col: (0, 0, 0, 255),
                },
                Prim::CompositePlate {
                    plate: 0,
                    hand: (10.0, 20.0),
                    lean_rad: 0.125,
                    anchor: (0.0, -0.0),
                    flip: true,
                    pitch_rad: -0.5,
                    hip: (256.0, 574.0),
                },
                Prim::HeadShade {
                    form: 1,
                    head_c: (100.0, 200.0),
                    head_r: (276.0, 267.7),
                    circ_c: (150.0, 220.0),
                    circ_r: (276.0, 276.0),
                    rot: -0.0,
                    rad: 277.0,
                    col: (10, 20, 30, 255),
                },
                Prim::MaskedBlend {
                    rect: (-3, 40, 900, 512),
                    mask: 1,
                    col: (255, 255, 255, 255),
                },
            ],
        };
        let text = list.to_json();
        let back = DisplayList::from_json(&text).expect("round-trip parse");
        // PartialEq on f32 fields uses ==, which is false for NaN
        // and true for -0.0 == 0.0, so compare bit patterns via a
        // second serialization: identical text <=> identical bits
        // everywhere (the writer is a pure function of the bits).
        assert_eq!(back.to_json(), text);
        assert_eq!(back.canvas_w, 2560);
        assert_eq!(back.plates[0].px, list.plates[0].px);
        match &back.prims[0] {
            Prim::FillEllipse { cx, cy, rot, .. } => {
                assert_eq!(cx.to_bits(), f32::INFINITY.to_bits());
                assert_eq!(cy.to_bits(), 1);
                assert_eq!(rot.to_bits(), 0x7fc0_0042);
            }
            other => panic!("prim 0 changed shape: {other:?}"),
        }
    }

    #[test]
    fn composite_plate_pre_b3_json_parses_at_identity_defaults() {
        // A pre-Batch-3 display list: composite_plate WITHOUT the
        // B3-CARRY-1 flip/pitch_rad/hip keys must parse at the
        // identity defaults (flip false, pitch 0, rest hip), so
        // old recordings replay unchanged.
        let text = "{\"format\":\"talerender-displaylist\",\"version\":1,\
            \"canvas_w\":64,\"canvas_h\":64,\"plates\":[],\"prims\":[\
            {\"t\":\"composite_plate\",\"plate\":0,\
            \"hand\":[{\"b\":1092616192},{\"b\":1101004800}],\
            \"lean_rad\":{\"b\":1040187392},\
            \"anchor\":[{\"b\":0},{\"b\":2147483648}]}]}";
        let list = DisplayList::from_json(text).expect("pre-B3 list parses");
        match &list.prims[0] {
            Prim::CompositePlate { flip, pitch_rad, hip, .. } => {
                assert!(!flip);
                assert_eq!(pitch_rad.to_bits(), 0.0f32.to_bits());
                assert_eq!(*hip, (256.0, 574.0));
            }
            other => panic!("prim 0 shape: {other:?}"),
        }
    }

    #[test]
    fn mask_resolution_survives_interleaved_masks() {
        // The stripes/vest pattern from paint_toon_full: bean
        // mask, then repeated masked fills whose own internal
        // polygon_mask calls move last_mask_id between them.
        begin(4, 4);
        let bean: Vec<bool> = vec![true; 16];
        let own: Vec<bool> = vec![false; 16];
        let bean_id = record_mask(Prim::PolygonMask { pts: vec![(0.0, 0.0)] }).unwrap();
        trace_fill_polygon_masked(&[(1.0, 1.0)], (1, 2, 3, 255), &bean);
        // Simulate fill_polygon_masked's internal own-mask record.
        record_mask(Prim::PolygonMask { pts: vec![(9.0, 9.0)] });
        assert_ne!(last_mask_id(), Some(bean_id));
        // Second fill over the SAME bean bitmap must still
        // resolve to the bean mask via the content registry —
        // and its own content differs from `own`'s bitmap above.
        let _ = &own;
        trace_fill_polygon_masked(&[(2.0, 2.0)], (4, 5, 6, 255), &bean);
        trace_fill_ellipse_masked(1.0, 1.0, 2.0, 2.0, 0.0, (7, 8, 9, 255), &bean);
        let list = take();
        // Stream: 0 bean mask, 1 masked fill, 2 own mask (the
        // interleaved record that moves last_mask_id), 3 masked
        // fill, 4 masked ellipse.
        for (i, expect_pts) in [(1usize, 1.0f32), (3, 2.0)] {
            match &list.prims[i] {
                Prim::FillPolygonMasked { pts, mask, .. } => {
                    assert_eq!(*mask, bean_id, "prim {i} mask threaded wrong");
                    assert_eq!(pts[0].0, expect_pts);
                }
                other => panic!("prim {i} shape: {other:?}"),
            }
        }
        match &list.prims[4] {
            Prim::FillEllipseMasked { mask, .. } => assert_eq!(*mask, bean_id),
            other => panic!("prim 4 shape: {other:?}"),
        }
    }

    #[test]
    fn inactive_recorder_is_a_noop() {
        assert!(!active());
        trace_fill_rect(0.0, 0.0, 1.0, 1.0, (0, 0, 0, 255));
        trace_polygon_mask(&[(0.0, 0.0)]);
        assert_eq!(last_mask_id(), None);
        let list = take();
        assert!(list.prims.is_empty());
    }
}
