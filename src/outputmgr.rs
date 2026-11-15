//! outputmgr — WP11 render output + encoding completion:
//! canonical frame naming, output-directory scanning
//! (resume / gap analysis), render metadata, and the
//! finalization checklist.
//!
//! =================================================================
//! DELIVERY NOTE — V2 Batch 2, Seat 8 (WP11: render output and
//! encoding completion). NEW FILE ONLY; no existing file was
//! edited, no git command was run, no dependency was added
//! (std only).
//! =================================================================
//! FILES CREATED:
//!   - src/outputmgr.rs  (this file; the ONLY file this seat
//!     wrote)
//!
//! CONTRACT POINTS IMPLEMENTED (master prompt §15 — PNG output
//! compatibility, output directory management, safe temporary
//! files, render metadata, frame numbering, finalization
//! reporting; V2-CONTRACTS §1 ownership law + §5 Batch 2 seat 8):
//!   - `FrameNamer` — the ONE canonical spelling of a frame
//!     file name, plus its strict inverse parser (below).
//!   - `OutputScan` / `ScanReport` — one-pass directory scan:
//!     which frames are present (sorted), which indices in
//!     `0..=max_index` are missing, the max index, and every
//!     non-frame name as a stray. This is the resume / gap
//!     analysis the film loop's skip-if-exists behaviour and
//!     the WP9 project validator both need.
//!   - `RenderMetadata` — end-of-run bookkeeping (renderer
//!     version, geometry, fps, rendered / skipped counts,
//!     adapter line, start / finish unix seconds) rendered as
//!     stable `render-meta: key=value` lines and written to
//!     `render-metadata.txt` ATOMICALLY (`.part` + rename).
//!   - `finalize_report(dir, expected_frames)` — the
//!     finalization checklist AS DATA: rows of
//!     `(name, ok, detail)` covering frames-complete,
//!     metadata-present, no-`.part`-files, and mp4
//!     present-or-pending. Pure reporting: it reads the
//!     directory and never mutates anything.
//!   - The production finishing workflow is PRESERVED, not
//!     replaced (master prompt §15): x264 CRF 18 / preset slow,
//!     BT.709 tags, `mix.wav` muxed unchanged (AAC 256k),
//!     `+faststart`, and the `.part` discipline all stay in
//!     `film/finish.sh` / WP4's `ffstream.rs`. This module
//!     owns the PNG-side bookkeeping around that workflow;
//!     it spawns no encoder and muxes nothing.
//!
//! VERIFIED NAMING FORMAT (byte-for-byte, with source anchors):
//!   `frame_{n:05}.png` — Rust `format!("frame_{n:05}.png")`,
//!   i.e. the literal prefix `frame_`, the index zero-padded
//!   to a MINIMUM width of 5, the literal suffix `.png`.
//!   Anchors, all agreeing:
//!     - src/film.rs, `run_film` frame loop:
//!         let path = format!("{out_dir}/frame_{n:05}.png");
//!       (the PNG save path; the loop skips any frame whose
//!       path already exists — the resume behaviour `scan`
//!       analyses)
//!     - src/main.rs `--film` help: "Frames are written to
//!       --out as frame_%05d.png; existing frames are skipped
//!       (resume)."
//!     - film/finish.sh: `ffmpeg -framerate 30 -i
//!       "$FRAMES/frame_%05d.png"` (the mux-side spelling).
//!     - tools/ci/byte_probes.py: DEFAULT_PATTERN =
//!       "frame_{n:05d}.png" (Python spelling of the same).
//!     - tools/golden/make_golden.py: builds
//!       f"render-out/frames/frame_{mid:05d}.png".
//!   Indices >= 100000 WIDEN NATURALLY: `{:05}` is a minimum
//!   width, never a truncation, so frame 100000 is
//!   `frame_100000.png` (6 digits), frame 4294967295 is
//!   `frame_4294967295.png`. Frame indices are 0-based (the
//!   film loop renders `n in from..to`, `from` defaulting to
//!   0, `t = n / 30`).
//!
//! PARSE STRICTNESS (what `parse_frame_name` accepts):
//!   The codebase's PRODUCERS and exact-name consumers
//!   (film.rs, byte_probes.py, make_golden.py, finish.sh)
//!   only ever use the canonical spelling, so the parser is
//!   the strict inverse of `frame_name`: a name parses iff
//!   `frame_name(parsed) == name`. Consequences:
//!     - `frame_1.png`, `frame_0001.png` (too short) and
//!       `frame_000001.png` (non-canonical extra zero) are
//!       REJECTED — no tool in the tree produces or probes
//!       for them.
//!     - Wrong prefix / suffix / case, embedded paths
//!       (`frames/frame_00001.png` — pass a bare file name,
//!       not a path), signs, non-digits, empty digit runs and
//!       indices above `u32::MAX` are all rejected.
//!   The one LOOSE consumer in the tree is discovery by glob:
//!   tools/golden/run_golden.sh and tools/perf/lab/bench_lab.py
//!   find candidate directories with a `frame_*.png` glob,
//!   then address individual frames by the exact `%05d`
//!   spelling. `scan` follows the exact spelling: a
//!   loose-glob name that is not canonical (e.g. a
//!   hand-renamed `frame_1.png`) is reported in
//!   `stray_files`, never silently counted as a frame.
//!
//! WIRING THE LEAD MUST DO (this seat edits nothing existing):
//!   1. mod line — in src/main.rs, with the other mod decls
//!      (anchor: after `mod observability;`):
//!          mod outputmgr;
//!      That single line is the only change required for this
//!      module to compile and its tests to run under CI.
//!   2. Film loop naming (optional, byte-identical): in
//!      src/film.rs `run_film`, the line
//!          let path = format!("{out_dir}/frame_{n:05}.png");
//!      may become
//!          let path = Path::new(out_dir)
//!              .join(crate::outputmgr::FrameNamer::frame_name(n))
//!              .to_string_lossy().into_owned();
//!      or stay as-is — `frame_name` is pinned to the same
//!      bytes by unit test either way.
//!   3. Resume pre-scan (optional): `run_film`'s per-frame
//!      `Path::new(&path).exists()` skip is exactly membership
//!      in `OutputScan::scan(Path::new(out_dir)).present`;
//!      the lead may pre-scan once before the loop instead.
//!      Pure bookkeeping — no frame byte can change (contract
//!      A4: a frame is a pure function of its timestamp).
//!   4. Metadata write (suggested): at the end of `run_film`,
//!      beside the existing completion lines, build a
//!      `RenderMetadata` (renderer_version =
//!      `env!("CARGO_PKG_VERSION")`; width / height / fps from
//!      the run; frames_rendered / frames_skipped_existing
//!      from the loop's `done` / `skipped` counters;
//!      adapter_line from the same adapter identity the run
//!      selected; started_unix / finished_unix from
//!      `SystemTime::now()` at loop entry / exit) and call
//!      `write_to(Path::new(out_dir))`, mapping an io error to
//!      `TaleError::image_io(out_dir, e)` at the call site if
//!      the lead wants it on the TaleResult path.
//!   5. Finalization (suggested): after `run_film` returns
//!      (and in any future validate / finish driver), print or
//!      assert on `finalize_report(out_dir, frames_total)`
//!      rows; the WP9 validator
//!      (tools/integration/validate_native_project.py) mirrors
//!      `scan`'s rules in Python — keep the two in step via
//!      this note.
//!
//! TESTS INCLUDED (#[cfg(test)], std + temp dirs only, run by
//! CI at integration — this seat cannot compile locally):
//!   naming: exact spellings at 0 / 1 / 99999, natural widening
//!   at 100000+ and u32::MAX, round-trip over boundary indices;
//!   parse rejects (short, extra-zero, wrong affixes, path,
//!   overflow, junk); scan: gap + stray on a temp dir, empty
//!   dir, missing dir errors; metadata: exact `to_lines()`
//!   shape incl. adapter-line sanitization, atomic `write_to`
//!   (final present, no `.part` left, overwrite replaces);
//!   finalize: all-ok rows for a complete dir, all-fail rows
//!   for a gapped dir with a leftover `.part.mp4`, and a
//!   no-mutation proof (directory listing unchanged).
//!
//! KNOWN LIMITATIONS:
//!   - Not yet wired: no `mod` line, no callers. film.rs still
//!     formats its own path and finish.sh is untouched, by
//!     design (ownership law).
//!   - `scan` is NAME-based, one directory level, and does not
//!     inspect PNG contents: a zero-byte or truncated PNG
//!     counts as present. That is exactly today's resume
//!     semantics (the film loop skips on `exists()` after a
//!     direct-to-final `img.save`), so scan never disagrees
//!     with the loop it analyses; hardening frame writes
//!     themselves (save-to-`.part` + rename per frame) is a
//!     film.rs change for the lead, out of this charter.
//!   - `stray_files` is literal: EVERY non-frame entry is a
//!     stray, including `render-metadata.txt`, any `.mp4`,
//!     hidden files and subdirectories. Callers that only
//!     care about unknown junk filter the names they know.
//!   - `RenderMetadata` never reads a clock or the adapter
//!     itself: callers supply every field, keeping this
//!     module pure, deterministic and unit-testable. The
//!     metadata is run bookkeeping, never frame data, so the
//!     determinism law is untouched.
//!   - `write_to`'s rename is atomic on the same filesystem
//!     (the `.part` file is created in the target directory
//!     by construction, mirroring finish.sh's
//!     `PART="${OUT}.part.mp4"` + `mv`). No fsync is done:
//!     this is crash-consistency (no plausible half-written
//!     file under the final name), not power-loss durability
//!     — the same guarantee finish.sh gives the MP4.
//!   - `finalize_report`'s mp4 row checks PRESENCE only: it
//!     cannot tell whether an `.mp4` in the frames directory
//!     was muxed from these frames (probing is ffprobe /
//!     finish.sh / WP9 territory), and its frames row trusts
//!     names, not PNG validity (see the scan limitation).
//! =================================================================
//!
//! Pixel law: nothing in this module touches a rendered
//! pixel. It names, counts and reports files the renderer
//! already produced.

