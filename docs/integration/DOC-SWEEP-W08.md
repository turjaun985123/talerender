# Doc sweep — W08 (wave 2), 2026-11-14

**Scope.** (1) Refresh of `docs/risk-register.md`: every risk
re-verified against the repo record. (2) A sweep of `docs/` +
`tools/` for stale status claims (search phrases: `not started`,
`pending`, `TBD`, `unverified`, `no --help`, plus targeted reads
of the live docs). (3) One authorised out-of-register fix:
`trap 1` in `scenes/fixtures/FIXTURES.md`.

**Rules followed.** Dated corrections only — no silent rewrites
of any historical text. Nothing in `src/` touched. Lead-owned
docs (`docs/scene-format.md`, `docs/integration/README.md`,
`docs/integration/INVENTORY.md`) were read but not edited; their
items are listed for the owner below. Verdicts: **still-true** /
**corrected** (by this seat, or by an earlier commit the sweep
verified) / **listed-for-owner** (stale, outside this seat's
edit grant — file, truth, and owner named) / **frozen record**
(true at writing; the file's function is to be a dated record).

---

## 1. Risk register refresh (docs/risk-register.md)

The M0 header claimed "no mitigations are complete yet". Each
risk now carries a dated `Refresh 2026-11-14` block; the M0
text is preserved verbatim.

| Risk | Verdict | Closing / status evidence |
|---|---|---|
| R1 Bokeh DoF | **CLOSED** | M3 gate PASS, commit `b0de44e`; docs/ab/dof-chart/README.md (≤0.05 px at 10/10 depths, 47.38 dB) |
| R2 Ink strokes | **CLOSED** | M4 battery PASS 46/46, commit `819ee16`; docs/ab/stroke-battery/README.md (47.88 dB) |
| R3 PMREM / IBL | **MITIGATED** (form changed: captured CubeUV data, not a native generator — gaps.md item 1) | M5 PASS, commit `79aff03`; docs/ab/composer/README.md + gaps.md (region ratios 1.002–1.048) |
| R4 QA gate deletion | **CLOSED** | M6 exact agreement, commit `9a8cc79`; docs/ab/telemetry/README.md; film gate ran: commit `df79162`, docs/ab/film/TELEMETRY-W85.md |
| R5 Upload cost / facade memory | **PARTIALLY MITIGATED — still open** | Cap landed: `1475b66` (160-entry FIFO), `e5860b4` (env override); key exactness: `cabcc30`. NOT landed: byte-denominated budget (no such mechanism in src/), dirty-region/atlas uploads, facade streaming (ADR's "M7" never existed) |
| R6 JPEG generation drift | **CLOSED by decision** | Contract §5 RESOLVED, commit `801f85c` (option (b), clean frames); docs/ab/telemetry/jpeg-decision.md (42.31 dB, 94.5 % of pixels) |
| R7 PCFSoft shadows | **MITIGATED, measured residual** | M5 PASS, `79aff03`; per-shot shadow data (spec §14). Residual: trunk sides 6–9 % bright, docs/ab/composer/gaps.md item 3. The standalone per-box chart package was never produced — stated in the register, not papered over |
| R8 Composer chain | **CLOSED** | M5 PASS with stage ablation 21.36 → 21.42 → 23.850 dB, `79aff03`; later fog stage gated the same way, `69952ab` |
| R9 Code-painted textures | **MITIGATED, one residual** | Native painting decided at M2 (`2a7ff89`); 13/13 generators armed (`c74cf0c`, census at `743d248`). Residual: sign border ink ≤3 px class, painters/picture-sign.md, declared gaps item 2 |
| R10 Software-only verification | **STILL OPEN** | All numbers still CI/VM lavapipe; PERF tables blank by design; discharge = docs/runbook/USER-PC.md on the user's PC. §6.1 renderer-identity line verified in force (RUNBOOK-VERIFY-W95.md) |
| Watchlist: points/stars | **still open** | No primitive implemented; no corpus scene names one; M2 decision point passed without a forcing case |
| Watchlist: colour working space | **CLOSED** | The predicted failure occurred (byte multiply, 25–40 LSB dark) and was root-fixed: `c00897d`; re-pin `d8fbe83` |
| Watchlist: grain hash | **still open, bounded** | ±0.006 bound, docs/ab/composer/gaps.md item 2; film grain time pinned 0 (`5e5b0a0`); RADV leg under R10 |
| Watchlist: held-prop closures | **CLOSED** | Named attachment kinds on the character node, loader consumes them: `4c06a1e`; residual angle note in integration README limits |

**Genuinely still open after this refresh:** R5 (in part), R10,
grain (RADV leg), points/stars, RADV fp16 rounding (under R10),
caption widths measured with PIL not Skia (a standing
measurement caveat — docs/integration/README.md, "Honest
remaining limits"; verified within ~1 % on the widest chunk,
TELEMETRY-W85.md). None of these were closed by assertion.

---

## 2. FIXTURES.md (scenes/fixtures/) — one fix applied, four items listed

**Corrected by this seat** (dated correction appended in the
fx-grade section; original text preserved):

- **Trap 1 (grade `saturation`/`exposure`)** — stale on both
  halves. Truth: (a) all 12 files that spelled `saturation`
  were renamed to `sat`, values byte-unchanged (commit
  `8af40eb`, seat W81; validator 7/7); the "11 of 14" count was
  itself superseded by the 12-of-14 census in
  docs/integration/LOADER-ACCEPTANCE-W71.md; zero `saturation`
  keys remain in scenes/film/ (reproduced in
  docs/integration/RED-TEAM-W93.md). (b) `exposure` is now
  consumed — wired to the composer OutputPass (commit
  `c74cf0c`; spec addendum `5b0889d`); canonical keys
  `sat`/`warm`/`exposure`/`time` (spec aligned, `ede0e92`).
  `saturation` remains a non-key (no alias) — that half of the
  loader claim survives; the data half does not.

**Listed for owner** (stale, outside this seat's edit grant;
owner: the fixtures pack's maintainer / integrator — the file
is not lead-owned):

- **Loader-facts table, `fog` row** — "never read by
  scenefile.rs … has not landed": **false since `69952ab`**.
  The loader parses the v2 fog block and applies three's
  fog_fragment pre-bokeh via the packed-depth prepass;
  verified ≤1 code at every fog-chart panel (`5b0889d`).
- **fx-fog section** — "the `fog` block is ignored, wholly" and
  the §8 summary "no shader change needed": both false. The
  wiring added a new shader (`shader_fog.wgsl`); the "no shader
  change" premise was already corrected in `69952ab`'s own
  commit message and in docs/integration/FOG-STATUS-W74.md
  (closure addendum, seat W89).
- **Load-verdicts header + fx-textures census + census
  summary** — "fx-textures fails by design", "7 of 14 resolve;
  the file fails", "6 generator arms … awaiting the lead's
  wiring": **false since `c74cf0c`**. All 13 generators are
  armed (census: docs/integration/LOADER-ACCEPTANCE-W71.md),
  and fx-textures renders — byte-identical between this VM and
  CI (docs/integration/LOADER-EVIDENCE-W86.md).
- The file's framing ("the loader as it actually behaves
  today") makes these three items load-bearing; they should be
  corrected with the same dated-note convention used for
  trap 1.

Everything else spot-checked in FIXTURES.md (fx-character,
fx-attachment, fx-channels, fx-lights, trap 2) agrees with the
record as of this sweep.

---

## 3. Sweep of docs/ + tools/ — claim by claim

### Stale → listed for owner

1. **docs/ab/film/README.md** — header: "telemetry verdict,
   golden staging, and MP4 evidence still pending"; §Results:
   telemetry `_pending_`, golden `_pending_ staging`, MP4
   `_pending_ (no finished MP4 exists yet)`. **All three
   stale.** Truth: telemetry ran (verdict CHECK FAILED on
   captions only — docs/ab/film/TELEMETRY-W85.md; disposition
   since ruled ACCEPT WITH WAIVER, commit `9790553`); the 14
   mids are staged (docs/golden/frames/, hash-verified); the
   MP4 was finished and evidenced (docs/ab/film/MP4-EVIDENCE.md)
   and later superseded by the final-head film (integration
   record, commit `838ea7c`). Owner: proof-package maintainer /
   integrator.
2. **docs/ab/film/RESULTS.md §"Still pending for this
   package"** — three of four bullets stale (telemetry line,
   golden staging, MP4 finish — same evidence as item 1). The
   fourth bullet (PC RADV evidence: none here) is **still
   true**. Owner: same as item 1.
3. **docs/integration/FINAL-REPORT.md §5.3** — the open list
   predates the lead's final landings: F1 (closed by `6ca97fc`),
   the post-`0f6c033` re-render (done at `f021991`; final film
   A/B mean 23.02 dB), the caption disposition (ruled,
   `9790553`), the s08/s08b/s09 explanation (addressed at the
   source, `1fc6fab`), and the determinism item (CI-lavapipe
   leg proven by the lead's final-head probes) all still read
   open. The file's own orchestrator addendum (end of file)
   already notes the landings; §5.3 itself should get the same
   dated treatment. The remaining §5.3 items (golden pins ×2,
   RADV proof, skill adoption) are **still true/open**.
   Owner: orchestrator/integrator. (This seat did not edit it:
   the fix is an addendum to a closing record, which belongs
   to its author.)
4. **shaders/fog/FOG.md §8 status note (W74)** — "still
   unwired": **false since `69952ab`**. FOG-STATUS-W74.md
   carries the closure addendum; FOG.md's own note was never
   updated. Owner: fog package maintainer.
5. **docs/integration/RENDER-READINESS.md** (lines ~443, ~456,
   ~574) — references the integration README's `TBD-PROOF` line
   as still standing. **Stale:** the proof section was filled
   and B1/C4/C5 closed in commit `838ea7c`. Owner: integrator
   (the file is a wave-1 working checklist; archiving it with
   a dated header would also discharge this).
6. **README.md (root), Build & run section** — omission, not a
   false claim: it predates the professional CLI (commit
   `d7bebdd`) and never mentions `-h`/`--help` or
   `-V`/`--version`. The Status section itself was rewritten
   at `3c2c129` and its remaining-items list (RADV proof,
   golden pins, adoption) is **still true**. Owner:
   orchestrator.

### Checked → still true (no action)

7. **tools/determinism/README.md + docs/integration/
   DETERMINISM-RUN-W96.md** — the `TBD-PROOF` framing stands in
   the formal sense: no pack-recorded `DETERMINISM: PASS`
   exists per renderer of record. Narrowed but not discharged
   by the lead's final-head probes (frames 1195–1204 10/10 and
   795–814 20/20 byte-identical at `f021991`); the RADV leg is
   untouched. The pack's own discharge section describes
   exactly this state.
8. **docs/golden/repin-packets/** — stroke-toon **PENDING**
   (no post-fix candidate) and dof-chart **PENDING-not-
   required**: both still true. Pins remain human acts
   (FINAL-REPORT §5.3 concurs).
9. **tools/CI-WORKFLOW.md** — "UNVERIFIED" for validator check
   3's source literals in CI: still true **by design** (the
   W83 source-absent mode, commit `8af40eb`; counted, listed,
   never silently passed; negative control documented in the
   same file).
10. **docs/runbook/USER-PC.md** — "RADV fp16 rounding is
    unverified (carried from M3)": still true (see R10). The
    file's old "no `--help`" claim was **already corrected**
    by commit `d7bebdd` with a dated note — verified, not
    re-fixed here.
11. **docs/runbook/PERF.md** — §9 asks (telemetry exit codes,
    cache statistics, progress elapsed) still open; repeated
    as open in the integration record. Its "no `--help`" lines
    were likewise corrected in `d7bebdd`.
12. **docs/scene-format.md §12** (lead-owned) — "Known limits
    of v1" is scoped to v1 by its own heading and remains true
    of v1; production status lives in §14 + the §14.5 addendum
    (`5b0889d`). No stale claim found.
13. **docs/integration/README.md** (lead-owned) — post-
    `838ea7c` the proof section and limits list agree with the
    record (loader fog consumption stated correctly; RADV fp16
    unverified — still true; caption PIL measurement — still
    true). No stale claim found in the swept phrases.
14. **docs/compatibility-contract.md §5** — status reads
    RESOLVED with the decision and its evidence; current.

### Frozen records (true at writing; not live claims)

15. **docs/ADR-001 deferred table** — a decision record; item
    statuses today: composer parity → closed at M5
    (`79aff03`); facade streaming → never implemented (see R5);
    star/points → decision never forced (see watchlist);
    telemetry channel → closed at M6 (`9a8cc79`). Tracked via
    the risk register refresh; decision records are not
    rewritten.
16. **docs/ab/composer/gaps.md item 6** ("scene format v1
    cannot express the light rig") — true of v1 as scoped;
    superseded for production by format v2 §14 (`5e5b0a0`,
    `4c06a1e`).
17. **docs/ab/dof-chart/gaps.md** — the HalfFloat storage note
    still describes the shipped default accurately
    (Rgba8Unorm; fp16 leg open under R10).
18. **docs/audit/** (M0 evidence), **docs/integration/AUDIT-
    W42 / REVIEW-W50 / GATE-W90 / GATE-W99 / CLOSEOUT-DRAFT-
    W98 / PUSH-CHECKLIST-W100 / WAVE-CLOSE / WAVE-RECORD** —
    dated seat/milestone records. Several describe states
    since superseded; that is their function, and the files
    that needed closure notes already carry them
    (FOG-STATUS-W74 via W89; FINAL-REPORT via its addendum).
    No action.
19. **docs/ab/telemetry/project/drive.mjs,
    tools/fog-chart/chrome/project/tools/capture.mjs** —
    phrase hits are inside the *reference* Chrome project's
    own harness files, kept as evidence. Not TaleRender status
    claims; out of scope.

---

## Method note

Every "closed" above names the gate package or commit that
closed it; every "still open" names why. Where the record and
a document disagreed, the record (git log + gate packages) won
and the document is listed — except the two files this seat
was charged with (risk-register.md, FIXTURES.md trap 1), which
now carry the dated corrections themselves.
