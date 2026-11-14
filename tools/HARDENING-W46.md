# HARDENING-W46 — adversarial robustness pass over the wave's Python tools

Seat W46. Scope: `tools/film/validate_film.py`,
`tools/captions/gen_caption_strips.py`, `tools/ab/measure_ab.py`,
`tools/golden/make_golden.py` — only these four files were edited,
plus this log. Nothing committed; `src/` and all data untouched.

## Method

Each tool was probed in /tmp sandboxes with adversarial inputs:
truncated/invalid JSON, missing referenced files, empty dirs,
wrong-dimension PNGs, a manifest with a tiling gap, paths with
spaces, wrong-shaped JSON (list instead of object, int where a list
belongs), non-image and body-truncated image files, output paths
that are existing files, and directories standing where files
belong. A probe counts as a **crash** when the tool dies with a
Python traceback on bad *input*; as **PASS** when it already
produces a report or a named non-zero exit.

Fix contract, identical across all four tools: bad input →
**one line on stderr** — `ERROR: <tool>: <what is wrong, naming the
file/field>` — and **exit 2**, no traceback. Rules, metrics, output
formats and behaviour on valid inputs are unchanged; every fix is
an input-validation wrapper around the original code path, and each
tool's docstring was extended with the exit-2 contract.

## Byte-identical proof (valid inputs, before vs after)

Known-good runs on the repo's real data, captured before the edits
and re-run after; after a VM replacement wiped /tmp mid-task, the
whole proof was repeated in the stronger form **pristine
`git show HEAD:<tool>` vs fixed tool, same inputs, same output
paths**, with identical results:

| tool | known-good run | proof |
|---|---|---|
| validate_film.py | repo root, real film data — 7/7 PASS, exit 0 | stdout byte-identical (orig vs fixed) |
| gen_caption_strips.py | real Onoda project, `--out` to scratch — 66 chunks, 67 files | `diff -r` of all 67 output files byte-identical (orig vs fixed); stdout byte-identical (before/after capture) |
| measure_ab.py (pair) | `docs/ab/composer/original.png` vs `native.png` | stdout + `ab-report.json` + `ab-report.md` + images byte-identical; PSNR 23.85 dB reproduced by both versions (matches the committed M5 metrics) |
| measure_ab.py (film) | real `scenes/film/film.json` + the project's 60 review frames (native side = the same frames re-saved as `frame_{n:05d}.png`, W29's plumbing set — 60/60 pairs, all PSNR ∞, per-segment counts 6/7/3/5/4/1/4/4/6/2/3/6/5/4) | stdout + all outputs byte-identical (orig vs fixed) |
| make_golden.py | repo root — 4 OK / 0 STALE / 14 MISSING, exit 0 | stdout byte-identical in verify and `--json` modes (orig vs fixed, run from equal-footing trees because the tool derives its root from `__file__`) |

## Per-tool results

### validate_film.py — 13 probes: 6 crashes fixed, 7 already clean

Already clean (report + exit 1, or exit 0): valid tree; valid tree
under a path with spaces; manifest with a frame-tiling gap (check 6
names it); a segment file missing (check 1 names it); a segment
file truncated mid-JSON (check 1 parse failure, verbatim error);
empty `segments` with `totalFrames: 0` (full report, exit 1 on the
caption/atmosphere checks).

Fixed (traceback → named error, exit 2 unless noted):

| probe | before | after |
|---|---|---|
| manifest file missing (`--root` with no data) | `FileNotFoundError` traceback | `ERROR: validate_film.py: film manifest: cannot read <path> (No such file or directory)`, exit 2 |
| manifest truncated JSON | `JSONDecodeError` traceback | `… film manifest: invalid JSON in <path> (Unterminated string …)`, exit 2 |
| atmospheres.json truncated | `JSONDecodeError` traceback | `… atmospheres: invalid JSON in <path> (…)`, exit 2 |
| manifest missing `fps` | `KeyError: 'fps'` | `… film manifest: 'fps' must be a number`, exit 2 |
| segment entry missing `scene` | `KeyError: 'scene'` | `… segments[0] 'scene' must be a string`, exit 2 |
| segment entry missing `frames` | `KeyError: 'frames'` | `… segments[1] 'frames' must be a number`, exit 2 |
| segment file parses to a JSON list, not an object | `AttributeError` in check 2 | folded into check 1 as a parse failure (`top level is list, not a JSON object`), report + exit 1 |
| segment file with `"attachments": 5` | `TypeError: object of type 'int' has no len()` in check 4 | check 4 violation line reports `(not a list (int))`, report + exit 1 |

The manifest schema gate (object shape + the string/number fields
every check dereferences: `title`, `project`, `fps`,
`totalFrames`, per-segment `id`/`scene`/`set`/`atmosphere`/
`filmLook`/`startS`/`endS`/`startFrame`/`endFrame`/`frames`, and
`filmLooks` as an object) runs before any check, so no later
`KeyError`/`TypeError` can escape from manifest-shaped input.

