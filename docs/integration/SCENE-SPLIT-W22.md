# SCENE-SPLIT-W22 — `src/scene.rs` modularisation blueprint

Seat W22, TaleRender Wave 2, Phase 2 (architecture). **Blueprint only —
no code was changed by this seat.** The lead executes from this
document, one module per commit, in the order in §5.

All line numbers are against `src/scene.rs` at the wave-2 audit head
(2,979 lines). Verify with `grep -n` before cutting; if the file has
moved since, re-derive the ranges from the item names, which are
stable.

---

## 1. What `scene.rs` actually is today

The module header (lines 1–19) says it plainly: this file is the
crate's shared-parts bin. It was created at M2 by extracting, verbatim,
everything `ab1.rs` and the new scene-format loader both needed, and
then it kept absorbing: the M4 character subset, the DoF chart texture
(moved out of `dof.rs`), a hand-rolled JSON parser, and — during the
production-integration wave — six registry texture painters pasted in
from the `painters/` staging pack, each with its test module attached.

Three facts shape the whole split:

1. **It has exactly one import:** `use crate::math::Mat4;` (line 20),
   and `Mat4` is used in exactly two places — the `model` fields of
   `ColourDraw`/`TexDraw` (lines 1461, 1466). Nothing else in the file
   depends on any other module. The file is internally layered but
   externally almost a leaf, which makes it the easiest of the big
   files to split safely.
2. **There is no global state.** No `static`, no `Mutex`, no
   `thread_local`, no lazy initialisation anywhere in the 2,979 lines.
   The only module-level constants are the picture/sign painter
   constants (lines 2664–2821). `rnd` is a pure sin-hash. Every
   extraction below is a pure code move — no state has to be
   re-homed, and no initialisation order exists to break.
3. **Its public surface is wide but flat:** ~46 `pub(crate)` functions,
   10 `pub(crate)` types, 11 `pub(crate)` constants, consumed by six
   modules (§3). The split must preserve every `crate::scene::…` path
   via re-exports so that **zero importer files change**.

---

## 2. Region map

Clusters are named by what the code *does*, with the file's own
section banners noted where they exist.

### Cluster A — colour helpers · lines 22–95 (~74 lines)

| Item | Lines |
|---|---|
| `hex_rgb` | 30 |
| `srgb_to_linear` / `linear_to_srgb` (f32 pair) | 35 / 43 |
| `hex_lin` | 52 |
| `shade_rgb` | 68 |
| `shade_lin` | 80 |
| `rnd` (deterministic sin-hash) | 92 |

File banner: "Colour helpers." Everything downstream that touches a
hex string or a shade factor passes through here — except the two
deliberate exceptions flagged in §6 (planks, felt).

### Cluster B — geometry builders · lines 97–462 (~366 lines)

| Item | Lines |
|---|---|
| `Geo` struct + impl (`new/vert/tri/fix_winding/append`) | 104–160 |
| `box_geo` | 162 |
| `cyl_ring` / `cyl_geo` | 185 / 200 |
| `sphere_geo` / `cone_geo` / `disc_geo` | 222 / 251 / 277 |
| `MapGeo` + `sphere_map_geo` (UV-mapped variant) | 298–362 |
| `TexGeo` struct + impl | 369–388 |
| `quad_geo` / `flat_quad_geo` / `ground_disc_geo` / `sky_sphere_geo` | 390 / 403 / 417 / 437 |

Two vertex formats live here and must not be conflated: `Geo`
(pos+normal+colour, the colour pipeline) and `TexGeo`/`MapGeo`
(pos+uv, the textured pipeline). The draw records that carry them are
in Cluster F.

### Cluster C — CPU rasteriser (the `Painter`) · lines 463–972 (~510 lines)

