# MP4-EVIDENCE — finished-film technical record (FILLED, W84)

Filled from the actual finished MP4 — every field measured on
the file, none copied from expectations. One mismatch was found
(colour tags) and is recorded as a finding, not edited away.

**File under test:** `~/workspace/your_files/onoda-native-381ff47.mp4`
· **29,722,802 bytes** · sha256
`f357a00434e4a8dcc4b78607a83b8010c159df86bf81b0bcb8f9cf9b00aecaed`
**Produced by:** `film/finish.sh ~/workspace/.w84-scratch/frames
~/faceless-shorts/the-soldier-who-didnt-know/data/mix.wav
~/workspace/your_files/onoda-native-381ff47.mp4` (ffmpeg 8.1.2,
this VM; encode elapsed 15 min 25 s at ≈ 2.0 fps, detached run,
log `~/workspace/.w84-scratch/finish.log`)
**Frames renderer:** **CI lavapipe** — GitHub ubuntu-22.04,
"Render (dispatch)" workflow, mode `film`, nine chunk runs at
head `381ff47` (run ids + artifact inventory in `RESULTS.md`
beside this file; artifacts expire ~2026-11-21).
**Binary commit:** frames rendered by talerender at repo head
`381ff47` · **Date:** 2026-11-14 (finish run 08:47–09:02 UTC)

**Provenance label.** Native CI lavapipe frames @`381ff47` +
the project's existing `data/mix.wav` muxed unchanged. This is
a **technical-proof MP4, not a channel master** — the channel
path remains a RADV render of the same frames on the user's PC
(grain is vendor-sensitive; see the package README).

**Frame assembly (before the finish).** The 1,811 frames were
assembled into `~/workspace/.w84-scratch/frames` from the nine
chunk artifacts (downloaded copies under `~/workspace/dl2-<run>/`,
hardlinked into the assembly dir). Verified first-hand:
count = **1,811**; the set is exactly `frame_00000.png` …
`frame_01810.png` — no gaps, no extras; dimensions spot-checked
1080×1920 on frames 0 / 905 / 1810; sha256 of the s01 mid
(frame 89, `ebcfeba1…`) and s12 mid (frame 1738, `1978d6e6…`)
match the golden-mid table in `RESULTS.md`, i.e. the encoded
frames are byte-identical to the frames W77 measured.

## The finish, as built (film/finish.sh)

x264 `-preset slow -crf 18`, `-pix_fmt yuv420p`, BT.709 tags
(`-color_primaries/-color_trc/-colorspace bt709`), audio AAC
`-b:a 192k` from the project's `data/mix.wav` **muxed unchanged**
(no gain stage, no re-mix), `-movflags +faststart`, and `.part`
discipline: ffmpeg writes `<out>.part.mp4`; the rename to the
final name happens only on success. The invocation was followed
verbatim — no settings adapted.

## Stream record (ffprobe)

```
ffprobe -v error -show_streams -show_format \
  -of default=noprint_wrappers=1 <out>.mp4
```

| field | expected | measured |
|---|---|---|
| video codec | `h264` (libx264, slow, CRF 18) | `h264`, High profile, level 5.0, encoder tag `Lavc62.28.102 libx264`; mean video bitrate 3,737,567 b/s |
| dimensions | 1080×1920 | 1080×1920 |
| pix_fmt | `yuv420p` | `yuv420p` (colour range `tv`) |
| frame rate | `r_frame_rate` / `avg_frame_rate` = 30/1 | 30/1 / 30/1 |
| colour tags | primaries / trc / space all `bt709` | space `bt709`; **primaries `unknown`; trc `unknown`** — see Finding F1 |
| video frames (`nb_frames`) | **1811** | **1811** |
| audio codec | `aac`, ≈192 kb/s | `aac` LC, 192,073 b/s |
| audio sample rate / channels | as the source mix (record actuals) | 48,000 Hz, stereo — source mix is `pcm_s16le`, 48,000 Hz, stereo: unchanged |
| format duration | ≈ 60.46 s (see duration note) | **60.457 s** (video stream 60.366667 s = 1811 ÷ 30 exactly; audio stream 60.457 s) |
| faststart | `moov` before `mdat` (see check below) | box order `ftyp, moov, free, mdat` → **faststart confirmed** |

