//! Structured runtime errors — audit finding D (wave-2 seat W05).
//!
//! Runtime paths (GPU setup, buffer readback, image save, scene/film
//! loading) historically failed with `expect()` / `panic!()`. This
//! module is the error type those call sites migrate to: every
//! failure carries *what operation failed* (context), *where* (the
//! file, for I/O variants), and the *underlying cause* (source text,
//! preserved verbatim — the loaders below this layer already report
//! `String` causes, and wgpu's error types are `Display`, so text is
//! the common denominator; no information is summarised away).
//!
//! Design notes for whoever wires this in:
//!
//! - `TaleResult<T>` is the migration target signature:
//!   `Result<T, String>` loaders become `TaleResult<T>` loaders.
//! - Constructors take `impl Display` for the source, so existing
//!   `String` errors, `std::io::Error`, and wgpu error types all
//!   slot in without conversion boilerplate at the call site.
//! - There is deliberately **no** blanket `From<std::io::Error>`:
//!   an io failure reading a scene file is `DataLoad` (exit 2,
//!   invalid input) while one writing a frame is `ImageIo`
//!   (exit 4, output failure). A blanket conversion would silently
//!   pick one and misclassify the exit code. Call sites say which:
//!   `TaleError::data_load(path, e)` / `TaleError::image_io(path, e)`.
//!   `From<image::ImageError>` *is* provided — image encode/decode
//!   failures are unambiguously `ImageIo`.
//! - Exit codes follow the audit's table (see `exit_code`): 0 ok,
//!   1 QA fail, 2 CLI/invalid input, 3 GPU/render failure,
//!   4 output/filesystem failure. No variant maps to 1: that code
//!   is the film-telemetry gate's *verdict*, not an error — the
//!   gate prints `FILM TELEMETRY: CHECK OK / CHECK FAILED` (the
//!   line stays authoritative) and, since EXIT-CODES-W02, exits
//!   0 / 1 in agreement with it. Do not route the telemetry
//!   verdict through this type.
//!
//! Wiring: declare `mod errors;` in `src/main.rs` (see
//! docs/integration/ERRORS-W05.md for the exact site and the
//! call-site migration inventory).

use std::fmt;
use std::path::PathBuf;

/// Result alias for the migration: `TaleResult<T>` replaces the
/// `Result<T, String>` signatures the loaders use today.
pub type TaleResult<T> = Result<T, TaleError>;

/// A runtime failure with context. Variants are grouped by the
/// audit's exit-code table; see `exit_code()`.
#[derive(Debug)]
pub enum TaleError {
    /// No usable Vulkan adapter / wgpu instance setup failed
    /// (e.g. `request_adapter` found nothing). Exit 3.
    GpuInit { context: String, source: String },
    /// The adapter was found but `request_device` failed. Exit 3.
    DeviceRequest { context: String, source: String },
    /// A GPU buffer could not be mapped for readback (the map
    /// channel closed, or the map itself errored). Exit 3.
    BufferMap { context: String, source: String },
    /// An image could not be encoded, decoded, or written —
    /// PNG frame saves, caption/asset image loads via the `image`
    /// crate, output-directory creation. Exit 4.
    ImageIo {
        path: Option<PathBuf>,
        source: String,
    },
    /// Scene/film data could not be read or parsed — scene JSON,
    /// film.json + manifests, caption/asset files, authored data
    /// values (e.g. a malformed hex colour). Exit 2 (invalid input).
    DataLoad {
        path: Option<PathBuf>,
        source: String,
    },
    /// A render pass failed after setup succeeded — pipeline,
    /// submission, or scene evaluation errors that are neither
    /// device loss nor buffer mapping. Exit 3.
    Render { context: String, source: String },
}

impl TaleError {
    /// No usable GPU adapter. `context` names the request site
    /// (e.g. `"request_adapter (scene path)"`).
    pub fn gpu_init(context: impl Into<String>, source: impl fmt::Display) -> Self {
        TaleError::GpuInit {
            context: context.into(),
            source: source.to_string(),
        }
    }

    /// `request_device` failed at the named site.
    pub fn device_request(context: impl Into<String>, source: impl fmt::Display) -> Self {
        TaleError::DeviceRequest {
            context: context.into(),
            source: source.to_string(),
        }
    }

    /// Buffer map/readback failed at the named site.
    pub fn buffer_map(context: impl Into<String>, source: impl fmt::Display) -> Self {
        TaleError::BufferMap {
            context: context.into(),
            source: source.to_string(),
        }
    }

    /// A render-time failure at the named site.
    pub fn render(context: impl Into<String>, source: impl fmt::Display) -> Self {
        TaleError::Render {
            context: context.into(),
            source: source.to_string(),
        }
    }

