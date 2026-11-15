// era_hint.mjs — era-pairing error surface for a missing telemetry verdict.
// CHROME-REPLACEMENT WAVE, Batch 1, Seat 10 (CR-B1-10).
//
// VERIFIED GAP (from code, 2026-11-15): when the gate finds no verdict line,
// capture_native.mjs's runTelemetry() fails with exactly:
//
//   "film telemetry produced no verdict line (binary <how>) — treated as a
//    FAILED gate. Exit 3 means the film data could not be read (the reason
//    is on stderr, above); exit 2 usually means the binary predates
//    --film-telemetry — use a current CI artifact."
//
// That names a cause ("predates --film-telemetry") and gestures at a remedy
// ("use a current CI artifact"), but it does NOT name the provisioning
// remedy the contract requires: it never mentions TALERENDER_BIN, never
// gives the CI download commands, and never warns that the repo-checkout
// discovery slot is the usual stale source (CAPTURE-NATIVE.md, Prerequisite
// 1 + "Era pairing"). An operator hitting the stale-binary wall — the exact
// failure of every local dry run in the adoption wave — gets no actionable
// next step from the adapter itself. This module supplies that message.
//
// Wiring (lead, see CR-B1-10-ADAPTER.md): in runTelemetry(), replace the
// final fail(...) for the no-verdict case with
//   fail(eraHintMessage({ status, signal, bin, via, stdout, stderr }))
// keeping the fail-closed semantics identical — this module changes only
// the TEXT, never the verdict.
//
// Node builtins only.
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { pathToFileURL } from 'node:url';

export const STALE_MARKER = 'unknown argument: --film-telemetry';
export const VERDICT_OK = 'FILM TELEMETRY: CHECK OK';
export const VERDICT_FAILED = 'FILM TELEMETRY: CHECK FAILED';

// Does this telemetry output contain a verdict line at all?
export function hasVerdictLine(stdout = '') {
  return String(stdout).split('\n').some((l) => l.startsWith(VERDICT_OK) || l.startsWith(VERDICT_FAILED));
}

// Classify a telemetry run that produced NO verdict line.
//   { status, signal, stdout, stderr } — spawnSync-shaped.
// Returns null when a verdict line IS present (not this module's case),
// otherwise { stale: boolean, how: string } where `stale` is true only on
// positive evidence of an era mismatch: exit 2, or the binary's own
// "unknown argument: --film-telemetry" complaint on either stream. Other
// no-verdict runs (exit 3 = data unreadable, a signal, a crash) are NOT
// labelled stale — they are reported as no-verdict with the stale check
// offered as one named possibility, never asserted.
export function classifyNoVerdict({ status = null, signal = null, stdout = '', stderr = '' } = {}) {
  if (hasVerdictLine(stdout)) return null;
  const combined = `${stdout}\n${stderr}`;
  const stale = status === 2 || combined.includes(STALE_MARKER);
  const how = status === null ? `killed by signal ${signal || '?'}` : `exit ${status}`;
  return { stale, how };
}

// The provisioning remedy, verbatim in the shape of CAPTURE-NATIVE.md
// Prerequisite 1 (the repo runbook's commands). TALERENDER_BIN is named
// because an explicit TALERENDER_BIN is never second-guessed by discovery —
// it is the one remedy that cannot be silently overridden by a stale file
// sitting in a discovery slot.
export function provisioningRemedy(binPath = null) {
  return [
    'Provision a current binary from a green CI run of turjaun985123/talerender and point TALERENDER_BIN at it:',
    '  gh run list --repo turjaun985123/talerender --workflow CI --limit 5',
    '  gh run download <run-id> --repo turjaun985123/talerender -n talerender-binary -D ~/talerender-bin',
    '  chmod +x ~/talerender-bin/talerender',
    '  export TALERENDER_BIN=~/talerender-bin/talerender',
    'Do not rely on the repo-checkout discovery slot (~/workspace/talerender/talerender): it holds whatever build happens to sit in the checkout and was a stale pre-gate build in every local run of the integration wave.',
    ...(binPath ? [`Binary used for this run: ${binPath}`] : []),
  ].join('\n');
}

