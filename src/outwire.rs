//! outwire — CR-B1-09 (WP11 production caller): the adapter
//! between `crate::outputmgr` (WP11, not previously wired)
//! and the film driver's output naming in `src/film.rs`.
//!
//! =================================================================
//! DELIVERY NOTE — Chrome-Replacement Batch 1, Seat 9. NEW FILE
//! ONLY; no existing file was edited, no dependency was added
//! (std + `crate::outputmgr` only).
//! =================================================================
//!
//! EVERY OUTPUT NAME THE FILM PATH CONSTRUCTS — ENUMERATED
//! FROM THE CODE (HEAD b093f3a; grep over `format!` / string
//! construction in src/film.rs + the finishing script):
//!
//!   1. THE FRAME PNG — the ONLY output path `run_film`
//!      constructs inline (film.rs:3674):
//!          let path = format!("{out_dir}/frame_{n:05}.png");
//!      Used twice per rendered frame: the resume `exists()`
//!      test and `img.save(&path)` (whose error surfaces the
//!      path via `TaleError::image_io(&path, e)`). This is
//!      the case [`frame_path`] covers, byte-for-byte.
//!   2. There is NO other inline output construction in
//!      film.rs. Verified non-cases, so the lead does not go
//!      looking for them:
//!      - `format!("calendar-{y}.png")` (film.rs:2241) is an
//!        INPUT asset path under `film/assets/`, loaded by
//!        `load_png` — not an output. Out of this wire.
//!      - The `--scene-file` mode's `--out` PNG path and the
//!        film's `--out` directory are USER-SUPPLIED
//!        (src/main.rs argument parsing), never constructed.
//!      - All other `run_film` output is stdout lines, not
//!        file names.
//!   3. THE FINISHING SCRIPT (film/finish.sh, the mux step
//!      that consumes run_film's frames — outside film.rs but
//!      part of the film output contract):
//!      - input pattern `"$FRAMES/frame_%05d.png"` (line 25) —
//!        the shell spelling of the same canonical name;
//!        [`frame_name`] / [`parse_frame_name`] pin it.
//!      - `PART="${OUT}.part.mp4"` (line 22), renamed to the
//!        user-supplied `$OUT` on success. `outputmgr` has NO
//!        mp4 namer (its finalize report only recognises the
//!        `.part.mp4` suffix), so [`part_mp4_path`] pins the
//!        shell spelling as data and the script stays the
//!        owner. Byte-identical by construction + test.
//!
//! THE JOIN TRAP (why [`frame_path`] is a `format!`, not a
//! `Path::join`): outputmgr's own delivery note suggests the
//! lead rewrite film.rs:3674 as
//! `Path::new(out_dir).join(FrameNamer::frame_name(n))`.
//! That is NOT byte-identical when `out_dir` ends in `/`:
//! the inline form yields `frames//frame_00001.png`, `join`
//! yields `frames/frame_00001.png`. On Linux both resolve to
//! the same file, so renders and resume agree — but the path
//! STRING differs, and that string is observable: it is the
//! path carried by `TaleError::image_io` on a save failure.
//! This wire therefore keeps the inline `format!` shape
//! exactly (including the empty-`out_dir` case, which yields
//! `/frame_00001.png` — a leading slash the inline form has
//! today and `join` would not reproduce). Tests pin all of
//! these spellings against the literal inline expression.
//!
//! ADDITIVE-ONLY SURFACES (not part of the byte-parity wire):
//!   - [`existing_frames`] wraps `OutputScan::scan` as a
//!     resume-set helper. CAVEAT: `scan` ERRORS on a missing
//!     directory where the loop's per-frame `exists()` just
//!     returns false. `run_film` creates the directory
//!     (`create_dir_all`, film.rs:3659) BEFORE the loop, so a
//!     post-creation scan is equivalent; a pre-creation scan
//!     would introduce a new failure. The lead must call it
//!     only after `create_dir_all` — or not at all: the
//!     production resume path in schedwire needs no scan.
//!   - [`metadata`] builds an outputmgr `RenderMetadata` from
//!     run_film's counters. Writing it (`write_to`) would
//!     ADD a `render-metadata.txt` file the current run does
//!     not produce — a directory-content change (it would
//!     appear in `OutputScan`'s stray list and in any
//!     downstream listing). That is a behaviour ADDITION,
//!     not a parity wire: it is scoped OUT of the default
//!     integration in CR-B1-09-CALLERS.md §3.3 and left as a
//!     lead decision. The builder exists so the field
//!     mapping is written and tested either way.
//!   - [`finalize`] is pure reporting (outputmgr guarantees
//!     no mutation); safe to call anywhere after a run.
//!
//! WIRING THE LEAD MUST DO: in src/main.rs add
//! `mod outwire;` after `mod outputmgr;` (line 38 at
//! b093f3a). Call-site swap in CR-B1-09-CALLERS.md §3.2.

