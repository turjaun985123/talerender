// Self-test for tools/export/export_project.mjs (TaleRender V2,
// Batch 2, Seat 5 — WP5 export pipeline, contract C4).
//
// Hermetic: builds fixture source dirs in a fresh temp dir, runs
// the exporter as a child process (node:child_process), and asserts
//   1. success case: exit 0, output parses, format string is
//      "talerender-project/1", meta/assets/section counts match —
//      including an array section passed through verbatim and a
//      numeric-count section expanded to that many {} objects;
//   2. missing-asset case: a fixture with one real asset file and
//      one missing asset exits 2 and names the missing path;
//   3. `..` case: an asset path with a `..` component exits 2.
// Prints one line per assertion and a final PASS/FAIL count line.
// Exit code: 0 iff FAIL is 0. No npm dependencies (node: builtins).

import { spawnSync } from 'node:child_process';
import * as fs from 'node:fs';
import * as os from 'node:os';
import * as path from 'node:path';
import { fileURLToPath } from 'node:url';

const EXPORTER = path.join(
  path.dirname(fileURLToPath(import.meta.url)), 'export_project.mjs');

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

function runExporter(sourceDir, outPath) {
  return spawnSync(process.execPath, [EXPORTER, sourceDir, outPath],
    { encoding: 'utf8' });
}

function writeSource(dir, doc, files = {}) {
  fs.mkdirSync(dir, { recursive: true });
  for (const [rel, content] of Object.entries(files)) {
    const p = path.join(dir, rel);
    fs.mkdirSync(path.dirname(p), { recursive: true });
    fs.writeFileSync(p, content);
  }
  fs.writeFileSync(
    path.join(dir, 'project-source.json'), JSON.stringify(doc, null, 2));
}

const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'talerender-export-test-'));
try {
  // ---- 1. success case -------------------------------------------
  const okDir = path.join(tmp, 'ok');
  writeSource(okDir, {
    title: 'Fixture One',
    fps: 30, width: 1080, height: 1920,
    assets: [{ id: 'hero', path: 'img/hero.png', kind: 'image' }],
    sections: {
      characters: [{ name: 'A' }],   // array: pass through verbatim
      scenes: 2,                     // count: -> [{}, {}]
      shots: [],                     // array, empty
      captions: [{ text: 'hi' }],    // array, one entry
      audio: 0,                      // count, zero -> []
    },
  }, { 'img/hero.png': Buffer.from([0x89, 0x50, 0x4e, 0x47]) });
  const okOut = path.join(tmp, 'ok-out.json');
  const ok = runExporter(okDir, okOut);
  check('success: exit 0', ok.status === 0,
    `status=${ok.status} stderr=${ok.stderr}`);
  check('success: summary is one line naming the title',
    typeof ok.stdout === 'string' &&
    ok.stdout.trim().split('\n').length === 1 &&
    ok.stdout.includes('Fixture One') && ok.stdout.includes('assets=1'),
    `stdout=${JSON.stringify(ok.stdout)}`);
  let doc = null;
  try { doc = JSON.parse(fs.readFileSync(okOut, 'utf8')); } catch { /* stays null */ }
  check('success: output parses as JSON', doc !== null);
  if (doc) {
    check('success: format string', doc.format === 'talerender-project/1',
      `format=${doc.format}`);
    check('success: meta carried through',
      doc.meta && doc.meta.title === 'Fixture One' && doc.meta.fps === 30 &&
      doc.meta.width === 1080 && doc.meta.height === 1920,
      `meta=${JSON.stringify(doc.meta)}`);
    check('success: assets count', Array.isArray(doc.assets) &&
      doc.assets.length === 1 && doc.assets[0].id === 'hero' &&
      doc.assets[0].path === 'img/hero.png' && doc.assets[0].kind === 'image',
      `assets=${JSON.stringify(doc.assets)}`);
    check('success: characters array passed through',
      Array.isArray(doc.characters) && doc.characters.length === 1 &&
      doc.characters[0].name === 'A',
      `characters=${JSON.stringify(doc.characters)}`);
    check('success: scenes count 2 expanded to 2 empty objects',
      Array.isArray(doc.scenes) && doc.scenes.length === 2 &&
      doc.scenes.every((e) => e && typeof e === 'object' &&
        !Array.isArray(e) && Object.keys(e).length === 0),
      `scenes=${JSON.stringify(doc.scenes)}`);
    check('success: shots/captions/audio counts',
      Array.isArray(doc.shots) && doc.shots.length === 0 &&
      Array.isArray(doc.captions) && doc.captions.length === 1 &&
      Array.isArray(doc.audio) && doc.audio.length === 0,
      `shots=${JSON.stringify(doc.shots)} captions=${JSON.stringify(doc.captions)} audio=${JSON.stringify(doc.audio)}`);
  }

  // ---- 2. missing-asset case: one real file, one missing ---------
  const missDir = path.join(tmp, 'missing');
  writeSource(missDir, {
    title: 'Fixture Missing',
    fps: 30, width: 1080, height: 1920,
    assets: [
      { id: 'present', path: 'img/present.png', kind: 'image' },
      { id: 'gone', path: 'img/missing-one.png', kind: 'image' },
    ],
    sections: { scenes: 1 },
  }, { 'img/present.png': Buffer.from([1, 2, 3]) });
  const missOut = path.join(tmp, 'missing-out.json');
  const miss = runExporter(missDir, missOut);
  const missText = `${miss.stdout}\n${miss.stderr}`;
  check('missing: exit 2', miss.status === 2, `status=${miss.status}`);
  check('missing: names the missing path',
    missText.includes('img/missing-one.png'), `output=${missText}`);
  check('missing: no output written', !fs.existsSync(missOut));

  // ---- 3. `..` path-escape case -----------------------------------
  const escDir = path.join(tmp, 'escape');
  writeSource(escDir, {
    title: 'Fixture Escape',
    fps: 30, width: 1080, height: 1920,
    assets: [{ id: 'x', path: '../secret.png', kind: 'image' }],
    sections: {},
  });
  const escOut = path.join(tmp, 'escape-out.json');
  const esc = runExporter(escDir, escOut);
  const escText = `${esc.stdout}\n${esc.stderr}`;
  check('dotdot: exit 2', esc.status === 2, `status=${esc.status}`);
  check('dotdot: names the escaping path',
    escText.includes('../secret.png') && escText.includes('..'),
    `output=${escText}`);
  check('dotdot: no output written', !fs.existsSync(escOut));
} finally {
  fs.rmSync(tmp, { recursive: true, force: true });
}

console.log(`PASS ${passed} FAIL ${failed}`);
process.exit(failed === 0 ? 0 : 1);
