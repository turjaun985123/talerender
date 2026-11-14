# GATE2-W10 — Wave 2 Batch-1 gate + integration packet

**Seat:** TaleRender Wave 2, W10 (batch-1 gate). **Date:** 2026-11-14.
**Gate time (final snapshot):** 2026-11-14 16:42:26Z.
**Tree state at gate time:** HEAD `6f00400` ("Wave 2 lead record:
mission, laws, stale-item register…", on top of launch HEAD
`d7bebdd`); working tree = 19 modified + 15 untracked paths, all of
them wave work (nothing committed by any seat, per the wave model).
Every check below is labelled with the tree state it ran against —
Batch 1 landed *while the gate ran*, so the film-data checks exist
in a pre-W01 and a post-W01 reading.

**Method.** Deliverables judged on disk only. Where a seat's own
record makes a claim, the claim is marked *seat-reported* unless
this gate re-ran it, in which case the gate's own output is quoted.

---

## 1. Inventory — W01–W09 deliverables

| Seat | Deliverable(s) | Present | Size | Sanity (one line) |
|---|---|---|---|---|
| W01 | Project `caps` edit (`~/faceless-shorts/the-soldier-who-didnt-know/script.json`, backup in seat scratch); scenes-side regen `assets/captions/` (73 strips + manifest); 13 `scenes/film/*.scene.json` + `film.json` caption renumbering; `docs/ab/film/RECHUNK-W01.md` | ✅ (driver-side leg open by design — §3) | record 13,789 B / 272 lines | Splits match W92 Option A exactly (7 `caps` lines, one `\|` each); record carries verdicts, the split table, and the lead's remaining steps (§6) |
| W02 | `docs/integration/EXIT-CODES-W02.md` (repo); work copy: `template/tools/capture_native.mjs` + `CAPTURE-NATIVE.md` updated, `tools/test_adapter_exit_codes.sh` (121 lines) | ✅ | spec 8,403 B | Spec implements the record §6 contract; states plainly the binary side is the lead's — seat touched no `src/` |
| W03 | `docs/golden/frames-f021991/` (14 PNG + `RENDERER.txt`) + `docs/integration/GOLDEN-RESTAGE-W03.md` | ✅ | frames 39 MB total; record 14,276 B | Provenance by hash (set identified as f021991 via its A/B table + MP4 byte-identity); **nothing pinned**; old staging untouched |
| W04 | `docs/integration/DETERMINISM-CONTRACT-W04.md` + `.github/workflows/determinism.yml` | ✅ | 14,978 B / 293 lines; yml 3,538 B / 89 lines | Contract carries the no-cross-vendor-bit-identity clause, per-vendor goldens, PSNR-band tolerance; yml is dispatch-only, ubuntu-22.04, runs the pack (W96 discharge route 1) |
| W05 | `src/errors.rs` + `docs/integration/ERRORS-W05.md` | ✅ | 367 lines / 14,601 B; note 8,872 B | `TaleError` + `TaleResult<T>`; `exit_code()` mapping is §6 verbatim (DataLoad→2, GPU family→3, ImageIo→4; 1 reserved, never an error) |
| W06 | `src/paintbudget.rs` + `docs/integration/PAINT-BUDGET-W06.md` | ✅ | 422 lines; note 10,708 B | `PaintBudget`: byte accounting, budget+entry-cap eviction policy, `stats_line()`; header argues pixel-neutrality from the exact-key law |
| W07 | `src/gpucore.rs` + `docs/integration/GPU-CORE-W07.md` | ✅ | 411 lines; note 10,024 B | Single canonical instance/adapter/device/target/readback/`save_png`; survey names the five duplicated sites and the migration order |
| W08 | `docs/integration/DOC-SWEEP-W08.md` + tracked edits: `docs/risk-register.md`, `scenes/fixtures/FIXTURES.md` | ✅ | note 13,516 B; diffs +151/−0 and +22/−0 | Dated refresh line per risk (original text preserved verbatim); FIXTURES trap-1 finally corrected (a Wave-1 open item) |
| W09 | `src/runlog.rs` + `docs/integration/RUNLOG-W09.md` | ✅ | 327 lines; note 9,956 B | `RunClock` (elapsed/ETA), peak-RSS self-report, `TALERENDER_FRAME_TIMING` switch — PERF §9.1 + §9.3; std-only, log-only |

