#!/usr/bin/env bash
# determinism_check.sh — settle the resume-purity / determinism proof item.
#
# docs/integration/README.md claims of the film driver: "chunking never
# changes a frame (frame(t) is pure — proven by re-rendering a chunk
# boundary twice and byte-comparing)", while the same file's proof list
# still carries "TBD-PROOF: … determinism byte-compare". Until this
# script has been run and its results file cited, the TBD is the honest
# side. This script is the byte-compare, packaged so ONE run — a CI
# dispatch job or a run on the user's PC — produces the evidence.
#
# WHAT IT RENDERS (film frames [100, 130), 30 frames, 1080x1920):
#   A  one run:            --from 100 --to 130            (default cache)
#   B  two chunked runs:   --from 100 --to 115, then --from 115 --to 130
#                          into the SAME output dir (the resume path)
#   C  one run:            --from 100 --to 130 with TALERENDER_PAINT_CACHE=24
#
# WHAT EACH COMPARISON PROVES:
#   A vs B  — resume/chunk purity: splitting a render at a chunk
#             boundary (and resuming into a partially-filled dir) does
#             not change any frame's bytes. This is the README claim.
#   A vs C  — paint-cache-cap pixel-neutrality: TALERENDER_PAINT_CACHE
#             (film.rs's cap on the ~29 MB-per-canvas paint cache;
#             default 160, tight-memory runs use 24) costs time, never
#             pixels. PERF.md §4 already reads different verdicts at
#             different caps as a determinism defect; this tests it at
#             the byte level.
#
# FRESHNESS: the driver skips frames whose PNG already exists, so a
# reused output dir could "pass" by comparing a run against itself.
# The script therefore refuses to start if any of its three frame dirs
# already contains files, asserts every run's own completion line
# reports "0 skipped", and checks the rendered count per run.
#
# LABELLING: the results file records the binary's path and sha256 and
# the `renderer:` line from run A (adapter name / backend / device
# type / driver). A PASS on CI lavapipe and a PASS on the user's PC
# (RADV) are separate evidence; neither stands in for the other.
#
# RUNTIME CLASS: four short renders (30 + 15 + 15 + 30 frames) plus
# hashing. No timing is measured or claimed — the PERF protocol
# (docs/runbook/PERF.md) owns performance numbers.
#
# USAGE:
#   determinism_check.sh <talerender-binary> <output-root> [--film-dir <dir>]
#   DET_FILM_DIR=<dir> determinism_check.sh <binary> <output-root>
# <film-dir> defaults to the repo's film/ (two levels up from this
# script). The output root is created if absent; results land in
# <output-root>/determinism-results.txt, per-run logs in
# <output-root>/logs/, frames in A-single/ B-chunked/ C-cache24/.
#
# EXIT CODES: 0 = DETERMINISM: PASS · 1 = DETERMINISM: FAIL ·
# 2 = refusal / broken run (no verdict claimed: stale binary, missing
# binary, non-fresh output root, a render that crashed or skipped).

set -uo pipefail

FROM=100
SPLIT=115
TO=130
TOTAL=$((TO - FROM))
CACHE_CAP_TIGHT=24

refuse() {
    echo "REFUSAL: $*" >&2
    exit 2
}

