# CLOSEOUT-DRAFT-W98 — goal + memory close-out texts (DRAFTS, not applied)

Seat W98, TaleRender 100-agent wave, Batch 10. Drafted 2026-11-14.
State stamped at drafting: repo local HEAD `8af40eb` (Batch 9 central
commit), `origin/main` at `ede0e92`; working tree clean; the Batch 10
central commit, the final push, and its CI verdicts happen AFTER these
drafts and are marked `[…]` where they belong. The lead/integrator agent
was still running at drafting time and its production-integration
completion report had not landed in the record; nothing below claims it.

The orchestrator applies these texts at push time. Paste-ready blocks
are fenced and labelled; everything outside the fences is drafter's
apparatus. Do not paste the apparatus into GOAL.md or MEMORY.md.

---

## DRAFT 1 — GOAL.md update

Target: `~/workspace/goals/talerender-native-rust-wgpu-renderer/GOAL.md`,
the three unchecked checklist items near the end of "Milestone
checklist", plus a new status section after the checklist. The six
checked M0–M6 items are untouched.

### 1a. Replacement for the "PRODUCTION INTEGRATION — RUNNING" item

```markdown
- [x] PRODUCTION INTEGRATION — COMPLETE WITH NAMED REMAINDERS (100-agent wave, 2026-11-14; base 34d7882; wave batch commits 0972e2c, 532dc5f, 311656d, c7db74d, dbf90b9, ce06011, 3781eb2, 743d248, 8af40eb [+ Batch 10 commit at close]): inventory of the Onoda film's 12 narration lines / 14 shot segments (docs/integration/INVENTORY.md — nothing blocked outright); scene format v2 (light rig, character node + attachments, camera moves, time; rulings R1–R5, spec §14.5); film driver (all 1,811 frames rendered in CI at 381ff47 provenance, 10 dispatch runs, artifacts tiling [0,1811) exactly); native telemetry gate RUN (verdict below); ffmpeg finishing DONE with the project's existing mix (MP4 evidence complete); proof package (per-shot A/B over the whole film, MP4 evidence, runbooks for CI + the user's PC). Remainders are listed under "Open at wave close" below — the gate's verdict is one of them, so this item is checked as *executed in full*, not as *passed in full*.
  - Film telemetry verdict (the gate exists only as its stdout line; the binary exits 0 either way): **FILM TELEMETRY: CHECK FAILED (0 overflow groups, 0 continuity violations, 7 caption failures)** — run independently by W85 with the hash-verified CI binary from run 43575835793 (head 5b0889d) at TALERENDER_PAINT_CACHE=24, and the lead's own telemetry4/telemetry5 logs end with the identical line and identical failure lines. Overflow NONE, continuity CLEAN; the only failures are 7 caption chunks over the 850 px floor (widest 1051 px; every width independently reproduced with PIL over film/assets/cap_*.png). The same 7 chunks are over the floor in Chrome's own audit — this is a film-data question (re-chunk vs waiver), not a native-port defect. Record: docs/ab/film/TELEMETRY-W85.md.
  - Per-shot A/B (docs/ab/film/RESULTS.md + ab-report): 60/60 pairs measured, native CI lavapipe @381ff47 vs Chrome SwiftShader review stills (q0.93 JPEG generation on the reference side; reported, never gated, per decision (B) clean frames). Overall PSNR min 18.378 / mean 21.836 dB; s01 anchor t=1.50 measures 24.094 dB (milestone chain: 23.850); s06 fog risk refuted by measurement (mean 24.77 dB, no depth-growing signature); s11 is the lowest segment (mean 18.523, all three worst pairs — side-by-sides in the package); s08/s08b/s09 carry a broad-delta cluster referred to the lead (s09 highest mean |Δ| 19.66).
  - MP4 proof (docs/ab/film/MP4-EVIDENCE.md): film/finish.sh verbatim (x264 slow CRF 18, project's data/mix.wav muxed unchanged) over the 1,811 assembled CI frames (hash-verified against the golden-mid table). Counted frames 1811; duration 60.457 s (audio-governed); loudness −14.00 LUFS vs the mix's −13.99 (Δ0.01); faststart confirmed. File: ~/workspace/your_files/onoda-native-381ff47.mp4 (29,722,802 bytes, sha256 f357a00434e4a8dcc4b78607a83b8010c159df86bf81b0bcb8f9cf9b00aecaed). One finding recorded, not patched: colour tags partially landed (color_space=bt709, primaries/transfer unknown despite finish.sh passing all three BT.709 flags; ffmpeg 8.1.2) — the lead's call. This is a technical-proof MP4, not a channel master.
  - Provenance caveat: the proof frames + MP4 carry 381ff47 provenance. The lead afterwards landed 0f6c033 (sky dome corrected to the source shader + atmosphere re-dressing wired) — whether the proof re-renders at the final head is the lead's call, open at wave close.
```

