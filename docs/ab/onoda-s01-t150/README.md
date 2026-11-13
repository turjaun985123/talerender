# A/B §28 — Onoda S01 (HOOK) @ t = 1.50 s

One real Tall Tale scene, rendered twice:

- **original.png** — the shipped pipeline: the delivered project's own
  review frame `review/frames/t1.50.jpg` (headless Chrome, SwiftShader
  WebGL, `OP.frame(t)` JPEG), converted losslessly to PNG. Provenance:
  the project's `review/NOTES.md` records that the `frames/` stills
  were generated fresh by the finishing pass from the byte-verified
  final build, with the full check green (CHECK OK · CANVAS OVERFLOW
  none · CONTINUITY clean).
- **native.png** — TaleRender `--scene ab1` (this repo), wgpu on
  **lavapipe** (software Vulkan), written directly to PNG.

Both frames are software-rendered; neither is a GPU master
(contract §6.2). Performance numbers from this VM are meaningless and
none are reported.

## The shot, and why this one

**Project:** `the-soldier-who-didnt-know` — "THE SOLDIER WHO DIDN'T
KNOW THE WAR WAS OVER" (Hiroo Onoda), delivered 2026-11-12, built
with the v4-era Tall Tale engine (the project vendors its own core
under `src/core/`; all citations below are to the project's files).

**Shot S01 (HOOK), t = 1.50 s** — Onoda alone at his jungle camp,
rifle at rest, slow push-in. Chosen because it is the simplest real
shot that still exercises the whole stack:

- ONE set (the JG jungle camp), ONE visible character, ONE 3D prop
  (the attach3D rifle), a fire with animated light, a caption.
- The camera move is a slow push-in: at t = 1.50 the camK interpolation
  is only k ≈ 0.052 along its path, so the frame is near-static and
  every animated quantity (head sweep, blink, fire) is exactly
  computable from the source formulas.
- An original frame at the *exact* timestamp already exists in the
  project's review folder — no re-capture ambiguity.

What it does **not** prove: depth of field (M3-gated, absent
natively), painter stroke parity (the M4 battery), multi-character
staging, talk/lip-sync (narration era — mouth shut), or caption
*metrics* parity beyond one word. Each absence is itemised in
[gaps.md](gaps.md).

## Staging extraction (every value cited)

Shot definition: `src/shots.ts` S01 block (≈lines 322–331). Duration
d = S02.at − S01.at; S02.at = cut(1) = L(1) − 0.06 = 5.967 − 0.06 =
**5.907 s** (`src/shots.ts` cut(); `data/lines.json` line 1 start
5.967). lt = t − 0 = **1.50**.

### Camera

`MO.camK(lt, d)` = smootherstep((lt/d − 0.1)/0.8) = smootherstep(0.19243)
= **k = 0.05228** (`src/core/motion.ts` camK/smootherstep).

| quantity | value | source |
|---|---|---|
| from | (0.85, 2.70, 5.40) | shots.ts S01 |
| to | (0.55, 1.62, 3.20) | shots.ts S01 |
| **position** | **(0.8343, 2.6435, 5.2850)** | mix(from, to, k) |
| look from | (0.35, 1.12, −0.55) | shots.ts S01 |
| look to | (0.35, 1.02, −0.55) | shots.ts S01 |
| **look target** | **(0.35, 1.1148, −0.55)** | mix(at, at2, k) |
| fov | **44°** | `{...G.jungle, fov: 44}` in the shoot() call |
| near / far | 0.05 / 800 | `src/core/engine.ts` camera() |
| DoF focus | 6.051 m (≡ \|pos − target\|) | shoot() default focus |
| grade (G.jungle) | bloom 0.16, aperture 0.0012, maxblur 0.007, sat 0.8, warm 0.04, vignette 0.24, grain 0.012 | shots.ts ≈line 148 |

`shoot()` (shots.ts ≈142–146) also calls `face(cam)` on the whole
cast; only Onoda is visible (jgStage hides the other five toons).

### Onoda (character)

| quantity | value | source |
|---|---|---|
| look | body `#5C6644`, hat `cap`, hatCol `#4C5638` | shots.ts LOOK registry, lines 24–30 |
| shade (non-identity) | `#878E6E` | person(): `look.shade = '#878E6E'` |
| head | `#F7F7FA` (default) | toon.ts ResolvedLook |
| ink | `#1F1C33`, LW 9 | toon.ts lines 38–39 |
| constructor | `{ perspective: true }`, envMatch true | person('onoda', …) |
| height | 1.75 m (default) → H = 1.75/0.96 = 1.8229 | toon.ts constructor |
| plane | 1.25H × 1.375H = 2.2786 × 2.5060 m, centre 552·(H/1024) above feet | toon.ts ≈531–546 |
| position | (0.35, 0, −0.55) | `jgOnoda.at(0.35, 0, -0.55)` |
| mood | `determined` | S01 set() |
| armR / armL | [16, −6] / [10, 6] (degrees) | S01 set() |
| lean | 1° about the feet | S01 set() |
| headTurn | **−0.025** — poseTrack([[0,−0.45],[2.6,0.4,2.2],[5.2,−0.2,2.2]], 1.5): u = (1.5−0.4)/2.2 = 0.5, smootherstep(0.5) = 0.5 → −0.45 + 0.85·0.5 | motion.ts poseTrack; shots.ts S01 |
| blink | **0** — blinkDirector(3) walk lands 1.21 s into a 2.9 s interval (≥ 0.12 window); blinkAt(1.5, [Wd('years',0)=4.2]) = 0; frame shows eyes open | motion.ts; data/words.json |
| camPitch | **0.28747** — camPitchOf: pitchAng = atan2(2.6435 − 1.0938, 5.8550) = 0.25872 rad, /0.9 | toon.ts camPitchOf |
| face() pivot | yaw = atan2(0.4843, 5.8350) = **0.08282 rad**; pitch = clamp(0.25872·0.45) = **0.11642 rad**, order YXZ | toon.ts face() |
| hand R (stage) | (373.98, 599.73) — FK: shoulder (322, 376), upper 118 @16°, fore 112 @10° | toon.ts drawToon arm() |
| rifle anchor | mesh-local ((384+373.98−640)·s, (1256−855.73)·s, 0) + offset (0, 0.03, 0.07), s = H/1024 | toon.ts syncAttachments |

Painter geometry (toon.ts drawToonLayer/drawHeadPart/drawFace/drawHat):
body box bx 256, top 330, bot 600, bw 92, hipY 574; bean = the four
cubic segments coded verbatim in the native painter; shade ellipse at
(388, 485) r (70, 260) alpha 0.85 clipped to the bean; highlight
ellipse (206, 420) r (16, 60) rot 0.2 alpha 0.25; legs stand (knee
hipY+210, foot y 1000, foot ellipse r (20, 10)); head centre (256,
178), r 138 → rp = r·(1+0.04·camPitch) = 139.59, radii (rp, rp·0.97);
head shade = head ∩ ¬circle((307.25, 222), rp), shadeOf(head, 0.9);
face fx = 256 + turn·30 = 255.25, ex = 46·(1−0.18·|turn|) = 45.79,
ey = 178 + 4 + camPitch·8 = 184.30; 'determined' brows [lift −4,
tilt 16] width 12; eyes: pupil ellipse (13, 17) + heavy lid (head-
colour rect (x−24, ey−28, 48, 26) + width-7 line at ey−2); mouth line
(fx−24, my+5)→(fx+24, my+1), my = ey+62 = 246.30, width 8; cap:
crown ellipse ((hatx+6, top+34), (rp·0.98, 62), rot −0.05, upper
half), brim ellipse ((hatx−24, top+40), (rp·0.78, 22), rot 0.05,
lower half) in shadeOf(cap, 0.75), button circle r 12 at
(hatx+6, top−26) in shadeOf(cap, 1.15); hatx = 256 + turn·18,
top = 178 − rp·0.97 = 42.60.

Rifle (shots.ts makeRifle ≈64–73), modelled along +Y, grip at origin:
stock box(0.075, 0.52, 0.13) wood `#6E4F30` @ y −0.17; receiver
box(0.06, 0.10, 0.09) steel `#3A3D42` metalness 0.55 @ 0.13; barrel
cyl(0.016, 0.016, 0.55) steel @ 0.42; front sight box(0.02, 0.05,
0.02) steel @ 0.68; grip swell box(0.05, 0.05, 0.16) wood @
(0, −0.02, 0.05). Attached via `jgOnoda.attach(rifleHeld, 'R',
{ offset: [0, 0.03, 0.07] })`.

Blob shadow (toon.ts): plane (H/2·0.9) × (H/2·0.34) at the group
position, y 0.01, radial texture rgba(30,20,40, 0.75→0.25@0.6→0),
material opacity 0.55.

### The set — JG jungle camp (shots.ts ≈139–196)

- **Sky:** `WD.sky` dome r 380, top `#6E9CC0`, horizon `#D8E8DA`;
  shader: y>0 → mix(hor, top, pow(clamp(y·1.6,0,1), 0.7)); below →
  horizon colour (world.ts sky()).
- **Lights:** `WD.sunlight` key `#FFEFD2` i 2.4 pos [7, 12, 5], box
  17, hemi sky `#DCECD2` / ground `#46603A` i 1.15; key target
  (0, 0, −2); shadow far 80, bias −0.0006; then `noonJG()` applies
  Atmosphere 'noon' (env.ts): keyCol `#FFF0D6`, **keyI 2.6**, rimI
  0.4, hemiSky `#CFE6FF`, hemiGround `#C48A5A`, **hemiI 1.25**, fog
  null, tint identity (envMatch is a no-op at noon). lightRig
  (engine.ts): rim `#FFFFFF` from (−3, 3, −3), shadow map 2048²,
  PCFSoft, normalBias 0.02. PMREM RoomEnvironment IBL,
  environmentIntensity 0.25 (sunlight default).
- **Ground:** circle r 80, grass texture `T.grass('#5B9A48', [48,48])`
  (512² canvas: base + 5000 hash strokes, shade 0.7–1.3, repeat 48×48).
- **Dirt patch:** circle r 4.8, `#86714C`, at (0, 0.012, −0.9).
- **Hills:** [[−40,−75,34,12],[10,−85,46,16],[60,−71,36,11],
  [−90,−65,40,10]] (x, z, r, h): sphere scaled (r, h, 0.6r), y −h/4,
  colour shade(`#5E8F52`, 0.92+rnd(i)·0.16) with grass map [6,3];
  6 small trees per hill, castShadow false.
- **Tree ring:** 19 trees [x, z, s, kind] — full table in
  `src/ab1.rs` (`ring`); rotation.y = i·1.7; colour i%3 ? `#3E7A34` :
  `#356B2C`. tree(): trunk cyl(0.12s, 0.18s, 1.6s) `#7A5234` @ 0.8s;
  round = 5 icosahedron blobs at (0,2.3,0,1.0) (0.6,2.0,0.2,0.75)
  (−0.6,2.0,−0.1,0.8) (0.1,2.9,−0.2,0.75) (0.2,2.2,0.6,0.7) ·s,
  shade(col, 0.9+rnd(i)·0.2); cypress = sphere scaled
  (0.7s, 2.6s, 0.7s) @ 3.2s.
- **Palms** (shots.ts palm ≈76–90): trunk cyl(0.085, 0.13, 4.4, 8)
  `#7A6248`; 7 fronds (sphere scaled (0.17, 0.05, 1.2) @ radius 1.0,
  tips (0.13, 0.04, 0.8) @ 1.85), leaf `#3F7D33` / leaf2 `#4C8A3A`;
  3 coconuts r 0.13 `#6B4A2E`. Placements: (4.6, 0, −4.6) s 1.05
  lean 0.14; (−5.2, 0, −5.4) s 0.9 lean −0.2 rotY 1.2; (10.5, 0,
  −11) s 1.15 lean 0.1.
- **Grass tufts:** InstancedMesh cone(0.05, 0.34, 4) `#4C8A3E` ×900;
  instance i: a = rnd(i·1.31)·2π, r = 2.6 + rnd(i·2.7)·20, at
  (cos a·r, 0.15, sin a·r − 3). rnd = frac(sin(i·127.1+311.7)·43758.5453).
- **Camp** (fire pit FX −1.35, FZ −2.45): 8 stones sphere(0.09 +
  rnd(i)·0.04) `#8B8B84` on r 0.44; 3 fire logs cyl(0.05, 0.05, 0.72)
  `#7A5A38`; flame cone(0.17, 0.52, 8) emissive `#FF7A1A` ei 2.4;
  flame2 cone(0.09, 0.34, 8) emissive `#FFC93C` ei 2.6; fireLight
  PointLight `#FF9A3C` distance 7 decay 1.6 @ (FX, 0.6, FZ); seat log
  cyl(0.17, 0.17, 1.15) @ (−0.5, 0.17, −1.8) rotZ π/2 rotY 0.12;
  lean-to poles cyl(0.035, 0.035, 1.5) @ x −3.9/−2.7, z −2.7; roof
  box(1.9, 0.05, 1.5) planks `#8A6B42` @ (−3.3, 1.52, −2.95) rotX
  0.42; crates 0.55 @ (−2.35, ·, −3.15) and 0.42 @ (−2.3, 0.55, −3.12)
  rotY 0.4, planks `#B07A48`; barrel cyl(0.33, 0.33, 0.8) `#9A6337`
  × planks @ (−3.05, ·, −2.2) + 2 bands `#3A3A3F`; bedroll box(1.5,
  0.09, 0.55) `#77805E` @ (−3.3, 0.05, −2.1) rotY 0.2.
- **Fire at t = 1.50** (fireLevel(1, t), shots.ts ≈291–295):
  f = 1 + 0.16·sin(19.5) + 0.07·sin(44.5) = **1.13146** → flame
  scaleY 1.13146, flame2 scaleY 0.86854; fireLight intensity =
  4.6 + 0.7·sin(25.5) + 0.5·sin(10.95) = **4.3519**.
- **Clouds:** sprites at (−28, 23, −85, s 13), (26, 29, −95, s 16),
  (58, 19, −75, s 10), scale (2s, s), tint `#EFF4EA`; cloudTex =
  512×256, 7 puffs [(110,175,58) (185,140,80) (285,118,96)
  (380,150,70) (250,178,70) (440,180,44) (70,190,36)], shadow pass
  `#B9CCE6` @ (+4,+14) blur 3, main white @ (−2,−2) r·0.92, bottom
  strip blurred out.

### Caption

Line 0 (`data/lines.json`: 0.30–5.787) has explicit caps chunks
(script.json line 0 `caps`); chunk "fighting": s = 1.38 − 0.03 =
1.35, e = next chunk s = 1.86 − 0.03 = 1.83 (captions.ts
buildCaptions). At t = 1.50: active, pop-in age 0.15 ≥ 0.1 → scale =
fit. Layout (captions.ts): centre (540, 1340), font 900 88px Inter,
fit = min(1, 960/measure) — FreeType measures "fighting" @88px =
354.03 px → **fit = 1.0**; passes: shadow rgba(0,0,0,0.45) @ (+3,+9),
stroke lineWidth 15 `#0B0B10` round join, fill white. The native
frame composites a pre-rendered strip (`assets/caption.png`, made by
`scripts/make_caption.py` from the project's own Inter-Black.otf) on
a camera-facing quad at the same NDC position.

### Engine (original side)

Renderer (engine.ts): WebGLRenderer, NeutralToneMapping exposure
1.0, sRGB out, PCFSoft shadows (2048², autoUpdate off + per-frame
needsUpdate), composer over HalfFloat MSAA-4×: RenderPass →
BokehPass (focus 6.051, aperture 0.0012, maxblur 0.007) →
UnrealBloomPass (0.16, radius 0.6, threshold 0.85) → OutputPass →
GRADE shader (sat 0.8, warm 0.04, vignette 0.24, grain 0.012, uTime =
t). Frame assembled in main.ts draw(): shot → blit → fade (1.0 at
t = 1.5) → caption. `OP.frame` returns the canvas as JPEG (q 0.92
default).

## Reproduce

Original frame (already in the project; this is where it came from):

```
cd ~/faceless-shorts/the-soldier-who-didnt-know
# review/frames/t1.50.jpg — capture tool, SwiftShader Chrome, one frame
python3 -c "from PIL import Image; Image.open('review/frames/t1.50.jpg').convert('RGB').save('<pkg>/original.png')"
```

Native frame (binary built by this repo's CI, run locally):

```
gh run download <run-id> -n talerender-binary -D /tmp/tr
cd ~/workspace/talerender
python3 docs/ab/onoda-s01-t150/scripts/make_caption.py \
  ~/faceless-shorts/the-soldier-who-didnt-know \
  docs/ab/onoda-s01-t150/assets/caption.png
VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json \
  /tmp/tr/talerender --scene ab1 --width 1080 --height 1920 \
  --caption docs/ab/onoda-s01-t150/assets/caption.png \
  --out docs/ab/onoda-s01-t150/native.png
python3 docs/ab/onoda-s01-t150/scripts/ab_metrics.py docs/ab/onoda-s01-t150
```

## Results

<!-- filled after the native render: metrics headline + CI verdict -->
