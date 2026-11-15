/*
 * DELIVERY NOTE — TALERENDER V2, Batch 2, Seat 1: WP1.4 resource lifetimes
 * =========================================================================
 *
 * FILES CREATED
 *   src/lifetimes.rs  (this file — the seat's WP1.4 deliverable;
 *   its sibling deliverable is the patch spec
 *   docs/integration/V2-WP1-LRU-SPEC.md, which covers WP1.1)
 *
 * CONTRACT POINTS IMPLEMENTED (V2-CONTRACTS.md §1 + §5 Batch 2
 * seat 1; V2-MASTER-PROMPT.md §5.4 "Resource lifetime
 * management": explicit ownership, safe disposal, temporary-
 * allocation tracking, lifetime diagnostics)
 *   - LifetimeTracker: pure bookkeeping for named resources.
 *     register(name, class, bytes) -> Handle records one live
 *     resource; release(Handle) retires it. The tracker owns NO
 *     resource itself — no wgpu types, no Drop side effects —
 *     it is the diagnostic ledger a resource owner consults,
 *     in the same relationship paintbudget.rs has to film.rs's
 *     PaintCache and bufferpool.rs has to gpucore::Readback.
 *   - snapshot() -> Vec<Entry>: EVERY registration of the run
 *     (live and released, each flagged), sorted by bytes
 *     DESCENDING with registration id ASCENDING as the
 *     tie-break — a total order, so the output is a pure
 *     function of the register/release sequence (deterministic;
 *     pinned by a replay test).
 *   - leaked() -> Vec<Entry>: the registered-never-released
 *     subset of snapshot(), same order. "Leaked" is meant
 *     literally at snapshot time: still live when the caller
 *     asks. Mid-run that is just the working set; at end of a
 *     scope/run it is the leak report.
 *   - Totals: live_bytes() (current), peak_bytes() (high-water
 *     mark of live_bytes, updated on register only),
 *     live_count() / registered_count() / released_count().
 *   - Misuse is counted, never panicked on: releasing the same
 *     Handle twice is a counted no-op (double_releases());
 *     releasing a Handle this tracker never issued is a counted
 *     no-op (unknown_releases()). release() returns bool —
 *     true exactly when it transitioned a live entry to
 *     released.
 *   - stats_line(): one greppable line in the house style of
 *     paintbudget::stats_line / bufferpool::stats_line.
 *   - std only. No new dependencies, no I/O, no env, no clock.
 *
 * WIRING THE LEAD MUST DO (no existing file is edited by this seat)
 *   1. src/main.rs: add the module line in the alphabetical run,
 *      after the anchor line
 *          mod gpucore;
 *      and before `mod math;`:
 *          mod lifetimes;
 *   2. No call site is required for this landing — the module is
 *      diagnostics infrastructure, landed ahead of its consumers
 *      exactly as bufferpool.rs's policy landed under
 *      gpucore::Readback. #[allow(dead_code)] below covers the
 *      intentionally wider-than-first-use API surface; remove
 *      the allows as consumers land. Suggested first consumers
 *      (lead's choice, follow-up landings, NOT this one):
 *        a. gpucore.rs's allocation ledger call sites — register
 *           per MemClass allocation with class "A".."G" to get
 *           per-resource leak reports beside the existing
 *           gpu-mem totals (the ledger counts bytes; this
 *           tracker names names — they complement, not replace).
 *        b. film.rs's Renderer — register each created texture /
 *           render target at creation and release at teardown,
 *           then print leaked() at end of run: any entry left is
 *           a target whose teardown path was missed.
 *        c. film.rs's PaintCache — register on insert, release
 *           on evict, as a cross-check of PaintBudget's byte
 *           accounting during the WP1.1 LRU landing's telemetry.
 *   3. Nothing else: no Cargo.toml change (zero new deps), no
 *      workflow change, no stats-line consumer change (the
 *      lifetimes: line is new and printed only where a consumer
 *      chooses to print it).
 *
 * TESTS (in-module, #[cfg(test)], std only)
 *   register/release accounting (bytes, counts, is_live) /
 *   double-release is a counted no-op, not a panic /
 *   unknown + zero handles are counted no-ops / peak is a
 *   high-water mark (register-only growth, release-proof) /
 *   snapshot sorted by bytes desc with id-asc tie-break and
 *   including released entries flagged / snapshot + leaked
 *   determinism under replayed operation sequences /
 *   leaked() lists only never-released entries and empties
 *   when all are released / duplicate names are distinct
 *   entries (identity is the Handle) / zero-byte registrations
 *   tracked / byte totals saturate instead of overflowing /
 *   stats_line format is stable.
 *
 * KNOWN LIMITATIONS
 *   - History is retained for the tracker's whole lifetime:
 *     released records are kept (that is what makes leaked(),
 *     peak and the audit trail possible), so memory grows with
 *     DISTINCT REGISTRATIONS (~two short Strings + scalars per
 *     entry), not with live resources. Register coarse-grained
 *     resources (textures, targets, caches, files) — not
 *     per-frame transients; the per-frame readback path already
 *     has bufferpool.rs + the gpucore ledger.
 *   - Handles are per-tracker: ids are a per-instance sequence
 *     starting at 1 (0 is never issued, so it can serve as a
 *     null handle). A Handle from tracker A released into
 *     tracker B whose id happens to exist there will release
 *     B's entry — the same per-owner discipline bufferpool.rs
 *     imposes per-device. Callers own one tracker per scope.
 *   - Not thread-safe and not Sync by design: plain owned
 *     state, one owner per run/scope, like PaintBudget and
 *     BufferPool. Wrap in a Mutex at the call site if a future
 *     consumer truly shares one across threads.
 *   - The tracker cannot force disposal: it records that a
 *     release was reported. A caller that drops a resource
 *     without release() shows up in leaked() — detection is
 *     the feature; enforcement stays with the owner's types.
 *   - Class and name are free-form &str labels (stored owned).
 *     No registry is enforced; consumers should reuse the
 *     class vocabulary of their subsystem (e.g. gpucore's
 *     MemClass letters) rather than inventing per-call-site
 *     spellings, or snapshot() groupings will fragment.
 */

