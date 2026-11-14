# LIPSYNC-EVAL-W59 — Lipsync adoption evaluation pack (Wave 2, Batch 6, seat W59)

**Delivered:** `tools/lipsync/evaluate_track.py` (new tool, stdlib
only) + this note. Nothing in `src/` or any existing file was
touched; nothing committed. Inputs were read-only: the W23 demo
track in this repo (`tools/lipsync/fixtures/onoda-lipsync.demo.scene.json`),
the film data (`film/onoda.json`), and the project's
`~/faceless-shorts/the-soldier-who-didnt-know/data/words.json`
(146 tokens, 12 lines — the same file W23 built the track from).

**Headline:** bound to the narration, the oscillator and the W23
track paint alike on only **26.0 %** of the 1,616 in-line frames
(band-agreement, §1); **74.0 % differ visibly**, mean |Δ| 0.258.
The oscillator is painted-open on **100 % of the 262 in-line
silence frames** — by construction, not by bad luck: its floor
while `on` is 0.25, five times the painter's open threshold, so
it *cannot* close, pause, or shut for a bilabial. And the honest
complication, measured in §4: in **this** film the oscillator
never drives the narration at all — its only two call sites are
diegetic speech by a different character — and the track itself,
evaluated with W15-linear semantics, leaks openness across
silences almost as often (229 of 262). Adoption is therefore
two decisions, not one: *which signal* (the track wins wherever
a phoneme is active, under any semantics) and *which evaluation
semantics* (a `step` binding fixes the track's silence behaviour;
§3.3). Both are the lead's/user's to take with these numbers in
hand.

---

## 1. Method

**The oscillator law**, transcribed verbatim from
`src/film.rs:232–238` at HEAD `aa92cee`:

```rust
pub(crate) fn talk(t: f64, on: bool) -> f64 {
    if on {
        0.25 + 0.75 * (13.0 * t).sin().abs() * (0.6 + 0.4 * (5.3 * t).sin())
    } else {
        0.0
    }
}
```

Three properties of the law itself drive every result below:

- **Floor 0.25 while on.** `|sin| ≥ 0` and `(0.6 + 0.4·sin) ≥ 0.2 > 0`,
  so `talk ≥ 0.25` whenever `on`. The painter draws the open mouth
  iff `talk > 0.05` (`src/toonfull.rs:698–700`: width `14 + 10·t`,
  height `6 + 18·t`; both call sites paint a Determined mood, so
  this branch — not Shock/Laugh — governs). While on, the mouth
  is therefore *always* visibly open and *never* closes: no
  silence, no `pp` closure (track value 0.02) is reachable.
- **Absolute-time phase.** `t` is film time; the 13 rad/s carrier
  and 5.3 rad/s envelope never reset per line, word, or shot.
  Nothing in the signal knows a word exists.
- **No state.** The law is a pure function of `(t, on)`; the only
  unknowns are the `on` windows, which are staging code — hence
  the two regimes below.

**The track** is evaluated with the W15 §4 semantics
(`FORMAT-EXT-CLIPS-W15.md`): hold the end value outside the key
range; inside, lerp across the bracketing segment eased by the
ending key's ease. All 551 keys are authored `ease: "linear"`
(the tool asserts and prints the ease set), so plain lerp is
the exact semantics, not an approximation of them. "Phoneme
active" and the viseme at a frame come from the track file's own
`lipsync.segments` table (564 phoneme segments; silences are
keyed into the track but are not segments — a frame is inactive
iff no segment covers it).

**The two regimes** (the comparison is meaningless without
saying which `on` is meant):

- **Regime A — narration binding (the adoption counterfactual).**
  `on` := the frame lies inside a narration line's word span
  (first-word `s` to last-word `e`, from words.json). This is
  the question adoption poses: *if a mouth followed the
  narration, oscillator or track?*
- **Regime B — the film as staged.** The driver's only two
  `.talk` assignments in the tree (grep-verified, `src/film.rs`):
  `:1478` in s03 — `talk(t, lt < w_surr − st0 + 0.35)`, st0 =
  13.061 (shot table `:1993ff`, s03 starts at
  `lines[2].start − 0.06`), `w_surr = anchors["surrender2"] =
  15.02` → on for t ∈ [13.061, 15.37), character SH_TANI,
  pose_explain; and `:1947` in s11 — `talk(t, t > w_order − 0.1
  && t < w_order + 0.75)`, `w_order = anchors["order10"] =
  51.78` → on for t ∈ (51.68, 52.53), character TANI giving the
  order. Everywhere else talk is never assigned and defaults to
  0 (`toonfull.rs:168`). The tool *derives* both windows from
  the film data + the quoted expressions (with asserts), it does
  not hardcode the frame sets.

