# RED TEAM — seat W70: the wave's headline claims, re-run first-hand

**Date:** 2026-11-14 · **Tree:** repo HEAD `ce06011` (Batch 6 integrated) ·
work copy `~/workspace/talerender-skill-wiring` as-is.
**Method:** every claim re-executed by this seat from the current tree —
no report taken on trust. Verdicts: **REPRODUCED** (my run matches the
claim), **FAILED-TO-REPRODUCE** (my run could not be made to match),
**REFUTED** (my run shows the claim, as stated, is false for this tree).

**Score: 7 REPRODUCED · 0 FAILED-TO-REPRODUCE · 1 REFUTED (claim 2, at
current HEAD — see its section; the seat's original measurement was real,
a later deliberate lead fix invalidated one assertion).**

---

## Claim 1 — "Validator 7/7" → **REPRODUCED**

`python3 tools/film/validate_film.py` → exit 0, verbatim tail:

```
[1 parse] PASS — 14 present, 0 missing, 0 unparsable
[2 version] PASS — 14 files checked, 0 wrong
[3 f32] PASS — 1389 numbers checked: 883 exact f32, 506 source literals (pass),
       0 computed values not round-tripping (fail)
[4 attachments] PASS — 0 top-level violations, 10 attachments on character nodes
[5 captions] PASS — 66 chunk listings, 66 unique chunks covered of 66,
       0 duplicates, 0 uncovered
[6 durations] PASS — sum(durations) = 60.357 s -> 1811 frames; tiling exact
[7 atmosphere] PASS — R5 presets/filmLooks resolve for 14/14
OVERALL: PASS (7/7 checks passed)
```

## Claim 2 — "Painter tests 52/52" → **REFUTED at current HEAD (51/52)**

Harness rebuilt exactly per `painters/TEST-RUN.md` (scratch crate in
`/tmp/w70`, real `src/scene.rs`/`src/math.rs` copies, `planks.rs`
appended to scene.rs, the six other painters as modules, the wgpu stub
rlib carrying only `COPY_BYTES_PER_ROW_ALIGNMENT = 256`; rustc 1.99.1):

```
test result: FAILED. 51 passed; 1 failed; 0 ignored; finished in 152.60 s
failures: felt::tests::shade_is_the_three_linear_multiply
```

Failure detail (single-test re-run):

```
thread 'felt::tests::shade_is_the_three_linear_multiply' panicked at src/felt.rs:136:9:
assertion `left != right` failed
  left: (28, 83, 64)
 right: (28, 83, 64)
```

**Root cause — the test pinned a divergence the lead deliberately
abolished.** The test asserts (a) felt's own `shade_three("#1F5A45",
0.85) == (28, 83, 64)` — still true — and (b) that this **differs** from
`scene::shade_rgb("#1F5A45", 0.85)`. When W23/W39 ran, `shade_rgb` was
the sRGB byte-multiply (26, 77, 59), so (b) held. The lead's root fix
`c00897d` ("shade_rgb: THREE.Color linear-space multiply… every texture
painter route through") redefined `shade_rgb` to the same linear
arithmetic, so it now returns (28, 83, 64) and the `assert_ne!` fails.
The painter is not wrong — its pinned value still matches the source —
and the other 51 tests (including the remaining shade witnesses in
planks/wallpaper/sign) all pass. What is broken is one cross-check
assertion whose premise ("the registry helper still byte-multiplies")
the root fix removed on purpose.

So: W39's 52/52 was an accurate measurement of the tree it ran
against; as a present-tense claim about HEAD `ce06011` it no longer
holds. Closing it is an integrator act (update the assertion to pin
the post-fix agreement instead of the divergence), not a painter fix.
This is the red team's one substantive catch.

## Claim 3 — "Bridge packager 21/21 + adapter accepts its output" → **REPRODUCED**

```
python3 tools/bridge/package_bridge.py --repo . --project /tmp/bridge-rt
→ packaged 94 files … VERIFICATION PASSED — 21/21 checks, exit 0
```

Then the real adapter against that package with the repo's stale
binary (the only binary on this VM):

```
NATIVE_DIR=/tmp/bridge-rt/proj/native node …/capture_native.mjs check
→ banner + "film: THE SOLDIER WHO DIDN'T KNOW THE WAR WAS OVER —
   duration 60.36 s · 1811 frames @ 30 fps · 1080x1920"   (printed only
   after manifest + film-data validation passed)
stderr: unknown argument: --film-telemetry
        "film telemetry produced no verdict line (binary exit 2) —
         treated as a FAILED gate"
→ exit 1
```

Data validation accepted the package; the gate failed closed on the
stale binary, exactly as claimed.

## Claim 4 — "Caption strips deterministic byte-identical" → **REPRODUCED**

```
python3 tools/captions/gen_caption_strips.py \
  --project ~/faceless-shorts/the-soldier-who-didnt-know --out /tmp/caps-rt
