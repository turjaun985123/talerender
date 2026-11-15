// ============================================================================
// DELIVERY NOTE — V2 Batch 2, Seat 2 (WP2: telemetry reporting
// completion). NEW FILE ONLY; no existing file was edited, no
// git was run, no dependency was added (std + wgpu only).
//
// Files created by this seat:
//   1. src/memreport.rs (this file)
//
// Contract points implemented (V2-CONTRACTS.md §3, contract C2;
// master prompt §6):
//   - Renders a `crate::gpucore::MemSnapshot` + adapter identity
//     two ways: `render_lines` (stable `mem-report:` key=value
//     lines, below) and `render_markdown` (a table version for
//     docs / PERF records). Both renderers are pure functions of
//     the snapshot — no GPU, no filesystem, no globals — so
//     every figure in them is unit-testable on CPU.
//   - The three honesty classes of master prompt §6 are explicit
//     in BOTH renderers via an `honesty=` field (lines) / a
//     `Source` column (Markdown):
//       TRACKED  — figures from the gpucore allocation ledger
//                  (contract C2's MemSnapshot ledger fields).
//       DRIVER   — driver-reported values: the adapter identity
//                  (reported by wgpu/the driver) and the sysfs
//                  device counters; a missing sysfs counter
//                  renders as the literal `unavailable`, never 0
//                  (the gpucore ledger's standing rule, W84 §7.6).
//       ESTIMATE — anything derived. The only derived figure
//                  this module produces restates the tracked
//                  total as `estimated_vram_lower_bound_bytes`:
//                  the ledger is a lower bound on physical usage
//                  by construction (driver tiling/alignment,
//                  pipelines and bind groups are invisible to it
//                  — gpucore.rs ledger header, W84 §7.3), so the
//                  tracked bytes interpreted AS physical VRAM
//                  are an estimate and are named as one. No other
//                  number is invented.
//   - Per-class lines use gpucore's OWN class meanings
//     (gpucore.rs, MemClass / ledger header; VRAM-TELEMETRY-W56
//     §4): A frame targets, B shadow pair, C bloom chain,
//     D PMREM, E uploads, F readback, G buffers. Class F is
//     peak-side only in the ledger (it never enters the current
//     total); its line carries `note=peak_side_only` so the
//     rendered (current, peak) pair is never misread.
//   - Integrated GPUs (adapter device_type `IntegratedGpu`) add
//     the standing note line in `render_lines` and a note
//     paragraph in `render_markdown`: VRAM is shared system
//     memory (wording below; the meaning is fixed by §6 — on an
//     integrated part the tracked/DRIVER figures draw on the
//     same system RAM, there is no dedicated VRAM pool).
//   - `gpu_timestamps_line(features)` returns exactly
//     `gpu-timestamps: available` iff the adapter's feature set
//     contains `wgpu::Features::TIMESTAMP_QUERY`, else exactly
//     `gpu-timestamps: unavailable` — capability-detected from
//     the caller's `adapter.features()`, never faked (C2).
//
// The gpucore MemSnapshot addition the lead must land WITH this
// seat (contract C2, restated verbatim — this module codes
// against exactly this shape and will not compile without it):
//     pub(crate) struct MemSnapshot {
//         pub accounted_bytes: u64, pub peak_accounted_bytes: u64,
//         pub per_class: [(u64, u64); 7],   // (current, peak) A..G
//         pub textures: u64, pub buffers: u64,
//         pub created_textures: u64, pub created_buffers: u64,
//         pub destroyed_textures: u64, pub destroyed_buffers: u64,
//         pub readback_bytes: u64,
//         pub sysfs_gtt_used: Option<u64>,
//         pub sysfs_vram_used: Option<u64>,
//     }
//     pub(crate) fn mem_snapshot() -> MemSnapshot;
//   Per C2, the creation/destruction counters are added to the
//   gpucore ledger by the lead in the same landing (the current
//   ledger tracks live counts and byte totals only). This seat's
//   tests never call `mem_snapshot()` — they build snapshots by
//   filling the struct literally (see the #[cfg(test)] helper),
//   so the tests pin THIS module's rendering, not the lead's
//   snapshot plumbing.
//
// AdapterLine duplication — flagged honestly for the lead to
// reconcile: this module defines a LOCAL view,
//     pub(crate) struct AdapterLine {
//         pub name: String, pub backend: String,
//         pub device_type: String, pub driver: String,
//         pub is_software: bool,
//     }
//   Batch 2 seat 3's `src/adapter.rs` owns the CANONICAL
//   adapter report (contract C7's `AdapterReport`, same five
//   fields). When adapter.rs lands, the lead should delete this
//   local struct and change the two renderer signatures to take
//   `&crate::adapter::AdapterReport` (field names are identical,
//   so the function bodies and tests need no other change), or
//   add a one-line `From<&AdapterReport> for AdapterLine` at the
//   call site. Until then this local view keeps WP2 independent
//   of WP3's landing order. `is_software` follows C7's
//   definition (device_type == Cpu at the call site) and is
//   ALWAYS rendered (`software=true|false`, never omitted —
//   master prompt §7: software is never reported as hardware).
//
// Wiring the lead must do (this seat edits nothing existing):
//   - src/main.rs: add `mod memreport;` among the module
//     declarations — suggested anchor: immediately after
//     `mod mempressure;` (line 32 at the tree this seat read),
//     before `mod observability;`. That single line is the only
//     change required for this module to compile and its tests
//     to run under CI (plus the C2 MemSnapshot landing above).
//   - Suggested call sites (lead's choice, none required for
//     compilation; the existing pinned `gpu-mem:` line from
//     gpucore::mem_stats_line() stays untouched — these reports
//     are ADDITIVE companions, printed beside it):
//       * At each existing `mem_stats_line()` print site
//         (film.rs:3671, main.rs:990, ab1.rs:1582, dof.rs:528,
//         scenefile.rs:1529 and :2191 at the tree this seat
//         read): build the AdapterLine from the site's
//         wgpu::AdapterInfo, then
//         `for line in memreport::render_lines(
//             &gpucore::mem_snapshot(), &adapter_line) {
//             println!("{line}"); }`
//         and print `memreport::gpu_timestamps_line(
//             adapter.features())` beside it.
//       * WP6's perf lab (contract C2's consumer) calls
//         `render_markdown(&snap, &adapter)` and files the
//         returned String into its PERF record verbatim.
//
// Line grammar (render_lines — field order and spelling are
// stable; a parser may split on spaces then on the first '='):
//     mem-report: honesty=DRIVER adapter_name=<tok> adapter_backend=<tok> adapter_device_type=<tok> adapter_driver=<tok> software=<true|false>
//     mem-report: note=shared_system_memory detail=integrated_gpu_vram_is_shared_system_memory_not_dedicated_vram   [integrated only]
//     mem-report: honesty=TRACKED accounted_bytes=<n> peak_accounted_bytes=<n> textures=<n> buffers=<n>
//     mem-report: honesty=TRACKED class=<A..G> class_name=<slug> current_bytes=<n> peak_bytes=<n>   [class F adds: note=peak_side_only]
//     mem-report: honesty=TRACKED created_textures=<n> created_buffers=<n> destroyed_textures=<n> destroyed_buffers=<n> readback_bytes=<n>
//     mem-report: honesty=DRIVER sysfs_gtt_used_bytes=<n|unavailable> sysfs_vram_used_bytes=<n|unavailable>
//     mem-report: honesty=ESTIMATE estimated_vram_lower_bound_bytes=<n> basis=tracked_ledger_accounted_bytes
//   Adapter identity values are whitespace-folded to '_' (the
//   observability.rs convention) so every line stays one
//   physical line of single-token fields.
//
// Tests included (#[cfg(test)], pure rendering only — no GPU,
// no filesystem, no globals): exact full line vector for a
// fully-populated snapshot (integrated adapter, numeric sysfs),
// sysfs None -> `unavailable` in both renderers, per-class
// names/order against gpucore's meanings, the integrated note
// present for IntegratedGpu and absent for a discrete adapter,
// software=true labelling, Markdown contains the TRACKED /
// DRIVER / ESTIMATE labels + the class table + the shared-
// memory note, and the timestamp line both ways
// (TIMESTAMP_QUERY present / absent).
//
// Known limitations:
//   - Log/report-only: nothing here feeds a pixel, a verdict or
//     a gate; determinism is untouched (same law as
//     observability.rs and the gpucore ledger itself).
//   - `per_class[F]` is rendered verbatim from the snapshot;
//     what the lead's `mem_snapshot()` places in F's current
//     slot (the ledger keeps no standing F bytes — readback is
//     a peak-side note) is the lead's landing decision; the
//     `note=peak_side_only` marker keeps the line honest either
//     way, and `readback_bytes` carries the largest single
//     readback on its own line regardless.
//   - Integrated detection is the exact wgpu device_type
//     spelling `IntegratedGpu` in AdapterLine::device_type;
//     other spellings (e.g. a pre-formatted "Integrated GPU")
//     will not trigger the note — callers should pass
//     `format!("{:?}", info.device_type)` verbatim, as the
//     existing `adapter:` lines do.
//   - No allocation HISTORY (master prompt §6 mentions the
//     word): MemSnapshot (C2) carries current/peak and lifetime
//     counters only, so history is limited to those; a time
//     series would need a contract extension by the lead.
// ============================================================================

