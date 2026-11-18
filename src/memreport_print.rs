// ============================================================================
// DELIVERY NOTE — Chrome-Replacement Wave, Batch 1, Seat 8
// (WP2: mem-report PRINT WIRING — glue half).
//
// Files created by this seat (NEW FILES ONLY; nothing existing
// was edited, no git was run, no dependency was added):
//   1. src/memreport_print.rs (this file)
//   2. src/dyntex.rs          (WP1.2 activation — separate note)
//   3. docs/integration/CR-B1-08-MEMWIRE.md (integration specs)
//
// What this module is: the thin formatting/glue layer between
// the three things that already exist and the six print sites
// that already print the pinned gpu-mem line:
//
//   * gpucore::mem_stats_line() — the PINNED line
//       gpu-mem: accounted_bytes=<n> peak_accounted_bytes=<n>
//       textures=<n> buffers=<n> readback_bytes=<n>
//       sysfs_gtt_used_bytes=<n|unavailable>
//       sysfs_vram_used_bytes=<n|unavailable>
//     (gpucore.rs format_mem_line; grammar parsed by
//     tools/perf/vram_report.py — DO NOT change it, and this
//     module never reformats it: it is passed through verbatim).
//   * gpucore::mem_snapshot() — contract C2's MemSnapshot.
//   * memreport::{render_lines, render_markdown,
//     gpu_timestamps_line} — WP2's renderers (V2 Batch 2 seat
//     2). They take memreport's LOCAL AdapterLine view;
//     adapter.rs owns the canonical AdapterReport (contract
//     C7) with the same five fields. The conversion the
//     memreport.rs delivery note anticipated ("add a one-line
//     From at the call site") lives here, once, instead of at
//     six call sites.
//
// Style/terminology law (matched to the existing sites): every
// render mode today ends with the same two-step tail — a
// `wrote <out> (<w>x<h>)` line, then, under the comment
// "VRAM ledger (W84 §5.3): its own line, after the wrote line",
//     println!("{}", crate::gpucore::mem_stats_line());
// The mem-report companions keep that shape exactly: plain
// println! lines, `mem-report:` / `gpu-timestamps:` prefixes
// (memreport.rs's stable grammar), printed IMMEDIATELY after
// the gpu-mem line, in this order:
//     gpu-mem: ...            (existing line, position untouched)
//     mem-report: ...         (render_lines block, 12–13 lines)
//     gpu-timestamps: ...     (one line)
// Nothing is interleaved, nothing existing moves, and film
// telemetry prints NONE of this (CPU-only mode; gpucore's
// ruling D1 — a structurally-zero ledger would be a
// fabricated-looking measurement).
//
// Wiring the lead must do (this seat edits nothing existing;
// the full per-site table with anchors is in
// docs/integration/CR-B1-08-MEMWIRE.md §1):
//   - src/main.rs: add `mod memreport_print;` immediately
//     after `mod memreport;` (line 36 at HEAD b093f3a).
//   - At each gpu-mem print site, EITHER keep the existing
//     println! and add one line after it:
//         crate::memreport_print::print_companion(
//             &report, adapter.features());
//     (report = adapter::describe(&info), which four of the
//     six sites already compute for their adapter: line), OR
//     replace the existing println! with the single call
//         crate::memreport_print::print_full_block(
//             &report, adapter.features());
//     which prints the gpu-mem line itself (via the same
//     mem_stats_line() call, byte-identical) followed by the
//     companions. Both forms are provided; print_full_block is
//     the recommended one (one call, order cannot drift).
//   - film.rs and scenefile.rs do NOT have an adapter in scope
//     at their print sites today (see the MD note): the lead
//     plumbs AdapterReport + Features there first. This module
//     deliberately takes (report, features) rather than an
//     &Adapter so those sites can pass stored values.
//
// Snapshot caveat (honest, pre-existing shape): mem_stats_line
// snapshots the ledger internally; mem_snapshot() snapshots it
// again microseconds later. At every print site the renderer
// is quiescent (no alloc/free between the two calls — only
// println!s), so the tracked figures agree; the sysfs fields
// are device-wide driver counters and can tick independently
// of this process — they already could between any two probes.
//
// Tests included (#[cfg(test)], pure — no GPU, no filesystem):
// adapter conversion field-by-field, companion/full-block
// ordering and verbatim gpu-mem pass-through, timestamps both
// ways through the glue, integrated-note and sysfs-unavailable
// propagation, Markdown delegation.
// ============================================================================

