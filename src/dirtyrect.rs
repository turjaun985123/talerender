// =====================================================================
// DELIVERY NOTE — TaleRender V2, Batch 1, Seat 1 (WP1.2)
//
// Files created:
//   src/dirtyrect.rs  (this file; the ONLY file this seat writes)
//
// Contract points implemented (V2-CONTRACTS.md §1 + §5 seat 1;
// V2-MASTER-PROMPT.md §5.2 "Dirty-region texture uploads"):
//   - `Rect` { x, y, w, h: u32 } — one dirty rectangle in texel
//     space, half-open: it covers [x, x+w) x [y, y+h).
//   - `DirtyRects` accumulator: `add`, `add_region`, `add_point`,
//     `add_clipped`, `add_full`, `absorb` (multi-character union),
//     `add_diff` / `from_diff` (byte-diff producer over RGBA rows),
//     `clear`, `is_empty`, `len`.
//   - `merged()` — overlapping AND edge-adjacent rects coalesce
//     into bounding boxes; output is sorted by (y, x) (ties by
//     (h, w)) and is a pure function of the SET of added rects,
//     never of insertion order (deterministic).
//   - `coverage(canvas_w, canvas_h)` — merged, canvas-clipped
//     area / canvas area, in [0.0, 1.0].
//   - `should_fall_back(canvas_w, canvas_h)` — true iff coverage
//     is STRICTLY greater than FALLBACK_COVERAGE (0.60); exactly
//     0.60 is NOT a fallback (pinned by a unit test).
//   - `total_uploaded_bytes(bytes_per_pixel)` — merged area x bpp
//     (pass 4 for the Rgba8UnormSrgb card textures), and
//     `planned_upload_bytes(canvas_w, canvas_h, bpp)` — the same,
//     or the full-canvas size when `should_fall_back` holds.
//   - Pure logic only: no wgpu types, no I/O, no env, no new
//     dependencies, no `unwrap`/`expect` outside tests.
//
// Wiring the lead must do (this seat edits no existing file):
//   1. src/main.rs — add the module line, alphabetically, after
//      the anchor line
//          mod dof;
//      and before `mod errors;`:
//          mod dirtyrect;
//   2. src/film.rs — THE consult site is the per-frame dynamic
//      character-card upload, `Renderer::upload_dyn`:
//          fn upload_dyn(&self, px: &[u8], w: u32, h: u32) -> wgpu::Texture {
//      whose body today ends in a full-canvas upload anchored by
//          self.queue.write_texture(
//              wgpu::TexelCopyTextureInfo { texture: &tex, mip_level: 0, origin: wgpu::Origin3d::ZERO, ...
//      That is the only full upload on the animated-character
//      path: `Renderer::render` calls it once per dynamic card
//      per frame, at the loop anchored by
//          for p in dyn_tex {
//              let t = self.upload_dyn(&p.0, p.1, p.2);
//      (`p.0` = pixels, `p.1` = w, `p.2` = h of film.rs's
//      `Painted` tuple; the canvases are 2560x2816 RGBA.)
//      Intended consult, once textures persist (see limitation
//      (a)): keep the previous frame's `Rc<Painted>` per
//      TexSrc::Dyn slot (an Rc clone — zero copy; PaintCache
//      already keeps the bytes alive), then per slot:
//          let mut dirty = crate::dirtyrect::DirtyRects::new();
//          dirty.add_diff(&prev.0, &cur.0, w, h, 4);
//          if prev slot missing || dirty.should_fall_back(w, h) {
//              // existing full write_texture, byte-for-byte as today
//          } else {
//              for r in dirty.merged() {
//                  queue.write_texture(
//                      wgpu::TexelCopyTextureInfo { texture: &tex, mip_level: 0,
//                          origin: wgpu::Origin3d { x: r.x, y: r.y, z: 0 },
//                          aspect: wgpu::TextureAspect::All },
//                      px,   // the FULL frame buffer, as today
//                      wgpu::TexelCopyBufferLayout {
//                          offset: (r.y as u64 * w as u64 + r.x as u64) * 4,
//                          bytes_per_row: Some(w * 4),
//                          rows_per_image: Some(r.h) },
//                      wgpu::Extent3d { width: r.w, height: r.h, depth_or_array_layers: 1 });
//              }
//          }
//      The VRAM-ledger call beside the upload
//      (`mem_alloc_texture(MemClass::E, tex_bytes(...))`) stays
//      exactly as it is — it accounts the texture, not the bytes
//      moved, and partial uploads do not change it.
//   3. Do NOT wire the other write_texture in film.rs: the
//      one-time standing-texture closure in `Renderer::new`
//      anchored by
//          let upload = |device: &wgpu::Device, queue: &wgpu::Queue, px: &[u8], w: u32, h: u32, format: wgpu::TextureFormat|
//      uploads each static texture exactly once at startup;
//      dirty regions do not apply. (composer.rs's env-map
//      write_texture is likewise one-time and out of scope.)
//
// Tests included (in-module, #[cfg(test)]): disjoint rects stay
// separate; overlapping merge (incl. transitive chains);
// horizontal + vertical edge-adjacency merge; corner-touch does
// NOT merge; one-pixel gap does NOT merge; containment; L-shape
// bounding-box over-cover (documented behaviour); zero-area
// rects ignored; add_point / adjacent points; add_clipped;
// absorb across sets; coverage math (empty / partial / full /
// out-of-canvas clipping); fallback boundary (0.60 exactly = no
// fallback, 0.61 = fallback, 0.59 = no fallback); byte estimates
// + planned bytes under fallback; ordering determinism (three
// insertion orders, one merged output, sorted by (y, x),
// merged() idempotent); diff producer (identical buffers, one
// changed pixel, vertically adjacent changed rows merging,
// wide row span, truncated buffers cannot panic).
//
// Known limitations / charter-vs-code notes:
//   (a) NO UPLOAD IS SAVED BY THIS MODULE ALONE. Today
//       `upload_dyn` creates a FRESH wgpu::Texture every frame,
//       so there is no resident texture to patch: partial
//       uploads only pay once the lead keeps one texture per
//       Dyn slot alive across frames and retains the previous
//       frame's pixels per slot (both sketched in step 2). Until
//       then the module is inert accounting/policy, exactly like
//       paintbudget.rs was on its landing day. When wired as
//       sketched it is pixel-neutral BY CONSTRUCTION: the union
//       of merged rects is a superset of the changed texels, so
//       the patched texture ends byte-identical to a full
//       upload; the fallback path IS today's full upload.
//   (b) The charter pack said this module "serves
//       gpucore.rs's upload helpers" — gpucore.rs has NO
//       queue.write_texture helper to serve (verified by grep:
//       its Readback path is the opposite direction,
//       texture -> buffer; the only write_texture sites are
//       film.rs x2 and composer.rs x1). The module is therefore
//       standalone pure logic for the film.rs site above; no
//       gpucore change is needed or proposed.
//   (c) merged() unions are BOUNDING BOXES: an L-shaped dirty
//       set uploads the clean corner of its bounding box too.
//       Over-upload only — a dirty texel is never missed.
//   (d) Edge-adjacency requires a positive-length shared edge;
//       rects meeting only at a corner point stay separate
//       (still correct, one extra upload call).
//   (e) add() does not clip to a canvas (the set does not know
//       one); producers are in-bounds by construction (add_diff)
//       or should use add_clipped. coverage() still intersects
//       with the canvas defensively, so stray rects cannot push
//       it past 1.0.
//   (f) add_diff works in whole-row spans: two distant changes
//       on one row upload the clean span between them. Merging
//       + the 0.60 fallback bound the waste; per-pixel runs
//       would cost more CPU than they save at card sizes.
//   (g) merged() is a sort + pairwise fixpoint, O(n^2) scans in
//       the worst case. n is the number of ADDED rects (a handful
//       per character via add_diff row spans), not texels; do
//       not feed it per-pixel rects for a whole canvas.
// =====================================================================

