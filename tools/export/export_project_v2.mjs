// =====================================================================
// DELIVERY NOTE — CHROME-REPLACEMENT WAVE, Batch 1, Seat 7:
// TS export hardening for arbitrary projects (TaleRender tools).
// Contract: docs/integration/CHROME-REPLACEMENT-CONTRACTS.md (Batch 1
// "TS export hardening"); pipeline contract + assumptions:
// docs/integration/CR-B1-07-EXPORT.md. Supersedes nothing: v1
// (tools/export/export_project.mjs) stays as the fixture-manifest
// exporter; this file is the REAL-PROJECT exporter.
//
// WHAT IT DOES
//   Given a Tall Tale skill project directory (the shape produced
//   by the skill template — verified against the template project
//   and the real Onoda project
//   ~/faceless-shorts/the-soldier-who-didnt-know), assemble the C4
//   interchange document (`talerender-project/1`, the document
//   src/projectfile.rs loads) from the project's REAL files:
//
//     script.json        -> meta.title + narration script (lines,
//                           caps, voice/speed/lead/gap/outro/music)
//     data/lines.json    -> line timings (the film clock)
//     data/words.json    -> word timings (caption + anchor source)
//     data/cues.json     -> authored cue events (if present)
//     src/shots.ts       -> cast (LOOK registry), sets, shots —
//                           STATIC extraction only (regex-level; the
//                           file is never executed)
//     project tree       -> the asset manifest (curated roots, see
//                           collectAssets)
//
//   …and additionally emit a SEGMENT SKELETON REPORT (JSON, format
//   `talerender-skeleton/1`): per-shot staging facts extracted
//   statically from src/shots.ts, plus an explicit `owes` list —
//   everything transcription still owes. That report is the bridge
//   builder's input contract (tools/bridge/): the bridge turns the
//   skeleton + this interchange document into a project's native/
//   package (film manifest, §15 scene files, film data). See
//   CR-B1-07-EXPORT.md for the full pipeline contract and the
//   assumptions this shape rests on.
//
// VALIDATION — rule-for-rule the projectfile.rs rules, as in v1:
//   * meta.title must be a string (MAY be empty — projectfile.rs
//     imposes no non-emptiness rule, and neither does this tool);
//   * meta fps / width / height must be positive integers (> 0,
//     integer-valued, fitting u32);
//   * asset `kind` is the closed set image | audio | data;
//   * asset `id` / `path` must be non-empty strings;
//   * asset paths are relative only (leading `/` or `\`, Windows
//     drive prefix, or absolute in either spelling rejected) and
//     no `..` component is allowed (path escape);
//   * a present section that is not an array is a problem (the
//     emitted sections are always arrays by construction; the
//     rule is re-checked on the assembled document before write);
//   * EVERY missing asset is listed, never a silent skip.
//   ALL problems are collected and listed together (stderr, one
//   per line) before exiting 2 — never just the first, and NEVER
//   a partial write: on failure neither output file is created
//   or modified. (v1/projectfile.rs parity note: the Rust loader
//   returns on its first error; an authoring tool batches. The
//   SET of rules is the same.)
//
//   v2 adds PROJECT-level problems v1 could not have (its input
//   was a fixture manifest): unreadable / malformed script.json,
//   data/lines.json, data/words.json or src/shots.ts; a script
//   whose line count disagrees with data/lines.json; duplicate
//   shot ids; shot start times that run backwards. Each is a
//   problem in the same batched list, exit 2.
//
// META RESOLUTION (fps / width / height), first hit wins:
//   1. CLI flags --fps / --width / --height;
//   2. src/core/lib.ts `export const W = …, H = …, FPS = …`
//      (the engine's own constants — 1080 / 1920 / 30 in every
//      project inspected);
//   3. native/film.json fps / resolution (a bridged project);
//   4. the skill defaults 30 / 1080 / 1920.
//   Title comes from script.json `title` (required, must be a
//   string). The resolved source of each meta field is recorded
//   in the skeleton report.
//
// CLI
//   node export_project_v2.mjs <project-dir> <out.json>
//        [--skeleton <path>] [--fps N] [--width N] [--height N]
//   The skeleton defaults to <out basename>.skeleton.json next to
//   <out.json>. Exit codes: 0 = both files written (exactly one
//   summary line on stdout); 2 = usage error or any problem.
//
// NO npm dependencies: Node >= 18, `node:` builtins only. The
// project directory is only ever READ.
// =====================================================================

import * as fs from 'node:fs';
import * as path from 'node:path';

const SUPPORTED_FORMAT = 'talerender-project/1';
const SKELETON_FORMAT = 'talerender-skeleton/1';
const ASSET_KINDS = new Set(['image', 'audio', 'data']);
const SECTION_NAMES = ['characters', 'scenes', 'shots', 'captions', 'audio'];
const U32_MAX = 4294967295;
const DEFAULTS = { fps: 30, width: 1080, height: 1920 };

// ------------------------------------------------------------------
// small helpers (typeName / isPlainObject / path law mirror v1)
// ------------------------------------------------------------------

function typeName(v) {
  if (v === null) return 'null';
  if (Array.isArray(v)) return 'an array';
  switch (typeof v) {
    case 'boolean': return 'a boolean';
    case 'number': return 'a number';
    case 'string': return 'a string';
    case 'object': return 'an object';
    default: return typeof v;
  }
}

function isPlainObject(v) {
  return v !== null && typeof v === 'object' && !Array.isArray(v);
}

function isPosU32(v) {
  return typeof v === 'number' && Number.isInteger(v) && v > 0 && v <= U32_MAX;
}

// C4 path law for one asset, projectfile.rs `validate_asset_path`
// rule: relative only, and no `..` component. Returns true when
// the path is usable (safe to existence-check under the root).
function checkAssetPath(id, p, problems, where) {
  const drivePrefixed = /^[A-Za-z]:/.test(p);
  if (p.startsWith('/') || p.startsWith('\\') || drivePrefixed ||
      path.posix.isAbsolute(p) || path.win32.isAbsolute(p)) {
    problems.push(
      `${where}: asset '${id}': absolute path rejected: '${p}' ` +
      '— asset paths must be relative to the project root');
    return false;
  }
  if (p.split(/[\\/]/).some((component) => component === '..')) {
    problems.push(
      `${where}: asset '${id}': path escape rejected: '${p}' ` +
      `contains a '..' component — asset paths must not escape the project root`);
    return false;
  }
  return true;
}

