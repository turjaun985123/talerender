# GATE2-W50 — Wave 2 Batch-5 gate + integration packet

**Seat:** TaleRender Wave 2, W50 (batch-5 gate). **Date:** 2026-11-14.
**Gate window:** 2026-11-14 17:39Z (first poll; no Batch-5 file
present) → 17:50Z (final snapshot). All nine seats landed by
~17:45Z; W42's note was the last file (17:49Z, during this gate's
runs — its tool and fixtures had landed earlier and were gated
against the tool's own register output before the note arrived).
**Tree state at gate close:** HEAD `edb058f` (lead: errors step 2);
working tree = 18 untracked paths, all Batch-5 seat files (§1) +
`__pycache__` run by-products in pre-existing tool dirs (§6.5);
**no modified tracked files**. HEAD moved three times during the
gate window, all lead commits: `617ea4e` (tools-ci classification
fix), the orchestrator's fixture-gitignore fix, `edb058f` (errors
step 2). No lead commit touched a Batch-5 file. Batch-5 seats
committed nothing and edited no existing file.

**CI at gate close (reported as observed, per standing law):**
tools-ci at HEAD **SUCCESS** (run 43622944902); main CI at HEAD
**FAILURE** (run 43622944889, job `build-and-render`) — the errors
step-2 commit's own message states its gates "follow", so this red
is the lead's in-flight item, recorded here as the tree's one open
CI fact, not as a Batch-5 finding.

**Method.** Deliverables judged on disk only. Every runnable
artefact was re-run by this gate; paper deliverables were
spot-checked against source at gate HEAD (`edb058f`) and against
`git log`. A seat's pasted output is quoted only where this gate's
run reproduced it.

**Gate verdict: PASS-WITH-NOTES** — nine seats accepted (two with
notes), no rejects; four cross-seat items flagged for the lead (§6).

---

## 1. Inventory — W41–W49 deliverables

| Seat | Deliverable(s) | Present | Sanity (one line) |
|---|---|---|---|
| W41 | `FILMJSON-W41.md` (14,056 B) + `tools/film/fixtures/entry/{filmjson,onodajson}/` | ✅ | F1 verified against source; entry fixtures for the CI acceptance tests; F2 asset audit, complete by construction |
| W42 | `tools/format/check_unknown_fields.py` (29,668 B) + 5 fixtures + `STRICTNESS-W42.md` | ✅ | Report-only unknown-key checker; known sets transcribed from the loaders; tolerated register citation-gated |
| W43 | `KNOBS-W43.md` (13,026 B) | ✅ | Spec-only `--res`/`--aa`/`--shadow`/`--draft`; defaults byte-identity law; draft labelling |
| W44 | `tools/film/make_sheet.py` (11,098 B) + `tools/film/fixtures/sheet/` (90 frames + generator + test suite) + `SHEET-W44.md` | ✅ | Skill's tile recipe over native frame dirs; exit 0/1/2 discipline; no audit claims |
| W45 | `CUES-W45.md` (18,123 B) | ✅ | Design-only native cue emission (F6 scope); blocked on seven skill-side answers, Q1 named, nothing invented |
| W46 | `RELEASE-101-W46.md` (30,717 B) | ✅ | v1.0.1 preparation pack: changelog, one bump point, exact-SHA release procedure; authorises nothing |
| W47 | `EXPORTER-W47.md` (43,841 B) | ✅ | Chrome-side exporter emission contract + sampling rules + format-gap register + acceptance design |
| W48 | `tools/ci/frame_hash_compare.py` (17,125 B) + `tools/ci/fixtures/frame_hash/` (5 dirs) + `SELFTEST-NATIVE-W48.md` | ✅ | Directory hash comparison in the visual-regression dialect + the native selftest leg spec (era pairing, band-not-strict) |
| W49 | `GAPFIX-MATRIX-W49.md` (28,236 B) | ✅ | G1–G19 / R0–R10 traceability vs HEAD with a verified triage and a missed-items register (D1–D8) |

---

## 2. Mechanical checks — this gate's own runs

### 2.1 W42 check_unknown_fields — fixtures

All five fixtures in one run (exit 1, by design — the run mixes
clean and planted files):

