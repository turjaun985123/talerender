# TaleRender scene format — v1 specification

Status: **v1 shipped with the M2 seat.** Loader: `talerender
--scene-file <path>` (`src/scenefile.rs`). Proof that the format
reproduces both hand-authored scenes: `docs/ab/scene-format/`
(**ab1: PSNR 95.17 dB, 0.000 % of pixels > 10, max Δ = 1 LSB;
dof-chart: PSNR 84.49 dB, 0.000 % > 10, max Δ = 1 LSB** — same
binary, same lavapipe adapter, hand-authored render as reference).

The format is deliberately *descriptive of the two scenes that
exist* (`--scene ab1`, `--scene dof-chart`). It covers the closed
geometry vocabulary those scenes use (A5: geometry flows through
cached `box()`/`cyl()`-style helpers in the source engine; the same
closed set appears here) and nothing speculative. New capabilities
enter through the versioning rule below, driven by a scene that
needs them.

## 1. Document skeleton

```jsonc
{
  "format": "talerender-scene",   // required, exact string
  "version": 1,                    // required, integer; loader rejects != 1
  "name": "human-readable scene name",
  "camera":  { ... },              // required
  "clear": "#D8E8DA",              // optional; scene clear colour (sRGB hex)
  "lighting": { "directional": { "direction": [0.5, 0.8, 0.6], "ambient": 0.35 } },
  "textures": [ ... ],             // optional
  "nodes": [ ... ],                // required (may be empty)
  "post": { "bokeh": { "focus": 10.0, "aperture": 0.0015, "maxblur": 0.006 } }
}
```

Unknown fields are **ignored** everywhere (so additive fields can
appear in files before every consumer understands them); unknown
*values* in discriminating positions — node `type`, geometry
`kind`, transform op, texture `generator`, scatter placement
`kind` — are **hard errors** with the node path in the message
(e.g. `scene.nodes[3].transform[1]: unknown transform op`).

## 2. Camera

```jsonc
"camera": {
  "fovY": 44,                      // required, degrees
  "near": 0.05, "far": 800,        // optional; engine defaults 0.05 / 800
  "eye": [0.83432, 2.64354, 5.28499],   // required
  "target": [0.35, 1.11477, -0.55],     // required
  "up": [0, 1, 0]                  // optional
}
```

Projection is the engine's: perspective with wgpu clip space
(z ∈ [0, 1]), view = look-at(eye, target, up). These are the only
camera semantics v1 has — no per-shot camera animation (scenes are
stills at a fixed staging; animation is M4+ territory, ADR-001
Decision 3).

## 3. Lighting

```jsonc
"lighting": { "directional": { "direction": [0.5, 0.8, 0.6], "ambient": 0.35 } }
```

Exactly the house light both native scenes use: a single
directional, Lambert term `max(dot(n, L), 0)`, plus a flat ambient
floor; colour-pipeline only. If `lighting` is absent the loader
uses these house values. Point lights (the camp fire), hemisphere/
IBL and shadow maps are **not** in v1 — see §10 (M5).

## 4. Nodes

Nodes render in **document order** (painter's order for the
transparent textured draws; the colour draws are opaque and depth-
tested, so their order is inert). Three node types:

### 4.1 `mesh`

```jsonc
{ "type": "mesh",
  "geometry": { "kind": "box", "size": [1.9, 0.05, 1.5] },
  "material": { "color": "#8A6B42" },          // colour pipeline
  // or
  "texture": "grass",                           // textured pipeline
  "transform": [ { "translate": [0, 1.55, -4.2] }, { "rotate_x": 0.42 } ] }
```

Exactly one of `material` / `texture`. Textured meshes are drawn
with alpha blending and back-face culling (the sky sphere relies on
the latter: it is viewed from inside).

**Geometry vocabulary** — the closed set the two scenes use.
Colour-pipeline kinds (positions + normals + vertex colours):