use std::path::Path;

// ---------------------------------------------------------------------
// Frame naming.
// ---------------------------------------------------------------------

/// The canonical frame file-name scheme (see the DELIVERY
/// NOTE for the verified format and its source anchors).
pub(crate) struct FrameNamer;

impl FrameNamer {
    /// The file name for frame `index`:
    /// `format!("frame_{index:05}.png")` — byte-for-byte the
    /// spelling `run_film` (src/film.rs) writes today.
    /// `{:05}` is a minimum width: indices >= 100000 widen
    /// naturally (`frame_100000.png`), never truncate.
    pub(crate) fn frame_name(index: u32) -> String {
        format!("frame_{index:05}.png")
    }

    /// Strict inverse of `frame_name`: `Some(index)` iff
    /// `name` is EXACTLY the canonical spelling of that index
    /// (bare file name, not a path). Rejects short forms
    /// (`frame_1.png`), extra leading zeros
    /// (`frame_000001.png`), wrong prefix / suffix / case,
    /// non-digit runs, and indices above `u32::MAX` — see the
    /// DELIVERY NOTE for why strictness matches the tree's
    /// own tooling.
    pub(crate) fn parse_frame_name(name: &str) -> Option<u32> {
        let digits = name.strip_prefix("frame_")?.strip_suffix(".png")?;
        if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        // Parse via u64 so an over-`u32` digit run is a clean
        // rejection, not a wrap or a panic.
        let value: u64 = digits.parse().ok()?;
        let index = u32::try_from(value).ok()?;
        // Canonical-form check: the only accepted spelling of
        // `index` is the one `frame_name` produces. This one
        // comparison rejects both too-short digit runs and
        // extra leading zeros.
        if Self::frame_name(index) == name {
            Some(index)
        } else {
            None
        }
    }
}

