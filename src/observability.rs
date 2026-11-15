// ============================================================================
// DELIVERY NOTE — V2 Batch 1, Seat 9 (WP12: structured errors +
// observability). NEW FILE ONLY; no existing file was edited.
//
// Files created by this seat:
//   1. src/observability.rs (this file)
//   2. docs/integration/V2-ERROR-REFERENCE.md (the error +
//      observability reference, derived from src/errors.rs)
//
// Contract points implemented (V2-CONTRACTS.md §3, contract C6;
// master prompt §16):
//   - `event(name, fields)` prints exactly one line to stdout:
//     `event=<name> k=v k=v`, fields in the caller's order; an
//     empty fields slice prints just `event=<name>`.
//   - `format_event(name, fields) -> String` is the pure
//     line-builder `event()` prints; all formatting tests target
//     it, so the printed line and the tested line cannot drift.
//   - Sanitization: every value (and, defensively, every key and
//     the event name) has spaces, '\n', '\r' and '\t' replaced
//     with '_', so an event is always exactly one physical line
//     and every value is a single whitespace-free token
//     (contract C6: "Values never contain spaces").
//   - Event names are dotted lowercase, per C6's vocabulary:
//     render.start, adapter.selected, frame.done, mem.pressure,
//     output.finalized (C6 also names stream.open; no helper is
//     provided for it in this seat — callers use `event()`
//     directly until WP4's streaming landing wires it).
//   - Typed helpers, all built on `event()`:
//       startup_line(mode)            -> event=render.start mode=<mode>
//       adapter_line(name, backend, device_type, driver,
//                    is_software)     -> event=adapter.selected
//                                         name=.. backend=..
//                                         device_type=.. driver=..
//                                         software=true|false
//                                         (the software field is
//                                         ALWAYS present — a
//                                         software adapter is
//                                         always labelled, master
//                                         prompt §7 / contract C7)
//       frame_done_line(index, total) -> event=frame.done index=<i> total=<t>
//       pressure_line(level, accounted)
//                                     -> event=mem.pressure level=<level> accounted=<bytes>
//       finalized_line(frames, out)   -> event=output.finalized frames=<n> out=<path>
//   - errors::TaleError remains THE error type (C6); this module
//     adds no error variants and no new error type. New modules
//     keep returning errors::TaleResult<T>.
//
// Wiring the lead must do (this seat edits nothing existing):
//   - src/main.rs: add `mod observability;` among the module
//     declarations (lines 18-37 at 65e4ea2), alphabetically
//     between `mod math;` (line 28) and `mod paintbudget;`
//     (line 30) — i.e. after `mod locomotion;` / `mod math;`,
//     before `mod paintbudget;`. That single line is the only
//     change required for this module to compile and its tests
//     to run under CI.
//   - Suggested call sites (lead's choice, none required for
//     compilation; existing human-readable lines stay as they
//     are — these events are ADDITIVE, greppable companions,
//     not replacements):
//       * startup_line(<mode label>) at each mode dispatch in
//         main() once the mode is known.
//       * adapter_line(...) beside the existing
//         `adapter: {} | backend: ...` lines (main.rs:620,
//         ab1.rs:597, scenefile.rs:1180, gpucore adapter
//         selection), fed from wgpu::AdapterInfo fields; the
//         is_software argument is `info.device_type ==
//         wgpu::DeviceType::Cpu` (contract C7's definition).
//       * frame_done_line(index, total) in the film frame loop,
//         at whatever cadence the lead wants (per frame is the
//         literal reading; throttling is a call-site decision).
//       * pressure_line(level, accounted_bytes) where the WP1.3
//         MemPressure level is evaluated (C1 consumers).
//       * finalized_line(frames, out_path) after the output
//         finishing/rename step completes.
//   - Device-loss event (C7): `event=device.lost reason=<...>`
//     is printed by the lead-wired gpucore handler, not by this
//     module; callers can emit it as
//     observability::event("device.lost", &[("reason",
//     reason.to_string())]) if they prefer this formatter.
//
// Tests included (#[cfg(test)], pure formatting only — the
// printing wrappers are one println! over format_event):
//   empty-fields line, single/multi-field order preservation,
//   exact lines for all five helper vocabularies (including
//   software=true AND software=false), space/newline/CR/tab
//   sanitization in values, sanitization of a path value with
//   spaces, empty value rendering (`k=`), and dotted-name
//   pass-through. No test touches stdout, the GPU, or the
//   filesystem.
//
// Known limitations:
//   - Log-only: no return value feeds a pixel, a verdict, or a
//     gate (same law as runlog.rs); events go to stdout via
//     println!, errors keep going to stderr via the existing
//     `error: {e}` + exit_code() path in main.rs.
//   - Sanitization is intentionally minimal: only whitespace
//     characters are folded to '_'. '=' or other punctuation in
//     a value is preserved verbatim (values are diagnostic
//     labels/paths, not a parseable wire format); a caller that
//     needs machine parsing of a free-text value must pass a
//     pre-tokenised string.
//   - No structured (JSON) output, no log levels, no filtering,
//     no timestamps: C6 specifies the single-line key=value
//     shape only, and runlog.rs already owns run timing. Adding
//     any of those is a lead decision beyond this charter.
//   - frame_done_line takes (u32, u32) and pressure_line's
//     accounted count is a raw byte count (u64) named
//     `accounted`, matching the VRAM ledger's vocabulary
//     (gpucore MemSnapshot::accounted_bytes, contract C2); the
//     field name in the line is `accounted`.
// ============================================================================

