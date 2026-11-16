# R2-01 NOTE — bridge docs pack: claim → source map, verification, discrepancies

Seat R2-01, remediation Batch 2 (Package 4 bridge docs). Base state verified
live: talerender `git rev-parse HEAD` = `01a941adda6cbb0c3dbb5fd06f663117fa0a4166`
(the Batch 1 landings 093f40d / 5bc1b30 / 9469b04 / 67fd9ef + hygiene commit).
Skill tree = shipped v6.0, unedited by this seat. NEW files only, in this
staging dir; nothing committed; neither tree modified.

Deliverables:

| File | What it is |
|---|---|
| `SKILL-step4.patch` | Unified diff vs shipped `SKILL.md`, replacing the §5 step 4 "Bridge —" paragraph (shipped lines 396–416) with the followable procedure. **Verified: `patch -p1` applies cleanly to a pristine copy of the shipped SKILL.md** (dry-run + real apply on a scratch copy; result contains the new block at line 396). |
| `SKILL-step4-before.txt` / `SKILL-step4-after.txt` | The exact replaced block / replacement block, for review without applying the patch. |
| `CAPTURE-NATIVE-additions.md` | Deliverable B: three insert-ready subsections (driver field semantics; measured caption fit; fresh-bridge validator semantics) with named anchors in the shipped `template/tools/CAPTURE-NATIVE.md`. |

Method law honoured: every command/flag in the drafts was checked against
`build_bridge.py --help` output (captured live, exit 0) and/or the source —
no flag was copied from a seat note or the contracts alone.

---

## 1. Claim → source map

`BB` = `~/workspace/talerender/tools/bridge/build_bridge.py` (1,231 lines,
post-R1-03/R1-10, commit 5bc1b30). `VAL` =
`~/workspace/talerender/tools/integration/validate_native_project.py`.
Line numbers are the current files'.

### Builder interface (Deliverable A)

| Claim in the drafts | Source |
|---|---|
| Required args are `--project` and `--scenes`; `--out` defaults to `<project>/native` | `BB --help` (usage line); `BB:1146ff` `main()` argparse; docstring `BB:67ff` |
| Full optional flag set: `--assets-dir`, `--film-data`, `--caption-font`, `--allow-assumed-fit`, `--anchors`, `--fps`, `--width`, `--height`, `--skeleton`, `--calendar-year` (append/repeatable), `--calendar-dark`, `--sign`, `--force`, `--no-telemetry`, `--talerender-bin`, `--compare` | `BB --help` verbatim (all 17 options present); argparse block at end of `main()` |
| Builder reads `script.json`, `data/lines.json`, `data/words.json` from `--project` and dies by name on unusable input | `build()` head (`BB:486ff`): `load_json(project/"script.json")`, `data/lines.json`, `data/words.json` + shape checks |
| Scenes dir must hold `*.scene.json`, each with a `shot` object; builder does not transcribe `shots.ts` | `BB:529` die "no *.scene.json files… it does not transcribe shots.ts itself"; `BB:541` die "no shot object" |
| Segment start = skeleton `atS` primary, else `shot.at`; duration likewise; disagreement > 0.0015 s prints WARNING, skeleton wins | `build()` segment loop |
| Skeleton consumed via `--skeleton`, must be `talerender-skeleton/1`; supplies shot times, frame tiling, segment→lines, set, atmosphere; `owes` → manifest `bridge.skeletonOwes`; cues NOT from skeleton | `BB` skeleton block in `build()` + header docstring (Inputs section); exporter CLI `tools/export/export_project_v2.mjs:79` (`node export_project_v2.mjs <project-dir> <out.json> [--skeleton <path>] [--fps N] [--width N] [--height N]`), format const `talerender-skeleton/1` at its line 93 |
| `--anchors` accepts `{"name": seconds}` verbatim or `[["word", line], …]` resolved via `Wd()` against `data/words.json`, key = `<word><line>`; precedence `--anchors` > `--film-data` prior > scene `anchors{}` (later sources only `setdefault`) | `wd()` (`BB:301ff` region) + anchors block in `build()`; docstring Inputs section |
| No anchors from any source → printed WARNING, not a failure | `build()` print block after write |
| Plates: scene-declared plate needs `x0`/`y0` origin in scene data or fallback from assets `plates.json`; no default exists (build dies naming the plate) | `scene_plates()`; `BB:795` die "requires an origin per plate, no default exists"; fallback-origins block in `build()` |
| Cue types validated against mix.py's closed FX vocabulary; event names in cues die with the R5/R7 remedy; legacy `{at,name,source}` cues normalised to `{t,type,gain?,dur?,pan?}`, provenance recorded outside entries | `normalise_cues()` (`FX_VOCAB`/`EVENT_VOCAB` at `BB:153ff` region); `BB:452` die (clip EVENT name in cues) |
| `shot.events[]` always emitted (empty when the transcription carries none; gap named in manifest `bridge.eventsGap`) | `build()` R5 block + manifest bridge block |
| Builder copies rasterisations, cannot create them; missing assets die with the full list; rasterisation reference = `film/gen_assets.py` (PIL) | `BB:836` die "rasterised assets not found (this builder copies rasterisations; it cannot create them — see film/gen_assets.py)"; docstring `--assets-dir` paragraph; `film/gen_assets.py` header (PIL plates + caption strips + calendar cards) |
| Needed assets are derived from the data: every plate file, every caption `strip`, `calendar-<year>.png` per declared year, declared dark/sign files | `build()` asset-resolution block (`needed_assets`) |
| F2 keys emitted ONLY when declared (CLI flags or a declaring `--film-data` prior); never invented | `build()` F2 block comment + code; header `BB:52-57`; R1-03 note §1e/P2 + §5 synthetic-build evidence |
| Manifest carries `"driver": "scene-sequence"` | `BB:889` (manifest dict literal); confirmed in smoke build output (§2 below) |
| Manifest `bridge.captionFit` block; film data written to `film/onoda.json`; outputs = `film.json`, `MANIFEST-SHA`, `scenes/film/*.scene.json`, `film/onoda.json`, `film/assets/plates.json` + PNGs; MANIFEST-SHA = sha256 of every built file except itself | `build()` write block; docstring Output section; smoke build file listing (§2) |
| Exit codes: 0 built + all non-skipped legs pass; 1 build/verification failed; 2 usage/refusal (populated `--out` without `--force`); `--force` clears only MANAGED = `film.json`, `MANIFEST-SHA`, `scenes`, `film` | Docstring exit-codes paragraph (`BB:121ff`); `BB:153` `MANAGED`; `main()` failure counting + `sys.exit` |
| Three verification legs: (a) P4 adapter semantics (data-driven asset derivation), (b) repo validator — SKIPPED (printed) when `--out` ≠ `<project>/native`, (c) binary telemetry requiring the literal line `FILM TELEMETRY: CHECK OK`, skipped only via `--no-telemetry` (printed); binary discovery `--talerender-bin` → `$TALERENDER_BIN` → `~/talerender-bin/talerender`, absent binary = leg FAIL (not skip) | `verify_p4` `BB:980`, `verify_validator` `BB:1036`, `verify_telemetry` `BB:1055`; header three-legged note `BB:38ff` |