// ---------------------------------------------------------------------
// Output directory scan (resume / gap analysis).
// ---------------------------------------------------------------------

/// The result of scanning one output directory. All vectors
/// are sorted (indices ascending, stray names
/// lexicographically) so reports and tests are deterministic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ScanReport {
    /// Indices of canonical frame files present, ascending,
    /// deduplicated by construction (one directory entry can
    /// carry only one canonical name).
    pub present: Vec<u32>,
    /// Indices in `0..=max_index` that are NOT present,
    /// ascending — the gaps a resume run still has to render.
    /// Empty when no frames are present (no range exists to
    /// have gaps in) and when the range is complete.
    pub missing_in_range: Vec<u32>,
    /// The largest present index, or `None` for a frameless
    /// directory.
    pub max_index: Option<u32>,
    /// Names of every directory entry that is NOT a canonical
    /// frame file — other files, `.part` leftovers,
    /// subdirectories, non-canonical `frame_*.png` spellings —
    /// sorted lexicographically. See the DELIVERY NOTE: this
    /// list is literal, known companions such as
    /// `render-metadata.txt` appear here too.
    pub stray_files: Vec<String>,
}

/// Namespace for the output-directory scan.
pub(crate) struct OutputScan;

impl OutputScan {
    /// Scan `dir` once and classify every entry by name.
    ///
    /// An entry counts as a frame iff its file name parses
    /// via `FrameNamer::parse_frame_name` AND the entry is
    /// not a directory (a directory named `frame_00001.png`
    /// is a stray, not a frame). The directory check uses
    /// `DirEntry::file_type`, which does not follow symlinks:
    /// a symlink carrying a canonical name counts by name.
    /// If the entry's type cannot be read, the entry is
    /// classified by name alone.
    ///
    /// Errors: the `read_dir` / per-entry io errors are
    /// propagated verbatim (`std::io::Result`) — a missing or
    /// unreadable directory is an error, never an empty
    /// report, so a mistyped output path cannot masquerade
    /// as "no frames yet".
    pub(crate) fn scan(dir: &Path) -> std::io::Result<ScanReport> {
        let mut present: Vec<u32> = Vec::new();
        let mut stray_files: Vec<String> = Vec::new();
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().into_owned();
            let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
            match (is_dir, FrameNamer::parse_frame_name(&name)) {
                (false, Some(index)) => present.push(index),
                _ => stray_files.push(name),
            }
        }
        present.sort_unstable();
        stray_files.sort();
        let max_index = present.last().copied();
        let missing_in_range = match max_index {
            Some(max) => (0..=max)
                .filter(|i| present.binary_search(i).is_err())
                .collect(),
            None => Vec::new(),
        };
        Ok(ScanReport {
            present,
            missing_in_range,
            max_index,
            stray_files,
        })
    }
}

