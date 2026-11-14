# GATE-W80 — Batch-8 Consolidated Gate + FINAL-REPORT Numbers Audit (seat W80)

Two jobs: (1) re-run the wave's local verification suite first-hand;
(2) audit the quantitative claims in `docs/integration/FINAL-REPORT.md`
against their first-hand sources. Nothing taken on trust.

- Repo HEAD at gate start: `3781eb2` (Batch-7 central commit).
- HEAD moved during the gate: at audit time `main` = `origin/main` =
  `69952ab` (Batch-8 work landed and was pushed concurrently). Gate
  runs 1–5 executed against the tree as found at `3781eb2`; the audit
  notes where later commits change a claim's present-tense truth.
- All times UTC, 2026-11-14.

## JOB 1 — Consolidated gate: **6/6 PASS**

| # | Run | Result |
|---|---|---|
| 1 | Film validator (`tools/film/validate_film.py`) | **PASS** |
| 2 | Caption determinism (regen + `diff -r`) | **PASS** |
| 3 | Golden pack verify (`tools/golden/make_golden.py`) | **PASS** |
| 4 | Bridge packager (`tools/bridge/package_bridge.py`) | **PASS** |
| 5 | Scene JSON parse sweep (`scenes/**/*.json`) | **PASS** |
| 6 | Painter test harness (`painters/TEST-RUN.md`) | **PASS** (production form 45/45; staging form 51/52 — the one failure is the residue FINAL-REPORT §2.7 itself names, see Run 6) |

### Run 1 — Film validator (08:12:11)

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

### Run 2 — Caption determinism (08:12:11)

- Command: `python3 tools/captions/gen_caption_strips.py --project ~/faceless-shorts/the-soldier-who-didnt-know --out /tmp/gate80-captions`, then `diff -r /tmp/gate80-captions assets/captions` (assets live at repo-root `assets/captions`, per GATE-W62).
- Expected: byte-identical (66 strips + manifest), diff silent, exit 0.
- Observed (verbatim):

```
CAPTION STRIPS: 66 chunks, min fit 0.92, max rendered width 960 px, 7 over the 850 px floor, 0 under the 0.68 fit floor
wrote /tmp/gate80-captions/manifest.json + 66 strips to /tmp/gate80-captions
GEN_EXIT=0
DIFF: byte-identical
DIFF_EXIT=0
```

- 67 files compared, no differences. **PASS**

### Run 3 — Golden pack verify (08:12:33)

- Command: `python3 tools/golden/make_golden.py`
- Gloss (the correct one): **OK = the pin matches its own committed
  bytes**; STALE is a property of *re-renders* against post-fix
  binaries, not of this verify; the 14 film segment mid-frames stay
  MISSING until the lead's film render lands.
- Observed (verbatim tail): `4 OK / 0 STALE / 14 MISSING of 18 golden items`, exit 0. Pins: s28-onoda-t150 `ff68fad3…`, dof-chart `957d5573…`, stroke-toon `9ba9ff89…`, ab1-composer `d7bd71a0…` — each pinned = current. **PASS**

### Run 4 — Bridge packager (08:12:33)

- Command: `python3 tools/bridge/package_bridge.py --repo . --project /tmp/gate80-bridge` (fresh target).
- Observed (verbatim tail):

```
  [PASS] MANIFEST-SHA covers every packaged file, hashes match — 94 files verified
package-bridge: VERIFICATION PASSED — 21/21 checks
```

- **PASS** — 21/21, 94 files hash-verified.

### Run 5 — Scene JSON parse sweep (08:12:33)

- Command: own one-liner over `scenes/**/*.json` (`json.load` each).
- Observed (verbatim): `JSON sweep: 32 files parsed, 0 invalid`. **PASS**

### Run 6 — Painter test harness (setup 08:13:51; runs to ~08:20)

TEST-RUN.md's harness predates the lead's paste-in (`c74cf0c`), so it
was run in both forms the tree now supports. Same method both times:
standalone rustc 1.99.1 (rustup, `~/.cargo/bin`), scratch crate in
/tmp, the documented wgpu stub rlib (only
`COPY_BYTES_PER_ROW_ALIGNMENT = 256`, for `math.rs`'s unrelated
dependency — no painter module shimmed), no repo files modified.

