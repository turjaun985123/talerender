# GPU-FIRST INTEG — how GPU paint plugs into production

Seat: INTEG, GPU-first TaleRender Phase 0. **Design only — no code.**
Repo verified at HEAD `bc12a5c2bbb33d93416f25eaebbe8fec8c4eb678`
(`git rev-parse HEAD`, this session). Every claim below cites the
code or document it rests on; statements that are reasoning rather
than reading are marked **[analysis]**.

Non-negotiable constraints (not variables in this design):

- **Parity law** — GPU paint may not lose quality: for any class
  routed to the GPU, the painted card must be byte-identical to
  the CPU paint of the same key, or the class is not routed.
- **Key-coverage law** — the cache key must name every field the
  painter consumes (`src/film.rs` `state_key` header comment,
  film.rs:2469ff; the sweat/tears precedent, §2.2).

---

## 1. PLUG POINT

### 1.1 The contract today

```rust
// src/film.rs:2372
type Painted = (Vec<u8>, u32, u32, (f32, f32), (f32, f32));
```

`.0` = RGBA8 pixels (2560×2816), `.1`/`.2` = w/h, `.3`/`.4` =
hand positions in stage coordinates — the exact return of
`paint_toon_full` (`src/toonfull.rs:699`, returning
`(p.px, cw, ch, hands.0, hands.1)` at toonfull.rs:1683, with
`cw = 1280*2, ch = 1408*2` at toonfull.rs:703).

The single production call site on the legacy film path:

```rust
// src/film.rs:2441, inside PaintCache::get_or_paint (film.rs:2421)
let v = std::rc::Rc::new(paint_toon_full(look, st));
```

`get_or_paint(key, look, st)` is called only from
`resolve_chars` (film.rs:2510), which first builds the
`FullState`, resolves held plates, computes
`key = state_key(c.look, &st, c.hold_l, c.hold_r)` (film.rs:2547),
then consumes the returned `Rc<Painted>` in exactly three ways:

1. `entry.4` (hand_r) → rifle placement (film.rs:2549, 2618).
2. `entry.clone()` → `CharInfo.painted` (film.rs:2590) — the
   telemetry consumer (§1.3).
3. `entry` pushed onto `dyn_tex: Vec<Rc<Painted>>` and a
   `BasicDraw { tex: TexSrc::Dyn(slot), alpha_test: 0.04, .. }`
   appended (film.rs:2601–2615), where `slot` is positional
   (`dyn_tex.len()`), not keyed.

### 1.2 Downstream: who copies pixels where, per frame

`Renderer::render(&mut self, spec, dyn_tex: &[Rc<Painted>])`
(film.rs:3164) drives the persistent pool **before** its
`MemScope` opens (film.rs:3170–3186, with the ledger reason
spelled out in the comment there):

```rust
let prev = self.prev_dyn[slot].as_ref().map(|rc| rc.0.as_slice());
self.dyn_pool.upload(&self.device, &self.queue, slot, &p.0, p.1, p.2, prev);
self.prev_dyn[slot] = Some(p.clone());
```

`DynTexturePool::upload` (`src/dyntex.rs`) owns one long-lived
`Rgba8UnormSrgb` texture per slot
(`DynTexturePool::create_slot`: format `CARD_FORMAT`,
usage `TEXTURE_BINDING | COPY_DST`, ledger class E via
`gpucore::mem_alloc_texture`). Per frame it plans
Full / Partial / Skip by **diffing CPU byte buffers**
(`SlotPlanner::plan` → `dirtyrect::DirtyRects::from_diff(prev,
px, ..)`) and issues `queue.write_texture` calls. The resulting
views (`dyn_pool.view(slot)`) are bound at
`TexSrc::Dyn(i) => &dyn_views[*i]` (film.rs:3421) for both the
basic card draw and the card-shadow pass (film.rs:3443).

So today, per character-frame, pixels travel:
**CPU paint → `Vec<u8>` (28.8 MB) → `write_texture` upload →
slot texture → card draws.** On a cache hit the first step is
skipped; the upload is skipped only when the pool's diff says
Skip (same allocation or no changed texels).

### 1.3 The other consumers of `Painted` pixels (the hard part)

Two consumers read `.0` **on the CPU**, and any GPU-resident
design must answer for both:

- **Telemetry overflow scan.** `run_film_telemetry` calls
  `resolve_chars`, then per character
  `scan_canvas_overflow(&ci.painted.0, ..)` (film.rs:4175).
  Telemetry is a **CPU-only mode — no device is ever created**
  (stated in `gpucore::mem_stats_line`'s doc, gpucore.rs:
  "Film telemetry prints NO line … telemetry is a CPU-only
  mode"). There is no `Renderer`, no device, no queue.
- **The pool's own diff** (above): `prev_dyn` retains the
  previous frame's `Rc<Painted>` per slot purely so its bytes
  can be diffed.

And two further direct callers of the painter exist **outside**
`get_or_paint`, which a plug at film.rs:2441 alone does not
touch:

- `src/scenefile.rs:752` — the generic scene path paints every
  character, every render, **uncached**:
  `let (pxbuf, pw, ph, hand_l, hand_r) = paint_toon_full(&look,
  &state); textures.push((pxbuf, pw, ph));`
- `src/filmtelemetry.rs:673` — the generic telemetry path paints
  directly for its cutouts.

**Consequence [analysis]:** the plug cannot be "replace the
function body at film.rs:2441 and nothing else". It must be a
*backend* the cache (and later scenefile) dispatches through,
with a CPU implementation that is today's code verbatim.

### 1.4 The device-availability problem

`PaintCache` owns no device: `PaintCache::new()` reads only env
vars (film.rs:2404–2419). `get_or_paint(&mut self, key, look,
st)` has no device/queue parameter. The `Renderer` owns the
only device/queue (film.rs:2677–2678), created in
`Renderer::new` via `gpucore::request_adapter_with_policy` +
`request_device_default` (film.rs:2733–2746). In `run_film` the
`Renderer` is constructed (film.rs:3868) before the cache
(film.rs:3880), so a device *exists* on the render path — but
on the telemetry path it never does (§1.3).

**Design rule D1:** `PaintCache` gains an *optional* paint
backend handle, `Option<GpuPaintCtx>`, supplied by the caller
that owns a device (the film render drivers), and **absent in
telemetry**, where every paint therefore takes the CPU path by
construction — telemetry's contract (overflow from real CPU
pixels) is untouched, not emulated.

