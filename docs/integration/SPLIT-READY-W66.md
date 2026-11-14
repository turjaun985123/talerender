# SPLIT-READY-W66 — split-programme readiness re-check (Wave 2, Batch 7, seat W66)

**Verified against HEAD `4dd6530`** (the tree moved past the brief's
`3a3a427` before this seat ran: `b6ee9fb` record fold-in, then
`4dd6530` — the §15 fold-in + §15.10 reservation guard. That landing
is itself one of the changes re-checked below.) Method: blob hashes
and body extraction via `git show`, item census by `grep -n` over
both target files and all importers, workflow grep. **No existing
file was edited; nothing is committed by this seat.**

**Headline:** neither programme is blocked. Of W21's 10 steps, one
is **DONE** (step 5), nine are **READY-WITH-UPDATED-CITES**, none is
blocked and none is plain READY — the file moved under every step,
but in one clean, fully mapped drift pattern. Of W22's 13 steps,
ten are **READY** (the file is byte-identical to the blueprint's
read) and three are **READY-WITH-UPDATED-CITES** (gate mechanics
and importer cites only — the file itself never moved).

---

## 1. The two frozen artefacts — verified, not assumed

**`state_key` (W21 trap T2, the freeze that matters most):
BYTE-IDENTICAL.** The function body was extracted
(`awk '/^fn state_key/,/^}/'`) from `src/film.rs` at four commits —
`1a62161`, `86f0c81`, `ff52b3a`, `HEAD` — and is the same 25 lines
with the same md5 (`95918b9941313f895dbee7bf35c8d14`) at all four.
The exact-f32-bit key, its format string, and the 72.34 dB incident
comment survive every landing since the blueprint (gpucore 5/5,
errors step 2, F2, the guard) untouched. It has only moved:
blueprint cite 2132–2160 → **now at film.rs:2183**.

**`src/scene.rs`: the whole file is byte-identical to W22's read.**
Blob `c6072e334a44985700de137fac73418a1622f259` at `1a62161`,
`ff52b3a`, `d3def63`, `3a3a427`, `4dd6530`, and HEAD — its last
change is `0f6c033` (Wave 1). Every W22 in-file cite was
spot-verified exact at HEAD: `hex_rgb` 30, `Geo` 104, `Painter`
475, `paint_toon` 1016, `paint_grass` 1323, `ColourDraw` 1459,
`Value` 1539, `parse_json` 1575, painter banners
1795/2123/2335/2502/2629/2753, test modules at
1766/1960/2245/2428/2562/2694/2903 (json/planks/wallpaper/felt/
parcel/picture/sign). The blueprint's surface census (~46 fns /
10 types / 11 consts) vs a column-0 grep census now (53/11/12) is
a counting-method difference only — identical blob, identical
surface by construction.

## 2. film.rs — measured drift, one pattern

| Measure | W21 blueprint read | ff52b3a | HEAD `4dd6530` |
|---|---|---|---|
| Lines | 3,694 | 3,723 | **3,670** |
| `fn ` occurrences | — | 124 | **124** |
| Column-0 top-level items | ~190 (its method) | 172 | **172** |

No item has been added to or removed from film.rs since the
blueprint; `git diff ff52b3a..HEAD -- src/film.rs` is 67
insertions / 120 deletions, and the drift is a single wave with
three causes, in file order:

- **+19 from C1 onward** — `load_film_data` grew inside its own
  span (63–~120; §3a/§3b below). Everything from C2 to C10 sits
  exactly +19: `clamp` 106→**125**, `PointState` 243→**262**,
  `FrameSpec` 335→**354**, TEX consts 389–409→**408–428**,
  `fl` 415→**434**, `rgb_of` 487→**506**, `paint_planks`
  496→**515**, `t3` 654→**673**, `JgDyn` 864→**883**,
  `sub3` 1134→**1153**, `s01` 1395→**1414**, `eval_frame`
  1986→**2005**, `Assets` 2031→**2050**.
- **+51 by C11–C13** — `load_assets` grew (F2 loud fallbacks,
  §3c) and the PaintCache impl grew (step-5 wiring, §4):
  `Painted` 2081→**2112**, `CharInfo` 2085→**2116**,
  `PaintCache` 2104→**2137**, `resolve_chars` 2162→**2213**,
  `mb` 2277→**2328**, `Renderer` 2309→**2360**.
