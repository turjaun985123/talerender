# FOG — integration spec (seat W28)

three.js `scene.fog` (linear `THREE.Fog`) for the native renderer:
the shared WGSL (`fog.wgsl.inc`, this directory), where it plugs in,
the uniform plumbing, the film's values, and how to prove it.

## 0. Status — read this first

The original inventory premise ("the native shaders have no fog
term") is **out of date**. The lead has already wired fog into the
film path, and this package was written against what is actually in
the tree at HEAD:

- `src/shader_pbr.wgsl` — `FrameU` carries `fog` / `fog2` / `cam_fwd`
  (lines 63–71); `fs_main` applies fog at its tail (lines 481–489).
- `src/shader_basic.wgsl` — `DrawU` carries `mv` / `fog` / `fog2` and
  a per-draw fog flag in `misc.y` (lines 21–27); `fs_main` applies
  fog after the alphaTest discard (lines 58–63).
- `src/film.rs` — `FrameSpec.fog: Option<([f32;3], f32, f32)>`
  (linear colour, near, far); set for S06 (line 1598), S07 (1668),
  S11 (1871), S12 (1919); packed into both uniform buffers
  (lines 2816–2828 PBR, 2972–2991 basic); `cam_fwd` derived at
  line 2786. Character cards and blob shadows are pushed with
  `fog: true` (lines 2163, 2192); sky dome, cloud sprites and the
  bookshop window view-plane with `fog: false`.

`fog.wgsl.inc` is **semantically identical** to those inline blocks —
extracting them into these calls must not change one pixel. What
this package adds: the canonical shared form, the exact three.js
ground truth (§1–2), the values + provenance (§5), the two-colour
rule (§6), the verification plan (§7), and the one path still
unwired — the scene-file loader (§8).

## 1. Ground truth: what three.js actually does

Extracted verbatim from the vendored `three.module.js`
(`~/workspace/custom-skills/faceless-shorts/styles/night-file/example/vendor/three.module.js`,
`ShaderChunk`):

```glsl
// fog_pars_vertex
#ifdef USE_FOG
    varying float vFogDepth;
#endif

// fog_vertex
#ifdef USE_FOG
    vFogDepth = - mvPosition.z;
#endif

// fog_fragment
#ifdef USE_FOG
    #ifdef FOG_EXP2
        float fogFactor = 1.0 - exp( - fogDensity * fogDensity * vFogDepth * vFogDepth );
    #else
        float fogFactor = smoothstep( fogNear, fogFar, vFogDepth );
    #endif
    gl_FragColor.rgb = mix( gl_FragColor.rgb, fogColor, fogFactor );
#endif
```

Consequences, all load-bearing:

- **Position in the chain.** `fog_fragment` is the last chunk of
  every built-in material's fragment shader: it mixes the **fully
  lit** colour (direct + indirect + emissive) toward the fog colour.
  Tone mapping and sRGB encode happen later (in this engine: the
  OutputPass), and the Bokeh/Bloom passes operate on the already-
  fogged buffer. Fog therefore belongs **inside the material
  shaders, pre-tonemap** — never in a post shader. The native chain
  has the same order.
- **Depth is axis depth, not distance.** `vFogDepth = -mvPosition.z`
  is the fragment's depth along the camera forward axis. A fragment
  off-axis at the same plane gets the same factor. Both native depth
  forms equal it exactly: `dot(world_pos − cam_pos, cam_fwd)` with
  `cam_fwd = normalize(target − eye)` (film.rs:2786), and
  `-view_pos.z` from the model-view matrix (basic path).
- **Colour space.** `fogColor` is the `THREE.Fog` colour in linear
  working space (three decodes the hex on assignment). Native:
  `hex_lin("#C9CFC2")`. Never pass sRGB bytes as the uniform.
- **Alpha is never fogged** (`gl_FragColor.rgb` only).
- **Per-material opt-out.** A material with `fog: false` compiles
  without `USE_FOG`. In this film: the sky dome and the cloud
  sprites opt out (world.ts:150, world.ts:169). Everything else —
  standard materials **and the character cards** — is fogged
  (env.ts:13–14: the card material "honours scene.fog like every
  standard material in the set, so people and world fade with
  distance consistently").

## 2. The formula — smoothstep, NOT the classic ramp

The classic fixed-function linear fog factor,
`clamp((far − depth) / (far − near), 0, 1)`, is **not** what
three.js computes. three's linear-`Fog` branch is
`smoothstep(fogNear, fogFar, vFogDepth)` — the Hermite curve
`t²(3 − 2t)` over `t = clamp((depth − near)/(far − near), 0, 1)`.
(WGSL `smoothstep` is the identical polynomial to GLSL's.)

Over the S06 range (near 4.5, far 20) the two curves differ by up to
**0.078 in factor** — tens of sRGB codes on distant geometry:

| view depth | three (smoothstep) | classic ramp | Δ |
|---|---|---|---|
| 3 m  | 0.0000 | 0.0000 | 0 |
| 6 m  | 0.0263 | 0.0968 | −0.0705 |
| 10 m | 0.2884 | 0.3548 | −0.0664 |
| 15 m | 0.7550 | 0.6774 | +0.0776 |
| 20 m | 1.0000 | 1.0000 | 0 |
| 25 m | 1.0000 | 1.0000 | 0 |

The 6 / 10 / 15 m points are the discriminators in the verification
plan (§7): a classic-ramp build fails them by ≫ any sane tolerance.
Edge behaviour: depth ≤ near → 0, depth ≥ far → 1 (the clamp inside
smoothstep). `near == far` would be undefined in both GLSL and WGSL
smoothstep; no value in the film comes close.

`FOG_EXP2` is included in the .inc for completeness and is **unused**:
every fogged shot in this film constructs `THREE.Fog`; `FogExp2`
appears nowhere in `env.ts` / `shots.ts`.

## 3. Insertion points

### 3.1 PBR — `src/shader_pbr.wgsl`

- Uniform: `FrameU` gains its last three fields **after
  `point3_misc`** (struct lines 63–71 as of HEAD):

  ```wgsl
  fog: vec4f,    // rgb: fog colour (linear), w: near
  fog2: vec4f,   // x: far, y: enabled (0/1)
  cam_fwd: vec4f,
  ```

- Application: the **tail of `fs_main`**, after
  `let outgoing = direct_diffuse + … + draw.emissive.rgb;` and
  before the return (lines 481–489 as of HEAD). As wired inline:

  ```wgsl
  var out_col = outgoing;
  if (frame.fog2.y > 0.5) {
      let fdepth = dot(in.world_pos - frame.cam_pos.xyz, frame.cam_fwd.xyz);
      let ffactor = smoothstep(frame.fog.w, frame.fog2.x, fdepth);
      out_col = mix(out_col, frame.fog.rgb, ffactor);
  }
  return vec4f(out_col, 1.0);
  ```

  Include form: `out_col = tr_fog_apply_frame(outgoing, in.world_pos, frame.cam_pos.xyz, frame.cam_fwd.xyz, frame.fog, frame.fog2);`

- There is **no per-draw flag on the PBR path** and none is needed:
  every PBR material in the source rig has `material.fog == true`.
  (If a future staging needs a fog-exempt standard material, the
  flag goes in `DrawU.mat_params` — currently `w` is `has_map`; do
  not overload it silently.)

### 3.2 Basic / card — `src/shader_basic.wgsl`

- Uniform: `DrawU` (lines 19–28 as of HEAD): the M1 fields
  (`mvp`, `tint`, `misc`) plus the film extension — `mv: mat4x4f`
  (model-view), `fog: vec4f`, `fog2: vec4f`. `misc.y` is the
  per-draw fog flag (`material.fog` of the source draw).
- Vertex: `vs_main` already outputs `view_pos = (mv * pos).xyz`.
- Application: in `fs_main`, **after the alphaTest discard**, before
  the return (lines 58–63 as of HEAD):

  ```wgsl
  if (draw.misc.y > 0.5 && draw.fog2.y > 0.5) {
      let ffactor = smoothstep(draw.fog.w, draw.fog2.x, -in.view_pos.z);
      rgba = vec4f(mix(rgba.rgb, draw.fog.rgb, ffactor), rgba.a);
  }
  ```

  Both gates are required: scene fog present (`fog2.y`) **and**
  this draw honouring fog (`misc.y`). Include form:
  `rgba = tr_fog_apply_basic(rgba, in.view_pos, draw.misc.y > 0.5, draw.fog, draw.fog2);`
- The caption overlay draws through this pipeline post-GRADE with
  the flag **0** — captions are screen-space and must never fog.

### 3.3 Where fog must NOT go

`shader_depth.wgsl`, `shader_depth_pos.wgsl`,
`shader_shadow_card.wgsl` (depth/shadow materials — three does not
fog depth output), `shader_bokeh/bloom/output/grade` (post chain),
`shader_tex.wgsl` (M1 stand-in, not in the composer chain).

## 4. Uniform plumbing (Rust side)

**PBR `FrameU` = 448 bytes** (film.rs asserts this:
`debug_assert_eq!(frame_bytes.len(), 448)`). Layout, in order:
`vp` (64) + `shadow_vp` (64) + 11 vec4 of rig (176) + point2/point3
(6 vec4, 96) + **`fog` @ byte 400, `fog2` @ 416, `cam_fwd` @ 432**.
Every field is a `vec4f` or a mat4 column — natural 16-byte
alignment throughout, no padding. Packing (film.rs:2816–2828):

```rust
match &spec.fog {
    Some((col, near, far)) => {
        vecs.push([col[0], col[1], col[2], *near]);
        vecs.push([*far, 1.0, 0.0, 0.0]);
    }
    None => { vecs.push([0.0; 4]); vecs.push([0.0; 4]); }
}
vecs.push(v4(cam_fwd)); // cam_fwd = norm3(target − eye), film.rs:2786
```

**Basic `DrawU` = 192 bytes**: `mvp` (64) + `tint` (16) + `misc`
(16) + `mv` (64) + `fog` (16) + `fog2` (16). The scene fog is
**copied into every basic draw's** `fog`/`fog2` (film.rs:2972–2991);
the per-draw difference is only the `misc.y` flag.

**Enable semantics:** `fog2.y = 0` must skip the fog block entirely
(not merely produce factor 0). The ab1 path zero-fills these fields
(ab1.rs:665–667, 1292–1294) and the noon shots pass `fog: None` —
both render **bit-identically** to the pre-fog engine. That property
is a regression gate (§7).

## 5. Values — every fogged shot in the film

All colours below are source hex; the uniform carries their
`hex_lin` (linear) decode. Provenance is first-hand from the project
source (`~/faceless-shorts/the-soldier-who-didnt-know/src/`).

| Shot(s) | Colour | Near | Far | Where it comes from |
|---|---|---|---|---|
| S06 | `#C9CFC2` | 4.5 | 20 | **Shot override**, shots.ts:409 (below) |
| S07 | `#FFD98A` | 30 | 140 | golden preset, env.ts:89 |
| S11, S12 | `#F59C6B` | 28 | 120 | dusk preset, env.ts:101 |
| all others | — (fog off) | — | — | noon preset fog is `null`, env.ts:79 |

**The mechanism (env.ts:168, inside `applyAtmosphere`):**

```ts
scene.fog = preset.fog ? new THREE.Fog(preset.fog.color, preset.fog.near, preset.fog.far) : null;
```

Every shot calls `applyAtmosphere`, so preset fog **is** scene fog
unless the shot replaces it.

**S06's override chain.** S06 calls
`applyAtmosphere(JG.scene, 'fog', { rig: JG.sun })` — which assigns
the fog *preset's* fog `{ #E6E9EA, 3, 38 }` (env.ts, fog preset) —
and the same draw then replaces it (shots.ts:409):

```ts
JG.scene.fog = new THREE.Fog(0xC9CFC2, 4.5, 20);
```

The preset values are **never rendered**. Native film.rs:1598
matches: `f.fog = Some((hex_lin("#C9CFC2"), 4.5, 20.0))`. The scene
JSON carries the same block (`scenes/film/s06.scene.json` →
`"fog": {"color": "#C9CFC2", "near": 4.5, "far": 20.0}`), as do
s07 / s11 / s12 with their rows of the table.

### S11 / S12 verdict: real scene.fog, NOT grade-only

The dusk haze in S11/S12 is sometimes described as a grade effect.
It is not. The chain is fully source-backed:

1. The **dusk preset carries a fog entry**: env.ts:101 —
   `fog: { color: '#F59C6B', near: 28, far: 120 }`.
2. S11 and S12 call `applyAtmosphere(JG.scene, 'dusk', { rig: JG.sun })`
   (shots.ts:504, shots.ts:529), and `applyAtmosphere` assigns
   `scene.fog` from the preset (env.ts:168). Neither shot overrides
   or clears it afterwards.
3. The dusk *grade* is a separate, additional term
   (`gradeOf(G.jungle, dusk)` → sat 1.08 / warm 0.06 / exposure
   0.97); it coexists with the fog, it does not replace it.

Native film.rs agrees (lines 1871, 1919:
`f.fog = Some((hex_lin("#F59C6B"), 28.0, 120.0))` for both shots),
and the data files record the same `fog` block. The same reasoning
covers S07 (golden preset fog, env.ts:89; shots.ts:427).

## 6. The two-colour rule — scene fog ≠ character tint

Two **independent** mechanisms mix character colour toward a fog
colour in the fogged shots. They stack; never merge or substitute
them.

1. **Scene fog (this spec)** — per-fragment, depth-driven,
   applies to *every* fog-honouring fragment including the cards.
   S06 colour: **`#C9CFC2`** (the shot override).
2. **Character env tint (`matchEnv`)** — a flat material-colour
   replacement on the toon planes (toon.ts:664–674), whose last
   step is `c.lerp(fogColor, fogMix)` (toon.ts:670). It has **no
   depth term**. For S06 its colour is the **preset's** fog colour
   **`#E6E9EA`** with `fogMix 0.35` (fog preset tint, env.ts) —
   *not* the overridden scene-fog colour. Native: film.rs
   `env_tint()` (line 1142) + `tint_fog()` (line 1261), passed as
   the card draw's `tint`.

Order on a card fragment (both engines): paint × tint (incl. the
`#E6E9EA` lerp) → alphaTest → fog mix toward `#C9CFC2` by depth.
In S07 / S11 / S12 the two colours coincide (preset design rule,
env.ts:57–60: "fog colour === horizon colour … tint.fogColor
follows the same colour"), but the terms remain distinct — flat
`fogMix` 0.06 (golden) / 0.08 (dusk) versus the depth factor.

Opt-outs, for the record: sky dome (`fog: false`, world.ts:150),
cloud sprites (`fog: false`, world.ts:169), caption overlays
(screen-space). The blob shadow and the cards are fogged.

## 7. Verification plan — the fog chart

A dedicated chart scene, in the spirit of the M3 dof-chart: one
shared spec driving both engines, flat swatches whose expected
colours are known analytically.

**Scene.** Five vertical panels centred on the camera axis at
forward-axis depths **3, 6, 10, 15, 25 m** (panel size ~1.5 m;
centred on-axis so axis depth == placement depth — off-axis corners
share the centre's factor by the axis-depth rule, a panel-interior
uniformity check in itself). Each depth in two materials:

- an **unlit basic** panel (MeshBasicMaterial ↔ native basic path),
  flat colour `#7A8B99`;
- a **standard PBR** panel (MeshStandardMaterial, roughness 1,
  metalness 0) of the same colour, lit by a fixed neutral rig.

Scene fog = the S06 values: `#C9CFC2`, near 4.5, far 20. Camera at
the origin looking down −Z (native) / +Z convention per engine,
target on-axis. Post: the film's composer with **aperture 0**
(bokeh disabled per engine.ts's enable rule) and a neutral grade,
so the chart isolates fog the way the dof-chart isolated bokeh.

**Expected factors** (linear fog, §2): 3 m → 0 · 6 m → 0.0263 ·
10 m → 0.2884 · 15 m → 0.7550 · 25 m → 1.0000. Expected panel
colour = `mix(base_linear, fog_linear, factor)` pushed through the
shared OutputPass; compute the reference numbers from the formula,
not from either renderer.

**References.** (a) the analytic values; (b) a Chrome capture of
the same chart through the real skill-template engine/composer
(SwiftShader — labelled as such; its master is JPEG, so judge panel
*centres*, not edges).

**Pass bars.**

- Panel-centre mean of each native panel within **±2 sRGB codes**
  of the analytic value after the OutputPass, both materials.
- The 6 / 10 / 15 m panels discriminate the formula: the classic
  ramp predicts factors 0.0705 / 0.0664 / 0.0776 away from three's —
  a wrong-formula build misses the bar by an order of magnitude.
- Native-vs-Chrome full-chart PSNR ≥ 45 dB (flat regions; the
  Chrome side's JPEG floor is the bound, as at M3).
- Secondary spot-check with the golden (30/140) and dusk (28/120)
  ranges at 60 m: factors 0.1826 / 0.2788 respectively.

**Regressions (must stay exact).**

- The ab1 frame and a noon film frame render **bit-identical**
  before/after (the `fog2.y = 0` skip).
- dof-chart unchanged (fog off there).
- In-film check: a native S06 frame vs the project's Chrome review
  frame inside the fog window (≈ t 28–30 s) — staging-dependent,
  so judged like the §28 A/B (PSNR reported, differences itemised),
  not as the gate.

## 8. The one unwired path: the scene-file loader

`src/scenefile.rs` does not parse the `fog` block that
`s06/s07/s11/s12.scene.json` already carry (the film evaluator in
film.rs is the only consumer today). Wiring it needs **no shader
change** — only: read the top-level `fog {color, near, far}` →
`hex_lin(color)` → the same `FrameSpec.fog`-equivalent the loader
already forwards to the PBR `FrameU` and basic `DrawU` fields
described in §4, with `fog2.y = 1`. Absent block → `None` → the
bit-identical no-fog path.

## 9. Residual risks

- **llvmpipe vs RADV fp16** (from M3): fog arithmetic is f32
  smoothstep/mix — no fp16 storage involved; no driver sensitivity
  expected. The chart (§7) runs on both anyway.
- **Panel-edge filtering**: texture filtering at panel borders can
  bleed unfogged colour into edge pixels in both engines; the
  chart judges centres only.
- **The basic path's per-draw fog copy**: because film.rs copies
  the scene fog into every basic draw's uniforms, a draw added
  with `fog: true` in a `fog: None` frame is safe (global enable is
  0) — but a draw with `fog: false` in a fogged frame is the *only*
  opt-out; new basic draws must choose their flag deliberately
  (sky/cloud/caption = false; world-attached = true).
