# Doc-fix pack, refreshed — W67 (Wave 2, Batch 7), 2026-11-15

**Purpose.** Batch 4's `DOCFIX-W38.md` was written against HEAD
`df89ad6` and never applied. This pack audits **every** W38 item
against the current tree and is the lead's **single doc-fix queue
for the fixes pass**: apply from here, not from W38. Two of
W38's paste-ready texts are now wrong and must not be applied
verbatim (§1 pins sentence, §1 adapter sentence; §2.4 and §3
golden bullets in part).

**Verified at HEAD `4dd6530`** ("Wave 2 lead: scene-format §15
fold-in + reservation guard"). The brief for this seat named
`3a3a427`; HEAD had advanced by that one lead commit when the
audit ran. It changes none of the findings below (it touched
`docs/scene-format.md` + `src/`, no W38 target file).

**Method (per the seat law).** Every "applied" claim below was
verified by **reading the target document at HEAD**, never from
a commit message. Headline result of that check:

> `git log df89ad6..HEAD` over all seven W38 target files
> (`FINAL-REPORT.md`, `docs/ab/film/README.md`,
> `docs/ab/film/RESULTS.md`, `shaders/fog/FOG.md`, `README.md`,
> `docs/runbook/PERF.md`, `docs/runbook/USER-PC.md`) returns
> **zero commits**. The lead's `b6ee9fb` fold-in updated
> `WAVE2-RECORD.md` only — it applied **none** of W38. Every
> W38 "OPEN" item is still open in the documents themselves.

**Verdict classes.** APPLIED (text is in the target doc, read
first-hand) · STILL-VALID (W38 text applies as written;
line numbers refreshed) · SUPERSEDED (the state it described
was replaced by a later landing) · WRONG-NOW (applying it
would put a false statement into the record).

## Master audit table

| W38 § | Item | Verdict at `4dd6530` |
|---|---|---|
| 1 | FINAL-REPORT Addendum 2 — caption bullet | STILL-VALID |
| 1 | FINAL-REPORT Addendum 2 — re-render / s08 / determinism-CI-leg / lead-README bullets | STILL-VALID |
| 1 | FINAL-REPORT Addendum 2 — "Golden pins STILL OPEN; no pin struck" | **WRONG-NOW** — both acts STRUCK (`916a109`, `166873c`) → §1 refreshed block |
| 1 | FINAL-REPORT Addendum 2 — "adapter real-binary FAILED-direction re-proof passed (recorded in `11a7a2e`)" | **WRONG-NOW** — retracted by the `b6ee9fb` correction → §1 refreshed block, §9-N2 |
| 1 | FINAL-REPORT Addendum 2 — closing paragraph (errors step 1, gpucore "site 1 of 5") | SUPERSEDED — gpucore 5/5 complete (`aadc193`); errors step 2 landed (`edb058f`, fix `aa92cee`) → §1 refreshed block |
| 1 | §5.3 disposition rows: fog-PSNR leg / PC RADV / skill adoption / final push | STILL-VALID (all still open) |
| 2.1 | film README header status (lines 3–5) | STILL-VALID, tail refreshed (§2.1) |
| 2.2 | film README step-1 exit-code comment (line 126) | STILL-VALID verbatim |
| 2.3 | film README telemetry verdict (lines 174–175) | STILL-VALID verbatim |
| 2.4 | film README golden verify (lines 210–213) | **WRONG-NOW** in its tail ("pins still outstanding") → refreshed §2.4 |
| 2.5 | film README MP4 evidence (lines 214–215) | STILL-VALID verbatim |
| 3 | RESULTS discharge note — telemetry + MP4 bullets | STILL-VALID |
| 3 | RESULTS discharge note — golden bullet ("pinning … has not been struck") | SUPERSEDED → refreshed §3 |
| 3 | RESULTS bullet 4 (PC RADV still none) | STILL-VALID (still true) |
| 4 | FOG.md §8 closure note | STILL-VALID verbatim (insert after line 385) |
| 5.1 | Root README Build & run CLI paragraph | STILL-VALID (insertion point moved: after line 58) |
| 5.2 | Root README Status-row caption clause | STILL-VALID (row is now line 30) |
| — | Root README "Remaining, by design" paragraph, pins clause (lines 38–41) — *not in W38* | **WRONG-NOW** → new item §5.3 |
| 6 | USER-PC.md `--help` claim | APPLIED — closed at `d7bebdd`; verified by reading lines 7–13 today. No action |
| 6 | PERF.md `--help` claim | APPLIED — closed at `d7bebdd`; verified by reading lines 24–26 today. No action |
| 7.1 | PERF §9 discharge note, items 1–4 | STILL-VALID verbatim (lines unchanged: 333/341/348/352) |
| 7.1 | PERF §9 item 5 row ("STILL OPEN, no text change") | SUPERSEDED — W56 design exists → new note §7.5 |
| 7.2 | PERF preamble telemetry claim (lines 21–22) | STILL-VALID verbatim |
| 7.3 | PERF §7 Telemetry FAILED row (line 298) | STILL-VALID verbatim |
| 7.4 | PERF §4 cache figure (line 195) + §7 OOM row (line 294) | STILL-VALID verbatim |
| 8 | W38 application order + greps | SUPERSEDED by §10/§11 below |

**New items created by the interval** (detail in §9):
N1 pins STRUCK — FINAL-REPORT + film README + RESULTS +
root README all still say/imply unpinned. N2 adapter
re-proof CORRECTION — the retracted PASS must not propagate
into FINAL-REPORT §5.3's neighbourhood. N3 PERF §9 item 5 —
designed (W56), not implemented.

---

## 1. docs/integration/FINAL-REPORT.md — Addendum 2, refreshed

**Current state (read today).** 953 lines. §5.3 at lines
828–927; Addendum 1 at 928–953. No Addendum 2 exists. Append
the refreshed block at EOF (after line 953). Do **not** use
W38's block: its pins sentence and adapter sentence are the
two WRONG-NOW items in the table above.

```markdown
---

## Addendum 2 — 2026-11-15 (Wave 2 landings; via DOCFIX2-W67)

§5.3 above is the wave-1 close list and is not rewritten. Its
state after Wave 2 (verified at HEAD `4dd6530`):

- **Telemetry caption disposition — CLOSED BY DATA.** The
  re-chunk option was taken: caption data regenerated 66 → 73
  chunks (Batch 1, `3a36f1f`), the driver leg regenerated by
  the lead (`5e90d7e`), and the re-run prints
  `FILM TELEMETRY: CHECK OK` with exit 0 (0 caption failures).
  The ACCEPT-WITH-WAIVER ruling (`9790553`) is superseded.
  Telemetry also gained verdict exit codes (0 OK / 1 CHECK
  FAILED / 3 could-not-run; the verdict line is unchanged and
  authoritative) in `aa10b13`.
- **Adapter re-proof — CORRECTED RECORD.** An earlier record
  (`11a7a2e`) reported the skill adapter's real-binary
  FAILED-direction re-proof as PASS. That claim is
  **retracted** (correction recorded in `b6ee9fb`): the run
  was SIGKILLed mid-telemetry under VM contention and its
  fixture paired 66-chunk data with the 73-era strip set, so
  it proved neither direction. The OK direction stands as
  proven (real binary on the matched 73-chunk fixture prints
  `CHECK OK`, adapter relays rc 0). The corrected fixture was
  rebuilt from `aa10b13` (66 captions, 78 asset files) and the
  FAILED-direction re-proofs are being re-run; §5.3's
  "adapter's real-binary PASS path waits on this" should be
  read with this correction, not with the retracted PASS.
- **Golden pins — BOTH ACTS STRUCK.** Decision 1 (14 film
  first-pins) landed as `916a109`: 11 mids from the `f021991`
  candidates plus 3 in-window mids from the post-re-chunk
  render at `aadc193` (s07 f969 `7f8e1555`, s08 f1119
  `3e8e703f`, s11 f1593 `7bed0da5`). Decision 2 (ab1-composer
  re-pin) landed separately as `166873c`: `d7bd71a0` →
  `60c32c1a`, the fresh integration-head render (23.98 dB vs
  Chrome), not the `c00897d`-era candidate. The golden pack
  verifies **18 OK / 0 STALE / 0 MISSING**
  (`tools/golden/golden-hashes.json`). The s28 pin was not
  part of either act and keeps its standing recommendation
  against re-pinning.
- **Re-render at the post-`0f6c033` head — CLOSED.** The full
  re-render at `f021991` completed: film A/B mean 23.02 dB,
  min 19.36 (60 pairs); proof MP4 `onoda-native-f021991.mp4`
  (all colour tags bt709 after `6ca97fc`; −14.00 LUFS).
- **s08/s08b/s09 broad-delta — ADDRESSED AT SOURCE.**
  `1fc6fab` (atmosphere inheritance; see Addendum 1) plus the
  `f021991` re-measure above.
- **Determinism formal PASS — CI-lavapipe leg DISCHARGED.**
  The determinism pack ran in CI (workflow added in Batch 1):
  run `43617370962`, completed/success — that job passes only
  on exit 0 with the pack's `DETERMINISM: PASS` line present
  (contract evidence E6). The PC RADV leg is still owed.
- **Lead's README disposition — CLOSED.** `838ea7c` filled the
  integration README proof section and closed B1/C4/C5.
- **Still open, unchanged:** the fog-chart PSNR leg
  (reference-side ×π re-capture), PC RADV evidence, and the
  skill adoption decision (user).

Wave 2 also landed, outside §5.3's frame: the professional
CLI (`d7bebdd`: `-h`/`--help`, `-V`/`--version`, panic-free
errors, Cargo 1.0.0); the structured-errors migration —
step 1 (`ba88596`) and step 2 (`edb058f`, delimiter fix
`aa92cee`: gpucore's adapter/device/readback paths return
`TaleResult`, exit 3, the documented no-adapter message
preserved verbatim); registration of the wave modules in the
crate (`2328e6f`, test fix `38930fa`); PaintBudget + RunLog
wiring into the film driver (`86f0c81`, discharging PERF §9
items 1–3); a `paint_budget_mb` dispatch input on render.yml
(`fbf37c0`); the gpucore migration **complete, 5 of 5 sites**
(`df89ad6` → `aadc193`, which also landed the generic
`film.json` entry, F1); the F2 first landing (`a2ec80c`: the
film driver's two silent asset fallbacks are now loud
errors); and the scene-format §15 fold-in with the loader
reservation guard (`4dd6530`). The wave record is
`docs/integration/WAVE2-RECORD.md`.
```

## 2. docs/ab/film/README.md — five lines, refreshed

File read in full today: all five stale texts stand verbatim
at the lines W38 cited. Four of W38's replacements stand;
§2.4's does not (its tail still called the pins outstanding).

### 2.1 Header status (lines 3–5) — replacement, tail refreshed

```markdown
**Status: MEASURED — film A/B filled in 2026-11-14 (W77);
telemetry verdict, golden staging, and MP4 evidence have
since landed, and the golden pins have since been struck
(see the Results section; the package's final film state is
the `f021991` re-render, A/B mean 23.02 dB).**
```

### 2.2 Step-1 comment (line 126) — W38 replacement verbatim

```
   # exit 0 = CHECK OK, 1 = CHECK FAILED, 3 = could not run;
   # the printed verdict line remains authoritative
```

### 2.3 Telemetry verdict (lines 174–175) — W38 replacement verbatim

```markdown
- Telemetry verdict: recorded in `TELEMETRY-W85.md` for head
  `381ff47` (CHECK FAILED on captions only; waiver ruled in
  `9790553`), then CLOSED BY DATA in Wave 2 — after the
  66 → 73 chunk re-chunk (`3a36f1f`) and driver regeneration,
  the re-run prints `FILM TELEMETRY: CHECK OK`, exit 0
  (`5e90d7e`)
```

### 2.4 Golden verify (lines 210–213) — replacement refreshed (pins struck)

W38's replacement ended "…the pins themselves are still
outstanding human acts" — false since `916a109`.

```markdown
- Golden verify (14 film mids): staging DONE and pins
  STRUCK — mids staged hash-verified in `docs/golden/frames/`
  (`381ff47` set), re-staged from the `f021991` frames in
  `docs/golden/frames-f021991/` (W03, `3a36f1f`), and pinned
  in Golden Act 1 (`916a109`; the 3 in-window mids — frames
  969 / 1119 / 1593 — come from the post-re-chunk render at
  `aadc193`, so their struck hashes differ from the staged
  table in `RESULTS.md`). Pack verifies 18 OK / 0 STALE /
  0 MISSING.
```

### 2.5 MP4 evidence (lines 214–215) — W38 replacement verbatim

```markdown
- MP4 evidence: see `MP4-EVIDENCE.md` — DONE (1,811 frames
  counted; mux −14.00 LUFS vs mix −13.99; the final proof
  MP4 from the `f021991` frames carries all-bt709 colour
  tags after the finish.sh fix `6ca97fc`)
```

## 3. docs/ab/film/RESULTS.md — discharge note, refreshed

File still ends at line 260 with the original "Still pending"
list (lines 244–260) untouched; no discharge note exists.
Per-bullet state is as W38 found it except the golden bullet.
Append at EOF (after line 260):

```markdown

---

## Discharge note — 2026-11-15 (Wave 2; via DOCFIX2-W67)

The "Still pending" list above was true when written (head
`381ff47`). Its state now:

- **FILM TELEMETRY verdict line — RECORDED.** The verdict for
  the `381ff47` frames is in `TELEMETRY-W85.md` (CHECK FAILED
  on captions only; ACCEPT WITH WAIVER ruled in `9790553`).
  In Wave 2 the caption item was closed by data instead: the
  66 → 73 chunk re-chunk (`3a36f1f`) plus driver regeneration
  re-ran the gate to `FILM TELEMETRY: CHECK OK`, exit 0
  (`5e90d7e`).
- **Golden staging + verify — DONE, and pins STRUCK.** The
  14 mids were staged hash-verified in `docs/golden/frames/`;
  candidates from the later `f021991` re-render were staged
  in `docs/golden/frames-f021991/` (`3a36f1f`); Golden Act 1
  (`916a109`) then struck the pins — 11 from those candidates
  and 3 in-window mids (s07 f969, s08 f1119, s11 f1593) from
  the post-re-chunk render at `aadc193`, whose struck hashes
  (`7f8e1555`, `3e8e703f`, `7bed0da5`) therefore differ from
  the staged table above for those segments. Pack verifies
  18 OK / 0 STALE / 0 MISSING.
- **MP4 finish + `MP4-EVIDENCE.md` — DONE.** The evidence file
  beside this one is filled (1,811 frames counted; mux
  −14.00 LUFS against the mix's −13.99); the final proof MP4
  was re-made from the `f021991` frames with all-bt709
  colour tags after `6ca97fc`.
- **PC RADV evidence — STILL NONE.** Unchanged; every native
  number in this package remains CI lavapipe.
```

## 4. shaders/fog/FOG.md — §8 closure note, STILL-VALID verbatim

Verified today: the W74 note (lines 370–384) still headlines
"still unwired"; no WIRED note exists (grep: zero hits).
§8 runs lines 359–385, §9 begins at line 386 — W38's
insertion point (after line 385) is unchanged. Apply W38 §4's
block exactly as written there.

## 5. README.md (root) — two W38 edits + one new

Verified today: `grep -n -- "--help\|--version" README.md`
still returns **zero hits**. Nothing in this file has been
touched since `3c2c129`.

### 5.1 Build & run — CLI pointer (insert after line 58)

The section moved up since W38: `## Build & run` is now at
line 53, its code block at lines 55–58. Insert W38 §5.1's
paragraph (beginning "Run `./target/release/talerender
--help`…") immediately after line 58, before the
"Requirements:" paragraph (line 60).

### 5.2 Status table, Production integration row — caption clause (line 30)

The row moved from line 26 to line 30; the stale clause is
verbatim: "…telemetry verdict recorded with the caption
waiver ruling; finished MP4 proof delivered." Apply W38
§5.2's replacement clause exactly as written there.

### 5.3 NEW — "Remaining, by design" paragraph (lines 38–41)

Not covered by W38. Current text:

> Remaining, by design: GPU proof on the target PC (RADV —
> every number so far is CI lavapipe), the golden-frame pins
> (deliberate human acts), and the decision on adopting the
> native backend into the shipped skill (a verified work
> copy exists; Chrome remains the default).

The pins clause is WRONG-NOW: both acts were struck
(`916a109`, `166873c`; pack 18 OK / 0 STALE / 0 MISSING).
Replacement for the pins clause only:

> …GPU proof on the target PC (RADV — every number so far
> is CI lavapipe), and the decision on adopting the native
> backend into the shipped skill (a verified work copy
> exists; Chrome remains the default). The golden-frame pins
> were struck in Wave 2 (`916a109`, `166873c`).

## 6. Runbooks' `--help` claims — APPLIED, no action

Re-verified by reading the documents today (not the commit):
USER-PC.md lines 7–13 carry the `-h`/`--help` +
`-V`/`--version` text with the dated 2026-11-14 correction;
PERF.md lines 24–26 carry "The binary has `-h`/`--help` and
`-V`/`--version` (added after v1.0.0)…". Both were closed by
`d7bebdd` itself. W38's CLOSED verdict stands.

## 7. docs/runbook/PERF.md — §9 discharges + line edits

The file is byte-identical in every region W38 cited (no
commit has touched it since before `df89ad6`): §9 items at
lines 333/341/348/352/356, preamble at 21–22, §4 figure at
195, §7 rows at 294/298.

### 7.1–7.4 — STILL-VALID verbatim

Apply W38 §7.1's §9 discharge note (append after item 5,
line ~362), §7.2's preamble replacement (lines 21–22),
§7.3's Telemetry-FAILED exit cell (line 298: exit **0** →
**1**), and §7.4's two replacements (line 195; OOM-row cause
clause, line 294) exactly as written there. Note the lead's
record independently flags the §7.2 item as owed
(`WAVE2-RECORD.md`, "PERF.md §1 dated correction owed", from
W02 §3d) — same fix, two sources.

### 7.5 NEW — §9 item 5 (line 356): designed, not implemented

W38's table said "STILL OPEN — no text change". Superseded:
Batch 6's W56 produced the design
(`docs/integration/VRAM-TELEMETRY-W56.md`, landed in
`3a3a427`): the blind spot's renderer-allocation layer is
exactly accountable by formula — film lane **210.7 MiB
steady / 218.7 MiB peak** accounted — with a companion
prototype (`tools/perf/vram_report.py`). No `src/` change
has landed; the true device-memory layer stays unknowable
from wgpu 0.20 (W56 §5). Append after the §7.1 discharge
note, or directly after item 5:

```markdown
**Item 5 status (2026-11-15, Wave 2).** The blind spot now
has a design: `docs/integration/VRAM-TELEMETRY-W56.md`
accounts the renderer-allocation layer by byte formula
(film lane 210.7 MiB steady / 218.7 MiB peak, including one
frame readback in flight) and specifies the adapter-info
extension. Status: **designed, not implemented** — no binary
output changes yet, and the driver-internal layer remains
unknowable from wgpu 0.20; `radeontop` on the PC remains
the only device-side observation.
```

## 9. The interval's new items, as a delta list for the lead

- **N1 — Pins struck.** `916a109` (Act 1, 14 film) +
  `166873c` (Act 2, ab1 `d7bd71a0` → `60c32c1a`, fresh head
  render, 23.98 dB vs Chrome — the packet's `c00897d`
  candidate was deliberately not used). Verified first-hand
  in the registry: `tools/golden/golden-hashes.json` carries
  `ab1-composer: 60c32c1a…` and `film-s07: 7f8e1555…` today.
  Every W38 text saying "pins open / outstanding / not
  struck" is superseded: FINAL-REPORT addendum (§1), film
  README (§2.4), RESULTS note (§3), root README (§5.3).
- **N2 — Adapter re-proof correction.** `b6ee9fb` retracts
  the `11a7a2e` FAILED-direction PASS: the run was SIGKILLed
  (OOM under contention; log ends ~sample 160) and its
  fixture mixed 66-chunk data with 73-era strips — on that
  fixture even the old binary prints CHECK OK. OK direction
  PASS stands. Corrected fixture rebuilt from
  `git archive aa10b13 film/`; re-proofs in flight (working
  tree holds `tools/ci/fixtures/adapter_reproof/`,
  uncommitted at audit time). Consequence for this pack:
  FINAL-REPORT's addendum must carry the correction (§1
  block does), and no §5.3-derived text may repeat the
  retracted PASS.
- **N3 — PERF §9 item 5 designed.** See §7.5.
- Also landed in the interval and reflected in §1's block:
  errors step 2 (`edb058f` + `aa92cee`), F2 loud fallbacks
  (`a2ec80c` — first landing only; the manifest-isation
  remains staged in W53's pack), §15 fold-in + reservation
  guard (`4dd6530`).

## 10. Application order (fixes pass)

1. §1 — FINAL-REPORT.md: append the refreshed Addendum 2.
2. §3 — RESULTS.md: append the refreshed discharge note.
3. §4 — FOG.md: insert W38 §4's closure note after line 385.
4. §7.1 — PERF.md §9 discharge note; then §7.5 item-5 note;
   then line edits §7.2, §7.3, §7.4.
5. §2 — film README: the five replacements (use §2.4 here,
   not W38's).
6. §5 — root README: §5.1 paragraph, §5.2 clause, §5.3
   clause.
7. §6 — no action (applied, verified).

## 11. Verification greps (after applying; expected noted)

```bash
# appended notes exist exactly once each
grep -c "Addendum 2" docs/integration/FINAL-REPORT.md          # = 1
grep -c "Discharge note — 2026-11-15" docs/ab/film/RESULTS.md  # = 1
grep -c "WIRED" shaders/fog/FOG.md                             # ≥ 1

# the retracted claim must NOT have propagated
grep -n "FAILED-direction re-proof passed" docs/integration/FINAL-REPORT.md  # no hits
grep -n "CORRECTED RECORD" docs/integration/FINAL-REPORT.md    # 1 hit (in Addendum 2)

# pins read as struck everywhere this pack touched
grep -n "no pin has been struck\|pins themselves are still outstanding" \
  docs/integration/FINAL-REPORT.md docs/ab/film/README.md docs/ab/film/RESULTS.md  # no hits
grep -n "golden-frame pins (deliberate" README.md              # no hits

# root README now names the CLI
grep -n -- "--help\|--version" README.md                       # ≥ 2 hits

# PERF corrections landed; item 5 carries its status note
grep -n "exits 0 on pass, fail, and error alike" docs/runbook/PERF.md  # no hits
grep -n "Telemetry FAILED.*exit \*\*0\*\*" docs/runbook/PERF.md        # no hits
grep -n "designed, not implemented" docs/runbook/PERF.md               # 1 hit

# untouched-history controls (dated records keep their words)
grep -n "still pending" docs/ab/film/RESULTS.md                # heading intact
grep -n "still unwired" shaders/fog/FOG.md                     # W74 note intact
grep -n "Dated correction, 2026-11-14" docs/runbook/USER-PC.md # 1 hit (control, §6)
```

**What this pack deliberately did not do:** no §5.3, W74-note,
or RESULTS-bullet text is rewritten (dated records keep their
words; fixes ride as addenda, per the W08 convention); W38's
two WRONG-NOW texts are replaced here, not silently — their
verdicts and evidence are in the master table; FIXTURES.md and
RENDER-READINESS.md items remain their owners', as in W38.
