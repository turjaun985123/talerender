# SHADERS-W29 — Static WGSL shader checker (Wave 2, seat W29)

**Deliverable:** `tools/ci/check_shaders.py` (stdlib-only Python, no
dependencies, no network). This note is its run record and limits
statement. Nothing in `src/` was edited by this seat; findings are
reported, not fixed.

**Run it:**

```bash
python3 tools/ci/check_shaders.py --repo .          # human report
python3 tools/ci/check_shaders.py --repo . --json   # machine report
python3 tools/ci/check_shaders.py --repo . -v       # + INFO notes
```

Exit codes: `0` = every shader PASS · `1` = any FAIL or UNRESOLVED ·
`2` = discovery itself failed. Runtime ≈ 3–7 s on this VM.

**Headline verdict: 21/21 shaders PASS, 0 FAIL, 0 UNRESOLVED,
0 unresolved sites. No real shader↔Rust mismatch was found.**
(21 = 15 `.wgsl` files + `COPY_WGSL` + the `blur_wgsl` template
instantiated at its 5 production radii.) Line numbers below are
from the run recorded here; the wave is landing concurrently, so
individual lines may drift — the checker re-derives them every run.

---

## 1. What the checker verifies

Per shader (files under `src/`, plus the two embedded sources in
`src/composer.rs`):

1. **Delimiter balance** (`{}`, `()`, `[]`) — a smoke layer only.
   This is **not** a WGSL parser (see §7).
2. **Entry points** — at least one declared; names are sane
   identifiers and not WGSL reserved words; no duplicates.
3. **Binding hygiene** — every `@group` is paired with a
   `@binding`; `(group, binding)` is unique per file; every
   `var<uniform>`'s type names a struct defined in the same file.
4. **Shader ↔ Rust layout agreement, per pipeline site** — binding
   numbers and binding *kinds* (uniform / texture / sampler /
   storage) per group; the pipeline's vertex/fragment entry-point
   names against the shader's declared entry points; and
   stage-visibility coverage (a binding the shader reads in a
   stage — tracked through the shader's own call graph — must be
   visible in that stage in the Rust layout; surplus visibility
   is reported as INFO, it is legal wgpu).
5. **Uniform struct pairs** — for the pairs in §4, the WGSL
   struct's layout size under the WGSL uniform-address-space
   rules is compared against the Rust side that fills the buffer.

**How the shader↔site map is built (evidence, not filenames):**
`include_str!("…wgsl")` sites, `const NAME: &str = include_str!(…)`
indirection (dof.rs), and the `module("label", src)` closure calls
in ab1.rs / film.rs / composer.rs give module variable → shader.
Pipelines are matched to layouts by the *nearest preceding
definition of the name they use* in the same file; template
pipelines (`make_pbr`, `make_depth`, `fs_pipeline`) are resolved
through their call sites, argument by argument. Bind-group layouts
are read from literal `create_bind_group_layout` entry lists, plus
anchor-verified interpreters for the helper shapes this codebase
actually uses (`tex_bgl`, `ubo_layout`, `uniform_bgl`, composer.rs
`bgl("…", &[ubo_entry(0), …])`). Each interpreter re-verifies the
helper's definition shape before trusting it; a changed shape, a
missing anchor, or an unrecognised expression resolves to
**UNRESOLVED — a failing verdict, never a silent pass.** A final
coverage sweep asserts every `create_shader_module` in the tree
is mapped to a shader source (0 unmapped in this run).

Totals for this run: **45 site rows** (41 distinct code sites;
the single blur code site is fanned out to its 5 radii) and
**34 pair rows** (30 distinct pair sites; the ab1 DrawU row
covers 3 assemblies).

## 2. The shader ↔ layout map (as derived from source)

Kinds: `U` uniform, `T` texture, `S` sampler. All shaders use
group 0 only, except `shader_pbr.wgsl` (groups 0+1).