No seat is MISSING. W01 landed last (first repo-side files
16:40Z); its remaining leg is scheduled inside its own record, not
absent (§3).

## 2. Mechanical checks — this gate's own runs

### 2.1 Film validator, full mode (project source tree present)

*Pre-W01 state (HEAD 6f00400, film data at 66 chunks), ~16:31Z:*

```
[5 captions] PASS — 66 chunk listings, 66 unique chunks covered of 66, 0 duplicates, 0 uncovered
OVERALL: PASS (7/7 checks passed)   — 1389 numbers: 883 exact f32, 506 source literals, 0 not round-tripping; tiling exact at 1811 frames
```

*Post-W01 scenes-side state (same HEAD, W01 data in tree), 16:47Z:*

```
[1 parse] PASS — 14 present, 0 missing, 0 unparsable
[2 version] PASS — 14 files checked, 0 wrong
[3 f32] PASS — 1389 numbers checked: 883 exact f32, 506 source literals (pass), 0 computed values not round-tripping (fail)
[4 attachments] PASS — 0 top-level violations, 10 attachments on character nodes
[5 captions] PASS — 73 chunk listings, 73 unique chunks covered of 73, 0 duplicates, 0 uncovered
[6 durations] PASS — sum(durations) = 60.357 s -> 1811 frames (total 1811); tiling exact
[7 atmosphere] PASS — R5: declared presets match the manifest for 14/14 files; (set, preset) -> filmLook resolves for 14/14
OVERALL: PASS (7/7 checks passed)
```

Identical to W01's seat-reported run. **Tooling note:** W01 also
edited `tools/film/validate_film.py` — `EXPECTED_CHUNKS` 66 → 73,
with an in-code comment disclosing that check 5 validates the
*scene* files and that `film/onoda.json` still carries 66 pending
driver-side regeneration. The change moves the expectation *with*
the data and is disclosed, not a loosened check — but it is a
checker edited by the seat whose data it checks; the lead should
eyeball the diff (it is small and exactly as described here)
before the central commit.

### 2.2 Caption determinism (regenerate → `diff -r` vs `assets/captions`)

- Pre-W01 (66 chunks): **byte-identical**, 67 files.
- Post-W01 (73 chunks): **byte-identical**, 74 files (73 strips +
  manifest). Generator's own audit line, verbatim:
  `CAPTION STRIPS: 73 chunks, min fit 1.00, max rendered width 835 px, 0 over the 850 px floor, 0 under the 0.68 fit floor`.

### 2.3 The ink-width question (investigated, bounded)

The gate's audit measure is PIL ink bbox (α > 32) — W85/W92's
method. Over the *scenes-side* strips post-re-chunk, two chunks
measure over the floor: cap-050 "Onoda still refused" ink **859**
(width field 834.8) and cap-053 "he would only stop" ink **852**
(width field 827.75). Neither is a split chunk.

This does **not** transfer to the gate. The strips the telemetry
audit reads are the *driver-side* set (`film/assets/cap_*.png`),
a different rasterisation: for the identical texts/widths, the
current driver-side strips measure ink **844** and **837**
(cap_046 / cap_049, 66-era numbering) — ≈15 px narrower, both
under the floor, consistent with Wave 1's audit never flagging
them. The re-chunk does not change these two chunks' text or
width at all.

