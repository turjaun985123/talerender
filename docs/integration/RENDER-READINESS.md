# Render-readiness checklist — production integration, final phase

---

**REFRESH — seat W76, 2026-11-14.** This checklist was re-walked
item by item against the tree at HEAD `3781eb2` (Wave batch 7
close). Every status was verified first-hand at the refresh —
git log/show, source grep, validator + golden re-runs, CI run
records — nothing carried on trust. Where a status updates
W58's text, the note is marked **W76:** and W58's wording is
left standing as the record of its moment.

- Audited HEAD `3781eb2`; **pushed** HEAD `d8fbe83` (only
  batch 7 is unpushed). CI on the pushed HEAD: run
  `43574473202` **success**, the in-tree cargo tests included
  (§2.1). The loader-lane commit `c74cf0c`'s own run
  (`43574018568`) failed its Test step on exactly the felt
  shade test W70 refuted; `d8fbe83` fixed that test and
  turned CI green.
- Validator re-run at the refresh: **7/7 PASS**, exit 0.
  Golden verify re-run: **4 OK / 0 STALE / 14 MISSING**.
- Working-tree caveat: at audit time the tree also carried
  **uncommitted** Batch-8 sibling work (the §1.8 sea-metalness
  fix in s02/s03 + ASSUMPTIONS, FOG-STATUS-W74, the
  FINAL-REPORT update, GATE-W80, an untracked
  `src/shader_fog.wgsl`). All statuses below are stated
  against HEAD `3781eb2`; in-flight working-tree changes are
  flagged where they touch an item.
- Marks: **CLOSED** (done, evidence cited) · **OPEN** (stands
  as W58 wrote it) · **CHANGED** (the state moved; the note
  says how) · **UNVERIFIED** (this audit could not determine
  the state; the reason is given, nothing guessed).
- §5 at the end is the live remainder: only what still
  stands between here and the proof.