//! WP2 telemetry reporting — renders the gpucore VRAM ledger
//! (contract C2's `MemSnapshot`) plus adapter identity as stable
//! `mem-report:` key=value lines and as a Markdown table.
//!
//! Honesty law (master prompt §6): every figure is labelled
//! TRACKED (the renderer's own allocation ledger), DRIVER
//! (driver-reported: adapter identity, sysfs counters —
//! `unavailable` when the driver/OS does not expose them), or
//! ESTIMATE (derived — named as such). Tracked bytes are a
//! lower bound on physical usage, never physical VRAM usage
//! itself; on integrated GPUs VRAM is shared system memory and
//! the report says so.

use crate::gpucore::MemSnapshot;

/// Local view of the selected adapter's identity (see the
/// DELIVERY NOTE: Batch 2 seat 3's `adapter.rs` owns the
/// canonical C7 `AdapterReport` with these same five fields;
/// this struct exists so WP2 does not depend on WP3's landing
/// order, and the lead reconciles the two at integration).
/// Callers fill it from `wgpu::AdapterInfo`: `device_type` is
/// the `{:?}` spelling (e.g. `IntegratedGpu`, `Cpu`) and
/// `is_software` is `device_type == wgpu::DeviceType::Cpu`
/// (contract C7's definition).
pub(crate) struct AdapterLine {
    pub name: String,
    pub backend: String,
    pub device_type: String,
    pub driver: String,
    pub is_software: bool,
}

