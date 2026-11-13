# A5 — Environment, Props & Content Catalog Inventory

**Scope (read-only):** `template/src/core/props.ts` (431 lines), `template/src/core/world.ts`
(740 lines, set builders + shared texture/material helpers), `library/index.json`,
`library/LIBRARY.md`. `engine.ts`/`env.ts` are another seat's; `world.ts` is covered here
only for its set-builder functions and the helpers they use.

**Method:** full read of both source files; counts from
`grep -o 'new THREE\.<Class>' | sort | uniq -c`, helper-call counts from grep,
catalog counts from a Python `json.load` + `collections.Counter` pass over `index.json`.
Runtime object counts per set/prop are derived from loop bounds and call sites in the code
(labelled ≈; they are arithmetic on the source, not measured renders).

**Headline totals**

| | props.ts | world.ts | total |
|---|---|---|---|
| Exported functions | 27 | 36 | **63** |
| Direct `new THREE.*Geometry` sites | 36 | 57 | **93** |
| `new THREE.InstancedMesh` sites | 1 | 4 | **5** |
| Direct `new THREE.*Material` sites | 7 | 16 | **23** |
| `mat()` call sites (→ MeshStandardMaterial) | 98 | 83 | **181** |
| `canvasTex()` call sites (→ CanvasTexture) | 9 | 17 (+1 def) | **26** |
| Direct `new THREE.CanvasTexture` sites | 2 | 2 | **4** |

---

## 1. Geometry / material / texture workload

### 1.1 Geometry classes constructed (direct construction sites)

| Class | props.ts | world.ts | total | Notes |
|---|---|---|---|---|
| PlaneGeometry | 10 | 21 | **31** | windows, posters, grounds, fronds, flags, water |
| SphereGeometry | 11 | 11 | **22** | hills/dunes/drifts (scaled unit spheres), sandbags, lamps, balloon |
| ConeGeometry | 2 | 5 | **7** | pine tiers, watchtower roof, peak(), cowcatcher |
| BoxGeometry | 2 | 4 | **6** direct | see helper note below — boxes dominate at runtime |
| CylinderGeometry | 5 | 1 | **6** direct | see helper note |
| CircleGeometry | 0 | 6 | **6** | meadow/mountain flat stage discs, ponds, crater bowls |
| TorusGeometry | 2 | 2 | **4** | lantern ring, bucket handle, desert ribcage, crater rims |
| IcosahedronGeometry | 0 | 3 | **3** | tree/fgLeaves foliage blobs, meadow flowers (instanced) |
| ExtrudeGeometry | 3 | 0 | **3** | house gable, barn gable, boat hull (all from `THREE.Shape`, 4 `new THREE.Shape` sites in props.ts) |
| BufferGeometry (custom) | 0 | 2 | **2** | bunting flag triangles (`setFromPoints`), stars point cloud |
| ShapeGeometry | 1 | 0 | **1** | tent end triangles |
| TubeGeometry | 0 | 1 | **1** site | `wire()` — CatmullRom catenary, 20 segments; called in loops (§5) |
| DodecahedronGeometry | 0 | 1 | **1** | desert rocks |

**The helper reality:** most runtime geometry flows through two local helpers in each file —
`box()` (props.ts 71 call sites, world.ts 31) and `cyl()` (23 / 16) — which share geometries
through a dimension-keyed `GEOCACHE` (`boxGeo`/`cylGeo`, identical contract in both files:
key = exact dimensions + segment count; geometries are never mutated after creation).
So the native geometry vocabulary is small: **parameterised boxes and cylinders dominate**,
plus a fixed set of primitive types above. Only Box/Cylinder go through the cache — Sphere,
Cone, Icosahedron etc. are constructed fresh at every call site (matters in §5).

Other geometry facts a native engine must reproduce:

- `new THREE.Mesh` sites: 35 (props) + 52 (world); `new THREE.Group` sites: 35 + 20;
  `new THREE.Sprite` sites: 1 + 2 (but loops multiply them — §3/§5).
