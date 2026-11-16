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
  "roll": 5                        // optional, v2 only (§14.4); degrees; default 0
}
```

Projection is the engine's: perspective with wgpu clip space
(z ∈ [0, 1]), view = look-at(eye, target, up). These are the only
camera semantics v1 has — no per-shot camera animation (scenes are
stills at a fixed staging; animation is M4+ territory, ADR-001
Decision 3). Format v2 adds one further camera field, `roll`
(§14.4); a v1 file carrying it is rejected, not reinterpreted
(§10 rule, the same gate as `keys` and `lights`).

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
| `sky-gradient` | `top`, `horizon` (hex), optional `ground` (hex, default `#BFE4F7`) | the source sky-dome shader sampled per row: `mix(hor, top, pow(clamp(y·1.6,0,1), 0.7))` above the horizon (y = cos(π(1−v))), flat `ground` below it |
| `cloud` | — | soft cloud sprite |
| `blob-shadow` | — | radial contact shadow |
| `onoda-card` | — | the §28 character painter (M4 replaces) |
| `dof-chart-pattern` | `size` (default 512) | the M3 chart pattern |
| `brick` | `base` (hex) | brick courses over shaded mortar (R-W5 port of world.ts `T.brick`) |
| `siding` | `base` (hex) | 8 board gradients + seams (R-W5 port of `T.siding`; byte-exact vs the TS op stream) |
| `dirt` | `base` (hex), `tracks` (bool, **default true** — the TS default) | speckled dirt + optional wheel ruts (R-W5 port of `T.dirt`) |
| `sand` | `base` (hex) | speckled sand + ripple strokes (R-W5 port of `T.sand`) |
| `facade` | `wall`, `kind`, `trim`, `sign`, `signText` (hex/str), optional `awning` (hex), `w` (default 5), `h` (default 7), `seed` (default 1) | shopfront composite over brick/siding tiling (R-W5 port of world.ts `facade()`); the sign-board glyphs are a PIL strip composited via `facade_with_text` — the engine has no Rust text rasteriser (the `sign` precedent) |

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

Since M5 the composer path also accepts two optional blocks
(inside `post`, alongside `bokeh` — both require `post.bokeh`,
because the composer is entered through it):

```jsonc
"bloom": { "threshold": 0.85, "strength": 0.0, "radius": 0.6 },
"grade": { "vignette": 0.18, "grain": 0.008, "warm": 0.02, "sat": 1.12, "time": 0.0 }
```

`post.bloom` runs the transcribed UnrealBloomPass between bokeh
and output; per-field defaults mirror the engine's fallbacks
(threshold 0.85, radius 0.6, strength 0 — an empty block is
inert). `post.grade` runs the engine's GRADE shader after output;
defaults mirror engine.ts (vignette 0.18, grain 0.008, warm 0.02,
sat 1.12, time 0). Absent blocks are skipped entirely:
the chain stays byte-identical to the M2/M3 form (proven — the
dof-chart as-data render is bit-identical to its committed
reference, and an identity bloom/grade variant renders
bit-identical to the plain chain; see docs/ab/composer/).

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
   the source engine are absent from the *format* — the
   hand-authored ab1 path gained the full rig at M5 (see
   docs/ab/composer/), so this is now a real format gap: a v1
   file cannot express the production light rig. Closing it is
   a schema question for a future version, not a silent field.
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

## 14. Format v2 (production integration)

v2 is additive over v1 under the §10 rule: v1 files load
bit-identically (the §9 proof pairs remain the regression
anchor), and a v1 file carrying v2 fields is **rejected**, not
reinterpreted. The v2 loader accepts versions 1 and 2 and
rejects everything else. v2 adds exactly what the Onoda
integration inventory (docs/integration/INVENTORY.md) demanded:

### 14.1 Time

Top-level `"time": <seconds>` — the shot time the file is
evaluated at (default 0). Camera keys (14.4) are evaluated at
this t; a `character` node's state is the state at this t.

### 14.2 Lights

Top-level `"lights"` — the production rig (engine.ts lightRig /
world.ts sunlight):

```jsonc
"lights": {
  "key":  { "color": "#FFF0D6", "intensity": 2.6,
            "position": [7, 12, 5], "target": [0, 0, -2],
            "shadow": true, "shadowBox": 17, "far": 80, "near": 0.5,
            "bias": -0.0006, "normalBias": 0.02 },
  "rim":  { "color": "#FFFFFF", "intensity": 0.4, "position": [-3, 3, -3] },
  "hemisphere": { "sky": "#CFE6FF", "ground": "#C48A5A", "intensity": 1.25 },
  "points": [ { "color": "#FF9A3C", "intensity": 4.6,
                "position": [-1.35, 0.6, -2.45], "distance": 7, "decay": 1.6 } ],
  "envIntensity": 0.25
}
```

Parsed and validated by the loader; consumed in full by the
film renderer (src/film.rs). The still renderers print the rig
in their header line and otherwise keep the v1 house light —
the same "carried, declared" status roughness had in v1 (§4.1).

### 14.3 Character node

```jsonc
{ "type": "character", "position": [0.35, 0, -0.55],
  "height": 1.75, "perspective": true,
  "look":  { "body": "#5C6644", "shade": "#878E6E", "pattern": "plain",
             "hat": "cap", "hatCol": "#4C5638", "neck": "none",
             "extra": ["glasses", "mustache"] },
  "state": { "mood": "determined", "armL": [10, 6], "armR": [16, -6],
             "look": [0, 0], "blink": 0, "talk": 0, "legs": "stand",
             "phase": 0, "lean": 1, "bob": 0, "squash": 0,
             "headTilt": 0, "headTurn": -0.025, "camPitch": 0.287,
             "sitOn": 0.34,
             "holdL": { "plate": "<texture id>", "x0": -160, "y0": -200 } } }
```

The node paints with the full drawToon port (src/toonfull.rs —
all moods, walk/sit legs, vest, tie/scarf, cap/bowler, glasses,
mustache, blink/talk/look, squash) and emits the blob shadow +
the card, replacing both v1 `code-ref` ids. Facing follows
`face()`: yaw toward the evaluated camera, plus the perspective
pitch (×0.45, clamped ±0.38) when `perspective` is true.
`sitOn` is metres, converted exactly as Toon#set converts it
(× 1024 / H). Held props are **plates**: pre-rendered RGBA
textures declared in `textures`, composited into the card at
the hand; the prop text (a paper's headline, a book's title)
therefore enters as data, as captions do (§12.9). Every prop
this film uses ignores its angle argument, so a plate is placed
by translation to the hand alone (declared in toonfull.rs).

