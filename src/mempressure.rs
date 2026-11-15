/*
 * DELIVERY NOTE — TALERENDER V2, Batch 1, Seat 2: WP1.3
 * memory-pressure controller (contract C1,
 * docs/integration/V2-CONTRACTS.md §3; master prompt §5.3).
 *
 * FILES CREATED (one, new files only — no existing file was
 * edited by this seat):
 *   src/mempressure.rs  (this file; module + in-module tests)
 *
 * CONTRACT POINTS IMPLEMENTED (C1, exactly as specified):
 *   - MemoryProfile { LowMemory, Balanced, HighPerformance }
 *     and PressureLevel { Normal, Elevated, Critical },
 *     both pub(crate), with the exact variant names/order.
 *   - MemPressure::from_env_and_arg(Option<&str>),
 *     profile(), paint_budget_bytes(),
 *     frame_queue_capacity() (LowMemory 2 / Balanced 4 /
 *     HighPerformance 8), level(accounted_bytes,
 *     peak_rss_mb), preload_enabled() (false on LowMemory
 *     only).
 *   - Profile resolution precedence: CLI arg >
 *     TALERENDER_MEM_PROFILE env > machine default, where
 *     the machine default is LowMemory when total RAM
 *     reported by /proc/meminfo is <= 8 GiB, else Balanced
 *     (a meminfo read/parse failure also yields Balanced,
 *     per C1). Invalid arg/env strings fall through to the
 *     next source; nothing panics.
 *   - paint_budget_bytes(): LowMemory is exactly the
 *     derived 2 GiB ceiling, referenced directly as
 *     crate::paintbudget::BUDGET_CEILING_BYTES (it is
 *     pub(crate) in src/paintbudget.rs, so no mirrored copy
 *     was needed); Balanced = 1.5x that ceiling (3 GiB),
 *     HighPerformance = 2x (4 GiB). All profiles bounded.
 *   - /proc/meminfo MemTotal parsing is a pure function,
 *     parse_meminfo_total_bytes(&str) -> Option<u64>
 *     (kB/KiB units, strict suffix, checked arithmetic),
 *     and resolution itself is the pure
 *     resolve_profile(arg, env, total_ram_bytes), so both
 *     are unit-tested without touching env or /proc.
 *   - level() thresholds are documented on the method:
 *     accounted bytes at >= 75% of paint_budget_bytes() is
 *     Elevated and >= 100% is Critical; peak RSS at >= 2x
 *     the budget is Elevated and >= 3x is Critical; the
 *     result is the worse of the two signals, hence
 *     monotonic non-decreasing in accounted_bytes for any
 *     fixed RSS input. None / NaN / negative RSS is ignored.
 *   - Extra helpers beyond C1 (all pub(crate), additive):
 *     MemPressure::from_profile (construct without I/O —
 *     for the scheduler/streaming seats and tests),
 *     MemPressure::total_ram_bytes (the probed total, for
 *     reporting), MemoryProfile::name (stable log spelling),
 *     parse_profile (the shared arg/env spelling parser).
 *
 * WIRING THE LEAD MUST DO (this seat edited nothing else):
 *   1. src/main.rs — mod line: insert `mod mempressure;`
 *      immediately after the existing `mod math;` line in
 *      the mod block at the top of main.rs.
 *   2. src/main.rs — CLI: add a `--mem-profile <PROFILE>`
 *      flag mirroring the existing `"--post-stage"` arm
 *      (value via the `value()` helper), storing into a new
 *      `let mut mem_profile_arg: Option<String> = None;`
 *      beside the other option locals; document it in the
 *      HELP const (OPTIONS + an ENVIRONMENT entry for
 *      TALERENDER_MEM_PROFILE). Suggested lead behaviour:
 *      main.rs may cli_error on an unparseable CLI value;
 *      this module itself never panics and falls through.
 *      Construct once after the argument loop (anchor: just
 *      before `if let Some(mode) = &telemetry_mode {`):
 *        let mem_pressure =
 *            crate::mempressure::MemPressure::from_env_and_arg(
 *                mem_profile_arg.as_deref());
 *      and pass it (or its derived values) down, OR — the
 *      minimal landing — construct it at the consumers:
 *   3. src/film.rs — in run_film, beside the existing
 *      anchors `let mut cache = PaintCache::new();` and
 *      `let clock = crate::runlog::RunClock::new();`,
 *      construct the controller for the run and use:
 *        - paint_budget_bytes() as the PROFILE paint
 *          budget when the PaintBudget is built. Note the
 *          interaction the lead must settle: PaintCache::new
 *          currently calls PaintBudget::from_env, whose
 *          TALERENDER_PAINT_BUDGET_MB override and derived
 *          2 GiB clamp already exist — keep that env var as
 *          the operator override and treat
 *          MemPressure::paint_budget_bytes() as the profile
 *          default ceiling it feeds, not a replacement.
 *        - preload_enabled() to gate any asset preload.
 *        - a pressure sample per progress tick / at end of
 *          run:
 *            mem_pressure.level(accounted, crate::runlog::peak_rss_mb())
 *          where `accounted` is the caller-supplied total
 *          accounted bytes (paint-cache bytes via the
 *          cache's PaintBudget::bytes() plus the GPU
 *          ledger's current accounted bytes — today that
 *          figure is printed by gpucore::mem_stats_line();
 *          once C2 lands it is MemSnapshot.accounted_bytes).
 *   4. Consumers named by the contracts: WP4 streaming
 *      (src/streamq.rs) takes its bounded capacity from
 *      frame_queue_capacity(); WP7 scheduler (C5) takes
 *      its LowMemory -> 1-in-flight default from profile();
 *      WP1/WP7 read preload_enabled() / level() as above.
 *      No call site changes behaviour until the lead wires
 *      one — this module is policy only and allocates,
 *      renders and prints nothing.
 *
 * TESTS (in-module, #[cfg(test)], run by CI at integration):
 *   meminfo parsing (exact kB->byte conversion incl. the
 *   8 GiB boundary value, MemFree/MemAvailable not mistaken
 *   for MemTotal, missing line, non-numeric value, wrong
 *   unit, missing unit, empty input); profile-spelling
 *   parsing; resolution precedence (arg > env > machine
 *   default, invalid values fall through, <= 8 GiB ->
 *   LowMemory, 8 GiB + 1 byte -> Balanced, unknown RAM ->
 *   Balanced); the capacity/preload/paint-budget table for
 *   all three profiles; level() boundaries (75% / 100% of
 *   budget, the 2x / 3x RSS thresholds in MiB, RSS ignored
 *   when None/NaN/negative, and a monotonicity sweep in
 *   accounted_bytes, with and without a fixed RSS signal).
 *   from_env_and_arg itself is a thin I/O wrapper (env var
 *   + /proc read) over the tested pure resolve_profile /
 *   parse_meminfo_total_bytes, and is deliberately not
 *   unit-tested by mutating process env.
 *
 * LIMITATIONS / KNOWN NOTES:
 *   - No local build was run (standing law: CI is the only
 *     build; this seat cannot compile). Syntax/API surface
 *     was checked against V2-CONTRACTS.md C1 and the
 *     paintbudget/runlog sources only.
 *   - level() is a policy signal, not an actuator: it does
 *     not evict, shrink or block anything by itself. What a
 *     caller does at Elevated/Critical (evict harder, stop
 *     preloading, shrink queues) is the wiring seat's /
 *     lead's decision at integration.
 *   - The RSS thresholds are expressed as multiples of the
 *     profile paint budget (documented on level()) rather
 *     than fractions of total RAM, so level() stays a pure
 *     function of its two arguments even when the meminfo
 *     probe failed at construction.
 *   - Non-Linux: /proc/meminfo is absent, the probe yields
 *     None, and the default is Balanced (C1's failure rule).
 */

