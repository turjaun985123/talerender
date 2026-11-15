# Fog chart — Chrome harness assembly + verification record (seat W66)

This file records: (1) the assembled runnable harness project under
[`project/`](project/), built exactly as the M3 dof-chart package was
(`docs/ab/dof-chart/README.md`, Reproduce); (2) a static, field-by-field
verification that the Chrome staging (`shots.ts`), the native scene
files (`scenes/fog-chart.scene.json` / `fog-chart-nofog.scene.json`)
and `expected.json` all encode the **same** chart; and (3) a real
headless-Chrome run of the harness on this VM with its panel-centre
measurements against the analytic expectations.

**Headline results**

- Static verification: **144/144 checks PASS** (reproduce:
  `python3 tools/fog-chart/chrome/verify_constants.py`, exit 0).
- Chrome run: frame captured (renderer string and hashes in §3).
  The **basic panels match the analytic fog expectations to ≤ 1 sRGB
  code at every depth** — including the three depths (6/10/15 m) where
  the classic-ramp formula misses by 26/12/8 codes. The fog formula
  the Chrome side implements is three's smoothstep, confirmed
  empirically, not just by source reading.
- The standard panels sit far from the analytic column, and the
  offset is **fully explained**: it is a light-intensity convention,
  not fog. Predicting them with the house factor divided by π
  reproduces every sampled centre to ≤ 2 codes (§4). The package
  already declares the analytic column as the gate and the Chrome
  standard centres as cross-check only; this record turns that
  declared residual into a measured, named quantity.

## 1. Assembly

Per the M3 pattern: copy the skill template, drop in the project's
`shots.ts` + `script.json`, add empty timing data, `bun run build`,
capture with `tools/capture.mjs frames`.

```
project/  =  cp -r ~/workspace/custom-skills/tall-tale-shorts/template/.
          +  src/shots.ts    <- chrome/shots.ts    (drop-in)
          +  script.json     <- chrome/script.json (drop-in)
          +  data/lines.json    {"duration": 2.0, "voice": "am_michael",
                                 "speed": 1.05, "lines": []}
          +  data/words.json    []
          (template's tools/__pycache__ not copied)
```

- Drop-ins are byte-identical to the canonical staging files in this
  directory: `shots.ts` sha256
  `a0be29494dd23bccd9f5822abddf549e043bcd11a06e3a7f025598d24909ede5`,
  `script.json` sha256
  `0f9c84e46beb21ce61893c134244c02349d356f693f9b3980682dd9f865c868e`
  (same hash at both paths).
- The empty timing data is the M3 harness convention: the chart's
  single shot is static and time-independent, `script.json` carries
  no lines, and there are no captions to audit. `data/lines.json`'s
  `duration: 2.0` sets the capture clock: at the capture instant
  t = 1.0 the page's fade term is
  `min(clamp(1.0/0.25), clamp((2.0-1.0)/0.4)) = 1`, so no fade
  overlay touches the frame. (Template leftovers `data/cues.json` /
  `data/words.txt` remain in the copy; the page never fetches them —
  `loadTiming()` reads only `lines.json` + `words.json`, and the
  frames mode never calls `OP.cues()`.)
- Build: `bun run build` (bun 1.4.4) → `build/main.js` (77.5 KB),
  `build/sheet.js`, `build/castsheet.js`; `three` stays external and
  resolves through `index.html`'s import map to `vendor/`.
- The page loads **this chart's own staging**: `index.html` loads
  `build/main.js`, compiled from `project/src/shots.ts` — the
  byte-identical drop-in above — whose `buildShots()` sets
  `scene.fog = new THREE.Fog(0xC9CFC2, 4.5, 20)`, neutralises the
  look, and renders with `aperture: 0` (see the section-C assertions
  in §2, which are run against the canonical `chrome/shots.ts`).

## 2. Static verification — method

