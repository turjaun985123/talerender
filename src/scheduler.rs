/*
 * DELIVERY NOTE — TALERENDER V2, Batch 1, Seat 7: WP7 render scheduler
 * ==================================================================
 *
 * FILES CREATED
 *   src/scheduler.rs  (this file — the seat's only deliverable)
 *
 * CONTRACT POINTS IMPLEMENTED (V2-CONTRACTS.md §3 C5; master prompt §11)
 *   - FrameRange { start, end } (end exclusive) with len() /
 *     is_empty() / contains().
 *   - SchedOpts { max_in_flight, checkpoint_path }: the
 *     concurrency value is an INPUT to this module (its defaults
 *     come from the C1 memory profile — LowMemory -> 1 in
 *     flight); this module deliberately does NOT depend on
 *     src/mempressure.rs. max_in_flight == 0 clamps to 1
 *     (see SchedOpts::effective_max_in_flight and batches()).
 *   - Checkpoint: a text file, one completed frame index per
 *     line. load_checkpoint() -> CheckpointLoad { done,
 *     malformed }: a missing file is an empty set (not an
 *     error); malformed lines are ignored but counted.
 *     append_checkpoint(path, index) appends one line and
 *     flushes before returning.
 *   - plan(range, done) -> Vec<u32>: PURE. The remaining
 *     indices in strictly ascending order. Determinism law
 *     (C5): output order and bytes never depend on completion
 *     order — plan() iterates the range, never the set.
 *   - batches(plan, max_in_flight) -> Vec<Vec<u32>>: chunking
 *     helper bounding how many frames may be in flight at once
 *     (no unlimited parallel rendering, per §11).
 *   - plan_resume(range, opts): the C5-shaped convenience
 *     wrapper — loads opts.checkpoint_path (if any) and calls
 *     the pure plan(). C5 spells plan(range, opts); this seat's
 *     charter spells the pure plan(range, done: &HashSet<u32>),
 *     so both surfaces exist and the lead may wire either.
 *   - CancelFlag: cooperative cancellation over an AtomicBool
 *     (new / cancel / is_cancelled). The caller owns the flag
 *     (share it as Arc<CancelFlag>); the scheduler never
 *     cancels anything on its own.
 *   - SchedEvent { FrameDone(u32), RangeDone, Cancelled(u32) }:
 *     the progress vocabulary a consumer maps onto its own log
 *     lines. Cancelled(n) carries the index that was about to
 *     run (or was in flight) when cancellation was observed —
 *     i.e. the first index NOT completed.
 *   - The scheduler schedules; it does NOT render. film.rs's
 *     loop stays the only renderer in this wave (C5).
 *
 * WIRING THE LEAD MUST DO (no existing file is edited by this seat)
 *   1. src/main.rs: add `mod scheduler;` (alphabetical slot:
 *      between `mod runlog;` and `mod scene;`).
 *   2. src/film.rs, run_film() — the consumer's current shape is
 *      `for n in from..to { if frame PNG exists { skipped += 1;
 *      continue; } ... }` with a progress println! every 25
 *      rendered frames. Later wiring would:
 *        a. build FrameRange { start: from, end: to };
 *        b. build the done set from the checkpoint file via
 *           load_checkpoint() (and/or the existing PNG-exists
 *           scan, which stays film.rs's own resume source);
 *        c. iterate batches(&plan(range, &done),
 *           opts.effective_max_in_flight()), checking the
 *           CancelFlag between frames;
 *        d. after each frame: append_checkpoint() (when a
 *           checkpoint path is configured) and translate
 *           SchedEvent::FrameDone(n) into the existing
 *           every-25-frames progress line; RangeDone after the
 *           loop, Cancelled(n) on an early exit.
 *      None of that wiring is done here; it is the lead's
 *      landing step, and film.rs's output must not change when
 *      max_in_flight == 1 and no checkpoint is configured.
 *
 * TESTS (in-module, #[cfg(test)], std only)
 *   plan skips completed / plan empty when all done / plan
 *   ignores done indices outside the range / plan output is
 *   independent of the done set's insertion order / FrameRange
 *   len-is_empty-contains / checkpoint round-trip in a unique
 *   temp subdir (cleaned up) / missing checkpoint = empty /
 *   malformed lines counted / append creates the file /
 *   plan_resume honours the checkpoint file / batches chunking
 *   (incl. empty plan) / clamp behaviour (SchedOpts 0 -> 1,
 *   batches 0 -> singleton batches) / CancelFlag transitions.
 *
 * KNOWN LIMITATIONS
 *   - Scheduling only: no rendering, no threads, no queue —
 *     batches() hands the consumer bounded groups; how a group
 *     is executed (sequentially in this wave) is the consumer's
 *     choice, and output order is fixed by plan() regardless.
 *   - append_checkpoint() flushes but does not fsync; a power
 *     loss can still drop the last line(s). That is safe by
 *     construction: a lost line only causes a re-render of a
 *     frame whose bytes are a pure function of its index.
 *   - A checkpoint line is malformed iff, after trimming, it is
 *     non-empty and does not parse as a u32; duplicates are
 *     valid and de-duplicated by the set.
 *   - No new dependencies: std only.
 */

