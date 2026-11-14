# GATE2-W70 — Wave 2 Batch 7 gate

**Seat:** W70 (Batch-7 gate). **Gate window:** HEAD `4dd6530` at open →
`f4f6ccf` at close (the lead landed F2 phases D + L+T, the blink
adoption, the §15 fold-in's follow-ups, and a tools-ci fix during the
window; no lead commit touched a seat file). **Method:** every claim
below marked *gate-run* was executed by this seat; claims marked
*seat's* are the seat's own evidence, read but not re-measured.

## Verdict: PASS-WITH-NOTES

8 of 9 seats gated (W68 absent — §8). Two material notes: W65's note
is itself truncated mid-§3 (the W43 failure signature — §4), with its
missing acceptance numbers supplied first-hand by this gate instead;
and W68's PC dry-run never landed, so PC-session readiness rests on
W36 + W64 §5 alone (§8).

| Seat | Verdict | Basis |
|---|---|---|
| W61 knobs-tail restoration | **ACCEPT** | Provenance samples all trace (§6) |
| W62 adapter re-proof | **ACCEPT** | Gate re-ran the full grid: 8/8 MATCH (§1) |
| W63 golden verify + VR triage | **ACCEPT** | Pack re-run 18/18; diff scopes via `git show` (§2) |
| W64 bench first run | **ACCEPT** | Numbers discipline verified in the text (§7) |
| W65 fold-in checker | **ACCEPT-WITH-NOTES** | Tool proven by gate runs (§3); **the note is truncated** (§4) |
| W66 split readiness | **ACCEPT** | `state_key` freeze reproduced by gate extraction (§5) |
| W67 doc-fix pack 2 | **ACCEPT** | Headline spot-checks gate-run (§9); full 30-row audit is the seat's |
| W68 PC dry-run | **NOT GATED — file absent** | §8 |
| W69 caption decision brief | **ACCEPT** | Chunk texts + tiling reproduced from git data (§10) |

---

## 1. W62 — adapter re-proof (gate re-run)

The gate re-ran `tools/ci/fixtures/adapter_reproof/run_reproof.sh`
in full (duty asked for ≥ 2 cells, one per generation; the grid is
cheap and all 8 are reported). The adapter under test was copied
fresh from the work copy; its sha256 at gate time is
`a66153856ddddac30e…` — **identical to the value W62 recorded**.

