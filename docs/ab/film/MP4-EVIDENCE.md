# MP4-EVIDENCE — finished-film technical record (TEMPLATE)

Fill this in from the actual finished MP4 — every field measured
on the file, none copied from expectations. The expectations
column states what the pipeline as built must produce; a
mismatch is a finding, recorded as-is, not edited away.

**File under test:** _path, size, sha256 — pending_
**Produced by:** `film/finish.sh <frames-dir> <project>/data/mix.wav <out>.mp4`
**Frames renderer:** _CI lavapipe / PC RADV — pending_
**Binary commit:** _pending_ · **Date:** _pending_

## The finish, as built (film/finish.sh)

x264 `-preset slow -crf 18`, `-pix_fmt yuv420p`, BT.709 tags
(`-color_primaries/-color_trc/-colorspace bt709`), audio AAC
`-b:a 192k` from the project's `data/mix.wav` **muxed unchanged**
(no gain stage, no re-mix), `-movflags +faststart`, and `.part`
discipline: ffmpeg writes `<out>.part.mp4`; the rename to the
final name happens only on success.

## Stream record (ffprobe)

```
ffprobe -v error -show_streams -show_format \
  -of default=noprint_wrappers=1 <out>.mp4
```

| field | expected | measured |
|---|---|---|
| video codec | `h264` (libx264, slow, CRF 18) | _pending_ |
| dimensions | 1080×1920 | _pending_ |
| pix_fmt | `yuv420p` | _pending_ |
| frame rate | `r_frame_rate` / `avg_frame_rate` = 30/1 | _pending_ |
| colour tags | primaries / trc / space all `bt709` | _pending_ |
| video frames (`nb_frames`) | **1811** | _pending_ |
| audio codec | `aac`, ≈192 kb/s | _pending_ |
| audio sample rate / channels | as the source mix (record actuals) | _pending_ |
| format duration | ≈ 60.46 s (see duration note) | _pending_ |
| faststart | `moov` before `mdat` (see check below) | _pending_ |

**Duration note.** Three durations are in play and must not be
conflated: the narration span is **60.357 s** (film.json
`durationS`); the video stream is 1811 frames ÷ 30 fps =
**60.3667 s**; the audio master is **60.46 s** (film.json audio
block). finish.sh passes no `-shortest`, so the container
duration follows the longest stream — expect ≈ **60.46 s**. A
container duration near 60.36 s would mean the audio was
trimmed; near 60.37 s with missing audio tail is likewise a
finding.

**Frame-count check** (header fields can lie; count the frames):

```
ffprobe -v error -select_streams v:0 -count_frames \
  -show_entries stream=nb_read_frames -of csv=p=0 <out>.mp4
# expected: 1811
```

**Faststart check** (box order in the file head):

```
python3 - <out>.mp4 <<'EOF'
import struct, sys
f = open(sys.argv[1], 'rb')
order = []
while True:
    h = f.read(8)
    if len(h) < 8: break
    size, typ = struct.unpack('>I4s', h)
    order.append(typ.decode())
    if size < 8: break
    f.seek(size - 8, 1)
print(order[:6], '-> faststart' if order.index('moov') < order.index('mdat') else '-> NOT faststart')
EOF
```

## Loudness (measured on the mux, not the mix)

The mix is the project master at **−14.0 LUFS integrated**
(film.json audio block, muxed unchanged). The MP4's audio is an
AAC encode of that master with no gain stage, so the mux must
measure the same within encode tolerance. Measure **both** files
with the same command so the comparison is apples-to-apples:

```
ffmpeg -i <file> -vn -af loudnorm=print_format=json -f null - 2>&1 | tail -12
# or: ffmpeg -i <file> -vn -af ebur128=peak=true -f null -
```

| measurement | mix.wav (reference) | finished MP4 | verdict |
|---|---|---|---|
| integrated loudness | _measure_ (≈ −14.0 LUFS expected) | _pending_ (must match mix within ≈ 0.3 LU) | _pending_ |
| true peak | _measure_ | _pending_ (no clipping introduced by the encode) | _pending_ |
| LRA | _measure_ | _pending_ | _pending_ |

A loudness mismatch beyond tolerance means the wrong audio file
was muxed or a gain stage crept in — the video evidence stands,
the MP4 does not ship until this passes.

## .part discipline

- During the finish, only `<out>.part.mp4` exists. The final
  filename appears **only** via the rename on success.
- A `.part.mp4` left behind = a failed or aborted finish. It is
  never evidence, never renamed by hand, and its presence is
  recorded here if observed: _none observed / pending_.
- Verify the file under test is the renamed output, not a
  `.part` copied into place (sha256 above is of the final name).

## Sign-off checklist

- [ ] ffprobe table filled; all expected fields match
- [ ] counted frames = 1811
- [ ] container duration ≈ 60.46 s, explained by the audio master
- [ ] loudness on the mux matches the mix (integrated + true peak)
- [ ] faststart confirmed by box order
- [ ] no `.part` file presented as, or confused with, the output
- [ ] renderer + binary commit recorded (grain is vendor-sensitive;
      the A/B package's label and this file's must agree)
