# paint_planks — parameter contract & usage mapping

Native port of `T.planks` (the film project's own canvas painter,
`~/faceless-shorts/the-soldier-who-didnt-know/src/core/world.ts:58-65`),
written to close the registry gap the wave found: the film's scene
data names a `planks` texture generator (crates, barrel, lean-to
roof, bookshop floor/wainscot/counter/desk) that
`src/scenefile.rs::resolve_texture` does not know yet — and there
is no graceful degradation: `resolve_texture` returns
`unknown texture generator 'planks'` (`src/scenefile.rs:426`) and
`load()` resolves the `textures` array eagerly with `?`
(`src/scenefile.rs:941`), so any file that references it fails to
load whole until the arm below lands
(`scenes/film/ASSUMPTIONS.md`: the S02/S03 crate+barrel notes, the
S10 registry-gaps list).

Module: [planks.rs](planks.rs) — signature, size and determinism
conventions mirror `paint_grass` exactly:
`pub(crate) fn paint_planks(base: &str) -> (Vec<u8>, u32, u32)`,
fixed **512×512** RGBA, straight alpha, row 0 = top.

Registration (one line, beside `"grass"` in `resolve_texture`):

```rust
"planks" => scene::paint_planks(str_field(spec, "base", ctx)?),
```

## Parameters the format must accept

| field (texture entry) | painter input? | meaning |
|---|---|---|
| `generator: "planks"` | — | dispatch key, as with `grass`/`cloud`/… |
| `base` (hex string) | **yes — the only one** | the source's `col` argument to `T.planks(col, rep)`; default in the source is `#A8693E`, but every film entry carries it explicitly, so the dispatch above requires it, exactly like `grass`'s `base`. |
| `repeat: [rx, ry]` | no | the source's `rep` argument — in the source it is **not** drawn into the canvas: `canvasTex` (world.ts:24-33) caches ONE 512×512 canvas per colour (`'planks' + col` key) and applies `rep` to a per-use texture **clone's** UV repeat. Baking it into the painter would break that sharing (the same canvas serves `[0.5, 0.5]` crates and `[3.2, 3.6]` floors). It belongs to the material/UV layer, where the film entries already record it. |
| `rotation` (radians) | no | texture-space rotation, likewise a clone property in the source (see the BK floor below: `map.rotation = PI/2`, world.ts interior). Recorded on the s10 floor entry as `1.5707963705062866`; `src/film.rs` currently emulates it with a swapped repeat. Not a painter concern. |
| `canvas: [512, 512]` | no | informational in the film entries; matches the painter's fixed size (the source's `canvasTex(..., 512, 512, ...)`). A loader may assert it. |

**There is no plank-direction parameter.** In the source the planks
always run horizontally — 8 rows stacked along +y, grain curves
running along +x — for every call site. Vertical-looking runs in the
sets are achieved with texture rotation/repeat on the mesh, never
with a painter variant. Do not invent one.

Internal constants (source-fixed, not parameters): 8 rows of 64 px;
row shade factor `0.86 + rnd(3i)·0.24`; 4 grain beziers per row in
shade 0.70 at alpha 0.35, lineWidth 2; seam band 4 px and butt joint
4 px wide in shade 0.45; two r=3 knots per row in shade 0.50 at
(joint − 14, joint + 18), mid-row.

## Usage sites in the film data → source calls

