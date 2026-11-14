# AUDIT-W61 — Third audit: the categories W40/W42 under-sampled

Seat W61, 2026-11-14. Sample: **140 values, 10 per segment × 14 segments —
138 MATCH, 2 MISMATCH.** Both mismatches are the same declared omission:
the sea's metalness (source 0.08) is carried in **neither** SH file
(s02, s03). Everything else in the under-sampled categories — texture
entries, set-node transforms, hemi colours, blink seeds and talk anchors,
clear/sky colours, shadow normalBias, PBR metalness/roughness, caption
fits — recomputes from source exactly.

## Method

- Sample design deliberately avoids W40 (s03/s07/s11, 90 values) and
  W42 (the other 11, 165 values): no camera field, no grade field, no
  key colour/intensity/position, no shadow bias/far/box, no caption
  timing, no character transform was re-sampled. Where a category in
  the brief was already taken for a specific segment (e.g. W40 took
  s07's hemi ground + intensity), a sibling field was substituted and
  the substitution is marked †.
- Recomputed first-hand from `~/faceless-shorts/the-soldier-who-didnt-know/src/`
  (`core/env.ts`, `core/world.ts`, `core/engine.ts`, `core/motion.ts`,
  `core/timing.ts`, `core/props.ts`, `shots.ts`), `data/words.json`,
  `data/lines.json`, and this repo's `assets/captions/manifest.json` +
  `tools/captions/gen_caption_strips.py`. ASSUMPTIONS.md was read only
  after all values were computed (comparison in §5).
- Tolerance (W40/W42 convention): a file value within f32 rounding of
  the f64 recomputation is a MATCH; a value stored at a stated decimal
  precision that rounds back is a MATCH. `Wd()` follows timing.ts
  exactly (first word in the line whose normalised form starts with
  the query). `rnd(i) = frac(sin(i·127.1 + 311.7)·43758.5453)`.

## 1. Mismatches (2)

### M1 — s02 + s03: `nodes[sea].material.metalness` is absent; source builds 0.08

- File (both): `material: { color: '#4E86AC', roughness: 0.35 }` — no
  metalness field, so any consumer default applies (0).
- Source: SH builder, `shots.ts` SH set — sea is
  `mat({ color: '#4E86AC', roughness: 0.35, metalness: 0.08 })`.
  Correct value: **0.08**.
- This is a **declared** omission: ASSUMPTIONS.md (SH set notes) says
  "Sea metalness 0.08 is not a format material field (only `roughness`
  is carried, `src/scenefile.rs:497–500`) — recorded here only."
- Two qualifications the declaration does not state: (a) the corpus
  itself disproves "not a format field" as a general claim — barrel
  bands (0.4, all 13 outdoor files), the s10 brass rail (0.6) and the
  s05a plane (0.25) all carry `metalness` in their material objects;
  the sea is the only PBR node whose value was dropped rather than
  carried. (b) Per W47 the loader does not consume metalness anywhere
  today, so the render impact is **latent, not live**: the value is
  unrecoverable from the scene data if a future loader starts
  consuming it. Fix is one field in two files, or a scenefile.rs
  material-field ruling from the lead.

## 2. Per-segment tables

### S01 — JG noon

| # | path | file | recomputed | verdict |
|---|------|------|-----------|---------|
| 1 | `textures[grass].generator` + `.base` | grass / `#5B9A48` | JG ground `T.grass('#5B9A48', [48,48])` (shots.ts JG set) | MATCH |
| 2 | `nodes['grass ground'].geometry.uvRepeat` | 48.0 | the same call's repeat [48, 48], carried as the uniform 48 | MATCH |
| 3 | `nodes['fire stone 1'].transform[0].translate` | [−1.0388730764389038, 0.05, −2.1388731002807617] | fire ring i=1: (−1.35 + cos(π/4)·0.44, 0.05, −2.45 + sin(π/4)·0.44) = [−1.0388730163, 0.05, −2.1388730163]; Δ6.0e-8 = f32 chain | MATCH |
| 4 | `nodes['barrel']→band.material.metalness` | 0.4 | barrel() bands `mat({color '#3A3A3F', metalness 0.4})` at y 0.12/0.68 (world.ts) | MATCH |
| 5 | `lights.hemi.sky` † (W42 took ground for s01) | `#CFE6FF` | noon preset hemiSky (env.ts), POST-atmosphere per R1 | MATCH |
| 6 | `lights.key.normalBias` | 0.02 | lightRig default 0.02 (engine.ts); sunlight() overrides far/bias only | MATCH |
| 7 | `nodes[Onoda].channels.blink.directorSeed` | 3 | blinkOf map: onoda 3 (shots.ts) | MATCH |
| 8 | `nodes[Onoda].channels.blinkAt` ('years', line 0) | 4.2 | Wd('years', 0) = 4.2 (words.json) | MATCH |
| 9 | `lights.rim.position` † (file has no `clear` field) | [−3, 3, −3] | lightRig rim position hardcoded (−3, 3, −3) (engine.ts) | MATCH |
| 10 | captions fit † (s01 captions carry no `fit` field) | — (absent) | substitute, manifest-internal: chunk cap-000 fit 1.0 = min(1, 960/195.36) per gen rule | MATCH |

