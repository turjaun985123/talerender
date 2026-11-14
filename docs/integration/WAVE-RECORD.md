# Wave record — the 100-agent production-integration wave

Consolidated record of what the wave produced, what it corrected,
and what it flagged, for the integration phase. Written
2026-11-14 (seat W37) from the wave's own records:
`scenes/film/ASSUMPTIONS.md` (the per-shot authoring record —
this file summarises it, it does not replace it),
`scenes/sets/README.md`, `docs/integration/INVENTORY.md`,
`tools/film/VALIDATION.md`, `painters/*.md`,
`shaders/fog/FOG.md`, `docs/golden/NATIVE-GOLDEN.md`, and the
git log. Divergence statuses were re-verified against the tree
at HEAD `311656d` on the day of writing.

The wave's shape: the lead seat owns the engine core
(`src/film.rs`, `src/scenefile.rs`, shaders, spec) and has been
integrating throughout; 30 wave seats (Batches 1–3, W01–W30)
produced the film data set, the texture painters, the fog
package, the validators and the golden pack in file-disjoint
paths. Commits: `0972e2c` (Batch 1), `532dc5f` (Batch 2),
`311656d` (Batch 3); rulings and engine fixes by the lead in
`5e5b0a0` and `4c06a1e`.

---

## 1. The film data set — state

Film: **"THE SOLDIER WHO DIDN'T KNOW THE WAR WAS OVER"**
(Onoda), 60.357 s, 30 fps, **1,811 frames**, audio = the
project's existing `data/mix.wav` (−14.0 LUFS) muxed unchanged.

| Component | Contents | State |
|---|---|---|
| `scenes/film/film.json` | Manifest: 14 segments (s01, s02, s03, s04, s05a, s05b, s06, s07, s08, s08b, s09, s10, s11, s12), frame ranges tiling [0, 1811) exactly, caption ranges by the start-time rule | committed |
| `scenes/film/*.scene.json` ×14 | Every segment as format-v2 data: set nodes, v2 `lights`, character nodes with evaluated `state` at the file's `time` + `channels` carrying the motion formulas, camera keys, caption chunk objects, cues/fades as data | committed; C1–C9-corrected (§4) |
| `scenes/sets/{shore,bookshop,jungle}.scene.json` | The three sets as standalone fragments (SH 47 nodes/229 meshes; BK 21 nodes/369 meshes incl. **310 books** in 360 deterministic slots; JG 78 nodes/325 meshes + 900-tuft scatter) + a merge-before-load proposal in `scenes/sets/README.md` | committed; merge step is the integrator's |
| `scenes/atmospheres.json` + `ATMOSPHERES.md` | All 7 env.ts presets, the base rigs, and **6 merged filmLooks** (jg-noon, sh-noon, jg-fog-s06, jg-golden-s07, bk-noon-shop, jg-dusk) with the dynamics as formulas | committed |
| `assets/captions/` + `tools/captions/` | All **66 caption chunks** as PIL strips + manifest. Fit table matches Chrome's audit exactly (min fit 0.9227, max renderedW 960, same 7 chunks over the 850 floor); **PIL-vs-Skia measured: max Δ 0.15 px, no audit verdict flips**; byte-deterministic re-runs | committed |
| `scenes/film/s07-calendar.json` | The S07 calendar overlay as data (card geometry, 8 elements, flip cycle, alpha law), transcribed from `drawCalendar` | committed |
| `film/` (lead) | `onoda.json` (staging data), `gen_assets.py` (plates: captions, calendars, sign, hold props), `gen_film_data.py`, `finish.sh`, `ab_compare.py`; generated assets committed in `b3f3c0a` after the first CI film render exposed a repo-wide `*.png` ignore silently excluding them | committed |

