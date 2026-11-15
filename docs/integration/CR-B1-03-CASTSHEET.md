# CR-B1-03 — Native castsheet (Chrome-replacement Batch 1, Seat 3)

Contract: `CHROME-REPLACEMENT-CONTRACTS.md` §1 row 6 (`castsheet`,
REPLACE-NATIVE). Deliverables (new files only; nothing existing was
edited, nothing committed):

| File | Role |
|---|---|
| `template/tools/native_modes/castsheet_mode.mjs` (skill) | The mode: `castsheetMode({ projectRoot, outdir, kind, talerenderBin, log })` |
| `template/tools/castsheet_native.py` (skill) | Stdlib-only generator: per-item scene files + `cast-manifest.json` |
| this note | Staging-format spec, lead wiring, proof, gaps |

## 1. What the Chrome mode does (extracted from code)

Sources: `template/src/castsheet.ts`, `template/castsheet.html`,
`tools/capture.mjs` `castsheetMode` (all in the skill template).

- Page: a bare 1080×1920 canvas (`#cv`), no three.js scene, no film.
  `CAST.draw(i)` stages item *i* with `drawToon` on a plain set;
  `CAST.items()` enumerates the catalog; `CAST.label(i)` overpaints
  the label for the contact-sheet copy.
- **Enumeration** (`CAST_ITEMS`, in order): `POSE_NAMES` from
  `core/poses.ts` `POSES` (17) → kind `pose`; `MOODS` from
  `core/toon.ts` (19) → kind `mood`; `Object.keys(LOOK_PRESETS)` from
  `core/looks.ts` (5: kid, adult, tall, elder, stocky) → kind `look`;
  `Object.keys(PROPS)` from `core/toon.ts` (36) → kind `held`.
  **Total 77.** The library's other `held` items — `props2.ts`
  (nest, bandage, dropper, splint, berrySprig) and `butterfly.ts`
  (butterfly) — are **not** in `CAST_ITEMS`; the Chrome castsheet
  never rendered them either (parity note, §5 G6).
- **Staging** (`stage()`): backdrop fill `#EFE7D6` over the whole
  canvas; floor band `#DCCBA6` from `FLOOR_Y = 1560` to the bottom;
  a soft shadow ellipse (centre (540, 1578), r 280×40,
  `rgba(60,45,20,0.18)`); the figure is drawn with
  `drawToon` in its 512×1024 stage (feet at stage y 1000), scaled by
  `s = 1.35 × (heightM / 1.75)` and translated so the feet land on
  `FLOOR_Y`, horizontally centred — a 1.75 m figure occupies
  ≈ 1382 px ≈ 72 % of frame height.
- **Per-kind staging**: `BASE_LOOK = { body: '#7A5FBF', hat: 'none' }`
  throughout. pose → `stage(BASE_LOOK, { ...POSES[id] })`; mood →
  `stage(BASE_LOOK, { mood: id })`; look →
  `stage({ ...BASE_LOOK, ...preset.look }, preset.pose ? { ...POSES[preset.pose] } : {}, preset.height)`;
  held → `stage(BASE_LOOK, { holdR: propFn(id), armR: [72, -38], armL: [14, 8] })`,
  where `propFn` calls factory props (declared ≤ 2 params: book,
  cash, paper, sign, map) with **no arguments** — i.e. book closed
  ('RULES'), cash ×1, paper 'NEWS', sign 'SALE'.
- **Outputs**: per item, the *unlabelled* frame →
  `<outdir>/<kind>-<id>.jpg` (JPEG q0.92) — this is the library
  preview name (`library/previews/<kind>-<id>.jpg`, "the name
  castsheet writes", `scripts/library_check.py`); then `label()`
  paints a 92 px black bar + `"<kind> — <id>"` in 700 54 px sans at
  (22, 62) and that copy (q0.8) is tiled by ffmpeg
  `scale=270:480, tile=4×rows, padding=4, color=black`, q:v 3 →
  `<outdir>/castsheet.jpg`. `[kind]` filters to one kind; no items
  of that kind is an error.

## 2. Native staging format (what the generator emits)

Per item, one **scene-format v2** file (`--scene-file` is the
binary's per-still surface; the character node is §14.3 of
`docs/scene-format.md`), rendered at 1080×1920:

- **Backdrop/floor**: two textured meshes with the `solid`
  generator carrying the *exact* Chrome hexes (textured draws are
  unlit, so the hexes land byte-exact — verified §4): a `quad`
  16×10 (y0 −3) at z = −1.5 in `#EFE7D6`; a `flat-quad` 16×12 at
  y = 0 in `#DCCBA6`. `clear` also carries `#EFE7D6`.
- **Figure**: one `character` node at `[0, 0, 0]`, `perspective:
  false`, height = the item's heightM (look presets 1.35–1.95,
  everything else 1.75). The node's own blob shadow replaces the
  painted ellipse. Look/state are the transcribed tables (below);
  `sweat`/`tears` are converted number→bool (the scene loader's
  `truthy()` semantics vs TS draw thresholds).
