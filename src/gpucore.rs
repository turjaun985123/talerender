//! gpucore — the shared GPU primitives for TaleRender.
//!
//! Wave 2, seat W07 (audit finding B): wgpu instance/adapter/device
//! creation, offscreen target creation, and texture→CPU readback
//! were re-implemented at five render entry points (main.rs,
//! ab1.rs, dof.rs, scenefile.rs, film.rs) with small divergences —
//! different device limits, different labels, two spellings of the
//! row-padding arithmetic, two PNG save spellings, four different
//! map-error messages. This module is the single canonical copy of
//! each primitive, mirroring the STRONGEST existing behaviour:
//!
//!   - adapter request = main.rs::request_adapter's two-step
//!     fallback (HighPerformance first, force-fallback second),
//!     already shared by every site via crate::request_adapter;
//!   - device limits are a PARAMETER. The default is standard
//!     limits (`wgpu::Limits::default()`): the real character
//!     canvas is 2560×2816, which downlevel limits (2048 texture
//!     cap) cannot hold — see the comment at ab1.rs's device
//!     request. main.rs::run is the one site still passing
//!     downlevel limits; it opts in explicitly at migration time;
//!   - readback records into the CALLER's encoder (every image
//!     site submits render + copy in one batch — on the film path
//!     that batching is per-frame and load-bearing), then maps
//!     with the mpsc + `device.poll(Wait)` idiom all sites use.
//!
//! PIXEL LAW: every function here is a structural move of code
//! that already ran; none of them may change a rendered pixel, a
//! PNG byte, a printed line, or an exit code. The migration plan
//! (docs/integration/GPU-CORE-W07.md) pairs each call site with
//! the A/B pin that proves it.
//!
//! VERIFICATION: the pure arithmetic (row padding, padding strip)
//! is unit-tested below on CPU. Everything that touches a GPU is
//! **CI-verified only** — the dev VM has no GPU and its llvmpipe
//! JIT segfaults on shader_pbr, so GPU paths are proven by the
//! existing CI pins (ab1 23.96 dB pair, dof-chart numbers,
//! fx-textures byte-identity, film chunk probe), never by a local
//! run. No new dependencies.

use std::sync::mpsc;

// ---------------------------------------------------------------------
// Instance + adapter + device
// ---------------------------------------------------------------------

/// The instance every render entry point builds — byte-identical
/// at all five existing sites (Vulkan backend only, rest default).
pub(crate) fn new_instance() -> wgpu::Instance {
    // wgpu 30: InstanceDescriptor no longer implements Default;
    // new_without_display_handle() is its default constructor.
    let mut desc = wgpu::InstanceDescriptor::new_without_display_handle();
    desc.backends = wgpu::Backends::VULKAN;
    wgpu::Instance::new(desc)
}

/// Canonical adapter request — the verbatim semantics of the
/// helper formerly defined in main.rs and already called by every
/// site: try a high-performance adapter first; if none answers,
/// retry with the fallback (software) adapter forced. When no
/// adapter exists at all it returns `GpuInit` (exit 3) carrying
/// the project's established message VERBATIM as its source —
/// that message is load-bearing (PERF.md's failure table quotes
/// it for the "No adapter" row; ERRORS-W05 step 2 / W35 §5).
pub(crate) async fn request_adapter(
    instance: &wgpu::Instance,
) -> crate::errors::TaleResult<wgpu::Adapter> {
    // wgpu 30: request_adapter returns Result (was Option in
    // 0.20); apply_limit_buckets: false keeps the adapter's
    // exact limits (0.20 behaviour).
    if let Ok(adapter) = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
            apply_limit_buckets: false,
        })
        .await
    {
        return Ok(adapter);
    }
    instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: None,
            force_fallback_adapter: true,
            apply_limit_buckets: false,
        })
        .await
        .map_err(|_| {
            crate::errors::TaleError::gpu_init(
                "request_adapter (gpucore)",
                "no wgpu adapter found — a Vulkan driver (hardware or lavapipe) is required",
            )
        })
}

/// Device + queue, failures as `DeviceRequest` (exit 3).
/// `limits` is explicit because the sites legitimately differ
/// today (standard everywhere except main.rs::run's downlevel);
/// new code should use `request_device_default`.
pub(crate) async fn request_device(
    adapter: &wgpu::Adapter,
    label: &'static str,
    limits: wgpu::Limits,
) -> crate::errors::TaleResult<(wgpu::Device, wgpu::Queue)> {
    adapter
        .request_device(&wgpu::DeviceDescriptor {
            label: Some(label),
            required_features: wgpu::Features::empty(),
            required_limits: limits,
            ..Default::default()
        })
        .await
        .map_err(|e| {
            crate::errors::TaleError::device_request(
                format!("request_device ({label})"),
                e,
            )
        })
}

/// The common case: standard limits (NOT downlevel — those cap
/// textures at 2048 and the character canvas is 2560×2816).
pub(crate) async fn request_device_default(
    adapter: &wgpu::Adapter,
    label: &'static str,
) -> crate::errors::TaleResult<(wgpu::Device, wgpu::Queue)> {
    request_device(adapter, label, wgpu::Limits::default()).await
}

// ---------------------------------------------------------------------
// Offscreen targets
// ---------------------------------------------------------------------

