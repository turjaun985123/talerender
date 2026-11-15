//! streammodes — WP4 streaming modes + recovery completion:
//! output-mode selection/reporting, stream-session planning,
//! interrupted-stream recovery policy, and the queue drain loop.
//!
//! =================================================================
//! DELIVERY NOTE — Batch 2, Seat 4 (WP4 streaming modes +
//! recovery completion)
//! =================================================================
//! FILES CREATED:
//!   - src/streammodes.rs  (this file; the ONLY file this seat
//!     wrote — no existing file edited, no git run, no new
//!     dependencies: std + in-crate modules only)
//!
//! CONTRACT POINTS IMPLEMENTED (V2-CONTRACTS.md §3 C3 modes
//! paragraph + C8 `output_mode` values; master prompt §8):
//!   - `OutputMode { Png, Stream, Auto }` + `parse_mode(&str)
//!     -> Option<OutputMode>` for `png|stream|auto` (trimmed,
//!     ASCII case-insensitive, following mempressure::
//!     parse_profile's house style; anything else is None) +
//!     `mode_from_config(&str) -> OutputMode` mapping C8's
//!     resolved string (`config::Config::output_mode`, already
//!     validated by config.rs against {"png","stream","auto"}).
//!     mode_from_config is TOTAL: an unrecognised string falls
//!     back to Png — C8's own default (config.rs
//!     DEFAULT_OUTPUT_MODE = "png") and the safe direction
//!     (Png never silently acquires an ffmpeg dependency).
//!   - `resolve_mode(requested, ffmpeg_on_path) ->
//!     ResolvedMode { mode, reason }`: `mode` is NEVER Auto
//!     after resolution. Auto resolves Stream iff
//!     `ffmpeg_on_path`, else Png, and `reason` SAYS SO in
//!     words (it names auto, ffmpeg, and the outcome) — the
//!     string is for the caller to PRINT: the mode choice is
//!     reported, never silent (master prompt §8). An explicit
//!     Stream request is NEVER silently downgraded: without
//!     ffmpeg it still resolves Stream, with a reason warning
//!     that spawn will fail (ffstream::FfmpegSink::spawn
//!     returns the hard TaleError) — only Auto consents to
//!     fallback. Explicit Png ignores ffmpeg entirely.
//!   - `StreamPlan` + `StreamPlan::plan(out_mp4, w, h, fps,
//!     expected)`: PURE planning, no I/O. Applies the `.part`
//!     discipline exactly as ffstream/finish.sh spell it:
//!     out_part = "<out>.part.mp4" (so "film.mp4" streams to
//!     "film.mp4.part.mp4"), out_final = out_mp4 unchanged.
//!   - `recovery_decision(part_exists, part_frames_known,
//!     expected) -> Recovery { FreshStart, DiscardPartial,
//!     NothingToRecover }`: an interrupted stream's `.part`
//!     is NEVER resumed mid-encode — a rawvideo-fed x264
//!     stream cannot append (the encoder state, headers and
//!     frame accounting all died with the ffmpeg child), so
//!     the honest recovery is discard + restart from frame 0.
//!     Decision table (see the fn doc for the full text):
//!     expected == 0 -> NothingToRecover; no `.part` ->
//!     FreshStart; ANY existing `.part` with expected > 0 ->
//!     DiscardPartial, INCLUDING a fully-fed one
//!     (part_frames_known == Some(expected)): feeding is not
//!     finishing — ffstream::finish() never ran on it, so
//!     its exit status and frame count were never verified
//!     and its container was never finalised. Frame-count
//!     accounting for a LIVE stream stays finish()'s job
//!     (ffstream), per §8; this fn only decides the fate of
//!     a leftover file. The fn is pure: deleting the file is
//!     the caller's act.
//!   - `drain_queue(rx, sink, pool, stats) -> TaleResult<u32>`:
//!     the consumer drain loop as real, compilable code (see
//!     TRAIT ADAPTATION below). Recv until the channel closes,
//!     validate each packet (streamq::FramePacket::validate),
//!     write it, record the pop in StreamStats, release the
//!     payload buffer to the PacketPool, count. Returns the
//!     number of frames written by THIS drain. On a validate
//!     or write error it returns the error immediately: the
//!     failed packet is neither counted nor pooled, and the
//!     caller drops the sink — FfmpegSink's Drop kills the
//!     ffmpeg child, leaving the `.part` for
//!     recovery_decision on the next run.
//!
//! TRAIT ADAPTATION (the one design note the lead must know):
//!   The charter spells drain_queue with `sink: &mut
//!   FfmpegSink`, but FfmpegSink is concrete and cannot be
//!   faked in tests. The loop is therefore factored over the
//!   small `FrameSink` trait defined HERE (write_frame +
//!   frames_written — exactly FfmpegSink's existing surface),
//!   and this file carries `impl FrameSink for FfmpegSink`
//!   delegating to the inherent methods (legal: the trait is
//!   local to this crate). The lead wires the charter shape
//!   verbatim — `drain_queue(rx, &mut sink, &mut pool,
//!   &stats)` with an FfmpegSink IS the concrete signature —
//!   and the tests below drive the same loop through an
//!   in-memory fake, including the error path. No change to
//!   ffstream.rs is needed or requested.
//!
//! WIRING THE LEAD MUST DO:
//!   1. mod line — in src/main.rs, with the other mod decls
//!      (anchor: after `mod streamq;` / near `mod ffstream;`):
//!          mod streammodes;
//!   2. Mode selection (film entry, before rendering):
//!          let requested =
//!              crate::streammodes::mode_from_config(
//!                  &cfg.output_mode);
//!          let resolved = crate::streammodes::resolve_mode(
//!              requested, crate::ffstream::ffmpeg_available());
//!          println!("output-mode: {} — {}",
//!              resolved.mode.as_str(), resolved.reason);
//!      The println is NOT optional decoration: §8/C3 require
//!      the chosen mode to be reported, never silent.
//!   3. Stream session (when resolved.mode == Stream):
//!          let plan = crate::streammodes::StreamPlan::plan(
//!              out_mp4, width, height, fps, expected_frames);
//!          match crate::streammodes::recovery_decision(
//!              std::path::Path::new(&plan.out_part).exists(),
//!              None, plan.expected_frames) {
//!              Recovery::DiscardPartial =>
//!                  { std::fs::remove_file(&plan.out_part)?; }
//!              _ => {}
//!          }
//!          let mut sink = crate::ffstream::FfmpegSink::spawn(
//!              &plan.out_part, plan.width, plan.height,
//!              plan.fps)?;
//!      (FfmpegSink's `-y` would overwrite a stale `.part`
//!      anyway; the explicit remove makes the discard a
//!      reported decision, not an accident of argv.)
//!   4. Consumer thread: own rx + sink + a PacketPool, run
//!          let n = crate::streammodes::drain_queue(
//!              rx, &mut sink, &mut pool, &stats)?;
//!      then `sink.finish(plan.expected_frames)` — finish()
//!      owns the frame-count reconciliation (n vs expected
//!      is the caller's cross-check line). On a drain error,
//!      drop the sink WITHOUT finish (Drop kills ffmpeg).
//!   5. No Cargo.toml change: std only, zero new deps.
//!
//! TESTS INCLUDED (#[cfg(test)], CPU-only, run by CI at
//! integration — this seat cannot compile locally):
//!   - parse_mode / mode_from_config matrices (incl. junk,
//!     case/whitespace tolerance, config fallback to Png)
//!   - resolve_mode full matrix: png/stream/auto x ffmpeg
//!     present/absent; resolved mode is never Auto; the
//!     Auto-without-ffmpeg reason mentions ffmpeg and png
//!   - plan path shapes (production + plain + nested paths)
//!   - recovery_decision table (incl. the fully-fed `.part`
//!     still DiscardPartial, and expected == 0)
//!   - drain_queue over a real frame_queue channel with an
//!     in-memory FakeSink: happy path (order, count, stats,
//!     pool retention), empty channel, sink write-error path,
//!     invalid-packet path (nothing written)
//!
//! KNOWN LIMITATIONS:
//!   - Planning/policy only: this module performs NO file
//!     I/O (recovery_decision does not stat or delete;
//!     StreamPlan does not create directories) and spawns
//!     nothing. All effects belong to the callers above.
//!   - part_frames_known is caller-supplied accounting
//!     (e.g. a StreamStats snapshot or runlog count from the
//!     interrupted run); nothing here probes the `.part`
//!     file itself (no ffprobe), and per the table above a
//!     known count never changes the verdict for an existing
//!     `.part` — it exists in the signature so callers can
//!     log it and so a future, verified finishing path can
//!     be added without changing the call shape.
//!   - drain_queue does not call finish(): finalisation and
//!     its frame-count check are ffstream's contract, and
//!     conflating them would hide which stage failed.
//! =================================================================
//!
//! Pixel law: nothing in this module touches a rendered
//! pixel. It selects, reports and plans how already-rendered
//! frames leave the process, unchanged.

