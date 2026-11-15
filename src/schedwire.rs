//! schedwire — CR-B1-09 (WP7 production caller): the adapter
//! between `crate::scheduler` (WP7, planning only) and the
//! frame loop in `crate::film::run_film`.
//!
//! =================================================================
//! DELIVERY NOTE — Chrome-Replacement Batch 1, Seat 9. NEW FILE
//! ONLY; no existing file was edited, no dependency was added
//! (std + `crate::scheduler` + `crate::outwire` only).
//! =================================================================
//!
//! THE CURRENT LOOP, VERIFIED IN CODE (src/film.rs, run_film,
//! HEAD b093f3a — line numbers at that HEAD):
//!   - Signature: `run_film(width, height, film_dir, out_dir,
//!     from, to, knobs, adapter_policy)` (film.rs:3646).
//!   - Range resolution, BEFORE the loop (film.rs:3657):
//!         let to = if to == 0 { frames_total }
//!                  else { to.min(frames_total) };
//!     `from` is NOT clamped. `frames_total =
//!     (data.duration * 30.0).ceil() as u32` (film.rs:3656).
//!   - The loop itself (film.rs:3673-3710):
//!         for n in from..to {
//!             let path = format!("{out_dir}/frame_{n:05}.png");
//!             if Path::new(&path).exists() { skipped += 1; continue; }
//!             ... render, img.save(&path)?; done += 1;
//!             if timing { println!("frame-time: ...") }
//!             if done % 25 == 0 { println!("  frame {n} ...") }
//!         }
//!     Load-bearing semantics this adapter preserves exactly:
//!     1. ASCENDING, HALF-OPEN: indices `from..to`, end
//!        exclusive; a reversed range (`from > to`) iterates
//!        zero times (a Rust range, never a panic).
//!     2. LAZY RESUME: the PNG-exists test runs per frame, at
//!        that frame's turn — NOT as a pre-scan. A frame file
//!        that appears (or disappears) mid-run is honoured at
//!        its own index and no other.
//!     3. COUNTERS: `skipped` increments on the skip itself;
//!        `done` increments only AFTER a successful save (a
//!        failed render/save aborts the run via `?`, so the
//!        counters are never observed mid-failure). The
//!        progress line fires iff `done % 25 == 0` after the
//!        increment, and its `timing_suffix` numerator is
//!        `(done + skipped)`, denominator `(to - from)`.
//!
//! WHAT THIS MODULE GIVES THE LEAD
//!   - [`resolve_range`]: film.rs:3657 as a pure function.
//!   - [`FrameIter`]: an iterator yielding [`Step::Skip`] /
//!     [`Step::Render`] in exactly the loop's order, with the
//!     exists test injected. [`fs_frame_iter`] wires the real
//!     filesystem test (path built by `crate::outwire::
//!     frame_path`, byte-identical to film.rs:3674).
//!   - [`FrameIter::note_rendered`]: the post-save counter
//!     step; returns true exactly when the current loop would
//!     print its every-25 progress line.
//!   - [`FrameIter::planned_render`] /
//!     [`FrameIter::planned_batches`]: the scheduler's pure
//!     `plan` / `batches` over a CALLER-SUPPLIED existing set.
//!     Under a static filesystem these equal the iterator's
//!     Render subsequence (proven by the in-file tests); the
//!     iterator — not the plan — is the production shape,
//!     because the plan snapshots its done set up front and
//!     the loop does not (semantics 2 above).
//!   - "Range bridging": two adjacent resolved ranges
//!     (`a..b` then `b..c`) driven back-to-back yield exactly
//!     the step sequence of the single range `a..c` under a
//!     static exists set — the property that makes a resumed
//!     `--from b` invocation continue a partial `--to b` run
//!     seamlessly. Proven by test. (No other "bridging"
//!     exists in the current code; grep-verified.)
//!
//! SCOPE-OUT — THE CHECKPOINT IS *NOT* WIRED (see
//! docs/integration/CR-B1-09-CALLERS.md §1.3 for the full
//! argument): `scheduler::plan_resume` / `load_checkpoint`
//! must NOT replace the PNG-exists resume source. A
//! checkpoint entry for a since-deleted PNG would skip that
//! frame forever; a present PNG with no checkpoint entry
//! would be re-rendered and overwritten. Both diverge from
//! the current loop. PNG existence stays the sole resume
//! source in this wire; the checkpoint helpers remain
//! available for a future, separately-approved semantics
//! change. Likewise `batches()` with `max_in_flight > 1`
//! must not be used to render in parallel in this wire:
//! the current loop is strictly sequential, and batching is
//! exposed here for planning/reporting only — flattening
//! the batches reproduces the plan exactly, so a sequential
//! consumer driving one batch at a time in order is
//! behaviour-identical.
//!
//! WIRING THE LEAD MUST DO (one mod line + the loop swap in
//! CR-B1-09-CALLERS.md §1.2): in src/main.rs add
//! `mod schedwire;` after `mod scheduler;` (line 43 at
//! b093f3a).

