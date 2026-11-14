# FORMAT-EXT-CLIPS-W15 — scene-format extension: clips & tracks

**Seat:** TaleRender Wave 2, W15 (scene-format extensions II — Phase 4).
**Date:** 2026-11-14. **Status:** data model + validator DONE and proven
on fixtures (§6). **Nothing in `src/` consumes this yet** — loader /
evaluator wiring is the lead's adoption step (§7). This document is the
extension spec the lead folds into `docs/scene-format.md`; it does not
edit that file.

**Scope, honestly:** this seat defines the *data side* of animation
clips — how named keyframe clips are expressed in scene-format JSON so
the evaluator W11 is building has something to load — plus a standalone
validator (`tools/format/check_clips.py`) and fixtures
(`scenes/fixtures/clips/`). No Rust was written, no existing scene file
was touched, and the clips below are **not** claimed to render.

**W11 alignment note:** W11's evaluator *note* was not in the tree
when this seat started (no `*W11*` file; `tools/format/` did not
exist either — W14's prefabs seat had not landed). Its **code**
landed mid-seat, though: `src/anim.rs` now exists, and this spec
was aligned to its data-facing names before closing —
`Easing::from_name`'s accepted spellings for `ease` (§3),
`Wrap::{Clamp, Loop, PingPong}` for the `clipTime` modes (§2.5,
§4), and `Key { t, value, ease }` for the key fields (§2.3). The
points where anim.rs's *structure* and this data model still
differ in placement are collected as reconciliation flags in §7
rather than papered over.

---

## 1. Why clips, and why this shape

The engine already animates by keyframes in three places, with three
spellings:

| Where | Shape today | Semantics |
|---|---|---|
| Scene format §14.4 (loader) | `camera.keys[] = {t, eye, target, fovY?, ease?}` | interpolate across the whole bracketing segment; smootherstep default, `"linear"` per key |
| film.rs staging (code) | `pose_track(&[key(t,v), keym(t,v,moveSecs)], lt)` | **hold** each value, then move into the next key over that key's `moveSecs`, smootherstep |
| Film scene files' `channels` audit blocks (data, loader-ignored) | `poseTrack` key triples `[[t, v], [t, v, moveSecs]]`; object-form node keys `[{t, value, ease}]`; formula records | the wave's transcription record of the source's time behaviour |

Clips generalise the first shape — the one that is already *data* —
to every channel the engine animates, give the result a **name**, and
let any number of nodes/blocks **instance** it with a time mapping.
The `pose_track` shape is *not* adopted as the key grammar (its
hold-then-move semantics differ); §5.2 gives the exact conversion for
the film's existing poseTrack data.

## 2. The data model

All additions are additive under the scene-format §10 versioning
ethos: a top-level `clips` block, plus `clip` / `clipTime` fields on
things that already exist. Numbers everywhere obey the **§11 f32 data
law** (enforced by the validator, check 5).

### 2.1 The `clips` block

```jsonc
"clips": {
  "<clip-name>": {                       // name: any JSON object key
    "duration": 5.907,                   // seconds, > 0, REQUIRED
    "tracks": [ Track, ... ],            // REQUIRED, non-empty
    "uses": [ "<clip-name>", ... ]       // OPTIONAL layering (§2.4)
  }
}
```

### 2.2 A track

```jsonc
{ "target": { "node": "camera", "channel": "eye" },
  "keys":   [ Key, ... ] }               // REQUIRED, at least one key
```

`target.node` is one of:

- the **reserved targets** `camera`, `lights`, `post`, `fog` — the
  scene-level blocks, or
- the **`name` of a node in the same file** (searched recursively
  through `children`). Node names colliding with a reserved target
  are rejected by the validator (the node would be unaddressable);
  no production file does this (names are like `Onoda (S01)`,
  `leaflet 3`).

### 2.3 A key

```jsonc
{ "t": 2.6, "value": 0.4, "ease": "easeOutCubic" }
```

- `t` — clip-local seconds; `0 ≤ t ≤ duration`; strictly increasing
  within the track (validator, check 3).
- `value` — shaped by the channel kind (§3): a number (scalar),
  `[x, y, z]` (vec3), `"#RRGGBB"` (colour), or an enum string (step
  channels: `mood`, `legs`). The field name is `value` — the name
  in W11's `Key` struct (anim.rs:305) and in the film's existing
  object-form keys (s05a/s11) alike.
- `ease` — OPTIONAL, default `smoother`. Names the curve of the
  segment **ending at this key** (the incoming segment) — the same
  placement as §14.4, where `eval_camera_keys` reads the ease from
  the segment's end key (`src/scenefile.rs:788`), and the same
  direction as poseTrack, whose move is governed by the key being
  moved *into*. The first key's `ease` has no incoming segment and
  is ignored, exactly as the §14.4 loader ignores it.
- A single key is a constant track. Before the first key and after
  the last, the end values **hold** — the §14.4 camera law
  ("outside the key range the end keys hold") and poseTrack's
  hold behaviour agree on this.

### 2.4 Layering: `uses`

A clip may name other clips in `uses`; an instance of the clip then
applies its own tracks **plus** the used clips' tracks, transitively.
This is the one inter-clip reference in the model, and it is why
reference cycles are a real failure class: the `uses` graph must be
acyclic (self-use counts), every name must resolve, and the layered
set may not define the same target+channel twice (no silent
precedence). All three are validator check 4. If W11's evaluator
does not want layering, deleting this field orphans nothing else in
the model — flagged in §7.

### 2.5 Instantiation

Any node (recursively), and the `camera`, `lights`, `post`, `fog`
blocks, may carry:

```jsonc
"clip": "look-around",
"clipTime": { "offset": 0.5, "rate": 1.0, "loop": "ping-pong" }
```

- `clip` — the clip name; must be defined in the same file
  (validator check 4). The same clip may be instanced by many
  nodes — that reuse is the point of naming clips.
- `clipTime` — OPTIONAL; every field defaults:
  - `offset` (default `0.0`) — seconds added to the mapped time:
    the clip is entered `offset` seconds in.
  - `rate` (default `1.0`, must be `> 0`) — time scale.
  - `loop` (default `"clamp"`) — what happens outside
    `[0, duration]`; §4 gives the formulas. The three values are
    the kebab spellings of W11's `Wrap::{Clamp, Loop, PingPong}`
    (anim.rs:163) — the JSON field keeps the brief's name `loop`;
    the evaluator's enum is `Wrap` (flagged in §7).
- **Conflict rule:** `camera` may not carry both `clip` and `keys`
  (one camera, one time source) — validator check 4. Which form a
  shot uses is the author's choice; carrying both is data rot.

What an instance *means* against static fields: the clip's tracks
**replace** the target's static value at evaluation time (the
static `state` / `eye` / rig remains the at-rest staging and the
t-before/after reference, exactly as §14.4's static `eye`/`target`
remain the required base/fallback next to `keys`).

