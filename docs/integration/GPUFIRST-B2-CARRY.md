# GPU-FIRST TALERENDER — BATCH 2, SEAT CARRY: DESIGN-CARRY PLUMBING + INPUT-SURFACE AUDIT

**Branch:** `gpufirst-b2-carry` on `gpufirst-phase0` @ f4b358d.
**Scope:** plumb the FIELDS (struct + loader + film copy + cache
key) for the five spec channels whose painters are deferred ports
— wing (Spec 1), ears (§5b), hair (§5c), flip (§5d), bodyPitch
(Spec 3a) — so no format break stands between here and those
ports. **No paint code was added or changed**; the painter does
not read any new field. This seat's dispositions supersede
INVB §11's for these channels (INVB dispositioned them as
un-plumbed debt; they are now plumbed debt — fields, loader keys
and cache-key coverage landed, painters still deferred).
Brow (§5a, Seat BROW) and sweat (§5e, Seat SWEAT) are audited
below for input-surface expressibility only; their fields are
theirs.

**Hard gate (parity law):** absent/default states reproduce the
pre-plumbing paints byte-identically. Pinned BEFORE the fields
were added, at f4b358d, as full-frame FNV-1a hashes over
`paint_toon_full` output (probe log:
`~/workspace/talerender-evidence/gpufirst-b2/carry/baseline-probe.log`),
and asserted after plumbing by
`toonfull::carry_plumbing_tests::default_carry_fields_reproduce_pre_plumbing_hashes`:

| Case | Look / state | FNV-1a |
|---|---|---|
| A-default | plain look, `FullState::default()` | `cfb4d768fb09fdbf` |
| B-happy-walk | Happy, Walk, phase 1.1 | `05a781daa65cb22b` |
| C-hero-vest | cap/vest/scarf hero, Determined pose | `1ed010bf0e163e77` |
| D-kid-sit-sad | kid, Sit sit_drop 300, Sad + tears | `cec92a8d3ebab654` |
| E-tani-worried | bowler/glasses/mustache, Worried + sweat/talk/blink/tilt/squash/cam_pitch | `2cb164e9c2e44aa6` |

---

## 1. Input-surface audit — method

For each channel: which `Prim` kinds (`src/gputrace.rs`) will
express its painting when the port lands, whether the
DisplayList JSON format carries everything, and whether the
`gpupaint.wgsl` `Params` encoding (p0–p3 f32 slots, u0, col)
carries everything. The load-bearing architectural fact,
verified in source: **all painter point transforms are host-side
(CPU)**. `paint_toon_full`'s point maps (`px`, `hpt`) compose
lean + canvas-anchor (+ squash / headTilt) in Rust and the trace
hooks record *device-space* geometry; the GPU executor never
re-derives a transform. The single exception is
`CompositePlate`, whose inverse map is re-implemented in WGSL
(`main_plate`) and therefore receives its transform as params
(lean sin/cos + anchor). Every verdict below follows from that
split.

## 2. Per-channel verdicts

### Wing — Spec 1 — EXPRESSIBLE, no gap

- **Prims:** one panel per segment per side =
  `FillEllipse` (centre/radii/rot from the segment geometry) +
  `StrokeEllipse` (the ink ring; CPU `stroke_ellipse`, width 7
  near / 6 far); the near-side shade feather line and per-joint
  shade lines = `StrokePolyline` or `StrokeQuad` (widths 4 / 3,
  shade colours as plain `Rgba`). Far wing paints *before* the
  body prims and near wing after them — the recorder preserves
  execution order and the executor dispatches in list order, so
  the layering is free.
- **DisplayList JSON:** `FillEllipse` carries
  cx/cy/rx/ry/rot/col; `StrokeEllipse` adds `width`;
  `StrokePolyline`/`StrokeQuad` carry pts + width + col. All
  f32 values are bit-encoded (`{"b": u32}`) — exact. Segment
  count is just prim count (1, 2, 3, 5…), unbounded by the
  format. `wingCol` / shade colours are ordinary `Rgba` values.
