// ============================================================================
// DELIVERY NOTE — Chrome-Replacement Wave, Batch 1, Seat 8
// (WP1.2: dirty-region ACTIVATION — persistent Dyn textures).
//
// Files created by this seat (NEW FILES ONLY; nothing existing
// was edited, no git was run, no dependency was added):
//   1. src/memreport_print.rs (WP2 glue — separate note)
//   2. src/dyntex.rs          (this file)
//   3. docs/integration/CR-B1-08-MEMWIRE.md (integration specs
//      + the frame-parity argument for this module)
//
// The problem (V2-HANDOVER, WP1.2 row): src/dirtyrect.rs
// implements the dirty-rectangle upload POLICY, but it is
// inert — film.rs's `Renderer::upload_dyn` creates a FRESH
// wgpu::Texture per dynamic card per frame, so there is no
// resident texture for a partial upload to patch. Activation
// = persistent textures. This module is that activation: a
// pool that owns one long-lived texture per TexSrc::Dyn slot,
// consults dirtyrect's policy against the previous frame's
// pixels for the slot, and issues either zero writes (nothing
// changed), one partial write_texture per merged dirty rect,
// or the existing full upload verbatim (the fallback).
//
// Designed against the ACTUAL call sites (verified at HEAD
// b093f3a, not from stale docs):
//   * film.rs `Renderer::upload_dyn` (line 2989) — the ONLY
//     dynamic-card upload in the tree; called once per card
//     per frame from the loop in `Renderer::render` (line
//     3220): `for p in dyn_tex { let t = self.upload_dyn(
//     &p.0, p.1, p.2); dyn_views.push(t.create_view(...));
//     dyn_keep.push(t); }` where dyn_tex: &[Rc<Painted>],
//     Painted = (Vec<u8>, u32, u32, (f32,f32), (f32,f32))
//     (film.rs line 2252), canvases 2560x2816 RGBA today
//     (toonfull.rs: cw = 1280*2, ch = 1408*2). The pool does
//     NOT hardcode those dimensions — any (w, h) works.
//   * The texture descriptor below is upload_dyn's, field for
//     field: label "card texture", Rgba8UnormSrgb, mip 1,
//     sample 1, D2, TEXTURE_BINDING | COPY_DST, view_formats
//     []. The view is create_view(default) — upload_dyn's
//     caller spelling. The full-upload write is upload_dyn's
//     write_texture call, field for field.
//   * The one-time standing-texture closure in Renderer::new
//     (film.rs line 2619) and composer.rs's env-map upload are
//     OUT of scope (upload once at startup; dirtyrect's own
//     note says the same).
//
// Structure (so the policy is testable without a GPU):
//   * `SlotPlanner` / `DynPlanner` — PURE per-slot decision
//     state. `plan()` consults dirtyrect and returns an
//     `UploadPlan` (Full / Partial / Skip + the exact
//     `WriteSpec` list a partial upload must issue);
//     `commit()` records what was uploaded. No wgpu types.
//   * `DynTexturePool` — the GPU half: owns the textures +
//     views, creates/recreates them, issues the plan's writes
//     through queue.write_texture, and keeps the VRAM ledger
//     truthful (alloc once per texture creation, free on
//     recreate/drop — NOT per frame; see the MemScope warning
//     in the MD note: the pool must be driven OUTSIDE
//     render()'s MemScope, or the scope's E-class rollback
//     erases the standing registration).
//
// Caller contract (the parity argument in the MD note rests
// on these — all three are the lead's wiring, §2 of the note):
//   1. `prev` for a slot must be EXACTLY the pixel bytes last
//      uploaded to that slot (the previous Rc<Painted> the
//      caller retains per slot), or None. Anything else is a
//      caller bug the pool cannot detect in general — except
//      length mismatches, which ARE detected and forced to a
//      full upload (PreviousMismatch), never a partial.
//   2. The identical-allocation Skip fast path compares the
//      current pixel slice's data pointer + length against
//      the last committed upload's. It is sound ONLY while
//      the caller retains the previous allocation alive
//      across the interval (the retained Rc<Painted> does
//      exactly that). A caller that cannot guarantee
//      retention must still pass `prev`; with a live `prev`
//      the diff path reaches Skip on its own when nothing
//      changed, at the cost of one buffer scan.
//   3. `px.len()` must be w*h*4 (true for every Painted the
//      painter produces). Partial/Skip-via-diff are only ever
//      chosen when prev.len() == px.len() == w*h*4 exactly;
//      any other shape falls back to the full upload, which
//      is today's call with today's behaviour.
//
// Tests included (#[cfg(test)], pure planner + a simulated
// write_texture applier — no GPU): first-upload Full, resize
// Full, no-prev Full, mismatched-prev Full, identical
// allocation Skip, identical contents Skip via diff, one-pixel
// Partial with the exact WriteSpec, a deterministic scattered-
// mutation simulation proving partial application reproduces
// the current buffer byte-for-byte (the parity core), the
// 0.60 coverage fallback, the rect-count cap, the stride
// guard, slot independence, and plan byte accounting.
// ============================================================================

