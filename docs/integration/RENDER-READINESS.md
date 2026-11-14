# Render-readiness checklist — production integration, final phase

The single checklist the final phase works from. Assembled by seat
W58 (2026-11-14) from the wave's records (WAVE-RECORD, AUDIT-W40,
AUDIT-W42, FIXTURES census, REVIEW-W50, FOG.md §8, painters/INDEX,
PERF §9) with **every claim re-verified against the tree at
writing**:

- Local HEAD `dbf90b9` (Wave batch 5), working tree clean.
- Last **pushed** HEAD `e5860b4`; CI run `43569768582` (push,
  main) **success**. Local batch-5 commits are not yet pushed —
  the push is the wave's close-out step, and the CI verdict on
  `dbf90b9` belongs to that step, not to this checklist.
- Validator re-run at writing: **7/7 PASS** (output §1).
- Golden verify re-run at writing: **4 OK / 0 STALE / 14 MISSING**.

**Read this first — the two lanes.** The film renders through the
lead's driver (`src/film.rs`, staging in `film/onoda.json` +
film.rs's own builders), gated by `--film-telemetry`. The format-v2
scene files are the data bridge: the still loader
(`src/scenefile.rs`), fixtures, the fog chart, and the skill-side
adapter consume them. Items below are labelled by lane —
**[film]** items sit on the 1,811-frame render path itself,
**[loader]** items sit on the scene-file path. Both lanes must be
green for the phase to close, but they fail independently.

---

## 1. Data side — GREEN, with evidence

| # | Item | Evidence (verified at writing) |
|---|---|---|
| 1.1 | Film validator **7/7 PASS** | Re-run at writing, exit 0: parse 14/14; version 14/14; f32 — 1,389 numbers, 883 exact f32 + 506 verified source literals + **0 computed failures**; attachments 0 top-level / 10 on-node; captions 66 listed / 66 unique / 0 dup / 0 uncovered; durations sum 60.357 s → 1,811 frames, tiling exact; atmosphere R5 14/14 presets resolve to filmLooks. Run record: `tools/film/VALIDATION.md` |
| 1.2 | Independent audits: **87/90** (W40: s03/s07/s11) + **162/165** (W42: the other eleven) = 249/255 sampled values recomputed from source | `docs/integration/AUDIT-W40.md`, `AUDIT-W42.md`. Of the 6 mismatches: grade.time ×2 → fixed by W41 (§1.4); s07 cap_039 → **reclassified** by W41 (not a defect: the wave convention is bleed-note-not-listing; s07 already carries the note; the manifest assigns chunk 39 to s06 and the validator forbids a second listing); s06 ×3 → **the one open data item, §1.7** |
| 1.3 | Captions single-owned + notes | Validator check 5 (66/66/0/0); chunks 31/39 (W27), 46/56 (W31) single-owned by the start-time rule; bleed chunks recorded in `captionsNote`. Strips + manifest committed: `assets/captions/manifest.json` = 66 chunks; `film/assets/` carries the strips + plates (assets-ignore accident fixed in `b3f3c0a`) |
| 1.4 | `grade.time` = 0 in **all 14 files** | Census at writing: every `scenes/film/*.scene.json` reads `post.grade.time` 0/0.0. W41 zeroed the 7 carriers (s02, s03, s04, s05a, s05b, s06, s07 — each had carried its own film-t). This mattered: the loader reads the field (`scenefile.rs:976` → `:2051`), so pre-fix stills of 7 segments would have grained differently from Chrome on every frame |
| 1.5 | Manifest state | `scenes/film/film.json`: frame ranges tile [0, 1811) exactly (validator check 6); caption ranges tile chunks 0–65; per-segment start/duration are the recomputed cuts (e.g. s06 startS 26.129 — the manifest was always right where the scene file was not, §1.7) |
| 1.6 | Sets + atmospheres | 3 set fragments committed (`scenes/sets/`: SH 229 meshes, BK 369 incl. 310 books, JG 325 + 900 tufts) with C1–C9 corrections applied to the shot files (W26) and SH C6/C7 completed in s02/s03 (W32); `scenes/atmospheres.json` — 7 presets + 6 merged filmLooks, validator check 7 resolves 14/14. Set *merge-before-load* is a loader-lane convenience (§2.6), not a data gap: shot files inline their sets |
| 1.7 | **OPEN — s06 cut-rounding (seat W51, NOT landed)** | `scenes/film/s06.scene.json` still carries the INVENTORY 2-dp values: `shot.at` **26.13** (correct **26.129** = cut(5)), `shot.duration` **4.260000228881836** (correct **4.261**), fire atTime value **0.6314269304275513** (correct **0.6333623112323646**). One root cause (W42 M1–M3). The validator does **not** catch this class — its tiling check reads the manifest, which is correct; only the audits sample scene-internal `shot.at`. Fix = the three values + ASSUMPTIONS correction note + validator re-run (must stay 7/7) |