**(a) Integrated form — the production code.** lib.rs = HEAD
`src/math.rs` + HEAD `src/scene.rs` verbatim. HEAD scene.rs now
contains the six pasted painters with their tests renamed per painter
(`planks_tests` 7, `wallpaper_tests` 6, `felt_tests` 5, `parcel_tests`
5, `picture_tests` 4, `sign_tests` 7 = 34) + `json_tests` 2, plus
math's 9 → 45 tests expected.

```
test result: ok. 45 passed; 0 failed; 0 ignored; finished in 138.89s
```

**45/45 PASS** — including `scene::felt_tests::
shade_is_the_three_linear_multiply`, the test `d8fbe83` re-pinned to
assert `shade_three == shade_rgb` (the post-root-fix agreement).

**(b) Staging form — TEST-RUN.md verbatim.** Staging `painters/*.rs`
modules + `painters/planks.rs` appended to scene.rs taken from
`c74cf0c^` (the last pre-paste tree) + HEAD `src/math.rs`.

```
test felt::tests::shade_is_the_three_linear_multiply ... FAILED
thread 'felt::tests::shade_is_the_three_linear_multiply' panicked at src/felt.rs:136:9:
assertion `left != right` failed
  left: (28, 83, 64)
 right: (28, 83, 64)
test result: FAILED. 51 passed; 1 failed; finished in 112.79s
```

**51/52, exactly the red team's refutation reproduced** — and the
reason is now precise: `d8fbe83` re-pinned the test **only in
`src/scene.rs`** (its diff touches that one file). The staging copy
`painters/felt.rs` still carries the superseded `assert_ne!` pinning
the divergence `c00897d` abolished on purpose; both sides compute the
source's value (28, 83, 64). This is not a painter defect and not a
production-test defect — it is a stale assertion in a superseded
staging copy, and FINAL-REPORT §2.7 already names this residue
verbatim. Run verdict: **PASS** for the production form (45/45);
the staging form's 51/52 is recorded here as the expected,
already-documented residue, not a new failure.

## JOB 2 — FINAL-REPORT numbers audit

**Version audited:** the on-disk `docs/integration/FINAL-REPORT.md`
is the **W75-refreshed** version, not the W69 draft — its own header
declares "completion pass at Batch 8 (seat W75)… at HEAD `3781eb2`
(70 of 100 seats complete)", and its content carries the Batch-8
assignments (W72/W73) and the `c74cf0c`/`d8fbe83` landings. Every
quantitative claim was checked against the owning doc or a re-run.

