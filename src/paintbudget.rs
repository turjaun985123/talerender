//! Paint-cache byte budget + statistics (Wave 2 seat W06).
//!
//! film.rs's `PaintCache` owns the storage (a `HashMap<String,
//! Rc<Painted>>` plus an LRU `VecDeque<String>`) and until now
//! enforced a single limit: an ENTRY cap (default 160, env
//! `TALERENDER_PAINT_CACHE`). One painted canvas is 2560x2816 RGBA
//! = 28,835,840 bytes, so 160 entries is a ~4.6 GB worst case on a
//! ~6.7 GiB machine — the audit's memory finding. This module is
//! the accounting + policy half the audit asked for: a BYTE budget
//! the cache consults, byte accounting, and hit / miss / eviction /
//! re-render statistics with a greppable end-of-run line (which is
//! also PERF.md §9 item 2's missing signal).
//!
//! Pixel-neutrality law: this module never sees a pixel. Painting
//! is a pure function of the EXACT state key (film.rs `state_key`,
//! f32 bit patterns since cabcc30), so a re-paint of an evicted key
//! is byte-identical to the evicted entry. The tracker only ever
//! changes HOW OFTEN `paint_toon_full` re-runs, never what it
//! produces. See docs/integration/PAINT-BUDGET-W06.md for the full
//! argument and the wiring sketch.
//!
//! Accounting unit: an entry's byte cost is the length of its
//! pixel buffer (`Painted.0.len()`, via `entry_bytes`). The Rc
//! control block, the tuple's scalars and the ~200-byte key string
//! are five orders of magnitude below one canvas and are excluded
//! on purpose: the budget is a memory ceiling with GiB of headroom,
//! not an RSS predictor (PERF.md §4 reads RSS against exactly this
//! canvas math).

use std::collections::HashSet;

/// Exact byte size of one painted character canvas:
/// 2560 x 2816 RGBA8. Matches `paint_toon_full`'s canvas
/// (1280*2 x 1408*2) in toonfull.rs.
pub(crate) const CANVAS_BYTES: u64 = 2560 * 2816 * 4;

/// The historic default entry cap (film.rs pre-budget behaviour).
pub(crate) const DEFAULT_ENTRY_CAP: usize = 160;

/// Hard ceiling for the DERIVED default byte budget: 2 GiB.
///
/// Justification. The target PC has ~6.7 GiB usable RAM, shared
/// with the Vega 7 iGPU; PERF.md §7 records the old ~4.6 GB
/// cache worst case as a genuine OOM risk next to the renderer,
/// the film assets and the OS. 2 GiB keeps the paint cache at
/// roughly 30% of usable RAM on that machine, and GitHub's
/// ubuntu-22.04 CI runners (7 GB) sit in the same envelope. In
/// entries it is ~74 full canvases: over 3x the memory-tight
/// telemetry setting (cap 24, ~0.69 GB) that already audits the
/// whole film, and — per film.rs's own note — still far above the
/// live working set of any single shot. Operators can override
/// with TALERENDER_PAINT_BUDGET_MB, or set it to 0 to disable
/// byte enforcement entirely (entry cap alone, legacy behaviour).
pub(crate) const BUDGET_CEILING_BYTES: u64 = 2048 * 1024 * 1024;

const MIB: u64 = 1024 * 1024;

/// Why one eviction happened (WP1.1). Captured at eviction
/// time from the pre-eviction state; see `eviction_reason`.
/// Precedence when both bounds are exceeded: EntryCap
/// (V2-WP1-LRU-SPEC §3.2), which keeps the invariant
/// evictions_byte + evictions_cap == evictions.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum EvictReason {
    ByteBudget,
    EntryCap,
}

/// Byte cost of one cache entry: the pixel buffer's length.
/// `painted` is film.rs's `Painted.0` (`Vec<u8>`, RGBA).
pub(crate) fn entry_bytes(pixels: &[u8]) -> u64 {
    pixels.len() as u64
}

/// The derived default budget: the entry cap expressed in bytes,
/// clamped to BUDGET_CEILING_BYTES. A small operator-chosen entry
/// cap therefore also shrinks the byte budget — the byte budget
/// never silently exceeds the chosen cap in byte terms.
pub(crate) fn default_budget_bytes(entry_cap: usize, canvas_bytes: u64) -> u64 {
    (entry_cap as u64)
        .saturating_mul(canvas_bytes)
        .min(BUDGET_CEILING_BYTES)
}