/// One offscreen texture — the verbatim body of the identical
/// `make_target` helpers in scenefile.rs and dof.rs, generalised
/// from the inline creations in main.rs / ab1.rs / film.rs.
///
/// `format` is a parameter and must stay one: the direct paths
/// render into `Rgba8UnormSrgb` (hardware sRGB on store) while
/// the composer paths finish into `Rgba8Unorm` (manual sRGB in
/// the Output pass) and work in `Rgba16Float` between stages.
/// Hardcoding either format here would change pixels.
pub(crate) fn make_target(
    device: &wgpu::Device,
    w: u32,
    h: u32,
    format: wgpu::TextureFormat,
    samples: u32,
    usage: wgpu::TextureUsages,
    label: &str,
) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: samples,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage,
        view_formats: &[],
    })
}

/// `make_target` plus the default view every inline site creates
/// immediately after (frame target, depth target, final target).
pub(crate) fn make_target_viewed(
    device: &wgpu::Device,
    w: u32,
    h: u32,
    format: wgpu::TextureFormat,
    samples: u32,
    usage: wgpu::TextureUsages,
    label: &str,
) -> (wgpu::Texture, wgpu::TextureView) {
    let tex = make_target(device, w, h, format, samples, usage, label);
    let view = tex.create_view(&wgpu::TextureViewDescriptor::default());
    (tex, view)
}

// ---------------------------------------------------------------------
// Readback: texture -> buffer -> tight RGBA bytes
// ---------------------------------------------------------------------

/// Bytes in one unpadded row of a `width`-px texture.
pub(crate) fn unpadded_row_bytes(width_px: u32, bytes_per_pixel: u32) -> u32 {
    width_px * bytes_per_pixel
}

/// Row stride a `copy_texture_to_buffer` requires: the unpadded
/// row rounded up to wgpu's 256-byte COPY_BYTES_PER_ROW_ALIGNMENT.
/// Generalises math::padded_row_bytes (which is this at
/// bytes_per_pixel = 4); the two agree at 4 bpp — pinned by a
/// unit test below so the spellings can never drift apart.
pub(crate) fn padded_row_bytes(width_px: u32, bytes_per_pixel: u32) -> u32 {
    unpadded_row_bytes(width_px, bytes_per_pixel)
        .div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT)
        * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT
}

/// Strip per-row padding from a mapped readback buffer, returning
/// tight rows (`width * bytes_per_pixel` bytes each). Pure CPU —
/// this is the exact loop the six existing sites spell two ways
/// (extend-into-Vec and copy-into-preallocated-Vec); both produce
/// these bytes. Unit-tested below.
pub(crate) fn strip_padding(
    mapped: &[u8],
    width: u32,
    height: u32,
    bytes_per_pixel: u32,
    padded_row: u32,
) -> Vec<u8> {
    let row = unpadded_row_bytes(width, bytes_per_pixel) as usize;
    let stride = padded_row as usize;
    let mut out = Vec::with_capacity(row * height as usize);
    for y in 0..height as usize {
        out.extend_from_slice(&mapped[y * stride..y * stride + row]);
    }
    out
}

/// A recorded texture→buffer copy awaiting submission + mapping.
///
/// Two-phase on purpose: every image readback site records the
/// copy into the encoder that already holds the frame's render
/// work and submits once. Splitting record/collect keeps that
/// batching (on film.rs it is one submit per frame — a second
/// submit would be a hot-path change, not a refactor).
pub(crate) struct Readback {
    buffer: wgpu::Buffer,
    width: u32,
    height: u32,
    bytes_per_pixel: u32,
    padded_row: u32,
    label: &'static str,
}

impl Readback {
    /// Record a copy of an RGBA8 texture into a fresh readback
    /// buffer on `encoder`. The caller submits the encoder, then
    /// calls `collect`.
    pub(crate) fn record(
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        texture: &wgpu::Texture,
        width: u32,
        height: u32,
        label: &'static str,
    ) -> Readback {
        Self::record_bpp(device, encoder, texture, width, height, 4, label)
    }

