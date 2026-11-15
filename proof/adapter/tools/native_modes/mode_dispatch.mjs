// mode_dispatch.mjs — proposed dispatch layer for the modes the Chrome-
// replacement wave changes. CHROME-REPLACEMENT WAVE, Batch 1, Seat 10
// (CR-B1-10). Contracts: CHROME-REPLACEMENT-CONTRACTS.md §1 rows 5/6/7.
//
// TODAY (verified in capture_native.mjs): `cues`, `castsheet` and `preview`
// sit in a REFUSALS table; main() fail()s with exit 1 and a "Run: node
// tools/capture.mjs ..." fallback line for each. This module replaces that
// table for those three modes — and nothing else. `check` / `frames` /
// `sheet` / `video` keep the adapter's existing implementations untouched.
//
//   castsheet  REPLACE-NATIVE (row 6): delegates to the sibling module
//              native_modes/castsheet_mode.mjs, dynamically imported so a
//              not-yet-landed / missing module is a LOUD failure naming the
//              missing piece — never a Chrome suggestion.
//   cues       NOT-CHROME, routing (row 5): cues are authored data, emitted
//              by the skill-side tool tools/cues_native.py from the
//              project's bridge/scene data — the adapter itself does not
//              emit them. This dispatch prints the routing resolution
//              (including whether an existing data/cues.json is already
//              newer than the scene data) and exits NON-ZERO with the exact
//              command to run. Fail-closed, informative, no silent pass:
//              a caller scripting `capture_native.mjs cues && mix.py` must
//              not proceed on a cues file this run did not produce.
//   preview    NOT-CHROME, retired (row 7): capture.mjs preview only
//              started a static server and printed a URL; it was never a
//              production step. The mode retires with Chrome. This dispatch
//              says so and names the native review deliverables that
//              replace it: sheet, frames, video.
//
// Exit-code discipline (CAPTURE-NATIVE.md, unchanged): 0 = success;
// 1 = capture failed / mode refused / a named prerequisite missing;
// 2 = usage / selection error. Accordingly: castsheet success = 0, its
// failures = 1; cues routing = 1 (a refusal with a remedy, exactly the
// old REFUSALS shape); preview retired = 1 (a refusal, not an unknown
// mode — the mode is known, it is retired). Unknown modes are NOT handled
// here (returns null) so the adapter's existing exit-2 usage path stands.
//
// Message style matches the adapter: "capture-native: " prefix on errors,
// the mode named in quotes, the remedy as an exact command, and the
// standing line that the adapter never falls back silently — except that
// for these three modes there is no Chrome fallback to offer at all.
//
// Wiring (lead, see CR-B1-10-ADAPTER.md): in main(), BEFORE the REFUSALS /
// known-mode checks and BEFORE binary discovery (none of these three modes
// needs the gate; castsheet's module receives the binary path when the
// lead has already discovered it, and resolves its own prerequisites
// otherwise), call:
//
//   if (isChangedMode(mode)) {
//     process.exitCode = await dispatchChangedMode({
//       mode, rest, projectRoot: ROOT,
//       talerenderBin: <bin if already discovered, else undefined>,
//     });
//     return;
//   }
//
// Node builtins only.
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { pathToFileURL } from 'node:url';

export const CHANGED_MODES = Object.freeze(['castsheet', 'cues', 'preview']);
export function isChangedMode(mode) { return CHANGED_MODES.includes(mode); }

export const CUES_COMMAND = 'python3 tools/cues_native.py';
export const CASTSHEET_MODULE_REL = 'tools/native_modes/castsheet_mode.mjs';

const say = (log, s) => log(s);
const sayErr = (err, s) => err(s);

// --- castsheet -----------------------------------------------------------
async function castsheetDispatch({ rest, projectRoot, talerenderBin, log, err }, deps) {
  const importCastsheet = deps.importCastsheet
    || (() => import('./castsheet_mode.mjs'));
  let mod;
  try {
    mod = await importCastsheet();
  } catch (e) {
    if (e && (e.code === 'ERR_MODULE_NOT_FOUND' || /castsheet_mode/.test(String(e.message)))) {
      sayErr(err, `capture-native: mode "castsheet" is a native mode in the Chrome-free skill, but its implementation module is missing: ${CASTSHEET_MODULE_REL} (expected beside mode_dispatch.mjs in template/tools/native_modes/). The castsheet seat's module has not landed in this skill copy — re-sync the skill or restore that file, then re-run. No other backend provides this mode in the Chrome-free pipeline.`);
      return 1;
    }
    sayErr(err, `capture-native: mode "castsheet" failed to load its implementation module (${CASTSHEET_MODULE_REL}): ${e.message}`);
    return 1;
  }
  if (!mod || typeof mod.castsheetMode !== 'function') {
    sayErr(err, `capture-native: mode "castsheet": ${CASTSHEET_MODULE_REL} loaded but does not export castsheetMode({ projectRoot, outdir, kind, talerenderBin, log }) — the module in this skill copy does not match the wave contract. Restore the contracted module, then re-run.`);
    return 1;
  }
  const outdirRaw = rest[0] || 'review/cast';
  const outdir = path.isAbsolute(outdirRaw) ? outdirRaw : path.resolve(projectRoot, outdirRaw);
  const kind = rest[1] || undefined;
  try {
    await mod.castsheetMode({ projectRoot, outdir, kind, talerenderBin, log });
  } catch (e) {
    sayErr(err, `capture-native: mode "castsheet" failed: ${e.message}`);
    return 1;
  }
  return 0;
}

