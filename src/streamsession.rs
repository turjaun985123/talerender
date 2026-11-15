//! streamsession — WP4 streaming SESSION layer (Chrome-
//! Replacement Wave, Batch 1, Seat 2): the session that makes
//! `--output-mode stream` real in `run_film`.
//!
//! =================================================================
//! DELIVERY NOTE — CR-B1-02 (see docs/integration/
//! CR-B1-02-STREAMSESSION.md for the lead's wiring spec)
//! =================================================================
//! FILES CREATED:
//!   - src/streamsession.rs (this file)
//!   - docs/integration/CR-B1-02-STREAMSESSION.md
//! No existing file edited, no git run, no new dependencies
//! (std + in-crate modules only).
//!
//! WHAT THIS MODULE IS
//! The V2 streaming core landed as pieces — `streamq` (bounded
//! queue + stats + pool), `ffstream` (video-only ffmpeg sink),
//! `streammodes` (mode resolution, `.part` plan, recovery
//! policy, drain loop) — but no code owned a whole session:
//! today `main.rs` resolves the mode, prints it, and
//! `cli_error`s on Stream. This module is that owner:
//!
//!   producer (calling thread) --FramePacket--> bounded queue
//!     --(backpressure)--> consumer thread --drain_queue-->
//!     FullContractSink (one ffmpeg child) --> <out>.part.mp4
//!     --ffprobe validate--> rename --> <out>
//!
//! * Producer interface matches `film::run_film` exactly:
//!   frame index in, TIGHT RGBA bytes out — the `Vec<u8>` that
//!   `Renderer::render` + the CPU overlays produce today and
//!   that the PNG path wraps in `image::RgbaImage::from_raw`.
//!   It is NOT PNG bytes: the rawvideo sink consumes RGBA, and
//!   a PNG-byte producer would need a decode the render loop
//!   never performs. PNG stays the disk path's serialisation,
//!   nothing else changes.
//! * The producer runs on the CALLING thread (the GPU
//!   `Renderer` in `film.rs` is built and used there; nothing
//!   in this module requires it to be `Send`). Only packets,
//!   `StreamStats` (Arc) and the sink cross threads.
//! * Every frame is accounted five ways — produced (producer
//!   loop), pushed / popped (`StreamStats`), drained
//!   (`streammodes::drain_queue` return), written (sink's
//!   `FinishReport`) — and all five must equal
//!   `expected_frames`, else the session fails loudly with
//!   every count named. A sixth check follows: ffprobe counts
//!   the encoded file's frames independently before rename.
//!
//! WHY THE SINK IS NOT `ffstream::FfmpegSink` (contradiction
//! found in the landed code, named for the lead)
//! `ffstream::build_args` encodes VIDEO ONLY: no BT.709
//! `-color_*` / `-x264-params` VUI triple, no audio input, no
//! AAC, no `+faststart`, no `-threads 4`, no `color_range`.
//! Its own delivery note says finishing "stays in the
//! existing finishing step". But there is no stream-copy
//! finishing step that can repair that after the fact: the
//! x264 VUI values must be written AT ENCODE TIME
//! (film/finish.sh header + docs/ab/film/MP4-EVIDENCE.md
//! finding F1: `-color_*` alone landed only colorspace), and
//! re-encoding the video-only `.part` would be a second x264
//! generation — a silent quality change, which is forbidden.
//! So this module's `FullContractSink` spawns ONE ffmpeg with
//! the adapter's FULL finishing argv and rawvideo on stdin —
//! exactly the shape `capture.mjs` video mode has always used
//! (frames piped into the same ffmpeg that muxes the audio),
//! with the frame source swapped from image2pipe JPEG to
//! rawvideo RGBA. `ffstream` remains the video-only sink for
//! any caller that wants one; the session does not use it as
//! its encoder. Its `expected_frame_bytes`, `ffmpeg_available`
//! and `FinishReport` ARE reused.
//!
//! ENCODE SETTINGS ARE CONTRACT, NOT CHOICES (`EncodeProfile`)
//! Mirrored argument-for-argument from the adapter
//! (`template/tools/capture_native.mjs` video finishing, which
//! itself mirrors `capture.mjs`):
//!   Master: libx264 preset slow, CRF 18, -threads 4,
//!     yuv420p, -color_primaries/-color_trc/-colorspace bt709,
//!     -color_range tv, -x264-params
//!     colorprim=bt709:transfer=bt709:colormatrix=bt709,
//!     AAC 256k @ 48000.
//!   Draft: libx264 preset veryfast, CRF 23, yuv420p,
//!     AAC 192k @ 48000 — and, exactly as in the adapter, NO
//!     colour triple and NO -threads on the draft path. That
//!     asymmetry is the adapter's own; reproducing it is
//!     parity, not a downgrade this module introduces.
//!   Both: -shortest, -movflags +faststart, audio sliced
//!   `-ss <start> -t <dur>` from `data/mix.wav` when a mix is
//!   supplied, else the adapter's silent lavfi track
//!   (`anullsrc=r=48000:cl=stereo`) — reported, never silent
//!   about being silent (see `SessionReport::audio`).
//!   film/finish.sh differs in details (no -threads /
//!   -color_range / -shortest / -ar, AAC 256k, mix REQUIRED);
//!   where the two contracts disagree the ADAPTER is the one
//!   this session mirrors, because the task names the
//!   adapter's finishing contract and the adapter is the
//!   skill's production finishing path. The doc note lists
//!   every divergence.
//!
//! SCHEDULER VERDICT (WP7, contract C5)
//! CONSUMED, in one place only: `run_session` builds its
//! index list with `scheduler::plan(FrameRange, &empty_done)`.
//! The `done` set is EMPTY by design and the session proves
//! the returned plan contiguous — streaming has no resume.
//! `streammodes::recovery_decision` already settled this: an
//! interrupted stream's `.part` is discarded and the encode
//! restarts from frame 0, because a rawvideo-fed x264 stream
//! cannot append. A scheduler "done" index in stream mode
//! would be a GAP in the encoded film (the queue carries a
//! contiguous sequence; a skipped index is a missing frame,
//! not a saving), so `run_session_inner` VALIDATES any plan
//! it is handed (exact length, strictly +1 contiguous,
//! starting at `spec.first_index`) and fails loudly on a
//! gapped one, before any process is spawned. What is NOT
//! consumed, and why:
//!   - `scheduler::batches` / `SchedOpts::max_in_flight`:
//!     batching bounds CONCURRENT rendering; `run_film` has
//!     exactly one `Renderer` on one thread, and in-flight
//!     STREAM frames are already bounded by the C1 queue
//!     capacity in `spec.queue_capacity`. Grouping a
//!     sequential loop's indices changes nothing.
//!   - Checkpoint files: a checkpoint exists to resume; a
//!     stream cannot resume (above). Writing one would
//!     promise a capability the format does not have.
//!   - `CancelFlag`: no signal handler / supervisor exists in
//!     `main.rs` today to own one; wiring cancellation is a
//!     CLI decision for the lead, not a session behaviour.
//!
//! VALIDATION (the repo's existing approach)
//! No Rust code probed an encoded file before this module
//! (ffstream's note says so explicitly). The repo's existing
//! validation approach is the ffprobe record of
//! docs/ab/film/MP4-EVIDENCE.md: `-count_frames` +
//! `nb_read_frames` (header `nb_frames` is not trusted), the
//! stream record, and the duration note. `probe_mp4` runs
//! exactly that probe (JSON form, parsed with the crate's own
//! `scene::parse_json` — no new dependency) and
//! `validate_probe` checks it against the spec: counted
//! frames == expected (exact), dimensions, h264, frame rate,
//! an AAC audio stream present, container duration within a
//! stated tolerance of expected/fps, and — Master only — all
//! three colour fields == bt709. Validation runs on the
//! `.part` BEFORE the rename; a file that fails it is never
//! promoted. ffprobe is a hard preflight: `run_session`
//! checks ffmpeg AND ffprobe availability before a single
//! frame is produced, so a missing probe fails in seconds,
//! not after a full render.
//!
//! FAILURE BEHAVIOUR (full table in the doc note)
//!   - ffmpeg dies mid-stream: the consumer's next
//!     `write_frame` gets EPIPE (or `finish` sees the
//!     non-zero exit); the drain error kills the session, the
//!     producer's blocked `send` unblocks with a disconnect,
//!     the `.part` is removed best-effort, NO final file is
//!     created, and the returned `TaleError::render` carries
//!     the stage + the ffmpeg stderr tail.
//!   - Producer error at frame k: the abort flag is set, the
//!     sender drops, the consumer drains what is queued then
//!     drops the sink WITHOUT `finish` (Drop kills ffmpeg);
//!     the producer's error is the one returned.
//!   - Count mismatch at any of the five/six checks: loud
//!     `TaleError::render` naming every count; no rename.
//!   - Leftover `.part` from a previous run:
//!     `streammodes::recovery_decision` -> DiscardPartial ->
//!     removed before spawn (the wiring note gives the caller
//!     the log line that reports the discard).
//!
//! WIRING THE LEAD MUST DO: see
//! docs/integration/CR-B1-02-STREAMSESSION.md (mod line,
//! main.rs Stream branch, film.rs producer closure + spec
//! construction). No Cargo.toml change: std only.
//!
//! TESTS INCLUDED (#[cfg(test)]):
//!   - build_full_args byte-for-byte for Master+mix and
//!     Draft+silent; EncodeProfile table pinned.
//!   - validate_spec / validate_plan rejections.
//!   - Queue/accounting core over a fake producer + fake sink:
//!     happy path (all counts), producer error (no finish,
//!     no promotion), sink write error, wrong-length frame,
//!     finish count mismatch (session-level reconciliation),
//!     gapped plan rejected before sink creation.
//!   - probe JSON parsing + validate_probe matrix (pure).
//!   - One hermetic real-ffmpeg smoke: 6 frames of 64x64 from
//!     a fake producer through `run_session` in a temp dir,
//!     asserting the renamed final exists, the `.part` is
//!     gone, and ffprobe counted 6 frames. It self-skips when
//!     ffmpeg/ffprobe are absent from PATH (host-dependent,
//!     the ffstream tests' own convention); on this VM and in
//!     CI both are present and it runs.
//! =================================================================
//!
//! Pixel law: nothing in this module touches a rendered
//! pixel. It moves already-rendered tight RGBA bytes through
//! a bounded queue into an encoder, unchanged, in order.