//! Render scheduler (WP7, contract C5).
//!
//! This module decides WHICH frames still need rendering, and
//! in what order — nothing more. It does not render: the film
//! loop in `film.rs` stays the only renderer, and a frame
//! remains a pure function of its index, so scheduling can
//! never change a frame's bytes.
//!
//! The pieces are deliberately small and mostly pure:
//!
//! * [`FrameRange`] — the half-open `[start, end)` index range.
//! * [`plan`] — the remaining indices, ascending, given the set
//!   already completed. Pure: the same (range, done) always
//!   yields the same plan, whatever order frames completed in.
//! * [`batches`] — the plan chunked so at most `max_in_flight`
//!   frames are ever in flight (bounded concurrency for
//!   low-memory machines; the bound itself comes from the
//!   caller's memory profile, contract C1, via [`SchedOpts`]).
//! * Checkpoint helpers — a text file with one completed index
//!   per line, so a long render can resume after an interrupt
//!   without holding any state in memory.
//! * [`CancelFlag`] + [`SchedEvent`] — cooperative cancellation
//!   and the progress vocabulary the consumer reports with.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

// ---------------------------------------------------------------------
// Frame range
// ---------------------------------------------------------------------

/// A half-open range of frame indices: `start..end`, with
/// `end` exclusive (the same convention as `film.rs`'s
/// `from..to` render loop). A range with `end <= start` is
/// empty.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct FrameRange {
    pub start: u32,
    pub end: u32,
}

impl FrameRange {
    pub(crate) fn new(start: u32, end: u32) -> Self {
        FrameRange { start, end }
    }

    /// Number of indices in the range; 0 for a reversed range
    /// (`end < start`), never an underflow.
    pub(crate) fn len(&self) -> u32 {
        self.end.saturating_sub(self.start)
    }

    /// True when the range covers no index (`end <= start`).
    pub(crate) fn is_empty(&self) -> bool {
        self.end <= self.start
    }

    /// True when `index` lies in `[start, end)`.
    pub(crate) fn contains(&self, index: u32) -> bool {
        self.start <= index && index < self.end
    }
}

// ---------------------------------------------------------------------
// Options
// ---------------------------------------------------------------------

/// Scheduler options, supplied by the caller.
///
/// `max_in_flight` is an input, not a decision this module
/// makes: its default comes from the memory-pressure profile
/// (contract C1 — LowMemory → 1 in flight, higher profiles
/// more). This module accepts the value as given and never
/// reads the mempressure module itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SchedOpts {
    /// Maximum frames the consumer may have in flight at once.
    /// **0 clamps to 1** — a scheduler that may run nothing
    /// would deadlock a consumer that trusts the value, so the
    /// clamp is applied everywhere the value is consumed
    /// ([`SchedOpts::effective_max_in_flight`], [`batches`]).
    pub max_in_flight: usize,
    /// Optional checkpoint file (one completed index per
    /// line). `None` disables checkpointing: nothing is read
    /// and the consumer writes nothing.
    pub checkpoint_path: Option<PathBuf>,
}

impl SchedOpts {
    pub(crate) fn new(max_in_flight: usize, checkpoint_path: Option<PathBuf>) -> Self {
        SchedOpts { max_in_flight, checkpoint_path }
    }