---

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
| 1.2 | Independent audits: **87/90** (W40: s03/s07/s11) + **162/165** (W42: the other eleven) = 249/255 sampled values recomputed from source | `docs/integration/AUDIT-W40.md`, `AUDIT-W42.md`. Of the 6 mismatches: grade.time ×2 → fixed by W41 (§1.4); s07 cap_039 → **reclassified** by W41 (not a defect: the wave convention is bleed-note-not-listing; s07 already carries the note; the manifest assigns chunk 39 to s06 and the validator forbids a second listing); s06 ×3 → §1.7 (**CLOSED at refresh**, see there). **W76: CHANGED** — a third audit (W61, `docs/integration/AUDIT-W61.md`, batch 7) sampled unsampled categories across all 14 segments: **138/140**; cumulative sampled **387/395**. Its 2 mismatches share one root → the new §1.8 |
| 1.3 | Captions single-owned + notes | Validator check 5 (66/66/0/0); chunks 31/39 (W27), 46/56 (W31) single-owned by the start-time rule; bleed chunks recorded in `captionsNote`. Strips + manifest committed: `assets/captions/manifest.json` = 66 chunks; `film/assets/` carries the strips + plates (assets-ignore accident fixed in `b3f3c0a`) |
| 1.4 | `grade.time` = 0 in **all 14 files** | Census at writing: every `scenes/film/*.scene.json` reads `post.grade.time` 0/0.0. W41 zeroed the 7 carriers (s02, s03, s04, s05a, s05b, s06, s07 — each had carried its own film-t). This mattered: the loader reads the field (`scenefile.rs:976` → `:2051`), so pre-fix stills of 7 segments would have grained differently from Chrome on every frame |
| 1.5 | Manifest state | `scenes/film/film.json`: frame ranges tile [0, 1811) exactly (validator check 6); caption ranges tile chunks 0–65; per-segment start/duration are the recomputed cuts (e.g. s06 startS 26.129 — the manifest was always right where the scene file was not, §1.7) |
| 1.6 | Sets + atmospheres | 3 set fragments committed (`scenes/sets/`: SH 229 meshes, BK 369 incl. 310 books, JG 325 + 900 tufts) with C1–C9 corrections applied to the shot files (W26) and SH C6/C7 completed in s02/s03 (W32); `scenes/atmospheres.json` — 7 presets + 6 merged filmLooks, validator check 7 resolves 14/14. Set *merge-before-load* is a loader-lane convenience (§2.6), not a data gap: shot files inline their sets |
| 1.7 | **CLOSED (W76) — s06 cut-rounding (seat W51, landed in `ce06011`)** | `scenes/film/s06.scene.json` still carries the INVENTORY 2-dp values: `shot.at` **26.13** (correct **26.129** = cut(5)), `shot.duration` **4.260000228881836** (correct **4.261**), fire atTime value **0.6314269304275513** (correct **0.6333623112323646**). One root cause (W42 M1–M3). The validator does **not** catch this class — its tiling check reads the manifest, which is correct; only the audits sample scene-internal `shot.at`. Fix = the three values + ASSUMPTIONS correction note + validator re-run (must stay 7/7). **W76 verified in the file at HEAD:** `shot.at` **26.129**, `shot.duration` **4.261000156402588** (the f32 of 4.261), fire **0.6333622932434082** — the old 26.13 / 4.260000228881836 / 0.6314269304275513 are gone, and the validator re-run at the refresh still passes 7/7 |
| 1.8 | **OPEN at HEAD (W76) — sea `metalness` dropped in s02/s03** | W61 finding M1 (`AUDIT-W61.md`): the sea node in s02 (`sea`) and s03 (`sea (plane stand-in)`) carries `{ color #4E86AC, roughness 0.35 }` with **no `metalness`**; the SH builder builds the sea with metalness **0.08**. A declared omission (ASSUMPTIONS' SH notes call it "not a format material field") that the corpus itself disproves — barrel bands 0.4, the s10 brass rail 0.6 and the s05a plane 0.25 all carry the field in data. **W76 verified at HEAD** by parsing `git show HEAD:` of both files (absent at dbf90b9, ce06011, 381ff47 and HEAD alike); the set fragment `scenes/sets/shore.scene.json` does carry it (0.07999999821186066, the f32), so the value was dropped when the set was inlined into the shot files. Impact is **latent, not live**: the loader consumes metalness nowhere today (W47 census; re-verified by grep at this refresh — `metalness` appears nowhere in `src/scenefile.rs` or `src/scene.rs`), so nothing on either lane renders differently — the value is simply unrecoverable from the shot files if a future loader starts consuming it. Fix = one field in two files, or a lead material-field ruling. **In flight at audit time:** an uncommitted working-tree fix (Batch-8 seat W73) adding `metalness: 0.08` to s02/s03 + the ASSUMPTIONS correction; this item closes when batch 8 commits |

**Data-side verdict: GREEN except §1.7** — a three-value fix,
already briefed, blocking nothing else.

**W76: CHANGED — §1.7 is CLOSED (`ce06011`), so the data
side is GREEN in full at HEAD, with one new latent item
(§1.8) that blocks nothing.**

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

**W76: CHANGED — the wiring LANDED in `c74cf0c`.**
`resolve_texture` (`src/scenefile.rs:414–435`) now carries
arms for `planks`, `wallpaper`, `felt`, `parcel`, `picture`,
`sign` and `solid` (64×64 flat, the fog-chart spec), with the
six painters pasted into `src/scene.rs` per
`painters/INDEX.md` — all verified by source grep at HEAD.
The paragraphs above are the pre-wiring record. The test half
of the acceptance below is now evidenced in-tree: `c74cf0c`'s
CI run `43574018568` failed its Test step on exactly one
test — `scene::felt_tests::shade_is_the_three_linear_multiply`,
the assertion W70's red team refuted (it pinned the
pre-`c00897d` divergence) — and the lead's `d8fbe83`
re-pinned it to the post-fix agreement; CI run `43574473202`
at `d8fbe83` is **green, Test step included**. The fixture
half of the acceptance is **UNVERIFIED**: no record in the
tree or CI available to this audit shows
`fx-textures.scene.json` loaded by a post-wiring binary
(FINAL-REPORT.md still records the pre-wiring failure). It
stays on the remainder (§5).

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