use std::collections::HashSet;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;

use crate::errors::{TaleError, TaleResult};
use crate::ffstream::FinishReport;
use crate::streamq::{FramePacket, PacketPool, StreamStats};

// ---------------------------------------------------------------------
// Encode profile — the adapter's finishing contract, as data
// ---------------------------------------------------------------------

/// Which of the adapter's two finishing argument sets a
/// session encodes with. The lead derives this from the run's
/// draft state (see the wiring note); this module never
/// chooses it silently.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EncodeProfile {
    /// Master: x264 slow / CRF 18, BT.709, AAC 256k.
    Master,
    /// Draft: x264 veryfast / CRF 23, AAC 192k, no colour
    /// triple — the adapter's draft set, verbatim.
    Draft,
}

impl EncodeProfile {
    pub(crate) fn crf(&self) -> u32 {
        match self {
            EncodeProfile::Master => 18,
            EncodeProfile::Draft => 23,
        }
    }
    pub(crate) fn preset(&self) -> &'static str {
        match self {
            EncodeProfile::Master => "slow",
            EncodeProfile::Draft => "veryfast",
        }
    }
    /// AAC bitrate spelling exactly as the adapter passes it.
    pub(crate) fn aac_bitrate(&self) -> &'static str {
        match self {
            EncodeProfile::Master => "256k",
            EncodeProfile::Draft => "192k",
        }
    }
    /// Whether the BT.709 `-color_*` + `-x264-params` VUI set
    /// is part of this profile's argv. True for Master only —
    /// the adapter's draft argv omits it (parity, see header).
    pub(crate) fn colour_tags(&self) -> bool {
        matches!(self, EncodeProfile::Master)
    }
    /// `-threads` value, or None to omit the flag. The adapter
    /// passes `-threads 4` on Master only.
    pub(crate) fn threads(&self) -> Option<u32> {
        match self {
            EncodeProfile::Master => Some(4),
            EncodeProfile::Draft => None,
        }
    }
    pub(crate) fn name(&self) -> &'static str {
        match self {
            EncodeProfile::Master => "master",
            EncodeProfile::Draft => "draft",
        }
    }
}

// ---------------------------------------------------------------------
// Audio source
// ---------------------------------------------------------------------

/// The session's audio input, decided by the CALLER (the
/// session never searches the filesystem for a mix: a path
/// the caller named is either used or the session fails —
/// see `validate_spec`).
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum AudioSpec {
    /// `data/mix.wav` (or any caller-named WAV), sliced the
    /// adapter's way: `-ss <start_secs> -t <duration_secs>`
    /// before `-i <path>` (input-side slice, as in
    /// capture_native.mjs). For a full-film run the caller
    /// passes start = first_index/fps and duration =
    /// expected_frames/fps.
    Mix {
        path: PathBuf,
        start_secs: f64,
        duration_secs: f64,
    },
    /// No mix: the adapter's silent track
    /// (`-f lavfi -i anullsrc=r=48000:cl=stereo`). The
    /// session report says so in words — a silent film is a
    /// reported state, never an accident.
    Silent,
}

impl AudioSpec {
    /// The report spelling: mirrors the adapter's own
    /// completion-line wording (`with data/mix.wav` /
    /// `SILENT: no data/mix.wav`).
    pub(crate) fn label(&self) -> String {
        match self {
            AudioSpec::Mix { path, .. } => format!("with mix.wav ({})", path.display()),
            AudioSpec::Silent => "SILENT: no data/mix.wav".to_string(),
        }
    }
    pub(crate) fn is_mix(&self) -> bool {
        matches!(self, AudioSpec::Mix { .. })
    }
}

// ---------------------------------------------------------------------
// Session spec
// ---------------------------------------------------------------------

/// Everything one streaming session needs, supplied by the
/// caller. Pure data; constructing one touches nothing.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct StreamSpec {
    /// Final MP4 path. The encoder writes
    /// `"<out_mp4>.part.mp4"` (the `streammodes::StreamPlan`
    /// spelling) and the session renames it here ONLY after
    /// encode + accounting + ffprobe validation all pass.
    pub out_mp4: PathBuf,
    pub width: u32,
    pub height: u32,
    /// Frame rate. `run_film` renders at a hardcoded 30 today
    /// (`frames_total = ceil(duration * 30)`, `t = n / 30`);
    /// the lead passes 30 — this field exists so the session
    /// never hardcodes a rate the caller did not state.
    pub fps: u32,
    /// Index of the first frame of the range (`--from`).
    pub first_index: u32,
    /// Frames this session must produce and encode:
    /// `to - from` as the lead computes it in `run_film`.
    pub expected_frames: u32,
    pub profile: EncodeProfile,
    pub audio: AudioSpec,
    /// Bounded-queue capacity. MUST come from C1
    /// (`MemPressure::frame_queue_capacity`: 2 / 4 / 8) at the
    /// call site, exactly as `streamq::frame_queue` requires.
    pub queue_capacity: usize,
}

impl StreamSpec {
    /// The `streammodes::StreamPlan` for this spec — the one
    /// source of the `.part` / final path spelling.
    pub(crate) fn plan(&self) -> TaleResult<crate::streammodes::StreamPlan> {
        let out = self.out_mp4.to_str().ok_or_else(|| {
            TaleError::render(
                "streamsession spec",
                format!("output path {} is not valid UTF-8", self.out_mp4.display()),
            )
        })?;
        Ok(crate::streammodes::StreamPlan::plan(
            out,
            self.width,
            self.height,
            self.fps,
            self.expected_frames,
        ))
    }
}

/// Reject a spec that cannot produce a contract-conforming
/// file, BEFORE any frame is produced or process spawned.
/// Every rejection names the field and the remedy.
fn validate_spec(spec: &StreamSpec) -> TaleResult<()> {
    let bad = |msg: String| TaleError::render("streamsession spec", msg);
    if spec.width == 0 || spec.height == 0 {
        return Err(bad(format!(
            "frame geometry {}x{} is zero-sized",
            spec.width, spec.height
        )));
    }
    if spec.width % 2 != 0 || spec.height % 2 != 0 {
        // yuv420p subsamples chroma 2x2: odd dimensions cannot
        // be encoded as yuv420p without scaling or cropping,
        // both of which are silent quality/geometry changes.
        // (main.rs's effective_size already rounds draft sizes
        // to even for exactly this reason.)
        return Err(bad(format!(
            "frame geometry {}x{} is odd-sized; yuv420p requires even dimensions \
             (no scaling/cropping is performed silently)",
            spec.width, spec.height
        )));
    }
    if spec.fps == 0 {
        return Err(bad("fps is 0".to_string()));
    }
    if spec.expected_frames == 0 {
        return Err(bad(
            "expected_frames is 0: a stream of no frames is not a film \
             (check the --from/--to range)"
                .to_string(),
        ));
    }
    if crate::ffstream::expected_frame_bytes(spec.width, spec.height).is_none() {
        return Err(bad(format!(
            "frame dimensions {}x{} overflow the RGBA byte count",
            spec.width, spec.height
        )));
    }
    if spec.out_mp4.as_os_str().is_empty() {
        return Err(bad("output mp4 path is empty".to_string()));
    }
    // The plan spelling requires a UTF-8 path (StreamPlan is
    // String-based, like ffstream's sink).
    let _ = spec.plan()?;
    if let AudioSpec::Mix { path, start_secs, duration_secs } = &spec.audio {
        if !path.is_file() {
            // The caller NAMED a mix. Silently substituting the
            // silent track would ship a silent film under a
            // "with audio" assumption — the adapter only goes
            // silent when NO mix exists at its one known path,
            // a caller-side discovery this session does not do.
            return Err(bad(format!(
                "audio mix {} does not exist or is not a file — \
                 fix the path, or pass AudioSpec::Silent explicitly",
                path.display()
            )));
        }
        if !start_secs.is_finite() || *start_secs < 0.0 {
            return Err(bad(format!("audio start_secs {start_secs} is not a finite >= 0 value")));
        }
        if !duration_secs.is_finite() || *duration_secs <= 0.0 {
            return Err(bad(format!(
                "audio duration_secs {duration_secs} is not a finite > 0 value"
            )));
        }
    }
    Ok(())
}