**Data-side verdict: GREEN except §1.7** — a three-value fix,
already briefed, blocking nothing else.

---

## 2. Lead side — the wiring list, with acceptance tests

### 2.1 Texture painter arms [loader]

`resolve_texture` (`scenefile.rs:398–429`) at HEAD knows only:
`file`, `grass`, `sky-gradient`, `cloud`, `blob-shadow`,
`onoda-card`, `dof-chart-pattern`. Resolution is **eager** — any
file naming another generator fails whole at load. The film data
names, with no arm today:

- [ ] `planks` (×54 uses), `wallpaper` (×7), `felt` (×4),
  `parcel`, `picture`, `sign` — the six wave painters, staged in
  `painters/*.rs` with paste-in dispatch arms in
  `painters/INDEX.md` (one shared f64 `shade_three` helper
  recommended there; the four implementations' pinned values
  agree on every corpus witness)
- [ ] `solid` — the flat-colour generator (×11 in s10 data; the
  fog chart's basic panels depend on it). One-line arm specified
  in `tools/fog-chart/README.md`

**Acceptance:** `scenes/fixtures/fx-textures.scene.json` loads
(it is the registry census, designed to fail until the arms land —
FIXTURES.md), and the painter test suite passes under CI cargo:
`painters/TEST-RUN.md` records **52/52** in the wave's scratch
harness (41 painter tests + 11 baseline); wiring turns those
into in-tree `cargo test` results, which is the first time they
run under the crate itself.

### 2.2 Fog loader wiring [loader]

The loader never reads the `fog` block (grep at writing: zero
hits in `scenefile.rs`); s06/s07/s11/s12 scene files and
`scenes/fog-chart.scene.json` carry it. The film path's fog is
already wired and uses three's correct smoothstep factor (W28);
FOG.md §8 is the loader wiring spec, no shader change needed.

**Acceptance:** the fog chart (`scenes/fog-chart.scene.json` vs
its no-fog control) reproduces `tools/fog-chart/expected.json`
panel centres within **±2 sRGB codes** (factors 0 / 0.0263 /
0.2884 / 0.7550 / 1.0 at 3/6/10/15/25 m). The chart genuinely
discriminates: the classic linear-ramp formula misses those
centres by 27/12/8 codes. Prerequisite: the `solid` arm (§2.1).

### 2.3 Grade key decision — `sat` / `exposure` [loader]

The loader's grade parser reads only `sat` (`scenefile.rs:975`,
default 1.12) and has **no `exposure` field**. Census at writing:
**12 of 14** segment files write `saturation` (only s03, s06 use
`sat`); 7 files also carry `exposure` (s01, s02, s07, s08b, s10,
s11, s12). All sampled *values* are correct (audits) — under the
loader they are silently unread, so stills render at sat 1.12
instead of e.g. s07's 0.85. (The [film] lane is unaffected: the
driver's PostState carries exposure itself.) Two options, stated
without recommendation, per the wave's ruling that this is the
lead's call:

- **(a) Loader accepts both spellings** — `saturation` as an
  alias for `sat`, `exposure` parsed into the grade params.
  Data untouched; the format grows a synonym + a field.
- **(b) Data rewrite** — the 12 files' `saturation` → `sat`,
  `exposure` dropped or moved under a documentary key.
  Format stays single-spelling; the diff touches 12 files and
  the validator gains a key-spelling check to keep it true.

