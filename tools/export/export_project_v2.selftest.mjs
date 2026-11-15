// Self-test for tools/export/export_project_v2.mjs (Chrome-
// Replacement Wave, Batch 1, Seat 7 — TS export hardening).
//
// Hermetic: builds fixture projects in a fresh temp dir, runs the
// exporter as a child process (node:child_process), and asserts
//   1. synthetic-project success: exit 0, interchange document
//      parses with format "talerender-project/1", meta resolved
//      from src/core/lib.ts, sections populated from the real
//      files (cast from LOOK, shots resolved on the line clock,
//      captions from lines+script, audio tracks + cues), asset
//      manifest covers data/fonts/script/shots.ts, and the
//      skeleton report carries segments + an explicit owes list;
//   2. missing-timing failure: a project without data/lines.json
//      exits 2 and writes NEITHER output;
//   3. backwards-shots failure: shot times running backwards exit
//      2 and write nothing;
//   4. bad meta override: --fps 0 exits 2 (the projectfile.rs
//      positive-integer rule) and writes nothing;
//   5. REAL-PROJECT run: a read-only COPY of the Onoda project
//      (~/faceless-shorts/the-soldier-who-didnt-know) copied into
//      the temp dir exports cleanly — title, 5 cast members, 14
//      shots, 12 captions, 9 file cues, frame tiling [0, 1811) —
//      and the ORIGINAL project tree is byte-for-byte untouched
//      (recursive listing compared before/after). The exporter
//      never writes into a project dir; this proves it.
// Prints one line per assertion and a final PASS/FAIL count line.
// Exit code: 0 iff FAIL is 0. No npm dependencies (node: builtins).

import { spawnSync } from 'node:child_process';
import * as fs from 'node:fs';
import * as os from 'node:os';
import * as path from 'node:path';
import { fileURLToPath } from 'node:url';

const EXPORTER = path.join(
  path.dirname(fileURLToPath(import.meta.url)), 'export_project_v2.mjs');
const REAL_PROJECT = path.join(
  os.homedir(), 'faceless-shorts', 'the-soldier-who-didnt-know');

let passed = 0;
let failed = 0;
function check(name, cond, detail = '') {
  if (cond) {
    passed += 1;
    console.log(`PASS ${name}`);
  } else {
    failed += 1;
    console.log(`FAIL ${name}${detail ? ` — ${detail}` : ''}`);
  }
}

function runExporter(projectDir, outPath, extraArgs = []) {
  return spawnSync(process.execPath, [EXPORTER, projectDir, outPath, ...extraArgs],
    { encoding: 'utf8' });
}

function writeFile(root, rel, content) {
  const p = path.join(root, rel);
  fs.mkdirSync(path.dirname(p), { recursive: true });
  fs.writeFileSync(p, content);
}

const FIXTURE_SHOTS_TS = `
import { registerCast } from './core/cast';
const LOOK: Record<string, ToonLook> = {
  hero: { body: '#112233', hat: 'cap', hatCol: '#445566' },
  pal: { body: '#AABBCC', extra: ['glasses'] },
};
Object.entries(LOOK).forEach(([k, v]) => registerCast(k, v));
export function buildShots(E: Engine): { shots: Shot[]; cues: Cue[] } {
  const S: Shot[] = [], CUES: Cue[] = [], add = (o: Shot) => (S.push(o), o), cue = (t: number, type: string, o: { gain?: number; dur?: number } = {}) => CUES.push({ t: +t.toFixed(3), type, ...o });
  const cut = (i: number) => (i === 0 ? 0 : L(i) - 0.06);
  // ══ SET 1 · the clearing ════════════════════════
  const JG = (() => {
    const scene = new THREE.Scene();
    WD.sky(scene, { top: '#6E9CC0', horizon: '#D8E8DA' });
    return { scene };
  })();
  const jgHero = person('hero'); JG.scene.add(jgHero.group);
  const jgPal = person('pal', { perspective: true }, '#778899'); JG.scene.add(jgPal.group);
  { const st = cut(0);
    add({ id: 'S01', at: st, note: 'hero enters the clearing', draw(lt, d, t, blit) {
      applyAtmosphere(JG.scene, 'noon', { rig: JG.sun });
      shoot(JG.scene, [jgHero], V3(0, 2, 5), V3(0, 1.6, 3), V3(0, 1, 0), null, k, { ...G.jungle, fov: 44 });
      blit();
    } });
    cue(st + 0.5, 'chirp', { gain: 0.2, dur: 1.6 }); }
  { const st = cut(1);
    add({ id: 'S02', at: st, note: 'pal arrives with the news', draw(lt, d, t, blit) {
      shoot(JG.scene, [jgHero, jgPal], V3(1, 2, 5), V3(1, 1.6, 3), V3(0, 1, 0), null, k, { ...G.jungle, fov: 40 });
      blit();
    } });
    cue(st, 'whoosh'); }
  return { shots: S, cues: CUES };
}
`;

