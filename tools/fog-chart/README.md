# Fog chart — the fog verification package (FOG.md §7)

The dedicated chart for three.js `scene.fog` in the native renderer, in
the spirit of the M3 dof-chart: one shared spec driving both engines,
flat panels whose expected centre colours are known **analytically** —
computed from the formulas in [`expected.json`](expected.json), not
from either renderer. It exists to prove the one formula that is easy
to get wrong: three's linear-`Fog` factor is
`smoothstep(near, far, depth)`, **not** the classic fixed-function
ramp (FOG.md §2), and the mix happens in linear space, pre-tonemap,
on every fog-honouring material.

## Status — two loader prerequisites

The film path (`src/film.rs`) already wires fog (FOG.md §0). The
**scene-file path does not yet**: at the time this package was
written, rendering either chart file produces the *unfogged* image.
Two small loader changes are prerequisites, both specified exactly:

1. **Fog wiring** — FOG.md §8: `src/scenefile.rs` reads the top-level
   `fog {color, near, far}` block → `hex_lin(color)` → the same fog
   uniforms the PBR/basic shaders already carry, `fog2.y = 1`. Absent
   block → the bit-identical no-fog path.
2. **A `solid` texture generator** — one arm in `resolve_texture`:
   a flat RGBA buffer (64×64 is plenty) filled with the entry's
   `color` sRGB bytes, uploaded `Rgba8UnormSrgb` like every scene
   texture. The five basic panels' textures use it. (No existing
   generator can produce a flat colour: `grass`/`cloud`/`blob-shadow`/
   `sky-gradient` are all patterned.)

Until (1) lands, this package's own control file is the diagnostic:
both files render identically, and their centres match the
`expectedSrgbNoFog` column.

## Files

| file | what it is |
|---|---|
| `../../scenes/fog-chart.scene.json` | the chart as a format v2 scene (fog block present) |
| `../../scenes/fog-chart-nofog.scene.json` | the regression control — byte-identical except `name` and the omitted `fog` block |
| `expected.json` | analytic expectations: per-panel factors, linear values, expected sRGB with and without fog, centre pixels, discriminator + spot-check tables, pass bars |
| `chrome/shots.ts` + `chrome/script.json` | the Chrome harness project (template drop-in, M3 pattern) |
| `README.md` | this file |

## The chart

Camera at the origin looking down −Z, fovY 40, near 0.05, far 800 —
the engine's camera defaults with the chart's fov. Scene fog is the
S06 override: **`#C9CFC2`, near 4.5, far 20**. Post is the film's
composer with **aperture 0** (bokeh disabled per engine.ts's enable
rule) and no bloom/grade — the output stage is Neutral tone mapping
(exposure 1) + sRGB OETF, exactly `src/shader_output.wgsl`. The
`clear` colour is the fog colour itself, in both files.

Ten panels, fronto-parallel, centred on the camera axis rows at
forward-axis depths **3, 6, 10, 15, 25 m**. Each depth in two
materials:

- **basic** (left, NDC x −0.45) — unlit: a textured quad whose
  texture is one flat colour (`solid` generator ↔ Chrome
  `MeshBasicMaterial` with a flat sRGB canvas texture). Its pre-fog
  colour is exactly the colour's linear decode — the purest fog
  measurement in the chart.
- **standard** (right, NDC x +0.45) — a 0.1 m box in the colour
  pipeline, seated so its **front face is at exactly the stated
  depth** (centre at d + 0.05). On the native side the colour
  pipeline is the scene loader's house light
  (`src/shader_scene3d.wgsl`): `col × (0.35 + 0.65 × max(dot(n, L), 0))`,
  `L = normalize(0.5, 0.8, 0.6)` — for the front faces
  (n = (0,0,1)) the factor is **0.6988266045**. This is the loader
  path's "standard" material, *not* film.rs's full PBR port; the
  chart gates the loader path, whose fog term is the shared include
  of FOG.md §3. (The film path's fog gets its own evidence from the
  in-film S06 A/B, FOG.md §7.)

Panels sit at fixed NDC (rows y +0.62 / +0.31 / 0 / −0.31 / −0.62)
with size 0.62 × 0.30 of the frustum half-extents at their depth, so
every panel measures ~335 × 288 px on screen at every depth — the
dof-chart's screen-constant property. Because fog depth is *axis*
depth (−mvPosition.z), a fronto-parallel panel's factor is uniform
across its face: any within-panel gradient at a centre is a bug
signal, not perspective. A dark ground box (top at y = −8) closes
the bottom of the frame; it is scenery, not gated (its depth varies
across its surface).