**W76: CHANGED — the prerequisites advanced; the wiring
itself is still OPEN.** The `solid` arm (§2.1) landed in
`c74cf0c`, removing this section's stated prerequisite. The
Chrome half of the acceptance now exists: seat W66 assembled
and **ran** the harness (`tools/fog-chart/chrome/`, batch 7)
— static verification 144/144, the captured frame committed,
the basic panels within **1 code** of the analytic smoothstep
centres at all five depths (vs the linear ramp's 26/12/8
misses), and the standard panels' offset fully explained as
the house light rig's ×1/π convention (VERIFY.md). The
loader wiring itself has **not** landed: seat W74's status
check (FOG-STATUS-W74.md — verdict: the `fog` block is
ignored entirely, not parsed-and-unused) and this refresh's
grep agree — at HEAD the loader never reads the `fog` key
(the only `fog` hits in `src/scenefile.rs` are the `solid`
arm's comment; none in `src/scene.rs`). FOG.md §8 remains
the spec. (An untracked `src/shader_fog.wgsl` sat in the
working tree at audit time — in-flight work, not HEAD
state.)

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

**W76: CHANGED — the lead decided in `c74cf0c`, in a third
shape: canonical keys, loader-side.** The commit fixes the
canonical grade keys as `sat`/`warm`/`exposure`/`time`;
`exposure` is now parsed (`src/scenefile.rs:1011`, default
1.0) and wired to the composer OutputPass uniform
(`shader_output_exp`; absent grade = 1.0, the old hardcoded
value bit-for-bit), and `sat` is read at `:1010`. **What the
decision did not do: there is no `saturation` alias anywhere
in `src/` (grep-verified), and the data was not renamed** —
this refresh's census finds `saturation` still written in
**12 of 14** files (only s03/s06 spell `sat`) and `exposure`
in the same 7 files W58 listed. So option (a)'s exposure
half landed and its alias half did not, and option (b)'s
rename is **still owed**: until the 12 files are renamed
(or an alias is ruled in), their authored saturation
remains silently unread on the loader lane, exactly as
described above. The doc trail lags the decision too:
FIXTURES.md's grade section still describes the
pre-decision state, and `docs/scene-format.md`'s example
still spells `saturation`. The acceptance below stands,
unmet on the saturation half.

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

**W76: CHANGED — the plates were corrected; the module was
not wired.** `src/film.rs`'s `overlay_calendar` (`:3331`,
called at `:3441`) still composites the pre-rendered plates
from `film/assets/`; no `calendar::` call exists in the
driver. What landed instead is the asset-side fix for D19:
`381ff47` regenerated the four calendar plates with the
source's rule colour `#8B8378` (only those 4 PNGs changed
on the asset side), so the shipping plate path no longer
carries the ink-colour delta (§2.6). The native-module
wiring below therefore stands as an **option with a named
price** (§4.3), not a pending correction: wiring it trades
true-Inter plates for data-driven rendering at a 0.4%
glyph delta.

**Acceptance:** S07 frames around the flips (film t ≈ 31.2–33.5)
from the native module match the plate path's staging (year
sequence 1945→1955→1965→1974, alpha law, squash) — and the
known delta is recorded, not discovered: the module's glyphs are
hand-authored geometric approximations of Inter (year width
283.2 vs ≈284.2 local units, 0.4%), so plate-vs-module is an
A/B-with-named-delta, not an equality gate (§4.3).

### 2.5 film.rs divergence register — status at writing

**W76: CLOSED in full — the lead's `381ff47`** ("WAVE-RECORD
S5 register closed in film.rs (R4: source wins)") enumerates
the fixes for D6, D7, D8/D9, D10, D11, D12, D14, D15, D17
and D19 one by one; D13 was the `c00897d` root fix (§2.6),
D18 was already equivalent, and D16 is benign by
construction — its blink window can never render, so there
is nothing to fix. This refresh spot-verified two rows in
code: D11's corrected gap predicate (`rnd(...) > 0.14`,
`src/film.rs:1011`) and D14's step at dropT + 0.55
(`src/film.rs:1898–1903`, with the source comment naming the
step). The table below is the pre-fix record: its marks
describe the state at `dbf90b9`, not at HEAD. Fidelity now
rests on the re-render (§3 step 2). W58's intro, for the
record: WAVE-RECORD §5 lists D6–D17 as open at its HEAD
(`311656d`); re-verified at `dbf90b9`, D13 was FIXED and the
rest stood. Marking: **F** = fidelity-blocking (the render runs,
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

**Count at refresh (W76): 11 of 11 rows CLOSED** — the 8
fidelity-blocking (D6–D11, D14, D15), 2 minor (D12, D17) and
1 benign (D16), all by `381ff47`. W58's count for its moment:
8 fidelity-blocking, 2 minor, 1 benign, all open. D12/D15
were flagged "not re-verified" in the register and stood as
reported until the lead's pass — `381ff47` is that pass.
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
  **W76: CLOSED by `381ff47`** — the four plates were
  regenerated with the source rule `#8B8378`; the driver's
  plate path (§2.4) now carries the source colour.
- Set **merge-before-load** (sets README proposal) — not built;
  not on either lane's critical path (shot files inline their
  sets). Listed so it is not mistaken for done.
  **W76: still OPEN, unchanged** — grep at HEAD finds no
  set-fragment merge in the loader (the `merged` hits in
  `scenefile.rs` are geometry scatter, unrelated).

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

