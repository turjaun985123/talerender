# CR-B3-09 — Final Chrome-Reference Audit (both repos)

Seat B3-09, Chrome-Replacement Batch 3. Read-only audit; nothing committed.
Full machine-readable table + summary: `~/workspace/chrome-replacement/batch3/B3-09/`
(`audit_table.csv`, `audit_table.json`, `summary.md`, `commands.txt`, `raw_skill.txt`, `raw_talerender.txt`).

**VERDICT: BLOCKED (strict charter) — production surface clean.** 1628 total hits
(skill 356, talerender 1272). OPERATIONAL outside `legacy/`: **144 lines** (skill STYLE.md 5;
talerender fog-chart vendored Chrome project 113; talerender docs/ab Chrome harness scripts 26) —
none reachable from the skill template, skill scripts, selftest, scaffolding, or any of the 10 CI
workflows (all verified browser-free; spawn targets are only talerender/ffmpeg/node/python).
Classes: OPERATIONAL 144 · RETIRED-LEGACY 125 · NEGATIVE-GUARD 138 · CONTRACT-COMMENT 1220 ·
FALSE-POSITIVE 1 (repo-root binary: `monochrome`/`chroma` strings). Disposition required before a
clean flip declaration: rewrite STYLE.md's five passages to capture_native.mjs; quarantine or delete
`tools/fog-chart/chrome/`; retire or quarantine the `docs/ab/` harness scripts. Detail, counts,
reproduction commands and the executable-surface table are in the B3-09 summary.

## Appendix — every OPERATIONAL hit, quoted in full (repo :: file:line :: text)

