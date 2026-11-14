# CUES-W45 — Native cue emission design (gap-fix F6)

**Status: DESIGN ONLY.** No code in this note, no schema field presented
as fact without a citation. Seat W45, Wave 2 Batch 5. Everything below
is surveyed from this repo at the time of writing; the skill
(`tall-tale-shorts`) and its `tools/mix.py` live **outside** this repo,
and where their behaviour is not recorded here it is marked NOT
VISIBLE, never guessed.

## 0. The gap, stated exactly

The skill pipeline (docs/audit/a6-pipeline.md, stages 4–5) is:

- Stage 4 (Cues): `node tools/capture.mjs cues` evaluates `OP.cues()`
  in the Chrome page and writes the project's `data/cues.json` —
  SFX cues *declared in `src/shots.ts`*, extracted from the page.
- Stage 5 (Mix): `tools/mix.py` consumes `data/narration.wav`,
  `data/lines.json`, **`data/cues.json`**, and `script.json`, and
  produces `data/mix.wav` (48 kHz master; −14 LUFS / ≤ −1 dBTP).

The native path has no stage 4. Verified at HEAD: `src/*.rs` contains
no cue handling at all (the only "cue" hit in `src/` is an unrelated
comment in `src/anim.rs`), the driver never reads the scene files'
`shot.cues` field, and nothing in the native path emits a
`cues.json`. The skill-side adapter in the wiring work copy refuses
the `cues` mode by name (docs/integration/CLOSEOUT-DRAFT-W98.md), so
today a native-rendered project still depends on a Chrome run for
its audio cues.

**What produced the audio for the delivered native film — the record:**
nothing native. `film/finish.sh` muxed the project's **existing**
`data/mix.wav` unchanged (docs/ab/film/MP4-EVIDENCE.md: "audio AAC
`-b:a 192k` from the project's `data/mix.wav` **muxed unchanged** (no
gain stage, no re-mix)"; the manifest `scenes/film/film.json` audio
block records `"treatment": "muxed unchanged (project master)"`,
60.46 s, −14.0 LUFS). That master was produced earlier by the
project's own skill pipeline — i.e. its cues came from the Chrome
stage 4 and its mix from stage 5, before the native path existed.
Native cue emission is therefore a *replacement for stage 4's
output*, not a change to stage 5.

## 1. Cue event inventory derivable from native film data

### 1.1 Authored SFX cues — the primary source (already data)

The scene files already carry the film's authored cues as data:
`shot.cues` in `scenes/film/*.scene.json`, transcribed from the
`shots.ts` `cue(...)` calls and audit-verified value-for-value
against them (docs/integration/AUDIT-W40.md: `shot.cues[thud].t/.gain`
53.84 / 0.5 MATCH; AUDIT-W42.md: `shot.cues[0].at/.gain` 21.884 / 0.5
MATCH, `shot.cues[0].t` 49.64 MATCH).

Census at HEAD — **9 cues over 14 segments**:

| segment | cues (as recorded) |
|---|---|
| s01 | chirp, t 0.5, gain 0.2, dur 1.6 |
| s05a | whoosh, at 21.884, gain 0.5, dur 1.6 |
| s07 | tock ×3, t 31.17 / 32.21 / 33.25, gain 0.32 |
| s10 | plop, t 49.63999938964844, gain 0.3 |
| s11 | inhale t 51.43 gain 0.28 dur 0.7; thud t 53.84 gain 0.5 |
| s12 | chirp, t 55.92, gain 0.16, dur 1.8 |
| s02, s03, s04, s05b, s06, s08, s08b, s09 | none (ASSUMPTIONS.md confirms per-shot: "No cues in this shot") |

- **Source field:** `shot.cues[]` in each segment's scene file;
  segment membership and absolute placement from the manifest
  (`scenes/film/film.json` `segments[]`).
- **Timing:** the recorded times are **absolute film times** in
  seconds (the s07 tocks at 31.17 s sit inside s07's segment range,
  not at segment-relative offsets — ASSUMPTIONS.md:863–864). They are
  **not frame-quantised**: several are derived from word anchors plus
  an authored offset (s10's plop = `Wd('flew',9) + 0.42` = 49.64,
  stored with its f32 spelling 49.63999938964844, AUDIT-W42). Data
  precision is float-seconds; the *effective* placement precision is
  whatever `mix.py` does with the value — NOT VISIBLE from here.
- **Two spellings exist in the data — a defect the design must fix,
  not inherit.** Eight entries use `{t, type, gain, dur?}`; s05a
  alone uses `{at, name, gain, dur, source}` (its `source` field is a
  provenance string quoting the `shots.ts` call). Both spellings were
  audit-verified against the same `cue()` call shape, so this is a
  transcription inconsistency, not two cue kinds.

