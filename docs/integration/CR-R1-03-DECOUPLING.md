# R1-03 — Loader decoupling patch candidates (Package 3)

Seat: R1-03, remediation Batch 1. Base: talerender HEAD
`aa3de63c1d4a526bd39e66b79ae679c6e73bda85` (verified live with
`git rev-parse HEAD` before starting; tree clean except the
pre-existing untracked `Cargo.lock`). NEW files only — nothing
in either repo was edited, nothing committed.

Deliverables in this directory:

| File | What it is |
|---|---|
| `film.rs.patch` | Unified diff vs `src/film.rs` at aa3de63 (6 hunks). `git apply` verified. |
| `build_bridge.py.patch` | Unified diff vs `tools/bridge/build_bridge.py` (driver field + no-invention lock-in + honest docs). |
| `validate_native_project.py.patch` | Unified diff vs `tools/integration/validate_native_project.py`, including `--self-test` additions (cases 7–9) inside the same file — the validator's tests live in-file (`run_self_test`, line 966), there is no separate test file. |
| `work/` | Staging: `.orig` copies, patched `.new` copies, and the `make_*_patch.py` scripts that produced them (each edit anchored on an exact string, asserted to occur exactly once). |

All three patches were applied with `git apply` to a scratch
tree containing the pristine files; the applied results are
byte-identical to the staged `.new` files.

---

## 1. Complete consumer map (code at aa3de63)

Every read of `calendarYears` / `calendarDark` / `sign`, of the
`FilmData.calendar_*` / `FilmData.sign` fields, and of the
`Assets.cal*` / `Assets.sign` fields, in the whole repo
(`grep -rn` over `src/`, re-run at delivery):

### 1a. Film-data layer (`src/film.rs`)

| # | Site (file:line) | Read | Notes |
|---|---|---|---|
| D1 | `film.rs:51-53` | `FilmData { calendar_years: Vec<String>, calendar_dark: String, sign: String }` field declarations | Types **unchanged** by the patch — empty vec / `""` are the absence sentinels, so every downstream consumer of `FilmData` compiles and behaves untouched. |
| D2 | `film.rs:126-139` | `load_film_data`: `g("calendarYears")?` demanded by name; array-of-strings checked per element | Patched: `root.get(...)` — absent → `Vec::new()`; present non-array → `calendarYears: expected array`; present array with non-string element → `calendarYears[{i}]: expected string` (both by name, unchanged text). |
| D3 | `film.rs:143-152` | `str_key("calendarDark")` / `str_key("sign")` via `g()` — demanded by name | Patched: `opt_str_key` — absent → `""`; present non-string → `{k}: expected string` (unchanged text). |

### 1b. Staging / evaluator consumers of `FilmData`

| # | Site | Read | Notes |
|---|---|---|---|
| S1 | `film.rs:1808-1831` `calendar_state(lt_cal, years)` | takes `years: usize` (a count, not the vec) | **Already absence-legal**: `if years == 0 { return None }` guard predates this patch (W53 s6.1 comment at 1809-1811). Untouched. |
| S2 | `film.rs:1880` (in `s07`) | `f.calendar = calendar_state(lt - 0.16, data.calendar_years.len())` | The **only** staging call site of `calendar_state`, and the only read of `calendar_years` outside the loader/asset loader. Empty vec → `len()==0` → `f.calendar` stays `None`. Untouched by the patch. |
| S3 | — | `data.calendar_dark`, `data.sign` in staging | **No staging read exists.** Grep confirms the only reads are D3 and A2/A3 below. |

### 1c. Asset layer (`src/film.rs`)

