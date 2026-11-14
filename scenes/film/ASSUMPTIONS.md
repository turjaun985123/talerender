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