    /// The in-flight bound actually to use: `max_in_flight`
    /// with 0 clamped to 1.
    pub(crate) fn effective_max_in_flight(&self) -> usize {
        clamp_in_flight(self.max_in_flight)
    }
}

impl Default for SchedOpts {
    /// The safest bound: 1 frame in flight (the C1 LowMemory
    /// value), no checkpointing.
    fn default() -> Self {
        SchedOpts { max_in_flight: 1, checkpoint_path: None }
    }
}

/// The single clamp rule for in-flight bounds: 0 → 1.
fn clamp_in_flight(max_in_flight: usize) -> usize {
    max_in_flight.max(1)
}

// ---------------------------------------------------------------------
// Checkpoint file
// ---------------------------------------------------------------------

/// The result of reading a checkpoint file: the completed
/// indices, plus how many lines were ignored as malformed.
/// The count is reported (not silently swallowed) so a caller
/// can surface a damaged checkpoint in its log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CheckpointLoad {
    pub done: HashSet<u32>,
    pub malformed: usize,
}

/// Load a checkpoint file (text, one completed index per line).
///
/// * A **missing file is an empty checkpoint**, not an error —
///   a first run has no checkpoint yet. Other I/O errors
///   (permissions, a directory in the path's place, …) are
///   returned to the caller.
/// * Blank / whitespace-only lines are skipped, not counted.
/// * Any other line that does not parse as a `u32` is ignored
///   and counted in [`CheckpointLoad::malformed`].
/// * Duplicate indices are valid and de-duplicated by the set.
pub(crate) fn load_checkpoint(path: &Path) -> std::io::Result<CheckpointLoad> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(CheckpointLoad { done: HashSet::new(), malformed: 0 });
        }
        Err(e) => return Err(e),
    };
    let mut done = HashSet::new();
    let mut malformed = 0usize;
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        match line.parse::<u32>() {
            Ok(index) => {
                done.insert(index);
            }
            Err(_) => {
                malformed += 1;
            }
        }
    }
    Ok(CheckpointLoad { done, malformed })
}

/// Append one completed index to a checkpoint file, creating
/// the file if needed, and flush before returning so a reader
/// (or a resumed process) that opens the file afterwards sees
/// the line. The file's parent directory must already exist.
pub(crate) fn append_checkpoint(path: &Path, index: u32) -> std::io::Result<()> {
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    writeln!(file, "{index}")?;
    file.flush()
}

// ---------------------------------------------------------------------
// Planning
// ---------------------------------------------------------------------

/// The pure plan: the indices of `range` that are not in
/// `done`, in strictly ascending order.
///
/// Purity is the determinism law (C5) made concrete: the plan
/// depends only on (range, done) — never on the order frames
/// completed in, never on wall time — so completion order can
/// never affect output order. Indices in `done` outside the
/// range are ignored.
pub(crate) fn plan(range: FrameRange, done: &HashSet<u32>) -> Vec<u32> {
    (range.start..range.end)
        .filter(|index| !done.contains(index))
        .collect()
}

/// Chunk a plan into consecutive batches of at most
/// `max_in_flight` indices (0 clamps to 1, so every batch is
/// a singleton rather than an error or an infinite chunk).
/// Concatenating the batches reproduces the plan exactly —
/// batching bounds concurrency, it never reorders. An empty
/// plan yields no batches.
pub(crate) fn batches(plan: &[u32], max_in_flight: usize) -> Vec<Vec<u32>> {
    let size = clamp_in_flight(max_in_flight);
    plan.chunks(size).map(|chunk| chunk.to_vec()).collect()
}

/// The C5-shaped convenience wrapper: load the checkpoint
/// named by `opts` (an empty set when no path is configured)
/// and return the pure [`plan`] for `range`. Only this
/// function touches the filesystem; `plan` itself stays pure
/// and unit-testable without I/O.
pub(crate) fn plan_resume(range: FrameRange, opts: &SchedOpts) -> std::io::Result<Vec<u32>> {
    let done = match &opts.checkpoint_path {
        Some(path) => load_checkpoint(path)?.done,
        None => HashSet::new(),
    };
    Ok(plan(range, &done))
}

// ---------------------------------------------------------------------
// Cancellation + progress vocabulary
// ---------------------------------------------------------------------