/// The stream contiguity law, checked against a scheduler
/// plan: a stream plan must be EXACTLY
/// `first_index .. first_index + expected_frames`, ascending
/// by 1. A gapped plan (e.g. a scheduler plan built from a
/// non-empty done set) is rejected here, before any spawn —
/// in a stream, a skipped index is a missing encoded frame.
fn validate_plan(spec: &StreamSpec, plan: &[u32]) -> TaleResult<()> {
    let bad = |msg: String| TaleError::render("streamsession plan", msg);
    if plan.len() != spec.expected_frames as usize {
        return Err(bad(format!(
            "plan carries {} indices but the session expects {} frames — \
             a streaming plan must be the full contiguous range (streaming \
             has no resume; a scheduler done-set does not apply here)",
            plan.len(),
            spec.expected_frames
        )));
    }
    for (i, &idx) in plan.iter().enumerate() {
        let want = spec.first_index + i as u32;
        if idx != want {
            return Err(bad(format!(
                "plan[{i}] is frame {idx}, expected {want}: streaming plans \
                 must be strictly contiguous from first_index {}",
                spec.first_index
            )));
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------
// Producer interface — run_film's frame production, abstracted
// ---------------------------------------------------------------------

/// One frame of production: given a frame index, return its
/// tight RGBA bytes (`width * height * 4`, the shape
/// `Renderer::render` + overlays produce in `run_film` and
/// `streamq::FramePacket::validate` checks). An error aborts
/// the session (see the header's failure table).
pub(crate) trait FrameProducer {
    fn produce(&mut self, index: u32) -> TaleResult<Vec<u8>>;
}

/// Any `FnMut(u32) -> TaleResult<Vec<u8>>` is a producer —
/// the wiring in `film.rs` is a closure over the renderer,
/// paint cache and assets, with no new named type needed.
impl<F> FrameProducer for F
where
    F: FnMut(u32) -> TaleResult<Vec<u8>>,
{
    fn produce(&mut self, index: u32) -> TaleResult<Vec<u8>> {
        self(index)
    }
}

// ---------------------------------------------------------------------
// Session sink — the drain target + finaliser
// ---------------------------------------------------------------------

/// What the consumer thread needs from an encoder: the
/// `streammodes::FrameSink` surface (so the landed
/// `drain_queue` drives it verbatim) plus `finish`, the
/// close-out that flushes the encoder and verifies its own
/// written count. Implemented by `FullContractSink` in
/// production and by fakes in tests.
pub(crate) trait SessionSink: crate::streammodes::FrameSink {
    fn finish(self, expected_frames: u32) -> TaleResult<FinishReport>;
}

// ---------------------------------------------------------------------
// FullContractSink — one ffmpeg, full adapter argv, rawvideo in
// ---------------------------------------------------------------------

/// How many stderr characters error messages / reports keep
/// (the ffstream convention).
const STDERR_TAIL_MAX_CHARS: usize = 4096;

fn stderr_tail(full: &str) -> String {
    let trimmed = full.trim_end();
    let count = trimmed.chars().count();
    if count <= STDERR_TAIL_MAX_CHARS {
        return trimmed.to_string();
    }
    let tail: String = trimmed.chars().skip(count - STDERR_TAIL_MAX_CHARS).collect();
    format!("...{tail}")
}

/// Seconds as the adapter's JS `String(x)` would spell the
/// values ffmpeg accepts: Rust's `{}` for f64 prints `2` for
/// 2.0 and `1.5` for 1.5, matching that spelling for the
/// magnitudes a film slice uses.
fn fmt_secs(v: f64) -> String {
    format!("{v}")
}

/// The full streaming ffmpeg argv (PURE: no I/O, no process).
/// Argument-for-argument the adapter's video finishing, with
/// the frame input swapped from its image2 PNG sequence to
/// rawvideo on stdin:
///
///   -v error -y -f rawvideo -pix_fmt rgba -s WxH -r FPS -i -
///   [-ss S -t D -i MIX | -f lavfi -i anullsrc=r=48000:cl=stereo]
///   -map 0:v -map 1:a
///   -c:v libx264 -preset P -crf C [-threads 4] -pix_fmt yuv420p
///   [Master: -color_primaries bt709 -color_trc bt709
///     -colorspace bt709 -color_range tv
///     -x264-params colorprim=bt709:transfer=bt709:colormatrix=bt709]
///   -c:a aac -b:a B -ar 48000
///   -shortest -movflags +faststart OUT_PART
///
/// `-i -` is stdin; frames must be fed as tightly packed
/// RGBA, exactly `width * height * 4` bytes each, in order.
pub(crate) fn build_full_args(
    width: u32,
    height: u32,
    fps: u32,
    profile: EncodeProfile,
    audio: &AudioSpec,
    out_part: &str,
) -> Vec<String> {
    let mut a: Vec<String> = vec![
        "-v".into(),
        "error".into(),
        "-y".into(),
        "-f".into(),
        "rawvideo".into(),
        "-pix_fmt".into(),
        "rgba".into(),
        "-s".into(),
        format!("{width}x{height}"),
        "-r".into(),
        fps.to_string(),
        "-i".into(),
        "-".into(),
    ];
    // Audio input (input 1), the adapter's two shapes.
    match audio {
        AudioSpec::Mix { path, start_secs, duration_secs } => {
            a.push("-ss".into());
            a.push(fmt_secs(*start_secs));
            a.push("-t".into());
            a.push(fmt_secs(*duration_secs));
            a.push("-i".into());
            a.push(path.to_string_lossy().into_owned());
        }
        AudioSpec::Silent => {
            a.push("-f".into());
            a.push("lavfi".into());
            a.push("-i".into());
            a.push("anullsrc=r=48000:cl=stereo".into());
        }
    }
    a.push("-map".into());
    a.push("0:v".into());
    a.push("-map".into());
    a.push("1:a".into());
    // Video encode (the profile's contract set).
    a.push("-c:v".into());
    a.push("libx264".into());
    a.push("-preset".into());
    a.push(profile.preset().into());
    a.push("-crf".into());
    a.push(profile.crf().to_string());
    if let Some(t) = profile.threads() {
        a.push("-threads".into());
        a.push(t.to_string());
    }
    a.push("-pix_fmt".into());
    a.push("yuv420p".into());
    if profile.colour_tags() {
        a.push("-color_primaries".into());
        a.push("bt709".into());
        a.push("-color_trc".into());
        a.push("bt709".into());
        a.push("-colorspace".into());
        a.push("bt709".into());
        a.push("-color_range".into());
        a.push("tv".into());
        a.push("-x264-params".into());
        a.push("colorprim=bt709:transfer=bt709:colormatrix=bt709".into());
    }
    // Audio encode + finishing flags (both profiles).
    a.push("-c:a".into());
    a.push("aac".into());
    a.push("-b:a".into());
    a.push(profile.aac_bitrate().into());
    a.push("-ar".into());
    a.push("48000".into());
    a.push("-shortest".into());
    a.push("-movflags".into());
    a.push("+faststart".into());
    a.push(out_part.into());
    a
}

/// Owner of the session's one ffmpeg child. Lifecycle mirrors
/// `ffstream::FfmpegSink` (spawn -> write_frame per frame ->
/// finish; Drop without finish kills and reaps the child) —
/// the duplication of that lifecycle is deliberate and is
/// explained in the header: this sink's argv carries the
/// full finishing contract ffstream's deliberately omits.
pub(crate) struct FullContractSink {
    child: Option<Child>,
    stdin: Option<ChildStdin>,
    stderr_thread: Option<JoinHandle<String>>,
    width: u32,
    height: u32,
    frames_written: u32,
    out_part: String,
}

impl FullContractSink {
    /// Spawn ffmpeg for `spec`, writing `plan.out_part`.
    /// Fails before spawning when ffmpeg is not on PATH.
    pub(crate) fn spawn(spec: &StreamSpec) -> TaleResult<Self> {
        if spec.width == 0 || spec.height == 0 || spec.fps == 0 {
            return Err(TaleError::render(
                "streamsession spawn",
                format!(
                    "invalid stream geometry {}x{} @ {} fps (all must be non-zero)",
                    spec.width, spec.height, spec.fps
                ),
            ));
        }
        if crate::ffstream::expected_frame_bytes(spec.width, spec.height).is_none() {
            return Err(TaleError::render(
                "streamsession spawn",
                format!(
                    "frame dimensions {}x{} overflow the RGBA byte count",
                    spec.width, spec.height
                ),
            ));
        }
        if !crate::ffstream::ffmpeg_available() {
            return Err(TaleError::render(
                "streamsession spawn",
                "ffmpeg not found on PATH — install ffmpeg or use --output-mode png",
            ));
        }
        let plan = spec.plan()?;
        let args = build_full_args(
            spec.width,
            spec.height,
            spec.fps,
            spec.profile,
            &spec.audio,
            &plan.out_part,
        );
        let mut child = Command::new("ffmpeg")
            .args(&args)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| {
                TaleError::render(
                    "streamsession spawn",
                    format!("failed to spawn ffmpeg for {}: {e}", plan.out_part),
                )
            })?;
        let stdin = match child.stdin.take() {
            Some(stdin) => stdin,
            None => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(TaleError::render(
                    "streamsession spawn",
                    format!("ffmpeg for {} spawned without a stdin pipe", plan.out_part),
                ));
            }
        };
        // Drain stderr continuously on its own thread (the
        // ffstream deadlock rule: an undrained pipe fills and
        // blocks the child mid-encode).
        let stderr_thread = child.stderr.take().map(|mut pipe| {
            std::thread::spawn(move || {
                let mut buf = Vec::new();
                let _ = pipe.read_to_end(&mut buf);
                String::from_utf8_lossy(&buf).into_owned()
            })
        });
        Ok(FullContractSink {
            child: Some(child),
            stdin: Some(stdin),
            stderr_thread,
            width: spec.width,
            height: spec.height,
            frames_written: 0,
            out_part: plan.out_part,
        })
    }

    /// Feed one frame; the byte length is validated BEFORE a
    /// byte reaches ffmpeg, so a bad frame can never
    /// desynchronise the rawvideo stream.
    pub(crate) fn write_frame(&mut self, rgba: &[u8]) -> TaleResult<()> {
        let expected = crate::ffstream::expected_frame_bytes(self.width, self.height);
        match expected {
            Some(e) if rgba.len() == e => {}
            Some(e) => {
                return Err(TaleError::render(
                    "streamsession write_frame",
                    format!(
                        "frame {} has {} bytes, expected {e} ({}x{} RGBA)",
                        self.frames_written,
                        rgba.len(),
                        self.width,
                        self.height
                    ),
                ))
            }
            None => {
                return Err(TaleError::render(
                    "streamsession write_frame",
                    format!(
                        "frame dimensions {}x{} overflow the RGBA byte count",
                        self.width, self.height
                    ),
                ))
            }
        }
        let index = self.frames_written;
        let stdin = self.stdin.as_mut().ok_or_else(|| {
            TaleError::render(
                "streamsession write_frame",
                format!("ffmpeg stdin for {} is already closed (frame {index})", self.out_part),
            )
        })?;
        stdin.write_all(rgba).map_err(|e| {
            TaleError::render(
                "streamsession write_frame",
                format!(
                    "writing frame {index} to ffmpeg stdin for {} failed: {e} \
                     (ffmpeg may have exited; its stderr tail is reported at finish)",
                    self.out_part
                ),
            )
        })?;
        self.frames_written += 1;
        Ok(())
    }

    pub(crate) fn frames_written(&self) -> u32 {
        self.frames_written
    }

    pub(crate) fn out_part(&self) -> &str {
        &self.out_part
    }

    /// Close stdin, wait for ffmpeg, verify: non-zero exit OR
    /// `frames_written != expected_frames` is a
    /// `TaleError::render` with the stderr tail. Same protocol
    /// as `ffstream::FfmpegSink::finish`.
    pub(crate) fn finish(mut self, expected_frames: u32) -> TaleResult<FinishReport> {
        let frames_written = self.frames_written;
        drop(self.stdin.take());
        let mut child = self.child.take().ok_or_else(|| {
            TaleError::render(
                "streamsession finish",
                format!("ffmpeg child for {} is already gone", self.out_part),
            )
        })?;
        let status = child.wait().map_err(|e| {
            TaleError::render(
                "streamsession finish",
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
                "streamsession finish",
                format!(
                    "ffmpeg exited with {status} writing {} ({frames_written} frame(s) fed); \
                     stderr tail: {tail_for_msg}",
                    self.out_part
                ),
            ));
        }
        if frames_written != expected_frames {
            return Err(TaleError::render(
                "streamsession finish",
                format!(
                    "frame-count mismatch closing {}: wrote {frames_written}, \
                     expected {expected_frames}; stderr tail: {tail_for_msg}",
                    self.out_part
                ),
            ));
        }
        Ok(FinishReport { frames_written, stderr_tail: tail })
    }
}

impl crate::streammodes::FrameSink for FullContractSink {
    fn write_frame(&mut self, rgba: &[u8]) -> TaleResult<()> {
        FullContractSink::write_frame(self, rgba)
    }
    fn frames_written(&self) -> u32 {
        FullContractSink::frames_written(self)
    }
}

impl SessionSink for FullContractSink {
    fn finish(self, expected_frames: u32) -> TaleResult<FinishReport> {
        FullContractSink::finish(self, expected_frames)
    }
}

impl Drop for FullContractSink {
    /// Abandoned sink (no finish): kill + reap the child so
    /// no orphan ffmpeg holds the `.part`; the file itself is
    /// left for the session's cleanup / recovery policy.
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

// ---------------------------------------------------------------------
// ffprobe validation — the MP4-EVIDENCE approach, in code
// ---------------------------------------------------------------------

/// True when an executable `ffprobe` is on PATH. Same probe
/// shape as `ffstream::ffmpeg_available` (split $PATH,
/// stat-check, no spawn); ffstream's helper is ffmpeg-only
/// and private, so the small duplication is local to here.
pub(crate) fn ffprobe_available() -> bool {
    let Some(path_var) = std::env::var_os("PATH") else {
        return false;
    };
    std::env::split_paths(&path_var).any(|dir| {
        ["ffprobe", "ffprobe.exe"].iter().any(|name| {
            let p = dir.join(name);
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                p.is_file()
                    && std::fs::metadata(&p)
                        .map(|m| m.permissions().mode() & 0o111 != 0)
                        .unwrap_or(false)
            }
            #[cfg(not(unix))]
            {
                p.is_file()
            }
        })
    })
}

/// What ffprobe measured about one finished file. All fields
/// come from the counted-frames probe (see `probe_mp4`).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ProbeReport {
    /// Counted video frames (`nb_read_frames`, NOT the header
    /// `nb_frames` — MP4-EVIDENCE: "header fields can lie;
    /// count the frames").
    pub video_frames: u32,
    pub width: u32,
    pub height: u32,
    /// `avg_frame_rate` as a float (e.g. 30.0 for "30/1").
    pub avg_fps: f64,
    pub video_codec: String,
    /// The audio stream's codec when one is present.
    pub audio_codec: Option<String>,
    /// Container (`format`) duration in seconds. With a mix
    /// longer than the video and no `-shortest` this follows
    /// the audio (the finish.sh note); the session passes
    /// `-shortest`, so `validate_probe` expects it near the
    /// video duration within its stated tolerance.
    pub duration_secs: f64,
    pub color_primaries: Option<String>,
    pub color_transfer: Option<String>,
    pub color_space: Option<String>,
}

