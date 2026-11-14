# GATE2-W60 — Wave 2, Batch 6 gate (adoption preparation + verification)

**Verdict: PASS-WITH-NOTES.** Eight of nine seats are complete and
every runnable claim in them reproduced first-hand. The one
incomplete deliverable is **W57's note** (`LOCOMOTION-AB-W57.md`),
which had not landed when this gate closed — its census tool did
land and its numbers were re-measured here, so the locomotion
evidence exists, but the A/B protocol / sign-off text the seat owed
does not. That absence is the headline note, not a failure of any
measured claim.

**Gate window / tree state.** The gate opened at HEAD `d3def63`
(Batch 5) and closed at HEAD `b6ee9fb`. HEAD moved four times under
lead integration during the window, none of it touching a Batch-6
seat file:

- `aa92cee` — lead fix for the `edb058f` main-CI failure (see §8);
- `916a109` — GOLDEN ACT 1 (14 film first-pins struck);
- `166873c` — GOLDEN ACT 2 (ab1-composer re-pin);
- `b6ee9fb` — WAVE2-RECORD fold-in (docs-only, verified:
  `git show --stat` = `docs/integration/WAVE2-RECORD.md` alone).

Seat files landed progressively through the window; W51's fixture
directory gained its eleventh file (`bad-guard-stray-cliptime`)
*while the gate's fixture loop was running* — the gate re-ran it
after it appeared (§1). W57's note never appeared.

**Per-seat verdicts:** W51 ACCEPT · W52 ACCEPT · W53 ACCEPT ·
W54 ACCEPT · W55 ACCEPT · W56 ACCEPT · **W57 ACCEPT-WITH-NOTES
(note missing; tool verified)** · W58 ACCEPT · W59 ACCEPT.

Every number below was re-measured by the gate unless explicitly
labelled "(seat's)".

---

## 1. W51 — §15 conformance checker (`check_format15.py`) — ACCEPT

First-hand runs:

| Target | Gate result |
|---|---|
| `valid-composite.scene.json` | **PASS 6/6, exit 0** |
| `note-first-key-ease.scene.json` | PASS, exit 0 (note printed, not a failure) |
| 9 `bad-*.scene.json` fixtures | **all FAIL, exit 1**, each tripping its designed check(s) per the note's coverage table |
| `bad-guard-stray-cliptime.scene.json` (landed mid-gate) | FAIL (0/1 — guard only), exit 1, exactly as the note's table claims |
| Real data: `scenes/*.json` + `scenes/film/*.json` + `scenes/sets/*.json` + `film/onoda.json` (26 files) | **26/26 OVERALL PASS, exit 0** — no production file trips the reservation guard |

**Reservation guard — gate's own plant.** A fresh pre-§15-shaped
file (top-level `prefabs` and `clips` as *lists*, written by the
gate to /tmp) is reported, never silently passed:

```
[1 guard] FAIL — 2 guard violation(s)
    top-level 'prefabs' is list, not an object of definitions — a reserved block in a pre-§15 shape is reported, never silently passed (§15.1)
    top-level 'clips' is list, not an object of definitions — a reserved block in a pre-§15 shape is reported, never silently passed (§15.1)
OVERALL: FAIL (0/3 checks passed)   exit 1
```

The seat's wider sweep (78 JSON files: 58 PASS, 20 non-pass all
accounted as designed-fixture failures) is the seat's figure; the
gate reproduced the production subset (26/26) and the one library
case the seat flags: `scenes/fixtures/prefabs/valid.scene.json`
fails library-less and **passes with its prescribed `--library`
(PASS 2/2)** in the gate's hands too. Exit-code discipline verified
unpiped: valid 0, bad 1, plant 1.

## 2. W52 — s12 bake pack — ACCEPT (65/65 confirmed)

The claim to test was exact: the bake unblocks s12's two `apply`
records, taking the staging census from 63/65 to **65/65** with
`check_clips` **PASS 5/5**. Gate re-runs:

- `stage_clips.py` over `tools/format/staging/s12-baked/s12.scene.json`:
  **PASS 4/4** — the only code-only records reported are the two
  §5.3 blink records; no `apply` failure remains.
- `check_clips.py` over the emitted clips document: **PASS 5/5**.
- The seat's `equivalence_check.py`: **VERDICT: FAITHFUL, exit 0** —
  key-time f32 bit-identity holds (modulo the disclosed sign-of-zero
  on `-bow`), the bake's own contribution (A vs B) is **0 ulp at
  every sample**, and the delivered total stays inside the
  conversion envelope every converted poseTrack pays.

The 65/65 total is therefore by construction: 63 records from
W32's census (gate-verified in Batch 4) + these 2, re-verified
here. The note's "14/14 scene files convert" matches that
arithmetic. The bake's disclosed cost stands: the factors are
frozen into data, regeneration command named in the note.

