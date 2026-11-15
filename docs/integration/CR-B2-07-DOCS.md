<!--
DELIVERY NOTE — Chrome Replacement, Batch 2, Seat 7 (CR-B2-07):
docs set + CHANGELOG/7.0 preparation.

Ownership law honoured: NEW files only, nothing committed, nothing
edited in place — in either tree. The skill tree
(~/workspace/custom-skills/tall-tale-shorts/, v6.0) and the live
adapter were never written to; all patch work was done on copies
under /tmp and verified there. The skill's version was changed
nowhere (SKILL.md still reads 6.0; the staged README still reads
6.0; 7.0 exists only inside the two prepared files, as text for
the lead's flip commit).

Staged deliverables (root:
~/workspace/chrome-replacement/batch2/B2-07/, paths mirror their
skill targets unless noted):
  docs/NATIVE-BACKEND.md                  full rewrite (§2)
  docs/NATIVE-BACKEND-SKILL-PATCH.md      banner-classified copy (§2)
  docs/ADOPTION-REVIEW-W59.md             banner-classified copy (§2)
  docs/DRY-RUN-W44.md                     banner-classified copy (§2)
  template/tools/CAPTURE-NATIVE.md        full rewrite (§2)
  template/tools/capture_native.mjs.patch the findManifest/
                                          findFilmDir fix (§3)
  README.md                               aligned copy (§2)
  CHANGELOG-7.0-entry.md                  prepared entry (§4)
  BUMP-RATIONALE-7.0.md                   prepared rationale (§4)
This note: docs/integration/CR-B2-07-DOCS.md (repo, new file).
-->

# CR-B2-07 — Docs set + CHANGELOG/7.0 preparation

## 1. Method

Every behavioural claim in the rewritten docs was read from the
Batch 1 code as wired at integration L6 (talerender `172052d`),
not from the v6.0 docs: `template/tools/capture_native.mjs`
(433 lines), `template/tools/native_modes/{mode_dispatch,
check_mode, sheet_mode, castsheet_mode, filmdata_validate,
era_hint, render_meta}.mjs`, `template/tools/cues_native.py`,
`template/tools/castsheet_native.py`, and the bridge tools
`tools/bridge/build_bridge.py` + `package_bridge.py` (repo).
Where a number appears in the docs it was counted in code:
castsheet catalog 77 items = 17 poses + 19 moods + 5 look
presets + 36 held props (`castsheet_native.py` tables);
cues parity 9/9 byte-identical (CR-B1-05); check parity
11/14 shots, 71/73 captions (CR-B1-04); bridge round-trip
telemetry PASS (CR-B1-06).

## 2. The staged docs set

| Staged file | Target | What changed and why |
|---|---|---|
| `template/tools/CAPTURE-NATIVE.md` | same path | **Full rewrite.** The v6.0 copy's mode table is stale in four places: `sheet` is no longer "Partial" (CR-B1-04 `sheet_mode.mjs`: exact tile filter, fixed 250×86 label box, sans-first font, audit-after-tiling with exit 1 when the audit cannot run); `castsheet` is a native mode, not a refusal (CR-B1-03); `cues` is a routing answer to `tools/cues_native.py`, not a Chrome-only refusal (CR-B1-05/10); `preview` is retired. Also new vs the old doc: the data-driven film-data validation (P4 — the old doc still lists the hardcoded Onoda asset list: "the five hold plates, the four calendar plates + dark plate + sign"); `check`'s parity report (tables input, labelled shot-note substitution, script.json authored correction, CAP_CHECK re-derivation, exit 3); the era-hint inline remedy; the `video` provenance sidecar; the changed-mode dispatch running *before* manifest validation. Removed: the "work copy / not yet part of the shipped skill" provenance block and the W64/W65 change notes (wave history; the facts they added are folded into the body). |
| `docs/NATIVE-BACKEND.md` | same path | **Full rewrite.** The v6.0 copy's load-bearing claims are all inverted by the mission: "Chrome is still the renderer of record", "opt-in alternative", `cues`/`castsheet` "✗ Chrome-only", "golden-sheet procedure stays Chrome", "until bridged, it is captured with Chrome — the adapter's refusal says so itself". The rewrite states native as default and only production renderer; gives all seven modes their as-built behaviour; documents **both** bridge routes (`build_bridge.py` general route + `package_bridge.py` packaging route — the old doc knew only the packager and called a general converter "future work"); carries CR-B1-06's honest manual remainder; names the castsheet gaps G1/G2/G3-G4 as known limits with their Batch 2 seats; and keeps the still-true sections' substance (clean-frames decision (B), era pairing, failure behaviour, caption-measurement / CPU-composite / DOF limits) updated to the wired code. Batch 3's proof gate is stated as a gate, not a result. |
| `docs/NATIVE-BACKEND-SKILL-PATCH.md` | same path | **Banner-classified copy** (body verbatim): RETIRED-LEGACY DOC. The proposal was consumed at adoption and its opt-in premise superseded by the replacement order; B2-02 owns the actual SKILL.md rewrite. |
| `docs/ADOPTION-REVIEW-W59.md` | same path | **Banner-classified copy** (body verbatim): HISTORICAL RECORD (pre-adoption review, gaps G1–G8). |
| `docs/DRY-RUN-W44.md` | same path | **Banner-classified copy** (body verbatim): HISTORICAL RECORD. Its Runs 4–5 transcripts show the pre-Batch-1 refusals ("Chrome-only … Run: node tools/capture.mjs …") — behaviour Batch 1 replaced; the banner says so at the top so no reader takes the transcripts as current. |
| `README.md` | skill root | **Aligned copy** (current-state sections only; pre-v5 CHANGELOG history byte-verbatim): version line 5.0 → **6.0** (it was already stale against the shipped skill; deliberately *not* 7.0 — the bump is the flip commit); the "GPU-first via capture.mjs/CHROME_FLAGS" bullet replaced with the native wgpu renderer; Bun's role corrected (build step; Node runs capture); **Needs** now lists the talerender binary + `TALERENDER_BIN` + a Vulkan driver and states no browser is required or used. A staging banner at the top marks it as prepared for the flip. |

**Not in this seat's remit, flagged for the lead:** `SKILL.md`
(B2-02 owns it — including its §6 golden-sheet procedure and the
opt-in native paragraph at lines 108–126), `library/qa-gates.md`
and `library/modes.md`. The grep in §5 matches the word
Chrome/Chromium/puppeteer only; those two library docs invoke
`capture.mjs` (qa-gates Gate 2/3 lines, a `capture preview`
mention) without the word, so they do not appear in the table —
they still need the Batch 2 sweep before the flip, or the flipped
skill will document a capture tool it no longer ships.

## 3. The patch — adapter error texts offer the bridge, not Chrome

**Defect (verified in code).** `capture_native.mjs` line 99,
`findManifest()`: *"…a project without it is captured with Chrome
(tools/capture.mjs). See tools/CAPTURE-NATIVE.md…"* — the last
adapter error text that offers the retired renderer as the remedy
for an unbridged project. `findFilmDir()`'s final failure (line
110) names the bridge generically ("generated by the bridge")
but gives no route. Both contradict contracts §1 row 5's rule
("an unbridged project fails loudly naming the bridge step —
never a Chrome fallback") and the style of the Batch 1 surfaces
(`era_hint.mjs`: symptom named, exact commands, no other backend
offered; `cues_native.py`'s unbridged error: bridge tools named,
"There is no Chrome fallback").

**Fix (staged as `template/tools/capture_native.mjs.patch`, two
one-line hunks, nothing else):**

- `findManifest()` remedy → *"…a project without it is not
  bridged yet — bridge it first, then re-run. Build the bridge
  from the project's own data with the bridge builder
  (tools/bridge/build_bridge.py in the talerender repo); for
  film data the repo already holds, package it instead
  (tools/bridge/package_bridge.py, same repo). See
  tools/CAPTURE-NATIVE.md for the layout. There is no other
  capture backend to fall back to."*
- `findFilmDir()` remedy → appends *"…Bridge the project first —
  build the bridge from the project's own data with
  tools/bridge/build_bridge.py, or package film data the repo
  already holds with tools/bridge/package_bridge.py (both in the
  talerender repo) — then re-run. See tools/CAPTURE-NATIVE.md
  for the layout."*

**Verification (run 2026-11-15, on copies under /tmp — the live
file was never touched).** The patch was produced by editing a
copy of the shipped adapter and diffing; then, in a fresh scratch
tree holding a pristine copy at the mirrored path:

```
$ git apply --check capture_native.mjs.patch
git apply --check: PASS
$ git apply capture_native.mjs.patch
git apply: PASS
$ node --check template/tools/capture_native.mjs   (applied copy)
node --check (applied copy): PASS
$ node --check <edited copy, pre-diff>
node --check (patched copy): PASS
$ diff applied-copy edited-copy
applied copy == edited copy: IDENTICAL
$ grep -c 'capture.mjs). See' <applied copy>
0        (the Chrome-offer sentence is gone)
$ grep -o 'bridge it first, then re-run' / 'There is no other capture backend to fall back to'
both present in the applied copy
```

**Out of the patch's scope, named for the lead:** two further
Chrome mentions in the same file describe the *selection story*,
not the unbridged remedy — the header (line 14, "capture.mjs
(Chrome) is the default backend") and the `RENDERER` guard text
(line 57, offering `node tools/capture.mjs <mode>`). Both become
false at the flip; both belong to the template/selection rewrite
(B2-01/B2-02 + the flip commit), so this seat's patch does not
touch them. They are classified in §5.

## 4. Prepared 7.0 — CHANGELOG entry + bump rationale (NOT landed)

- **`CHANGELOG-7.0-entry.md`** — the complete `## v7.0` entry for
  the lead to prepend to `CHANGELOG.md` at the flip: mode-by-mode
  replacement table with seat citations, the bridge as the
  breaking layout change, era pairing, the removed/retired list
  (`capture.mjs`/`capture.ts`/`perf_probe.mjs`, the absorbed
  native selftest, the `RENDERER` story), the unchanged list, and
  a v6→v7 migration note. Its verification block is bracketed
  slots (proof film, Onoda re-run, failure modes, flip sha, sync
  result) — Batch 3 evidence the lead fills at landing; nothing
  in the entry asserts a Batch 3 result.
- **`BUMP-RATIONALE-7.0.md`** — why 7.0 and not 6.1: three
  independent breaking changes, each quoted against code —
  (1) project layout gains the native bridge (a v6 project is
  not capturable until bridged); (2) `capture.mjs` leaves the
  template (every v6 capture command stops existing as shipped);
  (3) binary era-pairing becomes mandatory (one specific binary,
  generation-paired per project, replacing an auto-discovered
  browser). Plus the flip-commit checklist (version stamps,
  `LIBRARY OK` re-verification, the standing sync) and the
  rejected alternatives (6.1; bumping before the proof —
  contracts §4.5 forbids it).

## 5. Grep audit — every Chrome/Chromium/puppeteer mention outside SKILL.md

**Pattern & count.** `grep -rniE 'chrome|chromium|puppeteer'`
over the whole skill tree, `node_modules` excluded, run
2026-11-15 against the shipped v6.0 tree: **229 lines** total —
6 in `SKILL.md` (excluded here; B2-02 owns it) and **223 lines
across 27 files**, every one classified below. `puppeteer`:
**0 hits** anywhere. `chromium`: 12 lines, all inside the
browser-discovery lists of `capture.mjs` / `capture.ts` /
`setup.sh` (rows 1, 2, 6).

**Classes.** *Retired-legacy doc* = prose describing the retired
pipeline as current; replaced or banner-marked by the staged
docs, or owned by another seat's rewrite. *Code to patch* =
executable/catalog content that invokes, requires, or routes to
the retired renderer; retired or rewritten by its owning seat
(this seat's patch covers exactly one row, marked). *Historical
record* = dated wave/release history **and** parity-spec or
provenance mentions inside the native code that name the retired
renderer only as the measured-against baseline and offer no
route, flag, or behaviour — kept verbatim; deleting them would
falsify the record of what parity was proven against.

| File (skill-relative) | Hits | Class | Disposition / owner |
|---|---:|---|---|
| `template/tools/capture.mjs` | 26 | code to patch | The Chrome capture itself. Retire/quarantine out of the template — B2-01 (contracts rows 1–8). |
| `template/tools/capture.ts` | 26 | code to patch | Bun port, same dependency. Retire with it — B2-01 (row 8). |
| `template/tools/perf_probe.mjs` | 14 | code to patch | Chrome-page probe (`discoverChrome`, CDP spawn). Retire; native equivalent is the repo bench lab — B2-01 (row 9). |
| `tests/28/perf_probe.test.mjs` | 1 | code to patch | The probe's test (its one hit is the "no Chrome" header comment). Retires with the probe — B2-01. |
| `library/index.json` | 3 | code to patch | The `perf_probe` catalog entry documents its `--chrome` flag/fallback. Update when the probe retires — lead with B2-01; `library_check.py` must stay green. |
| `scripts/setup.sh` | 3 | code to patch | Browser discovery + hard Chrome requirement (lines 3, 31–32). Native requirements — B2-04 (row 10). |
| `scripts/selftest.sh` | 7 | code to patch | All four capture invocations are `capture.mjs` with `CHROME*` env passthrough. Native end-to-end — B2-03 (row 11). |
| `tools/selftest_native.sh` | 4 | code to patch | The opt-in smoke ("Chrome flow unaffected" SKIP texts). Absorbed into the native selftest — B2-03 (row 16). |
| `tools/SELFTEST-NATIVE.md` | 4 | retired-legacy doc | The absorbed smoke's doc ("additive-only; Chrome path byte-identical"). Retires with its script — B2-03 (row 16). |
| `template/tools/capture_native.mjs` | 8 | **split** | L99: **code to patch — THIS seat's patch (§3)**. L14 (header: "capture.mjs (Chrome) is the default") + L57 (RENDERER guard offering `capture.mjs`): code to patch at the flip — B2-01/B2-02 selection story, flagged in §3. L2, L10, L11, L270, L402: historical record (provenance/parity naming; L11 "There is no Chrome fallback" is current correct behaviour). |
| `docs/NATIVE-BACKEND.md` | 26 | retired-legacy doc | Opt-in-era operator doc. **Replaced by this seat's staged rewrite (§2).** |
| `template/tools/CAPTURE-NATIVE.md` | 12 | retired-legacy doc | Pre-Batch-1 adapter contract. **Replaced by this seat's staged rewrite (§2).** |
| `docs/NATIVE-BACKEND-SKILL-PATCH.md` | 8 | retired-legacy doc | Consumed adoption proposal. **Banner-classified staged copy (§2).** |
| `docs/ADOPTION-REVIEW-W59.md` | 5 | historical record | Dated adoption review. **Banner-classified staged copy (§2)**; body verbatim. |
| `docs/DRY-RUN-W44.md` | 11 | historical record | Dated dry run incl. pre-Batch-1 refusal transcripts. **Banner-classified staged copy (§2)**; body verbatim. |
| `CHANGELOG.md` | 4 | historical record | L8/L10: v6 release-verification note (CHROME env on the QA VM). L133/L142: v5.0 entry (browser auto-discovery, GPU-by-default). Keep verbatim; the 7.0 entry prepends at the flip (§4). |
| `README.md` | 5 | **split** | L19 + L30: retired-legacy doc (current-state claims — GPU-via-Chrome bullet, "Needs: Chrome/Chromium/Brave") — **fixed in this seat's staged README (§2)**. L226, L227, L256: historical record (pre-v5 changelog history, verbatim). |
| `STYLE.md` | 1 | retired-legacy doc | L105: `CHROME_FLAGS` guidance written as current craft advice. Outside this seat's `docs/*.md` remit — owner: lead's Batch 2 docs sweep (with B2-02). Flagged, not staged. |
| `template/tools/native_modes/mode_dispatch.mjs` | 14 | historical record | Wave provenance header + the three changed modes' statements. Verified at L81/L146/L166: its runtime texts offer **no** Chrome route (missing-module, cues-routing, preview-retired messages all name native remedies only). Keep. |
| `template/tools/native_modes/sheet_mode.mjs` | 11 | historical record | Parity-spec comments (tile filter, 250×86 label box, audit classes "Chrome's sheet prints"). The spec of what parity was measured against. Keep. |
| `template/tools/native_modes/castsheet_mode.mjs` | 9 | historical record | Parity-spec comments (per-item q, label semantics, tile filter/geometry). Keep. |
| `template/tools/native_modes/check_mode.mjs` | 5 | historical record | Parity-spec comments (which retired line classes were missing/divergent; CAP_CHECK semantics). Keep. |
| `template/tools/castsheet_native.py` | 6 | historical record | Transcription provenance (staging constants named `chromeFloorY`/`chromeScaleAt175` are data keys recording the source staging's proportions). Keep. |
| `template/tools/cues_native.py` | 6 | historical record | Includes L150 — the emitter's unbridged error, which is the **correct** bridge-route text ("There is no Chrome fallback: cues are read from native scene data, never from a page"). Keep. |
| `template/tools/native_modes/era_hint.mjs` | 1 | historical record | Wave provenance string only ("CHROME-REPLACEMENT WAVE, Batch 1, Seat 10"). Keep. |
| `template/tools/native_modes/filmdata_validate.mjs` | 2 | historical record | Wave provenance strings only. Keep. |
| `template/tools/native_modes/render_meta.mjs` | 1 | historical record | Wave provenance string only. Keep. |

Counts check: 26+26+14+1+3+3+7+4+4+8+26+12+8+5+11+4+5+1+14+11+9+5+6+6+1+2+1
= **223** ✓ (plus SKILL.md's 6 = the 229 total).

**Post-flip expectation (for Batch 3's final grep audit):** after
B2-01/B2-03/B2-04 land, this seat's patch applies, and the staged
docs land, every remaining hit outside SKILL.md should be class
*historical record* — the retired code rows gone, the legacy docs
replaced or banner-marked. Any *code to patch* row still present
at that audit is an incomplete landing, by name.

## 6. What this seat did NOT do

- No file in the skill tree or the repo was edited, and nothing
  was committed; the lead lands everything staged here.
- The version was not bumped anywhere (§4 is preparation only).
- `SKILL.md` was not touched or audited beyond its hit count
  (B2-02 owns it); `library/qa-gates.md` / `library/modes.md`
  carry `capture.mjs` references outside the grep pattern and
  are flagged in §2 for the lead's sweep, not silently passed.
- No render, telemetry run, or selftest was executed for this
  seat — its claims are code-read, and the patch verification in
  §3 is the only execution evidence offered.