- **CPU-displaced ground planes:** meadow `PlaneGeometry(200,200,100,100)` = 10,201 vertices,
  z displaced by sin/cos field, `computeVertexNormals()` (world.ts, meadow); mountain
  `PlaneGeometry(220,220,90,90)` = 8,281 vertices displaced by a smoothstep radial field.
  A native port needs the exact displacement formulas, not a heightmap asset.
- Sky dome: `SphereGeometry(380, 32, 16)` (world.ts `sky()`).
- Multi-material meshes (material arrays of 6) are used for textured-face boxes: house door,
  signBoard, mainStreet/cityStreet buildings (facade on one face), interior picture,
  chickCrate front — a native material system must support per-face material slots on boxes.
- `snapshot()` (world.ts) creates a `THREE.WebGLRenderTarget(1024, 768, { samples: 4,
  type: HalfFloatType })` and renders a scene into a texture — the one render-to-texture
  user in these files.

### 1.2 Materials

Direct construction sites: **MeshStandardMaterial 11** (4 props / 7 world),
**MeshBasicMaterial 7** (2 / 5), **SpriteMaterial 3** (1 / 2), **ShaderMaterial 1**
(the sky dome, world.ts), **PointsMaterial 1** (stars). On top of that, **181 `mat()` call
sites** each produce a `MeshStandardMaterial` (default `roughness 0.85, metalness 0`);
`mat()` caches flat, map-less specs in a `MATCACHE` keyed by the full parameter set, and
always builds textured materials fresh (three call sites mutate their texture afterwards —
boardwalk/floor/side-wall map rotation/repeat in mainStreet/interior — so a native port must
preserve per-mesh texture transform state, not share it).

Material features actually exercised: `map`, `color`, `roughness` (0.1–1), `metalness`
(0–0.7), `emissive` + `emissiveIntensity` (lamps, windows, lantern, train lamp),
`transparent` + `alphaTest` (signpost arms, palm fronds, chick cut-outs), `side: DoubleSide`,
`depthWrite: false` (smoke, foam, cloud deck, sprites), `fog: false`, `toneMapped: false`.
No MeshPhysical/Lambert/Phong anywhere in these two files.

### 1.3 Textures — everything is procedural; there are no image assets

Every texture in the environment/prop system is painted at runtime on a 2D canvas.
Two mechanisms:

- **`canvasTex(key, w, h, drawFn, repeat)`** (world.ts:41) — cached by string key in a
  module-level `TEX` map; returns a *clone* with its own repeat (RepeatWrapping both axes,
  SRGBColorSpace, anisotropy 8). 26 call sites across the two files.
- **Direct `new THREE.CanvasTexture`** — 4 sites: the `canvasTex` implementation itself,
  cloud sprite texture (world.ts), train smoke texture and the platform-scale dial
  (props.ts; the dial is repainted at runtime by `platformScale().set(lbs)` with
  `tex.needsUpdate = true`, memoised on unchanged value).

Named painters in world.ts — the `T` texture library (9): `planks, wallpaper, brick,
siding, dirt, grass, sand, snow, felt` (mostly 512×512, wallpaper/felt 256×256), plus
`facade(name, …)` (per-building shopfront, 110 px per metre → ~550×770 px typical,
up to ~990×2200 px for a cityStreet block) and `signBoard(text, …)` (200 px per metre).
Inline painters: world.ts — interior picture, palm frond, cloud deck (1024×1024),
beach foam, spaceLite earth, spaceLite flag (6); props.ts — house window, house door,
signpost arm, mail-sack label, parcel wrap, poster, crate holes, chick, barn door (9).

**Native implication:** the texture workload is a 2D-canvas rasterisation workload
(fills, gradients, bezier strokes, text with font fitting loops), not an asset-decoding
one. A native engine needs a Canvas2D-equivalent painter (or pre-bake) for ~26 painter
functions; several painters embed *text* (signs, posters, facades, the scale dial), so
font rendering is on the critical path for environments, not just captions.

---

## 2. Lighting & shadow usage inside the builders

### 2.1 Shadow flags