//! Memory-pressure controller (WP1.3; contract C1 in
//! docs/integration/V2-CONTRACTS.md §3; master prompt §5.3).
//!
//! One central policy object, [`MemPressure`], that the
//! paint cache, buffer pool, texture atlas, render targets,
//! scene assets, frame queues and temporary resources can
//! consult for their bounds, instead of each subsystem
//! inventing its own idea of "how much memory is fine on
//! this machine". The primary target is the user's
//! low-memory machine (~6.7 GiB usable RAM, shared with a
//! Vega 7 iGPU); high-end machines are the secondary target.
//!
//! HARD LAW (master prompt §5.3, restated for this module):
//! profiles NEVER change output quality, resolution, fps or
//! encoding. A profile bounds caches, queues and preload
//! only. Selecting LowMemory can make a run slower (more
//! re-paints, a shorter frame queue, no preload); it can
//! never make a frame different.
//!
//! This module is policy only: it allocates nothing, owns
//! no GPU resource, prints nothing, and — apart from the
//! one env + /proc read inside
//! [`MemPressure::from_env_and_arg`] — every function is
//! pure, so the whole surface is unit-tested below without
//! touching the process environment or the filesystem.
//!
//! std only; no new dependencies (zero-new-deps law).

/// One gibibyte, in bytes. All budgets in this module are
/// stated in GiB multiples so the numbers stay comparable
/// with `paintbudget`'s derivation (which is in MiB/GiB).
const GIB: u64 = 1024 * 1024 * 1024;