// ---------------------------------------------------------------------
// Render metadata.
// ---------------------------------------------------------------------

/// The metadata file's name inside the output directory.
pub(crate) const METADATA_FILENAME: &str = "render-metadata.txt";

/// The in-flight name `write_to` uses before the atomic
/// rename (the project's `.part` discipline: a reader only
/// ever sees the final name with complete contents, or no
/// file at all).
pub(crate) const METADATA_PART_FILENAME: &str = "render-metadata.txt.part";

/// End-of-run render bookkeeping. Every field is supplied by
/// the caller (this module reads no clock, no adapter and no
/// environment), so the struct is a plain record and
/// `to_lines` is a pure function of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RenderMetadata {
    /// Renderer version — the caller passes
    /// `env!("CARGO_PKG_VERSION")` ("1.0.1" at V2 launch).
    pub renderer_version: String,
    /// Frame width in pixels.
    pub width: u32,
    /// Frame height in pixels.
    pub height: u32,
    /// Frames per second the run rendered at (30 for the
    /// production film path).
    pub fps: u32,
    /// Frames actually rendered in this run (the film loop's
    /// `done` counter).
    pub frames_rendered: u32,
    /// Frames skipped because their PNG already existed —
    /// the resume count (the film loop's `skipped` counter).
    pub frames_skipped_existing: u32,
    /// The adapter identity line for the run, in whatever
    /// spelling the caller logged it (e.g. the `adapter: ...`
    /// line's payload, or observability's
    /// `event=adapter.selected` fields). Stored verbatim;
    /// whitespace is folded to `_` only in the rendered
    /// lines, per the C6 one-line discipline.
    pub adapter_line: String,
    /// Run start, seconds since the Unix epoch.
    pub started_unix: u64,
    /// Run finish, seconds since the Unix epoch.
    pub finished_unix: u64,
}

/// Fold the whitespace characters that would break the
/// one-line `key=value` shape (space, newline, carriage
/// return, tab) to `'_'` — the same minimal sanitization
/// observability.rs applies (contract C6: values never
/// contain spaces).
fn sanitize_value(text: &str) -> String {
    text.chars()
        .map(|c| match c {
            ' ' | '\n' | '\r' | '\t' => '_',
            _ => c,
        })
        .collect()
}

impl RenderMetadata {
    /// The stable line shape: one line per field, in struct
    /// declaration order, each exactly
    /// `render-meta: <key>=<value>` with the value sanitized
    /// (see `sanitize_value`). The order and spelling are a
    /// contract — scripts grep these lines — and are pinned
    /// by unit test.
    pub(crate) fn to_lines(&self) -> Vec<String> {
        let fields: [(&str, String); 9] = [
            ("renderer_version", self.renderer_version.clone()),
            ("width", self.width.to_string()),
            ("height", self.height.to_string()),
            ("fps", self.fps.to_string()),
            ("frames_rendered", self.frames_rendered.to_string()),
            (
                "frames_skipped_existing",
                self.frames_skipped_existing.to_string(),
            ),
            ("adapter_line", self.adapter_line.clone()),
            ("started_unix", self.started_unix.to_string()),
            ("finished_unix", self.finished_unix.to_string()),
        ];
        fields
            .iter()
            .map(|(key, value)| format!("render-meta: {key}={}", sanitize_value(value)))
            .collect()
    }

