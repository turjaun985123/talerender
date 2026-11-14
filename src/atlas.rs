//! Texture atlas packing for character canvases (Wave 2 seat W18).
//!
//! The audit (Phase 3) asks for texture atlasing of frequently
//! reused character states. Today every character card is its own
//! GPU texture: film.rs's `Renderer::render` calls
//! `Renderer::upload_dyn` once per card PER FRAME, creating a fresh
//! 2560x2816 `Rgba8UnormSrgb` texture (+ view, + two bind groups)
//! and re-uploading ~28.8 MB that the CPU-side `PaintCache` already
//! decided was worth keeping. An atlas page turns that into: one
//! persistent texture, one `queue.write_texture` per DISTINCT state
//! (sub-rectangle upload at the placement's origin), one shared
//! view/sampler for every card on the page, and per-card UVs
//! remapped into the page. This module is the pure packing half:
//! placements + UV math + statistics. It owns no GPU types and no
//! pixels, so it is unit-testable standalone
//! (`rustc --edition 2021 --test src/atlas.rs`). The wiring sketch
//! and the pixel-neutrality argument live in
//! docs/integration/ATLAS-W18.md.
//!
//! PIXEL LAW (hard): atlasing is an upload/storage optimisation
//! ONLY. Sampled pixels must be identical to the unatlased path.
//! The discipline that buys this, in brief (full argument in the
//! design note):
//!   - the card shaders (`shader_basic.wgsl` colour,
//!     `shader_shadow_card.wgsl` alpha) sample with a LINEAR /
//!     ClampToEdge sampler over a texture with mip_level_count = 1,
//!     so every fetch is a 2x2 bilinear footprint on level 0 whose
//!     out-of-range taps clamp to the edge texel;
//!   - each entry therefore reserves a CELL of (w + 2P) x (h + 2P)
//!     texels and the P-texel ring around the inner rect MUST be
//!     uploaded as edge-replicated texels (the clamp, materialised)
//!     — a bilinear footprint reaches at most 1 texel past the
//!     inner rect, so P = 2 (the default) keeps every tap on
//!     texels whose values equal the unatlased fetch;
//!   - page dimensions are powers-friendly: the standard page is
//!     8192x8192 = 2^13, so every UV endpoint is a dyadic rational
//!     that f32 represents EXACTLY.
//!
//! DETERMINISM: pack() sorts entries by a total order (area desc,
//! w desc, h desc, id asc) before placing, scans free rectangles
//! in a fixed vector order with a total tie-break key, and tries
//! pages in index order. No HashMap is iterated anywhere (the
//! duplicate check uses a BTreeSet for membership only). Same
//! input SET -> same placement list, in the same order, regardless
//! of input order.
//!
//! PACKER: MaxRects with Best-Short-Side-Fit. A shelf packer was
//! rejected: shelf wastes the strip above every shorter entry in a
//! row, and the moment entries are heterogeneous (plates, strips,
//! future smaller canvases) that waste compounds; MaxRects reuses
//! L-shaped leftovers. For the production census — uniform
//! 2560x2816 canvases — the two coincide (a clean 3x2 grid per
//! 8192 page), so nothing is lost on today's data.

use std::collections::BTreeSet;

/// Largest texture dimension the engine ever requests. Every
/// production device request in the tree uses
/// `wgpu::Limits::default()` — standard WebGPU limits — whose
/// `max_texture_dimension_2d` is 8192 (see the comment at
/// ab1.rs's device request: downlevel limits cap at 2048 and
/// cannot even hold one 2560x2816 canvas). A page larger than
/// this could not be created by any renderer in the engine.
pub(crate) const MAX_PAGE_DIM: u32 = 8192;

/// The production character canvas (toonfull.rs via film.rs):
/// 2560x2816 RGBA. All current character states share this size;
/// the packer still accepts arbitrary sizes.
pub(crate) const CANVAS_W: u32 = 2560;
pub(crate) const CANVAS_H: u32 = 2816;

/// Default gutter, in texels, replicated around every entry.
/// Bilinear reach is 1 texel (see the module header); 2 buys
/// margin for f32 UV arithmetic at zero cost on the production
/// census (a 2564x2820 cell still tiles 3x2 in an 8192 page).
pub(crate) const DEFAULT_PADDING: u32 = 2;