Colours are distinct per panel (not §7's single `#7A8B99` — that
colour is kept at basic-10m) so a depth or material mix-up is
visible at a glance: with one shared colour, two swapped panels
would be indistinguishable. All panel colours keep their
post-OutputPass peak below Neutral's 0.76 compression threshold
(checked when `expected.json` was generated), so the tone mapper
contributes only its fixed offset subtraction — the fog term is
what moves the numbers.

### Expected factors and centre colours

Factor = `smoothstep(4.5, 20, depth)`. Expected sRGB = the analytic
chain of `expected.json` (fog mix in linear → Neutral @ exposure 1 →
sRGB OETF, exponent 0.41666 → round). Centre = the pixel to sample
(mean over ±20 px).

| panel | centre px | factor | expected (fog) | expected (no fog) |
|---|---|---|---|---|
| basic-3m `#D94F30` | (297, 365) | 0.0000 | 214, 66, 17 | 214, 66, 17 |
| standard-3m `#30D94F` | (783, 365) | 0.0000 | 9, 182, 54 | 9, 182, 54 |
| basic-6m `#3F6FD9` | (297, 662) | 0.0263 | 44, 102, 211 | 33, 99, 212 |
| standard-6m `#D9B83F` | (783, 662) | 0.0263 | 180, 151, 32 | 180, 151, 21 |
| basic-10m `#7A8B99` | (297, 960) | 0.2884 | 141, 154, 158 | 110, 129, 144 |
| standard-10m `#8E44AD` | (783, 960) | 0.2884 | 140, 114, 154 | 111, 26, 140 |
| basic-15m `#2FA89B` | (297, 1258) | 0.7550 | 171, 192, 178 | 16, 163, 150 |
| standard-15m `#C94F7C` | (783, 1258) | 0.7550 | 188, 178, 170 | 164, 37, 91 |
| basic-25m `#8C8C46` | (297, 1555) | 1.0000 | 195, 201, 187 | 130, 130, 42 |
| standard-25m `#466B8C` | (783, 1555) | 1.0000 | 195, 201, 187 | 28, 76, 109 |

Sanity built into the table: the 3 m pair is *before* the fog near
plane (factor 0 — fog must not touch it), and the 25 m pair has
saturated (factor 1) — both materials converge to the identical
output **195, 201, 187**, the pure fog colour through the
OutputPass. If those two don't agree with each other, the rig is
wrong before fog is even discussed.

### The formula discriminator

The classic ramp `clamp((depth − near)/(far − near))` differs from
three's smoothstep by up to 0.078 in factor. On this chart's panels
that is worth, worst channel per depth:

| depth | smoothstep | classic ramp | Δ factor | worst Δ (codes) |
|---|---|---|---|---|
| 6 m | 0.0263 | 0.0968 | 0.0705 | 27 (standard) / 26 (basic) |
| 10 m | 0.2884 | 0.3548 | 0.0664 | 12 (standard) / 6 (basic) |
| 15 m | 0.7550 | 0.6774 | 0.0776 | 8 (both) |

Per-panel classic-ramp predictions are in `expected.json`
(`discriminators`) — a wrong-formula build doesn't fail vaguely, it
lands on those numbers.

### Spot checks (secondary)

For a basic `#7A8B99` panel at 60 m under the film's other two fog
ranges (factors from FOG.md §7): golden `#FFD98A` 30/140 →
factor 0.1826 → **149, 148, 141**; dusk `#F59C6B` 28/120 →
factor 0.2788 → **161, 134, 132**. (Analytic reference values; no
chart file is committed for them — the S06 range is the gate.)

## How the verification runs

### Native side (the gate)

Via the CI render workflow (`.github/workflows/render.yml`,
`workflow_dispatch`, mode `scene-file`) — this VM's llvmpipe JIT
segfaults on the PBR shader family, and CI lavapipe is the standing
render route for this repo:

```sh
gh workflow run render.yml -f mode=scene-file -f scene=scenes/fog-chart.scene.json
gh workflow run render.yml -f mode=scene-file -f scene=scenes/fog-chart-nofog.scene.json
# download both artifacts (render-out/scene.png each), then measure centres
```