function readJsonFile(absPath, label, problems) {
  let text;
  try {
    text = fs.readFileSync(absPath, 'utf8');
  } catch (e) {
    problems.push(`${label}: cannot read '${absPath}': ${e.message}`);
    return undefined;
  }
  try {
    return JSON.parse(text);
  } catch (e) {
    problems.push(`${label}: invalid JSON in '${absPath}': ${e.message}`);
    return undefined;
  }
}

function readTextFile(absPath, label, problems) {
  try {
    return fs.readFileSync(absPath, 'utf8');
  } catch (e) {
    problems.push(`${label}: cannot read '${absPath}': ${e.message}`);
    return undefined;
  }
}

// ------------------------------------------------------------------
// asset manifest: curated roots of a real project tree
// ------------------------------------------------------------------

const IMAGE_EXTS = new Set(['.png', '.jpg', '.jpeg', '.webp', '.gif']);
const AUDIO_EXTS = new Set(['.wav', '.mp3', '.ogg', '.m4a', '.aac', '.flac']);
const DATA_EXTS = new Set([
  '.json', '.txt', '.md', '.ttf', '.otf', '.woff', '.woff2', '.ts',
]);

function kindForExt(ext) {
  if (IMAGE_EXTS.has(ext)) return 'image';
  if (AUDIO_EXTS.has(ext)) return 'audio';
  if (DATA_EXTS.has(ext)) return 'data';
  return null; // not an interchange asset (code, html, binaries…)
}

// Directories never walked for assets: dependencies, VCS, build
// output and review/render products are not film inputs.
const SKIP_DIRS = new Set([
  'node_modules', '.git', 'build', 'review', 'renders', 'vendor',
]);

function walkFiles(rootAbs, relBase, out) {
  let entries;
  try {
    entries = fs.readdirSync(rootAbs, { withFileTypes: true });
  } catch {
    return;
  }
  entries.sort((a, b) => (a.name < b.name ? -1 : a.name > b.name ? 1 : 0));
  for (const e of entries) {
    const rel = relBase ? `${relBase}/${e.name}` : e.name;
    if (e.isDirectory()) {
      if (!SKIP_DIRS.has(e.name)) walkFiles(path.join(rootAbs, e.name), rel, out);
    } else if (e.isFile()) {
      out.push(rel);
    }
  }
}

