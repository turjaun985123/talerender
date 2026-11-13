# A1 — Engine & Renderer API Inventory (Tall Tale Shorts v5.0)

**Seat:** TaleRender Milestone 0, Seat A1 (read-only audit).
**Targets (all exist, read in full):**
- `template/src/core/engine.ts` — 73 lines (renderer, composer chain, light rig, camera factory)
- `template/src/core/lib.ts` — 44 lines (palette, math, Canvas2D primitives)
- `template/src/core/world.ts` — 740 lines (painted textures, sky, all environment sets)
- `template/src/core/env.ts` — 262 lines (atmosphere presets, scene re-dressing)

**Three.js version context:** `template/package.json` declares `@types/three ^0.170.0` — the API surface audited here is three **r170**. Addon behaviour (BokehPass, OutputPass, `scene.environmentIntensity`, NeutralToneMapping) is r170-specific and is cited as such.

**Headline findings:**
1. **lib.ts uses ZERO `THREE.*` APIs.** Its entire render surface is `CanvasRenderingContext2D` (`ctx.roundRect`, `ctx.ellipse`, `ctx.arc`, stroked/filled text in `Inter` 900) plus pure math helpers. It belongs to the 2D-canvas audit seat, not this one; it is listed here only to close the question.
2. The whole 3D pipeline funnels through **one composer chain in engine.ts** (RenderPass → BokehPass → UnrealBloomPass → OutputPass → custom GRADE ShaderPass) rendering into a **HalfFloat MSAA render target**. Nothing renders directly to the default framebuffer except `snapshot()` (world.ts:404–408).
3. `MeshStandardMaterial` with **roughness 0.85 / metalness 0 defaults** (world.ts:49–54, the `mat()` factory) is the material of practically the entire world; image-based lighting from a **PMREM'd RoomEnvironment** (engine.ts:33) touches every one of those surfaces.

Compatibility classes used below: **EXACT** = a wgpu implementation can match three r170's output bit-for-bit or within rounding; **APPROXIMATE** = same feature, visibly equivalent, but exact texel/pixel equality is not achievable without reimplementing three's exact shader arithmetic; **SOURCE-CHANGE** = the scene source or pipeline structure must change for a native equivalent.

---

## 1. Renderer configuration — as actually set (engine.ts)

All values quoted from `makeEngine` (engine.ts:29–52) and the module header (engine.ts:11–17).

| Setting | Value in code | Where |
|---|---|---|
| Canvas size constants | `W = 1080, H = 1920` | engine.ts:11 |
| `new THREE.WebGLRenderer` params | `{ canvas, antialias: false, preserveDrawingBuffer: true, powerPreference: 'high-performance' }` | engine.ts:30 |
| `renderer.setSize` | `setSize(w, h, false)` (w=1080, h=1920 defaults) | engine.ts:31 |
| `renderer.setPixelRatio` | `1` | engine.ts:31 |
| Tone mapping | `THREE.NeutralToneMapping`, `toneMappingExposure = 1.0` (per-shot override via `look.exposure`) | engine.ts:32, 64 |
| Shadow map | `shadowMap.enabled = true`; `type = THREE.PCFSoftShadowMap`; `autoUpdate = false`; `needsUpdate = true` forced every `render()` call | engine.ts:33, 57 |
| Output colour space | `THREE.SRGBColorSpace` | engine.ts:33 |
| Environment map | `PMREMGenerator.fromScene(new RoomEnvironment(), 0.04).texture` | engine.ts:33 |
| Composer render target | `new THREE.WebGLRenderTarget(w, h, { type: THREE.HalfFloatType, samples })`, `samples = 4` default | engine.ts:34, 29 |
| MSAA | **Off** on the WebGLRenderer (`antialias: false`); MSAA exists only as the composer RT's `samples: 4` | engine.ts:30, 34 |
| URL quality overrides | `?res=` (default 1, range (0,1]), `?aa=` (default 4, one of 0/2/4/8), `?shadow=` (default 2048, one of 512/1024/2048/4096) — validated by `qsVal`, engine.ts:14–17 | engine.ts:14–17 |
| Shadow size plumbing | `shadowSize` (default 2048) stashed in `renderer.userData` (engine.ts:35); the light rig reads the module-level `SHADOW` constant, not `shadowSize` | engine.ts:35, 71 |
| Default light-rig shadow | key light `shadow.mapSize.set(SHADOW, SHADOW)` = 2048×2048; `bias = -0.0004`; `normalBias = 0.02`; `radius = 6`; ortho box ±`shadowBox` (default 3 m), near 0.1, far 20 | engine.ts:70–72 |
| Interior sun shadow (world.ts) | `mapSize.set(2048, 2048)` hardcoded; box ±8, near 0.5, far 30 (later 80 via sunlight(), world.ts:189); `bias = -0.0006`; `radius = 4` | world.ts:359, 189 |

