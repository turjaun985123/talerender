# CR-B1-04 — Sheet + Check parity (Chrome Replacement, Batch 1, Seat 4)

Completion report for contract rows **1 (`check`)** and **4 (`sheet`)**
(`CHROME-REPLACEMENT-CONTRACTS.md` §1). The original seat delivered the
two modules and most evidence, then errored on a runtime restart before
writing its report; this seat reconstructed the state from code + the
inherited evidence, completed the missing runs, fixed one defect found
in verification, and writes the verdict here.

**Deliverables under review**

- `template/tools/native_modes/sheet_mode.mjs` (skill tree) — parity
  sheet intended to supersede the adapter's built-in partial sheet.
- `template/tools/native_modes/check_mode.mjs` (skill tree) — parity
  check intended to supersede the adapter's telemetry-only check.
- Baselines: `template/tools/capture.mjs` (Chrome, shipped v6 template)
  and `template/tools/capture_native.mjs` (the adapter). All claims
  below were verified against these files' code, not their docs.

**Verdict up front**

| Row | Verdict |
|---|---|
| 1 `check` | **REPLACE-NATIVE achieved** with `check_mode.mjs`. Every Chrome line class is present or present-with-a-labelled-substitution. One real defect was found and fixed in this seat (caption text dropped — §D). Residual differences are data-level and ≤ 0.24 s on three report lines (§E); none touches the gate verdict. No EXCEPTION proposed. |
| 4 `sheet` | **REPLACE-NATIVE achieved** with `sheet_mode.mjs`. Tile geometry, sampling, `--guides`, and final encode are byte-identical contracts; the audit Chrome's sheet prints is reproduced from native telemetry (finer sampling, disclosed). Label font/box divergences of the adapter are fixed. No EXCEPTION proposed. |
| Stale bridge | **Parity evidence, working as designed**: the Onoda project's packaged bridge data is stale and both native checks refuse it closed (§C.5). |

Evidence root: `~/workspace/talerender-evidence/parity-b1/`.
Binary used throughout: `~/talerender-bin/talerender` (current CI
artifact, supports `--film-telemetry --tables` — verified in its
`--help` and strings). Film: "THE SOLDIER WHO DIDN'T KNOW THE WAR WAS
OVER" (Onoda), 60.36 s, 1811 frames @ 30 fps, 1080×1920.

---

## A. Sheet parity (row 4)

Reference behaviour is `capture.mjs` sheet mode; the adapter's built-in
sheet is the "before" state; `sheet_mode.mjs` is the candidate.