//! Dirty-region tracking for partial texture uploads (WP1.2).
//!
//! The dynamic character cards are the renderer's recurring
//! upload cost: every frame, `Renderer::render` hands each
//! visible character's painted canvas (2560x2816 RGBA, ~29 MB)
//! to `Renderer::upload_dyn`, which re-uploads the WHOLE canvas
//! even when the character barely moved — a blink or a mouth
//! shape touches a few percent of the texels. This module is the
//! pure-logic half of the fix (master prompt §5.2): track which
//! regions changed, merge them into a small deterministic set of
//! rectangles, and let the caller either upload just those
//! rectangles or — once they cover too much of the canvas to be
//! worth splitting — fall back to the existing full upload.
//!
//! Pixel-neutrality law: this module never sees a GPU object and
//! never decides a pixel. A partial upload driven by `merged()`
//! writes every changed texel (the merged set is a superset of
//! the changes; see `DirtyRects::merged`), so the resulting
//! texture is byte-identical to the full upload it replaces, and
//! the fallback path is the full upload unchanged. Determinism:
//! `merged()` depends only on the set of added rectangles, never
//! on insertion order or hash iteration — there is no hash map
//! here at all.
//!
//! The producer shipped here is `DirtyRects::add_diff`, a
//! row-span byte diff between the previous and current painted
//! canvases. Callers that already know their changed regions
//! (a future painter that reports its own strokes, say) can use
//! `add` / `add_region` / `add_point` directly instead.