**Native equivalent / class:** a wgpu renderer with an offscreen `Rgba16Float` target, 4× MSAA (multisampled colour + resolve), a single 2048² depth texture for the key light re-rendered per frame (the `autoUpdate=false` + `needsUpdate=true` pattern means: shadow map is re-rendered every frame by explicit request — the native engine can simply always render it, EXACT behaviour). Neutral tone mapping is a defined curve (Khronos PBR neutral) — reimplementing the exact GLSL formula in WGSL is **EXACT**-class arithmetic. `preserveDrawingBuffer: true` exists only so the browser capture path can read the canvas back; a native engine owns its readback — **SOURCE-CHANGE** at the capture layer (not the scene layer).
**Parity test:** render the golden template scene through Chrome and through TaleRender with post disabled; compare linear-HDR buffers before tone mapping (isolates renderer config from post), then compare final sRGB frames (PSNR + per-pixel Δ heatmap).

---

## 2. three/addons imports (all in engine.ts:4–10)

No other target file imports addons. world.ts and env.ts import only `three` core.

### 2.1 EffectComposer — engine.ts:4, 36–37
- **Use:** `new EffectComposer(renderer, rt)` with the explicit HalfFloat RT above; `composer.setSize(w, h)`; passes added in fixed order (below); `composer.render()` is the only draw entry point (engine.ts:67).
- **Behaviour:** ping-pong HalfFloat buffers between passes; the supplied RT (with `samples: 4`) is the first pass's target, so scene MSAA happens inside the composer.
- **Native equivalent:** a fixed render-graph: HDR MSAA target → depth-prepass/DoF → bloom chain → tonemap/sRGB → grade, all as wgpu render passes over `Rgba16Float` textures. **APPROXIMATE** (buffer formats and resolve timing can match; per-pass arithmetic is what differs — see each pass).
- **Parity test:** pass-isolation renders — enable one pass at a time in both pipelines on the same scene and diff after each stage.

### 2.2 RenderPass — engine.ts:5, 38, 59
- **Use:** constructed with a throwaway `new THREE.Scene()` / `new THREE.PerspectiveCamera()`, then `rp.scene` / `rp.camera` are **reassigned every frame** in `render()` (engine.ts:59). Clears and renders the scene into the composer target.
- **Native equivalent:** the scene render pass itself. **EXACT** (modulo material shading, §3.4).
- **Parity test:** composer with all later passes bypassed vs direct renderer output.

### 2.3 BokehPass (depth of field) — engine.ts:7, 40, 60–62 — ⚠ see Risk #1
- **Configured at construction:** `{ focus: 3, aperture: 0.002, maxblur: 0.008 }` (engine.ts:40).
- **Per-frame overrides in `render()`:** `bokeh.enabled = (look.aperture ?? 0.0015) > 0`; uniforms `focus = look.focus ?? 3`, `aperture = look.aperture ?? 0.0015`, `maxblur = look.maxblur ?? 0.006` (engine.ts:60–62). So the *effective defaults in production* are aperture 0.0015 / maxblur 0.006, not the constructor values.
- **Behaviour (r170 addon):** renders the scene's depth into a depth texture with a depth material override, then a single fullscreen shader computes a circle of confusion from viewZ and blurs by sampling the colour buffer with a **fixed poisson/disc tap pattern**, including a depth-based factor and highlight/vignette terms baked into its shader. It does **not** do gather-as-scatter, near/far field separation, or partial occlusion — out-of-focus foreground does not bleed correctly over sharp background in the general case, and the Tall Tale look has been art-directed around whatever this shader actually does.
- **Native equivalent:** reimplement the r170 BokehShader verbatim in WGSL (same tap offsets, same CoC formula `factor = depth - focus`, same `aperture * -focus` scaling) over a native depth prepass. **APPROXIMATE** — the formula ports exactly, but the depth texture's precision/packing (three uses a float depth render target in this pass) and the tap sampling (linear filtering of an HDR buffer) will differ in edge texels.
- **Parity test:** dedicated DoF chart scene — sharp subject at `focus`, textured planes at known distances in front/behind, high-contrast edges crossing the focal plane; compare blur radius per distance and edge behaviour (this is the exact test that exposed the bokeh fork in the earlier headless-gl spike).