// --- cues ----------------------------------------------------------------
// Scene-data candidates whose mtimes decide whether an existing
// data/cues.json can be called current. Bridge data first (the emitter's
// actual inputs), then the authored source it is transcribed from.
export function sceneDataCandidates(projectRoot) {
  const c = [
    path.join(projectRoot, 'native', 'film.json'),
    path.join(projectRoot, 'native', 'scenes', 'film', 'film.json'),
    path.join(projectRoot, 'native', 'film', 'onoda.json'),
    path.join(projectRoot, 'src', 'shots.ts'),
  ];
  const scenesDir = path.join(projectRoot, 'native', 'scenes', 'film');
  try {
    for (const f of fs.readdirSync(scenesDir)) if (f.endsWith('.scene.json')) c.push(path.join(scenesDir, f));
  } catch { /* no scenes dir — the fixed candidates above are the whole list */ }
  return c;
}

// { exists, cuesMtime, newestScene: {path, mtime} | null, fresh: boolean|null }
// fresh is null when it cannot be judged (no cues file, or no scene data).
export function cuesFreshness(projectRoot) {
  const cuesPath = path.join(projectRoot, 'data', 'cues.json');
  let cuesMtime = null;
  try { cuesMtime = fs.statSync(cuesPath).mtimeMs; } catch { /* absent */ }
  let newestScene = null;
  for (const p of sceneDataCandidates(projectRoot)) {
    try {
      const m = fs.statSync(p).mtimeMs;
      if (!newestScene || m > newestScene.mtime) newestScene = { path: p, mtime: m };
    } catch { /* not present in this project */ }
  }
  return {
    cuesPath,
    exists: cuesMtime !== null,
    cuesMtime,
    newestScene,
    fresh: cuesMtime === null || !newestScene ? null : cuesMtime >= newestScene.mtime,
  };
}

function cuesDispatch({ projectRoot, log, err }) {
  const f = cuesFreshness(projectRoot);
  const lines = [
    'capture-native: mode "cues" is not an adapter render mode — cues are authored data, and in the Chrome-free pipeline they are emitted from the project\u2019s bridge/scene data by the skill-side emitter, not extracted from a page.',
  ];
  if (f.exists && f.fresh === true) {
    lines.push(`cues: data/cues.json already exists and is newer than the scene data (newest scene input: ${path.relative(projectRoot, f.newestScene.path)}, ${new Date(f.newestScene.mtime).toISOString()}; cues.json: ${new Date(f.cuesMtime).toISOString()}) — as far as timestamps can tell it is current, and tools/mix.py can consume it as-is. This dispatch still exits non-zero: the adapter did not produce that file in this run, and it never passes a mode it did not run.`);
  } else if (f.exists && f.fresh === false) {
    lines.push(`cues: data/cues.json exists but is OLDER than the scene data (newest scene input: ${path.relative(projectRoot, f.newestScene.path)}, ${new Date(f.newestScene.mtime).toISOString()}; cues.json: ${new Date(f.cuesMtime).toISOString()}) — treat it as stale and regenerate it before mixing.`);
  } else if (f.exists) {
    lines.push('cues: data/cues.json exists, but no scene data was found to compare it against — its currency cannot be verified from timestamps; regenerate it to be sure.');
  } else {
    lines.push('cues: data/cues.json does not exist in this project yet.');
  }
  lines.push(`Run (from the project folder):  ${CUES_COMMAND}`);
  lines.push('That writes data/cues.json in the exact shape tools/mix.py and tools/audio_recipes.py build_cues consume. An unbridged project fails there, naming the bridge step — this adapter does not improvise cues and offers no other route.');
  for (const l of lines) say(err, l);
  return 1;
}