use crate::scheduler::{self, FrameRange, SchedOpts};
use std::collections::HashSet;
use std::path::Path;

/// film.rs:3657, verbatim as a pure function: a raw `to` of
/// 0 means "through the end of the film"; any other raw `to`
/// is clamped to `frames_total`. `from` passes through
/// unclamped, exactly as today (a `from` beyond the resolved
/// `to` simply yields an empty range).
pub(crate) fn resolve_range(from: u32, to_raw: u32, frames_total: u32) -> (u32, u32) {
    let to = if to_raw == 0 {
        frames_total
    } else {
        to_raw.min(frames_total)
    };
    (from, to)
}

/// One iteration of the film loop, as data: either the frame
/// is already on disk (the loop's `skipped += 1; continue`)
/// or it must be rendered and saved (the loop's render arm).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Step {
    Render(u32),
    Skip(u32),
}

impl Step {
    pub(crate) fn index(&self) -> u32 {
        match *self {
            Step::Render(n) | Step::Skip(n) => n,
        }
    }
}

/// The frame-range iterator `run_film`'s loop consumes.
///
/// The exists test is injected (`FnMut(u32) -> bool`) so the
/// production wiring and the tests share one implementation:
/// production passes the lazy per-frame filesystem test (see
/// [`fs_frame_iter`]); tests pass a set membership test. The
/// test runs at `next()` time for that index — the same
/// moment the current loop runs `Path::new(&path).exists()`.
pub(crate) struct FrameIter<F: FnMut(u32) -> bool> {
    from: u32,
    to: u32,
    next: u32,
    exists: F,
    done: u32,
    skipped: u32,
}

impl<F: FnMut(u32) -> bool> FrameIter<F> {
    /// An iterator over the RESOLVED range `from..to` (feed
    /// raw CLI values through [`resolve_range`] first).
    pub(crate) fn new(from: u32, to: u32, exists: F) -> Self {
        FrameIter {
            from,
            to,
            next: from,
            exists,
            done: 0,
            skipped: 0,
        }
    }

    /// The range as the scheduler's type, for plan calls.
    pub(crate) fn range(&self) -> FrameRange {
        FrameRange::new(self.from, self.to)
    }

    pub(crate) fn from(&self) -> u32 {
        self.from
    }

    pub(crate) fn to(&self) -> u32 {
        self.to
    }

    /// Frames rendered so far (the loop's `done`): incremented
    /// ONLY via [`FrameIter::note_rendered`], mirroring the
    /// loop's post-save increment.
    pub(crate) fn done(&self) -> u32 {
        self.done
    }

    /// Frames skipped so far (the loop's `skipped`):
    /// incremented when a [`Step::Skip`] is yielded.
    pub(crate) fn skipped(&self) -> u32 {
        self.skipped
    }

    /// The progress line's denominator, `(to - from)`. The
    /// current loop computes this with a plain subtraction
    /// inside the progress `println!` — a line that is only
    /// reachable after a successful render, which requires
    /// `from < to`, so the subtraction never underflows
    /// there. This accessor saturates instead; the values
    /// agree in every reachable state.
    pub(crate) fn total(&self) -> u32 {
        self.to.saturating_sub(self.from)
    }