The `box()`/`cyl()` helpers set **`castShadow = receiveShadow = true` on every mesh they
create** — 102 `box()` + 39 `cyl()` call sites across the two files, so the default
posture of the whole prop/world system is "everything casts and receives".

Explicit flag assignments beyond the helpers:

| | props.ts | world.ts |
|---|---|---|
| `castShadow = true` sites | 16 | 11 |
| `receiveShadow = true` sites | 6 | 29 (ground planes, walls, dunes, ponds…) |
| `castShadow = false` sites | 1 (rails ballast bed) | 2 (pole insulators; `hills()` traverse strips cast from its trees) |
| `receiveShadow = false` sites | 0 | 1 (`cloudDeck` — deliberate: nothing casts from 55 m up) |

Sprites never cast. Instanced meshes: rails ties and interior slips set
`receiveShadow = true`; meadow flowers/tufts and farm cones set neither.

### 2.2 Lights created by builders

- **props.ts: none.** No light objects in any prop.
- **world.ts `sunlight()`** (world.ts:186) is called by 11 of the 12 sets and delegates
  rig construction to `lightRig()` in engine.ts (out of scope); world.ts itself only
  post-tunes it: shadow camera `far = 80`, `bias = -0.0006` (world.ts:189). It also assigns
  `scene.environment = E.envMap` and `scene.environmentIntensity` (default 0.25).
- **world.ts `interior()` builds its own rig** (the exception):
  - `THREE.DirectionalLight` '#FFE4B8' 2.4 (world.ts:358), `castShadow = true`,
    shadow map **2048×2048**, ortho box ±8, near 0.5 / far 30, bias −0.0006,
    **`shadow.radius = 4`** (soft/PCF-soft filtering is in use).
  - `THREE.HemisphereLight` (world.ts:360), intensity 1.1.
  - `THREE.PointLight` fill (world.ts:361): intensity 6, distance 12, decay 1.4.
  - 2× `THREE.PointLight` bulbs (world.ts:366): intensity 3, distance 6, decay 1.6,
    each paired with an emissive sphere mesh + a brass cord cylinder.
  - `scene.environment` also set here (intensity 0.35). Return shape exposes
    `{ key, fills[] }` so env.ts atmospheres can dim the bulbs.
- `snapshot()` forces `renderer.shadowMap.needsUpdate = true` before its render.

### 2.3 Fog & sky

- **Fog: no builder in either file ever sets `scene.fog`.** Fog appears only as opt-outs:
  `fog: false` on the sky ShaderMaterial, cloud/earth SpriteMaterials and stars
  PointsMaterial. Fog state is owned elsewhere (engine/env) — a native port must not
  expect sets to configure it.
- **Sky construction:** `sky()` (world.ts) — a BackSide `SphereGeometry(380)` with a
  custom **ShaderMaterial** (the only shader in these files): uniforms `top/hor/gnd`,
  gradient by normalised y (`pow(clamp(y*1.6), 0.7)` above horizon, flat ground colour
  below), `depthWrite: false, toneMapped: false, fog: false`, `renderOrder = -10`.
  Called by 11 sets (all but `interior`, which uses a flat `scene.background` colour).
- Clouds: one shared 512×256 canvas texture, drawn as `THREE.Sprite`s (5–14 per set,
  `openSky` generates 14 procedurally); plus `cloudDeck()` — a 500×500 plane with a
  1024×1024 painted alpha texture at deck height in openSky.

---

## 3. Set catalog (world.ts — 12 sets)

All sets return `{ scene, sun, … }`; v5 sets also return a `stage { y, r }` (a known-flat
acting disc at the origin). `sun` is the engine LightRig via `sunlight()` except interior.

