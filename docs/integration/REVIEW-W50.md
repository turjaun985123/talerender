# REVIEW-W50 — mid-wave external review of the integration doc set

Seat W50, TaleRender 100-agent wave. Read-only review: every doc
below was read as a stranger would read it, then its claims were
cross-checked against each other and against the tree at HEAD
`c7db74d` (git log, the scene files at HEAD, `src/scenefile.rs` /
`src/scene.rs` / `src/film.rs`, both workflows, and a live run of
`tools/golden/make_golden.py`).

Docs reviewed: `docs/integration/README.md`,
`docs/integration/WAVE-RECORD.md`, `docs/integration/AUDIT-W40.md`,
`docs/runbook/USER-PC.md`, `docs/runbook/CI.md`,
`tools/film/VALIDATION.md`, `painters/INDEX.md`,
`shaders/fog/FOG.md`, `docs/golden/NATIVE-GOLDEN.md`, and
`~/workspace/talerender-skill-wiring/docs/NATIVE-BACKEND.md`
(plus its sibling `template/tools/CAPTURE-NATIVE.md` where the two
had to agree).

**Counts: 6 contradictions, 11 stale items, 1 broken reference,
2 missing.** What checked clean is listed at the end — most of
the set is accurate, and the failures cluster in one place:
`WAVE-RECORD.md` was verified against HEAD `311656d`, and the
lead's `3e88c60` / `c00897d` / `e5860b4` and Batch 4's `c7db74d`
all landed after that snapshot without the record being
re-stamped.

---

## CONTRADICTIONS

### C1. grade.time: "inert" vs live in the loader (the calibration case — confirmed, and wider)

- `docs/integration/WAVE-RECORD.md:76` (R3 row): s05b/s07's
  `grade.time` values are "inert (the renderer pins 0)".
  Repeated at `WAVE-RECORD.md:209` (D20): "Inert — film.rs pins
  grade time 0 (R3); data hygiene fix only".
- `docs/integration/AUDIT-W40.md:83`: "This is live, not inert:
  the scene-file loader reads the field".
- **AUDIT-W40 is right.** `src/scenefile.rs:976` reads `time`
  into `GradeCfg` and `src/scenefile.rs:2051` passes it into the
  composer's `GradeParams`. film.rs pins 0 only in its own
  hardcoded path (`src/film.rs:3237`). The scene-file path —
  the one these files exist for — consumes the value, so every
  loader render of an affected file seeds its GRADE grain with
  film t instead of 0.

### C2. NATIVE-BACKEND claims a proof package the repo says does not exist

- `~/workspace/talerender-skill-wiring/docs/NATIVE-BACKEND.md:34`:
  the native path is "proven end-to-end on one complete short
  (…, telemetry CHECK OK, per-shot A/B + MP4 evidence in the
  TaleRender repo's `docs/integration/`)".
- `docs/integration/README.md:74`: the proof section is still
  "TBD-PROOF: telemetry verdict; per-shot A/B PSNR table
  headline; MP4 ffprobe + loudness; determinism byte-compare;
  format v2 proof render".
- `docs/integration/WAVE-RECORD.md:299`: "there is **no
  completed full-film verdict in the record**" — the 1,811-frame
  render, the whole-film telemetry gate, the per-shot A/B table,
  the MP4 evidence and the 14 golden mid-frames are all listed
  as remaining proof items.
- **WAVE-RECORD and the README are right; NATIVE-BACKEND is
  wrong.** Tree check: `docs/integration/` contains only
  README / INVENTORY / WAVE-RECORD / AUDIT-W40 — no A/B table,
  no MP4 evidence, no telemetry verdict record. The one
  user-facing doc in the set asserts exactly the verdicts the
  repo's own record lists as outstanding. (W43's reconciliation
  fixed this doc's adapter claims; the status paragraph was not
  touched.)

### C3. Unknown texture generators: flat-shade fallback vs hard error

- `docs/integration/WAVE-RECORD.md:104`: until the painter arms
  land, "scene files referencing them fall back to averaged
  flat shades in the still loader". (The same claim is repeated
  in `painters/planks.md`, the file INDEX points to for the full
  delta list.)
- `painters/INDEX.md:399` speaks of "the `resolve_texture`
  unknown-generator **error**" as the guard.
- **INDEX and the source are right.** `src/scenefile.rs:398`
  (`resolve_texture`) ends in
  `other => return Err(err(ctx, "unknown texture generator '{other}'"))`
  — a hard load error; there is no averaging fallback anywhere
  in scenefile.rs. Anyone running a bookshop scene file through
  the still loader today gets an error, not a flat-shaded
  render, and WAVE-RECORD tells them the opposite.

### C4. Integration README asserts and disclaims the same proof

- `docs/integration/README.md:46`: resume purity is "proven by
  re-rendering a chunk boundary twice and byte-comparing".
