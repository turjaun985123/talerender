# F2-MIGRATION-W53 — F2 migration staging pack (generic film data)

Seat W53, Wave 2 Batch 6. Surveyed first-hand at HEAD `aa92cee`
(lead's errors-step-2 delimiter fix on top of Batch 5's `d3def63`);
all `src/film.rs` line numbers below are this head's. **Status:
STAGED, NOT APPLIED.** This seat wrote only new files:

```
docs/integration/F2-MIGRATION-W53.md            (this note)
docs/integration/f2-migration/plates-manifest.schema.json
docs/integration/f2-migration/plates.json       (migrated manifest, real film)
docs/integration/f2-migration/film-data-additions.json
                                                (migrated film-data fragment)
```

Nothing in `film/`, `scenes/`, `src/`, or `tools/` was edited;
nothing is committed. The lead lands the migration from §2 as one
reviewed series.

**What F2 is.** W41's audit (FILMJSON-W41 §3) enumerated the film
driver's 13 asset loads: exactly one class — the caption strips — is
governed by film data. The other twelve are unconditional: a
hardcoded five-name plate array, a hardcoded four-year calendar
array, `calendar-dark.png`, `sign.png`, all behind a `plates.json`
that is read first and gates everything yet only supplies origins.
Two of those loads fail *silently* (§3). The fix direction accepted
at triage, and staged here: **`plates.json` becomes the real plate
manifest; the calendar set, calendar-dark, and sign move into film
data.** §1 is the data, §2 is the lead's change list, §3 the fallback
replacements, §4 the frame-impact analysis and proof plan, §5 the
generic-project payoff, §6 the open calls.

---

## 1. The proposed data

### 1.1 Plates manifest — `assets/plates.json`, migrated shape

Today's file is a bare object, `{name: [x0, y0]}`, written by
`film/gen_assets.py:153` and read at `src/film.rs:2056-2072`. Its key
set is decorative: the loader iterates its own hardcoded array
(`src/film.rs:2060`) and looks each name up. Proposed shape (schema:
`f2-migration/plates-manifest.schema.json`; migrated content for the
real film: `f2-migration/plates.json`):

```json
{
 "format": "talerender-plates",
 "version": 1,
 "plates": [
  {"name": "paper-map",   "file": "paper-map.png",   "origin": [-200.0, -270.0]},
  {"name": "leaflet",     "file": "leaflet.png",     "origin": [-200.0, -258.0]},
  {"name": "book-open",   "file": "book-open.png",   "origin": [-170.0, -180.0]},
  {"name": "book-closed", "file": "book-closed.png", "origin": [-170.0, -180.0]},
  {"name": "coconut",     "file": "coconut.png",     "origin": [-100.0, -108.0]}
 ]
}
```

Design points:

- **The envelope** (`format`/`version`) follows the repo's own
  precedent — `scenes/film/film.json` carries `format` and `version`
  keys. The tag is also the legacy-shape detector: a `plates.json`
  without it is the old shape and is rejected *by name* (§3, FB1)
  instead of being half-read.
- **`name`** is the staging lookup key — the string shot staging
  passes as `hold_l`/`hold_r`, resolved by name at
  `src/film.rs:2223-2227` into `src/toonfull.rs` `Plate`
  (toonfull.rs:115-121). Uniqueness is required (schema + loader):
  today's `HashMap` insert would silently overwrite a duplicate.
- **`file`** is explicit, assets-relative. Today the file name is
  *derived*, `format!("{name}.png")` at `src/film.rs:2061`, which is
  what welds the plate set to five Onoda names. Making it data —
  the same move the caption strips already made (`captions[].strip`,
  `src/film.rs:2073-2076`) — is the whole of F2 for plates.
- **`origin`** is `[x0, y0]`, REQUIRED, no default. Semantics are
  unchanged: the stage-local coordinate of the plate image's
  top-left corner relative to the hand point (src/toonfull.rs:113-114;
  plate pixels are res-2 in stage units, corners at
  hand + (x0, y0) .. hand + (x0 + w/2, y0 + h/2),
  src/toonfull.rs:233-235). Requiredness is the point — §3, FB2.
- **Entry order = load order**, and the migrated file keeps today's
  hardcoded array order (`src/film.rs:2060`). Load order is
  pixel-neutral (plates are consumed by name lookup, never by
  iteration), but keeping it makes the migration diff minimal.