**Residual watch item, stated precisely:** W01's expected
`CHECK OK` is argued in generator widths (max 835). The ink
evidence supports it — the tightest surviving chunk ("Onoda
still refused") sits at driver ink 844, ≈6 px of headroom, and
all seven new split pieces are ≤699 rawW per W92's computation —
but the verdict counts only when the lead re-runs the gate
against the regenerated driver data (W01 record §6 steps 1–2).
If that re-run surprises, these are the two chunks to look at.

### 2.4 `determinism.yml` — python yaml-parse

```
YAML PARSE OK; top-level keys: ['name', True, 'env', 'jobs']; jobs: ['determinism']
```

(The `True` key is the standard YAML-1.1 reading of `on:` —
GitHub parses it correctly.) Job content spot-checked: pinned
ubuntu-22.04, lavapipe ICD via `VK_ICD_FILENAMES`, invokes
`tools/determinism/determinism_check.sh` — W96's discharge
route 1, as the contract requires.

### 2.5 New `src/` modules — test presence + standalone rustc

Standalone toolchain present: `rustc 1.99.1`
(`~/.rustup/toolchains/stable-…/bin/rustc`). All four modules
carry `#[cfg(test)]` (errors 10 test fns, paintbudget 12,
runlog 12, gpucore 5). This gate compiled and **ran** the
std-only modules:

| Module | Standalone result (this gate's run) |
|---|---|
| `paintbudget.rs` | **12 passed, 0 failed.** One dead-code warning (`DEFAULT_ENTRY_CAP` unused) — pre-wiring API surface, acknowledged in W06's note |
| `runlog.rs` | **12 passed, 0 failed** |
| `errors.rs` | **10 passed, 0 failed** — with a stand-in `image` crate (the module's `From<image::ImageError>` needs the real crate). First stand-in attempt went 9/10: the failure was the *stand-in's* Display discarding the io message the test asserts on; with a faithful stand-in (message-carrying, as the real crate behaves) all 10 pass. Matches W05's seat-reported method and result |
| `gpucore.rs` | **CI-pending.** Standalone compile stops at `E0433: cannot find crate wgpu` (fully-qualified `wgpu::` paths throughout) — name resolution, i.e. the file parses; its 5 tests are pure arithmetic (row padding, strip-padding) and can only execute once CI compiles the crate with `mod gpucore;` wired. The lead's first wired CI should be watched for exactly these 5 |

### 2.6 W02 adapter stub suite — re-run by this gate

Command: `bash ~/workspace/talerender-skill-wiring/tools/test_adapter_exit_codes.sh`
(real adapter from the work copy against stub binaries; fixture in
a temp dir; nothing real touched). **Full output: 10 passed,
0 failed; harness exit 0.**

```
PASS: check/old-ok.sh    -> rc=0 (want 0) line='CHECK OK'
PASS: check/old-fail.sh  -> rc=1 (want 1) line='CHECK FAILED'
PASS: check/new-ok.sh    -> rc=0 (want 0) line='CHECK OK'
PASS: check/new-fail.sh  -> rc=1 (want 1) line='CHECK FAILED'
PASS: check/disagree.sh  -> rc=1, no CHECK OK line, fail-closed message
PASS: check/error3.sh    -> rc=1, no-verdict failure names exit 3
PASS: frames/old-fail    -> rc=1, gate stop message, zero artefacts
PASS: frames/disagree    -> rc=1, zero artefacts
PASS: cues refusal intact -> rc=1
PASS: fixture tree identical after all runs (no stray artefacts)
stub-gate proof: 10 passed, 0 failed
```

The five briefed stubs (old-ok, old-fail, new-ok, new-fail,
disagree) all pass, plus W02's bonus error stub and the
frames-mode/cues/fixture-purity checks. The dual convention
holds: the adapter reads the verdict *line* under both binary
generations, treats an exit-1-next-to-OK-line as fail-closed
disagreement, and creates zero artefacts on any failed gate.

### 2.7 W03 candidates — hash spot-verification (this gate)

- Staged `frames-f021991/s11.png` sha256 `231fc172…` ==
  `~/workspace/film-final/frames4/frame_01593.png` == the
  candidate hash printed in GOLDEN-RESTAGE-W03.md's table.
- Staged `frames-f021991/s04.png` sha256 `f135462d…` ==
  `frames4/frame_00560.png` == the doc's table.
- All 14 staged files are 1080×1920 RGBA PNGs; `RENDERER.txt`
  present. The seat's provenance chain (frames4 ⇐ A/B mean
  23.02 dB + the set's MP4 byte-identical to the delivered
  f021991 proof) is internally consistent with the lead's
  completion report.

## 3. Per-seat verdicts

| Seat | Verdict | Basis |
|---|---|---|
| W01 | **ACCEPT-WITH-NOTES** | Scenes-side chain fully landed and re-verified by this gate (validator 7/7 at 73, determinism byte-identical, splits = W92 Option A verbatim). Notes: (a) driver-side regeneration + telemetry re-run + bridge repackage are the lead's integration steps (record §6) — the gate verdict is still the 66-chunk FAILED line until they land; (b) the validator-tool diff needs the lead's eyeball (§2.1); (c) ink headroom watch item (§2.3) |
| W02 | **ACCEPT** | Spec = record §6 contract; stub suite re-run by this gate: 10/10, both conventions, fail-closed proven |
| W03 | **ACCEPT** | Candidates hash-verified first-hand (2/2 spot checks, full-length table in the record); pin discipline respected — nothing struck |
| W04 | **ACCEPT** | Contract covers same-renderer scope + cross-vendor tolerance with no bit-identity promise; yml parses and wires the pack's real CI route |
| W05 | **ACCEPT** | Exit-code mapping = §6 exactly; 10/10 tests reproduced by this gate (stand-in method documented in §2.5) |
| W06 | **ACCEPT** | 12/12 tests run green by this gate; byte accounting + policy match the audit's §A ask; pixel-neutrality argued from the exact-key law |
| W07 | **ACCEPT-WITH-NOTES** | Module + survey complete and coherent; its 5 tests are CI-pending (wgpu dependency, §2.5) — first wired CI must show them |
| W08 | **ACCEPT** | Refresh is append-only and dated (+173/−0 across both files); the Wave-1 FIXTURES.md trap-1 debt is closed |
| W09 | **ACCEPT** | 12/12 tests run green by this gate; delivers PERF §9.1/§9.3 as a log-only module |

**Batch-1 gate overall: PASS** — all nine seats landed, every
runnable check this gate could execute is green, and the two
open threads (W01's driver-side leg, W07's CI-pending tests) are
scheduled integration steps with named owners, not defects.

## 4. Conflicts & coordination findings

1. **W06 vs W09 stats ownership — NO conflict; pre-agreed
   split, verified in code.** runlog.rs's header states the
   contract: W06 *counts* paint-cache events (hits/misses/
   evictions/bytes, emitted as its own `paint-cache: …` segment
   via `PaintBudget::stats_line()`); W09 owns time + process
   memory (`RunClock::summary_suffix()` = elapsed + peak RSS)
   and explicitly "neither side formats the other's numbers".
   The composed end-of-run line is a **call-site** duty: runlog
   compiles standalone with no reference to paintbudget, and its
   doc comment fixes the composition order (counts → time/RSS
   suffix → W06 segment last). The lead wires the composition;
   there is no duplicate definition to reconcile.
2. **W02 ↔ record §6 ↔ W05 — one consistent contract.** The
   record's open coordination question ("the adapter may need a
   matching update so a CHECK FAILED run (exit 1) is read as a
   verdict, not a crash") is already answered by W02's adapter +
   stub proof (§2.6: new-fail → adapter rc 1 with the FAILED
   line relayed; disagree → fail-closed). W05's `exit_code()`
   never returns 1, matching §6's reservation of 1 for the
   telemetry verdict. No seat contradicts another.
3. **W03 vs the W87 pin warning — respected.** Candidates live
   in a *separate* directory (`frames-f021991/`); the 381ff47
   staging is untouched and still reads 4 OK / 0 STALE /
   14 MISSING. One `--pin` still must never strike both sets;
   see the packet (§5.7). Extra wrinkle from W01: staged mids
   inside the seven split windows predate the re-chunk — pin
   *after* the re-render they will grade (RECHUNK-W01 §6.4).
4. **W04's workflow vs existing CI — no interference.**
   `determinism.yml` is `workflow_dispatch`-only; it neither
   fires on push nor collides with `ci.yml`/`tools-ci.yml`
   path filters.
5. **W08's tracked-file edits — in-brief and append-only.**
   Both diffs are pure insertions of dated material (+173/−0
   total); no historical text rewritten, matching the wave's
   dated-correction convention.
6. **Waiver tension — handled as the record §5 prescribes.**
   W01 superseded the waiver in the *data* under the user's
   order, left `CAPTION-DECISION-W92.md` byte-untouched, and
   recorded that the user's final ruling at wave close can
   revert (the 66-chunk data regenerates from project history).
   Nothing in Batch 1 pre-empts that ruling.

## 5. Integration packet for the lead (landing order)

1. **Central batch-1 commit** (orchestrator): all §1 files.
   CI expectations: `ci.yml` green — the four new `src/`
   modules are not yet `mod`-declared, so cargo does not
   compile them and the test count is unchanged; `tools-ci.yml`
   green — validator (its `--no-source` runner mode unaffected
   by the EXPECTED_CHUNKS change), caption step skips with its
   usual warning on runners.
2. **W02 spec → binary exit codes** (lead; `film.rs` +
   `main.rs`): `run_film_telemetry` surfaces the verdict;
   main exits 0 (CHECK OK) / 1 (CHECK FAILED), `Err` → 2;
   verdict line byte-unchanged. *Then* re-prove the adapter
   against the **real** binary both directions (Wave-1 W99
   method; W02's stubs already prove the adapter logic).
   Safe order: the adapter in the work copy already handles
   both generations, so the binary change cannot strand it.
3. **W05 errors** (lead): `mod errors;`, then migrate the
   runtime `expect()` sites (adapter/device/buffer-map/PNG
   save/load paths) onto `TaleError` in small steps — exit 3
   replaces the panics per §6. Independent of steps 4–6.
4. **W06 paint budget** (lead): wire `PaintBudget` into
   `film.rs`'s `PaintCache` (consult on insert, feed
   hit/miss/evict events, print `stats_line()` at end of film
   + telemetry runs). Gate after wiring: A/B spot pairs +
   the determinism probe — eviction must stay pixel-neutral
   (pure paints under exact keys).
5. **W09 runlog** (lead): `mod runlog;` + the three call
   sites in RUNLOG-W09.md (progress elapsed, summary
   composition per §4.1's order, `TALERENDER_FRAME_TIMING`
   switch). Log-only; no gate beyond CI.
6. **W07 GPU core** (lead): `mod gpucore;`, confirm CI runs
   its 5 arithmetic tests, then migrate the five entry
   points one at a time (main → ab1 → dof → scenefile →
   film) with an A/B spot-proof per migration, per
   GPU-CORE-W07.md's plan.
7. **W01 driver-side leg** (lead, per RECHUNK-W01 §6):
   regenerate `film/onoda.json` + `film/assets/cap_*.png`
   to the 73-chunk layout → telemetry re-run (the Phase-1
   `CHECK OK` attempt; §2.3 names the watch chunks) →
   bridge repackage (73 strips) → film re-render + A/B
   footnote for the ≈10/60 pairs inside the split windows.
   Blocks Phase-1 close; depends on nothing else in Batch 1.
8. **W03 pins** (human/lead acts, last): two *separate*
   commits — the 14 film first-pins from `frames-f021991/`
   and the ab1 re-pin — never one `--pin` for both; s28
   stays unpinned per its packet. Sequence after step 7's
   re-render for the mids inside split windows (§4.3).
9. **W04 determinism job** (lead): dispatch
   `determinism.yml` once — it builds its own binary, so it
   can discharge the pack's CI-lavapipe leg against the
   current head independently of steps 2–6; the RADV leg
   remains the user's-PC item, prepared not claimed.

*End of gate record. This seat edited nothing but this file
and committed nothing.*
