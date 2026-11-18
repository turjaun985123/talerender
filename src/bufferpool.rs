//! Buffer pool policy — size-classed reuse of readback staging
//! buffers (Wave 2 seat W19, Phase 3).
//!
//! Every texture→CPU readback in the engine allocates a fresh
//! `COPY_DST | MAP_READ` staging buffer, maps it, unmaps it and
//! drops it. On the film path that is one ~8 MiB allocation per
//! frame — 1,811 of them for the Onoda film, ~36,000 for a
//! 20-minute longform. The wgpu calls themselves now live in
//! exactly one place (`gpucore::Readback`, seat W07); this module
//! is the CPU-side LIFECYCLE POLICY that slots UNDER it: which
//! sizes to retain after use, under what reuse key, and when to
//! evict. It never touches wgpu — it is generic over an opaque
//! handle `H` (in production `wgpu::Buffer`; in the tests below,
//! plain integers), so the whole policy is unit-testable on CPU
//! with `rustc --test`, no GPU, no wgpu crate.
//!
//! Pixel-neutrality law: a staging buffer's bytes are FULLY
//! rewritten by `copy_texture_to_buffer` before they are ever
//! read — the copy's extent is the whole buffer, because the
//! buffer's size IS `padded_row * height` (gpucore::Readback::
//! record_bpp). Padding bytes included. A reused buffer therefore
//! cannot leak a previous frame's byte into any output; which
//! physical buffer a frame's copy lands in is unobservable in the
//! pixels, the PNG bytes, the printed lines and the exit codes.
//! The pool only changes buffer LIFETIMES.
//!
//! Call contract (mirrors paintbudget.rs's): every handle handed
//! out by `acquire` must be returned by exactly one `release`,
//! and only when the buffer is unmapped and idle (in gpucore
//! terms: at the end of `Readback::collect`, after `unmap`). A
//! pool is per-DEVICE: a wgpu buffer belongs to the device that
//! created it, so a pool instance must never outlive or cross
//! devices — in practice one pool per render run, owned by the
//! same struct that owns the device (film.rs's `Renderer`).

use std::collections::VecDeque;

/// wgpu's `COPY_BYTES_PER_ROW_ALIGNMENT`, restated as a plain
/// constant so this module compiles without the wgpu crate (the
/// standalone-test requirement). Every real readback size is a
/// multiple of it: buffer size = padded_row * height, and
/// padded_row is itself a multiple of 256 (gpucore::padded_row_
/// bytes). The class quantum below is therefore the IDENTITY on
/// every size the engine actually requests — see `class_of`.
pub(crate) const ROW_ALIGNMENT: u64 = 256;

/// Byte size of the production frame readback: 1080x1920 RGBA8
/// with padded rows — padded_row_bytes(1080, 4) = 4352 (pinned by
/// gpucore's tests against math::padded_row_bytes), times 1920.
/// This is the size film.rs:3292, ab1.rs:1556, main.rs:768,
/// scenefile.rs:1451 + 2199 and dof.rs:527 allocate today.
pub(crate) const FRAME_READBACK_BYTES: u64 = 4352 * 1920; // 8,355,840

/// Byte size of the packed-depth dump readback at fp16
/// (dof.rs:605, `TALERENDER_DOF_DEPTH_FORMAT=halffloat`):
/// padded_row_bytes(1080, 8) = 8704, times 1920. (At the default
/// Rgba8Unorm packing the dump is FRAME_READBACK_BYTES.)
pub(crate) const DEPTH16_READBACK_BYTES: u64 = 8704 * 1920; // 16,711,680

/// Default retained-bytes budget: 64 MiB.
///
/// Derived the way PAINT-BUDGET-W06 derives its ceiling — a small,
/// stated fraction of the target PC's ~6.7 GiB of usable RAM
/// (shared with the Vega 7 iGPU) — but from the readback working
/// set, not from an entry count. The engine is serial: exactly
/// ONE readback is in flight at any moment in a run. The budget
/// holds the realistic retained set with headroom:
///
///   2 x FRAME_READBACK_BYTES + 1 x DEPTH16_READBACK_BYTES
///     = 33,423,360 B (~31.9 MiB)          — double-buffered
///       frame class plus the largest diagnostic class;
///   the full 64 MiB also fits 4 x frame + 2 x depth16
///     = 66,846,720 B.
///
/// 64 MiB is ~1% of the PC's usable RAM and ~3% of W06's 2 GiB
/// paint-cache ceiling; the combined worst case (paint 2 GiB +
/// pool 64 MiB + renderer targets + assets + OS) stays inside
/// the envelope W06 §3 already reasoned about. GitHub's
/// ubuntu-22.04 runners (7 GB) sit in the same envelope.
///
/// Override: `TALERENDER_BUFFER_POOL_MB` (see `from_env`).
pub(crate) const DEFAULT_BUDGET_BYTES: u64 = 64 * 1024 * 1024;