### 14.4 Camera keys

```jsonc
"camera": { "fovY": 44, "eye": [...], "target": [...],
  "keys": [ { "t": 0.0, "eye": [...], "target": [...], "fovY": 44 },
            { "t": 5.91, "eye": [...], "target": [...], "ease": "linear" } ] }
```

Keys evaluate at `time`: eye/target/fovY interpolate between
the bracketing keys with the engine's smootherstep (motion.ts
camK easing); a key with `"ease": "linear"` eases its incoming
segment linearly (the S05a plane law — constant speed, no
easing). Outside the key range the end keys hold. Static
eye/target/fovY remain required as the base/fallback.

**Camera roll (R-A7).** The camera may carry `"roll"` — statically
on the camera block, and per-key in `keys`:

```jsonc
"camera": { "fovY": 44, "eye": [...], "target": [...], "roll": 5,
  "keys": [ { "t": 0.0, "eye": [...], "target": [...], "roll": 0 },
            { "t": 5.91, "eye": [...], "target": [...], "roll": 5 } ] }
```

- **Unit: degrees** — the camera block's angle unit. `fovY`, the
  only other angle in this block (and in keys), is degrees (§2);
  the §5 radians law governs node *transform ops*, a different
  object. The loader stores the value as f32 degrees and converts
  once, `roll.to_radians()`, at the same place `fovY` is converted
  (`view_proj`). Authored values obey the §11 f32 data law: write
  the exact f32 (shortest-round-trip decimal is fine) — `12`
  is exact; a value transcribed from the source engine's radian
  `ShotLook.roll` is converted by the bridge in f64 and written
  as the resulting f32 degrees.
- **Meaning.** Roll is a rotation about the camera's view axis,
  applied **after** the look-at basis is built and before
  projection — the native form of the source engine's
  `cam.lookAt(tgt); if (roll) cam.rotateZ(roll)` (shots.ts
  `createShoot`). With `(x, y, z)` the look-at (right, true-up,
  backward) basis and `θ` the roll in radians, the view matrix's
  basis becomes `right′ = cos θ·x + sin θ·y`,
  `up′ = −sin θ·x + cos θ·y`, `back′ = z`; in matrix order,
  `view′ = Rz(−θ) · view` (the view-side inverse of three.js's
  world-side `rotateZ`). **Sign: roll > 0 tips the horizon
  clockwise in the rendered frame — its right side drops** (the
  `ShotLook.roll` law: positive roll rotates the camera
  counter-clockwise about its view axis). The catalog range is
  the dutch range, 3–7° (`dutch(deg)` in shots.ts); the format
  itself imposes no clamp.
- **What roll cannot change.** Because the rotation's axis is
  the view axis, view-space z is untouched: the target stays on
  the image centre, every point's depth is preserved, and
  `post.bokeh.focus` (a distance along that axis) means the
  same thing rolled as unrolled. Eye, target, fovY and the
  framing's centre are roll-invariant; only the frame's
  orientation about its centre rotates.
- **Keys.** A key without `roll` falls back to the base camera's
  `roll`, exactly as a key without `fovY` falls back to the base
  `fovY`. Between keys, roll interpolates as a plain number with
  the segment's eased `u` (smootherstep, or linear when the end
  key says `"ease": "linear"`) — **no angle wrapping and no
  normalisation**: 350 → 10 interpolates through 180, and an
  author who wants the short way writes 350 → 370. Outside the
  key range the end keys hold, as for every other channel.
- **Additivity.** Absent `roll`, and `roll: 0` (including `-0`),
  are the pre-amendment camera exactly: the loader's no-roll
  path performs no extra arithmetic, and renders are
  byte-identical to the pre-amendment binary (the R-A7 A/B
  proof). Static `roll` next to `keys` is the base/fallback,
  exactly as static eye/target/fovY are.

Proof: `scenes/character-v2.scene.json` (the S01 staging as
data: character node + keys + the noon JG rig) and the loader
unit tests (`v2_keys_lights_and_character_node`,
`v1_file_with_v2_fields_is_rejected`).

### 14.5 Integration rulings (2026-11-14, wave Batch 1)

