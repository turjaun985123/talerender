# LOCOMOTION-AB-W57 — Locomotion adoption A/B sign-off pack (Wave 2, Batch 6, seat W57)

**Seat:** TaleRender Wave 2, W57 (locomotion adoption A/B sign-off pack).
**Date:** 2026-11-14. **Repo HEAD at writing:** `aa92cee`.
**Status:** pack only — nothing adopted, nothing wired, no pixels
changed by this seat. **Deliverables:** `tools/film/locomotion_census.py`
(new) + this note. Nothing else written, nothing committed.

**Purpose.** W24 delivered `src/locomotion.rs` (gait laws as tests,
9/9) and flagged that adoption changes shipped pixels two ways —
bob timing and a new arm swing — so it needs an A/B sign-off
before the lead wires it into film staging. This pack is that
sign-off's evidence base: a frame-exact census of what adoption
would touch (§2), the argument for why each difference is the
correct motion (§4), the A/B protocol (§5), the wiring parameters
(§6), the open decisions (§7), and the checklists split by owner
(§8). **It does not claim adoption is safe or that the result
looks good** — it claims the change surface is exactly the frames
enumerated here, and that the motion arithmetic is law-tested.
Walking quality is a user-eye call in this project.

---

## 1. Step 0 — the module is not registered yet

Confirmed at HEAD `aa92cee`, as the Batch-5 matrix (W49) reported:

- `grep -n "locomotion" src/main.rs` → **no match**. The mod list
  (main.rs:18-36) is: ab1, anim, atlas, battery, bufferpool,
  charanim, composer, dof, errors, gpucore, math, paintbudget,
  rendergraph, runlog, scene, scenefile, telemetry, film,
  toonfull. No `mod locomotion;` (cameramoves is likewise
  unregistered — W25's seat, noted for the record only).
- `grep -rn "mod locomotion|locomotion::" src/` → zero hits
  anywhere. Nothing calls the module.

Consequence: `src/locomotion.rs` is currently dead code — it has
only ever been compiled by W24's standalone `rustc --test` run,
and its 9 tests do **not** run under `cargo test` / CI. Step 0 of
the adoption is therefore a one-line registration
(`mod locomotion;` in main.rs, lead's file) that by itself
changes zero pixels (no call sites) and makes the test suite
cover the module in CI from then on. It can land and be verified
independently before any staging is touched (§5 step 0).

---

## 2. The census — verbatim output

`python3 tools/film/locomotion_census.py` at HEAD `aa92cee`,
exit 0. The tool evaluates each site's driver (cam_k / seg, with
film.rs:121/139/144's own formulas) at every frame's staged
instant (manifest frame rule: frame n at t = n/30, lt = t −
startS), classifies each character-frame, and re-verifies its
site table against film.rs's actual call sites before printing
(a drifted table exits 2 instead of miscounting).

```
LOCOMOTION ADOPTION CENSUS — Onoda film, film lane (src/film.rs staging)
sources: scenes/film/film.json (segment table + frame rule) · film/onoda.json (anchors) ·
         src/film.rs site table transcribed at aa92cee · module src/locomotion.rs (W24)
VERIFY: manifest tiles [0, 1811) exactly — OK · site table vs film.rs walk_phase/walk_bob call sites — OK (7 sites, strides [0.58, 0.6, 0.6, 0.6, 0.6, 0.62, 0.62], bob amps [3.0, 4.0, 4.0, 4.0, 5.0, 5.0, 5.0])
classes: HEAD-STATIC driver=0 · ADVANCING 0<driver<1, legs Walk · ADVANCING-STAND (s02) phase advancing, legs Stand · TAIL-STATIC driver=1, legs Stand
changes: BOB = bob waveform replaced (every gait-staged frame) · SWING = counter-phase arm swing added (ADVANCING frames, free arms only)

SHOT s02 — segment frames 178-391 (214), t 5.907-13.061
  SITE film.rs:1432-1433 (fn s02:1428)
    driver cam_k(lt, d, 0.03) · dist k * 6.8 (travelled, unclamped) · stride 0.62 · bobAmp 5.0
    Onoda (SH_ONODA): gait-staged 214 frames
      HEAD-STATIC      178-183 (6)
      ADVANCING        184-353 (170)
      ADVANCING-STAND  354-385 (32)
      TAIL-STATIC      386-391 (6)
      changes: BOB on all 214 gait-staged frames; SWING (L+R) on 170 advancing frames
      arms: authored pair == GaitParams.arm_base default
    Companion A (SH_COMPA): gait-staged 214 frames
      HEAD-STATIC      178-183 (6)
      ADVANCING        184-353 (170)
      ADVANCING-STAND  354-385 (32)
      TAIL-STATIC      386-391 (6)
      changes: BOB on all 214 gait-staged frames; SWING (L+R) on 170 advancing frames
      arms: arms unset in staging -> FullState::default (8,6)/(8,6); module default base is (10,6)/(16,-6) — BASE MISMATCH, doc D2
    Companion B (SH_COMPB): gait-staged 214 frames
      HEAD-STATIC      178-183 (6)
      ADVANCING        184-353 (170)
      ADVANCING-STAND  354-385 (32)
      TAIL-STATIC      386-391 (6)
      changes: BOB on all 214 gait-staged frames; SWING (L+R) on 170 advancing frames
      arms: arms unset in staging -> FullState::default (8,6)/(8,6); module default base is (10,6)/(16,-6) — BASE MISMATCH, doc D2
  SHOT TOTAL: walking (ADVANCING, any character) 170 frames 184-353 · gait-staged 214 · static-gait (bob-only) 12 · advancing-stand (bob-only) 32

SHOT s06 — segment frames 784-911 (128), t 26.129-30.390
  SITE film.rs:1631-1641 (mk_walk closure, fn s06:1613)
    driver seg(lt, 0.1, 3.55) · dist wk * 5.96 (COMPA) / wk * 6.1 (COMPB) · stride 0.62 · bobAmp 5.0
    Companion A (COMPA): gait-staged 128 frames
      HEAD-STATIC      784-786 (3)
      ADVANCING        787-890 (104)
      TAIL-STATIC      891-911 (21)
      changes: BOB on all 128 gait-staged frames; SWING (L+R) on 104 advancing frames
      arms: arms unset in mk_walk -> FullState::default (8,6)/(8,6) — BASE MISMATCH, doc D2
    Companion B (COMPB): gait-staged 128 frames
      HEAD-STATIC      784-786 (3)
      ADVANCING        787-890 (104)
      TAIL-STATIC      891-911 (21)
      changes: BOB on all 128 gait-staged frames; SWING (L+R) on 104 advancing frames
      arms: arms unset in mk_walk -> FullState::default (8,6)/(8,6) — BASE MISMATCH, doc D2
  SHOT TOTAL: walking (ADVANCING, any character) 104 frames 787-890 · gait-staged 128 · static-gait (bob-only) 24

SHOT s07 — segment frames 912-1026 (115), t 30.390-34.209
  SITE film.rs:1707-1713 (fn s07:1691)
    driver seg(lt, 0.2, 1.85) · dist wk * 2.0 · stride 0.6 · bobAmp 4.0
    Onoda (ONODA): gait-staged 115 frames
      HEAD-STATIC      912-917 (6)
      ADVANCING        918-967 (50)
      TAIL-STATIC      968-1026 (59)
      changes: BOB on all 115 gait-staged frames; SWING (L) on 50 advancing frames
      arms: arm_l static (12,6), free
  SHOT TOTAL: walking (ADVANCING, any character) 50 frames 918-967 · gait-staged 115 · static-gait (bob-only) 65

SHOT s08 — segment frames 1027-1210 (184), t 34.209-40.360
  SITE film.rs:1743-1749 (fn s08:1737, Suzuki)
    driver seg(lt, 0.1, 3.3) · dist wk_s * 3.28 · stride 0.6 · bobAmp 5.0
    Suzuki (SUZUKI): gait-staged 184 frames
      HEAD-STATIC      1027-1029 (3)
      ADVANCING        1030-1125 (96)
      TAIL-STATIC      1126-1210 (85)
      changes: BOB on all 184 gait-staged frames; SWING (R) on 96 advancing frames
      arms: arm_r static (12,8), free
  SITE film.rs:1760-1766 (fn s08:1737, Onoda)
    driver seg(lt, 0.35, 1.95) · dist wk_o * 2.25 · stride 0.6 · bobAmp 4.0
    Onoda (ONODA): gait-staged 184 frames
      HEAD-STATIC      1027-1036 (10)
      ADVANCING        1037-1084 (48)
      TAIL-STATIC      1085-1210 (126)
      changes: BOB on all 184 gait-staged frames; SWING (L+R) on 48 advancing frames
      arms: authored pair == GaitParams.arm_base default
  SHOT TOTAL: walking (ADVANCING, any character) 96 frames 1030-1125 · gait-staged 184 · static-gait (bob-only) 88

SHOT s10 — segment frames 1361-1519 (159), t 45.365-50.648
  SITE film.rs:1848-1853 (fn s10:1845)
    driver seg(t, 49.72, 50.92)  [film time] · dist wk * 0.55 · stride 0.58 · bobAmp 3.0
    Tani (BK_TANI): gait-staged 159 frames
      HEAD-STATIC      1361-1491 (131)
      ADVANCING        1492-1519 (28)
      changes: BOB on all 159 gait-staged frames; SWING (L) on 28 advancing frames
      arms: arm_l static (14,8), free; NOTE: driver runs on film time t and its window (49.72-50.92) outlasts the segment (ends 50.648) — wk never reaches 1 inside the shot (max 0.761 at the last frame)
  SHOT TOTAL: walking (ADVANCING, any character) 28 frames 1492-1519 · gait-staged 159 · static-gait (bob-only) 131

SHOT s11 — segment frames 1520-1665 (146), t 50.648-55.520
  SITE film.rs:1914-1920 (fn s11:1897, walk-in branch)
    driver seg(lt, 0.1, 0.882) · dist wk * 2.5 · stride 0.6 · bobAmp 4.0
    Onoda (ONODA, walk-in): gait-staged 88 frames
      HEAD-STATIC      1520-1522 (3)
      ADVANCING        1523-1545 (23)
      TAIL-STATIC      1546-1607 (62)
      changes: BOB on all 88 gait-staged frames; no arm swing — D15 source walk-in arms (10,5) — W24: gait swing OFF unless the user re-stages
      arms: no arm swing at this site; bob change only. Site exists only while t < drop_t = 53.570 (film.rs:1914); from frame 1608 the rifle-drop branch stages ONODA2 with no gait channels at all
  SHOT TOTAL: walking (ADVANCING, any character) 23 frames 1523-1545 · gait-staged 88 · static-gait (bob-only) 65

FILM TOTALS (of 1811 frames)
  walking frames (ADVANCING, >= 1 character): 471 (26.0%)
  character-frames advancing: 963
  frames with arm-swing motion added (ADVANCING, >= 1 free arm): 448 (24.7%) · character-frames swinging: 940
  gait-staged frames (bob value changes, incl. static): 888 (49.0%) · of those: advancing-stand 32 (s02 slide tail) · static-gait (driver at an endpoint) 385
  shots with walking: s02, s06, s07, s08, s10, s11 · shots untouched by adoption: s01, s03, s04, s05a, s05b, s08b, s09, s12

CROSS-CHECK — scene-file lane (scenes/film/*.scene.json carry an instant state, not a range):
  s02: scene time 9.484 · states with legs=walk: 3 · film-lane walking frames: 170
  s06: scene time 3.5999999046325684 · states with legs=walk: 0 · film-lane walking frames: 104
  s07: scene time 0 · states with legs=walk: 1 · film-lane walking frames: 50
  s08: scene time 37.208999999999996 · states with legs=walk: 1 · film-lane walking frames: 96
  s10: scene time 48.6 · states with legs=walk: 0 · film-lane walking frames: 28
  s11: scene time 0 · states with legs=walk: 1 · film-lane walking frames: 23
```

**Headline numbers.** 7 walk sites, 9 walking characters, 6
shots. **471 frames (26.0%)** have at least one character actually
walking; **963 character-frames** advance. **888 frames (49.0%)**
are gait-staged and take the bob change. Arm swing lands on
**448 frames / 940 character-frames**. The remaining **923
frames** — s01, s03, s04, s05a, s05b, s08b, s09, s12 entire, plus
s11's rifle-drop branch (1608-1665) — are not touched by the
adoption at all and serve as the protocol's control set (§5).

---

## 3. What the census shows — five things the headline hides

1. **The bob change is mostly a *standing* change.** Only 471 of
   the 888 gait-staged frames are walking frames. The other 417
   (385 static-gait + s02's 32 advancing-stand) are frames where
   the driver sits at an endpoint — the character has not started
   or has already arrived — but staging still writes the gait
   bob and the painter still consumes it (toonfull.rs:324 adds
   `st.bob` unconditionally). Adoption replaces the value on
   every one of them: a constant body-Y re-level for the rest of
   the shot. Worked examples (module drop at phase 0 = 0.376,
   §4): s02/s06 heads — film bob 5.00 px → module 1.88 px, a
   **3.1 px** lift on the opening frames; **s10's 131 head-static
   frames — 3.00 px → 1.13 px, a ~1.9 px lift sustained for
   4.4 seconds** of standing bookshop footage (the most visible
   static change in the film); frozen tails re-level by ~1 px
   (e.g. s08 Onoda's 126-frame tail: film 1.17 px → module
   0.18 px at the frozen phase). None of this is motion; all of
   it is in the A/B whether anyone looks for it or not — so the
   protocol reviews it explicitly (D1).
2. **s02's slide tail is its own class (ADVANCING-STAND, 354-385).**
   Source truth, staged deliberately (film.rs s02 comment, D6):
   `travelled = k * 6.8` is unclamped, legs switch to Stand at
   k > 0.97, and the group slides the last 0.204 m with planted
   feet. Phase still advances there but the painter ignores
   phase for Stand legs; only the bob value changes. Arm
   behaviour on these frames is a decision (D3), not a default.
3. **s10's walk is truncated by the cut.** The driver runs on
   film time with window 49.72-50.92, but the segment ends at
   50.648: wk peaks at 0.761 on the last frame, Tani covers
   ~0.42 m ≈ 0.72 of a step, and there is no TAIL-STATIC class
   in s10 at all. Adoption changes nothing about the truncation —
   but the user should know the "walk" they are judging in s10
   is three-quarters of one step.
4. **s11's site is branch-bounded.** The walk-in staging exists
   only while t < drop_t (53.570); from frame 1608 the
   rifle-drop branch stages a different character state with no
   gait channels. 58 of s11's frames are therefore an in-shot
   control range (1608-1665 must stay byte-identical).
5. **Four of nine walkers have an arm-base mismatch (D2).**
   s02's two companions and s06's two companions never set arms
   in staging, so they stand at `FullState::default()` =
   (8,6)/(8,6) (toonfull.rs Default impl). The module's
   `arm_base` default is ((10,6),(16,-6)) — Onoda's authored
   pair. Wiring `eval` with default params at the companion
   sites would move their resting arms too (arm_r base
   (8,6) → (16,-6): +8° shoulder, −12° elbow) on top of adding
   swing. The census counts swing for the companions on the
   assumption that the lead passes per-site bases equal to
   today's pairs (§6); if the lead instead takes the module
   defaults, the companions' ADVANCING *and* static frames all
   change arm pose as well — a posing change, not a gait change.

**Cross-check (census §2, bottom).** The scene-file lane's
instant states agree with the film-lane windows at every shot —
including the two zeros, which the windows explain: s06's scene
instant (lt 3.60) sits just past its window end (3.55), so both
companions stand and Onoda sits, exactly as the file records;
s10's instant (t 48.6) precedes its window (49.72). s08's single
`walk` at lt 3.0 is Suzuki (wk_s = 0.935, still advancing) while
Onoda (wk_o = 1) already stands — the file even carries the two
legs laws as channel formulas matching film.rs. No shot
disagrees with the census.

---

## 4. The expected-difference argument — why each change is the correct motion

### Change A — bob timing (all 888 gait-staged frames)

The two waveforms, stated exactly:

- **Film today:** `walk_bob(phase, amp) = amp * (1 - |sin phase|)`
  (film.rs:181-183), phase = the painter's phase. It is
  **maximal (amp) at phase 0** — the passing pose, legs together
  under the hip (the painter's leg offset is `sin(phase) * amp`,
  toonfull.rs:358-364) — and **zero at phase π/2**, the widest
  leg split. The body therefore sits *lowest at mid-stance* and
  *highest at the split*, twice per cycle.
- **Module (L4):** bob = `bob_amp * drop`, with
  `drop = (min over stance legs |x|/A)^2` — the straight-leg
  geometry `hip height = sqrt(L^2 - x^2)` over the planted foot
  (locomotion.rs `support_drop`). Drop is **0 at each
  mid-stance** (support leg vertical, hip at full height) and
  peaks **twice per cycle inside the double-support windows**.
  Computed for the walk duty (0.62): drop = 0.376 at foot
  strike, peak **0.650** at cycle fraction 0.06 (the middle of
  the window), 0.000 at mid-stance.

So the adoption changes the bob three ways at once: **phase**
(the waveform shifts a quarter-step, W24 §4 — today the dip is
exactly where the support leg passes vertical, which is where a
walking body is *highest*), **amplitude** (module range
[0, 0.65·amp] vs film [0, amp] — at amp 5.0, 3.25 px vs 5.0 px of
travel), and **shape** (quadratic vs the |sin| cusp). The module
is the physically correct one for walking — the inverted
pendulum vaults *over* the planted leg — and it is asserted, not
asserted-about: `bob_two_minima_per_cycle_at_double_support`
(W24 §5) checks the period, the mid-stance zeros, and that both
maxima fall inside double support. This is also one of the exact
defects the sister project's audit proved arithmetically in a
hand-staged cycle (inverted bob, W24 §1): the film's current
bob is that defect, staged.

What adoption does **not** do to the legs: the painter still
draws its in-card sinusoid from `phase` alone (W24 §7) — L1's
linear stance sweep is ground truth the painter does not consume.
`painter_phase` exists precisely so the leg pose the audience
sees is unchanged by the adoption (module left mid-stance ↦
painter phase 0; test `painter_phase_origin`), and the phase law
itself is untouched (`phase_for_distance` ≡ film's walk_phase;
test `phase_law_matches_film_walk_phase`). If a leg pose differs
in the A/B, the wiring is wrong — it is not part of this change.

### Change B — arm swing (448 frames / 940 character-frames)

New motion: today the free arms are static pairs; nothing swings.
The module adds law **L3**: shoulder swing =
`−arm_amp · (same-side foot fore-aft fraction)` — the arm is
fully back when the same-side foot is fully forward, and vice
versa (locomotion.rs `arm_swing`), with the elbow echoing at
−0.35× (the elbow component is part of the change on free arms;
staging may suppress it and keep authored elbows — all free arms
here have static elbows, so the choice is visible and belongs to
the wiring, §5 step 2). Asserted by
`arm_counterphase_same_side_leg`: Pearson(arm, same-side leg) is
exactly −1 by construction (test bound < −0.99), and +0.836
against the opposite leg at duty 0.62 — correctly not ~1, since
the position waveform is not half-cycle antisymmetric at
duty ≠ 0.5. Counter-phase is the whole point: an arm swinging
*in-phase* with its own leg is the marching-robot tell, the
second defect the sister project's audit proved in hand staging
(W24 §1). Amplitude is the walk default `arm_amp` = 16° — a look
parameter, not a law; the user may retune it at sign-off without
re-opening the arithmetic.

Swing is **never** written over an authored arm. The exclusions
are staging facts, verified at HEAD: s07 arm_r (coconut
poseTrack beat + `hold_l` coconut from t ≥ 32.35), s08 Suzuki
arm_l ((38,-52) + paper-map in `hold_l` for the whole shot),
s10 arm_r (poseTrack beat + book in `hold_r` for the whole
shot), s11 both arms (D15 source walk-in arms — W24: gait swing
off unless the user re-stages; that makes it the user's
decision, D4).

### What must not change

Cuts, captions, camera, lights, sets, and every position driver
are untouched — the adoption rewrites only the `phase`/`bob`/
(free) `arm` channels at the seven sites, and phase stays
distance-driven, so s02's three marchers keep their shared-
`travelled` lockstep by construction. The other 923 frames must
come out byte-identical; that is a protocol gate (§5 step 3),
not a hope.

---

## 5. A/B protocol (for the lead)

All frame dirs use the house `frame_%05d.png` pattern. The
pre-adoption reference of record is the f021991 render on disk:
`~/workspace/film-final/frames4/` (all 1,811 frames).

0. **Register only.** Add `mod locomotion;` to main.rs. CI
   green; `cargo test` now runs locomotion's 9 tests in-tree.
   Expected pixel change: none. Spot-proof: default byte-probes
   (§5.3 command, no `--ranges`) against a fresh render —
   byte-identical; if not, something else moved — stop.
1. **Baseline proof at adoption HEAD.** HEAD has moved since
   f021991 (gpucore migration, errors step 2 — both landed as
   behaviour-preserving). Before adopting, render the film
   pre-adoption (render dispatch, film mode) into a scratch dir
   and prove the gait footage is still the f021991 footage:

   ```
   python3 tools/ci/byte_probes.py ~/workspace/film-final/frames4 <pre-render-dir> \
     --ranges 178-391,784-911,912-1026,1027-1210,1361-1519,1520-1607
   ```

   PASS (byte-identical) → frames4 stands as the A/B baseline.
   FAIL → resolve that drift first, or re-baseline the A/B on
   the pre-adoption render and record why in the adoption
   commit. Do not A/B across an unrelated drift.
2. **Wire the seven sites** per W24 §4 and the §6 table:
   per-site `step_length`/`bob_amp`, per-character `arm_base`,
   arms written **only** for census-free arms (and only on
   ADVANCING frames if D3 so decides); s11 arms not written at
   all. `phase_for_distance` wraps phase into [0, 2π) and
   `painter_phase` shifts its origin — the `phase` channel's
   representation changes even where its pose doesn't; step 4
   exists partly to catch any consumer of the raw value.
   Record D1/D2/D3 in the adoption commit message.
3. **Post-adoption render + probes** (same dispatch):

   ```
   # must DIFFER, finite PSNR — the adoption footprint:
   python3 tools/ci/byte_probes.py ~/workspace/film-final/frames4 <post-render-dir> \
     --ranges 178-391,784-911,912-1026,1027-1210,1361-1519,1520-1607 --psnr-fallback
   # must stay byte-IDENTICAL — the control set:
   python3 tools/ci/byte_probes.py ~/workspace/film-final/frames4 <post-render-dir> \
     --ranges 0-177,392-783,1211-1360,1608-1810
   ```

   Reading the numbers: bob differences are whole-card vertical
   shifts of ≤ 5 px plus arm strokes, so expect *moderate* PSNR
   on the footprint ranges — neither near-identity (that would
   mean the wiring didn't take) nor collapse. This pack sets
   **no PSNR pass bar**: the gates are confinement (controls
   byte-identical; differences only inside the footprint) and
   the user's eyes (§8). Investigate any footprint range whose
   PSNR is wildly out of family with the other five. Note the
   standing default probe ranges split across the classes —
   195-204 ⊂ s02 ADVANCING, 795-814 ⊂ s06 ADVANCING, 1195-1204
   ⊂ s08 TAIL-STATIC — so a default-probes run post-adoption
   must FAIL on all three; that failure is the expected
   signature (the third range proves the static re-level
   landed), to be triaged W37-style, not treated as a regression
   alarm.
4. **Telemetry gate.** The film-telemetry run must print
   `FILM TELEMETRY: CHECK OK` and exit 0 post-adoption
   (contract of `aa10b13`). Captions/cuts/continuity are
   untouched by construction; this is the proof, and it also
   covers cutout consumers of the re-represented phase channel.
5. **Determinism spot-check.** Render one advancing range twice
   (e.g. 918-967, s07's walk) from the adopted build;
   byte-identical. The module is pure (W24 §7) — this proves
   the wiring added no state.
6. **Contact sheets for the user** (`tools/film/make_sheet.py`,
   `--from`/`--to` in film seconds; it samples at step
   multiples inside the window). One pre (frames4) and one post
   sheet per walking shot. Step cycles run 5.5-18 frames
   (0.18-0.6 s), so:
   - s02 `--from 5.907 --to 13.061 --step 0.25`
   - s06 `--from 26.129 --to 30.390 --step 0.25`
   - s07 `--from 30.390 --to 34.209 --step 0.25`
   - s08 `--from 34.209 --to 40.360 --step 0.25`
   - s10 `--from 49.60 --to 50.648 --step 0.1` (28 advancing
     frames; less than one full step — the frames themselves
     are the review, the sheet is an index)
   - s11 `--from 50.648 --to 51.60 --step 0.1` (5.5
     frames/step; 0.1 s sampling ≈ 1.8 samples per step)
7. **Assemble the sign-off packet:** this census + the §5.1/§5.3
   probe tables + the telemetry line + the sheets → §8.

---

## 6. Per-site wiring parameters (for §5 step 2)

`duty_factor` is the walk default 0.62 at every site (no site
stages `Legs::Run`; the film's walks are all walks). `arm_amp`
is the walk default 16° everywhere unless the user retunes at
sign-off. `speed` is not an input in the film lane — phase stays
distance-driven via `phase_for_distance`, exactly as today.

| Site | step_length | bob_amp | arm_base (L, R) to pass | Arms written |
|---|---|---|---|---|
| s02 Onoda | 0.62 | 5.0 | ((10,6), (16,-6)) — module default | L+R |
| s02 Companions A/B | 0.62 | 5.0 | **((8,6), (8,6))** per D2 — NOT the module default | L+R |
| s06 Companions A/B | 0.62 | 5.0 | **((8,6), (8,6))** per D2 — NOT the module default | L+R |
| s07 Onoda | 0.6 | 4.0 | L base (12,6) | L only (R = coconut beat) |
| s08 Suzuki | 0.6 | 5.0 | R base (12,8) | R only (L = paper-map hold) |
| s08 Onoda | 0.6 | 4.0 | ((10,6), (16,-6)) — module default | L+R |
| s10 Tani | 0.58 | 3.0 | L base (14,8) | L only (R = book beat/hold) |
| s11 Onoda | 0.6 | 4.0 | — | none (D15 authored) |

For the single-arm sites, `GaitParams.arm_base` still takes a
pair; set the written side's base as tabled and write only that
side from the eval — the other side's base value is then inert.

---

## 7. Open decisions

- **D1 (lead) — static-frame policy.** Replace the
  (walk_phase, walk_bob) pair on **all** gait-staged frames (the
  census headline: 888 frames change; static characters
  re-level by the §3.1 amounts, constantly, for the rest of
  their shots) — or **gate** the module eval to advancing
  frames and keep film values at the endpoints (footprint
  shrinks to 503 frames; but the bob law then changes hands at
  every window edge, with a bounded step — up to ~3.1 px at
  amp 5.0 — exactly where the stride starts/stops, and W24 §7's
  doctrine is that starts/stops are staging-owned blends, never
  phase restarts). *Pack's recommendation:* full replacement —
  one law everywhere, no edge steps inside a shot, and the
  re-levels are small, constant, and sheet-reviewed. If gating
  is chosen instead, re-run the census with the gate before
  sign-off; this pack's counts assume full replacement.
- **D2 (lead) — companion arm bases.** Pass ((8,6), (8,6)) at
  the four companion sites (swing added, resting pose
  preserved — the census assumption) or the module defaults
  ((10,6), (16,-6)) (resting pose changes too).
  *Recommendation:* the sites' current pairs — adoption should
  change gait, not character posing.
- **D3 (lead) — arms on non-walking frames of walk sites.**
  Under full replacement the eval also yields arm values on
  static and advancing-stand frames (a constant offset while
  standing; swinging during s02's slide). *Recommendation:*
  write arms only on ADVANCING frames — arms are gait; a
  standing character keeps its staged arms. This makes the arm
  footprint exactly the censused 448 shot-frames / 940
  character-frames.
- **D4 (user) — s11's arms and the adoption itself.** W24 rules
  s11's D15 walk-in arms stay authored unless the user
  re-stages them; and the adoption as a whole is a deliberate
  re-baseline of the film's fidelity record, which only the
  user can accept (§8).

---

## 8. Sign-off checklists

### Lead — verifies the numbers

- [ ] `mod locomotion;` registered; CI green; locomotion's 9
      tests run under `cargo test` (standalone-only before).
- [ ] §5.1 baseline: byte-identical vs frames4 on the six gait
      ranges pre-adoption — or drift resolved / re-baseline
      recorded.
- [ ] Wiring matches §6: per-site step_length / bob_amp /
      arm_base; beat/hold/D15 arms untouched in staging (s07
      arm_r, s08 Suzuki arm_l, s10 arm_r, s11 both arms).
- [ ] §5.3 probes: footprint ranges DIFFER with finite PSNR;
      control ranges 0-177, 392-783, 1211-1360, 1608-1810
      byte-IDENTICAL.
- [ ] Telemetry: `FILM TELEMETRY: CHECK OK`, exit 0.
- [ ] Determinism: one advancing range rendered twice,
      byte-identical.
- [ ] D1/D2/D3 choices recorded in the adoption commit; if
      D1 = gated, a re-run census is attached to the packet.

### User — judges the motion (sheets pre/post, §5.6)

- [ ] **s02 march:** the bob reads as weight — body highest as
      the legs pass, settling lowest when both feet are down —
      not a bounce at the passing pose. All three marchers' arms
      swing *against* their same-side leg; no arm moves with its
      own leg. No visible pop where walking starts/stops
      (frames ~184, ~353, and the slide tail 354-385).
- [ ] **s06 companions:** swing and bob read natural in the fog
      mood; resting arm pose unchanged (D2) — no arm jump at
      frame 787; the arrival (891+) stands at a settled level.
- [ ] **s07:** the left arm swings freely while the right arm's
      coconut beat is untouched; the long standing tail
      (968-1026) — body level reads settled, not sunk or
      floating, against the pre-adoption sheet.
- [ ] **s08:** Suzuki's free right arm swings, the map arm is
      steady; Onoda's shorter walk nested inside Suzuki's
      longer one reads correctly; both long tails settle.
- [ ] **s10:** Tani's partial step (1492-1519) reads as a step,
      not a shuffle; the 4.4 s of standing book footage before
      it (131 frames at a ~1.9 px higher body level under full
      replacement) is acceptable — or D1 goes back to the lead.
- [ ] **s11:** walk-in arms are the source arms, unchanged
      (D15) — confirm keeping them; the bob-only change reads
      fine at the brisk ~5.5-frames/step cycle.
- [ ] **Verdict:** adopt as the film's new fidelity baseline
      (references re-struck through the wave's golden-pin
      process) / adopt with `arm_amp` retuned (state the value)
      / reject — the film keeps its hand-staged gait and the
      module stays available for the scene-file lane.

---

## 9. Honest limits

- **Film lane only.** The scene-file lane has no gait driver
  blocks today (W24 §4 — a character node carrying W24 §6's
  `gait` block is future format work). There is nothing to adopt
  there yet, and this pack says nothing about it.
- **Boundary precision.** Classes derive from the manifest's
  frame rule with its 3-decimal startS values — the same values
  the code's st0 literals carry (30.390, 45.365, 50.648) where
  they exist. If the driver's internal boundaries ever differ
  in the last decimal, a frame at a window edge could flip
  class; counts would move by at most ±1 frame per edge, ranges
  would not materially change.
- **Where, not how much.** The census predicts *where* pixels
  change, never their PSNR or their look. No A/B number exists
  until the lead runs §5. Moderate PSNR on the footprint is the
  expected signature of whole-card ≤ 5 px shifts, not a quality
  verdict in either direction.
- **No safety claim.** This pack does not say adoption is safe,
  correct as a look, or an improvement the channel should ship.
  It says: the module's gait arithmetic is law-tested (W24 §5),
  the change surface is exactly the 888 frames enumerated
  (§2), and the decision procedure is §5 + §8. The verdict is
  the user's.