| Shader | Bindings (group: binding=kind) | Rust sites (layout evidence) |
| --- | --- | --- |
| shader3d | 0: 0=U | main.rs `bgl3d` {0=U} |
| shader_tex | 0: 0=U 1=T 2=S | main.rs `bgl_tex`; dof.rs `tex_bgl`; scenefile.rs `tex_bgl()` — all {0=U,1=T,2=S} |
| shader_scene3d | 0: 0=U | scenefile.rs `uniform_bgl(true)` {0=U} (×2 paths) |
| shader_depth | 0: 0=U | dof.rs `depth_bgl`; ab1/film `ubo_layout` → `depth_layout` {0=U} |
| shader_depth_pos | 0: 0=U | scenefile.rs `uniform_bgl(false)`; ab1/film `depth_layout` {0=U} |
| shader_shadow_card | 0: 0=U 1=T 2=S | ab1/film `tex_bgl(true)` → `shadow_card_layout` |
| shader_basic | 0: 0=U 1=T 2=S | ab1/film `tex_bgl(true)` → `basic_layout` |
| shader_pbr | 0: 0=U 1=U · 1: 0=T 1=S 2=T 3=T 4=S | ab1/film `pbr_bgl0` {0=U,1=U}, `pbr_bgl1` {0=T,1=S,2=T,3=T,4=S}; vertex entries `vs_colour`/`vs_tex`/`vs_map` via `make_pbr` call args, fragment `fs_main` |
| shader_bokeh | 0: 0=U 1=T 2=S 3=T 4=S | identical 5-entry literal BGLs in dof.rs, ab1.rs, film.rs, scenefile.rs |
| shader_output | 0: 0=T 1=S | dof.rs `output_bgl` |
| shader_output_exp | 0: 0=T 1=S 2=U | scenefile.rs `output_bgl`; composer.rs `bgl("output layout", [tex 0, smp 1, ubo 2])` |
| shader_fog | 0: 0=U 1=T 2=S 3=T 4=S | scenefile.rs inline BGL in the fog block |
| shader_bloom_bright | 0: 0=U 1=T 2=S | composer.rs `bgl("bloom bright layout", [ubo 0, tex 1, smp 2])` |
| shader_bloom_composite | 0: 0=U 1–5=T 6=S | composer.rs `bgl("bloom composite layout", …)` |
| shader_grade | 0: 0=U 1=T 2=S | composer.rs `bgl("grade layout", …)` |
| COPY_WGSL (embedded) | 0: 0=T 1=S | composer.rs `bgl("bloom add layout", [tex 0, smp 1])` |
| blur_wgsl (generated) | 0: 0=U 1=T 2=S | composer.rs `bgl("bloom blur layout", [ubo 0, tex 1, smp 2])` |

## 3. Verbatim run (working tree, 2026-11-14)

