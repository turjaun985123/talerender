# RETIRED (Chrome-replacement wave, 2026-11-15): Chrome-era A/B evidence harness. Do not run — the native pipeline has no browser path. Kept with the docs/ab evidence it produced.
#!/usr/bin/env bash
# Reproduce the Chrome/Skia reference cards for the M4 stroke battery.
# Copies the shipped skill template to a scratch dir, drops in
# battery.ts + battery.html + the package spec, builds with bun,
# serves it locally and drives headless Chrome at it; the page POSTs
# the finished cards back to the server (project/serve.mjs).
# Usage: bash run-reference.sh <out-dir>
set -euo pipefail
PKG="$(cd "$(dirname "$0")/.." && pwd)"
TEMPLATE="$HOME/workspace/custom-skills/tall-tale-shorts/template"
CHROME="${CHROME:-$HOME/.faceless-shorts/chrome/opt/google/chrome/chrome}"
OUT="$(mkdir -p "$1" && cd "$1" && pwd)"
WORK="$(mktemp -d /tmp/stroke-battery-ref.XXXXXX)"
echo "workdir: $WORK  out: $OUT"
rsync -a --exclude node_modules --exclude build "$TEMPLATE/" "$WORK/"
cp "$PKG/project/battery.ts" "$WORK/src/battery.ts"
cp "$PKG/project/battery.html" "$WORK/battery.html"
cp "$PKG/spec.json" "$WORK/battery-spec.json"
cd "$WORK"
bun build src/battery.ts --outdir build --target browser --external three --external "three/addons/*"
node "$PKG/project/serve.mjs" "$OUT" 8971 &
SERVER=$!
sleep 1
"$CHROME" --headless=new --no-sandbox --ignore-gpu-blocklist \
  --enable-unsafe-swiftshader --use-angle=swiftshader \
  --disable-background-timer-throttling --disable-renderer-backgrounding \
  --window-size=1280,900 "http://127.0.0.1:8971/battery.html" &
CHROME_PID=$!
wait $SERVER || true
kill $CHROME_PID 2>/dev/null || true
ls -la "$OUT"
