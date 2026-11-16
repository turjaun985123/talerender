# GPUFIRST GPUX — the GPU executor (Phase 0 prototype core)

**Seat:** GPUX, GPU-first TaleRender Phase 0.
**Deliverables:** `src/gpupaint.rs` (executor, replacing the TRACE
seat's `replay` stub inside the `--gpu-paint` harness),
`src/gpupaint.wgsl` (the shader, embedded via `include_str!` —
chosen over string consts in the .rs so the WGSL stays reviewable
as WGSL), this document. No commits by this seat; CI is the
build/run verdict.
**Normative math source:** `GPUFIRST-INVA-RASTER-SPEC.md` (INV-A).
Where the coordinator's charter summaries and the spec differed,
the spec won. HEAD at time of writing: `bc12a5c` + the uncommitted
Phase-0 harness files (gputrace.rs, gpupaint.rs, gpupaint_states.rs,
gpupaint_time.rs and the trace hooks).

## 1. Interface

```
talerender --gpu-paint replay <list.json> <out.png>     (feature `gpupaint`)
```

Reads a `gputrace::DisplayList` (via `DisplayList::from_json` —
the TRACE seat's fixed interface: f32 as `{"b": u32}` bits, mask
ids = prim indices, plates as base64), replays it on the GPU, and
prints, in order: the adapter report line; one line per prim with
its dispatch size; `replay: N prims, M dispatches, canvas WxH`;
`gpu_wall_ms: <queue submit -> completion, readback excluded>`;
`sha256: <hex of the RGBA pixel bytes>`. The PNG is written by
the harness's existing `save_png`. Running `replay` twice on the
same list prints identical hashes (§5). Exit codes follow the
harness audit table: 2 unreadable/unparseable list, 3 executor
failure (including a masked prim referencing `NO_MASK`, a mask id
that does not name an earlier mask prim, or a bad plate index —
all fail loudly, never guessed), 4 image I/O.

Prim coverage — every variant in the landed `gputrace::Prim`:
`FillRect`, `FillEllipse`, `FillEllipseMasked`, `StrokePolyline`,
`StrokeQuad`, `StrokeEllipse`, `FillPolygon`, `FillPolygonMasked`,
`PolygonMask`, `EllipseMask`, `CompositePlate`, plus the two
amendment variants `HeadShade` (both forms) and `MaskedBlend`.

## 2. Architecture

- **Canvas:** one storage buffer, one `u32` per pixel, RGBA8
  packed (R in the low byte — its little-endian memory image is
  exactly the CPU's `Vec<u8>` RGBA; a big-endian conversion is
  applied on readback under `cfg(target_endian = "big")`).
  Chosen over a storage texture for exact read-modify-write
  control and adapter independence. wgpu zero-initialises it =
  `Painter::new`'s zeroed start. The canvas is GPU-resident;
  readback happens ONLY in this harness, for PNG + hash.
- **Dispatch:** one compute pass per prim, all passes in a single
  command encoder submitted once, in list order — pass order in
  one submit is execution order, so blend order = CPU call order
  (INV-A §6). Each dispatch covers exactly the prim's CPU bbox
  (Painter::bbox rules, per-prim padding from the spec §3 table),
  workgroup size **8×8 = 64**. Per-pixel independent invocations:
  no atomics, no cross-pixel reads except read-only plate data
  and mask buffers completed by earlier passes.
- **Host prep (Rust, per prim):** bbox (Painter::bbox verbatim),
  ellipse extents (separate `.cos()`/`.sin()`, spec §3.1),
  `(sin, cos)` of every rotation via `f32::sin_cos` uploaded as
  uniforms (never computed in-shader), CapFan construction
  (cap_fan's formula replicated textually — `scene.rs`'s CapFan
  fields are module-private; same Rust libm calls in the same
  order give identical `n`/`cos_half`), `stroke_ellipse`'s
  144-point flattening and `stroke_quad`'s `flatten_quad(.., 24)`
  (the latter calls `scene::flatten_quad` directly). All
  transcendental or step-function setup is host-side, per spec
  §1.8/§2.3.
- **Device:** `gpucore::new_instance` / `request_adapter` /
  `request_device_default` (the crate's existing helpers; wgpu
  30.0.2 + pollster, NO new dependencies — the JSON reader is
  gputrace's, the SHA-256 is hand-rolled in gpupaint.rs and
  unit-tested against the standard vectors).

## 3. Mask scoping (INV-B rule — normative for this executor)

A CPU mask is a full-canvas `Vec<bool>` (2560×2816 ≈ 7.2 MB, up
to ~11 built per paint — the dominant fixed overhead). The GPU
executor NEVER allocates a canvas-sized mask:

- Each mask prim (`PolygonMask` / `EllipseMask`) gets a buffer
  sized to **its own geometry bbox** (the CPU prim's unpadded
  `Painter::bbox`), allocated lazily at its dispatch,
  zero-initialised (the CPU's `vec![false]` start).
- The scope `(origin_x, origin_y, w, h)` travels in the consumer's
  uniform block (`Params.mk`). Mask writers index the buffer
  locally (dispatch origin = mask bbox origin). Masked consumers
  (`FillEllipseMasked`, `FillPolygonMasked`, `MaskedBlend`) call
  `mask_at(x, y)`: translate global → mask-local; **out-of-scope
  reads are false** — exactly the CPU value there, since the CPU
  mask is false everywhere outside the mask prim's own bbox.
- One buffer per mask prim index, shared by all its consumers;
  total mask memory per list is the sum of mask bbox areas, not
  11 × canvas.

## 4. The blend-exactness argument

The CPU model (spec §2.3) is: per prim, per pixel — read u8,
blend in f32, round to u8, store — with per-step quantisation.
The executor reproduces that literally:

- The canvas stores packed u8; every invocation unpacks, blends,
  repacks. There is no float accumulation anywhere across prims,
  no hardware blending, no premultiplied path.
- `blend_at` keeps both CPU fast paths verbatim (alpha 0 = no-op;
  alpha 255 = byte copy) and the general path's exact expression
  `((s*sa + d*da*(1.0 - sa)) / oa) * 255.0` in written order.
- `blend_cov_at` keeps the pre-quantisation: coverage first
  becomes a u8 alpha (`round(src_a * cov)` clamped), and only
  then blends (spec D5 — `16/255 ≠ 1/16` is preserved).
- **round() trap (amendment 2, spec §2.1):** Rust `f32::round` is
  half-AWAY-from-zero; WGSL `round()` is half-to-EVEN, and exact
  .5 products are systematic here (`src_a · k/16` ties whenever
  `src_a·k ≡ 8 (mod 16)`). The shader therefore never calls
  WGSL `round()`; it implements
  `round_away(x) = sign(x) * floor(abs(x) + 0.5)` and uses it at
  every quantization site: blend_cov's alpha pre-quantisation
  and blend's per-channel + alpha stores. (All sites are in fact
  non-negative, where this reduces to `floor(v + 0.5)`; the
  sign-aware form is kept so no future signed call site can
  silently change meaning.)
- `cov_area0` is instantiated per distance function with the
  spec's exact band rule: 4×4 binary inside tests at
  ±0.125/±0.375 **only when |d0| < 1 (strict)**; outside, `cov_of`
  — which is then saturated 0/1 (spec D1). The fractional cov_of
  ramp is reachable only through `HeadShade`, as on the CPU.
- Strokes: the two CPU passes are fused per pixel (spec §6.1
  item 2 sanctions this: pass 1's `dmin` is used only at the
  pixel's own index as a skip/branch oracle). The observable
  semantics are preserved exactly: `d0 = dmin_centre - hw`;
  `d0 >= 1.0` rejects; the fan-aware union distance is
  recomputed **iff `d0 > -1.5`**; coverage comes from that
  distance via cov_area0, whose 16 sub-samples also use the
  fan-aware closure. Pass 1's per-segment stamp boxes (hw+2,
  clamped to the union bbox) cannot change the result: a segment
  that does not stamp a pixel has centre distance > hw+1 to it,
  so it can neither be the min at a pixel with `d0 < 1.0` nor
  affect the `-1.5` branch decision (for `d0 <= -1.5` coverage
  is 1 under both distance models, fan shift ≤ sagitta 0.25).
  Fan substitution gates on the exact `t == 0.0` / `t == 1.0`
  from the same `seg_dist_t` arithmetic, with the CPU's
  if/else-if precedence for the single-segment case.
- `ellipse_mask` uses the implicit quadratic
  `(lx/rx)² + (ly/ry)² <= 1.0` at pixel centres (spec §4.2/D3) —
  NOT `ellipse_sd`. `fill_polygon_masked` is two centre booleans
  + full `blend` (no coverage), `masked_blend` is mask boolean +
  full `blend` over the recorded rect, `fill_ellipse_masked` is
  mask gate + full `cov_area(ellipse_sd)` — per spec §4.5.
- `HeadShade`: centre samples only, `cov_of` ramp (no
  supersample). Form 0: `unshaded = clamp(0.5 - ellipse_sd(circ))`;
  form 1: `unshaded = clamp(hypot(q - circ_c) - circ_r.0 + 0.5)`
  written in the CPU's op order (`h - R + 0.5`). The CPU's
  `cov_head <= 0` continue is kept. Bbox is `head_c ± rad` with
  the recorded `rad` (form 0: `rp*RES*1.05`; form 1: `rp*res + 1`)
  — the recorder owns the difference, the executor owns neither.
- `composite_plate`: the CPU's inverse lean map ported as-is
  (pivot (256, 1000), anchor, +(384, 256), ×RES=2), bilinear
  sampling on the 0–255 f32 scale in the CPU's written term
  order, sampled RGB **truncated** (`u32()` truncates toward
  zero; values are in [0, 255]) per spec D8, then `blend_cov`
  with source alpha forced to 255 at coverage `sampled_a / 255`.
  The dispatch bbox is the forward-mapped corner AABB, computed
  on the host with the same `sin_cos` and expression order.

## 5. Determinism

Per-pixel independent invocations; the only writes are the
invocation's own canvas pixel (blend entries) or its own mask
element (mask entries). No atomics, no reductions with float
order dependence (the in-shader min loops are exact `min`, the
supersample count is an integer count — both order-free per spec
§2.3). Sequential pass order in one submit fixes blend order.
`replay` twice on the same list therefore prints an identical
sha256 on the same adapter/driver; the harness compares the two
printed hashes.

## 6. Divergence risks — engineered around, and residual

Engineered around:
- **round half-even vs half-away** — explicit `round_away` (§4).
- **Per-primitive transcendentals** — all `sin`/`cos`/`acos` that
  are constant per prim (rotations, fan construction, ellipse
  point lists, bbox extents) are computed on the host in Rust.
- **powi** — spelled as explicit multiplications everywhere.
- **WGSL `round()`** — never called.

Residual (cannot be eliminated in WGSL; the parity seats will
measure the consequence — nothing here is silently approximated,
these are the known 1-ulp-class exposures, cf. spec §8):
- **`hypot`**: Rust `f32::hypot` is a libm routine with
  overflow-avoiding scaling; WGSL has no hypot, so the shader
  uses `sqrt(x*x + y*y)`. At this painter's magnitudes (≲ a few
  thousand px) overflow/underflow cannot occur, but results can
  differ by ~1 ulp, which can flip a `dist < 0` sign test, a
  `t == 0/1` fan gate within 1 ulp of an endpoint, or a .5
  rounding. Sites: `seg_dist_t` (every stroke/polygon edge),
  `fan_sd`, the fill_rect corner term, HeadShade form 1.