```text
PASS       embedded:COPY_WGSL  (const COPY_WGSL in src/composer.rs)
    site PASS: src/composer.rs:309 (fs_pipeline via fs_pipeline call at line 432)
PASS       embedded:blur_wgsl(r=11)  (blur_wgsl() template in src/composer.rs, instantiated radius=11 (of [3, 5, 7, 9, 11]), horizontal; coefficient VALUES are runtime f64 formatting and are substituted with 0.0 — only structure is checked)
    site PASS: src/composer.rs:309 (fs_pipeline via fs_pipeline call at line 404)
    pair PASS: src/composer.rs [pack_array '"bloom blur uniforms"']: 4 f32 slots = 16 B == WGSL 16 B (anchor line 511); slot order positional
PASS       embedded:blur_wgsl(r=3)  (blur_wgsl() template in src/composer.rs, instantiated radius=3 (of [3, 5, 7, 9, 11]), horizontal; coefficient VALUES are runtime f64 formatting and are substituted with 0.0 — only structure is checked)
    site PASS: src/composer.rs:309 (fs_pipeline via fs_pipeline call at line 404)
    pair PASS: src/composer.rs [pack_array '"bloom blur uniforms"']: 4 f32 slots = 16 B == WGSL 16 B (anchor line 511); slot order positional
PASS       embedded:blur_wgsl(r=5)  (blur_wgsl() template in src/composer.rs, instantiated radius=5 (of [3, 5, 7, 9, 11]), horizontal; coefficient VALUES are runtime f64 formatting and are substituted with 0.0 — only structure is checked)
    site PASS: src/composer.rs:309 (fs_pipeline via fs_pipeline call at line 404)
    pair PASS: src/composer.rs [pack_array '"bloom blur uniforms"']: 4 f32 slots = 16 B == WGSL 16 B (anchor line 511); slot order positional
PASS       embedded:blur_wgsl(r=7)  (blur_wgsl() template in src/composer.rs, instantiated radius=7 (of [3, 5, 7, 9, 11]), horizontal; coefficient VALUES are runtime f64 formatting and are substituted with 0.0 — only structure is checked)
    site PASS: src/composer.rs:309 (fs_pipeline via fs_pipeline call at line 404)
    pair PASS: src/composer.rs [pack_array '"bloom blur uniforms"']: 4 f32 slots = 16 B == WGSL 16 B (anchor line 511); slot order positional
PASS       embedded:blur_wgsl(r=9)  (blur_wgsl() template in src/composer.rs, instantiated radius=9 (of [3, 5, 7, 9, 11]), horizontal; coefficient VALUES are runtime f64 formatting and are substituted with 0.0 — only structure is checked)
    site PASS: src/composer.rs:309 (fs_pipeline via fs_pipeline call at line 404)
    pair PASS: src/composer.rs [pack_array '"bloom blur uniforms"']: 4 f32 slots = 16 B == WGSL 16 B (anchor line 511); slot order positional
PASS       src/shader3d.wgsl  (file src/shader3d.wgsl)
    site PASS: src/main.rs:563 (run)
    pair PASS: src/main.rs [rust_struct 'Uniforms']: field sequence identical (2 fields, 128 B; Rust struct line 365)
PASS       src/shader_basic.wgsl  (file src/shader_basic.wgsl)
    site PASS: src/ab1.rs:963 (make_basic)
    site PASS: src/film.rs:2674 (make_basic)
    pair PASS: src/ab1.rs [extends 'let mut bytes: Vec<u8> = Vec::with_capacity(192)']: 1 assembly reconstructs 192 B == WGSL 192 B (2 mat extends = 128 B; 2 vec4 groups = 32 B; zero fill 32 B; anchor line(s) 1285); slot order positional
    pair PASS: src/film.rs [extends 'Vec::with_capacity(192)']: 1 assembly reconstructs 192 B == WGSL 192 B (2 mat extends = 128 B; 2 vec4 groups = 32 B; 2 vec4 groups = 32 B; anchor line(s) 3055); slot order positional
PASS       src/shader_bloom_bright.wgsl  (file src/shader_bloom_bright.wgsl)
    site PASS: src/composer.rs:309 (fs_pipeline via fs_pipeline call at line 397)
    pair PASS: src/composer.rs [pack_array '"bloom bright uniforms"']: 4 f32 slots = 16 B == WGSL 16 B (anchor line 496); slot order positional
PASS       src/shader_bloom_composite.wgsl  (file src/shader_bloom_composite.wgsl)
    site PASS: src/composer.rs:309 (fs_pipeline via fs_pipeline call at line 414)
    pair PASS: src/composer.rs [pack_array '"bloom composite uniforms"']: 4 f32 slots = 16 B == WGSL 16 B (anchor line 523); slot order positional
PASS       src/shader_bokeh.wgsl  (file src/shader_bokeh.wgsl)
    site PASS: src/ab1.rs:1060 (run_ab1)
    site PASS: src/dof.rs:354 (run_dof)
    site PASS: src/film.rs:2745 (new)
    site PASS: src/scenefile.rs:1888 (run_composer)
    pair PASS: src/dof.rs [pack_array 'let bokeh_uniform_data']: 8 f32 slots = 32 B == WGSL 32 B (anchor line 322); slot order positional
    pair PASS: src/ab1.rs [pack_array 'let bokeh_uniform_data']: 8 f32 slots = 32 B == WGSL 32 B (anchor line 1027); slot order positional
    pair PASS: src/scenefile.rs [pack_array 'let bokeh_uniform_data']: 8 f32 slots = 32 B == WGSL 32 B (anchor line 1861); slot order positional
    pair PASS: src/film.rs [pack_array 'pack_f32(&[spec.focus']: 8 f32 slots = 32 B == WGSL 32 B (anchor line 3117); slot order positional
PASS       src/shader_depth.wgsl  (file src/shader_depth.wgsl)
    site PASS: src/ab1.rs:998 (make_depth via make_depth call at line 1016)
    site PASS: src/dof.rs:295 (run_dof)
    site PASS: src/film.rs:2706 (make_depth via make_depth call at line 2724)
    pair PASS: src/dof.rs [rust_struct 'Uniforms']: field sequence identical (2 fields, 128 B; Rust struct line 63)
    pair PASS: src/ab1.rs [rust_struct 'Uniforms']: field sequence identical (2 fields, 128 B; Rust struct line 545)
    pair PASS: src/film.rs [fn_mats 'depth_bytes']: fn assembles 128 B == WGSL 128 B (line 2322)
PASS       src/shader_depth_pos.wgsl  (file src/shader_depth_pos.wgsl)
    site PASS: src/ab1.rs:998 (make_depth via make_depth call at line 1014)
    site PASS: src/ab1.rs:998 (make_depth via make_depth call at line 1015)
    site PASS: src/ab1.rs:998 (make_depth via make_depth call at line 1017)
    site PASS: src/film.rs:2706 (make_depth via make_depth call at line 2722)
    site PASS: src/film.rs:2706 (make_depth via make_depth call at line 2723)
    site PASS: src/film.rs:2706 (make_depth via make_depth call at line 2725)
    site PASS: src/film.rs:2706 (make_depth via make_depth call at line 2726)
    site PASS: src/scenefile.rs:1763 (run_composer)
    pair PASS: src/scenefile.rs [fn_mats 'uniform_bytes_tex']: fn assembles 128 B == WGSL 128 B (line 1078)
    pair PASS: src/ab1.rs [rust_struct 'Uniforms']: field sequence identical (2 fields, 128 B; Rust struct line 545)
    pair PASS: src/film.rs [fn_mats 'depth_bytes']: fn assembles 128 B == WGSL 128 B (line 2322)
PASS       src/shader_fog.wgsl  (file src/shader_fog.wgsl)
    site PASS: src/scenefile.rs:1838 (run_composer)
    pair PASS: src/scenefile.rs [pack_array '"fog uniforms"']: 8 f32 slots = 32 B == WGSL 32 B (anchor line 1823); slot order positional
PASS       src/shader_grade.wgsl  (file src/shader_grade.wgsl)
    site PASS: src/composer.rs:309 (fs_pipeline via fs_pipeline call at line 442)
    pair PASS: src/composer.rs [pack_array 'let data: [f32; 8]']: 8 f32 slots = 32 B == WGSL 32 B (anchor line 574); slot order positional
PASS       src/shader_output.wgsl  (file src/shader_output.wgsl)
    site PASS: src/dof.rs:387 (run_dof)
PASS       src/shader_output_exp.wgsl  (file src/shader_output_exp.wgsl)
    site PASS: src/composer.rs:309 (fs_pipeline via fs_pipeline call at line 437)
    site PASS: src/scenefile.rs:1935 (run_composer)
    pair PASS: src/composer.rs [pack_array '"output uniforms"']: 4 f32 slots = 16 B == WGSL 16 B (anchor line 547); slot order positional
    pair PASS: src/scenefile.rs [pack_array '"output uniforms"']: 4 f32 slots = 16 B == WGSL 16 B (anchor line 1925); slot order positional
PASS       src/shader_pbr.wgsl  (file src/shader_pbr.wgsl)
    site PASS: src/ab1.rs:929 (make_pbr via make_pbr call at line 945)
    site PASS: src/ab1.rs:929 (make_pbr via make_pbr call at line 946)
    site PASS: src/ab1.rs:929 (make_pbr via make_pbr call at line 953)
    site PASS: src/film.rs:2642 (make_pbr via make_pbr call at line 2658)
    site PASS: src/film.rs:2642 (make_pbr via make_pbr call at line 2659)
    site PASS: src/film.rs:2642 (make_pbr via make_pbr call at line 2666)
    pair PASS: src/ab1.rs [extends 'let mut frame_bytes']: 1 assembly reconstructs 448 B == WGSL 448 B (2 mat extends = 128 B; 20 vec4 groups = 320 B; anchor line(s) 650); slot order positional
    pair PASS: src/film.rs [extends 'let mut frame_bytes']: 1 assembly reconstructs 448 B == WGSL 448 B (2 mat extends = 128 B; vec! init 7 vec4 = 112 B; 11 pushed/extended vec4 = 176 B; match arms agree on 2 vec4 = 32 B; anchor line(s) 2863); slot order positional
    pair PASS: src/ab1.rs [extends_all 'let mut draw_bytes']: 3 assemblies reconstruct 176 B == WGSL 176 B (2 mat extends = 128 B; 3 vec4 groups = 48 B; anchor line(s) 1146, 1201, 1244); slot order positional
    pair PASS: src/film.rs [extends 'let draw_bytes = |']: 1 assembly reconstructs 176 B == WGSL 176 B (2 mat extends = 128 B; 3 vec4 groups = 48 B; anchor line(s) 2975); slot order positional
PASS       src/shader_scene3d.wgsl  (file src/shader_scene3d.wgsl)
    site PASS: src/scenefile.rs:1263 (run_direct)
    site PASS: src/scenefile.rs:1718 (run_composer)
    pair PASS: src/scenefile.rs [fn_mats 'uniform_bytes_colour']: fn assembles 144 B == WGSL 144 B (line 1086)
PASS       src/shader_shadow_card.wgsl  (file src/shader_shadow_card.wgsl)
    site PASS: src/ab1.rs:998 (make_depth via make_depth call at line 1023)
    site PASS: src/film.rs:2706 (make_depth via make_depth call at line 2732)
    pair PASS: src/ab1.rs [rust_struct 'Uniforms']: field sequence identical (2 fields, 128 B; Rust struct line 545)
    pair PASS: src/film.rs [fn_mats 'depth_bytes']: fn assembles 128 B == WGSL 128 B (line 2322)
PASS       src/shader_tex.wgsl  (file src/shader_tex.wgsl)
    site PASS: src/dof.rs:269 (run_dof)
    site PASS: src/main.rs:646 (run)
    site PASS: src/scenefile.rs:1310 (run_direct)
    site PASS: src/scenefile.rs:1674 (run_composer)
    pair PASS: src/main.rs [rust_struct 'Uniforms']: field sequence identical (2 fields, 128 B; Rust struct line 365)
    pair PASS: src/dof.rs [rust_struct 'Uniforms']: field sequence identical (2 fields, 128 B; Rust struct line 63)
    pair PASS: src/scenefile.rs [fn_mats 'uniform_bytes_tex']: fn assembles 128 B == WGSL 128 B (line 1078)

check_shaders: 21/21 shaders PASS, 0 FAIL, 0 UNRESOLVED, 0 unresolved site(s)
```

