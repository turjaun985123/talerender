# run_golden.sh — staging film frames into the native golden pack

`tools/golden/run_golden.sh` is the pack's staging runner: it takes a
film render's frame directory, copies each segment's mid frame to its
golden path, records where the frames came from, and runs the pack's
verifier. Policy lives in
[docs/golden/NATIVE-GOLDEN.md](../../docs/golden/NATIVE-GOLDEN.md);
this file is usage, the re-baseline protocol, and the current
STALE expectation for the four founding references.

## Usage

```
tools/golden/run_golden.sh <frames-dir> --renderer <label> [--repo <path>]
```

| argument | meaning |
|---|---|
| `<frames-dir>` | a film render's output. May be the directory holding `frame_%05d.png` files directly, or a parent holding `frames/` or `render-out/frames/` (the CI artifact layout: `render-film-<from>-<to>-<run_id>` unpacks to `render-out/frames/`). The first candidate containing `frame_*.png` wins. |
| `--renderer <label>` | **required.** Names the renderer that produced the frames — in practice `"CI lavapipe"` or `"PC RADV"`. There is no default: frames of unknown provenance are refused, not guessed at. The label is stored beside the staged copies in `docs/golden/frames/RENDERER.txt`, together with the source dir, staging time (UTC), repo HEAD, and per-frame hashes. A label matching neither "lavapipe" nor "RADV" is accepted but warned about, in the output and in `RENDERER.txt`. |
| `--repo <path>` | repo root; defaults to the repo containing the script. |

What it does, in order:

1. Reads `scenes/film/film.json` live (never a hardcoded table) and
   computes each segment's mid frame index as
   `startFrame + frames // 2` — identical to the pack's
   `(startFrame + endFrame) // 2`, since `endFrame` is the exclusive
   end. Current indices, for reading: s01 89, s02 285, s03 435,
   s04 560, s05a 689, s05b 760, s06 848, s07 969, s08 1119, s08b 1231,
   s09 1306, s10 1440, s11 1593, s12 1738.
2. Copies each present mid frame to `docs/golden/frames/<segment>.png`.
3. Writes `docs/golden/frames/RENDERER.txt` (provenance, above).
4. Runs `python3 tools/golden/make_golden.py` and prints its verdict
   **verbatim** — the table and summary in the output are the tool's
   own, not a paraphrase.
5. Lists every segment whose mid frame was absent, by segment and
   exact frame index. A staged-but-unpinned frame reads **MISSING**
   in the verify table (on disk, never pinned — not yet a reference);
   that is the pack's semantics, not an error.

What it never does: render anything, touch the four milestone
references, compare against Chrome — and **pin**. Passing `--pin`
(or `pin` in any form) is refused with the reason printed: pinning is
the deliberate human re-baseline act defined by the policy, and a
runner must not be able to perform it as a side effect.

Exit codes:

| code | meaning |
|---|---|
| 0 | every segment staged; `make_golden.py` exited 0 |
| 1 | `make_golden.py` reported STALE (passed through) |
| 2 | usage / refusal: `--pin` attempted, `--renderer` missing or empty, unknown argument, frames dir or repo layout not found |
| 3 | PARTIAL: at least one mid frame was absent. The verbatim verify output still stands — MISSING film items do not fail `make_golden.py`, so this code exists to keep a partial staging from looking like a complete one |

Typical flows:

```
# CI: after a render.yml mode=film dispatch covering the whole film
unzip render-film-0-1811-<run_id>.zip -d /tmp/film-render
tools/golden/run_golden.sh /tmp/film-render --renderer "CI lavapipe"

# User PC: a local RADV film render
tools/golden/run_golden.sh ~/renders/onoda/frames --renderer "PC RADV"
```

## The re-baseline protocol

Hashes go STALE for two very different reasons: *drift* (the renderer
changed and nobody can say why — a defect until explained) and a
*deliberate change* (a fix that was supposed to move pixels). Only
the second may be re-baselined, and only like this — the operational
form of NATIVE-GOLDEN.md's "deliberate, disclosed, measured":

1. **Render** the affected item(s) with the current binary, in the
   pack's reference environment (the CI render workflow on lavapipe).
2. **Measure first.** Diff the fresh render against the pinned
   reference before any hash moves: PSNR / mean |Δ| / the differing
   region described in words (`tools/ab/measure_ab.py` is the house
   measurer). A hash change with no measured diff is not a
   re-baseline, it is drift.
3. **Stage** film frames with `run_golden.sh` and read its output —
   including which segments are still MISSING. Do not pin in the
   same motion as staging.
4. **Name the cause** — the specific change responsible (commit,
   fix), never "renderer improved".
5. **A human pins**: `python3 tools/golden/make_golden.py --pin`,
   then commit `golden-hashes.json`, the staged frames, and the
   measured diff together, with the cause in the commit message.