**R1 — `lights` blocks carry POST-atmosphere values.** Chrome
renders what `applyAtmosphere` leaves behind: it overwrites the
key colour/intensity, the rim intensity and both hemisphere
colours from the atmosphere preset (env.ts), replacing the set's
own `sunlight()` values. Scene files therefore record the
post-atmosphere rig (noon: key #FFF0D6 ×2.6, rim #FFFFFF ×0.4,
hemisphere #CFE6FF/#C48A5A ×1.25 — env.ts `noon`), never the
pre-atmosphere sunlight values (#DCECD2/#46603A ×1.15). The fire
point light likewise records its **rendered** basis, not its
constructor arguments: `fireLevel` (shots.ts) overwrites the
intensity every frame with
`(4.6 + 0.7·sin(17t) + 0.5·sin(7.3t)) · level`, so the block
carries intensity basis **4.6** with distance 7 / decay 1.6
(the constructor's 5.0 is never rendered). The §14.2 example
above is corrected to match. (Cross-check: t = 1.5, level 1 →
4.3519, the M5 ab1 value.)

**R2 — `attachments` lives ON the character node.** A held
attachment (e.g. `{ "kind": "rifle", "hand": "R",
"offset": [0, 0.03, 0.07] }`) is a property of the character
instance that carries it; the loader consumes it from the
character node. There is no top-level `attachments` block.
(The offset is the staging's syncAttachments card-space offset;
the hand anchor comes from the painted card, as in film.rs.)

**R3 — GRADE grain time is 0 for the whole film.** No path in
the source ever sets the grade's time uniform, so the grain is
a fixed spatial pattern across all frames. The film renderer
pins the grade time to 0 (src/film.rs, the composer's GradeParams
time field). Any time-animated grain would diverge from Chrome
on every frame by the grain term.

**Loader status addendum (2026-11-14, post-Batch-6).** The
scene-file loader now consumes everything this section
declares: the `lights` rig (carried; consumed in full by the
film evaluator), the character node incl. `attachments`
(rifle, hand-anchored), camera keys, `post.grade` with the
canonical keys **sat / warm / exposure / time** (`exposure`
feeds the OutputPass uniform; `saturation` is NOT a key —
files are renamed to `sat`), the top-level **`fog` block**
{color, near, far} (composer path: three's fog_fragment over
the packed-depth prepass, verified against the analytic fog
chart to ≤1 sRGB code at every panel), and the full texture
registry (grass, sky-gradient, cloud, blob-shadow, onoda-card,
dof-chart-pattern, planks, wallpaper, felt, parcel, picture,
sign, solid). The direct (non-composer) path applies no fog;
every fog-carrying production file routes through the
composer via `post.bokeh`.

**R5 — Scene files name atmospheres by PRESET; the driver
resolves (set × preset).** A scene file's `atmosphere` field
carries the preset name (`noon`, `fog`, `golden`, …) — the
staging's own vocabulary (`applyAtmosphere(scene, 'noon')`).
The effective rig/grade/tint for a shot is a function of the
shot's SET and the preset together (the same preset lands
differently on the jungle set's sun than on the bookshop's
interior rig), so the merge is resolved by the evaluator at
render time (film.rs's rig builders are exactly this
resolution), never baked into the scene file. The
`filmLooks` table in scenes/atmospheres.json is a derived
(set × preset) index kept for validation and diffing; it is
not the scene-file vocabulary. All 14 segments resolve
through it with no missing entries.

**R4 — Source wins over the evaluator on staging constants.**
Where a scene file and src/film.rs disagree with shots.ts,
shots.ts is ground truth and the wrong side is fixed, never
averaged. Batch-1 instances, both fixed in film.rs: S06's
companion walks use `walkBob(ph, 5)` (film.rs had 6.0), and
POSES.mourn's lean 5 / headTilt −4 / bob 4 apply in BOTH the
standing and sitting branches of S06 (the set() arm override
[16,−6]/[8,5] likewise applies in both); sitting adds only
legs/sitOn 0.34 and the settle squash. Preserved source
mismatch (do not "fix"): in S06 the scene fog is replaced by
Fog(#C9CFC2, 4.5, 20) in the same draw call, while the
character tint's fog colour stays the fog preset's #E6E9EA
(fogMix 0.35).

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

---

## 15. Format v2 extensions: prefabs and clips

Two additive extensions ride the existing format version
(`"version": 2`), under the §10 rule: neither adds a field to an
existing construct's meaning, neither changes an existing field,
and each one's fully processed form is an ordinary v2 document.
What they add is **reserved keys**: the top-level `"prefabs"`
and `"clips"` blocks, the node-level `"prefab"` key, `"pid"`
(meaningful only inside prefab templates), and the instance
fields `"clip"` / `"clipTime"` on nodes and on the `camera`,
`lights`, `post`, `fog` blocks. Prefabs (§15.2–15.5) are a
load-time macro over node subtrees. Clips (§15.6–15.8) are named
keyframe data evaluated at the file's `time` (§14.1). §15.1 is
the rule that makes both safe to reserve; §15.9 records the
reconciliation rulings where the two extensions meet the
evaluator (`src/anim.rs`); §15.10 is the adoption order.

### 15.1 The reservation guard (fail loud, never silently ignore)

A v2 loader that does not implement one of these extensions
must **hard-error, naming the unsupported extension**, when it
encounters that extension's reserved keys — a top-level
`"prefabs"` or `"clips"` block, a `"prefab"` key in a node
position, or a `"clip"` field on any node or block. Silently
dropping them is forbidden. This is a deliberate carve-out from
§1's "unknown fields are ignored", and it exists because §1's
rule is unsafe for exactly these keys: ignoring a `prefabs`
block while honouring its instances is incoherent, ignoring an
instance deletes scene content, and ignoring a `clip` field
silently freezes animated staging at its static values. The
precedent is already in the format: a v1 file carrying v2
fields is *rejected*, not reinterpreted (§14), and a v1 file
carrying `prefabs` or `clips` is rejected the same way.

Measured state of today's loader (`src/scenefile.rs`), so the
guard's work is named exactly: a top-level `"prefabs"` block is
never read — silently ignored, per §1 — while an instance node
already fails loud, because `walk_nodes` requires a `type` on
every node, but with a misleading missing-`type` message that
never mentions prefabs. The guard replaces both behaviours with
one named error per extension ("prefab extension not
supported", "clips extension not supported"). The guard applies
to every consumer that walks nodes, including `film.rs`'s own
node walk, whose behaviour must not be assumed to match the
still loader's. Until the guard and the corresponding
implementation land together (§15.10), the two reference
validators are the only consumers that understand these
extensions, and no production scene or film data may use them.

The reservation posture also covers `events` inside a clip
definition (§15.6): events are not staging a renderer may
drop — they are sound cues for the mix. A consumer that
evaluates clips but does not implement event emission must
hard-error on a clip carrying `events`, naming the gap,
exactly as for the extensions themselves.

### 15.2 Prefabs: grammar

A **prefab** is a named, parameterised node subtree:

```jsonc
"prefabs": {
  "<name>": {
    "description": "optional, documentary, ignored",
    "params": { "<slot>": <declaration>, ... },   // optional
    "node": <node>                                // required
  }
}
```

- The block may appear **inline** in a scene file, and/or in a
  **library file** — a JSON object `{"format":
  "talerender-scene", "version": 2, "prefabs": {…}}` with no
  `nodes`, which is data for the scope and is never rendered. A
  consumer assembles one merged scope from the inline block
  plus every library it is given. **No shadowing:** a name
  defined by two sources is a hard error naming both sources —
  the same file must mean the same thing everywhere.
- A prefab has a **single root node**, so an instance always
  occupies exactly one node position. Multi-object sets are a
  `group` root (the base format's own idiom, §4.2).
- Prefab names are any non-empty JSON object key. Names carry
  no path semantics (no `/`, no imports); a library is included
  whole or not at all.

**Param declarations** are a closed key set — `type`
(required), `required` (optional boolean), `default`
(optional), `description` (optional, documentary):

```jsonc
"params": {
  "height":   { "type": "number", "required": true },
  "capColor": { "type": "color",  "default": "#C23B2E" },
  "label":    { "type": "string", "default": "" }
}
```

Every param is exactly one of: `required: true` (the instance
must supply it) or carrying a `default` (the instance may). A
required param with a default, and a param that is neither, are
both definition errors — a slot must always expand to a value.
A `default` must type-check against its own declaration and
must be **literal**: it may not contain a `$param` slot
anywhere inside it.

**Slots.** A slot is an object with exactly one key,
`{ "$param": "<name>" }`, standing for a **whole JSON value**
anywhere a value may appear inside the template subtree: as a
field value, as an array element (one component of a `size` or
`translate` vector), as a whole array/object, and as the value
of a nested instance's `params` entry (forwarding, §15.3).
Slots are lexically scoped — a slot inside prefab P's template
names a param of P, never of a caller or a nested prefab — and
a slot anywhere in a scene file outside a template is an
error. A slot may not be an object key, may not be the value of
`"prefab"` (instantiation targets are literal strings, so the
reference graph of §15.5 is static), and may not appear inside
a param declaration or a default. A slot-shaped object with
any additional key is not a slot; it is an ordinary object and
fails base validation downstream if it lands somewhere it
makes no sense.

**Instances.** An instance is an object in a node position
carrying `"prefab"`:

```jsonc
{ "prefab": "marker-post",
  "params":    { "height": 1.2, "postY": 0.6, "capY": 1.24,
                 "capColor": "#2E5FC2" },
  "overrides": { "cap": { "material": { "roughness": 0.35 } } },
  "transform": [ { "translate": [2.5, 0, -2] } ],
  "name": "east gate post" }
```

The key set is closed: `prefab` (required, literal string),
`params`, `overrides` (§15.4), `transform` (a base-format
transform list), `name`. **Any other key is a hard error** — a
deliberate departure from §1, mirroring §1's own other half
(unknown *values in discriminating positions* are already hard
errors): an instance is a *call*, and a misspelled key in a
call would otherwise be silently dropped while the render
proceeds with defaults. An instance has **no `"type"` key** —
`"prefab"` is its discriminator. Instances may appear anywhere
a node may appear (the top-level `nodes` array, `group`
children, and — after substitution — in `node`/`nodes` param
positions); they may not appear where the base format does not
take nodes (`attachments`, `geometry`, `material`, `textures`,
`lights` are not node positions). Slots, being value-level,
*may* appear inside those sub-objects within a template.

**`pid`.** Any node inside a template may carry
`"pid": "<local-id>"`, existing only so `overrides` can
address the node (§15.4). A `pid` is not an `id` — the base
format's `id` fields name entries in closed, loader-level
registries and are never touched by this extension. Pids are
scoped to their prefab's **direct scope** (the template
subtree, not descending into a nested instance's interior),
must be unique within that scope, and are **stripped during
expansion** — a `pid` never reaches the renderer, a saved file,
or another prefab.

### 15.3 Prefab params: typing, substitution, and the f32 law

Param types are the JSON shapes the base format already
carries — nothing is coerced, converted, or computed:

| type | value must be | notes |
|---|---|---|
| `number` | a JSON number (never a boolean) | subject to the f32 law below |
| `string` | a JSON string | verbatim |
| `color` | a string matching `#RRGGBB` | the base format's hex colour-spec string (§4.1); the `{hex, shade}` object form is an `object` param, not a `color` |
| `boolean` | `true` / `false` | |
| `vec3` | an array of exactly 3 numbers | positions, sizes, targets |
| `array` | any JSON array | element shape is the author's responsibility; the expanded result is base-validated |
| `object` | a JSON object that is not a slot and not an instance | e.g. a material or colour spec |
| `node` | one node object (`type` or `prefab` present) | enables wrapper prefabs |
| `nodes` | an array of node objects | enables container prefabs |

There is deliberately no `integer` type (JSON has one number
type), no enum type (closed value sets belong to the base
validators of the expanded output), and no expression type.

**Substitution is verbatim — no arithmetic, no string
interpolation, no expressions.** A slot is replaced by a deep
copy of the effective param value, bit for bit, with no
re-rounding; there is no operation in this extension that
could produce a new number. Consequently the §11 f32 data law
needs no extension and applies **after expansion, unchanged**:
a value that lands, after expansion, under `lights`, `camera`,
or a character node is subject to exactly the same exact-f32 /
source-literal rule as if it had been written inline, and
`tools/film/validate_film.py` enforces it on the expanded
document unchanged. The cost is stated plainly: substitution
cannot derive values — a template needing both a post's height
and its centre (`height / 2`) declares **two params**, staged
accordingly. The moment slots could compute, param values
would stop being "numbers exactly as the base format carries
them", the f32 law would need an expression semantics, and
expansion would stop being auditable by reading the file.
Authors who need derived staging compute it where staging is
authored — as they do today — and pass the results as params.

**Forwarding.** A nested instance inside a template may supply
a slot as a param value, forwarding the outer prefab's param
inward. The forwarded slot's declared type in the outer prefab
must match the inner param's declared type exactly, with one
widening: a `color` may feed a `string` (every colour is a
string; the reverse is false and is an error). A template must
be **self-contained**: a nested instance that omits an inner
required param is a definition error, not something a
scene-level caller can repair.

### 15.4 Prefab overrides and transforms

There are exactly two override channels, with different
mechanics on purpose: **params replace wholesale; overrides
deep-merge.**

**Params** (§15.2): the effective value is the instance's
supplied value if present, else the declared default. A
supplied `object` value is *not* merged into its default field
by field — a half-merged material spec is invisible in the
file and unauditable. Per-field variation of an object is
separate params, or belongs in `overrides`.

**Overrides** are pid-addressed partial node objects,
deep-merged onto the template node carrying that `pid`
**before** param substitution:

```
merge(dst, src):
  for each key k in src:
    if k is forbidden (below)           -> hard error
    if src[k] is null                   -> hard error (no deletion)
    if dst[k] is a $param slot          -> hard error
    if dst[k] and src[k] are both plain objects (neither a slot)
                                        -> merge(dst[k], src[k])
    else                                -> dst[k] = deep copy of src[k]
                                           (arrays and scalars replace
                                           wholesale; arrays are never
                                           concatenated or merged by index)
```

Arrays replace wholesale because the base format's arrays are
positional or ordered — vectors, transform op lists (§5
composes left to right), camera keys. An override may **add**
keys the template node does not carry (adding `roughness` to a
material that omitted it is legitimate restyling); added keys
are still subject to the forbidden list, and the merged result
is base-validated after expansion like any authored node.
Overrides address only the target prefab's direct scope — the
interior of a nested instance is not addressable from outside;
restyle inside a nested prefab by calling that prefab directly
or exposing a param.

**What may NOT be overridden** (at any depth inside the
partial object):

| Forbidden | Why |
|---|---|
| `type` | The node's discriminating value — changing it changes *what the node is*, not how it is staged; that is a different prefab. |
| `kind` (geometry, attachment) | Same: a closed-vocabulary discriminator. |
| `generator` (texture) | Same: names a code painter from a closed registry. |
| `id` | Registry identity (texture ids, code-ref ids); the loader resolves textures eagerly in array order before walking nodes, so renaming by instance would make that table context-dependent. |
| `prefab` | Re-targeting an instance would make the §15.5 reference graph dynamic and uncheckable. |
| `pid` | The addressing mechanism itself; re-labelling during the merge that uses the labels is incoherent. |
| `transform` | Transforms compose (§15.4 below); they are never replaced. |
| any key whose template value is a `$param` slot | Two channels writing one position is an ordering ambiguity; slot positions are written by `params` only. |
| deletion (a `null` value) | The base format has no null semantics anywhere; variants that omit a field are separate prefabs. |

**Transforms compose; they never replace.** A template root's
own `transform` is part of the prefab's authored staging and
is protected by the forbidden list. An instance's `transform`
is its placement in the parent space. When an instance carries
a `transform`, the expanded root is therefore wrapped:

```jsonc
{ "type": "group",
  "transform": <the instance's transform>,
  "children": [ <expanded template root, its own transform intact> ] }
```

World = parent × instance × root-local, by the base format's
ordinary group rule — no new transform semantics anywhere.
When an instance carries no `transform`, no wrapper is
emitted. An instance's `name`, when present, is set on the
outermost expanded node (the wrapper if there is one); it is
documentary, like the scene `name`, never an identity.

### 15.5 Prefab expansion and cycles (normative)

Expansion happens at load, before any node is walked. Prefabs
are a load-time macro: the renderer, the telemetry, and the
film driver only ever see expanded plain v2 nodes. A
conforming consumer performs these steps, in order:

1. **Assemble the scope** (§15.2): inline `prefabs` + all
   supplied libraries. Duplicate name → hard error.
2. **Validate the library eagerly** — every definition, used
   or not: declaration grammar, defaults literal and
   type-correct, every slot declared, no slot naming a prefab,
   pids unique per direct scope. (Eager validation follows the
   loader's existing precedent: textures are resolved eagerly,
   in array order, at load.)
3. **Build the reference graph** (definition → names its
   template instantiates, literally) and reject cycles (below).
4. **Validate every instance** in the scene and in templates:
   closed key set, known target, required params present, no
   undeclared params, values type-check (§15.3), overrides
   legal (§15.4).
5. **Expand** depth-first, in document order. For each
   instance: (a) effective params = defaults overridden
   wholesale by supplied params; (b) deep-copy the template
   root; (c) apply overrides by pid; (d) substitute every slot
   with a deep copy of its effective param value (the same
   param used twice yields two independent copies); (e)
   recursively expand instances in the resulting subtree
   (nested instances, and instances arriving via `node`/`nodes`
   params); (f) strip every `pid`; (g) wrap for the instance
   transform and set `name` (§15.4).
6. **Proceed with the base format exactly as if the expanded
   document had been authored**: the expanded tree contains no
   `prefab`, no `pid`, no `$param` — the consumer asserts
   this — and the top-level `prefabs` block is consumed and
   dropped (it does not propagate into any saved or derived
   document).

Document order is preserved throughout (instances expand in
place), so painter's order, texture resolution order, and
every other order-sensitive behaviour of the base format are
unaffected.

**Cycles.** The reference graph of step 3 is **static**:
instantiation targets are literal strings (§15.2 forbids slots
there), so every edge is visible without expanding anything. A
cycle — a prefab whose template transitively instantiates
itself — is a hard error, and the error prints the cycle path
(`loop-a -> loop-b -> loop-a`). Instance data cannot create a
cycle: a `node` param value is a finite JSON tree; it can
contain instances, but each is expanded against the acyclic
definition graph, so expansion terminates. A depth guard is
still required in implementations (the reference validator
uses 64) as belt-and-braces: a consumer that expands without
running the graph check first must fail, not hang or overflow,
on adversarial data.

### 15.6 Clips: the data model

A **clip** is a named set of keyframe tracks. Clips generalise
the §14.4 camera-key shape — the engine's one keyframe form
that is already data — to every channel the engine animates,
and let any number of nodes/blocks instance a clip with a time
mapping. All additions are additive: a top-level `clips`
block, plus `clip` / `clipTime` fields on things that already
exist. Numbers everywhere obey the §11 f32 data law.

```jsonc
"clips": {
  "<clip-name>": {                       // name: any JSON object key
    "duration": 5.907,                   // seconds, > 0, REQUIRED
    "tracks": [ Track, ... ],            // REQUIRED, non-empty
    "uses": [ "<clip-name>", ... ]       // OPTIONAL layering
  }
}
```

A **track**:

```jsonc
{ "target": { "node": "camera", "channel": "eye" },
  "keys":   [ Key, ... ] }               // REQUIRED, at least one key
```

`target.node` is one of the **reserved targets** `camera`,
`lights`, `post`, `fog` — the scene-level blocks — or the
**`name` of a node in the same file** (searched recursively
through `children`). Node names colliding with a reserved
target are rejected by the validator: the node would be
unaddressable.

A **key**:

```jsonc
{ "t": 2.6, "value": 0.4, "ease": "easeOutCubic" }
```

- `t` — clip-local seconds; `0 ≤ t ≤ duration`; strictly
  increasing within the track.
- `value` — shaped by the channel kind (§15.7): a number
  (scalar), `[x, y, z]` (vec3), `"#RRGGBB"` (colour), or an
  enum string (step channels: `mood`, `legs`). The field name
  is `value` — the name in the evaluator's `Key { t, value,
  ease }` and in the film's existing object-form keys alike.
- `ease` — OPTIONAL, default `smoother`. Names the curve of
  the segment **ending at this key** (the incoming segment) —
  the same placement as §14.4, where the loader reads the ease
  from the segment's end key, and the same direction as
  poseTrack, whose move is governed by the key being moved
  *into*. The first key's `ease` has no incoming segment and is
  ignored, exactly as the §14.4 loader ignores it. (The
  evaluator stores easing the other way round; the loader
  remap is a ruling, §15.9.)
- A single key is a constant track. Before the first key and
  after the last, the end values **hold** — the §14.4 camera
  law and poseTrack's hold behaviour agree on this.

**Layering (`uses`).** A clip may name other clips in `uses`;
an instance of the clip then applies its own tracks **plus**
the used clips' tracks, transitively. The `uses` graph must be
acyclic (self-use counts), every name must resolve, and the
layered set may not define the same target+channel twice (no
silent precedence).

**Instantiation.** Any node (recursively), and the `camera`,
`lights`, `post`, `fog` blocks, may carry:

```jsonc
"clip": "look-around",
"clipTime": { "offset": 0.5, "rate": 1.0, "loop": "ping-pong" }
```

- `clip` — the clip name; must be defined in the same file.
  The same clip may be instanced by many nodes — that reuse is
  the point of naming clips.
- `clipTime` — OPTIONAL; every field defaults: `offset`
  (default `0.0`, seconds added to the mapped time — the clip
  is entered `offset` seconds in), `rate` (default `1.0`, must
  be `> 0`, time scale), `loop` (default `"clamp"` — what
  happens outside `[0, duration]`; §15.8 gives the formulas).
  The three `loop` values are the kebab spellings of the
  evaluator's `Wrap::{Clamp, Loop, PingPong}`.
- **Conflict rule:** `camera` may not carry both `clip` and
  `keys` (one camera, one time source). Which form a shot uses
  is the author's choice; carrying both is data rot, and no
  precedence is defined.
- What an instance *means* against static fields: the clip's
  tracks **replace** the target's static value at evaluation
  time (the static `state` / `eye` / rig remains the at-rest
  staging and the t-before/after reference, exactly as §14.4's
  static `eye`/`target` remain the required base/fallback next
  to `keys`).

**Events (`events`).** A clip definition may carry an optional
`events` array — sound cues the clip fires when a driver
evaluates it:

```jsonc
"events": [ { "t": 0.25, "cue": "takeoff", "gain": 0.8 } ]
```

- Each entry is an object with closed keys `{t, cue, gain,
  dur}`; `t` is clip-local, `0 ≤ t ≤ duration`; `gain` / `dur`
  are optional numbers, passed through verbatim.
- `cue` is a closed vocabulary: the audio event names
  (`takeoff`, `land`, `fall-impact`, `trip`, `slip`, `recover`,
  `lose-balance`, `roll`, `push-up`, `pop`, `inhale`) or a mix
  FX name (an FX-named event is a literal placement in the
  mix). Anything else is a load error naming both vocabularies.
- Events are **not staging**: evaluation never folds them into
  the document. They leave the evaluator only through its
  event side channel, in the shape `{t, cue, gain?, dur?}`
  (absent optionals omitted), feeding the mix's clip-events
  sidecar — never the authored cues file.
- Time law: an event at clip-local `te` under an instance's
  `clipTime` sits at scene-local `(te − offset) / rate` (the
  inverse of §15.8's mapping), and at absolute time
  `segment start + scene-local` inside a film segment. Events
  whose scene-local time is negative never fire. Drivers
  collect events over half-open windows `[w0, w1)`, so
  contiguous per-frame windows receive each event exactly
  once.
- **Looping:** an instance whose `clipTime.loop` is `loop` or
  `ping-pong` emits no events — periodic playback has no
  single firing time. The law is per-instance: the same clip
  may emit under a clamped instance and stay silent under a
  looping one.
- A `uses`-layered clip contributes its events transitively,
  its own first — the same rule as its tracks.
- Gains are final values in the data. Where the TS engine
  scales a gain by impact at runtime, the bridge bakes the
  scaled value at authoring time; the evaluator never
  computes one.

**Cross-instance conflicts.** Two instances whose clips write
the same target+channel are not an error (the no-duplicate
rule binds only inside one clip's layered `uses` set): the
document fold applies instances in document order and the
**last instance wins**. Authors who need a different result
layer with `uses` instead.

### 15.7 Clips: the channel vocabulary

Every channel below is a field the engine animates today,
cited to the code that consumes it (the survey lives in
`FORMAT-EXT-CLIPS-W15.md` §3). A channel not in these tables
is an unknown channel and fails validation. Kinds: **S**
scalar, **V** vec3, **C** colour hex, **Step** enum string (no
interpolation).

**camera** (target `"camera"`): `eye` (V), `target` (V),
`fovY` (S) — the §14.4 keys. `roll` (§14.4) is not a clip
channel in this amendment; adding `camera.roll` (S) to this
vocabulary is a separate evaluator change (eval15 channel
table + fold), not a loader change.

**lights** (target `"lights"`) — the §14.2 rig, carrying
post-atmosphere values per **R1**:

| channel | kind | | channel | kind |
|---|---|---|---|---|
| `key.intensity` | S | | `hemisphere.intensity` | S |
| `key.color` | C | | `hemisphere.sky` | C |
| `key.position` | V | | `hemisphere.ground` | C |
| `key.target` | V | | `envIntensity` | S |
| `rim.intensity` | S | | `points[i].intensity` | S |
| `rim.color` | C | | `points[i].color` | C |
| `rim.position` | V | | `points[i].position` | V |
| | | | `points[i].distance` / `points[i].decay` | S |

(`i` is the index into the file's `lights.points` array, as
written: `points[0].intensity`.) Not channels: `key.shadow*` /
`bias` — shadow *configuration*, never time-varied anywhere in
the engine.

**post** (target `"post"`):

| channel | kind | | channel | kind |
|---|---|---|---|---|
| `bokeh.focus` | S | | `grade.sat` | S |
| `bokeh.aperture` | S | | `grade.warm` | S |
| `bokeh.maxblur` | S | | `grade.exposure` | S |
| `bloom.threshold` | S | | `grade.vignette` | S |
| `bloom.strength` | S | | `grade.grain` | S |
| `bloom.radius` | S | | | |

**`grade.time` is deliberately NOT a channel.** **R3** pins
the GRADE grain time to 0 for the whole film (§14.5); a track
on it would animate grain away from Chrome on every frame. A
track targeting `post.grade.time` is rejected **by name**,
citing the ruling.

**fog** (target `"fog"`): `near` (S), `far` (S), `color` (C)
— the top-level fog block (§14.5).

**character nodes** (target = the node's `name`) — the §14.3
`state` fields. Pair fields split into components, because the
film's own data already tracks them separately:

| channel | kind | | channel | kind |
|---|---|---|---|---|
| `position` | V | | `lean` | S |
| `armL.0` / `armL.1` | S | | `bob` | S |
| `armR.0` / `armR.1` | S | | `squash` | S |
| `look.0` / `look.1` | S | | `headTilt` | S |
| `blink` | S | | `headTurn` | S |
| `talk` | S | | `camPitch` | S |
| `phase` | S | | `sitOn` | S |
| `height` | S | | `mood` | Step: the 17 strings of `Mood::from_str` |
| | | | `legs` | Step: `stand walk run sit jump` |

Strictness note: the loader's mood parsing silently falls back
to neutral on an unknown string; clip validation **fails** an
unknown `mood`/`legs` value instead — clips are new data, and
a typo'd mood should be caught at the gate, not rendered as
neutral. The same posture applies to unknown channels: a
channel name is a discriminating value, and §1 already
hard-errors on unknown discriminators.

**other nodes** (target = the node's `name`) — the transform
channels the film's data already keys: `translate` (V),
`scale` (V), `rotate.x` / `rotate.y` / `rotate.z` (S). Rotation
is spelled as euler scalars — see §15.9.

**Easing names.** The spellings are exactly the set the
evaluator's `Easing::from_name` accepts; the validator's list
is copied from it:

| name (canonical) | accepted aliases | engine curve |
|---|---|---|
| `smoother` (default) | `smootherstep` | the engine's smootherstep — camK, poseTrack, §14.4 camera keys |
| `linear` | — | raw lerp — the one ease name v2 already spells (§14.4) |
| `easeOutCubic` | `eo` | film `eo()` |
| `easeInOutCubic` | `eio` | film `eio()` |
| `easeOutBack` | `eob` | film `eob()` |
| `step` | — | hold — discrete channels; **required** on `mood`/`legs` keys |

### 15.8 Clips: evaluation semantics

For an instance at scene time `t`, with clip duration `D`:

```
raw = t * rate + offset
clamp:     tau = min(max(raw, 0), D)
loop:      tau = raw mod D            (result in [0, D))
ping-pong: m = raw mod 2D;  tau = m if m <= D else 2D - m
```

Per track at `tau`: find the bracketing keys `k0 (t0, value0)`,
`k1 (t1, value1)`; outside the key range, hold the end value.
Inside: `u = (tau - t0) / (t1 - t0)`, eased by `k1.ease` (the
arriving key's ease, §15.6), then:

- **scalar:** `value0 + (value1 - value0) * eased` (computed in
  f64, cast to f32 — the film's motion formulas are f64
  throughout; the evaluator's f32-native evaluation of the same
  law can differ by ~1 ulp, which is why evaluator adoption is
  A/B-gated per site, §15.10);
- **vec3:** componentwise, same law;
- **colour:** convert both hex values to linear (the engine's
  colour pipeline, §4.1/§8), lerp componentwise in linear, as
  the film's colour mixing does — never lerp sRGB bytes;
- **step:** the value of the latest key with `t <= tau` (no
  easing, whatever the key says — validation already requires
  `step`).

A clip is a pure function of `t` — no state, no frame history
— matching the format's stance that animation is `frame(t)`.
Loop modes do not change this (the formulas above are
closed-form).

**What must NOT migrate into clips** (the adoption map's
negative half; the positive half — `camera.keys`, poseTrack
channels via the conversion below, object-form node keys,
step data — is surveyed in `FORMAT-EXT-CLIPS-W15.md` §5.1):

- **`post.grade.time`** — R3 is a law, not data (§15.7).
- The fire-flicker formula over the **R1 basis 4.6** —
  a procedural law, not keys; baking it into keys would sample
  a continuous law into data and break the R1 cross-check.
  (Keying the fire's *level* envelope over a shot would be
  legitimate new authoring; it is not this film's data.)
- `walkPhase` / `walkBob` / `segLerp` position drivers —
  derived from travelled distance / shared drivers; they are
  functions of another channel's integral, not independent key
  data. The *position endpoints* migrate; the gait derivation
  stays code.
- `blink` directors, the `talk()` waveform, `settle()` shakes,
  `camPitchOf` — seeded or closed-form procedural systems; a
  seed is not a key table, and keying them would freeze today's
  sampling of a law.
- Pose bundles, stage swaps, and film-manifest segment timing
  (`startS`/`endS`, caption chunks) — pose data, structural
  swaps, and driver bookkeeping respectively, not scene
  animation.

**The poseTrack conversion (exact, mechanical).** A poseTrack
moves into key `k` over the `moveSecs` *before* `t_k` and
holds otherwise; a clip track interpolates across the *whole*
segment. So each poseTrack key `[t, v, m]` with `m > 0`
converts to **two** clip keys: a hold key `{t: t - m, v:
<previous value>}` (unless one already sits there) and the
arrival key `{t: t, v: v, ease: "smoother"}`. Keys with no
moveSecs convert the same way with the poseTrack default
`m = 0.35`. The converted track evaluates **identically** to
`pose_track()` at every `t`; this is a re-spelling, not a
re-timing.

### 15.9 Reconciliation rulings

Four points where the extensions meet the evaluator
(`src/anim.rs`) were left open by the source specs. They are
decided here, so the format text and the code agree from the
start.

**Colour and step channels get their own evaluator lanes.**
*Rationale: the evaluator's `AnimValue` lanes at the §15
landing cover `f32`, `[f32; 3]`, and `Quat` — scalar and vec3
channels map onto them directly, but colour and enum-string
values must never route through numeric interpolation.*
Decided: the evaluator gains a colour lane, `Color`, whose
values are **linear-space `[f32; 3]`**. The loader converts
each colour key's authored `#RRGGBB` to linear by the format's
existing hex→linear law (§15.8) **before** the track carries
it, so a `Color` track never holds an sRGB value; sampling
interpolates componentwise in linear, and consumers receive
linear values exactly as the film's folded light colours are
carried today. Step channels (`mood`, `legs`) are sampled by a
**step sampling mode**: hold the leaving key's value, take the
latest key with `t <= tau`, no interpolation. Their string
values ride a **side channel keyed like the numeric tracks**
(same target+channel names) — they never enter a numeric
track. This extension lands with the clips loader adoption
(§15.10); until then no evaluator lane is claimed to exist.

**Ease is stored on the arriving key; the loader remaps it.**
*Rationale: the format's placement is the §14.4 law (ease on
the key a segment ends at), while the evaluator's `Key.ease`
is easing-to-next (the segment a key leaves) — one of them
must move at the boundary, and the format's data is the side
that already exists.* Decided: a clip key's `ease` names the
segment **ending at that key**, exactly as in §14.4. When the
loader builds evaluator tracks it remaps **format
`key[i+1].ease` → anim `key[i].ease`** for every segment; the
format's first key has no incoming segment, so its `ease` is
dropped, and the last evaluator key's `ease` is never used.
This is the identical rule the evaluator's adoption map
already states for §14.4 camera keys, applied to clips
unchanged.

**Duration lives on the clip; wrap lives on the instance.**
*Rationale: one duration bound per clip is what validation
needs (a key beyond it is a single named failure), and two
instances of one clip may legitimately map time differently,
so the wrap mode cannot live on the clip or the track data.*
Decided: the format keeps `duration` on the clip and the wrap
mode on the instance (`clipTime.loop`), and the loader bridges
the placement mechanically — the clip's `duration` becomes
each built track's duration (the evaluator's
`Track::with_duration`), and the instance's `loop` becomes
each track's `Wrap` (`"clamp"` → `Wrap::Clamp`, `"loop"` →
`Wrap::Loop`, `"ping-pong"` → `Wrap::PingPong`). The
evaluator's per-`Track` duration/wrap remains its internal
shape; no format file ever spells them per-track.

**Rotation channels are euler scalars; there is no quaternion
channel.** *Rationale: the format spells what the scene data
carries and what the engine consumes — node rotation in the
data is euler components, and the math layer's rotation path
is Euler → matrix.* Decided: clip rotation channels are
`rotate.x` / `rotate.y` / `rotate.z` (kind S, §15.7). The
evaluator's quaternion lane (`Track<Quat>`, sampled via
`Clip::sample_quat`) remains an evaluator-internal capability
for code-side use; §15 defines no `rotation` channel, and a
future quaternion channel would be a separate amendment with
its own data evidence — not something authors may assume from
the evaluator's type list.