//! WP1.2 activation: the persistent Dyn-texture pool.
//!
//! One long-lived `Rgba8UnormSrgb` texture per dynamic-card
//! slot (`TexSrc::Dyn` index), created once and patched in
//! place. Per frame and slot, `dirtyrect`'s policy decides:
//! Skip (the texture already holds exactly these bytes),
//! Partial (one `write_texture` per merged dirty rect — the
//! union of merged rects is a superset of the changed texels,
//! so the patched texture ends byte-identical to a full
//! upload), or Full (the pre-existing `upload_dyn` upload,
//! verbatim — first frame, resize, no/invalid previous, or
//! the policy's coverage fallback).
//!
//! Pixel law: this module never decides a pixel value. It
//! only decides WHICH texels are (re)written, and every path
//! writes the current frame's bytes for every texel whose
//! value could differ from the texture's current contents.
//! The full argument, with every divergence case named, is in
//! docs/integration/CR-B1-08-MEMWIRE.md §3.

use crate::dirtyrect::{DirtyRects, CARD_BYTES_PER_PIXEL};

/// The dynamic cards' texture format — upload_dyn's, verbatim.
pub(crate) const CARD_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

/// Partial uploads stop paying long before this many calls:
/// past it, one full upload (a single call) replaces hundreds
/// of small ones. A performance cap only — Full is always a
/// parity-safe answer, so the cap can never change a pixel.
pub(crate) const MAX_PARTIAL_WRITES: usize = 256;

// ---------------------------------------------------------------------
// Plans (pure)
// ---------------------------------------------------------------------

/// What the pool decided for one slot on one frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum UploadKind {
    /// One full-canvas write_texture (today's upload_dyn call).
    Full,
    /// One write_texture per entry of `UploadPlan::writes`.
    Partial,
    /// No writes: the texture already holds these exact bytes.
    Skip,
}

/// Why a Full upload was chosen. Log/diagnostic only — every
/// reason produces the identical full upload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FullReason {
    /// No live texture for the slot yet (first frame, or the
    /// pool slot was never created).
    NewTexture,
    /// The slot's texture exists at different dimensions; it
    /// is recreated and fully written (old contents are
    /// undefined after recreation, so partial is forbidden).
    Resized,
    /// The caller supplied no previous pixels for the slot.
    NoPreviousPixels,
    /// The supplied previous pixels cannot be the texture's
    /// contents (length/dimension mismatch): never diff
    /// against a buffer that is not provably the resident one.
    PreviousMismatch,
    /// dirtyrect's policy: merged coverage > 0.60 of canvas.
    CoverageFallback,
    /// The merged set exceeds MAX_PARTIAL_WRITES rects.
    TooManyRects,
    /// The row stride (w*4) is not 256-byte aligned and a
    /// multi-row partial write would be needed: stay on the
    /// exact full-upload call shape the tree already exercises
    /// instead of a less-exercised layout. (Production cards
    /// are 2560 wide — stride 10240, aligned — so this never
    /// fires on the film path today.)
    StrideGuard,
}

/// Why a Skip was chosen (diagnostic only).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SkipReason {
    /// Same allocation (pointer + length) as the last commit,
    /// under the caller-retention contract.
    IdenticalAllocation,
    /// The byte diff against the previous pixels is empty.
    NoChanges,
}

/// One partial write, in exactly the terms
/// `queue.write_texture` consumes: copy the rect at
/// (origin_x, origin_y) of size (width, height) out of the
/// FULL current frame buffer, where the rect's first texel
/// sits at `src_offset` and rows are `bytes_per_row` apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct WriteSpec {
    pub origin_x: u32,
    pub origin_y: u32,
    pub src_offset: u64,
    pub bytes_per_row: u32,
    pub rows: u32,
    pub width: u32,
    pub height: u32,
}

impl WriteSpec {
    /// Texels this write moves.
    pub(crate) fn texels(&self) -> u64 {
        u64::from(self.width) * u64::from(self.height)
    }
}

/// A decided upload for one slot on one frame. (No Eq:
/// `coverage` is an f32.)
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct UploadPlan {
    pub kind: UploadKind,
    pub full_reason: Option<FullReason>,
    pub skip_reason: Option<SkipReason>,
    /// The writes to issue, in deterministic (merged, sorted)
    /// order. Full = exactly one whole-canvas WriteSpec;
    /// Partial = one per merged dirty rect; Skip = empty.
    pub writes: Vec<WriteSpec>,
    /// Bytes the writes move (texels x 4). Full = w*h*4,
    /// Skip = 0.
    pub bytes_moved: u64,
    /// Merged dirty coverage in [0.0, 1.0] when a diff ran;
    /// 1.0 for a no-diff Full, 0.0 for an identical-allocation
    /// Skip (no diff was needed to know).
    pub coverage: f32,
}