    /// Write the metadata into `dir` as
    /// `render-metadata.txt`, ATOMICALLY: the lines (joined
    /// with `\n`, one trailing newline) are written to
    /// `render-metadata.txt.part` in the same directory,
    /// then renamed over the final name. A concurrent or
    /// later reader sees either the previous complete file
    /// or the new complete file — never a half-written one
    /// under the final name. If the write itself fails, the
    /// error propagates and a `.part` file may remain, where
    /// `finalize_report`'s no-`.part` row will flag it.
    ///
    /// `dir` is created if missing (`create_dir_all`), like
    /// the film driver creates its output directory.
    pub(crate) fn write_to(&self, dir: &Path) -> std::io::Result<()> {
        std::fs::create_dir_all(dir)?;
        let part = dir.join(METADATA_PART_FILENAME);
        let contents = format!("{}\n", self.to_lines().join("\n"));
        std::fs::write(&part, contents)?;
        std::fs::rename(&part, dir.join(METADATA_FILENAME))?;
        Ok(())
    }
}

// ---------------------------------------------------------------------
// Finalization checklist (reporting only — never mutates).
// ---------------------------------------------------------------------

/// Summarise an index list for a one-line detail string:
/// every index when there are at most 8, otherwise the
/// first 8 plus the remaining count. Deterministic.
fn summarize_indices(indices: &[u32]) -> String {
    const SHOWN: usize = 8;
    let mut parts: Vec<String> = indices
        .iter()
        .take(SHOWN)
        .map(|i| i.to_string())
        .collect();
    if indices.len() > SHOWN {
        parts.push(format!("...+{} more", indices.len() - SHOWN));
    }
    format!("[{}]", parts.join(", "))
}

/// The finalization checklist for an output directory, as
/// data: rows of `(check name, ok, one-line detail)` in the
/// fixed order below. Pure reporting — this function reads
/// `dir` and never creates, writes, renames or deletes
/// anything.
///
/// Rows:
///   1. `frames-complete` — every index in
///      `0..expected_frames` is present (per `OutputScan`).
///      Extra frames beyond the expectation are noted in
///      the detail but do not fail the row; missing frames
///      do. `expected_frames == 0` never passes (there is
///      no expectation to be complete against).
///   2. `metadata-present` — `render-metadata.txt` exists
///      as a file in `dir`.
///   3. `no-part-files` — no entry name contains `.part`
///      (covers finish.sh's `<out>.part.mp4`, ffstream's
///      `.part.mp4` sink output, and this module's
///      `render-metadata.txt.part`): a leftover means some
///      finishing step started and did not rename.
///   4. `mp4` — a finished MP4 is present (a name ending in
///      `.mp4` that is not a `.part.mp4`). When only a
///      `.part.mp4` exists, or no MP4 exists yet, the row
///      fails with a `pending: ...` detail — on the pure-PNG
///      workflow the MP4 is made later by film/finish.sh,
///      so pending is a state, not an accusation.
///
/// An unreadable `dir` fails every row with the io error in
/// the detail instead of panicking or erroring: the caller
/// gets the whole checklist shape either way.
pub(crate) fn finalize_report(
    dir: &Path,
    expected_frames: u32,
) -> Vec<(String, bool, String)> {
    // One directory listing feeds rows 2–4; row 1 uses the
    // scan (which re-reads — acceptable for a reporting
    // call, and it keeps the scan the single source of the
    // frame rules).
    let (names, list_error): (Vec<String>, Option<String>) = match std::fs::read_dir(dir) {
        Ok(rd) => (
            rd.filter_map(|e| e.ok())
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .collect(),
            None,
        ),
        Err(e) => (Vec::new(), Some(e.to_string())),
    };

    // Row 1: frames complete.
    let frames_row = match OutputScan::scan(dir) {
        Ok(report) => {
            if expected_frames == 0 {
                (
                    "frames-complete".to_string(),
                    false,
                    "expected_frames=0: no expectation to check against".to_string(),
                )
            } else {
                let missing: Vec<u32> = (0..expected_frames)
                    .filter(|i| report.present.binary_search(i).is_err())
                    .collect();
                let extra = report
                    .present
                    .iter()
                    .filter(|&&i| i >= expected_frames)
                    .count();
                let in_range = report.present.len() - extra;
                if missing.is_empty() {
                    let detail = if extra > 0 {
                        format!(
                            "{in_range}/{expected_frames} frames present, \
                             plus {extra} beyond the expected range"
                        )
                    } else {
                        format!("{in_range}/{expected_frames} frames present, no gaps")
                    };
                    ("frames-complete".to_string(), true, detail)
                } else {
                    (
                        "frames-complete".to_string(),
                        false,
                        format!(
                            "{in_range}/{expected_frames} frames present, \
                             missing {}",
                            summarize_indices(&missing)
                        ),
                    )
                }
            }
        }
        Err(e) => (
            "frames-complete".to_string(),
            false,
            format!("scan failed: {e}"),
        ),
    };

    // Row 2: metadata present.
    let metadata_row = if dir.join(METADATA_FILENAME).is_file() {
        (
            "metadata-present".to_string(),
            true,
            format!("{METADATA_FILENAME} present"),
        )
    } else {
        (
            "metadata-present".to_string(),
            false,
            format!("{METADATA_FILENAME} not found"),
        )
    };

    // Row 3: no .part files left.
    let mut parts: Vec<&String> = names
        .iter()
        .filter(|n| n.contains(".part"))
        .collect();
    parts.sort();
    let part_row = if let Some(e) = &list_error {
        (
            "no-part-files".to_string(),
            false,
            format!("cannot list directory: {e}"),
        )
    } else if parts.is_empty() {
        (
            "no-part-files".to_string(),
            true,
            "no .part files".to_string(),
        )
    } else {
        let listed: Vec<&str> = parts.iter().take(8).map(|s| s.as_str()).collect();
        let suffix = if parts.len() > 8 {
            format!(", ...+{} more", parts.len() - 8)
        } else {
            String::new()
        };
        (
            "no-part-files".to_string(),
            false,
            format!("leftover .part files: {}{}", listed.join(", "), suffix),
        )
    };

    // Row 4: mp4 present-or-pending.
    let finished_mp4 = names
        .iter()
        .filter(|n| n.ends_with(".mp4") && !n.contains(".part"))
        .min();
    let part_mp4 = names.iter().filter(|n| n.ends_with(".part.mp4")).min();
    let mp4_row = match (finished_mp4, part_mp4) {
        (Some(name), _) => ("mp4".to_string(), true, format!("finished mp4 present: {name}")),
        (None, Some(name)) => (
            "mp4".to_string(),
            false,
            format!("pending: only {name} present (finish/rename has not run)"),
        ),
        (None, None) => (
            "mp4".to_string(),
            false,
            "pending: no mp4 yet (PNG workflow: mux with film/finish.sh)".to_string(),
        ),
    };

    vec![frames_row, metadata_row, part_row, mp4_row]
}

