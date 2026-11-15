// filmdata_validate.mjs — P4 generalisation: a data-driven, drop-in
// reimplementation of capture_native.mjs's validateFilmData().
// CHROME-REPLACEMENT WAVE, Batch 1, Seat 6 (CR-B1-06).
// Contracts: CHROME-REPLACEMENT-CONTRACTS.md §0/§1 (bridge state),
// V2-WP9-INTEGRATION.md §8 proposal P4.
//
// THE PROBLEM THIS FIXES: the adapter's validateFilmData() hardcodes
// the Onoda asset list — five hold plates (paper-map / leaflet /
// book-open / book-closed / coconut) plus calendar-1945/1955/1965/
// 1974, calendar-dark and sign — so any non-Onoda bridge is failed
// for lacking Onoda props, and a bridge whose data declares OTHER
// assets is not checked for them at all.
//
// THIS MODULE: export validateFilmData(filmDir, manifest) with the
// EXACT interface of the adapter's function —
//   inputs : filmDir (dir holding onoda.json + assets/), manifest
//            (the loadManifest() result, or null — the manifest-
//            dependent cross-check is then skipped, exactly as in
//            the adapter)
//   output : returns the parsed film data object on success
//   failure: throws an Error whose message is the adapter's message
//            verbatim ("film data in <dir> is incomplete — the
//            binary would fail on:\n  <missing...>\nRegenerate...")
//            with every problem accumulated, never just the first.
// The ONLY behavioural change is where the required-asset list
// comes from: the data itself, never a hardcoded film's names.
//   1. assets/plates.json — always required (the binary's
//      load_assets reads it first), and every plate it declares:
//        - manifest shape (the current binary's, film.rs
//          load_assets): {"format":"talerender-plates","plates":
//          [{"name","file","origin"}, ...]}  -> require `file`
//          (defaulting to `<name>.png` when file is absent)
//        - legacy flat shape (older bridges): {"<name>": [x, y]}
//          -> require `<name>.png`
//   2. The film data's own F2 declarations (onoda.json — the same
//      fields film.rs load_film_data demands by name):
//        calendarYears: ["1945", ...]  -> calendar-<year>.png each
//        calendarDark:  "calendar-dark.png" (a filename)
//        sign:          "sign.png"          (a filename)
//      Absent fields declare nothing and require nothing — a film
//      with no calendar overlay is not failed for lacking one.
//   3. Every caption strip named by captions[].strip (unchanged
//      from the adapter — that part was already data-driven).
// Everything else — duration/lines/anchors/captions presence, the
// manifest<->data frame-count agreement — is the adapter's check,
// byte for byte.
//
// WIRING (lead): in template/tools/capture_native.mjs, delete the
// local validateFilmData() definition and import this one — the
// call site in main() (`validateFilmData(filmDir, manifest)`,
// inside the existing try/catch that accumulates problems) does
// not change. See docs/integration/CR-B1-06-BRIDGE.md in the
// TaleRender repo for the exact swap point.
//
// Node builtins only. Self-test: `node filmdata_validate.mjs
// --self-test` builds temp fixtures (Onoda-shaped bridge passes;
// a bridge missing a DATA-NAMED asset fails naming it; a non-Onoda
// bridge without calendar plates passes) and exits 0/1.
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

function fail(msg) { throw new Error(msg); }
function readJson(p, what) {
  try { return JSON.parse(fs.readFileSync(p, 'utf8')); }
  catch (e) { fail(`${what}: cannot read/parse ${p}: ${e.message}`); }
}