## 4. Uniform struct pairs checked

There are **no `#[repr(C)]` structs and no bytemuck in this
crate** (verified by grep): uniforms are hand-packed f32 byte
assemblies. The pairs below are therefore WGSL struct ↔ the
Rust struct definition where one exists (`Uniforms`), or ↔ the
assembly site(s) that build the bytes. The checker verifies
field sequence for named Rust structs, and reconstructs the
byte total for assemblies (mat extends ×64 B, vec4 groups
×16 B, zero fills, `pack_f32` slot counts; for all-scalar WGSL
structs it also requires slot count == field count, so a
dropped pad field cannot hide inside the padded struct size).

| WGSL struct | Size | Rust counterpart(s) |
| --- | --- | --- |
| `Uniforms` {mvp, model} — shader3d / shader_tex / shader_depth / shader_depth_pos / shader_shadow_card | 128 B | `struct Uniforms { mvp: Mat4, model: Mat4 }` in main.rs, dof.rs, ab1.rs (field sequence identical); scenefile.rs `uniform_bytes_tex` (2 mats); film.rs `depth_bytes` (2 mats) |
| `Uniforms` {mvp, model, light} — shader_scene3d | 144 B | scenefile.rs `uniform_bytes_colour` (2 mats + `[f32; 4]` light param) |
| `FrameU` (2 mat4 + 20 vec4) — shader_pbr | 448 B | ab1.rs `frame_bytes` assembly (line 650); film.rs `frame_bytes` assembly (line 2863, Vec-accumulator form) |
| `DrawU` (2 mat4 + 3 vec4) — shader_pbr | 176 B | ab1.rs three `draw_bytes` assemblies (lines 1146, 1201, 1244 — all three checked); film.rs `draw_bytes` closure (line 2975) |
| `DrawU` (mat4, vec4, vec4, mat4, vec4, vec4) — shader_basic | 192 B | ab1.rs `prepare_basic` (line 1285); film.rs basic draw assembly (line 3055) |
| `BokehUniforms` (8 × f32) — shader_bokeh | 32 B | `[f32; 8]` packs in dof.rs, ab1.rs, scenefile.rs, film.rs |
| `U` {threshold: vec4} — shader_bloom_bright | 16 B | composer.rs line 496 `[threshold, 0, 0, 0]` |
| `U` {params: vec4} — shader_bloom_composite | 16 B | composer.rs line 523 `[strength, radius, 0, 0]` |
| `U` {params, time} — shader_grade | 32 B | composer.rs line 574 `[f32; 8]` |
| `U` {exposure: vec4} — shader_output_exp | 16 B | composer.rs line 547; scenefile.rs line 1925 — `[exposure, 0, 0, 0]` |
| `U` {fog_color, params} — shader_fog | 32 B | scenefile.rs line 1823 pack (8 slots) |
| `U` {inv_size: vec4} — blur_wgsl | 16 B | composer.rs line 511 `inv = [1/w, 1/h, 0, 0]` |

