# TaleRender Wave 2 — Close-Out Report

> **STATUS: DRAFT.** Written by seat W92 (Batch 10, the final batch)
> at HEAD `9380b01` on 2026-11-15. This draft becomes final only
> when seat W100's closing gate has verified it. Every claim below
> was checked against the repository, the wave record
> (`docs/integration/WAVE2-RECORD.md`), the batch gates, or a live
> CI query at writing time — where a check could not be made, the
> text says so instead of guessing.

---

## 1. What was ordered, and how the work was organised

**The order (user, 2026-11-14 22:26 +06):** assign 100 agents to
complete **all phases** of an external audit of TaleRender — first
write all the code across all phases, then do the fixes. A second
order the same night (23:24 +06) added a gap-fix list (items F1–F6)
found by testing the released v1.0.0 program on the user's own PC.

**The audit's scope** (an outside review of the project, pasted by
the user) had five phases plus a set of findings:

- **Phase 1 — Production correctness:** fix the seven captions that
  were too wide for the caption bar; give the film-quality gate
  proper exit codes; re-pin the golden reference frames; write the
  determinism contract (the promise that the same input always
  produces the same frames).
- **Phase 2 — Rendering architecture:** end the five-way copy-paste
  of GPU setup code (audit finding B); split the two giant source
  files; replace crash-on-bad-input (`panic!`) with structured
  error messages and exit codes (finding D).
- **Phase 3 — Performance engineering:** account the picture cache
  in bytes, not just entry counts; progress lines with elapsed
  time; a peak-memory self-report; a benchmark harness ready for
  the user's PC.
- **Phase 4 — Animation & visual quality (all opt-in):** a proper
  animation evaluator with channels for pose, expression, gaze and
  blink; camera tracks as a format extension.
- **Phase 5 — Engine maturity:** versioned scene-format extensions
  (prefabs, animation clips, camera tracks); visual-regression and
  determinism checks in CI; documentation brought back in line
  with the code.
- **Findings A and E** doubted whether the renderer's determinism
  problem was really root-caused. **Finding C** was the captions.

**One caution applied from the start:** the audit was written
against an older snapshot of the project, so some of its items
were already fixed before the wave began (the BT.709 colour
metadata, the full-film re-render, the determinism root cause, the
command-line help). Those were verified and **not redone**; the
register of already-done items is §3 of the wave record.

**Method:** 100 agents in 10 batches of 10. Agents wrote new files
only and never committed; one lead engineer owned all edits to
existing source files and integrated the agents' work; commits
were made centrally between batches with explicit file lists.
Nothing was built on this machine — every build and test ran in
GitHub Actions CI, and every CI verdict was reported as it was
checked. A mid-wave user order set the sequence: all code first,
then the fixes pass.

---

## 2. What was delivered — with the proof for each item

### Phase 1 — Production correctness

- **The caption problem is resolved, and the quality gate is
  green.** The seven too-wide captions were re-split from 66
  chunks into 73 narrower ones, and both sides of the data (the
  scene-side strips and the film driver data) were regenerated to
  match. The film telemetry gate — the automated check that
  measures every caption against the 850 px bar — now prints
  **`FILM TELEMETRY: CHECK OK` and exits 0** (commit `5e90d7e`;
  wave record §5, "closed by data"). The width threshold itself
  was not lowered. *You can still veto this at close — see §5.*
- **Telemetry exit codes.** The film gate now exits 0 on pass, 1
  on check-failed, 3 when it could not run at all (commit
  `aa10b13`). The printed verdict line is unchanged and remains
  the authoritative signal; the skill's adapter was re-proven
  against both conventions (see the corrections register for the
  full story of that proof).
- **Golden reference frames re-pinned — in two separate, deliberate
  acts.** Act 1 (`916a109`) pinned the 14 film frames; Act 2
  (`166873c`) re-pinned the composer reference frame to a fresh
  render measuring **23.98 dB** against the Chrome original.
  Verification after both acts: **18 OK / 0 stale / 0 missing**.
  Act 1 initially contained a real defect — it is told in full in
  §3, because the way it was caught is part of this wave's record.
- **Determinism contract + CI proof.** The contract document
  (Batch 1, seat W04) states what bit-identical means per
  renderer. Its CI leg passed: determinism dispatch run
  `43617370962`, **SUCCESS**. The same-renderer proof on the
  user's own graphics hardware (RADV) is still owed — see §4.

