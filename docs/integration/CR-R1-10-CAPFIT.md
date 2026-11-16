# R1-10 NOTE — caption `fit`: semantics, measurement, bridge patch

Seat R1-10, remediation Batch 1, Package 4 (F4). All findings from
code at talerender HEAD `9743850` + the L0 landing; file:line
references are to that tree. Deliverables in this directory:
`measure_caption_fit.py` (tool), `MEASUREMENT-REPORT.md` +
`measurement.json` (both films, per-chunk), 
`build_bridge_caption_fit.patch` (patch candidate, tested).

## 1. What `fit` MEANS — as found in code

`fit` is **not an authored scale factor**. It is a pure function of
the chunk text: the shrink-to-fit ratio that keeps a caption inside
the design cap. The definition lives in the Chrome-era renderer
(`tools/legacy/fog-chart-chrome/project/src/core/captions.ts`):

```ts
export const CAP = { y: 1340, size: 88, maxW: 960 };            // :15
const FONT = `900 ${CAP.size}px Inter`;
function measure(text) {                                        // :20
  const w = ctx.measureText(text).width;
  return { w, fit: Math.min(1, CAP.maxW / w) };
}
```

and its audit twin states it in words (:61-62): *"rawW = width at
88 px unscaled; fit = min(1, CAP.maxW / rawW); renderedW = rawW ×
fit (peak width; the 0.86→1 pop-in ramp is ignored)"*. In Chrome,
`captionText()` drew the text scaled by `sc * fit`, so `fit` was
simultaneously the render scale and the audit's input.

The native binary keeps both roles:

- **Loader (required, no default)** — `src/film.rs:118`:
  `fit: jnum(c.get("fit").ok_or("cap.fit")?, "cap.fit")?` — a film
  data file without a numeric `fit` per caption fails to load, by
  name. Any bridge design must always write a number (see §4).
- **Render** — `eval_frame`, `src/film.rs:2213-2214`:
  `f.caption = Some(ci); f.caption_scale = (sc * c.fit) as f32;`
  where `sc` is the 0.86→1 pop-in ramp over the chunk's first
  0.1 s. `overlay_caption` (film.rs:3688) then samples the strip —
  which is drawn at scale 1 (`film/gen_assets.py:167-171`:
  *"caption strips … captions.ts captionText at scale 1 (the fit
  and the pop-in scale are applied at composite time)"*) — at
  `scale = caption_scale`. So a wrong `fit` is a wrong on-screen
  size, not just a wrong audit number.
- **Audit** — the binary's telemetry does **not** use a font
  measurer at all. `run_film_telemetry` (film.rs, caption block
  ~:4069-4096) measures the strip itself:
  *"Caption audit: rendered width = ink bbox of the film's own
  strip; raw width = rendered / fit"* — the ink bbox is the span
  of strip pixels with `alpha > 32`, and the resulting
  `CaptionInfo` carries `rendered_w: rendered` (the UNSCALED ink
  width) and `raw_w: rendered / c.fit`. The floors are
  `telemetry.rs:1094-1095` (`print_caption_audit`, strict):
  `let wide = c.rendered_w > 850.0; let low_fit = c.fit < 0.68;`

Two consequences worth the lead's attention:

1. **The native audit's "rendered width" is not Chrome's.** Chrome:
   `renderedW = rawW * fit` (≤ 960 by construction; a WIDTH fail
   means the chunk is wide even after shrinking). Native:
   `rendered_w = ink width of the unscaled strip` (≈ rawW + stroke/
   shadow, +4…+15 px on these films). A chunk with fit < 1 reads
   ~960 in Chrome's audit and ~its rawW (≈1040) in the native one.
   Both fail the 850 bar, so verdicts agree on every chunk in both
   films — but the printed numbers are not interchangeable, and
   R1-02's generic telemetry (contracts §3: "widths computed by the
   same estimator the legacy audit uses") inherits the strip-ink
   estimator, not the font one. That is per contract; flagged here
   so nobody "fixes" it silently.
2. **`fit` is the only audit input the strip cannot supply.** The
   ink scan measures width honestly from the artefact; `fit` is
   taken on trust from the data (`fit: c.fit` verbatim). That is
   exactly the F4 hole: an assumed `fit` passes through the audit
   unexamined, and also mis-scales the render (above).

