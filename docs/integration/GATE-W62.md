# GATE-W62 — Consolidated Verification Gate (seat W62, replacement run)

Re-run of the wave's local verification suite, first-hand, at repo HEAD
`381ff47`. Nothing taken on trust from earlier seats' claims. Each run
records: command, expected, observed verdict lines (verbatim), PASS/FAIL.

| # | Run | Result |
|---|---|---|
| 1 | Film validator (`tools/film/validate_film.py`) | **PASS** |
| 2 | Caption determinism (regen + `diff -r`) | **PASS** |
| 3 | Golden pack verify (`tools/golden/make_golden.py`) | **PASS** |
| 4 | Bridge packager self-verification | **PASS** |
| 5 | Adapter spot check (syntax + fail-closed gate) | **PASS** |
| 6 | Scene JSON parse sweep | **PASS** |
| 7 | Film manifest tiling (independent recompute) | **PASS** |

---

## Run 1 — Film validator

- Command: `python3 tools/film/validate_film.py`
- Expected: 7/7 PASS, exit 0.
- Observed (verbatim verdict lines):

```
[1 parse] PASS — 14 present, 0 missing, 0 unparsable
[2 version] PASS — 14 files checked, 0 wrong
[3 f32] PASS — 1389 numbers checked: 883 exact f32, 506 source literals (pass), 0 computed values not round-tripping (fail)
[4 attachments] PASS — 0 top-level violations, 10 attachments on character nodes
[5 captions] PASS — 66 chunk listings, 66 unique chunks covered of 66, 0 duplicates, 0 uncovered
[6 durations] PASS — sum(durations) = 60.357 s -> 1811 frames (total 1811); tiling exact
[7 atmosphere] PASS — R5: declared presets match the manifest for 14/14 files; (set, preset) -> filmLook resolves via atmospheres.json for 14/14 segments
OVERALL: PASS (7/7 checks passed)
```

- Exit: 0. **PASS**

## Run 2 — Caption determinism

- Command: `python3 tools/captions/gen_caption_strips.py --project ~/faceless-shorts/the-soldier-who-didnt-know --out /tmp/gate-captions` then `diff -r /tmp/gate-captions assets/captions`
- Expected: byte-identical (66 strips + manifest), diff silent, exit 0.
- Observed (verbatim):

```
CAPTION STRIPS: 66 chunks, min fit 0.92, max rendered width 960 px, 7 over the 850 px floor, 0 under the 0.68 fit floor
wrote /tmp/gate-captions/manifest.json + 66 strips to /tmp/gate-captions
GEN_EXIT=0
DIFF: byte-identical
DIFF_EXIT=0
```

- The generator's headline (66 chunks, min fit 0.92, 7 over the floor)
  matches Chrome's own audit figures, and `diff -r` over all 67 files
  (66 strips + manifest) printed nothing. **PASS**

## Run 3 — Golden pack verify

- Command: `python3 tools/golden/make_golden.py`
- Expected: 4 OK / 0 STALE / 14 MISSING. Semantics: OK means the pin
  matches its own committed (pre-shade-fix) file; the STALE expectation
  applies to *re-renders* against post-fix binaries, not to this verify.
- Observed (verbatim):

```
s28-onoda-t150  OK       ff68fad3bbcc6155  ff68fad3bbcc6155
dof-chart       OK       957d5573497e6269  957d5573497e6269
stroke-toon     OK       9ba9ff890402b6cf  9ba9ff890402b6cf
ab1-composer    OK       d7bd71a02a3dfc96  d7bd71a02a3dfc96
film-s01 … film-s12 (14 items)  MISSING
4 OK / 0 STALE / 14 MISSING of 18 golden items
```

- Exit: 0. **PASS** — all 4 pins match their own files; the 14 film
  mid-frames remain MISSING (the film has not been rendered into the
  golden layout yet).

## Run 4 — Bridge packager

- Command: `mkdir -p /tmp/gate-bridge && python3 tools/bridge/package_bridge.py --repo . --project /tmp/gate-bridge`
- Expected: `VERIFICATION PASSED — 21/21 checks`, exit 0.
- Observed (verbatim tail):