**Validation** (`tools/film/validate_film.py` v2, run record
`tools/film/VALIDATION.md`): **5/7 checks PASS** over all 14
files — parse 14/14, format version 2, attachments (0 top-level,
10 on character nodes), durations (tiling exact, 60.357 s →
1811 frames), atmosphere under R5 (14/14 presets match, 14/14
resolve to filmLooks). Two residual failures, every item listed
verbatim in the run record:

- **f32:** of 1,389 numbers — 814 exact f32, 506 verified source
  literals (pass), **69 computed values written rounded** (fail):
  mostly camera/channel key times and evaluated state values.
  Genuine §11 data-law violations, now isolated from the
  literals W20's blunt check had lumped in.
- **captions:** chunks **46** (s08 + s08b; starts 39.95, owned
  by s08) and **56** (s10 + s11; starts 49.63, owned by s10)
  still double-listed — the defect W27 fixed for chunks 31/39
  (§4), in files outside that seat's scope. All 66 chunks
  covered, 0 uncovered.

**Golden pack** (`docs/golden/NATIVE-GOLDEN.md`): 18 items —
**4 OK** (the pinned milestone references: §28 frame, dof-chart,
stroke toon, ab1-composer), **0 STALE**, **14 MISSING** (the
film segment mid-frames, until the film renders). Policy:
native references only, clean PNG, re-baseline is deliberate +
measured + disclosed; A/B vs Chrome is reported, never gated as
equality.

---

## 2. Rulings R1–R5 (docs/scene-format.md §14.5) — and where each is applied

| Ruling | Content | Applied in |
|---|---|---|
| **R1** | `lights` blocks carry POST-atmosphere values (what `applyAtmosphere` leaves in force), never constructor/`sunlight()` values. Noon: key #FFF0D6 ×2.6, rim ×0.4, hemi #CFE6FF/#C48A5A ×1.25. Fire point carries its rendered basis **4.6** with the `fireLevel` formula, never the constructor 5.0 | All 14 scene files + 3 set files (BK is the exception that proves the rule: `applyAtmosphere(BK,'noon')` passes no rig, so BK carries its own interior rig — key #FFE4B8 ×2.4 — verified field-by-field against W01's independent transcription). Batch-1 files reconciled by W19 (§4). The lead's §14.2 spec example + `character-v2.scene.json` corrected in `4c06a1e` |
| **R2** | `attachments` lives ON the character node (`{kind, hand, offset}`); no top-level attachments block | All scene files (validator: 10 on-node, 0 top-level). Engine side: the loader now consumes attachments — rifle anchored at the painted hand via the syncAttachments offset (`4c06a1e`) |
| **R3** | GRADE grain time = 0 for the whole film (no source path sets `look.t`; grain is a fixed spatial pattern) | film.rs pins the grade time to 0. Data residual: s05b/s07 still record `grade.time` = frame t (25.31 / 30.39) on their face — inert (the renderer pins 0) but contradicting the ruling in the data; flagged in §5 |
| **R4** | Source wins over the evaluator on staging constants — the wrong side is fixed, never averaged | The divergence register, §5. Exercised by the lead in `5e5b0a0` (S06 walkBob + mourn) and by every scene seat (data follows shots.ts throughout) |
| **R5** | Scene files name atmosphere PRESETS; the driver resolves (set × preset); `filmLooks` is a derived validation index, not scene vocabulary | All 14 files declare presets; validator check 7 implements the resolution (14/14 resolve); film.rs's rig builders are the render-time resolution |

---

## 3. Painter modules + fog package — integration points

All modules are staging files in `painters/` (nothing in `src/`
was touched by wave seats). Each mirrors the registry painters'
signature `(Vec<u8>, u32, u32)`, is pure/deterministic, and
documents its own dispatch arm.

