<!--
DELIVERY NOTE — Chrome Replacement, Batch 2, Seat 8 (CR-B2-08):
P1 retrofit tooling for pre-adoption projects.

Files created (this seat, NEW files only; nothing existing edited
in either tree, nothing committed):
  ~/workspace/chrome-replacement/batch2/B2-08/retrofit.py
  ~/workspace/chrome-replacement/batch2/B2-08/README.md
  ~/workspace/chrome-replacement/batch2/B2-08/onoda-retrofit-report.{md,json}
  ~/workspace/chrome-replacement/batch2/B2-08/onoda-copy/   (fixture copy, retrofitted)
  ~/workspace/chrome-replacement/batch2/B2-08/evidence/     (raw outputs, hash snapshots, backups)
  docs/integration/CR-B2-08-RETROFIT.md                     (this note)

Contracts honoured:
  - CHROME-REPLACEMENT-CONTRACTS.md §1 row 17 / WP9 proposal P1:
    pre-adoption projects get a retrofit path (adapter + bridge).
  - Batch 1 outputs consumed, not rebuilt: build_bridge.py
    (CR-B1-06), package_bridge.py, filmdata_validate.mjs (P4),
    export_project_v2.mjs skeleton (CR-B1-07), the wired
    capture_native.mjs + native_modes/ set (L6).
  - Laws: verify against code; no fabricated results (every claim
    below is a command output or a hash, with the log named);
    the original fixture project was never mutated (§7); no
    quality change to any project asset.
-->

# CR-B2-08 — P1 retrofit tooling

## 1. Verdict

