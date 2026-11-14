# FOG-STATUS-W74 — the loader lane and `scene.fog` at HEAD

Seat W74, 2026-11-14. Tree state: HEAD `3781eb2` (Batch 7 close).
Scope: does the scene-file loader (`src/scenefile.rs`) consume a scene
file's top-level `fog` block at HEAD, what wiring remains, and how the
fog chart accepts it once wired. `src/` read-only for this seat;
nothing here is committed by the wave.

## 1. Verdict: (c) — ignored entirely

A scene file's `fog` block is **not parsed, not stored, and not
applied** on the loader lane. It is not case (b): there is no parsed-
but-unused field anywhere. Evidence, all first-hand at HEAD:

- **The parser never reads the key.** `load()` in `src/scenefile.rs`
  walks the root object key by key — `format` (:817), `version`
  (:822), `time` (:834), `name` (:839), `camera` (:844), `clear`
  (:881), `lighting` (:887), `lights` (:904), `textures` (:967),
  `nodes` (:981), `post.bokeh` (:986), `post.bloom` (:997),
  `post.grade` (:1005). A tree-wide grep for `fog` in
  `src/scenefile.rs` returns exactly two lines, both the `solid`
  texture arm's comment (:433–434). The word does not appear in
  `src/scene.rs` or `src/main.rs` at all.
- **No storage.** `struct Loaded` (`src/scenefile.rs:121–135`) —
  the value every render path consumes — has no fog field. Its
  members are name/version/time/camera/clear/light_dir/ambient/
  lights/textures/draws/bokeh/bloom/grade.
- **No error either.** The loader rejects unknown *sub-field* values
  (transform ops :231, geometry kinds :295/:353, texture generators
  :461, node types :749, …) but never unknown **top-level** keys, so a
  file carrying `fog` loads cleanly and renders unfogged, silently.
  This is exactly the behaviour `tools/fog-chart/expected.json`
  records under `prerequisites`, and it is observable today: the two
  chart files differ only in `name` + `fog` (verified by diff this
  seat) and must render identically until the wiring lands.
- **Nothing downstream could consume it anyway.** `run_scene_file`
  (`src/scenefile.rs:1214–1221`) dispatches on `bokeh.is_some()` to
  `run_composer`, else `run_direct`. Both paths render scene geometry
  with exactly two shaders — `src/shader_scene3d.wgsl` (colour draws;
  pipelines at :1242 direct / :1689 composer) and `src/shader_tex.wgsl`
  (textured draws; :1289 direct / :1645 composer) — and **neither
  shader carries a fog uniform or a fog term**: scene3d's `Uniforms`
  is `{mvp, model, light}`, tex's is `{mvp, model}`, and neither
  vertex stage emits a world/view position varying a fog factor could
  be computed from.

The fog-carrying shaders exist only on the other two lanes:
`src/shader_pbr.wgsl` (`FrameU.fog`/`fog2` :65–66, applied :484–488)
and `src/shader_basic.wgsl` (`DrawU.fog`/`fog2` :26–27, applied
:60–63), driven by `src/film.rs` (`FrameSpec.fog`, e.g. S06 set at
film.rs:1605) and by the hand-authored ab1 path, which writes those
fields **zero/disabled** (`src/ab1.rs:665–671`, :1292–1295).
`src/scenefile.rs` includes neither shader.

### One premise in FOG.md §8 needs correcting

§8 says wiring "needs **no shader change** — only: read the top-level
`fog` → `hex_lin` → the same `FrameSpec.fog`-equivalent the loader
already forwards to the PBR `FrameU` and basic `DrawU` fields". At
HEAD the loader forwards to no such fields: it never touches the
PBR/basic pipelines. The wiring is still small, but it **does**
include shader edits — to the loader's own two shaders (§2 below).
Extracting the film shaders' inline blocks into `fog.wgsl.inc` calls
would not help here either; the loader's shaders need the term added,
in the include's frame-host form. A dated note to this effect is
appended to FOG.md §8.

### Prerequisite status

