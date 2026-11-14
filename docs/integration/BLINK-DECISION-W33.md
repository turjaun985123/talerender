# BLINK-DECISION-W33 — Blink adoption: measured paint-cache cost of smooth vs quantized vs today's binary

**Delivered:** `tools/perf/blink_cost.py` (new file only; nothing
else touched, nothing committed). Stdlib-only, analytic — no
binary runs, no renders. Reproduce with:

```
python3 tools/perf/blink_cost.py            # tables on stdout
python3 tools/perf/blink_cost.py --json out.json
```

**The question** (left open by W12's cost note in
`docs/integration/CHARANIM-W12.md`): `state_key` keys the paint
cache on the exact f32 bit pattern of `FullState.blink`
(`src/film.rs:2152-2179`), so W12's smooth blink curve would
paint several distinct cards per blink event where today's
binary `Blinker` paints one. W12 estimated "~7 paints where
today there is 1" and offered `blink_amount_q(id, t, 8)` as the
bounded alternative. This seat measured the real numbers over
the real film staging instead of estimating them.

## Headline numbers (film-wide, exact simulation)

| Model | Events | Frames blink>0 | Distinct values per event (D) | Extra paints, ceiling | Ceiling as % of lookups |
|---|---|---|---|---|---|
| today (binary `Blinker` + beats) | 43 | 159 | 1.00 (max 1) | — (baseline) | — |
| **smooth** `blink_amount` | 36 | 255 | **mean 7.08, max 8** | **+219** | **6.06%** |
| **quantized** `blink_amount_q(·, 8)` | 35 | 199 | mean 4.51, max 6 | **+123** | **3.41%** |
| smooth + beats preserved | 42 | 279 | mean 6.21, max 8 | **+219** | 6.06% |
| quantized + beats preserved | 41 | 224 | mean 4.00, max 6 | **+123** | 3.41% |

Denominator: a full film render issues exactly **3,612 cache
lookups** (one per staged character per frame — see Method).
"Extra paints, ceiling" is the r = 1 model defined below; each
extra paint is one full 2560×2816 RGBA canvas
(`CANVAS_BYTES = 28,835,840` B, `src/paintbudget.rs:35`), so
the ceilings are also 5.9 GiB (smooth) / 3.3 GiB (quantized) of
canvas paint traffic across a 1,811-frame film — ≈ 0.12 /
0.07 extra paints per film frame.

W12's "~7" estimate was exact: the smooth curve's measured
mean is **7.08 distinct f32 values per event** (an event spans
7–8 frames at 30 fps; the curve essentially never repeats a
sampled value — Onoda's film-wide distinct-value count is 101
for 101 blink>0 frames).

## Method

Frame-accurate simulation, no approximation in the blink
component itself:

- **Frame times** `t = n/fps` per the manifest's frameRule
  (`scenes/film/film.json`), fps = 30, 1,811 frames, segment
  frame ranges and durations from the same manifest.
- **Models ported bit-faithfully** (f64 arithmetic in the
  Rust code's op order; f32 exactly where Rust casts):
  splitmix64 / `seeded_unit` / `blink_start` / smootherstep /
  `blink_amount` / `blink_amount_q` from `src/charanim.rs`;
  `Blinker::at` (interval table [2.7, 3.9, 3.1, 4.4, 2.9, 3.6]
  rotated by floor(seed·7) mod 6, clock offset seed·1.37, shut
  0.12 s) and `blink_at` from `src/film.rs:179-210`, combined
  as `blink_of` does today (`max`, `src/film.rs:1391-1393`).
  Port self-checks run on every invocation and PASS: the
  charanim port reproduces the Rust test's cadence band for
  id 3 (35 crossings of 0.5 in 120 s, band [25, 48]); first
  starts ∈ [0.6, 2.0) and gaps ∈ [2.6, 4.4) for all five ids;
  the Blinker port yields 34–36 events/120 s per seed against
  the table's 3.4333 s mean interval (≈35).
- **Presence** (who is staged, with which seed, in which
  segment) is transcribed into the tool from the fourteen
  shot functions in `src/film.rs`, with the blink-assignment
  line cited per character (s01:1407 … s12:1977). Presence
  means *staged*, because that is who the cache is consulted
  for: `resolve_chars` looks up every character in
  `f.chars` every frame (`src/film.rs:2182`), in-frame or
  not. Screen time per character (presence frames / fps):
  Onoda (id 3) 51.90 s, compA (7) 19.73 s, compB (11) 19.73 s,
  Taniguchi (9) 17.90 s, Suzuki (5) 11.13 s — 120.41
  character-seconds, 3,612 character-frames in total.
- **Events** are maximal runs of blink > 0 over a character's
  presence, counted once per character film-wide even across
  a cut (per-segment tables in the tool output count a
  straddling event in each segment it touches; in this data
  the two views reconcile exactly — segment extras sum to the
  film-wide 219 / 123).
- **D** = distinct nonzero f32 blink values sampled during
  one event. **Extra paints at r = 1** = Σ (D − 1) over events,
  minus the same for today's model (identically 0: binary D ≡
  1). The r factor is the one modelled step — next section.

