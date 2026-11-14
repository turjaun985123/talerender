# Re-pin packet — `ab1-composer`

**State: RE-PINNED 2026-11-14 (Wave 2 lead, Act 2) — pin
`d7bd71a0…` → `60c32c1a…`.** The pinned frame is a FRESH render
at the integration head (CI run 43621168728, head `9f497c7`,
CI lavapipe), not the `c00897d` candidate below: the candidate
predated the Wave-1 determinism fix chain, and a pin must
describe current production output. Measured at the act: fresh
vs Chrome 23.98 dB (the candidate measured 23.96 dB — the A/B
relationship is unchanged), fresh vs candidate 53.50 dB
(sub-visible residual from the post-candidate fix chain). The
packet's procedure was followed with that substitution; the
pin diff moved ONLY the ab1-composer line, and the golden pack
verifies 18 OK / 0 STALE / 0 MISSING with Act 1's film pins in
place. Original packet text follows.
This packet is evidence only. It pins nothing: the pin is the
deliberate human act defined in `docs/golden/NATIVE-GOLDEN.md`
("Re-baselining — deliberate, disclosed, measured"), and
`tools/golden/run_golden.sh` refuses `--pin` outright.
Packet prepared by wave seat W68 (replacement), 2026-11-14.

## The item

| | |
|---|---|
| Golden item | `ab1-composer` — ab1 (Onoda S01 @ t = 1.50) through the full M5 composer chain: Render → Bokeh → Bloom → Output → GRADE |
| Golden path (manifest) | `docs/ab/composer/native.png` |
| Current pin | `d7bd71a02a3dfc96232957123bfb82766bf7a4e96676f96e72db4e6c7adaf4f3` (`tools/golden/golden-hashes.json`) |
| Pin provenance | The pre-shade-fix frame, committed with the M5 composer package and pinned at the golden pack's founding (2026-11-14). Verification against the file on disk still reads **OK** today — pin and file are the same pre-fix bytes (hash re-verified first-hand by this seat). |
| Why it must move | `c00897d` root-fixed `shade_rgb` in `src/scene.rs` (sRGB byte-multiply, 25–40 LSB dark on derived shades → THREE.Color linear-space multiply via `shade_lin`) and routed every texture painter through it. The ab1 frame's dominant surfaces are shade-derived end to end (`paint_grass` blades, per-face hill shades, tufts), and the fix commit's own message declares the re-baseline ("ab1/hills/grass/felt shades change"). `tools/golden/RUNNER.md` predicted this item EXPECTED STALE on re-render, certain; the candidate below confirms it. |

## The candidate

| | |
|---|---|
| Frame | `docs/ab/composer-v2/native.png` (the W57 measurement pack) |
| sha256 | `a415ead80a1a17d2b190c03c237bd2f8129057a8688ebbfa31cd5d26ffceeee9` (re-verified first-hand by this seat) |
| Produced by | GitHub Actions **Render (dispatch)** run **43569670339**, mode `ab1`, at headSha `c00897d32bf86bd9a081b7152eaa2bd0fe3f1e2b` — the shade-fix commit itself |
| Artifact | `render-ab1-0-0-43569670339` (single `ab1.png`, 1080×1920; created 2026-11-14T07:10:03Z; the staged file is byte-identical to the artifact) |
| Renderer | talerender (wgpu) on **CI lavapipe** — the pack's reference environment |
| Production route note | This item is **not** staged by `tools/golden/run_golden.sh` — that runner stages *film* mid frames only. The ab1 still is produced by the binary's `--scene ab1` (render.yml mode `ab1` → `./target/release/talerender --scene ab1 --out render-out/ab1.png`). Its staging for the pin is the manual copy in step 2 below. |

## The measurement (protocol step 1–2: measured, cause named)

Tool: `tools/ab/measure_ab.py --pair` (house definitions; control
run on the committed M5 pair reproduced the pinned numbers exactly
before the candidate was measured — see
`docs/ab/composer-v2/README.md`).

Against the Chrome reference (`docs/ab/composer/original.png`,
three.js r170 in headless Chrome; carries one q0.93 JPEG
generation, so all figures understate native accuracy
one-directionally):

| metric | pre-fix pin (M5) | candidate (post-fix) | Δ |
|---|---|---|---|
| PSNR | 23.850 dB | **23.96 dB** | +0.11 dB |
| mean \|Δ\| per channel | 5.20056 | **4.82505** | −0.376 |
| %px max-channel Δ > 10 | 13.53318 | **11.44483** | −2.09 pts |
| %px any-channel Δ > 0 | 97.02947 | **96.26264** | −0.77 pts |
| max \|Δ\| | 255 | 255 | — |