/// Class letters in ledger order (gpucore `MemClass` A..G).
pub(crate) const CLASS_LETTERS: [&str; 7] = ["A", "B", "C", "D", "E", "F", "G"];

/// Class names — gpucore's OWN meanings (gpucore.rs ledger
/// header / `MemClass` docs; VRAM-TELEMETRY-W56 §4), slugified
/// to single tokens: A frame targets, B shadow pair, C bloom
/// chain, D PMREM, E uploads, F readback, G buffers.
pub(crate) const CLASS_NAMES: [&str; 7] = [
    "frame_targets",
    "shadow_pair",
    "bloom_chain",
    "pmrem",
    "uploads",
    "readback",
    "buffers",
];

/// Index of class F (readback) in `MemSnapshot::per_class` —
/// peak-side only in the ledger; see `render_lines`.
const CLASS_F_INDEX: usize = 5;

/// Fold every whitespace character (space, newline, carriage
/// return, tab) to `'_'` — the observability.rs convention — so
/// a rendered line is always one physical line of single-token
/// fields no matter what an adapter string contains.
fn sanitize(text: &str) -> String {
    text.chars()
        .map(|c| match c {
            ' ' | '\n' | '\r' | '\t' => '_',
            _ => c,
        })
        .collect()
}

/// Escape a value for a Markdown table cell: `|` would split
/// the cell, newlines would split the row.
fn md_cell(text: &str) -> String {
    text.replace('|', "/")
        .replace(['\n', '\r'], " ")
}