- **WGSL Params:** ellipse entry `main_ellipse` takes
  p0 = (cx, cy, rx, ry), p1 = (sin rot, cos rot) host-computed;
  strokes go through `prep_stroke` (p0.x = half-width, fan
  params host-computed). Width and colour are per-prim. ✓
- **CompositePlate independence:** not needed and not used —
  the wing is prims, per the seat brief. ✓

### Ears — §5b — EXPRESSIBLE, no gap

- **Prims:** per side, outer triangle = `FillPolygon` (body
  colour) + `StrokePolyline` (closed ring, INK, width = the
  outline appendage width, default 7); inner triangle =
  `FillPolygon` (`#F7F1E3`, no stroke). Painted behind the head
  disc = earlier in list order than the head `FillEllipse`. ✓
- **JSON / WGSL:** `FillPolygon` {pts, col} → `main_fillpoly`
  (u0.x = point count); `StrokePolyline` {pts, width, col} →
  `main_stroke`. The ear rotation (s·0.16 + ea about the anchor)
  is host-side point math — the recorded triangle points are
  already rotated. ✓

### Hair — §5c — EXPRESSIBLE, no gap

- **Prims:** strands are two-pass quadratic strokes =
  `StrokeQuad` ×2 per strand (pass 1 INK width 17, pass 2
  `hairCol` width 11). The CPU `stroke_quad` and the trace's
  `StrokeQuad` prim {p0, p1, p2, width, col} match exactly; GPU
  prep flattens the quad host-side (24 segments) into
  `main_stroke`, the same flattening the CPU painter does.
  Strand count = `hairSegs.len()` = prim count. ✓
- **JSON / WGSL:** width and colour per prim, points
  bit-encoded. ✓ The TS canvas anchor does **not** envelope
  hair (§5c) — no host-side obligation on this channel.

### Flip — §5d — EXPRESSIBLE for prims; ONE named gap (plates)

- **Prims:** none new. Flip is a stage-space reflection
  (x → 512 − x) in the painter's point maps, composed **after**
  the anchor translation and **before** lean (INVB §11's
  binding design note; Spec §5d). Because the maps are
  host-side, recorded prim geometry arrives already mirrored
  and the executor needs nothing. The same reflection in
  `canvas_anchor` is a CPU port-time task, not a format task.