→ "CAPTION STRIPS: 66 chunks, min fit 0.92, max rendered width 960 px,
   7 over the 850 px floor, 0 under the 0.68 fit floor", exit 0
diff -r /tmp/caps-rt assets/captions → no output (byte-identical,
   manifest + all 66 strips)
```

## Claim 5 — "W57: post-fix ab1 = 23.96 dB" → **REPRODUCED**

Re-measured from the staged repo files with `tools/ab/measure_ab.py
--pair` (reference `docs/ab/composer/original.png`):

| pair | committed metrics.json | my re-measurement |
|---|---|---|
| original vs `composer-v2/native.png` | 23.96 dB · mean \|Δ\| 4.82505 · %px>10 11.44483 · %any>0 96.26264 · max Δ 255 | **identical on every field** |
| pre-fix `composer/native.png` vs `composer-v2/native.png` | 41.351 dB · 0.81309 · 2.08131 · 36.96291 · max Δ 30 | **identical on every field** |

The staged frame's sha256 is `a415ead8…`, the hash W57 reported from
the CI artifact — the staged bytes are the measured bytes.

## Claim 6 — "Adapter has no fail-open path" → **REPRODUCED, with the one documented exception (sheet)**

Parser read first-hand (`capture_native.mjs:166-177`): the FAILED
prefix is tested **before** the OK line; OK requires an exact full-line
`FILM TELEMETRY: CHECK OK` (lines are `trimEnd()`ed, so CRLF from a
real binary is handled); silence, errors and FAILED all fail the gate.
`frames` (:284) and `video` (:367) both call `gateOrStop` before any
render call. My own attempts to defeat it, all against the real bridge
data:

| attempt | result |
|---|---|
| `frames` + stale real binary + **planted valid 1080×1920 PNG cache** | exit 1 at the gate; no `review/frames` created; nothing copied — the cache does not bypass the gate |
| `check` with `TALERENDER_BIN=/bin/true` (real executable, exit 0, total silence) | "no verdict line (binary exit 0) — treated as a FAILED gate", exit 1 |
| `frames` with a binary printing `FILM TELEMETRY: CHECK FAILED (…)` and exiting 0 | gate stops the capture ("no flag to render past a failed gate"), exit 1, nothing created |
| `check`/`frames`/`video` with the stale binary (no verdict line) | fail-closed, exit 1 (claim 3 transcript) |

No construction I tried renders or copies a frame in the gated modes
without a genuine OK verdict line.

**The exception, stated plainly:** `sheet` mode (:338) calls
`renderRanges` with **no** telemetry gate — by design. It announces
this in its own output ("the gate is `capture_native.mjs check`"), it
produces only a preview-grade contact sheet (it cannot stage review
frames or finish a video), and W44 recorded it as finding F2 in the
same report that made this claim. The claim therefore holds as the
wave stated it — gated modes are fail-closed — but anyone quoting the
headline without the F2 carve-out overstates it by one mode.

## Claim 7 — "Work copy drift = 0" → **REPRODUCED**

```
diff -rq ~/workspace/custom-skills/tall-tale-shorts ~/workspace/talerender-skill-wiring
→ 0 "differ" lines. Only-in entries: docs/ and tools/ (new skill-root
  dirs) and template/tools/{CAPTURE-NATIVE.md, capture_native.mjs}.
File counts: shipped 1663, copy 1671 — the 8 extras are exactly the
7 wave additions (capture_native.mjs, CAPTURE-NATIVE.md,
selftest_native.sh, SELFTEST-NATIVE.md, NATIVE-BACKEND.md,
NATIVE-BACKEND-SKILL-PATCH.md, DRY-RUN-W44.md) + W59's own
ADOPTION-REVIEW-W59.md, written by the reviewing seat after its diff.
```

Zero common files differ; the copy is the shipped skill plus the
wave's additions and nothing else.

## Claim 8 — "grade.time = 0 in all 14 files" + "s06 shot.at = 26.129" → **REPRODUCED**

Read from the files directly: `post.grade.time` is `0`/`0.0` in all 14
segment files (s01–s12 incl. s05a/s05b/s08b). s06:
`shot.at = 26.129`, `shot.duration = 4.261000156402588` (the exact f32
of 4.261) — the W51 fix is in the tree as claimed (and the validator's
[3 f32] pass above covers its round-trip).

---

## Summary for the integrator

- 7 of 8 headline claims reproduce first-hand on the current tree.
- The one refutation is a *stale test assertion*, not a product
  defect: `painters/felt.rs`'s `shade_is_the_three_linear_multiply`
  still pins the pre-`c00897d` divergence between `shade_three` and
  `scene::shade_rgb`. Post-fix the two agree at (28, 83, 64) — which
  is the source's value. Update the assertion to pin the agreement
  (and keep the (28, 83, 64) pin, which still passes); until then any
  "painter tests" run against HEAD reads **51/52**, and batch reports
  quoting 52/52 should carry the date/tree they measured.
- The adapter gate claim should always travel with its sheet-mode
  (F2) carve-out.