// --- preview -------------------------------------------------------------
function previewDispatch({ log, err }) {
  const lines = [
    'capture-native: mode "preview" retired with Chrome — it only ever served the project page for a browser to open; it was never a production step, and the native backend has no page to serve.',
    'The native review deliverables that replace it:',
    '  contact sheet:  RENDERER=native node tools/capture_native.mjs sheet review/all.jpg',
    '  still frames:   RENDERER=native node tools/capture_native.mjs frames review/frames "1.2,8.4"',
    '  finished film:  RENDERER=native node tools/capture_native.mjs video renders/out.mp4',
  ];
  for (const l of lines) say(err, l);
  return 1;
}

// --- dispatch ------------------------------------------------------------
// Returns an exit code (0/1) for a changed mode, or null when `mode` is
// not one of the three — the adapter's existing dispatch then applies.
// `deps.importCastsheet` exists for tests; production uses the dynamic
// import of the sibling module.
export async function dispatchChangedMode(opts = {}, deps = {}) {
  const mode = opts.mode;
  if (!isChangedMode(mode)) return null;
  const projectRoot = path.resolve(opts.projectRoot || opts.root || process.cwd());
  const rest = Array.isArray(opts.rest) ? opts.rest : [];
  const log = typeof opts.log === 'function' ? opts.log : console.log;
  const err = typeof opts.err === 'function' ? opts.err : console.error;
  const talerenderBin = opts.talerenderBin || opts.bin || undefined;
  if (mode === 'castsheet') return castsheetDispatch({ rest, projectRoot, talerenderBin, log, err }, deps);
  if (mode === 'cues') return cuesDispatch({ projectRoot, log, err });
  return previewDispatch({ log, err });
}

export default dispatchChangedMode;

