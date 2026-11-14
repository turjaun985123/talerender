# STRICTNESS-W42 — gap-fix F3: the unknown-field checker (report-only)

Seat W42, Wave 2 Batch 5 (gap-fix batch). Deliverables, new files
only, nothing committed:

- `tools/format/check_unknown_fields.py` — the checker (stdlib
  only, house style: numbered checks / OVERALL line / exit 0/1/2)
- `tools/format/fixtures/unknown_fields/` — 5 fixtures
- this note — the survey, the allowlist register, the verbatim
  real-data findings (the F3 evidence base), and the decision
  analysis for the lead

## 1. The gap, as verified

The loaders' vocabulary of **values** is closed — an unknown node
type, geometry kind, texture generator or transform op is a hard
loader error. The vocabulary of **keys** was completely open:
`src/scenefile.rs` and `src/film.rs` read the fields they know
via `Value::get` and ignore the rest, so a typo'd or dead key
renders exactly like an absent one. The external gap analysis
verified it from outside (`"bogus": 123` at scene root renders
fine); this seat verified it from the source and built the
instrument that makes the class visible. **Report-only: no loader
behaviour changes.**

## 2. Method: three classes of key

Every key in every object of every classified file lands in
exactly one class:

- **KNOWN** — read by a Rust loader. The sets were transcribed
  from source, not from the data: `src/scenefile.rs` (`load()`,
  `walk_nodes()`, `colour_geo` / `tex_geo` / `scatter_geo`,
  `material_colour`, `resolve_texture`, `eval_camera_keys`,
  `compose_transform`) for scene files; `src/film.rs`
  `load_film_data` (lines 66–115 at survey time) for film data.
- **TOLERATED** — not read by any Rust loader, but owned
  elsewhere: consumed by a named Python tool, or a deliberate
  record field documented in `scenes/film/ASSUMPTIONS.md` /
  `scenes/sets/README.md`. Every entry in the register (§4)
  carries its citation. A key with no citation does not get in —
  that rule is what keeps the register honest, and §6's residue
  is exactly the keys that failed it.
- **UNKNOWN** — nobody reads it, nothing documents it. Reported
  by JSON path; the run FAILs.

Files are classified by shape: SCENE (a `nodes` list), FILM-DATA
(`anchors` + `lines` + `captions`), MANIFEST (`segments` +
`format`), PLATES (a name→[x, y] map), ATMOSPHERES (`presets` +
`filmLooks`), OVERLAY-RECORD (`kind: "overlay"`), UNCLASSIFIED.
Classification is by shape, not filename — the checker therefore
works unchanged on the driver's data whether it is named
`film.json` (the lead's F1 fix, landed during this batch, prefers
it) or `onoda.json`.

Descent rules: loader-owned objects are descended into
recursively; TOLERATED keys are reported at the key and their
subtrees are **not** descended (the records under `shot`,
`captions`, `channels`, `overlays` belong to their own tools'
validators); free-form maps (`anchors`, `presets`, `filmLooks`,
plates) are not descended — their keys are data, never fields.
Geometry and texture entries are checked against **per-kind /
per-generator sets** when the kind names a known one, against
the union set otherwise. Findings aggregate by path pattern
(`[]` for array indices) with occurrence counts and the first
exact path.

## 3. The known sets (summary; the tool carries them verbatim)

- **Scene root** (scenefile `load`): format, version, time,
  name, camera, clear, lighting, lights, textures, nodes, post,
  fog. **Camera**: eye, target, fovY, near, far, up, keys;
  **camera key**: t, eye, target, fovY, ease. **Lighting**:
  directional {direction, ambient}. **Lights**: key, rim,
  hemisphere, points, envIntensity; key {color, intensity,
  position, target, shadow, shadowBox, far, bias, normalBias};
  rim {color, intensity, position}; hemisphere {sky, ground,
  intensity}; **points[] {color, intensity, position, distance,
  decay} — note: no `channel`** (see §4).
