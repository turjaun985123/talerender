# EXPORTER-W47 — the Chrome-side scene exporter: emission contract, sampling rules, format gaps, acceptance design

**Seat:** Wave 2 Batch 5 (gap-fix batch), W47. **Status: DESIGN
ONLY.** This note specifies an exporter that does not exist yet.
No code was written, no existing file was edited, and nothing in
here is a claim that any part of the exporter works. Every
format-side claim is grounded in this repo's sources of record,
cited inline; every claim about the TypeScript side that cannot
be verified from this repo is marked **TO-VERIFY** and collected
in §6.

**Sources of record (all read first-hand for this note):**
`docs/scene-format.md` (§11 f32 law, §14 v2, §14.5 rulings
R1–R5); `src/scenefile.rs` (the loader — `load()`,
`walk_nodes()`, `eval_camera_keys()`, `resolve_texture()`,
`colour_geo()`/`tex_geo()`, `compose_transform()`,
`material_colour()`); `src/film.rs` (`load_film_data()`,
`load_assets()`, the fade law); `src/toonfull.rs`
(`Mood::from_str`); `docs/integration/FORMAT15-DRAFT-W31.md`
(§15.1 guard, §15.9 rulings, §15.10 adoption order);
`docs/integration/ANIM-ADOPT-W32.md` (the staging census: what
converts, what stays code); the skill-wiring work copy
(`~/workspace/talerender-skill-wiring/docs/NATIVE-BACKEND.md` and
`template/tools/CAPTURE-NATIVE.md`, read-only) for terminology
and adapter contracts.

---

## 0. Position in the architecture

The gap analysis's §7 architecture, which this spec implements
on paper: **Chrome stays the authoring runtime** — it is the
only thing that executes `shots.ts` — and is used as an
*exporter*: walk the live staging at each shot's key times and
emit data. **TaleRender is the rasterizer.** `check` / `sheet` /
`preview` stay in Chrome until their native equivalents land;
the exporter is additive and changes nothing about the Chrome
path.

Terminology is the wiring copy's, deliberately:

- The exporter's output is **film data / a bridge**: a manifest,
  per-segment scene-format v2 files, a timing/caption data file,
  and an `assets/` tree — the exact layout the adapter
  (`capture_native.mjs`) validates before anything runs
  (NATIVE-BACKEND.md, "Film data — the bridge").
- The exporter is the general converter whose absence
  NATIVE-BACKEND.md records ("a general project→bridge converter
  is future work"; today the bridge is per-film transcription).
  This spec is that converter's contract.
- The adapter's laws bind the exporter transitively: selection
  is explicit (`RENDERER=native`), the adapter **fails closed
  and never falls back to Chrome silently**, refused modes
  (`cues`, `castsheet`, `preview`) stay Chrome-only, and a
  broken bridge must fail **up front, with the full missing
  list** — so the exporter's output must be complete enough that
  the adapter's pre-validation (manifest format/version, `fps`
  agreement, segment ranges tiling `[0, totalFrames)`,
  `totalFrames == ceil(durationS × fps)`, every named scene
  file and every loader-read asset present) passes on its first
  encounter. An exporter that emits partial bridges has failed
  its contract even if every emitted file is perfect.
- Cues are out of scope (gap-fix triage F6): cue export stays
  skill-side; the native finish muxes the project's existing
  `data/mix.wav` unchanged. The exporter emits nothing for
  `mix.py`.
- Caption *text* is never rasterised natively (standing
  contract): captions travel as **pre-rendered PNG strips**,
  composited post-grade. The exporter produces strips; it does
  not ask TaleRender to learn fonts.

**The two render shapes the exporter feeds** — stated plainly,
because they are not equivalent today:

- **Shape A — the scene-file lane (available now).**
  `talerender --scene-file <file>` evaluates one file at its
  `time`: camera keys interpolate (§14.4), but the character
  `state` in the file *is* the state at that time — the loader
  has no state animation (see G-EXP-1). A shot is therefore
  exported as one scene file per shot (camera carried by keys),
  plus per-sample files wherever character state must change
  inside the shot. All acceptance in §4 runs in this lane.
- **Shape B — the film lane (bridge data).** The manifest +
  timing/caption data + assets that the adapter's `check` and
  `video` modes consume. Today the `--film` evaluator's
  per-frame shot staging is film-specific code in `film.rs`;
  exported per-frame character motion for a *new* project has
  no generic evaluator until the §15 clips adoption lands
  (§15.10 steps 1–2) or a driver is transcribed per film (the
  Wave-1 route). The exporter emits Shape B data so the gate
  and the finish work; it does not conjure Shape B motion.
  Sequencing is §7 of this note.

---

## 1. The emission contract

### 1.1 File inventory per project

For a project of *N* shots the exporter emits:

| artefact | shape | consumer |
|---|---|---|
| `film.json` (manifest) | `talerender-film` v1: `format`, `version`, `title`, `fps`, `durationS`, `totalFrames`, `resolution {width, height}`, `segments[]` naming each segment's scene file and frame range | adapter pre-validation; `validate_film.py` |
| one `<shot>.scene.json` per shot (plus per-sample variants, §2.3) | scene format v2 (§1.2–§1.6) | `--scene-file`; telemetry |
| one film-data file, named **`film.json`** in its own directory | `anchors`, `lines[] {start, end}`, `captions[] {s, e, text, fit, strip}`, `duration` — the exact keys `load_film_data()` requires | `--film` driver data; telemetry caption audit |
| `assets/` | caption strip PNGs (one per caption chunk, named by the `strip` field), held-prop / calendar / sign plate PNGs, `plates.json` origins where plates are driver-consumed | driver `load_assets()`; scene files by relative path |
| texture files | any texture the closed generator registry (§1.3) cannot produce, rasterised to PNG beside the scene file | loader `resolve_texture()` (`file`, resolved relative to the scene file's directory) |

Naming note (gap F1, already fixed driver-side): the film
driver now prefers `film.json` and falls back to the historical
per-film name (`load_film_data()` in `src/film.rs`). The
exporter always emits the generic name. It never emits a
project-specific filename, and it never relies on the fallback.

Frame arithmetic is the manifest's own rule (from the reference
manifest, `scenes/film/film.json`): frame *n* is staged at
`t = n / fps`; a segment owns frames with
`startS <= n/fps < endS`; segment ranges tile
`[0, totalFrames)` exactly. Segment boundaries follow the
project's cut rule (the reference manifest records
`cut(i) = lines[i].start − 0.06`, with named intra-line splits);
the exporter takes boundaries from the project's own shot table
— **TO-VERIFY** that the TS side exposes them in exactly this
form (§6).

### 1.2 Scene file: header, time, camera

Per file (loader evidence: `load()` in `src/scenefile.rs`):

- `"format": "talerender-scene"`, `"version": 2`. A v1 file
  carrying v2 fields is **rejected** by the loader, so the
  exporter never emits v1-with-extras.
- `"name"`: the shot id (the loader defaults to `(unnamed)`;
  telemetry and error paths read better with real names — the
  exporter always emits it).
- `"time"`: the shot-local evaluation time in seconds (v2;
  default 0). Camera keys and the frozen character state are
  read at this `t` (§14.1).
- `"camera"`: `fovY` (required), `eye`, `target` (required —
  they are the base/fallback even when keys exist), `near` /
  `far` (emit explicitly; loader defaults 0.05 / 800), `up`
  (emit when not `[0,1,0]`), and `"keys"` per §2.2. Key shape
  is exactly `{t, eye, target, fovY?, ease?}`
  (`eval_camera_keys()`): the **destination key's** `ease`
  governs its incoming segment; `"linear"` is the only named
  value — anything else, including absence, means smootherstep.
- `"clear"`: the scene clear colour as an sRGB hex string, when
  the staging has one.

### 1.3 Nodes, materials, textures

Nodes render in **document order** (§4); the exporter emits
them back-to-front for transparent draws, as the hand-authored
scenes do. Node types the loader accepts (`walk_nodes()`):
`group`, `mesh`, `scatter`, `character`, and `code-ref` —
**the exporter never emits `code-ref`** (its two ids are
ab1-specific splices, §4.4).

- **`group`**: `{transform, children}` — the nesting the source
  builders use (§4.2).
- **`transform`**: a list of single-key ops composed **left to
  right** (§5): `translate [x,y,z]`, `rotate_x/y/z` (radians),
  `euler_xyz [x,y,z]` (one op, three.js XYZ order,
  `Rz * Ry * Rx` — the op a TS `rotation.set` maps to),
  `scale [sx,sy,sz]`. Any other op is a hard loader error, so
  the exporter decomposes everything else itself.
- **`mesh`**: exactly one of `material` / `texture` (§4.1).
  Geometry kinds are a **closed set** with exact parameter
  names (`colour_geo()` / `tex_geo()`):
  colour pipeline — `box {size}`, `cylinder {radiusTop,
  radiusBottom, height, segments}`, `sphere {radius,
  widthSegments, heightSegments}`, `cone {radius, height,
  segments}`, `disc {radius, segments}`;
  textured pipeline — `quad {width, height, y0?}`,
  `flat-quad {width, depth}`, `ground-disc {radius, segments,
  uvRepeat?}`, `sky-sphere {radius, widthSegments,
  heightSegments}`, `billboard {center, width, height}`
  (corners derived from the evaluated camera by the §7 basis
  law — the exporter emits centre/size only, never corners).
  A TS-side shape outside this set is exporter work (decompose
  into the closed set), not a format request — see G-EXP-8.
- **`material`**: `{color}` where colour is `"#RRGGBB"`,
  `{hex, shade}`, or `{hex, shadeHash {index, base, span}}`
  (§4.1; `shadeOf` / the §6 `rnd` law are reproduced by
  `material_colour()` exactly, so the exporter emits the
  *spec*, not a pre-multiplied colour, whenever the staging's
  colour was produced by the source's shade/shadeHash
  expressions — and the plain hex when it was not).
  `roughness` is **carried, not consumed** (§4.1): the exporter
  may emit it for forward fidelity but must not expect any
  visual effect. There is **no metalness field** — see
  G-EXP-7.
- **`texture`** on a mesh: an id string, or `{id, wrap}` with
  `wrap: "repeat"` the only non-default (default clamp).