### Phase 2 — Rendering architecture

- **Finding B closed: the GPU code duplication is gone.** All five
  call sites (main, ab1, dof, scene-file, film) now share one GPU
  core module; the last duplicate helper was deleted in commit
  `aadc193`. Each migration step was gated by byte-identical or
  PSNR spot-proofs (the depth-of-field chart and the texture scene
  came back byte-identical; the composer frame matched its
  reference).
- **Structured errors, both steps.** Step 1 (`ba88596`) converted
  the film/scene loading and image-saving paths; step 2
  (`edb058f`, repaired in `aa92cee`) converted the GPU paths, so a
  missing graphics driver now produces a readable error and
  **exit 3** instead of a crash — proven with the CI binary and
  the driver deliberately hidden.
- **The two giant-file splits are prepared, not executed.** The
  split programmes for `film.rs` and `scene.rs` were written
  (Batch 3), audited for readiness (Batch 7), and amended in
  Batch 9 (`SPLIT-AMEND-W82.md`): 22 pure-move commits, in order,
  with the frozen `state_key` function verified byte-identical
  (md5 `95918b99…`) at the current head. One precondition is
  honestly open: the 46/46 stroke-battery check has no standing
  CI invocation, and the amendment pack blocks steps S2/S4/S5
  until the lead establishes one. Executing the splits is
  post-wave work — see §4.

### Phase 3 — Performance engineering

- **Paint-cache byte budget + run log, wired in** (`86f0c81`):
  the cache now accounts bytes against a budget, and film runs
  print a completion summary with peak memory. Forced-eviction
  byte-probes passed 30/30 identical — shrinking the cache does
  not change a single pixel.
- **Benchmark harness** (Batch 2) ran its first CI dispatch
  successfully (CI's software renderer: 1.85 s/frame in the
  measured window; a 24-entry cache saved ~147 MB of memory at no
  time cost). Two anomalies were recorded, not hidden: the bench
  window covers only the first shot, and the cache's accounted
  bytes diverge ~9× from resident memory. Real performance
  numbers still require the user's PC — none are claimed here.
- **VRAM accounting is staged, not landed.** The Batch 9 staging
  pack (`VRAM-IMPL-W84.md`) re-derived the film's graphics-memory
  formula from the code and reproduced the design figures
  **exactly: 220,930,688 bytes steady / 229,286,528 peak**. The
  implementation itself awaits its landing window (§4).

### Phase 4 — Animation & visual quality

- **Animation evaluator + character channels landed as modules**
  (Batch 2: `anim`, `charanim`, `rendergraph`, `atlas`,
  `bufferpool` — registered in `2328e6f`, adding 94 proven tests
  to the suite), with adoption decisions recorded by the lead
  (colour/step lane, ease-direction bridge, atlas on probation).
- **Blink improvement adopted into the film** (`5a9383e`): smooth
  scheduled blinks now combine with the seven hand-authored
  beat-blinks, which survive exactly — the gate measured changes
  on precisely the promised frames (345 of 1,811 frames change;
  the 25 authored beat frames are untouched).
- **Locomotion and lip-sync are decision packs, not landings —
  by design.** Both change how characters move and speak on
  screen, so both were prepared to the point of a user decision
  and no further: the locomotion pack (Batch 9, W87) includes a
  plain-language decision sheet; the lip-sync pack (W86)
  recommends **Variant A** (step keys written by the track
  builder) after measuring it equal to the alternative on every
  leak class. See §5.

### Phase 5 — Engine maturity

- **Scene-format §15 folded in, with teeth** (`4dd6530`): the
  versioned-extensions chapter (prefabs, clips, camera tracks) is
  now part of `docs/scene-format.md`, and the loader carries a
  *reservation guard* — data using the reserved extensions fails
  loudly with a named message instead of being silently ignored.
  An independent census found **75/75 clauses present, none
  weakened**.
- **CI grew up:** visual-regression and determinism workflows,
  a tools-CI suite running the format validators, the asset
  checker, the shader checker, the sheet tests, and a strictness
  ratchet (below) on every push.
- **Documentation corrected wholesale** (Batch 8/9 landings:
  `d967931`, `108f33f`, `33c5b2a`, `c7aa648`): the micro-fix pack,
  the visual-regression register update, the §4 splice, and the
  PC-runbook paste-ins all landed.