Of the fog chart's two loader prerequisites (`tools/fog-chart/
README.md`, `expected.json: prerequisites`):

1. **`solid` texture generator — LANDED** (`c74cf0c`): the arm is in
   `resolve_texture` (`src/scenefile.rs:433–434` and the arm it
   introduces). The chart's five basic panels can now load.
2. **Fog wiring — OPEN.** The only remaining prerequisite. Until it
   lands, the native side of the fog-chart gate cannot run: both
   chart files render the unfogged image.

## 2. Remaining wiring — mechanical checklist for the lead

Target semantics are FOG.md §1–§4 and `shaders/fog/fog.wgsl.inc`:
factor = `smoothstep(near, far, viewDepth)` over **forward-axis**
depth, mix in linear space on the fully lit colour, rgb only, and
`fog2.y = 0` must skip the block entirely (bit-identical no-fog path).

1. **Parse** (`src/scenefile.rs`, in `load()`, beside `clear` at
   :881–885 — same shape): on `root.get("fog")`, read
   `{color: "#RRGGBB", near: f32, far: f32}`; decode the colour with
   the existing `hex_lin` (the helper `clear` already uses at :882);
   error context `scene.fog`; reject `near >= far` (smoothstep is
   undefined there, FOG.md §2). Absent block → `None`.
2. **Store**: add `fog: Option<([f32; 3], f32, f32)>` (linear colour,
   near, far — the `FrameSpec.fog` shape, film.rs:343) to
   `struct Loaded` (:121–135) and populate it in the `Loaded`
   construction at the end of `load()`.
3. **Shader — colour path** (`src/shader_scene3d.wgsl`): extend
   `Uniforms` after `light` with the include's contract —
   `fog: vec4f` (rgb linear, w: near), `fog2: vec4f` (x: far,
   y: enabled), plus `cam_pos: vec4f` and `cam_fwd: vec4f`. Emit a
   `world_pos` varying from the vertex stage (`model` is already a
   uniform: `world = model * pos`). At the tail of `fs_main`, apply
   `tr_fog_apply_frame(lit_rgb, world_pos, cam_pos, cam_fwd, fog,
   fog2)` from `fog.wgsl.inc` (paste the helper in; the repo's
   shaders are single-file includes today). This preserves the house
   light exactly — the chart's standard-panel expectations are
   computed from this shader's front-face factor 0.6988266044899671
   (`expected.json: houseLight`), so do **not** reroute the loader
   through the PBR pipeline to get fog.
4. **Shader — textured path** (`src/shader_tex.wgsl`): same uniform
   extension and `world_pos` varying; in `fs_main`, apply the same
   `tr_fog_apply_frame` to the sampled texel's rgb after
   `textureSample`, alpha untouched. No per-draw flag on this lane:
   scene files have no `material.fog` opt-out field (the loader's
   material parse reads texture/roughness/etc., no fog key), and per
   FOG.md §1/§6 the loader's textured draws — character cards
   included — honour scene fog in the source. The scene-level
   `fog2.y` gate is the whole gate here.
5. **Pack** (both run paths): wherever the two pipelines' uniform
   bytes are assembled (shared write helper at `src/scenefile.rs:
   1083`; `light` vec4 built at :1237 in `run_direct` and :1585 in
   `run_composer`), append per draw, in struct order:
   fog present → `[lin_r, lin_g, lin_b, near]`, `[far, 1.0, 0, 0]`;
   absent → two zero vec4s. `cam_pos` = `scene.camera.eye`,
   `cam_fwd` = `normalize(target − eye)` (the film.rs:2786
   derivation). Uniform buffer sizes grow by 64 B per draw on both
   pipelines (scene3d 144 → 208 B; tex 128 → 192 B).
6. **Do not touch**: `src/film.rs`, `src/shader_pbr.wgsl`,
   `src/shader_basic.wgsl`, `src/ab1.rs`, the chart files, or
   `expected.json`.
7. **Regression gates** (fog-chart README, PASS list): with the wired
   binary, ab1 and a noon film frame bit-identical before/after;
   dof-chart unchanged; the no-fog control (§3) reproduces
   `expectedSrgbNoFog` within ±2 codes — that last one is the proof
   the `fog2.y = 0` skip is exact, not approximate.

Side effect to expect, not a bug: the film segment files that already
carry `fog` blocks (`scenes/film/s06/s07/s11/s12.scene.json`, per
FOG.md §8) will render fogged on the loader lane once wired — which
is the point: the loader lane then agrees with the film lane on those
files instead of silently dropping the block.

## 3. Native acceptance procedure (run once §2 lands)

Chart files (note the real location — there is no
`tools/fog-chart/native/` directory): **`scenes/fog-chart.scene.json`**
(fog `#C9CFC2`, near 4.5, far 20) and the control
**`scenes/fog-chart-nofog.scene.json`** (identical apart from `name`
and the omitted `fog` block — diff-verified this seat). The chart
carries `post.bokeh` (aperture 0), so it takes the **composer** path.

