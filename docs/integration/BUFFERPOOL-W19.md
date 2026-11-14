# Buffer pool for readback staging — W19 design note

**Seat:** Wave 2 W19 (Phase 3). **Delivered:** `src/bufferpool.rs`
(new file, uncommitted — the lead wires it). **Nothing else
touched.** The module is verified standalone: `rustc --edition
2021 --test src/bufferpool.rs` → **14 passed, 0 failed**. (One
dead-code warning in the standalone build only — `from_env` and
`budget_bytes` are the API surface the wiring consumes; it
disappears once gpucore/film call them, exactly as with W06's
module.)

## 1. Why, and the real sizes

Every texture→CPU readback allocates a fresh `COPY_DST |
MAP_READ` staging buffer, maps, unmaps and drops it. W07's
survey catalogues 7 sites; measured against the sources, they
allocate exactly TWO distinct sizes in production:

| Site(s) | Allocation | Bytes |
|---|---|---|
| film.rs:3292 — **per frame, 1,811× per film** | `padded_row_bytes(1080) * 1920` = 4352 × 1920 | **8,355,840** |
| main.rs:768, ab1.rs:1556, scenefile.rs:1451, scenefile.rs:2199, dof.rs:527 — once per process run | same formula at CLI dims (default 1080×1920) | **8,355,840** |
| dof.rs:605 packed-depth dump, Rgba8Unorm (default) | same | **8,355,840** |
| dof.rs:605 packed-depth dump, Rgba16Float (`TALERENDER_DOF_DEPTH_FORMAT=halffloat`) | 8704 × 1920 | **16,711,680** |

Two honest corrections to the audit's loose phrasing:

- There is **no canvas-size GPU readback**. The 2560×2816
  character canvas is painted on CPU (a `Vec<u8>`, W06's domain)
  and *uploaded*; nothing reads it back. The pooled sizes are
  the frame class and the depth class above, plus arbitrary
  small sizes from fixtures/probes/operator `--width/--height`.
- Six of the seven sites run **once per process** — pooling is
  worthless there (a pool of one, used once). The churn is the
  film path: 1,811 allocations of ~8 MiB per Onoda film,
  ~36,000 for a 20-minute longform at 30 fps.

## 2. What the module is

`BufferPool<H>` — pure CPU policy, generic over an opaque
handle (production: `wgpu::Buffer`; tests: integers). It owns
no wgpu knowledge and makes no wgpu calls; it decides, and
gpucore acts:

| Method | Meaning |
|---|---|
| `BufferPool::from_env()` / `new(budget_bytes)` | budget from `TALERENDER_BUFFER_POOL_MB` or explicit |
| `acquire(size) -> Acquire<H>` | `Reused { key, handle }` — a retained buffer of the size class; or `Fresh { key }` — caller creates `key.class_bytes` bytes |
| `release(key, handle) -> Vec<H>` | retain the handle, enforce the budget, return evicted handles (oldest first) for the caller to drop |
| `stats_line()` | `buffer-pool: retained_bytes=… free=… hits=… misses=… evictions=… peak_retained_bytes=… budget_bytes=…` |

**Size classes.** `class_of(size)` rounds up to a multiple of
256 B (wgpu's `COPY_BYTES_PER_ROW_ALIGNMENT`, restated as a
constant so the module compiles without the wgpu crate).
Deliberately NOT power-of-two classes:

1. **Accounting exactness** — the budget states retained bytes;
   pow2 slack (up to ~2× on adversarial sizes) would make the
   pool evict earlier than the budget the operator set. The
   256 B quantum overstates by ≤255 B/entry and is the
   **identity on every real readback size** (8,355,840,
   16,711,680 and the tight 8,294,400 are all multiples of
   256, pinned in the tests).
2. **The distribution is concentrated** — pow2 bucketing serves
   broad size spectra; a film run requests one size 1,811
   times. Class proliferation doesn't occur, and the byte
   budget + LRU bound whatever odd sizes do appear.
3. Honest note: for the two production sizes pow2 would waste
   <0.4% (both sit just under a power of two), so this was a
   judgement call on (1)–(2), not a blowout.

**Eviction** is LRU by last-release order, carried by free-list
position (append-on-release; no clock), deterministic by
construction — a scripted replay test runs the same
acquire/release sequence through two pools and demands
identical decision logs. Reuse within a class takes the
most-recently-released entry.

## 3. The default budget, tied to W06

```
TALERENDER_BUFFER_POOL_MB unset/unparseable -> 64 MiB
                          N                 -> N MiB
                          0                 -> pooling disabled (retain nothing)
```

W06's philosophy: a stated, small fraction of the target PC's
~6.7 GiB usable RAM, derived from the real working set. The
readback working set is **one buffer in flight** (the engine is
serial). 64 MiB retains the realistic set with headroom —
2 × frame (8,355,840) + 1 × depth16 (16,711,680) = 33,423,360 B,
or 4 × frame + 2 × depth16 = 66,846,720 B — at ~1% of usable
RAM and ~3% of W06's 2 GiB paint ceiling. Combined worst case
(paint 2 GiB + pool 64 MiB + renderer targets + assets + OS)
stays inside the envelope PAINT-BUDGET-W06 §3 already reasoned
about; CI runners (7 GB) likewise.

One deliberate divergence from W06's env semantics: there,
`TALERENDER_PAINT_BUDGET_MB=0` disables byte *enforcement* on a
cache that exists independently. The pool has no existence
apart from its budget, so `0` disables *the pool* — releases
return the handle as evicted immediately. Documented in the
module header.

Semantics decided, not defaulted (mirroring W06 §5):

- **Single oversize entry.** Budget below one frame buffer:
  the sole retained entry is never self-evicted (evicting it
  saves nothing in steady state — the next acquire recreates
  it, per frame, forever). It becomes evictable when a second
  entry is retained. Pinned by test.
- **Zero size** is never pooled (wgpu buffers may not be
  zero-sized anyway).
- **Per-device pools.** A buffer belongs to its creating
  device; a pool instance lives next to one device and never
  crosses. In practice: film's `Renderer` owns the pool beside
  its device/queue.

## 4. Integration shape — UNDER gpucore, not beside it

gpucore::Readback is the single place a staging buffer is
created today (W07). The pool changes only that creation and
the end of collection; the two-phase record/collect design
(and its load-bearing one-submit-per-frame batching) is
untouched:

```rust
// gpucore::Readback gains a key; record consults the pool.
pub(crate) fn record_pooled(
    device, encoder, texture, width, height, bpp, label,
    pool: &mut BufferPool<wgpu::Buffer>,
) -> Readback {
    let size = (padded_row_bytes(width, bpp) * height) as u64;
    let (buffer, key) = match pool.acquire(size) {
        Acquire::Reused { key, handle } => (handle, key),
        Acquire::Fresh { key } => (device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size: key.class_bytes,   // == size for all real sizes
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        }), key),
    };
    encoder.copy_texture_to_buffer(/* unchanged, into &buffer */);
    Readback { buffer, key, /* …existing fields */ }
}

pub(crate) fn collect(self, device, pool: &mut BufferPool<wgpu::Buffer>) -> Vec<u8> {
    let out = /* existing collect path, incl. unmap */;
    for evicted in pool.release(self.key, self.buffer) { drop(evicted); }
    out
}
```

`collect_padded` releases the same way, after its `unmap()`.
`readback_rgba` (the self-contained path) keeps the unpooled
`record` — a one-shot call has nothing to reuse across.

**What changes at the 7 sites: nothing visible.** Six of them
never pass a pool; their readbacks behave exactly as today.
The film site changes only by owning a `BufferPool<Buffer>`
field in `Renderer` and passing `&mut` at record/collect —
lifetimes only: a buffer may now be a retained one, dropped
later than the frame that used it. No signature, output,
printed line or exit code changes anywhere. Two cosmetic
consequences, stated for the record: a reused buffer keeps its
creation-time wgpu label (all seven sites already label theirs
near-identically — "readback buffer" ×4, "composer readback",
"dof-readback", "dof-depth-readback"), and GPU debuggers will
see fewer, longer-lived buffer objects.

## 5. Expected benefit — honestly bounded

**Not speed.** One `create_buffer` + drop of an 8 MiB staging
buffer is sub-millisecond-to-milliseconds of driver work
against seconds of render per frame (CI lavapipe) — under
0.1% of frame time, unmeasurable in any wall-clock comparison
this project can run. Anyone claiming a render-time win from
this module is overselling it. The wins are:

1. **Long-run stability.** 1,811 (or 36,000) same-size
   alloc/free cycles become a steady state of 1–2 retained
   buffers: no allocator fragmentation growth, no sawtooth in
   the driver's memory footprint across a multi-hour render.
2. **A smaller failure surface.** An allocation failure can
   today strike at frame 1,700 of 1,811; pooled, allocation
   happens at frame 1 or not at all.
3. **Observability.** Readback memory becomes a budgeted,
   counted quantity with a greppable stats line — the readback
   half of PERF.md §9 item 5's GPU-memory blind spot.
   (Textures and render targets remain unaccounted; the pool
   does not close that item alone.)

## 6. Risks

- **Retained memory on a shared-RAM iGPU.** On the target PC
  a retained wgpu buffer is system RAM the paint cache, the
  renderer and the OS cannot use, held for the whole run —
  including during paint-cache peaks. This is the entire
  reason the default budget is 64 MiB (~1% of RAM) rather than
  "a few frames' worth, whatever that grows to", and why
  eviction is byte-driven, not count-driven.
- **The release contract is load-bearing.** A handle may be
  released ONLY when unmapped and idle. gpucore's collect is
  the single release point and unmaps before returning in both
  paths — but the pool, being wgpu-agnostic, cannot verify
  mapped state. A future caller releasing early gets a wgpu
  validation error or worse; the contract is stated in the
  module header and §4 keeps exactly one release site.
- **Eviction cannot touch in-flight buffers** — by
  construction (the pool owns only released handles, and
  release follows `collect`'s `device.poll(Wait)`), but any
  future double-buffered readback design must re-check this:
  two readbacks in flight need two distinct handles, which the
  acquire path already guarantees (a retained handle is handed
  out at most once until re-released).
- **Class proliferation** under adversarial operator sizes is
  bounded by the byte budget + LRU; worst case is today's
  behaviour (create per use), never worse.

## 7. Pixel-neutrality argument

1. A staging buffer's bytes are fully rewritten by
   `copy_texture_to_buffer` before any read: the copy extent
   covers the whole buffer because the buffer's size IS
   `padded_row * height`, padding included. No byte of a
   previous frame can survive into `strip_padding`'s input.
2. The pool decides WHICH buffer object receives a copy and
   when buffer objects are destroyed. It holds no pixel data,
   computes no layout (the row arithmetic stays in gpucore,
   pinned against math.rs by gpucore's own tests), and cannot
   reorder copies — each acquire is answered before its copy
   is recorded, each release happens after its collect.
3. Therefore any budget setting — 64 MiB, tiny, or 0 (pooling
   off) — changes only buffer lifetimes and driver allocation
   counts, never a rendered pixel, a PNG byte, a printed
   verdict or an exit code.

## 8. What the lead must verify post-wiring

1. **Registration + CI green**: add `mod bufferpool;` in
   main.rs (next to `mod paintbudget;`); the 14 new tests run
   under the crate's normal `cargo test` once registered.
2. **Byte-compare probes, pre/post wiring**, same protocol as
   W06's: frames **1195–1204** and **795–814** must be
   sha256-identical with the pool at default AND disabled
   (`TALERENDER_BUFFER_POOL_MB=0`) AND squeezed (`=1`, forcing
   the oversize-sole-entry path on every frame).
3. **Telemetry unchanged**: `--film-telemetry` at
   `TALERENDER_PAINT_CACHE=24` prints the same verdict line as
   pre-wiring.
4. **Sanity-read one stats line** from a full-film render:
   misses small (≈ distinct readback sizes in the run),
   hits ≈ frames rendered, retained ≤ max(budget, one frame
   buffer), peak_retained ≥ retained. (No RSS or speed claim
   is made from this VM; the PERF protocol run on the user's
   PC is where the steady-state footprint gets its reading.)
