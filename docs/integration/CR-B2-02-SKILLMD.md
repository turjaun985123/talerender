# CR-B2-02 — SKILL.md native rewrite (report note)

Seat B2-02, Chrome-replacement wave Batch 2. Deliverables (staged,
new files only — nothing edited in place, nothing committed):

- `~/workspace/chrome-replacement/batch2/B2-02/SKILL.md` — the full
  replacement SKILL.md for the lead to diff and apply.
- `~/workspace/chrome-replacement/batch2/B2-02/CHANGELOG-B2-02.md` —
  section-by-section change log (what changed, why, evidence).

Method: verified against code, not other docs — `capture_native.mjs`
as wired at Batch-1 integration close (L6), its `native_modes/`
modules, `cues_native.py`, post-Batch-1 `CAPTURE-NATIVE.md`, and the
Phase-0 contracts. A full `diff -u` against the shipped SKILL.md was
read end-to-end: every hunk is a capture/render hunk; story, voice,
music, mix, metadata, thumbnail doctrine, all gate thresholds and
caption floors, the dialogue honesty quote and the v6 animation map
are preserved verbatim. **No render proof is claimed** — this VM has
no Vulkan ICD; §0 of the rewrite states software Vulkan proves
correctness only and makes no speed claim.

## What the rewrite does (summary)

- §0: the ANGLE/GPU-flags paragraph is replaced by the native
  prerequisites (talerender binary via CI artifact + `TALERENDER_BIN`,
  discovery order, Vulkan/RADV with lavapipe correctness-only,
  ffmpeg) and an **era-pairing** section: binary + bridge data are
  era-paired; a stale binary fails closed (symptom `unknown
  argument: --film-telemetry`, exit 2, no verdict line = failed
  gate) and the remedy is the `era_hint.mjs` provisioning text,
  printed inline by the adapter.
- §1: layout + command block are native-only (cues via
  `python3 tools/cues_native.py`); the old "Native capture
  (optional…)" paragraph and its `RENDERER=native` selection story
  are replaced by a per-mode contract for check / frames / sheet /
  video / castsheet / cues-routing / preview-retired, exactly as the
  wired adapter behaves (incl. check exit 3, sheet audit-unavailable
  exit 1, castsheet skip-by-name, video `.render.json` sidecar,
  clean-PNG decision (B), native golden pack as the reference).
- §5: bridge step added inside step 4 (numbering preserved); cues
  extraction + clip-events sidecar corrected to `cues_native.py`'s
  actual behaviour; check step states the native exit codes.
- §3: one cell — client-review deliverable is now native sheet +
  frames + video (contracts row 7). §6: native sheet/castsheet/
  frames + skip-list instruction + re-bridge in the fix loop. §7:
  native master + sidecar; `--draft` named as never-a-master.
- Hard rules: speed rule re-pointed at Vulkan/RADV (threshold text
  unchanged); two rules added — **Native only** and **Era pairing
  fails closed**. No existing rule weakened.
- Version stamp held at **6.0** deliberately: the 7.0 bump lands
  with the lead's Batch-3 flip commit (contracts §4.6), prepared by
  B2-07 — not in this seat's staged file.
- Flow-order note: the charter's flow string lists
  "…video→sheet→castsheet→mix→metadata"; the document orders steps
  as the code requires — cues before mix (mix consumes cues.json),
  mix before video (video muxes `data/mix.wav`; without it the
  master is SILENT and says so). No step was dropped.

## Chrome-mention classification table

`grep -n -i -E 'chrome|chromium|puppeteer'` over the staged SKILL.md
returns exactly five lines. **Zero unclassified mentions.**

| # | Line(s) | Mention | Classification (inline in the document) |
|---|---|---|---|
| 1 | 20–22 | "captured through headless Chrome" | retired / legacy history — same sentence: "that backend is no longer part of the pipeline and is never a fallback" |
| 2 | 129–133 | "The Chrome/Chromium capture backend", `capture.mjs`, `capture.ts`, `CHROME`, `CHROME_FLAGS`, `SOFTWARE_GL`, ANGLE flags, `preview` mode | retired / legacy history — paragraph headed "**Retired (legacy history).**", predicate "is **retired**: none of it has any effect on the native adapter" |
| 3 | 558 | "the retired Chrome backend" | legacy history — same clause: "is **legacy history**, not a fallback" |

- `puppeteer`: zero mentions (none existed in v6.0 either).
- `capture.mjs` named only at line 130, inside the retired note; no
  command in the document invokes it.
- `RENDERER=native` invocations: zero. `RENDERER` appears once (§1)
  describing the adapter's exit-2 refusal guard — a provenance
  guard, not a selection.
- `three.js`: zero mentions (see gap G-6).

## Named gaps — what could not be rewritten truthfully