/// One entry to pack: a caller-chosen id (the paint cache's state
/// key is the intended id) and the INNER size in texels (padding
/// is added by the packer, never by the caller).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AtlasEntry {
    pub id: String,
    pub w: u32,
    pub h: u32,
}

impl AtlasEntry {
    pub(crate) fn new(id: &str, w: u32, h: u32) -> AtlasEntry {
        AtlasEntry { id: id.to_string(), w, h }
    }
}

/// Why an entry is not placed. Every entry ends up placed xor
/// overflowed with one of these reasons — never silently dropped.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum OverflowReason {
    /// Cell (inner + gutter) exceeds the page in some dimension.
    TooLarge,
    /// Fits a page in principle, but `max_pages` pages are full.
    PageLimit,
    /// Same id as an earlier entry (ids key the upload cache).
    DuplicateId,
    /// Zero width or height — nothing to upload.
    ZeroSize,
    /// The config itself is unusable (page dim 0 or > 8192).
    BadConfig,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Overflow {
    pub entry: AtlasEntry,
    pub reason: OverflowReason,
}

/// Packer configuration. Page dims are texels; `padding` is the
/// one-sided gutter; `max_pages` bounds GPU residency (None =
/// unbounded — the packer then places everything placeable).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct AtlasConfig {
    pub page_w: u32,
    pub page_h: u32,
    pub padding: u32,
    pub max_pages: Option<usize>,
}

impl AtlasConfig {
    /// Standard page: 8192x8192 (the engine's texture limit),
    /// default gutter, unbounded pages.
    pub(crate) fn standard() -> AtlasConfig {
        AtlasConfig { page_w: MAX_PAGE_DIM, page_h: MAX_PAGE_DIM, padding: DEFAULT_PADDING, max_pages: None }
    }

    fn valid(&self) -> bool {
        self.page_w >= 1 && self.page_h >= 1 && self.page_w <= MAX_PAGE_DIM && self.page_h <= MAX_PAGE_DIM
    }
}

/// An axis-aligned texel rectangle. (x, y) is the TOP-LEFT texel —
/// wgpu texture row 0 / v = 0 — matching the upload origin and the
/// UV convention the card geometry already uses (quad_geo emits
/// v = 0 at the card's top edge).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Rect {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

impl Rect {
    pub(crate) fn right(&self) -> u32 {
        self.x.saturating_add(self.w)
    }
    pub(crate) fn bottom(&self) -> u32 {
        self.y.saturating_add(self.h)
    }
    pub(crate) fn area(&self) -> u64 {
        self.w as u64 * self.h as u64
    }
    pub(crate) fn intersects(&self, o: &Rect) -> bool {
        self.x < o.right() && o.x < self.right() && self.y < o.bottom() && o.y < self.bottom()
    }
    pub(crate) fn contains(&self, o: &Rect) -> bool {
        o.x >= self.x && o.y >= self.y && o.right() <= self.right() && o.bottom() <= self.bottom()
    }
}

/// A placed entry. (x, y) is the INNER rect's top-left texel in
/// the page; the reserved CELL is the inner rect inflated by the
/// gutter on all four sides.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Placement {
    pub id: String,
    pub page: u32,
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
    pub padding: u32,
}

impl Placement {
    pub(crate) fn inner_rect(&self) -> Rect {
        Rect { x: self.x, y: self.y, w: self.w, h: self.h }
    }

    /// The reserved space, gutter included. Cells never overlap;
    /// inner rects of neighbours are therefore always separated
    /// by at least 2 * padding texels of replicated edge.
    pub(crate) fn cell_rect(&self) -> Rect {
        Rect {
            x: self.x - self.padding,
            y: self.y - self.padding,
            w: self.w + 2 * self.padding,
            h: self.h + 2 * self.padding,
        }
    }