fn json_str(v: Option<&crate::scene::Value>) -> Option<String> {
    v.and_then(|v| v.as_str()).map(|s| s.to_string())
}

fn json_u32(v: Option<&crate::scene::Value>, ctx: &str) -> TaleResult<u32> {
    match v {
        Some(crate::scene::Value::Num(n)) if *n >= 0.0 => Ok(*n as u32),
        Some(crate::scene::Value::Str(s)) => s.trim().parse::<u32>().map_err(|_| {
            TaleError::render("streamsession probe", format!("{ctx}: '{s}' is not a u32"))
        }),
        _ => Err(TaleError::render(
            "streamsession probe",
            format!("{ctx}: missing or not a number"),
        )),
    }
}

fn json_f64(v: Option<&crate::scene::Value>, ctx: &str) -> TaleResult<f64> {
    match v {
        Some(crate::scene::Value::Num(n)) => Ok(*n),
        Some(crate::scene::Value::Str(s)) => s.trim().parse::<f64>().map_err(|_| {
            TaleError::render("streamsession probe", format!("{ctx}: '{s}' is not a number"))
        }),
        _ => Err(TaleError::render(
            "streamsession probe",
            format!("{ctx}: missing or not a number"),
        )),
    }
}

/// Parse ffprobe's JSON output into a `ProbeReport` (PURE:
/// the fixture-tested half of probing).
pub(crate) fn parse_probe_json(text: &str) -> TaleResult<ProbeReport> {
    let root = crate::scene::parse_json(text).map_err(|e| {
        TaleError::render("streamsession probe", format!("ffprobe JSON did not parse: {e}"))
    })?;
    let streams = root.get("streams").and_then(|v| v.as_arr()).ok_or_else(|| {
        TaleError::render("streamsession probe", "ffprobe JSON has no streams array".to_string())
    })?;
    let mut video: Option<ProbeReport> = None;
    let mut audio_codec: Option<String> = None;
    for s in streams {
        match s.get("codec_type").and_then(|v| v.as_str()) {
            Some("video") if video.is_none() => {
                let rate = json_str(s.get("avg_frame_rate")).unwrap_or_default();
                let mut parts = rate.split('/');
                let num: f64 = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0.0);
                let den: f64 = parts.next().and_then(|p| p.parse().ok()).unwrap_or(1.0);
                let avg_fps = if den != 0.0 { num / den } else { 0.0 };
                video = Some(ProbeReport {
                    video_frames: json_u32(
                        s.get("nb_read_frames"),
                        "video stream nb_read_frames",
                    )?,
                    width: json_u32(s.get("width"), "video stream width")?,
                    height: json_u32(s.get("height"), "video stream height")?,
                    avg_fps,
                    video_codec: json_str(s.get("codec_name")).unwrap_or_default(),
                    audio_codec: None, // filled in below
                    duration_secs: 0.0, // filled in below
                    color_primaries: json_str(s.get("color_primaries")),
                    color_transfer: json_str(s.get("color_transfer")),
                    color_space: json_str(s.get("color_space")),
                });
            }
            Some("audio") if audio_codec.is_none() => {
                audio_codec = json_str(s.get("codec_name"));
            }
            _ => {}
        }
    }
    let mut report = video.ok_or_else(|| {
        TaleError::render("streamsession probe", "ffprobe JSON has no video stream".to_string())
    })?;
    report.audio_codec = audio_codec;
    report.duration_secs = json_f64(
        root.get("format").and_then(|f| f.get("duration")),
        "format duration",
    )?;
    Ok(report)
}

