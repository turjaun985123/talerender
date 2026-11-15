// =====================================================================
// DELIVERY NOTE — TALERENDER V2, Batch 2, Seat 5: WP5 TypeScript-side
// export pipeline (Node tool). Contract C4
// (docs/integration/V2-CONTRACTS.md §3), master prompt §9 (WP5).
//
// FILES CREATED (this seat, NEW files only, under tools/export/)
//   tools/export/export_project.mjs          — this file, the exporter.
//   tools/export/export_project.selftest.mjs — hermetic self-test for
//                                              it (temp-dir fixture,
//                                              child-process runs).
//   No existing file was edited, no git command was run, and there
//   are NO npm dependencies: Node >= 18, `node:` builtins only.
//
// CONTRACT POINTS IMPLEMENTED (C4 — the EMIT side of the schema whose
// Rust counterpart is src/projectfile.rs, Batch 1 seat B1-5)
//   * Emitted document is exactly the C4 shape projectfile.rs loads:
//       { "format": "talerender-project/1",
//         "meta": { "title": "...", "fps": 30,
//                   "width": 1080, "height": 1920 },
//         "assets": [ { "id": "...", "path": "relative/path",
//                       "kind": "image|audio|data" } ],
//         "characters": [ ... ], "scenes": [ ... ],
//         "shots": [ ... ], "captions": [ ... ], "audio": [ ... ] }
//     A document this tool writes on success passes
//     `projectfile::load_project`; a source this tool rejects is
//     never written out as an interchange document.
//   * Validation mirrors projectfile.rs, rule for rule, in the same
//     spirit (all failures are data-load class, exit code 2):
//       - meta.title must be a string (it MAY be empty — projectfile
//         imposes no non-emptiness rule, and neither does this tool);
//       - meta fps / width / height (flat in the source, see INPUT
//         CONTRACT) must be positive integers (> 0, integer-valued,
//         fitting u32) — zero, negative, fractional, non-numeric and
//         missing values are all problems naming the field;
//       - asset `kind` is the closed set image | audio | data; an
//         unknown kind is a problem naming the asset and the kind —
//         never a silent default;
//       - absent `assets` = empty manifest; present-but-not-an-array,
//         non-object entries, and missing / empty / non-string `id`
//         or `path` are problems;
//       - asset paths are relative only: a leading `/` or `\`, a
//         Windows drive prefix such as `C:\`, or an absolute path in
//         either POSIX or Windows spelling is rejected; ANY `..`
//         path component (split on both `/` and `\`) is rejected as
//         a path escape naming the asset id and the path;
//       - a present section that is neither an array nor a count
//         (see INPUT CONTRACT) is a problem naming the section — a
//         mistyped section is never silently counted as empty.
//   * ALL problems are collected and listed together (stderr, one
//     per line) before exiting 2 — never just the first. This is
//     deliberately STRONGER than projectfile.rs, which returns on
//     its first error: an exporter is an authoring tool, so the
//     author gets the whole list in one run. The SET of rules is
//     the same; only the reporting is batched.
//   * Missing assets (C4: EVERY missing asset, never a silent skip):
//     every asset whose (valid, relative, non-escaping) path does
//     not exist under <source-dir> is listed by its authored
//     relative path, and the run exits 2 without writing output.
//     Assets whose path already failed the absolute / `..` rules
//     are not existence-checked (the check would leave the root).
//   * On success: the interchange JSON is written (2-space indent,
//     trailing newline) and exactly ONE summary line is printed to
//     stdout: title, asset count, and the five section counts.
//
// INPUT CONTRACT (this tool's own manifest — NOT a guess at the
// Tall Tale skill's internals; real skill projects carry e.g.
// script.json, data/cues.json, native/film/… — an adapter that
// produces this manifest from a skill project is a separate,
// project-side step and is out of scope for this seat)
//   <source-dir>/project-source.json:
//     { "title": "...", "fps": 30, "width": 1080, "height": 1920,
//       "assets": [ { "id": "...", "path": "relative/path",
//                     "kind": "image|audio|data" } ],
//       "sections": { "characters": <n | array>,
//                     "scenes":     <n | array>,
//                     "shots":      <n | array>,
//                     "captions":   <n | array>,
//                     "audio":      <n | array> } }
//   - `assets` may be absent (= empty). `sections` may be absent
//     (= all five sections empty). A section may be absent
//     (= empty). Unknown extra keys are ignored, mirroring
//     projectfile.rs, which ignores unknown fields.
//   - A section given as an ARRAY is passed through verbatim into
//     the emitted document (entries are opaque here, exactly as
//     projectfile.rs counts but does not deep-validate them).
//   - A section given as a COUNT n becomes an array of n empty
//     objects {} in the emitted document. LOSSY CORNER, documented
//     honestly: a count carries no content, so the emitted entries
//     are placeholders with no data — {} and nothing else; no
//     content is invented, and none can be recovered downstream.
//     Prefer arrays in the source whenever the entries matter.
//   - CLI: node export_project.mjs <source-dir> <out.json>
//     Exit codes: 0 = written; 2 = usage error, unreadable /
//     malformed project-source.json, any validation problem, or
//     any missing asset (all problems listed on stderr).
//
// WIRING THE LEAD MUST DO
//   * None in Rust: this seat adds nothing to src/, Cargo.toml or
//     any existing file, and projectfile.rs needs no change — it
//     already loads what this tool emits. The tool is run
//     standalone by the authoring pipeline:
//         node tools/export/export_project.mjs <source-dir> <out.json>
//   * Optional check: `node tools/export/export_project.selftest.mjs`
//     (exit 0 = all assertions passed).
//
// KNOWN LIMITATIONS
//   * Section ENTRIES are not validated (pass-through / counts
//     only — the §15 step-2 loader's job, as in projectfile.rs).
//   * Duplicate asset `id`s are not rejected: C4 / projectfile.rs
//     impose no uniqueness rule, and inventing one could reject
//     documents the Rust loader accepts.
//   * Asset existence is existence only (fs.existsSync): a
//     directory at an asset path counts as present, mirroring
//     `Path::exists` in projectfile::validate_assets; readability /
//     decodability is the consuming loader's job.
//   * Asset entries are emitted as exactly { id, path, kind }:
//     extra per-asset keys in the source are dropped, since the
//     C4 schema carries only those three fields.
// =====================================================================

