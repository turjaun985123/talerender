# S12-BAKE-W52 — the s12 `apply` bake pack (bake option, proven)

**Seat:** TaleRender Wave 2, W52 (Batch 6 — animation adoption).
**Date:** 2026-11-14. **Repo HEAD at writing:** `aa92cee`.
**Status:** DONE, first-hand throughout. New files only; nothing
committed, no existing file edited — the real
`scenes/film/s12.scene.json` and `tools/format/stage_clips.py` are
untouched (the baked staging is a copy in a new directory).

## Verdict up front

**Both blocked records bake faithfully. There is no residue.**
`lean` (× 0.6) and `headTilt` (× −1) are pure scalar multiplies of
the evaluated bow track, and scalar multiplication distributes
over every operation `pose_track` performs (hold, and lerp with a
scalar phase), so multiplying the factors through the stored keys
reproduces the code's channel values — bit-identical f32 at both
key times, and mid-segment deviations that are *entirely* the
§5.2/§11 conversion cost the other 63 records already pay, with
the bake's own contribution ≤ 1 f32 ulp (≤ 2.98e-8) at every
sample. With the baked copy substituted, the census closes:
**14/14 scene files convert, 65/65 key-carrying records convert,
validator PASS 5/5 on all 14 outputs.** The lead does not need to
extend the tool for s12; the one disclosed imperfection is the
sign of an exact zero (headTilt key 1, §3).

## Deliverables

- `tools/format/staging/s12-baked/s12.scene.json` — a full copy of
  `scenes/film/s12.scene.json` with exactly two records changed
  (raw diff vs the original: 10 lines — the two key values, the
  two `apply` fields removed, the two `law` fields rewritten with
  bake provenance). Byte-format preserved (1-space indent, no
  trailing newline; a load/dump round-trip of the original is
  byte-identical, so the copy method is lossless).
- `tools/format/staging/s12-baked/bake_s12.py` — the regenerator
  (§6). It verifies the source records still match the recorded
  originals exactly and **fails without writing** if they have
  drifted.
- `tools/format/staging/s12-baked/equivalence_check.py` — the
  three-path sampled equivalence proof (§5), re-runnable.
- This note.

## 1. What was blocked (the record)

W32's converter fails the s12 run naming two records on node
`Onoda (JG onoda2)` (ANIM-ADOPT-W32 §4): both are `poseTrack`s
whose keys are the *pre-transform* bow track —

| record | keys (as staged) | `apply` (as staged) |
|---|---|---|
| `lean` | `[[0, 0], [2.5, 3.5, 0.45]]` | `lean = bowA * 0.6` |
| `headTilt` | `[[0, 0], [2.5, 3.5, 0.45]]` | `headTilt = -bowA` |

The code of record is the S12 driver in `src/film.rs` (the bow on
"loyal"; `w_loyal - st0 - 0.1 = 2.5` shot-local, per the records'
own `law`):

```rust
let bow = pose_track(&[key(0.0, 0.0), keym(w_loyal - st0 - 0.1, 3.5, 0.45)], lt);
onoda.lean      = (0.6 * bow) as f32;
onoda.head_tilt = (-bow) as f32;
```

Nothing sits between the track evaluation and the factor — no
clamp, no abs, no second curve — and nothing else is read from
these records. That is what makes the bake lawful; §3 does the
arithmetic.

## 2. Before / after

| record | key | before (pre-transform) | factor | after (baked) |
|---|---|---|---|---|
| `lean` | `[0, v]` | `v = 0` | × 0.6 | `v = 0` |
| `lean` | `[2.5, v, 0.45]` | `v = 3.5` | × 0.6 | `v = 2.1` |
| `headTilt` | `[0, v]` | `v = 0` | × −1 | `v = 0` (see §3, sign of zero) |
| `headTilt` | `[2.5, v, 0.45]` | `v = 3.5` | × −1 | `v = −3.5` |

Times and `moveSecs` are **copied, never multiplied** — the
factors act on values only, exactly as in the code. The `apply`
field is removed; each record's `law` now opens with the bake
provenance (factor, the film.rs line it came from, and the
pre-bake law verbatim after it), so the staged file stays
self-describing.