//! WP2 print wiring (glue): renders the gpucore ledger beside
//! the pinned `gpu-mem:` line at the render modes' tails.
//!
//! The pinned line itself is never reformatted here — it is
//! either produced by the caller exactly as today
//! (`gpucore::mem_stats_line()`) and passed through verbatim
//! (`full_block`), or printed by `print_full_block` via that
//! same call. Everything this module adds is additive
//! companions: the `mem-report:` block from
//! `crate::memreport::render_lines` and the one-line
//! `gpu-timestamps:` capability report. Log-only: nothing here
//! feeds a pixel, a verdict or a gate.

use crate::gpucore::MemSnapshot;

/// Convert the canonical C7 adapter report into memreport's
/// local view (identical five fields; see the delivery note).
/// `device_type` keeps the `{:?}` spelling memreport's
/// integrated-GPU detection compares against (`IntegratedGpu`),
/// because `adapter::describe` already stores exactly that
/// spelling.
pub(crate) fn adapter_line(report: &crate::adapter::AdapterReport) -> crate::memreport::AdapterLine {
    crate::memreport::AdapterLine {
        name: report.name.clone(),
        backend: report.backend.clone(),
        device_type: report.device_type.clone(),
        driver: report.driver.clone(),
        is_software: report.is_software,
    }
}

/// `adapter_line` straight from an adapter: describes it the
/// canonical C7 way first. For sites that hold the
/// `wgpu::Adapter` (or its info) but never built a report.
pub(crate) fn adapter_line_for_info(info: &wgpu::AdapterInfo) -> crate::memreport::AdapterLine {
    adapter_line(&crate::adapter::describe(info))
}

/// The companion lines for one print site, in print order:
/// the `mem-report:` block, then the `gpu-timestamps:` line.
/// Pure: same snapshot + report + features, same lines.
pub(crate) fn companion_lines(
    snap: &MemSnapshot,
    report: &crate::adapter::AdapterReport,
    features: wgpu::Features,
) -> Vec<String> {
    let line = adapter_line(report);
    let mut lines = crate::memreport::render_lines(snap, &line);
    lines.push(crate::memreport::gpu_timestamps_line(features));
    lines
}

/// The full tail block for one print site, in print order:
/// the caller's pinned `gpu-mem:` line VERBATIM (never parsed,
/// never reformatted), then `companion_lines`. Pure.
pub(crate) fn full_block(
    gpu_mem_line: &str,
    snap: &MemSnapshot,
    report: &crate::adapter::AdapterReport,
    features: wgpu::Features,
) -> Vec<String> {
    let mut lines = Vec::with_capacity(16);
    lines.push(gpu_mem_line.to_string());
    lines.extend(companion_lines(snap, report, features));
    lines
}

/// Print the companions beside an ALREADY-PRINTED gpu-mem
/// line: takes a fresh `mem_snapshot()` and prints
/// `companion_lines` in order. The lead's call site stays:
///     println!("{}", crate::gpucore::mem_stats_line());
///     crate::memreport_print::print_companion(&report, adapter.features());
pub(crate) fn print_companion(report: &crate::adapter::AdapterReport, features: wgpu::Features) {
    let snap = crate::gpucore::mem_snapshot();
    for line in companion_lines(&snap, report, features) {
        println!("{line}");
    }
}