### gen_caption_strips.py — 8 probes: 8 crashes fixed

All eight died with tracebacks before; all now exit 2 with a named
line:

| probe | before → after |
|---|---|
| project dir does not exist | `FileNotFoundError` → `ERROR: gen_caption_strips.py: cannot read script <path> (No such file or directory)` |
| script.json truncated | `JSONDecodeError` → `… invalid JSON in script <path> (…)` |
| data/words.json truncated | `JSONDecodeError` → `… invalid JSON in words data <path> (…)` |
| data/lines.json missing `duration` | `KeyError: 'duration'` → `… lines data <path>: requires a 'lines' list and a 'duration'` |
| words.json is `[]` (builds zero chunks) | `ValueError: min() iterable argument is empty` at the summary → `… no caption chunks built — data/words.json yields no chunks for script.json's lines` |
| font file missing | `OSError: cannot open resource` → `… cannot open font <path> (cannot open resource)` |
| `--font` points at a non-font file | `OSError: unknown file format` → `… cannot open font <path> (unknown file format)` |
| `--out` is an existing file | `FileExistsError` from `mkdir` → `… cannot create output dir <path> (File exists)` |

Field-level gaps inside otherwise-shaped project data (a word
entry missing `s`/`w`/`line`, a line missing `end`) are caught
around `build_captions()` and reported as
`project data is missing or mistypes a required field (<KeyError/TypeError/IndexError repr>)`.

### measure_ab.py — 19 probes: 9 crashes fixed, 10 already clean

Already clean: wrong-dimension native (measured with the documented
resize note, exit 0); identical pair (exit 0); empty review dir
(named ERROR, exit 2); empty native dir (full report with the
missing list + exit 2); no mode given (argparse usage error,
exit 2); a PNG missing only its IEND chunk (PIL tolerates it —
loads and measures; recorded as tolerated, not an error class).

Fixed (traceback → `ERROR: measure_ab.py: …`, exit 2):

| probe | crash class → named error |
|---|---|
| pair: reference file missing | `FileNotFoundError` → `cannot read image <path> ([Errno 2] …)` |
| pair: reference is a text file / empty file | `UnidentifiedImageError` → `cannot read image <path> (cannot identify image file …)` |
| pair: header-truncated PNG | `UnidentifiedImageError` → same line |
| pair: mid-IDAT truncated PNG (opens, fails at decode) | `OSError: image file is truncated` → `cannot read image <path> (image file is truncated)` |
| pair: reference path is a directory | `IsADirectoryError` → `cannot read image <path> ([Errno 21] …)` |
| pair: `--out` is an existing file | `FileExistsError` → `cannot create output dir <path> (File exists)` |
| film: `--native-dir`/`--review-dir` missing | `FileNotFoundError` from `os.listdir` → `cannot list native frames dir <path> (No such file or directory)` |
| film: manifest missing / truncated | `FileNotFoundError` / `JSONDecodeError` → `cannot read film manifest …` / `invalid JSON in film manifest …` |
| film: manifest without `segments`; segment without `endS` | `KeyError` → `film manifest <path>: 'segments' must be a list` / `segments[0] requires 'id', 'startS' and 'endS'` |
| film: corrupt PNG inside the native dir | `UnidentifiedImageError` mid-run → `cannot read image <path> (…)` naming the exact frame file |

Every image load now funnels through `load_rgb()`'s wrapper, so
both modes and both passes (metrics + worst-3 imaging) share the
same named failure.

### make_golden.py — 7 probes: 5 crashes fixed, 2 already clean

Already clean: missing reference files (MISSING, exit 0); a pin
whose bytes differ (STALE, exit 1); `--pin` on a healthy tree
(records hashes, exit 0).

Fixed (traceback → `ERROR: make_golden.py: …`, exit 2):

| probe | crash class → named error |
|---|---|
| golden-hashes.json truncated | `JSONDecodeError` → `golden-hashes.json: cannot load pins (Unterminated string …)` |
| golden-hashes.json is a JSON list | `AttributeError: 'list' object has no attribute 'get'` → `golden-hashes.json: must be a JSON object mapping item id to sha256 hex string` |
| a pin value is an int | `TypeError: 'int' object is not subscriptable` → same shape line |
| a directory stands at a golden item path | `IsADirectoryError` in the hasher → `golden path is not a file: docs/ab/dof-chart/native.png` (verify and `--pin` paths both guarded) |
| `--pin` with corrupt pins | `JSONDecodeError` (same load path) → same cannot-load line |

## Out of scope / unchanged

- No features added; no metric, rule, threshold, or valid-input
  output changed anywhere (proven byte-for-byte, above).
- Failures that are not bad input — e.g. a disk filling mid-write
  during a long film-mode report, or the caption renderer failing
  on a font that vanishes between the measure load and a strip
  render — still surface as-is; they are environment faults, not
  input classes, and naming them was not attempted.
