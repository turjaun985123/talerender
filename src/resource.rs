//! Resource governor (GPUFIRST Batch 3, seat GOV-CORE) — the
//! process-wide RAM / GPU-submission / thread limits from the
//! user's 2026-11-18 resource-governance ruling: roughly
//! GPU <= 60% and RAM <= 60% of the machine, memory-safe, never
//! hang the PC, ahead of raw speed.
//!
//! # What is enforceable, honestly
//!
//! - RAM is DIRECTLY enforceable for the consumers this module
//!   feeds: budgets are byte counts, checked against accounted
//!   bytes. The film paint cache (film.rs `PaintCache`, via
//!   paintbudget.rs) takes its byte budget from here; the
//!   dyntex pool and the buffer pool take theirs from here too.
//! - A GPU utilisation percentage is NOT enforceable from
//!   userspace: wgpu exposes no utilisation knob, no duty-cycle
//!   API, and no way to read back how busy the device is. What
//!   this module provides for the GPU cap is an APPROXIMATION —
//!   submission pacing (a minimum gap between queue submissions,
//!   scaled by the headroom under the cap) plus a derived
//!   max-in-flight figure the (already serial) submit sites
//!   structurally satisfy. Every surface that mentions it —
//!   this file, the `governor:` log line, the README — labels
//!   it an approximation. It is never a hard GPU-% guarantee.
//! - Thread count is directly enforceable by the consumers
//!   that spawn threads; this module owns parsing/resolution
//!   (`thread_cap()`), GOV-PAR owns its use in telemetry.
//!
//! # Byte-identity law
//!
//! The governor changes speed and memory, NEVER pixels or
//! verdicts. Budgets only evict/recompute (painting is a pure
//! function of the exact state key, so a repaint is identical),
//! pacing only delays submissions, and no governor value feeds
//! a painter, a threshold, or a verdict. Governor-on at the
//! default caps and `TALERENDER_GOVERNOR=off` therefore produce
//! identical frame hashes by construction.
//!
//! # Configuration (rider 2026-11-18 12:01: usable custom
//! configuration)
//!
//! One precedence chain, resolved ONCE in `resolve`:
//! CLI flags > env vars > config file > built-in defaults.
//!
//! - Flags: `--max-gpu-pct`, `--max-ram-pct`, `--max-threads`
//!   (parsed + validated in main.rs, handed in via `configure`).
//! - Env: `TALERENDER_MAX_GPU_PCT`, `TALERENDER_MAX_RAM_PCT`,
//!   `TALERENDER_MAX_THREADS`; kill switch
//!   `TALERENDER_GOVERNOR=off` (enforcement off, reporting on).
//! - Config file keys: `max_gpu_pct`, `max_ram_pct`,
//!   `max_threads` in the hand-parsed `key = value` file the
//!   crate already uses (config.rs). Location: the `--config
//!   <path>` file when given, else `./talerender.toml`, else
//!   `$XDG_CONFIG_HOME/talerender/talerender.toml`, else
//!   `~/.config/talerender/talerender.toml`. A missing file is
//!   never an error.
//! - Defaults: GPU 60, RAM 60, threads = half the logical
//!   cores (min 1) — the user's numbers.
//!
//! Invalid values in ANY layer are a hard CLI-style error
//! (`error: ...` + help hint, exit 2, via `crate::cli_error`)
//! — never a panic, never a silent ignore. The one clamp:
//! a requested thread count above the logical core count is
//! clamped to the core count (extra threads cannot run in
//! parallel on this machine; they would only contend), and
//! the clamp is PRINTED on the `governor:` line.
//!
//! Zero new dependencies: std only; total RAM is parsed from
//! /proc/meminfo by reusing mempressure.rs's tested parser.

use std::path::PathBuf;
use std::sync::OnceLock;

/// Built-in defaults — the user's numbers (2026-11-18).
pub(crate) const DEFAULT_GPU_CAP_PCT: u32 = 60;
pub(crate) const DEFAULT_RAM_CAP_PCT: u32 = 60;