//! Resource lifetime diagnostics (V2 WP1.4, master prompt §5.4).
//!
//! A `LifetimeTracker` is the bookkeeping half of explicit
//! resource lifetimes: the owner of a GPU texture, a render
//! target, a cache or any other named allocation registers it
//! on creation and releases the returned `Handle` on disposal,
//! and the tracker answers the three questions raw byte totals
//! cannot — *what* is still alive (`snapshot` / `leaked`),
//! *how big* it is (per-entry bytes, sorted), and *how much*
//! was ever simultaneously alive (`peak_bytes`). It mirrors the
//! shape of the two landed policy modules: `paintbudget.rs`
//! (accounting + stats line for a cache it does not own) and
//! `bufferpool.rs` (lifecycle policy generic over a handle it
//! never destroys). Like both, it is pure CPU bookkeeping:
//! nothing here can change a pixel, a byte of output, or an
//! exit code — it only ever changes what a diagnostic prints.

/// Opaque token identifying one registration. Issued by
/// `LifetimeTracker::register`, handed back to `release`.
/// Ids are a per-tracker sequence starting at 1; id 0 is never
/// issued, so a zero/default handle is always "unknown" and
/// releasing it is a counted no-op rather than a wild write.
/// The field is private: callers hold and return handles, they
/// do not fabricate them (the in-module tests can, which is
/// how the unknown-handle path is exercised).
#[allow(dead_code)]
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(crate) struct Handle {
    id: u64,
}

#[allow(dead_code)]
impl Handle {
    /// The registration sequence number (1-based). Exposed for
    /// log lines and for correlating an `Entry` with the
    /// registration order; never needed for release itself.
    pub(crate) fn id(&self) -> u64 {
        self.id
    }

    /// True for the never-issued id 0 (e.g. a default/null
    /// handle a caller initialised a slot with).
    pub(crate) fn is_null(&self) -> bool {
        self.id == 0
    }
}