**Slot order (human audit layer).** For packed assemblies the
checker verifies counts and sizes; the *semantic* order was
audited by reading each assembly against the WGSL field order —
all agree, e.g.:

- `BokehUniforms` (focus, aspect, aperture, maxblur, near_clip,
  far_clip, _pad0, _pad1) ↔ ab1.rs:1027 `[6.051, aspect, 0.0012,
  0.007, 0.05, 800.0, 0.0, 0.0]`; film.rs `[spec.focus, aspect,
  spec.post.aperture, spec.post.maxblur, 0.05, 800.0, 0.0, 0.0]`;
  scenefile.rs `[bokeh.focus, aspect, bokeh.aperture,
  bokeh.maxblur, cam.near, cam.far, 0.0, 0.0]`.
- `FrameU`: both assemblies write `vp`, `shadow_vp`, then the
  vec4 fields in WGSL declaration order (cam_pos … cam_fwd);
  film.rs writes point_misc as `[dist, decay, env, key_bias]`,
  matching the WGSL comment (x distance, y decay, z
  env_intensity, w shadow bias), and shadow_misc as
  `[1/2048, 2048, 0, 0]`.
- GRADE `U`: composer.rs:574 `[vignette, grain, warm, sat,
  time, 0, 0, 0]` ↔ WGSL params = (uVig, uGrain, uWarm, uSat),
  time.x = uTime.