- **G-1 Adapter + CAPTURE-NATIVE.md still carry pre-flip text.**
  `capture_native.mjs` `findManifest()` still tells an unbridged
  project it "is captured with Chrome (tools/capture.mjs)" — the
  rewrite does **not** repeat that (it names the bridge step), so
  until B2-07's follow-through lands, the adapter's own error text
  contradicts SKILL.md. Likewise `CAPTURE-NATIVE.md`'s framing
  ("Selection — explicit, never auto", "capture.mjs (Chrome) is the
  default backend", the W65 provenance block) is stale even though
  its mode/exit-code substance is post-Batch-1 accurate; SKILL.md
  points to it as the adapter contract, so the lead should land
  B2-07's doc edits with or before this file.
- **G-2 setup.sh on disk still hard-requires a browser binary**
  (verified in `scripts/setup.sh`: discovery loop + `chrome
  MISSING` sets `ok=0`). SKILL.md §0 states the native prerequisites
  directly from the adapter and no longer claims a browser check,
  but setup's own native checks are B2-04's landing; until then a
  browser-less machine fails setup even though §0's manual
  provisioning path works.
- **G-3 selftest.sh on disk is still the old pipeline** (capture.mjs
  legs + the RENDERER-guarded native hook). SKILL.md keeps only the
  pass-line contract (`SELFTEST PASSED`); B2-03's native selftest
  is what must satisfy it. The shipped selftest is not evidence for
  the native flow.
- **G-4 Template on disk still ships the retired files**
  (`capture.mjs`, `capture.ts`, `perf_probe.mjs`, the HTML pages).
  §1's layout lists the native toolset — all of which **does**
  exist in `template/tools/` today (verified) — but the quarantine
  is B2-01's landing, and `new_video.sh`'s printed flow is
  B2-01/B2-07 text.
- **G-5 Castsheet is NOT at parity — gaps G1/G2/G3/G4 stand.**
  36 of 77 catalog held items are plate-blocked and skipped by
  name; moods `puzzled`/`deadpan` are absent from the native
  painter (B2-09); look features are dropped by the painter
  (pattern/detail fields, freckles/blush/pencil/monocle, v6
  appendage fields; only cap/bowler hats, tie/scarf necks,
  plain/vest patterns painted — B2-10 assesses port-or-exception).
  The rewrite therefore claims only staged-frames + skip-by-name
  honesty and adds §6's skip-list instruction; it does not claim
  castsheet parity, and a castsheet read remains a partial review
  for held/mood/look coverage until B2-09/B2-10 land.
- **G-6 three.js remains in the authoring sources.**
  `template/src/shots.ts` (and `main.ts`/`sheet.ts`) still import
  three at build time. The rewrite makes no three.js claim (zero
  mentions; rendering is the binary's), but removing three from
  the authoring/build path is template scope (B2-01) and was not
  verified by this seat.
- **G-7 Bridge generality is proven on one film only.**
  §5 step 4 states the manual remainder (export-v2 skeleton +
  `owes`; draw-body animation, 3D geometry, caption strips,
  package authoring still authored work). The only end-to-end
  bridge evidence is the Onoda data (CR-B1-06 round-trip +
  telemetry CHECK OK on the built bridge). No brand-new project's
  bridge has been run end-to-end by any seat — Batch 3's proof is
  the first. Do not read §5 step 4 as "bridging is routine".
- **G-8 Check-report parity residuals (CR-B1-04b).**
  Native check shot lines carry set/atmosphere in place of the
  page-side free-text note (labelled in the output itself); two
  Onoda report-line boundary diffs (S01 start 0.24 vs 0.00; one
  S09/S10 boundary 45.36 vs 45.37) are report cosmetics, not
  verdict diffs; the CAP_CHECK strict/default divergence is
  code-proven but no current fixture exhibits it. SKILL.md does
  not claim byte-identical report text — recorded here so nobody
  reads it as doing so.
- **G-9 SKILL.md's own pointers are mid-migration.**
  `docs/NATIVE-BACKEND.md` still says the old backend is "the
  renderer of record" (opt-in ruling) — the rewrite no longer
  points to it. But §3/§6 still point to `library/modes.md` and
  `library/qa-gates.md` as full spec, and those files still carry
  old capture commands / the preview-URL deliverable (B2-07's
  docs set). Their gate **thresholds** are unaffected and nothing
  in SKILL.md weakens them; only their capture mechanics lag.
- **G-10 No local render verification was possible.**
  This VM has no Vulkan ICD at all: native frame rendering is
  env-blocked here (telemetry/check are CPU-only; the adapter
  check smoke passed at L6 integration). All render-behaviour
  statements rest on the adapter's code + Batch-1 lavapipe/CI
  evidence. First RADV confirmation rides the user's PC session;
  Batch 3's independent proof is still owed. SKILL.md claims no
  speed anywhere.
- **G-11 One command-form tension for the lead to rule on.**
  §0 keeps v6's "never call the system python for skill tools",
  while §1/§5 use `python3 tools/cues_native.py` — because that is
  the exact route the adapter itself prints
  (`mode_dispatch.mjs` `CUES_COMMAND`) and the emitter is a
  stdlib-only *project* tool (arg-free invocation verified
  byte-identical at B1-05). If the lead prefers one python rule,
  the fix is a one-line change to `CUES_COMMAND` + these two
  mentions together — flagged, not silently harmonised.