/// Coverage fraction above which partial uploads stop paying:
/// when the merged dirty set covers more than this share of the
/// canvas, one full `write_texture` moves no more bytes than the
/// partial set would (and costs one call instead of many), so
/// `DirtyRects::should_fall_back` sends the caller to it.
/// Exactly AT the threshold is not a fallback — the comparison
/// is strict, and a unit test pins the boundary.
pub(crate) const FALLBACK_COVERAGE: f64 = 0.60;

/// Bytes per texel of the dynamic card textures
/// (`wgpu::TextureFormat::Rgba8UnormSrgb`, film.rs `upload_dyn`).
/// The byte-estimate methods take the value as a parameter so
/// other formats stay expressible; this is the one to pass for
/// the character-card path.
pub(crate) const CARD_BYTES_PER_PIXEL: u32 = 4;

/// One dirty rectangle in texel space, half-open: it covers
/// columns `[x, x + w)` and rows `[y, y + h)`. A rect with
/// `w == 0` or `h == 0` is empty and covers nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct Rect {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

impl Rect {
    pub(crate) const fn new(x: u32, y: u32, w: u32, h: u32) -> Rect {
        Rect { x, y, w, h }
    }

    /// True when the rect covers no texels.
    pub(crate) fn is_empty(&self) -> bool {
        self.w == 0 || self.h == 0
    }

    /// Exclusive right edge (`x + w`), saturating at `u32::MAX`
    /// rather than overflowing on adversarial input.
    pub(crate) fn right(&self) -> u32 {
        self.x.saturating_add(self.w)
    }

    /// Exclusive bottom edge (`y + h`); see `right`.
    pub(crate) fn bottom(&self) -> u32 {
        self.y.saturating_add(self.h)
    }

    /// Covered texels, in u64 so a full-canvas rect cannot
    /// overflow (2560 x 2816 fits u32 easily; a maximal u32 rect
    /// would not).
    pub(crate) fn area(&self) -> u64 {
        u64::from(self.w) * u64::from(self.h)
    }

    /// The part of this rect inside a `canvas_w` x `canvas_h`
    /// canvas, or `None` when nothing of it is inside.
    pub(crate) fn clipped_to(&self, canvas_w: u32, canvas_h: u32) -> Option<Rect> {
        if self.is_empty() || self.x >= canvas_w || self.y >= canvas_h {
            return None;
        }
        Some(Rect {
            x: self.x,
            y: self.y,
            w: self.right().min(canvas_w) - self.x,
            h: self.bottom().min(canvas_h) - self.y,
        })
    }
}

/// Exclusive right edge in u64 (merge arithmetic never
/// saturates mid-computation the way the u32 accessors can).
fn right64(r: &Rect) -> u64 {
    u64::from(r.x) + u64::from(r.w)
}

/// Exclusive bottom edge in u64; see `right64`.
fn bottom64(r: &Rect) -> u64 {
    u64::from(r.y) + u64::from(r.h)
}

/// Do the 1-D spans `[a0, a0 + alen)` and `[b0, b0 + blen)`
/// overlap with positive length?
fn spans_overlap(a0: u32, alen: u32, b0: u32, blen: u32) -> bool {
    u64::from(a0) < u64::from(b0) + u64::from(blen)
        && u64::from(b0) < u64::from(a0) + u64::from(alen)
}

/// Do the spans overlap OR touch (one starts exactly where the
/// other ends)?
fn spans_touch(a0: u32, alen: u32, b0: u32, blen: u32) -> bool {
    u64::from(a0) <= u64::from(b0) + u64::from(blen)
        && u64::from(b0) <= u64::from(a0) + u64::from(alen)
}

/// Merge predicate: the rects overlap, or share an edge segment
/// of positive length (side-by-side with vertically overlapping
/// spans, or stacked with horizontally overlapping spans).
/// Meeting only at a corner point does NOT merge — neither span
/// pair overlaps with positive length in that case.
fn mergeable(a: &Rect, b: &Rect) -> bool {
    (spans_overlap(a.x, a.w, b.x, b.w) && spans_touch(a.y, a.h, b.y, b.h))
        || (spans_overlap(a.y, a.h, b.y, b.h) && spans_touch(a.x, a.w, b.x, b.w))
}

/// Smallest rect covering both inputs (the merge union). May
/// cover texels neither input covered — over-upload only.
fn bounding_box(a: &Rect, b: &Rect) -> Rect {
    let x = a.x.min(b.x);
    let y = a.y.min(b.y);
    let right = right64(a).max(right64(b)).min(u64::from(u32::MAX));
    let bottom = bottom64(a).max(bottom64(b)).min(u64::from(u32::MAX));
    Rect {
        x,
        y,
        w: (right - u64::from(x)) as u32,
        h: (bottom - u64::from(y)) as u32,
    }
}

