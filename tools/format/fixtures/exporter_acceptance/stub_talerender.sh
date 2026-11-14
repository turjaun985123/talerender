#!/bin/sh
# Stub talerender binary for exporter_acceptance.py --self-test.
# "Renders" by copying the planted frame <stem>.stub.png that sits
# beside the scene file to the --out path. Exercises the harness's
# classification logic end to end with no GPU and no real binary.
scene=""
out=""
while [ $# -gt 0 ]; do
  case "$1" in
    --scene-file) scene="$2"; shift 2 ;;
    --out) out="$2"; shift 2 ;;
    --width|--height) shift 2 ;;
    *) shift ;;
  esac
done
if [ -z "$scene" ] || [ -z "$out" ]; then
  echo "stub: --scene-file and --out required" >&2
  exit 2
fi
dir=$(dirname "$scene")
base=$(basename "$scene" .scene.json)
cp "$dir/$base.stub.png" "$out"
