//! streamq — WP4 streaming core: bounded frame queue, frame-
//! buffer reuse pool, and lock-free stream accounting.
//!
//! =================================================================
//! DELIVERY NOTE — Batch 1, Seat 3 (WP4 streaming core)
//! =================================================================
//! FILES CREATED:
//!   - src/streamq.rs  (this file; the ONLY file this seat wrote)
//!
//! CONTRACT POINTS IMPLEMENTED (V2-CONTRACTS.md §3, contract C3,
//! queue half; master prompt §8):
//!   - `FramePacket` exactly as C3 spells it:
//!     { index: u32, width: u32, height: u32, rgba: Vec<u8> },
//!     carrying TIGHT RGBA8 rows — the byte shape
//!     `gpucore::Readback::collect` returns (padding already
//!     stripped by `gpucore::strip_padding`): rgba.len() must
//!     equal width * height * 4. `byte_len()` reports the
//!     actual payload length; `validate()` checks the invariant.
//!   - `frame_queue(capacity)` over
//!     `std::sync::mpsc::sync_channel` — bounded ALWAYS. The
//!     producer BLOCKS on a full queue (backpressure); frames
//!     are never dropped and never accumulate unboundedly
//!     (master prompt §8: no full-film RAM accumulation on the
//!     6.7 GiB target). Capacity comes from C1
//!     (`MemPressure::frame_queue_capacity`: LowMemory 2 /
//!     Balanced 4 / HighPerformance 8) at the call site.
//!     Capacity 0 is REJECTED: `debug_assert!` fires in debug
//!     builds, and the value is clamped to 1 in all builds, so
//!     a release build can never silently create a rendezvous
//!     (zero-buffer) channel the caller did not ask for.
//!   - `PacketPool`: bounded reuse pool for the frame buffers
//!     themselves, so the streaming producer stops allocating a
//!     fresh ~8 MiB Vec per frame. Single-format by design (one
//!     stream has one fixed WxH — the ffstream child is spawned
//!     with `-s WxH`): reuse requires EXACTLY the needed
//!     capacity; oversized/undersized buffers are DROPPED,
//!     never truncated/extended/reshaped to fit (see `release`
//!     and `acquire` docs). Retention is bounded by the
//!     constructor cap (`new(max_retained)`); cap 0 disables
//!     pooling (every release drops, every acquire allocates).
//!   - `StreamStats`: AtomicU64 counters { pushed, popped,
//!     bytes } with `snapshot()` — cheap, lock-free, shareable
//!     across the producer/consumer threads via Arc. `bytes`
//!     counts payload bytes ONCE, at push time.
//!
//! WIRING THE LEAD MUST DO:
//!   1. mod line — in src/main.rs, with the other mod decls
//!      (anchor: after `mod scenefile;`):
//!          mod streamq;
//!   2. Producer call site (film render loop, where
//!      `gpucore::Readback::collect` returns the tight rgba
//!      Vec today): obtain the queue ONCE before the loop from
//!      the C1 profile —
//!          let (tx, rx) = crate::streamq::frame_queue(
//!              mem_pressure.frame_queue_capacity());
//!      then per frame: build
//!      `FramePacket { index, width, height, rgba }`, call
//!      `stats.record_push_packet(&packet)`, then
//!      `tx.send(packet)` — the BLOCKING send is the
//!      backpressure; do not replace it with try_send + drop.
//!   3. Consumer call site (src/ffstream.rs, Batch 1 Seat 4):
//!      `rx.recv()` per frame; `packet.validate()` before
//!      writing `packet.rgba` to the FFmpeg rawvideo stdin
//!      (a failed validate is a frame-count/length error for
//!      ffstream to surface, not bytes to write); after the
//!      write, `stats.record_pop_packet(&packet)` and return
//!      the buffer with `pool.release(packet.rgba)`; the next
//!      producer-side buffer comes from
//!      `pool.acquire(width, height)`.
//!   4. No Cargo.toml change: std only, zero new dependencies.
//!
//! TESTS INCLUDED (#[cfg(test)], CPU-only, run by CI at
//! integration — this seat cannot compile locally):
//!   - queue_is_bounded_and_fails_try_send_when_full
//!   - queue_delivers_packets_in_order
//!   - validate_catches_wrong_length (+ byte_len/expected size)
//!   - pool_reuse_returns_same_capacity_buffer (pointer-proven
//!     reuse, returned buffer re-zeroed)
//!   - pool_allocates_zeroed_when_empty
//!   - pool_cap_is_enforced
//!   - pool_drops_oversized_undersized_and_truncated_buffers
//!   - stats_count_correctly (incl. snapshot independence)
//!
//! KNOWN LIMITATIONS:
//!   - Not yet wired into any render path: `gpucore::Readback::
//!     collect` still allocates its own Vec via `strip_padding`,
//!     so PacketPool has no production caller until the lead
//!     lands the producer wiring above (or a later readback-pool
//!     integration). The pool is policy + storage only, exactly
//!     like bufferpool.rs slots under gpucore.
//!   - PacketPool is single-owner (`&mut self`), not internally
//!     synchronised: one pool per streaming thread that owns it.
//!     Only FramePackets (through the queue) and StreamStats
//!     (via Arc) cross threads.
//!   - `frame_queue(0)` panics in debug builds via
//!     debug_assert (by design — see above); callers must pass
//!     the C1 capacity, which is never 0.
//!   - Stats use Ordering::Relaxed: they are accounting for
//!     logs/finalisation checks, not synchronisation. A final
//!     frame-count reconciliation (pushed vs popped vs the
//!     ffstream child's written count) is ffstream's close-out
//!     job, per C3's sink half.
//! =================================================================
//!
//! Pixel law: nothing in this module touches a rendered pixel.
//! It moves already-rendered tight RGBA bytes from producer to
//! consumer in order, unchanged.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, SyncSender};