**W76: OPEN, unchanged.** None of the four signals landed
in the audited range. The exit-code point was re-verified
in source: `src/main.rs:131–140` — the `--film-telemetry`
branch returns (exit 0) on Ok and on Err alike, so every
gate still reads the verdict **line**. The stderr progress
lines (`e5860b4`) pre-date W58 and carry counts, not
elapsed seconds.

---

## 3. Proof sequence — the ordered run

Each step's output is the next step's input; a step is done when
its verdict exists in the record, not when it was started.

1. **CI build + tests.** Push (close-out) → ci.yml green on the
   pushed HEAD, including the painter tests as in-tree cargo
   tests once §2.1 lands. *(Last known: run `43569768582`
   success at `e5860b4`.)* **W76: CHANGED** — the pushed HEAD
   is now `d8fbe83`, CI run `43574473202` **success**,
   painter tests included (§2.1); only batch 7 (`3781eb2`,
   docs/tools) is unpushed. The close-out push still owes
   this step's verdict on the final HEAD (§5).
2. **CI render dispatch (chunks).** render.yml, film mode,
   `--from/--to` chunks covering [0, 1811) — this VM cannot
   render the PBR path (llvmpipe JIT segfaults on shader_pbr),
   so CI lavapipe is the render environment of record here.
   Frames land as `frame_%05d.png`, resume-by-existence;
   **a PNG truncated by a hard kill is skipped by existence —
   delete the highest-numbered frame of any interrupted chunk
   before resuming it** (PERF §7).
   **W76: CHANGED — the render has been dispatched and
   completed in CI.** Ten render runs at 2026-11-14
   08:00–08:01 UTC on HEAD `381ff47` (the register commit,
   whose message calls for exactly this re-render), all
   **success**; their artifacts tile **[0, 1811)
   completely** — film chunks 0–200, 200–400, 400–600,
   600–800, 800–1000, 1000–1200, 1200–1400, 1400–1600,
   1600–1811, plus a 1195–1205 spot chunk — verified run by
   run from the artifact names. `src/film.rs` is untouched
   since `381ff47`, so the frames carry current film-lane
   provenance. The frames exist as per-chunk CI artifacts;
   assembling them into one frames dir is the first act of
   the remaining chain (§5).
3. **Film telemetry verdict.** `talerender --film-telemetry`
   over the whole film at the production paint-cache setting
   (and, per PERF §4, repeated at `TALERENDER_PAINT_CACHE=24` —
   both runs must print the same verdict line; a difference is
   a determinism defect). PASS = the printed line
   `FILM TELEMETRY: CHECK OK`. The exit code is always 0
   (§2.7) and proves nothing. **W76: UNVERIFIED** — no
   `FILM TELEMETRY: CHECK OK` line exists in any record
   this audit can read; publishing the verdict is the
   lead's, with the proof package. This is the head of the
   remainder (§5).
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
     cause, not a silent refresh. **W76: CHANGED** — the
     re-pin evidence now exists as packets
     (`docs/golden/repin-packets/`, batch 7): `ab1-composer`
     has its post-fix candidate in hand (23.96 dB vs the
     Chrome reference); `s28-onoda-t150`'s candidate is
     measured and the packet recommends **against**
     re-pinning it under the shade debt (it differs
     structurally, 21.711 dB, max Δ 206 — the §28 pin is a
     prototype artefact, not a regression target);
     `stroke-toon` is pending a post-fix candidate;
     `dof-chart` is pending-not-required. The 14 film mids
     still await the assembled frames (step 2 note) and the
     human pin.
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
   **W76: CHANGED in part** — W53's correction landed
   (`ce06011`): the overclaim is replaced by the true state,
   and the old text survives only as a quoted "before" in
   the doc's correction record. The proof skeleton
   (`docs/ab/film/`: README + EXPECTED + MP4-EVIDENCE
   template) and the FINAL-REPORT draft (`3781eb2`) also
   exist. The index itself, and the integration README's
   proof-section update (its `TBD-PROOF` line still stands
   at `docs/integration/README.md:74` at HEAD), remain
   open — they are proof-completion acts (§5).