    /// Image output/IO failed for `path` (frame save, asset write,
    /// output-directory creation).
    pub fn image_io(path: impl Into<PathBuf>, source: impl fmt::Display) -> Self {
        TaleError::ImageIo {
            path: Some(path.into()),
            source: source.to_string(),
        }
    }

    /// Image failure with no single path attached (e.g. an
    /// in-memory decode via the `image` crate).
    pub fn image_io_msg(source: impl fmt::Display) -> Self {
        TaleError::ImageIo {
            path: None,
            source: source.to_string(),
        }
    }

    /// Scene/film data at `path` could not be read or parsed.
    pub fn data_load(path: impl Into<PathBuf>, source: impl fmt::Display) -> Self {
        TaleError::DataLoad {
            path: Some(path.into()),
            source: source.to_string(),
        }
    }

    /// Data failure with no single path attached (e.g. a malformed
    /// authored value inside otherwise-loaded data).
    pub fn data_load_msg(source: impl fmt::Display) -> Self {
        TaleError::DataLoad {
            path: None,
            source: source.to_string(),
        }
    }

    /// The preserved underlying-cause text, verbatim.
    pub fn source_message(&self) -> &str {
        match self {
            TaleError::GpuInit { source, .. }
            | TaleError::DeviceRequest { source, .. }
            | TaleError::BufferMap { source, .. }
            | TaleError::ImageIo { source, .. }
            | TaleError::DataLoad { source, .. }
            | TaleError::Render { source, .. } => source,
        }
    }

    /// Short category name for log lines (`gpu-init`, `image-io`, …).
    pub fn category(&self) -> &'static str {
        match self {
            TaleError::GpuInit { .. } => "gpu-init",
            TaleError::DeviceRequest { .. } => "device-request",
            TaleError::BufferMap { .. } => "buffer-map",
            TaleError::ImageIo { .. } => "image-io",
            TaleError::DataLoad { .. } => "data-load",
            TaleError::Render { .. } => "render",
        }
    }

    /// Process exit code per the audit's table:
    ///
    /// | code | meaning                       | mapped from |
    /// |------|-------------------------------|-------------|
    /// | 0    | success                       | (no error)  |
    /// | 1    | QA validation failed          | telemetry gate verdict only — never a `TaleError` |
    /// | 2    | invalid arguments or input    | `DataLoad` (CLI parse errors exit 2 via main.rs `cli_error`) |
    /// | 3    | GPU / rendering failure       | `GpuInit`, `DeviceRequest`, `BufferMap`, `Render` |
    /// | 4    | output / filesystem failure   | `ImageIo` |
    pub fn exit_code(&self) -> i32 {
        match self {
            TaleError::DataLoad { .. } => 2,
            TaleError::GpuInit { .. }
            | TaleError::DeviceRequest { .. }
            | TaleError::BufferMap { .. }
            | TaleError::Render { .. } => 3,
            TaleError::ImageIo { .. } => 4,
        }
    }
}

impl fmt::Display for TaleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TaleError::GpuInit { context, source } => {
                write!(f, "GPU initialisation failed ({context}): {source}")
            }
            TaleError::DeviceRequest { context, source } => {
                write!(f, "GPU device request failed ({context}): {source}")
            }
            TaleError::BufferMap { context, source } => {
                write!(f, "GPU buffer mapping failed ({context}): {source}")
            }
            TaleError::Render { context, source } => {
                write!(f, "render failed ({context}): {source}")
            }
            TaleError::ImageIo { path, source } => match path {
                Some(p) => write!(f, "image output failed for {}: {source}", p.display()),
                None => write!(f, "image operation failed: {source}"),
            },
            TaleError::DataLoad { path, source } => match path {
                Some(p) => write!(f, "failed to load {}: {source}", p.display()),
                None => write!(f, "data load failed: {source}"),
            },
        }
    }
}

impl std::error::Error for TaleError {
    // `source()` returns None: the underlying cause is preserved
    // verbatim as text in the variant (the codebase's lower layers
    // report `String` causes), and is shown by Display. See the
    // module header.
}