### 2.4 UnrealBloomPass — engine.ts:6, 41, 63
- **Configured at construction:** `new UnrealBloomPass(new THREE.Vector2(w, h), strength 0.35, radius 0.6, threshold 0.85)` (engine.ts:41).
- **Per-frame:** `bloom.strength = look.bloom ?? 0.3`; `bloom.threshold = look.bloomThreshold ?? 0.85`; `bloom.radius = 0.6` (hardcoded every frame, engine.ts:63). Effective production defaults: **strength 0.3, radius 0.6, threshold 0.85**, applied to **linear HDR before tone mapping** (pass order: bloom sits before OutputPass).
- **Behaviour (r170 addon):** 5-level mip chain of separable Gaussian blurs, high-pass on luminance with a smooth-width threshold curve, additive composite with per-level weights.
- **Native equivalent:** same mip/Gaussian architecture in WGSL. **APPROXIMATE** — three's blur kernel weights and its threshold smoothstep are portable arithmetic, but HalfFloat precision and the exact downsample filter (it uses a specific 9-tap kernel) must be transcribed, not reinvented; a "nicer" bloom will not match.
- **Parity test:** HDR emissive chart (lamp/bulb meshes — world.ts uses `emissiveIntensity` 0.3–2.2, the bloom sources) at several strengths; compare halo radius/energy per level.

### 2.5 OutputPass — engine.ts:9, 42
- **Use:** `composer.addPass(new OutputPass())` — applies the renderer's `toneMapping` (Neutral) + output colour-space conversion (linear → sRGB) using renderer state at render time.
- **Behaviour worth noting:** it sits **before** the GRADE pass, so grading (vignette/grain/saturation/warmth) operates in **display-referred sRGB space**, not linear. That ordering is part of the film look and must be preserved in the native graph even though it is "wrong" by textbook convention.
- **Native equivalent:** tonemap + sRGB encode step in the graph, in the same position. **EXACT** (both curves are closed-form).
- **
...[truncated 20871 chars]
### 3.3 Cameras

| API | file:line | Why / behaviour | Native equivalent · class | Parity test |
|---|---|---|---|---|
| `THREE.PerspectiveCamera(fov, aspect, near, far)` | engine.ts:38 (placeholder), 73 (`camera(fov=30)` → aspect `W/H` = 0.5625, near 0.05, far 800) | The house default camera; shots override fov/aspect elsewhere | Standard perspective projection (wgpu clip-space z differs: three/WebGL uses OpenGL NDC z∈[−1,1], wgpu uses z∈[0,1] — projection matrix must be adapted, then EXACT) | Frustum-corner projection of known points, both pipelines |
| `camera.updateProjectionMatrix()` via shadow camera | engine.ts:72, world.ts:189/359 | After mutating shadow ortho bounds | Same call on native ortho camera · EXACT | — (covered by shadow tests) |

### 3.4 Materials

| API | file:line | Why / behaviour | Native equivalent · class | Parity test |
|---|---|---|---|---|
| `THREE.MeshStandardMaterial` + params (`color`, `map`, `roughness`, `metalness`, `emissive`, `emissiveIntensity`, `side`, `transparent`, `alphaTest`, `depthWrite`) | world.ts:49–54 (`mat()` factory, defaults **roughness 0.85, metalness 0**), 230, 240, 365 (emissive bulbs/lamps, `emissiveIntensity` 0.6/0.3/2.2), 431 (fronds: DoubleSide + transparent + `alphaTest: 0.35`), 485 (cloud deck), passim | Practically every solid surface in every set; also the InstancedMesh materials (world.ts:369, 392) | A single PBR shader (GGX + Lambert diffuse + env IBL) reproducing three r170's `meshphysical` GLSL, including its direct-light punctual model, fog chunk and tonemapping/colorspace fragments. **APPROXIMATE** — this is the largest single shading surface; three's BRDF (GGX with its specific geometry term, envmap mip selection by roughness, sRGB texture decode points) can be transcribed but floating-point equality across drivers is not attainable | Material chart scene: sphere grid over (roughness × metalness × emissive) under the house light rig + env; per-swatch ΔE |
| `THREE.MeshBasicMaterial` | world.ts:217 (wire tubes), 271 (bunting triangles, DoubleSide), 341 (interior view plane, `toneMapped: false`), 355 (bulb cords), 619 (foam strips: map, transparent, `depthWrite: false`, `toneMapped: false`, `opacity` 0.95/0.77/0.59) | Unlit colour/texture — wires, foam, flat backdrops | Unlit textured shader honouring `fog`, `toneMapped`, opacity. **EXACT**-class (trivial arithmetic; only texture sampling differs) | Flat colour/texture quads at several fog distances |
| `THREE.SpriteMaterial` (`map`, `color`, `fog: false`, `toneMapped: false`, `depthWrite: false`, `transparent`) | world.ts:183 (clouds), 730 (Earth) | Unlit billboards exempt from tonemapping and fog | Same as Sprite above · EXACT-class | Sprite half-overlapping fogged geometry |
| `THREE.PointsMaterial` (`size: 1.6`, `sizeAttenuation: false`, `transparent`, `opacity: 0.9`, `depthWrite: false`, `fog: false`, `toneMapped: false`) | world.ts:465 (stars) | Fixed-pixel-size points in the sky dome | Native point rendering — ⚠ wgpu has **no variable point size**: `gl_PointSize` doesn't exist; points are 1px or must become camera-facing quads. **SOURCE-CHANGE / APPROXIMATE** (quad expansion reproduces 1.6 px stars approximately; rasterisation of sub-2px quads differs from GL points) | Star field render at 1080×1920, count/brightness of resolved stars |
| `THREE.ShaderMaterial` | world.ts:164 (sky, §4.2); env.ts only *recognises* it (`instanceof`, env.ts:192) | Sky dome | WGSL port · see §4 | Sky gradient sweep |
| Material **array** on `Mesh` (BoxGeometry groups) | world.ts:196 (signBoard face), 281/663 (building fronts), 349 (picture) | Different material per box face — facade texture on ±x face only | Multi-material = 6 draw groups with per-group material, or a face-index attribute in one shader. **EXACT** (structural) | Sign/building render, all faces |
| `material.map.rotation` / `.repeat` post-creation mutation | world.ts:263 (boardwalk `map.rotation = π/2`), 330, 336 (floor/side walls) | Rotating/repeating a shared cached texture per-mesh via the clone from `canvasTex()` | Per-texture UV transform uniform (three applies `uvTransform` from offset/repeat/center/rotation) · EXACT if the same matrix formula is used | Boardwalk/floor close-ups |
| `mat()` **material cache** + `GEOCACHE` (world.ts:48–63) | whole file | Flat materials and geometries are shared singletons keyed by parameter string; textured materials always fresh | Native asset cache with the same keying. **EXACT** (behavioural, not visual) — but the port must preserve the *mutation privacy* the comments describe (three call sites mutate textures after creation) | — (unit-level) |