Elsewhere in the skill: `template/tools/chunk_caps.py` is a
stdlib chunker — it packs words and merges by display TIME; it has
no font measurer and never touches `fit`. The only font measurers
in the ecosystem are the talerender-side PIL tools:
`film/gen_film_data.py` (`font = ImageFont.truetype(…/Inter-Black.otf, 88)`,
`c["fit"] = round(min(1.0, 960.0 / w), 5)`) and
`tools/captions/gen_caption_strips.py` (same recipe; its header
declares the PIL-vs-Skia caveat this seat quantified).

## 2. Measurer validation against the reference film (Onoda)

Onoda's data was the right validator, but not for the naive reason:
its authored fits are **also all 1.0** — yet they are MEASURED
1.0s, written by `gen_film_data.py`'s PIL measurer, whereas the
Lighthouse Cat's are ASSUMED 1.0s written by the bridge. The
measurement (full tables in MEASUREMENT-REPORT.md):

- PIL re-measurement of all 73 Onoda chunks: max rawW **834.8 px**,
  every measured fit **1.0000** → authored fits match measurement
  on **73/73**.
- Against Chrome's own first-party metrics
  (`docs/ab/telemetry/ab1-captions.json`, Skia `measureText`):
  59 chunks match by (text, start); **max |PIL − Skia| rawW delta
  = 0.1 px; fit agrees on 59/59**. The 14 unmatched chunks are
  re-chunked splits of ab1-era longer chunks (ab1: 66 chunks, 5
  with fit < 1, e.g. "was a Japanese soldier" rawW 1023.9 /
  fit 0.938 — today split in two). So the PIL measurer is not an
  approximation that happens to be close on short chunks: it
  reproduces Skia to a tenth of a pixel across the full width
  range present, including 801–835 px chunks.