### 1.2 Other timeline events present in the data (not cue sources today)

Listed so the boundary is explicit; none of these appear in any
repo evidence as `cues.json` content, and the design does **not**
emit them as cues:

| event | source field | precision | disposition |
|---|---|---|---|
| Line starts/ends | `film/onoda.json` `lines[]` `{i, start, end}` (12 lines) | word-timing grade (Whisper-derived upstream) | already a direct `mix.py` input via `data/lines.json` (a6 stage 5) — no cue role |
| Word anchors | `film/onoda.json` `anchors{}` (17 named word times, e.g. `flew9: 49.22`) | float seconds | derivation base for §1.1 times; provenance, not events |
| Shot cuts | manifest `segments[]` `startS` / `startFrame` | **frame-exact** (frameRule: a segment owns frames `startS <= n/fps < endS`) | derivable if a future consumer wants cut markers — no evidence `mix.py` does |
| Caption chunk boundaries | `film/onoda.json` `captions[]` `{s, e}` (73 chunks) | float seconds | visual track; no audio role evidenced |
| Frame fades | `shot.fades {in: 0.25, out: 0.4}` on s01/s12 only | seconds | the driver applies these as *visual* black fades; whether the skill ever pairs them with an audio fade is NOT VISIBLE — open question Q6, not an assumption |

**Inventory conclusion:** the native cue file's content is §1.1 and
only §1.1 — 9 events for this film, each already carrying time,
kind, gain, and (sometimes) duration in repo data.

## 2. Emission point, location, versioning

### 2.1 Where in the driver

**Proposed:** a dedicated emission step in the film driver,
executed **after full timeline resolution and before the render
loop**, plus a CPU-only mode alongside `--film-telemetry` that
emits (or checks) cues without rendering.

Justification:

- Cues are a pure function of resolved data (`onoda.json` +
  manifest + the 14 scene files). They depend on no rendered pixel,
  so they can — and should — be computed at the same point the
  driver has finished loading and validating all of it. Emitting
  earlier is impossible (the cues live in the scene files, the last
  inputs parsed); emitting later buys nothing.
- Pre-render emission means a render failure cannot strand the
  audio pipeline: stage 5 can proceed in parallel with the frame
  render, exactly as the Chrome pipeline's stage 4 is independent
  of its `video` mode (a2-capture.md §3.7 vs §3.8).
- The CPU-only leg mirrors the existing telemetry split (verdict
  work is already CPU-only and separate from rendering) and makes
  the acceptance test in §5 cheap: no 1,811-frame render to compare
  9 cues.