- **`textures[]`**: `{id, file}` (path relative to the scene
  file) or `{id, generator, …params}`. The generator registry
  is **closed** (`resolve_texture()`): `grass {base}`,
  `planks {base}`, `wallpaper {base, motif}`, `felt {base}`,
  `parcel {label}`, `picture {}`, `sign {bg, fg, canvas}`,
  `solid {color}`, `sky-gradient {top, horizon, ground?}`,
  `cloud {}`, `blob-shadow {}`, `onoda-card {}`,
  `dof-chart-pattern {size?}`. An unknown generator is a hard
  error. Therefore: a TS-side procedural texture whose painter
  is in the registry is emitted as a generator spec with its
  exact parameters; **any other procedural texture is
  rasterised by the exporter to a file texture.** The exporter
  never invents a generator name.
- **`scatter`**: colour-pipeline only, placement kind
  `annulus` only (§4.3), with the placement constants emitted
  verbatim (`angleHash`, `radiusHash`, `radiusBase`,
  `radiusSpan`, `angleScale`, `center`) — including literals
  like the lawn's `6.283185307` that are *not* 2π (§11 applies
  to scatter constants exactly as to transforms). Other
  placement laws are G-EXP-8.

### 1.4 The light rig (R1), atmosphere (R5), fog

`"lights"` (§14.2; parsed by `load()` into `LightsCfg`):

```jsonc
"lights": {
  "key":  { "color", "intensity", "position", "target",
            "shadow", "shadowBox", "far", "bias", "normalBias" },
  "rim":  { "color", "intensity", "position" },
  "hemisphere": { "sky", "ground", "intensity" },
  "points": [ { "color", "intensity", "position", "distance", "decay" } ],
  "envIntensity": 0.25
}
```

- **R1 binds absolutely: the block carries POST-atmosphere
  values** — what the atmosphere application leaves behind,
  never the set's pre-atmosphere sunlight values (§14.5 R1,
  with the noon example as the worked case). The exporter
  therefore samples the rig *after* the TS side has applied
  the shot's atmosphere preset (**TO-VERIFY**: the TS-side
  point at which post-atmosphere values are readable, §6).
- **The fire point light carries its rendered basis:**
  intensity basis **4.6**, `distance` 7, `decay` 1.6 — never
  the constructor's 5.0 (R1). The flicker law itself is not
  format data; see G-EXP-5 for what the exporter does instead
  in the still lane.
- **R5: the file also names the atmosphere by preset** —
  `"atmosphere": "<preset name>"` (the reference scene files
  carry e.g. `"atmosphere": "noon"`). But note the loader
  evidence: `src/scenefile.rs` never reads `atmosphere` — in
  the scene-file lane the name is documentary, and resolution
  (set × preset) is the film driver's business. **Contract:
  the exporter emits the name AND the fully resolved values**
  (lights above, fog below, grade in §1.5). An exported file
  whose only atmosphere content is the name renders with
  loader defaults, which are not the production rig — that is
  an exporter bug class, named here so it can be checked for.
- `"fog"`: `{color, near, far}` (all required when present;
  colour linearised by the loader). The exporter emits the
  *resolved* fog for the shot — including shots where the
  staging replaces the preset fog mid-draw (the R4-preserved
  S06 case: scene fog `Fog(#C9CFC2, 4.5, 20)` while the
  character tint keeps the preset's fog colour; the tint half
  of that pair is G-EXP-9).

### 1.5 Look / post block

`"post"` (§9, §14.2 addendum; `load()`'s `Bokeh`/`BloomCfg`/
`GradeCfg`):

- `post.bokeh {focus, aperture, maxblur}` — all three
  required when present. **Structural law the exporter must
  respect: bloom and grade require bokeh.** The loader hard-
  errors on `post.bloom`/`post.grade` without `post.bokeh`
  (the composer is entered through bokeh in this lane). A shot
  whose look has grade but no depth-of-field still emits a
  bokeh block (the staging's own focus/aperture values; the
  engine's `aperture <= 0` disable convention is TS-side —
  **TO-VERIFY** how a disabled-DoF shot's block should read so
  the native composer reproduces it, §6).
