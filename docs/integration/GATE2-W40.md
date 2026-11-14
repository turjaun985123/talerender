# GATE2-W40 — Wave 2 Batch-4 gate + integration packet

**Seat:** TaleRender Wave 2, W40 (batch-4 gate). **Date:** 2026-11-14.
**Gate window:** 2026-11-14 ~17:21Z (first poll; no Batch-4 file
present yet) → 17:35:30Z (final snapshot; all nine seats landed
by 17:31:58Z).
**Tree state at gate close:** HEAD `aadc193`; working tree = 13
untracked Batch-4 paths (§1) + `__pycache__` run by-products
(§6.6) + **no modified tracked files**. HEAD *moved four times
during this gate* — the lead completed the gpucore migration
in parallel: `c420d72` (site 2/5 scenefile), `9f497c7` (site 3/5
ab1), `dc0e555` (site 4/5 main), `aadc193` (site 5/5 film +
`request_adapter` duplicate deleted + W28's F3/F4 fixed). No
lead commit touched a Batch-4 file. Batch-4 seats committed
nothing and edited no existing file, per the wave model.

**Method.** Deliverables judged on disk only. Every check below
was re-run by this gate against the seats' own fixtures and the
real film data; a seat's pasted output is quoted only where this
gate's run reproduced it. Paper deliverables were spot-checked
against source at the HEAD each seat surveyed (`df89ad6` for
W31/W35/W38/W39) and, where the lead's in-window landings moved
the ground, again at `aadc193`.

---

## 1. Inventory — W31–W39 deliverables

| Seat | Deliverable(s) | Present | Size | Sanity (one line) |
|---|---|---|---|---|
| W31 | `docs/integration/FORMAT15-DRAFT-W31.md` | ✅ | 38,590 B | Paste-ready scene-format §15 (prefabs + clips) folding W14/W15 with the four reconciliations stated as decided rules per WAVE2-RECORD §9; explicitly *not* part of the spec until the lead pastes it |
| W32 | `tools/format/stage_clips.py` + `tools/format/fixtures/stage_clips/` (7 files) + `ANIM-ADOPT-W32.md` | ✅ | tool 52,309 B; note 12,340 B | Staging `channels` audit blocks + §14.4 camera → W15 clip documents; every non-converted record accounted by name under its §5.3 class |
| W33 | `tools/perf/blink_cost.py` + `BLINK-DECISION-W33.md` | ✅ | tool 20,902 B; note 13,669 B | Analytic (no renders) frame-accurate simulation of binary vs smooth vs quantized blink over the real staging; port self-checks on every run |
| W34 | `tools/ci/byte_probes.py` + `GPUCORE-PROBES-W34.md` | ✅ | tool 22,124 B; note 14,037 B | Same-renderer byte-identity probes over the standing ranges 195–204 / 795–814 / 1195–1204; refusal (exit 2) on an empty side; PSNR fallback is diagnostic-only |
| W35 | `docs/integration/ERRORS-STEP2-W35.md` | ✅ | 20,172 B | Step-2+ patch pack: 12 lines / 18 `expect` calls re-inventoried at `df89ad6`, ordered P1–P7 with per-patch gates; doc-quoted panic message flagged verbatim-preservation |
| W36 | `docs/runbook/PC-EVIDENCE-W36.md` | ✅ | 27,259 B | The one-visit RADV evidence session: E7 determinism leg, bench legs, full render + telemetry capture, golden *verification* (never pinning); claims no RADV numbers |
| W37 | `tools/ci/vr_triage.py` + `tools/ci/fixtures/vr_triage/` (3 fixtures) + `VR-DISPATCH-W37.md` | ✅ | tool 18,906 B; note 24,667 B | Classifies a first visual-regression report's rows as KNOWN-DEBT vs NEW-SIGNAL against a written register; never widens the 80 dB band, never whitelists |
| W38 | `docs/integration/DOCFIX-W38.md` | ✅ | 25,962 B | Paste-ready fixes for the W08 owner list (FINAL-REPORT addendum 2, film README/RESULTS, FOG §8, root README ×2) + PERF discharges found during verification |
| W39 | `docs/integration/TOOLSCI-W39.md` | ✅ | 17,014 B | The three Batch-3 validators verified as a set (real data + fixtures + negative controls) and unified into paste-ready tools-ci YAML; one real conflict found (C1, §6.1) |

No seat's deliverable is missing — code or note. (Contrast
Batch 3, where W29's note landed after the gate snapshot; this
batch's gate waited for all nine before opening.)

## 2. Mechanical checks — this gate's own runs

### 2.1 W32 stage_clips — fixtures, validator cross-run, real data

```
$ python3 tools/format/stage_clips.py …/fragment-camera-character.fragment.json -o /tmp/gate40/fcc.clips.json
OVERALL: PASS (4/4 checks passed)   exit 0
$ python3 tools/format/stage_clips.py …/fragment-nodes.fragment.json -o /tmp/gate40/fn.clips.json
OVERALL: PASS (4/4 checks passed)   exit 0
$ cmp fcc.clips.json fragment-camera-character.expected.clips.json   → byte-identical
$ cmp fn.clips.json  fragment-nodes.expected.clips.json              → byte-identical
$ python3 tools/format/check_clips.py fcc.clips.json fn.clips.json
OVERALL: PASS (5/5 checks passed)
```

All three negative fixtures reproduced: exit 1, check 2 FAIL,
**no output written** (`bad-unknown-channel` naming
`nodes[Hero].channels.zoom`; `bad-key-shape`; `bad-apply`).

Real data, this gate: `s03.scene.json` converts (OVERALL PASS
4/4) and its output passes `check_clips.py` **5/5** — the
brief's required camera-move + character-track run, reproduced.
`s12.scene.json` fails exactly as the note says: exit 1, no
output, both `apply` records named with their transforms
(`lean = bowA * 0.6`, `headTilt = -bowA`). The conversion is
therefore **proven on fixtures and sampled real data, not
asserted**; the note's full 14-file census (13 PASS, 63/65
key-carrying records converted) was not re-run in full — the
two endpoints of its claim (a clean file, the one blocked
file) both reproduce.