/// Print the whole tail block as one call: the pinned gpu-mem
/// line (via the same `mem_stats_line()` every site calls
/// today — byte-identical output), then the companions. The
/// lead REPLACES the site's existing gpu-mem println! with:
///     crate::memreport_print::print_full_block(&report, adapter.features());
pub(crate) fn print_full_block(report: &crate::adapter::AdapterReport, features: wgpu::Features) {
    let gpu_mem_line = crate::gpucore::mem_stats_line();
    let snap = crate::gpucore::mem_snapshot();
    for line in full_block(&gpu_mem_line, &snap, report, features) {
        println!("{line}");
    }
    // GOV-CORE (GPUFIRST Batch 3): the effective resource
    // limits print beside the mem-report block in every render
    // tail, so a run's log always names what governed it.
    // (The pure `full_block` above is untouched — its line
    // grammar stays pinned; this line is print-site glue.)
    println!("{}", crate::resource::governor().effective_line());
}

/// Convenience for sites holding the adapter itself (main.rs
/// `run`, ab1, dof): describe + features + full block in one
/// call. Equivalent to building the report by hand and calling
/// `print_full_block`.
pub(crate) fn print_full_block_for_adapter(adapter: &wgpu::Adapter) {
    let report = crate::adapter::describe(&adapter.get_info());
    print_full_block(&report, adapter.features());
}

