# CR-B3-05 — Parity spot-checks (Chrome Replacement, Batch 3, Seat 5)

Assembles the parity evidence comparing native output against the
skill's Chrome-era reference assets, at recipe/data level. This VM has
no Vulkan ICD (`/etc/vulkan/icd.d` empty), so **no frame was rendered
by this seat**; every claim below is labelled by the level at which it
is proven. Deliverables: `~/workspace/chrome-replacement/batch3/B3-05/`
(`castsheet-parity.md`, `plates-recheck.md`, `sheet-geometry.md`,
`plate-probe-results.json`, `repro-castdiff.py`, `repro-plates.py`).

Batch-3 rendered evidence check, done first and last: at the time of
writing, `~/workspace/chrome-replacement/batch3/` contains only B3-09
(audit text), B3-10 (build logs) and an **empty B3-03 tree** (logs/ +
work/, no files). **B3-02 does not exist.** No CI/B3 rendered output was
available to fold in — the rendered-parity boundary in §4 is therefore
not a formality.

## 1. Castsheet catalog (charter 1) — data-level parity PROVEN for the 41

Programmatic field-by-field diff, TS sources vs the native generator
(`castsheet_native.py`, imported + executed; fresh run 77 items,
0 plate-missing), full table in `castsheet-parity.md`:

- **pose 17/17** — every field (mood, arms, lean, bob, headTilt/Turn,
  look, squash) identical to `core/poses.ts`. One deliberate *type*
  conversion: `exhausted.sweat 1` (TS number) → scene `true`.
- **mood 17/17 READY** (B2-05 set) — state `{mood:id}` exact; the TS
  MOODS list and the generator's list are name- and order-identical
  (19/19).
- **look 5/5** — presets exact incl. heights (kid 1.35 / adult 1.75 /
  tall 1.95 / elder 1.68 + elderStoop state / stocky 1.66).
- **character 2/2 by proxy** — `characters.ts` ADULT_DEF/KID_DEF read
  directly: look + heightM match LOOK_PRESETS adult/kid exactly. The
  native item is a look scene written under a character filename —
  data parity, not kind parity. finn_fox/pip_bird remain blocked.
- Shared staging: BASE_LOOK `#7A5FBF`/`none`, backdrop `#EFE7D6`, floor
  `#DCCBA6`, canvas 1080×1920 — all equal. **Three disclosed
  non-parities**: floor is a perspective 3D quad vs Chrome's flat band
  at FLOOR_Y 1560; figure ≈71% of frame / feet ≈0.83 vs Chrome's
  ≈72% / 0.8125 (camera fovY 42 @ z 3.2 approximating scale 1.35);
  Chrome's painted shadow ellipse (280×40, rgba(60,45,20,0.18)) has no
  scene-level counterpart — the painter's blob shadow substitutes and
  cannot be compared without a render.
- Held context (post-B2-10): arm constants [72,−38]/[14,8] exact;
  PROP_IDS order == TS PROPS order 36/36; generator PLATE_ORIGINS ==
  B2-10 `plate_origins.json` 36/36 (programmatic equality).

## 2. Prop plates (charter 2) — sample of 8 independently re-verified

Expected bboxes derived by this seat from the `toon.ts` recipes
(geometry extremes + stroke half-widths) *before* consulting B2-10's
audit; probes chosen from authored fills; plates measured with PIL.
**8/8 pass**: every bbox edge within 1.13 units of derivation (all but
fish ≤0.5, the systematic PIL exclusive-max convention), **15/16
probes exact hex**, and the 16th (ball @ angle 0) was my own
expectation error — the TS pie slice `arc(5.6…6.9)` wraps past 2π and
covers angle 0 with `#3C6FB6`, which is exactly what the plate
contains. Sample: apple, axe, book, sign, fish, ball, key, umbrella.
All 8 skill-installed plates are sha256-identical to B2-10's copies.
Detail: `plates-recheck.md` + `plate-probe-results.json`.

## 3. Sheet geometry (charter 3) — identical contracts, labels approximate

Both implementations read from source (retired Chrome copy in
`legacy/chrome-capture/tools/capture.mjs`; native `sheet_mode.mjs` /
`castsheet_mode.mjs`). Full tables: `sheet-geometry.md`.

- Film sheet: sample times, defaults (step 0.5, cols 9), tile
  `scale=216:384`, tile filter string, `--guides` filter string, final
  `-q:v 3` — **string-identical**. Size formula W=cols·216+(cols−1)·4,
  H=rows·384+(rows−1)·4 reproduces B1-04's measured 1316×1936 (6×5)
  and 656×772 (3×2) exactly.
- Castsheet: `scale=270:480`, cols 4, padding 4, `-q:v 3` — identical;
  sheet width 1092.