// Derive the required asset filenames (relative to assets/) from
// the film data + plates.json content. Exported so the bridge
// builder's Python mirror and any test can assert the same list.
// `data` is the parsed onoda.json; `plates` is the parsed
// plates.json, or null when plates.json itself is missing /
// unparseable (its own absence is reported by the caller).
export function requiredAssets(data, plates) {
  const need = [];
  const add = (rel) => { if (rel && !need.includes(rel)) need.push(rel); };
  // 1. Plates declared by plates.json.
  if (plates && typeof plates === 'object') {
    if (Array.isArray(plates.plates)) {
      // Manifest shape: {"format": "talerender-plates", "plates": [...]}
      for (const e of plates.plates) {
        if (e && typeof e === 'object' && typeof e.name === 'string' && e.name) {
          add(typeof e.file === 'string' && e.file ? e.file : `${e.name}.png`);
        }
      }
    } else if (plates.plates && typeof plates.plates === 'object') {
      for (const name of Object.keys(plates.plates)) add(`${name}.png`);
    } else {
      // Legacy flat shape: {"<plate-name>": [x0, y0], ...}. A
      // top-level "format"/"version" key is metadata, not a plate.
      for (const name of Object.keys(plates)) {
        if (name === 'format' || name === 'version') continue;
        add(`${name}.png`);
      }
    }
  }
  // 2. The film data's own calendar / sign declarations (F2 fields,
  // demanded by name by the binary's load_film_data).
  if (data && typeof data === 'object') {
    if (Array.isArray(data.calendarYears)) {
      for (const y of data.calendarYears) add(`calendar-${y}.png`);
    }
    if (typeof data.calendarDark === 'string' && data.calendarDark) add(data.calendarDark);
    if (typeof data.sign === 'string' && data.sign) add(data.sign);
    // 3. Caption strips named by the data.
    if (Array.isArray(data.captions)) {
      for (const c of data.captions) {
        if (c && typeof c.strip === 'string' && c.strip) add(c.strip);
      }
    }
  }
  return need;
}

export function validateFilmData(filmDir, manifest) {
  const dataPath = path.join(filmDir, 'onoda.json');
  const data = readJson(dataPath, 'film data');
  const missing = [];
  if (!(typeof data.duration === 'number' && data.duration > 0)) missing.push('onoda.json: duration missing/invalid');
  if (!Array.isArray(data.lines) || !data.lines.length) missing.push('onoda.json: lines[] missing/empty');
  if (!data.anchors || typeof data.anchors !== 'object') missing.push('onoda.json: anchors{} missing');
  if (!Array.isArray(data.captions) || !data.captions.length) missing.push('onoda.json: captions[] missing/empty');
  if (manifest && typeof data.duration === 'number' && Math.ceil(data.duration * manifest.fps) !== manifest.totalFrames) {
    missing.push(`onoda.json: duration ${data.duration}s implies ${Math.ceil(data.duration * manifest.fps)} frames, manifest totalFrames is ${manifest.totalFrames} — the bridge data and the manifest disagree; regenerate them together`);
  }
  const adir = path.join(filmDir, 'assets');
  const need = (rel) => { if (!fs.existsSync(path.join(adir, rel))) missing.push(`assets/${rel}`); };
  need('plates.json');
  // Plates: derive from plates.json itself (both shapes). A missing
  // or unparseable plates.json is already reported by the need()
  // above; its entries then declare nothing further here — the
  // binary would fail on plates.json before any plate file.
  let plates = null;
  try { plates = JSON.parse(fs.readFileSync(path.join(adir, 'plates.json'), 'utf8')); } catch { plates = null; }
  if (Array.isArray(data.captions)) for (const c of data.captions) {
    if (!c || typeof c.strip !== 'string' || !c.strip) missing.push(`onoda.json: a caption chunk has no strip filename`);
  }
  for (const rel of requiredAssets(data, plates)) need(rel);
  if (missing.length) fail(`film data in ${filmDir} is incomplete — the binary would fail on:\n  ${missing.join('\n  ')}\nRegenerate the bridge data; the adapter does not improvise it.`);
  return data;
}

// ---------------------------------------------------------------------
// Self-test (node filmdata_validate.mjs --self-test)
// ---------------------------------------------------------------------
function writeFixture(root, { platesShape, dataExtra, files }) {
  const filmDir = path.join(root, 'film');
  const adir = path.join(filmDir, 'assets');
  fs.mkdirSync(adir, { recursive: true });
  const data = {
    duration: 0.1, fps: 30, frames: 3,
    lines: [{ start: 0, end: 0.09 }],
    anchors: {},
    captions: [{ s: 0, e: 0.09, text: 'One', fit: 1, strip: 'cap_000.png' }],
    ...dataExtra,
  };
  fs.writeFileSync(path.join(filmDir, 'onoda.json'), JSON.stringify(data, null, 1));
  fs.writeFileSync(path.join(adir, 'plates.json'), JSON.stringify(platesShape, null, 1));
  for (const f of files) fs.writeFileSync(path.join(adir, f), Buffer.from([137, 80, 78, 71]));
  return filmDir;
}