| Item | Lines |
|---|---|
| `Painter` struct | 475 |
| `Rgba` type alias | 481 |
| `rgba_of` | 483 |
| `impl Painter` (`new/blend/bbox/blend_cov/cov_of/fill_ellipse/stroke_ellipse/stroke_polyline/stroke_quad/fill_polygon/polygon_mask/ellipse_mask/fill_ellipse_masked/fill_rect`, 15 methods) | 488–802 |
| SDF helpers: `ellipse_sd` 803 · `seg_dist` 816 · `seg_dist_t` 822 | 803–842 |
| `CapFan` + `cap_fan` + `fan_sd` (Skia round-cap model) | 843–896 |
| `cov_area` / `cov_area0` | 897–917 |
| `point_in_polygon` | 918 |
| `flatten_quad` / `flatten_cubic` | 934 / 947 |

This is the compatibility contract §2 surface — the signed-distance
coverage model the M4 stroke battery verifies against Skia. It is the
most identity-critical code in the file (see §6, "stay monolithic").

### Cluster D — character paint, M4 subset · lines 974–1322 (~349 lines)

| Item | Lines |
|---|---|
| `ToonMood` (5 moods: Neutral, Determined, Angry, Laugh, Shock) | 974 |
| `ToonSpec` (10 fields, `&'static str` look parts) | 982–997 |
| `rounded_rect_pts` | 999 |
| `paint_toon` | 1016–1322 |

`paint_toon` paints onto a 2560×2816 canvas (`1280*2 × 1408*2`,
line ~1019) — this is the ~29 MB painted canvas the audit's memory
finding refers to. Keep that fact attached to this item when the
paint-budget work (W06) reads this blueprint.

### Cluster E — environment texture painters · lines 1323–1458 (~136 lines)

`paint_grass` 1323 · `paint_sky` 1343 · `paint_cloud` 1385 ·
`paint_blob` 1436. The original world.ts texture recipes, hand-written
for this module (not part of the registry pack).

### Cluster F — draw records, vec helpers, chart texture · lines 1459–1530 (~72 lines)

| Item | Lines |
|---|---|
| `ColourDraw { geo: Geo, model: Mat4 }` | 1459 |
| `TexDraw { geo: TexGeo, model: Mat4, tex: usize, repeat: bool }` | 1464 |
| `sub3` / `cross3` / `norm3` | 1470 / 1473 / 1480 |
| `chart_pattern` / `chart_texture_rgba` (DoF chart, f64-exact) | 1496 / 1515 |

### Cluster G — JSON parser · lines 1531–~1855 (~325 lines)

`Value` enum 1539 · `impl Value` 1548–1574 · `parse_json` 1575 ·
`JsonParser` 1586–1764 · `#[cfg(test)] mod json_tests` 1765–~1855.
Self-contained: no dependency on any other cluster, and no other
cluster depends on it. The crate's zero-dependency stance (wgpu /
pollster / image only) is why this exists; the audit's suggestion to
swap in serde was already considered and rejected on that ground —
this blueprint keeps the parser, it just gives it its own file.

### Clusters H–M — the six registry painters · lines 1795–2979 (~1,185 lines)

Each arrived from the wave-1 `painters/` pack with private helpers
and its own test module fused to it. **Code and tests move together
or not at all.**

| Painter | Banner | Code | Tests |
|---|---|---|---|
| H `planks` | 1795 | helpers 1859–1922, `paint_planks` 1923 | `planks_tests` 1960–2122 |
| I `wallpaper` | 2123 | `shade_hex` 2178, `paint_wallpaper` 2193 | `wallpaper_tests` 2245–2334 |
| J `felt` | 2335 | f64 colour trio 2368–2408, `paint_felt` 2409 | `felt_tests` 2428–2501 |
| K `parcel` | 2502 | `paint_parcel` 2532 | `parcel_tests` 2562–2628 |
| L `picture` | 2629 | consts 2664–2675, `paint_picture` 2676 | `picture_tests` 2694–2752 |
| M `sign` | 2753 | consts 2804–2821, `sign_canvas_size` 2822, `sign_text_anchor` 2831, `sign_edge_rgba` 2842, `paint_sign_px` 2854, `paint_sign` 2873, `sign_with_text` 2883 | `sign_tests` 2903–2979 |

There is **no camera code in `scene.rs`** — cameras are defined per
consumer (`scenefile.rs` has its own `Camera`, `film.rs` and `ab1.rs`
their own rigs). Anyone expecting a camera cluster here will not find
one; that absence is itself an architecture note for Phase 2, but it
is not this file's problem to fix.