[`verify_constants.py`](verify_constants.py) checks the staging three
ways, against one shared spec (fog `#C9CFC2` near 4.5 far 20; rows at
3/6/10/15/25 m, NDC y +0.62/+0.31/0/−0.31/−0.62; basic x −0.45,
standard x +0.45; panel size 0.62 × 0.30 of the frustum half-extents;
house light dir (0.5, 0.8, 0.6), ambient 0.35):

- **A — native scene file vs spec, numerically.** Panel geometry is
  re-derived in f64 from the spec constants
  (`tanH = tan(20°)`, `hh = d·tanH`, `hw = hh·(1080/1920)`,
  `w = 0.62·hw`, `h = 0.30·hh`, `cy = ndcY·hh`) and every stored
  number must equal the **exact f32 cast** of the derived value —
  the v2 data law, not a tolerance comparison.
- **B — control integrity.** The no-fog file deep-equals the fog
  file with `fog` and `name` removed.
- **C — Chrome staging, source-text assertions.** The constants and
  construction in `shots.ts` (fog, camera, lights, ground, panel
  placement, flat texture, the neutral draw look), plus the engine
  camera defaults it relies on (`src/core/engine.ts:73`:
  `PerspectiveCamera(fov, W/H, 0.05, 800)`).
- **D — `expected.json` internal consistency.** Fog factors equal
  `smoothstep(4.5, 20, d)` recomputed in f64; centre pixels equal
  the NDC centres mapped to 1080×1920 (round half up); base colours
  and the house-light front-face factor
  `0.35 + 0.65·(0.6/√1.25) = 0.6988266044899671` recomputed.

Full output of the final run (144/144 PASS, exit 0):

### A scene