Transcription — every value from two independent in-repo sources,
verified mechanically equal before this note was written (the staged
`plates.json` was diffed against `film/assets/plates.json`
programmatically; the check passed):

| plate | origin | source 1: current data | source 2: producer |
|---|---|---|---|
| paper-map | [-200.0, -270.0] | film/assets/plates.json:2-5 | film/gen_assets.py:69 (`paper_plate` returns `sub, -200.0, -270.0`) |
| leaflet | [-200.0, -258.0] | film/assets/plates.json:6-9 | film/gen_assets.py:88 (`leaflet_plate` returns `sub, -200.0, -258.0`) |
| book-open | [-170.0, -180.0] | film/assets/plates.json:10-13 | film/gen_assets.py:116 (`book_plate` returns `img, -170.0, -180.0`) |
| book-closed | [-170.0, -180.0] | film/assets/plates.json:14-17 | film/gen_assets.py:116 (same function, `open_book=false`) |
| coconut | [-100.0, -108.0] | film/assets/plates.json:18-21 | film/gen_assets.py:139 (`coconut_plate` returns `img, -100.0, -108.0`) |

`file` values: the PNGs on disk under `film/assets/` under exactly
these names (the loader's `{name}.png` derivation at film.rs:2061 is
the third source). Plate pixel sizes, for the record: paper-map
800×800, leaflet 800×800, book-open/book-closed 680×720, coconut
400×400 (IHDR of the files on disk).

### 1.2 Film-data additions — calendar set, calendar-dark, sign

Three new root keys in the film data (the `film.json`/`onoda.json`
document `load_film_data` reads, `src/film.rs:63-108`). Staged
fragment for the real film: `f2-migration/film-data-additions.json`:

```json
{
 "calendarYears": ["1945", "1955", "1965", "1974"],
 "calendarDark": "calendar-dark.png",
 "sign": "sign.png"
}
```

- **`calendarYears`** — the year *stems*, in display order. The
  loader keeps its existing derivation `calendar-{y}.png`
  (`src/film.rs:2079`) but iterates this list instead of the literal
  array at `src/film.rs:2078`. Order is load order *and* flip order:
  `assets.cal[cal.year]` is indexed by the flip counter
  (`src/film.rs:3336`), so the list's order is a staging fact, not a
  set. Values transcribed from `src/film.rs:2078` and the producer
  loop `film/gen_assets.py:194` — identical in both.
- **`calendarDark` / `sign`** — full assets-relative file names,
  replacing the literals at `src/film.rs:2085` and `src/film.rs:2086`
  (producers: film/gen_assets.py:219 and :214). File names rather
  than stems because neither has a templated sibling — there is
  nothing to derive.
- **All three keys are REQUIRED** in the migrated loader, demanded
  through the same `g()` mechanism as `anchors`/`lines`/`captions`/
  `duration` (`src/film.rs:76`): a film data file without them fails
  `film data: missing calendarYears` (exit paths in §3). Defaulting
  them to Onoda's values inside a nominally generic loader would be
  F2 preserved under a new coat of paint. The cost lands on §6.2.
- Naming: camelCase, matching the scenes-side manifest dialect this
  data's TS origin uses; the existing film-data keys are single
  lowercase words, and none of the three concepts fits one word
  honestly (`years` would collide conceptually with the anchors'
  `years0`). Flagged for the lead in §6.4 — renaming costs nothing
  at staging time and everything later.

`FilmData` (src/film.rs:44-49) gains `calendar_years: Vec<String>`,
`calendar_dark: String`, `sign: String`. `load_assets` already takes
`&FilmData` (src/film.rs:2054), so no signature changes there. One
staging consumer sits outside the loader — §2 step L4.

---

## 2. The lead's change list, in landing order

Order law for this migration: **data first, loader second, hardcoded
paths last** — with one refinement the code forces (§2.1): the plate
manifest's shape change and its loader change are a single atomic
step, because every intermediate state between them renders *wrong
pixels silently*. Each step names its intermediate-state guarantee.

### 2.1 Why the plates step is atomic (read before landing)