- fog `U`: scenefile.rs pack = `[color.rgb, near, far,
  cam.near, cam.far, 0]` ↔ fog_color = (rgb, near),
  params = (far, cam near, cam far, –).
- basic `DrawU`: misc = (alpha_test, fog flag, 0, 0), fog =
  (rgb, near), fog2 = (far, enabled, 0, 0) on the film path
  (film.rs:3035–3052); the ab1 path writes the same slots with
  fog disabled (zeros), as its comments state.

## 5. String-built shaders — coverage

Two shader sources are assembled in Rust strings, both in
`src/composer.rs`; a scan of every `create_shader_module` source
expression and of WGSL markers (`@group`, `@vertex`) across
`src/*.rs` finds no others (the coverage sweep in the checker
independently asserts every module creation is mapped):

- **`COPY_WGSL`** (const, composer.rs) — **fully in scope**:
  extracted verbatim and checked like a file shader (bindings
  {0=T, 1=S} vs the "bloom add layout"; entry points; balance).
- **`blur_wgsl(horizontal, radius)`** (`format!`-generated,
  composer.rs:181) — **in scope as a template**: the checker
  un-doubles the `format!` braces and instantiates it with
  `radius ∈ BLOOM_KERNEL_RADII = [3, 5, 7, 9, 11]`
  (composer.rs:36) and the horizontal direction, then runs the
  full check set per instance (all 5 PASS). Two honest limits:
  (a) the vertical instance differs only in the substituted
  `{dir}` constant (`vec2f(0.0, 1.0)` vs `vec2f(1.0, 0.0)`) —
  same structure, not separately instantiated; (b) the kernel
  coefficient *values* are runtime f64→string formatting, so
  the checker substitutes syntactically valid floats — it
  verifies the generated shader's structure, never the
  numerics of the kernel.

## 6. Observations (not mismatches — reported for the lead)

No FAIL and no UNRESOLVED verdicts: there is no shader↔layout
or uniform-size mismatch in the tree. Three observations from
the evidence, none of them defects today:

1. **ab1.rs:650 — stale capacity hint.** The FrameU assembly
   starts `Vec::with_capacity(304)` but appends 448 B (the Vec
   simply grows; the buffer is created with
   `size: frame_bytes.len()`, so output is correct). film.rs's
   twin assembly uses `with_capacity(448)` (line 2863) and adds
   `debug_assert_eq!(frame_bytes.len(), 448)` (line 2907);
   ab1.rs has no equivalent length assert. A reader could
   mistake 304 for the struct size — worth a one-line alignment
   when the lead next touches ab1.rs.
