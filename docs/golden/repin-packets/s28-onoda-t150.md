# Re-pin packet — `s28-onoda-t150`

**State: CANDIDATE IDENTIFIED AND MEASURED — re-pin NOT part of
the shade-debt clearing; the decision is a separate human one.**
This packet is evidence only; it pins nothing. Packet prepared
by wave seat W68 (replacement), 2026-11-14.

## The item

| | |
|---|---|
| Golden item | `s28-onoda-t150` — the §28 first-light frame: Onoda S01 @ t = 1.50, the first real scene reproduced natively |
| Golden path (manifest) | `docs/ab/onoda-s01-t150/native.png` |
| Current pin | `ff68fad3bbcc6155e8d7781325a7ad57dafa391ca4fc883a0e33dd28df60bf62` (`tools/golden/golden-hashes.json`; file on disk re-verified first-hand by this seat — verification reads **OK**) |
| Pin provenance | The §28 **prototype's** historical output, produced before the current painter/composer code existed. Milestone packages are not overwritten by renders, so the file keeps verifying OK until deliberately regenerated. The manifest designates render `mode=ab1` as its current-chain equivalent and `ab1-composer` as the living M5-chain reference for the same staging. |

## The candidate

The current chain's render of the same staging (ab1 @ t = 1.50)
already exists, produced after the shade fix:

| | |
|---|---|
| Frame | `docs/ab/composer-v2/native.png` — the same file that is the `ab1-composer` packet's candidate |
| sha256 | `a415ead80a1a17d2b190c03c237bd2f8129057a8688ebbfa31cd5d26ffceeee9` |
| Produced by | Render (dispatch) run **43569670339**, mode `ab1`, headSha `c00897d32bf86bd9a081b7152eaa2bd0fe3f1e2b` (the shade-fix commit) |
| Renderer | talerender (wgpu), **CI lavapipe** |
| Production route for a fresh one | `.github/workflows/render.yml` dispatch, inputs: `mode=ab1` (`from`/`to` unused, defaults `"0"`/`"0"`; `scene` unused) → artifact `render-ab1-0-0-<run_id>`, file `ab1.png`. No `run_golden.sh` staging applies (that runner stages film frames only). |

## The measurement — and why it changes the question

This seat measured the pinned §28 frame against the candidate
with the house tool (reproduce:
`python3 tools/ab/measure_ab.py --pair docs/ab/onoda-s01-t150/native.png docs/ab/composer-v2/native.png --out <dir>`):

| metric (pinned §28 frame vs post-fix candidate) | value |
|---|---|
| PSNR | **21.711 dB** |
| mean \|Δ\| per channel | 16.1313 |
| %px max-channel Δ > 10 | 79.39603 |
| %px any-channel Δ > 0 | 98.45809 |
| max \|Δ\| | **206** |

Compare the shade fix's own isolated signature (ab1-composer
packet: pre-fix native vs post-fix native on the *same* current
chain): 41.351 dB, max Δ 30, ~37 % of pixels touched. The
§28-vs-candidate difference is far larger and structural —
79 % of pixels beyond Δ 10 and a max Δ of 206 cannot be a shade
correction. It is the accumulated difference between the §28
prototype's output and the current M5-chain render of the same
staging (the prototype predates the current painter code; the
chain has since passed M2 format work and the M5 composer
build). The shade fix is a minor component of this delta, not
its cause.

**Consequence.** The RUNNER.md/NATIVE-GOLDEN.md expectation —
"EXPECTED STALE on re-render (same mechanism)" — is true only in
the narrow sense that a regeneration would not reproduce the
pin. But re-pinning this item from the current chain would not
be a shade re-baseline: it would replace a historical artifact
(the first frame this renderer ever produced of a real scene)
with a different pipeline generation's output, and the measured
diff could not honestly be attributed to `c00897d`. The pack's
own policy already draws this line: the §28 item is *kept as
the historical first-light pin*; `ab1-composer` is the living
reference, and its packet carries the actual shade re-pin.

## Recommendation

- **Do not re-pin `s28-onoda-t150` as part of clearing the
  `c00897d` debt.** The pin continues to verify OK against the
  historical file, which is the correct state for a historical
  reference.
- If a human separately decides the §28 item should track the
  current chain, that is a *retirement/replacement* decision,
  not a re-baseline: it should be taken with this measurement on
  the table (21.711 dB / max Δ 206 vs the candidate), recorded as
  superseding the §28 prototype output, and executed by staging
  a `mode=ab1` render (dispatch inputs above) at
  `docs/ab/onoda-s01-t150/native.png` and running
  `python3 tools/golden/make_golden.py --pin`, with the same
  scope check as the ab1-composer procedure (only this item's
  hash may move).
- Until such a decision, this packet's state stays: **candidate
  identified, measured, re-pin not recommended under the shade
  debt.**