- **GAP — `B3-CARRY-1` (shared with bodyPitch, below):**
  `CompositePlate` is the one prim whose WGSL entry
  re-implements a transform: `main_plate` inverse-maps through
  lean + anchor only (prim fields: plate, hand, lean_rad,
  anchor; prep p1 = (sin lean, cos lean, anchor.x, anchor.y)).
  A flipped figure's held plate would be sampled un-mirrored.
  The CPU `composite_plate` has the same lean+anchor-only
  inverse map, so this is a *port-time* gap on both sides, not
  a GPU-only divergence — and nothing is mis-rendered today
  because no painter emits flip. **Not fixed here:** the fix
  needs a `gpupaint.wgsl` change (outside this seat's scope)
  plus executor prep changes. The fix is small and the room is
  already there: plate prep leaves **p3 entirely unused** and
  **p2.w = 0.0 / u0.z = u0.w = 0** spare, and the DisplayList
  JSON for `composite_plate` can grow an additive field
  without breaking the bits-exact round-trip. Recorded as
  Batch 3 input `B3-CARRY-1`.

### bodyPitch — Spec 3a — EXPRESSIBLE for prims; same named gap

- **Prims:** none new. Pitch is a host-side rotation of the
  upper-assembly points about the hip pivot (bx, hipY); legs
  are recorded un-pitched because the CPU painter paints them
  outside the pitch transform. The head counter-tilt
  (headTilt − bodyPitch·bodyPitchCounter) only changes the
  `hpt` inputs. At pitch 0 the TS seam emits no transform; the
  Rust port must keep that byte-identity property (Spec 3a
  acceptance), which is a painter property, not a format one.
- **Gap:** `B3-CARRY-1` again — TS held-prop callbacks receive
  figure-local coordinates *inside the pitched frame*;
  `composite_plate`'s inverse map (CPU and WGSL alike) knows
  lean + anchor only, so a pitched hold's plate sampling needs
  the pitch (sin/cos + hip pivot) added to the same param
  block, in the spare slots named above. Also a CPU port-time
  obligation: `canvas_anchor` must model the pitch (Spec 3a).
  Recorded as Batch 3 input `B3-CARRY-1` (one item covering
  flip + pitch plate params).

### Brow — §5a (Seat BROW's port) — EXPRESSIBLE, no gap

- **Prims:** the existing brow stroke is a `StrokeQuad`
  (width 12, INK) in both painters. browLift/browTilt only move
  its control points host-side (`lift += browLift`,
  `tilt += browTilt·s` after the per-side overrides).
- **Precision check (the §5a trap):** the fixture's 0.5-unit
  steps must distinguish — the whole chain is exact: state f32
  → painter f32 → prim points bit-encoded in JSON → WGSL f32.
  No quantisation exists anywhere in the surface. ✓

### Sweat — §5e (Seat SWEAT's port) — EXPRESSIBLE, no gap

- **Prims:** the drop is `FillPolygon` (fill `#8FD3F4`) +
  `StrokePolyline` (ink ring, width 4) — the shapes the CPU
  painter already emits today.
- **Per-prim alpha — verified at every layer** (this was
  INVB §11's one recorded GPU design constraint for sweat):
  1. `Prim` colours are `Rgba = (u8, u8, u8, u8)` — alpha is a
     per-prim field, not a global.
  2. DisplayList JSON writes colour as `[r, g, b, a]` (all four
     channels) and the reader validates arity 4 / ≤ 255.
  3. `gpupaint.rs` `Params::col()` copies all four channels
     into the uniform `col: vec4<u32>` unchanged.
  4. WGSL `blend_at` uses `src.w / 255` as source alpha and
     `blend_cov_at` computes `a = round(src.w · cov)` — exactly
     the CPU `Painter::blend` / `blend_cov` composition
     (source alpha × coverage), including the per-prim
     round-to-u8.
  So gradation is expressed by putting
  `alpha = round(min(1, k) · 255)` in the drop prims' colour —
  no format change. Precision note: alpha is u8-quantised
  (255 steps); the fixture's distinguishing values 0.5 / 0.99 /
  1 map to 128 / 252 / 255 — pairwise distinct, and 1 ≡ 1.5
  saturation is preserved by the `min(1, k)` clamp on the CPU
  side before encoding. ✓

## 3. Format gaps — summary

| ID | Gap | Severity | Disposition |
|---|---|---|---|
| `B3-CARRY-1` | `CompositePlate` (DisplayList prim + `gpupaint.wgsl` `main_plate` + CPU `composite_plate`) inverse-maps lean + anchor only; flip (§5d) and bodyPitch (Spec 3a) transforms cannot reach plate sampling. Spare param room already exists (plate p3 unused; p2.w, u0.z, u0.w zero). | Port-time only — no painter emits flip/pitch today, CPU and GPU are equally affected, nothing mis-renders. | **Named Batch 3 input.** Not fixed here: the fix requires a WGSL change (outside this seat) and must land with the flip/pitch painter ports so CPU/GPU change together. |

No other gap found. In particular: per-prim Rgba alpha
(sweat), per-prim stroke width/colour (hair strands, wing
shade lines, ear ring, brow), rotated ellipses (wing panels),
and arbitrary prim counts (wing segments, hair strands) are
all carried by the existing format and param encoding.

## 4. Anchor obligations recorded for the painter ports

`canvas_anchor` deliberately gained **no** envelope points in
this plumbing — the anchor models painted geometry only, and
none of the new channels paint yet. Port-time obligations:

- **Wing port (Spec 1):** add both wing chains' points (with
  half-width + ink radii) as upper-assembly points, as the TS
  anchor does (toon.ts 722–726).
- **Ear port (§5b):** add both ear triangles' points (tips
  included), as the TS anchor does (toon.ts 683–693) — a tilted
  ear must not paint outside the cleared envelope.
