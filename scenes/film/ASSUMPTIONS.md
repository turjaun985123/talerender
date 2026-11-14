# Film scene files — assumptions, sources, inexpressible items

Per-shot sections appended by the seat that authored each file.
Project sources are in `~/faceless-shorts/the-soldier-who-didnt-know/src/`
unless a `talerender` path is given. All staging values follow the
f32 data law (docs/scene-format.md §11).

## S04 — s04.scene.json (JG noon, film t 15.95–21.38)

### Timing / shot identity
- `shot.at = cut(3) = L(3) − 0.06 = 15.95`, `end = cut(4) = 21.384`,
  `duration = 5.434` — project `shots.ts:125` (cut rule) with line
  starts from `film/onoda.json` lines[3].start = 16.01,
  lines[4].start = 21.444 (generated from the project's
  `data/lines.json`). Matches INVENTORY.md S04 row (15.95–21.38).
- `time = 18.67` is the file's evaluation instant (mid-shot,
  lt = 2.72): every character `state` in the file is the state at
  that t, per the v2 loader law (src/scenefile.rs:755–761).
- Captions: chunk ids 21–25 (line 3) with s/e/text/fit copied from
  `film/onoda.json` captions[21..25]. Chunk 20 ("never surrender.",
  e = 16.01) is still on screen for the first 0.06 s of the shot;
  it belongs to line 2 and is not listed.

### Set (JG camp)
- Nodes/textures are `scenes/ab1.scene.json`'s JG set verbatim
  (sky, grass ground, dirt, 3 hills + trees, 19-tree ring, 3 palms,
  900-tuft scatter, camp: fire ring/logs/flames/seat log/lean-to/
  crates/barrel/bedroll, 3 cloud billboards) — the same set S04
  uses (`shots.ts:141–188`, JG builder; S04 calls `noonJG()` +
  `jgStage({onoda, compA, compB, rifle})`, `shots.ts:359–360`).
- Dropped from the ab1 copy: the S01-staged code-ref Onoda card,
  his blob shadow and the rifle code-ref (replaced by the v2
  character node / declared attachment below), and the S01
  'fighting' caption billboard (S04 captions enter as chunk ids).

### Atmosphere / lights / post
- `atmosphere: "noon"` — `shots.ts:292` noonJG() =
  applyAtmosphere(JG.scene, 'noon', { rig: JG.sun, sky: false });
  preset values in project `core/env.ts:76–84`.
- `lights` block is identical to the lead's
  `scenes/character-v2.scene.json` (spec §14.2 example): key
  #FFF0D6 ×2.6 at [7,12,5] → target [0,0,−2], shadowBox 17
  (JG sunlight call `shots.ts:145`: pos [7,12,5], box 17, hemi
  sky #DCECD2 / ground #46603A ×1.15 → the hemisphere block),
  rim ×0.4, envIntensity 0.25, fire point #FF9A3C ×5.0 at
  [−1.35, 0.6, −2.45] distance 7 decay 1.6 (`shots.ts:179`).
  Declared divergence between sources: the JG sunlight call
  itself passes col #FFEFD2 ×2.4 while the env.ts noon preset and
  the v2 spec example use #FFF0D6 ×2.6 — the file follows the
  spec/S01 precedent; the film evaluator (src/film.rs:1176
  rig_noon) likewise folds #FFF0D6 ×2.6.
- `post` = G.jungle grade (`shots.ts:135`): bokeh aperture
  0.0012 / maxblur 0.007, bloom strength 0.16, grade saturation
  0.8 / warm 0.04 / vignette 0.24 / grain 0.012. Bloom threshold
  0.85 and radius 0.6 are the format defaults (spec §9) — the
  ShotLook carries no threshold. Bokeh `focus = 7.725` is
  derived: |eye − target| with the camera evaluated at
  `time` (see camera note). Post `grade.time = 18.67` (grain
  seed = frame t, as in the engine).

### Camera (push-in 50°)
- `camera.keys`: film-time keys [15.95 → from (1.0, 1.8, 7.6)],
  [21.384 → to (0.85, 1.65, 6.6)], target (0.8, 1.1, −0.6) held,
  fovY 50 (`shots.ts:370` shoot call; src/film.rs:1479 s04).
  Static eye/target repeat the first key (the §14.4 fallback law).
- Derived/approximate: the staging eases with
  `camK(lt, d)` = smootherstep with hold 0.1 (`core/motion.ts:41`;
  src/film.rs:1478 cam_k(lt, d, 0.1)), while the v2 key evaluator
  applies plain smootherstep between keys (spec §14.4). Endpoints
  are exact; mid-shot camera position differs by the hold term.

### Characters
- Looks from the cast registry (`shots.ts:27–33`): compA body
  #68744E / cap #525C3C; compB body #59613F / cap #59613F /
  scarf #8A7A52; Onoda body #5C6644 / cap #4C5638 with the worn
  shade #878E6E (`shots.ts:239`, a non-identity field). Onoda is
  the JG perspective instance (`perspective: true`,
  `shots.ts:239`); the companions are not (`shots.ts:241–242`).
- compA/compB `state` = POSES.celebrate (`core/poses.ts:13`:
  mood laugh, armL/armR [150, 14], bob −8) with the shot's own
  overrides (`shots.ts:361–364`): compA mood stays laugh, compB
  mood → happy; celebrate's static bob −8 is replaced by the
  bounce channel below, evaluated at `time` (compA −3.9234,
  compB −3.4758). Arms are constant through the shot, so they
  live in `state`, not in a channel.
- Onoda `state`: mood suspicious, armR [16, −6], armL [10, 6]
  (`shots.ts:366–368`); headTurn is the poseTrack channel below,
  evaluated at lt = 2.72 → −0.349993 in `state`.
- Blink `state` values are the closed-form blink functions
  evaluated at t = 18.67 (all three are 0 at that instant):
  blinkDirector seeds onoda 3 / compA 7 / compB 11
  (`shots.ts:30–33`; `core/motion.ts:260`), Onoda additionally
  blinkAt the 'message' word beat 20.7 (`shots.ts:368`;
  `core/motion.ts:285`; anchor from `film/onoda.json`).

### Keyframed channels (additive `channels` fields)
The v2 loader reads a character's static `state` at `time` only
(spec §14.3); time-varying formulas are recorded in `channels`
objects (ignored by the loader, implemented by the film
evaluator in src/film.rs s04, lines 1475–1503):
- compA `bob`: `-4.5*abs(sin(4.6*t))` (`shots.ts:362`).
- compB `bob`: `-4.5*abs(sin(4.6*t + 1.2))` (`shots.ts:364`).
- Onoda `headTurn`: poseTrack keys [[0, 0.5], [2.2, −0.35, 1.9],
  [4.6, 0.3, 1.9]] on the shot-local clock lt (`shots.ts:367`;
  semantics `core/motion.ts:188`, port src/film.rs:147–165).
- Fire (top-level `channels`): fireLight intensity
  `(4.6 + 0.7·sin(17t) + 0.5·sin(7.3t)) · level`, level 1, and
  the flame/flame2 scaleY flicker pair (`shots.ts:287–292`
  fireLevel(1, t); port src/film.rs:1224). The flame nodes'
  static scaleY in this file is that formula at `time`
  (0.94566 / 1.05434); the `lights` point intensity stays at the
  constructor base 5.0, as in character-v2.scene.json.

### Inexpressible in format v2 (declared, not substituted)
1. **Rifle as a scene node.** The v2 character node has no
   attachment field and the only rifle the loader knows is the
   v1 code-ref `ab1-onoda-rifle`, whose transform is S01's hand
   anchor — using it here would silently mis-stage the prop.
   The rifle is recorded in the top-level `attachments` field
   (hand R, offset [0, 0.03, 0.07], model `makeRifle()`
   `shots.ts:74–83`, attach `shots.ts:254`) and is drawn by the
   film evaluator (src/film.rs char(..., rifle=true)).
2. **Channel evaluation** (above): loader renders the state at
   `time`; the formulas exist as data but no loader code path
   evaluates them over the shot's duration.
3. **Fire light flicker as a lights field** — same reason;
   recorded as a top-level channel.
4. **Caption strips**: chunk ids/timing/fit are data (above);
   the strip images themselves are film assets
   (film/gen_assets.py), not scene-format textures — the S01
   file's caption-billboard route is a per-frame texture and
   would need all five strips plus per-frame selection.

## S02 — s02.scene.json (SH shore, noon, film t 5.907–13.061)

### Timing / shot identity
- `shot.at = cut(1) = L(1) − 0.06 = 5.907`, `end = cut(2) = 13.061`,
  `duration = 7.154` — project `shots.ts:125` (cut rule) with line
  starts from `film/onoda.json` lines[1].start = 5.967,
  lines[2].start = 13.121. Matches INVENTORY.md S02 row (5.91–13.06).
- `time = 9.484` is the file's evaluation instant (mid-shot,
  lt = 3.577 = d/2): every character `state` in the file is the
  state at that t, per the v2 loader law (talerender
  `src/scenefile.rs:756`). At that instant the camK driver sits
  exactly at its midpoint: k = 0.5, travelled = 3.4,
  phase = 17.22808837890625, bob = 0.0064174639992415905
  (both f32, derived under Characters).
- Captions: chunk ids 13–17 (line 1) with index/s/e/text/fit
  copied from `film/onoda.json` captions[13..17]. Chunk 12
  ("ended.", line 0, e = 5.937) is still on screen for the first
  0.03 s of the shot; it belongs to line 0 and is not listed.
  Chunk 17 ("in 1944.", e = 13.15) overruns the cut by 0.089 s
  into S03; it is line 1's chunk and is listed.

### Set (SH shore)
- Nodes follow the SH builder in build order (`shots.ts:197–221`):
  sky dome (199; `core/world.ts:149`), sand disc (203), sea/foam
  (205/207), hills + their mini trees (209; `core/world.ts:180`),
  7 shore trees (210; `core/world.ts:189`), 2 palms (211–212;
  `shots.ts:85`), boat (214–218), crates (219–220;
  `core/world.ts:216`), barrel (221; `core/world.ts:215`),
  clouds (202; `core/world.ts:167`).
- Sea/foam are 0.02-thin boxes, not planes: format v2 has no
  colour XZ-plane kind (the textured kinds only, talerender
  `src/scenefile.rs` tex_geo), so the file uses the film port's
  stand-in (`src/film.rs:953–955`) — box centred on the source
  plane's y, top face 0.01 above it. Sea metalness 0.08 is not
  a format material field (only `roughness` is carried,
  `src/scenefile.rs:497–500`) — recorded here only.
- Hills: the source hill material also carries a grass map
  (`T.grass(col, [6, 3])`, `core/world.ts:183`); a
  colour-pipeline sphere cannot carry a texture in v2, so the
  hills are colour + shadeHash only (the film port renders them
  mapped, `src/film.rs:825`). Hill mini-trees use the source
  `tree()` default colour `#4E9A3C` (`core/world.ts:189`); the
  ab1/film.rs transcription uses `#3E7A34` (`src/film.rs:843`)
  — the file follows the source. Mini-tree placement/scale
  replicates `src/film.rs:837–844` arithmetic exactly (the
  generator was validated to reproduce s01.scene.json's
  hill-0-tree-0 values bit-for-bit before use).
- Where the project source and the ab1/film.rs transcription
  differ, the file follows the PROJECT, declared here: tree
  trunk radial segments 12 (world.ts `cyl()` default) vs 7 in
  the transcription; shore trees carry NO `rotation.y`
  (`shots.ts:210`) vs `i * 1.7` in `src/film.rs:960`; shore
  tree kinds per source (even i = round, odd i = cypress,
  `shots.ts:210`) vs the port's `round = i % 2 == 1`
  (`src/film.rs:960`), which swaps them; palm frond/tip
  colours per `shots.ts:94–100` (frond: odd i → leaf
  `#3F7D33`, even i → leaf2 `#4C8A3A`; tips the opposite) vs
  the transcription's same-colour frond+tip pairs; palm tip
  spheres 8×5 (source) vs 10×6; sand disc 40 segments (source
  `CircleGeometry(30, 40)`) vs 48 in the port.
- Crates: the source crate is planks-textured
  (`T.planks('#B07A48', [0.5, 0.5])`, `core/world.ts:216`);
  file and port carry the base colour only. Barrel body:
  source is planks `#9A6337` (`core/world.ts:215`); the
  stand-in colour `#6E4525` follows the established
  transcription (s04.scene.json; `src/film.rs` build_sh).
  Barrel band metalness 0.4 is not a format field — recorded
  here only. `roughness` is written wherever the source sets
  it explicitly (sand 1, sea 0.35, foam 0.9, hull 0.9, palm
  bark 0.95, leaves/nuts 0.9); materials at the `mat()` default
  0.85 (`core/world.ts:44`) omit it.
- Clouds: billboard nodes, width 2s / height s (the source
  sprite scale, `core/world.ts:169`). The source sprite tint
  `#EFF4EA` has no billboard field in v2 (the loader's
  billboard is untinted, `src/scenefile.rs:301`) — not carried.
- Sky: the format sky-gradient carries top/horizon only; the
  source dome's below-horizon ground band (`#BFE4F7` default,
  `core/world.ts:149`) is not expressible. `clear` = the
  horizon `#E4EEDC` (the S04 pattern).

### Atmosphere / lights / post
- `atmosphere: "noon"` — `shots.ts:332` applyAtmosphere(SH.scene,
  'noon', { rig: SH.sun, sky: false }); preset values in
  project `core/env.ts:76–84`.
- `lights`: key = noon preset `#FFF0D6` ×2.6 at the SH sunlight
  position [−8, 11, 7], target [0, 0, −4] (`shots.ts:200–201`),
  shadowBox 17, far 80, bias −0.0006 (sunlight overrides,
  `core/world.ts:172`), normalBias 0.02 (`core/engine.ts:63`).
  Rim `#FFFFFF` ×0.4 at [−3, 3, −3] (lightRig rim; the noon
  rimI 0.4 leaves the sunlight rim unchanged). Hemisphere =
  the POST-atmosphere values `#CFE6FF` / `#C48A5A` ×1.25:
  applyAtmosphere redresses the hemisphere light because
  lightRig adds it inside rig.group (`core/engine.ts:63–72`;
  `core/env.ts:164`). Declared divergence between sources:
  the SH sunlight call's creation values are sky `#DCECD2` /
  ground `#8A7A52` ×1.15 (`shots.ts:200`) — those are what
  s04.scene.json carried for JG; this file follows the
  evaluated state, matching `src/film.rs:1176` rig_noon and
  scenes/film/s01.scene.json. envIntensity 0.25 (sunlight
  `env` default, `core/world.ts:172`). No `points`: the shore
  has no point lights (`src/film.rs:1400` passes
  PointState::off()).
- `post` = G.shore (`shots.ts:136`) with the shot's aperture
  override 0.0007 (`shots.ts:338`): bokeh maxblur 0.007,
  bloom strength 0.16 (threshold 0.85 and radius 0.6 are the
  engine's fixed pass values, `core/engine.ts:53`), grade
  saturation 0.84 / warm 0.05 / vignette 0.22 / grain 0.012,
  exposure 1.0 (the ShotLook carries no exposure → engine
  default, `core/engine.ts:48–58`). Bokeh `focus =
  6.5050458908081055` (f32) is derived: shoot()'s default
  focus = |eye − target| (`shots.ts:127`) with the crane
  evaluated at `time` (eye (1.45, 2.475, 1.5), target
  (−0.05, 1.2, −4.7)). Post `grade.time = 9.484` (grain seed
  = frame t, as in the S01/S04 files).

### Camera (crane down, 46°)
- `camera.keys`: film-time keys [5.907 → eye (2.0, 3.1, 2.4),
  target (−0.1, 1.25, −5.2)], [13.061 → eye (0.9, 1.85, 0.6),
  target (0, 1.15, −4.2)], fovY 46 (`shots.ts:337–338`:
  MO.crane endpoints + the shoot look; crane is a plain lerp
  on k, `core/motion.ts:380`). Static eye/target repeat the
  first key (the §14.4 fallback law, as in S04).
- `shot.cameraTrack = { kind: "camK", hold: 0.03 }`: the
  staging eases the crane with `camK(lt, d, 0.03)`
  (`shots.ts:333`; `core/motion.ts:41`), while the v2 key
  evaluator applies plain smootherstep between keys (spec
  §14.4). Endpoints are exact; mid-shot camera position
  differs by the hold term only (max |Δk| = 0.0166, ≈4 cm on
  the 2.45 m crane path).

### Characters
- Looks from the cast registry (`shots.ts:28–30`): onoda body
  `#5C6644` / cap `#4C5638`; compA `#68744E` / `#525C3C`;
  compB `#59613F` / `#59613F` with scarf `#8A7A52`. The SH
  instances are `person(k)` with NO opts (`shots.ts:246–248`):
  no `perspective` field and no shade override (contrast the
  JG instances). Height is the Toon default 1.75
  (`core/toon.ts:519`), so no `height` field is written.
- person() sets envMatch: true (`shots.ts:35`), but the noon
  tint is the identity (preset tint, `core/env.ts:76–84`;
  `src/film.rs:1254` TINT_NOON = [1, 1, 1]) — nothing to carry.
- `state` at `time`: Onoda mood `determined`, armR [16, −6],
  armL [10, 6] (the override set, `shots.ts:335`); companions
  mood `neutral` with arms at the drawToon defaults [8, 6]
  (march() passes no arms, `shots.ts:334`; defaults
  `core/toon.ts:179`). All three: legs `walk` (k = 0.5 ≤ 0.97),
  phase 17.22808837890625, bob 0.0064174639992415905, blink 0
  (blinkDirector seeds onoda 3 / compA 7 / compB 11 evaluated
  at t = 9.484; `shots.ts:41`, `core/motion.ts:260`).
- Phase-lock: phase and bob are computed ONCE from the shared
  travelled (`shots.ts:333`) and passed identically to all
  three marchers; the companions' dz offsets (−0.9 / −1.5)
  shift position only, never phase. The identical
  `phase`/`bob` values in the three states are that lock
  expressed at `time`. Positions at `time`: onoda
  (0, 0, −7.6), compA (−1.4, 0, −8.5), compB (1.4, 0, −9.1)
  (= x, 0, −11 + 3.4 + dz).
- shTani exists in the SH set but is staged hidden in this
  shot (shStage without tani, `shots.ts:332`) — no node.
- Facing is evaluated from the camera per frame (project
  `face()` in `core/toon.ts`; the loader does the same) —
  nothing authored.

### Keyframed channels (additive `channels` fields)
The v2 loader reads a character's static `state` at `time` only
(spec §14.3); time-varying formulas are recorded in per-character
`channels` objects (ignored by the loader, implemented by the
film evaluator in `src/film.rs:1398` s02):
- `position`: at(x, 0, −11 + travelled + dz) with the shared
  driver travelled = camK(lt, d, 0.03) × 6.8 (`shots.ts:333–336`).
- `phase`: walkPhase(travelled, 0.62) (`core/motion.ts:228`),
  shared by all three (see phase-lock above); `bob`:
  walkBob(phase, 5) (`core/motion.ts:241`); `legs`:
  k > 0.97 ? 'stand' : 'walk' (`shots.ts:334`); `blink`:
  blinkDirector with the character's seed.
- Declared port divergence: the film evaluator clamps the
  driver — its travelled uses min(k, 0.97)
  (`src/film.rs:1403–1405`) — so the march halts when the legs
  switch to stand. The project source keeps travelled = k × 6.8
  for the whole shot (in the last 3% of k the unit slides
  0.204 m with planted feet). The channels record the SOURCE
  formulas; the clamp is the evaluator's.

### Inexpressible in format v2 (declared, not substituted)
1. **Rifle as a scene node.** Same as S04: recorded in the
   top-level `attachments` field (hand R, offset [0, 0.03, 0.07],
   model `makeRifle()` `shots.ts:74`, attach `shots.ts:255`)
   and drawn by the film evaluator (`src/film.rs:1423`,
   char(..., rifle = true)). The rifle's own geometry and
   materials, recorded here since the attachment carries no
   model (modelled along +Y, grip at origin, butt −0.44 /
   muzzle +0.68): stock box(0.075, 0.52, 0.13) wood `#6E4F30`
   rough 0.85 at (0, −0.17, 0); receiver box(0.06, 0.1, 0.09)
   steel `#3A3D42` metalness 0.55 / rough 0.5 at (0, 0.13, 0);
   barrel cyl(0.016, 0.016, 0.55, seg 8) steel at (0, 0.42, 0);
   front sight box(0.02, 0.05, 0.02) steel at (0, 0.68, 0);
   grip swell box(0.05, 0.05, 0.16) wood at (0, −0.02, 0.05).
   Its per-frame anchor is the syncAttachments hand formula
   (`core/toon.ts:681`) — the loader has no equivalent.
2. **Channel evaluation** (above): the loader renders the
   state at `time`; the formulas exist as data but no loader
   code path evaluates them over the shot's duration.
3. **Textured / plane surfaces**: hills grass map, crate
   planks texture, sea/foam as true zero-thickness planes,
   cloud sprite tint, the sky dome's ground band, and
   material metalness (sea 0.08, barrel bands 0.4, rifle
   steel 0.55) — see the Set notes.
4. **Caption strips**: chunk ids/timing/fit are data (above);
   the strip images themselves are film assets
   (film/gen_assets.py), not scene-format textures.

## S01 — s01.scene.json (JG noon, film t 0.00–5.907) — seat W03

### Timing / shot identity
- `shot.at = cut(0) = 0`, `duration = cut(1) = L(1) − 0.06 =
  5.907`, `shot.line = 0` — `shots.ts:125` (cut rule) with line
  starts from `film/onoda.json` lines[0].start = 0.3,
  lines[1].start = 5.967 (generated from the project's
  `data/lines.json`). INVENTORY.md's S01 row (0.00–5.91) is the
  rounded form of 5.907.
- `time = 0.0` is the file's evaluation instant: the character
  `state` and the fire point intensity are the values at t = 0,
  per the v2 loader law (src/scenefile.rs load: the state in the
  file is the state at `time`).
- `shot.fades { in: 0.25, out: 0.4 }` — the main.ts first-frame
  fadeIn 0.25 and S01's fadeOut(0.4) (data-only; fades are not a
  scene-format field).