    /// The post-save step: call after a [`Step::Render`]'s
    /// frame has been rendered AND saved successfully (the
    /// point where the current loop does `done += 1`).
    /// Returns true exactly when the loop would print its
    /// progress line (`done % 25 == 0` after the increment);
    /// the caller prints it with `n`, `done`, `skipped` and
    /// [`FrameIter::progress_numerator`] as today.
    pub(crate) fn note_rendered(&mut self) -> bool {
        self.done += 1;
        self.done % 25 == 0
    }

    /// The progress line's `timing_suffix` numerator:
    /// `(done + skipped)`, as film.rs:3705 computes it.
    pub(crate) fn progress_numerator(&self) -> u64 {
        (self.done + self.skipped) as u64
    }

    /// The scheduler's pure plan over a caller-supplied
    /// existing set: the Render subsequence this iterator
    /// would yield if the filesystem matched `existing` and
    /// did not change during the run (proven by test).
    pub(crate) fn planned_render(&self, existing: &HashSet<u32>) -> Vec<u32> {
        scheduler::plan(self.range(), existing)
    }

    /// The plan chunked by `opts.effective_max_in_flight()`
    /// (0 clamps to 1 inside the scheduler). Flattening the
    /// result reproduces [`FrameIter::planned_render`]
    /// exactly; a sequential consumer that renders batch by
    /// batch, in order, is behaviour-identical to the loop.
    pub(crate) fn planned_batches(
        &self,
        existing: &HashSet<u32>,
        opts: &SchedOpts,
    ) -> Vec<Vec<u32>> {
        scheduler::batches(&self.planned_render(existing), opts.effective_max_in_flight())
    }
}

impl<F: FnMut(u32) -> bool> Iterator for FrameIter<F> {
    type Item = Step;

    fn next(&mut self) -> Option<Step> {
        // `next >= to` also covers the reversed range
        // (`from > to`): `next` starts at `from`, already
        // past `to`, so the iterator is empty — exactly the
        // current `for n in from..to` behaviour.
        if self.next >= self.to {
            return None;
        }
        let n = self.next;
        self.next += 1;
        if (self.exists)(n) {
            self.skipped += 1;
            Some(Step::Skip(n))
        } else {
            Some(Step::Render(n))
        }
    }
}

/// The production iterator type: the exists test is a boxed
/// lazy filesystem check.
pub(crate) type FsFrameIter = FrameIter<Box<dyn FnMut(u32) -> bool>>;

/// Build the production iterator for `run_film`: for each
/// index, at that index's turn, test
/// `Path::new(&frame_path(out_dir, n)).exists()` — the
/// current loop's test, with the path spelled by
/// `crate::outwire::frame_path` (byte-identical to
/// film.rs:3674's inline `format!`, including for an
/// `out_dir` with a trailing slash; see outwire's note on
/// why `Path::join` must NOT be used here).
pub(crate) fn fs_frame_iter(from: u32, to: u32, out_dir: &str) -> FsFrameIter {
    let out_dir = out_dir.to_string();
    FrameIter::new(
        from,
        to,
        Box::new(move |n: u32| {
            Path::new(&crate::outwire::frame_path(&out_dir, n)).exists()
        }),
    )
}