## The r = 1 assumption, and why the ceiling is conservative

(D − 1) extra paints per event are actually paid only for the
fraction r of events whose *other* state components repeat
bit-exactly across the event's frames (held pose, standing
listener, settled tail). Where the rest of the state is unique
every frame anyway — walk phase advancing, `talk`'s
oscillator, eased pose tracks, `settle` ringing, or
`cam_pitch` changing under a moving camera (it is a key
component, set per frame in `finish_chars`,
`src/film.rs:1352`) — the blink value rides along on
paints that happen regardless, and the marginal cost is 0.
The true extra is Σ (D − 1)·r_event with r_event ∈ [0, 1],
unknowable without running the full state evaluator; the
headline numbers are the r = 1 ceiling.

Directional evidence that the real figure sits well below the
ceiling: only **17.6%** of smooth blink>0 frames fall inside
camera-hold windows (where `cam_k(lt, d, hold)` sits at an
endpoint — hold 0.1 in every shot except s02's 0.03). Outside
those windows the camera is mid-move for ~80% of every
segment. Inside them the character must *also* be self-frozen
to pay full price, and several hold-window characters are
mid-gesture (pose tracks span the segment). This is evidence
about direction, not a bound, and no mid-point estimate is
offered — the ceiling is the decision number.

## Per-character detail (film-wide)

| Character | id | Screen | Today | Smooth extra | Quant extra |
|---|---|---|---|---|---|
| Onoda | 3 | 51.90 s | 21 ev / 82 on | 15 ev / 101 on / **+86** | 14 ev / 79 on / **+45** |
| compA | 7 | 19.73 s | 5 ev / 15 on | 6 ev / 45 on / **+39** | 6 ev / 34 on / **+20** |
| compB | 11 | 19.73 s | 6 ev / 24 on | 5 ev / 36 on / **+31** | 5 ev / 30 on / **+22** |
| Taniguchi | 9 | 17.90 s | 6 ev / 19 on | 6 ev / 43 on / **+37** | 6 ev / 34 on / **+23** |
| Suzuki | 5 | 11.13 s | 5 ev / 19 on | 4 ev / 30 on / **+26** | 4 ev / 22 on / **+13** |

Per-segment rollup (extra paints at r = 1, smooth / quant):
s01 +12/+7 · s02 +32/+19 · s03 +24/+14 · s04 +25/+15 ·
s05a 0 (nobody staged — plane and leaflets only) ·
s05b 0/+0 (no schedule fires in its 1.57 s under *any* model,
today's included) · s06 +25/+15 · s07 +6/+4 · s08 +26/+13 ·
s08b +6/+2 · s09 +19/+10 · s10 +13/+8 · s11 +18/+12 ·
s12 +13/+4. Full per-segment × character tables are the
tool's stdout.

Two observations the tables make visible:

- **The channel blinks slightly less often than today**
  (36 vs 43 events): charanim's mean gap is 3.5 s vs the
  Blinker table's 3.4333 s, and today's count includes the
  seven authored word-beat blinks. Cadence parity is close;
  nobody gains or loses a meaningful number of blinks.
- **Exposure, not count, is the visible change**: blink>0
  frames rise 159 → 255 (smooth) because an event lasts
  0.24 s (0.12 down + 0.12 up) against the binary 0.12 s
  shut. That is the fidelity being bought, priced above.

## The beats question — decided by the numbers

Seven staged blinks today are *authored*, not scheduled:
`blink_of(seed, t, &[anchor])` forces a blink on a narration
word (years0 s01, never2 s03, message3 s04, found7 s08,
commander8 s09, bookseller9 s10, loyal11 s12). Adopting the
charanim channel as a bare replacement silently deletes all
seven. The measured cost of keeping them — evaluating
`max(channel, blink_at(beats))` exactly as staging does today
— is **+0 extra paints under both variants** (219 and 123
unchanged), while restoring +6 events / +24 blink>0 frames
(smooth). The mechanism: a beat-only run takes the single
value 1.0, so its D − 1 is 0, and where a beat overlaps a
channel event it clamps frames to a value the curve already
samples near its peak. **Whichever variant is adopted, adopt
it with the beats maxed back in** — directorial fidelity at
zero measured paint cost.

## Assumptions, and what could NOT be derived

Stated plainly, per the seat's brief:

1. **Presence = staged for the whole segment.** Verified by
   reading all fourteen shot functions: every `char()` call
   has exactly one blink assignment and vice versa, and no
   character is staged conditionally for part of a segment —
   with one benign exception: s11 stages Onoda in both
   branches of its drop_t split (walk-in / post-drop), seed 3
   throughout, so presence is continuous. What staging does
   NOT tell us is *framing*: a staged character the camera
   crops still costs cache lookups, so staged presence is the
   correct population for cost — but "screen time" above means
   staged time, which can exceed on-screen time.
2. **Channels are evaluated at film time t**, as today's
   `Blinker.at(t)` call sites are and as W12's integration
   sketch prescribes (`channels_at(char_id, t)` replacing
   them). If integration instead fed shot-local `lt`, event
   counts would shift by phase luck only — the schedules are
   stationary, so D means and the ceiling's scale would not
   move, but the exact integers would.
3. **The film's total distinct-state count cannot be computed
   analytically**, and no measured full-film figure exists in
   the repo to cite (the one `paint-cache:` line in
   `docs/integration/PAINT-BUDGET-W06.md:167` is an
   illustrative output-shape example, not a run record). It
   is bounded above by the 3,612 lookups. That is why the
   condition below is stated against lookups (exact) and why
   the real denominator should be read off the next full
   render: the wired budget tracker now prints
   `paint-cache: … misses=… rerenders=…` (W06 wiring,
   `86f0c81`), and distinct states = misses − rerenders as
   the tracker defines them. For scale: if that denominator
   lands at 1,500, the ceilings are 14.6% / 8.2% of it; at
   3,000, 7.3% / 4.1%.
4. **Per-paint wall time is not derivable** from repo data
   (`paint_toon_full` cost is not isolated in PERF), so cost
   is reported in paints and canvas bytes, never seconds.
5. **Eviction interplay is real but second-order**: extra
   distinct states also occupy budget (2 GiB ≈ 74 canvases
   resident, W06), so on long frozen shots they can evict
   entries that later re-render. Rerenders are already a
   modelled budget line; the ceiling counts distinct states,
   not eviction cascades.
6. **Scope is blink only.** W12's idle channel has the same
   key-exposure in principle; per W12's own analysis its
   marginal cost is nil on animated shots and real only on
   locked-off stills, and it is not measured here.

## Decision summary

**The measured cost difference.** Smooth: at most **+219
paints** (+86 of them Onoda's) — 6.06% of the film's 3,612
cache lookups, D mean 7.08 per event, confirming W12's
estimate. Quantized (8 levels): at most **+123 paints** —
3.41% of lookups, D mean 4.51, and a hard structural bound
smooth lacks: only 8 nonzero blink values *ever* enter the
key per character, film-wide (7 for compB in practice).
Today's binary: 1 value, 0 extra, by construction. The real
extra for either variant is below its ceiling — plausibly
far below, since ≤17.6% of blink frames even occur while the
camera is static — but the ceilings are the honest numbers
to decide on.

**The visual difference** (from W12 + the painter's own
constants): the painter already renders partial blink — eye
height scales by `1.0 − blink·0.6` and the closed-eye line
appears only above 0.6 (`src/toonfull.rs:593,609`). Binary
never shows that range: the lid teleports shut for 0.12 s.
Smooth traverses it with a shaped 0.12 s down / 0.12 s up
smootherstep, doubling event length to 0.24 s. Quantized
traverses the same curve in 0.125 blink steps — eye-height
steps of 7.5 percentage points, each lasting ~1 frame
mid-curve and ~2 near the tails at 30 fps; the 0.6
closed-line threshold is crossed at the same instants to
within a frame.

**Recommendation.**

1. **Adopt SMOOTH, with the beats maxed back in**
   (`max(blink_amount(id, t), blink_at(t, beats))` at the
   seven `blink_of` call sites, channel alone elsewhere) —
   *condition: the +219-paint ceiling (6.06% of lookups) is
   acceptable.* It is the only variant that renders the
   partial-lid range the painter already supports, its cost
   is bounded, small in absolute terms (≈0.12 extra paints
   per film frame), and overstated by construction.
2. **If the paint budget is judged tight, adopt QUANTIZED
   with beats instead** — it halves the ceiling (+123,
   3.41%) and caps the blink key-space at 8 values forever,
   at a stepping cost that sits at the edge of
   perceptibility on a 0.24 s event.
3. **Do not adopt either variant without the beats**: the
   channel alone deletes seven authored blinks, and putting
   them back is measured to cost nothing.

The one follow-up that would sharpen this from ceiling to
measurement: on the next full film render, read
`misses − rerenders` off the `paint-cache:` line for the true
distinct-state denominator, then re-render with the channel
wired and diff the same line — the delta is the realised r,
and it should land well under 219 / 123.