- **Camera**: eye `[0, 0.78, 3.2]`, target `[0, 0.78, 0]`, fovY 42 —
  a 1.75 m figure measures ≈ 71 % of frame height (Chrome: 72 %)
  and the feet land at ≈ 0.83 of frame height (Chrome floor line:
  0.8125). This is *semantic* parity (same look, state, height,
  colours, proportions), not pixel parity: the native floor is a
  real 3D plane (its horizon sits where perspective puts it) where
  Chrome paints a flat band, and the shadow is the engine blob,
  not the 2D ellipse.
- **Held items**: state is the transcribed `armR [72, -38]`,
  `armL [14, 8]`, plus `holdR: { plate, x0, y0 }` **only** when a
  plate PNG exists (see G1). Plate origins for the two plates the
  film bridge proved are transcribed from `film/gen_assets.py`
  (paper (−200, −270), book (−170, −180)); any other plate defaults
  to (−200, −200) and is flagged `"origin": "assumed"` in the
  manifest. Texture `file` paths are `../plates/<id>.png` — texture
  files resolve relative to the *scene file's* directory (§8),
  and scenes live in `<work>/scenes/` (this cost one failed proof
  render before it was fixed; it is now a comment in the code).

**Transcribed tables** (in `castsheet_native.py`, each with its
source named in a comment): `POSES` (17, `core/poses.ts`, verbatim
including `sweat: 1` on exhausted), `MOODS` (19, `core/toon.ts`),
`LOOK_PRESETS` (5, `core/looks.ts`), `PROP_IDS` (36, `core/toon.ts`
`PROPS`) + factory defaults, `HELD_STATE`, `BASE_LOOK`.

**Manifest**: `<work>/cast-manifest.json`
(`{ format: "talerender-castsheet", version: 1, source, staging,
counts, items[] }`) — every item records kind/id/name/look/state/
height/scene, plus `plateMissing`, `nativeMoodFallback` (G2) and,
for cast items, `droppedLookFields` (G4) when present. The work
dir is `<outdir>/.native-castsheet/` so the data that produced a
sheet stays auditable beside it.

**Project cast source**: `--project <dir>` parses the
`const LOOK: Record<string, ToonLook> = { … }` literal in
`src/shots.ts` (the `registerCast` source shape; flat string/bool/
array/number values only — anything richer fails loudly) or
`--cast-json` takes `{"characters": [{name, look, height?}]}`.
Cast items are kind `cast`, neutral standing state, height 1.75
unless given. They are a project's own cast sheet, **not** library
previews (the library has no `cast` kind).

## 3. The mode (`castsheet_mode.mjs`) and lead wiring

Flow: resolve the binary (argument → `TALERENDER_BIN` →
`<project>/native/talerender` → repo checkout → PATH; a bad
explicit path is a hard error, mirroring `capture_native.mjs`) →
run the generator (`python3 tools/castsheet_native.py`) → per
item, `talerender --scene-file <scene> --out <frame.png>
--width 1080 --height 1920` → ffmpeg PNG→`<kind>-<id>.jpg`
(`-q:v 2`, mirroring q0.92) → labelled tile copy (ffmpeg
`drawbox` 92 px black + `drawtext` `"<kind> — <id>"`, 54 px, at
(22, 14), font from the project's `fonts/`; unlabelled with a
warning when no font/drawtext, mirroring the native sheet mode)
→ tile with the Chrome filter verbatim
(`scale=270:480,tile=4×rows:padding=4:color=black`, `-q:v 3`) →
`<outdir>/castsheet.jpg`. PNG frames are the render (cleanliness
contract B); JPEG exists only as the delivery/tiling format,
exactly as the sheet mode treats it. Skipped items (G1) are
logged per item and summarised by name; a run that renders
nothing throws. All failures throw; the dispatch maps a throw to
exit 1.

