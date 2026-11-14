# paint_wallpaper — registry painter notes (seat W22)

Source: `T.wallpaper` in the Onoda project's `src/core/world.ts:69-73`;
call sites in `interior()` at `world.ts:297-313`. Module:
`painters/wallpaper.rs` (integration: into `src/scene.rs` beside
`paint_grass`/`paint_sky`, registered in `src/scenefile.rs`'s
`resolve_texture`). This closes the `wallpaper` half of gap B8 in
`scenes/sets/README.md`. A local duplicate already exists in the film
driver (`src/film.rs:526`); differences from it are listed at the end.

## Registry contract

```
"wallpaper" => scene::paint_wallpaper(
    str_field(spec, "base", ctx)?,
    str_field(spec, "motif", ctx)?,
)
```

- Signature mirrors the registry painters: hex strings in,
  `(Vec<u8> RGBA, u32, u32)` out. Canvas **256×256** (the source's
  `canvasTex('wp' + col + motif, 256, 256, …)`).
- Spec fields are the ones the film data already declares:
  `base` (= the TS `col`), `motif`, `canvas: [256, 256]`, `repeat`.
- `repeat` is **not** a painter input. In the source it lives on the
  per-call texture clone (`canvasTex` caches by `'wp' + col + motif`
  — repeat is not in the cache key — and clones with a repeat), i.e.
  it is material/loader-side, exactly like `T.grass`'s `[48, 48]`,
  which `paint_grass` likewise ignores. Same for wrap mode and
  anisotropy 8: loader concerns.
- Determinism: the source painter calls **no `rnd()`** — the pattern
  is pure geometry — so the port is byte-deterministic by
  construction (pinned by a unit test).

## Parameters the film uses

Bookshop staging (`shots.ts:227` → `interior()` defaults overridden):
wall `#5A4632`, motif `#6B5638`, ceiling `#3A2E22`. One canvas image
serves every wallpaper surface; only the repeat differs per surface.

| Texture entry (sets / s10) | base | motif | repeat | Surface |
|---|---|---|---|---|
| `wallpaper-walls` / `wallpaper-back` | `#5A4632` | `#6B5638` | [1.7142857313156128, 3] | back-wall segments ×2, `wp((w−ww)/2)` = `wp(2.4)` (`world.ts:302`) |
| — / `wallpaper-back-top` | `#5A4632` | `#6B5638` | [2.2857143878936768, 3] | above-window band, `wp(ww)` = `wp(3.2)` (`world.ts:303`) |
| `wallpaper-sides` / `wallpaper-side` | `#5A4632` | `#6B5638` | [6.4285712242126465, 3] | side walls ×2; repeat set on the clone after construction (`world.ts:308-309`) |
| `wallpaper-ceiling` | `#3A2E22` | `#413427` | [6.666666507720947, 7.5] | ceiling, `T.wallpaper(ceiling, shade(ceiling, 1.25), [w/1.2, (depth+2)/1.2])` (`world.ts:313`) |

Notes on the variants:

- **Back-wall vs above-window band** are the same image at two
  repeats (motif scale differs 3:4 on the 1.2-high band). The sets
  file declares one entry with the segments' repeat and records the
  band's in its wallpaper-repeat note; the s10 file splits the band
  into its own entry. Either way the painter output is identical —
  the split is data-side only.
- **Ceiling** is a genuinely different image: darker base, and its
  motif is *derived* — `shade(ceiling, 1.25)` computed by the caller
  in the source and carried precomputed in the data (with a
  `motifLaw` note on the s10 entry). See the colour-space section.
- **The wainscot is not wallpaper.** The boards under the window
  (`world.ts:303`), along the sides (`world.ts:310`) and across the
  back (`world.ts:311`) are `wain(len)` = `T.planks('#6B3E26', …)` at
  repeats [1.6, 0.5], [4.5, 0.5] and [1.0, 0.5] respectively — they
  belong to the `planks` entries (W15's README table rows 5, 15-18,
  whose repeat values it corrected). Do not wire `wallpaper` to
  them; they are listed here only because they sit between wallpaper
  surfaces in the same table and were the variant mix-up W15 fixed.

## The drawing, op for op

| Source (world.ts:69-73) | Port |
|---|---|
| `fillStyle = col; fillRect(0,0,256,256)` | `fill_rect` over the canvas with `rgba_of(base)` |
| `fillStyle = motif` (strokeStyle/lineWidth 5 also set — see below) | motif `Rgba` for all subsequent ops |
| `fleur(64, 64, 0.9)`, `fleur(192, 192, 0.9)` | same two calls, same scale |
| petal: `moveTo(0,-40)` + 2× `bezierCurveTo` + `fill()` | both cubics via `flatten_cubic` (24 segments each), joined into one outline, `fill_polygon` |
| wings: one path, `arc(-22,4,12,0,7)`, `arc(22,4,12,0,7)`, `fill()` | two `fill_ellipse` discs r=10.8 at (±19.8, +3.6) — interpretation, see below |
| stem: `fillRect(-3, 10, 6, 26)` | `fill_rect` at the transformed rect |
| dots: `globalAlpha = 0.5`, 4× `arc(…, 6, 0, 7)` + `fill()` | `fill_ellipse` r 6, alpha **128**, centres in source order (64,192), (192,192), (64,64), (192,64) |