| # | Site | Read | Notes |
|---|---|---|---|
| A1 | `film.rs:2223-2229` `Assets { cal, cal_dark, sign }` | field declarations | Patched: `cal_dark` / `sign` become `Option<(Vec<u8>, u32, u32)>`; `cal` stays `Vec<..>` (already empty-legal). Only constructor is A4; only readers are A5/A6 — both patched. No other `Assets {` literal exists in `src/` (grep-verified; the `#[cfg(test)]` module at film.rs:4145 touches only `state_key`). |
| A2 | `film.rs:2298-2300` (`load_assets`) | `for y in &data.calendar_years { cal.push(load_png(calendar-{y}.png)) }` | Untouched code; behaviour changes only through D2 (absent key → empty vec → zero iterations → zero files touched). |
| A3 | `film.rs:2305-2306` (`load_assets`) | `cal_dark: load_png(&adir.join(&data.calendar_dark))?`, `sign: load_png(&adir.join(&data.sign))?` — unconditional; with today's loader these always had a filename; a hypothetical `""` would have tried to open the assets **directory** as a PNG and failed | Patched: each loads only when its key is non-empty, wrapped in `Some(..)`; empty → `None`, no file touched. |
| A4 | `film.rs:3733-3734, 3994-3996` | `load_film_data` + `load_assets` call sites in `run_film` and `run_film_telemetry` | Untouched; both inherit the new absence semantics. Telemetry never constructs a `Renderer`, so it never reads `assets.sign` / `assets.cal_dark` at all. |

### 1d. Render / overlay consumers of `Assets`

| # | Site | Read | Notes |
|---|---|---|---|
| R1 | `film.rs:3633-3687` `overlay_calendar(px, w, h, assets, cal)` | `assets.cal[cal.year]` (3659); `assets.cal_dark` via `sample_bilinear(&assets.cal_dark, ..)` (3674), only inside `if cal.dark > 0.0` | Called **only** under `if let Some(cal) = &spec.calendar` (film.rs:3782-3784 in `run_film`; 3890-3891 in `run_film_stream`). A `CalendarState` exists only when S1 returned `Some`, which requires `years >= 1`, which requires a non-empty declared vec, which (A2) guarantees `assets.cal.len() == years` and `cal.year < years` — the index is safe by the same chain as today. Patched dark branch: `let Some(cal_dark) = &assets.cal_dark else { continue; }` — `continue` is safe because the dark block is the **last** statement of the per-pixel loop body and the year-plate `blend_over` for that pixel has already run; skipping the dark pass is the only effect. |
| R2 | `film.rs:2649` `Renderer::new(.., sign: &(Vec<u8>, u32, u32), ..)`; `film.rs:2681` `sign.clone()` as `textures_px[10]` (= `TEX_SIGN`, const at film.rs:515) | the sign PNG becomes standing texture slot 10 | Patched: parameter becomes `Option<&(Vec<u8>, u32, u32)>`; `None` → local `sign_fallback = (vec![0,0,0,0], 1, 1)` (1×1, RGBA all-zero = fully transparent), `unwrap_or` before the unchanged `textures_px` vec, so slot order/indices are identical. |
| R3 | `film.rs:3738` (`run_film`) | `Renderer::new(width, height, &assets.sign, ..)` — the **only** `Renderer::new` call site in the repo (grep-verified) | Patched: `assets.sign.as_ref()`. |
| R4 | `film.rs:1217-1228` (`build_bk`) | emits a `TexMapDraw { tex: TEX_SIGN }` sign-board face plane | This is the sign plane's only draw emission. `build_bk` is called only by `s10` (film.rs:2004) — **legacy staging**. See discrepancy note §4. |

### 1e. Non-Rust consumers (for the bridge/validator patches)