| film entry / site | material in the source | `base` | `repeat` | notes |
|---|---|---|---|---|
| `planks-crate` (`scenes/sets/shore.scene.json`; JG crates identical) | `mat({ map: T.planks('#B07A48', [0.5, 0.5]) })` — **no material colour** (white × map, "colour-less / map-only"), world.ts:216 `crate()` | `#B07A48` | [0.5, 0.5] | the brief's "crate #B07A48 [0.5,0.5] colour-less map-only" |
| `planks-barrel` (`scenes/sets/shore.scene.json`) | `mat({ color: '#9A6337', map: T.planks('#9A6337') })`, world.ts:215 `barrel()` — material colour **equals** the map base, so the map is multiplied by its own base colour | `#9A6337` | [1, 1] (source default) | the brief's "barrel #9A6337 × planks [1,1]" |
| lean-to roof, JG camp (S01 set; `scenes/film/s01`…) | `WD.mat({ map: WD.T.planks('#8A6B42', [1, 1]), roughness: 0.95 })`, shots.ts:181 — **map-only** (no colour) | `#8A6B42` | [1, 1] | the brief's "roof map-only". (A different roof in props.ts:39 — the hut — uses `color: roof × T.planks(roof, [2, 2])`; not the film's lean-to.) |
| `planks-floor` (`scenes/film/s10.scene.json`) | `mat({ map: T.planks(floor, [w/2.5, (depth+2)/2.5]) })` on the floor plane, then `map.rotation = PI/2`, world.ts `interior()` (floor block) | `#7A5230` (s10's `floor` option) | [3.2, 3.6] (= [8/2.5, 9/2.5]) | **rotation PI/2 recorded on the entry**; film.rs emulates by swapping the repeat to [3.6, 3.2] |
| `planks-wainscot-window/back/side` (s10) | `wain(len) = mat({ map: T.planks('#6B3E26', [len/2, 0.5]) })`, world.ts `interior()` | `#6B3E26` | [1.6, 0.5] / [1.0, 0.5] / [4.5, 0.5] | map-only |
| `planks-counter` (s10) | `mat({ map: T.planks('#7A3E24', [1, 2]) })`, world.ts `interior()` counter | `#7A3E24` | [1, 2] | map-only |
| `planks-desk` (s10) | `mat({ map: T.planks('#6B3E26', [1, 1]) })` (props.ts desk `wood`) | `#6B3E26` | [1, 1] | map-only |

Other source call sites (not in this film, same painter): default
`#A8693E` — boardwalk `T.planks('#A8693E', [(len+20)/3, 1])`
(world.ts `mainStreet`), crate lid `[2, 1]` (props.ts); coloured
variants `#7A5236 [0.3, 1]` (pole), `#2E5A44 [2, 1]` (bench),
`#8A5A36 [1, 1]` (table) in props.ts — all are just other `base` /
`repeat` pairs; the painter covers them unchanged.

## What the module cannot reproduce exactly (named, not hidden)

1. **Skia's stroke rasterisation vs the house stroke model.** Canvas
   strokes the grain beziers directly; the port flattens each cubic
   to a 128-segment polyline (≤ ~0.01 px deviation) and strokes it
   with `Painter::stroke_polyline`, whose signed-distance coverage
   is the M4-verified model, not Skia's scan conversion — edge
   pixels can differ by a few LSB of alpha along the curves. Caps
   differ in kind: canvas defaults to **butt** caps, the house model
   uses the flattened **round** cap fan. Because every grain curve
   spans the full 512 px width, the cap difference is confined to
   the first/last pixel columns. (The registry already accepts this
   class of delta for `T.grass`, whose canvas strokes are likewise
   butt-capped lines.)
2. **Knot and fractional-rect edges.** Canvas `arc()` fills and
   `fillRect(jx, …)` with fractional `jx` are Skia-rasterised; the
   port uses `fill_ellipse` (approximate ellipse SD — exact for
   these r=3 circles) and the box-SDF `fill_rect`, both through
   `cov_area`. Interior pixels are exact (the tests pin knot
   centres and the seam band byte-for-byte); rim pixels can differ
   by ~1 LSB of coverage.
3. **`rnd()` last-bit divergence.** `rnd` is `frac(sin(i·127.1 +
   311.7)·43758.5453)`; Rust's f64 `sin` and V8's `Math.sin` can
   differ by ≤1 ulp on some inputs (the corpus already documents
   this for set placement, W10 G7). A hash sitting exactly on a
   rounding boundary could flip and move one grain curve or joint
   by a hair. Not eliminable in principle; affects at most a
   1-ulp change in one hash per texture.
4. **globalAlpha quantisation.** Canvas composites the grain
   strokes at alpha exactly 0.35 (float, over an 8-bit canvas);
   the port folds it into the colour's alpha byte 89, i.e.
   89/255 ≈ 0.34902 — a ≤0.1 LSB difference per stroke pixel
   before storage rounding. Sub-visible, but it is a real
   arithmetic difference, so it is named.
5. **Inherited registry inconsistency — `shade_rgb` (the big one,
   and it is NOT in this module).** `world.ts:41 shade()` is
   `THREE.Color` arithmetic: hex → linear working space,
   `multiplyScalar`, → sRGB hex. The M0 audit (docs/audit/
   a1-engine.md, `THREE.Color` row) warned: reproduce that "or
   every derived shade shifts", and the s10 wave data independently
   recorded shade() outputs in linear space (#413427, #302222 —
   both pinned as tests in planks.rs). `planks_shade` therefore
   reproduces the linear-space semantics. But `src/scene.rs`'s
   existing `shade_rgb` multiplies the sRGB **bytes** directly
   (it ports toon.ts's canvas `shadeOf`), and the registry's
   existing world.ts painters (`paint_grass`) and `src/film.rs`
   use it for `shade()` calls — e.g. `shade('#A8693E', 0.45)` is
   (116, 71, 40) by the source semantics vs (76, 47, 28) by
   `shade_rgb`: a 40-LSB difference, plainly visible. This module
   follows the source; reconciling `paint_grass`/`film.rs` to the
   same semantics is an integrator decision (it would change
   already-measured A/B frames), so it is flagged here, not done.
6. **three's LinearToSRGB exponent.** three r170's conversion uses
   `c ** 0.41666` (truncated) rather than the exact `1/2.4`;
   `planks_shade` deliberately uses three's constants (0.41666 and
   friends) so this is reproduced, not a delta — recorded so a
   future "cleanup" to scene.rs's spec-exact `linear_to_srgb`
   (which uses `1/2.4`) knows the two can disagree by ≤1 LSB on
   boundary values (~0.002 % relative).

Everything else is op-for-op: op order per row (fill → 4 strokes →
seam → joint → knots), the joint reusing the seam's 0.45 fillStyle,
the four strokes as separate blends, knot positions relative to the
hash-placed joint, and the full-circle `arc(…, 0, 7)`.