```
[1 parse + classify] PASS — SCENE ×3, FILM-DATA ×1, MANIFEST ×1
[2 scene vocabulary] FAIL — fx-unknown names 'bogus' ($), 'bogosity'
    ($.nodes[]), 'shininess' ($.nodes[].material), each with first path
[3 film-data vocabulary] FAIL — fx-film-data names 'studio' ($),
    'bogus' ($.captions[])
[4 manifest vocabulary] FAIL — fx-manifest names 'bogus' ($.segments[])
[5 tolerated register] PASS — fx-allowlisted lists 9 tolerated
    occurrences, each with its citation (atmosphere, atmospherePreset,
    captionsNote, shot, points[].channel, nodes[].channels, name,
    material.map, material.metalness)
OVERALL: FAIL (2/5, 6 unknown occurrences in 6 paths, 9 tolerated)
```

Matches W42's per-file record (§5 of its note) exactly, including
fx-clean and fx-allowlisted passing individually in its runs.

### 2.2 W42 check_unknown_fields — real data (the F3 evidence base)

This gate's run (17 files: `scenes/film/*.json` + `film/onoda.json`):

```
exit 1 — OVERALL: FAIL (3/5 checks passed, 179 unknown-key
occurrence(s) in 12 distinct path(s), 5519 tolerated occurrence(s))
```

W42's run (27 files — adds `scenes/*.scene.json`,
`scenes/sets/*.scene.json`, `scenes/atmospheres.json`,
`film/assets/plates.json`): **196 / 15 / 7,043**. The two runs
reconcile exactly: the delta is the three bookshop paths
(`lights.key.near` ×1, `nodes[].materials` ×2, textures `note`
×14 = 17 occurrences, 3 paths) plus tolerated volume from the
sets/top-level scenes. On the shared scope the findings are
**line-for-line identical** to W42's §6 (same paths, same counts,
same first-at indices):

