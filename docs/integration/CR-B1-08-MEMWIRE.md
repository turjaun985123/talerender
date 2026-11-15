# CR-B1-08 — WP2 mem-report print wiring + WP1.2 dirty-region activation

Chrome-Replacement Wave, Batch 1, Seat 8. Repo HEAD at time of
writing: `b093f3a`. All line anchors below are that HEAD,
verified by reading the code (not from V2-era docs — the
V2-era anchors in memreport.rs's own delivery note are stale
by ~700 lines in film.rs and ~100 in main.rs).

Deliverables (new files only; this seat edited nothing
existing and committed nothing):

| File | What |
|---|---|
| `src/memreport_print.rs` | WP2 glue: AdapterReport → memreport conversion, companion/full-block line builders, print entry points, Markdown delegation. 6 in-file tests. |
| `src/dyntex.rs` | WP1.2 activation: persistent Dyn-texture pool (pure planner + GPU half) wired to `dirtyrect`'s policy. 13 in-file tests. |
| this note | Both integration specs + the frame-parity argument for WP1.2. |

**Status, plainly:** both modules are COMPLETE as modules
(they compile, their tests pass — §5). The ACTIVATION of
both is **INCOMPLETE** in the only sense that matters: no
existing file calls them yet, because seats write new files
only and the call sites live in `main.rs` / `film.rs` /
`ab1.rs` / `dof.rs` / `scenefile.rs`, which are the lead's.
§1.3 and §2.3 are the exact, complete wiring the lead lands;
nothing else is missing. There is no technical blocker inside
either module.

---

## 1. WP2 — mem-report print wiring

### 1.1 Verified current state

- `gpucore::mem_stats_line()` renders the pinned line
  (`gpucore.rs` `format_mem_line`): `gpu-mem: accounted_bytes=
  … peak_accounted_bytes=… textures=… buffers=… readback_
  bytes=… sysfs_gtt_used_bytes=… sysfs_vram_used_bytes=…`.
  Its grammar is parsed by `tools/perf/vram_report.py`; it
  must not change. This seat's glue never reformats it.