| kind | params | source-engine shape |
|---|---|---|
| `box` | `size: [w, h, d]` | `box()` — 24 verts, per-face normals |
| `cylinder` | `radiusTop`, `radiusBottom`, `height`, `segments` | `cyl()` + capped ring variant |
| `sphere` | `radius`, `widthSegments`, `heightSegments` | lat/long sphere |
| `cone` | `radius`, `height`, `segments` | cone with base cap |
| `disc` | `radius`, `segments` | flat disc facing +Y |

Textured-pipeline kinds (positions + UVs):

| kind | params | notes |
|---|---|---|
| `quad` | `width`, `height`, `y0` (default 0) | XY-plane panel; chart panels use `y0 = -height/2` |
| `flat-quad` | `width`, `depth` | XZ-plane quad (blob shadows) |
| `ground-disc` | `radius`, `segments`, `uvRepeat` (default 1) | UVs span `uvRepeat` tiles |
| `sky-sphere` | `radius`, `widthSegments`, `heightSegments` | inside-out (winding flipped) |
| `billboard` | `center: [x,y,z]`, `width`, `height` | camera-facing quad; corners are computed by the loader from the scene camera (basis law §7) |

**Material:**

```jsonc
"material": { "color": <colour-spec>, "roughness": 0.8 }
```

`roughness` is **carried, not consumed**: v1's colour shader has no
PBR term. The field is type-checked when present so M5 can bind it
without a format change. Colour specs:

```jsonc
"#RRGGBB"                                            // sRGB hex -> linear
{ "hex": "#RRGGBB", "shade": 0.75 }                  // hex, then shadeOf(k)
{ "hex": "#RRGGBB", "shadeHash": { "index": 3, "base": 0.92, "span": 0.16 } }
```

`shadeOf(k)` is the house grade: per-channel byte multiply in sRGB
space, `round(clamp(c,0,1) * 255 * k) / 255`, then linearised — the
same operation the skill's `shade()` performs and the hand-authored
scenes use. `shadeHash` sets `k = base + rnd(index) * span`
computed in **f64** and cast to f32 — exactly the expression the
hand-authored hill/canopy colour variation uses (§6 defines `rnd`).

### 4.2 `group`

```jsonc
{ "type": "group",
  "transform": [ { "translate": [-6.5, 0, -6] }, { "rotate_y": 0.0 } ],
  "children": [ /* nodes */ ] }
```

Children inherit `parent_world * local` (§5). Groups are how the
19-tree ring and the palm fronds are structured — the same nesting
the hand-authored builders use.

### 4.3 `scatter`

Procedural placement of many instances of one colour geometry,
merged into a single draw (the hand-authored scenes merge too —
they append into one vertex buffer).

```jsonc
{ "type": "scatter",
  "geometry": { "kind": "cone", "radius": 0.05, "height": 0.34, "segments": 4 },
  "material": { "color": "#4C8A3E" },
  "count": 900,
  "placement": { "kind": "annulus",
                 "angleHash": 1.31, "radiusHash": 2.7,
                 "radiusBase": 2.6, "radiusSpan": 20.0,
                 "angleScale": 6.283185307,
                 "center": [0, 0.15, -3] },
  "transform": [ ] }
```

For instance `i` (0-based), in f64:

```
a      = rnd(i * angleHash) * angleScale
r      = radiusBase + rnd(i * radiusHash) * radiusSpan
offset = center + (cos a * r, 0, sin a * r)     // cast to f32 per component
```

`angleScale` defaults to 2π. The ab1 lawn records `6.283185307`
— the literal the source staging uses, **not** full 2π — because
placement must be identical, not similar (§6). v1 scatter is
colour-pipeline only and supports this one placement law: it is
the only scatter the two scenes perform.

### 4.4 `code-ref` — the declared escape hatch

```jsonc
{ "type": "code-ref", "id": "ab1-onoda-card" }
```

