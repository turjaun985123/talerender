# R2-08 — Final Chrome-reference audit + golden-sheet re-strike prep

Seat R2-08, Chrome-Replacement REMEDIATION wave, Batch 2. Date: 2026-11-16.
Trees audited (current, post-remediation Batch 1): skill
`~/workspace/custom-skills/tall-tale-shorts/` · talerender
`~/workspace/talerender` @ `01a941a`. Nothing committed; nothing in the
shipped trees edited. All work lives in this seat directory.

**Verdicts up front**

- **Part A: production class = 0 hits in both repos. No NAMED DEFECT** —
  no hit on any production surface (skill template/scripts, talerender
  src/tools-nonlegacy, either repo's workflows) launches or requires
  Chrome. All three B3-09 blockers are verified dispositioned. Two
  residuals are named (§A4), neither is a launch defect.
- **Part B: the strike is NOT executable this wave** — the golden
  project is still unbridged and the missing input is unchanged from
  B3-08, now scoped exactly (19 segment transcriptions + strip/plate
  rasterisation for a 21-shot film). The pack-verify ERROR does **not**
  clear. One NEW finding (F-B1): pack-verify can now false-PASS an
  unbridged project off its stale Chrome sheet. Strike candidate +
  exact CI procedure delivered (§B4–B5).

---

## PART A — Grep audit (re-run of B3-09)

### A1. Method

Pattern family recovered from `CR-B3-09-AUDIT.md` +
`batch3/B3-09/commands.txt` and re-run verbatim (see `audit/commands.txt`;
raw output in `audit/raw_skill.txt`, `audit/raw_talerender.txt`):

```
grep -R -n -i -E 'chrome|chromium|puppeteer|capture\.mjs|capture\.ts|perf_probe|headless' <REPO>
  --exclude-dir=.git --exclude-dir=node_modules --exclude-dir=previews --exclude='*.jpg' --exclude='*.png' --exclude='*.pyc'
```
plus the matching `find` for filename hits. `CHROME_FLAGS`/`CHROME` are
subsumed by case-insensitive `chrome` and are additionally tagged per
line in the CSV. Classification: `audit/parse_classify.py` (rules in its
header; decision order legacy → scrub/guard → comment-downgrade →
production-action → historical default, plus two human overrides, both
documented in code). An independent tree-walk counter (first audit
implementation) reproduced the headline line counts exactly
(skill 359, talerender 1652), so the collection is cross-validated.

Deviations from B3-09, declared: (1) talerender `target/` (gitignored
cargo build output; absent from B3-09's counts) is split out — 8 grep
lines (`.d` dependency files naming e.g. `headless_surface.rs`) in
`audit/audit_target_excluded.csv`, not in the headline; (2) binary
files are counted as files, not lines (the repo-root `talerender`
binary still matches — B3-09's FALSE-POSITIVE: `monochrome`/`chroma`
strings); (3) classes are this wave's 4-class scheme; mapping to
B3-09's: production ≈ OPERATIONAL, legacy-quarantine ≈ RETIRED-LEGACY,
negative-guard ≈ NEGATIVE-GUARD, historical-doc ≈ CONTRACT-COMMENT +
FALSE-POSITIVE.

### A2. Counts (machine-readable: `audit/audit.csv` — repo, file, line, pattern, class, excerpt; 2,011 rows)

| class | skill | talerender | total |
|---|---|---|---|
| production | **0** | **0** | **0** |
| legacy-quarantine | 100 | 141 | 241 |
| negative-guard | 77 | 211 | 288 |
| historical-doc | 182 | 1,300 | 1,482 |
| **total text hits** | **359** | **1,652** | **2,011** |

Per-pattern line counts (a line may match several): skill — chrome 269,
CHROME 74, capture.mjs 78, capture.ts 8, perf_probe 26, headless 14,
chromium 13, CHROME_FLAGS 31, puppeteer 0. talerender — chrome 1,348,
CHROME 163, capture.mjs 195, capture.ts 209, headless 90, chromium 58,
CHROME_FLAGS 51, perf_probe 11, puppeteer 10 (all 10 are negations of
the B3-09 kind: "no puppeteer dependency").

Filename hits: skill 10 paths — **all** under `legacy/chrome-capture/`.
talerender 89 paths — 84 under `tools/legacy/fog-chart-chrome/`, plus
`docs/ab/telemetry/chrome-cutouts.json` + `chrome-meta.json` (evidence
data), `docs/integration/CHROME-REPLACEMENT-CONTRACTS.md` (the wave
contract), and `tools/ci/fixtures/frame_hash/chrome/` + 3 fixture PNGs
(recorded frames, data only). Full lists: `audit/filenames_*.txt`.

### A3. Delta vs B3-09 (1,628 → 2,011; +383) and disposition verification

B3-09: skill 356, talerender 1,272; OPERATIONAL 144 (skill 5,
talerender 139). The growth is documentation, not regression:

| driver | Δ | what it is |
|---|---|---|
| talerender `docs/integration/` 644 → 896 | +252 | seat notes landed after B3-09's snapshot (CR-B3-01…10 in `fbfa0ca`/`9743850`, CR-R1-* + REMEDIATION-CONTRACTS in remediation Batch 1) quoting Chrome as history — historical-doc/negative-guard |
| talerender `proof/` 0 → 92 | +92 | the B3-02 CI proof fixture (a copy of the native adapter + fixture film.json) — its only actionable-looking line is the cues_native docstring (negative-guard, §A4) |
| fog-chart move | 113 OPERATIONAL → legacy-quarantine | `tools/fog-chart/chrome/` (122 hits) → `tools/legacy/fog-chart-chrome/` (117 hits) + `tools/legacy/README.md` ("do not run"); commit `de84b32` |
| docs/ab harnesses | 26 OPERATIONAL → 24 legacy-quarantine + 3 negative-guard | retired in place by `de84b32`: line-1 `RETIRED … Do not run` banner on drive.mjs / run-harness.sh / run-reference.sh, exec bits removed (`-rw-rw----`) |
| skill STYLE.md | 5 OPERATIONAL → 0 (2 hits remain) | line 70 now documents `capture_native.mjs` and names the Chrome pair as retired (negative-guard); line 107 is past-tense history. B-1 CLOSED |
| skill `library/publishing.md` | 1 hit → 0 | the F6 stale `capture.mjs frames` routing is gone. CLOSED |
| skill net | 356 → 359 (+3) | STYLE/publishing fixes and the `tests/28` → `legacy/` move, offset by Batch-1 landings (world_stage.py, poster_stage.py, validate_posters.py) and the rewritten selftest's guard text |

**Executable-surface re-verification (the load-bearing check):** all
427 hits on production surfaces (skill `template/`, `scripts/`,
SKILL.md, STYLE.md, README.md; talerender `src/`, `tools/` non-legacy,
`proof/adapter/`, `.github/workflows/`, Cargo.toml) were listed and
read: 299 historical-doc (parity/provenance comments — e.g.
`src/telemetry.rs` transcriptions from capture.ts, adapter headers
quoting the retired contract) + 128 negative-guard (selftest's
`env -u CHROME` invocations of capture_**native**, setup.sh's "NO
Chrome/Chromium/Brave is downloaded…", new_video.sh/retrofit.py
deletion/quarantine lists, proof.yml's scrub + shim + EMPTY-verdict
steps — 30 of its 31 workflow hits are guards; the 31st is ci.yml's
"headless rendering" step name, i.e. native offscreen). **No spawn,
discovery, env requirement, or live instruction for a browser exists
on any production surface.**

### A4. Named residuals (not defects — nothing here launches from a production path)

- **R-A1 — `scripts/*.v6-legacy` (10 hits, legacy-quarantine by suffix).**
  `scripts/selftest.sh.v6-legacy:29-32` still contains runnable
  `node tools/capture.mjs …` lines and `setup.sh.v6-legacy:31-32` a
  Chrome discovery loop. B3-09 classed them RETIRED-LEGACY; they are
  archived by filename suffix and **unreferenced** (verified: no live
  script or SKILL.md names them), but they sit in `scripts/`, not
  under `legacy/`. Recommendation to the lead: move both under
  `legacy/` so the quarantine is path-true, not name-true.
- **R-A2 — stale template authoring comments (historical-doc).**
  `template/src/core/board.ts:2`, `template/src/castsheet.ts:3`,
  `template/src/main.ts:3,63,68` still describe `tools/capture.mjs` in
  present tense as what drives the OP API / castsheet / stylesheet.
  Comments only (the sources are consumed via the bridge); same items
  B3-09 listed as non-blocking residue. One-line comment fixes whenever
  the template is next touched.

---

## PART B — Golden-sheet re-strike prep (B3-08 remainder)

### B1. Verdict

**NOT EXECUTABLE this wave.** The single missing input B3-08 named —
a native bridge of the golden project — still does not exist, and this
seat's retrofit attempt proves it cannot be produced by the shipped
tooling alone: `build_bridge.py` assembles a bridge from transcribed
segment scenes, and no transcription of this film exists anywhere.
Bridging = authoring 19 segment scene files (draw bodies + full 3D set
geometry for 6 sets) + caption-strip/plate rasterisation — a
transcription work item of the B3-01 class, not a strike side-effect.
Forcing a number now would require fabricating a hash; refused, as in
B3-08. Evidence: `golden-staging/RUNS.md` (verbatim outputs),
`golden-staging/retrofit-audit.md`, `golden-staging/skeleton.json`.

### B2. What was done in staging (`golden-staging/golden-proj`, a copy — the original was never touched)

1. **Located + audited.** `~/workspace/tall-tale-v5-work/golden-proj`
   ("The Girl Who Was Mailed To Grandma", 51.723 s, 21 shots, fonts
   incl. Inter-Black.otf — the native label font). Shipped
   `retrofit.py audit`: **RETROFIT NEEDED (15 FAIL, 3 WARN)** — native
   tooling absent, `native/film.json` + film data absent; the retired
   Chrome sheet still on disk hashes to the pinned
   `c535024d05b0d268186af4a4f7103642` (re-verified).
2. **Skeleton exported** with the current repo exporter: 19 segments,
   9 owes, **4 BLOCKING** (`draw-body-animation`, `scene-geometry`,
   `caption-strips`, `native-package-absent`).
3. **Bridge attempted** with the CURRENT shipped
   `tools/bridge/build_bridge.py` (post-decoupling: emits
   `"driver": "scene-sequence"`, invents no calendar/sign): fails
   closed — `scenes dir not found` (exit 2) / `no *.scene.json files …
   it does not transcribe shots.ts itself` (exit 1). `native/` never
   created. There are no golden-film scene files in either repo to
   feed it (the only film scenes on disk are Onoda's and the
   Lighthouse Cat's).
4. **Tooling installed** into the copy (capture_native.mjs +
   native_modes/ + cues/castshee
...[truncated 5016 chars]