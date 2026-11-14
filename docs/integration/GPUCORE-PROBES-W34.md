# GPUCORE-PROBES-W34 — the byte-probe pack for renderer-change gates

**Seat:** Wave 2 W34. **Delivered:** `tools/ci/byte_probes.py`
(new file, uncommitted — the lead wires/runs it). **Nothing else
touched.** The tool is stdlib-only, verified by its built-in
synthetic self-test (§5) and by two runs over real film frames
(§6), all reproduced verbatim below.

## 1. What this is

The standard same-renderer byte-identity gate to run around
**each step of the gpucore migration** (GPU-CORE-W07: instance
descriptor ×5, device request ×5, target helpers, readback ×7
being unified into `src/gpucore.rs`) and around any future
renderer change — paint-cache wiring (PAINT-BUDGET-W06 §8 item
2 is written in exactly this shape), readback unification,
shader changes. The migration order is **dof → scenefile → ab1
→ main → film**, each step gated on its pin: a step proceeds
only when the frames it can touch are byte-identical before and
after, on the same renderer.

(One record-keeping note: the committed GPU-CORE-W07.md ends at
its §5 heading — the section body is not in the tree. The order
above is as stated in the wave brief and is consistent with
W07 §1–§4; the per-site pins in §3 are assembled from W07
§1–§4, the determinism contract, and the golden/visual-
regression docs.)

The tool compares a **baseline** dir (rendered pre-change) with
a **candidate** dir (rendered post-change) over probe ranges
and prints per-range PASS/FAIL — first differing frame, first
differing byte offset, differing-byte count, and both sha256 —
plus one verdict line. Defaults are the project's standing
determinism probe ranges (DETERMINISM-CONTRACT-W04 §5, E4):

| range | frames | why this range |
|---|---|---|
| 195–204 | 10 | the v1 probe range |
| 795–814 | 20 | mid-film |
| 1195–1204 | 10 | straddles the old 160-entry paint-cache cap; frames 1195–1199 are where the quantised `state_key` defect measured 72.34 dB pre-fix (contract E2) |

Ranges are inclusive at both ends, matching how the evidence
register counts them. House exit codes (same convention as
`tools/ab/measure_ab.py`): **0 = PASS, 1 = FAIL, 2 = input
error / refusal** — one-line `ERROR:` on stderr, no traceback,
no verdict line. A side containing *none* of the probed frames
refuses (exit 2): a crashed or misdirected render is no
evidence, never a soft pass — the determinism pack refuses the
same way. A side missing *some* frames reports them as
differences (FAIL), like the pack's per-frame DIFFER lines.

## 2. Usage

```bash
# film-style ranges (defaults shown)
python3 tools/ci/byte_probes.py BASELINE_DIR CANDIDATE_DIR

# explicit ranges / naming
python3 tools/ci/byte_probes.py BASE CAND \
    --ranges 1195-1204,795-814 --pattern 'frame_{n:05d}.png'

# single still (dof chart, ab1 still): a pattern with no {n}
# probes that one literal file in each dir
python3 tools/ci/byte_probes.py BASE CAND --pattern dof-chart.png

# size a difference after a FAIL (diagnostic only, never the gate)
python3 tools/ci/byte_probes.py BASE CAND --psnr-fallback

# prove the tool itself
python3 tools/ci/byte_probes.py --self-test
```

Standing protocol for every gate, whichever site:

1. Render the baseline with the **pre-change** binary into a
   **fresh** dir (fresh matters: the film driver skips frames
   that already exist, so a reused dir can fake identity).
2. Make exactly one step's change; build the post-change
   binary.
3. Render the candidate with it — **same machine, same
   renderer**, same command line / env, into a fresh dir.
4. Run the probes. Proceed only on `BYTE PROBES: PASS`
   (exit 0). On FAIL, stop: the report names the first
   differing frame; diagnose with `--psnr-fallback` and
   `tools/ab/measure_ab.py`. Never widen the comparison to
   make a FAIL go away (contract §8).
5. Record with the step's commit: the probe output, the
   binary sha256, and the render's `renderer:` line — the
   contract §6 citation discipline, so the PASS is always
   attributable to an exact binary on an exact renderer.

## 3. Per-step gates (W07 migration order)