use crate::outputmgr::{FrameNamer, OutputScan, RenderMetadata};
use std::collections::HashSet;
use std::path::Path;

/// The canonical frame file NAME (no directory):
/// `frame_{n:05}.png`, via outputmgr's `FrameNamer` — the
/// strict-inverse-pinned spelling every producer and
/// consumer in the tree already uses.
pub(crate) fn frame_name(n: u32) -> String {
    FrameNamer::frame_name(n)
}

/// Strict inverse of [`frame_name`]: `Some(n)` iff `name`
/// is exactly the canonical spelling (see outputmgr's
/// delivery note for the rejection list).
pub(crate) fn parse_frame_name(name: &str) -> Option<u32> {
    FrameNamer::parse_frame_name(name)
}

/// The full frame path EXACTLY as film.rs:3674 constructs
/// it today: `format!("{out_dir}/frame_{n:05}.png")`. The
/// directory part is the caller's `out_dir` string verbatim
/// (trailing slash and empty string included — see the
/// JOIN TRAP note above); only the name part goes through
/// outputmgr's canonical namer.
pub(crate) fn frame_path(out_dir: &str, n: u32) -> String {
    format!("{out_dir}/{}", FrameNamer::frame_name(n))
}

/// film/finish.sh:22's in-flight MP4 name: `"${OUT}.part.mp4"`.
/// Pinned here as data; the shell script remains the owner
/// (outputmgr offers no mp4 namer).
pub(crate) fn part_mp4_path(out: &str) -> String {
    format!("{out}.part.mp4")
}

/// The set of frame indices present in `out_dir`, per
/// outputmgr's scan (canonical names only — a hand-renamed
/// `frame_1.png` is a stray, NOT frame 1; that matches the
/// loop, whose `exists()` tests the canonical path only).
///
/// Errors exactly when `OutputScan::scan` errors (missing /
/// unreadable directory) — see the ADDITIVE-ONLY note in the
/// module docs: call only after `create_dir_all(out_dir)`.
pub(crate) fn existing_frames(out_dir: &str) -> std::io::Result<HashSet<u32>> {
    Ok(OutputScan::scan(Path::new(out_dir))?
        .present
        .into_iter()
        .collect())
}

/// Build the outputmgr metadata record from a run_film run's
/// own values: `frames_rendered` is the loop's `done`,
/// `frames_skipped_existing` its `skipped`. Pure construction
/// — writing the record is a separate, scoped-out decision
/// (module docs). `renderer_version` is supplied by the
/// caller (`env!("CARGO_PKG_VERSION")` at the call site, as
/// outputmgr's note prescribes).
#[allow(clippy::too_many_arguments)]
pub(crate) fn metadata(
    renderer_version: &str,
    width: u32,
    height: u32,
    fps: u32,
    frames_rendered: u32,
    frames_skipped_existing: u32,
    adapter_line: &str,
    started_unix: u64,
    finished_unix: u64,
) -> RenderMetadata {
    RenderMetadata {
        renderer_version: renderer_version.to_string(),
        width,
        height,
        fps,
        frames_rendered,
        frames_skipped_existing,
        adapter_line: adapter_line.to_string(),
        started_unix,
        finished_unix,
    }
}

/// The outputmgr finalization checklist for `out_dir`, as
/// data. Pure reporting; never mutates the directory.
pub(crate) fn finalize(out_dir: &str, expected_frames: u32) -> Vec<(String, bool, String)> {
    crate::outputmgr::finalize_report(Path::new(out_dir), expected_frames)
}