- film data: root `film`/`fps`/`frames`/`project` ×1 each;
  `captions[].line` ×73, `captions[].width` ×73; `lines[].i` ×12,
  `lines[].text` ×12 — 174 dead occurrences (W42's F3-a).
- scenes: `textures[].note` ×2 + `motifLaw` ×1 (s10),
  `nodes[].note` ×1 (s11), ×1 (s12).

This gate independently confirmed the vocabulary from source
(`src/film.rs` load path): lines consume **only** `start`/`end`;
captions consume **only** `s`/`e`/`text`/`fit`/`strip`. W42's
known sets are the loader's, not the corpus's habits. Every
unknown key on the real corpus is provenance/record class — no
typo of a consumed key appears — which is the fact the lead's
hard-error decision (W42 §7, Option A recommended) turns on.

### 2.3 W44 make_sheet — fixtures + independent dimension check

```
$ python3 tools/film/fixtures/sheet/run_tests.py
OVERALL: PASS (10/10 checks)   exit 0
```
(A: 5 tiles in order, 286x96; B: defaults 1976x384; C: sampled
hole exits 1, frame named, no sheet left; D: no-ffmpeg exit 2;
E: --from/--to window selects frames 30/45 only.)

This gate's own default run on the fixture dir reproduced B
independently: 90 frames → samples t=0.5…2.5 s → frames
15/30/45/60/75, `SHEET OK … (5 tiles in 9x1, 216x384 each)`, and
the output JPEG's SOF header parsed by this gate reads
**1976×384** = 9·216 + 8·4 padding — the skill's tile geometry,
confirmed without trusting the tool's own arithmetic.

### 2.4 W48 frame_hash_compare — both modes, true exit codes

| Run | Verdict line | rc |
|---|---|---|
| strict, chrome vs native-identical | `FRAME HASH: PASS (3 MATCH)` | 0 |
| strict, chrome vs native-drift | `FAIL (2 MATCH, 1 DIFFERS)` — frame_00001, PSNR 10.53 dB | 1 |
| strict, chrome vs native-missing | `FAIL (… 1 MISSING-IN-CANDIDATE, 1 MISSING-IN-REFERENCE)` — frame_00002 / frame_00003 named | 1 |
| band 80, chrome vs native-withinband | `PASS (2 MATCH, 1 WITHIN-BAND)` — 90.28 dB ≥ 80 | 0 |
| band 80, chrome vs native-drift | `FAIL (… 1 DRIFT)` — 10.53 dB < 80 | 1 |
| band 80, chrome vs native-missing | `FAIL` — both missing classes named | 1 |
| `--self-test` | `SELF-TEST: PASS` (asserts exit codes internally) | 0 |

Exit codes were re-measured without pipes (a piped first pass
masked them — the self-test's internal assertions agree with the
unpiped values). All classifications match W48's §2 record.

---

## 3. W41 — F1 verification against source at gate HEAD

W41 surveyed at `f6616aa`; the claim set was re-verified at
`edb058f` (two lead commits later, one of which — errors step 2 —
touched `run_film`'s signature):

- `load_film_data` sits at `src/film.rs:63`, **verbatim as W41
  quotes it**: `film.json` existence wins unconditionally, else
  `onoda.json`; the choice is made before any read (so an
  unreadable `film.json` does **not** fall back); neither-file
  resolves to `onoda.json` and fails as `cannot read film data:
  …`, naming neither file. W41's both/neither/broken-symlink case
  analysis follows from the code exactly as written.
- **Telemetry shares the logic** — confirmed: exactly two call
  sites, `film.rs:3401` (render, wrapped as
  `TaleError::data_load` → exit 2) and `film.rs:3525` (telemetry,
  plain `?` → main maps to exit 3). No second implementation.
- Key-demand order (`anchors`, `lines`, `captions`, `duration`)
  and the §1.3 exit paths are as W41 states; its cited line
  numbers (63–75, 3401, 3525) are still exact at gate HEAD — the
  errors-step-2 commit did not drift them.
- F2 audit spot-confirmed: `load_assets`' unconditional set
  (plates five, calendar years 1945/1955/1965/1974,
  calendar-dark, sign) and the two silent fallbacks (non-object
  `plates.json` → zero plates; missing origin → (0,0)) are as
  W41 §3 states; W41's catch beyond the gap analysis — `sign.png`
  also **seeds `Renderer::new`** — is the kind of fact the audit
  existed to find. The §2.2 acceptance tests are honestly labelled
  CI-pending (the local binary predates F1; its era bracket is
  correctly argued).

**W41 verdict: ACCEPT.**

---

## 4. Paper spot-checks

### 4.1 W43 — hardcoded-value claims (7 sampled, 7 exact at gate HEAD)

| Claim | Source at `edb058f` |
|---|---|
| `composer::make_rt` hardcodes `sample_count: 1` (composer.rs:83, 109) | ✅ both lines read `sample_count: 1` |
| film HDR targets via `make_target(…, Rgba16Float, 4)` / `Depth32Float, 4` (film.rs:2545, 2551) | ✅ verbatim, labels "hdr msaa"/"hdr depth" |
| ab1 shadow_misc `[1.0/2048.0, 2048.0, 0, 0]` (ab1.rs:652) | ✅ verbatim |
| film shadow_misc (film.rs:2838) | ✅ verbatim |
| `--width/--height` defaults 1080×1920 (main.rs:174–175) | ✅ `let mut width: u32 = 1080; let mut height: u32 = 1920;` |
| `overlay_caption` design constants centre (540, 1340), half-height 150 (film.rs:3365+) | ✅ verbatim in the function head |
| WGSL carries the size only as a comment (shader_pbr.wgsl:51) | ✅ `// x: shadow texel size (1/2048), y: shadow map size (2048)` |
| ab1 inline MSAA creations, `sample_count: 4` (ab1.rs:842–867) | ✅ at 846 and 859 |

Spec discipline verified as briefed: defaults at §3.2 must be
byte-identical to today's output (an acceptance gate, not a
hope); inert knobs accepted silently per mode (§3.3); telemetry's
850 px audit explicitly never scaled (§1.3).

### 4.2 W46 — changelog grouping (count reconciled, 8 commits sampled)

W46 claims 20 commits in `3c2c129..f6616aa`, all accounted.
This gate counts **23** in `v1.0.0..HEAD` at `edb058f`; the three
past W46's snapshot are `617ea4e`, the orchestrator fixture fix,
and `edb058f` — the arithmetic reconciles exactly. Sampled
groupings all accurate: `d7bebdd` CLI (§2.1), `aa10b13` exit
codes, `5e90d7e` caption CHECK OK, `ba88596` errors step 1,
`86f0c81` paint-budget/runlog wiring, `aadc193` F1 (with its
honest boundary note — F1 does not make `--film` a generic
project loader), `2328e6f` registration (94 tests), the gpucore
5/5 sequence `df89ad6`/`c420d72`/`9f497c7`/`dc0e555`/`aadc193`,
`630d82e` tools-ci wiring, `f6616aa` Batch 4. W46 independently
flags the same registration caveat as W49's D7 (locomotion/
cameramoves unregistered) and forbids re-attaching the stale
v1.0.0 proof MP4. Bump surface verified: exactly one functional
point (`Cargo.toml` version), `--version` derived via
`env!("CARGO_PKG_VERSION")`.

