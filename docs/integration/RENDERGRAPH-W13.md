# RENDERGRAPH-W13 — render graph declaration + validation layer

**Seat:** W13, Wave 2 Phase 2 ("build a unified render graph").
**Delivered:** `src/rendergraph.rs` ONLY (new file) + this note.
No existing file was edited; the lead registers the module and
migrates callers (path below). Nothing committed by this seat.

## What it is

A small typed graph over the renderers' real passes: passes declare
named resource reads/writes; resources are declared with a kind
(texture / host data), a format (`Rgba16Float`, `Rgba8Unorm`,
`Depth32Float`, `PackedDepth`) and a size class (`FullFrame`,
`Fixed(w,h)`, `BloomMip(n)`, `Native`). `GraphBuilder::build()`
validates the declaration and derives:

- a **deterministic execution order** (Kahn; ties broken by
  declaration order — documented in the module, pinned by a test),
- a **lifetime table** (first-use / last-use position per resource)
  — the interval input a future transient allocator consumes,
- **errors, all collected in one report**: unknown resource,
  duplicate declarations, write to an external input, read of a
  never-written resource, write–write conflict *without ordering*,
  ambiguous read, dependency cycle,
- **warnings**: unused writes / unused externals.

The layer is **pure data + algorithms — no wgpu types** — which is
what makes it testable with a bare `rustc --test` on this machine.

### Semantics worth knowing before migrating

- **Version binding.** A multi-writer resource holds several values
  per frame. Each read binds to the nearest writer declared before
  it (source order for transcribed graphs), else the earliest
  writer after it (forward reference). Edges are bound writer →
  reader. This matters: the naive all-writers→all-readers edge set
  *false-cycles* on the real chain (bloom_add writes `bokeh_out`,
  bloom_bright reads it, so naive edges run add→bright→…→add). The
  first fixture run caught exactly that; binding is the fix, and it
  is the semantics the tests pin.
- **RMW is first-class.** A pass listing a resource in both reads
  and writes binds its read to the *other* writers (bloom_add onto
  `bokeh_out`, caption onto `final`); a lone RMW is a
  read-of-never-written error.
- **Double writes are legal iff data flow orders the writers.**
  The bloom composite really does write into the cascade's
  horiz[0] texture (composer.rs: "Composite into horiz[0]"); the
  cascade chain orders it, so the fixture validates. Two writers
  with no path between them are a `WriteConflict`.

## The catalogue (the fixture: `ab1_full_chain()`)

Transcribed from `ab1.rs::run_ab1` at post stage 3 (production)
with the cascade expanded from `composer.rs::PostChain::bloom`.
Resource names are the source texture labels, snakified.

| # | Pass (source) | Reads | Writes (format, size) |
|---|---|---|---|
| 1 | `shadow` (ab1 "shadow pass") | scene_data† | shadow_map (Rgba8, 2048²), shadow_depth (Depth32, 2048²) |
| 2 | `depth_prepass` (ab1 step 2) | scene_data† | packed_depth (PackedDepth, frame), prepass_depth (Depth32, frame) |
| 3 | `scene` (ab1 "ab1 scene pass") | scene_data†, scene_textures†, room_env†, shadow_map | hdr_msaa (F16, frame, MSAA-4x), hdr_resolved (F16, frame), hdr_depth (Depth32, frame) |
| 4 | `bokeh` (ab1 step 4) | hdr_resolved, packed_depth | bokeh_out (F16, frame) |
| 5 | `bloom_bright` | bokeh_out | bloom_bright (F16, mip 0) |
| 6–15 | `bloom_blur_hN`/`vN` ×5 | cascade (H0←bright, V←H, H←prev V) | bloom_hN / bloom_vN (F16, mip N) |
| 16 | `bloom_composite` | bloom_v0..v4 | bloom_h0 (reused horiz[0]) |
| 17 | `bloom_add` (Load) | bloom_h0, bokeh_out | bokeh_out (RMW) |
| 18 | `output` (OutputPass) | bokeh_out | output_srgb (Rgba8, frame) |
| 19 | `grade` (GRADE) | output_srgb | final (Rgba8, frame) |
| 20 | `caption` (ab1 step 8, Load) | caption_strip†, final | final (RMW) |
| 21 | `readback` | final | png_bytes‡ |

† declared external input · ‡ declared sink.
Stage variants: at post stage 2, `output` writes `final` and pass
19 is absent; at stage 1, passes 5–19 are absent. `PackedDepth` is
its own format class because the prepass target is Rgba8Unorm by
default and Rgba16Float under `TALERENDER_DOF_DEPTH_FORMAT=halffloat`.