function runSelfTest() {
  const cases = [];
  const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'filmdata-validate-'));
  try {
    const manifest = { fps: 30, totalFrames: 3 };
    const onodaPlates = {
      format: 'talerender-plates', version: 1,
      plates: ['paper-map', 'leaflet', 'book-open', 'book-closed', 'coconut']
        .map((name) => ({ name, file: `${name}.png`, origin: [0, 0] })),
    };
    const onodaExtra = {
      calendarYears: ['1945', '1955', '1965', '1974'],
      calendarDark: 'calendar-dark.png', sign: 'sign.png',
    };
    const onodaFiles = ['paper-map.png', 'leaflet.png', 'book-open.png', 'book-closed.png', 'coconut.png',
      'calendar-1945.png', 'calendar-1955.png', 'calendar-1965.png', 'calendar-1974.png',
      'calendar-dark.png', 'sign.png', 'cap_000.png'];

    // Case 1: an Onoda-shaped bridge (manifest plates + F2 calendar
    // declarations, every named file present) passes.
    {
      const dir = writeFixture(path.join(tmp, 'onoda'), { platesShape: onodaPlates, dataExtra: onodaExtra, files: onodaFiles });
      let ok = true, detail = '';
      try { validateFilmData(dir, manifest); } catch (e) { ok = false; detail = e.message; }
      cases.push(['Onoda-shaped bridge passes', ok, detail]);
    }
    // Case 2: a bridge missing a DATA-NAMED asset fails naming it.
    // The plate name "wombat" appears in no hardcoded list anywhere
    // — only plates.json declares it — so a failure that names it
    // can only come from the data-driven derivation.
    {
      const plates = { format: 'talerender-plates', version: 1, plates: [{ name: 'wombat', file: 'wombat.png', origin: [0, 0] }] };
      const dir = writeFixture(path.join(tmp, 'missing'), { platesShape: plates, dataExtra: {}, files: ['cap_000.png'] });
      let ok = false, detail = 'no error thrown';
      try { validateFilmData(dir, manifest); } catch (e) { ok = /assets\/wombat\.png/.test(e.message); detail = e.message.split('\n')[1] || e.message; }
      cases.push(['missing DATA-NAMED asset fails naming it (assets/wombat.png)', ok, detail.trim()]);
    }
    // Case 3: a non-Onoda bridge WITHOUT calendar plates passes —
    // no calendarYears/calendarDark/sign declared, none required.
    // (The adapter's hardcoded version fails this fixture.)
    {
      const plates = { format: 'talerender-plates', version: 1, plates: [{ name: 'lantern', file: 'lantern.png', origin: [-10, -20] }] };
      const dir = writeFixture(path.join(tmp, 'nononoda'), { platesShape: plates, dataExtra: {}, files: ['lantern.png', 'cap_000.png'] });
      let ok = true, detail = '';
      try { validateFilmData(dir, manifest); } catch (e) { ok = false; detail = e.message; }
      cases.push(['non-Onoda bridge without calendar plates passes', ok, detail]);
    }
    // Case 4 (bonus): the legacy flat plates.json shape is honoured.
    {
      const plates = { 'paper-map': [-200, -270], leaflet: [-200, -258] };
      const dir = writeFixture(path.join(tmp, 'legacy'), { platesShape: plates, dataExtra: {}, files: ['paper-map.png', 'cap_000.png'] });
      let ok = false, detail = 'no error thrown';
      try { validateFilmData(dir, manifest); } catch (e) { ok = /assets\/leaflet\.png/.test(e.message); detail = e.message.split('\n')[1] || e.message; }
      cases.push(['legacy flat plates.json: missing leaflet.png named', ok, detail.trim()]);
    }
  } finally { fs.rmSync(tmp, { recursive: true, force: true }); }
  let passed = 0;
  for (const [name, ok, detail] of cases) {
    console.log(`self-test case ${name}: ${ok ? 'PASS' : 'FAIL'}${detail ? ` (${detail})` : ''}`);
    if (ok) passed++;
  }
  console.log(`SELF-TEST: ${passed === cases.length ? 'PASS' : 'FAIL'} (${passed}/${cases.length})`);
  return passed === cases.length ? 0 : 1;
}

const invokedAsMain = process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url);
if (invokedAsMain) {
  if (process.argv.slice(2).includes('--self-test')) {
    process.exit(runSelfTest());
  } else {
    console.log('filmdata_validate.mjs — drop-in validateFilmData(filmDir, manifest) for capture_native.mjs (P4). Run with --self-test to verify.');
  }
}