/// A sysfs counter as rendered in both renderers: the number,
/// or the literal `unavailable` when the driver/OS exposes no
/// counter — never 0 (gpucore ledger rule, W84 §7.6).
fn fmt_sysfs(v: Option<u64>) -> String {
    match v {
        Some(n) => n.to_string(),
        None => "unavailable".to_string(),
    }
}

/// True when the adapter is an integrated GPU (exact wgpu
/// `DeviceType` spelling), whose VRAM is shared system memory.
fn is_integrated(adapter: &AdapterLine) -> bool {
    adapter.device_type == "IntegratedGpu"
}

/// Render the snapshot + adapter identity as stable
/// `mem-report:` key=value lines (grammar in the DELIVERY
/// NOTE). Pure: the same snapshot and adapter always produce
/// the same lines in the same order.
pub(crate) fn render_lines(snap: &MemSnapshot, adapter: &AdapterLine) -> Vec<String> {
    let mut lines = Vec::new();

    // Adapter identity — reported by the driver via wgpu, so
    // classed DRIVER. `software` is always present (master
    // prompt §7: software is never reported as hardware).
    lines.push(format!(
        "mem-report: honesty=DRIVER adapter_name={} adapter_backend={} adapter_device_type={} adapter_driver={} software={}",
        sanitize(&adapter.name),
        sanitize(&adapter.backend),
        sanitize(&adapter.device_type),
        sanitize(&adapter.driver),
        adapter.is_software,
    ));

    // Standing shared-memory note for integrated GPUs (§6):
    // there is no dedicated VRAM pool — every figure below
    // draws on the same system RAM.
    if is_integrated(adapter) {
        lines.push(
            "mem-report: note=shared_system_memory detail=integrated_gpu_vram_is_shared_system_memory_not_dedicated_vram"
                .to_string(),
        );
    }

    // Ledger totals — TRACKED.
    lines.push(format!(
        "mem-report: honesty=TRACKED accounted_bytes={} peak_accounted_bytes={} textures={} buffers={}",
        snap.accounted_bytes, snap.peak_accounted_bytes, snap.textures, snap.buffers,
    ));

    // Per-class current + peak — TRACKED, gpucore's class
    // names. Class F (readback) is peak-side only in the
    // ledger and says so on its line.
    for (i, (letter, name)) in CLASS_LETTERS.iter().zip(CLASS_NAMES.iter()).enumerate() {
        let (current, peak) = snap.per_class[i];
        let mut line = format!(
            "mem-report: honesty=TRACKED class={letter} class_name={name} current_bytes={current} peak_bytes={peak}"
        );
        if i == CLASS_F_INDEX {
            line.push_str(" note=peak_side_only");
        }
        lines.push(line);
    }

    // Lifetime creation/destruction counters + the largest
    // single readback — TRACKED.
    lines.push(format!(
        "mem-report: honesty=TRACKED created_textures={} created_buffers={} destroyed_textures={} destroyed_buffers={} readback_bytes={}",
        snap.created_textures,
        snap.created_buffers,
        snap.destroyed_textures,
        snap.destroyed_buffers,
        snap.readback_bytes,
    ));

    // Driver-reported device-wide sysfs counters — DRIVER.
    // Device-wide (all processes), not this process's share;
    // `unavailable` when the OS exposes no counter.
    lines.push(format!(
        "mem-report: honesty=DRIVER sysfs_gtt_used_bytes={} sysfs_vram_used_bytes={}",
        fmt_sysfs(snap.sysfs_gtt_used),
        fmt_sysfs(snap.sysfs_vram_used),
    ));

    // The one derived figure — ESTIMATE, named as such: the
    // tracked ledger total read as physical VRAM is only a
    // lower-bound estimate (driver-internal allocations are
    // invisible to the ledger).
    lines.push(format!(
        "mem-report: honesty=ESTIMATE estimated_vram_lower_bound_bytes={} basis=tracked_ledger_accounted_bytes",
        snap.accounted_bytes,
    ));

    lines
}

