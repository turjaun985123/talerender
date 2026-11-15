# TaleRender V2 — Streaming and output (as implemented + contract)

## 1. The three output modes — status first

Master prompt §8 / contract C3 define three modes. Their real
state at HEAD `712a6aa`:

| Mode | State |
|---|---|
| **PNG sequence** | **Implemented and the only wired mode.** `--film` writes `frame_%05d.png` via `film::run_film`; single scenes write one PNG. |
| **Direct streaming** | **Modules exist, not wired.** `src/streamq.rs` + `src/ffstream.rs` are registered in `main.rs` and unit-tested, but no render loop constructs a queue or an `FfmpegSink`, and there is no `--output-mode` flag in the CLI (see `CLI.md`). |
| **Auto** | **Landing in this wave.** `src/streammodes.rs` (764 lines) landed during this wave as an untracked seat file, not yet registered in `main.rs`: `OutputMode { Png, Stream, Auto }`, `parse_mode` (`png\|stream\|auto`, trimmed, ASCII case-insensitive), `mode_from_config` (total; unrecognised → Png, the safe direction), and `resolve_mode(requested, ffmpeg_on_path) -> ResolvedMode { mode, reason }` — Auto never survives resolution: it becomes Stream iff ffmpeg is on PATH, else Png, and `reason` says so in words for the caller to **print — never silent**. The probe it uses already exists: `ffstream::ffmpeg_available()`. The module also carries stream-session planning, interrupted-stream recovery policy, and the queue drain loop. None of it is wired into a render path yet. |

`config.rs` already validates an `output_mode` value of
`png | stream | auto` (default `png`) at all four precedence
layers, but nothing consumes the resolved value yet
(`CONFIG.md`).

## 2. PNG mode — the production path today

- `run_film(width, height, film_dir, out_dir, from, to, knobs)`:
  frames `[from, to)` (`to = 0` means through the end;
  total = `ceil(duration × 30)`), each evaluated deterministically
  (`eval_frame(t)`, `t = n / 30`), rendered, composited
  (calendar, fade, caption), and saved as
  `{out_dir}/frame_{n:05}.png`.
- **Resume:** a frame whose PNG already exists is skipped and
  counted as skipped — the existing file is the resume record.
  Progress prints every 25 rendered frames; the completion line
  reports rendered/skipped counts, the runlog summary, and the
  `paint-cache:` stats, followed by the `gpu-mem:` line.
- No frame accumulation: one frame is in flight at a time;
  readback → PNG → next frame. This already satisfies the
  "no full-film RAM accumulation" law on the PNG path.

## 3. Queue design — `src/streamq.rs` (WP4, C3 queue half)

- `FramePacket { index: u32, width: u32, height: u32,
  rgba: Vec<u8> }` — tight RGBA8 rows, exactly the shape
  `gpucore::Readback::collect` returns after padding is
  stripped. `validate()` checks `rgba.len() == width × height
  × 4`; `byte_len()` reports the payload length.
- `frame_queue(capacity) -> (SyncSender<FramePacket>,
  Receiver<FramePacket>)` over `std::sync::mpsc::sync_channel`
  — **bounded always**. The producer *blocks* on a full queue
  (backpressure); frames are never dropped and never accumulate
  unboundedly. Capacity is meant to come from C1
  (`MemPressure::frame_queue_capacity()`: LowMemory 2 /
  Balanced 4 / HighPerformance 8) at the call site. Capacity 0
  is rejected: a `debug_assert!` in debug builds, clamped to 1
  in all builds, so no rendezvous channel can appear silently.
- `PacketPool`: bounded reuse of the frame buffers themselves
  (one stream = one fixed W×H). Reuse requires *exactly* the
  needed capacity; oversized/undersized buffers are dropped,
  never reshaped. Retention cap from the constructor; cap 0
  disables pooling. Single-owner (`&mut self`); only packets
  and `StreamStats` cross threads.
- `StreamStats`: `AtomicU64 { pushed, popped, bytes }` with
  `snapshot()`; bytes are counted once, at push. Relaxed
  ordering — accounting for logs/finalisation, not
  synchronisation. Final pushed-vs-popped-vs-written
  reconciliation is the sink's close-out job.