The old loader half-reads the new manifest shape: the migrated
`plates.json` still parses to a `Value::Obj`, so the guard at
`src/film.rs:2059` passes, the hardcoded loop at :2060 looks up
`paper-map` among the keys `format`/`version`/`plates`, finds
nothing, and every plate loads with origin **(0, 0)** — the exact
silent failure F2 exists to kill, now applied to all five plates at
once. The converse intermediate (new loader, old data) fails loudly
by design (§3, FB1). So: film-data keys land first (inert under the
old loader), calendar/sign loader support second (data already
present), and the plates data+loader swap lands as **one commit**,
never two.

### Phase D — data and producers (old loader still renders the identical film)

- **D1. `film/onoda.json`** gains the three keys of §1.2 (staged
  fragment: `f2-migration/film-data-additions.json`). Inert today:
  the loader ignores unknown keys (that tolerance is gap F3; here it
  is what makes data-first safe).
- **D2. `film/gen_film_data.py`** — the `out` dict
  (film/gen_film_data.py:90-100) emits the three keys alongside
  `duration`/`lines`/`anchors`/`captions`, so regenerating the film
  data cannot silently drop them. The values are this film's
  constants in the generator, mirroring film/gen_assets.py:194/214/219.
- **D3. Entry fixtures.** `tools/film/fixtures/entry/` (W41)
  deliberately carries *exactly the loader-consumed root keys*; once
  L1 makes the three keys required, both fixture files gain them
  (any valid values) in the same series, or tests A/B/D of
  FILMJSON-W41 §2.2 fail at `film data: missing calendarYears`
  instead of reaching the asset stage.

### Phase L — loader and staging (src/film.rs)

- **L1. Parse the new film-data keys.** `FilmData` gains the three
  fields (src/film.rs:44-49); `load_film_data` demands them via
  `g()` (src/film.rs:76) and parses them with the loader's existing
  idioms — `as_str` for the two file names (the pattern used for
  caption `text`/`strip`, src/film.rs:100-103), a string-list walk
  for `calendarYears` in the shape of the `lines` walk
  (src/film.rs:83-91), each element failure named
  (`calendarYears[2]: expected string` style, after `jnum`'s
  `{ctx}: expected number` at src/film.rs:59-61).
- **L2. Calendar + sign loads go data-driven; the literals die
  here.** In `load_assets` (src/film.rs:2054-2089): the calendar
  loop (src/film.rs:2078-2080) iterates `data.calendar_years`,
  formatting `calendar-{y}.png` as today; `cal_dark` (src/film.rs:2085)
  loads `adir.join(&data.calendar_dark)`; `sign` (src/film.rs:2086)
  loads `adir.join(&data.sign)`. The `Assets` struct
  (src/film.rs:2040-2046) is unchanged — same fields, same order,
  so `Assets.cal[i]` keeps meaning "the i-th declared year".
- **L3. Plates block rewrite — atomic with D4 below.** Replace
  src/film.rs:2056-2072 wholesale: read `plates.json`, parse,
  require the manifest shape (§3 FB1), then for each entry in
  `plates` order: require `name`, `file`, `origin` (§3 FB2), reject
  duplicate names, `load_png(adir.join(file))`, insert
  `Plate { px, w, h, x0: origin[0], y0: origin[1] }`. The hardcoded
  array (src/film.rs:2060), the `{name}.png` derivation (:2061),
  and the origin-default cascade (:2062-2069) are deleted in this
  step — they are the last hardcoded asset paths in the driver.
- **D4. `film/assets/plates.json`** is replaced by the staged
  manifest (`f2-migration/plates.json`), **in the same commit as
  L3**, and **`film/gen_assets.py`** learns to write it: the PLATES
  build/write at film/gen_assets.py:144-153 emits entries
  `{name, file: f"{name}.png", origin: [x0, y0]}` under the
  format/version envelope (the plate functions already return
  exactly `(img, x0, y0)` — gen_assets.py:69/88/116/139).
- **L4. The second `4` literal: `calendar_state`.** The calendar
  year *count* is hardcoded a second time, in staging:
  `calendar_state` (src/film.rs:1668-1688) opens with
  `let years = 4;` and clamps the flip index by it. Its only caller
  is s07 (src/film.rs:1731), which has `data`. Pass
  `data.calendar_years.len()` in. This is not optional polish:
  with L2 landed and L4 not, a data file declaring 3 years makes
  `assets.cal[cal.year]` (src/film.rs:3336) index past the vector
  (a panic, not a wrong pixel), and one declaring 5 silently never
  shows the fifth. Loader length and staging length must derive
  from the same list. (The `calendarYears: []` case needs the guard
  in §6.1.)