### S02 — SH noon

| # | path | file | recomputed | verdict |
|---|------|------|-----------|---------|
| 1 | `textures[grass-hills].generator` + `.base` + `.repeat` | grass / `#5E8F52` / [6, 3] | SH `hills({z:55, col:'#5E8F52'})` → `T.grass(col, [6,3])` (world.ts hills()) | MATCH |
| 2 | `textures[planks-crate].generator` + `.base` + `.repeat` | planks / `#B07A48` / [0.5, 0.5] | crate() map `T.planks('#B07A48', [0.5,0.5])` (world.ts) | MATCH |
| 3 | `nodes['crate 0.6'].transform[0].translate` | [2.7, 0, −1.9] | SH set: crate(0.6) at (2.7, 0, −1.9) (shots.ts) | MATCH |
| 4 | `nodes['sea'].material.metalness` | **absent** | 0.08 (SH sea mat) | **MISMATCH — M1** |
| 5 | `lights.hemi.ground` | `#C48A5A` | noon preset hemiGround; overrides SH sunlight's `#46603A` (R1) | MATCH |
| 6 | `lights.key.normalBias` | 0.02 | lightRig 0.02; sunlight() leaves it | MATCH |
| 7 | `nodes[onoda].channels.blink.director.seed` | 3 | blinkOf onoda 3 | MATCH |
| 8 | `sky.top` † (no blinkAt/talk exists in S02) | `#6E9CC0` | SH `sky({top:'#6E9CC0', horizon:'#E4EEDC'})`; shot keeps the set sky | MATCH |
| 9 | `clear` | `#E4EEDC` | SH sky horizon `#E4EEDC` | MATCH |
| 10 | `captions[cap-013].fit` | 1.0 | manifest cap-013 fit 1.0; rule min(1, 960/544.7) = 1.0 | MATCH |

### S03 — SH noon

| # | path | file | recomputed | verdict |
|---|------|------|-----------|---------|
| 1 | `textures[grass-hills]` (generator/base/repeat) | grass / `#5E8F52` / [6, 3] | as S02 row 1 | MATCH |
| 2 | `textures[planks-crate]` (generator/base/repeat) | planks / `#B07A48` / [0.5, 0.5] | as S02 row 2 | MATCH |
| 3 | `nodes['crate 0.6'].transform[0].translate` | [2.7, 0, −1.9] | SH crate(0.6) at (2.7, 0, −1.9) | MATCH |
| 4 | `nodes['sea (plane stand-in)'].material.metalness` | **absent** | 0.08 | **MISMATCH — M1** |
| 5 | `lights.hemi.ground` † (W40 took intensity for s03) | `#C48A5A` | noon preset hemiGround (R1) | MATCH |
| 6 | `lights.key.normalBias` † (W40 took bias/far/box for s03) | 0.02 | lightRig 0.02 | MATCH |
| 7 | `nodes[onoda].channels.blink.director.seed` † (W40 took blink pattern fields) | 3 | blinkOf onoda 3 | MATCH |
| 8 | `nodes[suzuki].channels.talk` window end (lt) | 2.309 | Wd('surrender', 2) 15.02 − cut(2) 13.061 + 0.35 = 2.309 (shots.ts S03) | MATCH |
| 9 | `lights.rim.position` † (sky/clear family; W40 took rim intensity/colour) | [−3, 3, −3] | lightRig (−3, 3, −3) | MATCH |
| 10 | captions fit † (s03 captions carry no `fit` field) | — (absent) | substitute, manifest-internal: owned chunk cap-018 fit 1.0 = min(1, 960/451.8) | MATCH |