### 15.10 Adoption order

The extensions land in this order, and no step is skipped:

1. **Reservation guard first** (§15.1): the named hard errors
   on the reserved keys, in `scenefile.rs` and in `film.rs`'s
   own node walk. Gate: until this lands, `check_prefabs.py`
   and `check_clips.py` are the only consumers of the
   extensions, and **no production data uses them**.
2. **Expansion and evaluation next**: the §15.5 expansion
   ported into the loader ahead of its node walk; the clips
   loader and the §15.9 evaluator lanes (`Color`, the step
   side channel) with the §15.9 remaps. Gate: the fixture sets
   become loader tests — the valid fixtures load (and the
   valid prefab scene renders its expanded tree), each invalid
   fixture fails with its corresponding named error
   (`check_prefabs.py`: 6 checks over `scenes/fixtures/prefabs/`;
   `check_clips.py`: 5 checks over `scenes/fixtures/clips/`,
   each bad fixture failing exactly one). Evaluator adoption at
   existing call sites (§14.4 camera keys first) is separately
   A/B-gated per site, because the evaluator's f32-native
   evaluation can differ from the current f64 paths by ~1 ulp
   (§15.8).
   **Status: LANDED for the scene-loader path 2026-11-15**
   (Chrome-replacement wave, Batch 1): `src/eval15.rs`
   implements §15.5 expansion and §15.6–15.8 evaluation,
   including the §15.6 event side channel, and is consumed
   from `scenefile::load`; its in-file tests mirror the
   reference checkers' fixture checks. The loader's §15.1
   top-level guard checks are retired into this consumption;
   the `walk_nodes` guard remains as a residue assertion.
   Evaluator adoption at film-driver call sites (the s12
   retirement) remains a separate gated landing under this
   step.