// ---------------------------------------------------------------------
// Tests — CPU + temp-dir only; CI runs them at integration.
// ---------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// House temp-dir pattern (assetstream.rs / scheduler.rs):
    /// a per-test subdirectory of the system temp dir, keyed
    /// by process id + tag, cleared before use; the caller
    /// removes it with `remove_dir_all` when done.
    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "talerender-outputmgr-test-{}-{tag}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    fn touch(dir: &Path, name: &str) {
        std::fs::write(dir.join(name), b"x").expect("write fixture file");
    }

    fn sample_metadata() -> RenderMetadata {
        RenderMetadata {
            renderer_version: "1.0.1".to_string(),
            width: 1080,
            height: 1920,
            fps: 30,
            frames_rendered: 1800,
            frames_skipped_existing: 11,
            adapter_line: "llvmpipe | backend: Vulkan".to_string(),
            started_unix: 1_731_000_000,
            finished_unix: 1_731_005_400,
        }
    }

    // ---- FrameNamer -------------------------------------------

    #[test]
    fn frame_name_matches_film_rs_spelling() {
        // The exact bytes src/film.rs's
        // format!("{out_dir}/frame_{n:05}.png") produces for
        // the name part.
        assert_eq!(FrameNamer::frame_name(0), "frame_00000.png");
        assert_eq!(FrameNamer::frame_name(1), "frame_00001.png");
        assert_eq!(FrameNamer::frame_name(42), "frame_00042.png");
        assert_eq!(FrameNamer::frame_name(1810), "frame_01810.png");
        assert_eq!(FrameNamer::frame_name(99999), "frame_99999.png");
    }

    #[test]
    fn frame_name_widens_naturally_past_five_digits() {
        // {:05} is a minimum width, never a truncation.
        assert_eq!(FrameNamer::frame_name(100_000), "frame_100000.png");
        assert_eq!(FrameNamer::frame_name(123_456), "frame_123456.png");
        assert_eq!(
            FrameNamer::frame_name(u32::MAX),
            "frame_4294967295.png"
        );
    }

    #[test]
    fn frame_name_round_trips_through_parse() {
        for index in [
            0u32, 1, 7, 42, 999, 1810, 99999, 100_000, 123_456, 9_999_999,
            u32::MAX,
        ] {
            let name = FrameNamer::frame_name(index);
            assert_eq!(
                FrameNamer::parse_frame_name(&name),
                Some(index),
                "round-trip failed for {name}"
            );
        }
    }

    #[test]
    fn parse_frame_name_rejects_noncanonical_spellings() {
        // Short / unpadded forms no tool in the tree produces.
        assert_eq!(FrameNamer::parse_frame_name("frame_1.png"), None);
        assert_eq!(FrameNamer::parse_frame_name("frame_0001.png"), None);
        // Non-canonical extra leading zero (index 1 in 6 digits).
        assert_eq!(FrameNamer::parse_frame_name("frame_000001.png"), None);
        // Wrong affixes, case, extension.
        assert_eq!(FrameNamer::parse_frame_name("frame_00001.PNG"), None);
        assert_eq!(FrameNamer::parse_frame_name("frame_00001.jpg"), None);
        assert_eq!(FrameNamer::parse_frame_name("Frame_00001.png"), None);
        assert_eq!(FrameNamer::parse_frame_name("frame_00001.png.bak"), None);
        assert_eq!(FrameNamer::parse_frame_name("frame_00001"), None);
        assert_eq!(FrameNamer::parse_frame_name("xframe_00001.png"), None);
        // Empty / non-digit / signed digit runs.
        assert_eq!(FrameNamer::parse_frame_name("frame_.png"), None);
        assert_eq!(FrameNamer::parse_frame_name("frame_abcde.png"), None);
        assert_eq!(FrameNamer::parse_frame_name("frame_0000a.png"), None);
        assert_eq!(FrameNamer::parse_frame_name("frame_-0001.png"), None);
        assert_eq!(FrameNamer::parse_frame_name(""), None);
        // A path is not a bare file name.
        assert_eq!(
            FrameNamer::parse_frame_name("frames/frame_00001.png"),
            None
        );
        // Above u32::MAX (4294967295): clean rejection, no wrap.
        assert_eq!(FrameNamer::parse_frame_name("frame_4294967296.png"), None);
        assert_eq!(
            FrameNamer::parse_frame_name("frame_99999999999999999999.png"),
            None
        );
    }

    // ---- OutputScan -------------------------------------------

    #[test]
    fn scan_reports_gap_stray_and_max() {
        let dir = temp_dir("scan-gap");
        touch(&dir, "frame_00000.png");
        touch(&dir, "frame_00001.png");
        touch(&dir, "frame_00003.png");
        touch(&dir, "notes.txt");
        // A non-canonical frame-ish name is a stray, not frame 1
        // by another spelling.
        touch(&dir, "frame_1.png");

        let report = OutputScan::scan(&dir).expect("scan temp dir");
        assert_eq!(report.present, vec![0, 1, 3]);
        assert_eq!(report.missing_in_range, vec![2]);
        assert_eq!(report.max_index, Some(3));
        assert_eq!(
            report.stray_files,
            vec!["frame_1.png".to_string(), "notes.txt".to_string()]
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn scan_of_empty_dir_is_empty_not_an_error() {
        let dir = temp_dir("scan-empty");
        let report = OutputScan::scan(&dir).expect("scan empty dir");
        assert_eq!(report.present, Vec::<u32>::new());
        assert_eq!(report.missing_in_range, Vec::<u32>::new());
        assert_eq!(report.max_index, None);
        assert_eq!(report.stray_files, Vec::<String>::new());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn scan_counts_leading_gap_from_zero() {
        // Frame indices are 0-based: a dir starting at frame 2
        // is missing 0 and 1, not "complete from its start".
        let dir = temp_dir("scan-leading-gap");
        touch(&dir, "frame_00002.png");
        let report = OutputScan::scan(&dir).expect("scan temp dir");
        assert_eq!(report.present, vec![2]);
        assert_eq!(report.missing_in_range, vec![0, 1]);
        assert_eq!(report.max_index, Some(2));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn scan_of_missing_dir_errors() {
        let dir = std::env::temp_dir().join(format!(
            "talerender-outputmgr-test-{}-does-not-exist",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        assert!(OutputScan::scan(&dir).is_err());
    }

    // ---- RenderMetadata ----------------------------------------

    #[test]
    fn metadata_lines_have_the_exact_stable_shape() {
        let lines = sample_metadata().to_lines();
        assert_eq!(
            lines,
            vec![
                "render-meta: renderer_version=1.0.1".to_string(),
                "render-meta: width=1080".to_string(),
                "render-meta: height=1920".to_string(),
                "render-meta: fps=30".to_string(),
                "render-meta: frames_rendered=1800".to_string(),
                "render-meta: frames_skipped_existing=11".to_string(),
                // The adapter line's spaces are folded to '_'
                // (C6: values never contain spaces).
                "render-meta: adapter_line=llvmpipe_|_backend:_Vulkan".to_string(),
                "render-meta: started_unix=1731000000".to_string(),
                "render-meta: finished_unix=1731005400".to_string(),
            ]
        );
    }

    #[test]
    fn metadata_write_to_is_atomic_and_complete() {
        let dir = temp_dir("meta-write");
        let meta = sample_metadata();
        meta.write_to(&dir).expect("write metadata");

        let final_path = dir.join(METADATA_FILENAME);
        assert!(final_path.is_file(), "final metadata file exists");
        assert!(
            !dir.join(METADATA_PART_FILENAME).exists(),
            "no .part file remains after the rename"
        );
        let on_disk = std::fs::read_to_string(&final_path).expect("read metadata");
        assert_eq!(on_disk, format!("{}\n", meta.to_lines().join("\n")));

        // A second write replaces the first under the same
        // final name (rename-over), still leaving no .part.
        let mut meta2 = sample_metadata();
        meta2.frames_rendered = 0;
        meta2.frames_skipped_existing = 1811;
        meta2.write_to(&dir).expect("rewrite metadata");
        let on_disk2 = std::fs::read_to_string(&final_path).expect("reread metadata");
        assert!(on_disk2.contains("frames_rendered=0"));
        assert!(on_disk2.contains("frames_skipped_existing=1811"));
        assert!(!dir.join(METADATA_PART_FILENAME).exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    // ---- finalize_report ----------------------------------------

    #[test]
    fn finalize_report_all_ok_for_a_complete_dir() {
        let dir = temp_dir("finalize-ok");
        touch(&dir, "frame_00000.png");
        touch(&dir, "frame_00001.png");
        touch(&dir, "frame_00002.png");
        sample_metadata().write_to(&dir).expect("write metadata");
        touch(&dir, "film.mp4");

        let rows = finalize_report(&dir, 3);
        assert_eq!(rows.len(), 4);
        assert_eq!(rows[0].0, "frames-complete");
        assert_eq!(rows[1].0, "metadata-present");
        assert_eq!(rows[2].0, "no-part-files");
        assert_eq!(rows[3].0, "mp4");
        for (name, ok, detail) in &rows {
            assert!(ok, "row {name} should pass, detail: {detail}");
        }
        assert!(rows[0].2.contains("3/3"));
        assert!(rows[3].2.contains("film.mp4"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn finalize_report_flags_a_gapped_unfinished_dir() {
        let dir = temp_dir("finalize-gapped");
        touch(&dir, "frame_00000.png");
        touch(&dir, "frame_00002.png"); // frame 1 missing
        // An interrupted finish: only the .part.mp4 exists.
        touch(&dir, "film.mp4.part.mp4");

        let rows = finalize_report(&dir, 3);
        assert_eq!(rows.len(), 4);
        for (name, ok, _) in &rows {
            assert!(!ok, "row {name} should fail for the gapped dir");
        }
        assert!(rows[0].2.contains("2/3"), "frames detail: {}", rows[0].2);
        assert!(rows[0].2.contains("[1]"), "frames detail: {}", rows[0].2);
        assert!(
            rows[1].2.contains("render-metadata.txt"),
            "metadata detail: {}",
            rows[1].2
        );
        assert!(
            rows[2].2.contains("film.mp4.part.mp4"),
            "part detail: {}",
            rows[2].2
        );
        assert!(
            rows[3].2.starts_with("pending:"),
            "mp4 detail: {}",
            rows[3].2
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn finalize_report_never_mutates_the_directory() {
        let dir = temp_dir("finalize-pure");
        touch(&dir, "frame_00000.png");
        touch(&dir, "film.mp4.part.mp4");
        let listing = |d: &Path| -> Vec<String> {
            let mut names: Vec<String> = std::fs::read_dir(d)
                .expect("list dir")
                .filter_map(|e| e.ok())
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .collect();
            names.sort();
            names
        };
        let before = listing(&dir);
        let _ = finalize_report(&dir, 2);
        assert_eq!(listing(&dir), before);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn finalize_report_on_missing_dir_fails_all_rows_without_panicking() {
        let dir = std::env::temp_dir().join(format!(
            "talerender-outputmgr-test-{}-finalize-missing",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        let rows = finalize_report(&dir, 10);
        assert_eq!(rows.len(), 4);
        for (name, ok, _) in &rows {
            assert!(!ok, "row {name} should fail for a missing dir");
        }
        assert!(rows[0].2.contains("scan failed"), "detail: {}", rows[0].2);
    }
}