/// Run the MP4-EVIDENCE probe over `path`:
/// `ffprobe -v error -count_frames -show_entries
/// stream=codec_type,codec_name,width,height,avg_frame_rate,
/// color_primaries,color_transfer,color_space,nb_read_frames
/// -show_entries format=duration -of json <path>`.
pub(crate) fn probe_mp4(path: &Path) -> TaleResult<ProbeReport> {
    let output = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-count_frames",
            "-show_entries",
            "stream=codec_type,codec_name,width,height,avg_frame_rate,\
             color_primaries,color_transfer,color_space,nb_read_frames",
            "-show_entries",
            "format=duration",
            "-of",
            "json",
        ])
        .arg(path)
        .output()
        .map_err(|e| {
            TaleError::render(
                "streamsession probe",
                format!("failed to run ffprobe on {}: {e} — install ffprobe", path.display()),
            )
        })?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(TaleError::render(
            "streamsession probe",
            format!(
                "ffprobe exited with {} on {}: {}",
                output.status,
                path.display(),
                stderr_tail(&stderr)
            ),
        ));
    }
    let text = String::from_utf8_lossy(&output.stdout);
    parse_probe_json(&text)
}

/// Check a probe against the spec that produced the file.
/// Every failed property is named in ONE error (the caller
/// sees the whole mismatch, not the first third of it).
pub(crate) fn validate_probe(spec: &StreamSpec, probe: &ProbeReport) -> TaleResult<()> {
    let mut failures: Vec<String> = Vec::new();
    if probe.video_frames != spec.expected_frames {
        failures.push(format!(
            "counted video frames {} != expected {}",
            probe.video_frames, spec.expected_frames
        ));
    }
    if probe.width != spec.width || probe.height != spec.height {
        failures.push(format!(
            "dimensions {}x{} != spec {}x{}",
            probe.width, probe.height, spec.width, spec.height
        ));
    }
    if probe.video_codec != "h264" {
        failures.push(format!("video codec '{}' != 'h264'", probe.video_codec));
    }
    if (probe.avg_fps - spec.fps as f64).abs() > 0.01 {
        failures.push(format!(
            "avg frame rate {} != spec fps {}",
            probe.avg_fps, spec.fps
        ));
    }
    match &probe.audio_codec {
        Some(c) if c == "aac" => {}
        Some(c) => failures.push(format!("audio codec '{c}' != 'aac'")),
        None => failures.push("no audio stream (the contract always carries AAC: mix or silent)".to_string()),
    }
    // Duration: the video is expected/fps seconds; the
    // container may run slightly long (AAC priming) and
    // -shortest may trim a fraction of the silent/mix tail.
    // Tolerance: two frames + 0.30 s, stated here, not tuned
    // per run.
    let expected_dur = spec.expected_frames as f64 / spec.fps as f64;
    let tol = 2.0 / spec.fps as f64 + 0.30;
    if (probe.duration_secs - expected_dur).abs() > tol {
        failures.push(format!(
            "container duration {:.3}s differs from video duration {:.3}s by more than {:.3}s",
            probe.duration_secs, expected_dur, tol
        ));
    }
    if spec.profile.colour_tags() {
        for (field, value) in [
            ("color_primaries", &probe.color_primaries),
            ("color_transfer", &probe.color_transfer),
            ("color_space", &probe.color_space),
        ] {
            match value {
                Some(v) if v == "bt709" => {}
                Some(v) => failures.push(format!("{field} '{v}' != 'bt709' (master contract)")),
                None => failures.push(format!("{field} missing != 'bt709' (master contract)")),
            }
        }
    }
    if failures.is_empty() {
        Ok(())
    } else {
        Err(TaleError::render(
            "streamsession validate",
            format!(
                "encoded file failed validation ({} check(s)): {}",
                failures.len(),
                failures.join("; ")
            ),
        ))
    }
}

// ---------------------------------------------------------------------
// The session
// ---------------------------------------------------------------------

/// The accounting core's report (everything except the
/// ffprobe record, which only the production path can make).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct CoreReport {
    pub frames_produced: u32,
    pub frames_encoded: u32,
    pub frames_expected: u32,
    pub bytes_streamed: u64,
    pub stderr_tail: String,
}

/// A completed session's full report.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SessionReport {
    pub frames_produced: u32,
    pub frames_encoded: u32,
    pub frames_expected: u32,
    pub bytes_streamed: u64,
    /// The final MP4 path (the rename target).
    pub out_final: PathBuf,
    /// Audio wording for the caller's completion line
    /// (`AudioSpec::label`).
    pub audio: String,
    /// Encode profile name (`master` / `draft`).
    pub profile: String,
    pub probe: ProbeReport,
    pub stderr_tail: String,
}

/// What the consumer thread hands back: how far the drain
/// got, and either the finish report or the error that ended
/// it. `aborted` marks the producer-error path, where the
/// sink was dropped WITHOUT finish by design.
struct ConsumerOutcome {
    drained: u32,
    result: TaleResult<FinishReport>,
    aborted: bool,
}