| # | Claim (FINAL-REPORT) | First-hand source | Verdict |
|---|---|---|---|
| 1 | 14 segments, 1,811 frames, narration 60.357 s (§1, §2.1) | Validator re-run, Run 1: 14 segments, 1811 frames; sum(durations) = 60.357 s | **MATCH** |
| 2 | mix.wav 60.46 s, −14.0 LUFS (§1) | ffprobe duration 60.457 s; ffmpeg loudnorm input_i = −13.99 | **MATCH** |
| 3 | Shore set: 229 leaf meshes (§2.1) | `scenes/sets/README.md`: "229 leaf meshes, 5 textures" | **MATCH** |
| 4 | Bookshop: 310 books across 360 slots (§2.1) | `"book"` count in `scenes/sets/bookshop.scene.json` = 310 (recounted); sets README 360-slot table | **MATCH** |
| 5 | Atmospheres: 7 presets, six filmLooks (§2.1) | `scenes/atmospheres.json` census: presets 7 (+`_cite`), filmLooks 6 (+`_note`) | **MATCH** |
| 6 | S06 fog #C9CFC2, near 4.5, far 20 (§2.1) | `scenes/film/s06.scene.json` fog block read this audit: #C9CFC2, 4.5, 20.0 | **MATCH** |
| 7 | Captions: 66 strips, min fit 0.9227, max width 960 px, 7 over the 850 floor, 0 under 0.68 (§2.1) | `assets/captions/manifest.json`: 66 chunks, min fit 0.9227 (recounted); generator re-run, Run 2 headline (960 / 7 / 0) | **MATCH** |
| 8 | PIL-vs-Skia raw width: max Δ 0.15 px, mean 0.054 (§2.1) | WAVE-RECORD.md:123: "PIL-vs-Skia measured: max Δ 0.15 px, no audit verdict flips" | **MATCH** for max Δ; the **mean 0.054** sub-figure's first-hand source was not located in the repo docs checked (no contradicting value found) |
| 9 | Validator: 1,389 checked, 883 exact f32 + 506 literals + 0 failures (§2.1, §5.2) | Re-run, Run 1, verbatim | **MATCH** |
| 10 | W31 recomputed 69 values; caption chunks 46/56 single-owned (§2.1) | `tools/film/VALIDATION.md` ("69 computed values"); WAVE-RECORD.md:40 ("chunks 46/56 single-owned (W31)") | **MATCH** |
| 11 | Painter tests 41/41, 52/52 with baseline "as measured on the Batch-4 tree" (§2.2) | `painters/TEST-RUN.md` (52 passed, 0 failed; per-module table sums 41 + 11) | **MATCH** (historical claim, correctly scoped in the report) |
| 12 | Red team read 51/52 at `ce06011`; staging copy still reads 51/52 (§2.2, §2.7, §5.2) | Re-run (b), Run 6: 51 passed / 1 failed, same test, same panic values (28, 83, 64) | **MATCH** |
| 13 | Production painter tests correct after the lead's re-pin (§2.2, §3) | Re-run (a), Run 6: 45/45 incl. the re-pinned felt agreement test | **MATCH** |
| 14 | Fog-chart factors 0 / 0.0263 / 0.2884 / 0.7550 / 1.0 (§2.3) | `tools/fog-chart/expected.json`: smoothstepFactor 0.026283…, 0.288375…, 0.754959… at 6/10/15 m | **MATCH** |
| 15 | Classic ramp misses 27/12/8 codes (§2.3, chart design) and 26/12/8 (§2.3, Chrome run) | expected.json discriminators: standard-material maxChannelDeltaCodes 27/12/8 at 6/10/15 m; basic-material 26 at 6 m; VERIFY.md: "misses by 26/12/8 codes" | **MATCH** — both numbers correct, each in its own material column |
| 16 | Fog-chart Chrome harness static verify 144/144; house factor 0.6988266044899671 (§2.3, §5.2) | `tools/fog-chart/chrome/VERIFY.md` (144/144; the factor verbatim) | **MATCH** |
| 17 | A/B tool reproduces the milestone packages digit-for-digit: 23.850 / 21.820 / 84.486 dB (§2.4, §5.2) | `docs/ab/composer/metrics.json` (23.850); `docs/ab/scene-format/ab1/metrics.json` psnr_db 21.82; `docs/ab/scene-format/dof-chart/metrics.json` (84.486) | **MATCH** |
| 18 | Shade fix: 23.96 dB vs pre-fix 23.850; mean abs Δ 5.20056 → 4.82505; %px >10 13.533 → 11.445 (§3, §5.2) | `docs/ab/composer-v2/metrics.json`: psnr_db 23.96, mean_abs_delta 4.82505, pct>10 11.44483 | **MATCH** |
| 19 | Frame-vs-frame 41.35 dB, max Δ 30, ~37% px touched (§3); packet figure 41.351 (§4.3) | `docs/ab/composer-v2/README.md`: 41.351, max abs Δ 30, 37% | **MATCH** |
| 20 | Golden pack 4 OK / 0 STALE / 14 MISSING of 18 (§2.4, §4.3, §5.2) | Re-run, Run 3, verbatim | **MATCH** |
| 21 | Bridge packager 21/21, 94 files hash-verified (§2.4, §5.2) | Re-run, Run 4, verbatim | **MATCH** |
| 22 | Hardening: 28 crash-on-bad-input classes fixed (§2.4) | `tools/HARDENING-W46.md` per-tool sections: 6 + 8 + 9 + 5 crashes fixed = 28 (the doc never prints the total; the total is its sections' sum) | **MATCH** |
| 23 | Fixtures: 6/7 load; fx-textures failed by design (§2.4) | `scenes/fixtures/FIXTURES.md` (fx-textures fails by design; the other six load) | **MATCH** |
| 24 | Audit W40: 90 sampled, 87 match, 3 mismatch (§2.7) | `docs/integration/AUDIT-W40.md` totals (Total 90; 87 match) | **MATCH** |
| 25 | Audit W42: 165 sampled, 162 match, 3 mismatch (§2.7) | `docs/integration/AUDIT-W42.md` (165 values; 162/165 per its record) | **MATCH** |
| 26 | Audit W61: 140 sampled, 138 match, 2 mismatch (§2.7) | `docs/integration/AUDIT-W61.md` (140 / 138 in its totals) | **MATCH** |
| 27 | Audit totals: 395 sampled, 387 match, 8 mismatch (§2.7, §5.2) | Arithmetic over rows 24–26: 90+165+140 = 395; 87+162+138 = 387; 3+3+2 = 8 | **MATCH** |
| 28 | Grade time: 7 carriers (s02–s07) zeroed (§2.7) | Census this audit: all 14 scene files now carry post.grade.time = 0.0 | **MATCH** |
| 29 | Saturation residue: 12 of 14 files spell `saturation`; only s03/s06 use `sat` (§2.7, §4.1) | Census this audit: `sat` in exactly s03 + s06; `saturation` in exactly 12 files | **MATCH** |
| 30 | s06 fire 0.6333623112323646 (f64) (§2.7) | Stored f32 in the s06 file = 0.6333622932434082 (the f32 cast of the claimed f64), read this audit | **MATCH** |
| 31 | Sign painter: "TANIGUCHI BOOKS" at fs = 40, no shrink (§2.2) | `painters/picture-sign.md`: "fs = 40 is PIL-measured" | **MATCH** |
| 32 | Paint cache: 24 ≈ 700 MB vs ~4.6 GB worst case at 160 (§2.6, §3) | `e5860b4` commit message, verbatim: "runs at 24 = ~700 MB instead of the 4.6 GB worst case" | **MATCH** |
| 33 | Decision (B) recorded in commit `801f85c` (§1) | `git log 801f85c`: "Integration: record JPEG decision (B) in contract §5 + film capability inventory" | **MATCH** |
| 34 | Commits since base `34d7882` at the report's HEAD: 11 lead + 7 wave = 18 (§5.1) | `git log 34d7882..3781eb2` enumerated this audit: exactly the 11 lead hashes and 7 batch hashes the report lists | **MATCH** (at the report's stated HEAD `3781eb2`) |
| 35 | CI run 43569768582 for `e5860b4` = success (§4.5, via RENDER-READINESS) | `gh run view 43569768582` this audit: conclusion "success", headSha e5860b48… | **MATCH** (now first-hand, not just cited) |
| 36 | Skill work copy: 0 files differ, 7 additions (§2.5, §4.4) | `docs/integration/RED-TEAM-W70.md`: recursive diff 1663 vs 1671 files, 0 differing common files (8th extra = the review file itself) | **MATCH** |
| 37 | Reference set: exactly 60 stills, t0.50…t59.50, no gaps (§4.2) | Census this audit of the project's `review/frames/`: 60 files, first 0.50, last 59.50 | **MATCH** |
| 38 | Push state (§4.5): origin/main at `d8fbe83`, local main ahead by `3781eb2`; total 18 commits since base (§5.1) | This audit: origin/main = main = `69952ab`; 19 commits since base | **DIFFERS — stale by concurrent events, not wrong at writing.** The push the report awaited (§4.5 tags it [TO COMPLETE]) has since happened and one further commit landed during this gate; the report's present-tense push paragraph and the "18" total are superseded and should be refreshed in the close-out pass. |