/// Render the snapshot + adapter identity as a Markdown table
/// version of `render_lines`, for docs / PERF records (WP6).
/// Same figures, same honesty labels (the `Source` column),
/// same `unavailable` spelling; adapter strings keep their
/// spaces here (only `|` and newlines are escaped, via
/// `md_cell`).
pub(crate) fn render_markdown(snap: &MemSnapshot, adapter: &AdapterLine) -> String {
    let mut out = String::new();
    out.push_str("### GPU memory report (mem-report)\n\n");

    out.push_str("| Field | Value | Source |\n");
    out.push_str("|---|---|---|\n");
    out.push_str(&format!(
        "| Adapter | {} | DRIVER |\n",
        md_cell(&adapter.name)
    ));
    out.push_str(&format!(
        "| Backend | {} | DRIVER |\n",
        md_cell(&adapter.backend)
    ));
    out.push_str(&format!(
        "| Device type | {} | DRIVER |\n",
        md_cell(&adapter.device_type)
    ));
    out.push_str(&format!(
        "| Driver | {} | DRIVER |\n",
        md_cell(&adapter.driver)
    ));
    out.push_str(&format!(
        "| Software rendering | {} | DRIVER |\n",
        adapter.is_software
    ));
    out.push('\n');

    if is_integrated(adapter) {
        out.push_str(
            "> Note: integrated GPU — VRAM is shared system memory; \
             the tracked and driver-reported figures below draw on \
             the same system RAM, not a dedicated VRAM pool.\n\n",
        );
    }

    out.push_str("| Class | Name | Current (bytes) | Peak (bytes) | Source |\n");
    out.push_str("|---|---|---|---|---|\n");
    for (i, (letter, name)) in CLASS_LETTERS.iter().zip(CLASS_NAMES.iter()).enumerate() {
        let (current, peak) = snap.per_class[i];
        out.push_str(&format!(
            "| {letter} | {name} | {current} | {peak} | TRACKED |\n"
        ));
    }
    out.push('\n');

    out.push_str("| Metric | Value | Source |\n");
    out.push_str("|---|---|---|\n");
    out.push_str(&format!(
        "| Accounted bytes | {} | TRACKED |\n",
        snap.accounted_bytes
    ));
    out.push_str(&format!(
        "| Peak accounted bytes | {} | TRACKED |\n",
        snap.peak_accounted_bytes
    ));
    out.push_str(&format!("| Textures (live) | {} | TRACKED |\n", snap.textures));
    out.push_str(&format!("| Buffers (live) | {} | TRACKED |\n", snap.buffers));
    out.push_str(&format!(
        "| Textures created | {} | TRACKED |\n",
        snap.created_textures
    ));
    out.push_str(&format!(
        "| Buffers created | {} | TRACKED |\n",
        snap.created_buffers
    ));
    out.push_str(&format!(
        "| Textures destroyed | {} | TRACKED |\n",
        snap.destroyed_textures
    ));
    out.push_str(&format!(
        "| Buffers destroyed | {} | TRACKED |\n",
        snap.destroyed_buffers
    ));
    out.push_str(&format!(
        "| Readback bytes (largest single) | {} | TRACKED |\n",
        snap.readback_bytes
    ));
    out.push_str(&format!(
        "| Sysfs GTT used (bytes, device-wide) | {} | DRIVER |\n",
        fmt_sysfs(snap.sysfs_gtt_used)
    ));
    out.push_str(&format!(
        "| Sysfs VRAM used (bytes, device-wide) | {} | DRIVER |\n",
        fmt_sysfs(snap.sysfs_vram_used)
    ));
    out.push_str(&format!(
        "| Estimated VRAM lower bound (bytes) | {} | ESTIMATE |\n",
        snap.accounted_bytes
    ));
    out.push('\n');
    out.push_str(
        "ESTIMATE basis: the tracked ledger total is a lower bound on physical \
         GPU memory usage (driver tiling/alignment, pipelines, shader modules, \
         bind groups and samplers are not visible to the ledger); it is an \
         estimate of physical VRAM, never a driver measurement.\n",
    );
    out
}