use std::sync::mpsc::Receiver;

use crate::errors::{TaleError, TaleResult};
use crate::ffstream::FfmpegSink;
use crate::streamq::{FramePacket, PacketPool, StreamStats};

// ---------------------------------------------------------------------
// OutputMode (contract C3 modes; config values per C8)
// ---------------------------------------------------------------------

/// The three output modes of master prompt §8 / contract C3:
/// (1) Png — the existing PNG sequence, preserved; (2) Stream
/// — direct FFmpeg streaming; (3) Auto — automatic with
/// reported fallback (see `resolve_mode`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum OutputMode {
    Png,
    Stream,
    Auto,
}

impl OutputMode {
    /// The canonical config/CLI spelling (C8's value set).
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            OutputMode::Png => "png",
            OutputMode::Stream => "stream",
            OutputMode::Auto => "auto",
        }
    }
}

/// Parse an output-mode string: `png` | `stream` | `auto`,
/// trimmed and ASCII case-insensitive (the tolerance house
/// style of mempressure::parse_profile). Anything else is
/// `None` — callers fall through to the next resolution
/// source rather than guessing a mode.
pub(crate) fn parse_mode(raw: &str) -> Option<OutputMode> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "png" => Some(OutputMode::Png),
        "stream" => Some(OutputMode::Stream),
        "auto" => Some(OutputMode::Auto),
        _ => None,
    }
}