    /// `record` for non-RGBA8 sources (e.g. dof.rs's packed-depth
    /// dump reads 4- or 8-byte pixels).
    pub(crate) fn record_bpp(
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        texture: &wgpu::Texture,
        width: u32,
        height: u32,
        bytes_per_pixel: u32,
        label: &'static str,
    ) -> Readback {
        let padded_row = padded_row_bytes(width, bytes_per_pixel);
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size: (padded_row * height) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        // VRAM ledger (W84 §5.2): the staging buffer is freed
        // inside collect, before any print — peak-side note
        // only, never a standing allocation.
        mem_note_readback((padded_row * height) as u64);
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_row),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
        );
        Readback { buffer, width, height, bytes_per_pixel, padded_row, label }
    }

    /// Map the buffer (after the caller submitted the encoder),
    /// wait, and return the tight rows with padding stripped.
    /// Map failures are `BufferMap` (exit 3) since ERRORS-W05
    /// step 2 — the two layers stay distinguishable in the error
    /// source ("map channel closed" vs the wgpu map error).
    pub(crate) fn collect(self, device: &wgpu::Device) -> crate::errors::TaleResult<Vec<u8>> {
        let (width, height, bpp, padded_row) =
            (self.width, self.height, self.bytes_per_pixel, self.padded_row);
        let (padded, _) = self.collect_padded(device)?;
        Ok(strip_padding(&padded, width, height, bpp, padded_row))
    }

    /// Like `collect`, but returns the raw padded rows plus the
    /// row stride — for callers that index the mapped layout in
    /// place (dof.rs's packed-depth panel sampler).
    pub(crate) fn collect_padded(
        self,
        device: &wgpu::Device,
    ) -> crate::errors::TaleResult<(Vec<u8>, u32)> {
        let ctx = format!("readback collect ({})", self.label);
        let slice = self.buffer.slice(..);
        let (tx, rx) = mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = tx.send(result);
        });
        // wgpu 30: Maintain::Wait's replacement is the
        // struct variant Wait { None, None } — wait for all
        // queued submissions, no timeout (0.20 semantics).
        device.poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: None,
        });
        rx.recv()
            .map_err(|_| crate::errors::TaleError::buffer_map(&ctx, "map channel closed"))?
            .map_err(|e| crate::errors::TaleError::buffer_map(&ctx, e))?;
        // wgpu 30: get_mapped_range is fallible (was infallible
        // in 0.20).
        let data = slice
            .get_mapped_range()
            .map_err(|e| crate::errors::TaleError::buffer_map(&ctx, e))?;
        let out = data.to_vec();
        drop(data);
        self.buffer.unmap();
        Ok((out, self.padded_row))
    }
}

/// Self-contained RGBA8 readback: own encoder, own submit — the
/// shape dof.rs's packed-depth dump already uses, for callers
/// with no encoder of their own in flight.
pub(crate) fn readback_rgba(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    width: u32,
    height: u32,
) -> crate::errors::TaleResult<Vec<u8>> {
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("gpucore readback"),
    });
    let rb = Readback::record(device, &mut encoder, texture, width, height, "readback buffer");
    queue.submit(Some(encoder.finish()));
    rb.collect(device)
}

/// Save tight RGBA8 pixels as a PNG — the majority spelling
/// (main.rs / ab1.rs / scenefile.rs direct path / film.rs's frame
/// writer): RgbaImage::from_raw + save, same expect messages.
/// dof.rs and scenefile.rs's composer path spell it
/// `image::save_buffer`; both go through the image crate's PNG
/// encoder, and the fx-textures byte-identity pin guards the
/// composer-path migration (see GPU-CORE-W07.md).
pub(crate) fn save_png(path: &str, pixels: &[u8], width: u32, height: u32) {
    let img = image::RgbaImage::from_raw(width, height, pixels.to_vec())
        .expect("pixel buffer size mismatch");
    img.save(path).expect("failed to save PNG");
}

// ---------------------------------------------------------------------
// GPU memory ledger (VRAM telemetry, VRAM-IMPL-W84)
// ---------------------------------------------------------------------
//
// A process-global accounting of the device's memory AS THE
// BINARY ALLOCATES IT: every texture/buffer creation site in
// src/ either flows through a chokepoint that registers here
// (make_target callers register per-caller; composer::make_buffer
// and Readback::record_bpp register in their bodies) or carries
// its own registration line at the site — the tools-ci grep
// check (tools/ci/check_mem_registration.py) enforces that no
// site is silent. Log-only: the ledger reads descriptors and
// returns numbers; it is never on a pixel path, so determinism
// is untouched (W84 §5.1).
//
// Classes are VRAM-TELEMETRY-W56 §4's letters: A frame targets,
// B shadow pair, C bloom chain, D PMREM, E uploads, F readback
// (peak-side only — never enters `current`), G buffers. HOST
// memory is never ingested: the paint cache's canvases are CPU
// Vec<u8> and print on their own `paint-cache:` line (W84 §7.1);
// BufferPool bytes are not registered either (the pool has no
// production caller; if Readback ever adopts it, its retained
// buffers register ONCE at pool retention, never per collect —
// W84 §7.2). `accounted_bytes` is a LOWER BOUND by construction:
// driver tiling/alignment beyond the 256-byte row rule,
// compression metadata, pipelines, shader modules, bind groups
// and samplers are inside the driver, invisible to wgpu 0.20,
// and unpriced here (W84 §7.3).

/// Ledger classes (W56 §4). F is note-only (see
/// `mem_note_readback`); it never contributes to `current`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum MemClass {
    A,
    B,
    C,
    D,
    E,
    F,
    G,
}

impl MemClass {
    fn idx(self) -> usize {
        match self {
            MemClass::A => 0,
            MemClass::B => 1,
            MemClass::C => 2,
            MemClass::D => 3,
            MemClass::E => 4,
            MemClass::F => 5,
            MemClass::G => 6,
        }
    }
}

#[derive(Clone, Copy)]
struct GpuMem {
    bytes: [u64; 7],
    peak: u64,
    tex_counts: [u64; 7],
    buf_counts: [u64; 7],
    readback_max: u64,
    // V2 C2 (WP2): per-class peak bytes + lifetime
    // registration/free call counters for mem_snapshot().
    // Watermarks and lifetime totals: MemScope never rolls
    // these back, exactly like `peak`.
    peak_bytes: [u64; 7],
    created_tex: u64,
    created_buf: u64,
    destroyed_tex: u64,
    destroyed_buf: u64,
}

