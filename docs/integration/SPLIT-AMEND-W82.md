# SPLIT-AMEND-W82 — split programmes: amended gate pack (Wave 2, Batch 9, seat W82)

**Verified against HEAD `c440d90`** (one lead commit past the brief's
`021ca01`: `c440d90`, an ab1-only delimiter fix — it touches no cite
in this pack). Method: full re-read of both blueprints and
SPLIT-READY-W66, grep census of every cited item in `src/film.rs`
at HEAD, `state_key` body extraction + md5, blob hash of
`src/scene.rs`, workflow greps. **No existing file was edited;
nothing is committed by this seat.** This pack is amendment *text*
for the lead to apply to FILM-SPLIT-W21.md and SCENE-SPLIT-W22.md
before executing either programme.

**What this pack does:** (§1) re-words every G-TEL gate to the
standing CHECK OK contract — old and new text side by side, every
occurrence listed; (§2) strikes W21 §4's void adapter paragraph and
supplies its replacement note; (§3) records one W66 note superseded
since its run (calendar_state); (§4) refreshes every drifted cite
in W66's step tables against HEAD; (§5) the execution protocol —
22 pure-move commits, per-step CI watch, the still-open battery
precondition for W22 steps 2/4/5, the abort rule; (§6) W22 step
13's staging-dir decision framed with both options costed.

**Delta since W66 (`4dd6530` → `c440d90`):** F2 migration landed
(`20a81a7`/`2c41313`), blink adoption landed (`5a9383e`), knobs
phases 1–3 landed (`ea98458`/`9ad2177`/`76b922c`) plus the knobs
gate workflow (`2017e86`), record fold-ins, and the ab1 delimiter
fix. `src/film.rs`: 3,670 → **3,761 lines**; `fn ` occurrences
**124, unchanged**; no item added or removed. `src/scene.rs`:
blob `c6072e334a44985700de137fac73418a1622f259` — **still
byte-identical** to W22's read and to W66's check.

---

## 1. Amendment A — G-TEL re-worded everywhere it appears

### 1.1 Occurrence census (complete, grep-verified)

**FILM-SPLIT-W21.md:**

| # | Location | Form | Action |
|---|---|---|---|
| A1 | §5, gate vocabulary, lines 354–358 | The G-TEL **definition** — quotes the CHECK FAILED line + exit 1 | **REPLACE** (§1.2) |
| A2 | §5, step-10 row, line 389 | Inline: "line verbatim + exit 1" | **REPLACE phrase** (§1.3) |
| A3 | §5, step rows 1 (line 380), 2 (381), 4 (383), 7 (386); trap T4 (line 484); §6 gates (lines 444–446) | Cite G-TEL **by name only** | No text change — they inherit A1's new definition. §6's gates are additionally **moot**: step 5 is DONE (W66 §4), its gates were discharged at `86f0c81` |
| A4 | §5, step-9 row, line 388 | Negative mention: "G-TEL is not evidence here" | No change — true under both wordings |
| A5 | §1, cluster C16, line 164 | Descriptive: "verdict line `FILM TELEMETRY: CHECK OK / CHECK FAILED (…)`" | No change — this is the code's format, not a gate expectation |

**SCENE-SPLIT-W22.md:** W22 defines no G-TEL. Its only
verdict-line gate is:

| # | Location | Form | Action |
|---|---|---|---|
| A6 | §5, step-13 row, line 343 | "film telemetry verdict line unchanged" — "unchanged" is unpinned: read against the blueprint-era record it means the CHECK FAILED line | **REPLACE phrase** (§1.4) |
| A7 | §5, step-4 row, line 339 | "M6 telemetry exact-agreement spot check" | No change — this is `src/telemetry.rs` cutout agreement vs Chrome (M6), **not** the film verdict line. Do not conflate them when applying A6 |

### 1.2 A1 — the definition (W21 §5, lines 354–358)

**OLD:**

> - **G-TEL** — `--film-telemetry` verdict line reproduced
>   **verbatim**: `FILM TELEMETRY: CHECK FAILED (0 overflow groups,
>   0 continuity violations, 7 caption failures)` (the standing,
>   waiver-covered record) and exit code 1. Counts moving in *either*
>   direction = stop and investigate; "improvement" is also a
>   regression signal in a pure move.

