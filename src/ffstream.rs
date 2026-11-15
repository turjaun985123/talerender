//! =====================================================================
//! DELIVERY NOTE — TALERENDER V2, Batch 1, Seat 4 (WP4: FFmpeg process
//! integration). Written against docs/integration/V2-CONTRACTS.md §3
//! contract C3 (sink half), §1 ownership law, and master prompt §8.
//!
//! FILES CREATED
//! - src/ffstream.rs — this file only. No existing file was edited,
//!   no git commands were run, no dependencies were added (std only:
//!   std::process / std::io / std::thread / std::env / std::path).
//!
//! CONTRACT POINTS IMPLEMENTED (C3, sink half)
//! - `ffmpeg_available() -> bool`: PATH lookup WITHOUT spawning a
//!   process or a shell — splits $PATH and probes for an executable
//!   file named `ffmpeg` (`ffmpeg.exe` also accepted, for Windows).
//! - `build_args(width, height, fps, crf, preset, out_part)
//!   -> Vec<String>`: PURE argv builder. Input is rawvideo `rgba`
//!   on stdin (`-f rawvideo -pix_fmt rgba -s WxH -r FPS -i -`);
//!   encode is libx264 with `-crf` / `-preset`, `-pix_fmt yuv420p`
//!   output, written to the caller-supplied `.part` path. `-y`
//!   leads so a stale `.part` from an interrupted run is replaced,
//!   mirroring film/finish.sh. Unit-tested byte-for-byte (below).
//! - `FfmpegSink::spawn(out_part, w, h, fps)` uses the production
//!   defaults CRF 18 / preset "slow" (production law — the video
//!   settings of film/finish.sh: `libx264 -preset slow -crf 18
//!   -pix_fmt yuv420p`). `FfmpegSink::with_settings(.., crf, preset)`
//!   is the explicit variant (e.g. a draft's 23 / "veryfast").
//! - `write_frame(&mut self, rgba)`: validates
//!   `rgba.len() == w*h*4` BEFORE writing (pure check, factored into
//!   `validate_frame_len` so tests exercise it without a process),
//!   writes all bytes to the child's stdin, counts the frame only
//!   after the write succeeds.
//! - `finish(self, expected_frames) -> TaleResult<FinishReport>`:
//!   drops stdin (EOF = ffmpeg's flush/finalize signal), waits for
//!   exit, joins the stderr-drain thread. Non-zero exit OR
//!   `frames_written != expected_frames` is a `TaleError::render`
//!   (category "render", exit code 3) with the stderr tail attached.
//!   On success returns `FinishReport { frames_written,
//!   stderr_tail }`.
//! - stderr is piped and drained on a dedicated thread from spawn
//!   to finish, so a chatty ffmpeg can never deadlock the render
//!   loop on a full pipe. stdout is null (nothing consumes it).
//! - Drop WITHOUT finish() kills the child and reaps it
//!   (`kill()` + `wait()`): an abandoned sink never leaves an
//!   orphan ffmpeg holding the `.part` file.
//!
//! WIRING THE LEAD MUST DO
//! - mod line: add `mod ffstream;` to the mod block in src/main.rs
//!   (after `mod film;`, ~line 36 at 65e4ea2). No other existing
//!   file needs a change for this module to compile or test.
//! - Call sites (stream mode; sibling seat's streamq.rs supplies
//!   the bounded queue half of C3):
//!   1. Before the frame loop, construct the sink with the `.part`
//!      path, following finish.sh's PART="${OUT}.part.mp4"
//!      discipline, e.g.:
//!        let mut sink = ffstream::FfmpegSink::spawn(
//!            &format!("{out_mp4}.part.mp4"), width, height, fps)?;
//!      Guard construction with `ffstream::ffmpeg_available()`
//!      when implementing C3's Auto mode (Batch 2 streammodes.rs):
//!      Auto = Stream when this returns true, else Png, printed.
//!   2. Per frame: in film::run_film (src/film.rs, the loop at
//!      ~line 3634 where `px` is currently saved with
//!      `img.save(&path)`), Stream mode calls
//!      `sink.write_frame(&px)?` with the same RGBA buffer that
//!      would have become the PNG. If draining a
//!      `streamq::FramePacket` instead, pass `&packet.rgba` (the
//!      packet's width/height must equal the sink's; write_frame's
//!      length check enforces the byte consequence).
//!   3. After the loop: `let report = sink.finish(expected)?;`
//!      where `expected` is the number of frames actually fed to
//!      THIS sink — if the PNG resume-skip path is kept in stream
//!      mode, that is the rendered count, not `to - from`.
//!      Print `report.frames_written` in the completion line.
//!   4. Only after finish() returns Ok does the caller run the
//!      existing finishing step (rename `.part` -> final + mux).
//!
//! TESTS INCLUDED (#[cfg(test)], NONE execute ffmpeg)
//! - build_args: exact argv for the production shape
//!   (1080x1920@30, CRF 18, slow) and for a parameterised draft
//!   shape (1280x720@24, CRF 23, veryfast) — full-vector equality.
//! - Defaults pinned: DEFAULT_CRF == 18, DEFAULT_PRESET == "slow".
//! - expected_frame_bytes math incl. the 1080x1920 frame size.
//! - validate_frame_len: exact length Ok; short/long rejected as
//!   TaleError category "render" / exit code 3 with both byte
//!   counts in the message.
//! - Availability: ffmpeg_in_path(None / "") is false; an empty
//!   dir list is false; a temp dir holding a fake executable file
//!   named `ffmpeg` IS found (file is created, never run), and on
//!   unix the same file without an execute bit is NOT found;
//!   ffmpeg_available() itself is only asserted not to panic (its
//!   value depends on the host).
//! - with_settings rejects zero width/height/fps BEFORE any spawn
//!   attempt, so those tests run no process either.
//! - stderr_tail: short output kept verbatim (trailing newline
//!   trimmed); long output truncated to the last 4096 chars with a
//!   "..." prefix.
//!
//! KNOWN LIMITATIONS
//! - VIDEO-ONLY `.part`: this sink encodes picture only. A/V mux
//!   with the project's data/mix.wav, the BT.709 colour tags /
//!   x264 VUI params, AAC encode, `+faststart` and the final
//!   rename ALL stay in the existing finishing step (the
//!   film/finish.sh contract) — the sink's argv deliberately
//!   matches the seat spec (rawvideo in, libx264 CRF/preset,
//!   yuv420p out) and does not duplicate finishing flags.
//!   Note for integration: finish.sh currently consumes a PNG
//!   directory, so wiring the streamed path means the finishing
//!   invocation for a streamed `.part` (stream-copy mux) is the
//!   lead's adaptation; the sink never muxes and never renames.
//! - Frame accounting counts frames successfully WRITTEN to
//!   ffmpeg's stdin vs the caller's expected count. The encoded
//!   file's frame count is not independently probed (no ffprobe
//!   here); with constant-rate rawvideo input ffmpeg neither
//!   drops nor duplicates frames, and a short write surfaces as
//!   a broken-pipe error at write_frame/finish time.
//! - stderr is accumulated in memory on the drain thread. With
//!   the default loglevel and no -stats flag this is the banner
//!   plus progress/summary lines (small); only the last 4096
//!   chars are retained in FinishReport / error messages.
//! - Drop-kill uses Child::kill (SIGKILL on unix): the `.part`
//!   file is left on disk for the caller to remove — the sink
//!   never deletes output files.
//! - ffmpeg is resolved by bare name through the child's own
//!   PATH lookup at spawn; ffmpeg_available() is a pre-flight
//!   probe for mode selection/reporting, not a spawn guarantee
//!   (spawn failure is still mapped to TaleError::render).
//! =====================================================================
//!
//! FFmpeg streaming sink (WP4): pipes raw RGBA frames into an
//! ffmpeg child process that encodes them to a video-only `.part`
//! MP4 with the production x264 settings. See the DELIVERY NOTE
//! above for the contract, wiring and limitations.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::thread::JoinHandle;

