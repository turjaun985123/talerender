#!/usr/bin/env bash
# Reproduce the M6 harness outputs (caption metrics + the overflow
# variant's Chrome cutouts). Copies the Onoda project to a scratch
# dir, drops in harness.ts + harness.html, builds with bun, serves
# it locally and drives headless Chrome (SwiftShader) at it; the
# page POSTs the results back to the server (project/serve.mjs).
# Usage: bash run-harness.sh <out-dir>
set -euo pipefail
PKG="$(cd "$(dirname "$0")/.." && pwd)"
PROJECT="$HOME/faceless-shorts/the-soldier-who-didnt-know"
CHROME="${CHROME:-$HOME/.faceless-shorts/chrome/opt/google/chrome/chrome}"
OUT="$(mkdir -p "$1" && cd "$1" && pwd)"
WORK="$(mktemp -d /tmp/m6-harness.XXXXXX)"
echo "workdir: $WORK  out: $OUT"
rsync -a --exclude node_modules --exclude build --exclude renders --exclude review "$PROJECT/" "$WORK/"
cp "$PKG/project/harness.ts" "$WORK/src/harness.ts"
cp "$PKG/project/harness.html" "$WORK/harness.html"
cd "$WORK"
bun build src/harness.ts --outdir build --target browser --external three --external "three/addons/*"
node "$PKG/project/serve.mjs" "$OUT" 8973 &
SERVER=$!
sleep 1
"$CHROME" --headless=new --no-sandbox --ignore-gpu-blocklist \
  --enable-unsafe-swiftshader --use-angle=swiftshader \
  --disable-background-timer-throttling --disable-renderer-backgrounding \
  --window-size=1280,900 "http://127.0.0.1:8973/harness.html" &
CHROME_PID=$!
wait $SERVER || true
kill $CHROME_PID 2>/dev/null || true
ls -la "$OUT"