---

## 3. Who consumes what — the seams that already exist

### 3.1 Importers of `crate::scene` (grep-verified)

| Consumer | What it takes | Clusters touched |
|---|---|---|
| `scenefile.rs` | `use` block of 20 names (line 26) + qualified `scene::paint_*` dispatch in `resolve_texture` (~420–461), `scene::parse_json` (826), `scene::rnd` (263, 390) | A, B, E, F, G, H–M |
| `ab1.rs` | `use` block of 24 names (line 27) + `paint_toon`/`ToonSpec`/`ToonMood` qualified (44–49) | A, B, E, F, D |
| `film.rs` | `use` block of 21 names (line 33) + qualified `parse_json`, `hex_rgb`, `shade_rgb`, geo builders | A, B, E, F, G, D (`rounded_rect_pts` only) |
| `toonfull.rs` | `use` block of 7 names (line 26): `ellipse_sd, flatten_cubic, flatten_quad, hex_rgb, shade_rgb, Painter, Rgba` + `rounded_rect_pts` qualified (716) | A, C, D (one fn) |
| `battery.rs` | 7 names (line 15): `paint_toon, parse_json, rgba_of, Painter, ToonMood, ToonSpec, Value` | C, D, G |
| `telemetry.rs` | Qualified only: `parse_json`, `Value`, `ToonSpec`, `paint_toon` (1219–1323) | D, G |
| `dof.rs` | Exactly one call: `chart_texture_rgba` (207) | F |

`anim.rs`, `charanim.rs`, `atlas.rs`, `composer.rs`, `rendergraph.rs`,
`gpucore.rs` do **not** import `scene` at all. The blast radius of
this split is the seven files above, and with re-exports preserved it
is zero files.

### 3.2 `scene.rs` ↔ `toonfull.rs` — the character-paint seam

This is the relationship the audit's "duplication" finding points at,
so it gets an honest statement:

- **`scene::paint_toon` (Cluster D) is the M4 subset**: 5 moods, the
  look/state surface the §28 scene and the stroke battery exercise.
  Its consumers are `ab1.rs`, `battery.rs`, `telemetry.rs` — the
  *verification* side of the project.
- **`toonfull::paint_toon_full` (876 lines) is the production
  painter**: all 17 moods, walk/run/sit/jump legs, hats, neckwear,
  extras, held-prop plates. Its consumer is `film.rs` — the
  *production* side.
- The seam between them is real and narrow: `toonfull` builds **on**
  Cluster C (`Painter`, `Rgba`, `ellipse_sd`, `flatten_quad`,
  `flatten_cubic`) and Cluster A (`hex_rgb`, `shade_rgb`), reuses
  exactly one Cluster D helper (`rounded_rect_pts`), and otherwise
  re-implements the figure drawing at production scope. It defines
  its own `Mood` (17 variants) parallel to `ToonMood` (5), and its
  own private `rgba`/`shade_rgba` wrappers (toonfull.rs 188–199)
  over the same Cluster A functions.
- **What this split does and does not do:** it files Cluster D as
  `scene/toon.rs` next to the rasteriser it uses, and it names the
  `toonfull` relationship in that module's header. It does **not**
  merge the two painters. Unifying `ToonMood`/`Mood` or the two
  figure bodies is a fidelity-risk change against the battery 46/46
  and M6 exact-agreement evidence, not a code move, and it is
  explicitly out of scope here. Whoever wants that merger writes a
  separate proposal with a battery-first proof plan.

### 3.3 `scene.rs` ↔ `painters/*.rs` — duplication that already happened

The `painters/` directory at repo root (7 `.rs` files, ~1,970 lines)
is **not compiled**: no `mod`, no `include!`, no build reference
(grep-verified). It is the wave-1 staging pack. Six of its seven
painters were pasted into `scene.rs` (the `// ===== registry painter:
… (wave pack, painters/…)` banners at 1795/2123/2335/2502/2629/2753
mark each paste site); the seventh, `calendar.rs`, was never a
texture painter and is not in `scene.rs` at all.