/// One registration as reported by `snapshot()` / `leaked()`:
/// a detached value copy, so a snapshot stays valid (and
/// unchanged) while the tracker keeps running.
#[allow(dead_code)]
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) struct Entry {
    /// Registration sequence number — the `Handle::id` this
    /// entry was registered under.
    pub id: u64,
    /// Caller-supplied resource name (e.g. `"card-tex-3"`).
    /// Not unique: identity is the id, never the name.
    pub name: String,
    /// Caller-supplied class label (e.g. a gpucore MemClass
    /// letter, `"texture"`, `"render-target"`). Free-form by
    /// design; see the delivery note's limitation on vocabulary.
    pub class: String,
    /// Byte cost attributed to the resource at registration.
    /// The tracker trusts it exactly as paintbudget trusts
    /// `entry_bytes`: it is an accounting figure, not a
    /// measurement of RSS or VRAM.
    pub bytes: u64,
    /// True once `release` has retired this entry. Entries in
    /// `leaked()` always have `released == false`.
    pub released: bool,
}

/// Internal record: an `Entry` minus the id, which is its
/// index in the tracker's Vec (+ 1). Keeping records in a Vec
/// makes release O(1) and snapshot's registration order free.
struct Record {
    name: String,
    class: String,
    bytes: u64,
    released: bool,
}

/// Pure bookkeeping over named resources: registrations,
/// releases, live/peak byte totals, and leak reporting.
///
/// Call contract (mirrors paintbudget.rs's): every resource
/// the owner creates inside the tracked scope gets exactly one
/// `register`, and its disposal gets exactly one `release` of
/// the returned handle. The tracker tolerates violations of
/// the second half — double and unknown releases are counted
/// no-ops — because a diagnostics module must never be the
/// thing that crashes the run it is observing.
#[allow(dead_code)]
pub(crate) struct LifetimeTracker {
    /// Next id to issue; starts at 1 (0 is the null handle).
    next_id: u64,
    /// Every registration ever made, in registration order;
    /// `records[i]` has id `i + 1`. Released records are kept
    /// (history is the audit trail — see the delivery note).
    records: Vec<Record>,
    live_bytes: u64,
    peak_bytes: u64,
    double_releases: u64,
    unknown_releases: u64,
}

#[allow(dead_code)]
impl LifetimeTracker {
    pub(crate) fn new() -> LifetimeTracker {
        LifetimeTracker {
            next_id: 1,
            records: Vec::new(),
            live_bytes: 0,
            peak_bytes: 0,
            double_releases: 0,
            unknown_releases: 0,
        }
    }

    /// Record one newly created resource and return its
    /// handle. `bytes` is added to the live total and the peak
    /// is raised if the total sets a new high-water mark —
    /// registration is the only operation that can raise the
    /// peak, by construction (releases only lower the live
    /// total, snapshots change nothing). Duplicate names and
    /// classes are fine: each call is a distinct entry with a
    /// distinct handle.
    pub(crate) fn register(&mut self, name: &str, class: &str, bytes: u64) -> Handle {
        let handle = Handle { id: self.next_id };
        self.next_id = self.next_id.saturating_add(1);
        self.records.push(Record {
            name: name.to_string(),
            class: class.to_string(),
            bytes,
            released: false,
        });
        self.live_bytes = self.live_bytes.saturating_add(bytes);
        if self.live_bytes > self.peak_bytes {
            self.peak_bytes = self.live_bytes;
        }
        handle
    }

    /// Retire the resource behind `handle`. Returns `true`
    /// exactly when this call transitioned a live entry to
    /// released (and subtracted its bytes from the live total).
    /// Every other outcome is a counted no-op, never a panic:
    /// an already-released handle increments `double_releases`,
    /// a handle this tracker never issued (including the null
    /// handle) increments `unknown_releases`, and neither
    /// touches any entry or total.
    pub(crate) fn release(&mut self, handle: Handle) -> bool {
        if handle.id == 0 {
            self.unknown_releases = self.unknown_releases.saturating_add(1);
            return false;
        }
        let idx = match usize::try_from(handle.id - 1) {
            Ok(i) => i,
            Err(_) => {
                self.unknown_releases = self.unknown_releases.saturating_add(1);
                return false;
            }
        };
        let record = match self.records.get_mut(idx) {
            Some(r) => r,
            None => {
                self.unknown_releases = self.unknown_releases.saturating_add(1);
                return false;
            }
        };
        if record.released {
            self.double_releases = self.double_releases.saturating_add(1);
            return false;
        }
        record.released = true;
        self.live_bytes = self.live_bytes.saturating_sub(record.bytes);
        true
    }

