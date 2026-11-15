// Native capture backend — the same capture contract as tools/capture.mjs, rendered by
// TaleRender (native Rust + wgpu) instead of headless Chrome. No npm packages (Node 22+).
// Run from the project folder, with the backend selected explicitly:
//   RENDERER=native node tools/capture_native.mjs check            film telemetry audit (the QA gate) over the whole film — full parity report (native_modes/check_mode.mjs)
//   RENDERER=native node tools/capture_native.mjs frames review/frames "1.2,8.4"   clean PNG frames at those times (t<time>.png)
//   RENDERER=native node tools/capture_native.mjs sheet review/all.jpg [step=0.5] [cols=9] [--guides]   review contact sheet from native frames, with the telemetry audit (native_modes/sheet_mode.mjs)
//   RENDERER=native node tools/capture_native.mjs video renders/out.mp4 [start end] [--draft]   the film (30 fps) with data/mix.wav if it exists
//   RENDERER=native node tools/capture_native.mjs castsheet [outdir] [kind]   native cast sheet (native_modes/castsheet_mode.mjs)
//   cues   authored data, not a render: routed to tools/cues_native.py — this adapter prints the route + data/cues.json freshness and exits 1 (it never produces cues.json itself)
//   preview   retired with Chrome: it only ever served the project page. Native review deliverables are sheet / frames / video.
// There is no Chrome fallback for any mode in this adapter.
// Full contract: tools/CAPTURE-NATIVE.md. Backend overview: docs/NATIVE-BACKEND.md (skill tree).
//
// Native-first template (Chrome-replacement Batch 2, B2-01): this adapter IS the
// production capture backend — the template ships no browser capture file. This
// adapter refuses to run under an explicit RENDERER=<other> selection so a
// mislabelled render cannot slip through.
//
// The binary: TALERENDER_BIN=/path/to/talerender wins; otherwise discovery tries the
// project native dir (<project>/native/talerender), the TaleRender repo checkout
// (~/workspace/talerender/talerender), then PATH. The binary is a GitHub Actions artifact
// (turjaun985123/talerender, artifact "talerender-binary"); there is no local build.
// Film data: NATIVE_DIR (default <project>/native) holds film.json (the manifest) and
// film/ (the binary's data dir: onoda.json + assets/). NATIVE_FILM_DIR overrides the latter.
// Rendered PNG frames cache in NATIVE_FRAMES_DIR (default <project>/renders/native-frames).
//
// GATE: frames/video run the film telemetry audit first (talerender --film-telemetry).
// The binary prints its verdict as a stdout line — "FILM TELEMETRY: CHECK OK" /
// "FILM TELEMETRY: CHECK FAILED (...)". The LINE is authoritative and identical
// under both binary generations: OLD binaries exit 0 either way (errors also exit
// 0, with "film telemetry failed: ..." on stderr); NEW binaries agree with the
// line (0 = OK, 1 = FAILED, 3 = telemetry could not run, 2 = CLI error). The
// adapter accepts both conventions and fails closed on any disagreement between
// line and exit code, and on a missing verdict line. Spec:
// docs/integration/EXIT-CODES-W02.md in the TaleRender repo.
import { spawn, spawnSync } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { dispatchChangedMode, isChangedMode } from './native_modes/mode_dispatch.mjs';
import { writeRenderMeta } from './native_modes/render_meta.mjs';
import { eraHintMessage } from './native_modes/era_hint.mjs';
import { validateFilmData } from './native_modes/filmdata_validate.mjs';
import { runCheck } from './native_modes/check_mode.mjs';
import { runSheet } from './native_modes/sheet_mode.mjs';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const FPS = +(process.env.FPS || 30);
// Same guard as capture.mjs (I4): FPS must be a positive finite number.
if (!Number.isFinite(FPS) || FPS <= 0) { console.error(`capture-native: invalid FPS "${process.env.FPS}" — FPS must be a positive finite number (e.g. FPS=30)`); process.exit(1); }
const args = process.argv.slice(2), flags = new Set(args.filter((a) => a.startsWith('--')));
const [mode, ...rest] = args.filter((a) => !a.startsWith('--'));

