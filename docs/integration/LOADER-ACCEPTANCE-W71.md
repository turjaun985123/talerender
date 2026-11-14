# LOADER-ACCEPTANCE — W71 verification of the c74cf0c loader lane

Seat W71 (100-agent wave, Batch 8). Read-only on `src/`; verified at HEAD
`3781eb2` (the loader lane is commit `c74cf0c`, felt-test fix `d8fbe83`).
All line numbers are at HEAD.

## 1. Texture-generator census — 13/13 armed, none missing

Every `"generator"` value named anywhere under `scenes/**/*.json`
(film segments, sets, standalones, and the fixtures file, which names
all 13 once each plus one `file` entry):

| generator | uses in scenes/ | resolve_texture arm |
|---|---|---|
| grass | 30 | `src/scenefile.rs:410` → `scene::paint_grass` |
| planks | 55 | `src/scenefile.rs:413` → `scene::paint_planks` |
| wallpaper | 8 | `src/scenefile.rs:414` → `scene::paint_wallpaper` |
| felt | 5 | `src/scenefile.rs:418` → `scene::paint_felt` |
| parcel | 3 | `src/scenefile.rs:419` → `scene::paint_parcel` |
| picture | 3 | `src/scenefile.rs:420` → `scene::paint_picture` |
| sign | 3 | `src/scenefile.rs:421` → `scene::paint_sign_px` |
| solid | 12 | `src/scenefile.rs:434` → inline 64×64 flat |
| sky-gradient | 18 | `src/scenefile.rs:446` → `scene::paint_sky` |
| cloud | 17 | `src/scenefile.rs:450` → `scene::paint_cloud` |
| blob-shadow | 2 | `src/scenefile.rs:451` → `scene::paint_blob` |
| onoda-card | 2 | `src/scenefile.rs:452` → `crate::ab1::paint_onoda` |
| dof-chart-pattern | 2 | `src/scenefile.rs:453` → `chart_texture_rgba` |

`resolve_texture` is `src/scenefile.rs:399`; the hard error survives
only as the fallthrough at `:461`
(`unknown texture generator '{other}'`). **No generator named by the
data is unarmed.** The 7 generators W47 found unarmed (planks,
wallpaper, felt, parcel, picture, sign, solid) are exactly the arms
c74cf0c added (`:413–445`). Use counts reconcile with W47's census
plus the one fixture entry each (planks 54+1, solid 11+1, wallpaper
7+1, felt 4+1, …).

`file` textures resolve via `image::open` (`:400–408`). The fixture's
one file entry, `../../film/assets/book-open.png`, resolves from
`scenes/fixtures/` to the repo-root `film/assets/book-open.png`, which
exists (the same relative path s10 uses from `scenes/film/`).

Per-arm required fields vs the fixture's entries: every arm's
`str_field` requirements are met by `fx-textures.scene.json`
(planks/felt carry `base`; wallpaper `base`+`motif`; sign
`bg`+`fg`+`canvas`; solid `color`; sky-gradient `top`+`horizon`).
Resolution is eager at load (`:976`), so a successful load of the
fixture exercises all 13 arms plus the file path.

## 2. Painter paste fidelity — pins present, and CI-executed

Painter entry points in `src/scene.rs`: `paint_planks` `:1913`,
`paint_wallpaper` `:2183`, `paint_felt` `:2399`, `paint_parcel`
`:2522`, `paint_picture` `:2666`, `paint_sign_px` `:2844`.

Test modules pasted per painter with counts identical to
`painters/*.rs`: planks 7, wallpaper 6, felt 5, parcel 5, picture 4,
sign 7 — **34 tests**. Full test-body comparison against
`painters/*.rs`: **33/34 byte-identical**. The one difference is
felt's `shade_is_the_three_linear_multiply`, where the pasted copy is
the *newer* one: `d8fbe83` edited only `src/scene.rs` to assert
`shade_three == shade_rgb` (the post-`c00897d` truth); the staging
file `painters/felt.rs` still carries the retired `assert_ne!`. The
staging copy lags by that one test; the shipped code is current.