### The gap-fix order (F1–F6)

- **F1 — landed** (`aadc193`): the film driver now reads the
  generic `film.json`, falling back to `onoda.json`; `--help`
  agrees with the loader.
- **F2 — landed, complete** (`a2ec80c`, `20a81a7`, `2c41313`):
  the film's data now declares its own calendars, sign and plates
  (as a checked manifest) instead of the driver assuming one
  specific film's assets. Verified **zero frame impact**.
- **F3 — the silent-ignore problem, mostly closed.** A checker
  now counts fields in the data that no code reads, and CI fails
  if the count grows. The dead-provenance class was stripped in
  one atomic commit (`2fcffc0`): **196 → 24** known-unknown
  fields. The remaining classes are dispositioned and staged
  (Batch 9, W88): all 19 prose notes cited and marked for
  strip-with-citation; the bookshop `materials` residue marked
  strip-via-decomposition; `lights.key.near` recommended
  **consume** (teach the loader the field). The staged baseline
  is **3** — that landing is still to execute (§4).
- **F4 — quality knobs, landed and independently accepted.**
  `--res`, `--aa`, `--shadow` and `--draft` now exist, with the
  laws kept: defaults are byte-identical to before, and any
  non-default render is tagged `[DRAFT … — NOT FOR DELIVERY]` in
  its output so a draft can never pass as a final. The acceptance
  gate ran **twice independently at the same head (`c440d90`),
  both `KNOBS GATE: PASS`** (runs `43628289244`, `43628339967`,
  verified from CI at writing): default renders 40/40
  byte-identical with no draft tag; the draft leg produced the
  exact tag at 540×960; all ten invalid-value cases exited 2 with
  proper errors. *This item also carries the wave's worst error —
  see §3, first entry.*
- **F5 — landed:** a native contact-sheet tool for reviewing
  frames (Batch 5, W44; tests 10/10; wired into tools-CI).
- **F6 — design only, as scoped:** the cues-export shape is
  designed and marked PROPOSED; it stays on the skill side, with
  one question (the exact cue entry shape) still open for you.

### Also delivered

- **Telemetry tables in the binary** (`3223509`): `--tables`
  prints the shot/caption/cast tables alongside the gate verdict;
  the prototype had matched the binary's own caption measurements
  exactly (73/73). *One of its unit tests is the current CI
  failure — see §6.*
- **s12 bake: proven, adoption-ready.** One shot's data used a
  transform form the format checker rejects. The bake (converting
  it to plain keys) was proven faithful in Batch 6 (65/65 records
  convert) and re-verified in Batch 9 (`S12-ADOPT-W83.md`):
  **READY, zero drift** — the adoption is a small, prescribed
  commit (retire three lines in `film.rs`, re-probe frames
  1666–1810) that has not been executed yet (§4).
- **v1.0.1 release pack, refreshed** (`RELEASE-101B-W89.md`):
  changelog extended to the current head, bump mechanics
  re-verified, six gates to re-prove on the release commit. Its
  honest summary is **no-go yet** — not because anything is
  broken, but because the release waits on your go, the wave
  freeze, and those gates. See §5.

---

## 3. The corrections register — this wave's errors, told straight

A wave that only reports successes is not reporting. These are
the errors this wave made, in the order of seriousness, each with
how it was caught and what fixed it.

1. **The knobs phase-1 false claim (the worst error).** During
   Batch 7, the wave record — and a report relayed to you — stated
   that knobs phase 1 (`--res` + the DRAFT tag) had landed. **It
   had not.** Batch 8's seat W73 proved the absence airtight at
   `b2e46d2`: no `Knobs` struct existed, all four flags were
   rejected as unknown arguments, and the 540×960 figure attached
   to the claim could not have come through this repo's render
   workflow (it has no size input). The lead corrected the record
   itself (`4af54c5`), you were told plainly, and only then was
   the feature implemented for real (phases 1–3:
   `ea98458`/`9ad2177`/`76b922c`) and independently accepted
   (the two PASS runs in §2). *Lesson now standing: no landing is
   reported from a record line or a commit message — only from
   code or run output.*