/// Conservative total-RAM fallback when /proc/meminfo cannot
/// be read or parsed (non-Linux, restricted /proc): 4 GiB.
/// Deliberately LOW: a budget derived from it under-allocates
/// on any real machine rather than over-committing RAM the
/// governor cannot prove exists. `total_ram_detected=false`
/// on the `governor:` line says when this fallback is in use.
pub(crate) const FALLBACK_TOTAL_RAM_BYTES: u64 = 4 * 1024 * 1024 * 1024;

/// Share of the RAM budget granted to the film paint cache:
/// 50%. The paint cache is by far the largest host-RAM
/// consumer (one canvas is ~29 MB; its pre-governor ceiling
/// was 2 GiB). Half the RAM budget keeps it dominant but
/// leaves the other half for everything else the budget must
/// cover on an integrated GPU (one shared pool): renderer
/// targets (~221 MB accounted in the gpucore ledger at
/// 1080x1920), the dyntex pool (10% share, below), assets,
/// and the streamed frame buffers.
pub(crate) const PAINT_CACHE_SHARE_PCT: u64 = 50;

/// Share of the RAM budget granted to the dyntex card-texture
/// pool: 10% (each card texture is one canvas, ~29 MB, and a
/// frame holds only a handful of concurrent characters).
pub(crate) const DYNTEX_SHARE_PCT: u64 = 10;

/// Share of the RAM budget granted to the readback buffer
/// pool: 5%, additionally capped at bufferpool.rs's own
/// 64 MiB default in `BufferPool::from_governor`.
pub(crate) const BUFFER_POOL_SHARE_PCT: u64 = 5;

/// CLI-layer values handed in by main.rs via `configure`,
/// already parsed + validated there (main owns the CLI error
/// surface). `None` = flag not given.
#[derive(Clone, Debug, Default)]
pub(crate) struct CliLimits {
    pub gpu_pct: Option<u32>,
    pub ram_pct: Option<u32>,
    pub threads: Option<usize>,
    pub config_path: Option<PathBuf>,
}

static CLI_LIMITS: OnceLock<CliLimits> = OnceLock::new();
static GOVERNOR: OnceLock<Governor> = OnceLock::new();

/// Hand the CLI layer's parsed values to the governor. Called
/// by main.rs BEFORE the first `governor()` call; a second
/// call is ignored (the first configuration wins, matching
/// the process-wide-once semantics of `governor()`).
pub(crate) fn configure(limits: CliLimits) {
    let _ = CLI_LIMITS.set(limits);
}

/// The process-wide governor, initialised once from the
/// configured CLI values + env + config file + machine. An
/// invalid env or config-file value is a CLI-style error
/// (exit 2) — see the module header.
pub(crate) fn governor() -> &'static Governor {
    GOVERNOR.get_or_init(|| match build_from_process() {
        Ok(g) => g,
        Err(msg) => crate::cli_error(&msg),
    })
}

/// The resolved, effective limits for this process. Immutable
/// after construction: every consumer sees the same numbers.
pub(crate) struct Governor {
    enabled: bool,
    gpu_cap_pct: u32,
    ram_cap_pct: u32,
    total_ram_bytes: u64,
    total_ram_detected: bool,
    thread_cap: usize,
    logical_cores: usize,
    gpu_source: &'static str,
    ram_source: &'static str,
    threads_source: &'static str,
    /// Human-readable clamp note, e.g. `threads:99->12`;
    /// `None` when nothing was clamped.
    clamp_note: Option<String>,
    config_file: Option<String>,
}

impl Governor {
    /// Enforcement on/off. `TALERENDER_GOVERNOR=off` turns
    /// enforcement off; reporting (this struct, the log line)
    /// continues either way.
    pub(crate) fn enabled(&self) -> bool {
        self.enabled
    }

    pub(crate) fn gpu_cap_pct(&self) -> u32 {
        self.gpu_cap_pct
    }

    pub(crate) fn ram_cap_pct(&self) -> u32 {
        self.ram_cap_pct
    }

    /// Total RAM the budget derives from: the /proc/meminfo
    /// MemTotal when detected, else FALLBACK_TOTAL_RAM_BYTES.
    pub(crate) fn total_ram_bytes(&self) -> u64 {
        self.total_ram_bytes
    }

