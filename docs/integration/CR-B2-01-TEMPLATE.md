# CR-B2-01 — Template native-first (Chrome-replacement Batch 2, Seat 01)

**Mission:** make the Tall Tale Shorts skill *template* native-first, so a
fresh project scaffolds the native tooling and a bridged, native-ready
skeleton, and no step of making a film launches a browser.
**Ownership:** new files only; nothing edited in place, nothing deleted,
nothing committed. Deliverables are staged under
`~/workspace/chrome-replacement/batch2/B2-01/` mirroring target paths;
the lead integrates and executes `REMOVALS.md`.
**Skill audited:** `~/workspace/custom-skills/tall-tale-shorts/` v6.0,
template + `scripts/new_video.sh`, verified against the files on disk
(not docs). Contracts: `CHROME-REPLACEMENT-CONTRACTS.md` §1.
**Render proof:** none claimed — this VM has no Vulkan ICD. All checks
below are CPU-side and are reported as such.

---

## 1. Audit — every Chrome touchpoint found in the template / new-project flow

Line refs are to the v6.0 files as audited.

| # | Touchpoint (path + lines) | What it does | Disposition |
|---|---|---|---|
| A1 | `template/tools/capture.mjs` L1, L9–18, L28–40, L43–50, L89–95, L293–295 | The browser capture: browser-binary discovery (`CHROME` env + system/bundled/Brave paths), `CHROME_FLAGS`/`SOFTWARE_GL` handling, headless spawn (`--headless=new`, CDP), HTTP server for the project (L68 ff.), all modes `check·cues·sheet·frames·video·preview·castsheet` | **QUARANTINE OUT of template** (REMOVALS #1). Every mode is REPLACE-NATIVE or retired per contracts rows 1–7. Stays only as named legacy outside the template. |
| A2 | `template/tools/capture.ts` L1, L13–22, L32–44, L100–107, L316–318 | Bun port of A1, identical browser dependency | **QUARANTINE OUT** (REMOVALS #2; contracts row 8). |
| A3 | `template/tools/perf_probe.mjs` L7, L97–105, L115–121, L145–146 | Browser-page draw-cost probe: own discovery, headless spawn, hard-fails without a browser | **QUARANTINE OUT** (REMOVALS #3; contracts row 9 — native equivalent is the repo WP6 bench). |
| A4 | `template/index.html` (whole file) | Film page served to A1's browser (`build/main.js`, `?capture` class) | **QUARANTINE OUT** (REMOVALS #4). No native mode reads it; the adapter renders from `native/` film data. |
| A5 | `template/sheet.html` (whole file) | Style-sheet page for A1 | **QUARANTINE OUT** (REMOVALS #5). Native `sheet` mode tiles native frames instead. |
| A6 | `template/castsheet.html` (whole file) | Cast-sheet page; A1's castsheet mode requires it by name (capture.mjs L332) | **QUARANTINE OUT** (REMOVALS #6). Native castsheet = `native_modes/castsheet_mode.mjs` + `castsheet_native.py`. |
| A7 | `capture.mjs preview` mode (capture.mjs L8, L329–330) | Starts a static server, prints a URL for a human browser; launches no browser itself | **RETIRED** (contracts row 7). Already retired in the Batch-1 adapter: `capture_native.mjs preview` exits 1 naming sheet/frames/video as the review deliverables. No staged change needed. |
| A8 | `template/tools/capture_native.mjs` L14–16, L57, L99 (Batch-1 adapter, retained) | Adapter is native-only at runtime, but its header/error text still presented the browser file as "the default backend" and an unbridged project as "captured with Chrome (tools/capture.mjs)" | **STAGED WORDING FIX** — 3 text spots only, logic untouched (diff verified: exactly those 3 spots). New text: this adapter *is* the production backend; an unbridged project "has no capture until the bridge step runs (see native/README.md)". |
| A9 | `template/tools/CAPTURE-NATIVE.md` Selection section | Same stale framing as A8 ("`capture.mjs` (Chrome) is the default backend… re-run via `capture.mjs`") | **STAGED WORDING FIX** — Selection bullets rewritten: native is the only production backend, template ships no browser capture file, no fallback exists. Historical comparison passages elsewhere in the doc left as history. |
| A10 | `scripts/new_video.sh` (v6.0, whole file) | Scaffold flow itself: copies the template wholesale (so A1–A6 land in every fresh project), runs `bun run build`, prints "before the first capture…" guidance that assumes the browser flow | **STAGED REPLACEMENT** — see §3. The v6.0 script contains no literal browser reference of its own; its defect is that it *distributes* A1–A6 and never scaffolds or mentions the native bridge. |
| A11 | `template/tools/qa_visual.py` L17–37, L147, L182–188 | QA helper whose determinism / pack-verify legs invoke the project's own `tools/capture.mjs` by hard-coded path | **NOT FIXED BY THIS SEAT — named INCOMPLETE / EXCEPTION-candidate** (see §4). It will break the day A1 leaves the template. |
| A12 | `template/tools/qa-pack.json` | Regression pack consumed by A11; golden entry pins a browser-era sheet MD5 (`c535024d…`) | **NOT FIXED — same flag as A11.** Data file; re-pinning belongs with A11's routing fix. |
| A13 | `template/src/main.ts` L1–3 (comment) | "window.OP is the API tools/capture.mjs drives" — comment in a retained authoring source | **RETAINED, docs-sweep item.** Not a runtime touchpoint: nothing in the native pipeline executes this file in a browser (the bridge transcribes `src/`). Listed so the final grep audit classifies it knowingly, not silently. |
| A14 | `template/build/main.js`, `build/sheet.js` | Prebuilt bundles for A4–A6 | **KEEP, flagged** (REMOVALS #7). Contracts row 19 keeps `bun run build` NOT-CHROME/unchanged; with the pages gone the bundles have no production consumer. Removing the build step is the lead's separate decision — it also type-validates the authoring source. Not taken here. |

**Verified non-touchpoints (checked, not assumed):**
`template/src/**` apart from A13 (authoring/transcription source —
`castsheet_native.py` parses `castsheet.ts`/`shots.ts`, the bridge
transcribes `shots.ts`); `template/tools/*.py` other than A11
(audio/discover/lint tooling, no browser); `data/`, `fonts/`,
`vendor/` (build-time three.js for the retained TS source),
`tests/`, `script.json`, `package.json`, `tsconfig.json`.
Skill-level `scripts/setup.sh` (browser check, L31–32) and
`scripts/selftest.sh` (browser capture legs, L24–32) are real
touchpoints but belong to other Batch-2 seats (contracts rows
10–11), not this seat's template/new_video charter — named here so
they are not lost.

## 2. What a fresh project now contains

Staged template additions/changes (all under the batch dir,
`template/` mirrors the skill template):

- `tools/capture_native.mjs` — Batch-1 adapter with the A8 wording
  fix (3 spots, logic byte-identical otherwise).
- `tools/CAPTURE-NATIVE.md` — contract doc with the A9 Selection fix.
- `tools/native_modes/*.mjs` (7 files), `tools/cues_native.py`,
  `tools/castsheet_native.py` — **byte-identical** to the Batch-1
  wiring already in the v6.0 template (sha256 verified for the two
  Python files: cues `0d35ff62…`, castsheet `9d7c9d90…`; the modes
  were copied, not modified). Staged so the lead can verify the
  native set ships complete from this batch.
- `native/` — the bridged-ready skeleton: `README.md` (the exact
  layout the adapter discovers, verified against
  `capture_native.mjs` `findManifest`/`findFilmDir`:
  `film.json` or `scenes/film/film.json`; `film/onoda.json` +
  `film/assets/`), `film.json.example` (manifest shape with
  placeholder values, `.example`-suffixed so the adapter — which
  only reads `film.json` — can never mistake it for a bridge),
  `scenes/film/.gitkeep`, `film/assets/.gitkeep`. **No fake
  `film.json` is scaffolded**: a manifest asserts duration/frames/
  tiling that only exist after voice timing + transcription.
- `README.md` (project root) — what a fresh project contains and the
  native flow, replacing the absent project readme.
- Everything else in the template is retained unchanged; A1–A6 leave
  via `REMOVALS.md` at integration.

## 3. `scripts/new_video.sh` — staged replacement

Same CLI (`<short-name> [--dir <parent>]`). Behaviour changes:

1. After copying the template, it **strips the six quarantined
   files** (A1–A6 paths) from the copy and **fails the scaffold**
   if any survives — fresh projects are clean even before the lead
   executes REMOVALS.md.
2. It **asserts the full native tooling set** (adapter, contract
   doc, both Python tools, all 7 `native_modes/` modules) exists in
   the copy; a template missing any piece fails at scaffold time
   with the file named.
3. It creates the **native bridge skeleton**
   (`native/scenes/film/`, `native/film/assets/`,
   `renders/native-frames/`) and writes `native/BRIDGE-STATUS.txt`
   marking the project **UNBRIDGED**.
4. The v5 P-05 timing-set strip and the Bun authoring build are
   carried over unchanged (build = TypeScript compilation,
   contracts row 19).
5. The printed next-steps are the native flow only: voice/words →
   bridge (`talerender` repo `tools/bridge/build_bridge.py`) →
   `python3 tools/cues_native.py .` → `capture_native.mjs check` →
   `sheet` / `castsheet` → `video` (master still **x264 CRF 18**),
   plus `TALERENDER_BIN` provisioning.
6. The script text contains **zero** occurrences of the browser's
   name or its env vars (grep-verified, §5) — it names the
   quarantined files by path only, which quarantine enforcement
   requires.

## 4. What could NOT be made native-first by this seat (named, with reason)

1. **`tools/qa_visual.py` + `tools/qa-pack.json` (A11/A12) —
   INCOMPLETE / EXCEPTION-candidate.** Its determinism and
   pack-verify legs hard-code `tools/capture.mjs`. Routing it
   native (its frame runs become adapter `frames` runs; its golden
   becomes the native golden pack) is a QA-seat change with its own
   parity questions, outside this seat's template/scaffold charter.
   It must be routed or retired **before the default flip**; until
   then it is a known-broken-on-removal file, named here and in
   REMOVALS.md — never silently kept.
2. **The bridge builder is not in the template.** A fresh project
   scaffolds the bridge *skeleton* and the exact layout contract,
   but building `native/` still runs from the TaleRender repo
   (`tools/bridge/build_bridge.py`, CR-B1-06) after per-project
   scene transcription. Shipping a skill-side bridge builder is
   Batch-1/WP9 follow-up territory, not something this seat could
   fabricate. Consequence, stated plainly in the scaffold output
   and `native/README.md`: a fresh project is UNBRIDGED and capture
   refuses until that external step runs. This is a workflow gap,
   not a browser fallback — the refusal path was verified live (§5).
3. **`build/` bundles + the `bun run build` step (A14)** — kept per
   contracts row 19; their only consumers were the quarantined
   pages. Flagged for the lead's decision, not unilaterally
   removed.

No other EXCEPTION is claimed: every production capture mode
(check/frames/sheet/video/castsheet/cues) has a native path in the
staged template, and `preview` is retired by design (row 7).

## 5. Checks run (CPU-side only — no render proof claimed)

| Check | Result |
|---|---|
| `bash -n` on staged `scripts/new_video.sh` | PASS |
| `grep -ci chrome` on staged `new_video.sh` | 0 matches |
| `node --check` on staged `capture_native.mjs` + all 7 `native_modes/*.mjs` | PASS (8/8) |
| `python3 -m py_compile` on staged `cues_native.py`, `castsheet_native.py` | PASS |
| `film.json.example` JSON validity + `format`/`version` fields | PASS (`talerender-film`, v1) |
| Diff of staged adapter vs v6.0 adapter | Exactly the 3 intended wording spots (A8); no logic change |
| sha256 of staged Python tools vs v6.0 template | Identical (cues `0d35ff62…`, castsheet `9d7c9d90…`) |
| **Scaffold proof**: test skill in `/tmp` = v6.0 template + staged overlay; ran staged `new_video.sh b2-01-proof` (Bun build ran, 20 modules) | Project produced; all 6 quarantined files **absent**; all 11 asserted native files **present**; `native/` skeleton + `BRIDGE-STATUS.txt` (UNBRIDGED) present; template timing set stripped from `data/` |
| Unbridged refusal, staged adapter in the scaffolded project: `RENDERER=native node tools/capture_native.mjs check` | Fails closed naming both manifest paths, with the new B2-01 wording ("has no capture until the bridge step runs (see native/README.md)") — no browser suggestion anywhere in the output |
| Unbridged refusal, cues: `python3 tools/cues_native.py .` in the scaffolded project | Fails loudly naming the bridge step and the missing manifest paths |

## 6. Staged deliverables

```
~/workspace/chrome-replacement/batch2/B2-01/
  REMOVALS.md                     quarantine list for the lead (7 entries, §1 A1–A6 + build flag)
  scripts/new_video.sh            native-first scaffold (§3)
  template/README.md              fresh-project readme (§2)
  template/native/README.md       bridge skeleton contract
  template/native/film.json.example
  template/native/scenes/film/.gitkeep
  template/native/film/assets/.gitkeep
  template/tools/capture_native.mjs   (A8 wording fix only)
  template/tools/CAPTURE-NATIVE.md    (A9 wording fix only)
  template/tools/native_modes/*.mjs   (7 files, byte-identical)
  template/tools/cues_native.py       (byte-identical)
  template/tools/castsheet_native.py  (byte-identical)
```

Integration notes for the lead: (a) execute REMOVALS.md before or
with applying the staged overlay — the staged `new_video.sh` is safe
either way; (b) the template's own `native/` skeleton only takes
effect for projects scaffolded after integration; existing projects
need the P1 retrofit (contracts row 17, another seat); (c) A11/A12
need an owner before the flip.