/// Map C8's resolved `output_mode` string (config::Config::
/// output_mode — already validated by config.rs, default
/// "png") to an OutputMode. Total function: config guarantees
/// the value set, so the fallback arm is unreachable for a
/// resolved Config; if a hand-built string ever reaches it,
/// the fallback is Png, C8's default — never Stream, so an
/// unrecognised value can never silently acquire an ffmpeg
/// dependency or change the output path.
pub(crate) fn mode_from_config(resolved: &str) -> OutputMode {
    parse_mode(resolved).unwrap_or(OutputMode::Png)
}

// ---------------------------------------------------------------------
// Mode resolution — reported, never silent (§8)
// ---------------------------------------------------------------------

/// The outcome of resolving a requested mode against the
/// environment: a concrete `mode` (NEVER Auto) plus the
/// human-readable `reason` the caller prints. The reason is
/// part of the contract, not decoration: §8 requires the
/// chosen mode and any fallback to be reported.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ResolvedMode {
    pub mode: OutputMode,
    pub reason: String,
}

/// Resolve `requested` against ffmpeg's presence on PATH
/// (pass `ffstream::ffmpeg_available()`).
///
/// - Png -> Png, always; ffmpeg is irrelevant and unused.
/// - Stream -> Stream, always. Without ffmpeg the reason
///   warns that spawning the sink will fail with ffstream's
///   hard error: an explicit request is never silently
///   downgraded to a different output path.
/// - Auto -> Stream iff `ffmpeg_on_path`, else Png; the
///   reason names auto, ffmpeg, and the resolved mode either
///   way, so the fallback is on the record.
pub(crate) fn resolve_mode(requested: OutputMode, ffmpeg_on_path: bool) -> ResolvedMode {
    match requested {
        OutputMode::Png => ResolvedMode {
            mode: OutputMode::Png,
            reason: "requested png: PNG sequence output (ffmpeg not used)".to_string(),
        },
        OutputMode::Stream => {
            if ffmpeg_on_path {
                ResolvedMode {
                    mode: OutputMode::Stream,
                    reason: "requested stream: ffmpeg found on PATH; \
                             direct FFmpeg streaming"
                        .to_string(),
                }
            } else {
                ResolvedMode {
                    mode: OutputMode::Stream,
                    reason: "requested stream: ffmpeg NOT found on PATH; \
                             streaming will fail at spawn — an explicit \
                             stream request is never silently downgraded to png"
                        .to_string(),
                }
            }
        }
        OutputMode::Auto => {
            if ffmpeg_on_path {
                ResolvedMode {
                    mode: OutputMode::Stream,
                    reason: "auto: ffmpeg found on PATH; resolved to stream".to_string(),
                }
            } else {
                ResolvedMode {
                    mode: OutputMode::Png,
                    reason: "auto: ffmpeg not found on PATH; \
                             resolved to png (PNG sequence fallback)"
                        .to_string(),
                }
            }
        }
    }
}