impl UploadPlan {
    fn full(reason: FullReason, w: u32, h: u32, coverage: f32) -> UploadPlan {
        UploadPlan {
            kind: UploadKind::Full,
            full_reason: Some(reason),
            skip_reason: None,
            writes: vec![WriteSpec {
                origin_x: 0,
                origin_y: 0,
                src_offset: 0,
                bytes_per_row: w * CARD_BYTES_PER_PIXEL,
                rows: h,
                width: w,
                height: h,
            }],
            bytes_moved: u64::from(w) * u64::from(h) * u64::from(CARD_BYTES_PER_PIXEL),
            coverage,
        }
    }

    fn skip(reason: SkipReason) -> UploadPlan {
        UploadPlan {
            kind: UploadKind::Skip,
            full_reason: None,
            skip_reason: Some(reason),
            writes: Vec::new(),
            bytes_moved: 0,
            coverage: 0.0,
        }
    }
}

/// Per-slot decision state (pure). `plan` is a query;
/// `commit` records the upload the caller actually issued,
/// and `invalidate` forgets the contents (texture recreated).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct SlotPlanner {
    w: u32,
    h: u32,
    has_contents: bool,
    last_ptr: usize,
    last_len: usize,
}

impl SlotPlanner {
    pub(crate) fn new() -> SlotPlanner {
        SlotPlanner::default()
    }

    /// True once a commit has landed and no invalidate since:
    /// the planner believes the texture holds known bytes.
    pub(crate) fn has_contents(&self) -> bool {
        self.has_contents
    }

    /// Forget the resident contents (the GPU half recreated
    /// the texture: its texels are undefined, so the next plan
    /// is Full and no Skip may be taken against the old state).
    pub(crate) fn invalidate(&mut self) {
        self.has_contents = false;
        self.last_ptr = 0;
        self.last_len = 0;
    }

    /// Decide this frame's upload for a slot whose texture is
    /// live at (tex_w, tex_h) — the GPU half passes its actual
    /// texture dimensions, or (0, 0) when it holds none.
    ///
    /// `px` is the current frame's full canvas; `prev` is the
    /// caller-retained bytes last uploaded to this slot (the
    /// delivery note's contract), or None.
    pub(crate) fn plan(
        &self,
        tex_w: u32,
        tex_h: u32,
        px: &[u8],
        w: u32,
        h: u32,
        prev: Option<&[u8]>,
    ) -> UploadPlan {
        // No live texture at these dimensions: the GPU half
        // will (re)create, and a fresh texture's contents are
        // undefined — Full is the only correct plan.
        if tex_w != w || tex_h != h || !self.has_contents {
            let reason = if tex_w == 0 && tex_h == 0 && !self.has_contents {
                FullReason::NewTexture
            } else if self.has_contents || (tex_w != 0 && tex_h != 0) {
                FullReason::Resized
            } else {
                FullReason::NewTexture
            };
            return UploadPlan::full(reason, w, h, 1.0);
        }
        // Identical-allocation fast path: the caller retains
        // the previous allocation alive (contract), so pointer
        // + length + dimensions equality means the texture
        // already holds this exact buffer. Checked before the
        // exact-length gate: a committed buffer was validated
        // by the frame that committed it.
        if px.as_ptr() as usize == self.last_ptr && px.len() == self.last_len && px.len() > 0 {
            return UploadPlan::skip(SkipReason::IdenticalAllocation);
        }
        // From here on, decisions are made by DIFFING, which is
        // only sound on exact-size buffers (contract point 3).
        let full_bytes = (w as usize)
            .checked_mul(h as usize)
            .and_then(|n| n.checked_mul(CARD_BYTES_PER_PIXEL as usize));
        let prev = match prev {
            None => return UploadPlan::full(FullReason::NoPreviousPixels, w, h, 1.0),
            Some(p) => p,
        };
        if full_bytes != Some(px.len()) || prev.len() != px.len() {
            return UploadPlan::full(FullReason::PreviousMismatch, w, h, 1.0);
        }
        let dirty = DirtyRects::from_diff(prev, px, w, h, CARD_BYTES_PER_PIXEL);
        if dirty.is_empty() {
            return UploadPlan::skip(SkipReason::NoChanges);
        }
        let coverage = dirty.coverage(w, h);
        if dirty.should_fall_back(w, h) {
            return UploadPlan::full(FullReason::CoverageFallback, w, h, coverage);
        }
        let merged = dirty.merged();
        if merged.len() > MAX_PARTIAL_WRITES {
            return UploadPlan::full(FullReason::TooManyRects, w, h, coverage);
        }
        // Stride guard (see FullReason::StrideGuard).
        let stride = w * CARD_BYTES_PER_PIXEL;
        if stride % wgpu::COPY_BYTES_PER_ROW_ALIGNMENT != 0
            && merged.iter().any(|r| r.h > 1)
        {
            return UploadPlan::full(FullReason::StrideGuard, w, h, coverage);
        }
        let writes: Vec<WriteSpec> = merged
            .iter()
            .map(|r| WriteSpec {
                origin_x: r.x,
                origin_y: r.y,
                src_offset: (u64::from(r.y) * u64::from(w) + u64::from(r.x))
                    * u64::from(CARD_BYTES_PER_PIXEL),
                bytes_per_row: stride,
                rows: r.h,
                width: r.w,
                height: r.h,
            })
            .collect();
        let bytes_moved =
            writes.iter().map(|s| s.texels()).sum::<u64>() * u64::from(CARD_BYTES_PER_PIXEL);
        UploadPlan {
            kind: UploadKind::Partial,
            full_reason: None,
            skip_reason: None,
            writes,
            bytes_moved,
            coverage,
        }
    }