**Renderer labelling law for every artefact above:** each frame
set is named by its renderer (`CI lavapipe` here; `PC RADV` for
the user's-PC run under `docs/runbook/PERF.md`). No performance
numbers come out of steps 1–7 — performance is the PC
protocol's job, on RADV, and is a separate deliverable.

---

## 4. Known residuals — these survive a successful render

The final report inherits these verbatim; a green §3 does not
retire any of them.

**W76:** residuals 1, 2, 5, 6 and 7 stand unchanged.
Residual 3 is CHANGED (note inline). Residual 4 stands as
an open unverified — no RADV evidence has appeared since
W58 wrote; the user's-PC run remains its first test.

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
   open, or the plates stay. **W76: CHANGED** — the plates
   stayed, and were corrected: `381ff47` regenerated them
   with the source rule colour `#8B8378`, so the plate path
   currently ships **no** named delta; the glyph
   approximation materialises only if the native module is
   wired later (§2.4).
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

---

## 5. Live remainder — after the W76 refresh (2026-11-14)

Only what still stands between here and the proof, by lane.
Everything not listed here is CLOSED above.

**[film] lane — the proof chain itself (§3 steps 3–7):**

1. Assemble the `381ff47`-provenance frames (CI artifacts,
   coverage of [0, 1811) verified at the refresh) into one
   frames dir and run the **film telemetry gate** at the
   production cache setting and at
   `TALERENDER_PAINT_CACHE=24` — the verdict **line**
   `FILM TELEMETRY: CHECK OK` must exist in the record
   (§3.3; currently UNVERIFIED — no line exists in any
   record this audit can read).
2. The **per-shot A/B table** over all 14 segments (§3.4),
   read against the JPEG floor as §3 prescribes.
3. **Golden staging + the human pin decisions** (§3.5):
   the 14 film mids' first pins, and the `ab1-composer`
   re-pin from its in-hand candidate. (The packets
   recommend *against* re-pinning `s28-onoda-t150`.)
4. **ffmpeg finish + MP4 evidence** (§3.6): ffprobe fields
   (1080×1920, 30 fps, 1,811 frames) + loudness measured
   on the mux.
5. **Proof package index** + the integration README's
   `TBD-PROOF` update and verdict-string correction
   (§3.7).

**[loader] lane:**

6. The **`saturation` → `sat` data rename in 12 files**
   (or a lead alias ruling) so the authored saturation
   actually takes effect, plus the validator key-spelling
   check and the FIXTURES.md / `scene-format.md` alignment
   (§2.3).
7. **Fog loader wiring** per FOG.md §8, then the fog-chart
   acceptance at ±2 codes — the Chrome reference frame,
   expected.json and the `solid` arm are all in place
   (§2.2).
8. **`fx-textures` load evidence** — the fixture loaded by
   a post-wiring binary, recorded (§2.1).

**Close-out:**

9. Push `3781eb2` + this refresh; read the CI verdict on
   the pushed HEAD (ci.yml + tools-ci.yml) the moment it
   lands.

*Also open, deliberately NOT on the proof path: §1.8 sea
metalness (latent; an uncommitted fix was in the working
tree at audit time and closes it when batch 8 commits),
the §2.4 calendar-module wiring (an option — the plates
are now correct), set merge-before-load (§2.6, a loader
convenience), and the PERF §9 signals (§2.7, optional).*