// ---------------------------------------------------------------------
// StreamPlan — pure session planning (no I/O)
// ---------------------------------------------------------------------

/// Everything a streaming session needs, planned up front:
/// geometry, rate, the frame count `ffstream::finish` will
/// be checked against, and the `.part` / final output paths.
/// Pure data — constructing one touches nothing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct StreamPlan {
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub expected_frames: u32,
    /// "<out_final>.part.mp4" — where the sink encodes.
    pub out_part: String,
    /// The caller's final MP4 path, unchanged.
    pub out_final: String,
}

impl StreamPlan {
    /// Plan a stream to `out_mp4` applying the `.part`
    /// discipline shared with film/finish.sh and ffstream:
    /// the encoder writes `format!("{out_mp4}.part.mp4")`
    /// and only the finishing step (mux + faststart rename,
    /// after finish() verifies the frame count) promotes it
    /// to `out_mp4`. A crashed run therefore never leaves a
    /// plausible-looking final file — only a `.part`, whose
    /// fate `recovery_decision` decides.
    pub(crate) fn plan(
        out_mp4: &str,
        width: u32,
        height: u32,
        fps: u32,
        expected_frames: u32,
    ) -> StreamPlan {
        StreamPlan {
            width,
            height,
            fps,
            expected_frames,
            out_part: format!("{out_mp4}.part.mp4"),
            out_final: out_mp4.to_string(),
        }
    }
}

// ---------------------------------------------------------------------
// Recovery — an interrupted stream is discarded, never resumed
// ---------------------------------------------------------------------

/// What the caller should do about a previous run's `.part`
/// before starting a new stream. See `recovery_decision`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Recovery {
    /// No `.part` exists: start the encode from frame 0,
    /// no cleanup needed.
    FreshStart,
    /// A `.part` exists and must be deleted, then the
    /// encode restarts from frame 0. The partial is NEVER
    /// resumed or appended to (see fn doc).
    DiscardPartial,
    /// No stream was in flight (nothing was expected), so
    /// there is no interrupted encode to recover from.
    NothingToRecover,
}