    /// Record that `px` (at w x h) is now the texture's
    /// content. Called by the GPU half after it issues the
    /// plan's writes — for Skip plans too (the content claim
    /// is unchanged, but the newest allocation's pointer is
    /// the one future fast-path checks must compare against).
    pub(crate) fn commit(&mut self, px: &[u8], w: u32, h: u32) {
        self.w = w;
        self.h = h;
        self.has_contents = true;
        self.last_ptr = px.as_ptr() as usize;
        self.last_len = px.len();
    }
}

/// The planner for all slots (pure). Slot indices are the
/// TexSrc::Dyn indices — positional, exactly as render()
/// enumerates dyn_tex.
#[derive(Debug, Clone, Default)]
pub(crate) struct DynPlanner {
    slots: Vec<SlotPlanner>,
}

impl DynPlanner {
    pub(crate) fn new() -> DynPlanner {
        DynPlanner { slots: Vec::new() }
    }

    fn slot(&mut self, slot: usize) -> &mut SlotPlanner {
        if self.slots.len() <= slot {
            self.slots.resize_with(slot + 1, SlotPlanner::new);
        }
        &mut self.slots[slot]
    }

    pub(crate) fn plan(
        &mut self,
        slot: usize,
        tex_w: u32,
        tex_h: u32,
        px: &[u8],
        w: u32,
        h: u32,
        prev: Option<&[u8]>,
    ) -> UploadPlan {
        self.slot(slot).plan(tex_w, tex_h, px, w, h, prev)
    }

    pub(crate) fn commit(&mut self, slot: usize, px: &[u8], w: u32, h: u32) {
        self.slot(slot).commit(px, w, h);
    }

    pub(crate) fn invalidate(&mut self, slot: usize) {
        self.slot(slot).invalidate();
    }
}

// ---------------------------------------------------------------------
// Pool statistics (log-only)
// ---------------------------------------------------------------------

/// Lifetime upload accounting for the pool. Pure bookkeeping:
/// nothing here feeds a decision.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct PoolStats {
    pub full_uploads: u64,
    pub partial_uploads: u64,
    pub skips: u64,
    pub textures_created: u64,
    pub textures_recreated: u64,
    pub bytes_moved: u64,
    /// Full-canvas bytes the Skip/Partial plans did NOT move,
    /// against the always-full baseline (per upload: canvas
    /// bytes − bytes_moved). An upload-cost figure, never a
    /// memory figure.
    pub bytes_saved_vs_full: u64,
}

impl PoolStats {
    fn record(&mut self, plan: &UploadPlan, canvas_bytes: u64) {
        match plan.kind {
            UploadKind::Full => self.full_uploads += 1,
            UploadKind::Partial => self.partial_uploads += 1,
            UploadKind::Skip => self.skips += 1,
        }
        self.bytes_moved = self.bytes_moved.saturating_add(plan.bytes_moved);
        self.bytes_saved_vs_full = self
            .bytes_saved_vs_full
            .saturating_add(canvas_bytes.saturating_sub(plan.bytes_moved));
    }

    /// One-line summary in the tree's stats-line style
    /// (compare PaintBudget::stats_line / gpucore's gpu-mem
    /// line): single line, key=value fields. For the lead to
    /// print at end-of-run if wanted — printing it is optional
    /// and changes no existing line.
    pub(crate) fn stats_line(&self, live_textures: usize, ledger_bytes: u64) -> String {
        format!(
            "dyn-tex: live_textures={} ledger_bytes={} created={} recreated={} full={} partial={} skip={} bytes_moved={} bytes_saved_vs_full={}",
            live_textures,
            ledger_bytes,
            self.textures_created,
            self.textures_recreated,
            self.full_uploads,
            self.partial_uploads,
            self.skips,
            self.bytes_moved,
            self.bytes_saved_vs_full,
        )
    }
}