**NEW:**

> - **G-TEL** — `--film-telemetry` verdict line reproduced
>   **verbatim**: `FILM TELEMETRY: CHECK OK` (the standing record
>   since the caption re-chunk, `5e90d7e`; the pre-re-chunk
>   CHECK FAILED line with 7 caption failures and its waiver are
>   history, superseded by the Wave-2 re-chunk) and exit code 0.
>   Exit wiring, unchanged in form: main.rs maps `Ok(true)` → 0,
>   `Ok(false)` → 1, `Err` → 3. The verdict moving in *either*
>   direction = stop and investigate; a return to CHECK FAILED
>   is a regression signal in a pure move, exactly as an
>   unexpected "improvement" was under the old wording.

### 1.3 A2 — step-10 row phrase (W21 §5, line 389)

**OLD:** `G-TEL (both directions of the contract: line verbatim + exit 1)`

**NEW:** `G-TEL (both halves of the contract: line verbatim —` `FILM TELEMETRY: CHECK OK` `— + exit 0)`

### 1.4 A6 — step-13 gate phrase (W22 §5, line 343)

**OLD:** `film telemetry verdict line unchanged`

**NEW:** `film telemetry verdict reproduced verbatim (`FILM TELEMETRY: CHECK OK`, exit 0 — the standing post-re-chunk verdict; "unchanged" is measured against this line, not the blueprint-era CHECK FAILED record)`

### 1.5 Why this is load-bearing, restated at HEAD

The verdict construction is at film.rs:3744
(`FILM TELEMETRY: CHECK OK`) / 3747 (the FAILED format string);
the exit wiring is main.rs:397–404. An executor running the
unamended gates would watch a healthy tree "fail" G-TEL at W21
step 1 and — worse — could read the old text as licence to
restore the 7 caption failures. The re-chunk data and the
CHECK OK verdict are the contract now; the W92 waiver covers a
verdict that no longer occurs.


---

## 2. Amendment B — W21 §4's adapter paragraph is struck

### 2.1 The text to strike (W21 §4, end of the "Path preservation"
paragraph, lines 330–339; and the §1 C13 region-map phrase, line 132)

**OLD (§4):**

> `crate::request_adapter` (main.rs:301) stays the renderer's
> adapter source — gpucore.rs's duplicate is a different wiring
> track (GPU-CORE-W07); do not re-point during the split.

**OLD (§1, line 132):**

> — adapter via `crate::request_adapter`, static texture table

### 2.2 Replacement note (paste in §4 where the struck text stood)

> **Adapter (amended, SPLIT-AMEND-W82):** there is no adapter
> decision in this split. The re-point the original text forbade
> already happened outside the split (`aadc193`, gpucore site
> 5/5): main.rs's `request_adapter` helper is **deleted**, the
> single implementation is `crate::gpucore::request_adapter`
> (gpucore.rs:63), and every site calls it — film.rs:2474 (inside
> `Renderer::new`), main.rs:603, scenefile.rs:1169, ab1.rs:594,
> dof.rs:143. The call inside `Renderer::new` is ordinary moved
> code in step 8; it moves verbatim like everything else.
> (Residue, not split business: gpucore.rs:14's header comment
> still credits `crate::request_adapter` — a stale comment in a
> file this programme does not touch.)

The §1 line-132 phrase is corrected in place to:
"— adapter via `crate::gpucore::request_adapter`, static texture table".

**Verified at HEAD:** `grep -rn request_adapter src/` returns the
gpucore definition, its five call sites above, the stale comment,
and errors.rs doc/test mentions — no `crate::request_adapter`
definition or call exists anywhere.

---

## 3. Amendment C — one W66 note is superseded: calendar_state