// Backend selection guard: an explicit non-native RENDERER selection never runs here.
const RENDERER_SEL = String(process.env.RENDERER ?? '').trim().toLowerCase();
if (RENDERER_SEL && RENDERER_SEL !== 'native') {
  console.error(`capture-native: RENDERER="${process.env.RENDERER}" does not select the native backend, and this adapter is the native backend only.\n  Native:               RENDERER=native node tools/capture_native.mjs <mode>\nSelection is explicit; the adapter never falls back silently, and this template ships no other capture backend.`);
  process.exit(2);
}

const NATIVE_DIR = path.resolve(ROOT, process.env.NATIVE_DIR || 'native');
const FRAMES_DIR = path.resolve(ROOT, process.env.NATIVE_FRAMES_DIR || path.join('renders', 'native-frames'));
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
function fail(msg) { throw new Error(msg); }
function readJson(p, what) {
  try { return JSON.parse(fs.readFileSync(p, 'utf8')); }
  catch (e) { fail(`${what}: cannot read/parse ${p}: ${e.message}`); }
}
function isExec(p) {
  try { return fs.statSync(p).isFile() && (fs.accessSync(p, fs.constants.X_OK), true); } catch { return false; }
}

// ---------------------------------------------------------------------
// Prerequisites: binary discovery + film-data validation (fail closed,
// every missing item named — the adapter never improvises film data).
// ---------------------------------------------------------------------
function findBinary() {
  const env = process.env.TALERENDER_BIN;
  if (env) {
    if (isExec(env)) return { bin: env, via: 'TALERENDER_BIN' };
    fail(`TALERENDER_BIN="${env}" is not an executable file. An explicit TALERENDER_BIN is never second-guessed — fix the path or unset it to use discovery.`);
  }
  const candidates = [
    [path.join(NATIVE_DIR, 'talerender'), 'project native dir'],
    [path.join(os.homedir(), 'workspace', 'talerender', 'talerender'), 'TaleRender repo checkout'],
  ];
  for (const [p, via] of candidates) if (isExec(p)) return { bin: p, via };
  for (const d of String(process.env.PATH || '').split(path.delimiter)) {
    if (!d) continue;
    const p = path.join(d, 'talerender');
    if (isExec(p)) return { bin: p, via: 'PATH' };
  }
  fail(`talerender binary not found. Tried: ${candidates.map(([p]) => p).join(', ')}, then PATH.\nSet TALERENDER_BIN=/path/to/talerender — the binary is the "talerender-binary" artifact from a green CI run of turjaun985123/talerender (there is no local build).`);
}
function findManifest() {
  for (const p of [path.join(NATIVE_DIR, 'film.json'), path.join(NATIVE_DIR, 'scenes', 'film', 'film.json')]) {
    if (fs.existsSync(p)) return p;
  }
  fail(`native film manifest not found. Tried: ${path.join(NATIVE_DIR, 'film.json')} and ${path.join(NATIVE_DIR, 'scenes', 'film', 'film.json')}.\nThe native backend captures only projects bridged to film data; a project without it is not bridged yet — bridge it first, then re-run. Build the bridge from the project's own data with the bridge builder (tools/bridge/build_bridge.py in the talerender repo); for film data the repo already holds, package it instead (tools/bridge/package_bridge.py, same repo). See tools/CAPTURE-NATIVE.md for the layout. There is no other capture backend to fall back to.`);
}
function findFilmDir() {
  if (process.env.NATIVE_FILM_DIR) {
    const p = path.resolve(ROOT, process.env.NATIVE_FILM_DIR);
    if (fs.existsSync(path.join(p, 'onoda.json'))) return p;
    fail(`NATIVE_FILM_DIR="${p}" does not contain onoda.json — an explicit film dir is never second-guessed.`);
  }
  for (const p of [path.join(NATIVE_DIR, 'film'), NATIVE_DIR]) {
    if (fs.existsSync(path.join(p, 'onoda.json'))) return p;
  }
  fail(`film data dir not found: no onoda.json in ${path.join(NATIVE_DIR, 'film')} or ${NATIVE_DIR}. The binary's film data (onoda.json + assets/) is generated by the bridge, never improvised at capture time. Bridge the project first — build the bridge from the project's own data with tools/bridge/build_bridge.py, or package film data the repo already holds with tools/bridge/package_bridge.py (both in the talerender repo) — then re-run. See tools/CAPTURE-NATIVE.md for the layout.`);
}
// Manifest: format "talerender-film" v1 — fps/resolution/totalFrames plus the
// segment table. Validated against the binary's own rules (frame n at t = n/fps;
// totalFrames = ceil(durationS * fps); segment frame ranges tile [0, totalFrames)).
// Every problem found is collected and reported together (W54/F10) — a tiling
// fault no longer hides a missing scene file behind it.
function loadManifest(manifestPath) {
  const m = readJson(manifestPath, 'film manifest');
  const errors = [];
  if (m.format !== 'talerender-film') errors.push(`film manifest ${manifestPath}: format is "${m.format}", expected "talerender-film"`);
  if (m.version !== 1) errors.push(`film manifest ${manifestPath}: version ${m.version} not supported by this adapter (expected 1)`);
  if (m.fps !== FPS) errors.push(`film manifest is ${m.fps} fps but FPS=${FPS} — the binary renders at the manifest rate; re-run with FPS=${m.fps}`);
  const W = m.resolution?.width, H = m.resolution?.height;
  if (!(Number.isInteger(W) && W > 0 && Number.isInteger(H) && H > 0)) errors.push(`film manifest ${manifestPath}: resolution missing/invalid`);
  if (!Array.isArray(m.segments) || !m.segments.length) errors.push(`film manifest ${manifestPath}: no segments`);
  if (Array.isArray(m.segments)) {
    let expect = 0;
    for (const s of m.segments) {
      if (s.startFrame !== expect || !(s.endFrame > s.startFrame)) errors.push(`film manifest: segment ${s.id || '?'} frame range ${s.startFrame}..${s.endFrame} does not tile [0, ${m.totalFrames}) (expected start ${expect})`);
      if (Number.isInteger(s.endFrame)) expect = s.endFrame;
      const scenePath = path.resolve(NATIVE_DIR, s.scene || '');
      if (!s.scene || !fs.existsSync(scenePath)) errors.push(`film manifest: segment ${s.id || '?'} scene file missing: ${scenePath}`);
    }
    if (m.segments.length && expect !== m.totalFrames) errors.push(`film manifest: segments tile to frame ${expect}, manifest totalFrames is ${m.totalFrames}`);
  }
  if (typeof m.durationS === 'number' && typeof m.fps === 'number' && typeof m.totalFrames === 'number') {
    const wantTotal = Math.ceil(m.durationS * m.fps);
    if (wantTotal !== m.totalFrames) errors.push(`film manifest: totalFrames ${m.totalFrames} != ceil(durationS ${m.durationS} * fps ${m.fps}) = ${wantTotal} — manifest disagrees with the binary's frame rule`);
  }
  if (errors.length) fail(errors.join('\n'));
  return { ...m, W, H };
}
// Film-data validation lives in native_modes/filmdata_validate.mjs
// (imported above): data-driven required-asset derivation from the
// film data + plates manifest, replacing the hardcoded Onoda asset
// list that used to sit here (WP9 P4, CR-B1-06). Same call shape:
// validateFilmData(filmDir, manifest) — fails with the full missing
// list so a broken bridge fails HERE instead of as a Rust panic
// mid-capture; `manifest` may be null (W54/F10 accumulation).
// ---------------------------------------------------------------------
// The gate. The verdict LINE is authoritative and byte-identical under
// both binary generations (EXIT-CODES-W02): old binaries exit 0 on
// pass, on fail, and on error; new binaries agree with the line
// (0 = CHECK OK, 1 = CHECK FAILED, 3 = could not run). The adapter
// therefore cross-checks line against exit code: a FAILED line passes
// the gate's fail path under either generation's code (0 or 1), an OK
// line passes only with exit 0, and any disagreement — or no verdict
// line at all — fails the capture closed, with the binary's full
// output already echoed.
// ---------------------------------------------------------------------
function runTelemetry(bin, filmDir, via) {
  const r = spawnSync(bin, ['--film-telemetry', filmDir], { encoding: 'utf8', maxBuffer: 64 * 1024 * 1024 });
  if (r.error) fail(`film telemetry failed to start: ${r.error.message}`);
  if (r.stdout) process.stdout.write(r.stdout);
  if (r.stderr) process.stderr.write(r.stderr);
  const lines = String(r.stdout || '').split('\n').map((l) => l.trimEnd());
  const status = r.status; // null when the process died on a signal
  const how = status === null ? `killed by signal ${r.signal}` : `exit ${status}`;
  // FAILED is tested BEFORE OK (W54/F1): if a binary ever emitted both lines
  // (e.g. a per-section OK ahead of an overall FAILED), the gate must fail.
  if (lines.some((l) => l.startsWith('FILM TELEMETRY: CHECK FAILED'))) {
    if (status === 0 || status === 1) return false;
    fail(`film telemetry printed a CHECK FAILED verdict but the process ended with ${how} — line and exit code disagree, so the gate fails closed. A verdict run ends 0 (old binary) or 1 (new binary); anything else means the binary is broken or mismatched.`);
  }
  if (lines.some((l) => l === 'FILM TELEMETRY: CHECK OK')) {
    if (status === 0) return true;
    fail(`film telemetry printed CHECK OK but the process ended with ${how} — line and exit code disagree, so the gate fails closed. An OK run exits 0 under both binary generations; a non-zero exit next to an OK line is never trusted.`);
  }
  // No verdict line: the era-pairing surface (native_modes/era_hint.mjs,
  // CR-B1-10) classifies stale-vs-data causes from positive evidence and
  // prints the TALERENDER_BIN provisioning remedy inline. Still fail(),
  // still exit 1, still fail-closed — only the text grew.
  fail(eraHintMessage({ status, signal: r.signal, stdout: r.stdout, stderr: r.stderr, bin, via }));
}
// Integrity check for a cached/rendered frame (W54/F3): the cache is never
// trusted by existence alone. A frame passes only if it is a regular file with
// a PNG signature, an IHDR first chunk carrying the manifest's dimensions, and
// a closing IEND chunk — the last test is what catches a file truncated by a
// hard kill mid-write (its header survives; its tail does not). Returns null
// when the frame is usable, else a short reason.
const PNG_SIG = [137, 80, 78, 71, 13, 10, 26, 10];
function pngCheck(p, W, H) {
  let st;
  try { st = fs.statSync(p); } catch { return 'missing'; }
  if (!st.isFile()) return 'not a regular file';
  if (st.size < 45) return `only ${st.size} bytes — truncated`;
  let fd;
  try {
    fd = fs.openSync(p, 'r');
    const head = Buffer.alloc(24);
    fs.readSync(fd, head, 0, 24, 0);
    for (let i = 0; i < 8; i++) if (head[i] !== PNG_SIG[i]) return 'bad PNG signature';
    if (head.toString('ascii', 12, 16) !== 'IHDR') return 'first chunk is not IHDR';
    const w = head.readUInt32BE(16), h = head.readUInt32BE(20);
    if (w !== W || h !== H) return `dimensions ${w}x${h}, expected ${W}x${H}`;
    const tail = Buffer.alloc(12);
    fs.readSync(fd, tail, 0, 12, st.size - 12);
    if (tail.readUInt32BE(0) !== 0 || tail.toString('ascii', 4, 8) !== 'IEND') return 'no closing IEND chunk — truncated';
    return null;
  } catch (e) { return `unreadable: ${e.message}`; }
  finally { if (fd !== undefined) { try { fs.closeSync(fd); } catch {} } }
}
// Render frame ranges [a, b) with the binary; resume is the driver's own
// (frames already in FRAMES_DIR are skipped, and frame(t) is pure, so
// chunking never changes a frame). Before rendering, every cached frame in
// the ranges is integrity-checked (pngCheck): an unusable one is deleted and
// logged, so the driver re-renders it instead of skipping past it (W54/F3).
// After rendering, the ranges are re-verified — a frame that is missing or
// invalid after a "successful" render fails the capture here, with the
// remedy named. A non-zero exit fails the capture — the partial frames dir
// is left for resume, never presented as done.
async function renderRanges(bin, filmDir, manifest, ranges) {
  fs.mkdirSync(FRAMES_DIR, { recursive: true });
  for (const [a, b] of ranges) {
    for (let n = a; n < b; n++) {
      const p = framePath(n);
      if (!fs.existsSync(p)) continue;
      const bad = pngCheck(p, manifest.W, manifest.H);
      if (bad) {
        console.log(`capture-native: cached frame ${n} is unusable (${bad}) — deleted; it will be re-rendered`);
        fs.rmSync(p, { force: true, recursive: true });
      }
    }
  }
  for (const [a, b] of ranges) {
    if (b <= a) continue;
    const code = await new Promise((res, rej) => {
      const p = spawn(bin, ['--film', filmDir, '--out', FRAMES_DIR, '--from', String(a), '--to', String(b), '--width', String(manifest.W), '--height', String(manifest.H)], { stdio: 'inherit' });
      p.on('error', rej);
      p.on('close', res);
    }).catch((e) => fail(`talerender failed to start: ${e.message}`));
    if (code !== 0) fail(`talerender exited ${code} rendering frames ${a}..${b} — ${FRAMES_DIR} holds a partial render (re-run to resume; existing frames are skipped)`);
  }
  for (const [a, b] of ranges) {
    for (let n = a; n < b; n++) {
      const bad = pngCheck(framePath(n), manifest.W, manifest.H);
      if (bad) fail(`frame ${n} is ${bad === 'missing' ? 'missing' : `invalid (${bad})`} after a successful render — ${FRAMES_DIR} is inconsistent; delete it and re-run`);
    }
  }
}
// Merge sorted frame indices into render ranges, bridging gaps of <= maxGap
// frames (rendering a few extra cached frames beats a process spawn each).
function rangesFor(indices, maxGap) {
  const sorted = [...new Set(indices)].sort((x, y) => x - y);
  const ranges = [];
  for (const n of sorted) {
    const last = ranges[ranges.length - 1];
    if (last && n - last[1] <= maxGap) last[1] = n + 1;
    else ranges.push([n, n + 1]);
  }
  return ranges;
}
const framePath = (n) => path.join(FRAMES_DIR, `frame_${String(n).padStart(5, '0')}.png`);