    pub(crate) fn total_ram_detected(&self) -> bool {
        self.total_ram_detected
    }

    /// The RAM budget: `ram_cap_pct`% of total RAM. On an
    /// integrated GPU this one budget governs host RAM and
    /// GPU allocations together — they are the same pool.
    pub(crate) fn ram_budget_bytes(&self) -> u64 {
        ((self.total_ram_bytes as u128 * self.ram_cap_pct as u128) / 100) as u64
    }

    /// The thread cap GOV-PAR consumes: the resolved limit,
    /// already clamped to the logical core count.
    pub(crate) fn thread_cap(&self) -> usize {
        self.thread_cap
    }

    pub(crate) fn logical_cores(&self) -> usize {
        self.logical_cores
    }

    /// Byte budget for the film paint cache: PAINT_CACHE_SHARE
    /// of the RAM budget (see the share constant's rationale).
    pub(crate) fn paint_cache_budget_bytes(&self) -> u64 {
        self.ram_budget_bytes() * PAINT_CACHE_SHARE_PCT / 100
    }

    /// Byte budget for the dyntex pool (DYNTEX_SHARE of the
    /// RAM budget).
    pub(crate) fn dyntex_budget_bytes(&self) -> u64 {
        self.ram_budget_bytes() * DYNTEX_SHARE_PCT / 100
    }

    /// Byte budget for the readback buffer pool
    /// (BUFFER_POOL_SHARE of the RAM budget).
    pub(crate) fn buffer_pool_budget_bytes(&self) -> u64 {
        self.ram_budget_bytes() * BUFFER_POOL_SHARE_PCT / 100
    }

    /// GPU pacing, APPROXIMATION (see module header): the
    /// max in-flight submission count derived from the GPU
    /// cap — 1 per started 25% of cap, 1..=4. The tree's
    /// submit sites are serial (each submit is followed by a
    /// completion wait), so in-flight is structurally 1 and
    /// always within this bound; the bound is reported, and
    /// `pace_gpu_submission` below is the active mechanism.
    pub(crate) fn gpu_max_in_flight(&self) -> u32 {
        (self.gpu_cap_pct.saturating_add(24) / 25).clamp(1, 4)
    }

    /// GPU pacing, APPROXIMATION: the minimum gap between
    /// queue submissions, scaled by the headroom under the
    /// cap — one 10 ms pacing quantum x (100 - pct) / 100,
    /// truncated to whole ms (60% -> 4 ms, 90% -> 1 ms,
    /// 100% or governor-off -> 0). Throttling the submission
    /// rate throttles the GPU's duty cycle only roughly —
    /// a submission's GPU duration is not known in advance —
    /// which is exactly why this is labelled an approximation
    /// everywhere it appears.
    pub(crate) fn gpu_pace_gap_ms(&self) -> u64 {
        if !self.enabled || self.gpu_cap_pct >= 100 {
            0
        } else {
            (100 - self.gpu_cap_pct) as u64 / 10
        }
    }

    /// The one greppable effective-limits line, printed at
    /// startup (stderr, by main) and beside the mem-report
    /// block (stdout, by memreport_print). Names every
    /// effective value, its source layer when it is not the
    /// plain default mix, and any clamp applied.
    pub(crate) fn effective_line(&self) -> String {
        format!(
            "governor: enabled={} gpu_cap_pct={} gpu_source={} ram_cap_pct={} ram_source={} ram_budget_bytes={} total_ram_bytes={} total_ram_detected={} threads={} threads_source={} logical_cores={} clamp={} config_file={} gpu_pacing=approximation max_in_flight={} pace_gap_ms={} paint_cache_budget_bytes={} dyntex_budget_bytes={} buffer_pool_budget_bytes={}",
            self.enabled,
            self.gpu_cap_pct,
            self.gpu_source,
            self.ram_cap_pct,
            self.ram_source,
            self.ram_budget_bytes(),
            self.total_ram_bytes,
            self.total_ram_detected,
            self.thread_cap,
            self.threads_source,
            self.logical_cores,
            self.clamp_note.as_deref().unwrap_or("none"),
            self.config_file.as_deref().unwrap_or("none"),
            self.gpu_max_in_flight(),
            self.gpu_pace_gap_ms(),
            self.paint_cache_budget_bytes(),
            self.dyntex_budget_bytes(),
            self.buffer_pool_budget_bytes(),
        )
    }
}

