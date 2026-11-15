# CR-B1-01 — §15 evaluator step-2: prefab expansion + clip evaluation (TaleRender, Rust)

**Deliverable:** `src/eval15.rs` (new file only, ~4,020 lines incl. tests;
nothing existing edited, nothing committed). Repo HEAD at time of work:
`b093f3a` ("Chrome replacement: Phase 0 contracts").
**Contract:** CHROME-REPLACEMENT-CONTRACTS.md — §15.10 adoption step 2
(step 1, the reservation guard, is already landed in `scenefile.rs` /
`film.rs`). **Cross-seat input incorporated:** CR-B1-05 (cues) R5/R6/R7 —
clip-event emission, §5 of this note.

**Status of the module:** compiles standalone and **36/36 in-file unit
tests pass** under a `rustc --edition 2021 --test` harness in /tmp (stub
`scene` module carrying only `Value`, which is eval15's sole crate
dependency). That harness is a sanity check only — **CI is the gate**,
per the wave rules. No Cargo.toml change is needed (no new deps: std +
`crate::scene::Value` only).

---

## 1. Design

`eval15` is a pure document→document / document→values layer over
`crate::scene::Value`. No wgpu, no I/O. Two halves mirror the spec's own
split, plus the event side channel (§5):

### 1.1 Prefabs (§15.2–15.5) — `expand_prefabs(root, libraries)`

Load-time macro, exactly the §15.5 order: assemble scope (inline
`prefabs` + library blocks; duplicate name anywhere = hard error, no
shadowing) → validate **every** definition eagerly (used or not) →
static reference-graph cycle check (error prints the path, e.g.
`loop-a -> loop-b -> loop-a`) → validate every instance → expand
depth-first: (a) effective params (supplied + defaults), (b) deep-copy
template, (c) apply pid-addressed overrides by deep-merge, (d) substitute
`{"$param": name}` slots whole-value, (e) recursively expand nested
instances, (f) strip `pid`s, (g) wrap in a transform group / set `name`
only when the instance carries them. The `prefabs` block is consumed and
dropped; a residue assertion on exit guarantees no `prefab` / `pid` /
`$param` survives anywhere in the document.

