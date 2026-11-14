# The native golden pack — policy

**Why this pack exists.** Compatibility contract §5 resolved
(2026-11-14) to option **(b): native frames stay CLEAN** — no
deliberate q0.93 JPEG generation on native masters. The stated cost
of (b) was that "native QA references are re-baselined once,
natively". This pack is that re-baseline: TaleRender's own golden
reference set, produced by the native renderer, pinned by hash, and
kept strictly separate from the Chrome side.

**What this pack is not.** The Tall Tale skill's golden sheet
(md5 `c535024d05b0d268186af4a4f7103642`) remains the skill's golden,
checked by the skill's own pipeline against Chrome renders. It is
not imported here, not compared here, and no native frame is ever
expected to match it byte-for-byte — the Chrome masters sit behind
their q0.93 JPEG generation (measured: 42.31 dB, 94.5 % of pixels
changed, `docs/ab/telemetry/jpeg-decision.md`); the native frames do
not. Conflating the two goldens would bake that loss back in as an
equality target, which is exactly what decision (b) refused.

## The golden set (18 items)

Four milestone references, already committed as milestone evidence
and pinned at this pack's founding, plus one mid frame per film
segment once the film renders (14 items, currently MISSING — the
honest state until the frames exist).

| item | what it pins | reference path |
|---|---|---|
| `s28-onoda-t150` | §28 first-light frame: Onoda S01 @ t = 1.50, the first real scene reproduced natively | `docs/ab/onoda-s01-t150/native.png` |
| `dof-chart` | M3 DoF chart (contract §1): the transcribed BokehShader against known depths | `docs/ab/dof-chart/native.png` |
| `stroke-toon` | M4 stroke-battery character card (contract §2): the CPU painter's stroke rasterisation | `docs/ab/stroke-battery/native-toon.png` |
| `ab1-composer` | M5: ab1 through the full composer chain (Render → Bokeh → Bloom → Output → GRADE) | `docs/ab/composer/native.png` |
| `film-s01` … `film-s12` | one mid frame per segment of the Onoda film (14 segments: s01, s02, s03, s04, s05a, s05b, s06, s07, s08, s08b, s09, s10, s11, s12), mid frame = (startFrame + endFrame) // 2 from `scenes/film/film.json` | `docs/golden/frames/<segment>.png` |

The milestone items live in their milestone packages (that is where
their A/B evidence lives); the film frames live here. The manifest
tool (`tools/golden/make_golden.py`) is the single registry: item
ids, sources, generator commands, expected paths, pinned hashes.

## How each reference is generated

- **Renderer:** the CI render workflow
  (`.github/workflows/render.yml`, workflow_dispatch) on lavapipe —
  the environment every milestone proof was produced on. Commands
  per item are recorded in the manifest (`--json`): `ab1` and
  `dof-chart` modes render their stills directly; film frames use
  mode `film` with `--from <mid> --to <mid+1>` and the single output
  `frame_<mid:05>.png` is copied to the item's golden path at
  pinning time; the stroke card is CPU-painter output
  (`talerender --scene stroke-battery`), no GPU involved.
- **Format: clean PNG, always.** No JPEG generation is applied to a
  golden reference at any point — not q0.93, not any other quality.
  Preview JPEGs (0.8/0.92) remain preview-grade QA surfaces exactly
  as contract §5 states; they are never pinned as goldens.
- **Determinism:** contract §4 — `frame(t)` is a pure function of
  (t, project data, seeds), so a re-render of a pinned item must be
  byte-identical. That is what makes a hash pin meaningful: OK means
  *the same frame*, not *a similar frame*.

## Re-baselining — deliberate, disclosed, measured

A golden changes only by a deliberate act, mirroring the skill's
golden discipline (where the sit-solver fix moved the golden sheet
only as a disclosed, diff-confined re-baseline candidate):

1. **Measure first.** Before any re-pin, render the item with the
   current binary and diff it against the pinned reference (PSNR /
   mean |Δ| / the differing region described in words). The diff
   goes in the commit message or the milestone package — a hash
   change with no measured diff is not a re-baseline, it is drift.
2. **Name the cause.** Engine change, staging-data correction,
   format change — the commit that updates `golden-hashes.json`
   states which, and cites the change responsible. "Renderer
   improved" is not a cause; the specific fix is.
3. **Re-pin explicitly.** `python3 tools/golden/make_golden.py --pin`
   records the new hashes. It is a human act, never a CI step and
   never a side effect of rendering.