```
  [PASS] ceil(onoda.duration * fps) == manifest totalFrames — ceil(60.357 * 30) = 1811 vs 1811
  [PASS] assets/plates.json exists
  [PASS] assets plate/calendar/sign PNGs exist — all 11 present
  [PASS] every caption names a strip file
  [PASS] every caption strip exists in assets/ — all 66 strips present
  [PASS] MANIFEST-SHA exists
  [PASS] MANIFEST-SHA covers every packaged file, hashes match — 94 files verified
package-bridge: VERIFICATION PASSED — 21/21 checks
```

- Exit: 0. **PASS**

## Run 5 — Adapter spot check

- Commands: `node --check ~/workspace/talerender-skill-wiring/template/tools/capture_native.mjs`;
  then the adapter's `check` mode against the /tmp/gate-bridge package
  with `TALERENDER_BIN=~/workspace/talerender/talerender` (the stale
  13 Nov build).
- Expected: syntax clean; check run exits 1 (gate fails closed — the
  stale binary prints no telemetry verdict line) and creates no frames
  or video artifacts.
- Observed:
  - Syntax: `node --check` clean ("SYNTAX OK").
  - Invocation note: the adapter resolves its project ROOT from its own
    path (it expects to live at `<project>/tools/capture_native.mjs`),
    so the packaged `native/` was pointed at it via the absolute
    `NATIVE_DIR=/tmp/gate-bridge/native` env (the adapter's documented
    override).
  - Check run (verbatim):

```
capture-native: backend=native (TaleRender) · binary /home/hatch/workspace/talerender/talerender (via TALERENDER_BIN) · manifest /tmp/gate-bridge/native/film.json
film: THE SOLDIER WHO DIDN'T KNOW THE WAR WAS OVER — duration 60.36 s · 1811 frames @ 30 fps · 1080x1920 · film data /tmp/gate-bridge/native/film
unknown argument: --film-telemetry
capture-native: film telemetry produced no verdict line (binary exit 2) — treated as a FAILED gate. If the binary predates --film-telemetry, use a current CI artifact.
ADAPTER_EXIT=1
```

  - The banner + film header print only after the adapter's manifest +
    film-data pre-validation passes — so the Run-4 package was accepted
    as data, and the gate then failed closed on the stale binary
    exactly as designed. File snapshots of both the package tree and
    the adapter's tree before/after are identical: **no artifacts
    created**.
- **PASS**

## Run 6 — Scene JSON parse sweep

- Command: own one-liner over `scenes/**/*.json`.
- Expected: every file parses.
- Observed (verbatim): `JSON sweep: 32 files parsed, 0 invalid`
- Exit: 0. **PASS**

## Run 7 — Film manifest tiling (independent)

- Command: own Python recompute over `scenes/film/film.json` (not the
  validator).
- Expected: segment frame ranges tile [0, 1811) exactly; caption chunk
  ranges tile chunks 0–65 exactly.
- Observed (verbatim):

```
frames: 14 segments tile [0,1811) contiguous=True, totalFrames field=1811, endFrame last=1811
captions: ranges tile [0,66) contiguous=True, total chunks=66, last chunk index=65
```

- Note: `captionChunks` is a `{first, last, count}` object with an
  inclusive `last`; per-segment counts also verified
  (`count == last - first + 1` for all 14).
- **PASS**

---

## Gate summary

**7/7 PASS** at HEAD `381ff47` (validator 7/7; caption strips
byte-identical; golden 4 OK / 0 STALE / 14 MISSING; bridge 21/21 with
the adapter accepting the package as data and failing closed at the
stale binary's gate; 32/32 scene JSONs parse; manifest tiling exact
by independent recompute). No failures. The standing open item is
unchanged and is not a gate failure: the 14 film golden mid-frames
stay MISSING until the film renders, and a live telemetry PASS still
requires a current CI-artifact binary (none exists on this VM).