### 2.2 W33 blink_cost — real film data, headline comparison

Port self-checks PASS on this gate's run (charanim id-3
crossings 35 in [25,48]; Blinker events/120 s per seed 34–36).
Film totals, this gate's run vs BLINK-DECISION-W33's table —
**identical on every figure**:

| Model | Note | This gate |
|---|---|---|
| lookups | 3,612 | 3,612 |
| today | 43 ev, 159 on, D 1.00 | 43 ev, 159 on, D 1.00 |
| smooth | 36 ev, 255 on, D mean 7.08 max 8, **+219 (6.06%, 5.9 GiB)** | same |
| quantized | 35 ev, 199 on, D mean 4.51 max 6, **+123 (3.41%, 3.3 GiB)** | same |
| smooth + beats | 42 ev, 279 on, +219 | same |
| quantized + beats | 41 ev, 224 on, +123 | same |
| camera-hold share (smooth) | 17.6% | 17.6% (45/255) |

Per-segment rollups and per-character extras also match
number-for-number (s02 +32/+19 the largest; Onoda +86/+45;
s05a nobody staged; s05b zero under every model). W12's "~7
paints" estimate is confirmed by measurement: D mean 7.08.

### 2.3 W34 byte_probes — self-test + real frames

```
$ python3 tools/ci/byte_probes.py --self-test
… 13/13 assertions ok (identical → PASS exit 0; planted byte →
  FAIL naming frame 1197 at byte 1234, other ranges PASS;
  missing frame 801 named; empty side refused exit 2;
  PSNR 39.0999 dB == hand computation; identical PNGs → inf)
SELF-TEST: PASS   exit 0
$ python3 tools/ci/byte_probes.py ~/workspace/film-final/frames4 ~/workspace/film-final/frames4
BYTE PROBES: PASS (40/40 frames identical across 3 range(s))   exit 0
```

Both reproduce the note's §5/§6 verbatim (modulo the per-run
temp-dir suffix, as the note itself warns).

### 2.4 W37 vr_triage — self-test + both synthetic cases direct

```
$ python3 tools/ci/vr_triage.py --self-test
SELF-TEST: PASS (3/3 fixtures triaged as designed)   exit 0
$ python3 tools/ci/vr_triage.py tools/ci/fixtures/vr_triage/expected-first-dispatch.md
Counts: 1 OK · 17 KNOWN-DEBT · 0 NEW-SIGNAL (of 18 rows)
TRIAGE VERDICT: first-dispatch result is AS EXPECTED   exit 0
$ python3 tools/ci/vr_triage.py tools/ci/fixtures/vr_triage/planted-new-drift.md
Counts: 0 OK · 17 KNOWN-DEBT · 1 NEW-SIGNAL (of 18 rows)
TRIAGE VERDICT: contains NEW SIGNALS: dof-chart (DRIFT)   exit 1
```