/// GPU timestamp capability line: exactly
/// `gpu-timestamps: available` iff `features` contains
/// `wgpu::Features::TIMESTAMP_QUERY`, else exactly
/// `gpu-timestamps: unavailable`. Capability-detected from the
/// adapter's real feature set (callers pass
/// `adapter.features()`) — never faked (contract C2).
pub(crate) fn gpu_timestamps_line(features: wgpu::Features) -> String {
    if features.contains(wgpu::Features::TIMESTAMP_QUERY) {
        "gpu-timestamps: available".to_string()
    } else {
        "gpu-timestamps: unavailable".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Fully-populated snapshot, built by filling the C2 struct
    /// literally. Every number is distinct so an ordering or
    /// field swap fails the exact-line test. Class currents sum
    /// A–E + G = 100+200+300+400+500+700 = 2200 (class F never
    /// enters the current total — gpucore ledger semantics);
    /// F's pair is (0, 600): no standing bytes, peak-side only.
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

    /// Integrated hardware adapter (the 5650G-class target):
    /// the name's spaces exercise the whitespace folding.
    fn integrated_adapter() -> AdapterLine {
        AdapterLine {
            name: "AMD Radeon Vega 7".to_string(),
            backend: "Vulkan".to_string(),
            device_type: "IntegratedGpu".to_string(),
            driver: "radv".to_string(),
            is_software: false,
        }
    }

    fn discrete_adapter() -> AdapterLine {
        AdapterLine {
            name: "GeForce RTX 4070".to_string(),
            backend: "Vulkan".to_string(),
            device_type: "DiscreteGpu".to_string(),
            driver: "nvidia".to_string(),
            is_software: false,
        }
    }

    #[test]
    fn exact_lines_for_fully_populated_snapshot() {
        let lines = render_lines(&sample_snapshot(), &integrated_adapter());
        let expected: Vec<String> = [
            "mem-report: honesty=DRIVER adapter_name=AMD_Radeon_Vega_7 adapter_backend=Vulkan adapter_device_type=IntegratedGpu adapter_driver=radv software=false",
            "mem-report: note=shared_system_memory detail=integrated_gpu_vram_is_shared_system_memory_not_dedicated_vram",
            "mem-report: honesty=TRACKED accounted_bytes=2200 peak_accounted_bytes=2900 textures=11 buffers=7",
            "mem-report: honesty=TRACKED class=A class_name=frame_targets current_bytes=100 peak_bytes=150",
            "mem-report: honesty=TRACKED class=B class_name=shadow_pair current_bytes=200 peak_bytes=250",
            "mem-report: honesty=TRACKED class=C class_name=bloom_chain current_bytes=300 peak_bytes=350",
            "mem-report: honesty=TRACKED class=D class_name=pmrem current_bytes=400 peak_bytes=450",
            "mem-report: honesty=TRACKED class=E class_name=uploads current_bytes=500 peak_bytes=550",
            "mem-report: honesty=TRACKED class=F class_name=readback current_bytes=0 peak_bytes=600 note=peak_side_only",
            "mem-report: honesty=TRACKED class=G class_name=buffers current_bytes=700 peak_bytes=750",
            "mem-report: honesty=TRACKED created_textures=20 created_buffers=9 destroyed_textures=9 destroyed_buffers=2 readback_bytes=8355840",
            "mem-report: honesty=DRIVER sysfs_gtt_used_bytes=123456 sysfs_vram_used_bytes=789",
            "mem-report: honesty=ESTIMATE estimated_vram_lower_bound_bytes=2200 basis=tracked_ledger_accounted_bytes",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        assert_eq!(lines, expected);
    }

    #[test]
    fn sysfs_none_renders_unavailable_never_zero() {
        let mut snap = sample_snapshot();
        snap.sysfs_gtt_used = None;
        snap.sysfs_vram_used = None;
        let lines = render_lines(&snap, &discrete_adapter());
        let sysfs_line = lines
            .iter()
            .find(|l| l.contains("sysfs_gtt_used_bytes"))
            .expect("sysfs line present");
        assert_eq!(
            sysfs_line,
            "mem-report: honesty=DRIVER sysfs_gtt_used_bytes=unavailable sysfs_vram_used_bytes=unavailable"
        );
        // Mixed case: one counter present, the other not.
        snap.sysfs_gtt_used = Some(42);
        let lines = render_lines(&snap, &discrete_adapter());
        let sysfs_line = lines
            .iter()
            .find(|l| l.contains("sysfs_gtt_used_bytes"))
            .expect("sysfs line present");
        assert_eq!(
            sysfs_line,
            "mem-report: honesty=DRIVER sysfs_gtt_used_bytes=42 sysfs_vram_used_bytes=unavailable"
        );
        // Markdown spells it the same way.
        let mut snap = sample_snapshot();
        snap.sysfs_gtt_used = None;
        snap.sysfs_vram_used = None;
        let md = render_markdown(&snap, &discrete_adapter());
        assert!(md.contains("| Sysfs GTT used (bytes, device-wide) | unavailable | DRIVER |"), "got:\n{md}");
        assert!(md.contains("| Sysfs VRAM used (bytes, device-wide) | unavailable | DRIVER |"), "got:\n{md}");
    }

    #[test]
    fn per_class_names_match_gpucore_meanings() {
        // gpucore.rs (MemClass / ledger header) and
        // VRAM-TELEMETRY-W56 §4: A frame targets, B shadow pair,
        // C bloom chain, D PMREM, E uploads, F readback,
        // G buffers — in that order, one line per class.
        assert_eq!(
            CLASS_NAMES,
            ["frame_targets", "shadow_pair", "bloom_chain", "pmrem", "uploads", "readback", "buffers"]
        );
        assert_eq!(CLASS_LETTERS, ["A", "B", "C", "D", "E", "F", "G"]);
        let lines = render_lines(&sample_snapshot(), &discrete_adapter());
        let class_lines: Vec<&String> = lines.iter().filter(|l| l.contains(" class=")).collect();
        assert_eq!(class_lines.len(), 7);
        for (i, line) in class_lines.iter().enumerate() {
            assert!(
                line.contains(&format!("class={}", CLASS_LETTERS[i])),
                "line {i}: {line}"
            );
            assert!(
                line.contains(&format!("class_name={}", CLASS_NAMES[i])),
                "line {i}: {line}"
            );
        }
        // Only class F carries the peak-side-only note.
        assert!(class_lines[5].contains("note=peak_side_only"));
        for (i, line) in class_lines.iter().enumerate() {
            if i != 5 {
                assert!(!line.contains("peak_side_only"), "line {i}: {line}");
            }
        }
    }

    #[test]
    fn integrated_note_present_only_for_integrated() {
        let lines = render_lines(&sample_snapshot(), &integrated_adapter());
        assert!(
            lines.iter().any(|l| l.contains("note=shared_system_memory")
                && l.contains("shared_system_memory_not_dedicated_vram")),
            "integrated lines: {lines:?}"
        );
        let lines = render_lines(&sample_snapshot(), &discrete_adapter());
        assert!(
            !lines.iter().any(|l| l.contains("shared_system_memory")),
            "discrete lines must not carry the note: {lines:?}"
        );
        // One line fewer without the note (13 -> 12).
        assert_eq!(lines.len(), 12);
    }

    #[test]
    fn software_adapter_is_labelled_software() {
        let software = AdapterLine {
            name: "llvmpipe (LLVM 17)".to_string(),
            backend: "Vulkan".to_string(),
            device_type: "Cpu".to_string(),
            driver: "llvmpipe".to_string(),
            is_software: true,
        };
        let lines = render_lines(&sample_snapshot(), &software);
        assert_eq!(
            lines[0],
            "mem-report: honesty=DRIVER adapter_name=llvmpipe_(LLVM_17) adapter_backend=Vulkan adapter_device_type=Cpu adapter_driver=llvmpipe software=true"
        );
        // A Cpu adapter is not integrated: no shared-memory note.
        assert!(!lines.iter().any(|l| l.contains("shared_system_memory")));
    }

    #[test]
    fn markdown_contains_honesty_labels_and_all_sections() {
        let md = render_markdown(&sample_snapshot(), &integrated_adapter());
        // The three honesty classes are explicit.
        assert!(md.contains("TRACKED"), "got:\n{md}");
        assert!(md.contains("DRIVER"), "got:\n{md}");
        assert!(md.contains("ESTIMATE"), "got:\n{md}");
        // Adapter identity, with spaces preserved in Markdown.
        assert!(md.contains("| Adapter | AMD Radeon Vega 7 | DRIVER |"), "got:\n{md}");
        assert!(md.contains("| Software rendering | false | DRIVER |"), "got:\n{md}");
        // Every class row, with gpucore's names and figures.
        assert!(md.contains("| A | frame_targets | 100 | 150 | TRACKED |"), "got:\n{md}");
        assert!(md.contains("| B | shadow_pair | 200 | 250 | TRACKED |"), "got:\n{md}");
        assert!(md.contains("| C | bloom_chain | 300 | 350 | TRACKED |"), "got:\n{md}");
        assert!(md.contains("| D | pmrem | 400 | 450 | TRACKED |"), "got:\n{md}");
        assert!(md.contains("| E | uploads | 500 | 550 | TRACKED |"), "got:\n{md}");
        assert!(md.contains("| F | readback | 0 | 600 | TRACKED |"), "got:\n{md}");
        assert!(md.contains("| G | buffers | 700 | 750 | TRACKED |"), "got:\n{md}");
        // Totals, counters, readback, sysfs, estimate.
        assert!(md.contains("| Accounted bytes | 2200 | TRACKED |"), "got:\n{md}");
        assert!(md.contains("| Peak accounted bytes | 2900 | TRACKED |"), "got:\n{md}");
        assert!(md.contains("| Textures created | 20 | TRACKED |"), "got:\n{md}");
        assert!(md.contains("| Buffers destroyed | 2 | TRACKED |"), "got:\n{md}");
        assert!(md.contains("| Readback bytes (largest single) | 8355840 | TRACKED |"), "got:\n{md}");
        assert!(md.contains("| Sysfs GTT used (bytes, device-wide) | 123456 | DRIVER |"), "got:\n{md}");
        assert!(md.contains("| Estimated VRAM lower bound (bytes) | 2200 | ESTIMATE |"), "got:\n{md}");
        // The integrated shared-memory note.
        assert!(md.contains("VRAM is shared system memory"), "got:\n{md}");
        // A discrete adapter's Markdown has no such note.
        let md = render_markdown(&sample_snapshot(), &discrete_adapter());
        assert!(!md.contains("shared system memory"), "got:\n{md}");
    }

    #[test]
    fn gpu_timestamps_line_both_ways() {
        assert_eq!(
            gpu_timestamps_line(wgpu::Features::TIMESTAMP_QUERY),
            "gpu-timestamps: available"
        );
        assert_eq!(
            gpu_timestamps_line(wgpu::Features::empty()),
            "gpu-timestamps: unavailable"
        );
        // Other features do not imply timestamp support.
        assert_eq!(
            gpu_timestamps_line(wgpu::Features::TEXTURE_COMPRESSION_BC),
            "gpu-timestamps: unavailable"
        );
        // Combined with unrelated features, detection still holds.
        assert_eq!(
            gpu_timestamps_line(
                wgpu::Features::TIMESTAMP_QUERY | wgpu::Features::TEXTURE_COMPRESSION_BC
            ),
            "gpu-timestamps: available"
        );
    }

    #[test]
    fn every_line_is_a_single_mem_report_line() {
        // The stable grammar: one physical line each, prefixed,
        // no value ever containing whitespace beyond the field
        // separators (adapter strings are folded by sanitize).
        for adapter in [integrated_adapter(), discrete_adapter()] {
            for line in render_lines(&sample_snapshot(), &adapter) {
                assert!(line.starts_with("mem-report: "), "got: {line}");
                assert!(!line.contains('\n'), "got: {line}");
            }
        }
    }
}