    /// Normalized UV rect of the INNER rect — the coordinate form
    /// the character shader consumes. `shader_basic.wgsl` /
    /// `shader_shadow_card.wgsl` do
    /// `textureSample(tex, smp, in.uv)` with `in.uv` interpolated
    /// from the card geometry's [0,1]x[0,1] vertex UVs
    /// (scene.rs `quad_geo`: v = 0 at the top edge, matching
    /// texture row 0). Remapping a geometry UV is then
    /// `rect.remap(uv)`, applied per-vertex when the card's
    /// geometry is built — the shaders do not change.
    pub(crate) fn uv_rect(&self, page_w: u32, page_h: u32) -> UvRect {
        UvRect {
            u0: self.x as f32 / page_w as f32,
            v0: self.y as f32 / page_h as f32,
            u1: self.x.saturating_add(self.w) as f32 / page_w as f32,
            v1: self.y.saturating_add(self.h) as f32 / page_h as f32,
        }
    }

    /// UV rect of the CELL (gutter included) — for the uploader's
    /// edge-replication pass and for debug overlays; NOT for card
    /// geometry.
    pub(crate) fn cell_uv_rect(&self, page_w: u32, page_h: u32) -> UvRect {
        let c = self.cell_rect();
        UvRect {
            u0: c.x as f32 / page_w as f32,
            v0: c.y as f32 / page_h as f32,
            u1: c.right() as f32 / page_w as f32,
            v1: c.bottom() as f32 / page_h as f32,
        }
    }
}

/// A normalized UV rectangle + the remap the card geometry needs.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct UvRect {
    pub u0: f32,
    pub v0: f32,
    pub u1: f32,
    pub v1: f32,
}

impl UvRect {
    /// Map an unatlased [0,1]x[0,1] UV into the atlas page.
    /// Exact at the corners by construction: remap([0,0]) is
    /// (u0, v0) with zero arithmetic error (0 * span + u0), and
    /// remap([1,1]) is u0 + (u1 - u0), which for power-of-two page
    /// sizes is the exact dyadic endpoint (see tests).
    pub(crate) fn remap(&self, uv: [f32; 2]) -> [f32; 2] {
        [self.u0 + uv[0] * (self.u1 - self.u0), self.v0 + uv[1] * (self.v1 - self.v0)]
    }
}

/// The result of one pack: placements (in deterministic
/// processing order), explicit overflows, and the page geometry
/// the statistics need.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct AtlasPack {
    pub placements: Vec<Placement>,
    pub overflow: Vec<Overflow>,
    pub pages: u32,
    pub page_w: u32,
    pub page_h: u32,
    pub padding: u32,
}

impl AtlasPack {
    pub(crate) fn placed_texels(&self) -> u64 {
        self.placements.iter().map(|p| p.w as u64 * p.h as u64).sum()
    }

    pub(crate) fn cell_texels(&self) -> u64 {
        self.placements.iter().map(|p| p.cell_rect().area()).sum()
    }

    pub(crate) fn page_texels(&self) -> u64 {
        self.pages as u64 * self.page_w as u64 * self.page_h as u64
    }

    /// Fraction of allocated page area carrying real image data
    /// (inner rects). 0.0 when no page was allocated.
    pub(crate) fn utilisation_inner(&self) -> f64 {
        let t = self.page_texels();
        if t == 0 {
            0.0
        } else {
            self.placed_texels() as f64 / t as f64
        }
    }

    /// Fraction of allocated page area reserved (cells, gutter
    /// included) — the packing efficiency of the layout itself.
    pub(crate) fn utilisation_cell(&self) -> f64 {
        let t = self.page_texels();
        if t == 0 {
            0.0
        } else {
            self.cell_texels() as f64 / t as f64
        }
    }

    /// GPU bytes of one RGBA8 page.
    pub(crate) fn page_bytes(&self) -> u64 {
        self.page_w as u64 * self.page_h as u64 * 4
    }

    /// Total GPU bytes of all allocated pages.
    pub(crate) fn total_bytes(&self) -> u64 {
        self.page_bytes() * self.pages as u64
    }

    /// Effective GPU bytes per placed entry (page_bytes * pages /
    /// placed) — the number to compare against the unatlased
    /// per-canvas texture size. f64::INFINITY when nothing placed.
    pub(crate) fn bytes_per_placed(&self) -> f64 {
        if self.placements.is_empty() {
            f64::INFINITY
        } else {
            self.total_bytes() as f64 / self.placements.len() as f64
        }
    }