/// Pure form of the TALERENDER_PAINT_BUDGET_MB parse, split out
/// for tests (env access itself is `from_env`).
/// - `None` (unset) or unparseable -> the derived default.
/// - `"0"` -> 0, byte enforcement DISABLED.
/// - `"N"` -> N MiB.
pub(crate) fn budget_bytes_from_env_value(
    raw: Option<&str>,
    entry_cap: usize,
    canvas_bytes: u64,
) -> u64 {
    match raw {
        Some(v) => match v.trim().parse::<u64>() {
            Ok(mb) => mb.saturating_mul(MIB),
            Err(_) => default_budget_bytes(entry_cap, canvas_bytes),
        },
        None => default_budget_bytes(entry_cap, canvas_bytes),
    }
}

/// Budget + statistics tracker for one `PaintCache` lifetime (one
/// film render or one telemetry run — film.rs constructs a fresh
/// cache per run, so the counters are per-run by construction).
///
/// Call contract with the cache (every map mutation MUST be
/// reported, or the accounting drifts):
/// - hit: `on_hit`.
/// - miss: `on_miss(key)` (before painting).
/// - insert into the map: `on_insert(bytes)`.
/// - each eviction: `on_evict(key, bytes, reason)`.
/// - eviction loop condition: `should_evict()`.
pub(crate) struct PaintBudget {
    /// Byte ceiling; 0 = disabled (entry cap alone governs).
    budget_bytes: u64,
    entry_cap: usize,
    bytes: u64,
    peak_bytes: u64,
    entries: usize,
    hits: u64,
    misses: u64,
    evictions: u64,
    evictions_byte: u64,
    evictions_cap: u64,
    /// Misses whose key had been evicted earlier in this run —
    /// the re-render count the budget actually costs.
    rerenders: u64,
    /// Keys evicted at least once. Consulted only on a miss.
    /// Growth is bounded by distinct evicted states (~200 B per
    /// key); even 50k evictions cost ~10 MB, two orders below one
    /// canvas. If this ever matters, drop the `rerenders` metric,
    /// not the budget.
    evicted_keys: HashSet<String>,
}

impl PaintBudget {
    pub(crate) fn new(budget_bytes: u64, entry_cap: usize) -> PaintBudget {
        PaintBudget {
            budget_bytes,
            entry_cap,
            bytes: 0,
            peak_bytes: 0,
            entries: 0,
            hits: 0,
            misses: 0,
            evictions: 0,
            evictions_byte: 0,
            evictions_cap: 0,
            rerenders: 0,
            evicted_keys: HashSet::new(),
        }
    }

    /// Production constructor: byte budget from
    /// TALERENDER_PAINT_BUDGET_MB (or the derived default),
    /// entry cap handed in by the cache (which reads
    /// TALERENDER_PAINT_CACHE, default DEFAULT_ENTRY_CAP).
    pub(crate) fn from_env(entry_cap: usize, canvas_bytes: u64) -> PaintBudget {
        let raw = std::env::var("TALERENDER_PAINT_BUDGET_MB").ok();
        PaintBudget::new(
            budget_bytes_from_env_value(raw.as_deref(), entry_cap, canvas_bytes),
            entry_cap,
        )
    }

    /// Would an entry of `bytes` fit within the byte budget once
    /// resident? Always true when the byte budget is disabled.
    /// The cache may use this to skip retaining a single entry
    /// that exceeds the whole budget (paint it, hand out the Rc,
    /// do not insert) — an optional wiring variant; the primary
    /// wiring keeps the entry and lets `should_evict` govern.
    pub(crate) fn admit(&self, bytes: u64) -> bool {
        self.budget_bytes == 0 || self.bytes.saturating_add(bytes) <= self.budget_bytes
    }

    /// The reason the cache must evict now, or None —
    /// mirroring the legacy loop `while map.len() > cap` and
    /// extending it with the byte budget. Two deliberate
    /// semantics:
    /// - The entry cap is checked exactly as before (so cap 0
    ///   still evicts everything, as the legacy code did).
    /// - The byte condition never evicts the LAST remaining
    ///   entry: a single canvas larger than the whole budget is
    ///   the live entry, and evicting it on its own insertion
    ///   would thrash without saving a byte of live memory.
    /// Cap takes precedence when both bounds are exceeded
    /// (V2-WP1-LRU-SPEC §3.2). `should_evict()` is exactly
    /// `eviction_reason().is_some()` — one implementation, so
    /// the loop condition and the recorded reason cannot drift.
    pub(crate) fn eviction_reason(&self) -> Option<EvictReason> {
        if self.entries > self.entry_cap {
            Some(EvictReason::EntryCap)
        } else if self.budget_bytes > 0 && self.bytes > self.budget_bytes && self.entries > 1 {
            Some(EvictReason::ByteBudget)
        } else {
            None
        }
    }

