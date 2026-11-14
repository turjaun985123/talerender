#!/usr/bin/env bash
# run_golden.sh — stage film mid frames into the native golden pack and
# verify the pack. See docs/golden/NATIVE-GOLDEN.md (policy) and
# tools/golden/RUNNER.md (usage + re-baseline protocol).
#
# WHERE THE FRAMES COME FROM
#   A film render's frame directory: frame_%05d.png files produced by
#   `talerender --film`. In practice one of:
#     - a CI render-workflow artifact (.github/workflows/render.yml,
#       workflow_dispatch, mode=film): the artifact is named
#       render-film-<from>-<to>-<run_id> and unpacks to a directory
#       holding render-out/frames/frame_%05d.png;
#     - a user-PC film render's frames directory (RADV).
#   The frames dir you pass may be the directory holding the
#   frame_%05d.png files themselves, or a parent holding frames/ or
#   render-out/frames/ under it; the first candidate that contains
#   frame_*.png files is used.
#
# THE LABELLING LAW
#   Frames are never accepted anonymously. --renderer <label> is
#   REQUIRED and names the renderer that produced them — in practice
#   "CI lavapipe" or "PC RADV". There is no default: a frames dir of
#   unknown provenance is refused, not guessed at. The label is stored
#   beside the staged copies (docs/golden/frames/RENDERER.txt) with
#   the source dir, the staging time, and the repo HEAD, so a staged
#   frame's origin stays auditable for as long as it sits there.
#   A label matching neither "lavapipe" nor "RADV" is accepted but
#   warned about, loudly, in the output and the provenance file.
#
# THE NO-PIN LAW
#   This script NEVER pins. Pinning (make_golden.py --pin) is the
#   deliberate re-baseline act of NATIVE-GOLDEN.md: a human act,
#   preceded by a measured diff, committed with the cause named.
#   Passing --pin (or "pin" in any form) to this script is refused,
#   with the reason printed. Staging a frame only places it at its
#   golden path; make_golden.py reports an on-disk-but-unpinned frame
#   as MISSING — not yet a reference — until a human pins it.
#
# WHAT IT DOES
#   1. Reads scenes/film/film.json; per segment the mid frame index is
#      startFrame + frames//2 (identical to the pack's
#      (startFrame + endFrame)//2 — endFrame is the exclusive end).
#   2. Copies each present mid frame to docs/golden/frames/<seg>.png.
#   3. Writes docs/golden/frames/RENDERER.txt (provenance, see above).
#   4. Runs `python3 tools/golden/make_golden.py` and prints its
#      verdict VERBATIM (the table and summary are the tool's own).
#   5. Reports every segment whose mid frame was absent, by segment
#      and exact frame index. Partial runs are reported, not hidden.
#
# EXIT CODES
#   0  every segment staged and make_golden.py exited 0
#   1  make_golden.py reported STALE (its exit code, passed through)
#   2  usage / refusal: --pin attempted, --renderer missing or empty,
#      unknown argument, frames dir or repo layout not found
#   3  PARTIAL: at least one segment's mid frame was absent (the
#      verbatim verify output above still stands; MISSING film items
#      do not fail make_golden.py, so this code is how a partial
#      staging refuses to look like a complete one)

set -u

PROG="$(basename "$0")"

refuse() {
    echo "$PROG: REFUSED: $*" >&2
    exit 2
}

usage() {
    cat >&2 <<EOF
usage: $PROG <frames-dir> --renderer <label> [--repo <path>]

  <frames-dir>    film render output (frame_%05d.png files, or a parent
                  holding frames/ or render-out/frames/)
  --renderer      REQUIRED provenance label, e.g. "CI lavapipe" or "PC RADV"
  --repo          talerender repo root (default: this script's repo)

This script stages and verifies only. It never pins — see the header.
EOF
    exit 2
}

# ---- arguments -------------------------------------------------------

FRAMES_DIR=""
RENDERER=""
REPO=""
while [ $# -gt 0 ]; do
    case "$1" in
        --pin|pin|--re-pin|--repin)
            refuse "--pin is a human act, not a runner step. Pinning is the deliberate re-baseline of docs/golden/NATIVE-GOLDEN.md: measure the diff first, then run 'python3 tools/golden/make_golden.py --pin' yourself and commit the hash change with the cause named. This script stages and verifies only."
            ;;
        --renderer)
            [ $# -ge 2 ] || usage
            RENDERER="$2"; shift 2 ;;
        --renderer=*)
            RENDERER="${1#--renderer=}"; shift ;;
        --repo)
            [ $# -ge 2 ] || usage
            REPO="$2"; shift 2 ;;
        --repo=*)
            REPO="${1#--repo=}"; shift ;;
        -*)
            echo "$PROG: unknown argument: $1" >&2; usage ;;
        *)
            [ -z "$FRAMES_DIR" ] || { echo "$PROG: unexpected extra argument: $1" >&2; usage; }
            FRAMES_DIR="$1"; shift ;;
    esac
done

[ -n "$FRAMES_DIR" ] || usage
# The labelling law: no label, no staging.
[ -n "$RENDERER" ] || refuse "no --renderer label given. Frames must be named by the renderer that produced them (e.g. \"CI lavapipe\" or \"PC RADV\"); anonymous frames are not staged."

# ---- repo layout -----------------------------------------------------

if [ -z "$REPO" ]; then
    REPO="$(cd "$(dirname "$0")/../.." && pwd)"