//! Structured observability events — V2 WP12 (contract C6).
//!
//! One greppable line per event on stdout:
//! `event=<name> k=v k=v`. This is the machine-readable
//! companion to the house human-readable log lines (runlog.rs
//! owns time/process lines; the `adapter: ...` lines stay as
//! they are): a script can pull every adapter selection, every
//! memory-pressure transition, or the finalization record out
//! of a run log with a single `grep '^event='` and split on
//! spaces, because the format guarantees no value ever
//! contains whitespace.
//!
//! Errors are NOT routed through here: `errors::TaleError`
//! remains the single error type, printed by main.rs as
//! `error: {e}` on stderr with `exit_code()` (see
//! docs/integration/V2-ERROR-REFERENCE.md). This module is
//! log-only — nothing it prints feeds a pixel, a verdict, or
//! a gate, so the determinism law is untouched.
//!
//! std only, no new dependencies.

/// Fold every whitespace character that would break the
/// one-line, space-separated shape (space, newline, carriage
/// return, tab) to `'_'`. Applied to values, keys, and the
/// event name, so `format_event`'s output is a single line of
/// single-token fields no matter what a caller passes.
fn sanitize(text: &str) -> String {
    text.chars()
        .map(|c| match c {
            ' ' | '\n' | '\r' | '\t' => '_',
            _ => c,
        })
        .collect()
}

/// Build the event line without printing it:
/// `event=<name> k=v k=v`, fields in the given order, every
/// value sanitized (see `sanitize`). An empty `fields` slice
/// yields just `event=<name>`. This is the pure half of
/// `event()`; the tests pin its exact output.
pub(crate) fn format_event(name: &str, fields: &[(&str, String)]) -> String {
    let mut line = format!("event={}", sanitize(name));
    for (key, value) in fields {
        line.push(' ');
        line.push_str(&sanitize(key));
        line.push('=');
        line.push_str(&sanitize(value));
    }
    line
}

/// Print one structured event line to stdout:
/// `event=<name> k=v k=v`. Event names are dotted lowercase
/// (`render.start`, `adapter.selected`, `stream.open`,
/// `mem.pressure`, `output.finalized`, …). Values never
/// contain spaces — whitespace is substituted with
/// underscores by `format_event`.
pub(crate) fn event(name: &str, fields: &[(&str, String)]) {
    println!("{}", format_event(name, fields));
}

/// Renderer startup: `event=render.start mode=<mode>`.
/// `mode` is the caller's mode label (e.g. the CLI mode being
/// dispatched); any whitespace in it is folded to `_`.
pub(crate) fn startup_line(mode: &str) {
    event("render.start", &[("mode", mode.to_string())]);
}

/// Adapter selection:
/// `event=adapter.selected name=<name> backend=<backend> device_type=<device_type> driver=<driver> software=<true|false>`.
/// The `software` field is ALWAYS emitted (never omitted for
/// hardware adapters): a software adapter is always labelled
/// software in every report (master prompt §7, contract C7).
/// Callers pass `is_software = (adapter_info.device_type ==
/// wgpu::DeviceType::Cpu)`.
pub(crate) fn adapter_line(
    name: &str,
    backend: &str,
    device_type: &str,
    driver: &str,
    is_software: bool,
) {
    event(
        "adapter.selected",
        &[
            ("name", name.to_string()),
            ("backend", backend.to_string()),
            ("device_type", device_type.to_string()),
            ("driver", driver.to_string()),
            ("software", is_software.to_string()),
        ],
    );
}

/// One frame completed:
/// `event=frame.done index=<index> total=<total>`.
/// `index` is the frame index as the caller counts it and
/// `total` the run's frame total; this module does not
/// interpret either number.
pub(crate) fn frame_done_line(index: u32, total: u32) {
    event(
        "frame.done",
        &[
            ("index", index.to_string()),
            ("total", total.to_string()),
        ],
    );
}

/// Memory-pressure observation:
/// `event=mem.pressure level=<level> accounted=<bytes>`.
/// `level` is the caller's pressure-level label (contract C1's
/// `Normal` / `Elevated` / `Critical`, in whatever spelling
/// the caller uses — whitespace is folded); `accounted` is
/// the VRAM ledger's accounted byte count at observation
/// time (gpucore's `accounted_bytes`, contract C2).
pub(crate) fn pressure_line(level: &str, accounted: u64) {
    event(
        "mem.pressure",
        &[
            ("level", level.to_string()),
            ("accounted", accounted.to_string()),
        ],
    );
}