/// Cooperative cancellation: a tiny wrapper over an
/// [`AtomicBool`]. The caller owns the flag — share it as
/// `Arc<CancelFlag>` between the code that requests the stop
/// (a signal handler, a UI, a supervisor) and the render loop,
/// which polls [`CancelFlag::is_cancelled`] between frames and
/// stops at a frame boundary. Nothing here forces a stop, and
/// a frame already in flight always finishes whole.
#[derive(Debug)]
pub(crate) struct CancelFlag {
    flag: AtomicBool,
}

impl CancelFlag {
    pub(crate) fn new() -> Self {
        CancelFlag { flag: AtomicBool::new(false) }
    }

    /// Request cancellation. Idempotent; once set it stays set
    /// for the life of the flag.
    pub(crate) fn cancel(&self) {
        self.flag.store(true, Ordering::SeqCst);
    }

    /// True once [`CancelFlag::cancel`] has been called.
    pub(crate) fn is_cancelled(&self) -> bool {
        self.flag.load(Ordering::SeqCst)
    }
}

impl Default for CancelFlag {
    fn default() -> Self {
        Self::new()
    }
}

/// Progress events, in the vocabulary of the scheduler. A
/// consumer translates these into its own reporting (for
/// `film.rs`, the existing progress / completion log lines);
/// the scheduler itself prints nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum SchedEvent {
    /// One frame finished rendering.
    FrameDone(u32),
    /// Every planned index finished: the range is complete.
    RangeDone,
    /// Cancellation was observed. The payload is the index
    /// that was about to run (or was in flight) at that
    /// moment — the first index NOT completed.
    Cancelled(u32),
}

