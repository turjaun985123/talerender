// render_meta.mjs — WP9 proposal P3: machine-readable render-metadata sidecar.
// CHROME-REPLACEMENT WAVE, Batch 1, Seat 10 (CR-B1-10).
//
// When the native adapter finishes a `video` render, provenance today lives
// only on stdout (binary path, gate verdict, encode summary — see
// V2-WP9-INTEGRATION.md §8/P3). This module turns the adapter's run context
// into `<out>.render.json`, written beside the video output:
//
//   renders/out.mp4  ->  renders/out.mp4.render.json
//
// (The sidecar name is the full output filename + ".render.json", exactly as
// P3 spells it — `<out>.render.json` — not a stem swap, so two outputs that
// differ only in extension never collide.)
//
// Node builtins only. No behaviour of capture_native.mjs changes until the
// lead wires this in (see docs/integration/CR-B1-10-ADAPTER.md in the
// TaleRender repo): in videoMode, after the `.part` -> final rename succeeds
// and ONLY then, call writeRenderMeta(ctx, out).
//
// Context shape (nested preferred; the flat aliases exist so the lead can
// pass the adapter's existing variables without reshaping them):
//
//   {
//     backend: 'native',                       // or ctx.backendName
//     binary: { path, via, version },          // or ctx.bin / ctx.binaryPath,
//                                              //    ctx.via / ctx.discoveryRoute,
//                                              //    ctx.binaryVersion / ctx.versionOutput
//     manifest: { path, sha256 },              // or ctx.manifestPath / ctx.manifestSha256
//     mode: 'video',
//     frameRange: { from, to, count, fps },    // `to` is exclusive, as in the
//                                              //    adapter's [n0, n0+n) ranges;
//                                              // or ctx.frames / ctx.range /
//                                              //    ctx.n0 + ctx.n (+ ctx.fps)
//     encode: { draft, preset, crf, ... },     // or ctx.encodeSettings / ctx.draft
//     gate: { verdict: 'CHECK OK' },           // or ctx.gateVerdict (string)
//     resolution: { width, height },           // optional; or ctx.W / ctx.H
//     durationS, hasMix,                       // optional, recorded when known
//     generatedAt,                             // optional ISO string; defaults
//                                              //    to now — inject in tests
//   }
//
// Honesty rules, matching the adapter's own discipline:
//   - A field the caller did not supply is recorded as null (or omitted from
//     a sub-object), NEVER invented. In particular the binary `--version`
//     output is recorded verbatim (trimmed) when supplied and null when not
//     — this module does not run the binary itself.
//   - The manifest sha256 IS computed here when a manifest path is supplied
//     without a hash and the file can be read: a hash of a file on disk is a
//     measurement, not an invention. If the file cannot be read, sha256 is
//     null and `manifest.sha256Error` says why.
//   - The gate verdict is normalised to 'CHECK OK' / 'CHECK FAILED' only
//     when the supplied value clearly says one of those; anything else is
//     recorded verbatim under gate.verdict and gate.recognised=false.
import crypto from 'node:crypto';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { pathToFileURL } from 'node:url';

export const RENDER_META_FORMAT = 'talerender-render-meta';
export const RENDER_META_VERSION = 1;

// The sidecar path for a video output: "<out>.render.json".
export function sidecarPathFor(outPath) {
  if (!outPath || typeof outPath !== 'string') throw new Error('render-meta: an output path is required to name the sidecar');
  return `${outPath}.render.json`;
}

function sha256File(p) {
  const h = crypto.createHash('sha256');
  h.update(fs.readFileSync(p));
  return h.digest('hex');
}

function pick(...vals) {
  for (const v of vals) if (v !== undefined && v !== null && v !== '') return v;
  return null;
}

function normGate(raw) {
  if (raw === undefined || raw === null || raw === '') return { verdict: null, recognised: false };
  if (typeof raw === 'object') {
    // Accept { verdict } / { ok: true|false } / { line: 'FILM TELEMETRY: ...' }.
    if (typeof raw.ok === 'boolean') return { verdict: raw.ok ? 'CHECK OK' : 'CHECK FAILED', recognised: true };
    return normGate(pick(raw.verdict, raw.line, raw.status));
  }
  const s = String(raw).trim();
  if (/CHECK OK/.test(s)) return { verdict: 'CHECK OK', recognised: true };
  if (/CHECK FAILED/.test(s)) return { verdict: 'CHECK FAILED', recognised: true };
  return { verdict: s, recognised: false };
}