| check | spec (derived) | staged | verdict |
|---|---|---|---|
| camera.fovY | 40.0 | 40.0 | PASS |
| camera.near = f32(0.05) | 0.05000000074505806 | 0.05000000074505806 | PASS |
| camera.far | 800.0 | 800.0 | PASS |
| camera.eye | [0, 0, 0] | [0, 0, 0] | PASS |
| camera.target | [0, 0, -1] | [0, 0, -1] | PASS |
| camera.up | [0, 1, 0] | [0, 1, 0] | PASS |
| clear = fog colour | #C9CFC2 | #C9CFC2 | PASS |
| fog.color | #C9CFC2 | #C9CFC2 | PASS |
| fog.near | 4.5 | 4.5 | PASS |
| fog.far | 20.0 | 20.0 | PASS |
| lighting.direction = f32(0.5, 0.8, 0.6) | [0.5, 0.800000011920929, 0.6000000238418579] | [0.5, 0.800000011920929, 0.6000000238418579] | PASS |
| lighting.ambient = f32(0.35) | 0.3499999940395355 | 0.3499999940395355 | PASS |
| post keys (bokeh only; no bloom/grade) | ['bokeh'] | ['bokeh'] | PASS |
| post.bokeh.focus | 10.0 | 10.0 | PASS |
| post.bokeh.aperture (bokeh disabled) | 0 | 0 | PASS |
| post.bokeh.maxblur = f32(0.006) | 0.006000000052154064 | 0.006000000052154064 | PASS |
| texture panel-basic-3m | solid #D94F30 | solid #D94F30 | PASS |
| texture panel-basic-6m | solid #3F6FD9 | solid #3F6FD9 | PASS |
| texture panel-basic-10m | solid #7A8B99 | solid #7A8B99 | PASS |
| texture panel-basic-15m | solid #2FA89B | solid #2FA89B | PASS |
| texture panel-basic-25m | solid #8C8C46 | solid #8C8C46 | PASS |
| basic 3 m geometry | quad f32(0.380804) x f32(0.327573), y0 f32(-0.163787) | quad 0.3808038532733917 x 0.3275732100009918, y0 -0.1637866050004959 | PASS |
| basic 3 m translate | [-0.2763898968696594, 0.6769846081733704, -3.0] | [-0.2763898968696594, 0.6769846081733704, -3.0] | PASS |
| basic 3 m texture ref | panel-basic-3m | panel-basic-3m | PASS |
| standard 3 m geometry | box f32(0.380804) x f32(0.327573) x f32(0.1) | box [0.3808038532733917, 0.3275732100009918, 0.10000000149011612] | PASS |
| standard 3 m material | #30D94F rough 1.0 | #30D94F rough 1.0 | PASS |
| standard 3 m translate | [0.2763898968696594, 0.6769846081733704, -3.049999952316284] | [0.2763898968696594, 0.6769846081733704, -3.049999952316284] | PASS |
| standard 3 m front face at exactly 3 m | -3.0 | -2.9999999523162844 | PASS |
| basic 6 m geometry | quad f32(0.761608) x f32(0.655146), y0 f32(-0.327573) | quad 0.7616077065467834 x 0.6551464200019836, y0 -0.3275732100009918 | PASS |
| basic 6 m translate | [-0.5527797937393188, 0.6769846081733704, -6.0] | [-0.5527797937393188, 0.6769846081733704, -6.0] | PASS |
| basic 6 m texture ref | panel-basic-6m | panel-basic-6m | PASS |
| standard 6 m geometry | box f32(0.761608) x f32(0.655146) x f32(0.1) | box [0.7616077065467834, 0.6551464200019836, 0.10000000149011612] | PASS |
| standard 6 m material | #D9B83F rough 1.0 | #D9B83F rough 1.0 | PASS |
| standard 6 m translate | [0.5527797937393188, 0.6769846081733704, -6.050000190734863] | [0.5527797937393188, 0.6769846081733704, -6.050000190734863] | PASS |
| standard 6 m front face at exactly 6 m | -6.0 | -6.0000001907348635 | PASS |
| basic 10 m geometry | quad f32(1.26935) x f32(1.09191), y0 f32(-0.545955) | quad 1.2693462371826172 x 1.091910719871521, y0 -0.5459553599357605 | PASS |
| basic 10 m translate | [-0.9212996363639832, 0.0, -10.0] | [-0.9212996363639832, 0.0, -10.0] | PASS |
| basic 10 m texture ref | panel-basic-10m | panel-basic-10m | PASS |
| standard 10 m geometry | box f32(1.26935) x f32(1.09191) x f32(0.1) | box [1.2693462371826172, 1.091910719871521, 0.10000000149011612] | PASS |
| standard 10 m material | #8E44AD rough 1.0 | #8E44AD rough 1.0 | PASS |
| standard 10 m translate | [0.9212996363639832, 0.0, -10.050000190734863] | [0.9212996363639832, 0.0, -10.050000190734863] | PASS |
| standard 10 m front face at exactly 10 m | -10.0 | -10.000000190734863 | PASS |
| basic 15 m geometry | quad f32(1.90402) x f32(1.63787), y0 f32(-0.818933) | quad 1.9040192365646362 x 1.6378660202026367, y0 -0.8189330101013184 | PASS |
| basic 15 m translate | [-1.3819494247436523, -1.692461609840393, -15.0] | [-1.3819494247436523, -1.692461609840393, -15.0] | PASS |
| basic 15 m texture ref | panel-basic-15m | panel-basic-15m | PASS |
| standard 15 m geometry | box f32(1.90402) x f32(1.63787) x f32(0.1) | box [1.9040192365646362, 1.6378660202026367, 0.10000000149011612] | PASS |
| standard 15 m material | #C94F7C rough 1.0 | #C94F7C rough 1.0 | PASS |
| standard 15 m translate | [1.3819494247436523, -1.692461609840393, -15.050000190734863] | [1.3819494247436523, -1.692461609840393, -15.050000190734863] | PASS |
| standard 15 m front face at exactly 15 m | -15.0 | -15.000000190734863 | PASS |
| basic 25 m geometry | quad f32(3.17337) x f32(2.72978), y0 f32(-1.36489) | quad 3.173365592956543 x 2.7297768592834473, y0 -1.3648884296417236 | PASS |
| basic 25 m translate | [-2.3032491207122803, -5.641538619995117, -25.0] | [-2.3032491207122803, -5.641538619995117, -25.0] | PASS |
| basic 25 m texture ref | panel-basic-25m | panel-basic-25m | PASS |
| standard 25 m geometry | box f32(3.17337) x f32(2.72978) x f32(0.1) | box [3.173365592956543, 2.7297768592834473, 0.10000000149011612] | PASS |
| standard 25 m material | #466B8C rough 1.0 | #466B8C rough 1.0 | PASS |
| standard 25 m translate | [2.3032491207122803, -5.641538619995117, -25.049999237060547] | [2.3032491207122803, -5.641538619995117, -25.049999237060547] | PASS |
| standard 25 m front face at exactly 25 m | -25.0 | -24.999999237060546 | PASS |
| ground geometry | box [80, f32(0.1), 72] | [80.0, 0.10000000149011612, 72.0] | PASS |
| ground material | #2E4A34 rough 1.0 | #2E4A34 rough 1.0 | PASS |
| ground translate | [0.0, -8.050000190734863, -29.0] | [0.0, -8.050000190734863, -29.0] | PASS |