// ---------------------------------------------------------------------
// The GPU half
// ---------------------------------------------------------------------

/// One resident card texture + its default view.
struct GpuSlot {
    tex: wgpu::Texture,
    view: wgpu::TextureView,
    w: u32,
    h: u32,
    bytes: u64,
}

/// The persistent Dyn-texture pool. Owns one texture per slot,
/// drives the planner, and issues the writes. Lives in
/// film.rs's `Renderer` (lead wiring, MD note §2); the pool
/// itself is renderer-agnostic — it needs only a device and a
/// queue per upload call, exactly as upload_dyn used
/// self.device / self.queue.
pub(crate) struct DynTexturePool {
    planner: DynPlanner,
    gpu: Vec<Option<GpuSlot>>,
    stats: PoolStats,
}

impl DynTexturePool {
    pub(crate) fn new() -> DynTexturePool {
        DynTexturePool {
            planner: DynPlanner::new(),
            gpu: Vec::new(),
            stats: PoolStats::default(),
        }
    }

    /// Create a fresh card texture — upload_dyn's descriptor,
    /// field for field — and register it with the VRAM ledger
    /// (class E, once: this is a STANDING allocation now, not
    /// a per-frame transient; see the MemScope warning in the
    /// delivery note / MD note §2).
    fn create_slot(
        device: &wgpu::Device,
        w: u32,
        h: u32,
    ) -> GpuSlot {
        let tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("card texture"),
            size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: CARD_FORMAT,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let bytes = crate::gpucore::tex_bytes(w, h, CARD_FORMAT, 1);
        crate::gpucore::mem_alloc_texture(crate::gpucore::MemClass::E, bytes);
        let view = tex.create_view(&wgpu::TextureViewDescriptor::default());
        GpuSlot { tex, view, w, h, bytes }
    }

    /// Upload one card for one frame: plan against the slot's
    /// resident texture, (re)create the texture when the plan
    /// requires it, issue the plan's writes, commit. Returns
    /// the plan (for stats/logging at the call site).
    ///
    /// `prev` is the delivery note's contract value: the exact
    /// bytes last uploaded to this slot, or None.
    pub(crate) fn upload(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        slot: usize,
        px: &[u8],
        w: u32,
        h: u32,
        prev: Option<&[u8]>,
    ) -> UploadPlan {
        if self.gpu.len() <= slot {
            self.gpu.resize_with(slot + 1, || None);
        }
        let (tex_w, tex_h) = match &self.gpu[slot] {
            Some(s) => (s.w, s.h),
            None => (0, 0),
        };
        let plan = self.planner.plan(slot, tex_w, tex_h, px, w, h, prev);

        // (Re)creation is driven by the plan's premise: a Full
        // plan taken because there is no live texture at these
        // dimensions must create before writing. Partial/Skip
        // plans only exist when the texture is live at (w, h).
        let needs_create = tex_w != w || tex_h != h;
        if needs_create {
            if let Some(old) = self.gpu[slot].take() {
                crate::gpucore::mem_free_texture(crate::gpucore::MemClass::E, old.bytes);
                self.stats.textures_recreated += 1;
                // The old GpuSlot (and its wgpu::Texture) drops
                // here — after the ledger free, before the new
                // creation registers.
                drop(old);
            } else {
                self.stats.textures_created += 1;
            }
            self.planner.invalidate(slot);
            self.gpu[slot] = Some(Self::create_slot(device, w, h));
        }

        let gpu_slot = self.gpu[slot].as_ref().expect("slot created above");
        for write in &plan.writes {
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &gpu_slot.tex,
                    mip_level: 0,
                    origin: wgpu::Origin3d { x: write.origin_x, y: write.origin_y, z: 0 },
                    aspect: wgpu::TextureAspect::All,
                },
                px,
                wgpu::TexelCopyBufferLayout {
                    offset: write.src_offset,
                    bytes_per_row: Some(write.bytes_per_row),
                    rows_per_image: Some(write.rows),
                },
                wgpu::Extent3d { width: write.width, height: write.height, depth_or_array_layers: 1 },
            );
        }
        self.planner.commit(slot, px, w, h);
        self.stats.record(
            &plan,
            u64::from(w) * u64::from(h) * u64::from(CARD_BYTES_PER_PIXEL),
        );
        plan
    }

    /// The resident texture's view for a slot — what the
    /// caller binds in place of upload_dyn's per-frame view.
    /// None when the slot has never been uploaded.
    pub(crate) fn view(&self, slot: usize) -> Option<&wgpu::TextureView> {
        self.gpu.get(slot).and_then(|s| s.as_ref()).map(|s| &s.view)
    }

    /// Live (created, not since replaced) texture count.
    pub(crate) fn live_textures(&self) -> usize {
        self.gpu.iter().filter(|s| s.is_some()).count()
    }

    /// Bytes the pool's live textures account in the ledger —
    /// the figure the MemScope warning is about; the lead can
    /// cross-check it against gpucore's class-E current.
    pub(crate) fn ledger_bytes(&self) -> u64 {
        self.gpu
            .iter()
            .filter_map(|s| s.as_ref())
            .map(|s| s.bytes)
            .sum()
    }

    pub(crate) fn stats(&self) -> PoolStats {
        self.stats
    }

    /// The stats line (PoolStats::stats_line with the pool's
    /// live figures filled in).
    pub(crate) fn stats_line(&self) -> String {
        self.stats.stats_line(self.live_textures(), self.ledger_bytes())
    }
}