function normFrameRange(ctx) {
  const fr = ctx.frameRange || ctx.frames || ctx.range || null;
  if (fr && typeof fr === 'object') {
    const from = pick(fr.from, fr.start, fr.n0);
    const count = pick(fr.count, fr.n);
    let to = pick(fr.to, fr.end);
    if (to === null && from !== null && count !== null) to = from + count;
    return {
      from: from === null ? null : Number(from),
      to: to === null ? null : Number(to),
      count: count !== null ? Number(count) : (from !== null && to !== null ? Number(to) - Number(from) : null),
      fps: pick(fr.fps, ctx.fps, ctx.manifest?.fps) === null ? null : Number(pick(fr.fps, ctx.fps, ctx.manifest?.fps)),
    };
  }
  if (ctx.n0 !== undefined || ctx.n !== undefined) {
    const from = ctx.n0 === undefined ? null : Number(ctx.n0);
    const count = ctx.n === undefined ? null : Number(ctx.n);
    return { from, to: from !== null && count !== null ? from + count : null, count, fps: ctx.fps ? Number(ctx.fps) : null };
  }
  return { from: null, to: null, count: null, fps: ctx.fps ? Number(ctx.fps) : null };
}

function normEncode(ctx) {
  const e = ctx.encode || ctx.encodeSettings || null;
  if (e && typeof e === 'object') {
    // Record exactly the settings the caller used; unknown keys are kept so
    // a future encoder change is visible in old sidecars, not hidden.
    return { ...e };
  }
  if (ctx.draft !== undefined) {
    // The adapter's two argument sets (CAPTURE-NATIVE.md, video finishing
    // parity) — recorded only because the caller told us which one ran.
    return ctx.draft
      ? { draft: true, preset: 'veryfast', crf: 23, audioBitrate: '192k' }
      : { draft: false, preset: 'slow', crf: 18, audioBitrate: '256k' };
  }
  return null;
}

// Build the sidecar object from a run context. Pure apart from the optional
// manifest hash read described above.
export function buildRenderMeta(ctx = {}, outPath = null) {
  const binary = (ctx.binary && typeof ctx.binary === 'object') ? ctx.binary : {};
  const manifestIn = (ctx.manifest && typeof ctx.manifest === 'object' && (ctx.manifest.path || ctx.manifest.sha256)) ? ctx.manifest : {};
  const manifestPath = pick(manifestIn.path, ctx.manifestPath, typeof ctx.manifest === 'string' ? ctx.manifest : null);
  let manifestSha = pick(manifestIn.sha256, ctx.manifestSha256, ctx.manifestHash);
  let shaError = null;
  if (!manifestSha && manifestPath) {
    try { manifestSha = sha256File(manifestPath); }
    catch (e) { shaError = e.message; }
  }
  const gate = normGate(pick(ctx.gate, ctx.gateVerdict, ctx.telemetryVerdict));
  const res = (ctx.resolution && typeof ctx.resolution === 'object') ? ctx.resolution : null;
  const meta = {
    format: RENDER_META_FORMAT,
    version: RENDER_META_VERSION,
    generatedAt: pick(ctx.generatedAt, ctx.timestamp) || new Date().toISOString(),
    backend: pick(ctx.backend, ctx.backendName) || 'native',
    mode: pick(ctx.mode) || 'video',
    output: outPath ? path.basename(outPath) : pick(ctx.output, ctx.out) ? path.basename(String(pick(ctx.output, ctx.out))) : null,
    binary: {
      path: pick(binary.path, ctx.bin, ctx.binaryPath, ctx.talerenderBin),
      via: pick(binary.via, ctx.via, ctx.discoveryRoute, ctx.discovery),
      version: pick(binary.version, ctx.binaryVersion, ctx.versionOutput),
    },
    manifest: {
      path: manifestPath,
      sha256: manifestSha,
      ...(shaError ? { sha256Error: shaError } : {}),
      title: pick(manifestIn.title, ctx.manifestTitle),
      totalFrames: pick(manifestIn.totalFrames, ctx.totalFrames) === null ? null : Number(pick(manifestIn.totalFrames, ctx.totalFrames)),
    },
    frameRange: normFrameRange(ctx),
    resolution: res
      ? { width: pick(res.width, res.W), height: pick(res.height, res.H) }
      : (ctx.W || ctx.H ? { width: ctx.W ?? null, height: ctx.H ?? null } : null),
    durationS: pick(ctx.durationS, manifestIn.durationS) === null ? null : Number(pick(ctx.durationS, manifestIn.durationS)),
    encode: normEncode(ctx),
    audio: ctx.hasMix === undefined && ctx.audio === undefined ? null : { source: ctx.audio ?? (ctx.hasMix ? 'data/mix.wav' : 'silent (lavfi anullsrc)') },
    gate,
  };
  return meta;
}

// Write <out>.render.json beside the video output `outPath` and return
// { sidecarPath, meta }. Written via a temp sibling + rename so a reader
// never sees a half-written sidecar (the same discipline as the adapter's
// `.part.mp4` -> final rename).
export function writeRenderMeta(ctx = {}, outPath) {
  const target = outPath || ctx.outPath || ctx.output || ctx.out;
  if (!target) throw new Error('render-meta: no video output path supplied (pass outPath or ctx.out) — the sidecar is named from the output');
  const sidecarPath = sidecarPathFor(String(target));
  const meta = buildRenderMeta(ctx, String(target));
  fs.mkdirSync(path.dirname(sidecarPath), { recursive: true });
  const tmp = `${sidecarPath}.tmp-${process.pid}`;
  fs.writeFileSync(tmp, JSON.stringify(meta, null, 2) + '\n', 'utf8');
  fs.renameSync(tmp, sidecarPath);
  return { sidecarPath, meta };
}

