# GATE2-W90 — Wave 2, Batch 9 gate

**Gate seat:** W90. **Gate window HEADs:** seats surveyed
`021ca01`–`c440d90`; the tree advanced during the gate itself
(lead fixes-pass landings `d967931`, `3c07564`, `2fcffc0`,
`108f33f`, `3223509`). Every figure below is labelled with the
head or run it was measured on. First-hand evidence is this
gate's own runs/reads; anything not re-measured is labelled
"the seat's".

## Verdict: PASS-WITH-NOTES

**Per-seat:** W81 ACCEPT · W82 ACCEPT · W83 ACCEPT · W84 ACCEPT ·
W85 ACCEPT · W86 ACCEPT · W87 ACCEPT · W88 ACCEPT · W89 ACCEPT ·
**W68 (Batch 7) NOT GATED — file still absent, third consecutive
batch close** · **W77 (Batch 8) note NOT GATED — still absent**
(tool + fixtures were gated in Batch 8; see §9).

---

## 1. W81 — knobs gate acceptance: ACCEPT (first-hand)

This gate independently read the dispatch logs (`gh run view
--log`), not W81's summary:

- **Two dispatches at `c440d90`, both completed/success:**
  `43628289244` (the lead's) and `43628339967` (W81's own,
  disclosed in its note). Neither cancelled — a real verdict
  exists, twice.
- **Exact verdict line** in the run log: `KNOBS GATE: PASS`,
  and the workflow's Verdict step greps for exactly that line
  (`grep -qx`) and passed.
- **Leg A** (from the log): byte probes 195–204 **10/10**,
  795–814 **20/20**, 1195–1204 **10/10** identical →
  `BYTE PROBES: PASS (40/40…)` → `leg A: PASS (defaults are
  byte-identical, stdout clean)`. Matches W81's account exactly.
- **Leg B:** progress line carries the DRAFT tag; probes differ
  0/10 as designed (a draft that matched would be the failure);
  `1080x1920 -> 540x960`; tag in header, progress, completion →
  `leg B: PASS`. Matches.
- **Leg C:** `leg C: PASS` in the log; the ten refusal cases
  (exit 2 + `error:` + no `unknown argument`) are W81's
  account from the results file — consistent with the log,
  not separately re-read by this gate.

**F4 is ACCEPTED** on this evidence: phases 1–3 pass their
designed gate on a real dispatch at a named head, twice.
W81's two thinness notes stand and are carried, not blocking:
leg B/C evidence is assertion-level (the tagged lines and
refusal texts are checked by the tool, not echoed into the
artefacts), and intermediate solo values (`--aa 2`,
`--shadow 1024`) are exercised nowhere — they belong to the
PC session (§11).

## 2. W82 — split amendment pack: ACCEPT (first-hand)

- **state_key re-extracted by this gate** from the live tree:
  `src/film.rs:2246`, 25 lines (2246–2270), md5
  **`95918b9941313f895dbee7bf35c8d14`** — byte-identical to
  W66's, W70's, and W82's recorded hash. The determinism
  freeze holds through gpucore, errors 2, F2, blink, and all
  three knobs phases.
- **G-TEL census verified complete by this gate's own greps**
  of both blueprints: in FILM-SPLIT-W21.md the old verdict
  string occurs at line 355 (the definition — W82's A1) and
  descriptively at 164 (A5); name-only G-TEL cites at 380,
  381, 383, 386 (steps 1/2/4/7 — A3), 388 (step 9 negative —
  A4), 389 (step 10, "line verbatim + exit 1" — A2), 444–446
  (§6 — A3), 484 (trap T4 — A3). Every occurrence is in
  W82's census with a disposition. In SCENE-SPLIT-W22.md the
  only verdict-line gate is line 343's "film telemetry
  verdict line unchanged" (A6) — confirmed by grep; the
  step-4 M6 spot check (A7) is a different contract and W82
  correctly leaves it alone.
- The pack's remaining content (adapter-paragraph strike,
  calendar_state supersession, cite refresh, 22-commit
  protocol, the S2/S4/S5 battery precondition, step-13
  staging-dir decision) is the seat's work, read and
  internally consistent; the battery precondition is real —
  ci.yml at the surveyed heads runs build + test only, and
  the battery is a binary scene mode, not a workflow mode.

## 3. W83 — s12 adoption readiness: ACCEPT (first-hand)

