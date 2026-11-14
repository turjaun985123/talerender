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
