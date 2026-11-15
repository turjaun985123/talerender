// sheet_mode.mjs — parity-complete native `sheet` for capture_native.mjs
// (CR-B1-04). Self-contained: the lead can either spawn it as a CLI or
// import { runSheet } and pass the adapter's context.
//
// What it fixes over capture_native.mjs's built-in sheet (all verified
// against template/tools/capture.mjs in CR-B1-04-PARITY.md):
//   1. AUDIT: Chrome's sheet runs the cutouts audit after tiling and
//      prints CANVAS OVERFLOW / EDGE TOUCH + composition lines
//      (report-only; a lost audit fails the sheet). The adapter's sheet
//      ran none. This module runs the binary's film telemetry after
//      tiling and prints the same line classes (overflow, edge-touch,
//      CAPTION ZONE / SAFE AREA / FRAMING / COMPOSITION summary),
//      sampled at telemetry's 0.25 s — a finer superset of Chrome
//      sheet's step-spaced samples, labelled as such. If telemetry
//      cannot run, it prints Chrome's own fallback line
//      `CANVAS OVERFLOW: audit unavailable (<reason>)` and exits 1 —
//      a sheet never certifies an audit it did not run.
//   2. LABEL FONT: Chrome burns labels in a bold SANS (canvas
//      '700 60px sans-serif'). The adapter picks the alphabetically
//      first font under fonts/ — on the Onoda project that is
//      Caladea-Bold.ttf, a serif. This module prefers a sans
//      (Inter / *Sans* / DejaVu Sans Bold system font) before any
//      other project font.
//   3. LABEL BOX: Chrome's label is a FIXED 250x86 black box, text at
//      (14, baseline 64). drawtext's box auto-sizes to the glyphs, so
//      the adapter's box geometry drifts with the timestamp's digit
//      count. This module draws the fixed box with drawbox (fill)
//      and places drawtext inside it (no auto box), matching the
//      250x86 geometry.
// Everything else is the adapter's/Chrome's shared contract, kept
// byte-identical: sample times t = step/2 + k*step, the tile filter
// string (incl. --guides drawgrid + caption-zone drawbox), the final
// JPEG encode (-q:v 3), the output log line shape. Per-tile JPEG
// quality stays ffmpeg -q:v 2 (a fidelity SUPERSET of Chrome's
// canvas q0.8 — preview-grade either way; not a parity gap).
//
// CLI:  node tools/native_modes/sheet_mode.mjs <out.jpg> [step] [cols] [--guides] [--no-audit]
// Env:  same discovery as capture_native.mjs (TALERENDER_BIN,
//       NATIVE_DIR, NATIVE_FILM_DIR, NATIVE_FRAMES_DIR, FPS).
// Exit: 0 sheet written (audit reported, never folded into the sheet
//       verdict — Chrome parity); 1 prerequisite/render failure or
//       audit unavailable; 2 usage.
import { spawn, spawnSync } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const ROOT = path.resolve(HERE, '..', '..');
const FPS = +(process.env.FPS || 30);
if (!Number.isFinite(FPS) || FPS <= 0) { console.error('sheet_mode: invalid FPS'); process.exit(2); }
const NATIVE_DIR = path.resolve(ROOT, process.env.NATIVE_DIR || 'native');
const FRAMES_DIR = path.resolve(ROOT, process.env.NATIVE_FRAMES_DIR || path.join('renders', 'native-frames'));
const framePath = (n) => path.join(FRAMES_DIR, `frame_${String(n).padStart(5, '0')}.png`);
function fail(msg) { throw new Error(msg); }
function isExec(p) { try { return fs.statSync(p).isFile() && (fs.accessSync(p, fs.constants.X_OK), true); } catch { return false; } }