/// Total-RAM ceiling for the machine default: a machine
/// reporting at most this much total RAM (and no explicit
/// profile from arg or env) defaults to
/// [`MemoryProfile::LowMemory`]. 8 GiB exactly is LowMemory
/// (the comparison is `<=`, per contract C1).
pub(crate) const LOW_MEMORY_RAM_CEILING_BYTES: u64 = 8 * GIB;

/// The memory profile in force for a run. Selected once,
/// at startup, by [`MemPressure::from_env_and_arg`]; it
/// never changes mid-run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MemoryProfile {
    /// 6–8 GiB system-RAM target (the primary machine):
    /// conservative preload (off), the strictest paint
    /// budget (the derived 2 GiB ceiling), the shortest
    /// frame queue.
    LowMemory,
    /// Moderate caching and preloading; the default on any
    /// machine reporting more than 8 GiB of RAM.
    Balanced,
    /// Larger allowances where beneficial, for machines
    /// with memory to spare. Still bounded — never
    /// unlimited.
    HighPerformance,
}

impl MemoryProfile {
    /// Stable lowercase spelling for logs and reports
    /// (`low-memory` / `balanced` / `high-performance`).
    /// Also one of the spellings [`parse_profile`] accepts.
    pub(crate) fn name(self) -> &'static str {
        match self {
            MemoryProfile::LowMemory => "low-memory",
            MemoryProfile::Balanced => "balanced",
            MemoryProfile::HighPerformance => "high-performance",
        }
    }
}

/// How much memory pressure the run is under right now, as
/// judged by [`MemPressure::level`]. Declaration order is
/// the severity order (`Normal < Elevated < Critical`),
/// which the derived `Ord` preserves.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum PressureLevel {
    /// Comfortably inside the profile's bounds.
    Normal,
    /// Approaching the profile's bounds: a caller should
    /// stop preloading and let caches evict before growing
    /// anything new.
    Elevated,
    /// At or past the profile's paint budget (or an RSS
    /// high-water far beyond it): a caller should reclaim
    /// aggressively before allocating anything new.
    Critical,
}

/// Parse one profile spelling (a `--mem-profile` value or
/// the `TALERENDER_MEM_PROFILE` env value). Case-insensitive;
/// `-`, `_` and spaces are ignored, so `low-memory`,
/// `low_memory`, `LowMemory` and `low` all name LowMemory,
/// and `high`, `high-performance` and `highperformance`
/// all name HighPerformance. Anything else is `None` —
/// callers fall through to the next resolution source
/// rather than panicking (the lead's CLI layer may still
/// choose to hard-error on a bad explicit flag).
pub(crate) fn parse_profile(raw: &str) -> Option<MemoryProfile> {
    let norm: String = raw
        .trim()
        .to_ascii_lowercase()
        .chars()
        .filter(|c| *c != '-' && *c != '_' && *c != ' ')
        .collect();
    match norm.as_str() {
        "low" | "lowmemory" => Some(MemoryProfile::LowMemory),
        "balanced" => Some(MemoryProfile::Balanced),
        "high" | "highperformance" => Some(MemoryProfile::HighPerformance),
        _ => None,
    }
}

