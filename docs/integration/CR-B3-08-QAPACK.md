# CR-B3-08 — QA-pack golden re-strike (native)

Seat B3-08, chrome-replacement Batch 3. Full analysis + runnable procedure:
`~/workspace/chrome-replacement/batch3/B3-08/` (`ANALYSIS.md`,
`restrike_qapack.py`, `selftest-output.txt`). Nothing committed; the shipped
`qa-pack.json` was not edited; no hash in this note was invented — every MD5
below was computed from a file on disk during the run.

## Verdict

**Procedure delivered and proven; the strike itself is blocked on exactly
one input** — a native bridge of the golden project (below). No staged
`qa-pack.json` is delivered, because the only honest source for one does
not exist yet.

## What the golden is

`qa_visual.py pack-verify` MD5s exactly one file per pack entry: the sheet
JPEG from `node tools/capture_native.mjs sheet review/all.jpg 0.5 9` in the
entry's project (`run_sheet_md5` → `md5_file`; code quoted in ANALYSIS.md §1).
Pinned entry `golden-template`: project
`~/workspace/tall-tale-v5-work/golden-proj` ("The Girl Who Was Mailed To
Grandma", 51.723 s), retired Chrome golden `c535024d05b0d268186af4a4f7103642`
— **verified live**: the Chrome sheet still on disk at
`golden-proj/review/all.jpg` hashes to exactly that value, 1976×4652 px
(103 samples at t = 0.25 + k·0.5, 9 cols × 12 rows of 216×384 tiles, 4 px
gaps — the native sheet contract preserves this geometry exactly).

The native MD5 cannot equal the Chrome MD5 for three independent reasons:
renderer pixels differ (film A/B mean 23.02 dB; Onoda sheet A/B **22.47 dB**
per CR-B1-04 — reproduced this run from the parity-b1 evidence sheets:
native `e275aa5bf248a6476b416901634f20b5` vs Chrome
`ece77b78b64ccfae4a5909c2dd94bb0f`, both 1316×1936, PSNR **22.4675 dB** by
this seat's computation), per-tile JPEG generation differs (Chrome canvas
q0.8 vs native ffmpeg `-q:v 2` from clean PNG), and the label font differs
(Chrome generic sans vs sheet_mode's sans-first pick — for the golden
project, `Inter-Black.otf` from its own fonts dir). The golden is a function
of (film data, binary + backend, ffmpeg build, font, step, cols); the
re-strike provenance records all of them, and the strike must happen in the
environment class that will verify it (the hash ends in two ffmpeg JPEG
encodes; lavapipe vs RADV pixels may also differ).

## The one input still needed

> A native bridge of `golden-proj` (retrofit: `tools/capture_native.mjs` +
> `native_modes/` in the project, `native/` film data with manifest
> `durationS 51.723`, 1552 frames @ 30 fps, 1080×1920), rendered **twice**
> on a Vulkan host (CI lavapipe per the census/render workflow pattern, or
> the RADV PC), each run with a **fresh `NATIVE_FRAMES_DIR`**, as
> `sheet review/all.jpg 0.5 9` — plus the Hard-Law-7 human look.

Why it doesn't exist: the project has never been bridged (no `native/`, no
adapter in its `tools/` — only Chrome `capture.mjs`/`capture.ts`); Batch 3's
B3-02 seat has produced no outputs on disk yet (batch3 holds only B3-09/
B3-10); the retrofitted Onoda sheets are a different film at step 2 / cols 6
and were **not** used — striking this golden from them would be fabrication.
Until the bridge exists, pack-verify cannot even reach the MD5 comparison:
the project's missing `capture_native.mjs` makes the entry report **ERROR**
(node exit 1), and the record-only `spike-fox` entry (also unbridged) ERRORs
the same way — `verify_pack` fails the run on any exception.

## The procedure (delivered, selftest 15/15)

`restrike_qapack.py`: `--render` (two captures, per-run fresh frame cache)
or `--sheet-a/--sheet-b` (CI-produced sheets) → determinism gate (two MD5s
byte-identical; basis: the census protocol's repeated 1,811/1,811
byte-identical frame sets) → PSNR vs the retired Chrome sheet (auto-detected
while it still hashes to the retired golden; dimension mismatch refuses) →
staged `qa-pack.json` + `qa-pack.restrike-provenance.json` under the seat's
`staged/`, written **only** under `--confirm-reviewed` (Hard Law 7 enforced;
otherwise candidate-only, exit 3), then stub-verified through qa_visual's
own `verify_pack`. Selftest proves every leg offline, including reproducing
22.47 dB and leaving the shipped pack untouched.

## Determinism-leg audit (qa_visual, native mode) — findings for the lead

- **D1 — shipped test suite is red by one stale assertion.**
  `tests/27/qa_visual.test.py:35` asserts `frame_time("t6.5.png") is None`;
  the lead's `FRAME_RE` .png fix invalidated it. Run this run: **27 passed,
  1 failed**, the failure is exactly that line. Fix: assert
  `frame_time("t6.50.png") == 6.5` instead. (Named, not edited — tests are
  outside this seat's charter.)
- **D2 — the leg's independence is defeated by the shared frame cache.**
  Both "independent" runs go through `renderRanges`/`renderIndices`, which
  skip already-valid frames in `renders/native-frames`; run 2 copies run
  1's files, so PASS is guaranteed by construction (the parity-b1 sheet log
  shows `0 rendered, 1 skipped (resume)` per frame). Chrome-era runs
  rendered fresh each time. Fix: distinct `NATIVE_FRAMES_DIR` per run —
  the adapter honours it and `_run_capture` inherits the environment.
  `restrike_qapack.py --render` already does this for sheets. Renderer
  determinism itself remains census-proven; what D2 weakens is this leg's
  value as *ongoing* evidence.
- **D3 — capture outdirs are never cleaned.** `capture_frames` scans the
  whole run dir: a stale Chrome-era `t*.jpg` and a native `t*.png` at the
  same time collide on one dict key with `os.listdir`-order deciding the
  winner, and dropped `--times` values keep being compared from stale
  files. Requested-frame failures still fail loudly (framesMode +
  `_run_capture`), so the residual is extra/stale entries, not silent
  passes on missing frames.
- **D4 — sheet audit exit-1 swallows the observed MD5 in pack-verify.**
  sheet_mode sets exit 1 when the post-tile telemetry audit is unavailable
  *although the sheet was written* (kept by lead ruling); `_run_capture`
  raises on any non-zero exit, so `run_sheet_md5` never hashes and the
  entry reports ERROR with no observed value. Suggest: pack/determinism
  captures pass `--no-audit`, or `run_sheet_md5` hashes a freshly written
  sheet despite the exit code (the re-strike script does the latter).
- Holds: `FRAME_RE`/naming (`t<2dp>.png`) ✓, `compare_runs` union +
  missing-fails ✓, `sheet+` on PNG frames ✓, 1/30 s grid snapping is
  identical across runs (filename keeps requested t — ≤1/60 s label
  offset for trace consumers, not a leg break).

## Bottom line for integration

1. Bridge the golden project (Batch-2 retrofit tooling lineage), render the
   sheet pair on CI lavapipe, run `restrike_qapack.py --render
   --confirm-reviewed` after a human look, and copy the staged pack in as a
   reviewed step. Until then pack-verify's golden entry is ERROR, not FAIL.
2. Land D1's one-line test fix and D2's per-run `NATIVE_FRAMES_DIR` with
   whichever qa_visual touch comes next; D4 if pack-verify should surface
   observed MD5s on audit-less machines.
