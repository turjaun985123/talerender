# M5 — Composer chain: lighting, shadows, bloom, GRADE

**Verdict: PASS.** The §28 A/B named two gap classes as its
largest: (1) no composer in the native scene path, (2) a flat
lighting/shadow model. Both are closed. Re-rendering Onoda S01 at
t = 1.50 through the full native chain (RenderPass → BokehPass →
UnrealBloomPass → OutputPass → GRADE, HalfFloat MSAA-4×) scores
**23.850 dB** against the committed production original — above
the §28-as-shipped baseline (20.152 dB) and the post-M4 painter
baseline (20.218 dB) by **+3.63 dB**, with channel means within
+1.0 of the original on every channel.

- `original.png` — the production frame (three.js r170, headless
  Chrome), copied verbatim from `docs/ab/onoda-s01-t150/`.
- `native.png` — the full native chain (this package's render).
- `stage-lighting.png`, `stage-bloom.png` — the ablation stages.
- `side-by-side.png` — original left, native right, 540×960 each.
- `diff.png` — |Δ| × 4, clamped.
- `room-env-pmrem.f16` — the captured PMREM environment (below).
- `project/` — the Chrome harness that captured it.
- `metrics.json` — every number in this README, machine-readable.

Renderer labels: original = production engine in Chrome;
everything else = talerender on lavapipe (software Vulkan —
correctness evidence only, no performance claim).

## Ablation (expectations stated first)

Expectation, set before measuring: the lighting stage must beat
the 20.218 dB flat-lighting baseline on its own (it replaces the
model the baseline used), bloom should be near-neutral at this
shot's strength (0.16, threshold 0.85 — only the fire qualifies),
and GRADE should buy the most (the original carries vignette,
saturation 0.8 and warmth that no earlier native stage applied).

| stage | PSNR | mean \|Δ\| | px > 10 |
|---|---|---|---|
| 1 lighting+shadows (+bokeh, +OutputPass) | 21.359 dB | 13.228 | 71.5 % |
| 2 + bloom | 21.421 dB | 13.027 | 69.8 % |
| 3 + GRADE (full chain) | **23.850 dB** | **5.201** | **13.5 %** |

Every stage is non-negative; GRADE buys +2.43 dB as predicted,
bloom +0.06 dB (small because the shot is mostly below the 0.85
threshold — the fire's glow is present in both frames).

## What was built

- **Standard-material shader** (`src/shader_pbr.wgsl`): a
  transcription of three r170's physical lighting — GGX direct
  specular, Lambert diffuse, hemisphere irradiance, punctual
  point-light falloff (distance/decay law), and PMREM IBL
  (irradiance + radiance with the DFG approximation), with
  MIT attribution headers. Constants cited from the skill's
  engine/world sources: key #FFF0D6 × 2.6 at (7, 12, 5) → target
  (0, 0, −2); rim #FFFFFF × 0.4 at (−3, 3, −3); hemisphere
  #CFE6FF/#C48A5A × 1.25; fire point light #FF9A3C, distance 7,
  decay 1.6, intensity 4.3519 at t = 1.50 (the engine's fireLevel
  formula, evaluated in code); environment intensity 0.25.
- **Shadow map**: 2048², ortho ±17 / 0.1 / 80, bias −0.0006,
  normalBias 0.02, PCF-soft 9-tap transcription; the character
  card casts through an alpha-tested depth material
  (alphaTest 0.5), exactly the engine's customDepthMaterial law.
  The palm-frond shadow across the dirt — the §28 package's named
  missing feature — is present in the native frame.
- **PMREM environment, captured not re-derived**: an exact
  PMREMGenerator reproduction was judged impractical to verify,
  so the real generator's output was captured in headless Chrome
  (`project/env-capture.html`) as raw float16 (768×1024, the
  CubeUV layout) and ships as `assets/room-env-pmrem.f16`; the
  native shader samples it with a verbatim textureCubeUV port.
  The binary re-derives the capture's statistics at load and a
  unit test pins them (max 56.3125, meanG 0.67713).
- **Bloom** (`src/composer.rs` + bloom shaders): UnrealBloomPass
  transcribed — luminosity high-pass (threshold + 0.01 smooth
  width), 5 mips at the addon's rounded-half sizes, separable
  Gaussian kernels [3, 5, 7, 9, 11] with the addon's coefficient
  formula, composite with the radius-mixed factors, and the
  additive CopyShader blend (SrcAlpha, One).
- **GRADE** (`src/shader_grade.wgsl`): the engine's shader
  transcribed line for line — vignette, hash grain seeded by
  uTime (deterministic per t, matching the engine's formula),
  warmth, saturation — applied after OutputPass, in sRGB space,
  as production does. The caption composites after GRADE, also
  as production does.
- **Hills carry their map**: diagnosis (below) found the hills —
  not the light rig — were the largest single error. They now
  render as mapped standard draws (vertex colour × grass map,
  repeat [6, 3]), per `world.ts` hills().
- **Scene format**: `post.bloom` / `post.grade` join
  `post.bokeh` (docs/scene-format.md §9), with engine-mirroring
  per-field defaults; absent blocks leave the chain untouched.

## Diagnosis trail (kept because it changed the work)

The first full-chain render scored 21.54 dB with the *lighting*
stage at 19.40 dB — below the flat-lighting baseline, with green
+17.6 high in the channel means. Region sampling separated the
causes: sky/dirt deltas were the ablation's own missing GRADE
(vignette darkens the original's corners 0.78–0.84×); the hills
measured 1.65–2.98× too bright because the §28 build had
substituted flat colours for their grass map (colour × map is
much darker than either alone). After the map fix, every sampled
region lands within 1.00–1.05 of the original except trunk sides
(1.06–1.09, see gaps.md) — the light rig itself needed no
correction: hemisphere folding, the PMREM sampling and the fire
falloff were all verified against the frame, not assumed.

## Regression proofs

- **dof-chart gate: bit-identical** to the M3 committed native
  (PSNR ∞); the M3 verdict stands untouched.
- **dof-chart as-data: bit-identical** to the M2 committed
  reference (PSNR ∞).
- **Identity test**: the dof chart with `post.bloom` strength 0
  and a neutral `post.grade` renders **bit-identical** to the
  plain chain — the new wiring is a true no-op when inert.
- **ab1 as-data vs M2 committed: 39.280 dB** (mean |Δ| 0.325,
  0.771 % px > 10, concentrated in the grass band). Bisected:
  the M4 CI binary renders the same scene file **bit-identical**
  to the M5 binary, so the drift entered with M4's painter
  rework (which re-based painters the format's textures use) and
  M2's committed reference predates it. M5 changed nothing in
  the direct path; re-committing that reference is an M2-package
  maintenance item, flagged here rather than silently done.

## Remaining divergences

See `gaps.md` — every approximation with its measured residual.
Headlines: the PMREM is captured data (residual bounded by the
region table in metrics.json — up-facing rough surfaces within
~5 %); GRADE grain is GPU-sine-hash sensitive (bounded ±0.006,
PSNR ceiling ≈ 49 dB); trunk sides run 6–9 % bright; texture
mipmapping/anisotropy and the tuft/painter approximations are
carried from §28 unchanged.