**Fixture verdicts (asserted in tests):** derived order == source
encoder order, exactly; the only `UnusedWrite` warnings are
`shadow_depth`, `prepass_depth`, `hdr_msaa`, `hdr_depth` — the
attachments consumed *inside* their own pass (depth test / MSAA
resolve), which a cross-pass graph correctly cannot see.
Lifetimes: e.g. `bokeh_out` lives 3→17 (written by bokeh, RMW'd by
bloom_add at 16, last read by output), `bloom_h0` 5→16 across both
its writers, `final` 18→20.

## Tests (standalone, this machine)

```
rustc --edition 2021 --test src/rendergraph.rs -o /tmp/rendergraph_test
/tmp/rendergraph_test   →  8 passed; 0 failed
```

1. Real chain validates; order == source order; key edges present;
   warning set == the four in-pass attachments.
2. Lifetime table correct on the fixture (positions above).
3. Planted 2-pass cycle → `Cycle { [a, b] }`.
4. Planted unordered double-write → `WriteConflict`.
5. Ordered double-write (the composite pattern, in miniature) is
   legal and schedules in data order.
6. Unknown resource / never-written / write-to-external all
   reported in one build.
7. Topo reorders a consumer declared before its producer;
   declaration order breaks ties between independent passes.
8. Lone RMW → `ReadNeverWritten`.

## Migration path for the lead

1. **Register** (one line, lead-owned file): add
   `mod rendergraph;` to `src/main.rs`.
2. **Declare + validate, change nothing else.** Beside each
   renderer's existing recording code, build its graph and
   `debug_assert!` / log the build result. ab1 uses
   `ab1_full_chain()` verbatim; film.rs's per-frame chain is the
   same declaration with per-shot externals; scenefile's composer
   path is the dof.rs variant of the same shape. The graph becomes
   a checked specification of the order the code already runs —
   any drift between the two fails loudly at startup.
3. **Order from the graph.** Replace the hand-sequenced encoder
   blocks with a loop over `graph.order()` dispatching to per-pass
   closures. For every transcribed graph the derived order equals
   today's order (the fixture pins this property for ab1), so this
   step is byte-identical by construction.
4. **Allocate from lifetimes (later seat).** Feed
   `graph.lifetimes()` to a transient pool: non-overlapping
   intervals share one physical texture. First wins on this chain:
   the bloom mip set, `packed_depth` vs `output_srgb`, and the
   three depth attachments (all dead by pass 4).

## What the graph does NOT do (yet)

- **No transient allocation.** Lifetimes are computed and exposed;
  nothing consumes them. Deliberate: allocation changes memory
  behaviour and wants its own gate (byte-identical frames out).
- **No barrier insertion.** Acceptable because every renderer here
  records one frame into ONE `CommandEncoder` and submits once
  (ab1.rs, film.rs): within a single submission wgpu inserts the
  transitions/barriers between render passes automatically, and
  the graph's order is the recording order. The graph models one
  frame; resources do not carry across frames inside it (the film
  paint cache lives outside the frame encoder and is W06/W07
  territory, not this layer's). If a future design splits a frame
  across submissions or queues, barriers become a graph concern —
  the edge set here is exactly where they would be derived from.
- **No execution.** Pass bodies stay in ab1.rs / composer.rs /
  film.rs; the graph orders and validates, it does not draw.

## The direct (non-composer) path in graph terms

scenefile.rs's direct mode (no `post` block) is a degenerate
two-node graph — one pass into an sRGB target, then readback:

```rust
let mut b = GraphBuilder::new();
b.add_resource(ResourceDesc::host_input("scene_data"));
b.add_resource(ResourceDesc::external_texture(
    "scene_textures", TexFormat::Rgba8Unorm, SizeClass::Native));
b.add_resource(ResourceDesc::texture(
    "final", TexFormat::Rgba8Unorm, SizeClass::FullFrame));
b.add_resource(ResourceDesc::host_sink("png_bytes"));
b.add_pass(PassDesc::new("scene_direct")
    .reads(&["scene_data", "scene_textures"]).writes(&["final"]));
b.add_pass(PassDesc::new("readback")
    .reads(&["final"]).writes(&["png_bytes"]));
```

Same shape serves main.rs's built-in stills. The value is that
direct and composer paths become two declarations in one system,
diffable and gated by the same validators.

## Granularity note

Per-draw scene textures (painted character canvas, ground/cloud
sprites) are aggregated as the single external `scene_textures`:
cross-pass structure doesn't change if they are split (all are
scene-pass inputs), and the split belongs to whichever seat models
the paint cache. Room env and the caption strip are separate
externals because their provenance differs (captured asset /
per-project upload).
