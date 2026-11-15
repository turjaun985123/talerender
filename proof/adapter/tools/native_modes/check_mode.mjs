// check_mode.mjs — parity-complete native `check` for capture_native.mjs
// (CR-B1-04). Self-contained: spawn as a CLI, or import { runCheck }.
//
// The adapter's built-in check prints ONLY the binary's telemetry
// block (header + audit lines + `FILM TELEMETRY:` verdict) and its own
// CHECK OK/FAILED. Against template/tools/capture.mjs check, the
// following Chrome line classes were missing or divergent (verified
// in CR-B1-04-PARITY.md); this module closes each one skill-side,
// using the binary's `--film-telemetry --tables` output (which the
// adapter never requests) plus the project's own script.json:
//
//   1. `duration <d> s` line                — from the manifest.
//   2. Shot list lines (`S01  0.00 → 5.85  (5.85 s)  <note>`) —
//      reformatted from the tables' SHOT TABLE (at/end/dur exact).
//      SUBSTITUTION, labelled in the output: the trailing field is
//      the shot's set/atmo­sphere from the tables, because OP.shots()'
//      free-text `note` exists only page-side and is not in the film
//      data. Everything else in the line is identical in shape.
//   3. `captions:` list (`  s–e text`, time order) — reformatted
//      from the tables' CAPTION TABLE (window + text exact).
//   4. `CAST: a, b, …` line — from the tables' CAST LIST ids.
//   5. Caption-audit authored/estimate lines — CORRECTED. The binary
//      constructs every film caption with auto=true, so its
//      `CAPTION AUDIT: … 0/12 lines authored` + estimate-grade line
//      are artefacts on any project whose script.json authors caps
//      (Onoda: all 12 lines author `caps`). When script.json is
//      present, this module recomputes authored lines from
//      lines[].caps and replaces those two lines; without script.json
//      it passes the binary's lines through and says so.
//   6. CAP_CHECK semantics — Chrome defaults caption WIDTH/FIT to
//      WARN (verdict-neutral); only CAP_CHECK=strict folds them in.
//      The binary's film telemetry hard-codes strict (film.rs calls
//      print_caption_audit(&caps, true, …)), so its verdict counts
//      caption failures unconditionally. This module re-derives the
//      verdict from the binary's own FAILED counts: caption failures
//      count ONLY under CAP_CHECK=strict (the FAIL tag is downgraded
//      to WARN otherwise), exactly as in capture.mjs.
//   7. `renderer:` line — Chrome prints the WebGL renderer string.
//      There is no WebGL here; the module prints the native
//      equivalent (`renderer: native (TaleRender) · binary <path>`)
//      so the provenance slot in the output is never silently absent.
//
// Preserved exactly: the binary's audit line classes and formats
// (CANVAS OVERFLOW, EDGE TOUCH, CONTINUITY, CAPTION ZONE, SAFE AREA,
// FRAMING, COMPOSITION, CAPTION AUDIT/WIDTH/FIT) pass through
// byte-for-byte (except the §6 tag downgrade), in Chrome's print
// order; the final line is `CHECK OK` / `CHECK FAILED`.
//
// Exit codes: 0 = CHECK OK; 1 = CHECK FAILED or a prerequisite /
// film-data failure (adapter parity); 3 = the telemetry itself could
// not run (no verdict line — stale binary, unreadable data). The 3
// is NEW vs the adapter (which collapses it to 1) and is the binary's
// own EXIT-CODES-W02 code, surfaced instead of hidden; the lead may
// keep the adapter's 1 if strict adapter parity is preferred — the
// verdict line behaviour is identical either way.
// Env: TALERENDER_BIN, NATIVE_DIR, NATIVE_FILM_DIR, FPS, CAP_CHECK —
// as in capture_native.mjs.
import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const ROOT = path.resolve(HERE, '..', '..');
const NATIVE_DIR = path.resolve(ROOT, process.env.NATIVE_DIR || 'native');
const STRICT = String(process.env.CAP_CHECK ?? '').trim().toLowerCase() === 'strict';
function fail(msg) { throw new Error(msg); }
function isExec(p) { try { return fs.statSync(p).isFile() && (fs.accessSync(p, fs.constants.X_OK), true); } catch { return false; } }