Semantics mirror `check_prefabs.py` check-for-check: closed key sets
(instance `{prefab, params, overrides, transform, name}`; declaration
`{type, required, default, description}`; definition
`{params, node, description}`); each param is exactly one of
`required: true` XOR `default`; defaults are literal (no slot) and
type-checked; slot = single-key object, whole-value substitution,
verbatim (no arithmetic/interpolation), lexically scoped, forbidden as
object key / as the value of `"prefab"` / inside declarations; `pid`
scoped to the direct scope (lookup does not descend into a nested
instance's interior), unique per template, stripped on expansion.
Param types: `number` (never bool), `string`, `color` (`#RRGGBB` string
only — the `{hex, shade}` object form is type `object`), `boolean`,
`vec3`, `array`, `object` (may not be a slot or an instance), `node`,
`nodes`. Forwarding: a nested instance may supply a slot as a param
value; declared types must match exactly, with the single widening
`color → string`. Overrides (§15.4) merge **before** substitution:
both-plain-objects recurse, anything else replaces wholesale, arrays are
never merged by index; forbidden at any depth: `type`, `kind`,
`generator`, `id`, `prefab`, `pid`, `transform`, any key whose template
value is a slot, and `null` (no deletion). Expansion depth guard 64
(the reference validator's value).

### 1.2 Clips (§15.6–15.8) — `ClipSet`, `evaluate_clips`

Two consumption shapes over one validated model:

- **Typed sampling** — `ClipSet::from_root(&Value)` validates and holds
  the `clips` block; `ClipSet::sample(clip, &ClipTime, scene_t)` returns
  `Vec<(node, channel, Sampled)>` for the clip's layered tracks
  (`uses`-transitive, own tracks first). For per-frame consumers (the
  film driver; the s12 retirement, §4).
- **Document fold** — `evaluate_clips(root)` discovers every instance
  (camera/lights/post/fog blocks in pair order, then nodes recursively),
  samples each at the file's `time`, folds values into the targets'
  static fields, strips all `clip`/`clipTime` fields, drops the `clips`
  block, and asserts no residue. For the still/scene loader path.

Validation mirrors `check_clips.py` checks 1–4: clip `{duration > 0,
tracks non-empty, uses?}`; key `{t, value, ease?}` with `t ∈
[0, duration]`, strictly increasing; ease on the **arriving** key
(§15.6), first key's ease ignored, default `smoother`; the 10 easing
spellings of `anim::Easing::from_name`; the full §15.7 channel
vocabulary (camera `eye/target/fovY`; lights incl. `points[i].*`; post
incl. the deliberate **rejection of `grade.time` by name, citing R3**;
fog; character channels incl. `armL.0/.1`-style component pairs; plain
nodes `translate/scale/rotate.x|y|z`); step channels (`mood` — the 17
`Mood` names, validated strictly, **not** via `Mood::from_str`'s silent
Neutral fallback; `legs` — 5 states) whose keys must carry ease `step`;
targets resolve against post-expansion node names; a node named as a
reserved target is an error; `uses` acyclic (self-use counts) and a
layered set may not define the same target+channel twice; camera may not
carry both `clip` and `keys`; `clipTime` without `clip` is an error.
**Check 5 (the §11 f32 law) is deliberately NOT replicated** — §15.3
assigns it to `validate_film.py` over the processed document.

Sampling law (§15.8): `raw = t·rate + offset`; clamp to `[0, D]`, loop
`raw mod D`, ping-pong over `2D`; scalar/vec3 interpolation computed in
**f64, cast to f32 at the boundary** (a deliberate divergence from
`anim`'s f32-native tracks, ~1 ulp — the reason §15.10 A/B-gates
adoption per site); colour keys converted hex→**linear before** the
track carries them and lerped in linear (§15.9), delivered as
`Sampled::ColorLin`; step values ride `Sampled::Step`, never a numeric
lane. The §15.9 ease remap is internalised: the sampler reads ease from
the arriving key directly; if sampling is ever re-based on `anim::Track`
(easing-to-next), the remap (format `key[i+1].ease` → anim `key[i].ease`)
must be applied in `ClipSet` construction — this is stated in the module
doc so it cannot be lost.

`pose_track_to_clip_keys` + `sample_scalar_keys` are public: the
§15.8 poseTrack conversion (`[t, v, m]`, `m > 0` → hold key at `t−m`
with the previous value + arrival `{t, v, ease: "smoother"}`; missing
`m` → 0.35; `m == 0` → arrival ease `step`; overlapping move windows
fail, as in `stage_clips.py`) — the bridge between film.rs's
`pose_track` and clip data.

### 1.3 Orchestration — `process_scene(root, libraries)`

Duplicate-JSON-key scan (only when §15 constructs are present — `Value`
preserves duplicate pairs, and the checkers treat duplicates as
defects), then `expand_prefabs`, then `evaluate_clips`. Version 2 is
required when either extension is used (§15.1); a construct-free
document (incl. every v1 still) passes through unchanged.

### 1.4 Fold rulings (where the spec/checkers are silent — flagged, not hidden)

- Plain-node `translate/scale/rotate.*` fold into the node's transform
  op list: an existing op of the same kind is replaced in place, a
  missing op is appended (order-preserving).
- Character component pairs (`armL.0/.1`, `look.0/.1`, …): an absent
  pair is initialised from the loader's own defaults — armL/armR
  `(8, 6)`, look `(0, 0)` (scenefile.rs character branch) — before the
  sampled component is written.
- `points[i]` beyond the authored array extends it with empty objects.
- **Cross-instance conflicts** (two instances whose clips write the same
  target+channel): neither §15 nor `check_clips.py` defines this — the
  no-duplicate rule exists only inside one clip's layered `uses` set.
  eval15 folds in document order, **last instance wins**, and this note
  flags it as a spec gap for a ruling (§6).

---

## 2. Integration points (the lead's act — exact)

eval15 is inert until wired. Three edits, all in existing files:

1. **`src/main.rs`, after line 31 (`mod errors;`):** insert
   `mod eval15;` (alphabetical position; the module list is sorted).

2. **`src/scenefile.rs`, `fn load` (line 834):** replace the §15.1
   reservation-guard block, **lines 853–872** (the comment + the two
   top-level `prefabs`/`clips` checks + the camera/lights/post/fog
   `clip`/`clipTime` loop), with consumption, immediately after the
   version match and before any downstream use of `root`:

   ```rust
   let root = crate::eval15::process_scene(root, &[])
       .map_err(|e| format!("{path}: {e}"))?;
   ```

   The `&[]` is the libraries argument: no library-loading CLI/path
   exists yet, so inline `prefabs` only until one is specified.
   **Keep** the `walk_nodes` guard (lines 514/517, "prefab extension
   not supported" / "clips extension not supported"): after processing
   it is unreachable for valid documents and becomes a free residue
   assertion — it can only fire on an eval15 bug. v1 stills are
   unaffected: `process_scene` is a no-op on construct-free documents.

3. **`src/film.rs`: no change for the loader wiring.** The
   `load_film_data` guard (lines 86/89) stays as-is — film data
   (`film.json`, `onoda.json`) is not a scene document and eval15 does
   not apply there. film.rs changes only as part of the separately
   gated s12 retirement (§4.3).

CI expectations on the wiring commit: CI + Tools CI green at step level;
`FILM TELEMETRY: CHECK OK`; Onoda frames byte-identical (the wiring is
behaviour-neutral for all current data — no production scene file
carries prefabs/clips yet, §15.10 step 3 is production data, last).

---

## 3. Test coverage

36 in-file tests, all passing in the standalone harness. Mapped to the
authority they mirror:

- **Prefabs (check_prefabs.py, 6 checks; fixtures
  `scenes/fixtures/prefabs/`):** substitution incl. into vec3
  components; defaults; pid stripping; override deep-merge; transform
  wrapper + `name` on the outermost node; no-wrapper without transform;
  nested forwarding; forwarding type mismatch; `color → string`
  widening accepted, reverse rejected; cycle error printing
  `loop-a -> loop-b -> loop-a`; duplicate definition; missing-required /
  undeclared-param / param-type / unknown-prefab / closed-key-set /
  bool-is-not-number errors; definition errors (required+default,
  neither, slot in default, undeclared slot, duplicate pid); override
  errors (forbidden `type`, forbidden `transform`, unknown pid, `null`,
  through-slot); arrays replace wholesale; `object` params replace
  wholesale (no field merge); `node`-param injection with recursive
  expansion; slot in scene data outside any template; 70-deep acyclic
  chain hitting the depth-64 guard.
- **Clips (check_clips.py checks 1–4; fixtures
  `scenes/fixtures/clips/`):** document fold of camera + character at
  the file's time with consumption assertions; wrap modes (clamp /
  loop / ping-pong incl. negative raw, rate + offset); arriving-key
  ease semantics (probe at u = 0.25: linear 0.25 vs smootherstep
  0.103515625); colour lerp in linear space (midpoint of a 0→1 linear
  ramp folds back to hex `#BCBCBC`; hex round-trip exact); step
  hold/fire boundaries; every validation error class (R3 `grade.time`,
  unknown channel, unknown easing, duplicate key time, unsorted keys,
  key beyond duration, bad mood, step channel without ease `step`,
  duplicate track, reserved-name collision); `uses` layering happy
  path + conflict + self-cycle + unknown `uses`; instance errors
  (camera clip+keys, unknown clip, rate 0, bad loop mode, `clipTime`
  without `clip`); fold of lights `points[0]`, post bokeh, plain-node
  transform ops replaced in place; pair-component fold with loader
  defaults; scalar hold/constant sampling.
- **poseTrack conversion:** converted keys sample identically to a
  transcribed f64 reference of film.rs `pose_track` (< 1e-9);
  overlapping move windows fail.
- **s12 evidence test** — §4.2.
- **Events (R5/R6)** — §5.4.
- **End-to-end:** `process_scene` over a prefab+clip document;
  duplicate JSON keys on the §15 path are an error.

Fixture baselines re-run while building (all green): `check_prefabs.py`
on `valid.scene.json` + library → PASS 6/6; `check_clips.py` on
`valid-clips.scene.json` → PASS 5/5; `check_format15.py` on the valid
prefab fixture → PASS 2/2.

---

## 4. S12 evidence pack — the bow code path is reproducible; retire it byte-probe gated

### 4.1 The code path (at HEAD `b093f3a` — line numbers verified, not relayed)

`fn s12` in `src/film.rs`, lines **2080–2115**. The bow is three lines:

```rust
// film.rs:2098-2100
let bow = pose_track(&[key(0.0, 0.0), keym(w_loyal - st0 - 0.1, 3.5, 0.45)], lt);
onoda.lean = (0.6 * bow) as f32;
onoda.head_tilt = (-bow) as f32;
```

with `st0 = 55.520` (line 2081) and `w_loyal = data.anchors["loyal11"]`
(line 2082); `film/onoda.json:72` gives `loyal11 = 58.12`, so the move's
arrival key sits at local time `58.12 − 55.520 − 0.1 = 2.5` exactly, move
length 0.45. `bow` has **no other consumer** in `fn s12`. `w_loyal` also
drives the mood step (line 2104: `Relieved` before, `Neutral` at/after)
and the blink director (line 2105) — those are separate consumers, see
§4.3 for what retires and what does not.

> **Citation drift, flagged:** S12-ADOPT-W83.md cites these lines as
> film.rs:2021–2023 at its HEAD `021ca01`. At `b093f3a` they are at
> **2098–2100** (`fn s12` at 2080). Substance unchanged; anyone landing
> the patch must work from the current numbers.

### 4.2 The data side already landed — what remains is one atomic act

Commit `52ca55f` (F3-b/c data landing + S12-ADOPT-W83) already baked the
`apply` formulas into `scenes/film/s12.scene.json`: the Onoda node's
`lean` channel carries poseTrack keys `[[0, 0], [2.5, 2.1, 0.45]]` (the
0.6 factor multiplied through: 3.5 × 0.6 = 2.1), `headTilt` carries
`[[0, 0], [2.5, −3.5, 0.45]]`, `mood` carries step keys
`[[0, "relieved"], [2.6, "neutral"]]` (2.6 = 58.12 − 55.52), each with a
`law` field recording provenance. **No `apply` field remains in any
scene file.** `blink` remains a blinkDirector — code-only by design
(§15.8 negative list).

Re-run this session: `stage_clips.py scenes/film/s12.scene.json` →
**PASS 4/4**, "2 clips, 6 tracks, 14 keys staged; 2 records code-only"
(the 2 blink directors), largest f32 rounding 1.91e-07; `check_clips.py`
on the output → **PASS 5/5**. The staged clip that carries the bow is
**`s12-onoda-jg-onoda2`**, duration 4.836999893188477:

- `lean`: `{0, 0}` · `{2.049999952316284, 0}` · `{2.5, 2.0999999046325684, smoother}`
- `headTilt`: `{0, 0}` · `{2.049999952316284, 0}` · `{2.5, −3.5, smoother}`
- `mood`: `{0, "relieved", step}` · `{2.5999999046325684, "neutral", step}`

(The second staged clip, `s12-camera`, is **not** part of this
retirement: s12's camera in film.rs runs the `cam_k(lt, d, 0.1)` crane
law, not clip sampling.)

**Equivalence, three independent legs:**

1. **W52's bake proof** (S12-BAKE-W52.md, re-verified by W83 at
   `021ca01` with zero drift): baked key values bit-identical f32 at key
   times (lean arrival 2.0999999046325684, bits `0x40066666`, both
   routes); the bake's own contribution ≤ 1 ulp everywhere (max
   2.98e-8); total mid-move deviation ≤ **3.576e-7** (headTilt at
   lt 2.2), all of it the standard §15.8/§11 conversion cost (the hold
   key sits at f32(2.05) = 2.049999952316284 vs the code's f64
   2.0499999999999998). One disclosed imperfection: headTilt key 1
   spells `0` where the code computes `−0.0` — equal under every
   comparison, no consumer reads the sign of exact zero.
