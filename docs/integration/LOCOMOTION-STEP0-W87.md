# LOCOMOTION-STEP0-W87 — registration step-0 pack + user decision sheet (Wave 2, Batch 9, seat W87)

**Seat:** TaleRender Wave 2, W87 (locomotion registration step-0 pack
+ decision sheet). **Date:** 2026-11-15. **Repo HEAD at writing:**
`c440d90` (the lead's ab1 delimiter fix, on top of the Batch-8
central commit `021ca01`).
**Status:** pack only — nothing registered, nothing wired, no
pixels changed by this seat. One new file (this note); nothing
else written, nothing committed.

**Purpose.** `src/locomotion.rs` (W24, Batch 3) holds the gait
laws as code + tests, but it is **not a module of the binary**:
`main.rs` never declares it, nothing calls it, and its 9 tests
have only ever run under W24's standalone `rustc --test`
harness — CI has never compiled the file. Batch 6's
LOCOMOTION-AB-W57.md (the adoption pack: census, A/B protocol,
decisions D1–D4) makes registration **step 0** of the adoption:
a one-line change that by itself changes zero pixels and puts
the gait tests under CI permanently. Part 1 specifies that
patch exactly, with its zero-pixel proof protocol. Part 2 is
the drift check (the file today vs W24's delivery). Part 3 is
the **user decision sheet** — self-contained, plain language —
for the adoption decision itself, which is the user's to make
at wave close.

---

## Part 1 — The registration patch, exactly

### 1.1 The change

One line in `src/main.rs` (the lead's file — this pack
specifies, the lead lands). The mod list at HEAD `c440d90`,
verified by grep this session (lines 18–36):

```
18  mod ab1;
19  mod anim;
20  mod atlas;
21  mod battery;
22  mod bufferpool;
23  mod charanim;
24  mod composer;
25  mod dof;
26  mod errors;
27  mod gpucore;
28  mod math;
29  mod paintbudget;
30  mod rendergraph;
31  mod runlog;
32  mod scene;
33  mod scenefile;
34  mod telemetry;
35  mod film;
36  mod toonfull;
```

The list is alphabetical from `ab1` through `telemetry`, with
`film`/`toonfull` appended after. Insert in the alphabetical
run, between `math` (line 28) and `paintbudget` (line 29):

```diff
 mod math;
+mod locomotion;
 mod paintbudget;
```

Everything from line 29 down shifts by one. No other file is
touched. (For the record only: `cameramoves` is in the same
unregistered state — W25's module, a separate seat's concern,
not part of this pack.)

### 1.2 Why this changes zero pixels

- **Zero call sites.** Verified at `c440d90`:
  `grep -rn "locomotion::" src/` → no hits;
  `grep -n "locomotion" src/main.rs` → no hits. Nothing in the
  crate references the module, so no execution path can reach
  it, registered or not.
- Registration does exactly two things: rustc now compiles
  `src/locomotion.rs` into the crate, and its `#[cfg(test)]`
  tests join the test harness. The module is pure functions
  over its own types (`GaitParams`, `FootState`, `GaitState`,
  `eval`, `foot_at`, `phase_at_time`, … — W24 §7); it holds no
  state and shares nothing with `film.rs` staging. Compilation
  cannot alter staging behaviour.
- The proof that this held is §1.4 — byte-level, not this
  argument.

### 1.3 What CI then compiles and runs

- **Build step** (`.github/workflows/ci.yml`, step "Build",
  `cargo build --release`): compiles `locomotion.rs` for the
  first time in CI. Expect at most `dead_code` warnings for
  its public items (nothing calls them yet). CI does **not**
  deny warnings — its only `RUSTFLAGS` is the mold linker
  (`ci.yml:22`) — so warnings cannot fail the build.
- **Test step** (`.github/workflows/ci.yml`, step "Test",
  line 63–64, `cargo test --release`): gains exactly the
  module's **9 tests**:

  ```
  legs_exactly_antiphase            planted_foot_world_velocity_zero
  stance_sweep_matches_ground_speed arm_counterphase_same_side_leg
  bob_two_minima_per_cycle_at_double_support
  swing_continuous_and_lifted       walk_run_blend_duty_factor
  phase_law_matches_film_walk_phase painter_phase_origin
  ```

  (Grep census for scale: 195 `#[test]` attributes exist in
  `src/` text, of which 20 sit in the two unregistered files —
  locomotion's 9 and cameramoves' 11. Registration moves
  exactly +9 into the compiled suite.)
- If any of the 9 fails in-tree — they passed standalone in
  W24's harness — that is a real finding about the in-tree
  build, not a formality: stop, report, and do not proceed to
  any wiring step on top of it.

### 1.4 The zero-pixel proof protocol

Dispatch shape: `render.yml` is `workflow_dispatch` with
`mode` ∈ {ab1, dof-chart, film, scene-file} and `from`/`to`
frame inputs for film mode. The probe set is
`tools/ci/byte_probes.py`'s default —
**195-204, 795-814, 1195-1204 (40 frames)** — chosen
deliberately: all three ranges sit inside gait-staged footage
(195-204 ⊂ s02 advancing, 795-814 ⊂ s06 advancing,
1195-1204 ⊂ s08's standing tail), so byte-identity there
proves registration changed nothing *even where the later
adoption will*.

1. **Before.** At the pre-registration HEAD, dispatch
   `render.yml` three times, `mode=film`:
   (from=195, to=204), (from=795, to=814), (from=1195,
   to=1204). Keep the three frame dirs as BEFORE.
2. **Land the one-line commit.** CI must come back green —
   build + the Test step now including the 9 gait tests
   (§1.3). This is also the test-join proof.
3. **After.** Repeat the same three dispatches at the
   registration HEAD. Keep as AFTER.
4. **Compare.**

   ```
   python3 tools/ci/byte_probes.py <BEFORE-dir> <AFTER-dir>
   ```

   (default ranges are exactly the probe set above).
   **Required: 40/40 byte-identical, exit 0.** Any differing
   frame means something other than the mod line moved —
   stop and diagnose before any further step.
5. **Telemetry unchanged.** A film-telemetry run at the
   registration HEAD must print `FILM TELEMETRY: CHECK OK`
   and exit 0 — the same verdict line and exit code as at the
   pre-registration HEAD.

Step 0 is independently landable and independently
verifiable; no staging file is touched, and nothing in this
pack needs re-issue if the lead lands it before, during, or
after the other fixes-pass work. (Forward note for whoever
runs W57 §5 later: after the *wiring* step, this same
default-probes run must **fail on all three ranges** — that
failure is the adoption's expected signature, per W57 §5.3,
not a regression alarm.)

---

## Part 2 — Drift check: the file today vs W24's delivery

Verified first-hand at HEAD `c440d90`:

- **History.** `git log --follow -- src/locomotion.rs`
  returns exactly one commit: `ff52b3a` (Wave 2 Batch 3 —
  W24's delivery). No lead edits, no fixes, no touches of any
  kind since.
- **Bytes.** The file's blob hash is
  `773297f7eabf26c8acf411530de7f592aa868b6e` at `ff52b3a`
  and the identical hash at HEAD; `git diff ff52b3a HEAD --
  src/locomotion.rs` is **0 lines**. The file on disk is
  byte-identical to what W24 delivered.
- **Tests today.** 9 `#[test]` functions (names in §1.3) in
  693 lines — the same 9/9 W24 delivered; nothing added,
  nothing removed.
- **Registration state.** `main.rs`'s mod list at `c440d90`
  is character-for-character the list W57 quoted at `aa92cee`
  (§1.1); `locomotion` appears nowhere in `main.rs`, and
  `locomotion::` appears nowhere in `src/`. W57's step-0
  finding stands unchanged.

**Drift verdict: none.** The adoption pack (W57) was written
against this exact file, and its census, parameters, and
protocol need no revision on drift grounds.

---

## Part 3 — USER DECISION SHEET: should the film adopt the gait module?

*This part is self-contained. It is about one question: the
Onoda film's walking was staged by hand. A separate piece of
code — the "locomotion module" — computes walking from the
physical laws of gait, and its arithmetic is proven by tests.
Adopting it means re-rendering the film's walking shots with
the module's numbers instead of the hand-staged ones. Nothing
is adopted yet. The change is fully mapped in advance (an
earlier audit counted every affected frame), and the decision
is yours, because it is a judgement about how the motion
looks — the numbers can only say where the picture changes,
not whether it looks better.*

### What would change on screen

**1. The bobbing rhythm of walking bodies is currently
upside-down; the module sets it right.**
When a person walks, the body is at its *highest* as the
support leg passes underneath, and dips *lowest* in the brief
moment both feet are on the ground. The film's hand-staged bob
does the opposite: the body rides highest when the legs are
split widest and dips as they pass each other. The module
replaces that rhythm with the physical one. It also makes the
bob a little smaller (about two-thirds of today's travel —
at the film's largest setting, ~3.3 px of rise-and-fall
instead of 5 px) and smoother in shape. **The legs themselves
do not change pose at all** — only the body's vertical rhythm
on top of them. If a leg ever looks different in the
comparison, the wiring is wrong; that is not part of this
change.

**2. Walking characters gain arm swing; today their free arms
are still.**
In real walking, each arm swings *opposite* to the leg on the
same side — left arm forward as the left leg goes back. That
counter-swing is what the module adds (a gentle 16° shoulder
swing by default; the size of the swing is a dial you can ask
to have turned up or down without reopening anything else).
An arm swinging *with* its own leg is the marching-robot look;
the module's tests prove its swing is exactly counter-phase.
Arms that are doing something stay untouched: the arm holding
a coconut (s07), the arm holding the paper map (s08), the arm
holding the book (s10), and both arms in the s11 walk-in keep
their staged poses exactly.

**3. Some standing characters shift height slightly — this is
the part nobody would think to look for.**
The film's staging writes a bob value even when a character
is standing still at the start or end of a walk, so swapping
the bob formula also re-levels standing figures, constantly,
for as long as they stand in those shots. The sizes are
small: the opening frames of the s02 march and the s06 walk
lift by about 3 px; most standing tails settle by about 1 px.
The one to look at is **s10 (the bookshop): Tani stands about
2 px higher for roughly 4.4 seconds** before her short walk.
It is not motion — the figure simply stands at a slightly
different height for that stretch.

**Scale, honestly.** Of the film's 1,811 frames: 471 have
someone actually walking; 888 frames (about half the film)
take the bob change, including the standing frames above; 448
frames gain arm swing. The other **923 frames — eight whole
shots and the second half of s11 — do not change at all**,
and the checking protocol requires them to come out
byte-for-byte identical, not merely similar.

### The four decisions

- **D1 — Standing frames: include them, or only change the
  walking itself?** (The engine lead decides the mechanics;
  the s10 standing stretch is yours to accept or send back.)
  *Include (recommended):* one walking law everywhere; the
  standing re-levels listed above happen, all small and all
  shown on the comparison sheets. *Exclude:* only the 503
  truly-walking frames change, but then a character's height
  steps visibly — by up to ~3 px — at the exact moment each
  walk starts and stops.
- **D2 — Four companion walkers' resting arm pose: keep
  theirs, or give them Onoda's?** Four background walkers (two
  in s02, two in s06) were never given an arm pose, so they
  use a plain default. The module's own default arm pose is
  Onoda's, which is different — wiring it in carelessly would
  move their resting arms too (one arm shifting noticeably at
  the shoulder and elbow), on top of adding swing.
  *Recommended:* keep each walker's current resting pose —
  this adoption should change how characters walk, not how
  they are posed.
- **D3 — Arms on the in-between frames: swing only while
  actually walking?** *Recommended: yes.* A standing character
  keeps its staged arms, and during s02's staged "slide" at
  the end of the march (feet planted, group gliding) the arms
  stay still rather than swinging over planted feet.
- **D4 — Yours alone: s11's arms, and the adoption itself.**
  The s11 walk-in uses arms staged from the source material;
  the module's author ruled they stay as they are unless you
  ask to re-stage them. And the adoption as a whole replaces
  the film's current walking with a new version — accepting it
  makes the new render the film's reference of record (the
  golden frames get re-struck through the usual process).

### How you would judge it — the evidence pack

Before anyone asks for your verdict, the lead runs a fixed
procedure (from the audit pack, LOCOMOTION-AB-W57 §5):

1. Registration only (Part 1) — proves nothing moved.
2. A baseline check that today's build still reproduces the
   film's existing reference frames exactly.
3. The wiring, at the seven walking sites only.
4. Machine checks: the frames that must change are confined
   to the walking shots; the 923 untouched frames must be
   byte-identical; the film's caption/cut telemetry must
   still pass; one walking stretch rendered twice must match
   itself exactly.
5. **Contact sheets for your eyes** — one before and one
   after sheet for each walking shot, sampled several times
   per step: s02 the march (0:06–0:13), s06 the companions
   (0:26–0:30), s07 (0:30–0:34), s08 (0:34–0:40), s10 the
   bookshop stand and short step (0:49.6–0:50.6), s11 the
   walk-in (0:50.6–0:51.6).

What to look for, shot by shot: in **s02**, the body should
read as weight settling — highest as legs pass, dipping when
both feet land — and every arm should swing *against* its
same-side leg, with no pop where walking starts, stops, or
slides. In **s06**, no arm jump as the walk begins and a
settled stance on arrival. In **s07**, the free left arm
swings while the coconut arm is untouched, and the long
standing tail looks settled, not sunk or floating. In
**s08**, Suzuki's free arm swings while the map arm stays
steady. In **s10**, the short partial step (it is only about
three-quarters of one step before the cut — that is the
staging, unchanged) should read as a step, and the 4.4
seconds of standing before it, ~2 px higher, should be
acceptable to you. In **s11**, the walk-in arms should be
exactly the ones already there.

**Your verdict options:** adopt as the film's new baseline /
adopt with the arm-swing size retuned (name the size) /
reject — the film keeps its hand-staged walking, and the
module stays available for future work.

### Honest limits of this sheet

The frame counts above are exact — they come from a census
tool that re-checks itself against the film's code, and the
checking protocol will verify the change stayed inside them.
But no measurement in this pack says the new motion *looks
better*; there is deliberately no image-quality score used as
a pass mark, because whole-body shifts of a few pixels score
moderately no matter how they look. The arithmetic is proven
(planted foot stationary, legs in anti-phase, arms in
counter-phase, bob dipping at double support). Whether it
reads as better walking is an eye judgement — yours, on the
sheets, at wave close.