### 3.1 Render — CI route (standing route for this repo)

```sh
gh workflow run render.yml -f mode=scene-file -f scene=scenes/fog-chart.scene.json
gh workflow run render.yml -f mode=scene-file -f scene=scenes/fog-chart-nofog.scene.json
gh run download <run-id> -n render-scene-file-0-0-<run-id>   # per run; contains render-out/scene.png
```

(Renderer of record for this evidence: **native CI lavapipe**.)
Local route, on a healthy Vulkan device (user's PC / RADV), with a
locally built binary — defaults are already 1080×1920
(`src/main.rs:38–39`):

```sh
./talerender --scene-file scenes/fog-chart.scene.json     --out fog-chart.png
./talerender --scene-file scenes/fog-chart-nofog.scene.json --out fog-chart-nofog.png
```

### 3.2 Measure — same sampler, same bars as the Chrome side

`tools/fog-chart/chrome/sample_centres.py` is renderer-agnostic: it
takes any 1080×1920 frame, takes the mean over each panel's
±`sampleBoxPx` (20 px) box at `centrePixel` from
`tools/fog-chart/expected.json`, and prints deltas against both
expectation columns.

```sh
python3 tools/fog-chart/chrome/sample_centres.py fog-chart.png        # vs expectedSrgb
python3 tools/fog-chart/chrome/sample_centres.py fog-chart-nofog.png  # vs expectedSrgbNoFog
```

**PASS requires all of** (`expected.json: passBars`):

- fog render: **every** panel centre within **±2 sRGB codes** of
  `expectedSrgb`, per channel, both materials, all five depths;
- control render: every centre within ±2 of `expectedSrgbNoFog`
  with the same binary;
- native-vs-Chrome full-chart **PSNR ≥ 45 dB** against the Chrome
  frame `tools/fog-chart/chrome/project/review/full/t1.00.jpg`
  (Chrome SwiftShader, JPEG) using `tools/ab/measure_ab.py` pair
  mode — subject to the caveat below;
- the §2.7 regressions stay exact.

The formula discriminator is built in: a classic-ramp build lands on
`expected.json`'s `discriminators.classicRampSrgb` values and misses
the ±2 bar by 26/12/8 codes at 6/10/15 m on the basic panels — the
same margins the Chrome run confirmed empirically (worst |Δ| = 1 code
vs analytic at all depths, `tools/fog-chart/chrome/VERIFY.md`).

**The ×π caveat (standard panels, PSNR leg only).** The native
standard panels are a true ±2 gate against `expected.json`: their
expectations are computed from the loader colour shader's own house
factor 0.6988266044899671. The Chrome harness's rig, however,
measures an effective front-face factor of **house/π = 0.222429…**
(three r170 folds 1/π into physical light intensities;
VERIFY.md:333–356). So a native-vs-Chrome centre comparison on the
standard panels carries that known rig factor, and an unadjusted
full-chart PSNR will sag on the standard half without anything being
wrong with fog. For the ≥45 dB leg, either re-capture the Chrome side
with VERIFY.md's ×π rig amendment (ambient 0.35π ≈ 1.0996,
directional 0.65π ≈ 2.0420) or judge the PSNR expectation on the
basic half. **The gate is the basic panels** — the pure fog
measurement, texture colour in, fogged colour out, no lighting term
at all. The fog verdict never rests on the standard centres alone.

**FAIL signatures** are tabulated in `tools/fog-chart/README.md`
(PASS/FAIL section) — notably: fog render matching `expectedSrgbNoFog`
= the block is still being ignored / `fog2.y` never set; 25 m pair ≠
(195, 201, 187) ≈ (224, 227, 221) = fog colour passed as raw sRGB
bytes instead of `hex_lin` linear; one material fogged and the other
not = only one of the two loader pipelines got the term.

## 4. Bottom line

- Loader-lane fog at HEAD: **(c) ignored entirely** — no parse, no
  field, no error, and no fog term in either shader the lane renders
  with. The chart cannot pass natively until §2 lands; its Chrome
  side has already proven the formula and the expectations.
- Remaining work is one parse block, one `Loaded` field, one uniform
  extension + tail application in each of `shader_scene3d.wgsl` /
  `shader_tex.wgsl`, and the matching packing in `run_direct` /
  `run_composer` — checklist in §2, acceptance in §3.