## 3. W54 — knobs patch pack + `knobs_gate.py` — ACCEPT

`knobs_gate.py --self-test` (against the on-disk frames4 corpus):

```
SELF-TEST: PASS   (8/8 internal checks: identity, planted-difference,
png_size 1080x1920, res arithmetic incl. odd-base identity, tag logic ×3)
```

The tool itself states, and the gate confirms, which legs are
**CI-only**: legs A/B/C drive a real built binary (defaults
byte-identity render, explicit-defaults render, `--draft` smoke) —
none is runnable on this VM (no GPU build), so the byte-identity
law for the knobs remains *specified and gated-in-code* but
*unproven* until the implementation lands and CI runs the gate.
That is the honest state; the pack does not claim otherwise.

Citation spot-checks at HEAD `b6ee9fb` (see §6): exact.

## 4. W55 — tools-ci pack 2 (strictness ratchet + sheet step) — ACCEPT

**Method audit first** (the tools-ci incident's lesson): the seat's
verification used `git archive HEAD` (at `aa92cee`) extracted to a
scratch dir, working tree never used, with exactly one addition —
the pack's own `unknown_fields_baseline.json` — disclosed in the
note. That is the correct method; the gate did **not** have to
redo it for method's sake, and re-ran the core legs anyway:

Gate's own clean-tree run (fresh `git archive HEAD` at `b6ee9fb`,
baseline copied in):

- `fx-clean.scene.json`: **PASS 5/5, exit 0**;
- `fx-unknown.scene.json`: fails, **3 planted unknown keys named**
  (`bogus`, `bogosity`, `shininess`);
- real corpus (27 files): **196 unknown occurrences / 15 distinct
  paths / 7,043 tolerated** — reproduces STRICTNESS-W42 §6 to the
  occurrence, in the committed tree with no working-disk help;
- sheet fixture suite in the clean tree: **PASS 10/10, exit 0**.

The baseline JSON itself: 15 paths, class sums 174 + 19 + 3 =
**196**, matching its own totals fields and the W42 register. The
ratchet semantics (growth fails — both shapes; shrinkage passes
and prints the lowered register; a missing corpus file is exit 2,
not shrinkage) are the seat's transcripts, consistent with the
checker's behaviour the gate observed; the gate did not re-run
the mutation controls.

## 5. W56 / W58 / W59 / W57 — telemetry + adoption evidence

### W56 — VRAM telemetry — ACCEPT

`vram_report.py` over its fixture log (`tools/perf/fixtures/vram/
film-run-tail.log`), gate run, exit 0:

- accounted device: end **220,930,688 B**, **peak 229,286,528 B
  (218.7 MiB)** — the peak is *exactly* the figure W56's design
  note derives arithmetically in its own model table;
- buffer-pool peak 8,355,840 B (the one-frame readback);
- paint-cache peak 2,133,852,160 B, correctly reported as a
  host-side component of RSS, never added on top of it;
- OS counters reported **unavailable** on a lavapipe log — the
  tool refuses to read absence as 0, as designed.

### W58 — blink adoption pack — ACCEPT

`blink_census.py` gate re-run, exit 0. Its in-tool reconciliation
against W33 **passes on every line**:

- cache lookups (character-frames): **3,612** = W33's 3,612;
- today's blink>0 frames: **159** = W33; adopted (smooth+beats):
  **279** = W33; beats law: **25 beat character-frames, 0 changed**
  — the 7 authored word-beat blinks survive `max(channel, beats)`
  bit-exactly, as W33 promised;
- decomposition identity holds (279 = 27 both-on + 252 opened;
  159 = 27 both-on + 132 removed).

New numbers the census adds (gate-measured): **386 changed
character-frames of 3,612 (10.69%)** — 252 opened, 132 removed,
2 reshaped — touching **345 of 1,811 film frames**. Byte-probe
ranges against the changed set: 195–204 has **1** changed frame
(204), 795–814 has **1** (814), **1195–1204 is fully clean** — so
post-adoption byte-identity remains assertable on 38 of the 40
probe frames. The note's figures match the gate's run exactly.
**Not a conflict, but a units warning for the lead:** W33's +219
is *paints* (distinct values), W58's 386 is *frames*; the census
itself states the two are consistent, not interchangeable —
nobody should quote 386 as a paint cost.

### W59 — lipsync evaluation pack — ACCEPT

`evaluate_track.py` gate re-run: **SELF-CHECK: PASS**; the
evaluation reproduces the note's headline verbatim:

- **Regime A** (oscillator bound to narration): **1,196 of 1,616
  in-line frames differ (74.0%)**, band-agreement 26.0%,
  class-agreement 96.7%, mean |Δ| 0.2583, 21 missed closures;
  densest line = line 3 (**79.5%** of its frames differ);
