# F3BC-W88 — gap-fix F3, classes F3-b + F3-c: dispositions and the either/or

Seat W88, Wave 2 Batch 9. Deliverables, **new files only, nothing
committed, no existing file edited**:

- this note — the F3-b per-item dispositions (all 19, prose
  verbatim) and the F3-c either/or analyses with recommendations
- `docs/integration/f3bc/s10.scene.json` — staged: F3-b keys removed
- `docs/integration/f3bc/s11.scene.json` — staged: F3-b key removed
- `docs/integration/f3bc/s12.scene.json` — staged: F3-b key removed
- `docs/integration/f3bc/bookshop.scene.json` — staged: 14 texture
  `note`s removed; the two `materials` nodes rewritten in the s10
  decomposed form; `lights.key.near` **kept** (consume recommended,
  §3.2)
- `docs/integration/f3bc/unknown_fields_baseline.json` — the
  post-landing ratchet baseline (24 → 3), staged

Verified at HEAD `c440d90`. Method note: all four scene files
round-trip byte-identically through `json.dumps(indent=1,
ensure_ascii=False)` (bookshop with a trailing newline), so the
staged files were produced by structural edits + re-dump and then
verified by recursive diff against the originals — the diffs are
exactly the intended changes and nothing else (printed in §4).

Starting state: STRICTNESS-W42 §6's register as lowered by
F3A-STRIP-W74 — **24 occurrences / 9 paths / 7,043 tolerated**:
F3-a-kept 2 (`fps`, `frames`), F3-b 19, F3-c 3.

---

## 1. F3-b — the 19 prose items

Disposition vocabulary, per the seat brief:

- **CITE** — the note's claim is already documented elsewhere
  (document/section named); the disposition is
  **strip-with-citation-recorded**: the key leaves the data, the
  citation lives here and in the landing commit.
- **KEEP** — the note carries information recorded nowhere else;
  its home should be a doc: disposition **move-to-doc**, target
  named.
- **STRIP** — noise, or duplicated by the entry's own adjacent
  data fields.