### Measured caption fit (Deliverables A + B2)

| Claim | Source |
|---|---|
| `fit = min(1, 960/rawW)`, rawW = PIL/FreeType `getlength()` at 88 px, rounded to 5 dp; font resolution `--caption-font` → `<project>/fonts/Inter-Black.otf` → `$TALERENDER_CAPTION_FONT` | `BB:313` `CAP_SIZE, CAP_MAXW = 88, 960.0`; `resolve_caption_font` `BB:316`; `measure_caption_fits` `BB:332` |
| Measured value written for EVERY chunk; `--film-data` fit is cross-check only (disagreement > 0.005 → WARNING, measured wins) | Captions block in `build()` (R1-10 comment) |
| Unmeasurable fit (no Pillow / no font) dies with a named error unless `--allow-assumed-fit`; escape is recorded as `bridge.captionFit = {mode: "assumed-1.0", reason, chunksAssumed}`; measured mode records `{mode, measurer, font, fontMd5, capMaxW, chunksRevisedVsFilmData}` | `measure_caption_fits` die/return paths; manifest bridge block in `build()`; smoke build (§2) printed the measured block with fontMd5 `c26b4e77dc103fd3a1e3b426549296ed` — identical to CR-R1-10's validated font MD5 |
| Pillow is an OPTIONAL import in the builder (degradation path names its absence) | `BB:140ff` try/except import block |
| Binary loader requires a numeric `fit` per caption (no default) | `src/film.rs:118` `fit: jnum(c.get("fit").ok_or("cap.fit")?…)` (via CR-R1-10-CAPFIT §1) |
| Render consumes fit: `caption_scale = sc * fit` (sc = 0.86→1 pop-in ramp) | `src/film.rs:2255` `f.caption_scale = (sc * c.fit) as f32` |
| Audit floors: rendered (strip-ink) width ≤ 850 hard, fit ≥ 0.68 hard; width test is ink-based and does not involve fit (so fit cannot fix a WIDTH fail; re-chunking is the remedy — LC idx 0/3/40) | CR-R1-10-CAPFIT §1/§3; CR-R1-10-MEASUREMENT headline + tables |