    /// Greppable one-line summary, in the house style of
    /// paintbudget's stats line.
    pub(crate) fn stats_line(&self) -> String {
        format!(
            "atlas: pages={} placed={} overflow={} placed_texels={} page_texels={} util_inner={:.2}% util_cell={:.2}% bytes={}",
            self.pages,
            self.placements.len(),
            self.overflow.len(),
            self.placed_texels(),
            self.page_texels(),
            self.utilisation_inner() * 100.0,
            self.utilisation_cell() * 100.0,
            self.total_bytes(),
        )
    }
}

// ---------------------------------------------------------------------
// MaxRects page
// ---------------------------------------------------------------------

struct Page {
    free: Vec<Rect>,
}

impl Page {
    fn new(w: u32, h: u32) -> Page {
        Page { free: vec![Rect { x: 0, y: 0, w, h }] }
    }

    /// Best-Short-Side-Fit placement of a cell. Returns the
    /// winning free rect's index and the cell's top-left there,
    /// or None if no free rect holds it. Pure scan: the tie-break
    /// key (short leftover, long leftover, y, x) is a total order
    /// over distinct free rects, so the winner does not depend on
    /// free-list order.
    fn find(&self, cw: u32, ch: u32) -> Option<(usize, u32, u32)> {
        let mut best: Option<(u32, u32, u32, u32, usize)> = None; // (short, long, y, x, idx)
        for (i, f) in self.free.iter().enumerate() {
            if cw <= f.w && ch <= f.h {
                let dw = f.w - cw;
                let dh = f.h - ch;
                let key = (dw.min(dh), dw.max(dh), f.y, f.x, i);
                if best.map_or(true, |b| key < b) {
                    best = Some(key);
                }
            }
        }
        best.map(|(_, _, y, x, i)| (i, x, y))
    }

    /// Commit a cell placed at the origin of free rect `chosen`:
    /// the chosen rect is replaced by its right remainder (the
    /// used rect's height) and bottom remainder (its own full
    /// width) — the textbook MaxRects split, whose two remainders
    /// never overlap — while every OTHER free rect the cell
    /// overlaps is split into its non-overlapping remainders.
    /// Contained rects are then pruned. Vector order is stable
    /// throughout.
    fn commit(&mut self, used: Rect, chosen: usize) {
        let mut next: Vec<Rect> = Vec::with_capacity(self.free.len() + 2);
        for (i, f) in self.free.drain(..).enumerate() {
            if i == chosen {
                if used.right() < f.right() {
                    next.push(Rect { x: used.right(), y: f.y, w: f.right() - used.right(), h: used.h });
                }
                if used.bottom() < f.bottom() {
                    next.push(Rect { x: f.x, y: used.bottom(), w: f.w, h: f.bottom() - used.bottom() });
                }
                continue;
            }
            if !f.intersects(&used) {
                next.push(f);
                continue;
            }
            // Left / right / top / bottom remainders of f \ used.
            if used.x > f.x {
                next.push(Rect { x: f.x, y: f.y, w: used.x - f.x, h: f.h });
            }
            if used.right() < f.right() {
                next.push(Rect { x: used.right(), y: f.y, w: f.right() - used.right(), h: f.h });
            }
            if used.y > f.y {
                next.push(Rect { x: f.x, y: f.y, w: f.w, h: used.y - f.y });
            }
            if used.bottom() < f.bottom() {
                next.push(Rect { x: f.x, y: used.bottom(), w: f.w, h: f.bottom() - used.bottom() });
            }
        }
        // Prune contained rects (an equal twin counts as contained:
        // keep the first, drop later copies).
        let mut kept: Vec<Rect> = Vec::with_capacity(next.len());
        for (i, r) in next.iter().enumerate() {
            let contained = next.iter().enumerate().any(|(j, o)| {
                (j < i && o.contains(r)) || (j > i && o.contains(r) && !r.contains(o))
            });
            if !contained {
                kept.push(*r);
            }
        }
        self.free = kept;
    }
}

// ---------------------------------------------------------------------
// The packer
// ---------------------------------------------------------------------