/// The session core, generic over the sink so tests drive it
/// with a fake. Production calls this via `run_session` with
/// `FullContractSink::spawn` as the factory.
///
/// Threading: the consumer thread owns the sink, the queue
/// receiver and a `PacketPool`; the producer (this thread)
/// owns `producer`, the sender and the plan. The bounded
/// queue's blocking send IS the backpressure.
pub(crate) fn run_session_inner<P, S, F>(
    spec: &StreamSpec,
    mut producer: P,
    plan: &[u32],
    make_sink: F,
) -> TaleResult<CoreReport>
where
    P: FrameProducer,
    S: SessionSink + Send + 'static,
    F: FnOnce(&StreamSpec) -> TaleResult<S> + Send + 'static,
{
    validate_spec(spec)?;
    validate_plan(spec, plan)?;
    let stream_plan = spec.plan()?;
    let expected = spec.expected_frames;

    // Output directory + recovery, before any spawn: the
    // adapter creates the output dir; the recovery decision
    // (streammodes) disposes of a previous run's `.part`.
    if let Some(parent) = spec.out_mp4.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .map_err(|e| TaleError::image_io(parent, e))?;
        }
    }
    let part_path = PathBuf::from(&stream_plan.out_part);
    match crate::streammodes::recovery_decision(part_path.exists(), None, expected) {
        crate::streammodes::Recovery::DiscardPartial => {
            std::fs::remove_file(&part_path)
                .map_err(|e| TaleError::image_io(part_path.clone(), e))?;
        }
        _ => {}
    }

    let stats = Arc::new(StreamStats::new());
    let abort = Arc::new(AtomicBool::new(false));
    let (tx, rx) = crate::streamq::frame_queue(spec.queue_capacity);

    // Consumer thread.
    let consumer = {
        let stats = Arc::clone(&stats);
        let abort = Arc::clone(&abort);
        let spec_owned = spec.clone();
        std::thread::spawn(move || -> ConsumerOutcome {
            let mut sink = match make_sink(&spec_owned) {
                Ok(sink) => sink,
                Err(e) => {
                    return ConsumerOutcome { drained: 0, result: Err(e), aborted: false };
                }
            };
            let mut pool = PacketPool::new(spec_owned.queue_capacity);
            let drained = crate::streammodes::drain_queue(rx, &mut sink, &mut pool, &stats);
            match drained {
                Err(e) => {
                    // Drop WITHOUT finish: for the real sink,
                    // Drop kills ffmpeg. The `.part` is the
                    // session cleanup's business (below). The
                    // drained count is not reported here: the
                    // session returns this error before any
                    // count is consulted.
                    drop(sink);
                    ConsumerOutcome { drained: 0, result: Err(e), aborted: false }
                }
                Ok(n) => {
                    if abort.load(Ordering::SeqCst) {
                        drop(sink);
                        ConsumerOutcome {
                            drained: n,
                            result: Err(TaleError::render(
                                "streamsession consumer",
                                "aborted before finish: the producer did not complete; \
                                 the sink was dropped without finish and the .part \
                                 was not promoted"
                                    .to_string(),
                            )),
                            aborted: true,
                        }
                    } else {
                        let result = sink.finish(expected);
                        ConsumerOutcome { drained: n, result, aborted: false }
                    }
                }
            }
        })
    };

    // Producer loop (calling thread).
    let mut produced: u32 = 0;
    let mut producer_err: Option<TaleError> = None;
    let mut send_failed = false;
    for &index in plan {
        let rgba = match producer.produce(index) {
            Ok(rgba) => rgba,
            Err(e) => {
                producer_err = Some(e);
                abort.store(true, Ordering::SeqCst);
                break;
            }
        };
        let packet = FramePacket::new(index, spec.width, spec.height, rgba);
        if !packet.validate() {
            producer_err = Some(TaleError::render(
                "streamsession producer",
                format!(
                    "frame {index} producer returned {} bytes, expected {} \
                     ({}x{} tight RGBA) — frame not queued",
                    packet.byte_len(),
                    spec.width as u128 * spec.height as u128 * 4u128,
                    spec.width,
                    spec.height
                ),
            ));
            abort.store(true, Ordering::SeqCst);
            break;
        }
        stats.record_push_packet(&packet);
        match tx.send(packet) {
            Ok(()) => produced += 1,
            Err(_) => {
                // The consumer is gone (sink spawn failed or
                // the drain errored): its outcome, joined
                // below, carries the root cause.
                send_failed = true;
                break;
            }
        }
    }
    drop(tx); // channel close is the consumer's end-of-stream

    let outcome = match consumer.join() {
        Ok(outcome) => outcome,
        Err(_) => {
            cleanup_part(&part_path);
            return Err(TaleError::render(
                "streamsession",
                "consumer thread panicked; the session is aborted and no file was promoted"
                    .to_string(),
            ));
        }
    };

    // Error precedence: a producer error on the abort path is
    // primary (the consumer's abort marker is its echo); a
    // consumer error alone is primary (ffmpeg died, spawn
    // failed); both together are reported together.
    let finish_report = match (producer_err, outcome.result) {
        (Some(pe), Err(_)) if outcome.aborted => {
            cleanup_part(&part_path);
            return Err(pe);
        }
        (Some(pe), Err(ce)) => {
            cleanup_part(&part_path);
            return Err(TaleError::render(
                "streamsession",
                format!("producer failed: {pe}; consumer also failed: {ce}"),
            ));
        }
        (Some(pe), Ok(_)) => {
            cleanup_part(&part_path);
            return Err(pe);
        }
        (None, Err(ce)) => {
            cleanup_part(&part_path);
            return Err(ce);
        }
        (None, Ok(report)) => report,
    };
    if send_failed {
        // Unreachable in practice (a dead consumer returns
        // Err above), but never let a short send pass as a
        // full session.
        cleanup_part(&part_path);
        return Err(TaleError::render(
            "streamsession",
            "the frame queue closed before all frames were sent, but the \
             consumer reported no error — accounting cannot be reconciled"
                .to_string(),
        ));
    }

    // Five-way reconciliation: produced (producer loop) ==
    // pushed == popped (StreamStats) == drained (drain_queue)
    // == written (sink finish) == expected.
    let snap = stats.snapshot();
    let encoded = finish_report.frames_written;
    let drained = outcome.drained;
    if produced != expected
        || snap.pushed != expected as u64
        || snap.popped != expected as u64
        || drained != expected
        || encoded != expected
    {
        cleanup_part(&part_path);
        return Err(TaleError::render(
            "streamsession accounting",
            format!(
                "frame-count mismatch at session close: produced {produced}, \
                 pushed {}, popped {}, drained {drained}, encoded {encoded}, \
                 expected {expected} — no file was promoted",
                snap.pushed, snap.popped
            ),
        ));
    }

    Ok(CoreReport {
        frames_produced: produced,
        frames_encoded: encoded,
        frames_expected: expected,
        bytes_streamed: snap.bytes,
        stderr_tail: finish_report.stderr_tail,
    })
}

/// Best-effort `.part` removal on a failed session: a failed
/// encode never leaves a plausible file behind (the
/// adapter's own failure discipline). Ignores its own errors
/// — the session's real error is already in hand, and a
/// leftover `.part` is `recovery_decision`'s business on the
/// next run.
fn cleanup_part(part_path: &Path) {
    if part_path.exists() {
        let _ = std::fs::remove_file(part_path);
    }
}

/// Run a complete streaming session: preflight (ffmpeg +
/// ffprobe present, before a single frame is produced),
/// scheduler plan, the accounting core over
/// `FullContractSink`, ffprobe validation of the `.part`,
/// and ONLY then the rename to the final path.
pub(crate) fn run_session<P>(spec: &StreamSpec, producer: P) -> TaleResult<SessionReport>
where
    P: FrameProducer,
{
    validate_spec(spec)?;
    if !crate::ffstream::ffmpeg_available() {
        return Err(TaleError::render(
            "streamsession preflight",
            "ffmpeg not found on PATH — install ffmpeg or use --output-mode png",
        ));
    }
    if !ffprobe_available() {
        return Err(TaleError::render(
            "streamsession preflight",
            "ffprobe not found on PATH — the session validates its output with \
             ffprobe before promoting it; install ffprobe (it ships with ffmpeg) \
             or use --output-mode png",
        ));
    }
    // Scheduler consumption (see the header's verdict): the
    // plan is the pure, ascending, FULL range — the done set
    // is empty because a stream cannot resume.
    let end = spec.first_index.checked_add(spec.expected_frames).ok_or_else(|| {
        TaleError::render(
            "streamsession plan",
            format!(
                "frame range {}..+{} overflows u32",
                spec.first_index, spec.expected_frames
            ),
        )
    })?;
    let plan = crate::scheduler::plan(
        crate::scheduler::FrameRange::new(spec.first_index, end),
        &HashSet::new(),
    );

    let stream_plan = spec.plan()?;
    let part_path = PathBuf::from(&stream_plan.out_part);
    let core = run_session_inner(spec, producer, &plan, |s| FullContractSink::spawn(s))?;

    // Sixth accounting check: probe the ENCODED file (not the
    // counts of the machinery that made it), then promote.
    let probe = match probe_mp4(&part_path).and_then(|p| {
        validate_probe(spec, &p)?;
        Ok(p)
    }) {
        Ok(p) => p,
        Err(e) => {
            cleanup_part(&part_path);
            return Err(e);
        }
    };
    std::fs::rename(&part_path, &spec.out_mp4)
        .map_err(|e| TaleError::image_io(spec.out_mp4.clone(), e))?;

    Ok(SessionReport {
        frames_produced: core.frames_produced,
        frames_encoded: core.frames_encoded,
        frames_expected: core.frames_expected,
        bytes_streamed: core.bytes_streamed,
        out_final: spec.out_mp4.clone(),
        audio: spec.audio.label(),
        profile: spec.profile.name().to_string(),
        probe,
        stderr_tail: core.stderr_tail,
    })
}