- Because the driver currently **ignores** `shot.cues`, emission
  requires the loader to start consuming the field — which also
  means the §1.1 spelling split must be reconciled at load time
  (accept both, normalise to one, or reject the minority spelling
  loudly). Silently accepting both forever would bake the defect
  in; the design recommends: normalise on load, warn once per
  non-canonical entry, and let the film-data validator
  (`tools/film/validate_film.py`) flag the spelling in its checks
  so the data itself gets fixed upstream. (Validator change is a
  separate seat's call; recorded here as a recommendation only.)

### 2.2 File location

The contract location is the **project's** `data/cues.json`
(a6-pipeline.md stage 4 output / stage 5 input). Two candidate
targets, one decision needed:

- **(a) Direct:** `<project>/data/cues.json`, with `<project>`
  taken from the film data's recorded project path
  (`film/onoda.json` `project`; manifest `project`). This is the
  only location `mix.py` is evidenced to read.
- **(b) Staged:** emit beside the film data and let the skill-side
  bridge place it. Safer (no writes outside the directory the
  operator pointed the binary at), but adds a hand-off where a
  stale Chrome-produced `cues.json` can silently survive — the
  exact stale-timing failure the scaffold stage deletes timing
  files to prevent (a6-pipeline.md stage 0).

**Recommendation: (a), but never as a silent side effect of a
render.** Emission happens when the operator runs the cue mode
(or passes an explicit emit flag), prints the a2-evidenced console
line shape (`data/cues.json: <n> cues, <dur> s`, §3), and refuses
to overwrite an existing `cues.json` whose content differs unless
told to — a render must not quietly rewrite the audio contract.

### 2.3 Versioning

Follow the repo's own precedent: the film manifest carries
`{"format": "talerender-film", "version": 1}`. For `cues.json`
the compatibility constraint dominates (§4): the envelope is the
Chrome one (§3), so **no version field goes inside the contract
file** unless the skill side confirms extra top-level keys are
tolerated (Q4). Provenance (generator, source-data identity) is
proposed for the console line and, if needed, a sidecar
(`data/cues.native.json`) — PROPOSED, not part of the contract.

## 3. Schema proposal — cited vs PROPOSED, field by field

### What the repo actually evidences about the Chrome file

From docs/audit/a2-capture.md (the M0 audit of the skill's own
capture tool, i.e. repo-side evidence about the skill-side file):

- §3.7: `OP.cues()` → writes `data/cues.json` as
  `JSON.stringify({duration, cues}, null, 1)` — **envelope
  `{duration, cues}`, 1-space indent** — and prints
  `data/cues.json: <n> cues, <dur> s`. "This file is the contract
  with `tools/mix.py` downstream."
- Line 97 (OP API table): `OP.cues()` returns "**sound cues sorted
  by `t`**" — the array is sorted, and entries carry a `t`.
- Checklist (line 517) repeats: `data/cues.json` = `{duration,
  cues}` with 1-space JSON indent.

**What is NOT visible from this repo — stated plainly:** the
per-entry field set of the array `OP.cues()` returns. No repo
document enumerates it. The scene-data entries in §1.1 are a
transcription of the **`shots.ts` `cue()` call arguments** (that
is what AUDIT-W40/W42 verified MATCH against), which is evidence
about the *authoring* shape — it is **not** proof that `OP.cues()`
emits the same field names into `cues.json`. Anyone claiming
`{t, type, gain, dur}` *is* the file format is going beyond this
repo's evidence.

### Proposed emitted schema

| field | status | basis |
|---|---|---|
| `duration` (top level) | CITED | a2 §3.7 envelope; = film duration (60.357 here) |
| `cues` (top level array) | CITED | a2 §3.7 envelope |
| array order: sorted by `t` ascending | CITED | a2 line 97 |
| serialisation: JSON, 1-space indent | CITED | a2 §3.7 + line 517 |
| entry `t` (seconds, float, unquantised) | CITED as a field name | a2 line 97 ("sorted by `t`"); values from §1.1 |
| entry kind field | **PROPOSED: `type`** | scene data majority spelling (8 of 9 entries); s05a's `name` is the outlier. Awaiting Q1 |
| entry `gain` | **PROPOSED** | present on all 9 scene-data entries, audit-matched to `cue()` opts; plausible `OP.cues()` carries it, unproven |
| entry `dur` | **PROPOSED, omit-when-absent** | present on 4 of 9 entries (chirp ×2, whoosh, inhale), absent on tock/plop/thud — mirroring the authoring optionality; whether `OP.cues()` omits or defaults it is Q2 |
| entry `source` (s05a provenance string) | **PROPOSED: drop** | authoring provenance for auditors, present on 1 of 9; no evidence `mix.py` reads it; dropping loses nothing audible. Awaiting Q1 |
| any top-level key beyond `duration`/`cues` | **PROPOSED: none** | see §2.3 / Q4 |

### Open questions for the skill side (questions, not assumptions)

- **Q1.** What are the exact entry fields (names and order) of the
  array `OP.cues()` returns? Specifically: is the kind field
  `type`, `name`, or something else, and does the emitted entry
  keep the `cue()` opts shape?
- **Q2.** When a `cue()` call omits `dur` (or `gain`), what does
  the emitted entry contain — absent key, or a filled default?
  What default does `mix.py` apply if the key is absent?
- **Q3.** How does `mix.py` treat an unknown cue kind, a cue past
  `duration`, or duplicate times — error, warn, or silent skip?
- **Q4.** Does `mix.py` tolerate extra top-level keys (or extra
  entry fields) in `cues.json`, or does it read strictly?
- **Q5.** Is sorted-by-`t` a requirement `mix.py` relies on, or
  merely `OP.cues()`'s observed behaviour?
- **Q6.** Are `shot.fades` ever paired with audio fades in
  `cues.json` on the Chrome path, or are fades purely visual?
- **Q7.** Is `duration` in the envelope the film duration, the
  narration duration, or the mix duration (they differ here:
  60.357 film vs 60.46 mix.wav)?

## 4. mix.py compatibility analysis

**Can be promised from repo evidence:**

- **Location and envelope discipline.** Same path
  (`<project>/data/cues.json`), same two top-level keys, same
  1-space-indent JSON, same sorted order (§3 CITED rows).
- **Value provenance.** The native values are not re-derived or
  re-estimated: they are the same numbers the Chrome path would
  extract, because both come from the same `shots.ts` declarations
  — the native side via scene data that was audit-verified
  value-for-value against those declarations (AUDIT-W40/W42 MATCH
  lines, §1.1). Anchor-derived times keep their full float
  spelling rather than being rounded to frames.
- **Independence of the other mix inputs.** `narration.wav`,
  `lines.json`, and `script.json` are untouched by this design;
  native emission replaces exactly one of stage 5's four inputs'
  producers.

**Cannot be verified from here (and must not be claimed):**

- `mix.py`'s parser is not in this repo; nothing here shows which
  fields it reads, its defaults, or its error behaviour (Q2–Q4).
  "Same envelope" is therefore a *necessary-looking* condition,
  not a proven sufficient one.
- Whether the SFX synthesis behind a cue kind (e.g. what `tock`
  sounds like) is deterministic — irrelevant to the file contract
  but relevant to the §5 end-to-end leg's tolerance.
- That the Onoda film's *existing* `data/cues.json` in the project
  directory matches the scene data. The project lives outside this
  repo's committed tree; the comparison in §5 is precisely how
  this gets established rather than assumed.

## 5. Acceptance test design

**Object.** Prove the native emitter is a drop-in stage-4
replacement on a real project, field by field, with tolerances
stated per field — semantic equality, not byte equality (field
order and the Q1/Q2 answers may legitimately differ until the
skill side ratifies them; byte equality becomes the gate only if
Q1/Q2/Q4 all come back "identical and strict").

**Where it runs.** Two environments, honestly split:

- The **native leg** runs anywhere the binary runs, CPU-only
  (§2.1): emit to a staging directory, never over the project's
  real file.
- The **Chrome leg** (`capture.mjs cues` on the same project) and
  the **mix leg** run in the skill environment (user's PC /
  skill-wiring harness) — Chrome capture does not run from this
  repo's CI, and pretending otherwise would fake the comparison.
  The Onoda project is the test subject: it is the one project
  with both a Chrome-produced cue history and complete native
  scene data.

**Comparator** (future tool, `tools/film/` class; design only
here): reads both `cues.json` files and reports per-cue pairing
and per-field verdicts.

| check | tolerance / rule |
|---|---|
| cue count | exact equality (9 for Onoda) |
| pairing | by kind, nearest `t`; unpaired cue on either side = FAIL |
| `t` | \|Δ\| ≤ 1 frame (1/30 s). Both sides derive from the same word timings and authored offsets, so agreement should be near-exact; the frame bound absorbs float-spelling differences (cf. 49.63999938964844) without hiding a real misplacement |
| `gain` | exact (same authored literal both sides) |
| `dur` | exact when present on both; presence/absence mismatch is reported separately and fails until Q2 is answered |
| kind field | exact string after the Q1 spelling is ratified; before that, the comparator reports the spelling mapping it had to apply |
| `duration` | \|Δ\| ≤ 1 frame, pending Q7 |
| ordering | native file sorted by `t` (a2 line 97) — checked on the native file alone |

**Negative control.** Remove one cue from a *copy* of the scene
data (e.g. one s07 tock) and re-emit: the comparator must FAIL
with the missing pair named. A comparator that cannot fail this
is not evidence.

**End-to-end leg (skill environment).** Run stage 5 twice — once
with the Chrome `cues.json`, once with the native one — and
compare the two `mix.wav` outputs on duration (exact sample count
expected) and integrated loudness (within 0.1 LU, the finish
record's own comparison discipline in MP4-EVIDENCE.md), plus a
listen check by the user. Waveform/sample equality is **not**
required: SFX synthesis determinism is unverifiable from here
(§4), and requiring it would gate the design on an unknown.

**Regression anchor.** Once passed on Onoda, the comparator's
expected-cue table (9 cues, §1.1 values) becomes a fixture: any
future film-data change that alters an emitted cue must show up
as a deliberate fixture update, the same discipline the film
validator already applies to chunk counts.

## 6. Summary

The native side already holds everything a cue file needs — 9
authored cues as audit-verified data (§1.1) — and the Chrome
file's envelope discipline is fully evidenced (§3). The design is
therefore small: consume `shot.cues` at load, normalise its two
spellings, emit `{duration, cues}` to the project's `data/` from
a pre-render, CPU-runnable step, and prove equivalence against
the Chrome file on Onoda with the §5 comparator. The one thing
standing between this design and an implementation is not native
work at all — it is the seven skill-side answers in §3, Q1 above
all: the exact entry shape of `OP.cues()`'s output, which this
repo's evidence does not contain and this note does not invent.
