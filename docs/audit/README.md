# Milestone 0 Audit — Findings Index

Six read-only audit seats inventoried the shipped Tall Tale Shorts skill
(v5.0, three.js **r170**) before any native code was designed against it.
The findings files in this directory are the original reports, archived
verbatim; this index states what each one established and the conclusions
that cut across all six. The decisions taken from them live in
[ADR-001](../ADR-001-talepender-architecture.md), the testable contract in
[compatibility-contract.md](../compatibility-contract.md), and the ranked
risks in [risk-register.md](../risk-register.md).

## The six findings

| File | Seat | What it established |
|---|---|---|
| [a1-engine.md](a1-engine.md) | Engine & renderer | The whole 3D pipeline funnels through **one composer chain** (RenderPass → BokehPass → UnrealBloomPass → OutputPass → custom GRADE) over a HalfFloat MSAA-4× target, with PMREM RoomEnvironment IBL touching every `MeshStandardMaterial` surface. Full per-API inventory of engine.ts / lib.ts / world.ts / env.ts with EXACT / APPROXIMATE / SOURCE-CHANGE classes. Only **two** hand-written shaders exist (GRADE, sky dome). `lib.ts` uses **zero** THREE APIs — it is Canvas2D. **#1 parity risk: the BokehPass DoF** — a physically incorrect shader the house look is art-directed around; its r170 arithmetic must be transcribed, not improved. |
| [a2-capture.md](a2-capture.md) | Capture & browser dependency | There is **no screenshot API anywhere**: capture drives the page through one global, `window.OP`, over CDP `Runtime.evaluate`, and frames leave as canvas data. The **QA gate is semantic telemetry, not pixels** — `check`'s verdict rests on `OP.cutouts()` overflow counts, look-signature continuity, and plane-box composition thresholds. Master frames reach x264 only after a **q0.93 JPEG generation**. Renderer-identity and failure honesty (software-renderer warnings, `.part` + rename, spawn-error vs exit-code) are defended in detail. Ends with the full native-CLI reproduction checklist. |
| [a3-character.md](a3-character.md) | Character & 2D canvas | The character system is **one procedural Canvas2D painter** (`drawToonLayer`) — no image assets. **42 distinct Canvas2D APIs** inventoried. Characters are drawn on a 512×1024 stage inside a 1280×1408 canvas (2560×2816 px at res 2), uploaded as a CanvasTexture onto a camera-facing plane; repaint + **full re-upload happens on every state-key change**. The painter is deterministic (zero `Math.random`). **#1 identity risk: ink stroke rasterisation** — characters are stroked paths with round caps/joins; any rasterizer that flattens or anti-aliases differently changes line weight everywhere. Captions use one font (Inter 900) with a measured fit law shared by renderer and audit. |
| [a4-animation-camera.md](a4-animation-camera.md) | Animation & camera | Animation evaluation is **proven a pure function of (t, anchors, seeds)** — no clocks, no RNG, no integrators; follow-through is analytic. **motion.ts, poses.ts, timing.ts, library.ts are PURE-TS** (zero three.js values); the shot catalog's 12 helpers are pure; the only three-coupled core function is `createShoot`. Poses (17) and moods (19) are plain data registries. The example film's staging is three-coupled and ports by re-authoring, not translation. |
| [a5-environment.md](a5-environment.md) | Environment & content | **63 builder functions** (12 sets, 27 props in props.ts + reusable pieces). The runtime geometry vocabulary is tiny and closed — dimension-cached `box()`/`cyl()` helpers (141 call sites) plus 11 primitive classes; the workload is **many small meshes** (heaviest set ≈ 450–500). **Zero image assets**: all textures are code-painted by ~26 canvas painters, several text-bearing — a Canvas2D-equivalent painter with font rendering is a native-engine requirement. Trees are the top instancing opportunity (~200 draw calls → a few). Facade textures on street sets run ≈ 40–90 MB. Sets never configure fog; nearly everything casts and receives shadows. |
| [a6-pipeline.md](a6-pipeline.md) | Pipeline & dependencies | The full stage table with exact invocations: **only the capture tool's page modes are browser-dependent** (`cues`, `check`, `sheet`, `frames`, `video`, `castsheet`, `preview`); voice, words, estimate, lint, build and mix never touch a browser. **FFmpeg is a second cross-cutting dependency** (sheet tiling, video encode, loudnorm measurement, audio decode). There are **no runtime npm dependencies** — three r170 is a vendored bundle; capture drives Chrome with raw CDP over Node built-ins. Narration drives all timing: shots anchor to word timings, so a re-voice invalidates shot anchoring. |

> **Archive note (a6):** the a6 file as delivered ends mid-sentence in
> §2.2 (its later sections were not present in the archived copy). The
> `window.OP` seam it was documenting is specified in full by a2 §1.2, and
> its pipeline/browser-boundary findings are complete in the archived
> text. It is archived verbatim, gap included — see the M0 gate report.

## Cross-cutting conclusions

1. **The browser surface is exactly one stage family.** Everything else
   in the pipeline is already browser-free (a6). TaleRender replaces the
   capture tool, nothing else — and must serve the same `OP` contract
   (a2 §1.2) or `check` and `cues` break with it.
2. **The renderer's seam is a pure function of film time.** `OP.draw(t)` /
   `OP.frame(t, q)` render a composited 1080×1920 frame seekably, with
   fades baked in. Animation (a4) and painting (a3) are both deterministic
   pure functions, so a native `frame(t)` can be held to the same law.
3. **Correctness in this pipeline is telemetry, not pixels.** The QA gate
   gates on overflow counts, continuity signatures and composition
   thresholds computed inside the painter and projected through the
   camera (a2 §3.2–3.3). A frames-only renderer silently deletes it.
4. **The look lives in code-painted rasters and one composer chain.**
   Every texture in the world and every character is a canvas raster
   (a3, a5); the film look is the composer chain's pass order and
   arithmetic (a1). Parity is therefore a property of two components —
   the 2D painter and the composer — more than of the geometry layer,
   whose vocabulary is small and exact (a5, a1 §3.5).
5. **Depth of field and stroke rasterisation are the twin fidelity
   risks**, named independently by a1 and a3: the Bokeh shader's
   artefacts are load-bearing style, and the characters *are* their
   stroke edges. Both get dedicated chart tests in the contract before
   any real scene is ported.
6. **Nothing so far is verified on a GPU.** All native evidence to date
   is lavapipe (software Vulkan) in CI. GPU and performance claims are
   only admissible from the target PC (Ryzen 5 PRO 5650G, Vega 7, RADV).