// ---------------------------------------------------------------------
// Tests: byte-parity against the LITERAL current expressions.
// ---------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// The current inline construction, copied verbatim from
    /// film.rs:3674 — the parity reference.
    fn inline_frame_path(out_dir: &str, n: u32) -> String {
        format!("{out_dir}/frame_{n:05}.png")
    }

    #[test]
    fn frame_path_is_byte_identical_to_the_inline_form() {
        // The full range of existing cases: ordinary indices,
        // the padding boundary, natural widening past 5
        // digits, u32::MAX — crossed with every out_dir shape
        // the CLI can pass: plain, trailing-slash, absolute,
        // nested, and empty.
        let indices = [
            0u32, 1, 7, 42, 999, 1810, 1811, 99_999, 100_000, 123_456, 9_999_999,
            u32::MAX,
        ];
        let out_dirs = ["frames", "frames/", "/tmp/x", "a/b/c", "", "."];
        for out_dir in out_dirs {
            for n in indices {
                assert_eq!(
                    frame_path(out_dir, n),
                    inline_frame_path(out_dir, n),
                    "diverged for out_dir={out_dir:?} n={n}"
                );
            }
        }
    }

    #[test]
    fn frame_path_keeps_the_inline_quirks_join_would_lose() {
        // Trailing slash: the inline form doubles the slash;
        // Path::join would collapse it. The string is
        // observable (save-error path), so the quirk is pinned.
        assert_eq!(frame_path("frames/", 1), "frames//frame_00001.png");
        assert_ne!(
            frame_path("frames/", 1),
            Path::new("frames/")
                .join(frame_name(1))
                .to_string_lossy()
                .into_owned(),
            "join and inline must differ here — that is the trap"
        );
        // Empty out_dir: inline yields a leading slash.
        assert_eq!(frame_path("", 1), "/frame_00001.png");
    }

    #[test]
    fn frame_name_round_trips_and_rejects_noncanonical() {
        for n in [0u32, 1, 42, 1810, 99_999, 100_000, u32::MAX] {
            assert_eq!(parse_frame_name(&frame_name(n)), Some(n));
        }
        assert_eq!(parse_frame_name("frame_1.png"), None);
        assert_eq!(parse_frame_name("frame_000001.png"), None);
        assert_eq!(parse_frame_name("frame_00001.jpg"), None);
    }

    #[test]
    fn part_mp4_path_matches_finish_sh() {
        // film/finish.sh:22 — PART="${OUT}.part.mp4".
        assert_eq!(part_mp4_path("film.mp4"), "film.mp4.part.mp4");
        assert_eq!(
            part_mp4_path("/renders/onoda.mp4"),
            "/renders/onoda.mp4.part.mp4"
        );
    }

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "talerender-outwire-test-{}-{tag}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    #[test]
    fn existing_frames_agrees_with_the_loops_exists_tests() {
        let dir = temp_dir("existing");
        let out = dir.to_string_lossy().into_owned();
        for n in [0u32, 1, 3] {
            std::fs::write(frame_path(&out, n), b"x").expect("seed frame");
        }
        // A non-canonical namesake and an unrelated file: the
        // loop's canonical-path exists() ignores both, and so
        // must the scan-based set.
        std::fs::write(dir.join("frame_1.png"), b"x").expect("seed stray");
        std::fs::write(dir.join("notes.txt"), b"x").expect("seed stray");

        let existing = existing_frames(&out).expect("scan seeded dir");
        assert_eq!(existing, [0, 1, 3].into_iter().collect::<HashSet<u32>>());
        // Pointwise agreement with the loop's own test over a
        // window covering present, absent and stray-shadowed
        // indices.
        for n in 0..8u32 {
            assert_eq!(
                existing.contains(&n),
                Path::new(&inline_frame_path(&out, n)).exists(),
                "exists disagreement at frame {n}"
            );
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn existing_frames_errors_on_a_missing_dir_unlike_exists() {
        // The documented caveat, pinned: scan errors where the
        // loop's exists() would return false. Callers must
        // scan only after create_dir_all (film.rs:3659).
        let dir = std::env::temp_dir().join(format!(
            "talerender-outwire-test-{}-missing",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        let out = dir.to_string_lossy().into_owned();
        assert!(existing_frames(&out).is_err());
        assert!(!Path::new(&inline_frame_path(&out, 0)).exists());
    }

    #[test]
    fn metadata_maps_the_loop_counters_verbatim() {
        let meta = metadata("1.0.1", 1080, 1920, 30, 1800, 11, "adapter x", 10, 20);
        assert_eq!(meta.frames_rendered, 1800); // the loop's `done`
        assert_eq!(meta.frames_skipped_existing, 11); // the loop's `skipped`
        let lines = meta.to_lines();
        assert!(lines.contains(&"render-meta: frames_rendered=1800".to_string()));
        assert!(lines.contains(&"render-meta: frames_skipped_existing=11".to_string()));
        assert!(lines.contains(&"render-meta: width=1080".to_string()));
        assert!(lines.contains(&"render-meta: height=1920".to_string()));
        assert!(lines.contains(&"render-meta: fps=30".to_string()));
    }

    #[test]
    fn finalize_reports_four_rows_for_a_real_dir() {
        let dir = temp_dir("finalize");
        let out = dir.to_string_lossy().into_owned();
        std::fs::write(frame_path(&out, 0), b"x").expect("seed frame");
        let rows = finalize(&out, 1);
        assert_eq!(rows.len(), 4);
        assert_eq!(rows[0].0, "frames-complete");
        assert!(rows[0].1, "single seeded frame completes 0..1: {}", rows[0].2);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
