// castsheet_mode.mjs — the NATIVE castsheet mode (Chrome replacement,
// contract row 6). CHROME-REPLACEMENT WAVE, Batch 1, Seat 3 (CR-B1-03).
//
// The Chrome mode (tools/capture.mjs `castsheet`) stages one frame per
// pose / mood / look / held prop on the template castsheet.html page and
// writes <kind>-<id>.jpg per item plus a labelled castsheet.jpg contact
// sheet — the same <kind>-<id>.jpg names library/previews/ uses ("the
// name castsheet writes", scripts/library_check.py). This module is the
// native equivalent, with NO browser in the loop:
//
//   1. tools/castsheet_native.py (stdlib-only, beside capture_native.mjs)
//      generates one scene-format v2 file per item + a cast manifest
//      into <outdir>/.native-castsheet/ (the auditable staging data).
//   2. Each scene is rendered with the talerender binary's --scene-file
//      mode to a clean PNG (the native cleanliness contract — PNG is
//      the render, JPEG is only the delivery tiling format).
//   3. Each PNG is converted with ffmpeg to <outdir>/<kind>-<id>.jpg
//      (-q:v 2, mirroring the Chrome per-item q0.92), a labelled copy
//      (black bar + "<kind> — <id>", the Chrome label() semantics) is
//      made at tile grade, and the labelled tiles are assembled into
//      <outdir>/castsheet.jpg with the Chrome mode's exact tile filter
//      (scale=270:480, tile=4xROWS, padding=4, black, -q:v 3).
//
// Signature (the wave contract, consumed by native_modes/mode_dispatch.mjs):
//
//   castsheetMode({ projectRoot, outdir, kind, talerenderBin, log })
//
//   projectRoot   the project folder (fonts/ is read here for labels)
//   outdir        output dir, resolved against projectRoot when relative
//                 (default 'review/cast', the Chrome mode's default)
//   kind          '' = the whole catalog; 'pose' | 'mood' | 'look' |
//                 'held' filter it; 'cast' renders the PROJECT's cast
//                 (generator --project mode) instead of the catalog
//   talerenderBin explicit binary path; discovery otherwise mirrors
//                 capture_native.mjs (TALERENDER_BIN -> <project>/native
//                 -> repo checkout -> PATH) and fails closed on a bad
//                 explicit path
//   log           line logger (default console.log)
//
// Held props: the native painter takes held props ONLY as pre-rendered
// plates (scene-format §14.3). A held item with no plate PNG in the
// work dir's plates/ folder is SKIPPED and named in the summary — never
// silently dropped, never faked. If a 'held' run renders nothing at
// all, the mode fails (exit via throw) naming the missing plates step.
// Failures throw; the dispatch turns a throw into exit 1. Node builtins
// only. Spec + proof: docs/integration/CR-B1-03-CASTSHEET.md (TaleRender
// repo).
import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const TOOLS_DIR = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const GENERATOR = path.join(TOOLS_DIR, 'castsheet_native.py');
const W = 1080, H = 1920;

function isExec(p) {
  try { return fs.statSync(p).isFile() && (fs.accessSync(p, fs.constants.X_OK), true); } catch { return false; }
}

function findBinary(explicit, projectRoot) {
  if (explicit) {
    if (isExec(explicit)) return { bin: explicit, via: 'argument' };
    throw new Error(`castsheet: talerenderBin="${explicit}" is not an executable file — an explicit binary is never second-guessed.`);
  }
  const env = process.env.TALERENDER_BIN;
  if (env) {
    if (isExec(env)) return { bin: env, via: 'TALERENDER_BIN' };
    throw new Error(`castsheet: TALERENDER_BIN="${env}" is not an executable file — fix the path or unset it to use discovery.`);
  }
  const candidates = [
    [path.join(projectRoot, 'native', 'talerender'), 'project native dir'],
    [path.join(os.homedir(), 'workspace', 'talerender', 'talerender'), 'TaleRender repo checkout'],
  ];
  for (const [p, via] of candidates) if (isExec(p)) return { bin: p, via };
  for (const d of String(process.env.PATH || '').split(path.delimiter)) {
    if (!d) continue;
    const p = path.join(d, 'talerender');
    if (isExec(p)) return { bin: p, via: 'PATH' };
  }
  throw new Error('castsheet: talerender binary not found. Set TALERENDER_BIN=/path/to/talerender (the "talerender-binary" artifact from a green CI run of turjaun985123/talerender).');
}

function run(cmd, argv, what) {
  const r = spawnSync(cmd, argv, { encoding: 'utf8', maxBuffer: 64 * 1024 * 1024 });
  if (r.error) throw new Error(`castsheet: ${what} failed to start: ${r.error.message}`);
  if (r.status !== 0) throw new Error(`castsheet: ${what} failed (exit ${r.status}): ${(r.stderr || r.stdout || 'no output').trim().split('\n').slice(-4).join('\n')}`);
  return r;
}