### S04 — JG noon

| # | path | file | recomputed | verdict |
|---|------|------|-----------|---------|
| 1 | `textures[grass]` (generator/base) | grass / `#5B9A48` | as S01 row 1 | MATCH |
| 2 | `textures[grass-hills]` (generator/base/repeat) | grass / `#5E8F52` / [6, 3] | JG `hills({z:−75, col:'#5E8F52'})` → [6, 3] | MATCH |
| 3 | `nodes['fire stone 1'].transform[0].translate` | [−1.0388730764389038, 0.05, −2.1388731002807617] | as S01 row 3 | MATCH |
| 4 | `nodes['barrel']→band.material.metalness` | 0.4 | as S01 row 4 | MATCH |
| 5 | `lights.hemi.ground` | `#C48A5A` | noon preset (R1) | MATCH |
| 6 | `lights.key.normalBias` | 0.02 | lightRig 0.02 | MATCH |
| 7 | `nodes[compA].channels.blink.director.seed` | 7 | blinkOf compA 7 | MATCH |
| 8 | `nodes[onoda].channels.blinkAt.beats[0]` | 20.700000762939453 | f32(Wd('message', 3)) = f32(20.7); S04 blink = max(blinkOf, blinkAt(t, [Wd('message',3)])) | MATCH |
| 9 | `clear` | `#D8E8DA` | JG sky horizon `#D8E8DA` | MATCH |
| 10 | `captions[index 21].fit` (cap-021) | 1.0 | manifest 1.0; rule min(1, 960/351.0) = 1.0 | MATCH |

### S05a — JG noon (no characters in this shot)

| # | path | file | recomputed | verdict |
|---|------|------|-----------|---------|
| 1 | `textures[grass]` (generator/base) | grass / `#5B9A48` | as S01 row 1 | MATCH |
| 2 | `textures[grass-hills]` (generator/base/repeat) | grass / `#5E8F52` / [6, 3] | as S04 row 2 | MATCH |
| 3 | `nodes['leaflet 0'].channels.translate.keys[0].value` | [−7.928479, 9.4, −12.042065] | ft=0 state: sx = lerp(−13, 11, seg(0.7, 0.05, d−0.05)) with d = 3.176 → −7.9284785; z = −13 + (rnd(0)−0.5)·3 = −12.0420648 (rnd(0) = 0.81931173) | MATCH |
| 4 | `nodes['leaflet 0'].material.roughness` | 0.9 | leaflet `MeshStandardMaterial({color '#F2EDE0', roughness 0.9, DoubleSide})` (shots.ts) | MATCH |
| 5 | `lights.hemi.ground` | `#C48A5A` | noon preset (R1) | MATCH |
| 6 | `lights.key.normalBias` | 0.02 | lightRig 0.02 | MATCH |
| 7 | `nodes['plane'].transform[1].rotate_y` † (no characters → no blink seed) | 1.5707963267948966 | planeG.rotation.y = π/2 exactly (f64, not f32) | MATCH |
| 8 | `nodes['leaflet 0'].transform[1].euler_xyz[0]` † (no talk anchor) | 39.0558 | rotation.x = t·1.7 at the file's film t 22.974 → 39.0558 (raw source value, unnormalised) | MATCH |
| 9 | `clear` | `#D8E8DA` | JG horizon | MATCH |
| 10 | `captions[cap_026].fit` | 0.92269 | manifest 0.9227; rule 960/1040.44 = 0.922687–0.922694 — file/manifest/rule agree within 4e-6 (representation precision: 5dp vs 4dp vs 2dp rawW) | MATCH |

### S05b — JG noon

| # | path | file | recomputed | verdict |
|---|------|------|-----------|---------|
| 1 | `textures[grass]` | grass / `#5B9A48` | as S01 row 1 | MATCH |
| 2 | `textures[grass-hills]` | grass / `#5E8F52` / [6, 3] | as S04 row 2 | MATCH |
| 3 | `nodes['fire stone 1'].transform[0].translate` | as S01 | as S01 row 3 | MATCH |
| 4 | `nodes['barrel']→band.material.metalness` | 0.4 | as S01 row 4 | MATCH |
| 5 | `lights.hemi.ground` | `#C48A5A` | noon preset (R1) | MATCH |
| 6 | `lights.key.normalBias` | 0.02 | lightRig 0.02 | MATCH |
| 7 | `nodes[onoda].channels.blink.director.seed` | 3 | blinkOf onoda 3 | MATCH |
| 8 | `nodes[onoda].channels.armL.poseTrack.keys[1][0]` † (no blinkAt/talk in S05b) | 1.590000033378601 | f32(dropT − st): dropT = Wd('trick', 4) + 0.55 = 26.15, st = Wd('thought', 4) − 0.12 = 24.56 → 1.59 | MATCH |
| 9 | `clear` | `#D8E8DA` | JG horizon | MATCH |
| 10 | `captions[cap_030].fit` | 1.0 | manifest 1.0; rule min(1, 960/626.89) = 1.0 | MATCH |

