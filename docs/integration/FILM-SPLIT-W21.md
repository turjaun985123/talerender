# FILM-SPLIT-W21 — film.rs modularisation blueprint

Seat W21, TaleRender Wave 2. **Blueprint only — no code was touched.**
Drawn from a full end-to-end read of `src/film.rs` at the wave-2
working tree: **3,694 lines**, 16 cohesion clusters, ~190 top-level
items. Line numbers below are exact for that read; if the file has
moved since, re-grep the item names — the cluster structure, not the
line numbers, is the contract.

Purpose: the audit's Phase 2 asks for `film.rs` to be modularised.
The lead executes the split from this document, one module per
commit, **no behaviour change in any move commit**. The paint-budget
wiring (§6) is the single deliberately behaviour-touching step and
is sequenced and gated separately.

---

## 1. Region map — every top-level item, clustered

### Preamble — 1–42
Module docs (production-integration bridge, determinism law A4) and
imports: `ab1::{MapDraw, StdDraw, StdMat}`,
`composer::{BloomParams, GradeParams, PostChain}`,
`math::{self, Mat4}`, a 15-name `scene::{…}` import,
`toonfull::{paint_toon_full, FullLook, FullState, Legs, Mood, Plate}`,
`std::{HashMap, Path, PathBuf}`.

### C1 · Project data — 44–104
| Lines | Item | Vis |
|---|---|---|
| 44–49 | `struct FilmData` (duration, anchors, captions, lines) | pub(crate) |
| 51–57 | `struct Caption` (s, e, text, fit, strip) | pub(crate) |
| 59–61 | `fn jnum` | private |
| 63–104 | `fn load_film_data` (reads `onoda.json` via `scene::parse_json`) | pub(crate) |

### C2 · Motion formulas (core/lib.ts + moves.ts + poses.ts, f64) — 106–238
`clamp` 106, `lerp` 109, `seg` 112, `eo` 115, `eio` 118, `eob` 125,
`smoother` 130, `cam_k` 135, `type Key` 140, `key` 141, `keym` 144,
`pose_track` 150–168, `walk_phase` 169, `walk_bob` 172,
`struct Blinker` 175 + `impl Blinker::at` 178–204, `blink_at` 205,
`blink_max` 212, `settle` 215, `talk` 223, `mix3` 230–238.
**All pub(crate). Pure functions, zero state, zero GPU.**

### C3 · Frame contract types — 243–387, plus texture index consts 389–409
`struct PointState` 243 (+ `impl PointState::off` 249–255),
`struct LightState` 256–271, `struct PostState` 273–283,
`enum TexSrc` 285–288, `struct BasicDraw` 290–303,
`struct TexMapDraw` 305–313, `struct CharDraw` 315–326,
`struct CalendarState` 328–333, `struct FrameSpec` 335–359 +
`impl FrameSpec::new` 361–387 (**the method is private** — see §4).
Consts `TEX_SKY_JG`=0 … `TEX_PLANKS_COUNTER`=17, `TEX_SKY_FOG`=18,
`TEX_SKY_GOLDEN`=19, `TEX_SKY_DUSK`=20 (389–409; note 18/19/20 are
declared *before* 11–17 in source — the values, not the source
order, are the contract; see trap T1). All pub(crate).

### C4 · Cast looks (shots.ts LOOK table) — 415–485
`fn fl` 415–432 (private), `fn look_registry` 434–485 (pub(crate);
returns `Vec<(FullLook, bool, f64)>` — 11 instances).

### C5 · Texture painters (world.ts recipes) — 487–652
`fn rgb_of` 487, `fn shade_of` 490 (both private, thin wrappers over
`scene::hex_rgb`/`shade_rgb`); `paint_planks` 496–532,
`paint_wallpaper` 534–580, `paint_felt` 582–601, `paint_parcel`
603–618, `paint_picture` 620–652. All five painters pub(crate),
each returns `(Vec<u8>, u32, u32)`.

### C6 · Geometry / draw helpers — 654–862
`fn t3` 654, `fn sm` 662, `fn sme` 665, `fn add` 668 (pub(crate));
`fn def` 671 (private); `fn add_tree` 675–706, `fn add_palm` 708–747
(private); `fn box_map_geo` 749–772 (pub(crate)); `fn add_mapped`
774 (private); `fn s3`/`x3`/`n3` 779–788 (private — used **only** by
`add_clouds`); `fn add_clouds` 790–823 (private — called **only** by
shots, 13 sites); `fn add_sky` 825–838, `fn hills` 840–862 (private —
called only by the set builders).