**Acceptance (either option):** `scenes/fixtures/fx-grade.scene.json`
(which deliberately uses the loader's keys today) plus one
rewritten-or-aliased segment still (s07) render at the authored
saturation, and FIXTURES.md's grade section is updated to match
the decision.

### 2.4 Calendar film-path wiring [film]

Today the film driver composites **pre-rendered plates**
(`film.rs:2033` loads `film/assets/calendar-{1945,1955,1965,1974}.png`
+ `calendar-dark.png` from `gen_assets.py`). The wave's native
module (`painters/calendar.rs`, 7/7 tests) rasterises the same
card from `scenes/film/s07-calendar.json` data with the source's
flip/alpha law, and `painters/INDEX.md` gives its wiring:
`calendar::composite_over(&mut px, width, height, t_film_seconds)`
at film.rs's `overlay_calendar` call point.

**Acceptance:** S07 frames around the flips (film t ≈ 31.2–33.5)
from the native module match the plate path's staging (year
sequence 1945→1955→1965→1974, alpha law, squash) — and the
known delta is recorded, not discovered: the module's glyphs are
hand-authored geometric approximations of Inter (year width
283.2 vs ≈284.2 local units, 0.4%), so plate-vs-module is an
A/B-with-named-delta, not an equality gate (§4.3).

### 2.5 film.rs divergence register — status at writing

WAVE-RECORD §5 lists D6–D17 as open at its HEAD (`311656d`).
Re-verified at `dbf90b9`: **D13 is FIXED** (see §2.6) and the
rest stand. Marking: **F** = fidelity-blocking (the render runs,
but those frames differ from the staged source in a visible way),
**m** = minor (single instant or subtle), **B** = benign by
construction.