const MIB: u64 = 1024 * 1024;

/// The size class a request of `size` bytes belongs to: `size`
/// rounded UP to a multiple of ROW_ALIGNMENT.
///
/// Why not power-of-two classes (the classic allocator answer)?
/// Three reasons, in order of weight:
///
/// 1. ACCOUNTING EXACTNESS. The budget's job is to state, in
///    bytes, how much GPU/shared memory the pool retains. Pow2
///    classes make retained accounting overstate the truth by
///    the class slack — up to ~2x for adversarial sizes — so the
///    pool would evict earlier than the budget the operator set.
///    With the 256-byte quantum the overstatement is at most
///    255 bytes per entry, and ZERO for every real readback size
///    (all are multiples of 256 already: FRAME_READBACK_BYTES,
///    DEPTH16_READBACK_BYTES, and the tight 1080x1920 size
///    8,294,400 are all pinned in the tests).
/// 2. THE SIZE DISTRIBUTION IS CONCENTRATED, not broad. Pow2
///    classes exist to serve many sizes with few buckets. A film
///    run requests ONE size 1,811 times; a scene run requests
///    one or two. Class proliferation — pow2's raison d'etre —
///    does not occur, and the byte budget bounds the retained
///    set when odd sizes do appear (fixtures, probes, operator
///    --width/--height).
/// 3. NO REUSE IS LOST. Reuse requires a retained buffer at
///    least as large as the request whose copy layout is
///    computed from the REQUEST's own padded row; a same-class
///    buffer is always exactly that. (Honest note: for the two
///    production sizes pow2 would waste < 0.4% — 8,355,840 sits
///    just under 2^23, 16,711,680 just under 2^24 — so this was
///    a judgement call on (1) and (2), not a blowout.)
pub(crate) fn class_of(size: u64) -> u64 {
    size.div_ceil(ROW_ALIGNMENT) * ROW_ALIGNMENT
}

/// Identifies the class a handle belongs to. Handed out by
/// `acquire` (both arms) and handed back to `release`, so the
/// pool files the handle correctly even when the caller's
/// request size differed from the class size by the quantum.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(crate) struct PoolKey {
    /// Class size in bytes == the allocated size of any buffer
    /// filed under this key (the quantum makes them equal).
    pub(crate) class_bytes: u64,
}

/// The decision `acquire` returns: reuse this retained handle,
/// or create a fresh buffer of `key.class_bytes` bytes.
pub(crate) enum Acquire<H> {
    Reused { key: PoolKey, handle: H },
    Fresh { key: PoolKey },
}

/// One retained (free) buffer. LRU order is carried by the
/// entry's POSITION in the pool's free deque (front = least-
/// recently-released): entries are only ever appended at the
/// back on release, so deque order IS release order — no
/// timestamps, no clock, and two runs issuing the same
/// acquire/release sequence make the same decisions (pinned by
/// the replay test below).
struct FreeEntry<H> {
    key: PoolKey,
    handle: H,
}

/// Size-classed buffer pool POLICY: reuse decisions, byte
/// accounting, budget enforcement. Owns only handles the caller
/// has released; handles in flight belong to the caller.
pub(crate) struct BufferPool<H> {
    /// Retained-bytes ceiling; 0 = pooling DISABLED (retain
    /// nothing — every release returns the handle as evicted).
    /// Divergence from paintbudget's "0 disables enforcement",
    /// deliberate: W06's 0 turns off a ceiling on a cache that
    /// exists independently; the pool has no existence apart
    /// from its budget, so 0 turns the pool off. The DEFAULT is
    /// a positive number either way.
    budget_bytes: u64,
    /// Free entries, front = least-recently-released (LRU),
    /// back = most-recently-released (MRU).
    free: VecDeque<FreeEntry<H>>,
    retained_bytes: u64,
    peak_retained_bytes: u64,
    hits: u64,
    misses: u64,
    releases: u64,
    evictions: u64,
}