## Summary

**Gate: 6/6 PASS** at the tree as found (`3781eb2` at start) —
validator 7/7; caption regeneration byte-identical over 67 files;
golden 4 OK / 0 STALE / 14 MISSING (OK = pin matches its own bytes;
the 14 film mid-frames await the lead's render); bridge 21/21 with
94 files hash-verified; 32/32 scene JSONs parse; painter tests pass
in their production form, 45/45, including the re-pinned felt shade
agreement test.

**Numbers audit: 38 claims — 37 MATCH, 1 DIFFERS.**

- The one DIFFERS (row 38) is the push-state paragraph: events
  overtook it during Batch 8 itself (origin/main is now `69952ab`
  and pushed), and the report already tags that item [TO COMPLETE]
  for the close-out pass.
- One sub-figure could not be tied to a first-hand source: the
  caption width mean Δ 0.054 px (row 8) — its sibling figure,
  max Δ 0.15 px, verifies exactly in WAVE-RECORD.md:123, and no
  contradicting value exists anywhere checked.
- The brief's anticipated painter-test question is settled
  first-hand: the production tests pass in full (45/45); the
  wave's staging harness still reads 51/52 because `d8fbe83`
  re-pinned the felt test only in `src/scene.rs`, leaving the
  superseded `assert_ne!` in `painters/felt.rs` — precisely the
  residue FINAL-REPORT §2.7 names. Reconciling that staging copy
  is a one-line wave-side edit whenever the wave chooses to make it.

Nothing in this gate modified any file except this report; nothing
was committed.