Gate-run summary, verbatim: `8 matched, 0 mismatched, 0 unproven
(of 8 cells)`, every cell MATCH on attempt 1 (gate walls 0.32–1.99 s;
loadavg ~13.0–13.3 during the run — contention comparable to the
seat's, disclosed the same way). Per-cell gate verdicts equal the
seat's table exactly: old × OK accepted rc 0; old × FAILED refused
rc 1 on the line despite binary exit 0; old × could-not-run fail
closed rc 1; new × OK rc 0; new × FAILED rc 1; new × could-not-run
fail closed rc 1 with exit 3 named; strip mismatch refused at
pre-validation in both modes, zero artefacts in frames mode.

**W62 ACCEPT.** The corrected record's re-proof now stands on two
independent runs (the seat's + this gate's) of the convention logic.
Scope limit unchanged and restated: stubs prove the adapter's logic;
real-binary behaviour is the lead's complementary rows.

## 2. W63 — golden verification (gate re-run)

Gate-run at HEAD `2c41313` (since superseded by `f4f6ccf`, docs/CI
commits only): `python3 tools/golden/make_golden.py` →
**18 OK / 0 STALE / 0 MISSING of 18**, exit 0 — the expected values,
at a head three lead landings past W63's own verification point.

Pin-diff scopes, gate-verified via `git show`:

- **Act 1 (`916a109`):** `golden-hashes.json` gains exactly the 14
  film lines; the ab1-composer line changes only by its trailing
  comma (value `d7bd71a0…` unmoved); exactly 3 frame PNGs replaced
  (s07, s08, s11 — the in-window mids, new hashes `7f8e1555` /
  `3e8e703f` / `7bed0da5`). ✓ as W63 §2 records.
- **Act 2 (`166873c`):** exactly one hash line — ab1-composer
  `d7bd71a0…` → `60c32c1a…`. ✓
- **Corrective (`2755ff3`):** 10 frame PNGs (s01–s06, s08b, s09,
  s12) + exactly 20 changed hash lines = the 10 re-pinned film
  lines; film-s10 unmoved (the identical-bytes control). ✓ W63's
  headline stands: the original Act 1 pinned 381ff47-era staged
  bytes for the 11 out-of-window mids, the VR canary caught it
  (film-s08b DRIFT 24.85 dB against a pin whose rendered bytes were
  the intended candidate), and the corrective is the one legitimate
  re-pin — provenance broken, candidate bytes independently on
  record beforehand.
- **W63's provenance correction is gate-confirmed:** Act 2's commit
  message cites render run `43621168728` "at `9f497c7`"; `gh run
  view` records that run's head as **`dc0e555`** (success). Off by
  one commit in the record; substance (fresh post-migration
  integration-head render) stands. One-line record fix owed (§11).

VR conclusions, gate-confirmed via `gh`: run `43624398371`
(@ `b6ee9fb`) completed/**failure** — 2 DRIFT (s28 + the s08b pin
error); run `43625139818` (@ `2755ff3`) completed/**failure** —
s28 the **sole** DRIFT (21.65 dB, byte-identical inputs in both
runs). The post-pin steady state is exactly what W37 predicted:
**VR red on s28 alone.** W63 §7's disposition options (re-pin /
retire / leave) are precise; the choice remains a human act (§11).

## 3. W65 — fold-in checker (gate re-runs; brief's premise updated)

The brief expected the current `docs/scene-format.md` to be pre-fold.
It is not: the lead landed the fold-in at `4dd6530` during the batch
(the seat's replacement run handled this correctly, reporting both
states). The gate therefore ran the checker against the pre-fold
snapshot for the MISSING expectation, and against the live document
for the acceptance the note asserts:

- **Pre-fold** (`git show 3a3a427:docs/scene-format.md` extracted,
  gate-run): `OVERALL: FAIL (0/75 FOUND, 5 ALTERED-SUSPECT,
  70 MISSING)` — **exactly** W65 §3's recorded baseline, suspects
  included.
- **Post-fold** (live `docs/scene-format.md`, gate-run):
  `OVERALL: PASS (75/75 FOUND, 0 ALTERED-SUSPECT, 0 MISSING)` —
  the `4dd6530` fold-in is complete against the W31 draft's full
  census (11 headings + 46 normative sentences + 18 named rules).
  This is the acceptance result W65's §4 asserts; its text is
  missing from the note (§4 below), so the numbers of record are
  this gate's.
- **Fixtures** (gate-run `tools/format/fixtures/foldin/run_tests.py`):
  `SELF-TEST: PASS 3/3` — folded-full all FOUND exit 0;
  folded-altered flags the dropped cycle clause MISSING and the
  weakened "never" clause ALTERED-SUSPECT (strong keyword lost),
  exit 1; missing target exit 2 with an `ERROR:` line. The
  weakened-clause flag works as specified.

## 4. ⚠ GATE FINDING — FOLDIN-W65.md is truncated (same signature as KNOBS-W43)

W65's note as landed is **8,310 bytes / 173 lines and ends
mid-sentence inside §3's suspect table** (the R-gradetime row),
followed by the literal marker `...[truncated 3072 chars]` with no
§4. The introduction's claim — "the run against the current document
… passes §4's bar" — points at a section that never reached the
repo. This is the KNOBS-W43 transport-truncation class (3,075 chars
there, 3,072 here), second occurrence in the wave.

Disposition: the **tool and fixtures are complete and proven**
(§3 above — the gate's runs stand in for the missing §4), so the
seat is ACCEPT-WITH-NOTES, not rejected. Owed: a restored §4 (the
post-fold run's itemisation + the note's closing sections) from the
seat's retained report, or the record folds in this gate's §3
numbers by reference. Lead's call. Process note for the wave: any
seat note > ~8 KB should be written in append chunks; two
truncations is a pattern, not an accident.

## 5. W66 — split readiness; `state_key` freeze (gate extraction)

Gate extraction exactly as W66 describes (`awk '/^fn state_key/,/^}/'`
on `src/film.rs`):

- At gate HEAD: 25 lines, **md5 `95918b9941313f895dbee7bf35c8d14`** —
  identical to W66's claimed value.
- Gate-reproduced at `1a62161`, `86f0c81`, `ff52b3a`, `4dd6530`: the
  same md5 at all four. The frozen body (exact-f32-bit key, format
  string, 72.34 dB incident comment) survives gpucore 5/5, errors
  step 2, F2, the §15 guard, the blink adoption, and F2 L+T.
- One cite update on top of W66's table: at gate HEAD the function
  sits at **film.rs:2246**, not 2183 (+63 — the blink adoption and
  F2 landings grew the file above it). Body frozen; position keeps
  drifting exactly as W66's pattern predicts. Executors re-grep,
  per W66's own instruction.

The rest of W66 (drift census, step tables, the two executor
warnings — strike W21 §4's void adapter paragraph; re-word every
G-TEL gate to the CHECK OK / exit 0 contract) is the seat's
first-hand work, internally consistent with everything the gate
independently touched. **W66 ACCEPT.**

## 6. W61 — knobs tail provenance (gate samples)

The truncation facts are gate-verified: committed `KNOBS-W43.md`
is 263 newline-terminated lines + the literal
`...[truncated 3075 chars]` on unterminated line 264. The gate
sampled three appendix paragraphs (plus two bonuses) and traced
each into `KNOBS-PATCH-W54.md`:

| Sample | Tag | Gate trace |
|---|---|---|
| T3 (§4.2 risk: MSAA target created at count 1, never bound, 16.6 MB fp16) | [RECONSTRUCTED from W54 §4 + §9.2] | **Traced** — W54 lines 388–389 carry the fact and figure near-verbatim ("still created (with count 1) and simply never bound — 16.6 MB of fp16 at 1080×1920, created and dropped"); §9.2 item 4 at W54:937. |
| T6 (§5 gate paragraph: per-knob + combined, no DRAFT tag at defaults) | [RECONSTRUCTED from W54 §10] | **Traced** — W54:977 "This is the per-knob gate as well as the combined one"; the defaults-identity framing is §10's leg A. |
| T8 (§6 adapter risk: `--aa 8`, never clamp 8 → 4, gate does not exercise it) | [RECONSTRUCTED from W54 §9.4] | **Traced** — W54 §9.4 heading at line 953 is literally "(from W43's report, restated)"; line 958 "never clamp 8 → 4"; the gate-does-not-exercise clause follows at 959. |
| T5 (bonus: `shadow_misc` lockstep) | [RECONSTRUCTED] | Traced — W54 F5 (line 486) + A1 (line 698); the word "lockstep" is W43 §1.2's own (KNOBS-W43.md:78). |
| T1 (bonus: resolve-selection bullet) | [INFERRED — lowest confidence] | Traced to its stated basis — surviving KNOBS-W43 §1.1 lines 67–68 carry the quoted rule ("render directly into the resolved target's view with `resolve_target: None` when samples == 1"). The tag honestly marks that §4.2's original wording is inferred; the rule itself is the document's own words. |

**No sampled paragraph tagged RECONSTRUCTED failed to trace.**
W61's appendix does not overclaim: the one extension (T2) is tagged
and fenced behind a splice-time decision, and the UNRECOVERABLE
register (U1–U6) names what no source can restore — including that
the restoration is substance-level, not the original 3,075 chars.
**W61 ACCEPT.** The splice itself (with W61 §0.3's five content
spot-checks, of which the seat ran all five: 4 exact, 1 content-equal
at +22 drift) is fixes-pass work (§11).

## 7. W64 / W68 — numbers discipline

**W64 (bench): discipline PASS, verified in the text.** Every figure
sits under the run of record — GitHub Actions run **43623743480**
(gate-confirmed: completed/**success**, head `aa92cee`) — with the
verbatim renderer string printed in the note (`llvmpipe (LLVM
15.0.7, 256 bits) | backend: Vulkan | device type: Cpu | driver:
llvmpipe Mesa 23.2.1…`), a software-renderer banner up front, and an
explicit no-RADV-claim law repeated at §3. The one renderer-free
table (§1.3 frame sizes) is labelled content data — a property of
frame content, transferable to the PC only as a prior expectation,
which is how the note frames it. Anomalies A1–A3 are labelled
hypotheses/findings distinctly (A2's ~9× accounting-vs-resident
divergence is stated as an accounting-unit finding, not a memory
measurement). No unlabelled performance figure found. **W64 ACCEPT.**

**W68: no file, no numbers to discipline** — §8.

## 8. W68 — ABSENT

`docs/integration/PC-DRYRUN-W68.md` does not exist at gate close
(checked at open, mid-window, and close; no `*W68*` / `*dryrun*`
artefact anywhere in the tree). The seat produced nothing the gate
can verify. Consequence, stated plainly: the PC evidence session's
readiness currently rests on W36's runbook + W64 §5's RADV bench
addendum (both gated in their own batches). If W68's dry-run found
friction in W36's pack (stale flags, ordering, provisioning gaps),
that knowledge is not in the record — the session should be treated
as **unrehearsed** until W68 lands or the lead waives it.

## 9. W67 — doc-fix pack 2 (spot-checks)

The gate did not re-audit all ~30 rows (that audit is the seat's,
and its method — read the target doc, never the commit message — is
the right one). Headline claims gate-checked:

- "Both pin acts STRUCK; W38 texts saying otherwise are WRONG-NOW"
  — **confirmed** (§2 above; registry carries `ab1-composer:
  60c32c1a…` at line 5 of `golden-hashes.json`, gate-seen).
- Root README still has zero `--help`/`--version` hits —
  **gate-confirmed** (`grep -c` = 0), so §5's two edits + new §5.3
  pins clause are live fixes-pass items.
- The adapter-correction framing (N2) matches the `b6ee9fb`
  retraction in the git record, and §1's refreshed Addendum-2 block
  correctly refuses to propagate the retracted PASS.

**W67 ACCEPT** — it is the single doc-fix queue for the fixes pass;
apply from it, not from W38 (two W38 texts are now actively wrong).

## 10. W69 — caption decision brief (gate verification)

Gate-reproduced from the data (`git show aa10b13:film/onoda.json`
vs HEAD `film/onoda.json`, parsed field-by-field):

- Census **66 → 73** captions. ✓
- Sampled chunk 1 (s05a): old data contains the single caption
  **"Planes dropped leaflets"** (window 21.61–22.61, width 1040.44,
  fit 0.92269); HEAD contains **"Planes dropped"** (21.61–22.09,
  width 687.14) followed by **"leaflets"** (22.09–22.61, width
  335.77) — the pair tiles the parent window exactly, verbatim as
  W69's table claims. ✓
- Sampled chunk 2 (s02): old **"was a Japanese soldier"** present;
  HEAD **"was a Japanese"** + **"soldier"** present as separate
  captions. ✓
- Non-caption invariance: `lines`, `anchors`, `duration`, `fps`,
  `frames` compare **equal** old vs HEAD in the gate's parse. ✓
- `git show --stat 5e90d7e`: **60 files** (onoda.json + 59 strips),
  as the brief states. ✓

The brief is decision-ready: options, mechanical consequences of a
veto (reverts, 3 stale pins, gate returns to CHECK FAILED + the W92
waiver), and a marked recommendation (keep 73) with its honest
caveat (two width-forced mid-phrase splits). **W69 ACCEPT.**

## 11. CI state + hygiene at gate close

**HEAD at close: `f4f6ccf`** (lead: tools-ci ffmpeg install — see
below). Runs of record (gate-checked via `gh`):

| Workflow / run | Head | Conclusion |
|---|---|---|
| Tools CI `43626299876` | `f4f6ccf` | **success** |
| CI `43626299883` | `f4f6ccf` | in_progress at close |
| CI `43622944889`-lineage (errors step 2 → `aa92cee` fix) | — | resolved in-window; CI success at `2c41313` (`43625940087`) |
| Tools CI at `5a9383e` / `20a81a7` / `2c41313` | — | **failure** — sheet suite only: the hosted image has no ffmpeg, every make_sheet case exited 2 (the tool's ffmpeg-absent code). W55's pack assumed the image ships ffmpeg; the lead's `f4f6ccf` declares the dependency in the workflow and tools-ci is green at close. Strictness ratchet + migrated asset steps passed throughout — the wiring itself is sound. |
| Bench (dispatch) `43623743480` | `aa92cee` | **success** (W64's run) |
| Visual Regression `43624398371` | `b6ee9fb` | **failure** — as designed pre-corrective (2 DRIFT) |
| Visual Regression `43625139818` | `2755ff3` | **failure** — steady state: s28 sole DRIFT |
| Render (dispatch) ×3 at `2c41313`, ×3 at `5a9383e` | — | success (F2/blink gate renders) |

**Hygiene:** no seat modified a tracked file (gate-checked:
`git status` shows zero tracked modifications; untracked = the 8
seat notes + W62's fixture tree + W65's tool/fixtures + the
pre-existing `__pycache__` dirs). Nothing committed by any seat.

## 12. Integration packet for the lead (ordered)

1. **Tree health.** Main CI green through `2c41313`; tools-ci green
   at `f4f6ccf` after the ffmpeg declaration (the W55 wiring's one
   false assumption — record it in the incident register beside the
   two earlier root causes: each tools-ci red this wave traced to
   an environment assumption local runs couldn't see). CI on
   `f4f6ccf` was in_progress at gate close — its verdict lands
   after this gate; nothing in the batch predicts trouble with it.
2. **W65 note restoration.** Commission the restored §4+ of
   FOLDIN-W65 (or fold this gate's §3 numbers in by reference).
   Standing rule proposed: seat notes > ~8 KB written in append
   chunks — two truncations (W43 3,075 chars, W65 3,072) is a
   pattern.
3. **Fixes-pass queue, in the packs' own order:**
   a. **DOCFIX2-W67** — apply as its §10 orders; it supersedes W38
      wholesale (two W38 texts are WRONG-NOW). Includes the
      Addendum-2 correction language for the retracted adapter
      PASS (N2) — no §5.3-derived text may repeat it.
   b. **W43 tail splice (W61)** — delete KNOBS-W43 lines 263–264,
      append W61 §1 verbatim; decide T2 (the tagged W54 extension)
      at splice time; run W61 §0.3's five content spot-checks
      (expect drift, not contradiction).
   c. **Record fix from W63 §3** — Act 2's provenance cite:
      run `43621168728` was at `dc0e555`, not `9f497c7`. One line
      wherever the act is recorded (WAVE2-RECORD fold-in).
   d. **vr_triage register update** (W63 §8 item 2) — the register
      still describes the pre-pin world; post-pin rows print as
      NEW-SIGNAL until reconciled. Classification hygiene only;
      the tool's verdicts were correct in both triaged runs.
   e. **Split programmes (W66)** — executable as amended: strike
      W21 §4's adapter paragraph, re-word all G-TEL gates to
      `FILM TELEMETRY: CHECK OK` / exit 0, apply W66's cite tables
      (plus `state_key` at 2246 as of this gate), serial order
      film-then-scene stands. Remaining: 9 W21 commits + 13 W22
      commits + W22 step 13's staging-dir decision. Do not
      interleave with §15 step 2 or the F2 manifest-isation —
      both touch the same film.rs clusters.
4. **Decision briefs ready for the user:**
   a. **Caption re-chunk (W69)** — keep-73 vs veto-to-66, fully
      briefed with verbatim evidence (gate-verified §10).
      Recommendation on record: keep. This is the wave-close
      decision the re-chunk has waited on since Batch 1.
   b. **s28 disposition (still outstanding)** — W63 §7 states the
      three options precisely (re-pin to current ab1 output /
      retire from the registry / leave). Until a human rules, VR
      reads `FAIL (4 pins, 1 drift)` with s28 the only DRIFT row —
      verified steady state, not a new failure. The W03 packet's
      standing recommendation is against re-pinning.
   c. **v1.0.1 release** (from Batch 5's W46 pack) — procedure
      unchanged; its pre-release gates re-prove on the bump
      commit's SHA. (Carried for completeness; not a Batch-7
      artefact.)
5. **PC session readiness.** W36's runbook + W64 §5's RADV bench
   addendum (commensurable with CI run `43623743480`) are the
   session pack. **W68's dry-run is missing** — the pack is
   unrehearsed; either land W68 before the session or record the
   waiver. The session still owes: RADV determinism leg (E7),
   PERF tables, golden verification on RADV, and the resident-
   bytes-per-canvas measurement W64's anomaly A2 argues for.

— W70, Batch-7 gate. One file written (this note); nothing
committed; no existing file edited.
