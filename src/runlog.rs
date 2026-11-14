//! Run timing + process telemetry for logs (Wave 2 seat W09;
//! PERF.md §9 items 1 and 3).
//!
//! Division of labour, by contract with W06: W06's cache work
//! *counts* paint-cache events (hits / misses / evictions /
//! bytes); this module owns *time and process memory* — a
//! monotonic run clock, the progress/summary line formats, and
//! the peak-RSS self-report. Neither side formats the other's
//! numbers; the end-of-run line composes both (order in
//! `RunClock::summary_suffix`).
//!
//! Everything here is log-only: no return value feeds a pixel,
//! a verdict, or a gate, so the determinism law (a frame is a
//! pure function of its inputs) is untouched. Elapsed times are
//! expected to differ between runs; rendered frames must not.
//!
//! std only, no new dependencies. Peak RSS reads
//! `/proc/self/status` (Linux); off-Linux it reports None and
//! the summary says so rather than inventing a number.
//!
//! Wiring (lead): `mod runlog;` in main.rs, then the three call
//! sites in docs/integration/RUNLOG-W09.md.

use std::time::{Duration, Instant};

/// Format seconds as `MM:SS`. Minutes are total minutes —
/// zero-padded to two digits and growing past 59 on long runs
/// (5 845 s → `97:25`) — so the format never changes shape
/// mid-log. Seconds round to the nearest whole second.
/// Non-finite or negative input renders `--:--`, the same
/// sentinel the ETA uses before a rate exists, so a log reader
/// treats one placeholder uniformly.
pub fn clock_str(secs: f64) -> String {
    if !secs.is_finite() || secs < 0.0 {
        return "--:--".to_string();
    }
    let total = secs.round() as u64;
    format!("{:02}:{:02}", total / 60, total % 60)
}

/// Completion percentage of `done` out of `total` work units.
/// A zero total is defined as 0.0: a run with no work has no
/// meaningful rate (and the loops that print progress always
/// have ≥ 1 unit, so this is a definition, not a display path).
pub fn percent(done: u64, total: u64) -> f64 {
    if total == 0 {
        0.0
    } else {
        done as f64 * 100.0 / total as f64
    }
}

/// Estimated seconds remaining at the observed rate,
/// `elapsed × (total − done) / done`. None when no rate exists
/// yet (`done == 0`) — callers render the `--:--` placeholder
/// rather than a fabricated number. Some(0.0) once
/// `done >= total`.
pub fn eta_secs(elapsed_secs: f64, done: u64, total: u64) -> Option<f64> {
    if done == 0 {
        None
    } else if done >= total {
        Some(0.0)
    } else {
        Some(elapsed_secs * (total - done) as f64 / done as f64)
    }
}

/// A monotonic clock for one run (one film render, one
/// telemetry pass). Construct it where the run's work starts;
/// ask it for line segments as the run progresses. Cheap: one
/// `Instant`, no allocation except the formatted strings.
pub struct RunClock {
    start: Instant,
}

impl RunClock {
    pub fn new() -> Self {
        RunClock {
            start: Instant::now(),
        }
    }