// ---------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn done_of(indices: &[u32]) -> HashSet<u32> {
        indices.iter().copied().collect()
    }

    /// A unique subdirectory of the system temp dir for one
    /// test; the caller removes it with `remove_dir_all`.
    fn temp_subdir(tag: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let dir = std::env::temp_dir().join(format!(
            "talerender-scheduler-{tag}-{}-{nanos}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).expect("create temp subdir");
        dir
    }

    // -- FrameRange -------------------------------------------------

    #[test]
    fn range_len_is_empty_contains() {
        let r = FrameRange::new(3, 7);
        assert_eq!(r.len(), 4);
        assert!(!r.is_empty());
        assert!(r.contains(3));
        assert!(r.contains(6));
        assert!(!r.contains(7), "end is exclusive");
        assert!(!r.contains(2));

        let empty = FrameRange::new(5, 5);
        assert_eq!(empty.len(), 0);
        assert!(empty.is_empty());
        assert!(!empty.contains(5));

        let reversed = FrameRange { start: 9, end: 4 };
        assert_eq!(reversed.len(), 0, "reversed range must not underflow");
        assert!(reversed.is_empty());
    }

    // -- plan --------------------------------------------------------

    #[test]
    fn plan_skips_completed() {
        let range = FrameRange::new(0, 6);
        assert_eq!(plan(range, &done_of(&[1, 3, 4])), vec![0, 2, 5]);
    }

    #[test]
    fn plan_empty_when_all_done() {
        let range = FrameRange::new(10, 13);
        assert_eq!(plan(range, &done_of(&[10, 11, 12])), Vec::<u32>::new());
    }

    #[test]
    fn plan_empty_range_and_done_outside_range_ignored() {
        assert_eq!(
            plan(FrameRange::new(4, 4), &HashSet::new()),
            Vec::<u32>::new()
        );
        // Done indices below/above the range change nothing.
        let range = FrameRange::new(2, 5);
        assert_eq!(plan(range, &done_of(&[0, 1, 5, 900])), vec![2, 3, 4]);
    }

    #[test]
    fn plan_order_never_depends_on_completion_order() {
        // The same completed set, built by inserting in two
        // different orders, must yield the identical ascending
        // plan (determinism law, C5).
        let range = FrameRange::new(0, 8);
        let a = plan(range, &done_of(&[2, 5, 1]));
        let b = plan(range, &done_of(&[5, 1, 2]));
        assert_eq!(a, b);
        assert_eq!(a, vec![0, 3, 4, 6, 7]);
    }

    // -- checkpoint --------------------------------------------------

    #[test]
    fn checkpoint_round_trip_in_temp_dir() {
        let dir = temp_subdir("roundtrip");
        let path = dir.join("checkpoint.txt");

        // append_checkpoint creates the file on first use.
        append_checkpoint(&path, 3).expect("append 3");
        append_checkpoint(&path, 5).expect("append 5");
        append_checkpoint(&path, 9).expect("append 9");
        append_checkpoint(&path, 5).expect("append duplicate 5");

        let loaded = load_checkpoint(&path).expect("load checkpoint");
        std::fs::remove_dir_all(&dir).expect("clean up temp subdir");

        assert_eq!(loaded.done, done_of(&[3, 5, 9]), "duplicates de-duplicate");
        assert_eq!(loaded.malformed, 0);
    }

    #[test]
    fn missing_checkpoint_is_empty_not_an_error() {
        let dir = temp_subdir("missing");
        let path = dir.join("no-such-checkpoint.txt");
        let loaded = load_checkpoint(&path).expect("missing file loads as empty");
        std::fs::remove_dir_all(&dir).expect("clean up temp subdir");
        assert!(loaded.done.is_empty());
        assert_eq!(loaded.malformed, 0);
    }

    #[test]
    fn malformed_lines_are_ignored_but_counted() {
        let dir = temp_subdir("malformed");
        let path = dir.join("checkpoint.txt");
        // Valid: 10, 12 (whitespace-trimmed). Skipped silently:
        // the blank line. Malformed (4): "nope", "-3", "1.5",
        // and a number too large for u32.
        std::fs::write(&path, "10\n\nnope\n 12 \n-3\n1.5\n99999999999999999999\n")
            .expect("write checkpoint fixture");
        let loaded = load_checkpoint(&path).expect("load checkpoint");
        std::fs::remove_dir_all(&dir).expect("clean up temp subdir");

        assert_eq!(loaded.done, done_of(&[10, 12]));
        assert_eq!(loaded.malformed, 4);
    }

    #[test]
    fn plan_resume_honours_checkpoint_file() {
        let dir = temp_subdir("resume");
        let path = dir.join("checkpoint.txt");
        append_checkpoint(&path, 0).expect("append 0");
        append_checkpoint(&path, 2).expect("append 2");
        let opts = SchedOpts::new(2, Some(path));
        let planned = plan_resume(FrameRange::new(0, 4), &opts).expect("plan_resume");
        std::fs::remove_dir_all(&dir).expect("clean up temp subdir");
        assert_eq!(planned, vec![1, 3]);

        // No checkpoint path configured: the full range is planned.
        let bare = SchedOpts::new(2, None);
        assert_eq!(
            plan_resume(FrameRange::new(0, 4), &bare).expect("plan_resume bare"),
            vec![0, 1, 2, 3]
        );
    }

    // -- batches + clamp ----------------------------------------------

    #[test]
    fn batches_chunk_the_plan_in_order() {
        let p: Vec<u32> = (0..7).collect();
        let bs = batches(&p, 3);
        assert_eq!(bs, vec![vec![0, 1, 2], vec![3, 4, 5], vec![6]]);
        // Flattening the batches reproduces the plan exactly.
        let flat: Vec<u32> = bs.iter().flatten().copied().collect();
        assert_eq!(flat, p);
        // A bound larger than the plan yields a single batch.
        assert_eq!(batches(&p, 100), vec![p]);
        // An empty plan yields no batches.
        assert_eq!(batches(&[], 4), Vec::<Vec<u32>>::new());
    }

    #[test]
    fn max_in_flight_zero_clamps_to_one() {
        // Via the options accessor…
        let opts = SchedOpts { max_in_flight: 0, checkpoint_path: None };
        assert_eq!(opts.effective_max_in_flight(), 1);
        assert_eq!(SchedOpts::default().effective_max_in_flight(), 1);
        // …and via batches(): 0 gives singleton batches, never
        // an empty-chunk failure.
        assert_eq!(batches(&[1, 2, 3], 0), vec![vec![1], vec![2], vec![3]]);
    }

    // -- cancellation ---------------------------------------------------

    #[test]
    fn cancel_flag_starts_clear_and_sticks() {
        let flag = CancelFlag::new();
        assert!(!flag.is_cancelled());
        flag.cancel();
        assert!(flag.is_cancelled());
        flag.cancel();
        assert!(flag.is_cancelled(), "cancel is idempotent and sticky");
    }
}