### Driver field semantics (Deliverable B1)

| Claim | Source |
|---|---|
| Manifest discovery for film dir `D`: `D/../film.json`, then `D/film.json`, then `D/../scenes/film/film.json`; fork decided before film data loads | REMEDIATION-CONTRACTS §1.1; `src/filmseq.rs:360` `discover_manifest` (+ layout tests at `filmseq.rs:659/699`) |
| `"driver": "scene-sequence"` → generic driver (`filmseq::run_film_seq`) and generic telemetry (`filmtelemetry::run_telemetry_seq`); otherwise legacy path unchanged | `src/film.rs` `run_film` fork (~line 3805 comment + 3812 test) and `run_film_telemetry` fork (~4088–4091) |
| Generic driver: per-frame segment scene loaded at local time `lt`, rendered via scene path; caption overlay from FilmData post-render; NO calendar overlay / fade unless scene data carries them; banner uses the manifest's title | REMEDIATION-CONTRACTS §2; `src/filmseq.rs:541` banner format `film: {} — {} frames @{}fps…` (title from manifest) |
| Generic telemetry verdict line is the same `FILM TELEMETRY: CHECK OK` / `CHECK FAILED (…)` | `src/filmtelemetry.rs:965/968` `verdict_line` |
| Legacy-entry guard: missing any of the 17 required Onoda anchors → named error "film data is not staged by the legacy driver (missing anchors: …); bridge it with driver: scene-sequence (see REMEDIATION-CONTRACTS §1)", never a panic | `src/film.rs:175` `legacy_anchor_guard`, `REQUIRED` 17 names at `:176`; called at `film.rs:3827` (film) and in telemetry |
| Scene-sequence driver refuses `OutputMode::Stream` by name, before any load; adapter video mode is disk-frames + finish so no adapter leg needs it | `src/filmseq.rs:491` error text; contracts §2 streaming bullet |

### Validator semantics (Deliverables A + B3)

| Claim | Source |
|---|---|
| Standalone use: `validate_native_project.py [project] [--frames DIR] [--expect-frames N] [--self-test]`; verdicts `RESULT: OK` (exit 0) / `RESULT: FAILED (…)` (exit 2) | `VAL --help` (captured live); `VAL:865/868` prints; `main()` return values |
| Fresh bridge with NO F2 keys is NOT stale; absence = legal declaration | `VAL` F2 block (~`:518ff`): only present-but-malformed fails; header amendment `VAL:18-27`; self-test case 7 (`VAL:1108ff`) |
| Present-but-malformed F2 fails BY NAME (`calendarYears` not a list of strings; `calendarDark`/`sign` not a string) | Same F2 block; self-test case 9 (`VAL:~1140ff`) |
| STALE case 1: files drifted from `MANIFEST-SHA` → "STALE BRIDGE: native/MANIFEST-SHA does not match…" with re-bridge `--force` remedy | `VAL:681ff` SHA block; problem text at `VAL:712` |
| STALE case 2: `assets/plates.json` legacy flat shape → STALE with re-bridge remedy | `VAL:~630ff`; text at `VAL:636` |
| Missing `MANIFEST-SHA` = FLAGGED only ("integrity not verifiable"), not a problem | `VAL:721` `flagged.append(…)` |
| Validator self-test passes 9/9 post-patches (cases 7–9 are the new semantics) | **Run live by this seat**: `validate_native_project.py --self-test` → `SELF-TEST: PASS (9/9)`, exit 0 |
| `BRIDGE_REMEDY` names `tools/bridge/build_bridge.py` | `VAL:115` |

---

## 2. Verification performed by this seat (first-hand)

1. `build_bridge.py --help` and `validate_native_project.py --help` captured
   live; every flag in the drafts cross-checked against both outputs and the
   argparse source.