### C7 · Set builders — 864–1132
`struct JgDyn` 864–868 (pub(crate); fields `fire`, `t`,
`extra_std: Vec<StdDraw>`); `fn build_jg` 870–962, `fn build_sh`
964–1002, `fn build_bk` 1037–1132 (all pub(crate));
`fn shelf_unit` 1004–1035 (private, BK-only).

### C8 · Evaluator support (rigs, tints, poses, camera) — 1134–1393
`fn sub3` 1134, `fn cross3` 1137, `fn norm3` 1140 (pub(crate) —
callers: rigs **and** `Renderer::render` at 2840–2841; `cross3` has
**no caller inside film.rs**), `fn fold` 1148, `fn folded_point`
1151, `fn env_tint` 1157–1176 (private); grade presets `g_jungle`
1178, `g_shore` 1181, `g_shop` 1184; light rigs `rig_noon` 1191,
`rig_golden` 1208, `rig_dusk` 1219, `rig_fog` 1230, `fire_light`
1239, `rig_bk` 1247 (all private); `const TINT_NOON` 1269,
`tint_golden` 1270, `tint_dusk` 1273, `tint_fog` 1276; instance
consts `ONODA`…`ONODA2` 1281–1290 + `SH_TANI` 1380 (declared
mid-cluster, between `cam_pitch_of` and `char` — move it with its
siblings); `fn fs` 1292, pose fns `pose_explain` 1295,
`pose_celebrate` 1301, `pose_mourn` 1307, `pose_listen` 1315,
`pose_think` 1322; `fn shoot` 1332–1350, `fn finish_chars`
1352–1371, `fn cam_pitch_of` 1373–1378, `fn char` 1382–1388,
`fn blink_of` 1391–1393 (all private).

### C9 · The shots + evaluator entry — 1395–2030
`s01` 1395, `s02` 1413, `s03` 1457, `s04` 1492, `plane_extras`
1519–1559, `s05a` 1561, `s05b` 1571, `s06` 1606, `calendar_state`
1659–1678, `s07` 1680, `s08` 1727, `s08b` 1767, `s09` 1796, `s10`
1831, `rifle_down_draws` 1863–1883, `s11` 1885, `s12` 1946 (all
private fns returning `FrameSpec`); `fn eval_frame` 1986–2030
(pub(crate)) — the segment table + dispatch + fade/caption tail.

### C10 · Assets — 2031–2079
`struct Assets` 2031–2037 (pub(crate); plates/strips/cal/cal_dark/
sign), `fn load_png` 2039 (private), `fn load_assets` 2045–2079
(pub(crate); takes `&FilmData`).

### C11 · Paint cache + character resolution — 2081–2275
`type Painted` 2081 (**private alias** —
`(Vec<u8>, u32, u32, (f32,f32), (f32,f32))`); `struct CharInfo`
2085–2102 (pub(crate)); `struct PaintCache` 2104–2108 +
`impl PaintCache` 2109–2130 (`new` and `get_or_paint` both
**private**; env cap `TALERENDER_PAINT_CACHE` parsed **on every
miss**, 2122–2125; `.expect("cache order")` at 2127);
`fn state_key` 2132–2160 (private; exact-f32-bit key — the cabcc30
determinism fix, **frozen format**, trap T2); `fn resolve_chars`
2162–2275 (**private**; `std::mem::take(&mut f.chars)`, paints via
cache, appends blob/card/rifle draws, returns
`(Vec<Rc<Painted>>, Vec<CharInfo>)`).

### C12 · Byte-pack helpers — 2277–2307
`fn mb` 2277, `fn pack_f32` 2288, `fn pack_u32` 2295,
`fn depth_bytes` 2302. All private, renderer-only.

### C13 · The GPU renderer — 2309–3332
`struct Renderer` 2309–2351 (pub(crate); ~40 wgpu fields);
`impl Renderer` #1: `async fn new` 2353–2790 (pub(crate); 437 lines
— adapter via `crate::gpucore::request_adapter`, static texture table
`textures_px` 2390–2412, all pipelines/targets);
`struct Prepared` 2791–2801 (private);
`impl Renderer` #2: `fn upload_dyn` 2803–2821 (private),
`pub(crate) fn render` 2823–3332 (**one 510-line method**: prepare →
shadow → prepass → PBR → bokeh → `self.post.bloom/output/grade`
with `BloomParams` 3279 / `GradeParams` 3288 built from
`spec.post`, `time: 0.0` pinned per ruling R3, `map_async`
readback). The two impl blocks are **one** logical unit (trap T10).