/// GPU submission pacing, APPROXIMATION (see
/// `Governor::gpu_pace_gap_ms`): called immediately before a
/// `queue.submit` on the gpupaint replay path and the film
/// render path. Sleeps only the remainder of the pacing gap
/// since the previous paced submission in this process;
/// a no-op when the governor is off or the cap is 100%.
/// Delaying a submission cannot change its contents, so
/// this is pixel-neutral by construction. Never panics
/// (a poisoned lock is recovered, as in gpucore's ledger).
pub(crate) fn pace_gpu_submission() {
    let gap_ms = governor().gpu_pace_gap_ms();
    if gap_ms == 0 {
        return;
    }
    static LAST: std::sync::Mutex<Option<std::time::Instant>> = std::sync::Mutex::new(None);
    let mut guard = LAST.lock().unwrap_or_else(|e| e.into_inner());
    let gap = std::time::Duration::from_millis(gap_ms);
    if let Some(last) = *guard {
        let elapsed = last.elapsed();
        if elapsed < gap {
            std::thread::sleep(gap - elapsed);
        }
    }
    *guard = Some(std::time::Instant::now());
}

// ---------------------------------------------------------------------
// Resolution (pure core, unit-tested; the process wrapper is thin)
// ---------------------------------------------------------------------

/// Everything `resolve` needs, gathered from the process by
/// `build_from_process` and fabricated directly by tests.
struct RawInputs {
    cli: CliLimits,
    env_gpu: Option<String>,
    env_ram: Option<String>,
    env_threads: Option<String>,
    env_governor: Option<String>,
    file_pairs: Vec<(String, String)>,
    config_file: Option<String>,
    total_ram_bytes: Option<u64>,
    logical_cores: usize,
}

/// Parse one percentage value: an integer in 1..=100.
/// Anything else is an error naming the surface it came from.
fn parse_pct(raw: &str, surface: &str) -> Result<u32, String> {
    let v: u32 = raw
        .trim()
        .parse()
        .map_err(|_| format!("invalid value for {surface}: '{raw}' (expected an integer percentage, 1..=100)"))?;
    if !(1..=100).contains(&v) {
        return Err(format!(
            "invalid value for {surface}: '{raw}' (expected an integer percentage, 1..=100)"
        ));
    }
    Ok(v)
}

/// Parse one thread-count value: an integer >= 1.
fn parse_threads(raw: &str, surface: &str) -> Result<usize, String> {
    let v: usize = raw
        .trim()
        .parse()
        .map_err(|_| format!("invalid value for {surface}: '{raw}' (expected a positive integer)"))?;
    if v == 0 {
        return Err(format!(
            "invalid value for {surface}: '{raw}' (expected a positive integer)"
        ));
    }
    Ok(v)
}

/// Resolve one percentage across CLI > env > file > default.
fn resolve_pct(
    cli: Option<u32>,
    env: Option<String>,
    env_name: &str,
    file: Option<&str>,
    file_key: &str,
    default: u32,
) -> Result<(u32, &'static str), String> {
    if let Some(v) = cli {
        return Ok((v, "cli"));
    }
    if let Some(raw) = env {
        if !raw.trim().is_empty() {
            return Ok((parse_pct(&raw, env_name)?, "env"));
        }
    }
    if let Some(raw) = file {
        return Ok((parse_pct(raw, &format!("config key {file_key}"))?, "config"));
    }
    Ok((default, "default"))
}