/// Parse the `MemTotal:` line of `/proc/meminfo` text into
/// total RAM in BYTES. Pure and testable: takes the file's
/// text, never reads the filesystem itself.
///
/// The kernel reports KiB while labelling the unit `kB`
/// (`MemTotal:       16384256 kB` = 16,384,256 KiB =
/// 17,179,869,184 bytes); this parser requires that exact
/// `kB` suffix, converts with checked arithmetic, and
/// returns `None` for a missing line, a non-numeric value,
/// a wrong or missing unit, or an overflowing value.
/// Lines for other fields (`MemFree:`, `MemAvailable:`,
/// …) are never mistaken for the total: only a line
/// starting exactly with `MemTotal:` is considered.
pub(crate) fn parse_meminfo_total_bytes(text: &str) -> Option<u64> {
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("MemTotal:") {
            let kib: u64 = rest.trim().strip_suffix("kB")?.trim().parse().ok()?;
            return kib.checked_mul(1024);
        }
    }
    None
}

/// The machine default when neither arg nor env names a
/// profile: LowMemory on a machine reporting at most
/// [`LOW_MEMORY_RAM_CEILING_BYTES`] of total RAM, Balanced
/// otherwise — including when the total is unknown (a
/// failed /proc probe), per contract C1's failure rule.
pub(crate) fn default_profile_for_ram(total_ram_bytes: Option<u64>) -> MemoryProfile {
    match total_ram_bytes {
        Some(total) if total <= LOW_MEMORY_RAM_CEILING_BYTES => MemoryProfile::LowMemory,
        _ => MemoryProfile::Balanced,
    }
}

/// Pure profile resolution, in contract C1 precedence
/// order: CLI arg > env > machine default (see
/// [`default_profile_for_ram`]). An arg or env value that
/// does not parse falls through to the next source; it
/// never panics and never invents a profile.
pub(crate) fn resolve_profile(
    arg: Option<&str>,
    env: Option<&str>,
    total_ram_bytes: Option<u64>,
) -> MemoryProfile {
    if let Some(p) = arg.and_then(parse_profile) {
        return p;
    }
    if let Some(p) = env.and_then(parse_profile) {
        return p;
    }
    default_profile_for_ram(total_ram_bytes)
}

/// Best-effort total-RAM probe: read `/proc/meminfo` and
/// parse its `MemTotal:` line. Any failure (non-Linux, an
/// unreadable file, malformed contents) yields `None`,
/// which resolution treats as Balanced.
fn read_total_ram_bytes() -> Option<u64> {
    let text = std::fs::read_to_string("/proc/meminfo").ok()?;
    parse_meminfo_total_bytes(&text)
}

/// The memory-pressure controller for one run: the
/// resolved [`MemoryProfile`] plus the total RAM observed
/// at construction (kept for reporting; `level()` does not
/// depend on it — see its docs). Construct once at startup
/// via [`MemPressure::from_env_and_arg`] and share the
/// derived bounds with every consumer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MemPressure {
    profile: MemoryProfile,
    total_ram_bytes: Option<u64>,
}

impl MemPressure {
    /// Production constructor (contract C1): resolve the
    /// profile from the `--mem-profile` CLI value in `arg`
    /// first, then the `TALERENDER_MEM_PROFILE` environment
    /// variable, then the machine default derived from
    /// `/proc/meminfo`'s total RAM (<= 8 GiB → LowMemory,
    /// else Balanced; probe failure → Balanced). This is
    /// the only constructor that touches the environment
    /// or the filesystem.
    pub(crate) fn from_env_and_arg(arg: Option<&str>) -> MemPressure {
        let env_raw = std::env::var("TALERENDER_MEM_PROFILE").ok();
        let total_ram_bytes = read_total_ram_bytes();
        MemPressure {
            profile: resolve_profile(arg, env_raw.as_deref(), total_ram_bytes),
            total_ram_bytes,
        }
    }

    /// Construct directly from an already-resolved profile,
    /// with no env or /proc access. For consumers that
    /// receive the profile from the lead's wiring (and for
    /// tests). The total-RAM field is `None` in this form.
    pub(crate) fn from_profile(profile: MemoryProfile) -> MemPressure {
        MemPressure {
            profile,
            total_ram_bytes: None,
        }
    }

    /// The profile in force for this run.
    pub(crate) fn profile(&self) -> MemoryProfile {
        self.profile
    }

    /// Total RAM in bytes observed at construction, when
    /// the /proc probe succeeded. Reporting only.
    pub(crate) fn total_ram_bytes(&self) -> Option<u64> {
        self.total_ram_bytes
    }

