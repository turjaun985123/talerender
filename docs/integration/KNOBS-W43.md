# KNOBS-W43 — quality knobs spec: `--res` / `--aa` / `--shadow` (gap-fix F4)

Seat W43, Wave 2 Batch 5. Spec only — no code written, no existing file
edited. The implementation is the lead's lane; every file:line below is
at HEAD `f6616aa` and was read first-hand.

**The law this spec is subordinate to (user, standing): no quality loss
for speed.** These knobs exist for DRAFT/PREVIEW only. The defaults are
exactly today's full quality, a run at defaults must be **byte-identical**
to today's output, and no code path in this spec may silently change a
setting in either direction — a knob the adapter cannot honour is a
hard error, never a quiet downgrade, and a draft is labelled DRAFT in
its own output so it can never be mistaken for a final.

Gap reference: gap analysis G12 — Chrome exposes `?res=0.6`,
`?aa=0|2|4|8`, `?shadow=512..4096`; TaleRender today has fixed
HalfFloat MSAA-4x and a 2048 shadow map, with `--width/--height` as the
only size lever.

---

## 1. Survey — where the three quantities live today

### 1.1 MSAA sample count (scene pass only)

The composer architecture already confines MSAA to one stage: the HDR
scene pass renders into a multisampled `Rgba16Float` target and resolves
into a single-sample texture; **everything downstream is single-sample
by design** — `composer::make_rt` hardcodes `sample_count: 1`
(src/composer.rs:83, 109), and the whole PostChain (bokeh → bloom →
output → grade) works on the resolved texture.

MSAA **targets** (colour + its depth twin — the pair must always match):

| site | how created | line |
|---|---|---|
| film | `gpucore::make_target(…, Rgba16Float, 4, …)` "hdr msaa" | src/film.rs:2545 |
| film | `gpucore::make_target(…, Depth32Float, 4, …)` "hdr depth" | src/film.rs:2551 |
| scene-file (composer path) | `gpucore::make_target(…, Rgba16Float, 4, …)` | src/scenefile.rs:1574 |
| dof-chart | `gpucore::make_target(…, Rgba16Float, 4, …)` | src/dof.rs:240 |
| ab1 | **inline** `device.create_texture`, `sample_count: 4` (colour) + a second inline block for depth | src/ab1.rs:842–867 |

`gpucore::make_target` already takes `samples` as a parameter
(src/gpucore.rs:129–147) — three of the four paths pass it a literal
`4`; ab1 is the one site that never adopted the helper for these two
targets.

MSAA **pipeline states** (must equal the target's count or wgpu
validation fails):

- Literal `count: 4`: ab1 PBR factory (src/ab1.rs:927), film PBR
  factory (src/film.rs:2606), scenefile composer-path scene pipelines
  (src/scenefile.rs:1635, 1679), dof scene pipeline (src/dof.rs:281).
- Parameterised already: the `make_basic` factories take `samples`
  (ab1.rs:966, film.rs:2643) and are called with `4` for the
  scene-writing variants and `1` for the post-grade overlay variant.
- `MultisampleState::default()` (= 1) sites — depth prepass, shadow,
  overlay pipelines (e.g. ab1.rs:996, 1062; film.rs:2670, 2713;
  scenefile.rs:1728). These match their single-sample targets
  (packed depth, shadow map, final) and **must not be plumbed to the
  AA value** — see §4.2.

**The resolve seam (the one structural catch):** both scene passes set
`resolve_target: Some(&hdr_view)` unconditionally (film.rs:3173–3178,
ab1.rs:1414–1419). With a single-sample attachment wgpu forbids a
resolve target, so `--aa 0` cannot be had by changing counts alone —
the pass must render directly into the resolved target's view with
`resolve_target: None` when samples == 1.

Paths with **no MSAA stage at all**: `--scene m1` and the scene-file
*simple* path are single-sample end to end (scenefile.rs:1110, 1418,
1429 targets at `sample_count: 1`). `--aa` is inert there by design
(§3.3) — it must not grow an MSAA stage on those paths.

### 1.2 Shadow map size

Exactly two paths build a shadow map, both at 2048², each with a
matching uniform slot that must move in lockstep:

| | target creation | `shadow_misc` uniform |
|---|---|---|
| ab1 | `create_texture` 2048×2048 Rgba8Unorm (src/ab1.rs:809–819) + `composer::make_rt(2048, 2048, Depth32Float)` (src/ab1.rs:820) | `[1.0/2048.0, 2048.0, 0.0, 0.0]` (src/ab1.rs:652) |
| film | `gpucore::make_target(2048, 2048, Rgba8Unorm, 1, …)` (src/film.rs:2528) + `composer::make_rt(2048, 2048, Depth32Float)` (src/film.rs:2531) | `[1.0/2048.0, 2048.0, 0.0, 0.0]` (src/film.rs:2838) |

