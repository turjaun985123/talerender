# CR-B1-07 — TS export hardening: the project → exporter → bridge pipeline contract

Chrome-Replacement Wave, Batch 1, Seat 7. Mission context:
`CHROME-REPLACEMENT-CONTRACTS.md`. Files delivered (new files
only; nothing existing was edited, nothing committed):

| File | Role |
|---|---|
| `tools/export/export_project_v2.mjs` | The hardened exporter: real skill project → C4 interchange document + segment skeleton report |
| `tools/export/export_project_v2.selftest.mjs` | Hermetic self-test, incl. a run against a read-only copy of the real Onoda project |
| `docs/integration/CR-B1-07-EXPORT.md` | This contract |

v1 (`tools/export/export_project.mjs`, V2 seat B2-5) is untouched
and still valid for what it was: it consumes a fixture-shaped
`project-source.json` manifest that no real project has. Its own
delivery note named the gap — "an adapter that produces this
manifest from a skill project is a separate, project-side step".
v2 is that step.

## 1. The pipeline

```
skill project ──► export_project_v2.mjs ──► talerender-project/1 document
 (script.json,        (this seat)            (loads via src/projectfile.rs)
  data/, src/,                              + talerender-skeleton/1 report
  fonts/, native/)                                │
                                                  ▼
                                     bridge builder (tools/bridge/)
                                     authors §15 scene files, film
                                     manifest + film data per segment
                                                  │
                                                  ▼
                                     <project>/native/ package ──►
                                     capture_native.mjs / talerender
```

Stage contracts:

- **Exporter (this seat)** — reads a project directory, never
  writes into it. Emits the interchange document and the skeleton
  report, or nothing (exit 2, all problems listed, no partial
  write — the same discipline as v1).
- **Bridge builder** — consumes BOTH outputs: the interchange
  document is the envelope of record (meta, asset manifest,
  section counts, exactly what `projectfile.rs` validates); the
  skeleton report is the staging input (§2). It authors the
  native scene/film data and packages `<project>/native/`.
  **Honest status:** the bridge builder that consumes the
  skeleton does not exist yet. `tools/bridge/package_bridge.py`
  as shipped packages only the pre-authored Onoda repo data and
  does not read either output. The skeleton is the input contract
  for that future builder (Batch 1 bridge seat / Batch 2 wiring);
  until it lands, the pipeline above is proven only through the
  exporter stage.
- **Native package / renderer** — unchanged, owned by the
  adapter and binary seats.

## 2. The skeleton report (`talerender-skeleton/1`) — bridge input contract

Top level: `format`, `project {dir, title}`, `timing`, `cast`,
`instances`, `sets`, `segments`, `cuesFromFile`, `extracted`,
`owes`, `assumptions`.

- `timing` — `fps/width/height` (+ `metaSource` per field: `cli` /
  `src/core/lib.ts` / `native/film.json` / `skill default`),
  `durationS`, `totalFrames = ceil(durationS × fps)`, and the
  script's `lead/gap/outro/voice/speed/music`, line/word counts.
- `cast` — the `LOOK` registry from `src/shots.ts`:
  `{id, look, lookRaw}` per character. `look` values are raw
  (strings, string arrays, numbers, booleans); ToonLook semantics
  are the bridge's business.
- `instances` — every `person('id', opts?, shade?)` instance:
  `{var, character, optionsRaw, shade, set}`; `set` from
  `<SET>.scene.add(<var>.group)` where statically visible.
- `sets` — every `const X = (() => {…})()` set block:
  `{id, label, kind}`; label from the `SET n · …` banner comment,
  kind `interior` (WD.interior) / `exterior` (WD.sky) / `unknown`.