### 1b. Replacement for the "Scene-format character node" item

```markdown
- [x] Scene-format character node: DONE — real character node replaces ab1's code-ref character/rifle (spec §13); loader consumes the character node + attachments (4c06a1e: rifle anchored at the painted hand via syncAttachments; character-v2.scene.json reconciled to R1/R2 — post-atmosphere noon rig, fire basis 4.6, attachments ON the character node); loader-lane census at close: 13/13 texture generators armed (docs/integration/LOADER-ACCEPTANCE-W71.md), fx-textures renders byte-identical local vs CI (docs/integration/LOADER-EVIDENCE-W86.md).
```

### 1c. Replacement for the "M6 QA telemetry: cutouts() semantic gate…" item

```markdown
- [x] M6 QA telemetry: cutouts() semantic gate reproduction (contract §3) + painter hooks; JPEG q0.93 decision brought to the user with evidence — CLOSED (this line was stale: M6 itself passed at 9a8cc79 with exact agreement, and the user decided (B) keep native frames clean, recorded at 801f85c). The production-integration form of the gate — film telemetry over the complete short — has also now RUN; its verdict is recorded under PRODUCTION INTEGRATION above (FAILED on 7 caption chunks only; overflow none, continuity clean).
```

### 1d. Replacement for the "Production integration: TaleRender wired into the Tall Tale skill…" item

```markdown
- [~] Production integration: TaleRender wired into the Tall Tale skill as an alternative capture backend (frame(t) seam per A6/ADR-001), proven on a complete short end-to-end; final report + honest limits list — DONE IN THE WORK COPY + REPO-SIDE PROOF; SHIPPED-SKILL ADOPTION PENDING THE USER'S DECISION. Work copy ~/workspace/talerender-skill-wiring/ (the shipped skill was never touched by the wave): adapter template/tools/capture_native.mjs (hardened F1/F3/F4/F10; gates on the telemetry verdict LINE; refuses cues/castsheet/preview by name; never falls back silently), bridge packager (21/21 self-verified, assembles <project>/native/ in the adapter's layout), selftest with SKIP/PASS/FAIL/ENV-BLOCKED classes, docs corrected to true state (the earlier "proven end-to-end" overclaim was found by W50 and replaced). The gate's fail-closed direction is proven against the real adapter + real bridge data; the PASS direction is exercised via stub binaries only — no real end-to-end native capture through the skill adapter has run, because this VM holds no current binary (provisioning route is documented; binary + bridge data must come from compatible repo states). The complete-short proof itself was executed on the repo side (CI render → telemetry → finish → MP4, above). Final report: docs/integration/FINAL-REPORT.md. Adoption (opt-in backend in the shipped skill; adoption ≠ making it the default) is the user's decision — see "Open at wave close".
```

### 1e. New section to append after the milestone checklist: "Open at wave close"

