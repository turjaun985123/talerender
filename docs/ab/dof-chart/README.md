# DoF chart A/B — the Milestone 3 bokeh gate (compatibility contract §1)

**Verdict: PASS.** The WGSL transcription of three.js r170's BokehShader
reproduces the reference blur-vs-depth curve within **0.05 px** (step
edge, worst plane) / **0.12 px** (checkerboard median, worst plane) at
every one of the ten chart depths, against a stated tolerance of
max(0.75 px, 15% of the reference width). Full-frame PSNR 47.38 dB,
mean |Δ| 0.242 per channel. One real divergence was found on the way —
not in the shader arithmetic but in packed-depth *storage* — and is
documented with its evidence in [gaps.md](gaps.md); the shipped native
configuration avoids it deliberately.

## The pair

| file | what it is | renderer (as printed by the tool itself) |
|---|---|---|
| `original.png` | the chart through the **real engine**: skill-template project (`project/shots.ts`) running the production composer chain, captured with `tools/capture.mjs frames` at t = 1.0 s | Chrome `ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero)))` — software GL on this VM. PNG converted from the capture's JPEG (OP.frame, q ≈ 0.92) |
| `native.png` | `talerender --scene dof-chart` (this repo, commit `4db3689`) | wgpu on `llvmpipe (LLVM 20.1.2, 256 bits)` (lavapipe) — software Vulkan. Lossless PNG |

No performance claim is made for either side; both are software
rasterisers on a GPU-less VM.

The reference look neutralises everything except the bokeh pass
(`bloom: 0, exposure: 1, vignette: 0, grain: 0, warm: 0, sat: 1`), so
the chain under test is RenderPass → BokehPass → OutputPass, with
UnrealBloomPass adding exactly 0 and GRADE an exact identity. The
native side reproduces the same stages: HalfFloat MSAA-4× scene target
→ packed-depth prepass → bokeh composite (HalfFloat) → Neutral
tone mapping + sRGB OETF output stage.

## The chart (one spec, two implementations)

