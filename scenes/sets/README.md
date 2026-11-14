# Reusable set files — `scenes/sets/`

A **set file** is a scene-format v2 file holding one reusable set —
geometry, textures and the set's default light rig — and nothing
else: no camera, no `time`, no characters, no captions, no `post`.
Shot files reference a set instead of copying its nodes (the current
practice — `scenes/film/s04.scene.json` inlines the whole jungle set
by copy — means a set fix has to be re-pasted into every shot).

## Files

| file | set | source staging |
|---|---|---|
| `shore.scene.json` | SH — the shore, December 1944 | Onoda project `src/shots.ts`, SET 2 IIFE `SH` (lines 196–222) + `src/core/world.ts` helpers |
| `bookshop.scene.json` | BK — the bookshop, Japan, 1974 | Onoda project `src/shots.ts`, SET 3 IIFE `BK` (lines 226–235) over `src/core/world.ts` `interior()` (lines 292–341); shelves `shelfUnit()` `shots.ts:105–121`; desk/parcel `src/core/props.ts:158–166` |
| `jungle.scene.json` | JG — the jungle and the hidden camp, Lubang | Onoda project `src/shots.ts`, SET 1 IIFE `JG` (lines 140–189) + `palm()` (lines 85–102) + `src/core/world.ts` helpers; node set derived from `scenes/ab1.scene.json`, corrected against the source (see the jungle part below) |

Project root for every citation below:
`~/faceless-shorts/the-soldier-who-didnt-know/`. Paths are given as
`shots.ts:NNN` (= `src/shots.ts`), `world.ts:NNN`
(= `src/core/world.ts`), `engine.ts:NNN`, `env.ts:NNN`.

## Which shots use the shore set

Per `docs/integration/INVENTORY.md` and the shot functions in
`shots.ts`:

- **S02** — the landing / march along the shore. Draw calls
  `applyAtmosphere(SH.scene, 'noon', { rig: SH.sun, sky: false })`
  (`shots.ts:332`), grade `G.shore` (`shots.ts:135`).
- **S03** — Taniguchi delivers the order on the shore. Same noon
  re-dress (`shots.ts:345`), same grade.

## Proposal: how a shot file references a set (integrator decides)

1. A shot file names its set with one top-level field:
   `"set": "sets/shore.scene.json"` (path relative to `scenes/`).
   Unknown top-level fields are ignored by today's loader
   (scene-format §1), so existing tooling is unaffected.
2. **Merge happens before load.** `src/scenefile.rs:768` requires a
   `camera` in every loaded file, so a set file is a *fragment*: it
   is never loaded standalone. The integrator's merge step produces
   the complete shot file the loader consumes:
   - **nodes** — the set's nodes are spliced in *before* the shot's
     own nodes (set first, shot content after; only transparent
     objects are order-sensitive, and the set's sky/clouds keep the
     roles they had in the source scene graph).
   - **textures** — appended to the shot's table. On an `id`
     collision with *different* parameters, the set's entry is
     renamed `<id>@<setname>` and its references rewritten (the
     shore ids `sky` / `cloud` intentionally match the corpus
     generators; `grass-hills`, `planks-crate`, `planks-barrel` are
     set-specific to minimise collisions).
   - **lights** — the shot's `lights` block wins if present; if the
     shot has none, it inherits the set's rig (that is what the
     set's rig is *for*: the default under which the set was
     authored). Per-shot atmosphere re-dressing, when the integrator
     models it, applies after the merge, exactly as
     `applyAtmosphere` applies after build in the source.
   - **camera / time / post / captions / characters** — from the
     shot only; a set file never carries them.
3. Alternative the integrator may prefer: teach the loader a
   fragment mode (camera optional, `"set"` resolved internally).
   The merge-before-load route above needs no loader change.

## The noon light rig (two layers — read before editing)

The `lights` block in `shore.scene.json` is the rig **in force**
when S02/S03 draw, which is *not* the rig as built: every SH draw
re-dresses it with the `noon` atmosphere. Both layers, cited:

| field | as built — `sunlight()` call, `shots.ts:200-201` (+ `world.ts:172-176`, `engine.ts:63-71`) | in force — after `applyAtmosphere('noon')`, `env.ts:164`, preset `env.ts:76-84` |
|---|---|---|
| key colour | `#FFEFD2` | **`#FFF0D6`** (preset `keyCol`) |
| key intensity | 2.4 | **2.6** (preset `keyI`) |
| key position | `[-8, 11, 7]` | unchanged (noon sets no `keyPos`) |
| key target | `[0, 0, -4]` (`shots.ts:201`) | unchanged |
| key shadow | box 17, far 80, bias −0.0006 (`world.ts:175`), normalBias 0.02 (`engine.ts:66`) | unchanged |
| rim | `#FFFFFF`, 0.4, `[-3, 3, -3]` (`world.ts:173` via `engine.ts:69`) | intensity 0.4 (preset `rimI` — same value) |
| hemisphere | sky `#DCECD2`, ground `#8A7A52`, 1.15 | **sky `#CFE6FF`, ground `#C48A5A`, 1.25** (preset) |
| points | none in SH | none |
| envIntensity | 0.25 (`world.ts:173`, `env` default) | untouched by the atmosphere |
| fog | none | `null` (noon has no fog) |

Bold values are the ones written to the file. (The jungle shot file
`s04.scene.json` instead wrote the *as-built* hemisphere with the
*in-force* key — this file documents the full transformation so the
integrator can pick one convention deliberately.)

## Shore inventory — element by element

47 top-level nodes, 229 leaf meshes, 5 textures. All numbers follow
the f32 data law (scene-format §11): each emitted float is the exact
f32 value, printed in full.