**Headline: all 19 items are CITE. Zero KEEP, zero bare STRIP.**
The judgement class existed because W42's register rule requires a
citation *in the register* and nobody had done the per-item
citation work; the corpus's own documentation (ASSUMPTIONS.md, the
sets README) and — for two painter recipes — the `src/scene.rs`
painter headers already record every claim. The two code citations
are flagged as such (B8 of the sets README rules painter-internal
behaviour "pinned by the source, not by this file", and the
painters' source transcriptions live in the code).

### 1.1 `scenes/film/s10.scene.json` — 3 items

**B-1. `$.textures[3].motifLaw`** (texture `wallpaper-ceiling`):

> `motif = shade(ceiling, 1.25) — three.js Color multiplyScalar in linear space (world.ts interior ceil)`

**CITE** — `scenes/film/ASSUMPTIONS.md`, S10 section, "Sources
followed over film.rs where they differ" item 3, **Ceiling motif
colour** (line 2453): the source `shade()` is THREE.Color
multiplyScalar in linear space yielding `#413427`; film.rs paints
`#493A2B` (sRGB helper) — and the entry itself carries the source
value in its `motif` field (`#413427`). Strip: the law survives in
ASSUMPTIONS, the value survives in the data.

**B-2. `$.textures[4].note`** (texture `planks-floor`):

> `the source sets map.rotation = PI/2 on the floor material (world.ts interior); film.rs emulates it with the swapped repeat [3.6, 3.2]`

**CITE** — ASSUMPTIONS.md, S10, "Inexpressible in format v2" item
1, **Floor map rotation π/2** (line 2397): records the source
`map.rotation = π/2`, its carriage as `rotation` on this very
entry (the field is in the data: `1.5707963705062866`), and
film.rs's emulation "by swapping the repeat to [3.6, 3.2] on its
floor box". Strip.

**B-3. `$.textures[15].note`** (texture `view`):

> `the window view is a MeshBasicMaterial colour plane in the source (toneMapped: false) — a flat colour, no painted depth (INVENTORY BK row)`

**CITE** — ASSUMPTIONS.md, S10, "Inexpressible" item 4,
**MeshBasicMaterial surfaces** (line 2411): the window view plane
(`#BFE4F7`, toneMapped: false) is unlit in the source; film.rs
renders it as a basic quad; the colour enters via this entry's
`solid` generator + `color` field. Strip.

### 1.2 `scenes/film/s11.scene.json` — 1 item

**B-4. `$.nodes[81].note`** (node `rifle (on ground)`, type group):

> `makeRifle() (shots.ts): modelled along +Y with the grip at the origin (butt -0.44, muzzle +0.68); castShadow/receiveShadow on every part in the source (the format carries no per-node shadow flags). The group's static transform is its state at first visibility (drop start, u = 0); before dropT the node is invisible and the SAME rifle renders as Onoda's hand attachment.`

**CITE** — ASSUMPTIONS.md, S11 section, **"The rifle: attachment
→ ground node"** (line 2621): the five makeRifle parts, the
creation transform, the static transform defined as the state at
first visibility (drop start, u = 0), the visible window from
dropT, and the same rifle as Onoda's hand attachment before
dropT — all in more detail than the note. The shadow-flag clause
is "Inexpressible" item 10, **Per-node shadow flags** (line
2687), verbatim in substance. The +Y layout with butt −0.44 /
muzzle +0.68 follows from the part offsets tabulated in the S12
section's "Rifle (ground node)" (line 2110): stock box centred
[0, −0.17, 0] × 0.52 long → butt ≈ −0.44; front sight at
[0, 0.68, 0] → muzzle +0.68. Strip.

### 1.3 `scenes/film/s12.scene.json` — 1 item

**B-5. `$.nodes[78].note`** (node `rifle (on ground)`, type group):

> `makeRifle() (shots.ts:74) at its resting transform: position/rotation from the construction (shots.ts:256) with the S11 drop animation settled (y 0.32 -> 0.07, rotation.z -> PI/2) and re-pinned verbatim by the S12 draw (shots.ts:531). Modelled along +Y, grip at the group origin (butt -0.44, muzzle +0.68).`

**CITE** — ASSUMPTIONS.md, S12 section, **"Rifle (ground node)"**
(line 2110): the five parts (`shots.ts:74–83`) with exact
geometry, the resting transform [0.62, 0.07, −0.1] / [0, −0.5,
π/2] derived from the construction (`shots.ts:256`), the S11
drop settling y 0.32 → 0.07 and rotation.z → π/2, and the S12
draw re-pinning both verbatim (`shots.ts:531`) — every clause of
the note, expanded. Strip.

### 1.4 `scenes/sets/bookshop.scene.json` — 14 texture `note`s

The sets README's bookshop inventory (lines 245–283) plus its
B-decisions are the citing document throughout. Each entry's own
fields (`base`/`motif`/`repeat`/`rotation`/`label`/`text`/`bg`/
`fg`/`font`/`canvas`) carry the values; the notes restated the
source calls.

**B-6. `$.textures[0].note`** (`wallpaper-walls`):

> `back-wall segments: T.wallpaper(wall, motif, [len/1.4, h/1.4]) at len = (w-ww)/2 = 2.4 (world.ts:302). The above-window band calls wp(ww) at len 3.2 -> repeat [2.2857143878936768, 3] (world.ts:303); see the README wallpaper-repeat note`

**CITE** — sets README, **Wallpaper repeat note (rows 3–6)**
(line 272) + inventory rows 3–6: `wp(len)` = `[len/1.4, h/1.4]`,
segments (len 2.4) `[1.7142857313156128, 3]`, band (len = ww =
3.2) `[2.2857143878936768, 3]` — the note itself defers to that
paragraph. Strip.

**B-7. `$.textures[1].note`** (`wallpaper-sides`):

> `side walls: clone repeat set to [(depth+2)/1.4, h/1.4] (world.ts:309)`

**CITE** — sets README inventory rows 13–14 ("the source sets
the map repeat on the shared clone to [(depth+2)/1.4, h/1.4] =
[6.428…, 3]") + **B5** (line 346). The entry's `repeat` carries
[6.4285712242126465, 3]. Strip.

**B-8. `$.textures[2].note`** (`wallpaper-ceiling`):

> `ceiling: T.wallpaper(ceiling, shade(ceiling, 1.25), [w/1.2, (depth+2)/1.2]) (world.ts:313); motif is the source shade() result, computed with three.js colour management`

**CITE** — sets README inventory row 19 (line 262): "motif =
`shade(ceiling, 1.25)` = `#413427` (computed with three.js
colour management), repeat [w/1.2, (depth+2)/1.2]". The entry
carries `motif: #413427` and the repeat. Strip. (Same law as
B-1, documented independently in both places.)

**B-9. `$.textures[3].note`** (`planks-floor`):

> `floor: T.planks(floor, [w/2.5, (depth+2)/2.5]) then texture rotation = PI/2 about the centre (world.ts:296-297)`

**CITE** — sets README inventory row 1 (planks `#7A5230`,
repeat [3.2, 3.6], texture rotation π/2) + **B5** (rotation π/2
about its centre, `world.ts:297`). The entry carries `repeat`
and `rotation` as fields. Strip.

**B-10. `$.textures[4].note`** (`planks-wainscot-back`):

> `back-wall wainscot strips: wain(2) = [len/2, 0.5] — len hardcoded to 2 at world.ts:311, whatever the segment length`

**CITE** — sets README inventory rows 17–18: "`wain(2)` =
[len/2, 0.5] → repeat [1.0, 0.5]; the len is hardcoded 2, not
the segments' 2.4" — including the warning, which is the note's
only non-obvious content. Strip.

**B-11. `$.textures[5].note`** (`planks-wainscot-sides`):

> `side wainscot: wain(depth+2) = [len/2, 0.5] (world.ts:310)`

**CITE** — sets README inventory rows 15–16: "`wain(depth+2)` =
[4.5, 0.5]". Strip.

**B-12. `$.textures[6].note`** (`planks-wainscot-under-window`):

> `the under-window board's own wain(ww) call: [ww/2, 0.5] with ww = 3.2 (world.ts:303)`

**CITE** — sets README inventory row 5: the 3.2-wide
under-window board, "**wainscot planks**, `wain(ww)` = repeat
[1.6, 0.5]". Strip.

**B-13. `$.textures[7].note`** (`planks-counter`):

> `counter body: T.planks('#7A3E24', [1, 2]) (world.ts:317)`

**CITE** — sets README inventory row 21: "body planks
`#7A3E24` [1, 2]"; also duplicated by the entry's own
`base`/`repeat`. Strip.

**B-14. `$.textures[8].note`** (`planks-desk`):

> `desk wood: T.planks('#6B3E26', [1, 1]) (props.ts:163)`

**CITE** — sets README inventory row 27: "wood planks
`#6B3E26` [1, 1]"; duplicated by the entry's fields. Strip.

**B-15. `$.textures[9].note`** (`felt-counter`):

> `counter top: T.felt('#1F5A45') (world.ts:318)`

**CITE** — sets README inventory row 21: "top felt `#1F5A45`";
duplicated by the entry's `base`. Strip.

**B-16. `$.textures[10].note`** (`felt-desk`):

> `desk top: T.felt('#2E5A44') (props.ts:164) — a different green from the counter felt`

**CITE** — sets README inventory row 27: "felt `#2E5A44` (not
the counter's green)" — the contrast warning, which is the
note's point, is in the README in so many words. Strip.

**B-17. `$.textures[11].note`** (`picture`):

> `the framed line-drawing: paper #E9DCC0, ink #3A2A22 quadratic hill strokes, sun disc #C9603F r 22 at (190, 60) (world.ts:322)`

**CITE — to code** (flagged: this citation points at code, not
a doc): `src/scene.rs`, registry painter `paint_picture` —
its header comment transcribes the recipe 1:1 (paper
`#E9DCC0`; ink `#3A2A22`, the two quadratic hill strokes; sun
`#C9603F` disc r 22 at (190, 60)) and the `PICTURE_PAPER` /
`PICTURE_INK` / `PICTURE_SUN` constants pin the colours. Sets
README **B8** (line 366) rules painter-internal behaviour
"pinned by the source, not by this file", which is why no doc
carries the recipe. (The note's `world.ts:322` is the frame
node's call site per README row 22; the painter header cites
`world.ts:319` for the canvas recipe — a citation nuance, not
a content gap.) Strip.

**B-18. `$.textures[12].note`** (`sign`):

> `signBoard(signText, { w: 2.2, h: 0.4, bg: '#3A2A2A', fg: '#F2E6CF' }) (world.ts:324); font rule: 700 Caladea/Georgia serif starting at 40 px, shrunk by 2 until the text fits 86% of the board width (world.ts:143)`

**CITE** — sets README inventory row 23 (line 266) quotes the
`signBoard('TANIGUCHI BOOKS', { w: 2.2, h: 0.4, bg: '#3A2A2A',
fg: '#F2E6CF' })` call, and **B9** (line 380) records the
shrink rule (from 40 px, by 2, until the text fits 86% of the
440 px canvas, `world.ts:143–147`). The weight/stack detail
(`700 … Caladea, Georgia, serif`) and the measured outcome
(no shrink step fires; fs = 40) are recorded in
`src/scene.rs`'s `paint_sign` header — a code citation, as
B-17. The entry carries `text`/`bg`/`fg`/`font`/`canvas`.
Strip.

**B-19. `$.textures[13].note`** (`parcel-plain`):

> `parcel(w, h, d, false): base #C8935A, 30 fibre streaks #B07A48 (rnd-seeded), tape cross #E9D7A5; the label block is NOT drawn at label:false (props.ts:159)`

**CITE — to code** (as B-17): `src/scene.rs`, registry painter
`paint_parcel` — header + body transcribe the whole recipe
(base `#C8935A`; 30 streaks `#B07A48`, hash-placed; tape cross
`#E9D7A5`; label block only at `label = true`). Sets README
**B8** records the 30 rnd-seeded streaks and inventory rows
28–29 the `label: false` staging; the entry carries `base`
and `label: false`. Strip.

### 1.5 Landing shape for F3-b

One data commit applying the staged s10/s11/s12/bookshop files
(the bookshop file in that commit also carries the §2
decomposition), with the landing message naming this note as
the citation record. No register change is needed afterwards:
the 19 paths leave the baseline by shrinkage, which the
ratchet passes and prints (§4).

---

## 2. F3-c (i) — `nodes[].materials` ×2 (bookshop nodes 11, 12)

The data (verbatim shape): nodes `picture frame` (nodes[11])
and `sign board` (nodes[12]) are single box meshes whose only
material data is a six-entry `materials` array in three.js
BoxGeometry group order [+x, −x, +y, −y, +z, −z] — five entries
`{color, roughness: 0.85}` (frame `#C9A13B`, board `#302222`)
and the +z entry `{map: "picture"|"sign", roughness: 0.85}`.
The singular `material` is absent.

**Provenance: hand-authored intent, not dead generator
output.** The set file was transcribed first-hand from source
(sets README, "Provenance (bookshop)"), and **B2** (line 326)
declares the arrays deliberately: "the corpus's first
material-array meshes… carried as data (unknown-field law,
§1) for the film/PBR path", noting the loader's §3.3
type-check covers only the single `material` form today. Any
honest disposition starts from that: stripping overrides a
recorded authoring decision, so the burden is on showing the
decision's premise has been overtaken. It has — by both
consumers it named:

- **The film path chose decomposition.** `s10.scene.json` —
  the loaded shot file for the only BK shot — represents the
  same two objects as a group: a box child with singular
  `material {color}` plus a quad child carrying the mapped
  face at local z +0.026 (ASSUMPTIONS S10 "Inexpressible" item
  2, line ~2402: "Authored as a plain box + a separate quad…
  (film.rs does the same split)"). The production painter
  likewise hard-codes the arrangement (`src/scene.rs`
  `paint_picture`: the frame is "the box mesh's face
  materials… `#C9A13B` on five faces, this texture on the
  front" — resolved in code, not via the array).
- **The PBR/still path's material model is one colour per
  node.** `src/scenefile.rs` `material_colour()` returns a
  single `[f32; 3]` (colour, optionally shade/shadeHash);
  `colour_geo` builds single-colour geometry; the node's
  `texture` field — not a material slot — is the loader's map
  mechanism, and the register already records that "a
  colour-pipeline node cannot carry a map in v2".

**What CONSUME would mean.** A format feature, not a field:
box geometry gains six material groups (draw splitting in
`colour_geo`/`box_geo` and in the PBR mesh path); a second,
per-face material resolution (colour + roughness + `map`,
with `map` moving from node level into a material slot —
contradicting the v2 texture mechanism every other file
uses); scene-format §3.3 spec text; checker NODE/MATERIAL
set changes; fixtures and tests. Its entire beneficiary set
is two nodes in one file that **no loader ever loads whole**
(set files are fragments: `load()` rejects bookshop for the
missing `camera` — W42 §6). And it canonises a second
representation of objects the corpus already represents
canonically elsewhere, inviting the two to drift.

**What STRIP means (as staged).** The two nodes are rewritten
in the corpus's own canonical decomposed form — group with
the node's existing transform; box child with singular
`material {color, roughness: 0.8500000238418579}` (roughness
preserved: it is a loader-known material field,
`src/scenefile.rs:552` — one deliberate delta from s10, which
dropped it; the set file is the transcription of record and
keeps it); quad child named as in s10 (`picture canvas` /
`sign face`) with `texture` = the array's +z map id and the
same local z +0.026 translate s10 uses. Every value in the
array survives: both colours, the roughness, the map binding,
and the "+z face" fact (now structural). Node names: the
group takes s10's names (`picture` / `sign board`); the box
children keep `picture frame` / `sign board body`.

**Recommendation: STRIP, via the staged decomposition.**
**Losing option's cost (consume):** an engine + spec + checker
feature whose only data is two nodes in an unloadable
fragment, duplicating a solved representation — the classic
shadow-format trap W42 flagged, one wave later with a loader
attached. **Strip's own cost, stated plainly:** B2's declared
intent is overridden, so the landing must amend B2's text to
record that the set file now carries the decomposed (s10)
form and that the material-array form is **not** part of the
format — otherwise the next transcription re-adds it. The
data also stops being a literal transcription of the three.js
construction (one mesh + array) at these two nodes; the
construction remains fully described by B2's prose (group
order) + README row 22/23 + the staged structure.

---

## 3. F3-c (ii) — `lights.key.near` ×1 (bookshop)

The block (verbatim): key `{color #FFE4B8, intensity 2.4,
position [-2, 6, -13], target [0.5, 0, -1], shadow: true,
shadowBox: 8, far: 30, near: 0.5, bias: -0.0006, normalBias:
0, radius: 4, mapSize: 2048}`.

**Who reads key-light params today.** Two rigs, two homes:
(a) `src/scenefile.rs` (the §14.2 loader) parses key `color /
intensity / position / target / shadowBox / far / bias /
normalBias` (lines 955–962) — `near`, `radius`, `mapSize` are
not in its set (radius/mapSize are register-tolerated via B7;
`near` is the residue). (b) `src/film.rs`'s `LightState`
carries `key_near` (line 313) into the shadow ortho projection
(line 2895) — and its bookshop rig `rig_bk()` hardcodes
**`key_near: 0.5, key_far: 30.0, key_box: 8.0, key_bias:
-0.0006`** — the data's values, already in force in the
production renderer as code constants. ASSUMPTIONS S10
"Inexpressible" item 6 (line 2421) records the same fact from
the data side: "near 0.5 and radius 4… have no `lights.key`
fields — recorded in the lights note above only."

**What CONSUME would mean.** The smallest possible loader
completion: `KeyCfg` gains `key_near` and the parse gains one
line beside `far` — `near: f32_field_or(k, "near",
"scene.lights.key", 0.5)?` (default 0.5 = the three.js
DirectionalLightShadow default and the value both rigs'
code carries today: `rig_bk` 0.5; `rig_noon` uses 0.1, which
is exactly why the field belongs in data per-rig rather than
as a global constant); scene-format §14.2's key example gains
`"near"`; the checker's `LIGHT_KEY` set gains `"near"`. No
rendered pixel changes anywhere (the scene loader's rig is
header-print/house-light for stills per §14.2, and the film
renderer reads its rigs from code).

**What STRIP would mean.** Deleting `near: 0.5` from the one
file that records the interior sun's full shadow camera. The
block would keep `far` (loader-known), `shadowBox`, `bias`,
`normalBias`, and the B7-cited `radius`/`mapSize` — an
asymmetric record of a single three.js object, with the
missing field being one the renderer's own rig struct
carries. Worse, it plants the exact hazard F3 exists to kill:
if set files are ever loaded whole (the sets README's
referencing proposal is still the integrator's open call),
`far` would come from data (30) while `near` silently took
a parser default — a shadow camera half-transcribed.

**Recommendation: CONSUME** — the loader learns `near`.
**Losing option's cost (strip):** saves one parser line;
costs the record's symmetry and a future silent divergence,
and deletes a true, in-force value that two other places in
the repo (film.rs `rig_bk`, ASSUMPTIONS item 6) already treat
as part of the rig. Note the asymmetry with §2 is deliberate:
`materials` forks the format's *representation*; `near` is a
scalar that completes an existing one — the same distinction
as B7's radius/mapSize, except the renderer's own struct
proves `near` load-bearing.

**Staging consequence:** the staged bookshop file **keeps**
`near` (this seat never edits the live file, and the data is
correct as authored). Until the consume landing, `near`
remains a baselined finding — it is the third survivor in §4.
The consume change itself (loader + checker + spec) is the
lead's landing, specified above to one line of Rust.

Follow-up flagged for that landing (not staged here):
`s10.scene.json`'s key block omits `near` (ASSUMPTIONS item 6
records 0.5 as its value too); adding `"near": 0.5` to s10's
`lights.key` in the same PR makes the film file's rig
complete under the widened vocabulary — optional, zero-risk
(the loader default is the same value).

---

## 4. Ratchet arithmetic (first-hand runs)

Checker: `tools/format/check_unknown_fields.py`, the
STRICTNESS-W42 §6 corpus command (27 files), run in a scratch
tree (`/tmp` mirror of the corpus paths) in three states:

| state | unknown | paths | tolerated | verdict |
|---|---|---|---|---|
| 1. pristine corpus (control) | **196** | 15 | 7,043 | FAIL 3/5 |
| 2. + F3-a strip (`f3a-strip/onoda.json`) | **24** | 9 | 7,043 | FAIL 3/5 |
| 3. + F3-b/c staged files (this seat) | **3** | 3 | **7,047** | FAIL 3/5 |

States 1–2 reproduce the recorded baselines exactly (196/15
and W74's 24/9), which validates the scratch method. State 3's
tolerated rise (+4) is accounted: the §2 decomposition
replaces one named mesh with a named group + two named
children, twice — node `name` is a tolerated key (+4
occurrences). Checks 4/5 pass in all states; the remaining
FAIL is checks 2/3 carrying the three survivors.

**Every survivor, named** (state-3 findings, verbatim):

```
scenes/sets/bookshop.scene.json: $.lights.key — unknown key 'near' in light-key object (1 occurrence(s); first at $.lights.key.near)
film/onoda.json: $ — unknown key 'fps' in film-root object (1 occurrence(s); first at $.fps)
film/onoda.json: $ — unknown key 'frames' in film-root object (1 occurrence(s); first at $.frames)
```

- `fps`, `frames` — **F3-a-kept** (W74): HARD readers in the
  Python tooling (blink_cost / blink_census / evaluate_track);
  they leave the baseline only when the checker learns a
  tooling-known set for film-root keys. Not this seat's class.
- `near` — **F3-c, consume recommended** (§3.2): leaves the
  baseline in the consume landing, taking the register to
  **2 occurrences / 2 paths / 7,047 tolerated**.

**Staged baseline:** `docs/integration/f3bc/
unknown_fields_baseline.json` — the 24-baseline lowered to
these 3 paths (same shape and corpus list as
`tools/format/unknown_fields_baseline.json` / W74's staged
file), with the classes rewritten to say why each survivor
stays. Landing order, one PR series, ratchet-green at every
step: (i) data commit (staged files + this baseline — the
ratchet sees only shrinkage vs the 24-baseline: PASS);
(ii) the `near` consume (loader + checker + §14.2) with the
baseline lowered again to 2 in the same commit (a widened
known-set without the baseline edit would print a resolved
entry — the step's shrink-print is the mechanism); (iii) the
sets README B2 amendment (§2) rides either commit.

**Verification of the staged files** (recursive parsed diff
vs the live files) — s10: exactly `textures[3].motifLaw`,
`textures[4].note`, `textures[15].note` removed; s11:
`nodes[81].note` removed; s12: `nodes[78].note` removed;
bookshop: 14 texture `note`s removed, nodes[11]/[12]
restructured (§2), nothing else — `lights.key.near` verified
present and untouched.

---

## 5. What this seat did not do

- No live file edited; nothing committed. The lead lands the
  data commit, the `near` consume, and the B2 amendment.
- `fps`/`frames` (F3-a-kept) are out of scope by the brief;
  their exit route is recorded in the staged baseline's class
  text.
- The §15 reservation guard is untouched: none of the
  stripped keys is a §15 feature name, and the decomposition
  uses only v2 vocabulary (group/children/material/texture) —
  verified by the state-3 run producing no new findings of
  any class.