- **Regime B** (the film as staged): the oscillator paints open on
  only **95 of 1,811 frames**, inside its two diegetic windows —
  so adoption changes *who's talking* in at most those windows'
  terms plus the narrator track everywhere else; the regime-B
  film-wide "differing 1,594" figure spans two different speakers
  and is explicitly **not** an adoption metric — the note and the
  tool both say so, and the gate repeats it so the number is
  never quoted as one.

Per-line rows in the note match the gate's run line-for-line
(line 0: 129 differing, mean 0.2716 — identical).

### W57 — locomotion A/B pack — ACCEPT-WITH-NOTES (note missing)

`docs/integration/LOCOMOTION-AB-W57.md` **did not land** in the
gate window. The seat's tool did: `tools/film/locomotion_census.py`,
which the gate ran first-hand (exit 0). Gate-measured census over
the real staging:

- **471 walking frames (26.0% of 1,811)**, 963 advancing
  character-frames;
- **448 frames gain arm-swing motion** (940 character-frames) —
  confirming W24's warning that adoption adds NEW arm motion, it
  does not merely re-time existing motion;
- 888 gait-staged frames (49.0%) see a bob-value change;
- 8 shots untouched (s01, s03, s04, s05a, s05b, s08b, s09, s12);
  walking shots: s02, s06, s07, s08, s10, s11;
- per-site traps preserved in the tool's output (e.g. s10's driver
  window outlasts its segment — `wk` never reaches 1; s11's
  walk-in has swing OFF per the D15 source staging).

These numbers are the **gate's**, measured from the seat's tool —
the seat published no figures of its own to compare against, and
the A/B protocol + sign-off checklist the seat owed (which frames
go to the A/B sheet, in what order the user/lead signs) is simply
absent. The adoption decision for locomotion is therefore the
least-prepared of the three despite its census being complete.

## 6. Citation spot-checks (W53 / W54 / W58) at HEAD `b6ee9fb`

HEAD moved under the seats (surveys ran at `d3def63`/`aa92cee`);
the gate checked citations against `b6ee9fb`. The intermediate
commits touch no `src/` file except `aa92cee`'s film.rs fix, and
no sampled citation drifted:

- **W53:** `film.rs:2056` = the `plates.json` read ✓; `:2059–2061`
  = the `if let Value::Obj` gate + the hardcoded five plates ✓;
  `:2076–2086` = the four calendar years, `calendar-dark.png`,
  `sign.png` ✓.
- **W54:** film.rs `make_target(..., 4, ...)` literal-4 MSAA sites
  ✓; `shadow_misc` value `[1.0 / 2048.0, 2048.0, …]` at
  **film.rs:2838** and **ab1.rs:652** — both exact ✓; the scene
  pass's unconditional `resolve_target: Some(&hdr_view)` ✓ (the
  `--aa 0` branch really is required).
- **W58:** its census numbers are the gate's re-run (§5); its
  charanim/film.rs patch-site citations were sampled and are
  plausible at HEAD, but the gate verified the *numbers* fully
  and the *patch sites* only by sample — the lead should treat
  W58's line numbers as survey-grade, its counts as gate-grade.

W53's per-value transcription (migrated `plates.json` + film-data
additions in `docs/integration/f2-migration/`) was **not**
re-audited value-by-value by this gate; the load-site citations
and the pack's internal consistency check out, and the migration's
own byte-probe proof plan (§9, item 5) is the mechanism that will
catch a transcription slip — it changes zero frames or it fails.

## 7. Hygiene