- `shot.cues` — `cue(st + 0.5, 'chirp', { gain: 0.2, dur: 1.6 })`
  from the S01 draw (`shots.ts:316–330`); audio is outside the
  scene format, recorded as data only.
- Captions: the 13 chunk ids cap_000…cap_012 (line 0) with s/e/
  text copied from `film/onoda.json` captions[0..12] — the
  captions.ts chunking of `script.json` line 0's caps string over
  `data/words.json` (gap 0.18 / lead 0.3). Ids are the strip
  names minus `.png`; all 13 chunks have fit 1.0.

### Set (JG camp)
- Nodes/textures are `scenes/ab1.scene.json`'s JG set verbatim
  (373 of its 376 nodes) — the M5-proven staging of the same set
  S01 uses (`shots.ts:141–188` JG builder; sky gradient
  #6E9CC0/#D8E8DA, grass, dirt, 3 hills + trees, 19-tree ring,
  3 palms, 900-tuft scatter, camp incl. fire ring/logs/flames,
  3 cloud billboards).
- Dropped from the ab1 copy: the S01 code-ref Onoda card and the
  rifle code-ref (replaced by the v2 character node / declared
  attachment below), the separate blob-shadow mesh (the v2
  character node emits its own blob), and the 'fighting' caption
  billboard + its texture (film captions enter as chunk ids;
  strips are film assets, cf. S04 item 4).

### Atmosphere / lights / post
- `shot.atmosphere: "noon"` — `noonJG()` =
  applyAtmosphere(JG.scene, 'noon', { rig: JG.sun, sky: false })
  (INVENTORY S01; preset in `core/env.ts:76–84`).
- `lights` block is the rig AFTER applyAtmosphere('noon'):
  applyAtmosphere step 3 (`core/env.ts` applyAtmosphere) sets
  the rig's key colour/intensity, rim intensity and hemisphere
  FROM the noon preset, and leaves the key position at the JG
  sunlight call's [7,12,5] (noon sets no keyPos). Result: key
  #FFF0D6 ×2.6 at [7,12,5] → target [0,0,−2], shadow true,
  shadowBox 17 (JG sunlight call `shots.ts:145`), far 80 /
  bias −0.0006 (`core/world.ts:172–176` sunlight), normalBias
  0.02 (`core/engine.ts` lightRig); rim #FFFFFF ×0.4 at
  [−3,3,−3]; hemisphere #CFE6FF / #C48A5A ×1.25 (noon preset);
  envIntensity 0.25. This equals src/film.rs:1176 rig_noon
  exactly. DECLARED DIVERGENCE from the S04 entry above: S04
  followed the §14.2 spec example / character-v2.scene.json,
  whose hemisphere carries the JG sunlight call's PRE-atmosphere
  values (#DCECD2 / #46603A ×1.15). Both cannot be the rendered
  rig; this file follows applyAtmosphere + film.rs. The
  integrator should reconcile character-v2/S04 to one choice.
- Fire point: #FF9A3C at [−1.35, 0.6, −2.45], distance 7,
  decay 1.6 (`shots.ts` JG camp PointLight('#FF9A3C', 5, 7, 1.6)
  at (FX, 0.6, FZ)). Static `intensity = 4.6` is the fireLevel
  formula at t = 0 (the constructor's 5.0 is overwritten by
  fireLevel on the first frame); the flicker formula is the
  point's additive `channel` field (inexpressible item 3).
- `post` = G.jungle (`shots.ts:135`): bokeh aperture 0.0012 /
  maxblur 0.007, bloom strength 0.16 (threshold 0.85 / radius
  0.6 are `core/engine.ts` composer constants — the ShotLook
  carries neither), grade saturation 0.8 / warm 0.04 / vignette
  0.24 / grain 0.012, exposure 1.0 (S01 passes G.jungle directly,
  no gradeOf). Bokeh `focus = 6.176479816436768` is DERIVED:
  |eye₀ − target₀|, the shoot() default (focus = distance to
  the mixed target, `shots.ts` shoot); in the source it is a
  per-frame channel, stored here at its t = 0 value.
  `grade.time = 0.0` = the file's `time`.

### Camera (push-in 44°)
- `camera.keys`: [0 → eye (0.85, 2.7, 5.4), target
  (0.35, 1.12, −0.55)], [5.907 → eye (0.55, 1.62, 3.2), target
  (0.35, 1.02, −0.55)], fovY 44 at both keys — the S01 shoot()
  call (`shots.ts:316–330`; `{...G.jungle, fov: 44}`; port
  src/film.rs s01). Static eye/target/fovY repeat the first key
  (the §14.4 fallback law).
- Derived/approximate: the staging eases with `camK(lt, d)`
  (hold 0.1, `core/motion.ts` camK), while the v2 key evaluator
  applies plain smootherstep between keys (spec §14.4).
  Endpoints exact; mid-shot position differs by the hold term.
  The source easing is recorded as `shot.cameraTrack
  { kind: "camK", hold: 0.1 }` (inexpressible item 2).

### Character
- Onoda at [0.35, 0, −0.55] (`shots.ts` S01 at(0.35, 0, −0.55)),
  `perspective: true`, look from the cast registry
  (`shots.ts` LOOK table: body #5C6644, hat cap, hatCol
  #4C5638) with the worn shade #878E6E (as in
  scenes/character-v2.scene.json).
- `state` at t = 0: mood 'determined', armR [16, −6],
  armL [10, 6], lean 1 (`shots.ts:316–330` S01 draw);
  headTurn −0.45 = poseTrack key 0 (channel below); blink 0
  (director clock at t = 0 sits 4.11 s into a 4.4 s interval,
  past the 0.12 s window; the 'years' beat at 4.2 is unreached);
  camPitch 0.29956525564193726 is DERIVED: camPitchOf at t = 0
  = atan2(2.7 − 1.75·0.6, hypot(0.5, 5.95)) / 0.9 (`shots.ts`
  camPitchOf; port src/film.rs cam_pitch_of).

### Keyframed channels (additive `channels` fields)
The v2 loader reads a character's static `state` at `time`
only (spec §14.3); time-varying formulas are recorded in
`channels` objects (ignored by the loader, implemented by the
film evaluator in src/film.rs s01):
- Onoda `headTurn`: poseTrack keys [[0, −0.45], [2.6, 0.4, 2.2],
  [5.2, −0.2, 2.2]] on the shot-local clock lt (`shots.ts` S01
  draw; semantics `core/motion.ts:188–206` poseTrack — a key's
  move runs over its moveSecs ending at its time).
- Onoda `blink`: blinkOf = blinkDirector(seed 3) (`shots.ts`
  blinkOf record; `core/motion.ts` blinkDirector) max blinkAt
  at the 'years' beat, Wd('years', 0) = 4.2 (`data/words.json`
  line 0; anchor years0 in `film/onoda.json`).
- Onoda `camPitch`: kind camPitchOf, divisor 0.9 (derived per
  frame from the evaluated camera).
- Fire point `channel`: intensity
  `(4.6 + 0.7·sin(17t) + 0.5·sin(7.3t)) · level`, level 1, and
  the flame meshes' scaleY flicker pair f / 2−f with
  f = 1 + 0.16·sin(13t) + 0.07·sin(29t + 1) (`shots.ts:287–292`
  fireLevel(1, t); port src/film.rs:1224 fire_light — cross-
  check: the formula gives 4.35186 at t = 1.5, the M5 value).
  The flame nodes in this file keep ab1's static (t = 1.5)
  scale; at the file's `time` = 0 the formula gives
  f = 1.0589029689365528 — the evaluator's channel governs.

### Inexpressible in format v2 (declared, not substituted)
1. **Rifle as an attachment.** The v2 character node has no
   attachment field (src/scenefile.rs character branch parses
   none) and the only rifle the loader knows is the v1 code-ref
   `ab1-onoda-rifle`. The rifle is recorded as an additive
   `attachments: [{ kind: "rifle", hand: "R", offset:
   [0, 0.03, 0.07] }]` on the character node — attach call
   `shots.ts:254`, geometry `makeRifle()` `shots.ts:73` — and
   is drawn by the film evaluator (src/film.rs
   char(..., rifle=true)). (Placement differs from S04's
   top-level `attachments` field; same content.)
2. **camK hold** in camera keys (see Camera note).
3. **Channel evaluation** (headTurn / blink / camPitch / fire
   flicker / flame scaleY / bokeh focus): the loader renders
   the state at `time`; the formulas exist as data but no
   loader code path evaluates them over the shot's duration.
4. **Caption strips**: chunk ids/timing are data (above); the
   strip images are film assets (film/gen_assets.py), not
   scene-format textures.
5. **Audio cue + fades**: recorded in `shot.cues` / `shot.fades`
   as data only; neither exists in the scene format.

## S06 — s06.scene.json (JG FOG, film t 26.13–30.39)

### Timing / shot identity
- `shot.at = cut(5) = 26.13`, `end = cut(6) = 30.39`,
  `duration = 4.26` — project `shots.ts:406–421` (S06) with line
  starts from `film/onoda.json`. Source line L5 "His friends gave
  up or died, until he was alone." plus the head of L6 (its tail
  is S07). Matches INVENTORY.md S06 row (26.13–30.39).
- `time = 3.6` is the file's evaluation instant (shot-local lt;
  film t = 29.73), chosen just after the sit (sitAt = 3.55) so the
  still carries the sitting state, the companions at their exit
  endpoints, and the fire at its dying level. Every character
  `state`, the camera base eye/target, the flame scales and the
  fire point intensity in the file are the state at that t, per
  the v2 loader law (src/scenefile.rs).
- Captions: chunk objects 31–39 (index/s/e/text/fit copied from
  `film/onoda.json` captions[31..39]). Chunk 31 ("trick.", line 4)
  ends at 26.19, 0.06 s into the shot; chunks 32–36 are line 5;
  chunks 37–39 are line 6, and 39 ("for decades,") runs past the
  cut into S07.