## 3. The channel vocabulary — surveyed, not invented

Every channel below is a field the engine animates *today*, cited to
the code that consumes it. A channel not in this table is an unknown
channel (validator check 2). Kinds: **S** scalar, **V** vec3,
**C** colour hex, **Step** enum string (no interpolation).

### camera (target `"camera"`)

| channel | kind | evidence |
|---|---|---|
| `eye` | V | §14.4 keys; `eval_camera_keys` (scenefile.rs:775) |
| `target` | V | same |
| `fovY` | S | same |

### lights (target `"lights"`)

The §14.2 rig, evaluated per frame by film.rs into `LightState`
(film.rs:258 — "folded linear (colour × intensity)" fields, point
states at film.rs:243). Post-atmosphere values, per **R1**.

| channel | kind | | channel | kind |
|---|---|---|---|---|
| `key.intensity` | S | | `hemisphere.intensity` | S |
| `key.color` | C | | `hemisphere.sky` | C |
| `key.position` | V | | `hemisphere.ground` | C |
| `key.target` | V | | `envIntensity` | S |
| `rim.intensity` | S | | `points[i].intensity` | S |
| `rim.color` | C | | `points[i].color` | C |
| `rim.position` | V | | `points[i].position` | V |
| | | | `points[i].distance` / `points[i].decay` | S |