export function findBinary() {
  const env = process.env.TALERENDER_BIN;
  if (env) { if (isExec(env)) return { bin: env, via: 'TALERENDER_BIN' }; fail(`TALERENDER_BIN="${env}" is not an executable file`); }
  for (const [p, via] of [[path.join(NATIVE_DIR, 'talerender'), 'project native dir'], [path.join(os.homedir(), 'workspace', 'talerender', 'talerender'), 'TaleRender repo checkout']]) if (isExec(p)) return { bin: p, via };
  for (const d of String(process.env.PATH || '').split(path.delimiter)) { if (!d) continue; const p = path.join(d, 'talerender'); if (isExec(p)) return { bin: p, via: 'PATH' }; }
  fail('talerender binary not found (set TALERENDER_BIN)');
}
export function findFilmDir() {
  if (process.env.NATIVE_FILM_DIR) { const p = path.resolve(ROOT, process.env.NATIVE_FILM_DIR); if (fs.existsSync(path.join(p, 'onoda.json'))) return p; fail(`NATIVE_FILM_DIR="${p}" has no onoda.json`); }
  for (const p of [path.join(NATIVE_DIR, 'film'), NATIVE_DIR]) if (fs.existsSync(path.join(p, 'onoda.json'))) return p;
  fail('film data dir not found (no onoda.json)');
}
export function loadManifest() {
  for (const p of [path.join(NATIVE_DIR, 'film.json'), path.join(NATIVE_DIR, 'scenes', 'film', 'film.json')]) {
    if (fs.existsSync(p)) return JSON.parse(fs.readFileSync(p, 'utf8'));
  }
  fail('native film manifest not found');
}
// Authored-caption lines from script.json (lines[i].caps present =
// authored chunks; absent = auto-chunked, captions.ts:3-5).
export function authoredLines(root = ROOT) {
  try {
    const s = JSON.parse(fs.readFileSync(path.join(root, 'script.json'), 'utf8'));
    if (!Array.isArray(s.lines)) return null;
    const authored = new Set(), all = new Set();
    s.lines.forEach((l, i) => { all.add(i); if (typeof l.caps === 'string' && l.caps.trim()) authored.add(i); });
    return { authored, all };
  } catch { return null; }
}

const SHOT_ROW = /^(\S+)\s+\d+-\d+\s+(\d+\.\d+)-(\d+\.\d+)\s+(\d+\.\d+)\s{2,}(\S*)\s+(\S+)\s+(.*)$/;
const CAP_ROW = /^(\d+)\s+(\d+)\s+(\S+)\s+(\d+\.\d+)-(\d+\.\d+)\s+(\d+\.\d+)\s+(\d+)\s+(\d+)\s{2}(\S*)\s+"(.*)"$/;
const CAST_ROW = /^(\S+)\s+\d+\s+\d+\s+\d+\s{2}(\S.*)$/;

export function reformatTables(tableLines) {
  const shots = [], caps = [], cast = [];
  let section = null;
  for (const l of tableLines) {
    if (l.startsWith('SHOT TABLE')) { section = 'shots'; continue; }
    if (l.startsWith('CAPTION TABLE')) { section = 'caps'; continue; }
    if (l.startsWith('CAST LIST')) { section = 'cast'; continue; }
    if (section === 'shots') { const m = l.match(SHOT_ROW); if (m) shots.push({ id: m[1], at: +m[2], end: +m[3], dur: +m[4], set: m[5], atmo: m[6] }); }
    else if (section === 'caps') { const m = l.match(CAP_ROW); if (m) caps.push({ s: +m[4], e: +m[5], text: m[10] }); }
    else if (section === 'cast') { const m = l.match(CAST_ROW); if (m && !l.includes('beats:')) cast.push(m[1]); }
  }
  return { shots, caps, cast };
}