### 1.5 Recommended contract change: a sum type, deferred paint

Keep `Painted` exactly as-is for the CPU path. Introduce
**[analysis — proposed types, names are the design's]**:

```rust
enum PaintedCard {
    Cpu(std::rc::Rc<Painted>),
    Gpu(std::rc::Rc<GpuPainted>),
}
struct GpuPainted {
    tex: wgpu::Texture,          // 2560x2816, Rgba8UnormSrgb
    view: wgpu::TextureView,
    w: u32, h: u32,              // Painted.1 / .2
    hand_l: (f32, f32),          // Painted.3
    hand_r: (f32, f32),          // Painted.4
    job: Option<PaintJob>,       // display list, until materialised
}
```

- Format is **not** negotiable: `Rgba8UnormSrgb`
  (`dyntex::CARD_FORMAT`) — the card pipelines, the dual
  alpha thresholds (0.04 colour / 0.5 shadow,
  compatibility-contract §2.3) and sRGB sampling behaviour are
  all built on it. Usage becomes
  `RENDER_ATTACHMENT | TEXTURE_BINDING` (`COPY_SRC` added only
  if a readback is ever demanded of a resident entry — §1.6).
- **Hands stay CPU scalars.** In the CPU painter the hands are
  set during arm drawing from pure f32 geometry
  (toonfull.rs:950–972). The GPU path's display-list builder
  computes the same FK on the CPU and returns the hands in the
  struct — hands never require a readback.
- **Deferred materialisation.** `get_or_paint` on a GPU-routed
  miss does *not* submit GPU work. It builds the display list
  (the geometry-side work both paths share), allocates the
  texture, stores `job: Some(..)` and returns the handle. The
  paint executes at the top of `Renderer::render`, in exactly
  the seam where the pool is driven today (film.rs:3170–3186,
  before the `MemScope`): for each `dyn_tex` entry —
  `Cpu` → `dyn_pool.upload(..)` as today; `Gpu` → if
  `job.is_some()`, record the paint pass into one encoder for
  all pending cards, submit once, clear `job`; the card's view
  is `gpu.view.clone()` instead of `dyn_pool.view(slot)`.
  Rationale: (a) the cache stays device-free (it stores jobs,
  not devices — D1 stays clean); (b) per-miss submit storms are
  avoided — one submit per frame for all cards; (c) eviction of
  an unmaterialised entry is just dropping a display list;
  (d) the frame's `Rc` in `dyn_tex` keeps the texture alive
  across the resolve→render gap, so mid-frame eviction cannot
  dangle a view.
- **What disappears for a GPU card:** the `Vec<u8>`, the
  `write_texture` upload, the `prev_dyn` retention and the
  entire dirty-rect diff for that slot. The card texture **is**
  the paint canvas (option A). The alternative, option B —
  paint into a cache texture then `copy_texture_to_texture`
  into the existing pool slot — preserves pool ownership but
  pays a 28.8 MB GPU→GPU copy per card-frame and keeps a
  second resident copy of every card; **[analysis]** it buys
  nothing once `dyn_views` can come from either source, and is
  recommended only as a fallback if direct binding hits an
  unforeseen pipeline constraint in Phase 1 prototyping.
- **What does not change:** `resolve_chars`' geometry
  (blob draw, card model, anchor compensation, rifle) reads
  only hands + slot indices; `TexSrc::Dyn(slot)` addressing is
  unchanged — `dyn_views` becomes a per-slot choice of view
  source, indexed identically.

### 1.6 CPU-pixel access for a GPU entry

If any future consumer needs `.0` of a GPU-resident entry, the
only sanctioned route is an explicit, counted readback through
`gpucore::Readback` / `gpucore::readback_rgba` (gpucore.rs —
record into an encoder, `collect` strips row padding). It is
a *diagnostic* operation (parity battery, defect attribution),
never a per-frame path: a per-frame readback reinstates the
full round-trip the design exists to remove (§4.3). Telemetry
never needs it (D1: telemetry has no backend, so it never
holds a `Gpu` entry).

---

## 2. KEY DISCIPLINE

### 2.1 Why no new key components — stated precisely

`state_key(look: usize, st: &FullState, hold_l, hold_r)`
(film.rs:2453) formats the look **registry index** plus the
f32 **bit patterns** (`v.to_bits()`) of every continuous
`FullState` channel, the `Legs` discriminant, `sit_drop`, the
two hold *names*, and — appended last by the 2026-11-15 fix —
`st.sweat, st.tears` (film.rs:2496–2505; the format has 23
components). The header comment states the coverage law:
"the key must name EVERY field the painter consumes, or two
different paints share a key and the cache stops being a pure
optimisation" (film.rs:2469ff).

The GPU path needs **no new key components if and only if**
it is a second *implementation of the same function*:
`paint(key) `is defined by the inputs `(FullLook registry
entry, FullState, hold plates)` — the look index names the
`FullLook` (all its fields are painter inputs, fixed per run
by the registry; film.rs:2480ff), the key body names the
`FullState`, and the hold names name the plates (resolved to
`Plate` pixels deterministically from `assets.plates` in
`resolve_chars`, film.rs:2540–2546). A GPU executor that
consumes **exactly** those inputs — the same display list the
CPU painter's geometry derives from them — is keyed by the
same string, because the key is a name for the *input tuple*,
not for the implementation. Implementation identity (CPU vs
GPU, adapter, class mask) must **never** enter the key: if it
did, the same input would occupy two entries and the cache
would silently double its working set; worse, a mask change
mid-run would orphan entries instead of re-routing paints.

Corollary: the routing decision (§4) is a function of the
*display list derived from the keyed inputs* (which prim
classes occur), evaluated after the key, never an input to it.

### 2.2 The audit procedure (the sweat/tears precedent, made law)

Precedent: the painter read `st.sweat` (toonfull.rs:775) and
`st.tears` (toonfull.rs:788) while `state_key` omitted both;
states differing only in sweat/tears collided and reused the
wrong paint. Found by the WP10 `charstate.rs` audit, disclosed
in V2-HANDOVER.md, fixed in `b1355b1` by appending both
booleans, proven by a full-film census (1,811 frames, zero
differing — the film never exercises sweat/tears-true states)
and by new regression tests in film.rs `state_key_tests`
(V2-HANDOVER.md, "Discovered defect" + RESOLVED note).

Any future GPU-path input — any field, uniform, table, or
external texture the GPU executor reads that the CPU painter
does not, or vice versa — must go through this procedure
**before** the input is consumed:

1. **Field audit.** Diff the GPU executor's consumed-input
   list against `paint_toon_full`'s, field by field, the way
   the 2026-11-15 audit was done against toonfull.rs (all 19
   `FullState` fields; `FullLook` via the registry index).
   The audit is a written artefact in the landing's
   integration note, not a claim in a commit message.
2. **Key first.** If the new input is not named by
   `state_key` (directly, or transitively via the look index /
   hold names), extend the key *in its own landing*, under the
   byte-probe protocol the sweat/tears fix used — a key change
   lands alone, never folded into a paint landing
   (V2-HANDOVER.md: "must land alone … with the differing
   frames quantified and reported").
3. **Regression test.** Add a `state_key_tests`-style test:
   two states differing only in the new input must produce
   different keys (the test that failed pre-fix in CI
   43703339048 and passed post-fix).
4. **Census.** Full-film census on the key-change landing;
   differing frames are expected exactly where the new input
   is exercised, and are reported by count, as the
   sweat/tears census reported zero.

No GPU executor may consume an input that has not passed
steps 1–3. This is the key-coverage law applied to the new
path; it is not optional process.

### 2.3 `canvas_anchor`: geometry-side, unchanged

`canvas_anchor(look, st) -> (f32, f32)` (toonfull.rs:403) is
a **pure CPU function of (look, state)** that rebuilds the
painted envelope from the painter's own geometry and returns
the whole-figure translation that keeps it inside the canvas.
Its own doc comment records the key fact: "The anchor consumes
only inputs the painter already consumes, so the paint-cache
key … needs no new component" (toonfull.rs:396–402).

It is applied in **two** places today, both CPU:

- inside the paint: `paint_toon_full` calls it at
  toonfull.rs:714 and folds `(anx, any)` into the
  stage→canvas `px` closure — i.e. it is part of the
  *transform stack*, not of any rasterised prim;
- at card placement: `resolve_chars` recomputes it
  (film.rs:2584) and counter-offsets the card model
  (`card_draw_model`, film.rs:2585–2589); scenefile.rs does
  the same at its card draw (comment at scenefile.rs:~790,
  "Canvas re-anchor compensation (B-1), as in film.rs
  resolve_chars").

**GPU design:** the anchor stays exactly where it is — CPU,
geometry-side. The display-list builder calls the *same*
`canvas_anchor(look, st)`, and the resulting translation
enters the GPU paint as a transform uniform / vertex-stage
offset applied to every prim of the card, in the same stack
position (after lean, as the outermost transform — the
position toonfull.rs:709–714 documents). The card-placement
counter-offset in `resolve_chars` is untouched, because it
already recomputes the anchor independently of who painted.

**What proves it:** (a) the existing `canvas_anchor_tests`
(toonfull.rs:1687) pin the CPU function — the GPU path calls
that function, it does not reimplement it, so the tests cover
the GPU path's anchor by construction; (b) the GPU executor
must **not** recompute the envelope in WGSL — the envelope
math uses `sin_cos` of lean and the painter's f32 geometry,
and a WGSL re-derivation would import exactly the
cross-adapter transcendental variance §5 excludes from the
paint; (c) the parity battery (§7) includes anchored states
(high-|lean|, high seats — the two families the anchor exists
for, toonfull.rs:365–374), so a misplaced anchor application
fails as pixel diffs at the canvas edges, where
`scan_canvas_overflow`'s border strip also guards it.

---

## 3. RESIDENCY

### 3.1 The numbers today

One card: 2560×2816 RGBA8 = **28,835,840 B**
(`paintbudget::CANVAS_BYTES`, paintbudget.rs:26; the task's
28.8 MB). `PaintBudget` (paintbudget.rs) enforces an entry
cap (default 160, `DEFAULT_ENTRY_CAP`) **and** a byte budget
whose derived default is clamped to **2 GiB**
(`BUDGET_CEILING_BYTES`, paintbudget.rs:36–52) → **≈74 cards**
resident (the recorded full-film run tail shows exactly this:
`entries=74 bytes=2133852160 … budget_bytes=2147483648`,
tools/perf/fixtures/vram/film-run-tail.log).

### 3.2 APU vs discrete: one pool of money or two?

On the user's PC the GPU is a Vega 7 iGPU in a Ryzen 5 PRO
5650G: **VRAM is a carve-out of the same system RAM** — stated
in-tree by gpucore's sysfs probe ("on the 5650G the VRAM
figure is a carve-out of the same system RAM", gpucore.rs,
`probe_sysfs` doc) and by paintbudget's own justification
("The target PC has ~6.7 GiB usable RAM, shared with the Vega
7 iGPU", paintbudget.rs:38–41). A GPU-resident card on that
machine does not cost *different* memory than a CPU-resident
one; it costs the **same** memory, accounted in a different
ledger (gpucore class E instead of the `paint-cache:` line —
gpucore.rs is explicit that host paint-cache bytes are *not*
ingested into the GPU ledger today, W84 §7.1).

**Design rule D2 (UMA):** on an integrated adapter
(`AdapterReport.device_type == IntegratedGpu`,
adapter.rs:454ff `describe`), the paint budget binds the
**sum**: CPU bytes (`Painted.0.len()`) + GPU bytes
(`gpucore::tex_bytes(w, h, CARD_FORMAT, 1)` — identical
28,835,840 today) across both variants, under the one existing
2 GiB ceiling. `PaintBudget`'s accounting unit generalises
from `entry_bytes(&v.0)` to a per-variant byte cost; since
both variants cost `CANVAS_BYTES`, the arithmetic of
`eviction_reason` / `on_insert` / `on_evict` is unchanged —
only a domain tag (host / device) is added so the gpu-mem
ledger (class E) and the `paint-cache:` line can each report
their share without double-counting the total.

**Design rule D3 (discrete):** on a discrete adapter the two
domains are physically separate, and the budget binds each
separately: the existing 2 GiB host ceiling for `Cpu` entries,
and a VRAM ceiling for `Gpu` entries whose sane default
**[analysis]** is also 2 GiB (≈74 cards) until PC profiling
says otherwise — the binding constraint on a discrete card is
not total VRAM but *contention* with the scene renderer's
standing set (gpucore's ledger replay puts the film lane's
standing set at ~221 MB, gpucore.rs
`ledger_replay_film_lane_acceptance_figures`), so 74 cards is
already generous headroom, and raising it buys hit-rate only
where the working set is staged (§8 shows the sampled working
set is beyond any ceiling).

In both cases the *entry cap* (160) stays as the second,
independent bound, exactly as `eviction_reason` composes them
today (cap precedence, paintbudget.rs `eviction_reason`).

### 3.3 What eviction means when repaint is GPU-cheap

Eviction mechanics for a `Gpu` entry = drop the
`wgpu::Texture` + `mem_free_texture(MemClass::E, bytes)`
(the exact pattern `DynTexturePool` already uses on slot
recreate/drop, dyntex.rs). No readback, no spill: the entry
is *recomputable*, so eviction discards it outright.

Because a GPU repaint is expected to cost far less than a CPU
repaint (the quantity Phase 0's PERF seat measures, §8.2),
**[analysis]** the optimal resident set shrinks: residency
buys avoided *repaints*, and cheaper repaints are worth less
memory. The design therefore does **not** raise any ceiling
for the GPU path; if anything the Phase 1 planner should
trial a *smaller* GPU ceiling on the APU (e.g. 24–32 cards,
the telemetry-proven working size — paintbudget.rs:44–46
notes cap 24 already audits the whole film) and let the
`rerenders` counter arbitrate. That is a tuning question,
deliberately left open here; the invariant below is not.

### 3.4 The thrash failure mode — and what guarantees it costs time, never pixels

Thrash = working set > residency → hit rate collapses,
`rerenders` (misses on previously-evicted keys,
paintbudget.rs `on_miss`) climbs, and wall time becomes
`paints × repaint cost`. On the CPU path this is a known,
bounded degradation. On the GPU path the same shape holds
**iff** one guarantee is in force:

> **Paint is a pure function of the key on BOTH paths.**
> For every key, `gpu_paint(key) == cpu_paint(key)`
> byte-for-byte (for GPU-routed classes), and each is a pure
> function of the keyed inputs alone.

Given that, residency is a pure optimisation exactly as the
existing cache header states for the CPU path — "eviction can
only cost time, never a pixel" (film.rs:2387–2397;
paintbudget.rs header: "a re-paint of an evicted key is
byte-identical to the evicted entry") — and LRU order, cap,
and budget can be tuned, bypassed (§8), or even randomised
without any pixel consequence.

**Honesty clause:** for the GPU path this purity is not a
fact today, it is a **gate obligation**. It holds *by
construction* only for classes proven byte-equal by the
parity battery on the adapter of record (§5, §7). A class
that has not passed is CPU-routed (§4), so at every moment
the shipped configuration satisfies the guarantee — the
guarantee is maintained by routing, not assumed of the GPU.
The counters to watch are already printed:
`paint-cache: … hits misses evictions rerenders …`
(PaintBudget::stats_line) plus the pool's `dyn-tex:` line;
§7 adds a `gpu-paint:` line with routed/materialised counts.

---

## 4. FALLBACK SWITCH SHAPE

### 4.1 The hard truth about per-prim fallback

A card is **one display list painted in one global order**.
The CPU painter is strictly sequential: every prim blends
onto the accumulated canvas (`Painter::blend` / `blend_cov`,
scene.rs:510ff; masked fills test masks built from earlier
prims, e.g. `fill_polygon_masked` toonfull.rs:229; the head
shade is a clip + even-odd composite, compatibility-contract
§2.3; held plates composite *into* the figure mid-list,
toonfull.rs:284). Later prims read the results of earlier
ones. Blend order is therefore not a per-prim property that
can be interleaved across devices for free.

Per-prim (per-class) fallback inside one card would require,
for each maximal run of CPU-class prims: the canvas as CPU
bytes (a 28.8 MB readback if the GPU painted the preceding
run, a 28.8 MB upload if the CPU starts), CPU painting of the
run, and re-upload — **at least one full-canvas transfer and
one device synchronisation per run boundary**, with runs
alternating as often as the list interleaves classes. For a
path whose entire purpose is removing a 28.8 MB upload per
card, a design that can pay several is self-defeating.

### 4.2 Recommendation: fallback granularity is per-CARD

**Route whole cards.** The display list is built on the CPU
(geometry side) before any painting, so the set of prim
classes a card *contains* is known at route time. Rule:

> A card is GPU-routed iff **every** prim class present in its
> display list is GO **and** enabled in the mask (§4.3) **and**
> proven on the current adapter (§5). Otherwise the whole card
> takes `paint_toon_full`, verbatim, and enters the cache as a
> `Cpu` entry.

Defence: (a) zero round-trips — each card is painted exactly
once, on exactly one device; (b) the parity obligation stays
per-card and testable (a card is the unit the battery paints);
(c) the cost is bounded and *measurable*: a card containing
one non-GO prim loses the GPU win for that card only, and a
per-class counter (`gpu-paint: cpu_route_<class>=n`, counting
the *first* non-GO class that forced each CPU route) tells
the programme exactly which class is costing the most cards,
which is the input the next GO/NO-GO verdict needs;
(d) frames still mix freely — different *cards* in one frame
take different paths, because cards composite into the scene
as independent textures (depth/alpha in the renderer's
passes), never into each other.

### 4.3 Switch shape: config surface → startup mask → executor bitmask

Three layers, each doing one job:

1. **Operator surface** — a config value resolved through the
   tree's existing precedence (CLI > `TALERENDER_*` env >
   `talerender.toml`; config.rs:190–219 documents the pattern
   and the `TALERENDER_*` naming). Proposed name:
   `TALERENDER_GPU_PAINT`, value = comma-separated class names
   (`stroke,fill,ellipse,…`), with `off` (default at first
   landing) and `all` as spellings. This follows the house
   pattern of env-gated behaviour (cf.
   `TALERENDER_PAINT_CACHE`, `TALERENDER_PAINT_BUDGET_MB`,
   film.rs:2405ff).
2. **Startup resolution** — parsed **once**, at `GpuPaintCtx`
   construction, into a `u32` bitmask over a `PrimClass` enum
   (one bit per Phase 0 verdict class), intersected with the
   per-adapter proven set (§5.3): an adapter with no proof for
   class X has bit X cleared regardless of the operator value,
   and the clearing is **printed** (renderer-line discipline:
   identity and routing are always printed, compatibility-
   contract §6.1). An operator override to force unproven
   classes, if Phase 1 wants one, must be a separate explicit
   spelling that prints a warning — never a silent widening
   (§6.4 of the contract: no quality-affecting configuration
   silently).
3. **Executor check** — per card, at route time in
   `get_or_paint` (and at the scenefile plug, §8.1): the
   display list carries a `classes_present: u32` bitset,
   accumulated during list construction at zero marginal cost;
   GPU route iff `classes_present & !mask == 0`.

**Not** compile-time: a `cfg` switch would make the fallback
unavailable in the shipped binary precisely when a defect is
attributed in the field, and would fork the CI matrix.
**Not** per-prim in the executor: §4.1. The bitmask lives in
the executor; the granularity it *acts* on is the card.

---

## 5. DETERMINISM

### 5.1 The law as it stands

- compatibility-contract §4: `frame(t)` is a pure function;
  repeated and out-of-order evaluation yields pixel-identical
  frames **within one implementation**.
- DETERMINISM-CONTRACT-W04 §1: the guarantee is "Same scene
  data + same time inputs + same binary + **same renderer** ⇒
  byte-identical frames", where *same renderer* = the adapter
  + driver the binary prints. §2 is explicit: **no
  cross-vendor / cross-driver bit-identity is promised** —
  fp32 `sin()` in the grade grain and floating-point rounding
  across shader JITs are the named mechanisms; cross-vendor
  comparisons live in PSNR bands with per-vendor goldens.
- The determinism pack (W04 §4) renders frames [100,130) as
  single / chunked-resumed / cache-cap-24 runs and
  sha256-compares; a PASS is **per renderer of record** —
  CI lavapipe and PC RADV are the two, and one never stands in
  for the other.

GPU paint does not amend any of this. It adds a new
*component* whose output must satisfy the same law.

### 5.2 What the design must prove, per adapter

For each adapter of record (CI lavapipe; the user's PC RADV),
before any class is GO **on that adapter**:

1. **Self-determinism:** repeated GPU paints of the same key
   — including out-of-order, across cache-cap variation, and
   across chunk boundaries — are byte-identical. (The pack's
   A/B/C structure, applied to paint: the existing pack
   already exercises cache-cap variation end-to-end, W04 §3
   mechanism 2.)
2. **Parity:** GPU paint of a key is byte-identical to CPU
   paint of the same key, for every state in the parity
   battery (production poses/moods at res 2 — the
   compatibility-contract §2.2 stroke battery surface, extended
   to every GO class and to anchored states, §2.3).
   This is stronger than the contract's cross-implementation
   rule (§4.2 judges Chrome-vs-native by parity tests, not
   byte equality) — deliberately: the *cache* requires it,
   because the same key must return the same card whichever
   path a routing change, a mask change, or an adapter change
   sends it to (§3.4's purity guarantee).
3. **Arithmetic discipline:** where the CPU painter's result
   is integer-exact (u8 blending with f32 coverage,
   `Painter::blend_cov`), the GPU implementation must
   reproduce the rounding exactly — integer/fixed-point WGSL
   where needed. Any class whose GPU form depends on
   adapter-variable transcendental or fp16 behaviour for its
   *pixels* (not merely its geometry inputs, which stay CPU —
   §2.3) is NO-GO until that dependence is removed. This is
   the concrete form the parity law takes for paint.

### 5.3 Where adapter identity enters

Paint must execute on **the renderer's own device** — the
`Renderer` creates the only device (film.rs:2733–2746) and
`GpuPaintCtx` is built from its device/queue handles. A
second device (hence possibly a second adapter) for paint is
forbidden by design: it would make a frame's pixels a
function of two adapters and void the renderer-of-record
discipline.

Adapter identity is already first-class in-tree:
`adapter::describe(&AdapterInfo) -> AdapterReport`
(adapter.rs:454), `adapter::report_line` (adapter.rs:476),
stored on the `Renderer` (`adapter_report`,
film.rs:2721, 2739) and printed at startup (film.rs:2744).
The design keys the **proven set** (§4.3 layer 2) off that
report — `(name, backend, device_type, driver)` — recorded
per landing in the integration note, exactly as W04 §6
records a determinism PASS (binary sha256 + renderer line +
run id). A driver update is a new renderer: proofs re-run
(W04 §8 maintenance rule).

*Terminology caution:* the skill-side **era-pairing**
machinery (`era_hint`, and in-tree the fail-closed
`legacy_anchor_guard`, film.rs:169) pairs *binary era ↔ data
era*. It is not an adapter mechanism and must not be cited as
one. The adapter-pairing discipline this design relies on is
the renderer-of-record system above (adapter report +
determinism pack records), which is the machinery that
actually exists for adapters.

### 5.4 The census question — options and recommendation

The Onoda census is a CI-lavapipe, full-film (1,811-frame)
byte comparison between two binaries (the protocol used for
the sweat/tears fix, V2-HANDOVER.md). If the user's PC paints
on RADV and the GPU paint differs from lavapipe's at **any**
pixel, a GPU-painted census breaks — not as a defect in the
film, but as a renderer change inside a byte gate.

Options:

- **(a) Census stays on the CPU paint path.** The census
  binary runs with the GPU mask resolved to `off`. The census
  keeps its exact current meaning (it gates the *film
  pipeline*, and paint-path changes are gated by §5.2's
  battery + pack instead). Cost: the census no longer
  exercises the production-default path if GPU paint later
  defaults on — mitigated by (d).
- **(b) Require cross-adapter byte-stability of GPU paint.**
  Strongest and simplest to reason about — and **[analysis]**
  almost certainly unattainable for any class touching
  fp32 transcendentals or vendor rounding, i.e. it would
  silently convert the parity law into a permanent NO-GO for
  most classes. Rejected as a *requirement*; welcome as a
  per-class *finding* where it happens to hold.
- **(c) Re-baseline the census per adapter.** Multiplies gate
  surfaces, destroys the census's comparability across time,
  and the RADV census cannot run in CI at all (compatibility-
  contract §6.2: CI is lavapipe; RADV evidence is PC-only).
  Rejected.
- **(d) [analysis] Companion evidence run:** a GPU-on census
  *variant* on lavapipe (same protocol, mask on) run per
  landing as evidence — if §5.2 parity holds, it is
  byte-identical by transitivity, and any diff is
  defect-attributed to a named frame/state. It informs; it
  does not replace (a) as the gate of record.

**Recommendation: (a) + (d).** The census gate stays exactly
what it is today; GPU paint earns its place through the
per-adapter battery, the determinism pack on both renderers
of record, and the variant run as landing evidence.

---

## 6. DEPS

**Zero new dependencies. No exception.**

`Cargo.toml` today: `wgpu = 30.0.2`, `pollster = 1.0.1`,
`image = 0.25.10` — and V2-HANDOVER.md records "Zero new
dependencies in all of V2" as a maintained invariant. The
design uses only:

- `wgpu` — render passes, textures, buffers, readback: the
  same API surface `gpucore`, `dyntex` and the film renderer
  already use; WGSL shaders are in-tree string modules (the
  `shader_*.wgsl` pattern), compiled by wgpu itself — no
  shader toolchain dependency;
- `pollster` — not even needed on the paint path (the device
  already exists; materialisation is synchronous queue work);
- `image` — only in test/diagnostic code that already uses it
  (PNG comparison surfaces).

Anything that would need a new crate (a GPU vector-graphics
library, a compute framework) is by definition a different
programme and must be flagged as such, not smuggled into
Phase 1.

---

## 7. MIGRATION PHASING (sketch for the Phase 1 planner)

Landing order is per GO class, in the Phase 0 verdicts'
order (simplest/most-contained class first — the planner
refines). Every landing carries the **same gate set**; a
landing that cannot produce a gate's evidence has not landed.

| Step | Content | Gates |
|---|---|---|
| **P0 — plumbing** | `PaintedCard` sum type, `GpuPaintCtx`, mask resolution + printed routing line, `classes_present` bitset, deferred-materialisation seam in `render()`. Mask default `off`; **no class routed**. | CI green; telemetry CHECK OK; census byte-identical (trivially — nothing routes); golden 17 OK; byte probes 40/40 vs baseline (the V2 protocol, V2-HANDOVER.md); determinism pack PASS (lavapipe). Pixel-neutral by construction, proven, not asserted. |
| **P1..Pn — one GO class each** | Executor support for class X; bit X available in the mask, default **off**, then flipped default-on in a *separate* commit once gates pass with it on. | All P0 gates, **plus**: parity battery for X byte-identical (CPU vs GPU, both adapters of record); determinism pack PASS on lavapipe **and** the RADV PC leg (W04 §7 procedure) with X on; GPU-on census variant (d) byte-identical or every differing frame defect-attributed to a named state; telemetry CHECK OK (unchanged path, D1); `gpu-paint:` stats line shows routed/materialised counts and per-class CPU-route counters (§4.2). |
| **Final — default-on** | Mask default becomes the proven set (operator `off` always available). | Full gate set at the final head; PC RADV throughput leg (§8.2) recorded in PERF, not CI. |

Rollback at every step is the mask: `TALERENDER_GPU_PAINT=off`
(or removing one class) restores the exact CPU behaviour with
no data migration, because the key, the cache and the card
format never changed (§2.1, §1.5).

---

## 8. THE SAMPLED PROFILE (first-class constraint)

*Coordinator update, verified at `bc12a5c` this session:*
native projects are authored with **per-frame sampled
character poses** in scene JSON. The scene-sequence driver
clones the parsed root per frame and sets `time`
(filmseq.rs:20–23 module doc; `set_root_time` filmseq.rs:422;
clone+set at filmseq.rs:577); staged state is otherwise
static; and the loader **rejects clip animation outright** —
`scenefile.rs walk_nodes`: "clips extension not supported"
(scenefile.rs:574). Acting is therefore baked as per-frame
pose samples. This changes the cache's economics, the
throughput requirement, and the size of the GPU win. Each is
worked through below.

### 8.1 Cache under the sampled profile

**First, a structural fact:** the sampled path has **no cache
to degenerate**. filmseq renders through
`scenefile::render_loaded_pixels`, and scenefile paints at
scenefile.rs:752 — direct, uncached, every character every
frame. The `PaintCache` exists only on the legacy driver
(film.rs). So "the cache degenerates under sampling" is, for
the path that actually samples, already the status quo:
hit rate is exactly 0 because there is no cache. The
degeneration analysis matters for (i) any proposal to *add*
the legacy cache to the sampled path, and (ii) legacy-driver
films authored with sampled-style dense state changes.

**The staged hit rates, for calibration** (recorded
`paint-cache:` lines): the full Onoda film run —
hits=1204, misses=611 (≈**66%** hit; tools/perf/fixtures/
vram/film-run-tail.log); a telemetry run — hits=812,
misses=97 (PAINT-BUDGET-W06.md); a short leg — hits=96,
misses=386 (≈20%; CR-B2-04 / PC-DRYRUN-W68). Hit rate is a
property of the *authoring profile*, varying 20–89% across
recorded staged runs — the coordinator's "40–70%" is the
full-film census band.

**Sampled arithmetic [analysis, from the cited mechanics]:**
a 1,811-frame film with 2 sampled characters issues
2 × 1811 = **3,622 lookups**, of which the misses are
≈3,622 minus only those frames where a sampled pose repeats
*bit-exactly* (a genuinely held pose in the source data —
exact keys, film.rs:2455ff, mean interpolated/continuous
samples essentially never repeat). Unique states: **thousands,
~10× the staged film's 611**. Residency needed to hold them:
3,622 × 28.8 MB ≈ **104 GB**, against a feasible 74 cards
(§3.1) — the cache would hold ~2% of the working set. LRU
under a sequential unique stream is the worst case: every
insert evicts (evictions ≈ inserts), `rerenders` stays ~0
(nothing is ever re-requested), and the cache's entire yield
is the rare held-pose hit.

**Cost paid for that ~0 yield, per lookup:** `state_key`
builds a formatted `String` of 23 components (film.rs:2496–
2505) — 17 `u32` decimal conversions of f32 bit patterns, a
`Mood` debug format, a legs string, two hold names — a
~200-byte key (paintbudget.rs:20–23 prices "the ~200-byte key
string" as negligible *next to one canvas*, which is true for
memory and irrelevant for per-lookup CPU). Add: one `String`
allocation, one `HashMap` hash+probe over ~200 bytes, and on
the *hit* path a **linear** LRU touch —
`order.iter().position(|k| k == &key)` over up to `cap`
entries with string compares (film.rs:2432–2435), plus
`VecDeque::remove/insert`; on the miss path, insertion plus
the eviction loop, and on the CPU path the drop of a
28.8 MB `Vec` per eviction. **[Analysis — bound, not
measurement:]** formatting+hashing is of order ~1 µs per
lookup; even at 60 lookups/s that is ~0.06 ms/s — *the key
cost itself is not the problem and this design does not
pretend otherwise.* The real costs of caching a sampled
stream are (a) 28.8 MB allocation/drop churn per insert on
the CPU path, (b) eviction bookkeeping per character-frame,
and (c) **pollution**: sampled inserts evict the staged
working set (the 74 slots are shared), so a mixed film —
sampled foreground, staged background cast — can *lose* the
staged hit rate it would otherwise have had. (c) is the
decisive argument; (a)–(b) are noise by comparison.

**Recommendation — dual mode, selected by the driver, never
by heuristic:**

- **Staged mode (legacy driver):** the cache exactly as
  today (§§1–3). Its hit rates justify it.
- **Streaming / write-through mode (scene-sequence driver,
  and any future sampled consumer):** **no insertion.** Paint
  → bind for this frame's draws → release. On the CPU path
  this is scenefile's status quo (its `textures` vec is
  per-render by construction). On the GPU path it is a
  **per-slot ring**: one paint-target texture per `TexSrc::Dyn`
  slot, repainted in place each frame — safe with a ring of
  **one** because all submissions ride a single queue and
  execute in order (the paint pass for frame n+1 is submitted
  after frame n's draws that sampled the same texture), with
  a ring of two as the planner's option if Phase 1 pipelines
  paint against draw. Residency for the sampled film above:
  2 slots × 28.8 MB ≈ **58 MB**, versus 2 GiB of cache that
  yields ~0 hits. Write-through entries never enter
  `PaintBudget`, never compute a `state_key` (there is nothing
  to look up — pose identity is frame-scoped), and never
  evict anything.
- Mode is a property of the **driver/profile**, recorded in
  `GpuPaintCtx` / the scenefile plug — *not* an adaptive
  hit-rate heuristic. A heuristic would make residency (hence
  timing) depend on run history for zero pixel benefit and
  would complicate every gate in §7; the profile is known
  statically at the point of authorship. (For the legacy
  cache, `TALERENDER_PAINT_CACHE=0` already approximates
  write-through — cap 0 evicts everything, paintbudget.rs
  `zero_entry_cap_preserves_legacy_evict_all` — but still
  pays key+insert+evict per lookup; it is an operator knob,
  not the sampled design.)

**Open item for the Phase 1 planner (flagged, not designed
away):** filmseq calls `render_loaded_pixels` per frame
(filmseq.rs:577ff), so any GPU paint session for the sampled
path must own its device/textures *across* frames at the
driver level, not inside a per-frame render call, or the
ring's persistence — and the device itself — is rebuilt per
frame. The plug for scenefile (§1.3) and the session lifetime
must be designed together.

### 8.2 Throughput, not just latency

Under sampling the binding constraint is **sustained
paints/second**, because nearly every character-frame is a
paint (§8.1). Parametric requirement — let:

- `C` = sampled characters on screen, `F` = 30 fps,
- `m` = GPU paint+present cost per card, ms/state (the PERF
  seat's measured quantity),
- `R` = the rest of the frame (scene render, compositing,
  readback of the final frame), ms.

Sustained paint demand is `C·F` paints/s (60/s at C=2).
Real-time-equivalent throughput — the point where paint stops
being the term that sets wall time — requires:

```
C·m + R ≤ 1000/F = 33.3 ms        ⇔        m ≤ (33.3 − R)/C
```

At C=2: **m ≤ 16.7 ms** even with R=0; with a realistic R the
allowance shrinks linearly (R=10 ms ⇒ m ≤ 11.7 ms). Offline
rendering is not hard-bound by 33.3 ms, but wall time is
exactly `frames × (C·m + R)`, so m is *the* lever:

| m (ms/state) | paints/s (serial) | paint wall, 3,622 states |
|---|---|---|
| 5 | 200 | 18 s |
| 10 | 100 | 36 s |
| 16.7 | 60 | 61 s |
| 33 | 30 | 120 s |

**Evidence discipline:** `m` measured on **lavapipe** (the
PERF seat's harness, CI) is a *labelled* number — per
compatibility-contract §6.2 lavapipe proves correctness, and
"performance, GPU-feature and timing claims are admissible
only when measured on the target PC" (RADV, Vega 7). The
requirement above must therefore be carried as **two** gates:
the lavapipe figure as a regression floor in CI, and the
**RADV leg on the user's PC** (same harness, W04 §7-style
record: date, binary sha256, renderer line) as the only
admissible throughput claim. Any Phase 1 report that quotes a
lavapipe ms/state without its label violates §6.2.

### 8.3 The GPU-win claim — verified, with the counter-considerations

Claim under test: *sampling enlarges the GPU win.* Worked
against the cache math:

- **Staged (Onoda census profile):** GPU paint can only win
  on **misses** — hits already cost no paint. Saving per film
  = `misses × (P_cpu − P_gpu)` = **611 × Δ**, where Δ is the
  per-paint CPU→GPU saving. The other 1,204 lookups were
  already free.
- **Sampled (same length, C=2):** saving = **3,622 × Δ** —
  a **5.9×** larger saving opportunity, because ~every
  character-frame is a miss. The claim **holds**, arithmetically,
  *for the paint term*.

But the same arithmetic cuts the other way, twice:

1. **The CPU fallback gets absolutely slower.** Sampled CPU
   paint per film = 3,622 × P_cpu vs the staged 611 × P_cpu —
   the same 5.9×. GPU paint under sampling is not an
   optimisation layered on a viable CPU path; it is
   **load-bearing**. A sampled film containing a NO-GO class
   pays full CPU paint for every card containing it, every
   frame (§4.2's per-card routing makes this exact). Phase 1
   prioritisation should therefore rank classes by their
   frequency *in sampled content*, not in the staged film.
2. **Per-paint fixed costs can dominate for small figures.**
   A GPU paint pays fixed costs the CPU painter does not:
   display-list/geometry buffer construction and upload per
   card, bind-group setup, a render pass, and — unless the
   executor clips to the figure's bbox — rasterisation over
   the full 2560×2816 canvas (7.2 M texels) regardless of how
   small the character is on it. The CPU painter's loops are
   bbox-bounded per prim (`Painter::bbox`, used throughout
   toonfull.rs), so a small figure is *cheap* on CPU and
   *fixed-price* on GPU. Break-even **[analysis]**:
   GPU wins a card iff `P_cpu(bbox) > F_gpu + R_gpu(canvas)`,
   where `F_gpu` is the fixed dispatch+upload term. At
   60 paints/s, if `F_gpu` alone were, say, 2 ms, fixed costs
   consume 120 ms of every second before a single texel is
   shaded — `F_gpu` is therefore a **required PERF-seat
   measurement** (empty/tiny-card paint), not a detail. The
   known mitigation — paint into a bbox-cropped target with
   the crop offset folded into the card UVs — is recorded
   here as a Phase 2 candidate; it changes texture size per
   state (hence key-adjacent bookkeeping) and is **not**
   assumed anywhere in this design.

**Net conclusion:** the sampled profile enlarges the GPU win
~5.9× on the paint term *and* makes the streaming
(write-through, §8.1) form of the design — not the cached
form — the one that captures it: cached residency spends
memory and bookkeeping to avoid repaints that sampling has
already made unavoidable, while streaming spends a fixed
58 MB of ring textures and converts every character-frame
into the throughput question of §8.2, where the GPU's
advantage actually lives. The win is conditional on
`F_gpu + R_gpu < P_cpu` at production figure sizes, which is
a measurement Phase 0's PERF seat owes this design — asserted
nowhere in this document.

---

## Appendix — source map for the Phase 1 planner

| Topic | Source |
|---|---|
| Plug point | film.rs:2372 (`Painted`), 2421–2451 (`get_or_paint`), 2441 (call) |
| Key | film.rs:2453–2505 (`state_key` + coverage-law comment) |
| Downstream | film.rs:2510–2635 (`resolve_chars`), 3164–3186 (`render` pool drive), 3421 (Dyn binding), 4175 (telemetry overflow) |
| Dyn pool | dyntex.rs (`DynTexturePool`, `CARD_FORMAT`, class-E ledger) |
| Budget | paintbudget.rs:26 (canvas), 36–52 (2 GiB ceiling), `eviction_reason`, `stats_line` |
| Anchor | toonfull.rs:365–402 (doc), 403 (`canvas_anchor`), 714 (application), 1687 (tests); film.rs:2571–2589 (compensation) |
| Hands | toonfull.rs:950–972, 1683 (return) |
| Other painter callers | scenefile.rs:752, filmtelemetry.rs:673 |
| Device/adapter | gpucore.rs (`request_adapter_with_policy`, `Readback`, ledger, sysfs UMA note); adapter.rs:454–483 (`describe`/`report_line`); film.rs:2721–2746 (Renderer) |
| Determinism | docs/compatibility-contract.md §§4, 6; docs/integration/DETERMINISM-CONTRACT-W04.md §§1–2, 4, 6–8 |
| Sweat/tears precedent | docs/integration/V2-HANDOVER.md (defect + RESOLVED); docs/integration/STATE-KEY-FIX.md |
| Sampled profile | filmseq.rs:20–23, 422 (`set_root_time`), 577 (clone+set); scenefile.rs:574 (clips rejected) |
| Staged hit rates | tools/perf/fixtures/vram/film-run-tail.log (1204/611); docs/integration/PAINT-BUDGET-W06.md (812/97) |
| Deps | Cargo.toml (wgpu 30.0.2, pollster 1.0.1, image 0.25.10) |