4. **Scope stays visible.** If a change moves one item, only that
   item's hash moves. A commit that re-pins the whole set must say
   why every item moved (e.g. a composer-chain change legitimately
   moves `ab1-composer` and every film frame, but not the CPU
   painter's `stroke-toon`).

Verification is `python3 tools/golden/make_golden.py`: **OK** =
bytes match the pin; **STALE** = on disk but not the pinned bytes
(the run exits 1 — a STALE item is a failed gate until it is either
fixed or deliberately re-baselined under the rules above);
**MISSING** = not yet rendered/pinned (reported, not a failure —
the film frames sit here until the film renders).

## Native-vs-Chrome A/B: reported, never gated as equality

Per-shot A/B numbers against the Chrome review frames (PSNR table,
side-by-sides, the residual classes in each milestone package) are
**reported evidence for a human to read — they are not a pass/fail
equality gate**, for the reason decision (b) documented: the Chrome
side of every comparison sits behind its q0.93 JPEG generation, so
exact equality is the wrong target and a dB threshold pretending
otherwise would quietly gate on the JPEG floor, not on the renderer.

The actual gates for native output are:

1. **The telemetry verdict** (contract §3) — the native `cutouts()`
   audit must agree with Chrome's on the fields it judges
   (overflow, boxes, signatures, caption audit, one verdict
   expression). This gate is semantic and exact, and it already
   passed at M6 with exact agreement.
2. **The per-shot A/B table, reviewed by a human** — every shot's
   numbers present, the largest residuals explained in the package
   (the standing residual classes: the JPEG floor on the Chrome
   side, texture mip/aniso, IBL-sensitive regions, the GRADE grain
   hash's vendor sensitivity).
3. **This pack's hash verification** — the native frames must keep
   matching the *native* references until a disclosed re-baseline
   says otherwise.

## Current state (founding baseline, 2026-11-14)

Produced by `python3 tools/golden/make_golden.py` at this pack's
founding. The four milestone references were pinned from the files
as committed (their sha256 is the committed bytes); the film frames
read MISSING until the film renders — reported as such, not hidden.

```
item            status   pinned            current           path
s28-onoda-t150  OK       ff68fad3bbcc6155  ff68fad3bbcc6155  docs/ab/onoda-s01-t150/native.png
dof-chart       OK       957d5573497e6269  957d5573497e6269  docs/ab/dof-chart/native.png
stroke-toon     OK       9ba9ff890402b6cf  9ba9ff890402b6cf  docs/ab/stroke-battery/native-toon.png
ab1-composer    OK       d7bd71a02a3dfc96  d7bd71a02a3dfc96  docs/ab/composer/native.png
film-s01        MISSING  -                 -                 docs/golden/frames/s01.png
film-s02        MISSING  -                 -                 docs/golden/frames/s02.png
film-s03        MISSING  -                 -                 docs/golden/frames/s03.png
film-s04        MISSING  -                 -                 docs/golden/frames/s04.png
film-s05a       MISSING  -                 -                 docs/golden/frames/s05a.png
film-s05b       MISSING  -                 -                 docs/golden/frames/s05b.png
film-s06        MISSING  -                 -                 docs/golden/frames/s06.png
film-s07        MISSING  -                 -                 docs/golden/frames/s07.png
film-s08        MISSING  -                 -                 docs/golden/frames/s08.png
film-s08b       MISSING  -                 -                 docs/golden/frames/s08b.png
film-s09        MISSING  -                 -                 docs/golden/frames/s09.png
film-s10        MISSING  -                 -                 docs/golden/frames/s10.png
film-s11        MISSING  -                 -                 docs/golden/frames/s11.png
film-s12        MISSING  -                 -                 docs/golden/frames/s12.png

4 OK / 0 STALE / 14 MISSING of 18 golden items
```

Pinned hashes (full): `tools/golden/golden-hashes.json` —
`s28-onoda-t150` ff68fad3bbcc6155e8d7781325a7ad57dafa391ca4fc883a0e33dd28df60bf62 ·
`dof-chart` 957d5573497e626991a81f719745e5b861e40a28a59159e9a505790939eb36de ·
`stroke-toon` 9ba9ff890402b6cf3aba82c6d488f035ab0caab95cd0a4dbdd0b9bf853b2347c ·
`ab1-composer` d7bd71a02a3dfc96232957123bfb82766bf7a4e96676f96e72db4e6c7adaf4f3.