### B control

| check | spec (derived) | staged | verdict |
|---|---|---|---|
| nofog == fog file minus fog block + name | deep-equal | deep-equal | PASS |
| nofog has no fog block | absent | absent | PASS |
| names differ (fog vs NO-FOG control) | different | fog chart — S06 fog ... / fog chart, NO-FOG co... | PASS |

### C chrome

| check | spec (derived) | staged | verdict |
|---|---|---|---|
| FOV const | 40 | present | PASS |
| fog consts | 0xC9CFC2 / 4.5 / 20 | present | PASS |
| ROWS | [d, ndcY] x5 | present | PASS |
| panel NDC x | -0.45 / +0.45 | present | PASS |
| panel NDC size | 0.62 x 0.30 | present | PASS |
| BASIC_COL[3] | #D94F30 | present | PASS |
| STD_COL[3] | #30D94F | present | PASS |
| BASIC_COL[6] | #3F6FD9 | present | PASS |
| STD_COL[6] | #D9B83F | present | PASS |
| BASIC_COL[10] | #7A8B99 | present | PASS |
| STD_COL[10] | #8E44AD | present | PASS |
| BASIC_COL[15] | #2FA89B | present | PASS |
| STD_COL[15] | #C94F7C | present | PASS |
| BASIC_COL[25] | #8C8C46 | present | PASS |
| STD_COL[25] | #466B8C | present | PASS |
| GROUND_COL | #2E4A34 | present | PASS |
| scene.fog set | THREE.Fog(FOG_COLOR, 4.5, 20) | present | PASS |
| background = fog colour | Color(FOG_COLOR) | present | PASS |
| camera(FOV) | engine camera(40) | present | PASS |
| camera at origin | (0, 0, 0) | present | PASS |
| camera looks down -Z | lookAt(0, 0, -1) | present | PASS |
| ambient light | 0.35 white | present | PASS |
| directional light | 0.65 white | present | PASS |
| light dir = 20 x (0.5, 0.8, 0.6) | (10, 16, 12) | present | PASS |
| ground box | 80 x 0.1 x 72 | present | PASS |
| ground position | (0, -8.05, -29) | present | PASS |
| ground material | GROUND_COL rough 1 metal 0 | present | PASS |
| frustum halves | hw = d*tanH*aspect, hh = d*tanH | present | PASS |
| panel size/centre | 0.62*hw, 0.30*hh, ny*hh | present | PASS |
| basic position | (-0.45*hw, cy, -d) | present | PASS |
| standard box depth | 0.1 | present | PASS |
| standard position | (+0.45*hw, cy, -(d+0.05)) | present | PASS |
| standard material | STD_COL[d] rough 1 metal 0 | present | PASS |
| basic material | flat texture, unlit | present | PASS |
| flat texture size | 64x64 canvas | present | PASS |
| flat texture colour space | SRGBColorSpace | present | PASS |
| flat texture mipmaps off | single level, linear | present | PASS |
| draw look (neutral, aperture 0) | focus 10, aperture 0, maxblur 0.006, bloom 0, exposure 1, vignette 0, grain 0, warm 0, sat 1 | present | PASS |
| engine camera() defaults (engine.ts) | PerspectiveCamera(fov, W/H, 0.05, 800) | present | PASS |