A named splice of hand-authored draws, for content whose producer
is a later milestone. v1 defines exactly two ids, both from ab1:

| id | draws | why it is code |
|---|---|---|
| `ab1-onoda-card` | the character card (1 textured draw) | the character painter is **M4** (ADR-001 Decision 4 — fidelity-critical). Requires a texture with id `onoda-card` declared in the file. |
| `ab1-onoda-rifle` | the rifle (5 colour draws) | its transform derives from the painter's hand position via `syncAttachments` — painter data that does not exist until M4. |

Any other id is a hard error. A scene file that uses a code-ref is
still a valid v1 file; the README of the proof package declares
the usage, and §10 records the M4 replacement.

## 5. Transforms

A transform is a list of single-key ops, **composed left to
right**: starting from identity, `m = m * op` for each op in
order. A node's world matrix is `parent_world * local`. Ops
(angles in radians):

| op | value |
|---|---|
| `translate` | `[x, y, z]` |
| `rotate_x` / `rotate_y` / `rotate_z` | angle |
| `euler_xyz` | `[x, y, z]` — one op equal to `Rz * Ry * Rx` (three.js XYZ order, the `rotation.set` the lean-to logs and palm tips use) |
| `scale` | `[sx, sy, sz]` |

Association note: the hand-authored palm subtree nests its
matrices right-associated (`T * (Ry * (Rz * S))`) where the format
composes left-associated. The difference is f32 rounding at the
last bit; in the proof render it produced **no** pixel differences
(the residual pixels are elsewhere — see the proof README). The
format's law is the left-to-right one above; it is not negotiable
per-node.

## 6. The scatter hash — `rnd`

The engine's only randomness (A4), the sin-hash from the skill's
`lib.ts:25`, reproduced exactly:

```
rnd(i) = frac( sin(i * 127.1 + 311.7) * 43758.5453 ),   i in f64
```

`frac(x) = x - floor(x)`. Both the loader's scatter and the
`shadeHash` colour law use this function and nothing else. A unit
test pins instance 0 of the ab1 lawn against the formula written
out in the test.

## 7. Billboard basis law

For `billboard` geometry the loader derives the camera basis from
the scene's camera exactly as the hand-authored scenes do:

```
fwd   = norm(target - eye)
right = norm(cross(fwd, (0,1,0)))
upv   = cross(right, fwd)
corner(sx, sy) = center + right * (width/2 * sx) + upv * (height/2 * sy)
```

with corners BL/BR/TR/TL = (−1,−1)/(+1,−1)/(+1,+1)/(−1,+1), UVs
(0,1)/(1,1)/(1,0)/(0,1), triangles (0,1,2),(0,2,3). World up is
fixed at (0,1,0) in v1 — both scenes' billboards (clouds, caption)
are built this way.

## 8. Textures

```jsonc
"textures": [
  { "id": "grass", "generator": "grass", "base": "#5B9A48" },
  { "id": "caption", "file": "../docs/ab/onoda-s01-t150/assets/caption.png" }
]
```