impl GpuMem {
    const fn new() -> GpuMem {
        GpuMem {
            bytes: [0; 7],
            peak: 0,
            tex_counts: [0; 7],
            buf_counts: [0; 7],
            readback_max: 0,
            peak_bytes: [0; 7],
            created_tex: 0,
            created_buf: 0,
            destroyed_tex: 0,
            destroyed_buf: 0,
        }
    }

    /// Live accounted bytes: classes A–E + G. Class F is
    /// peak-side only and is never summed in.
    fn current(&self) -> u64 {
        self.bytes.iter().sum::<u64>() - self.bytes[MemClass::F.idx()]
    }

    fn note_peak(&mut self) {
        let c = self.current();
        if c > self.peak {
            self.peak = c;
        }
    }
}

static GPU_MEM: std::sync::Mutex<GpuMem> = std::sync::Mutex::new(GpuMem::new());

fn with_mem<R>(f: impl FnOnce(&mut GpuMem) -> R) -> R {
    // The critical sections are pure arithmetic (no I/O, no
    // calls back into wgpu), so a poisoned lock can only come
    // from a panic mid-arithmetic — recover the state rather
    // than cascade a logging failure into a render failure.
    let mut guard = GPU_MEM.lock().unwrap_or_else(|e| e.into_inner());
    f(&mut guard)
}

/// Bytes of one texture allocation: `w × h × bpp(format) ×
/// samples`, layers = 1 (no site in the tree creates a layered
/// texture). The byte-per-pixel comes from wgpu's own block
/// size table so a new format can never be silently mispriced.
pub(crate) fn tex_bytes(w: u32, h: u32, format: wgpu::TextureFormat, samples: u32) -> u64 {
    let bpp = format.block_copy_size(None).unwrap_or(4) as u64;
    w as u64 * h as u64 * bpp * samples.max(1) as u64
}

pub(crate) fn mem_alloc_texture(class: MemClass, bytes: u64) {
    with_mem(|m| {
        let i = class.idx();
        m.bytes[i] = m.bytes[i].saturating_add(bytes);
        m.tex_counts[i] = m.tex_counts[i].saturating_add(1);
        m.created_tex = m.created_tex.saturating_add(1);
        if m.bytes[i] > m.peak_bytes[i] {
            m.peak_bytes[i] = m.bytes[i];
        }
        m.note_peak();
    });
}

pub(crate) fn mem_free_texture(class: MemClass, bytes: u64) {
    with_mem(|m| {
        let i = class.idx();
        m.bytes[i] = m.bytes[i].saturating_sub(bytes);
        m.tex_counts[i] = m.tex_counts[i].saturating_sub(1);
        m.destroyed_tex = m.destroyed_tex.saturating_add(1);
    });
}

pub(crate) fn mem_alloc_buffer(class: MemClass, bytes: u64) {
    with_mem(|m| {
        let i = class.idx();
        m.bytes[i] = m.bytes[i].saturating_add(bytes);
        m.buf_counts[i] = m.buf_counts[i].saturating_add(1);
        m.created_buf = m.created_buf.saturating_add(1);
        if m.bytes[i] > m.peak_bytes[i] {
            m.peak_bytes[i] = m.bytes[i];
        }
        m.note_peak();
    });
}

#[allow(dead_code)] // API symmetry (W84 §5.1); MemScope restores
                   // by snapshot, and the one live caller of the
                   // free half today is the bloom re-registration
                   // in composer.rs (texture side).
pub(crate) fn mem_free_buffer(class: MemClass, bytes: u64) {
    with_mem(|m| {
        let i = class.idx();
        m.bytes[i] = m.bytes[i].saturating_sub(bytes);
        m.buf_counts[i] = m.buf_counts[i].saturating_sub(1);
        m.destroyed_buf = m.destroyed_buf.saturating_add(1);
    });
}

/// Peak-side only (W84 §5.1): a readback staging buffer is
/// created and freed inside `collect`, before any end-of-run
/// print, so it must never enter `current` — it raises the
/// peak watermark by `bytes` over the current standing set and
/// records the largest single readback.
pub(crate) fn mem_note_readback(bytes: u64) {
    with_mem(|m| {
        let with = m.current().saturating_add(bytes);
        if with > m.peak {
            m.peak = with;
        }
        if bytes > m.readback_max {
            m.readback_max = bytes;
        }
        let fi = MemClass::F.idx();
        if bytes > m.peak_bytes[fi] {
            m.peak_bytes[fi] = bytes;
        }
    });
}

/// Snapshot/restore guard for per-frame transients (film
/// `render()` scope, PostChain pass scopes). On drop it restores
/// the TRANSIENT classes — E (uploads) and G (buffers) — to
/// their snapshot values; standing classes (A–D) registered
/// inside the scope survive (PostChain's first-bloom class-C
/// registration happens inside film's render scope and must
/// persist). Peak is NOT restored: it is a watermark.
pub(crate) struct MemScope {
    e_bytes: u64,
    g_bytes: u64,
    e_tex: u64,
    g_buf: u64,
}