export default writeRenderMeta;

// ---------------------------------------------------------------------
// Self-test: node tools/native_modes/render_meta.mjs --self-test
// Temp fixtures only; exits 0 on PASS, 1 on FAIL.
// ---------------------------------------------------------------------
export function runSelfTest(log = console.log) {
  const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'render-meta-test-'));
  let failures = 0;
  const check = (name, cond, detail = '') => {
    log(`render-meta self-test: ${cond ? 'PASS' : 'FAIL'} ${name}${cond ? '' : ` — ${detail}`}`);
    if (!cond) failures++;
  };
  try {
    const manifestPath = path.join(tmp, 'film.json');
    const manifestBody = JSON.stringify({ format: 'talerender-film', version: 1, fps: 30, totalFrames: 90, durationS: 3 });
    fs.writeFileSync(manifestPath, manifestBody);
    const wantSha = crypto.createHash('sha256').update(manifestBody).digest('hex');
    const out = path.join(tmp, 'renders', 'out.mp4');
    fs.mkdirSync(path.dirname(out), { recursive: true });
    fs.writeFileSync(out, 'fake mp4 bytes');

    const { sidecarPath, meta } = writeRenderMeta({
      backend: 'native',
      binary: { path: '/fixtures/talerender', via: 'TALERENDER_BIN', version: 'talerender 1.0.1 (fixture)' },
      manifest: { path: manifestPath },
      mode: 'video',
      frameRange: { from: 30, count: 60, fps: 30 },
      encode: { draft: false, preset: 'slow', crf: 18, audioBitrate: '256k' },
      gate: { verdict: 'FILM TELEMETRY: CHECK OK' },
      generatedAt: '2026-11-15T00:00:00.000Z',
      hasMix: true,
    }, out);

    check('sidecar is <out>.render.json beside the output', sidecarPath === `${out}.render.json` && fs.existsSync(sidecarPath), sidecarPath);
    const onDisk = JSON.parse(fs.readFileSync(sidecarPath, 'utf8'));
    check('format/version stamped', onDisk.format === 'talerender-render-meta' && onDisk.version === 1);
    check('backend + binary provenance recorded verbatim', onDisk.backend === 'native' && onDisk.binary.path === '/fixtures/talerender' && onDisk.binary.via === 'TALERENDER_BIN' && onDisk.binary.version === 'talerender 1.0.1 (fixture)');
    check('manifest sha256 computed from the file on disk', onDisk.manifest.sha256 === wantSha, String(onDisk.manifest.sha256));
    check('frame range: to is derived as from+count', onDisk.frameRange.from === 30 && onDisk.frameRange.to === 90 && onDisk.frameRange.count === 60);
    check('encode settings recorded as supplied', onDisk.encode.crf === 18 && onDisk.encode.preset === 'slow');
    check('gate verdict normalised from the telemetry line', onDisk.gate.verdict === 'CHECK OK' && onDisk.gate.recognised === true);
    check('returned meta matches the file on disk', JSON.stringify(meta) === JSON.stringify(onDisk));

    // Flat-alias context + honesty: unsupplied fields are null, never invented.
    const flat = buildRenderMeta({ bin: '/x/talerender', via: 'PATH', gateVerdict: 'CHECK FAILED', n0: 0, n: 10, fps: 30, draft: true }, path.join(tmp, 'd.mp4'));
    check('flat aliases accepted (bin/via/gateVerdict/n0/n)', flat.binary.path === '/x/talerender' && flat.gate.verdict === 'CHECK FAILED' && flat.frameRange.to === 10);
    check('draft flag expands to the documented draft encode set', flat.encode.crf === 23 && flat.encode.preset === 'veryfast');
    check('unsupplied version/manifest are null, not fabricated', flat.binary.version === null && flat.manifest.path === null && flat.manifest.sha256 === null);
    const missing = buildRenderMeta({ manifest: { path: path.join(tmp, 'nope.json') } });
    check('unreadable manifest: sha256 null + sha256Error named', missing.manifest.sha256 === null && typeof missing.manifest.sha256Error === 'string');
    let threw = false;
    try { writeRenderMeta({}); } catch { threw = true; }
    check('no output path is a loud error, not a silent skip', threw);
  } finally {
    fs.rmSync(tmp, { recursive: true, force: true });
  }
  log(`render-meta self-test: ${failures === 0 ? 'PASS' : `FAIL (${failures} failing)`}`);
  return failures === 0 ? 0 : 1;
}

const isMain = process.argv[1] && import.meta.url === pathToFileURL(path.resolve(process.argv[1])).href;
if (isMain) {
  if (process.argv.includes('--self-test')) process.exitCode = runSelfTest();
  else { console.error('render-meta: this module is imported by capture_native.mjs; run it directly only as: node tools/native_modes/render_meta.mjs --self-test'); process.exitCode = 2; }
}