- **L5. Closing grep gate.** After L2-L4, these literals must not
  survive anywhere in `src/film.rs`'s load/staging paths:
  `"paper-map"`, `"leaflet"`, `"book-open"`, `"book-closed"`,
  `"coconut"` *as a load list* (the staging hold strings at
  src/film.rs:1609/1729/1760/1800/1835/1868 are data keys now —
  they stay, and they are the manifest's reason to exist);
  `"1945"`, `"1955"`, `"1965"`, `"1974"`; `"calendar-dark.png"`;
  `"sign.png"`; `let years = 4`. The sign's *consumer* wiring is
  untouched: `Renderer::new` still receives `&assets.sign`
  (src/film.rs:3406) and still places it at texture slot
  `TEX_SIGN = 10` (src/film.rs:408; the `sign.clone()` entry in
  `textures_px`, src/film.rs:2408).

### Phase T — tools re-transcription (same series, after L)

- **T1. `tools/format/check_assets.py`.** Its constants
  `PLATE_NAMES` / `CALENDAR_YEARS` / `FIXED_ASSETS`
  (check_assets.py:100-104) are transcriptions of the very literals
  L2/L3 delete, and check 5 (check_assets.py:383-410) guards the
  legacy plates.json shape. Re-point: the plate set and origins are
  read from the manifest in scope (and validated against
  `f2-migration/plates-manifest.schema.json`'s rules); calendar/sign
  expectations are read from the film data in scope (driver mode
  already loads it). Check 5's guard texts are rewritten to the new
  failure names in §3. Until T1 lands, tools-ci's asset step will
  (correctly, by its old lights) fail the migrated tree — land T1
  in the same push series so the red window is one CI round wide.
- **T2. `tools/format/check_unknown_fields.py`.** Its FILM-DATA
  known-key set is transcribed from `load_film_data` (W42); it
  gains `calendarYears`, `calendarDark`, `sign`. Between D1 and T2
  the checker reports the three new keys as UNKNOWN — expected,
  and the reason T2 is in this series rather than a follow-up.
- **T3. `tools/film/rerender_plan.py`.** Its `_diff_plates`
  (rerender_plan.py:643, helper `plates()` at :645) resolves
  plates.json as the legacy bare object; fed the migrated shape it
  reads `{}` and degrades every plate diff to noise. Teach it the
  entries form (resolve `origin` per `name`). §4's planner runs
  were made against the legacy shape and stay valid as *pre-migration*
  evidence; post-T3 the same perturbations should reproduce them.
- **T4. Format docs.** The three film-data keys and the plates
  manifest need a documented home (lead's call: `docs/scene-format.md`
  film-data section or the integration README — both lead-owned).
  FILMJSON-W41 §3's table is superseded for loads #1-6, #8-13 by
  this migration once landed; a pointer edit there keeps the audit
  trail honest.

---

## 3. The two silent fallbacks — replacement behaviour

Both replacements ride the error surface that already exists:
`load_assets` returns `Result<Assets, String>`; `run_film` wraps it
as `TaleError::data_load` (src/film.rs:3402), which `main` prints as
`error: failed to load <DIR>: {source}` and exits **2**
(src/errors.rs:183, :212-215); `--film-telemetry` takes the plain
`?` (src/film.rs:3527) and exits **3** with
`film telemetry failed: {source}` (EXIT-CODES-W02 contract). No new
exit codes, no new error variants.

| # | Today (silent) | Migrated behaviour |
|---|---|---|
| FB1 | **Non-object `plates.json` → zero plates.** `if let Value::Obj(m) = &origins` (src/film.rs:2059) simply skips the loop: `[]` or `"x"` parses fine, no plate PNG is even opened, the film renders plateless and telemetry is the only possible witness. | The manifest shape is required: root must be an object with `format: "talerender-plates"` and a `plates` array. Anything else → `Err("plates.json: not a plates manifest (expected {\"format\": \"talerender-plates\", \"plates\": [...]})")` from `load_assets`, surfacing as `error: failed to load <DIR>: plates.json: …` exit **2** (render) / `film telemetry failed: …` exit **3** (telemetry). A legacy-shape file gets the same rejection — there is no dual-read (§6.3). |
| FB2 | **Missing origin → (0, 0).** Three nested defaults (src/film.rs:2062-2069): the `(0.0, 0.0)` initialisation, a missing entry, a non-array entry, and `as_f64().unwrap_or(0.0)` on each component all land on (0, 0) without a word. The plate draws 200-270 stage units from where it belongs. | `origin` is required per entry and must be exactly two numbers. Missing entry field / wrong shape → `Err` naming the plate: `plates.json: plate 'leaflet': origin must be [x, y] numbers`. Same exit paths as FB1. The schema (§1.1) states the same rule for checkers. |

Adjacent, deliberately **not** changed by this migration:

- A staging hold naming a plate the manifest lacks still resolves
  to `None` at src/film.rs:2223-2224 and draws nothing. The loader
  cannot know the staging's names (they are code, §2 L5), so no
  load-time check is possible in the current architecture; the net
  for it is the telemetry cutout audit, as today. What the
  migration *does* remove is the inverse masking: today an unused
  sixth plate in plates.json is ignored (W41 §3.3); post-migration
  every manifest entry is at least loaded and PNG-validated.
- Duplicate plate names — impossible in the legacy object shape,
  possible in an array — are a load error under L3, not a silent
  `HashMap` overwrite.
- Caption `strip` defaulting to `""` (src/film.rs, captions block)
  is W41 §3.2's noted message-quality wart on the *model* class;
  out of F2's scope.

---

## 4. Frame-impact analysis

**Claim: a correct migration changes ZERO frames.** The argument
has two legs: (§1) every loaded byte and every origin is
value-identical by transcription — the only data changes are the
plates.json *shape*, three *added* film-data keys, and which code
path reads them; and (this section) for every migrated asset, the
exact frames it can touch are known, and §4.3's probe set covers
all of them plus controls.

### 4.1 Method

Two instruments, cross-checked. (a) `tools/film/rerender_plan.py`
run first-hand by this seat: snapshots of the current tree with a
single asset perturbed (one plate's origin +1.0, or one PNG's last
byte flipped), OLD = the pristine snapshot — the planner resolves
asset changes to segments by scene-data references, including the
`s07-calendar.json` sidecar. (b) The staging code itself: hold
gates, anchors, and `calendar_state`'s arithmetic, evaluated by
hand over the manifest's frame tiling (`scenes/film/film.json`:
s05b [737, 784), s07 [912, 1027), s08 [1027, 1211), s08b [1211, 1252),
s09 [1252, 1361), s10 [1361, 1520); 30 fps, frame n at t = n/30).
Planner ranges are segment-granular; staging ranges are the exact
pixel-bearing subsets. They agree everywhere the planner can see.

### 4.2 Per-asset impact

| asset | planner (perturbed) | exact frames (staging) | why |
|---|---|---|---|
| leaflet (origin) | [737, 784) — s05b, 47 fr | **[737, 784)** | s05b holds it while `t < drop_t`, `drop_t = 26.150` literal (src/film.rs:1583, hold at :1609); the last s05b frame is t = 26.100 < 26.150, so all 47 frames carry it. (After drop_t the falling paper is a `box_geo`, not this plate.) |
| paper-map (origin) | [1027, 1361) — s08+s08b+s09, 334 fr | **[1027, 1361)** | Suzuki's `hold_l` is set unconditionally in all three shots (src/film.rs:1760, :1800, :1835) — the union is three whole segments, back to back. |
| coconut (origin) | [912, 1027) — s07, 115 fr | **[971, 1027)** | s07 sets the hold only when `t >= w_coco - 0.05` (src/film.rs:1728-1729), `w_coco = anchors["coconuts6"] = 32.4` (film/onoda.json), so the gate opens at t = 32.35 → frame 971. The planner's segment bound is the conservative envelope. |
| book-open (bytes) | [1361, 1520) — s10 | **[1361, 1477)** | s10's `hold_r` is `"book-open"` while `t < w_flew`, `"book-closed"` after (src/film.rs:1868); `w_flew = anchors["flew9"] = 49.22`; frame 1476 is t = 49.200 (open), frame 1477 is t = 49.233 (closed). |
| book-closed (origin) | [1361, 1520) — s10, 159 fr | **[1477, 1520)** | Same switch, other side. (The planner found book-closed via s10's scene references — both book files are named in s10's scene data.) |
| calendar-1945 | [912, 1027) — s07 | **[917, 942)** | The calendar overlay exists only in s07 (`f.calendar = calendar_state(lt - 0.16)`, src/film.rs:1731; drawn by `overlay_calendar`, src/film.rs:3315, plate chosen as `assets.cal[cal.year]`, :3336). Displayed-year windows computed from `calendar_state` (src/film.rs:1668-1688) itself: cycle 1.04 s, flip at local > 0.62 (year advances at u >= 0.5). |
| calendar-1955 | [912, 1027) — s07 | **[942, 973)** | Same arithmetic. |
| calendar-1965 | [912, 1027) — s07 | **[973, 1004)** | Same arithmetic. |
| calendar-1974 | [912, 1027) — s07 | **[1004, 1027)** | Last year: no outgoing flip (`idx < years - 1` fails), shown to the segment end. Year windows total 110 frames with alpha > 0 inside [917, 1027). |
| calendar-dark | [912, 1027) — s07 (via the s07-calendar sidecar) | **[936, 948) ∪ [967, 979) ∪ [998, 1011)** | `cal_dark` is sampled only while `cal.dark > 0.0` (src/film.rs:3350-3352) — the three flip windows where the card is mid-turn. Outside them a wrong dark plate is invisible; inside them it darkens the flip. |
| sign | **[0, 1811) — full film (conservative)** | **[1361, 1520)** | The planner's one blind spot, and it says so: no scene file references sign.png, so dependents are "not derivable from data". The code settles it: the sign texture is drawn only by `build_bk`'s sign board (TEX_SIGN quad, src/film.rs:1103-1116), and `build_bk` is called by exactly one shot — s10 (src/film.rs:1846). s11/s12 are jungle shots (`build_jg`). The texture *uploads* on every run (slot `TEX_SIGN = 10`, src/film.rs:408, seeded at :2408 via `Renderer::new` at :3406) but is sampled only in s10's frames. |
| plates.json (the gate) | per-plate, via `_diff_plates` | **all frames, as a blocker** | It is read first (src/film.rs:2056) in both modes; a missing/unparseable manifest stops the film before frame 0. Post-migration that blocking position is unchanged — only its failure becomes named (§3 FB1) instead of silent-zero. |

Caveat carried from the planner's own docstring: the driver's
in-code cuts for s05b/s07/s08b sit up to 0.25 s *before* the manifest
boundaries, so segment-derived ranges can under-cover by ~8 frames
at those three cuts. §4.3's probe ranges use whole segments for
exactly those shots, which absorbs the caveat.

### 4.3 What the planner will say about the migration itself — and the proof that actually applies

Run `rerender_plan.py` on (pre-migration tree) vs (migrated tree)
and it will answer **full film**, for two of its standing rules:
driver data changed outside `captions` (the three new keys — its
rule exists because staging derives from driver data in code and it
refuses to guess), and (until T3) a plates.json it cannot parse
into per-plate entries. **That answer is the planner being
conservative about a change class, not evidence of pixel risk** —
and the converse also holds: the planner cannot *prove* this
migration pixel-neutral, because the neutrality claim is about code
paths reading identical values, which is below its data-level view.

The proof instrument is `tools/ci/byte_probes.py` (W34): render
the film at the pre-migration head and at the post-migration head
on the same adapter, and byte-compare. Defaults first, then the
prescribed additions (`--ranges` accepts comma-separated inclusive
ranges):

| range | covers |
|---|---|
| 195-204 (default) | control — s02; no migrated asset is loaded-onto-screen here |
| 795-814 (default) | control — s06; s06 stages no holds and no calendar |
| 1195-1204 (default) | paper-map frames inside s08 |
| 737-783 | leaflet — all of s05b (whole segment, per the cut caveat) |
| 917-1026 | the whole calendar window: all four years, all three dark-flip windows, and the coconut tail [971, 1027) |
| 1027-1360 | paper-map — all of s08/s08b/s09 |
| 1361-1519 | sign + book-open + book-closed — all of s10 |

Every frame in §4.2's exact column is inside one of these ranges;
the two controls stand for the complement. Expected result: **all
ranges byte-identical**, plus `FILM TELEMETRY: CHECK OK` and
`tools/film/validate_film.py` 7/7 on the migrated data. Any
mismatch localises by §4.2's table to one asset and one step of §2
— that is what the table is for.

---

## 5. The generic-project payoff

What a second project must supply after this migration that it
could previously leave implicit — i.e., what becomes *declared*:

- **A plates manifest** naming exactly its own plates — any count,
  any names its staging uses, each with its file and origin. A
  project whose staging holds one prop declares one plate.
- **Three film-data keys**: its own `calendarYears` (its own year
  stems, any count), `calendarDark`, and `sign` file names.
- The files themselves, under its own `assets/` — as today, but now
  *only* the files its data names (plus the caption strips, already
  data-driven).

What it no longer inherits from Onoda's film:

- **The five plate files as a precondition.** Today `load_assets`
  opens all five PNGs before any frame work in both modes
  (src/film.rs:2060-2061, called at :3402/:3527): a film with no
  bookshop in it still cannot render — at all — without a
  `book-open.png` on disk. Post-migration, an undeclared plate
  costs nothing and a missing declared plate fails by its own name.
- **The four calendar years, and the count 4.** A film with two
  calendar cards, or six, or years of its own century, declares
  them; the flip sequence follows the declared order through both
  the loader (L2) and staging (L4).
- **The `calendar-dark.png` / `sign.png` filenames** — now data,
  so a differently-named (or differently-languaged) sign board is a
  data edit, not a source edit.
- **Both silent failure modes** (§3). A malformed manifest or a
  missing origin is now a named exit-2/exit-3 error at load, not a
  film that quietly renders without its props or with them offset
  by (200, 270) stage units.

What it still inherits — named so nobody over-claims F2: the shot
staging itself is in-code per film (the `s01`-`s12` functions are
this film's transcription; generalising *that* is the exporter
problem, W47, not F2), the driver's 30 fps arithmetic, the
caption-strip model, and the renderer's texture-slot architecture
(the sign's slot is TEX_SIGN regardless of the file's name — only
the file became data).

---

## 6. Open calls for the lead

1. **`calendarYears: []` — legal?** Recommended: yes (a film with
   no calendar sequence should not have to declare a dummy year),
   but then L4 must guard `calendar_state` for `years == 0`: its
   clamp arithmetic computes `years - 1` on a `usize`, which
   underflows — a panic in debug builds, a wrapped clamp in
   release. The guard is one line (`years == 0 → None`, and no
   caller sets `f.calendar` meaningfully without years anyway).
   Decide before L4, not after a 0-year fixture finds it.
2. **`sign` / `calendarDark` required vs optional.** Recommended:
   required, this migration. `Renderer::new` is seeded from the
   sign unconditionally (src/film.rs:3406), so an optional sign is
   a Renderer change wearing a data change's clothes. A sign-less
   film supplies a blank PNG and the requirement is at least
   declared and error-named. Likewise `calendarDark` with
   `calendarYears: []` is loaded-but-never-sampled dead weight —
   harmless, and named here rather than hidden. Optional-ising
   either is legitimate *separate* work.
3. **No dual-read of the legacy plates shape.** Recommended, per
   §2.1: the F1 dual-name precedent (`film.json`/`onoda.json`)
   exists because film data lives in bridge packages outside this
   repo; `plates.json` has exactly one consumer and one producer,
   both landing in this series. A dual-read window would keep FB1
   alive inside it.
4. **Naming + schema strictness.** `calendarYears`/`calendarDark`
   camelCase (§1.2) vs the film data's lowercase habit — cosmetic,
   free now, permanent later. And the schema's
   `additionalProperties: false` binds the *checkers* (T1); whether
   the loader itself ever hard-errors on unknown keys is the F3 /
   format-v3 question (W42's recommendation: not now), not F2's.

---

*Evidence note: the §4.1 planner runs were made against scratch
copies of the tree (a 4 MB subset: `scenes/film/`, `scenes/atmospheres.json`,
`film/onoda.json`, `film/assets/`) with one perturbation each; the
perturbations never touched the repo. The §1.1 transcription was
verified by a mechanical diff of the staged manifest against
`film/assets/plates.json`. All other claims are source reads at
HEAD `aa92cee`, cited inline.*