- **`fan_sd`'s per-sample `acos` / `cos`**: cannot be hoisted
  (depends on the sample direction). WGSL/vendor transcendental
  accuracy is not correctly-rounded by specification; the cap-end
  region is the expected worst case (spec §8 rates fan_sd HIGH).
- **`ellipse_sd` division chain**: `sqrt` is correctly rounded,
  but the `grad` normalisation chains divisions whose inputs can
  sit 1 ulp off if any upstream value is; boundary sign flips
  change coverage by 1/16 steps.
- **FMA contraction**: Rust never contracts `a*b + c`; a WGSL
  compiler may. WGSL has no portable contraction opt-out in
  wgpu 30. Written expression order is preserved everywhere so
  contraction is the only possible deviation, worth ≤1 ulp on
  the affected products (blend numerator, seg_dist_t dot
  product, ellipse_sd, flattening — the latter two are host-side
  and unaffected).
- **`u32(f32)` for plate RGB truncation**: CPU `as u8` saturates;
  WGSL float→int conversion of an out-of-range value is
  undefined. Bilinear outputs of in-range plate bytes are within
  [0, 255] up to 1-ulp excursions (weights summing to 1±ulp);
  a -ε excursion truncates to 0 under both semantics in
  practice, a 255+ε excursion is the only theoretical gap.

## 7. Notes for the coordinator / other seats

- The executor lives in the TRACE seat's `gpupaint.rs` (the
  designed seam: its `replay` stub), not a separate module —
  `main.rs` and `Cargo.toml` needed no changes from this seat.
- `HeadShade` / `MaskedBlend` had already landed in
  `gputrace.rs` (with the `form: u8` field) by the time this seat
  integrated; the executor matches those definitions as-is and
  changed nothing in gputrace.rs. One stale line to fix there:
  the gputrace module header still says the head-shade and teeth
  loops "are absent from the stream" — they are recorded now.
- `compare` and `time` remain the other seats' stubs.
- Local build check was impossible (no cargo on this VM);
  compile verification is CI's, per the programme's rule.