    /// The paint-cache byte budget for this profile.
    ///
    /// LowMemory is exactly the derived 2 GiB ceiling,
    /// [`crate::paintbudget::BUDGET_CEILING_BYTES`] — the
    /// constant `paintbudget.rs` derives from the primary
    /// machine's ~6.7 GiB of usable RAM (referenced
    /// directly, not mirrored; it is `pub(crate)` there) —
    /// and never above it. Balanced allows 1.5× that
    /// ceiling (3 GiB) and HighPerformance 2× (4 GiB):
    /// larger allowances where a bigger machine makes them
    /// beneficial, still hard-bounded on every profile.
    /// This is a budget for cached painted canvases only;
    /// per the module law it can cost re-paint time, never
    /// pixels.
    pub(crate) fn paint_budget_bytes(&self) -> u64 {
        match self.profile {
            MemoryProfile::LowMemory => crate::paintbudget::BUDGET_CEILING_BYTES,
            MemoryProfile::Balanced => {
                crate::paintbudget::BUDGET_CEILING_BYTES
                    + crate::paintbudget::BUDGET_CEILING_BYTES / 2
            }
            MemoryProfile::HighPerformance => crate::paintbudget::BUDGET_CEILING_BYTES * 2,
        }
    }

    /// Capacity of the bounded frame queue (contract C3's
    /// `frame_queue`): LowMemory 2, Balanced 4,
    /// HighPerformance 8. Bounded on every profile — the
    /// producer blocks on a full queue (backpressure); it
    /// never drops a frame and never accumulates without
    /// limit, which is what makes streaming safe on the
    /// 6.7 GiB target.
    pub(crate) fn frame_queue_capacity(&self) -> usize {
        match self.profile {
            MemoryProfile::LowMemory => 2,
            MemoryProfile::Balanced => 4,
            MemoryProfile::HighPerformance => 8,
        }
    }

    /// Whether speculative preload of upcoming assets is
    /// allowed under this profile: false on LowMemory
    /// (conservative preloading — load on demand only),
    /// true on Balanced and HighPerformance.
    pub(crate) fn preload_enabled(&self) -> bool {
        !matches!(self.profile, MemoryProfile::LowMemory)
    }