/// Output finalization:
/// `event=output.finalized frames=<frames> out=<out>`.
/// Emitted after the finishing step (mux + rename) completes;
/// `out` is the final output path (whitespace in the path is
/// folded to `_` like every other value).
pub(crate) fn finalized_line(frames: u32, out: &str) {
    event(
        "output.finalized",
        &[
            ("frames", frames.to_string()),
            ("out", out.to_string()),
        ],
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_fields_prints_bare_event() {
        assert_eq!(format_event("render.start", &[]), "event=render.start");
        assert_eq!(format_event("output.finalized", &[]), "event=output.finalized");
    }

    #[test]
    fn fields_keep_given_order() {
        let line = format_event(
            "stream.open",
            &[
                ("width", 1080.to_string()),
                ("height", 1920.to_string()),
                ("fps", 30.to_string()),
            ],
        );
        assert_eq!(line, "event=stream.open width=1080 height=1920 fps=30");
    }

    #[test]
    fn startup_vocabulary() {
        // The exact line startup_line("film") prints.
        assert_eq!(
            format_event("render.start", &[("mode", "film".to_string())]),
            "event=render.start mode=film"
        );
    }

    #[test]
    fn adapter_vocabulary_hardware_is_labelled_not_software() {
        // The exact line adapter_line(.., false) prints:
        // software=false is present, never omitted.
        let line = format_event(
            "adapter.selected",
            &[
                ("name", "AMD RADV VEGA".to_string()),
                ("backend", "Vulkan".to_string()),
                ("device_type", "IntegratedGpu".to_string()),
                ("driver", "radv".to_string()),
                ("software", false.to_string()),
            ],
        );
        // The adapter name's spaces are sanitized like any value.
        assert_eq!(
            line,
            "event=adapter.selected name=AMD_RADV_VEGA backend=Vulkan \
             device_type=IntegratedGpu driver=radv software=false"
        );
    }

    #[test]
    fn adapter_vocabulary_software_is_labelled_software() {
        // The exact line adapter_line(.., true) prints.
        let line = format_event(
            "adapter.selected",
            &[
                ("name", "llvmpipe".to_string()),
                ("backend", "Vulkan".to_string()),
                ("device_type", "Cpu".to_string()),
                ("driver", "llvmpipe".to_string()),
                ("software", true.to_string()),
            ],
        );
        assert_eq!(
            line,
            "event=adapter.selected name=llvmpipe backend=Vulkan \
             device_type=Cpu driver=llvmpipe software=true"
        );
    }

    #[test]
    fn frame_done_vocabulary() {
        // The exact line frame_done_line(1051, 1811) prints.
        assert_eq!(
            format_event(
                "frame.done",
                &[
                    ("index", 1051.to_string()),
                    ("total", 1811.to_string()),
                ]
            ),
            "event=frame.done index=1051 total=1811"
        );
    }

    #[test]
    fn pressure_vocabulary() {
        // The exact line pressure_line("elevated", 1_500_000_000) prints.
        assert_eq!(
            format_event(
                "mem.pressure",
                &[
                    ("level", "elevated".to_string()),
                    ("accounted", 1_500_000_000u64.to_string()),
                ]
            ),
            "event=mem.pressure level=elevated accounted=1500000000"
        );
    }

    #[test]
    fn finalized_vocabulary() {
        // The exact line finalized_line(1811, "out/film.mp4") prints.
        assert_eq!(
            format_event(
                "output.finalized",
                &[
                    ("frames", 1811.to_string()),
                    ("out", "out/film.mp4".to_string()),
                ]
            ),
            "event=output.finalized frames=1811 out=out/film.mp4"
        );
    }

    #[test]
    fn spaces_in_values_become_underscores() {
        assert_eq!(
            format_event("render.start", &[("mode", "scene file".to_string())]),
            "event=render.start mode=scene_file"
        );
    }

    #[test]
    fn newlines_and_tabs_in_values_become_underscores() {
        // A value can never split the event across lines or
        // fake a second field boundary.
        assert_eq!(
            format_event(
                "output.finalized",
                &[("out", "my renders/final\ncut\tv2.mp4".to_string())]
            ),
            "event=output.finalized out=my_renders/final_cut_v2.mp4"
        );
        assert_eq!(
            format_event("mem.pressure", &[("level", "crit\rical".to_string())]),
            "event=mem.pressure level=crit_rical"
        );
    }

    #[test]
    fn empty_value_renders_as_bare_key_equals() {
        assert_eq!(
            format_event("adapter.selected", &[("driver", String::new())]),
            "event=adapter.selected driver="
        );
    }

    #[test]
    fn result_is_always_one_line() {
        let line = format_event(
            "device.lost",
            &[("reason", "device lost\nwhile rendering frame 7".to_string())],
        );
        assert!(!line.contains('\n'), "got: {line}");
        assert_eq!(
            line,
            "event=device.lost reason=device_lost_while_rendering_frame_7"
        );
    }
}