### 4.3 W49 — matrix rows (8 sampled across status classes)

| Row | Class | Gate finding |
|---|---|---|
| G14 fade | STALE→CLOSED | ✅ `film.rs:2025`: `fade_factor = clamp(t/0.25, …).min(clamp((duration−t)/0.4, …))`; `apply_fade` applied at 3429 — the analysis's inference was wrong, W49's correction is exact |
| G16 deliverable | CLOSED | ✅ `film/finish.sh`: x264 `-preset slow -crf 18`, all BT.709 tags, faststart discipline — and AAC `-b:a 192k`, which simultaneously confirms **D3's OPEN deviation** (skill master is 256k) |
| G1 generic input | PARTLY CLOSED | ✅ title string hardcoded at film.rs:3410 and 3624; `frames_total = duration * 30.0` at 3404 — the "single-film program" remainder is real |
| G7 check parity | PARTLY STALE / OPEN remainder | ✅ telemetry.rs printers are exactly `print_audit`, `print_continuity`, `print_composition`, `print_caption_audit` — no shot table, full caption table, or cast list exists. The OPEN remainder is genuinely unqueued: no Batch-5 seat owns it (verified against this batch's assignments) |
| G9/D7 modules | STALE (driver) / QUEUED (adoption) | ✅ at gate HEAD: `mod anim;` (main.rs:19) and `mod charanim;` (:23) registered with **zero** non-comment call sites in `src/`; `locomotion`/`cameramoves` absent from the mod list entirely |
| R0/D5 reproducibility | OPEN | ✅ no `Cargo.lock` in the tree at gate HEAD; the "build twice → identical sha256" accept test remains unmet and unqueued, as W49 states |
| D2 tools-ci red | OPEN (at snapshot) | ⚠️ **Status evolved after W49's snapshot** — see §5.1 |
| G17/R9 errors step 2 | QUEUED (at snapshot) | ⚠️ **Landed during this gate** — `edb058f`; see §5.2 |

The OPEN rows sampled (G7 remainder, D5, D3, G18's PC-session
evidence) were each grepped against the wave docs: none is
claimed by any Batch-1–5 seat or the lead's recorded queue
except where W49 itself names the owner. W49's "unqueued"
assertions hold.

---

## 5. Status evolution during the gate (snapshot drift, recorded not hidden)

### 5.1 W49 D2 (tools-ci red) — discharged in two steps

W49 was right at its snapshot (`f6616aa`): the asset checker's
manifest mode failed on the runner over authored absolute paths.
The lead's classification fix `617ea4e` landed — but **its own
tools-ci run still failed** (43622601972), for a second, distinct
cause: fixture PNGs silently swallowed by the `*.png` gitignore.
The orchestrator's fixture fix closed that; tools-ci has been
SUCCESS since (43622881922, and at HEAD 43622944902). D2 is
**closed at gate HEAD**; the two-step path is recorded because
"the fix commit's own CI was still red" is exactly the kind of
fact a close-out report must not smooth over.

### 5.2 Errors step 2 — landed; its main-CI gate is the tree's one red

`edb058f` (during this gate) converted gpucore's adapter/device/
readback sites to `TaleResult` per W35's collapsed pack — so
W49's G17/R9 "QUEUED" and W46 §5's pre-release blocker list are
both stale on this point as of gate close. The commit message
states its verification gates (CI + dof/fx/ab1 pins + film probes
+ no-adapter exit-3 proof) "follow"; at the final snapshot the
main CI run for `edb058f` is **FAILURE** (43622944889,
`build-and-render`) while its tools-ci is SUCCESS. The gate does
not diagnose the failure (logs not inspected; not a Batch-5
artefact). It belongs at the top of the lead's list in §7.

### 5.3 W46's §5 blocker list

W46 (snapshot `f6616aa`) lists the tools-ci red as a release
blocker; per §5.1 it is discharged at gate HEAD. Its procedure
already forces re-verification on the exact release SHA (Step 3),
so no pack change is needed — noted here so the close-out does
not carry a dead blocker forward.

---

## 6. Cross-seat conflicts and seams (flagged, not resolved)

1. **W42's tolerated register vs W26/W28 — consistent, one scope
   seam.** The register cites its neighbours rather than
   competing with them: `points[].channel` is tolerated *because*
   check_lights check 6 reads it (LIGHTS-W26 is cited in the
   register entry); `atmospherePreset` is tolerated as a record
   precisely along W26's deliberate non-check; `overlays` is
   tolerated citing check_assets' `overlays[].ref` walk
   (ASSETS-W28). The seam: W42 does **not descend** tolerated
   subtrees (§8 of its note), so keys *inside* `overlays` /
   `shot` / `channels` records — including `ref` itself — are
   never vocabulary-checked by any tool. Owners' validators
   govern there; the lead should confirm that is the intent, not
   an accident of two tools' scopes.
2. **W41's fixture wording vs W42's vocabulary — evidence
   resolves it.** W41 §2.1 describes its fixture lines as
   `{i, start, end, text}` "shaped like the real file"; W42
   flags `i`/`text` (and captions' `width`/`line`) as unknown.
   Both are accurate about different things — the fixture mirrors
   the real file's *shape*, the loader consumes only
   `{start, end}` / `{s, e, text, fit, strip}` (this gate
   verified from `src/film.rs`). W41 itself names
   `film`/`fps`/`frames`/`project` as loader-ignored; its wording
   just doesn't extend the same note to the per-line/per-caption
   keys. No artefact changes; the lead may want the fixture
   comment aligned when F3-a is dispositioned.
3. **W43's caption-under-`--res` position vs the strips
   contract.** W43 §4.1 decides captions scale with the frame:
   the overlay's design-space placement is multiplied by
   `(ew/1080, eh/1920)`, so the pre-rendered strip is resampled
   in drafts. The strips contract (compatibility-contract §5 /
   W47 §1.7) treats strip bytes as final sRGB at design space —
   true at defaults, where W43's defaults-identity law (§3.2)
   keeps every byte. The tension is confined to draft mode and
   W43 argues it matches Chrome's whole-canvas draft semantics;
   its audit-side guard (telemetry stays design-space) is
   explicit. Flagged for the lead to ratify at implementation:
   the sampling/filter behaviour of the scaled overlay is
   unspecified in both documents.
4. **W47's field map vs W31's §15 draft — ease consistent, one
   forward reference.** W47 §2.2 works entirely inside the §14.4
   camera law (destination key carries `ease`; smootherstep
   default) and W31's §15 keeps format ease on the arriving key
   with the loader-side remap for anim — the two documents agree,
   and W47's dense-sampling fallback is exactly the bridge for
   rigs exceeding that law. The nuance: W47 §2.2 speaks of "the
   §15 clips evaluator" as the home of the wider easing set, in
   present tense — §15 is still an unfolded draft and no loader
   evaluates clips today. Harmless as design prose; the lead
   should keep the tense honest at fold-in. Channel naming in
   W47 §2.3 (`phase`, `bob`, `blink`, `talk`, `lean`, gaze) is
   prose-level and broadly matches the W15/W31 vocabulary; the
   normative list remains §15's.
5. **W48's classification language vs W16/W37 — a documented
   dialect extension.** W16's classes are MATCH / WITHIN-BAND /
   DRIFT / MISSING-RENDER / REFERENCE-MISSING / UNPINNED /
   NOT-RENDERED. W48 keeps the first three, renames the missing
   pair directionally (MISSING-IN-CANDIDATE /
   MISSING-IN-REFERENCE), and adds strict-mode DIFFERS, which has
   no W16 counterpart. W48 §1 documents the mapping explicitly,
   so a human reader is never misled; but any machine consumer
   keyed on W16 names (the W37 triage register's vocabulary)
   would not match W48 output verbatim. Flagged for ratification
   or harmonisation at selftest-leg adoption — the tool is
   correct either way; the dialect should be one.

---

## 7. Hygiene

- `git status` at gate close: 18 untracked paths — the nine
  Batch-5 notes, three tools, and four fixture directories
  (`tools/film/fixtures/entry/`, `tools/film/fixtures/sheet/`,
  `tools/format/fixtures/unknown_fields/`,
  `tools/ci/fixtures/frame_hash/`) — plus `__pycache__` in
  `tools/ab/`, `tools/ci/`, `tools/golden/` (pre-existing tool
  dirs; by-products of this gate's and the seats' runs).
- **No `__pycache__` inside any seat-owned fixture path**
  (find over the four fixture dirs: none).
- No tracked file is modified; the only tree movement during the
  gate was the lead's three commits (§5), none touching a
  Batch-5 path. No seat committed anything.

---

## 8. Per-seat verdicts

| Seat | Verdict | Basis |
|---|---|---|
| W41 | **ACCEPT** | F1 re-verified verbatim at gate HEAD incl. both call sites; F2 audit complete-by-construction, caught `sign.png` beyond the analysis; CI-pending honestly labelled |
| W42 | **ACCEPT** | Fixtures + real data reproduced; counts reconcile across scopes to the occurrence; recommendation (Option A ratchet) follows from its own evidence |
| W43 | **ACCEPT** | Spec-only as briefed; 7/7 sampled cites exact; defaults byte-identity is a stated acceptance gate; draft labelling in-band |
| W44 | **ACCEPT** | Suite 10/10 reproduced; sheet geometry independently confirmed from the JPEG header; differences from the Chrome sheet stated, not hidden |
| W45 | **ACCEPT** | Design stays inside F6's scope; the seven skill-side questions are the honest blocker and are asked, not assumed |
| W46 | **ACCEPT-WITH-NOTES** | Changelog reconciles to the commit; §5.3 of this gate discharges its tools-ci blocker; executor must extend the changelog past `f6616aa` (Batch 5, errors step 2) per its own Step 2 |
| W47 | **ACCEPT** | Emission contract + sampling rules coherent with §14.4/§15 as sampled; gap register and TO-VERIFY list keep TS-side unknowns explicit |
| W48 | **ACCEPT** | Both modes + self-test reproduced with true exit codes; band-not-strict reasoning is the era-pairing/JPEG-generation facts, not preference |
| W49 | **ACCEPT-WITH-NOTES** | 8 rows spot-verified accurate at snapshot; D2 and the errors-step-2 rows evolved after its snapshot (§5) — annotate at close, do not rewrite |

**Batch verdict: PASS-WITH-NOTES.**

---

## 9. Merged integration packet (Batch 5 + still-open queue)

For the lead, ordered by dependency. Items marked ◆ are new or
changed by this batch/gate.

1. ◆ **Tree red first:** main CI at `edb058f` FAILURE
   (`build-and-render`, run 43622944889) — the errors-step-2
   gate evidence (dof/fx/ab1 pins, film probes, no-adapter
   exit-3 proof) is still owed per its own commit message.
   Nothing else in this packet should land on a red tree.
2. **§15 fold-in + reservation guard** (Batch 4's W31 draft):
   paste §15 into `docs/scene-format.md` with the fail-loud
   reservation guard first (loader rejects unimplemented
   prefabs/clips by name), then expansion/evaluation; W42's
   Option C analysis (§7 of STRICTNESS-W42) is now the evidence
   that a key-level hard error is a *different, rejected-for-v2*
   act — do not conflate the two at fold-in.
3. ◆ **W42 strictness adoption (Option A):** wire
   `check_unknown_fields.py` into tools-ci in W39's shape and
   baseline the 15 known unknown-paths as the ratchet; then
   disposition the findings as data work — **F3-a**: teach
   `film/gen_film_data.py` to stop writing its 8 dead keys (or
   give `width`/`line` a cited consumer); **F3-b**: one citing
   sentence per `note`/`motifLaw` class in ASSUMPTIONS / sets
   README, or strip them; **F3-c**: extend sets README B7 to
   `lights.key.near` and decide `nodes[].materials` explicitly
   (the run's one shadow-format finding).
4. **Blink adoption:** Batch 4's evidence stands (smooth blink
   ≤ +219 paints, 6.06% ceiling, story beats preserved at +0) —
   the decision is ready for the lead/user; adoption wires
   charanim's evaluator into the film path, A/B-gated.
5. **s12 apply decision:** the standing caption/data apply item
   from the earlier packets remains the lead's call, now priced
   by W27's planner and audited by W42's checker — apply or
   defer explicitly at wave close; do not let it ride silently.
6. **Split programmes:** FILM-SPLIT-W21 / SCENE-SPLIT-W22 steps,
   serially, one module per commit, each gated on the byte-probe
   pack (W34) + telemetry verdict line.
7. **Atlas / buffer-pool / rendergraph adoption:** probation
   terms as recorded (atlas: Dyn first paint, atlas on second
   hit, never evict a live page mid-film; rendergraph:
   declare+validate beside the composer first). Byte-probes
   discharge neutrality per wiring.
8. **Errors step-2 remainder:** the migration itself landed
   (`edb058f`); what remains is its *evidence* (item 1) plus the
   W35 §3 invariants, which stay by design. `hex_rgb` stays last,
   per W35/W42-era consensus — loader-side colour validation is
   the recorded alternative.
9. **W16 / W17 first dispatches:** visual-regression (expected
   to fail on the two stale pins — read it with W37's triage;
   do not widen the 80 dB band) and the bench workflow; both are
   dispatch-only and still unrun.
10. **Golden pin acts + s28 disposition:** the 14 film
    first-pins and the ab1 re-pin remain two separate human
    acts; the s28 packet's recommendation against re-pinning
    stands until the user rules. Pins clear item 9's expected
    failure.
11. ◆ **F2 lead work list (from W41 §3.4):** make `plates.json`
    the manifest it already half-is (load its keys, not a
    hardcoded five; a non-object manifest must stop silently
    yielding zero plates); move the calendar year set +
    `calendar-dark` + `sign` into film data; keep caption strips
    as the model class. This is the generic-input (R1)
    remainder, sequenced with W47's exporter design.
12. ◆ **W43 knob implementation** (draft-only; defaults
    byte-identical is the acceptance gate; the resolve-target
    seam at samples == 1 is the one structural catch), ◆ **W44
    tools-ci step** (run the sheet fixture suite after the film
    validator step), ◆ **W45's seven skill-side questions**
    (Q1 — the exact `OP.cues()` entry shape — blocks any native
    cue implementation; route to the skill side, not a seat).
13. ◆ **W49's micro-fixes:** D3 — align `film/finish.sh` AAC to
    the skill master's 256k or record the 192k deviation as
    deliberate in MP4-EVIDENCE; D4 — correct the `--help`
    "segment scenes resolve from scenes/film/" wording (the
    driver evaluates shots in code). ◆ **G7 remainder** (telemetry
    shot table / full caption table / cast list) stays OPEN and
    unqueued — the lead's fixes pass or a Batch-6 seat; it is
    the last piece of R6 parity.
14. **v1.0.1 release at wave close** (W46 pack): user go/no-go
    first; then the pack's procedure on the final HEAD —
    changelog extended past `f6616aa`, version bump as the only
    functional edit, CI + tools-ci proven green on the exact
    release SHA, binary verified (`--version` = `talerender
    1.0.1`, telemetry smoke = CHECK OK exit 0) before upload,
    and the two user decisions on record (release go; caption
    re-chunk confirm/veto). D5 (no `Cargo.lock` →
    reproducibility unproven) must be surfaced in the release
    notes or closed by committing the lockfile — W46 §3.1/W49
    D5 carry the fact.

*End of gate.*
