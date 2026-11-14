// Frame-exact capture through headless Chrome. No npm packages (Node 22+). Run from the project folder.
//   node tools/capture.mjs check                         boot, list shots + captions, report errors, cast + continuity report
//   node tools/capture.mjs cues                          write data/cues.json (the sound cues, for tools/mix.py)
//   node tools/capture.mjs sheet review/all.jpg [step=0.5] [cols=9] [--guides]   contact sheet, a frame every `step` s, time-stamped; --guides overlays a grid + caption zone
//   node tools/capture.mjs frames review/full "1.2,8.4"  full-size JPEGs of those times
//   node tools/capture.mjs castsheet <outdir> [kind]     library previews: one staged frame per pose/mood/look/held prop as <kind>-<id>.jpg, plus a labelled castsheet.jpg contact sheet ([kind] renders one kind only)
//   node tools/capture.mjs video renders/out.mp4 [start end] [--draft]   the film (30 fps) with data/mix.wav if it exists
//   node tools/capture.mjs preview                       serve the project and print a URL to watch it in a browser
// CHROME=/path/to/chrome overrides the browser. Without it, discovery tries (in order) the
// system Chrome/Chromium/Edge installs, then the faceless-shorts bundled Chrome
// ($FACELESS_HOME/chrome/chrome, default ~/.faceless-shorts/chrome/chrome), then Brave.
// CHROME_FLAGS="--no-sandbox --use-gl=angle --use-angle=gl"  extra Chrome flags, split on
//   whitespace; an explicit value always wins. When it is UNSET on Linux with a GPU
//   present (/dev/dri), capture applies the GPU default "--use-gl=angle --use-angle=gl"
//   itself (v5 Phase 3, GPU_DEFAULT_FLAGS below): on a real GPU (e.g. AMD iGPU over
//   ANGLE/GL) frames render at ~0.15 s/frame instead of silently falling back to
//   SwiftShader (~3 s/frame on CPU), at identical quality. SOFTWARE_GL=1 opts out of
//   the default and forces the software path; never combine it with CHROME_FLAGS.
import { spawn, spawnSync } from 'node:child_process';
import http from 'node:http';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const FPS = +(process.env.FPS || 30);
const CHROME = [process.env.CHROME, '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome', '/Applications/Chromium.app/Contents/MacOS/Chromium',
  '/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge', '/usr/bin/google-chrome', '/usr/bin/google-chrome-stable', '/usr/bin/chromium',
  '/usr/bin/chromium-browser', '/snap/bin/chromium', 'C:/Program Files/Google/Chrome/Application/chrome.exe',
  'C:/Program Files (x86)/Google/Chrome/Application/chrome.exe',
  // v5 (P-09): appended LAST so any browser a machine already discovered keeps winning — these
  // only rescue machines that found none: the ecosystem's own bundled Chrome (setup.sh's install
  // spot, $FACELESS_HOME/chrome/chrome) and Brave (a Brave-only machine previously got
  // 'Chrome not found' even though setup.sh already knew these paths).
  path.join(process.env.FACELESS_HOME || path.join(os.homedir(), '.faceless-shorts'), 'chrome', 'chrome'),
  '/usr/bin/brave-browser', '/usr/bin/brave', '/usr/bin/brave-browser-stable', '/Applications/Brave Browser.app/Contents/MacOS/Brave Browser'].find((p) => p && fs.existsSync(p));
// I4: FPS must be a positive finite number — FPS=abc/0/negative previously produced a 0-frame "success".
if (!Number.isFinite(FPS) || FPS <= 0) { console.error(`capture: invalid FPS "${process.env.FPS}" — FPS must be a positive finite number (e.g. FPS=30)`); process.exit(1); }
// I4: SOFTWARE_GL is parsed explicitly — only '1' or 'true' enable the SwiftShader path
// (previously any non-empty value, including SOFTWARE_GL=0 / =false, forced software rendering).
const SOFTWARE_GL = ['1', 'true'].includes(String(process.env.SOFTWARE_GL ?? '').trim().toLowerCase());
if (SOFTWARE_GL && process.env.CHROME_FLAGS) console.warn('capture: WARNING — SOFTWARE_GL and CHROME_FLAGS are both set. SOFTWARE_GL forces SwiftShader (software/CPU rendering) while CHROME_FLAGS is meant to select the GPU; the two conflict — unset one of them.');
// v5 (Phase 3): GPU-by-default — see the header. /dev/dri is the Linux DRM device dir:
// present on the user's GPU machine, absent on GPU-less QA VMs (no default there, so
// behavior on such machines is exactly the pre-v5 behavior).
const GPU_DEFAULT_FLAGS = '--use-gl=angle --use-angle=gl';
const ENV_FLAGS = String(process.env.CHROME_FLAGS ?? '').split(/\s+/).filter(Boolean);
const GPU_DEFAULTED = !ENV_FLAGS.length && !SOFTWARE_GL && process.platform === 'linux' && fs.existsSync('/dev/dri');
const CHROME_FLAGS_EFF = GPU_DEFAULTED ? GPU_DEFAULT_FLAGS.split(/\s+/) : ENV_FLAGS;
// I4 (experimental, r07): raw frame transport state for FRAME_TRANSPORT=raw in video mode.
// #cv is fixed at 1080x1920 (index.html); QS res only shrinks the hidden WebGL canvas, which draw()
// upscales onto #cv — exactly as on the JPEG path — so raw frames are always 1080x1920 RGBA.
const RAW_W = 1080, RAW_H = 1920, RAW_BYTES = RAW_W * RAW_H * 4;
let rawBuf = null, rawWaiter = null;
// v5 (P-18): the raw waiter gains a 30 s timeout — if the page dies mid-frame, the render
// fails with an error instead of hanging the render queue forever on a frame that never comes.
function takeRawFrame() { if (rawBuf) { const b = rawBuf; rawBuf = null; return Promise.resolve(b); } return new Promise((res, rej) => { const to = setTimeout(() => { rawWaiter = null; rej(new Error('raw frame timeout — page did not deliver a frame')); }, 30000); rawWaiter = (b) => { clearTimeout(to); res(b); }; }); }
function pushRawFrame(b) { if (rawWaiter) { const w = rawWaiter; rawWaiter = null; w(b); } else rawBuf = b; }
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const args = process.argv.slice(2), flags = new Set(args.filter((a) => a.startsWith('--')));
const [mode, ...rest] = args.filter((a) => !a.startsWith('--'));