Transform per fleur: `translate(x, y)` then `scale(0.9, 0.9)` —
canvas point = centre + 0.9 · local. All geometry above is local.

## Colour space — the one place this painter's family differs

`world.ts:35`: `shade = (hex, k) => { const c = new THREE.Color(hex);
c.multiplyScalar(k); return '#' + c.getHexString(); }`.
THREE.Color stores components in the **linear** working space (the
hex is converted on construction), so `shade()` multiplies in linear
space and re-encodes on read-back. That is *not* the canvas
`shadeOf()` byte multiply that `scene::shade_rgb` ports.

The pinned witness is the ceiling motif:
`shade('#3A2E22', 1.25)` = **`#413427`** linear-space, but
`#493A2B` via `shade_rgb`. (Recorded independently by W15 on the
sets entry and by W14 as S10 divergence item 3 against film.rs,
whose caller used `shade_rgb`.) The module therefore carries its
own `shade_hex` implementing the THREE.Color order — hex → linear →
×k → sRGB, clamped, rounded — and a unit test pins
`shade_hex("#3A2E22", 1.25) == (0x41, 0x34, 0x27)`.

The painter proper never shades: both colours arrive as literals
(the ceiling motif precomputed by the caller, in source and data
alike) and are painted verbatim. `paint_wallpaper("#5A4632",
"#6B5638")` puts bytes `(0x5A,0x46,0x32)` / `(0x6B,0x56,0x38)` in the
buffer; tests pin base, petal, wing, stem and dot-composite pixels
for both film variants plus the `T.wallpaper` defaults
(`#9E2E3A` / `#B8414C`).

## Interpretations and unreproducible items

1. **The shared wing path.** The source adds both wing arcs to ONE
   path before filling. Read literally with a 7-radian sweep (401°),
   the connector lines between the arcs would fill a thin diagonal
   band joining the wings. Chrome/Skia clamps an arc's sweep to a
   full circle, so each arc ends where it starts, the connectors are
   degenerate (the same horizontal segment traversed out and back),
   and the fill is exactly two discs. The port paints two discs —
   the same reading film.rs's port took. This is the one structural
   interpretation in the painter; it is not separately measurable
   from this VM (no local build; the A/B path is CI).
2. **`arc(…, 0, 7)`** — 7 > 2π is the codebase's full-circle idiom
   (the same literal ends arcs in `dirt`, `facade`, the `pic`
   canvas). Treated as a full circle everywhere, per item 1.
3. **Dead fleur positions.** The position list is
   `[[64,64],[192,192],[192,64-256],[64,192]]` but the callback is
   `i < 2 && fleur(…)` — positions 3 and 4 are never drawn (and
   `(192,-192)` would be off-canvas regardless). The port draws two
   fleurs; a unit test pins untouched base where the fourth fleur's
   petal would have fallen. The dots, by contrast, DO land on all
   four grid points — including on top of both drawn fleurs'
   centres, where motif-over-motif at alpha 0.5 is a no-op.
4. **Dead stroke state.** `strokeStyle = motif; lineWidth = 5` are
   set and never used — no `stroke()` call exists in the painter.
   Nothing to port.
5. **Alpha 0.5 → 128.** The dots' `globalAlpha 0.5` becomes source
   alpha 128 (0.5·255 = 127.5; canvas quantisation and the Painter's
   blend both sit within one step of this; film.rs's port uses the
   same literal). The composite over base is test-pinned:
   `(99, 78, 53, 255)` at the bookshop wall colours.
6. **Anti-aliasing model.** Filled shapes go through the shared
   Painter's signed-distance coverage (the M4 model verified against
   Skia by the stroke battery), not Skia's own rasteriser. Interior
   pixels are exact; edge pixels carry the house model's known
   sub-quantisation deltas. Nothing in this painter is
   browser-dependent (no fonts, no images, no `rnd`), so unlike the
   `sign` painter there is no metric-dependent item to declare.

## Differences from film.rs's local duplicate

- Petal flattening: film.rs hand-samples the two cubics with 13
  points total; this module uses `flatten_cubic` at 24 segments per
  curve (the registry painters' flattening convention, sagitta well
  under the 0.25 px house tolerance).
- Everything else — canvas size, the two fleurs, wing discs, stem,
  dot order/alpha — is identical, deliberately: the registry version
  should not fork the film driver's pixels.