// ---------------------------------------------------------------------
// Self-test: node tools/native_modes/mode_dispatch.mjs --self-test
// Temp project fixtures + injected castsheet importers (the real sibling
// module is another seat's delivery and may or may not have landed).
// Exits 0 on PASS, 1 on FAIL.
// ---------------------------------------------------------------------
export async function runSelfTest(log = console.log) {
  let failures = 0;
  const check = (name, cond, detail = '') => {
    log(`mode-dispatch self-test: ${cond ? 'PASS' : 'FAIL'} ${name}${cond ? '' : ` — ${detail}`}`);
    if (!cond) failures++;
  };
  const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'mode-dispatch-test-'));
  try {
    const capture = () => {
      const out = [];
      return { lines: out, log: (s) => out.push(String(s)), err: (s) => out.push(String(s)), text: () => out.join('\n') };
    };

    // preview: retired, exit 1, deliverables named.
    {
      const c = capture();
      const code = await dispatchChangedMode({ mode: 'preview', projectRoot: tmp, log: c.log, err: c.err });
      check('preview exits 1 (refusal discipline, not usage 2)', code === 1, `got ${code}`);
      check('preview says it retired with Chrome', /retired with Chrome/.test(c.text()));
      check('preview names sheet / frames / video deliverables', /sheet review\/all\.jpg/.test(c.text()) && /frames review\/frames/.test(c.text()) && /video renders\/out\.mp4/.test(c.text()));
    }

    // cues, missing file: exit 1, exact command, no silent pass.
    {
      const c = capture();
      const code = await dispatchChangedMode({ mode: 'cues', projectRoot: tmp, log: c.log, err: c.err });
      check('cues (no cues.json) exits non-zero', code === 1, `got ${code}`);
      check('cues prints the exact emitter command', c.text().includes(CUES_COMMAND));
      check('cues says the file does not exist yet', /does not exist in this project yet/.test(c.text()));
    }

    // cues, fresh file: still exit 1, but says it is newer than scene data.
    {
      const proj = path.join(tmp, 'fresh'); fs.mkdirSync(path.join(proj, 'data'), { recursive: true }); fs.mkdirSync(path.join(proj, 'src'), { recursive: true });
      const scene = path.join(proj, 'src', 'shots.ts'); fs.writeFileSync(scene, '// fixture');
      const past = new Date(Date.now() - 60_000); fs.utimesSync(scene, past, past);
      fs.writeFileSync(path.join(proj, 'data', 'cues.json'), '{"duration":1,"cues":[]}');
      const c = capture();
      const code = await dispatchChangedMode({ mode: 'cues', projectRoot: proj, log: c.log, err: c.err });
      check('cues (fresh cues.json) still exits non-zero — no silent pass', code === 1, `got ${code}`);
      check('cues (fresh) says cues.json is newer than the scene data', /already exists and is newer than the scene data/.test(c.text()));
      check('cues (fresh) still prints the exact command', c.text().includes(CUES_COMMAND));
    }

    // cues, stale file: exit 1 + stale named.
    {
      const proj = path.join(tmp, 'stale'); fs.mkdirSync(path.join(proj, 'data'), { recursive: true }); fs.mkdirSync(path.join(proj, 'src'), { recursive: true });
      const cues = path.join(proj, 'data', 'cues.json'); fs.writeFileSync(cues, '{}');
      const past = new Date(Date.now() - 60_000); fs.utimesSync(cues, past, past);
      fs.writeFileSync(path.join(proj, 'src', 'shots.ts'), '// newer fixture');
      const c = capture();
      const code = await dispatchChangedMode({ mode: 'cues', projectRoot: proj, log: c.log, err: c.err });
      check('cues (stale) exits 1 and names the staleness', code === 1 && /OLDER than the scene data/.test(c.text()));
    }

    // castsheet, missing module: loud, names the piece, NO Chrome suggestion.
    {
      const c = capture();
      const missing = Object.assign(new Error(`Cannot find module './castsheet_mode.mjs'`), { code: 'ERR_MODULE_NOT_FOUND' });
      const code = await dispatchChangedMode({ mode: 'castsheet', rest: ['review/cast'], projectRoot: tmp, log: c.log, err: c.err }, { importCastsheet: async () => { throw missing; } });
      check('castsheet (module missing) exits 1', code === 1, `got ${code}`);
      check('castsheet (module missing) names the missing module', c.text().includes(CASTSHEET_MODULE_REL));
      check('castsheet (module missing) offers NO Chrome fallback', !/capture\.mjs/.test(c.text()) && !/Chrome-only/.test(c.text()), c.text());
    }

    // castsheet, contracted module: delegates with the contracted argument shape.
    {
      const c = capture();
      let seen = null;
      const code = await dispatchChangedMode(
        { mode: 'castsheet', rest: ['review/cast', 'pose'], projectRoot: tmp, talerenderBin: '/fixtures/talerender', log: c.log, err: c.err },
        { importCastsheet: async () => ({ castsheetMode: async (a) => { seen = a; } }) },
      );
      check('castsheet (module present) exits 0', code === 0, `got ${code}`);
      check('castsheet delegates {projectRoot, outdir, kind, talerenderBin, log}',
        !!seen && seen.projectRoot === tmp && seen.outdir === path.join(tmp, 'review/cast') && seen.kind === 'pose' && seen.talerenderBin === '/fixtures/talerender' && typeof seen.log === 'function',
        JSON.stringify(seen && { ...seen, log: typeof seen.log }));
    }

    // castsheet, module without the export / module that throws: exit 1.
    {
      const c1 = capture();
      const code1 = await dispatchChangedMode({ mode: 'castsheet', projectRoot: tmp, log: c1.log, err: c1.err }, { importCastsheet: async () => ({}) });
      const c2 = capture();
      const code2 = await dispatchChangedMode({ mode: 'castsheet', projectRoot: tmp, log: c2.log, err: c2.err }, { importCastsheet: async () => ({ castsheetMode: async () => { throw new Error('fixture boom'); } }) });
      check('castsheet (bad export) exits 1 and names the contract', code1 === 1 && /does not export castsheetMode/.test(c1.text()));
      check('castsheet (module throws) exits 1 with its message', code2 === 1 && /fixture boom/.test(c2.text()));
    }

    // Not a changed mode: null, adapter's own dispatch stands.
    check('unchanged modes return null (check/frames/sheet/video/unknown)', (await dispatchChangedMode({ mode: 'check', projectRoot: tmp })) === null
      && (await dispatchChangedMode({ mode: 'video', projectRoot: tmp })) === null
      && (await dispatchChangedMode({ mode: 'bogus', projectRoot: tmp })) === null);
    check('isChangedMode covers exactly the three wave modes', CHANGED_MODES.join(',') === 'castsheet,cues,preview');
  } finally {
    fs.rmSync(tmp, { recursive: true, force: true });
  }
  log(`mode-dispatch self-test: ${failures === 0 ? 'PASS' : `FAIL (${failures} failing)`}`);
  return failures === 0 ? 0 : 1;
}

const isMain = process.argv[1] && import.meta.url === pathToFileURL(path.resolve(process.argv[1])).href;
if (isMain) {
  if (process.argv.includes('--self-test')) process.exitCode = await runSelfTest();
  else { console.error('mode-dispatch: this module is imported by capture_native.mjs; run it directly only as: node tools/native_modes/mode_dispatch.mjs --self-test'); process.exitCode = 2; }
}