function assetIdFor(relPath) {
  return relPath.replace(/\.[^.]*$/, '').replace(/[^A-Za-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '').toLowerCase() || 'asset';
}

// The manifest covers the film's INPUTS: the script, everything
// under data/ and fonts/, the staging source src/shots.ts, and —
// when a bridge package already exists — the native/ tree that
// package consists of. Deterministic (sorted) order.
function collectAssets(projectDir) {
  const rels = new Set();
  const addTree = (sub) => {
    const abs = path.join(projectDir, sub);
    if (fs.existsSync(abs) && fs.statSync(abs).isDirectory()) {
      const files = [];
      walkFiles(abs, sub, files);
      for (const f of files) rels.add(f);
    }
  };
  if (fs.existsSync(path.join(projectDir, 'script.json'))) rels.add('script.json');
  addTree('data');
  addTree('fonts');
  if (fs.existsSync(path.join(projectDir, 'src', 'shots.ts'))) rels.add('src/shots.ts');
  addTree('native');
  const assets = [];
  for (const rel of [...rels].sort()) {
    const kind = kindForExt(path.extname(rel).toLowerCase());
    if (kind) assets.push({ id: assetIdFor(rel), path: rel, kind });
  }
  return assets;
}

// ------------------------------------------------------------------
// timing expression resolver (shots.ts static evaluation)
//
// shots.ts places shots and cues with a tiny expression language
// over the narration clock (src/core/timing.ts):
//   cut(i) = i == 0 ? 0 : L(i) - 0.06      L(i)/Le(i) = line bounds
//   Wd('word', line) = start of the first word in `line` whose
//   normalised form starts with the normalised needle (timing.ts
//   `norm`: lowercase, strip [^a-z0-9']).
// The resolver evaluates: numbers, identifiers bound by a simple
// `const x = <expr>;`, cut/L/Le/Wd calls, and + / - chains of
// those. Anything else resolves to null and is reported in the
// skeleton's `owes` — never guessed.
// ------------------------------------------------------------------

function makeResolver(lines, words) {
  const norm = (w) => String(w).toLowerCase().replace(/[^a-z0-9']+/g, '');
  const lineStart = (i) => (lines[i] ? lines[i].start : null);
  const lineEnd = (i) => (lines[i] ? lines[i].end : null);
  const wordAt = (needle, line, which = 's') => {
    const n = norm(needle);
    const hit = words.find(
      (w) => (line == null || w.line === line) && norm(w.w).startsWith(n));
    return hit ? hit[which] : null;
  };

  function evaluate(expr, env, depth = 0) {
    if (depth > 8 || typeof expr !== 'string') return null;
    const src = expr.trim();
    if (/^-?\d+(?:\.\d+)?$/.test(src)) return Number(src);
    // split a top-level + / - chain (no parentheses nesting in the
    // grammar except inside Wd()/cut() calls, handled by depth scan)
    const parts = [];
    let depthP = 0;
    let cur = '';
    let sign = 1;
    for (let i = 0; i < src.length; i++) {
      const c = src[i];
      if (c === '(') depthP += 1;
      if (c === ')') depthP -= 1;
      if (depthP === 0 && (c === '+' || c === '-') && cur.trim() !== '') {
        parts.push({ sign, text: cur });
        cur = '';
        sign = c === '+' ? 1 : -1;
      } else {
        cur += c;
      }
    }
    parts.push({ sign, text: cur });
    if (parts.length > 1) {
      let total = 0;
      for (const part of parts) {
        const v = evaluate(part.text, env, depth + 1);
        if (v === null) return null;
        total += part.sign * v;
      }
      return total;
    }
    let m = src.match(/^cut\(\s*(\d+)\s*\)$/);
    if (m) {
      const i = Number(m[1]);
      if (i === 0) return 0;
      const s = lineStart(i);
      return s === null ? null : s - 0.06;
    }
    m = src.match(/^L\(\s*(\d+)\s*\)$/);
    if (m) return lineStart(Number(m[1]));
    m = src.match(/^Le\(\s*(\d+)\s*\)$/);
    if (m) return lineEnd(Number(m[1]));
    m = src.match(/^Wd\(\s*'((?:[^'\\]|\\.)*)'\s*(?:,\s*(\d+)\s*)?(?:,\s*'([se])'\s*)?\)$/);
    if (m) {
      return wordAt(m[1].replace(/\\'/g, "'"),
        m[2] === undefined ? null : Number(m[2]), m[3] || 's');
    }
    if (/^[A-Za-z_$][\w$]*$/.test(src)) {
      if (env && Object.prototype.hasOwnProperty.call(env, src)) {
        return evaluate(env[src], env, depth + 1);
      }
      return null;
    }
    return null;
  }
  return { evaluate };
}

// Collect simple `const name = <expr>;` bindings whose RHS is in
// the resolver grammar (used for shot `at` vars and cue anchors).
function collectBindings(tsText) {
  const env = {};
  const re = /const\s+([A-Za-z_$][\w$]*)\s*=\s*([^;]+);/g;
  let m;
  while ((m = re.exec(tsText)) !== null) {
    const rhs = m[2].trim();
    if (/^(?:-?[\d.]+|[A-Za-z_$][\w$]*(?:\s*[+-]\s*.+)?|cut\(\d+\)|L\(\d+\)|Le\(\d+\)|Wd\('(?:[^'\\]|\\.)*'(?:\s*,\s*\d+)?(?:\s*,\s*'[se]')?\))$/.test(rhs)) {
      env[m[1]] = rhs;
    }
  }
  return env;
}

// ------------------------------------------------------------------
// src/shots.ts static extraction (regex-level; never executed)
// ------------------------------------------------------------------

function unescapeTsString(s) {
  return s.replace(/\\'/g, "'").replace(/\\"/g, '"').replace(/\\\\/g, '\\');
}

// The LOOK registry: `const LOOK: Record<string, ToonLook> = { … }`
// with one flat object per character. Field values are kept raw
// (strings / string arrays / numbers) — deep ToonLook semantics
// belong to the bridge, not the exporter.
function extractCast(tsText) {
  const cast = [];
  const start = tsText.search(/const\s+LOOK\s*:\s*Record<string,\s*ToonLook>\s*=/);
  if (start < 0) return cast;
  const open = tsText.indexOf('{', start);
  if (open < 0) return cast;
  let depth = 0;
  let end = -1;
  for (let i = open; i < tsText.length; i++) {
    if (tsText[i] === '{') depth += 1;
    if (tsText[i] === '}') {
      depth -= 1;
      if (depth === 0) { end = i; break; }
    }
  }
  if (end < 0) return cast;
  const body = tsText.slice(open + 1, end);
  const entryRe = /([A-Za-z_$][\w$]*)\s*:\s*\{([^{}]*)\}/g;
  let m;
  while ((m = entryRe.exec(body)) !== null) {
    const fields = {};
    const fieldRe = /([A-Za-z_$][\w$]*)\s*:\s*('(?:[^'\\]|\\.)*'|\[[^\]]*\]|-?[\d.]+|true|false)/g;
    let f;
    while ((f = fieldRe.exec(m[2])) !== null) {
      const raw = f[2];
      if (raw.startsWith("'")) fields[f[1]] = unescapeTsString(raw.slice(1, -1));
      else if (raw.startsWith('[')) {
        fields[f[1]] = [...raw.matchAll(/'((?:[^'\\]|\\.)*)'/g)]
          .map((x) => unescapeTsString(x[1]));
      } else if (raw === 'true' || raw === 'false') fields[f[1]] = raw === 'true';
      else fields[f[1]] = Number(raw);
    }
    cast.push({ id: m[1], look: fields, lookRaw: m[2].trim() });
  }
  return cast;
}

// Per-set person() instances: `const jgOnoda = person('onoda',
// { perspective: true }, '#878E6E')` — also the comma-chained
// template form `const a = person('x'), b = person('y', {...})`.
// Scene membership from `<SET>.scene.add(<var>.group)`.
function extractInstances(tsText) {
  const instances = [];
  const re = /(?:const\s+|,\s*)([A-Za-z_$][\w$]*)\s*=\s*person\(\s*'((?:[^'\\]|\\.)*)'\s*(?:,\s*(\{[^{}]*\}))?\s*(?:,\s*'((?:[^'\\]|\\.)*)')?\s*\)/g;
  let m;
  while ((m = re.exec(tsText)) !== null) {
    instances.push({
      var: m[1],
      character: unescapeTsString(m[2]),
      optionsRaw: m[3] ? m[3].trim() : null,
      shade: m[4] ? unescapeTsString(m[4]) : null,
      set: null,
    });
  }
  const addRe = /([A-Za-z_$][\w$]*)\.scene\.add\(\s*([A-Za-z_$][\w$]*)\.group\s*\)/g;
  const setOfVar = new Map();
  while ((m = addRe.exec(tsText)) !== null) setOfVar.set(m[2], m[1]);
  for (const inst of instances) {
    if (setOfVar.has(inst.var)) inst.set = setOfVar.get(inst.var);
  }
  return instances;
}