use crate::errors::{TaleError, TaleResult};

/// Production CRF (production law; film/finish.sh master encode).
pub(crate) const DEFAULT_CRF: u32 = 18;

/// Production x264 preset (film/finish.sh master encode).
pub(crate) const DEFAULT_PRESET: &str = "slow";

/// How many stderr characters `FinishReport` / error messages keep.
const STDERR_TAIL_MAX_CHARS: usize = 4096;

/// True when an executable `ffmpeg` is on PATH. Pure filesystem
/// probe: splits $PATH and stat-checks each entry — no process is
/// spawned and no shell is involved. Used by callers to select /
/// report the output mode (C3: Auto = Stream iff this is true).
pub(crate) fn ffmpeg_available() -> bool {
    ffmpeg_in_path(std::env::var_os("PATH").as_deref())
}

/// `ffmpeg_available` over an explicit PATH value (test seam).
fn ffmpeg_in_path(path_var: Option<&std::ffi::OsStr>) -> bool {
    let Some(path_var) = path_var else {
        return false;
    };
    ffmpeg_in_dirs(std::env::split_paths(path_var))
}

/// True when any directory in `dirs` holds an executable ffmpeg.
fn ffmpeg_in_dirs<I>(dirs: I) -> bool
where
    I: IntoIterator<Item = PathBuf>,
{
    dirs.into_iter().any(|dir| {
        ffmpeg_candidate_names()
            .iter()
            .any(|name| is_executable_file(&dir.join(name)))
    })
}