```
skill :: STYLE.md:70 :: | `tools/capture.mjs` / `tools/capture.ts` | capture in both runtimes: check mode prints the `CAST: …` line and the `CONTINUITY: clean (N named characters tracked)` / `CONTINUITY VIOLATION: …` report lines (§12 — violations fail the check); sheet mode accepts `--guides` (rule-of-thirds grid + caption-zone overlay on every tile, §12) |
skill :: STYLE.md:105 ::   GPU ANGLE flags on **by default** when a GPU is present — you no longer pass `CHROME_FLAGS` by hand; set it
skill :: STYLE.md:257 ::   capture tooling runs under both (`bun run tools/capture.mjs` / `node tools/capture.mjs`).
skill :: STYLE.md:261 ::   `node tools/capture.mjs sheet <file> 0.5 9` (0.5 s step, 9 columns). `selftest.sh`'s own sheet
skill :: STYLE.md:480 :: **Sheet `--guides`.** `capture sheet … --guides` (both `capture.mjs` and `capture.ts`) overlays
talerender :: docs/ab/stroke-battery/project/battery.ts:2 :: // the real Canvas2D painter (Skia, in Chrome): the primitive ops use
talerender :: docs/ab/stroke-battery/project/run-reference.sh:2 :: # Reproduce the Chrome/Skia reference cards for the M4 stroke battery.
talerender :: docs/ab/stroke-battery/project/run-reference.sh:5 :: # serves it locally and drives headless Chrome at it; the page POSTs
talerender :: docs/ab/stroke-battery/project/run-reference.sh:11 :: CHROME="${CHROME:-$HOME/.faceless-shorts/chrome/opt/google/chrome/chrome}"
talerender :: docs/ab/stroke-battery/project/run-reference.sh:24 :: "$CHROME" --headless=new --no-sandbox --ignore-gpu-blocklist \
talerender :: docs/ab/stroke-battery/project/run-reference.sh:28 :: CHROME_PID=$!
talerender :: docs/ab/stroke-battery/project/run-reference.sh:30 :: kill $CHROME_PID 2>/dev/null || true
talerender :: docs/ab/stroke-battery/project/battery.html:2 :: <html><head><meta charset="utf-8"><title>M4 stroke battery — Chrome/Skia reference</title></head>
talerender :: docs/ab/telemetry/project/drive.mjs:2 :: // Serves a project root, boots headless Chrome (SwiftShader — the
talerender :: docs/ab/telemetry/project/drive.mjs:6 :: // just drawn (capture.ts check mode's protocol, a2 §3.1).
talerender :: docs/ab/telemetry/project/drive.mjs:9 :: // Writes: <out-dir>/chrome-cutouts.json  (OP.cutouts() at t=1.50)
talerender :: docs/ab/telemetry/project/drive.mjs:10 :: //         <out-dir>/chrome-meta.json     (renderer, duration, captions, logs)
talerender :: docs/ab/telemetry/project/drive.mjs:20 :: const CHROME = process.env.CHROME || path.join(os.homedir(), '.faceless-shorts/chrome/opt/google/chrome/chrome');
talerender :: docs/ab/telemetry/project/drive.mjs:34 :: const prof = fs.mkdtempSync(path.join(os.tmpdir(), 'm6-chrome-'));
talerender :: docs/ab/telemetry/project/drive.mjs:35 :: const chrome = spawn(CHROME, [
talerender :: docs/ab/telemetry/project/drive.mjs:36 ::   '--headless=new', '--remote-debugging-port=0', `--user-data-dir=${prof}`,
talerender :: docs/ab/telemetry/project/drive.mjs:49 :: if (!dbgPort) throw new Error('Chrome did not start');
talerender :: docs/ab/telemetry/project/drive.mjs:85 ::   fs.writeFileSync(path.join(OUT, 'chrome-cutouts.json'), JSON.stringify(cuts, null, 1));
talerender :: docs/ab/telemetry/project/drive.mjs:86 ::   fs.writeFileSync(path.join(OUT, 'chrome-meta.json'), JSON.stringify({ renderer, duration, captions, cast, logs, t: T }, null, 1));
talerender :: docs/ab/telemetry/project/drive.mjs:91 ::   chrome.kill('SIGKILL');
talerender :: docs/ab/telemetry/project/run-harness.sh:3 :: # variant's Chrome cutouts). Copies the Onoda project to a scratch
talerender :: docs/ab/telemetry/project/run-harness.sh:5 :: # it locally and drives headless Chrome (SwiftShader) at it; the
talerender :: docs/ab/telemetry/project/run-harness.sh:11 :: CHROME="${CHROME:-$HOME/.faceless-shorts/chrome/opt/google/chrome/chrome}"
talerender :: docs/ab/telemetry/project/run-harness.sh:23 :: "$CHROME" --headless=new --no-sandbox --ignore-gpu-blocklist \
talerender :: docs/ab/telemetry/project/run-harness.sh:27 :: CHROME_PID=$!
talerender :: docs/ab/telemetry/project/run-harness.sh:29 :: kill $CHROME_PID 2>/dev/null || true
talerender :: tools/fog-chart/chrome/shots.ts:8 :: // them, this Chrome capture is reference (b) (PSNR >= 45 dB, centres only —
talerender :: tools/fog-chart/chrome/project/src/core/board.ts:2 :: // matches what renders. Used by sheet.html -> node tools/capture.mjs stylesheet ../style-sheet.png
talerender :: tools/fog-chart/chrome/project/src/castsheet.ts:3 :: // drawToon — so `tools/capture.mjs castsheet` can render every library preview
talerender :: tools/fog-chart/chrome/project/src/main.ts:3 :: // (labels, counters) and the captions go on top. window.OP is the API tools/capture.mjs drives.
talerender :: tools/fog-chart/chrome/project/src/main.ts:63 ::     // WP2: per-toon audit data for tools/capture.mjs — canvas overflow (painted
talerender :: tools/fog-chart/chrome/project/src/main.ts:68 ::     // converts `box` into the drawn figure's box (figBox in capture.ts — fixed
talerender :: tools/fog-chart/chrome/project/src/shots.ts:8 :: // them, this Chrome capture is reference (b) (PSNR >= 45 dB, centres only —
talerender :: tools/fog-chart/chrome/project/tools/capture.mjs:1 :: // Frame-exact capture through headless Chrome. No npm packages (Node 22+). Run from the project folder.
talerender :: tools/fog-chart/chrome/project/tools/capture.mjs:2 :: //   node tools/capture.mjs check                         boot, list shots + captions, report errors, cast + continuity report
talerender :: tools/fog-chart/chrome/project/tools/capture.mjs:3 :: //   node tools/capture.mjs cues                          write data/cues.json (the sound cues, for tools/mix.py)
talerender :: tools/fog-chart/chrome/project/tools/capture.mjs:4 :: //   node tools/capture.mjs sheet review/all.jpg [step=0.5] [cols=9] [--guides]   contact sheet, a frame every `step` s, time-stamped; --guides overlays a grid + caption zone
talerender :: tools/fog-chart/chrome/project/tools/capture.mjs:5 :: //   node tools/capture.mjs frames review/full "1.2,8.4"  full-size JPEGs of those times
talerender :: tools/fog-chart/chrome/project/tools/capture.mjs:6 :: //   node tools/capture.mjs castsheet <outdir> [kind]     library previews: one staged frame per pose/mood/look/held prop as <kind>-<id>.jpg, plus a labelled castsheet.jpg contact sheet ([kind] renders one kind only)
talerender :: tools/fog-chart/chrome/project/tools/capture.mjs:7 :: //   node tools/capture.mjs video renders/out.mp4 [start end] [--draft]   the film (30 fps) with data/mix.wav if it exists
talerender :: tools/fog-chart/chrome/project/tools/capture.mjs:8 :: //   node tools/capture.mjs preview                       serve the project and print a URL to watch it in a browser
talerender :: tools/fog-chart/chrome/project/tools/capture.mjs:9 :: // CHROME=/path/to/chrome overrides the browser. Without it, discovery tries (in order) the
talerender :: tools/fog-chart/chrome/project/tools/capture.mjs:10 :: // system Chrome/Chromium/Edge installs, then the faceless-shorts bundled Chrome
talerender :: tools/fog-chart/chrome/project/tools/capture.mjs:11 :: // ($FACELESS_HOME/chrome/chrome, default ~/.faceless-shorts/chrome/chrome), then Brave.
talerender :: tools/fog-chart/chrome/project/tools/capture.mjs:12 :: // CHROME_FLAGS="--no-sandbox --use-gl=angle --use-angle=gl"  extra Chrome flags, split on
talerender :: tools/fog-chart/chrome/project/tools/capture.mjs:18 :: //   the default and forces the software path; never combine it with CHROME_FLAGS.
talerender :: tools/fog-chart/chrome/project/tools/capture.mjs:28 :: const CHROME = [process.env.CHROME, '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome', '/Applications/Chromium.app/Contents/MacOS/Chromium',
talerender :: tools/fog-chart/chrome/project/tools/capture.mjs:29 ::   '/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge', '/usr/bin/google-chrome', '/usr/bin/google-chrome-stable', '/usr/bin/chromium',
talerender :: tools/fog-chart/chrome/project/tools/capture.mjs:30 ::   '/usr/bin/chromium-browser', '/snap/bin/chromium', 'C:/Program Files/Google/Chrome/Application/chrome.exe',
talerender :: tools/fog-chart/chrome/project/tools/capture.mjs:31 ::   'C:/Program Files (x86)/Google/Chrome/Application/chrome.exe',
talerender :: tools/fog-chart/chrome/project/tools/capture.mjs:33 ::   // only rescue machines that found none: the ecosystem's own bundled Chrome (setup.sh's install
talerender :: tools/fog-chart/chrome/project/tools/capture.mjs:34 ::   // spot, $FACELESS_HOME/chrome/chrome) and Brave (a Brave-only machine previously got
talerender :: tools/fog-chart/chrome/project/tools/capture.mjs:35 ::   // 'Chrome not found' even though setup.sh already knew these paths).
talerender :: tools/fog-chart/chrome/project/tools/capture.mjs:36 ::   path.join(process.env.FACELESS_HOME || path.join(os.homedir(), '.faceless-shorts'), 'chrome', 'chrome'),
talerender :: tools/fog-chart/chrome/project/tools/capture.mjs:43 :: if (SOFTWARE_GL && process.env.CHROME_FLAGS) console.warn('capture: WARNING — SOFTWARE_GL and CHROME_FLAGS are both set. SOFTWARE_GL forces SwiftShader (software/CPU rendering) while CHROME_FLAGS is meant to select the GPU; the two conflict — unset one of them.');
talerender :: tools/fog-chart/chrome/project/tools/capture.mjs:48 :: const ENV_FLAGS = String(process.env.CHROME_FLAGS ?? '').split(/\s+/).filter(Boolean);
talerender :: tools/fog-chart/chrome/project/tools/capture.mjs:50 :: const CHROME_FLAGS_EFF = GPU_DEFAULTED ? GPU_DEFAULT_FLAGS.split(/\s+/) : ENV_FLAGS;
talerender :: tools/fog-chart/chrome/project/tools/capture.mjs:89 ::   if (!CHROME) throw new Error('Chrome not found: install Google Chrome or set CHROME=/path/to/chrome');
talerender :: tools/fog-chart/chrome/project/tools/capture.mjs:90 ::   const prof = fs.mkdtempSync(path.join(os.tmpdir(), 'fs-chrome-'));
talerender :: tools/fog-chart/chrome/project/tools/capture.mjs:91 ::   const chrome = spawn(CHROME, ['--headless=new', '--remote-debugging-port=0', `--user-data-dir=${prof}`, '--no-first-run', '--no-default-browser-check',
talerender :: tools/fog-chart/chrome/project/tools/capture.mjs:92 ::     '--hide-scrollbars', '--window-size=1080,1920', '--ignore-gpu-blocklist', '--enable-unsafe-swiftshader', ...(SOFTWARE_GL ? ['--use-angle=swiftshader'] : []), ...CHROME_FLAGS_EFF, ...(process.getuid?.() === 0 ? ['--no-sandbox'] : []), '--disable-background-timer-throttling', '--disable-renderer-backgrounding', 'about:blank'], { stdio: 'ignore' });
talerender :: tools/fog-chart/chrome/project/tools/capture.mjs:95 ::   if (!dbg) throw new Error('Chrome did not start');
talerender :: tools/fog-chart/chrome/project/tools/capture.mjs:117 ::   const close = () => { try { ws.close(); } catch {} chrome.kill('SIGKILL'); setTimeout(() => { try { fs.rmSync(prof, { recursive: true, force: true }); } catch {} }, 300); };
talerender :: tools/fog-chart/chrome/project/tools/capture.mjs:293 ::       if (GPU_DEFAULTED) console.log(`*** WARNING: SOFTWARE RENDERER *** "${renderer}" indicates SwiftShader/llvmpipe even though the GPU default flags were applied (/dev/dri present) — frames will render on the CPU (~3 s/frame, hot machine) instead of the GPU (~0.15 s/frame). Check the GPU/driver (or set CHROME_FLAGS explicitly) before rendering.`);
talerender :: tools/fog-chart/chrome/project/tools/capture.mjs:294 ::       else if (process.env.CHROME_FLAGS) console.log(`*** WARNING: SOFTWARE RENDERER *** "${renderer}" indicates SwiftShader/llvmpipe even though CHROME_FLAGS is set — frames will render on the CPU (~3 s/frame, hot machine) instead of the GPU (~0.15 s/frame). Check CHROME_FLAGS and the GPU/driver before rendering.`);
talerender :: tools/fog-chart/chrome/project/tools/capture.mjs:295 ::       else console.log(`*** WARNING: SOFTWARE RENDERER *** "${renderer}" indicates SwiftShader/llvmpipe — frames will render on the CPU (~3 s/frame, hot machine) instead of the GPU (~0.15 s/frame). On a Linux GPU machine capture normally applies the ANGLE/GL flags by itself when /dev/dri is present (CHROME_FLAGS overrides, SOFTWARE_GL=1 forces software); on a GPU-less machine this warning is expected for QA captures.`);
talerender :: tools/fog-chart/chrome/project/tools/capture.ts:1 :: // Frame-exact capture through headless Chrome. No npm packages. Run from the project folder.
talerender :: tools/fog-chart/chrome/project/tools/capture.ts:2 :: // TypeScript port of tools/capture.mjs (same logic, same CLI, same env) — light, honest types
talerender :: tools/fog-chart/chrome/project/tools/capture.ts:4 :: // run with Bun:  bun run tools/capture.ts check
talerender :: tools/fog-chart/chrome/project/tools/capture.ts:5 :: // (capture.mjs under Node remains the supported fallback:  node tools/capture.mjs check)
talerender :: tools/fog-chart/chrome/project/tools/capture.ts:13 :: // CHROME=/path/to/chrome overrides the browser. Without it, discovery tries (in order) the
talerender :: tools/fog-chart/chrome/project/tools/capture.ts:14 :: // system Chrome/Chromium/Edge installs, then the faceless-shorts bundled Chrome
talerender :: tools/fog-chart/chrome/project/tools/capture.ts:15 :: // ($FACELESS_HOME/chrome/chrome, default ~/.faceless-shorts/chrome/chrome), then Brave.
talerender :: tools/fog-chart/chrome/project/tools/capture.ts:16 :: // CHROME_FLAGS="--no-sandbox --use-gl=angle --use-angle=gl"  extra Chrome flags, split on
talerender :: tools/fog-chart/chrome/project/tools/capture.ts:22 :: //   the default and forces the software path; never combine it with CHROME_FLAGS.
talerender :: tools/fog-chart/chrome/project/tools/capture.ts:32 :: const CHROME = [process.env.CHROME, '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome', '/Applications/Chromium.app/Contents/MacOS/Chromium',
talerender :: tools/fog-chart/chrome/project/tools/capture.ts:33 ::   '/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge', '/usr/bin/google-chrome', '/usr/bin/google-chrome-stable', '/usr/bin/chromium',
talerender :: tools/fog-chart/chrome/project/tools/capture.ts:34 ::   '/usr/bin/chromium-browser', '/snap/bin/chromium', 'C:/Program Files/Google/Chrome/Application/chrome.exe',
talerender :: tools/fog-chart/chrome/project/tools/capture.ts:35 ::   'C:/Program Files (x86)/Google/Chrome/Application/chrome.exe',
talerender :: tools/fog-chart/chrome/project/tools/capture.ts:37 ::   // only rescue machines that found none: the ecosystem's own bundled Chrome (setup.sh's install
talerender :: tools/fog-chart/chrome/project/tools/capture.ts:38 ::   // spot, $FACELESS_HOME/chrome/chrome) and Brave (a Brave-only machine previously got
talerender :: tools/fog-chart/chrome/project/tools/capture.ts:39 ::   // 'Chrome not found' even though setup.sh already knew these paths).
talerender :: tools/fog-chart/chrome/project/tools/capture.ts:40 ::   path.join(process.env.FACELESS_HOME || path.join(os.homedir(), '.faceless-shorts'), 'chrome', 'chrome'),
talerender :: tools/fog-chart/chrome/project/tools/capture.ts:47 :: if (SOFTWARE_GL && process.env.CHROME_FLAGS) console.warn('capture: WARNING — SOFTWARE_GL and CHROME_FLAGS are both set. SOFTWARE_GL forces SwiftShader (software/CPU rendering) while CHROME_FLAGS is meant to select the GPU; the two conflict — unset one of them.');
talerender :: tools/fog-chart/chrome/project/tools/capture.ts:52 :: const ENV_FLAGS = String(process.env.CHROME_FLAGS ?? '').split(/\s+/).filter(Boolean);
talerender :: tools/fog-chart/chrome/project/tools/capture.ts:54 :: const CHROME_FLAGS_EFF = GPU_DEFAULTED ? GPU_DEFAULT_FLAGS.split(/\s+/) : ENV_FLAGS;
talerender :: tools/fog-chart/chrome/project/tools/capture.ts:100 ::   if (!CHROME) throw new Error('Chrome not found: install Google Chrome or set CHROME=/path/to/chrome');
talerender :: tools/fog-chart/chrome/project/tools/capture.ts:101 ::   const prof = fs.mkdtempSync(path.join(os.tmpdir(), 'fs-chrome-'));
talerender :: tools/fog-chart/chrome/project/tools/capture.ts:103 ::   const chrome = spawn(CHROME, ['--headless=new', '--remote-debugging-port=0', `--user-data-dir=${prof}`, '--no-first-run', '--no-default-browser-check',
talerender :: tools/fog-chart/chrome/project/tools/capture.ts:104 ::     '--hide-scrollbars', '--window-size=1080,1920', '--ignore-gpu-blocklist', '--enable-unsafe-swiftshader', ...(SOFTWARE_GL ? ['--use-angle=swiftshader'] : []), ...CHROME_FLAGS_EFF, ...(getuid?.() === 0 ? ['--no-sandbox'] : []), '--disable-background-timer-throttling', '--disable-renderer-backgrounding', 'about:blank'], { stdio: 'ignore' });
talerender :: tools/fog-chart/chrome/project/tools/capture.ts:107 ::   if (!dbg) throw new Error('Chrome did not start');
talerender :: tools/fog-chart/chrome/project/tools/capture.ts:130 ::   const close = () => { try { ws.close(); } catch {} chrome.kill('SIGKILL'); setTimeout(() => { try { fs.rmSync(prof, { recursive: true, force: true }); } catch {} }, 300); };
talerender :: tools/fog-chart/chrome/project/tools/capture.ts:316 ::       if (GPU_DEFAULTED) console.log(`*** WARNING: SOFTWARE RENDERER *** "${renderer}" indicates SwiftShader/llvmpipe even though the GPU default flags were applied (/dev/dri present) — frames will render on the CPU (~3 s/frame, hot machine) instead of the GPU (~0.15 s/frame). Check the GPU/driver (or set CHROME_FLAGS explicitly) before rendering.`);
talerender :: tools/fog-chart/chrome/project/tools/capture.ts:317 ::       else if (process.env.CHROME_FLAGS) console.log(`*** WARNING: SOFTWARE RENDERER *** "${renderer}" indicates SwiftShader/llvmpipe even though CHROME_FLAGS is set — frames will render on the CPU (~3 s/frame, hot machine) instead of the GPU (~0.15 s/frame). Check CHROME_FLAGS and the GPU/driver before rendering.`);
talerender :: tools/fog-chart/chrome/project/tools/capture.ts:318 ::       else console.log(`*** WARNING: SOFTWARE RENDERER *** "${renderer}" indicates SwiftShader/llvmpipe — frames will render on the CPU (~3 s/frame, hot machine) instead of the GPU (~0.15 s/frame). On a Linux GPU machine capture normally applies the ANGLE/GL flags by itself when /dev/dri is present (CHROME_FLAGS overrides, SOFTWARE_GL=1 forces software); on a GPU-less machine this warning is expected for QA captures.`);
talerender :: tools/fog-chart/chrome/project/tools/mix.py:6 :: Reads data/cues.json (node tools/capture.mjs cues), data/narration.wav if it exists (voice.py or words.py --audio),
talerender :: tools/fog-chart/chrome/project/tools/mix.py:29 :: cj = _load_json('data/cues.json', 'run `node tools/capture.mjs cues` first')
talerender :: tools/fog-chart/chrome/project/tools/mix.py:31 ::     print('mix.py: data/cues.json has no "duration"/"cues" — regenerate with `node tools/capture.mjs cues`'); sys.exit(2)
talerender :: tools/fog-chart/chrome/verify_constants.py:10 ::   C. the Chrome staging (chrome/shots.ts) constants and construction,
talerender :: tools/fog-chart/chrome/verify_constants.py:20 ::   python3 tools/fog-chart/chrome/verify_constants.py
talerender :: tools/fog-chart/chrome/verify_constants.py:29 :: HERE = Path(__file__).resolve().parent            # tools/fog-chart/chrome
talerender :: tools/fog-chart/chrome/verify_constants.py:162 :: # ---- C. Chrome staging (shots.ts source-text assertions) -----------------------
talerender :: tools/fog-chart/chrome/verify_constants.py:165 ::     rec("C chrome", label, spec_desc, staged_desc or ("present" if ok else "ABSENT"), ok)
talerender :: tools/fog-chart/chrome/verify_constants.py:205 :: rec("C chrome", "engine camera() defaults (engine.ts)", "PerspectiveCamera(fov, W/H, 0.05, 800)",
talerender :: tools/fog-chart/chrome/sample_centres.py:9 :: the analytic column; a Chrome capture is reference (b) — this script
talerender :: tools/fog-chart/chrome/sample_centres.py:11 :: judges Chrome by the basic panels' centres and, once a native frame
talerender :: tools/fog-chart/chrome/sample_centres.py:14 ::   python3 tools/fog-chart/chrome/sample_centres.py <frame.png|frame.jpg>
talerender :: tools/fog-chart/chrome/VERIFY.md:1 :: # Fog chart — Chrome harness assembly + verification record (seat W66)
talerender :: tools/fog-chart/chrome/VERIFY.md:6 :: verification that the Chrome staging (`shots.ts`), the native scene
talerender :: tools/fog-chart/chrome/VERIFY.md:9 :: headless-Chrome run of the harness on this VM with its panel-centre
talerender :: tools/fog-chart/chrome/VERIFY.md:15 ::   `python3 tools/fog-chart/chrome/verify_constants.py`, exit 0).
talerender :: tools/fog-chart/chrome/VERIFY.md:16 :: - Chrome run: frame captured (renderer string and hashes in §3).
talerender :: tools/fog-chart/chrome/VERIFY.md:20 ::   the Chrome side implements is three's smoothstep, confirmed
talerender :: tools/fog-chart/chrome/VERIFY.md:26 ::   already declares the analytic column as the gate and the Chrome
talerender :: tools/fog-chart/chrome/VERIFY.md:34 :: capture with `tools/capture.mjs frames`.
talerender :: tools/fog-chart/chrome/VERIFY.md:38 ::           +  src/shots.ts    <- chrome/shots.ts    (drop-in)
talerender :: tools/fog-chart/chrome/VERIFY.md:39 ::           +  script.json     <- chrome/script.json (drop-in)
talerender :: tools/fog-chart/chrome/VERIFY.md:70 ::   in §2, which are run against the canonical `chrome/shots.ts`).
talerender :: tools/fog-chart/chrome/VERIFY.md:88 :: - **C — Chrome staging, source-text assertions.** The constants and
talerender :: tools/fog-chart/chrome/VERIFY.md:173 :: ### C chrome
talerender :: tools/fog-chart/chrome/VERIFY.md:269 :: The M3 pattern demonstrably ran headless Chrome on this VM — the
talerender :: tools/fog-chart/chrome/VERIFY.md:275 :: cd tools/fog-chart/chrome/project
talerender :: tools/fog-chart/chrome/VERIFY.md:277 :: CHROME=/home/hatch/.faceless-shorts/chrome/opt/google/chrome/chrome \
talerender :: tools/fog-chart/chrome/VERIFY.md:278 ::   node tools/capture.mjs frames review/full "1.0"
talerender :: tools/fog-chart/chrome/VERIFY.md:292 ::   failed with `Chrome did not start` — the tool polls 20 s for the
talerender :: tools/fog-chart/chrome/VERIFY.md:294 ::   this Chrome build on the VM exceeded it during initialisation.
talerender :: tools/fog-chart/chrome/VERIFY.md:300 :: ## 4. Centre measurements (Chrome frame vs analytic)
talerender :: tools/fog-chart/chrome/VERIFY.md:324 ::   (15 m) codes; the Chrome side lands within 1. The 25 m panel is
talerender :: tools/fog-chart/chrome/VERIFY.md:347 ::   The effective Chrome factor measures as house/π — consistent
talerender :: tools/fog-chart/chrome/VERIFY.md:353 ::   `chrome/`): if the Chrome standard panels are ever meant to
talerender :: tools/fog-chart/chrome/VERIFY.md:357 ::   the native side against the analytic column, and the Chrome
talerender :: tools/fog-chart/chrome/VERIFY.md:366 ::   native-vs-analytic ±2-code verdict and the native-vs-Chrome
talerender :: tools/fog-chart/chrome/VERIFY.md:368 ::   only the Chrome half and the staging-consistency question.
talerender :: tools/fog-chart/chrome/VERIFY.md:370 ::   staging always sets `scene.fog`); no Chrome no-fog variant was
talerender :: tools/fog-chart/chrome/VERIFY.md:381 :: python3 tools/fog-chart/chrome/verify_constants.py   # 144/144, exit 0
talerender :: tools/fog-chart/chrome/VERIFY.md:382 :: cd tools/fog-chart/chrome/project && bun run build
talerender :: tools/fog-chart/chrome/VERIFY.md:383 :: CHROME=<chrome binary> node tools/capture.mjs frames review/full "1.0"
talerender :: tools/fog-chart/chrome/VERIFY.md:384 :: python3 tools/fog-chart/chrome/sample_centres.py \
talerender :: tools/fog-chart/chrome/VERIFY.md:385 ::   tools/fog-chart/chrome/project/review/full/t1.00.jpg
```