impl<H> BufferPool<H> {
    pub(crate) fn new(budget_bytes: u64) -> BufferPool<H> {
        BufferPool {
            budget_bytes,
            free: VecDeque::new(),
            retained_bytes: 0,
            peak_retained_bytes: 0,
            hits: 0,
            misses: 0,
            releases: 0,
            evictions: 0,
        }
    }

    /// Production constructor: budget from
    /// `TALERENDER_BUFFER_POOL_MB` — unset or unparseable yields
    /// DEFAULT_BUDGET_BYTES, `N` yields N MiB, `0` disables
    /// pooling (retain nothing). Mirrors the
    /// TALERENDER_PAINT_BUDGET_MB pattern in paintbudget.rs.
    pub(crate) fn from_env() -> BufferPool<H> {
        let raw = std::env::var("TALERENDER_BUFFER_POOL_MB").ok();
        BufferPool::new(budget_bytes_from_env_value(raw.as_deref()))
    }

    /// GOV-CORE (GPUFIRST Batch 3) production constructor:
    /// the retained-bytes budget is the SMALLER of this
    /// module's 64 MiB default and the resource governor's
    /// buffer-pool share of the RAM budget (resource.rs
    /// BUFFER_POOL_SHARE_PCT), so a low RAM cap shrinks what
    /// the pool retains. An explicit TALERENDER_BUFFER_POOL_MB
    /// still wins over both (the operator override this module
    /// has always honoured). The pool's own enforcement —
    /// LRU eviction of retained handles past the budget, with
    /// the single-live-entry rule — is unchanged, so this
    /// only ever costs a buffer re-creation, never a pixel.
    /// NOTE: the pool has no production caller in the tree
    /// today (gpucore's W84 §7.2 records why); this constructor
    /// is the governor-wired entry point for the caller that
    /// adopts it.
    pub(crate) fn from_governor() -> BufferPool<H> {
        if let Ok(raw) = std::env::var("TALERENDER_BUFFER_POOL_MB") {
            return BufferPool::new(budget_bytes_from_env_value(Some(&raw)));
        }
        let gov = crate::resource::governor();
        if gov.enabled() {
            BufferPool::new(DEFAULT_BUDGET_BYTES.min(gov.buffer_pool_budget_bytes()))
        } else {
            BufferPool::new(DEFAULT_BUDGET_BYTES)
        }
    }

    /// Decide the buffer for a request of `size` bytes.
    ///
    /// If a retained buffer of the request's class exists, the
    /// MOST-recently-released one is handed back (stack
    /// discipline within a class: it is the one least likely to
    /// have been paged or fragmented away, and the choice is
    /// deterministic). Otherwise the caller must create a fresh
    /// buffer of `key.class_bytes` bytes. A zero-size request is
    /// always Fresh and is never retained (wgpu buffers may not
    /// be zero-sized at all).
    pub(crate) fn acquire(&mut self, size: u64) -> Acquire<H> {
        let key = PoolKey { class_bytes: class_of(size) };
        if key.class_bytes > 0 {
            if let Some(idx) = self
                .free
                .iter()
                .rposition(|e| e.key.class_bytes == key.class_bytes)
            {
                let entry = self.free.remove(idx).expect("pool index from rposition");
                self.retained_bytes = self.retained_bytes.saturating_sub(entry.key.class_bytes);
                self.hits += 1;
                return Acquire::Reused { key: entry.key, handle: entry.handle };
            }
        }
        self.misses += 1;
        Acquire::Fresh { key }
    }

    /// Return a handle to the pool after use, then enforce the
    /// budget: while retained bytes exceed the budget AND more
    /// than one entry is retained, the least-recently-released
    /// entries are evicted. Evicted handles are RETURNED (oldest
    /// first) for the caller to drop — the pool never destroys
    /// what it does not own the type of.
    ///
    /// The `len > 1` guard is W06's live-entry rule restated for
    /// buffers: a single retained entry larger than the whole
    /// budget is kept, because evicting it saves nothing in
    /// steady state — the next acquire of its class would just
    /// recreate it, per frame, forever. It becomes evictable the
    /// moment a second entry is retained. (With budget 0 the
    /// pool is disabled instead: the released handle itself is
    /// returned as evicted immediately.)
    pub(crate) fn release(&mut self, key: PoolKey, handle: H) -> Vec<H> {
        self.releases += 1;
        if self.budget_bytes == 0 || key.class_bytes == 0 {
            self.evictions += 1;
            return vec![handle];
        }
        self.retained_bytes = self.retained_bytes.saturating_add(key.class_bytes);
        if self.retained_bytes > self.peak_retained_bytes {
            self.peak_retained_bytes = self.retained_bytes;
        }
        self.free.push_back(FreeEntry { key, handle });
        let mut evicted = Vec::new();
        while self.retained_bytes > self.budget_bytes && self.free.len() > 1 {
            let entry = self.free.pop_front().expect("pool free list non-empty");
            self.retained_bytes = self.retained_bytes.saturating_sub(entry.key.class_bytes);
            self.evictions += 1;
            evicted.push(entry.handle);
        }
        evicted
    }