### 3.5 Geometry

All are `THREE.BufferGeometry` subclasses generating indexed position/normal/uv attributes on the CPU. Native equivalent for every one: generate the identical vertex data in Rust (or bake it), upload as buffers. Class **EXACT** if segment counts and vertex construction match three's parametric formulas (they are deterministic and documented); tessellation differences are the only risk, and they show up as silhouette differences at glancing angles.

| API | file:line (representative) | Notes |
|---|---|---|
| `THREE.PlaneGeometry(w, h[, segX, segY])` | world.ts:255, 260, 384 (200×200 @ 100×100 segs, displaced), 430 (fronds, then `.translate()`), 485, 549 passim, 671 (220×220 @ 90×90, displaced) | The ground/terrain workhorse; displaced variants feed `attributes.position` + `computeVertexNormals()` (§3.7) |
| `THREE.BoxGeometry(w, h, d)` | world.ts:61 (cached), 196, 281, 663 | Always via `boxGeo()` cache or multi-material |
| `THREE.SphereGeometry(r, w, h)` | world.ts:164 (sky, 32×16), 207 (hills, 32×16), 230/240/365 (bulbs), 559, 579 | Sky dome and hills are unit spheres scaled non-uniformly |
| `THREE.CircleGeometry(r, seg)` | world.ts:387, 474 (stage discs), 561, 589 | Flat stage/pond discs |
| `THREE.ConeGeometry(r, h, seg)` | world.ts:215 (pine tiers, seg 10), 393 (grass tufts, seg 3), 470 (peaks, seg 5), 608 (crops, seg 6) | |
| `THREE.CylinderGeometry(rTop, rBottom, h, seg)` | world.ts:62 (cached), seg 4–16 | Trunks, poles, barrels, wheels |
| `THREE.IcosahedronGeometry(r, detail)` | world.ts:219 (foliage, detail 2), 392 (flowers, detail 0), 517 (fgLeaves, detail 1) | Flat-shaded look comes from non-indexed normals — replicate three's exact vertex normals |
| `THREE.TorusGeometry(r, tube, radSeg, tubSeg[, arc])` | world.ts:565 (ribs, arc π), 702 (crater rims) | |
| `THREE.DodecahedronGeometry(r, 0)` | world.ts:563 (desert rocks) | |
| `THREE.TubeGeometry(CatmullRomCurve3, 20, 0.015, 4)` | world.ts:217 (`wire()`) | Sagging wires; curve evaluation must match three's Catmull-Rom parameterisation |
| `THREE.CatmullRomCurve3(points)` | world.ts:217 | See above |
| `geometry.setFromPoints([...])` on raw `BufferGeometry` | world.ts:271 (bunting triangles) | Unindexed triangle soup, no normals (Basic material) |
| `geometry.setAttribute('position', BufferAttribute)` + `Float32Array` positions | world.ts:462–464 (`stars()`) | Point cloud positions on a hemisphere shell |
| `geometry.translate(x, y, z)` | world.ts:430 (frond pivot at base) | Bake-time transform · EXACT |
| `geometry.computeVertexNormals()` | world.ts:386, 671 | After CPU displacement of terrain planes — smooth normals from indexed geometry; formula is standard, **EXACT** if the same accumulation/normalisation is used |

