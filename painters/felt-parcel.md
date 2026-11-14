# Painters: felt + parcel (seat W23)

Registry painters for the two bookshop texture generators the
scene format names but `scenefile.rs::resolve_texture` cannot yet
dispatch (W14/W15 gap lists; `scenes/sets/README.md` B8). Written
to the scene.rs registry conventions exactly: `paint_*(...) ->
(Vec<u8>, u32, u32)` RGBA bytes, the CPU `Painter` with its
coverage-AA rasteriser, the deterministic sin-hash `rnd` in f64,
doc comments citing the source recipe line-for-line.

| File | Generator | Source |
|---|---|---|
| `painters/felt.rs` | `felt` | `world.ts:93` (`T.felt`) + `world.ts:35` (`shade`) |
| `painters/parcel.rs` | `parcel` | `props.ts:159` (`PR.parcel` texture) |

Suggested dispatch arms for the integrator (mirroring `grass`):

```rust
"felt" => scene::paint_felt(str_field(spec, "base", ctx)?),
"parcel" => scene::paint_parcel(truthy(spec.get("label"))),
```

(`label` is absent in the data at hand — `parcel-plain` /
s10's `parcel` both carry `"label": false` explicitly.)

## Parameters

**felt** — `paint_felt(base: &str)`, canvas 256×256.
- `base`: hex colour, speckle shades derive from it. The source
  default is `'#1F5A45'` (`felt: (col = '#1F5A45')`); there is no
  size/repeat parameter in the painter — repeat lives on the
  texture clone (`canvasTex` rep, `[1, 1]` at both film sites).
- Algorithm: opaque base fill, then 3000 speckle rects of 2×2 px
  at `(rnd(i*3)*256, rnd(i*7)*256)`, each filled
  `shade(base, 0.85 + rnd(i)*0.3)`, i = 0..3000 in order (later
  speckles paint over earlier ones).

**parcel** — `paint_parcel(label: bool)`, canvas 256×256.
- The source's `parcel(w, h, d, label)` box dimensions never
  reach the canvas; only `label` does. All colours are literals
  in the recipe, so there is no base parameter (`base: '#C8935A'`
  in the data is descriptive).
- Algorithm: base `#C8935A`; 30 fibre streaks `#B07A48`, 40×2 px
  at `(rnd(i)*256, rnd(i*3)*256)`; tape cross `#E9D7A5` —
  `fillRect(120, 0, 16, 256)` + `fillRect(0, 120, 256, 16)`;
  at `label = true` only: label `#FFFDF4` `fillRect(150, 150,
  90, 70)` and stamp `#D8343F` `fillRect(214, 156, 20, 24)`.

## Usage sites in the film data

| Texture id | Where | Parameters |
|---|---|---|
| `felt-counter` | `scenes/sets/bookshop.scene.json`, `scenes/film/s10.scene.json` | base `#1F5A45`, 256×256, repeat [1,1] — counter top, `world.ts:316` (`interior()`) |
| `felt-desk` | same two files | base `#2E5A44`, 256×256, repeat [1,1] — desk top, `props.ts:164` (`desk()`), staged `shots.ts:231` |
| `parcel-plain` / `parcel` | bookshop set / s10 | label **false**, 256×256 — two desk parcels, `shots.ts:232–233`: `PR.parcel(0.5, 0.16, 0.36, false)` and `PR.parcel(0.44, 0.13, 0.32, false)` |

Both bookshop parcels share ONE texture (canvasTex cache key
`'parcelfalse'`); the two felt colours are two cache entries
(`'felt#1F5A45'`, `'felt#2E5A44'`). Texture-level settings from
`canvasTex` (world.ts:27) are renderer concerns, not painter
output: `SRGBColorSpace`, `RepeatWrapping` on both axes,
anisotropy 8.

## Colour-space notes — the felt shade is a three.js shade

`T.felt` shades its speckles with world.ts:35:

```ts
const shade = (hex, k) => { const c = new THREE.Color(hex);
  c.multiplyScalar(k); return '#' + c.getHexString(); };
```

That is **not** the canvas `shadeOf` (toon.ts:94) that
`scene::shade_rgb` mirrors, and not `scene::shade_lin` either.
Pinned from the vendored three r170 sources:

- `new THREE.Color(hex)` = `setHex`: bytes/255, then sRGB →
  linear working space with
  `SRGBToLinear(c) = c < 0.04045 ? 0.0773993808*c
  : pow(0.9478672986*c + 0.0521327014, 2.4)`.
- `multiplyScalar(k)` scales the **linear** components.
- `getHexString()` = `getHex`: linear → sRGB with
  `LinearToSRGB(c) = c < 0.0031308 ? 12.92*c
  : 1.055*pow(c, 0.41666) − 0.055` (exponent literal 0.41666,
  not 1/2.4), then `Math.round(clamp(255*c, 0, 255))` per
  channel.

`painters/felt.rs` reproduces this in f64 verbatim (private
`shade_three`). Measured consequences for `#1F5A45`:

| k | three shade (this painter) | sRGB byte multiply (`shade_rgb`) |
|---|---|---|
| 0.85 | (28, 83, 64) | (26, 77, 59) |
| 1.00 | (31, 90, 69) — exact round-trip | (31, 90, 69) |
| 1.15 | (34, 96, 74) | (36, 103, 79) |

The linear multiply darkens less and brightens less than the
byte multiply — mid-tones lift on the way down, highlights
compress on the way up. Same class of divergence as W14's
ceiling motif (#413427 three vs #493A2B via shade_lin), from the
same root cause.

**Known divergence this creates with the film path:**
`film.rs::paint_felt` shades with `scene::shade_rgb` (the byte
multiply), so the hand-authored film staging's felt speckles
differ from the source — and from this registry painter — on
every speckle whose k ≠ 1 (i.e. all of them). `film.rs::
paint_parcel` is op-for-op identical to `painters/parcel.rs`
at label = false (same fills, order, sizes), so the two paths
agree there.

Parcel involves **no** colour management: every fill is a
literal canvas hex, stored byte-exact; the painter output bytes
are the canvas bytes (modulo AA, below).

## Tests (all passing, run against the real scene.rs Painter)

Each module carries `#[cfg(test)]` tests: size/dims,
determinism (byte-identical repaints), plus —

**felt**
- Fully opaque: every alpha byte is 255 (analytic — opaque base
  at coverage 1, opaque speckles over it).
- Shade pins: k = 1 round-trips byte-exact; k = 0.85 gives
  (28, 83, 64) and provably ≠ `shade_rgb`.
- Pinned statistic: **50047** of 65536 pixels keep the exact
  base colour for `#1F5A45` (66 distinct colours total; desk
  green measures 50050 survivors / 68 distinct). Tolerance ±6 —
  measured, see below.

**parcel** (all exact, no tolerances — the geometry is
integer-aligned, so coverage is exactly 0 or 1)
- Pinned statistic: exactly **7936** tape-coloured pixels
  (16·256 + 16·256 − 16·16), at label false *and* true; the
  whole column x = 128 is tape-coloured.
- Label false draws zero label-paper and zero stamp pixels
  (analytic: no blend of the three source colours can reach
  either — stamp g = 52 < every source g ≥ 122).
- Label true: paper = 90·70 − 20·24 = 5820, stamp = 480.

## Unreproducible items (named, not hidden)

1. **Canvas AA vs Painter AA.** Chrome/Skia rasterises the
   fractional speckle/streak edges with its analytic coverage;
   the native `Painter` uses the M4 model (box SDF + 4×4
   supersample). Interior pixels are byte-identical; edge blend
   bytes can differ by a few counts on individual pixels. This
   is the standing painter approximation, shared with
   `paint_grass` — not a felt/parcel-specific gap.
2. **sin() last ulp.** `rnd` is `frac(sin(i*127.1 + 311.7) *
   43758.5453)` in f64: V8's Math.sin (fdlibm) and Rust's
   f64::sin (system libm) can differ by 1 ulp. Measured impact
   on felt: re-running the painter with every sin argument
   nudged ±1 ulp moves the base-survivor count by ≤ 4 pixels
   and changes ~107–149 bytes (~0.05%, all speckle-edge blends).
   Hence the ±6 tolerance on the felt pin. Parcel's pins are
   integer-aligned and unaffected.
3. **pow() last ulp** in the shade round-trip (fdlibm vs libm):
   could flip a speckle's shade by one byte step only if the
   product landed within ~1e-13 of a .5 rounding boundary. The
   tested pins come nowhere near: the closest channel (blue at
   k = 0.85: 63.5655) sits 0.066 byte-steps from its boundary,
   eleven orders of magnitude above pow ulp noise — verified
   numerically. Individual unpinned k values are not audited
   one by one; a rare one-byte shade difference on a single
   speckle is the bounded worst case.
4. **Premultiplied vs straight alpha.** Canvas stores
   premultiplied; `Painter` stores straight alpha and blends in
   f32. All felt/parcel fills are opaque, so this only touches
   the same AA edge pixels as item 1.
5. **Texture sampling** (anisotropy 8, mipmaps, repeat wrap) is
   a render-time property of the texture object, identical for
   any painter; it is not part of the painted bytes and is not
   reproduced here.