**Frame grid:** n = 0…1810, t = n/30 (film data: 1,811 frames,
30 fps, 60.357 s — the driver's own arithmetic).

**Agreement definitions** (painter-grounded, printed in the
tool's docstring): painted-open(v) := v > 0.05; **band-agree**
:= |osc − trk| ≤ 0.10 (mouth height differs ≤ 1.8 px, width
≤ 1.0 px — paints alike); **class-agree** := same open/closed
class; **differing** := not band-agree; **open-on-silence** :=
no phoneme active while the signal is painted-open (reported
for *both* signals); **missed closure** := phoneme active,
evaluated track ≤ 0.05, oscillator painted-open.

**Precision / what is not reproduced.** Python floats are IEEE
f64, matching the Rust law; `math.sin` is the platform libm, as
is Rust's `f64::sin` on Linux. Two residues, both bounded and
immaterial: cross-libm sin differences (~1 ulp) and the call
sites' `as f32` cast (≤ 2⁻²⁴ relative), which the tool does not
reproduce. Against thresholds of 0.05/0.10 these cannot flip a
classification except at the boundary itself — so the tool
counts frames with either signal within 0.001 of the 0.05
threshold: **1 frame** in the whole in-line set. Classifications
are robust.

### 1.1 Hand validation (the law, computed by hand)

Frame 15, t = 0.5, `on` = true (inside line 0). From the source
law, step by step:

```
13t            = 6.5 rad
sin(6.5)       = sin(6.5 − 2π) = sin(0.216815)
               ≈ 0.216815 − 0.216815³/6 + 0.216815⁵/120
               ≈ 0.216815 − 0.001699 + 0.000004 = 0.215120
5.3t           = 2.65 rad
sin(2.65)      = sin(π − 2.65) = sin(0.491593)
               ≈ 0.491593 − 0.019802 + 0.000239 − 0.000001 = 0.472031
envelope       = 0.6 + 0.4 × 0.472031 = 0.788812
talk           = 0.25 + 0.75 × 0.215120 × 0.788812
               = 0.25 + 0.75 × 0.169689 = 0.25 + 0.127267
               = 0.377267
```

Cross-checked with a 40-digit Decimal Taylor evaluation
(independent of libm): sin(6.5) = 0.21511998808781552,
sin(2.65) = 0.4720305412898826, talk = **0.37726695596532633** —
the hand chain agrees to 6 decimal places. The tool's
`--self-check` asserts its transcription reproduces
0.37726695596532633 to < 1e-9, plus a hand-computed track
bracket (keys (5.74, 0.0) → (5.967, 0.5); at t = 5.85,
u = 0.11/0.227 = 0.484581…, value = 0.24229074889867777,
asserted to < 1e-6), the off-state, hold-outside-range, and a
segment lookup. `SELF-CHECK: PASS` (verbatim in §2).

---

## 2. Verbatim results

```
$ python3 tools/lipsync/evaluate_track.py --self-check
[self-check] osc(0.5, on) = 0.37726695596533 vs hand 0.37726695596532633 -> OK
[self-check] track(5.85) = 0.24229074889868 vs hand 0.24229074889867777 -> OK
[self-check] off-state + hold-outside-range -> OK
[self-check] segment(0.31) = th/This -> OK
SELF-CHECK: PASS

$ python3 tools/lipsync/evaluate_track.py
[1 inputs] words=/home/hatch/faceless-shorts/the-soldier-who-didnt-know/data/words.json (146 tokens, 12 lines) · track keys=551 eases=['linear'] segments=564 · film frames=1811 fps=30 duration=60.357
    regime-B windows from source: W1 s03 [13.061, 15.370) · W2 s11 (51.680, 52.530) · painter threshold 0.05 · band 0.1

[2 regime A — oscillator bound to narration (on = inside line span), per line]
    line  0 [0.300, 5.740): frames 164 · band-agree 21.3% (35) · class-agree 95.7% (157) · differing 129 · mean|d| 0.2716 · osc-open-on-silence 21 · trk-open-on-silence 20 · missed-closures 6
    line  1 [5.967, 12.360): frames 191 · band-agree 25.1% (48) · class-agree 98.4% (188) · differing 143 · mean|d| 0.2651 · osc-open-on-silence 16 · trk-open-on-silence 15 · missed-closures 2
    line  2 [13.180, 15.620): frames 73 · band-agree 28.8% (21) · class-agree 95.9% (70) · differing 52 · mean|d| 0.2748 · osc-open-on-silence 19 · trk-open-on-silence 16 · missed-closures 0
    line  3 [16.040, 21.100): frames 151 · band-agree 20.5% (31) · class-agree 86.1% (130) · differing 120 · mean|d| 0.3193 · osc-open-on-silence 29 · trk-open-on-silence 12 · missed-closures 4
    line  4 [21.640, 25.900): frames 127 · band-agree 26.0% (33) · class-agree 98.4% (125) · differing 94 · mean|d| 0.2387 · osc-open-on-silence 26 · trk-open-on-silence 25 · missed-closures 1
    line  5 [26.220, 28.960): frames 82 · band-agree 34.1% (28) · class-agree 98.8% (81) · differing 54 · mean|d| 0.2334 · osc-open-on-silence 12 · trk-open-on-silence 11 · missed-closures 0
    line  6 [29.381, 33.760): frames 131 · band-agree 20.6% (27) · class-agree 96.2% (126) · differing 104 · mean|d| 0.2807 · osc-open-on-silence 27 · trk-open-on-silence 23 · missed-closures 1
    line  7 [34.300, 41.460): frames 215 · band-agree 30.2% (65) · class-agree 97.2% (209) · differing 150 · mean|d| 0.2427 · osc-open-on-silence 31 · trk-open-on-silence 30 · missed-closures 5
    line  8 [41.792, 45.245): frames 104 · band-agree 29.8% (31) · class-agree 99.0% (103) · differing 73 · mean|d| 0.2224 · osc-open-on-silence 15 · trk-open-on-silence 15 · missed-closures 1
    line  9 [45.425, 50.240): frames 145 · band-agree 30.3% (44) · class-agree 97.2% (141) · differing 101 · mean|d| 0.2469 · osc-open-on-silence 37 · trk-open-on-silence 34 · missed-closures 1
    line 10 [50.720, 55.160): frames 133 · band-agree 23.3% (31) · class-agree 100.0% (133) · differing 102 · mean|d| 0.2386 · osc-open-on-silence 10 · trk-open-on-silence 10 · missed-closures 0
    line 11 [55.580, 58.920): frames 100 · band-agree 26.0% (26) · class-agree 99.0% (99) · differing 74 · mean|d| 0.2497 · osc-open-on-silence 19 · trk-open-on-silence 18 · missed-closures 0
    ALL LINES: frames 1616 · band-agree 26.0% (420) · class-agree 96.7% (1562) · differing 1196 · mean|d| 0.2583 · osc-open-on-silence 262 · trk-open-on-silence 229 · missed-closures 21
    track leak outside lines: 119 of 195 out-of-line frames painted-open with no phoneme active

[3 regime A — difference distribution over in-line frames]
    |d| bins: <=0.05 208 · (0.05,0.10] 212 · (0.10,0.25] 489 · (0.25,0.50] 491 · >0.50 216 · osc hotter 898 · track hotter 718 · boundary-sensitive 1
    by active viseme (osc bound to narration):
        A: frames  254 · band-agree  15.0% · mean|d| 0.2824 · missed-closures 2 · osc-open-on-silence 0
        E: frames  122 · band-agree  25.4% · mean|d| 0.2233 · missed-closures 1 · osc-open-on-silence 0
        I: frames  229 · band-agree  38.0% · mean|d| 0.1860 · missed-closures 1 · osc-open-on-silence 0
        O: frames   71 · band-agree  26.8% · mean|d| 0.2280 · missed-closures 0 · osc-open-on-silence 0
        U: frames   55 · band-agree  14.5% · mean|d| 0.2790 · missed-closures 0 · osc-open-on-silence 0
        ch: frames   13 · band-agree  23.1% · mean|d| 0.2460 · missed-closures 0 · osc-open-on-silence 0
        dd: frames  136 · band-agree  35.3% · mean|d| 0.2222 · missed-closures 3 · osc-open-on-silence 0
        ff: frames   64 · band-agree  28.1% · mean|d| 0.2461 · missed-closures 2 · osc-open-on-silence 0
        kk: frames   36 · band-agree  30.6% · mean|d| 0.2285 · missed-closures 0 · osc-open-on-silence 0
        nn: frames  181 · band-agree  32.0% · mean|d| 0.2107 · missed-closures 1 · osc-open-on-silence 0
        pp: frames   36 · band-agree  13.9% · mean|d| 0.3293 · missed-closures 6 · osc-open-on-silence 0
        rr: frames   66 · band-agree  28.8% · mean|d| 0.2815 · missed-closures 3 · osc-open-on-silence 0
      sil: frames  262 · band-agree  21.0% · mean|d| 0.3475 · missed-closures 0 · osc-open-on-silence 262
       ss: frames   67 · band-agree  25.4% · mean|d| 0.2889 · missed-closures 2 · osc-open-on-silence 0
       th: frames   24 · band-agree  12.5% · mean|d| 0.3329 · missed-closures 0 · osc-open-on-silence 0

[4 regime B — the film as staged (oscillator only in its two diegetic windows)]
    W1 s03 Tani explains [13.061, 15.370): frames 70 · band-agree 30.0% (21) · class-agree 95.7% (67) · differing 49 · mean|d| 0.2651 · osc-open-on-silence 23 · trk-open-on-silence 20 · missed-closures 0
    W2 s11 Tani orders (51.680, 52.530): frames 25 · band-agree 16.0% (4) · class-agree 100.0% (25) · differing 21 · mean|d| 0.3076 · osc-open-on-silence 2 · trk-open-on-silence 2 · missed-closures 0
    oscillator painted-open frames film-wide: 95 of 1811; outside the windows the driver never assigns talk (state default 0)
    line  2 under regime B (window W1 covers its start): frames 73 · band-agree 27.4% (20) · class-agree 86.3% (63) · differing 53 · mean|d| 0.2804 · osc-open-on-silence 19 · trk-open-on-silence 16 · missed-closures 0
    line 10 under regime B (window W2 sits inside it): frames 133 · band-agree 3.8% (5) · class-agree 18.8% (25) · differing 128 · mean|d| 0.4788 · osc-open-on-silence 2 · trk-open-on-silence 10 · missed-closures 0
    FILM-WIDE (regime B osc vs narrator track — different speakers; reported for completeness, NOT an adoption metric): frames 1811 · band-agree 12.0% (217) · class-agree 12.1% (219) · differing 1594 · mean|d| 0.4267 · osc-open-on-silence 25 · trk-open-on-silence 348 · missed-closures 0

[5 densest-difference line for the A/B sheet gate] line 3 (79.5% of its frames differ under regime A)
EVALUATION COMPLETE — no pass/fail bar exists for this comparison; the numbers above are the evidence (LIPSYNC-EVAL-W59.md).
```

---

## 3. Reading the regime-A results

**3.1 The oscillator is not a rough approximation of the
track; it is a different signal.** Band-agreement is 20–34 %
per line, with no line above 35 %. The |Δ| distribution is
heavy-tailed away from zero: only 420 of 1,616 frames within
band, 707 frames (44 %) beyond 0.25 — a quarter of the whole
mouth-height range (6…24 px). Class-agreement (96.7 %) looks
reassuring until you see why it is high: during phonemes both
signals are usually *open*, so they agree on the binary while
disagreeing on everything else. Where the binary matters most
it fails outright:

- **Silence: 262 of 262 frames wrong in kind.** Every in-line
  frame with no phoneme active has the oscillator painted-open
  (its 0.25 floor makes any other outcome impossible). Mean |Δ|
  on silence frames is the worst of any class (0.3475).
- **Closures: unreachable.** The 21 `pp` phonemes (36 frames)
  exist to slam the mouth shut; the oscillator's minimum is
  five times the closure's value. Painted missed closures
  number 21 film-wide (the count is of frames where the
  evaluated track actually sits ≤ 0.05 — see §3.3 for why it
  isn't 36).
- **Fixed-shape visemes score worst.** `th` 12.5 %, `pp`
  13.9 %, `U` 14.5 %, `A` 15.0 % band-agreement: the classes
  with a *specific* correct openness are exactly where a free
  oscillator — whose value is set by the clock, not the word —
  is furthest off. Mid classes (`I` 38.0 %, `dd` 35.3 %) score
  best only because their values happen to sit inside the
  oscillator's typical range.

**3.2 Neither signal is "hotter" in a fixable way.** The
oscillator exceeds the track on 898 frames, the track exceeds
the oscillator on 718. There is no bias or gain correction to
apply; the signals are uncorrelated in phase (the oscillator's
phase is absolute time — §1), so per-frame error is structural.

**3.3 The track's own defect, measured: linear attacks leak
across silence.** W23 keys silence (talk 0) at a word's end
when the gap to the next word is ≥ 0.04 s, and each phoneme
value at its segment start; W15-linear evaluation then *ramps*
from 0 up to the next value across the whole gap — the attack
happens during the silence, not at the word. Consequences in
the painted record: the track itself is painted-open on **229
of 262** in-line silence frames and **119 of 195** out-of-line
frames (inter-line gaps and the 0.3 s before the first word,
where it ramps 0 → 0.4), and a `pp` closure paints closed on
only **6 of its 36** segment frames (the value touches 0.02 at
the segment-start instant, then ramps away).

Re-evaluating the *same keys* under W15 `step` semantics
(value = latest key at or before t — the vocabulary already
defines it, FORMAT-EXT-CLIPS-W15 §4, and `check_clips.py`
requires it on discrete channels) — a what-if measured with
this tool's frame loop, not asserted:

| measure (same 551 keys) | linear | step |
|---|---|---|
| in-line silence frames painted-open | 229 / 262 | **106 / 262** |
| out-of-line silence frames painted-open | 119 / 195 | **0 / 195** |
| `pp` frames painted closed | 6 / 36 | **36 / 36** |
| frames where the two semantics differ by > 0.10 | — | 967 / 1811 |

Step is not a free win either: the 106 residual in-line frames
are sub-0.04 s inter-word gaps, where W23 keys no silence, so
step *holds the previous phoneme's posture* for 1–2 frames.
That reads as co-articulation bridging (real mouths do not
slam shut for a 30 ms gap) — arguably correct, visibly
different from linear, and eliminable only builder-side (key
silence at *every* word end, at the cost of micro-chops). The
decision the lead actually faces is therefore three-way for
the silence classes — oscillator (262 open), track-linear
(229), track-step (106) — while for phoneme-active frames the
track wins under *either* semantics (the oscillator's
agreement there is 12–38 % per viseme; the track is the data).
Any adoption that binds the track with default linear
evaluation inherits the leak; the binding should name its
semantics explicitly (§5, item 2).

## 4. Reading regime B — what adoption must not touch

In the film as staged, the oscillator paints an open mouth in
exactly **95 of 1,811 frames** — every frame of its two
windows, never closed inside them (the floor law again). Those
windows are **diegetic**: in s03 the officer Tani explains the
situation (pose_explain, arm track), in s11 he delivers the
order the whole film turns on. The narrator whose phonemes the
W23 track encodes is a voice-over (lines.json voice
`bm_george`, per LIPSYNC-W23 §6.3) with no on-screen mouth.
Three consequences, stated plainly:

1. **Literal replacement at the two call sites would be a
   bug, not an adoption.** It would drive Tani's mouth with
   the narrator's phonemes. The regime-B "agreements" above
   (W2: 16 % band-agree) are cross-speaker coincidences and are
   reported only so nobody mistakes them for a metric.
2. **The windows themselves are anchor-bounded, not
   speech-bounded.** W1 opens at the s03 cut (13.061), 0.119 s
   *before* line 2's first word, and runs 2.309 s continuously;
   within it, 23 of 70 frames have no narration phoneme active.
   That is *not* proof Tani's mouth moves in silence — his own
   speech has no timing data anywhere (words.json is the
   narration) — it is proof that nothing in the current staging
   *could* synchronise him: there is no diegetic speech signal
   to bind, only two anchors per window. If diegetic lip-sync
   is ever wanted, it needs diegetic word timings as data;
   until then the oscillator is the only available driver for
   these beats, and its continuous flap is a known staging
   limitation, not a regression adoption introduces.
3. **Adoption, done by the binding rule in §5, changes zero
   frames of this film.** No shot currently binds a narration
   speaker to an on-screen character. The adoption's value is
   prospective: the first shot that *does* put the narrator (or
   any words.json-timed speaker) on screen gets phoneme-true
   mouth motion instead of a 26 %-agreement oscillator — and
   the gate in §7 exists to prove that moment when it comes.

## 5. Adoption patch sketch (for the lead)

1. **Track loading — the clips lane, not a new parser.** The
   track is already W15 data: a scene file carries it under
   `clips` (clip `lipsync-onoda`, one track, target
   `{node: "narrator", channel: "talk"}`), validated today by
   `tools/format/check_clips.py` (PASS 5/5, LIPSYNC-W23 §1).
   Adoption rides the §15 clips adoption, in its order: the
   loader learns to *instance* a clip on a character node
   (`clip` + `clipTime {offset, rate, loop: "clamp"}` — the
   demo file's node stub shows the exact shape) and the
   evaluator resolves the track at scene time per W15 §4
   (time map first, then bracketing keys). No new file format,
   no lipsync-specific loader code: the film-data lane
   (`load_film_data`) does not need the track at all unless a
   hardcoded-lane shot binds one, in which case the same
   evaluator is called with the clip parsed from the film's
   scene file — one evaluator, two data sources.
2. **The binding must name its semantics.** W15's default
   ease machinery is smootherstep, the demo keys are authored
   `linear`, and §3.3 shows the silence classes are decided
   by this choice. Recommended binding for the `talk` channel:
   evaluate authored keys but treat post-silence attacks as
   steps — concretely, either (a) the builder emits `step` on
   every key whose predecessor key is a silence key (value 0
   after a word end), keeping intra-word attacks linear, or
   (b) the evaluator steps any segment that *starts* from a
   silence key. (a) keeps semantics in the data (validator can
   check it); (b) keeps W23's files untouched. Either way the
   choice is recorded in the clip, never implicit. Expected
   effect, from §3.3's what-if: silence-leak 229 → ~106
   in-line (residue = sub-0.04 s gaps), 119 → 0 out-of-line,
   closures painted 6 → 36 of 36.
3. **Call sites.** In `src/film.rs` the two existing
   assignments — `:1478` (s03) and `:1947` (s11) — **do not
   change**: they are diegetic windows on a character with no
   speech data (§4). The adoption site is the staging of any
   shot whose character *is* the track's speaker: the
   assignment becomes, in the scene-file lane,

   ```
   st.talk = match node.clip_on("talk") {
       Some(clip) => eval_clip_scalar(clip, scene_t) as f32,  // W15 S4 + item 2
       None       => talk(t, on) as f32,                     // unchanged legacy
   };
   ```

   i.e. the oscillator remains the fallback for *unbound*
   characters — a staging-visible property of the node (it
   either instances a clip or it doesn't), exactly the split
   LIPSYNC-W23 §6.1 prescribed. Speaker attribution is the
   binding's precondition: the track's target node name must
   be the character the staging is driving. The multi-speaker
   upgrade (per-line clips from the segments table's `line`
   field) is mechanical and stays out of this adoption.
4. **Fallback when a token failed — the generic rule.**
   Failure is handled at *build* time, in the data, never at
   evaluation time: `build_visemes.py` already emits a failed
   token as an explicit silence segment spanning its [s, e],
   counts it, and lists it in `lipsync.failedWords` /
   `counts.failed` (this film: 0 failed; the fixture exercises
   the G2P path, and a token yielding no phonemes at all is
   the failed case). The evaluator therefore needs **no**
   failure branch — a failed word is authored silence, and
   `check_clips` + a build report showing `failed > 0` are
   where a human sees it. The remaining generic cases:
   - *Clip instanced, t outside the key range* → W15 hold
     semantics (first/last value). Not an error.
   - *Speaking shot, no clip instanced* → the oscillator,
     by item 3 — visible in staging, never a silent runtime
     substitution for a *missing file*: if staging references
     a clip name the scene file doesn't define, that is a
     `check_clips` [4 references] failure at validation time,
     before any render.
   - *Track stale vs words.json* → regeneration is
     deterministic (W23: byte-identical output, sha256
     recorded), so CI can regen-and-diff; a mismatch is a
     build failure, not a runtime fallback.
5. **What adoption deliberately does not do:** sub-word
   timing stays the weighted split (W23 §3 — a duration
   model, stated as such); mouth *shape* stays a scalar (the
   segments table remains the seam for a future shape
   channel); the 8 G2P tokens in this film (all proper names —
   *onoda* ×4, *hiroo*, *norio*, *suzuki*, *philippine*)
   inherit W23's documented G2P mispronunciations — the
   `--pronunciation` override is the fix path, and this
   evaluation inherits the caveat rather than laundering it.

## 6. Determinism and paint cost

Both signals are closed-form functions of t (W15 §4 says it
of clips verbatim: "a clip is a pure function of t — no state,
no frame history"; the oscillator is the same shape), so
adoption neither gains nor loses determinism *of the value*.
The difference is in the paint cache. `state_key`
(`src/film.rs:2161`, key at `:2176`) keys `talk` by exact f32
bits (the post-`cabcc30` law — a quantised key was the 72.34 dB
determinism bug), so *any* talk change repaints the character
card. Under linear evaluation the track's value differs on
essentially every speaking frame — the same repaint profile
the oscillator already imposes (LIPSYNC-W23 §6.2's
cost-neutral argument holds). Under the §5-item-2 step
binding, talk is constant between keys: at most 551 distinct
values film-wide, and whole segments share one card wherever
the rest of the state also recurs — strictly better than
today, never worse. Adoption also retires a code path *as the
default mouth*: `talk()` survives only as the explicit
unbound-character fallback (item 3), so the oscillator's
remaining uses are enumerable from staging instead of being
the invisible default for speech.

## 7. The A/B gate (specified, not run — adoption changes no
frame of this film until a shot binds a speaker)

Mouth-region crops are **not available** (the A/B tooling is
full-frame), so the gate is built from full-frame instruments
plus human sheet review:

1. **Candidate frame set.** For a bound shot, the repaint set
   is every frame whose talk value changes — under exact-bits
   state keys that is all 1,616 in-line frames of the bound
   lines; the *visibly* differing subset is the 1,196
   band-differing frames (§2). The gate renders the bound
   shot(s) twice (oscillator binding vs track binding, same
   binary otherwise) and runs the full differing set through
   PSNR; no sampling inside the densest line.
2. **PSNR instrument.** `tools/ci/byte_probes.py BASELINE_DIR
   CANDIDATE_DIR --ranges <list>` (positional dirs, `--ranges`
   in the house `482-632` style). For the rehearsal binding on
   this film's data the densest-difference line is **line 3**
   (79.5 % of frames differ; frames **482–632**, t ∈ [16.04,
   21.10)) — the gate's minimum viable rehearsal even before a
   production binding exists. Expectation to record, not a
   threshold: full-frame PSNR will read *high* (the mouth is a
   small region of a 1080×1920 frame), so PSNR is the
   regression tripwire (any differing frame *outside* the
   bound shot's lines, or any frame the evaluation says
   band-agrees scoring < 60 dB, is a defect), **not** the
   acceptance measure.
3. **Sheet review.** `tools/film/make_sheet.py` over the
   densest line's window (`--from 16.04 --to 21.10`, defaults
   otherwise) from each binding's frames; the reviewer checks
   the three classes §3 names: mouth shut in the line's
   inter-word gaps, closures on its `pp` words, openness
   tracking the vowels — the sheet is the acceptance evidence,
   signed off by the user like the W24 locomotion A/B (pixels
   change by design; this is a sign-off gate, not a
   byte-identity gate).
4. **Invariants.** `FILM TELEMETRY: CHECK OK` must be
   unchanged (talk feeds no telemetry channel — captions,
   cutouts, continuity are computed from staging data, and the
   mouth is interior paint that does not alter a character
   card's alpha silhouette; stated as an expectation for the
   gate to confirm, since this seat ran no render). The
   determinism probes (byte_probes default ranges, double
   render byte-identical) must stay green — §6 says why they
   should.

## 8. Limits of this evaluation

- **Data-level only.** No frames were rendered; every claim
  about what *paints* routes through the painter's own
  threshold and geometry formulas (`toonfull.rs:698–700`),
  not through pixels. The gate in §7 is where pixels enter.
- **One film, one track.** The track is W23's demo artefact,
  regenerated deterministically from the project's
  words.json; if the project's timings are ever re-cut, the
  track and this evaluation must be regenerated together.
- **Diegetic speech is unknowable from the data.** §4's W1
  numbers compare Tani's window against the *narration's*
  phonemes because those are the only phonemes that exist as
  data; they bound what staging could synchronise, they do
  not grade Tani's performance.
- **Phoneme timing inside words is a model.** Segment
  boundaries come from W23's weighted split of word spans
  (no sub-word ground truth exists in words.json); frame
  counts near boundaries inherit that model's error, which is
  why agreement is reported with a 0.10 band rather than
  exact equality.