    /// True iff `handle` names an entry that is registered and
    /// not yet released. False for released, unknown and null
    /// handles alike — this is a query, so unlike `release` it
    /// counts nothing.
    pub(crate) fn is_live(&self, handle: Handle) -> bool {
        if handle.id == 0 {
            return false;
        }
        match usize::try_from(handle.id - 1) {
            Ok(idx) => match self.records.get(idx) {
                Some(r) => !r.released,
                None => false,
            },
            Err(_) => false,
        }
    }

    /// Every registration of the run, live and released, as
    /// value copies — sorted by `bytes` descending, ties by id
    /// ascending. The tie-break makes the order total: two
    /// trackers fed the same register/release sequence produce
    /// byte-identical snapshots regardless of sort stability
    /// or hash iteration (there is no hash map in this module).
    pub(crate) fn snapshot(&self) -> Vec<Entry> {
        let mut out: Vec<Entry> = self
            .records
            .iter()
            .enumerate()
            .map(|(i, r)| Entry {
                id: (i as u64) + 1,
                name: r.name.clone(),
                class: r.class.clone(),
                bytes: r.bytes,
                released: r.released,
            })
            .collect();
        out.sort_by(|a, b| b.bytes.cmp(&a.bytes).then_with(|| a.id.cmp(&b.id)));
        out
    }

    /// The leak report: registrations never released as of
    /// this call, in the same order as `snapshot()`. At the
    /// end of a scope where every resource should have been
    /// disposed, a non-empty result names exactly what was
    /// missed, largest first.
    pub(crate) fn leaked(&self) -> Vec<Entry> {
        self.snapshot().into_iter().filter(|e| !e.released).collect()
    }

    /// Bytes currently attributed to live (unreleased) entries.
    pub(crate) fn live_bytes(&self) -> u64 {
        self.live_bytes
    }

    /// High-water mark of `live_bytes` over the tracker's
    /// lifetime. Releases never lower it.
    pub(crate) fn peak_bytes(&self) -> u64 {
        self.peak_bytes
    }

    pub(crate) fn registered_count(&self) -> usize {
        self.records.len()
    }

    pub(crate) fn released_count(&self) -> usize {
        self.records.iter().filter(|r| r.released).count()
    }

    /// Registered minus released — the length `leaked()`
    /// would return, without building it.
    pub(crate) fn live_count(&self) -> usize {
        self.registered_count() - self.released_count()
    }

    /// Releases of already-released handles (counted no-ops).
    pub(crate) fn double_releases(&self) -> u64 {
        self.double_releases
    }

    /// Releases of handles this tracker never issued,
    /// including the null handle (counted no-ops).
    pub(crate) fn unknown_releases(&self) -> u64 {
        self.unknown_releases
    }

    /// One greppable line, same house style as paintbudget's
    /// and bufferpool's `stats_line` (the PERF.md §9
    /// missing-signal family). Field order is the stable
    /// contract; future fields append at the end only.
    pub(crate) fn stats_line(&self) -> String {
        format!(
            "lifetimes: live={} live_bytes={} peak_bytes={} registered={} released={} double_releases={} unknown_releases={}",
            self.live_count(),
            self.live_bytes,
            self.peak_bytes,
            self.registered_count(),
            self.released_count(),
            self.double_releases,
            self.unknown_releases
        )
    }
}

impl Default for LifetimeTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_release_accounting() {
        let mut t = LifetimeTracker::new();
        let a = t.register("card-tex", "texture", 100);
        let b = t.register("depth-target", "render-target", 300);
        let c = t.register("atlas-page", "texture", 200);
        assert_eq!((a.id(), b.id(), c.id()), (1, 2, 3));
        assert!(!a.is_null());
        assert_eq!(t.registered_count(), 3);
        assert_eq!(t.live_count(), 3);
        assert_eq!(t.released_count(), 0);
        assert_eq!(t.live_bytes(), 600);
        assert!(t.is_live(a) && t.is_live(b) && t.is_live(c));