- **No seat edited a tracked source file.** `git status` shows
  exactly one tracked modification:
  `tools/format/__pycache__/check_clips.cpython-312.pyc` — a
  bytecode artifact that has been tracked since Batch 2
  (`1a62161`) and whose bytes regenerate whenever any tool runs
  (the gate's own runs included). It is not a source edit by any
  seat; the lead may want to untrack it, but that is a separate
  housekeeping call.
- **No `__pycache__` inside any seat fixture path** (find over
  `tools/format/fixtures`, `tools/film/fixtures`,
  `tools/ci/fixtures`, `tools/perf/fixtures`,
  `tools/format/staging`: none).
- **Nothing was committed by any seat** — the window's commits are
  the lead's four (§0) plus the orchestrator's Batch-5 close.
- The untracked set is exactly the Batch-6 deliverables (8 notes,
  5 tools, the s12-baked dir, the f2-migration dir, the format15
  fixtures, the vram fixture, the baseline JSON) plus pre-existing
  `__pycache__` dirs in tool roots.

## 8. CI state at gate close (first-hand, `gh run list`)

- **CI at `b6ee9fb`: SUCCESS** (run 43624139322); a Render
  dispatch at `b6ee9fb` was **in progress** at gate close.
- **Tools CI at `166873c`: SUCCESS**; **Determinism dispatch at
  `166873c`: SUCCESS**.
- **The `edb058f` main-CI failure is FIXED and closed.** Cause
  (from the Batch-5 gate): errors step 2 converted
  `Renderer::new`/`render` signatures without the two `Ok(...)`
  construction wraps — a delimiter/Ok-wrap miss in film.rs.
  The lead's fix `aa92cee` landed during this gate's window;
  every head since (`aa92cee` → `b6ee9fb`) is green on CI, and
  both golden pin acts were struck on top of the fixed tree.
  Nothing from Batch 6 touches `src/`, so no seat work is
  implicated in that failure or its fix.

## 9. Integration packet for the lead (ordered)

1. **Tree health.** Tree is green at `b6ee9fb` (CI + tools-ci +
   determinism). No Batch-6 item blocks on tree health. Optional
   housekeeping: untrack the committed `__pycache__` .pyc (§7).
   **W57's note is owed** — either the seat completes
   `LOCOMOTION-AB-W57.md` or the lead writes the A/B protocol
   from §5's census before locomotion adoption is scheduled.
2. **§15 fold-in.** Fold FORMAT15-DRAFT-W31 into
   `docs/scene-format.md` with W51's checker as the conformance
   gate: 11 fixtures (valid PASS 6/6; 9 planted violations each
   failing at their designed check; the stray-`clipTime` case no
   Batch-2 tool could see), the reservation guard proven against
   the gate's own pre-§15 plant, and a 26/26 production-file
   trivial pass. Wire `check_format15.py` into tools-ci with the
   fold-in commit so the draft and the doc cannot drift apart.
3. **Adoption packs — three decisions, three owners.**
   - **Blink (W58)** — *decision owner: lead, on W33's
     recommendation.* Evidence is complete and gate-verified:
     `max(channel, blink_at(beats))`, beats preserved bit-exactly,
     386 changed character-frames / 345 film frames, probe ranges
     stay assertable on 38/40 frames. Land with the byte-probe
     gate on the clean sub-ranges (195–203, 795–813, all of
     1195–1204).
   - **Lipsync (W59)** — *decision owner: lead; visual sign-off:
     user.* Regime A says the track and the oscillator agree in
     class 96.7% but differ in value on 74.0% of in-line frames —
     adoption is a real look change, concentrated in line 3
     (79.5%). The A/B sheet should be built on line 3 + the two
     regime-B windows before any driver change.
   - **Locomotion (W57)** — *decision owner: user (pixel-changing,
     new arm motion).* Census is gate-measured (471 walking
     frames; arm swing is NEW on 448 frames; bob changes on 888
     gait-staged frames), but the protocol note is missing
     (item 1). Do not schedule adoption until the protocol
     exists; W24's per-site warnings (incl. swing-OFF sites like
     s11's walk-in) bind whichever protocol is written.
4. **s12 / staging close-out (W52).** The bake is proven faithful
   (0-ulp own contribution; check_clips 5/5; census 65/65).
   Adopt the baked records into the staging corpus so the §15
   clips adoption starts from 14/14, and record the regeneration
   command where the corpus lives.
5. **F2 migration (W53).** Land in the pack's order — plates
   manifest as data first, loader second, hardcoded paths last —
   with the pack's byte-probe proof (a correct migration changes
   zero frames). This closes the generic-film blocker scoped at
   triage; the two silent fallbacks die with named errors.
6. **Knobs implementation (W54).** Implement per the patch pack
   (resolve-branch in both scene passes; shadow uniform lockstep;
   captions scale with the frame; DRAFT tagging), then run
   `knobs_gate.py` legs A/B/C in CI — the defaults byte-identity
   law is unproven until that run exists.
7. **Tools-ci pack 2 (W55).** Wire the strictness ratchet +
   sheet steps from TOOLSCI2-W55 verbatim. Gate-verified in a
   clean archive tree: corpus reproduces 196/15/7,043, fixtures
   behave, sheet suite 10/10. Baseline lowers only in the PR
   that fixes the data.
8. **VRAM implementation (W56).** Implement the `gpu-mem:` line
   per the design note (ledger watermarks + sysfs where present,
   `unavailable` never 0). The report tool and its fixture are
   the acceptance test: peak accounted 229,286,528 B on the
   fixture log, reproduced by this gate.

**Conflicts between seats:** none that survive inspection. The
three apparent ones are units/framing, each resolved inside the
deliverables themselves — W33 paints vs W58 frames (§5), W59's
regime-B film-wide figure vs its own not-a-metric warning (§5),
and W51's colour-lane check enforcing §15.9's files-author-hex
rule exactly as W31's reconciliation wrote it. The one genuine
open seam is procedural, not technical: W57's missing note
leaves the locomotion adoption — the only one of the three that
adds new motion — with evidence but no protocol.