// ---------------------------------------------------------------------
// Frame size arithmetic
// ---------------------------------------------------------------------

/// Expected tight-RGBA byte count for a `width` x `height` frame
/// (4 bytes per pixel), computed in u128 so no u32 dimension
/// pair can overflow the multiplication. `None` only when the
/// result cannot fit in a `usize` (impossible for any real frame;
/// the production frame is 1080x1920 = 8,294,400 bytes).
fn expected_rgba_bytes(width: u32, height: u32) -> Option<usize> {
    let n = width as u128 * height as u128 * 4u128;
    usize::try_from(n).ok()
}

// ---------------------------------------------------------------------
// FramePacket (contract C3)
// ---------------------------------------------------------------------

/// One rendered frame in flight from the native renderer to the
/// FFmpeg consumer: tight RGBA8 rows, `width * height * 4` bytes,
/// in deterministic frame order (`index` is the film frame
/// number). Produced from `gpucore::Readback::collect`'s output.
pub(crate) struct FramePacket {
    pub index: u32,
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

impl FramePacket {
    pub(crate) fn new(index: u32, width: u32, height: u32, rgba: Vec<u8>) -> FramePacket {
        FramePacket { index, width, height, rgba }
    }

    /// Actual payload size in bytes (`rgba.len()`).
    pub(crate) fn byte_len(&self) -> usize {
        self.rgba.len()
    }

    /// The invariant every consumer may rely on: the payload is
    /// exactly one tight RGBA8 frame of the stated dimensions.
    /// Checked in u128 so adversarial dimensions cannot wrap the
    /// expected size into a false pass.
    pub(crate) fn validate(&self) -> bool {
        expected_rgba_bytes(self.width, self.height)
            .map(|expected| expected == self.rgba.len())
            .unwrap_or(false)
    }
}

impl std::fmt::Debug for FramePacket {
    // Hand-written: the derived impl would dump the whole ~8 MiB
    // payload into any log or panic message that formats a packet.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FramePacket")
            .field("index", &self.index)
            .field("width", &self.width)
            .field("height", &self.height)
            .field("rgba_len", &self.rgba.len())
            .finish()
    }
}

