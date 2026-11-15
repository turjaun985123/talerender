# TaleRender V2 — Memory architecture (as implemented)

Target context (master prompt §4): the primary machine has
~6.7 GiB usable RAM shared with a Vega 7 iGPU. Every mechanism
below is bounded; none of them ever changes resolution, fps,
encoding, or any pixel — they change how often work re-runs and
how much is retained. Anything stronger than that is labelled
CONTRACT.

## 1. Paint budget — `src/paintbudget.rs` + `film.rs::PaintCache` (WIRED)

- One painted character canvas is 2560×2816 RGBA =
  `CANVAS_BYTES = 28,835,840` bytes (`paintbudget.rs`).
- The cache (`film.rs`) stores `HashMap<String, Rc<Painted>>`
  plus a FIFO `VecDeque<String>` order. **Eviction today is
  FIFO**, governed by two simultaneous limits:
  - entry cap `DEFAULT_ENTRY_CAP = 160`, overridable with
    `TALERENDER_PAINT_CACHE`;
  - byte budget: default `min(entry_cap × CANVAS_BYTES,
    BUDGET_CEILING_BYTES)` where
    `BUDGET_CEILING_BYTES = 2048 MiB (2 GiB)`, overridable with
    `TALERENDER_PAINT_BUDGET_MB` (`0` disables byte enforcement,
    leaving the entry cap alone).
- `PaintBudget` accounts hits, misses, evictions, re-renders
  (a miss on a previously evicted key), current and peak bytes,
  and prints them on the end-of-run `paint-cache:` line composed
  by `film.rs` with `runlog`'s summary.
- Pixel-neutrality: painting is a pure function of the exact
  state key (`film.rs::state_key`, f32 bit patterns), so eviction
  costs time only. The budget module never sees a pixel.
- **WP1.1 byte-aware LRU is NOT implemented.** V2's plan
  (master prompt §5.1) replaces FIFO with recency-on-hit LRU
  under the same byte budget. At this HEAD there is no LRU code
  in `film.rs` (the cache still pops from the front of the
  `VecDeque`). The Batch 2 patch spec
  `docs/integration/V2-WP1-LRU-SPEC.md` **landed during this
  wave** (682 lines, untracked) — it is a spec for the lead to
  apply, not applied code: eviction in the running tree is
  still FIFO.

## 2. Buffer pool — `src/bufferpool.rs` (POLICY ONLY, NO CALLER)

- Size-classed reuse policy for readback staging buffers:
  `class_of(size)` rounds up to a multiple of
  `ROW_ALIGNMENT = 256`; the class is the identity on every real
  readback size.
- Known sizes: `FRAME_READBACK_BYTES = 4352 × 1920 = 8,355,840`
  (1080×1920 RGBA8 with padded rows);
  `DEPTH16_READBACK_BYTES = 8704 × 1920 = 16,711,680`.
- Default retained budget `DEFAULT_BUDGET_BYTES = 64 MiB`,
  overridable with `TALERENDER_BUFFER_POOL_MB` (`from_env`).
  Tracks in-flight/retained/peak bytes, hits/misses/evictions,
  and a `stats_line()`.
- The module is generic over an opaque handle `H` and never
  touches wgpu. **It has no production caller**: every readback
  still allocates its staging buffer inside `gpucore::Readback`,
  and gpucore's ledger comments state the pool "has no
  production caller" (if `Readback` ever adopts it, retained
  buffers register in the VRAM ledger once, at retention).
  Reuse would be pixel-neutral by construction (a staging
  buffer is fully rewritten by `copy_texture_to_buffer` before
  it is read) — that is the design argument, not a measured fact.

## 3. Texture atlas — `src/atlas.rs` (PACKING ONLY, NO CALLER)

- MaxRects packer (Best-Short-Side-Fit), deterministic: entries
  sorted by (area desc, w desc, h desc, id asc); same input set
  → same placements regardless of input order.