### S06 — JG fog

| # | path | file | recomputed | verdict |
|---|------|------|-----------|---------|
| 1 | `textures[grass]` | grass / `#5B9A48` | as S01 row 1 | MATCH |
| 2 | `textures[grass-hills]` | grass / `#5E8F52` / [6, 3] | as S04 row 2 | MATCH |
| 3 | `nodes['fire stone 1'].transform[0].translate` | as S01 | as S01 row 3 | MATCH |
| 4 | `nodes['barrel']→band.material.metalness` | 0.4 | as S01 row 4 | MATCH |
| 5 | `lights.hemi.ground` | `#B8BCBE` | fog preset hemiGround (env.ts) | MATCH |
| 6 | `lights.key.normalBias` | 0.02 | lightRig 0.02 (fog preset rig is the JG sunlight rig) | MATCH |
| 7 | `nodes[compB].channels.blink.director.seed` | 11 | blinkOf compB 11 | MATCH |
| 8 | `sky.top` † (no blinkAt/talk in S06) | `#C8CDD0` | fog preset skyTop (env.ts) | MATCH |
| 9 | `clear` | `#E6E9EA` | fog preset skyHor | MATCH |
| 10 | `captions[index 32].fit` (cap-032) | 1.0 | manifest 1.0; rule min(1, 960/481.72) = 1.0 | MATCH |

### S07 — JG golden

| # | path | file | recomputed | verdict |
|---|------|------|-----------|---------|
| 1 | `textures[grass]` | grass / `#5B9A48` | as S01 row 1 | MATCH |
| 2 | `textures[grass-hills]` | grass / `#5E8F52` / [6, 3] | as S04 row 2 | MATCH |
| 3 | `nodes[palm]→'frond 0'.transform[0].translate` | [0.3894183337688446, 4.449999809265137, 0.9210609793663025] | palm() i=0: a = 0.4; (sin a·1.0, topY 4.35 + 0.1 − 0, cos a·1.0) = (0.38941834, 4.45, 0.92106099) → f32 spellings | MATCH |
| 4 | `nodes[palm]→'frond 0'.material.roughness` (+ colour) | 0.9 (`#4C8A3A`) | palm leaf2 `WD.mat({color '#4C8A3A', roughness 0.9})`; i=0 even → leaf2 | MATCH |
| 5 | `lights.hemi.sky` † (W40 took ground + intensity for s07) | `#FFE0B0` | golden preset hemiSky | MATCH |
| 6 | `lights.key.normalBias` | 0.02 | lightRig 0.02 | MATCH |
| 7 | `nodes[onoda].channels.blink.director.seed` | 3 | blinkOf onoda 3 | MATCH |
| 8 | `sky.top` † (no blinkAt/talk in S07) | `#4A90D9` | golden preset skyTop | MATCH |
| 9 | `clear` | `#FFD98A` | golden preset skyHor | MATCH |
| 10 | `captions[cap_040].fit` | 0.96974 | manifest 0.9697; rule 960/989.95 = 0.969746 — agreement within 6e-6 (representation precision) | MATCH |

### S08 — JG noon