impl Drop for DynTexturePool {
    fn drop(&mut self) {
        // Standing registrations leave with the textures.
        for slot in self.gpu.iter().flatten() {
            crate::gpucore::mem_free_texture(crate::gpucore::MemClass::E, slot.bytes);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A w x h RGBA canvas filled with one byte value.
    fn canvas(w: u32, h: u32, fill: u8) -> Vec<u8> {
        vec![fill; (w * h * 4) as usize]
    }

    /// Simulate queue.write_texture's copy semantics for a
    /// plan's writes onto a resident texture buffer: for each
    /// write, for each row, copy width*4 bytes from the full
    /// source buffer at src_offset + row*bytes_per_row into
    /// the texture at ((origin_y+row)*w + origin_x)*4. This is
    /// the semantics wgpu implements for the exact call the
    /// GPU half issues; the parity tests below assert the
    /// simulated resident buffer ends byte-identical to the
    /// current frame buffer.
    fn simulate_apply(plan: &UploadPlan, cur: &[u8], tex_mem: &mut [u8], w: u32) {
        for write in &plan.writes {
            for row in 0..write.rows {
                let src = (write.src_offset + u64::from(row) * u64::from(write.bytes_per_row))
                    as usize;
                let dst = (((write.origin_y + row) * w + write.origin_x) * 4) as usize;
                let n = (write.width * 4) as usize;
                tex_mem[dst..dst + n].copy_from_slice(&cur[src..src + n]);
            }
        }
    }

    #[test]
    fn first_upload_is_full_new_texture() {
        let mut p = DynPlanner::new();
        let px = canvas(8, 8, 7);
        let plan = p.plan(0, 0, 0, &px, 8, 8, None);
        assert_eq!(plan.kind, UploadKind::Full);
        assert_eq!(plan.full_reason, Some(FullReason::NewTexture));
        assert_eq!(plan.bytes_moved, 8 * 8 * 4);
        assert_eq!(plan.writes.len(), 1);
        assert_eq!(plan.writes[0].width, 8);
        assert_eq!(plan.writes[0].height, 8);
    }

    #[test]
    fn no_previous_pixels_is_full() {
        let mut p = DynPlanner::new();
        let px = canvas(8, 8, 7);
        let first = p.plan(0, 0, 0, &px, 8, 8, None);
        assert_eq!(first.kind, UploadKind::Full);
        p.commit(0, &px, 8, 8);
        // A later frame whose caller lost the previous pixels
        // (slot reappeared after the caller pruned its retained
        // copy) must not diff against nothing.
        let px2 = canvas(8, 8, 9);
        let plan = p.plan(0, 8, 8, &px2, 8, 8, None);
        assert_eq!(plan.kind, UploadKind::Full);
        assert_eq!(plan.full_reason, Some(FullReason::NoPreviousPixels));
    }

    #[test]
    fn resize_is_full_and_partial_is_forbidden() {
        let mut p = DynPlanner::new();
        let px = canvas(8, 8, 7);
        p.commit(0, &px, 8, 8);
        let big = canvas(16, 16, 7);
        // Texture still at 8x8 (planner is told the GPU half's
        // actual size): a resize can never be a partial plan.
        let plan = p.plan(0, 8, 8, &big, 16, 16, Some(&px));
        assert_eq!(plan.kind, UploadKind::Full);
        assert_eq!(plan.full_reason, Some(FullReason::Resized));
    }

    #[test]
    fn mismatched_previous_is_full_never_partial() {
        let mut p = DynPlanner::new();
        let px = canvas(8, 8, 7);
        p.commit(0, &px, 8, 8);
        let cur = canvas(8, 8, 8);
        // Truncated "previous" (a caller bug): the diff would
        // silently miss rows — the planner must refuse it.
        let short = vec![7u8; 100];
        let plan = p.plan(0, 8, 8, &cur, 8, 8, Some(&short));
        assert_eq!(plan.kind, UploadKind::Full);
        assert_eq!(plan.full_reason, Some(FullReason::PreviousMismatch));
        // Truncated current buffer: same refusal.
        let plan = p.plan(0, 8, 8, &short, 8, 8, Some(&px));
        assert_eq!(plan.kind, UploadKind::Full);
        assert_eq!(plan.full_reason, Some(FullReason::PreviousMismatch));
    }

    #[test]
    fn identical_allocation_skips_without_a_diff() {
        let mut p = DynPlanner::new();
        let px = canvas(8, 8, 7);
        p.commit(0, &px, 8, 8);
        // Same Vec (the retained Rc<Painted> case): even a
        // garbage `prev` is never consulted on this path.
        let plan = p.plan(0, 8, 8, &px, 8, 8, Some(&[1, 2, 3]));
        assert_eq!(plan.kind, UploadKind::Skip);
        assert_eq!(plan.skip_reason, Some(SkipReason::IdenticalAllocation));
        assert_eq!(plan.bytes_moved, 0);
        assert!(plan.writes.is_empty());
    }

    #[test]
    fn identical_contents_in_a_new_allocation_skip_via_diff() {
        let mut p = DynPlanner::new();
        let px = canvas(8, 8, 7);
        p.commit(0, &px, 8, 8);
        let repaint = canvas(8, 8, 7); // different Vec, same bytes
        let plan = p.plan(0, 8, 8, &repaint, 8, 8, Some(&px));
        assert_eq!(plan.kind, UploadKind::Skip);
        assert_eq!(plan.skip_reason, Some(SkipReason::NoChanges));
    }

    #[test]
    fn one_pixel_change_is_a_one_texel_partial() {
        let mut p = DynPlanner::new();
        let prev = canvas(16, 16, 0);
        p.commit(0, &prev, 16, 16);
        let mut cur = prev.clone();
        cur[((5 * 16 + 3) * 4) as usize] = 255; // texel (3, 5)
        let plan = p.plan(0, 16, 16, &cur, 16, 16, Some(&prev));
        assert_eq!(plan.kind, UploadKind::Partial);
        assert_eq!(plan.writes.len(), 1);
        let ws = plan.writes[0];
        assert_eq!((ws.origin_x, ws.origin_y, ws.width, ws.height), (3, 5, 1, 1));
        assert_eq!(ws.src_offset, (5 * 16 + 3) * 4);
        assert_eq!(ws.bytes_per_row, 16 * 4);
        assert_eq!(ws.rows, 1);
        assert_eq!(plan.bytes_moved, 4);
    }

    #[test]
    fn partial_application_reproduces_the_current_buffer() {
        // THE parity test: deterministic scattered mutations
        // (an LCG, fixed seed — no randomness across runs) over
        // a mid-size canvas; apply the plan to a simulated
        // resident texture holding `prev`; the result must be
        // byte-identical to `cur`, exactly as a full upload
        // would leave it.
        let (w, h) = (64u32, 96u32);
        let mut p = DynPlanner::new();
        let prev = canvas(w, h, 0);
        p.commit(0, &prev, w, h);
        let mut cur = prev.clone();
        let mut state: u32 = 0x1234_5678;
        let mut next = move || {
            state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            state
        };
        // Small clusters confined to one localised region
        // (the blink/mouth case the policy exists for). The
        // confinement matters: add_diff works in whole-row
        // spans, so changes scattered across the full width
        // inflate the merged coverage towards the fallback —
        // measured while writing this test (full-width scatter
        // at this density covers >90% after row-spanning).
        for _ in 0..40 {
            let x = 8 + next() % 16;
            let y = 40 + next() % 20;
            for dy in 0..3 {
                for dx in 0..3 {
                    let k = (((y + dy) * w + (x + dx)) * 4) as usize;
                    cur[k] = 200;
                    cur[k + 3] = 255;
                }
            }
        }
        let plan = p.plan(0, w, h, &cur, w, h, Some(&prev));
        assert_eq!(plan.kind, UploadKind::Partial, "plan: {plan:?}");
        let mut tex_mem = prev.clone();
        simulate_apply(&plan, &cur, &mut tex_mem, w);
        assert_eq!(tex_mem, cur, "patched texture must equal a full upload byte-for-byte");
        // Byte accounting: moved = merged area x 4, well under
        // the full canvas for this change scale.
        assert!(plan.bytes_moved < u64::from(w * h * 4) / 2, "moved {}", plan.bytes_moved);
        assert!(plan.coverage > 0.0 && plan.coverage <= 0.60);
    }

    #[test]
    fn coverage_above_the_threshold_falls_back_to_full() {
        let mut p = DynPlanner::new();
        let (w, h) = (16u32, 16u32);
        let prev = canvas(w, h, 0);
        p.commit(0, &prev, w, h);
        let cur = canvas(w, h, 255); // every texel changed
        let plan = p.plan(0, w, h, &cur, w, h, Some(&prev));
        assert_eq!(plan.kind, UploadKind::Full);
        assert_eq!(plan.full_reason, Some(FullReason::CoverageFallback));
        assert_eq!(plan.bytes_moved, u64::from(w * h * 4));
    }

    #[test]
    fn too_many_rects_falls_back_to_full() {
        // Isolated single-texel changes on every OTHER row:
        // nothing merges, so the rect count is the row count.
        let (w, h) = (4u32, 1024u32);
        let mut p = DynPlanner::new();
        let prev = canvas(w, h, 0);
        p.commit(0, &prev, w, h);
        let mut cur = prev.clone();
        for y in (0..h).step_by(2) {
            cur[(y * w * 4) as usize] = 9;
        }
        // Coverage is tiny (512 / 4096 texels) — only the rect
        // cap can fire.
        let plan = p.plan(0, w, h, &cur, w, h, Some(&prev));
        assert_eq!(plan.kind, UploadKind::Full);
        assert_eq!(plan.full_reason, Some(FullReason::TooManyRects));
    }

    #[test]
    fn unaligned_stride_multirow_change_falls_back_single_row_partials() {
        // w = 65 -> stride 260, not a multiple of 256.
        let (w, h) = (65u32, 8u32);
        let mut p = DynPlanner::new();
        let prev = canvas(w, h, 0);
        p.commit(0, &prev, w, h);
        // Two adjacent changed rows merge into an h=2 rect.
        let mut cur = prev.clone();
        cur[(2 * w * 4) as usize] = 9;
        cur[(3 * w * 4) as usize] = 9;
        let plan = p.plan(0, w, h, &cur, w, h, Some(&prev));
        assert_eq!(plan.kind, UploadKind::Full);
        assert_eq!(plan.full_reason, Some(FullReason::StrideGuard));
        // A single-row change stays partial (rows_per_image = 1
        // never exercises the stride between rows).
        let mut cur1 = prev.clone();
        cur1[(2 * w * 4) as usize] = 9;
        let plan = p.plan(0, w, h, &cur1, w, h, Some(&prev));
        assert_eq!(plan.kind, UploadKind::Partial);
        assert_eq!(plan.writes[0].rows, 1);
    }

    #[test]
    fn slots_are_independent() {
        let mut p = DynPlanner::new();
        let a = canvas(8, 8, 1);
        let b = canvas(8, 8, 2);
        p.commit(0, &a, 8, 8);
        p.commit(1, &b, 8, 8);
        // Slot 0 unchanged, slot 1 changed in one texel.
        let plan0 = p.plan(0, 8, 8, &a, 8, 8, Some(&a));
        assert_eq!(plan0.kind, UploadKind::Skip);
        let mut b2 = b.clone();
        b2[0] = 99;
        let plan1 = p.plan(1, 8, 8, &b2, 8, 8, Some(&b));
        assert_eq!(plan1.kind, UploadKind::Partial);
        // Slot 2 was never committed: Full, unaffected by 0/1.
        let plan2 = p.plan(2, 0, 0, &a, 8, 8, None);
        assert_eq!(plan2.kind, UploadKind::Full);
    }

    #[test]
    fn invalidate_forces_full_after_recreate() {
        let mut p = DynPlanner::new();
        let px = canvas(8, 8, 7);
        p.commit(0, &px, 8, 8);
        p.invalidate(0);
        // Same allocation, but the texture was recreated: its
        // contents are undefined, so the fast path must NOT
        // fire — the planner reports no live texture (0, 0 is
        // what the GPU half passes after recreation begins;
        // here we pass the planner's own view: has_contents is
        // false, which alone forces Full).
        let plan = p.plan(0, 8, 8, &px, 8, 8, Some(&px));
        assert_eq!(plan.kind, UploadKind::Full);
    }

    #[test]
    fn stats_record_and_line() {
        let mut stats = PoolStats::default();
        let full = UploadPlan::full(FullReason::NewTexture, 8, 8, 1.0);
        stats.record(&full, 8 * 8 * 4);
        let skip = UploadPlan::skip(SkipReason::NoChanges);
        stats.record(&skip, 8 * 8 * 4);
        assert_eq!(stats.full_uploads, 1);
        assert_eq!(stats.skips, 1);
        assert_eq!(stats.bytes_moved, 256);
        assert_eq!(stats.bytes_saved_vs_full, 256);
        let line = stats.stats_line(2, 512);
        assert!(line.starts_with("dyn-tex: live_textures=2 ledger_bytes=512"), "got: {line}");
        assert!(line.contains("full=1 partial=0 skip=1"), "got: {line}");
    }
}
