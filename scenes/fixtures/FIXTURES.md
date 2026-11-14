# Format v2 fixtures pack

Minimal, valid v2 scene files — one feature each — that document
**the loader as it actually behaves today** (`src/scenefile.rs`),
not as the spec wishes it behaved. This is the loader-test data
set for the integrator and future CI: each fixture is small enough
to read at a glance, every staging value is an exact f32 (spec
§11), and every behaviour claim below cites the loader line that
produces it. Where the film data and the loader disagree, the
fixture follows the **loader**, and the disagreement is named.

Run any of them with:

```
talerender --scene-file scenes/fixtures/<name>.scene.json
```

All fixtures are `"format": "talerender-scene", "version": 2`.
The loader accepts versions 1 and 2 and rejects anything else
(`scenefile.rs:787–795`); a v1 file carrying `camera.keys` or a
`lights` block is rejected, not reinterpreted (`:833–835`,
`:872–874`).

**Load verdicts (statically simulated against the loader's own
rules; rendering itself is CI's job):** fx-character, fx-attachment,
fx-fog, fx-channels, fx-grade and fx-lights load today.
**fx-textures fails by design** — it is the registry census, and
its failure message is the measurement (see its section).

## Loader facts that apply to every fixture

| top-level field | loader behaviour today | cite |
|---|---|---|
| `format` | checked only if present; must be `"talerender-scene"` | `:784–788` |
| `version` | required in effect; 1 or 2 accepted, else hard error | `:787–795` |
| `name` | read, printed in the render header | `:805–809` |
| `time` | read for **any** version (no v2 gate); camera keys evaluate at it; a character's `state` is the state at this t by construction | `:799–802` |
| `camera` | `fovY`/`eye`/`target` required; `near`/`far`/`up` default 0.05 / 800 / +Y | `:813–829` |
| `camera.keys` | v2 only; smootherstep between keys, `"ease": "linear"` eases the incoming segment linearly; end keys hold outside the range | `:831–848` |
| `clear` | hex string → scene clear colour | `:850–854` |
| `lighting` | the **only** light the still paths use: `directional.direction` + `ambient` (defaults `[0.5, 0.8, 0.6]` / 0.35) become the shader light uniform on both render paths | `:858–868`; uniform `:1201`, `:1549` |
| `lights` | **parsed and validated, then unused by the still paths** — stored on `Loaded`, read by nothing in `scenefile.rs` outside its own unit test. Consumed in full by the film renderer (`src/film.rs`, spec §14.2) | parse `:870–931` |
| `atmosphere` | **never read** by `scenefile.rs` (zero hits). Ruling R5: the film driver resolves (set × preset) at render time | — |
| `fog` | **never read** by `scenefile.rs` (zero hits). The film path's fog is wired (W28); the loader wiring is specified in `shaders/fog/FOG.md` §8 and has not landed | — |
| `textures` | resolved **eagerly, in array order, at load** — one bad entry fails the whole file before any node is walked | `:932–943`, `resolve_texture` `:398–429` |
| `nodes` | required array, walked in document order | `:946–948` |
| `post.bokeh` | its presence selects the composer path; absence selects the direct path | `:951–962`, dispatch `:1180–1184` |
| `post.bloom` / `post.grade` | parsed, but **hard error without `post.bokeh`** | `:963–985` |
| unknown fields | ignored everywhere (spec §1) — this is what makes `channels`, `fog`, `atmosphere`, `captions`, `shot` and the data's documentary fields (`sourceDrawing`, `note`, `motifLaw`) inert rather than errors | §1 |
| unknown *values* | hard errors with the node path: node `type`, geometry `kind`, transform op, texture `generator`, attachment `kind`, code-ref `id` | e.g. `:426`, `:680` |

Two version-gating asymmetries, pinned because they are reality,
not design: the **character node, its `attachments`, and `time`
are not version-gated at all** (`walk_nodes` never receives the
version), so a v1 file with a character node loads — while a v1
file with camera keys or a `lights` block is refused.

---

## fx-character — one character, plain state

**Exercises:** the v2 `character` node (spec §14.3) with the full
`look` and `state` key sets spelled out at neutral values, so the
fixture doubles as the key census.

**Loader behaviour today — consumed.** `walk_nodes` character
branch (`:548–728`): `position` is required (`:556`); `height`
defaults 1.75 (`:557`); `perspective` is truthiness-tested
(`:559`). It paints the full drawToon port, pushes the card +
blob-shadow textures, and emits exactly two textured draws (blob,
card). Facing is computed from the evaluated camera — yaw toward
the eye, plus pitch ×0.45 clamped ±0.38 when `perspective` is set
(`:652–667`). Defaults when keys are omitted, all from the same
branch: look `body #E8734A`, `head #F7F7FA`, `pattern "plain"`,
`hat "none"`, `neck "none"` (`:565–582`); state `mood "neutral"`,
`armL`/`armR` `[8, 6]`, `look` `[0, 0]`, everything else 0
(`:604–640`). `legs` accepts `walk`/`run`/`sit`/`jump`; **any
other string silently means stand** (`:611–617`). `sitOn` is
metres, converted `× 1024 / (height / 0.96)` (`:625–628`).
`holdL`/`holdR` take `{plate, x0, y0}` referencing a declared
texture id — an undeclared id is a hard error (`:584–602`).

**When the pending wirings land:** nothing changes for this
fixture; it is already fully consumed.

## fx-attachment — character + on-node rifle (R2)

**Exercises:** ruling R2 — `attachments` lives **on** the
character node; there is no top-level attachments block anywhere
in the loader. The staging is `scenes/character-v2.scene.json`'s
Onoda (the §14 proof character) with the rifle on hand R.

**Loader behaviour today — consumed.** The attachments loop
(`:673–728`) runs after the card is painted, anchoring the rifle
at the painted hand position via the syncAttachments card-space
offset. `kind` must be exactly `"rifle"` — any other kind is a
hard error (`:678–681`); `"rifle"` is the only kind that exists.
`hand: "L"` selects the left hand; anything else (including
absent) selects the right (`:683`). `offset` defaults to
`[0, 0.03, 0.07]` (`:684–690`). The rifle's five parts (stock,
receiver, barrel, front sight, grip) and its two colours (wood
`#6E4F30`, steel `#3A3D42`) are **hardcoded in the loader**
(`:706–714`) — the file carries no rifle geometry, and cannot
restyle it.