- Standard page: `MAX_PAGE_DIM = 8192`; production canvas
  `CANVAS_W × CANVAS_H = 2560 × 2816`; each entry reserves a
  cell of (w + 2P)×(h + 2P) with `DEFAULT_PADDING = 2` texels of
  edge-replicated gutter, so bilinear taps stay pixel-identical
  to the unatlased path (the module header gives the full
  argument). A uniform-canvas page holds a 3×2 grid.
- `AtlasPack` reports placed/cell/page texels, utilisation, and
  byte totals (`page_bytes`, `total_bytes`, `stats_line`).
- **Not wired:** `film.rs::Renderer::upload_dyn` still creates a
  fresh 2560×2816 texture (+ view + bind groups) per card per
  frame and re-uploads the full canvas. The atlas is the pure
  packing half awaiting that wiring.

## 4. VRAM ledger — `src/gpucore.rs` (WIRED)

The ledger counts *renderer-tracked* GPU allocations from their
descriptors; it is a lower bound by construction (driver tiling/
alignment beyond the 256-byte row rule, pipelines, shader
modules, bind groups and samplers are not priced). Full line
grammar and honesty classes are in `GPU.md`.

- Classes (`MemClass`, VRAM-TELEMETRY-W56 §4): **A** frame
  targets, **B** shadow pair, **C** bloom chain, **D** PMREM,
  **E** uploads, **F** readback (peak-side only — never enters
  current), **G** buffers.
- API: `mem_alloc_texture` / `mem_free_texture` /
  `mem_alloc_buffer` / `mem_free_buffer` / `mem_note_readback`,
  priced by `tex_bytes(w, h, format, samples)` from wgpu's own
  block-size table. State lives in a `Mutex<GpuMem>` with
  saturating arithmetic; a poisoned lock is recovered, never
  cascaded into a render failure.
- `accounted_bytes` = sum of A–E + G; peak is a watermark.
  Reference point recorded at landing (`f9429a8`, design pack
  VRAM-IMPL-W84): 220,930,688 B steady / 229,286,528 B peak for
  the standard staging — a ledger figure, not a driver reading.
- `MemScope` is the lifetime guard for per-frame transients: on
  drop it restores the transient classes (E, G) to their
  snapshot; standing classes (A–D) registered inside the scope
  survive; the peak watermark is never restored.
- Host memory is never ingested: paint-cache canvases are CPU
  `Vec<u8>` reported on their own `paint-cache:` line, and
  buffer-pool bytes are not registered (no production caller).
- Registration is enforced mechanically:
  `tools/ci/check_mem_registration.py` fails CI if a texture/
  buffer creation site is silent.

## 5. Memory-pressure profiles — `src/mempressure.rs` (REGISTERED, NOT WIRED)

Contract C1, implemented as a pure policy module. No render
path consults it yet; the values below are what a consumer
*would* get when the lead wires it.

| | LowMemory | Balanced | HighPerformance |
|---|---|---|---|
| `paint_budget_bytes()` | 2 GiB (exactly `paintbudget::BUDGET_CEILING_BYTES`) | 3 GiB (1.5×) | 4 GiB (2×) |
| `frame_queue_capacity()` | 2 | 4 | 8 |
| `preload_enabled()` | false | true | true |