export function findBinary() {
  const env = process.env.TALERENDER_BIN;
  if (env) { if (isExec(env)) return env; fail(`TALERENDER_BIN="${env}" is not an executable file`); }
  for (const p of [path.join(NATIVE_DIR, 'talerender'), path.join(os.homedir(), 'workspace', 'talerender', 'talerender')]) if (isExec(p)) return p;
  for (const d of String(process.env.PATH || '').split(path.delimiter)) { if (!d) continue; const p = path.join(d, 'talerender'); if (isExec(p)) return p; }
  fail('talerender binary not found (set TALERENDER_BIN)');
}
export function findFilmDir() {
  if (process.env.NATIVE_FILM_DIR) { const p = path.resolve(ROOT, process.env.NATIVE_FILM_DIR); if (fs.existsSync(path.join(p, 'onoda.json'))) return p; fail(`NATIVE_FILM_DIR="${p}" has no onoda.json`); }
  for (const p of [path.join(NATIVE_DIR, 'film'), NATIVE_DIR]) if (fs.existsSync(path.join(p, 'onoda.json'))) return p;
  fail('film data dir not found (no onoda.json)');
}
export function loadManifest() {
  for (const p of [path.join(NATIVE_DIR, 'film.json'), path.join(NATIVE_DIR, 'scenes', 'film', 'film.json')]) {
    if (fs.existsSync(p)) { const m = JSON.parse(fs.readFileSync(p, 'utf8')); return { ...m, W: m.resolution?.width, H: m.resolution?.height }; }
  }
  fail('native film manifest not found');
}
const PNG_SIG = [137, 80, 78, 71, 13, 10, 26, 10];
export function pngCheck(p, W, H) {
  let st; try { st = fs.statSync(p); } catch { return 'missing'; }
  if (!st.isFile() || st.size < 45) return 'truncated/not a file';
  let fd;
  try {
    fd = fs.openSync(p, 'r');
    const head = Buffer.alloc(24); fs.readSync(fd, head, 0, 24, 0);
    for (let i = 0; i < 8; i++) if (head[i] !== PNG_SIG[i]) return 'bad PNG signature';
    if (head.readUInt32BE(16) !== W || head.readUInt32BE(20) !== H) return 'wrong dimensions';
    const tail = Buffer.alloc(12); fs.readSync(fd, tail, 0, 12, st.size - 12);
    if (tail.readUInt32BE(0) !== 0 || tail.toString('ascii', 4, 8) !== 'IEND') return 'no closing IEND — truncated';
    return null;
  } catch (e) { return `unreadable: ${e.message}`; } finally { if (fd !== undefined) { try { fs.closeSync(fd); } catch {} } }
}
async function renderIndices(bin, filmDir, manifest, indices) {
  fs.mkdirSync(FRAMES_DIR, { recursive: true });
  for (const n of indices) {
    const p = framePath(n);
    if (fs.existsSync(p) && pngCheck(p, manifest.W, manifest.H)) fs.rmSync(p, { force: true });
  }
  for (const n of [...new Set(indices)].sort((a, b) => a - b)) {
    if (!fs.existsSync(framePath(n))) {
      const code = await new Promise((res, rej) => { const pr = spawn(bin, ['--film', filmDir, '--out', FRAMES_DIR, '--from', String(n), '--to', String(n + 1), '--width', String(manifest.W), '--height', String(manifest.H)], { stdio: 'inherit' }); pr.on('error', rej); pr.on('close', res); }).catch((e) => fail(`talerender failed to start: ${e.message}`));
      if (code !== 0) fail(`talerender exited ${code} rendering frame ${n}`);
    }
    const bad = pngCheck(framePath(n), manifest.W, manifest.H);
    if (bad) fail(`frame ${n} invalid after render (${bad})`);
  }
}
// Sans-first font choice (fix 2): project fonts matching sans/inter
// first, then a system DejaVu Sans Bold (the usual headless-Chrome
// 'sans-serif' face), then any project font, else null (unlabelled).
export function findFont(root = ROOT) {
  const dir = path.join(root, 'fonts');
  let files = [];
  try { files = fs.readdirSync(dir).filter((x) => /\.(otf|ttf)$/i.test(x)).sort(); } catch {}
  const sans = files.find((f) => /inter|sans|dejavu|noto/i.test(f));
  if (sans) return path.join(dir, sans);
  for (const p of ['/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf']) if (fs.existsSync(p)) return p;
  return files.length ? path.join(dir, files[0]) : null;
}
const drawEsc = (s) => s.replace(/\\/g, '\\\\').replace(/:/g, '\\:').replace(/'/g, "\\'");
let labelWarned = false;
function sampleJpeg(png, jpgOut, label, font) {
  if (label && font) {
    // Fix 3: fixed 250x86 black box (drawbox fill) + text inside it.
    // Chrome: fillRect(0,0,250,86), fillText at (14, baseline 64) in
    // 60px bold — drawtext y is the text top, ~baseline-ascender, so
    // y=8 approximates the canvas placement at this size.
    const vf = `drawbox=x=0:y=0:w=250:h=86:color=black:t=fill,drawtext=fontfile='${drawEsc(font)}':text='${label}':x=14:y=8:fontsize=60:fontcolor=white`;
    const r = spawnSync('ffmpeg', ['-v', 'error', '-y', '-i', png, '-vf', vf, '-frames:v', '1', '-q:v', '2', jpgOut], { encoding: 'utf8' });
    if (!r.error && r.status === 0) return;
    if (!labelWarned) { labelWarned = true; console.log('sheet: timestamp labels unavailable (ffmpeg drawtext failed) — tiles are unlabelled'); }
  } else if (label && !font && !labelWarned) { labelWarned = true; console.log('sheet: no usable font found — tiles are unlabelled'); }
  const r = spawnSync('ffmpeg', ['-v', 'error', '-y', '-i', png, '-frames:v', '1', '-q:v', '2', jpgOut], { encoding: 'utf8' });
  if (r.error) fail('ffmpeg failed to start: ' + r.error.message);
  if (r.status !== 0) fail(`ffmpeg failed converting ${png} (exit ${r.status}): ` + (r.stderr || ''));
}
// The audit line classes Chrome's sheet prints (printAudit +
// printComposition in capture.mjs) — extracted from telemetry stdout.
const AUDIT_LINE = /^(CANVAS OVERFLOW:|EDGE TOUCH|CAPTION ZONE|SAFE AREA|FRAMING|COMPOSITION[: ]|COMPOSITION:)/;
export function auditFromTelemetry(bin, filmDir) {
  const r = spawnSync(bin, ['--film-telemetry', filmDir], { encoding: 'utf8', maxBuffer: 64 * 1024 * 1024 });
  if (r.error) return { ok: false, reason: r.error.message };
  const lines = String(r.stdout || '').split('\n');
  if (!lines.some((l) => l.startsWith('FILM TELEMETRY: CHECK'))) {
    const err = String(r.stderr || '').split('\n').find((l) => l.trim()) || `binary exit ${r.status}`;
    return { ok: false, reason: err.trim() };
  }
  return { ok: true, lines: lines.filter((l) => AUDIT_LINE.test(l)) };
}

export async function runSheet({ out, step = 0.5, cols = 9, guides = false, audit = true, bin, filmDir, manifest } = {}) {
  bin = bin || findBinary(); filmDir = filmDir || findFilmDir(); manifest = manifest || loadManifest();
  out = path.resolve(ROOT, out || 'review/all.jpg');
  if (!Number.isFinite(step) || step <= 0 || !Number.isInteger(cols) || cols < 1) { console.error('sheet_mode: sheet needs a positive step and a positive integer column count'); process.exit(2); }
  const dur = manifest.durationS, samples = [];
  for (let t = step / 2; t < dur; t += step) samples.push(t);
  const indices = samples.map((t) => Math.min(Math.round(t * FPS), manifest.totalFrames - 1));
  await renderIndices(bin, filmDir, manifest, indices);
  const font = findFont();
  const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'fs-native-sheet-'));
  try {
    samples.forEach((t, i) => sampleJpeg(framePath(indices[i]), path.join(tmp, `f${String(i).padStart(4, '0')}.jpg`), t.toFixed(2), font));
    const rows = Math.ceil(samples.length / cols);
    fs.mkdirSync(path.dirname(out), { recursive: true });
    const vf = guides ? `scale=216:384,drawgrid=w=72:h=128:t=1:c=white@0.35,drawbox=x=0:y=268:w=216:h=42:color=red@0.55:t=1,tile=${cols}x${rows}:padding=4:color=black` : `scale=216:384,tile=${cols}x${rows}:padding=4:color=black`;
    const r = spawnSync('ffmpeg', ['-v', 'error', '-y', '-framerate', '1', '-i', path.join(tmp, 'f%04d.jpg'), '-vf', vf, '-frames:v', '1', '-q:v', '3', out], { encoding: 'utf8' });
    if (r.error) fail('ffmpeg failed to start: ' + r.error.message);
    if (r.status !== 0) fail(`ffmpeg failed (exit ${r.status}): ` + (r.stderr || ''));
  } finally { fs.rmSync(tmp, { recursive: true, force: true }); }
  console.log(`${out} (${samples.length} frames, every ${step} s)`);
  if (audit) {
    const a = auditFromTelemetry(bin, filmDir);
    if (!a.ok) { console.log(`CANVAS OVERFLOW: audit unavailable (${a.reason})`); process.exitCode = 1; }
    else { for (const l of a.lines) console.log(l); console.log('sheet audit: native film telemetry at 0.25 s sampling (finer than this sheet\'s step; report-only, as in capture.mjs sheet)'); }
  }
  return out;
}

const isMain = process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url);
if (isMain) {
  const argv = process.argv.slice(2), fl = new Set(argv.filter((a) => a.startsWith('--')));
  const [out, step, cols] = argv.filter((a) => !a.startsWith('--'));
  runSheet({ out, step: step ? +step : 0.5, cols: cols ? +cols : 9, guides: fl.has('--guides'), audit: !fl.has('--no-audit') })
    .catch((e) => { console.error(`sheet_mode: ${e.message}`); process.exit(1); });
}