- **Node**: type, transform, children, geometry, material,
  texture, id, position, height, perspective, look, state,
  attachments, count, placement. **Transform ops** (single-key
  objects): translate, rotate_x, rotate_y, rotate_z, euler_xyz,
  scale. **Geometry** per kind: box {size}; cylinder {radiusTop,
  radiusBottom, height, segments}; sphere {radius,
  widthSegments, heightSegments}; cone {radius, height,
  segments}; disc {radius, segments}; quad {width, height, y0};
  flat-quad {width, depth}; ground-disc {radius, segments,
  uvRepeat}; sky-sphere {radius, widthSegments, heightSegments};
  billboard {center, width, height}. **Material**: color,
  roughness; **colour object**: hex, shade, shadeHash {index,
  base, span}. **Texture ref**: id, wrap. **Placement**: kind,
  angleHash, radiusHash, radiusBase, radiusSpan, angleScale,
  center. **Look**: body, head, shade, pattern, patCol, hat,
  hatCol, band, neck, neckCol, mustacheCol, extra, kid.
  **State**: mood, look, blink, talk, sweat, tears, armL, armR,
  legs, phase, lean, bob, squash, headTilt, headTurn, camPitch,
  sitOn, holdL, holdR; **holds**: plate, x0, y0. **Attachments**:
  kind, hand, offset.
- **Textures[]**: id, file, generator + per-generator fields —
  grass/planks/felt {base}, wallpaper {base, motif}, parcel
  {label}, picture {}, sign {canvas, bg, fg}, solid {color},
  sky-gradient {top, horizon, ground}, cloud {}, blob-shadow {},
  onoda-card {}, dof-chart-pattern {size}.
- **Post**: bokeh {focus, aperture, maxblur}, bloom {threshold,
  strength, radius}, grade {vignette, grain, warm, sat,
  exposure, time}. **Fog**: color, near, far.
- **Film data** (film.rs `load_film_data`): root {anchors,
  lines, captions, duration}; lines[] {start, end}; captions[]
  {s, e, text, fit, strip}. Nothing else is read — in
  particular the driver does **not** read `fps` (30 is in code)
  or any per-line text.
- **Manifest** (no Rust reader; the Python tooling's set —
  `validate_film.py`, `check_assets.py`, `rerender_plan.py`):
  root {version, format, title, project, resolution {width,
  height}, fps, durationS, totalFrames, frameRule,
  boundaryRule, sourceData, audio {file, durationS,
  loudnessLUFS, treatment, projectPath}, segments[] {id, scene,
  set, atmosphere, filmLook, lines, startS, endS, durationS,
  startFrame, endFrame, frames, captionChunks}}.

One reverse finding from the survey: camera-key **`ease` is
loader-known but used 0 times in the corpus** (all 32 real keys
carry only t/eye/target/fovY) — vocabulary ahead of data, the
mirror image of F3, recorded here so it is not mistaken for a
dead field later.

## 4. The allowlist register (TOLERATED keys)