- **−33/−25 in the tail** — gpucore shrank `Renderer::new`
  (§3d): `Prepared` 2791→**2785**, `sample_bilinear`
  3334→**3301**, `overlay_calendar` 3370→**3337**,
  `run_film` 3454→**3421**, `tel_look` 3509→**3484**,
  `run_film_telemetry` 3570→**3545**.

The §3 crossing census (≈82 in-subtree names) and the §4
visibility delta (**exactly 5 widenings**) were re-verified, not
extrapolated: `Painted` (2112), `PaintCache::new` (in impl at
2137), `resolve_chars` (2213), `overlay_calendar` (3337),
`overlay_caption` (3387) and `apply_fade` (3408) are **all still
private today** — the delta list is unchanged, item for item.

## 3. New cross-references the blueprints do not know about

**(a) W21 §4's adapter paragraph is VOID — strike it before
executing.** The blueprint instructs: "`crate::request_adapter`
(main.rs:301) stays the renderer's adapter source — gpucore.rs's
duplicate is a different wiring track; do not re-point during the
split." The re-point already happened outside the split
(`aadc193`, gpucore site 5/5): main.rs's helper is **deleted**,
and film.rs:2407 now calls `crate::gpucore::request_adapter`
(as do main.rs:466, scenefile.rs:1169, ab1.rs:587). An executor
following the blueprint literally would try to preserve a helper
that no longer exists. The split needs no adapter decision at
all. (Residue: gpucore.rs:14's header comment still says
"already shared by every site via crate::request_adapter" —
stale comment, not split business.)