        assert!(t.release(b), "first release of a live handle succeeds");
        assert_eq!(t.live_bytes(), 300);
        assert_eq!(t.live_count(), 2);
        assert_eq!(t.released_count(), 1);
        assert!(!t.is_live(b));
        assert!(t.is_live(a) && t.is_live(c));
        // Peak remembers the 600 high-water mark.
        assert_eq!(t.peak_bytes(), 600);
    }

    #[test]
    fn double_release_is_a_counted_noop_not_a_panic() {
        let mut t = LifetimeTracker::new();
        let h = t.register("staging", "buffer", 256);
        assert!(t.release(h));
        let (live, peak, released) = (t.live_bytes(), t.peak_bytes(), t.released_count());
        assert!(!t.release(h), "second release reports false");
        assert!(!t.release(h), "and so does the third");
        assert_eq!(t.double_releases(), 2);
        assert_eq!(t.unknown_releases(), 0);
        // Nothing else moved: the entry was retired exactly once.
        assert_eq!(t.live_bytes(), live);
        assert_eq!(t.peak_bytes(), peak);
        assert_eq!(t.released_count(), released);
        assert_eq!(t.live_count(), 0);
    }

    #[test]
    fn unknown_and_null_handles_are_counted_noops() {
        let mut t = LifetimeTracker::new();
        let h = t.register("real", "texture", 64);
        // Handles the tracker never issued: the tests live in
        // the module, so they can fabricate what callers cannot.
        assert!(!t.release(Handle { id: 9999 }));
        assert!(!t.release(Handle { id: 0 }));
        assert_eq!(t.unknown_releases(), 2);
        assert_eq!(t.double_releases(), 0);
        assert!(Handle { id: 0 }.is_null());
        assert!(!t.is_live(Handle { id: 9999 }));
        assert!(!t.is_live(Handle { id: 0 }));
        // The real entry is untouched by all of the above.
        assert!(t.is_live(h));
        assert_eq!(t.live_bytes(), 64);
        assert_eq!(t.released_count(), 0);
    }

    #[test]
    fn peak_is_a_high_water_mark() {
        let mut t = LifetimeTracker::new();
        let a = t.register("a", "texture", 500);
        assert_eq!(t.peak_bytes(), 500);
        let b = t.register("b", "texture", 250);
        assert_eq!(t.live_bytes(), 750);
        assert_eq!(t.peak_bytes(), 750);
        assert!(t.release(a));
        assert!(t.release(b));
        assert_eq!(t.live_bytes(), 0);
        assert_eq!(t.peak_bytes(), 750, "releases never lower the peak");
        // A later, smaller working set does not move it either.
        let _c = t.register("c", "texture", 100);
        assert_eq!(t.live_bytes(), 100);
        assert_eq!(t.peak_bytes(), 750);
        // Only exceeding the old mark does.
        let _d = t.register("d", "texture", 700);
        assert_eq!(t.live_bytes(), 800);
        assert_eq!(t.peak_bytes(), 800);
    }

    #[test]
    fn snapshot_is_sorted_bytes_desc_id_asc_and_flags_released() {
        let mut t = LifetimeTracker::new();
        let _a = t.register("small", "texture", 100);
        let big1 = t.register("big-early", "texture", 900);
        let _c = t.register("mid", "buffer", 400);
        let _big2 = t.register("big-late", "texture", 900);
        assert!(t.release(big1));
        let snap = t.snapshot();
        // Bytes descending; the two 900-byte entries tie-break
        // by registration order (id 2 before id 4).
        let order: Vec<(u64, u64, bool)> =
            snap.iter().map(|e| (e.id, e.bytes, e.released)).collect();
        assert_eq!(
            order,
            vec![(2, 900, true), (4, 900, false), (3, 400, false), (1, 100, false)]
        );
        // Snapshot entries are full value copies, not views.
        assert_eq!(snap[0].name, "big-early");
        assert_eq!(snap[0].class, "texture");
        // And a snapshot does not disturb the tracker.
        assert_eq!(t.live_bytes(), 100 + 400 + 900);
    }

    #[test]
    fn leaked_lists_only_never_released() {
        let mut t = LifetimeTracker::new();
        let a = t.register("kept-a", "texture", 300);
        let b = t.register("freed", "texture", 800);
        let _c = t.register("kept-c", "buffer", 500);
        assert!(t.release(b));
        let leaked = t.leaked();
        let names: Vec<&str> = leaked.iter().map(|e| e.name.as_str()).collect();
        // Same order as snapshot (bytes desc): 500 before 300.
        assert_eq!(names, vec!["kept-c", "kept-a"]);
        assert!(leaked.iter().all(|e| !e.released));
        assert_eq!(leaked.len(), t.live_count());
        // Releasing everything empties the leak report.
        assert!(t.release(a));
        assert!(t.release(t.snapshot().iter().find(|e| e.name == "kept-c").map(|e| Handle { id: e.id }).expect("kept-c present")));
        assert!(t.leaked().is_empty());
        assert_eq!(t.live_count(), 0);
    }

    #[test]
    fn duplicate_names_are_distinct_entries() {
        let mut t = LifetimeTracker::new();
        let h1 = t.register("card-tex", "texture", 128);
        let h2 = t.register("card-tex", "texture", 128);
        assert_ne!(h1, h2, "identity is the handle, never the name");
        assert_eq!(t.registered_count(), 2);
        assert!(t.release(h1));
        assert!(t.is_live(h2), "releasing one namesake leaves the other live");
        assert_eq!(t.leaked().len(), 1);
        assert_eq!(t.leaked()[0].id, h2.id());
    }

    #[test]
    fn zero_byte_registrations_are_tracked() {
        let mut t = LifetimeTracker::new();
        let h = t.register("marker", "handle", 0);
        assert_eq!(t.registered_count(), 1);
        assert_eq!(t.live_bytes(), 0);
        assert_eq!(t.peak_bytes(), 0);
        assert!(t.is_live(h));
        assert!(t.release(h));
        assert_eq!(t.released_count(), 1);
    }

    #[test]
    fn byte_totals_saturate_instead_of_overflowing() {
        let mut t = LifetimeTracker::new();
        let _a = t.register("huge", "texture", u64::MAX);
        let b = t.register("one-more", "texture", 1);
        assert_eq!(t.live_bytes(), u64::MAX, "saturating, not wrapping or panicking");
        assert_eq!(t.peak_bytes(), u64::MAX);
        assert!(t.release(b));
        assert_eq!(t.live_bytes(), u64::MAX - 1);
    }

    #[test]
    fn replay_is_deterministic() {
        // The same operation sequence on two fresh trackers
        // must produce identical snapshots, leaked lists and
        // stats lines — order comes from ids and bytes only.
        fn run_script() -> (Vec<Entry>, Vec<Entry>, String) {
            let mut t = LifetimeTracker::new();
            let mut handles = Vec::new();
            for (name, class, bytes) in [
                ("n1", "texture", 512),
                ("n2", "buffer", 128),
                ("n3", "texture", 512),
                ("n4", "render-target", 2048),
                ("n5", "buffer", 128),
            ] {
                handles.push(t.register(name, class, bytes));
            }
            assert!(t.release(handles[1]));
            assert!(t.release(handles[3]));
            assert!(!t.release(handles[3])); // double release, counted
            (t.snapshot(), t.leaked(), t.stats_line())
        }
        assert_eq!(run_script(), run_script());
    }

    #[test]
    fn stats_line_format_is_stable() {
        let mut t = LifetimeTracker::new();
        let a = t.register("a", "texture", 100);
        let _b = t.register("b", "texture", 250);
        assert!(t.release(a));
        assert!(!t.release(a)); // one double release
        assert!(!t.release(Handle { id: 77 })); // one unknown release
        assert_eq!(
            t.stats_line(),
            "lifetimes: live=1 live_bytes=250 peak_bytes=350 registered=2 released=1 double_releases=1 unknown_releases=1"
        );
    }
}