3. **Production data last**: a production scene or film file
   may use prefabs or clips only after steps 1–2 have landed,
   and both validators run over it in tools-ci (exit 0 = pass,
   1 = a check failed, 2 = input error — the repo tool
   convention), with `validate_film.py` still enforcing the
   §11 f32 law on the expanded result.


---

## Appendix — the film driver's data file (F1/F2, Wave 2)

Not a scene document: the `--film` driver's per-film data
(`load_film_data` in `src/film.rs`), documented here because it
shares the format's discipline (f32 law §11 applies to its
staging values; unknown keys are tolerated per §1, with the
strictness ratchet watching them).

- **Entry name:** `film.json` is the generic entry; `onoda.json`
  is accepted as the fallback for the existing film (F1). The
  file sits beside an `assets/` directory.
- **Required keys:** `duration`, `anchors`, `lines`,
  `captions` (each with `s`, `e`, `text`, `fit`, `strip` — the
  strips are pre-rendered PNGs in `assets/`, the standing
  caption contract), and since the F2 migration (W53):
  `calendarYears` (year stems in flip order; `[]` is legal),
  `calendarDark` and `sign` (assets-relative file names). A
  file missing any of them fails by name — the driver defaults
  nothing to any particular film's values.
- **`assets/plates.json` is a manifest:**
  `{"format": "talerender-plates", "version": 1, "plates":
  [{"name", "file", "origin": [x, y]}]}`. Its entries are the
  plate set the driver loads — any count, any names the
  staging holds by name. `origin` is required per entry
  (no default); duplicate names are a load error; any other
  shape is rejected by name (no dual-read of the legacy bare
  object). Schema: `docs/integration/f2-migration/
  plates-manifest.schema.json`.