```markdown
## Open at wave close (2026-11-14, 100-agent wave complete)

1. **Caption decision (lead/user):** the film telemetry gate's only failures — 7 caption chunks over the 850 px floor (estimate-grade auto-chunks). Re-chunk/split them (data-side fix; caption decision pack prepared in Batch 10, docs/ab/film/) or accept the film with a documented waiver. Until decided, the gate's verdict stands as CHECK FAILED on captions only.
2. **Golden pins (human acts, two separate decisions):** the 14 film mid-frames are staged and hash-verified (docs/golden/frames/ + GOLDEN-STAGING-W87.md) but NOT pinned — a single --pin run would strike the 14 first-pins AND the ab1-composer re-pin (candidate in hand, post-shade-fix 23.96 dB) in one act; they must be decided and committed separately. The s28 packet recommends AGAINST re-pinning s28 (structural difference from the historical prototype pin, 21.711 dB); dof-chart is pending-not-required; stroke-toon has no post-fix candidate yet. Four film mids (560/760/969/1593) sit past the 160-entry paint-cache cap, where re-renders were measured to differ at |Δ|=1 on ~0.1% of pixels (frames 1195–1199, 72.34 dB) — byte-identity across chunk boundaries is proven only up to the cache cap.
3. **RADV proof (user's PC):** every native number in the proof is CI/VM lavapipe. No RADV evidence exists yet: no PC render, no PERF numbers (docs/runbook/PERF.md is a protocol with blank tables by design), and the determinism pack's formal DETERMINISM: PASS is still owed per renderer of record (CI lavapipe AND PC RADV — only stub PASSes exist). Discharge path: docs/runbook/USER-PC.md with the CI-provisioned binary.
4. **Skill adoption (user):** adopt the native backend into the shipped tall-tale-shorts skill as an opt-in alternative, or wait. Work copy is complete and drift-verified (0 files vs the shipped skill beyond the 7 wave additions + review docs); the adoption review's §§3–4 were restored from the seat's report after the file was found truncated (two gap topics among G3/G4/G6 are lost and were not invented). Adoption ≠ defaulting: Chrome remains renderer of record either way until the user says otherwise.
5. **Frames provenance (lead):** proof frames/MP4 are at 381ff47; the lead's 0f6c033 sky-dome + atmosphere fix landed after. Re-render at final head, or accept the 381ff47 proof — the lead's call.
6. **Lead's integration README disposition:** docs/integration/README.md still carries its TBD-PROOF line and the C4/C5/B1 defects; paste-ready replacements are prepared (docs/integration/README-PROOF-PATCH-W88.md) for the lead to apply.
7. **Fog-chart PSNR leg:** the ±2-code analytic gate PASSED on both sides (Chrome ≤1 code, W66; native CI frames re-sampled ≤1 code, W86), but the ≥45 dB PSNR leg vs the Chrome frame remains open — it needs the ×π-amended Chrome re-capture (reference-side task; the Chrome rig's effective light factor is house/π), not a native fix.
```

### 1f. Goal status line — CASE A (close now)

Use only if the orchestrator judges the goal's "whole path" satisfied
by the repo-side proof + work-copy wiring, with §1e carried into
MEMORY.md as the standing remainder.

```markdown
## Status: CLOSED 2026-11-14 — production integration executed and proven on a complete short (repo-side render → telemetry → finish → MP4; skill backend complete in the work copy). The telemetry gate's FAILED-on-captions verdict, the golden pins, RADV proof, and shipped-skill adoption remain open as named items under "Open at wave close" and in MEMORY.md; they are decisions and PC-side proofs, not unfinished build work.
```

### 1g. Goal status line — CASE B (keep open; drafter's recommendation)

The goal's own law is "close the goal only when the whole path is
done", and two elements of the literal path still gate on decisions
that had not landed at wave close: the telemetry verdict's disposition
(caption re-chunk vs waiver) and adoption into the shipped skill. The
evidence supports Case B.

```markdown
## Status: OPEN — build and proof complete; closing on decisions. The 100-agent wave executed the full production-integration path (format v2, film driver, 1,811-frame CI render, telemetry gate run, ffmpeg finish, per-shot A/B, runbooks, skill backend in the work copy). The goal stays open on the named remainder in "Open at wave close": the caption verdict disposition (lead/user), the two golden-pin decisions (human), RADV proof on the user's PC, the frames-provenance call (lead), and shipped-skill adoption (user). Close when those land or are formally waived.
```

---

## DRAFT 2 — MEMORY.md entry

