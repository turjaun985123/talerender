# GPUFIRST Batch 2 — Seat EDGE: legs-walk-p3 alpha diff

**Verdict: CONDITIONAL** (no code change). The 2-px alpha-only
difference is a coverage value sitting 1.4e-5 px from a rounding
boundary, where the CPU (Rust libm) and GPU (WGSL on lavapipe)
f32 evaluations of `fan_sd`'s acos/cos chain legitimately land
on opposite sides. Measured bound: **2 px, alpha-only,
ΔA = 16, ΔRGB = 0, zone EDGE** — details and evidence below.
Base: `gpufirst-phase0` @ f4b358d; branch `gpufirst-b2-edge`.

## The case (CI run #1, 43945477118)

| case | diff px | max Δ (RGBA) | RGBA PSNR | zone |
|---|---:|---|---:|---|
| legs-walk-p3 | 2 | 0/0/0/16 | 95.64 dB | EDGE |

The only alpha diff and the only EDGE diff in the run.

## Local reproduction (this VM, debug build, llvmpipe Mesa 25.2.8)

Recorded at HEAD (`--gpu-paint record-states`; the state was
also recorded standalone — identical list, 31 prims) and
replayed with the unmodified executor. The CI numbers reproduce
**exactly**:

| px | CPU RGBA | GPU RGBA |
|---|---|---|
| (1156, 2515) | 31, 28, 51, **239** | 31, 28, 51, **255** |
| (1403, 2515) | 31, 28, 51, **239** | 31, 28, 51, **255** |

2 differing px, Δ = 0/0/0/16, same as CI (release build, Mesa
23.2.1) — the diff is deterministic and environment-stable.

## Localisation

Both pixels lie in the feet of the walk-p3 figure, each covered
by exactly two prims, in order:

- prim 0 / prim 2 — `stroke_polyline`, the leg (flattened quad,
  21 pts, width 30, col (31,28,51,255)); terminal vertex
  (1142.6, 2512) / (1417.4, 2512), half-width 15;
- prim 1 / prim 3 — `fill_ellipse`, the foot (rx 40, ry 20,
  cy 2504).

The foot ellipse contributes **nothing** at these pixels: all
16 of its cov_area subsamples are outside (centre sd +0.693,
nearest subsample sd +0.171). Painting prim 0 alone on a blank
canvas with the real CPU painter yields alpha **239** at
(1156,2515) (likewise prim 2 at (1403,2515)) — the stroke is
the sole contributor, and the final alpha is exactly
`round(255 * cov)` for its coverage.

## Mechanism

CPU coverage at (1156,2515) is **15/16 = 0.9375** →
`blend_cov` alpha `round(255 * 15/16) = 239`. GPU coverage is
**16/16 = 1.0** → full blend, alpha 255. Exactly one of the
4×4 cov_area subsamples flips its inside test
(`scene.rs::cov_area0`: `dist(x+ox, y+oy) < 0.0`):

- flipped subsample: pixel centre + (+0.375, +0.375) =
  **(1156.875, 2515.875)** (prim 2's pixel: the mirrored
  (−0.375, +0.375) = (1403.125, 2515.875); the two feet are
  mirror images and every value below is identical);
- the min distance there comes from the **end cap fan**
  (`fan_sd`, last segment, t = 1): the flattened round cap at
  the foot end of the leg, fan centre (1142.6, 2512),
  axis (−0.03918562, 0.999232), r = 15, n = 9 chords;
- **CPU f32 `fan_sd` at that point = +1.4305115e-5**
  (bits 0x37700000; f64 evaluation: +1.3786e-5) — outside by
  fourteen micro-pixels. The plain disc term at the same point
  is −0.208 (well inside): it is the fan's chord cut
  (rb = 14.791604 vs |v| = 14.791617) that places the boundary
  within 1.4e-5 px of this subsample;
- the other 15 subsamples are ≤ −0.053 — none is within
  0.05 px of the boundary, so exactly one subsample can flip;
- ruled out: `hypot` vs `sqrt(x²+y²)` — at this point the CPU's
  f32 `hypot` and the WGSL `hyp2` formulation agree bit-for-bit
  (rl = 14.791617 both ways); the fan branch selection
  (t == 1.0) is not borderline (raw t ≈ 1.073, clamped);
  the centre distance (−0.446) and the |d0| < 1 supersample
  gate are not borderline.