Candidate vs the pinned frame itself (pre-fix native vs post-fix
native, `docs/ab/composer-v2/metrics-native-vs-native.json`):
**PSNR 41.351 dB**, mean |Δ| 0.81309, %px > 10 = 2.08131,
%px > 0 = 36.96291, max |Δ| 30. About 37 % of pixels changed,
almost all by small amounts — the signature of a shade-arithmetic
correction spread over the grass, hills and felt surfaces, not a
staging change. Every A/B metric moved *toward* the Chrome
reference, the expected direction: the native shades now agree
with the engine's own linear-space derivation instead of sitting
systematically dark of it. This is a **deliberate-change** STALE
(cause measured and named), not drift.

## The re-pin procedure (human act — exact)

Run from the repo root, on a clean tree at a HEAD that includes
both `c00897d` and the composer-v2 pack:

1. **Confirm the starting state.**
   ```sh
   python3 tools/golden/make_golden.py
   ```
   Expect `ab1-composer OK` (pin and on-disk file are both the
   pre-fix bytes) and no STALE items. Also confirm
   `docs/golden/frames/` holds no staged-but-unpinned film frames
   you do not intend to pin in the same act — `--pin` records the
   hash of *every* reference currently on disk, so anything staged
   on disk gets pinned too. Scope stays visible: this packet
   authorises exactly one hash to move.
2. **Stage the candidate at the golden path.**
   ```sh
   cp docs/ab/composer-v2/native.png docs/ab/composer/native.png
   sha256sum docs/ab/composer/native.png
   # must print a415ead80a1a17d2b190c03c237bd2f8129057a8688ebbfa31cd5d26ffceeee9
   ```
   (The pre-fix bytes are not lost: they remain in git history and
   in the M5 record via the pin `d7bd71a0…`.)
3. **Expect the honest intermediate state.**
   ```sh
   python3 tools/golden/make_golden.py
   ```
   `ab1-composer` now reads **STALE** (exit 1). That is the gate
   doing its job between staging and pinning — not a failure of
   the candidate.
4. **Pin.**
   ```sh
   python3 tools/golden/make_golden.py --pin
   ```
5. **Verify the scope of what moved.**
   ```sh
   python3 tools/golden/make_golden.py          # all four milestone items OK
   git diff tools/golden/golden-hashes.json     # ONLY the ab1-composer line may change:
                                                # d7bd71a0… -> a415ead8…
   ```
   If any other line moved, stop and explain it before committing
   (protocol step 6: scope stays visible).
6. **Commit together**: `tools/golden/golden-hashes.json`, the
   replaced `docs/ab/composer/native.png`, and this packet's
   state update (step 7), with the cause in the commit message —
   suggested text:
   > Re-pin ab1-composer golden: deliberate re-baseline for the
   > shade root-fix c00897d (shade_rgb byte-multiply →
   > THREE.Color linear-space). Measured: candidate (CI run
   > 43569670339, headSha c00897d, CI lavapipe) vs Chrome
   > reference 23.850 → 23.96 dB, mean |Δ| 5.20056 → 4.82505;
   > vs the old pin 41.351 dB, max Δ 30 — a shade correction,
   > not staging drift. Evidence: docs/ab/composer-v2/,
   > docs/golden/repin-packets/ab1-composer.md. Pin
   > d7bd71a0… → a415ead8…; no other golden hash moves.
7. **Close the packet**: after the commit, update this file's
   State line to record the re-pin commit and the new pin, and
   update the index (`README.md` in this directory).

## Disclosure (the text that travels with the re-pin)

> The `ab1-composer` golden reference was re-baselined on
> 2026-11-14 from pin `d7bd71a0…` (pre-fix) to `a415ead8…`
> because commit `c00897d` corrected the shared shade helper
> from sRGB byte-multiplication to the source engine's
> THREE.Color linear-space arithmetic, which the old helper ran
> 25–40 LSB dark of on derived shades. The new reference is the
> post-fix frame rendered by CI run 43569670339 at the fix
> commit itself (CI lavapipe); it measures closer to the Chrome
> reference on every A/B metric (23.96 dB vs 23.850 dB) and
> differs from the old pin only as a shade correction (41.351 dB
> frame-to-frame, max Δ 30, no staging change). The pre-fix
> frame and its measurement remain on record in
> `docs/ab/composer/` (M5 pack) and `docs/ab/composer-v2/`.
