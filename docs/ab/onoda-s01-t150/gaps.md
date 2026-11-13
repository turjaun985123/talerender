# Gaps — Onoda S01 @ t = 1.50, native vs original

Every substitution and known difference between the native frame and
the shipped pipeline's frame, each mapped to the audit finding or
compatibility-contract clause it belongs to. Nothing here was
discovered by accident after the fact — these are the deltas the
re-authoring knowingly carries. Metrics in
[metrics.json](metrics.json) measure their combined effect.

## The three largest

1. **The composer chain is absent — no DoF, no bloom, no GRADE, no
   tone mapping.** The original runs RenderPass → BokehPass (focus
   6.051 m, aperture 0.0012, maxblur 0.007) → UnrealBloomPass (0.16) →
   OutputPass (Neutral tone mapping) → GRADE (sat 0.8, warm 0.04,
   vignette 0.24, grain 0.012). The native frame has none of it: the
   background trees and hills are sharp where the original softens
   them, the campfire flame has no bloom halo (it reads pastel-pink
   in the original, flat orange here), colours are un-graded (the
   original is desaturated to 0.8 and warmed), there is no vignette
   and no grain. → Audit A1 (composer inventory; Bokeh named the #1
   parity risk); contract §1 (DoF is M3-gated: the r170 BokehShader
   arithmetic and the DoF chart test come before any production
   scene port); ADR-001 defers the composer to M5.
2. **The lighting/shadow model.** The original: a shadow-casting key
   (2048² PCFSoft) + rim + hemisphere, PMREM RoomEnvironment IBL on
   every standard material (environmentIntensity 0.25), the fire's
   animated PointLight (intensity 4.3519 at this instant), and the
   character plane casting a real shadow through its alphaTest-0.5
   depth material — the soft tree shadows across the grass and
   Onoda's ground shadow are structural features of the original
   frame. The native renderer has ONE fixed directional light
   (direction (0.5, 0.8, 0.6), ambient 0.35) in its M1 shader, no
   shadow maps, no hemisphere, no IBL, no point light, no emissive.
   The only ground shadow under Onoda is his painted blob shadow.
   → Audit A1 (light rig + renderer config); risk register R3
   (PBR/PMREM IBL) and R7 (PCFSoft); ADR-001 defers shadow maps to M5.
3. **Character painter fidelity.** The original character is drawn by
   Skia (Canvas2D in Chrome): stroked paths with round caps/joins,
   bezier bean body, even-odd head shading, then uploaded as a
   2560×2816 CanvasTexture with anisotropy 4 and mipmaps, and
   alpha-tested at 0.04 (colour) / 0.5 (shadow). The native card is
   painted by this repo's small CPU rasteriser (per-pixel ellipse /
   capsule-stroke / polygon evaluation at the same 2560×2816) —
   geometry and colours are the extracted values, but stroke edge
   profiles, AA and joins differ at the pixel level, the texture has
   no mipmaps, and there is no dual-alphaTest edge behaviour. This is
   exactly the surface the contract's §2 stroke-identity battery
   exists to judge — it gates M4; this A/B does not claim it.
   → Audit A3 (#1 identity risk: ink stroke rasterisation);
   contract §2.

## Substitutions made while re-authoring (each deliberate)

| # | Original | Native substitution | Maps to |
|---|---|---|---|
| S1 | Grass texture: Skia-painted 512² canvas (T.grass) | Same recipe re-implemented in the Rust painter (same sin-hash placement, same shade law; capsule strokes instead of Skia lines; texture repeat phase not aligned to the original's) | A5 (≈26 canvas painters; a native Canvas2D-equivalent painter is a hard requirement) |
| S2 | Planks textures on roof/crates/barrel (T.planks) | Flat colours (`#8A6B42`, `#B07A48`; barrel darkened to `#6E4525` to approximate colour×map multiply) | A5 painted-texture breadth (R9) |
| S3 | Hills carry a grass map [6,3] | Flat shade-varied colour per hill | A5 / R9 |
| S4 | Round-tree canopies: IcosahedronGeometry(detail 2) | UV spheres (12×9) with the same centres/radii/shade law | A5 geometry vocabulary |
| S5 | Sky: ShaderMaterial gradient dome (toneMapped false) | Painted 2×512 gradient texture (same formula, mixed in linear space) on an inward sphere | A1 sky dome (the one ShaderMaterial) |
| S6 | Clouds: canvas texture with 3 px / 10 px Gaussian blurs | Radial alpha falloff discs + a bottom alpha fade approximating both blurs | A5 cloudTex painter |
| S7 | Flame materials: MeshStandardMaterial emissive (2.4 / 2.6) | Flat bright colours (`#FF8A30`, `#FFD23C`) through the diffuse shader | A1 materials; composer/bloom absence (gap 1) |
| S8 | Steel rifle parts: metalness 0.55 under IBL | Flat `#3A3D42` diffuse | R3 (IBL) |
| S9 | Caption: Skia strokeText/fillText (Inter 900) | PIL/FreeType strip from the project's Inter-Black.otf, same layout law (centre (540, 1340), fit = min(1, 960/measure) = 1.0, stroke 15, shadow (+3, +9) @ 0.45) | Contract §2.4 (text metrics/rasterisation are identity surface) |
| S10 | Character plane: MeshBasicMaterial, toneMapped false | Textured pipeline samples the sRGB card directly (a colour round-trip) | A3 canvas→3D path |
| S11 | MSAA 4× on a HalfFloat composer target | Single-sample Rgba8 target | A1 renderer config |
| S12 | Textures: mipmaps + anisotropy 4 (card) / 8 (grass) | No mipmaps, bilinear only | A3 texture path |
| S13 | Hill trees: tree() with default colour/kind | Round trees in `#3E7A34` (the ring trees' majority colour) | A5 (defaults not separately extracted) |
| S14 | Blob shadow at y 0.01 (under the dirt disc's 0.012) | Drawn at y 0.02, above the dirt disc, so it stays visible; the original's visible foot shadow is mostly its shadow-map shadow, which gap 2 covers | A3 blob + A1 shadows |

## Differences that are properties of the evidence, not the scene

- **The original is a JPEG.** `OP.frame` returns the composited
  canvas as JPEG (q 0.92 default); the review frame carries that
  generation's artefacts into every pixel comparison. The native
  frame is a direct PNG. Contract §5 (the q0.93-JPEG decision) is
  explicitly OPEN — this package does not settle it; it records that
  the comparison baseline itself is JPEG-generation material.
- **Both renderers are software.** Original: SwiftShader (Chrome's
  software GL). Native: lavapipe (software Vulkan via wgpu). Neither
  frame is the GPU master the user's PC produces; per contract §6.2
  the native side is correctness evidence only.
- **The native lighting is a fixed M1 shader**, not a port of the
  scene's rig: even where geometry and albedo match exactly, shading
  differs by the (0.35 + 0.65·diffuse) term against the original's
  key/rim/hemisphere/IBL sum. Expect broad, smooth differences across
  every sloped surface (tree canopies, hills, ground at grazing
  angles) on top of the itemised gaps.