2. **eval15's in-file test** `s12_baked_tracks_reproduce_the_code_path`:
   the staged keys above, sampled by this evaluator's
   `sample_scalar_keys`, against a transcribed f64 reference of
   `pose_track` with the 0.6 / −1 factors applied — bit-exact at holds
   and at the arrival (each route probed at its own key times),
   ≤ 4e-7 envelope mid-move, mood step exact. Passing (36/36 suite).
3. **Frame arithmetic:** segment s12 in `scenes/film/film.json`:
   startS 55.52, endS 60.357, startFrame 1666, endFrame 1811
   (exclusive) → **frames 1666–1810** at 30 fps (frame n staged at
   t = n/fps). The move window lt ∈ (2.05, 2.5) strictly covers
   **frames 1728–1740** (frame 1727: lt = 2.0467 → exactly 0; frame
   1741: lt = 2.5133 → arrived). W83's probe window 1727–1741 is the
   safe superset (move frames + both bracketing hold/arrival frames);
   this refines, does not contradict, W83.

**Verdict: REPRODUCIBLE — within the already-accepted conversion
envelope, NOT bit-identical mid-move.** Hold frames (1666–1727) and
arrival frames (1741–1810) are expected byte-identical; the 13 strictly
moving frames (1728–1740) may differ by ≤ ~3.58e-7 in channel value.
The retirement must therefore be gated on **byte probes with PSNR
judgement on the move window**, not on byte identity (§4.3).