### 3.6 Textures

| API | file:line | Why / behaviour | Native equivalent · class | Parity test |
|---|---|---|---|---|
| `THREE.CanvasTexture(canvas)` | world.ts:29 (via `canvasTex()` cache), 171 (cloud), and every painted texture (planks, wallpaper, brick, siding, dirt, grass, sand, snow, felt, facades, signs, earth, flags, foam, fronds, cloud deck) | **The world's entire surface detail is painted on 2D canvases at build time** and uploaded as textures | Upload the same raster as a wgpu texture. The texture object is **EXACT**; the *raster itself* is only as identical as the 2D rasteriser that painted it — see Risk #5. In TaleRender the canvas painters must run somewhere (native 2D layer or a retained JS/canvas step) | Texture-atlas diff: dump each cached canvas from Chrome and from the native painter, diff per texture |
| Texture props set on every `canvasTex`: `colorSpace = SRGBColorSpace`, `wrapS = wrapT = RepeatWrapping`, `anisotropy = 8` | world.ts:29 | sRGB decode in shader; repeat for tiling grounds; 8× aniso for grazing-angle ground planes | `srgb` texture view + repeat sampler + `anisotropy_clamp = 8` in wgpu (requires no special feature) · **EXACT**-class (sampler LOD bias/aniso filtering implementation varies by driver — APPROXIMATE at extreme grazing angles) | Ground plane at grazing angle, moiré/shimmer comparison |
| `texture.clone()` + `repeat.set()` + `needsUpdate = true` | world.ts:34 | One cached image, many UV scalings; clones share the image but own their transform | Shared image, per-use UV transform · EXACT | — |
| `texture.image` read-back (`.image` of another texture used as `drawImage` source) | world.ts:143–144 (facade tiles brick/siding canvases) | Texture composition at paint time | Painter-internal · n/a to GPU layer | Covered by atlas diff |
| `THREE.WebGLRenderTarget(w, h, { samples: 4, type: HalfFloatType })` + `renderer.getRenderTarget/setRenderTarget` + direct `renderer.render()` | world.ts:404–408 (`snapshot()` — renders a set into a texture for window views / PiP) | Offscreen render reused as a `map` | Native offscreen pass with its own target, texture then bound as a sample · **APPROXIMATE** (same MSAA/HDR caveats as §2.1) | Snapshot texture vs direct render of the same sub-scene |
| `scene.background = new THREE.Color('#1A1418')` | world.ts:328 (interior) | Flat clear colour behind the room | Clear colour · EXACT | — |

### 3.7 Math, instancing, scene-state helpers