**When the pending wirings land:** nothing pending; a second
attachment kind would be a loader change, not a data change.

## fx-fog — fog block + two panels

**Exercises:** the top-level `fog` block in the exact shape the
film data carries it (s06: `{color #C9CFC2, near 4.5, far 20.0}`),
plus the `atmosphere: "fog"` preset name s06 pairs it with, and
two colour panels at 6 m and 15 m — the depths whose smoothstep
factors FOG.md §7 pins at 0.0263 and 0.7550.

**Loader behaviour today — the `fog` block is ignored, wholly.**
`scenefile.rs` contains no read of `fog` at all; the file loads
and renders **unfogged**, and `atmosphere` is likewise unread
(the film driver resolves it, R5). The panels and `clear` (set to
the fog colour, as the fog-chart does) are consumed normally, so
today's render is the control image: both panels at full colour.

**When the pending wiring lands** (FOG.md §8: parse the block in
`load()`, thread it to both shader paths' fog uniforms — no
shader change needed, the include already implements three's
smoothstep factor): the same file, unedited, fogs — the 6 m panel
barely (factor 0.0263), the 15 m panel strongly (0.7550), exactly
the `scenes/fog-chart.scene.json` expectations in miniature.

## fx-channels — character + channels block

**Exercises:** a `channels` block on the character node, in the
shape the film data uses (s02's march: `position` driver +
`phase`/`bob`/`legs` formulas + `blink` director seed). The
character's static `state` and `position` are s02's Onoda at the
file's `time` (9.484) verbatim — the state at that instant,
evaluated by hand from the very formulas the channels record.

**Loader behaviour today — the `channels` block is ignored,
wholly.** `scenefile.rs` contains no read of `channels`; the
still loader renders the static `state` at `time` and never
evaluates a formula. The block's inner shape is not even parsed —
the entries are free-form records (formula strings + source
citations), carried as documentation.

