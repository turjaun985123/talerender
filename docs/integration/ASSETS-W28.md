# ASSETS-W28 — asset validation tool (`tools/format/check_assets.py`)

**Seat W28 (Phase 5). Deliverable: one new tool, fixtures, this
note. No existing file was edited; nothing is committed.**

Scene and film JSON reference files outside themselves — texture
images, caption strips, plate PNGs, segment scenes, one external
project tree. Until now nothing verified those references as a
set: a missing asset surfaced as a render-time failure deep in a
load, and two loader behaviours are *silent* (below). The tool
walks every reference the loaders actually read and checks it
against **the loader's own resolution rule** — existence,
file-kind sanity (magic bytes / JSON parse), the silent-fallback
guards, CWD-dependence warnings, and an orphan report.

House conventions follow `tools/format/check_prefabs.py` (W14) /
`check_clips.py` (W15): numbered checks, `OVERALL: PASS/FAIL`,
exit 0 = PASS, 1 = any FAIL, 2 = input error (one-line `ERROR`
on stderr, no report), deterministic output. Warnings (check 4)
and orphans (check 6) are report-only and never change the exit
code.

---

## 1. Field census — what asset references actually exist

Enumerated from source, not from the format docs. Line numbers
are for the tree as surveyed (Wave 2, post-CLI commit `d7bebdd`).

### 1a. Read by the Rust loaders at render/telemetry time

| # | Reference | Where written | Loader evidence | Resolution rule |
|---|-----------|---------------|-----------------|-----------------|
| 1 | texture image | scene `textures[]` entry `"file"` | `src/scenefile.rs:408-418` `resolve_texture` (`image::open`), called from the textures build at `:977-986` | `scene_dir.join(file)` — **the scene file's own directory** (`:974` `Path::new(path).parent()`); `docs/scene-format.md:261-262` documents the same rule |
| 2 | film data | *(path itself)* `<dir>/onoda.json` | `src/film.rs:64` `read_to_string(dir.join("onoda.json"))` | the `--film` / `--film-telemetry` dir argument |
| 3 | plate origins | `<dir>/assets/plates.json` | `src/film.rs:2047` | film dir + fixed name |
| 4 | plate PNGs ×5 | hardcoded names `paper-map`, `leaflet`, `book-open`, `book-closed`, `coconut` | `src/film.rs:2051-2052` `adir.join(format!("{name}.png"))` | film dir `assets/` + fixed names — **not data-driven** |
| 5 | caption strips | `onoda.json` `captions[].strip` | read at `src/film.rs:89` (defaults to `""` if absent), loaded at `:2066` `adir.join(&c.strip)` | film dir `assets/` + the strip string |
| 6 | calendar cards ×4 | hardcoded years `1945`, `1955`, `1965`, `1974` | `src/film.rs:2069-2071` | film dir `assets/` + fixed names |
| 7 | calendar dark + sign | fixed names `calendar-dark.png`, `sign.png` | `src/film.rs:2076-2077` | film dir `assets/` + fixed names |
| 8 | caption strip (ab1 mode) | CLI `--caption`, default `docs/ab/onoda-s01-t150/assets/caption.png` | default at `src/main.rs:159`; `image::open(caption_path)` verbatim at `src/ab1.rs:688` | **the caller's CWD** — the path is used exactly as given (see §4, finding F3) |
| 9 | caption metrics | CLI `--captions <path>` | `src/telemetry.rs:1303-1304` `load_captions` reads the path verbatim | caller's CWD (an invocation input, not scene data — censused, not tool-checked) |

### 1b. Data-layer references (named by the data; no Rust loader reads them today)

| # | Reference | Where written | Consumer evidence |
|---|-----------|---------------|-------------------|
| 10 | node data file | scene node `"ref"` — the only instance in the tree is `scenes/film/s07.scene.json:10487-10488`: an `overlays[0]` node `{"type": "calendar", "ref": "s07-calendar.json"}` | **no `src/*.rs` reads `"ref"` or any scene `overlays` block** (grep-verified); the calendar overlay is hand-ported in `src/film.rs:3349+`. The pointer serves the painter tooling (`painters/calendar.rs`, `painters/calendar.md`). The tool checks it anyway — the data names the file, and a dangling pointer is a defect whoever consumes it |
| 11 | segment scenes | manifest `segments[].scene` (`"scenes/film/s01.scene.json"` …) | `tools/film/validate_film.py:231` joins it against `--root` |
| 12 | source data | manifest `"sourceData": "film/onoda.json"` | manifest field; the driver data it names is #2 |
| 13 | external project | manifest `"project"` (absolute) | `tools/film/validate_film.py:282` reads `<project>/src` for source-literal checks; the Rust film driver never reads the project |
| 14 | film audio | manifest `"audio.file": "data/mix.wav"` + `"audio.projectPath"` (absolute) | data for the ffmpeg finishing step; no Rust reader |
| 15 | atmosphere presets | scene `"atmosphere": "noon"` etc. (names, not paths) | the film evaluator's presets are code-side; the names are validated against `scenes/atmospheres.json` only by `tools/film/validate_film.py:191`. **Not a file reference — out of this tool's scope** (validate_film check 7 owns it) |