const MIME = { '.html': 'text/html', '.js': 'text/javascript', '.mjs': 'text/javascript', '.json': 'application/json', '.otf': 'font/otf',
  '.ttf': 'font/ttf', '.woff2': 'font/woff2', '.wav': 'audio/wav', '.mp3': 'audio/mpeg', '.png': 'image/png', '.jpg': 'image/jpeg' };
function serve(port = 0) {
  return new Promise((res) => {
    const srv = http.createServer((req, r) => {
      // I4 (experimental, r07): raw frame sink for FRAME_TRANSPORT=raw — the page POSTs one frame's
      // RGBA bytes here; the response is sent only once the full body has been handed to the waiter,
      // so the page's awaited fetch is the frame-exact handshake (ordering matches the JPEG path).
      if (req.method === 'POST' && req.url.split('?')[0] === '/__frame') {
        const chunks = [];
        req.on('data', (c) => chunks.push(c));
        req.on('end', () => { pushRawFrame(Buffer.concat(chunks)); r.writeHead(200, { 'Content-Type': 'text/plain' }); r.end('ok'); });
        req.on('error', () => { try { r.writeHead(500); r.end(); } catch {} });
        return;
      }
      const u = decodeURIComponent(req.url.split('?')[0]), f = path.join(ROOT, u === '/' ? 'index.html' : u);
      if (!f.startsWith(ROOT) || !fs.existsSync(f) || fs.statSync(f).isDirectory()) { r.writeHead(404); r.end(); return; }
      r.writeHead(200, { 'Content-Type': MIME[path.extname(f)] || 'application/octet-stream', 'Cache-Control': 'no-store' });
      fs.createReadStream(f).pipe(r);
    });
    srv.listen(port, '127.0.0.1', () => res(srv));
  });
}