W66 §3c and its §5 step-7 row said: "`calendar_state`'s hardcoded
`years = 4` moves as-is — dedup is F2 business, not split
business." **That is no longer true and the note must not be
carried into execution.** The F2 migration landed after W66's
run (`2c41313`): at HEAD, `calendar_state` is
`fn calendar_state(lt_cal: f64, years: usize)` (film.rs:1711)
with the zero-years guard at 1715 (the `years - 1` usize
underflow the guard exists for), called at 1779 with
`data.calendar_years.len()`; the years themselves are film data
(`FilmData.calendar_years`, film.rs:51, parsed in
`load_film_data` at 120). Step 7 moves the data-driven form
exactly as it stands; **no dedup is owed by anyone**, and an
executor hunting for a `let years = 4` to preserve will not find
one. Nothing else in W66 §3 is superseded: §3b's guard strings
(§15.1, inside `load_film_data`) and §3d's TaleResult signatures
are as W66 described, at the refreshed cites in §4 below.

---

## 4. Cite refresh — W66's tables re-checked at HEAD `c440d90`

Only cites that moved again are listed with new numbers; the
drift since W66 is one pattern (F2 growth in the data/assets
region, blink adoption in staging, knobs plumbing in render and
overlays). Anchors are grep-verified at HEAD; spans derived from
adjacent anchors are marked ≈.

### 4.1 W21 (film.rs) — blueprint → W66 → HEAD