/// The Markdown form for WP6 perf records: delegates to
/// `memreport::render_markdown` with the converted adapter
/// view, so a PERF record and the stdout block can never carry
/// different figures for the same snapshot.
pub(crate) fn markdown_for(
    snap: &MemSnapshot,
    report: &crate::adapter::AdapterReport,
) -> String {
    crate::memreport::render_markdown(snap, &adapter_line(report))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Distinct-valued snapshot (same fixture shape as
    /// memreport.rs's own tests, so the two modules' fixtures
    /// read alike). Class currents A–E + G sum to the accounted
    /// total; F is (0, peak) — peak-side only.
    fn sample_snapshot() -> MemSnapshot {
        MemSnapshot {
            accounted_bytes: 2200,
            peak_accounted_bytes: 2900,
            per_class: [
                (100, 150),
                (200, 250),
                (300, 350),
                (400, 450),
                (500, 550),
                (0, 600),
                (700, 750),
            ],
            textures: 11,
            buffers: 7,
            created_textures: 20,
            created_buffers: 9,
            destroyed_textures: 9,
            destroyed_buffers: 2,
            readback_bytes: 8_355_840,
            sysfs_gtt_used: Some(123_456),
            sysfs_vram_used: Some(789),
        }
    }

    fn integrated_report() -> crate::adapter::AdapterReport {
        crate::adapter::AdapterReport {
            name: "AMD Radeon Vega 7".to_string(),
            backend: "Vulkan".to_string(),
            device_type: "IntegratedGpu".to_string(),
            driver: "radv".to_string(),
            is_software: false,
        }
    }

    fn discrete_report() -> crate::adapter::AdapterReport {
        crate::adapter::AdapterReport {
            name: "GeForce RTX 4070".to_string(),
            backend: "Vulkan".to_string(),
            device_type: "DiscreteGpu".to_string(),
            driver: "nvidia".to_string(),
            is_software: false,
        }
    }

    /// A pinned-grammar gpu-mem line, spelled exactly as
    /// gpucore::format_mem_line spells it (the gpucore test
    /// fixture's figures).
    const GPU_MEM_LINE: &str = "gpu-mem: accounted_bytes=220930688 peak_accounted_bytes=229286528 textures=41 buffers=18 readback_bytes=8355840 sysfs_gtt_used_bytes=unavailable sysfs_vram_used_bytes=unavailable";

    #[test]
    fn adapter_conversion_preserves_all_five_fields() {
        let report = crate::adapter::AdapterReport {
            name: "llvmpipe (LLVM 17)".to_string(),
            backend: "Vulkan".to_string(),
            device_type: "Cpu".to_string(),
            driver: "llvmpipe 0.0.1".to_string(),
            is_software: true,
        };
        let line = adapter_line(&report);
        assert_eq!(line.name, report.name);
        assert_eq!(line.backend, report.backend);
        assert_eq!(line.device_type, report.device_type);
        assert_eq!(line.driver, report.driver);
        assert_eq!(line.is_software, report.is_software);
    }

    #[test]
    fn full_block_starts_with_the_gpu_mem_line_verbatim() {
        let block = full_block(
            GPU_MEM_LINE,
            &sample_snapshot(),
            &discrete_report(),
            wgpu::Features::empty(),
        );
        // The pinned line passes through byte-for-byte, first.
        assert_eq!(block[0], GPU_MEM_LINE);
        // Exactly one gpu-mem line in the block: the companions
        // must never grow a second one.
        assert_eq!(block.iter().filter(|l| l.starts_with("gpu-mem:")).count(), 1);
        // Discrete adapter: 12 mem-report lines (no integrated
        // note) + 1 timestamps line after the pinned line.
        assert_eq!(block.len(), 1 + 12 + 1);
        assert!(block[1].starts_with("mem-report: honesty=DRIVER adapter_name="));
        assert_eq!(block[block.len() - 1], "gpu-timestamps: unavailable");
    }

    #[test]
    fn companion_lines_are_mem_report_then_timestamps() {
        let lines = companion_lines(
            &sample_snapshot(),
            &integrated_report(),
            wgpu::Features::TIMESTAMP_QUERY,
        );
        // Integrated adapter: 13 mem-report lines (incl. the
        // shared-memory note) + the timestamps line.
        assert_eq!(lines.len(), 13 + 1);
        assert!(lines[..13].iter().all(|l| l.starts_with("mem-report: ")));
        assert!(lines.iter().any(|l| l.contains("note=shared_system_memory")));
        assert_eq!(lines[13], "gpu-timestamps: available");
        // The companions delegate to memreport's renderer: the
        // first TRACKED totals line carries the snapshot's
        // figures unchanged.
        assert!(lines
            .iter()
            .any(|l| l == "mem-report: honesty=TRACKED accounted_bytes=2200 peak_accounted_bytes=2900 textures=11 buffers=7"));
    }

    #[test]
    fn timestamps_line_follows_the_real_feature_set() {
        let snap = sample_snapshot();
        let report = discrete_report();
        let with = companion_lines(&snap, &report, wgpu::Features::TIMESTAMP_QUERY);
        assert_eq!(with[with.len() - 1], "gpu-timestamps: available");
        let without = companion_lines(
            &snap,
            &report,
            wgpu::Features::TEXTURE_COMPRESSION_BC,
        );
        assert_eq!(without[without.len() - 1], "gpu-timestamps: unavailable");
    }

    #[test]
    fn sysfs_unavailable_propagates_through_the_glue() {
        let mut snap = sample_snapshot();
        snap.sysfs_gtt_used = None;
        snap.sysfs_vram_used = None;
        let lines = companion_lines(&snap, &discrete_report(), wgpu::Features::empty());
        assert!(lines.iter().any(|l| {
            l == "mem-report: honesty=DRIVER sysfs_gtt_used_bytes=unavailable sysfs_vram_used_bytes=unavailable"
        }));
    }

    #[test]
    fn markdown_delegates_with_converted_adapter() {
        let md = markdown_for(&sample_snapshot(), &integrated_report());
        // Same figures and honesty labels as the line form,
        // adapter strings with spaces preserved (Markdown form).
        assert!(md.contains("| Adapter | AMD Radeon Vega 7 | DRIVER |"), "got:\n{md}");
        assert!(md.contains("| Accounted bytes | 2200 | TRACKED |"), "got:\n{md}");
        assert!(md.contains("VRAM is shared system memory"), "got:\n{md}");
        let md = markdown_for(&sample_snapshot(), &discrete_report());
        assert!(!md.contains("shared system memory"), "got:\n{md}");
    }
}