| API | file:line | Why / behaviour | Native equivalent · class | Parity test |
|---|---|---|---|---|
| `THREE.Color(hex)` + `.multiplyScalar(k)` + `.getHexString()` | world.ts:41 (`shade()` — the palette derivation for the *entire* painted look) | Darkens/lightens hex colours in **sRGB space as stored** (three Color stores components as given; `multiplyScalar` is linear on the stored values, no colour-space conversion for hex-set colours in r152+ working space — in r170 `setHex` converts to linear-srgb working space by default ⚠) | Rust `Color` with three r170's exact semantics: `ColorManagement.enabled` defaults true in r170, so hex colours ARE converted sRGB→linear on set, and `getHexString()` converts back. Reproduce that or every derived shade shifts. **EXACT** if the conversion is replicated; a naive port that treats hex as raw floats will be visibly wrong everywhere | `shade()` truth table: all palette hexes × k ∈ {0.45…1.3}, compare output hex strings |
| `THREE.Color.lerp(other, t)`, `.set(hex)` | env.ts:229–232 (fills), 205–216 (rig colours) | Atmosphere re-colours lights | Same — colour-space caveat above · EXACT | Preset application unit tests |
| `THREE.Vector3` (`set`, `lerpVectors`, `copy`, `distanceTo`) | world.ts:216–217, 268–270, 687 | Wire/bunting curve points, pole tops | Rust glam `Vec3` · EXACT | — |
| `THREE.Matrix4` (`compose`, `makeTranslation`), `THREE.Quaternion.setFromEuler`, `THREE.Euler` | world.ts:370–371 (litter), 392–394, 608 | Per-instance transforms for InstancedMesh | glam · EXACT | Instance placement dumps |
| `THREE.MathUtils.smoothstep(x, min, max)` | world.ts:670 (mountain height falloff) | Terrain shaping | Same formula · EXACT | — |
| `THREE.InstancedMesh(geo, mat, count)` + `setMatrixAt` + `setColorAt` + `.count =` | world.ts:369 (140 litter slips, with per-instance colour), 392 (1600 flowers, per-instance colour), 393 (4000 grass tufts), 608 (120 crop cones) | Mass scatter without draw-call explosion; `setColorAt` creates `instanceColor`, multiplied into the material colour in three's shader | Native instanced draw with per-instance matrix + colour attributes fed to the same PBR shader chunk · **APPROXIMATE** (only via the material it shares, §3.4) | Scatter-field renders; instance colour multiplication check |
| `scene.traverse(cb)` + type probes (`isSprite`, `isHemisphereLight`, `instanceof ShaderMaterial`) | env.ts:204–212, 220–227, 238–244 | Atmosphere pass finds sprites, sky materials, hemisphere lights, toons | Native scene graph needs the same traversal + type tags · EXACT (structural) | applyAtmosphere before/after state dumps |
| `THREE.IUniform` / `material.uniforms` mutation | env.ts:187–197 (`setUniformColour`, `dressSkyMaterial`) | Re-dressing the sky shader's `top/hor/gnd` uniforms | Uniform buffer update · EXACT | Sky colours per preset |
| `object.userData` (`toon` probe) | env.ts:239 | Finds character objects structurally | Native metadata slot · EXACT (structural) | — |

---

## 4. Custom shaders (GLSL in the target files)

Only **two** hand-written shaders exist across the four files. Everything else that shades (MeshStandardMaterial, Bokeh, Bloom) is three r170's own internal GLSL, which a native port must reimplement from behaviour, not transcribe from these files.

### 4.1 GRADE pass shader — engine.ts:18–27 (the film grade)
- **Vertex:** trivial passthrough (`vUv = uv`, standard MVP).
- **Fragment inputs:** uniforms `tDiffuse`, `uVig` (default-in-object 0.35; production 0.18), `uGrain` (0.025; production 0.008), `uTime`, `uWarm` (0.04; production 0.02), `uSat` (1.12).
- **Computes, in order:** (1) vignette — `d = vUv − 0.5` with **`d.x *= 0.5625`** (the portrait aspect baked into a literal), `c.rgb *= 1 − uVig · smoothstep(0.25, 0.75, length(d)·1.4)`; (2) animated film grain — hash `h(p) = fract(sin(dot(p, vec2(12.9898, 78.233)) + uTime·7.13) · 43758.5453)` sampled at `vUv·1000`, added as `(h − 0.5)·uGrain`; (3) warmth — `c.r += uWarm·0.5; c.b −= uWarm·0.5`; (4) saturation around Rec.601 luma (`dot(c, vec3(0.299, 0.587, 0.114))`), `mix(luma, c, uSat)`.
- **WGSL porting difficulty: LOW.** ~15 lines, no loops/textures beyond one sample. Two traps: (a) it runs on **sRGB-encoded** input (after OutputPass) — port must keep that position in the chain; (b) the grain hash's `sin()` on large arguments is GPU-vendor-sensitive — GLSL `sin(43758·…)` precision differs between ANGLE/Metal/native Vulkan drivers, so grain *pattern* will differ per frame even with identical code. That is invisible in stills at uGrain 0.008 but means grain can never be pixel-compared; compare statistically (mean/std of grain-only frames).

### 4.2 Sky dome shader — world.ts:164–168
- **Vertex:** `vP = normalize(position)`, standard MVP.
- **Fragment inputs:** uniforms `top`, `hor`, `gnd` (THREE.Color); varying `vP`.
- **Computes:** `y = vP.y`; above horizon `mix(hor, top, pow(clamp(y·1.6, 0, 1), 0.7))`, below horizon flat `gnd`. Material flags: `side: BackSide, depthWrite: false, fog: false, toneMapped: false`; mesh `renderOrder = −10` on a 380 m sphere (world.ts:164, 169).
- **WGSL porting difficulty: LOW.** `pow()` edge behaviour at y≤0 is branch-guarded. One subtlety: because `toneMapped: false` and it renders into the HDR target, the sky's sRGB-authored colours pass through three's colour management (uniform colours are linear-workflow values) — the port must set the uniform through the same Color conversion or the sky shifts (see §3.7 Color caveat).