// ---------------------------------------------------------------------
// Modes.
// ---------------------------------------------------------------------
async function checkMode(ctx) {
  // Parity check (native_modes/check_mode.mjs, CR-B1-04): the full
  // Chrome-shaped report — shot list, captions, CAST, audit lines,
  // the script.json-corrected caption audit, CAP_CHECK verdict
  // semantics, exit 0/1/3 — from `--film-telemetry --tables`.
  // runCheck sets process.exitCode itself.
  runCheck({ bin: ctx.bin, via: ctx.via, filmDir: ctx.filmDir, manifest: ctx.manifest });
}
async function gateOrStop(ctx) {
  const ok = runTelemetry(ctx.bin, ctx.filmDir, ctx.via);
  if (!ok) {
    console.log('CHECK FAILED — capture stopped at the film telemetry gate: no frames rendered, nothing finished. There is no flag to render past a failed gate.');
    process.exitCode = 1;
  }
  return ok;
}
async function framesMode(ctx) {
  const dir = path.resolve(ROOT, rest[0] || 'review/frames');
  const times = (rest[1] || '0').split(',').map(Number);
  const picks = [];
  for (const t of times) {
    if (!Number.isFinite(t) || t < 0 || t >= ctx.manifest.durationS) fail(`frames: time "${t}" is outside the film (0..${ctx.manifest.durationS}s)`);
    let n = Math.round(t * FPS);
    if (n >= ctx.manifest.totalFrames) n = ctx.manifest.totalFrames - 1;
    picks.push({ t, n });
  }
  if (!(await gateOrStop(ctx))) return;
  await renderRanges(ctx.bin, ctx.filmDir, ctx.manifest, rangesFor(picks.map((p) => p.n), 8));
  // Curated completeness check of the mode's own outputs (W54/F4 — the check
  // videoMode has always had, which framesMode used to lack): every picked
  // frame must exist and be a valid PNG of the manifest's dimensions before
  // anything is copied, and every staged copy is verified afterwards. A miss
  // names the frame and the remedy instead of surfacing as a raw ENOENT.
  for (const { n } of picks) {
    const bad = pngCheck(framePath(n), ctx.manifest.W, ctx.manifest.H);
    if (bad) fail(`frames: frame ${n} is ${bad === 'missing' ? 'missing' : `invalid (${bad})`} after a successful render — delete ${FRAMES_DIR} and re-run`);
  }
  fs.mkdirSync(dir, { recursive: true });
  let copied = 0;
  for (const { t, n } of picks) {
    fs.copyFileSync(framePath(n), path.join(dir, `t${t.toFixed(2)}.png`));
    copied++;
    if (Math.abs(n / FPS - t) > 1e-9) console.log(`  t=${t}s → frame ${n} (native frames live on the 1/${FPS}s grid; staged at ${(n / FPS).toFixed(4)}s)`);
  }
  if (copied !== picks.length) fail(`frames: staged ${copied} of ${picks.length} requested frames in ${dir} — re-run`);
  for (const { t } of picks) {
    const dest = path.join(dir, `t${t.toFixed(2)}.png`);
    if (!fs.existsSync(dest)) fail(`frames: staged copy missing after copy: ${dest} — re-run`);
  }
  console.log(dir);
}
// Sheet: delegated to native_modes/sheet_mode.mjs (CR-B1-04) —
// capture.mjs's exact tile filter and --guides variant, the fixed
// 250x86 label box with a sans-first font search, and the telemetry
// audit printed after tiling (report-only; an unavailable audit
// exits 1 — a sheet never certifies an audit it did not run).
async function sheetMode(ctx) {
  await runSheet({
    out: rest[0] || 'review/all.jpg',
    step: +(rest[1] || 0.5),
    cols: +(rest[2] || 9),
    guides: flags.has('--guides'),
    bin: ctx.bin, filmDir: ctx.filmDir, manifest: ctx.manifest,
  });
}
async function videoMode(ctx) {
  const out = path.resolve(ROOT, rest[0] || 'renders/video.mp4');
  const draft = flags.has('--draft');
  const dur = ctx.manifest.durationS;
  const a0 = rest[1] ? +rest[1] : 0, a1 = rest[2] ? Math.min(+rest[2], dur) : dur;
  if (!Number.isFinite(a0) || !Number.isFinite(a1) || a0 < 0 || !(a1 > a0)) fail(`video: bad range [${rest[1]}, ${rest[2]}] — need 0 <= start < end <= ${dur}`);
  const n0 = Math.round(a0 * FPS), n = Math.round((a1 - a0) * FPS);
  if (n <= 0) fail('video: range rounds to zero frames');
  if (!(await gateOrStop(ctx))) return;
  await renderRanges(ctx.bin, ctx.filmDir, ctx.manifest, [[n0, n0 + n]]);
  for (let i = n0; i < n0 + n; i++) {
    if (!fs.existsSync(framePath(i))) fail(`frame ${i} is missing after a successful render — ${FRAMES_DIR} is inconsistent; delete it and re-run`);
    const bad = pngCheck(framePath(i), ctx.manifest.W, ctx.manifest.H);
    if (bad) fail(`frame ${i} is invalid (${bad}) after a successful render — ${FRAMES_DIR} is inconsistent; delete it and re-run`);
  }
  // Finishing: the same step capture.mjs's video mode runs — the same encoder
  // argument sets (master/draft), the same data/mix.wav handling, the same
  // .part.mp4 + rename discipline. The one difference is the frame SOURCE:
  // an image2 sequence of clean PNGs from disk instead of JPEGs piped from a
  // page (that difference is the point of the backend — decision (B): native
  // masters carry no q0.93 JPEG generation).
  const mix = path.join(ROOT, 'data/mix.wav'), hasMix = fs.existsSync(mix);
  fs.mkdirSync(path.dirname(out), { recursive: true });
  const part = `${out}.part.mp4`;
  const inArgs = ['-f', 'image2', '-framerate', String(FPS), '-start_number', String(n0), '-i', path.join(FRAMES_DIR, 'frame_%05d.png')];
  const encArgs = draft ? ['-c:v', 'libx264', '-preset', 'veryfast', '-crf', '23', '-pix_fmt', 'yuv420p', '-c:a', 'aac', '-b:a', '192k', '-ar', '48000']
    : ['-c:v', 'libx264', '-preset', 'slow', '-crf', '18', '-threads', '4', '-pix_fmt', 'yuv420p', '-color_primaries', 'bt709', '-color_trc', 'bt709', '-colorspace', 'bt709', '-color_range', 'tv', '-x264-params', 'colorprim=bt709:transfer=bt709:colormatrix=bt709', '-c:a', 'aac', '-b:a', '256k', '-ar', '48000'];
  const t0 = Date.now();
  const ffCode = await new Promise((res, rej) => {
    const ff = spawn('ffmpeg', ['-v', 'error', '-y', ...inArgs,
      ...(hasMix ? ['-ss', String(a0), '-t', String(a1 - a0), '-i', mix] : ['-f', 'lavfi', '-i', 'anullsrc=r=48000:cl=stereo']), '-map', '0:v', '-map', '1:a',
      ...encArgs, '-shortest', '-movflags', '+faststart', part], { stdio: 'inherit' });
    ff.on('error', rej);
    ff.on('close', res);
  }).catch((e) => fail('ffmpeg failed to start: ' + e.message));
  if (ffCode !== 0) { try { fs.rmSync(part, { force: true }); } catch {} fail(`ffmpeg failed with exit code ${ffCode} — ${out} is missing or incomplete, do NOT use it`); }
  fs.renameSync(part, out);
  console.log(`\r${out} (${n} frames, ${hasMix ? 'with data/mix.wav' : 'SILENT: no data/mix.wav'}, ${((Date.now() - t0) / 1000).toFixed(0)} s)`);
  console.log('renderer: native (TaleRender) — clean PNG frames, no JPEG generation (contract decision (B))');
  // P3 provenance sidecar (native_modes/render_meta.mjs, CR-B1-10):
  // written only after the rename succeeded — a failed encode
  // leaves no sidecar claiming a finished render.
  const { sidecarPath } = writeRenderMeta({
    backend: 'native',
    binary: { path: ctx.bin, via: ctx.via, version: ctx.binaryVersion },
    manifest: { path: ctx.manifestPath },
    mode: 'video',
    frameRange: { from: n0, count: n, fps: FPS },
    encode: { draft, preset: draft ? 'veryfast' : 'slow', crf: draft ? 23 : 18, audioBitrate: draft ? '192k' : '256k' },
    gate: { verdict: 'CHECK OK' }, // videoMode only reaches here past gateOrStop()
    resolution: { width: ctx.manifest.W, height: ctx.manifest.H },
    durationS: ctx.manifest.durationS, hasMix,
  }, out);
  console.log(`render metadata: ${sidecarPath}`);
}
// ---------------------------------------------------------------------
async function main() {
  // Changed modes (native_modes/mode_dispatch.mjs, CR-B1-10):
  // castsheet is native, cues routes to tools/cues_native.py,
  // preview is retired. None needs the film manifest; castsheet
  // gets the discovered binary when discovery succeeds (its own
  // failure names the missing prerequisite if there is none).
  if (isChangedMode(mode)) {
    let bin;
    try { bin = findBinary().bin; } catch { bin = undefined; }
    process.exitCode = await dispatchChangedMode({ mode, rest, projectRoot: ROOT, talerenderBin: bin });
    return;
  }
  if (!['check', 'frames', 'sheet', 'video'].includes(mode)) {
    console.log('modes (native): check · frames · sheet · video · castsheet — cues routes to tools/cues_native.py (exit 1) · preview retired with Chrome');
    process.exit(2);
  }
  const { bin, via } = findBinary();
  // P3: the binary's --version, captured once for the render
  // sidecar; a binary that cannot answer records null, never a guess.
  let binaryVersion = null;
  try {
    const v = spawnSync(bin, ['--version'], { encoding: 'utf8' });
    if (!v.error && v.status === 0) binaryVersion = String(v.stdout || '').trim() || null;
  } catch { /* tolerated: null in the sidecar */ }
  // Pre-validation accumulates across stages (W54/F10): a manifest fault no
  // longer masks film-data faults behind it. Each stage that can run, runs,
  // and every problem found is reported together; a stage whose inputs are
  // gone (no manifest file, no film dir) contributes its own error and is
  // skipped. Binary discovery stays first-failure — nothing can run without it.
  const problems = [];
  let manifestPath = null, manifest = null, filmDir = null;
  try { manifestPath = findManifest(); } catch (e) { problems.push(e.message); }
  if (manifestPath) { try { manifest = loadManifest(manifestPath); } catch (e) { problems.push(e.message); } }
  try { filmDir = findFilmDir(); } catch (e) { problems.push(e.message); }
  if (filmDir) { try { validateFilmData(filmDir, manifest); } catch (e) { problems.push(e.message); } }
  if (problems.length) fail(problems.join('\n'));
  const ctx = { bin, via, manifest, manifestPath, filmDir, binaryVersion };
  console.log(`capture-native: backend=native (TaleRender) · binary ${bin} (via ${via}) · manifest ${manifestPath}`);
  console.log(`film: ${manifest.title || '(untitled)'} — duration ${manifest.durationS.toFixed(2)} s · ${manifest.totalFrames} frames @ ${manifest.fps} fps · ${manifest.W}x${manifest.H} · film data ${filmDir}`);
  if (mode === 'check') await checkMode(ctx);
  else if (mode === 'frames') await framesMode(ctx);
  else if (mode === 'sheet') await sheetMode(ctx);
  else if (mode === 'video') await videoMode(ctx);
}
main().catch((e) => { console.error(`capture-native: ${e.message}`); process.exit(1); });
