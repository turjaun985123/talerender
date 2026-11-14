#!/bin/bash
# run_reproof.sh — W62 INDEPENDENT adapter re-proof.
#
# Unit under test: the REAL skill-side adapter, copied fresh at run time from
# the skill-wiring work copy (template/tools/capture_native.mjs). Nothing in
# the work copy or the repo is modified; each cell runs in a fresh temp
# project assembled from the fixture projects beside this script.
#
# Grid: BOTH telemetry exit-code generations x THREE outcomes, per
# docs/integration/EXIT-CODES-W02.md section 4:
#   old (pre-aa10b13): exits 0 on OK, on FAILED, and on could-not-run.
#   new (post-aa10b13): 0 = OK, 1 = FAILED, 3 = could-not-run (no line).
# Plus the strip-mismatch cell: film data whose captions name a strip the
# assets do not contain must be REFUSED at pre-validation (the binary stub
# would print CHECK OK — the refusal must happen anyway), in check mode
# and in frames mode with zero artefacts.
#
# Resource discipline (the original false PASS was a SIGKILL under
# contention): cells run STRICTLY SEQUENTIALLY, each under `timeout 120`,
# with /proc/loadavg recorded before every cell. One disclosed retry is
# allowed per cell, only when the attempt was killed (rc 124/137 or the
# adapter reported a signal death); a cell whose attempts never complete
# cleanly is UNPROVEN, never passed by assertion.
set -u
HERE="$(cd "$(dirname "$0")" && pwd)"
WORKCOPY="${TALERENDER_SKILL_WIRING:-$HOME/workspace/talerender-skill-wiring}"
ADAPTER_SRC="$WORKCOPY/template/tools/capture_native.mjs"
TIMEOUT_S=120
chmod +x "$HERE"/stubs/*.sh 2>/dev/null

[ -f "$ADAPTER_SRC" ] || { echo "FATAL: adapter not found at $ADAPTER_SRC"; exit 2; }
node --check "$ADAPTER_SRC" || { echo "FATAL: adapter has a syntax error"; exit 2; }

TMP="$(mktemp -d /tmp/w62-reproof.XXXXXX)"
echo "== W62 adapter re-proof =="
echo "date:        $(date -u '+%Y-%m-%dT%H:%M:%SZ')"
echo "adapter:     $ADAPTER_SRC"
echo "adapter sha: $(sha256sum "$ADAPTER_SRC" | cut -d' ' -f1)"
echo "node:        $(node --version) ($(command -v node))"
echo "cpus:        $(nproc)   loadavg at start: $(cut -d' ' -f1-3 /proc/loadavg)"
echo "mem:         $(free -m | awk '/^Mem:/{print $7" MB available of "$2" MB"}')"
echo "timeout:     ${TIMEOUT_S}s per adapter invocation; cells sequential"
echo "tmp:         $TMP"

echo
echo "-- stub self-check (each stub run directly with the telemetry argv) --"
for s in gen-old-ok gen-old-fail gen-old-norun gen-new-ok gen-new-fail gen-new-norun; do
  SOUT="$("$HERE/stubs/$s.sh" --film-telemetry /nonexistent 2>&1)"; SRC=$?
  echo "stub $s -> rc=$SRC out=$(printf '%s' "$SOUT" | head -1)"
done

PASSN=0; FAILN=0; UNPROVEN=0
report() { # status, label
  case "$1" in
    MATCH) PASSN=$((PASSN+1));;
    MISMATCH) FAILN=$((FAILN+1));;
    UNPROVEN) UNPROVEN=$((UNPROVEN+1));;
  esac
  echo "CELL-VERDICT: $1 — $2"
}

run_once() { # celldir stub mode args... -> sets RC, OUT, WALL
  local celldir="$1" stub="$2" mode="$3"; shift 3
  local proj="$celldir/proj"
  local t0 t1
  t0="$(date +%s.%N)"
  OUT="$(cd "$proj" && RENDERER=native TALERENDER_BIN="$HERE/stubs/$stub" timeout "$TIMEOUT_S" node tools/capture_native.mjs "$mode" "$@" 2>&1)"
  RC=$?
  t1="$(date +%s.%N)"
  WALL="$(awk -v a="$t0" -v b="$t1" 'BEGIN{printf "%.2f", b-a}')"
}

run_cell() { # name fixture stub mode args... -> sets RC OUT WALL ATTEMPTS PROJ
  local name="$1" fixture="$2" stub="$3" mode="$4"; shift 4
  local celldir="$TMP/$name"
  mkdir -p "$celldir"
  cp -r "$HERE/$fixture" "$celldir/proj"
  mkdir -p "$celldir/proj/tools"
  cp "$ADAPTER_SRC" "$celldir/proj/tools/capture_native.mjs"
  PROJ="$celldir/proj"
  ATTEMPTS=0
  while [ "$ATTEMPTS" -lt 2 ]; do
    ATTEMPTS=$((ATTEMPTS+1))
    echo "cell $name (fixture=$fixture stub=$stub mode=$mode $*) attempt $ATTEMPTS loadavg=$(cut -d' ' -f1-3 /proc/loadavg)"
    run_once "$celldir" "$stub" "$mode" "$@"
    echo "  rc=$RC wall=${WALL}s"
    printf '%s\n' "$OUT" | sed 's/^/  | /'
    if [ "$RC" != 124 ] && ! printf '%s' "$OUT" | grep -q 'killed by signal'; then break; fi
    echo "  !! attempt killed or timed out (rc=$RC) — disclosed retry after 20s pause"
    sleep 20
  done
  if [ "$RC" = 124 ] || printf '%s' "$OUT" | grep -q 'killed by signal'; then return 9; fi
  return 0
}

expect() { # condition-result(0/1) label
  if [ "$1" != 0 ]; then echo "  expectation FAILED: $2"; return 1; fi
  echo "  expectation ok: $2"; return 0
}

echo
echo "-- grid cells (check mode, matched fixture) --"

run_cell old-ok project gen-old-ok.sh check; CLEAN=$?
ok=0
[ "$CLEAN" = 0 ] || ok=2
if [ "$ok" = 0 ]; then
  [ "$RC" = 0 ]; expect $? "adapter rc 0" || ok=1
  printf '%s\n' "$OUT" | grep -qx 'CHECK OK'; expect $? "adapter terminal line 'CHECK OK'" || ok=1
fi
case $ok in 0) report MATCH "old generation x OK (line OK, exit 0) -> accepted, rc 0";;
            1) report MISMATCH "old generation x OK";;
            *) report UNPROVEN "old generation x OK (no clean run)";; esac

run_cell old-fail project gen-old-fail.sh check; CLEAN=$?
ok=0
[ "$CLEAN" = 0 ] || ok=2
if [ "$ok" = 0 ]; then
  [ "$RC" = 1 ]; expect $? "adapter rc 1 (gate fails)" || ok=1
  printf '%s\n' "$OUT" | grep -qx 'CHECK FAILED'; expect $? "adapter terminal line 'CHECK FAILED'" || ok=1
fi
case $ok in 0) report MATCH "old generation x FAILED (line FAILED, exit 0) -> gate refuses, rc 1";;
            1) report MISMATCH "old generation x FAILED";;
            *) report UNPROVEN "old generation x FAILED (no clean run)";; esac

run_cell old-norun project gen-old-norun.sh check; CLEAN=$?
ok=0
[ "$CLEAN" = 0 ] || ok=2
if [ "$ok" = 0 ]; then
  [ "$RC" = 1 ]; expect $? "adapter rc 1 (fail closed)" || ok=1
  printf '%s\n' "$OUT" | grep -qF 'no verdict line'; expect $? "fail-closed message: no verdict line" || ok=1
  printf '%s\n' "$OUT" | grep -qF 'binary exit 0'; expect $? "message names binary exit 0" || ok=1
fi
case $ok in 0) report MATCH "old generation x could-not-run (no line, stderr, exit 0) -> fail closed, rc 1";;
            1) report MISMATCH "old generation x could-not-run";;
            *) report UNPROVEN "old generation x could-not-run (no clean run)";; esac

run_cell new-ok project gen-new-ok.sh check; CLEAN=$?
ok=0
[ "$CLEAN" = 0 ] || ok=2
if [ "$ok" = 0 ]; then
  [ "$RC" = 0 ]; expect $? "adapter rc 0" || ok=1
  printf '%s\n' "$OUT" | grep -qx 'CHECK OK'; expect $? "adapter terminal line 'CHECK OK'" || ok=1
fi
case $ok in 0) report MATCH "new generation x OK (line OK, exit 0) -> accepted, rc 0";;
            1) report MISMATCH "new generation x OK";;
            *) report UNPROVEN "new generation x OK (no clean run)";; esac

run_cell new-fail project gen-new-fail.sh check; CLEAN=$?
ok=0
[ "$CLEAN" = 0 ] || ok=2
if [ "$ok" = 0 ]; then
  [ "$RC" = 1 ]; expect $? "adapter rc 1 (gate fails)" || ok=1
  printf '%s\n' "$OUT" | grep -qx 'CHECK FAILED'; expect $? "adapter terminal line 'CHECK FAILED'" || ok=1
fi
case $ok in 0) report MATCH "new generation x FAILED (line FAILED, exit 1) -> gate refuses, rc 1";;
            1) report MISMATCH "new generation x FAILED";;
            *) report UNPROVEN "new generation x FAILED (no clean run)";; esac

run_cell new-norun project gen-new-norun.sh check; CLEAN=$?
ok=0
[ "$CLEAN" = 0 ] || ok=2
if [ "$ok" = 0 ]; then
  [ "$RC" = 1 ]; expect $? "adapter rc 1 (fail closed)" || ok=1
  printf '%s\n' "$OUT" | grep -qF 'no verdict line'; expect $? "fail-closed message: no verdict line" || ok=1
  printf '%s\n' "$OUT" | grep -qF 'Exit 3'; expect $? "message names exit 3 (could-not-run)" || ok=1
fi
case $ok in 0) report MATCH "new generation x could-not-run (no line, stderr, exit 3) -> fail closed, rc 1";;
            1) report MISMATCH "new generation x could-not-run";;
            *) report UNPROVEN "new generation x could-not-run (no clean run)";; esac

echo
echo "-- strip-mismatch cells (binary stub would print CHECK OK / exit 0) --"

run_cell mismatch-check project-strip-mismatch gen-new-ok.sh check; CLEAN=$?
ok=0
[ "$CLEAN" = 0 ] || ok=2
if [ "$ok" = 0 ]; then
  [ "$RC" = 1 ]; expect $? "adapter rc 1 (refused)" || ok=1
  printf '%s\n' "$OUT" | grep -qF 'assets/cap_002.png'; expect $? "refusal names the missing strip assets/cap_002.png" || ok=1
  printf '%s\n' "$OUT" | grep -qF 'is incomplete'; expect $? "refusal is the film-data completeness failure" || ok=1
  if printf '%s\n' "$OUT" | grep -q 'FILM TELEMETRY'; then
    echo "  expectation FAILED: a FILM TELEMETRY line appeared — the gate was reached"; ok=1
  else
    echo "  expectation ok: telemetry verdict never reached (no FILM TELEMETRY line in output)"
  fi
fi
case $ok in 0) report MATCH "strip mismatch x check -> refused at pre-validation, gate never reached";;
            1) report MISMATCH "strip mismatch x check";;
            *) report UNPROVEN "strip mismatch x check (no clean run)";; esac

run_cell mismatch-frames project-strip-mismatch gen-new-ok.sh frames review/frames 1.0; CLEAN=$?
ok=0
[ "$CLEAN" = 0 ] || ok=2
if [ "$ok" = 0 ]; then
  [ "$RC" = 1 ]; expect $? "adapter rc 1 (refused)" || ok=1
  printf '%s\n' "$OUT" | grep -qF 'assets/cap_002.png'; expect $? "refusal names the missing strip assets/cap_002.png" || ok=1
  [ ! -e "$PROJ/renders" ] && [ ! -e "$PROJ/review" ]; expect $? "zero artefacts (no renders/, no review/)" || ok=1
fi
case $ok in 0) report MATCH "strip mismatch x frames -> refused at pre-validation, zero artefacts";;
            1) report MISMATCH "strip mismatch x frames";;
            *) report UNPROVEN "strip mismatch x frames (no clean run)";; esac

echo
echo "== summary: $PASSN matched, $FAILN mismatched, $UNPROVEN unproven (of 8 cells) =="
echo "loadavg at end: $(cut -d' ' -f1-3 /proc/loadavg)"
[ "$FAILN" = 0 ] && [ "$UNPROVEN" = 0 ]
