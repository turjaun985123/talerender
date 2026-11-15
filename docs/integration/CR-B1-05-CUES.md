# CR-B1-05 — Cues routing / native cues emitter (Chrome Replacement, Batch 1, Seat 5)

**Deliverable:** `~/workspace/custom-skills/tall-tale-shorts/template/tools/cues_native.py`
(stdlib-only Python; new file, nothing existing edited, nothing committed).
**Contract row:** CHROME-REPLACEMENT-CONTRACTS.md §1 row 5 — `cues` is
NOT-CHROME (routing). **Design input:** CUES-W45.md (F6) — its seven open
questions Q1–Q7 are answered from skill code in §2 below.
**Parity verdict: EXACT — byte-identical to the Chrome-produced file on
Onoda** (§4).

---

## 1. What was built

`cues_native.py [PROJECT_OR_NATIVE_DIR] [-o OUT.json] [--events EVENTS.json]
[--strict-events]`

- Discovers the bridged native data in either layout the bridge itself
  accepts (`package_bridge.py` findManifest): `<native>/film.json` or
  `<native>/scenes/film/film.json`; the argument may be a project dir
  (with `native/` inside), a native dir, or the manifest file itself.
- Reads the segment scene files **in manifest order** (segment `scene`
  paths are native-root-relative in both layouts), collects `shot.cues`
  (top-level `cues` accepted as a per-scene fallback; if both exist,
  `shot.cues` wins and the duplication is reported), normalises them
  (§2), sorts by `t`, and writes the contract file (§2) — by default to
  `<project>/data/cues.json`, mirroring `capture.mjs cues` including its
  console line shape (`data/cues.json: <n> cues, <dur> s`).
- An **unbridged project fails loudly (exit 2)** naming the bridge step
  ("Bridge it first … There is no Chrome fallback"). No output file is
  written on any failure.
- Clip events are **never merged into cues.json**; they are routed to a
  `clip-events.json` sidecar in `build_cues`' input shape (§3).
  `--strict-events` additionally exits 3 when any event was routed.

Exit codes: 0 written · 2 bad input / unbridged / contract violation ·
3 `--strict-events` tripped (file still written).

## 2. The exact cues.json contract (from code — this answers W45 Q1–Q7)

Producer side (Chrome flow, for reference):

- `template/tools/capture.mjs`, mode `cues`:
  `cues = OP.cues()`; writes
  `JSON.stringify({duration: dur, cues}, null, 1)` to `data/cues.json`
  (**envelope `{duration, cues}`, 1-space indent, no trailing newline**)
  and prints `data/cues.json: <n> cues, <dur> s`. `capture.ts` is identical.
- `template/src/main.ts:62`: `OP.cues = () => cues.slice().sort((a, b) => a.t - b.t)`
  — **sorted by `t` ascending**; the array is exactly what `buildShots`
  collected.