- Selection (`resolve_profile`, pure): CLI arg >
  `TALERENDER_MEM_PROFILE` env > machine default, where the
  machine default is LowMemory when `/proc/meminfo` `MemTotal`
  ≤ 8 GiB, else Balanced; a meminfo read/parse failure also
  yields Balanced. Invalid spellings fall through to the next
  source; nothing panics. Spellings: `low-memory`, `balanced`,
  `high-performance` (shared with `config.rs`'s validation set).
- `level(accounted_bytes, peak_rss_mb) -> PressureLevel`:
  accounted ≥ 75% of the profile paint budget → Elevated,
  ≥ 100% → Critical; peak RSS ≥ 2× budget → Elevated, ≥ 3× →
  Critical; the result is the worse of the two signals.
  `None`/NaN/negative RSS is ignored. `level()` is a signal,
  not an actuator — it evicts and blocks nothing by itself.
- Interaction the wiring must respect (from the module's own
  delivery note): `PaintBudget::from_env`'s
  `TALERENDER_PAINT_BUDGET_MB` stays the operator override;
  `MemPressure::paint_budget_bytes()` is the profile default
  ceiling it feeds, not a replacement.
- Profiles never change output quality, resolution, fps or
  encoding (C1 law) — they bound caches, queues and preload only.

## 6. Dirty-region policy — `src/dirtyrect.rs` (REGISTERED, NOT WIRED)

Contract WP1.2, pure logic (no wgpu types, no I/O):

- `Rect { x, y, w, h }` is half-open texel space.
  `DirtyRects` accumulates via `add` / `add_region` /
  `add_point` / `add_clipped` / `add_full` / `absorb`
  (multi-character union) and `add_diff` / `from_diff`
  (byte-diff over RGBA rows against the previous frame).
- `merged()` coalesces overlapping **and edge-adjacent** rects
  into bounding boxes (corner-touching and 1-px-gapped rects do
  not merge; an L-shape over-covers to its bounding box — stated
  behaviour). Output is sorted by (y, x), ties by (h, w), and is
  a pure function of the set of added rects, never insertion
  order.
- Fallback: `should_fall_back()` is true iff merged coverage is
  **strictly greater** than `FALLBACK_COVERAGE = 0.60` (exactly
  0.60 is not a fallback; pinned by a unit test). On fallback
  the planned upload is the full canvas; otherwise
  `planned_upload_bytes` = merged area × bytes-per-pixel
  (`CARD_BYTES_PER_PIXEL = 4` for the card textures).
- **No upload is saved by this module alone** (its own delivery
  note): `upload_dyn` creates a fresh texture every frame, so
  there is no resident texture to patch. Partial uploads pay
  only once one texture per dynamic slot is kept alive across
  frames with the previous frame's pixels retained. When wired
  as sketched, it is pixel-neutral by construction (merged
  rects are a superset of the changed texels).

## 7. Lifetimes — as implemented today

**Landing in this wave:** `src/lifetimes.rs` (612 lines,
untracked, not registered in `main.rs`) — WP1.4's
`LifetimeTracker`: pure bookkeeping over named resources
(`register(name, class, bytes) -> Handle`, `release(Handle)`,
deterministic `snapshot()` sorted by bytes desc / registration
id asc, `leaked()` = registered-never-released, `live_bytes()` /
peak totals). The tracker owns no resource itself (no wgpu
types, no Drop side effects) — it is a diagnostic ledger in the
same relationship to resource owners as `paintbudget.rs` has
to `PaintCache`. Until the lead registers and wires it, the
implemented lifetime mechanisms are the ones below:

- **Paint cache:** entries are `Rc<Painted>` owned by
  `film.rs::PaintCache`; eviction removes the map entry and the
  FIFO record together (`on_evict` accounting in the same step).
  One cache per run (film render or telemetry run), so all
  counters are per-run.
- **GPU ledger scopes:** `gpucore::MemScope` (above) is the
  explicit transient-lifetime mechanism for classes E/G.
- **Readback buffers:** allocated per `Readback::collect` today;
  the reuse lifetime policy that would govern them is
  `bufferpool.rs` (§2, no caller).
- **Streaming buffers (unwired):** `streamq::PacketPool` reuses
  frame `Vec<u8>`s only at exactly the needed capacity —
  oversized/undersized buffers are dropped, never reshaped;
  retention is capped by the constructor (`0` disables pooling).
  It is single-owner (`&mut self`), not internally synchronised.
- **FFmpeg sink (unwired):** dropping an `FfmpegSink` without
  `finish()` kills and reaps the child; the `.part` file is left
  on disk for the caller (the sink never deletes output files).
- **Assets (metadata only):** `assetstream::Manifest` tracks
  residency as bookkeeping — `mark_resident` loads nothing and
  `evict_unreferenced(keep)` frees nothing; it returns the
  evicted ids sorted and sums only *known* declared bytes
  (`resident_bytes()` is a stated undercount when `bytes` is
  absent, never an estimate).
- **Scheduler checkpoints (unwired):** `scheduler::
  append_checkpoint` flushes but does not fsync; a lost line
  only causes a re-render of a frame whose bytes are a pure
  function of its index, so the weakness is safe by construction.