export function runCheck({ bin, via, filmDir, manifest, out = console.log } = {}) {
  if (!bin) ({ bin, via } = findBinary());
  filmDir = filmDir || findFilmDir();
  manifest = manifest || loadManifest();
  const r = spawnSync(bin, ['--film-telemetry', filmDir, '--tables'], { encoding: 'utf8', maxBuffer: 64 * 1024 * 1024 });
  if (r.error) fail(`film telemetry failed to start: ${r.error.message}`);
  if (r.stderr) process.stderr.write(r.stderr);
  const lines = String(r.stdout || '').split('\n');
  const verdictIdx = lines.findIndex((l) => l.startsWith('FILM TELEMETRY: CHECK'));
  if (verdictIdx === -1) {
    // No verdict line: stale binary / unreadable data (EXIT-CODES-W02
    // exit 3 territory). Fail closed, distinctly.
    console.error('check_mode: film telemetry produced no verdict line — treated as a FAILED gate (see stderr above)');
    out('CHECK FAILED');
    process.exitCode = 3;
    return false;
  }
  const verdictLine = lines[verdictIdx];
  const auditLines = lines.slice(0, verdictIdx).filter((l) => !l.startsWith('FILM TELEMETRY —') && !l.startsWith('paint-cache:') && l.trim() !== '');
  const { shots, caps, cast } = reformatTables(lines.slice(verdictIdx + 1));

  // Verdict counts from the binary's own FAILED line.
  let nOverflow = 0, nViol = 0, nCap = 0;
  const vm = verdictLine.match(/CHECK FAILED \((\d+) overflow groups, (\d+) continuity violations, (\d+) caption failures\)/);
  if (vm) { nOverflow = +vm[1]; nViol = +vm[2]; nCap = +vm[3]; }

  out(`duration ${Number(manifest.durationS).toFixed(2)} s`);
  out(`renderer: native (TaleRender) · binary ${bin} (via ${via || 'given'}) · no WebGL renderer string exists on the native path`);
  for (const s of shots) out(`${s.id.padEnd(5)} ${s.at.toFixed(2).padStart(6)} → ${s.end.toFixed(2).padStart(6)}  (${s.dur.toFixed(2)} s)  set=${s.set} atmosphere=${s.atmo} (tables dressing; OP.shots() note is page-side only)`);
  out('captions:');
  for (const c of caps) out(`  ${c.s.toFixed(2).padStart(6)}–${c.e.toFixed(2).padEnd(6)} ${c.text}`);
  if (cast.length) out(`CAST: ${cast.join(', ')}`);

  // Audit lines, with the §5 authored correction and §6 tag rule.
  const auth = authoredLines();
  let auditSummarySeen = false;
  for (const l of auditLines) {
    if (l.startsWith('CAPTION AUDIT:')) {
      auditSummarySeen = true;
      if (auth) {
        const m = l.match(/^CAPTION AUDIT: (\d+) chunks, \d+\/\d+ lines authored, min fit (\S+), max rendered width (\S+) px$/);
        if (m) { out(`CAPTION AUDIT: ${m[1]} chunks, ${auth.authored.size}/${auth.all.size} lines authored, min fit ${m[2]}, max rendered width ${m[3]} px (authored count corrected from script.json — the binary marks every film caption auto)`); continue; }
      }
      out(l); continue;
    }
    if (l.startsWith('CAPTIONS: estimate-grade')) {
      if (auth) {
        const auto = [...auth.all].filter((i) => !auth.authored.has(i)).sort((a, b) => a - b);
        if (!auto.length) continue; // all lines authored: Chrome prints no such line
        out(`CAPTIONS: estimate-grade (auto-chunked) on lines ${auto.map((i) => i + 1).join(', ')} — author caps in script.json for a master (corrected from script.json)`);
        continue;
      }
      out(l); continue;
    }
    if (!STRICT && /^CAPTION (WIDTH|FIT) FAIL:/.test(l)) { out(l.replace(' FAIL:', ' WARN:')); continue; }
    out(l);
  }
  if (!auditSummarySeen) out('CAPTION AUDIT: (not present in telemetry output)');

  const failed = nOverflow > 0 || nViol > 0 || (STRICT && nCap > 0);
  out(failed ? 'CHECK FAILED' : 'CHECK OK');
  if (failed) process.exitCode = 1;
  return !failed;
}

const isMain = process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url);
if (isMain) {
  try { runCheck(); } catch (e) { console.error(`check_mode: ${e.message}`); process.exit(1); }
}