function findFont(projectRoot) {
  for (const dir of [path.join(projectRoot, 'fonts'), path.join(TOOLS_DIR, '..', 'fonts')]) {
    try {
      const f = fs.readdirSync(dir).filter((x) => /\.(otf|ttf)$/i.test(x)).sort()[0];
      if (f) return path.join(dir, f);
    } catch { /* try the next dir */ }
  }
  return null;
}
const drawEsc = (s) => s.replace(/\\/g, '\\\\').replace(/:/g, '\\:').replace(/'/g, "\\'");

export async function castsheetMode({ projectRoot, outdir, kind, talerenderBin, log } = {}) {
  const say = typeof log === 'function' ? log : (s) => console.log(s);
  const root = path.resolve(projectRoot || process.cwd());
  const out = path.resolve(root, outdir || 'review/cast');
  const kindFilter = String(kind || '');
  if (kindFilter && !['pose', 'mood', 'look', 'held', 'cast'].includes(kindFilter)) {
    throw new Error(`castsheet: unknown kind "${kindFilter}" — kinds are pose, mood, look, held, cast (or empty for the whole catalog)`);
  }
  if (!fs.existsSync(GENERATOR)) throw new Error(`castsheet: generator missing: ${GENERATOR} — the skill copy is incomplete; restore tools/castsheet_native.py.`);
  const { bin, via } = findBinary(talerenderBin, root);
  say(`capture-native: castsheet via ${bin} (via ${via})`);

  // 1 — staging data (generator). Work dir lives inside outdir so the
  // scenes + manifest that produced a sheet stay auditable beside it.
  const work = path.join(out, '.native-castsheet');
  fs.mkdirSync(out, { recursive: true });
  const genArgs = [GENERATOR, '--out', work];
  if (kindFilter === 'cast') genArgs.push('--project', root);
  else if (kindFilter) genArgs.push('--kind', kindFilter);
  const gen = run('python3', genArgs, 'castsheet_native.py');
  if (gen.stdout) process.stdout.write(gen.stdout);
  const manifest = JSON.parse(fs.readFileSync(path.join(work, 'cast-manifest.json'), 'utf8'));
  const items = manifest.items || [];
  if (!items.length) throw new Error(`castsheet: generator produced zero items${kindFilter ? ` of kind "${kindFilter}"` : ''}`);

  // 2+3 — render each item, convert, label.
  const font = findFont(root);
  if (!font) say('castsheet: no font file under fonts/ — contact-sheet tiles are unlabelled');
  const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'fs-cast-native-'));
  const rendered = [], skipped = [];
  try {
    let n = 0;
    for (const it of items) {
      const base = `${it.kind}-${it.id}`;
      if (it.plateMissing) { skipped.push(`${base} (no held-prop plate)`); say(`${base}: SKIPPED — no plate at ${work}/plates/${it.id}.png (native held props are plates; see CR-B1-03)`); continue; }
      const png = path.join(work, 'frames', `${base}.png`);
      fs.mkdirSync(path.dirname(png), { recursive: true });
      run(bin, ['--scene-file', path.join(work, it.scene), '--out', png, '--width', String(W), '--height', String(H)], `talerender rendering ${base}`);
      const jpgOut = path.join(out, `${base}.jpg`);
      run('ffmpeg', ['-v', 'error', '-y', '-i', png, '-frames:v', '1', '-q:v', '2', jpgOut], `ffmpeg converting ${base}`);
      // Labelled tile copy: Chrome label() paints a 92 px black bar and
      // 54 px bold text "<kind> — <id>" at (22, 62-baseline).
      const tile = path.join(tmp, `f${String(n++).padStart(4, '0')}.jpg`);
      if (font) {
        const vf = `drawbox=x=0:y=0:w=${W}:h=92:color=black:t=fill,drawtext=fontfile='${drawEsc(font)}':text='${drawEsc(`${it.kind} — ${it.id}`)}':x=22:y=14:fontsize=54:fontcolor=white`;
        const r = spawnSync('ffmpeg', ['-v', 'error', '-y', '-i', png, '-vf', vf, '-frames:v', '1', '-q:v', '5', tile], { encoding: 'utf8' });
        if (r.error || r.status !== 0) run('ffmpeg', ['-v', 'error', '-y', '-i', png, '-frames:v', '1', '-q:v', '5', tile], `ffmpeg tiling ${base}`);
      } else {
        run('ffmpeg', ['-v', 'error', '-y', '-i', png, '-frames:v', '1', '-q:v', '5', tile], `ffmpeg tiling ${base}`);
      }
      rendered.push(base);
      say(`${base}.jpg`);
    }
    if (!rendered.length) {
      throw new Error(`castsheet: nothing rendered${skipped.length ? ` — all ${skipped.length} item(s) skipped: ${skipped.join(', ')}` : ''}. For held props, supply plates (PNG per prop id) in ${path.join(work, 'plates')} and re-run.`);
    }
    // Contact sheet: the Chrome mode's exact tile filter and geometry.
    const cols = 4, rows = Math.ceil(rendered.length / cols);
    const sheet = path.join(out, 'castsheet.jpg');
    run('ffmpeg', ['-v', 'error', '-y', '-framerate', '1', '-i', path.join(tmp, 'f%04d.jpg'), '-vf', `scale=270:480,tile=${cols}x${rows}:padding=4:color=black`, '-frames:v', '1', '-q:v', '3', sheet], 'ffmpeg assembling castsheet.jpg');
    say(`${sheet} (${rendered.length} items${kindFilter ? `, kind=${kindFilter}` : ''}${skipped.length ? `, ${skipped.length} skipped` : ''})`);
    if (skipped.length) say(`castsheet: skipped (named, not dropped): ${skipped.join(', ')}`);
  } finally { fs.rmSync(tmp, { recursive: true, force: true }); }
  return { rendered, skipped, sheet: path.join(out, 'castsheet.jpg'), workdir: work };
}

export default castsheetMode;