impl From<image::ImageError> for TaleError {
    /// Image encode/decode failures are unambiguously output-side:
    /// they become `ImageIo` with no path attached. Call sites that
    /// know the path should prefer `TaleError::image_io(path, e)`.
    fn from(e: image::ImageError) -> Self {
        TaleError::ImageIo {
            path: None,
            source: e.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn display_gpu_init() {
        let e = TaleError::gpu_init(
            "request_adapter (scene path)",
            "no wgpu adapter found — a Vulkan driver is required",
        );
        assert_eq!(
            e.to_string(),
            "GPU initialisation failed (request_adapter (scene path)): \
             no wgpu adapter found — a Vulkan driver is required"
        );
        assert_eq!(e.category(), "gpu-init");
        assert_eq!(
            e.source_message(),
            "no wgpu adapter found — a Vulkan driver is required"
        );
    }

    #[test]
    fn display_device_request() {
        let e = TaleError::device_request("make_device (scene-file)", "device lost");
        assert_eq!(
            e.to_string(),
            "GPU device request failed (make_device (scene-file)): device lost"
        );
        assert_eq!(e.category(), "device-request");
    }

    #[test]
    fn display_buffer_map() {
        let e = TaleError::buffer_map("readback (film frame 1200)", "map channel closed");
        assert_eq!(
            e.to_string(),
            "GPU buffer mapping failed (readback (film frame 1200)): map channel closed"
        );
        assert_eq!(e.category(), "buffer-map");
    }

    #[test]
    fn display_render() {
        let e = TaleError::render("composer pass (ab1)", "pipeline compilation failed");
        assert_eq!(
            e.to_string(),
            "render failed (composer pass (ab1)): pipeline compilation failed"
        );
        assert_eq!(e.category(), "render");
    }

    #[test]
    fn display_image_io_with_and_without_path() {
        let e = TaleError::image_io("frames/frame_01200.png", "permission denied");
        assert_eq!(
            e.to_string(),
            "image output failed for frames/frame_01200.png: permission denied"
        );
        assert_eq!(e.category(), "image-io");

        let e = TaleError::image_io_msg("unsupported format");
        assert_eq!(e.to_string(), "image operation failed: unsupported format");
    }

    #[test]
    fn display_data_load_with_and_without_path() {
        let e = TaleError::data_load(Path::new("scenes/film/film.json"), "unexpected end of file");
        assert_eq!(
            e.to_string(),
            "failed to load scenes/film/film.json: unexpected end of file"
        );
        assert_eq!(e.category(), "data-load");

        let e = TaleError::data_load_msg("bad hex colour '#ZZZ'");
        assert_eq!(e.to_string(), "data load failed: bad hex colour '#ZZZ'");
    }

    #[test]
    fn exit_code_mapping_for_every_variant() {
        // The audit's table: 2 = invalid input, 3 = GPU/render
        // failure, 4 = output/filesystem failure. (0 = success and
        // 1 = telemetry QA verdict are not TaleError territory.)
        assert_eq!(TaleError::data_load_msg("x").exit_code(), 2);
        assert_eq!(TaleError::data_load("p", "x").exit_code(), 2);
        assert_eq!(TaleError::gpu_init("c", "x").exit_code(), 3);
        assert_eq!(TaleError::device_request("c", "x").exit_code(), 3);
        assert_eq!(TaleError::buffer_map("c", "x").exit_code(), 3);
        assert_eq!(TaleError::render("c", "x").exit_code(), 3);
        assert_eq!(TaleError::image_io("p", "x").exit_code(), 4);
        assert_eq!(TaleError::image_io_msg("x").exit_code(), 4);
    }

    #[test]
    fn from_image_error_is_image_io() {
        let io = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied by fs");
        let img: image::ImageError = io.into();
        let e = TaleError::from(img);
        assert!(matches!(e, TaleError::ImageIo { path: None, .. }));
        assert_eq!(e.exit_code(), 4);
        assert!(e.to_string().contains("denied by fs"), "got: {e}");
    }

    #[test]
    fn constructors_accept_string_and_io_sources() {
        // Existing loaders report String causes; io errors must
        // slot into the file variants without manual to_string().
        let e = TaleError::data_load("s.json", String::from("bad version"));
        assert_eq!(e.source_message(), "bad version");
        let io = std::io::Error::new(std::io::ErrorKind::NotFound, "no such file");
        let e = TaleError::data_load("s.json", io);
        assert!(e.to_string().contains("no such file"), "got: {e}");
        let io = std::io::Error::new(std::io::ErrorKind::Other, "disk full");
        let e = TaleError::image_io("f.png", io);
        assert!(e.to_string().contains("disk full"), "got: {e}");
    }

    #[test]
    fn tale_result_alias_round_trips() {
        fn ok() -> TaleResult<u32> {
            Ok(7)
        }
        fn err() -> TaleResult<u32> {
            Err(TaleError::render("pass", "boom"))
        }
        assert_eq!(ok().unwrap(), 7);
        assert_eq!(err().unwrap_err().exit_code(), 3);
    }
}