| Set | One-line description | ≈ objects |
|---|---|---|
| `mainStreet(E, opts)` | Small-town street along −z: facade-textured shop boxes both sides, boardwalks, poles+wires, lamps, bunting, hills, cars, crowd extras | **≈450–500 meshes** (24 buildings, 14 poles ×7, 10 lamps ×3, 48 wire tubes, ~54 bunting meshes, hills ≈148, 3 cars ≈39, 9 Toons) |
| `interior(E, opts)` | Wallpapered shop/home/office/classroom room, window in back wall, counter, own light rig (§2.2), 4 presets | ≈45 meshes + 140 instanced floor-litter slips |
| `meadow(E, opts)` | Displaced-vertex grass plain, hills, trees, fence, instanced flowers/tufts | ≈200 meshes (hills ≈148) + 1,600 flowers + 4,000 tufts (instanced) |
| `openSky(E, opts)` | Altitude set: sky dome, cloud deck at −55 m, 14 free cloud sprites, **no ground/stage** | ≈17 |
| `farm(E, opts)` | Dirt-yard stage (r 9), barn + farmhouse, 2 fences, crop rows | ≈120 meshes + 120 instanced crop cones |
| `desert(E, opts)` | Sand stage, 4 dunes, cacti, rocks; optional oasis (pond + palms), ribcage bones | ≈25 |
| `snowfield(E, opts)` | Snow stage, 4 drifts, 6 pines; optional cabin (house + snow roof caps), frozen pond | ≈55 |
| `beach(E, opts)` | Sand stage sloping to waterline, sea plane, 3 returned foam strips, boat, palms | ≈40 |
| `cityStreet(E, opts)` | mainStreet grammar at modern scale: tall facade blocks (h 12–20) at x ±11.5, lamps, cars | ≈90 (≈16 buildings, 14 lamps, 2 cars) |
| `mountain(E, opts)` | Displaced rocky valley, flat stage disc, 4 `peak()` backdrops, slope pines at true terrain height | ≈30 + 8,281-vert ground |
| `fieldCamp(E, opts)` | Serious-tone camp: ≤5 tents, 2 sandbag walls, crates/barrels, watchtower, wired poles (hard rule: no weapons/gore dressing) | ≈110 (28 sandbag spheres, tower ≈20) |
| `spaceLite(E, opts)` | Moonscape diorama: stars, 5 craters, painted-Earth sprite, lander, flag; vacuum lighting, no atmosphere | ≈35 + 420 star points |

Reusable world pieces (also exported, also catalogued): `sky`, `clouds`, `sunlight`,
`hills` (4 flattened spheres + 6 trees each), `tree` (round/pine/cypress), `pole`, `wire`,
`lamp`, `barrel`, `crate`, `car` (≈13 meshes), `fence` (post every 1.5 m + 2 rails),
`bunting`, `extras` (background Toon crowd), `palm`, `cactus`, `peak`, `stars`,
`cloudDeck`, `fgLeaves`, plus `facade`/`signBoard` texture painters and `snapshot`.

## 4. Prop catalog (props.ts — 27 builders)

Mesh counts are per single call at defaults (≈, from call sites/loops).

| Prop | One-line description | ≈ meshes |
|---|---|---|
| `house(opts)` | Farmhouse, gabled extruded roof, porch, hinged door (`.door`) | ≈35 |
| `signpost(arms)` | Post + painted directional arm planes (canvas text) | 3 |
| `platformScale()` | Brass platform scale; `.set(lbs)` repaints dial canvas | 5 |
| `rails(len)` | Two steel rails + ballast; ties are one InstancedMesh (⌊len/0.7⌋ = 285 @ len 200) | 3 + 285 inst. |
| `train()` | Steam loco + U.S. MAIL car; `.update(t)` spins wheels, animates 14 smoke sprites | ≈60 |
| `cubbies(w, h)` | Mail-cubby wall: divider grid + seeded letters (≈65% of cells) | ≈60 @ 3×1.8 m |
| `mailSack(s, col)` | Squashed-sphere sack + tie + painted label | 3 |
| `parcel(w,h,d)` | Wrapped parcel with painted tape/label texture | 1 |
| `desk(w)` | Felt-topped desk | 5 |
| `poster(lines, opts)` | Painted notice plane; `.mesh` exposed | 1 |
| `chickCrate(n)` | Crate + n bobbing 2D chick cut-outs; `.update(t)` | 2 + n (6) |
| `aircraft(opts)` | ~8 m prop plane, spinning prop, hinged door; staged flying | ≈20 |
| `parachute(opts)` | Canopy = partial sphere shells + 8 strut lines; origin at harness | 11 |
| `windsock(opts)` | 5 m pole + open cone sock (`.sock` handle) | 3 |
| `barn(opts)` | Gambrel-roof barn, cross-brace painted door, loft hatch | ≈12 |
| `tent(opts)` | Prism canvas tent, ShapeGeometry ends, guy-line struts | ≈8 |
| `sandbags(n)` | n squashed spheres, two staggered rows | n (12) |
| `watchtower(opts)` | Splayed strut legs, railed cab, ladder, cone roof | ≈20 |
| `boat(opts)` | Extruded rowboat hull, rim + benches; origin at waterline | 4 |
| `balloon(opts)` | Striped envelope (2 sphere shells) + wicker basket (`.basket`) | 7 |
| `lander(opts)` | Moon lander: gold box, 4 strut legs + pads, dish | ≈15 |
| `table(opts)` | Table, top at exactly 0.74 (sitOn convention) | 5 |
| `chair()` | Chair, seat at 0.45 | 8 |
| `shelf(opts)` | Bookcase, up to 27 seeded book boxes | ≈30 |
| `lantern3D()` | Attach-first lantern, origin at handle ring | 4 |
| `bucket(opts)` | Attach-first bucket, torus handle arc | 2 |
| `toolbox(opts)` | Attach-first toolbox | 5 |

