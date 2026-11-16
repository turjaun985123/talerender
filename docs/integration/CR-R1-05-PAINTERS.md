# R1-05 — Texture painters port (R-W5): siding, brick, dirt, sand, facade

Seat deliverable, Chrome-replacement REMEDIATION wave Batch 1.
NEW files only; nothing edited, nothing committed. All five
painters ported — **none of the five existed natively under any
name** (evidence in §2). The port's form follows the registry
precedent found in code (§1): one drop-in `.rs` per painter in
`painters/`, written to be pasted into `talerender/src/scene.rs`
beside `paint_grass` and registered by name in
`scenefile.rs::resolve_texture`, exactly as the bookshop family
was absorbed.

Deliverables in this directory:

| file | what |
|---|---|
| `painters/brick.rs` | `paint_brick(base)` + `brick_shade` + tests |
| `painters/siding.rs` | `paint_siding(base)` + gradient helper + tests |
| `painters/dirt.rs` | `paint_dirt(base, tracks)` + tests |
| `painters/sand.rs` | `paint_sand(base)` + tests |
| `painters/facade.rs` | `paint_facade(...)` + `facade_with_text` + PIL consts + tests |
| `harness/record_ts_ops.ts` | runs the skill's OWN world.ts painters (bun), records their canvas op streams |
| `harness/ts_reference.py` | rasterises the op stream (canvas centre-sample semantics) |
| `harness/port_mirror.py` | mirror of the Rust port's arithmetic (scene.rs Painter semantics) |
| `harness/compare.py` | driver: metrics, PNGs, PIL text measurements → `harness/out/report.json` |
| `harness/out/` | `ops.json` (the recorded op streams), `report.json`, `*_ref.png` / `*_port.png` |

---

## 1. Registry investigation — where native painters live, and the bookshop precedent

**The registry is code, in two places:**

1. **Painters**: `talerender/src/scene.rs` — CPU painters over the
   module's own rasteriser (`Painter`, scene.rs:472ff: `blend`,
   `blend_cov`, `fill_rect` via box SDF, `fill_ellipse` via
   `ellipse_sd`, `stroke_polyline`, `fill_polygon`, coverage from
   signed distance via `cov_area`'s 4×4 supersample). Every
   registry painter has the shape
   `pub(crate) fn paint_*(...) -> (Vec<u8>, u32, u32)` (RGBA bytes,
   row 0 = top), colours in as hex strings, all randomness from
   `scene::rnd` (the f64 sin-hash `frac(sin(i·127.1+311.7)·43758.5453)`,
   scene.rs:97 — the same function as world.ts:12).
   Pre-existing registry painters there: `paint_grass`,
   `paint_sky`, `paint_cloud`, `paint_blob`, `paint_toon`, and the
   bookshop wave pack (below). `film.rs:619ff` carries older local
   duplicates of some of these for the film path only — the
   *registry* (the one scene files resolve against) is scene.rs.
2. **Dispatch**: `talerender/src/scenefile.rs::resolve_texture`
   (scenefile.rs:410) — a match on the scene file's texture
   `generator` string: `"grass"`, `"planks"`, `"wallpaper"`,
   `"felt"`, `"parcel"`, `"picture"`, `"sign"`, `"solid"`,
   `"sky-gradient"`, `"cloud"`, `"blob-shadow"`, `"onoda-card"`,
   `"dof-chart-pattern"`; anything else is a named load error
   (`unknown texture generator '<name>'`). Spec params are read
   with `str_field(spec, "base", ctx)?` etc. The format is
   documented in `talerender/docs/scene-format.md` §8; scene
   texture entries look like
   `{ "id": "planks-floor", "generator": "planks", "base": "#7A5230", "canvas": [512, 512], "repeat": [3.2, 3.6] }`
   (`talerender/scenes/sets/bookshop.scene.json`). **`repeat` is a
   texture-entry property, never a painter input** — in the source
   it is applied to the texture clone by `canvasTex`
   (world.ts:28-35) while one canvas per colour is cached/shared;
   every registry painter documents this and mine do too.

