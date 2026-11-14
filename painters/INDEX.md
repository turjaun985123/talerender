# Painter integration pack — INDEX (seat W39)

Every staging module in `painters/`, what the lead pastes, and where.
Verification: [TEST-RUN.md](TEST-RUN.md) — all 41 painter tests pass
against the repo's real `src/scene.rs` / `src/math.rs`.

Two integration shapes exist in the pack, matching how the seats
wrote them:

- **Paste into `src/scene.rs`** beside `paint_grass` (the module's
  `use crate::scene::…` line, where present, is deleted — the items
  are scene.rs's own there), plus one dispatch arm in
  `src/scenefile.rs::resolve_texture` (line 398). This is the shape
  planks / wallpaper / felt / parcel / picture / sign were written
  for (picture/sign's own docs also offer a `src/painters/` module
  form; the paste form is listed here, the arm differs only in the
  path prefix).
- **New module** (calendar only — it is not a texture generator):
  copy to `src/calendar.rs`, add `mod calendar;` in `src/main.rs`
  (beside lines 22–31), call from `src/film.rs`.

All texture arms return `(Vec<u8>, u32, u32)` — the exact type
`resolve_texture` already returns — and consume only fields the
film data already declares (checked against
`scenes/sets/bookshop.scene.json` / `scenes/film/s10.scene.json`:
54 `planks`, 7 `wallpaper`, 4 `felt`, 2 `parcel`, 2 `picture`,
2 `sign` generator entries). `repeat` / `rotation` on those entries
are **not** painter inputs in any module (texture-clone properties
in the source too); the loader/material layer owns them, as today.

---

## 1. planks — `painters/planks.rs` → `src/scene.rs`

Public function:

```rust
pub(crate) fn paint_planks(base: &str) -> (Vec<u8>, u32, u32)   // 512×512
```

(Private helpers in the fragment: `planks_shade`,
`planks_shade_rgba`, `planks_grain_pts`. The fragment has **no**
`use` line — it references scene.rs's `Painter` / `Rgba` /
`hex_rgb` / `rnd` unqualified, so it pastes as-is.)

Dispatch arm (beside `"grass"`):

```rust
"planks" => scene::paint_planks(str_field(spec, "base", ctx)?),
```

Tests: **7** (`planks_tests`: size/opacity, determinism,
`shade_truth_table`, seam band, row fills, knot centres, mean
colour) — all pass.

Known deltas (full list: [planks.md](planks.md)): Skia stroke
rasterisation + butt-vs-round caps on the grain curves (edge
pixels / first+last columns only); knot & fractional-rect rims
~1 LSB; `rnd()` ≤1-ulp libm/V8 sin divergence; globalAlpha 0.35
folded to alpha byte 89 (≤0.1 LSB/stroke px).

## 2. wallpaper — `painters/wallpaper.rs` → `src/scene.rs`

Public functions:

```rust
pub(crate) fn paint_wallpaper(base: &str, motif: &str) -> (Vec<u8>, u32, u32)  // 256×256
pub(crate) fn shade_hex(hex: &str, k: f32) -> (u8, u8, u8)   // THREE.Color shade — see §8
```

(Delete the file's `use crate::scene::{flatten_cubic, hex_lin,
linear_to_srgb, rgba_of, Painter};` line on paste-in.)

Dispatch arm:

```rust
"wallpaper" => scene::paint_wallpaper(
    str_field(spec, "base", ctx)?,
    str_field(spec, "motif", ctx)?,
),
```

Tests: **6** (size/opacity, determinism, wall colours, alpha-dot
composite, ceiling variant + linear shade, source defaults) —
all pass.

Known deltas (full list: [wallpaper.md](wallpaper.md)): the two
wing arcs of one canvas path are painted as two discs (Skia clamps
the 7-rad sweep to a full circle — the one structural
interpretation; film.rs's local port reads it the same way); dots'
alpha 0.5 → byte 128; house SDF coverage vs Skia on shape edges.
Also: `src/film.rs:524` carries a local duplicate `paint_wallpaper`
whose petal flattening is coarser (13 hand-sampled points vs
`flatten_cubic` 24/curve) — everything else identical by design;
reconciling the duplicate is optional and separate from this arm.

## 3. felt — `painters/felt.rs` → `src/scene.rs`

Public function:

```rust
pub(crate) fn paint_felt(base: &str) -> (Vec<u8>, u32, u32)   // 256×256
```

(Private: `shade_three` + `srgb_to_linear_f64` /
`linear_to_srgb_f64` — see §8. On paste-in, drop the
`use crate::scene::{hex_rgb, rnd, Painter};` line and unqualify
the two full-path references: `crate::scene::rgba_of` in
`paint_felt` and `crate::scene::shade_rgb` in one test.)

Dispatch arm:

```rust
"felt" => scene::paint_felt(str_field(spec, "base", ctx)?),
```

Tests: **5** (size, determinism, fully opaque, three-shade pins,
pinned base-survivor count 50047 ±6) — all pass.

Known deltas (full list: [felt-parcel.md](felt-parcel.md)):
canvas AA vs Painter AA on speckle edges; sin()/pow() last-ulp
(measured: ≤4 px on the pinned count, ~0.05% of bytes);
premultiplied-vs-straight alpha on AA edges only. Note
`src/film.rs:572`'s local `paint_felt` shades with the byte
multiply `shade_rgb` — it differs from this module (and the
source) on every speckle; that is the §8 item, not a felt defect.

## 4. parcel — `painters/parcel.rs` → `src/scene.rs`

Public function:

```rust
pub(crate) fn paint_parcel(label: bool) -> (Vec<u8>, u32, u32)   // 256×256
```

(Drop the `use crate::scene::{hex_rgb, rnd, Painter};` line on
paste-in.)

Dispatch arm (`truthy` already exists in scenefile.rs, line 430;
the data carries `"label": false` explicitly):

```rust
"parcel" => scene::paint_parcel(truthy(spec.get("label"))),
```

Tests: **5** (size, determinism, tape count exactly 7936, no
label colours at `label=false`, label counts 5820/480 at
`label=true`) — all pass, all exact (integer-aligned geometry).

Known deltas (full list: [felt-parcel.md](felt-parcel.md)): the
generic painter-AA class only — no colour management is involved
(all fills are literal hex). `src/film.rs:593`'s local
`paint_parcel()` is op-identical to this module at `label=false`.

## 5. picture — `painters/picture.rs` → `src/scene.rs`

Public function and constants:

```rust
pub(crate) fn paint_picture() -> (Vec<u8>, u32, u32)   // 256×200, no parameters
pub(crate) const PICTURE_W: u32 = 256;
pub(crate) const PICTURE_H: u32 = 200;
pub(crate) const PICTURE_PAPER: &str = "#E9DCC0";
pub(crate) const PICTURE_INK: &str = "#3A2A22";
pub(crate) const PICTURE_SUN: &str = "#C9603F";
pub(crate) const PICTURE_FRAME: &str = "#C9A13B";  // mesh material colour, NOT texture
```

(Drop the `use crate::scene::{flatten_quad, rgba_of, Painter};`
line on paste-in. Module-form alternative from
[picture-sign.md](picture-sign.md): `src/painters/picture.rs` +
`"picture" => painters::picture::paint_picture(),`.)

Dispatch arm (the spec entry carries no fields):

```rust
"picture" => scene::paint_picture(),
```

Tests: **4** (size/opacity, determinism, pinned pixels,
sun-disc area band 1250..1600) — all pass; the band is the
module's own pre-CI estimate and passed as-is in the harness —
W24's doc suggests tightening it to an exact count after a CI
run; the harness measured it inside the band, no change needed
for wiring.

Known deltas (full list: [picture-sign.md](picture-sign.md)):
Skia analytic AA vs house SDF coverage (edges only);
`flatten_quad` n = 20 vs Skia's adaptive flattening (AA band
only). The gold frame is geometry (`PICTURE_FRAME`), never part
of this texture.

## 6. sign — `painters/sign.rs` → `src/scene.rs`

Public functions and constants:

```rust
pub(crate) fn paint_sign(bg: &str, fg: &str, w_units: f32, h_units: f32) -> (Vec<u8>, u32, u32)
pub(crate) fn paint_sign_px(bg: &str, fg: &str, cw: u32, ch: u32) -> (Vec<u8>, u32, u32)
pub(crate) fn sign_canvas_size(w_units: f32, h_units: f32) -> (u32, u32)
pub(crate) fn sign_text_anchor(cw: u32, ch: u32) -> (f32, f32)
pub(crate) fn sign_edge_rgba(bg: &str) -> Rgba                       // shade(bg, 0.7), §8
pub(crate) fn sign_with_text(board: (Vec<u8>, u32, u32), strip: &[u8]) -> (Vec<u8>, u32, u32)
pub(crate) const SIGN_PX_PER_UNIT: f32 = 200.0;
pub(crate) const SIGN_FONT_START_FRAC: f32 = 0.5;
pub(crate) const SIGN_FIT_FRACTION: f32 = 0.86;
pub(crate) const SIGN_FONT_STEP: f32 = 2.0;
pub(crate) const SIGN_TEXT_DY: f32 = 3.0;
pub(crate) const TANIGUCHI_BOOKS_PIL: (&str, f32, f32) = ("TANIGUCHI BOOKS", 40.0, 335.36);
```

(Drop the `use crate::scene::{hex_lin, linear_to_srgb, rgba_of,
Painter, Rgba};` line on paste-in. Module-form alternative:
`src/painters/sign.rs`, paths prefixed `painters::sign::`.)

Dispatch arm — the registry paints board + border; the spec's
`canvas: [440, 80]` is authoritative (it equals the 2.2×0.4-unit
mesh under the 200 px/unit rule, `sign_canvas_size`):

```rust
"sign" => {
    let c = req(spec, "canvas", ctx)?
        .as_arr()
        .ok_or_else(|| err(ctx, "'canvas' must be [w, h]"))?;
    if c.len() != 2 {
        return Err(err(ctx, "'canvas' must be [w, h]"));
    }
    scene::paint_sign_px(
        str_field(spec, "bg", ctx)?,
        str_field(spec, "fg", ctx)?,
        num(&c[0], ctx)? as u32,
        num(&c[1], ctx)? as u32,
    )
}
```

(`req` / `num` / `err` are scenefile.rs's existing helpers, used
the same way by the `dof-chart-pattern` arm.) Text is **not**
part of this arm by design: if/when a spec names a glyph-strip
file for its `text`, composite it after the arm with
`sign_with_text(board, strip_bytes)` — the strip is the PIL step
(`film/gen_assets.py`, 700 Caladea at `sign_text_anchor`,
fs = 40 per `TANIGUCHI_BOOKS_PIL`). Without a strip the texture
is the board exactly as the canvas looks before `fillText`.

Tests: **7** (canvas-size rule, edge shade pin, text anchor,
board pixels, determinism, PIL no-shrink measurement,
strip compositing) — all pass.

Known deltas (full list: [picture-sign.md](picture-sign.md)):
**B9 stands** — browser (Skia) Caladea metrics are unreproducible
from source; fs = 40 is PIL-measured (335.36 px vs the 378.4 px
limit, so no shrink fires — the 43 px headroom makes a browser
shrink unlikely, not impossible); the PIL film asset's border
ink sits inside the box where canvas `strokeRect` centres it
(native follows canvas — up to 3 px along the border vs
`film/assets/sign.png`); AA/flattening classes as picture.

## 7. calendar — `painters/calendar.rs` → `src/calendar.rs` (film path, NOT resolve_texture)

Public API (the file is std-only, `#![allow(dead_code)]`, plain
`pub`):