    pub fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }

    pub fn elapsed_secs(&self) -> f64 {
        self.elapsed().as_secs_f64()
    }

    /// `elapsed 03:12` — the segment PERF §9.1 asks progress
    /// lines to carry.
    pub fn elapsed_part(&self) -> String {
        format!("elapsed {}", clock_str(self.elapsed_secs()))
    }

    /// `eta 02:24` at the observed done-rate, or `eta --:--`
    /// before the first unit completes.
    ///
    /// `done`/`total` are in whatever work units the caller
    /// measures. For the film driver the note recommends
    /// *frames visited* (rendered + skipped) against the
    /// requested range length: skips are ~free, so a resume
    /// run's early ETA is optimistic and converges as real
    /// renders dominate; on a fresh run skipped = 0 and the
    /// basis is exact.
    pub fn eta_part(&self, done: u64, total: u64) -> String {
        match eta_secs(self.elapsed_secs(), done, total) {
            Some(s) => format!("eta {}", clock_str(s)),
            None => "eta --:--".to_string(),
        }
    }

    /// The standalone progress line:
    /// `frame 1051/1811 (58.0%) elapsed 03:12 eta 02:24`.
    /// Provided for modes that print a bare progress line. The
    /// film driver does NOT switch to it — it appends
    /// `timing_suffix` to its existing line so the format
    /// quoted in PERF.md §2 keeps its head (see RUNLOG-W09.md).
    pub fn elapsed_line(&self, done: u64, total: u64) -> String {
        format!(
            "frame {done}/{total} ({:.1}%) {} {}",
            percent(done, total),
            self.elapsed_part(),
            self.eta_part(done, total)
        )
    }

    /// `elapsed 03:12 · eta 02:24` — the segment to append to
    /// an existing progress line, after its own counts.
    pub fn timing_suffix(&self, done: u64, total: u64) -> String {
        format!("{} · {}", self.elapsed_part(), self.eta_part(done, total))
    }

    /// End-of-run segment: `elapsed 03:12 · peak RSS 512.3 MB`
    /// (or `peak RSS unavailable` off-Linux). Composition order
    /// for the completion line, by contract: the run's existing
    /// counts first, then this suffix (time, then memory), then
    /// W06's cache stats segment last. Peak RSS is the process
    /// high-water mark at call time, so call this as late in
    /// the run as the line allows.
    pub fn summary_suffix(&self) -> String {
        match peak_rss_mb() {
            Some(mb) => format!("{} · peak RSS {mb:.1} MB", self.elapsed_part()),
            None => format!("{} · peak RSS unavailable", self.elapsed_part()),
        }
    }
}

impl Default for RunClock {
    fn default() -> Self {
        Self::new()
    }
}

/// Whether the optional per-frame timing line (PERF §9.1's
/// second half) is enabled: `TALERENDER_FRAME_TIMING` set to
/// anything but `0`. The render loop reads this once and emits
/// the per-frame millisecond lines itself; this helper is only
/// the switch so the env name lives in exactly one place.
pub fn frame_timing_enabled() -> bool {
    std::env::var("TALERENDER_FRAME_TIMING")
        .map(|v| v != "0")
        .unwrap_or(false)
}

/// This process's peak resident set size in MiB (VmHWM from
/// `/proc/self/status`), or None off-Linux or on any read/parse
/// failure. Self-reported, so the signal survives launchers and
/// wrappers that lose `/usr/bin/time -v`'s numbers (PERF §9.3).
/// It is a process-wide high-water mark: in a multi-run process
/// it can only grow, never reset.
pub fn peak_rss_mb() -> Option<f64> {
    #[cfg(target_os = "linux")]
    {
        let text = std::fs::read_to_string("/proc/self/status").ok()?;
        parse_vmhwm_mb(&text)
    }
    #[cfg(not(target_os = "linux"))]
    {
        None
    }
}

