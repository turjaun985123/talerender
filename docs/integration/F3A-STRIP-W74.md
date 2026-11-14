# F3A-STRIP-W74 — gap-fix F3, ratchet first lowering: the F3-a strip pack

Seat W74, Wave 2 Batch 8. Deliverables, new files only, nothing
committed, neither live file edited:

- this note — the consumption audit, the equivalence proof, the
  ratchet arithmetic, the atomic landing order
- `docs/integration/f3a-strip/onoda.json` — the stripped driver
  data, staged drop-in for `film/onoda.json`
- `docs/integration/f3a-strip/unknown_fields_baseline.json` —
  the lowered ratchet baseline, staged drop-in for
  `tools/format/unknown_fields_baseline.json`
- the `film/gen_film_data.py` patch as a fenced diff in §4
  (the generator stops emitting the dead keys in the same
  landing, or regeneration restores them)

Verified at HEAD `b2e46d2`. All runs first-hand; scratch work in
`~/workspace/.w74-scratch/` (outside the repo).

## 0. HEADLINE — two of the eight F3-a paths are NOT dead: `fps` and `frames` stay

The brief expected a 196 → 22 lowering. The audit found readers
for root `fps` and root `frames`, so per the seat law both are
**removed from the strip list**. The strip is 6 paths / 172
occurrences; the baseline lowers **196 → 24** (9 paths), not 22.

STRICTNESS-W42 §6's claim "read by nobody" was true of the Rust
loader it surveyed (`load_film_data` hardcodes 30 fps and never
touches either key) but not of the Python tooling:

| key | reader | strength |
|---|---|---|
| `fps` | `tools/perf/blink_cost.py:351` — `fps = onoda["fps"]` | HARD (KeyError) |
| `fps` | `tools/perf/blink_census.py:169` — `fps = onoda["fps"]` | HARD (KeyError) |
| `fps` | `tools/lipsync/evaluate_track.py:274` — `args.fps or film.get("fps", 30)` | defaulted |
| `fps` | `tools/film/rerender_plan.py:237-242` — `Snapshot.fps`: manifest first, driver fallback | fallback |
| `frames` | `tools/perf/blink_cost.py:359` — cross-check `sum(seg frames) != onoda["frames"]` | HARD |
| `frames` | `tools/perf/blink_census.py:179` — same cross-check | HARD |
| `frames` | `tools/lipsync/evaluate_track.py:275-279` — `film.get("frames")`; **exit 2 "film data lacks frames/duration"** if absent | HARD |
| `frames` | `tools/film/rerender_plan.py:250-251` — `total_frames` fallback after manifest/`duration` | fallback |

`fps`/`frames` are checker-unknown only because the checker's
film-data KNOWN set mirrors the Rust loader's vocabulary
(STRICTNESS-W42 §3). Their disposition is a **checker fix**
(teach the film-data set that `fps`/`frames` are tool-consumed,
or add them to the tolerated register with these citations) —
a separate landing, not a data strip. In the staged baseline
they are re-classed `F3-a-kept` with the citations inline, so
the ratchet keeps watching them at 1 occurrence each.

## 1. The strip list (final)

| path | key | occ. | producer | disposition |
|---|---|---|---|---|
| `film/onoda.json $` | `film` | 1 | gen_film_data.py `out` literal | STRIP |
| `film/onoda.json $` | `project` | 1 | gen_film_data.py `out` literal | STRIP |
| `film/onoda.json $.lines[]` | `i` | 12 | lines comprehension | STRIP |
| `film/onoda.json $.lines[]` | `text` | 12 | lines comprehension | STRIP |
| `film/onoda.json $.captions[]` | `line` | 73 | caps construction | STRIP |
| `film/onoda.json $.captions[]` | `width` | 73 | caps width pass | STRIP |
| `film/onoda.json $` | `fps` | 1 | `out` literal | **KEEP — readers (§0)** |
| `film/onoda.json $` | `frames` | 1 | `out` literal | **KEEP — readers (§0)** |

Total stripped: **172 occurrences, 6 paths.**

## 2. Safety proof (a) — field-by-field consumption audit

Repo-wide grep for readers of `film/onoda.json`
(`src/`, `tools/`, `film/`, `.github/`), then per-reader key
extraction. Every consumer of the file, and exactly what it
takes:

| reader | root keys read | lines[] keys | captions[] keys |
|---|---|---|---|
| `src/film.rs` `load_film_data` (film.rs:69-141) | duration, anchors, calendarYears, calendarDark, sign (+ §15 guard probes prefabs/clips) | start, end | s, e, text, fit, strip |
| `film/gen_assets.py:33,171` | captions | — | text, strip |
| `tools/format/check_assets.py:400-452` | calendarYears, calendarDark, sign | — | — |
| `tools/bridge/package_bridge.py` (validateFilmData) | duration (>0), lines (non-empty), anchors (present), captions (non-empty); the file is also copied verbatim into bridge packages (line 173) | — | — |
| `tools/film/locomotion_census.py:259` | anchors | — | — |
| `tools/film/rerender_plan.py` | duration, **fps (fallback), frames (fallback)**; captions via chunk tuples; generic whole-value diff of all other root keys | (whole-value compare only) | text, s, e, fit (tuples) |
| `tools/perf/blink_cost.py:350-360` | **fps, frames**, anchors | — | — |
| `tools/perf/blink_census.py:168-180` | **fps, frames**, anchors | — | — |
| `tools/lipsync/evaluate_track.py:274-282` | **fps (defaulted), frames (required)**, duration, anchors | start | — |
| `tools/format/check_unknown_fields.py` | the checker itself (the instrument, not a consumer) | | |
| `tools/format/check_format15.py` (CI passes the file) | no film-data keys — §15 guard surface only | — | — |
| `tools/determinism/determinism_check.sh:89` | existence only | — | — |

No other file in the repo opens the driver data. Checked and
cleared as non-readers: `tools/captions/gen_caption_strips.py`
(its `line`/`width` vocabulary is its own output dialect built
from the *project's* `data/lines.json`/`words.json`/`script.json`
— it never opens onoda.json; it is the historical source of the
transplant shapes, not a consumer), `tools/film/validate_film.py`
(mentions the file in a comment only), `tools/golden/*` and
`tools/ci/vr_triage.py` (the name `s28-onoda-t150` only),
`tools/format/staging/s12-baked/bake_s12.py` (`onoda` is a Rust
variable name in quoted source).

Per stripped key, the negative: no reader above references
root `film`/`project`, lines `i`/`text`, or captions
`line`/`width` in any form (subscript, `.get`, or whole-entry
use) — the only whole-value consumer, rerender_plan's
outside-captions diff, is analysed in §5.

## 3. Safety proof (b) — regeneration equivalence

`film/gen_film_data.py` is the file's only producer, and it
reproduces the committed file exactly today, which makes the
equivalence byte-level rather than field-level:

```
$ python3 film/gen_film_data.py ~/faceless-shorts/the-soldier-who-didnt-know <scratch>/regen-unpatched.json
wrote ...: 73 caption chunks, 17 anchors, duration 60.357
$ cmp film/onoda.json <scratch>/regen-unpatched.json
BYTE-IDENTICAL
```

Then, in scratch (live files untouched):

```
$ python3 <scratch>/gen_film_data_patched.py <project> <scratch>/regen-stripped.json
$ <strip transform applied to the live file>  ->  <scratch>/staged-onoda.json
$ cmp staged-onoda.json regen-stripped.json
BYTE-IDENTICAL        (parsed-deep-equal also verified)
```

So the staged data in `f3a-strip/onoda.json` is simultaneously
(a) the live file minus exactly the 6 stripped paths and
(b) the patched generator's exact output. After the landing,
regenerating the film data is a no-op diff, as it is today.

## 4. The generator patch (fenced diff, for the landing commit)

```diff
--- a/film/gen_film_data.py
+++ b/film/gen_film_data.py
@@ -87,13 +87,18 @@
     c["fit"] = round(min(1.0, 960.0 / w), 5)
     c["strip"] = f"cap_{k:03d}.png"
 
+# F3-a strip (W74): captions[] "line"/"width" were generator
+# bookkeeping read by no consumer — the loader takes s/e/text/
+# fit/strip, gen_assets.py takes text/strip. "line" has done its
+# work above (the e-window pass); it is not emitted.
+caps = [{k: v for k, v in c.items() if k not in ("line", "width")}
+        for c in caps]
+
 out = {
-    "film": "THE SOLDIER WHO DIDN'T KNOW THE WAR WAS OVER",
-    "project": str(PROJ),
     "duration": T_DUR,
     "fps": 30,
     "frames": int(T_DUR * 30) + 1,
-    "lines": [{"i": l["i"], "start": l["start"], "end": l["end"], "text": l["text"]} for l in LINES],
+    "lines": [{"start": l["start"], "end": l["end"]} for l in LINES],
     "anchors": anchors,
     "captions": caps,
```