| step | baseline (pre-migration render) | probe | verdict needed before proceeding |
|---|---|---|---|
| **1. dof** | The DoF chart output PNG(s) + packed-depth dump from the un-migrated `run_dof` | literal `--pattern` on the chart file(s) | PASS. The one intended non-pixel change at this step is invisible to probes and must be checked by eye in the diff: dof's panic message `"request device"` → canonical `"request_device failed"` (W07 §4.3). |
| **2. scenefile** | Scene-file render set, above all the **fx-textures** outputs — the tree's one byte-sensitive consumer (W07 §4.7) | literal `--pattern` per pinned output, or the scene set's frame pattern | PASS, fx-textures explicitly included. This step carries the `image::save_buffer` → unified-save spelling change; PNG bytes are exactly what this probe exists to clear. |
| **3. ab1** | The ab1 golden-pin frame(s) at the current head | the pin's frame(s) via `--ranges`/`--pattern` | PASS against the pin lineage. (W29's observation stands: ab1's `Vec::with_capacity(304)` is a stale hint for the 448 B FrameU — not pixel-relevant, not this gate's business.) |
| **4. main** | The M1 smoke-scene frame from un-migrated `run()` | literal `--pattern` | PASS. Migration must pass `downlevel_defaults()` explicitly (W07 §1/§4.4) — the probe can't see limits, but a wrong-limits build fails or changes the frame, and either shows up here or in CI. |
| **5. film** | Film frames at the three default ranges, default cache, from un-migrated `Renderer::new` | defaults, all three ranges | **PASS 40/40**, plus the separate telemetry gate unchanged (`FILM TELEMETRY:` verdict line — probes do not replace it). Film is last because its one-submit batching is load-bearing (W07 §4.3): a readback split into a second submit is a hot-path change, and this gate is how the lead proves it didn't also become a pixel change. |

**Paint-budget wiring instance** (PAINT-BUDGET-W06 §8 item 2 —
the same shape, kept here as the standing pattern for cache
changes): pre/post wiring, ranges **1195–1204 and 795–814**
only, **two legs** — (a) both sides at defaults, (b) candidate
under forced eviction `TALERENDER_PAINT_BUDGET_MB=100`
(~3-canvas budget, exercises the byte condition, not just the
entry cap). Both legs must print PASS. The wiring itself has
since landed (lead, per GATE2-W30); this pack is the instrument
for the next cache change, and for re-running that verification
on demand.

## 4. `--psnr-fallback`

For every differing frame the tool also decodes both PNGs and
reports PSNR in the house definition — `10*log10(255²/MSE)`,
MSE over all decoded channels, float64 (as
`tools/ab/measure_ab.py`). It exists to *size* a difference the
lead has already decided to investigate: the contract's E2
defect measured 72.34 dB — nearly invisible, still a FAIL.

The decoder is stdlib (zlib + unfiltering) and deliberately
small: bit depth 8, colour types 0/2/4/6, non-interlaced — the
subset the image crate writes, proven below on real
1080×1920 film frames. Anything outside the subset
(interlaced, 16-bit, palette) prints `PSNR unavailable
(<reason>)` and the byte-diff data stands; differing geometry
also reports unavailable rather than inventing a number. PSNR
**never** affects the verdict or the exit code.

## 5. Self-test — verbatim output

`python3 tools/ci/byte_probes.py --self-test` (temp dirs;
A: identical dirs → PASS; B: one planted byte, frame 1197
offset 1234 → FAIL naming exactly that frame and offset,
other ranges unaffected; C: one removed frame → FAIL naming
it missing; D: an empty side → refusal; E: the PSNR path
against a hand-computed value):

```
self-test A: identical dirs -> expect PASS, exit 0
byte probes: baseline=/tmp/byte-probes-selftest-0l2i_tco/base candidate=/tmp/byte-probes-selftest-0l2i_tco/cand pattern=frame_{n:05d}.png
range 195-204: PASS (10/10 identical)
range 795-814: PASS (20/20 identical)
range 1195-1204: PASS (10/10 identical)
BYTE PROBES: PASS (40/40 frames identical across 3 range(s))
self-test: identical dirs exit 0: ok
self-test: identical dirs verdict PASS: ok
self-test B: flip byte 1234 of candidate frame 1197 -> expect FAIL, exit 1, range 1195-1204 only
byte probes: baseline=/tmp/byte-probes-selftest-0l2i_tco/base candidate=/tmp/byte-probes-selftest-0l2i_tco/cand pattern=frame_{n:05d}.png
range 195-204: PASS (10/10 identical)
range 795-814: PASS (20/20 identical)
range 1195-1204: FAIL (9/10 identical, 1 differ)
  first difference: frame 1197 (frame_01197.png) at byte 1234 (1 byte differs; sizes 8192 baseline / 8192 candidate)
    sha256 baseline  b18e5707eee85d18d410cde9b626de138a20b82bcc72a1feebfadf1530e0cc11
    sha256 candidate 7f77e61d9c1fb177712c50aa40812fa7009ef699ee0dfc34f30bfd2328b0a543
BYTE PROBES: FAIL (1 frame(s) differ in 1 of 3 range(s))
self-test: planted byte exit 1: ok
self-test: planted byte names frame 1197 at byte 1234: ok
self-test: unaffected ranges still PASS: ok
self-test: affected range FAILs: ok
self-test C: remove candidate frame 801 -> expect FAIL naming it missing
byte probes: baseline=/tmp/byte-probes-selftest-0l2i_tco/base candidate=/tmp/byte-probes-selftest-0l2i_tco/cand pattern=frame_{n:05d}.png
range 195-204: PASS (10/10 identical)
range 795-814: FAIL (19/20 identical, 1 differ)
  first difference: frame 801 (frame_00801.png): missing in candidate
range 1195-1204: PASS (10/10 identical)
BYTE PROBES: FAIL (1 frame(s) differ in 1 of 3 range(s))
self-test: missing frame exit 1: ok
self-test: missing frame named: ok
self-test D: candidate dir with no probed frames -> expect refusal (InputError, exit 2)
ERROR: candidate dir /tmp/byte-probes-selftest-0l2i_tco/empty contains none of the 40 probed frames (pattern 'frame_{n:05d}.png') — render missing or wrong dir/pattern; refusing, this is no evidence
self-test: empty side refused: ok
self-test E: PSNR path — 4x2 RGBA PNGs, one sample differs by 16 -> MSE 8, expect PSNR 39.10 dB
self-test: computed PSNR 39.0999 dB, expected 39.0999 dB
self-test: PSNR matches hand computation: ok
self-test: identical PNGs give inf PSNR: ok
SELF-TEST: PASS
```

