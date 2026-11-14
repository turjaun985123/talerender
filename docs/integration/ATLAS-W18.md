# Texture atlas module — W18 design note

**Seat:** Wave 2 W18 (Phase 3 — texture atlasing). **Delivered:**
`src/atlas.rs` (new file, uncommitted — the lead wires it).
**Nothing else touched.** Verified standalone: `rustc --edition
2021 --test src/atlas.rs` → **11 passed, 0 failed, 0 warnings**
(the module has no wgpu dependency, so the standalone build is
the whole verification surface for the packer itself; the GPU
wiring below is the lead's follow-up and carries its own proof
list, §7).

## 1. What exists today (surveyed, not assumed)

The character card's path through film.rs, as built:

- `PaintCache::get_or_paint` (film.rs) returns `Rc<Painted>` —
  the CPU canvas, 2560×2816 RGBA = **28,835,840 B** — keyed by
  the exact f32-bit-pattern `state_key`. W06's `paintbudget.rs`
  (this wave) governs this CPU side.
- `resolve_chars` returns the frame's canvases as
  `Vec<Rc<Painted>>` (`dyn_tex`, `TexSrc::Dyn(slot)` order) and
  builds each card's `BasicDraw` with `quad_geo` geometry.
- `Renderer::render` then, **per frame**, loops over `dyn_tex`
  calling `Renderer::upload_dyn(&p.0, p.1, p.2)`: a brand-new
  `Rgba8UnormSrgb` texture (`mip_level_count: 1`,
  `TEXTURE_BINDING | COPY_DST`), a full `queue.write_texture` of
  28.8 MB, a view — and per card, two bind groups (the basic
  colour draw + the card-shadow draw). All destroyed after the
  frame. A canvas the paint cache proved identical across 200
  frames is still re-uploaded 200 times.

Sampling, surveyed in the WGSL + sampler construction:

- Colour: `shader_basic.wgsl` — `textureSample(tex, smp, in.uv)`,
  `in.uv` interpolated from the card's vertex UVs
  (scene.rs `quad_geo`: (0,1),(1,1),(1,0),(0,0) — **v = 0 at the
  card's top edge**, matching texture row 0). Cards bind
  `sampler_clamp` (`repeat: false` in `resolve_chars`).
- Shadow: `shader_shadow_card.wgsl` — same UV, same sampler,
  alpha ≥ 0.5 gate. The card's depth prepass binds no texture.
  These are the **only two sampling sites** for a card.
- Samplers (film.rs `Renderer::new`): `sampler_clamp` =
  Linear mag / Linear min, everything else `..Default::default()`
  → ClampToEdge addressing, mipmap filter Nearest, anisotropy 1.
- **One mip level exists** (`mip_level_count: 1` in
  `upload_dyn`), so the mipmap filter can never engage.

Texture-size limit: every production device request
(film.rs, ab1.rs, scenefile.rs, dof.rs) uses
`wgpu::Limits::default()` — standard WebGPU limits,
`max_texture_dimension_2d = 8192` (ab1.rs's comment states it:
downlevel's 2048 cap cannot hold one canvas). The packer
therefore validates page dims to ≤ 8192 and defaults to it.

## 2. The module

`pack(entries, cfg) -> AtlasPack` — **MaxRects, Best-Short-Side-Fit**.

- **Why MaxRects over shelf:** shelf packing wastes the strip
  above every shorter entry in a row; with heterogeneous entries
  (plates, caption strips, any future smaller canvas) that waste
  compounds, and MaxRects reuses L-shaped leftovers via its
  free-rect splits. On the production census the choice is free:
  uniform 2564×2820 cells (canvas + gutter) tile an 8192 page
  into the same clean 3×2 grid either way (asserted in tests:
  inner origins (2,2), (2566,2), (5130,2), (2,2822), …).
- **Determinism:** entries are sorted by a total order — area
  desc, w desc, h desc, id asc — before placement; free-rect
  selection scans a `Vec` with a total tie-break key
  (short leftover, long leftover, y, x); pages are tried in
  index order. No HashMap is iterated (duplicate detection uses
  a `BTreeSet` for membership only). Test
  `determinism_input_order_invariant` packs the census forward,
  reversed, and rotated and requires byte-equal placement lists.
- **Every entry is placed or explicitly overflowed** —
  `TooLarge` (cell > page), `PageLimit` (`max_pages` full),
  `DuplicateId`, `ZeroSize`, `BadConfig` (page dim 0 or > 8192).
  Nothing is dropped silently.
- **UV helper:** `Placement::uv_rect(page_w, page_h) -> UvRect`
  and `UvRect::remap([u, v])` convert a placement into the exact
  form the shaders consume: the card geometry's [0,1]² vertex
  UVs remapped per-vertex, shaders untouched. `cell_uv_rect`
  (gutter included) exists for the uploader/debug, not geometry.
- **Statistics:** `stats_line()` in paintbudget's house style —
  pages, placed, overflow, texel counts, inner/cell utilisation,
  total bytes, `bytes_per_placed()`.

## 3. Wiring sketch (film.rs — lead's edit)

**main.rs:** add `mod atlas;` after `mod gpucore;` (wherever the
lead prefers; the module needs no other crate item).

**Plan from telemetry, render from plan.** `pack()` is a batch
API and film states are not known ahead of a streaming render —
but the engine already evaluates the whole film CPU-only in
`run_film_telemetry` (which walks every sampled frame through
`resolve_chars` + the paint cache). That pass *is* the census:
record the distinct `state_key`s it paints, `pack()` them once,
and the film render consumes fixed placements. A direct film
render without a planning pass keeps unplanned states on the
existing Dyn path (mixed mode is safe — the per-card bind group
already selects a view per draw).

**GPU side (new code in film.rs, ~120 lines):**

- `AtlasSet { pages: Vec<wgpu::Texture>, views: Vec<TextureView>,
  slots: HashMap<String, Placement> }`, pages created like
  `upload_dyn`'s texture (`Rgba8UnormSrgb`, `mip_level_count: 1`,
  `TEXTURE_BINDING | COPY_DST`) at the packed page size.
- Upload per distinct state, once: build the **cell buffer** on
  CPU — inner canvas with the P-texel ring filled by edge
  replication (rows/columns clamp-stretched, corners = corner
  texel), (2564×2820×4 ≈ 28.9 MB transient) — and a single
  `queue.write_texture` at `Origin3d { x: cell.x, y: cell.y }`.
  One upload call, no strip bookkeeping.
- `resolve_chars`: after `cache.get_or_paint`, look the key up
  in the plan. On a hit, remap the card geometry's four UVs
  through `placement.uv_rect(PAGE, PAGE).remap(uv)` as the
  `TexGeo` is built (verts carry `[pos, uv]` pairs; only the uv
  components change), and give the draw a tex source naming
  (page, placement) instead of `TexSrc::Dyn(slot)`. On a miss,
  today's Dyn path, unchanged.
- `Renderer::render`: the atlased card's basic bind group binds
  the **page view** + `sampler_clamp`; the card-shadow bind
  group likewise. The per-frame `upload_dyn` loop shrinks to
  only the unplanned Dyn stragglers.

## 4. Pixel-neutrality argument (the hard law)

Claim: for any fragment, the atlased fetch returns the same
value as the unatlased fetch. The fetch model, from §1:

1. **Bilinear, level 0, clamped.** Mag/min filters are Linear;
   only one mip level exists, so `textureSample`'s
   derivative-chosen level is 0 and every fetch is a 2×2
   bilinear blend; anisotropy is 1 (no elongated footprint);
   addressing is ClampToEdge. Out-of-range taps in the unatlased
   path therefore read the **edge texel**, with weights computed
   as if the texture extended by replication.
2. **Sample positions stay inside the inner rect.** Ideal
   interpolated UVs at fragment centres lie within the quad's
   [0,1]², i.e. within the placement's inner rect after remap.
   A bilinear footprint extends ≤ 0.5 texel past the sample
   position on each axis, so its taps touch at most the edge
   texel and **1 texel beyond** the inner rect.
3. **The gutter materialises the clamp.** Each cell reserves
   P = 2 texels around the inner rect, uploaded as
   edge-replicated texels (§3). Every tap in step 2 therefore
   lands either on the same texel as the unatlased fetch or on
   a gutter texel whose value equals the edge texel the clamp
   would have synthesised — same values, same weights, same
   blend. The second gutter texel is margin for step 4; on this
   census it costs nothing (a 2564×2820 cell tiles 3×2 exactly
   as a 2560×2816 canvas would).
4. **UV arithmetic is exact where it can be, bounded where it
   can't.** Endpoints are dyadic: page 8192 = 2¹³, integer texel
   numerators < 2¹³, so `u0, v0, u1, v1` are exact f32 values,
   and `remap` at the geometry's exact 0/1 corners reproduces
   them bit-exactly (asserted with `assert_eq!` on f32 in
   `uv_corners_exact`). Interior fragments interpolate the
   remapped attribute instead of [0,1]; in exact arithmetic the
   sample position is identical, in f32 the interpolation can
   differ by ~1 ulp of UV (≈ 2×10⁻⁴ texel at canvas scale),
   shifting bilinear weights by that fraction — below one output
   code except for inputs sitting exactly on a rounding
   boundary. **Honest status:** texel-value equality is proven
   by construction (steps 1–3); sample-position equality is
   exact at corners and ulp-bounded inside — the same class of
   variation wgpu already permits across drivers. The discharge
   is empirical, via §7's byte-probes and A/B pairs, which would
   localise any bleed to card borders immediately.
5. **Format and tests unchanged.** Page format is
   `Rgba8UnormSrgb`, identical to `upload_dyn` — sRGB decode is
   per-texel at fetch, so storage layout cannot change decoded
   values. The alpha tests (0.04 colour, 0.5 shadow) compare the
   fetched alpha — equal by steps 1–3, so discard decisions,
   silhouettes and shadows are unchanged.

**What would break neutrality (do-not list for the wiring):**
generating mipmaps for a page (coarse levels blend across
entries — the single most likely future regression); binding
`sampler_repeat` for a card; padding with transparent black
instead of edge replication (dark halo + alpha-test erosion at
the silhouette); P = 0; a non-power-of-two page whose endpoints
sit near f32 texel boundaries combined with P < 2.

## 5. Memory math (honest: atlasing is not a capacity saving)

| Quantity | Bytes |
|---|---|
| One canvas (CPU or its own GPU texture) | 28,835,840 (27.5 MiB) |
| One 8192² RGBA page | 268,435,456 (256 MiB) |
| Canvases per page (cells 2564×2820, 3×2) | 6 |
| Effective GPU bytes per resident canvas, full pages | ≈ 44.7 MB (**1.55×** its own texture) |
| Page utilisation, inner / cell (tested) | 64.45% / 64.65% |
| Synthetic census: 240 states → pages | 40 pages = 10 GiB (vs 6.9 GB as own textures) |

So packing uniform 2560×2816 canvases into 8192 pages *costs*
VRAM per resident state — 2816 does not divide 8192 kindly, and
no packer can change that arithmetic. The returns are elsewhere:

- **Upload traffic.** Today: ~28.8 MB re-uploaded per card per
  frame — over 1,811 frames at 1–2 cards/frame, ≈ **52–105 GB**
  of `write_texture` traffic per film. Atlased: 28.8 MB once per
  distinct state (240-state synthetic census: 6.9 GB; the real
  distinct count comes from W06's stats line on a real run). On
  the target PC the Vega 7 shares system RAM, so this traffic
  competes directly with the CPU painter for bandwidth.
- **Object churn.** Per frame per card today: 1 texture + 1
  view + 2 bind groups created and dropped. Atlased: zero —
  pages, views and the sampler are built once.
- **Bounded residency.** `max_pages` converts GPU texture
  memory into a hard, plannable budget (e.g. 4 pages = 1 GiB =
  24 states) instead of per-frame transient allocation.
- CPU-side memory is **unchanged** — the PaintCache still holds
  its canvases under W06's byte budget, and atlas pages are
  *additional* GPU-side copies. Both budgets must be counted
  together; that is a further argument for selective admission
  (§6), not for atlasing everything.

## 6. When NOT to atlas (honest limits)

- **Single-use states.** Continuous channels (talk, blink,
  non-recurring walk phases) mint states a film uses exactly
  once. An atlas slot for one costs the same upload as the Dyn
  path, plus occupancy until the whole page is recycled — and
  pages can only be recycled whole. Entry-granular eviction is
  impossible without a re-pack; the Dyn path's residency is
  zero by construction and strictly better here.
- **High-churn passages.** PERF.md notes paint misses cluster
  at the 14 segment starts; a working set that turns over
  faster than a page amortises gains nothing.
- **Small working sets.** A shot touching a handful of distinct
  states is already cheap on the Dyn path; atlasing adds a
  planning pass and page allocation for no measurable return.
- **Telemetry.** The audit path is CPU-only and never uploads;
  it is the atlas's *planner* (§3), never its customer.
- **Recommended admission policy — probation.** First paint:
  Dyn path. A state earns a slot only on its **second paint-
  cache hit** (proven reuse), pages allocated lazily up to
  `max_pages`; once pages are full, further states stay on Dyn
  rather than evicting a live page mid-film. Re-pack only
  between films. Worst case is then exactly today's behaviour
  plus a bounded number of pages.
- **Batch API caveat.** `pack()` packs a known set. The
  streaming film renderer does not know its set in advance —
  the telemetry-planned route (§3) is the intended consumer.
  An incremental packer (stable placements as entries arrive)
  is a different, harder contract and is deliberately **not**
  faked here.

## 7. What the lead must verify post-wiring

1. **CI green**, including the 11 atlas tests under the crate's
   `cargo test` once `mod atlas;` lands, and the existing suite.
2. **Byte-probes, atlased vs Dyn**, the determinism protocol's
   frames **1195–1204** and **795–814**: sha256-identical. Any
   difference localises to card borders — inspect those first
   (gutter upload and UV remap are the only new code paths).
3. **A/B 60 pairs** re-run with atlasing on: no PSNR regression
   vs the 23.02 dB record; diff images checked at card edges
   and shadow silhouettes specifically (§4.5's failure shape).
4. **Traffic claim measured, not asserted:** PERF-protocol film
   run both ways on the user's PC (RADV) — wall time and the
   new `atlas:` stats line (pages/placed/utilisation) printed
   at end of run next to W06's `paint-cache:` line.
