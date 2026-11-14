# Registry painters: `picture` + `sign` (seat W24)

Two texture generators the bookshop data names but the registry in
`src/scenefile.rs` (`resolve_texture`) does not have. Staging
modules — nothing in `src/` was touched; integration is the lead's.

| generator | painter | canvas | spec fields |
|---|---|---|---|
| `picture` | `painters/picture.rs` → `paint_picture()` | 256×200 fixed | none |
| `sign` | `painters/sign.rs` → `paint_sign(bg, fg, w, h)` / `paint_sign_px(bg, fg, cw, ch)` | `round(w·200)` × `round(h·200)` | `text`, `bg`, `fg`, `font`, `canvas` |

Spec source: `scenes/sets/bookshop.scene.json` texture entries
`picture` (canvas [256, 200]) and `sign` (text "TANIGUCHI BOOKS",
bg `#3A2A2A`, fg `#F2E6CF`, font "Caladea", canvas [440, 80]).

## Integration (for the lead)

Move the files to `src/painters/{picture,sign}.rs`, add the module,
and add two arms to `resolve_texture` in `src/scenefile.rs`:

```rust
"picture" => painters::picture::paint_picture(),
"sign" => {
    // canvas [440, 80] in the spec is authoritative-equal to the
    // mesh size 2.2 x 0.4 units under the 200 px/unit rule.
    let (cw, ch) = /* from spec "canvas" */;
    painters::sign::paint_sign_px(
        str_field(spec, "bg", ctx)?, str_field(spec, "fg", ctx)?, cw, ch)
    // + sign_with_text(board, strip) if the spec grows a glyph
    //   strip file for "text" — see "Text strategy" below.
}
```

(Or paste the bodies into `src/scene.rs`, where every helper they
use is already in scope, and drop the `use` lines.)

Both follow the registry painters' conventions (`paint_grass`,
`paint_cloud`, `paint_blob` in `src/scene.rs`): return
`(Vec<u8> RGBA, w, h)`, canvas coordinates with row 0 = top, pure
and deterministic — no state, no RNG (neither recipe uses the
sin-hash). Each module carries unit tests in the crate's style:
size, determinism, pinned pixels / a pinned statistic band.
**The tests have not been run** — this VM has no cargo (standing
law: builds only via GitHub Actions CI); they verify on the first
CI run after integration, and the sun-disc band in `picture.rs`
should be tightened to an exact count then.

## `picture` — parameters and recipe

Source: the Onoda project's `core/world.ts:319` (`interior()`'s
`pic` canvasTex). Fixed canvas 256×200, no parameters:

1. Fill `#E9DCC0` (paper).
2. Ink `#3A2A22`, lineWidth 6, one path:
   `moveTo(40,140)`, `quadraticCurveTo(90,60 → 140,120)`,
   `quadraticCurveTo(170,80 → 220,140)`, stroke.
   Painted as ONE `stroke_polyline` over both quadratics flattened
   with the house `flatten_quad` (n = 20 per quad — the
   `toonfull.rs` precedent), so the join at (140,120) is the union
   shape, exactly like a single canvas stroke.
3. Sun: fill `#C9603F`, `arc(190,60,22, 0, 7)` — end angle 7 rad
   exceeds 2π, so a full disc; painted with `fill_ellipse`.

The gold frame is **not** in the texture: in the source it is the
box mesh's face materials, `#C9A13B` on five faces with this
texture on the front (`world.ts:320`). The constant
`PICTURE_FRAME` in `picture.rs` records it for the geometry side.

## `sign` — parameters and recipe

Source: `core/world.ts:137–146` (`signBoard`), called from
`interior()` at `world.ts:321` with
`{ w: 2.2, h: 0.4, bg: '#3A2A2A', fg: '#F2E6CF' }` and the text
"TANIGUCHI BOOKS".

- Canvas: `Math.round(w·200)` × `Math.round(h·200)` px
  (200 px/unit) → 440×80 (`sign_canvas_size`).
- Fill `bg`; border `fg`, lineWidth 6, `strokeRect(10, 10, cw−20,
  ch−20)` — canvas strokes are centred on the path, so border ink
  spans 7–13 px from each edge. Painted as one closed
  `stroke_polyline`.