### D expected

| check | spec (derived) | staged | verdict |
|---|---|---|---|
| basic-3m fogFactor | 0.0 | 0.0 | PASS |
| basic-3m baseColor | #D94F30 | #D94F30 | PASS |
| basic-3m centrePixel | (297, 365) | (297, 365) | PASS |
| basic-3m centreNdc | [-0.45, 0.62] | [-0.45, 0.62] | PASS |
| standard-3m fogFactor | 0.0 | 0.0 | PASS |
| standard-3m baseColor | #30D94F | #30D94F | PASS |
| standard-3m centrePixel | (783, 365) | (783, 365) | PASS |
| standard-3m centreNdc | [0.45, 0.62] | [0.45, 0.62] | PASS |
| basic-6m fogFactor | 0.026283105635930316 | 0.026283105635930316 | PASS |
| basic-6m baseColor | #3F6FD9 | #3F6FD9 | PASS |
| basic-6m centrePixel | (297, 662) | (297, 662) | PASS |
| basic-6m centreNdc | [-0.45, 0.31] | [-0.45, 0.31] | PASS |
| standard-6m fogFactor | 0.026283105635930316 | 0.026283105635930316 | PASS |
| standard-6m baseColor | #D9B83F | #D9B83F | PASS |
| standard-6m centrePixel | (783, 662) | (783, 662) | PASS |
| standard-6m centreNdc | [0.45, 0.31] | [0.45, 0.31] | PASS |
| basic-10m fogFactor | 0.2883756839313887 | 0.2883756839313887 | PASS |
| basic-10m baseColor | #7A8B99 | #7A8B99 | PASS |
| basic-10m centrePixel | (297, 960) | (297, 960) | PASS |
| basic-10m centreNdc | [-0.45, 0.0] | [-0.45, 0.0] | PASS |
| standard-10m fogFactor | 0.2883756839313887 | 0.2883756839313887 | PASS |
| standard-10m baseColor | #8E44AD | #8E44AD | PASS |
| standard-10m centrePixel | (783, 960) | (783, 960) | PASS |
| standard-10m centreNdc | [0.45, 0.0] | [0.45, 0.0] | PASS |
| basic-15m fogFactor | 0.754959551542412 | 0.754959551542412 | PASS |
| basic-15m baseColor | #2FA89B | #2FA89B | PASS |
| basic-15m centrePixel | (297, 1258) | (297, 1258) | PASS |
| basic-15m centreNdc | [-0.45, -0.31] | [-0.45, -0.31] | PASS |
| standard-15m fogFactor | 0.754959551542412 | 0.754959551542412 | PASS |
| standard-15m baseColor | #C94F7C | #C94F7C | PASS |
| standard-15m centrePixel | (783, 1258) | (783, 1258) | PASS |
| standard-15m centreNdc | [0.45, -0.31] | [0.45, -0.31] | PASS |
| basic-25m fogFactor | 1.0 | 1.0 | PASS |
| basic-25m baseColor | #8C8C46 | #8C8C46 | PASS |
| basic-25m centrePixel | (297, 1555) | (297, 1555) | PASS |
| basic-25m centreNdc | [-0.45, -0.62] | [-0.45, -0.62] | PASS |
| standard-25m fogFactor | 1.0 | 1.0 | PASS |
| standard-25m baseColor | #466B8C | #466B8C | PASS |
| standard-25m centrePixel | (783, 1555) | (783, 1555) | PASS |
| standard-25m centreNdc | [0.45, -0.62] | [0.45, -0.62] | PASS |
| fog block | #C9CFC2 4.5/20.0 | #C9CFC2 4.5/20.0 | PASS |
| houseLight dir/ambient | [0.5, 0.8, 0.6] / 0.35 | [0.5, 0.8, 0.6] / 0.35 | PASS |
| houseLight front-face factor | 0.6988266044899671 | 0.6988266044899671 | PASS |