Condensed from the tool (which carries the full citation text).
Citations are to consumers or to the corpus's own documentation —
the film/sets scene files are a **dual dialect by design**: the
loader's format plus a transcription record of the source
project, and ASSUMPTIONS.md is explicit that the record half is
carried data ("the loader consumes metalness nowhere yet, but
the value is now recoverable from scene data" — W73 correction).

| context | key(s) | owned by |
|---|---|---|
| scene root | shot | validate_film.py (shot.atmosphere), check_lights.py (shot.fire.pit) |
| scene root | captions | validate_film.py (id census), rerender_plan.py (chunk matching) |
| scene root | captionsNote | rerender_plan.py; tools/film/VALIDATION.md |
| scene root | atmosphere | validate_film.py (declared atmosphere), check_lights.py check 4 (R5) |
| scene root | atmospherePreset | **record of resolved preset values, read by no loader** — ASSUMPTIONS.md (the s06 record); check_lights.py deliberately does not check it (LIGHTS-W26) |
| scene root | duration, resolution | staging records (ASSUMPTIONS.md corpus); the loader's sources are `time`/the manifest and --width/--height |
| scene root | channels | animation-channel formula records (ASSUMPTIONS.md) |
| scene root | fire | check_lights.py check 6 tier B reads fire.pit |
| scene root | overlays | check_assets.py walks overlays[].ref (ASSETS-W28) |
| scene root (sets) | background, bookRows | set records (scenes/sets/README.md) |
| node | name | authoring label; validate_film.py diagnostics; ~every node |
| node | channels | stage_clips.py (ANIM-ADOPT-W32); ASSUMPTIONS.md |
| node | tint, envTint, role, rifleNote | source records (ASSUMPTIONS.md; role at its walk-target passage) |
| node (sets) | receiveShadow, basic | source facts/markers (sets README B6, view-plane passage) |
| material | map | source-material texture record (ASSUMPTIONS.md); the loader's texture mechanism is the node's `texture` — a colour-pipeline node cannot carry a map in v2 |
| material | metalness | carried field, loader consumes it nowhere yet (ASSUMPTIONS.md, W73 correction) |
| material | doubleSide, basic, emissive, emissiveIntensity | carried source-material records (ASSUMPTIONS.md, incl. the s10 bulb passage) |
| material (sets) | toneMapped | source record (sets README, view-plane passage) |
| texture | repeat, sourceDrawing, canvas | carried entry parameters (ASSUMPTIONS.md: "base/motif/repeat/canvas size…"); `canvas` is loader-KNOWN for the sign generator only |
| texture | text, font | the recorded signBoard(text, …) input + font rule (sets README B9) |
| texture | rotation | source map.rotation record (sets README, floor row) |
| texture (parcel) | base | set-dialect record (sets README B8); the parcel painter reads only `label` — handled per-generator in the tool |
| post | focusChannel | focus-driver record (ASSUMPTIONS.md: "post.focusChannel records…", post.bokeh.focus carries the value) |
| light point | channel | **check_lights.py check 6 tier A** (channel.kind == "fireLevel") — scenefile.rs's PointCfg has NO channel field; a two-consumer dialect fact |
| light key (sets) | radius, mapSize | source literals carried on the block (sets README B7) |
| any | `_`-prefixed | in-file documentation convention (atmospheres.json _cite/_note; check_lights.py skips `_` keys) |

Tolerated total on the real corpus: **7,043 occurrences** across
39 distinct (context, key) pairs — dominated by node `name`
(6,528), material `map` (158), texture `repeat` (78), node
`channels` (48), material `metalness` (41). This volume is
itself a decision input (§7): the tolerated dialect is not a
fringe, it is the corpus's normal state.

## 5. Fixture verification (all first-hand)

```
$ python3 tools/format/check_unknown_fields.py tools/format/fixtures/unknown_fields/fx-clean.scene.json
OVERALL: PASS (5/5 checks passed, 0 unknown-key occurrence(s) in 0 distinct path(s), 0 tolerated occurrence(s))   exit 0
$ ... fx-unknown.scene.json
[2 scene vocabulary] FAIL
    ... $ — unknown key 'bogus' in scene-root object (1 occurrence(s); first at $.bogus)
    ... $.nodes[] — unknown key 'bogosity' in node object (1 occurrence(s); first at $.nodes[0].bogosity)
    ... $.nodes[].material — unknown key 'shininess' in material object (1 occurrence(s); first at $.nodes[0].material.shininess)
OVERALL: FAIL (4/5 checks passed, 3 unknown-key occurrence(s) in 3 distinct path(s)...)   exit 1
$ ... fx-allowlisted.scene.json      (atmospherePreset + shot + captionsNote + material metalness/map + point channel + node channels/name)
OVERALL: PASS (5/5 checks passed, 0 unknown-key occurrence(s)..., 9 tolerated occurrence(s))   exit 0
$ ... fx-film-data.json              (planted root 'studio' + captions[].bogus)
OVERALL: FAIL — both paths named under check 3   exit 1
$ ... fx-manifest.json               (planted segments[].bogus)
OVERALL: FAIL — $.segments[0].bogus named under check 4   exit 1
$ python3 tools/format/check_unknown_fields.py /nonexistent.json
ERROR: check_unknown_fields.py: input file: cannot read ...   exit 2
```

The clean fixture deliberately exercises the full loader
surface (camera keys with `ease`, shadeHash colour, scatter +
placement, character look/state/holds/attachments, code-ref,
every post block, the sign texture's `canvas`) — it passes with
**zero** tolerated occurrences, proving the known sets are the
loader's, not the corpus's habits.

## 6. Real-data run — the F3 evidence base

Command (27 files — all scene files in the repo, the manifest,
the driver data, and the three vocabulary-less data files;
`scenes/fixtures/**` excluded by design: planted-invalid data
and not-yet-implemented §15 extensions):

```
python3 tools/format/check_unknown_fields.py \
  scenes/*.scene.json scenes/film/*.scene.json \
  scenes/film/film.json scenes/film/s07-calendar.json \
  scenes/sets/*.scene.json scenes/atmospheres.json \
  film/onoda.json film/assets/plates.json
exit 1 — OVERALL: FAIL (3/5 checks passed, 196 unknown-key
occurrence(s) in 15 distinct path(s), 7043 tolerated
occurrence(s))
```

Classification: 22 SCENE, 1 MANIFEST, 1 FILM-DATA, 1 PLATES
(5 entries — exactly film.rs `load_assets`' hardcoded five
names), 1 ATMOSPHERES, 1 OVERLAY-RECORD. Check 4 (manifest)
**PASS** — the manifest matches its tooling set exactly.

Findings, verbatim:

```
[2 scene vocabulary] FAIL
    scenes/film/s10.scene.json: $.textures[] — unknown key 'motifLaw' in texture object (1 occurrence(s); first at $.textures[3].motifLaw)
    scenes/film/s10.scene.json: $.textures[] — unknown key 'note' in texture object (2 occurrence(s); first at $.textures[4].note)
    scenes/film/s11.scene.json: $.nodes[] — unknown key 'note' in node object (1 occurrence(s); first at $.nodes[81].note)
    scenes/film/s12.scene.json: $.nodes[] — unknown key 'note' in node object (1 occurrence(s); first at $.nodes[78].note)
    scenes/sets/bookshop.scene.json: $.lights.key — unknown key 'near' in light-key object (1 occurrence(s); first at $.lights.key.near)
    scenes/sets/bookshop.scene.json: $.nodes[] — unknown key 'materials' in node object (2 occurrence(s); first at $.nodes[11].materials)
    scenes/sets/bookshop.scene.json: $.textures[] — unknown key 'note' in texture object (14 occurrence(s); first at $.textures[0].note)
[3 film-data vocabulary] FAIL
    film/onoda.json: $ — unknown key 'film' in film-root object (1 occurrence(s); first at $.film)
    film/onoda.json: $ — unknown key 'fps' in film-root object (1 occurrence(s); first at $.fps)
    film/onoda.json: $ — unknown key 'frames' in film-root object (1 occurrence(s); first at $.frames)
    film/onoda.json: $ — unknown key 'project' in film-root object (1 occurrence(s); first at $.project)
    film/onoda.json: $.captions[] — unknown key 'line' in film-caption object (73 occurrence(s); first at $.captions[0].line)
    film/onoda.json: $.captions[] — unknown key 'width' in film-caption object (73 occurrence(s); first at $.captions[0].width)
    film/onoda.json: $.lines[] — unknown key 'i' in film-line object (12 occurrence(s); first at $.lines[0].i)
    film/onoda.json: $.lines[] — unknown key 'text' in film-line object (12 occurrence(s); first at $.lines[0].text)
```

Reading them:

- **F3-a — the driver data carries 174 dead occurrences.**
  `film/onoda.json`'s root `film` / `project` / `fps` / `frames`,
  every line's `i` / `text`, and every caption's `line` /
  `width` are written by `film/gen_film_data.py` and read by
  **nobody**: `load_film_data` consumes anchors, lines
  {start, end}, captions {s, e, text, fit, strip} and duration
  only. `fps`/`frames` are doubly dead — the frame rate exists
  only as the code's hardcoded 30 (the gap analysis's G1
  observation, now with the key-level evidence). The shapes are
  provenance transplants: lines mirror the skill's `lines.json`
  ({i, start, end, text}), captions carry their generator's
  bookkeeping (line index, measured width). Harmless today;
  exactly the class that hides a typo tomorrow.
- **F3-b — uncited prose `note` fields (18 occurrences) + 
  `motifLaw` (1).** Texture/node `note` strings in s10, s11,
  s12 and bookshop are deliberate-looking provenance prose, and
  s10's `motifLaw` records the wallpaper-ceiling motif law as a
  string. None has a citing document or a consumer, so under
  the register's rule they cannot be allowlisted from this
  seat. They are the judgement-call class: one ASSUMPTIONS /
  sets-README sentence each would cite them; or they can be
  stripped. Surfaced, not silently resolved either way.
- **F3-c — bookshop dialect residue (3 occurrences).**
  `lights.key.near` is the same class as B7's cited
  radius/mapSize pair (a three.js shadow-camera literal) but is
  not named in B7. `nodes[].materials` (plural, ×2) is the
  sharpest finding in the run: those two nodes carry their
  **only** material data in a key no loader reads — the
  singular `material` is absent — so the field is not a record
  alongside the loader's data, it is a *replacement* for it.
  Inert today (the set files are staging fragments, never
  loaded whole — `load()` would also reject them for the
  missing `camera`), but it is the one place the dialect drift
  crosses from "record" to "shadow format".

Everything else is clean: all 14 film scenes and the ab1 /
character-v2 / dof-chart / fog-chart scenes match the loader
sets plus the cited register, with no per-kind geometry or
per-generator texture surprises.

## 7. Decision analysis for the lead

**Option A — report-only (this checker) + a tools-ci gate.**
Cost: ~0 (2 s-class runtime alongside the other validators;
W39's wiring shape applies). Keeps the dual dialect intact;
prevents *new* unknowns from landing unnoticed (a ratchet:
the 15 known paths can be baselined, any new path fails CI).
Risk: without the CI gate the register rots — the findings
above sat invisible until someone built this.

**Option B — warn in the loaders.** Print unknown keys to
stderr at load time. Costs: (i) the same register must then
live in Rust too, or every film-scene render warns thousands
of times (node `name` alone: 6,528) — two registers to keep in
sync, or one generated from the other; (ii) stderr discipline:
the telemetry adapter parses **stdout** (the verdict-line law),
so warnings are safe only while they never touch stdout, and
every future mode must re-derive that; (iii) warnings that
always fire train everyone to ignore warnings. Not recommended
while the register is young.

**Option C — hard-error in the loaders** (the gap analysis's
R9/G2 ask). Breaks **today**, by count: all 14 film scenes +
3 set fragments (the record dialect, incl. s06's deliberately
documented `atmospherePreset` — hard-error forces deleting
documentation data or carving exceptions that re-create the
allowlist inside the loader); the driver data until
`gen_film_data.py` stops writing its 8 dead keys (F3-a — a
generator change, not a data edit); any exporter-generated
file carrying provenance (the skill-side exporter path in the
gap analysis emits lines.json-shaped lines with `text`/`i` —
the same transplant as F3-a); and the §15 prefabs/clips
fixtures, whose keys are specified-but-unimplemented — a
key-level hard-error would conflate "unknown key" with
"unimplemented feature" and fight the reservation-guard design
in FORMAT15-DRAFT-W31 (§15.1 rejects *features* by name, a
different and narrower act). Deeper cost: the corpus's whole
transcription method — carry the source value now, document it
in ASSUMPTIONS, let a later wave consume it (metalness is the
worked example) — depends on key tolerance. Hard-error in v2
makes every future record field a breaking change.

**Recommendation:** land **A now** (checker + tools-ci +
baseline the 15 paths), disposition the findings as data fixes
in the fixes phase — F3-a by teaching `gen_film_data.py` to
stop writing dead keys (or giving `width`/`line` a cited
consumer), F3-b by one citing sentence per class or stripping,
F3-c by extending sets README B7 to `near` and deciding
`materials` explicitly. Revisit hard-error only at a format
v3, where the register *is* the spec; if a middle step is
wanted, a default-off `--strict` flag on the loader gives
R9's accept-test a home without breaking the corpus. B is
dominated by A on every axis that matters here.

## 8. Limits

Keys only — requiredness, types and value ranges belong to the
other validators and the loaders. Tolerated subtrees are not
descended, so an unknown key *inside* `shot` / scene `captions`
/ `channels` / `overlays` records is invisible here (their
owners' tools govern them). Free-form maps are not descended.
Per-kind/per-generator sets fall back to the union when the
kind/generator value is itself unknown (a bad kind is the
loader's error to give). The manifest's known set is
tooling-defined — no Rust loader reads it — so its PASS is
weaker by construction than the scene/film-data verdicts.
ATMOSPHERES / PLATES / OVERLAY-RECORD files have no loader key
vocabulary and cannot FAIL this checker; they are listed so
their existence in the corpus is on record.