| Aspect | Chrome `capture.mjs` | Adapter (before) | `sheet_mode.mjs` | Verdict |
|---|---|---|---|---|
| Sample selection | `t = step/2 + k·step`, `t < dur` | identical | identical | **present, exact** |
| Tile filter | `scale=216:384,tile=<cols>x<rows>:padding=4:color=black` | identical string | identical string | **present, exact** |
| `--guides` filter | adds `drawgrid=w=72:h=128:t=1:c=white@0.35,drawbox=x=0:y=268:w=216:h=42:color=red@0.55:t=1` | identical string | identical string | **present, exact** |
| Final encode | ffmpeg `-q:v 3`, one JPEG | same | same | **present, exact** |
| Output log line | `<out> (<n> frames, every <step> s)` | same + a disclaimer paragraph (no audit; "golden-sheet procedure stays Chrome") | same line; disclaimer replaced by the audit itself + one sampling-disclosure line | **equivalent** (disclaimer correctly retired — contract row 4 retires the Chrome golden-sheet clause) |
| Per-tile quality | canvas JPEG q0.8 | ffmpeg `-q:v 2` from clean PNG | ffmpeg `-q:v 2` from clean PNG | **superset** (preview-grade either way; disclosed in module header) |
| Label text | `t.toFixed(2)`, canvas `fillText` at (14, baseline 64) | drawtext `x=14:y=12` | drawtext `x=14:y=8`, fontsize 60 | **equivalent** (drawtext y is text-top, not baseline; placement approximates canvas at this size) |
| Label font | `700 60px sans-serif` (bold sans) | alphabetically-first project font → on Onoda `Caladea-Bold.ttf`, a **serif** | sans-first search (`Inter`/`Sans`/`DejaVu`) → on Onoda `Inter-Black.otf` | **fixed** (unit-verified: `findFont` returns Inter-Black for the project and the scratch copy) |
| Label box | fixed `fillRect(0,0,250,86)` black | drawtext auto box (`box=1:boxborderw=10`) — geometry drifts with digit count | `drawbox … w=250:h=86 … t=fill` + drawtext with no auto box | **fixed, exact geometry** |
| Audit after tiling | `printAudit` (CANVAS OVERFLOW, EDGE TOUCH) + `printComposition` (CAPTION ZONE, SAFE AREA, FRAMING, COMPOSITION advisory + summary), sampled at the sheet's own step times, report-only | **none** | binary film telemetry after tiling; the same line classes extracted and printed in the same order | **present** (see sampling row) |
| Audit sampling | sheet step (e.g. every 2 s at step 2) | — | telemetry's 0.25 s — a **finer superset**; counts/first-at times are therefore the audit's true values, not the step-sampled approximations; disclosed on an extra output line | **equivalent, disclosed difference** |
| Audit unavailable | per-sample errors swallowed (project-era) / counted with `unknown` + exit 1 (template-era); outer failure prints `CANVAS OVERFLOW: audit unavailable (<reason>)` with **no** exit code (template-era catch) | — | prints Chrome's own fallback line and sets **exit 1** | **divergent, stricter** — a sheet never certifies an audit it did not run (template-era P-03 spirit). Named here; lead may align to Chrome's exit 0 if byte-parity of exit codes is preferred over the stricter reading |
| CONTINUITY in sheet audit | not printed by Chrome's sheet (check-only) | — | not printed (regex deliberately excludes it) | **present, exact** |
| Exit codes | 0 sheet written; 1 ffmpeg failure / (template) incomplete audit; 2 usage | 0/1/2 same shape | 0 sheet written (audit report-only); 1 render/ffmpeg failure or audit unavailable; 2 usage | **equivalent** (modulo the stricter row above) |

**Run evidence (sheet)**

- Inherited: `native-sheet-step2.jpg` + `native-sheet-step2.log` —
  adapter sheet at step 2 on the re-bridged project (pre-restart VM,
  lavapipe): 30 frames, output 1316×1936.
- This seat: `chrome-sheet-step2.jpg` + `chrome-sheet-step2.log` —
  Chrome sheet on a scratch **copy** of the Onoda project, same step 2,
  cols 6, bundled Chrome
  (`~/.faceless-shorts/chrome/opt/google/chrome/chrome`; the flat
  `~/.faceless-shorts/chrome/chrome` path named in the seat brief does
  not exist on this VM — the binary lives one level down under
  `opt/google/chrome/`). The sheet image was written
  (`(30 frames, every 2 s)`); the process was then killed by the
  900 s timeout **during its audit phase** — Chrome's sheet audit on
  this VM (SwiftShader, one CDP evaluate per sample) did not finish in
  15 minutes. That asymmetry is itself parity-relevant: the native
  audit is in-process telemetry.
  - Dimensions: Chrome **1316×1936** = native **1316×1936** (6 cols ×
    5 rows × 216×384 tiles + 4 px padding — geometry identical).
  - Tile count: 30 = 30, same sample times.
  - Sheet-vs-sheet PSNR **22.47 dB** — a cross-renderer A/B (Chrome
    three.js vs native), consistent with the established film A/B
    (mean 23.02 dB, `V2-HANDOVER` era); the tiles carry the same
    scenes at the same times.
  - Label-box mean luminance, tile 0: native 51.0 vs Chrome 50.4 —
    both burn the dark timestamp box in the same place.
