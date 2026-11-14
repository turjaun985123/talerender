# CHARANIM-W12 — Character animation channels (Phase 4, seat W12)

**Delivered:** `src/charanim.rs` (new file only; nothing else in
`src/` touched, nothing committed). Self-contained module —
compiles and tests standalone:

```
rustc --edition 2021 --test src/charanim.rs -o /tmp/charanim_test && /tmp/charanim_test
→ 12 passed; 0 failed   (also: --crate-type lib build, 0 warnings)
```

The module builds **channels that feed the existing rig**, not a
parallel one. Every output is a value/delta for a field of
`toonfull::FullState` under the same name, units and sign
conventions. It is not wired into `film.rs`/`scenefile.rs` yet —
that is the lead's integration step (sketch below).

## Channels

| Channel | Entry point | Shape |
|---|---|---|
| Blink | `blink_amount(id, t) -> f32` (+ `blink_amount_q` quantized) | Smooth 0.12 s down / 0.12 s up smootherstep curve; per-blink schedule hashed from `(id, blink_index)`: first blink at 0.6–2.0 s, gaps 2.6–4.4 s |
| Idle | `idle_at(id, t) -> IdleDelta` (+ combined `channels_at`) | Breathing bob/squash in phase, sway, head micro-drift; per-id hashed periods/phases |
| Gaze | `gaze_at(head, facing_yaw, target) -> GazeTarget`, `gaze_delta(...)` | Absolute head/pupil targets + a delta form, clamped to authored rig ranges |
| Expression | `expression_at(from, to, t0, dur, t) -> ExpressionBlend` | Smootherstep crossfade over the painter's continuous brow parameters + card-mix factor |

## Mapping: channel outputs → real character-state fields

Target struct: `toonfull::FullState`, `src/toonfull.rs:140-161`.
JSON names are the scene-format v2 state keys
(`src/scenefile.rs:646-673`).

| charanim output | FullState field (line) | JSON key | Units / convention (painter evidence) |
|---|---|---|---|
| `blink_amount` → absolute | `blink: f32` (toonfull.rs:143) | `blink` | [0,1]; painter scales eye height by `1.0 - blink*0.6` and draws the closed-eye line when `blink > 0.6` (toonfull.rs:593, 609) |
| `IdleDelta.bob` → delta | `bob: f32` (toonfull.rs:152) | `bob` | Stage units (stage = 512×1024, toonfull.rs:10-11); added to body/head Y (toonfull.rs:320-328). Amp ±1.5 |
| `IdleDelta.lean` → delta | `lean: f32` (toonfull.rs:151) | `lean` | Degrees, `to_radians` at toonfull.rs:306. Amp ±0.35° |
| `IdleDelta.squash` → delta | `squash: f32` (toonfull.rs:153) | `squash` | Unitless; head height ×(1+squash) (toonfull.rs:334). Amp ±0.004 |
| `IdleDelta.head_tilt` / `GazeTarget.head_tilt` | `head_tilt: f32` (toonfull.rs:154) | `headTilt` | Degrees, in-plane roll about the neck pivot (toonfull.rs:330-341). Idle amp ±0.5°; gaze returns exactly 0 (see limits) |
| `IdleDelta.head_turn` / `GazeTarget.head_turn` | `head_turn: f32` (toonfull.rs:155) | `headTurn` | Unitless; face shift `turn*30` stage px, eye spacing ×(1−0.18·\|turn\|) (toonfull.rs:507-545). Idle amp ±0.012; gaze clamp ±0.5 |
| `GazeTarget.look` → absolute | `look: [f32; 2]` (toonfull.rs:142) | `look` | Pupil offset `look*[9, 8]` stage px (toonfull.rs:603); +y is DOWN-stage, so a target above yields negative `look[1]`. Clamp ±1 per component |
| `ExpressionBlend` | `mood: Mood` (toonfull.rs:141) | `mood` | Discrete — see the expression limit below. `Expr` mirrors `Mood` 1:1 (17 variants, same strings, toonfull.rs:38-79) |

Character id: `u64`. The film's blink seeds in the look registry
(`film.rs:434-435` — seeds 3, 5, 7, 9, 11 as `f64`) map by
`seed as u64`; scene-file characters can take any stable per-node
id (e.g. a hash of the node name, or an authored `"animId"`).

### Rig limits — honest provenance

The painter contains **no clamp constants** for these fields; it
is linear in all of them. The channel limits are therefore the
ranges the production film actually authors, cited so the lead
can re-derive them:

- `head_turn` ±0.5 — pose-track keys reach −0.45…+0.5
  (`film.rs:1405`, `film.rs:1512`).
- Staged static `head_tilt` −5°…+6° (`film.rs:1312-1327`);
  channel-driven totals are clamped to ±10° in `apply_channels`.
  Bows exceed this (`film.rs:1937`, `1966`) and remain staging's
  business — `apply_channels` is for channel output only.
- `look` components: staged values reach only ±0.4
  (`film.rs:1299-1327`, `1712`, `1969`); ±1 is the natural pupil
  saturation (pupil offset would exceed the 13-unit eye radius
  beyond it, toonfull.rs:603-609).
- Gaze angular scales (design constants, stated in the module):
  60° of off-facing angle per 1.0 `head_turn` (so the ±0.5 clamp
  = ±30° of head yaw), pupils saturating at ±40° horizontal /
  ±25° vertical — eyes lead, head follows, matching the staged
  poses (e.g. `look [0.4, -0.3]` with `head_turn 0.3`,
  `film.rs:1322-1327`).

## Determinism, per channel

All functions are pure in their arguments — no RNG state, no
wall clock, no globals. The only variation source is splitmix64
over integer inputs (platform-exact).

