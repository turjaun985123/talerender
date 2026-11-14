# Re-pin packet — `dof-chart`

**State: PENDING-not-required — this item carries no shade
debt and no re-pin is owed; no post-fix candidate exists, and
none is needed unless a verification re-render drifts.** This
packet is evidence only; it pins nothing. Packet prepared by
wave seat W68 (finisher), 2026-11-14.

## The item

| | |
|---|---|
| Golden item | `dof-chart` — the M3 DoF chart (contract §1): the transcribed BokehShader against known depths |
| Golden path (manifest) | `docs/ab/dof-chart/native.png` |
| Current pin | `957d5573497e626991a81f719745e5b861e40a28a59159e9a505790939eb36de` (`tools/golden/golden-hashes.json`; the file on disk hashes to exactly the pin — re-verified first-hand by this seat, verification reads **OK**) |
| Pin provenance | The committed M3 package (`docs/ab/dof-chart/`, 2026-11-13) — pre-shade-fix bytes, like the other three founding pins. Unlike them, this item's content does not pass through the helper `c00897d` changed, so pre-fix provenance creates no debt here (analysis below). |

## Why this item carries no shade debt

The full analysis is in `tools/golden/RUNNER.md` ("Current
expectation: the founding four vs the shade fix") and is
summarised in the re-pin debt note in
`docs/golden/NATIVE-GOLDEN.md`:

- The chart texture is **literal arithmetic only**
  (`chart_pattern`, `src/scene.rs`): a white border, a
  (238,238,238)/(20,20,20) checker, a (128,128,128) band —
  **no `shade()` call anywhere in its construction**.
- `c00897d`'s diff is confined to `shade_rgb`'s body in that
  one file. A helper the chart never calls cannot move the
  chart.
- The neighbouring post-pin commits were checked too:
  `e5860b4` (paint-cache cap) is pixel-neutral by construction
  (paints are pure; eviction costs time, never pixels), and
  `3e88c60` touched only the scene-file composer TEX path in
  `src/scenefile.rs` — not the chart, not the bokeh chain.

**Consequence:** a post-fix re-render of this chart is
expected to be byte-identical to the pin. If `dof-chart` ever
verifies **STALE**, the shade fix is *not* the explanation —
treat it as drift, investigate and name the real cause
*before* any re-pin, and do not let it ride on the `c00897d`
disclosure that covers `ab1-composer`.

## Candidate state — none in the repo, none required

Every file in `docs/ab/dof-chart/` is the 2026-11-13 M3
package (README, metrics, diff/side-by-side images, the pinned
`native.png`); a tree-wide check finds **no dof-chart frame
produced at or after `c00897d`** anywhere in the repo. That is
the correct state for this item: producing a candidate is not
debt-clearing, only routine verification (next section), and
this packet claims no post-fix number.

## Production route (routine verification only)

`.github/workflows/render.yml` dispatch, inputs: `mode=dof-chart`
(`from`/`to` unused, defaults `"0"`/`"0"`; `scene` unused) →
`./target/release/talerender --scene dof-chart --out render-out/dof-chart.png`
→ artifact `render-dof-chart-0-0-<run_id>` (retention 7 days),
renderer CI lavapipe. Then:

```sh
sha256sum dof-chart.png
# expected: 957d5573497e626991a81f719745e5b861e40a28a59159e9a505790939eb36de
```

- **Hash equals the pin** → the item is verified post-fix;
  record the run id in this packet and close it. No staging,
  no pin, nothing moves.
- **Hash differs** → the item reads STALE and the shade fix
  cannot explain it. Stop: measure against the pinned frame
  (`tools/ab/measure_ab.py --pair`), find the actual cause,
  and only then — with that cause named — would a human
  re-pin follow the standard procedure in
  `docs/golden/NATIVE-GOLDEN.md`.

## What would close this packet

A routine post-fix verification render matching the pin
byte-for-byte (expected outcome), or a completed drift
investigation if one does not. Until either happens:
**PENDING-not-required — no re-pin owed, no candidate
claimed.**