| # | element (file name) | source | notes |
|---|---|---|---|
| 1 | sky dome — sky-sphere r 380, 32×16, texture `sky` | `shots.ts:199`; `world.ts:149` | gradient `top #6E9CC0`, `horizon #E4EEDC`; the shader's ground colour (default `#BFE4F7`) has no field in the `sky-gradient` generator — gap G4 |
| 2–3 | cloud sprites ×2 — billboards 24×12 @ (−24, 22, −85), 30×15 @ (30, 27, −90), texture `cloud` | `shots.ts:202`; `world.ts:167-171` | sprite scale is (s·2, s, 1); sprite tint `#EFF4EA` not carried — gap G5 |
| 4 | sand disc — disc r 30, 40 seg, `#DEC795`, roughness 1 | `shots.ts:203-204` | flat colour, **no map in the source** |
| 5 | sea — 190×70 plane @ (0, −0.06, −58), `#4E86AC`, roughness 0.35, metalness 0.08 | `shots.ts:205-206` | represented as a zero-height box — gap G1 |
| 6 | foam line — 190×1.6 plane @ (0, −0.02, −29.2), `#EDF2EA`, roughness 0.9 | `shots.ts:207-208` | same representation as the sea — G1 |
| 7–10 | hills ×4 — sphere r 1 (32×16) scaled (r, h, r·0.6) @ (x, −h/4, z): (−40, 55, 34, 12), (10, 45, 46, 16), (60, 59, 36, 11), (−90, 65, 40, 10) | `shots.ts:209`; `world.ts:180-187` (default list with z = 55) | colour = shadeHash {#5E8F52, 0.92 + rnd(i)·0.16} **× grass map, repeat [6, 3]** — G2 |
| 11–34 | hill trees ×24 (6 per hill) — `tree()` defaults: round, col `#4E9A3C`, scale (0.5 + rnd(k)·0.4)·2.2, positions from rnd(i·9+k) | `world.ts:185`; `tree()` `world.ts:189-198` | rnd placements baked as f32 numbers — G7; source clears their castShadow — G6 |
| 35–41 | shore tree row ×7 — (x, z, s): (−7, 7, 1.5) round, (7.5, 8, 1.4) cypress, (−11, 10, 1.7) round, (12, 11, 1.6) cypress, (0, 13, 1.8) round, (−4, 11, 1.3) cypress, (4.5, 12, 1.35) round; col `#3E7A34` for all, **no rotation.y** | `shots.ts:210`; `world.ts:189-198` | round = trunk + 5 blobs (shadeHash {#3E7A34, 0.9 + rnd(j)·0.2}); cypress = trunk + one sphere scaled (0.7s, 2.6s, 0.7s) @ y 3.2s, plain colour |
| 42–43 | palms ×2 — bp1 s 1.0, lean +0.18 @ (−6.2, 0, 4.5); bp2 s 0.92, lean −0.14 @ (6.8, 0, 5.5); 18 children each (trunk seg 8, 7 fronds, 7 tips, 3 coconuts) | `shots.ts:211-212`; `palm()` `shots.ts:85-102` | group transform = translate · rotate_z(lean) · scale(s), children at unit scale (the `ab1.scene.json` palm convention); frond/tip colour parity per source — G8 |
| 44 | landing boat — group @ (−4.6, 0, −7.5), rotate_y 1.15; hull box 1.5×0.5×4.4 @ y 0.32, bow box 1.1×0.42×0.5 @ (0, 0.36, 2.35) rotate_x −0.5, two seat boxes 1.3×0.08×0.5 @ y 0.5, z −0.8 / +0.9; all `#6E5136`, roughness 0.9 | `shots.ts:214-217`; `box()` `world.ts:53` | — |
| 45–46 | crates ×2 — 0.6 @ (2.7, 0, −1.9); 0.45 @ (3.3, 0, −2.5) rotate_y 0.5; box child @ y s/2 | `shots.ts:219-220`; `world.ts:216` | material is **map-only**: planks `#B07A48`, repeat [0.5, 0.5], no colour — G2 |
| 47 | barrel — group @ (2.4, 0, −2.95): body cyl r 0.33 h 0.8 seg 16 @ y 0.4; bands cyl r 0.345 h 0.05 seg 16 @ y 0.12 / 0.68, `#3A3A3F`, metalness 0.4 | `shots.ts:221`; `world.ts:215` | body = colour `#9A6337` **× planks map** (repeat [1, 1]) — G2 |

Tree trunks everywhere: cylinder (0.12s, 0.18s, 1.6s) @ y 0.8s,
colour `#7A5234`, segments 12 (the `cyl()` default, `world.ts:54`).
Canopy blobs: icosahedron detail 2 in the source, carried as sphere
12×9 (the corpus approximation, as in `ab1.scene.json`).

## Format decisions and declared gaps

- **G1 — sea & foam planes.** The v2 vocabulary has no colour
  *plane* kind (colour geometry: box/cylinder/sphere/cone/disc;
  `quad`/`flat-quad` are textured-only). Both planes are written as
  **zero-height boxes** whose top face is coplanar with the source
  plane at its exact position — safe in this codebase because
  `box_geo` uses fixed per-face normals (`src/scene.rs:150`).
  Proposal: add a `plane` colour kind and swap these two nodes.
- **G2 — colour × map materials (the M5 lesson).** Where the source
  material maps a texture, the file carries it as
  `material.map = <texture id>` (three.js semantics: map × colour;
  the crate's source material has no colour, so none is written).
  Flat-colour substitutions for mapped materials ran up to ~3× too
  bright in the M5 ablation, so no map was dropped. Two
  consequences the integrator owns: (a) the still loader's material
  parsing has no `map` field yet — it is carried as data for the
  film/PBR path and the integrator's loader work; (b) the `planks`
  texture **generator is not in the format registry** (registry:
  grass, sky-gradient, cloud, blob-shadow, onoda-card,
  dof-chart-pattern) — `planks-crate` / `planks-barrel` declare
  `generator: "planks"` with the source's base colour and repeat so
  the painter can be registered under the name the sets already use
  (the bookshop set needs the same painter). Texture `repeat`
  mirrors `canvasTex`'s repeat (`world.ts:32-39`); the hills' grass
  repeat [6, 3] is the `T.grass(col, [6, 3])` call at
  `world.ts:183`.
- **G3 — metalness.** Carried as an extra `material.metalness`
  field where the source sets it (sea 0.08, barrel bands 0.4). The
  loader type-checks `color`/`roughness` only today (§3.3); unknown
  fields are ignored, so the value survives as data.
  `roughness` is written wherever the source `mat()` call sets it
  explicitly and omitted where it takes the `mat()` default 0.85
  (`world.ts:44-49`).
- **G4 — sky ground colour.** `sky()`'s shader takes a third
  colour, `ground` (default `#BFE4F7`, not overridden by SH), shown
  below the horizon. The `sky-gradient` generator carries top +
  horizon only (same limit as `ab1.scene.json`). From every SH
  camera the band is behind sea/sand.
- **G5 — cloud tint.** Sprites are tinted `#EFF4EA`
  (`shots.ts:202`); billboard nodes carry no tint field (same gap
  as `ab1.scene.json`). The tint is within 6% of white per channel.
- **G6 — shadow flags.** Colour-mesh nodes carry no
  castShadow/receiveShadow fields. Notable source facts not
  represented: `hills()` clears castShadow on all 24 hill trees
  (`world.ts:185`); sand/sea/foam differ in receiveShadow
  (sand receives; sea/foam are plain `scene.add` meshes whose
  receiveShadow stays false — only `box()`/`cyl()` set both flags,
  `world.ts:53-54`).
- **G7 — rnd-derived placements.** Hill-tree positions/scales come
  from `rnd()` (`world.ts:11`; spec §6). They are baked as exact
  f32 numbers, computed with the same formula in f64. Cross-check
  against the corpus generator's JG values (identical staging
  code): hill tree 0 trunk radii match exactly; its x position
  agrees to 1 f32 ulp (≤ 2 µm — a libm `sin` last-bit difference,
  sub-pixel at any focal length, but not bit-identical). Canopy and
  hill *colours* are not baked — they use the `shadeHash` form so
  the consumer evaluates the hash itself.
- **G8 — divergences from `ab1.scene.json` (deliberate).** That
  file mirrors the hand-authored `src/scene.rs`; where it and the
  film's TypeScript source disagree, this file follows the source:
  hill spheres 32 wide segments (source `world.ts:182`) vs 24;
  tree trunks 12 segments (`cyl()` default, `world.ts:54`) vs 7;
  palm tips 8×5 (`shots.ts:95`) vs 10×6; palm frond colour parity —
  source: frond i odd → leaf `#3F7D33`, even → leaf2 `#4C8A3A`
  (`shots.ts:93`); ab1 has the fronds inverted (its tips match the
  source). Flagged here so the integrator can rule on the corpus
  files; the set file does not silently pick the corpus side.

## Provenance

Transcribed first-hand from the project sources listed above on
2026-11-14 (TaleRender production-integration wave, seat W10).
No values were taken from `ab1.scene.json` except the cross-checks
named in G7/G8.

---

# The bookshop set — `bookshop.scene.json`

BK is `WD.interior(E, { wall: '#5A4632', motif: '#6B5638',
ceiling: '#3A2E22', floor: '#7A5230', signText: 'TANIGUCHI BOOKS',
picture: true, litter: false })` (`shots.ts:227`) with three shelf
units, a desk and two parcels added by the BK IIFE itself
(`shots.ts:228–233`). `interior()` runs with its defaults
w = 8, depth = 7, h = 4.2 (`world.ts:292`), so the room spans
x ∈ [−4, 4], z ∈ [−7, +2] (the floor/walls are depth+2 long,
centred z = −depth/2 + 1 = −2.5), the back wall sits at z = −7,
and its window opening is ww 3.2 × wh 1.9 at sill height wy 1.1
(`world.ts:300`).

## Which shots use the bookshop set

- **S10** — Taniguchi among his shelves (`shots.ts:488–496`),
  grade `G.shop` (`shots.ts:137`), the only BK shot in the film
  (INVENTORY shot table). The shot's content — Taniguchi, the
  LEDGER prop, the push-in — is not set content and is not here.

**Cross-check status:** `scenes/film/s10.scene.json` did not
exist when this file was authored (checked 2026-11-14), so no
shot file could be cross-checked; the values below are
transcribed from source and are the reference an s10 file should
match. The rig was cross-checked against wave seat W01's
independent transcription (`scenes/atmospheres.json`
`baseRigs.bookshopInterior` and `filmLooks.bk-noon-shop`): every
field agrees — key, hemisphere, fill point, both bulb points,
envIntensity 0.35, background `#1A1418`.

## The noon question (as built === in force — the exception)

`shots.ts:236` calls `applyAtmosphere(BK.scene, 'noon')` with
**no options object** — no rig, sky defaulting to true. Walking
`applyAtmosphere` (`env.ts:164–216`) for this scene:

| step | effect on BK |
|---|---|
| fog | `scene.fog = null` — already null; noon has no fog |
| sky/clouds | the traverse finds no sky-dome ShaderMaterial (the interior has no dome; its background is a plain `scene.background` colour, `world.ts:293`) and no sprites — nothing is re-dressed |
| light rig | `opts.rig` is undefined — the interior's lights are **not touched** |
| env-match | no Toon is in the scene yet (`inTani` is created at `shots.ts:251`, after the call), and the noon tint is the identity anyway |

So BK is the one set in the film where the as-built rig **is**
the in-force rig. The file's `lights` block therefore carries
the interior's own values — key `#FFE4B8` × 2.4, *not* the noon
preset's `#FFF0D6` × 2.6 that JG/SH end up with. (Recording the
preset values here, by symmetry with the shore file, would be
wrong; this is the trap the two-layer table above exists to
prevent.) There is no rim light and no `post` in the set file.

## Bookshop inventory — element by element

21 top-level nodes, 369 leaf meshes, 14 textures. All numbers
follow the f32 data law (scene-format §11).

| # | element (file name) | source | notes |
|---|---|---|---|
| 1 | floor — 8×9 plane @ (0, 0, −2.5) | `world.ts:296–297` | planks `#7A5230`, repeat [3.2, 3.6], texture rotation π/2 — B1, B5; receiveShadow only in source |
| 2 | back wall (group @ z = −7) | `world.ts:299` | children 3–12 are in group space |
| 3–4 | back wall segments ×2 — 2.4×4.2×0.2 @ x ∓2.8 | `world.ts:302` | wallpaper `wp(2.4)` → repeat [1.7142857313156128, 3] — the variant `wallpaper-walls` declares |
| 5 | under-window board — 3.2×1.1×0.2 @ y 0.55 | `world.ts:303` | **wainscot planks**, `wain(ww)` = repeat [1.6, 0.5] |
| 6 | above-window band — 3.2×1.2×0.2 @ y 3.6 | `world.ts:303` | wallpaper `wp(3.2)`, repeat [2.2857143878936768, 3] — see the note below the table |
| 7–11 | window frame ×5 — sill/head 3.4×0.14×0.3 @ y 1.1 / 3.0, z 0.05; jambs 0.12×1.9×0.3 @ x ±1.6; mullion 0.06×1.9×0.1; all `#2F5C52` | `world.ts:304` | flat colour; `mat()` default roughness 0.85 |
| 12 | window view plane — 8.32×4.94 @ (0, 2.05, −2.5) in group space (world z −9.5) | `world.ts:306` | MeshBasicMaterial `#BFE4F7`, toneMapped false — the flat-colour view the INVENTORY notes; no texture (`view` option is null) — B1, B3 |
| 13–14 | side walls ×2 — 0.2×4.2×9 @ x ±4 | `world.ts:308` | wallpaper; the source sets the map repeat on the shared clone to [(depth+2)/1.4, h/1.4] = [6.428…, 3] — B5 note |
| 15–16 | side wainscot ×2 — 0.24×1.0×9 @ x ±3.98 | `world.ts:310` | planks `#6B3E26`, `wain(depth+2)` = [4.5, 0.5] |
| 17–18 | back wainscot strips ×2 — 2.4×1.0×0.24 @ (±2.8, 0.5, −6.98) | `world.ts:311` | `wain(2)` = [len/2, 0.5] → repeat [1.0, 0.5]; the len is hardcoded 2, not the segments' 2.4 |
| 19 | ceiling — 8×0.2×9 @ y 4.3 | `world.ts:313` | wallpaper base `#3A2E22`, motif = `shade(ceiling, 1.25)` = `#413427` (computed with three.js colour management), repeat [w/1.2, (depth+2)/1.2] |
| 20 | ceiling beam — 8×0.16×0.3 @ (0, 4.12, −6.9), `#E8DCC0` | `world.ts:314` | flat colour |
| 21 | counter (group @ (2.7, 0, −2.9)) — body 1.0×1.1×5.5, top 1.2×0.08×5.7, brass rail 0.1×0.06×5.7 @ x −0.62 | `world.ts:316–319` | body planks `#7A3E24` [1, 2]; top felt `#1F5A45`; rail `#C9A13B` metalness 0.6 / roughness 0.3 (G3 rule) |
| 22 | picture frame — 0.9×0.7×0.05 @ (−3.87, 2.4, −3.5), rotate_y π/2 | `world.ts:322–323` | six-material box: frame `#C9A13B`, +z face the `picture` texture — B2, B6 |
| 23 | sign board — 2.2×0.4×0.05 @ (3.87, 2.9, −2.9), rotate_y −π/2 | `world.ts:324`; text from `shots.ts:227` | `signBoard('TANIGUCHI BOOKS', { w: 2.2, h: 0.4, bg: '#3A2A2A', fg: '#F2E6CF' })` (`world.ts:141–148`); edge colour `shade(bg, 0.7)` = `#302222`; the text texture data is on the `sign` texture entry — B2, B9 |
| 24–26 | shelf units ×3 — (3.4, seed 1) @ (−1.8, 0, −6.55); (2.4, seed 2) @ (1.9, 0, −6.55); (3.2, seed 3) @ (−3.82, 0, −3.6) rotate_y π/2 | `shots.ts:228–230`; builder `shots.ts:105–121` | frame `#5A4630`; books below |
| 27 | desk (1.6) — group @ (−1.5, 0, −3.4), rotate_y 0.15: felt top 1.6×0.08×1.0 @ y 0.84, slab 1.7×0.06×1.1 @ y 0.78, pedestals 0.5×0.76×0.95 @ x ∓0.5, back panel 0.6×0.6×0.06 @ (0, 0.45, −0.45) | `shots.ts:231`; `props.ts:162–166` | felt `#2E5A44` (not the counter's green); wood planks `#6B3E26` [1, 1]; the back panel's width is `w − 1` = 0.6 at this w |
| 28–29 | parcels ×2 — 0.5×0.16×0.36 @ (−1.4, 0.8, −3.35) rotate_y 0.4; 0.44×0.13×0.32 @ (−1.35, 0.95, −3.3) rotate_y 0.15 | `shots.ts:232–233`; `props.ts:158–161` | both `label: false` → the `parcel-plain` texture (no label block); they rest on the desk (top surface y = 0.88) with slight sink/tilt-free overlap exactly as staged |
| 30–31 | bulb fixtures ×2 @ (−1.5, −2.2), (1.6, −4.5) — cord cyl r 0.008 h 1.0 @ y 3.7, bulb sphere r 0.1 @ y 3.15 | `world.ts:327–331` | cord is MeshBasicMaterial `#C9A13B` (B3); bulb emissive `#FFE7A0` × 2.2 (B4); the matching point lights are in `lights.points` at y = h − 1.15 = 3.05 |

**Wallpaper repeat note (rows 3–6).** `wp(len)` builds a fresh
material per call with repeat `[len/1.4, h/1.4]`: the two
full-height back-wall segments (len 2.4) carry
[1.7142857313156128, 3], the above-window band (len = ww = 3.2)
carries [2.2857143878936768, 3] (the under-window board between
them is wainscot, not wallpaper). A texture entry holds one
repeat, so `wallpaper-walls` declares the segments' variant
(the larger area) and this note records the band's; splitting
a second entry for the band is the integrator's call if the
difference ever becomes visible (a motif scale of 3:4 on one
1.2-high band).

## The book rows (deterministic — parameters and results)

`shelfUnit(w, h, seed)` (`shots.ts:105–121`): frame colour
`#5A4630`; four rows at board y = 0.12 + r·0.58 (r = 0..3); each
row walks x from −w/2 + 0.12 while x < w/2 − 0.16, consuming a
slot of width bw = 0.055 + rnd(seed·31 + r·17 + i)·0.05 and
stepping bw + 0.012. A slot holds a book iff
rnd(seed·91 + r·13 + i) > 0.14; the book is
bw × (0.34 + rnd(seed·47 + r·7 + i·3)·0.13) × 0.2 at
(x + bw/2, boardY + 0.03 + bh/2, 0.02), colour
`cols[(i + r·3 + seed) % 8]` from
`['#7A4A3A', '#4A5A6A', '#6A6248', '#54455E', '#77603A',
'#3F5A50', '#83584A', '#5E5142']`, leaning (rotation.z = 0.1)
iff rnd(i·7 + r + seed) > 0.93. `rnd` is the §6 sin-hash.

The file's `bookRows` block records the full evaluated result —
per unit, per row, per slot: x, width, height, presence, colour,
lean — and the shelf groups' children carry the same books as
meshes. Resulting counts:

| unit | slots per row | books | gaps | leans |
|---|---|---|---|---|
| seed 1 (w 3.4) | 35 / 34 / 33 / 34 | 115 | 21 | 8 |
| seed 2 (w 2.4) | 22 / 22 / 24 / 23 | 79 | 12 | 5 |
| seed 3 (w 3.2) | 33 / 33 / 33 / 34 | 116 | 17 | 7 |
| **total** | **360** | **310** | **50** | **20** |

Robustness: the tightest threshold test in all 360 slots is
seed 3, row 2, slot 27's lean test — rnd = 0.9297837879566941
against the 0.93 cut, a margin of 2.2×10⁻⁴, still ~7 orders of
magnitude above any f64 libm `sin` divergence (the shore G7
caveat); every other test clears its cut by ≥ 2.9×10⁻³. The rows
are stable, not knife-edge.

## Format decisions and declared gaps (bookshop)

- **B1 — floor & window-view planes.** Same G1 situation as the
  shore: no colour *plane* kind exists, so both `PlaneGeometry`
  meshes are zero-height boxes (top face coplanar at the exact
  source height). The floor's rotation.x = −π/2 is the box's
  identity orientation; the view plane stands vertical at the
  back of the window group, facing +z as the source plane does.
- **B2 — multi-material boxes.** The picture frame and the sign
  are the corpus's first material-array meshes. They carry a
  `materials` array of six in three.js BoxGeometry group order
  [+x, −x, +y, −y, +z, −z]; the loader's §3.3 type-check covers
  only the single `material` form today, so the arrays are
  carried as data (unknown-field law, §1) for the film/PBR path.
- **B3 — basic materials.** Three BK materials are
  MeshBasicMaterial in the source and ignore lighting: the
  window view plane (`toneMapped: false` — its `#BFE4F7` is
  displayed raw), the bulb cords, and the sign/picture face
  materials are MeshStandardMaterial (not basic). Carried as
  `"basic": true` on the affected nodes plus `toneMapped` on
  the view plane's material; a consumer that shades the view
  plane with the rig will darken the window noticeably.
- **B4 — emissive.** The bulb spheres carry
  `material.emissive` / `emissiveIntensity` (source:
  `#FFE7A0` × 2.2). No corpus scene file has used the fields
  before; the PBR path's standard material has an emissive term
  (`src/ab1.rs:89`), and the fields are carried under the same
  unknown-field law as G3's metalness.
- **B5 — texture rotation & shared clones.** The floor's planks
  texture is rotated π/2 about its centre (`world.ts:297`) —
  carried as `rotation` on the texture entry; the format's
  texture entries have no rotation field in the loader today.
  The side walls' wallpaper repeat mutation (`world.ts:309`)
  sets the same values `wp(depth+2)` already produced, so
  `wallpaper-sides` needs no special case; the back-wall
  segments' differing repeat is the table note above.
- **B6 — shadow flags.** Not carried (the G6 rule). Source
  facts for the record: floor receives only (raw `Mesh`, cast
  defaults false); the view plane neither; everything built by
  `box()`/`cyl()` casts and receives — including the walls and
  ceiling.
- **B7 — key shadow extras.** The interior key's shadow
  `radius: 4` and `mapSize: 2048` are source literals
  (`world.ts:326`) carried on the `lights.key` block; the
  §14.2 loader type-checks a subset. `normalBias: 0` is the
  three.js DirectionalLightShadow default — `interior()` never
  sets it (it does not use `lightRig`) — flagged here and in
  the file, not presented as a source value.
- **B8 — wallpaper / planks / felt / picture / sign / parcel
  registry.** None of these painters is in the format registry
  (registry: grass, sky-gradient, cloud, blob-shadow,
  onoda-card, dof-chart-pattern). The texture entries declare
  `generator` names `wallpaper`, `planks`, `felt`, `picture`,
  `sign`, `parcel` with the source's base colours, canvas sizes
  and repeats so the painters can be registered under the names
  the sets already use (the shore set declared `planks` the
  same way, G2). Painter-internal behaviour is pinned by the
  source, not by this file: the wallpaper painter draws only
  two of its four listed fleur positions (`i < 2`,
  `world.ts:67`) plus four alpha-0.5 dots; the planks painter's
  grain is rnd-seeded per plank row; the parcel painter's 30
  fibre streaks are rnd-seeded.
- **B9 — sign font size.** `signBoard` shrinks its font from
  40 px by 2 until the text fits 86% of the 440 px canvas
  (`world.ts:143–147`). Whether any shrink step fires depends
  on the browser's Caladea (fallback Georgia) metrics, which
  cannot be reproduced from source — the rule and the starting
  size are recorded on the `sign` texture entry; the final
  size is the one value in this set not pinned exactly.
- **B10 — litter excluded, deliberately.** `interior()` can
  scatter 140 instanced paper slips (`world.ts:334–338`), but
  BK passes `litter: false` (`shots.ts:227`), so they never
  enter this set and are not in the file.

## Provenance (bookshop)

Transcribed first-hand from the project sources listed in the
files table on 2026-11-14 (TaleRender production-integration
wave, seat W15). The shelf rows were evaluated by running the
builder's own arithmetic (same `rnd`, same loop) rather than
by sampling a render; generation is byte-deterministic.
`s10.scene.json` did not exist at authoring time — see the
cross-check note above.

---

# The jungle camp set — `jungle.scene.json`

The JG set as a standalone fragment: 78 top-level nodes, 325 leaf
meshes plus one scatter node (900 grass-tuft instances), 7
textures, and the noon light rig. The node set is derived from
`scenes/ab1.scene.json` — the M5-proven S01 staging — with every
element re-checked against the project source (`shots.ts` JG
IIFE, lines 140–189; `palm()` lines 85–102; `world.ts` helpers).
Where ab1 and the source disagree, the source wins; every such
correction is listed below with its citation. What the set file
deliberately does *not* carry from ab1: the character card, the
rifle code-ref, the blob shadow and the caption (shot content),
and ab1's camera / v1 `lighting` block (replaced by the v2
`lights` block).

## Which shots use the jungle camp set

Per `docs/integration/INVENTORY.md`: **S01, S04, S05a, S05b,
S06, S07, S08, S08b, S09, S11, S12** — every shot except the two
shore shots and the bookshop shot. The set file carries the
**noon** rig, the default under which the set is authored. The
atmosphere variants are per-shot re-dresses, exactly as in the
source, where every JG draw calls
`applyAtmosphere(JG.scene, …, { rig: JG.sun, sky: false })`:
S06 draws fog, S07 golden, S11/S12 dusk. Note `sky: false` — JG
shots never re-dress the sky dome or clouds from a preset (the
dome keeps the set's `#6E9CC0` / `#D8E8DA`); only the rig, the
scene fog and the grade change. On a merge (proposal above) each
such shot's own `lights` block wins over the set's.

## The noon light rig (two layers — JG)

Same two-layer situation as the shore, plus the fire point. The
file carries the **in-force** values (ruling R1, scene-format
§14.5):

| field | as built — `sunlight()` call, `shots.ts:143` (+ `world.ts:172-176`, `engine.ts` `lightRig`) | in force — after `applyAtmosphere('noon')`, preset `env.ts:76-84` |
|---|---|---|
| key colour | `#FFEFD2` | **`#FFF0D6`** |
| key intensity | 2.4 | **2.6** |
| key position | `[7, 12, 5]` | unchanged (noon sets no `keyPos`) |
| key target | `[0, 0, -2]` (`shots.ts:144`) | unchanged |
| key shadow | box 17, far 80, bias −0.0006 (`world.ts:175`), normalBias 0.02 (`engine.ts` `lightRig`) | unchanged |
| rim | `#FFFFFF`, 0.4, `[-3, 3, -3]` (`engine.ts` `lightRig`) | intensity 0.4 (preset `rimI` — same value) |
| hemisphere | sky `#DCECD2`, ground `#46603A`, 1.15 | **sky `#CFE6FF`, ground `#C48A5A`, 1.25** |
| fire point | `#FF9A3C`, intensity **5**, distance 7, decay 1.6, at `[-1.35, 0.6, -2.45]` (`shots.ts:179`) | intensity basis **4.6** — `fireLevel` overwrites it every frame with `(4.6 + 0.7·sin 17t + 0.5·sin 7.3t)·level` (R1) |
| envIntensity | 0.25 (`world.ts` `sunlight()` default) | untouched by the atmosphere |
| fog | none | `null` (noon has no fog) |

## Jungle inventory — element by element

Nodes are in source build order. All numbers follow the f32
data law (scene-format §11).

| # | element (file name) | source | notes |
|---|---|---|---|
| 1 | sky dome — sky-sphere r 380, 32×16, texture `sky` | `shots.ts:142`; `world.ts:149` | gradient `top #6E9CC0`, `horizon #D8E8DA`; the shader's ground colour (default `#BFE4F7`) has no field in the `sky-gradient` generator — gap J2 |
| 2–4 | cloud sprites ×3 — billboards 26×13 @ (−28, 23, −85), 32×16 @ (26, 29, −95), 20×10 @ (58, 19, −75), texture `cloud` | `shots.ts:145`; `world.ts:167-171` | sprite scale is (s·2, s, 1); sprite tint `#EFF4EA` not carried — gap J3 |
| 5 | grass ground — ground-disc r 80, texture `grass` (repeat via `uvRepeat` 48) | `shots.ts:146` | `T.grass('#5B9A48', [48, 48])`; disc segments corrected 48 → 40 — C5 |
| 6 | dirt patch — disc r 4.8, 28 seg @ (0, 0.012, −0.9), `#86714C`, roughness 1 | `shots.ts:147-148` | flat colour, no map in the source |
| 7–10 | hills ×4 — sphere r 1 (32×16) scaled (r, h, r·0.6) @ (x, −h/4, z): (−40, −75, 34, 12), (10, −85, 46, 16), (60, −71, 36, 11), (−90, −65, 40, 10) | `shots.ts:149`; `world.ts:180-187` (z = −75 list) | colour = shadeHash {#5E8F52, 0.92 + rnd(i)·0.16} **× grass map, repeat [6, 3]** — C3, C6 |
| 11–34 | hill trees ×24 (6 per hill) — `tree()` defaults: round, col `#4E9A3C`, scale (0.5 + rnd(k)·0.4)·2.2, positions from rnd(i·9+k) | `world.ts:185`; `tree()` `world.ts:189-198` | rnd placements baked as f32 numbers — J6; source clears their castShadow — J4 |
| 35–53 | tree ring ×19 — (x, z, s, kind) per `shots.ts:151-155`; colour `i % 3 ? #3E7A34 : #356B2C`; rotation.y = i·1.7; 14 round / 5 cypress | `shots.ts:151-155`; `world.ts:189-198` | round = trunk + 5 blobs (shadeHash {tree col, 0.9 + rnd(j)·0.2}); cypress = trunk + one sphere scaled (0.7s, 2.6s, 0.7s) @ y 3.2s, plain colour; trunks corrected to 12 segments — C4 |
| 54–56 | palms ×3 — p1 s 1.05, lean +0.14 @ (4.6, 0, −4.6); p2 s 0.9, lean −0.2, rotation.y 1.2 @ (−5.2, 0, −5.4); p3 s 1.15, lean +0.1 @ (10.5, 0, −11); 18 children each (trunk seg 8, 7 fronds, 7 tips, 3 coconuts) | `shots.ts:160-162`; `palm()` `shots.ts:85-102` | group transform = translate · rotate_y (p2 only) · rotate_z(lean) · scale(s), children at unit scale (the ab1 palm convention); frond colour parity corrected — C1; tips corrected to 8×5 — C2 |
| 57 | grass tufts — scatter, cone (0.05, 0.34, 4), `#4C8A3E`, 900 instances, annulus placement | `shots.ts:164-166` | node copied verbatim from `ab1.scene.json` — J5 |
| 58–65 | fire stones ×8 — sphere (0.09 + rnd(i)·0.04, 8×6) on a 0.44 ring @ (−1.35, 0.05, −2.45), `#8B8B84`, roughness 0.95 | `shots.ts:168-173` | radii reproduce ab1's baked values exactly — J6 |
| 66–68 | fire logs ×3 — cyl (0.05, 0.05, 0.72, seg 7), `#7A5A38` roughness 0.95, rotation.z π/2, rotation.y i·0.7, offsets (0.25, 0.3) / (−0.2, −0.28) / (0.05, 0.02) from the fire pit | `shots.ts:175-176` | local `cyl()` with segN 7 |
| 69 | flame — cone (0.17, 0.52, 8) @ (−1.35, 0.34, −2.45) | `shots.ts:176-177` | as built, scale 1 — C9; emissive not representable — J1 |
| 70 | flame inner — cone (0.09, 0.34, 8) @ (−1.32, 0.3, −2.41) | `shots.ts:178` | same notes as the flame |
| 71 | seat log — cyl (0.17, 0.17, 1.15, seg 9) @ (−0.5, 0.17, −1.8), rotation.z π/2, rotation.y 0.12, log material | `shots.ts:180` | top of the log is the S06 seat (sitOn 0.34) |
| 72–73 | lean-to poles ×2 — cyl (0.035, 0.035, 1.5, seg 6) @ (−3.9, 0.75, −2.7) and (−2.7, 0.75, −2.7), log material | `shots.ts:181` | — |
| 74 | lean-to roof — box 1.9×0.05×1.5 @ (−3.3, 1.52, −2.95), rotation.x 0.42 | `shots.ts:182` | material is **map-only**: planks `#8A6B42` [1, 1], roughness 0.95 — C7 |
| 75 | crate 0.55 — group @ (−2.35, 0, −3.15); box child @ y 0.275 | `shots.ts:183`; `world.ts:216` | material is **map-only**: planks `#B07A48`, repeat [0.5, 0.5] — C7 |
| 76 | crate 0.42 — group @ (−2.3, 0.55, −3.12), rotation.y 0.4; box child @ y 0.21 | `shots.ts:184`; `world.ts:216` | same material as crate 0.55 |
| 77 | barrel — group @ (−3.05, 0, −2.2): body cyl r 0.33 h 0.8 seg 16 @ y 0.4; bands cyl r 0.345 h 0.05 seg 16 @ y 0.12 / 0.68, `#3A3A3F`, metalness 0.4 | `shots.ts:185`; `world.ts:215` | body = colour `#9A6337` **× planks map** (repeat [1, 1]) — C7 |
| 78 | bedroll — box 1.5×0.09×0.55 @ (−3.3, 0.05, −2.1), rotation.y 0.2, `#77805E`, roughness 1 | `shots.ts:186` | — |

Tree trunks everywhere: cylinder (0.12s, 0.18s, 1.6s) @ y 0.8s,
colour `#7A5234`, segments 12 (the `cyl()` default,
`world.ts:54`). Canopy blobs: icosahedron detail 2 in the
source, carried as sphere 12×9 (the corpus approximation, as in
`ab1.scene.json` and the shore file).

## Corrections to `ab1.scene.json` (deliberate)

ab1 mirrors the hand-authored `src/scene.rs` at canvas
resolution and predates the format's map/emissive vocabulary;
where it and the film's TypeScript source disagree, this file
follows the source:

- **C1 — palm frond colour parity (the flagged inversion).**
  Source: frond i odd → leaf `#3F7D33`, even → leaf2 `#4C8A3A`
  (`shots.ts:93`); tips the opposite (`i % 2 ? leaf2 : leaf`,
  `shots.ts:95`). ab1 has the fronds inverted (its tips match
  the source). Corrected in all three palms. The film shot
  files authored from ab1 (`scenes/film/s01.scene.json` among
  them) carry the same inversion — flagged here for the
  integrator; this seat did not edit them.
- **C2 — palm tip segments.** Source tips are
  `SphereGeometry(1, 8, 5)` (`shots.ts:95`); ab1 wrote 10×6
  (the frond size). Fronds stay 10×6 (`shots.ts:93`).
- **C3 — hill sphere segments.** Source `SphereGeometry(1, 32,
  16)` (`world.ts:182`); ab1 wrote 24 wide segments.
- **C4 — tree trunk segments.** Source trunks use the
  `cyl()` default of 12 (`world.ts:54`); ab1 wrote 7. Applies
  to the ring trees and the hill trees.
- **C5 — ground disc segments.** Source
  `CircleGeometry(80, 40)` (`shots.ts:146`); ab1 wrote 48 —
  that 48 is the grass texture's repeat, which stays where
  ab1 put it (`uvRepeat` 48, from `T.grass('#5B9A48',
  [48, 48])`).
- **C6 — hill grass map restored.** The source hill material
  is colour × map (`world.ts:183`); v1 ab1 dropped the map.
  Carried as `material.map = "grass-hills"` (the M5 lesson —
  flat substitutions for mapped materials run bright), with a
  texture entry identical to the shore file's.
- **C7 — camp maps and group forms restored.** Crates and the
  barrel are groups in the source (`world.ts:215-216`); ab1
  flattened them to single meshes with flat colours (barrel
  body `#6E4525`, an averaged shade). Now: crate boxes are
  map-only `planks-crate`; the barrel body is `#9A6337` ×
  `planks-barrel` and its bands regain metalness 0.4; the
  lean-to roof was flat `#8A6B42` and is now map-only
  `planks-roof` (`shots.ts:181-182`). The `planks` generator
  is not yet in the format registry — same declaration as
  shore gap G2.
- **C8 — roughness restored.** Written wherever the source
  sets it explicitly (the shore G3 rule): dirt 1, stones
  0.95, log material 0.95 (fire logs, seat log, lean-to
  poles), roof 0.95, bedroll 1, palm bark 0.95, fronds / tips
  / coconuts 0.9, flames 1. Omitted where `mat()` takes its
  0.85 default (`world.ts:44-49`). ab1 carried none of these.
- **C9 — flames as built.** ab1 (and s01) bake the t = 1.5
  flicker scale (1.13146 / 0.86854). A set file has no time:
  the flames are written at scale 1, as built — the flicker
  is per-frame state (`fireTo`, `shots.ts:190-194`), not
  build state.

## Format decisions and declared gaps (jungle)

- **J1 — flame emissive.** The source flames are dark bodies
  with strong emissive: `#3A1C08` + emissive `#FF7A1A` ×2.4
  and `#3A2A08` + emissive `#FFC93C` ×2.6 (`shots.ts:176-178`).
  The format's material carries no emissive field, so the
  corpus stand-in colours from ab1 (`#FF8A30` / `#FFD23C`)
  are kept — writing the source body colours without the
  emissive term would render the flames near-black. When the
  format gains an emissive field, these two nodes are the
  first to revisit.
- **J2 — sky ground colour.** The JG `sky()` call passes no
  `ground`, so the shader's default `#BFE4F7` shows below the
  horizon (`world.ts:149`). The `sky-gradient` generator
  carries top + horizon only (same limit as shore G4). From
  every JG camera the band is behind the ground disc.
- **J3 — cloud tint.** Sprites are tinted `#EFF4EA`
  (`shots.ts:145`); billboard nodes carry no tint field
  (same gap as shore G5).
- **J4 — shadow flags.** Colour-mesh nodes carry no
  castShadow/receiveShadow fields (same gap as shore G6).
  Notable source facts not represented: `hills()` clears
  castShadow on all 24 hill trees (`world.ts:185`); the
  ground and dirt discs receive only; the camp's directly
  constructed meshes (stones, palm parts) set castShadow
  only, while `box()`/`cyl()` set both (`world.ts:53-54`).
- **J5 — scatter descriptor verbatim.** The tufts node —
  geometry and placement alike — is copied exactly from
  `ab1.scene.json`: it is the proven corpus form of the
  placement law in `shots.ts:164-166` (annulus: angle from
  rnd(i·1.31), radius 2.6 + rnd(i·2.7)·20, centre
  (0, 0.15, −3)). One literal difference from the source is
  preserved with it, flagged rather than harmonised: the
  source computes the angle with `6.28`, the descriptor's
  `angleScale` is 2π (6.283185307) as authored for the
  corpus renderer — changing it would move all 900 tufts off
  the M5-proven frame.
- **J6 — rnd-derived placements.** Hill-tree positions and
  scales and the fire-stone radii come from `rnd()`
  (`world.ts:11`; spec §6). They are baked as exact f32
  numbers, computed with the same formula in f64. The stone
  radii reproduce ab1's baked values exactly; hill tree 0's
  x position agrees with ab1 to ~2 µm (a libm `sin` last-bit
  difference — the same observation as shore G7, sub-pixel
  at any focal length, but not bit-identical). Canopy and
  hill colours are not baked — they use the `shadeHash`
  form so the consumer evaluates the hash itself.
- **J7 — the fire point's per-shot level.** The set's
  `lights` block carries the fire point at the R1 basis
  (level 1, basis 4.6). Per-shot levels (S05a fire out, S06
  dying, S07 level 0), the flicker formula and the flame
  scales are shot state: the film files carry them (s01
  carries the `fireLevel` channel on its point), and a
  merged shot's `lights` block replaces the set's wholesale
  under the merge proposal.

## Merge notes (jungle)

Consistent with the proposal at the top of this document:

- A JG shot file names its set with `"set":
  "sets/jungle.scene.json"`; the set's nodes splice in
  before the shot's own nodes, in the build order above.
- **Textures.** The jungle `sky` entry differs from the
  shore file's under the same id (horizon `#D8E8DA` vs
  `#E4EEDC`) — the two sets are never merged into one scene
  today, but under the proposal's collision rule a mixed
  merge renames the set entry to `sky@jungle`. The
  `grass-hills`, `planks-crate` and `planks-barrel` entries
  are identical to the shore file's, so they merge without
  renaming; `grass` and `planks-roof` are jungle-only ids.
- **Lights.** Shot block wins if present, else the set's
  noon rig is inherited — and every JG film shot authored
  so far carries its own block, so in practice the set rig
  is the default for new shots and the reference for what
  "noon" means on this set.
- **The camp group.** In the source the camp is a
  `THREE.Group` with no transform (`shots.ts:168`); the
  file therefore keeps the camp items as top-level nodes in
  build order — there is no group transform to preserve.
  Crates and the barrel keep their source group form
  because their children carry offsets (and the stacked
  crate's group carries its rotation.y 0.4).

## Provenance (jungle)

Derived 2026-11-14 (TaleRender production-integration
wave, seat W18) from `scenes/ab1.scene.json`'s node set,
re-checked element by element against `shots.ts` (JG IIFE
140–189, `palm()` 85–102), `world.ts`, `env.ts` (noon
preset) and ruling R1 (scene-format §14.5). No values were
taken from the film shot files; `s01.scene.json` was
cross-checked only for the `lights`-block form, where it
agrees with §14.2. The corrections above are this file's
reason to exist alongside ab1 rather than as a copy of it.