// ---------------------------------------------------------------------
// Tests. The reference implementation below is a line-for-line
// transcription of film.rs:3673-3678's control flow (the render
// arm collapsed to a marker — it cannot affect the sequence).
// ---------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use std::path::PathBuf;

    fn set_of(indices: &[u32]) -> HashSet<u32> {
        indices.iter().copied().collect()
    }

    /// The current loop's step sequence, transcribed from
    /// film.rs (HEAD b093f3a):
    ///   for n in from..to {
    ///       if exists(frame_path(n)) { skipped; continue; }
    ///       render(n);
    ///   }
    fn current_loop_steps(from: u32, to: u32, existing: &HashSet<u32>) -> Vec<Step> {
        let mut steps = Vec::new();
        for n in from..to {
            if existing.contains(&n) {
                steps.push(Step::Skip(n));
            } else {
                steps.push(Step::Render(n));
            }
        }
        steps
    }

    fn wire_steps(from: u32, to: u32, existing: &HashSet<u32>) -> Vec<Step> {
        let existing = existing.clone();
        FrameIter::new(from, to, move |n| existing.contains(&n)).collect()
    }

    /// Representative (from, to, existing) cases: empty,
    /// reversed, single, all-cached, none-cached, sparse
    /// cache, mid-film start, the Onoda-scale full range.
    fn representative_cases() -> Vec<(u32, u32, HashSet<u32>)> {
        vec![
            (0, 0, set_of(&[])),
            (5, 3, set_of(&[4])), // reversed: zero iterations
            (0, 1, set_of(&[])),
            (0, 1, set_of(&[0])),
            (0, 6, set_of(&[1, 3, 4])),
            (10, 13, set_of(&[10, 11, 12])), // all cached
            (10, 13, set_of(&[])),           // none cached
            (10, 13, set_of(&[0, 11, 999])), // outside-range entries ignored
            (1810, 1811, set_of(&[])),       // last Onoda frame (1811 total)
            (0, 1811, set_of(&[0, 1, 2, 905, 1810])),
        ]
    }

    #[test]
    fn wire_sequence_equals_current_loop_for_representative_ranges() {
        for (from, to, existing) in representative_cases() {
            assert_eq!(
                wire_steps(from, to, &existing),
                current_loop_steps(from, to, &existing),
                "sequence diverged for range {from}..{to}"
            );
        }
    }

    #[test]
    fn plan_equals_the_iterators_render_subsequence() {
        for (from, to, existing) in representative_cases() {
            let existing_clone = existing.clone();
            let iter = FrameIter::new(from, to, move |n| existing_clone.contains(&n));
            let planned = iter.planned_render(&existing);
            let rendered: Vec<u32> = wire_steps(from, to, &existing)
                .iter()
                .filter_map(|s| match s {
                    Step::Render(n) => Some(*n),
                    Step::Skip(_) => None,
                })
                .collect();
            assert_eq!(planned, rendered, "plan diverged for {from}..{to}");
            // And the Skip subsequence is exactly the cached
            // indices inside the range, ascending.
            let skipped: Vec<u32> = wire_steps(from, to, &existing)
                .iter()
                .filter_map(|s| match s {
                    Step::Skip(n) => Some(*n),
                    Step::Render(_) => None,
                })
                .collect();
            let mut expected_skips: Vec<u32> =
                (from..to).filter(|n| existing.contains(n)).collect();
            expected_skips.sort_unstable();
            assert_eq!(skipped, expected_skips, "skips diverged for {from}..{to}");
        }
    }

    #[test]
    fn batches_flatten_to_the_plan() {
        let existing = set_of(&[1, 3, 4]);
        let iter = FrameIter::new(0, 10, |_| false);
        for bound in [0usize, 1, 2, 3, 25, 1000] {
            let opts = SchedOpts::new(bound, None);
            let flat: Vec<u32> = iter
                .planned_batches(&existing, &opts)
                .iter()
                .flatten()
                .copied()
                .collect();
            assert_eq!(flat, iter.planned_render(&existing), "bound {bound}");
            // Every batch respects the clamped bound.
            let eff = opts.effective_max_in_flight();
            for batch in iter.planned_batches(&existing, &opts) {
                assert!(batch.len() <= eff, "bound {bound}: oversized batch");
            }
        }
    }

    #[test]
    fn resolve_range_matches_film_rs_3657() {
        // to == 0 → through the end.
        assert_eq!(resolve_range(0, 0, 1811), (0, 1811));
        assert_eq!(resolve_range(100, 0, 1811), (100, 1811));
        // to beyond the film clamps to frames_total.
        assert_eq!(resolve_range(0, 99_999, 1811), (0, 1811));
        // Ordinary window passes through.
        assert_eq!(resolve_range(100, 200, 1811), (100, 200));
        // from is NOT clamped: a from past the end stays,
        // and the resulting range iterates zero times.
        assert_eq!(resolve_range(2000, 0, 1811), (2000, 1811));
        assert_eq!(wire_steps(2000, 1811, &set_of(&[])), Vec::<Step>::new());
    }

    #[test]
    fn adjacent_ranges_bridge_to_the_single_range() {
        // A run stopped at --to 3 and resumed with --from 3
        // produces, concatenated, exactly the single 0..7
        // run's sequence (static filesystem).
        let existing = set_of(&[1, 5]);
        let mut joined = wire_steps(0, 3, &existing);
        joined.extend(wire_steps(3, 7, &existing));
        assert_eq!(joined, wire_steps(0, 7, &existing));
        assert_eq!(joined, current_loop_steps(0, 7, &existing));
    }

    #[test]
    fn counters_and_progress_cadence_match_the_loop() {
        // 60 uncached frames: progress is due after the 25th
        // and 50th SAVES — and only there. Skips never advance
        // the cadence (the loop checks `done`, not done+skipped).
        let mut iter = FrameIter::new(0, 60, |_| false);
        let mut due_at = Vec::new();
        while let Some(step) = iter.next() {
            if let Step::Render(_) = step {
                if iter.note_rendered() {
                    due_at.push(iter.done());
                }
            }
        }
        assert_eq!(due_at, vec![25, 50]);
        assert_eq!(iter.done(), 60);
        assert_eq!(iter.skipped(), 0);
        assert_eq!(iter.progress_numerator(), 60);
        assert_eq!(iter.total(), 60);

        // Interleaved skips: 3 skips among the first frames do
        // not shift the save-based cadence, but they do count
        // in the numerator the progress line prints.
        let existing = set_of(&[0, 1, 2]);
        let mut iter = FrameIter::new(0, 28, move |n| existing.contains(&n));
        let mut first_due = None;
        while let Some(step) = iter.next() {
            match step {
                Step::Skip(_) => {}
                Step::Render(_) => {
                    if iter.note_rendered() && first_due.is_none() {
                        first_due = Some((iter.done(), iter.progress_numerator()));
                    }
                }
            }
        }
        assert_eq!(first_due, Some((25, 28))); // 25 rendered + 3 skipped
        assert_eq!(iter.skipped(), 3);
    }

    #[test]
    fn exists_is_checked_lazily_at_each_frames_turn() {
        // The production semantics the plan cannot reproduce:
        // a frame file appearing MID-RUN is skipped when its
        // turn comes, because the test runs inside next().
        // Real-filesystem version: create frame 2's file after
        // frame 0 has been yielded; the lazy check must see it.
        let dir = temp_frames_dir();
        let out = dir.to_string_lossy().into_owned();
        let mut iter = fs_frame_iter(0, 4, &out);
        assert_eq!(iter.next(), Some(Step::Render(0)));
        std::fs::write(crate::outwire::frame_path(&out, 2), b"x")
            .expect("frame 2 appears mid-run");
        assert_eq!(iter.next(), Some(Step::Render(1)));
        assert_eq!(iter.next(), Some(Step::Skip(2)));
        assert_eq!(iter.next(), Some(Step::Render(3)));
        assert_eq!(iter.next(), None);
        assert_eq!(iter.skipped(), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn temp_frames_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "talerender-schedwire-test-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create temp frames dir");
        dir
    }

    #[test]
    fn fs_iterator_matches_exists_loop_on_a_real_directory() {
        let dir = temp_frames_dir();
        let out = dir.to_string_lossy().into_owned();
        for n in [1u32, 3] {
            std::fs::write(crate::outwire::frame_path(&out, n), b"x")
                .expect("seed cached frame");
        }
        let steps: Vec<Step> = fs_frame_iter(0, 5, &out).collect();
        assert_eq!(
            steps,
            vec![
                Step::Render(0),
                Step::Skip(1),
                Step::Render(2),
                Step::Skip(3),
                Step::Render(4),
            ]
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