### 4.3 The retirement patch (for the lead to land — do not land piecemeal)

W52's atomicity law: the baked data and the code path must never coexist
as parallel sources of truth. The data half is already in; the remaining
act is **one commit** containing both halves:

1. **Data:** merge the `stage_clips.py` output into
   `scenes/film/s12.scene.json`: add the top-level `clips` block with
   `s12-onoda-jg-onoda2`, and put `"clip": "s12-onoda-jg-onoda2"` on the
   "Onoda (JG onoda2)" node. Do **not** instance `s12-camera` (the
   film.rs camera stays `cam_k` code). The staged file's own instance
   stub agrees (node clip + camera clip — take only the node half).
2. **Code (`fn s12`):** delete lines 2098–2100 (the `bow` pose_track +
   the lean/head_tilt assignments). Source the three channels from the
   clip instead: parse `scenes/film/s12.scene.json` once alongside
   `load_film_data` (the lead picks the exact load point — film.rs
   already resolves the film data directory), build
   `ClipSet::from_root`, and per frame assign from
   `sample("s12-onoda-jg-onoda2", &ClipTime::default(), lt)`: `lean` ←
   Scalar, `head_tilt` ← Scalar, and the mood line 2104 — which is
   exactly the clip's step track — may retire in the same patch
   (`mood` ← `Sampled::Step` → `Mood::from_str`, safe here because
   eval15 validated the names strictly at load). **Do not retire:**
   blink (2105, negative list), the `cam_k` camera (2092–2096), the
   rig/post/fog constants, `rifle_down_draws`, or the Tani statics.
   Interim minimal variant, if the lead prefers it: keep the values
   flowing through `sample_scalar_keys` over the converted keys
   (`pose_track_to_clip_keys`) — same numbers, smaller diff, but it
   leaves a second data copy in code; the ClipSet route is the honest
   end state.