A texture is either a **file** (any image the `image` crate reads;
path resolved **relative to the scene file's directory**) or a
**generator** — a named code painter, mirroring the source engine
where environment textures are ~26 code painters, not assets (A5).
The v1 registry is exactly the painters the two scenes call:

| generator | params | painter |
|---|---|---|
| `grass` | `base` (hex) | ground texture: base + hash speckle |
| `sky-gradient` | `top`, `horizon` (hex) | vertical sky gradient |
| `cloud` | — | soft cloud sprite |
| `blob-shadow` | — | radial contact shadow |
| `onoda-card` | — | the §28 character painter (M4 replaces) |
| `dof-chart-pattern` | `size` (default 512) | the M3 chart pattern |

A node references a texture by id, or by `{ "id": ..., "wrap":
"repeat" }` for repeat addressing (default `clamp`). sRGB textures
upload as `Rgba8UnormSrgb` with a linear filter — the §28/M3
configuration.

## 9. Post

```jsonc
"post": { "bokeh": { "focus": 10.0, "aperture": 0.0015, "maxblur": 0.006 } }
```

Presence of `post.bokeh` selects the **composer path**: HalfFloat
MSAA-4× scene target → packed-depth prepass → the transcribed r170
bokeh composite (M3) → Neutral tone mapping + sRGB output. Absence
selects the **direct path**: a single pass to an sRGB target, the
§28 configuration. Packed-depth storage defaults to `Rgba8Unorm`
on both paths' loaders exactly as in the hand-authored chart
(`TALERENDER_DOF_DEPTH_FORMAT=halffloat` restores the literal
BokehPass target; see the M3 package for why).

## 10. Versioning rule

- `version` is a required integer. The v1 loader accepts **only**
  `1` and rejects anything else with an explicit error — silent
  reinterpretation of a newer file is worse than a loud refusal.
- Within v1, additive fields may appear (unknown fields are
  ignored, §1). Any change that alters the *meaning* of an
  existing field, removes a field, or changes a law in §5–§7
  bumps the version.
- A future loader that supports v2 must keep loading v1 files
  bit-identically (the proof pairs in `docs/ab/scene-format/` are
  the regression anchor).

## 11. The f32 data law

JSON numbers are parsed as f64 and cast to f32 where the engine
computes in f32. **Staging values that were computed in f32 must be
written as the exact f32 values** (shortest-round-trip decimal is
fine). This is measured, not pedantry: the first generator pass
for `scenes/ab1.scene.json` wrote f64 approximations of the
hand-authored f32 literals (tree scales, blob offsets) and scored
**71.9 dB** with visible edge flips on near geometry; writing true
f32 values scored **95.2 dB, max Δ = 1 LSB**. Scene *authors* who
place objects by eye will never notice; scene *transcriptions* of
existing staging must honour the law.

## 12. Known limits of v1 (all measured or declared, none silent)

1. **The character is a code-ref** (§4.4). The format has no
   character node; ab1's card + rifle splice hand-authored draws.
2. **Roughness is carried, not consumed** (§4.1) — no PBR in v1.
3. **One light**: the single house directional (§3). The camp
   fire's point light, hemisphere/PMREM IBL and shadow maps from
   the source engine are absent — they were already absent from
   the hand-authored scenes (the §28 gap list), so the format
   loses nothing the scenes had.
4. **Scatter is annulus-only and colour-only** (§4.3) — the one
   placement law the scenes use.
5. **Textures are a named-painter registry or files** (§8) — no
   painter node graphs, no embedded raster data.
6. **No animation**: a scene file is one still staging. Camera
   moves, pose/acting channels, and `frame(t)` are ADR-001
   Decision 3 territory (pure functions over t), not v1 data.
7. **No atmosphere/fog block** — ab1's sky is a textured sphere
   and the chart has none; the source engine's atmosphere state
   transform (A3) arrives when a scene needs it, via the
   versioning rule.
8. **Draw order is document order** — transparent draws must be
   authored back-to-front, as the hand-authored scenes do.
9. **Caption text is a texture file**, not a text node — text
   layout belongs to the caption system, not the scene format.

## 13. What M4 / M5 must add

**M4 (painter):** a `character` node — card geometry + a painter
reference (character id, pose/acting channels at time t) —
replacing both `code-ref` ids; the rifle becomes an attachment
anchored to the painter's hand position. The loader's texture
registry gains the production character painters behind the same
generator mechanism.

**M5 (composer completion):** bind `roughness` and the rest of the
MeshStandardMaterial subset in use (A5: 181 `mat()` materials) to
real shader terms; a `lights` vocabulary beyond the house
directional (point lights with distance/decay — the fire);
per-node shadow cast/receive flags once shadow maps exist;
instanced scatter draws if a production scene's instance count
makes the merged-vertex approach wasteful (v1 merges, like the
hand-authored code).