## 3. Per-key f32 arithmetic (no silent rounding)

The state the film stores is f32 (`(0.6 * bow) as f32`), and the
converter rounds every emitted number to the nearest f32 (§11
law; its `f32q`). Each baked key is checked against the code's
own cast, in bits:

- **lean, arrival (the only non-trivial product).**
  0.6 as f64 is 0.59999999999999997779553950749686919152736663818359375.
  The f64 product 0.6 × 3.5 rounds to *exactly* f64(2.1)
  (verified by exact fraction arithmetic:
  `Fraction(0.6 * 3.5) == Fraction(2.1)`), and both routes to f32
  land on the same value:
  code cast `(0.6*3.5) as f32` = **2.0999999046325684, bits
  0x40066666**; baked token `2.1` → `f32q` = **2.0999999046325684,
  bits 0x40066666**. Bit-identical — writing `2.1` in the data is
  not an approximation of the code's value, it *is* the code's
  value after the two roundings each side performs.
- **lean, key 1.** 0.6 × 0 = 0 exactly; bits 0x00000000 both sides.
- **headTilt, arrival.** −1 × 3.5 = −3.5, exact in f64 and in f32;
  bits 0xC0600000 both sides.
- **headTilt, key 1 — the one disclosed imperfection.** The code
  computes −(0.0) = **−0.0** (bits 0x80000000). The baked data
  spells `0` (bits 0x00000000), as the source record itself does.
  This is a pure scalar multiply whose result differs only in the
  sign of an exact zero: −0.0 == 0.0 under every comparison, and
  `lerp(−0.0, v, k) == lerp(0.0, v, k)` except for the sign of an
  exact-zero result, which no consumer reads (the value feeds an
  f32 pose state used arithmetically). It is named here rather
  than hidden inside a "bit-identical" claim; if the lead wants
  even this gone, the data token would have to be `-0.0`, which
  no other record in the corpus uses.

Neither transform is anything but a scalar multiply of stored
keys — there is no composite-value case in either record (the
values are scalars; the vectors in this scene live on other
channels and carry no `apply`).

## 4. Conversion runs (verbatim)

Baseline at this HEAD — the original file still fails, naming
both records:

```
$ python3 tools/format/stage_clips.py scenes/film/s12.scene.json -o /tmp/w52/s12-orig.clips.json
stage_clips.py — scenes/film/s12.scene.json (2 clips, 4 tracks, 8 keys staged; 2 records code-only)

[1 input] PASS — no failures

[2 channels] FAIL — 2 failure(s)
    nodes[Onoda (JG onoda2)].channels.lean: record carries an 'apply' transform ('lean = bowA * 0.6') — its keys are pre-transform values, not channel values; converting them verbatim would write wrong data. Apply the transform in the staging data or extend this tool deliberately
    nodes[Onoda (JG onoda2)].channels.headTilt: record carries an 'apply' transform ('headTilt = -bowA') — its keys are pre-transform values, not channel values; converting them verbatim would write wrong data. Apply the transform in the staging data or extend this tool deliberately

[3 conversion] PASS — no failures
[4 code-only] PASS — no failures
OVERALL: FAIL (3/4 checks passed) — no output written     (exit 1)
```

The baked copy:

```
$ python3 tools/format/stage_clips.py tools/format/staging/s12-baked/s12.scene.json -o /tmp/w52/s12-baked.clips.json
stage_clips.py — tools/format/staging/s12-baked/s12.scene.json (2 clips, 6 tracks, 14 keys staged; 2 records code-only)

[1 input] PASS — no failures
[2 channels] PASS — no failures
[3 conversion] PASS — no failures
[4 code-only] PASS — no failures
OVERALL: PASS (4/4 checks passed)                          (exit 0)

$ python3 tools/format/check_clips.py /tmp/w52/s12-baked.clips.json
[1 structure] PASS   [2 channels] PASS   [3 keys] PASS
[4 references] PASS  [5 f32] PASS
OVERALL: PASS (5/5 checks passed)                          (exit 0)
```