    /// Handles currently checked out (acquires minus releases).
    pub(crate) fn in_flight(&self) -> u64 {
        (self.hits + self.misses).saturating_sub(self.releases)
    }

    pub(crate) fn retained_bytes(&self) -> u64 {
        self.retained_bytes
    }

    pub(crate) fn peak_retained_bytes(&self) -> u64 {
        self.peak_retained_bytes
    }

    pub(crate) fn free_entries(&self) -> usize {
        self.free.len()
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

    pub(crate) fn budget_bytes(&self) -> u64 {
        self.budget_bytes
    }

    /// One greppable end-of-run line, same house style as
    /// paintbudget's `stats_line` (PERF.md §9's missing-signal
    /// family). The wiring in gpucore/film prints it next to the
    /// existing end-of-run lines; the field order is the stable
    /// contract.
    pub(crate) fn stats_line(&self) -> String {
        format!(
            "buffer-pool: retained_bytes={} free={} hits={} misses={} evictions={} peak_retained_bytes={} budget_bytes={}",
            self.retained_bytes,
            self.free.len(),
            self.hits,
            self.misses,
            self.evictions,
            self.peak_retained_bytes,
            self.budget_bytes
        )
    }
}

/// Pure form of the TALERENDER_BUFFER_POOL_MB parse, split out
/// for tests (env access itself is `from_env`).
/// - `None` (unset) or unparseable -> DEFAULT_BUDGET_BYTES.
/// - `"0"` -> 0, pooling DISABLED (retain nothing).
/// - `"N"` -> N MiB.
pub(crate) fn budget_bytes_from_env_value(raw: Option<&str>) -> u64 {
    match raw {
        Some(v) => match v.trim().parse::<u64>() {
            Ok(mb) => mb.saturating_mul(MIB),
            Err(_) => DEFAULT_BUDGET_BYTES,
        },
        None => DEFAULT_BUDGET_BYTES,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const F: u64 = FRAME_READBACK_BYTES;
    const D: u64 = DEPTH16_READBACK_BYTES;

    /// Handles in these tests are integer ids; each `Fresh`
    /// decision is materialised by the test taking the next id,
    /// exactly as the caller would create a new wgpu::Buffer.
    fn fresh_id(next: &mut u32) -> u32 {
        *next += 1;
        *next
    }

    #[test]
    fn real_readback_sizes_are_what_the_sites_allocate() {
        // Pinned from the sources, not from memory: padded row
        // for 1080 px at 4 bpp is 4352 (gpucore's own tests pin
        // padded_row_bytes(1080, 4) = 4352 against math.rs),
        // height 1920 (the production frame). 8 bpp fp16 depth:
        // padded row 8704.
        assert_eq!(F, 8_355_840);
        assert_eq!(D, 16_711_680);
        assert_eq!(F, 4352 * 1920);
        assert_eq!(D, 8704 * 1920);
        // The derived default budget fits the stated working set:
        // 2 frame + 1 depth16, and 4 frame + 2 depth16.
        assert!(2 * F + D <= DEFAULT_BUDGET_BYTES);
        assert!(4 * F + 2 * D <= DEFAULT_BUDGET_BYTES);
        assert_eq!(DEFAULT_BUDGET_BYTES, 64 * 1024 * 1024);
    }

    #[test]
    fn class_rounding_table() {
        // Identity on every real size (all multiples of 256):
        assert_eq!(class_of(F), F);
        assert_eq!(class_of(D), D);
        assert_eq!(class_of(1080 * 4 * 1920), 1080 * 4 * 1920); // tight rows, 8,294,400
        assert_eq!(class_of(256), 256);
        assert_eq!(class_of(512 * 512 * 4), 512 * 512 * 4); // small probe square
        // Quantum edges:
        assert_eq!(class_of(0), 0);
        assert_eq!(class_of(1), 256);
        assert_eq!(class_of(255), 256);
        assert_eq!(class_of(257), 512);
        assert_eq!(class_of(1000), 1024);
        assert_eq!(class_of(1023), 1024);
        assert_eq!(class_of(1025), 1280);
        // One byte past a real frame still lands in its own class
        // (accounting stays honest; this size never occurs in
        // practice because readback sizes are row-padded).
        assert_eq!(class_of(F + 1), F + 256);
    }

    #[test]
    fn first_acquire_is_fresh_then_reuse_hits() {
        let mut pool: BufferPool<u32> = BufferPool::new(DEFAULT_BUDGET_BYTES);
        let mut next = 0;
        let (key, h) = match pool.acquire(F) {
            Acquire::Fresh { key } => (key, fresh_id(&mut next)),
            Acquire::Reused { .. } => panic!("empty pool must decide Fresh"),
        };
        assert_eq!(key.class_bytes, F);
        assert_eq!(pool.misses(), 1);
        assert_eq!(pool.hits(), 0);
        assert_eq!(pool.in_flight(), 1);
        let evicted = pool.release(key, h);
        assert!(evicted.is_empty());
        assert_eq!(pool.retained_bytes(), F);
        assert_eq!(pool.free_entries(), 1);
        assert_eq!(pool.in_flight(), 0);
        // The film loop's steady state: 1,811 times, all hits.
        for _ in 0..10 {
            match pool.acquire(F) {
                Acquire::Reused { key: k2, handle: h2 } => {
                    assert_eq!(h2, h, "the same buffer comes back");
                    assert_eq!(k2, key);
                    assert!(pool.release(k2, h2).is_empty());
                }
                Acquire::Fresh { .. } => panic!("warm pool must reuse"),
            }
        }
        assert_eq!(pool.hits(), 10);
        assert_eq!(pool.misses(), 1);
        assert_eq!(pool.evictions(), 0);
        assert_eq!(pool.retained_bytes(), F);
    }

    #[test]
    fn classes_do_not_cross_serve() {
        let mut pool: BufferPool<u32> = BufferPool::new(DEFAULT_BUDGET_BYTES);
        let mut next = 100;
        // Retain one frame-class buffer.
        let (kf, hf) = match pool.acquire(F) {
            Acquire::Fresh { key } => (key, fresh_id(&mut next)),
            _ => unreachable!(),
        };
        pool.release(kf, hf);
        // A depth-class request must NOT take the frame buffer
        // (it is smaller than the request — serving it would
        // overflow the copy), and a smaller request must not take
        // it either (exact-class accounting, class_of doc §1).
        match pool.acquire(D) {
            Acquire::Fresh { key } => assert_eq!(key.class_bytes, D),
            Acquire::Reused { .. } => panic!("frame buffer must not serve depth class"),
        }
        match pool.acquire(1024) {
            Acquire::Fresh { key } => assert_eq!(key.class_bytes, 1024),
            Acquire::Reused { .. } => panic!("frame buffer must not serve small class"),
        }
        assert_eq!(pool.hits(), 0);
        assert_eq!(pool.misses(), 3);
        // And the frame buffer is still there for its own class.
        match pool.acquire(F) {
            Acquire::Reused { handle, .. } => assert_eq!(handle, hf),
            _ => panic!("frame buffer should still be retained"),
        }
    }

    #[test]
    fn quantum_unifies_near_sizes() {
        let mut pool: BufferPool<u32> = BufferPool::new(DEFAULT_BUDGET_BYTES);
        let (k, h) = match pool.acquire(1000) {
            Acquire::Fresh { key } => (key, 7u32),
            _ => unreachable!(),
        };
        assert_eq!(k.class_bytes, 1024);
        pool.release(k, h);
        // 1023 rounds to the same 1024 class and reuses.
        match pool.acquire(1023) {
            Acquire::Reused { key: k2, handle: h2 } => {
                assert_eq!(k2.class_bytes, 1024);
                assert_eq!(h2, 7);
            }
            _ => panic!("same class must reuse"),
        }
    }

    #[test]
    fn mru_within_class_lru_across_eviction() {
        // Small unit sizes keep the arithmetic readable.
        let mut pool: BufferPool<u32> = BufferPool::new(768); // exactly 3 x 256
        let rel = |pool: &mut BufferPool<u32>, id: u32| pool.release(PoolKey { class_bytes: 256 }, id);
        // Retain A(1), B(2), C(3) in that release order.
        assert!(rel(&mut pool, 1).is_empty());
        assert!(rel(&mut pool, 2).is_empty());
        assert!(rel(&mut pool, 3).is_empty());
        assert_eq!(pool.retained_bytes(), 768);
        // Reuse prefers the most-recently-released: C, then B.
        match pool.acquire(256) {
            Acquire::Reused { handle, .. } => assert_eq!(handle, 3),
            _ => unreachable!(),
        }
        match pool.acquire(256) {
            Acquire::Reused { handle, .. } => assert_eq!(handle, 2),
            _ => unreachable!(),
        }
        // Return C and B (C first), then retain a new D: budget
        // forces exactly one eviction, of the LRU entry — A.
        assert!(rel(&mut pool, 3).is_empty());
        assert!(rel(&mut pool, 2).is_empty());
        let evicted = rel(&mut pool, 9);
        assert_eq!(evicted, vec![1], "LRU entry (A) evicted, in order");
        assert_eq!(pool.evictions(), 1);
        assert_eq!(pool.retained_bytes(), 768);
    }

    #[test]
    fn eviction_returns_multiple_handles_oldest_first() {
        let mut pool: BufferPool<u32> = BufferPool::new(1000);
        let key = |b: u64| PoolKey { class_bytes: b };
        assert!(pool.release(key(256), 1).is_empty()); // retained 256
        assert!(pool.release(key(256), 2).is_empty()); // retained 512
        // Releasing a 768-class entry: 1280 > 1000 -> evict LRU
        // (id 1) -> 1024 > 1000 -> evict next (id 2) -> 768 fits,
        // and the big entry is now the sole (live) entry.
        let evicted = pool.release(key(768), 3);
        assert_eq!(evicted, vec![1, 2]);
        assert_eq!(pool.retained_bytes(), 768);
        assert_eq!(pool.evictions(), 2);
    }

    #[test]
    fn budget_zero_disables_pooling() {
        let mut pool: BufferPool<u32> = BufferPool::new(0);
        let (k, h) = match pool.acquire(F) {
            Acquire::Fresh { key } => (key, 42u32),
            _ => unreachable!(),
        };
        // Release hands the handle straight back as evicted.
        assert_eq!(pool.release(k, h), vec![42]);
        assert_eq!(pool.retained_bytes(), 0);
        assert_eq!(pool.free_entries(), 0);
        // And the next acquire is Fresh again — pooling is off.
        match pool.acquire(F) {
            Acquire::Fresh { .. } => {}
            _ => panic!("disabled pool must never reuse"),
        }
        assert_eq!(pool.hits(), 0);
    }

    #[test]
    fn single_oversize_entry_is_kept_until_a_second_arrives() {
        // Budget smaller than one frame buffer (an operator
        // squeezing memory): W06's live-entry rule — the sole
        // retained entry is never self-evicted.
        let mut pool: BufferPool<u32> = BufferPool::new(1_000_000);
        let (kf, hf) = match pool.acquire(F) {
            Acquire::Fresh { key } => (key, 5u32),
            _ => unreachable!(),
        };
        assert!(pool.release(kf, hf).is_empty(), "sole oversize entry retained");
        assert_eq!(pool.retained_bytes(), F);
        // It still serves reuse (that is the point of keeping it).
        let (kf2, hf2) = match pool.acquire(F) {
            Acquire::Reused { key, handle } => (key, handle),
            _ => panic!("oversize sole entry must be reusable"),
        };
        assert_eq!(hf2, hf);
        assert!(pool.release(kf2, hf2).is_empty());
        // A second, small entry arrives: NOW the frame entry is
        // evictable, and it is the LRU one.
        let evicted = pool.release(PoolKey { class_bytes: 256 }, 6);
        assert_eq!(evicted, vec![hf]);
        assert_eq!(pool.retained_bytes(), 256);
        match pool.acquire(F) {
            Acquire::Fresh { .. } => {}
            _ => panic!("evicted class must miss"),
        }
    }

    #[test]
    fn peak_retained_is_a_high_water_mark() {
        let mut pool: BufferPool<u32> = BufferPool::new(DEFAULT_BUDGET_BYTES);
        let key = PoolKey { class_bytes: 256 };
        pool.release(key, 1);
        pool.release(key, 2);
        assert_eq!(pool.retained_bytes(), 512);
        assert_eq!(pool.peak_retained_bytes(), 512);
        // Acquiring (checking out) lowers retained, not the peak.
        match pool.acquire(256) {
            Acquire::Reused { .. } => {}
            _ => unreachable!(),
        }
        assert_eq!(pool.retained_bytes(), 256);
        assert_eq!(pool.peak_retained_bytes(), 512);
    }

    #[test]
    fn zero_size_is_never_pooled() {
        let mut pool: BufferPool<u32> = BufferPool::new(DEFAULT_BUDGET_BYTES);
        let (k, h) = match pool.acquire(0) {
            Acquire::Fresh { key } => (key, 3u32),
            _ => unreachable!(),
        };
        assert_eq!(k.class_bytes, 0);
        assert_eq!(pool.release(k, h), vec![3]);
        assert_eq!(pool.retained_bytes(), 0);
        assert_eq!(pool.free_entries(), 0);
    }

    #[test]
    fn decision_replay_is_deterministic() {
        // A fixed script of acquires/releases; two fresh pools
        // must produce byte-identical decision logs (the LRU
        // order comes from release order, never a clock).
        fn run_script() -> Vec<String> {
            let mut pool: BufferPool<u32> = BufferPool::new(3 * 256);
            let mut next = 0u32;
            let mut log = Vec::new();
            let sizes = [256u64, 512, 256, 768, 256, 512, 256, 1024, 256];
            let mut held: Vec<(PoolKey, u32)> = Vec::new();
            for (i, &s) in sizes.iter().enumerate() {
                match pool.acquire(s) {
                    Acquire::Reused { key, handle } => {
                        log.push(format!("{i}:reuse:{handle}"));
                        held.push((key, handle));
                    }
                    Acquire::Fresh { key } => {
                        let h = fresh_id(&mut next);
                        log.push(format!("{i}:fresh:{h}"));
                        held.push((key, h));
                    }
                }
                if i % 2 == 1 {
                    let (k, h) = held.remove(0);
                    let ev = pool.release(k, h);
                    log.push(format!("{i}:evict:{ev:?}"));
                }
            }
            log.push(pool.stats_line());
            log
        }
        assert_eq!(run_script(), run_script());
    }

    #[test]
    fn env_value_parsing() {
        assert_eq!(budget_bytes_from_env_value(None), DEFAULT_BUDGET_BYTES);
        assert_eq!(budget_bytes_from_env_value(Some("0")), 0);
        assert_eq!(budget_bytes_from_env_value(Some("128")), 128 * MIB);
        assert_eq!(budget_bytes_from_env_value(Some(" 32 ")), 32 * MIB);
        assert_eq!(budget_bytes_from_env_value(Some("junk")), DEFAULT_BUDGET_BYTES);
        assert_eq!(budget_bytes_from_env_value(Some("")), DEFAULT_BUDGET_BYTES);
        assert_eq!(budget_bytes_from_env_value(Some("-5")), DEFAULT_BUDGET_BYTES);
    }

    #[test]
    fn governor_constructor_never_exceeds_the_default_budget() {
        // Machine-dependent below the cap (a tiny-RAM host's
        // governor share can be smaller), so the portable
        // claims are: bounded by the default, and positive
        // (pooling stays on under the governor's defaults).
        let pool: BufferPool<u32> = BufferPool::from_governor();
        assert!(pool.budget_bytes() <= DEFAULT_BUDGET_BYTES);
        assert!(pool.budget_bytes() > 0);
    }

    #[test]
    fn stats_line_format_is_stable() {
        let mut pool: BufferPool<u32> = BufferPool::new(1000);
        let (k, h) = match pool.acquire(256) {
            Acquire::Fresh { key } => (key, 1u32),
            _ => unreachable!(),
        };
        pool.release(k, h);
        match pool.acquire(256) {
            Acquire::Reused { key: k2, handle: h2 } => {
                pool.release(k2, h2);
            }
            _ => unreachable!(),
        }
        assert_eq!(
            pool.stats_line(),
            "buffer-pool: retained_bytes=256 free=1 hits=1 misses=1 evictions=0 peak_retained_bytes=256 budget_bytes=1000"
        );
    }
}