fi
[ -f "$REPO/tools/golden/make_golden.py" ] || refuse "no tools/golden/make_golden.py under repo: $REPO"
[ -f "$REPO/scenes/film/film.json" ] || refuse "no scenes/film/film.json under repo: $REPO"
MAKE_GOLDEN="$REPO/tools/golden/make_golden.py"
FILM_JSON="$REPO/scenes/film/film.json"
GOLDEN_FRAMES="$REPO/docs/golden/frames"

# ---- frames dir resolution ------------------------------------------

[ -d "$FRAMES_DIR" ] || refuse "frames dir not found: $FRAMES_DIR"
FRAME_SRC=""
for cand in "$FRAMES_DIR" "$FRAMES_DIR/frames" "$FRAMES_DIR/render-out/frames"; do
    if [ -d "$cand" ] && [ -n "$(find "$cand" -maxdepth 1 -name 'frame_*.png' -print -quit 2>/dev/null)" ]; then
        FRAME_SRC="$cand"
        break
    fi
done
[ -n "$FRAME_SRC" ] || refuse "no frame_*.png files in $FRAMES_DIR (looked in the dir itself, frames/, and render-out/frames/)"

# ---- renderer label sanity (warn, do not invent) ---------------------

LABEL_WARN=""
case "$RENDERER" in
    *[Ll]avapipe*|*[Rr][Aa][Dd][Vv]*) ;;
    *)
        LABEL_WARN="WARNING: renderer label \"$RENDERER\" matches neither \"lavapipe\" nor \"RADV\" — recorded as given, but check the provenance before trusting these frames."
        echo "$PROG: $LABEL_WARN"
        ;;
esac

# ---- segment mid frames from the film manifest -----------------------
# Emits one line per segment: "<segment-id> <mid-frame-index>".
# mid = startFrame + frames//2 (== (startFrame + endFrame)//2; endFrame
# is exclusive and startFrame + frames == endFrame in this manifest).

SEGMENTS="$(python3 - "$FILM_JSON" <<'PYEOF'
import json, sys
with open(sys.argv[1]) as f:
    film = json.load(f)
for seg in film["segments"]:
    mid = seg["startFrame"] + seg["frames"] // 2
    print(seg["id"], mid)
PYEOF
)" || refuse "could not parse $FILM_JSON"
[ -n "$SEGMENTS" ] || refuse "no segments in $FILM_JSON"

# ---- staging ----------------------------------------------------------

mkdir -p "$GOLDEN_FRAMES"
STAGED=""
MISSING=""
STAGED_COUNT=0
TOTAL_COUNT=0
while read -r SEG MID; do
    [ -n "$SEG" ] || continue
    TOTAL_COUNT=$((TOTAL_COUNT + 1))
    printf -v FRAME_NAME 'frame_%05d.png' "$MID"
    if [ -f "$FRAME_SRC/$FRAME_NAME" ]; then
        cp "$FRAME_SRC/$FRAME_NAME" "$GOLDEN_FRAMES/$SEG.png"
        SHA="$(sha256sum "$GOLDEN_FRAMES/$SEG.png" | cut -c1-16)"
        STAGED="${STAGED}film-${SEG}  frame ${MID} (${FRAME_NAME})  sha256 ${SHA}…
"
        STAGED_COUNT=$((STAGED_COUNT + 1))
    else
        MISSING="${MISSING}film-${SEG}  frame ${MID} (${FRAME_NAME})
"
    fi
done <<EOF
$SEGMENTS
EOF

# ---- provenance, stored beside the staged copies ----------------------

HEAD_SHA="$(git -C "$REPO" rev-parse --short HEAD 2>/dev/null || echo unknown)"
{
    echo "renderer: $RENDERER"
    [ -n "$LABEL_WARN" ] && echo "label-warning: $LABEL_WARN"
    echo "frames-dir (as passed): $FRAMES_DIR"
    echo "frames taken from: $FRAME_SRC"
    echo "staged at (UTC): $(date -u '+%Y-%m-%dT%H:%M:%SZ')"
    echo "repo HEAD: $HEAD_SHA"
    echo "staged segment mid frames:"
    if [ -n "$STAGED" ]; then printf '%s' "$STAGED"; else echo "(none)"; fi
    if [ -n "$MISSING" ]; then
        echo "still missing (not staged by this run):"
        printf '%s' "$MISSING"
    fi
    echo "note: staged frames are NOT pins. A frame becomes a golden reference"
    echo "only when a human runs make_golden.py --pin (see NATIVE-GOLDEN.md)."
} > "$GOLDEN_FRAMES/RENDERER.txt"

echo "$PROG: renderer: $RENDERER"
echo "$PROG: frames taken from: $FRAME_SRC"
echo "$PROG: staged $STAGED_COUNT of $TOTAL_COUNT segment mid frames into docs/golden/frames/"
echo "$PROG: provenance written to docs/golden/frames/RENDERER.txt"
echo

# ---- verify: the tool's verdict, verbatim -----------------------------

python3 "$MAKE_GOLDEN"
VERIFY_EXIT=$?
echo

# ---- the runner's own report ------------------------------------------

if [ -n "$MISSING" ]; then
    echo "$PROG: still-MISSING segment mid frames (exact indices):"
    printf '%s' "$MISSING"
fi

if [ "$VERIFY_EXIT" -ne 0 ]; then
    exit "$VERIFY_EXIT"
fi
if [ -n "$MISSING" ]; then
    echo "$PROG: PARTIAL RUN — $STAGED_COUNT of $TOTAL_COUNT segment mid frames staged; the rest are listed above with their exact frame indices."
    exit 3
fi
echo "$PROG: all $TOTAL_COUNT segment mid frames staged; verify verdict above is make_golden.py's own."
exit 0