// ---------------------------------------------------------------------
// Bounded frame queue (contract C3)
// ---------------------------------------------------------------------

/// Create the bounded producer→consumer frame channel.
///
/// `capacity` is the maximum number of frames buffered between
/// producer and consumer; it MUST come from the C1 memory-
/// pressure profile (`frame_queue_capacity`). When the buffer is
/// full, `SyncSender::send` blocks — that blocking IS the
/// backpressure that bounds streaming memory.
///
/// Capacity 0 is rejected: it would create a rendezvous channel
/// (zero buffering, send blocks until a recv is already waiting),
/// which no caller in this design asks for. A `debug_assert!`
/// fires in debug builds; in all builds the capacity is clamped
/// to a minimum of 1 so a bad value degrades to the tightest
/// real queue instead of changing the channel's semantics.
pub(crate) fn frame_queue(
    capacity: usize,
) -> (SyncSender<FramePacket>, Receiver<FramePacket>) {
    debug_assert!(capacity > 0, "frame_queue capacity 0 rejected (clamped to 1)");
    std::sync::mpsc::sync_channel(capacity.max(1))
}

// ---------------------------------------------------------------------
// PacketPool — bounded reuse of frame buffers
// ---------------------------------------------------------------------

/// Bounded reuse pool for frame payload buffers.
///
/// Why: a 1080x1920 RGBA frame is 8,294,400 bytes; allocating
/// one fresh Vec per frame for a 1,811-frame film is pure churn
/// the allocator and the page tables pay for, on a 6.7 GiB
/// machine. The pool keeps a small, CAPPED number of released
/// buffers and hands them back for the next frame.
///
/// Single-format law: one stream has one frame size for its
/// whole life, so the pool serves exactly one buffer size at a
/// time. A buffer is reusable ONLY when its capacity is exactly
/// the needed `width * height * 4` AND its length equals its
/// capacity (the canonical full-frame shape every producer
/// buffer has). Anything else — oversized, undersized, or
/// truncated (`len != capacity`) — is DROPPED, never resized,
/// truncated or extended to fit: silent reshaping would hide a
/// producer bug as a corrupt frame.
///
/// Reused buffers are returned fully ZEROED (fresh allocations
/// are zeroed too), so a packet buffer never leaks a previous
/// frame's bytes into a partially written frame.
///
/// Single-owner: all methods take `&mut self`. One pool per
/// owning thread; the pool itself never crosses threads.
pub(crate) struct PacketPool {
    /// Maximum number of released buffers retained. 0 disables
    /// pooling entirely (release always drops, acquire always
    /// allocates) — the low-memory escape hatch.
    max_retained: usize,
    /// Retained buffers, most-recently-released LAST; acquire
    /// pops from the back (stack discipline: the hottest buffer
    /// is reused first, and the choice is deterministic).
    buffers: Vec<Vec<u8>>,
}

impl PacketPool {
    pub(crate) fn new(max_retained: usize) -> PacketPool {
        PacketPool { max_retained, buffers: Vec::new() }
    }

    /// The constructor cap on retained buffers.
    pub(crate) fn max_retained(&self) -> usize {
        self.max_retained
    }

    /// Buffers currently retained and available for reuse.
    pub(crate) fn retained(&self) -> usize {
        self.buffers.len()
    }

    /// Get a buffer for one `width` x `height` RGBA frame:
    /// a retained buffer of EXACTLY the needed capacity, re-
    /// zeroed, when one is available; otherwise a fresh zeroed
    /// allocation of `width * height * 4` bytes.
    ///
    /// Retained buffers of any other capacity are dropped by
    /// this call (single-format law above): they belong to a
    /// previous frame size and can never serve this stream.
    ///
    /// Panics only if `width * height * 4` cannot fit in a
    /// `usize` — unreachable for any frame this engine renders.
    pub(crate) fn acquire(&mut self, width: u32, height: u32) -> Vec<u8> {
        let needed = expected_rgba_bytes(width, height)
            .expect("frame dimensions overflow usize byte count");
        if needed == 0 {
            // A zero-pixel frame has no buffer worth pooling.
            self.buffers.clear();
            return Vec::new();
        }
        // Drop every retained buffer that is not exactly this
        // stream's canonical shape (wrong capacity, or truncated
        // so len != capacity): never reshape them to fit.
        self.buffers
            .retain(|b| b.capacity() == needed && b.len() == needed);
        match self.buffers.pop() {
            Some(mut buf) => {
                buf.fill(0);
                buf
            }
            None => vec![0u8; needed],
        }
    }