/// Resolve the thread count across CLI > env > file > default
/// (half the logical cores, min 1), then apply the one clamp:
/// never above the logical core count (printed by the caller
/// via the returned note).
fn resolve_threads(
    cli: Option<usize>,
    env: Option<String>,
    file: Option<&str>,
    logical_cores: usize,
) -> Result<(usize, &'static str, Option<String>), String> {
    let cores = logical_cores.max(1);
    let (requested, source) = if let Some(v) = cli {
        (v, "cli")
    } else if let Some(raw) = env.filter(|s| !s.trim().is_empty()) {
        (parse_threads(&raw, "TALERENDER_MAX_THREADS")?, "env")
    } else if let Some(raw) = file {
        (parse_threads(raw, "config key max_threads")?, "config")
    } else {
        (cores / 2, "default")
    };
    let requested = requested.max(1);
    if requested > cores {
        Ok((cores, source, Some(format!("threads:{requested}->{cores}"))))
    } else {
        Ok((requested, source, None))
    }
}

/// The full precedence chain in one place. Pure: no env, no
/// filesystem, no globals — every input arrives in `raw`.
fn resolve(raw: RawInputs) -> Result<Governor, String> {
    let enabled = match raw.env_governor.as_deref() {
        None => true,
        Some(v) => match v.trim().to_ascii_lowercase().as_str() {
            "" | "on" | "1" | "true" => true,
            "off" | "0" | "false" => false,
            _ => {
                return Err(format!(
                    "invalid value for TALERENDER_GOVERNOR: '{v}' (expected 'off' or 'on')"
                ))
            }
        },
    };
    let (gpu_cap_pct, gpu_source) = resolve_pct(
        raw.cli.gpu_pct,
        raw.env_gpu,
        "TALERENDER_MAX_GPU_PCT",
        crate::config::lookup(&raw.file_pairs, "max_gpu_pct"),
        "max_gpu_pct",
        DEFAULT_GPU_CAP_PCT,
    )?;
    let (ram_cap_pct, ram_source) = resolve_pct(
        raw.cli.ram_pct,
        raw.env_ram,
        "TALERENDER_MAX_RAM_PCT",
        crate::config::lookup(&raw.file_pairs, "max_ram_pct"),
        "max_ram_pct",
        DEFAULT_RAM_CAP_PCT,
    )?;
    let (thread_cap, threads_source, clamp_note) = resolve_threads(
        raw.cli.threads,
        raw.env_threads,
        crate::config::lookup(&raw.file_pairs, "max_threads"),
        raw.logical_cores,
    )?;
    let (total_ram_bytes, total_ram_detected) = match raw.total_ram_bytes {
        Some(n) if n > 0 => (n, true),
        _ => (FALLBACK_TOTAL_RAM_BYTES, false),
    };
    Ok(Governor {
        enabled,
        gpu_cap_pct,
        ram_cap_pct,
        total_ram_bytes,
        total_ram_detected,
        thread_cap,
        logical_cores: raw.logical_cores.max(1),
        gpu_source,
        ram_source,
        threads_source,
        clamp_note,
        config_file: raw.config_file,
    })
}

/// Discover the config file to read, in order: the explicit
/// `--config` path, `./talerender.toml`, then the per-user
/// XDG location. Returns the path + parsed pairs, or
/// (None, empty) when no file exists — a missing file is
/// never an error. An existing-but-unreadable file is also
/// treated as absent (the existing config system's posture);
/// malformed governor VALUES inside a readable file are
/// errors, raised by `resolve`.
fn discover_config_file(
    explicit: Option<&std::path::Path>,
) -> (Option<String>, Vec<(String, String)>) {
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Some(p) = explicit {
        // An explicit --config names THE file (config.rs's own
        // posture): if it is missing/unreadable there is no
        // governor file layer, never a silent substitute.
        candidates.push(p.to_path_buf());
    } else {
        candidates.push(PathBuf::from(crate::config::DEFAULT_CONFIG_FILE));
        let xdg_base = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")));
        if let Some(base) = xdg_base {
            candidates.push(base.join("talerender").join("talerender.toml"));
        }
    }
    for path in candidates {
        if let Ok(text) = std::fs::read_to_string(&path) {
            return (
                Some(path.display().to_string()),
                crate::config::parse_config_file(&text),
            );
        }
    }
    (None, Vec::new())
}