3. **Gates, all in the adoption commit:** byte probes `--ranges
   1666-1810` against a pre-adoption render (the standing probe sets
   195-204 / 795-814 / 1195-1204 do **not** cover s12) — expect
   identical outside 1728–1740, PSNR-judge inside; `FILM TELEMETRY:
   CHECK OK`; conversion census re-run (14/14 files, 65/65 records with
   the baked s12).
4. **Frozen-factor warning (W52/W83 §3):** after adoption the data says
   `2.1` / `−3.5`, not "0.6 × bow" — the `law` fields in the scene file
   carry the provenance. Regeneration is **not** "run bake_s12.py": its
   guard refuses on the adopted file, by design.

---

## 5. Clip events — CR-B1-05 R5/R6 incorporated

B1-05 landed the native cues emitter (`cues_native.py`, byte-identical
cues.json on Onoda) and set bridge/evaluator requirements R1–R8. R5/R6
bind this seat; R7 binds its validation. What was built, and where it
conflicts with §15 as written (§5.3 — read before using events in
production data):

### 5.1 The emission surface (what the evaluator returns alongside staging)

Events are **not staging**: `evaluate_clips` / `process_scene` never
fold them into the document, and a test asserts no event data survives
processing. They leave the evaluator only through:

- `ClipSet::events_in_window(clip, &ClipTime, segment_start, w0, w1)
  -> Vec<EmittedEvent>` — one instance.
- `ClipSet::collect_events(&[(clip, ClipTime)], segment_start, w0, w1)`
  — merged across instances, sorted by `t` (document order breaks ties).