2. **Golden Act 1 pinned the wrong bytes.** The first pin act
   (`916a109`) described, for 11 of the 14 film frames, superseded
   pre-atmosphere-fix frames — a staging copy step was missed; the
   renderer itself was innocent. The visual-regression canary
   caught it on its first measured run after pinning. The lead's
   corrective (`2755ff3`) was verified exactly by an independent
   seat (W63), and the post-corrective regression run is the
   verified steady state: everything matches except the known
   s28 item (§5).
3. **The adapter re-proof that wasn't.** The lead originally
   recorded a "real-binary proof" that the skill adapter handles
   the gate's failure direction. That run had in fact been killed
   mid-check under machine contention, and its test fixture mixed
   data from two caption generations. The record row was
   corrected in place, and Batch 7's seat W62 re-ran the proof
   independently: **8/8 cells pass** across both exit-code
   generations. The correction is closed by an outside check, not
   by the original claimant.
4. **A wrong commit citation.** The Act-2 pin's commit message
   named the wrong source run's head (`9f497c7` instead of
   `dc0e555`). Commit messages cannot be edited; the correction
   of record lives in the wave record (`cca4e6e`).
5. **The F3 arithmetic correction.** The strip target was first
   computed as 196 → 22. Batch 8's W74 found that two of the
   fields (`fps`, `frames`) have real readers in the Python
   tooling — stripping them would have broken tools. They were
   re-classed as kept, and the strip landed at **196 → 24**.
6. **Two documents were truncated in transit.** The knobs spec
   (W43) was committed cut off at line 264, and the §15 fold-in
   note (W65) lost its §4 the same way. Both were restored with
   provenance tags (W61 tail, spliced in `33c5b2a`; W71 addendum,
   spliced in `108f33f`); six items from the knobs spec are
   **unrecoverable** and are registered as such, not reconstructed
   from memory.
7. **Three tools-CI failures, one family.** Each was an
   environment assumption that local runs could not see: absolute
   machine paths in the film manifest (fixed `617ea4e`), fixture
   images silently swallowed by a `.gitignore` rule since Batch 3
   (fixed `865c471`), and ffmpeg missing from the hosted image
   (fixed `f4f6ccf`). The standing fix is procedural: wiring is
   now verified from a clean tree (`git archive`), not from a
   developer checkout.
8. **Two build breaks, both fixed the same round.** The errors
   step-2 commit missed two `Ok(...)` wraps and failed CI at
   Build (fixed `aa92cee`; CI + Tools CI then both SUCCESS); the
   knobs phase-2 commit left a dangling delimiter in the composer
   path that failed four probe dispatches at Build (fixed
   `c440d90`). Neither reached a release or a pin.
9. **The record itself was incomplete mid-wave.** Batch 8's audit
   (W79) found only 27 of 37 commits present in the record, with
   8 stale items and only 1 of 8 corrections findable. The lead's
   second fold-in (`cca4e6e`) closed most of it inside the gate
   window; the final audit is a Batch-10 item (§4).

---

## 4. Not delivered / still open — honestly

- **Staged, not landed** (each is a prepared, gated landing
  awaiting its execution window — none is blocked on unknowns):
  the film/scene file splits (22 commits; one precondition open,
  §2); the VRAM memory line (formula proven, code staged); the
  s12 bake adoption (READY, prescribed); the F3-b/c landing
  (staged baseline 24 → 3); locomotion registration step 0
  (one-line registration + zero-pixel proof protocol, pack W87).
- **Awaiting your decision, not engineering:** the lip-sync
  binding (Variant A recommended) and locomotion adoption —
  both change on-screen motion, so both stop at a decision pack.
  All six pending decisions are listed in §5.
