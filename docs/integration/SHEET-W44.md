# SHEET-W44 — contact sheet for native frame dirs (`tools/film/make_sheet.py`)

Wave 2, Batch 5 (gap-fix F5). New files only; nothing committed by
the seat. The skill's most-used QA surface is the contact sheet, and
native renders today are loose PNGs — this tool gives a native
frame dir the same surface, with the skill's own tiling recipe.

## Deliverables

- `tools/film/make_sheet.py` — the tool (stdlib only; shells to
  ffmpeg for tiling, as the skill does).
- `tools/film/fixtures/sheet/make_fixtures.py` — generates the
  synthetic fixture frames (zlib/struct PNG writer; no binary
  renders).
- `tools/film/fixtures/sheet/frames/` — 90 frames, 108×192,
  `frame_00000.png`…`frame_00089.png` (3.00 s at 30 fps, 360 KB
  total). Each frame's index is encoded in its colour
  (R = n, G = 255−n, B = 2n mod 256) plus a white 8×8 marker block
  at (10·(n mod 10), 0).
- `tools/film/fixtures/sheet/run_tests.py` — the verification
  below, re-runnable.

## Usage

```
python3 tools/film/make_sheet.py FRAMES_DIR OUT.jpg
    [--step 0.5] [--cols 9] [--scale 216x384] [--from 0] [--to SECONDS]
```

Defaults are the capture-sheet defaults (step 0.5 s, cols 9, tile
216x384). `--scale` also accepts a bare width (`--scale 108`), in
which case the tile height is derived from the first sampled
frame's aspect and rounded to even. Exit codes, house style:
**0** sheet written · **1** a sampled frame is missing or ffmpeg
failed (no sheet; a partial output is removed) · **2** input error
(bad arguments, unreadable/empty dir, empty sample window, or
ffmpeg absent — one-line `ERROR:` on stderr; the ffmpeg check is
`shutil.which`, done before any other work).

## Frame selection — the driver's arithmetic

Shared with `rerender_plan.py`, not invented here:

- fps = 30 (`src/film.rs` hardcodes it); frame *n* is evaluated at
  t = n/30 and is on screen for [n/30, (n+1)/30).
- Sample times are t = step, 2·step, 3·step, … while t <
  duration, where duration = (highest frame index present + 1)/30.
- The frame at time t is ⌊t·30⌋, snapping to the nearest integer
  when t·30 is within 1e-6 of it (the planner's boundary-exact
  tolerance, so a 1-ulp float difference cannot flip a boundary
  frame). Sub-frame steps de-duplicate by frame, in time order.
- `--from`/`--to` keep samples with from ≤ t < to (to defaults to
  duration, clamped to it).

## Tiling — the skill's recipe, and its verified geometry

Sampled frames are staged as an `f%04d.png` sequence in a temp dir
(symlinks, copy fallback), then exactly capture.ts's invocation
shape (docs/audit/a2-capture.md §3.4):

```
ffmpeg -framerate 1 -i f%04d.png -vf \
  "scale=<W>:<H>,tile=<cols>x<rows>:padding=4:color=black" \
  -frames:v 1 -q:v 3 OUT.jpg
```

rows = ⌈samples/cols⌉; ffmpeg pads an incomplete final row with
the padding colour. Spawn failure and non-zero exit are checked
and reported separately, as the skill's `spawnSync` code does.

**Tile geometry, measured** (ffmpeg 8.1.2, ffprobe): in the
`tile=` filter `padding` is the *inner* gap between tiles and
there is no outer border, so sheet W = cols·tileW + (cols−1)·4,
H = rows·tileH + (rows−1)·4. An earlier draft of the test assumed
an outer border (expected 278x104) and failed against the real
286x96; the filter string is the skill's either way — only the
test's expectation was wrong, and the correction is recorded in
`run_tests.py`'s header.

## Verification (all first-hand, this machine)

Fixture test suite — dimensions via **ffprobe**, tile count,
order and identity via an **ffmpeg rawvideo decode** of the sheet
(each tile's centre pixel must match its source frame's encoded
colour within JPEG tolerance 28; observed centre R values
14/29/43/59/73 for frames 15/30/45/60/75 — JPEG YUV shift ≈1–2):

```
$ python3 tools/film/fixtures/sheet/run_tests.py
PASS  A exit 0  (rc=0)
PASS  A dims 286x96  (got (286, 96))
PASS  A tiles are frames 15/30/45/60/75 in order  (centres R=[14, 29, 43, 59, 73])
PASS  A success line
PASS  B defaults exit 0, dims 1976x384  (rc=0, dims=(1976, 384))
PASS  C missing frame exit 1  (rc=1)
PASS  C frame named
PASS  C no sheet left
PASS  D no-ffmpeg exit 2 + ERROR line  (rc=2)
PASS  E window frames 30/45 only, dims 112x96  (rc=0, dims=(112, 96))
OVERALL: PASS (10/10 checks)
```