pub(crate) fn mem_scope() -> MemScope {
    with_mem(|m| MemScope {
        e_bytes: m.bytes[MemClass::E.idx()],
        g_bytes: m.bytes[MemClass::G.idx()],
        e_tex: m.tex_counts[MemClass::E.idx()],
        g_buf: m.buf_counts[MemClass::G.idx()],
    })
}

impl Drop for MemScope {
    fn drop(&mut self) {
        with_mem(|m| {
            m.bytes[MemClass::E.idx()] = self.e_bytes;
            m.bytes[MemClass::G.idx()] = self.g_bytes;
            m.tex_counts[MemClass::E.idx()] = self.e_tex;
            m.buf_counts[MemClass::G.idx()] = self.g_buf;
        });
    }
}

/// The sysfs side of the line: AMD device-wide counters from
/// `/sys/class/drm/cardN/device/mem_info_{gtt,vram}_used`.
/// Device-wide (all processes), and on the 5650G the VRAM
/// figure is a carve-out of the same system RAM — neither is
/// this process's share (W84 §5.4). Any failure at any step
/// yields None for that field, printed as the literal
/// `unavailable` — never 0 (W84 §7.6).
#[derive(Clone, Copy)]
struct SysfsMem {
    gtt: Option<u64>,
    vram: Option<u64>,
}

fn probe_sysfs() -> SysfsMem {
    probe_sysfs_at(std::path::Path::new("/sys/class/drm"))
}

fn probe_sysfs_at(base: &std::path::Path) -> SysfsMem {
    let none = SysfsMem { gtt: None, vram: None };
    let mut cards: Vec<std::path::PathBuf> = match std::fs::read_dir(base) {
        Ok(rd) => rd
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| {
                        n.strip_prefix("card")
                            .map(|rest| !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit()))
                            .unwrap_or(false)
                    })
                    .unwrap_or(false)
            })
            .collect(),
        Err(_) => return none,
    };
    cards.sort();
    for card in cards {
        let dev = card.join("device");
        let gtt_path = dev.join("mem_info_gtt_used");
        if !gtt_path.exists() {
            continue;
        }
        let read = |p: &std::path::Path| -> Option<u64> {
            std::fs::read_to_string(p).ok()?.trim().parse().ok()
        };
        return SysfsMem { gtt: read(&gtt_path), vram: read(&dev.join("mem_info_vram_used")) };
    }
    none
}

fn fmt_sysfs(v: Option<u64>) -> String {
    match v {
        Some(n) => n.to_string(),
        None => "unavailable".to_string(),
    }
}

/// The pinned line grammar (W84 §5.3 — field order and spelling
/// are parsed by tools/perf/vram_report.py; do not change).
fn format_mem_line(m: &GpuMem, sysfs: SysfsMem) -> String {
    let textures: u64 = m.tex_counts.iter().sum();
    let buffers: u64 = m.buf_counts.iter().sum();
    format!(
        "gpu-mem: accounted_bytes={} peak_accounted_bytes={} textures={} buffers={} readback_bytes={} sysfs_gtt_used_bytes={} sysfs_vram_used_bytes={}",
        m.current(),
        m.peak,
        textures,
        buffers,
        m.readback_max,
        fmt_sysfs(sysfs.gtt),
        fmt_sysfs(sysfs.vram)
    )
}

/// The end-of-run `gpu-mem:` line for RENDER modes. Film
/// telemetry prints NO line (W84 ruling D1: telemetry is a
/// CPU-only mode — no device is ever created — so its ledger
/// would be structurally zero, a fabricated-looking
/// measurement; the verdict line remains telemetry's whole
/// contract).
pub(crate) fn mem_stats_line() -> String {
    let snap = with_mem(|m| *m);
    format_mem_line(&snap, probe_sysfs())
}

/// V2 contract C2 (WP2): the machine-readable snapshot the
/// reporting interface renders. `per_class` is (current,
/// peak) per class A..G; class F's current is always 0
/// (peak-side only, by the ledger's design) and its peak is
/// the largest single readback. `created_*` / `destroyed_*`
/// are LIFETIME call counters of the registration/free
/// functions — MemScope rollbacks restore live counts
/// without calling the free functions, so created −
/// destroyed is not the live count; `textures` / `buffers`
/// are. All figures are TRACKED allocations (the ledger's
/// honesty class), never driver numbers; sysfs is carried
/// separately and may be unavailable.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MemSnapshot {
    pub accounted_bytes: u64,
    pub peak_accounted_bytes: u64,
    pub per_class: [(u64, u64); 7],
    pub textures: u64,
    pub buffers: u64,
    pub created_textures: u64,
    pub created_buffers: u64,
    pub destroyed_textures: u64,
    pub destroyed_buffers: u64,
    pub readback_bytes: u64,
    pub sysfs_gtt_used: Option<u64>,
    pub sysfs_vram_used: Option<u64>,
}

pub(crate) fn mem_snapshot() -> MemSnapshot {
    let m = with_mem(|m| *m);
    let sysfs = probe_sysfs();
    let mut per_class = [(0u64, 0u64); 7];
    for i in 0..7 {
        per_class[i] = (m.bytes[i], m.peak_bytes[i]);
    }
    MemSnapshot {
        accounted_bytes: m.current(),
        peak_accounted_bytes: m.peak,
        per_class,
        textures: m.tex_counts.iter().sum(),
        buffers: m.buf_counts.iter().sum(),
        created_textures: m.created_tex,
        created_buffers: m.created_buf,
        destroyed_textures: m.destroyed_tex,
        destroyed_buffers: m.destroyed_buf,
        readback_bytes: m.readback_max,
        sysfs_gtt_used: sysfs.gtt,
        sysfs_vram_used: sysfs.vram,
    }
}

