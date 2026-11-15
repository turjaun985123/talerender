<!--
DELIVERY NOTE — Chrome Replacement, Batch 1, Seat 6 (CR-B1-06):
bridge builder + P4 generalisation.

Files created (this seat, NEW files only; nothing existing edited,
nothing committed in either repo):
  tools/bridge/build_bridge.py                                  (repo)
  docs/integration/CR-B1-06-BRIDGE.md                           (this doc)
  ~/workspace/custom-skills/tall-tale-shorts/template/tools/
    native_modes/filmdata_validate.mjs                          (skill, P4)

Contracts honoured:
  - CHROME-REPLACEMENT-CONTRACTS.md §0 (bridge state) and §1 rows
    5/17 context (cues routing; existing projects' bridges).
  - V2-WP9-INTEGRATION.md §8 proposals P4 (this seat implements it,
    skill side, as a drop-in module — the swap itself is the lead's)
    and P5 (the general project->bridge converter — this seat's
    builder is the first working form of it, with the honest
    remainder in §7).
  - CR-B1-05 (cues seat) bridge requirements R1–R5, R7, R8 (§3).
  - CR-B1-07 (exporter seat): the talerender-skeleton/1 report is
    accepted as a primary input source (§2).
  - Lead finding (2026-11-15, this wave): the Onoda PROJECT's
    packaged bridge is STALE — two data generations behind the
    repo's film data. The round-trip (§6) therefore treats the
    repo's current film data as the target generation and the
    packaged project copy as the stale reference, and the
    builder's self-verification includes the binary's own
    telemetry verdict, not only the Python validator (§5).
-->

# CR-B1-06 — Bridge builder + P4 generalisation

## 1. What the builder automates

`tools/bridge/build_bridge.py` (stdlib-only Python) assembles a
complete `<project>/native/` bridge — the exact layout
`package_bridge.py` produces — from inputs a new project actually
has, instead of from film data the repo already holds:

```bash
python3 tools/bridge/build_bridge.py \
    --project <skill-project> \        # script.json, data/lines.json,
                                       # data/words.json (+ data/mix.wav)
    --scenes <dir> \                   # transcribed *.scene.json, one per segment
    [--skeleton <skeleton.json>] \     # talerender-skeleton/1 (CR-B1-07)
    [--assets-dir <dir>] \             # rasterised PNGs (plates/strips/calendar/sign)
    [--film-data <onoda.json>] \       # prior film data: anchors, F2
                                       # declarations, caption fits carried over
    [--anchors <anchors.json>] \       # {"name": s} or [["word", line], ...]
    [--calendar-year YYYY ...] [--calendar-dark F] [--sign F] \
    [--out DIR] [--force] [--compare DIR] \
    [--talerender-bin PATH] [--no-telemetry]
```

| Output | Derived from | Rule (verified against code) |
|---|---|---|
| `film.json` segments + tiling | scene `shot.at`/`shot.duration`, skeleton `atS`/`startFrame`/`endFrame` primary when supplied | frame n staged at t = n/fps; segment owns startS ≤ n/fps < endS ⇒ startFrame = ceil(startS·fps); ranges tile [0, totalFrames) (repo manifest's own `frameRule`) |
| `film.json` durationS / totalFrames | `data/lines.json` `duration` | totalFrames = ceil(durationS × fps); durationS is the LINES duration, never the mix duration (R4: Onoda 60.357, not 60.46) |
| segment `lines[]` | skeleton coverage primary; fallback: timed lines overlapping [startS, endS) | reproduces the repo manifest's mapping incl. S06 → [5,6] and the intra-line splits |
| segment `captionChunks` | generated captions with s ∈ [startS, endS) | matches the repo manifest on all 14 segments |
| segment `set` / `atmosphere` | skeleton `set` / `atmospheres[0].preset`, else scene `shot.atmosphere` | skeleton is the set-identity source (CR-B1-07) |
| `film/onoda.json` lines | `data/lines.json` | `{start, end}` per line — the binary loader's shape (the stale packaged copy's `i`/`text` bookkeeping is NOT emitted) |
| `film/onoda.json` anchors | `--anchors` (Wd() rule: first word in the line whose normalised form starts with the query — `film/gen_film_data.py`), `--film-data` carry-over, scene-level `anchors{}` | resolved at BRIDGE time from words.json (R3); anchor NAMES are hand-chosen (skeleton `owes: word-anchors`) — the builder resolves values, it does not invent names |
| `film/onoda.json` captions | `script.json` `caps` + words/lines, port of `tools/captions/gen_caption_strips.py build_captions()` (authored chunks, s = word.s − 0.03, line-gap end rule, final chunk ends at duration; autoChunks fallback ported) | strip names `cap_<NNN>.png` (the film data's convention); `fit` carried from `--film-data` for identical chunks, else assumed 1.0 **and counted in a printed WARNING** — a stdlib builder has no font measurer (PIL/Skia do that in `gen_caption_strips.py` / `gen_film_data.py`) |
| `film/onoda.json` F2 fields | `--film-data` carry-over or `--calendar-year/--calendar-dark/--sign` flags | `calendarYears` / `calendarDark` / `sign` emitted ONLY when the inputs declare them — never filled with Onoda's values. The current binary loader demands these keys by name (`film.rs load_film_data`); a film whose data declares none fails the telemetry leg by name until it declares them |
| `film/assets/plates.json` | scene data: every `{plate: name, x0, y0}` hold entry; origin fallback: the assets source's own plates.json | emitted in the CURRENT binary's manifest shape `{format: "talerender-plates", version: 1, plates: [{name, file, origin}]}` (`film.rs load_assets` rejects the legacy flat shape by name). Onoda's `book-closed` — named in a channel `to` with no origin anywhere in the scenes — is exactly why the fallback exists |
| `film/assets/*.png` | scene-relative file references first, then `--assets-dir` | the builder COPIES rasterisations; a missing one is a hard failure naming every missing file (§7) |
| scene files | the transcribed scenes, written back with R2/R5 applied (§3) | JSON-serialised in the repo's own style (indent 1, literal unicode, no trailing newline) |
| `MANIFEST-SHA` | sha256 of every built file | same format as `package_bridge.py` |
| audio block | `data/mix.wav` when present | duration measured with the stdlib `wave` module; LUFS is NOT measurable stdlib-side and is omitted, never copied from a stale manifest |

## 2. Skeleton consumption (CR-B1-07 input)

`--skeleton` accepts a `talerender-skeleton/1` report and treats
it as **primary where it covers a need**:

- **Consumed:** `timing.fps/width/height` (defaults, below CLI
  flags, above scene/default fallbacks); per-segment `atS`,
  `durationS`, `startFrame`/`endFrame` (tiling), `lines`, `set`,
  `atmospheres[0].preset`. Each is cross-checked against the
  scene-derived value; a disagreement prints a WARNING naming the
  segment and the skeleton value is used.
- **Carried, not consumed:** `owes[]` is copied into the built
  manifest's `bridge.skeletonOwes` (item + severity), so the
  manual remainder travels with the package.
- **Deliberately NOT consumed:** per-shot `cues` (the skeleton's
  cue times can be unresolved — S07's loop-offset tocks are its
  known gap — and per CR-B1-05 the scene files' `shot.cues[]`
  are the cue source of truth at bridge time, with
  `data/cues.json` authoritative for mixing); `camera` endpoints,
  `staged`, `lookRef`, `cast`, `instances`, `sets` (scene
  authoring inputs for the transcription step, not assembly
  inputs — the transcribed scene files already embody them).
  The skeleton's `lookRef` is NOT the manifest's `filmLook`
  (Onoda: lookRef `jungle` vs filmLook `jg-noon`) and is never
  mapped onto it; `filmLook` remains underivable (§6, §7).

## 3. Cues requirements (CR-B1-05 R1–R8) — compliance

| Req | Builder behaviour |
|---|---|
| R1 `shot.cues[]` on every segment scene | Carried from the transcribed scenes; a segment with no authored cues is written with `cues: []` (present, empty — never invented content) |
| R2 canonical spelling, provenance outside the entry | Legacy `{at, name, source}` entries (Onoda s05a) are rewritten to `{t, type, gain?, dur?, pan?}`; any other extra key is a hard error; dropped `source` strings are collected into the manifest's `bridge.cuesProvenance` |
| R3 absolute t, anchors resolved at bridge time | Cue `t` carried verbatim (already absolute in scene data); film-data anchors resolved here from words.json |
| R4 manifest durationS = lines duration | §1 — 60.357 on Onoda; the mix.wav duration (60.457 measured / 60.46 packaged) appears only inside the audio block |
| R5 `shot.events[]` | **Emitted on every shot.** Unless the transcribed scene already carries events (validated + normalised), the field is an **empty list** and the gap is named three times: on stdout, in the manifest's `bridge.eventsGap`, and here. Computing events needs clip-instance data (instance offset + clip-local ClipEvent.t, param-adjusted, none on looping clips) that the scene transcription does not carry; the builder will not fabricate them. Onoda's staging uses no clips at all (CR-B1-05 §3), so for this film the empty lists are also substantively correct |
| R7 closed vocabularies | Cue types validated against mix.py's 21 FX names; an event name found in `shot.cues` fails loudly pointing at R5; event cues validated against the 11-name event vocabulary ∪ FX |
| R8 manifest names every scene file | Every segment names `scenes/film/<file>`; leg (a) re-checks existence |

## 4. P4 — the data-driven validator + the exact wiring note

**Module:** `template/tools/native_modes/filmdata_validate.mjs`
(skill tree). It exports `validateFilmData(filmDir, manifest)`
with the adapter's exact interface — same inputs, returns the
parsed film data on success, throws an `Error` with the adapter's
message verbatim on failure, problems accumulated — and one
additional export, `requiredAssets(data, plates)`, naming its
derivation. The required-asset list comes only from the data:

1. `assets/plates.json` itself, plus every plate it declares —
   manifest shape (`plates[].file`, defaulting to `<name>.png`)
   and the legacy flat shape (`<name>.png` per key);
2. the film data's F2 declarations: `calendarYears[]` →
   `calendar-<year>.png` each, `calendarDark`, `sign` (filenames
   as declared; absent fields require nothing);
3. every `captions[].strip`.

No hardcoded plate, calendar, or sign names remain.

**Wiring note for the lead (exact swap point):** in
`template/tools/capture_native.mjs`, the local
`function validateFilmData(filmDir, manifest) { … }` definition
(the block beginning `function validateFilmData(filmDir, manifest) {`
and ending just before the `// The gate.` comment banner) is
deleted, and at the top of the file, with the other imports, the
lead adds:

```js
import { validateFilmData } from './native_modes/filmdata_validate.mjs';
```

Nothing else changes: the sole call site — in `main()`,
`try { validateFilmData(filmDir, manifest); } catch (e) { problems.push(e.message); }`
— already matches the module's signature and error contract, and
`readJson`/`fail` remain in the adapter for its other stages.
This seat did not edit `capture_native.mjs` (new files only).

**Self-test** (`node filmdata_validate.mjs --self-test`, run
2026-11-15): **PASS 4/4** — (1) an Onoda-shaped bridge (manifest
plates + F2 calendar declarations, all files present) passes;
(2) a bridge missing a DATA-NAMED asset fails naming it — the
fixture's plate is named `wombat`, which appears in no hardcoded
list, so the failure can only come from the data; (3) a
non-Onoda bridge with NO calendar plates declared or present
passes (the adapter's current hardcoded version fails this
fixture); (4) bonus: the legacy flat plates.json shape is
honoured (`leaflet.png` named when missing). The module was
also run against the round-trip-built Onoda bridge (§6):
**PASS** (73 captions, 17 anchors).

## 5. Self-verification — three legs, and why three

The builder verifies its own output three ways because the legs
are NOT equally strict, and the differences are themselves
findings:

- **(a) P4 adapter semantics** (in-process; the §4 derivation +
  manifest tiling + duration/frame agreement). Round-trip: **PASS**.
- **(b) `tools/integration/validate_native_project.py`** (subprocess,
  when `--out` is the project's own `native/`). Round-trip:
  **FAIL — and the failure is the validator's, evidenced:** its
  three problems are `plate format.png missing`, `plate
  plates.png missing`, `plate version.png missing` — it iterates
  the CURRENT plates.json's top-level keys (`format`, `plates`,
  `version`) as if they were plate names, because the WP9-era
  validator assumes the legacy flat plates shape. The binary
  requires the manifest shape and rejects the flat one by name,
  so the builder cannot satisfy both; it satisfies the binary.
  The validator and the stale packaged bridge (§6) are the same
  data generation; both predate the F2 plates migration. This
  leg's failure is reported, not hidden, and fixing the
  validator is a follow-up for its owner — this seat writes new
  files only. (The validator's one FLAGGED item — the scratch
  project has no `tools/capture_native.mjs` — is the known P1
  retrofit item, expected for a scratch copy.)
- **(c) Binary film telemetry** (lead-required):
  `~/talerender-bin/talerender --film-telemetry <built>/film`,
  verdict line + exit 0 required. Round-trip: **PASS —
  `FILM TELEMETRY: CHECK OK`** on the built bridge's film dir
  (binary at `~/talerender-bin/talerender`). Two honest notes:
  the audit is slow on this shared-CPU VM (~20 min wall for the
  full film; the builder's own leg-(c) invocation was cut off
  twice by runtime/VM kills during it, so the completed verdict
  was obtained by running the identical command against the
  identical built output detached, and reading its verdict
  line — the line the adapter contract makes authoritative);
  and the audit's caption section reports the chunks as
  estimate-grade/auto-chunked, which is its classification of
  the scene-carried captions, not a failure — the verdict line
  is CHECK OK. Skippable only via `--no-telemetry`, which
  prints the skip.

## 6. Round-trip evidence (Onoda, 2026-11-15)

Rebuilt from the repo's CURRENT sources — project
`script.json`/`data/` copied to a scratch project, scenes from
`~/workspace/talerender/scenes/film/`, assets + prior film data
from `~/workspace/talerender/film/`, skeleton generated fresh by
`export_project_v2.mjs` — into the scratch project's `native/`,
then compared (`--compare`) against the project's packaged
bridge `~/faceless-shorts/the-soldier-who-didnt-know/native/`.
The real project was never written to.

**Inventory:** built 102 files (101 in MANIFEST-SHA + itself) vs
packaged 102 files; **84 byte-identical, 18 differing, 0 only in
either**. Every differing file is itemised below — no difference
is hidden, and none is a builder error:

**Film data — the target generation, reached.**
The built `film/onoda.json` is **semantically identical to the
repo's current `film/onoda.json`** — every key, all 73 captions
(s/e/text/fit/strip exact), 17 anchors, 12 lines, and the F2
fields. Against the PACKAGED copy it differs exactly by the
stale-generation drift the lead named, plus its bookkeeping:

- packaged **lacks `calendarYears` / `calendarDark` / `sign`**
  (the binary refuses it verbatim: `film telemetry failed:
  film data: missing calendarYears` — observed this session);
- packaged captions carry the pre-cleanup `line` + `width`
  bookkeeping keys (the current generator strips them — F3-a);
  the chunks' s/e/text/fit/strip are otherwise **identical**;
- packaged lines carry `i` + `text`; current shape is `{start, end}`;
- packaged has extra top-level `film` / `project` keys.

**Manifest.** Built segments are field-identical to the repo
manifest's segments on id/scene/startS/endS/startFrame/endFrame/
frames/lines/captionChunks (and to the packaged manifest's —
its tiling was still current). Remaining manifest differences,
all explained: per-segment **`filmLook` absent in the built
manifest** (present in repo + packaged; underivable — §2/§7);
top-level **`boundaryRule` absent** (authored provenance prose,
not data the inputs carry); audio `durationS` 60.457
(wave-measured, 3 dp) vs packaged 60.46, and **`loudnessLUFS`
absent** (not measurable stdlib-side); `project`/
`audio.projectPath` point at the scratch project (correct for
the build target); the `bridge` block records this builder, the
cue provenance, the skeleton owes, and the R5 events gap.

**Assets.** All 85 asset files byte-identical except
`plates.json`: built = current manifest shape; packaged =
legacy flat shape (the binary's loader rejects the flat shape
by name — the packaged bridge is two generations behind here
too). Caption strips, plates, calendar and sign PNGs: identical
bytes (copied from the repo's current asset tree).

**Scenes (14 files, all differ — every difference named).**
Builder-introduced, by design: `shot.events: []` added to all 14
(R5); `shot.cues: []` added to the 8 segments whose scenes
carried no cues key (R1 present-but-empty); s05a's legacy cue
normalised `{at, name, source}` → `{t, type}` (R2). **Generation
drift in the packaged copies** (repo scenes have moved since
packaging; the builder copies the repo's current scenes): s10
packaged lacks `lights.key.near` and three texture notes the
repo scene carries; s11 packaged lacks a node note; s12
packaged predates the W52 headTilt bake (its keys carry the
un-baked signs — `headTilt.keys[1][1]` 3.5 vs repo −3.5, `lean`
likewise, and the `apply`/`law` fields the bake removed/rewrote).

**Verdict:** a bridge built from current data **matches the
current generation exactly on everything the binary consumes**,
and the packaged project bridge is confirmed stale on film
data, plates shape, and three scene files. The correct
end-state per the lead — the built bridge passing the binary's
telemetry — is leg (c) in §5.

## 7. The honest manual remainder

What this builder does NOT do, stated plainly:

1. **Transcription itself.** The segment scene files — §15
   staging, draw-body animation, 3D set geometry, camera motion,
   grades (the skeleton's `owes`: draw-body-animation,
   scene-geometry, camera-motion, atmosphere-grade-resolution,
   shot-segment-mapping) — remain per-film authored work. The
   builder assembles a bridge FROM a transcription; it does not
   produce one. The skeleton report shrinks the search space
   (times, tiling, sets, line coverage) but authors nothing.
2. **Rasterisation.** Plate, caption-strip, calendar and sign
   PNGs are rendered by PIL code (`film/gen_assets.py`,
   `tools/captions/gen_caption_strips.py`) from drawing recipes
   and the project's fonts. A stdlib-only builder cannot
   rasterise; it copies existing rasterisations and fails loudly
   naming any it cannot find.
3. **Caption fit for new chunks.** Without a font measurer,
   fit is carried from prior data or assumed 1.0 with a printed
   count. An over-wide NEW chunk's fit can therefore be wrong
   until a measuring pass (`gen_caption_strips.py`) runs.
4. **Anchor names, `filmLook`, `boundaryRule`, LUFS.** Anchor
   names are hand-chosen by the bridge author (values are
   resolved mechanically); `filmLook` strings and the manifest's
   boundary-rule prose are authored metadata with no source in
   the timing/scene/skeleton data; LUFS needs a loudness meter.
5. **Clip events (R5).** `shot.events[]` is emitted empty until
   a transcription carries clip-instance data (or the §15 step-2
   evaluator emits events, CR-B1-05 R6). For films whose staging
   uses clips, event sound is still a manual step.
6. **Validator drift (leg b).** `validate_native_project.py`
   needs its plates check updated to the manifest shape before
   leg (b) can pass on a current-generation bridge.

## 8. Limits

- One bridged film exists; the builder's rules are verified
  against it alone. The autoChunks caption fallback is ported
  but unexercised by Onoda (every line authors `caps`).
- All native evidence on this VM is lavapipe/llvmpipe-class;
  no RADV or performance claim is made.
- The builder writes only under `--out` and refuses to
  overwrite a populated bridge without `--force` (managed
  paths only — an operator-dropped `native/talerender` binary
  is never touched, matching `package_bridge.py`).
