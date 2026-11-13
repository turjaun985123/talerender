# ADR-001 — TaleRender Architecture

**Status:** Accepted (Milestone 0 gate, 2026-11-13)
**Evidence base:** the six M0 audit findings in [docs/audit/](audit/) —
cited below as a1 (engine), a2 (capture), a3 (character), a4 (animation),
a5 (environment), a6 (pipeline). Nothing in this record asserts behaviour
the findings did not establish.

## Context

Tall Tale Shorts renders through headless Chrome: a page builds a three.js
r170 scene, paints its 2D characters on canvases, and a CDP-driven capture
tool seeks the page and reads frames back. The audit established that the
browser surface of the whole pipeline is exactly one stage family — the
capture tool's page modes (a6). Everything else (voice, words, lint,
build, mix) is already browser-free, and FFmpeg finishing stays out of
scope for this project (a6, a2 §2).

TaleRender replaces the capture stage with a native Rust + wgpu renderer.
This ADR records the architecture that replacement takes.

## Decision 1 — The seam is `frame(t)`: a pure, seekable function of film time

The existing system already has the right boundary; we keep it. In the
page, `window.OP` is the whole contract (a2 §1.2): `OP.draw(t)` renders
the composited 1080×1920 frame at film time *t* — 3D canvas blitted onto
the 2D canvas, overlays and captions on top, fade-in (0.25 s) and
fade-out (0.4 s) baked in — and `OP.frame(t, q)` returns its pixels.
Capture never renders anything itself; it seeks, reads, QA-checks and
encodes.

The native engine therefore exposes the same shape in-process:

- `frame(t)` → composited RGBA frame at exactly 1080×1920, a **pure
  function of *t*** (plus project data loaded at boot). No wall clock, no
  vsync, no accumulated state — arbitrary seek order must give identical
  results, because the audit and QA tooling draw times out of order
  (a2 §3.1, a4 §3).
- The metadata half of the OP contract is served by typed calls, not
  string evaluation: `duration()`, `shots()`, `captions()` (with width
  metrics from the real text measurer — a2 §5), `cues()`, `cutouts()`,
  `cast()`, and the castsheet `CAST` surface.
- Boot is gated: fonts loaded and project data (script, lines, words)
  read before the first frame can exist — the native equivalent of
  `OP.ready` / `OP.bootError` (a2 §1.1–1.2).

## Decision 2 — Native scene model: small closed geometry vocabulary, one material family

Per a5 and a1 §3.5, the scene the engine must hold is not a general
three.js scene graph:

- **Geometry** is generated from a closed vocabulary: parameterised
  boxes and cylinders cached by exact dimensions (the `box()`/`cyl()`
  helpers, 141 call sites), 11 primitive classes with three's exact
  parametric construction, two CPU-displaced ground planes (displacement
  formulas ported, not heightmap assets), and one extrude/shape family.
  Vertex data is generated in Rust to match three's construction, so
  geometry is EXACT-class (a1 §3.5).
