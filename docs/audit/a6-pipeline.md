# A6 — Pipeline, Dependencies & QA Inventory (TaleRender Milestone 0)

**Seat:** A6, read-only audit. **Target:** the shipped skill at
`~/workspace/custom-skills/tall-tale-shorts/` (**SKILL.md version 5.0**).
All file names, versions and behaviours below were read from that tree on
2026-11-13. Nothing in the target tree was modified.

Layout facts that frame everything else:

- The skill is two layers: **skill-level** files (`SKILL.md`, `STYLE.md`,
  `CHARTER.md`, `library/`, `scripts/`) and a **template** (`template/`)
  that `scripts/new_video.sh` copies wholesale into each new project
  folder. A project therefore carries its *own copies* of the engine
  (`src/core/`), its shots (`src/shots.ts`), and its tools (`tools/`).
- `scripts/voice.py` and `scripts/words.py` are the exception: they stay
  skill-level and are invoked against a project folder; they are never
  copied into projects (SKILL.md §1).
- Python never runs on system python: everything goes through
  `scripts/py`, a shim that execs the private venv at
  `~/.faceless-shorts/venv` (falling back to system `python3` pre-setup).

---

## 1. The production pipeline as it actually runs

Canonical order (SKILL.md §§3–6, `scripts/selftest.sh`). One project
folder is the working directory for every project-local tool.