    pub(crate) fn should_evict(&self) -> bool {
        self.eviction_reason().is_some()
    }

    pub(crate) fn on_hit(&mut self) {
        self.hits += 1;
    }

    pub(crate) fn on_miss(&mut self, key: &str) {
        self.misses += 1;
        if self.evicted_keys.contains(key) {
            self.rerenders += 1;
        }
    }

    pub(crate) fn on_insert(&mut self, bytes: u64) {
        self.entries += 1;
        self.bytes = self.bytes.saturating_add(bytes);
        if self.bytes > self.peak_bytes {
            self.peak_bytes = self.bytes;
        }
    }

    pub(crate) fn on_evict(&mut self, key: &str, bytes: u64, reason: EvictReason) {
        self.entries = self.entries.saturating_sub(1);
        self.bytes = self.bytes.saturating_sub(bytes);
        self.evictions += 1;
        match reason {
            EvictReason::ByteBudget => self.evictions_byte += 1,
            EvictReason::EntryCap => self.evictions_cap += 1,
        }
        self.evicted_keys.insert(key.to_string());
    }

    pub(crate) fn bytes(&self) -> u64 {
        self.bytes
    }

    pub(crate) fn peak_bytes(&self) -> u64 {
        self.peak_bytes
    }

    pub(crate) fn entries(&self) -> usize {
        self.entries
    }

    pub(crate) fn hits(&self) -> u64 {
        self.hits
    }

    pub(crate) fn misses(&self) -> u64 {
        self.misses
    }

    pub(crate) fn evictions(&self) -> u64 {
        self.evictions
    }

    pub(crate) fn evictions_byte(&self) -> u64 {
        self.evictions_byte
    }

    pub(crate) fn evictions_cap(&self) -> u64 {
        self.evictions_cap
    }

    pub(crate) fn rerenders(&self) -> u64 {
        self.rerenders
    }

    pub(crate) fn budget_bytes(&self) -> u64 {
        self.budget_bytes
    }

    pub(crate) fn entry_cap(&self) -> usize {
        self.entry_cap
    }

    /// Hit rate in permille (hits x 1000 / lookups), derived
    /// from the counters at print time so it cannot drift.
    /// 0 when there were no lookups — a defined sentinel, NOT
    /// a claim of 0% success for an empty run.
    fn hit_rate_permille(&self) -> u64 {
        let total = self.hits.saturating_add(self.misses);
        if total == 0 {
            0
        } else {
            self.hits.saturating_mul(1000) / total
        }
    }