| # | Site | Behaviour at HEAD |
|---|---|---|
| P1 | `tools/bridge/build_bridge.py:595-602` | calendar/sign sources: CLI flags, else carried from a `--film-data` prior that declares them. |
| P2 | `tools/bridge/build_bridge.py:613-625` | film-data keys emitted **only if** some source declared them — verified by synthetic build (§5): a fresh build with no calendar inputs already emitted **no** F2 keys at HEAD. The invention in the B3-01 Lighthouse Cat bridge (years `1893/1904/1915/1924`, `calendar-dark.png`, `sign.png` — read directly from `batch3/B3-01/bridge-native/film/onoda.json`) did **not** come from this builder's default path; it came from feeding it declared/prior values. The patch therefore locks the behaviour in, fixes the docs that claimed the loader *demands* the keys, and adds the driver field. |
| P3 | `tools/bridge/build_bridge.py:750-776` | manifest dict — no `driver` key at HEAD (verified: synthetic build's manifest has `driver: None`). Patched: `"driver": "scene-sequence"` (§1 contract). |
| P4 | `tools/bridge/build_bridge.py:865-869` (`verify_p4`) | derives required calendar/sign assets from `data.get(...)` — already absence-safe; untouched. |
| V1 | `tools/integration/validate_native_project.py:119` + `512-524` | `F2_KEYS` + the loop that reports **each absent F2 key** as `STALE BRIDGE`. This is the mislabel the task names. Patched (§3). |
| V2 | `validate_native_project.py:632-656` | calendar/sign **asset** derivation — already "required ONLY as the film data itself declares them"; untouched. |
| V3 | `validate_native_project.py:660-702` | MANIFEST-SHA drift → `STALE BRIDGE`. **Kept verbatim** — post-patch this (plus the flat-plates generation check at 608-624) is what staleness means. |

---

## 2. Byte-identity argument for Onoda, consumer by consumer

Onoda's data (`film/onoda.json`, read directly) declares
`calendarYears: ["1945","1955","1965","1974"]`,
`calendarDark: "calendar-dark.png"`, `sign: "sign.png"` — all
present, all well-typed, all files present in `film/assets/`.

- **D2/D3 (loader):** every key takes the `Some(..)` arm, which
  is the old arm verbatim (same per-element checks, same error
  strings, same resulting values). `FilmData` is field-for-field
  identical to today's.
- **S1/S2 (staging):** untouched code fed an identical
  `calendar_years.len() == 4`. `calendar_state` outputs are
  identical; `f.calendar` is `Some`/`None` on exactly the same
  frames.
- **A2 (year PNGs):** same loop over the same 4 years, same
  filenames, same `load_png` order → `assets.cal` identical.
- **A3 (dark/sign loads):** both keys non-empty → both take the
  `Some(load_png(...)?)` arm with the same path expression as
  today → identical tuples, identical load errors if a file
  were missing.
- **R1 (overlay):** `assets.cal_dark` is `Some`, so the
  `let Some … else` binds immediately and the dark block runs
  the old statements with the same sampled texture. The year
  plate path is untouched. Identical pixels.
- **R2/R3 (sign texture):** `assets.sign` is `Some` → call site
  passes `Some(&tuple)` → `unwrap_or` returns it → slot 10 of
  `textures_px` is `sign.clone()`, the same expression in the
  same position; all 21 entries and their order are unchanged.
  Identical GPU uploads.
- **Everything else** (eval_frame, painter, composer, encoder):
  no hunk touches it.

Therefore Onoda is byte-identical **by construction**; the
§5 census (1,811/1,811 vs run 43797684898) remains the proof,
and the patch is designed so the census has nothing new to
catch. The one intentional behaviour change for *declared*
data: none. All changes fire only on **absence** (or on the
previously-impossible empty-string value, which today fails
trying to PNG-load a directory and now means "not declared" —
no shipped film data carries an empty-string value; Onoda's
are non-empty, verified above).

Edge cases the design pins down:

- `calendarYears` present but `calendarDark` absent, with a
  calendar sequence running: year plates composite; the dark
  shading pass is skipped (R1). No load error, no panic — the
  dark plate is shading-only (`cal.dark * 0.22` blend).
- `calendarDark` declared but `calendarYears` absent: the dark
  PNG **is** loaded (contract §4's literal rule: load when the
  key is non-empty) and a missing file still fails by path,
  exactly as today; the overlay never runs (S1), so it is
  never sampled.
- Legacy path with no sign but `s10` staging reached: slot 10
  is the 1×1 transparent texture. Legacy staging is Onoda's
  (contracts §1.3/§1.4) and Onoda declares a sign, so this
  case is unreachable for any film the legacy driver can
  actually stage; for the generic driver `build_bk` never
  runs at all (see §4).

## 3. What each patch does

**film.rs.patch** — 6 hunks: (1) `load_film_data` absence
semantics (D2/D3); (2) `Assets.cal_dark`/`sign` →
`Option<..>` (A1); (3) `load_assets` conditional loads (A3);
(4) `Renderer::new` `Option` sign + 1×1 transparent fallback
(R2); (5) `overlay_calendar` dark branch Option-aware (R1);
(6) `run_film` call site `.as_ref()` (R3). `FilmData` types
deliberately **not** changed — the contract's sentinel design
(empty vec / `""`) keeps S1/S2 and both drivers untouched.

**build_bridge.py.patch** — (1) manifest gains
`"driver": "scene-sequence"` (P3, contracts §1.2/§4);
(2) the F2 emission block's comment rewritten to state the
rule it already implements — emit only when CLI flags or a
declaring `--film-data` prior actually declare the keys,
never invented (P1/P2); (3) header/usage text that claimed
"the binary loader demands the F2 keys by name" corrected to
the post-decoupling rule. No behavioural change besides the
driver field: that is the honest state of the HEAD code
(evidence §5), and inventing a bigger diff would misrepresent
it. Note the prior-carry at P1 stays: a prior that *declares*
the keys is a declaration, which is exactly what §4 permits.

**validate_native_project.py.patch** — (1) the V1 stale loop
is deleted; in its place, present-but-malformed F2 values fail
**by name** (`calendarYears` not a list of strings;
`calendarDark`/`sign` not a string), mirroring the patched
loader; (2) header + `F2_KEYS` comment rewritten: absence is
current-generation, staleness = MANIFEST-SHA / generation
drift only (V3 and the flat-plates check unchanged);
(3) self-test additions **in the same file** (the validator
has no separate test file): fixture builder gains
`manifest_driver` / `write_sha` / `f2_wrong_type` parameters,
case 4's comment corrected (its STALE verdict now rests on
the flat plates.json shape alone), and three new cases —
**7:** fresh scene-sequence bridge, no F2 keys, matching
MANIFEST-SHA → PASS, no `STALE` in output;
**8:** same bridge with a post-SHA edit (trailing space) →
`STALE BRIDGE` + `MANIFEST-SHA`;
**9:** `calendarYears: "1945"` (present, wrong type) → fails
naming `calendarYears`.