/// Candidate binary names, most specific first.
fn ffmpeg_candidate_names() -> [&'static str; 2] {
    ["ffmpeg", "ffmpeg.exe"]
}

#[cfg(unix)]
fn is_executable_file(p: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    if !p.is_file() {
        return false;
    }
    std::fs::metadata(p)
        .map(|m| m.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(not(unix))]
fn is_executable_file(p: &Path) -> bool {
    p.is_file()
}

/// The full ffmpeg argv for one streaming encode (PURE: no I/O,
/// no process). Layout:
///
///   -y -f rawvideo -pix_fmt rgba -s WxH -r FPS -i -
///   -c:v libx264 -crf CRF -preset PRESET -pix_fmt yuv420p OUT_PART
///
/// `-i -` is stdin; frames must be fed as tightly packed RGBA,
/// exactly `width * height * 4` bytes each, in display order.
pub(crate) fn build_args(
    width: u32,
    height: u32,
    fps: u32,
    crf: u32,
    preset: &str,
    out_part: &str,
) -> Vec<String> {
    vec![
        "-y".to_string(),
        "-f".to_string(),
        "rawvideo".to_string(),
        "-pix_fmt".to_string(),
        "rgba".to_string(),
        "-s".to_string(),
        format!("{width}x{height}"),
        "-r".to_string(),
        fps.to_string(),
        "-i".to_string(),
        "-".to_string(),
        "-c:v".to_string(),
        "libx264".to_string(),
        "-crf".to_string(),
        crf.to_string(),
        "-preset".to_string(),
        preset.to_string(),
        "-pix_fmt".to_string(),
        "yuv420p".to_string(),
        out_part.to_string(),
    ]
}

/// Byte length of one raw RGBA frame, or None on overflow.
pub(crate) fn expected_frame_bytes(width: u32, height: u32) -> Option<usize> {
    (width as usize)
        .checked_mul(height as usize)?
        .checked_mul(4)
}

/// The write_frame length check, factored out as a pure function
/// so tests can exercise it without spawning ffmpeg.
fn validate_frame_len(width: u32, height: u32, len: usize) -> TaleResult<()> {
    match expected_frame_bytes(width, height) {
        Some(expected) if len == expected => Ok(()),
        Some(expected) => Err(TaleError::render(
            "ffstream write_frame",
            format!(
                "frame has {len} bytes, expected {expected} ({width}x{height} RGBA)"
            ),
        )),
        None => Err(TaleError::render(
            "ffstream write_frame",
            format!("frame dimensions {width}x{height} overflow the RGBA byte count"),
        )),
    }
}

/// The last `STDERR_TAIL_MAX_CHARS` characters of ffmpeg's stderr
/// (trailing whitespace trimmed; "..." prefix when truncated).
fn stderr_tail(full: &str) -> String {
    let trimmed = full.trim_end();
    let count = trimmed.chars().count();
    if count <= STDERR_TAIL_MAX_CHARS {
        return trimmed.to_string();
    }
    let tail: String = trimmed
        .chars()
        .skip(count - STDERR_TAIL_MAX_CHARS)
        .collect();
    format!("...{tail}")
}

/// Outcome of a successful `FfmpegSink::finish`.
#[derive(Debug, Clone)]
pub(crate) struct FinishReport {
    /// Frames successfully written to ffmpeg's stdin.
    pub frames_written: u32,
    /// Tail of ffmpeg's stderr (see `stderr_tail`).
    pub stderr_tail: String,
}

/// Owner of one ffmpeg child process encoding a streamed film.
///
/// Lifecycle: `spawn` / `with_settings` -> `write_frame` per frame
/// -> `finish`. Dropping the sink without `finish` kills the child
/// (see the Drop impl) — there is no detached state.
pub(crate) struct FfmpegSink {
    child: Option<Child>,
    stdin: Option<ChildStdin>,
    stderr_thread: Option<JoinHandle<String>>,
    width: u32,
    height: u32,
    fps: u32,
    frames_written: u32,
    out_part: String,
}

impl FfmpegSink {
    /// Spawn ffmpeg with the production encode settings
    /// (CRF 18, preset "slow") writing the video-only file at
    /// `out_part` (the caller passes the `.part` path; see the
    /// `.part` discipline in the DELIVERY NOTE).
    pub(crate) fn spawn(out_part: &str, width: u32, height: u32, fps: u32) -> TaleResult<Self> {
        Self::with_settings(out_part, width, height, fps, DEFAULT_CRF, DEFAULT_PRESET)
    }

    /// Spawn ffmpeg with explicit encode settings.
    pub(crate) fn with_settings(
        out_part: &str,
        width: u32,
        height: u32,
        fps: u32,
        crf: u32,
        preset: &str,
    ) -> TaleResult<Self> {
        if width == 0 || height == 0 || fps == 0 {
            return Err(TaleError::render(
                "ffstream spawn",
                format!(
                    "invalid stream geometry {width}x{height} @ {fps} fps (all must be non-zero)"
                ),
            ));
        }
        if expected_frame_bytes(width, height).is_none() {
            return Err(TaleError::render(
                "ffstream spawn",
                format!("frame dimensions {width}x{height} overflow the RGBA byte count"),
            ));
        }
        if !ffmpeg_available() {
            return Err(TaleError::render(
                "ffstream spawn",
                "ffmpeg not found on PATH — install ffmpeg or use PNG output mode",
            ));
        }
        let args = build_args(width, height, fps, crf, preset, out_part);
        let mut child = Command::new("ffmpeg")
            .args(&args)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| {
                TaleError::render(
                    "ffstream spawn",
                    format!("failed to spawn ffmpeg for {out_part}: {e}"),
                )
            })?;
        let stdin = match child.stdin.take() {
            Some(stdin) => stdin,
            None => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(TaleError::render(
                    "ffstream spawn",
                    format!("ffmpeg for {out_part} spawned without a stdin pipe"),
                ));
            }
        };
        // Drain stderr continuously on its own thread: ffmpeg logs
        // progress to stderr, and an undrained pipe would fill and
        // block the child mid-encode (classic pipe deadlock).
        let stderr_thread = child.stderr.take().map(|mut pipe| {
            std::thread::spawn(move || {
                let mut buf = Vec::new();
                let _ = pipe.read_to_end(&mut buf);
                String::from_utf8_lossy(&buf).into_owned()
            })
        });
        Ok(FfmpegSink {
            child: Some(child),
            stdin: Some(stdin),
            stderr_thread,
            width,
            height,
            fps,
            frames_written: 0,
            out_part: out_part.to_string(),
        })
    }

    /// Feed one frame. `rgba` must be exactly `width*height*4`
    /// bytes (tightly packed RGBA, display order); anything else
    /// is rejected BEFORE a byte reaches ffmpeg, so a bad frame
    /// can never desynchronise the rawvideo stream.
    pub(crate) fn write_frame(&mut self, rgba: &[u8]) -> TaleResult<()> {
        validate_frame_len(self.width, self.height, rgba.len())?;
        let index = self.frames_written;
        let stdin = self.stdin.as_mut().ok_or_else(|| {
            TaleError::render(
                "ffstream write_frame",
                format!(
                    "ffmpeg stdin for {} is already closed (frame {index})",
                    self.out_part
                ),
            )
        })?;
        stdin.write_all(rgba).map_err(|e| {
            TaleError::render(
                "ffstream write_frame",
                format!(
                    "writing frame {index} to ffmpeg stdin for {} failed: {e}",
                    self.out_part
                ),
            )
        })?;
        self.frames_written += 1;
        Ok(())
    }

    /// Close stdin, wait for ffmpeg to finalize the `.part` file,
    /// and verify the run: non-zero exit OR a frame-count mismatch
    /// against `expected_frames` is a `TaleError::render` carrying
    /// the stderr tail. Consumes the sink either way (the child is
    /// always reaped; Drop is a no-op after finish).
    pub(crate) fn finish(mut self, expected_frames: u32) -> TaleResult<FinishReport> {
        let frames_written = self.frames_written;
        // EOF on stdin is ffmpeg's signal to flush and finalize.
        drop(self.stdin.take());
        let mut child = self.child.take().ok_or_else(|| {
            TaleError::render(
                "ffstream finish",
                format!("ffmpeg child for {} is already gone", self.out_part),
            )
        })?;
        let status = child.wait().map_err(|e| {
            TaleError::render(
                "ffstream finish",
                format!("waiting for ffmpeg writing {} failed: {e}", self.out_part),
            )
        })?;
        let stderr_full = self
            .stderr_thread
            .take()
            .map(|handle| handle.join().unwrap_or_default())
            .unwrap_or_default();
        let tail = stderr_tail(&stderr_full);
        let tail_for_msg = if tail.is_empty() {
            "(no stderr captured)".to_string()
        } else {
            tail.clone()
        };
        if !status.success() {
            return Err(TaleError::render(
                "ffstream finish",
                format!(
                    "ffmpeg exited with {status} writing {} ({frames_written} frame(s) fed); \
                     stderr tail: {tail_for_msg}",
                    self.out_part
                ),
            ));
        }
        if frames_written != expected_frames {
            return Err(TaleError::render(
                "ffstream finish",
                format!(
                    "frame-count mismatch closing {}: wrote {frames_written}, \
                     expected {expected_frames}; stderr tail: {tail_for_msg}",
                    self.out_part
                ),
            ));
        }
        Ok(FinishReport {
            frames_written,
            stderr_tail: tail,
        })
    }

    /// Frames successfully written so far.
    pub(crate) fn frames_written(&self) -> u32 {
        self.frames_written
    }

    /// Stream width the sink was spawned with.
    pub(crate) fn width(&self) -> u32 {
        self.width
    }

    /// Stream height the sink was spawned with.
    pub(crate) fn height(&self) -> u32 {
        self.height
    }

    /// Stream frame rate the sink was spawned with.
    pub(crate) fn fps(&self) -> u32 {
        self.fps
    }

    /// The `.part` output path the sink was spawned with.
    pub(crate) fn out_part(&self) -> &str {
        &self.out_part
    }
}