function writeFixtureProject(dir, shotsTs = FIXTURE_SHOTS_TS) {
  writeFile(dir, 'script.json', JSON.stringify({
    title: 'Fixture Film',
    voice: 'bm_george', speed: 1.0, lead: 0.3, gap: 0.18, outro: 1.2,
    music: 'bed',
    lines: [
      { text: 'Hello world out there.', caps: 'Hello | world | out there.' },
      { text: 'The pal arrives now.', caps: 'The pal | arrives now.' },
    ],
  }, null, 2));
  writeFile(dir, 'data/lines.json', JSON.stringify({
    duration: 10.0, voice: 'bm_george', speed: 1.0,
    lines: [
      { i: 0, text: 'Hello world out there.', start: 0.3, end: 4.0 },
      { i: 1, text: 'The pal arrives now.', start: 4.5, end: 9.0 },
    ],
  }, null, 2));
  writeFile(dir, 'data/words.json', JSON.stringify([
    { w: 'Hello', line: 0, s: 0.3, e: 0.6 },
    { w: 'world', line: 0, s: 0.7, e: 1.1 },
    { w: 'The', line: 1, s: 4.5, e: 4.7 },
    { w: 'pal', line: 1, s: 4.8, e: 5.1 },
  ], null, 2));
  writeFile(dir, 'data/cues.json', JSON.stringify({
    duration: 10.0,
    cues: [{ t: 0.5, type: 'chirp', gain: 0.2, dur: 1.6 }],
  }, null, 2));
  writeFile(dir, 'data/narration.wav', Buffer.from([0x52, 0x49, 0x46, 0x46]));
  writeFile(dir, 'data/mix.wav', Buffer.from([0x52, 0x49, 0x46, 0x46]));
  writeFile(dir, 'fonts/Test.ttf', Buffer.from([0x00, 0x01, 0x00, 0x00]));
  writeFile(dir, 'src/core/lib.ts',
    'export const W = 1080, H = 1920, FPS = 30;\n');
  writeFile(dir, 'src/shots.ts', shotsTs);
}

function listTree(root) {
  const out = [];
  const walk = (abs, rel) => {
    for (const e of fs.readdirSync(abs, { withFileTypes: true })) {
      const r = rel ? `${rel}/${e.name}` : e.name;
      if (e.isDirectory()) walk(path.join(abs, e.name), r);
      else if (e.isFile()) {
        const st = fs.statSync(path.join(abs, e.name));
        out.push(`${r}:${st.size}:${st.mtimeMs}`);
      }
    }
  };
  walk(root, '');
  return out.sort();
}

