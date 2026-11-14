# Atmospheres — provenance and dynamics

Data file: `scenes/atmospheres.json` (format `talerender-atmospheres`, v1).
Source project: `~/faceless-shorts/the-soldier-who-didnt-know` — "THE SOLDIER
WHO DIDN'T KNOW THE WAR WAS OVER". All citations are `file:line` in that
project's `src/` unless a talerender file is named.

**Number law.** Every number in the JSON is the exact decimal literal in the
TypeScript source; it narrows to the identical f32 the source produces at
runtime. Nothing is a sampled approximation. Quantities the source *computes*
(fire intensity, bokeh focus, merged grades) are given as formulas (§9), not
frozen numbers.

## 1 · How an atmosphere reaches a frame (the merge that matters)

There are **three** layers, and a native port that merges them in the wrong
order will not match:

1. **The set's own rig** (`shots.ts` set builders + `core/world.ts`),
   constructed once. Recorded in the JSON as `baseRigs`.
2. **`applyAtmosphere(scene, name, { rig })`** (`core/env.ts:164`), called
   inside *every* shot's draw. It mutates fog, sky dome, cloud tint, the
   rig (key colour/intensity/position, rim intensity, hemisphere), and the
   character tint of every `envMatch` Toon. Its `grade` is **advisory** —
   it never touches the renderer (env.ts:9-11, 215).
3. **The shot's render look**: the shot-grade constant `G.jungle / G.shore /
   G.shop` (`shots.ts:134-138`), for fog/golden/dusk shots merged with the
   preset grade by `gradeOf` (`shots.ts:139`), plus literal per-shot
   overrides. Only this layer reaches `E.render`.

`filmLooks` in the JSON is the fully-merged result of the three layers per
shot group — the set the native engine must reproduce. The sections below
give, per atmosphere: the preset values, where they came from, which shots
use them, and what the merge does to them.

## 2 · Engine and post chain (shared by every shot)

Source: `core/engine.ts`.

| Item | Value | Cite |
|---|---|---|
| Tone mapping | `NeutralToneMapping`; exposure = `look.exposure ?? 1.0` per render | engine.ts:34, 57 |
| Output colour space | `SRGBColorSpace` | engine.ts:35 |
| Composer target | HalfFloat, MSAA samples = QS `aa` (default 4, allowed 0/2/4/8) | engine.ts:16, 33 |
| Environment map | PMREM of `RoomEnvironment`, blur 0.04; one map shared by all sets. Sets scale it: JG/SH `environmentIntensity 0.25`, BK `0.35` | engine.ts:37; world.ts:173, 294 |
| Shadows | enabled, `PCFSoftShadowMap`, `autoUpdate false`, `needsUpdate = true` once per render; map size = QS `shadow` (default 2048, allowed 512/1024/2048/4096) | engine.ts:35-36, 51, 16 |
| Bloom | `UnrealBloomPass` constructed (strength 0.35, radius 0.6, threshold 0.85); per render: strength = `look.bloom ?? 0.3`, threshold = `look.bloomThreshold ?? 0.85`, radius forced 0.6 | engine.ts:42, 56 |
| Bokeh | constructed (focus 3, aperture 0.002, maxblur 0.008); per render: focus = `look.focus ?? 3`, aperture = `look.aperture ?? 0.0015`, maxblur = `look.maxblur ?? 0.006`; enabled iff effective aperture > 0. Shots **always** pass focus explicitly — see §9.4 | engine.ts:41, 54-55 |
| GRADE pass | ShaderPass, authored uniform defaults uVig 0.35 / uGrain 0.025 / uWarm 0.04 / uSat 1.12 / uTime 0 — **all overwritten every render**: uVig = `look.vignette ?? 0.18`, uGrain = `look.grain ?? 0.008`, uSat = `look.sat ?? 1.12`, uWarm = `look.warm ?? 0.02`, uTime = `look.t ?? 0` | engine.ts:17-24, 58 |
| Pass order | Render → Bokeh → Bloom → Output → GRADE | engine.ts:40-44 |

The **generic light-rig factory** (`engine.ts:66-73`, `lightRig`) defaults —
key `#FFE2C2` ×3.2 at [2.5, 4, 3], rim `#BFD8FF` ×2.2 at [-3, 3, -3], hemi
`#FFF4E6`/`#8C6E55` ×0.9, shadowBox 3, target [0, 1, 0], shadow bias −0.0004,
normalBias 0.02, radius 6, shadow camera near 0.1 / far 20 — are *not* what
the film renders with: `WD.sunlight` re-parameterises it per set (rim
`#FFFFFF` ×0.4, shadow bias −0.0006, shadow far 80, environmentIntensity
0.25) and `shots.ts` passes its own values on top. The JSON records the
factory only under `engine.lightRigFactory` so the deltas are visible.