Re-run by this gate from a scratch mirror (the bake script
writes beside itself):

- `bake_s12.py` against the current live
  `scenes/film/s12.scene.json`: **drift guard PASS, exit 0**,
  output identical to W83's quoted output (lean ×0.6 → 2.1,
  f32 arrival 2.0999999046325684; headTilt ×−1.0 → −3.5), and
  the regenerated file is **byte-identical** (`cmp` clean) to
  the committed staging file.
- One conversion check: `stage_clips.py` on the baked copy →
  **OVERALL: PASS (4/4)**; `check_clips.py` on its output →
  **OVERALL: PASS (5/5)**. Exactly W83's recorded verdicts.

The adoption shape (baked data + film.rs:2021–2023 retirement
in one commit; byte probes `--ranges 1666-1810` with the move
window 1727–1741, since no standing probe set covers s12) is
the seat's, adopted into the Batch-10 packet (§11).

## 4. W84 — VRAM implementation staging: ACCEPT (first-hand)

This gate re-derived the film-lane figure **from the format
universe + dimensions, mechanically** (not from either doc):

```
px = 1080×1920 = 2,073,600
A = px·(8·4 + 4·4 + 8 + 4 + 4 + 8 + 4 + 4)          = 165,888,000
B = 2 × (2048×2048×4)                                =  33,554,432
C = 8 × (518,400 + 2×690,600)   [bloom mips Σ=690,600] = 15,196,800
D = 768×1024×8                                       =   6,291,456
steady = A+B+C+D                                     = 220,930,688 ✓
peak   = steady + 4352×1920 (padded readback)        = 229,286,528 ✓
```

Three-way agreement: W56's formula, W84's re-derivation from
today's post-knobs code (per-site census, closed form
`A = W·H·(12s + p + 28)`), and this gate's computation all
land on **220,930,688 B steady / 229,286,528 B peak** at
defaults. W84's two design rulings are flagged for the lead,
not settled by this gate: D1 (telemetry prints no gpu-mem
line — the mode is CPU-only, a zero line would be a
fabricated measurement) and D2 (bloom's per-call chain is
counted as a standing working set by convention, or hoisted).

## 5. W85 — G7 tables implementation spec: ACCEPT

Read in full. The spec's cites are snapshot-honest (verified
at `021ca01`) and its decisions D1–D7 are argued from the
code (printer beside `print_caption_audit` in telemetry.rs;
cast presence from the audit's own sampled presence, not the
prototype's shot-granularity counts; the D3 attribution-tiling
delta vs the manifest disclosed with a worked example).
During this gate window the lead landed the implementation
(`3223509`, "G7 tables in the binary (TABLES-IMPL-W85)").
**This gate did not verify that implementation** — no duty
covered it and the binary's `--tables` output has not been
diffed against the prototype here; that check belongs to
Batch 10's close verification (§11).

## 6. W86 — lipsync binding decision pack: ACCEPT

Read in full. Recommendation: **Variant A** (builder-side
closed-exit step keys in `build_visemes.py`; threshold 0.05
named from the painter's open threshold), with a correction
to W59 §5 item 2(a) (the rule must be closed-exit, not
silence-exit, or the `pp` closures stay open). Its §2
measurements are **the seat's**, produced by a frame loop
that first reproduced every W59-published figure exactly
(1,196 band-differing; 229/262; 119/195; 6/36; step what-if
106/0/36-of-36 — those anchors are the wave's existing
record); the new rows (Variant A matching full-step's
painted classes 106/0/36-of-36 with 77 of 551 ease fields
changed; A2 priced at +97 keys and 51 one-frame mouth-slams;
the global-step blast radius — 95 of 145 production tracks
changed, every camera track staircased) are labelled the
seat's here. The §5 sign-off sheet (line 3, frames 482–632,
three bindings, the 17-frame silence at 570–586 as the
headline judgement) is decision-ready for the user (§11).

## 7. W87 — locomotion registration step 0: ACCEPT