The third fixture (planted MISSING-RENDER on the control) is
covered by the self-test's 3/3. The note's Runs A/B reproduce
row-for-row, including the s28 row's neither-act warning.

### 2.5 W39 three-validator sequence — at gate HEAD `aadc193`

W39 ran at `df89ad6`; this gate re-ran the whole sequence at
`aadc193` — i.e. *after* the lead's gpucore sites 2–5 rewrote
the shader checker's entire input tree:

| Run | W39 claims | This gate at `aadc193` |
|---|---|---|
| `check_lights.py --film scenes/film` | OVERALL PASS 6/6, exit 0, 0 warnings | **same** |
| `check_assets.py --root . scenes/film film` | 26 + 86 refs, 17 manifest warnings, 0 orphans, exit 0 | **same numbers** — but see the count note below |
| `check_shaders.py --repo .` | 21/21 PASS, 0 FAIL, 0 UNRESOLVED, exit 0 | **same** |

Fixture pairs (this gate, all as claimed): lights pass-pair
`OVERALL: PASS (6/6, 1 warning)`, exit 0; all 7 negative
fixtures exit 1; assets valid-scene PASS 5/5 exit 0,
valid-driver PASS 6/6 exit 0, missing + wrong-kind exit 1;
CWD pair — exit 1 from `/tmp` without `--root`, PASS 5/5
exit 0 with it.