# ---------------------------------------------------------------- args
BIN="${1:-}"
OUTROOT="${2:-}"
FILM_DIR="${DET_FILM_DIR:-}"
if [ $# -ge 2 ]; then shift 2; else shift $#; fi
while [ $# -gt 0 ]; do
    case "$1" in
        --film-dir) [ $# -ge 2 ] || refuse "--film-dir needs a value"; FILM_DIR="$2"; shift 2 ;;
        *) refuse "unknown argument: $1 (usage: determinism_check.sh <binary> <output-root> [--film-dir <dir>])" ;;
    esac
done
[ -n "$BIN" ] || refuse "no binary given (usage: determinism_check.sh <binary> <output-root> [--film-dir <dir>])"
[ -n "$OUTROOT" ] || refuse "no output root given"
[ -x "$BIN" ] || refuse "binary not found or not executable: $BIN"
command -v sha256sum >/dev/null 2>&1 || refuse "sha256sum not found (coreutils required)"

SCRIPT_DIR=$(cd "$(dirname "$0")" && pwd)
if [ -z "$FILM_DIR" ]; then
    FILM_DIR="$SCRIPT_DIR/../../film"
fi
[ -f "$FILM_DIR/onoda.json" ] || refuse "film data dir lacks onoda.json: $FILM_DIR (pass --film-dir <repo>/film)"
FILM_DIR=$(cd "$FILM_DIR" && pwd)
BIN_ABS=$(cd "$(dirname "$BIN")" && pwd)/$(basename "$BIN")
BIN_SHA=$(sha256sum "$BIN_ABS" | awk '{print $1}')

# ------------------------------------------- capability probe (--film)
# A current binary pointed at an EMPTY data dir dies in load_film_data
# ("film data: …", exit 101) before any GPU work or file writes. A
# stale binary (e.g. the 13 Nov repo-root build) does not know --film
# at all: "unknown argument: --film", exit 2. Anything else is
# unexplained and also refused — no verdict from an unprobed binary.
PROBE_DIR=$(mktemp -d)
PROBE_ERR=$(mktemp)
"$BIN_ABS" --film "$PROBE_DIR" > /dev/null 2>"$PROBE_ERR"
PROBE_RC=$?
if [ "$PROBE_RC" -eq 2 ] && grep -q "unknown argument" "$PROBE_ERR"; then
    rm -rf "$PROBE_DIR" "$PROBE_ERR"
    refuse "stale binary: $BIN_ABS does not know --film (probe: 'unknown argument', exit 2). Provision a current CI-artifact binary (talerender-binary) and re-run."
fi
if grep -q "film data:" "$PROBE_ERR" || [ "$PROBE_RC" -eq 101 ]; then
    : # --film is understood; the empty data dir failed exactly as expected.
else
    sed 's/^/  probe stderr: /' "$PROBE_ERR" >&2
    rm -rf "$PROBE_DIR" "$PROBE_ERR"
    refuse "binary probe inconclusive: $BIN_ABS --film <empty dir> exited $PROBE_RC without the expected 'film data:' load failure. Not running."
fi
rm -rf "$PROBE_DIR" "$PROBE_ERR"

# ------------------------------------------------------- fresh staging
mkdir -p "$OUTROOT" || refuse "cannot create output root: $OUTROOT"
OUTROOT=$(cd "$OUTROOT" && pwd)
DIR_A="$OUTROOT/A-single"
DIR_B="$OUTROOT/B-chunked"
DIR_C="$OUTROOT/C-cache24"
LOG_DIR="$OUTROOT/logs"
RESULTS="$OUTROOT/determinism-results.txt"
for d in "$DIR_A" "$DIR_B" "$DIR_C"; do
    if [ -d "$d" ] && [ -n "$(ls -A "$d" 2>/dev/null)" ]; then
        refuse "output dir not fresh: $d already contains files. The driver skips existing frames, so a reused dir could fake a pass. Remove it or choose a new output root."
    fi
done
mkdir -p "$DIR_A" "$DIR_B" "$DIR_C" "$LOG_DIR" || refuse "cannot create staging dirs under $OUTROOT"

# ------------------------------------------------------------- output
: > "$RESULTS" || refuse "cannot write results file: $RESULTS"
say() {
    echo "$1"
    echo "$1" >> "$RESULTS"
}

# --------------------------------------------------------- render runs
# render_run <label> <frames-dir> <from> <to> <cache-cap-or-empty>
render_run() {
    local label="$1" dir="$2" from="$3" to="$4" cap="$5"
    local log="$LOG_DIR/$label.log"
    local rc skipped done_n
    if [ -n "$cap" ]; then
        ( cd "$OUTROOT" && TALERENDER_PAINT_CACHE="$cap" "$BIN_ABS" \
            --film "$FILM_DIR" --out "$dir" --from "$from" --to "$to" \
            --width 1080 --height 1920 ) > "$log" 2>&1
        rc=$?
    else
        ( cd "$OUTROOT" && "$BIN_ABS" \
            --film "$FILM_DIR" --out "$dir" --from "$from" --to "$to" \
            --width 1080 --height 1920 ) > "$log" 2>&1
        rc=$?
    fi
    if [ "$rc" -ne 0 ]; then
        tail -5 "$log" | sed 's/^/  render log: /' >&2
        refuse "render run '$label' exited $rc (log: $log). No determinism verdict from a broken run."
    fi
    if ! grep -q "film render complete:" "$log"; then
        refuse "render run '$label' printed no completion line (log: $log). No determinism verdict from an incomplete run."
    fi
    skipped=$(sed -n 's/.*film render complete: [0-9]* rendered, \([0-9]*\) skipped.*/\1/p' "$log" | tail -1)
    done_n=$(sed -n 's/.*film render complete: \([0-9]*\) rendered,.*/\1/p' "$log" | tail -1)
    if [ "$skipped" != "0" ]; then
        refuse "render run '$label' skipped $skipped existing frames — the output dir was not fresh, so its frames cannot be trusted for a byte-compare (log: $log)."
    fi
    if [ "$done_n" != "$((to - from))" ]; then
        refuse "render run '$label' rendered $done_n frames, expected $((to - from)) (log: $log)."
    fi
}

say "determinism byte-compare — frames [$FROM, $TO) at 1080x1920"
say "date (UTC): $(date -u '+%Y-%m-%dT%H:%M:%SZ')"
say "binary: $BIN_ABS"
say "binary sha256: $BIN_SHA"
say "film data dir: $FILM_DIR"
say ""

render_run A-single "$DIR_A" "$FROM" "$TO" ""
render_run B-chunk1 "$DIR_B" "$FROM" "$SPLIT" ""
render_run B-chunk2 "$DIR_B" "$SPLIT" "$TO" ""
render_run C-cache24 "$DIR_C" "$FROM" "$TO" "$CACHE_CAP_TIGHT"

RENDERER_LINE=$(grep -m1 '^renderer: ' "$LOG_DIR/A-single.log" || true)
[ -n "$RENDERER_LINE" ] || refuse "run A printed no 'renderer:' line (log: $LOG_DIR/A-single.log) — the run cannot be labelled, so its bytes cannot be cited as evidence."
say "renderer (run A): $RENDERER_LINE"
say "runs: A single [$FROM,$TO) · B chunked [$FROM,$SPLIT)+[$SPLIT,$TO) same dir · C single [$FROM,$TO) TALERENDER_PAINT_CACHE=$CACHE_CAP_TIGHT"
say ""

# ----------------------------------------------------------- comparing
TOTAL_DIFFER=0

# compare <title> <dir-left> <left-label> <dir-right> <right-label>
compare() {
    local title="$1" dl="$2" ll="$3" dr="$4" rl="$5"
    local n f fl fr hl hr differ=0
    say "== $title =="
    for ((n = FROM; n < TO; n++)); do
        f=$(printf 'frame_%05d.png' "$n")
        fl="$dl/$f"
        fr="$dr/$f"
        if [ ! -f "$fl" ]; then
            say "frame $(printf '%05d' "$n"): DIFFER (missing in $ll)"
            differ=$((differ + 1))
        elif [ ! -f "$fr" ]; then
            say "frame $(printf '%05d' "$n"): DIFFER (missing in $rl)"
            differ=$((differ + 1))
        else
            hl=$(sha256sum "$fl" | awk '{print $1}')
            hr=$(sha256sum "$fr" | awk '{print $1}')
            if [ "$hl" = "$hr" ]; then
                say "frame $(printf '%05d' "$n"): IDENTICAL"
            else
                say "frame $(printf '%05d' "$n"): DIFFER (sha256 mismatch, $ll vs $rl)"
                differ=$((differ + 1))
            fi
        fi
    done
    say "section: $((TOTAL - differ))/$TOTAL identical ($differ differ)"
    say ""
    TOTAL_DIFFER=$((TOTAL_DIFFER + differ))
}

compare "chunk-boundary: A (single) vs B (chunked, resumed)" "$DIR_A" "A" "$DIR_B" "B"
compare "cache-cap: A (default cache) vs C (TALERENDER_PAINT_CACHE=$CACHE_CAP_TIGHT)" "$DIR_A" "A" "$DIR_C" "C"

if [ "$TOTAL_DIFFER" -eq 0 ]; then
    say "DETERMINISM: PASS"
    exit 0
else
    say "DETERMINISM: FAIL ($TOTAL_DIFFER differ)"
    exit 1
fi