Target: `~/MEMORY.md`, section "## TaleRender — LAUNCHED 2026-11-13",
appended after the existing "USER ORDER 2026-11-14: 100-agent wave…"
bullet (which ends "Wave holds for the user's next instruction after
the final push."). Written in that section's dense bullet style.
`[…]` placeholders are filled by the orchestrator at push time.

```markdown
- **TaleRender 100-agent wave COMPLETE 2026-11-14 — 100/100 seats (10 batches of 10; three Batch-7 seats ended silently and were recovered by replacement/finisher agents — W62, W67, W68 — work verified from disk each time).** Base 34d7882 → wave batch commits 0972e2c, 532dc5f, 311656d, c7db74d, dbf90b9, ce06011, 3781eb2, 743d248, 8af40eb, [Batch 10 commit]; final push HEAD [push HEAD], CI [ci.yml run/verdict], tools-ci [run/verdict]. The production-integration seat ran as lead/integrator with sole ownership of src/ throughout; wave seats wrote new files only, commits centralised.
- **Lead landings during the wave:** §5 divergence register CLOSED in film.rs (381ff47 — D6–D15, D17, D19 from source; full film re-render declared from that commit); loader lane (c74cf0c — 6 registry painters + solid texture arm pasted into scene.rs, exposure wired to the OutputPass uniform, canonical grade keys sat/warm/exposure/time); loader fog wiring (69952ab — v2 fog block parsed, three's fog_fragment applied pre-bokeh via packed-depth prepass; verified 5b0889d, fog chart ≤1 code at every panel); sky dome corrected to the source shader + atmosphere re-dressing (0f6c033); scene-format doc aligned to the sat key (ede0e92). Earlier in the wave: black-trapezoid fix (3e88c60, card transparency was writing opaque black into the HDR target) and the shade root fix (c00897d, linear-space THREE.Color multiply at the single helper; ab1 A/B 23.850 → 23.96 dB measured from the lead's own CI render).
- **Proof — per-shot A/B (docs/ab/film/RESULTS.md):** the full Onoda film rendered natively in CI (10 dispatch runs at 381ff47, artifacts tiling [0,1811) exactly); 60/60 pairs vs Chrome review stills, PSNR min 18.378 / mean ≈21.84 dB (reported, never gated — decision (B) clean frames); s01 anchor 24.094 dB; s06 fog risk refuted; s11 lowest (18.523, worst-3 side-by-sides in the package); s08/s08b/s09 broad-delta cluster referred to the lead (s09 highest mean |Δ| 19.66).
- **Proof — film telemetry verdict: `FILM TELEMETRY: CHECK FAILED (0 overflow groups, 0 continuity violations, 7 caption failures)`.** Independent W85 run (CI binary from run 43575835793, head 5b0889d, hash-verified; TALERENDER_PAINT_CACHE=24) and the lead's own logs agree verbatim. Overflow none, continuity clean; the 7 failures are caption chunks over the 850 px floor (widest 1051 px) — the same 7 Chrome's audit flags; a film-data question (re-chunk vs waiver, lead/user decision), not a port defect. Verdict law reconfirmed: the line is the verdict; the binary exits 0 either way.
- **Proof — MP4 DONE (docs/ab/film/MP4-EVIDENCE.md):** finish.sh verbatim (x264 slow CRF 18, existing mix muxed unchanged) over the 1,811 hash-verified CI frames → ~/workspace/your_files/onoda-native-381ff47.mp4 (29.7 MB, sha256 f357a004…ecaed): counted frames 1811, 60.457 s, −14.00 LUFS vs mix −13.99, faststart OK. Technical proof only, not a channel master (frames are CI lavapipe @381ff47; the lead's 0f6c033 sky fix postdates them — re-render at final head is the lead's call). Finding recorded for the lead: colour tags partial (bt709 space, primaries/transfer unknown) despite finish.sh's flags.
- **Loader lane at close:** 13/13 texture generators armed; fx-textures renders byte-identical local vs CI; fog-chart analytic gate passed both sides (≤1 code; the ≥45 dB PSNR leg vs the Chrome frame still needs the ×π-amended Chrome re-capture — reference-side). Local caveat: this VM's llvmpipe renders the v2+composer path black (v2+direct and v1+composer fine) — environmental, CI frames are the evidence of record.
- **Data fixes by the wave:** saturation→sat rename in 12 scene files (values byte-unchanged; the loader had been silently grading them at the 1.12 default — loader lane only, film unaffected); sea metalness 0.08 restored in s02/s03 from shots.ts:205 (third audit 138/140 → root closed); grade.time zeroed in 7 files; s06 cut corrected (26.129 / 4.261 s). Audits across the wave: 395 values sampled, 387 match, 8 mismatches — every mismatch dispositioned.
- **Tooling fix:** Tools CI had failed at 381ff47 for an environmental reason (validator check 3 greps source literals from the Onoda project tree, absent on runners → 506 false failures). validate_film.py gained a source-absent mode (--no-source / auto-detect): exact-f32 still enforced on all 1,389 numbers, literals reported UNVERIFIED with a COVERAGE: REDUCED line, >6-significant-digit non-round-trippers still FAIL (negative control proven); tools-ci.yml passes the flag explicitly.
- **Determinism, bounded honestly:** chunk-boundary byte-identity proven for the determinism pack's tested ranges and measured byte-identical at production overlap frames 1200–1204, BUT frames 1195–1199 differed across invocations at 72.34 dB (86% of differing pixels at |Δ|=1) past the 160-entry paint-cache cap. The pack's formal DETERMINISM: PASS is still owed per renderer of record (CI lavapipe + PC RADV); only stub PASSes exist.
- **Open at wave close (decisions/proofs, not build work):** caption re-chunk vs waiver (lead/user); golden pins — 14 film mids staged + hash-verified, NOT pinned, and the ab1-composer re-pin is a SEPARATE human act (one --pin would strike both; s28 packet recommends against re-pinning s28); RADV proof on the user's PC (all native numbers so far are lavapipe; PERF.md tables blank by design); shipped-skill adoption (user's call — work copy ~/workspace/talerender-skill-wiring/ complete, drift 0 beyond the wave additions; adoption ≠ default, Chrome stays renderer of record); lead's integration README TBD-PROOF disposition (paste-ready patch in README-PROOF-PATCH-W88.md); frames provenance vs 0f6c033 (lead's call). Final report: docs/integration/FINAL-REPORT.md. **Wave holds for the user's next instruction.**
```

---

## DRAFT 3 — "next instruction" holding note (one paragraph, send at close)

Facts only; `[…]` filled at push time.

```markdown
TaleRender production integration is done and pushed: the 100-agent wave (100/100 seats) plus the lead integrator took the Onoda short end-to-end natively — scene format v2, the full 1,811-frame render in CI, the telemetry gate, ffmpeg finishing, and a per-shot A/B package — and main now stands at [push HEAD] on turjaun985123/talerender with CI [ci.yml: verdict, run id] and Tools CI [verdict, run id]. Headline proof: A/B over the whole film measures 60/60 pairs at mean ≈21.84 dB against the Chrome reference (the §28 anchor shot: 24.09 dB), and the finished proof MP4 is at [MP4 link] (1,811 counted frames, −14.0 LUFS, technical proof from CI-lavapipe frames — not a channel master). Two verdicts need your or the lead's word rather than more building: the telemetry gate reports CHECK FAILED on exactly one thing — 7 caption chunks wider than the 850 px floor, the same 7 Chrome's audit flags — so the choice is re-chunking those captions or accepting them with a waiver; and the golden frames are staged but deliberately unpinned, since pinning is a human act (the 14 film frames and the ab1 re-pin are two separate decisions). Also still open, by design: proof on your PC's RADV GPU (every native number so far is software-lavapipe) and whether to adopt the native backend into the shipped skill (the work copy is ready; Chrome remains the default either way). Full record: docs/integration/FINAL-REPORT.md in the repo. Holding here for your next instruction, as ordered.
```

---

## Drafter's notes for the orchestrator (NOT part of any paste-ready text)

Apply-time confirmations:
- Fill `[Batch 10 commit]`, `[push HEAD]`, and both CI placeholders from
  the actual push; if tools-ci.yml's verdict is not yet final when the
  note is sent, say so rather than implying it.
- Draft 2's seat accounting ("three Batch-7 seats… W62, W67, W68") is
  the conservative form: Batch 7 took 13–14 agent runs for its 10 seats
  (replacements for W62/W67/W68 + the W68 finisher); if a single exact
  run total is wanted, recount from the agent list before applying.
- Draft 1a/2 cite the A/B mean as 21.836 / ≈21.84 dB: W90 recomputed
  21.837 from the stored 3-dp pair values (0.001 rounding artefact in
  the source table). The ≈ form in Drafts 2–3 avoids the contested
  third decimal.
- If W91's FINAL-REPORT completion or W97's wave accounting land before
  application and contradict any figure here, the later verified figure
  wins; these drafts were written from the record available at 8af40eb.
- The caption decision pack is referenced generically ("Batch 10,
  docs/ab/film/") because its final filename was not in the record at
  drafting time; substitute the exact path if it has landed.

States the drafter could NOT determine from the record (see the seat's
completion report): the lead's completion-report status and its
frames-provenance call; Batch 10 sibling outcomes still in flight at
drafting time.