Corpus pins verified present in `src/scene.rs` (all inside the
byte-identical test bodies):

- `#302222` = shade(#3A2A2A, 0.7): `planks_shade` pin at `:1985`
  `(0x30, 0x22, 0x22)`; sign edge pin at `:2915` `(48, 34, 34, 255)`.
- `#413427` = shade(#3A2E22, 1.25): `planks_shade` pin at `:1986`
  `(0x41, 0x34, 0x27)`; wallpaper `shade_hex` pin at `:2310` and
  `CEIL_MOTIF (0x41, 0x34, 0x27, 255)` at `:2251`, exercised end-to-end
  via `paint_wallpaper("#3A2E22", "#413427")` at `:2311`.

Spot pins per painter (3+ each, all in byte-identical bodies):
planks — canvas 512×512 (`:1961`), determinism (`:1972`), the two
shade pins above; wallpaper — 256×256 (`:2256`), `at(10,10) == WALL`
(`:2274`), ceiling pins above; felt — 256×256 (`:2430`), two-base
determinism (`:2436–2437`), `shade_three("#1F5A45", 0.85) == (28, 83,
64)` (`:2459`); parcel — 256×256 (`:2564`), label/no-label determinism
(`:2570–2571`); picture — 256×200 (`:2695`), alpha-255 sweep
(`:2700`), determinism (`:2708`); sign — `sign_canvas_size(2.2, 0.4)
== (440, 80)` (`:2907`), `(2.0, 0.5) == (400, 100)` (`:2909`),
`sign_text_anchor(440, 80) == (220.0, 43.0)` (`:2920`).

Count note: the c74cf0c message says "their 41 pinned tests".
`painters/TEST-RUN.md`'s 41 = these 34 **plus calendar's 7**;
calendar is not a texture generator (no `resolve_texture` arm — it
composites through the film path), so 34 is the correct scene.rs
count and the message overstates the paste by calendar's 7.

**CI executed the pins.** Push CI for c74cf0c (run `43574018568`)
ran the suite: **70 passed / 1 failed** — the single failure was
`scene::felt_tests::shade_is_the_three_linear_multiply`, panicking at
`src/scene.rs:2460` on the stale `assert_ne!` the paste had carried
in. Push CI for the fix `d8fbe83` (run `43574473202`): **71 passed /
0 failed**. So at HEAD the pasted painter tests are present, pinned,
and green in CI — not merely staged.

## 3. The grade-key decision — exposure wired; `saturation` DEFAULTED, silently

The loader's grade parse is `src/scenefile.rs:1005–1015`
(`GradeCfg` struct at `:66–74`). Canonical keys read: `vignette`,
`grain`, `warm`, `sat`, `exposure`, `time` — each via
`f32_field_or` (`:163–168`): exact-key lookup, absent → default
(`sat` default **1.12**, exposure 1.0). There is **no alias** for
`saturation` and **no unknown-key rejection** anywhere in the loader
(grep: no key-set validation in scenefile.rs).

**Answer for files that write `saturation`: not broken, not aliased —
defaulted.** The file loads normally; the `saturation` key is never
read; the render grades at sat = 1.12 regardless of the authored
value. Census of `post.grade` blocks at HEAD:

- **12 of 14 film files write `saturation` and no `sat`** — s01 0.8,
  s02 0.84, s04 0.8, s05a 0.8, s05b 0.8, s07 0.85, s08 0.8, s08b 0.8,
  s09 0.8, s10 0.78, s11 1.08, s12 1.08. Every one differs from the
  1.12 default (largest error: s10, 0.78 → 1.12). On the loader lane
  these segments render with the wrong saturation, silently.
- Only s03 (`sat` 0.84) and s06 (`sat` 0.52) use the canonical key and
  are honoured; the fx-grade fixture is also canonical (0.52).

The exposure half of the decision **was** implemented: `exposure` is
parsed (`:1011`) and wired to the composer OutputPass uniform
(`:1843`, `shader_output_exp`); the 7 film files writing it (s01,
s02, s07, s08b, s10, s11, s12) are honoured, absent → 1.0 = the old
hardcoded output bit-for-bit.

Two consistency notes for the lead (report-only; both files are
lead-owned or data-owned):

- `docs/scene-format.md:302,310` still documents the grade key as
  **`saturation`** (default 1.12) — the spec text and the loader code
  now disagree on the key name, in opposite directions from the data.
- `scenes/film/ASSUMPTIONS.md` "Inexpressible 4" (and the s07
  field-name note) predate c74cf0c: their "no exposure field" half is
  resolved; the saturation half stands, now as a silent-default
  rather than a declared gap.

Scope of impact: **loader lane only.** The film driver (`film.rs`)
does not read scene-file grade blocks, so the full-film re-render is
unaffected. The fix is mechanical on either side — rename the key in
the 12 data files (matching s03/s06 and the fixture), teach the
loader the `saturation` alias, or rename in code + spec — but it is a
lead decision, and until it lands, scene-file renders of 12 segments
grade at the wrong saturation with no error anywhere.

## 4. CI evidence — no scene-file dispatch at ≥ c74cf0c

Read-only `gh` checks (account `turjaun985123`):

- Render-dispatch runs at headSha `c74cf0c`, `d8fbe83`, or `3781eb2`:
  **none**. The most recent `render-scene-file-0-0-*` artifacts are
  from 2026-11-14 06:52–07:07 UTC — pre-loader-lane heads (the
  character-v2 scene-file mode's earlier runs). The 08:00–08:01 UTC
  dispatches at `381ff47` are all film mode (`render-film-*`
  artifacts, the full-film re-render).
- Push CI did run at the loader-lane commits — `43574018568`
  (c74cf0c, failure: the one felt test, §2) and `43574473202`
  (d8fbe83, success, 71/71 tests) — but CI's build-and-render job does
  not render `fx-textures`.

**The exact dispatch that produces the acceptance render**
(workflow `.github/workflows/render.yml`, dispatch-only):

- workflow: **Render (dispatch)**
- inputs: `mode = scene-file`, `scene = scenes/fixtures/fx-textures.scene.json`
  (`from`/`to` unused in this mode; leave defaults `0`/`0`)
- equivalent CLI once dispatched by hand:
  `gh workflow run render.yml -f mode=scene-file -f scene=scenes/fixtures/fx-textures.scene.json`
- expected artifact: `render-scene-file-0-0-<run_id>` containing
  `render-out/scene.png` (1080×1920, CI lavapipe). A produced PNG
  proves the fixture loads (all 13 arms + the file texture, eagerly)
  and renders end-to-end through the composer-less direct path
  (the fixture has no `post` block).

## 5. Verdict

**The fx-textures acceptance test is passable in principle — the
load path is fully armed.** Every generator the fixture (and the
whole corpus) names resolves (§1), every arm's required fields are
present in the fixture's entries (§1), the file texture resolves,
and the painters behind the new arms carry their pinned values with
CI-green tests at HEAD (§2). Nothing in the loader can still fail
this file with `unknown texture generator`.

What remains unproven:

1. **The render itself.** No scene-file dispatch at ≥ c74cf0c exists;
   the acceptance PNG has never been produced. Local rendering is not
   a substitute (this VM's llvmpipe JIT segfaults on the PBR path —
   render.yml's own header). One dispatch (§4) closes this.
2. **Pixel correctness of the newly armed painters in situ.** The
   pins prove painter outputs in isolation; no A/B of a scene-file
   render against the Chrome original exists for a planks/wallpaper/
   felt/parcel/picture/sign/solid texture yet.
3. **The `saturation` silent default (§3)** — orthogonal to
   fx-textures (the fixture carries no grade block) but the loader
   lane's one live behavioural defect: 12 film segments would grade
   at sat 1.12 instead of their authored 0.78–1.08, with no error.
   Awaiting the lead's rename-vs-alias decision.