- Strip-ink cross-check: Onoda max ink 844 px (< 850), consistent
  with the established `FILM TELEMETRY: CHECK OK` for this film;
  ink runs +5.6…+15.0 px over PIL rawW (stroke 8 + shadow offset
  in gen_assets.py's recipe) — a systematic bias of the audit's
  estimator, reported in the tables per chunk.

**Font limitation, precisely:** there is none on the font FILE.
`Inter-Black.otf` is md5-identical (`c26b4e77dc103fd3a1e3b426549296ed`)
in the Onoda project, the Lighthouse Cat project, and the skill
template — the file measured here is the production caption font.
The only divergence risk is the MEASURER (FreeType vs Skia),
bounded above at 0.1 px on 59 matched chunks. Residual knife-edge,
named not hidden: no matched chunk has rawW in the 850–1040 band,
so a hypothetical chunk landing within ~0.1 px of the 960 cap
could in principle round its fit differently between measurers;
no chunk in either film is within 120 px of that edge on the
under-cap side (nearest over-cap: Lighthouse idx 3, 1039.7 px,
whose fit 0.9233 is corroborated by ab1-era analogues).

## 3. Headline numbers (both films; full tables in the report)

**Lighthouse Cat — 43 chunks, authored fit 1.0 (assumed) on all:**
- Measured fit differs on **1 chunk**: idx 3 "the lighthouse
  burning." — rawW **1039.7**, fit **0.9233**. True min fit is
  0.9233, not 1.00. No chunk is under the 0.68 FIT floor either way.
- **Verdict changes vs the as-authored native audit: 0.** Three
  chunks are WIDTH fails under every reading (idx 0 rawW 917.9 /
  ink 922; idx 3 rawW 1039.7 / ink 1049; idx 40 rawW 882.6 /
  ink 887) — the native WIDTH test is ink-based and never involved
  the assumed fit. They need chunk splits at re-bridge (R2-03),
  not a fit correction.
- What the assumed 1.0 actually hid, then, is threefold and none
  of it is the WIDTH verdict: (a) the FIT floor was **vacuous** —
  the audit printed "min fit 1.00" for a film whose true min is
  0.9233; had any chunk measured < 0.68, the audit would still
  have passed it; (b) the **render** of idx 3 composites at scale
  1.0 instead of 0.9233 — on screen its ink spans 1049 px instead
  of ~969 px, breaching the 960 design cap `fit` exists to
  enforce; (c) the audit's own diagnostic `raw_w = ink / fit` was
  understated for idx 3 (1049 reported vs 1136 at measured fit).

**Onoda — 73 chunks:** 0 fit differences, 0 fails under any
reading, max rawW 834.8 / max ink 844. The reference distribution
the task brief expected (a spread of fits) does not exist in the
current data — it existed in the ab1-era chunking (fits down to
0.923) and was chunked away. Onoda validates the measurer (§2),
not the bridge.

## 4. Bridge patch design + degradation behaviour

`build_bridge_caption_fit.patch` (applies with `patch -p1` in the
talerender repo; **tested**: applies cleanly to a pristine copy,
compiles, and is byte-identical to the build I smoke-tested):

- `measure_caption_fits()` added beside the captions section:
  PIL/FreeType `getlength()` at 88 px against the resolved font
  (--caption-font → `<project>/fonts/Inter-Black.otf` →
  `$TALERENDER_CAPTION_FONT`), `fit = round(min(1, 960/rawW), 5)`
  — gen_film_data.py's exact recipe and rounding.
- In `build()`: measured fit is written for EVERY chunk. A
  `--film-data` fit survives only as a cross-check — disagreement
  > 0.005 prints a WARNING and the measured value wins (the old
  "carry" behaviour inverted: data no longer outranks measurement).
- The manifest bridge block gains `captionFit`: in measured mode
  `{mode: "measured", measurer, font, fontMd5, capMaxW,
  chunksRevisedVsFilmData}` — provenance a later reader (or the
  validator) can check without re-running anything.
- **Dependency pattern:** the bridge was stdlib-only; the patch
  makes Pillow an OPTIONAL import (the repo's caption tooling —
  gen_film_data.py, gen_caption_strips.py — already hard-requires
  it, so this follows the repo pattern while keeping the bridge
  runnable for diagnosis without it). The header/usage text stops
  claiming the old stdlib excuse.
- **Degradation (Pillow missing or no font resolves):** fail
  CLOSED by default — `die()` with a named error explaining the
  fit floor would be unmeasured. Only with the explicit new
  `--allow-assumed-fit` flag does it fall back to the legacy
  carry-or-1.0 behaviour, and then: a `WARNING: CAPTION FIT
  UNMEASURED` line prints, and the manifest records
  `captionFit = {mode: "assumed-1.0", reason, chunksAssumed}` —
  the assumption is a property of the built artefact, not just of
  the console log. Both paths were smoke-tested (PIL blocked via
  a shadow module): die path names Pillow; flag path built 59
  files with `chunksAssumed: 43` recorded.
- **Loader contract respected, no pairing needed:** `fit` is
  ALWAYS written as a number on every path, so film.rs's
  `cap.fit` requirement (jnum, errors if absent) is untouched.
  **Note for R1-03: do NOT make `fit` optional in the loader** —
  an absent fit would re-open F4 as a silent default; the bridge
  side now guarantees presence. (R1-03's decoupling scope is
  calendarYears/calendarDark/sign only, per contracts §4 — this
  patch is compatible with it and touches none of those keys. It
  also does not add the §1 `driver` field; that is R1-03's hunk,
  in a different part of the file — no textual overlap with this
  patch beyond the shared header comment region, which the lead
  should apply first-come and rebase the other over.)

**Smoke test evidence (patched builder, this VM):** rebuilt the
Lighthouse Cat bridge to a scratch dir from the project's own
native scenes + assets: 59 files, P4 leg PASS, output data has
idx 3 fit **0.92331** and all other chunks 1.0 — exactly the
measurement tool's values; manifest `captionFit.mode =
"measured"`, fontMd5 matches §2.

## 5. Lead wiring spec

1. Apply `build_bridge_caption_fit.patch` in the talerender repo
   (`patch -p1`, or cherry-pick the hunks); run the bridge's own
   verification legs on any existing fixture bridge to confirm no
   regression (leg (a) P4 must stay PASS; the Onoda-shaped build
   should report `chunksRevisedVsFilmData: 0`).
2. Bridge environment needs Pillow (already true wherever
   gen_film_data.py / gen_caption_strips.py run). No binary, loader,
   or telemetry change is required or proposed by this seat.
3. R2-03 (Lighthouse re-bridge) picks up measured fits by using the
   patched builder; expect exactly one non-1.0 fit (idx 3,
   0.92331) and `captionFit.mode: "measured"` in the new manifest.
   The 3 WIDTH-failing chunks (§3) are an authoring fix (split
   chunks in the project's script caps) — the re-bridge will still
   fail the generic telemetry's caption audit on them until split;
   that failure is honest and pre-existing, not a patch regression.
4. Optional, not required: `validate_native_project.py` could flag
   `bridge.captionFit.mode != "measured"` as a named warning —
   suggested to R1-03/R2-10 as a follow-up, not built here.
5. `measure_caption_fit.py` is reusable as-is for any future film:
   `measure_caption_fit.py <film/onoda.json> [--out PREFIX]`
   prints both verdict families and the Skia cross-check when the
   ab1 reference is present.
