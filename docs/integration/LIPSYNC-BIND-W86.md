# LIPSYNC-BIND-W86 — Lipsync binding decision pack: builder-side step keys vs evaluator-side stepping (Wave 2, Batch 9, seat W86)

**Seat:** W86 (Batch 9). **HEAD surveyed:** `c440d90`. **Inputs:**
`docs/integration/LIPSYNC-EVAL-W59.md` (the evaluation this pack
decides on), `tools/lipsync/build_visemes.py`,
`tools/lipsync/evaluate_track.py`, `tools/lipsync/fixtures/
onoda-lipsync.demo.scene.json` (551 keys), the project's
`~/faceless-shorts/the-soldier-who-didnt-know/data/words.json`
(146 tokens, 12 lines), `src/film.rs`, `src/anim.rs`,
`src/scenefile.rs`, `src/toonfull.rs`, `docs/scene-format.md`
§15, `tools/format/check_clips.py`, and the W32 staging
conversion of all 14 production scene files (re-run by this
seat; s12 via the W52 baked staging). **Delivered:** this note
only. Nothing edited, nothing committed, no renders run (this
VM's lavapipe walls make local frames ENV-BLOCKED evidence;
the sign-off renders in §5 are specified for CI/PC).

**The decision, in one paragraph.** W59 proved the signal
choice (the viseme track beats the oscillator wherever a
phoneme is active, under any semantics) and isolated the
remaining choice: the track's *silence behaviour* is decided
by evaluation semantics, not by the key values. Evaluated
linearly (the W15 default the demo keys are authored in), the
track leaks openness across silences almost as badly as the
oscillator (painted-open on 229 of 262 in-line silence frames;
bilabial closures paint closed on only 6 of 36 frames). Two
mechanisms fix it: **Variant A — builder-side:** `build_visemes.py`
emits `ease: "step"` on every key whose predecessor key is a
*closed* key (value ≤ 0.05), so the standard §15 evaluator —
unchanged, channel-blind — holds the closed posture across
each silence and closure and jumps at the next phoneme.
**Variant B — evaluator-side:** the builder's files stay
all-linear and the clips evaluator (or its loader) applies
the same closed-exit rule while sampling the `talk` channel.
This seat measured both against W59's frame grid: **Variant A
reproduces full-step evaluation's painted classes exactly**
(silence-open 106/262, out-of-line leak 0/195, closures
36/36) while keeping every open→open segment linear, and its
data passes `check_clips` 5/5 today. **Recommendation (§4):
Variant A.** One correction to W59 along the way: its §5
item 2(a) as literally worded (step only after *silence*
keys) does **not** fix the closures — see §1.3.

---

## 1. The two variants, specified to implementation depth

### 1.0 Shared ground (all cites at HEAD `c440d90`)

- **The oscillator and its seams.** `talk(t, on)` is
  `src/film.rs:275–282` (law unchanged from W59's
  transcription). Its only two assignments are diegetic Tani
  windows — `src/film.rs:1521` (s03) and `src/film.rs:1995`
  (s11) — and per W59 §4 **neither changes under any
  variant**: Tani has no speech-timing data; the track is the
  narrator's. The adoption site is the staging of a character
  who *is* the track's speaker (W59 §5 item 3's sketch:
  `st.talk = clip value if the node instances a clip on the
  talk channel, else talk(t, on)`).
- **The value's path.** `FullState.talk` is
  `src/toonfull.rs:144` (field; default 0.0 at `:168`); the
  painter opens the mouth iff `t > 0.05` in the
  Determined branch (`src/toonfull.rs:694`) — **0.05 is the
  closed/open threshold both variants key on**; `state_key`
  keys talk by exact f32 bits (`src/film.rs:2261`; the key
  function at `:2246`), so any value change repaints.