    /// Judge the current pressure from the caller's two
    /// honest signals:
    ///
    /// - `accounted_bytes`: the total bytes the caller can
    ///   account for right now (e.g. paint-cache bytes plus
    ///   the GPU ledger's current accounted bytes). This
    ///   module does not read any ledger itself; the caller
    ///   supplies the figure.
    /// - `peak_rss_mb`: the process peak RSS in MiB, as
    ///   reported by `runlog::peak_rss_mb()` (a high-water
    ///   mark from `/proc/self/status`'s VmHWM), or `None`
    ///   when unavailable.
    ///
    /// Thresholds (both signals are judged against this
    /// profile's `paint_budget_bytes()`, B):
    ///
    /// - accounted: `>= B` is Critical; `>= 75% of B` is
    ///   Elevated; below that is Normal.
    /// - peak RSS: RSS covers far more than the accounted
    ///   caches (renderer, assets, the OS's share of the
    ///   iGPU's memory), so its bar is a multiple of B:
    ///   `>= 3×B` is Critical; `>= 2×B` is Elevated. On
    ///   LowMemory (B = 2 GiB) that is Elevated at
    ///   4096 MiB and Critical at 6144 MiB of peak RSS —
    ///   Critical lands just under the primary machine's
    ///   ~6.7 GiB of usable RAM, where it belongs.
    ///
    /// The result is the worse of the two signals. Both
    /// signals are threshold functions of their input, so
    /// for any fixed `peak_rss_mb` the result is monotonic
    /// non-decreasing in `accounted_bytes` (and vice
    /// versa). A `None`, NaN or negative RSS contributes
    /// no signal rather than a fabricated one. This is a
    /// pure judgement: it evicts, shrinks and blocks
    /// nothing by itself — acting on the level is the
    /// caller's decision.
    pub(crate) fn level(
        &self,
        accounted_bytes: u64,
        peak_rss_mb: Option<f64>,
    ) -> PressureLevel {
        let budget = self.paint_budget_bytes();
        let accounted_level = if accounted_bytes >= budget {
            PressureLevel::Critical
        } else if (accounted_bytes as u128) * 4 >= (budget as u128) * 3 {
            PressureLevel::Elevated
        } else {
            PressureLevel::Normal
        };
        let rss_level = match peak_rss_mb {
            Some(mb) if mb.is_finite() && mb >= 0.0 => {
                // MiB -> bytes. The float-to-int cast
                // saturates, so an absurd input can only
                // read as Critical, never wrap.
                let rss_bytes = (mb * 1_048_576.0) as u64;
                if (rss_bytes as u128) >= (budget as u128) * 3 {
                    PressureLevel::Critical
                } else if (rss_bytes as u128) >= (budget as u128) * 2 {
                    PressureLevel::Elevated
                } else {
                    PressureLevel::Normal
                }
            }
            _ => PressureLevel::Normal,
        };
        if rss_level > accounted_level {
            rss_level
        } else {
            accounted_level
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GIB_T: u64 = 1024 * 1024 * 1024;
    const LOW: MemoryProfile = MemoryProfile::LowMemory;
    const BAL: MemoryProfile = MemoryProfile::Balanced;
    const HIGH: MemoryProfile = MemoryProfile::HighPerformance;

    // ---- /proc/meminfo parsing --------------------------------

    #[test]
    fn meminfo_parses_kib_units_to_bytes() {
        // A realistic /proc/meminfo head. 16384256 KiB is
        // the classic ~16 GiB report: x1024, not x1000.
        let text = "MemTotal:       16384256 kB\n\
                    MemFree:         1234567 kB\n\
                    MemAvailable:    7654321 kB\n\
                    Buffers:          123456 kB\n";
        assert_eq!(
            parse_meminfo_total_bytes(text),
            Some(16_384_256 * 1024)
        );
        assert_eq!(parse_meminfo_total_bytes(text), Some(16_777_478_144));
        // The exact 8 GiB boundary value: 8388608 KiB.
        assert_eq!(
            parse_meminfo_total_bytes("MemTotal:        8388608 kB\n"),
            Some(8 * GIB_T)
        );
        // Column whitespace is the kernel's, and varies.
        assert_eq!(
            parse_meminfo_total_bytes("MemTotal: 1024 kB\n"),
            Some(1024 * 1024)
        );
    }

    #[test]
    fn meminfo_does_not_mistake_other_fields_for_total() {
        let text = "MemFree:         8388608 kB\n\
                    MemAvailable:    8388608 kB\n";
        assert_eq!(parse_meminfo_total_bytes(text), None);
        // MemTotal not at the start of a line does not count.
        assert_eq!(
            parse_meminfo_total_bytes("  MemTotal:       8388608 kB\n"),
            None
        );
    }

    #[test]
    fn meminfo_rejects_malformed_lines() {
        assert_eq!(parse_meminfo_total_bytes(""), None);
        assert_eq!(parse_meminfo_total_bytes("no totals here\n"), None);
        // Non-numeric value.
        assert_eq!(parse_meminfo_total_bytes("MemTotal: plenty kB\n"), None);
        // Wrong unit: the kernel's unit is kB (KiB); MB is
        // a different number and must not be read as KiB.
        assert_eq!(parse_meminfo_total_bytes("MemTotal: 8192 MB\n"), None);
        // Missing unit.
        assert_eq!(parse_meminfo_total_bytes("MemTotal: 8388608\n"), None);
        // Missing value.
        assert_eq!(parse_meminfo_total_bytes("MemTotal:\n"), None);
        // A value too large for u64 KiB fails, never wraps.
        assert_eq!(
            parse_meminfo_total_bytes("MemTotal: 99999999999999999999999999 kB\n"),
            None
        );
    }

    // ---- profile spelling parsing -----------------------------

    #[test]
    fn profile_spellings_parse() {
        assert_eq!(parse_profile("low"), Some(LOW));
        assert_eq!(parse_profile("low-memory"), Some(LOW));
        assert_eq!(parse_profile("Low_Memory"), Some(LOW));
        assert_eq!(parse_profile(" LOWMEMORY "), Some(LOW));
        assert_eq!(parse_profile("balanced"), Some(BAL));
        assert_eq!(parse_profile("Balanced"), Some(BAL));
        assert_eq!(parse_profile("high"), Some(HIGH));
        assert_eq!(parse_profile("high-performance"), Some(HIGH));
        assert_eq!(parse_profile("HIGH_PERFORMANCE"), Some(HIGH));
        assert_eq!(parse_profile("turbo"), None);
        assert_eq!(parse_profile(""), None);
        // The log spelling round-trips through the parser.
        for p in [LOW, BAL, HIGH] {
            assert_eq!(parse_profile(p.name()), Some(p));
        }
    }

    // ---- resolution precedence --------------------------------

    #[test]
    fn resolution_arg_beats_env_beats_machine_default() {
        let small_ram = Some(6 * GIB_T); // would default Low
        let big_ram = Some(16 * GIB_T); // would default Balanced
        // Arg wins over env and over the machine default.
        assert_eq!(
            resolve_profile(Some("high"), Some("low"), small_ram),
            HIGH
        );
        assert_eq!(
            resolve_profile(Some("low"), Some("high"), big_ram),
            LOW
        );
        // Env wins over the machine default when no arg.
        assert_eq!(resolve_profile(None, Some("high"), small_ram), HIGH);
        assert_eq!(resolve_profile(None, Some("low"), big_ram), LOW);
        // Neither: the machine default decides.
        assert_eq!(resolve_profile(None, None, small_ram), LOW);
        assert_eq!(resolve_profile(None, None, big_ram), BAL);
    }

    #[test]
    fn resolution_invalid_values_fall_through() {
        // A bad arg falls through to a good env value.
        assert_eq!(
            resolve_profile(Some("junk"), Some("low"), Some(16 * GIB_T)),
            LOW
        );
        // Bad arg and bad env fall through to the default.
        assert_eq!(
            resolve_profile(Some("junk"), Some("junk"), Some(6 * GIB_T)),
            LOW
        );
        assert_eq!(resolve_profile(Some(""), None, None), BAL);
    }

    #[test]
    fn machine_default_ram_boundary_is_8_gib_inclusive() {
        assert_eq!(default_profile_for_ram(Some(8 * GIB_T)), LOW);
        assert_eq!(default_profile_for_ram(Some(8 * GIB_T - 1)), LOW);
        assert_eq!(default_profile_for_ram(Some(8 * GIB_T + 1)), BAL);
        // The primary machine's class (~6.7 GiB usable; its
        // MemTotal reports a little more) is LowMemory.
        assert_eq!(default_profile_for_ram(Some(7 * GIB_T)), LOW);
        // Unknown RAM (probe failure) defaults to Balanced,
        // per contract C1 — never to a fabricated LowMemory.
        assert_eq!(default_profile_for_ram(None), BAL);
        // End to end through the parser: a 6 GiB meminfo
        // defaults Low, a 16 GiB meminfo defaults Balanced.
        let six = parse_meminfo_total_bytes("MemTotal:        6291456 kB\n");
        assert_eq!(resolve_profile(None, None, six), LOW);
        let sixteen = parse_meminfo_total_bytes("MemTotal:       16777216 kB\n");
        assert_eq!(resolve_profile(None, None, sixteen), BAL);
    }

    // ---- per-profile tables ------------------------------------

    #[test]
    fn capacity_preload_and_budget_table() {
        let low = MemPressure::from_profile(LOW);
        let bal = MemPressure::from_profile(BAL);
        let high = MemPressure::from_profile(HIGH);

        // Frame queue capacities: the contract's 2 / 4 / 8.
        assert_eq!(low.frame_queue_capacity(), 2);
        assert_eq!(bal.frame_queue_capacity(), 4);
        assert_eq!(high.frame_queue_capacity(), 8);

        // Preload: off only on LowMemory.
        assert!(!low.preload_enabled());
        assert!(bal.preload_enabled());
        assert!(high.preload_enabled());

        // Paint budgets: LowMemory is exactly paintbudget's
        // derived 2 GiB ceiling and never above it; the
        // other profiles are bounded multiples of it.
        assert_eq!(
            low.paint_budget_bytes(),
            crate::paintbudget::BUDGET_CEILING_BYTES
        );
        assert_eq!(low.paint_budget_bytes(), 2 * GIB_T);
        assert_eq!(bal.paint_budget_bytes(), 3 * GIB_T);
        assert_eq!(high.paint_budget_bytes(), 4 * GIB_T);
        assert!(low.paint_budget_bytes() < bal.paint_budget_bytes());
        assert!(bal.paint_budget_bytes() < high.paint_budget_bytes());

        // profile() round-trips the constructor's profile.
        assert_eq!(low.profile(), LOW);
        assert_eq!(bal.profile(), BAL);
        assert_eq!(high.profile(), HIGH);
    }

    // ---- level() ------------------------------------------------

    fn rank(l: PressureLevel) -> u8 {
        match l {
            PressureLevel::Normal => 0,
            PressureLevel::Elevated => 1,
            PressureLevel::Critical => 2,
        }
    }

    #[test]
    fn level_accounted_boundaries() {
        let mp = MemPressure::from_profile(LOW);
        let b = mp.paint_budget_bytes(); // 2 GiB
        assert_eq!(mp.level(0, None), PressureLevel::Normal);
        // Just below 75% of the budget: still Normal.
        assert_eq!(mp.level(b * 3 / 4 - 1, None), PressureLevel::Normal);
        // Exactly 75%: Elevated (the boundary is inclusive).
        assert_eq!(mp.level(b * 3 / 4, None), PressureLevel::Elevated);
        // Just below the full budget: still Elevated.
        assert_eq!(mp.level(b - 1, None), PressureLevel::Elevated);
        // Exactly the budget, and beyond: Critical —
        // reachable, not an asymptote.
        assert_eq!(mp.level(b, None), PressureLevel::Critical);
        assert_eq!(mp.level(b * 2, None), PressureLevel::Critical);
        assert_eq!(mp.level(u64::MAX, None), PressureLevel::Critical);
    }

    #[test]
    fn level_rss_boundaries() {
        let mp = MemPressure::from_profile(LOW);
        // LowMemory budget is 2048 MiB, so the RSS bars are
        // 2x = 4096 MiB (Elevated) and 3x = 6144 MiB
        // (Critical), judged with zero accounted bytes.
        assert_eq!(mp.level(0, Some(4095.0)), PressureLevel::Normal);
        assert_eq!(mp.level(0, Some(4096.0)), PressureLevel::Elevated);
        assert_eq!(mp.level(0, Some(6143.0)), PressureLevel::Elevated);
        assert_eq!(mp.level(0, Some(6144.0)), PressureLevel::Critical);
        // The worse signal wins in both directions.
        assert_eq!(
            mp.level(mp.paint_budget_bytes(), Some(0.0)),
            PressureLevel::Critical
        );
        assert_eq!(mp.level(0, Some(6144.0)), PressureLevel::Critical);
    }

    #[test]
    fn level_ignores_unusable_rss() {
        let mp = MemPressure::from_profile(BAL);
        // None / NaN / negative / infinite RSS contributes
        // no signal; the accounted signal alone decides.
        assert_eq!(mp.level(0, None), PressureLevel::Normal);
        assert_eq!(mp.level(0, Some(f64::NAN)), PressureLevel::Normal);
        assert_eq!(mp.level(0, Some(-1.0)), PressureLevel::Normal);
        assert_eq!(mp.level(0, Some(f64::INFINITY)), PressureLevel::Normal);
        let b = mp.paint_budget_bytes();
        assert_eq!(mp.level(b, Some(f64::NAN)), PressureLevel::Critical);
    }

    #[test]
    fn level_is_monotonic_in_accounted_bytes() {
        // For every profile, sweeping accounted bytes from
        // zero to twice the budget never lowers the level,
        // with no RSS signal and with a fixed Elevated one.
        for p in [LOW, BAL, HIGH] {
            let mp = MemPressure::from_profile(p);
            let b = mp.paint_budget_bytes();
            for rss in [None, Some(4096.0)] {
                let mut prev = 0u8;
                let mut accounted = 0u64;
                while accounted <= b * 2 {
                    let r = rank(mp.level(accounted, rss));
                    assert!(
                        r >= prev,
                        "profile {} rss {rss:?}: level fell at {accounted}",
                        p.name()
                    );
                    prev = r;
                    accounted += b / 16;
                }
                // The sweep must actually reach Critical via
                // the accounted signal alone at 2x budget.
                assert_eq!(mp.level(b * 2, None), PressureLevel::Critical);
            }
        }
    }
}