- `template/src/shots.ts:38` (same line in the Onoda project's `src/shots.ts:124`):
  `cue = (t, type, o = {}) => CUES.push({ t: +t.toFixed(3), type, ...o })`.
  The entry type is `Cue` in `template/src/core/shots.ts:27`:
  `{ t: number; type: string; gain?: number; dur?: number }`.

Consumer side:

- `template/tools/mix.py`: hard-requires the keys `duration` and `cues`
  (exit 2 otherwise); `DUR = float(cj['duration'])` sizes the mix canvas
  (`N = DUR*SR + SR//10`). Per cue it reads exactly: `c['type']` (must be
  a key of its 21-entry `FX` table — ding, chirp, chug, stamp, inhale,
  heartbeat, whoosh, slide, pop, plop, gulp, boing, bubbles, sizzle,
  sparkle, thud, slam, sting, tock, zip, alarm — else it prints
  `unknown cue type, skipped:` and continues), `c['t']`
  (`i = int(t*SR)`; placed only if `i >= 0` and samples remain),
  `c.get('gain', 1.0)`, `c.get('dur')` (passed as the synth's `d` **only
  if truthy**), `c.get('pan', 0.0)`.
- `template/tools/audio_recipes.py` `build_cues`: base cues pass through
  validated — type must be in the same FX table or `RecipeError`
  (deliberately loud where mix.py is silent); it copies exactly
  `t` (float), `type`, and `gain`/`dur`/`pan` when not None, in the emit
  order `t, type, gain, dur, pan`. Its own output adds top-level
  `music`/`report` keys and is documented mix-compatible.

Field-by-field contract the emitter writes:

| field | rule | evidence |
|---|---|---|
| top level | exactly `{duration, cues}`, 1-space indent, no trailing newline | capture.mjs serialisation; Onoda file bytes |
| `duration` | film/lines duration in seconds (see Q7) | capture.mjs `dur = OP.duration()` |
| `cues[]` order | sorted by `t` ascending, stable | main.ts OP.cues |
| entry `t` | absolute film seconds, **rounded to 3 dp** (half-up; mirrors `+t.toFixed(3)`) | shots.ts cue() helper |
| entry `type` | one of the 21 FX names | core/shots.ts Cue; mix.py FX |
| entry `gain` | verbatim passthrough, **key omitted when absent** (mix.py default 1.0) | cue() spread; mix.py `.get('gain', 1.0)` |
| entry `dur` | verbatim passthrough, **key omitted when absent** (mix.py then uses the synth's default length) | cue() spread; mix.py `if c.get('dur')` |
| entry `pan` | verbatim passthrough when present (not in the TS `Cue` type, but read by mix.py and emitted by build_cues) | mix.py `.get('pan', 0.0)`; audio_recipes emit |
| number spelling | integral values emitted without a decimal point (`1`, not `1.0`), mirroring JSON.stringify | Onoda file (`"gain": 1`-style authorship generalises); emitter normalises |

W45's open questions, answered:

- **Q1 (kind field):** `type`. The Onoda Chrome file spells the s05a
  whoosh `{t: 21.884, type: "whoosh", gain: 0.5, dur: 1.6}` — the scene
  data's `{at, name, …, source}` is a bridge-transcription variant, not
  the file format. Field order `t, type, gain, dur`.
- **Q2 (omitted opts):** the key is **absent** in the file; defaults live
  in mix.py (gain 1.0; synth default duration).
- **Q3 (mix.py edge behaviour):** unknown type → printed skip, mix
  continues; cue at/past `duration` → silently not placed (sample window
  empty); duplicate times → both placed (additive). The emitter is
  deliberately stricter *before* mix.py for the one case that is pure
  loss: an unknown authored type is a hard error (exit 2), mirroring
  build_cues' base-cue discipline. Out-of-range `t` is kept verbatim
  with a warning (the Chrome file would carry it too).
- **Q4 (extra keys):** mix.py tolerates them (it checks only
  `duration`/`cues` presence and reads named entry keys; build_cues'
  output proves extra top-level keys are accepted in practice). The
  emitter nevertheless writes the strict two-key envelope — byte parity
  is worth more than the tolerance.
- **Q5 (sorting):** mix.py does not rely on order (each cue is placed by
  absolute sample index); sorting is the OP.cues contract and the
  emitter keeps it.
- **Q6 (fades):** purely visual. `shot.fades` exists in scene data
  (s01/s12) and in main.ts's draw; no fade ever enters the CUES array,
  and mix.py has no fade cue concept.
- **Q7 (duration):** `OP.duration()` = `T.dur` = **`data/lines.json`'s
  `duration`** (`core/timing.ts`: `T.dur = lines.duration`). On Onoda:
  lines.json 60.357 = manifest `durationS` 60.357 = `film/onoda.json`
  `duration` 60.357 ≠ mix.wav 60.46. The emitter takes manifest
  `durationS`, falling back to the manifest's `sourceData` duration,
  then the last segment's `endS` — never the audio block's duration.

## 3. Clip-event resolution (evidence, not guesses)

**Question:** how do clip events (takeoff/land/fall-impact/…) enter
cues.json in the Chrome flow, and where do they live on the native side?

**Chrome flow — they don't enter cues.json at all.**

- The vocabulary: `template/src/core/clips.ts:140`
  `interface ClipEvent { t: number; cue: string; gain?: number }` —
  "**compiled to Cue[] by the recipe/shots seam (S3), not here**";
  `ClipDef.events?: ClipEvent[]` (clips.ts:158) with **clip-local** `t`.
  Producers: `core/actions.ts` (jump/trip/slip/fall/roll/push-up —
  events at takeoff/land/fall-impact/trip/slip/lose-balance/recover/
  roll/push-up, gains impact-scaled), `core/cliplib.ts:432,484`
  (celebrate `pop` at 0.36, startle `inhale` at 0.45 — "only cue names
  that exist in mix.py's FX table"), `core/interact.ts:341`,
  `core/physics.ts` PhysEvents (a separate `type`-spelled family, same
  S3 comment). `audio_recipes.py` mirrors the vocabulary as
  `EVENT_VOCABULARY` (11 names; `pop`/`inhale` double as FX).
- No code collects them into the file: grep over template + Onoda
  `src/` shows `.events` is consumed only *inside* actions.ts
  (sequence re-timing) and physics.ts — never read by `buildShots`,
  `main.ts`, or `OP.cues()`. `buildShots` returns only the `cue()`
  helper's array. SKILL.md §5.7 states the consequence: an event name
  written straight into cues.json is **skipped by mix.py** (not an FX
  type) and the beat gets no sound; the standard path is
  `build_cues(recipe, events, base_cues=cues.json)`, which maps events
  through a recipe and **fails loudly** on unmapped ones. The events
  input to build_cues is a *separate* file of `[{t, cue, gain?}]` in
  **film time** — assembling it (clip-local t + shot/clip placement)
  is today a manual author/agent step; no tool emits it.
- Onoda specifically: its `src/shots.ts` uses no clips/actions at all —
  exactly 9 `cue()` calls, matching its cues.json 1:1. There are no
  clip events in this film to lose.

**Native side — they live nowhere yet.**

- Scene data: none of the 14 Onoda `*.scene.json` files carries an
  events field (the only keyword hits are substrings — `bedroll`, …);
  the sole audio data is `shot.cues`.
- Scene format §15: the clips data model (§15.6) is keyframe **tracks
  only** — `{duration, tracks[], uses[]}`; there is no events field
  anywhere in §15.6–15.8, and `tools/format/check_clips.py` validates
  tracks/channels/keys, not events.
- Binary: `src/*.rs` contains no cue or clip-event handling (the only
  "cue" hit is an unrelated comment in `anim.rs`); `film.rs` never reads
  `shot.cues` either — the field is data the bridge transcribed and
  only this emitter now consumes.
- §15 step-2 (loader consumption of clips) is guard-only per the Phase 0
  contracts, so even when clips are evaluated natively there is
  currently no event *emission* point. **This is an input to the
  bridge/evaluator seats — the requirement is specified in §5 (R5/R6).**

**Routing the emitter implements:** events discovered in scene data
(`shot.events`, §5 R5 shape) or passed via `--events` — and authored
`shot.cues` entries whose *type* is a known event name — are written to
`clip-events.json` beside the output, in build_cues' input shape
`{"duration", "events": [{t, cue, gain?, dur?}]}`, sorted by `t`, and
reported on stdout with the exact build_cues command line. Verified
end-to-end (test T6, §4): the sidecar + the emitted cues.json compile
through the shipped `audio_recipes.py build forest_calm` —
fall-impact→thud, takeoff→whoosh, land→plop, base cue preserved.

## 4. Parity test — Onoda (the fixture named in the contract)

Command (scratch output only; the project's real file untouched):

```
python3 template/tools/cues_native.py \
  ~/faceless-shorts/the-soldier-who-didnt-know/native \
  -o /tmp/cues-parity/cues.json
→ data/cues.json: 9 cues, 60.36 s
→ cues_native: s05a cues[0]: whoosh normalised (at->t name->type, dropped source)
diff / cmp against ~/faceless-shorts/the-soldier-who-didnt-know/data/cues.json
→ BYTE-IDENTICAL (629 bytes, including the absent trailing newline)
```

The 9 authored cues, field by field (`t`/`type`/`gain`/`dur` all exact):

| t | type | gain | dur | scene source |
|---|---|---|---|---|
| 0.5 | chirp | 0.2 | 1.6 | s01 canonical |
| 21.884 | whoosh | 0.5 | 1.6 | s05a `{at, name, source}` → normalised, `source` dropped |
| 31.17 / 32.21 / 33.25 | tock ×3 | 0.32 | — | s07 canonical |
| 49.64 | plop | 0.3 | — | s10 stored `49.63999938964844` → 3 dp rounding = Chrome's `+t.toFixed(3)` on `Wd('flew',9)+0.42` |
| 51.43 | inhale | 0.28 | 0.7 | s11 canonical |
| 53.84 | thud | 0.5 | — | s11 canonical |
| 55.92 | chirp | 0.16 | 1.8 | s12 canonical |

`duration` 60.357 exact. **Event-class differences: none to analyse —
both sides contain zero clip events** (Onoda's staging uses no clips;
§3). The two spelling normalisations above (s05a, s10) are the only
transformations, and both reproduce the Chrome bytes exactly.

Further tests run (all in /tmp scratch):

- **T1** project-root invocation, default output path → byte-identical.
- **T2** repo layout (`~/workspace/talerender`, manifest at
  `scenes/film/film.json`) → byte-identical.
- **T3** unbridged dir → exit 2, bridge named, no file written.
- **T4** synthetic film with `shot.events` + an event-named authored cue
  + `--events` file → cues.json holds only the 1 FX cue; all 3 events in
  `clip-events.json`, sorted, build_cues-shaped.
- **T5** `--strict-events` on T4's data → exit 3 (file still written).
- **T6** `audio_recipes.py build forest_calm` over T4's sidecar with the
  emitted cues.json as `--cues` base → compiles, 5 placed / 1 base,
  bus trim 1.0.
- **T7** unknown authored type (`kazoo`) → exit 2 listing both valid
  vocabularies, nothing written.
- **T8** event with `clock: "clip"` → exit 2 (rebasing is the bridge's
  job; the emitter never guesses an offset).
- **T9** conflicting `t`/`at` in one entry → exit 2.
- **T10** `pan` passthrough and integral `gain: 1` spelled `1`
  (JSON.stringify parity).

## 5. Bridge-time requirements spec (input to the bridge & evaluator seats)

What scene data **must** carry for cues to work on a future project.
The emitter enforces R2/R3/R7 loudly; the rest are data obligations
only the bridge can meet.

- **R1 — `shot.cues` on every segment scene file.** The authored SFX
  cues live at `shot.cues[]` in each `scenes/film/*.scene.json` the
  manifest names (absent = no cues in that segment). The bridge
  already does this for Onoda (via transcription from `shots.ts`);
  a future bridge builder must do it for new projects — the emitter
  has no other authored-cue source and will not invent one.
- **R2 — canonical spelling.** Emit `{t, type, gain?, dur?, pan?}`
  only. The legacy variant `{at, name, gain, dur, source}` (Onoda
  s05a) is accepted and normalised, but new data must not be written
  that way; provenance like `source` belongs in an audit note outside
  the cue entry (it is dropped here, as the Chrome file drops it).
- **R3 — `t` is absolute film time, anchors pre-resolved.** Cues in
  scene data are film-absolute (Onoda: s07 tocks at 31.17 s sit inside
  the segment's film range, not at segment-relative offsets). The
  bridge must resolve word anchors (`Wd(...)`, line starts, shot
  starts) against the project's word timings *at bridge time* and
  write the resulting float; the emitter never reads words.json.
  Full-float precision is fine — the emitter applies the Chrome
  3 dp rounding itself.
- **R4 — manifest `durationS` = lines duration.** It must equal the
  project's `data/lines.json` `duration` (the film duration), never
  the mix.wav duration (Onoda: 60.357, not 60.46) — mix.py sizes its
  canvas from the emitted value.
- **R5 — clip events: a NEW field, `shot.events[]`, absolute time.**
  Shape `[{t, cue, gain?, dur?}]` (build_cues' event shape). The
  bridge must compute, per event:
  `t = segment startS (manifest) + clip instance offset within the
  shot + ClipEvent.t (clip-local)`, applying any parameter that moves
  the event in the source (cliplib's caveat: e.g. startle's `inhale`
  sits at the *default* `hit` time — a caller who moves `hit` moves
  the event). Looping clips carry no events (cliplib law). Do **not**
  put events in `shot.cues` (wrong vocabulary — routed away here,
  silently skipped by mix.py if they ever reached it), and do not
  encode them as §15 tracks — §15 has no event channel.
- **R6 — evaluator seat (§15 step-2):** if native clip evaluation is
  built, its event output must be emitted in the R5 shape at the same
  absolute times the TS evaluation would produce (including
  impact-scaled gains from actions.ts), so this routing — sidecar →
  build_cues — works unchanged. Until then, R5 is bridge-transcription
  only, exactly as `shot.cues` is today.
- **R7 — vocabularies are closed.** Cue `type` ∈ mix.py's 21 FX;
  event `cue` ∈ audio_recipes' `EVENT_VOCABULARY` (11) or an FX name
  (literal placement). Anything else fails this emitter (and
  build_cues) loudly, by design — a new FX or event name is a
  mix.py/recipe change first, data second.
- **R8 — manifest completeness.** Every segment must name its scene
  file and the segments must cover the film; the emitter reads scenes
  in manifest order and fails on a named-but-missing file.

## 6. Limits / honest remainders

- The emitter replaces the **extraction** step only. Word timings still
  originate from the (already Chrome-free) voice/words pipeline; a
  project whose staging was never bridged has no native cue source —
  that gap is the bridge builder's (contract row 5's "unbridged fails
  loudly" is implemented; the builder itself is another seat's).
- `build_cues` remains a separate, recipe-driven step: this tool
  deliberately does not pick a recipe or a music style.
- The Batch 2 adapter work still owns rewiring `capture_native.mjs`'s
  `cues` refusal to call this tool; the refusal text there ("no native
  equivalent") is superseded by this landing.