6. **Scope stays visible.** Only the items the cause actually moves
   get new hashes. A whole-set re-pin must justify every item.

## Current expectation: the founding four vs the shade fix (c00897d)

The four OK references were pinned at the pack's founding from the
files as committed — pre-fix bytes, and their pins are those same
bytes, so **verifying the tree as it stands still reads 4 OK**.
Nothing about them is STALE today. The expectation below is about
the next *re-render*: the moment any of these references is
regenerated with a binary at or after the shade root-fix `c00897d`,
its bytes change or don't according to whether its content passes
through the code that commit changed.

`c00897d` redefined the single helper `shade_rgb`
(`src/scene.rs`) from an sRGB byte-multiply to THREE.Color
linear-space arithmetic — the old form ran 25–40 LSB darker on
derived shades — and routed every texture painter through it. Its
own commit message declares the re-baseline: "ab1/hills/grass/felt
shades change".

- **`ab1-composer` — EXPECTED STALE on re-render (certain).** The
  ab1 frame's dominant surfaces are shade-derived end to end: the
  grass ground is `paint_grass`, whose blades are drawn per-blade in
  `shade_rgb` shades; the hills are per-face
  `shade_lin("#5E8F52", 0.92 + rnd·0.16)` (src/ab1.rs) and the tufts
  likewise. The fix commit names this exact scene. A current-binary
  `mode=ab1` render cannot reproduce pin `d7bd71a0…`.
- **`s28-onoda-t150` — EXPECTED STALE on re-render (same mechanism,
  historical-pin caveat).** The same ab1 staging — grass ground,
  shaded hills — so the same conclusion. Two nuances: the pinned PNG
  is the §28 prototype's historical output, produced before the
  current painter code existed, and milestone packages are not
  overwritten by renders, so the file on disk keeps verifying OK
  until someone deliberately regenerates it; and the manifest
  already designates render `mode=ab1` as its current-chain
  equivalent — which is the `ab1-composer` case above. The pack
  keeps this item as the historical first-light pin; `ab1-composer`
  is the living M5-chain reference.
- **`stroke-toon` — EXPECTED STALE on re-render (real, but
  confined).** The battery card is `paint_toon` output
  (`src/battery.rs`), and `paint_toon` calls the very helper
  `c00897d` redefined: the head's even-odd shade crescent is filled
  unconditionally with `shade_rgb(head, 0.9)` (src/scene.rs), the
  body carries a clipped shade ellipse at `shade_rgb(body, 0.8)`
  whenever the spec doesn't override `shade`, and cap shades
  (0.75 / 1.15) apply when a cap is drawn. Those regions move by the
  25–40 LSB class the fix corrects; flat fills and ink strokes do
  not. Note this runs against the *illustrative* example in
  NATIVE-GOLDEN.md — "a composer-chain change legitimately moves
  `ab1-composer` and every film frame, but not `stroke-toon`" —
  which was written for a change confined to the composer chain.
  The shade fix is not composer-confined: it sits in `scene.rs`'s
  shared helper, which the CPU painter also routes through. The
  example's principle (scope stays visible) stands; its specific
  prediction does not cover this fix.
- **`dof-chart` — NOT expected STALE from the shade fix.** The
  chart texture is literal arithmetic only (`chart_pattern`,
  src/scene.rs): a white border, a (238,238,238)/(20,20,20)
  checker, a (128,128,128) band — no `shade()` call anywhere in its
  construction, and `c00897d`'s diff is confined to `shade_rgb`'s
  body in one file. The neighbouring post-pin commits were checked
  too: `e5860b4` (paint-cache cap) is pixel-neutral by construction
  (paints are pure; eviction costs time, never pixels), and
  `3e88c60` touched only the scene-file composer TEX path in
  `src/scenefile.rs` — not the chart, not the bokeh chain. If
  `dof-chart` ever verifies STALE, the shade fix is not the
  explanation: treat it as drift and investigate before any re-pin.

The 14 film items carry no STALE expectation — they have never been
pinned. Their first pins, when the film renders, will be struck from
post-fix frames and will simply be correct-by-construction for the
shade behaviour; the re-baseline question above belongs only to the
founding four.

## Runner test evidence

Exercised in a scratch copy of the repo (never against the real
golden paths with dummy frames): `--pin` refused (exit 2, reason
printed, nothing staged); missing `--renderer` refused (exit 2);
partial staging (3 of 14 frames present) staged 3, listed the other
11 with exact indices, printed the verbatim verify table, exited 3;
full staging from the nested `render-out/frames/` artifact layout
exited 0 with a complete `RENDERER.txt`; an unrecognised renderer
label warned and was recorded; a deliberately corrupted pinned
reference surfaced as STALE with the runner passing through exit 1.