```rust
pub struct CalState { pub year_idx: usize, pub year: u32, pub sy: f32, pub dark: f32, pub alpha: f32 }
pub fn state_at(t: f64) -> Option<CalState>
pub fn render_overlay(t: f64) -> Option<Vec<u8>>                       // standalone 1080×1920 RGBA
pub fn composite_over(px: &mut [u8], w: u32, h: u32, t: f64) -> bool   // film-path entry
pub fn text_width(s: &str, size: f32) -> f32
pub const FRAME_W: u32 = 1080;
pub const FRAME_H: u32 = 1920;
pub const T0_FILM: f64 = 30.55;
pub const LIFE: f64 = 4.4;
pub const YEARS: [u32; 4] = [1945, 1955, 1965, 1974];
```

Wiring (no dispatch arm — this is the S07 overlay, data spec
`scenes/film/s07-calendar.json`):

1. Copy the file to `src/calendar.rs`; add `mod calendar;` in
   `src/main.rs`. It compiles exactly as staged (verified
   standalone and in the combined harness).
2. In `src/film.rs`, at the per-frame composite point that calls
   `overlay_calendar` (film.rs:3425; the state comes from
   `calendar_state(lt - 0.16)`, film.rs:1639/1700), call instead
   — or in parallel, behind whichever switch the lead chooses:

```rust
crate::calendar::composite_over(&mut px, width, height, t_film_seconds);
```

   `composite_over` takes the **film-clock** time and runs
   `state_at` internally (the module's `T0_FILM` already encodes
   calT0 = shot start + 0.16). Field mapping if the lead keeps
   `CalendarState`: film.rs's `year` is the 0..3 index =
   `CalState::year_idx` here; the numeral is `CalState::year`.
   Keep the draw order: inside the shot draw, before fade and
   caption (main.ts's order — the same point the plate path
   uses).

Tests: **7** (alpha-law boundaries, years at t = 31.0/31.5/32.5/
33.5, sy floor at flip midpoint, exact card/header/rule/ink
pixel colours, mid-flip shading ≈ (202, 194, 176), text fit,
off-screen) — all pass.

Known deltas (full list: [calendar.md](calendar.md)): the
embedded glyphs are geometric approximations of Inter
ExtraBold/Black, **not** Inter outlines (advances hand-set;
digits 0.60 em vs Inter's 0.6020 — year width 283.2 vs ≈284.2
local units); text AA is the 4×4 inside-test (softer than
Skia); pixel-centre sampling vs film.rs's pixel-index plate
sampling (≤0.5 px); flip-frame AA ramp narrows by sy where the
plate path bilinear-softens; gen_assets.py paints the rule with
INK `#1F1C33` where drawCalendar/the data spec use `#8B8378` —
this module follows the source. The PIL plates remain the
reference rendering.