(The temp-dir suffix in the paths is per-run; everything else
is stable across runs — the stub bytes are deterministic, so
the sha256 values above reproduce exactly.)

## 6. Runs over real film frames — verbatim

Real frames from the Wave-1 final render set
(`~/workspace/film-final/frames4/`, the `f021991` film, 1,811
frames at 1080×1920). A directory probed against itself — the
PASS direction on real data:

```
$ python3 tools/ci/byte_probes.py ~/workspace/film-final/frames4 ~/workspace/film-final/frames4
byte probes: baseline=/home/hatch/workspace/film-final/frames4 candidate=/home/hatch/workspace/film-final/frames4 pattern=frame_{n:05d}.png
range 195-204: PASS (10/10 identical)
range 795-814: PASS (20/20 identical)
range 1195-1204: PASS (10/10 identical)
BYTE PROBES: PASS (40/40 frames identical across 3 range(s))
exit=0
```

And the PSNR path exercised on a genuinely differing real
pair — frame 1197 from the previous render set (`frames3/`)
against `frames4/`. This pair crosses build changes, so a
difference is expected; the run's purpose is proving the
stdlib decoder handles real full-size PNGs and sizes the
difference. It is **not** a migration verdict:

```
$ python3 tools/ci/byte_probes.py --ranges 1197 --psnr-fallback /tmp/w34-a /tmp/w34-b
byte probes: baseline=/tmp/w34-a candidate=/tmp/w34-b pattern=frame_{n:05d}.png
range 1197: FAIL (0/1 identical, 1 differ)
  first difference: frame 1197 (frame_01197.png) at byte 34 (2927497 bytes differ; sizes 2960543 baseline / 2946722 candidate)
    sha256 baseline  0e464d67ecc4063d683f03f10d7c9bd3561b3eb1270bf530bee2c63da6911b2f
    sha256 candidate e3463ceeec275999f5c018e944e003d76994bfc5ac85cc71fc74c1bfaa682b1f
  frame 1197 (frame_01197.png): PSNR 26.69 dB (--psnr-fallback; diagnostic only)
BYTE PROBES: FAIL (1 frame(s) differ in 1 of 1 range(s))
exit=1
```

## 7. Honest limits

1. **Probes prove no-change, not correctness.** A
   byte-identical wrong frame passes. Correctness lives in
   the milestone A/B evidence and the telemetry gate; this
   tool only certifies that a change did not move pixels it
   wasn't supposed to move.
2. **Same-renderer baselines only — never mix lavapipe and
   RADV.** Cross-vendor bit-identity is physically not
   promised (contract §2: the GRADE grain's fp32 `sin()`,
   fp16 packing and JIT rounding differ by vendor). A
   lavapipe baseline probed against a RADV candidate will
   FAIL, and that FAIL is expected physics, not a migration
   defect. Each renderer of record keeps its own baselines
   (contract §4); cross-renderer questions go to
   `tools/ab/measure_ab.py` and its PSNR bands.
3. **Sampling, not census.** The default ranges cover 40 of
   the film's 1,811 frames, chosen at known-sensitive
   boundaries (the cache-cap straddle above all). A change
   whose effect lands only outside the ranges escapes. For a
   broad change, widen `--ranges` — up to `0-1810` for a full
   census — and pay the I/O.
4. **Byte offsets are file offsets, not pixel locations.**
   PNG is a compressed stream: almost any pixel change
   diverges the stream near its start (the real run above
   reports byte 34 for a whole-frame content change). The
   offset's job is reproducing the defect-era evidence style
   and confirming *that* bytes moved; PSNR sizes the move and
   `measure_ab.py` localises it.
5. **Provenance is on the operator.** The tool compares
   files; it cannot tell whether the two dirs really came
   from the claimed binaries on the claimed renderer. That
   is why §2 step 5 records binary sha256 + `renderer:` line
   with every gate result.
6. **PSNR subset.** The fallback decoder covers what the
   image crate writes (8-bit, colour types 0/2/4/6,
   non-interlaced). Outside that it says so and stops; the
   byte verdict never depends on it.