- `post.bloom {threshold, strength, radius}` — emit all three
  (loader defaults 0.85 / 0 / 0.6 are the engine fallbacks, but
  §1.8's totality rule applies).
- `post.grade`: the canonical keys are **`sat`, `warm`,
  `exposure`, `time`** (§14.5 loader addendum), plus
  `vignette` and `grain`, both consumed by the loader's
  `GradeCfg`. **`saturation` is not a key** — a file carrying
  it is silently ignored on that key (§1's unknown-field rule),
  which is precisely the failure the exporter exists to
  prevent: it emits `sat`, always. **`time` is pinned to 0**
  (R3: no path in the source ever animates grain time; the
  film renderer pins it, and the exporter does the same).
  Per-shot look is natural in this lane — one file per shot,
  one static grade per file. Look *transitions inside* a shot
  are G-EXP-6.

### 1.6 Character specs

One `character` node per cast member present in the shot
(§14.3; consumed by `walk_nodes()`'s character arm, painted by
`src/toonfull.rs`):

- Node fields: `position [x,y,z]`, `height` (metres; the
  loader derives the card from `height / 0.96`), `perspective`
  (bool — facing follows `face()`: yaw toward the *evaluated*
  camera, plus pitch ×0.45 clamped ±0.38 when true).
- `"look"`: `body`, `head`, `shade`, `pattern`, `patCol`,
  `hat`, `hatCol`, `band`, `neck`, `neckCol`, `extra`
  (array; the loader recognises exactly `"glasses"` and
  `"mustache"`), `mustacheCol`, `kid`. This is the cast
  member's identity spec — in exporter terms it comes from the
  project's cast definition, is constant across shots, and is
  emitted identically every time (character identity does not
  drift between scene files; if two exported files disagree on
  a cast member's `look`, the exporter is broken, not creative).
- `"state"` — the state **at the file's `time`**: `mood`
  (vocabulary below), `look [x, y]` (gaze pair), `blink`,
  `talk`, `sweat`, `tears`, `armL [shoulder, elbow]`,
  `armR [shoulder, elbow]`, `legs` (`"stand" | "walk" | "run"
  | "sit" | "jump"` — any other string silently becomes
  `stand` in the loader, so the exporter validates its own
  emission), `phase`, `lean`, `bob`, `squash`, `headTilt`,
  `headTurn`, `camPitch`, `sitOn` (**metres** — the loader
  converts `× 1024 / (height/0.96)` exactly as the source's
  seat conversion; the exporter emits metres, never the
  converted value), `holdL` / `holdR`: `{plate: <texture id>,
  x0, y0}` — held props are **plates**: pre-rendered RGBA
  textures declared in `textures[]`, composited at the hand;
  prop text enters as pixels, as captions do (§14.3).
- `"attachments"`: declared **on the character node** (R2).
  The loader implements exactly one kind, `"rifle"`
  (`{kind, hand: "L"|"R", offset: [x,y,z]}` — the
  syncAttachments card-space offset; default `[0, 0.03,
  0.07]`). Any other kind is a hard error. Every other held
  object goes through `holdL`/`holdR` plates — see G-EXP-10.
- **Mood vocabulary** (`Mood::from_str` in `src/toonfull.rs`):
  `neutral, smug, happy, laugh, shock, worried, angry, sad,
  sly, proud, tired, dizzy, determined, embarrassed, awe,
  suspicious, relieved`. Any other string **silently falls
  back to `neutral`** — there is no loader error to catch an
  exporter typo. The exporter validates mood strings against
  this list before writing (§1.8).

### 1.7 Caption strips

Standing contract: strips are finished sRGB PNGs, composited
**post-grade**; their bytes are final. Two lanes, never mixed
in one comparison:

- **Film lane (delivery + telemetry).** The film-data file's
  `captions[]` entries `{s, e, text, fit, strip}` — the exact
  fields `load_film_data()` reads (`text` and `strip` default
  to `""` when absent, which the exporter treats as a defect:
  it always emits both). `load_assets()` loads
  `assets/<strip>` **in caption order**, so strip files and
  caption entries are a matched set: the exporter generates
  chunking, `fit`, and the strip raster from the same chunk
  table (the project's own caption rules — chunking semantics
  are the skill's; the exporter reproduces them from the
  project's `lines`/`words` data, it does not redefine them),
  and the telemetry audit then checks the same `text`/`fit`
  the exporter emitted. A strip whose pixels disagree with
  its `fit` is an exporter bug the audit is designed to catch.
- **Scene-file lane (acceptance stills).** A caption enters a
  scene file as a **file texture on a `billboard` node** —
  the §8 pattern the ab1 proof used (`caption.png` as a
  declared texture; §12.9: caption text is a texture file, not
  a text node). The exporter places the billboard per the §7
  basis law inputs (centre, width, height). Caveat recorded as
  G-EXP-11: in this lane the strip is drawn in document order
  inside the composer's grade, while the film lane composites
  post-grade on the CPU (`film.rs` draw order: shot, fade,
  caption) — acceptance comparisons must be lane-consistent
  (§4).

### 1.8 The totality rule (exporter self-validation)

The loader is permissive in ways that make exporter omissions
invisible: unknown fields are ignored (§1), loader defaults
exist for nearly every lights/grade/look/state field and are
**not** the production values (e.g. the loader's key-light
fallback is `#FFE2C2 × 3.2` at `[2.5, 4, 3]`; the character
`look` fallback body is `#E8734A`; state arm fallback is
`[8, 6]`), mood and legs strings fall back silently, and
`roughness`/`atmosphere` are carried without effect in this
lane. Therefore:

> **The exporter emits every consumed field explicitly, and
> validates its own output before writing it**: mood/legs
> strings against their vocabularies, colour specs against
> the three forms, every texture id referenced actually
> declared, every `strip` file present, segment ranges
> tiling exactly. The existing repo validators are the
> check pattern (`check_lights.py` for the rig + grade-key
> discipline, `check_assets.py` for reference completeness);
> exporter output must pass both classes of check with zero
> warnings-as-errors before the bridge is declared. "The
  loader accepted it" is not evidence of correctness in this
  format — most exporter mistakes load fine and render wrong.

---

## 2. Sampling rules

### 2.1 The time grid

All sampling happens on the film's frame grid: frame *n* at
`t = n / fps` (§1.1). The exporter never samples at times the
renderer cannot be asked to reproduce, and scene-file `time`
values are shot-local seconds on that grid. Still-lane samples
(the acceptance lane) are named frame times, not wall-clock
approximations.

### 2.2 Camera: key-preserved vs densely sampled

The loader's camera evaluator knows exactly two segment
behaviours (`eval_camera_keys()`): **smootherstep** (the
default; the engine's camK easing) and **linear** (destination
key carries `"ease": "linear"`). That fact decides the rule:

- **Key-preserved** when every segment of the rig's motion, as
  authored, eases by smootherstep or linearly **and** the
  rig's state is fully captured by `eye / target / fovY` at
  its keys. The exporter emits the authored keys verbatim
  (exact f32 values, §11), with `ease` present only where the
  authored segment is linear. Position/look-at rigs qualify:
  a look-at-subject rig is key-preserved by evaluating its
  `target` at the keys — the format carries evaluated points,
  not constraints. A dolly-zoom qualifies (fovY is a key
  channel; the coupling is evaluated into the keys).
- **Densely sampled** otherwise: rigs whose easing vocabulary
  exceeds the two behaviours (the evaluator's wider easing set
  in `src/anim.rs` — ease-out cubic, ease-in-out, ease-out
  back / overshoot, springs, settle/anticipate behaviours —
  lives in the §15 clips evaluator, **not** in the camera-key
  evaluator; §15.9 confirms the camera law stays §14.4's).
  The exporter evaluates the rig at **every frame time** in
  the shot and emits one key per frame, each arrival marked
  `"ease": "linear"`. Between adjacent frame samples the
  renderer then interpolates linearly, so the exporter's
  error is confined to sub-frame curvature of the true rig —
  and the §4 PSNR gate, not this argument, is the arbiter of
  whether a given rig survives sampling. Dense sampling is
  the fallback, never the default: it multiplies key count by
  the shot's frame count and discards the authored structure
  that later editing needs.
- Outside the key range the end keys hold (loader law); the
  exporter emits keys spanning the shot's full `[start, end]`
  so no hold is ever implicit.

### 2.3 Character state: per-change in the still lane, laws in the film lane

The scene-file lane has exactly one state per file, so the
exporter's unit of character emission is the **state sample**:
a scene file whose `state` blocks are the evaluated states at
its `time`.

- **Discrete changes** — mood steps, arm pose changes, hold
  swaps, legs-class changes, `sitOn` changes, visibility
  swaps: emit a new sample at every change point. Between
  change points the state is constant *by construction of the
  sample set*, which is what makes a still-lane render exact
  at its sampled times.
- **Continuous channels** — `phase`, `bob`, `blink`, `talk`,
  `lean` drift, gaze drift: these are law-driven on the TS
  side (gait derivations, the blink director, talk waveforms;
  §3). In the still lane the exporter emits their **evaluated
  values at the sample time** — the file records results, not
  laws. Sampling density for a *moving* shot in this lane is
  therefore per-frame if the deliverable is per-frame; the
  exporter does not pretend a per-beat sample set reproduces
  a walk cycle.
- **Exactness**: evaluated continuous values are emitted as
  exact f32 (§2.5). The paint cache keys on exact state
  values — an exporter that rounds a blink or phase value to
  two decimals is not "simplifying", it is changing which
  paints occur (the determinism root-cause lesson of the
  quantised `state_key`: keys are exact f32 bit patterns).
- In the film lane, continuous state belongs to evaluator
  laws (§15 clips, when adopted) or driver code — the
  exporter emits the bridge data and does not bake per-frame
  states into it. What it must not do is emit a *static*
  state and let it pass for motion (G-EXP-1).

### 2.4 Sets and lights sampling

- **Sets** are static within a shot by construction of the
  shot model: one node tree per shot. Structural changes the
  staging performs mid-shot (visibility windows, instance
  swaps — 20 records in the W32 census) are change points
  under §2.3's rule, producing per-sample trees. There is no
  `visible` field in the loader — a node exists in a file or
  it does not — so the exporter never emits a superset tree
  with flags.
- **Lights** are static per shot except the fire (G-EXP-5).
  Any other rig animation has no key mechanism in the format
  (there are no light keys): the shot is split into per-sample
  files, or the animation is declared out of the exporter's
  current scope for that shot and the shot stays on Chrome.
  Declaring is a valid exporter outcome; silently freezing a
  moving rig is not.

### 2.5 The f32 data law in emission (§11)

Every number the TS side computed in f32 is written as the
exact f32 value (shortest-round-trip decimal). This is the
measured difference between 71.9 dB with silhouette-edge
flips and 95.2 dB clean (§11), and it applies to transforms,
camera keys, scatter constants, light values, and evaluated
character state alike. Values computed in f64 on the TS side
(the scatter/shadeHash laws are f64 expressions, §4.1/§6) are
emitted as the format specifies them — the *laws' inputs*,
not pre-evaluated results — wherever the format carries the
law (shadeHash, annulus placement); everywhere else the
exporter emits the f32 the engine's own arithmetic produces.
Where the exporter cannot tell which precision a TS value was
computed in, that is a §6 TO-VERIFY item, not a guess.

---

## 3. Format gaps the exporter would hit

Each gap: what the exporter needs, the loader evidence that
it is absent, and the exporter's prescribed behaviour until
the format grows. "Behaviour" never includes inventing fields
— §1 guarantees invented fields are ignored, i.e. silently
wrong.

**G-EXP-1 — No character animation over time.**
The loader evaluates one `state` at `time`; there are no
state keys, and the §15 clips that would carry them are not
implemented in the loader (nor is the §15.1 reservation
guard: measured state in FORMAT15-DRAFT §15.1 — `prefabs` /
`clips` blocks are silently ignored today). *Behaviour:*
§2.3's state samples in the still lane; film-lane motion
waits for §15.10 steps 1–2. The exporter never emits `clips`
into production data before then (§15.10 step 3's rule).

**G-EXP-2 — Gait is a law, not data.**
`legs` + `phase` + `bob` are values; the derivation of phase
and bob from travelled distance (the walk-phase / walk-bob
functions) is code on both sides (W32 census: 23 gait records
code-only by design; LOCOMOTION-W24 records the laws and the
facing-sign discipline — in-card cycle direction depends on
travel direction). *Behaviour:* the exporter emits evaluated
`phase`/`bob` per sample (§2.3) and takes the facing/axis
conventions from the staging's own evaluation, never from a
re-derivation of its own — two implementations of the gait
law will disagree in the last bit and in sign conventions,
and R4 (source wins) settles which one is right.

**G-EXP-3 — Blink is a director, not a track.**
`blink` is a value in `state`; the schedule (seeded,
per-character) is a procedural system (W32 census: 28 blink
records code-only). *Behaviour:* evaluated `blink` per
sample. (The smooth-vs-quantized paint-cost question,
BLINK-DECISION-W33, is a film-lane adoption question; in the
still lane the exporter emits the true evaluated value,
which is the smooth one by construction.)

**G-EXP-4 — Talk is a waveform, not a track.**
`talk` is a value; its waveform is code (W32 census). A
lip-sync *track* exists as data work (LIPSYNC-W23: viseme
keys in W15 clip shape, validated) but clips are not
loader-implemented (G-EXP-1). *Behaviour:* evaluated `talk`
per sample in the still lane; the viseme track is emitted
only as §15 data once adoption lands.

**G-EXP-5 — Fire flicker is an evaluator law (R1's other
half).**
The format carries the fire's *basis* (4.6 / 7 / 1.6); the
rendered intensity `(4.6 + 0.7·sin(17t) + 0.5·sin(7.3t)) ·
level` is computed by the evaluator (R1; W32 census: 11 fire
records code-only, "baking them would break the R1
cross-check"). *Behaviour, by lane:* film-lane data carries
the basis, untouched. **Still-lane files carry the evaluated
intensity at the file's `time`** — a still has no evaluator
to flicker for it — and the exporter marks such files as
evaluated samples (the `time` field already does this; a
still whose fire intensity equals the basis at a `t` where
the law disagrees is a findable exporter bug, and §4's
protocol looks for exactly it).

**G-EXP-6 — No look/grade channels over time.**
`post.grade` / `post.bloom` are static per file; there are
no post keys, and §15 clips define no grade track the
loader consumes (and `post.grade.time` tracks are rejected
by name in the clips vocabulary, §15.7/R3). *Behaviour:*
per-shot static look (§1.5); a shot with an authored
in-shot grade transition is split into per-sample files at
the transition's key times (grade evaluated per sample) or
stays on Chrome. The exporter reports which choice it made
per shot — a transition flattened without a report is a
silent quality loss of exactly the kind the user has
forbidden.

**G-EXP-7 — Materials stop at colour.**
`material_colour()` reads `color` and type-checks
`roughness`; nothing else in a material object is read. A
staging material with metalness (the source's material
subset includes it — the M5 audit docs discuss the `mat()`
subset) has **no field**: emitted, it would be silently
ignored; omitted, the render differs from Chrome wherever
metalness matters (the Wave-1 transcription audit's two
misses were exactly a sea material's metalness absent from
staging data). *Behaviour:* the exporter emits colour +
roughness, and **reports every material whose source spec
carries terms the format cannot hold** (per shot, by name)
so the loss is a known list, not a surprise in A/B.

**G-EXP-8 — Geometry and scatter vocabularies are closed.**
Colour geometry: 5 kinds. Textured: 5 kinds. Scatter: one
placement law, colour-only (§1.3). *Behaviour:* decompose
out-of-vocabulary shapes into the closed sets exporter-side
(explicit meshes/groups); bake non-annulus scatter into
explicit instances. Decomposition changes draw structure,
not pixels — where a decomposition *would* change pixels
(e.g. a shape whose normals the closed kinds cannot
reproduce), the shot is reported and stays on Chrome.

**G-EXP-9 — The character fog tint has no field.**
The film evaluator tints character paint by a fog mix whose
colour can deliberately differ from the scene fog's (R4's
preserved S06 mismatch: tint keeps the preset fog colour,
`fogMix` 0.35, while the scene fog is replaced). The
scene-file loader has no per-character tint input — the
character node is painted from `look`/`state` only.
*Behaviour:* in the still lane the exporter cannot express
the tint; shots whose staging uses a divergent tint are
named in the exporter's per-shot report, and their A/B
expectation is set accordingly (§4's protocol treats a
uniform character-only colour delta on such a shot as a
known-gap signature, not a pass).

**G-EXP-10 — Attachments: exactly one kind.**
`attachments` implements `"rifle"` and hard-errors on
anything else (§1.6). *Behaviour:* rifle → attachment;
everything else held → `holdL`/`holdR` plates. A staging
whose held prop needs true 3D behaviour beyond a plate
(angle-dependent geometry the plate cannot fake) is
reported per shot; it is not approximated silently.

**G-EXP-11 — Captions in the scene-file lane are graded.**
Film lane: strips composite post-grade on the CPU, bytes
final. Scene-file lane: the strip is a scene texture, drawn
in document order and graded with the frame (§1.7).
*Behaviour:* acceptance comparisons are lane-consistent
(§4); delivery captions always travel the film lane. The
exporter never "fixes" the difference by pre-grading a
strip — strip bytes are the Chrome caption system's output,
verbatim.

**G-EXP-12 — No light keys, no node visibility, no state
keys: time exists only as `time` + camera keys.**
The format's entire temporal vocabulary in the loader is
the top-level `time`, camera `keys`, and (film-lane) the
driver's data. Every other time-varying quantity is a
sample-or-law decision (§2). *Behaviour:* the exporter
owns a per-shot **sampling manifest** (which channels were
key-preserved, which densely sampled, which evaluated per
sample, which reported as unexportable) — this manifest is
part of the bridge, reviewed like the data, and is what
makes an export auditable instead of anecdotal.

---

## 4. Acceptance design

One real shot, end to end, before any second shot is
exported. The shot is chosen against three criteria: it has
a character with a non-trivial state (mood + arms + a hold),
a camera move (so §2.2 is exercised), and a look with grade
(so §1.5 is exercised) — the §28 anchor shot's profile. A
static, characterless shot proves nothing about this
contract.

**Procedure.**

1. **Chrome reference.** The project's own capture renders
   the shot's frames at the chosen sample times (the skill's
   `frames` mode). Per the wiring copy's honesty rule, a
   native-vs-Chrome comparison carries the known
   one-directional bias: the reference's JPEG generation
   counts as native "error" (decision (B): native frames
   stay clean). Numbers are reported with that bias named,
   never silently absorbed.
2. **Export.** The exporter emits the shot's scene file(s)
   (state samples at the reference times), its bridge slice
   (manifest segment + film-data lines/captions for the
   shot), its strips and plates, and its sampling manifest
   (§3, G-EXP-12).
3. **Data gates (before any render).** The emitted rig and
   files pass the repo's validator classes: light-rig checks
   (grade keys canonical, `grade.time == 0`, atmosphere
   named + resolved values present, fire basis correct in
   film-lane data / evaluated in still files), asset checks
   (every referenced file present, zero missing), and the
   exporter's own totality validation (§1.8). Any failure
   stops here — rendering bad data to see how it looks is
   not a step.
4. **Native render.** `talerender --scene-file` at each
   sample time, on the same adapter class as the reference
   comparison claims (CI lavapipe proves correctness; a
   RADV claim requires the user's PC — no cross-adapter
   numbers are mixed).
5. **Pixel gate.** Per sample pair, PSNR via the repo's A/B
   tooling, reported as the repo reports its pairs (mean /
   min across samples, % pixels > 10, max Δ). **PASS bar:
   ≥ 45 dB per sample with ≤ 0.01 % of pixels > 10** — the
   bar the format's own transcription proofs are measured
   against (the as-data proofs scored 95.17 / 84.49 dB
   against hand-authored native references; against a Chrome
   JPEG reference the §28 anchor scored 20.15 dB for
   staging-transcription reasons that predate the format, so
   the bar is set for *exporter* fidelity — same staging,
   both engines — and a first-export result between 30 and
   45 dB is a FAIL that triggers §5, not a negotiation).
   Known-gap signatures (G-EXP-9 tint shots) are declared
   *before* the render, with their expected delta described;
   a declared signature caps the verdict at
   PASS-WITH-KNOWN-GAP for that sample, never PASS.
6. **Telemetry gate.** For the film-lane slice: the film
   telemetry verdict on the exported data must be
   `FILM TELEMETRY: CHECK OK` (exit 0 under the current
   exit-code contract), with the caption audit reading the
   exporter's own `text`/`fit` — i.e. the exported chunks
   fit the floor the audit enforces. For the still lane:
   the character cutouts from the native render (boxes,
   overflow) are compared against the Chrome side's cutout
   data for the same times — the M6 discipline (native
   `cutouts()` matched Chrome exactly) applied to exported
   data.
7. **Determinism spot-check.** Each exported scene file is
   rendered twice; the pair must be byte-identical. An
   exporter whose output renders non-deterministically has
   emitted a value the format evaluates unstably, and that
   is an exporter/format finding, filed before any pixel
   discussion.

**What PASS buys:** the exporter is trusted for *this shot's
feature set*. Generalisation is incremental and explicit —
the sampling manifest accumulates the feature classes proven
(camera behaviours, state channels, texture routes, material
terms), and a shot using an unproven class re-enters this
procedure for that class. The full-short proof (the adapter's
`check` + `video` over an exported bridge) is a separate,
later gate — it is the bridge packager's layout the exporter
must remain compatible with, and it does not substitute for
this one.

---

## 5. Failure protocol

Named, in order, no improvisation:

1. **Data-gate failure** → the defect is in emission. Fix
   the exporter rule that produced it (never hand-edit the
   emitted JSON — hand-edited output proves nothing about
   the exporter), re-emit, re-run §4 step 3.
2. **PSNR failure** → bisect by layer, in the order the
   pipeline composes: (a) re-render with the direct path
   (no post) — if the delta vanishes, the defect is in the
   look/post emission (§1.5); (b) lights-only comparison
   (grade/bloom neutralised on both sides) — isolates
   §1.4 (the first suspect is always an R1 violation: a
   pre-atmosphere value that slipped through); (c)
   character-only comparison via cutout boxes and the card
   region — isolates §1.6 (mood/legs silent-fallback
   typos live here, which is why §1.8 validation exists);
   (d) set geometry/textures — isolates §1.3 (decomposition
   and generator-vs-file routing). Each bisection result is
   recorded with its numbers.
3. **A bisection that lands on a format gap** → the gap is
   added to §3's register with the bisection evidence, the
   shot (or that feature class) is declared Chrome-stays
   for now, and the exporter's sampling manifest records
   the declaration. **The renderer is never tuned to match,
   and values are never averaged** (R4: source wins; where
   the exporter and the source disagree, the exporter is
   wrong by definition).
4. **Telemetry failure on exported data** → treated as a
   data defect (step 1), with one exception: if the audit
   itself disagrees with the Chrome-side audit on identical
   staging, that is a telemetry finding for the lead, filed
   with both outputs verbatim — the exporter does not adjust
   data to please one audit against the other.
5. **Any failure whose cause cannot be isolated** → the
   shot's export is abandoned for this round, the evidence
   (pair table, bisection notes, sampling manifest) is
   filed in an A/B package in the repo's `docs/ab/` style,
   and the shot stays on Chrome. An unexplained partial
   export does not ship, does not get a waiver by assertion,
   and does not block other shots whose feature classes are
   unaffected.

---

## 6. TO-VERIFY register (TS-side, unverifiable from this repo)

The exporter runs inside the skill's runtime; this repo can
cite the TS concepts its own docs name (the light rig and
sunlight builders, the atmosphere application and its
presets, the fire-level law, camera keys / camK, pose
tracks, `syncAttachments`) but cannot verify TS APIs. The
following must be verified against the skill source before
any exporter code is written; each is phrased as the
question the implementation must answer:

1. Can the staging be **evaluated at an arbitrary `t`**
   outside the draw path (the gap analysis records
   `OP.draw(t)` as a pure function of `t` — the exporter
   needs the *evaluated scene state*, not a drawn frame:
   node transforms, rig values, character state, at time
   `t`, in data form)?
2. At what point are **post-atmosphere** rig values readable
   (R1) — is there a post-application state to sample, or
   must the exporter re-apply the preset table itself (in
   which case the preset table becomes exporter input, and
   its f32/f64 precision per value must be pinned, §2.5)?
3. How does a **disabled-DoF** shot represent itself (the
   `aperture <= 0` convention), and what bokeh block makes
   the native composer reproduce it (§1.5)?
4. In what form does the TS side expose **shot boundaries**
   (the cut rule and intra-line splits the manifest records)
   and the **caption chunk table** (chunk `s/e/text/fit`
   before any strip is rasterised)?
5. Which TS values are computed in **f32 vs f64** (§2.5) —
   in particular evaluated character state and camera keys.
6. What is the TS-side **cast definition's** field set, and
   does it map onto §1.6's `look` fields one-to-one (the
   repo side can only show the consumer's fields and their
   defaults)?
7. Does the staging evaluate **gait/blink/talk/fire** at
   arbitrary `t` through the same code the draw path uses
   (so §2.3's evaluated samples are the draw's own values),
   or are some of them frame-sequential (accumulated state),
   in which case sampling must run the sequence, not sample
   the function?

No exporter implementation begins on an assumed answer to
any of these. An answer that contradicts this spec amends
the spec first (dated amendment, same discipline as the
format's own addenda).

---

## 7. Sequencing and non-goals

**Sequencing.** (1) §6 verification pass against the skill
source. (2) Exporter v0: Shape A for one shot — §1's
contract, §2's sampling, §4's acceptance. (3) Feature-class
generalisation per §4's closing rule, sampling manifest
growing. (4) Shape B bridge emission for a full short,
packaged through the existing bridge layout, gated by the
adapter's `check`. (5) Film-lane *motion* for exported
projects when §15 adoption (§15.10 steps 1–2) gives clips a
loader, or by explicit per-film driver decision — never by
the exporter quietly baking 1,811 state files and calling
it a film format.

**Non-goals.** No native text rasterisation (strips are the
contract). No cues export (F6; skill-side). No `sheet` /
`preview` / `castsheet` implications — those modes'
Chrome/native split is the adapter's existing contract and
the exporter does not move it. No draft-quality knobs in
data (resolution/AA/shadow-map size are renderer flags —
gap F4's lane — and the exporter emits one master-quality
bridge; a draft is a render decision, not an export
variant). No changes to the Chrome path: the exporter reads
staging; it does not refactor it.

**The one-line summary for the lead:** the exporter is a
*total, self-validating transcription* of evaluated staging
into format v2 + bridge data — keys where the format can
carry keys (camera), evaluated samples where it can only
carry values (character state, fire), laws left in code
where the format has no laws (gait, blink, talk, flicker) —
with every unexportable term reported by name, and one real
shot's A/B + telemetry verdict as its licence to generalise.