- **Materials** reduce to one PBR family — three r170's
  `MeshStandardMaterial` with the house defaults (roughness 0.85,
  metalness 0) — plus unlit Basic/Sprite materials, one sky
  ShaderMaterial, and one PointsMaterial (a5 §1.2, a1 §3.4). The PBR
  shader and its PMREM RoomEnvironment IBL are a first-class component
  with their own chart tests (a1 Risk #2), not a stock wgpu example
  shader.
- **Multi-material boxes** (per-face material slots) are supported
  structurally — facades depend on them (a5 §1.1).
- **Repetition** is handled by instanced draws where the source repeats
  geometry: the five existing InstancedMesh uses port directly, and
  trees — built fresh per call in the source, ≈200 draw calls in a
  typical frame — are the first additional instancing target (a5 §6).
- **Scene state transforms are data.** An atmosphere is 10 presets plus
  a deterministic scene-state transform (fog, light re-dress, sky
  uniforms, env-match tint) (a1 §5). Fog is owned by the
  atmosphere/engine layer, never by sets (a5 §2.3).

## Decision 3 — Animation ports as pure functions over plain data

The animation layer needs no compatibility shim for its evaluation core
(a4):

- motion.ts, poses.ts, timing.ts (evaluation) and library.ts are
  **PURE-TS** — plain numbers in and out, `Vec3Like`/`RigFrame` as the
  three-free boundary. They port mechanically to Rust functions over
  plain structs, preserving the law that evaluation is a pure function
  of (t, anchors, seeds) (a4 §3).
- The 12 catalog shot helpers port the same way. The single
  three-coupled core function, `createShoot`, is replaced by the native
  camera application step: RigFrame → camera → characters `face()` the
  camera → render (a4 §2.3).
- Poses (17), moods (19) and the shot catalog are **data registries**
  ingested unchanged (a4 §5).
- Project shot files (`src/shots.ts`) are content, not engine: they are
  re-authored against the native API, never translated (a4 §4).
- Transcendentals are evaluated in **f64** (the JS sources are f64
  throughout; a3 §4.3, a4 §3) — byte-identical cross-implementation
  frame hashes are not promised, but last-ulp drift is kept out of the
  state layer.

## Decision 4 — The character painter is a native 2D layer, and it is the fidelity-critical component

Characters, captions and every environment texture are canvas rasters
(a3, a5). TaleRender therefore includes a native 2D raster layer —
path/stroke/fill/clip, gradients, and text — that paints:

- **Characters** via the `drawToon` painter semantics: the 512×1024
  stage inside the 1280×1408 canvas, the repaint-key model (state
  serialised, numbers rounded to 2 dp; repaint only on key change —
  a3 §1.0), premultiplied-alpha output, and the dual alpha thresholds
  (0.04 colour / 0.5 shadow) that turn canvas alpha into the 3D
  silhouette (a3 §2, §5.5). `drawToon`'s returned hand coordinates are
  part of the contract — 3D prop attachment depends on them (a3 §2).
- **Captions** in the 2D compositing layer over the rendered frame,
  using the shared fit law (`fit = min(1, 960 / measured width)`, Inter
  900) — one measurer for renderer and audit, never two (a3 §3).
- **Environment textures** from the ~26 code painters (a5 §1.3), with
  text-bearing textures (facades, signs, posters, the scale dial) held
  to the strictest fidelity bar (a1 Risk #5).

Held props are closures in the source (`holdL`/`holdR`, the 36-entry
`PROPS` table) and cannot cross a data-only boundary as-is: in the
native scene model they become **named prop IDs with parameters**
(a3 §6).

Because stroke rasterisation decides whether the characters still look
like themselves (a3 §5.1), the painter is scheduled and tested as the
project's critical component, ahead of scene breadth.

## Decision 5 — Module boundaries

Following the audit's own seams:

| Module | Responsibility | Audit basis |
|---|---|---|
| `scene` | Native scene model: geometry vocabulary, materials, lights, instancing, atmosphere application | a1, a5 |
| `painter` | 2D raster layer: characters, captions, painted textures, overflow telemetry source | a3, a1 Risk #5 |
| `anim` | Pure evaluation: easings, tracks, cues, rigs, registries | a4 |
| `composer` | Fixed render graph: HDR render → DoF → bloom → Neutral tonemap + sRGB → GRADE, in that order, over HalfFloat MSAA-4× | a1 §2, Risk #4 |
| `runtime` | The `frame(t)` seam + OP-equivalent metadata/telemetry API + boot gating | a2 §1.2, a6 |
| `cli` | Capture replacement: modes `check`, `cues`, `sheet`, `frames`, `video`, `castsheet` with the existing argument shapes, env contract, output formats and exit-code lattice | a2 §3–§5 |

The renderer stays independent of the encoder: `cli` hands frames to
FFmpeg exactly as capture does today (a2 §4.3), and TaleMedia is a
separate project with its own decisions.

## Explicitly deferred

| Item | Deferred to | Reason / gate |
|---|---|---|
| Composer **parity** (per-pass arithmetic match: bloom kernels, GRADE position, stage-by-stage diffing) | M5 (cinematic quality) | Architecture is fixed now (Decision 5); arithmetic parity is proven pass-by-pass during M5 bring-up (a1 §2.1). The **DoF chart test is not deferred** — it gates M3 (see contract). |
| Shadow maps (PCFSoft-equivalent sampling, per-set bias re-derivation in wgpu depth) | M5 | a1 Risk #3: softness pattern and bias must be re-derived per shadow-box size against charts; M1–M3 work proceeds without shadow parity claims. |
| Star/points rendering (wgpu has no point-size primitive) | M2 decision | Quad expansion is APPROXIMATE-class (a1 §3.4); decide when the sky is ported. |
| Texture baking location (native painter vs a retained build-time canvas step) | M2 decision | a1 Risk #5: font fidelity on text-bearing textures decides; a build-time tool is compatible with the no-browser rule, a runtime browser is not. |
| Semantic QA telemetry implementation (`cutouts()` channel in the CLI) | M6, with painter hooks from M4 | The painter must expose overflow counts from its first version (a2 §3.2); the full `check` verdict lattice lands with the capture replacement. |
| The q0.93 JPEG master-generation question | Decision required **before M6** | Open decision — see the compatibility contract §5. Not decided by this ADR. |
| Facade texture streaming/atlas strategy | M7 | Memory is budgeted (≈40–90 MB per street set, a5 §6); streaming only if production profiles demand it. |

## Consequences

- M2 builds the scene/animation core against Decisions 2–3; M3's first
  real scene is gated by the contract's DoF chart and painter stroke
  tests, not by scene completeness.
- Anything that would change the `frame(t)` seam, the module boundaries,
  or the composer pass order requires a superseding ADR.