### 1c. Generation-time references (the gen tools that produce the assets)

| Reference | Evidence |
|-----------|----------|
| project fonts `fonts/Inter-Black.otf`, `fonts/Inter-ExtraBold.otf`, `fonts/Caladea-Bold.ttf` | `film/gen_assets.py:35-36`, `:203` — the plates/strips/sign in `film/assets/` are rendered from these |
| project data `data/lines.json`, `data/words.json`, `script.json`; font for caption measuring | `film/gen_film_data.py:25-27`, `:56`; project dir from argv or the default `~/faceless-shorts/the-soldier-who-didnt-know` (`:22`) |
| rechunk caption strips → `assets/captions/cap-NNN.png` + `manifest.json` | `tools/captions/gen_caption_strips.py:54` (font), `:211-213` (project + out defaults). **This is a staging output, not a loader input** — the driver still reads `film/assets/cap_NNN.png` (underscore names). Censused so the two strip sets are never confused |

### 1d. NOT runtime references (compile-time; nothing to check on disk per-run)

- PMREM environment: `assets/room-env-pmrem.f16` is **embedded** via
  `include_bytes!` at `src/composer.rs:103` and `:154`. Its sidecar
  `assets/room-env-pmrem.json` is provenance for the capture page
  (`docs/ab/composer/project/env-capture.html`); no code reads it at
  runtime.
- All WGSL shaders (`include_str!`, e.g. `src/ab1.rs:922`) and the
  stroke-battery spec (`src/battery.rs:17`) — compile-time.
- `onoda.json`'s own `"project"` field (written by
  `film/gen_film_data.py:92`) is provenance only:
  `load_film_data` reads `duration`/`anchors`/`lines`/`captions`
  and never touches it.

### 1e. The loader's silent behaviours (why check 5 exists)

- `src/film.rs:2050`: plates load inside `if let Value::Obj(m)` —
  a non-object `plates.json` loads **no plates at all, silently**
  (the plate PNGs are never even opened).
- `src/film.rs:2054-2061`: a plate name missing from `plates.json`
  silently gets origin **(0, 0)**.
- `src/film.rs:89`: a caption without `strip` silently becomes
  `""`, and `load_png(assets/)` then fails without naming the
  caption.
- `src/scenefile.rs:985`: `tex_index.insert(id, …)` — a duplicate
  texture `id` silently overwrites; the first texture becomes
  unreachable while its file is still loaded.

---

## 2. The tool

```
python3 tools/format/check_assets.py [--root DIR] TARGET [TARGET ...]
TARGET = scene .json file | driver film dir (holds onoda.json)
       | manifest film dir (holds film.json)
```

Checks: **1 parse** (target parses / layout recognised; referenced
JSON parses) · **2 references** (every reference in §1a/1b resolves
under its own rule) · **3 kinds** (PNG magic for `.png`, RIFF/WAVE
for `.wav`, font magic supported, JSON parse for `.json`) ·
**4 portability** (WARN-only: CWD/`--root`-dependent bases,
absolute machine paths) · **5 silent-fallback guards** (§1e made
loud; manifest mode SKIPPED — no such surface) · **6 orphans**
(report-only: driver `assets/` contents, or a manifest dir's own
asset files, that no reference names; SKIPPED for a lone scene
file).

Manifest `--root` defaults to the current directory, mirroring
`validate_film.py`'s own `--root` default — the tool prints the
root it used in every manifest report line, so a wrong-root run is
self-evident rather than mysterious.

---

## 3. Fixtures (`scenes/fixtures/assets/`) and their runs

Four fixture families: `valid-scene/` + `valid-driver/` (mini-film,
all references satisfied, one deliberate orphan), `missing/`,
`wrong-kind/`, `cwd/` (manifest layout proving the root rule).
PNG fixtures are real 1×1 RGBA PNGs.

### valid