// ---------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ----- fixtures -----

    /// Small spec: 4x2 frames (32 bytes), 5 frames @ 30 fps.
    fn spec_in(dir: &Path) -> StreamSpec {
        StreamSpec {
            out_mp4: dir.join("film.mp4"),
            width: 4,
            height: 2,
            fps: 30,
            first_index: 0,
            expected_frames: 5,
            profile: EncodeProfile::Master,
            audio: AudioSpec::Silent,
            queue_capacity: 2,
        }
    }

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "talerender-streamsession-test-{}-{tag}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    fn full_plan(spec: &StreamSpec) -> Vec<u32> {
        (spec.first_index..spec.first_index + spec.expected_frames).collect()
    }

    /// Producer closure: solid frames, armable to fail or to
    /// return a wrong-length buffer at a chosen index.
    fn fake_producer(
        spec: &StreamSpec,
        fail_at: Option<u32>,
        short_at: Option<u32>,
    ) -> impl FnMut(u32) -> TaleResult<Vec<u8>> {
        let len = (spec.width * spec.height * 4) as usize;
        move |index| {
            if fail_at == Some(index) {
                return Err(TaleError::render(
                    "fake producer",
                    format!("fake producer failure at frame {index}"),
                ));
            }
            if short_at == Some(index) {
                return Ok(vec![0u8; len - 1]);
            }
            Ok(vec![index as u8; len])
        }
    }

    /// In-memory SessionSink: records payload lengths, can be
    /// armed to fail a write call, and to report a wrong
    /// finish count. `finish_called` lets tests prove the
    /// abort path never finishes.
    struct FakeSink {
        written_lens: Vec<usize>,
        calls: u32,
        fail_on_call: Option<u32>,
        finish_count_delta: i64,
        finish_called: Arc<AtomicBool>,
    }

    impl FakeSink {
        fn new(finish_called: Arc<AtomicBool>) -> FakeSink {
            FakeSink {
                written_lens: Vec::new(),
                calls: 0,
                fail_on_call: None,
                finish_count_delta: 0,
                finish_called,
            }
        }
    }

    impl crate::streammodes::FrameSink for FakeSink {
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

    impl SessionSink for FakeSink {
        fn finish(self, _expected_frames: u32) -> TaleResult<FinishReport> {
            self.finish_called.store(true, Ordering::SeqCst);
            let n = (self.written_lens.len() as i64 + self.finish_count_delta).max(0) as u32;
            Ok(FinishReport { frames_written: n, stderr_tail: String::new() })
        }
    }

    // ----- encode contract -----

    #[test]
    fn profile_table_is_the_adapter_contract() {
        assert_eq!(EncodeProfile::Master.crf(), 18);
        assert_eq!(EncodeProfile::Master.preset(), "slow");
        assert_eq!(EncodeProfile::Master.aac_bitrate(), "256k");
        assert!(EncodeProfile::Master.colour_tags());
        assert_eq!(EncodeProfile::Master.threads(), Some(4));
        assert_eq!(EncodeProfile::Draft.crf(), 23);
        assert_eq!(EncodeProfile::Draft.preset(), "veryfast");
        assert_eq!(EncodeProfile::Draft.aac_bitrate(), "192k");
        assert!(!EncodeProfile::Draft.colour_tags());
        assert_eq!(EncodeProfile::Draft.threads(), None);
    }

    #[test]
    fn build_full_args_master_with_mix_exact() {
        let audio = AudioSpec::Mix {
            path: PathBuf::from("data/mix.wav"),
            start_secs: 1.5,
            duration_secs: 2.25,
        };
        let args = build_full_args(1080, 1920, 30, EncodeProfile::Master, &audio, "renders/film.mp4.part.mp4");
        assert_eq!(
            args.join(" "),
            "-v error -y -f rawvideo -pix_fmt rgba -s 1080x1920 -r 30 -i - \
             -ss 1.5 -t 2.25 -i data/mix.wav -map 0:v -map 1:a \
             -c:v libx264 -preset slow -crf 18 -threads 4 -pix_fmt yuv420p \
             -color_primaries bt709 -color_trc bt709 -colorspace bt709 \
             -color_range tv -x264-params colorprim=bt709:transfer=bt709:colormatrix=bt709 \
             -c:a aac -b:a 256k -ar 48000 -shortest -movflags +faststart \
             renders/film.mp4.part.mp4"
        );
    }

    #[test]
    fn build_full_args_draft_silent_exact() {
        let args = build_full_args(1280, 720, 24, EncodeProfile::Draft, &AudioSpec::Silent, "draft.mp4.part.mp4");
        assert_eq!(
            args.join(" "),
            "-v error -y -f rawvideo -pix_fmt rgba -s 1280x720 -r 24 -i - \
             -f lavfi -i anullsrc=r=48000:cl=stereo -map 0:v -map 1:a \
             -c:v libx264 -preset veryfast -crf 23 -pix_fmt yuv420p \
             -c:a aac -b:a 192k -ar 48000 -shortest -movflags +faststart \
             draft.mp4.part.mp4"
        );
        // The draft set carries NO colour triple and NO
        // threads flag (adapter parity, pinned so a future
        // "helpful" addition is a visible contract change).
        assert!(!args.iter().any(|a| a == "-color_primaries"));
        assert!(!args.iter().any(|a| a == "-threads"));
    }

    // ----- spec / plan validation -----

    #[test]
    fn validate_spec_rejects_unencodable_specs() {
        let dir = temp_dir("spec");
        let base = spec_in(&dir);
        let mut odd = base.clone();
        odd.width = 5;
        assert!(validate_spec(&odd).is_err(), "odd width must fail (yuv420p)");
        let mut zero_fps = base.clone();
        zero_fps.fps = 0;
        assert!(validate_spec(&zero_fps).is_err());
        let mut zero_frames = base.clone();
        zero_frames.expected_frames = 0;
        assert!(validate_spec(&zero_frames).is_err());
        let mut missing_mix = base.clone();
        missing_mix.audio = AudioSpec::Mix {
            path: dir.join("no-such-mix.wav"),
            start_secs: 0.0,
            duration_secs: 1.0,
        };
        let e = validate_spec(&missing_mix).unwrap_err();
        assert!(e.to_string().contains("does not exist"), "got: {e}");
        assert!(validate_spec(&base).is_ok());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn gapped_plan_is_rejected_before_any_sink_is_created() {
        let dir = temp_dir("gapped");
        let spec = spec_in(&dir);
        // A scheduler plan with frame 2 "already done" — the
        // resume shape that must never reach a stream.
        let gapped = vec![0, 1, 3, 4];
        let sink_made = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&sink_made);
        let err = run_session_inner(
            &spec,
            fake_producer(&spec, None, None),
            &gapped,
            move |_s: &StreamSpec| -> TaleResult<FakeSink> {
                flag.store(true, Ordering::SeqCst);
                Ok(FakeSink::new(Arc::new(AtomicBool::new(false))))
            },
        )
        .unwrap_err();
        assert!(!sink_made.load(Ordering::SeqCst), "no sink may be created for a gapped plan");
        assert!(err.to_string().contains("plan"), "got: {err}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    // ----- session core over fakes -----

    #[test]
    fn session_happy_path_accounts_every_frame() {
        let dir = temp_dir("happy");
        let spec = spec_in(&dir);
        let plan = full_plan(&spec);
        let finish_called = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&finish_called);
        let core = run_session_inner(
            &spec,
            fake_producer(&spec, None, None),
            &plan,
            move |_s: &StreamSpec| -> TaleResult<FakeSink> {
                Ok(FakeSink::new(flag))
            },
        )
        .expect("clean session");
        assert_eq!(core.frames_produced, 5);
        assert_eq!(core.frames_encoded, 5);
        assert_eq!(core.frames_expected, 5);
        assert_eq!(core.bytes_streamed, 5 * 32);
        assert!(finish_called.load(Ordering::SeqCst));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn session_producer_error_aborts_without_finish() {
        let dir = temp_dir("producer-error");
        let spec = spec_in(&dir);
        let plan = full_plan(&spec);
        let finish_called = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&finish_called);
        let err = run_session_inner(
            &spec,
            fake_producer(&spec, Some(2), None),
            &plan,
            move |_s: &StreamSpec| -> TaleResult<FakeSink> {
                Ok(FakeSink::new(flag))
            },
        )
        .unwrap_err();
        // The PRODUCER's error is the one returned...
        assert!(
            err.to_string().contains("fake producer failure at frame 2"),
            "got: {err}"
        );
        // ...and the sink was dropped without finish (for the
        // real sink, Drop is what kills ffmpeg).
        assert!(!finish_called.load(Ordering::SeqCst), "finish must not run on the abort path");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn session_wrong_length_frame_is_rejected_at_the_producer() {
        let dir = temp_dir("short-frame");
        let spec = spec_in(&dir);
        let plan = full_plan(&spec);
        let finish_called = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&finish_called);
        let err = run_session_inner(
            &spec,
            fake_producer(&spec, None, Some(1)),
            &plan,
            move |_s: &StreamSpec| -> TaleResult<FakeSink> {
                Ok(FakeSink::new(flag))
            },
        )
        .unwrap_err();
        assert!(err.to_string().contains("frame 1"), "got: {err}");
        assert!(err.to_string().contains("31 bytes"), "got: {err}");
        assert!(!finish_called.load(Ordering::SeqCst));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn session_sink_write_error_propagates() {
        let dir = temp_dir("sink-error");
        let spec = spec_in(&dir);
        let plan = full_plan(&spec);
        let finish_called = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&finish_called);
        let err = run_session_inner(
            &spec,
            fake_producer(&spec, None, None),
            &plan,
            move |_s: &StreamSpec| -> TaleResult<FakeSink> {
                let mut sink = FakeSink::new(flag);
                sink.fail_on_call = Some(1);
                Ok(sink)
            },
        )
        .unwrap_err();
        assert!(
            err.to_string().contains("injected write failure"),
            "got: {err}"
        );
        assert!(!finish_called.load(Ordering::SeqCst));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn session_reconciles_a_sink_that_underreports_its_finish_count() {
        let dir = temp_dir("finish-mismatch");
        let spec = spec_in(&dir);
        let plan = full_plan(&spec);
        // The fake's finish CLAIMS one frame fewer than it was
        // fed: drain and stats agree, the finish count does
        // not — the session's own reconciliation must catch
        // what the sink's finish did not.
        let err = run_session_inner(
            &spec,
            fake_producer(&spec, None, None),
            &plan,
            move |_s: &StreamSpec| -> TaleResult<FakeSink> {
                let mut sink = FakeSink::new(Arc::new(AtomicBool::new(false)));
                sink.finish_count_delta = -1;
                Ok(sink)
            },
        )
        .unwrap_err();
        assert!(err.to_string().contains("frame-count mismatch"), "got: {err}");
        assert!(err.to_string().contains("encoded 4"), "got: {err}");
        assert!(err.to_string().contains("expected 5"), "got: {err}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn session_stale_part_is_discarded_and_replaced() {
        // A leftover `.part` from an interrupted run is
        // recovery's DiscardPartial: the session removes it
        // before starting (here the fake sink writes nothing,
        // so after the session the stale bytes must be GONE,
        // not silently kept).
        let dir = temp_dir("stale-part");
        let spec = spec_in(&dir);
        let plan = spec.plan().unwrap();
        std::fs::write(&plan.out_part, b"stale partial").unwrap();
        let core = run_session_inner(
            &spec,
            fake_producer(&spec, None, None),
            &full_plan(&spec),
            move |_s: &StreamSpec| -> TaleResult<FakeSink> {
                Ok(FakeSink::new(Arc::new(AtomicBool::new(false))))
            },
        )
        .expect("session over a stale part");
        assert_eq!(core.frames_encoded, 5);
        assert!(
            !Path::new(&plan.out_part).exists(),
            "the stale .part must have been discarded, not kept"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    // ----- probe parsing / validation (pure) -----

    fn probe_json(frames: &str, duration: &str, colour: bool) -> String {
        let colour_fields = if colour {
            "\"color_range\":\"tv\",\"color_space\":\"bt709\",\
             \"color_transfer\":\"bt709\",\"color_primaries\":\"bt709\","
        } else {
            ""
        };
        format!(
            "{{\"streams\":[\
               {{\"codec_name\":\"h264\",\"codec_type\":\"video\",\"width\":4,\"height\":2,\
                 \"avg_frame_rate\":\"30/1\",{colour_fields}\
                 \"nb_frames\":\"{frames}\",\"nb_read_frames\":\"{frames}\"}},\
               {{\"codec_name\":\"aac\",\"codec_type\":\"audio\",\"nb_read_frames\":\"9\"}}],\
             \"format\":{{\"duration\":\"{duration}\"}}}}"
        )
    }

    #[test]
    fn probe_json_parses_the_evidence_shape() {
        let p = parse_probe_json(&probe_json("5", "0.200000", true)).expect("parse");
        assert_eq!(p.video_frames, 5);
        assert_eq!((p.width, p.height), (4, 2));
        assert_eq!(p.avg_fps, 30.0);
        assert_eq!(p.video_codec, "h264");
        assert_eq!(p.audio_codec.as_deref(), Some("aac"));
        assert_eq!(p.duration_secs, 0.2);
        assert_eq!(p.color_primaries.as_deref(), Some("bt709"));
        assert_eq!(p.color_space.as_deref(), Some("bt709"));
    }

    #[test]
    fn validate_probe_accepts_a_conforming_master() {
        let dir = temp_dir("probe-ok");
        let spec = spec_in(&dir);
        let p = parse_probe_json(&probe_json("5", "0.200000", true)).unwrap();
        assert!(validate_probe(&spec, &p).is_ok());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn validate_probe_rejects_each_contract_breach() {
        let dir = temp_dir("probe-bad");
        let spec = spec_in(&dir);
        // Counted frames short.
        let p = parse_probe_json(&probe_json("4", "0.200000", true)).unwrap();
        let e = validate_probe(&spec, &p).unwrap_err();
        assert!(e.to_string().contains("counted video frames 4"), "got: {e}");
        // Master without the colour triple (finding F1's
        // shape) must fail — the session does not promote a
        // master whose BT.709 did not land.
        let p = parse_probe_json(&probe_json("5", "0.200000", false)).unwrap();
        let e = validate_probe(&spec, &p).unwrap_err();
        assert!(e.to_string().contains("color_primaries"), "got: {e}");
        // Duration far from the video's (audio not trimmed /
        // wrong file).
        let p = parse_probe_json(&probe_json("5", "9.000000", true)).unwrap();
        let e = validate_probe(&spec, &p).unwrap_err();
        assert!(e.to_string().contains("duration"), "got: {e}");
        // The same colour-less file is FINE as a Draft (the
        // adapter's draft carries no colour triple).
        let mut draft = spec.clone();
        draft.profile = EncodeProfile::Draft;
        let p = parse_probe_json(&probe_json("5", "0.200000", false)).unwrap();
        assert!(validate_probe(&draft, &p).is_ok());
        let _ = std::fs::remove_dir_all(&dir);
    }

    // ----- hermetic real-ffmpeg smoke -----

    #[test]
    fn real_ffmpeg_smoke_streams_a_tiny_film() {
        if !crate::ffstream::ffmpeg_available() || !ffprobe_available() {
            // Host-dependent, the ffstream convention: the
            // VALUE of availability depends on the machine;
            // where both tools exist (this VM, CI) this runs.
            eprintln!("smoke skipped: ffmpeg/ffprobe not on PATH");
            return;
        }
        let dir = temp_dir("smoke");
        let spec = StreamSpec {
            out_mp4: dir.join("smoke.mp4"),
            width: 64,
            height: 64,
            fps: 30,
            first_index: 0,
            expected_frames: 6,
            profile: EncodeProfile::Master,
            audio: AudioSpec::Silent,
            queue_capacity: 2,
        };
        let len = (spec.width * spec.height * 4) as usize;
        let report = run_session(&spec, move |index: u32| -> TaleResult<Vec<u8>> {
            // A per-frame solid colour that changes with the
            // index (a frozen single frame would also pass the
            // count checks; motion is not what is under test,
            // but distinct frames keep the encode honest).
            let mut px = vec![0u8; len];
            for (i, chunk) in px.chunks_exact_mut(4).enumerate() {
                chunk[0] = (index as u8).wrapping_mul(37);
                chunk[1] = (i % 251) as u8;
                chunk[2] = 128;
                chunk[3] = 255;
            }
            Ok(px)
        })
        .expect("real streaming session");
        assert_eq!(report.frames_produced, 6);
        assert_eq!(report.frames_encoded, 6);
        assert_eq!(report.probe.video_frames, 6);
        assert_eq!((report.probe.width, report.probe.height), (64, 64));
        assert!(report.out_final.is_file(), "renamed final mp4 exists");
        assert!(
            !dir.join("smoke.mp4.part.mp4").exists(),
            "no .part remains after a successful session"
        );
        assert!(report.audio.contains("SILENT"), "report names the silent track: {}", report.audio);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