// Build the full fail() message for the no-verdict case. Returns null when
// a verdict line is present (the caller is in the wrong branch).
export function eraHintMessage({ status = null, signal = null, stdout = '', stderr = '', bin = null, via = null } = {}) {
  const c = classifyNoVerdict({ status, signal, stdout, stderr });
  if (!c) return null;
  const viaNote = via ? ` (via ${via})` : '';
  if (c.stale) {
    return [
      `film telemetry produced no verdict line (binary ${c.how}) — treated as a FAILED gate.`,
      `Symptom: era mismatch — the talerender binary${viaNote} predates the film gate: it does not recognise --film-telemetry (a current binary answers the telemetry call with a "FILM TELEMETRY: CHECK OK / CHECK FAILED" line; this one produced none). The binary and the project's bridge data must come from compatible repo states; a stale binary renders nothing rather than rendering wrong frames.`,
      'Remedy: ' + provisioningRemedy(bin),
      'Then re-run the same command. There is no flag to render past a failed gate.',
    ].join('\n');
  }
  return [
    `film telemetry produced no verdict line (binary ${c.how}) — treated as a FAILED gate.`,
    'Exit 3 means the film data could not be read (the reason is on stderr, above); a crash or signal means the binary itself failed — check the output above first.',
    `One other named cause is an era mismatch (a stale binary that predates --film-telemetry). Verify currency before re-running:  "$TALERENDER_BIN" --film-telemetry 2>&1 | grep -q '${STALE_MARKER}' && echo STALE || echo OK`,
    'If the binary is stale, the remedy is: ' + provisioningRemedy(bin),
  ].join('\n');
}

export default eraHintMessage;

// ---------------------------------------------------------------------
// Self-test: node tools/native_modes/era_hint.mjs --self-test
// Fixtures only (no binary is executed); exits 0 on PASS, 1 on FAIL.
// ---------------------------------------------------------------------
export function runSelfTest(log = console.log) {
  let failures = 0;
  const check = (name, cond, detail = '') => {
    log(`era-hint self-test: ${cond ? 'PASS' : 'FAIL'} ${name}${cond ? '' : ` — ${detail}`}`);
    if (!cond) failures++;
  };
  // Sanity: the fixtures dir is a temp dir, proving nothing here touches a project.
  const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'era-hint-test-'));
  try {
    const stale = eraHintMessage({ status: 2, stdout: '', stderr: `talerender: ${STALE_MARKER}`, bin: '/fixtures/talerender', via: 'TaleRender repo checkout' });
    check('stale fixture (exit 2 + unknown-argument) yields a message', typeof stale === 'string');
    check('stale message names the symptom (era mismatch / predates the film gate)', /era mismatch/.test(stale) && /predates the film gate/.test(stale));
    check('stale message names TALERENDER_BIN as the remedy', /export TALERENDER_BIN=/.test(stale));
    check('stale message gives the CI provisioning commands', /gh run download <run-id> --repo turjaun985123\/talerender -n talerender-binary/.test(stale));
    check('stale message warns against the repo-checkout slot', /repo-checkout discovery slot/.test(stale));
    check('stale classification is positive-evidence only', classifyNoVerdict({ status: 2, stdout: '', stderr: '' })?.stale === true && classifyNoVerdict({ status: 3, stdout: '', stderr: 'cannot read film data' })?.stale === false);

    const generic = eraHintMessage({ status: 3, stdout: '', stderr: 'film telemetry failed: cannot read ...', bin: '/fixtures/talerender' });
    check('exit-3 fixture: message keeps the data-cause explanation', /Exit 3 means the film data could not be read/.test(generic));
    check('exit-3 fixture: stale offered as a check, not asserted', /One other named cause is an era mismatch/.test(generic) && /Verify currency/.test(generic));
    check('exit-3 fixture: remedy still names TALERENDER_BIN', /TALERENDER_BIN/.test(generic));

    const sig = eraHintMessage({ status: null, signal: 'SIGKILL', stdout: '', stderr: '' });
    check('signal fixture: how is reported as killed-by-signal', /killed by signal SIGKILL/.test(sig));

    check('verdict present is not this module\u2019s case (null)', eraHintMessage({ status: 0, stdout: 'FILM TELEMETRY: CHECK OK\n' }) === null && classifyNoVerdict({ status: 1, stdout: 'FILM TELEMETRY: CHECK FAILED (x)\n' }) === null);
    check('hasVerdictLine matches the adapter\u2019s exact prefixes', hasVerdictLine('FILM TELEMETRY: CHECK OK') && hasVerdictLine('x\nFILM TELEMETRY: CHECK FAILED (a)') && !hasVerdictLine('CHECK OK'));
  } finally {
    fs.rmSync(tmp, { recursive: true, force: true });
  }
  log(`era-hint self-test: ${failures === 0 ? 'PASS' : `FAIL (${failures} failing)`}`);
  return failures === 0 ? 0 : 1;
}

const isMain = process.argv[1] && import.meta.url === pathToFileURL(path.resolve(process.argv[1])).href;
if (isMain) {
  if (process.argv.includes('--self-test')) process.exitCode = runSelfTest();
  else { console.error('era-hint: this module is imported by capture_native.mjs; run it directly only as: node tools/native_modes/era_hint.mjs --self-test'); process.exitCode = 2; }
}