**Finding F1 — colour tags partially landed.** finish.sh passes
all three BT.709 flags, but the finished file carries only the
matrix: `color_space=bt709` with `color_primaries=unknown` and
`color_transfer=unknown`, confirmed both in the stream record
and in the decoded frames' own colour fields. The video is
otherwise exactly as specified. Recorded as-is for the lead:
either the flag set needs a companion (e.g. x264 VUI params) on
this ffmpeg, or the expectation column needs amending — the
file was NOT re-encoded or hand-patched by this seat.

**Duration note.** Three durations are in play and must not be
conflated: the narration span is **60.357 s** (film.json
`durationS`); the video stream is 1811 frames ÷ 30 fps =
**60.3667 s**; the audio master is **60.46 s** (film.json audio
block). finish.sh passes no `-shortest`, so the container
duration follows the longest stream — expect ≈ **60.46 s**. A
container duration near 60.36 s would mean the audio was
trimmed; near 60.37 s with missing audio tail is likewise a
finding. **Measured: 60.457 s — the audio master governs, as
built; no trimming.**

**Frame-count check** (header fields can lie; count the frames):

```
ffprobe -v error -select_streams v:0 -count_frames \
  -show_entries stream=nb_read_frames -of csv=p=0 <out>.mp4
# expected: 1811
```

**Measured: `nb_read_frames` = 1811.** ✓

**Faststart check** (box order in the file head): the template's
python box walk prints `['ftyp', 'moov', 'free', 'mdat'] ->
faststart`. ✓

## Loudness (measured on the mux, not the mix)

The mix is the project master at **−14.0 LUFS integrated**
(film.json audio block, muxed unchanged). The MP4's audio is an
AAC encode of that master with no gain stage, so the mux must
measure the same within encode tolerance. Both files were
measured with the same command:

```
ffmpeg -i <file> -vn -af loudnorm=print_format=json -f null -
```

| measurement | mix.wav (reference) | finished MP4 | verdict |
|---|---|---|---|
| integrated loudness | **−13.99 LUFS** | **−14.00 LUFS** | Δ 0.01 LU — match (tolerance ≈ 0.3 LU) ✓ |
| true peak | **−1.65 dBTP** | **−1.58 dBTP** | +0.07 dB AAC overshoot; no clipping ✓ |
| LRA | **1.40 LU** | **1.40 LU** | identical ✓ |

The muxed audio is the project mix, unchanged: same integrated
loudness, same LRA, true peak within encode overshoot.

## .part discipline

- During the finish, only
  `~/workspace/your_files/onoda-native-381ff47.mp4.part.mp4`
  existed (observed across the encode, 08:47–09:02 UTC). The
  final filename appeared **only** via finish.sh's rename on
  success.
- A `.part.mp4` left behind = a failed or aborted finish. It is
  never evidence, never renamed by hand, and its presence is
  recorded here if observed: **none — no `.part` file remains.**
- The file under test is the renamed output of finish.sh, not a
  `.part` copied into place (sha256 above is of the final name).

## Sign-off checklist

- [x] ffprobe table filled — all expected fields match **except
      colour primaries/trc (Finding F1)**
- [x] counted frames = 1811
- [x] container duration 60.457 s, explained by the audio master
- [x] loudness on the mux matches the mix (integrated + true peak)
- [x] faststart confirmed by box order
- [x] no `.part` file presented as, or confused with, the output
- [x] renderer + binary commit recorded (CI lavapipe @`381ff47`;
      agrees with the A/B package's label in `RESULTS.md`)