- **bodyPitch port (Spec 3a):** model the pitch (upper-assembly
  points rotate about (bx, hipY); legs never pitched).
- **Hair:** no obligation — the TS anchor does not model hair
  strands; mirror that, invent nothing (§5c).
- **Flip (§5d):** the anchor applies the same final x-mirror
  (px = 512 − px) at port time.

## 5. Plumbed-field list (as landed — the batch record)

### FullLook (`src/toonfull.rs`)

| Field | Type | Default | Loader key (`src/scenefile.rs`) | Key coverage |
|---|---|---|---|---|
| `wing` | `bool` | `false` | `"wing"` (bool) | n/a — look field (named by registry index in `state_key`; scenefile paints uncached) |
| `wing_col` | `Option<String>` | `None` | `"wingCol"` | n/a — look field |
| `wing_lens` | `Vec<f32>` | `[]` | `"wingLens"` (number array) | n/a — look field |
| `ears` | `bool` | `false` | `"ears"` (string `"fox"` → true; JSON `true` also accepted) | n/a — look field |
| `hair` | `bool` | `false` | `"hair"` (bool) | n/a — look field (`hair_col` pre-exists for `"hairCol"`; not duplicated) |

### FullState (`src/toonfull.rs`; `Default` impl carries the defaults)

| Field | Type | Default | Loader key (`src/scenefile.rs`) | Key coverage (`src/film.rs` `state_key`) |
|---|---|---|---|---|
| `wing_segs` | `Vec<f32>` | `[]` | `"wingSegs"` (number array) | length + element bits |
| `wing_a` | `f32` | `0.0` | `"wingA"` | exact f32 bits |
| `ear_a` | `f32` | `0.0` | `"earA"` | exact f32 bits |
| `ear_al` | `Option<f32>` | `None` | `"earAL"` | presence bit + bits |
| `ear_ar` | `Option<f32>` | `None` | `"earAR"` | presence bit + bits |
| `hair_segs` | `Vec<f32>` | `[]` | `"hairSegs"` (number array) | length + element bits |
| `flip` | `bool` | `false` | `"flip"` (bool) | bool (existing pattern) |
| `body_pitch` | `f32` | `0.0` | `"bodyPitch"` | exact f32 bits |
| `body_pitch_counter` | `f32` | **`1.0`** (TS `?? 1`) | `"bodyPitchCounter"` | exact f32 bits |

Film path: `src/film.rs` `resolve_chars` copies all nine
FullState fields from the staged `CharDraw.state`; the telemetry
mirror (`src/filmtelemetry.rs`) parses the same scene keys with
the same defaults. Loader unit surface:
`scenefile::carry_look_fields` / `scenefile::carry_state_fields`
(the character branch builds through them; tests call them
directly). Unknown look/state keys keep the loader's standing
silently-ignored behaviour; known keys with wrong types error
like every other typed field.

### Tests landed

- `toonfull::carry_plumbing_tests` — the five pinned
  pre-plumbing hashes (§0) reproduced at defaults;
  TS resolved defaults asserted (incl. counter = 1.0).
- `film::state_key_tests::state_key_covers_every_carry_field` —
  per-field key distinctness (Vec by length and by element
  bits; Option by presence and by value).
- `scenefile` tests — `carry_look_keys_parse_into_fields`,
  `carry_state_keys_parse_into_fields` (every key, defaults,
  wrong-type errors, ears `"fox"`/true/other), and
  `carry_keys_load_end_to_end_and_unknown_keys_still_ignored`
  (full v2 scene file through `load()`).