| # | path | file | recomputed | verdict |
|---|------|------|-----------|---------|
| 1 | `textures[grass]` | grass / `#5B9A48` | as S01 row 1 | MATCH |
| 2 | `textures[grass-hills]` | grass / `#5E8F52` / [6, 3] | as S04 row 2 | MATCH |
| 3 | `nodes['fire stone 1'].transform[0].translate` | as S01 | as S01 row 3 | MATCH |
| 4 | `nodes['barrel']→band.material.metalness` | 0.4 | as S01 row 4 | MATCH |
| 5 | `lights.hemi.ground` | `#C48A5A` | noon preset (R1) | MATCH |
| 6 | `lights.key.normalBias` | 0.02 | lightRig 0.02 | MATCH |
| 7 | `nodes[suzuki].channels.blink.director.seed` | 5 | blinkOf suzuki 5 | MATCH |
| 8 | `nodes[suzuki].channels.blinkAt[0]` | 38.47999954223633 | f32(Wd('found', 7)) = f32(38.48) | MATCH |
| 9 | `clear` | `#D8E8DA` | JG horizon | MATCH |
| 10 | `captions[cap_042].fit` | 1.0 | manifest 1.0; rule min(1, 960/336.66) = 1.0 | MATCH |

### S08b — JG noon

| # | path | file | recomputed | verdict |
|---|------|------|-----------|---------|
| 1 | `textures[grass]` | grass / `#5B9A48` | as S01 row 1 | MATCH |
| 2 | `textures[grass-hills]` | grass / `#5E8F52` / [6, 3] | as S04 row 2 | MATCH |
| 3 | `nodes['fire stone 1'].transform[0].translate` | as S01 | as S01 row 3 | MATCH |
| 4 | `nodes['barrel']→band.material.metalness` | 0.4 | as S01 row 4 | MATCH |
| 5 | `lights.hemi.ground` | `#C48A5A` | noon preset (R1) | MATCH |
| 6 | `lights.key.normalBias` | 0.02 | lightRig 0.02 | MATCH |
| 7 | `nodes[Onoda].channels.blink.director.seed` | 3 | blinkOf onoda 3 | MATCH |
| 8 | `nodes['fire stone 0'].geometry.radius` † (S08b has no blinkAt/talk; the source has none either) | 0.1227724701166153 | 0.09 + rnd(0)·0.04 = 0.1227724692 (rnd(0) = 0.81931173); Δ9.1e-10 | MATCH |
| 9 | `lights.rim.position` † (file has no `clear` field) | [−3, 3, −3] | lightRig (−3, 3, −3) | MATCH |
| 10 | `captions[cap_047].fit` (+ `.width`) | 1.0 (378.27) | manifest fit 1.0, rawW 378.27 = file width; rule = 1.0 | MATCH |

### S09 — JG noon

| # | path | file | recomputed | verdict |
|---|------|------|-----------|---------|
| 1 | `textures[grass]` | grass / `#5B9A48` | as S01 row 1 | MATCH |
| 2 | `textures[grass-hills]` | grass / `#5E8F52` / [6, 3] | as S04 row 2 | MATCH |
| 3 | `nodes['fire stone 1'].transform[0].translate` | as S01 | as S01 row 3 | MATCH |
| 4 | `nodes['barrel']→band.material.metalness` | 0.4 | as S01 row 4 | MATCH |
| 5 | `lights.hemi.ground` | `#C48A5A` | noon preset (R1) | MATCH |
| 6 | `lights.key.normalBias` | 0.02 | lightRig 0.02 | MATCH |
| 7 | `nodes[suzuki].channels.blink.director.seed` | 5 | blinkOf suzuki 5 | MATCH |
| 8 | `nodes[onoda].channels.blinkAt` ('commander' beat) | 44.040000915527344 | f32(Wd('commander', 8)) = f32(44.04); channel law cites blinkDirector(3) + blinkAt(t, [Wd('commander',8)]) | MATCH |
| 9 | `clear` | `#D8E8DA` | JG horizon | MATCH |
| 10 | `captions[cap_048].fit` (+ `.width`) | 1.0 (324.02) | manifest fit 1.0, rawW 324.02 = file width; rule = 1.0 | MATCH |

### S10 — BK interior