### C14 · CPU overlays (post-readback, sRGB space) — 3334–3450
`fn sample_bilinear` 3334, `fn blend_over` 3358,
`fn overlay_calendar` 3370–3418, `fn overlay_caption` 3420–3439,
`fn apply_fade` 3441–3450. All private. Caller: the driver only.

### C15 · Film driver — 3454–3506
`pub(crate) async fn run_film` — the only caller-facing render
entry: load data/assets → `Renderer::new` → per frame
`eval_frame` → `resolve_chars` → `render` → overlays in main.ts
order (calendar, fade, caption) → PNG save, skip-if-exists resume.
Returns `crate::errors::TaleResult<()>` (EXIT-CODES-W02 shape;
`.expect("pixel buffer size mismatch")` at 3497 is a known
remaining panic site — not this split's business, trap T7).

### C16 · Film telemetry glue — 3509–3694
`fn tel_look` 3509–3529 (private; `Box::leak` ×~9 per look to
fabricate `&'static str` for `telemetry::Look` — trap T6),
`fn mood_name` 3531, `fn legs_name` 3553,
`const INSTANCE_IDS` 3566 (all private);
`pub(crate) fn run_film_telemetry` 3570–3694 — own `PaintCache`,
0.25 s sampling of `eval_frame` + `resolve_chars`, `crate::telemetry::*`
glob import *inside* the fn (3571), caption ink-bbox audit, verdict
line `FILM TELEMETRY: CHECK OK / CHECK FAILED (…)`, returns
`Ok(bad == 0)` (bool wired to main's exit code by EXIT-CODES-W02;
**the line stays the verdict**).

**File-wide negatives (verified by grep):** no `static`, no
`OnceCell`/`OnceLock`, no `thread_local`, no lazy anything — the
only global-ish state is env vars read at runtime
(`TALERENDER_PAINT_CACHE` per cache miss; `TALERENDER_DOF_DEPTH_FORMAT`
once in `Renderer::new` 2527). No `#[test]` in the file. No module
outside `main.rs` names any `film::` item — the crate's whole
surface into film.rs is `main.rs:253` (`run_film_telemetry`) and
`main.rs:267` (`run_film`).

---

## 2. Target layout

Rust-2018 child modules: keep `src/film.rs`, add `src/film/*.rs`,
declare `mod …;` in film.rs. **film.rs stays the parent** — this is
load-bearing, see §4.

**Stays in film.rs (~230 lines):** the module docs + imports, the
whole frame contract C3 (frame-state types + `FrameSpec` +
`TEX_*` consts), the three shared vec helpers `sub3`/`cross3`/
`norm3` (they serve both staging rigs and `Renderer::render`),
the `mod` declarations, and a re-export block (§4).

| New module | Moves in | ≈ lines |
|---|---|---|
| `film::motion` | C2 entire | 145 |
| `film::data` | C1 entire | 75 |
| `film::paint` | C4 + C5 (looks + texture painters) | 240 |
| `film::sets` | C6 minus `s3/x3/n3`/`add_clouds`; C7 entire | 440 |
| `film::staging` | C8 minus `sub3/cross3/norm3`; `s3/x3/n3` + `add_clouds` from C6; C9 entire | 925 |
| `film::chars` | C10 + C11 entire | 250 |
| `film::render` | C12 + C13 entire | 1,055 |
| `film::overlay` | C14 entire | 120 |
| `film::driver` | C15 (`run_film`) | 60 |
| `film::telemetry` | C16 entire | 190 |

Notes on placements that differ from raw file order:

- **`add_clouds` + `s3/x3/n3` go to staging, not sets.** Their only
  callers are the 13 shot fns; the set builders never call them.
  File adjacency (they sit inside C6) is not cohesion.
- **`add_sky`/`hills` stay in sets** — called only by `build_jg`/
  `build_sh`.
- **`plane_extras`/`rifle_down_draws` stay in staging** — they
  produce `JgDyn.extra_std` payloads; the data flows staging →
  sets through the struct, never the reverse (trap T12).
- **`film::render` remains >1,000 lines after the split.** That is
  honest: `Renderer::new` (437) and `render` (510) are single
  functions. The split isolates the GPU chain behind one boundary;
  decomposing `render()` internally is a *later* Phase-2 step with
  its own design (it is also where gpucore/rendergraph wiring will
  land) and is **not** part of these pure-move steps.
- `film::paint` holds two sub-clusters (looks feed driver/telemetry;
  painters feed render's texture table). They share only the
  "2D paint data" theme and nothing else — acceptable as one module;
  splitting them further buys nothing.

What film.rs does **not** keep: any function body longer than
`FrameSpec::new`. There is no "thin driver" left in film.rs itself
— the driver is `film::driver`, re-exported.

---

## 3. Dependency DAG + crossing census

Edges point consumer → provider; in-subtree edges only (every
module also uses `scene`/`math`/`toonfull`/`ab1`/`composer` as
today — those edges don't change).

```
                 ┌──────────── film.rs (parent: FrameSpec & frame types,
                 │              TEX_* consts, sub3/cross3/norm3)
                 │   ▲ used by: sets, staging, chars, render, overlay
   driver ──┬──► data ◄── staging, chars, telemetry
            ├──► staging ──► motion (leaf: nothing in-subtree)
            │       │    └──► sets ──► (parent types only)
            │       └───────► data, parent
            ├──► chars ──► data, parent, sets helpers (t3/sm/add)
            ├──► render ──► parent, chars (Painted type), paint (5 painters)
            ├──► overlay ─► chars (Assets), parent (CalendarState)
            └──► paint (look_registry)
   telemetry ─► data, staging (eval_frame), chars, paint (look_registry)
```

**No cycles.** The two apparent risks check out: `chars` names no
renderer type (render consumes `chars::Painted`, chars never names
`Renderer`); `sets` never calls staging (`extra_std` is data-in).

Crossing census — distinct item names crossing each in-subtree edge:

| Edge | Names | Count |
|---|---|---|
| staging → motion | clamp, lerp, seg, eo, eio*, smoother, cam_k, key, keym, pose_track, walk_phase, walk_bob, Blinker, blink_at, blink_max, settle, talk, mix3 (*eio via eval tail) | 16 |
| staging → sets | build_jg, build_sh, build_bk, JgDyn, t3, sm, add | 7 |
| staging → parent | FrameSpec, LightState, PostState, PointState, CalendarState, TEX_SKY_FOG/GOLDEN/DUSK | 8 |
| staging → data | FilmData | 1 |
| sets → parent | FrameSpec, BasicDraw, TexSrc, ~10 TEX_* consts | 13 |
| chars → data / parent / sets | FilmData · FrameSpec, BasicDraw, TexSrc, TEX_BLOB · t3, sm, add | 1+4+3 |
| render → parent / chars / paint | FrameSpec, TexSrc, sub3, norm3 · Painted · 5 painters | 4+1+5 |
| overlay → chars / parent | Assets · CalendarState | 2 |
| driver → all | load_film_data, look_registry, eval_frame, load_assets, resolve_chars, PaintCache, Renderer, overlay_calendar, overlay_caption, apply_fade | 10 |
| telemetry → all | load_film_data, look_registry, eval_frame, load_assets, resolve_chars, PaintCache, CharInfo | 7 |
| **Total** | | **≈82** |

**Verdict on the split's quality — read this before objecting to
the count.** 82 name-crossings *sounds* like the "40 cross-imports
= bad split" failure mode. It isn't, for three measured reasons:

1. **77 of the 82 target items are already `pub(crate)` today.**
   The crossing count is a *reading* cost (imports at the top of
   each child), not a *change* cost. The actual visibility delta
   for the entire split is **5 items** (§4).
2. **The two fat edges are fat by design, not by entanglement.**
   staging → motion (16) is a transcription layer calling a pure
   formula leaf — a glob import of side-effect-free f64 functions.
   sets → parent (13) is builders writing into the shared frame
   contract — that contract is precisely what the parent module
   exists to hold.
3. **No mutable state crosses any boundary except one place, and
   it already crosses today:** `PaintCache` is passed `&mut` into
   `resolve_chars` by its two owners (driver, telemetry). The
   split does not create, widen, or duplicate that flow.

So: wide-but-shallow. The alternative "fewer modules" split
(staging+sets+motion as one ~1,500-line child) reduces the import
count by hiding the same dependencies inside one file — it does
not reduce coupling, it reduces *visibility of* coupling. This
blueprint recommends the honest map.

---

## 4. Visibility delta — the exact list

Rust privacy rule that does the heavy lifting: **items declared in
`film` (the parent) with default visibility are visible in all of
film's descendant modules.** Keeping C3 + the vec helpers in
film.rs means every child can use `FrameSpec`, `BasicDraw`,
`TEX_*`, `sub3`… with **zero** visibility edits — including the
private `FrameSpec::new` method, called today by every shot fn
and still callable from `film::staging` unchanged.

Everything that is `pub(crate)` today keeps its visibility and
moves as-is. The complete delta for the whole split is **five
widenings, all private → `pub(super)`** (visible across the film
subtree, still invisible outside it):

| # | Item | From → to | Why |
|---|---|---|---|
| 1 | `type Painted` (2081) | private → pub(super) | `render::render`'s signature names `&[Rc<Painted>]`; produced by `chars`, routed by driver |
| 2 | `PaintCache::new` (2110) | private → pub(super) | constructed by driver + telemetry |
| 3 | `fn resolve_chars` (2162) | private → pub(super) | called by driver + telemetry |
| 4 | `fn overlay_calendar` (3370) | private → pub(super) | called by driver |
| 5 | `fn overlay_caption` (3420), `fn apply_fade` (3441) | private → pub(super) | called by driver |

Deliberately **not** widened (their only callers move with them):
`state_key`, `PaintCache::get_or_paint` (caller: `resolve_chars`),
`load_png`, `jnum`, `fl`, `rgb_of`/`shade_of`, `def`, `add_tree`,
`add_palm`, `add_mapped`, `shelf_unit`, `fold`, `env_tint`, all
rigs/tints/poses, all shot fns, `mb`/`pack_*`/`depth_bytes`,
`upload_dyn`, `Prepared`, `sample_bilinear`, `blend_over`,
`tel_look`, `mood_name`, `legs_name`, `INSTANCE_IDS`.

**Path preservation (the compatibility surface).** `main.rs` is
the only external consumer (`film::run_film`,
`film::run_film_telemetry`). Regardless, film.rs re-exports the
*entire* current `pub(crate)` surface — one
`pub(crate) use <child>::{…};` block per child module — so every
`crate::film::<name>` path that exists today keeps resolving.
This costs ~15 lines and makes the split invisible to the rest of
the crate, present and future.

**Adapter (amended, SPLIT-AMEND-W82):** there is no adapter
decision in this split. The re-point the original text forbade
already happened outside the split (`aadc193`, gpucore site
5/5): main.rs's `request_adapter` helper is **deleted**, the
single implementation is `crate::gpucore::request_adapter`
(gpucore.rs:63), and every site calls it — film.rs (inside
`Renderer::new`), main.rs, scenefile.rs, ab1.rs, dof.rs. The
call inside `Renderer::new` is ordinary moved code in step 8;
it moves verbatim like everything else. (Residue, not split
business: gpucore.rs's header comment still credits
`crate::request_adapter` — a stale comment in a file this
programme does not touch.)

---

## 5. Extraction sequence — risk-ordered, gated

**Gate vocabulary** (existing project evidence; no new tooling):

- **G-CI** — ci.yml + tools-ci.yml green on the step commit (repo
  law: CI is the only build; no local cargo).
- **G-PROBE** — determinism byte-probes re-rendered byte-identical
  via render.yml dispatch overlaps: frames **195–204** (10/10),
  **795–814** (20/20), **1195–1204** (10/10) — the E4 probe set
  from DETERMINISM-CONTRACT-W04 at the final wave-1 head.
- **G-TEL** — `--film-telemetry` verdict line reproduced
  **verbatim**: `FILM TELEMETRY: CHECK OK` (the standing record
  since the caption re-chunk, `5e90d7e`; the pre-re-chunk
  CHECK FAILED line with 7 caption failures and its waiver are
  history, superseded by the Wave-2 re-chunk) and exit code 0.
  Exit wiring, unchanged in form: main.rs maps `Ok(true)` → 0,
  `Ok(false)` → 1, `Err` → 3. The verdict moving in *either*
  direction = stop and investigate; a return to CHECK FAILED
  is a regression signal in a pure move, exactly as an
  unexpected "improvement" was under the old wording.
- **G-VAL** — `tools/film/validate_film.py` 7/7 (with the standing
  `--no-source` coverage caveat on runners).
- **G-AB** — A/B spot pairs from the wave's 60-pair set
  (`film-final/ab-out4/`), per-step pairs named below; PSNR must
  match the recorded per-shot means within pair noise — for a pure
  move the expectation is *identical frames*, so any pair dropping
  below its shot's recorded mean is a stop.

**Universal commit shape (every step):** one module per commit;
the diff is item moves + `mod` declaration + (only where §4 lists)
visibility bumps + imports. No renames, no dedup, no logic edits,
no comment "improvements" inside moved code. Message:
`film split N/10: extract film::<name> (pure move, no behaviour change)`.
CI green before the next step starts. **If a step needs a
non-move edit to compile, stop**: that means this map missed a
crossing — record the edit in the commit body and flag it back
against §3 before proceeding.

| Step | Extract | Why this position | Gates |
|---|---|---|---|
| 1 | `film::motion` | Leaf, pure f64, all items already pub(crate), zero visibility delta. Proves the child-module mechanics on the lowest-risk cluster. | G-CI, G-TEL (motion feeds every eval sample) |
| 2 | `film::data` | Nearly leaf (depends only on `scene::parse_json`); FilmData is the most-referenced type, so pinning its path early de-risks every later step's imports. | G-CI, G-TEL, G-VAL |
| 3 | `film::paint` | Painters feed render's texture table; looks feed driver/telemetry. Self-contained (scene Painter + toonfull only). | G-CI, G-PROBE, G-AB: **S10 mid pair mandatory** (BK is the only set using planks/wallpaper/felt/parcel/picture) + S01 pair |
| 4 | `film::chars` — **pure move only** (Assets, load_assets, Painted, CharInfo, PaintCache, state_key, resolve_chars; widenings #1–#3). Budget wiring is explicitly **not** in this step. | The determinism heartland (state_key + paint purity) moves while nothing about its behaviour is in flux. | G-CI, G-PROBE (headline gate — probes exist because of this cache), G-TEL, G-AB: S08 pair (the cache-boundary shot) |
| 5 | **Paint-budget wiring** — separate commit, behaviour-touching; full detail in §6. | Sequenced *after* the cache sits in its new home, so the policy change is one reviewable diff against a stable module. | §6 gates |
| 6 | `film::sets` | Builders + geo helpers; `add_clouds`/`s3/x3/n3` stay behind in film.rs for step 7 — extraction is by item, not line range; a temporarily mixed film.rs mid-sequence is expected and fine. | G-CI, G-PROBE, G-AB: S02 (shore) + S10 (BK) pairs |
| 7 | `film::staging` | Largest move (~925 lines) deliberately late: by now every provider it imports (motion, data, sets, parent types) has a settled path. Includes `add_clouds` + `s3/x3/n3`, rigs, poses, all 14 shot fns, `eval_frame`. | G-CI, G-PROBE, G-TEL, G-AB: S01, S02, S07 (golden JG), S10, S11 (dusk) pairs |
| 8 | `film::render` | GPU chain moves as one block (pack helpers + Renderer + Prepared). Nothing else remains that render depends on except `chars::Painted` (settled in step 4) and painters (step 3). | G-CI, G-PROBE, G-AB: 6 pairs across sets. Optional-but-recommended at this step only: full 60-pair A/B + one full-film re-render, since this is the production chain itself |
| 9 | `film::overlay` | Pure CPU post-readback; widenings #4–#5. **G-TEL is not evidence here** (telemetry never runs overlays) — the gate is direct frame comparison. | G-CI + byte-compare pre/post of one S07 calendar frame (≈ frame 950, calendar active) and one captioned frame — sha256 equal |
| 10 | `film::telemetry` + `film::driver` | Entry points move last, onto fully settled providers; film.rs reaches its final shape (types + mods + re-exports). | G-CI, G-TEL (both halves of the contract: line verbatim — `FILM TELEMETRY: CHECK OK` — + exit 0), CLI smoke: `--film` short range (e.g. `--from 195 --to 205`) renders + skips on re-run |

After step 10: one closing commit may update the film.rs header
docs to describe the module layout (docs-only, no code motion).

---

## 6. Step 5 in detail — paint-budget wiring (the one non-move)

**Correction to any assumption that this is "already wired":**
at the time of this read, `src/paintbudget.rs` (422 lines, W06)
was an **orphan file** — not in `main.rs`'s `mod` list, and
`film.rs` contained **zero** references to it. `PaintCache` at
read time enforced only the entry cap (default 160, env
`TALERENDER_PAINT_CACHE`, re-parsed on every miss).

> **Postscript (same evening, after the read):** the lead's
> commit `2328e6f` has since registered the wave modules
> (`mod paintbudget;` now at main.rs:29), and the lead's
> uncommitted working-tree diff already adds the first
> `PaintBudget` references inside `film.rs`'s PaintCache area
> (~7 refs). **Before executing step 5, re-check its premises
> against the then-current tree** — item 1 below may already be
> done, and the wiring may be partially in flight under the
> lead's own hand. Steps 1–4 and 6–10 are unaffected; only the
> PaintCache impl's line numbers (~2104–2130) may have shifted.

The wiring commit does exactly four things:

1. Register `mod paintbudget;` in main.rs.
2. `PaintCache` gains a `budget: PaintBudget` field, constructed
   **once** in `PaintCache::new`: entry cap resolved from
   `TALERENDER_PAINT_CACHE` (as today), then
   `PaintBudget::from_env(entry_cap, CANVAS_BYTES)` (adds
   `TALERENDER_PAINT_BUDGET_MB`; 0 disables byte enforcement).
   The per-miss env parse disappears.
3. `get_or_paint` reports per its call contract (`on_hit` /
   `on_miss` / `on_insert(entry_bytes)` / `on_evict(key, bytes)`)
   and the eviction loop becomes
   `while map.len() > cap || budget.should_evict()`.
4. Driver and telemetry print `budget.stats_line()` at end of run
   (the PERF.md §9 missing signal). New stdout lines in the two
   run paths are the only intended output change besides eviction
   timing.

**Known semantic delta, stated plainly:** the derived default
byte budget is `min(160 × 28.85 MB, 2 GiB ceiling) = 2 GiB` —
stricter than today's ~4.6 GB worst case (~74 canvases vs 160).
Per PAINT-BUDGET-W06's own analysis that is ≥3× the telemetry-24
working set and far above any single shot's live set, so eviction
should never bind *within* a shot; across a long film it will
evict-and-repaint more often (time cost only — painting is a pure
function of the exact key, §T2). This is a policy change the lead
accepts explicitly by making this commit separate.

**Gates for step 5:** G-CI · G-PROBE · G-TEL at defaults ·
G-TEL again with `TALERENDER_PAINT_CACHE=24` (the W85
configuration — verdict must reproduce) · G-AB S08 pair ·
`stats_line` present in both run outputs with `peak_bytes`
≤ budget. If probes or telemetry move, the purity assumption is
wrong somewhere — revert the wiring, keep the step-4 module.

---

## 7. Traps

- **T1 · TEX index ↔ texture table positional coupling.**
  `Renderer::new`'s `textures_px` vec (2390–2412) is indexed by
  the `TEX_*` consts (389–409); slot 10 is not a painter at all —
  it's `assets.sign` passed in as a parameter. The consts and the
  vec live in different future modules (parent vs render). Any
  reordering on either side silently swaps textures film-wide;
  byte-probes (S08-centred) may *not* catch a sky/planks swap —
  G-AB will. Rule: neither side's order is touched in any step;
  the step-8 commit body should state the table was moved
  verbatim.
- **T2 · `state_key` is a frozen format.** The exact-f32-bit key
  (2132–2160) is the cabcc30 determinism fix; its comment records
  the 72.34 dB incident. Move it byte-for-byte. No formatting
  "cleanup", no field reordering, no `{:?}` changes — ever — in
  this split. (A key change is pixel-neutral *in theory*; the
  incident history says the theory is not the point.)
- **T3 · Env reads are behaviour.** `TALERENDER_PAINT_CACHE`
  parsed per-miss (2122–2125) and `TALERENDER_DOF_DEPTH_FORMAT`
  read in `Renderer::new` (2527) must stay exactly where they are
  through the pure moves. Hoisting the cache env read to
  construction is part of step 5 and *only* step 5.
- **T4 · The shot table exists twice.** eval_frame's dispatch
  table (2007–2022) and run_film_telemetry's `ShotInfo` table
  (3583–3598) encode the same 14 cut points, including the three
  word-anchored offsets (`thought4 −0.12`, `decades6 −0.25`,
  `refused7 −0.1`). The split puts them in `staging` and
  `telemetry` respectively — after which drift is cross-file.
  Do **not** unify them during the split (different shapes:
  dispatch fns vs audit data). Recorded here as a follow-up
  refactor with its own gate (G-TEL verbatim before/after).
- **T5 · Duplicate vec helpers.** `s3/x3/n3` (779–788) and
  `sub3/cross3/norm3` (1134–1147) are the same math under two
  names. This blueprint keeps both (s3-set → staging with
  `add_clouds`; sub3-set → parent, because `render` also calls
  `sub3`/`norm3` at 2840–2841). Dedup is a separate gated commit
  if ever; merging during a move muddies the diff. Note
  `cross3` has no in-file caller (pub(crate) hides the dead-code
  warning) — leave it; removal is not split business.
- **T6 · `tel_look` leaks on purpose.** `Box::leak` fabricates
  `&'static str` for `telemetry::Look` (3510–3516) — bounded
  (~99 strings per telemetry run), intentional, and a wart of
  the telemetry API's lifetime design, not of film.rs. Moves
  as-is. Do not "fix" mid-split.
- **T7 · Panic sites ride along unchanged.** `.expect("cache
  order")` (2127, inside `get_or_paint`) and
  `.expect("pixel buffer size mismatch")` (3497, driver).
  Structured-error conversion is the errors track's business
  (ERRORS-W05 / Phase 2 D); converting them inside move commits
  breaks the pure-move rule.
- **T8 · Introduce no statics.** The file today has none (§1
  negatives). The budget wiring keeps its state inside the
  `PaintCache` instance (constructed per run by driver/telemetry,
  so counters are per-run by construction). Any step that reaches
  for a `static`/`OnceCell` to make something compile has taken a
  wrong turn — stop.
- **T9 · Telemetry's glob import hides an edge.**
  `use crate::telemetry::*;` *inside* `run_film_telemetry` (3571)
  pulls ~12 names (Audit, Cutout, ShotInfo, ProjCamera, describe,
  pose_summary, PoseState, scan_canvas_overflow, project_box,
  model_from_f32, look_signature, print_caption_audit,
  CaptionInfo). The §3 census counts only the explicit signature
  edge; the glob is why `film::telemetry`'s real coupling to
  `crate::telemetry` is larger than it looks. It moves verbatim.
- **T10 · Cohesion that only looks accidental — see §8.**
- **T11 · `Renderer::new` takes the sign texture as a raw tuple**
  (`sign: &(Vec<u8>, u32, u32)`), not `&Assets`. That is a
  *good* boundary — render does not depend on chars' asset
  loading, only on the `Painted` type. Preserve it; do not
  "improve" the signature to take `&Assets` during step 8.
- **T12 · `JgDyn.extra_std` is a one-way data bridge.** Shots
  build `Vec<StdDraw>` payloads (plane, leaflets, fallen rifle)
  that `build_jg` splices in. If a future reader is tempted to
  give sets a staging dependency to "simplify" this, the DAG
  gains a cycle (staging → sets → staging). The struct is the
  seam; keep it.

---

## 8. What NOT to split (cohesion that only looks accidental)

1. **`resolve_chars` stays whole** (chars). Its rifle-attachment
   block and blob/card appends look like render prep, but the fn
   *mutates the FrameSpec* (`mem::take` of chars, pushes into
   `basics`/`std`) while painting through the cache keyed by
   `state_key`. Key ↔ paint ↔ draw-append is one triangle;
   splitting any leg away severs the determinism argument that
   holds the three together.
2. **`finish_chars` stays in staging** (not render). It writes
   `yaw`/`pitch`/`cam_pitch` into `CharDraw.state` — and
   `cam_pitch` is a **state_key channel**. It is an input to the
   paint determinism contract, not a render detail.
3. **eval_frame's fade/caption tail stays in staging**
   (2024–2029). It looks like driver work; it actually writes
   `FrameSpec.fade/caption/caption_scale`, which the driver's
   overlays consume and which telemetry's identical-eval
   assumption rests on. Moving it to the driver would make
   telemetry evaluate a *different* function than the renderer
   runs.
4. **The two `impl Renderer` blocks are one module.** They are
   separated in-file only by `struct Prepared`. Splitting "new"
   from "render" into different modules would force ~40 private
   wgpu fields across a boundary for zero cohesion gain.
5. **The shots' post-hoc `f.chars[idx].hold_*` pattern**
   (s05b/s07/s08/s08b/s09/s10) is not a defect to refactor during
   the split. `char()` pushes and returns nothing; callers index
   back in. A builder API is a behaviour-adjacent redesign —
   out of scope here.
6. **`overlay_calendar`'s inline dark-plate block** (3395–3410)
   stays inside the fn. Extracting a helper is a fine later
   cleanup; inside a move commit it is noise.

---

## 9. End state, in one paragraph

`film.rs` ≈ 230 lines (docs, frame contract, 3 vec helpers, mod
decls, re-exports) + ten children, largest `film::render` ≈ 1,055
and `film::staging` ≈ 925 — both large because they contain
single large *functions*, which is the next phase's problem, now
visible behind clean boundaries instead of mixed into 3,694
lines of everything. Total visibility delta: 5 × private →
`pub(super)`. Total behaviour delta across steps 1–4 and 6–10:
zero, each proven by the gates named per step. Step 5 is the
only intended behaviour change in the programme, and it is the
audit's own Phase-3 head item (byte-budgeted paint cache)
landed early because the split makes it a one-module diff.

— W21