- **The evaluator core already exists; the loader wiring is
  the adoption.** `src/anim.rs` (registered `src/main.rs:19`)
  carries `Track::sample` (`:472`), `Easing::Step`
  (from_name `:104`; eval `:124–130` — 0.0 for u < 1, i.e.
  hold the segment's start value), and `Clip::sample_f32`
  (`:594`). But no production loader consumes clips yet:
  `src/scenefile.rs` rejects them, loudly, at three guard
  sites — node `clip`/`clipTime` (`:511–517`), a root
  `clips` block (`:859–861`), and `clip` fields on the
  camera/lights/post/fog blocks (`:862–868`) — all naming
  §15.10 as the landing order. Both variants therefore ride
  the same §15 adoption; they differ only in *where the
  stepping rule lives*.
- **Ease governance.** Per §15.8 (`docs/scene-format.md:
  1087ff`), a segment is eased by the **arriving** key's
  ease (`k1.ease`). `anim.rs`'s `Track::sample` applies the
  *starting* key's ease (`a.ease`, `:472ff`), so the §15
  loader performs the documented remap — file `key[i+1].ease`
  → track `key[i].ease` (§15.9's ruling, `docs/scene-format.md:
  953`). Under that governance, stepping the segment that
  *exits* a closed key means setting `step` on the key that
  *ends* that segment — the first phoneme key after the
  closed key. Both variants implement exactly that segment
  set; they differ in who writes the ease.
- **Composition precedent at the staging seam.** The blink
  adoption already composes a library signal with authored
  data at these very sites: `blink_of` (`src/film.rs:
  1443–1444`) takes `max(charanim::blink_amount, authored
  beats)`, and both Tani sites now call
  `crate::charanim::blink_amount(9, t)` beside their `talk`
  assignments (`:1522`, `:1996`). A clip-bound talk value
  lands the same way: a pure function of t, composed in
  staging, no new state.

### 1.1 Variant A — builder-side step keys (semantics in the data)

**The rule.** In `tools/lipsync/build_visemes.py`,
`_keys_from_segments` (`:650–676`): a key K (never the first)
is emitted with `ease: "step"` iff the immediately preceding
key **in the final, collapsed key list** has value
≤ 0.05 — a *closed* key: a silence key (0.0, emitted at
`:659–660` when the gap to the next segment is ≥ 0.04 s) or
a `pp` closure key (0.02). Every other key keeps
`ease: "linear"`. Segments exiting an open key are untouched:
vowel→vowel and consonant→vowel ramps stay linear.

**The patch (exact).** The function's final loop already
tracks the predecessor value (`last_v`, `:664ff`) for the
equal-value collapse. The change is at the append site
(`:671`):

```python
keys.append({"t": t, "value": v,
             "ease": "step" if (last_v is not None
                                and last_v <= 0.05)
                     else "linear"})
```

plus the same rule for the trailing duration key
(`:674–676`, predecessor = the last kept key) for uniformity,
and a named constant for the threshold with its source
(`CLOSED_EXIT_MAX = 0.05  # the painter's open threshold,
src/toonfull.rs:694`). Three properties make this the whole
patch:

- The predecessor relation must be read **after** the
  collapse, and the collapse cannot break it: a dropped key
  has the same value as its kept predecessor, so if a closed
  key is collapsed away, the key before it is equally closed
  (0.0 follows 0.0; a 0.02 follows only a 0.02) — the next
  key's predecessor-closed test gives the same answer.
- No key times or values change. On this film the entire
  data delta is **77 of 551 keys** changing their ease field
  (`linear` → `step`); regeneration stays deterministic, and
  the regen-and-diff CI check W59 §5 item 4 relies on sees
  an ease-only diff.
- The result is legal data **today**: this seat built the
  variant-A demo file and ran the validator —
  `check_clips.py` **PASS 5/5**, the same verdict as the
  all-linear original. (`talk` is a SCALAR channel in the
  validator's table, `tools/format/check_clips.py:134`; the
  step mandate at `:44–45`/`:444–446` binds only the
  discrete STEP channels `mood`/`legs`, and mixed eases on a
  scalar track are within the W15 vocabulary.)

**What the evaluator must do:** nothing beyond the §15
adoption itself. §15.8's arriving-key governance + the
existing `Easing::Step` express Variant A completely.

### 1.2 Variant B — evaluator-side stepping (semantics in the binding)

**The rule** is the same closed-exit segment set, applied
at sampling time instead of in the file. Two implementation
shapes, both real seams:

- **B1, load-time ease rewrite.** When the §15 loader (the
  future consumer behind `src/scenefile.rs:859–861`'s guard)
  instances a clip track whose target channel is `talk`, it
  rewrites the track's eases during the same pass that
  performs the §15.9 remap: for each key i > 0 with
  `keys[i−1].value ≤ 0.05` (in file order, pre-remap), set
  the remapped ease governing segment (i−1 → i) to `Step`.
  The file on disk stays all-linear; the in-memory track
  becomes byte-identical to Variant A's.
- **B2, sampling flag.** `anim.rs`'s `Track` gains a
  step-after-closed mode consulted inside `sample`
  (`:472ff`): if the segment's start value ≤ 0.05, return
  the start value. The flag is set from the binding (the
  node's clip instance names the semantics — e.g. a
  `sampling` field the format does not yet have, which makes
  B2 a format change as well as a code change).

**What B costs, concretely:**

- **Channel knowledge in generic code.** The §15 evaluator
  is channel-blind by design (its only channel semantics
  are the validator's discrete-channel step law). B puts a
  channel name (`talk`) and a magic threshold (0.05) into
  the loader or the sampler. Every future channel with
  closed-exit behaviour needs the same special case, or
  silently doesn't get it.
- **Validation stops describing rendering.** `check_clips`
  validates the all-linear file and reports PASS 5/5 — for
  a signal the renderer will not produce. The two evidence
  incidents this wave already corrected (a record claiming
  a state the artefacts didn't hold) are the same failure
  shape: a checked artefact that under-describes behaviour.
- **Every out-of-renderer evaluator must re-implement the
  rule** or evaluate a different signal: `rerender_plan.py`,
  the locomotion/blink censuses, `evaluate_track.py`
  itself, and the exporter acceptance harness (W77) all
  read these files. Under A they all stay correct for free,
  forever, because the file *is* the semantics.
- B's one genuine advantage: existing W23 artefacts stay
  byte-untouched. Weighed in §4.

### 1.3 Correction to W59 §5 item 2(a) — the rule must be closed-exit, not silence-exit

W59 offered (a) as "the builder emits `step` on every key
whose predecessor key is a silence key (value 0 after a word
end)" and listed closures 6 → 36 among its expected effects.
Measured here, that literal rule cannot deliver the closure
effect: the segment that leaks a `pp` closure open is the
segment **starting** at the closure key, governed (arriving-
key rule) by the **next** key's ease — and that key's
predecessor is the closure (0.02), not a silence (0.0). The
closed-exit rule of §1.1 (predecessor ≤ 0.05) covers both
leaks with one threshold, and §2's measurements show it
reproduces full-step's painted classes exactly. This pack's
Variant A is therefore W59's option (a) with its rule
corrected, not a third invention; W59's option (b) is
Variant B above.

### 1.4 Sub-variant A2 — silence keys at every word end (priced, not recommended)

Dropping the builder's 0.04 s gap threshold (`:659`) so a
silence key is emitted at *every* word end, then applying
the §1.1 rule, additionally closes the 106 residual frames
(W59 §3.3: sub-0.04 s gaps hold the previous posture under
step semantics). Measured price on this film: keys
551 → **648 (+97)**; the newly closed frames form **85
micro-closure runs totalling 119 frames — 51 runs of exactly
1 frame, 34 of 2 frames** — i.e. the mouth visibly slams
shut for 33–66 ms inside continuous fast speech, where no
silence exists in the audio. A2 is a one-line builder flag
(`--silence-every-word-end`); §5's sheet lets the user see
the bridged gaps A leaves and order A2 only if they read as
stuck rather than as co-articulation.

---

## 2. Repaint sets for a hypothetical bound shot (measured)

**Method.** This seat re-ran W59's frame loop (30 fps,
frames 0…1810; regime A: oscillator `on` = inside a
narration line span) with four track evaluations: the
original keys linear, the original keys full-step, Variant
A's keys under standard per-key evaluation, and A2's keys.
**Validation: every W59-published figure is reproduced
exactly** — linear band-differing 1,196; silence-open
229/262; out-of-line leak 119/195; closures 6/36; line-3
band-differing 120; step what-if 106 / 0 / 36-of-36. The
rows W59 did not publish (A, A2, the f32-exact counts) are
this seat's, computed by the validated loop.

| measure (regime A, in-line frames = 1,616) | osc (control) | track-linear | track-full-step | **Variant A** | Variant A2 |
|---|---|---|---|---|---|
| band-differing vs oscillator (\|Δ\| > 0.10) | — | 1,196 | 1,203 | **1,221** | 1,278 |
| repaint set vs oscillator (f32-exact ≠) | — | 1,616 | 1,616 | **1,616** | 1,616 |
| in-line silence frames painted open (of 262) | 262 | 229 | 106 | **106** | 0 |
| out-of-line frames painted open (of 195) | 0 (never on) | 119 | 0 | **0** | 0 |
| `pp` frames painted closed (of 36) | 0 | 6 | 36 | **36** | 36 |
| line 3 (frames 482–632): band-differing / 151 | — | 120 | 107 | **120** | 122 |
| line 3: repaint set / 151 | — | 151 | 151 | **151** | 151 |

**How to read this table — three findings.**

1. **The repaint set does not discriminate the variants.**
   At the `state_key` standard (exact f32 bits,
   `src/film.rs:2261`), a bound shot repaints **all 1,616
   in-line frames under every semantics**, because the
   oscillator's value differs from any track value on
   effectively every frame (its phase is the clock, W59
   §3.2). The A/B gate's candidate set is therefore the
   whole bound span regardless of the decision — no variant
   is cheaper to *gate*. Anyone quoting a smaller repaint
   set for one variant is measuring the band, not the
   repaint.
2. **The band counts measure difference from the
   oscillator, not quality** — Variant A's 1,221 > linear's
   1,196 because A is *more* different exactly where the
   oscillator is most wrong (silences: correctly shut
   against a flapping control). The quality rows are the
   painted classes: **A matches full-step on all three**
   (106 / 0 / 36-of-36) while stepping only 77 segments;
   the remaining open→open motion — 1,720 of the talk
   track's 1,768 grid frames differ between linear and
   full-step evaluation — stays linear under A.
3. **Where A and full-step diverge is a feature with a
   name:** the 106 residual silence frames are sub-0.04 s
   inter-word gaps with no silence key; A holds the prior
   posture across them exactly as full-step does (both read
   106). A2 alone closes them, at the §1.4 price.

**Paint economy (the second-order repaint effect).** What
the semantics change *does* buy is the track's own value
economy across the film (W59 §6's argument, now attached to
a variant): under A, talk is constant across every silence
and closure exit — at most the 551 authored values, with
whole spans sharing one character card wherever the rest of
the state recurs — versus linear's per-frame-distinct
values through every gap. Strictly better than the
oscillator's profile, never worse; identical for B (same
sampled values), so this row does not discriminate A from
B either. The A-vs-B decision rests on §1.2/§4, not on
performance.

---

## 3. Blast radius: a global step rule is not a legal form of this decision

The temptation to avoid naming semantics per track is to
make the *evaluator* step everything. This seat censused
what that would do, by converting all 14 production scene
files with `tools/format/stage_clips.py` (s12 through its
W52 baked staging, the only form that converts) and
sampling every resulting track twice on the 30 fps grid —
authored eases vs forced step:

- **Production staging: 145 tracks — 95 CHANGED, 42
  constant (immune), 6 discrete (`mood`/`legs`, already
  step by validator law), 2 already-step.** The changed
  tracks differ on **7,513 track-frames** in sum.
- **Every shot's camera is in the blast radius.** All 14
  `eye` tracks change, most on essentially every frame of
  their span (s01 eye 177/178 frames, s02 eye 214/215, s08
  eye 184/185, s04 eye 163/164): a global step turns every
  camera move into a staircase of teleports. Changed
  production channels: `translate` 19, `rotate.z` 18,
  `rotate.x` 17, `eye` 14, `position` 8, `armR.0` 6,
  `target` 5, `headTurn` 2, `armR.1` 2, `lean` 2,
  `armL.0` 1, `headTilt` 1.
- Including the fixtures and the lipsync files, the census
  covers **173 tracks, 116 changed** (lights, grade, fog,
  bokeh among them in the format-15 composite).

**Conclusion the lead can rely on:** step semantics must
be scoped per track (Variant A: by construction — only the
lipsync builder emits the pattern, no other artefact
changes) or per channel (Variant B: by a channel check in
the loader that must never widen). A global evaluator rule
is not a simplification of this decision; it is a
re-staging of the entire film. The §15 spec already encodes
the correct scoping instinct — its step mandate binds the
discrete channels only, and §15.8 leaves every other
track's ease to its keys.

---

## 4. RECOMMENDATION (marked as such): Variant A — builder-side step keys

**Adopt Variant A (§1.1): the closed-exit step rule in
`build_visemes.py`, threshold 0.05 as a named constant.**
Reasons, in the order the format's own promises rank:

1. **Determinism and auditability.** A clip is a pure
   function of t (§15.8) whose whole behaviour is in the
   file. Under A, the artefact `check_clips` validates is
   the artefact that renders; the adoption diff on this
   film is 77 ease fields, reviewable in one screen, and
   W59's regen-and-diff determinism check keeps its
   meaning. Under B, the rendered signal exists nowhere as
   data.
2. **W15's linear default survives intact.** A needs *zero*
   evaluator features beyond the §15 adoption itself —
   arriving-key governance and `Easing::Step` already
   express it. B adds a channel-aware semantics path
   (loader rewrite or sampler flag) that must be maintained
   beside the generic one forever, and re-implemented by
   every tool that evaluates a scene file outside the
   renderer (§1.2).
3. **Exporter totality.** The exporter's law (W47) is that
   every consumed field is emitted explicitly, because
   loader defaults are not production values. B violates
   the same principle one level up: a consumed *behaviour*
   emitted nowhere. Under A, totality holds — `ease` is a
   consumed field and it is in the file, per key.
4. **The cost asymmetry is one-directional.** A's price is
   a ~3-line builder patch and a regeneration of builder
   artefacts (deterministic, cheap, and the demo files are
   the builder's own outputs). B's price is permanent:
   semantics split between data and code for the life of
   the format.
5. **B's advantage doesn't survive contact.** "W23's files
   stay untouched" protects artefacts whose entire purpose
   is to be regenerated by the tool being adopted; W59
   already established regeneration is byte-deterministic.

**Rejected:** A2 as the default (§1.4 — 51 single-frame
mouth-slams is a visible artefact traded for closing gaps
shorter than a frame-pair; keep as a builder flag).
Full-step evaluation of the talk track (steps open→open
segments too — 1,720 differing grid frames against linear
for zero painted-class gain over A). Variant B (above).

**Lead's residual decisions** (small, named): the
threshold constant's name/placement in the builder; the
trailing duration key following the same rule (this pack
applies it — uniformity, no measured effect); whether
`check_clips` grows a *lint* (not a gate) noting scalar
tracks whose closed exits are unstepped — useful for
hand-authored talk tracks, out of scope here.

---

## 5. User sign-off sheet — line 3, three bindings (decision artefact spec)

Adoption changes zero frames of the current film (W59 §4),
so this sheet is the decision artefact for the *semantics*
choice, executed against the **rehearsal binding** (W59 §7:
the narrator track bound to an on-screen character in a
scratch staging of line 3's shot; the binding mechanism is
the lead's, the artefact is identical either way). **Render
route: CI dispatch or the user's PC only** — this VM's
software-Vulkan frames are ENV-BLOCKED evidence, not
judgement material.

**Frames.** Line 3, **frames 482–632** (151 frames,
t ∈ [16.04, 21.10)) — the densest-difference line under
regime A (W59 §5 of its results). Three frame sets over
the identical window, same binary, same staging, differing
only in the talk binding:

1. `frames-osc` — oscillator binding (the control: today's
   only driver).
2. `frames-lin` — the W23 track, all-linear keys (the
   default-adoption outcome if semantics go unnamed).
3. `frames-A` — Variant A's track (77 step eases) under the
   standard evaluator.

**Layout.** One contact sheet per binding via
`tools/film/make_sheet.py` over `--from 16.04 --to 21.10`
(W44's tool, its own defaults otherwise), identical tiling
across the three sheets, native resolution — the mouth is
a small region of a 1080×1920 frame and no crop tooling
exists (W59 §7), so the reviewer zooms the sheet; do not
upscale. Sheets are compared side by side; frame numbers
printed on the sheet are the judgement addresses below.

**What the user judges** (in order; the first is the
headline):

1. **The 17-frame silence after "ended." — frames
   570–586** ([18.980, 19.540), the line's one long
   inter-word gap). `frames-lin` shows the mouth drifting
   open across the pause (the leak); **`frames-A` must
   show it shut for all 17 frames.** This single strip is
   the decision: if A reads shut and natural here, the
   semantics are right.
2. **The bilabial closure in "message." — frames
   621–623** (M, [20.700, 20.767)). A: shut on all three
   frames. Linear: open on the later frames (the closure
   exists only at the key instant).
3. **The two 1-frame bridges — frame 518** (after
   "1945,") **and frame 547** (after "the"). These gaps
   are exactly 0.04 s with an open phoneme on both sides;
   A deliberately bridges them (no silence key exists, by
   the builder's threshold). Judge: co-articulation or
   stuck mouth? If the user reads them as stuck, that is
   the A2 order (§1.4) — decided from these two frames,
   not from the counts.
4. **Vowel tracking through the line's speech** (any
   frames outside items 1–3): A and linear should be
   indistinguishable except at closed exits. A visible
   jump *mid-word* on the A sheet is a defect in the
   rule's threshold — report the frame; it reopens §4.
5. **Explicitly not judged:** the B closure in "But"
   ([19.540, 19.557)) — 17 ms that falls between frame
   instants 586/587; no binding can paint it at 30 fps.
   It is listed so nobody grades a phantom.

**Instruments around the sheet** (W59 §7, carried over):
`tools/ci/byte_probes.py` over `--ranges 482-632` for each
candidate against the oscillator set — PSNR is the
tripwire, not the measure (any differing frame *outside*
the bound line, or a band-agreeing frame scoring < 60 dB,
is a defect); `FILM TELEMETRY: CHECK OK` must be unchanged
by the binding; the determinism probes stay green (§2's
values are pure functions of t under every variant).

**Sign-off semantics.** The user approves *Variant A's
semantics* (or orders A2 / linear by the items above);
motion quality is the user's call, per the standing A/B
precedent (W24/W57/W58). Approval here binds future
on-screen speakers; it re-renders nothing in the current
film, whose two oscillator windows (Tani, `src/film.rs:
1521`/`:1995`) remain oscillator-driven under every
outcome — they are diegetic beats with no speech data to
bind, and replacing them with the narrator's phonemes
would be a bug, not an adoption (W59 §4).

---

## 6. Provenance and limits

- Every number in §2 was produced by this seat's frame
  loop over the real data, after reproducing W59's
  published figures exactly (listed in §2's method note);
  the census in §3 is this seat's conversion + sampling of
  the 14 production scene files at HEAD `c440d90`.
- Variant A's validator verdict (PASS 5/5, 77 step eases)
  was run by this seat against a variant-A file built from
  the real words.json at this HEAD.
- No frames were rendered by this seat; painted-class
  claims route through the painter's own threshold
  (`src/toonfull.rs:694`), as in W59. Pixel judgement
  belongs to §5's sheet, on CI/PC renders.
- Diegetic speech (Tani) remains unmeasurable from data —
  no timings exist for it anywhere; nothing in this pack
  grades or changes it.