The two poseTracks add 2 tracks and 6 keys over the baseline's
staged-but-unwritten counts (each `[t, v, m]` record key becomes
the §5.2 hold key + arrival: keys at 0, 2.049999952316284, 2.5).
The emitted tracks, verbatim from the clips document:

```
lean     [{"t": 0.0, "value": 0}, {"t": 2.049999952316284, "value": 0},
          {"t": 2.5, "value": 2.0999999046325684, "ease": "smoother"}]
headTilt [{"t": 0.0, "value": 0}, {"t": 2.049999952316284, "value": 0},
          {"t": 2.5, "value": -3.5, "ease": "smoother"}]
```

**Census, re-run in full at this HEAD** (converter + validator
over every file): s01–s11 and s05a/s05b/s08b — **CONVERT-PASS,
validator PASS 5/5 each (13/13)**; original s12 — CONVERT-FAIL
(transcript above); baked s12 — CONVERT-PASS, PASS 5/5. Totals
with the bake adopted: **14/14 files, and 65/65 key-carrying
records** (W32's 63 + this pair). s12's other records (mood step,
camera, the two blink directors accounted code-only) are
unaffected — they convert in the same run, as W32 reported.

## 5. Equivalence (sampled, three paths)

Why it should hold: `pose_track` (film.rs) returns either a
stored key value or `lerp(v0, v1, smoother(u))`. A scalar factor
distributes over both — `s·v` for the held values, and
`s·lerp(a,b,u) = lerp(s·a, s·b, u)` in exact arithmetic — and the
factor never touches times or phases. So the baked track and the
code compute the same function; the only deviations possible are
floating-point rounding, which the three-path comparison below
decomposes:

- **CODE** — film.rs transcribed (f64 `pose_track`/`smoother`/
  `lerp` replicas), factor applied to the evaluated track, cast
  to f32, as the driver does.
- **A** — bake-then-convert: the real converter's output for the
  baked staging, evaluated under the §5.2 spelling.
- **B** — convert-then-scale (the control): the same §5.2
  spelling built from the *original* keys with the converter's
  own f32 rounding, evaluated, and only then multiplied by the
  factor and cast. **A−B isolates the bake; B−CODE isolates the
  conversion.**

Samples: both key times (0, 2.5), the move-window start (2.05),
two mid-segment times (2.2, 2.35), a hold-region time (1.0), a
post-arrival time (3.0). Full output of
`tools/format/staging/s12-baked/equivalence_check.py` (exit 0):

```
TRACK lean (factor 0.6)
  t      CODE f32            A baked f32         B ctrl f32          A-B ulp  B-C ulp  A-C ulp  |A-C|
  0.0    0.0                 0.0                 0.0                 0        0        0        0.000e+00
  1.0    0.0                 0.0                 0.0                 0        0        0        0.000e+00
  2.05   0.0                 2.498571580078732e-20  2.4985717416374453e-20  1   *    *        2.499e-20
  2.2    0.4407407343387604  0.4407409429550171  0.4407409727573395  1        8        7        2.086e-07
  2.35   1.6592592000961304  1.65925931930542    1.65925931930542    0        1        1        1.192e-07
  2.5    2.0999999046325684  2.0999999046325684  2.0999999046325684  0        0        0        0.000e+00
  3.0    2.0999999046325684  2.0999999046325684  2.0999999046325684  0        0        0        0.000e+00

TRACK headTilt (factor -1.0)
  t      CODE f32            A baked f32         B ctrl f32          A-B ulp  B-C ulp  A-C ulp  |A-C|
  0.0    -0.0                0.0                 -0.0                0        0        0        0.000e+00
  1.0    -0.0                0.0                 -0.0                0        0        0        0.000e+00
  2.2    -0.7345678806304932 -0.7345682382583618 -0.7345682382583618 0        6        6        3.576e-07
  2.35   -2.765432119369507  -2.765432357788086  -2.765432357788086  0        1        1        2.384e-07
  2.5    -3.5                -3.5                -3.5                0        0        0        0.000e+00
  3.0    -3.5                -3.5                -3.5                0        0        0        0.000e+00
(headTilt t=2.05 omitted here for width: A = B = -4.164286020650791e-20, CODE = -0.0 — same degenerate class as lean's 2.05 row.)
```

Reading the numbers:

- **Key times are bit-identical** (both tracks, t = 0 and 2.5,
  modulo the disclosed sign-of-zero at headTilt's zeros). The
  stored data — the thing the §11 law governs — is exact.
- **The bake's own contribution (A−B) is ≤ 1 ulp everywhere**,
  max absolute 2.98e-8 (lean, t = 2.2); it is 0 at 11 of 14
  samples. Whether the factor is applied before or after the
  lerp is below the data's precision.
- **All remaining deviation is the conversion's, not the
  bake's** (B−CODE column): worst 3.576e-7 absolute (headTilt,
  t = 2.2). Its cause is mechanical and already accepted for
  the other 63 records: the spelled hold key sits at
  f32(2.05) = 2.0499999523162841796875, which is
  4.768371580921027e-08 s (1.4e-6 frames at 30 fps) before the
  code's f64 window start 2.5 − 0.45 =
  2.0499999999999999888977697537484345957636833190917968750,
  plus the f32 key values themselves. At t = 2.05 specifically,
  CODE is still (just) outside/at the window edge and underflows
  to exactly 0.0 while the spelled tracks, 1.06e-7 into their
  phase, read ~2.5e-20 / −4.2e-20 — the ulp metric is meaningless
  across the subnormal boundary (marked `*`), and the absolute
  values are zero at any renderable precision.
- At lean t = 2.2 the baked path lands *closer* to CODE than the
  control (7 vs 8 ulps) — reported as measured; the point is
  that A never exceeds the conversion envelope by more than the
  1-ulp bake term.

In one line: adopting the bake costs nothing the adoption hasn't
already accepted for every other poseTrack, and the values it
stores at the keys are the code's own f32 results.

## 6. The honest cost, and regeneration

Baking freezes the factors into data. The scene file no longer
says "0.6 × however the bow track evolves" — it says 2.1. If the
S12 driver in film.rs changes the factor (0.6 → anything), or the
bow keys move (a different `w_loyal`, a deeper bow), **the baked
data does not follow**; it must be re-baked, and until it is, the
data silently describes the old bow. Three mitigations, all in
place:

1. The baked records' `law` fields carry the factor, the film.rs
   expression, and the pre-bake law — the dependency is written
   in the file, not only here.
2. Regeneration is one command:
   `python3 tools/format/staging/s12-baked/bake_s12.py`
   It re-derives the baked copy from the *current*
   `scenes/film/s12.scene.json` — and it **refuses (exit 1,
   nothing written)** if the source records' keys or `apply`
   text differ from the recorded originals, so a changed bow
   fails loudly at re-bake time instead of propagating silently.
   A factor change is a one-line edit to `FACTORS` in that
   script, next to the film.rs lines it mirrors.
3. The bake is a *staging* artefact: the adoption step should
   apply the same transform to the real scene file (or adopt
   this copy over it) in the same commit that removes the
   film.rs `apply` evaluation — keeping a baked copy and an
   unbaked original as parallel sources of truth is the failure
   mode to avoid, and this pack deliberately leaves the real
   file untouched so that decision stays the lead's.

Boundary of the precedent: this ruling is for records that carry
their **own pre-transform keys plus an `apply` string**. It does
not reach W32's *derived channels* class — e.g. s11's headTilt,
which carries no keys and is defined as −0.8 × another record's
track; that stays code under §5.3, as ruled. If a future record's
`apply` is not a pure scalar multiply of its own keys (an offset,
a clamp, a composite/vector operation), the bake does not
generalise and the tool-extension option is the right one — that
case does not occur in the current corpus (verified directly:
`grep '"apply"' scenes/film/*.scene.json` matches only
s12.scene.json, exactly 2 records).

## 7. Recommendation

Adopt the bake. The tool-extension option buys nothing for s12:
the factors are two constants, the baked values are bit-exact at
the keys, and the extension would add an `apply` evaluator to the
loader path to reproduce arithmetic this pack has already done
and proven. Keep the extension in reserve for the first
non-scalar `apply`, if one is ever staged.