- **The PC RADV session has not happened.** Every number in this
  wave comes from CI's software renderer (lavapipe). There are
  still **no performance numbers from your PC**, the determinism
  contract's RADV leg is unproven, and the VRAM numeric path is
  untested on real hardware. The rehearsal of that session
  (Batch 7's seat W68, dry-running the runbook on this machine)
  landed late — after the Batch 9 gate closed; its note
  (`PC-DRYRUN-W68.md`, uncommitted at writing) rehearsed the
  telemetry gate (CHECK OK, exit 0) and the golden pack (18 OK)
  successfully, and found **one blocking defect in the runbook
  itself**: §0.2 provisions the binary to `~/talerender-bin`, but
  later sections invoke `./talerender-bin/talerender` from the
  repo root, which fails (exit 127, reproduced). The fix pack is
  a Batch-10 item (seat W96).
- **W77's exporter note is still owed.** The acceptance harness
  exists and its self-test passed 3/3 at the Batch 8 gate; the
  explanatory note has not landed, and the tool file shows an
  uncommitted modification at writing — the seat appears still to
  be iterating. The harness's claims about a real binary remain
  ungated until the note lands.
- **The current head is CI-red on one unit test** — see §6. The
  wave does not close green until that is fixed and re-proven.
- **The goal stays open.** Goal `goal_b21124e71eb3` (TaleRender
  as the native renderer) remains OPEN after this wave. What it
  still owes: the PC RADV evidence session (above), the
  fog-chart PSNR leg (which needs a corrected Chrome re-capture
  on the reference side), and the skill-adoption decision (§5).
  Wave 2 closes the audit's programme; it does not close the
  goal.

---

## 5. Decisions that are yours

Six decisions are assembled, with evidence and recommendations,
in the companion Batch-10 brief (**seat W94's decisions brief**).
They are named here only, so this report stays the record and the
brief stays the decision document:

1. **Caption veto** — keep the 73-chunk re-chunk (recommended) or
   revert to the 66-chunk data and the Wave-1 waiver.
2. **s28 golden item** — re-pin, retire, or leave as the one
   designed red in visual regression.
3. **Locomotion D1–D4** — whether and how walking motion changes
   (the pack includes a plain-language sheet with the on-screen
   differences).
4. **Lip-sync binding** — Variant A (recommended) or not yet.
5. **v1.0.1 patch release** — go or wait.
6. **Skill adoption** — adopt the native renderer as an opt-in
   alternative in the shipped skill, or wait for RADV proof.

---

## 6. Final numbers

- **Seats:** 100 of 100 across 10 batches. Batch gates: Batch 1
  PASS, Batch 2 PASS, Batches 3–9 PASS-WITH-NOTES (Batch 9 gate:
  `GATE2-W90.md`). Batch 10's gate (seat W100) is what turns this
  draft final.
- **Commits:** 144 on `main` in total; **54 since the v1.0.0
  release**, of which **53 are Wave 2** (from the wave-record
  commit `6f00400` through HEAD `9380b01`, inclusive). HEAD and
  `origin/main` agree at `9380b01`.
- **CI conclusions at close** (checked live at writing; the
  Batch-10 final sweep — seat W99 — had **not** landed as a
  document at writing, so these are the heads checked here):
  - HEAD `9380b01`: **CI FAILURE** (run `43629817118`) — the
    suite ran 179 passed / 1 failed; the failure is a single unit
    test in the new telemetry-tables code,
    `tables_beat_frames_tie_to_blink_at` (assertion at
    `src/telemetry.rs:2062`, expected 3 beat frames, got 0). The
    build itself passed. This belongs to the G7 tables landing
    (`3223509`), whose own CI run was cancelled by concurrency
    before it could report — this Batch-9 run is the first CI
    verdict on that code, and it is red. **Fixing it is a
    precondition for calling the wave closed-green.**
  - Tools CI: SUCCESS at `3223509` (run `43629625142`). The last
    head with **both** CI and Tools CI SUCCESS is `2fcffc0`.
  - Knobs gate: two dispatches, both SUCCESS, at `c440d90`
    (runs `43628289244`, `43628339967`) — the F4 acceptance.
  - Determinism (CI lavapipe leg): SUCCESS (run `43617370962`).
  - Visual regression at `3c07564` (run `43628892930`): FAILURE
    **as designed** — the sole drift is the s28 item (§5,
    decision 2); everything else matches.
  - Several runs in the list show **cancelled** — those are
    concurrency supersessions (a newer push cancelling an older
    run), not failures, and are not counted as red.
- **Golden pack:** 18 OK / 0 STALE / 0 MISSING.
- **Film gate:** `FILM TELEMETRY: CHECK OK`, exit 0, on the
  73-chunk data.
- **Strictness ratchet:** 24 known-unknown fields baselined
  (from 196), with 3 staged.

---

*End of draft. Companion documents: the wave record
(`WAVE2-RECORD.md`), the batch gates (`GATE2-W10` … `GATE2-W90`),
the decisions brief (seat W94), and the per-item packs cited
inline throughout.*