```
$ python3 tools/format/check_assets.py scenes/fixtures/assets/valid-scene/mini.scene.json
check_assets.py — scene scenes/fixtures/assets/valid-scene/mini.scene.json

[1 parse] PASS — scenes/fixtures/assets/valid-scene/mini.scene.json parses as a JSON object; scene-directory resolution (src/scenefile.rs:974)

[2 references] PASS — 2 references walked (scene-file mode)

[3 kinds] PASS — magic-byte / parse sanity of every referenced file

[4 portability] PASS — 0 warning(s), report-only

[5 silent-fallback guards] PASS — loader-silent behaviours, made loud

[6 orphans] SKIPPED — a lone scene file has no assets directory to orphan-scan

OVERALL: PASS (5/5 checks passed)
[exit=0]

$ python3 tools/format/check_assets.py scenes/fixtures/assets/valid-driver
check_assets.py — film dir /home/hatch/workspace/talerender/scenes/fixtures/assets/valid-driver (driver)

[1 parse] PASS — /home/hatch/workspace/talerender/scenes/fixtures/assets/valid-driver/onoda.json parses; driver layout (film dir + assets/, src/film.rs:64, :2046)

[2 references] PASS — 15 references walked (driver film-dir mode)

[3 kinds] PASS — magic-byte / parse sanity of every referenced file

[4 portability] PASS — 0 warning(s), report-only

[5 silent-fallback guards] PASS — loader-silent behaviours, made loud

[6 orphans] PASS — 1 orphan(s), report-only
    orphan: /home/hatch/workspace/talerender/scenes/fixtures/assets/valid-driver/assets/stray.png — present in the film assets dir, referenced by nothing

OVERALL: PASS (6/6 checks passed)
[exit=0]
```

(The orphan is reported and the run still passes — report-only,
as specced.)

### missing

```
$ python3 tools/format/check_assets.py scenes/fixtures/assets/missing/broken.scene.json
[2 references] FAIL — 2 references walked (scene-file mode)
    textures[0].file: 'nope.png' (scene file's directory) -> /home/hatch/workspace/talerender/scenes/fixtures/assets/missing/nope.png — not found
    nodes[0].ref: 'gone.json' (scene file's directory) -> /home/hatch/workspace/talerender/scenes/fixtures/assets/missing/gone.json — not found
OVERALL: FAIL (4/5 checks passed)
[exit=1]

$ python3 tools/format/check_assets.py scenes/fixtures/assets/missing
[2 references] FAIL — 15 references walked (driver film-dir mode)
    captions[1].strip: 'cap_001.png' (film dir assets/) -> /home/hatch/workspace/talerender/scenes/fixtures/assets/missing/assets/cap_001.png — not found
    assets/sign.png (loader-fixed name): 'sign.png' (film dir assets/) -> /home/hatch/workspace/talerender/scenes/fixtures/assets/missing/assets/sign.png — not found
OVERALL: FAIL (5/6 checks passed)
[exit=1]
```

(Excerpted to the failing check; all other checks PASS. Full
output shape is identical to the valid runs above.)

### wrong-kind

```
$ python3 tools/format/check_assets.py scenes/fixtures/assets/wrong-kind/kind.scene.json
[3 kinds] FAIL — magic-byte / parse sanity of every referenced file
    textures[0].file: /home/hatch/workspace/talerender/scenes/fixtures/assets/wrong-kind/fake.png is named .png but does not start with the PNG magic bytes (first bytes: 7b 22 61 63 74 75 61 6c) — the loader hands it to image::open as an image
OVERALL: FAIL (4/5 checks passed)
[exit=1]

$ python3 tools/format/check_assets.py scenes/fixtures/assets/wrong-kind
[3 kinds] FAIL — magic-byte / parse sanity of every referenced file
    captions[0].strip: /home/hatch/workspace/talerender/scenes/fixtures/assets/wrong-kind/assets/cap_000.png is named .png but does not start with the PNG magic bytes (first bytes: 74 68 69 73 20 69 73 20) — the loader hands it to image::open as an image

[5 silent-fallback guards] FAIL — loader-silent behaviours, made loud
    plates.json: top level is not an object — src/film.rs:2050 loads NO plates at all in that case, silently (the plate PNGs are never even opened)
OVERALL: FAIL (4/6 checks passed)
[exit=1]
```

### CWD-dependent (manifest root rule)

Run from `/tmp` with no `--root`, the manifest's references
resolve against `/tmp` and fail — the failure names the root it
used; with `--root` they pass, and check 4 keeps the dependence
visible either way:

```
$ cd /tmp && python3 ~/workspace/talerender/tools/format/check_assets.py ~/workspace/talerender/scenes/fixtures/assets/cwd/scenes/film
[2 references] FAIL — 2 references walked (manifest film-dir mode, root=/tmp)
    segments[0].scene: 'scenes/film/s01.scene.json' (--root /tmp) -> /tmp/scenes/film/s01.scene.json — not found
    sourceData: 'film/onoda.json' (--root /tmp) -> /tmp/film/onoda.json — not found
[4 portability] PASS — 2 warning(s), report-only
    WARN: segments[0].scene: 'scenes/film/s01.scene.json' resolves against --root /tmp — its resolution depends on the caller's working directory / --root; run from elsewhere without --root and it breaks
    WARN: sourceData: 'film/onoda.json' resolves against --root /tmp — its resolution depends on the caller's working directory / --root; run from elsewhere without --root and it breaks
OVERALL: FAIL (4/5 checks passed)
[exit=1]

$ python3 ~/workspace/talerender/tools/format/check_assets.py --root ~/workspace/talerender/scenes/fixtures/assets/cwd ~/workspace/talerender/scenes/fixtures/assets/cwd/scenes/film
[2 references] PASS — 2 references walked (manifest film-dir mode, root=/home/hatch/workspace/talerender/scenes/fixtures/assets/cwd)
[4 portability] PASS — 2 warning(s), report-only (same two WARN lines, root = the fixture root)
OVERALL: PASS (5/5 checks passed)
[exit=0]
```

Note the wrong-root run also lists `s01.scene.json` as an orphan:
under that root the manifest cannot reach it, so within the
checked resolution it *is* unreferenced. With the correct root
the orphan count is 0. This is the resolution rule reporting
itself, not noise.

Input-error path (house exit 2), verified:
`check_assets.py /nonexistent-target` →
`ERROR: check_assets.py: no such file or directory: /nonexistent-target`;
`check_assets.py scenes` →
`ERROR: check_assets.py: scenes: unrecognised film dir — holds neither film.json (manifest) nor onoda.json (driver data)`.

---

## 4. Real film run — verbatim

```
$ python3 tools/format/check_assets.py --root . scenes/film film
check_assets.py — film dir /home/hatch/workspace/talerender/scenes/film (manifest)

[1 parse] PASS — /home/hatch/workspace/talerender/scenes/film/film.json parses; manifest layout (segments resolved against --root, the validate_film.py convention)

[2 references] PASS — 26 references walked (manifest film-dir mode, root=/home/hatch/workspace/talerender)

[3 kinds] PASS — magic-byte / parse sanity of every referenced file

[4 portability] PASS — 17 warning(s), report-only
    WARN: segments[0].scene: 'scenes/film/s01.scene.json' resolves against --root /home/hatch/workspace/talerender — its resolution depends on the caller's working directory / --root; run from elsewhere without --root and it breaks
    WARN: segments[1].scene: 'scenes/film/s02.scene.json' resolves against --root /home/hatch/workspace/talerender — its resolution depends on the caller's working directory / --root; run from elsewhere without --root and it breaks
    ... (segments[2..13] identical in form, one per segment scene) ...
    WARN: sourceData: 'film/onoda.json' resolves against --root /home/hatch/workspace/talerender — its resolution depends on the caller's working directory / --root; run from elsewhere without --root and it breaks
    WARN: project: '/home/hatch/faceless-shorts/the-soldier-who-didnt-know' is an absolute machine path — portable only to the machine that authored it
    WARN: audio.projectPath: '/home/hatch/faceless-shorts/the-soldier-who-didnt-know/data/mix.wav' is an absolute machine path — portable only to the machine that authored it

[5 silent-fallback guards] SKIPPED — no loader-silent fallback surface in this mode

[6 orphans] PASS — 0 orphan(s), report-only

OVERALL: PASS (5/5 checks passed)

check_assets.py — film dir /home/hatch/workspace/talerender/film (driver)

[1 parse] PASS — /home/hatch/workspace/talerender/film/onoda.json parses; driver layout (film dir + assets/, src/film.rs:64, :2046)

[2 references] PASS — 86 references walked (driver film-dir mode)

[3 kinds] PASS — magic-byte / parse sanity of every referenced file

[4 portability] PASS — 0 warning(s), report-only

[5 silent-fallback guards] PASS — loader-silent behaviours, made loud

[6 orphans] PASS — 0 orphan(s), report-only

OVERALL: PASS (6/6 checks passed)
[exit=0]
```

(The 14 per-segment WARN lines are elided above with `...` for
length; every other line is byte-verbatim. The elided lines are
identical in form to the two shown.)

### Findings from the real run