## 3 · The presets (verbatim)

Source: `core/env.ts:62-141` (`ATMOSPHERES`). Fields: `skyTop/skyHor/skyGnd`
(dome uniforms), `cloudTint` (Sprite material colour), `fog` (THREE.Fog
colour/near/far or null), `keyCol/keyI/keyPos?` (key DirectionalLight),
`rimI` (rim intensity — the rim's *colour* is never in a preset),
`hemiSky/hemiGround/hemiI`, `grade { sat, warm, exposure }`, and `tint`
(the EnvTint fed to `Toon.matchEnv`: sat/warm/exposure multipliers plus a
fog-colour pull, see §9.2). `keyPos` exists only on dawn, golden, dusk,
night; where absent, applyAtmosphere leaves the set's key position in
place (env.ts:190). Design rule stated in the source (env.ts:58-61):
fog colour === horizon colour, and `tint.fogColor` follows it scaled by
`tint.fogMix`.

The film uses **noon, fog, golden, dusk**. dawn, night and storm are
transcribed in the JSON (`presets.*`, `usedByFilm: false`) for completeness
but no shot applies them.

## 4 · noon — S01, S02, S03, S04, S05a, S05b, S08, S08b, S09, S10

Preset: env.ts:76-85. Identity by design — the header at env.ts:80-84
states these are exactly the values the sets/engine already ship, so
adopting noon changes no pixel.

- **Application**: JG shots call `noonJG()` = `applyAtmosphere(JG.scene,
  'noon', { rig: JG.sun, sky: false })` (`shots.ts:292`); SH shots call it
  directly with `sky: false` (`shots.ts:332, 345`); BK calls
  `applyAtmosphere(BK.scene, 'noon')` — **no rig, sky not false**
  (`shots.ts:236`).
- **Sky**: because JG/SH pass `sky: false`, the dome keeps the **set's**
  colours, not the preset's (`#3D8EE0`/`#BFE4F7` never reach these domes):
  JG top `#6E9CC0`, horizon `#D8E8DA` (`shots.ts:144`); SH top `#6E9CC0`,
  horizon `#E4EEDC` (`shots.ts:199`); ground = the `sky()` factory default
  `#BFE4F7` in both (world.ts:149); clouds keep their build tint
  `#EFF4EA` (shots.ts:147, 202). BK has no dome; its scene background
  stays `#1A1418` (world.ts:293).
- **Fog**: null everywhere (preset fog is null; env.ts:188 assigns null).
- **Rig after application** (JG; SH identical except key position
  [-8, 11, 7] and key target [0, 0, -4]): key `#FFF0D6` ×2.6 — note this
  *overrides* the as-built sunlight colour `#FFEFD2` ×2.4 (`shots.ts:145`)
  — position unchanged [7, 12, 5] (no keyPos in the preset), target
  [0, 0, -2] (`shots.ts:146`); rim `#FFFFFF` ×0.4 at [-3, 3, -3]
  (rim colour/position from `sunlight`, world.ts:174; preset sets only
  rimI 0.4, which equals the as-built value); hemisphere
  `#CFE6FF`/`#C48A5A` ×1.25 — overriding the as-built
  `#DCECD2`/`#46603A` ×1.15; shadow: map 2048², ortho box ±17, near 0.1,
  far 80, bias −0.0006, normalBias 0.02, radius 6 (engine.ts:68 +
  world.ts:175); `environmentIntensity` 0.25 (world.ts:173).
- **Character tint**: the identity tint (sat 1, warm 0, exposure 1,
  fogMix 0) — `matchEnv` evaluates to exactly (1, 1, 1); see §9.2.
- **Grade**: the preset grade (sat 1.12, warm 0.02, exposure 1) is **not
  used** by any noon shot. JG renders `G.jungle` verbatim (sat 0.8,
  warm 0.04, vignette 0.24, grain 0.012, bloom 0.16, aperture 0.0012,
  maxblur 0.007, exposure → 1.0 default); SH renders `G.shore` (sat 0.84,
  warm 0.05, vignette 0.22); S02 additionally overrides aperture to
  0.0007 (`shots.ts:340`).
- **BK (S10)** is graded `G.shop` (sat 0.78, warm 0.06, vignette 0.26,
  grain 0.012, bloom 0.3, aperture 0.001, maxblur 0.008) over the
  interior's untouched rig — §7.

## 5 · fog — S06 only (JG)

Preset: env.ts:131-141. Applied at `shots.ts:409`:
`p = applyAtmosphere(JG.scene, 'fog', { rig: JG.sun })` — **sky is not
false**, so the dome is re-dressed to top `#C8CDD0` / horizon `#E6E9EA` /
ground `#D8DCDE` and the clouds take tint `#EDEFEF`.

- **Fog — the shot overrides the preset.** applyAtmosphere first assigns
  the preset fog `{ #E6E9EA, near 3, far 38 }`; the very next statement in
  the same draw replaces it: `JG.scene.fog = new THREE.Fog(0xC9CFC2,
  4.5, 20)` (`shots.ts:409`). **Effective: colour `#C9CFC2`, near 4.5,
  far 20** — the values INVENTORY.md names. three.js `Fog` is linear:
  factor = clamp((far − depth) / (far − near), 0, 1), colour mixed toward
  the fog colour per fragment on every material — including the character
  cards, whose `MeshBasicMaterial` honours scene.fog (env.ts:5-8). The
  sky dome and cloud sprites opt out (`fog: false`, world.ts:150, 168)
  and are instead re-coloured by the preset directly.
- **Rig**: key `#EAF0F2` ×0.7, position unchanged [7, 12, 5] (no keyPos),
  target [0, 0, -2]; rim ×0.15; hemisphere `#E6E9EA`/`#B8BCBE` ×0.9;
  shadow and environmentIntensity as JG base (§4).
- **Character tint**: sat 0.82, warm 0, exposure 0.94, fogColor
  `#E6E9EA`, fogMix 0.35 — the *hardest* fog pull of any preset. Note the
  tint's fog colour is the preset's `#E6E9EA`, **not** the shot's
  effective scene fog `#C9CFC2`; characters are pulled toward a slightly
  different grey than the world is. That mismatch is in the source, kept.
- **Grade**: `{ ...gradeOf(G.jungle, p), sat: 0.52 }` (`shots.ts:420`).
  gradeOf contributes sat 0.82 / warm 0 / exposure 0.96; the shot literal
  then overrides **sat to 0.52**. Vignette 0.24, grain 0.012, bloom 0.16,
  aperture 0.0012, maxblur 0.007 stay G.jungle's.
- The fire dies inside this shot: `fireLevel` level =
  `clamp(1 − seg(lt, 2.95, 3.7))` (sitAt 3.55; `shots.ts:407, 411`) — §8.

## 6 · golden — S07 only (JG)

Preset: env.ts:86-97. Applied at `shots.ts:427` with the rig, sky dressed:
dome `#4A90D9` / `#FFD98A` / `#FFD98A`, clouds `#FFE3B0`.

- **Fog**: the preset fog reaches the render unmodified:
  `#FFD98A`, near 30, far 140.
- **Rig**: key `#FFB35C` ×2.3 **at the preset keyPos [-8, 4.5, 5]** — this
  is one of the presets that *moves* the sun (from JG's [7, 12, 5]);
  target stays [0, 0, -2]. Rim ×0.6; hemisphere `#FFE0B0`/`#B07A48`
  ×1.0; shadow/environment as JG base. (The position write persists in
  the shared rig object; the next shot's applyAtmosphere decides the
  position again — dusk also carries a keyPos, noon/fog do not, and JG
  noon shots after S07 are none, so nothing in this film inherits it
  silently. Noted for the native port: rig state is per-scene,
  re-dressed per draw.)
- **Character tint**: sat 1.02, warm 0.08, exposure 1, fogColor
  `#FFD98A`, fogMix 0.06.
- **Grade**: `{ ...gradeOf(G.jungle, p), sat: Math.min(p.grade.sat, 0.85),
  warm: 0.06 }` (`shots.ts:435`). gradeOf contributes sat 1.15 / warm
  0.08 / exposure 1.02; the shot **clamps sat to 0.85** and **pins warm
  to 0.06**. Effective: sat 0.85, warm 0.06, exposure 1.02, vignette
  0.24, grain 0.012, bloom 0.16, aperture 0.0012, maxblur 0.007.

## 7 · dusk — S11, S12 (JG)

Preset: env.ts:98-107. Applied at `shots.ts:504` (S11) and `shots.ts:529`
(S12), sky dressed: dome `#3B3F8F` / `#F59C6B` / `#F59C6B`, clouds
`#FFC9B0`.

- **Fog**: preset fog unmodified: `#F59C6B`, near 28, far 120.
- **Rig**: key `#FFB27A` ×2.0 at preset keyPos **[-8, 5, -14]**, target
  [0, 0, -2]; rim ×0.5; hemisphere `#B9A2D8`/`#7A4A3A` ×0.9; shadow and
  environment as JG base.
- **Character tint**: sat 0.97, warm 0.06, exposure 0.96, fogColor
  `#F59C6B`, fogMix 0.08.
- **Grade**: `gradeOf(G.jungle, p)` with **no further overrides**
  (`shots.ts:521, 538`): sat 1.08, warm 0.06, exposure 0.97, vignette
  0.24, grain 0.012, bloom 0.16, aperture 0.0012, maxblur 0.007.

## 8 · The bookshop rig (S10, "noon" in name only) and the camp fire

**Bookshop.** `WD.interior(E, …)` (`shots.ts:227`; world.ts:292-341) with
w 8, depth 7, h 4.2. The noon call passes **no rig**, so none of these
values come from env.ts at all:

- Key (window sun): DirectionalLight `#FFE4B8` ×2.4 at (-2, 6, -13),
  target (0.5, 0, -1), castShadow; shadow map 2048², ortho ±8, near 0.5,
  far 30, bias −0.0006, radius 4 (world.ts:323-324). **normalBias is
  never set** for this light — see §10.
- Hemisphere `#FFE9D2`/`#5A3A2A` ×1.1 (world.ts:325).
- Fill PointLight `#FFD9A8`, intensity 6, distance 12, decay 1.4, at
  (0, 3.2, -1) (world.ts:326).
- Two bulb PointLights `#FFD9A0`, intensity 3, distance 6, decay 1.6, at
  (x, h−1.15, z) = (-1.5, 3.05, -2.2) and (1.6, 3.05, -4.5)
  (world.ts:328-331). Each hangs under a bulb mesh (sphere r 0.1 at
  y = h−1.05 = 3.15, material colour `#FFF6DA`, emissive `#FFE7A0`,
  emissiveIntensity 2.2) on a brass cord (r 0.008, h 1.0, `#C9A13B`).
- `scene.environmentIntensity` 0.35 (world.ts:294); scene background
  `#1A1418`; the window view is a flat `#BFE4F7` plane, `toneMapped:
  false` (world.ts:306).
- There is **no rim light** in the interior.

**Camp fire (JG).** Pit at (FX, FZ) = (-1.35, -2.45) (`shots.ts:170`).
PointLight `#FF9A3C`, constructed intensity 5, distance 7, decay 1.6, at
(-1.35, 0.6, -2.45) (`shots.ts:179`) — the constructed intensity is never
rendered; `fireLevel` overwrites it every frame (§9.5). Two emissive
cones (`shots.ts:175-178`): outer (r 0.17, h 0.52) at (−1.35, 0.34,
−2.45), colour `#3A1C08`, emissive `#FF7A1A` ×2.4; inner (r 0.09, h 0.34)
at (−1.32, 0.30, −2.41), colour `#3A2A08`, emissive `#FFC93C` ×2.6; both
roughness 1. Levels per shot are tabulated in the JSON (`fire.levelByShot`).

## 9 · Dynamics — what the source computes

### 9.1 applyAtmosphere (env.ts:164-216)

```
scene.fog = preset.fog ? new THREE.Fog(preset.fog.color, preset.fog.near, preset.fog.far) : null
if (sky !== false):
    every Sprite: material.color = preset.cloudTint
    every ShaderMaterial with top/hor/gnd uniforms: uniforms = preset.skyTop/skyHor/skyGnd
if (rig given):
    rig.key.color = preset.keyCol;  rig.key.intensity = preset.keyI
    if (preset.keyPos) rig.key.position = preset.keyPos        // position ONLY when present
    rig.rim.intensity = preset.rimI                            // rim colour never touched
    HemisphereLight found inside rig.group: color = hemiSky, groundColor = hemiGround, intensity = hemiI
every scene Toon with envMatch === true: toon.matchEnv(preset.tint)
return preset   // grade is the CALLER's business
```

### 9.2 Character env-match tint (toon.ts:664-674)

Given tint `t`, the material colour multiplier (body plane and head plane)
is computed fresh each call (it *replaces*, not accumulates):

```
r = 1 + 0.6 * t.warm;   g = 1;   b = 1 - 0.6 * t.warm
if (t.sat < 1):  k = (1 - t.sat) * 0.25;  each channel c += (0.92 - c) * k
e = clamp(sqrt(max(t.exposure, 0)), 0, 1.15);  r,g,b *= e
if (t.fogMix > 0):  colour = lerp(colour, Color(t.fogColor), clamp(t.fogMix, 0, 1))
```

The source is explicit that this is a multiply approximation: it cannot
truly desaturate a painted texture (env.ts:12-15; toon.ts:656-663). For
the noon identity tint the result is exactly (1, 1, 1). Every character
in this film is built with `envMatch: true` (`shots.ts:36-39`, `person()`).

### 9.3 Sky dome shader (world.ts:149-155)

Sphere r = 380, BackSide, `depthWrite: false`, `fog: false`,
`toneMapped: false`, renderOrder −10:

```
y = normalize(position).y
c = y > 0 ? mix(hor, top, pow(clamp(y * 1.6, 0, 1), 0.7)) : gnd
```

### 9.4 Bokeh focus is a per-frame distance

`shoot()` (`shots.ts:127-131`) passes
`focus = cam.position.distanceTo(current look target)` on every frame,
the target being the shot's `at`/`at2` mixed by the same camera parameter
k as the position. During push-ins and cranes the focus therefore tracks
the subject continuously. There is no per-shot focus constant to
transcribe; aperture/maxblur are the constants (in `shotGrades`, with
S02's aperture override 0.0007 at `shots.ts:340`).

### 9.5 Fire (shots.ts:287-291, `fireLevel(level, t)`)

`t` is **global film time**, not shot-local. `level ∈ [0, 1]`.

```
vis = level > 0.02;  flame.visible = flame2.visible = vis
if (vis):
    f = 1 + 0.16 * sin(13 t) + 0.07 * sin(29 t + 1)
    flame.scale  = (level, f * level, level)
    flame2.scale = (level, (2 - f) * level, level)
    fireLight.intensity = (4.6 + 0.7 * sin(17 t) + 0.5 * sin(7.3 t)) * level
else: fireLight.intensity = 0
```

Levels: 1 in S01/S04/S05b/S08/S08b/S09/S11/S12; 0 in S05a/S07; S06 eases
1 → 0 over `seg(lt, 2.95, 3.7)` (shot-local `lt`). The older boolean
helper `fireTo` (`shots.ts:190-194`, same `f`, intensity unscaled) is
**defined but never called** — recorded in the JSON so the port doesn't
mistake it for the live path.

### 9.6 GRADE shader (engine.ts:20-24) and the time seed

```
h(p) = fract(sin(dot(p, vec2(12.9898, 78.233)) + uTime * 7.13) * 43758.5453)
d = vUv - 0.5;  d.x *= 0.5625
rgb *= 1 - uVig * smoothstep(0.25, 0.75, length(d) * 1.4)
rgb += (h(vUv * 1000.0) - 0.5) * uGrain
r += uWarm * 0.5;  b -= uWarm * 0.5
l = dot(rgb, vec3(0.299, 0.587, 0.114));  rgb = mix(vec3(l), rgb, uSat)
```

No code path in `shots.ts` or `main.ts` ever sets `look.t`, so `uTime =
0` for **every frame of this film**: the grain term is a fixed per-pixel
spatial offset, not animated grain. A native port that animates grain
from the frame index would diverge.

### 9.7 gradeOf (shots.ts:139)

`gradeOf(base, p) = { ...base, sat: p.grade.sat, warm: p.grade.warm,
exposure: p.grade.exposure }`. Only sat/warm/exposure cross from the
preset into the look; bloom, aperture, maxblur, vignette and grain always
come from the shot-grade constant. Used by S06, S07, S11, S12 only.

## 10 · Values that could NOT be pinned down exactly

Named, not guessed:

1. **Capture-time QS parameters.** Shadow map size (default 2048), MSAA
   samples (default 4) and render scale RES (default 1) come from the
   capture URL query string (`engine.ts:16`), which lives in the capture
   invocation, not in `src/`. The JSON records the engine defaults; the
   delivered master's actual QS is not recorded anywhere in the project
   tree, so if the original capture overrode them, that fact is not
   recoverable from source.
2. **Interior key `normalBias`.** Never assigned for the bookshop sun
   (the interior does not use `lightRig`). The JSON records 0, which is
   the three.js `DirectionalLightShadow` library default — a library
   fact, not a source literal.
3. **Bokeh focus** is not a constant at all (it is the per-frame camera
   distance, §9.4) — given as a rule, not a number, deliberately.