/// Canonical order for merge inputs and `merged()` output:
/// by (y, x), ties by (h, w) so the order is total and two
/// equal-(y, x) rects can never swap between runs.
fn sort_rects(rects: &mut [Rect]) {
    rects.sort_by_key(|r| (r.y, r.x, r.h, r.w));
}

/// A set of dirty rectangles over one texture canvas, not yet
/// merged. Cheap to accumulate into (rects are stored verbatim,
/// zero-area ones dropped); all the work happens in `merged()`,
/// which the caller runs once per upload decision.
///
/// Typical per-card, per-frame use:
/// 1. `DirtyRects::from_diff(prev_pixels, cur_pixels, w, h, 4)`
///    (or manual `add` calls),
/// 2. `should_fall_back(w, h)` → the existing full upload, else
/// 3. one partial `write_texture` per rect of `merged()`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct DirtyRects {
    rects: Vec<Rect>,
}

impl DirtyRects {
    pub(crate) fn new() -> DirtyRects {
        DirtyRects { rects: Vec::new() }
    }

    /// Add one dirty rect. Zero-area rects are dropped here, so
    /// every stored rect covers at least one texel.
    pub(crate) fn add(&mut self, rect: Rect) {
        if !rect.is_empty() {
            self.rects.push(rect);
        }
    }

    /// `add` spelled with bare coordinates.
    pub(crate) fn add_region(&mut self, x: u32, y: u32, w: u32, h: u32) {
        self.add(Rect::new(x, y, w, h));
    }

    /// Mark a single texel dirty (a 1x1 rect at (x, y)). Useful
    /// for sparse point changes; adjacent points coalesce in
    /// `merged()` like any other rects.
    pub(crate) fn add_point(&mut self, x: u32, y: u32) {
        self.add(Rect::new(x, y, 1, 1));
    }

    /// Add only the part of `rect` that lies inside a
    /// `canvas_w` x `canvas_h` canvas; adds nothing when the
    /// rect is entirely outside.
    pub(crate) fn add_clipped(&mut self, rect: Rect, canvas_w: u32, canvas_h: u32) {
        if let Some(clipped) = rect.clipped_to(canvas_w, canvas_h) {
            self.add(clipped);
        }
    }

    /// Mark the whole canvas dirty (first frame for a texture,
    /// a resize, a format change — anything with no valid
    /// previous contents to diff against).
    pub(crate) fn add_full(&mut self, canvas_w: u32, canvas_h: u32) {
        self.add(Rect::new(0, 0, canvas_w, canvas_h));
    }

    /// Absorb another set's rects into this one (still
    /// unmerged). This is how several animated characters' dirty
    /// sets combine when they share one texture or one upload
    /// decision: absorb each character's set, then merge once.
    pub(crate) fn absorb(&mut self, other: &DirtyRects) {
        self.rects.extend_from_slice(&other.rects);
    }

    /// Diff two same-sized pixel buffers and add one row-span
    /// rect per changed row: for each row whose bytes differ,
    /// the span from the first to the last differing texel.
    /// Vertically adjacent spans coalesce later in `merged()`.
    ///
    /// Defensive by construction (this runs on the render hot
    /// path): a zero width/height/bytes-per-pixel adds nothing,
    /// and only COMPLETE rows present in both buffers are
    /// scanned — a short buffer truncates the scan instead of
    /// panicking or reading out of bounds. When in doubt the
    /// caller should treat a size mismatch as a full upload
    /// (`add_full`); this helper never invents changes, it can
    /// only miss rows it was not given.
    pub(crate) fn add_diff(
        &mut self,
        prev: &[u8],
        next: &[u8],
        width: u32,
        height: u32,
        bytes_per_pixel: u32,
    ) {
        if width == 0 || height == 0 || bytes_per_pixel == 0 {
            return;
        }
        let bpp = bytes_per_pixel as usize;
        let row_bytes = match (width as usize).checked_mul(bpp) {
            Some(n) => n,
            None => return,
        };
        let rows_available = (prev.len().min(next.len())) / row_bytes;
        let rows = (height as usize).min(rows_available);
        for y in 0..rows {
            let start = y * row_bytes;
            let (prev_row, next_row) = match (prev.get(start..start + row_bytes), next.get(start..start + row_bytes)) {
                (Some(p), Some(n)) => (p, n),
                _ => break,
            };
            if prev_row == next_row {
                continue;
            }
            // The rows differ, so both scans below terminate on
            // a real differing byte inside the row.
            let mut first = 0usize;
            while prev_row[first] == next_row[first] {
                first += 1;
            }
            let mut last = row_bytes - 1;
            while prev_row[last] == next_row[last] {
                last -= 1;
            }
            let x0 = (first / bpp) as u32;
            let x1 = (last / bpp) as u32;
            self.add(Rect::new(x0, y as u32, x1 - x0 + 1, 1));
        }
    }