or with a locally built binary on a healthy Vulkan device (the
user's PC / RADV is the production target):

```sh
VK_ICD_FILENAMES=$(ls /usr/share/vulkan/icd.d/lvp_icd*.json | head -1) \
  ./talerender --scene-file scenes/fog-chart.scene.json --out fog-chart.png
VK_ICD_FILENAMES=$(ls /usr/share/vulkan/icd.d/lvp_icd*.json | head -1) \
  ./talerender --scene-file scenes/fog-chart-nofog.scene.json --out fog-chart-nofog.png
```

### Chrome side (reference b)

Follow the M3 pattern (`docs/ab/dof-chart/README.md`, Reproduce):
copy the skill template, drop in `chrome/shots.ts` + `chrome/script.json`
(+ empty `data/lines.json`, `data/words.json`), `bun run build`, then

```sh
node tools/capture.mjs frames review/full "1.0"
```

The harness builds the identical chart through the template's real
engine/composer (`scene.fog` set from the same constants; the look
neutralised; aperture 0). Its standard panels use a three.js rig —
AmbientLight 0.35 + DirectionalLight 0.65 from the house light's
direction — chosen to reproduce the native front-face factor
0.6988266045; three's GGX specular at roughness 1 is a declared
residual on those four panels' centres, so the Chrome comparison is
judged by full-chart PSNR and by the *basic* panels' centres, never
by the standard centres alone. Renderer, as printed by the tool
itself at M3: Chrome `ANGLE (Google, Vulkan 1.3.0 (SwiftShader
Device (Subzero)))`; its master is JPEG (q ≈ 0.92) — judge centres,
not edges.

### Measuring

Panel-centre means against `expected.json`. A minimal sampler:

```python
import json
from PIL import Image
exp = json.load(open('tools/fog-chart/expected.json'))
img = Image.open('fog-chart.png').convert('RGB')
for p in exp['panels']:
    x, y, b = p['centrePixel']['x'], p['centrePixel']['y'], p['sampleBoxPx']
    box = img.crop((x - b, y - b, x + b + 1, y + b + 1))
    px = list(box.getdata())
    mean = [round(sum(c[i] for c in px) / len(px)) for i in range(3)]
    want = p['expectedSrgb']
    ok = all(abs(m - w) <= 2 for m, w in zip(mean, want))
    print(p['id'], mean, 'want', want, 'PASS' if ok else 'FAIL')
```

## PASS / FAIL

**PASS** requires all of:

- every fog-render panel centre within **±2 sRGB codes** of
  `expectedSrgb`, per channel, both materials, all five depths;
- the no-fog control's centres within ±2 of `expectedSrgbNoFog`
  with the same binary (proves the skip is exact, not approximate);
- native-vs-Chrome full-chart PSNR **≥ 45 dB** (the Chrome side's
  JPEG floor is the bound, as at M3);
- the standing regressions stay exact: ab1 and a noon film frame
  bit-identical before/after the wiring; dof-chart unchanged.

**FAIL signatures** — each points at a specific defect:

| symptom | defect |
|---|---|
| fog render's centres match `expectedSrgbNoFog` | fog block ignored / `fog2.y` never set (prerequisite 1 missing) |
| 6/10/15 m centres land on the `classicRampSrgb` values in `expected.json` | classic-ramp formula instead of three's smoothstep |
| 25 m pair ≠ 195, 201, 187 — e.g. ≈ 224, 227, 221 | fog colour passed as raw sRGB bytes instead of `hex_lin` linear |
| basic panels right, standard panels unfogged (or vice versa) | fog wired into only one of the two pipelines (tex vs colour) |
| 3 m pair differs between the two files | fog leaking into the factor-0 zone — depth term wrong (distance instead of axis depth shows up off-axis first; check corners) |
| no-fog control off by a code or two everywhere | the `fog2.y = 0` skip isn't a skip (a mix at factor ~0 with a wrong colour still moves values) |
| centres right but panel edges bleed colour | texture filtering at borders — expected, not a failure: centres are the gate (FOG.md §9) |

## Notes and declared deviations from FOG.md §7

- §7 gives the panels one shared colour (`#7A8B99`); this package
  uses a distinct colour per panel (that colour survives at
  basic-10m) so depth/material swaps are visible — the measurement
  is unchanged, since each panel's expectation is computed from its
  own colour.
- §7's "panel size ~1.5 m" is realised as screen-constant NDC size
  (0.38 m wide at 3 m … 3.17 m at 25 m): centre colour is
  size-independent for fronto-parallel panels, and constant screen
  size keeps every centre measurable at the same pixel scale — the
  same trade the dof-chart made.
- The v2 `lights` rig block is deliberately **omitted** from the
  chart files: still renderers carry it unconsumed (spec §14.2),
  while the `lighting` block is what the colour shader actually
  reads. The chart records the consumed values, explicitly, at
  their house defaults.
- Fog on the character cards and the two-colour rule (scene fog
  `#C9CFC2` vs tint `#E6E9EA`, FOG.md §6) are **not** exercised
  here — no character is in the chart. That interaction is covered
  by the film path's S06 work, not by this gate.