/// Pack `entries` into atlas pages per `cfg`. See the module
/// header for the sort discipline and the determinism argument.
pub(crate) fn pack(entries: &[AtlasEntry], cfg: &AtlasConfig) -> AtlasPack {
    let mut out = AtlasPack {
        placements: Vec::new(),
        overflow: Vec::new(),
        pages: 0,
        page_w: cfg.page_w,
        page_h: cfg.page_h,
        padding: cfg.padding,
    };
    if !cfg.valid() {
        for e in entries {
            out.overflow.push(Overflow { entry: e.clone(), reason: OverflowReason::BadConfig });
        }
        return out;
    }

    // Total-order sort: area desc, w desc, h desc, id asc. Two
    // entries that agree on all four are interchangeable except
    // for id ordering, which the id key fixes.
    let mut sorted: Vec<&AtlasEntry> = entries.iter().collect();
    sorted.sort_by(|a, b| {
        let area_a = a.w as u64 * a.h as u64;
        let area_b = b.w as u64 * b.h as u64;
        area_b
            .cmp(&area_a)
            .then_with(|| b.w.cmp(&a.w))
            .then_with(|| b.h.cmp(&a.h))
            .then_with(|| a.id.cmp(&b.id))
    });

    let pad2 = 2 * cfg.padding as u64;
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    let mut pages: Vec<Page> = Vec::new();

    for e in sorted {
        if e.w == 0 || e.h == 0 {
            out.overflow.push(Overflow { entry: (*e).clone(), reason: OverflowReason::ZeroSize });
            continue;
        }
        if !seen.insert(e.id.as_str()) {
            out.overflow.push(Overflow { entry: (*e).clone(), reason: OverflowReason::DuplicateId });
            continue;
        }
        let cw = e.w as u64 + pad2;
        let ch = e.h as u64 + pad2;
        if cw > cfg.page_w as u64 || ch > cfg.page_h as u64 {
            out.overflow.push(Overflow { entry: (*e).clone(), reason: OverflowReason::TooLarge });
            continue;
        }
        let (cw, ch) = (cw as u32, ch as u32);

        // First page (lowest index) that can hold the cell wins —
        // earlier pages stay dense, later pages stay few.
        let mut spot: Option<(usize, usize, u32, u32)> = None;
        for (pi, page) in pages.iter().enumerate() {
            if let Some((fi, x, y)) = page.find(cw, ch) {
                spot = Some((pi, fi, x, y));
                break;
            }
        }
        let (pi, fi, cx, cy) = match spot {
            Some(s) => s,
            None => {
                if cfg.max_pages.map_or(false, |m| pages.len() >= m) {
                    out.overflow.push(Overflow { entry: (*e).clone(), reason: OverflowReason::PageLimit });
                    continue;
                }
                pages.push(Page::new(cfg.page_w, cfg.page_h));
                let pi = pages.len() - 1;
                let (fi, x, y) = pages[pi].find(cw, ch).expect("fresh page holds any cell that passed TooLarge");
                (pi, fi, x, y)
            }
        };
        pages[pi].commit(Rect { x: cx, y: cy, w: cw, h: ch }, fi);
        out.placements.push(Placement {
            id: e.id.clone(),
            page: pi as u32,
            x: cx + cfg.padding,
            y: cy + cfg.padding,
            w: e.w,
            h: e.h,
            padding: cfg.padding,
        });
    }
    out.pages = pages.len() as u32;
    out
}

