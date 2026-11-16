# GPUFIRST-TRACE — Phase 0 display-list trace hook (seat TRACE)

Status: landed, uncommitted, for coordinator integration. HEAD at
authoring: `bc12a5c`. Everything here is behind the cargo feature
`gpupaint` (default OFF, no new dependencies).

## 1. What this builds

Phase 0's premise: record the EXACT semantic primitive stream the
CPU painter executes for a paint (a display list), then replay it
on GPU and compare byte-for-byte. This seat builds the recording
half plus the harness CLI skeleton. The GPU executor (replay) is a
later seat; its entry points exist as loud stubs.

Components:

- `src/gputrace.rs` — the `Prim` / `PlateData` / `DisplayList`
  types (fixed interfaces, verbatim from the brief), a
  thread-local recorder (`begin` / `record` / `record_mask` /
  `push_plate` / `take` / `last_mask_id`), one `trace_*` shim per
  hook site, and hand-rolled JSON serialization (write) +
  `DisplayList::from_json` (read, reusing `scene::parse_json`).
- `src/gpupaint.rs` — the `--gpu-paint` CLI: `run(args) -> i32`,
  subcommands `record-battery` / `record-states` (working, pure
  CPU) and `replay` / `compare` / `time` (stubs: stderr message,
  exit 3). Also `state_spread()` (empty vec; the STATES seat fills
  it) and `finish_card()` (the battery cards' trace-window exit).
- Hooks in `src/scene.rs` (`Painter`), `src/toonfull.rs`
  (free functions), and trace windows in `src/battery.rs`.
- `Cargo.toml`: `[features] gpupaint = []`. `src/main.rs`: the
  two module declarations + a pre-loop dispatch for `--gpu-paint`
  + one extra `--help` line, all cfg-gated.

CLI behaviour:

```
talerender --gpu-paint record-battery <outdir>   # prims/toon/face-*.png + .json each
talerender --gpu-paint record-states <outdir>    # <name>.png + <name>.json per state
talerender --gpu-paint replay <list.json> <out.png>   # stub, exit 3
talerender --gpu-paint compare <a.png> <b.png>        # stub, exit 3
talerender --gpu-paint time <list.json>               # stub, exit 3
```

## 2. Hook map and the stroke call graph (verified in code)

Each `Painter` hook is a single cfg-gated call at the TOP of the
method, recording the semantic prim with the arguments AS
RECEIVED:

| Method | Recorded prim |
|---|---|
| `fill_ellipse` | `FillEllipse` |
| `stroke_polyline` | `StrokePolyline` (pts as received) |
| `fill_polygon` | `FillPolygon` |
| `fill_rect` | `FillRect` |
| `polygon_mask` | `PolygonMask` (a mask prim; sets last-mask) |
| `ellipse_mask` | `EllipseMask` (a mask prim; sets last-mask) |
| `fill_ellipse_masked` | `FillEllipseMasked` (mask resolved per §3) |
| `toonfull::fill_polygon_masked` | `FillPolygonMasked` (mask resolved per §3) |
| `toonfull::composite_plate` | `CompositePlate` (+ plate pixels via `push_plate`) |
| head-shade block, `toonfull::paint_toon_full` | `HeadShade` form 0 (block-level hook, §4) |
| head-shade block, `scene::paint_toon` | `HeadShade` form 1 (block-level hook, §4) |
| laugh teeth loop, both painters | `MaskedBlend` (block-level hook, §4) |

The last three hooks are not method tops — the loops they record
are open-coded blocks inside the painters, so each hook is one
cfg-gated call placed at the block, after the locals it captures
(the computed centres/radii/bbox) exist and before the loop runs.

Deliberately NOT hooked, because they delegate:

- `Painter::stroke_ellipse` flattens the ellipse to 145 points
  and calls `self.stroke_polyline(&pts, width, col)`
  (scene.rs, in `stroke_ellipse`'s body). The polyline hook
  records the flattened points — the exact executed geometry.
- `Painter::stroke_quad` calls `flatten_quad(p0, p1, p2, 24)` and
  delegates to `stroke_polyline` the same way. `stroke_quad` is
  also called DIRECTLY by toonfull (brows, monocle chain, mouths,
  scarf shade); every one of those calls still passes through
  `stroke_polyline`, so the delegation record covers them all.
  Verified by grep: there is no other path from `stroke_quad` /
  `stroke_ellipse` to pixels — both bodies consist of the
  flatten + delegate and nothing else.

Consequences for the executor seat: the `StrokeEllipse` and
`StrokeQuad` variants exist in the `Prim` enum (fixed interface)
but never appear in a recorded list from these hooks; recorded
ellipse/quad outlines are `StrokePolyline`s with 145 / 25 points.
A replay that wants byte parity must execute the polyline form,
not re-derive an ellipse.

One guard: `stroke_polyline` returns before painting when
`pts.len() < 2`; the trace shim mirrors that guard so a no-op
call is not recorded as an executed primitive.

Recording is GLOBAL to the `Painter`: `stroke_polyline` and
friends are also used by `paint_toon`, the world/texture painters
in scene.rs (`paint_grass`, `paint_planks`, …), film.rs's texture
painters, and everything else that paints through `Painter`.
There is no per-callsite filtering. The harness controls
`begin()` / `take()` windows so only the target paint is
captured: a call made outside a window is a no-op (one
thread-local borrow check). Paints that run concurrently on
other threads are unaffected (the recorder is thread-local);
paints interleaved on the SAME thread inside a window WOULD be
captured — the drivers open windows tightly around a single
card/state paint, and production painting is single-threaded per
canvas in every current call path.

Battery windows: the per-card boundaries exist only inside
`battery::run_battery`, so the windows live there (three
cfg-gated `begin_card` / `finish_card` pairs: prims card at the
spec canvas size, toon and face cards at paint_toon's fixed
2560x2816). `begin_card` opens a window ONLY when a `--gpu-paint`
driver has armed the recorder (`gputrace::set_armed`), so a
`gpupaint` build running the production `--scene stroke-battery`
mode writes no JSON and behaves exactly like production.
`record-states` needs no such guard: its driver calls
`paint_toon_full` directly, opening `begin(2560, 2816)` itself.

## 3. Mask-id threading: proof, the paths where naive
##    last-mask FAILS, and the fix landed

Brief's design: `polygon_mask`'s hook sets a thread-local
`last_mask_id`; masked fills read it. A mask's id is its index in
`prims`. I verified every masked-fill call path in the tree
(grep over `polygon_mask` / `ellipse_mask` / `fill_ellipse_masked`
/ `fill_polygon_masked`; call sites exist only in scene.rs's
`paint_toon` and toonfull.rs's `paint_toon_full` +
`fill_polygon_masked` itself):

Where naive last-mask is CORRECT (single masked fill, or
consecutive fills over one mask with no mask prim recorded in
between):

- paint_toon bean shade + highlight: `mask = polygon_mask(bean)`,
  then two `fill_ellipse_masked(.., &mask)` back to back.
  `fill_ellipse_masked` records no mask prim, so last-mask is
  still the bean for the second fill. ✓
- paint_toon laugh tongue: `m = polygon_mask(mouth pts)`, one
  `fill_ellipse_masked(.., &m)`. ✓
- paint_toon shock tongue: `m = ellipse_mask(..)`, one
  `fill_ellipse_masked(.., &m)` — requires `ellipse_mask` to set
  last-mask too; its hook does. ✓
- toonfull open-mouth tongue (closure in paint_toon_full):
  `msk = ellipse_mask(..)`, one `fill_ellipse_masked`. ✓
- toonfull laugh tongue: `msk = polygon_mask(pts)`, one
  `fill_ellipse_masked`. ✓
- toonfull apron panel: ONE `fill_polygon_masked(.., &bean_mask)`
  — correct as a first use. ✓ (but see the shade below)

Where naive last-mask FAILS (flagged to the coordinator; the
brief asked for exactly this verification):

1. **`fill_polygon_masked` poisons last-mask for its own
   caller.** Its first line is `let own = p.polygon_mask(pts);`
   — that call records a `PolygonMask` prim (the panel's own
   mask) and moves last-mask to it. So in the STRIPES loop
   (paint_toon_full: up to 8 iterations of
   `fill_polygon_masked(&mut p, &rect, &mask, pc)` over the bean
   mask), iteration 2+ would resolve to iteration 1's OWN mask,
   not the bean mask. Same for the VEST loop (2 panels).
   The hook in `fill_polygon_masked` therefore sits at the very
   TOP of the function, above the internal `polygon_mask` call,
   so at hook time last-mask is still the caller's mask for the
   FIRST call — but that alone does not save iterations 2+.
2. **Bean shade/highlight after any pattern.** In
   paint_toon_full the shade and highlight
   `fill_ellipse_masked(.., &mask)` run AFTER the pattern block.
   With pattern = stripes / vest / apron, last-mask at that point
   is the last pattern panel's own mask, not the bean mask. With
   pattern = plain, no intervening mask prim exists and naive
   threading works — so the bug is pattern-dependent, the worst
   kind for a parity harness to inherit silently.

Fix landed (deviation from the brief's literal mechanism,
flagged): the recorder keeps BOTH last-mask AND a content
registry — `(FNV-1a hash of the mask bitmap, length) -> mask id`.
`resolve_mask` (used by both masked-fill shims) checks the
registry first; on a miss it falls back to last-mask AND
registers that resolution. Correctness over the full call graph:

- Every mask bitmap's FIRST masked use happens at a point where
  last-mask IS its producer (case analysis above: bean shade is
  the first bean-mask use when plain; stripes/vest/apron panel 1
  is the first bean-mask use when patterned; every mouth mask's
  only use is its first). So the fallback resolution is always
  correct at registration time.
- Every LATER use of the same bitmap hits the registry, immune
  to whatever mask prims were recorded in between.
- Two DIFFERENT bitmaps with identical content resolving to one
  id is harmless: replaying either mask prim reproduces the same
  bitmap. A 64-bit hash collision between different contents is
  the only failure mode (length is also keyed).
- `fill_polygon_masked`'s internal own-mask is never registered
  (it is indexed directly, never passed to a masked fill), so it
  cannot shadow a real entry; it remains in the stream as an
  unreferenced `PolygonMask` prim, which the executor may compute
  or skip. (Semantically the own mask is redundant for replay
  anyway: `FillPolygonMasked` replay = polygon fill of `pts`
  intersected with `mask`, binary centre-sampled, matching the
  CPU's `mask[k] && own[k]` centre test with `own` = inside-`pts`.)
- Unresolvable case (masked fill whose mask was created before
  the window opened): recorded with mask = `u32::MAX`
  (`gputrace::NO_MASK`) so the executor fails loudly. Unreachable
  in the current call graph — every masked fill's mask is created
  inside the same paint the window wraps.

A unit test in gputrace.rs (`mask_resolution_survives_interleaved_masks`)
replays the stripes interleaving and asserts both masked fills
and the following masked ellipse resolve to the bean id.

MaskedBlend's mask (the laugh teeth blit, §4): at both call sites
the sequence inside the Laugh branch is — `fill_polygon(mouth
pts)`; `msk`/`m = polygon_mask(pts)` (records the mouth
PolygonMask, sets last-mask); tongue
`fill_ellipse_masked(.., &msk)` (records a FillEllipseMasked and
creates NO mask prim); teeth bbox + loop. Verified by reading
both branches end to end: nothing between the `polygon_mask`
call and the teeth loop records a mask, so at the loop
last-mask IS the mouth mask — and the content registry holds the
same id, registered when the tongue fill resolved `msk`. The
hook resolves through `resolve_mask`, so both routes agree by
construction; if a future edit inserts a mask-producing call
into that gap, the registry route stays correct and the
last-mask coincidence is not load-bearing.

## 4. What is NOT in the stream (executor seat: read this)

The hooks record `Painter`-METHOD primitives. Three paint paths
bypass methods and write pixels through `blend` / `blend_cov`
directly; they produce NO prims and a replay of the list alone
will differ exactly there:

1. The head even-odd shade loop in `paint_toon` AND in
   `paint_toon_full` (inside-head-ellipse minus offset-circle,
   coverage form, open-coded over the bbox).
2. The laugh-mouth teeth blit in both painters (white fill over
   the mouth-mask region, open-coded).
3. `paint_cloud`'s soft discs and bottom-strip/tint pass,
   `paint_sky` / `paint_blob` (raw pixel writers; they use no
   `Painter` methods at all — a window around them records an
   empty list with the right canvas size only if the driver sets
   it; no current driver windows them).

Also: `Painter::blend_cov` AA quantisation, cap fans, and the
coverage model live INSIDE the recorded primitives' semantics —
the list records geometry + colour, and byte parity requires the
executor to reproduce the CPU coverage/blend arithmetic per
primitive class (that is Phase 0's per-class feasibility
question, not something the trace can settle).

## 5. Serialization and the round-trip argument

Format (`talerender-displaylist`, version 1): a JSON object
`{format, version, canvas_w, canvas_h, plates[], prims[]}`. Each
prim is an object with a `t` tag and its fields; points are
`[[f, f], ...]`, colours are `[r, g, b, a]` integer arrays, mask
/ plate references are integer prim/plate indices, and plate
pixels are standard base64 in `px_b64`.

Every f32 is serialized as `{"b": <u32>}` — its raw
`f32::to_bits()` as a decimal integer. Choice stated per the
brief: **bits form, not Display form.** Argument:

- `f32::to_bits` / `f32::from_bits` are exact inverses over ALL
  2^32 bit patterns: NaN payloads, ±inf, -0.0, and subnormals
  round-trip bit-identically.
- The u32 travels as a JSON integer: the writer emits its decimal
  form exactly; the reader (`scene::parse_json`) parses numbers
  as f64, and every u32 is < 2^53, hence exactly representable,
  so the `as u32` cast back is exact.
- Display form was rejected: Rust's `Display` for f32 DOES
  round-trip finite values (shortest-representation guarantee),
  but it renders NaN / ±inf as `NaN` / `inf` / `-inf`, which are
  not JSON numbers and which this crate's parser cannot read —
  the format would be non-total over the values a painter can
  legitimately carry (e.g. a degenerate geometry computation
  producing an infinite radius). Bits form has no such hole.

A unit test (`json_round_trip_is_bit_exact`) round-trips a list
containing a NaN payload pattern, -0.0, +inf, and the smallest
subnormal, and asserts the re-serialized text is identical
(serialization is a pure function of the bits, so text equality
is bit equality everywhere).

## 6. Feature-off invariance argument

- Feature OFF (default): `gputrace` / `gpupaint` modules are not
  declared and not compiled; every hook line is cfg-removed
  before type checking; `Cargo.toml`'s `[features]` table adds
  no dependency and changes no existing entry; `--gpu-paint`
  reaches the argument loop's unknown-argument error (exit 2),
  exactly today's behaviour for that flag. The compiled binary
  is the production binary by construction, not by inspection.
- Feature ON, not recording: each hook is one call into a shim
  that performs a single thread-local `RefCell` borrow check and
  returns (`push_plate` clones nothing when inactive; the mask
  hash is computed only inside a window). No painter state,
  argument, or control flow is touched: hooks read only the
  arguments the method already received, before the method body
  runs, and write nothing outside the recorder. Battery cards in
  a feature-on build additionally require the armed flag before
  any window opens (§2), so `--scene stroke-battery` output —
  files and stdout — is unchanged.
- The only production-file edits are the cfg-gated hook lines and
  comments listed in §7. No signature, ordering, or arithmetic
  anywhere was changed.

## 7. Edit inventory (every production-file touch)

- `Cargo.toml`: `[features] gpupaint = []` added above
  `[dependencies]`.
- `src/main.rs`: (a) `pub(crate) mod gpupaint;` +
  `pub(crate) mod gputrace;`, both cfg-gated, after the existing
  module list; (b) cfg-gated `--gpu-paint` dispatch before the
  argument loop; (c) one cfg-gated extra `--help` line.
- `src/scene.rs`: one cfg-gated hook line at the top of each of
  `fill_ellipse`, `stroke_polyline`, `fill_polygon`,
  `polygon_mask`, `ellipse_mask`, `fill_ellipse_masked`,
  `fill_rect`; plus two block-level hooks in `paint_toon` (the
  form-1 `HeadShade` after its bbox line, the `MaskedBlend`
  after the teeth bbox line). (`stroke_ellipse` / `stroke_quad`
  untouched — §2.)
- `src/toonfull.rs`: hook at the top of `fill_polygon_masked`
  (with the ordering comment, §3) and at the top of
  `composite_plate`; plus two block-level hooks in
  `paint_toon_full` (the form-0 `HeadShade` after its bbox line,
  the `MaskedBlend` after the teeth bbox line).
- `src/battery.rs`: three cfg-gated `begin_card` / `finish_card`
  pairs around the prims, toon, and face card paints. (Extra
  file beyond the brief's list — the per-card windows can only
  exist where the card boundaries are; flagged for the
  coordinator.)
- New: `src/gputrace.rs`, `src/gpupaint.rs`, this doc.
- No commits made; CI is the build verdict.