**When the pending wirings land:** the consumer is not the still
loader but the film evaluator — `src/film.rs` already evaluates
these channels over each shot's duration (s02's evaluator is at
film.rs:1398, per `scenes/film/ASSUMPTIONS.md`). Nothing about
this fixture changes; it exists so the data shape has a minimal,
valid home that any future channel consumer can be tested
against.

## fx-grade — post grade in the loader's own keys

**Exercises:** `post.grade` spelled the way **the loader** reads
it — `vignette`, `grain`, `warm`, **`sat`**, `time` — carrying
s06's real values (`sat 0.52`, vignette 0.24, grain 0.012, warm 0,
time 0 per R3). `post.bokeh` is present with aperture 0 because
grade cannot exist without it (below); aperture 0 keeps the DoF
term inert so the grade is the only post effect acting.

**Loader behaviour today — consumed.** Grade parses at
`:970–978` with per-field defaults vignette 0.18, grain 0.008,
warm 0.02, **sat 1.12**, time 0 (`:972–976`), and runs on the
composer path after output. Two traps this fixture exists to
pin:

1. **The film data mostly spells it `saturation` (plus an
   `exposure` key)** — 11 of 14 segments do (AUDIT-W40). The
   loader reads neither: `saturation`/`exposure` are unknown
   fields and are ignored, so those files render stills at the
   default sat **1.12**, not their staged value. Only s03/s06
   spell `sat`. The fixture uses `sat`; the data-side fix is a
   separate, deliberate act.
2. **Grade (and bloom) without bokeh is a hard error**, not a
   silent drop (`:983–985`) — the composer is entered through
   bokeh. Also note the loader **honours the file's grade
   `time`** (`:976`): the film renderer pins it to 0 (R3), but a
   still render of s03/s07 — whose data carries `time` = film t —
   grains differently from Chrome. The fixture's `time: 0.0` is
   the R3-correct value.