/// Decide the fate of a leftover `.part` from an interrupted
/// stream. PURE: stats and deletes nothing — the caller acts
/// on the verdict (and reports it).
///
/// Why never resume: the sink is a rawvideo pipe into a
/// fresh x264 encode. The encoder's state, the container
/// headers and the frame accounting all lived in the dead
/// ffmpeg child; there is no append mode for such a stream,
/// and a `.part` that never went through
/// `ffstream::finish()` has unverified frame count, unknown
/// exit status and an unfinalised container. The honest
/// recovery is discard + restart, at any fill level.
///
/// Decision table (first matching row wins):
///   expected == 0                       -> NothingToRecover
///     (no frames were ever expected: no encode was in
///     flight, whatever stray file may exist is not an
///     interrupted stream)
///   part_exists == false                -> FreshStart
///   part_exists == true, expected > 0   -> DiscardPartial
///     — for EVERY part_frames_known, including
///     Some(expected): fully FED is not fully FINISHED.
///     `part_frames_known` (the caller's accounting from
///     the interrupted run, e.g. a StreamStats snapshot) is
///     accepted for logging/future use and deliberately
///     never upgrades a partial to keepable.
pub(crate) fn recovery_decision(
    part_exists: bool,
    part_frames_known: Option<u32>,
    expected: u32,
) -> Recovery {
    if expected == 0 {
        return Recovery::NothingToRecover;
    }
    if !part_exists {
        return Recovery::FreshStart;
    }
    // A `.part` exists and frames were expected. The known
    // count — none, some, even all of them — cannot rescue
    // it: finish() never verified this file. Discard it.
    let _ = part_frames_known;
    Recovery::DiscardPartial
}

// ---------------------------------------------------------------------
// FrameSink trait + the drain loop
// ---------------------------------------------------------------------

/// The minimal sink surface the drain loop needs: feed one
/// tight-RGBA frame, and report how many frames have been
/// written. This is exactly FfmpegSink's existing surface;
/// the trait exists so the loop is testable with an
/// in-memory fake (FfmpegSink is a concrete child-process
/// owner and cannot be faked).
pub(crate) trait FrameSink {
    fn write_frame(&mut self, rgba: &[u8]) -> TaleResult<()>;
    fn frames_written(&self) -> u32;
}

impl FrameSink for FfmpegSink {
    fn write_frame(&mut self, rgba: &[u8]) -> TaleResult<()> {
        // Fully qualified: the inherent method, not this one.
        FfmpegSink::write_frame(self, rgba)
    }
    fn frames_written(&self) -> u32 {
        FfmpegSink::frames_written(self)
    }
}

/// Drain the bounded frame queue into `sink` until the
/// channel closes (the producer dropped its sender): the
/// consumer half of contract C3.
///
/// Per packet, in order: `validate()` the tight-RGBA
/// invariant, write the payload to the sink, record the
/// pop in `stats` (bytes stay counted at push — see
/// streamq::StreamStats), release the payload buffer to
/// `pool` for producer reuse, and count it. Returns the
/// number of frames written by this drain; the caller
/// cross-checks it and then calls the sink's finish
/// (ffstream::FfmpegSink::finish owns the authoritative
/// frame-count reconciliation against the plan's
/// expected_frames).
///
/// Error path: a failed validate or write returns the
/// error immediately. The failed packet is not counted,
/// not recorded as popped, and its buffer is not pooled —
/// and frames already written stay written. The caller
/// must then drop the sink WITHOUT finishing it: for
/// FfmpegSink, Drop kills the ffmpeg child, and the
/// orphaned `.part` is `recovery_decision`'s business on
/// the next run. (The count of frames written before the
/// error remains available via `sink.frames_written()` /
/// the stats snapshot.)
pub(crate) fn drain_queue<S: FrameSink>(
    rx: Receiver<FramePacket>,
    sink: &mut S,
    pool: &mut PacketPool,
    stats: &StreamStats,
) -> TaleResult<u32> {
    let mut written: u32 = 0;
    loop {
        let packet = match rx.recv() {
            Ok(packet) => packet,
            // Channel closed and empty: the producer is
            // done. Normal termination of the drain.
            Err(_) => break,
        };
        if !packet.validate() {
            return Err(TaleError::render(
                "streammodes drain_queue",
                format!(
                    "frame {} failed validation: {}x{} packet carries {} bytes, \
                     expected {} (tight RGBA) — packet not written",
                    packet.index,
                    packet.width,
                    packet.height,
                    packet.byte_len(),
                    packet.width as u128 * packet.height as u128 * 4u128,
                ),
            ));
        }
        // On error this propagates verbatim (ffstream's
        // TaleError already names the frame and the sink's
        // `.part` path); the caller drops the sink, whose
        // Drop kills ffmpeg. See the fn doc.
        sink.write_frame(&packet.rgba)?;
        stats.record_pop_packet(&packet);
        pool.release(packet.rgba);
        written += 1;
    }
    Ok(written)
}