- **Blink, Expression** — integer hash + f64 add/mul + the
  smootherstep polynomial (house easing, `film.rs:130-133`).
  No transcendentals → **bit-exact on any platform**.
- **Idle, Gaze** — use `sin`/`atan2` → bit-repeatable for a given
  platform/binary; last-ulp differences across libm versions are
  possible. This is the same caveat the existing film evaluators
  already carry (`settle`/`talk`, `film.rs:214-235`).
- Blink schedule lookup walks from t=0 accumulating hashed gaps
  (fixed addition order → same result every call); <200
  iterations for any film-length t.

Tests pin: blink ∈ [0,1] over a dense 120 s sweep × 8 ids; blink
repeatable per id (bitwise) and differing across ids; blink
cadence 25–48 events/120 s; idle bounded by its stated amps;
gaze clamps behind/above/far targets; gaze dead-ahead = 0;
expression endpoints **bit-exact** at/before t0 and at/after
t0+dur (endpoints are returned directly, never interpolated);
midpoint = arithmetic mean of brows (smootherstep(0.5) = 0.5);
the 17-name vocabulary round-trips and spot-pins the brow table
against `toonfull.rs:80-98`.

## What integration looks like (for the lead)

**film.rs path.** In the per-shot staging functions, after a
pose builder fills a `FullState`, replace/augment today's
`st.blink = Blinker { seed }.at(t)` calls with:

```rust
let d = charanim::channels_at(char_id, t);
st.blink = d.blink;               // or keep Blinker for A/B parity — see cost note
st.bob += d.bob; st.lean += d.lean;
st.head_tilt += d.head_tilt; st.head_turn += d.head_turn;
st.squash += d.squash;
// gaze, where a shot has a look-target:
let g = charanim::gaze_at(head_world, yaw, target_world);
st.head_turn = g.head_turn; st.look = g.look;
```

`mod charanim;` in main.rs + `Expr`↔`Mood` conversion by name
(`as_str`/`from_str` are string-identical on both sides) is the
only glue. `CharStateMirror` in the module is the standalone
stand-in for `FullState` and maps field-for-field; it can be
deleted once the real struct is imported.

**scenefile.rs path.** In the character-node state builder
(`scenefile.rs:646-673`), after constructing `state`, apply
`channels_at(anim_id, t)` when the node opts in (suggested JSON:
`"anim": {"id": 3, "idle": true, "blink": true}`), and a
`"gaze": [x, y, z]` world target → `apply_gaze`. Default off =
byte-identical behaviour for all existing scene files.

### Cost note the integrator must decide

`state_key` keys the paint cache on the exact blink value
(`film.rs:2132-2151`). The binary `Blinker` produces ONE distinct
blink value per event (1.0 for 0.12 s); the smooth curve
produces ~7 distinct values per 0.24 s event → up to ~7 paints
where today there is 1. `blink_amount_q(id, t, 8)` bounds this
to ≤8 levels total at a small fidelity cost. Idle has the same
property in principle (it changes every frame — but idle applied
to a *posed* state already differs per frame wherever staging
animates pose tracks, so its marginal cost is nil on animated
shots and real only on locked-off stills; there, staging can
freeze idle by evaluating it at the shot's first frame).

## Honest scope limits

- **Expressions do not morph inside one card.** `FullState.mood`
  is a single enum and the painter switches mouth/brow/eye
  branches discretely on it (toonfull.rs:542-675). A truthful
  crossfade therefore ships as three honest forms: `mix` for a
  composite-level blend of the two painted cards, interpolated
  `brow_lift`/`brow_tilt`/`heavy_lid` for a future brow-override
  state field, and `mood` = dominant side (switch at mix 0.5)
  for integrators who can only switch. The mouth shapes have no
  continuous parameters in the painter and are NOT interpolated.
- **Gaze head_tilt is 0 by design.** In this painter head_tilt
  is an in-plane roll (head-cock), not pitch; vertical gaze is
  carried by `look[1]`, and `cam_pitch` is camera-derived
  (`film.rs:1364`) and must not be authored by a character
  channel. A deliberate head-cock remains a staging choice.
- **No locomotion.** Walk/run cycles already exist
  (`Legs::Walk/Run` + `phase`, film.rs `walk_phase`/`walk_bob`);
  this seat adds none.
- **No lip-sync / phonemes — deliberately out of scope.** They
  need word-timing data plumbed into the character evaluator,
  which no current code path carries. The data that exists:
  the project's `words.json`
  (`~/faceless-shorts/the-soldier-who-didnt-know/data/words.json`)
  is a flat array of `{"w": <word>, "line": <int>, "s": <start s>,
  "e": <end s>}` (faster-Whisper timings), with `lines.json`
  beside it as `{"duration", "voice", "speed", "lines":
  [{"i", "text", "start", "end"}]}`; the film build already
  derives word anchors from this pipeline (`film.rs:7`,
  `film.rs:46-71`, consumed by `blink_at` beats). What lip-sync
  would additionally need: (1) a phoneme decomposition of each
  word — `words.json` is orthographic only, so a pronouncing
  dictionary (CMUdict-style) or grapheme→phoneme rules plus a
  viseme grouping; (2) a viseme→mouth mapping onto
  `FullState.talk` (today a procedural oscillation,
  `film.rs:228-235`, consumed by the mouth painter in
  toonfull.rs) — i.e. talk would become a *driven* channel, not
  a free oscillator, for speaking characters; (3) per-line
  speaker attribution — neither `words.json` nor `lines.json`
  carries a speaker field, so who speaks line N must come from
  staging/cue data at build time.