2. `validate_native_project.py --self-test` → **PASS 9/9**, exit 0 (includes
   case 7 "fresh bridge without calendar/sign passes", case 8 "MANIFEST-SHA
   drift is STALE (no F2 keys)", case 9 "malformed calendarYears fails by
   name").
3. **Synthetic smoke build** (scratch project in /tmp: 1 line, 1 scene, 2
   caption strips, project font copied from the skill template;
   `--no-telemetry` since no current binary is provisioned for this seat):
   - builder wrote 6 files: `film.json`, `MANIFEST-SHA`, `scenes/film/s01.scene.json`,
     `film/onoda.json`, `film/assets/plates.json`, `cap_000.png`, `cap_001.png`;
   - manifest contains `"driver": "scene-sequence"` and
     `bridge.captionFit.mode = "measured"` with fontMd5
     `c26b4e77dc103fd3a1e3b426549296ed` (matches CR-R1-10's validated font);
   - film data keys are exactly `[anchors, captions, duration, fps, frames,
     lines]` — **no `calendarYears`/`calendarDark`/`sign`** (nothing declared,
     nothing invented);
   - verify leg (a) P4: **PASS**; leg (c): SKIPPED (printed, `--no-telemetry`);
   - leg (b) validator: FAIL — but the single problem was `required file
     missing: src/shots.ts` in my synthetic project (a real validator
     requirement for a skill project; the synthetic dir had no `src/`), plus
     2 FLAGGED warnings (no `data/mix.wav`, no project copy of
     `capture_native.mjs`). Builder exit was therefore 1, as designed.
     **No `STALE` appeared anywhere in the validator output for the fresh
     no-F2 bridge** (grep count 0) — the fresh-bridge semantics in the
     drafts are confirmed on a real build, not only on the self-test fixture.
4. `SKILL-step4.patch` applied with `patch -p1` to a pristine copy of the
   shipped `SKILL.md` in /tmp: applies cleanly, one hunk, new block lands at
   line 396.
5. Pillow availability in this environment: `python3 -c "import PIL"` →
   Pillow 10.2.0 (the measured path is the default here).

Not verified by this seat (no claim made): an end-to-end telemetry leg (c)
PASS on a real film (needs a provisioned current binary + a real project —
that is R2-03/R2-05 territory); `package_bridge.py` behaviour beyond its
`--help` and repo README (the drafts describe it only as "packages repo-held
film data, self-verifies", per both).

---

## 3. Discrepancies found (honest flags for the lead)

1. **Task-brief path vs reality:** the charter named the skill's
   `template/tools/build_bridge.py` as a key path. **No such file exists**
   in the shipped skill tree (verified: `template/tools/` listing has no
   `build_bridge.py`). The builder lives only in the talerender repo at
   `tools/bridge/build_bridge.py`. The shipped SKILL.md §5 step 4 already
   says this correctly ("The TaleRender repo carries the bridge tooling"),
   and the drafts keep that. No action needed beyond noting the brief's
   path was wrong.
2. **Stale header line inside build_bridge.py itself:** the delivery-note
   header still ends "Stdlib only. Never commits. Writes only under
   --out." (`BB:~137`). Post-R1-10 the builder imports Pillow (optionally)
   and *requires* it for the default measured-fit path. The header's other
   remediation text and the usage text are correct; only the "Stdlib
   only" line is stale. My SKILL.md draft states the Pillow requirement
   explicitly rather than repeating the line. Repo-side doc fix, lead's
   call.
3. **Shipped CAPTURE-NATIVE.md provenance header is stale:** the file in
   the shipped template still opens with "> **STAGED — B2-07 …** staged at
   `~/workspace/chrome-replacement/batch2/B2-07/…` for the lead to land
   with the Batch 3 default-flip" — yet the file *is* in the shipped
   template tree. Either it was landed without removing the staging
   banner, or the tree copy is the staging copy. The additions draft
   (Deliverable B) anchors against the file as it stands; the lead may
   want to retire that banner when landing B1–B3.
4. **`tools/bridge/README.md` (talerender repo) closing line is
   pre-replacement:** "…an unbridged project is captured with Chrome, and
   the adapter says so itself." Post-replacement the adapter refuses and
   names the bridge step; there is no Chrome capture route. Outside this
   seat's deliverables (repo doc, not skill doc) — flagged, not drafted.
5. **Contracts §4 shot attribution** (sign plane "only drawn by s07"):
   already flagged by R1-03 (CR-R1-03-DECOUPLING §4.1 — the sign plane is
   emitted by `build_bk`, called only by `s10`). My drafts make no
   shot-number claim about the sign plane, so nothing in them inherits
   the error.
6. **WAVE-STATE vs brief HEAD:** the task brief said talerender HEAD
   `01a941a`; live `git rev-parse HEAD` confirms `01a941a` (the WAVE-STATE
   text still says Batch 1 closed at `67fd9ef` — `01a941a` is the
   subsequent hygiene commit on top; consistent, not a discrepancy in
   substance).

---

## 4. Landing notes for the lead

- Apply `SKILL-step4.patch` with `patch -p1` (or `git apply`) in the skill
  tree root; it touches only the §5 step 4 bridge block (shipped lines
  396–416 → ~170 lines). No other SKILL.md section changes.
- CAPTURE-NATIVE additions B1–B3 are independent of the SKILL.md patch and
  of each other except for their relative order (B1 → B2 → B3 in one
  insertion run).
- The drafts deliberately do **not** document `--compare` beyond the
  builder's own help (a diffing aid, not part of the followable procedure)
  and do not promise a telemetry PASS for any specific film — that
  evidence belongs to R2-03/R2-05.