- `scene_events(root, segment_start, w0, w1)` — document level:
  discovers instances exactly as `evaluate_clips` does and collects.
  Call it on the expanded document **before** `evaluate_clips` consumes
  the `clips` block, or on a fresh parse.
- `EmittedEvent { t: f64, cue: String, gain: Option<f64>, dur:
  Option<f64> }` with `to_value()` producing the R5 shape
  `[{t, cue, gain?, dur?}]` — keys in that order, absent opts **omitted**
  (mirroring authored-cue discipline). This output feeds the
  `clip-events.json` sidecar (B1-05's routing, in `build_cues` input
  shape) — **never cues.json**: event names are not mix.py FX types and
  would be silently skipped there (SKILL.md §5.7).

Input side: a clip definition may carry
`events: [{t, cue, gain?, dur?}]` with clip-local `t` — the data form of
the engine's `ClipDef.events?: ClipEvent[]` (`core/clips.ts:158`).

### 5.2 Time law, windows, looping, layering, vocabulary

- **Time law:** an event at clip-local `te` sits at scene-local
  `(te − clipTime.offset) / clipTime.rate` — the exact inverse of
  §15.8's `raw = t·rate + offset` — and at absolute film time
  `segment_start + scene-local`. With the default ClipTime (offset 0,
  rate 1) this **reduces exactly to R5's formula** `startS + ClipEvent.t`.
  Events whose scene-local time is negative (clip-local `t` before the
  instance's entry offset) never fire.
- **Windows are half-open `[w0, w1)`:** a driver stepping frames with
  contiguous windows `[n/fps, (n+1)/fps)` receives every event exactly
  once. (Single-instant sampling cannot define firing; the window is
  the contract.)
- **Looping (R5/cliplib law):** instances with `clipTime.loop` =
  `loop` **or** `ping-pong` emit nothing. `ClipSet::has_events(clip)`
  lets a caller distinguish "no events authored" from "events
  suppressed by looping".
- **Layering:** a `uses`-layered clip contributes its events
  transitively, own events first — the same rule as tracks (§5.3.6).
- **Vocabulary (R7, closed):** `cue` must be in `EVENT_NAMES` (the 11 of
  `audio_recipes.EVENT_VOCABULARY`: takeoff, land, fall-impact, trip,
  slip, recover, lose-balance, roll, push-up, pop, inhale) or in
  `FX_NAMES` (mix.py's 21 FX — an FX-named event is a literal placement
  in build_cues terms). Anything else fails at load, loudly, listing
  both vocabularies. Validation also enforces: entry is an object with
  closed keys `{t, cue, gain, dur}`; `t ∈ [0, duration]`; `gain`/`dur`
  numeric when present.

### 5.3 Conflicts with §15 as written — flagged, not silently resolved

1. **§15.6 has no events.** The clip grammar is `{duration, tracks,
   uses}`; there is no events field anywhere in §15.6–15.8 (verified
   against the spec text and B1-05 §3). `events` in a clip definition
   is a **new field requiring a §15 amendment**. Until amended, note
   the §15.1 hazard: under §1's unknown-fields rule a non-consuming
   loader would *silently ignore* events — precisely the failure
   §15.1's reservation guard exists to prevent. Recommendation to the
   lead/spec owner: amend §15.6 to define `events`, and add `events`
   to the §15.1 reservation posture so a loader that does not
   implement emission must fail loud on a clip carrying it (eval15
   itself consumes it, so the guard question is moot here).
2. **Checker divergence:** `check_clips.py` does not look at `events`
   at all — a file with malformed events passes the checker and fails
   eval15. This is deliberate (a consumer that reads a field validates
   it, the §15.1 posture), but it is a real divergence from the
   "checkers are the semantic authority" rule and is recorded here as
   such; the checker should grow event validation with the §15
   amendment in (1).
3. **R5's formula vs §15 `clipTime`:** R5 says `t = segment startS +
   clip instance offset within the shot + ClipEvent.t`. §15 has no
   "instance offset within the shot" — instances are whole-scene
   bindings, and `clipTime.offset` is an *entry* offset (the clip is
   entered offset-seconds in), a different quantity with a colliding
   name. eval15's law (§5.2) is the generalisation: R5's formula is
   its offset-0/rate-1 special case, with placement carried entirely
   by `segment_start`. R5 also never mentions `rate`; the division by
   `rate` is eval15's extension of it, forced by §15.8's mapping.
4. **The looping law changes layer:** in cliplib it is a property of
   the clip *definition* ("looping clips carry no events"). In §15 the
   wrap lives on the *instance* (`clipTime.loop`) — the same clip can
   be clamped in one instance and looped in another. eval15 therefore
   applies the law per-instance at emission. Data authors cannot mark
   a clip itself event-free-for-looping; if that is ever needed it is
   another spec amendment.
5. **Ping-pong is an interpretation:** R5 names looping only.
   Ping-pong is equally periodic and unbounded, so eval15 suppresses
   its events too. Flagged for B1-05/lead confirmation — if they rule
   otherwise, the change is one match arm.
6. **Param-adjusted events are unreachable in §15 data:** R5's caveat
   (startle's `inhale` sits at the *default* `hit` time; moving `hit`
   moves the event) describes TS clip *generators*. §15 clips are
   static data, and prefab params — the only parameter mechanism —
   are lexically scoped to node-subtree templates (§15.3); slots are
   errors inside the `clips` block. So in native data the adjustment
   can only be **baked at authoring/bridge time**, exactly as R5
   assigns it to the bridge. If dynamic adjustment is ever expected
   of the evaluator, that is a spec gap, not an eval15 bug.
7. **`dur` is a superset of the TS type:** `ClipEvent` is
   `{t, cue, gain?}` (clips.ts:140) — no `dur`. The R5/build_cues
   shape carries `dur?`. eval15 accepts and passes it through
   verbatim. Harmless, but the two shapes should not be conflated.
8. **Impact-scaled gains are baked, not computed:** R6 asks for "the
   same absolute times the TS evaluation would produce (including
   impact-scaled gains from actions.ts)". actions.ts computes gains
   from impact at runtime; §15 data carries final values only, so the
   evaluator emits the authored gain verbatim. R6 is satisfied by
   data, not by computation — consistent with §15's staging-as-data
   philosophy, stated here so nobody expects a scaling law in eval15.
9. **`uses` layering of events is eval15's decision:** §15.6 layers
   *tracks*; it is silent on events. Dropping used clips' events
   would silently lose sounds, so they layer like tracks. If the
   spec amendment (1) rules differently, `layered_events` is the
   single place to change.

### 5.4 Event tests (7, in the 36)

R5-shape emission at absolute times (startS 55.52 → takeoff at 55.77,
land at 57.02; key order and opt-omission in `to_value()`); offset+rate
law (offset 0.5, rate 2 → land at scene 0.5; a pre-entry event never
fires); suppression under loop **and** ping-pong with `has_events`
still true; half-open window boundaries (event at `w0` fires, at `w1`
does not); `uses`-layered events; closed-vocabulary validation
(`kazoo` fails, FX `thud` passes, `t` beyond duration fails, unknown
key `pan` fails, non-numeric gain fails, numeric `dur` passes through);
document-level `scene_events` + the never-folded assertion.

---

## 6. Contradictions and open gaps (summary)

- **W83 line drift** (§4.1): s12 bow lines cited 2021–2023 at
  `021ca01`; at `b093f3a` they are 2098–2100. Substance unchanged.
- **Task framing vs repo state:** the s12 path was described as
  "slated for retirement once step-2 can reproduce its staging" — in
  fact the data side landed earlier (52ca55f); what remains is exactly
  the §4.3 atomic commit. Verdict: **reproducible within the accepted
  conversion envelope** (bit-exact at holds/arrival; ≤ ~3.58e-7
  mid-move) — gate on probes + PSNR, not byte identity.
- **Cross-instance clip conflicts** (§1.4): undefined by §15 and the
  checkers; eval15 = document-order last-wins. Needs a ruling if
  production data ever relies on it.
- **Events vs §15** (§5.3): nine flagged items, of which (1) the
  missing §15.6 amendment + §15.1 guard treatment and (5) the
  ping-pong reading most need a lead/spec-owner decision.
- **check_clips check 5** stays with `validate_film.py` by design
  (§1.2) — not missing coverage.
- Nothing in the mission contracts contradicted the code on this
  seat's scope: the contracts' claim that step-2 consumption is
  "guard-only" at `b093f3a` was verified true in `scenefile.rs`
  (load + walk_nodes) and `film.rs` (load_film_data).