The divergence is therefore inside `fan_sd`'s transcendental
chain — `phi = acos(cosang)`, `rb = r·cos_half / cos(phi −
phi_mid)` — as evaluated by the GPU (WGSL → lavapipe/LLVM)
versus Rust's libm `acosf`/`cosf`: the GPU's result lands
< 0 where the CPU's lands at +1.43e-5. This is the spot the
INV-A spec (§8) already flags as the highest-risk in the
shader ("fan_sd itself … its acos/cos are the flagged
highest-risk spot"). A subsample 1.4e-5 px from the true
boundary is a rounding-boundary case by construction: *any*
two conforming f32 evaluations of this chain may disagree on
its sign.

**Independent corroboration (Seat NUMERICS, same batch):**
their seat measured lavapipe's WGSL `acos` landing up to
~851 ulp from libm's `acosf` for the same input bits (their
toon cap pixel: CPU sd +4.8e-6, flipped the same way). At
this pixel, an acos error of that size moves `rb` by
≈ rb·tan(phi − phi_mid)·δphi ≈ 14.79 × 0.0514 × 2.5e-5
≈ 1.9e-5 > 1.43e-5 — quantitatively sufficient to flip the
sign, and no other term in the chain is within two orders of
magnitude of being able to.

## Why no fix from this seat (CONDITIONAL, not FIXED)

- Bit-exact reproduction of libm `acosf`/`cosf` is not
  expressible in portable WGSL; the parity law forbids
  papering over the gap with an epsilon bias in `fan_sd` or
  in the coverage test — that is a silent approximation, and
  it would shift the boundary for **every** cap pixel in
  every case (the battery's cap-end / cap-seg classes and all
  other stroke cases are currently byte-identical; an epsilon
  risks moving them, and Seat NUMERICS owns the shader's SDF
  functions in this batch).
- The principled fix for this mechanism — a Newton
  refinement of the shader's `acos` inside `fan_sd` — is
  exactly Seat NUMERICS' change, and it is in their territory
  (the SDF functions), not in the coverage/blend code this
  seat was confined to. Duplicating it here would collide at
  merge. **Merge note for the coordinator:** with NUMERICS'
  refined-acos binary, replaying this case's recorded list
  on this VM's lavapipe produced the CPU pixel hash
  (63026af9…) — i.e. legs-walk-p3 becomes byte-identical
  under their change (measured here, incidentally, before
  the contamination below was understood; re-verify on the
  merged tree). If their change lands, this CONDITIONAL
  closes as FIXED-by-NUMERICS; if it does not, the bound
  below stands.
- An arithmetic-only reformulation of the fan (signed
  distance to the chord line) is not the CPU's function: the
  CPU's radial model is `line_sd / cos(phi − phi_mid)`, a
  magnitude factor of up to 1/cos(π/2n) ≈ 1.0155 at n = 9,
  which feeds the |d0| < 1 supersample gate and `cov_of`
  saturation behaviour; and its acos-based chord bucketing
  (k) would need a proven-equivalent arithmetic replacement.
  That is a fan redesign with its own parity proof, not a
  confined coverage fix.

## Bound (the CONDITIONAL terms)

- Per affected pixel: coverage differs by exactly one
  supersample step (1/16); at source alpha 255 that is
  ΔA ≤ 16, ΔRGB = 0 (the flipped blend is over the same ink
  colour — RGB is bit-identical even at the flipped pixels).
- Per case: 2 px in legs-walk-p3 (the mirrored pair of feet);
  it is the **only** alpha-affected and only EDGE case in the
  run-#1 table (the four 1-px INTERIOR RGB cases are a
  different family — Seat NUMERICS).
- Zone: EDGE by construction — the flip can only occur at a
  subsample within ~1e-5 px of a cap-fan chord, which is on
  the silhouette boundary.