The size reaches the shader **only** through that FrameU slot
(`x: shadow texel size (1/2048), y: shadow map size (2048)` —
src/shader_pbr.wgsl:51, a comment; grep confirms no WGSL file contains
a numeric 2048). PCF offsets are in texel units, so changing the slot
changes softness in world units exactly as Chrome's `?shadow` does —
no shader edit is needed, but the comment at shader_pbr.wgsl:51 should
be reworded to drop the baked-in numbers when this lands.

No shadow map exists in scenefile.rs (it parses `lights.key.shadow` /
`shadowBox`, src/scenefile.rs:927–928, but builds no shadow target in
either path), dof.rs, or main.rs's m1 — `--shadow` is inert there.

### 1.3 Resolution flow

`--width/--height` (defaults 1080×1920, src/main.rs:174–175, parsed at
main.rs:194–201) are threaded into every render entry point
(main.rs:286–310). Everything downstream derives from that one pair:
all targets (`make_target`/`make_rt` at width×height), the packed-depth
target (film.rs:2539), bloom mips (`composer::bloom_mip_sizes(w, h)`,
src/composer.rs:41–44), the PostChain run (src/composer.rs:488), the
readback (gpucore `Readback`, 256-byte row alignment computed from
width — src/gpucore.rs:175+) and the PNG size. **No post shader
hardcodes the frame size** (grep: no `textureDimensions`/resolution
constants in shader_bokeh/grade/output WGSL). One effective-size
substitution at the entry points therefore propagates everywhere —
with two exceptions, both in film's CPU overlays:

- `overlay_caption` places the strip in **design-space constants**:
  centre (540, 1340), half-height 150 (src/film.rs:3365–3386).
- `overlay_calendar` likewise: centre (352, 648), plate frame
  ±180/±220 (src/film.rs:3315–3340).

Both clamp to (w, h) but never scale — so at any non-1080×1920 size
today the overlays already sit at design-space positions while the
scene scales around them. That is the concrete reason `--res` must be
a designed path, not "just use `--width`".

Unaffected by all three knobs: the CPU painter canvas (2560×2816,
paint cache) — character cards are painted at their own resolution and
sampled as textures; a draft does not get cheaper paints, only cheaper
GPU passes. And `--film-telemetry` is CPU/design-space: its 850 px
caption audit must **not** be scaled by any knob.

---

## 2. CLI surface (exact)

New flags, parsed in main.rs beside `--width/--height`, validated
**before any GPU work** via the existing `cli_error` (src/main.rs:
131–134 — prints `error: …` + the help hint, exit 2):

```
    --res <SCALE>           Render scale for drafts: 0 < SCALE <= 1,
                            applied to --width/--height (default: 1
                            = full size). The whole frame scales,
                            captions included. Draft only.
    --aa <0|2|4|8>          MSAA samples for the HDR scene pass
                            (default: 4 = full quality; 0 =
                            single-sample). Draft only below 4.
    --shadow <SIZE>         Key shadow map size: 512 | 1024 | 2048 |
                            4096 (default: 2048 = full quality).
    --draft                 Draft preset: --res 0.5 --aa 0
                            --shadow 512. Any of the three flags
                            given after --draft overrides it.
                            Draft runs label every output line
                            DRAFT — never use for delivery.
```

Validation rules (all violations → CLI error, exit 2, no render):

- `--res`: must parse as a float with `0.0 < res <= 1.0`. `> 1` is
  rejected, not clamped — upscaling is what `--width/--height` are
  for, and Chrome's `?res` is a downscale-only knob.
- `--aa`: must be one of `0, 2, 4, 8`. (`0` is the user-facing
  spelling of single-sample, matching Chrome's `?aa=0`; internally it
  maps to sample count 1.)
- `--shadow`: must be one of `512, 1024, 2048, 4096`.
- Missing value after any of the four flags: exit 2 (the existing
  `value()` behaviour).

No environment variables are added — CLI only, deliberately (the
`TALERENDER_*` family is for cache/memory behaviour, not quality).

## 3. Interaction rules

### 3.1 Composition