- `gpucore::mem_snapshot()` (contract C2) and
  `memreport::{render_lines, render_markdown,
  gpu_timestamps_line}` already exist and are already
  declared in `main.rs` (`mod memreport;`, line 36). What was
  missing is only: (a) the conversion from the canonical
  `adapter::AdapterReport` to memreport's local `AdapterLine`
  (memreport's own note anticipated it), and (b) the prints.
- Every render mode's tail has the same shape today: a
  `wrote …` line, then under the comment *"VRAM ledger
  (W84 §5.3): its own line, after the wrote line"*:
  `println!("{}", crate::gpucore::mem_stats_line());`
- Film telemetry (`--film-telemetry`, `--telemetry`) prints
  NO gpu-mem line and must print no mem-report either
  (gpucore ruling D1: CPU-only mode; a structurally-zero
  ledger would be a fabricated-looking measurement). It is
  deliberately absent from the site table. `stroke-battery`
  also prints no gpu-mem line (CPU card writer) — absent too.

### 1.2 What `memreport_print.rs` provides

- `adapter_line(&AdapterReport) -> memreport::AdapterLine`
  and `adapter_line_for_info(&wgpu::AdapterInfo)` — the C7→WP2
  conversion, once, field for field (device_type keeps the
  `{:?}` spelling memreport's IntegratedGpu check compares).
- `companion_lines(&snap, &report, features) -> Vec<String>`
  — the `mem-report:` block (12 lines, 13 on integrated GPUs)
  followed by the `gpu-timestamps:` line. Pure.
- `full_block(gpu_mem_line, &snap, &report, features)` — the
  pinned line verbatim at index 0, then the companions. Pure.
- `print_companion(&report, features)` — prints companions
  beside an already-printed gpu-mem line.
- `print_full_block(&report, features)` — prints the gpu-mem
  line via the same `mem_stats_line()` call the sites use
  today, then the companions. **Recommended form.**
- `print_full_block_for_adapter(&wgpu::Adapter)` — describe +
  features + full block, for sites holding the adapter.
- `markdown_for(&snap, &report)` — WP6 perf-lab delegation.

Print order at every site (pinned by the glue's tests):
`wrote …` (existing) → `gpu-mem: …` (existing position,
byte-identical) → `mem-report: …` block → `gpu-timestamps: …`.
The only stdout change is additive lines after the gpu-mem
line; no existing line moves or changes. Snapshot caveat:
`mem_stats_line()` and `mem_snapshot()` snapshot the ledger
microseconds apart at a quiescent point (only println!s
between) — tracked figures agree; sysfs fields are
device-wide driver counters that can tick independently, as
they already could between any two probes.

### 1.3 Exact call sites for the lead

Module line first: `src/main.rs`, add
`mod memreport_print;` immediately after `mod memreport;`
(line 36).

| # | Site (HEAD b093f3a) | Existing gpu-mem line | Adapter in scope? | Exact wiring |
|---|---|---|---|---|
| 1 | `src/main.rs` `run()` (m1) | line 1091 | YES — `adapter` (line 730), `info` (731) | Replace line 1091's println with `crate::memreport_print::print_full_block_for_adapter(&adapter);` |
| 2 | `src/ab1.rs` `run_ab1()` | line 1579 | YES — `adapter` (594), `info` (595) | Same replacement as #1 |
| 3 | `src/dof.rs` `run_dof()` | line 528 | YES — `adapter` (143), `info` (144) | Same replacement as #1 |
| 4 | `src/film.rs` `run_film()` | line 3717 | **NO** — see below | Plumbing + replacement (below) |
| 5 | `src/scenefile.rs` `run_direct()` | line 1526 | **NO** — see below | Plumbing + replacement (below) |
| 6 | `src/scenefile.rs` `run_composer()` | line 2188 | **NO** — see below | Plumbing + replacement (below) |

Sites 4–6 plumbing (the only non-trivial part):

- **film.rs:** the adapter lives inside `Renderer::new`
  (lines 2584–2589: requested, described, printed, then
  dropped; only device/queue are stored). Lead: add two
  fields to `Renderer` — `adapter_report:
  crate::adapter::AdapterReport` and `adapter_features:
  wgpu::Features` — filled in `Renderer::new` from the
  `adapter` it already holds (`describe(&info)` is already
  computed there for the report line; add
  `adapter.features()`), and add a one-line accessor or make
  `run_film` call a `Renderer::print_mem_report(&self)` that
  forwards to `memreport_print::print_full_block(
  &self.adapter_report, self.adapter_features)`. Then
  replace line 3717's println with that call. (Storing
  `wgpu::Features` is safe: it is a plain bitflags value.)
- **scenefile.rs:** both paths get their device from
  `make_device(label)` (line 1173), which creates the adapter
  internally, prints the report line, and returns only
  `(Device, Queue)`. Lead: change `make_device` to also
  return `(AdapterReport, wgpu::Features)` (it already
  computes `describe(&info)`), thread the pair through
  `run_direct` / `run_composer` to their tails, and replace
  lines 1526 / 2188 as in #1–3 with
  `print_full_block(&report, features)`.

Alternative minimal form at sites 1–3 (if the lead prefers
not to touch the existing println): keep line as-is and add
immediately after it
`crate::memreport_print::print_companion(&crate::adapter::
describe(&info), adapter.features());` — `describe(&info)`
is already computed at each site's head for its `adapter:`
line, so the lead may hoist that value instead of recomputing.

---

## 2. WP1.2 — dirty-region activation (persistent Dyn textures)

### 2.1 Verified current path (the thing being replaced)

- `film.rs:2252` — `type Painted = (Vec<u8>, u32, u32,
  (f32,f32), (f32,f32))`; canvases are 2560×2816 RGBA today
  (`toonfull.rs`: `cw = 1280*2, ch = 1408*2`).
- `film.rs:2384–2491` — `resolve_chars` returns
  `Vec<Rc<Painted>>` in `TexSrc::Dyn` slot order (slot =
  positional index, assigned at line 2457).
- `film.rs:2989` — `Renderer::upload_dyn(&self, px, w, h)`:
  creates a fresh texture (descriptor: label `"card
  texture"`, `Rgba8UnormSrgb`, mip 1, sample 1, D2,
  `TEXTURE_BINDING | COPY_DST`), registers class E with the
  ledger, and issues ONE full `write_texture` (offset 0,
  `bytes_per_row = w*4`, `rows_per_image = h`, origin ZERO).
- `film.rs:3218–3224` — `render()`'s loop calls it once per
  card per frame, collecting `dyn_views` / `dyn_keep`;
  `dyn_keep` drops at line 3506 (textures die every frame).
  Views are consumed at line 3252 (`TexSrc::Dyn(i) =>
  &dyn_views[*i]`) and by the card-shadow bind group built
  from the same `tex_view` local.
- `film.rs:3018` — `render()` opens with
  `let _mem_scope = crate::gpucore::mem_scope();`, whose
  drop rolls back ledger classes E and G. Today's per-frame
  E registration for cards is therefore transient by design.
- Out of scope (verified): the standing-texture closure in
  `Renderer::new` (line 2619; uploads once at startup) and
  `composer.rs`'s env-map upload — one-time, no dirty region
  applies (dirtyrect's own note concurs).

### 2.2 What `dyntex.rs` provides

Two layers, so the policy is testable without a GPU:

- **Pure planner** (`SlotPlanner` / `DynPlanner`): per-slot
  decision state. `plan(tex_w, tex_h, px, w, h, prev)` →
  `UploadPlan { kind, writes, bytes_moved, coverage, … }`
  where kind is Full (with a named `FullReason`), Partial
  (one `WriteSpec` per merged dirty rect — origin,
  src_offset into the full frame buffer, stride, rows,
  extent), or Skip (with a named `SkipReason`). Decision
  order: no live texture at these dims / no committed
  contents → Full(NewTexture|Resized); identical allocation
  (ptr+len, under the retention contract) → Skip; no prev →
  Full(NoPreviousPixels); any length mismatch → Full
  (PreviousMismatch); empty diff → Skip(NoChanges);
  coverage > 0.60 → Full(CoverageFallback); merged rects
  > 256 → Full(TooManyRects); unaligned stride + multi-row
  rect → Full(StrideGuard); else Partial via
  `DirtyRects::from_diff` + `merged()`.
- **GPU pool** (`DynTexturePool`): owns `Vec<Option<GpuSlot>>`
  (texture + default view + dims + ledger bytes). `upload(
  device, queue, slot, px, w, h, prev) -> UploadPlan`
  creates/recreates the texture when the plan's premise
  requires it — descriptor and full-upload write copied
  field-for-field from `upload_dyn` — issues the plan's
  writes, commits the planner, records `PoolStats`.
  Ledger: `mem_alloc_texture(E, …)` ONCE per creation,
  `mem_free_texture(E, …)` on recreate and on pool Drop —
  a standing allocation, not a per-frame transient.
  `view(slot)`, `live_textures()`, `ledger_bytes()`,
  `stats()`, `stats_line()` (`dyn-tex: …`, the tree's
  stats-line style) are the read side.

### 2.3 Exact wiring for the lead (film.rs)

1. `src/main.rs`: add `mod dyntex;` among the mod lines —
   suggested anchor: after `mod dof;` (line 30), keeping
   the d-cluster together.
2. `Renderer` gains two fields:
   `dyn_pool: crate::dyntex::DynTexturePool` (init
   `DynTexturePool::new()` in `Renderer::new`) and
   `prev_dyn: Vec<Option<std::rc::Rc<Painted>>>` (init
   empty). `prev_dyn[slot]` must always hold the Rc whose
   bytes were last uploaded to that slot — the pool's
   caller contract (§3, case C1).
3. In `render()` (line 3012), REPLACE the upload loop at
   lines 3218–3224 with the pool drive, and MOVE it above
   the `let _mem_scope` line (3018) — see the ledger warning
   below. Shape (disjoint field borrows; `dyn_tex` is the
   existing parameter):
   ```rust
   // before `let _mem_scope = ...`:
   for (slot, p) in dyn_tex.iter().enumerate() {
       if self.prev_dyn.len() <= slot {
           self.prev_dyn.resize_with(slot + 1, || None);
       }
       let prev = self.prev_dyn[slot].as_ref().map(|rc| rc.0.as_slice());
       self.dyn_pool.upload(&self.device, &self.queue, slot, &p.0, p.1, p.2, prev);
       self.prev_dyn[slot] = Some(p.clone());
   }
   let dyn_views: Vec<&wgpu::TextureView> = (0..dyn_tex.len())
       .map(|s| self.dyn_pool.view(s).expect("uploaded above"))
       .collect();
   ```
   Borrow note: `prev` borrows `self.prev_dyn` while
   `upload` borrows `self.dyn_pool` mutably — disjoint
   fields, accepted when accessed as `self.field` directly
   as sketched; if the borrow checker objects at the call,
   bind `let prev_ptr_len` first or split via a local
   `let pool = &mut self.dyn_pool;` after computing `prev`
   into an owned small value — the pixels themselves are
   only read inside `upload`, so the lead may instead pass
   `prev` as the raw slice re-borrowed from
   `self.prev_dyn` in the same statement (the sketch's
   shape). Do NOT clone pixel bytes anywhere: `prev` is a
   borrow of the retained Rc's Vec, and `p.clone()` is an
   Rc clone (zero copy) — the same retention dirtyrect's
   note prescribed.
4. Adjust the one consumer of `dyn_views`: line 3252's
   `TexSrc::Dyn(i) => &dyn_views[*i]` becomes
   `TexSrc::Dyn(i) => dyn_views[*i]` (the Vec now holds
   references). The card-shadow bind group uses the same
   `tex_view` local and needs no further change.
   Delete `dyn_keep` and its `drop(dyn_keep)` (line 3506).
5. Delete `upload_dyn` (lines 2989–3010) — the pool's
   create + full write are its verbatim replacement; its
   per-frame ledger line goes with it (replaced by the
   pool's create-time registration).
6. Optional: print `self.dyn_pool.stats_line()` in
   `run_film` after the completion line, beside the gpu-mem
   block. Optional and additive only.

**Ledger warning (the one real integration hazard).**
`render()`'s `MemScope` (line 3018) snapshots classes E/G
and restores them on drop. If the pool's first uploads
happen INSIDE the scope (the loop's current position), the
pool's standing E registration is rolled back at the end of
frame 1 and never re-registered (the textures persist) —
the gpu-mem / mem-report class-E figures then undercount by
`pool.ledger_bytes()` for the rest of the run. Pixels are
unaffected (the ledger is log-only), but the telemetry
would lie. Step 3's move — pool drive BEFORE the
`_mem_scope` binding — is the fix: the dyn uploads depend
only on device/queue + `dyn_tex`, nothing the scope
protects, and every other transient (frame uniforms,
vertex/uniform buffers, PostChain) is created after the
current scope line and stays inside it. Expected telemetry
delta once wired, stated so it is not misread as a
regression: end-of-run `gpu-mem:` accounted_bytes GROWS by
the pool's standing bytes (slots × 28,835,840 B at
2560×2816 — e.g. ≈86.5 MB at 3 concurrent characters)
versus today, because today's cards are transient-in-scope
and excluded from the final standing total. The line's
grammar is unchanged; the frames are unchanged (§3).

### 2.4 What stays INCOMPLETE, and why

- The film.rs wiring itself (§2.3) — lead's files, seat
  rule. **Blocker: none technical.** The restructure
  (MemScope hoist) is a lead decision by design: it touches
  the render hot path's accounting, and this seat was
  chartered to deliver the module + spec, not to edit
  film.rs.
- RADV/hardware confirmation of partial `write_texture`
  with sub-rect origins on the user's PC — impossible from
  this VM (no /dev/dri; all proof here is lavapipe-class,
  per the wave's standing caveat). The byte-probe gate
  (§3, verification) is the check that must run at
  integration: render the Onoda film (or a chunk) with the
  pool wired and diff frames against the established
  native reference — expected: byte-identical.

---

## 3. Frame-parity argument for WP1.2

Claim: with §2.3's wiring, every frame the pool produces is
byte-identical to the frame today's fresh-texture path
produces. The renderer samples the card textures through a
linear/clamp sampler at mip 0 (`sampler_clamp`, film.rs);
the final frame is a deterministic function of the textures'
texel bytes at sample time. So parity reduces to one
statement per slot per frame:

> **(P) At the moment render()'s passes sample slot S's
> texture, its texels equal the current frame's Painted
> bytes for S — exactly what upload_dyn's fresh texture
> holds.**

The pool maintains the invariant **(I): after every
`upload()` call for slot S, S's texture holds exactly the
`px` passed to that call**, given it held the `prev` passed
to that call (or the call was a Full). Proof by plan kind:

- **Full** — the write is upload_dyn's own call, field for
  field, into a texture created with upload_dyn's own
  descriptor. (I) holds unconditionally; (P) follows.
- **Skip / IdenticalAllocation** — no write; the claim is
  the texture already holds `px`. It holds the last
  committed buffer (invariant from the previous frame), and
  pointer+length equality says `px` IS that buffer — sound
  iff the allocation stayed alive across the interval,
  which the caller's retained `Rc<Painted>` guarantees
  (an Rc clone is held in `prev_dyn` from commit to check;
  a live allocation's address cannot be reused). Dims are
  unchanged (checked before this path: a dims change is a
  recreate → Full).
- **Skip / NoChanges** — no write; the byte diff
  (`DirtyRects::from_diff`, exact-size buffers only) found
  zero differing bytes between the texture's known contents
  (`prev`, by (I) + contract) and `px`. So the texture
  already equals `px`.
- **Partial** — writes exactly the merged dirty rects.
  `merged()` is a SUPERSET of the changed texels (dirtyrect's
  construction: row-span diff finds every differing row and
  spans its first–last differing texel; merging only grows
  coverage to bounding boxes). Every texel NOT written is
  therefore byte-equal in `prev` and `px`; the texture held
  `prev` there (by (I) + contract), so it already holds
  `px`'s value. Every texel written receives `px`'s bytes
  (each WriteSpec copies from the full current buffer at
  the rect's own offset/stride — the simulated-application
  test in dyntex.rs proves the mapping reproduces `px`
  byte-for-byte). Hence the texture equals `px` everywhere.
- **Ordering** — `queue.write_texture` calls are ordered on
  the queue ahead of the encoder submit that samples them,
  exactly as today's upload_dyn writes are (same call, same
  position in render()'s flow). Partial writes within one
  upload touch disjoint-or-overlapping rects but all read
  from the same immutable source buffer and write the same
  values to any overlap, so their relative order cannot
  matter.
- **Sampling identity** — format, mip count, usage, view
  descriptor and sampler are unchanged, so identical texel
  bytes sample identically. No mipmaps exist (mip_level_
  count = 1) — there is no generated-mip state a partial
  upload could leave stale, which is the classic
  partial-upload parity trap and does not apply here.

### Every case where parity might NOT hold, named

- **C1 — caller passes a `prev` that is not the slot's
  resident bytes.** The diff paths trust `prev` as the
  texture's contents. If the lead's `prev_dyn` bookkeeping
  ever desyncs from the pool (e.g. indexing prev by frame
  position instead of slot, pruning a slot's retained Rc
  while its texture lives on, or two slots sharing one
  retained Rc), a Partial/Skip can leave stale texels —
  silently wrong pixels. Mitigations in the module: length
  mismatches force Full; dims mismatches force Full; a
  missing prev forces Full. What cannot be detected in
  general is a same-length, wrong-content prev — that is
  why §2.3 pins the bookkeeping shape (per-slot Vec, set
  from the same Rc that was uploaded, never pruned while
  the pool lives). This is the activation's principal risk
  and it lives entirely in the lead's 6-line loop.
- **C2 — identical-allocation fast path without retention.**
  If a caller ever passes a freshly-painted buffer whose
  allocation coincidentally reuses the freed address + length
  of the last commit, the pool would Skip over different
  bytes. The §2.3 wiring makes this unreachable (the
  previous allocation is retained alive by `prev_dyn`, so
  its address cannot be recycled while the pool compares
  against it). A caller that cannot retain must not rely on
  this path — but note it also cannot trigger it through
  the pool API as wired, since the comparison target is the
  last commit through the same retained chain.
- **C3 — texture recreation races.** None: wgpu textures
  are reference-counted handles; dropping the old GpuSlot
  on resize cannot invalidate views already created from
  it within the same frame — and the wiring creates views
  only AFTER all uploads for the frame, from the pool's
  current slots. A resize mid-frame is impossible (dims
  come from the same Painted being uploaded).
- **C4 — driver variance in partial write_texture.** The
  partial call shape (sub-rect origin, full-buffer source
  with per-rect offset + full-row stride) is the shape
  dirtyrect.rs's note specified and WebGPU defines; on an
  exotic driver it is conceivably slower, but its SEMANTICS
  are not driver-optional — a conforming driver writes
  exactly those texels. Residual risk is therefore
  conformance-class, retired by the byte-probe gate at
  integration (§2.4), first on lavapipe/CI and finally on
  RADV at the PC session. The StrideGuard keeps one
  less-exercised layout (unaligned multi-row stride) off
  the table entirely by falling back to Full.
- **C5 — truncated buffers.** A Painted shorter than w*h*4
  would make today's full write_texture read out of the
  slice (wgpu validates and panics — today's behaviour,
  preserved on the Full path). The diff paths refuse such
  buffers (PreviousMismatch → Full) rather than diffing a
  prefix and missing tail changes — so the pool never
  converts a would-be-loud failure into silent wrong
  pixels.
- **C6 — ledger figures change (not pixels).** §2.3's
  warning: accounted/peak bytes grow by the standing pool.
  Any gate that compares gpu-mem FIGURES across the wiring
  (rather than frame bytes) will see a delta; it is
  truthful (the textures really are resident now) and is
  named here so it is not misread.
- **C7 — first frame after process start / after a slot's
  first appearance.** No resident contents exist; plan is
  Full(NewTexture) — identical to today by construction.
  A slot that vanishes for some frames and returns keeps
  its texture AND its `prev_dyn` entry, so (I) still holds
  on return; if the lead ever prunes `prev_dyn` for absent
  slots, the return degrades to Full(NoPreviousPixels) —
  slower, still correct.

---

## 4. Test inventory

`memreport_print.rs` (6): adapter conversion field-by-field;
full block starts with the gpu-mem line verbatim and
contains exactly one; companion order (mem-report block,
integrated note, timestamps last); timestamps both ways
through the glue; sysfs `unavailable` propagation; Markdown
delegation with converted adapter.

`dyntex.rs` (13): first-upload Full(NewTexture);
no-prev Full; resize Full; truncated prev/cur →
Full(PreviousMismatch), never partial; identical allocation
Skip; identical contents Skip via diff; one-pixel Partial
with the exact WriteSpec (origin/offset/stride); **partial-
application simulation** (deterministic LCG mutations,
localised cluster pattern) proving the patched texture
equals the current buffer byte-for-byte; coverage > 0.60 →
Full(CoverageFallback); > 256 merged rects →
Full(TooManyRects); stride guard (multi-row → Full,
single-row → Partial at w=65); slot independence;
invalidate-after-recreate forces Full; stats accounting +
line shape.

## 5. Verification performed by this seat

- Both modules compiled and ALL tests executed via a
  scratch harness in /tmp (stub `wgpu`/`gpucore`/`adapter`
  surfaces; the fake Queue implements real write_texture
  copy semantics): **54 passed, 0 failed, 0 warnings** —
  the count includes dirtyrect.rs's and memreport.rs's own
  in-file tests pulled in unchanged, this seat's 19 tests,
  and one harness-only end-to-end pool test (full → partial
  → skip → coverage-full → independent slot → resize, with
  the resident texture bytes asserted after every frame and
  the ledger total cross-checked). The harness is scratch,
  not a deliverable; the in-file tests are the CI gate.
- The harness caught three real defects before delivery
  (an `Eq` derive over an f32 field; two u64/u32 test
  mismatches) and one policy interaction worth knowing:
  full-width scattered changes inflate row-span coverage
  past the 0.60 fallback very quickly (a 17%-of-texels
  scatter measured 92.5% merged coverage) — the fallback is
  doing its job; localised changes (the actual blink/mouth/
  gesture case) partial-upload as intended.
- NOT verified here (CI's job at integration): compilation
  against the real wgpu 30.0.2 within the full crate (the
  GPU call shapes are copied field-for-field from
  film.rs's proven `upload_dyn` and dirtyrect's specified
  sketch, and the harness stubs mirror those signatures,
  but only CI closes this); any GPU execution; the
  byte-probe gate of §2.4.
- Repo state after this seat: three new untracked files,
  zero edits to existing files, no `target/` left behind,
  no commits.