const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'talerender-export-v2-test-'));
try {
  // ---- 1. synthetic-project success ------------------------------
  const proj = path.join(tmp, 'fixture');
  writeFixtureProject(proj);
  const out = path.join(tmp, 'fixture-out.json');
  const ok = runExporter(proj, out);
  check('success: exit 0', ok.status === 0,
    `status=${ok.status} stderr=${ok.stderr}`);
  check('success: summary is one line naming the title',
    typeof ok.stdout === 'string' &&
    ok.stdout.trim().split('\n').length === 1 &&
    ok.stdout.includes('Fixture Film'),
    `stdout=${JSON.stringify(ok.stdout)}`);
  let doc = null;
  try { doc = JSON.parse(fs.readFileSync(out, 'utf8')); } catch { /* null */ }
  check('success: interchange document parses', doc !== null);
  const skelPath = path.join(tmp, 'fixture-out.skeleton.json');
  let skel = null;
  try { skel = JSON.parse(fs.readFileSync(skelPath, 'utf8')); } catch { /* null */ }
  check('success: skeleton report written by default', skel !== null);
  if (doc) {
    check('success: format string', doc.format === 'talerender-project/1',
      `format=${doc.format}`);
    check('success: meta from lib.ts constants',
      doc.meta.title === 'Fixture Film' && doc.meta.fps === 30 &&
      doc.meta.width === 1080 && doc.meta.height === 1920,
      `meta=${JSON.stringify(doc.meta)}`);
    check('success: characters from LOOK registry',
      Array.isArray(doc.characters) && doc.characters.length === 2 &&
      doc.characters[0].id === 'hero' &&
      doc.characters[0].look.body === '#112233' &&
      doc.characters[1].look.extra?.[0] === 'glasses',
      `characters=${JSON.stringify(doc.characters)}`);
    check('success: scenes from set blocks',
      Array.isArray(doc.scenes) && doc.scenes.length === 1 &&
      doc.scenes[0].id === 'JG' && doc.scenes[0].kind === 'exterior',
      `scenes=${JSON.stringify(doc.scenes)}`);
    check('success: shots resolved on the line clock',
      Array.isArray(doc.shots) && doc.shots.length === 2 &&
      doc.shots[0].id === 'S01' && doc.shots[0].atS === 0 &&
      Math.abs(doc.shots[1].atS - (4.5 - 0.06)) < 1e-9 &&
      Math.abs(doc.shots[1].endS - 10.0) < 1e-9,
      `shots=${JSON.stringify(doc.shots)}`);
    check('success: captions carry caps + timing',
      Array.isArray(doc.captions) && doc.captions.length === 2 &&
      doc.captions[0].caps === 'Hello | world | out there.' &&
      doc.captions[0].start === 0.3 && doc.captions[0].words === 2,
      `captions=${JSON.stringify(doc.captions)}`);
    check('success: audio carries tracks + file cues',
      Array.isArray(doc.audio) && doc.audio.length === 3 &&
      doc.audio.filter((a) => a.type === 'track').length === 2 &&
      doc.audio.some((a) => a.type === 'cue' && a.cueType === 'chirp' && a.t === 0.5),
      `audio=${JSON.stringify(doc.audio)}`);
    const assetPaths = (doc.assets || []).map((a) => a.path);
    check('success: asset manifest covers the real files',
      assetPaths.includes('script.json') &&
      assetPaths.includes('data/lines.json') &&
      assetPaths.includes('data/words.json') &&
      assetPaths.includes('data/narration.wav') &&
      assetPaths.includes('fonts/Test.ttf') &&
      assetPaths.includes('src/shots.ts') &&
      doc.assets.every((a) => ['image', 'audio', 'data'].includes(a.kind)),
      `assets=${JSON.stringify(assetPaths)}`);
    check('success: wav assets are kind audio, json kind data',
      doc.assets.find((a) => a.path === 'data/mix.wav')?.kind === 'audio' &&
      doc.assets.find((a) => a.path === 'data/lines.json')?.kind === 'data');
  }
  if (skel) {
    check('skeleton: format + timing',
      skel.format === 'talerender-skeleton/1' &&
      skel.timing.durationS === 10.0 && skel.timing.totalFrames === 300 &&
      skel.timing.metaSource.fps === 'src/core/lib.ts',
      `timing=${JSON.stringify(skel.timing)}`);
    check('skeleton: segments tile with frames + camera',
      Array.isArray(skel.segments) && skel.segments.length === 2 &&
      skel.segments[0].startFrame === 0 &&
      skel.segments[0].endFrame === Math.ceil((4.5 - 0.06) * 30) &&
      skel.segments[1].endFrame === 300 &&
      skel.segments[0].camera?.from?.[2] === 5 &&
      skel.segments[0].fov === 44 &&
      skel.segments[0].atmospheres?.[0]?.preset === 'noon',
      `segments=${JSON.stringify(skel.segments)}`);
    check('skeleton: per-shot cue resolved',
      skel.segments[0].cues?.length === 1 &&
      skel.segments[0].cues[0].type === 'chirp' &&
      skel.segments[0].cues[0].tS === 0.5,
      `cues=${JSON.stringify(skel.segments[0]?.cues)}`);
    check('skeleton: instances carry set membership',
      Array.isArray(skel.instances) && skel.instances.length === 2 &&
      skel.instances.every((i) => i.set === 'JG') &&
      skel.instances[1].shade === '#778899',
      `instances=${JSON.stringify(skel.instances)}`);
    const oweItems = (skel.owes || []).map((o) => o.item);
    check('skeleton: owes names the manual remainder',
      oweItems.includes('draw-body-animation') &&
      oweItems.includes('scene-geometry') &&
      oweItems.includes('caption-strips') &&
      oweItems.includes('native-package-absent'),
      `owes=${JSON.stringify(oweItems)}`);
  }

  // ---- 2. missing-timing failure ---------------------------------
  const missProj = path.join(tmp, 'missing-timing');
  writeFixtureProject(missProj);
  fs.rmSync(path.join(missProj, 'data', 'lines.json'));
  const missOut = path.join(tmp, 'missing-out.json');
  const miss = runExporter(missProj, missOut);
  check('missing lines.json: exit 2', miss.status === 2,
    `status=${miss.status}`);
  check('missing lines.json: names the file',
    `${miss.stdout}\n${miss.stderr}`.includes('lines.json'),
    `stderr=${miss.stderr}`);
  check('missing lines.json: neither output written',
    !fs.existsSync(missOut) &&
    !fs.existsSync(path.join(tmp, 'missing-out.skeleton.json')));

  // ---- 3. backwards-shots failure --------------------------------
  const backProj = path.join(tmp, 'backwards');
  // deterministic backwards case: swap the two shots' cut indices,
  // so S01 starts at cut(1) and S02 at cut(0)
  writeFixtureProject(backProj, FIXTURE_SHOTS_TS
    .replace('const st = cut(0);', 'const st = cut(1);')
    .replace("cue(st + 0.5, 'chirp'", "cue(st + 0.5, 'chirp'")
    .replace('{ const st = cut(1);\n    add({ id: \'S02\'', '{ const st = cut(0);\n    add({ id: \'S02\''));
  const backOut = path.join(tmp, 'backwards-out.json');
  const back = runExporter(backProj, backOut);
  check('backwards shots: exit 2', back.status === 2,
    `status=${back.status} stderr=${back.stderr}`);
  check('backwards shots: no output written', !fs.existsSync(backOut));

  // ---- 4. bad meta override ---------------------------------------
  const metaOut = path.join(tmp, 'meta-out.json');
  const meta = runExporter(proj, metaOut, ['--fps', '0']);
  check('meta --fps 0: exit 2', meta.status === 2, `status=${meta.status}`);
  check('meta --fps 0: names meta.fps',
    `${meta.stdout}\n${meta.stderr}`.includes('meta.fps'),
    `stderr=${meta.stderr}`);
  check('meta --fps 0: no output written', !fs.existsSync(metaOut));

  // ---- 5. real-project run (read-only copy of Onoda) --------------
  check('real project: present on this machine', fs.existsSync(REAL_PROJECT),
    REAL_PROJECT);
  if (fs.existsSync(REAL_PROJECT)) {
    const before = listTree(REAL_PROJECT);
    const copyDir = path.join(tmp, 'onoda-copy');
    fs.cpSync(REAL_PROJECT, copyDir, {
      recursive: true,
      filter: (src) => {
        const rel = path.relative(REAL_PROJECT, src);
        const top = rel.split(path.sep)[0];
        return !['review', 'renders', 'node_modules'].includes(top);
      },
    });
    const realOut = path.join(tmp, 'onoda-out.json');
    const real = runExporter(copyDir, realOut);
    check('real project: exit 0', real.status === 0,
      `status=${real.status} stderr=${real.stderr}`);
    let rdoc = null;
    let rskel = null;
    try { rdoc = JSON.parse(fs.readFileSync(realOut, 'utf8')); } catch { /* null */ }
    try {
      rskel = JSON.parse(fs.readFileSync(
        path.join(tmp, 'onoda-out.skeleton.json'), 'utf8'));
    } catch { /* null */ }
    check('real project: outputs parse', rdoc !== null && rskel !== null);
    if (rdoc) {
      check('real project: title + meta',
        rdoc.meta.title === "The Soldier Who Didn't Know The War Was Over" &&
        rdoc.meta.fps === 30 && rdoc.meta.width === 1080 &&
        rdoc.meta.height === 1920,
        `meta=${JSON.stringify(rdoc.meta)}`);
      check('real project: 5 cast members',
        rdoc.characters.length === 5 &&
        rdoc.characters.map((c) => c.id).join(',') ===
          'onoda,compA,compB,suzuki,taniguchi',
        `characters=${JSON.stringify(rdoc.characters.map((c) => c.id))}`);
      check('real project: 14 shots, 12 captions',
        rdoc.shots.length === 14 && rdoc.captions.length === 12,
        `shots=${rdoc.shots.length} captions=${rdoc.captions.length}`);
      check('real project: first/last shot ids',
        rdoc.shots[0].id === 'S01' && rdoc.shots[13].id === 'S12',
        `first=${rdoc.shots[0]?.id} last=${rdoc.shots[13]?.id}`);
      check('real project: 9 file cues in audio section',
        rdoc.audio.filter((a) => a.type === 'cue').length === 9,
        `audio=${JSON.stringify(rdoc.audio.map((a) => a.type))}`);
      check('real project: native assets in manifest',
        rdoc.assets.some((a) => a.path === 'native/film/onoda.json') &&
        rdoc.assets.some((a) => a.path === 'native/film/assets/cap_000.png' &&
          a.kind === 'image'),
        `assets=${rdoc.assets.length}`);
    }
    if (rskel) {
      const segs = rskel.segments || [];
      check('real project: skeleton segments tile [0, 1811)',
        segs.length === 14 && segs[0].startFrame === 0 &&
        segs[13].endFrame === 1811 &&
        segs.every((s, i) => i === 0 || s.startFrame === segs[i - 1].endFrame),
        `frames=${segs.map((s) => `${s.startFrame}-${s.endFrame}`).join(' ')}`);
      check('real project: skeleton sets JG/SH/BK',
        (rskel.sets || []).map((s) => s.id).join(',') === 'JG,SH,BK',
        `sets=${JSON.stringify(rskel.sets)}`);
      check('real project: skeleton owes the manual remainder',
        (rskel.owes || []).some((o) => o.item === 'draw-body-animation') &&
        (rskel.owes || []).some((o) => o.item === 'caption-strips'),
        `owes=${JSON.stringify((rskel.owes || []).map((o) => o.item))}`);
      check('real project: no native-package-absent owe (bridged project)',
        !(rskel.owes || []).some((o) => o.item === 'native-package-absent'));
    }
    const after = listTree(REAL_PROJECT);
    check('real project: original tree untouched',
      JSON.stringify(before) === JSON.stringify(after),
      `before=${before.length} files after=${after.length} files`);
  }
} finally {
  fs.rmSync(tmp, { recursive: true, force: true });
}

console.log(`PASS ${passed} FAIL ${failed}`);
process.exit(failed === 0 ? 0 : 1);