| # | path | file | recomputed | verdict |
|---|------|------|-----------|---------|
| 1 | `textures[wallpaper-back].base` + `.repeat` | `#5A4632` / [1.7142857, 3.0] | BK wall override `#5A4632`; wp(len) repeat [len/1.4, 4.2/1.4] with len = (8−3.2)/2 = 2.4 → [1.7142857, 3.0] (world.ts interior()) | MATCH |
| 2 | `textures[wallpaper-ceiling].motif` (+ `.repeat`) | `#413427` ([6.6666665, 7.5]) | shade('#3A2E22', 1.25) in linear space = #413427 (recomputed channel-wise); repeat [8/1.2, 9/1.2] | MATCH |
| 3 | `nodes['shelf unit 1 (seed 1)'].transform[0].translate` | [−1.7999999523162842, 0, −6.550000190734863] | BK places shelfUnit(3.4, 2.5, 1) at (−1.8, 0, −6.55) → f32 spellings | MATCH |
| 4 | `nodes['counter']→'counter brass rail'.material.metalness` | 0.6 | interior() brass `mat({color '#C9A13B', metalness 0.6, roughness 0.3})` (file roughness also 0.3) | MATCH |
| 5 | `lights.hemi.ground` | `#5A3A2A` | interior() HemisphereLight ground `#5A3A2A` (no atmosphere rig in BK) | MATCH |
| 6 | `lights.key.normalBias` | 0 | interior() sun never sets normalBias → three.js default 0 (contrast the outdoor rig's 0.02 — the file gets the subtle case right) | MATCH |
| 7 | `nodes[Taniguchi].channels.blink.law` seed | blinkDirector(9) | blinkOf taniguchi 9 | MATCH |
| 8 | `nodes[Taniguchi].channels.blinkAt` ('bookseller' beat) | 48.15999984741211 | f32(Wd('bookseller', 9)) = f32(48.16) | MATCH |
| 9 | `clear` | `#1A1418` | interior() `scene.background = #1A1418` | MATCH |
| 10 | `captions[cap_052].fit` (+ `.width`) | 1.0 (682.92) | manifest fit 1.0, rawW 682.92 = file width; rule = 1.0 | MATCH |

### S11 — JG dusk

| # | path | file | recomputed | verdict |
|---|------|------|-----------|---------|
| 1 | `textures[grass]` | grass / `#5B9A48` | as S01 row 1 | MATCH |
| 2 | `textures[grass-hills]` | grass / `#5E8F52` / [6, 3] | as S04 row 2 | MATCH |
| 3 | `nodes['fire stone 1'].transform[0].translate` | as S01 | as S01 row 3 | MATCH |
| 4 | `nodes['barrel']→band.material.metalness` | 0.4 | as S01 row 4 | MATCH |
| 5 | `lights.hemi.ground` † (W40 took sky + intensity for s11) | `#7A4A3A` | dusk preset hemiGround | MATCH |
| 6 | `lights.key.normalBias` † (W40 took far for s11) | 0.02 | lightRig 0.02 | MATCH |
| 7 | `nodes[onoda].channels.blink.director.seed` | 3 | blinkOf onoda 3 | MATCH |
| 8 | `nodes[suzuki].channels.talk` window (film t) | 51.68..52.53 | Wd('order', 10) = 51.78; window = 51.78 − 0.1 .. 51.78 + 0.75 (shots.ts S11); formula string is talk() verbatim | MATCH |
| 9 | `clear` | `#F59C6B` | dusk preset skyHor | MATCH |
| 10 | `captions[cap_057].fit` (+ `.width`) | 1.0 (420.47) | manifest fit 1.0, rawW 420.47 = file width; rule = 1.0 | MATCH |

### S12 — JG dusk

| # | path | file | recomputed | verdict |
|---|------|------|-----------|---------|
| 1 | `textures[grass]` | grass / `#5B9A48` | as S01 row 1 | MATCH |
| 2 | `textures[grass-hills]` | grass / `#5E8F52` / [6, 3] | as S04 row 2 | MATCH |
| 3 | `nodes['fire stone 1'].transform[0].translate` | as S01 | as S01 row 3 | MATCH |
| 4 | `nodes['barrel']→band.material.metalness` | 0.4 | as S01 row 4 | MATCH |
| 5 | `lights.hemi.ground` | `#7A4A3A` | dusk preset hemiGround | MATCH |
| 6 | `lights.key.normalBias` | 0.02 | lightRig 0.02 | MATCH |
| 7 | `nodes[Taniguchi].channels.blink.law` seed | blinkDirector(9) | blinkOf taniguchi 9 | MATCH |
| 8 | `nodes[Onoda].channels.blinkAt` ('loyal' beat) | 58.119998931884766 | f32(Wd('loyal', 11)) = f32(58.12) (law string also carries the plain 58.12) | MATCH |
| 9 | `clear` | `#F59C6B` | dusk preset skyHor | MATCH |
| 10 | `captions[cap_061].fit` (+ `.width`) | 1.0 (737.27) | manifest fit 1.0, rawW 737.27 = file width; rule = 1.0 | MATCH |

## 3. Substitutions register

- **s01, s08b — no `clear` field exists** in either file (an absence,
  not a wrong value; the other 12 files carry it). Substituted
  `lights.rim.position`, unsampled by W40/W42 in any segment.
- **s01, s03 — caption entries carry no `fit` field.** The fit sample
  was taken manifest-internally on an owned chunk (cap-000 / cap-018):
  the manifest's own fit equals the generator rule
  `min(1, 960/rawW)` exactly. The absence itself is a schema variance
  between authoring seats (s02/s04/s06 key captions by `index` only;
  s07–s12 carry id+width+fit+strip), already covered by W55's census.
- **s05a — no characters** (plane/leaflets shot): blink-seed and
  talk-anchor categories substituted with the plane's rotation.y and
  leaflet 0's rotation channel value at the file's instant.
- **s02, s05b, s06, s07, s08b — no blinkAt/talk anchor exists** in the
  source staging for these segments (verified in shots.ts, not just
  the files): substituted sky.top (s02/s06/s07), the armL drop-timing
  key (s05b), and fire stone 0's radius (s08b).

## 4. Verified beyond the 10-per-segment sample

- **Fire stone 0 radius** is identical in all 11 JG files and matches
  rnd(0) to Δ9.1e-10 (used as a sample only in s08b).
- **Fire stone 1 radius**: file 0.09418729692697525 vs f64 recompute
  0.09418729702039855 (Δ9.3e-8, all 11 JG files). This is the rnd()
  chain's known sensitivity — sin of a ~439-radian argument, amplified
  by 43758.5453 — plus f32 storage; the position Δ is 6.0e-8. Both
  are orders below one pixel at any focal length in the film; recorded
  here so the number is on record, not as a mismatch.
- **s05a leaflet 0 static transform** at the file's instant also
  reproduces from the staging formula at lt = 1.59 (file time 22.974 −
  at 21.384): recomputed (−6.5683945, 6.3543674, −10.6601293) vs file
  (−6.568394, 6.354367, −10.660129) at 6dp storage; and the plane's
  px = −0.9843953 vs file −0.984395. The file's shot at/duration
  (21.384 / 3.176) are exactly cut(4) and Wd('thought',4)−0.12 − cut(4).
- **s10 texture cross-checks** (all MATCH against interior()/props.ts):
  planks-floor `#7A5230` [3.2, 3.6] with map rotation f32(π/2);
  wainscot-window/back/side [1.6, 0.5] / [1.0, 0.5] / [4.5, 0.5] (the
  wain(len) call values 3.2 / 2 / 9); planks-counter `#7A3E24` [1, 2];
  felt-counter `#1F5A45`; planks-desk `#6B3E26` [1, 1]; felt-desk
  `#2E5A44`; wallpaper-side repeat [6.4285712, 3.0] = wp(9);
  wallpaper-back-top [2.2857144, 3.0] = wp(3.2).
- **Caption fits**: only two chunks in the film have fit < 1
  (cap_026, cap_040); both agree file ↔ manifest ↔ generator rule
  within 6e-6. All other sampled fits are exactly 1.0 on both sides.

## 5. ASSUMPTIONS.md comparison (read after computing)

- The sea omission (M1) **is** declared in the SH set notes, quoted in
  §1. The same paragraph claims barrel band metalness 0.4 "is not a
  format field — recorded here only": that half is **stale** — the
  s02/s03 files (and all other outdoor files) do carry 0.4 in the
  band material objects, verified first-hand in this audit. The
  declaration's remaining list (material metalness: sea 0.08, barrel
  bands 0.4, rifle steel 0.55) is best read as "not consumed by the
  loader", which matches W47's census.
- ASSUMPTIONS' S01 section cites normalBias 0.02 to `core/engine.ts:63`,
  agreeing with this audit's independent read of lightRig.
- Nothing in ASSUMPTIONS contradicts any MATCH above; no sampled value
  in this audit rests on an ASSUMPTIONS derivation — all were
  recomputed from the TypeScript sources, words.json and the caption
  generator rule.

## Totals

**140 sampled — 138 MATCH, 2 MISMATCH (M1, the same omission in two
files).** The under-sampled categories are otherwise clean, including
the two easiest places to have gone wrong: the interior rig's
normalBias 0 (s10) and the POST-atmosphere hemi grounds everywhere
(the sets' own sunlight grounds survive nowhere, correctly, per R1).