impl Drop for FfmpegSink {
    /// Abandoned sink (no finish): kill the child so no orphan
    /// ffmpeg keeps the `.part` file open, and reap it. The
    /// `.part` file itself is left on disk — deleting output is
    /// the caller's decision, never the sink's.
    fn drop(&mut self) {
        drop(self.stdin.take());
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        if let Some(handle) = self.stderr_thread.take() {
            let _ = handle.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_args_production_exact() {
        // The production shape: portrait film, master encode.
        let args = build_args(1080, 1920, 30, 18, "slow", "renders/film.mp4.part.mp4");
        assert_eq!(
            args.join(" "),
            "-y -f rawvideo -pix_fmt rgba -s 1080x1920 -r 30 -i - \
             -c:v libx264 -crf 18 -preset slow -pix_fmt yuv420p \
             renders/film.mp4.part.mp4"
        );
        // Structural pins: stdin input, output path last.
        let dash = args.iter().position(|a| a == "-i").unwrap();
        assert_eq!(args[dash + 1], "-");
        assert_eq!(args.last().unwrap(), "renders/film.mp4.part.mp4");
        assert_eq!(args.len(), 20);
    }

    #[test]
    fn build_args_parameters_flow_through() {
        // Every parameter must land in the argv — a draft-shaped
        // call proves none of the production values are hardcoded.
        let args = build_args(1280, 720, 24, 23, "veryfast", "draft.part.mp4");
        assert_eq!(
            args.join(" "),
            "-y -f rawvideo -pix_fmt rgba -s 1280x720 -r 24 -i - \
             -c:v libx264 -crf 23 -preset veryfast -pix_fmt yuv420p \
             draft.part.mp4"
        );
    }

    #[test]
    fn defaults_are_production_law() {
        assert_eq!(DEFAULT_CRF, 18);
        assert_eq!(DEFAULT_PRESET, "slow");
    }

    #[test]
    fn expected_frame_bytes_math() {
        assert_eq!(expected_frame_bytes(1080, 1920), Some(8_294_400));
        assert_eq!(expected_frame_bytes(2, 3), Some(24));
        assert_eq!(expected_frame_bytes(1, 1), Some(4));
    }

    #[test]
    fn validate_frame_len_accepts_exact_and_rejects_others() {
        assert!(validate_frame_len(4, 2, 32).is_ok());
        for bad in [0usize, 31, 33] {
            let e = validate_frame_len(4, 2, bad).unwrap_err();
            assert_eq!(e.category(), "render");
            assert_eq!(e.exit_code(), 3);
            assert!(
                e.to_string().contains("expected 32"),
                "message must name the expected size, got: {e}"
            );
        }
    }

    #[test]
    fn ffmpeg_in_path_empty_is_false() {
        assert!(!ffmpeg_in_path(None));
        assert!(!ffmpeg_in_path(Some(std::ffi::OsStr::new(""))));
        assert!(!ffmpeg_in_dirs(Vec::<PathBuf>::new()));
    }

    #[test]
    fn ffmpeg_in_dirs_finds_executable_file() {
        // A fake `ffmpeg` FILE in a temp dir: created, never run.
        let dir = std::env::temp_dir().join(format!(
            "talerender-ffstream-find-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let fake = dir.join(if cfg!(windows) { "ffmpeg.exe" } else { "ffmpeg" });
        std::fs::write(&fake, b"not a real ffmpeg").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        assert!(ffmpeg_in_dirs(vec![dir.clone()]));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn ffmpeg_in_dirs_ignores_non_executable_file() {
        use std::os::unix::fs::PermissionsExt;
        let dir = std::env::temp_dir().join(format!(
            "talerender-ffstream-noexec-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let fake = dir.join("ffmpeg");
        std::fs::write(&fake, b"not a real ffmpeg").unwrap();
        std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o644)).unwrap();
        assert!(!ffmpeg_in_dirs(vec![dir.clone()]));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn ffmpeg_available_does_not_panic() {
        // The VALUE depends on the host (CI has ffmpeg, a bare dev
        // box may not); the contract is a plain bool, no spawn.
        let _found: bool = ffmpeg_available();
    }

    #[test]
    fn with_settings_rejects_zero_geometry_without_spawning() {
        // Validation runs before the PATH probe and before any
        // spawn, so these error on every host and run no process.
        for (w, h, fps) in [(0, 720, 30), (1280, 0, 30), (1280, 720, 0)] {
            let err = FfmpegSink::with_settings("x.part.mp4", w, h, fps, 18, "slow")
                .err()
                .expect("zero geometry must be rejected");
            assert_eq!(err.category(), "render");
            assert!(err.to_string().contains("invalid stream geometry"));
        }
    }

    #[test]
    fn stderr_tail_keeps_short_output_verbatim() {
        assert_eq!(stderr_tail("ffmpeg version 8.1.2\n"), "ffmpeg version 8.1.2");
        assert_eq!(stderr_tail(""), "");
        assert_eq!(stderr_tail("one line"), "one line");
    }

    #[test]
    fn stderr_tail_truncates_long_output() {
        let long = "a".repeat(STDERR_TAIL_MAX_CHARS + 100);
        let tail = stderr_tail(&long);
        assert_eq!(tail.chars().count(), STDERR_TAIL_MAX_CHARS + 3);
        assert!(tail.starts_with("..."));
        assert!(tail.ends_with("aaa"));
    }
}
