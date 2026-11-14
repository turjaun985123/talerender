#!/usr/bin/env bash
# Finish the native Onoda film (production integration).
#
# Frames: clean PNG masters from TaleRender --film (renderer:
# lavapipe on this VM, RADV on the user's PC — see
# docs/integration/README.md). Audio: the project's existing
# data/mix.wav, muxed unchanged (AAC encode only). Settings are the
# pipeline's own finishing settings: x264 slow CRF 18, BT.709 tags,
# faststart, .part discipline (write to *.part.mp4, rename on
# success — a failed finish never leaves a plausible file).
# Tag note (found by the wave's MP4 evidence, verified on a 3 s
# test clip): with ffmpeg 8.1.2 the -color_* options alone land
# only colorspace in the MOV colr atom (primaries/transfer probe
# as "unknown"); the -x264-params VUI values make all three read
# bt709 in ffprobe. Both are passed; they agree.
#
# Usage: film/finish.sh FRAMES_DIR MIX_WAV OUT_MP4
set -euo pipefail
FRAMES=${1:?frames dir}
MIX=${2:?mix wav}
OUT=${3:?out mp4}
PART="${OUT}.part.mp4"
rm -f "$PART"
ffmpeg -y -v error -stats \
  -framerate 30 -i "$FRAMES/frame_%05d.png" \
  -i "$MIX" \
  -map 0:v:0 -map 1:a:0 \
  -c:v libx264 -preset slow -crf 18 -pix_fmt yuv420p \
  -color_primaries bt709 -color_trc bt709 -colorspace bt709 \
  -x264-params "colorprim=bt709:transfer=bt709:colormatrix=bt709" \
  -c:a aac -b:a 256k \
  -movflags +faststart "$PART"
mv "$PART" "$OUT"
echo "finished: $OUT"