**Count note (the sequence's one inaccuracy):** the assets
*manifest* target prints `OVERALL: PASS (5/5 checks passed)`
— check 5 (silent-fallback guards) is SKIPPED in manifest
mode — not the 6/6 that W39's §1b transcript and §4
expected-lines table print. W30's gate recorded 5/5 for the
same run; this gate reproduces 5/5 at two different HEADs.
Everything else in §1b (26 references, 17 warnings, 0
orphans, exit 0) is exact. The wiring is unaffected — W39's
own C4 forbids asserting count strings for this tool and the
steps gate on exit codes — but the §4 table row must be
corrected to `5/5 (manifest) + 6/6 (driver)` before the lead
pastes it as expected output (§6.3).

### 2.6 W39's C1 — verified against the workflow file

`.github/workflows/tools-ci.yml` at HEAD: both `push:` and
`pull_request:` path lists are `tools/**`, `scenes/**`,
`assets/captions/**`, `film/**`, `docs/golden/**`, `docs/ab/**`,
the workflow file — **no `src/**`**. W39's conflict is real
and W29's §9 claim ("already runs on `src/**`-touching
changes") is wrong. Without W39's edit (A), the shader step
would never run on the changes it exists to catch.

## 3. Paper-deliverable spot-checks

### 3.1 W31 — `anim.rs` identifiers (12 sampled, all exact)

Checked against `src/anim.rs` at HEAD: `Easing::{Linear, Step,
Smootherstep, EaseOutCubic, EaseInOutCubic, EaseOutBack}`
(anim.rs:84–91); `Easing::DEFAULT = Smootherstep` (:96);
`Easing::from_name` (:101); `Wrap::{Clamp, Loop, PingPong}`
(:166 ff.); trait `AnimValue` (:186) with impls for `f32`
(:195), `[f32; 3]` (:205), `Quat` (:267); `Quat(pub [f32; 4])`
(:232); `Key { t, value, ease }` (:308–312); `Track::with_duration`
(:382); `Clip::add_f32 / add_vec3 / add_quat` (:577/:583/:589)
and `sample_f32 / sample_vec3 / sample_quat` (:594/:601/:608).
Every quoted identifier exists with the claimed shape. §15.9's
four rulings match WAVE2-RECORD §9 items 1, 2, 6 verbatim in
substance, and §15.8's step law ("latest key with t <= tau")
is the same law W32's emitted data encodes evaluator-side —
checked, not a conflict (§6.4).

### 3.2 W35 — file:line cites (10 sampled at surveyed HEAD `df89ad6`)

| Cite | At `df89ad6` | Verdict |
|---|---|---|
| A1 `main.rs:334` adapter expect | exact, message verbatim incl. em-dash | ✅ |
| A2 `gpucore.rs:80` adapter expect | exact | ✅ |
| A3 `main.rs:498` `request_device failed` | exact | ✅ |
| A4 `gpucore.rs:103` `request_device failed` | exact | ✅ |
| A5 `ab1.rs:619` `request_device failed` | exact | ✅ |
| A6 `film.rs:2395` `request_device failed` | exact | ✅ |
| B1 `main.rs:875` readback idiom | exact | ✅ |
| B6 `gpucore.rs:295` readback idiom | exact | ✅ |
| B3 `scenefile.rs:1510` readback | idiom is at **1524** (−14) | ⚠️ shifted |
| B4 `scenefile.rs:2196` readback | idiom is at **2231** (−35) | ⚠️ shifted |
| (§4.2) `scene.rs:31` `hex_rgb` expect | exact (`"bad hex colour"`) | ✅ |

The two shifted cites are both in scenefile.rs — the file the
lead was editing *uncommitted* in the working tree W35
surveyed (W39's note records the same tree state). Containing
functions (`run_direct` / `run_composer`) and the exact
expressions are correct; the pack's own header instruction
("re-grep before applying each patch — every site is keyed by
containing function + exact expression") covers the drift.
The header's claim "exact at `df89ad6`" is, for those two
lines, exact at the *working tree* instead — flagged, not
fatal (§6.2).

**Absorption check at gate HEAD (this gate's grep):** at
`aadc193`, `grep "map channel closed | request_device failed
| no wgpu adapter"` over the five entry files finds sites
**only** in `src/gpucore.rs` — :80 (W35's A2), :103 (A4),
:295 (B6). main.rs / ab1.rs / scenefile.rs / film.rs have
**zero** remaining sites of these classes: the gpucore
migration absorbed 9 of W35's 12 inventoried lines (their
adapter/device/readback calls now route through gpucore).
Step 2 therefore reduces to W35's P1 (B6) + the gpucore half
of P5 (A2+A4), plus P6's one manual no-adapter failure-path
proof. P6's in-place migration is moot exactly as W35
predicted ("if the gpucore migration reaches main/ab1 first
and deletes this helper, P6 is moot"). `hex_rgb` is untouched
by all of this — still last, as scheduled.

### 3.3 W38 — disposition calls (4 sampled against the files)

| W38 call | Actual file at HEAD | Verdict |
|---|---|---|
| Film README telemetry line "still stale" (§2.3) | `docs/ab/film/README.md` still reads "Telemetry verdict: _pending_ (not run by the fill-in seat…)" | ✅ still stale, as called |
| PERF preamble telemetry claim stale since `aa10b13` (§7.2) | `docs/runbook/PERF.md:21–22` still reads "the binary exits 0 on pass, fail, and error alike (§7)" | ✅ still stale, as called |
| FINAL-REPORT §5.3 dispositions (§1 table) | Caption CLOSED-by-data, pins open, fog leg open — matches WAVE2-RECORD + lead commits | ✅ |
| Root README caption clause (§5.2) | Quoted text "telemetry verdict recorded with the caption waiver ruling" is verbatim in the Production-integration row | ✅ text exact — ⚠️ **line cite wrong: the row is at line 30, not 26** |

One more application-time staleness, same class as W35's:
W38's paste-ready Addendum 2 describes the gpucore migration
as "site 1 of 5 (dof.rs) at `df89ad6`". At gate close all five
sites have landed and finding B is closed (`aadc193`). The
lead must refresh that sentence when applying the addendum,
or the addendum will be born stale (§6.2).

### 3.4 W36 — CLI flags vs `src/main.rs`, determinism procedure vs the pack

Flags W36 puts on the `talerender` binary, checked against
the `--help` text / parse arms in `src/main.rs` at HEAD:
`--film` ✅, `--film-telemetry` ✅, `--from` ✅, `--to` ✅,
`--out` ✅, `--scene` ✅ (values `m1 | ab1 | dof-chart`),
`--scene-file` ✅, `--version` ✅ — 8/8 sampled exist with the
semantics W36 uses (its §3.1 telemetry-first command and
§3.2 `--film film --out radv-frames/` match the usage lines
verbatim). The remaining flags in the note (`--limit`,
`--workflow`, `--pin`, `--pair`, `--repo`, `--summary`,
`--native-label`, `--reference-label`) belong to `gh`,
`make_golden.py`, `bench_report.py`, `vulkaninfo` — not the
binary — and are used in those tools' own sections.

Procedure check: W36 §1's description of the determinism
pack (frames [100, 130); A single / B chunked into the same
dir / C at `TALERENDER_PAINT_CACHE=24`; sha256 compare; exit
2 = refusal) matches `tools/determinism/determinism_check.sh`
line-for-line (its header states the same legs; FROM=100,
TO=130 in the script). Its expected telemetry line is the
current `FILM TELEMETRY: CHECK OK`. The pack names no RADV
number anywhere — durations are anchored to the lavapipe
bound and labelled orientation, exactly per the wave's
honesty rule.

## 4. Per-seat verdicts

| Seat | Verdict | Basis |
|---|---|---|
| W31 | **ACCEPT** | 12/12 sampled `anim.rs` identifiers exact (§3.1); the four reconciliations encode the lead's recorded §9 decisions without inventing new semantics; the draft repeatedly marks what does *not* exist yet (colour/step lanes land with adoption — "until then no evaluator lane is claimed to exist") |
| W32 | **ACCEPT** | Fixtures byte-identical to the frozen expected files; W15-validator compatibility proven by this gate's cross-run (5/5 on both fixtures and on real s03 output); negatives fail by name with no output; s12's block is the tool refusing, correctly (§2.1) |
| W33 | **ACCEPT** | Every headline number reproduced exactly on the real film data (§2.2), per-segment and per-character included; the r = 1 ceiling is labelled a ceiling with directional evidence, never sold as a measurement; the beats finding (+0 cost to keep 7 authored blinks) is the kind of thing only a measurement catches |
| W34 | **ACCEPT** | Self-test 13/13 and the real-frames identity run reproduce verbatim (§2.3); limits are stated as precisely as W29's (proves no-change, not correctness; same-renderer only). Note: §3's per-step table was written against a migration in flight — by gate close all five sites have landed, so the table is now the standing instrument for *future* renderer/cache changes (and for re-running the film site's verification on demand), not a pending checklist |
| W35 | **ACCEPT-WITH-NOTES** | 8/10 sampled cites exact at the surveyed HEAD; the 2 scenefile cites are shifted by the lead's concurrent uncommitted edit the seat was actually reading (§3.2). The pack's substance survived the batch: its P6-moot prediction came true in-window, and the absorption grep (§3.2) shrinks step 2 to P1 + gpucore-P5 — the lead re-keys the sequence at `aadc193` before applying |
| W36 | **ACCEPT** | 8/8 binary flags verified against `main.rs`; the E7 procedure matches the pack script line-for-line (§3.4); golden section is verify-never-pin throughout; no RADV number is claimed or implied anywhere |
| W37 | **ACCEPT** | Self-test 3/3 plus both direct fixture runs reproduce the note's verdicts and exit codes exactly (§2.4); the register matches the W03/W16 record, including the s28 neither-act warning stated where a future reader will trip over it |
| W38 | **ACCEPT-WITH-NOTES** | All 4 sampled disposition calls correct against the files (§3.3). Notes: one line cite off by 4 (§5.2: row at :30, not :26 — quoted text exact, edit still findable), and the Addendum-2 paste text's gpucore sentence was overtaken by the lead's in-window landings and must be refreshed at application (§3.3) |
| W39 | **ACCEPT-WITH-NOTES** | The full sequence + every fixture pair reproduced at a *later* HEAD than the seat's (§2.5) — the wiring expectations survive the gpucore migration; C1 is a genuine, verified workflow defect (§2.6). Note: the §1b transcript / §4 expected table misstate the assets manifest count as 6/6 (actual 5/5, check 5 SKIPPED in that mode) — correct the table row before pasting (§2.5) |

**Batch-4 gate overall: PASS-WITH-NOTES** — all nine seats
landed complete (code *and* notes); every runnable artefact
this gate could execute is green and number-identical to its
note: stage_clips fixtures byte-identical + validator 5/5,
blink film totals exact, byte-probes self-test + 40/40 real
frames, vr_triage 3/3 + both verdict shapes, the three
Batch-3 validators green as a set at the new HEAD with all
fixture pairs biting. The notes are all of one family —
packs written against `df89ad6` while the lead landed four
commits underneath them (W35's two cites, W38's one sentence,
W34's §3 tense) — plus two self-contained slips (W38's line
26→30, W39's 6/6→5/5). No defect in any seat's code or
analysis; the open threads are integration steps with named
owners (§5) and conflicts flagged for the lead (§6).

## 5. Integration packet for the lead (landing order)

*Merged queue: Batch-4 landings interleaved with the still-open
Batch-1/2/3 items as this gate understands them from
GATE2-W10 §5, GATE2-W20 §5, GATE2-W30 §5, WAVE2-RECORD §§7–9
and the lead's own commits through `aadc193`. Conflicts are
flagged, not resolved (§6).*

**Already landed by the lead during Batches 3–4 (context, not
queue):** module registration + gpucore test fix (`2328e6f`,
`38930fa`), PaintBudget + RunLog wiring (`86f0c81`), render.yml
paint-budget input (`fbf37c0`), and — in this gate's window —
**the gpucore migration complete, sites 1–5** (`df89ad6`,
`c420d72`, `9f497c7`, `dc0e555`, `aadc193`), closing audit
finding B (the `request_adapter` duplicate is deleted) and
W28's F3/F4 with it. Batch-2 adoption decisions (colour/step
lane, ease bridge, W12 ratified independent, atlas probation,
wrap/duration) stand as recorded in `11a7a2e` §9.

1. **Central Batch-4 commit** (orchestrator): all §1 files —
   9 notes, 4 tools (`stage_clips.py`, `blink_cost.py`,
   `byte_probes.py`, `vr_triage.py`), 2 fixture dirs
   (`tools/format/fixtures/stage_clips/`,
   `tools/ci/fixtures/vr_triage/`). **Exclude `__pycache__`**
   (tools/format, tools/ci — seats' and this gate's runs;
   tools/ab + tools/golden pre-date the batch). CI
   expectations: `ci.yml` green, test count unchanged (no
   `src/` change in the batch); `tools-ci.yml` green (none of
   the new tools is wired into it yet — items 2–3).
2. **W39 tools-ci wiring** (lead; `.github/workflows/tools-ci.yml`):
   paste W39 §4 verbatim — trigger edit (A) adding `src/**`
   to **both** path lists, then the five steps — with one
   correction from §2.5: the expected-lines table's assets
   row reads `5/5 (manifest) + 6/6 (driver)`. This lands the
   Batch-3 packet's steps 3–4 (shader checker — its note
   dependency is satisfied, SHADERS-W29.md is in tree — plus
   the lights/assets validators) and discharges the standing
   "validators unwired" caveat.
3. **W32 stage_clips step** (lead; tools-ci, after or with
   item 2 — it is *not* in W39's YAML, §6.5): run the
   converter over `scenes/film/*.scene.json` into a temp dir
   and `check_clips.py` over the outputs, tolerating exactly
   one expected failure (s12) until item 8's decision lands.
4. **§15 fold-in** (lead; `docs/scene-format.md`): paste
   W31's §15 after §14. Confirm the colour lane's name at
   paste time (`Color` in the draft vs `AnimValue::Color` in
   WAVE2-RECORD §9.1 — §6.4). Then §15.10's order binds:
   **reservation guard first** (scenefile.rs + film.rs node
   walk), expansion/evaluation second (W14/W15 fixture sets
   become loader tests), production data last. W14's
   fail-loud rule still binds prefab data.
5. **Errors step 2 — re-keyed** (lead): re-inventory at
   `aadc193` first; per §3.2's absorption grep the remaining
   surface is `gpucore.rs:80 / :103 / :295` only. Apply W35's
   P1 (B6 `Readback::collect*` → `TaleResult`, `dump_packed_depth`
   with it) and the gpucore half of P5 (A2+A4 →
   `TaleResult`, §5's verbatim-message rule for the adapter
   string), then P6's one manual proof: the CI binary in a
   no-ICD job must print `error: GPU initialisation failed …`
   and exit **3**, not panic/101. P2/P3/P4/P6/P7's local sites
   no longer exist — do not hunt them. `hex_rgb` stays last
   (W35 §4.2's loader-validation split is the optional
   companion patch). PERF.md:295's row label and
   USER-PC.md:153 need their dated corrections when this
   lands (W35 §5) — W38's pack does not cover those two lines.
6. **Blink adoption decision** (lead, on W33's evidence):
   the measured pack is the decision input — recommendation
   on record is **smooth + beats** (ceiling +219 paints,
   6.06% of 3,612 lookups; beats restored at +0 measured
   cost), fallback **quantized + beats** (+123, 3.41%,
   key-space capped at 8 values). Either variant ships only
   with the beats maxed back in (W33's §"beats question").
   Follow-up measurement on the next full render: read
   `misses − rerenders` off the `paint-cache:` line for the
   true denominator and the realised r. The decision itself
   is the lead's/user's — this gate takes no position.
7. **Animation adoption, per-site** (lead, continuing):
   clips loader per item 4; §14.4 camera keys first (A/B-gated,
   ~1 ulp f32 caveat per §15.8); W32's converted corpus is the
   staging data (139 tracks / 684 keys over 13 files). W23
   lipsync adoption (Batch-3 packet item 6) composes here —
   its `talk` clip is already W15/§15-shaped.
8. **s12 `apply` decision** (lead): apply the two affine
   factors (`lean = bowA * 0.6`, `headTilt = −bowA`) in the
   staging data, or extend `stage_clips.py` to apply them
   deliberately. Blocks 2 of 172 records and item 3's CI
   tolerance. Nothing else is blocked on it.
9. **Locomotion adoption — gated, per-site** (lead;
   unchanged from Batch-3 packet item 7): shared-driver
   sites first; bob-phase and arm-swing pixel changes go
   through A/B + user sign-off; prop/story arms never
   overwritten.
10. **Split programmes** (lead; unchanged from Batch-3
    packet items 9–10): W21 film split steps 1–4, 6–10
    (step 5 consumed; G-TEL re-pointed at `CHECK OK`, exit
    0), W22 scene split steps 1–13 (line numbers were exact
    at `11a7a2e`; re-grep after the gpucore landings —
    scenefile.rs moved, scene.rs did not). Serial, CI-green
    commits; the `crate::scene::` path-stability seam
    (GATE2-W30 §6.4) still binds.
11. **Still-open Batch-1/2 queue, unchanged owners:**
    W11 anim per-site adoption (item 7's frame); W12
    charanim beyond blink (idle/gaze/expression — blink is
    item 6); W13 rendergraph steps 2–3; W18 atlas wiring
    under its probation policy + §7 byte-probes (W34's pack
    is now the instrument); W19 buffer pool under gpucore's
    `Readback` (note: gpucore migration complete — the pool's
    landing spot is final); bridge repackage + film
    re-render with the A/B footnote for the 10 caption-window
    pairs (RECHUNK-W01 §6 items 3–4); W27's planner as the
    standing first step of any future data-change re-render.
12. **W16 first dispatch** (lead/orchestrator): dispatch
    `visual-regression.yml` when ready and read the result
    with **W37's triage** — the register is valid only while
    its §3.1 preconditions hold (golden-hashes still the 4
    founding pins, neither act landed). Expected: runner
    `FAIL (3 pins, 2 drift)`, dof-chart MATCH → triage
    **AS EXPECTED**. Nobody widens the 80 dB band.
13. **W17 first bench dispatch** (unchanged): harness
    verification only; lavapipe numbers are never promoted
    to PERF claims.
14. **Golden pins — two separate human acts** (unchanged,
    W03): Act 1 = 14 film first-pins, Act 2 = ab1 re-pin —
    with W37 §2's warning now on the packet: **neither act
    clears `s28-onoda-t150`**; that pin needs its own human
    disposition (deliberate re-pin or retirement) or the VR
    job stays red on s28 alone, by design.
15. **PC RADV session** (user's PC; W36 is now the complete
    script): E7 determinism leg (contract TBD-PROOF
    discharge), bench legs filling PERF's tables, full film
    render + telemetry capture, golden verification per
    W36 §4. Risk R10 stays OPEN until this runs; every
    number before it is CI/VM lavapipe.
16. **DOCFIX application** (lead/doc seat): apply W38's
    paste blocks with its two refreshes — §5.2's cite is
    line **30**, and Addendum 2's gpucore sentence updated
    to sites 1–5 complete at `aadc193` (finding B closed).
    Its §8 verification greps then close the W08 owner list.

**Deliberately NOT wired by this batch** (for the record):
stage_clips / blink_cost / byte_probes / vr_triage in any CI
(item 2 wires only the Batch-3 three); the §15 text in
`docs/scene-format.md` (draft only); the `Color`/step
evaluator lanes (specified, unimplemented); W35's patches
(pack only — no `src/` touched); W36's session (prepared,
not run — no RADV evidence exists).

## 6. Conflicts & coordination findings

1. **W39-C1 — tools-ci never triggers on `src/**` (verified,
   §2.6).** W29's hookup note claimed the workflow already
   runs on src-touching changes; it does not, in either path
   list. W39's proposed resolution (add `src/**` to both
   lists, accepting that tools-ci then also runs on
   Rust-only pushes) is the only one on the table. **Flagged
   for the lead to ratify; this gate picks neither side** —
   but wiring the steps without the trigger edit would
   produce a gate that is green precisely when it should be
   watching.
2. **Two packs surveyed a tree the lead was standing on.**
   W35's scenefile cites and W38's gpucore sentence (and
   W34 §3's present tense) all describe `df89ad6`-era state;
   the lead landed sites 2–5 during the gate window. In
   every case the seat's *analysis* holds and only its
   *present tense* expired — the same pattern as Batch 3's
   W21 postscript. Standing recommendation, third batch
   running: packs that cite line numbers should name the
   working-tree state they read (committed HEAD vs dirty
   tree) in their headers; W39 did ("the working tree
   carries the lead's uncommitted scenefile edit") and its
   claims needed no such note.
3. **W39's §4 expected-lines table misstates one count**
   (§2.5): assets manifest `6/6` → actual `5/5`. The seat's
   own C4 ("no global check-count assertion is safe for
   check_assets") is the correct doctrine and contradicts
   its own table — the table is what a future operator will
   read. Correct before pasting (packet item 2).
4. **W31 vs WAVE2-RECORD §9.1 — a name, not a semantics.**
   The record names the colour extension `AnimValue::Color`;
   W31's draft names the lane `Color`. At HEAD `AnimValue`
   is a trait (anim.rs:186), so the implementable form is a
   new lane type — W31's reading is the coherent one, and
   both texts agree on substance (linear-space `[f32; 3]`,
   loader converts by the hex→linear law before the track
   carries it). Lead to confirm the name at fold-in so the
   spec and the record don't diverge in print. The related
   W31↔W32 surface was checked and is **not** a conflict:
   §15.8's step law and W32's `ease: "step"`-on-every-key
   emission are the format and evaluator descriptions of
   the same rule, and §15.8's "position endpoints migrate,
   gait stays code" is exactly W32's segLerp treatment.
5. **Tools-ci wiring is a three-layer queue, not one paste.**
   W39 unifies the Batch-3 three (lights/assets/shaders).
   W32 proposes a fourth step (stage_clips, §5 item 3) and
   §15.10 step 3 will want check_prefabs/check_clips steps
   at adoption. No two proposals contradict; they were
   written in ignorance of each other's scope. The lead
   lands them in packet order rather than expecting any
   single pack to be the whole wiring.
6. **W36 and W34 use different ranges — by design, checked.**
   The brief for this gate named "W36 commands vs W34 probe
   ranges" as a conflict candidate. There is none: W36's
   determinism leg runs the W04 pack (frames [100, 130),
   contract §4 — the E7 instrument of record), while W34's
   probes cover the migration ranges (195–204 / 795–814 /
   1195–1204, contract §5). W36 never invokes byte_probes.py,
   correctly — a single PC render produces no same-renderer
   baseline pair to probe against; its §4 golden check is
   hash-then-PSNR against the staged candidates, which is
   the contract §2 discipline for cross-renderer comparison.
   The synergy is unclaimed, not conflicting: the §3.2 full
   render's frames are exactly what a future RADV
   byte-probe baseline would be struck from.
7. **`__pycache__` artefacts** (packet §5.1): tools/format
   (check_clips / check_prefabs / stage_clips `.pyc` from
   seats' and this gate's runs), tools/ci (check_shaders
   `.pyc`), plus the pre-existing tools/ab + tools/golden —
   all run by-products; exclude from the central commit.
8. **Concurrent lead work is not seat work.** HEAD's four
   moves during the window are the lead's gpucore sites
   2–5. Batch-4 verdicts judge only the untracked seat
   files; nothing of the lead's is in the batch, and no
   seat file was touched by a lead commit.

*End of gate record. This seat edited nothing but this file
and committed nothing.*