## 4. Discrepancies found vs the contracts text (honest flags)

1. **Sign-plane staging attribution.** REMEDIATION-CONTRACTS
   §4 says "the sign plane is only drawn by s07 staging, which
   only runs when calendar data exists." In the aa3de63 code
   the sign plane is emitted by `build_bk` (film.rs:1217-1228)
   and `build_bk` is called only by **`s10`** (film.rs:2004);
   s07 is the *calendar* shot (its `calendar_state` call is at
   film.rs:1880). The load-bearing conclusion is unaffected —
   both are legacy-driver staging, which a scene-sequence film
   never executes, so a generic film never samples TEX_SIGN —
   but the lead should correct §4's shot number when landing.
2. **Contract §0's consumer list is incomplete.** It names
   `calendar_state`, the overlay, asset loads and "the s07
   staging". The complete map (§1) additionally includes
   `Renderer::new`'s standing-texture upload (film.rs:2681)
   and the `build_bk` sign plane (R4) — both handled by this
   patch; neither changes the §4 design.
3. **build_bridge.py did not itself invent keys at HEAD**
   (see P2). The B3-01 Lighthouse Cat bridge's invented values
   exist (verified in its `onoda.json`), but they entered as
   declared/prior inputs, not from this builder's defaults.
   The patch's bridge-side value is the `driver` field plus
   locking the no-invention rule into the code's own docs so a
   future editor cannot "fix" a telemetry failure by adding
   defaults.

## 5. Evidence gathered by this seat (staging copies only)

- Patched validator `--self-test`: **PASS 9/9** (original:
  6/6). Run both from the staged `.new` copy and from the
  git-applied scratch tree.
- Synthetic bridge build (minimal project, no calendar
  inputs, `--no-telemetry`): original builder → manifest
  `driver: None`, film data F2 keys `[]`; patched builder →
  manifest `driver: "scene-sequence"`, film data F2 keys `[]`,
  verify leg (a) P4 PASS. (Leg (b) reports SKIPPED when the
  script runs from the staging dir because it resolves the
  validator relative to its own path — an artefact of running
  a copy, not of the patch; in-tree it resolves.)