**(b) F1 + the §15 reservation guard live inside C1.**
`load_film_data` (film.rs:63) no longer "reads `onoda.json`": it
prefers `film.json`, falls back to `onoda.json` (`aadc193`), and
since `4dd6530` hard-errors on top-level `prefabs`/`clips` with
the §15.1 messages ("prefab extension not supported" / "clips
extension not supported"). Those strings are format-contract
text — W21 step 2 moves them verbatim or not at all. Note the
scheduling consequence: §15.10 adoption step 2 (the evaluator)
is still to come and will touch this same function; if it lands
before W21 step 2, step 2's cites shift again — re-grep, don't
re-plan (the cluster is unchanged).

**(c) F2's first landing rewrote C10's behaviour, not its
shape.** `load_assets` (2064–2110) now errors loudly where the
blueprint's read silently defaulted: non-object plates.json →
Err, missing/malformed plate origin → Err naming the plate, and
an empty `captions[].strip` → Err naming the caption index
(`a2ec80c`). Struct `Assets` and the load set are unchanged, so
no step's move set changes; but W53's staging point stands in
the code: the calendar year list exists exactly once as strings
(film.rs:2100) while the **count** is independently hardcoded
in `calendar_state` (`let years = 4;`, film.rs:~1682, staging
cluster) plus the mapping comment at film.rs:348 (frame-contract
cluster). The F2 manifest-isation, when the lead lands it, will
touch C1/C10 data paths and this staging count — another reason
it must not interleave with split steps (§6).

**(d) Errors step 2 changed C13's signatures.** `Renderer::new`
(film.rs:2405) returns `crate::errors::TaleResult<Renderer>`
with the `Ok(Renderer { … })` wrap at 2739; `render` (2817)
returns `TaleResult<Vec<u8>>` with `Ok(pixels)` at 3291
(`edb058f` + the `aa92cee` wrap fix). `run_film` already
returned TaleResult in the blueprint's read; now the renderer
does too. Pure-move impact: none — signatures move as-is — but
C13's blueprint description ("437-line new, 510-line render")
is stale: new is now ~380 lines (2405–2784), render ~475
(2817–3291), because gpucore absorbed the adapter/target/
readback setup. `textures_px` is at 2419 (blueprint 2390–2412)
and trap T3's env read `TALERENDER_DOF_DEPTH_FORMAT` at 2556
(blueprint 2527) — both intact, both moved.

**(e) Three new crate-level edges hang off film clusters.**
The blueprint's DAG drew only in-subtree edges; the landings
added leaf-service edges the executor will meet while moving:
`film::chars` → `crate::paintbudget` (the `PaintCache.budget`
field, film.rs:2140), `film::driver` → `crate::runlog`
(`RunClock::new()` at 3436, inside `run_film`), `film::render`
→ `crate::gpucore` (2407 + target/readback calls). None crosses
the film subtree, none adds a widening — each is one `use` line
in the child that owns the cluster. Recorded here so no step
"discovers" them mid-move and stops under the blueprint's
if-it-needs-a-non-move-edit rule: these imports are already in
the code being moved, so carrying them is part of the move.

**(f) G-TEL's quoted expectation is stale in every step that
cites it (W21 steps 1, 2, 4, 7, 10).** The blueprint's gate text
expects the line `FILM TELEMETRY: CHECK FAILED (0 overflow
groups, 0 continuity violations, 7 caption failures)` and exit
code 1. That was the pre-re-chunk record. Since `5e90d7e` the
standing verdict is **`FILM TELEMETRY: CHECK OK`, exit 0**
(verdict construction unchanged in form: film.rs:3653/3656;
exit wiring in main.rs — Ok(true)→0, Ok(false)→1, Err→3).
Every G-TEL gate in the programme must be re-worded before use:
*reproduce the current verdict verbatim (CHECK OK, exit 0);
any change in either direction = stop.* A gate left as written
would report a healthy tree as failed — or worse, train an
executor to "fix" the tree back to 7 caption failures.

**(g) Small print.** Trap T7: `.expect("cache order")` is now
**two** sites inside `get_or_paint` (pop_front + map.remove,
~2172–2173), not one; `.expect("pixel buffer size mismatch")`
is at film.rs:3456 (blueprint 3497), still in the driver.
Trap T4 verified intact: both shot tables survive with the
same word-anchored offsets — eval_frame dispatch at
2014/2016/2018 (`thought4 −0.12`, `decades6 −0.25`,
`refused7 −0.1`) and telemetry's `ShotInfo` table at
3553–3562 (same offsets at 3558/3560). main.rs's only film
call sites: blueprint 253/267 → **now 272**
(`run_film_telemetry`) and **286** (`run_film`); the mod list
has grown around them (`paintbudget` main.rs:29, `runlog` 31,
`film` 35), which is all that moved them.

## 4. W21 step 5 — DONE, verified item by item

The blueprint's §6 four wiring items, checked against the
current `PaintCache` (film.rs:2137–2181) and its callers:

1. `mod paintbudget;` registered — main.rs:29 (`2328e6f`). ✔
2. `budget: PaintBudget` field constructed **once** in
   `PaintCache::new` via `PaintBudget::from_env(cap,
   CANVAS_BYTES)` (2140, 2155–2158); the entry cap is resolved
   once in `new()` (2149–2152) and the per-miss env parse is
   **gone** — `get_or_paint` contains no env read. ✔
3. `get_or_paint` reports `on_hit` / `on_miss` / `on_insert` /
   `on_evict` (2164–2175). One shape difference from §6's
   sketch: the eviction loop is `while
   self.budget.should_evict()` alone — the entry cap is
   internalised in `PaintBudget`, where the blueprint sketched
   `while map.len() > cap || budget.should_evict()`. Same
   contract, tidier form; noted so nobody "restores" the
   sketched form. ✔
4. `stats_line()` is printed by **both** run paths: driver at
   film.rs:3469, telemetry at 3612 (via the `PaintCache`
   wrapper at 2178–2179). RunLog's `RunClock` rides along in
   the driver (3436) from the same commit (`86f0c81`; render.yml
   gained the `paint_budget_mb` dispatch input at `fbf37c0`). ✔

The step's gates were discharged at landing (probes +
telemetry per the `86f0c81` record). The executor should
**skip step 5 entirely** — and note that step 4's framing
("pure move only; budget wiring is explicitly not in this
step") is now moot in the best way: the wiring is *inside* the
code step 4 moves, so step 4 is again a pure move, of slightly
more code.

## 5. W21 — step-by-step readiness

| Step | Extract | Status | Cite updates (blueprint → HEAD) + notes |
|---|---|---|---|
| 1 | `film::motion` | READY-WITH-UPDATED-CITES | Cluster 106–238 → **125–~257**: `pose_track` 150→**169**, `walk_phase` 169→**188**, `Blinker` 175→**194**, `talk` 223→**242**, `mix3` 230→**249**. All items still `pub(crate)` (spot-verified), zero visibility delta — unchanged. Re-word G-TEL per §3f. |
| 2 | `film::data` | READY-WITH-UPDATED-CITES | Item anchors unmoved (`FilmData` 44, `Caption` 51, `jnum` 59, `load_film_data` 63) but the cluster's content changed (§3b): film.json preference + §15 guard are part of the moved code; guard strings move verbatim. Re-word G-TEL. Scheduling note §3b (§15 step 2 may re-shift this one function). |
| 3 | `film::paint` | READY-WITH-UPDATED-CITES | `fl` 415→**434**, `look_registry` 434→**453**, painters +19 (`paint_planks` 496→**515**). Consumer-side cite: render's `textures_px` 2390→**2419**. Gates unchanged (S10 + S01 pairs). |
| 4 | `film::chars` | READY-WITH-UPDATED-CITES | `Assets`→step-2-side anchors in §2; cluster items: `Painted` 2081→**2112**, `CharInfo` 2085→**2116**, `PaintCache` 2104→**2137**, `state_key` 2132–2160→**2183** (frozen, §1), `resolve_chars` 2162→**2213**. Widenings #1–#3 still exactly as §4 lists. Carries the new `crate::paintbudget` edge (§3e) as an existing import-in-code. Re-word G-TEL. |
| 5 | Paint-budget wiring | **DONE** | Landed as `86f0c81` (+`2328e6f`, `fbf37c0`); verified §4 above. Remove from the programme. |
| 6 | `film::sets` | READY-WITH-UPDATED-CITES | `t3` 654→**673**, `add_clouds` 790→**809** (stays behind for step 7, as designed), `add_sky` 825→**844**, `JgDyn` 864→**883**, `build_jg` 870→**889**, `shelf_unit` 1004→**1023**, `build_bk` 1037→**1056**. Gates unchanged. |
| 7 | `film::staging` | READY-WITH-UPDATED-CITES | All +19: `sub3`-companions stay in parent as designed; `fold` 1148→**1167**, `rig_noon` 1191→**1210**, `finish_chars` 1352→**1371**, `char` 1382→**1401**, `blink_of` 1391→**1410**, `s01` 1395→**1414**, `plane_extras` 1519→**1538**, `s05a` 1561→**1580**, `calendar_state` 1659→**1678**, `s07` 1680→**1699**, `s12` 1946→**1965**, `eval_frame` 1986→**2005**. T4 dispatch offsets verified (§3g). `calendar_state`'s hardcoded `years = 4` (§3c) moves as-is — dedup is F2 business, not split business. Re-word G-TEL. |
| 8 | `film::render` | READY-WITH-UPDATED-CITES | `Renderer` 2309→**2360**, `new` 2353→**2405** (now `TaleResult<Renderer>`, §3d), `textures_px` 2390→**2419**, DOF env read 2527→**2556**, `Prepared` 2791→**2785**, `render` 2823→**2817** (now `TaleResult<Vec<u8>>`). **§4's adapter paragraph is void (§3a)** — the gpucore edge is already in the code and moves with it. Function sizes: new ~437→~380 lines, render ~510→~475. |
| 9 | `film::overlay` | READY-WITH-UPDATED-CITES | All −33: `sample_bilinear` 3334→**3301**, `overlay_calendar` 3370→**3337**, `overlay_caption` 3420→**3387**, `apply_fade` 3441→**3408**. Widenings #4–#5 still exact. Driver call order verified unchanged in `run_film`: calendar → fade → caption. Gate (byte-compare) unchanged. |
| 10 | `film::telemetry` + `film::driver` | READY-WITH-UPDATED-CITES | `run_film` 3454→**3421** (carries `RunClock` 3436 + stats print 3469, §3e), `tel_look` 3509→**3484**, `INSTANCE_IDS` 3566→**3541**, `run_film_telemetry` 3570→**3545** (still `Result<bool, String>`; exit mapping in main.rs 266–281). External cites: main.rs 253→**272**, 267→**286**. G-TEL re-worded per §3f — this step's gate is the contract both ways: CHECK OK + exit 0. |

## 6. W22 — step-by-step readiness

The file never moved (§1), so readiness turns on the importer
cites and gate mechanics around it. Importer set is unchanged —
exactly the same seven consumers (`scenefile`, `ab1`, `film`,
`toonfull`, `battery`, `telemetry`, `dof`); no Wave-2 module
(`anim`, `charanim`, `atlas`, `composer`, `rendergraph`,
`gpucore`, …) has grown a `crate::scene` import. Use-block
cites are all exact at HEAD: scenefile.rs:26, ab1.rs:27,
film.rs:33, toonfull.rs:26, battery.rs:15. `toonfull`'s
`rounded_rect_pts` (716) and ab1's qualified `paint_toon`
(44–49) and telemetry's qualified block (1219–1323) are exact.
Two qualified-call cites drifted in files that changed for
other reasons: scenefile's `scene::parse_json` 826→**836** and
`resolve_texture` ~420→**408** (§15-era growth), and dof's
`chart_texture_rgba` 207→**173** (gpucore shrank dof.rs).
Neither gates any move — re-exports preserve every path — but
the executor's map should carry the new numbers.

| Step | Extract | Status | Notes |
|---|---|---|---|
| 1 | `json.rs` | READY | Cites exact (Value 1539, parse_json 1575, json_tests 1766). Gate (CI green + test count) mechanics unchanged; ci.yml runs `cargo build` + `cargo test --release` (ci.yml:61/64). |
| 2 | `colour.rs` | READY-WITH-UPDATED-CITES | In-file cites exact. Gate cite updated: the battery is **not** invoked by ci.yml (grep-verified — build + test only), so the 46/46 leg needs a render.yml dispatch, exactly the confirmation W22's trap 6 demanded. Precondition, not a blocker: confirm the dispatch path once before this step. |
| 3 | `geo.rs` | READY | Cites exact (Geo 104, ColourDraw 1459). Gates (s28 pair, ab1-as-data) unchanged — and note both pins were re-struck in Wave 2 (`916a109`/`166873c`), so the gate evidence is fresher than at blueprint time. |
| 4 | `raster.rs` | READY-WITH-UPDATED-CITES | In-file cites exact (Painter 475, SDF block 803–917). Same battery-dispatch note as step 2; the M6 spot check is telemetry-side and unaffected. |
| 5 | `toon.rs` | READY-WITH-UPDATED-CITES | In-file cites exact (paint_toon 1016). Same battery-dispatch note as step 2. |
| 6 | `textures.rs` | READY | Cites exact (paint_grass 1323, chart fns 1496/1515). fx byte-identity + dof-chart gates unchanged. |
| 7–12 | `painters/{planks,wallpaper,felt,parcel,picture,sign}.rs` | READY (each) | Banners and test modules at the exact blueprint lines (1795/1960, 2123/2245, 2335/2428, 2502/2562, 2629/2694, 2753/2903). Trap 3's deliberate colour inconsistencies are in the unmoved file, verbatim. Order among the six still arbitrary; sign last, as written. |
| 13 | `scene/mod.rs` slim-down + staging decision | READY-WITH-UPDATED-CITES | Importer cites updated per the preamble above (826→836, 207→173). **The staging-dir decision is still open**: `painters/` is on disk, unmarked — INDEX.md carries no superseded banner — so step 13 still owes the delete-or-banner call, exactly as W22 §6.5 left it. The step-13 grep proof (seven importers, 0-line diff across the series) is measurable only under serial execution (§7). |

## 7. Serial order — re-derived, and the updated risk total

The Batch-3 ruling (recorded in WAVE2-RECORD: "split
programmes noted SERIAL (film.rs then scene.rs)") **stands,
and the reasons have narrowed to two that still bind:**

1. **Gate attribution.** Both programmes prove pure moves
   through the same CI + render-dispatch evidence. Interleaved,
   a red run has two suspects; serial, it has one, and the
   failing step is findable by diffing moved text (W22 §5's
   rule). This alone justifies serial.
2. **W22 step 13's proof window.** "No importer file modified
   across the whole series" is only measurable while W21 is
   not editing film.rs (a scene importer). Either programme
   may go first *if* it runs to completion before the other
   starts; the recorded film-first order remains the right
   one, because film.rs is still receiving lead landings
   (§15 step 2, F2 manifest-isation) that would re-drift
   W22's importer neighbourhood mid-series, while scene.rs
   has been frozen since Wave 1.

What no longer binds: the old worry that the two programmes
share a moving file surface. W22 needs zero importer edits by
design, and W21's moves do not rename any `scene::` import
(film.rs:33 use block is content-identical at HEAD). The
programmes are coupled by evidence hygiene, not by code.

**Updated totals.** W21: 10 steps → **9 remaining commits**
(step 5 discharged as a landing, §4), largest single move
unchanged in kind (staging ~925 lines, render now ~1,000 with
its two TaleResult signatures). W22: **13 commits**, all
against a frozen file. Combined remaining programme: **22
pure-move commits + W22 step 13's decision**, visibility
delta still exactly 5 (all in W21), behaviour delta still
zero — with two executor warnings that did not exist when
the blueprints were written: **strike W21 §4's adapter
paragraph (§3a), and re-word every G-TEL gate to the CHECK OK
contract (§3f) before step 1 runs.** A programme executed
from the blueprints unamended would not fail to compile — it
would fail its own gates and chase a helper that no longer
exists. With those two amendments and the cite tables above,
both blueprints are executable as written.

— W66