| # | Divergence | Mark | Note |
|---|---|---|---|
| D6 | S02 march clamp `min(k, 0.97)` halts the march; source slides the final 3% (0.204 m) | **F** | motion, end of S02 |
| D7 | S03 rifle drawn; source hides it in S03 | **F** | wrong object on screen |
| D8 | Shore trees rotated `i·1.7` (the JG ring's value); SH sets none | **F** | s02/s03 set dressing |
| D9 | Shore tree kinds swapped (round/cypress parity inverted) | **F** | s02/s03 set dressing |
| D10 | Palm frond parity inverted; tips take frond colour; 10×6 vs source 8×5 | **F** | every JG palm |
| D11 | Shelf-unit construction differs on 8 points incl. **inverted gap predicate** → different books/gaps in S10 | **F** | deterministic content difference |
| D12 | S10 legs = Walk at wk = 0; source stands until wk > 0 | m | the cut-in instant only |
| D14 | S11 Onoda2 lean eased via poseTrack; source is a **step** at dropT + 0.55 | **F** | motion timing |
| D15 | S11 arm defaults ([8,6]/[8,6] etc.) vs source armR [16,−6], armL [10,5] / [8,4] | **F** | visible poses, S11 walk-in |
| D16 | S11 blink term omitted | B | its window can never render (W16) |
| D17 | Cloud tint `#EFF4EA` not applied by `add_clouds`; also a format gap (billboards carry no tint field) | m | subtle colour shift |

**Count: 8 fidelity-blocking (D6–D11, D14, D15), 2 minor
(D12, D17), 1 benign (D16).** D12/D15 were flagged "not
re-verified" in the register — they stand as reported until the
lead's pass confirms them in code. None of these block the
render *running*; they block calling its output faithful.
Checked clean and needing nothing: S08/S08b/S09 field-by-field;
the S11↔S12 rifle resting transform (independently derived
identically by two seats).

### 2.6 Register items already closed since WAVE-RECORD

- **D13 shade colour-space — FIXED by the lead, `c00897d`.**
  `shade_rgb` now does the THREE.Color linear-space multiply via
  the single helper; corpus pins reproduce exactly (#413427,
  #302222). Cost, declared in the commit: pre-fix references
  re-baseline (§3 step 5).
- **Black trapezoid — FIXED, `3e88c60`.** Composer TEX pipeline
  blended REPLACE into the HDR target; ALPHA_BLENDING now
  matches the source's transparent card plane.
- **D20 grade.time — CLOSED by W41** (§1.4). The record's
  "inert" framing was film-lane only; on the loader lane the
  values were live, in 7 files not 2.
- **D19** (gen_assets plate paints the calendar rule in ink
  `#1F1C33` vs drawCalendar's `#8B8378`) — asset-side, still
  open, minor: the native module and the data follow the
  source; the plate PNGs carry the ink colour until
  `gen_assets.py` is corrected and the plates regenerated.
- Set **merge-before-load** (sets README proposal) — not built;
  not on either lane's critical path (shot files inline their
  sets). Listed so it is not mistaken for done.

### 2.7 PERF §9 signals — optional, none blocking

`docs/runbook/PERF.md` §9 names four missing binary signals the
PC measurement protocol works around: elapsed seconds on the
25-frame progress lines (optionally per-frame ms behind an env
flag), an end-of-run paint-cache stats line, a peak-RSS
self-report, and real exit codes from `--film-telemetry` (it
currently exits 0 on pass, fail, and error alike — every gate
in §3 must read the verdict **line**, `FILM TELEMETRY: CHECK
OK`, never the exit code). Landing any of these makes the
user-PC run more measurable; landing none of them blocks
nothing.

---

## 3. Proof sequence — the ordered run

Each step's output is the next step's input; a step is done when
its verdict exists in the record, not when it was started.

1. **CI build + tests.** Push (close-out) → ci.yml green on the
   pushed HEAD, including the painter tests as in-tree cargo
   tests once §2.1 lands. *(Last known: run `43569768582`
   success at `e5860b4`.)*
2. **CI render dispatch (chunks).** render.yml, film mode,
   `--from/--to` chunks covering [0, 1811) — this VM cannot
   render the PBR path (llvmpipe JIT segfaults on shader_pbr),
   so CI lavapipe is the render environment of record here.
   Frames land as `frame_%05d.png`, resume-by-existence;
   **a PNG truncated by a hard kill is skipped by existence —
   delete the highest-numbered frame of any interrupted chunk
   before resuming it** (PERF §7).
3. **Film telemetry verdict.** `talerender --film-telemetry`
   over the whole film at the production paint-cache setting
   (and, per PERF §4, repeated at `TALERENDER_PAINT_CACHE=24` —
   both runs must print the same verdict line; a difference is
   a determinism defect). PASS = the printed line
   `FILM TELEMETRY: CHECK OK`. The exit code is always 0
   (§2.7) and proves nothing.
4. **Per-shot A/B table.** `tools/ab/measure_ab.py` in film
   mode against the project's `review/frames/` (Chrome
   SwiftShader reference): per-shot PSNR / mean |Δ| / %px >10
   for all 14 segments + the missing list empty. Read the
   numbers with the JPEG floor in hand: the references carry
   one q0.93 generation (42.31 dB from clean on a detailed
   frame), which biases every score *downward* — the table
   understates native accuracy, never overstates it
   (`tools/ab/README.md`). Calibration: headless-gl band
   15.8–19.3 dB; this pipeline's S01 trajectory
   20.152 → 20.218 → 23.850 dB.
5. **Golden staging + the human pin decision.**
   `tools/golden/run_golden.sh <frames-dir> --renderer
   "CI lavapipe"` stages the 14 segment mid-frames and verifies.
   Two pin decisions, both **human acts** (the runner refuses
   `--pin` by design):
   - the 14 film mids get their **first pins** from these frames;
   - `ab1-composer` (and on regeneration `s28-onoda-t150`,
     `stroke-toon`) are **expected STALE** against post-`c00897d`
     renders — the shade root-fix moved every shade()-derived
     colour in them (W45's analysis; `dof-chart` is the control:
     it contains no shade() output and must stay OK — if it goes
     STALE, the cause is something else). Re-pinning them is
     the declared re-baseline from `c00897d`, recorded with its
     cause, not a silent refresh.
6. **ffmpeg finish + MP4 evidence.** `film/finish.sh`: frames +
   the project's existing `data/mix.wav` muxed unchanged; x264
   slow CRF 18, BT.709, faststart, `.part` discipline. Evidence
   = ffprobe of the finished MP4 (1080×1920, 30 fps, 1,811
   frames, 60.357 s ± a frame) + loudness measured **on the
   mux** (≈ −14.0 LUFS expected because the mix is unchanged —
   measure, don't assume).
7. **Proof package index.** One page linking: CI run ids
   (build + each render dispatch), the telemetry verdict line,
   the A/B report (`ab-report.md` + side-by-sides/diffs for the
   three lowest-PSNR pairs), the golden verify output + pin
   record, the ffprobe + loudness lines, and §4's residuals
   verbatim. Also update the integration README's proof section
   (still `TBD-PROOF`) and correct its verdict-string quote —
   it prints `FILM TELEMETRY: CHECK OK`, not `CHECK OK`
   (REVIEW-W50). Until this index exists, no doc may claim the
   film is proven end-to-end (REVIEW-W50 finding C2 — the
   skill-wiring copy's NATIVE-BACKEND.md overclaims exactly
   this; its correction is seat W53, work-copy side).

**Renderer labelling law for every artefact above:** each frame
set is named by its renderer (`CI lavapipe` here; `PC RADV` for
the user's-PC run under `docs/runbook/PERF.md`). No performance
numbers come out of steps 1–7 — performance is the PC
protocol's job, on RADV, and is a separate deliverable.

---

## 4. Known residuals — these survive a successful render

The final report inherits these verbatim; a green §3 does not
retire any of them.

1. **S06 fog fidelity [film].** The foggiest shot is also the
   least-verifiable staging: S06's effective scene fog
   (#C9CFC2, near 4.5, far 20 — shots.ts:409 replaces the
   preset in the same draw call) coexists with the character
   tint's *different* fog colour (#E6E9EA, the preset's) — a
   genuine source mismatch, preserved deliberately (do not
   "fix"). The loader-lane fog chart (§2.2) verifies the fog
   *curve*; the in-shot composite of fog + fire + grade in S06
   has no equivalent analytic reference, only the A/B table.
2. **Caption width bound.** Caption strips are PIL-measured
   against the project's Inter-Black: max rawW Δ 0.15 px vs
   Skia on this film, fit Δ ≤ 0.0005, **no audit verdict
   flips** (W02). That is a measured bound for *this* film's 66
   chunks, not a typesetting engine — the native side still has
   no Inter measurer (contract §2.4, carried from M6), and a
   future film's text could land closer to a threshold.
3. **Calendar glyphs: approximation vs plates.** The native
   calendar module's glyphs are hand-authored vector
   approximations of Inter (year width 283.2 vs ≈284.2 local
   units, 0.4%; box-supersample AA, slightly softer than Skia).
   The plate path (current film driver) renders true Inter via
   PIL. §2.4's wiring therefore trades a small, named fidelity
   delta for data-driven rendering — the delta ships in the
   open, or the plates stay.
4. **RADV fp16 unverified.** M3's depth-packing default
   (Rgba8Unorm) exists because HalfFloat storage truncates
   ~1 ulp on llvmpipe; RADV's fp16 rounding behaviour has never
   been measured from this environment (no GPU here). The
   user's-PC run is the first RADV evidence; until then the
   conservative default stands and no RADV-specific claim is
   made.
5. **Painter fidelity classes (standing).** Skia analytic AA vs
   the house SDF/4×4-supersample coverage model — edge bytes
   only, M4 battery-verified for strokes; `rnd()` sin-hash can
   differ ≤1 ulp between V8 and Rust libm (measured ≤4 px on
   the felt pin statistic); three's truncated LinearToSRGB
   exponent 0.41666 reproduced deliberately. These bound every
   painter in §2.1, wired or not.
6. **Sign font size is PIL-proven only** (gap B9): fs = 40
   leaves 43.04 px of headroom under the shrink rule, so a
   browser-metric shrink is unlikely, not impossible. The
   native sign painter follows the PIL measurement.
7. **VM-environment caveat on all of §3.** CI lavapipe proves
   correctness, not the production renderer: SwiftShader/
   lavapipe/RADV frames of the same staging can differ in the
   last ulp (M3 finding). The golden pins struck in step 5 are
   lavapipe pins; the PC RADV run re-verifies against them and
   any renderer-attributable delta is reported as such, not
   absorbed silently.

---

*Sources: `docs/integration/WAVE-RECORD.md` (§5 register, §6
limits), `AUDIT-W40.md` + its W41 resolution addendum,
`AUDIT-W42.md`, `scenes/fixtures/FIXTURES.md`,
`docs/integration/REVIEW-W50.md`, `shaders/fog/FOG.md`,
`painters/INDEX.md` + `TEST-RUN.md`, `docs/runbook/PERF.md`,
`docs/golden/NATIVE-GOLDEN.md` + `RUNNER.md`,
`tools/film/VALIDATION.md`. Where this checklist and
WAVE-RECORD disagree (validator count, D13/D20 status, black
trapezoid), this checklist is right: WAVE-RECORD is a snapshot
at `311656d` and its dated refresh (seat W52) had not landed
when this was written.*