- Effective size = `--width/--height` × `--res`, computed **once**,
  in one place, before dispatch:
  `ew = max(2, round(w × res))`, rounded to **even** (draft frames are
  encoded with the skill's ffmpeg settings, whose yuv420 pipeline
  needs even dimensions); same for `eh`. At `res = 1` the effective
  size is the base size exactly.
- `--aa` and `--shadow` are orthogonal to size and to each other; all
  three compose freely.
- The output PNG **is** the effective size (there is no upscale step,
  matching Chrome, whose captured draft frame is the scaled canvas).
  Delivery therefore requires `res = 1` — see the labelling rule.

### 3.2 The defaults identity law

With no knob flags (or all three given explicitly at their defaults —
`--res 1 --aa 4 --shadow 2048`), the binary must produce **byte-
identical frames to the pre-knob binary**, and print exactly the lines
it prints today. Byte-identity at defaults is the acceptance gate
(§5), not a hope: the implementation must take the identical code path
at defaults (e.g. the resolve-target selection in §4.2 must yield
`Some(...)` at samples > 1 exactly as today).

### 3.3 Per-mode applicability

| mode | `--res` | `--aa` | `--shadow` |
|---|---|---|---|
| `--film` | ✔ (overlays scale, §4.1) | ✔ | ✔ |
| `--scene ab1` | ✔ | ✔ | ✔ |
| `--scene dof-chart` | ✔ | ✔ | — (no shadow map) |
| `--scene-file` composer path | ✔ | ✔ | — (no shadow map) |
| `--scene-file` simple path / `--scene m1` / stroke-battery | ✔ | — (no MSAA stage) | — |
| `--film-telemetry`, `--telemetry` | — (CPU, design-space) | — | — |

An inert knob is accepted silently (it is not an error to pass
`--shadow 512` to dof-chart), because wrapper scripts will pass a
global knob set to every mode.

### 3.4 Draft labelling

If **any** knob is off its default (including via `--draft`), the run
is a draft and says so in-band:

- `--film`: the header line gains
  `[DRAFT res=<r> aa=<a> shadow=<s> — NOT FOR DELIVERY]`, every
  progress line keeps the `DRAFT` tag, and the RunLog completion
  summary repeats it.
- Still modes: the `wrote …` line gains the same bracketed tag.
- At defaults nothing extra is printed (§3.2).

The tag lists the effective values, so a draft frame's provenance is
recoverable from any log snippet. (Downstream naming — finish.sh, the
skill's draft encode — is the skill's side; the native side's duty is
that its own output can never pass as a final.)

## 4. Implementation sketch (lead's lane)

### 4.1 `--res`

- main.rs: parse + validate; compute `(ew, eh)` once; pass the
  effective pair into the existing entry points in place of
  `(width, height)` (signatures already take the pair —
  main.rs:286–310). Nothing else in the size flow changes (§1.3).
- film.rs overlays: `overlay_caption` (film.rs:3365) and
  `overlay_calendar` (film.rs:3315) must work in a scaled space:
  multiply their design-space coordinates by `(ew/1080, eh/1920)`
  (equivalently, pass a `ui_scale` into both). Decision, per the
  brief: **captions scale with the frame** — the skill's draft is a
  whole-frame scale (Chrome draws the caption on the scaled canvas),
  so a draft's caption geometry stays proportionally correct and the
  draft remains a faithful timing/composition preview. The caption
  *audit* stays design-space (§1.3).
- Risk: any other design-space constant in the film path (fade is
  size-free; the calendar's `2 px per unit` plate sampling at
  film.rs:3317 rides on the same transform — one scale factor must
  cover both overlays or the calendar and caption will disagree).

### 4.2 `--aa`

- Value map: `0→1, 2→2, 4→4, 8→8`. Thread one `samples: u32` from
  main.rs into `run_ab1` / `run_dof` / `run_scene_file` / `run_film`
  (film's `Renderer::new`, film.rs:2545's caller).
- Targets: pass `samples` instead of the literal `4` at film.rs:2545
  **and** film.rs:2551 (depth twin — a mismatched pair is a wgpu
  validation error), scenefile.rs:1574, dof.rs:240. ab1's two inline
  creations (ab1.rs:842–867) take the parameter (or move to
  `gpucore::make_target_viewed`, closing that leftover).
- Pipelines: the literal `count: 4` states (ab1.rs:927; film.rs:2606;
  scenefile.rs:1635, 1679; dof.rs:281) and the `make_basic` scene
  variants' `4` arguments become `samples`. The overlay variant stays
  `1`; the `MultisampleState::default()` prepass/shadow pipelines
  stay untouched (§1.1).
-
...[truncated 3075 chars]