- Determinism: identical values in CI run #1 (release,
  Mesa 23.2.1/LLVM 15) and locally (debug, Mesa 25.2.8/
  LLVM 20). GPU-vs-GPU determinism is unaffected (14/14 in
  run #1).

## Full local replay vs run #1

Local environment: debug build, llvmpipe Mesa 25.2.8
(LLVM 20.1.2) — CI run #1 was release, Mesa 23.2.1
(LLVM 15.0.7), so per-case equality with the CI table is
checked on identical/diff-px, and environment-only divergences
are called out explicitly.

**Contamination incident (why this table was produced
twice):** all Batch-2 seats share
`CARGO_TARGET_DIR=~/workspace/talerender-target`; at
2026-11-17 02:28:52 UTC a build from Seat NUMERICS' worktree
(their modified `gpupaint.wgsl`) overwrote
`target/debug/talerender`. GPU replays made with the shared
binary after that timestamp (battery face-shock / prims /
toon and the first states sweep) silently ran *their* shader
— that, not base behaviour, is why toon first appeared
locally identical. CPU records are unaffected (the recording
processes exec'd the base binary before the overwrite, and
the CPU painter is not in the shader). All GPU results in
the table below were re-produced with a private binary built
from this branch's unmodified tree
(`CARGO_TARGET_DIR=~/workspace/talerender-b2/edge-target`),
after re-verifying that it reproduces legs-walk-p3 exactly
(GPU sha256 ee924cc4…, 2 px Δ16). The coordinator should
note the shared-target hazard for the remaining seats.

### Battery (7 cards, whole-card; classes via gpupaint-compare.py)

| case | local (base, Mesa 25.2.8, debug) | CI run #1 |
|---|---|---|
| prims | identical | identical |
| toon (card) | **1 px**, Δ 14/14/12/0, INTERIOR, RGBA PSNR 95.438 dB | 1 px, Δ 14/14/12/0, INTERIOR, 95.44 dB |
| toon-head-outline (class) | **1 px** (the toon-card pixel) | 1 px |
| face-neutral / determined / angry / laugh / shock | identical | identical |
| all other 44 battery classes | identical | identical |

### States (75, whole-frame)

**72/75 byte-identical.** The three differing states are
exactly CI run #1's three differing states, with identical
diff counts and deltas:

| case | local diff px | local max Δ (RGBA) | CI run #1 |
|---|---:|---|---|
| legs-walk-p3 | 2 | 0/0/0/16 | 2 px, 0/0/0/16, EDGE |
| xf-campitch-s01 | 1 | 13/13/12/0 | 1 px, 13/13/12/0, INTERIOR |
| xf-headtilt-pos6-think | 1 | 13/13/12/0 | 1 px, 13/13/12/0, INTERIOR |

Identical (72): anchor-high-sit, anchor-lying-lean82,
extra-blush, extra-freckles, extra-glasses, extra-monocle,
extra-mustache, extra-pencil, face-blink-050, face-blink-100,
face-look-lu, face-look-r, face-talk-025, face-talk-050,
face-talk-075, face-talk-100, film-comp-a-celebrate,
film-comp-b-celebrate, film-onoda-jg-s01,
film-onoda-sh-s02-walk, film-suzuki-listen-worried,
film-tani-explain, fx-sad-tears-via-mood, fx-sweat-only,
fx-sweat-tears, fx-tears-only, fx-worried-sweat-via-mood,
hat-beanie, hat-bow, hat-bun, hat-cap, hat-mailcap,
hat-tophat, legs-jump, legs-run-p0, legs-run-p1, legs-sit-film,
legs-walk-p0, legs-walk-p1, legs-walk-p2, mood-angry,
mood-awe, mood-deadpan, mood-determined, mood-dizzy,
mood-embarrassed, mood-happy, mood-laugh, mood-neutral,
mood-proud, mood-puzzled, mood-relieved, mood-sad, mood-shock,
mood-sly, mood-smug, mood-suspicious, mood-tired, mood-worried,
neck-collar, neck-scarf, neck-tie, pattern-apron,
pattern-stripes, pattern-vest, plate-holdl-parcel,
plate-holdr-parcel-label, plate-holdr-parcel-plain,
xf-headtilt-neg5, xf-headturn-pos05, xf-kid, xf-squash-050.

**Agreement with run #1 is total:** every case that was
identical in CI is identical locally; every case that
differed in CI differs locally with the same pixel count and
the same per-channel deltas (legs-walk-p3 also at the same
coordinates). No case moved in either direction — as expected
for a seat landing no code change, and the required baseline
proof that the CONDITIONAL bound is the whole story for this
case. (The NUMERICS cases — toon-head-outline,
xf-campitch-s01, xf-headtilt-pos6-think — reproduce at base
with their run-#1 numbers; this seat's work neither improves
nor worsens them.)

## Evidence

Text evidence (probe output, diff listings, compare JSON/MD,
replay logs, this table) is kept at
`~/workspace/talerender-evidence/gpufirst-b2/edge/`; the CPU
and GPU legs-walk-p3 PNGs are stored there only (no rendered
images are committed to the repo).