Design notes: `fps`/`frames` emissions are **kept** (§0).
Captions are filtered *after* the end-window pass because the
generator itself consumes `c["line"]` internally while building
(`LINES[c["line"]]["end"]`, next-chunk line comparison) — the
filter is the last step before `out`, so construction behaviour
is bit-for-bit unchanged (proven by §3: output equals the
strip of today's output exactly). Key order in emitted entries
is preserved (`s, e, text, fit, strip`; lines `start, end`;
root order unchanged apart from the two removed literals).

## 5. Safety proof (c) — ratchet arithmetic + live-reader runs

Corpus run (the tools-ci command verbatim) against a scratch
tree whose only change is the stripped `film/onoda.json`:

```
OVERALL: FAIL (3/5 checks passed, 24 unknown-key occurrence(s)
in 9 distinct path(s), 7043 tolerated occurrence(s))
```

(FAIL is the checker's standing state — the ratchet, not the
checker exit, is the gate. Check 3 still names `fps`/`frames`;
that is the F3-a-kept residue, expected.)

Remaining 9 paths: `fps` (1) + `frames` (1) [F3-a-kept];
F3-b — s10 `motifLaw` (1), s10 textures `note` (2), s11 nodes
`note` (1), s12 nodes `note` (1), bookshop textures `note` (14)
= 19; F3-c — bookshop `lights.key.near` (1), bookshop
`nodes[].materials` (2) = 3. **2 + 19 + 3 = 24.** ✓

The workflow's own ratchet comparison, run locally:

- vs the **current** baseline (196): `RATCHET: PASS`, with
  exactly the 6 stripped paths reported SHRANK 1→0 / 12→0 /
  73→0 — no NEW, no GREW.
- vs the **staged** baseline (24): `RATCHET: PASS`, zero
  shrinkage — the stable post-landing state.

Live readers executed against the stripped data (scratch tree):

- `tools/perf/blink_cost.py --root <tree>` →
  `[PASS] data: 14 segments, 1811 frames, fps=30, all 7 beat
  anchors inside their segments` (its own fps/frames cross-check
  passing on the retained keys).
- `tools/perf/blink_census.py --root <tree>` → full census
  produced, data checks PASS.
- `tools/lipsync/evaluate_track.py --film-data <staged>
  --words <project>/data/words.json` → exit 0;
  `[1 inputs] … film frames=1811 fps=30 duration=60.357`.
- `tools/film/rerender_plan.py <repo> <tree>` → **zero caption
  chunk diffs** (the (text, s, e, fit) tuples are untouched) and
  exactly one driver-data finding, the expected one-time note:
  `driver data changed outside captions (keys: film, lines,
  project)` — the planner's whole-value diff naming the strip
  surface. Any pre/post-strip plan comparison says this once;
  post-landing comparisons are clean. (The same run's 73
  "asset cap-*.png changed" lines are a snapshot-copy artifact:
  a control run with the *pristine* file in the copied tree
  emits them identically; strip PNG bytes verified unchanged
  with `cmp`.)
- Access replay of the remaining consumers against the staged
  file (film.rs's demanded keys + per-entry gets, gen_assets,
  check_assets, package_bridge's validateFilmData checks,
  locomotion_census anchors): **7/7 PASS**; stripped keys
  verified absent.

## 6. Atomicity — the landing is ONE commit, in this order

The ratchet compares the corpus against the baseline in the
same checkout, so any partial landing goes red: data stripped
without the baseline lowering fails on nothing (shrinkage
passes — safe direction), but baseline lowered without the
data strip fails NEW/GREW in reverse (run 196 > baseline 24 →
TOTAL grew + per-path GREW), and data stripped without the
generator patch is silently undone by the next regeneration.
One commit, four files:

1. `film/onoda.json` ← `docs/integration/f3a-strip/onoda.json`
2. `film/gen_film_data.py` ← apply the §4 diff
3. `tools/format/unknown_fields_baseline.json` ←
   `docs/integration/f3a-strip/unknown_fields_baseline.json`
4. `docs/integration/STRICTNESS-W42.md` §6 register — add the
   disposition line (F3-a stripped per F3A-STRIP-W74; fps/frames
   re-classed F3-a-kept with tooling citations). Doc-only, but
   the baseline's `_about` cites the register, so land it in
   the same commit.

Immediately post-landing, in the same PR: tools-ci strictness
step green at 24/9 (checker exit 1 with only the 9 baselined
paths is the ratchet's PASS state), and a regeneration no-op
check (`python3 film/gen_film_data.py` → `git diff` empty).

Follow-ups this pack deliberately does NOT do: the checker
known-set fix for `fps`/`frames` (would zero F3-a-kept and
lower the baseline to 22/7 — a checker landing with its own
fixtures); F3-b citing sentences / strip decision; F3-c
`materials` disposition. Those remain the fixes-phase queue
exactly as STRICTNESS-W42 §7 left them.