- **Labels are the only non-exact element**: fixed box geometry exact
  (film sheet 250×86; castsheet bar 1080×92), text content/size exact,
  but glyph placement is a drawtext text-top approximation of a canvas
  baseline (film sheet y=8 vs baseline 64; castsheet y=14 vs baseline
  62), and font family is not guaranteed Chrome's sans-serif
  (sheet_mode searches sans-first; castsheet_mode takes the first font
  file alphabetically — weaker).

## 4. The rendered-parity boundary, precisely

**Proven at data/recipe level (this seat + cited seats, code-verified):**
castsheet staging data for all 77 catalog items (§1); plate geometry/
colours for 8/36 plates independently, 36/36 by B2-10's own audit;
sheet/castsheet tile geometry and encode contracts (§3); check/sheet
line-class parity and the Onoda telemetry audit (B1-04, CPU-only —
unaffected by the ICD gap); mood painter constants for puzzled/deadpan
(B2-09 harness probes vs TS numbers).

**Proven only on lavapipe, pre-Batch-2 (inherited, not re-runnable
here):** B1-04's sheet A/B — dimensions identical, PSNR **22.47 dB**;
check diff — shots **11/14**, captions **71/73** identical, 0 text
diffs; B1-03's castsheet renders (look 5/5, Onoda cast 5/5, fixture
cast 3/3). These predate the B2-09 mood port, the B2-10 plates and the
Batch-2 skill migration, so they do **not** cover today's native path
for held items or the two new moods.

**Not proven anywhere yet — needs a render run:**
(a) native castsheet A/B vs the 114 Chrome-era previews for pose/mood/
look (any environment);
(b) held-item renders with B2-10 plates composited (no native held
frame exists in any evidence this seat could find);
(c) puzzled/deadpan rendered frames (harness face probes only);
(d) the §1 staging approximations (floor perspective, ≈71% figure,
blob shadow) sized visually;
(e) film-sheet labels rendered (font/baseline approximation in §3).
**Exact runs that would settle them:** a talerender CI run (lavapipe)
or PC run (RADV) of `castsheet_mode` over the full catalog against a
green binary from current HEAD, A/B'd per item against
`library/previews/` — i.e. precisely the B3-02/B3-03 deliverables,
which do not exist yet — plus one `sheet_mode` run on the Onoda film
for (e). The most recent relevant CI on record is the G2 push
(2026-11-15): run 43742929066 (Tools CI) **success**, run 43742929056
(main CI) **failure** — so even the mood port's CI state is mixed and
should not be quoted as "CI green".

## 5. Unsupported / stale parity claims flagged

1. **B2-09 fidelity additivity line is vacuous as printed.**
   `results-fidelity.txt` records `[PASS] additivity: all original-17
   cases byte-identical — 0 cases compared, 0 mismatches`. A PASS over
   zero comparisons proves nothing. The claim is *rescuable* —
   `results.txt` contains real additivity runs (10 cases compared,
   0 mismatches, repeated per chunk) — but anyone citing the fidelity
   file's line, or a "21/21" that silently includes it, is citing an
   unsupported number. The other fidelity checks (brow/mouth centroids,
   lid/pupil probes, with measured values) are genuinely supported.
2. **"41 natively-ready items" is stale as a current statement**
   (B2-05 snapshot). After B2-09 + B2-10, this seat's fresh generator
   run emits 77/77 catalog items with 0 plate-missing. Any Batch-3
   text repeating 41 as the present count understates data-level
   readiness — while conversely, any text upgrading that to "77
   previews rendered" would be fabrication: 0 of the newly-ready items
   have a native render in evidence.
3. **The skill generator's G2 flag contradicts current painter
   source.** `castsheet_native.py` still stamps puzzled/deadpan scenes
   `nativeMoodFallback: "neutral"` and its header claims 17 painter
   variants; repo `toonfull.rs` at current HEAD has 19. Until the
   generator is updated, its manifest *understates* mood parity.
4. **"114 Chrome-era previews" must not be read as 114 distinct
   current renders**: 29 files share a byte-hash with another item's
   preview and 59 predate their item's source file (mtime heuristic,
   B2-05 manifest, census re-verified this seat: 115 files on disk =
   114 + castsheet.jpg; `LIBRARY OK (479 items)`).
5. **No complete Chrome check transcript exists** (B1-04 §C.4: the
   Chrome run timed out during its audit loop; CAST/audit/verdict
   lines were never captured). Native parity for those line classes
   rests on code + M6 field-identity, not on a transcript diff — any
   note claiming transcript-proven check parity overstates it.
   B1-04 itself discloses this; the flag is against downstream
   summaries, not against B1-04.

## 6. Inherited Batch-1 numbers (as briefed, re-checked against the note)

CR-B1-04-PARITY.md states: check shots 11/14 identical (3 data-level
differences ≤0.24 s / 0.01 s class), captions 71/73 identical with
**0 text differences**, sheet-vs-sheet PSNR 22.47 dB at identical
1316×1936 dimensions. All three figures match the note's §A/§C text as
written; their evidentiary weight is §4 above (lavapipe, pre-Batch-2,
incomplete Chrome transcript).