2. **Visibility supersets (4, all legal).** `-v` reports four
   bindings whose Rust layout grants `VERTEX | FRAGMENT` while
   the shader reads the uniform in the vertex stage only:
   shader_scene3d via scenefile.rs `uniform_bgl(fragment=true)`
   (both scene paths) and shader_shadow_card via ab1/film
   `tex_bgl(true)`. wgpu accepts this; listed so the fact is on
   record, not because action is needed.
3. **The checker is fully static** — it never executes a
   shader, so this VM's known llvmpipe JIT segfault while
   compiling `shader_pbr.wgsl` cannot mask a structural error
   here (and conversely a PASS here says nothing about that
   JIT issue, which remains a runtime/CI concern).

## 7. Negative controls (the checker bites)

Planted in a scratch copy of `src/` (the real tree untouched);
each produced the designed verdict and exit code 1 overall:

| Planted defect | Result |
| --- | --- |
| shader_grade.wgsl `@binding(1)` → `@binding(7)` | FAIL — "group 0 binding 7 ('tex', texture) missing from Rust layout [composer.rs bgl(…) evidence]" |
| shader_tex.wgsl `fs_main` renamed in WGSL only | FAIL at all 4 usage sites — pipeline fragment entry not declared by the shader |
| shader_bokeh.wgsl `_pad1` field deleted | FAIL ×4 pair sites — "8 f32 slots vs 7 scalar WGSL fields" |
| shader_fog.wgsl unbalanced brace appended | FAIL — delimiter balance smoke check |
| scenefile.rs `uniform_bytes_tex` renamed | UNRESOLVED (loud) — pair anchor no longer matches; never a silent pass |

## 8. Limits — what this checker cannot catch

This is a structural cross-checker, not a compiler. **naga /
tint / wgpu validation catch all of the following; this tool
catches none of them:**

- Real WGSL parsing: missing semicolons, malformed declarations,
  bad attribute syntax — anything beyond delimiter balance.
- Type checking: expression types, implicit conversions,
  swizzle/component validity, builtin function signatures and
  argument types, operator overloads, `textureSample*` argument
  shapes beyond the binding's existence/kind.
- Name resolution: undeclared identifiers, use-before-declare,
  shadowing and scope rules, function overload resolution.
- Stage-interface matching: vertex outputs vs fragment inputs
  (`@location` sets and types between stages), and vertex
  *input* `@location`s vs the Rust `VertexBufferLayout`
  attribute lists (a natural extension; not implemented).
- Sampler/texture compatibility rules: filtering vs
  non-filterable formats, comparison samplers vs
  `textureSampleCompare`, depth-texture rules, storage-texture
  access/format rules, binding `count` (arrays of resources).
- Const evaluation and limits: constant-expression validity,
  workgroup-size and resource-count limits, `max_bindings`
  style device limits.
- Control-flow analysis: missing returns, unreachable code,
  uniformity analysis for derivative/sample-in-nonuniform
  contexts.
- The actual `create_bind_group` *entry* lists (which texture
  view / buffer is bound at runtime) — the checker compares
  layouts, not runtime bind groups; and buffer *sizes* at
  creation beyond the pair reconstructions in §4 (e.g. a
  `min_binding_size` the layout leaves `None` is unchecked).
- Backend translation: SPIR-V/MSL/HLSL codegen failures and
  driver-specific issues (including the llvmpipe JIT segfault
  class) — those need a real compile on a real device, which
  is what the CI render paths already exercise.
- Semantic slot order inside packed assemblies (counts/sizes
  are machine-checked; order is the citation audit in §4).

## 9. CI hookup (for the lead)

One step in `.github/workflows/tools-ci.yml`, beside the other
stdlib tool steps — no new dependencies, no GPU, no network:

```yaml
      - name: WGSL shader structural check (W29)
        run: python3 tools/ci/check_shaders.py --repo .
```

Gate on the exit code as-is: `1` covers both FAIL and
UNRESOLVED, so a future refactor that changes a helper's shape
fails the job loudly instead of silently dropping coverage.
Suggested trigger: the workflow's existing push/PR paths (it
already runs on `src/**`-touching changes); runtime is seconds.
If a legitimate new helper shape appears, the right response is
to extend the interpreter in `check_shaders.py` (with its
anchor verification), not to weaken a verdict.