    /// One greppable end-of-run line (PERF.md §9 item 2). The
    /// first EIGHT fields are the stable prefix — tools/perf/
    /// vram_report.py's regex ends at budget_bytes with no end
    /// anchor, so fields may only ever be APPENDED after it
    /// (the WP1.1 extension: hit_rate_permille, evictions_byte,
    /// evictions_cap). budget_bytes=0 means disabled.
    pub(crate) fn stats_line(&self) -> String {
        format!(
            "paint-cache: entries={} bytes={} hits={} misses={} evictions={} rerenders={} peak_bytes={} budget_bytes={} hit_rate_permille={} evictions_byte={} evictions_cap={}",
            self.entries,
            self.bytes,
            self.hits,
            self.misses,
            self.evictions,
            self.rerenders,
            self.peak_bytes,
            self.budget_bytes,
            self.hit_rate_permille(),
            self.evictions_byte,
            self.evictions_cap
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const C: u64 = CANVAS_BYTES;

    #[test]
    fn canvas_bytes_matches_the_painters_canvas() {
        // toonfull.rs paints at (1280*2) x (1408*2), RGBA8.
        assert_eq!(C, 2560 * 2816 * 4);
        assert_eq!(C, 28_835_840);
        assert_eq!(entry_bytes(&vec![0u8; C as usize]), C);
        assert_eq!(entry_bytes(&[]), 0);
    }

    #[test]
    fn accounting_tracks_bytes_entries_and_peak() {
        let mut b = PaintBudget::new(0, 160);
        b.on_insert(100);
        b.on_insert(250);
        assert_eq!(b.entries(), 2);
        assert_eq!(b.bytes(), 350);
        assert_eq!(b.peak_bytes(), 350);
        b.on_evict("k1", 100, EvictReason::EntryCap);
        assert_eq!(b.entries(), 1);
        assert_eq!(b.bytes(), 250);
        assert_eq!(b.evictions(), 1);
        // Peak is a high-water mark: eviction does not lower it.
        assert_eq!(b.peak_bytes(), 350);
        b.on_insert(50);
        assert_eq!(b.bytes(), 300);
        assert_eq!(b.peak_bytes(), 350);
    }

    #[test]
    fn hits_do_not_change_accounting() {
        let mut b = PaintBudget::new(0, 160);
        b.on_insert(100);
        b.on_hit();
        b.on_hit();
        assert_eq!(b.hits(), 2);
        assert_eq!(b.misses(), 0);
        assert_eq!(b.bytes(), 100);
        assert_eq!(b.entries(), 1);
    }

    #[test]
    fn byte_budget_triggers_eviction_need() {
        // Budget holds exactly two canvases.
        let mut b = PaintBudget::new(2 * C, 160);
        b.on_insert(C);
        b.on_insert(C);
        assert!(!b.should_evict());
        b.on_insert(C);
        assert!(b.should_evict());
        b.on_evict("k1", C, EvictReason::ByteBudget);
        assert!(!b.should_evict());
        assert_eq!(b.bytes(), 2 * C);
    }

    #[test]
    fn single_oversize_entry_is_never_self_evicted() {
        // Budget smaller than one canvas: the live entry stays.
        let mut b = PaintBudget::new(C / 2, 160);
        assert!(!b.admit(C));
        b.on_insert(C);
        assert!(!b.should_evict());
        // A second entry makes the FIRST evictable.
        b.on_insert(C);
        assert!(b.should_evict());
        b.on_evict("k1", C, EvictReason::ByteBudget);
        assert!(!b.should_evict());
        assert_eq!(b.entries(), 1);
    }

    #[test]
    fn admit_edges() {
        let disabled = PaintBudget::new(0, 160);
        assert!(disabled.admit(u64::MAX / 2));
        let mut b = PaintBudget::new(1000, 160);
        assert!(b.admit(1000)); // exact fit
        assert!(!b.admit(1001)); // one byte over
        b.on_insert(400);
        assert!(b.admit(600));
        assert!(!b.admit(601));
    }

    #[test]
    fn zero_budget_disables_bytes_but_not_entry_cap() {
        let mut b = PaintBudget::new(0, 2);
        b.on_insert(C);
        b.on_insert(C);
        assert!(!b.should_evict());
        b.on_insert(C);
        assert!(b.should_evict()); // entry cap 2, not bytes
        b.on_evict("k1", C, EvictReason::EntryCap);
        assert!(!b.should_evict());
    }

    #[test]
    fn zero_entry_cap_preserves_legacy_evict_all() {
        // Legacy loop with cap 0 evicted even the just-inserted
        // entry; the byte guard must not soften the cap check.
        let mut b = PaintBudget::new(0, 0);
        b.on_insert(C);
        assert!(b.should_evict());
    }

    #[test]
    fn rerenders_counted_only_after_eviction() {
        let mut b = PaintBudget::new(0, 1);
        b.on_miss("walk-a");
        assert_eq!(b.misses(), 1);
        assert_eq!(b.rerenders(), 0);
        b.on_insert(C);
        b.on_insert(C); // forces nothing by itself; cache evicts:
        b.on_evict("walk-a", C, EvictReason::EntryCap);
        b.on_miss("walk-a"); // a re-render of an evicted state
        assert_eq!(b.misses(), 2);
        assert_eq!(b.rerenders(), 1);
        b.on_miss("never-seen");
        assert_eq!(b.rerenders(), 1);
    }

    #[test]
    fn default_budget_derivation() {
        // Default cap: the 2 GiB ceiling binds (160 canvases
        // would be ~4.6 GB).
        assert_eq!(default_budget_bytes(160, C), BUDGET_CEILING_BYTES);
        assert_eq!(BUDGET_CEILING_BYTES, 2048 * 1024 * 1024);
        // Small caps scale down linearly below the ceiling.
        assert_eq!(default_budget_bytes(24, C), 24 * C);
        assert_eq!(default_budget_bytes(1, C), C);
        assert_eq!(default_budget_bytes(0, C), 0);
    }

    #[test]
    fn env_value_parsing() {
        assert_eq!(budget_bytes_from_env_value(None, 160, C), BUDGET_CEILING_BYTES);
        assert_eq!(budget_bytes_from_env_value(Some("0"), 160, C), 0);
        assert_eq!(budget_bytes_from_env_value(Some("512"), 160, C), 512 * MIB);
        assert_eq!(budget_bytes_from_env_value(Some(" 256 "), 160, C), 256 * MIB);
        assert_eq!(budget_bytes_from_env_value(Some("junk"), 160, C), BUDGET_CEILING_BYTES);
        assert_eq!(budget_bytes_from_env_value(Some(""), 160, C), BUDGET_CEILING_BYTES);
        assert_eq!(budget_bytes_from_env_value(Some("-5"), 160, C), BUDGET_CEILING_BYTES);
        // Unset with a small cap follows the small cap.
        assert_eq!(budget_bytes_from_env_value(None, 24, C), 24 * C);
    }

    #[test]
    fn eviction_reason_classification() {
        // Byte-only: budget 2 canvases, cap 160, 3 resident.
        let mut b = PaintBudget::new(2 * C, 160);
        b.on_insert(C);
        b.on_insert(C);
        assert_eq!(b.eviction_reason(), None);
        b.on_insert(C);
        assert_eq!(b.eviction_reason(), Some(EvictReason::ByteBudget));
        // Cap-only: budget disabled, cap 2, 3 resident.
        let mut b = PaintBudget::new(0, 2);
        b.on_insert(C);
        b.on_insert(C);
        b.on_insert(C);
        assert_eq!(b.eviction_reason(), Some(EvictReason::EntryCap));
        // Both predicates live -> EntryCap precedence.
        let mut b = PaintBudget::new(C / 2, 1);
        b.on_insert(C);
        b.on_insert(C);
        assert_eq!(b.eviction_reason(), Some(EvictReason::EntryCap));
        // Budget 0 is never ByteBudget.
        let mut b = PaintBudget::new(0, 160);
        b.on_insert(C);
        assert_eq!(b.eviction_reason(), None);
        // Single oversize entry: the live-entry guard survives.
        let mut b = PaintBudget::new(C / 2, 160);
        b.on_insert(C);
        assert_eq!(b.eviction_reason(), None);
    }

    #[test]
    fn eviction_reason_matches_should_evict() {
        // Anti-drift pin: should_evict IS eviction_reason.
        for (budget, cap, inserts) in
            [(0u64, 160usize, 0usize), (0, 2, 3), (2 * C, 160, 3), (C / 2, 160, 1), (C / 2, 1, 2), (1000, 160, 1)]
        {
            let mut b = PaintBudget::new(budget, cap);
            for _ in 0..inserts {
                b.on_insert(C);
            }
            assert_eq!(b.eviction_reason().is_some(), b.should_evict(), "budget={budget} cap={cap} inserts={inserts}");
        }
    }

    #[test]
    fn eviction_counters_sum_to_evictions() {
        let mut b = PaintBudget::new(2 * C, 160);
        b.on_insert(C);
        b.on_insert(C);
        b.on_insert(C);
        let r = b.eviction_reason().expect("byte eviction pending");
        b.on_evict("k1", C, r);
        assert_eq!(b.evictions_byte(), 1);
        assert_eq!(b.evictions_cap(), 0);
        let mut b = PaintBudget::new(0, 1);
        b.on_insert(C);
        b.on_insert(C);
        let r = b.eviction_reason().expect("cap eviction pending");
        b.on_evict("k1", C, r);
        assert_eq!(b.evictions(), b.evictions_byte() + b.evictions_cap());
        assert_eq!(b.evictions_cap(), 1);
    }

    #[test]
    fn hit_rate_permille_edges() {
        let b = PaintBudget::new(0, 160);
        assert_eq!(b.hit_rate_permille(), 0); // no lookups: sentinel
        let mut b = PaintBudget::new(0, 160);
        b.on_hit();
        b.on_hit();
        assert_eq!(b.hit_rate_permille(), 1000); // all hits
        let mut b = PaintBudget::new(0, 160);
        b.on_hit();
        b.on_hit();
        b.on_miss("x");
        b.on_miss("y");
        b.on_miss("z");
        assert_eq!(b.hit_rate_permille(), 400); // 2 of 5
        let mut b = PaintBudget::new(0, 160);
        b.on_miss("x");
        assert_eq!(b.hit_rate_permille(), 0); // all misses
    }

    #[test]
    fn stats_line_format_is_stable() {
        let mut b = PaintBudget::new(1000, 160);
        b.on_miss("a");
        b.on_insert(100);
        b.on_hit();
        b.on_hit();
        b.on_miss("b");
        b.on_insert(250);
        b.on_evict("a", 100, EvictReason::EntryCap);
        b.on_miss("a"); // rerender
        assert_eq!(
            b.stats_line(),
            "paint-cache: entries=1 bytes=250 hits=2 misses=3 evictions=1 rerenders=1 peak_bytes=350 budget_bytes=1000 hit_rate_permille=400 evictions_byte=0 evictions_cap=1"
        );
    }
}