| Module | Generator it registers | Dispatch / wiring | Verification state |
|---|---|---|---|
| `painters/planks.rs` | `planks` (crates, barrel, lean-to roof, BK floor/wainscot/counter/desk) | `"planks" => scene::paint_planks(str_field(spec, "base", ctx)?)` in `resolve_texture`. Only `base` is a painter input; `repeat`/`rotation` stay material-side, as in the source | 7 in-module tests, analytically pinned; **not run locally** (no cargo) — CI on integration |
| `painters/wallpaper.rs` | `wallpaper` (BK walls/ceiling) | `"wallpaper" => scene::paint_wallpaper(base, motif)`; carries its own linear-space `shade_hex` (pins #413427, §5 D11) | 6 tests, analytically pinned; not run locally |
| `painters/felt.rs` | `felt` (counter/desk tops) | `"felt" => scene::paint_felt(base)`; private `shade_three` reproduces THREE.Color arithmetic in f64 | **Tests run**: scratch harness against the real scene.rs — 22/22 incl. existing module tests; pinned statistic 50,047/65,536 base survivors for #1F5A45 (±6, ulp-measured) |
| `painters/parcel.rs` | `parcel` (desk parcels, label false in this film) | `"parcel" => scene::paint_parcel(label)` | Tests run in the same harness; exact integer-aligned pins (7,936 tape px; label counts 5,820/480) |
| `painters/picture.rs` | `picture` (BK wall picture) | `"picture" => paint_picture()` (fixed 256×200, no params); frame colour is geometry-side (`PICTURE_FRAME` const) | Tests written; **not run** — sun-disc statistic is a band to tighten to an exact count on the first CI run |
| `painters/sign.rs` | `sign` ('TANIGUCHI BOOKS' board) | `"sign" => paint_sign_px(bg, fg, cw, ch)` + `sign_with_text(board, strip)` — board/border native, glyphs composited from a PIL strip (the engine has no Rust text rasteriser, by design). Font size: the shrink rule measured with PIL against the project's Caladea-Bold — 335.36 px at 40 px vs the 378.4 limit → **fs = 40, no shrink fires** (PIL-measured data; browser metrics remain unprovable — gap B9 stands) | Tests written; not run |
| `painters/calendar.rs` | S07 calendar overlay (vector path) | `composite_over(&mut px, w, h, t)` at the point film.rs calls `overlay_calendar`, or `render_overlay(t)` standalone. Hand-authored glyphs for the fixed string set only — **an approximation; the PIL plates remain the reference rendering** | **7/7 tests pass** under standalone `rustc --test`; overlays visually inspected at three times |
| `shaders/fog/fog.wgsl.inc` + `FOG.md` | three.js linear fog for the PBR + basic shaders | **Already wired by the lead** in the film path (FrameU fog fields, DrawU per-draw flag, film.rs sets S06/S07/S11/S12) — the package is the canonical extraction (must not change a pixel), the ground truth (fog factor is **smoothstep**, not the classic ramp — they differ by up to 0.078 over S06's range), the values table, and a fog-chart verification plan with analytic factors (3/6/10/15/25 m → 0 / 0.0263 / 0.2884 / 0.7550 / 1.0; pass bar ±2 sRGB codes on panel centres) | Chart not yet built/run — verification plan only |

Registry state after integration of the above: the six
generators the set/shot data names (`planks`, `wallpaper`,
`felt`, `parcel`, `picture`, `sign`) all have painters; until
the arms land in `resolve_texture`, scene files referencing
them fall back to averaged flat shades in the still loader.

---

## 4. Corrections register

What was wrong in the ab1-derived data, and the source-faithful
values now in place. Source of the C-list: seat W18's jungle
set audit (`scenes/sets/README.md`, Jungle part); application
to the shot files: seat W26; reconciliation: seat W19; caption
ownership: seat W27.

| # | Was (ab1 form) | Source-faithful (now in the data) | Citation |
|---|---|---|---|
| C1 | Palm frond colours inverted (even → leaf) | Frond i odd → leaf `#3F7D33`, even → leaf2 `#4C8A3A`; tips the opposite | shots.ts:93, :95 |
| C2 | Palm tips 10×6 segments | Tips `SphereGeometry(1, 8, 5)`; fronds stay 10×6 | shots.ts:95 |
| C3 | Hill spheres 24 wide segments | 32 | world.ts:182 |
| C4 | Tree trunks 7 segments | 12 (the `cyl()` default) | world.ts:54 |
| C5 | Ground disc 48 segments | 40 (48 is the grass repeat, kept as `uvRepeat`) | shots.ts:146 |
| C6 | Hills flat colour (map dropped in v1) | Colour × `grass-hills` map, repeat [6,3] | world.ts:183 |
| C7 | Crates/barrel/roof flattened to single meshes with averaged flat colours (barrel body #6E4525) | Source group forms: crates map-only `planks-crate` #B07A48 [0.5,0.5]; barrel body `#9A6337` × `planks-barrel`, bands regain metalness 0.4; roof map-only `planks-roof` | world.ts:215–216; shots.ts:181–182 |
| C8 | No roughness anywhere | Restored wherever the source sets it: dirt 1, stones/logs/poles 0.95, bedroll 1, palm bark 0.95, fronds/tips/coconuts 0.9, flames 1 | world.ts:44–49 rule |
| C9 | Flames baked at the t=1.5 flicker scale (1.13146/0.86854) regardless of file time | Set file: flames as built (scale 1). Shot files: bake = f(the file's own film time), f(t) = 1 + 0.16·sin13t + 0.07·sin(29t+1), inner = 2 − f | shots.ts:190–194 |

Application (W26): all 11 JG shot files carried the full ab1
form and received the identical node-level correction set
(per file: C1 ×21 fronds, C2 ×21 tips, C3 ×4 hills, C4 ×43
trunks, C5 ×1, C6 ×4, C7 regroups + 3 texture entries, C8 ×72
roughness additions — ×70 in s05a/s07, no flames). C9 verdicts:
**s01 was the only blind carry** — re-baked to f(0) =
1.0589029788970947 / 0.9410970211029053; the other nine flame
files verified against their own times unchanged; s05a/s07
n/a (no flame nodes). SH files: s02 already correct on every
checked class; **s03 got C4** (31 trunks 7 → 12). Post-edit
re-audit: zero remaining C1–C8 violations in the JG files; all
files re-parse.

Reconciliation (W19, rulings R1/R2 on Batch-1 files): s04 —
hemisphere #DCECD2/#46603A ×1.15 → **#CFE6FF/#C48A5A ×1.25**,
fire point 5.0 → **4.6** + the `fireLevel` channel, rifle moved
from top-level onto the character node; s05b, s02 — attachments
moved on-node (lights verified already post-atmosphere);
s01/s03/s06/s07/s05a verified compliant untouched.

Caption ownership (W27, manifest start-time rule): chunk 31
removed from s06 (owned by s05b; bleed note kept), chunk 39
removed from s07 (owned by s06; bleed note kept). Chunks 46/56
remain — see follow-ups.

**Follow-up fixes — scoped, not yet landed at this writing**
(the Batch-4 seats W31/W32 in the wave plan):
1. The **69 rounded computed values** (§1) rewritten at full
   f32-exact precision, per file/field as listed in
   `tools/film/VALIDATION.md`.
2. Caption chunks **46** (drop from s08b) and **56** (drop from
   s11) — the same start-time fix W27 applied to 31/39.
   Items 1+2 together take the validator to 7/7.
3. SH C6/C7 completion in s02/s03: hills regain the
   `grass-hills` map; crates/barrel take the shore set's
   planks group forms (observed by W26 in both files, outside
   its tasked scope).

---

## 5. film.rs-vs-source divergence register

Every staging divergence the seats flagged between the lead's
hand-written evaluator (`src/film.rs`) and the project source
(shots.ts/world.ts/env.ts/toon.ts). Per R4 the source is ground
truth. Statuses re-verified at HEAD `311656d` where marked ✓.
The data files follow the source in every case.

### Fixed

| # | Divergence | Fixed by |
|---|---|---|
| D1 | S06 companion **walkBob amplitude 6.0**; source `walkBob(ph, 5)` (shots.ts:413) | Lead, `5e5b0a0` (R4) ✓ |
| D2 | S06 **POSES.mourn** lean 5 / headTilt −4 / bob 4 (+ arm override) applied only standing; source merges the pose in **both** standing and sitting branches | Lead, `5e5b0a0` ✓ |
| D3 | (Engine, not staging) loader ignored character-node `attachments` | Lead, `4c06a1e` — loader consumes them, hand-anchored ✓ |
| D4 | (Data) lead's `character-v2.scene.json` example carried pre-atmosphere lights + no attachments | Lead, `4c06a1e` ✓ |
| D5 | (Engine) film frame count off by one (1812) | Lead, `4c06a1e` — 1811 exact, `ceil(duration·30)` ✓ |

### Open — in film.rs at HEAD

| # | Divergence (film.rs → source) | Citation | Flagged by |
|---|---|---|---|
| D6 | **S02 march clamp**: evaluator clamps the driver `kk = min(k, 0.97)` (film.rs:1404), halting the march when the legs stand; source keeps `travelled = k·6.8` for the whole shot (final 3% slides 0.204 m with planted feet — staged truth) | shots.ts:333–336 | W04 |
| D7 | **S03 rifle drawn**: `char(..., SH_ONODA, ..., true)` shows the rifle; source stages S03 with the rifle hidden (shStage without the rifle flag) | shots.ts S03 draw | W05 |
| D8 | **Shore tree rotation**: `build_sh` rotates shore trees `i·1.7` — that is the JG ring's rotation; the SH loop sets none | shots.ts:210 vs :151–155 | W05, W04 |
| D9 | **Shore tree kinds swapped**: `build_sh` passes `round = i % 2 == 1` (film.rs:961); source: even = round, odd = cypress | shots.ts:210 | W04 |
| D10 | **Palm frond parity inverted** in `add_palm` (even → leaf; source odd → leaf), tips take the frond's colour (source: opposite) at 10×6 (source 8×5) | shots.ts:93–95 (`palm()`) | W10 (G8), W05, W18 (C1/C2) |
| D11 | **Shelf-unit construction** differs on 8 points: upright inset ±(w/2−0.03) vs ±w/2; boards (w−0.12, 0.04, 0.3) vs (w, 0.05, 0.32); bottom depth 0.34 vs 0.3; book y +0.02 vs +0.03; advance bw+0.005 vs +0.012; stop w/2−0.14 vs −0.16; **gap predicate inverted** — port places a book when `rnd ≤ 0.86`, source when `rnd > 0.14` (film.rs:1006) — different books/gaps wherever the hash falls outside (0.14, 0.86] | shots.ts:105–121 | W14 |
| D12 | **S10 legs**: port sets Walk whenever wk < 1, including wk = 0; source passes 'stand' until wk > 0 (film.rs:1819 — flagged, not re-verified at HEAD) | shots.ts:495 | W14 |
| D13 | **Shade colour-space family**: world.ts `shade()` is THREE.Color (linear-space multiply), but film.rs/scene.rs use byte-multiply `shade_rgb` / sRGB `shade_lin` for it — felt speckles all differ (e.g. #1F5A45 @0.85: three (28,83,64) vs byte (26,77,59)); BK ceiling motif renders #493A2B vs source **#413427** (film.rs:2325); sign board edge (41,29,29) vs source **#302222**; the registry's existing `paint_grass` shares the byte-multiply. The wave's new painters (planks/wallpaper/felt/sign) implement the linear semantics. **Integrator decision**: reconciling the older code changes already-measured A/B frames — deliberately not done by the seats | world.ts:35; M0 audit a1 warned of exactly this | W21, W22, W23, W24, W14 |
| D14 | **S11 Onoda2 lean eased** via poseTrack (film.rs:1893); source is a **step** at dropT + 0.55 | shots.ts S11 draw | W16 |
| D15 | **S11 arms**: walk-in instance left at FullState defaults [8,6]/[8,6] (source armR [16,−6], armL [10,5]); Onoda2 armL [8,6] vs source [8,4] (flagged, not re-verified at HEAD) | shots.ts S11 draw | W16 |
| D16 | **S11 blink**: evaluator omits Onoda2's `blinkAt(t, [wSurr])` term. **Benign** — its window ends 0.43 s before the swap and can never render; flagged for transcription completeness | shots.ts S11 draw | W16 |
| D17 | **Cloud tint** `#EFF4EA` not applied by `add_clouds` (sprites render untinted); also a format gap — billboard nodes carry no tint field (data-side J3/G5) | shots.ts:145, :202; world.ts:167–171 | W09, W18, W05 |

### Closed as equivalent / data-side

| # | Item | Disposition |
|---|---|---|
| D18 | film.rs `calendar_state` uses `local > 0.62` where the source uses `ph >= 0.62` — at the boundary u = 0 and year/sy/dark are identical either way | Equivalent (W25); module follows the source |
| D19 | **gen_assets.py plate** paints the calendar rule in INK `#1F1C33`; `drawCalendar` + the data spec use `#8B8378` | Open on the asset side; the W25 module and the data follow the source (shots.ts:295) |
| D20 | s05b/s07 `grade.time` = frame t in the data (25.31 / 30.39), pre-R3 | Inert — film.rs pins grade time 0 (R3); data hygiene fix only |

### Checked clean

S08, S08b and S09 were compared field-by-field against
shots.ts and agree with film.rs throughout (W11, W12, W13);
S11 agrees on everything except D14–D16 (rig_dusk, fog triple,
rifle drop law + geometry, Taniguchi tracks/talk, camera, post
— W16). The S11↔S12 rifle resting transform
([0.62, 0.07, −0.1], euler [0, −0.5, π/2]) was derived
independently by W16 and W17 from the source and agrees
exactly across the cut.

**Register count: 20 entries — 5 fixed (D1–D5), 12 open in
film.rs (D6–D17, of which D16 is benign by construction), 1
closed as equivalent (D18), 2 data/asset-side (D19, D20).** Of the open film.rs items, D16
is benign by construction and D13 is a held integrator
decision; the remaining 10 are straight staging corrections.

Also open on the engine side, from the fog package: the
**scene-file loader does not parse the `fog` block** the
s06/s07/s11/s12 files already carry (grep-verified at HEAD —
no fog handling in `src/scenefile.rs`); FOG.md §8 is the
wiring spec, no shader change needed. And the lead is
bisecting a **black-trapezoid artefact** in v2 scene-file
renders (TEMP diagnostics `12c7a5e`: diag variants without
character / without attachments) — unresolved at this writing.

---

## 6. Honest limits — integration phase

1. **Channel evaluation lives in film.rs, not the format.**
   The v2 still loader renders a character's static `state`
   at the file's `time` only (spec §14.3). Every motion
   formula — poseTrack, walkPhase/walkBob, blink directors,
   settle, talk, fire tracks, camK holds — exists in the
   scene files as `channels` data for the film evaluator.
   A loader-only still is a single staged instant, never a
   shot proof.
2. **The loader does not consume, yet:** the `fog` block
   (FOG.md §8); `material.map` on colour materials (sets G2 —
   the still loader has no map field; the film/PBR path does);
   `material.metalness` (G3, carried as data under the
   unknown-field law); multi-material `materials` arrays
   (bookshop B2 — picture frame, sign); texture `rotation`
   (B5 — film.rs emulates the BK floor's π/2 with a swapped
   repeat); per-node shadow flags (G6/J4 — hills' hill-trees
   cast in the data path where the source clears them);
   `post.grade` exposure (the loader's grade parser has no
   exposure field; S06's 0.96 and the dusk 0.97 live in the
   film PostState); the pose **STEP** construct (S09
   listen→think, S10 mood steps, S11 visibility windows are
   additive data, no v2 channel type); **attachment
   lifetime** (the S11 rifle leaving the hand is expressed as
   two character instances + visible windows — the format has
   no instance-swap or cast-identity construct).
3. **Set files are fragments.** The merge-before-load step
   (sets README proposal) is not built; today's shot files
   inline their sets by copy (which is why W26's corrections
   had to touch 13 files). The loader requires a `camera` in
   every file (scenefile.rs:768).
4. **Captions are data + PIL assets, not native typesetting.**
   Chunk text/timing are exact; widths are PIL-measured
   against the project's own Inter-Black (max Δ 0.15 px vs
   Skia on this film, no verdict flips — W02). The native
   side has no Inter measurer (contract §2.4, carried from
   M6); strips come from `film/gen_assets.py`.
5. **Sign font size is PIL-measured, not browser-proven**
   (gap B9): fs = 40 with 43 px of headroom makes a browser
   shrink step unlikely, not impossible.
6. **Painter fidelity classes (standing):** Skia analytic AA
   vs the house SDF/4×4-supersample coverage (edge bytes only;
   M4 battery-verified for strokes); `rnd()` sin-hash can
   differ ≤1 ulp between V8 and Rust libm (measured: ≤4 px on
   the felt pin); three's LinearToSRGB exponent is the
   truncated 0.41666, reproduced deliberately. Painter tests
   for planks/wallpaper/picture/sign are written but **not yet
   run** — no local cargo; CI verifies on integration (the
   standing CI-only law).
7. **This VM cannot render the PBR path locally:** llvmpipe's
   JIT segfaults on `shader_pbr` (Mesa 23.2/24.0/25.2 alike),
   so PBR/film renders run through the dispatch-only CI
   workflow on lavapipe (`5e5b0a0`). No performance claims
   exist or can be made from this VM; RADV behaviour on the
   user's PC (incl. the M3 fp16 PMREM rounding question) is
   unverified from here.
8. **Proof state of the film itself:** the film mode exists
   (`bc97ed5`) and the first CI render already taught one
   lesson (the `*.png` asset ignore, `b3f3c0a`), but at this
   writing there is **no completed full-film verdict in the
   record**: the 1,811-frame render, the `--film-telemetry`
   gate over the whole film, the per-shot A/B table (tool
   ready — `tools/ab/measure_ab.py` reproduces the committed
   milestone metrics digit-for-digit), the finished MP4's
   technical evidence, and the 14 golden mid-frames are the
   integration phase's remaining proof items. The fog chart
   (FOG.md §7) is likewise planned, not run.
9. **Skill wiring is specified, not landed:** TaleRender as
   an alternative capture backend at the frame(t) seam
   (A6/ADR-001) has its design in the audits and the
   integration README; the skill-side adapter in a work copy
   is Batch-4 scope. The Chrome path and the skill's golden
   law are untouched by everything above.
10. **Preserved source mismatches (do not "fix"):** S06's
    scene fog (#C9CFC2, 4.5, 20 — shots.ts:409) vs the
    character tint's fog colour (#E6E9EA, the preset's —
    FOG.md §6); S05b's leaflet drop completing 0.021 s after
    the shot's out-point, so the dropped leaflet never appears
    in S05b (staged truth, film.rs agrees); Taniguchi's S11 →
    S12 position change is a cut, not a walk; S12's Onoda2
    carries camPitch 0 because the staging never passes
    camPitchOf for him (an evaluator auto-applying it would
    diverge on every frame of S11/S12).

---

*Pointers: per-shot detail — `scenes/film/ASSUMPTIONS.md`;
set detail + merge proposal — `scenes/sets/README.md`;
capability inventory — `docs/integration/INVENTORY.md`;
validator run record — `tools/film/VALIDATION.md`; fog —
`shaders/fog/FOG.md`; golden policy — `docs/golden/NATIVE-GOLDEN.md`;
rulings — `docs/scene-format.md` §14.5.*