- **F1 — the film's asset graph is complete and clean.** 112
  references walked across both trees (26 manifest + 86 driver),
  zero missing, zero wrong-kind, zero orphans, `plates.json`
  carries all five origins. `film/assets/` is exactly the
  referenced set: 73 caption strips (the W01 re-chunk count) + 5
  plates + 4 calendars + dark + sign + `plates.json` = 85 files.
- **F2 — the manifest layer is machine-bound by construction**
  (17 warnings, all report-only): every `segments[].scene` and
  `sourceData` resolves only against a repo-root CWD/`--root`,
  and `project` / `audio.projectPath` are absolute paths into
  `/home/hatch/faceless-shorts/…`. On this machine all resolve;
  on any other checkout the manifest's external references break
  while the driver tree (`film/`) still passes untouched. This
  matches the format's intent (the manifest is a bridge document,
  the driver dir is the portable unit) — recorded, not fixed.
- **F3 — the ab1 caption default is CWD-relative.**
  `src/main.rs:159` defaults `--caption` to
  `docs/ab/onoda-s01-t150/assets/caption.png`, opened verbatim
  (`src/ab1.rs:688`): `--scene ab1` with no `--caption` works only
  from the repo root. The §28 scene file itself does it properly —
  `scenes/ab1.scene.json:62` references the same PNG as a texture
  `file`, scene-dir-relative. CLI-surface behaviour, outside this
  tool's targets; flagged for the lead.
- **F4 — CLI help wording drift (observation, not this seat's
  file).** `talerender --help` says `--film <DIR>` "holds film.json
  + its scene files"; the driver actually reads `<DIR>/onoda.json`
  + `<DIR>/assets/` (`src/film.rs:64`, `:2046`) and never opens
  the manifest or the segment scenes at render time. The tool's
  two film-dir modes exist precisely because both layouts are
  real: manifest for validation, driver for rendering.
- **F5 — two strip sets exist and must not be confused**
  (§1c): `film/assets/cap_NNN.png` (driver input, 73 files,
  referenced by `onoda.json`) vs `assets/captions/cap-NNN.png`
  (gen_caption_strips staging output, 73 files + `manifest.json`,
  referenced by no loader). The staging dir is outside both film
  dirs, so it is not orphan-scanned; if the re-chunk adoption ever
  points the driver at it, this tool's driver mode will verify
  the new set unchanged.

### Tool defects found and fixed during the real run

Honest record: the first real run reported two false findings,
both tool bugs, both fixed before the run above was taken:

1. `project` reported "not found" — the existence test used
   `isfile()` on a reference that names a **directory**. `Ref`
   now carries `want_dir` for that one reference class.
2. `s07-calendar.json` reported as an orphan and the ref count
   was 25, not 26 — the `ref` walker only descended `nodes`, but
   s07's calendar node lives under the top-level **`overlays`**
   block (`s07.scene.json:10487`). The walker now scans the whole
   document, which is also the truthful census (§1b #10).

No film/scene data was changed at any point — investigation
only, per this seat's orders.

---

## 5. What remains unchecked (by design)

- **Asset CONTENT correctness** — that a strip shows the right
  words, a plate the right drawing, an origin the right offset,
  a texture the right image. Existence + kind is this tool's
  whole claim; content belongs to the A/B corpus, the telemetry
  gates, and `tools/film/validate_film.py`'s source-literal
  checks.
- **Image decodability beyond the header** — kind checks read
  magic bytes only; a truncated PNG with a valid header passes
  here and fails in `image::open`. A full decode pass was
  deliberately not added (stdlib-only, and decode is the
  loader's own error path, which is loud).
- **References inside the external project** — `project` and
  `audio.*` are existence-checked at the boundary; the project's
  own internals (its `src/`, `data/`, fonts) belong to the
  project and to `validate_film.py`'s `--no-source` boundary.
- **CLI-invocation inputs** (`--captions`, `--cutouts-out`,
  `--out`, the `--caption` value itself) — censused in §1a #8/#9
  but not tool targets; they are the caller's paths, checked by
  the binary's own error messages.
- **`film/finish.sh` and the bridge packager's file reads** —
  shell/tooling layer, not surveyed for this tool's census.
- **Duplicate JSON keys** in scene/film files (the W14 tool
  reports them for the prefab layer; the loaders' hand parser
  and Python's `json` disagree on last-wins handling — a
  candidate extension, not implemented here).

## 6. Files written by this seat

- `tools/format/check_assets.py` (new)
- `scenes/fixtures/assets/` (new: `valid-scene/`, `valid-driver/`,
  `missing/`, `wrong-kind/`, `cwd/` — 53 small fixture files)
- `docs/integration/ASSETS-W28.md` (this note)

Nothing else was touched; nothing is committed.
