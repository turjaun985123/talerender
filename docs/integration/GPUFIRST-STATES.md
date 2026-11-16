# GPUFIRST-STATES — Phase 0 STATES seat: real-film-state parity spread

Seat: STATES (Batch 1, GPU-first programme). Repo HEAD at authoring:
`bc12a5c` (verified). Code: `src/gpupaint_states.rs` (feature
`gpupaint`, default off). Entry point as chartered:
`gpupaint::state_spread() -> Vec<(String, FullLook, FullState)>` —
`src/gpupaint.rs::state_spread()` delegates to
`crate::gpupaint_states::state_spread()`. CPU
`toonfull::paint_toon_full` output is the reference for every
measurement. This seat does not commit; the coordinator integrates.

## 1. The spread — 75 states

Every entry is a REAL state: film-staged (values copied from
`film.rs` staging / the look registry) or reachable through the
scene-format loader (`scenefile.rs` character node: free
`pattern`/`hat`/`neck` strings, `extra` list, `kid`, free f32
state fields). No random fuzzing anywhere. Names are stable and
class-prefixed; `state_group()` / `rollup_class()` in the code
compute the roll-ups used in §5.

### 1.1 Moods — `mood-*` (19) — roll-up: mood-features

All 19 moods at the neutral standing state (`FullState::default()`
with only `mood` set) on the base look (scene-format defaults,
`scenefile.rs` character node: body `#E8734A`, head `#F7F7FA`,
plain, no hat/neck/extras): `mood-neutral`, `mood-smug`,
`mood-happy`, `mood-laugh`, `mood-shock`, `mood-worried`,
`mood-angry`, `mood-sad`, `mood-sly`, `mood-proud`, `mood-tired`,
`mood-dizzy`, `mood-determined`, `mood-embarrassed`, `mood-awe`,
`mood-suspicious`, `mood-relieved`, `mood-puzzled`,
`mood-deadpan`. (Mood spellings are `Mood::from_str`'s.)

### 1.2 Film looks — `film-*` (6) — roll-up: film-looks

The look registry's six distinct `FullLook`s — `film.rs`
`look_registry()` at **film.rs:598**, built by `fl()` at
**film.rs:572** — field values copied exactly:

| entry | registry lines | look (beyond `fl()` defaults: head `#F7F7FA`, plain, no extras) | staged state |
|---|---|---|---|
| `film-onoda-jg-s01` | film.rs:603-607 (idx 0/10, hero) | body `#5C6644`, cap `#4C5638`, **shade `Some(#878E6E)`** | s01: Determined, armR (16,-6), armL (10,6), lean 1, headTurn −0.45 (track start) |
| `film-onoda-sh-s02-walk` | film.rs:608 (idx 5) | body `#5C6644`, cap `#4C5638`, shade **None** (only difference from jg) | s02: Determined, walk, phase π/2, bob 0 |
| `film-comp-a-celebrate` | film.rs:609 (idx 1/6) | body `#68744E`, cap `#525C3C` | `pose_celebrate` (film.rs:1465): Laugh, arms (150,14)/(150,14), bob −8 |
| `film-comp-b-celebrate` | film.rs:610-615 (idx 2/7) | body `#59613F`, cap `#59613F`, **neck scarf `#8A7A52`** | s04: celebrate overridden to Happy |
| `film-suzuki-listen-worried` | film.rs:616-620 (idx 3) | body `#C96A42`, cap `#E8B83A`, **glasses** | s08b: `pose_listen` (film.rs:1479) + Worried (sweat fires via mood) |
| `film-tani-explain` | film.rs:621-631 (idx 4/8/9) | body `#4A4560`, bowler `#2E2A3A`, **vest `#33304A`, tie `#7A2E3A`, glasses, mustache `#C9C9D2`** | s03: `pose_explain` (film.rs:1459), mood → Determined, armR (68,−42), look [0.3, 0] |

### 1.3 Limbs — `legs-*` (8) — roll-up: limbs

On the shOnoda look with s02's staged arms (16,−6)/(10,6),
Determined:

- `legs-walk-p0/p1/p2/p3` — Walk at phase 0, π/2, π, 3π/2; bob
  per the film's `walk_bob` law (amp 5: 5, 0, 5, 0).
- `legs-run-p0/p1` — Run at phase 0, π. **Not film-staged**:
  scene-format reachable (`legs: "run"`); recorded so the
  painter's run branch (amp 1.0, lift 70) is not silently
  unmeasured.
- `legs-sit-film` — s06's sit exactly: `pose_mourn`
  (film.rs:1471) + Sit + `sit_drop = (0.34*1024/(1.75/0.96))`
  = **190.99** (film.rs:1840 expression, evaluated in code) +
  s06 arm overrides (16,−6)/(8,5), on the jgOnoda look.
- `legs-jump` — Jump, Happy. Scene-format reachable; the film
  never stages it.

### 1.4 Face dynamics — `face-*` (8) — roll-up: mood-features

- `face-talk-025/050/075/100` — talk = 0.25/0.5/0.75/1.0 on
  Determined (the film's talking configuration: Tani talks
  while Determined in s03/s11 via `film::talk`).
- `face-blink-050` — blink 0.5: the painter's partial branch
  (≤ 0.6, pupil height ×(1−blink·0.6)); scene-format reachable.
  `face-blink-100` — blink 1.0: the film's blink value
  (`blink_amount`/`blink_at` are 0/1) and the painter's
  closed-eye stroke branch (> 0.6).
- `face-look-r` — look = [1, 0]; `face-look-lu` — look =
  [−1, 0.5]. Task-specified extremes; the painter scales look
  by 9/8 px unclamped, and the scene format takes the pair
  verbatim.

### 1.5 Sweat / tears — `fx-*` (5) — roll-up: mood-features

**Synthetic-but-reachable — stated plainly:** the film path
never assigns `FullState.sweat`/`tears` (no staging site sets
them), which is exactly why they need explicit coverage: the
painter fires sweat on `sweat || mood == Worried` and tears on
`tears || mood == Sad` (toonfull.rs), and the `state_key`
precedent (film.rs `state_key` + regression tests,
`state_key_distinguishes_sweat_only_difference` /
`_tears_only_difference`) proved sweat-only/tears-only
differences are real paints a key once collapsed. Separate
entries, per that precedent:

- `fx-sweat-only` (sweat=true, Neutral), `fx-tears-only`
  (tears=true, Neutral), `fx-sweat-tears` (both, Neutral),
  `fx-worried-sweat-via-mood` (Worried, sweat=false),
  `fx-sad-tears-via-mood` (Sad, tears=false). A unit test
  asserts the first three paint differently from neutral and
  from each other.

### 1.6 Held plates — `plate-*` (3) — roll-up: plates

Plates generated by `scene::paint_parcel(label)` (**scene.rs:
2550**): 256×256 px = 128×128 stage units at res 2, centred on
the hand (`x0 = y0 = −64`, the centred-hold framing the film's
coconut plate approximates — `film/gen_assets.py`: 200×200
units at origin (−100,−108)). `label=true` is the
label-bearing variant (label paper + stamp rects — glyph text
proper enters production plates via gen_assets.py; the rects
are what this painter draws).

- `plate-holdr-parcel-label` — hold_r, label=true,
  `pose_explain` arms, Happy.
- `plate-holdr-parcel-plain` — hold_r, label=false, same pose.
- `plate-holdl-parcel` — hold_l, label=false, s05b's pre-drop
  leaflet-hold state (Suspicious, armL (58,−24), look
  [0.1, 0.3]) on the jgOnoda look.

### 1.7 Anchor — `anchor-*` (2) — roll-up: anchor

B-1 states transcribed from `toonfull.rs`
`canvas_anchor_tests` (finn look, toonfull.rs:1690 — body
`#DE7A35`, plain, hatless):

- `anchor-lying-lean82` — Dizzy, armL (150,20), armR (138,−30),
  headTilt 4, **lean 82** (toonfull.rs:1726).
  Measured anchor: **(−317.56, −14.16)** — strong left pull +
  slight up, as the B-1 test family predicts.
- `anchor-high-sit` — Sit, **sit_drop 900** (toonfull.rs:1751).
  Measured anchor: **(0.00, +187.70)** — pure downward push.
  (Both measured by the `anchor_entries_actually_engage` test,
  which also asserts both ≠ (0,0).)
- Control: `mood-neutral`'s anchor is exactly (0,0) — asserted.

### 1.8 Patterns / hats / neck / extras — roll-up: patterns-and-looks (18)

Isolated one-feature entries on the base look, Neutral
(vest/tie/scarf/cap/glasses/mustache are additionally covered
in situ in §1.2):

- Patterns (3): `pattern-stripes` (patCol None → painter's
  shade-of-body branch), `pattern-apron` (patCol None →
  default `#F4EFE4`), `pattern-vest` (patCol `#33304A`, Tani's).
- Hats (6): `hat-cap` (hero green `#4C5638`), `hat-tophat`,
  `hat-bun`, `hat-bow`, `hat-beanie`, `hat-mailcap` — the last
  five with hatCol None, exercising the painter's per-hat
  default colours.
- Neck (3): `neck-tie`, `neck-scarf` (neckCol None → default
  `#D8343F`), `neck-collar` (painter branch only; neckCol is
  ignored for collar by design, toonfull.rs).
- Extras (6): `extra-glasses`, `extra-mustache` (colour None →
  default `#3A2A22`), `extra-blush`, `extra-freckles`,
  `extra-pencil`, `extra-monocle` (the last four are the
  CR-B3-06 ports, reachable via the scene-format `extra` list).

### 1.9 Transforms / proportions — `xf-*` (6) — roll-up: transforms

- `xf-kid` — kid=true (painter `by += 190`), Happy.
- `xf-squash-050` — squash 0.5, Happy. Task-specified extreme;
  scene-format reachable (free f32). Film-staged squash is
  only s06's −0.04 settle — the gap is the point of the entry.
- `xf-headtilt-neg5` — headTilt −5 (`pose_listen`, film.rs:1479).
- `xf-headtilt-pos6-think` — `pose_think` (film.rs:1486):
  headTilt 6, headTurn 0.3, look [0.4, −0.3].
- `xf-headturn-pos05` — headTurn +0.5 (s04 track start, the
  film's positive extreme; −0.45 is in `film-onoda-jg-s01`).
- `xf-campitch-s01` — camPitch **0.28747** ≠ 0: the S01
  perspective value staged in `scenes/character-v2.scene.json`
  (film t=1.50; produced by film.rs `finish_chars` /
  `cam_pitch_of`), Determined hero state.

## 2. Metrics (canonical — coordinator ruling, aligned with BATT + AUDIT)

Implemented in `gpupaint_states::compare_paints` and used by
both the dry run (§4) and the GPU harness tables.

- **Headline per state:** byte-identical Y/N · differing-px
  count (pixels differing in ANY channel) · max per-channel
  |Δ| reported as (ΔR, ΔG, ΔB, ΔA).
- **RGBA PSNR** = `10·log10(255² / MSE)`, MSE over all
  4·w·h channel samples. Labelled "RGBA PSNR".
- **Alpha PSNR** = `10·log10(255² / MSE_alpha)`, MSE over the
  alpha channel only. Labelled "alpha PSNR". (BATT's headline;
  the M4 battery's 47.9 dB was alpha PSNR.) **Both PSNRs are
  reported for every state** — an unlabelled "PSNR" is not
  comparable across seats.
- **Localization** of differing pixels, by Chebyshev distance
  d to the boundary set **B = { p : 0 < A_cpu(p) < 255 }** —
  the fractional-α pixels of the CPU reference ONLY (the
  boundary is not dilated before measuring):
  **EDGE** d ≤ 1 · **FRINGE** d = 2 · **INTERIOR** d > 2.
  EDGE+FRINGE+INTERIOR = differing count (debug-asserted in
  code; asserted by the AUDIT checker). Specified consequence:
  ink inside an opaque region (e.g. mouth interior) has no
  α-boundary nearby and classifies INTERIOR. A hand-checkable
  fixture test (`compare_localization_matches_the_ruling`)
  pins the definition: fractional px at x=4 of a 9×1 strip,
  differences at x=2..6 → EDGE 3 / FRINGE 2 / INTERIOR 0; and
  with no fractional pixel anywhere, a differing pixel is
  INTERIOR.
- **Canvas caveat (from BATT §warning):** these definitions
  need a real alpha channel. The character canvas
  (2560×2816 RGBA from `paint_toon_full`) has one; the
  committed M4 battery cards are RGB-flattened, where alpha
  PSNR is trivially inf and localization vacuous — do not
  carry battery-card alpha columns into states tables.

## 3. Commands

Record the CPU reference cards + display lists (TRACE harness):

```
cargo run --release --features gpupaint -- --gpu-paint record-states ~/workspace/talerender-evidence/gpufirst-phase0/states-cpu
```

CPU-vs-CPU dry-run validation (§4):

```
cargo test --release --features gpupaint dry_run -- --ignored --nocapture
```

Fast spread/metric tests (no painting of the full spread
except the fx paint test):

```
cargo test --release --features gpupaint gpupaint_states -- --nocapture
```

Real GPU run (Batch 2/3 executor — the parity table's GPU
column lands when the replay executor is integrated; the
executor's real `replay` landed during this seat's authoring
and, at this delivery's last check, coexisted with the
integration stub of the same name in `src/gpupaint.rs`
(E0428 — integration-seat fix). The independent checker is
the AUDIT seat's tool, cross-validated in GPUFIRST-AUDIT.md):

```
# per state, once the executor lands:
talerender --gpu-paint replay <state>.json <state>-gpu.png
# then compare with compare_paints / the AUDIT checker and
# fill the §5 table (identical Y/N, diff px, max Δ per channel,
# RGBA PSNR, alpha PSNR, EDGE/FRINGE/INTERIOR).
```

## 4. Dry-run validation (CPU vs CPU) — PASS, 75/75

`dry_run_cpu_vs_cpu` paints all 75 states twice through
`paint_toon_full` and requires byte identity on every entry
(100% identical expected — the painter is a pure function of
(look, state)). **Result: 75/75 byte-identical** — every row:
identical = true, diff px 0, max Δ (0,0,0,0), RGBA PSNR inf,
alpha PSNR inf, EDGE/FRINGE/INTERIOR 0/0/0. Full CSV:
`~/workspace/talerender-evidence/gpufirst-phase0/dry-run-cpu-vs-cpu.csv`;
logs beside it (`fast-tests.log`, `dry-run.log`). Fast tests:
4/4 pass (spread shape, anchor engagement, sweat/tears paint
distinctness, localization fixture).

Validation-tree note (honest provenance): at validation time
the real tree did not compile for reasons outside this seat —
the executor seat's `replay` existed twice in `src/gpupaint.rs`
(the integration stub + the landed real function, E0428). The
runs above were executed in a scratch copy of the tree
(identical `gpupaint_states.rs`) with ONLY the duplicate stub
removed in the copy; no STATES code differs between copy and
tree. The duplicate-definition fix in the real tree is an
integration-seat item.

## 5. Per-state parity table (GPU vs CPU)

Status: **spread + dry run delivered; GPU column awaits the
Batch 2/3 executor.** Table schema (one row per state, in
spread order), to be filled by the real run:

| state | identical | diff px | max ΔR | max ΔG | max ΔB | max ΔA | RGBA PSNR (dB) | alpha PSNR (dB) | EDGE | FRINGE | INTERIOR |
|---|---|---|---|---|---|---|---|---|---|---|---|
| (75 rows — see `state_spread()` order) | | | | | | | | | | | |

Per-class roll-ups aggregate by `rollup_class()`:
mood-features (32), film-looks (6), limbs (8), plates (3),
anchor (2), patterns-and-looks (18), transforms (6).

## 6. Worst-5 hypotheses (for the real run, pre-registered)

Predicted hardest states, with the mechanism to check first —
written BEFORE any GPU number exists, so the real table can
confirm or refute them:

1. `anchor-lying-lean82` — the whole figure rides the lean
   rotation + a large canvas re-anchor translation; any
   transform-order or anchor mismatch moves EVERYTHING, and
   the raised-arm envelope grazes the canvas edge.
2. `plate-holdr-parcel-label` — bilinear plate compositing
   inverse-mapped through lean; label/stamp rect edges are
   the highest-frequency content in the spread and the first
   place resampling differences localize (expect EDGE-heavy).
3. `pattern-stripes` — masked fills (`fill_polygon_masked`:
   per-pixel mask AND) have no direct GPU analogue; a
   stencil/clip approximation will show at stripe/bean
   boundaries.
4. `fx-sweat-only` / `fx-tears-only` — tiny isolated features
   (one drop / two 7×14 ellipses): a class-level miss (e.g.
   the GPU face path dropping the OR-branch) reads as a small
   diff-px count that PSNR will flatter — judge these by
   diff px + max Δ, not PSNR.
5. `face-talk-100` / `mood-laugh` — the laugh mouth's masked
   tongue + the open-mouth ellipse/stroke pair are the most
   branch-dense face paints; tongue mask edges are INTERIOR
   under the α-boundary definition, so read max Δ there, not
   the split.

## 7. Notes / gaps

- `FullLook`/`FullState` have no `Clone` derive in
  toonfull.rs; the spread builds every value fresh via
  constructor helpers (no cloning needed).
- Run and Jump are the only non-film-staged leg branches;
  both are scene-format reachable and are flagged as such
  wherever they appear (§1.3).
- The TRACE harness stub this seat replaced said the spread
  "is an empty vec until that seat lands" — it has landed;
  `record-states` now records 75 states.