**The bookshop precedent** (DESIGN.md's "the registry already
absorbed the bookshop family the same way"): the painters now
living at scene.rs:1795-2960 arrived as a *wave pack* — one file
per painter, each a self-contained drop-in with (a) a header block
naming the TS source lines, the integration instructions and the
registration snippet, (b) its own per-painter `shade` helper
reproducing three.js r170 `THREE.Color` arithmetic in f64
(`planks_shade`, scene.rs:1861; felt's `shade_three`,
scene.rs:2379), (c) the painter in `Painter` ops transcribed
op-for-op, (d) a `#[cfg(test)]` module with pinned pixels, shade
truth tables (including the two corpus witnesses
`shade('#3A2A2A', 0.7) = #302222` and `shade('#3A2E22', 1.25) =
#413427` recorded from the real three.js pipeline) and statistical
pins. The family: `painters/planks.rs` → `paint_planks`
(scene.rs:1923), `wallpaper.rs` → `paint_wallpaper` (2193),
`felt.rs` → `paint_felt` (2409), `parcel.rs` → `paint_parcel`
(2532), `picture.rs` → `paint_picture` (2676), `sign.rs` →
`paint_sign_px` / `paint_sign` (2854/2873). They serve the
bookshop set (`scenes/sets/bookshop.scene.json`, transcribed from
the Onoda project's interior). **My five files follow this form
exactly** — same header structure, same per-painter `*_shade`
helper (renamed per painter so all five can coexist in one
module), same test style.

**Text precedent** (load-bearing for facade): the engine has **no
Rust text rasteriser**. `paint_sign` (scene.rs:2753ff) paints the
board + border natively and composites a PIL glyph strip via
`sign_with_text()`; all film text (captions, calendar plates, the
sign itself) enters as PIL assets rendered from the project's own
fonts by `talerender/film/gen_assets.py`. Facade follows this
exactly (§3).

## 2. The five TS painters, and proof none existed natively

All five live in the `T` object + `facade()` in
`~/workspace/custom-skills/tall-tale-shorts/template/src/core/world.ts`:

| painter | source | canvas | signature |
|---|---|---|---|
| brick | world.ts:76-79 | 512×512 | `T.brick(col = '#B5553C', rep)` |
| siding | world.ts:80-82 | 256×256 | `T.siding(col = '#6FA3B8', rep)` |
| dirt | world.ts:83-89 | 512×512 | `T.dirt(col = '#C8763F', rep, tracks = true)` |
| sand | world.ts:94-100 | 512×512 | `T.sand(col = '#E3C078', rep)` |
| facade | world.ts:111-149 | round(w·110)×round(h·110) | `facade(name, {wall, kind, trim, sign, signText, awning, w = 5, h = 7, seed = 1})` |

Callsites that need them (the R-W5 gap): props.ts house/barn
(siding), chimney + world2.ts fireplace (brick), props.ts rails
(dirt bed), mainStreet/farmyard/desertCamp/cityBlock roads and
yards (dirt), desert/beach/pond (sand), mainStreet + cityBlock
shopfronts (facade). B3-07 ACCOUNTING flags them per item
(prop3d-barn, prop3d-rails, prop3d-pond, prop3d-fireplace,
prop3d-house, and every set that stages those builders).

**Not already native:** `grep` for `paint_brick|paint_siding|
paint_dirt|paint_sand|paint_facade` and for the generator strings
`"brick"|"siding"|"dirt"|"sand"|"facade"` across
`talerender/src/`, `talerender/scenes/` and
`docs/scene-format.md` returns nothing. The two apparent hits —
`"name": "sand"` in `scenes/film/s02.scene.json:211` /
`s03.scene.json:209` — are *node names* on flat-colour discs
(`"color": "#DEC795"`, no texture at all; cf. sets README: "sand
disc … flat colour, **no map in the source**"). No duplication.

## 3. Per-painter transcription notes

Shared: `shade()` is three.js `THREE.Color` (hex → linear,
multiply, → sRGB, rounded) — each file carries its own f64
implementation with three r170's exact constants
(0.0773993808 / 0.9478672986 / 0.0521327014, exponent 2.4 /
0.41666), per the planks/felt precedent. (scene.rs's older
`shade_rgb` now computes the same linear multiply; the wave-pack
painters nevertheless carry their own f64 copy — I follow the
wave pack.) `globalAlpha` is folded into the colour's alpha byte
as `round(alpha·255)` — the planks precedent (0.35 → 89): speckles
0.5 → 128, dirt ruts 0.18 → 46, sand ripples 0.16 → 41, facade
gloss polygons 0.35 → 89 and 0.3 → 77.

- **brick** — mortar `shade(col, 0.6)`; 16 rows × 9 columns
  (bw 85: `c` from −1 while `c < 512/85 + 1 = 7.02`; the edge
  bricks hang off-canvas, which is what makes the tile seamless —
  `Painter` clipping reproduces it). Brick fill
  `shade(col, 0.85 + rnd(r·31 + c)·0.3)`, rect inset 3 px.
  No construct without a native analogue.
- **siding** — the one painter with **no randomness**. Each of the
  8 boards is a *vertical linear gradient*
  `shade(col, 1.06) → shade(col, 0.86)` plus a 2 px
  `shade(col, 0.6)` seam. TS construct with no direct `Painter`
  analogue: `createLinearGradient`. Resolution: a per-painter
  gradient helper (`siding_grad_rect`) that evaluates the stop
  lerp in sRGB space at each pixel centre (what canvas does) with
  coverage from the same box SDF `fill_rect` uses. The boards are
  integer-aligned, so interior rows take coverage 1 and the
  gradient lands per-scanline exact — the harness shows siding
  **byte-exact** (§5).
- **dirt** — literal base fill (no shade); 2600 speckle discs
  (`arc(..., 0, 7)` — 7 rad > 2π, full discs; radius
  1 + rnd·2.5) at alpha 0.5; then, iff `tracks`, four 14 px ruts
  at x ∈ {0.3, 0.38, 0.62, 0.7}·512 in `shade(col, 0.6)` at alpha
  0.18, painted *over* the speckles. `fill_ellipse` is the disc
  analogue (exact for circles: scene.rs's `ellipse_sd` reduces to
  d − r). Registration note: the TS default is `tracks = true`,
  but scenefile's existing `truthy()` defaults absent → false, so
  the lead must spell this one flag `map_or(true, …)` (noted in
  the file header).
- **sand** — literal base; 3200 speckle discs (radius 1 + rnd·2)
  at alpha 0.5; 7 ripple strokes: polylines sampled at
  x = 0, 16, …, 512, `y = 30 + r·70 + sin(x·0.05 + r·2.2)·7`
  (Math.sin in f64), lineWidth 3, `shade(col, 0.68)` at alpha
  0.16, one `stroke_polyline` per line (the source strokes seven
  separate paths; crossings/joins follow the house union-distance
  model). The lines run off-canvas at both ends, so
  `stroke_polyline`'s CapFan end-cap refinement never shows.
- **facade** — a composite painter; three TS constructs needed
  resolutions:
  1. **`drawImage` tiling** of `T.brick(wall).image` /
     `T.siding(wall).image`: facade calls the sibling painters
     and tiles their canvases. Resolution: `paint_facade` calls
     `paint_brick` / `paint_siding` (kind `"brick"` → brick,
     anything else → siding — the source's exact branch;
     callsites pass `"wood"` for the siding branch) and blits
     their byte output (`facade_blit`; the tiles are fully
     opaque, so canvas source-over is a copy). Paste order in
     scene.rs therefore matters: brick/siding before facade.
  2. **Two diagonal linear gradients** (upper-window glass;
     shop-window glass): same gradient helper as siding's
     (`facade_grad_rect`). One subtlety transcribed as written:
     the shop-window gradient's axis starts at `(wx, by)` while
     its rect starts at `(wx, by + 22)` — the source creates the
     gradient before offsetting the rect, so the glass's top rows
     start part-way down the ramp. The recorded op stream (§5)
     confirms this is the real behaviour, not a reading error.
  3. **The sign-board text** (`fillText` with a
     measureText-driven shrink loop): no native analogue exists
     *by design* — resolution is the `paint_sign` precedent.
     `paint_facade` paints everything except the glyphs
     (including the board and its `signText` border);
     `facade_with_text(board, strip)` composites a same-size PIL
     glyph strip. The layout law is exported as data:
     `FACADE_PX_PER_UNIT = 110`, `FACADE_FONT_START_FRAC = 0.56`
     (of the sign-band height `ch·0.1`), `FACADE_FIT_FRACTION =
     0.78` (of cw), `FACADE_FONT_STEP = 2`, anchor
     `facade_sign_anchor(cw, ch) = (cw/2, ch·0.47 + ch·0.05 + 2)`.
     PIL measurements at the default 550×770 canvas (skill's own
     Caladea-Bold.ttf; start fs 43.12, limit 429.0): **BAKERY**
     158.73 px, **GENERAL STORE** 313.88 px — no shrink step
     fires for either canonical name; both are recorded as
     `BAKERY_PIL` / `GENERAL_STORE_PIL` consts in facade.rs,
     mirroring sign's `TANIGUCHI_BOOKS_PIL`. Caveat, same as
     sign's gap B9: these are PIL metrics; Skia's Caladea metrics
     are not provable from here, but the headroom (≥115 px) makes
     a browser-side shrink implausible for these names. Longer
     names would need their own PIL measurement when a project
     bridges them (the strip generator's job, not the painter's).
  Also: the awning's `arc(cx, cy, 20, 0, PI)` semicircle scallops
  are painted as 24-chord flattened half-disc polygons
  (`facade_semi_pts`; sagitta 0.043 px at r 20 — the house
  curve-flattening precedent from planks/picture). The window
  blind (`rnd(seed·13 + i) > 0.5` → `shade(wall, 0.8)` over the
  top 35%) is painted **after** the mullions, as in the source;
  the door side is `rnd(seed) > 0.5 ? 0.08·cw : 0.72·cw` — the
  harness pins exercise both branches (bakery seed 1 → door
  right; general store seed 2 → door left).

## 4. Determinism argument

Every painter is a pure function of its parameters:

- The only randomness source in the TS recipes is
  `rnd(i) = frac(sin(i·127.1 + 311.7)·43758.5453)` — a hash of
  loop indices, no state. The ports call `scene::rnd` with the
  *same f64 arguments in the same expressions* (including
  negative `c = −1` brick indices and facade's `seed·13 + i` /
  `seed`), so the hash streams are identical by construction.
- `shade` is a closed-form f64 computation (three's constants);
  gradients are closed-form per pixel; the facade's `seed`
  enters only through `rnd`.
- No wall-clock, no HashMap iteration, no RNG state anywhere —
  the same property the sibling painters assert, and each file's
  tests assert byte-equality of two calls (plus base-sensitivity).
- Residual cross-engine risk is the one the registry already
  documents (felt's pinned-count test): Rust libm `sin` vs V8
  fdlibm `sin` can differ by ~1 ulp, which can move a speckle's
  edge blend by a byte on a handful of pixels. It cannot move
  *which* colours appear or any interior pixel. The dirt/sand
  statistical pins carry a ±64 tolerance for exactly this, to be
  tightened after the first CI run (the picture painter's
  precedent for measured bands).

## 5. Harness — method and real numbers

**Method.** `record_ts_ops.ts` extracts the `T` literal and
`facade()` **verbatim** from the skill's world.ts and executes
them under bun with a recording 2D context (only the environment
is stubbed: `shade` via three r170 constants in f64; `canvasTex`;
`measureText` backed by PIL widths from the skill's own
Caladea-Bold.ttf). The recorded op counts match the source loops
exactly (brick 145 ops = 1 + 16×9; dirt 2605 = 1 + 2600 + 4;
sand 3208 = 1 + 3200 + 7; siding 16 = 8×2). `ts_reference.py`
rasterises those op streams with plain canvas semantics
(pixel-centre sampling, source-over). `port_mirror.py` is an
independent transcription of the same TS recipes implementing
scene.rs `Painter` semantics (SDF coverage, `blend_cov` alpha
quantisation, f64 three-shade) — i.e. the Rust files' arithmetic,
mirrored because **this VM has no rustc** (standing rule: Rust is
verified in CI). The mirror omits `stroke_polyline`'s CapFan
end-cap refinement; it affects only open stroke ends, and sand's
ripples end off-canvas, so it cannot show in these numbers.
Run: `python3 harness/compare.py` (~30 s) →
`harness/out/report.json` + PNG pairs.

**Results** (reference vs port, per recorded call):

| call | dims | exact-equal | within ±8 | MAE (r,g,b) | max Δ | palette ref→port | shared-colour px |
|---|---|---|---|---|---|---|---|
| brick_default (#B5553C) | 512×512 | 99.05% | 99.05% | 0.171, 0.088, 0.065 | 25 | 38 → 57 | 99.06% |
| brick_chimney (#9E4A3A) | 512×512 | 99.05% | 99.05% | 0.152, 0.078, 0.065 | 22 | 35 → 52 | 99.06% |
| siding_default (#6FA3B8) | 256×256 | **100.00%** | 100.00% | 0, 0, 0 | 0 | 22 → 22 | 100.00% |
| siding_house (#F2E6CF) | 256×256 | **100.00%** | 100.00% | 0, 0, 0 | 0 | 31 → 31 | 100.00% |
| dirt_default (tracks) | 512×512 | 76.90% | 99.999% | 0.245, 0.241, 0.108 | 11 | 106 → 160 | 83.36% |
| dirt_notracks (#8A5A36) | 512×512 | 87.28% | 100.00% | 0.145, 0.108, 0.079 | 8 | 73 → 123 | 93.12% |
| sand_default (#E3C078) | 512×512 | 82.65% | 99.998% | 0.247, 0.212, 0.141 | 10 | 129 → 192 | 89.43% |
| facade_bakery (brick, awning, seed 1) | 550×770 | 97.67% | 97.75% | 1.036, 0.878, 0.668 | 124 | 1058 → 3026 | 97.69% |
| facade_general (wood, no awning, seed 2) | 550×770 | 98.00% | 98.10% | 1.040, 0.716, 0.427 | 124 | 1094 → 2741 | 98.01% |

Mean colours agree to ≤0.13 per channel on every call (worst:
facade_general r 136.06 vs 135.93).

**Reading the diffs (all AA-class, no structural diffs found):**

- *brick*: every one of the 2496 pixels with Δ>8 is
  *blend-contained* in its 3×3 reference neighbourhood (2496/2496)
  — they are the 1-px AA rims `Painter` draws around each brick
  where the centre-sampled reference has a hard edge. Port
  palettes are larger for the same reason (AA intermediates).
- *dirt/sand*: Δ>8 pixels number 3 and 4 respectively in the
  whole canvas; everything else is ≤ ±8 speckle-rim noise from
  the coverage model plus the 128/255-vs-0.5 alpha quantisation
  the registry's globalAlpha convention carries.
- *facade*: the Δ>8 pixels form 1-px outlines on the
  fractional-coordinate rect edges (window pitch ww = 183.33 px).
  Blend-containment: 7996/8051 (general) and 9298/9528 (bakery);
  the bakery remainder (230 px) sits on stacked-AA edges — chiefly
  the blind's top edge at y = 123.2, where the port blends blind
  over glass at coverage 0.75 while the centre-sampled reference
  takes the blind whole. True Skia area coverage there is 0.80 —
  **the port sits closer to Chrome than the reference does** on
  exactly these pixels. Interiors (cornice, dentils, sign board,
  door, product boxes) are byte-exact; the Rust tests pin six of
  them, including both door-side branches.
- *siding* is byte-exact because the recipe has no curves and its
  gradient is scanline-aligned — a useful control: it proves the
  gradient model and the shade arithmetic are exactly right, so
  the other painters' residuals are attributable to AA modelling,
  not to colour or layout errors.

## 6. Lead wiring spec (what the lead registers where)

1. **Paste** the five files' bodies into `talerender/src/scene.rs`
   in this order: brick, siding, dirt, sand, facade (facade calls
   `paint_brick` / `paint_siding`). Each file's items are named to
   coexist (`brick_shade`, `siding_shade`, `siding_grad_rect`,
   `facade_grad_rect`, `facade_blit`, …); each carries its own
   `#[cfg(test)] mod <name>_tests`. They use only existing
   `pub(crate)` scene.rs items (`Painter`, `Rgba`, `hex_rgb`,
   `rgba_of`, `rnd`, `cov_area`).
2. **Register** in `scenefile.rs::resolve_texture`, beside
   `"grass"` / the bookshop family:
   - `"brick" => scene::paint_brick(str_field(spec, "base", ctx)?)`
   - `"siding" => scene::paint_siding(str_field(spec, "base", ctx)?)`
   - `"dirt" => scene::paint_dirt(str_field(spec, "base", ctx)?, tracks)` with `tracks` defaulting **true** when the spec omits it (see §3; do not reuse `truthy()`'s absent→false here)
   - `"sand" => scene::paint_sand(str_field(spec, "base", ctx)?)`
   - `"facade" => scene::paint_facade(wall, kind, trim, sign, signText, awning, w, h, seed)` reading spec fields `wall`, `kind`, `trim`, `sign`, `signText` (strings), `awning` (string or null/absent → `None`), `w`, `h`, `seed` (numbers; seed integral). Schematic snippet is in facade.rs's header; accessor spellings are the lead's to match to scenefile's helpers.
3. **Scene files** can then name the surfaces exactly as the TS
   builders do, e.g.
   `{ "id": "wall", "generator": "siding", "base": "#F2E6CF", "repeat": [3, 2] }`,
   `{ "id": "road", "generator": "dirt", "base": "#C98A52", "tracks": true }`,
   `{ "id": "front", "generator": "facade", "wall": "#C9603F", "kind": "brick", "trim": "#F2E6CF", "sign": "#2E4E3F", "signText": "#F2D37A", "awning": "#D8343F", "w": 5, "h": 7, "seed": 1 }`
   — sets naming siding/brick/dirt/sand/facade then resolve
   instead of failing with `unknown texture generator`.
4. **Facade text**: a scene that wants the shop name rendered
   needs a PIL glyph strip for its name at
   `facade_canvas_size(w, h)`, text at `facade_sign_anchor(cw,
   ch)` in the `signText` colour, font size from the shrink rule
   (start `ch·0.1·0.56`, step −2 while PIL width > `cw·0.78`),
   composited via `facade_with_text` — the same build-time plate
   duty R1-06 generalises for text props. Without a strip the
   facade renders with a blank sign board (the sign precedent's
   behaviour, pinned by facade's tests).
5. **Docs**: add the five rows to `docs/scene-format.md` §8's
   generator table (params: brick/siding/sand — `base`; dirt —
   `base`, optional `tracks`; facade — the field list above).
6. **Verification on landing** (per contracts §8): the five test
   modules run under `cargo test` in CI (first run also settles
   the dirt/sand statistical-pin tolerances); golden verify 17 OK
   must stay green (these are additive registry arms — no existing
   painter or scene changes a byte); a scene file naming each new
   generator should be added to the loader's test scenes or the
   proof fixture so the arms are exercised end-to-end.

## 7. Honest limitations

- **The Rust files have not been compiled.** This VM has no
  rustc (the repo's standing rule is CI verification; cf.
  StykAnimate build rule and the talerender CI pattern). The
  files were written against scene.rs APIs read directly from
  the source at HEAD `9743850`-era contracts (all items used are
  `pub(crate)` in scene.rs today), and their arithmetic is
  verified through the Python mirror + the recorded op stream.
  First-CI compile/test is the lead's landing step; the
  statistical pins (dirt/sand survivor counts) are the most
  likely first-run adjustments, and their comments say so.
- The harness reference is a *centre-sampled* rasterisation of
  the recorded ops, not a Skia render — by design it isolates
  transcription fidelity; the AA-model delta between `Painter`
  and Skia is the registry's already-documented gap class
  (stroke battery), not something this port introduces.
- Facade text fidelity is the PIL pipeline's (shared by both
  harness sides, so it cancels in the comparison): PIL Caladea
  metrics/AA vs Skia's will differ at glyph edges in production,
  exactly as they already do for the bookshop sign and every
  caption strip in the shipped engine.