/// Parse the `VmHWM:` line of `/proc/self/status` text into
/// MiB. The kernel reports KiB (`kB` in the file); 1 MiB =
/// 1024 KiB. Tolerant of the kernel's column whitespace,
/// strict about the unit suffix. Exposed for tests.
pub fn parse_vmhwm_mb(status: &str) -> Option<f64> {
    for line in status.lines() {
        if let Some(rest) = line.strip_prefix("VmHWM:") {
            let kib: f64 = rest.trim().strip_suffix("kB")?.trim().parse().ok()?;
            return Some(kib / 1024.0);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clock_str_formats_mm_ss() {
        assert_eq!(clock_str(0.0), "00:00");
        assert_eq!(clock_str(9.0), "00:09");
        assert_eq!(clock_str(192.0), "03:12"); // the PERF §9 example value
        assert_eq!(clock_str(144.0), "02:24");
        assert_eq!(clock_str(3599.0), "59:59");
        assert_eq!(clock_str(3600.0), "60:00"); // minutes grow past 59
        assert_eq!(clock_str(5845.0), "97:25");
    }

    #[test]
    fn clock_str_rounds_to_nearest_second() {
        assert_eq!(clock_str(192.4), "03:12");
        assert_eq!(clock_str(192.6), "03:13");
        assert_eq!(clock_str(59.6), "01:00"); // rounding carries into minutes
    }

    #[test]
    fn clock_str_sentinel_for_bad_input() {
        assert_eq!(clock_str(-1.0), "--:--");
        assert_eq!(clock_str(f64::NAN), "--:--");
        assert_eq!(clock_str(f64::INFINITY), "--:--");
    }

    #[test]
    fn percent_math() {
        assert_eq!(percent(0, 1811), 0.0);
        assert_eq!(percent(1811, 1811), 100.0);
        assert!((percent(1051, 1811) - 58.034_2).abs() < 0.001);
        assert_eq!(percent(5, 0), 0.0); // zero-total definition
    }

    #[test]
    fn eta_edges() {
        assert_eq!(eta_secs(100.0, 0, 1811), None); // no rate yet
        assert_eq!(eta_secs(100.0, 1811, 1811), Some(0.0)); // complete
        assert_eq!(eta_secs(100.0, 2000, 1811), Some(0.0)); // over-complete
        assert_eq!(eta_secs(100.0, 50, 100), Some(100.0)); // half done → half left
        // 1051 of 1811 done in 192 s → 192 × 760/1051 ≈ 138.84 s left
        let eta = eta_secs(192.0, 1051, 1811).unwrap();
        assert!((eta - 138.84).abs() < 0.01);
    }

    #[test]
    fn elapsed_line_format_and_eta_sentinel() {
        let clock = RunClock::new();
        // A fresh clock: elapsed rounds to 00:00, and with no
        // units done the ETA must be the placeholder, end to end.
        assert_eq!(
            clock.elapsed_line(0, 1811),
            "frame 0/1811 (0.0%) elapsed 00:00 eta --:--"
        );
        assert_eq!(
            clock.elapsed_line(1811, 1811),
            "frame 1811/1811 (100.0%) elapsed 00:00 eta 00:00"
        );
        // The brief's example decomposes into the pinned parts:
        // 1051/1811 → 58.0%, 192 s → 03:12.
        assert_eq!(format!("{:.1}", percent(1051, 1811)), "58.0");
        assert_eq!(clock_str(192.0), "03:12");
    }

    #[test]
    fn timing_suffix_composes_parts() {
        let clock = RunClock::new();
        assert_eq!(clock.timing_suffix(0, 100), "elapsed 00:00 · eta --:--");
        assert_eq!(clock.timing_suffix(100, 100), "elapsed 00:00 · eta 00:00");
    }

    #[test]
    fn summary_suffix_shape() {
        let clock = RunClock::new();
        let s = clock.summary_suffix();
        // Value depends on platform/process; pin only the shape:
        // time first, then the peak-RSS segment (number or the
        // explicit unavailable marker — never a fabricated 0).
        assert!(s.starts_with("elapsed 00:00 · peak RSS "), "got: {s}");
        assert!(!s.contains("peak RSS 0.0 MB"), "got: {s}");
    }

    #[test]
    fn vmhwm_parses_kernel_format() {
        let status = "Name:\ttalerender\n\
                      Umask:\t0022\n\
                      State:\tR (running)\n\
                      VmPeak:\t  512340 kB\n\
                      VmSize:\t  498120 kB\n\
                      VmHWM:\t  262144 kB\n\
                      VmRSS:\t  200000 kB\n";
        assert_eq!(parse_vmhwm_mb(status), Some(256.0)); // 262144 KiB = 256 MiB
        assert_eq!(parse_vmhwm_mb("VmHWM:    1536 kB\n"), Some(1.5));
    }

    #[test]
    fn vmhwm_rejects_missing_or_malformed() {
        assert_eq!(parse_vmhwm_mb("VmRSS:\t  200000 kB\n"), None); // no VmHWM line
        assert_eq!(parse_vmhwm_mb("VmHWM:\t  plenty kB\n"), None); // non-numeric
        assert_eq!(parse_vmhwm_mb("VmHWM:\t  262144 MB\n"), None); // wrong unit
        assert_eq!(parse_vmhwm_mb(""), None);
    }

    #[test]
    fn peak_rss_is_some_on_linux() {
        // The test runner is Linux CI; the live call must find
        // the real line in our own /proc/self/status. (Off-Linux
        // this test is compiled out with the cfg below.)
        #[cfg(target_os = "linux")]
        {
            let mb = peak_rss_mb().expect("VmHWM must parse on Linux");
            assert!(mb > 0.0);
        }
    }

    #[test]
    fn frame_timing_env_switch() {
        std::env::remove_var("TALERENDER_FRAME_TIMING");
        assert!(!frame_timing_enabled());
        std::env::set_var("TALERENDER_FRAME_TIMING", "1");
        assert!(frame_timing_enabled());
        std::env::set_var("TALERENDER_FRAME_TIMING", "0");
        assert!(!frame_timing_enabled());
        std::env::remove_var("TALERENDER_FRAME_TIMING");
    }
}