Plus 6 catalogued `prop3d` pieces living in world.ts: `palm` (≈13 meshes), `cactus` (6),
`peak` (2), `stars` (1 Points), `cloudDeck` (1), `fgLeaves` (6).

Several props are *animated/stateful*, not static geometry — a native runtime must keep
their update contracts: `train().update`, `chickCrate().update`, `aircraft().update`
(prop spin), `platformScale().set` (texture repaint), doors as hinge Groups
(house/aircraft), `parachute().harness`, `balloon().basket`, `windsock().sock`.

---

## 5. index.json structure & the content catalog

`library/index.json` = `{ "version": 1, "items": [ … ] }`, **190 items**. Each item:
`{ id, kind, name, snippet, captionSafe, preview, optIn, since, file, call }` — i.e. a
*catalog of code entry points*, not of loadable assets.

Item counts by kind (Python `Counter` over `items[].kind`):

| kind | count | | kind | count |
|---|---|---|---|---|
| held | 36 | | shot | 13 |
| prop3d | 33 | | set | 12 |
| sfx | 21 | | atmosphere | 10 |
| mood | 19 | | look | 5 |
| pose | 17 | | motion | 5 |
| tool | 16 | | music | 3 |

`since`: 4.0 → 92 items, 5.0 → 98. Items by owning file (top): toon.ts 55 (all 36 held +
19 mood), props.ts 27 (all prop3d builders defined there), mix.py 25, world.ts 18
(12 set + 6 prop3d), poses.ts 17, shots.ts 13, env.ts 10.

**What a native runtime must load / ignore:**

- **Load (authoring truth):** `id`, `kind`, `call`, `file`, `snippet` — these identify
  which builder/function implements the item. The runtime needs the *builders themselves*
  ported; index.json only enumerates them. For this seat's area: 12 sets + 33 prop3d.
- **Ignore at runtime:** `preview` — all 190 items have one, **all are `.jpg`**
  (`library/previews/<kind>-<id>.jpg`), used for docs/QA browsing only; `captionSafe`
  (190/190 present, human staging guidance text); `name` (display label); `optIn`
  (true for 8 items — an authoring flag, e.g. opt-in engine features); `since`
  (version bookkeeping).
- LIBRARY.md adds conventions a port should respect: items are added by a single patch
  touching index.json + area doc + preview together, and `library_check` validates the
  catalog — the catalog is a registry contract, not a scene format.

---

## 6. Instancing & repetition analysis

### Already instanced (5 `InstancedMesh`, ≈5,745 instances in a combined scene)