- `segments` — one per shot, in time order:
  `{shotId, note, lines, atExpr, atS, endS, durationS,
  startFrame, endFrame, set, fov, lookRef, atmospheres, staged,
  camera, cameraDynamic, cues}`.
  - Times are resolved by a static evaluator for the timing.ts
    expression grammar: `cut(i)` (= `L(i) − 0.06`, 0 for i = 0),
    `L(i)`, `Le(i)`, `Wd('word', line)` (timing.ts `norm` +
    prefix rule over `data/words.json`), block-local `const`
    bindings (comma-chained declarations included), and `+`/`−`
    chains of those. Anything outside the grammar resolves to
    `null` and is named in `owes` — never guessed.
  - `lines` — narration lines whose start falls in the shot,
    plus the line whose speech interval contains the shot start
    (mid-line splits: S05b → `[4]`).
  - `camera` — `{from, to, at, at2}` literal V3 endpoints, only
    when the shot's first `shoot()` call is fully literal;
    `cameraDynamic: true` marks rig/mix/helper-driven cameras.
  - `staged` — keys set `true` in the shot's first
    `<x>Stage({...})` visibility call (characters and props).
  - `cues` — the shot's authored `cue(...)` calls with resolved
    `tS` where the grammar allows (`tS: null` otherwise).
- `cuesFromFile` — `data/cues.json` cues verbatim (the cue record
  of authority for mixing; per-shot cue calls are the staging
  view of the same events).
- `owes` — the still-manual remainder, machine-readable (§4).
- `assumptions` — the shape assumptions the extraction rests on
  (§5), stated in the report itself so a bridge author sees them
  at the point of use.

## 3. What v2 extracts vs v1