#[cfg(test)]
pub(crate) fn mem_class_bytes(class: MemClass) -> u64 {
    with_mem(|m| m.bytes[class.idx()])
}

#[cfg(test)]
pub(crate) fn mem_reset() {
    with_mem(|m| *m = GpuMem::new());
}

// ---------------------------------------------------------------------
// Tests — CPU arithmetic only. GPU paths are CI-verified (header).
// ---------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn padded_row_known_values() {
        // The values math.rs already pins for 4 bpp.
        assert_eq!(padded_row_bytes(1080, 4), 4352); // 4320 -> 4352
        assert_eq!(padded_row_bytes(64, 4), 256); // exact multiple: no pad
        assert_eq!(padded_row_bytes(65, 4), 512); // 260 -> 512
        // Degenerate and wide cases.
        assert_eq!(padded_row_bytes(1, 4), 256);
        assert_eq!(padded_row_bytes(1920, 4), 7680); // 7680 = 30 * 256, no pad
        // 8 bpp (fp16 packed depth, dof.rs's dump).
        assert_eq!(padded_row_bytes(1080, 8), 8704); // 8640 -> 8704
        assert_eq!(padded_row_bytes(32, 8), 256);
    }

    #[test]
    fn padded_row_agrees_with_math_module_at_4bpp() {
        // gpucore generalises math::padded_row_bytes; the two
        // spellings must never drift (both feed live readbacks
        // until migration completes).
        for w in [1u32, 2, 63, 64, 65, 100, 255, 256, 257, 1080, 1920, 2560] {
            assert_eq!(padded_row_bytes(w, 4), crate::math::padded_row_bytes(w), "width {w}");
        }
    }

    /// A synthetic mapped buffer: each row is `row` payload bytes
    /// followed by 0xEE sentinels. The payload pattern stays in
    /// 1..=0xED so it can never alias the sentinel (the original
    /// y*16+x pattern reaches 0xEE itself at x=238 — the lead's
    /// fix when this test first ran in CI, run 43619241590).
    fn payload(y: usize, x: usize) -> u8 {
        (1 + (y * 16 + x) % 0xED) as u8
    }

    fn fake_mapped(width: u32, height: u32, bpp: u32) -> (Vec<u8>, u32) {
        let padded = padded_row_bytes(width, bpp);
        let row = (width * bpp) as usize;
        let mut buf = vec![0xEEu8; (padded * height) as usize];
        for y in 0..height as usize {
            for x in 0..row {
                buf[y * padded as usize + x] = payload(y, x);
            }
        }
        (buf, padded)
    }

    #[test]
    fn strip_padding_drops_sentinels() {
        let (buf, padded) = fake_mapped(65, 3, 4); // row 260, padded 512
        let out = strip_padding(&buf, 65, 3, 4, padded);
        assert_eq!(out.len(), 65 * 4 * 3);
        assert!(!out.contains(&0xEE), "padding sentinel leaked into output");
        for y in 0..3usize {
            for x in 0..260usize {
                assert_eq!(out[y * 260 + x], payload(y, x), "row {y} byte {x}");
            }
        }
    }

    #[test]
    fn strip_padding_no_pad_needed() {
        // 64 px * 4 bpp = 256 = exactly one alignment unit.
        let (buf, padded) = fake_mapped(64, 2, 4);
        assert_eq!(padded, 256);
        let out = strip_padding(&buf, 64, 2, 4, padded);
        assert_eq!(out.len(), buf.len());
        assert_eq!(out, buf[..out.len()]);
    }

    #[test]
    fn strip_padding_8bpp_and_single_row() {
        let (buf, padded) = fake_mapped(10, 1, 8); // row 80, padded 256
        let out = strip_padding(&buf, 10, 1, 8, padded);
        assert_eq!(out.len(), 80);
        assert_eq!(out, buf[..80]);
    }

    // -----------------------------------------------------------------
    // GPU memory ledger (VRAM-IMPL-W84). The ledger is a
    // process-global, so every test that touches it holds this
    // lock for its whole body and resets first — cargo runs
    // tests on parallel threads.
    // -----------------------------------------------------------------

    static LEDGER_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn tex_bytes_known_values() {
        use wgpu::TextureFormat as F;
        // The film lane's arithmetic (W84 §3.1).
        assert_eq!(tex_bytes(1080, 1920, F::Rgba16Float, 4), 66_355_200); // hdr msaa
        assert_eq!(tex_bytes(1080, 1920, F::Depth32Float, 4), 33_177_600); // hdr depth
        assert_eq!(tex_bytes(1080, 1920, F::Rgba16Float, 1), 16_588_800);
        assert_eq!(tex_bytes(1080, 1920, F::Rgba8Unorm, 1), 8_294_400);
        assert_eq!(tex_bytes(1080, 1920, F::Rgba8UnormSrgb, 1), 8_294_400);
        assert_eq!(tex_bytes(2048, 2048, F::Rgba8Unorm, 1), 16_777_216); // one shadow term
        assert_eq!(tex_bytes(768, 1024, F::Rgba16Float, 1), 6_291_456); // PMREM
    }

    #[test]
    fn mem_snapshot_reports_classes_counters_and_peaks() {
        let _g = LEDGER_TEST_LOCK.lock().unwrap();
        mem_reset();
        mem_alloc_texture(MemClass::A, 1000);
        mem_alloc_texture(MemClass::A, 500);
        mem_free_texture(MemClass::A, 500);
        mem_alloc_buffer(MemClass::G, 64);
        mem_note_readback(300);
        let s = mem_snapshot();
        assert_eq!(s.accounted_bytes, 1064);
        assert_eq!(s.per_class[MemClass::A.idx()], (1000, 1500));
        assert_eq!(s.per_class[MemClass::G.idx()], (64, 64));
        assert_eq!(s.per_class[MemClass::F.idx()], (0, 300));
        assert_eq!(s.textures, 1);
        assert_eq!(s.buffers, 1);
        assert_eq!(s.created_textures, 2);
        assert_eq!(s.destroyed_textures, 1);
        assert_eq!(s.created_buffers, 1);
        assert_eq!(s.destroyed_buffers, 0);
        assert_eq!(s.readback_bytes, 300);
        mem_reset();
    }

    #[test]
    fn ledger_alloc_free_scope_and_peak_semantics() {
        let _g = LEDGER_TEST_LOCK.lock().unwrap();
        mem_reset();
        mem_alloc_texture(MemClass::A, 1000);
        mem_alloc_texture(MemClass::E, 200);
        mem_alloc_buffer(MemClass::G, 50);
        assert_eq!(mem_class_bytes(MemClass::A), 1000);
        // Peak-side readback note: raises peak, never current.
        mem_note_readback(300);
        let snap = with_mem(|m| *m);
        assert_eq!(snap.current(), 1250);
        assert_eq!(snap.peak, 1550);
        assert_eq!(snap.readback_max, 300);
        // A smaller later note changes nothing.
        mem_note_readback(100);
        let snap = with_mem(|m| *m);
        assert_eq!(snap.peak, 1550);
        assert_eq!(snap.readback_max, 300);
        // Scope: E/G transients roll back on drop; A survives;
        // the peak watermark does not roll back.
        {
            let _scope = mem_scope();
            mem_alloc_texture(MemClass::E, 700);
            mem_alloc_buffer(MemClass::G, 900);
            mem_alloc_texture(MemClass::C, 400); // standing class inside a scope
            let inside = with_mem(|m| *m);
            assert_eq!(inside.current(), 1250 + 700 + 900 + 400);
            assert_eq!(inside.peak, 1250 + 700 + 900 + 400);
        }
        let after = with_mem(|m| *m);
        assert_eq!(after.current(), 1250 + 400);
        assert_eq!(after.peak, 1250 + 700 + 900 + 400);
        assert_eq!(mem_class_bytes(MemClass::C), 400);
        // Free returns bytes and counts.
        mem_free_texture(MemClass::C, 400);
        assert_eq!(mem_class_bytes(MemClass::C), 0);
        let after = with_mem(|m| *m);
        assert_eq!(after.current(), 1250);
        let textures: u64 = after.tex_counts.iter().sum();
        assert_eq!(textures, 2); // one A + one E
    }

    #[test]
    fn mem_line_format_is_the_pinned_grammar() {
        // Hand-built state: no global touched, no lock needed.
        let mut m = GpuMem::new();
        m.bytes[MemClass::A.idx()] = 220_930_688;
        m.peak = 229_286_528;
        m.readback_max = 8_355_840;
        m.tex_counts[MemClass::A.idx()] = 41;
        m.buf_counts[MemClass::G.idx()] = 18;
        let line = format_mem_line(&m, SysfsMem { gtt: None, vram: None });
        assert_eq!(
            line,
            "gpu-mem: accounted_bytes=220930688 peak_accounted_bytes=229286528 textures=41 buffers=18 readback_bytes=8355840 sysfs_gtt_used_bytes=unavailable sysfs_vram_used_bytes=unavailable"
        );
        let line = format_mem_line(&m, SysfsMem { gtt: Some(123), vram: Some(456) });
        assert!(line.ends_with("sysfs_gtt_used_bytes=123 sysfs_vram_used_bytes=456"), "got: {line}");
    }

    #[test]
    fn sysfs_probe_unavailable_and_numeric_paths() {
        // Empty dir -> no card -> both unavailable (the VM/CI
        // path; W84 §5.4 verified this VM has no cardN at all).
        let base = std::env::temp_dir().join(format!("talerender-sysfs-test-{}-empty", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).unwrap();
        let s = probe_sysfs_at(&base);
        assert_eq!(s.gtt, None);
        assert_eq!(s.vram, None);
        // A fake card0 with both counters -> numeric path.
        let dev = base.join("card0").join("device");
        std::fs::create_dir_all(&dev).unwrap();
        std::fs::write(dev.join("mem_info_gtt_used"), "12345\n").unwrap();
        std::fs::write(dev.join("mem_info_vram_used"), "678\n").unwrap();
        let s = probe_sysfs_at(&base);
        assert_eq!(s.gtt, Some(12345));
        assert_eq!(s.vram, Some(678));
        // Malformed counter -> that field unavailable, the
        // other still read.
        std::fs::write(dev.join("mem_info_vram_used"), "plenty\n").unwrap();
        let s = probe_sysfs_at(&base);
        assert_eq!(s.gtt, Some(12345));
        assert_eq!(s.vram, None);
        let _ = std::fs::remove_dir_all(&base);
    }

    /// The exact gate (W84 §6.2): replay the film lane's
    /// standing registrations at (1080, 1920, s = 4, S = 2048)
    /// through the ledger API and assert the acceptance
    /// figures — re-derived from the code's own size
    /// expressions in W84 §3, not copied from a log.
    #[test]
    fn ledger_replay_film_lane_acceptance_figures() {
        let _g = LEDGER_TEST_LOCK.lock().unwrap();
        mem_reset();
        use wgpu::TextureFormat as F;
        let (w, h, s, sh) = (1080u32, 1920u32, 4u32, 2048u32);
        // Class A (default packed depth, p = 4: Rgba8Unorm).
        mem_alloc_texture(MemClass::A, tex_bytes(w, h, F::Rgba16Float, s)); // hdr msaa
        mem_alloc_texture(MemClass::A, tex_bytes(w, h, F::Depth32Float, s)); // hdr depth
        mem_alloc_texture(MemClass::A, tex_bytes(w, h, F::Rgba16Float, 1)); // hdr resolved
        mem_alloc_texture(MemClass::A, tex_bytes(w, h, F::Rgba8Unorm, 1)); // packed depth
        mem_alloc_texture(MemClass::A, tex_bytes(w, h, F::Depth32Float, 1)); // prepass
        mem_alloc_texture(MemClass::A, tex_bytes(w, h, F::Rgba16Float, 1)); // bokeh out
        mem_alloc_texture(MemClass::A, tex_bytes(w, h, F::Rgba8Unorm, 1)); // output srgb
        mem_alloc_texture(MemClass::A, tex_bytes(w, h, F::Rgba8Unorm, 1)); // final
        // Class B: the shadow pair.
        mem_alloc_texture(MemClass::B, tex_bytes(sh, sh, F::Rgba8Unorm, 1));
        mem_alloc_texture(MemClass::B, tex_bytes(sh, sh, F::Depth32Float, 1));
        // Class C: one bloom chain (D2 retained registration).
        mem_alloc_texture(MemClass::C, crate::composer::bloom_chain_bytes(w, h));
        // Class D: PMREM.
        mem_alloc_texture(MemClass::D, tex_bytes(768, 1024, F::Rgba16Float, 1));
        assert_eq!(mem_class_bytes(MemClass::A), 165_888_000);
        assert_eq!(mem_class_bytes(MemClass::B), 33_554_432);
        assert_eq!(mem_class_bytes(MemClass::C), 15_196_800);
        assert_eq!(mem_class_bytes(MemClass::D), 6_291_456);
        let snap = with_mem(|m| *m);
        assert_eq!(snap.current(), 220_930_688, "steady total (W84 §3.1)");
        assert_eq!(snap.peak, 220_930_688);
        // One frame readback: padded row 4352 x 1920.
        mem_note_readback(8_355_840);
        let snap = with_mem(|m| *m);
        assert_eq!(snap.peak, 229_286_528, "peak = steady + one readback (W84 §3.1)");
        assert_eq!(snap.readback_max, 8_355_840);
        // Halffloat packed-depth variant (p = 8): A and steady
        // grow by exactly the packed-depth delta (W84 §3.3).
        mem_reset();
        mem_alloc_texture(MemClass::A, tex_bytes(w, h, F::Rgba16Float, s));
        mem_alloc_texture(MemClass::A, tex_bytes(w, h, F::Depth32Float, s));
        mem_alloc_texture(MemClass::A, tex_bytes(w, h, F::Rgba16Float, 1));
        mem_alloc_texture(MemClass::A, tex_bytes(w, h, F::Rgba16Float, 1)); // packed, halffloat
        mem_alloc_texture(MemClass::A, tex_bytes(w, h, F::Depth32Float, 1));
        mem_alloc_texture(MemClass::A, tex_bytes(w, h, F::Rgba16Float, 1));
        mem_alloc_texture(MemClass::A, tex_bytes(w, h, F::Rgba8Unorm, 1));
        mem_alloc_texture(MemClass::A, tex_bytes(w, h, F::Rgba8Unorm, 1));
        mem_alloc_texture(MemClass::B, tex_bytes(sh, sh, F::Rgba8Unorm, 1));
        mem_alloc_texture(MemClass::B, tex_bytes(sh, sh, F::Depth32Float, 1));
        mem_alloc_texture(MemClass::C, crate::composer::bloom_chain_bytes(w, h));
        mem_alloc_texture(MemClass::D, tex_bytes(768, 1024, F::Rgba16Float, 1));
        assert_eq!(mem_class_bytes(MemClass::A), 174_182_400);
        let snap = with_mem(|m| *m);
        assert_eq!(snap.current(), 229_225_088, "halffloat steady (W84 §3.3)");
    }
}