async function openPage(port, page = 'index.html', readyExpr = null) {
  if (!CHROME) throw new Error('Chrome not found: install Google Chrome or set CHROME=/path/to/chrome');
  const prof = fs.mkdtempSync(path.join(os.tmpdir(), 'fs-chrome-'));
  const chrome = spawn(CHROME, ['--headless=new', '--remote-debugging-port=0', `--user-data-dir=${prof}`, '--no-first-run', '--no-default-browser-check',
    '--hide-scrollbars', '--window-size=1080,1920', '--ignore-gpu-blocklist', '--enable-unsafe-swiftshader', ...(SOFTWARE_GL ? ['--use-angle=swiftshader'] : []), ...CHROME_FLAGS_EFF, ...(process.getuid?.() === 0 ? ['--no-sandbox'] : []), '--disable-background-timer-throttling', '--disable-renderer-backgrounding', 'about:blank'], { stdio: 'ignore' });
  let dbg;
  for (let i = 0; i < 200 && !dbg; i++) { try { dbg = fs.readFileSync(path.join(prof, 'DevToolsActivePort'), 'utf8').split('\n')[0].trim(); } catch { await sleep(100); } }
  if (!dbg) throw new Error('Chrome did not start');
  const r = await fetch(`http://127.0.0.1:${dbg}/json/new?${encodeURIComponent(`http://127.0.0.1:${port}/${page}?capture=1${process.env.QS || ''}`)}`, { method: 'PUT' });
  const target = await r.json();
  const ws = new WebSocket(target.webSocketDebuggerUrl);
  await new Promise((res, rej) => { ws.onopen = res; ws.onerror = rej; });
  let id = 0; const pend = new Map(), logs = [];
  ws.onmessage = (e) => {
    const m = JSON.parse(e.data);
    if (m.id && pend.has(m.id)) { const p = pend.get(m.id); pend.delete(m.id); m.error ? p.rej(new Error(m.error.message)) : p.res(m.result); }
    else if (m.method === 'Runtime.exceptionThrown') logs.push('EXCEPTION ' + (m.params.exceptionDetails.exception?.description || m.params.exceptionDetails.text));
    else if (m.method === 'Runtime.consoleAPICalled' && m.params.type === 'error') logs.push('ERROR ' + m.params.args.map((a) => a.value ?? a.description).join(' '));
  };
  const send = (method, params = {}) => new Promise((res, rej) => { const i = ++id; pend.set(i, { res, rej }); ws.send(JSON.stringify({ id: i, method, params })); });
  const ev = async (expr) => { const x = await send('Runtime.evaluate', { expression: expr, returnByValue: true, awaitPromise: true }); if (x.exceptionDetails) throw new Error(x.exceptionDetails.exception?.description || x.exceptionDetails.text); return x.result.value; };
  await send('Runtime.enable');
  for (let i = 0; ; i++) {
    const st = await ev(readyExpr || 'window.OP ? (OP.bootError ? "ERR:" + OP.bootError : OP.ready) : false').catch(() => false);
    if (st === true) break;
    if (typeof st === 'string') throw new Error('boot failed: ' + st + '\n' + logs.join('\n'));
    if (i > 1200) throw new Error('boot timeout\n' + logs.join('\n'));
    await sleep(100);
  }
  const close = () => { try { ws.close(); } catch {} chrome.kill('SIGKILL'); setTimeout(() => { try { fs.rmSync(prof, { recursive: true, force: true }); } catch {} }, 300); };
  return { ev, logs, close };
}
// I4 (experimental, r07): FRAME_TRANSPORT=raw frame expression — draws via the same draw path as
// OP.frame (OP.frame = draw(t) + toDataURL), but reads the #cv pixels with getImageData in the same
// evaluate and POSTs the raw RGBA bytes to this process's own HTTP server (see serve()/pushRawFrame),
// instead of a base64 JPEG over CDP. EXPERIMENTAL: pending the r07 V1-V7 validation protocol on the
// user's GPU PC (readback identity, RGB->YUV colour pipeline, benchmarks, master quality delta).
// The default JPEG path below is unchanged.
const RAW_FRAME = (t) => `(async () => { OP.draw(${t}); const cv = document.getElementById('cv'); const d = cv.getContext('2d').getImageData(0, 0, cv.width, cv.height); const r = await fetch('/__frame', { method: 'POST', body: new Uint8Array(d.data.buffer, d.data.byteOffset, d.data.byteLength) }); if (!r.ok) throw new Error('raw frame POST failed: HTTP ' + r.status); return d.data.byteLength; })()`;
// WP2 audit (canvas overflow + edge touch), shared by check and sheet modes.
// An audit collects, over a set of sampled times: overflow events deduped by
// id+pose+sides (first-seen counts kept), and edge-touch entries keyed by
// (shotId, id, side) with the first sampled t and a sampled-frame count.
const OVERFLOW_SIDES = ['top', 'right', 'bottom', 'left'];
function makeAudit(shotList) { return { shots: shotList || [], overflow: new Map(), edge: new Map(), sigs: new Map(), violations: [], violationKeys: new Set(), comp: new Map(), compShots: new Map() }; }
function sigDiffFields(a, b) {
  try {
    const pa = JSON.parse(a), pb = JSON.parse(b);
    const keys = [...Object.keys(pa), ...Object.keys(pb).filter((k) => !(k in pa))];
    return keys.filter((k) => JSON.stringify(pa[k]) !== JSON.stringify(pb[k]));
  } catch { return ['sig']; }
}
function shotIdAt(audit, t) { let id = audit.shots.length ? audit.shots[0].id : '?'; for (const s of audit.shots) { if (s.at <= t + 1e-9) id = s.id; else break; } return id; }
function auditSample(audit, t, cuts) {
  if (!Array.isArray(cuts)) return;
  const shotId = shotIdAt(audit, t);
  let focalFig = null;
  for (const c of cuts) {
    if (!c) continue;
    const ov = c.overflow;
    if (ov) {
      const sides = OVERFLOW_SIDES.filter((s) => (ov[s] || 0) > 0);
      if (sides.length) {
        const key = `${c.id}\0${c.pose || ''}\0${sides.join(',')}`;
        if (!audit.overflow.has(key)) audit.overflow.set(key, { id: c.id, desc: c.desc, pose: c.pose, sides: sides.map((s) => `${s}=${ov[s]}`) });
      }
    }
    if (c.sig) {
      if (!audit.sigs.has(c.id)) audit.sigs.set(c.id, c.sig);
      else if (audit.sigs.get(c.id) !== c.sig) {
        const fields = sigDiffFields(audit.sigs.get(c.id), c.sig);
        const vKey = `${c.id}\0${shotId}\0${fields.join(',')}`;
        if (!audit.violationKeys.has(vKey)) { audit.violationKeys.add(vKey); audit.violations.push({ id: c.id, shotId, fields }); }
      }
    }
    const b = c.box;
    if (b) {
      // v5 (Phase 3): EDGE TOUCH measures the DRAWN FIGURE (figBox), not the toon
      // plane — the plane carries the 384-unit canvas safety margin around the
      // figure on every side, so a centred figure whose plane overhangs the frame
      // warned spuriously (upstream field report: 37 warnings on a clean film;
      // the v4 chicken rebuild printed 99). CANVAS OVERFLOW above is untouched:
      // it keeps the canvas/plane pixel semantics exactly.
      const fb = figBox(b);
      const crossed = [];
      if (fb.y0 < 0) crossed.push('top');
      if (fb.x1 > 1080) crossed.push('right');
      if (fb.y1 > 1920) crossed.push('bottom');
      if (fb.x0 < 0) crossed.push('left');
      for (const side of crossed) {
        const key = `${shotId}\0${c.id}\0${side}`;
        const prev = audit.edge.get(key);
        if (prev) prev.n++;
        else audit.edge.set(key, { shot: shotId, id: c.id, side, first: t, n: 1 });
      }
      // v5 (B7): composition + framing derive from the SAME box in the SAME
      // sample — zero extra draws. Everything here is WARN/ADVISORY only; the
      // CHECK verdict expression never sees these findings.
      if (!b.behind) {
        const fig = figBox(b), face = faceBox(fig);
        const rec = (kind, detail) => { const key = `${shotId}\0${c.id}\0${kind}`; const prev = audit.comp.get(key); if (prev) prev.n++; else audit.comp.set(key, { kind, shot: shotId, id: c.id, first: t, n: 1, detail }); };
        const faceArea = Math.max(0, face.x1 - face.x0) * Math.max(0, face.y1 - face.y0);
        const zoneX = Math.max(0, Math.min(face.x1, 1080) - Math.max(face.x0, 0)), zoneY = Math.max(0, Math.min(face.y1, 1550) - Math.max(face.y0, 1340));
        if (faceArea > 0 && zoneX * zoneY > 0.05 * faceArea) rec('zone');
        const fcx = (face.x0 + face.x1) / 2, fcy = (face.y0 + face.y1) / 2;
        if (fcx > 920) rec('rail');
        if (fcy > 1620) rec('bottom');
        const figW = fig.x1 - fig.x0, figH = fig.y1 - fig.y0;
        if (figW > 1080 || figH > 1920) rec('exceeds', `${Math.round(figW)}×${Math.round(figH)} px vs the 1080×1920 frame`);
        else if (figH > 0 && figH < 288) rec('small', `${Math.round((figH / 1920) * 100)}% of frame height`);
        if (!focalFig || figH > focalFig.h) focalFig = { h: figH, cx: (fig.x0 + fig.x1) / 2, cy: (fig.y0 + fig.y1) / 2 };
      }
    }
  }
  // v5 (B7): rule-of-thirds aggregate — a sample's focal subject is its
  // tallest figure; the per-shot verdict is computed at print time.
  let agg = audit.compShots.get(shotId);
  if (!agg) { agg = { samples: 0, focalSamples: 0, centreSamples: 0 }; audit.compShots.set(shotId, agg); }
  agg.samples++;
  if (focalFig) { agg.focalSamples++; if (focalFig.cx >= 360 && focalFig.cx <= 720 && focalFig.cy >= 640 && focalFig.cy <= 1280) agg.centreSamples++; }
}
function printAudit(audit, incomplete = false) {
  if (audit.overflow.size) for (const e of audit.overflow.values()) console.log(`CANVAS OVERFLOW: ${e.id} (${e.desc}) ${e.sides.join(' ')} — pose ${e.pose}`);
  // v5 (P-03): an audit that lost samples must never certify 'none' — it reports 'unknown'.
  else console.log(incomplete ? 'CANVAS OVERFLOW: unknown (audit incomplete)' : 'CANVAS OVERFLOW: none');
  if (audit.edge.size) for (const e of audit.edge.values()) console.log(`EDGE TOUCH (warning): ${e.shot} ${e.id} crosses ${e.side} (first at t=${e.first.toFixed(2)}s, ${e.n} sampled frames)`);
  else console.log(incomplete ? 'EDGE TOUCH: unknown (audit incomplete)' : 'EDGE TOUCH: none');
}
function printContinuity(audit) {
  if (audit.violations.length) for (const v of audit.violations) console.log(`CONTINUITY VIOLATION: ${v.id} in ${v.shotId} — changed: ${v.fields.join(', ')}`);
  else console.log(`CONTINUITY: clean (${audit.sigs.size} named characters tracked)`);
}
// v5 (B7, D6-3): composition checker + framing linter. OP.cutouts' `box` is the
// toon PLANE (canvas margins included); the figure and face boxes are fixed
// fractions of it, from core/toon's STAGE (the stage occupies x 384..896 of the
// 1280-wide canvas, y 256..1280 of the 1408-tall canvas) and the drawToon head
// geometry (head disc spans stage x 118..394, y 40..316, standing adult).
// Stated approximations (acceptable because every consumer is advisory): kid
// heads sit lower in the stage, sitting/jumping shift the disc, lean rotates
// the figure inside the plane. A face finding means "the face is approximately
// here" — enough to catch a face parked in the caption zone, never a gate.
function figBox(b) { const w = b.x1 - b.x0, h = b.y1 - b.y0;
  return { x0: b.x0 + w * (384 / 1280), y0: b.y0 + h * (256 / 1408),
           x1: b.x0 + w * (896 / 1280), y1: b.y0 + h * (1280 / 1408) }; }
function faceBox(f) { const w = f.x1 - f.x0, h = f.y1 - f.y0;
  return { x0: f.x0 + w * (118 / 512), y0: f.y0 + h * (40 / 1024),
           x1: f.x0 + w * (394 / 512), y1: f.y0 + h * (316 / 1024) }; }
function printComposition(audit) {
  let zone = 0, safe = 0, thirds = 0, framing = 0;
  for (const e of audit.comp.values()) {
    if (e.kind === 'zone') { zone++; console.log(`CAPTION ZONE (warning): ${e.shot} ${e.id} face in caption zone (first at t=${e.first.toFixed(2)}s, ${e.n} sampled frames)`); }
    else if (e.kind === 'rail' || e.kind === 'bottom') { safe++; console.log(`SAFE AREA (warning): ${e.shot} ${e.id} face centre ${e.kind === 'rail' ? 'under right rail' : 'in bottom UI band'} (first at t=${e.first.toFixed(2)}s, ${e.n} sampled frames)`); }
    else if (e.kind === 'exceeds' || e.kind === 'small') { framing++; console.log(`FRAMING (warning): ${e.shot} ${e.id} subject ${e.kind === 'exceeds' ? 'exceeds the frame' : 'under 15% of frame height'} (${e.detail}) (first at t=${e.first.toFixed(2)}s, ${e.n} sampled frames)`); }
  }
  for (const [shot, a] of audit.compShots) {
    if (a.focalSamples >= 4 && a.focalSamples >= 0.8 * a.samples && a.centreSamples >= 0.9 * a.focalSamples) { thirds++; console.log(`COMPOSITION (advisory): ${shot} subject dead-centre for the whole shot — check the --guides sheet`); }
  }
  const total = zone + safe + thirds + framing + audit.edge.size;
  console.log(`COMPOSITION: ${total} findings (caption-zone ${zone}, safe-area ${safe}, thirds ${thirds}, framing ${framing}, edge-touch ${audit.edge.size})`);
}
// v5 (B7, D8 §1b + ruling R1/R5): caption audit. The renderer KEEPS CAP.maxW =
// 960 (v4 pixels identical); the documented floors — rendered width ≤ 850 px,
// fit ≥ 0.68 — are enforced HERE, from OP.captions()' own measureText-based
// metrics, never a second measurer. Default: WARN only (a v4 project that
// passes today still passes). CAP_CHECK=strict folds violations into the
// CHECK verdict. R5: the width floor WINS — at the skill's own target length
// the speech runs ~0.31 s/word, so the ≥0.33 s/word time floor cannot also
// hold; it is advisory only and no audit line enforces it.
const CAP_CHECK_STRICT = String(process.env.CAP_CHECK ?? '').trim().toLowerCase() === 'strict';
function printCaptionAudit(capList) {
  const total = capList.length;
  const authored = new Set(capList.filter((c) => !c.auto).map((c) => c.line)).size;
  const lines = new Set(capList.map((c) => c.line)).size;
  const minFit = total ? Math.min(...capList.map((c) => c.fit)) : 1;
  const maxW = total ? Math.max(...capList.map((c) => c.renderedW)) : 0;
  console.log(`CAPTION AUDIT: ${total} chunks, ${authored}/${lines} lines authored, min fit ${minFit.toFixed(2)}, max rendered width ${Math.round(maxW)} px`);
  const autoLines = [...new Set(capList.filter((c) => c.auto).map((c) => c.line))].sort((a, b) => a - b);
  if (autoLines.length) console.log(`CAPTIONS: estimate-grade (auto-chunked) on lines ${autoLines.map((i) => i + 1).join(', ')} — author caps in script.json for a master`);
  let fails = 0; const seen = new Set();
  for (const c of capList) {
    const wide = c.renderedW > 850, lowFit = c.fit < 0.68;
    if (!wide && !lowFit) continue;
    const key = `${c.line}\0${c.text}`;
    if (seen.has(key)) continue; seen.add(key);
    fails++;
    const tag = CAP_CHECK_STRICT ? 'FAIL' : 'WARN';
    if (wide) console.log(`CAPTION WIDTH ${tag}: line ${c.line + 1} chunk "${c.text}" renders ${Math.round(c.renderedW)} px (> 850) at t=${c.s.toFixed(2)}s — the width floor wins (the per-word time floor is advisory); split the chunk`);
    if (lowFit) console.log(`CAPTION FIT ${tag}: line ${c.line + 1} chunk "${c.text}" fit ${c.fit.toFixed(2)} (< 0.68; raw width ${Math.round(c.rawW)} px) at t=${c.s.toFixed(2)}s — split the chunk, never shrink it`);
  }
  return CAP_CHECK_STRICT ? fails : 0;
}
const jpg = (d) => Buffer.from(d.slice(d.indexOf(',') + 1), 'base64');
const FRAME = (t, q, label) => `(() => { const d = OP.frame(${t}, ${q}); ${label ? `const cv = document.getElementById('cv'), c = cv.getContext('2d'); c.setTransform(1,0,0,1,0,0); c.fillStyle = '#000'; c.fillRect(0, 0, 250, 86); c.fillStyle = '#fff'; c.font = '700 60px sans-serif'; c.textAlign = 'left'; c.textBaseline = 'alphabetic'; c.fillText('${(+t).toFixed(2)}', 14, 64); return cv.toDataURL('image/jpeg', ${q});` : 'return d;'} })()`;

// v5 (P-10): renderer probe, factored out of check mode (I4 r08) — every page mode (check,
// sheet, frames, video) reports the renderer once, right after the page opens, so a silent
// SwiftShader fallback is visible in the mode that actually burns the render time, not only
// in a check that may have run in a different environment. Advisory only — it never fails a
// run: GPU-less machines legitimately capture QA frames on SwiftShader.
const RENDERER_PROBE = `(() => { try { const c = document.getElementById('gl'); const gl = c.getContext('webgl2') || c.getContext('webgl'); if (!gl) return 'unknown (no WebGL context)'; const ext = gl.getExtension('WEBGL_debug_renderer_info'); return String(ext ? gl.getParameter(ext.UNMASKED_RENDERER_WEBGL) : gl.getParameter(gl.RENDERER)); } catch (e) { return 'unknown (' + e.message + ')'; } })()`;
async function probeRenderer(P) {
  try {
    const renderer = await P.ev(RENDERER_PROBE);
    console.log(`renderer: ${renderer}`);
    if (/swiftshader|llvmpipe|software/i.test(renderer)) {
      if (GPU_DEFAULTED) console.log(`*** WARNING: SOFTWARE RENDERER *** "${renderer}" indicates SwiftShader/llvmpipe even though the GPU default flags were applied (/dev/dri present) — frames will render on the CPU (~3 s/frame, hot machine) instead of the GPU (~0.15 s/frame). Check the GPU/driver (or set CHROME_FLAGS explicitly) before rendering.`);
      else if (process.env.CHROME_FLAGS) console.log(`*** WARNING: SOFTWARE RENDERER *** "${renderer}" indicates SwiftShader/llvmpipe even though CHROME_FLAGS is set — frames will render on the CPU (~3 s/frame, hot machine) instead of the GPU (~0.15 s/frame). Check CHROME_FLAGS and the GPU/driver before rendering.`);
      else console.log(`*** WARNING: SOFTWARE RENDERER *** "${renderer}" indicates SwiftShader/llvmpipe — frames will render on the CPU (~3 s/frame, hot machine) instead of the GPU (~0.15 s/frame). On a Linux GPU machine capture normally applies the ANGLE/GL flags by itself when /dev/dri is present (CHROME_FLAGS overrides, SOFTWARE_GL=1 forces software); on a GPU-less machine this warning is expected for QA captures.`);
    }
  } catch (e) { console.log(`renderer: unknown (probe failed: ${e.message.split('\n')[0]})`); }
}

// v5 (Phase 3): castsheet mode — render the skill library's preview frames from code.
// The castsheet.html page (src/castsheet.ts) stages one item per CAST.draw(i) on a
// plain set; this mode saves each as <kind>-<id>.jpg (the library/index.json preview
// naming) and tiles the labelled frames into <outdir>/castsheet.jpg for eyeballing.
async function castsheetMode(P) {
  await probeRenderer(P);
  const outdir = path.resolve(ROOT, rest[0] || 'review/cast');
  const kindFilter = rest[1] || '';
  fs.mkdirSync(outdir, { recursive: true });
  const items = await P.ev('CAST.items()');
  const sel = items.map((it, i) => ({ ...it, i })).filter((it) => !kindFilter || it.kind === kindFilter);
  if (!sel.length) { console.error(`castsheet: no items${kindFilter ? ` of kind "${kindFilter}"` : ''} — kinds on this page: ${[...new Set(items.map((x) => x.kind))].join(', ')}`); process.exitCode = 1; return; }
  const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'fs-cast-'));
  let n = 0;
  for (const it of sel) {
    await P.ev(`CAST.draw(${it.i})`);
    fs.writeFileSync(path.join(outdir, `${it.kind}-${it.id}.jpg`), jpg(await P.ev(`document.getElementById('cv').toDataURL('image/jpeg', 0.92)`)));
    await P.ev(`CAST.label(${it.i})`);
    fs.writeFileSync(path.join(tmp, `f${String(n++).padStart(4, '0')}.jpg`), jpg(await P.ev(`document.getElementById('cv').toDataURL('image/jpeg', 0.8)`)));
    console.log(`${it.kind}-${it.id}.jpg`);
  }
  const cols = 4, rows = Math.ceil(n / cols), out = path.join(outdir, 'castsheet.jpg');
  const r = spawnSync('ffmpeg', ['-v', 'error', '-y', '-framerate', '1', '-i', path.join(tmp, 'f%04d.jpg'), '-vf', `scale=270:480,tile=${cols}x${rows}:padding=4:color=black`, '-frames:v', '1', '-q:v', '3', out], { encoding: 'utf8' });
  fs.rmSync(tmp, { recursive: true, force: true });
  if (r.error) throw new Error('ffmpeg failed to start: ' + r.error.message);
  if (r.status !== 0) throw new Error(`ffmpeg failed (exit ${r.status}): ` + (r.stderr || 'no stderr'));
  console.log(`${out} (${n} items${kindFilter ? `, kind=${kindFilter}` : ''})`);
}