// Sets: `const JG = (() => { … })()` blocks, labelled by the
// preceding `SET n · …` banner comment when one exists.
function extractSets(tsText) {
  const sets = [];
  const re = /const\s+([A-Za-z_$][\w$]*)\s*=\s*\(\(\)\s*=>\s*\{/g;
  let m;
  while ((m = re.exec(tsText)) !== null) {
    const before = tsText.slice(Math.max(0, m.index - 400), m.index);
    const banner = [...before.matchAll(/SET\s+\d+\s*·\s*([^\n*]+)/g)].pop();
    const after = tsText.slice(m.index, m.index + 6000);
    sets.push({
      id: m[1],
      label: banner ? banner[1].trim() : null,
      kind: /WD\.interior\s*\(/.test(after) ? 'interior'
        : /WD\.sky\s*\(/.test(after) ? 'exterior' : 'unknown',
    });
  }
  return sets;
}

const SHOT_ADD_RE =
  /add\(\{\s*id:\s*'((?:[^'\\]|\\.)*)'\s*,\s*at:\s*([^,]+?)\s*,\s*note:\s*'((?:[^'\\]|\\.)*)'/g;

// Split a declaration tail (`x = <expr>, y = <expr>; …`) on
// TOP-LEVEL commas, stopping at the top-level `;`. Parentheses /
// brackets / braces and string literals are depth-tracked, so the
// comma inside Wd('thought', 4) does not split.
function splitTopLevel(text) {
  const parts = [];
  let depth = 0;
  let cur = '';
  let quote = null;
  for (let i = 0; i < text.length; i++) {
    const c = text[i];
    if (quote) {
      cur += c;
      if (c === '\\') { cur += text[i + 1] || ''; i += 1; }
      else if (c === quote) quote = null;
      continue;
    }
    if (c === "'" || c === '"' || c === '`') { quote = c; cur += c; continue; }
    if (c === '(' || c === '[' || c === '{') depth += 1;
    if (c === ')' || c === ']' || c === '}') depth -= 1;
    if (depth === 0 && c === ';') { parts.push(cur); return parts; }
    if (depth === 0 && c === ',') { parts.push(cur); cur = ''; continue; }
    cur += c;
  }
  if (cur.trim() !== '') parts.push(cur);
  return parts;
}

// Parse `name = expr` bindings out of the text right after a
// `const ` keyword (handles comma-chained declarations).
function parseDeclBindings(afterConst) {
  const out = {};
  for (const piece of splitTopLevel(afterConst)) {
    const m = piece.match(/^\s*([A-Za-z_$][\w$]*)\s*=\s*([\s\S]+)$/);
    if (m) out[m[1]] = m[2].trim();
  }
  return out;
}

// Every `{ const …` block start in the file, with its bindings —
// candidate shot headers.
function findConstBlocks(tsText) {
  const blocks = [];
  const re = /\{\s*const\s+/g;
  let m;
  while ((m = re.exec(tsText)) !== null) {
    blocks.push({
      index: m.index,
      bindings: parseDeclBindings(tsText.slice(m.index + m[0].length)),
    });
  }
  return blocks;
}

// Shots + per-shot block facts. A shot's block runs from its
// `{ const st = …;` header (or the add() itself) to the next
// shot's header. (Block ends must come from the header list, not
// from a naive `{ const` scan: draw() bodies contain their own
// `{ const …` in callbacks, which would truncate the block
// before its shoot()/cue() lines.)
function extractShots(tsText, resolver, env) {
  const adds = [];
  let m;
  SHOT_ADD_RE.lastIndex = 0;
  while ((m = SHOT_ADD_RE.exec(tsText)) !== null) {
    adds.push({
      id: unescapeTsString(m[1]),
      atExpr: m[2].trim(),
      note: unescapeTsString(m[3]),
      index: m.index,
    });
  }
  const constBlocks = findConstBlocks(tsText);
  // header per shot: the last const block after the previous
  // shot's add() and before this add() — preferring one that
  // actually binds the variable the add() uses for `at`.
  const headers = adds.map((add, i) => {
    const atVar = /^[A-Za-z_$][\w$]*$/.test(add.atExpr) ? add.atExpr : null;
    const lo = i === 0 ? 0 : adds[i - 1].index + 1;
    const candidates = constBlocks.filter(
      (b) => b.index >= lo && b.index < add.index);
    if (candidates.length === 0) return null;
    if (atVar) {
      const binding = candidates.filter(
        (b) => Object.prototype.hasOwnProperty.call(b.bindings, atVar));
      if (binding.length > 0) return binding[binding.length - 1];
    }
    return candidates[candidates.length - 1];
  });
  const blockStarts = adds.map((add, i) => (headers[i] ? headers[i].index : add.index));

  const shots = adds.map((add, i) => {
    const header = headers[i];
    const blockStart = blockStarts[i];
    const blockEnd = i + 1 < adds.length ? blockStarts[i + 1] : tsText.length;
    const block = tsText.slice(blockStart, blockEnd);

    // Block-local bindings win over the file-global ones: every
    // shot block redeclares `st` (and anchors like `w0`), so the
    // globals collected file-wide are the LAST block's values.
    const shotEnv = { ...env };
    if (header) Object.assign(shotEnv, header.bindings);
    const blockConstRe = /const\s+/g;
    let lm;
    while ((lm = blockConstRe.exec(block)) !== null) {
      Object.assign(shotEnv,
        parseDeclBindings(block.slice(lm.index + lm[0].length)));
    }

    // resolve the start time through the block environment (the
    // header binding for `st` & friends is already merged in)
    const atS = resolver.evaluate(add.atExpr, shotEnv);
    if (atS !== null) shotEnv[add.atExpr] = String(atS);

    // first shoot() call: scene, literal camera endpoints, fov
    let set = null;
    let fov = null;
    let cameraEndpoints = null;
    let cameraDynamic = false;
    const shootAt = block.indexOf('shoot(');
    if (shootAt >= 0) {
      const callText = block.slice(shootAt, block.indexOf('\n', shootAt) < 0
        ? block.length : block.indexOf('\n', shootAt));
      const sceneM = callText.match(/([A-Za-z_$][\w$]*)\.scene/);
      if (sceneM) set = sceneM[1];
      const fovM = callText.match(/fov:\s*(\d+(?:\.\d+)?)/);
      if (fovM) fov = Number(fovM[1]);
      if (/rig\.|mix\(|gmaFront\(|\.clone\(\)/.test(callText)) {
        cameraDynamic = true;
      } else {
        const v3s = [...callText.matchAll(
          /V3\(\s*(-?[\d.]+)\s*,\s*(-?[\d.]+)\s*,\s*(-?[\d.]+)\s*\)/g)]
          .map((v) => [Number(v[1]), Number(v[2]), Number(v[3])]);
        if (v3s.length >= 3) {
          cameraEndpoints = {
            from: v3s[0], to: v3s[1], at: v3s[2],
            at2: v3s.length >= 4 ? v3s[3] : null,
          };
        } else if (v3s.length > 0) {
          cameraDynamic = true;
        }
      }
    }
    // grade / look reference: `...G.jungle`, `...LOOKS.street`,
    // or a bare `LOOKS.room` shoot argument
    let lookRef = null;
    const lookM = block.match(/\b(?:G|LOOKS)\.([A-Za-z_$][\w$]*)/);
    if (lookM) lookRef = lookM[1];
    // atmosphere presets applied inside the shot
    const atmospheres = [...block.matchAll(
      /applyAtmosphere\(\s*([A-Za-z_$][\w$]*)\.scene\s*,\s*'([^']+)'/g)]
      .map((a) => ({ set: a[1], preset: a[2] }));
    // staged visibility: jgStage({ onoda: true, rifle: true })
    let staged = null;
    const stageM = block.match(/[A-Za-z_$][\w$]*Stage\(\{([^}]*)\}\)/);
    if (stageM) {
      staged = [...stageM[1].matchAll(/([A-Za-z_$][\w$]*)\s*:\s*true/g)]
        .map((s) => s[1]);
    }
    // cues authored inside the shot block
    const cues = [];
    const cueRe = /cue\(\s*([^,]+?)\s*,\s*'((?:[^'\\]|\\.)*)'\s*(?:,\s*\{([^}]*)\})?\s*\)/g;
    let cm;
    while ((cm = cueRe.exec(block)) !== null) {
      const cue = {
        tExpr: cm[1].trim(),
        tS: resolver.evaluate(cm[1].trim(), shotEnv),
        type: unescapeTsString(cm[2]),
      };
      if (cm[3]) {
        const gainM = cm[3].match(/gain:\s*([\d.]+)/);
        const durM = cm[3].match(/dur:\s*([\d.]+)/);
        if (gainM) cue.gain = Number(gainM[1]);
        if (durM) cue.dur = Number(durM[1]);
      }
      cues.push(cue);
    }
    // narration line(s) this shot covers: from the banner comment
    // `── S07 · L6 "…"` when present, else from the resolved time
    const bannerM = tsText.slice(Math.max(0, blockStart - 300), blockStart)
      .match(/·\s*L(\d+)\s+"/);
    return {
      id: add.id,
      note: add.note,
      atExpr: header && header.bindings[add.atExpr]
        ? header.bindings[add.atExpr] : add.atExpr,
      atS,
      set,
      fov,
      lookRef,
      atmospheres,
      staged,
      cameraEndpoints,
      cameraDynamic,
      cues,
      bannerLine: bannerM ? Number(bannerM[1]) : null,
      blockBytes: block.length,
    };
  });
  return shots;
}

// ------------------------------------------------------------------
// main
// ------------------------------------------------------------------

function fail(problems) {
  for (const problem of problems) {
    console.error(`export_project_v2: error: ${problem}`);
  }
  console.error(
    `export_project_v2: ${problems.length} problem(s) — no output written`);
  process.exit(2);
}

function parseArgs(argv) {
  const positional = [];
  const opts = { skeleton: null, fps: null, width: null, height: null };
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    if (a === '--skeleton') opts.skeleton = argv[++i];
    else if (a === '--fps') opts.fps = Number(argv[++i]);
    else if (a === '--width') opts.width = Number(argv[++i]);
    else if (a === '--height') opts.height = Number(argv[++i]);
    else if (a.startsWith('--')) return { error: `unknown flag '${a}'` };
    else positional.push(a);
  }
  if (positional.length !== 2) {
    return {
      error: 'usage: node export_project_v2.mjs <project-dir> <out.json> ' +
        '[--skeleton <path>] [--fps N] [--width N] [--height N]',
    };
  }
  return { projectDir: positional[0], outPath: positional[1], opts };
}

function main() {
  const parsed = parseArgs(process.argv.slice(2));
  if (parsed.error) fail([parsed.error]);
  const { projectDir, outPath, opts } = parsed;
  const problems = [];
  const where = 'project';

  if (!fs.existsSync(projectDir) || !fs.statSync(projectDir).isDirectory()) {
    fail([`${where}: project dir not found or not a directory: '${projectDir}'`]);
  }

  // ---- script.json: meta + narration script ----------------------
  const script = readJsonFile(
    path.join(projectDir, 'script.json'), 'script.json', problems);
  let title = '';
  if (script !== undefined) {
    if (!isPlainObject(script)) {
      problems.push(`script.json: top-level value must be a JSON object, found ${typeName(script)}`);
    } else if (script.title === undefined) {
      problems.push(`script.json: missing required field 'title'`);
    } else if (typeof script.title !== 'string') {
      problems.push(`script.json: 'title' must be a string, found ${typeName(script.title)}`);
    } else {
      title = script.title;
    }
  }
  const scriptLines = isPlainObject(script) && Array.isArray(script.lines)
    ? script.lines : [];
  if (isPlainObject(script) && script.lines !== undefined && !Array.isArray(script.lines)) {
    problems.push(`script.json: 'lines' must be an array, found ${typeName(script.lines)}`);
  }

  // ---- data/lines.json + data/words.json: the film clock ---------
  const linesDoc = readJsonFile(
    path.join(projectDir, 'data', 'lines.json'), 'data/lines.json', problems);
  let lines = [];
  let durationS = null;
  if (linesDoc !== undefined) {
    if (!isPlainObject(linesDoc) || !Array.isArray(linesDoc.lines)) {
      problems.push(`data/lines.json: must be an object with a 'lines' array`);
    } else {
      lines = linesDoc.lines;
      lines.forEach((l, i) => {
        if (!isPlainObject(l) || typeof l.start !== 'number' ||
            typeof l.end !== 'number' || typeof l.text !== 'string') {
          problems.push(
            `data/lines.json: lines[${i}] must be an object with numeric start/end and string text`);
        }
      });
      if (typeof linesDoc.duration === 'number' && linesDoc.duration > 0) {
        durationS = linesDoc.duration;
      } else {
        problems.push(`data/lines.json: 'duration' must be a positive number`);
      }
    }
  }
  if (scriptLines.length > 0 && lines.length > 0 &&
      scriptLines.length !== lines.length) {
    problems.push(
      `project: script.json has ${scriptLines.length} lines but ` +
      `data/lines.json has ${lines.length} — the script and its timing ` +
      `disagree (re-run voice.py / words.py)`);
  }
  const wordsDoc = readJsonFile(
    path.join(projectDir, 'data', 'words.json'), 'data/words.json', problems);
  let words = [];
  if (wordsDoc !== undefined) {
    if (!Array.isArray(wordsDoc)) {
      problems.push(`data/words.json: must be an array of { w, line, s, e }`);
    } else {
      words = wordsDoc;
    }
  }

  // ---- data/cues.json (optional: authored cue events) ------------
  let fileCues = [];
  const cuesPath = path.join(projectDir, 'data', 'cues.json');
  if (fs.existsSync(cuesPath)) {
    const cuesDoc = readJsonFile(cuesPath, 'data/cues.json', problems);
    if (cuesDoc !== undefined) {
      if (!isPlainObject(cuesDoc) || !Array.isArray(cuesDoc.cues)) {
        problems.push(`data/cues.json: must be an object with a 'cues' array`);
      } else {
        fileCues = cuesDoc.cues;
      }
    }
  }

  // ---- src/shots.ts: static extraction ---------------------------
  const shotsTs = readTextFile(
    path.join(projectDir, 'src', 'shots.ts'), 'src/shots.ts', problems);
  const resolver = makeResolver(lines, words);
  const env = shotsTs !== undefined ? collectBindings(shotsTs) : {};
  const cast = shotsTs !== undefined ? extractCast(shotsTs) : [];
  const instances = shotsTs !== undefined ? extractInstances(shotsTs) : [];
  const sets = shotsTs !== undefined ? extractSets(shotsTs) : [];
  const rawShots = shotsTs !== undefined ? extractShots(shotsTs, resolver, env) : [];

  if (shotsTs !== undefined && rawShots.length === 0) {
    problems.push(
      `src/shots.ts: no shots extracted (no add({ id, at, note }) blocks ` +
      `matched) — the staging source does not follow the skill's shot shape`);
  }
  const seenShotIds = new Set();
  for (const s of rawShots) {
    if (seenShotIds.has(s.id)) {
      problems.push(`src/shots.ts: duplicate shot id '${s.id}'`);
    }
    seenShotIds.add(s.id);
  }
  let prevAt = null;
  for (const s of rawShots) {
    if (s.atS !== null && prevAt !== null && s.atS < prevAt - 1e-9) {
      problems.push(
        `src/shots.ts: shot '${s.id}' starts at ${s.atS}s, before the ` +
        `previous shot's ${prevAt}s — shot times must not run backwards`);
    }
    if (s.atS !== null) prevAt = s.atS;
  }

  // shot end times + line coverage
  const shots = rawShots.map((s, i) => {
    const endS = i + 1 < rawShots.length ? rawShots[i + 1].atS : durationS;
    const lineIndices = [];
    if (s.atS !== null && endS !== null) {
      lines.forEach((l, li) => {
        if (typeof l.start === 'number' && l.start >= s.atS - 0.06 - 1e-9 &&
            l.start < endS - 1e-9) lineIndices.push(li);
      });
      // a shot that starts mid-line (the S05b pattern) covers the
      // line containing its start even though that line began in
      // the previous shot
      let containing = -1;
      lines.forEach((l, li) => {
        if (typeof l.start === 'number' && typeof l.end === 'number' &&
            l.start <= s.atS + 1e-9 && s.atS < l.end) {
          containing = li;
        }
      });
      if (containing >= 0 && !lineIndices.includes(containing)) {
        lineIndices.unshift(containing);
      }
    }
    if (lineIndices.length === 0 && s.bannerLine !== null) {
      lineIndices.push(s.bannerLine);
    }
    return { ...s, endS, lineIndices };
  });

  // ---- meta: fps / width / height resolution ---------------------
  const metaSource = {};
  const resolveMeta = (field, cliVal, libKey, filmVal) => {
    if (cliVal !== null && cliVal !== undefined) {
      metaSource[field] = 'cli';
      return cliVal;
    }
    if (libKey !== null && libKey !== undefined) {
      metaSource[field] = 'src/core/lib.ts';
      return libKey;
    }
    if (filmVal !== null && filmVal !== undefined) {
      metaSource[field] = 'native/film.json';
      return filmVal;
    }
    metaSource[field] = 'skill default';
    return DEFAULTS[field];
  };
  let libConsts = {};
  const libPath = path.join(projectDir, 'src', 'core', 'lib.ts');
  if (fs.existsSync(libPath)) {
    const libText = fs.readFileSync(libPath, 'utf8');
    const grab = (name) => {
      // both spellings occur: `export const W = 1080, H = 1920,
      // FPS = 30;` (one chained declaration) and separate lines
      const m = libText.match(
        new RegExp(`(?:export const\\s+|[,;]\\s*)${name}\\s*=\\s*(\\d+)`));
      return m ? Number(m[1]) : null;
    };
    libConsts = { W: grab('W'), H: grab('H'), FPS: grab('FPS') };
  }
  let filmMeta = {};
  const filmJsonPath = path.join(projectDir, 'native', 'film.json');
  if (fs.existsSync(filmJsonPath)) {
    try {
      const fj = JSON.parse(fs.readFileSync(filmJsonPath, 'utf8'));
      filmMeta = {
        fps: typeof fj.fps === 'number' ? fj.fps : null,
        width: fj.resolution && typeof fj.resolution.width === 'number'
          ? fj.resolution.width : null,
        height: fj.resolution && typeof fj.resolution.height === 'number'
          ? fj.resolution.height : null,
      };
    } catch { /* a malformed bridge manifest is the bridge's problem */ }
  }
  const fps = resolveMeta('fps', opts.fps, libConsts.FPS, filmMeta.fps);
  const width = resolveMeta('width', opts.width, libConsts.W, filmMeta.width);
  const height = resolveMeta('height', opts.height, libConsts.H, filmMeta.height);
  for (const [field, v] of [['fps', fps], ['width', width], ['height', height]]) {
    if (!isPosU32(v)) {
      problems.push(
        `${where}: 'meta.${field}' must be a positive integer (> 0), found ${v} ` +
        `(source: ${metaSource[field]})`);
    }
  }

  // ---- assets: manifest + C4 validation + existence --------------
  const assets = collectAssets(projectDir);
  const existenceChecks = [];
  assets.forEach((asset, i) => {
    if (typeof asset.id !== 'string' || asset.id.length === 0) {
      problems.push(`${where}: assets[${i}]: 'id' must be a non-empty string`);
    }
    if (typeof asset.path !== 'string' || asset.path.length === 0) {
      problems.push(`${where}: assets[${i}]: 'path' must be a non-empty string`);
    }
    if (!ASSET_KINDS.has(asset.kind)) {
      problems.push(
        `${where}: assets[${i}] (id '${asset.id}'): unknown asset kind ` +
        `'${asset.kind}' — expected one of "image", "audio", "data"`);
    }
    if (checkAssetPath(asset.id, asset.path, problems, where)) {
      existenceChecks.push(asset);
    }
  });
  for (const check of existenceChecks) {
    if (!fs.existsSync(path.join(projectDir, check.path))) {
      problems.push(
        `${where}: missing asset '${check.path}' (id '${check.id}') ` +
        `not found under '${projectDir}'`);
    }
  }

  // ---- sections (assembled document shape) -----------------------
  const characters = cast.map((c) => ({ id: c.id, look: c.look }));
  const scenes = sets.map((s) => ({ id: s.id, label: s.label, kind: s.kind }));
  const shotsSection = shots.map((s) => ({
    id: s.id,
    note: s.note,
    lines: s.lineIndices,
    atS: s.atS,
    endS: s.endS,
    durationS: s.atS !== null && s.endS !== null
      ? +(s.endS - s.atS).toFixed(3) : null,
    set: s.set,
    fov: s.fov,
  }));
  const captions = lines.map((l, i) => ({
    line: i,
    text: l.text,
    caps: scriptLines[i] && typeof scriptLines[i].caps === 'string'
      ? scriptLines[i].caps : null,
    start: l.start,
    end: l.end,
    words: words.filter((w) => w.line === i).length,
  }));
  const audio = [];
  for (const track of ['narration', 'mix']) {
    const rel = `data/${track}.wav`;
    if (fs.existsSync(path.join(projectDir, rel))) {
      audio.push({ type: 'track', id: track, path: rel });
    }
  }
  for (const c of fileCues) {
    // the cue's own `type` (chirp/whoosh/…) is renamed cueType so
    // the entry discriminator `type: 'cue'` is never clobbered
    if (isPlainObject(c)) {
      const { type, ...rest } = c;
      audio.push({ type: 'cue', cueType: type ?? null, ...rest });
    } else {
      audio.push({ type: 'cue', cueType: null, raw: c });
    }
  }
  const sections = {
    characters, scenes, shots: shotsSection, captions, audio,
  };
  for (const name of SECTION_NAMES) {
    if (!Array.isArray(sections[name])) {
      problems.push(`${where}: section '${name}' must be an array`);
    }
  }

  if (problems.length > 0) fail(problems);

  // ---- interchange document --------------------------------------
  const doc = {
    format: SUPPORTED_FORMAT,
    meta: { title, fps, width, height },
    assets,
    characters,
    scenes,
    shots: shotsSection,
    captions,
    audio,
  };

  // ---- segment skeleton report (the bridge builder's input) ------
  const totalFrames = durationS !== null ? Math.ceil(durationS * fps) : null;
  const owes = [
    {
      item: 'draw-body-animation',
      severity: 'blocking',
      detail: 'Per-frame pose, motion, blink, lip-sync and prop ' +
        'animation inside each shot\'s draw() body is code, not data; ' +
        'nothing in this report transcribes it. The bridge builder ' +
        'must author §15 scene/clip data per segment from shots.ts.',
      neededBy: 'tools/bridge (scene authoring)',
    },
    {
      item: 'scene-geometry',
      severity: 'blocking',
      detail: 'Full 3D set construction (meshes, materials, lighting, ' +
        'world/props library calls, project-local builders and canvas ' +
        'props) is extracted only as set identity (id/label/kind). ' +
        'Geometry must be re-authored as native scene data.',
      neededBy: 'tools/bridge (scene authoring)',
    },
    {
      item: 'camera-motion',
      severity: 'partial',
      detail: 'Only literal V3 endpoints + fov are extracted. Easing ' +
        'curves (camK / eio), dynamic endpoints (rig/mix/helper ' +
        'expressions — flagged cameraDynamic per shot) and focus ' +
        'pulls are runtime values and are still owed.',
      neededBy: 'tools/bridge (clip authoring)',
    },
    {
      item: 'caption-strips',
      severity: 'blocking',
      detail: 'Caption strip PNGs and their width/fit measurements ' +
        'are produced by canvas text rendering (core/captions.ts ' +
        'buildCaptions). The exporter carries caption text + word ' +
        'timing only; strips must be rendered bridge-side.',
      neededBy: 'tools/bridge (film data)',
    },
    {
      item: 'word-anchors',
      severity: 'partial',
      detail: 'Named word anchors (film data anchors{}) are ' +
        'hand-named by the bridge author. Full word timings are in ' +
        'data/words.json (an asset of the interchange document); ' +
        'anchor naming is not derivable from them.',
      neededBy: 'tools/bridge (film data)',
    },
    {
      item: 'atmosphere-grade-resolution',
      severity: 'partial',
      detail: 'applyAtmosphere preset NAMES are extracted per shot; ' +
        'the concrete grade numbers a preset resolves to are ' +
        'runtime values from core/env.ts and are not transcribed.',
      neededBy: 'tools/bridge (film look)',
    },
  ];
  const unresolved = shots.filter((s) => s.atS === null);
  if (unresolved.length > 0) {
    owes.push({
      item: 'unresolved-shot-times',
      severity: 'blocking',
      detail: 'Shot start expressions the static resolver could not ' +
        'evaluate: ' + unresolved.map((s) => `${s.id} (at: ${s.atExpr})`).join(', ') +
        '. Their atS/endS/frames are null in this report; the bridge ' +
        'must place them by hand.',
      neededBy: 'tools/bridge (segment tiling)',
    });
  }
  const unresolvedCues = shots.flatMap((s) => s.cues.filter((c) => c.tS === null)
    .map((c) => `${s.id}:${c.type} (t: ${c.tExpr})`));
  if (unresolvedCues.length > 0) {
    owes.push({
      item: 'unresolved-cue-times',
      severity: 'partial',
      detail: 'Cue time expressions the static resolver could not ' +
        'evaluate (loop offsets and indirect anchors): ' +
        unresolvedCues.join(', ') + '. The authored data/cues.json ' +
        '(when present) remains the cue record of authority.',
      neededBy: 'tools/bridge (film data)',
    });
  }
  if (shots.some((s) => s.lineIndices.length > 1) ||
      new Set(shots.flatMap((s) => s.lineIndices)).size < lines.length) {
    owes.push({
      item: 'shot-segment-mapping',
      severity: 'partial',
      detail: 'Shots and native segments are not always 1:1 — one ' +
        'narration line can be split across two shots (the S05a/S05b ' +
        'pattern) and a shot can span a line boundary. The bridge ' +
        'must fix final segment boundaries; this report gives the ' +
        'shot tiling and per-shot line coverage to decide from.',
      neededBy: 'tools/bridge (segment tiling)',
    });
  }
  if (!fs.existsSync(path.join(projectDir, 'native', 'film.json'))) {
    owes.push({
      item: 'native-package-absent',
      severity: 'blocking',
      detail: 'This project carries no native/ bridge package. ' +
        'package_bridge.py only packages pre-authored repo data; ' +
        'creating film.json + scene files + film data for THIS ' +
        'project from this skeleton is the bridge builder\'s job.',
      neededBy: 'tools/bridge (packaging)',
    });
  }

  const skeleton = {
    format: SKELETON_FORMAT,
    project: {
      dir: path.basename(path.resolve(projectDir)),
      title,
    },
    timing: {
      fps, width, height,
      durationS,
      totalFrames,
      lead: isPlainObject(script) && typeof script.lead === 'number' ? script.lead : null,
      gap: isPlainObject(script) && typeof script.gap === 'number' ? script.gap : null,
      outro: isPlainObject(script) && typeof script.outro === 'number' ? script.outro : null,
      voice: isPlainObject(script) && typeof script.voice === 'string' ? script.voice : null,
      speed: isPlainObject(script) && typeof script.speed === 'number' ? script.speed : null,
      music: isPlainObject(script) && typeof script.music === 'string' ? script.music : null,
      metaSource,
      lines: lines.length,
      words: words.length,
    },
    cast,
    instances,
    sets,
    segments: shots.map((s) => ({
      shotId: s.id,
      note: s.note,
      lines: s.lineIndices,
      atExpr: s.atExpr,
      atS: s.atS,
      endS: s.endS,
      durationS: s.atS !== null && s.endS !== null
        ? +(s.endS - s.atS).toFixed(3) : null,
      startFrame: s.atS !== null ? Math.ceil(s.atS * fps) : null,
      endFrame: s.endS !== null ? Math.ceil(s.endS * fps) : null,
      set: s.set,
      fov: s.fov,
      lookRef: s.lookRef,
      atmospheres: s.atmospheres,
      staged: s.staged,
      camera: s.cameraEndpoints,
      cameraDynamic: s.cameraDynamic,
      cues: s.cues,
    })),
    cuesFromFile: fileCues,
    extracted: [
      'meta (script.json title; fps/width/height per metaSource)',
      'line timings + duration (data/lines.json)',
      'word timings count (data/words.json; full data rides as an asset)',
      'cast LOOK registry (src/shots.ts, static)',
      'person() instances + scene membership (src/shots.ts, static)',
      'set identities (src/shots.ts, static)',
      'shot ids/notes/start expressions + resolved start/end/frames',
      'per-shot set, fov, look reference, atmosphere presets, staged visibility',
      'literal camera endpoints where the shoot() call is fully literal',
      'authored cue calls per shot (times resolved where the grammar allows)',
      'asset manifest (curated project-tree walk)',
    ],
    owes,
    assumptions: [
      'Shot start expressions follow the timing.ts grammar ' +
        '(cut(i) = L(i) - 0.06, 0 for i = 0); a project that ' +
        'redefines cut() in shots.ts invalidates resolved times — ' +
        'they are reported with their source expression so the ' +
        'bridge can check.',
      'One add() block = one shot, in file order = time order ' +
        '(main.ts sorts shots by `at` at boot; the exporter ' +
        'validates the resolved order does not run backwards).',
      'Sections of the interchange document are envelope data for ' +
        'projectfile.rs (which counts but does not deep-validate ' +
        'entries); their per-entry shape here is the bridge ' +
        'contract documented in CR-B1-07-EXPORT.md.',
      'The skeleton is an input to a FUTURE general bridge ' +
        'builder; package_bridge.py as shipped packages only the ' +
        'pre-authored Onoda repo data and does not consume it yet.',
    ],
  };

  // ---- write both outputs (only after every check passed) --------
  const skeletonPath = opts.skeleton ||
    outPath.replace(/\.json$/, '') + '.skeleton.json';
  for (const [p, data] of [[outPath, doc], [skeletonPath, skeleton]]) {
    fs.mkdirSync(path.dirname(path.resolve(p)), { recursive: true });
    fs.writeFileSync(p, JSON.stringify(data, null, 2) + '\n', 'utf8');
  }
  const counts = SECTION_NAMES.map((n) => `${n}=${sections[n].length}`).join(' ');
  console.log(
    `exported ${JSON.stringify(title)}: assets=${assets.length} ${counts} ` +
    `segments=${shots.length} owes=${owes.length} -> ${outPath} + ${skeletonPath}`);
}

main();