Case A is an exact 5×1 fill (step 0.5 over 3.00 s, cols 5); B is
the all-defaults invocation (5 tiles in a 9×1 grid, 4 padded
cells); C deletes `frame_00030.png` from a copy and samples it via
step 1; D removes ffmpeg from PATH; E checks the `--from 1 --to 2`
window selects frames 30/45 only.

Direct runs, verbatim:

```
$ python3 tools/film/make_sheet.py tools/film/fixtures/sheet/frames /tmp/sheet-demo.jpg --step 0.5 --cols 5 --scale 54x96
sheet: tools/film/fixtures/sheet/frames — 90 frames present (0..89), duration 3.00 s
samples (5): t=0.5s→frame 15, t=1s→frame 30, t=1.5s→frame 45, t=2s→frame 60, t=2.5s→frame 75
tiling: scale=54x96 tile=5x1 padding=4 color=black q:v 3
/tmp/sheet-demo.jpg (5 frames, every 0.5 s)
SHEET OK: /tmp/sheet-demo.jpg (5 tiles in 5x1, 54x96 each)
exit=0
$ ffprobe -v error -select_streams v:0 -show_entries stream=width,height -of csv=p=0 /tmp/sheet-demo.jpg
286,96
```

A hole that is *not* sampled is a NOTE, not a failure — and never
silent (`frame_00007.png` deleted; step 1 samples frames 30/60):

```
sheet: /tmp/sheet-holes — 89 frames present (0..89), duration 3.00 s
NOTE: 1 non-sampled frame(s) absent from the dir: frame_00007.png
samples (2): t=1s→frame 30, t=2s→frame 60
...
SHEET OK: /tmp/sheet-holes.jpg (2 tiles in 3x1, 54x96 each)
exit=0
```

A hole that *is* sampled is a hard failure naming the frame, and
leaves no sheet behind (frame_00030 also deleted):

```
NOTE: 1 non-sampled frame(s) absent from the dir: frame_00007.png
MISSING frame_00030.png (sample at t=1.00 s)
SHEET: FAILED (1 sampled frame(s) missing — no sheet written)
exit=1
```

## How this differs from the skill's Chrome sheet — honestly

The Chrome sheet (capture.ts sheet mode) does more than tile:

- **It audits.** After tiling it re-runs the overflow/composition
  audit at the sheet's own sample times (report-only, except its
  completeness rule). **This tool performs no overflow, edge-touch,
  continuity, composition or caption audit of any kind** — native
  frames carry no scene graph to audit against; the QA gate for
  native films remains `--film-telemetry`. The one QA act shared
  with the Chrome sheet is completeness: there, samples lost to a
  throw make the audit print INCOMPLETE and exit 1; here, a missing
  sampled frame fails the run by name.
- **It labels.** Chrome burns a time label into each tile
  (`FRAME(t, 0.8, label=true)`) and offers `--guides` (thirds grid
  + caption-zone box). This tool tiles the raw native frames: no
  labels, no guides. The sample list it prints (t→frame) is the
  index you read the sheet with.
- **Sampling offset.** Chrome samples mid-step
  (t = step/2 + k·step); this tool samples on-step (t = k·step)
  per the film tooling's driver-arithmetic convention above. For
  the same step the two sheets show frames half a step apart —
  stated so nobody diffs them tile-for-tile expecting identity.

## Composition with finish.sh and the re-render planner

The review loop this slots into:

1. `rerender_plan.py OLD NEW` prices a data change in frame
   ranges. Delete those ranges, resume the driver — only they
   re-render.
2. `make_sheet.py FRAMES_DIR review.jpg --from <range start>/30
   --to <range end>/30` — eyeball exactly the re-rendered region
   from the PNGs, before paying for an encode. A whole-film sheet
   (defaults) is the cheap full review; because sampled-frame
   holes fail by name, a sheet that prints SHEET OK also certifies
   the sampled frames all exist.
3. `film/finish.sh FRAMES_DIR data/mix.wav out.mp4` — the encode
   consumes the same `frame_%05d.png` dir unchanged (x264 slow
   CRF 18, BT.709, faststart). The sheet is a pre-finish gate,
   never an input to it.

tools-ci hookup (for the lead, one step): run
`tools/film/fixtures/sheet/run_tests.py` after the existing film
validator step; it needs ffmpeg+ffprobe on the runner (already
required by the film tooling) and takes ≈8 s here, most of it the
all-defaults 1976x384 case.