Read. The pack's core claims are consistent with two
independent censuses this gate holds: W89's mod-list census
(main.rs declares `anim` :19 and `charanim` :23, no
`mod locomotion;` / `mod cameramoves;`) matches W87's
premise that locomotion is unregistered at `c440d90`. The
zero-pixel argument (zero `locomotion::` call sites; the
patch is one mod-list line; +9 tests join `cargo test`) is
the seat's grep work, labelled as such. Part 3's plain-
language decision sheet restates W57's D1–D4 for the user
(§11) — including the disclosed visible costs (arm swing is
NEW motion on 448 frames; standing characters re-level
~1.9 px in s10 for 4.4 s) rather than smoothing them.

## 8. W88 — F3-b/F3-c dispositions: ACCEPT (first-hand)

This gate re-ran the STRICTNESS-W42 corpus command
(27 files) in scratch trees:

| state | unknown | paths | tolerated |
|---|---|---|---|
| live tree as-is at gate time | **24** | 9 | 7,043 |
| + W88's staged files (s10/s11/s12/bookshop) | **3** | 3 | **7,047** |

W88's claimed end state reproduces **exactly**: 3 unknown
occurrences in 3 paths, 7,047 tolerated (the +4 tolerated is
the decomposition's added node `name` keys, as W88 accounts).
The three survivors, verbatim from this gate's own run:
`scenes/sets/bookshop.scene.json $.lights.key.near`;
`film/onoda.json $.fps`; `film/onoda.json $.frames` — exactly
W88's named survivors.

**Method note (a labelling correction, not a seat error):**
W88's state-1 control reads 196/15 — that was the corpus
*before* the lead landed W74's F3-a strip (`2fcffc0`, during
this gate window the live tree already sits at 24/9, which
this gate's control run confirms). W88's states 2–3 and its
arithmetic (24 → 3 for its own classes) are unaffected.
Dispositions: all 19 F3-b items CITE (strip-with-citation,
each citation named to ASSUMPTIONS.md / the sets README /
painter headers — spot-read by this gate in B-1/B-4/B-17 and
found specific, not boilerplate); F3-c `materials` → STRIP
via the staged s10-form decomposition; F3-c `lights.key.near`
→ **CONSUME** (one parser line; the value is already in force
as `rig_bk`'s hardcoded `key_near: 0.5`). Both either/ors are
argued with the losing option's cost stated.

## 9. W89 — v1.0.1 pack refresh: ACCEPT (spot-checked)

- `Cargo.toml:3` = `version = "1.0.0"` ✓ — the one functional
  bump, still.
- Literal census re-counted by this gate: `git grep '1\.0\.0'`
  outside the two release packs = **29 lines = 1 bump + 1
  decision + 27 keep**, exactly W89's §3.3/§8 arithmetic.
- Spot-checked keep-literals verbatim: `README.md:65`
  ("`talerender 1.0.0`" — the one *decision* line, recommend
  updating inside the bump commit), `docs/runbook/PERF.md:26`
  ("added after v1.0.0" — historical, keep),
  `docs/runbook/PC-EVIDENCE-W36.md:90` ("sanity: 1.0.0 or
  later" — minimum check, keep). All as W89 classifies them.
- One cite drift, caused by lead landings after W89's
  snapshot: the `--version` print (`env!("CARGO_PKG_VERSION")`)
  sits at `src/main.rs:280` at gate-close HEAD, not :274.
  The executor re-derives cites at the freeze anyway (the
  pack's own Step 0/2 discipline); no claim is affected.
- The pack's go/no-go (§5/§7) is honest: **no-go yet** —
  user go outstanding, wave not frozen, bump not made, all
  six Step-3 gates to be re-proven on the bump SHA.

## 10. CI state + hygiene at gate close

**Runs of record (gh, read by this gate):**

| workflow | head | run | conclusion |
|---|---|---|---|
| Knobs gate (dispatch) ×2 | `c440d90` | 43628289244, 43628339967 | **success / success** — `KNOBS GATE: PASS` (§1) |
| CI | `9ad2177` | 4362797885 | **failure** — the ab1 delimiter break; fixed by the lead's `c440d90` (whose own push CI was then **cancelled** by supersession; its green evidence is the dispatch set below + `d967931`) |
| Render (dispatch) ×4 | `c440d90` | 43628279059 + 3 | **success** |
| CI + Tools CI | `d967931` | 43628681545 / 43628681576 | **success / success** |
| CI | `3c07564` | 43628889658 | **cancelled** (concurrency supersession — not a failure) |
| Tools CI | `3c07564` | 43628889684 | **success** |
| Visual Regression (dispatch) | `3c07564` | 43628892930 | **failure — as designed**: this gate read the log: **3 MATCH · 1 DRIFT · 14 NOT-RENDERED**; the sole DRIFT is s28 (KNOWN-DEBT under the W72-updated register the same commit landed); the runner stays red on s28 alone until the user's disposition |
| CI + Tools CI | `2fcffc0` | 43629008511 / 43629008503 | **success / success** |
| CI | `108f33f` | 43629313479 | **success** |
| CI + Tools CI | `3223509` (gate-close HEAD) | 43629625139 / 43629625142 | **in progress** at gate close — no conclusion claimed |

**Hygiene:** every Batch-9 seat deliverable is a new
untracked file (9 notes + `docs/integration/f3bc/`); no seat
edited a tracked file. Two working-tree facts, both the
lead's/other seats' lanes, recorded for completeness:
`WAVE2-RECORD.md` carried the lead's in-flight fold-in edits
during the window (since landed), and
`tools/format/exporter_acceptance.py` shows as **modified**
at gate close — that is W77's own tool file with its seat
apparently still iterating; the Batch-8 gate's acceptance of
the tool stands on the committed version, and the
modification is not any Batch-9 seat's work.

## 11. FINAL integration packet — what Batch 10 must contain

Batch 10 is the wave-close batch. Ordered:

### 11.1 Close verification (evidence, not new construction)

1. **Final WAVE2-RECORD audit** — re-run W79's method
   against the completed record at the freeze: every batch
   ledger (1–10), every incident (tools-ci absolute paths,
   gitignored fixture PNGs, edb058f delimiter, ab1 delimiter),
   both pin acts + the corrective, the adapter retraction +
   independent re-proof, the knobs record-error correction
   (P6) *and* the real three-phase landing + gate PASS, the
   F2 migration, the F3 strip chain (196 → 24 → 3, with the
   `near` consume and the fps/frames exit route stated), the
   §15 fold-in census, the G7 tables landing — each PRESENT
   with its verdict, or the fold-in is not done. The two
   historical "appended when checked" promises are discharged
   or struck in the text.
2. **G7 tables implementation check** (owed from §5): the
   landed `--tables` output on the real film data diffed
   against W75's prototype output — headline numbers
   (14 shots / 73 chunks / max 844 px / 25 beat-frames) and
   the disclosed D3/D5 deltas accounted; default (no-flag)
   telemetry output byte-identical to the pre-tables binary.
3. **Close-out report draft** — the wave's honest summary
   for the user: scope delivered vs the audit's phases;
   every correction the wave made to its own record
   (adapter PASS retraction, Act-1 pin bytes, knobs phase-1
   record error, bench-window claim, W42's fps/frames
   misclassification, the two transport truncations); the
   final seat accounting (100 seats + lead + replacement
   runs, counted from the batch ledgers, not rounded).
4. **Owed-note closure:** W68's PC dry-run note and W77's
   exporter note (real-binary run / ENV-BLOCKED report)
   either land in Batch 10 or are **formally waived in the
   record** — a third silent carry is not acceptable at
   close. (W68's absence does not block the PC session —
   W78 proved the runbook covers E7 — but the session stays
   *unrehearsed* until it lands.)

### 11.2 User decisions — assemble for ONE close conversation

Each with its prepared brief; none may be silently defaulted:

1. **Caption veto** (CAPTION-DECISION-W69): KEEP the
   73-chunk re-chunk (recommendation: keep — telemetry
   CHECK OK, splits at phrase boundaries) or VETO to the
   66-chunk data (waiver stands again; 3 in-window golden
   pins go stale and are restored from candidates).
2. **s28 disposition:** re-pin to current output (VR goes
   green; s28 becomes a byte-duplicate of the ab1 pin —
   the W03 packet recommends against), retire the item, or
   leave it red as a deliberate marker. Until decided, VR
   stays red on s28 alone — verified steady state (§10).
3. **Locomotion D1–D4** (LOCOMOTION-AB-W57 + W87's decision
   sheet): static-frame policy (D1, pack recommends full
   replacement), per-site arm bases (D2), arms only on
   advancing frames (D3), s11's branch-bounded site (D4).
   Motion quality is the user's call. Step-0 registration
   itself (W87) is zero-pixel and lead-landable without
   waiting for D1–D4 — but the *adoption* waits for them.
4. **Lipsync semantics** (LIPSYNC-BIND-W86): the lead's
   Variant A vs B call, plus the user's visual sign-off on
   the line-3 sheet (frames 482–632; the 570–586 silence is
   the headline judgement) before any future on-screen
   speaker binds the track. Changes zero frames of the
   current film either way.
5. **v1.0.1 go** (RELEASE-101B-W89): the release fires only
   on the user's explicit go at the freeze, then: bump
   `Cargo.toml` 1.0.0 → 1.0.1 (+ the README:65 line in the
   same commit), extend the changelog over the freeze
   delta, re-prove all six Step-3 gates **on the bump SHA**
   (CI, Tools CI, knobs gate, determinism CI leg, golden
   18 OK, telemetry CHECK OK from the downloaded binary),
   publish the CI-built binary only — **never re-attach the
   v1.0.0 proof MP4** (it predates the re-chunk).
6. **Skill adoption:** adopt the native backend into the
   shipped skill as opt-in (work copy
   `~/workspace/talerender-skill-wiring/` ready, adapter
   re-proof PASS §1 of GATE2-W70) or wait for RADV proof.
   Adoption ≠ default; Chrome stays renderer of record
   either way.

### 11.3 Fixes-pass remainder (patch-ready, lead's lane)

W76 micro-fixes (partly landed at `d967931` — verify the
remainder), W72 register (landed `3c07564`, verified §10),
W74 F3-a strip (landed `2fcffc0`), W88's landing series
(data commit + baseline 24 → 3; then the `near` consume with
baseline → 2; sets README B2 amendment), W71 §4 splice +
W78 G1/G2 paste-ins (landed `108f33f` — verify), DOCFIX2
remainder, W82's blueprint amendments applied to
FILM-SPLIT-W21 / SCENE-SPLIT-W22 before any split step runs,
and the split programmes themselves (22 pure-move commits;
**the battery dispatch leg must exist before S2/S4/S5**,
§2). s12 adoption (W83's shape), VRAM implementation
(W84's shape, D1/D2 ruled by the lead), locomotion step 0
(W87's one line).

### 11.4 Goal-status review — goal_b21124e71eb3 stays OPEN

What the wave closes: production integration proven on a
full short (Wave 1) plus the audit's Phases 1–5 code,
the gap-fix set F1–F5 in code (F6 design-only as ruled),
golden pins struck + verified, determinism CI leg,
telemetry exit codes, §15 in the format, knobs gated.

What remains open **after** the wave, with owners:

- **PC RADV evidence session** [user's PC]: every native
  number so far is CI/VM lavapipe. PERF tables (blank by
  design), the determinism **RADV leg (E7)** — the contract's
  last unproven leg — the RADV bench legs (W64 §5 addendum),
  knobs intermediate-value spot checks (§1), VRAM sysfs
  numeric path (W84 §5.4). W36's runbook + W78's paste-ins
  script the whole visit; W68's dry-run rehearsal is the
  only unready piece (§11.1.4).
- **Fog-chart PSNR leg** [reference-side]: the ≥45 dB leg
  vs the Chrome frame still needs the ×π-amended Chrome
  re-capture (house light factor; ambient ≈1.0996,
  directional ≈2.0420). The analytic-centre gate passed on
  both sides (≤1 code); the PSNR leg was never run.
- **Skill adoption** [user decision, §11.2.6].
- **s28 disposition** [user decision, §11.2.6/§11.2.2].
- **Colour QA** [user]: the 2026-11-14 "looks pale" report
  was measured *not* globally faded (native ≥ Chrome
  saturation at all six sampled points); still awaiting the
  user's timestamp/scene pinpoint. (The headless-three-
  renderer study's tone-mapping WGSL diff is a lab lead,
  not a finding.)

The goal closes only when the RADV session and the
fog-chart leg land or are formally waived by the user —
not at the wave's final push.

---

**Gate method note.** Duties (1)–(6) were executed
first-hand as recorded above (dispatch logs read directly;
state_key extracted and hashed; s12 guard + conversion
re-run; VRAM arithmetic recomputed mechanically; F3 corpus
re-run in three states; version census re-counted and
spot-read). Duty (7) is §10's run table, read at gate
close. Duty (8) is §10's hygiene paragraph. Seat claims not
re-measured are labelled "the seat's" where they appear.

— W90, Batch-9 gate