- Known limitation (module's own note): `Readback::collect`
  still allocates its own `Vec` via `strip_padding`, so
  `PacketPool` has no production caller until the lead lands
  the producer wiring.

## 4. Sink design — `src/ffstream.rs` (WP4, C3 sink half)

- `ffmpeg_available() -> bool`: PATH probe without spawning a
  process or a shell (splits `$PATH`, stat-checks for an
  executable `ffmpeg`; `ffmpeg.exe` also accepted).
- `build_args(w, h, fps, crf, preset, out_part)` is a pure argv
  builder, unit-tested byte-for-byte:
  `-y -f rawvideo -pix_fmt rgba -s WxH -r FPS -i - … -c:v
  libx264 -crf <CRF> -preset <preset> -pix_fmt yuv420p <out_part>`
  (input on stdin; `-y` leads so a stale `.part` is replaced).
- `FfmpegSink::spawn(out_part, w, h, fps)` uses the production
  defaults `DEFAULT_CRF = 18`, `DEFAULT_PRESET = "slow"`;
  `with_settings(.., crf, preset)` is the explicit variant
  (e.g. a draft's 23 / veryfast). Zero width/height/fps is
  rejected before any spawn attempt.
- `write_frame(rgba)`: validates the exact byte length *before*
  writing, writes all bytes to the child's stdin, and counts
  the frame only after the write succeeds.
- `finish(expected_frames) -> TaleResult<FinishReport>`: drops
  stdin (EOF = ffmpeg's flush/finalise signal), waits for exit,
  joins the stderr-drain thread. Non-zero exit **or**
  `frames_written != expected_frames` is a `TaleError::render`
  (category `render`, exit 3) with the stderr tail attached
  (last 4096 chars). On success it returns
  `FinishReport { frames_written, stderr_tail }`.
- stderr is piped and drained on a dedicated thread from spawn
  to finish (a chatty ffmpeg can never deadlock the render loop
  on a full pipe); stdout is null. Dropping a sink without
  `finish()` kills and reaps the child — no orphan ffmpeg is
  left holding the `.part` file.

## 5. `.part` discipline

Project law, in both the existing and the streaming paths:
output is written to a temporary `.part` name and only becomes
the final file after a fully successful encode.

- Existing: `film/finish.sh` writes `${OUT}.part.mp4`
  (`rm -f` first), and `mv`s it to `$OUT` only after ffmpeg
  exits cleanly under `set -euo pipefail`. A failed finish
  never leaves a plausible final file.
- Streaming (designed): the sink encodes **video only** to the
  caller-supplied `<out>.part.mp4`; only after `finish()`
  returns Ok does the caller run the finishing step (rename +
  mux). The sink itself never muxes, never renames, and never
  deletes output files (a Drop-killed sink leaves its `.part`
  on disk for the caller).

## 6. Finishing contract — what streaming must not duplicate

`film/finish.sh FRAMES_DIR MIX_WAV OUT_MP4` is the production
finishing contract, unchanged by V2:

```
ffmpeg -y -v error -stats \
  -framerate 30 -i "$FRAMES/frame_%05d.png" -i "$MIX" \
  -map 0:v:0 -map 1:a:0 \
  -c:v libx264 -preset slow -crf 18 -pix_fmt yuv420p \
  -color_primaries bt709 -color_trc bt709 -colorspace bt709 \
  -x264-params "colorprim=bt709:transfer=bt709:colormatrix=bt709" \
  -c:a aac -b:a 256k -movflags +faststart "$PART"
mv "$PART" "$OUT"
```

So the finishing step owns: the `data/mix.wav` mux (audio
unchanged, AAC encode only), BT.709 colour tags *and* the x264
VUI params (both are passed because, with ffmpeg 8.1.2, the
`-color_*` options alone land only colorspace in the MOV colr
atom — the header of `finish.sh` records this), `+faststart`,
and the final rename. The streaming sink's argv deliberately
matches only the video encode (rawvideo in, libx264 CRF/preset,
yuv420p out): wiring a streamed `.part` into finishing means a
stream-copy mux invocation, which is the lead's adaptation at
integration — it does not exist yet. Final encode remains
x264 CRF 18 / preset slow (production law); no mode may
silently switch quality or encoding parameters.

Frame-count accounting caveat (sink's own note): the sink
counts frames *written to stdin* against the caller's expected
count; it does not ffprobe the encoded file. With constant-rate
rawvideo input ffmpeg neither drops nor duplicates frames, and
a short write surfaces as a broken-pipe error — but the encoded
file's frame count is **not independently verified** by any
code in the tree today.