- This seat: `sheet-mode-step10.log` — `sheet_mode.mjs` on the current
  VM **fails closed at frame render**: `no wgpu adapter found` (see
  §C.6, environment). `sheet-mode-step10-seeded.log` +
  `sheet-mode-step10.jpg` — same run with the six sampled frames
  pre-seeded into the frame cache (synthetic 1080×1920 PNGs; the
  module's integrity check passed them and skipped rendering):
  sheet written at exactly 656×772 (3 cols × 2 rows), exit 0, and the
  **full audit block printed** — CANVAS OVERFLOW, all 18 EDGE TOUCH
  lines, FRAMING/SAFE AREA/COMPOSITION lines and the
  `COMPOSITION: 31 findings …` summary, byte-identical to the
  telemetry audit block, followed by the sampling-disclosure line.
  No "labels unavailable" warning was emitted (drawtext leg ran with
  the sans font).

## B. Check parity (row 1)

Reference: template `capture.mjs` check. (The Onoda **project's own**
`tools/capture.mjs` is an older, pre-v5 copy: its check has no
composition and no caption audit at all, and its sheet audit prints
only overflow/edge-touch. Parity is judged against the shipped
template, per the contract; the project copy is a Batch-2 retrofit
item, row 17.)

| Chrome line class | Adapter check (before) | `check_mode.mjs` | Verdict |
|---|---|---|---|
| `duration <d> s` | only inside its own banner line | printed, from the manifest (`60.36 s` = Chrome's) | **present, exact** |
| `renderer: <WebGL string>` + software-renderer warning | absent | `renderer: native (TaleRender) · binary <path> (via <how>) · no WebGL renderer string exists on the native path` | **substitution, labelled** — no WebGL exists natively; the provenance slot is never silently absent. The software-renderer warning has no native analogue (adapter identity is printed by the binary's render modes; telemetry is CPU-only) |
| Shot list `S01  0.00 → 5.91  (5.91 s)  <note>` | absent | reformatted from telemetry `--tables` SHOT TABLE; id/at/end/dur in Chrome's exact format | **present** with two data-level differences (§E) and a **labelled substitution**: the trailing free-text `note` is `OP.shots()` page-side data, absent from film data; the module prints `set=<set> atmosphere=<atmo>` from the tables and says so on every line |
| `captions:` list (`  s–e text`, time order) | absent | reformatted from CAPTION TABLE (window + text) | **present** — 73/73 entries, **0 text differences** vs the Chrome run; 2 entries differ by 0.01 s at one boundary (§E) |
| `CAST: a, b, …` | absent | from CAST LIST ids: `CAST: onoda, compA, compB, tani, suzuki` | **present** (Chrome's own check run timed out before its CAST line — §C.4 — so this is verified against the tables, not a Chrome transcript) |
| `ERROR at <t>: …` sample-error lines + page logs in verdict | n/a | n/a — telemetry samples in-process; a telemetry that cannot run is the exit-3 path, never a partial audit | **equivalent by design** |
| `CANVAS OVERFLOW: none` / itemised overflow | via telemetry | telemetry lines passed through byte-for-byte | **present, exact** (M6 field-identity) |
| `EDGE TOUCH (warning): …` | via telemetry | passed through | **present, exact** — all 18 lines identical to the prior telemetry run |
| `CONTINUITY: clean (N named characters tracked)` / violations | via telemetry | passed through | **present, exact** (`clean (5 named characters tracked)`) |
| `CAPTION ZONE` / `SAFE AREA` / `FRAMING` warnings | via telemetry | passed through, in Chrome's print order (audit → continuity → composition → caption audit — the binary's order is Chrome's) | **present, exact** (Onoda run: caption-zone 0, safe-area 8, framing 2) |
| `COMPOSITION (advisory): …` + `COMPOSITION: N findings (…)` summary | via telemetry | passed through | **present, exact** (`31 findings (caption-zone 0, safe-area 8, thirds 3, framing 2, edge-touch 18)`) |
| `CAPTION AUDIT: N chunks, A/L lines authored, min fit …, max rendered width … px` | printed **wrong**: the binary constructs every film caption with `auto=true` (`film.rs`), so it always reports `0/12 lines authored` on this project | **corrected from `script.json`**: `73 chunks, 12/12 lines authored, min fit 1.00, max rendered width 844 px (authored count corrected from script.json — …)` | **fixed** — Onoda's `script.json` authors `caps` on all 12 lines (verified in the file). Without `script.json` the module passes the binary's lines through and says so |
| `CAPTIONS: estimate-grade (auto-chunked) on lines …` | printed (artefact of the same `auto=true`) | recomputed: with all lines authored the line is **absent**, exactly as Chrome prints nothing for a fully-authored project; with some auto lines it prints Chrome's line for those lines only | **fixed** |
| `CAPTION WIDTH/FIT WARN:` (default) vs `FAIL:` + verdict folding (`CAP_CHECK=strict`) | binary hard-codes strict (`film.rs` calls `print_caption_audit(&caps, true, …)`), so its verdict counts caption failures unconditionally — **divergent from Chrome's default** | verdict **re-derived** from the binary's own FAILED counts: caption failures count only under `CAP_CHECK=strict`; FAIL tags are downgraded to WARN otherwise | **fixed in semantics** — on this film the two modes are indistinguishable (no chunk is over the width/fit bars: min fit 1.00, max width 844 ≤ 850), and the default and strict runs are byte-identical with exit 0 (§C.3). The divergence/fix is code-proven, not fixture-proven; no Onoda fixture can distinguish them |
| Final line `CHECK OK` / `CHECK FAILED` | adapter prints it from the telemetry verdict | printed from the **re-derived** verdict (differs from the binary's exactly when caption-only failures meet non-strict mode) | **present, corrected semantics** |
| Exit codes | 0 OK; 1 FAILED / any prerequisite or data failure (telemetry exit 3 collapsed to 1) | 0 OK; 1 FAILED or prerequisite/data failure; **3 = telemetry could not run** (no verdict line), surfaced from the binary's EXIT-CODES-W02 instead of collapsed | **divergent, disclosed** in the module header; verdict-line behaviour identical. Lead's call at integration |

## C. Run evidence (all under `~/workspace/talerender-evidence/parity-b1/`)

1. **Telemetry with tables** — `telemetry-tables-repo.txt` (159 lines,
   exit 0): `talerender --film-telemetry ~/workspace/talerender/film
   --tables`. Audit block identical to the inherited
   `telemetry-repo-prior-run.txt`; verdict `FILM TELEMETRY: CHECK OK`;
   SHOT TABLE (14), CAPTION TABLE (73), CAST LIST (5) all present in
   the exact formats `check_mode.mjs` parses.
2. **`check_mode.mjs`, default** — `check-mode-default.txt`: run
   against a scratch project assembled by
   `tools/bridge/package_bridge.py` from the repo's current film data
   (bridge self-verification 21/21 PASS), plus the project's
   `script.json`/`fonts/`. Output: every §B line present, corrected
   CAPTION AUDIT (12/12), no estimate-grade line, `CHECK OK`,
   **exit 0**.
3. **`check_mode.mjs`, `CAP_CHECK=strict`** — `check-mode-strict.txt`:
   byte-identical to the default run, exit 0 (expected on this film —
   §B, caption row).
4. **Chrome check (inherited)** — `chrome-check.txt`: the predecessor's
   Chrome check on the project: duration, renderer (SwiftShader),
   all 14 shot lines, all 73 caption lines — then **exit 124
   (timeout) during the 0.25 s audit loop**; no audit/CAST/verdict
   lines were captured. The native side of those classes is therefore
   verified against code + telemetry (whose cutouts derivation is
   M6-proven field-identical to Chrome's), not against a complete
   Chrome transcript. Diff of the captured portion:
   `chrome-vs-checkmode-diff.txt` (this seat) — shots 11/14 identical,
   captions 71/73 identical, **0 text diffs** (§E for the three
   differing values).
5. **Stale bridge (inherited + this seat)** —
   `stale-bridge-telemetry.txt`: binary on the project's packaged data
   → `film data: missing calendarYears`, exit 3.
   `adapter-check-stale-project.txt`: adapter check → no verdict line,
   FAILED gate, exit 1. `check-mode-stale.txt` (this seat): check_mode
   against the same stale data → `CHECK FAILED`, **exit 3**.
   `bridge-field-diff.txt` + this seat's re-diff: the project's
   `onoda.json` lacks exactly `calendarDark`, `calendarYears`, `sign`
   (repo-only keys); its captions differ from the repo's only by
   project-only `line`/`width` fields and its lines only by
   project-only `i`/`text` — s/e/text cores are equal. **The era/data
   drift failure mode works as designed on both native checks and is
   itself parity evidence**: where Chrome would happily re-evaluate
   the page, the native gate refuses data it cannot prove current,
   loudly, naming the missing field.
6. **Environment note** — this VM was replaced mid-task (the same
   failure that killed the original seat). The replacement has the
   Vulkan loader but **no ICD at all** (`/etc/vulkan/icd.d/` empty, no
   lavapipe anywhere), so native **frame rendering** is env-blocked
   here (`sheet-mode-step10.log`: binary exit 3, `no wgpu adapter
   found` — the module fails closed and names it). The predecessor's
   native sheet (lavapipe, §A) and this seat's seeded-frame sheet run
   cover the sheet legs; a fresh lavapipe/RADV render confirmation
   rides the user's PC session, as the contracts already state for
   the whole wave. Telemetry/check are CPU-only and unaffected.
7. **Adapter check on valid data (contrast)** —
   `adapter-check-rebridged.txt`: adapter on the same scratch project
   → `CHECK OK`, exit 0, but with the **uncorrected** caption audit
   (`0/12 lines authored` + the estimate-grade line for all 12 lines)
   — the artefact `check_mode.mjs` exists to fix, captured side by
   side with `check-mode-default.txt`.

## D. Module status

### `sheet_mode.mjs` — complete; no fix needed

Syntax-checked; `findFont` unit-verified; tiling, geometry, audit
extraction, and fail-closed paths all exercised (§A runs). It carries
no self-test of its own (the only native_modes self-test in the tree
is `era_hint.mjs`'s). Two named divergences, both disclosed in its
header or here: audit sampling is a finer superset than Chrome's
sheet-step sampling; audit-unavailable exits 1 where template Chrome
exits 0 (§A). Neither is a defect by the contract's lights — the
contract asks for the audit's line classes and report-only semantics,
both preserved.

### `check_mode.mjs` — complete after one fix by this seat

- **Defect found and fixed**: `reformatTables` read caption text from
  capture-group `m[11]`; `CAP_ROW` has only 10 groups, so every
  caption's text came out `undefined` and the `captions:` list would
  have printed `undefined` 73 times. The predecessor's runs never
  reached a live tables parse, so it shipped unnoticed. **Fix applied
  in place**: `m[11]` → `m[10]` (one token,
  `template/tools/native_modes/check_mode.mjs`). Verified three ways:
  synthetic Rust-format rows (incl. a `WIDTH+FIT` flag row) parse with
  text intact; the live default run prints all 73 caption texts; the
  Chrome diff shows 0 text differences.
- Everything else verified live (§C.2/3/5): shot/caption/cast
  reformat, authored-count correction, estimate-line suppression,
  verdict re-derivation, exit codes 0/1/3 each observed.
- Header disclosure stands: exit 3 for telemetry-could-not-run is new
  vs the adapter's collapsed 1 — the lead may keep either; the verdict
  line is identical.

## E. Residual gaps (no EXCEPTION proposed)

1. **Shot `note` substitution** — Chrome's shot lines end with the
   authored free-text note from `OP.shots()`; that text exists only
   page-side (`src/shots.ts`) and is not carried by the film data, so
   no native run can print it. `check_mode.mjs` substitutes the shot's
   set/atmosphere from the tables and labels the substitution on every
   line. Closing this fully is bridge work (carry shot notes in the
   bridge data — Batch 1 bridge seat / WP9 P5 territory), not a Chrome
   dependency: **not an EXCEPTION**, a named data gap.
2. **First-shot start** — tables report S01 at **0.24** (the film
   driver's first staged beat) where Chrome reports **0.00**; S01's
   frame range still tiles from frame 0. Report-line cosmetic only.
3. **One boundary, 0.01 s** — the S09/S10 cut prints **45.36** natively
   vs **45.37** in Chrome, and the two captions straddling it differ
   by the same 0.01 s (45.39 vs 45.40). A rounding difference between
   the film data's stored boundary and the page's, on report lines
   only; no frame, caption text, or verdict is affected.
4. **CAP_CHECK fixture gap** — the strict/non-strict verdict
   divergence is real in code (binary hard-codes strict) and the fix
   is implemented, but no current Onoda data can exhibit it (no chunk
   breaches the width/fit bars on the native measure). If the lead
   wants a behavioural fixture, a synthetic over-wide caption in a
   scratch bridge would distinguish the modes in one run.

**Bottom line for the lead**: rows 1 and 4 are REPLACE-NATIVE with the
two modules as delivered (plus the one-token check_mode fix above).
Nothing on either row needs to stay on Chrome, and nothing is proposed
as an EXCEPTION. The two open choices for integration are both
disclosed divergences, not gaps: sheet audit-unavailable exit 1 vs
Chrome's 0, and check exit 3 vs the adapter's collapsed 1.