import * as fs from 'node:fs';
import * as path from 'node:path';

const SUPPORTED_FORMAT = 'talerender-project/1';
const ASSET_KINDS = new Set(['image', 'audio', 'data']);
const SECTION_NAMES = ['characters', 'scenes', 'shots', 'captions', 'audio'];
const U32_MAX = 4294967295;

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

// A meta dimension field (fps / width / height), projectfile.rs
// `meta_u32` rule: required, numeric, integer-valued, > 0, fits u32.
function checkMetaField(src, field, problems) {
  const v = src[field];
  if (v === undefined) {
    problems.push(`project-source: missing required field '${field}'`);
    return 0;
  }
  if (typeof v !== 'number') {
    problems.push(
      `project-source: '${field}' must be a number, found ${typeName(v)}`);
    return 0;
  }
  if (!Number.isInteger(v) || v <= 0 || v > U32_MAX) {
    problems.push(
      `project-source: '${field}' must be a positive integer (> 0), found ${v}`);
    return 0;
  }
  return v;
}

// C4 path law for one asset, projectfile.rs `validate_asset_path`
// rule: relative only, and no `..` component. Both separator
// spellings are scanned so the check is authoring-platform
// independent. Returns true when the path is usable (relative and
// non-escaping), i.e. safe to existence-check under the root.
function checkAssetPath(id, p, problems) {
  const drivePrefixed = /^[A-Za-z]:/.test(p);
  if (p.startsWith('/') || p.startsWith('\\') || drivePrefixed ||
      path.posix.isAbsolute(p) || path.win32.isAbsolute(p)) {
    problems.push(
      `project-source: asset '${id}': absolute path rejected: '${p}' ` +
      '— asset paths must be relative to the project root');
    return false;
  }
  if (p.split(/[\\/]/).some((component) => component === '..')) {
    problems.push(
      `project-source: asset '${id}': path escape rejected: '${p}' ` +
      `contains a '..' component — asset paths must not escape the project root`);
    return false;
  }
  return true;
}

function fail(problems) {
  for (const problem of problems) {
    console.error(`export_project: error: ${problem}`);
  }
  console.error(
    `export_project: ${problems.length} problem(s) — no output written`);
  process.exit(2);
}