// ---------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// SYNTHETIC census, labelled as such (no measured distinct-
    /// state count exists yet — paint-cache statistics only landed
    /// with W06 this wave). Basis: the Onoda film is 1,811 frames
    /// over 14 segments; every painted canvas is exactly
    /// 2560x2816. 240 uniform entries is chosen to exceed every
    /// retention limit the engine configures — the paint cache's
    /// default entry cap (160) and its 2 GiB byte budget
    /// (~74 canvases) — so the pack is stressed past the regime
    /// where atlasing would be admitted in production.
    fn census_uniform(n: usize) -> Vec<AtlasEntry> {
        (0..n).map(|i| AtlasEntry::new(&format!("state-{i:03}"), CANVAS_W, CANVAS_H)).collect()
    }

    fn assert_no_cell_overlap(p: &AtlasPack) {
        for a in &p.placements {
            let ca = a.cell_rect();
            assert!(ca.right() <= p.page_w && ca.bottom() <= p.page_h, "cell escapes page: {a:?}");
            for b in &p.placements {
                if a.page == b.page && a.id < b.id {
                    assert!(!ca.intersects(&b.cell_rect()), "cells overlap: {a:?} {b:?}");
                }
            }
        }
    }

    #[test]
    fn census_240_uniform_canvases() {
        let pack = pack(&census_uniform(240), &AtlasConfig::standard());
        assert_eq!(pack.overflow.len(), 0);
        assert_eq!(pack.placements.len(), 240);
        // 2564x2820 cells tile an 8192 page exactly 3x2.
        assert_eq!(pack.pages, 40);
        assert_no_cell_overlap(&pack);
        // Utilisation: 6 canvases per page, exact dyadic figures.
        assert!((pack.utilisation_inner() - 0.64453125).abs() < 1e-12, "{}", pack.stats_line());
        let expect_cell = 6.0 * (2564.0 * 2820.0) / (8192.0 * 8192.0);
        assert!((pack.utilisation_cell() - expect_cell).abs() < 1e-12);
        // 40 pages x 256 MiB = 10 GiB of pages for 240 canvases —
        // the honest cost line the design note quotes.
        assert_eq!(pack.total_bytes(), 40 * 8192 * 8192 * 4);
        assert!(pack.stats_line().contains("pages=40 placed=240 overflow=0"));
    }

    #[test]
    fn grid_positions_first_page() {
        let pack = pack(&census_uniform(6), &AtlasConfig::standard());
        assert_eq!(pack.pages, 1);
        let want = [(2, 2), (2566, 2), (5130, 2), (2, 2822), (2566, 2822), (5130, 2822)];
        for (p, w) in pack.placements.iter().zip(want) {
            assert_eq!((p.x, p.y), w, "{}", p.id);
            assert_eq!(p.page, 0);
        }
    }

    #[test]
    fn gutter_separates_neighbours() {
        let pack = pack(&census_uniform(12), &AtlasConfig::standard());
        for p in &pack.placements {
            let c = p.cell_rect();
            assert_eq!((p.x - c.x, p.y - c.y), (p.padding, p.padding));
            assert_eq!((c.w, c.h), (p.w + 2 * p.padding, p.h + 2 * p.padding));
        }
        // Horizontally adjacent inner rects are 2*gutter apart:
        // no bilinear footprint (1 texel) can reach a neighbour.
        let a = &pack.placements[0];
        let b = &pack.placements[1];
        assert_eq!(b.x - a.inner_rect().right(), 2 * DEFAULT_PADDING);
    }

    #[test]
    fn determinism_input_order_invariant() {
        let entries = census_uniform(37);
        let mut reversed = entries.clone();
        reversed.reverse();
        let mut rotated = entries.clone();
        rotated.rotate_left(11);
        let a = pack(&entries, &AtlasConfig::standard());
        let b = pack(&reversed, &AtlasConfig::standard());
        let c = pack(&rotated, &AtlasConfig::standard());
        assert_eq!(format!("{:?}", a.placements), format!("{:?}", b.placements));
        assert_eq!(format!("{:?}", a.placements), format!("{:?}", c.placements));
        assert_eq!(a, pack(&entries, &AtlasConfig::standard()));
    }

    #[test]
    fn mixed_sizes_no_overlap_all_placed() {
        // SYNTHETIC heterogeneous set (plates / strips / small
        // canvases alongside full canvases), sizes via a fixed
        // LCG — no rand dependency, same seed every run.
        let mut entries = Vec::new();
        let mut s: u32 = 0x1234_5678;
        let mut rnd = move || {
            s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            s >> 16
        };
        for i in 0..200 {
            let w = 64 + rnd() % 2497; // 64..=2560
            let h = 64 + rnd() % 2753; // 64..=2816
            entries.push(AtlasEntry::new(&format!("mix-{i:03}"), w, h));
        }
        let pack = pack(&entries, &AtlasConfig::standard());
        assert_eq!(pack.overflow.len(), 0, "{}", pack.stats_line());
        assert_eq!(pack.placements.len(), 200);
        assert_no_cell_overlap(&pack);
        assert!(pack.utilisation_cell() > 0.5, "{}", pack.stats_line());
    }

    #[test]
    fn uv_corners_exact() {
        let pack = pack(&census_uniform(1), &AtlasConfig::standard());
        let p = &pack.placements[0];
        let r = p.uv_rect(8192, 8192);
        // Corner remaps are exact f32: page 8192 = 2^13 makes
        // every endpoint a dyadic rational with a short mantissa.
        assert_eq!(r.remap([0.0, 0.0]), [2.0 / 8192.0, 2.0 / 8192.0]);
        assert_eq!(r.remap([1.0, 0.0]), [2562.0 / 8192.0, 2.0 / 8192.0]);
        assert_eq!(r.remap([0.0, 1.0]), [2.0 / 8192.0, 2818.0 / 8192.0]);
        assert_eq!(r.remap([1.0, 1.0]), [2562.0 / 8192.0, 2818.0 / 8192.0]);
        // Midpoint lands on the inner rect's centre texel block.
        let mid = r.remap([0.5, 0.5]);
        assert_eq!(mid, [(2.0 + 1280.0) / 8192.0, (2.0 + 1408.0) / 8192.0]);
        // Cell rect (gutter included) starts at the page origin
        // for the first placement.
        let c = p.cell_uv_rect(8192, 8192);
        assert_eq!((c.u0, c.v0), (0.0, 0.0));
    }

    #[test]
    fn uv_rect_custom_page() {
        let cfg = AtlasConfig { page_w: 4096, page_h: 4096, padding: 0, max_pages: None };
        let pack = pack(&[AtlasEntry::new("a", 1024, 512)], &cfg);
        let r = pack.placements[0].uv_rect(4096, 4096);
        assert_eq!(r.remap([0.0, 0.0]), [0.0, 0.0]);
        assert_eq!(r.remap([1.0, 1.0]), [0.25, 0.125]);
    }

    #[test]
    fn overflow_paths_are_explicit() {
        let cfg = AtlasConfig { page_w: 8192, page_h: 8192, padding: 2, max_pages: Some(1) };
        let entries = vec![
            AtlasEntry::new("too-wide", 8192, 100), // cell 8196 > page
            AtlasEntry::new("zero", 0, 100),
            AtlasEntry::new("dup", 64, 64),
            AtlasEntry::new("dup", 64, 64),
        ];
        let pack = pack(&entries, &cfg);
        assert_eq!(pack.placements.len(), 1);
        let reasons: Vec<OverflowReason> = pack.overflow.iter().map(|o| o.reason).collect();
        assert!(reasons.contains(&OverflowReason::TooLarge));
        assert!(reasons.contains(&OverflowReason::ZeroSize));
        assert!(reasons.contains(&OverflowReason::DuplicateId));

        // Page limit: 7 canvases, one page holds 6.
        let pack = super::pack(&census_uniform(7), &cfg);
        assert_eq!(pack.placements.len(), 6);
        assert_eq!(pack.overflow.len(), 1);
        assert_eq!(pack.overflow[0].reason, OverflowReason::PageLimit);
    }

    #[test]
    fn bad_config_overflows_everything() {
        let cfg = AtlasConfig { page_w: 16384, page_h: 8192, padding: 2, max_pages: None };
        let pack = pack(&census_uniform(3), &cfg);
        assert_eq!(pack.placements.len(), 0);
        assert_eq!(pack.pages, 0);
        assert!(pack.overflow.iter().all(|o| o.reason == OverflowReason::BadConfig));
    }

    #[test]
    fn empty_input() {
        let pack = pack(&[], &AtlasConfig::standard());
        assert_eq!(pack.pages, 0);
        assert_eq!(pack.utilisation_inner(), 0.0);
        assert_eq!(pack.bytes_per_placed(), f64::INFINITY);
    }

    #[test]
    fn utilisation_small_exact() {
        let cfg = AtlasConfig { page_w: 1000, page_h: 1000, padding: 0, max_pages: None };
        let pack = pack(&[AtlasEntry::new("a", 100, 100)], &cfg);
        assert!((pack.utilisation_inner() - 0.01).abs() < 1e-15);
        assert_eq!(pack.bytes_per_placed(), 4_000_000.0);
    }
}