Consequence, stated plainly: **the registry painters currently exist
twice** — the staging originals in `painters/` and the live copies in
`scene.rs`. The staging copies are already stale by construction
(they carry `use crate::scene::…` lines the paste deleted, per
`painters/INDEX.md`). The split in §4 gives the live copies a real
home under `src/scene/painters/`; §6 flags the staging directory as
a decision the lead must close in the same series, or the duplication
the audit complained about survives the refactor wearing a new hat.

### 3.4 Which parts of `scene.rs` duplicate painter concerns?

- Clusters H–M *are* painter concerns that happen to live in the
  graph-layer file — they use `Painter` and colour helpers exactly
  the way `toonfull` does from outside. They belong with the painters,
  not the graph.
- Cluster E is the same kind of code (texture recipes) but predates
  the registry and is dispatched by name from both `ab1.rs` and
  `scenefile.rs`; it moves with the painters in spirit (§4 puts it in
  `textures.rs`, one file for the four originals + the chart).
- Clusters A, B, C, F, G genuinely belong to the shared/graph layer:
  colour conversion is a contract with three.js semantics, not a
  painter concern; the geometry vocabulary *is* the graph layer's
  data; the JSON parser serves the format, not painting.

---

## 4. Proposed target layout

A directory module, because flat `src/scene_*.rs` files would break
the `crate::scene::` prefix every importer uses. `src/scene/mod.rs`
re-exports everything, so `crate::scene::paint_toon`,
`crate::scene::Geo`, etc. resolve exactly as today.

```
src/scene/
├── mod.rs        re-exports only (~90 lines) + the module doc
├── colour.rs     Cluster A            (~90 lines)
├── geo.rs        Cluster B + F draw records + vec helpers (~440)
├── raster.rs     Cluster C            (~510)  ← stays whole, §6
├── toon.rs       Cluster D            (~350)
├── textures.rs   Cluster E + chart (F, part) (~210)
├── json.rs       Cluster G            (~325, tests included)
└── painters/
    ├── mod.rs    re-exports the six paint_* entry points
    ├── planks.rs     H (~330, tests included)
    ├── wallpaper.rs  I (~215, tests included)
    ├── felt.rs       J (~170, tests included)
    ├── parcel.rs     K (~130, tests included)
    ├── picture.rs    L (~125, tests included)
    └── sign.rs       M (~230, tests included)
```

`mod.rs` additionally re-exports `painters::{paint_planks, …}` at the
`scene::` level, since `scenefile.rs` dispatches them as
`scene::paint_planks` today.

### Dependency DAG (within `scene::*`, plus `math` below all)

```
                math (Mat4)
                  ▲
        ┌─────────┴─────────┐
     colour ──────► geo (draw records carry Mat4)
        ▲  ▲        ▲
        │  │        │ (Geo/TexGeo/ColourDraw/TexDraw)
     raster ────────┤
        ▲           │
        ├──── toon ─┤         json ── (leaf; no internal edges)
        ├─ textures │
        └─ painters/* (each: raster + colour only)
```

Edges are one-directional and already true in the current file by
textual order — the split makes them compiler-checked instead of
conventional. `json` touches nothing. `geo` needs `colour` only
through caller-supplied values today (builders take `[f32; 3]`
pre-linearised), so the `colour → geo` edge is *potential*, listed
for honesty; the real hard edges are `raster → colour` (`rgba_of`
parses hex), `toon → raster + colour`, `textures → raster + colour`,
`painters/* → raster + colour`, and everything → `math` only via
`geo`'s draw records.

### Boundary-crossing types — the honest count

**10 types cross module boundaries**, and they are the entire
coupling problem:

`Geo`, `MapGeo`, `TexGeo`, `ColourDraw`, `TexDraw` (geo layer) ·
`Painter`, `Rgba` (raster) · `ToonMood`, `ToonSpec` (toon) · `Value`
(json).