| Where | Geometry | Count | Per-instance color |
|---|---|---|---|
| `rails()` ties (props.ts) | Box 0.25×0.1×2.2 | ⌊len/0.7⌋ = 285 @ len 200 | no |
| `interior()` floor litter | Plane 0.16×0.1 | 140 | yes (4 colours) |
| `meadow()` flowers | Icosahedron r 0.05 | 1,600 | yes (5 colours) |
| `meadow()` grass tufts | Cone 0.025×0.16 | 4,000 | no |
| `farm()` crop rows | Cone 0.16×0.5 | 120 | no |

### Not instanced — ranked opportunities

1. **Trees (top opportunity).** `tree()` builds 6 meshes (round: 1 cached-cylinder trunk +
   5 foliage blobs) or 4 (pine: trunk + 3 cones) — and the foliage geometry is built
   **fresh at every call** (`new THREE.IcosahedronGeometry` / `ConeGeometry` per blob;
   only Box/Cylinder go through GEOCACHE). `hills()` plants 6 trees per hill × 4 hills =
   **24 trees ≈ 144 meshes** in every set that calls it (mainStreet, meadow); meadow adds
   6 more trees, snowfield 6 pines, mountain 5 pines, farm 3, desert 2–3 palms
   (palm ≈ 13 meshes each, also uncached frond planes). A street+meadow frame carries
   roughly **35+ trees ≈ 200 draw calls** that collapse to a handful of instanced draws
   (trunk, blob, cone, frond) in a native engine.
2. **Street wires.** mainStreet creates **48 separate TubeGeometry meshes** (each a
   unique 20-segment catenary tube) for pole-to-pole wires, plus ≈51 individual
   `BufferGeometry` triangle meshes for bunting flags across 3 spans (fieldCamp adds
   2 more wire tubes). Mergeable into static geometry or instanced flag quads.
3. **Fences.** `fence(len)` = a post box every 1.5 m (12 posts @ len 16) + 2 rails;
   used by meadow (×1) and farm (×2). Posts share cached geometry, so this is pure
   draw-call count: ≈42 meshes → 1 instanced mesh per fence.
4. **Cubbies letters.** Default `cubbies(3, 1.8)` fills ≈65% of a 9×7 grid ≈ **41
   individual letter boxes** (+18 divider boxes) in one prop.
5. **Shelf books.** Up to **27 individual book boxes** per `shelf()` (3 levels × ≤9),
   each a cached-geometry box with a per-book colour material from a 6-colour cycle.
6. **Sandbags.** `n` fresh `SphereGeometry` meshes per wall (default 12; fieldCamp
   places 2 × 14 = 28) — geometry not cached, material shared.
7. **Buildings (texture-bound, not instanceable).** mainStreet ≈24 and cityStreet ≈16
   box meshes, each with a **unique facade CanvasTexture** (≈550×770 px ≈ 1.7 MB RGBA;
   cityStreet up to ~990×2200 ≈ 8.7 MB) → roughly **40–90 MB of facade texture** per
   street scene. They can't instance (each shopfront differs), but a native engine
   should budget texture memory/streaming for this, and could atlas or downscale
   distant facades.

Already efficient, no action: clouds (1 shared texture across all sprites), train smoke
(14 sprites, 1 shared texture), stars (single `THREE.Points`, 400–420 points),
`extras()` crowd (9 Toons in mainStreet — character system, another seat).

---

## 7. Bottom line for the native port

- Geometry vocabulary is tiny and closed: parameterised boxes/cylinders (cached by
  dimension) + 11 primitive classes + 2 CPU-displaced ground planes + 1 extrude/shape
  family. The workload is **many small meshes**, not big geometry: the heaviest set
  (mainStreet) is ≈500 meshes of trivial geometry.
- Materials reduce to MeshStandardMaterial with a small parameter envelope, a handful
  of Basic/Sprite materials, one ShaderMaterial (sky), one PointsMaterial.
- The real native-engine costs in this area are: (a) a **Canvas2D-equivalent procedural
  texture painter incl. text** (~26 painter functions, all textures are code-painted);
  (b) **draw-call count** from un-instanced repetition — trees first, then wires,
  fences, letters, books, sandbags; (c) **facade texture memory** on street sets;
  (d) shadow flags defaulting to cast+receive on nearly everything.