**144/144 checks PASS, 0 FAIL**

## 3. The run

The M3 pattern demonstrably ran headless Chrome on this VM — the
dof-chart's `original.png` was captured here through this same
template and tool, and `../README.md` quotes its renderer string —
so the run was attempted, with the M3 commands verbatim:

```sh
cd tools/fog-chart/chrome/project
bun run build
CHROME=/home/hatch/.faceless-shorts/chrome/opt/google/chrome/chrome \
  node tools/capture.mjs frames review/full "1.0"
```

- Renderer, as printed by the tool itself:
  `ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0x0000C0DE)), SwiftShader driver)`
  — the same software-GL renderer as the M3 reference (this VM has
  no GPU; no performance claim). The capture is a JPEG master
  (OP.frame, q ≈ 0.92), per the package's labelling law.
- Frame: [`project/review/full/t1.00.jpg`](project/review/full/t1.00.jpg)
  — JPEG, 1080×1920, 29,611 bytes, sha256
  `2623e2aef97a441c271be496336cd65ad929c650288a6aa73ce22cf3af86b0d9`.
  The chart is static, so t = 1.0 s is the staging instant (the M3
  convention); the shot's `draw` ignores time.
- One operational note, recorded honestly: the **first** invocation
  failed with `Chrome did not start` — the tool polls 20 s for the
  profile's `DevToolsActivePort` file, and the first-ever launch of
  this Chrome build on the VM exceeded it during initialisation.
  A manual warm-up launch of the same binary wrote
  `DevToolsActivePort` in ~1 s, and the identical capture command
  then succeeded unchanged. Cold-start timeout of the tool's poll,
  not a staging defect.

## 4. Centre measurements (Chrome frame vs analytic)

