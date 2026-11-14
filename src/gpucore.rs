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
    wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::VULKAN,
        ..Default::default()
    })
}

/// Canonical adapter request — the verbatim semantics of the
/// helper formerly defined in main.rs and already called by every
/// site: try a high-performance adapter first; if none answers,
/// retry with the fallback (software) adapter forced. Panics with
/// the project's established message when no adapter exists at
/// all — that message is load-bearing (PERF.md's failure table
/// quotes it verbatim for the "No adapter" row).
pub(crate) async fn request_adapter(instance: &wgpu::Instance) -> wgpu::Adapter {
    if let Some(adapter) = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
        })
        .await
    {
        return adapter;
    }
    instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: None,
            force_fallback_adapter: true,
        })
        .await
        .expect("no wgpu adapter found — a Vulkan driver (hardware or lavapipe) is required")
}

/// Device + queue with the project's error message preserved.
/// `limits` is explicit because the sites legitimately differ
/// today (standard everywhere except main.rs::run's downlevel);
/// new code should use `request_device_default`.
pub(crate) async fn request_device(
    adapter: &wgpu::Adapter,
    label: &'static str,
    limits: wgpu::Limits,
) -> (wgpu::Device, wgpu::Queue) {
    adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: Some(label),
                required_features: wgpu::Features::empty(),
                required_limits: limits,
                ..Default::default()
            },
            None,
        )
        .await
        .expect("request_device failed")
}

/// The common case: standard limits (NOT downlevel — those cap
/// textures at 2048 and the character canvas is 2560×2816).
pub(crate) async fn request_device_default(
    adapter: &wgpu::Adapter,
    label: &'static str,
) -> (wgpu::Device, wgpu::Queue) {
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
        encoder.copy_texture_to_buffer(
            wgpu::ImageCopyTexture {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::ImageCopyBuffer {
                buffer: &buffer,
                layout: wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_row),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
        );
        Readback { buffer, width, height, bytes_per_pixel, padded_row }
    }

    /// Map the buffer (after the caller submitted the encoder),
    /// wait, and return the tight rows with padding stripped.
    /// Map-failure messages are the majority spelling used by
    /// main.rs / ab1.rs / film.rs / scenefile.rs.
    pub(crate) fn collect(self, device: &wgpu::Device) -> Vec<u8> {
        let (width, height, bpp, padded_row) =
            (self.width, self.height, self.bytes_per_pixel, self.padded_row);
        let (padded, _) = self.collect_padded(device);
        strip_padding(&padded, width, height, bpp, padded_row)
    }

    /// Like `collect`, but returns the raw padded rows plus the
    /// row stride — for callers that index the mapped layout in
    /// place (dof.rs's packed-depth panel sampler).
    pub(crate) fn collect_padded(self, device: &wgpu::Device) -> (Vec<u8>, u32) {
        let slice = self.buffer.slice(..);
        let (tx, rx) = mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = tx.send(result);
        });
        device.poll(wgpu::Maintain::Wait);
        rx.recv().expect("map channel closed").expect("buffer map failed");
        let data = slice.get_mapped_range();
        let out = data.to_vec();
        drop(data);
        self.buffer.unmap();
        (out, self.padded_row)
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
) -> Vec<u8> {
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
}
