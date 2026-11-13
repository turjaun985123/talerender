# A2 — Capture & Browser Dependency Inventory

**Scope:** the Chrome/CDP capture pipeline TaleRender must replace.
**Sources (read in full):** `template/tools/capture.ts` (481 lines, the typed reference),
`template/tools/capture.mjs` (457 lines — the untyped twin; same logic, same CLI, same env,
per capture.ts:2–4; both ship and are kept in sync by hand),
`template/index.html` (17 lines), `template/src/main.ts` (147 lines, page-side protocol only),
`template/src/sheet.ts` (100 lines).
Line citations are to capture.ts / main.ts / index.html / sheet.ts unless marked `.mjs`.
**Paths** below are relative to `template/` inside a project folder; `tools/capture.*` resolves
`ROOT` as the template/project root (capture.ts:30).

---

## 1. The capture protocol as implemented

There is **no screenshot API anywhere**. The entire pipeline is: serve the project over local
HTTP, boot headless Chrome, attach CDP over a hand-rolled WebSocket, and drive the page by
evaluating JavaScript strings against a single page global, `window.OP`. Frames leave the page
as **canvas data**, never as browser screenshots.

### 1.1 Boot sequence (capture.ts:70–131)

1. **Static server first** — `serve()` (capture.ts:70–88) starts a Node `http` server on
   `127.0.0.1`, port 0 (ephemeral) or 8765 for preview mode (capture.ts:353). It serves files
   from the project root with a fixed MIME table (.html/.js/.mjs/.json/.otf/.ttf/.woff2/.wav/
   .mp3/.png/.jpg), `Cache-Control: no-store`, a path-traversal guard (`f.startsWith(ROOT)`,
   capture.ts:83), and one non-static route: `POST /__frame`, the raw-frame sink (§1.4).
2. **Chrome discovery** — `CHROME` is the first existing path from an ordered list
   (capture.ts:32–40): `$CHROME` env, macOS Chrome/Chromium/Edge app paths,
   `/usr/bin/google-chrome`, `google-chrome-stable`, `chromium`, `chromium-browser`,
   `/snap/bin/chromium`, two Windows paths, then — appended last in v5 so a discovered
   browser keeps winning — the bundled `$FACELESS_HOME/chrome/chrome`
   (default `~/.faceless-shorts/chrome/chrome`) and four Brave paths. No match →
   `openPage` throws `Chrome not found…` (capture.ts:100).
3. **Chrome spawn** (capture.ts:102–105): fresh temp profile (`fs-chrome-*` in os.tmpdir),
   flags: `--headless=new --remote-debugging-port=0 --user-data-dir=<prof> --no-first-run
   --no-default-browser-check --hide-scrollbars --window-size=1080,1920
   --ignore-gpu-blocklist --enable-unsafe-swiftshader`, then `--use-angle=swiftshader`
   iff SOFTWARE_GL, then the effective CHROME_FLAGS (§1.5), then `--no-sandbox` iff uid==0,
   then `--disable-background-timer-throttling --disable-renderer-backgrounding`, target
   `about:blank`, stdio ignored.
4. **Debug port discovery** — poll the profile's `DevToolsActivePort` file, 200 tries ×
   100 ms ≈ 20 s max (capture.ts:106–107); failure → `Chrome did not start`.
5. **Target creation** — `PUT http://127.0.0.1:<dbg>/json/new?<url-encoded page URL>`
   (capture.ts:108). The page URL is
   `http://127.0.0.1:<port>/<page>?capture=1` + the raw contents of env `QS` appended
   verbatim (host→page query-string passthrough, capture.ts:108).
6. **CDP socket** — a global `WebSocket` to `target.webSocketDebuggerUrl` (capture.ts:110).
   The client is ~15 lines: incrementing ids, a pending-promise map, JSON frames
   (capture.ts:113–120). Only two CDP methods are **ever** sent: `Runtime.enable`
   (capture.ts:122) and `Runtime.evaluate` with `{returnByValue: true, awaitPromise: true}`
   — that pair *is* the whole protocol (the `ev()` helper, capture.ts:121). Two CDP events
   are consumed: `Runtime.exceptionThrown` and `Runtime.consoleAPICalled` with
   `type === 'error'`; both are appended to a `logs[]` array as `EXCEPTION …` / `ERROR …`
   strings (capture.ts:116–118) and later fail check mode (§3.1).
7. **Readiness handshake** — poll every 100 ms, up to 1200 iterations (≈120 s), evaluating
   `window.OP ? (OP.bootError ? "ERR:" + OP.bootError : OP.ready) : false`
   (capture.ts:123–128). `true` → ready; a string → `boot failed: …` + collected logs;
   timeout → `boot timeout` + logs.