    /// Return a buffer for possible reuse.
    ///
    /// The buffer is retained ONLY when all of these hold:
    /// pooling is enabled (cap > 0), the pool is below its cap,
    /// the buffer is non-empty, the buffer is in canonical
    /// shape (`len == capacity` — a truncated or over-extended
    /// buffer is dropped, not repaired), and its capacity
    /// matches the size this pool already retains (single-
    /// format law; the first retained buffer sets that size).
    /// In every other case the buffer is dropped. `release`
    /// never resizes the buffer it is given.
    pub(crate) fn release(&mut self, buf: Vec<u8>) {
        if self.max_retained == 0 || self.buffers.len() >= self.max_retained {
            return; // dropped: pooling off, or cap reached
        }
        if buf.is_empty() || buf.len() != buf.capacity() {
            return; // dropped: empty, truncated or over-extended
        }
        if let Some(existing) = self.buffers.first() {
            if existing.capacity() != buf.capacity() {
                return; // dropped: other format's buffer
            }
        }
        self.buffers.push(buf);
    }
}

// ---------------------------------------------------------------------
// StreamStats — lock-free accounting
// ---------------------------------------------------------------------

/// Plain snapshot of the stream counters, taken by
/// `StreamStats::snapshot`. Suitable for end-of-run lines and
/// the ffstream frame-count reconciliation at close.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct StreamSnapshot {
    pub pushed: u64,
    pub popped: u64,
    pub bytes: u64,
}

/// AtomicU64 stream accounting: frames pushed into the queue,
/// frames popped by the consumer, and payload bytes pushed.
///
/// Cheap and lock-free by design — three relaxed atomic adds on
/// the per-frame path, no Mutex, no contention design needed at
/// one producer + one consumer. Relaxed ordering is sufficient:
/// these are statistics, never synchronisation; a reader that
/// needs the final totals reads them after joining the threads,
/// which provides the happens-before edge.
///
/// `bytes` is counted ONCE, at push time (payload bytes entering
/// the stream). Pop does not add to it, so `bytes` is the total
/// streamed payload, not double-counted throughput.
pub(crate) struct StreamStats {
    pushed: AtomicU64,
    popped: AtomicU64,
    bytes: AtomicU64,
}

impl StreamStats {
    pub(crate) const fn new() -> StreamStats {
        StreamStats {
            pushed: AtomicU64::new(0),
            popped: AtomicU64::new(0),
            bytes: AtomicU64::new(0),
        }
    }

    /// Record one frame entering the queue, carrying `bytes`
    /// payload bytes.
    pub(crate) fn record_push(&self, bytes: u64) {
        self.pushed.fetch_add(1, Ordering::Relaxed);
        self.bytes.fetch_add(bytes, Ordering::Relaxed);
    }

    /// `record_push` for a whole packet (its `byte_len()`).
    pub(crate) fn record_push_packet(&self, packet: &FramePacket) {
        self.record_push(packet.byte_len() as u64);
    }

    /// Record one frame leaving the queue (consumed).
    pub(crate) fn record_pop(&self) {
        self.popped.fetch_add(1, Ordering::Relaxed);
    }

    /// `record_pop` for a whole packet (byte count ignored —
    /// bytes are counted at push; see the type doc).
    pub(crate) fn record_pop_packet(&self, _packet: &FramePacket) {
        self.record_pop();
    }

    pub(crate) fn snapshot(&self) -> StreamSnapshot {
        StreamSnapshot {
            pushed: self.pushed.load(Ordering::Relaxed),
            popped: self.popped.load(Ordering::Relaxed),
            bytes: self.bytes.load(Ordering::Relaxed),
        }
    }
}

