# WAVE-CLOSE — the 100-agent wave's own ledger

> Written by seat W97 (Batch 10), 2026-11-14T09:12Z, at HEAD
> `8af40eb` (local `main`, ahead of `origin/main` at `ede0e92` by
> exactly the Batch-9 commit). Batch 10 was still in progress at
> writing; its central commit lands after this document and is
> recorded in the FINAL-REPORT's final tallies, not here.
> Audience: the user, the lead/integrator, and any future session
> picking this repo up cold.

---

## 1. The order and the operating model

- **The order** (user, main chat, 2026-11-14 12:20–12:21 +06,
  confirmed via options widget): *"Assign 100 agents to write all
  planned code then push"* — project: TaleRender production
  integration (the Onoda short, "THE SOLDIER WHO DIDN'T KNOW THE
  WAR WAS OVER", as a full native film). The user added: *"next I
  will tell you what to do"* — so the wave completes, pushes,
  reports, and then **holds**.
- **Shape:** 100 wave seats in **10 batches of 10** (the real
  parallel ceiling), run by an orchestrator in this chat.
- **Ownership:** the production-integration seat acted as
  **lead/integrator** throughout, with sole ownership of the core:
  `src/*.rs`, `src/*.wgsl`, `docs/scene-format.md`,
  `docs/integration/README.md` + `INVENTORY.md`. Wave seats wrote
  only their assigned new paths, never `src/`, and **never
  committed**; the orchestrator validated and committed centrally
  between batches, explicit paths only — the lead's `src/` work
  was never swept into a wave commit.
- **Laws in force:** builds only via GitHub Actions CI (this VM
  has no cargo/rustc in PATH and its llvmpipe JIT segfaults on the
  PBR shader path); every CI verdict reported the moment it was
  checked; decision **(B)** — native frames stay clean, no q0.93
  JPEG matching (user, 2026-11-14); rulings **R1–R5** (§14.5);
  tolerances stated before measuring; images labelled by renderer
  (Chrome SwiftShader / native lavapipe / native CI lavapipe /
  PC RADV); no performance claims from this VM.
- **Base:** the wave started from a clean tree at `34d7882` with
  CI run `43564181258` GREEN.

---

## 2. Batch ledger

### Batch 1 — W01–W10 → `0972e2c`
The film as data: 7 atmosphere presets + six GRADE filmLooks
(grain time pinned 0), the caption pipeline (**66 chunks, min fit
0.9227, max rendered width 960 px, 7 chunks over the 850 floor, 0
under 0.68** — matching Chrome's own audit headline), scene files
S01/S04/S05a/S05b/S06/S07 + calendar data, and the reusable shore
set (229 leaf meshes, gaps G1–G8 registered).

### Batch 2 — W11–W20 → `532dc5f`
All **14 segments authored as format-v2 data** (S01, S02, S03,
S04, S05a, S05b, S06, S07, S08, S08b, S09, S10, S11, S12); S10
bookshop set with **310 books** across 360 slots; the S11→S12
rifle hand-off derived independently by two seats to the same
transform (position [0.62, 0.07, −0.1], euler [0, −0.5, π/2]).
Film manifest tiles **[0, 1811)** exactly at 30 fps; validator
`tools/film/validate_film.py` created.

### Batch 3 — W21–W30 → `311656d`
Registry painters staged in `painters/` (planks, wallpaper, felt,
parcel, picture, sign, calendar — corpus pins #302222/#413427
reproduced exactly); jungle corrections C1–C9 applied; the fog
package established that fog was already wired in the film path
with three's **smoothstep** factor (the unwired piece was the
loader's `fog` block); A/B tool `tools/ab/measure_ab.py`
reproduces all committed milestone metrics digit-for-digit
(23.850 / 21.820 / 84.486 dB); native golden pack stood at
4 OK / 0 STALE / 14 MISSING.

### Batch 4 — W31–W40 → `c7db74d`
**Validator 7/7 PASS** (check 3 rebuilt: 883 exact f32 + 506
source literals + 0 failures); W40's independent audit 87/90;
the painter integration pack (52/52 tests in a combined harness;
exact dispatch arms for the lead); the skill-wiring trio in the
work copy `~/workspace/talerender-skill-wiring/` (native capture
adapter + docs + selftest — the shipped skill never touched);
runbooks; WAVE-RECORD's 20-entry divergence register as the
lead's checklist.

### Batch 5 — W41–W50 → `dbf90b9`
grade.time zeroed in 7 scene files (every wrong value was the
film time at the file's evaluation instant); W42's second audit
162/165 isolated the s06 cut-rounding root cause; adapter dry
run found **no fail-open path**; tooling hardening fixed 28
crash-on-bad-input classes with byte-identical output on valid
data proven; fixtures census (7 texture generators unarmed →
whole-file load failure); bridge packager self-verifies 21/21;
PERF protocol (blank tables by design); REVIEW-W50's 20 findings
against the wave's own docs.

### Batch 6 — W51–W60 → `ce06011`
s06 data fix landed (evaluation chain first reproduced
bit-exact, then 8 values recomputed at cut(5) = 26.129); the
lead's shade root-fix **measured** from its own CI render:
23.850 → **23.96 dB**, mean |Δ| 5.20056 → 4.82505; doc-truth
fixes incl. NATIVE-BACKEND's overclaim replaced with the true
state; adapter hardening F1/F3/F4/F10 applied and verified (the
telemetry PASS path exercised end-to-end via stub binaries);
captionsNote census completed; tools-ci workflow authored;
RENDER-READINESS checklist; proof-package skeleton.

### Batch 7 — W61–W70 → `3781eb2`
Third audit 138/140 (one root: sea metalness 0.08 missing in
s02/s03); consolidated gate **7/7 PASS**; fog-chart Chrome
harness **run for real** — smoothstep confirmed within 1 code
at all depths, standard-panel offset explained as a ×π light
convention; determinism byte-compare pack stub-proven in both
directions; golden re-pin packets (ab1 candidate in hand);
adoption gaps closed in the work copy. **Red team: 7 claims
reproduced, 1 refuted** — the "52/52 painter tests" claim had
become 51/52 because one felt test pinned the divergence the
lead's shade fix deliberately abolished; the lead re-pinned it
(`d8fbe83`). This batch also surfaced a file-integrity defect:
the on-disk adoption review was truncated mid-sentence (its
§§3–4 existed only in the seat's report); restored in-batch,
marked as a restoration, losses named.

### Batch 8 — W71–W80 → `743d248`
Loader acceptance census **13/13 texture generators armed**
(CI-tested green, 71/71 at `d8fbe83`); the saturation silent-
default characterised (12 files grading at the 1.12 default on
the loader lane, film unaffected); fog loader verdict (c)
*ignored entirely* + a mechanical wiring checklist — which the
lead overtook with its own implementation (`69952ab`, verified
≤1 code in `5b0889d`); sea metalness restored from source
(`shots.ts:205`); **the film A/B filled in: 60/60 pairs, mean
21.836 dB**, s01 anchor 24.094, s06 fog risk refuted — plus two
findings referred to the lead (the s08/s08b/s09 broad-delta
cluster and s11 lowest at 18.523; the paint-cache boundary
effect at frames 1195–1199, 72.34 dB, |Δ|=1 class); determinism
pack currency 8/8; gate 6/6; FINAL-REPORT numbers audit 37/38
(the one DIFFERS was push state moving during the audit).

### Batch 9 — W81–W90 → `8af40eb`
saturation→sat rename in 12 files (values byte-unchanged,
validator 7/7); staging painter reconcile (52/52 staging +
45/45 integrated); tools-ci validator fix — the CI failure at
`381ff47` was environmental (check 3 greps a project tree that
doesn't exist on runners → 506 false failures), fixed with a
source-absent mode that reports literals UNVERIFIED, never
silently passes, and still fails a planted bad value;
**MP4 finish DONE** (1,811 counted frames, 60.457 s, mux
loudness −14.00 LUFS vs the mix's −13.99 — evidence in
`docs/ab/film/MP4-EVIDENCE.md`; one colour-tag finding recorded
for the lead); **film telemetry verdict recorded: CHECK FAILED
on the 7 caption chunks only** (overflow none, continuity
clean) by an independent cache-24 run agreeing verbatim with
the lead's own logs; fx-textures renders byte-identical local
vs CI; fog-chart native gate independently reproduced from CI
artifacts (≤1 code); 14 golden mids staged hash-verified
(pinning stayed human); README proof-patch proposal for the
lead; gate 6/6.

### Batch 10 — W91–W100 → central commit pending at writing
In progress at writing. Seats (per the orchestrator's launch):
W91 FINAL-REPORT completion pass; W92 caption decision pack
(the 7 over-floor chunks — analysis only, nothing applied);
W93 second red team on the final claims; W94 adoption decision
one-pager (work copy); W95 runbook verification against the
final binary; W96 determinism formal-run attempt; W97 this
ledger; W98 goal/memory close-out drafts; W99 final gate
(`GATE-W99.md`, running at writing); W100 pre-push checklist.
Close-out after the batch: orchestrator pushes, verifies CI
first-hand (ci.yml + tools-ci.yml), updates the goal/memory
records, reports, and the wave **holds for the user's next
instruction**.

---

## 3. Agent accounting — seats vs runs, honestly

- **Ordered: 100 seats.** Batches 1–9: 90 seats, all reported.
  Batch 10: 10 seats in progress at writing.
- **Runs exceed seats where seats ended silently.** Three Batch-7
  seats ended without a usable report and were recovered from
  disk + replacement/finisher runs: **W62** (replacement),
  **W67** (replacement audited the first attempt's on-disk
  files), **W68** (replacement + a finisher seat). Batch 7
  therefore took **13 runs for 10 seats** (per FINAL-REPORT
  §5.1). No replacement runs were recorded for Batches 1–6,
  8, or 9 (per the orchestrator's batch reports).
- **Total: 103 agent runs for the 100 seats through Batch 9**,
  plus Batch 10's runs (final count lands in the FINAL-REPORT's
  closing tallies). The lead/integrator seat is additional to
  the 100 and is not counted in either figure.
- Nothing was padded: every count above is seats that actually
  ran and reported (or were recovered from disk and verified).

---

## 4. Verdicts register

One line each; the owning doc carries the evidence.

| Verdict | Result | Record |
|---|---|---|
| Film data validator | **7/7 PASS**, repeatedly, incl. on the final renamed tree (883 exact f32 + 506 literals + 0 failures; tiling exact at 1,811) | GATE-W62/W80/W90, tools/film |
| Caption determinism | Regeneration **byte-identical** (66 strips + manifest) | gates W62/W80/W90 |
| Data audits (W40+W42+W61) | **395 sampled / 387 match / 8 mismatch** — every mismatch resolved, reclassified, or fix-landed | AUDIT-W40/W42/W61 |
| Painter tests | 52/52 (B4) → **51/52** at `ce06011` (red-team refutation) → lead re-pin `d8fbe83` → **52/52 staging + 45/45 integrated** (B9 reconcile) | TEST-RUN.md, RED-TEAM-W70 |
| CI test runs, loader lane | `c74cf0c` run 70/71 (the stale felt assertion) → `d8fbe83` run **green 71/71** | LOADER-ACCEPTANCE-W71 |
| Consolidated gates | W62 **7/7 PASS**; W80 **6/6**; W90 **6/6** | GATE-W62/W80/W90 |
| Numbers audits | W80: **37/38 MATCH**; W90 pre-close: 6 MATCH / 6 STALE, all STALE concurrency artefacts | GATE-W80/W90 |
| Red team (W70) | **7 reproduced / 1 refuted** (the refutation resolved by `d8fbe83`) | RED-TEAM-W70 |
| Adapter gate | **No fail-open found**; fails closed on stale binary / missing verdict line; PASS path exercised via stubs | DRY-RUN-W44, W54 fixes |
| Bridge packager | **21/21 PASS**, 94 files hash-verified | tools/bridge |
| Golden pack | **4 OK / 0 STALE / 14 MISSING** — the 14 now *staged, hash-verified, unpinned* | GOLDEN-STAGING-W87 |
| A/B tool control | Reproduces committed milestone metrics **digit-for-digit** (23.850 / 21.820 / 84.486 dB) | tools/ab |
| Shade root-fix (lead, measured by W57) | ab1 **23.850 → 23.96 dB**; mean \|Δ\| 5.20056 → 4.82505; %px>10 13.53318 → 11.44483 | docs/ab/composer-v2 |
| Film render (lead, CI) | 10 dispatch runs at `381ff47`, all success; artifacts tile **[0, 1811)** | docs/ab/film/RESULTS.md |
| Film per-shot A/B | **60/60 pairs; mean 21.836 dB, min 18.378**; s01 mean 25.005 (anchor pair 24.094); s11 lowest 18.523; s06 fog hypothesis refuted | docs/ab/film/RESULTS.md + ab-report |
| **Film telemetry** | **`FILM TELEMETRY: CHECK FAILED (0 overflow groups, 0 continuity violations, 7 caption failures)`** — two independent runs (lead's logs; W85 at cache 24, CI binary @`5b0889d`) agree verbatim; all 7 widths independently reproduced | docs/ab/film/TELEMETRY-W85.md |
| MP4 finish | **DONE** — counted frames **1811**, 60.457 s, **−14.00 LUFS** vs mix −13.99 (Δ0.01), faststart ✓, sha256 `f357a004…`; finding: colour primaries/transfer tags landed `unknown` despite finish.sh's flags (lead's call) | docs/ab/film/MP4-EVIDENCE.md |
| Fog chart, Chrome side | Static verify **144/144**; basic panels **≤1 code** vs analytic smoothstep at all depths (linear ramp would miss by up to 26) | tools/fog-chart/chrome/VERIFY.md |
| Fog, loader lane | Verdict (c) ignored-entirely at `3781eb2` → lead wired it (`69952ab`) → **≤1 code at every panel** (`5b0889d`); independently reproduced from CI artifacts (W86). PSNR leg vs the Chrome frame still open (reference-side ×π rig) | FOG-STATUS-W74 §5, LOADER-EVIDENCE-W86 |
| fx-textures fixture | Renders under the current binary; **byte-identical local vs CI** frame | LOADER-EVIDENCE-W86 |
| Loader texture census | **13/13 generators armed**; no data-named generator fails a load | LOADER-ACCEPTANCE-W71 |
| Tools CI | Run `43573712817` at `381ff47` **FAILED — environmental** (validator's source tree absent on runners, 506 false failures); fix landed B9 (source-absent mode; negative control still fails); re-verification on a pushed HEAD happens at the final push | W83, tools/CI-WORKFLOW.md |
| Push CI (spot) | `e5860b4` run `43569768582` **SUCCESS** (verified first-hand via `gh`, W80) | GATE-W80 |
| Determinism pack | Stub-proven PASS/FAIL/stale-refusal; currency **8/8** after the register rewrite; formal per-renderer PASS still open (see §6) | tools/determinism, W78 |
| Skill work copy | Drift vs shipped skill **0 files** beyond the wave additions; selftest on this VM: **ENV-BLOCKED (exit 3)** — the correct class (stale local binary detected, never misreported as FAIL) | SELFTEST-W79.md (work copy) |

---

## 5. What the wave did NOT do

- **No `src/` changes.** Every engine change in the window is
  the lead's: 15 commits since base (rulings `5e5b0a0`, paint
  cache `1475b66`, loader attachments `4c06a1e`, asset commit
  `b3f3c0a`, trapezoid fix `3e88c60`, shade root-fix `c00897d`,
  cache env `e5860b4`, register close `381ff47`, loader lane
  `c74cf0c`, felt re-pin `d8fbe83`, fog wiring `69952ab`, spec
  addendum `5b0889d`, sky dome `0f6c033`, scene-format key doc
  `ede0e92`, + the TEMP diagnostics `12c7a5e`). Wave commits:
  9 (one per batch, §2). Total since base at writing: **24**.
- **No golden pinning.** The runner refuses `--pin` by design;
  the wave staged frames and prepared packets. First pins for
  the 14 film mids and the ab1-composer re-pin remain human
  acts — and one `--pin` run would now strike both decisions in
  a single act, so they should be committed separately
  (GOLDEN-STAGING-W87).
- **No skill adoption.** The wiring lives only in the work copy;
  the shipped skill is untouched. Adoption is the user's
  decision (§6).
- **No RADV evidence.** Every native number in this ledger is
  CI/VM lavapipe or local llvmpipe. RADV (the user's PC) numbers
  — including fp16 rounding behaviour — do not exist yet and
  were never claimed.
- **No caption decision.** The 7 over-floor chunks were
  measured, reproduced, and (in Batch 10, W92) analysed — not
  re-chunked and not waived. That call belongs to the lead/user.
- **No final push yet.** At writing, local `main` is ahead of
  `origin/main` by the Batch-9 commit; the Batch-10 commit and
  the closing push + CI verification are the orchestrator's
  close-out acts after this batch.

---

## 6. Open items at close, with owners

Source: RENDER-READINESS §5 (W76 refresh) **updated by events
through Batch 9** — its items 2, 6 (rename), 7, and 8 closed
during Batches 8–9 as noted in §4; FINAL-REPORT's W91
completion pass had not landed when this ledger was written,
so where the two would differ, this list follows the events.

**Proof-chain remainders**

1. **Caption decision — owner: lead/user.** The film telemetry
   verdict is FAILED on exactly 7 caption chunks over the
   850 px floor (widest 1051 px; overflow none, continuity
   clean). Options: re-chunk the 7 at generator rules (W92's
   decision pack, Batch 10) or accept with a documented waiver
   (the chunks fit the 1080-wide frame; the floor is a
   safe-area rule). Until decided, the verdict stands as
   recorded — it is not spun as a pass anywhere.
2. **Golden pins — owner: human (user/lead).** 14 film mids
   staged + the ab1-composer candidate in hand; the s28 packet
   recommends *against* re-pinning (structural difference,
   21.711 dB — a historical prototype artifact); stroke-toon
   awaits a post-fix candidate; dof-chart is
   pending-not-required. Four staged mids (560/760/969/1593)
   carry the paint-cache boundary caveat for re-renders.
3. **Determinism, formal — owner: lead/CI + user's PC.** The
   pack is proven as an instrument; a recorded
   `DETERMINISM: PASS` is still owed per renderer of record
   (CI lavapipe and PC RADV). The honest statement is two-part:
   byte-identity proven in the tested ranges and at overlap
   frames 1200–1204; frames 1195–1199 differed at the |Δ|=1
   class past the 160-entry paint cache.
4. **Proof provenance vs `0f6c033` — owner: lead.** The proof
   frames/A/B/MP4 carry `381ff47` provenance; the lead's later
   sky-dome + atmosphere commit postdates them. Whether the
   proof re-renders at the newer head is the lead's call.
5. **s08/s08b/s09 broad-delta cluster + s11 — owner: lead.**
   Referred with worst-frame side-by-sides in
   `docs/ab/film/`; no wave-side explanation was forced.

**Loader-lane remainders**

6. **Integration README disposition — owner: lead.** Three
   REVIEW-W50 findings (verdict-string form, resume-purity
   wording, fog sentence) plus the TBD-PROOF list: a
   paste-ready patch proposal exists (README-PROOF-PATCH-W88);
   applying it is the lead's act on its own doc.
7. **FIXTURES.md alignment — owner: wave/lead (minor).** Its
   trap-1 note is stale on both halves after the sat rename +
   exposure wiring (scene-format.md was aligned by the lead in
   `ede0e92`). A validator key-spelling check for grade keys
   was proposed alongside the rename and is not yet built.

**Decisions and external evidence**

8. **Skill adoption — owner: user.** Adopt the native backend
   into the shipped skill as an opt-in alternative now, or
   wait; adoption ≠ making it the default. The adoption
   review (restored, W72) + W94's one-pager frame it.
9. **RADV evidence — owner: user's PC.** All performance and
   RADV-specific fidelity numbers (PERF protocol, fp16
   rounding, the fog-chart PSNR leg's ×π-amended Chrome
   re-capture) wait on runs there.
10. **MP4 colour tags — owner: lead.** finish.sh passes all
    three BT.709 flags; the mux carries color_space=bt709 but
    primaries/transfer `unknown` on this ffmpeg (8.1.2).
    Flag-set fix or expectation amendment — lead's call.
11. **Final push + CI verdicts — owner: orchestrator.**
    Push the Batch-9/10 commits, read ci.yml + tools-ci.yml on
    the final HEAD first-hand (tools-ci's first fully-green
    hosted run is expected there, post-W83), then close goal
    `goal_b21124e71eb3` only if the whole path verifies, and
    hold for the user's next instruction.

---

*End of ledger. Companion documents: `FINAL-REPORT.md` (the
wave's report), `WAVE-RECORD.md` (the divergence register),
`RENDER-READINESS.md` (the lane checklist), `docs/ab/film/`
(the proof package).*

---

*Addendum — orchestrator, 2026-11-14: after this ledger's stamp
(HEAD `8af40eb`) the lead landed `6ca97fc` (finish.sh BT.709 —
open item "colour tags" CLOSED), `cabcc30` (determinism root
fix: exact f32 paint-cache keys), `1fc6fab` (atmosphere
inheritance for S06/S08/S08b/S09), `f021991` (E0600 fix), and
dispatched a full re-render at `f021991`. Census at the
Batch-10 commit: 28 commits since `34d7882` = 19 lead + 9 wave
(+ the batch-10 commit = 10 wave). Detail: FINAL-REPORT.md
addendum.*