const srv = await serve(mode === 'preview' ? 8765 : 0), port = srv.address().port;
if (mode === 'preview') { console.log(`open http://127.0.0.1:${port}/  (Ctrl+C to stop)`); }
else {
  if (mode === 'castsheet' && (!fs.existsSync(path.join(ROOT, 'castsheet.html')) || !fs.existsSync(path.join(ROOT, 'build', 'castsheet.js')))) {
    console.error('castsheet: castsheet.html / build/castsheet.js not found in this project — it predates the v5 template. Add castsheet.html + src/castsheet.ts from the v5 template, rebuild (bun run build), then re-run.');
    process.exit(1);
  }
  const P = await openPage(port, mode === 'castsheet' ? 'castsheet.html' : 'index.html', mode === 'castsheet' ? 'window.CAST ? (CAST.bootError ? "ERR:" + CAST.bootError : CAST.ready) : false' : null);
  try {
    if (mode === 'castsheet') { await castsheetMode(P); }
    else {
    const dur = await P.ev('OP.duration()');
    if (mode === 'check') {
      console.log(`duration ${dur.toFixed(2)} s`);
      // I4 (r08) + v5 (P-10): renderer probe — see probeRenderer (now shared by all page modes).
      await probeRenderer(P);
      const shotList = await P.ev('OP.shots()');
      for (const s of shotList) console.log(`${s.id.padEnd(5)} ${s.at.toFixed(2).padStart(6)} → ${s.end.toFixed(2).padStart(6)}  (${(s.end - s.at).toFixed(2)} s)  ${s.note}`);
      const capList = await P.ev('OP.captions()');
      console.log('captions:'); for (const c of capList) console.log(`  ${c.s.toFixed(2).padStart(6)}–${c.e.toFixed(2).padEnd(6)} ${c.text}`);
      // WP2: the audit is merged into this same 0.25 s loop (no second pass) —
      // each sample does draw + cutouts in ONE evaluate so the cutouts describe
      // exactly the frame just drawn.
      const audit = makeAudit(shotList);
      let bad = 0; for (let t = 0; t < dur; t += 0.25) { try { const cuts = await P.ev(`(async () => { OP.draw(${t}); return OP.cutouts(); })()`); auditSample(audit, t, cuts); } catch (e) { bad++; console.log(`ERROR at ${t.toFixed(2)}: ${e.message.split('\n')[0]}`); } }
      try { const castNames = await P.ev('OP.cast ? OP.cast() : null'); if (Array.isArray(castNames)) console.log(`CAST: ${castNames.join(', ')}`); } catch {}
      printAudit(audit);
      printContinuity(audit);
      // v5 (B7): composition/framing are WARN/ADVISORY (never in the verdict);
      // the caption audit folds in ONLY under CAP_CHECK=strict.
      printComposition(audit);
      const capFails = printCaptionAudit(capList);
      if (P.logs.length) console.log(P.logs.join('\n'));
      const failed = !!(bad || P.logs.length || audit.overflow.size || audit.violations.length || capFails);
      console.log(failed ? 'CHECK FAILED' : 'CHECK OK');
      // v5 (P-01): a failed check must fail the process too — exitCode (not exit()) so buffered
      // stdout and the finally { P.close() } cleanup still run. Successful checks still exit 0.
      if (failed) process.exitCode = 1;
    } else if (mode === 'cues') {
      const cues = await P.ev('OP.cues()');
      fs.mkdirSync(path.join(ROOT, 'data'), { recursive: true });
      fs.writeFileSync(path.join(ROOT, 'data/cues.json'), JSON.stringify({ duration: dur, cues }, null, 1));
      console.log(`data/cues.json: ${cues.length} cues, ${dur.toFixed(2)} s`);
    } else if (mode === 'frames') {
      await probeRenderer(P);
      const dir = path.resolve(ROOT, rest[0] || 'review/frames'); fs.mkdirSync(dir, { recursive: true });
      for (const t of (rest[1] || '0').split(',').map(Number)) { fs.writeFileSync(path.join(dir, `t${t.toFixed(2)}.jpg`), jpg(await P.ev(FRAME(t, 0.92)))); }
      console.log(dir);
    } else if (mode === 'sheet') {
      await probeRenderer(P);
      const out = path.resolve(ROOT, rest[0] || 'review/all.jpg'), step = +(rest[1] || 0.5), cols = +(rest[2] || 9);
      const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'fs-sheet-')); let n = 0;
      for (let t = step / 2; t < dur; t += step) fs.writeFileSync(path.join(tmp, `f${String(n++).padStart(4, '0')}.jpg`), jpg(await P.ev(FRAME(t, 0.8, true))));
      const rows = Math.ceil(n / cols); fs.mkdirSync(path.dirname(out), { recursive: true });
      const vf = flags.has('--guides') ? `scale=216:384,drawgrid=w=72:h=128:t=1:c=white@0.35,drawbox=x=0:y=268:w=216:h=42:color=red@0.55:t=1,tile=${cols}x${rows}:padding=4:color=black` : `scale=216:384,tile=${cols}x${rows}:padding=4:color=black`;
      const r = spawnSync('ffmpeg', ['-v', 'error', '-y', '-framerate', '1', '-i', path.join(tmp, 'f%04d.jpg'), '-vf', vf, '-frames:v', '1', '-q:v', '3', out], { encoding: 'utf8' });
      fs.rmSync(tmp, { recursive: true, force: true });
      // I4: spawnSync reports a missing ffmpeg as r.error (ENOENT) with r.status === null — the old
      // `if (r.status)` check treated that as success and printed a path for a file never written.
      if (r.error) throw new Error('ffmpeg failed to start: ' + r.error.message);
      if (r.status !== 0) throw new Error(`ffmpeg failed (exit ${r.status}): ` + (r.stderr || 'no stderr'));
      console.log(`${out} (${n} frames, every ${step} s)`);
      // WP2: same audit as check mode, at the sheet's own sample times
      // (t = step/2 + k*step). Overflow itself stays report-only here — but v5 (P-03):
      // a sample that throws is now COUNTED (check mode's pattern), and if any sample was
      // lost the sheet must not certify 'CANVAS OVERFLOW: none' for a page it never fully
      // audited: it reports the audit INCOMPLETE, prints 'unknown', and exits non-zero.
      try {
        const audit = makeAudit(await P.ev('OP.shots()'));
        let bad = 0; for (let t = step / 2; t < dur; t += step) { try { const cuts = await P.ev(`(async () => { OP.draw(${t}); return OP.cutouts(); })()`); auditSample(audit, t, cuts); } catch { bad++; } }
        if (bad > 0) { console.log(`ERROR: audit failed at ${bad} sample(s) — sheet audit INCOMPLETE`); printAudit(audit, true); process.exitCode = 1; }
        else printAudit(audit);
        printComposition(audit);
      } catch (e) { console.log(`CANVAS OVERFLOW: audit unavailable (${e.message.split('\n')[0]})`); }
    } else if (mode === 'video') {
      await probeRenderer(P);
      const out = path.resolve(ROOT, rest[0] || 'renders/video.mp4'); fs.mkdirSync(path.dirname(out), { recursive: true });
      // v5 (P-18): encode to a sibling temp file and rename to `out` only after the exit-code
      // check below passes (same directory, so the rename is atomic on one filesystem). A
      // killed or failed encode leaves at worst this clearly-named .part file — never a
      // truncated MP4 at the real path that looks like a finished render.
      const part = `${out}.part.mp4`;
      const mix = path.join(ROOT, 'data/mix.wav'), hasMix = fs.existsSync(mix), a0 = rest[1] ? +rest[1] : 0, a1 = rest[2] ? Math.min(+rest[2], dur) : dur, n = Math.round((a1 - a0) * FPS);
      const draft = flags.has('--draft');
      // I4 (experimental, r07): FRAME_TRANSPORT=raw swaps only the input declaration + frame source;
      // the x264/output argument sets below are identical on both transports.
      const useRaw = process.env.FRAME_TRANSPORT === 'raw';
      if (useRaw) console.log('FRAME_TRANSPORT=raw: EXPERIMENTAL raw RGBA frame transport (pending the r07 V1-V7 validation on the GPU PC) — default is the JPEG path');
      const inArgs = useRaw ? ['-f', 'rawvideo', '-pixel_format', 'rgba', '-video_size', `${RAW_W}x${RAW_H}`, '-framerate', String(FPS), '-i', '-']
        : ['-f', 'image2pipe', '-framerate', String(FPS), '-i', '-'];
      // I4 (r09 §9 + a12 G3): master = preset slow (CRF stays 18), x264 -threads 4, AAC 256k, explicit
      // -ar 48000, BT.709 colour tags; draft = veryfast CRF 23 (was ultrafast/28), AAC stays 192k.
      // The -x264-params colour triple backs up the generic -color_* options: verified on ffmpeg
      // 8.1.2, the generic options alone tag matrix+range but leave primaries/transfer "unknown"
      // with libx264, while the x264-params triple lands all three in the VUI (tags only, no pixels).
      const encArgs = draft ? ['-c:v', 'libx264', '-preset', 'veryfast', '-crf', '23', '-pix_fmt', 'yuv420p', '-c:a', 'aac', '-b:a', '192k', '-ar', '48000']
        : ['-c:v', 'libx264', '-preset', 'slow', '-crf', '18', '-threads', '4', '-pix_fmt', 'yuv420p', '-color_primaries', 'bt709', '-color_trc', 'bt709', '-colorspace', 'bt709', '-color_range', 'tv', '-x264-params', 'colorprim=bt709:transfer=bt709:colormatrix=bt709', '-c:a', 'aac', '-b:a', '256k', '-ar', '48000'];
      const ff = spawn('ffmpeg', ['-v', 'error', '-y', ...inArgs,
        ...(hasMix ? ['-ss', String(a0), '-t', String(a1 - a0), '-i', mix] : ['-f', 'lavfi', '-i', 'anullsrc=r=48000:cl=stereo']), '-map', '0:v', '-map', '1:a',
        ...encArgs, '-shortest', '-movflags', '+faststart', part], { stdio: ['pipe', 'inherit', 'inherit'] });
      // I4: ffmpeg failures were previously invisible — the close code was discarded and success was
      // printed unconditionally. Capture spawn/stdin errors and fail loudly on a non-zero exit.
      let ffErr = null; ff.on('error', (e) => { ffErr = ffErr || e; }); ff.stdin.on('error', (e) => { ffErr = ffErr || e; });
      // v5 (P-18): any failure inside this block removes the .part temp before the error
      // propagates; success renames it onto `out` after the exit-code check.
      try {
        const t0 = Date.now();
        for (let i = 0; i < n; i++) {
          let b;
          if (useRaw) {
            const got = takeRawFrame(); // register the waiter before the page POSTs (see serve())
            const len = await P.ev(RAW_FRAME(a0 + i / FPS));
            b = await got;
            if (len !== RAW_BYTES || b.length !== RAW_BYTES) throw new Error(`raw frame ${i}: expected ${RAW_BYTES} bytes (${RAW_W}x${RAW_H} RGBA), page sent ${len}, server received ${b.length}`);
          } else b = jpg(await P.ev(FRAME(a0 + i / FPS, draft ? 0.8 : 0.93)));
          if (!ff.stdin.write(b)) await new Promise((r) => ff.stdin.once('drain', r));
          if (ffErr) throw new Error('ffmpeg: ' + ffErr.message);
          if (i % 150 === 0) process.stdout.write(`\r${i}/${n} frames`);
        }
        ff.stdin.end(); const ffCode = await new Promise((r) => ff.on('close', r));
        if (ffErr) throw new Error('ffmpeg: ' + ffErr.message);
        if (ffCode !== 0) throw new Error(`ffmpeg failed with exit code ${ffCode} — ${out} is missing or incomplete, do NOT use it`);
        fs.renameSync(part, out);
        console.log(`\r${out} (${n} frames, ${hasMix ? 'with data/mix.wav' : 'SILENT: no data/mix.wav'}, ${((Date.now() - t0) / 1000).toFixed(0)} s)`);
      } catch (e) { try { fs.rmSync(part, { force: true }); } catch {} throw e; }
    } else console.log('modes: check · cues · sheet · frames · video · preview · castsheet');
    }
  } finally { P.close(); srv.close(); }
}