impl Default for StreamStats {
    fn default() -> StreamStats {
        StreamStats::new()
    }
}

// ---------------------------------------------------------------------
// Tests — CPU only. No GPU, no FFmpeg, no filesystem.
// ---------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc::TrySendError;

    /// A valid small packet: 4x2 px = 32 payload bytes.
    fn packet(index: u32) -> FramePacket {
        FramePacket::new(index, 4, 2, vec![0u8; 32])
    }

    #[test]
    fn queue_is_bounded_and_fails_try_send_when_full() {
        let (tx, rx) = frame_queue(2);
        // Fill the queue. std's sync_channel may hold the bound
        // plus a per-sender slot, so fill until Full rather than
        // asserting an exact internal count — the CONTRACT is
        // that it is bounded and refuses once full.
        let mut sent = 0u32;
        loop {
            match tx.try_send(packet(sent)) {
                Ok(()) => {
                    sent += 1;
                    assert!(sent <= 8, "queue accepted unbounded sends");
                }
                Err(TrySendError::Full(_)) => break,
                Err(TrySendError::Disconnected(_)) => panic!("receiver alive, must not disconnect"),
            }
        }
        assert!(sent >= 2, "capacity 2 must accept at least 2 packets, got {sent}");
        // Still full on the next attempt — it stays bounded.
        assert!(matches!(tx.try_send(packet(999)), Err(TrySendError::Full(_))));
        // Draining one slot makes room again (backpressure, not
        // loss): the refused packet was never queued or dropped
        // silently — the caller still owns it via the error.
        let first = rx.recv().expect("queued packet");
        assert_eq!(first.index, 0);
        tx.try_send(packet(1000)).expect("room after one recv");
    }

    #[test]
    fn queue_delivers_packets_in_order() {
        let (tx, rx) = frame_queue(4);
        for i in 0..4u32 {
            tx.send(packet(i)).expect("send within capacity");
        }
        for i in 0..4u32 {
            let p = rx.recv().expect("queued packet");
            assert_eq!(p.index, i);
            assert!(p.validate());
        }
    }

    #[test]
    fn validate_catches_wrong_length() {
        // Correct: 1080x1920 tight RGBA = 8,294,400 bytes (the
        // production frame size from gpucore's readback).
        let good = FramePacket::new(0, 1080, 1920, vec![0u8; 8_294_400]);
        assert!(good.validate());
        assert_eq!(good.byte_len(), 8_294_400);
        // One byte short and one byte long both fail.
        let short = FramePacket::new(1, 1080, 1920, vec![0u8; 8_294_399]);
        assert!(!short.validate());
        let long = FramePacket::new(2, 4, 2, vec![0u8; 33]);
        assert!(!long.validate());
        // Empty payload against non-empty dimensions fails; a
        // zero-dimension packet with an empty payload is exact.
        assert!(!FramePacket::new(3, 4, 2, Vec::new()).validate());
        assert!(FramePacket::new(4, 0, 0, Vec::new()).validate());
        // Padded-row bytes are NOT a valid tight packet: 1080 px
        // rows padded to 4352 (gpucore::padded_row_bytes) x 1920
        // = 8,355,840 must fail — validate guards the tight-row
        // contract, not the readback staging size.
        let padded = FramePacket::new(5, 1080, 1920, vec![0u8; 8_355_840]);
        assert!(!padded.validate());
    }

    #[test]
    fn pool_reuse_returns_same_capacity_buffer() {
        let mut pool = PacketPool::new(4);
        let buf = pool.acquire(4, 2); // 32 bytes, fresh
        assert_eq!(buf.len(), 32);
        assert_eq!(buf.capacity(), 32);
        let ptr = buf.as_ptr();
        pool.release(buf);
        assert_eq!(pool.retained(), 1);
        // Dirty the retained buffer's replacement path: acquire
        // must hand back THE SAME allocation (pointer-proven),
        // at exactly the same capacity.
        let again = pool.acquire(4, 2);
        assert_eq!(again.as_ptr(), ptr, "acquire must reuse the released buffer");
        assert_eq!(again.capacity(), 32);
        assert_eq!(again.len(), 32);
        assert_eq!(pool.retained(), 0);
    }

    #[test]
    fn pool_allocates_zeroed_when_empty() {
        let mut pool = PacketPool::new(2);
        let buf = pool.acquire(1080, 1920);
        assert_eq!(buf.len(), 8_294_400);
        assert!(buf.iter().all(|&b| b == 0), "fresh buffer must be zeroed");
        // A reused buffer is re-zeroed too: release it full of
        // non-zero bytes, re-acquire, expect zeros — no previous
        // frame may leak through the pool.
        let mut dirty = pool.acquire(4, 2);
        dirty.iter_mut().for_each(|b| *b = 0xAB);
        pool.release(dirty);
        let clean = pool.acquire(4, 2);
        assert!(clean.iter().all(|&b| b == 0), "reused buffer must be re-zeroed");
    }

    #[test]
    fn pool_cap_is_enforced() {
        let mut pool = PacketPool::new(2);
        assert_eq!(pool.max_retained(), 2);
        pool.release(vec![0u8; 32]);
        pool.release(vec![0u8; 32]);
        assert_eq!(pool.retained(), 2);
        // Third release exceeds the cap: dropped, not retained.
        pool.release(vec![0u8; 32]);
        assert_eq!(pool.retained(), 2);
        // Cap 0 disables pooling entirely.
        let mut off = PacketPool::new(0);
        off.release(vec![0u8; 32]);
        assert_eq!(off.retained(), 0);
        assert_eq!(off.acquire(4, 2).len(), 32);
        assert_eq!(off.retained(), 0);
    }

    #[test]
    fn pool_drops_oversized_undersized_and_truncated_buffers() {
        let mut pool = PacketPool::new(4);
        // Establish the pool's format with a canonical buffer.
        pool.release(vec![0u8; 32]);
        assert_eq!(pool.retained(), 1);
        // Oversized (64) and undersized (16) buffers for this
        // pool's format are dropped on release, never reshaped.
        pool.release(vec![0u8; 64]);
        pool.release(vec![0u8; 16]);
        assert_eq!(pool.retained(), 1);
        // Truncated buffer (capacity 32, length cut to 16) is
        // dropped, not repaired by resizing.
        let mut truncated = vec![0u8; 32];
        truncated.truncate(16);
        assert_eq!(truncated.capacity(), 32);
        pool.release(truncated);
        assert_eq!(pool.retained(), 1);
        // Acquiring a DIFFERENT size never reuses the retained
        // 32-byte buffer reshaped: it allocates the exact new
        // size and the stale buffer is dropped (single-format).
        let other = pool.acquire(2, 2); // 16 bytes
        assert_eq!(other.len(), 16);
        assert_eq!(other.capacity(), 16);
        assert_eq!(pool.retained(), 0);
        // Empty buffers are never retained.
        pool.release(Vec::new());
        assert_eq!(pool.retained(), 0);
    }

    #[test]
    fn stats_count_correctly() {
        let stats = StreamStats::new();
        assert_eq!(stats.snapshot(), StreamSnapshot { pushed: 0, popped: 0, bytes: 0 });
        stats.record_push_packet(&packet(0));
        stats.record_push_packet(&packet(1));
        stats.record_push(100);
        let snap = stats.snapshot();
        assert_eq!(snap.pushed, 3);
        assert_eq!(snap.bytes, 32 + 32 + 100);
        assert_eq!(snap.popped, 0);
        stats.record_pop();
        stats.record_pop_packet(&packet(0));
        let snap = stats.snapshot();
        assert_eq!(snap.popped, 2);
        // Pop does not double-count bytes.
        assert_eq!(snap.bytes, 164);
        // Snapshots are independent copies: later records do not
        // mutate an earlier snapshot.
        stats.record_push(1);
        assert_eq!(snap.pushed, 3);
        assert_eq!(stats.snapshot().pushed, 4);
    }
}