**Row 17 (existing projects' tooling) lands as REPLACE-NATIVE
tooling, proven on the canonical fixture.** `retrofit.py` takes a
pre-adoption project — Chrome-era `tools/capture.mjs` copy, stale
or missing bridge, no native tooling — and makes it native-ready:
re-bridged to the current film-data generation, shipped native
tool set installed byte-for-byte, Chrome pair quarantined (moved,
never deleted), story/media assets provably untouched. On the
Onoda copy the retrofitted bridge reaches **`FILM TELEMETRY:
CHECK OK`** from the provisioned binary, and the shipped adapter's
own `check` mode against the retrofitted copy prints **`CHECK OK`,
exit 0**. No EXCEPTION is proposed for this row.

## 2. The tool

`retrofit.py` (stdlib-only Python 3; staged in the seat folder for
the lead to install — suggested home `scripts/retrofit.py` plus a
SKILL.md P1 section). Three verbs:

- **`audit`** — DETECT, read-only. Finding ids: `D-TOOL-LEGACY`
  (capture.mjs / capture.ts / perf_probe.mjs present, hash-compared
  against the template's copies so the generation is named),
  `D-TOOL-NATIVE` (each of the 11 shipped native files missing /
  stale / current, by hash), `D-TOOL-DRIFT` (mix.py — NOT-CHROME,
  flagged, never overwritten), `D-BRIDGE-MISSING`, `D-BRIDGE-SHA`
  (MANIFEST-SHA recomputation), `D-BRIDGE-F2` (calendarYears /
  calendarDark / sign presence), `D-BRIDGE-SHAPE` (legacy
  bookkeeping keys in film data), `D-BRIDGE-PLATES` (manifest vs
  legacy flat plates.json), `D-SCENE-DRIFT` (byte compare against
  the repo generation), `D-BINARY-ERA` + `D-BINARY-TRAP` (the
  provisioned binary is era-probed; a stale repo-checkout binary
  is named as a discovery trap), `D-FILMDATA` (the shipped P4
  `filmdata_validate.mjs` run through a temp harness), and
  `D-TELEMETRY` (optional probe — a stale bridge refuses in
  seconds; a passing one needs the full audit, so the probe
  reports NO-VERDICT-IN-WINDOW honestly instead of guessing).
  Exit 0 = native-ready, 3 = retrofit needed.
- **`apply`** — ACT, on the project dir it is given. Safety
  interlock: it **refuses the canonical fixture's original path**
  (exit 2) unless `--allow-original` is passed, so a rehearsal
  cannot mutate the original by accident (exercised, §5).
  Pipeline: (1) hash the protected trees (script.json, data/, src/,
  fonts/, vendor/, build/, index.html, sheet.html, package.json,
  tsconfig.json, review/, renders/); (2) select the bridge donor —
  the TaleRender repo **only** when it provably holds this film
  (manifest title matches case/punct-folded AND film durations
  agree exactly), else the project's own packaged scenes, else a
  named blocker (transcription is the manual first step; the tool
  never guesses); (3) generate the skeleton with
  `export_project_v2.mjs`, selecting the `talerender-skeleton/1`
  report **by its format field** (the exporter writes two reports;
  §6, finding F1); (4) re-bridge into staging with
  `build_bridge.py --no-telemetry` (the project's own
  script/lines/words are the timing source; the donor supplies the
  current scene generation, rasterised assets and the F2/fit
  carry-over donor); (5) run `package_bridge.py` into a second
  staging shell as the package step and **cross-check the two film
  data files semantically** (duration/fps/frames/lines/anchors/
  captions/F2) — a mismatch blocks installation; (6) carry the
  prior manifest's underivable provenance into the built manifest
  (display title, boundaryRule, per-segment filmLook, atmosphere
  and set where the builder left them empty, audio loudnessLUFS),
  each field recorded, MANIFEST-SHA regenerated; (7) back the old
  bridge up **outside** the project and install the built bridge;
  (8) install the 11-file native set byte-for-byte (hash-verified)
  and quarantine capture.mjs / capture.ts / perf_probe.mjs under
  `tools/legacy-chrome/` (content hashes proven unchanged);
  (9) re-hash the protected trees and FAIL if any byte moved.
- **`verify`** — PROVE: binary era + P4 validator + the binary's
  own `--film-telemetry` verdict on `<project>/native/film`
  (verdict line authoritative). For this seat's proof the
  telemetry leg was run as the identical direct command (same
  binary, same target) so its raw log could be kept verbatim;
  the earlier `verify` invocation is disclosed in §6 (F2).

Reports: per-project Markdown + JSON (`--report-md/--report-json`)
listing every finding by name, every action, and every blocker.

## 3. DETECT evidence — the fixture before retrofit

`retrofit.py audit onoda-copy --telemetry-probe 120` on the
pristine copy (log: seat `evidence/audit-before.txt`):
**RETROFIT NEEDED — 18 FAIL, 6 WARN, exit 3.** The named core:

- `tools/capture.mjs` (sha256 `3c2797fc3ba6…`) and `capture.ts`
  (`f5b6766ff28c…`) present and **older than the template's copies**
  (`72ef32e5b51c…` / `8bb3b3886e8e…`); all **11 native files
  missing** (capture_native.mjs, CAPTURE-NATIVE.md, cues_native.py,
  castsheet_native.py, 7× native_modes/).
- Bridge present but stale generation: `onoda.json` **lacks all
  three F2 fields**; captions 73/73 carry legacy `line`/`width`
  keys; lines 12/12 carry `i`/`text`; extra top-level `film` /
  `project` keys; `plates.json` is the **legacy flat shape**;
  packaged scenes s10/s11/s12 differ from the repo generation
  (s12 substantively: pre-retirement bow keys + no `clips` block).
  MANIFEST-SHA itself verifies (101 files) — the package is
  internally consistent, just two generations old.
- The strictness gap, live: the P4 validator **passes** the stale
  bridge (`FILMDATA_VALIDATE: PASS (73 captions, 17 anchors)` —
  data-driven, it does not demand F2), while the binary refuses it
  verbatim: `film telemetry failed: film data: missing
  calendarYears`, exit 3. DETECT therefore treats the binary, not
  the validator, as the bridge-generation authority.
- Environment: the repo-checkout binary `~/workspace/talerender/
  talerender` is stale (`unknown argument: --film-telemetry`,
  sha256 `3e955092d177…`) and sits on the adapter's discovery path;
  the tool pins the provisioned binary (`~/talerender-bin/
  talerender`, sha256 `b331f6704ebc8836…`, v1.0.1) explicitly.
- Negative fixtures: a synthetic unbridged project audits
  14 FAIL with `D-BRIDGE-MISSING` naming the absent manifest and
  film data (no Chrome suggestion anywhere in the output).

## 4. ACT evidence — the Onoda copy retrofit

Final run log: seat `evidence/apply.txt` (+ staging logs in
`evidence/staging/`). Donor = repo (title match, durations agree
exactly at 60.357 s). Skeleton: 14 segments, 8 owes items.
`build_bridge.py` exit 0, **VERIFICATION PASSED** (P4 leg; binary
leg deferred to the single post-install run, §5).
`package_bridge.py` exit 0, **21/21 checks**. Cross-check: built
vs packaged film data **semantically identical** on all nine
compared fields. Carry-over recorded field-by-field: display
title, boundaryRule, filmLook ×14, atmosphere ×5 (s04/s05a/s05b/
s08/s09 — §6, F3), audio.loudnessLUFS −14.0. Old bridge backed up
to `evidence/backup/native-pre-retrofit/` (outside the project);
built bridge installed (102 files). Native set: 11/11 hash OK.
Quarantine: capture.mjs / capture.ts moved to
`tools/legacy-chrome/`, hashes unchanged. **Protected assets: all
157 hashed files byte-identical before/after.**

Post-apply audit (`evidence/audit-after.txt`): **NATIVE-READY —
0 FAIL, 3 WARN**, exit 0. The WARNs: mix.py drift (by design);
scene bytes vs repo (semantic diff,
`evidence/scene-semantic-diff.txt`, shows the only deltas are the
builder's R2/R5 normalisations — `shot.events: []` made explicit,
`shot.cues: []` where absent, s05a's legacy `{at,name,source}` →
`{t,type}` with provenance recorded in the manifest bridge block);
the stale-repo-binary trap (environmental).

Bridge vs the repo's current generation
(`evidence/bridge-vs-repo.txt`): built `film/onoda.json`
**identical** to `~/workspace/talerender/film/onoda.json`; all 14
manifest segments **field-identical** to the repo manifest;
assets 85/85 files except `plates.json` (plate ordering only —
same names/files/origins); remaining manifest deltas are the
`project`/`audio.projectPath` values (correctly the copy's), the
builder's `bridge` provenance block, the audio durationS
(60.457 wave-measured vs 60.46 packaged rounding), and one
frameRule prose word ("the segment ranges" vs "the 14 ranges").

## 5. PROVE evidence — verdicts on the retrofitted copy

- **Binary telemetry** (log: `evidence/telemetry-v2.log`):
  `~/talerender-bin/talerender --film-telemetry
  onoda-copy/native/film`, started 2026-11-15T17:25:37Z, complete
  17:46:33Z (~21 min wall on this shared-CPU VM, contended).
  Output tail: `CANVAS OVERFLOW: none` · `CONTINUITY: clean
  (5 named characters tracked)` · `CAPTION AUDIT: 73 chunks,
  0/12 lines authored, min fit 1.00, max rendered width 844 px` ·
  **`FILM TELEMETRY: CHECK OK`**. (The raw binary classifies the
  captions estimate-grade/auto-chunked — its known classification
  of scene-carried captions, cf. CR-B1-06 §5; the adapter corrects
  the authored count from script.json, next bullet. The
  composition warnings in the log are the film's established
  reference warnings, not retrofit deltas — the film data is
  byte-identical to the repo generation whose telemetry is
  CHECK OK at HEAD.)
- **Adapter check mode** (log: `evidence/adapter-check.log`):
  inside the retrofitted copy, `RENDERER=native TALERENDER_BIN=
  ~/talerender-bin/talerender node tools/capture_native.mjs
  check`, 17:48:32Z → 17:59:39Z. The adapter accepted the bridge
  (banner + film header), printed the 14-shot table with set/
  atmosphere for every shot, `CAPTION AUDIT: 73 chunks, 12/12
  lines authored`, and **`CHECK OK` with exit 0** — the shipped
  tooling running against the retrofitted bridge, end to end,
  CPU-side.
- **Cues parity**: `python3 tools/cues_native.py` in the copy
  reproduces the project's Chrome-produced `data/cues.json`
  **byte-identical** (sha256 `efbeb79e01ead1df…` on both sides) —
  the retrofitted bridge feeds the native cues route exactly.
- **Interlock**: `retrofit.py apply` against the original
  fixture path exits 2 with the refusal text, before any write.

## 6. Findings from building the tool

- **F1 — exporter writes two reports.** `export_project_v2.mjs
  <project> <out>` writes `<out>` as `talerender-project/1` and
  the skeleton as the `<stem>.skeleton.json` sidecar
  (`talerender-skeleton/1`). The first apply run fed the wrong
  file to build_bridge (`skeleton: format is
  'talerender-project/1', expected 'talerender-skeleton/1'`) and
  the tool's package fallback installed a valid bridge instead —
  the fallback working as designed, disclosed here rather than
  hidden. Fixed by selecting the skeleton **by format field**;
  the final run (§4) used the build path. Worth a line in the
  exporter's doc for the next consumer.
- **F2 — first verify run lost to a runtime restart.** A
  `retrofit.py verify` against the pre-carry-over build was
  killed ~10 min in by a VM/runtime restart (empty log, no
  verdict — nothing was claimed from it). The seat re-applied
  with the fixed tool to a fresh copy and obtained the §5
  verdicts on the final artifact via detached processes whose
  logs survive session drains (the CR-B1-06 precedent).
- **F3 — build_bridge atmosphere gap (repo-side, small).** The
  builder sources manifest `atmosphere` only from the skeleton
  or `shot.atmosphere`; segments whose scenes declare atmosphere
  at scene top level (Onoda s04/s05a/s05b/s08/s09) lose it in the
  manifest. The scene files themselves carry the atmosphere (the
  render/telemetry path is unaffected — §5 passes), and this
  tool's recorded carry-over closes the manifest gap from the
  project's own prior manifest. A builder-side fix (fall back to
  the scene's top-level `atmosphere` string) is a one-line
  candidate for the bridge's owner; not made here (new files
  only).
- **F4 — repo validator drift, reconfirmed on the retrofitted
  project.** `tools/integration/validate_native_project.py`
  against the retrofitted copy reports exactly the 3 phantom
  plates from CR-B1-06 §5 (`plate format.png / plates.png /
  version.png missing`) — it iterates the current plates.json's
  top-level keys as plate names. The bridge satisfies the binary
  and the P4 validator; this leg is the validator's own defect
  (B2-06 doctor seat's remit), reported, not worked around.

## 7. Guarantees and remainders

- **Original untouched, by construction + by measurement.** The
  tool only ever ran `apply` against the seat-folder copy (and
  refuses the original path by default). Measurement: full-tree
  sha256 snapshots of `~/faceless-shorts/the-soldier-who-didnt-
  know` before the seat's first action and after its last
  (`evidence/original-hashes-before.txt` /
  `original-hashes-after.txt`) — **all 262 files byte-identical**.
  The copy was itself verified byte-identical to the original
  before retrofit (`copy-hashes-before-retrofit.txt`; `cp -a`
  emitted directory-ownership warnings only, no content effect).
- **No quality change.** The retrofit writes only `native/`
  (bridge data, copied/rebuilt — never re-encoded media) and
  `tools/` (byte copies + quarantine moves). Protected trees are
  hash-pinned across apply. Final encode settings are untouched
  (they live in the adapter, which is installed byte-identical).
- **Remainders, named:**
  1. GPU render modes (`frames` / `sheet` / `video` / castsheet
     rendering) were **not** exercised: this VM has no Vulkan ICD
     at all (Batch 1 close note), so render legs are env-blocked
     here. Telemetry + adapter check are CPU-only and pass; the
     render confirmation rides the PC session / Batch 3, exactly
     as the contracts already state for the wave.
  2. `shot.events[]` is emitted empty by the builder (R5 gap,
     recorded in the built manifest's bridge block). For Onoda
     this is substantively correct for authored cues; the s12
     clip data lives in the scene `clips` block the evaluator
     consumes, and telemetry passes with it.
  3. The generic donor path (project's own scenes, no repo-held
     film) is implemented and fails loudly without a donor, but
     is **proven only on the Onoda fixture**; a second,
     non-repo-held fixture is the honest next proof (Batch 3
     hardening candidate).
  4. `tools/mix.py` version drift is flagged, not synced — a
     diffed manual sync is the operator's call, since mix.py may
     carry project tuning.
- **Lead integration asks:** install `retrofit.py` into the skill
  (suggested `scripts/retrofit.py`) with a SKILL.md P1 section
  naming the audit→apply→verify flow and the copy-first rehearsal
  discipline; Batch 3's Onoda re-run (§4 acceptance item 3) can
  then run it against the real project — with `--allow-original`
  only after the pre-retrofit backup discipline this seat used
  (backup outside the project + protected-tree hash pin) is in
  place.