### 4.3 Implicit shader surface (not in these files, but executed because of them)
- three r170 `meshphysical.glsl.js` (all MeshStandardMaterial) — GGX specular, sheen-less, env IBL via PMREM mip sampling, shadow-map sampling with PCFSoft kernel, fog exp2/linear chunk, tonemapping + colourspace fragments.
- BokehShader, UnrealBloom's composite/blur shaders, OutputPass shader (in the r170 addons).
A native engine must reimplement these from their *observable behaviour*; they are the actual parity battlefield (§6).

---

## 5. env.ts — what it does to the render state (for the IR design)

`applyAtmosphere(scene, name, { rig, sky })` (env.ts:200–247) mutates, in order:
1. `scene.fog` ← `new THREE.Fog(color, near, far)` or `null` (env.ts:201). Preset fog ranges: dawn 24–115, golden 30–140, dusk 28–120, night 18–90, storm 12–65, **fog 3–38**, snow 20–95, underwater 4–42, dust 10–60; noon = no fog. **Fog colour always equals horizon colour** (design rule, env.ts:56–58).
2. Every Sprite's `material.color` ← cloud tint; every ShaderMaterial with top/hor/gnd uniforms re-dressed (sky pass, skippable via `sky: false`).
3. Light rig: `key.color/intensity` (+ position only when the preset defines `keyPos`), `rim.intensity`, hemisphere colour/ground/intensity found by traversal; interior `fills[]` PointLights: `intensity *= grade.exposure`, `color.lerp(keyColor, 0.5)`.
4. Env-match: every object with `userData.toon.envMatch === true` gets `toon.matchEnv(preset.tint)` — tint is a **multiply approximation** { sat, warm, exposure, fogColor, fogMix } (env.ts header, honest about its limits; fogMix up to 0.35 for 'fog').
5. Returns the preset; its `grade` { sat, warm, exposure } is **advisory** — the caller feeds it into `E.render()`'s look. The renderer is never touched by env.ts.

**IR consequence:** an atmosphere is pure *data* (the 10 presets, env.ts:61–183) + a deterministic scene-state transform. Fully portable; the only parity surface is the Color semantics (§3.7) and fog's shader chunk (§3.4).

---

## 6. Hardest parity risks (these files)

### Risk #1 — The BokehPass depth-of-field (engine.ts:40, 60–62)
The single least portable element in the pipeline. Three compounding reasons: (a) its look is one specific fullscreen shader with a fixed tap pattern and a CoC formula with known physical incorrectness (no near-field scatter, foreground bleed artefacts) that **the house look has been art-directed around** — a "better" DoF is a *different* look and fails parity; (b) it consumes a depth texture whose precision/representation in the r170 addon (float depth target, viewZ reconstruction from perspective depth) interacts with the 0.05–800 near/far range of the house camera — banding and blur-radius errors concentrate exactly at subject edges, where the eye looks; (c) it is enabled by default in every render (`aperture ?? 0.0015 > 0`), so it touches essentially every shipped frame, including the fgLeaves foreground-framing device (world.ts:505) that exists *specifically* to be blurred by it. The earlier headless-gl spike independently hit this wall: its bokeh/depth path produced the only *structural* artefacts in the A/B (hard billboard edges, lost lamp glow) — same pass, same failure mode, different host. **Mitigation:** transcribe the r170 BokehShader arithmetic into WGSL line-for-line before considering any improvement; build the DoF chart test (§2.3) before the first real scene.

### Risk #2 — MeshStandardMaterial + PMREM RoomEnvironment IBL (engine.ts:33; world.ts:49–54, 187)
Every surface in the world is the same PBR material lit substantially by an environment map the audience never sees directly. Parity requires matching: three's GGX/geometry/fresnel arithmetic, its PMREM generation (the blur kernel over the RoomEnvironment scene at roughness-mapped mip levels, sigma 0.04), `scene.environmentIntensity` scaling (0.25 outdoors via sunlight(), 0.35 interior), and envmap sampling conventions (LOD by roughness, the PMREM UV mapping). Small errors here are not localised — they shift *every* material's brightness/colour balance a few percent in a scene-dependent way, which reads as "the same film, slightly off" everywhere and is brutal to chase. A naive wgpu PBR (e.g. a stock wgpu/examples shader) will visibly diverge. **Mitigation:** port PMREM generation and the env-sampling math as a first-class component with its own chart tests (roughness sweep on spheres must match before any set is ported).