Plus ~46 free functions and 11 constants. That is a *wide but
shallow* surface: no type embeds another module's type except
`ColourDraw`/`TexDraw` (geo + `Mat4`) and `ToonSpec` (plain data).
There are no trait objects, no generics crossing boundaries except
`cov_area`'s closure parameter (internal to raster), and no
lifetime-carrying types except the parser's internal `JsonParser<'a>`
(which never crosses). Judgement: **this is a filing problem, not a
design problem.** The split is low-risk precisely because the
coupling is already this thin — the lead should resist any urge to
"improve" signatures en route; every signature change converts a
code move into a behaviour risk.

---

## 5. Extraction steps — leaf-first, one module per commit

Gate discipline: every step is a pure move (no logic edits, no
renames beyond the path), CI green required before the next step,
and the named evidence re-checked at the step where it applies.
`cargo test` (CI) runs the fused painter/json test modules — a test
count drop at any step means a test module was left behind; stop.

| # | Extract | Gate (existing evidence) |
|---|---|---|
| 1 | `json.rs` (Cluster G + `json_tests`) | Full CI green; `cargo test` count unchanged. Cheapest possible first cut: zero internal deps, and `scenefile`/`telemetry`/`film` parsing exercises it end-to-end. |
| 2 | `colour.rs` (Cluster A) | Battery **46/46** (colour conversion is under every stroke); fx-textures byte-identity unaffected but cheap to confirm in this step's CI. |
| 3 | `geo.rs` (Cluster B + F) | **s28 anchor pair** + **ab1-as-data 95.17 dB**: geometry is the §28 scene's skeleton; if a winding or vertex-order slip exists it shows here first. |
| 4 | `raster.rs` (Cluster C, whole) | Battery **46/46** again — this is *the* battery gate; the SDF model moves intact or the wave stops here. Also the M6 telemetry exact-agreement spot check (it paints through this code). |
| 5 | `toon.rs` (Cluster D) | Battery 46/46 (face cards) + s28 anchor (the Onoda card is `paint_toon` output). |
| 6 | `textures.rs` (Cluster E + chart fns) | **fx-textures byte-identity** (local vs CI sha) — the chart and the four recipes are fixture surfaces; dof-chart A/B as the second leg. |
| 7–12 | `painters/{planks,wallpaper,felt,parcel,picture,sign}.rs`, one painter per commit, code+tests fused | Per-step: that painter's own test module green in CI + fx-textures byte-identity (the loader arms dispatch all six). Order among the six is arbitrary; sign last (it is the only one with exported constants other modules may grow to touch). |
| 13 | Slim `scene.rs` → `scene/mod.rs`; delete the emptied file; staging-dir decision (§6) lands or is explicitly deferred in the commit message | Full gate suite once: s28 pair, ab1-as-data, battery, fx byte-identity, film telemetry verdict reproduced verbatim (`FILM TELEMETRY: CHECK OK`, exit 0 — the standing post-re-chunk verdict; "unchanged" is measured against this line, not the blueprint-era CHECK FAILED record), plus a grep proof that `grep -rn "crate::scene::" src/` resolves with no importer file modified across the whole series (`git diff --stat` on the seven importers = 0 lines). |

If any gate fails at any step: the step is a move, so the failure is
a move error — find it by diffing the moved text against the
pre-move file (`git show HEAD~1:src/scene.rs`), never by "fixing"
logic inside the new module.

---

## 6. Traps

1. **`pub(crate)` surface.** Every moved item keeps `pub(crate)`;
   `mod.rs` re-exports with `pub(crate) use`. The failure mode to
   expect is the opposite of the usual one: an item that was
   file-visible-by-default inside the monolith (e.g. planks' private
   helpers `planks_shade`, `planks_grain_pts`, felt's f64 trio) must
   **not** be promoted to `pub(crate)` just because the move makes
   the compiler ask. Helpers stay private to their new module; only
   today's `pub(crate)` items get re-exported. A surface audit
   (`grep -c "pub(crate)"` before/after, names diffed) belongs in
   step 13's commit.
2. **Importers of the quad/geometry helpers, named** (the audit asked
   specifically): `quad_geo`, `flat_quad_geo`, `ground_disc_geo`,
   `sky_sphere_geo`, and the `Geo` builders are imported by
   **`scenefile.rs`, `ab1.rs`, `film.rs`** (use-blocks at
   scenefile.rs:26, ab1.rs:27, film.rs:33) and by nobody else.
   `disc_geo`/`cyl_geo` are additionally called qualified from
   film.rs (882, 966) and scenefile.rs (746). All resolve through
   `crate::scene::…`, so re-exports cover them — but these three
   files are why a rename-in-transit would be expensive. Don't.
3. **The planks colour inconsistency is deliberate.** Lines
   1849–1857 document that planks' `planks_shade` is *not*
   `crate::scene::shade_rgb` — an inherited registry inconsistency,
   reproduced on purpose. An extractor "helpfully" unifying it
   changes pixels. Same class: felt carries its own f64
   `srgb_to_linear_f64`/`shade_three` (2368–2408) because its texel
   bytes are f64-exact; do not fold it into Cluster A's f32 pair.
4. **Tests are fused to code.** Six `#[cfg(test)]` modules live
   inside the regions they test (json 1765, planks 1960, wallpaper
   2245, felt 2428, parcel 2562, picture 2694, sign 2903). Moving
   code without its test module silently shrinks the suite — the
   step gates count tests for this reason.