    /// A fresh set holding exactly the `add_diff` row spans
    /// between two pixel buffers — the normal producer for the
    /// character-card path (previous vs current `Painted`
    /// pixels).
    pub(crate) fn from_diff(
        prev: &[u8],
        next: &[u8],
        width: u32,
        height: u32,
        bytes_per_pixel: u32,
    ) -> DirtyRects {
        let mut dirty = DirtyRects::new();
        dirty.add_diff(prev, next, width, height, bytes_per_pixel);
        dirty
    }

    /// Drop every stored rect, ready for the next frame.
    pub(crate) fn clear(&mut self) {
        self.rects.clear();
    }

    /// True when no (non-empty) rect has been added, or after
    /// `clear`. An empty set means "nothing changed": the caller
    /// uploads nothing at all.
    pub(crate) fn is_empty(&self) -> bool {
        self.rects.is_empty()
    }

    /// Number of stored, UNMERGED rects. `merged().len()` is the
    /// upload-call count; this is the accumulation count.
    pub(crate) fn len(&self) -> usize {
        self.rects.len()
    }

    /// The merged dirty set: every stored rect coalesced with
    /// any rect it overlaps or shares an edge with, transitively,
    /// into bounding boxes. The result covers every dirty texel
    /// (and possibly some clean ones inside a bounding box —
    /// over-upload only, never a missed texel).
    ///
    /// Deterministic: the stored rects are canonically sorted
    /// before merging and after every union, so the output is a
    /// function of the SET of added rects, not their order, and
    /// is itself sorted by (y, x) with (h, w) tie-breaks.
    pub(crate) fn merged(&self) -> Vec<Rect> {
        let mut rects: Vec<Rect> = self.rects.iter().copied().filter(|r| !r.is_empty()).collect();
        sort_rects(&mut rects);
        loop {
            let mut unioned = false;
            'scan: for i in 0..rects.len() {
                for j in (i + 1)..rects.len() {
                    if mergeable(&rects[i], &rects[j]) {
                        rects[i] = bounding_box(&rects[i], &rects[j]);
                        rects.remove(j);
                        unioned = true;
                        break 'scan;
                    }
                }
            }
            if !unioned {
                break;
            }
            sort_rects(&mut rects);
        }
        sort_rects(&mut rects);
        rects
    }

    /// Total texels covered by the merged set, NOT clipped to
    /// any canvas (the stored rects, merged). For canvas-relative
    /// figures use `coverage` / `should_fall_back`, which clip.
    pub(crate) fn merged_area(&self) -> u64 {
        self.merged().iter().map(|r| r.area()).sum()
    }

    /// Merged area clipped to the canvas — the numerator shared
    /// by `coverage` and `should_fall_back`.
    fn covered_area(&self, canvas_w: u32, canvas_h: u32) -> u64 {
        self.merged()
            .iter()
            .filter_map(|r| r.clipped_to(canvas_w, canvas_h))
            .map(|r| r.area())
            .sum()
    }

    /// Coverage as an exact f64 ratio (0.0 for an empty set or a
    /// zero-area canvas). Kept in f64 so the fallback boundary
    /// compares exactly: a true 60% set yields the f64 nearest
    /// 0.60, which is NOT greater than the threshold.
    fn coverage_ratio(&self, canvas_w: u32, canvas_h: u32) -> f64 {
        let canvas_area = u64::from(canvas_w) * u64::from(canvas_h);
        if canvas_area == 0 {
            return 0.0;
        }
        self.covered_area(canvas_w, canvas_h) as f64 / canvas_area as f64
    }

    /// Fraction of the canvas the merged dirty set covers:
    /// merged (canvas-clipped) area / canvas area, in
    /// [0.0, 1.0]. 0.0 for an empty set or a zero-area canvas.
    pub(crate) fn coverage(&self, canvas_w: u32, canvas_h: u32) -> f32 {
        self.coverage_ratio(canvas_w, canvas_h) as f32
    }

    /// True when the dirty set covers STRICTLY more than
    /// `FALLBACK_COVERAGE` (0.60) of the canvas — the caller
    /// should do one full upload instead of partial uploads.
    /// Exactly 0.60 is not a fallback.
    pub(crate) fn should_fall_back(&self, canvas_w: u32, canvas_h: u32) -> bool {
        self.coverage_ratio(canvas_w, canvas_h) > FALLBACK_COVERAGE
    }

    /// Estimated bytes a partial upload of the merged set moves:
    /// merged area x `bytes_per_pixel` (pass
    /// `CARD_BYTES_PER_PIXEL` = 4 for the RGBA8 cards). An
    /// estimate for telemetry/logging — the wire cost of
    /// `write_texture` is exactly this for tight rows.
    pub(crate) fn total_uploaded_bytes(&self, bytes_per_pixel: u32) -> u64 {
        self.merged_area() * u64::from(bytes_per_pixel)
    }

    /// Bytes the caller will actually move for this set on a
    /// `canvas_w` x `canvas_h` canvas: the full-canvas size when
    /// `should_fall_back` holds, else `total_uploaded_bytes`.
    pub(crate) fn planned_upload_bytes(
        &self,
        canvas_w: u32,
        canvas_h: u32,
        bytes_per_pixel: u32,
    ) -> u64 {
        if self.should_fall_back(canvas_w, canvas_h) {
            u64::from(canvas_w) * u64::from(canvas_h) * u64::from(bytes_per_pixel)
        } else {
            self.total_uploaded_bytes(bytes_per_pixel)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(rects: &[(u32, u32, u32, u32)]) -> DirtyRects {
        let mut d = DirtyRects::new();
        for &(x, y, w, h) in rects {
            d.add(Rect::new(x, y, w, h));
        }
        d
    }

    #[test]
    fn empty_set() {
        let d = DirtyRects::new();
        assert!(d.is_empty());
        assert_eq!(d.len(), 0);
        assert_eq!(d.merged(), Vec::<Rect>::new());
        assert_eq!(d.merged_area(), 0);
        assert_eq!(d.coverage(2560, 2816), 0.0);
        assert!(!d.should_fall_back(2560, 2816));
        assert_eq!(d.total_uploaded_bytes(CARD_BYTES_PER_PIXEL), 0);
        assert_eq!(d.planned_upload_bytes(2560, 2816, CARD_BYTES_PER_PIXEL), 0);
        // Zero-area canvas is defined, not a divide-by-zero.
        assert_eq!(d.coverage(0, 0), 0.0);
        assert!(!d.should_fall_back(0, 100));
    }

    #[test]
    fn zero_area_rects_are_dropped() {
        let mut d = DirtyRects::new();
        d.add(Rect::new(5, 5, 0, 4));
        d.add(Rect::new(5, 5, 4, 0));
        d.add_region(1, 1, 0, 0);
        assert!(d.is_empty());
        assert_eq!(d.merged(), Vec::<Rect>::new());
    }

    #[test]
    fn clear_empties_the_set() {
        let mut d = set(&[(0, 0, 4, 4), (10, 10, 2, 2)]);
        assert!(!d.is_empty());
        d.clear();
        assert!(d.is_empty());
        assert_eq!(d.len(), 0);
        assert_eq!(d.merged(), Vec::<Rect>::new());
    }

    #[test]
    fn disjoint_rects_stay_separate() {
        let d = set(&[(0, 0, 4, 4), (20, 20, 4, 4), (40, 0, 3, 3)]);
        let m = d.merged();
        assert_eq!(m.len(), 3);
        assert_eq!(d.merged_area(), 16 + 16 + 9);
    }

    #[test]
    fn overlapping_rects_merge() {
        let d = set(&[(0, 0, 10, 10), (5, 5, 10, 10)]);
        assert_eq!(d.merged(), vec![Rect::new(0, 0, 15, 15)]);
    }

    #[test]
    fn overlap_merges_transitively() {
        // A overlaps B, B overlaps C, A does not touch C: the
        // fixpoint still lands on one bounding box.
        let d = set(&[(0, 0, 6, 6), (5, 0, 6, 6), (10, 0, 6, 6)]);
        assert_eq!(d.merged(), vec![Rect::new(0, 0, 16, 6)]);
    }

    #[test]
    fn horizontal_edge_adjacency_merges() {
        // Second rect starts exactly at the first's right edge,
        // with vertically overlapping spans.
        let d = set(&[(0, 0, 4, 4), (4, 1, 4, 4)]);
        assert_eq!(d.merged(), vec![Rect::new(0, 0, 8, 5)]);
    }

    #[test]
    fn vertical_edge_adjacency_merges() {
        let d = set(&[(1, 0, 4, 4), (0, 4, 4, 4)]);
        assert_eq!(d.merged(), vec![Rect::new(0, 0, 5, 8)]);
    }

    #[test]
    fn corner_touch_does_not_merge() {
        // Meeting at the single point (2, 2) shares no edge
        // segment, so the rects stay separate by design.
        let d = set(&[(0, 0, 2, 2), (2, 2, 2, 2)]);
        assert_eq!(d.merged().len(), 2);
    }

    #[test]
    fn one_texel_gap_does_not_merge() {
        let d = set(&[(0, 0, 2, 2), (3, 0, 2, 2)]);
        assert_eq!(d.merged().len(), 2);
        let d = set(&[(0, 0, 2, 2), (0, 3, 2, 2)]);
        assert_eq!(d.merged().len(), 2);
    }

    #[test]
    fn containment_merges_to_the_outer_rect() {
        let d = set(&[(2, 2, 3, 3), (0, 0, 10, 10)]);
        assert_eq!(d.merged(), vec![Rect::new(0, 0, 10, 10)]);
        assert_eq!(d.merged_area(), 100);
    }

    #[test]
    fn bounding_box_overcovers_l_shapes() {
        // Documented behaviour: an L merges to its bounding box,
        // uploading the clean corner too — never missing dirt.
        let d = set(&[(0, 0, 4, 1), (0, 1, 1, 3)]);
        assert_eq!(d.merged(), vec![Rect::new(0, 0, 4, 4)]);
        assert_eq!(d.merged_area(), 16);
    }

    #[test]
    fn points_merge_when_adjacent() {
        let mut d = DirtyRects::new();
        d.add_point(3, 3);
        assert_eq!(d.merged(), vec![Rect::new(3, 3, 1, 1)]);
        d.add_point(4, 3);
        assert_eq!(d.merged(), vec![Rect::new(3, 3, 2, 1)]);
        d.add_point(10, 10);
        assert_eq!(d.merged().len(), 2);
    }

    #[test]
    fn clipped_add_keeps_only_the_inside_part() {
        let mut d = DirtyRects::new();
        d.add_clipped(Rect::new(8, 8, 10, 10), 10, 10);
        assert_eq!(d.merged(), vec![Rect::new(8, 8, 2, 2)]);
        d.add_clipped(Rect::new(20, 0, 5, 5), 10, 10);
        assert_eq!(d.len(), 1); // fully outside: nothing added
        assert_eq!(Rect::new(0, 0, 4, 4).clipped_to(10, 10), Some(Rect::new(0, 0, 4, 4)));
        assert_eq!(Rect::new(10, 0, 4, 4).clipped_to(10, 10), None);
    }

    #[test]
    fn absorb_combines_sets_before_merging() {
        // Two characters, one shared decision: rects from
        // different sets still merge with each other.
        let a = set(&[(0, 0, 4, 4)]);
        let b = set(&[(4, 0, 4, 4), (100, 100, 2, 2)]);
        let mut both = a.clone();
        both.absorb(&b);
        assert_eq!(both.merged(), vec![Rect::new(0, 0, 8, 4), Rect::new(100, 100, 2, 2)]);
    }

    #[test]
    fn coverage_math() {
        // 100x100 canvas.
        let quarter = set(&[(0, 0, 50, 50)]);
        assert!((quarter.coverage(100, 100) - 0.25).abs() < 1e-6);
        let full = set(&[(0, 0, 100, 100)]);
        assert!((full.coverage(100, 100) - 1.0).abs() < 1e-6);
        // Overlapping rects are not double-counted: merged area
        // is 150, not 100 + 100.
        let overlap = set(&[(0, 0, 10, 10), (5, 0, 10, 10)]);
        assert!((overlap.coverage(100, 100) - 0.015).abs() < 1e-6);
        // Rects reaching past the canvas are clipped for
        // coverage, so it can never exceed 1.0.
        let stray = set(&[(90, 90, 50, 50)]);
        assert!((stray.coverage(100, 100) - 0.01).abs() < 1e-6);
    }

    #[test]
    fn fallback_threshold_boundary() {
        // Canvas 10x10 = 100 texels; the threshold is strict.
        let at = set(&[(0, 0, 10, 6)]); // exactly 60%
        assert!((at.coverage(10, 10) - 0.60).abs() < 1e-6);
        assert!(!at.should_fall_back(10, 10), "0.60 exactly must NOT fall back");

        let mut above = set(&[(0, 0, 10, 6)]);
        above.add_point(0, 6); // 61 dirty texels, 70 after the merge
        assert!(above.should_fall_back(10, 10));

        // 59 texels, in two rects a clear row apart so the
        // bounding-box merge cannot inflate the count.
        let below = set(&[(0, 0, 10, 5), (0, 6, 9, 1)]);
        assert!((below.coverage(10, 10) - 0.59).abs() < 1e-6);
        assert!(!below.should_fall_back(10, 10));

        let full = set(&[(0, 0, 10, 10)]);
        assert!(full.should_fall_back(10, 10));
    }

    #[test]
    fn byte_estimates() {
        let d = set(&[(0, 0, 2, 2), (10, 10, 2, 2)]);
        assert_eq!(d.total_uploaded_bytes(4), (4 + 4) * 4);
        assert_eq!(d.total_uploaded_bytes(1), 8);
        // Below the fallback threshold the plan is the partial sum.
        assert_eq!(d.planned_upload_bytes(100, 100, 4), (4 + 4) * 4);
        // Above it the plan is the full canvas, not the (larger
        // or equal) partial sum.
        let big = set(&[(0, 0, 10, 10)]);
        assert_eq!(big.planned_upload_bytes(10, 10, 4), 10 * 10 * 4);
    }

    #[test]
    fn ordering_is_deterministic() {
        let rects = [(20, 30, 4, 4), (0, 0, 4, 4), (20, 0, 4, 4), (0, 30, 4, 4), (10, 15, 2, 2)];
        let forward = set(&rects);
        let mut reversed = rects;
        reversed.reverse();
        let backward = set(&reversed);
        let mut rotated = rects;
        rotated.rotate_left(2);
        let rotated_set = set(&rotated);

        let m = forward.merged();
        assert_eq!(m, backward.merged());
        assert_eq!(m, rotated_set.merged());
        // merged() is a pure query: calling it again changes nothing.
        assert_eq!(m, forward.merged());
        // Output order is (y, x).
        assert_eq!(
            m,
            vec![
                Rect::new(0, 0, 4, 4),
                Rect::new(20, 0, 4, 4),
                Rect::new(10, 15, 2, 2),
                Rect::new(0, 30, 4, 4),
                Rect::new(20, 30, 4, 4),
            ]
        );
    }

    #[test]
    fn diff_of_identical_buffers_is_empty() {
        let px = vec![7u8; 4 * 3 * 4]; // 4x3 RGBA
        let d = DirtyRects::from_diff(&px, &px.clone(), 4, 3, 4);
        assert!(d.is_empty());
    }

    #[test]
    fn diff_finds_one_changed_pixel() {
        let prev = vec![0u8; 4 * 2 * 4]; // 4x2 RGBA
        let mut next = prev.clone();
        next[(0 * 4 + 1) * 4] = 255; // texel (1, 0), first byte
        let d = DirtyRects::from_diff(&prev, &next, 4, 2, 4);
        assert_eq!(d.merged(), vec![Rect::new(1, 0, 1, 1)]);
    }

    #[test]
    fn diff_spans_the_changed_part_of_a_row() {
        let prev = vec![0u8; 8 * 1 * 4]; // 8x1 RGBA
        let mut next = prev.clone();
        next[1 * 4] = 9; // texel x=1
        next[6 * 4 + 3] = 9; // texel x=6
        let d = DirtyRects::from_diff(&prev, &next, 8, 1, 4);
        // One row-span covering both changes and the clean
        // texels between them (documented over-cover).
        assert_eq!(d.merged(), vec![Rect::new(1, 0, 6, 1)]);
    }

    #[test]
    fn diff_rows_merge_vertically() {
        let prev = vec![0u8; 4 * 3 * 4]; // 4x3 RGBA
        let mut next = prev.clone();
        next[(1 * 4 + 2) * 4] = 1; // (2, 1)
        next[(2 * 4 + 2) * 4] = 1; // (2, 2)
        let d = DirtyRects::from_diff(&prev, &next, 4, 3, 4);
        assert_eq!(d.merged(), vec![Rect::new(2, 1, 1, 2)]);
    }

    #[test]
    fn diff_tolerates_short_and_mismatched_buffers() {
        let prev = vec![0u8; 4 * 2 * 4];
        let short = vec![0u8; 4]; // shorter than one row (16 bytes)
        let mut d = DirtyRects::new();
        d.add_diff(&prev, &short, 4, 2, 4); // must not panic
        assert!(d.is_empty()); // no complete row pair exists to scan
        let mut d2 = DirtyRects::new();
        d2.add_diff(&prev, &prev, 4, 2, 0); // zero bpp: no-op
        d2.add_diff(&prev, &prev, 0, 2, 4); // zero width: no-op
        assert!(d2.is_empty());
    }

    #[test]
    fn full_canvas_marks_everything() {
        let mut d = DirtyRects::new();
        d.add_full(2560, 2816);
        assert_eq!(d.merged(), vec![Rect::new(0, 0, 2560, 2816)]);
        assert!((d.coverage(2560, 2816) - 1.0).abs() < 1e-6);
        assert!(d.should_fall_back(2560, 2816));
        assert_eq!(d.total_uploaded_bytes(CARD_BYTES_PER_PIXEL), 2560 * 2816 * 4);
    }
}