| # | Stage | Exact invocation | Inputs | Outputs | Browser? |
|---|-------|------------------|--------|---------|----------|
| 0 | Scaffold | `bash SKILL/scripts/new_video.sh <name>` | `template/` (whole tree copied) | project folder; template's timing files **deleted** (`data/narration.wav`, `mix.wav`, `lines.json`, `words.json`, `words.txt`, `cues.json`) so a fresh project can't be checked against stale timings | No |
| 0b | Estimate (pre-voice, optional but prescribed) | `SKILL/scripts/py SKILL/scripts/estimate.py <project>` | `script.json` | stdout: per-line + total duration from the voice's measured words/sec (`--plan` = line-by-line plan). Can also *write* estimated `data/lines.json`/`words.json` for captions-only/selftest `--no-voice` runs | No |
| 0c | Script lint (Gate 0) | `SKILL/scripts/py tools/script_lint.py .` (project copy) | `script.json` (+ `data/lines.json`/`words.json` in full mode) | stdout verdict; exit 0/non-zero. Pure stdlib Python | No |
| 1 | **Voice** | `SKILL/scripts/py SKILL/scripts/voice.py .` | `script.json` (`voice`, `speed`, `lead`, `outro`, `gap`, optional `pronounce` map); Kokoro model files in `~/.faceless-shorts/kokoro/` | `data/narration.wav` (24 kHz) + `data/lines.json` `{duration, lines:[{i,text,start,end}]}` with exact line times. Lines are grouped into sentences and synthesised in **one pass per sentence**; intra-sentence cuts are located in the audio (pause-snapped), never re-synthesised | No |
| 2 | **Words** | `SKILL/scripts/py SKILL/scripts/words.py .` | `data/narration.wav` + `data/lines.json`; or `--audio <file>` = user's own recording (ffmpeg converts it to `data/narration.wav`, line times rebuilt) | `data/words.json` `[{w, line, s, e}]` (script's spelling, Whisper's times) + `data/words.txt` (eyeball copy). **Fallback:** if Whisper is unavailable, words are length-spread inside exact line times — loudly: banner printed + `data/words.fallback` marker; `FS_WORDS_STRICT=1` makes it exit 2. Refuses (exit 2) if `lines.json` no longer matches `script.json` | No (ffmpeg yes) |
| 3 | **Build** | `bun run build` in the project (package.json script) | `src/main.ts`, `src/sheet.ts`, `src/castsheet.ts` + `src/core/*` + `src/shots.ts` | `build/main.js`, `build/sheet.js`, `build/castsheet.js` — browser-target bundles with `three` and `three/addons/*` **external** (resolved at runtime by the page's importmap to the vendored copy, §2). `tsc -p tsconfig.json` is the separate strict typecheck gate. Without Bun, only the template's shipped prebuilt `build/` runs — edited sources cannot be rebuilt (SKILL.md §0, stated limit) | No |
| 4 | **Cues** | `node tools/capture.mjs cues` | the built page (see §3) | `data/cues.json` `{duration, cues:[…]}` — SFX cues *declared in `src/shots.ts`*, extracted by evaluating `OP.cues()` in the page | **YES** |
| 5 | **Mix** | `SKILL/scripts/py tools/mix.py .` (project copy) | `data/narration.wav`, `data/lines.json`, `data/cues.json`, `script.json` (`music`, `tone`, `"loop": true` opt-in) | `data/mix.wav` (48 kHz master audio): narration + procedural music/SFX (synthesised in numpy inside mix.py) ducked under the voice, loudness-normalised; prints `loudness: PASS/FAIL`, exits non-zero (3) on FAIL. Target **−14 LUFS integrated, true peak ≤ −1 dBTP** | No (ffmpeg yes, for loudnorm measurement + external-track decode) |
| 6 | **Check** | `node tools/capture.mjs check` (Gate 2 form: `CAP_CHECK=strict node tools/capture.mjs check`) | the built page | stdout verdict block (§4); exit 1 on failure | **YES** |
| 7 | **Sheet** | `node tools/capture.mjs sheet review/all.jpg 0.5 9` (`--guides` variant for Gate 3) | the built page | `review/all.jpg` contact sheet (frames every `step` s, tiled `cols` wide by ffmpeg) + the same overflow/composition audit as check, at sheet sample times | **YES** (Chrome for frames, ffmpeg for tiling) |
| 8 | **Frames** (spot QA) | `node tools/capture.mjs frames <t[,t…]> [dir]` | the built page | full-size 1080×1920 JPEGs (q 0.92) in `review/frames/` (default) | **YES** |
| 9 | **Video (master)** | `node tools/capture.mjs video renders/<slug>.mp4 [t0 t1] [--draft]` | the built page + `data/mix.wav` (optional; absent ⇒ silent track) | final MP4: frames piped to ffmpeg `image2pipe` → libx264 **preset slow, CRF 18**, yuv420p, BT.709 tags, AAC 256k @ 48 kHz, `+faststart`. Encoded to `<out>.part.mp4` and **renamed only after ffmpeg exits 0** (kill-safe). `--draft` = veryfast / CRF 23 / AAC 192k | **YES** |
| — | Preview (dev only) | `node tools/capture.mjs preview` | the built page | serves the project on `http://127.0.0.1:8765` for a human to watch in a browser (interactive play button + `data/mix.wav` playback in `main.ts`) | Yes — human-facing only, not a production stage |
| — | Castsheet (library previews) | `node tools/capture.mjs castsheet [outdir] [kind]` | `castsheet.html` + `build/castsheet.js` (the `window.CAST` page API) | per-item `<kind>-<id>.jpg` previews + tiled `castsheet.jpg` — this is how `library/index.json` preview images are produced | **YES** |

**Browser-dependence verdict (verified, not assumed):** only the
capture tool's page modes — `cues`, `check`, `sheet`, `frames`,
`video`, `castsheet`, `preview` — touch a browser. Voice, words,
estimate, lint, build, and mix are browser-free. §3 gives the exact
mechanism. Note the asymmetry TaleRender must respect: **one** stage
family (capture) is the entire browser surface of the pipeline; the
audio half never goes near a page.

**Data-flow shape worth naming:** narration drives everything. Shots
are not timed in seconds by hand — `src/shots.ts` anchors cuts and
captions to *words* via the timing module (`L(i)`/`Le(i)` line
start/end, `Wd('word', i)` word start; `core/timing.ts`). A re-voice
invalidates shot anchoring, which is why SKILL.md §4 step 3 orders a
duration check immediately after words, before shots are written.

---

## 2. Dependency inventory

### 2.1 npm / JavaScript

`template/package.json` (identical at skill root) declares **only
devDependencies — there are no runtime npm dependencies at all:**

| Package | Declared version | Purpose |
|---|---|---|
| `typescript` | `^7.0.2` | The strict typecheck gate (`tsc -p tsconfig.json`, run from the skill-root install by selftest) |
| `@types/three` | `^0.170.0` | Type declarations for three r170 |

- **three itself is vendored, not npm-installed:**
  `template/vendor/three.module.js` (691,648 bytes, minified build,
  embedded REVISION `"170"` → **three r170**) plus
  `template/vendor/addons/` (EffectComposer, RenderPass, ShaderPass,
  MaskPass, OutputPass, UnrealBloomPass, BokehPass; shaders CopyShader,
  LuminosityHighPassShader, BokehShader, OutputShader; environments
  RoomEnvironment) and `vendor/LICENSE-three.txt`. `template/index.html`
  maps `three` → `./vendor/three.module.js` and `three/addons/` →
  `./vendor/addons/` via an importmap; the Bun build marks both
  `--external`, so the bundle never embeds three.
- **node_modules is shared infrastructure:** `scripts/setup.sh` installs
  the skill-root `package.json` once into a shared hub
  (`~/.faceless-shorts/node_modules`) and symlinks it into the skill
  root — one ~60 MB store per machine, not per project.
- `capture.mjs` uses **Node built-ins only** (`node:http`,
  `node:child_process`, `node:fs`, `node:path`, `node:os`, and the
  global `WebSocket` client) — no puppeteer/playwright dependency.
  `capture.ts` (40,729 B) is its TypeScript twin; `capture.mjs`
  (37,897 B) is the file actually executed.

### 2.2 External binaries the pipeline shells out to

| Binary | Used by | For what |
|---|---|---|
| **Chrome/Chromium** (headless) | `capture.mjs` — the *only* consumer | Page host for all capture modes. Discovery order: `$CHROME`, macOS app paths, `/usr/bin/google-chrome(-stable)`, `chromium(-browser)`, snap chromium, Windows paths, then `$FACELESS_HOME/chrome/chrome` (the bundled Chrome), then Brave. Launched `--headless=new --remote-debugging-port=0` with a throwaway profile |
| **ffmpeg** | `capture.mjs` (sheet/castshe
...[truncated 9113 chars]