function main() {
  const args = process.argv.slice(2);
  if (args.length !== 2) {
    console.error(
      'usage: node export_project.mjs <source-dir> <out.json>');
    process.exit(2);
  }
  const [sourceDir, outPath] = args;
  const problems = [];

  // ---- read the source manifest --------------------------------
  const sourcePath = path.join(sourceDir, 'project-source.json');
  let text;
  try {
    text = fs.readFileSync(sourcePath, 'utf8');
  } catch (e) {
    fail([`project-source: cannot read '${sourcePath}': ${e.message}`]);
  }
  let src;
  try {
    src = JSON.parse(text);
  } catch (e) {
    fail([`project-source: invalid JSON in '${sourcePath}': ${e.message}`]);
  }
  if (!isPlainObject(src)) {
    fail([`project-source: top-level value must be a JSON object, ` +
          `found ${typeName(src)}`]);
  }

  // ---- meta (flat in the source; nested `meta` in the output) --
  let title = '';
  if (src.title === undefined) {
    problems.push(`project-source: missing required field 'title'`);
  } else if (typeof src.title !== 'string') {
    problems.push(
      `project-source: 'title' must be a string, found ${typeName(src.title)}`);
  } else {
    title = src.title;
  }
  const fps = checkMetaField(src, 'fps', problems);
  const width = checkMetaField(src, 'width', problems);
  const height = checkMetaField(src, 'height', problems);

  // ---- assets ----------------------------------------------------
  const assets = [];
  const existenceChecks = []; // { id, path } with a usable path
  if (src.assets !== undefined) {
    if (!Array.isArray(src.assets)) {
      problems.push(
        `project-source: 'assets' must be an array, found ${typeName(src.assets)}`);
    } else {
      src.assets.forEach((item, i) => {
        if (!isPlainObject(item)) {
          problems.push(
            `project-source: assets[${i}] must be an object with id, path ` +
            `and kind, found ${typeName(item)}`);
          return;
        }
        const idOk = typeof item.id === 'string' && item.id.length > 0;
        if (!idOk) {
          problems.push(
            `project-source: assets[${i}]: 'id' must be a non-empty string`);
        }
        const idLabel = idOk ? item.id : `#${i}`;
        const pathOk = typeof item.path === 'string' && item.path.length > 0;
        if (!pathOk) {
          problems.push(
            `project-source: assets[${i}] (id '${idLabel}'): 'path' must be ` +
            'a non-empty string');
        }
        let kindOk = false;
        if (typeof item.kind !== 'string') {
          problems.push(
            `project-source: assets[${i}] (id '${idLabel}'): 'kind' must be ` +
            'a string ("image", "audio" or "data")');
        } else if (!ASSET_KINDS.has(item.kind)) {
          problems.push(
            `project-source: assets[${i}] (id '${idLabel}'): unknown asset ` +
            `kind '${item.kind}' — expected one of "image", "audio", "data"`);
        } else {
          kindOk = true;
        }
        let pathUsable = false;
        if (pathOk) {
          pathUsable = checkAssetPath(idLabel, item.path, problems);
        }
        if (idOk && pathOk && kindOk) {
          assets.push({ id: item.id, path: item.path, kind: item.kind });
          if (pathUsable) {
            existenceChecks.push({ id: item.id, path: item.path });
          }
        }
      });
    }
  }

  // ---- sections --------------------------------------------------
  // Array => pass through verbatim. Count n => n empty objects {}
  // (the documented lossy corner in the DELIVERY NOTE header).
  const sections = {};
  for (const name of SECTION_NAMES) sections[name] = [];
  if (src.sections !== undefined) {
    if (!isPlainObject(src.sections)) {
      problems.push(
        `project-source: 'sections' must be an object, found ${typeName(src.sections)}`);
    } else {
      for (const name of SECTION_NAMES) {
        const v = src.sections[name];
        if (v === undefined) continue; // absent section = empty
        if (Array.isArray(v)) {
          sections[name] = v;
        } else if (typeof v === 'number') {
          if (!Number.isInteger(v) || v < 0) {
            problems.push(
              `project-source: section '${name}': count must be a ` +
              `non-negative integer, found ${v}`);
          } else {
            sections[name] = Array.from({ length: v }, () => ({}));
          }
        } else {
          problems.push(
            `project-source: section '${name}' must be an array or a ` +
            `count (non-negative integer), found ${typeName(v)}`);
        }
      }
    }
  }

  // ---- missing assets: EVERY one, never a silent skip ------------
  for (const check of existenceChecks) {
    if (!fs.existsSync(path.join(sourceDir, check.path))) {
      problems.push(
        `project-source: missing asset '${check.path}' (id '${check.id}') ` +
        `not found under '${sourceDir}'`);
    }
  }

  if (problems.length > 0) fail(problems);

  // ---- emit the C4 interchange document --------------------------
  const doc = {
    format: SUPPORTED_FORMAT,
    meta: { title, fps, width, height },
    assets,
    characters: sections.characters,
    scenes: sections.scenes,
    shots: sections.shots,
    captions: sections.captions,
    audio: sections.audio,
  };
  const parent = path.dirname(path.resolve(outPath));
  fs.mkdirSync(parent, { recursive: true });
  fs.writeFileSync(outPath, JSON.stringify(doc, null, 2) + '\n', 'utf8');

  const counts = SECTION_NAMES.map((n) => `${n}=${sections[n].length}`).join(' ');
  console.log(
    `exported ${JSON.stringify(title)}: assets=${assets.length} ${counts} -> ${outPath}`);
}

main();