// ---------------------------------------------------------------------
// Tests — CPU only. No GPU, no FFmpeg process, no filesystem.
// ---------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::streamq::{frame_queue, FramePacket};

    // ----- parse_mode / mode_from_config -----

    #[test]
    fn parse_mode_accepts_the_c8_value_set() {
        assert_eq!(parse_mode("png"), Some(OutputMode::Png));
        assert_eq!(parse_mode("stream"), Some(OutputMode::Stream));
        assert_eq!(parse_mode("auto"), Some(OutputMode::Auto));
        // Trimmed + case-insensitive (house parse tolerance).
        assert_eq!(parse_mode("  PNG "), Some(OutputMode::Png));
        assert_eq!(parse_mode("Stream"), Some(OutputMode::Stream));
        assert_eq!(parse_mode("\tauto\n"), Some(OutputMode::Auto));
    }

    #[test]
    fn parse_mode_rejects_anything_else() {
        for bad in ["", " ", "bmp", "pngs", "str eam", "autoo", "default", "0"] {
            assert_eq!(parse_mode(bad), None, "{bad:?} must not parse");
        }
    }

    #[test]
    fn mode_from_config_maps_resolved_strings() {
        // C8's resolved values, incl. config's default "png".
        assert_eq!(mode_from_config("png"), OutputMode::Png);
        assert_eq!(mode_from_config("stream"), OutputMode::Stream);
        assert_eq!(mode_from_config("auto"), OutputMode::Auto);
        // Total-function fallback: junk degrades to the C8
        // default Png, never to Stream.
        assert_eq!(mode_from_config("bogus"), OutputMode::Png);
        assert_eq!(mode_from_config(""), OutputMode::Png);
        // Round-trip: as_str is the spelling config stores.
        for m in [OutputMode::Png, OutputMode::Stream, OutputMode::Auto] {
            assert_eq!(mode_from_config(m.as_str()), m);
        }
    }

    // ----- resolve_mode -----

    #[test]
    fn resolve_mode_png_ignores_ffmpeg() {
        for ffmpeg in [true, false] {
            let r = resolve_mode(OutputMode::Png, ffmpeg);
            assert_eq!(r.mode, OutputMode::Png);
            assert!(r.reason.contains("png"), "reason: {}", r.reason);
        }
    }

    #[test]
    fn resolve_mode_stream_is_never_downgraded() {
        let with = resolve_mode(OutputMode::Stream, true);
        assert_eq!(with.mode, OutputMode::Stream);
        assert!(with.reason.contains("ffmpeg"), "reason: {}", with.reason);
        // Explicit stream without ffmpeg: STILL Stream (the
        // spawn will hard-fail) — never a silent png fallback.
        let without = resolve_mode(OutputMode::Stream, false);
        assert_eq!(without.mode, OutputMode::Stream);
        assert!(without.reason.contains("NOT"), "reason: {}", without.reason);
        assert!(without.reason.contains("ffmpeg"), "reason: {}", without.reason);
    }

    #[test]
    fn resolve_mode_auto_follows_ffmpeg_and_says_so() {
        let with = resolve_mode(OutputMode::Auto, true);
        assert_eq!(with.mode, OutputMode::Stream);
        assert!(with.reason.contains("auto"), "reason: {}", with.reason);
        assert!(with.reason.contains("ffmpeg"), "reason: {}", with.reason);
        assert!(with.reason.contains("stream"), "reason: {}", with.reason);

        let without = resolve_mode(OutputMode::Auto, false);
        assert_eq!(without.mode, OutputMode::Png);
        // The task-pinned property: the fallback reason
        // mentions ffmpeg (and the mode it fell back to).
        assert!(without.reason.contains("ffmpeg"), "reason: {}", without.reason);
        assert!(without.reason.contains("png"), "reason: {}", without.reason);
        assert!(without.reason.contains("auto"), "reason: {}", without.reason);
    }

    #[test]
    fn resolve_mode_never_returns_auto() {
        for requested in [OutputMode::Png, OutputMode::Stream, OutputMode::Auto] {
            for ffmpeg in [true, false] {
                assert_ne!(resolve_mode(requested, ffmpeg).mode, OutputMode::Auto);
            }
        }
    }

    // ----- StreamPlan -----

    #[test]
    fn plan_applies_part_discipline() {
        let plan = StreamPlan::plan("renders/film.mp4", 1080, 1920, 30, 1811);
        assert_eq!(plan.width, 1080);
        assert_eq!(plan.height, 1920);
        assert_eq!(plan.fps, 30);
        assert_eq!(plan.expected_frames, 1811);
        assert_eq!(plan.out_final, "renders/film.mp4");
        assert_eq!(plan.out_part, "renders/film.mp4.part.mp4");
    }

    #[test]
    fn plan_path_shapes_for_plain_and_nested_names() {
        let plain = StreamPlan::plan("out.mp4", 1280, 720, 24, 96);
        assert_eq!(plain.out_part, "out.mp4.part.mp4");
        assert_eq!(plain.out_final, "out.mp4");
        // No extension assumptions: the suffix is appended,
        // never substituted.
        let bare = StreamPlan::plan("a/b/c/film", 2, 2, 30, 1);
        assert_eq!(bare.out_part, "a/b/c/film.part.mp4");
        assert_eq!(bare.out_final, "a/b/c/film");
    }

    // ----- recovery_decision -----

    #[test]
    fn recovery_decision_table() {
        // Nothing expected: no encode was in flight.
        assert_eq!(recovery_decision(false, None, 0), Recovery::NothingToRecover);
        assert_eq!(recovery_decision(true, Some(0), 0), Recovery::NothingToRecover);
        assert_eq!(recovery_decision(true, None, 0), Recovery::NothingToRecover);
        // No leftover: clean fresh start, whatever the
        // (meaningless) known count says.
        assert_eq!(recovery_decision(false, None, 1811), Recovery::FreshStart);
        assert_eq!(recovery_decision(false, Some(900), 1811), Recovery::FreshStart);
        // Any existing `.part` with frames expected is
        // discarded: unknown count, empty, partial…
        assert_eq!(recovery_decision(true, None, 1811), Recovery::DiscardPartial);
        assert_eq!(recovery_decision(true, Some(0), 1811), Recovery::DiscardPartial);
        assert_eq!(recovery_decision(true, Some(900), 1811), Recovery::DiscardPartial);
        // …and even fully fed or over-fed: finish() never
        // verified this file, so it is not keepable.
        assert_eq!(recovery_decision(true, Some(1811), 1811), Recovery::DiscardPartial);
        assert_eq!(recovery_decision(true, Some(2000), 1811), Recovery::DiscardPartial);
    }

    // ----- drain_queue with a fake sink -----

    /// In-memory FrameSink: records payload lengths, can be
    /// armed to fail on a chosen 0-based call.
    struct FakeSink {
        written_lens: Vec<usize>,
        calls: u32,
        fail_on_call: Option<u32>,
    }

    impl FakeSink {
        fn new() -> FakeSink {
            FakeSink { written_lens: Vec::new(), calls: 0, fail_on_call: None }
        }
    }

    impl FrameSink for FakeSink {
        fn write_frame(&mut self, rgba: &[u8]) -> TaleResult<()> {
            let call = self.calls;
            self.calls += 1;
            if self.fail_on_call == Some(call) {
                return Err(TaleError::render(
                    "fake sink",
                    format!("injected write failure on call {call}"),
                ));
            }
            self.written_lens.push(rgba.len());
            Ok(())
        }
        fn frames_written(&self) -> u32 {
            self.written_lens.len() as u32
        }
    }

    /// A valid small packet: 4x2 px = 32 payload bytes.
    fn packet(index: u32) -> FramePacket {
        FramePacket::new(index, 4, 2, vec![0u8; 32])
    }

    #[test]
    fn drain_writes_all_packets_and_recycles_buffers() {
        let (tx, rx) = frame_queue(8);
        let stats = StreamStats::new();
        for i in 0..3u32 {
            let p = packet(i);
            stats.record_push_packet(&p); // producer-side accounting
            tx.send(p).expect("send within capacity");
        }
        drop(tx); // channel close ends the drain

        let mut sink = FakeSink::new();
        let mut pool = PacketPool::new(4);
        let n = drain_queue(rx, &mut sink, &mut pool, &stats).expect("clean drain");
        assert_eq!(n, 3);
        assert_eq!(sink.frames_written(), 3);
        assert_eq!(sink.written_lens, vec![32, 32, 32]);
        // Stats: pops match pushes; bytes counted once, at push.
        let snap = stats.snapshot();
        assert_eq!(snap.pushed, 3);
        assert_eq!(snap.popped, 3);
        assert_eq!(snap.bytes, 96);
        // Every written buffer was released to the pool in
        // canonical shape (len == capacity == 32).
        assert_eq!(pool.retained(), 3);
    }

    #[test]
    fn drain_empty_channel_returns_zero() {
        let (tx, rx) = frame_queue(2);
        drop(tx);
        let stats = StreamStats::new();
        let mut sink = FakeSink::new();
        let mut pool = PacketPool::new(2);
        assert_eq!(drain_queue(rx, &mut sink, &mut pool, &stats).expect("empty drain"), 0);
        assert_eq!(sink.frames_written(), 0);
        assert_eq!(stats.snapshot().popped, 0);
    }

    #[test]
    fn drain_propagates_sink_write_error() {
        let (tx, rx) = frame_queue(8);
        let stats = StreamStats::new();
        for i in 0..3u32 {
            tx.send(packet(i)).expect("send within capacity");
        }
        drop(tx);

        let mut sink = FakeSink::new();
        sink.fail_on_call = Some(1); // second frame fails
        let mut pool = PacketPool::new(4);
        let err = drain_queue(rx, &mut sink, &mut pool, &stats)
            .err()
            .expect("write error must propagate");
        assert_eq!(err.category(), "render");
        assert!(err.to_string().contains("injected write failure"), "got: {err}");
        // Exactly the frames before the failure were written,
        // counted, popped and pooled; the failed frame was
        // none of those. (The caller now drops the sink —
        // for FfmpegSink that Drop kills ffmpeg.)
        assert_eq!(sink.frames_written(), 1);
        assert_eq!(stats.snapshot().popped, 1);
        assert_eq!(pool.retained(), 1);
    }

    #[test]
    fn drain_rejects_invalid_packet_without_writing() {
        let (tx, rx) = frame_queue(8);
        let stats = StreamStats::new();
        tx.send(packet(0)).expect("send");
        // 31 bytes for a 4x2 frame: fails validate().
        tx.send(FramePacket::new(1, 4, 2, vec![0u8; 31])).expect("send");
        tx.send(packet(2)).expect("send");
        drop(tx);

        let mut sink = FakeSink::new();
        let mut pool = PacketPool::new(4);
        let err = drain_queue(rx, &mut sink, &mut pool, &stats)
            .err()
            .expect("invalid packet must error the drain");
        assert_eq!(err.category(), "render");
        assert!(err.to_string().contains("frame 1"), "got: {err}");
        // Only the valid first frame reached the sink; the
        // drain stopped at the bad packet (frame 2 unsent
        // to the sink even though it was queued).
        assert_eq!(sink.frames_written(), 1);
        assert_eq!(stats.snapshot().popped, 1);
    }
}