- Same synthetic project with the patched bridge installed as
  its `native/`: **original validator → FAILED, 3 problems,
  all `STALE BRIDGE: onoda.json has no 'calendarYears' /
  'calendarDark' / 'sign'`; patched validator → `RESULT: OK`**
  (2 unrelated FLAGGED warnings: missing mix.wav /
  capture_native.mjs in the synthetic project).
- film.rs: standalone `rustc` parse of the patched file
  produces an **identical error profile** to the pristine file
  (6× E0432 unresolved-import + 2× E0599 — expected for a
  crate-internal file parsed alone; zero syntax errors in
  either). This is a syntax sanity check only, **not** a
  compile: per the repo's standing pattern the real build
  gate is CI, and this seat ran no cargo build.
- All three patches: `git apply --check` + `git apply` clean
  against pristine copies; applied results byte-identical to
  the staged `.new` files.

## 6. Patch application order (for the lead)

Order matters only for coherence of intermediate states; the
three patches touch disjoint files and apply in any order
mechanically. Recommended:

1. `film.rs.patch` (talerender) — the loader/assets/renderer
   semantics everything else documents.
2. `validate_native_project.py.patch` (talerender) — so the
   validator stops mislabelling fresh bridges before any
   re-bridge is validated.
3. `build_bridge.py.patch` (talerender) — new bridges gain
   `driver: "scene-sequence"`. **Dependency:** the driver
   field is only *consumed* once R1-01's §1 manifest fork
   lands in `run_film` / `run_film_telemetry`; until then the
   field is inert data (the legacy path ignores unknown
   manifest fields — the binary today does not read the
   manifest at all, contracts §0). Land R1-03 before or with
   R1-01, not after the flip.

## 7. Test plan for the lead

1. **CI gates (standing §8):** talerender CI + Tools CI green
   at step level after applying all three patches; golden
   verify 17 OK (no scenefile change here, but run it);
   determinism byte-probes intact.
2. **Onoda data unchanged verdict:** `talerender
   --film-telemetry <repo film dir>` on the unmodified repo
   film → `FILM TELEMETRY: CHECK OK`; then the §5(a) census —
   post-remediation frame hashes vs run 43797684898 must be
   **1,811/1,811 identical**. (Designed byte-identical per §2;
   any diff is a defect in this patch, full stop.)
3. **LC bridge without the three keys loads + telemetry-runs**
   (once R1-01/R1-02 land): take the B3-01 Lighthouse Cat
   project, strip `calendarYears` / `calendarDark` / `sign`
   from its bridge film data (or use R2-03's re-bridge, which
   must not carry them), delete the invented
   `calendar-*.png` / `calendar-dark.png` / `sign.png` assets,
   and run: (a) the patched validator → `RESULT: OK`, no
   `STALE`; (b) `talerender --film-telemetry <film dir>` →
   loads (no `missing calendarYears` error) and reaches the
   generic telemetry verdict; (c) a frames leg → renders with
   no calendar/sign file present on disk.
4. **Wrong-type still fails by name:** set
   `calendarYears: "1945"` in a copy of Onoda's data →
   telemetry must fail with `calendarYears: expected array`;
   set `sign: 42` → `sign: expected string`. Set
   `calendarYears: [1945]` → `calendarYears[0]: expected
   string`.
5. **Declared-but-missing file still fails:** Onoda copy with
   `sign.png` removed from assets → load error naming the
   path (the `Some` arm is the old unconditional load).
6. **Validator self-test:** `python3
   tools/integration/validate_native_project.py --self-test`
   → `SELF-TEST: PASS (9/9)`.
7. **Bridge round-trip:** build a no-calendar project with
   the patched `build_bridge.py` → manifest contains
   `"driver": "scene-sequence"`, film data contains none of
   the three keys; build an Onoda-like project **with**
   `--calendar-year/--calendar-dark/--sign` → all three keys
   present with the declared values (declaration path
   unbroken).