Sampled with [`sample_centres.py`](sample_centres.py) — the package
README's sampler, extended to print deltas against both expectation
columns (mean over the ±20 px box at each panel's centre pixel):

| panel | sampled mean | expected (fog) | Δ | expected (no fog) | Δ |
|---|---|---|---|---|---|
| basic-3m | 214, 66, 18 | 214, 66, 17 | 0, 0, +1 | 214, 66, 17 | 0, 0, +1 |
| standard-3m | 2, 106, 27 | 9, 182, 54 | −7, −76, −27 | 9, 182, 54 | −7, −76, −27 |
| basic-6m | 44, 102, 210 | 44, 102, 211 | 0, 0, −1 | 33, 99, 212 | +11, +3, −2 |
| standard-6m | 107, 89, 15 | 180, 151, 32 | −73, −62, −17 | 180, 151, 21 | −73, −62, −6 |
| basic-10m | 141, 155, 158 | 141, 154, 158 | 0, +1, 0 | 110, 129, 144 | +31, +26, +14 |
| standard-10m | 115, 107, 118 | 140, 114, 154 | −25, −7, −36 | 111, 26, 140 | +4, +81, −22 |
| basic-15m | 171, 191, 179 | 171, 192, 178 | 0, −1, +1 | 16, 163, 150 | +155, +28, +29 |
| standard-15m | 176, 176, 164 | 188, 178, 170 | −12, −2, −6 | 164, 37, 91 | +12, +139, +73 |
| basic-25m | 195, 201, 187 | 195, 201, 187 | 0, 0, 0 | 130, 130, 42 | +65, +71, +145 |
| standard-25m | 195, 201, 187 | 195, 201, 187 | 0, 0, 0 | 28, 76, 109 | +167, +125, +78 |

Reading:

- **Basic panels — the pure fog measurement: worst |Δ| = 1 code**
  across all five depths. At the three discriminating depths the
  classic-ramp formula would miss by 26 (6 m), 12 (10 m) and 8
  (15 m) codes; the Chrome side lands within 1. The 25 m panel is
  the exact pure-fog colour (195, 201, 187), and the sampled means
  sit far from the no-fog column exactly where fog must act
  (basic-15m: +155 on R vs no-fog). Fog is present, in linear
  space, with three's smoothstep factor — measured, not inferred.
- **Standard panels — a lighting convention, quantified.** The
  deltas are large at factor 0 (3 m: −76 on G, where fog cannot be
  involved), shrink as fog takes over, and are exactly 0 at
  factor 1 (25 m) — the signature of a pre-fog lighting difference,
  not a fog difference. Replacing the house front-face factor
  0.6988266044899671 by **house/π = 0.222429…** in the analytic
  chain (fog mix → Neutral → sRGB OETF, the repo's own
  `src/shader_output.wgsl` transcription) predicts the sampled
  centres at every depth:

  | panel | predicted (house/π) | sampled | residual |
  |---|---|---|---|
  | standard-3m | 1, 107, 27 | 2, 106, 27 | ≤ 1 |
  | standard-6m | 106, 89, 13 | 107, 89, 15 | ≤ 2 |
  | standard-10m | 115, 108, 118 | 115, 107, 118 | ≤ 1 |
  | standard-15m | 176, 176, 165 | 176, 176, 164 | ≤ 1 |
  | standard-25m | 195, 201, 187 | 195, 201, 187 | 0 |

  The effective Chrome factor measures as house/π — consistent
  with three r170's physical light convention (the Lambert BRDF's
  1/π, with no legacy π fold on light intensities, `useLegacyLights`
  having been removed). The ≤ 2-code residual is the declared GGX
  specular at roughness 1 plus JPEG. **Consequence for the package
  owner** (not applied here — this seat writes only under
  `chrome/`): if the Chrome standard panels are ever meant to
  reproduce the house factor rather than merely cross-check, scale
  both rig intensities by π (ambient 0.35π ≈ 1.0996, directional
  0.65π ≈ 2.0420). The fog gate does not depend on it: the gate is
  the native side against the analytic column, and the Chrome
  side's fog evidence is the basic panels.

## 5. What this does not prove

- **The native gate is still unrun.** Both loader prerequisites in
  `../README.md` stand (scenefile fog wiring, FOG.md §8; the `solid`
  texture arm). Until they land and the CI render workflow produces
  the two scene-file renders, there is no native frame, so the
  native-vs-analytic ±2-code verdict and the native-vs-Chrome
  full-chart PSNR (≥ 45 dB bar) remain open — this record closes
  only the Chrome half and the staging-consistency question.
- **The no-fog control is native-side by design** (the harness
  staging always sets `scene.fog`); no Chrome no-fog variant was
  captured. Its role — proving the `fog2.y = 0` skip is exact —
  belongs to the same binary that runs the fog file, per the
  package README.
- The standard-panel π finding is an empirical identification
  (predicted vs sampled, five depths, two exact endpoints), stated
  as measured; it is not a claim about any three.js source line.

## 6. Reproduce

```sh
python3 tools/fog-chart/chrome/verify_constants.py   # 144/144, exit 0
cd tools/fog-chart/chrome/project && bun run build
CHROME=<chrome binary> node tools/capture.mjs frames review/full "1.0"
python3 tools/fog-chart/chrome/sample_centres.py \
  tools/fog-chart/chrome/project/review/full/t1.00.jpg
```