| Concern | v1 (fixture manifest) | v2 (real project) |
|---|---|---|
| Input | hand-written `project-source.json` | the project folder itself |
| Meta | flat fields in the manifest | `script.json` title; fps/W/H resolved CLI → `src/core/lib.ts` constants → `native/film.json` → skill defaults (30/1080/1920), source recorded |
| Assets | manifest list, existence-checked | curated tree walk: `script.json`, all of `data/` + `fonts/`, `src/shots.ts`, and the `native/` tree when present; kind by extension (image/audio/data); `node_modules`, `.git`, `build/`, `review/`, `renders/`, `vendor/` never walked |
| characters | pass-through / count placeholders | LOOK registry parsed from `src/shots.ts` |
| scenes | pass-through / count placeholders | set blocks (id/label/kind) |
| shots | pass-through / count placeholders | shot id/note + resolved at/end/duration + line coverage + set + fov |
| captions | pass-through / count placeholders | per-line `{line, text, caps, start, end, words}` from `lines.json` + `script.json` + `words.json` |
| audio | pass-through / count placeholders | `{type:'track', id, path}` for `data/narration.wav` / `data/mix.wav` when present + `{type:'cue', cueType, t, gain?, dur?}` per `data/cues.json` cue (the cue's own `type` is renamed `cueType` so the discriminator is never clobbered) |
| Validation | C4 rules on the manifest | the same C4 rules (meta positive-u32, closed asset kinds, relative/no-`..` paths, every missing asset listed, batched problems, exit 2, no partial write) applied to the assembled document, PLUS project-level rules: script/lines line-count agreement, duplicate shot ids, non-monotonic shot times, malformed timing files |
| Extra output | — | the skeleton report (§2) |

The emitted document is the same C4 shape in both tools and
passes `projectfile::load_project` under the same rules; section
entries remain opaque to the Rust loader (counted, not
deep-validated) exactly as C4 specifies.

## 4. The still-manual remainder (stated honestly)

The skeleton's `owes` list is the contract for this; summary:

- **draw-body-animation (blocking)** — per-frame pose, motion,
  blink, lip-sync and prop animation inside `draw()` bodies is
  code. Nothing transcribes it; the bridge authors §15 scene/clip
  data per segment from `shots.ts`.
- **scene-geometry (blocking)** — full 3D set construction
  (meshes, materials, lighting, world/props calls, project-local
  builders, canvas props) is extracted only as set identity.
- **caption-strips (blocking)** — strip PNGs + width/fit come
  from canvas text rendering (`core/captions.ts`); the exporter
  carries text + timing only.
- **native-package-absent (blocking, when applicable)** — a
  project without `native/` needs the whole package authored;
  `package_bridge.py` cannot do it (pre-authored data only).
- **camera-motion (partial)** — easing curves, dynamic endpoints
  (flagged per shot), focus pulls are runtime values.
- **word-anchors (partial)** — film-data anchor names are
  hand-chosen; full word timings ride as an asset.
- **atmosphere-grade-resolution (partial)** — preset names are
  extracted; the numeric grades they resolve to live in
  `core/env.ts` at runtime.
- **unresolved-shot-times / unresolved-cue-times (when present)**
  — expressions outside the static grammar are listed by name.
  On the real Onoda run: zero unresolved shot times; exactly one
  unresolved cue (S07's loop-offset `calT0 + o` tocks — the file
  cues remain authoritative).
- **shot-segment-mapping (partial, when present)** — shots and
  native segments are not always 1:1 (one line split across two
  shots; a shot spanning a line boundary). The bridge fixes final
  segment boundaries from the shot tiling + line coverage given.

Extraction limits worth knowing: staging facts come from the
shot's FIRST `shoot()` / `<x>Stage({...})` call in its block;
`person()` instances added to a scene by array/`forEach` (rather
than a direct `<SET>.scene.add(var.group)`) get `set: null`;
LOOK entries with nested-object values would lose the nested
part (none exist in the template or Onoda — values are flat).

## 5. Assumptions

Stated in every skeleton report, repeated here:

1. Shot start expressions follow the `timing.ts` grammar
   (`cut(i) = L(i) − 0.06`). A project that redefines `cut()` in
   `shots.ts` invalidates resolved times — they are reported
   alongside their source expression (`atExpr`) so the bridge
   can check.
2. One `add({...})` block = one shot, file order = time order
   (`main.ts` sorts by `at` at boot; the exporter validates the
   resolved order never runs backwards and fails the export if
   it does).
3. Interchange section entries are envelope data for
   `projectfile.rs`; their per-entry shape (§3) is this seat's
   bridge contract, not a Rust-validated schema.
4. The skeleton's consumer is a future general bridge builder;
   `package_bridge.py` does not consume it yet (§1).

## 6. Verification (this seat, 2026-11-15)

`node tools/export/export_project_v2.selftest.mjs` —
**PASS 40 FAIL 0**, covering: synthetic-project success (meta
from lib.ts, all sections, manifest, skeleton tiling/camera/
cues/instances/owes), missing-`lines.json` failure (exit 2, no
outputs), backwards-shot-times failure (exit 2, no output),
`--fps 0` failure (exit 2, names `meta.fps`), and the
real-project run below. v1's self-test still passes untouched
(PASS 15 FAIL 0).

Real-project run (copy of
`~/faceless-shorts/the-soldier-who-didnt-know` in a temp dir;
the original tree's recursive listing — names, sizes, mtimes —
is compared before/after and is unchanged):

- exit 0; title "The Soldier Who Didn't Know The War Was Over";
  meta 30 fps / 1080×1920, all three sourced from
  `src/core/lib.ts`.
- 112 assets, 5 characters (onoda, compA, compB, suzuki,
  taniguchi), 3 sets (JG/SH/BK), 14 shots, 12 captions, audio =
  2 tracks + 9 file cues.
- All 14 shot times resolved statically; skeleton segments tile
  `[0, 1811)` frames with boundaries matching the project's own
  authored bridge manifest (`native/film.json`) — including the
  intra-line splits S05a/S05b at 24.56 s (frame 737) and
  S08/S08b at 40.36 s (frame 1211), which the exporter derives
  from `Wd('thought', 4) − 0.12` and `Wd('refused', 7) − 0.1`
  rather than from any native data.
- Per-shot line coverage matches the authored manifest's
  segment→lines mapping (S06 → [5,6], S05a/S05b → [4], …).
- `owes` on this project: draw-body-animation, scene-geometry,
  camera-motion, caption-strips, word-anchors,
  atmosphere-grade-resolution, unresolved-cue-times (the one
  loop-offset tock), shot-segment-mapping. No
  unresolved-shot-times, no native-package-absent (the project
  is already bridged).

## 7. Usage

```
node tools/export/export_project_v2.mjs <project-dir> <out.json> \
    [--skeleton <path>] [--fps N] [--width N] [--height N]
node tools/export/export_project_v2.selftest.mjs   # exit 0 = pass
```

Node ≥ 18, `node:` builtins only, no npm dependencies. Exit 0 =
both files written + one summary line on stdout; exit 2 = usage
error or any validation problem (all problems on stderr, no
output written).
