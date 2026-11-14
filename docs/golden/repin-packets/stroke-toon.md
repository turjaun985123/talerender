# Re-pin packet — `stroke-toon`

**State: PENDING — no post-fix candidate exists yet; the exact
production route is named below.** This packet is evidence
only; it pins nothing. Packet prepared by wave seat W68
(replacement), 2026-11-14.

## The item

| | |
|---|---|
| Golden item | `stroke-toon` — the M4 stroke-battery character card (contract §2): the CPU painter's stroke rasterisation |
| Golden path (manifest) | `docs/ab/stroke-battery/native-toon.png` |
| Current pin | `9ba9ff890402b6cf3aba82c6d488f035ab0caab95cd0a4dbdd0b9bf853b2347c` (`tools/golden/golden-hashes.json`; file on disk re-verified first-hand by this seat — verification reads **OK**) |
| Pin provenance | Pinned at the golden pack's founding from the committed M4 battery output — pre-shade-fix bytes. |
| Why it is expected to move | `paint_toon` routes through the helper `c00897d` redefined: the head's even-odd shade crescent is filled unconditionally with `shade_rgb(head, 0.9)`, the body shade ellipse uses `shade_rgb(body, 0.8)` when the spec doesn't override `shade`, and cap shades (0.75 / 1.15) apply when drawn. Flat fills and ink strokes do **not** pass through the helper and will not move. RUNNER.md's verdict: EXPECTED STALE on re-render — real, but confined to those shade regions, at the 25–40 LSB class the fix corrects. (This also corrects NATIVE-GOLDEN.md's original step-4 example, which predicted this item would not move under a composer change; the shade fix lives in the shared helper, not the composer.) |

## Why there is no candidate yet

- The battery is **not a render.yml mode**. The Render
  (dispatch) workflow offers exactly `ab1`, `dof-chart`,
  `film`, `scene-file` (`.github/workflows/render.yml`,
  verified first-hand) — no dispatch input produces the battery
  card, so none of the post-fix dispatch runs (eleven at
  headSha `c00897d` alone) contains one.
- The generator, per the golden manifest, is the battery mode
  of the binary itself: `talerender --scene stroke-battery
  --out <dir>` ("CI binary; battery mode of src/main.rs"). It
  writes `<dir>/toon.png` (and `prims.png`) from the spec
  embedded in the binary (`src/battery.rs`). It is **CPU
  painter output — no GPU is involved** — so the candidate can
  be produced anywhere a current CI binary runs.

## Production route (exact)

1. **Get a post-fix CI binary.** Download the `talerender-binary`
   artifact from a CI run whose head includes `c00897d`. One
   verified instance from this seat's run listing: run
   **43569768582** (workflow `CI`, headSha `e5860b4` — a
   descendant of `c00897d`, completed/success,
   2026-11-14T07:09:46Z). Any later CI run on a descendant head
   serves equally.
   ```sh
   gh run download 43569768582 --repo turjaun985123/talerender \
     -n talerender-binary -D ~/talerender-bin
   chmod +x ~/talerender-bin/talerender
   ```
2. **Produce the candidate** (from the repo root, so the run is
   recorded against this tree):
   ```sh
   ~/talerender-bin/talerender --scene stroke-battery --out /tmp/battery-postfix
   sha256sum /tmp/battery-postfix/toon.png
   ```
3. **Measure before anything moves** (protocol steps 1–2):
   ```sh
   python3 tools/ab/measure_ab.py \
     --pair docs/ab/stroke-battery/native-toon.png /tmp/battery-postfix/toon.png \
     --out /tmp/measure-stroke-toon
   ```
   Expected character of the diff (a prediction to check, not a
   result): differences confined to the head shade crescent, the
   body shade ellipse and any drawn caps; flat fills and ink
   byte-identical; frame-to-frame PSNR high (most of the card is
   untouched). If the diff shows movement anywhere else, treat
   it as drift and stop — the shade fix cannot explain it.
4. **Stage, pin, commit** — only after the measurement is on
   record: copy `/tmp/battery-postfix/toon.png` over
   `docs/ab/stroke-battery/native-toon.png`, run
   `python3 tools/golden/make_golden.py --pin`, verify with a
   plain run that all four milestone items read OK and that
   `git diff tools/golden/golden-hashes.json` moves **only**
   the `stroke-toon` line, and commit the hash file, the
   replaced reference and the measurement together, naming
   `c00897d` as the cause (same disclosure pattern as the
   ab1-composer packet).

## What would close this packet

A produced candidate + its measurement, after which the human
re-pin act above clears the item's share of the shade debt.
Until then: **PENDING — route named, no candidate, no number
claimed.**