### Risk #3 — PCFSoft shadow sampling (engine.ts:33, 70–72; world.ts:359)
Three specifics conspire: (a) three's PCFSoft kernel samples a fixed pattern of depth-texel offsets with hardware depth-compare filtering — the *softness pattern* is a fingerprint of that kernel, and wgpu/Vulkan comparison samplers filter differently at texel borders; (b) `shadow.radius = 6` (key) / `4` (interior) is set but **ignored by PCFSoftShadowMap in three** (radius only affects PCF/VSM) — a porter who "honours" radius produces softer shadows than production and fails parity by being faithful to the API instead of the behaviour; (c) the shadow camera is a tight ±3 m box at 2048² for hero shots but ±16–18 m boxes for street sets at the same 2048² (world.ts sunlight `box` 12–18), so texel density — and thus edge character — varies by set in a way that must be reproduced set-for-set. Add `bias −0.0004/−0.0006` and `normalBias 0.02` tuned against three's depth precision; wgpu's [0,1] clip-z changes depth distribution and therefore acne/peter-panning balance at the same bias values. **Mitigation:** shadow chart (angled plane + occluder sweep) per shadow-box size used in production; expect to re-derive bias in native depth units and record the mapping.

### Risk #4 — The composer chain as a system: HDR HalfFloat + 4×MSAA + pass order (engine.ts:34–44, 57–67)
No single element is exotic; the *combination* is the fingerprint. Bloom thresholds are evaluated on **linear HDR** values (emissive 2.2 bulbs bloom; diffuse 0.85-threshold crossings depend on env intensity), tone mapping happens mid-chain (OutputPass), and the grade then operates on sRGB data — so any difference in buffer precision (HalfFloat vs the native target format), MSAA resolve behaviour on the HDR target (wgpu resolves multisampled `Rgba16Float` fine, but edge coverage values feed bloom/bokeh differently), or a "helpful" reordering (grading in linear, blooming after tonemap) shifts highlights and edges film-wide. The chain also re-renders the shadow map every frame by the `needsUpdate` pattern — a native engine that caches shadows "for performance" introduces a behavioural difference the moment a shot moves a light. **Mitigation:** freeze the native graph order as HDR-render → depth/DoF → bloom → Neutral tonemap + sRGB → grade, and diff stage-by-stage (§2.1) during bring-up, not just final frames.

### Risk #5 — The CanvasTexture paint layer is the look (world.ts:26–34, 79–161, and every `canvasTex` call)
Planks, brick, wallpaper, grass, facades with typeset shop names (`Caladea, Georgia, serif`, world.ts:149–152), signs, clouds (painted with **`g.filter = 'blur(3px)'/'blur(10px)'`** — a browser-canvas filter, world.ts:176–179), foam, earth, flags: every texture is a Chrome-canvas rasterisation, with Skia's antialiasing, gradient interpolation, font shaping/hinting and blur kernels baked into the pixels. A native 2D rasteriser (tiny-skia, resvg-class, or a custom one) will produce *nearly* identical textures — differing in AA edge pixels, blur falloff, and text glyph edges — and those differences are then magnified by repeat-wrapping, anisotropy and close-ups across every set in the library. This is the broadest divergence surface in the four files (it touches every frame) even though each individual difference is sub-pixel. It is also the risk most likely to be *accepted*: a 1-texel AA difference in a plank texture is below the visual bar, while the same class of difference in the **sign/facade text** is not. **Mitigation:** per-texture atlas diffs (§3.6) with a per-texture threshold — text-bearing textures held to the strictest bar; consider keeping texture *baking* on a retained canvas implementation if font fidelity demands it (a build-time tool, not a runtime browser — compatible with the no-browser rendering rule, and a decision for the Chief Architect, not this seat).

**Honourable mentions:** `THREE.Points` star rendering (no wgpu point-size primitive — §3.4); the `Color` working-space conversion semantics (§3.7 — a silent global colour shift if mishandled, but fully deterministic once replicated); texture `map.rotation` (a three-ism the UV-transform port must not forget — it is load-bearing on the mainStreet boardwalk and every interior floor).

---

## 7. Usage counts (grep `THREE.<Name>` occurrences, all four files)

Top of the distribution (type annotations included in counts):
`Mesh` 58 · `Group` 47 · `Scene` 34 · `PlaneGeometry` 22 · `Vector3` 17 · `MeshStandardMaterial` 12+ · `SphereGeometry` 11 · `Color` 11 · `CanvasTexture` 6+ · `MeshBasicMaterial` 6 · `CircleGeometry` 6 · `Sprite` 7 (world+env) · `ConeGeometry` 5 · `PointLight` 5 (world+env) · `DirectionalLight` 6 (engine+world+env) · `InstancedMesh` 4 · `BoxGeometry` 4 · `BufferGeometry/BufferAttribute` 3+3.
engine.ts alone: 7 addon imports, 15 distinct `THREE.*` names. lib.ts: 0.

*End of A1 findings.*