---

## 8. The shared linear-shade helper — dedupe recommendation

THREE.Color `shade(hex, k)` (world.ts:35: hex → **linear**,
multiply, → sRGB hex) was implemented **four times** in the
pack, because at pack time `src/scene.rs`'s existing
`shade_rgb` was a different function (canvas `shadeOf`: sRGB
**byte** multiply — a port of toon.ts) and `shade_lin`
multiplied the bytes first, then converted:

**[W52 update — post-`c00897d`: that premise is gone.** The
lead root-fixed `shade_rgb` in `c00897d`: it now performs
the THREE.Color linear-space multiply itself (f64 body in
`src/scene.rs`), and `shade_lin` is defined over it. The
four implementations below therefore no longer exist
*because the registry helper is wrong* — they exist because
each module was staged self-contained. The dedupe
recommendation stands unchanged, with one refinement: there
are now two candidate canonical bodies — felt's
`shade_three` (three r170's literal constants, strict `<`
at the boundary) and the registry's own `shade_rgb`
(spec-exact 1/2.4 exponent, `<=` boundary). The pinned
witnesses under the table agree under both; the ≤1 LSB
unpinned-boundary caveat is exactly the difference between
those constants, so the choice should be made deliberately,
once.]

| implementation | where | form | pinned witnesses |
|---|---|---|---|
| `planks_shade(hex, k: f64)` | planks.rs (private) | f64, three r170 literal constants throughout (incl. exponent 0.41666), Math.round quantise | `#3A2A2A`@0.7 → (48,34,34); `#3A2E22`@1.25 → (65,52,39); `#A8693E`@{0.45,0.5,0.7,1.0} |
| `shade_three(hex, k: f64)` | felt.rs (private, + private `srgb_to_linear_f64`/`linear_to_srgb_f64`) | f64, same three literals; strict `<` at the 0.04045 boundary (three's form) | `#1F5A45`@0.85 → (28,83,64); @1.0 exact round-trip |
| `shade_hex(hex, k: f32)` | wallpaper.rs (pub(crate)) | f32, built on scene's `hex_lin` + `linear_to_srgb` (spec-exact 1/2.4 exponent, `<=` boundary) | `#3A2E22`@1.25 → (65,52,39) |
| `sign_edge_rgba(bg)` | sign.rs (pub(crate)) | f32 via `hex_lin`/`linear_to_srgb`, k fixed at 0.7, returns `Rgba` | `#3A2A2A`@0.7 → (48,34,34,255) |

**The pinned values agree everywhere the corpus pins them:**
`#302222` from planks (f64) and sign (f32); `#413427` from
planks (f64) and wallpaper (f32). The f32/f64 and
0.41666-vs-1/2.4 differences are sub-byte-step on every pinned
input; they can disagree by ≤1 LSB only on unpinned boundary
values (planks.md item 6 names this).

**Recommendation — one shared helper, in `src/scene.rs`:**

```rust
/// world.ts shade(): THREE.Color linear-space multiply
/// (three r170 constants, f64 — the planks/felt form).
pub(crate) fn shade_three(hex: &str, k: f64) -> (u8, u8, u8) { /* felt.rs's body */ }
```

Take **felt.rs's `shade_three`** as the canonical body (it is
the most literal transcription: f64, three's own constants,
strict `<`, round-half-away-from-zero = JS Math.round on the
clamped domain; planks' `planks_shade` is arithmetically the
same form and can be deleted in its favour after checking its
truth-table test still pins). Then:

- planks.rs: delete `planks_shade`/`planks_shade_rgba`; call
  `shade_three` (wrap in an `Rgba` tuple where needed).
- wallpaper.rs: `shade_hex` becomes a thin f32 wrapper or is
  deleted — its only in-module use is the pinned test; the
  painter proper never shades (the ceiling motif arrives
  precomputed in the data).
- sign.rs: `sign_edge_rgba(bg)` = `shade_three(bg, 0.7)` +
  alpha 255.

Do the dedupe **with or before** the first of {planks, felt,
wallpaper} so the other two land on the helper directly; if it
is deferred, nothing breaks — each module is self-contained and
all four implementations pass their pins as staged (TEST-RUN.md).

Related, and **not** part of the dedupe: `paint_grass` and
film.rs's local painters use `shade_rgb` for world.ts `shade()`
calls (planks.md item 5, felt-parcel.md). Switching them to
`shade_three` changes already-measured A/B frames — an
integrator decision, flagged by W21/W23, deliberately not done
by any seat in this pack.

**[W52 update: that decision was taken at the root in
`c00897d`.** Rather than moving the callers to `shade_three`,
the lead redefined `shade_rgb` itself to the linear-space
arithmetic — so `paint_grass` and film.rs's painters carry
the corrected shades through the helper they already call.
Already-measured A/B frames did change, exactly as flagged
here; the re-baseline is declared in `c00897d`'s message,
and its golden consequences are recorded in
`docs/golden/NATIVE-GOLDEN.md` and `tools/golden/RUNNER.md`.
The dedupe above is now only about the pack's own four
implementations.]

## 9. Wiring order

All six texture arms are independent of each other (each adds
one match arm + one painter body; none touches an existing
generator, and `resolve_texture`'s fall-through error stays the
guard for anything unwired). Suggested order, by data pressure:

1. **§8 shade dedupe** (optional but cheapest now) — or skip and
   let each module carry its own, per §8's last paragraph.
2. **planks** — 54 data entries reference it; the largest gap.
3. **wallpaper, felt, parcel** — the rest of the bookshop set
   (7 / 4 / 2 entries); any order among themselves.
4. **picture, sign** — 2 entries each; sign's arm is the only
   one with a block body (canvas extraction).
5. **calendar** — fully independent of the texture registry
   (different file, different consumer); land it whenever, but
   note it *replaces a live path* (the PIL-plate overlay) rather
   than filling an error fall-through, so it wants the lead's
   A/B eye, not a mechanical paste.

After wiring, the `resolve_texture` unknown-generator error
should be unreachable from the shipped scene data: the only
other generator names in `scenes/` are the already-registered
ones (`grass`, `sky-gradient`, `cloud`, `blob-shadow`,
`onoda-card`, `dof-chart-pattern`) — plus `"solid"` entries
(s10.scene.json, fog-chart.scene.json), which are **not** in
this pack and not currently registered; flagging them here so
they aren't mistaken for a painter gap this pack was meant to
close.
