# M5 gaps — every approximation, with its measured residual

The §28 work orders (docs/ab/onoda-s01-t150/gaps.md items 1–2)
are closed by this milestone: the composer exists and the light
rig is the production rig. What remains is approximation, not
absence. Residuals are measured on the Onoda S01 t=1.50 frame
unless stated otherwise.

1. **PMREM environment is captured data, not regenerated.**
   `PMREMGenerator.fromScene(RoomEnvironment, 0.04)` was run
   once in headless Chrome and its 768×1024 float16 CubeUV
   target ships verbatim (`assets/room-env-pmrem.f16`, copy in
   this package). The native side samples it with a verbatim
   textureCubeUV port, so the only divergence from production is
   the capture itself (bit-exact float16 readback) plus GPU
   filtering arithmetic. Measured residual: up-facing,
   roughness-≈1 surfaces — the most IBL-sensitive class in the
   shot — land at region ratios 1.002–1.048 (dirt bottom,
   canopy top, grass); no region shows an IBL-shaped error.
   A native PMREM generator remains future work; it is no
   longer on the critical path for parity.
2. **GRADE grain is GPU-vendor sensitive.** The engine's grain
   hash uses `sin()` in fp32; lavapipe and a hardware GPU
   (RADV) will not produce identical hash values frame-wide.
   The term is bounded by construction (amplitude ±0.006 of
   full scale → a ≈ 49 dB PSNR ceiling if it decorrelated
   entirely); the shipped frame's total error (mean |Δ| 5.2)
   is far above that floor, so grain is not the limiting term
   here. Deterministic per t on a given GPU, as the engine's
   formula requires.
3. **Trunk sides run 6–9 % bright** (region ratio 1.06–1.09,
   the largest remaining regional bias). Cylinder side normals
   are the most grazing key-light geometry in the shot; the
   residual is consistent with the geometry-roughness term and
   12-segment cylinder faceting interacting with PCF-soft
   shadow edges. Not chased further at M5: it is a ~1 dB-class
   term inside a 23.85 dB frame, and the bias is uniform rather
   than structured.
4. **No texture mipmapping or anisotropy** (carried from §28).
   Samplers are Linear/Clamp-or-Repeat with no mip chains; the
   ground and hill maps alias slightly at grazing angles where
   three samples its mipmapped CanvasTextures. Visible in the
   diff as fine noise across the grass band; part of the
   remaining 13.5 % px > 10.
5. **Painter/texture approximations carried from §28/M4**
   (grass blade pattern, cloud sprite painting, blob shadow)
   are unchanged — the M4 gate owns painter fidelity; their
   residuals are inside the numbers above.
6. **Scene format v1 cannot express the light rig.** The
   format's scenes still render with the single house
   directional (docs/scene-format.md §12 item 3, updated):
   `post.bloom`/`post.grade` arrived at M5, but key/rim/
   hemisphere/PMREM/point lights and shadow configuration are
   hand-authored-path only. A schema decision for a future
   format version, declared rather than silently fielded.
7. **Lavapipe-only evidence.** All renders here are software
   Vulkan. The HalfFloat depth variant note from M3 stands
   (shipped depth storage is Rgba8Unorm; RADV fp16 rounding
   remains unverified on hardware).