### Atmosphere / fog (the inventory's declared risk)
- Top-level `"atmosphere": "fog"` + explicit `"fog"` block:
  **color #C9CFC2, near 4.5, far 20.0** — the shot's own override
  `JG.scene.fog = new THREE.Fog(0xC9CFC2, 4.5, 20)` at
  `shots.ts:408`, NOT the env.ts fog preset's depth range
  (3–38, #E6E9EA). The preset's remaining values live in
  `"atmospherePreset"` (sky #C8CDD0/#E6E9EA/#D8DCDE, cloudTint
  #EDEFEF, tint sat 0.82 / exposure 0.94 / fogColor #E6E9EA /
  fogMix 0.35 — `env.ts` ATMO.fog; film.rs `tint_fog()`).
- Sky texture retinted to the fog preset (top #C8CDD0, horizon
  #E6E9EA) and `clear` = #E6E9EA; all other set values are noon
  JG's (below).

### Set (JG camp)
- Nodes/textures are `scenes/ab1.scene.json`'s JG set verbatim
  (grass ground, dirt, hills, 19-tree ring, 3 palms, 900-tuft
  scatter, camp: fire ring/logs/seat log/lean-to/crates/barrel/
  bedroll, 3 cloud billboards) — the same set S06 builds
  (`shots.ts:406–408` jgStage over the JG builder). Dropped from
  ab1: the card + rifle code-refs and the S01-position blob
  (replaced by the v2 character nodes, which emit their own
  blob + card), and the caption billboard (captions are data).
- Seat log (the sit target): cylinder r 0.17, centre
  (−0.5, 0.17, −1.8) — its top is exactly y = 0.34, which is the
  `sitOn: 0.34` in Onoda's state.

### Lights (v2 block; env.ts fog rig over the JG noon rig)
- key #EAF0F2 @ 0.7, position [7,12,5], target [0,0,−2], shadow,
  shadowBox 17, far 80, bias −0.0006, normalBias 0.02 — ATMO.fog
  keyCol/keyI over the noon rig geometry (film.rs `rig_fog()`).
- rim #FFFFFF @ 0.15 (fog rimI) at [−3,3,−3]; hemisphere
  #E6E9EA/#B8BCBE @ 0.9 (fog hemi); envIntensity 0.25.
- Fire point #FF9A3C at (−1.35, 0.6, −2.45), distance 7,
  decay 1.6, intensity **0.63142693** = the evaluated track value
  at the still (see Fire).

### Post
- `post.bokeh` aperture 0.0012 / maxblur 0.007, focus
  **6.30620193** = |eye − target| at the still (film.rs computes
  focus the same way). Bloom strength 0.16 (jungle grade);
  grade vignette 0.24, grain 0.012, warm 0, **sat 0.52** (the
  shot's gradeOf sat override, `shots.ts:409`), grade time =
  film t 29.73. All from film.rs post for S06 / gradeOf jungle.

### Camera
- Push-in 46°: keys t 0 eye [1.7, 1.9, 4.4] → t 4.26 eye
  [1.15, 1.68, 2.9], target [−0.3, 1.05, −3.2] constant, fovY 46
  (`shots.ts:420` shoot + camK(lt, d, 0.1)). At the still:
  k = smootherstep(((3.6/4.26) − 0.1)/0.8) = **0.99708719**,
  eye = **[1.15160203, 1.68064082, 2.90436912]**.

### Characters (state at lt = 3.6; channels carry the motion)
- **compA** (cast.ts:29 — body #68744E, cap #525C3C): mood `sad`,
  headTilt −4. Walk channel: wk = seg(lt, 0.1, 3.55); at the
  still wk = 1 → position **[−4.0, 0, −10.5]**, legs `stand`,
  phase = walkPhase(5.96, 0.62) = **30.19982529**,
  bob = walkBob(phase, 5) = **0.31123933**. Blink seed 7.
- **compB** (cast.ts:30 — body #59613F, cap #59613F, scarf
  #8A7A52): mood `tired`, headTilt −4. At the still wk = 1 →
  position **[−2.6, 0, −10.9]**, legs `stand`,
  phase = walkPhase(6.1, 0.62) = **30.90921783**,
  bob = walkBob(phase, 5) = **2.57349014**. Blink seed 11.
- **Onoda** (body #5C6644, shade #878E6E, cap #4C5638,
  perspective true): POSES.mourn (poses.ts:14 — lean 5,
  headTilt −4, bob 4) with the shot's overrides mood `sad`,
  armR [16,−6], armL [8,5] (`shots.ts:416–418`); position
  z = lerp(−1.48, −1.76, seg(lt, 3.40, 3.80)) = **−1.62** at the
  still. Sitting: legs `sit`, **sitOn 0.34** (the seat log's
  top), squash = −0.04·sin(seg(3.6, 3.55, 3.85)·π) = **−0.02**.
  Blink seed 3. Rifle: node `attachments` entry (hand R,
  offset [0, 0.03, 0.07]) from `shots.ts` jgStage({rifle:true})
  + syncAttachments; geometry is makeRifle()'s five draws.

### Fire (dying)
- Top-level `"fire"` block: level keys 1.0 until lt 2.95 → 0 at
  lt 3.70, law = fireLevel in `shots.ts:410`
  (level = clamp(1 − seg(lt, sitAt−0.6, sitAt+0.15)), sitAt
  3.55). At the still: level = **0.13333334**; flicker
  f = 1 + 0.16 sin(13t) + 0.07 sin(29t+1) at film t 29.73 =
  **1.03678211**; flame node scales baked to
  [0.13333334, 0.13823761, 0.13333334] and
  [0.13333334, 0.12842906, 0.13333334] (ab1's flat flame
  colours/geometry kept); point intensity =
  (4.6 + 0.7 sin(17t) + 0.5 sin(7.3t)) · level = **0.63142693**.

### Sources followed over film.rs where they differ (flagged)
- walkBob amplitude **5** (shots.ts:413) — film.rs s06 passes
  6.0. This file follows shots.ts.
- Onoda's sitting state keeps POSES.mourn's lean 5 / headTilt
  −4 / bob 4 under the overrides (shots.ts merges the pose);
  film.rs s06's sitting branch sets only mood/arms/squash.

### Inexpressible in the v2 format (carried as data / declared)
1. **Fog itself**: the v2 still loader parses no `fog` /
   `atmosphere` field (fog is implemented in the film
   evaluator, src/film.rs FrameSpec.fog). The file carries the
   exact block as data; a still rendered through the loader
   alone has no depth fog.
2. **Grade exposure 0.96**: `post.grade` has no exposure field
   (the film PostState does). Recorded only in the shot's
   grade chain (jungle grade + fog sat override).
3. **Channel evaluation**: character `keys`/`walk`/`sit`
   channels, the fire level/intensity/flicker track, blink
   directors (seeds 3/7/11) and the camera keys' camK hold are
   data; the loader renders the single evaluated state at
   `time`. Blink is omitted from `state` (= 0), the
   character-v2 precedent.
4. **Rifle geometry**: the character node carries the
   attachment (hand/offset) as data; the loader has no rifle
   field. The ab1 `ab1-onoda-rifle` code-ref was NOT reused —
   its baked transform is the S01 standing anchor, wrong for
   the sit pose; the film evaluator draws the rifle from the
   attachment (src/film.rs char(..., rifle=true)).
5. **Cloud tint #EDEFEF**: the cloud billboards share the cloud
   generator texture; per-atmosphere sprite tint is not
   expressible — they render at the noon tint.
6. **Sky ground band #D8DCDE**: the sky-gradient generator
   takes top/horizon only; the fog preset's skyGnd is recorded
   in `atmospherePreset` but not painted.
7. **envMatch character tint** (fog tint sat 0.82 / exposure
   0.94 / fogMix 0.35): a film-renderer per-character uniform,
   not a character-node field; recorded in `atmospherePreset`.
8. **Caption strips**: chunk ids/timing/fit are data (above);
   the strip images are film assets (film/gen_assets.py), not
   scene-format textures.

## S07 — s07.scene.json + s07-calendar.json (JG GOLDEN, film t 30.39–34.21) — seat W09

### Timing / shot identity
- `shot.at = Wd('decades', 6) − 0.25 = 30.64 − 0.25 = 30.39`
  (the S07 draw anchors its start on the 'decades' word, not on
  cut(6)); `end = cut(7) = L(7) − 0.06 = 34.269 − 0.06 = 34.209`;
  `duration = 3.819`. Anchors from the project's `data/words.json` /
  `data/lines.json`; matches INVENTORY.md S07 row and src/film.rs
  s07 (`st0 = 30.390`).
- `time = 0` is the file's evaluation instant (shot-local lt = 0):
  the character `state` and `post.bokeh.focus` are the values at
  shot start, per the v2 loader law (src/scenefile.rs §14.3). All
  shot-local times below are lt = film t − 30.39.
- Captions: `cap_039` "for decades," (30.29–31.55), `cap_040`
  "surviving on coconuts" (31.55–32.99), `cap_041` "and bananas."
  (32.99–34.27) — id/s/e/text/width/fit/strip copied verbatim from
  `film/onoda.json` captions (line 6). Independently recomputed
  from `data/words.json` + `script.json` caps under the
  `core/captions.ts` buildCaptions rules (s = first word s − 0.03;
  e = next chunk s; line-end rule not triggered, gap 0.15 ≤ 0.6) —
  identical. Chunk cap_038 "in the jungle" ends at 30.29, before
  the shot starts, and is not listed. cap_041 is cut off mid-window
  by the shot end (34.209 < 34.27), as in the film.

### Set (JG camp)
- Nodes/textures are `scenes/ab1.scene.json`'s JG set transformed:
  the same builder output S07 uses (`shots.ts` JG builder; S07
  calls `jgStage({onoda: true, rifle: true})`). Kept verbatim: sky
  dome, grass ground, dirt patch, 4 hills + trees, 19-tree ring,
  3 palms, 900-tuft annulus scatter, camp (8 fire stones, 3 fire
  logs, seat log, lean-to poles + roof, 2 crates, barrel + bands,
  bedroll), 3 cloud billboards.
- The palm Onoda walks to is the node with additive
  `"role": "walk-target"` — set position [4.6, 0, −4.6], scale
  1.05, lean (rotate_z) 0.14 (the other palms: [−5.2, 0, −5.4]
  s0.9 lean −0.2 rotY 1.2; [10.5, 0, −11] s1.15 lean 0.1).
- Dropped from the ab1 list, deliberately: the S01-staging nodes
  "Onoda card (painter, code until M4)" (replaced by the v2
  character node), "caption 'fighting'" (captions are data, above),
  "blob shadow" (the v2 character node emits its own blob draw,
  src/scenefile.rs character branch), the "rifle (attach3D...)"
  code-ref (see Inexpressible 1), and the two flame cones (see
  Inexpressible 5).

### Atmosphere / lights / post
- Atmosphere `golden` (`core/env.ts` preset; S07 calls
  `applyAtmosphere(JG.scene, 'golden', {rig: JG.sun})` with no
  `sky: false`, so sky + clouds are re-dressed): sky texture
  top #4A90D9 / horizon #FFD98A (= `clear`), cloud billboards
  carry additive `"tint": "#FFE3B0"`, fog #FFD98A near 30 far 140
  (top-level `fog`, the s06 shape — format v2 has no fog field;
  src/film.rs FrameSpec carries it).
- `lights` = film.rs `rig_golden(fire_light(0, t))`: key #FFB35C
  i2.3 at [−8, 4.5, 5], target [0, 0, −2], shadow box 17 / far 80 /
  bias −0.0006 / normalBias 0.02 (JG noon rig fields the
  atmosphere does not override); rim #FFFFFF i0.6 at [−3, 3, −3];
  hemisphere #FFE0B0 / #B07A48 i1.0; envIntensity 0.25.
- Fire point: house form (cf. S01) — intensity base 4.6 with a
  `channel {kind: "fireLevel", level: 0, formula: "(4.6 +
  0.7*sin(17t) + 0.5*sin(7.3t)) * level"}`. Level is
  `fireLevel(0, t)`, so the evaluated intensity is 0 for the
  whole shot (film.rs `fire_light(0.0, t)` = 0).
- `post` = film.rs `g_jungle()` with the s07 overrides:
  bloom strength 0.16 (threshold 0.85 / radius 0.6 written out,
  the loader defaults), bokeh aperture 0.0012 / maxblur 0.007,
  grade saturation 0.85 / warm 0.06 / exposure 1.02 (the shot's
  overrides of the golden preset grade) over the JG base
  vignette 0.24 / grain 0.012. `grade.time` = 30.39 (film t at
  shot start; the grain hash is time-seeded, per-frame value is
  film t). Field-name note: the files write `saturation` /
  `exposure` (house convention, cf. s01); the current loader's
  grade parser reads `sat` and has no exposure field — see
  Inexpressible 4.

### Camera (crane up 46°)
- `camera.keys`: lt 0 eye [1.4, 1.6, 3.4] target [1.4, 1.0, −1.8];
  lt 3.819 eye [3.6, 5.4, 7.2] target [1.8, 0.8, −2.4]; fovY 46
  throughout (`MO.crane` endpoints, `shots.ts` S07; shoot called
  with the mix pre-applied). Static eye/target/fovY = the lt-0
  key, per the character-v2 precedent.
- `shot.cameraTrack = {kind: "camK", hold: 0.1}`: the project
  mixes the crane on k = camK(lt, d) — holds at the ends,
  smootherstep between (`core/motion.ts`); the loader's keys use
  plain smootherstep per segment — see Inexpressible 2.
- Bokeh focus is `|eye − target|` per frame (shoot() default):
  5.2345 at lt 0 → 10.7963 at lt 3.819. Recorded as
  `post.focusChannel`; `post.bokeh.focus` carries the lt-0 value.

### Character (Onoda)
- v2 character node at the lt-0 state: position [−0.5, 0, −1.76],
  height 1.75, perspective true; look body #5C6644, cap #4C5638,
  shade override #878E6E (`person('onoda', {perspective: true},
  '#878E6E')`); state mood 'tired', look [0.2, −0.1], armL
  [12, 6], armR [12, −10], legs 'walk', phase 0, bob 4
  (walkBob(0, 4) = 4·(1−|sin 0|)), headTilt −3, camPitch 0.1108
  (camPitchOf at lt 0), blink 0, lean 0 (not set by the staging;
  engine default). Cross-checked field-by-field against
  src/film.rs s07's FullState construction.
- `envTint` (additive, on the node): the golden envMatch tint
  {sat 1.02, warm 0.08, exposure 1, fogColor #FFD98A, fogMix
  0.06} applied to the character card (film.rs `tint_golden()`);
  the v2 character node has no tint field.

### Keyframed channels (additive `channels` on the character node)
The v2 loader reads a character's static `state` at `time` only
(spec §14.3); the shot's animation is recorded in `channels`
objects (ignored by the loader, implemented by the film evaluator
in src/film.rs s07):
- `position` {kind: segLerp, segment [0.2, 1.85]}: wk =
  seg(lt, 0.2, 1.85); pos = lerp([−0.5, 0, −1.76],
  [2.2, 0, −2.3], wk) — the walk to the palm.
- `phase` {kind: walkPhase}: phase = (wk · 2.0) / 0.6 · π
  (travelled distance wk·2.0, stride 0.6).
- `bob` {kind: walkBob, amp 4}: bob = 4·(1 − |sin phase|).
- `legs` {kind: step}: 'walk' while wk < 1, then 'stand'.
- `armR` {kind: poseTrack}: keys [[0, 12], [1.91, 136, 0.3],
  [2.86, 34, 0.5]] (shoulder; key times are
  Wd('coconuts', 6) − st − 0.1 and + 0.85), elbow constant −10 —
  the reach up at the palm and settle. armL is static [12, 6].
- `mood` / `headTilt` {kind: step}: 'tired' / −3 until lt 1.2,
  then 'neutral' / 0.
- `holdL` {kind: fromTime, at lt 1.96}: the coconut plate appears
  in the left hand when film t ≥ Wd('coconuts', 6) − 0.05 = 32.35
  (version key 'coco1'). Plate = texture `coconut`
  (film/assets/coconut.png), origin x0 −100 / y0 −108 from
  film/assets/plates.json. The brief named props.ts as the prop
  source; in fact no coconut exists in `core/props.ts` or the
  `core/toon.ts` PROPS table — it is the project-local canvas
  PropFn `coconutProp` in `shots.ts`, whose drawing ops are
  transcribed in the texture entry's `sourceDrawing` (body arc
  r36 fill #6B4A2E, ink stroke #1F1C33 lw 6; three r5 dots
  #543820 at (−11,−12), (10,−14), (0,−2); sprout stroke #87A860
  lw 7 via quadratic (2,−34) ctrl (16,−52) → (34,−50); drawn at
  translate(x, y−8)).
- `blink` {kind: blinkDirector, directorSeed 3, evaluatedAt:
  film t}: Onoda's seed is 3 in every set; state.blink = 0 is
  the lt-0 value only in the sense "eyes open at shot start" —
  the director's pulses are a function, not a scalar.
- `camPitch` {kind: camPitchOf, divisor 0.9}: 0.1108 at lt 0 →
  0.4726 at lt 3.819 as the crane rises (formula in the channel).

### Calendar overlay (s07-calendar.json, referenced by `overlays`)
- Source: `drawCalendar(t, calT0)` in `shots.ts`, drawn after
  `blit()` inside the S07 draw; calT0 = st + 0.16 = film 30.55
  (overlay lt_cal = shot lt − 0.16). Composite order in the film
  path: calendar → fade → caption (src/film.rs).
- Full data in the companion file: frame 1080×1920; transform
  translate(352, 648) → rotate(−0.03) → scale(0.92, 0.92·sy);
  card 320×400 roundRect r14 fill #F4EBD6, stroke #1F1C33 lw 7,
  drop shadow rgba(20,16,10,0.35) at offset (+8, +10); header
  band 320×66 #8A3038 with "C A L E N D A R" (800 30px Inter,
  #F4EBD6, at (0, −166)); year (900 118px Inter, #1F1C33, at
  (0, 8)); "LUBANG ISLAND" (800 25px Inter, #8B8378, at
  (0, 118)); rule fillRect(−110, 158, 220, 5) #8B8378; flip
  shading rgba(30,25,18, dark·0.22) over the card.
- Cycle: years [1945, 1955, 1965, 1974], hold 0.62, flip 0.42,
  cycle 1.04; flips at lt_cal 0.62–1.04 / 1.66–2.08 / 2.70–3.12
  (sy = max(0.1, |cos(uπ)|), dark = 1 − sy, year swaps at
  u = 0.5); 1974 steady from lt_cal 3.12. Alpha =
  clamp(lt_cal/0.18) · clamp((4.4 − lt_cal)/0.35), life 4.4 s.
  A 'tock' cue (gain 0.32) fires at each flip start — recorded
  in `shot.cues` (film 31.17 / 32.21 / 33.25).
- The card is still on screen at the cut: the shot ends at
  lt_cal 3.659 of the 4.4 s life, alpha 1, year 1974 steady.
  The card's frame-crossing presence at t31–32 in the delivered
  film is the project's own staging; it is transcribed as
  keyframed overlay motion in s07-calendar.json, not "fixed".
- film.rs rasterises the card into plates (calendar-{year}.png +
  calendar-dark.png, local x [−180, 180], y [−220, 220] at
  2 px/unit) and composites CPU-side (`overlay_calendar`); the
  plate frame is recorded in the companion file.

### Inexpressible in format v2 (declared, not substituted)
1. **Rifle as a scene node.** The v2 character node has no
   attachment field and the only rifle the loader knows is the
   v1 code-ref `ab1-onoda-rifle`, whose transform is S01's hand
   anchor — using it here would silently mis-stage the prop.
   The rifle is recorded in the node's `attachments`
   [{kind: "rifle", hand: "R", offset: [0, 0.03, 0.07]}]
   (attach `shots.ts` rifleHeld(jgOnoda, 'R', [0, 0.03, 0.07]);
   geometry `makeRifle()` in `core/world.ts`) and is drawn by
   the film evaluator (src/film.rs char(..., rifle = true)).
2. **camK hold in camera keys**: the project's crane mix runs on
   camK(lt, d, hold 0.1); loader keys interpolate with plain
   smootherstep per segment. Endpoints and fov are exact; the
   mid-shot path differs by the hold law. Recorded in
   `shot.cameraTrack`.
3. **Channel evaluation** (above): the loader renders the state
   at `time`; the formulas exist as data but no loader code path
   evaluates them over the shot's duration.
4. **Grade exposure / saturation naming**: files write
   `saturation` + `exposure` (house convention); the loader's
   grade block reads `sat` and carries no exposure field, so
   exposure 1.02 is data-only until the format grows it.
5. **Flame visibility**: format v2 has no node `visible` flag.
   With fireLevel(0) the TS flame meshes are hidden and
   film.rs build_jg(fire = 0) emits none, so the two flame cone
   nodes are omitted from this file rather than shown wrongly.
6. **Scene fog**: no fog field in format v2 (film.rs FrameSpec
   has one). Recorded as top-level `fog` (the s06 shape).
7. **Cloud tint**: billboard nodes have no tint field in the
   loader; the golden cloud tint #FFE3B0 is recorded as an
   additive `tint` on the three cloud sprite nodes. Note
   src/film.rs `add_clouds` renders clouds untinted [1, 1, 1] —
   the TS atmosphere tints, the native evaluator does not; the
   data follows the project source.
8. **Character env tint**: no tint field on the v2 character
   node; the golden envMatch tint is recorded as `envTint`.
9. **Calendar overlay**: format v2 has no overlay construct at
   all — hence the companion file s07-calendar.json and the
   additive top-level `overlays` reference; film.rs composites
   it CPU-side from plates.
10. **Caption strips**: chunk ids/timing/fit are data (above);
    the strip images are film assets (film/gen_assets.py), not
    scene-format textures.
11. **Audio cues**: the three 'tock' cues are recorded in
    `shot.cues` as data only; audio does not exist in the scene
    format.

## S05a — s05a.scene.json (JG noon, film t 21.384–24.56) — seat W07

### Timing / shot identity
- `shot.at = cut(4) = L(4) − 0.06 = 21.384`, `end = S05b.at =
  Wd('thought',4) − 0.12 = 24.56`, `duration = 3.176` — project
  `shots.ts` S05a/S05b headers with line starts and anchors from
  `film/onoda.json` (lines[4].start = 21.444, thought4 = 24.68,
  trick4 = 25.6). INVENTORY.md's 21.38–24.56 is the rounded form.
- `time = 22.974` (lt = 1.59, mid-shot): every static transform in
  the file is the state at that t, per the v2 loader law.
- Captions: chunks 26–29 (line 4) — the chunks that START inside
  the shot, each `{index, id, s, e, text, fit}` copied from
  `film/onoda.json` (ids are the strip names). Chunk 25
  ("the message.") tails in until 21.61 but starts in S04 and is
  not listed (same rule as S04's chunk 20); chunk 29
  ("He thought it", 24.49–24.97) starts here and plays out across
  the cut into S05b — captions are film-global, drawn by time.
- `shot.cues`: `cue(st + 0.5, 'whoosh', {gain: 0.5, dur: 1.6})`
  (at 21.884) — audio, data only.

### Set (JG camp) / fire OUT
- Nodes/textures are `scenes/ab1.scene.json`'s JG set verbatim,
  minus the flames (below); dropped as in S04: the S01 code-ref
  card and rifle, the blob shadow (no character in this shot),
  the caption billboard + texture.
- **Fire out**: S05a calls `fireLevel(0, t)` (`shots.ts`): the
  flame cones are hidden, so the two flame cone nodes are
  OMITTED from this file (not scaled to 0), and `lights` has NO
  `points` entry — the fire light's intensity is forced to 0.
  The cold fire pit (stones, logs) stays. Recorded as top-level
  `channels` with `level: 0` (fireLight.intensity /
  flame.visible).

### Atmosphere / lights / post
- `atmosphere: "noon"` — `noonJG()` with `sky: false`; the sky
  texture stays the JG default (#6E9CC0/#D8E8DA).
- `lights` follows the S01 seat's reading — the rig AFTER
  `applyAtmosphere('noon')`, identical to `src/film.rs` rig_noon:
  hemisphere #CFE6FF / #C48A5A ×1.25; key #FFF0D6 ×2.6 at
  [7,12,5] → [0,0,−2], shadowBox 17, far 80, bias −0.0006,
  normalBias 0.02; rim ×0.4; envIntensity 0.25. (S04 /
  character-v2 carry the pre-atmosphere hemisphere
  #DCECD2 / #46603A ×1.15 — declared divergence, see the S01
  entry; the integrator should reconcile to one choice.)
- `post` = G.jungle (`shots.ts`): bokeh aperture 0.0012 /
  maxblur 0.007, bloom 0.16 (threshold 0.85 / radius 0.6 are the
  format defaults), grade saturation 0.8 / warm 0.04 /
  vignette 0.24 / grain 0.012. `focus = 16.296` = |eye − target|
  with the crane evaluated at `time`; `grade.time = 22.974`.

### Camera (crane 46°)
- `camera.keys`: [21.384 → eye (2.4, 1.0, 4.2), target
  (−4.5, 6.4, −11)], [24.56 → eye (2.0, 1.2, 3.6), target
  (0.5, 6.0, −11)], fovY 46 (`shots.ts` S05a `crane(...)` call;
  port `src/film.rs` s05a). Static eye/target repeat the first
  key (§14.4 fallback law).
- Derived/approximate (same note as S04): the staging eases with
  `camK(lt, d)` = smootherstep with hold 0.1 (`core/motion.ts`),
  while the v2 key evaluator applies plain smootherstep between
  keys (spec §14.4). Endpoints are exact; mid-shot camera
  position differs by the hold term.

### Plane (PBR primitives)
- Group at (px, 9.6, −13), `rotation.y = π/2`; children from the
  `shots.ts` S05a plane build: fuselage cylinder (rTop 0.17 /
  rBottom 0.23 / h 2.7 / seg 10, rotX π/2); nose cone (r 0.17 /
  h 0.5 / seg 10) at z 1.55, rotX π/2; wing box
  (3.6, 0.05, 0.72) at (0, 0.02, 0.15); tailplane box
  (1.1, 0.05, 0.4) at (0, 0.05, −1.25); fin box
  (0.05, 0.55, 0.35) at (0, 0.28, −1.25); propeller group at
  z 1.86 with two crossed blade boxes (0.09, 1.05, 0.03).
- Material #67705F, roughness 0.6 (its metalness 0.25 is
  inexpressible — item 2). Blades #2E2E33 carry the
  `core/world.ts` `mat()` DEFAULT roughness 0.85 / metalness 0 —
  the file follows the source; `src/film.rs` instead folds the
  blades into the plane material (0.6 / 0.25). Declared
  divergence.
- Channels: the group's `translate` keys (clock lt) reproduce
  `px = lerp(−13, 11, seg(lt, 0.05, d − 0.05))` exactly
  (hold / linear / hold); the propeller's `euler.z` is the
  formula `35 * t` (absolute t). Static transforms are the state
  at `time` (px = −0.984395; prop z = 804.09 rad).
- Source quirk kept: the nose cone's `castShadow` is never set
  in `shots.ts` (false), while the `box()` / `cyl()` helpers set
  cast + receive on the other parts. The format has no per-node
  shadow flags — nothing to carry; noted so nobody "fixes" it.

### Leaflets (16)
- One mesh per leaflet i = 0..15. Source: `PlaneGeometry(0.34,
  0.45)`, material #F2EDE0 roughness 0.9 **DoubleSide**,
  castShadow true. The format has no plane kind and no
  double-side switch, so each is authored as the thin box
  (0.34, 0.45, 0.005) the film evaluator itself uses
  (`src/film.rs` s05a), with `"doubleSide": true` carried in
  the material. **INTEGRATOR FLAG: paper is DoubleSide in the
  source and the basic pipeline is back-face culled — a
  single-sided draw makes the falling sheets vanish from
  behind. The leaflets (and S05b's dropped leaflet) need a
  double-sided draw path or two draws.**
- Channels per leaflet (clock lt): `translate` keys sampled
  every 0.1 s after release from the closed form
  (delay = 0.4 + 0.13i; ft = seg(lt, delay, delay + 3.1);
  x = sx + sin(4ft + 1.7i)·0.9 + 1.2ft with
  sx = lerp(−13, 11, seg(delay + 0.3, 0.05, d − 0.05));
  y = 9.4 − ft^1.12 · 8.9; z = −13 + (rnd(3i) − 0.5)·3 + 3.6ft)
  — worst linear-interp deviation vs the closed form ≈ 8 mm
  (the ft^1.12 y-curve right at release). `euler` keys are
  exact (rotation is linear in absolute t:
  (1.7t + i, 1.3i, 1.1t + 2i)). `visible` window
  [delay, 3.176] from `mesh.visible = ft > 0 && ft < 1.12`; the
  1.12 end is unreachable in-shot (max ft = 0.897) — no leaflet
  lands inside S05a.
- At `time` (lt 1.59) leaflets 0–9 are mid-fall; 10–15 are not
  yet released — their static transforms sit at the ft = 0
  spawn state and only the visible windows keep them off
  screen. A consumer that ignores channels/windows will show
  six un-dropped sheets hovering at y 9.4.

### Inexpressible in format v2 (declared, not substituted)
1. **DoubleSide paper** (above): no format field; carried as a
   material flag + this flag for the integrator.
2. **Metalness**: the format material carries colour +
   roughness only; the plane's metalness 0.25 cannot be
   expressed (film.rs's PBR path uses its own material table).
3. **Channel evaluation**: the loader renders the statics at
   `time`; the channels exist as data but no loader code path
   evaluates them over the shot's duration (same status as
   S04's channels).
4. **Per-node shadow flags** (nose cone, above).
5. **The whoosh cue**: audio; recorded in `shot.cues` as data
   only.

## S05b — s05b.scene.json (JG noon, film t 24.56–26.129) — seat W07

### Timing / shot identity
- `shot.at = Wd('thought',4) − 0.12 = 24.56`, `end = cut(5) =
  L(5) − 0.06 = 26.129`, `duration = 1.569` (anchors from
  `film/onoda.json`: thought4 = 24.68, trick4 = 25.6,
  lines[5].start = 26.189). INVENTORY.md's 24.56–26.13 is the
  rounded form.
- `time = 25.31` (lt = 0.75): the character `state`, flame
  scales and fire intensity below are the values at that t.
- Captions: chunks 30–31 (line 4) — the chunks that start
  inside the shot. Chunk 29 starts in S05a and plays out here
  until 24.97; chunk 31 ("trick.", 25.57–26.19) runs past the
  out-point — a mid-chunk cut, as captions are time-driven.
- No cues in this shot (`shots.ts` has none for S05b).

### Set / atmosphere / lights / post
- `scenes/ab1.scene.json`'s JG set verbatim (flames KEPT);
  dropped as in S04: code-ref card/rifle, blob (the character
  node emits its own), caption billboard + texture. Textures
  add `leaflet` (file `../../film/assets/leaflet.png`).
- `atmosphere: "noon"`; `lights` as S05a (rig_noon reading —
  see its divergence note), WITH the fire point: #FF9A3C at
  [−1.35, 0.6, −2.45], distance 7, decay 1.6, static intensity
  = the fireLevel formula at `time` (4.968072), the flicker
  formula carried as the point's `channel` field (S01 dialect;
  the constructor base is 5.0).
- Fire lit (`fireLevel(1, t)`): flame scaleY is baked at
  `time` (f = 1.108893; flame2 = 2 − f = 0.891107) and the
  flicker pair is recorded as top-level `channels`
  (flame.scaleY / flame2.scaleY, S04 dialect).
- `post` = G.jungle as S05a; `focus = 2.868` (push-in evaluated
  at `time`); `grade.time = 25.31`.

### Camera (push-in 42°)
- `camera.keys`: [24.56 → eye (0.95, 1.75, 2.6)],
  [26.129 → eye (0.7, 1.6, 1.8)], target (0.3, 1.3, −0.55)
  held, fovY 42 (`shots.ts` S05b shoot call; port
  `src/film.rs` s05b). camK-vs-plain-smootherstep note as
  S05a / S04.

### Onoda (character node)
- `position` (0.35, 0, −0.55); `look` body #5C6644 / shade
  #878E6E / cap #4C5638 (cast registry; `person('onoda',
  {perspective: true}, '#878E6E')`); `perspective: true`.
- `state` at `time`: mood `suspicious`; armL [58, −24] (the
  poseTrack has not started moving at lt 0.75 — its move runs
  [1.14, 1.59]); armR [16, −6]; look [0.1, 0.3]; blink 0
  (`blinkDirector(3)` evaluated at t = 25.31); headTurn 0.04
  (the pre-trick constant — the settle channel below has not
  started). S05b never calls `camPitchOf`, so no `camPitch`
  (unlike S01).
- `state.holdL` = plate `leaflet`, x0 −200, y0 −258 — from
  `film/gen_assets.py` `leaflet_plate()` (the canvas
  `leafletProp`: paper 124×164 at translate(0, −58),
  rotate(−0.1), 'WAR IS' / 'OVER' + 4 grey bars, all baked
  into the 800×800 plate; anchor law per `toonfull::Plate`).
- Channels: `armL` as a poseTrack (keys [[0, 58],
  [1.59, 16, 0.45]], clock lt — S04 dialect shape); `headTurn`
  as the closed-form formula `t > 25.6 ? 0.38 * settle(t,
  25.6, 6.5, 3.2) : 0.04` (settle in `core/motion.ts`; the
  shake starts at lt 1.04, inside the shot); `blink` as
  director seed 3.

### The drop that never plays inside the shot (timing finding)
- `dropT = Wd('trick',4) + 0.55 = 26.15` is 0.021 s AFTER the
  out-point (26.129): within S05b the arm move completes only
  at lt 1.59 (past the cut) and `dropLeaf.visible`
  (t ≥ dropT) never turns true — in the shipped staging Onoda
  still holds the leaflet when S06 cuts in, and the fall
  (u over [26.15, 27.0]) belongs to no shot. `src/film.rs`
  agrees (drop_t hard-coded 26.150 vs its S05b end 26.129).
  The dropped-leaflet node is authored anyway, faithfully:
  thin box (0.24, 0.32, 0.005), #F2EDE0 roughness 0.9,
  doubleSide (source: `PlaneGeometry` DoubleSide, castShadow
  NOT set); static transform = the u = 0 pose
  (−0.25, 1.05, −0.3), euler (0, 0.4, 0); translate/euler
  channels keyframed over lt [1.59, 2.44] from
  u = seg(t, dropT, dropT + 0.85) (pos (−0.25 + 0.14u,
  lerp(1.05, 0.03, u²), −0.3 + 0.1u); rotation
  (2.4u, 0.4, 1.8u)); visible window [1.59, null]. Do not
  "fix" the timing — it is the staged truth.

### Attachments / inexpressible in format v2
1. **Rifle as a scene node** — declared in the top-level
   `attachments` field exactly as S04 (kind rifle, attachedTo
   onoda, hand R, offset [0, 0.03, 0.07]); the film evaluator
   draws it. `makeRifle()` model (`shots.ts`): stock box
   (0.075, 0.52, 0.13) #6E4F30 at y −0.17; receiver box
   (0.06, 0.1, 0.09) #3A3D42 (metalness 0.55 — also beyond the
   format's material); barrel cylinder r 0.016 h 0.55 at
   y 0.42; front sight box (0.02, 0.05, 0.02) at y 0.68; grip
   box (0.05, 0.05, 0.16) wood at (0, −0.02, 0.05), rotX 0.5.
2. **Channel evaluation** and **fire flicker as lights fields**
   — as S04 items 2–3.
3. **DoubleSide paper** — as S05a item 1 (both the held
   leaflet plate is a card texture, unaffected; the DROPPED
   leaflet mesh is the affected one here).

## S03 — s03.scene.json (SH noon, film t 13.061–15.950) — seat W05

### Timing / shot identity
- Segment 3 of 14; window = cut(2)…cut(3) = (L(2) − 0.06)…(L(3) − 0.06)
  = 13.061…15.950 s, duration 2.889 s (film/onoda.json: lines[2].start
  13.121, lines[3].start 16.01; src/film.rs s03 uses st0 = 13.061).
- Representative `time` = lt 2.0 (film t 15.061): after Taniguchi's
  pointing gesture lands (poseTrack ends lt 1.589), inside the talk
  window (lt < 2.309), during the word "surrender" (anchor 15.02).
  All four blink channels evaluate to 0 at this instant (eyes open).
- Captions: line 2's three chunks recorded as top-level data with
  film times — cap_018 "His orders" 13.15–13.61, cap_019
  "were simple:" 13.61–14.75, cap_020 "never surrender." 14.75–16.01.
  cap_020 ends 0.06 s past the shot end because caption chunks follow
  the narration track, not the cut. The strip images are film assets
  (film/gen_assets.py), not scene-format textures.

### Set (SHORE — authored inline from source)
- scenes/film/s02.scene.json does not exist (checked at authoring
  time), so the SH set is authored inline from the SH builder
  (shots.ts:198–224) + world.ts, not copied from a sibling file.
- Sky: sky-gradient top #6E9CC0 / horizon #E4EEDC (WD.sky). Sand:
  colour disc r 30, 40 segments, #DEC795, roughness 1 (CircleGeometry
  rotated flat; the format disc is XZ-flat by construction). Sea and
  foam: see inexpressible #4. Hills: WD.hills {z: 55, col #5E8F52} —
  unit spheres scaled (r, h, r·0.6) at y = −h·0.25 with shadeHash
  colour (0.92 + rnd(i)·0.16), plus 6 round trees per hill whose
  positions (a = rnd(i·9+k)·2−1) and total scales
  ((0.5 + rnd(k)·0.4)·2.2) were evaluated in f64 and f32-rounded.
- Shore trees: the 7-entry list (shots.ts:210), kinds alternate
  round/cypress starting round, colour #3E7A34, position only — no
  rotation.y (see inexpressible #8). Palms bp1 (1.0, 0.18) @
  (−6.2, 0, 4.5) and bp2 (0.92, −0.14) @ (6.8, 0, 5.5) from the local
  palm() (shots.ts:85–100); group transform [translate, rotate_z
  (lean), scale]. Boat, crates, barrel per shots.ts:216–224 +
  world.ts crate()/barrel().

### Atmosphere / lights / post
- applyAtmosphere(scene, 'noon', {rig: SH.sun, sky: false}). The
  recorded rig is the POST-atmosphere rig: env.ts noon overwrites
  key colour/intensity (#FFF0D6 ×2.6) and — the hemisphere lives
  inside rig.group — hemi sky/ground/intensity (#CFE6FF / #C48A5A
  ×1.25); key position [−8, 11, 7], target [0, 0, −4], shadowBox
  17, far 80, bias −0.0006, normalBias 0.02 stay the SH sunlight()
  call's; rim #FFFFFF ×0.4 @ [−3, 3, −3]; envIntensity 0.25. This
  matches src/film.rs rig_noon([-8, 11, 7], [0, 0, -4], off).
- Post = G.shore (shots.ts grade registry): bokeh aperture 0.0012 /
  maxblur 0.007 with focus = the evaluated |eye − target| at `time`
  (6.768082141876221); bloom strength 0.16; grade sat 0.84, warm
  0.05, vignette 0.22, grain 0.012, time = film t 15.061.

### Camera (push-in 54°)
- shoot(from [0.2, 2.0, 8.4] → to [0.05, 1.85, 7.2], target
  [0, 1.22, 0.6], fov 54), k = camK(lt, d) with hold 0.1
  (core/motion.ts:35). The v2 key evaluator has no hold parameter,
  so the hold is encoded structurally: keys at t = 0 and t = 0.2889
  (= 0.1·d) hold the from-pose, keys at t = 2.6001 (= 0.9·d) and
  t = 2.889 hold the to-pose; the middle segment's smootherstep
  then reproduces camK exactly (k = 0.886047… at `time`). Verified
  by re-evaluating the keys under loader semantics: eye
  [0.0671, 1.8671, 7.3367], identical to the camK mix.

### Characters
- Taniguchi at (−1.05, 0, 1.0) — FULL accessory set, all of it
  first-class v2 character-node fields, nothing accessory-wise is
  inexpressible: body #4A4560, pattern "vest" + patCol #33304A,
  hat "bowler" + hatCol #2E2A3A, neck "tie" + neckCol #7A2E3A,
  extra ["glasses", "mustache"] + mustacheCol #C9C9D2. State =
  {...POSES.explain (armL [28, −24], look [0.3, 0]) with mood
  overridden to "determined", armR from the poseTrack channel,
  talk/blink evaluated at `time`}.
- Onoda at (1.0, 0, 0.75), compA at (1.85, 0, −0.85), compB at
  (−1.9, 0, −0.65) — at attention: armL/armR [5, 3]; Onoda mood
  "determined", companions "neutral". Looks per the shots.ts
  registry (compB carries the scarf: neck "scarf" #8A7A52). All
  four perspective: false (only the JG Onoda instances pass
  perspective: true), no shade override (the JG fade #878E6E does
  not apply to the SH instances), and envMatch is a no-op at noon
  (identity tint), so nothing is recorded for it.
- The shore rifle is NOT staged in S03 (shStage is called without
  the rifle flag), so no rifle node or attachment is recorded —
  see inexpressible #7 for the film.rs divergence.

### Keyframed channels (additive `channels` fields)
The v2 loader reads a character's static `state` at `time` only
(spec §14.3); time-varying formulas are recorded in `channels`
objects (ignored by the loader, implemented by the film evaluator
in src/film.rs s03):
- Taniguchi `armR`: poseTrack keys [[0, 68], [1.589, 96, 0.3]] on
  the shot-local clock (key time = wNever − st − 0.13 = 14.78 −
  13.061 − 0.13; the pair's second component is a static −6).
  State at `time` carries the landed value 96.
- Taniguchi `talk`: talk(t, on) = on ? 0.25 + 0.75·|sin(13t)|·
  (0.6 + 0.4·sin(5.3t)) : 0 on the FILM clock, on while
  lt < wSurr − st + 0.35 = 2.309. Evaluated at `time`:
  0.38780462741851807.
- Blinks: blinkDirector seeds — Taniguchi 9, Onoda 3, compA 7,
  compB 11; Onoda's is max(blinkDirector(3)(t), blinkAt(t,
  [wNever = 14.78])). All evaluate to 0 at `time`.

### Inexpressible in format v2 (declared, not substituted)
1. **Cloud sprite tint** #EFF4EA (WD.clouds call): the billboard
   texture reference has no tint field; clouds are recorded
   untinted.
2. **Hill grass map**: hills are mat({color: shade(col, …), map:
   T.grass(col, [6, 3])}); a v2 sphere mesh carries material XOR
   texture, so hills are shadeHash colour only (the
   ab1.scene.json convention).
3. **Planks textures** (T.planks) on both crates and the barrel:
   no planks generator exists in the texture registry. Crates are
   flat #B07A48; the barrel is flat #6E4525 — the lead's
   planks-averaged shade (ab1.scene.json, src/film.rs build_sh);
   the three base colour under the map is #9A6337.
4. **Sea + foam planes**: PlaneGeometry has no flat colour-plane
   kind; both are expressed as 0.02-thick boxes (src/film.rs
   build_sh convention). The sea's metalness 0.08 has no material
   field and is dropped; the barrel bands' metalness 0.4 likewise.
5. **Icosahedron canopy blobs** → spheres (12×9 segments): the
   established §28 substitution. The cypress canopy is a true
   sphere in three (16×12) and is recorded as one.
6. **Per-node shadow flags**: three sets castShadow = false on the
   hills and all hill trees, receive-only on the sand, and none on
   sea/foam; the format carries no shadow flags on set nodes, so
   loader defaults apply.
7. **Rifle visibility divergence (flagged for the lead)**:
   shots.ts S03 leaves Onoda's rifle hidden (shStage without the
   rifle flag), but src/film.rs s03 calls char(..., SH_ONODA,
   ..., rifle = true), which draws it attached to hand R. This
   file follows shots.ts.
8. **Shore-tree rotation divergence (flagged)**: src/film.rs
   build_sh rotates the shore trees by i·1.7 — that is the JG
   ring's rotation (shots.ts:160); the SH loop (shots.ts:210)
   sets no rotation. This file follows shots.ts.
9. **Palm frond/tip colours divergence (flagged)**: shots.ts
   palm() gives frond i odd → leaf #3F7D33, even → leaf2
   #4C8A3A, with the tips on the opposite alternation and tip
   geometry SphereGeometry(1, 8, 5). ab1.scene.json and
   src/film.rs add_palm use even → leaf, tip = frond colour, and
   10×6 tips. This file follows shots.ts.
10. **Hemisphere cross-file inconsistency (flagged)**:
    character-v2.scene.json recorded the JG rig's hemisphere at
    its sunlight()-call values; the applyAtmosphere traversal
    does reach the rig-group hemisphere, so the noon values are
    what three renders (and what src/film.rs rig_noon uses).
    This file records the noon values.
11. **Engine constants omitted**: bloom threshold/radius and the
    grade exposure are engine constants, not per-shot data
    (loader defaults 0.85 / 0.6 / 1.0). The key shadow near plane
    (0.1) has no v2 lights field.
12. **Sky ground stop**: WD.sky's third gradient stop (#BFE4F7
    default) is a fixed constant of the format's sky-gradient
    generator (spec §6); only top/horizon are recorded.
13. **Channel evaluation**: the loader renders the static state
    at `time`; the formulas above exist as data but no loader
    code path evaluates them over the shot's duration (same
    declaration as the sibling files).

## RECONCILIATION — seat W19 (Batch 1 files vs §14.5 rulings R1/R2)

Applied 2026-11-14. Rulings: docs/scene-format.md §14.5 — **R1**
(`lights` blocks carry POST-atmosphere values; noon hemisphere
#CFE6FF / #C48A5A ×1.25 and fire basis 4.6, never the
constructor / sunlight()-call values) and **R2** (`attachments`
lives ON the character node; no top-level `attachments` block).
Every file below was read first-hand before and after the edit;
all eight film files re-parse as valid JSON after the edits.
The moved rifle entries were normalised to the on-node shape
s01/s06 already use — `{"kind": "rifle", "hand": "R", "offset":
[0.0, 0.03, 0.07]}`: the old `attachedTo` field is dropped
(placement on the node names the carrier) and the `source`
citation is dropped from the JSON (it survives in each shot's
own section above); kind/hand/offset values are unchanged.

### Changed: s04.scene.json (R1 + R2)
- `lights.hemisphere.sky`: #DCECD2 → **#CFE6FF**;
  `lights.hemisphere.ground`: #46603A → **#C48A5A**;
  `lights.hemisphere.intensity`: 1.15 → **1.25**. The old values
  were the JG `sunlight()` call's creation values
  (`shots.ts:145`), copied from the lead's character-v2 example;
  the new values are the env.ts `noon` preset that
  `applyAtmosphere` leaves in force (project
  `core/env.ts:76–84`: hemiSky #CFE6FF / hemiGround #C48A5A /
  hemiI 1.25) — §14.5 R1.
- `lights.points[0].intensity`: 5.0 → **4.6**, and the point
  gained the `channel` field s01 carries —
  `{kind: "fireLevel", level: 1, formula: "(4.6 + 0.7*sin(17t)
  + 0.5*sin(7.3t)) * level"}` — so the point now mirrors
  s01.scene.json's fire light exactly. The old 5.0 was the
  PointLight constructor argument (`shots.ts:179`), which
  `fireLevel` overwrites on the first frame; the rendered
  basis is 4.6 (`shots.ts:287–292`) — §14.5 R1. The pre-existing
  top-level `channels.fireLight.intensity` entry is retained
  (it is also the record for the flame scaleY pair); its
  formula already used the 4.6 basis and now agrees with the
  point. Colour #FF9A3C, position [−1.35, 0.6, −2.45],
  distance 7.0, decay 1.6 unchanged (`shots.ts:179`).
- `lights.key` (#FFF0D6 ×2.6) and `lights.rim` (#FFFFFF ×0.4)
  were already the post-atmosphere values (`core/env.ts:76–84`
  keyCol/keyI/rimI) — unchanged.
- R2: the top-level `attachments` block was **deleted** and the
  rifle entry added to the `onoda` character node as its
  `attachments` field (placed after `state`, the s01
  placement). Attach call: `shots.ts` jgOnoda.attach
  (rifleHeld, 'R', offset [0, 0.03, 0.07]); model `makeRifle()`
  `shots.ts:74–83`.
- Note: the S04 section at the top of this file describes the
  pre-reconciliation state (lights "identical to
  character-v2.scene.json", point "constructor base 5.0",
  "top-level `attachments` field"). It is the authoring
  history; this section supersedes it.

### Changed: s05b.scene.json (R2 only)
- R2: the top-level `attachments` block was **deleted** and the
  rifle entry added to the `onoda` character node as its
  `attachments` field (after `state`, the s01 placement).
  Attach: `shots.ts` jgStage({rifle: true}), offset
  [0, 0.03, 0.07] — values unchanged.
- Lights **verified, not changed**: the block already follows
  film.rs `rig_noon` / the post-atmosphere noon rig — key
  #FFF0D6 ×2.6 at [7, 12, 5] → [0, 0, −2], rim #FFFFFF ×0.4,
  hemisphere #CFE6FF / #C48A5A ×1.25, envIntensity 0.25
  (`core/env.ts:76–84` via applyAtmosphere). The fire point's
  static `intensity` 4.968072 is the fireLevel formula
  evaluated at the file's `time` (25.31) with the formula
  carried on the point as its `channel` (the s01 dialect);
  its basis is 4.6 per R1 — left as authored.

### Changed: s02.scene.json (R2 only)
- R2: the top-level `attachments` block was **deleted** and the
  rifle entry added to the `onoda` character node (nodes[49],
  id "onoda") as its `attachments` field (after `state`).
  compA / compB gained nothing. Attach call `shots.ts:255`
  (shOnoda.attach(rifleShore, 'R', {offset: [0, 0.03, 0.07]}));
  model `makeRifle()` `shots.ts:74–83` — values unchanged.
- Lights **verified, not changed** (W04's claim confirmed by
  reading): already post-atmosphere — key #FFF0D6 ×2.6 at the
  SH position [−8, 11, 7] → [0, 0, −4], rim #FFFFFF ×0.4,
  hemisphere #CFE6FF / #C48A5A ×1.25, envIntensity 0.25; no
  `points` (the shore has no fire light). The SH sunlight
  call's creation hemisphere (#DCECD2 / #8A7A52 ×1.15,
  `shots.ts:200`) is NOT what the file carries.

### Verified compliant — no changes made
- **s01.scene.json** — R1 ✓: hemisphere #CFE6FF / #C48A5A
  ×1.25; fire point intensity 4.6 with the `fireLevel` channel
  on the point; key/rim post-atmosphere. R2 ✓: rifle
  `attachments` on the Onoda node; no top-level block. (This
  file is the shape the others were reconciled to.)
- **s03.scene.json** — R1 ✓: post-atmosphere noon rig (key
  #FFF0D6 ×2.6 at [−8, 11, 7] → [0, 0, −4], rim ×0.4,
  hemisphere #CFE6FF / #C48A5A ×1.25, `points: []`). R2 ✓
  vacuously: no `attachments` anywhere — the rifle is
  deliberately not staged in S03 (`shots.ts` shStage without
  the rifle flag; see the S03 section, inexpressible #7).
- **s06.scene.json** — R1 ✓ for its atmosphere: the lights are
  the post-atmosphere FOG rig applyAtmosphere leaves (key
  #EAF0F2 ×0.7, rim #FFFFFF ×0.15, hemisphere #E6E9EA /
  #B8BCBE ×0.9 — env.ts ATMO.fog over the noon rig geometry),
  not any constructor state. The fire point's intensity
  0.6314269304275513 is the evaluated fireLevel track at the
  still (level 0.1333… × the 4.6-basis formula, carried in the
  file's `fire` block), i.e. the rendered value, not the
  constructor's 5.0. R2 ✓: rifle `attachments` on the onoda
  node; no top-level block.
- **s07.scene.json** — R1 ✓ for its atmosphere: post-atmosphere
  GOLDEN rig (key #FFB35C ×2.3, rim #FFFFFF ×0.6, hemisphere
  #FFE0B0 / #B07A48 ×1.0 — `core/env.ts:86–92` golden preset);
  fire point carries the 4.6 basis with a `fireLevel` channel
  at level 0. R2 ✓: rifle `attachments` on the Onoda node; no
  top-level block.
- (Also observed, outside the tasked list: s05a.scene.json
  already complies — post-atmosphere noon lights, no `points`
  with the fire out, no characters and no attachments.)

### Note for the lead — scenes/character-v2.scene.json (NOT edited)
The lead's example file still shows PRE-atmosphere values in
its `lights` block, against §14.5 R1: hemisphere sky #DCECD2 /
ground #46603A ×1.15 (the JG `sunlight()` creation values,
`shots.ts:145`) where the rendered noon rig is #CFE6FF /
#C48A5A ×1.25, and fire point `intensity` 5.0 (the constructor
argument, `shots.ts:179`) where R1 fixes the basis at 4.6.
Its key (#FFF0D6 ×2.6) and rim (#FFFFFF ×0.4) already match
the post-atmosphere rig. Secondary observation: its character
node carries no `attachments` field. Left untouched per this
seat's brief — the lead owns the file.

## S08 — s08.scene.json (JG noon, film t 34.209–40.36) — seat W11

### Timing / shot identity
- `shot.at = cut(7) = L(7) − 0.06 = 34.209` (stored at full f64
  precision, 34.208999999999996), `end = Wd('refused', 7) − 0.1
  = 40.36`, `duration = 6.151` — project `shots.ts:441–442`
  (S08 header: `st = cut(7)`, `endAt = Wd('refused', 7) − 0.1`;
  the S08 draw itself never reads endAt — `void endAt` — the
  boundary is S08b's own start, `shots.ts:458`). Line starts /
  anchors from `film/onoda.json` (lines[7].start = 34.269,
  found7 = 38.48, refused7 = 40.46). INVENTORY.md's S08 row
  (34.21–40.36) is the rounded form.
- `time = 37.209` (shot-local lt = 3.0) is the file's evaluation
  instant, chosen so both walk drivers are in informative
  states: Suzuki mid-walk (wkS = 0.90625), Onoda arrived
  (wkO = 1). Every character `state`, the flame scales, the
  fire point intensity, the bokeh focus and Onoda's camPitch
  are the values at that t, per the v2 loader law.
- Captions: chunk objects 42–46 (line 7) with index/s/e/text/
  fit copied from `film/onoda.json` captions[42..46] — the
  chunks that START inside the shot. Chunk 41 ("and bananas.",
  line 6, e = 34.27) tails 0.06 s into the shot head and is not
  listed (the S04 chunk-20 rule); chunk 46 ("Onoda still
  refused", 39.95–40.83) starts here and plays out across the
  cut into S08b — captions are film-global, drawn by time.

### Set (JG camp)
- Nodes/textures are `scenes/ab1.scene.json`'s JG set verbatim
  (80 of its 84 nodes; sky/grass/cloud textures) — the same set
  S08 uses (`shots.ts` JG builder; S08 calls `noonJG()` +
  `jgStage({onoda, suzuki, rifle})`, `shots.ts:444`).
- Dropped from the ab1 copy: the S01-staged code-ref Onoda
  card, the rifle code-ref, the blob shadow and the 'fighting'
  caption billboard + the blob/onoda-card/caption textures
  (replaced by the v2 character nodes / declared attachment /
  chunk data, as in S04/S05b).
- Fire lit (level 1): the flame cones' static scaleY is the
  fireLevel flicker at `time` — flame 0.94370675, flame inner
  1.05629325 (f = 1 + 0.16·sin(13t) + 0.07·sin(29t + 1) at
  t = 37.209 = 0.94370677; the second cone is the file's
  `flame2` channel target, node name "flame inner").

### Atmosphere / lights / post
- `atmosphere: "noon"` — `noonJG()` = applyAtmosphere(JG.scene,
  'noon', { rig: JG.sun, sky: false }) (`shots.ts:444, 293`).
- `lights` is the POST-atmosphere noon rig per **R1**
  (docs/scene-format.md §14.5): key #FFF0D6 ×2.6 at [7,12,5] →
  [0,0,−2], shadowBox 17, far 80, bias −0.0006, normalBias
  0.02; rim #FFFFFF ×0.4 at [−3,3,−3]; hemisphere #CFE6FF /
  #C48A5A ×1.25; envIntensity 0.25 — identical to S01/S05b and
  to src/film.rs `rig_noon`. Fire point #FF9A3C at
  [−1.35, 0.6, −2.45], distance 7, decay 1.6; its static
  `intensity = 4.4747124` is the fireLevel formula evaluated at
  `time` (the rendered value at the still); the formula rides
  in the point's `channel` with the R1 basis 4.6.
- `post` = G.jungle verbatim (the draw passes `{...G.jungle,
  fov: 62}`, `shots.ts:454` — no gradeOf overrides): bokeh
  aperture 0.0012 / maxblur 0.007, bloom 0.16 (threshold 0.85 /
  radius 0.6 are the engine's fixed pass values), grade
  saturation 0.8 / warm 0.04 / vignette 0.24 / grain 0.012.
  Bokeh `focus = 6.9042368` is DERIVED: shoot()'s default
  focus = |eye − target| (`shots.ts:127–131`) with the camera
  evaluated at `time` (k = camK(3.0, 6.151, 0.1) = 0.47124987,
  eye (0.90575, 1.90575, 5.6345), target (0.35, 1.1, −1.2)).
  `grade.time = 0.0` per **R3** (grain time is 0 for the whole
  film) — this departs from S05b's file-time value on purpose;
  R3 is the later, binding ruling.

### Camera (push-in 62°)
- `camera.keys`: film-time keys [34.209 → eye (1.0, 2.0, 6.2)],
  [40.36 → eye (0.8, 1.8, 5.0)], target (0.35, 1.1, −1.2) held,
  fovY 62 (`shots.ts:454` shoot call; port src/film.rs:1710).
  Static eye/target repeat the first key (the §14.4 fallback
  law).
- Derived/approximate: the staging eases with `camK(lt, d)`
  (hold 0.1, `core/motion.ts:41`), recorded as
  `shot.cameraTrack {kind: "camK", hold: 0.1}`, while the v2
  key evaluator applies plain smootherstep between keys (spec
  §14.4). Endpoints are exact; mid-shot camera position
  differs by the hold term (same declared status as S01/S04).

### Characters
- Looks from the cast registry (`shots.ts:27–33`): Onoda body
  #5C6644 / cap #4C5638 with the worn shade #878E6E
  (`shots.ts:239`); Suzuki body #C96A42 / cap #E8B83A / extra
  ['glasses'] (`shots.ts:31`). Onoda is the JG perspective
  instance (`perspective: true`, `shots.ts:239`); Suzuki is
  `person('suzuki')` with no opts (`shots.ts:243`), so his node
  carries NO `perspective` field (the S02 convention).
  person() sets envMatch: true for both, but the noon tint is
  the identity (src/film.rs TINT_NOON) — nothing to carry.
- **Suzuki** `state` at `time`: position (1.7421875, 0,
  −1.025) = lerp of (3.6, 0, −3.2) → (1.55, 0, −0.8) at
  wkS = seg(3.0, 0.1, 3.3) = 0.90625; legs `walk`, phase
  15.563973 = walkPhase(0.90625 × 3.28, 0.6), bob 4.282537 =
  walkBob(phase, 5); mood `neutral` (t = 37.209 < wFound
  38.48); armL [38, −52], armR [12, 8]; blink 0 (director
  seed 5 evaluated at t; the blinkAt beat at 38.48 is
  unreached); `holdL` = the MAP plate (below).
- **Onoda** `state` at `time`: wkO = seg(3.0, 0.35, 1.95) = 1,
  so he stands at his walk endpoint (−1.1, 0, −1.5); legs
  `stand`, phase 11.780972 = walkPhase(2.25, 0.6) (= 3.75π,
  the full travelled distance), bob 1.171573 =
  walkBob(phase, 4); mood `determined`, armR [16, −6],
  armL [10, 6]; **blink 1** — blinkDirector(3) evaluated
  honestly at t: x − acc = 0.119 s, inside the 0.12 s pulse
  window by ~0.001 s (declared borderline: the director runs
  on f64 film t in the source, so 1 is the source's own
  value); camPitch 0.127733 is DERIVED: camPitchOf at `time`
  (atan2(eye.y − 1.75·0.6, hypot(eye.x + 1.1, eye.z + 1.5)) /
  0.9, `core/toon.ts:720`).
- **The MAP**: Suzuki's `holdL` plate is `paper-map`
  (film/assets/paper-map.png), origin x0 −200 / y0 −270 from
  film/assets/plates.json. Source drawing is `PROPS.paper(
  'MAP')` (`core/toon.ts:481`): translate(x, y − 70),
  rotate(−0.08); sheet rect(−70, −90, 140, 180) fill #F2EDE0,
  ink stroke lw 5; 'MAP' in 900 30px Inter centred at
  (0, −50); six grey #9B958A rule rects (−56, −26 + i·18,
  i == 2 ? 60 : 112, 7) — transcribed in the texture entry's
  `sourceDrawing`; plate pre-rendered by film/gen_assets.py
  `paper_plate` (`v: 'map'` in the staging is the painter's
  prop-version key — see Inexpressible 3).

### Keyframed channels (additive `channels` fields)
The v2 loader reads a character's static `state` at `time`
only (spec §14.3); time-varying formulas are recorded in
`channels` objects (ignored by the loader, implemented by the
film evaluator in src/film.rs s08, lines 1705–1742):
- Suzuki `position`/`phase`/`bob`/`legs`: the seg(lt, 0.1,
  3.3) walk driver above (travelled = wkS × 3.28, stride 0.6).
- Suzuki `mood`: stepAt film t = 38.48 — neutral → `shock` on
  the word 'found' (`shots.ts:448`).
- Suzuki `blink`: maxOf blinkDirector(5) and blinkAt at
  [38.48] (`shots.ts:448`).
- Onoda `position`/`phase`/`bob`/`legs`: the seg(lt, 0.35,
  1.95) walk driver (travelled = wkO × 2.25, stride 0.6,
  bob amp 4).
- Onoda `blink`: blinkDirector seed 3. Onoda `camPitch`:
  camPitchOf per frame (the S08 staging never sets it in
  set(); film.rs computes it centrally in finish_chars).
- Fire (top-level `channels` + the light's `channel`):
  flame/flame-inner scaleY flicker pair and the point
  intensity track, level 1 (`shots.ts` fireLevel(1, t)).

### film.rs divergence check
- src/film.rs `s08` (lines 1705–1742) was compared field by
  field against `shots.ts:443–456`: post-atmosphere rig, fire
  level 1, camera endpoints/target/fov 62 with cam_k hold 0.1,
  both walk drivers (segments, distances 3.28 / 2.25, strides
  0.6, bob amps 5 / 4), Suzuki arms [38, −52] / [12, 8] and
  the shock step at w_found, blink seeds 5 (max blinkAt at
  w_found) and 3, Suzuki `hold_l = "paper-map"`, and the rifle
  carried by Onoda's char() call (its `rifle` argument, true
  for Onoda, false for Suzuki). **No divergence found** — this
  file and the evaluator agree with the source throughout.

### Inexpressible in format v2 (declared, not substituted)
1. **Channel evaluation** (above): the loader renders the
   state at `time`; the formulas exist as data but no loader
   code path evaluates them over the shot's duration.
2. **camK hold** in camera keys (see Camera note).
3. **The MAP's `v: 'map'` prop-version key** — a cache/version
   tag the source painter attaches to the held prop
   (`shots.ts:448`); plates are static files in the format, so
   there is nothing for it to key. Recorded here only.
4. **Rifle geometry**: the character node carries the
   attachment (kind/hand/offset) per **R2**; makeRifle()'s
   five draws are the evaluator's model (geometry recorded in
   the S02 entry). Note this file follows R2's on-node
   placement — the older top-level `attachments` blocks in
   S02/S04/S05b predate the ruling.
5. **Caption strips**: chunk ids/timing/fit are data (above);
   the strip images are film assets (film/gen_assets.py), not
   scene-format textures.

## S09 — s09.scene.json (JG noon, film t 41.732–45.365) — seat W13

### Timing / shot identity
- `shot.at = cut(8) = L(8) − 0.06 = 41.732`, `end = cut(9) =
  45.365`, `duration = 3.633` — cut rule `shots.ts` (S09 block,
  the `cut` helper at buildShots head) with line starts from
  `film/onoda.json` lines[8].start = 41.792, lines[9].start =
  45.425. Matches INVENTORY.md S09 row (41.73–45.37). No audio
  cues are authored in this shot.
- `time = 43.92` is the file's evaluation instant: lt = 2.188,
  the exact instant the chest-tap poseTrack lands on its
  'commander' key (wComm − 0.12). Every character `state` in
  the file is the state at that t (v2 loader law, spec §14.1).
- Captions: chunk objects cap_048–cap_051 (line 8) with
  s/e/text/width/fit copied verbatim from `film/onoda.json`
  captions[48..51]. Chunk 47 ("to leave.", line 7,
  e = 41.762) is still on screen for the first 0.03 s of the
  shot; it belongs to line 7 and is not listed (the S04
  convention).

### Set (JG camp)
- Nodes/textures are `scenes/ab1.scene.json`'s JG set verbatim,
  taken via s04.scene.json's already-cleaned copy (sky, grass
  ground, dirt, 3 hills + trees, 19-tree ring, 3 palms, 900-tuft
  scatter, camp: fire ring/logs/flames/seat log/lean-to/crates/
  barrel/bedroll, 3 cloud billboards) — the same set S09 uses
  (`jgStage({onoda, suzuki, rifle})`, fire lit). The only diffs
  from the s04 copy are the two flame statics, re-evaluated at
  this file's `time` (below).
- S09 stages no plane, leaflets, calendar or rifle-down: no
  extra nodes exist in the file.

### Atmosphere / lights / post
- `atmosphere: "noon"` — noonJG() =
  applyAtmosphere(JG.scene, 'noon', { rig: JG.sun,
  sky: false }). The `lights` block carries the POST-atmosphere
  rig per ruling R1 (spec §14.5): key #FFF0D6 ×2.6 at [7,12,5]
  → target [0,0,−2], shadowBox 17, far 80, bias −0.0006,
  normalBias 0.02; rim #FFFFFF ×0.4 at [−3,3,−3]; hemisphere
  #CFE6FF/#C48A5A ×1.25; envIntensity 0.25 — identical to
  src/film.rs rig_noon (film.rs:1176). The fire point carries
  the R1 rendered basis **4.6** (never the constructor 5.0)
  with distance 7 / decay 1.6 and a fireLevel channel
  (`(4.6 + 0.7·sin(17t) + 0.5·sin(7.3t)) · level`, level 1;
  value at `time` 4.076015727691372 recorded in the channel).
- No `envTint` on either character: the noon preset's tint is
  the identity tint (env.ts noon: sat 1, warm 0, exposure 1,
  fogMix 0), so envMatch changes no pixel (same treatment as
  S01/S04/S05b; contrast S07 golden, which records its tint).
- `post` = G.jungle grade (shots.ts G table): bokeh aperture
  0.0012 / maxblur 0.007, bloom strength 0.16 (threshold 0.85 /
  radius 0.6 are the format defaults), grade saturation 0.8 /
  warm 0.04 / vignette 0.24 / grain 0.012. Bokeh
  `focus = 5.6708228225257145` is derived: |eye − target| with
  the camera evaluated at `time`; `post.focusChannel` records
  the per-frame rule with atStart 6.075154319027625 /
  atEnd 5.520905722795853. **Grade `time` is 0** per ruling R3
  (spec §14.5): no source path ever sets the grade time, so
  the grain is a fixed spatial pattern. (The Batch-1 files
  wrote frame t here, pre-ruling; this file follows R3.)

### Camera (push-in 54°)
- `camera.keys`: film-time keys [41.732 → from (0.3, 1.75,
  4.9)], [45.365 → to (0.25, 1.68, 4.35)], target
  (0.25, 1.2, −1.15) held, fovY 54 (the S09 shoot call;
  identical staging in src/film.rs s09). Static eye/target
  repeat the first key (the §14.4 fallback law). Evaluated eye
  at `time` (camK k = 0.7294278547674429): (0.263529, 1.698940,
  4.498815) — recorded via the focus values above, not as a key.
- Declared approximation (shared with S04/S07): the staging
  eases with camK hold 0.1 (recorded as `shot.cameraTrack`),
  while the v2 key evaluator applies plain smootherstep
  between keys. Endpoints exact; mid-shot differs by the hold
  term.

### Characters
- Looks from the cast registry (shots.ts LOOK table): Onoda
  body #5C6644 / cap #4C5638 with the worn shade #878E6E (a
  non-identity field), the JG perspective instance
  (`perspective: true`); Suzuki body #C96A42 / cap #E8B83A /
  extra [glasses], no shade override, `perspective: false`.
  Both at height 1.75 (the Toon default; person() passes no
  height).
- Onoda at (−0.95, 0, −1.5), static through the shot. `state`
  at `time`: mood determined, armR [56, −116] (the chest-tap
  poseTrack AT its peak key — see channels), armL [12, 4],
  blink 0 (director seed 3 and the 'commander' blinkAt beat
  44.04 both evaluate to 0 at t = 43.92; the beat fires
  0.12 s later). Everything else is the Toon resolved default
  (toon.ts:179: look [0,0], talk 0, legs stand, lean/bob/
  squash 0, headTilt/headTurn 0): S09's set() names no other
  fields, and set() resolves fresh from defaults every call —
  there is no cross-shot state carry-over to transcribe.
  **camPitch is 0**: S09 never passes camPitch (unlike S01/S07,
  which call camPitchOf), so the drawing compensation is off;
  the face() plane pitch is renderer-side (perspective) and is
  not scene data.
- **Rifle ON the character node** (ruling R2): Onoda's
  `attachments` = [{kind: rifle, hand: R, offset:
  [0, 0.03, 0.07]}] — the shots.ts attach of rifleHeld to
  jgOnoda; Suzuki carries none (jgStage's rifle is Onoda's).
- Suzuki at (1.55, 0, −0.8), static through the shot, holding
  the paper MAP in his left hand for the whole shot:
  `state.holdL = {plate: "paper-map", x0: −200, y0: −270}`
  (plate pre-rendered by film/gen_assets.py from
  PROPS.paper('MAP'); origin from film/assets/plates.json) —
  the same plate/origin as S08/S08b. `state` at `time` is the
  THINK bundle (see channels): mood neutral, armR [48, −118],
  armL [20, −30], headTilt 6, headTurn 0.3, look [0.4, −0.3],
  lean 0 (think carries no lean; the resolved default),
  blink 0 (director seed 5 at t = 43.92).

### Keyframed channels (additive `channels` fields)
- Onoda `armR`: poseTrack on the lt clock — shoulder keys
  [[0, 12], [2.188, 56, 0.3], [3.408, 14, 0.45]] and elbow
  keys [[0, 4], [2.188, −116, 0.3], [3.408, 6, 0.45]], the
  exact key times being wComm − st − 0.12 = 2.188 and
  wComm − st + 1.1 = 3.408 (wComm = Wd('commander', 8) =
  44.04). The tap peak lands 0.12 s BEFORE the word, per the
  staging's own key placement; after lt 3.408 the pose holds
  at [14, 6] to the cut.
- Onoda `blink`: director seed 3 + blinkAt beat
  {word 'commander', at 44.04} (combined by max, as staged).
- Suzuki `pose`: a STEP channel — POSES.listen until film
  t = Wd('only', 8) + 0.3 = 42.82 (lt 1.088), then
  POSES.think. The source is a hard ternary
  (`t > wOnly + 0.3 ? POSES.think : POSES.listen`), no
  interpolation; both bundles are spelled out in the channel
  (listen: armR [118, −105], armL [10, 8], lean 3, headTilt
  −5, headTurn −0.2, look [0, 0]; think: as in `state`).
  holdL and blink sit outside the pose spread in the source
  set() call and do not change at the switch.
- Suzuki `blink`: director seed 5, no word beats in this shot.
- Fire (top-level `channels`): flame/flame2 scaleY flicker
  pair (fireLevel(1, t)); the flame nodes' static scaleY in
  this file is the formula at `time` (0.833593 / 1.166407,
  rounded to 6 dp as in the sibling files).

### Divergences flagged
- **None in S09's staging**: src/film.rs `s09` (film.rs:1770)
  agrees with shots.ts on every value transcribed here —
  st0 41.732, camera from/to/target, fov 54, camK hold 0.1,
  both poseTrack key sets, armL [12, 4], blink_of(3, [wComm]),
  positions, the think/listen step at wOnly + 0.3, Suzuki
  hold_l "paper-map", rifle on Onoda (char(..., true)).
- Sibling-convention divergences, resolved by the §14.5
  rulings rather than by copying the Batch-1 files: grade
  `time` 0 here (R3) vs frame t in s04/s05b/s07; fire point
  basis 4.6 here (R1) vs s05b's evaluated-at-time 4.968072;
  attachments on the character node here (R2) vs s05b's
  top-level block (since reconciled by seat W19).

### Inexpressible in format v2 (declared, not substituted)
1. **Channel evaluation**: the loader reads a character's
   static `state` at `time` only; the poseTrack, the pose
   step, the blink directors/beats and the fire formulas
   exist as data for the film evaluator (src/film.rs s09
   implements all of them).
2. **A pose STEP as a format construct**: there is no v2
   channel type for a hard pose-bundle switch; the listen →
   think transition is recorded as additive channel data with
   both bundles spelled out.
3. **camK hold 0.1** — see the camera note; recorded in
   `shot.cameraTrack` only.
4. **Rifle geometry**: the attachment records kind/hand/
   offset only; the makeRifle() model (shots.ts) is drawn by
   the film evaluator, not by the loader.
5. **Caption strips**: chunk ids/timing/fit are data; the
   strip images are film assets (film/gen_assets.py), not
   scene-format textures.

## S08b — s08b.scene.json (JG noon, film t 40.36–41.73) — seat W12

### Timing / shot identity
- `shot.at = Wd('refused', 7) − 0.1 = 40.46 − 0.1 = 40.36`
  (shots.ts:458 — S08b is word-anchored like S05b/S07, not cut());
  `end = cut(8) = L(8) − 0.06 = 41.792 − 0.06 = 41.732` (S09's at);
  `duration = 1.372`. Anchors from the project's `data/words.json` /
  `data/lines.json`; matches INVENTORY.md S08b row and src/film.rs
  (ShotInfo S08b at = refused7 − 0.1; s09 st0 = 41.732). Timing
  fields are written as f32 casts of the f64 anchor arithmetic
  (f32 data law), e.g. duration 1.371999979019165.
- `time = 0.686` is the evaluation instant (shot-local lt,
  mid-shot): every character `state`, the base camera and
  `post.bokeh.focus` are the true values at film t = 41.046.
  Camera keys are shot-local (0 → duration), the S06/S07
  convention.
- Captions: `cap_046` "Onoda still refused" (39.95–40.83) —
  begins inside S08's window and is on screen for S08b's first
  0.47 s; `cap_047` "to leave." (40.83–41.762) — ends 0.030 s
  past the shot end (41.732), cut off mid-window as in the film
  (the S07/cap_041 pattern). id/line/s/e/text/width/fit/strip
  copied verbatim from `film/onoda.json` captions (line 7).
- No audio cues in this shot (the `cue()` calls in shots.ts
  belong to S01/S05a/S07/S10/S11/S12) and no film fades (the
  fade-in is S01's, the fade-out S12's — main.ts); the shot
  block carries neither.

### Set (JG camp)
- Nodes 0–79 are `s01.scene.json`'s JG set verbatim (sky dome,
  grass ground, dirt patch, 4 hills + trees, 19-tree ring,
  3 palms, 900-tuft scatter, full camp) — the set S08b draws
  (`noonJG(); jgStage({ onoda: true, suzuki: true, rifle:
  true })`, shots.ts:460). S01's character node is dropped;
  the two S08b character nodes are appended at indices 80–81
  in cast draw order (onoda → suzuki, as in jgCast).
- Flames re-baked at the file's instant under `fireLevel(1)`
  (shots.ts:289): f = 1 + 0.16·sin(13t) + 0.07·sin(29t + 1)
  at t = 41.046 = 0.88366 → flame scaleY 0.8836559057235718,
  flame inner scaleY (2 − f) = 1.1163440942764282. (S01's file
  carries the same nodes baked at its own instant.)
- Textures: sky/grass/cloud as in S01, plus `paper-map`
  (`../../film/assets/paper-map.png`).

### Atmosphere / lights / post
- `lights` = S01's block verbatim — the POST-atmosphere noon
  rig per ruling R1 (key #FFF0D6 ×2.6 [7,12,5]→[0,0,−2],
  shadowBox 17, rim ×0.4, hemisphere #CFE6FF/#C48A5A ×1.25,
  envIntensity 0.25) including the fire point light (#FF9A3C,
  basis 4.6 at [−1.35, 0.6, −2.45], distance 7, decay 1.6)
  with its `fireLevel` channel at level 1. At the evaluation
  instant the channel formula gives 4.6 + 0.7·sin(17t) +
  0.5·sin(7.3t) = 4.3758 — carried as the channel, not baked
  (the S01/S04/S07 convention).
- `post` = G.jungle as in S01 (bokeh aperture 0.0012 /
  maxblur 0.007, bloom 0.85/0.16/0.6, grade saturation 0.8 /
  warm 0.04 / vignette 0.24 / grain 0.012 / exposure 1.0 /
  time 0 per ruling R3), with focus = |eye − target| at the
  evaluation instant = 5.999403953552246.

### Camera
- Push-in 54° (shots.ts:467): eye (0.3, 1.78, 5.1) →
  (0.25, 1.7, 4.55), target (0.25, 1.2, −1.15) constant,
  driven by k = camK(lt, d) with hold 0.1 — recorded as
  `shot.cameraTrack {kind: "camK", hold: 0.1}` plus two
  loader keys. At `time`, k = 0.5 exactly (mid-shot is camK's
  midpoint), so the base eye is the exact midpoint
  (0.275, 1.74, 4.825). Loader-key divergence (declared,
  shared with S03/S04): plain smootherstep keys vs camK's
  hold differ by max |Δk| = 0.0596, ≈ 0.033 m on this eye
  path, zero at both ends and at mid-shot.

### Characters
- **Onoda** — character node at (−0.95, 0, −1.5)
  (shots.ts:461); the JG instance `person('onoda',
  { perspective: true }, '#878E6E')` (shots.ts:242) → height
  1.75, perspective true, look {body #5C6644, shade #878E6E,
  hat cap, hatCol #4C5638}. State at `time` (shots.ts:464):
  mood determined, armR [16, −6], armL [12, 4], lean 1,
  headTurn = shake(41.046) = 0.42 · settle(41.046, 40.46,
  5.5, 3.4) = 0.05645225569605827, blink =
  blinkDirector(3)(41.046) = 0, camPitch = camPitchOf at the
  evaluated camera = atan2(1.74 − 1.05, hypot(1.225, 6.325))
  / 0.9 = 0.11854889243841171 (core/toon.ts:720, H·0.6 =
  1.05). Rifle: `attachments` ON the character node per
  ruling R2 — {kind: "rifle", hand: "R", offset:
  [0, 0.03, 0.07]} = `jgOnoda.attach(rifleHeld, 'R',
  { offset: [0, 0.03, 0.07] })` (shots.ts:258), shown by
  `jgStage({ rifle: true })`.
- **Suzuki** — character node at (1.55, 0, −0.8)
  (shots.ts:465); `person('suzuki')` (shots.ts:244) →
  height 1.75, perspective false, look {body #C96A42, hat
  cap, hatCol #E8B83A, extra ["glasses"]} (cast LOOK,
  shots.ts:31; no shade override is passed for Suzuki, so
  no `shade` field — the S04 companion convention). State
  at `time` (shots.ts:466): `{ ...POSES.listen, mood:
  'worried' }` = armR [118, −105], armL [10, 8], lean 3,
  headTilt −5, headTurn −0.2 (core/poses.ts:16, with the
  pose's mood 'neutral' overridden to 'worried'), blink =
  blinkDirector(5)(41.046) = 0, holdL = the paper MAP
  plate {plate: "paper-map", x0: −200, y0: −270} (origin
  from film/assets/plates.json). The set() field `v: 'map'`
  is the painter's variant key for PROPS.paper; in the
  format the plate id carries the variant (paper-map is
  the MAP drawing — film/gen_assets.py `paper_plate()`:
  translate(0,−70), rotate(−0.08), rect(−70,−90,140,180),
  'MAP' 900 30px, 6 grey lines).
- The MAP hold was authored from source (core/toon.ts
  PROPS.paper + gen_assets.py); s08.scene.json (seat W11,
  landed in parallel) was checked afterwards — it uses the
  identical hold (plate "paper-map", x0 −200, y0 −270, same
  texture entry). The two files agree.

### Channels (data only — the loader renders the state at `time`)
- Onoda: `headTurn` formula channel on the film clock
  (t > 40.46 ? 0.42 · settle(t, 40.46, 5.5, 3.4) : 0);
  `blink` director seed 3; `camPitch` camPitchOf channel
  (divisor 0.9).
- Suzuki: `pose` channel (listen, with the mood override
  noted); `blink` director seed 5; `holdL` plate channel.
- Fire: the `fireLevel` channel on the point light; the
  flame flicker formula (above) baked into the two flame
  nodes at `time`.

### Inexpressible (declared, not substituted)
1. **Rifle as scene geometry** — v2 has no attachment
   geometry; the ON-node `attachments` entry (R2) is the
   record, and film.rs draws makeRifle() via the card's
   hand anchor. Geometry spec (shots.ts:74–83): stock box
   0.075×0.52×0.13 wood #6E4F30 rough 0.85 at y −0.17;
   receiver 0.06×0.1×0.09 steel #3A3D42 metal 0.55 rough
   0.5 at y 0.13; barrel cyl r 0.016 h 0.55 steel at
   y 0.42; front sight 0.02×0.05×0.02 steel at y 0.68;
   grip swell 0.05×0.05×0.16 wood at (0, −0.02, 0.05).
2. **Channel evaluation over time** — headTurn settle,
   blink directors, camK, fireLevel/flicker exist as data;
   the loader renders the one evaluated state at `time`
   (shared with every sibling file).
3. **camK hold in loader keys** — bounded in Camera above
   (max ≈ 0.033 m, zero at the ends and at `time`).
4. **Cloud sprite tint #EFF4EA** — billboards have no tint
   field (shared: S03/S05b).
5. **Sky-gradient ground stop #BFE4F7** — generator
   constant (shared).
6. **Hill grass colour×map** — a v2 sphere takes material
   XOR texture; the set nodes carry shadeHash colour only
   (shared: S03; the ab1 convention).
7. **Per-node shadow flags** — the format carries none
   (shared).
8. **envMatch tint** — both characters are envMatch: true
   in source (person(), shots.ts:35–38); the noon tint is
   a film-renderer uniform (film.rs TINT_NOON), not
   per-file data — S01/S04 likewise record no envTint at
   noon (S07's golden envTint is the exception).
9. **Caption strip images** — film assets
   (film/assets/cap_046.png, cap_047.png); chunks are
   carried as data only (shared).

### Divergences flagged
- **None against src/film.rs**: its `s08b()` (film.rs:1743)
  agrees with shots.ts on every staging value used here —
  positions, arms, lean, the 0.42 · settle(·, 5.5, 3.4)
  head-shake, pose_listen + worried, the paper-map hold,
  camera (0.3, 1.78, 5.1) → (0.25, 1.7, 4.55), target
  (0.25, 1.2, −1.15), fov 54, blink seeds 3/5. Per ruling
  R4 there was nothing to resolve.
- Staging note, recorded not "fixed": the head-shake's
  settle() starts at wRef = 40.46, 0.1 s AFTER the cut
  (40.36) — for the shot's first 0.1 s headTurn is exactly
  0 by the source formula.

## S12 — s12.scene.json (JG DUSK, film t 55.52–60.357) — seat W17

### Timing / shot identity
- `shot.at = cut(11) = L(11) − 0.06 = 55.58 − 0.06 = 55.52`
  (`shots.ts:125` cut rule; line starts from `film/onoda.json`
  lines[11], generated from the project's `data/lines.json`);
  `end = 60.357` (the film duration, `film/onoda.json`);
  `duration = 4.837`. INVENTORY.md's S12 row (55.52–60.36) is
  the rounded form. The shot spans frames n 1666…1810
  (t 55.5333…60.3333) — 145 of the film's 1,811 frames.
- `time = 3.48` is shot-local lt (the S06/S07 clock convention;
  camera keys are lt-keyed), i.e. film t = 59.0. The instant is
  a settled one, chosen deliberately: the bow completes at
  lt 2.5, the mood step lands at lt 2.6 (film 58.12), both
  blink terms evaluate to 0 there, and the film fade-out (from
  film 59.957) has not begun. Every character `state`, flame
  scale and fire value below is the value at film t = 59.0,
  per the v2 loader law (the state in the file is the state
  at `time`).
- `shot.fades { in: 0.25, out: 0.4 }` — the main.ts:35
  film-level law `fade = min(clamp(t / 0.25), clamp((T.dur
  − t) / 0.4))`; only the out-term touches this shot (black
  ramps in over film 59.957–60.357, lt 4.437–4.837). Recorded
  as data (fades are not a scene-format field), the S01
  convention.
- `shot.cues` — `cue(st + 0.4, 'chirp', { gain: 0.16, dur:
  1.8 })` from the S12 add (`shots.ts:541`) → t 55.92. Audio
  is outside the scene format, recorded as data only.
- Captions: cap_061…cap_065 (line 11) with s/e/text/width/
  fit/strip copied verbatim from `film/onoda.json`. Note
  cap_060 "after 29 years." (line 10) ends at 55.55 and so
  overlaps the shot head by 0.03 s, as in the film; it is
  line 10's chunk and belongs to the S11 file — chunks are
  film-level data, the overlap is staging truth.

### Set (JG camp)
- Nodes/textures are `s07.scene.json`'s JG set transformed —
  the same builder output S12 uses (`shots.ts` JG builder;
  S12 calls `jgStage({ onoda2: true, tani: true, down:
  true })`) — with the dusk re-dress: sky texture top
  #3B3F8F / horizon #F59C6B, the three cloud billboards carry
  additive `"tint": "#FFC9B0"`, everything else verbatim.
  S07's palm `"role": "walk-target"` annotation is dropped
  (it was S07-specific staging metadata).
- The two flame cones return (`fireLevel(1, t)` — S07
  omitted them at level 0): nodes from
  `scenes/ab1.scene.json` with scaleY baked at the file
  time (the S06 convention): f = 1 + 0.16·sin(13t) +
  0.07·sin(29t + 1) at film t = 59.0 = 1.0814995381649202;
  flame scale [1, f, 1], flame inner [1, 2 − f, 1] =
  0.9185004618350798.
- Fire pit at (−1.35, 0, −2.45) (`shot.fire.pit`, level 1).

### Atmosphere / lights / post
- `shot.atmosphere: "dusk"` — S12 calls
  `applyAtmosphere(JG.scene, 'dusk', { rig: JG.sun })` with
  no `sky: false`, so sky + clouds are re-dressed and
  `scene.fog` is set from the preset (`core/env.ts` dusk).
- `lights` block is the rig AFTER applyAtmosphere('dusk'),
  per ruling R1 (POST-atmosphere values): key #FFB27A ×2.0
  at the preset's keyPos [−8, 5, −14], target stays the JG
  rig's [0, 0, −2], shadow true, shadowBox 17 / far 80 /
  bias −0.0006 / normalBias 0.02 (the JG sunlight +
  lightRig fields the atmosphere does not override, as in
  the noon files); rim #FFFFFF ×0.5 at [−3, 3, −3];
  hemisphere #B9A2D8 / #7A4A3A ×0.9; envIntensity 0.25.
- `fog` (top-level, the s06 shape — format v2 has no fog
  field): #F59C6B, near 28, far 120 (dusk preset). `clear`
  = #F59C6B (the preset horizon colour).
- Fire point: house form per R1 — static basis 4.6 with the
  `fireLevel` channel at level 1; the formula at film
  t = 59.0 evaluates to 3.9339081298560488.
- `post` = gradeOf(G.jungle, dusk) (`shots.ts:139`:
  sat/warm/exposure come from the preset grade over the
  G.jungle base): saturation 1.08 / warm 0.06 / exposure
  0.97 / vignette 0.24 / grain 0.012; bloom strength 0.16
  (threshold 0.85 / radius 0.6 are `core/engine.ts`
  composer constants); bokeh aperture 0.0012 / maxblur
  0.007, focus = |eye − target| at the file time =
  11.389776820801682 (`post.focusChannel` carries atStart
  5.815926409438139 / atTime / atEnd 11.881287808987711;
  the source computes focus per frame, the format takes
  one static value).
- `grade.time = 0` per ruling R3 (no source path ever sets
  the grade's time uniform; the grain is a fixed spatial
  pattern). DIVERGENCE FROM S07: the pre-ruling S07 file
  records `grade.time = 30.39` — reconcile S07 to R3, not
  this file.

### Camera (crane up/away 46°)
- `camera.keys`: lt 0 eye [0.4, 1.8, 4.6] target
  [0.05, 1.0, −1.15]; lt 4.837 eye [1.6, 4.6, 9.8] target
  [0.05, 0.8, −1.35]; fovY 46 at both keys — the
  `MO.crane` endpoints (`shots.ts` S12 draw; shoot() is
  called with the crane position pre-mixed, from == to,
  mix 0). Static eye/target/fovY repeat the first key
  (the §14.4 fallback law).
- At the file time: k = camK(3.48, 4.837) =
  0.9204535282241786 → eye [1.5045442338690145,
  4.3772698790277, 9.386358346765729], target [0.05,
  0.8159092943551643, −1.3340907056448357] (recorded in
  `post.focusChannel`).
- Derived/approximate: the staging mixes the crane on
  `camK(lt, d)` (hold 0.1, `core/motion.ts`), while the v2
  key evaluator applies plain smootherstep between keys.
  Endpoints exact; mid-shot path differs by the hold term.
  Recorded as `shot.cameraTrack { kind: "camK", hold:
  0.1 }` (inexpressible item 2).

### Characters
- **Onoda** is the `jgOnoda2` instance
  (`person('onoda', { perspective: true }, '#878E6E')`,
  `shots.ts:240`) at (−0.15, 0, −0.95) — the second,
  rifle-less instance of the same registered man that S11
  swaps to at the drop beat (attach3D re-shows its object
  on every face(), so the swap is the only way the rifle
  leaves his hand — `shots.ts` S11 comment). Look from the
  cast registry (body #5C6644, hat cap, hatCol #4C5638)
  with the worn shade #878E6E; `height: 1.75` (the Toon
  default — no person() call overrides it).
- Onoda `state` at film t = 59.0: mood 'neutral' (the
  'relieved' → 'neutral' step at wLoyal lands at 58.12),
  armR [9, 5], armL [9, 5], look [0.25, −0.15], lean 2.1
  (= bowA·0.6, bowA settled at 3.5), headTilt −3.5
  (= −bowA), blink 0 (blinkDirector seed 3 evaluates to 0
  at 59.0; blinkAt's 'loyal' beat at 58.12 is 0.88 s past
  its 0.12 s window). Resolved defaults the staging never
  sets: legs 'stand', phase 0, bob 0, squash 0, talk 0,
  headTurn 0.
- DIVERGENCE FLAG — **camPitch**: neither S12's nor S11's
  `jgOnoda2.set()` ever passes camPitch, so the resolved
  state carries camPitch 0 (`core/toon.ts:179`:
  `camPitch: st.camPitch ?? 0`) — recorded explicitly in
  `state`. This is staging truth, not an omission: S01
  and S07 pass `camPitchOf` per frame; S11/S12 do not.
  An evaluator that auto-applies camPitchOf to perspective
  characters would diverge on every frame of both shots
  (at this file's time camPitchOf(jgOnoda2) would be
  0.34194935469375076; the term only scales head radius
  and slides features, but it is systematic).
- **Taniguchi** (`person('taniguchi')`, no opts →
  perspective false) at (2.3, 0, −2.6) — his S12 `at()`.
  In S11 he stood at (1.3, 0, −1.3); the change is a cut,
  not a walk (positions are per-shot at() calls — do not
  "fix" it as a continuity error). Full cast look (vest
  #33304A on body #4A4560, bowler #2E2A3A, tie #7A2E3A,
  glasses + mustache #C9C9D2). State: mood 'neutral',
  armR/armL [10, 6], headTilt −3, blink 0 (blinkDirector
  seed 9 at 59.0). He is still — his only channel is
  blink.
- `envTint` (additive, both nodes): the dusk envMatch
  tint { sat 0.97, warm 0.06, exposure 0.96, fogColor
  #F59C6B, fogMix 0.08 } (`core/env.ts` dusk tint;
  person() sets envMatch: true).

### Rifle (ground node)
- Group node "rifle (on ground)": the five `makeRifle()`
  parts (`shots.ts:74–83`) — stock box 0.075×0.52×0.13 at
  [0, −0.17, 0], receiver box 0.06×0.1×0.09 at
  [0, 0.13, 0], barrel cylinder r0.016 h0.55 (8 seg) at
  [0, 0.42, 0], front sight box 0.02×0.05×0.02 at
  [0, 0.68, 0], grip swell box 0.05×0.05×0.16 at
  [0, −0.02, 0.05]; wood #6E4F30 roughness 0.85, steel
  #3A3D42 roughness 0.5 — at the resting transform:
  position [0.62, 0.07, −0.1], rotation euler_xyz
  [0, −0.5, π/2].
- Consistency with S11: **s11.scene.json does not exist
  yet** (Batch 2, seat W16), so no sibling file could be
  matched; the transform is derived from source and the
  derivation is exact — construction (`shots.ts:256`)
  sets position (0.62, 0.07, −0.1) / rotation
  (0, −0.5, π/2); S11's drop animates only position.y
  (0.32 → 0.07, eo easing) and rotation.z
  (π/2 − 0.1·(1 − eo(u)) → π/2) over dropT…dropT + 0.3,
  with dropT = Wd('surrendered', 10) + 0.55 = 53.57
  (settled by 53.87, before S11 ends); the S12 draw
  re-pins position.y = 0.07 and rotation.z = π/2 verbatim
  (`shots.ts:531`). When s11 lands, its drop end state
  must equal this transform exactly.
- Onoda2 carries NO `attachments`: the held-rifle
  attachment belongs to the `jgOnoda` instance (S01–S11
  pre-drop). Recording one here would re-arm him.

### Keyframed channels (additive `channels` fields)
The v2 loader reads a character's static `state` at
`time` only (spec §14.3); time-varying formulas are
recorded in `channels` objects (ignored by the loader,
implemented by the film evaluator):
- Onoda `mood`: step keys [[0, 'relieved'], [2.6,
  'neutral']] (lt; the step is film t = wLoyal = 58.12).
- Onoda `lean` / `headTilt`: the bow poseTrack
  [[0, 0], [2.5, 3.5, 0.45]] on lt (key time =
  wLoyal − st − 0.1 = 2.5; semantics `core/motion.ts`
  poseTrack — the move runs over its moveSecs ending at
  its time); lean = bowA·0.6, headTilt = −bowA.
- Onoda `blink`: blinkOf.onoda = blinkDirector(seed 3)
  max blinkAt at the 'loyal' beat, Wd('loyal', 11) =
  58.12 (anchor loyal11 in `film/onoda.json`), both on
  the film clock.
- Taniguchi `blink`: blinkDirector(seed 9), film clock.
- Fire point `channel`: intensity
  `(4.6 + 0.7·sin(17t) + 0.5·sin(7.3t)) · level`, level 1
  (`shots.ts` fireLevel(1, t)), and the flame meshes'
  scaleY flicker pair f / 2 − f (formula in Set, above).
- Bokeh `focus`: |eye − target| per frame
  (`post.focusChannel`).

### Inexpressible in format v2 (declared, not substituted)
1. **The rifle's identity.** Its geometry is expressible
   (the group node above), but the format has no rifle
   construct: the loader knows the rifle only as a
   character attachment or the v1 `ab1-onoda-rifle`
   code-ref. A consumer sees five boxes and a cylinder,
   not "the rifle". The steel's metalness 0.55 also has
   no material field — carried as an additive `metalness`
   on the steel parts (the W10 shore precedent), unread
   by the loader.
2. **camK hold** in camera keys (see Camera note).
3. **Channel evaluation** (mood / bow / blink / fire
   flicker / flame scaleY / bokeh focus): the loader
   renders the state at `time`; the formulas exist as
   data but no loader code path evaluates them over the
   shot's duration.
4. **Scene fog**: no fog field in format v2; recorded as
   top-level `fog` (the s06 shape).
5. **Cloud tint**: billboard nodes have no tint field;
   the dusk tint #FFC9B0 is recorded additively (the S07
   precedent; film.rs `add_clouds` renders clouds
   untinted — the data follows the project source).
6. **Grade exposure / saturation naming**: files write
   `saturation` + `exposure` (house convention); the
   loader's grade block reads `sat` and carries no
   exposure field, so exposure 0.97 is data-only until
   the format grows it.
7. **Character env tint**: no tint field on the v2
   character node; the dusk envMatch tint is recorded as
   `envTint`.
8. **Sky-gradient ground stop**: the generator takes
   top/horizon only (the S08-declared limit). For dusk
   the preset's skyGnd #F59C6B equals the horizon, so
   nothing is lost on this shot.
9. **Caption strips**: chunk ids/timing/fit are data
   (above); the strip images are film assets
   (film/gen_assets.py), not scene-format textures.
10. **Audio cue + fades**: recorded in `shot.cues` /
    `shot.fades` as data only; neither exists in the
    scene format.

## S10 — s10.scene.json (BK bookshop noon, film t 45.365–50.648) — seat W14

### Timing / shot identity
- `shot.at = cut(9) = L(9) − 0.06 = 45.365`, `end = cut(10) =
  50.648`, `duration = 5.283` — project `shots.ts:125` (cut rule)
  with line starts from `film/onoda.json` lines[9].start = 45.425,
  lines[10].start = 50.708. INVENTORY.md's S10 row (45.37–50.65)
  is the rounded form.
- Word anchors from `film/onoda.json`: `bookseller9` = 48.16
  (wBooks), `flew9` = 49.22 (wFlew). The 'plop' cue fires at
  wFlew + 0.42 = 49.64, gain 0.3 (`shots.ts` S10 tail) — data
  only in `shot.cues`.
- `time = 48.6` is the file's evaluation instant — inside the
  awe window (wBooks ≤ t < wFlew), book still open, walk not
  started. Every character `state`, the camera-derived bokeh
  focus and the fire-free lights block are the state at that t,
  per the v2 loader law (src/scenefile.rs §14.3).
- Captions: chunk objects 52–56 (line 9) with index/s/e/text/
  width/fit/strip copied from `film/onoda.json` captions. Chunk
  51 ("commander ordered it.", line 8, e = 45.395) is still on
  screen for the first 0.03 s of the shot; it belongs to line 8
  and is not listed. Chunk 56 ("to the island.", e = 50.69)
  overruns the cut by 0.042 s into S11; it is line 9's chunk and
  is listed.

### Set (BK bookshop — `WD.interior` + shelves/desk/parcels)
- The room is `WD.interior(E, { wall: '#5A4632', motif:
  '#6B5638', ceiling: '#3A2E22', floor: '#7A5230', signText:
  'TANIGUCHI BOOKS', picture: true, litter: false })`
  (`shots.ts:227`) with the builder's defaults w = 8, depth = 7,
  h = 4.2, bulbs = true, counterOn = true (`core/world.ts:292`).
  All geometry below is that call's output, in build order:
  floor plane 8×9 at z = −2.5 (planks, map rotated π/2 — see
  Inexpressible 1); back wall at z = −7 as four boxes around the
  window opening (ww 3.2, wh 1.9, wy 1.1) + the #2F5C52 frame
  (sill/head/jambs/mullion) + the flat view plane 8.32×4.94 at
  local z = −2.5; side walls + wainscots; ceiling + cornice;
  counter at (2.7, 0, −2.9) (planks body, felt top, brass rail);
  picture at (−3.87, 2.4, −3.5) rotY π/2; sign board at
  (3.87, 2.9, −2.9) rotY −π/2; two bulb assemblies (cord +
  sphere + point) at (−1.5, −2.2) and (1.6, −4.5).
  `litter: false` — the builder constructs the 140-slip
  InstancedMesh but never adds it; no litter nodes exist.
- Shelves (`shots.ts:228–230`, `shelfUnit` `shots.ts:105`):
  unit 1 (3.4×2.5, seed 1) at (−1.8, 0, −6.55); unit 2
  (2.4×2.5, seed 2) at (1.9, 0, −6.55); unit 3 (3.2×2.5, seed 3)
  at (−3.82, 0, −3.6) rotY π/2. Frame #5A4630 roughness 0.9:
  uprights at x = ±w/2, top board (w, 0.06, 0.34), bottom board
  (w, 0.06, 0.3), four row boards (w, 0.05, 0.32) at
  y = 0.12 + r·0.58. Book rows were GENERATED with the source
  algorithm verbatim: x from −w/2 + 0.12 while x < w/2 − 0.16;
  bw = 0.055 + rnd(seed·31 + r·17 + i)·0.05; a book is placed
  iff rnd(seed·91 + r·13 + i) > 0.14, with bh = 0.34 +
  rnd(seed·47 + r·7 + i·3)·0.13, colour
  cols[(i + r·3 + seed) mod 8] from the 8-colour shelf palette,
  centre (x + bw/2, y + 0.03 + bh/2, 0.02), depth 0.2,
  roughness 0.85, leaning (rotation.z 0.1) iff
  rnd(i·7 + r + seed) > 0.93; x advances bw + 0.012 every
  iteration, placed or not. rnd is `core/lib.ts:25`
  (sin-hash). Yield: 115 / 79 / 116 books (310 total). The
  hash was evaluated with the host libm `sin` in f64 — a
  last-ulp difference from V8's could flip a borderline
  gap/tilt test (the S02 hill-tree caveat, same class).
- Desk (`PR.desk(1.6)`, `core/props.ts:162`) at (−1.5, 0, −3.4)
  rotY 0.15: felt top #2E5A44 (1.6, 0.08, 1.0) at y 0.84, rim
  (w + 0.1, 0.06, 1.1) at 0.78, two pedestals (0.5, 0.76, 0.95)
  at x ±(w/2 − 0.3) = ±0.5, back panel (w − 1, 0.6, 0.06) at
  (0, 0.45, −0.45); wood = planks #6B3E26 [1,1]. Parcels
  (`PR.parcel(..., false)`, `core/props.ts:158`): (0.5, 0.16,
  0.36) group at (−1.4, 0.8, −3.35) rotY 0.4 and (0.44, 0.13,
  0.32) group at (−1.35, 0.95, −3.3) rotY 0.15; the boxes sit
  at local y = h/2 inside their groups (film.rs folds this
  into the world centre — same result).

### Atmosphere / lights / post
- `atmosphere: "noon"` — `applyAtmosphere(BK.scene, 'noon')`
  (`shots.ts:236`) is called with NO rig argument, so R1 is
  satisfied trivially: nothing is re-dressed and the `lights`
  block is exactly the rig `interior()` built (the noon
  identity for this set; W01's atmospheres.json
  `baseRigs.bookshopInterior` + `filmLooks.bk-noon-shop`
  record the same). The character tint is the noon identity —
  no `envTint` (the S02 precedent).
- `lights`: key = the interior's window DirectionalLight
  #FFE4B8 ×2.4 at (−2, 6, −13) → target (0.5, 0, −1), shadow,
  shadowBox 8, far 30, bias −0.0006, **normalBias 0 — never set
  in source** (`interior()` does not use lightRig; 0 is the
  three.js DirectionalLightShadow default, flagged as such by
  W01, not a source literal). The shadow camera near (0.5) and
  radius (4) have no format fields — recorded here only.
  Hemisphere #FFE9D2 / #5A3A2A ×1.1. Points: the fill
  #FFD9A8 ×6 at (0, 3.2, −1), distance 12, decay 1.4, plus the
  two bulb points #FFD9A0 ×3 at y = h − 1.15 = 3.05, distance
  6, decay 1.6. envIntensity 0.35 (scene.environmentIntensity).
  **There is no rim light in this rig** — the block omits `rim`
  (film.rs rig_bk likewise folds rim colour [0,0,0]).
- `clear` = #1A1418 (scene.background). No `fog` key: the
  interior's fog is null and the noon call leaves it null.
- `post` = G.shop (`shots.ts:137`): bokeh aperture 0.001 /
  maxblur 0.008, bloom strength 0.3 (threshold 0.85 / radius
  0.6 are the engine's fixed pass values, `core/engine.ts`),
  grade saturation 0.78 / warm 0.06 / vignette 0.26 / grain
  0.012, exposure 1.0 (the ShotLook carries no exposure).
  Bokeh `focus = 6.319650559588458` (f32) is DERIVED:
  |eye − target| with the camera evaluated at `time`
  (k = camK(3.235, 5.283, 0.1) = 0.7497820960693715 →
  eye (0.4376, 1.8750, 0.7003)); the per-frame law is recorded
  in `post.focusChannel` (atStart 7.259476565152615, atEnd
  6.00603862791441). Post `grade.time = 0` per ruling R3 —
  the grain seed is pinned for the whole film (the S07 file
  predates R3 and carries its frame t; this file follows the
  ruling).

### Camera (push-in 46°)
- `camera.keys`: film-time keys [45.365 → eye (0.7, 1.95, 1.6)],
  [50.648 → eye (0.35, 1.85, 0.4)], target (−1.5, 1.45, −5.3)
  held, fovY 46 (`shots.ts:496` shoot call; port
  src/film.rs:1810 s10). Static eye/target repeat the first
  key (the §14.4 fallback law).
- Derived/approximate: the staging eases with `camK(lt, d)`
  (hold 0.1, `core/motion.ts:41`), recorded as
  `shot.cameraTrack`; the v2 key evaluator applies plain
  smootherstep between keys (spec §14.4). Endpoints exact;
  mid-shot position differs by the hold term.

### Character (Taniguchi)
- Look from the cast registry (`shots.ts:32`): body #4A4560,
  pattern vest + patCol #33304A, bowler #2E2A3A, tie #7A2E3A,
  extra [glasses, mustache], mustacheCol #C9C9D2 — identical to
  the S03 shore node's look. The BK instance is
  `person('taniguchi')` with NO opts (`shots.ts:251`): no
  `perspective`, no shade override (contrast the JG Onoda).
- `state` at t = 48.6: mood `awe` (wBooks passed, wFlew not),
  headTurn 0.3 (the awe window), armR [42, −28] — the
  poseTrack [[0, 42], [wFlew − st = 3.855, 17, 0.5]] is still
  in its hold at lt 3.235 (the move runs 3.355–3.855), armL
  [14, 8], look [0, 0] (never set by the staging — drawToon
  default), talk 0, legs `stand`, phase 0, bob 3
  (walkBob(0, 3) — the staging passes bob in EVERY state,
  standing included), lean 0, headTilt 0, camPitch 0 (no
  perspective → face() applies yaw only), blink 0
  (blinkDirector seed 9 evaluated at 48.6 = 0; blinkAt's
  wBooks pulse ended at 48.28), position (−1.7, 0, −5.5)
  (wk = 0: the walk starts at wFlew + 0.5 = 49.72).
- `holdR` = the book LEDGER plate, open at `time`:
  {plate "book-open", x0 −170, y0 −180} from
  `film/assets/plates.json` (`PROPS.book('LEDGER', 1)`,
  `core/toon.ts:469`); the closed plate is the channel's
  other state. The book is a hold plate, NOT an R2
  attachment — no `attachments` field.

### Keyframed channels (additive `channels` fields)
The v2 loader reads a character's static `state` at `time`
only (spec §14.3); time-varying formulas are recorded in the
`channels` object (ignored by the loader, implemented by the
film evaluator in src/film.rs s10, lines 1803–1833):
- `mood`: step on film t — neutral → awe at 48.16 →
  determined at 49.22 (`shots.ts:495`).
- `headTurn`: 0.3 while wBooks ≤ t < wFlew, else 0.
- `armR`: poseTrack keys [[0, 42], [3.855, 17, 0.5]] on the
  lt clock; second component static −28.
- `holdR`: step at wFlew — book-open/`bkOpen` → book-closed/
  `bkClosed` (the book closes ON the word 'flew'; the set()
  `v` field names the painter variant).
- `position`: wk = seg(t, wFlew + 0.5, wFlew + 1.7) on the
  FILM clock (the source's seg takes t, not lt); position =
  (−1.7, 0, lerp(−5.5, −4.95, wk)). wk reaches only 0.7733 at
  the cut — the short walk is unfinished inside the shot, as
  staged; the channel records the full segment law.
- `phase`: walkPhase(wk·0.55, 0.58); `bob`: walkBob(phase, 3);
  `legs`: 'walk' while 0 < wk < 1 else 'stand'.
- `blink`: max(blinkDirector(9)(t), blinkAt(t, [wBooks])) —
  Taniguchi's seed is 9 in every set (`shots.ts:41`).

### Painter-registry gaps (declared, not substituted)
The texture entries declare generators **wallpaper, planks,
felt, parcel, picture, sign, solid** — none exist in the
loader registry (src/scenefile.rs resolve_texture knows
grass / sky-gradient / cloud / blob-shadow / onoda-card /
dof-chart-pattern + `file`). Names follow the source painter
names (`T.wallpaper`, `T.planks`, `T.felt` in
`core/world.ts`; canvasTex keys 'pic'/'parcel…'/'sign…') and
W10's precedent for `planks`, so they can be registered once.
Each entry carries its full parameters (base/motif/repeat/
canvas size; parcel, picture and sign also carry their
painter ops as `sourceDrawing`). Mapped boxes use the
`material: {"map": id}` extension (the shore-crate dialect);
the still loader's colour path cannot carry a map on a box
(its textured kinds are quad / flat-quad / ground-disc /
sky-sphere / billboard) — the floor therefore uses flat-quad
and the view plane / picture / sign faces use quad, while
mapped boxes are film-evaluator data, as in S02/S03.

### Inexpressible in format v2 (declared, not substituted)
1. **Floor map rotation π/2**: the source sets
  `(floor.material).map.rotation = π/2` (`core/world.ts`
  interior); recorded as `rotation` on the planks-floor
  texture entry. film.rs emulates it by swapping the repeat
  to [3.6, 3.2] on its floor box.
2. **Multi-material boxes**: the picture frame and the sign
  board are 6-material boxes in the source (mapped front,
  plain sides). Authored as a plain box + a separate quad
  carrying the mapped face at local z +0.026 (film.rs does
  the same split). The parcel texture wraps all faces in the
  source — a single `map` on the box, as authored.
3. **Material metalness**: the brass rail's metalness 0.6 is
  an additive material field (only `roughness` is a format
  field, src/scenefile.rs:497–500) — recorded, unconsumed.
4. **MeshBasicMaterial surfaces**: the bulb cords (#C9A13B)
  and the window view plane (#BFE4F7, toneMapped: false) are
  unlit in the source. The cord carries an additive
  `"basic": true`; the view plane's colour enters via the
  declared `solid` texture. film.rs renders the cord as an
  emissive-only standard draw and the view as a basic quad.
5. **Bulb emissive**: the bulb spheres' emissive #FFE7A0 ×2.2
  is recorded as additive material fields (`emissive`,
  `emissiveIntensity`); the format material carries colour +
  roughness only.
6. **Key shadow near / radius**: near 0.5 and radius 4
  (`core/world.ts` interior sun) have no `lights.key` fields
  — recorded in the lights note above only.
7. **Channel evaluation** (above): the loader renders the
  state at `time`; the formulas exist as data but no loader
  code path evaluates them over the shot's duration.
8. **Caption strips**: chunk ids/timing/fit are data (above);
  the strip images are film assets (film/gen_assets.py), not
  scene-format textures.
9. **Audio cue**: the 'plop' at 49.64 is recorded in
  `shot.cues` as data only; audio does not exist in the
  scene format.

### Sources followed over film.rs where they differ (flagged)
1. **Shelf construction** (`src/film.rs` shelf_unit vs
   `shots.ts:105–121`): the port insets the uprights to
   ±(w/2 − 0.03) (source ±w/2), narrows the row boards to
   (w − 0.12, 0.04, 0.3) (source (w, 0.05, 0.32)), gives the
   bottom board depth 0.34 (source 0.3), sits books at
   by + 0.02 (source y + 0.03), advances x by bw + 0.005
   (source + 0.012), stops at w/2 − 0.14 (source w/2 − 0.16),
   and places a book when rnd(seed·91 + r·13 + i) ≤ 0.86
   where the source places when the same hash is > 0.14 —
   the two predicates agree only on (0.14, 0.86], so the
   port's rows have different books and gaps wherever the
   hash falls outside that band. This file follows the
   source on every point (R4).
2. **Legs before the walk** (`src/film.rs:1819`): the port
   sets legs Walk whenever wk < 1 — including wk = 0, before
   the walk starts (phase 0). The source's set() passes
   'stand' until wk > 0 (`shots.ts:495`). The file's state
   and `legs` channel follow the source.
3. **Ceiling motif colour**: film.rs paints the ceiling
   wallpaper with `shade_rgb('#3A2E22', 1.25)` = #493A2B —
   its shade helper multiplies in sRGB space. The source's
   `shade()` (`core/world.ts:34`) is THREE.Color
   multiplyScalar, which operates in linear space and yields
   #413427. The texture entry carries the source value.
4. **Sign board side colour**: the source's signBoard builds
   its five non-face materials from `shade(bg, 0.7)` =
   #302222 (linear-space, as item 3); film.rs folds
   `shade_lin('#3A2A2A', 0.7)`, whose sRGB-space pre-multiply
   gives (41, 29, 29) before linearising — a darker board.
   The file carries #302222.

## S11 — s11.scene.json (JG DUSK, film t 50.648–55.52) — seat W16

### Timing / shot identity
- `shot.at = cut(10) = L(10) − 0.06 = 50.708 − 0.06 = 50.648`;
  `end = cut(11) = L(11) − 0.06 = 55.58 − 0.06 = 55.52`;
  `duration = 4.872`. Anchors: `Wd('order', 10) = 51.78`
  (lt 1.132), `Wd('surrendered', 10) = 53.02` (lt 2.372),
  `dropT = wSurr + 0.55 = 53.57` (lt 2.922). Matches
  INVENTORY.md S11 row and src/film.rs s11 (`st0 = 50.648`,
  `drop_t = 53.570`).
- `time = 0` is the file's evaluation instant (shot-local
  lt = 0): character `state`s and `post.bokeh.focus` are the
  values at shot start, per the v2 loader law (§14.1/§14.3).
  All shot-local times below are lt = film t − 50.648.
- Captions: every chunk intersecting the shot window (the
  s06/s07 convention), copied verbatim from `film/onoda.json`
  (line 10): `cap_056` "to the island." (49.63–50.69 — line 9's
  last chunk, bleeds 0.042 s into the shot head), `cap_057`
  "He finally" (50.69–51.19), `cap_058` "gave the order,"
  (51.19–52.07), `cap_059` "and Onoda surrendered"
  (52.07–53.57, fit 0.92305), `cap_060` "after 29 years."
  (53.57–55.55 — starts exactly at dropT and is cut off
  mid-window by the shot end, as in the film).
- Cues (in `shot.cues`, film t): 'inhale' at
  `wOrder − 0.35 = 51.43` (gain 0.28, dur 0.7); 'thud' at
  `dropT + 0.27 = 53.84` (gain 0.5 — inside the drop window,
  as the rifle lands). The film-global fades (in 0.25 s /
  out 0.4 s, main.ts) both evaluate to 1 across S11, so the
  file carries no `shot.fades` key.

### Set (JG camp)
- Nodes/textures are `scenes/s07.scene.json`'s JG set (the
  ab1-transformed builder output; S11 stages in the same JG
  scene) with three changes: the S07 character node and the
  `coconut` texture are dropped; the S07-only additive
  `"role": "walk-target"` palm marker is stripped; and the two
  flame cones — omitted in S07, whose fire is out — are
  re-inserted from `s01.scene.json` (same nodes) after
  fire log 2, with their scale baked at this file's time via
  `fireLevel(1, t)`: at film t 50.648 the flicker
  `f = 1 + 0.16·sin(13t) + 0.07·sin(29t + 1) = 0.81348`, so
  flame scale [1, 0.81348, 1], flame inner [1, 1.18652, 1]
  (the formula itself rides on the fire point's channel).

### Atmosphere / lights / post
- Atmosphere `dusk` (`core/env.ts` preset; S11 calls
  `applyAtmosphere(JG.scene, 'dusk', {rig: JG.sun})` with no
  `sky: false`, so sky + clouds are re-dressed): sky texture
  top #3B3F8F / horizon #F59C6B (= `clear`), cloud billboards
  carry additive `"tint": "#FFC9B0"`, scene fog #F59C6B
  near 28 far 120 (top-level `fog`, the s06 shape).
- `lights` carries POST-atmosphere values (ruling R1):
  key #FFB27A i2.0 at [−8, 5, −14] — the dusk preset's
  `keyPos` replaces the JG sunlight position [7, 12, 5];
  target [0, 0, −2], shadow box 17 / far 80 / bias −0.0006 /
  normalBias 0.02 (the fields the atmosphere does not
  override); rim #FFFFFF i0.5 at [−3, 3, −3]; hemisphere
  #B9A2D8 / #7A4A3A i0.9; envIntensity 0.25. Identical to
  src/film.rs `rig_dusk(fire_light(1.0, t))`.
- Fire point: house form — intensity basis 4.6 (R1: the
  constructor's 5.0 is never rendered) with
  `channel {kind: "fireLevel", level: 1, formula: "(4.6 +
  0.7*sin(17t) + 0.5*sin(7.3t)) * level"}`; evaluated at the
  file time it gives 4.33794. `fireLevel(1, t)` burns for the
  whole shot.
- `post` = `gradeOf(G.jungle, dusk)`: bloom 0.16
  (threshold 0.85 / radius 0.6, the loader defaults), bokeh
  aperture 0.0012 / maxblur 0.007, grade saturation 1.08 /
  warm 0.06 / exposure 0.97 (the dusk preset's grade over
  G.jungle), vignette 0.24 / grain 0.012. `grade.time = 0`
  per ruling R3 (no source path sets the grade's time
  uniform; film.rs pins it to 0). Note the s06/s07 files
  predate R3 and record the film t there — flagged for the
  reconciliation pass, not silently harmonised here.

### Camera (push-in 48°)
- `camera.keys`: lt 0 eye [0.45, 1.95, 5.0]; lt 4.872 eye
  [0.3, 1.7, 3.6]; target [0.45, 1.15, −1.1] constant;
  fovY 48 throughout (`shots.ts` S11 shoot from/to/at).
- `shot.cameraTrack = {kind: "camK", hold: 0.1}` — the
  project mixes on `MO.camK(lt, d)`; the loader's keys use
  plain smootherstep per segment (Inexpressible 4).
- Bokeh focus is `|eye − target|` per frame (shoot()
  default): 6.15224 at lt 0 → 4.73445 at lt 4.872. Recorded
  as `post.focusChannel`; `post.bokeh.focus` carries the
  lt-0 value.

### Characters — the instance swap
- S11 stages Onoda TWICE: `jgOnoda` (rifle attached) while
  `t < dropT`, then `jgOnoda2` (rifle-less) from dropT. The
  source comment is explicit: attach3D re-shows its object
  on every `face()`, so the rifle can only leave his hand by
  swapping to the second instance of the same registered
  man. Both file nodes carry the identical registered look
  (body #5C6644, cap #4C5638, shade override #878E6E,
  perspective true) and the dusk `envTint` (additive:
  {sat 0.97, warm 0.06, exposure 0.96, fogColor #F59C6B,
  fogMix 0.08}, film.rs `tint_dusk()`). The swap is
  position-continuous: the walk-in lerp ends exactly at
  Onoda2's fixed position [−0.15, 0, −0.95].
- **No character in S11 sets `camPitch`** — the staging
  never calls `camPitchOf`, so `resolveState` (toon.ts)
  defaults it to 0 for all three nodes for the whole shot
  (the `face()` plane pitch still applies; only the drawing
  compensation stays 0). States record `camPitch: 0`
  explicitly. Unset fields generally resolve to the
  resolveState defaults (look [0, 0], talk 0, headTurn 0,
  squash 0 …) — the file's states write them out.
- **Onoda** (first instance), lt-0 state: position
  [−2.65, 0, −0.45], mood 'determined', armR [16, −6],
  armL [10, 5], lean 1, legs 'walk', phase 0, bob 4
  (walkBob(0, 4)), blink 0 (blinkDirector(3) at film
  50.648 verified against the interval pattern = 0).
  Channels: `visible` window [0, 2.922]; `position`
  segLerp on wk = seg(lt, 0.1, 0.882) — the seg end is
  `max(0.4, wOrder − st − 0.25) = 0.882`, and lib.ts `seg`
  is clamp-LINEAR (no easing — the walk-in is a constant-
  speed approach; the camera's camK supplies the easing);
  `phase` walkPhase (travelled wk·2.5, stride 0.6); `bob`
  walkBob amp 4; `legs` step (wk < 1 → walk); `mood` step
  at lt 1.132 ('awe' from 'order'); `blink` blinkDirector
  seed 3. The rifle rides as the node's `attachments`
  (R2) + `rifleNote`.
- **Onoda (second instance)**, state at lt 0 (invisible
  until lt 2.922; the values the poseTracks hold at 0):
  position [−0.15, 0, −0.95], mood 'awe', armR [34, 0],
  armL [8, 4], lean 1, legs 'stand', blink 0. Channels:
  `visible` window [2.922, null]; `armR` poseTrack
  [[0, 34], [3.472, 7, 0.45]] with the elbow as a second
  poseTrack [[0, 0], [3.472, 3, 0.45]] (the house form
  carries a constant elbow; here the elbow is itself a
  track, recorded as a nested channel object); `mood`
  step at lt 3.772 ('relieved' from dropT + 0.85); `lean`
  STEP [[0, 1], [3.472, 0]] — the source is a ternary on t
  (`t >= dropT + 0.55 ? 0 : 1`), flipping at the same
  instant the arm track lands; `blink`
  blinkDirector+blinkAt (seed 3, beat 'surrendered'
  53.02 — its 0.12 s window ends 0.43 s before this
  instance becomes visible, so that term never renders;
  recorded because the source sets it).
- **Taniguchi** at [1.3, 0, −1.3], perspective false; the
  full registered look (vest #33304A, bowler #2E2A3A, tie
  #7A2E3A, glasses + mustache #C9C9D2 — identical to the
  s03 node). lt-0 state: mood 'determined', armR [12, −8]
  (orderArm at 0), armL [12, 6], lean 0, headTilt 0,
  talk 0, blink 0 (seed 9 verified). Channels: `armR`
  poseTrack [[0, 12], [1.002, 112, 0.32], [3.072, 14, 0.5]]
  elbow −8 — the point lands 0.13 s before 'order' and the
  arm returns 0.7 s after 'surrendered'; `lean` poseTrack
  [[0, 0], [4.072, 7, 0.5]] (the bow, keyed 1.15 s after
  dropT, holds 7 to the cut); `headTilt` derived
  (−0.8 · bow); `talk` (kind "talk", the s03 shape) gated
  to film t ∈ (51.68, 52.53) around 'order'; `blink`
  blinkDirector seed 9.

### The rifle: attachment → ground node
- Before dropT the rifle is Onoda's hand attachment (above).
  From dropT it is the group node `rifle (on ground)`:
  `makeRifle()`'s five parts as children (stock / receiver /
  barrel / front sight / grip swell, with the source
  geometry and materials; steel metalness 0.55 carried as
  an additive material field).
- Creation transform (shots.ts): position [0.62, 0.07,
  −0.1], rotation [0, −0.5, π/2], `visible = false`. The
  node's static transform is its state at FIRST VISIBILITY
  (drop start, u = 0): [0.62, 0.32, −0.1] /
  [0, −0.5, π/2 − 0.1]. Channels: `visible` window
  [2.922, null]; `translate` / `euler` keys sampled every
  0.025 s from the closed form `u = seg(lt, 2.922, 3.222)`,
  `y = lerp(0.32, 0.07, eo(u))`,
  `rz = π/2 − 0.1·(1 − eo(u))` (eo = easeOutCubic,
  lib.ts) — max sampling deviation in y 0.00125 m.
- **Cross-file agreement (checked with seat W17):** the
  drop ends at exactly position [0.62, 0.07, −0.1], euler
  [0, −0.5, π/2] — the creation transform, which S12's
  file pins verbatim (S12's draw re-pins
  `rifleDown.position.y = 0.07; rifleDown.rotation.z =
  Math.PI / 2` every frame). The last drop key (lt 3.222,
  u = 1) reproduces it bit-for-bit.

### Inexpressible in format v2 (declared, not substituted)
1. **The instance swap itself.** Format v2 has no
   visibility construct on character nodes, and no way to
   say "same cast identity, second instance". It is
   carried as two character nodes with additive
   `channels.visible` windows ([0, 2.922] / [2.922, null])
   — a consumer ignoring the windows shows both Onodas for
   the whole shot.
2. **Attachment lifetime.** The v2 `attachments` entry
   (R2) is static — it has no window field, so "the rifle
   leaves the hand at dropT" exists only as the first
   instance's visible window + `rifleNote`. A loader that
   renders the first instance past dropT would keep the
   rifle in his hand alongside the ground node.
3. **Channel evaluation** (all `channels` above): the
   loader renders the static state at `time`; the formulas
   exist as data for the film evaluator (src/film.rs s11).
4. **camK hold in camera keys**: the project mixes the
   push-in on camK(lt, d, hold 0.1); loader keys use plain
   smootherstep per segment. Endpoints/fov exact; the
   mid-shot path differs by the hold law. Recorded in
   `shot.cameraTrack`.
5. **Grade exposure / saturation naming**: files write
   `saturation` + `exposure` (house convention); the
   loader's grade parser reads `sat` and has no exposure
   field — exposure 0.97 is data-only until the format
   grows it.
6. **Scene fog**: no fog field in format v2 (film.rs
   FrameSpec has one). Recorded as top-level `fog`.
7. **Cloud tint**: billboard nodes have no tint field;
   the dusk tint #FFC9B0 is an additive `tint` on the
   three cloud sprite nodes. Note src/film.rs
   `add_clouds` renders clouds untinted [1, 1, 1] — the
   data follows the project source.
8. **Character env tint**: no tint field on the v2
   character node; the dusk envMatch tint is recorded as
   `envTint` on all three character nodes.
9. **Metalness**: v2 materials carry colour + roughness;
   the rifle steel's metalness 0.55 is written as an
   additive material field (the s03/shore precedent) and
   is not consumed by the loader.
10. **Per-node shadow flags**: every makeRifle part has
    castShadow/receiveShadow in the source; the format
    carries no per-node shadow flags.
11. **Sky ground band**: the `sky-gradient` generator
    takes top/horizon only; the dusk preset's skyGnd
    #F59C6B equals its horizon colour, so nothing is
    lost in this shot (the general limit stands, cf. S06).
12. **Caption strips**: chunk ids/timing/fit are data
    (above); the strip images are film assets
    (film/gen_assets.py), not scene-format textures.
13. **Audio cues**: 'inhale' / 'thud' are recorded in
    `shot.cues` as data only; audio does not exist in
    the scene format.

### Divergences flagged for the integrator (film.rs vs source; R4 — source wins)
1. **Onoda2's lean**: src/film.rs s11 eases it with
   poseTrack [[0, 1], [dropT − st + 0.55, 0, 0.45]]; the
   source is a step at t = dropT + 0.55. This file records
   the step.
2. **Walk-in arms**: src/film.rs s11 never sets the first
   instance's arms, leaving the FullState defaults
   [8, 6] / [8, 6]; the source sets armR [16, −6],
   armL [10, 5] for the whole walk-in. This file records
   the source values.
3. **Onoda2's armL**: src/film.rs leaves the default
   [8, 6]; the source sets [8, 4]. Recorded as [8, 4].
4. **Onoda2's blink**: src/film.rs omits the
   `blinkAt(t, [wSurr])` term the source takes the max
   with. It can never render (its window ends before the
   swap), so the omission is benign — flagged for
   transcription completeness.
- Everything else in src/film.rs s11 was cross-checked
  field-by-field against shots.ts (rig_dusk, the fog
  triple, rifle_down_draws' drop law and part geometry,
  Taniguchi's tracks and talk window, the camera and the
  gradeOf post) and agrees.