(`i` is the index into the file's `lights.points` array, as written:
`points[0].intensity`.) Not channels: `key.shadow*` / `bias` —
shadow *configuration*, never time-varied anywhere in the engine.

### post (target `"post"`)

The §9 blocks as parsed at scenefile.rs:999–1022, evaluated into
film.rs's `PostState` (film.rs:272: bloom, aperture, maxblur, sat,
warm, vig, grain, exposure).

| channel | kind | | channel | kind |
|---|---|---|---|---|
| `bokeh.focus` | S | | `grade.sat` | S |
| `bokeh.aperture` | S | | `grade.warm` | S |
| `bokeh.maxblur` | S | | `grade.exposure` | S |
| `bloom.threshold` | S | | `grade.vignette` | S |
| `bloom.strength` | S | | `grade.grain` | S |
| `bloom.radius` | S | | | |

**`grade.time` is deliberately NOT a channel** — see §5.3 (R3). The
validator rejects a track on it by name, citing the ruling.

### fog (target `"fog"`)

`near` (S), `far` (S), `color` (C) — the top-level fog block of the
loader addendum (§14.5: `{color, near, far}`).

### character nodes (target = the node's `name`)

The §14.3 `state` fields as parsed at scenefile.rs:647–669 and
painted by toonfull.rs. Pair fields split into components because
the film's own data already tracks them separately (s09 carries
`keys` for armR[0] and `elbowKeys` for armR[1]).

| channel | kind | | channel | kind |
|---|---|---|---|
| `position` | V | | `lean` | S |
| `armL.0` / `armL.1` | S | | `bob` | S |
| `armR.0` / `armR.1` | S | | `squash` | S |
| `look.0` / `look.1` | S | | `headTilt` | S |
| `blink` | S | | `headTurn` | S |
| `talk` | S | | `camPitch` | S |
| `phase` | S | | `sitOn` | S |
| `height` | S | | `mood` | Step: the 17 strings of `Mood::from_str` (toonfull.rs:55) |
| | | | `legs` | Step: `stand walk run sit jump` (scenefile.rs:655) |

Note the strictness asymmetry: `Mood::from_str` *silently* falls
back to neutral on an unknown string; the validator **fails** an
unknown mood/legs value instead — clips are new data and a typo'd
mood should be caught at the gate, not rendered as neutral.

### other nodes (target = the node's `name`)

The transform channels the film's data already keys (s05a/s11
`translate` key tables; the root audit's `euler.z` record):

| channel | kind | | channel | kind |
|---|---|---|---|
| `translate` | V | | `rotate.x` / `rotate.y` / `rotate.z` | S |
| `scale` | V | | | |

### Easing names ↔ engine curves

The spellings are **exactly** the set W11's `Easing::from_name`
(anim.rs:98) accepts — the validator's list is copied from it:

| name (canonical) | accepted aliases | engine curve |
|---|---|---|
| `smoother` (default) | `smootherstep` | `smoother()` film.rs:130 / `smootherstep()` scenefile.rs:767 — camK, poseTrack, §14.4 camera keys |
| `linear` | — | `seg()`/`lerp` film.rs:109–114 — the one ease name v2 already spells (§14.4; s05a's plane law) |
| `easeOutCubic` | `eo` | `eo()` film.rs:117 |
| `easeInOutCubic` | `eio` | `eio()` film.rs:120 |
| `easeOutBack` | `eob` | `eob()` film.rs:125 |
| `step` | — | hold (poseTrack with moveSecs 0) — discrete channels; **required** on `mood`/`legs` keys |

## 4. Evaluation semantics (the contract W11 loads against)

For an instance at scene time `t`, with clip duration `D`:

```
raw = t * rate + offset
clamp:     tau = min(max(raw, 0), D)
loop:      tau = raw mod D            (result in [0, D))
ping-pong: m = raw mod 2D;  tau = m if m <= D else 2D - m
```

Per track at `tau`: find the bracketing keys `k0 (t0, value0)`,
`k1 (t1, value1)`; outside the key range, hold the end value. Inside:
`u = (tau - t0) / (t1 - t0)`, eased by `k1.ease`, then:

- scalar: `value0 + (value1 - value0) * eased` (film.rs `lerp`, computed in f64,
  cast to f32 — the film's motion formulas are f64 throughout);
- vec3: componentwise, same law;
- colour: convert both hex values to linear (the engine's colour
  pipeline, §4.1/§8), lerp componentwise in linear, as film.rs's
  `mix3` does (film.rs:230) — never lerp sRGB bytes;
- step: the value of the latest key with `t <= tau` (no easing,
  whatever the key says — the validator already requires `step`).

Determinism: a clip is a pure function of `t` — no state, no frame
history — matching the format's ADR-001 Decision 3 stance that
animation is `frame(t)`, and the wave's determinism contract. Loop
modes do not change this (the formulas above are closed-form).

## 5. Migration: what in the current film data becomes clips — and what must not

Surveyed from `scenes/film/*.scene.json` (the `channels` audit
blocks and `camera.keys`) and film.json. This is the adoption map
for the lead; nothing here was migrated by this seat.

### 5.1 Migrates directly (already key data)

| Current data | Becomes |
|---|---|
| `camera.keys` in all 14 segment files (§14.4) | one clip per shot instanced on `camera` (tracks `eye`/`target`/`fovY`), *or* per-shot clips stay as `keys` — both forms are legal data; the validator only forbids carrying both on one camera |
| poseTrack channels: s01 `headTurn`; s03 `armR`; s05b `armL`; s07 `armR`; s09 `armR` + `elbowKeys`; s11 `armR`/`elbow`/`lean`; s12 `lean`/`headTilt` | character scalar tracks (`armR.0`, `armR.1`, …) via the §5.2 conversion |
| Object-form node keys: s05a plane + 16 leaflets, s11 falling rifle (`{t, value, ease}` tables) | node `translate` tracks — the shape is already the clip key shape, field names included |
| s06 `position` keys `[[t, [x,y,z]], …]` | character `position` tracks |
| Step data: s12 `mood` keys, s10 `mood` steps | `mood` tracks with `ease: "step"` |

### 5.2 The poseTrack conversion (exact, mechanical)

A poseTrack moves into key `k` over the `moveSecs` *before* `t_k`
and holds otherwise. A clip track interpolates across the *whole*
segment. So each poseTrack key `[t, v, m]` with `m > 0` converts to
**two** clip keys: a hold key `{t: t - m, v: <previous value>}`
(unless one already sits there) and the arrival key
`{t: t, v: v, ease: "smoother"}`. Keys with no moveSecs (the
`[t, v]` form, default 0.35 in `key()` — film.rs:143) convert the
same way with `m = 0.35`. The converted track evaluates
**identically** to `pose_track()` at every `t`; this is a
re-spelling, not a re-timing.

### 5.3 Must NOT migrate

| Data | Why not |
|---|---|
| **`post.grade.time`** | **R3 is a law, not data:** "GRADE grain time is 0 for the whole film" (§14.5 R3) — no source path ever sets it. A clip track on it would animate grain away from Chrome on every frame. The validator rejects it by name (fixture `bad-grade-time-r3`). |
| Fire flicker `fireLight.intensity` formula `(4.6 + 0.7·sin(17t) + 0.5·sin(7.3t))·level` (s04/s05a root channels) | a procedural law over the **R1 basis 4.6**, not keys. Baking it into keys would sample a continuous law into data and break the R1 cross-check. (Keying the fire's *level* envelope over a shot would be legitimate new authoring — it is not this film's data.) |
| `walkPhase` / `walkBob` / `segLerp` position drivers (s02, s06, s07, s08, s11) | derived from *travelled distance* / shared drivers (`sharedBy` groups march in lockstep); they are functions of another channel's integral, not independent key data. The *position endpoints* migrate (§5.1); the gait derivation stays code. |
| `blink` directors (`blinkDirector(seed)`, `blinkAt` beats) | seeded procedural system (film.rs `Blinker`, IV pattern rotation) — a seed is not a key table. |
| `talk()` waveform, `settle()` shakes, `camPitchOf` (s05b/s08b/s01) | closed-form functions of `t` or of the evaluated camera; keying them would freeze today's sampling of a law. |
| Pose bundles & stage swaps (s09/s10 pose ternaries, s11 `visible` windows) | pose *data* (`POSES.listen/think`) and structural instance swaps, not channel keys. Their already-scalar keyed components (s10's `armR` poseTrack) migrate per §5.1; the bundle switch itself belongs to the pose system. |
| film.json segment timing (`startS`/`endS`, caption chunks) | manifest bookkeeping for the driver, not scene animation; clips live inside scene files and know nothing of segments. |

## 6. The validator and its fixtures

`tools/format/check_clips.py` follows the repo tools conventions
(validate_film.py): numbered checks, per-check `PASS`/`FAIL` with
specific failure lines, an `OVERALL:` verdict, exit **0** pass /
**1** any check fails / **2** input error (one-line `ERROR:` on
stderr, no report). Checks: **1 structure · 2 channels ·
3 keys · 4 references · 5 f32** — as itemised in the tool's
docstring. Run it:

```bash
python3 tools/format/check_clips.py scenes/fixtures/clips/*.scene.json
```

Fixtures (one valid, one per failure class; each bad fixture fails
**exactly one** check, so the failing check names the class):

| fixture | result |
|---|---|
| `valid-clips.scene.json` | PASS 5/5 — camera + light + character + mesh tracks, 4 clips, 4 instances (incl. ping-pong mapping) |
| `bad-unknown-channel.scene.json` | FAIL check 2 |
| `bad-unsorted-keys.scene.json` | FAIL check 3 |
| `bad-duplicate-key-time.scene.json` | FAIL check 3 |
| `bad-key-beyond-duration.scene.json` | FAIL check 3 |
| `bad-unknown-easing.scene.json` | FAIL check 3 |
| `bad-clip-cycle.scene.json` | FAIL check 4 (both cycles reported) |
| `bad-grade-time-r3.scene.json` | FAIL check 2 (R3 guard) |

Actual output — valid fixture:

```
check_clips.py — scenes/fixtures/clips/valid-clips.scene.json (4 clips, 9 tracks, 4 instances)

[1 structure] PASS — no failures

[2 channels] PASS — no failures

[3 keys] PASS — no failures

[4 references] PASS — no failures

[5 f32] PASS — no failures

OVERALL: PASS (5/5 checks passed)
```

Actual output — the seven failure fixtures in one run (exit 1):

```
check_clips.py — scenes/fixtures/clips/bad-unknown-channel.scene.json (1 clips, 1 tracks, 0 instances)

[2 channels] FAIL — 1 failure(s)
    clips.bad-zoom.tracks[0]: unknown channel 'camera.zoom' — camera channels are: eye, fovY, target

check_clips.py — scenes/fixtures/clips/bad-unsorted-keys.scene.json (1 clips, 1 tracks, 0 instances)

[3 keys] FAIL — 1 failure(s)
    clips.bad-order.tracks[0].keys[2].t: key times out of order — 1.3 follows 2.6 (keys[1]); times must be strictly increasing

check_clips.py — scenes/fixtures/clips/bad-duplicate-key-time.scene.json (1 clips, 1 tracks, 0 instances)

[3 keys] FAIL — 1 failure(s)
    clips.bad-dupe.tracks[0].keys[2].t: duplicate key time 2.6 (already at keys[1]) — two values at one instant is ambiguous

check_clips.py — scenes/fixtures/clips/bad-key-beyond-duration.scene.json (1 clips, 1 tracks, 0 instances)

[3 keys] FAIL — 1 failure(s)
    clips.bad-range.tracks[0].keys[1].t: 2.5 is beyond the clip duration 2.0 (clip 'bad-range')

check_clips.py — scenes/fixtures/clips/bad-unknown-easing.scene.json (1 clips, 1 tracks, 0 instances)

[3 keys] FAIL — 1 failure(s)
    clips.bad-ease.tracks[0].keys[1].ease: unknown easing 'bounce' — the engine's curves are: linear, smoother, smootherstep, step, easeOutCubic, eo, easeInOutCubic, eio, easeOutBack, eob

check_clips.py — scenes/fixtures/clips/bad-clip-cycle.scene.json (3 clips, 3 tracks, 1 instances)

[4 references] FAIL — 2 failure(s)
    clips: reference cycle: cycle-a -> cycle-b -> cycle-a
    clips: reference cycle: cycle-self -> cycle-self

check_clips.py — scenes/fixtures/clips/bad-grade-time-r3.scene.json (1 clips, 1 tracks, 0 instances)

[2 channels] FAIL — 1 failure(s)
    clips.bad-grain.tracks[0]: channel 'post.grade.time' is not animatable: R3 pins the GRADE grain time to 0 for the whole film (docs/scene-format.md §14.5 R3)
```

(Passing check sections elided per file above; every bad fixture
shows `OVERALL: FAIL (4/5 checks passed)` — exactly one failing
check each, as designed.)

Input-error path (exit 2, stderr, no report), also verified:

```
$ python3 tools/format/check_clips.py scenes/film/s01.scene.json
ERROR: scenes/film/s01.scene.json: no 'clips' block and no clip instances — nothing to validate
```

## 7. Adoption steps and reconciliation flags (for the lead / W11)

**Adoption (lead):** (a) fold §2–§4 into `docs/scene-format.md` as a
new §15 (format v2 additive field — a v1 file carrying `clips` must
be rejected like other v2 fields, per §14's rule); (b) loader:
parse `clips` + instances in scenefile.rs, evaluate per §4 at the
file's `time`; (c) film lane: evaluate at frame `t` and feed the
existing `LightState`/`PostState`/character-state paths; (d) wire
`check_clips.py` into tools-ci (the workflow already triggers on
`tools/**` and `scenes/**` paths) and run it over migrated segment
files when §5.1 is executed.

**Reconciliation flags — decisions this seat made that W11/the lead
may need to overrule:**

1. **Names are aligned; placement still differs.** Easing
   spellings, `value`, and the wrap modes were aligned to
   `src/anim.rs` when it landed mid-seat (see the header note).
   What remains: anim.rs puts `wrap` and an effective `duration`
   on each **Track**, while this model puts `duration` on the
   **clip** (one bound for all its tracks — the validator's
   beyond-duration check needs a single bound) and the wrap mode
   on the **instance** (`clipTime.loop`, since two instances of
   one clip may legitimately map time differently). The loader
   bridges this mechanically (clip duration → each track's
   duration; instance mode → each track's wrap), but the lead
   should ratify the placement. Related: anim.rs also has
   quaternion tracks (`Track<Quat>`); this vocabulary spells
   rotation as euler scalars (`rotate.x/y/z`) because that is
   what the scene data carries — a `rotation` quat channel is a
   possible addition, deliberately not invented here.
2. **Key semantics are the §14.4 whole-segment model**, not
   poseTrack's hold-then-move; §5.2's conversion keeps the film's
   poseTrack data exact under this choice. If W11 implemented
   moveSecs-style keys instead, the grammar gains an optional
   per-key `move` field and §5.2 is deleted.
3. **`uses` layering is new data surface** (§2.4) — it exists
   because "clip reference cycles" was a required failure class,
   and cycles need a reference graph. If the evaluator rejects
   layering, drop `uses` and check 4's cycle/layering half goes
   with it; instances and the rest of the model are unaffected.
4. **Validator strictness exceeds loader leniency in two named
   places:** unknown `mood`/`legs` strings fail (loader falls back
   silently), and unknown channels fail (the §1 ethos ignores
   unknown *fields* — but a channel name is a discriminating value,
   and §1 already hard-errors on unknown discriminators).
5. **`clip` vs `camera.keys`**: the validator forbids both on one
   camera rather than defining precedence — precedence is an
   evaluator decision; the data should not carry the ambiguity
   into it.