/// Gather the process inputs and resolve. The only place
/// (besides `governor()`'s error exit) that touches env,
/// /proc, and the filesystem for governor purposes.
fn build_from_process() -> Result<Governor, String> {
    let cli = CLI_LIMITS.get().cloned().unwrap_or_default();
    let (config_file, file_pairs) = discover_config_file(cli.config_path.as_deref());
    let total_ram_bytes = std::fs::read_to_string("/proc/meminfo")
        .ok()
        .and_then(|text| crate::mempressure::parse_meminfo_total_bytes(&text));
    let logical_cores = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);
    resolve(RawInputs {
        env_gpu: std::env::var("TALERENDER_MAX_GPU_PCT").ok(),
        env_ram: std::env::var("TALERENDER_MAX_RAM_PCT").ok(),
        env_threads: std::env::var("TALERENDER_MAX_THREADS").ok(),
        env_governor: std::env::var("TALERENDER_GOVERNOR").ok(),
        file_pairs,
        config_file,
        total_ram_bytes,
        logical_cores,
        cli,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const GIB: u64 = 1024 * 1024 * 1024;

    fn raw() -> RawInputs {
        RawInputs {
            cli: CliLimits::default(),
            env_gpu: None,
            env_ram: None,
            env_threads: None,
            env_governor: None,
            file_pairs: Vec::new(),
            config_file: None,
            total_ram_bytes: Some(16 * GIB),
            logical_cores: 12,
        }
    }

    #[test]
    fn defaults_are_the_users_numbers() {
        let g = resolve(raw()).expect("resolve");
        assert!(g.enabled());
        assert_eq!(g.gpu_cap_pct(), 60);
        assert_eq!(g.ram_cap_pct(), 60);
        assert_eq!(g.thread_cap(), 6); // half of 12 cores
        assert_eq!(g.ram_budget_bytes(), 16 * GIB * 60 / 100);
        assert_eq!(g.paint_cache_budget_bytes(), g.ram_budget_bytes() / 2);
        assert_eq!(g.dyntex_budget_bytes(), g.ram_budget_bytes() / 10);
        assert_eq!(g.buffer_pool_budget_bytes(), g.ram_budget_bytes() / 20);
    }

    #[test]
    fn precedence_cli_beats_env_beats_file_beats_default() {
        let mut r = raw();
        r.file_pairs = crate::config::parse_config_file("max_ram_pct = 30\nmax_gpu_pct = 31\n");
        assert_eq!(resolve(r).expect("file").ram_cap_pct(), 30);
        let mut r = raw();
        r.file_pairs = crate::config::parse_config_file("max_ram_pct = 30\n");
        r.env_ram = Some("45".to_string());
        let g = resolve(r).expect("env over file");
        assert_eq!(g.ram_cap_pct(), 45);
        let mut r = raw();
        r.env_ram = Some("45".to_string());
        r.cli.ram_pct = Some(70);
        let g = resolve(r).expect("cli over env");
        assert_eq!(g.ram_cap_pct(), 70);
        assert!(g.effective_line().contains("ram_source=cli"));
    }

    #[test]
    fn threads_precedence_and_default_half_cores_min_one() {
        let mut r = raw();
        r.logical_cores = 1;
        assert_eq!(resolve(r).expect("one core").thread_cap(), 1);
        let mut r = raw();
        r.file_pairs = crate::config::parse_config_file("max_threads = 3\n");
        r.env_threads = Some("4".to_string());
        assert_eq!(resolve(r).expect("env threads").thread_cap(), 4);
        let mut r = raw();
        r.file_pairs = crate::config::parse_config_file("max_threads = 3\n");
        assert_eq!(resolve(r).expect("file threads").thread_cap(), 3);
    }

    #[test]
    fn thread_request_above_cores_is_clamped_and_noted() {
        let mut r = raw();
        r.cli.threads = Some(99);
        let g = resolve(r).expect("clamp");
        assert_eq!(g.thread_cap(), 12);
        assert!(
            g.effective_line().contains("clamp=threads:99->12"),
            "line: {}",
            g.effective_line()
        );
    }

    #[test]
    fn invalid_values_are_errors_never_panics() {
        for bad in ["0", "101", "-5", "abc", "60.5", ""] {
            let mut r = raw();
            // Empty env is treated as unset (workflow-friendly);
            // every other malformed value must error.
            if bad.is_empty() {
                continue;
            }
            r.env_ram = Some(bad.to_string());
            assert!(resolve(r).is_err(), "env ram '{bad}' must error");
        }
        let mut r = raw();
        r.env_threads = Some("0".to_string());
        assert!(resolve(r).is_err());
        let mut r = raw();
        r.file_pairs = crate::config::parse_config_file("max_gpu_pct = lots\n");
        assert!(resolve(r).is_err());
        let mut r = raw();
        r.env_governor = Some("maybe".to_string());
        assert!(resolve(r).is_err());
    }

    #[test]
    fn governor_off_disables_enforcement_not_reporting() {
        let mut r = raw();
        r.env_governor = Some("off".to_string());
        let g = resolve(r).expect("off");
        assert!(!g.enabled());
        assert_eq!(g.gpu_pace_gap_ms(), 0);
        assert!(g.effective_line().contains("enabled=false"));
        // Budgets still resolve and report.
        assert_eq!(g.ram_cap_pct(), 60);
    }

    #[test]
    fn ram_detection_failure_uses_the_documented_fallback() {
        let mut r = raw();
        r.total_ram_bytes = None;
        let g = resolve(r).expect("fallback");
        assert_eq!(g.total_ram_bytes(), FALLBACK_TOTAL_RAM_BYTES);
        assert!(!g.total_ram_detected());
        assert_eq!(g.ram_budget_bytes(), FALLBACK_TOTAL_RAM_BYTES * 60 / 100);
        assert!(g.effective_line().contains("total_ram_detected=false"));
    }

    #[test]
    fn budget_math_uses_total_not_a_hardcoded_machine() {
        let mut r = raw();
        r.total_ram_bytes = Some(8 * GIB);
        r.cli.ram_pct = Some(25);
        let g = resolve(r).expect("math");
        assert_eq!(g.ram_budget_bytes(), 2 * GIB);
        assert_eq!(g.paint_cache_budget_bytes(), GIB);
    }

    #[test]
    fn gpu_pacing_is_derived_from_the_cap() {
        // APPROXIMATION figures: in-flight 1 per started 25%,
        // gap = headroom quantum. 60% -> 3 in flight, 4 ms.
        let g = resolve(raw()).expect("60");
        assert_eq!(g.gpu_max_in_flight(), 3);
        assert_eq!(g.gpu_pace_gap_ms(), 4);
        let mut r = raw();
        r.cli.gpu_pct = Some(100);
        let g = resolve(r).expect("100");
        assert_eq!(g.gpu_max_in_flight(), 4);
        assert_eq!(g.gpu_pace_gap_ms(), 0);
        let mut r = raw();
        r.cli.gpu_pct = Some(10);
        let g = resolve(r).expect("10");
        assert_eq!(g.gpu_max_in_flight(), 1);
        assert_eq!(g.gpu_pace_gap_ms(), 9);
    }

    #[test]
    fn effective_line_names_every_limit_and_source() {
        let mut r = raw();
        r.cli.gpu_pct = Some(55);
        r.env_ram = Some("65".to_string());
        let line = resolve(r).expect("line").effective_line();
        assert!(line.starts_with("governor: "), "got: {line}");
        for needle in [
            "gpu_cap_pct=55",
            "gpu_source=cli",
            "ram_cap_pct=65",
            "ram_source=env",
            "threads=6",
            "gpu_pacing=approximation",
            "ram_budget_bytes=",
            "paint_cache_budget_bytes=",
        ] {
            assert!(line.contains(needle), "line missing {needle}: {line}");
        }
    }

    #[test]
    fn pct_parser_bounds() {
        assert_eq!(parse_pct("1", "x").expect("1"), 1);
        assert_eq!(parse_pct(" 100 ", "x").expect("100"), 100);
        assert!(parse_pct("0", "x").is_err());
        assert!(parse_pct("101", "x").is_err());
        assert!(parse_threads("3", "x").expect("3") == 3);
        assert!(parse_threads("0", "x").is_err());
    }
}