- `docs/integration/README.md:74`: the same file's Proof
  section lists "determinism byte-compare" as TBD-PROOF.
- **The TBD is the honest side** — no byte-compare evidence
  exists anywhere in the tree. The CLI section should not claim
  as proven what the Proof section admits is outstanding.

### C5. Integration README: fog "validated by S06 A/B frames… not by a separate gate"

- `docs/integration/README.md:95`: "the fog path itself is
  validated by the film's S06 (fog set) A/B frames, not by a
  separate gate".
- `shaders/fog/FOG.md` §7 designs a dedicated fog-chart gate
  (analytic panel centres, ±2 sRGB codes, the ramp-vs-smoothstep
  discriminator), and the chart now exists as built files —
  `scenes/fog-chart.scene.json`, `scenes/fog-chart-nofog.scene.json`,
  `tools/fog-chart/` (committed in `c7db74d`).
- `docs/integration/WAVE-RECORD.md:299–306`: no film A/B table
  exists yet at all.
- **The README is wrong on both halves:** there are no S06 A/B
  frames to validate anything yet, and a separate gate does
  exist (built, not yet run). FOG.md + WAVE-RECORD describe the
  real state.

### C6. The saturation-key defect: right mechanism, wrong scope (×6), and the record omits it

- `docs/integration/AUDIT-W40.md:203–210` (§4) frames the
  `saturation`-vs-`sat` defect as an s07/s11 problem, with
  "s03 writes `sat` — consumable" as the conforming example.
- The HEAD file set disagrees on scope: only **s03** (`sat`
  0.84) and **s06** (`sat` 0.52) use the key the loader reads;
  the other **12 files** (s01, s02, s04, s05a, s05b, s07, s08,
  s08b, s09, s10, s11, s12) all write `saturation`, which
  `src/scenefile.rs:975` never reads — the loader's default
  1.12 applies to all twelve.
- **AUDIT-W40's mechanism is right; its scope understates the
  defect sixfold.** Companion gap: `WAVE-RECORD.md` §6 item 2
  (the limits list) names only the loader's missing `exposure`
  field for `post.grade` and never mentions the saturation
  key-name mismatch — the integration record's limits omit the
  larger half of a live consumption defect affecting 12 of the
  14 files it catalogues.

---

## STALE NUMBERS / STATUSES

All of S1–S7 and S10 are `WAVE-RECORD.md` present-tense claims
from its `311656d` snapshot, overtaken by later commits. The
record's own header discloses the snapshot ("re-verified
against the tree at HEAD `311656d`"), but the body never
re-stamps, so a stranger reads superseded states as current.

### S1. Validator: 5/7 vs 7/7

- `WAVE-RECORD.md:42`: "**5/7 checks PASS**", with the 69
  rounded computed values and caption chunks 46/56 double-listed
  as residual failures.
- `tools/film/VALIDATION.md:36`: "**7/7 checks PASS**… 883
  exact + 506 source literals, 0 computed failures"; chunks 46
  and 56 removed (W31).
- **VALIDATION.md is right** (its run is the current validator
  state, committed in `c7db74d`).

### S2. The three "scoped, not yet landed" follow-ups all landed

- `WAVE-RECORD.md:153` (§4): the 69 f32 rewrites, caption
  chunks 46/56, and the SH C6/C7 completion in s02/s03 are
  listed as follow-ups "not yet landed at this writing".
- All three landed in `c7db74d` (VALIDATION.md 7/7; s02/s03
  regrouped). **Stale.**

### S3. D13 shade colour-space: "held integrator decision" vs fixed at the root

- `WAVE-RECORD.md:197` (D13): the shade divergence is an
  "**Integrator decision**… deliberately not done by the
  seats"; repeated at `WAVE-RECORD.md:225` ("D13 is a held
  integrator decision").
- Commit `c00897d`: "shade_rgb: THREE.Color linear-space
  multiply (was sRGB byte multiply, 25–40 LSB dark on derived
  shades). Root fix at the single helper shade_lin and every
  texture painter route through; corpus pins reproduce
  exactly… RE-BASELINE: ab1/hills/grass/felt shades change".
  `src/scene.rs:61–78` now implements `shade_rgb` itself as the
  linear-space arithmetic.
- **The commit is right; the register is stale.** The decision
  was taken, the fix landed, and the register still lists the
  item as open-and-held.

### S4. Black trapezoid: "unresolved" vs root-caused and fixed

- `WAVE-RECORD.md:233–236`: the lead "is bisecting a
  black-trapezoid artefact… unresolved at this writing".
- Commit `3e88c60`: root cause found (the composer path's scene
  TEX pipeline blended REPLACE into the HDR target, writing
  card transparency as opaque black) and fixed
  (ALPHA_BLENDING); the diag variants were removed.
- **Stale.**

### S5. Painter tests: "not run" vs 52/52 run

- `
...[truncated 8536 chars]