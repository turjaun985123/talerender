# Camera-move presets — seat W25 (Phase 4)

**Deliverable:** `src/cameramoves.rs` (new, self-contained, std-only).
**Tests:** 11/11 pass under standalone `rustc --edition 2021 --test
src/cameramoves.rs` (endpoints bit-exact, handheld repeatability +
smoothness, orbit radius, JSON shape). **Nothing else was edited;**
in particular the module is NOT yet in `main.rs`'s module list —
wiring is the lead's one line (`mod cameramoves;`).

## What this is

Named, parameterised camera moves that **generate key sets** in the
scene format's existing §14.4 camera-key shape. They author data;
they do not render anything.

The consumed shape, surveyed from `src/scenefile.rs::eval_camera_keys`
and `docs/scene-format.md` §14.4 — each key is a JSON object
`{ t, eye: [f32; 3], target: [f32; 3], fovY?, ease? }` inside the
scene's `camera` block. Two evaluator laws shaped the design:

1. Segments ease with **smootherstep by default**; a segment eases
   linearly only when its **destination** key carries
   `"ease": "linear"` (the S05a plane law). The ease lives on the
   key the segment arrives at — the mirror image of `anim::Key`.
2. `fovY` may be omitted per key (falls back to the camera block's
   base `fovY`); end keys hold outside the key range.

`cameramoves::CameraKey` mirrors those fields one-for-one, and
`to_json()` / `keys_to_json()` emit the exact objects the loader
parses.

## How a preset's easing survives the format

The renderer offers only two segment curves (smootherstep, linear),
so each generator picks one of two emission modes:

| Mode | When | Result |
| --- | --- | --- |
| **Native** | profile = smootherstep, `samples <= 2` | exactly 2 keys, no `ease` fields — the renderer's own smootherstep reproduces the profile bit-for-bit |
| **Sampled** | any other profile, or `samples > 2` (orbit: always) | the eased curve is sampled at `samples` points; every key after the first is marked `"ease": "linear"`, so the renderer walks the samples piecewise-linearly |

In both modes the first/last keys are the requested framings
assigned verbatim (never interpolated), so endpoints are bit-exact
at any sample count. More samples only tightens the interior fit.

The easing set (`Ease`) mirrors `src/anim.rs::Easing`
variant-for-variant with identical polynomial forms. `anim` is not
yet wired into `main.rs`, so the enum is local for now; the swap
when the lead wires it is one line
(`pub use crate::anim::Easing as Ease;`, deleting the local block) —
call sites do not change.

## Preset parameter tables

Common parameters: `t0` = start time (s, shot-local), `duration`
(s), `ease` = profile, `samples` = keys to emit (≥ 2). A **framing**
is `{ eye, target, fovY }`.

### `dolly` / `dolly_in` / `dolly_out`
| Parameter | Meaning |
| --- | --- |
| `from`, `to` | full start/end framings; eye, target and fovY all interpolate |
| — | direction is data: dolly-in = nearer framing last, dolly-out = nearer first; the two named wrappers are the same generator |

### `static_with_drift`
| Parameter | Meaning |
| --- | --- |
| `base` | the locked-off framing; target and fovY hold on every key |
| `drift` | eye creep as a fraction of the eye→target distance (0.03 ≈ 3% micro push-in; negative = micro pull-back) |

### `pan_tilt`
| Parameter | Meaning |
| --- | --- |
| `from`, `to` | start/end framings; only `to.target` / `to.fovY` are used — the eye is `from.eye` verbatim on every key, by construction |

### `orbit`
| Parameter | Meaning |
| --- | --- |
| `center` | subject point; the target on every key |
| `radius` | circle radius (m), constant up to f32 sin/cos rounding (~1e-7 rel.) |
| `height` | eye height offset above `center.y` |
| `start_angle`, `sweep` | radians; eye = center + [r·sin a, height, r·cos a], a from `start_angle`, sweeping `sweep` (signed) |
| `fov_y` | held constant |
| `samples` | clamped to ≥ 8 — two keys would chord across the circle |

### `tracking`
| Parameter | Meaning |
| --- | --- |
| `subject` | the subject's own keys: `&[(t, point)]` (e.g. a character position track) |
| `offset` | constant world-space eye offset from the subject point (behind/above/beside) |
| `fov_y` | held constant |

Emits one camera key per subject key: target = subject point
verbatim, eye = point + offset, keys linear so the camera follows
the subject's authored path without lag/surge from easing.

### `handheld_static` / `handheld_overlay`
`Handheld` parameters (defaults are the subtle production values):

| Parameter | Default | Meaning |
| --- | --- | --- |
| `seed` | 1 | any u64; same seed → identical move, on any machine (integer hash only) |
| `amp_eye` | 0.02 m | peak eye wander per axis |
| `amp_target` | 0.04 m | peak target wander per axis (reads as rotational sway) |
| `cell` | 0.8 s | noise lattice spacing; lower = busier |

The noise is smooth value noise — a smootherstep blend between
`hash(seed, axis, cell)` lattice values — so it is continuous, has
no per-frame jitter steps, and depends only on (seed, t), never on
frame history: any frame rate samples the same underlying move. An
envelope fades the noise in/out over the first/last 15% of the
span, so shots start and end exactly on the authored framing.
`handheld_overlay` perturbs any existing key set (dolly, orbit,
hand-authored…) preserving times, fovY and linear flags;
`handheld_static` is overlay over a locked-off framing sampled at
`samples` keys (sample at or above frame rate for the smoothest
ride).

## How a generated key set lands in production data

A staging tool calls a preset and writes `keys_to_json(...)` into
the scene file's `camera.keys`. Concrete example — a 5.91 s
sampled dolly-in (easeOutCubic, 3 keys shown of the emitted set),
in §14.4's actual shape:

```jsonc
"camera": { "fovY": 44, "eye": [0, 2, 8], "target": [0, 1, 0],
  "keys": [
    { "t": 0, "eye": [0, 2, 8], "target": [0, 1, 0], "fovY": 44 },
    { "t": 2.955, "eye": [0.4375, 1.8249999, 4.5], "target": [0, 1.1750001, 0], "fovY": 37, "ease": "linear" },
    { "t": 5.91, "eye": [0.5, 1.8, 4], "target": [0, 1.2, 0], "fovY": 36, "ease": "linear" }
  ] }
```

From there the existing paths take over unchanged:
`scenefile.rs::load` reads `camera.keys` at the scene's `time` and
`eval_camera_keys` evaluates them — this is the loader lane the
film pipeline's scene files already travel. (The hand-coded staging
inside `film.rs` — its `shoot()` from/to mixing — does not consume
§14.4 keys today; routing film staging through generated scene
files is a staging choice, not a renderer gap.)

## Honesty note

Presets generate keys — the renderers **already evaluate keys**, so
adoption needs **no renderer change at all**: no new format fields,
no evaluator work, no A/B re-baselining (a native-mode smootherstep
dolly emits the same two keys an author would write by hand). What
adoption actually requires is staging tooling choices, all outside
this module:

1. a caller — a staging/authoring tool (or film staging code) that
   invokes a preset and writes the JSON where authors currently
   hand-write keys;
2. a seed registry convention for handheld (which seed per shot),
   if productions want reproducible art direction across edits;
3. the lead's two one-line wirings when convenient:
   `mod cameramoves;` in `main.rs`, and the `crate::anim::Easing`
   re-export swap described above.

Until a caller exists, this module is a tested library, not a
feature any render exercises — it claims nothing more.