- Board side faces (geometry, not texture): `shade(bg, 0.7)`.
  `sign_edge_rgba` computes it the source's way — `world.ts:35`
  shade is `THREE.Color`: hex → **linear**, multiply, → sRGB.
  For `#3A2A2A` that gives **(48, 34, 34) = `#302222`** (verified
  by direct computation). Note `src/scene.rs`'s existing
  `shade_rgb`/`shade_lin` multiply in sRGB space and would give
  (41, 29, 29) — the colour-space divergence seat W14 flagged;
  this helper deliberately follows the source.
- Text layout: `700 {fs}px Caladea, Georgia, serif`, centred,
  middle baseline, at `(cw/2, ch/2 + 3)` (`sign_text_anchor`).

### Text strategy — the choice, and why

**This engine has no Rust text rasteriser, by design.** Every
text-bearing texture in the project enters as a PIL asset rendered
from the project's own fonts: the caption strips, the S07 calendar
plates, and this very sign (`film/assets/sign.png`) — all produced
by `film/gen_assets.py` (see the module docs at `src/film.rs:10`:
text enters as assets). `paint_sign` follows that split:

- board + border are painted natively (the deterministic vector
  part — the registry's job);
- glyphs are composited from a same-size transparent strip with
  `sign_with_text()` — the strip is the PIL step's output (700
  Caladea in `fg`, laid out at `sign_text_anchor` with the font
  size below), blended with the house `Painter::blend`.

Without a strip, `paint_sign` returns the board exactly as the
canvas looks before `fillText` — it never invents glyphs.

### The font-size decision (gap B9) — plainly

The rule (`world.ts:143–144`): `fs` starts at `ch·0.5` = **40**,
and shrinks by 2 while `measureText(text) > cw·0.86` = 378.4 px.
The outcome depends on font metrics, and **browser (Skia/Chrome)
Caladea metrics are unreproducible from source — that caveat
stands.** What can be pinned is the PIL measurement, and it is
recorded as data (`TANIGUCHI_BOOKS_PIL` in `sign.rs`):

- Measured 2026-11-14 with PIL against the project's own
  `fonts/Caladea-Bold.ttf` (the font `signBoard`'s `700 … Caladea`
  resolves to): "TANIGUCHI BOOKS" at 40 px is **335.36 px** wide.
- 335.36 ≤ 378.4 → **no shrink step fires; fs = 40.** Headroom
  43.04 px, so a shrink firing in the browser is unlikely — but
  not proven, and this doc does not claim it.
- `film/gen_assets.py`'s own implementation of the same loop
  (with PIL metrics) also lands on 40.

## Unreproducible / declared gaps

1. **Browser font metrics (B9)** — as above: the browser's fs is
   not provable from source or from PIL. fs = 40 is PIL-measured
   data, labelled as such everywhere it appears.
2. **The PIL film asset differs at the border.** `gen_assets.py`
   draws the sign's border with PIL `rectangle(..., width=6)`,
   whose ink sits inside the given box; the canvas `strokeRect`
   centres ink on the path. `paint_sign` follows the **canvas
   source**, so the native sign and `film/assets/sign.png` differ
   along the border by up to the stroke half-width (3 px). Same
   declared PIL-vs-canvas class as the caption strips.
3. **Anti-aliasing model.** Canvas/Skia analytic coverage vs the
   painter's signed-distance coverage (the M4 model, battery-
   proven for strokes) — edges can differ sub-pixel; interiors
   are exact.
4. **Quadratic flattening.** Skia flattens adaptively; the painter
   uses `flatten_quad` n = 20 (chord error ≪ 0.1 px at these
   radii). A deviation source only in the stroke's AA band.
5. **Sign font stack fallback.** If Caladea were missing, the
   browser would fall back to Georgia/serif with different
   metrics — another reason the browser's fs is unreproducible;
   the project ships Caladea-Bold, so the primary path is the
   measured one.
6. **Tests unverified locally** — no cargo on this VM; first CI
   run after integration is the verification (see above).