8. **Teardown** — `close()`: close WS, `chrome.kill('SIGKILL')`, delete the temp profile
   after 300 ms (capture.ts:130). Every mode runs inside `try { … } finally { P.close();
   srv.close(); }` (capture.ts:361, 479).

### 1.2 The page↔host contract — `window.OP` (main.ts:23–146)

`index.html` provides the stage: a visible 2D canvas `#cv` at exactly **1080×1920**
(index.html:12), a **hidden** WebGL canvas `#gl` (`display:none`, index.html:8,12) of the
same attribute size, `@font-face` Inter 900/800 + Caladea 700 (index.html:4–6), an importmap
pinning `three` to `./vendor/three.module.js` (index.html:15), and `build/main.js` as the
module entry (index.html:16). If the URL contains `capture`, `document.body.className =
'capture'` (index.html:14), which only changes CSS sizing of `#cv`.

`main.ts` creates `window.OP = { ready: false }` immediately (main.ts:26), then boots
asynchronously:

- `await document.fonts.load('900 100px Inter')` and `('800 100px Inter')` — **fonts are
  fully loaded before the first frame can exist** (main.ts:44).
- `fetch('script.json', {cache:'no-store'})` (main.ts:45), timing load, caption build.
- The hidden GL canvas is resized to `1080*RES × 1920*RES` (main.ts:48; `RES` is the
  engine's resolution scale — this is what the `QS` passthrough typically drives), the
  engine is created on it, shots are built, and `E.render` is **wrapped** so every render
  records `OP._cam` / `OP._scene` (main.ts:51–53) — the audit's box projection depends on
  this interception.
- Any boot exception sets `OP.bootError = <stack string>` (main.ts:146); success sets
  `OP.ready = true` **last**, after all functions below exist (main.ts:135).

The OP surface the host calls (all via `Runtime.evaluate` string templates):

| Host call | Page definition | Returns |
|---|---|---|
| `OP.duration()` | main.ts:57 | film duration seconds (`T.dur`) |
| `OP.draw(t)` | main.ts:58 (draw at main.ts:28–38) | renders film time *t* onto `#cv`, returns nothing |
| `OP.frame(t, q)` | main.ts:59 | `draw(t)` then `cv.toDataURL('image/jpeg', q)` — a base64 data URL |
| `OP.shots()` | main.ts:60 | `[{id, at, end, note}]`, times rounded to 3 dp; `end` = next shot's `at` or duration |
| `OP.captions()` | main.ts:61 | `[{s, e, text, line, auto, rawW, fit, renderedW}]`; `text` joins words, highlighted words prefixed `*`; the width metrics are measured **in the page** by the caption engine (measureText-based) |
| `OP.cues()` | main.ts:62 | sound cues sorted by `t` |
| `OP.cutouts()` | main.ts:73–130 | per-character audit telemetry (§3.2) |
| `OP.cast()` | main.ts:131 | named cast ids (optional — host probes `OP.cast ? … : null`, capture.ts:378) |
| `CAST.items()/draw(i)/label(i)` | castsheet page only | library-preview staging (§3.5); readiness global is `CAST.ready`/`CAST.bootError`, passed as `readyExpr` (capture.ts:359) |

`draw(t)` itself (main.ts:28–38): clamps `t` to `[0, dur − 1e-4]`, picks the active shot,
calls the shot's draw with a `blit` callback that copies `#gl` onto `#cv`
(`ctx.drawImage(GL, 0, 0, W, H)`), then draws 2D overlays and the caption for `t`, and —
critically — **bakes the film's fade-in (0.25 s) and fade-out (0.4 s) black fades into
draw** (main.ts:36–37), so they are part of every captured frame, not an encode step.

Also page-side: `window.__registerOverflowCanvas` (main.ts:134) lets project code register
extra canvas-drawn characters into the overflow audit. If `?capture` is absent the page
instead runs an interactive preview (PLAY button + `Audio('data/mix.wav')` +
requestAnimationFrame loop, main.ts:136–145); capture never triggers that branch.

### 1.3 Frame request/render/readback — the default (JPEG) path

The frame expression is built by `FRAME(t, q, label?)` (capture.ts:303):

```
(() => { const d = OP.frame(t, q); return d; })()
```

i.e. **one synchronous evaluate does draw + JPEG encode + data-URL return**. Frame-exactness
comes from this synchrony: there is no requestAnimationFrame, no vsync, no compositor in
the loop; `draw(t)` is a pure function of `t`. The host strips the data-URL prefix at the
first comma and base64-decodes (`jpg()`, capture.ts:302).

With `label` set (sheet mode), the expression additionally burns a timestamp into the
returned pixels **in the page, after OP.frame has already encoded once**: it re-acquires
the 2D context, resets the transform, fills a 250×86 black rect at top-left, draws
`t.toFixed(2)` in `700 60px sans-serif` white, and re-encodes `#cv` via
`cv.toDataURL('image/jpeg', q)` (capture.ts:303). The label is therefore part of the
sheet JPEG bytes, not an ffmpeg overlay.

### 1.4 Frame readback — the experimental raw path (`FRAME_TRANSPORT=raw`, video only)

`RAW_FRAME(t)` (capture.ts:139): `OP.draw(t)`, then `getImageData(0,0,1080,1920)` on `#cv`,
then `fetch('/__frame', {method:'POST', body: <RGBA bytes>})` to capture's own server.
The server holds the POST response until the full body has been handed to the waiting
consumer (capture.ts:74–80) — **the awaited fetch is the backpressure handshake**. The
host registers its waiter *before* evaluating (capture.ts:463), enforces a **30 s timeout**
(capture.ts:62), and verifies the byte count on both sides: page-reported length and
received length must both equal `RAW_BYTES = 1080*1920*4 = 8,294,400` (capture.ts:57,
466–467). Header comments mark this path EXPERIMENTAL, pending validation on the user's
GPU PC (capture.ts:134–138); the JPEG path is the default and is unchanged.

Raw exists because `#cv` is fixed at 1080×1920 while `QS`/`RES` only shrinks the hidden
GL canvas (which draw upscales onto `#cv`), so raw frames are always full-res RGBA
(capture.ts:55–57).

### 1.5 Renderer selection flags (env contract)

- `CHROME` — explicit browser path, wins over discovery (capture.ts:32).
- `CHROME_FLAGS` — split on whitespace; an explicit value **always wins** (capture.ts:52).
- **GPU-by-default (v5):** only when `CHROME_FLAGS` is unset AND SOFTWARE_GL is off AND
  platform is Linux AND `/dev/dri` exists → effective flags become
  `--use-gl=angle --use-angle=gl` (capture.ts:51–54). On GPU-less machines nothing is
  applied and behaviour is exactly pre-v5 (SwiftShader).
- `SOFTWARE_GL` — parsed strictly: only `'1'` or `'true'` (trimmed, lower-cased) enable
  it (capture.ts:46); adds `--use-angle=swiftshader` at spawn (capture.ts:104). Setting
  both SOFTWARE_GL and CHROME_FLAGS prints a conflict warning (capture.ts:47).
- `FPS` — default 30 (capture.ts:31); non-finite or ≤ 0 → error + `process.exit(1)`
  **before any browser starts** (capture.ts:43).
- `CAP_CHECK=strict` — folds caption-audit violations into the check verdict (§3.3,
  capture.ts:279).
- `QS` — raw query-string suffix appended to the page URL (capture.ts:108).
- `FRAME_TRANSPORT=raw` — §1.4 (capture.ts:439–440).

After every page open in check/sheet/frames/video, `probeRenderer()` (capture.ts:310–325)
evaluates a WebGL probe on `#gl` (`getContext('webgl2') || getContext('webgl')`,
`WEBGL_debug_renderer_info` → `UNMASKED_RENDERER_WEBGL`, fallback `RENDERER`) and prints
`renderer: <string>`. If it matches `/swiftshader|llvmpipe|software/i` it prints a loud
`*** WARNING: SOFTWARE RENDERER ***` variant depending on whether GPU-defaulting or
CHROME_FLAGS was in play. **Advisory only — it never fails a run** (capture.ts:306–309).

---

## 2. Browser / CDP / DOM API inventory

| API | Where | Why | Native equivalent |
|---|---|---|---|
| `child_process.spawn` of Chrome `--headless=new` | capture.ts:103 | the renderer process itself | **Not needed** — the Rust executable *is* the renderer |
| `DevToolsActivePort` file polling | capture.ts:106 | discover CDP port | Not needed — in-process startup |
| `PUT /json/new` (CDP HTTP endpoint) | capture.ts:108 | create target/page | Not needed — direct scene construction |
| `WebSocket` (global) | capture.ts:110 | CDP transport | Not needed — function calls across the runtime boundary |
| CDP `Runtime.enable` | capture.ts:122 | subscribe to exceptions/console | Structured diagnostics channel from the scene runtime (error enum + log sink) |
| CDP `Runtime.evaluate` (`returnByValue`, `awaitPromise`) | capture.ts:121 | **every** host→page operation | Direct API: `evaluate_scene(t)`, metadata queries (shots/captions/cues/cutouts) as typed calls |
| CDP `Runtime.exceptionThrown` event | capture.ts:117 | collect page exceptions into `logs[]` | Runtime error propagation (`Result` + captured error list); must preserve "any page error fails check" |
| CDP `Runtime.consoleAPICalled` (error) | capture.ts:118 | collect `console.error` into `logs[]` | Same diagnostics sink |
| `document.getElementById('cv'/'gl')` | capture.ts:139, 303, 310; main.ts:23; sheet.ts:11 | address the two canvases | Named framebuffers: offscreen 3D target + 2D composite surface |
| `HTMLCanvasElement.toDataURL('image/jpeg', q)` | main.ts:59; capture.ts:303; castsheet capture.ts:337–339 | frame readback as JPEG bytes | In-memory JPEG encode of the composited framebuffer at the **same quality numbers** (0.93/0.92/0.8, §4) |
| `CanvasRenderingContext2D.getImageData` | capture.ts:139 | raw RGBA readback | wgpu framebuffer readback (buffer copy + map) — the raw path is the architectural model |
| `fetch('/__frame', POST)` (page→host) | capture.ts:139 | raw frame upload with backpressure | In-process buffer handoff / bounded channel |
| `fetch('script.json')` + static HTTP server | main.ts:45; capture.ts:70–88 | load project assets | Plain filesystem reads from the project dir |
| `document.fonts.load(...)` / `@font-face` | main.ts:44; index.html:4–6 | guarantee caption/face fonts before first frame | Font loading (fontdb + shaping/rasterisation) with an explicit "fonts ready" gate before rendering |
| `location.search` / `URLSearchParams` | index.html:14; main.ts:136 | `?capture`, `?t`, QS passthrough | CLI flags / env |
| `Audio`, `requestAnimationFrame`, PLAY `<button>` | main.ts:140–145 | interactive preview only | Native preview window with its own clock — or explicitly out of scope; never in the capture path |
| `WEBGL_debug_renderer_info` / `UNMASKED_RENDERER_WEBGL` | capture.ts:310 | identify the actual GL renderer | wgpu adapter info (name + backend + device type) printed in the same `renderer: …` line |
| `performance.now()` | main.ts:142 | preview playback clock only | Not used by capture; deterministic `t` is an input, never a clock reading |
| `ffmpeg` subprocess (spawn/spawnSync) | capture.ts:343, 412, 450 | tiling + video encode | **Stays** — the brief keeps FFmpeg finishing; the CLI shells out identically (§4) |

APIs notably **absent** (a rewrite must not invent dependencies on them):
`Page.captureScreenshot`, `Page.navigate`, `Input.*`, `Emulation.*`, DOM scraping,
cookies/storage, network interception. The page is driven exclusively through `OP`.

**sheet.ts note:** `src/sheet.ts` is *not* the contact-sheet generator (that is capture's
`sheet` mode, §3.4). It is the page script of a separate static style-sheet showcase: it
builds two more hidden WebGL canvases (`#gl` 648×1152, `#gl2` 1280×860, both `samples: 0`,
sheet.ts:11–13), blits them onto a board canvas via the same drawImage compositing pattern,
and sets `window.SHEET_READY = true` when done (sheet.ts:100). Capture never loads it
(capture loads only `index.html` and `castsheet.html`, capture.ts:359). It matters to the
audit only as a second consumer of the same two-canvas compositing model.

---

## 3. Capture modes

CLI shape: `capture <mode> [args] [--flags]`; mode = first non-flag arg (capture.ts:67–68).
Unknown mode prints the mode list and exits 0 (capture.ts:478). All page modes share:
serve → open page → `OP.duration()` → mode body → finally close.

### 3.1 `check` (capture.ts:365–390) — the QA gate

Output/behaviour, in order:

1. `duration <d> s`.
2. Renderer probe line (+ possible software warning), §1.5.
3. Shot table from `OP.shots()`: `<id> <at> → <end> (<len> s) <note>` per shot.
4. Caption list from `OP.captions()`: `  <s>–<e> <text>` per chunk.
5. **The audit loop, merged into one pass at 0.25 s sampling** (capture.ts:377): for each
   `t` from 0 while `t < dur`, step 0.25 — a *single* evaluate
   `(async () => { OP.draw(t); return OP.cutouts(); })()` so the telemetry describes
   exactly the frame just drawn. A throwing sample increments `bad` and prints
   `ERROR at <t>: <first line of message>`; the loop continues.
6. `CAST: <names>` if `OP.cast()` returns an array (capture.ts:378).
7. `printAudit` (overflow + edge touch) → `printContinuity` → `printComposition` →
   `printCaptionAudit` (§3.2–3.3), then any collected page logs.
8. **Verdict** (capture.ts:387–390): `failed = bad>0 || logs.length>0 || overflow.size>0
   || continuity violations>0 || strict-caption fails>0`. Prints `CHECK FAILED` /
   `CHECK OK`; on failure sets `process.exitCode = 1` (deliberately `exitCode`, not
   `exit()`, so stdout flushes and cleanup runs).

### 3.2 What the audit measures (capture.ts:142–277; telemetry from main.ts:73–130)

Page side, per `OP.cutouts()` call, per registered character ("cutout"):

- `overflow` — pixel counts of painted content in the character canvas's border strips,
  per side (`{top, right, bottom, left}` counts), computed inside the painter
  (`t.overflow()`), plus `scanCanvasOverflow` for project-registered extra canvases
  (main.ts:124–128). Scene-independent: reported for every toon whether in the current
  shot's scene or not.
- `box` — the toon **plane's** screen-space box: the plane geometry's bounding-box
  corners projected through the **last-rendered camera** (`OP._cam`, captured by the
  E.render wrapper), NDC→px against 1080×1920, clamped only to ±10000 px (deliberately
  *not* clamped to the frame, or edge detection could never fire), with a `behind` flag
  set when any corner has view-space z > 0, NDC z > 1, or a non-finite projection
  (main.ts:94–113). Only toons inside the last-rendered scene and visible get a box;
  registered extra canvases get `box: null`.
- `sig` — the character's look signature string (`lookSignature(t.look)`), the identity
  fingerprint for continuity.

Host side, per sample (`auditSample`, capture.ts:163–229):

- **CANVAS OVERFLOW** — any side count > 0 records an entry, **deduped by
  `id + pose + sides`**, keeping first-seen counts (capture.ts:170–177). Printed as
  `CANVAS OVERFLOW: <id> (<desc>) <side>=<px> … — pose <pose>`, or `CANVAS OVERFLOW:
  none`. **Gate:** any entry fails check.
- **CONTINUITY** — first-seen `sig` per character id is stored; a later different sig is
  a violation, deduped by `id + shot + changed-fields`, where changed fields are computed
  by JSON-parsing both signatures and diffing keys (capture.ts:181–189, 195–202).
  Printed `CONTINUITY VIOLATION: <id> in <shot> — changed: <fields>` or
  `CONTINUITY: clean (<n> named characters tracked)`. **Gate:** any violation fails check.
- **EDGE TOUCH** (warning only) — the plane box is converted to the **drawn figure's**
  box by fixed stage fractions (`figBox`, capture.ts:248–250: x from 384/1280 to
  896/1280, y from 256/1408 to 1280/1408 of the plane box — the toon stage occupies that
  sub-rect of the 1280×1408 character canvas). The figure box is tested against the
  frame: `y0<0` top, `x1>1080` right, `y1>1920` bottom, `x0<0` left; entries keyed
  `(shotId, id, side)` keep first-sampled `t` and a sample count (capture.ts:203–214).
  (v5 change: pre-v5 this tested the plane itself and produced dozens of spurious
  warnings on clean films — capture.ts:198–202.)
- **Composition / framing** (advisory only, skipped for `behind` boxes; capture.ts:215–
  228, printed 258–277). A face box is derived from the figure box by more fixed
  fractions (`faceBox`, capture.ts:251–253: x 118/512→394/512, y 40/1024→316/1024 —
  the standing-adult head disc within the stage):
  - `zone` (CAPTION ZONE warning): face box intersects the caption zone
    `x 0..1080, y 1340..1550` with intersection area **> 5% of face area**.
  - `rail` (SAFE AREA warning): face centre x **> 920** (Shorts right rail).
  - `bottom` (SAFE AREA warning): face centre y **> 1620** (bottom UI band).
  - `exceeds` (FRAMING warning): figure wider than 1080 or taller than 1920.
  - `small` (FRAMING warning): figure height **< 288 px** (= 15% of 1920).
  - **Dead-centre aggregate** (COMPOSITION advisory, computed at print time,
    capture.ts:270–273): a sample's focal subject is its tallest figure; a shot is
    flagged when focal samples ≥ 4, focal present in ≥ 80% of the shot's samples, and
    the focal centre lies in the centre ninth (cx 360..720, cy 640..1280) in ≥ 90% of
    focal samples.
  - Summary line: `COMPOSITION: <n> findings (caption-zone …, safe-area …, thirds …,
    framing …, edge-touch …)`.
  - The fractions are stated approximations (kid heads sit lower; sitting/jumping move
    the disc; lean rotates the figure) — acceptable because every consumer is advisory
    (capture.ts:241–247). The CHECK verdict never sees composition findings.

Shot attribution: `shotIdAt` assigns each sample to the last shot with `at <= t + 1e-9`
(capture.ts:203).

### 3.3 Caption audit (capture.ts:279–301)

Runs on `OP.captions()`' page-measured metrics — deliberately **the renderer's own
measurements, never a second measurer**:

- Headline: `CAPTION AUDIT: <n> chunks, <authored>/<lines> lines authored, min fit
  <min>, max rendered width <max> px`.
- Lines whose captions were auto-chunked (`auto: true`) are listed:
  `CAPTIONS: estimate-grade (auto-chunked) on lines … — author caps in script.json for a master`.
- **Width floor:** a chunk with `renderedW > 850 px` → `CAPTION WIDTH WARN/FAIL: line
  <n> chunk "<text>" renders <w> px (> 850) at t=<s>s …`.
- **Fit floor:** a chunk with `fit < 0.68` → `CAPTION FIT WARN/FAIL: … fit <f> (< 0.68;
  raw width <rawW> px) … — split the chunk, never shrink it`.
- Violations deduped by `line + text`. Default severity is **WARN** (a v4 project that
  passes today still passes); with `CAP_CHECK=strict` they print as FAIL, are counted,
  and the count is folded into the check verdict (capture.ts:279, 298–300).
- **Explicitly not enforced:** the ≥ 0.33 s/word time floor. Ruling R5 recorded in the
  source (capture.ts:274–278): at the skill's target length speech runs ~0.31 s/word, so
  the time floor cannot hold simultaneously; the width floor wins and the time floor is
  advisory only. The renderer keeps `CAP.maxW = 960`; the 850 px floor lives here, in
  the audit (capture.ts:267–273).

### 3.4 `sheet [out=review/all.jpg] [step=0.5] [cols=9] [--guides]` (capture.ts:401–426)

- Renderer probe, then frames at `t = step/2 + k·step` while `t < dur`
  (capture.ts:404) — i.e. mid-step sampling — each via `FRAME(t, 0.8, label=true)`
  written as `f%04d.jpg` into a temp dir.
- Tiling is an ffmpeg subprocess (`spawnSync`): `-framerate 1 -i f%04d.jpg -vf
  "scale=216:384,tile=<cols>x<rows>:padding=4:color=black" -frames:v 1 -q:v 3`
  (capture.ts:407–408). With `--guides`, the filter gains
  `drawgrid=w=72:h=128:t=1:c=white@0.35` and
  `drawbox=x=0:y=268:w=216:h=42:color=red@0.55:t=1` (grid thirds + caption zone, in
  tile-scaled coordinates) inserted before the tile filter.
- ffmpeg failure honesty: `spawnSync`'s `r.error` (e.g. ENOENT, status null) is checked
  **separately** from `r.status !== 0`; either throws (capture.ts:413–415).
- Success prints `<out> (<n> frames, every <step> s)`.
- **Then the audit runs again**, at the sheet's own sample times (capture.ts:421–425):
  printAudit + printComposition. Overflow here is report-only — **except** the
  completeness rule: samples that throw are counted, and if any were lost the mode
  prints the audit as `unknown (audit incomplete)`, reports `sheet audit INCOMPLETE`,
  and sets `process.exitCode = 1` (capture.ts:423). An audit that lost samples must
  never certify `none` (capture.ts:233–235). Note sheet mode collects continuity sigs
  via auditSample but never prints or gates on them (no printContinuity call).

### 3.5 `castsheet <outdir> [kind]` (capture.ts:328–351, 356–359)

- Preflight: if `castsheet.html` or `build/castsheet.js` is missing in the project,
  prints remediation instructions and `process.exit(1)` (capture.ts:356–359).
- Opens **castsheet.html** with the CAST readiness expression (capture.ts:359).
  `CAST.items()` → `[{kind, id, name}]`; optional kind filter; empty selection →
  error listing available kinds + `process.exitCode = 1` (capture.ts:334–335).
- Per item: `CAST.draw(i)` → save `document.getElementById('cv').toDataURL('image/jpeg',
  0.92)` as `<outdir>/<kind>-<id>.jpg` (the library/index.json preview naming); then
  `CAST.label(i)` → save a 0.8-quality frame to the temp sequence (capture.ts:336–340).
- Contact sheet: ffmpeg tile, `scale=270:480,tile=4x<rows>:padding=4:color=black`,
  `-q:v 3` → `<outdir>/castsheet.jpg` (capture.ts:342–346), same r.error/r.status
  honesty as sheet.

### 3.6 `frames <dir> "<t,t,…>"` (capture.ts:396–400)

Renderer probe; for each listed time, `FRAME(t, 0.92)` written to
`<dir>/t<t.toFixed(2)>.jpg`. Prints the directory path. No audit.

### 3.7 `cues` (capture.ts:391–395)

`OP.cues()` → writes `data/cues.json` as `JSON.stringify({duration, cues}, null, 1)`
(1-space indent), prints `data/cues.json: <n> cues, <dur> s`. This file is the contract
with `tools/mix.py` downstream.

### 3.8 `video [out=renders/video.mp4] [start end] [--draft]` (capture.ts:427–479)

See §4 for transport/encode detail. Mode-specific facts: frame range is
`a0 = start||0`, `a1 = min(end||dur, dur)`, `n = round((a1−a0)*FPS)` frames at times
`a0 + i/FPS` (capture.ts:435); progress `\r<i>/<n> frames` every 150 frames
(capture.ts:470); audio is `data/mix.wav` sliced `-ss a0 -t (a1−a0)` if present, else a
generated silent track (`anullsrc=r=48000:cl=stereo`) with the final line reporting
`SILENT: no data/mix.wav` (capture.ts:450–451, 476).

### 3.9 `preview` (capture.ts:353–354)

Serves the project on fixed port **8765** and prints the URL. **No Chrome is spawned** —
the human opens their own browser and uses the page's interactive branch (§1.2).

---

## 4. Frame transport, encoding, and failure discipline

### 4.1 JPEG path (default)

| Consumer | Source | Quality |
|---|---|---|
| video master | `OP.frame(t, 0.93)` | 0.93 (capture.ts:467) |
| video --draft | `OP.frame(t, 0.8)` | 0.8 (capture.ts:467) |
| frames mode | `OP.frame(t, 0.92)` | 0.92 (capture.ts:398) |
| sheet tiles | `FRAME(t, 0.8, label)` | 0.8 (capture.ts:404) |
| castsheet previews | `#cv.toDataURL(…, 0.92)` | 0.92 (capture.ts:337) |
| castsheet tiles | `#cv.toDataURL(…, 0.8)` | 0.8 (capture.ts:339) |

Transport is base64-in-JSON over the CDP WebSocket (one evaluate per frame), decoded by
`jpg()` (capture.ts:302). Frames are written straight into ffmpeg's stdin
(`image2pipe`, capture.ts:442) with stdin backpressure honoured (`write()` →
`once('drain')`, capture.ts:468). **Consequence that must be understood:** master frames
reach x264 *after* a q0.93 JPEG generation — the pipeline's masters carry that
double-compression as part of their current noise profile.

### 4.2 Raw path

§1.4: RGBA 1080×1920, 8,294,400 B/frame, POST + held response as the handshake,
30 s timeout, dual byte-count verification, ffmpeg input declared
`-f rawvideo -pixel_format rgba -video_size 1080x1920` (capture.ts:441). Only the input
declaration and frame source differ; encoder args are identical (capture.ts:438–440).

### 4.3 Video encode argument sets (capture.ts:448–451)

- **Master:** `libx264 -preset slow -crf 18 -threads 4 -pix_fmt yuv420p`, BT.709 tags via
  `-color_primaries bt709 -color_trc bt709 -colorspace bt709 -color_range tv` **plus**
  `-x264-params colorprim=bt709:transfer=bt709:colormatrix=bt709` (the generic options
  alone leave primaries/transfer "unknown" with libx264; the params triple lands all
  three in the VUI — tags only, no pixel change), AAC **256k**, `-ar 48000`.
- **Draft (--draft):** `libx264 -preset veryfast -crf 23`, yuv420p, AAC **192k**, 48 kHz,
  no colour-tag args.
- Both: `-map 0:v -map 1:a -shortest -movflags +faststart`, ffmpeg stderr inherited,
  `-v error`.

### 4.4 .part / rename discipline (video; capture.ts:434, 456–477)

Encode target is **`<out>.part.mp4`** in the same directory. Any failure inside the
frame loop or encode — page throw, raw byte mismatch, ffmpeg spawn/stdin error, non-zero
ffmpeg exit — deletes the `.part` file and rethrows. Only after `ffCode === 0` does
`fs.renameSync(part, out)` publish the file (atomic, same filesystem). A killed or
failed render therefore never leaves a truncated MP4 at the real path. Sheet/castsheet
tiling writes through a temp frame dir that is always removed (capture.ts:410, 344).

### 4.5 Exit-code honesty lattice

- Invalid `FPS` → exit 1 before any work (capture.ts:43).
- Chrome missing / didn't start / boot failed / boot timeout → thrown error, non-zero
  exit, collected page logs attached (capture.ts:100, 107, 126–127).
- check: `CHECK FAILED` ⇒ `process.exitCode = 1`; `CHECK OK` ⇒ 0 (capture.ts:389–390).
- sheet: incomplete audit ⇒ exit 1 even though the sheet image itself was produced
  (capture.ts:423).
- castsheet: missing page/build ⇒ exit 1 preflight (capture.ts:358); empty kind
  selection ⇒ exitCode 1 (capture.ts:335).
- video: ffmpeg spawn error and stdin error are captured into `ffErr` and checked
  per-frame and at close (capture.ts:453, 469, 473); `ffCode !== 0` throws with
  "…do NOT use it" (capture.ts:474). ffmpeg *tiling* distinguishes `r.error` (couldn't
  start, status null) from `r.status !== 0` (capture.ts:414–415) — the historical bug
  was treating a null status as success and printing a path for a file never written.
- Raw path: page death mid-frame ⇒ 30 s timeout error instead of an infinite hang
  (capture.ts:62).

---

## 5. What a native `talerender` CLI must reproduce — checklist

CLI surface & modes

- [ ] Modes `check`, `cues`, `sheet`, `frames`, `video`, `preview`, `castsheet` with the
      same argument shapes, defaults (`review/all.jpg`, step 0.5, cols 9,
      `renders/video.mp4`, castsheet outdir `review/cast`), and `--draft` / `--guides`
      flags (capture.ts:1–12).
- [ ] Env contract: `FPS` (validated > 0, fail before work), `CAP_CHECK=strict`,
      a `QS`-equivalent for render-resolution scaling, a `FRAME_TRANSPORT`-equivalent
      switch, and renderer-selection equivalents of `CHROME_FLAGS` / `SOFTWARE_GL`
      semantics (explicit choice always wins; GPU-by-default when a GPU exists).
- [ ] A `renderer: <identity>` line in check/sheet/frames/video, with a loud
      software-renderer warning when a fallback/software adapter is in use (§1.5).
- [ ] Deterministic evaluation: frame at time *t* is a pure function of *t*; no
      wall-clock, vsync, or rAF anywhere in the capture path; boot completes
      (fonts loaded, scene built) before the first frame is produced.

Page-equivalent scene API (the OP contract, §1.2)

- [ ] `duration()`, `draw(t)` (including the baked 0.25 s fade-in / 0.4 s fade-out),
      `frame(t, q)`, `shots()`, `captions()` **with page-equivalent width metrics**
      (rawW / fit / renderedW from the real text measurer), `cues()`, `cast()`.
- [ ] `cutouts()` telemetry per character: border-strip overflow pixel counts,
      plane screen box with behind-camera semantics, look signature string — plus
      registration of project-defined canvas characters (overflow-only, box null).
- [ ] `CAST.items()/draw(i)/label(i)` equivalent for castsheet.

Audit fidelity (check + sheet, §3.2–3.4)

- [ ] Sampling: check at 0.25 s steps with draw+telemetry atomic per sample; sheet at
      its own step times, mid-step offset.
- [ ] CANVAS OVERFLOW gate (dedupe id+pose+sides), CONTINUITY gate (signature diff with
      named changed fields), verdict expression exactly: bad samples OR page errors OR
      overflow OR violations OR strict-caption fails.
- [ ] EDGE TOUCH + composition/framing advisories with the figBox/faceBox stage
      fractions (384/896 of 1280, 256/1280 of 1408; head disc 118–394 of 512,
      40–316 of 1024) and thresholds: caption zone y 1340–1550 at >5% face overlap,
      rail x > 920, bottom y > 1620, exceeds-frame, small < 288 px, dead-centre
      aggregate (≥4 focal samples, ≥80% presence, ≥90% centred).
- [ ] Caption audit: rendered width ≤ 850 px, fit ≥ 0.68, WARN by default, FAIL under
      strict; auto-chunked lines reported as estimate-grade; time floor NOT enforced.
- [ ] Output line formats preserved (`CANVAS OVERFLOW: …`, `CONTINUITY: clean (… )`,
      `CAPTION AUDIT: …`, `CHECK OK` / `CHECK FAILED`, …) — downstream tooling and
      human QA read these.
- [ ] Incomplete-audit honesty: lost samples ⇒ `unknown (audit incomplete)`, never
      `none`, and a non-zero exit in sheet mode.

Transport & finishing (§4)

- [ ] JPEG frame qualities: video master 0.93, draft 0.8, frames 0.92, sheet 0.8,
      castsheet 0.92/0.8 — or a documented, deliberate decision to change the master's
      q0.93-JPEG generation (it is part of the current masters' noise profile).
- [ ] Raw RGBA readback path with byte-count verification and a frame timeout.
- [ ] Sheet tiling geometry: tiles scaled 216×384, padding 4, black background,
      ffmpeg `-q:v 3`; `--guides` grid (72×128) + caption-zone box (0, 268, 216×42)
      overlays; sheet timestamps burned into the frame at capture time, top-left
      250×86 black box, 60 px bold.
- [ ] Castsheet tiling: 270×480 tiles, 4 columns, padding 4, `-q:v 3`; preview files
      named `<kind>-<id>.jpg`.
- [ ] `frames` file naming `t<seconds, 2dp>.jpg`; `cues` output
      `data/cues.json` = `{duration, cues}` with 1-space JSON indent.
- [ ] Video encode: master x264 slow/CRF 18/threads 4/yuv420p + full BT.709 tag set
      (incl. the x264-params triple), AAC 256k/48 kHz; draft veryfast/CRF 23, AAC
      192k; mix.wav sliced to [start, end]; explicit silence track + `SILENT:` report
      when mix.wav is absent; `-shortest`, `+faststart`.
- [ ] `.part.mp4` discipline: encode to sibling temp, delete on any failure, rename
      only after encoder exit 0; temp frame dirs always cleaned.
- [ ] Exit-code lattice of §4.5, including distinguishing "encoder failed to start"
      from "encoder exited non-zero", and per-frame progress reporting
      (`\r<i>/<n> frames` every 150).
