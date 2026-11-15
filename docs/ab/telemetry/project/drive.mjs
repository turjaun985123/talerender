// RETIRED (Chrome-replacement wave, 2026-11-15): Chrome-era A/B evidence harness. Do not run — the native pipeline has no browser path. Kept with the docs/ab evidence it produced.
// CDP driver for the REAL Onoda project page (M6 agreement proof).
// Serves a project root, boots headless Chrome (SwiftShader — the
// renderer is labelled in the output), and evaluates the page's own
// OP surface exactly as tools/capture does: draw(t) + cutouts() in
// ONE evaluate per sample, so the telemetry describes the frame
// just drawn (capture.ts check mode's protocol, a2 §3.1).
//
// Usage: node drive.mjs <project-root> <out-dir>
// Writes: <out-dir>/chrome-cutouts.json  (OP.cutouts() at t=1.50)
//         <out-dir>/chrome-meta.json     (renderer, duration, captions, logs)
import http from 'node:http';
import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import { spawn } from 'node:child_process';

const ROOT = path.resolve(process.argv[2]);
const OUT = path.resolve(process.argv[3]);
fs.mkdirSync(OUT, { recursive: true });
const CHROME = process.env.CHROME || path.join(os.homedir(), '.faceless-shorts/chrome/opt/google/chrome/chrome');
const T = 1.5;

const MIME = { '.html': 'text/html', '.js': 'text/javascript', '.mjs': 'text/javascript', '.json': 'application/json', '.otf': 'font/otf', '.ttf': 'font/ttf', '.woff2': 'font/woff2', '.wav': 'audio/wav', '.mp3': 'audio/mpeg', '.png': 'image/png', '.jpg': 'image/jpeg' };
const srv = http.createServer((req, res) => {
  const rel = decodeURIComponent(new URL(req.url, 'http://x').pathname).slice(1) || 'index.html';
  const f = path.join(ROOT, rel);
  if (!f.startsWith(ROOT) || !fs.existsSync(f) || !fs.statSync(f).isFile()) { res.writeHead(404); res.end(); return; }
  res.writeHead(200, { 'Content-Type': MIME[path.extname(f)] || 'application/octet-stream', 'Cache-Control': 'no-store' });
  fs.createReadStream(f).pipe(res);
});
await new Promise((r) => srv.listen(0, '127.0.0.1', r));
const port = srv.address().port;

const prof = fs.mkdtempSync(path.join(os.tmpdir(), 'm6-chrome-'));
const chrome = spawn(CHROME, [
  '--headless=new', '--remote-debugging-port=0', `--user-data-dir=${prof}`,
  '--no-first-run', '--no-default-browser-check', '--hide-scrollbars',
  '--window-size=1080,1920', '--ignore-gpu-blocklist', '--enable-unsafe-swiftshader',
  '--use-angle=swiftshader', '--no-sandbox',
  '--disable-background-timer-throttling', '--disable-renderer-backgrounding', 'about:blank',
], { stdio: 'ignore' });

let dbgPort = 0;
for (let i = 0; i < 200 && !dbgPort; i++) {
  const f = path.join(prof, 'DevToolsActivePort');
  if (fs.existsSync(f)) dbgPort = Number(fs.readFileSync(f, 'utf8').split('\n')[0]);
  else await new Promise((r) => setTimeout(r, 100));
}
if (!dbgPort) throw new Error('Chrome did not start');
const pageUrl = `http://127.0.0.1:${port}/index.html?capture=1`;
const target = await (await fetch(`http://127.0.0.1:${dbgPort}/json/new?${encodeURIComponent(pageUrl)}`, { method: 'PUT' })).json();
const ws = new WebSocket(target.webSocketDebuggerUrl);
await new Promise((r) => ws.addEventListener('open', r, { once: true }));
let nextId = 1;
const pending = new Map();
const logs = [];
ws.addEventListener('message', (ev) => {
  const m = JSON.parse(ev.data);
  if (m.id && pending.has(m.id)) { pending.get(m.id)(m); pending.delete(m.id); }
  else if (m.method === 'Runtime.exceptionThrown') logs.push('EXCEPTION ' + (m.params.exceptionDetails.exception?.description || m.params.exceptionDetails.text || '').split('\n')[0]);
  else if (m.method === 'Runtime.consoleAPICalled' && m.params.type === 'error') logs.push('ERROR ' + m.params.args.map((a) => a.value ?? a.description ?? '').join(' '));
});
const send = (method, params = {}) => new Promise((res) => { const id = nextId++; pending.set(id, res); ws.send(JSON.stringify({ id, method, params })); });
const ev = async (expr) => {
  const m = await send('Runtime.evaluate', { expression: expr, returnByValue: true, awaitPromise: true });
  if (m.result.exceptionDetails) throw new Error((m.result.exceptionDetails.exception?.description || m.result.exceptionDetails.text || 'evaluate failed').split('\n')[0]);
  return m.result.result.value;
};
await send('Runtime.enable');

try {
  let ready = false;
  for (let i = 0; i < 1200 && !ready; i++) {
    const r = await ev('window.OP ? (OP.bootError ? "ERR:" + OP.bootError : OP.ready) : false');
    if (r === true) ready = true;
    else if (typeof r === 'string') throw new Error('boot failed: ' + r);
    else await new Promise((res) => setTimeout(res, 100));
  }
  if (!ready) throw new Error('boot timeout');
  const renderer = await ev(`(() => { try { const c = document.getElementById('gl'); const gl = c.getContext('webgl2') || c.getContext('webgl'); if (!gl) return 'unknown (no WebGL context)'; const ext = gl.getExtension('WEBGL_debug_renderer_info'); return String(ext ? gl.getParameter(ext.UNMASKED_RENDERER_WEBGL) : gl.getParameter(gl.RENDERER)); } catch (e) { return 'unknown (' + e.message + ')'; } })()`);
  const duration = await ev('OP.duration()');
  const cuts = await ev(`(async () => { OP.draw(${T}); return OP.cutouts(); })()`);
  const captions = await ev('OP.captions()');
  const cast = await ev('OP.cast ? OP.cast() : null');
  fs.writeFileSync(path.join(OUT, 'chrome-cutouts.json'), JSON.stringify(cuts, null, 1));
  fs.writeFileSync(path.join(OUT, 'chrome-meta.json'), JSON.stringify({ renderer, duration, captions, cast, logs, t: T }, null, 1));
  console.log('renderer:', renderer);
  console.log('duration:', duration, 'cutouts:', cuts.length, 'captions:', captions.length, 'logs:', logs.length);
} finally {
  try { ws.close(); } catch {}
  chrome.kill('SIGKILL');
  srv.close();
  setTimeout(() => fs.rmSync(prof, { recursive: true, force: true }), 300);
}