Fixed camera at the origin looking down −Z; fov 30, near 0.05, far 800
(the engine's `camera(30)` defaults). Focus **10 m**, aperture
**0.0015**, maxblur **0.006** (the production per-frame defaults,
engine.ts). Ten fronto-parallel panels textured with one procedural
pattern; geometry and texture derive from the same constants on both
sides (`project/shots.ts` and `src/dof.rs`):

- grid (NDC x ∈ {−0.62, 0, +0.62}, NDC y ∈ {+0.66, 0, −0.66}), panel
  size 0.56 × 0.36 NDC (≈302 × 346 px). Depths by row: 30 / 14 / 11 m;
  12 / **10 (focus)** / 8 m; 7 / 5 / 3.5 m.
- a tenth panel at **2 m** (NDC (0, −0.30), height 0.30) overlaps the
  focal panel's bottom strip: the foreground-bleed case (the shader
  gathers with the centre pixel's depth only — no per-tap depth test).
- pattern (function of u, w-down, evaluated identically in f64 on both
  sides, 512×512, single mip level, linear sampling): white frame;
  checkerboard for u < 0.5 (0.1 cells, #EEEEEE / #141414); #808080
  separator 0.5–0.58; one big step edge at u = 0.79.

Expected `dofblur = clamp((focus − d) · aperture, ±maxblur)`:
2–5 m → +0.006 (clamped), 7 m → +0.0045, 8 m → +0.003, 10 m → 0,
11 m → −0.0015, 12 m → −0.003, 14–30 m → −0.006 (clamped).

## Transcription map

three.js is MIT-licensed; the WGSL files carry attribution headers
(BokehShader credited there to Martins Upitis's GLSL original).

| three r170 source | native |
|---|---|
| `BokehShader.js` fragment: `factor = focus + viewZ`; `dofblur = clamp(factor·aperture, −maxblur, maxblur)`; rings ×1.0/×0.9/×0.7/×0.4; 41 taps; `col/41.0`, alpha 1.0 | `src/shader_bokeh.wgsl` — tap-for-tap, same order, same constants |
| `packing.glsl.js`: `unpackRGBAToDepth` = `dot(v, UnpackFactors4)`, UnpackFactors4 = (255/256 / (1, 256, 65536), 1/16777216) | `unpack_rgba_to_depth()` in shader_bokeh.wgsl, constants expanded from the same expressions |
| `packing.glsl.js`: `perspectiveDepthToViewZ(d, n, f) = (n·f)/((f−n)·d − f)` (PERSPECTIVE_CAMERA = 1) | `get_view_z()` in shader_bokeh.wgsl |
| `MeshDepthMaterial` + `RGBADepthPacking`: fragment writes `packDepthToRGBA(fragCoordZ)`; `fragCoordZ = 0.5·zw/w + 0.5` | `src/shader_depth.wgsl`: `pack_depth_to_rgba(@builtin(position).z)` — wgpu's [0,1] depth is the same number as GL fragCoordZ; GLSL `modf` emulated with floor/fract (all inputs ≥ 0) |
| `BokehPass.js`: depth RT HalfFloat + NearestFilter; white clear (1,1,1,1) before the override render; uniforms focus/aperture/maxblur; aspect = camera.aspect; near/far refreshed per render | depth prepass with white clear, nearest sampler on read, uniforms {focus 10, aspect 1080/1920, aperture 0.0015, maxblur 0.006, near 0.05, far 800}. **Storage format deviates on purpose** — see gaps.md #1 |
| `engine.ts`: composer RT HalfFloat, samples = 4 | scene target Rgba16Float, MSAA 4× + resolve |
| `OutputPass`: NeutralToneMapping (exposure 1) then sRGBTransferOETF | `src/shader_output.wgsl` — both chunks transcribed line-for-line, including the 0.41666 exponent |
| orientation: vUv origin bottom-left (GL) vs top-left (wgpu) | immaterial here: one uv serves colour, depth and output, and the 41-tap set is symmetric under u→−u and v→−v |

Fidelity checks that run in CI (`cargo test`): the CPU mirrors of
pack/unpack recover all ten chart depths to < 0.05 m and reproduce the
dofblur table above. A curiosity both sides share: for fragZ ∈
[0.5, 1) — the whole chart band — fp32 `v · 2²⁴` is always integral,
so the pack's fourth channel (`af`) is identically 0; depth resolution
lives in the three byte channels, which is exactly why gaps.md #1
matters.

## Measurement

`scripts/measure_dof.py` (mirrors the §28 package conventions:
side-by-side at half width, diff ×4, same metrics keys). Edge spread =
10–90% transition width of row-averaged profiles: the u = 0.79 step
edge (primary) and the checkerboard vertical edges (median of ~40 per
panel). "Kernel prediction" = the discrete 41-tap kernel's own step
response; both renders sit above it at small blur because both carry
the same pre-bokeh baseline width (≈0.8–1.6 px from ~1.7× texture
minification with single-level linear sampling + MSAA resolve —
measured with bokeh disabled on both sides and identical to ≤0.04 px,
which validates the chart construction itself).

### Blur table (step-edge width, px)

| depth | dofblur | reference | native | Δ | kernel pred. | checker ref / nat | step-zone PSNR |
|---|---|---|---|---|---|---|---|
| 30 m | −0.006 | 4.33 | 4.38 | +0.04 | 4.29 | 4.08 / 4.07 | 54.4 dB |
| 14 m | −0.006 | 4.23 | 4.27 | +0.04 | 4.29 | 4.09 / 4.07 | 56.6 dB |
| 12 m | −0.003 | 2.71 | 2.71 | 0.00 | 2.17 | 2.29 / 2.26 | 50.8 dB |
| 11 m | −0.0015 | 1.71 | 1.70 | −0.02 | 1.29 | 1.81 / 1.88 | 55.2 dB |
| **10 m (focus)** | 0 | 1.29 | 1.33 | +0.04 | 0.80 | 0.81 / 0.80 | 55.7 dB |
| 8 m | +0.003 | 2.25 | 2.26 | +0.01 | 2.17 | 2.40 / 2.52 | 57.7 dB |
| 7 m | +0.0045 | 3.44 | 3.39 | −0.04 | 3.21 | 3.32 / 3.35 | 52.8 dB |
| 5 m | +0.006 | 4.23 | 4.27 | +0.04 | 4.29 | 4.09 / 4.07 | 56.6 dB |
| 3.5 m | +0.006 | 4.11 | 4.07 | −0.04 | 4.29 | 4.20 / 4.23 | 56.4 dB |
| 2 m (bleed) | +0.006 | 4.23 | 4.27 | +0.04 | 4.29 | 4.09 / 4.07 | 56.6 dB |

Foreground bleed: ref-vs-native RMSE in the focal-panel strip within
one maxblur band above the 2 m panel's top edge is **0.658**, *below*
the control strip's 1.056 — both at the reference's JPEG noise floor.
The shader's signature no-depth-test bleed is reproduced, not fixed.

## Reproduce

```sh
# native (CI-built binary or cargo build --release)
VK_ICD_FILENAMES=$(ls /usr/share/vulkan/icd.d/lvp_icd*.json | head -1) \
  ./talerender --scene dof-chart --out native.png
# reference: copy the skill template, drop in project/shots.ts +
# project/script.json (+ empty data/lines.json, data/words.json),
# bun run build, then
node tools/capture.mjs frames review/full "1.0"
python3 scripts/measure_dof.py
```

Regression: with the same binary, `--scene m1` and `--scene ab1` were
re-rendered locally after the change and produce their usual frames;
CI's own M1 smoke render is green.