**Wiring (lead's act, not done here)**: Seat 10's
`native_modes/mode_dispatch.mjs` already delegates this mode —
it dynamically imports `./castsheet_mode.mjs` and calls
`castsheetMode({ projectRoot, outdir, kind, talerenderBin, log })`
with `outdir = rest[0] || 'review/cast'`, `kind = rest[1] || ''`.
The lead only needs to (a) route `castsheet` through
`dispatchChangedMode` in `capture_native.mjs` before the REFUSALS
check (per CR-B1-10's wiring note) and (b) delete the `castsheet`
entry from `REFUSALS`. No change to this module is needed. One
behaviour to bless at integration: `kind === 'cast'` is a
mode-level extension with no Chrome counterpart (Chrome's second
argument is a catalog-kind filter); it routes the generator to
`--project <projectRoot>`.

## 4. Proof (lavapipe, correctness only; binary = `~/talerender-bin/talerender`, v1.0.1 CI binary)

Evidence lives OUTSIDE both repos in
`~/workspace/talerender-evidence/castsheet-b1/` (14 PNG frames,
18 JPEGs). All runs through `castsheetMode` itself:

| Run | Result |
|---|---|
| Fixture cast (3-char `shots.ts` LOOK literal, kind `cast`) | 3/3 rendered + sheet, 32.6 s |
| **Onoda project cast** (`~/faceless-shorts/the-soldier-who-didnt-know`, kind `cast`) | **5/5 rendered** (onoda, compA, compB, suzuki, taniguchi) + sheet, 44.4 s |
| Catalog kind `look` (5 items) | 5/5 + sheet, 46.9 s — kid/tall heights and the elder `elderStoop` pose all visibly correct |
| Catalog kind `held` with a synthetic `book` plate present | book rendered (plate composited at the raised right hand, arms at the transcribed [72, −38]); other 35 skipped **by name**; sheet of 1 |
| Catalog kind `held`, no plates | fails loudly naming the plates remedy (no sheet, no silent pass) |
| Generator, full catalog | 77 items (17 pose / 19 mood / 5 look / 36 held), 36 plate-missing |
| Generator, Onoda `--project` | 5 characters parsed exactly (incl. taniguchi's vest/bowler/tie/glasses/mustache) |

Pixel checks on the Onoda taniguchi frame: backdrop pixel =
(239, 231, 214) = `#EFE7D6` exactly; floor pixel = (220, 203, 166)
= `#DCCBA6` exactly. Sheet hashes (sha256):
fixture-cast `a4c7e5de04f7361c…`, fixture-look
`d7d4a69753090503…`, onoda-cast `c0148c5ba5707ba…`,
fixture-held `61be5d73330a2de2…`. The Onoda sheet was eyeballed:
caps + cap colours, compB's scarf, suzuki's glasses, taniguchi's
bowler/vest/tie/mustache all render as the registered looks.

## 5. Gaps — candidate EXCEPTIONs (evidence, not silence)

- **G1 — held props have no native painters (36 of 77 items).**
  `src/toonfull.rs` takes a held prop ONLY as a pre-rendered RGBA
  plate (`Plate`); there is no port of any `PROPS` PropFn. The
  film bridge proved exactly four plates (paper, leaflet, book
  open/closed, coconut) via the PIL script `film/gen_assets.py`,
  which is film-specific, not a catalog generator. Until a seat
  transcribes the 36 PropFn painters into a plate generator, kind
  `held` is a **candidate EXCEPTION**: the mode renders held items
  that have plates and names the rest as skipped. The Chrome
  semantics (factory defaults, grip arms) are fully transcribed
  and ready for that seat.
- **G2 — moods `puzzled` and `deadpan` do not exist natively.**
  TS `MOODS` has 19; `toonfull.rs` `Mood::from_str` has 17 and
  silently maps both to neutral. The generator flags them
  (`nativeMoodFallback: "neutral"` in the manifest); do NOT
  generate library previews for these two from the native path
  until the painter gains the variants — the frame would be a
  neutral face under a puzzled/deadpan label.
- **G3 — hats/necks/patterns beyond the film's set are silently
  unpainted by `toonfull.rs`.** Hats: only `cap` and `bowler`
  have paint branches (`_ => {}` for boater, beanie, visor,
  tophat, bun, flower, bow, mailcap). Necks: only `tie`/`scarf`
  (not `bowtie`/`collar`). Patterns: only `plain`/`vest` (not
  check/stripes/overalls/apron/stamps). No catalog item in the
  current 77 hits this (BASE_LOOK hat is `none`; look presets
  change only `kid`/height/pose), but a project cast using e.g. a
  tophat would lose it without a warning from the binary — the
  manifest's `droppedLookFields` covers dropped *keys*, not
  unpainted *values*; closing that needs a painter-side census.
- **G4 — look fields the scene loader drops.** `scenefile.rs`'s
  character branch reads body/head/shade/pattern/patCol/hat/
  hatCol/band/neck/neckCol/extra(glasses, mustache only)/kid/
  mustacheCol. Dropped: `patCol2`, `dots`, `stamps`, `hairCol`,
  extras freckles/blush/pencil/monocle, and the v6 appendage
  fields (ears/tail/wing/hair/scarf-streamer — `FullLook` has no
  such fields at all). The generator strips dropped keys from the
  scene (so the file never claims them) and lists them per cast
  item in the manifest.
- **G5 — framing is semantic, not pixel, parity** (§2): 3D floor
  perspective vs the flat band; engine blob shadow vs the painted
  ellipse; figure ≈ 71 % vs 72 % of frame height. Acceptable for
  a catalog/preview tool; stated so nobody A/Bs these against
  Chrome previews expecting byte kinship.
- **G6 — parity note, not a gap**: the library `held` census is
  42, the castsheet census is 36 — `props2.ts`'s five items and
  the butterfly were never castsheet items in Chrome either.
  Where their previews come from is a Batch 2 library question,
  not something this mode invents an answer to.

## 6. Notes for Batch 2 (library previews)

`castsheetMode` with `outdir = <skill>/library/previews` and no
kind filter produces the catalog previews under the exact names
`library_check.py` validates, minus G1/G2 items (41 of 77 fully
native today: 17 pose + 17 mood + 5 look + held-with-plates).
`--strict-previews` will keep failing on the held set until G1
closes; report that state honestly rather than rendering
prop-less figures under prop names.