5. **The staging `painters/` directory.** After step 12, three
   copies of each painter nominally exist in the repo's history:
   staging original, monolith paste (gone), module (live). The lead
   must either delete `painters/*.rs` in step 13 (keeping the `.md`
   provenance notes, or archiving the pack under `docs/`) or add a
   banner to `painters/INDEX.md` marking it superseded by
   `src/scene/painters/`. Leaving it unmarked recreates the exact
   "which copy is real?" trap the wave-1 paste created.
6. **`battery.rs` embeds its spec, not scene code**
   (`include_str!("../docs/ab/stroke-battery/spec.json")`) — the
   split does not touch it, but the battery is the gate for steps
   2/4/5, so confirm it runs in CI *before* step 1 starts; a battery
   that was silently not running would void three gates.
7. **No `mod` declaration edits outside `main.rs`.** `scene` is
   declared once in `main.rs`; converting to `scene/mod.rs` keeps
   that line valid. Submodules are declared in `mod.rs` only. If a
   step finds itself editing `main.rs`, it has taken a wrong turn.
8. **f32 literal drift.** This file is full of values transcribed
   under the project's f32 data law. A move must be byte-level:
   recommend `git diff -w` shows only path/use-line changes per step;
   any numeric literal appearing in a step's diff is a stop signal.

### What should stay monolithic — argued

**Cluster C (`raster.rs`, ~510 lines) should not be split further.**
The `Painter` impl, the SDF helpers, the cap fan and the coverage
functions are one mathematical model — contract §2's identity-critical
surface, verified as a unit by the battery. Splitting `stroke_*`
from `fill_*`, or the SDF helpers into the painters that use them,
would scatter exactly the code whose *joint* behaviour the 46/46
verdict certifies, and would multiply boundary types (every helper
would need re-export). 510 lines for one cohesive rasteriser is a
reasonable file. Revisit only if the rasteriser grows a second,
separable concern (e.g. a GPU-uploader) — it has none today.

**Each registry painter likewise stays whole** — code, private
helpers, constants and tests in one file of 125–330 lines. They were
authored as units, their tests are written against that unit, and
their provenance docs (`painters/*.md`) describe them as units.

Everything else in the monolith is filing, and the filing is overdue:
a 2,979-line file where a JSON parser, a rasteriser, a character
painter and six texture recipes share nothing but a directory is how
the next wave's agents end up editing each other's regions. The split
above costs thirteen pure-move commits and buys compiler-checked
boundaries at the exact seams the code already respects.

---

*Evidence basis: item/line census by `grep -n` over `src/scene.rs`,
`src/toonfull.rs`, and all `crate::scene` importers at the wave-2
audit head; `painters/INDEX.md` for the staging pack's integration
history; gate names (s28 anchor, ab1-as-data 95.17 dB, battery 46/46,
fx-textures byte-identity) as recorded in the M2/M4 packages and
`docs/integration/LOADER-EVIDENCE-W86.md`. No code was read from the
staging `painters/*.rs` beyond the index — the live copies in
`scene.rs` are the source of truth for every range above.*