| Step | Item | Blueprint | W66 | HEAD |
|---|---|---|---|---|
| 1 | `clamp` / `pose_track` / `walk_phase` / `Blinker` / `talk` / `mix3` | 106 / 150 / 169 / 175 / 223 / 230 | 125 / 169 / 188 / 194 / 242 / 249 | **155 / 199 / 218 / 227 / 275 / 282** |
| 2 | `FilmData` / `Caption` / `jnum` / `load_film_data` | 44 / 51 / 59 / 63 | 44 / 51 / 59 / 63 | **44 / 56 / 64 / 68** |
| 3 | `fl` / `look_registry` / `paint_planks` | 415 / 434 / 496 | 434 / 453 / 515 | **467 / 486 / 548** |
| 4 | `Assets` / `load_assets` | 2031 / 2045 | 2050 / — | **2088 / 2102** |
| 4 | `Painted` / `CharInfo` / `PaintCache` / `resolve_chars` | 2081 / 2085 / 2104 / 2162 | 2112 / 2116 / 2137 / 2213 | **2175 / 2179 / 2200 / 2276** |
| 4 | `state_key` | 2132–2160 | 2183 | **2246 — md5 `95918b9941313f895dbee7bf35c8d14`, 25 lines, BYTE-IDENTICAL** (re-extracted at HEAD; the freeze holds through gpucore, errors 2, F2, blink, knobs) |
| 5 | DONE — cites historical only | — | `PaintCache` 2137–2181; stats prints 3469 / 3612 | `PaintCache` impl 2206, `get_or_paint` 2224, `stats_line` wrapper 2241; prints: driver **3560**, telemetry **3703** |
| 6 | `t3` / `add_clouds` / `add_sky` / `JgDyn` / `build_jg` / `shelf_unit` / `build_bk` | 654 / 790 / 825 / 864 / 870 / 1004 / 1037 | 673 / — / 844 / 883 / 889 / 1023 / 1056 | **706 / 842 / 877 / 916 / 922 / 1056 / 1089** |
| 7 | `fold` / `rig_noon` / `finish_chars` / `char` / `blink_of` / `s01` | 1148 / 1191 / 1352 / 1382 / 1391 / 1395 | 1167 / 1210 / 1371 / 1401 / 1410 / 1414 | **1200 / 1243 / 1404 / 1434 / 1443 / 1447** (`blink_of`'s body is the post-adoption form, `5a9383e` — it moves as-is) |
| 7 | `plane_extras` / `s05a` / `calendar_state` / `s07` / `s12` / `eval_frame` | 1519 / 1561 / 1659 / 1680 / 1946 / 1986 | 1538 / 1580 / 1678 / 1699 / 1965 / 2005 | **1571 / 1613 / 1711 / 1737 / 2003 / 2043** (`calendar_state`: signature changed, §3) |
| 8 | `Renderer` / `new` / adapter call / `textures_px` / DOF env read / `Prepared` / `render` | 2309 / 2353 / — / 2390 / 2527 / 2791 / 2823 | 2360 / 2405 / 2407 / 2419 / 2556 / 2785 / 2817 | **2423 / 2470 / 2474 / 2486 / 2623 / 2854 / 2886** (`new` now takes `knobs: crate::Knobs` as a fourth parameter — a knobs-phase signature, part of the moved code) |
| 9 | `sample_bilinear` / `overlay_calendar` / `overlay_caption` / `apply_fade` | 3334 / 3370 / 3420 / 3441 | 3301 / 3337 / 3387 / 3408 | **3378 / 3414 / 3469 / 3494** (overlay bodies carry knobs phase 1's factor-last scaling, `ea98458` — moved code, not a split edit) |
| 10 | `run_film` / `tel_look` / `INSTANCE_IDS` / `run_film_telemetry` | 3454 / 3509 / 3566 / 3570 | 3421 / 3484 / 3541 / 3545 | **3507 / 3575 / 3632 / 3636** (`run_film` also takes `knobs: crate::Knobs`) |
| 10 | verdict construction / exit wiring / main.rs call sites | — / — / 253, 267 | 3653, 3656 / main 266–281 / 272, 286 | **3744 (OK), 3747 (FAILED format) / main.rs:397–404 / 397 (telemetry), 418 (run_film)** |

The §4 visibility delta is unchanged: the five widening items
sit at `Painted` 2175, `PaintCache::new` (impl at 2206),
`resolve_chars` 2276, `overlay_calendar` 3414,
`overlay_caption` 3469 + `apply_fade` 3494 — grep-verified still
private at HEAD where the surface shows it; the compiler remains
the final check at each step, as W21's design intends.

### 4.2 W22 (scene.rs) — almost nothing to refresh

`src/scene.rs` blob at HEAD is `c6072e33…`, identical to W22's
read: **every in-file cite in W22 is exact as printed** (clusters,
banners 1795/2123/2335/2502/2629/2753, test modules
1766/1960/2245/2428/2562/2694/2903). Importer cites: the five
use blocks are exact at HEAD — film.rs:33, scenefile.rs:26,
ab1.rs:27, toonfull.rs:26, battery.rs:15 (each verified to open
`use crate::scene::{`). Qualified-call cites: scenefile's
`scene::parse_json` 826 → 836 (W66) → **836, unchanged**; dof's
`chart_texture_rgba` 207 → 173 (W66) → **179** (+6, knobs phase-2
dof plumbing). Neither gates a move — re-exports preserve every
path — but the executor's map should carry 179.


---

## 5. Execution protocol for the lead

### 5.1 The 22 commits, in serial order (film first, per W66 §7)

**Programme F (W21 — 9 commits; step 5 is DONE and is not in the
series):**

| # | Commit | Gates (W21 §5 vocabulary, as amended) |
|---|---|---|
| F1 | step 1: extract `film::motion` | G-CI, G-TEL |
| F2 | step 2: extract `film::data` | G-CI, G-TEL, G-VAL |
| F3 | step 3: extract `film::paint` | G-CI, G-PROBE, G-AB (S10 mid + S01) |
| F4 | step 4: extract `film::chars` (pure move; widenings #1–#3) | G-CI, G-PROBE, G-TEL, G-AB (S08) |
| F5 | step 6: extract `film::sets` | G-CI, G-PROBE, G-AB (S02 + S10) |
| F6 | step 7: extract `film::staging` | G-CI, G-PROBE, G-TEL, G-AB (S01, S02, S07, S10, S11) |
| F7 | step 8: extract `film::render` | G-CI, G-PROBE, G-AB (6 pairs; full 60-pair + one full-film re-render recommended at this step only) |
| F8 | step 9: extract `film::overlay` (widenings #4–#5) | G-CI + byte-compare of one S07 calendar frame (≈950) and one captioned frame, sha256 equal (G-TEL is not evidence here) |
| F9 | step 10: extract `film::telemetry` + `film::driver` | G-CI, G-TEL, CLI smoke (`--film --from 195 --to 205` renders, then skips on re-run) |

(One optional docs-only closing commit updating film.rs's header
to describe the layout follows F9, per W21 §5 — outside the 22.)

**Programme S (W22 — 13 commits), starting only after F9 is
green** (W66 §7's two binding reasons stand: gate attribution —
one suspect per red run — and step 13's zero-importer-diff proof
window, which is only measurable while film.rs is not being
edited):

| # | Commit | Gates (W22 §5, as amended) |
|---|---|---|
| S1 | step 1: `scene/json.rs` | Full CI green; `cargo test` count unchanged |
| S2 | step 2: `scene/colour.rs` | **Battery 46/46** (see §5.3 precondition) |
| S3 | step 3: `scene/geo.rs` | s28 anchor pair + ab1-as-data 95.17 dB |
| S4 | step 4: `scene/raster.rs` | **Battery 46/46** + M6 exact-agreement spot check |
| S5 | step 5: `scene/toon.rs` | **Battery 46/46** (face cards) + s28 anchor |
| S6 | step 6: `scene/textures.rs` | fx-textures byte-identity + dof-chart A/B |
| S7–S12 | steps 7–12: `scene/painters/{planks,wallpaper,felt,parcel,picture,sign}.rs`, one per commit, code+tests fused, sign last | Per step: that painter's test module green in CI + fx-textures byte-identity |
| S13 | step 13: slim `scene.rs` → `scene/mod.rs` + the staging-dir decision (§6) | Full gate suite once: s28 pair, ab1-as-data, battery, fx byte-identity, G-TEL as amended (A6), + the grep proof (`git diff --stat` on the seven importers across the whole S series = 0 lines) + the step-13 surface audit (trap 1: `pub(crate)` names diffed before/after) |

**Total: 22 pure-move commits.** Visibility delta across both
programmes: exactly the 5 W21 widenings (all private →
`pub(super)`); W22's delta is zero. Behaviour delta: zero.

### 5.2 Per-step CI watch (what "green" means at each step)

- **G-CI on every commit:** ci.yml **and** tools-ci.yml green on
  the step commit before the next step starts (repo law: CI is
  the only build; no local cargo). ci.yml at HEAD runs
  `cargo build --release` (line 61) + `cargo test --release`
  (line 64) — a test-count drop in the test leg is a stop signal
  at any step (W22 trap 4: tests are fused to the moved code).
  A run **cancelled** by concurrency supersession is not a red:
  re-check the superseding run's conclusion.
- **G-PROBE steps (F3, F4, F5, F6, F7):** render.yml dispatch,
  film mode, byte-probes at 195–204 (10/10), 795–814 (20/20),
  1195–1204 (10/10) byte-identical to the pre-step tree.
- **G-TEL steps (F1, F2, F4, F6, F9; and S13's suite):** the
  verdict evidence is the CI-run binary's `--film-telemetry`
  output on the step tree, quoted in the commit body or the run
  log — `FILM TELEMETRY: CHECK OK` + process exit 0 (amended
  wording, §1). Not a local run: this VM's lavapipe walls make
  local film renders non-evidence.
- **G-VAL (F2):** `validate_film.py` 7/7, with the standing
  `--no-source` reduced-coverage caveat on runners recorded, not
  hidden.
- **G-AB steps:** the named pairs from the wave's 60-pair set;
  for a pure move the expectation is identical frames — any pair
  below its shot's recorded mean is a stop.

### 5.3 Precondition for S2/S4/S5 — the battery leg does not exist yet

The 46/46 battery gates in W22 steps 2, 4 and 5 have **no
standing CI invocation at HEAD** — this is W22 trap 6 and W66's
gate-mechanics note, still open, now stated precisely:

- The battery is a **binary scene mode**: `--scene stroke-battery`
  → `battery::run_battery` (main.rs:443; battery.rs:1). It is not
  a render.yml mode (dispatch options are exactly
  `[ab1, dof-chart, film, scene-file]`).
- ci.yml never invokes it (build + test only, lines 61/64), and
  `cargo test` does not cover it — battery.rs contains no
  `#[test]`; its spec is embedded via `include_str!` (W22 trap 6).
- **Therefore, before S2 starts:** the lead establishes one
  dispatch that runs the battery on the pre-split tree and
  reports **46/46**, and records the invocation (workflow or run
  id) in the S1 or S2 commit body. A workflow addition is lead
  business; this pack prescribes no shape for it. **S2, S4 and
  S5 do not start until that leg demonstrably exists** — a gate
  that was silently not running would void all three steps'
  evidence, and with it the M4 identity argument for the whole
  S programme.

### 5.4 Abort rule

Any red stops the programme at that step: CI or tools-CI failure,
a probe mismatch, a telemetry-verdict change in either direction,
a test-count drop, battery < 46/46, an A/B pair below its recorded
mean. **No step stacks on a red** — the next commit is never the
place to fix the previous step. Diagnosis follows the pure-move
rule (W22 §5): a failure in a move is a move error — find it by
diffing the moved text against the pre-move file
(`git show <prev>:src/film.rs` / `src/scene.rs`), never by fixing
logic inside the new module. If a step needs a **non-move edit**
to compile, stop: the map missed a crossing — record the edit in
the commit body and flag it back against W21 §3's census before
proceeding (W21 §5's universal commit shape). Programme-level:
no other landing touches film.rs or scene.rs mid-series; if one
must (e.g. §15 adoption step 2 lands in `load_film_data`),
**pause the programme, re-grep the cites, resume** — cites are a
cache, the cluster structure is the contract (W21's own header).

---

## 6. W22 step 13 — the staging-dir decision, framed for the lead

**Facts at HEAD (verified):** repo-root `painters/` holds 7 `.rs`
files (planks, wallpaper, felt, parcel, picture, sign, calendar)
+ provenance `.md` files + `INDEX.md` (the W39 integration pack's
index) + `TEST-RUN.md` (41 painter tests recorded passing against
the real scene.rs). The directory is **not compiled** — no `mod`,
no `include!`, no build reference. `INDEX.md` carries **no
superseded banner** (grep-verified at HEAD: no "superseded", no
pointer to a live home). The staging copies are stale by
construction (they carry the `use crate::scene::…` lines the
wave-1 paste deleted). Today the live copies are in scene.rs;
after S7–S12 they live in `src/scene/painters/`. Leaving the
directory as-is after S13 means **three** copies in the repo's
story — staging original, monolith paste (in git history), live
module — with nothing in the tree saying which is real: exactly
the trap the wave-1 paste created (W22 §3.3, trap 5).

**Option A — DELETE `painters/*.rs` in S13; keep the `.md`
provenance (INDEX.md, TEST-RUN.md, the per-painter notes), or
archive the whole pack under `docs/`.**
Buys: exactly one live copy in the tree; the audit's duplication
finding actually closes; no future agent can paste from the stale
copies by mistake.
Costs: the staging originals leave the working tree (they remain
in git history — deletion is recoverable, but no longer
side-by-side diffable without a checkout); TEST-RUN.md's claim
becomes a historical record rather than a re-runnable pack;
INDEX.md must be edited anyway to say where the pack went, or it
becomes a dangling index to deleted files.

**Option B — BANNER: add a superseded notice to
`painters/INDEX.md`** marking `src/scene/painters/` as the live
home and the staging files as frozen provenance.
Buys: zero deletion risk; the staging originals stay diffable
in-tree; the provenance chain (pack → paste → module) stays
readable in one place; one small edit.
Costs: the stale code stays in the tree forever — grep and
tooling keep surfacing two copies of every painter, and the
banner only protects a reader who enters through INDEX.md; a
direct hit on `painters/planks.rs` shows plausible, compilable-
looking, wrong code with no warning in the file itself. The
duplication finding is documented, not closed.

**Either way, two riders:** (1) the decision — or its explicit
deferral, with a reason — goes **in the S13 commit message**
(W22 step 13's own gate text); unmarked-and-undeleted is the
only unacceptable outcome. (2) `calendar.rs` needs its own line
in that message: it was never a texture painter and was never
pasted into scene.rs (W22 §3.3), so it is not covered by "the
six painters moved" under either option — INDEX.md's proposed
home for it (`src/calendar.rs`) never happened either, and S13
should say what becomes of it rather than letting it ride along
silently in a deletion or a banner written for the other six.

---

**Pack summary for the lead:** apply §1's three replacement texts
and §2's strike + replacement note to the blueprints, carry §3's
supersession and §4's cite tables as the executor's map, close
§5.3's battery precondition before S2, and decide §6 in the S13
commit. With those, both blueprints are executable as written:
22 pure-move commits, visibility delta 5, behaviour delta zero.

— W82