**When the pending wirings land:** no wiring pending for grade
itself; the open item is the data/loader spelling decision
(rename the data's keys, or teach the loader the alias) — an
integrator call, recorded in AUDIT-W40.

## fx-textures — the registry census

**Exercises:** one quad mesh per texture the film data names —
all 13 generator names used anywhere under `scenes/` plus one
`file` entry — with each entry copied in the shape its real call
site uses (ab1, the shore set, s10). Entries are ordered
**resolved-first**, deliberately: the loader resolves eagerly in
array order, so the failure message names the first data-only
generator.

**Loader behaviour today — 7 of 14 resolve; the file fails.**
`resolve_texture` (`:398–429`) has exactly six generator arms —
`grass` (`base`), `sky-gradient` (`top`, `horizon`), `cloud`,
`blob-shadow`, `onoda-card`, `dof-chart-pattern` (`size`,
default 512) — plus the `file` branch (path relative to the
scene file's directory). Everything else returns
`unknown texture generator '<name>'` (`:426`). Census:

| entry | generator | today |
|---|---|---|
| `grass` | `grass` | ✅ resolves (`paint_grass`) |
| `sky` | `sky-gradient` | ✅ resolves (`paint_sky`) |
| `cloud` | `cloud` | ✅ resolves (`paint_cloud`) |
| `blob` | `blob-shadow` | ✅ resolves (`paint_blob`) |
| `onoda-card` | `onoda-card` | ✅ resolves (§28 painter) |
| `chart` | `dof-chart-pattern` | ✅ resolves |
| `plate-book-open` | *(file)* | ✅ resolves (`image` crate) |
| `planks-crate` | `planks` | ❌ **hard error — the file fails here today** |
| `felt-counter` | `felt` | ❌ no arm |
| `wallpaper-back` | `wallpaper` | ❌ no arm |
| `parcel` | `parcel` | ❌ no arm |
| `picture` | `picture` | ❌ no arm |
| `sign` | `sign` | ❌ no arm |
| `view` | `solid` | ❌ no arm |

Data census behind the table (grep over `scenes/`): planks ×54,
grass ×29, sky-gradient ×17, cloud ×16, solid ×11, wallpaper ×7,
felt ×4, sign ×2, picture ×2, parcel ×2, the rest ×1 each.

Also pinned by this fixture: **entry-level fields the loader
ignores even for resolved generators** — `repeat`, `canvas`,
`rotation`, `note`, `motifLaw`, `sourceDrawing` are carried in
the data (consumed by `film.rs` or purely documentary) but
`resolve_texture` reads only `file`, or `generator` + that arm's
own params. Repeat addressing in the still path comes from the
node's texture reference (`"texture": {"id": …, "wrap":
"repeat"}`, `:479–495`), never from the entry.

**When the pending wirings land:** the seven missing painters
already exist as tested modules in `painters/` (41/41 tests,
W39) and `painters/INDEX.md` specifies each `resolve_texture`
arm verbatim (plus the one-line `solid` arm the fog-chart also
needs). Each landed arm moves one row of the census to ✅; when
all seven land, this fixture loads end-to-end and renders the
full row of panels — that green load is the wiring's acceptance
test.

## fx-lights — the production rig (R1)

**Exercises:** the top-level `lights` block exactly as the film
files carry it — the noon rig from `scenes/character-v2.scene.json`
verbatim, values **POST-atmosphere per ruling R1** (key
`#FFF0D6` ×2.6, rim ×0.4, hemisphere `#CFE6FF`/`#C48A5A` ×1.25,
fire point basis **4.6** — the rendered `fireLevel` basis, never
the constructor's 5.0), with the `atmosphere: "noon"` name the
data pairs it with.

**Loader behaviour today — parsed, validated, and not consumed
by the still paths.** The parse (`:870–931`) is real: a v1 file
carrying the block is rejected (`:872–874`); key reads `color`,
`intensity`, `position`, `target`, `shadow`, `shadowBox`, `far`,
`bias`, `normalBias` (`:876–887`); rim `color`/`intensity`/
`position`; hemisphere `sky`/`ground`/`intensity`; points[]
`color`/`intensity`/`position` (required)/`distance`/`decay`;
`envIntensity` (default 0.25). Loader defaults for omitted
fields are the loader's own, **not** the source engine's (key
intensity 3.2, colour `#FFE2C2`, shadowBox 3.0, far 20, bias
−0.0004; rim 2.2 `#BFD8FF`; hemisphere 0.9 `#FFF4E6`/`#8C6E55`;
point distance 0, decay 2.0) — the film files spell every field
out, and so does this fixture. After parsing, the rig sits on
`Loaded` unused: both still paths light from the v1 `lighting`
block (house defaults here, since the fixture omits it), and the
only other reader in the file is the loader's own unit test.
**The film renderer (`src/film.rs`) consumes the rig in full** —
shadows, points, env — which is what the block is for.

**When the pending wirings land:** nothing is pending in the
loader; if a future still path ever binds the rig, this fixture
is its smallest test case.

---

## Census summary

**Fixtures:** 7 written — 6 load today, 1 (fx-textures) fails by
design at its first unwired generator.

**Features exercised, by loader status today:**

| status | features |
|---|---|
| **consumed** | character node (+ look/state/facing), on-node rifle attachment, camera + keys, `clear`, `lighting` (house), `post.bokeh`, `post.bloom`, `post.grade` (loader keys), mesh/group/scatter geometry, material colour specs, `roughness` (type-checked only — carried, `:497–501`) |
| **parsed, not consumed** (still paths) | `lights` rig — validated into `Loaded`, consumed only by `film.rs` |
| **carried, ignored** | `fog` block, `channels` block, `atmosphere` name, texture-entry `repeat`/`canvas`/`rotation`/documentary fields, grade `saturation`/`exposure` spellings |
| **hard error** | unknown texture generator (7 of the data's 13), unknown attachment kind, unknown node/geometry/transform values, grade/bloom without bokeh, v1 file + keys/lights, version ∉ {1, 2} |

**Texture registry:** 6 generator arms + `file` resolve today;
7 data-named generators (planks, felt, wallpaper, parcel,
picture, sign, solid) have tested painter modules staged in
`painters/` and specified arms in `painters/INDEX.md`, awaiting
the lead's wiring.

*Verification note: fixtures were validated as JSON and checked
field-by-field against the loader source above (a static
simulation of `load()`'s reads, including the fx-textures failure
point); no fixture was rendered from this VM — the PBR/composer
paths verify through CI, per the wave's standing rule.*
