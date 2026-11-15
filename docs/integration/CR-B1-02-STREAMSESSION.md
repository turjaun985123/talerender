# CR-B1-02 — WP4 streaming session wiring (lead integration note)

Seat: Chrome-Replacement Wave, Batch 1, Seat 2.
Deliverables: `src/streamsession.rs` (new) + this note.
Base verified at HEAD `b093f3a`. No existing file was edited by
the seat; every change below is the lead's to make.

---

## 1. What exists now, verified in code

- `src/main.rs`, `--film` branch: prints the `config:` line,
  resolves the mode via `streammodes::mode_from_config` +
  `streammodes::resolve_mode`, prints
  `output-mode: <mode> — <reason>`, then — for a resolved
  Stream — `cli_error(...)` ("streaming output is not yet
  wired into the --film driver … use --output-mode png").
  The `MemPressure` construction + `mem-profile:` print sit
  AFTER that error block.
- `src/film.rs::run_film(width, height, film_dir, out_dir,
  from, to, knobs, adapter_policy)`: loads film data +
  assets, builds ONE `Renderer`, then loops `from..to`:
  skip-if-PNG-exists (resume), `eval_frame(t = n/30)`,
  `resolve_chars`, `renderer.render` → tight RGBA `px`,
  CPU overlays in main.ts order (calendar → fade → caption),
  `image::RgbaImage::from_raw(..).save(frame_%05d.png)`.
  `frames_total = ceil(duration * 30)`; fps is the literal
  30 everywhere in this driver.
- Landed pieces the session consumes verbatim:
  `streamq::frame_queue` / `StreamStats` / `PacketPool`,
  `streammodes::drain_queue` (the consumer loop),
  `streammodes::StreamPlan` (the `.part` spelling),
  `streammodes::recovery_decision`,
  `ffstream::expected_frame_bytes` / `ffmpeg_available` /
  `FinishReport`, `scheduler::plan` (see §6).
- `ffstream::FfmpegSink` is NOT the session's encoder. See
  §7, contradiction C1 — this is the one design decision in
  this landing that departs from the V2 seats' wiring notes,
  and it is forced by the finishing contract.

## 2. Wiring step 1 — mod line (main.rs)

In the mod block, after `mod streammodes;` (line 48):

```rust
mod streamsession;
```

That is the only change `streamsession.rs` needs to compile
and test. No `Cargo.toml` change (std only).

## 3. Wiring step 2 — main.rs `--film` branch

### 3.1 Delete the fail-loudly block, keep the mode print

Remove exactly this block:

```rust
        if resolved.mode == streammodes::OutputMode::Stream {
            cli_error(
                "output_mode=stream resolved, but streaming output is not yet wired \
                 into the --film driver (streamq/ffstream/streammodes are landed; \
                 the driver wiring is a follow-up) — use --output-mode png",
            );
        }
```

The mode print stays EXACTLY where it is, for both modes:

```rust
        println!("output-mode: {} — {}", resolved.mode.as_str(), resolved.reason);
```

That print already fires on PNG runs today, so keeping it
verbatim is part of PNG's byte-identical behaviour (§5).
Auto's fallback wording remains `resolve_mode`'s job.

### 3.2 Move the MemPressure construction above the dispatch

The session's queue capacity is C1's
`mp.frame_queue_capacity()`. Today `mp` is built after the
deleted block; move that construction (and its
`mem-profile:` print, unchanged) to just after the
`output-mode:` print, so the value exists at the
`run_film` call.

### 3.3 Pass the stream inputs into `run_film`

Extend the call (the only call site of `run_film`):

```rust
        exit_on_err(pollster::block_on(film::run_film(
            ew,
            eh,
            dir,
            &out,
            film_from,
            film_to,
            knobs,
            &cfg.adapter_policy,
            resolved.mode,
            mp.frame_queue_capacity(),
        )));
```

### 3.4 HELP text (same commit, lead's call on wording)

- `--output-mode` entry: delete "stream is not yet wired
  into the --film driver and fails loudly".
- `--out` entry: document that for `--film` in stream mode
  `--out` is the output MP4 FILE (PNG mode: the frames
  directory, unchanged).

## 4. Wiring step 3 — film.rs `run_film`

### 4.1 Signature

```rust
pub(crate) async fn run_film(
    width: u32, height: u32, film_dir: &str, out_dir: &str,
    from: u32, to: u32, knobs: crate::Knobs,
    adapter_policy: &str,
    output_mode: crate::streammodes::OutputMode,
    queue_capacity: usize,
) -> crate::errors::TaleResult<()>
```

### 4.2 The stream branch — placement is the whole point

Insert AFTER the existing setup lines (data, assets, looks,
`frames_total` / `to` resolution, `Renderer::new`) and
BEFORE `std::fs::create_dir_all(out_dir)` and the PNG loop.
Everything above the branch is shared, untouched code;
everything below it is the PNG path, untouched code. The
branch early-returns, so PNG execution cannot observe it.

```rust
    if output_mode == crate::streammodes::OutputMode::Stream {
        return run_film_stream(
            width, height, dir, &data, &assets, &looks,
            &mut renderer, from, to, knobs, out_dir, queue_capacity,
        );
    }
```

(If the lead prefers no new function, inline the same body;
the branch body below is written as a helper purely so the
PNG loop stays visually untouched in the diff.)

### 4.3 The branch body (new code in film.rs)

```rust
fn run_film_stream(
    width: u32, height: u32, film_dir: &Path, data: &FilmData,
    assets: &Assets, looks: &[(FullLook, bool, f64)],
    renderer: &mut Renderer, from: u32, to: u32,
    knobs: crate::Knobs, out_mp4: &str, queue_capacity: usize,
) -> crate::errors::TaleResult<()> {
    let tag = if knobs.is_draft() {
        format!(" {}", knobs.tag())
    } else {
        String::new()
    };
    // Audio discovery is the CALLER's (see §4.4): the first
    // existing candidate wins; none -> explicit Silent.
    let mix = [film_dir.join("data/mix.wav"),
               film_dir.join("../data/mix.wav")]
        .into_iter().find(|p| p.is_file());
    let audio = match &mix {
        Some(path) => crate::streamsession::AudioSpec::Mix {
            path: path.clone(),
            start_secs: from as f64 / 30.0,
            duration_secs: (to - from) as f64 / 30.0,
        },
        None => crate::streamsession::AudioSpec::Silent,
    };
    let spec = crate::streamsession::StreamSpec {
        out_mp4: PathBuf::from(out_mp4),
        width,
        height,
        fps: 30, // run_film's own rate: t = n / 30, frames_total = ceil(duration * 30)
        first_index: from,
        expected_frames: to - from,
        profile: if knobs.is_draft() {
            crate::streamsession::EncodeProfile::Draft
        } else {
            crate::streamsession::EncodeProfile::Master
        },
        audio,
        queue_capacity,
    };
    let mut cache = PaintCache::new();
    let clock = crate::runlog::RunClock::new();
    let mut done = 0u32;
    // The producer: run_film's loop body, minus skip + save.
    // Overlay order is main.ts order, as in the PNG loop.
    let producer = |n: u32| -> crate::errors::TaleResult<Vec<u8>> {
        let t = n as f64 / 30.0;
        let mut spec_f = eval_frame(data, looks, t);
        let (dyn_tex, _) = resolve_chars(&mut spec_f, looks, assets, &mut cache);
        let mut px = renderer.render(&spec_f, &dyn_tex)?;
        if let Some(cal) = &spec_f.calendar {
            overlay_calendar(&mut px, width, height, assets, cal);
        }
        if spec_f.fade > 0.0 {
            apply_fade(&mut px, spec_f.fade);
        }
        if let Some(ci) = spec_f.caption {
            overlay_caption(&mut px, width, height, assets, ci, spec_f.caption_scale);
        }
        done += 1;
        if done % 25 == 0 {
            println!(
                "  frame {n} (t = {t:.2}s) — {done} streamed · {}{tag}",
                clock.timing_suffix(done as u64, (to - from) as u64)
            );
        }
        Ok(px)
    };
    let report = crate::streamsession::run_session(&spec, producer)?;
    println!(
        "film stream complete: {} frames encoded, {} produced -> {} ({}, {}) · {} · {}{tag}",
        report.frames_encoded, report.frames_produced,
        report.out_final.display(), report.profile, report.audio,
        clock.summary_suffix(), cache.stats_line()
    );
    println!("{}", crate::gpucore::mem_stats_line());
    Ok(())
}
```

Borrow notes for the lead: `renderer`, `cache`, `assets`,
`data`, `looks` are all captured mutably/immutably by the
one closure and used nowhere else during the session — the
same borrow shape as today's loop body. `Renderer::render`
takes `&mut self` in effect through its existing call; if
the closure capture fights the borrow checker, hoist the
branch body inline in `run_film` where the loop already
owns these locals (the semantics are identical).

### 4.4 Decisions the lead owns at this wiring (named, not hidden)

1. **Mix discovery.** The session NEVER searches for a mix
   (a named-but-missing mix is a loud spec error). The two
   candidates above cover the two layouts in the tree:
   `<film_dir>/data/mix.wav` (film data packaged inside a
   project) and `<film_dir>/../data/mix.wav` (the contracts
   §0 layout, `<project>/native/` + `<project>/data/`). If
   the lead prefers an explicit `--mix <PATH>` CLI flag
   instead, pass it through the same `AudioSpec::Mix` —
   the session is indifferent to where the path came from.
2. **Encode profile from `knobs.is_draft()`.** This couples
   the ENCODE draft (CRF 23 / veryfast) to the RENDER draft
   knobs (res/aa/shadow). That matches the skill's `--draft`
   (one flag, both meanings in capture.mjs's video mode),
   but if the lead wants them separable, add the flag at
   the CLI and pass the profile down — the spec field is
   already explicit.
3. **`--out` semantics in stream mode** (§3.4): a file,
   not a directory. The PNG resume/skip machinery does not
   apply and is not consulted: existing PNGs in any frames
   directory are irrelevant to a stream run, and a stream
   run writes no PNGs.

## 5. Why PNG mode stays byte-identical

- The PNG loop, its `frame_{n:05}.png` path spelling, its
  skip-if-exists resume, its progress + completion lines:
  not one line changes. The stream branch sits before it
  and returns early; it allocates nothing on the PNG path.
- `main.rs`: the deleted block only ever fired for Stream;
  the `output-mode:` print and the `config:` / `mem-profile:`
  prints keep their exact text and relative order for PNG
  (the mem-profile print moves earlier in the branch, but
  it already printed before `run_film` was called — the
  sequence of lines a PNG run emits is unchanged).
- Frames are pure functions of their index (contract A4);
  the session is never constructed on the PNG path, so no
  frame byte can change. The determinism byte-probes remain
  the gate that proves it at integration.

## 6. Scheduler verdict (WP7)

Consumed in exactly one place: `streamsession::run_session`
builds its index list with
`scheduler::plan(FrameRange::new(first, first+expected),
&HashSet::new())`.

- The done set is EMPTY on purpose. Streaming has no
  resume: `streammodes::recovery_decision` discards an
  interrupted `.part` and restarts from frame 0 (a
  rawvideo-fed x264 stream cannot append — encoder state,
  headers and accounting died with the child). A "done"
  index inside a stream would be a GAP in the encoded
  film, so `run_session_inner` validates any plan it is
  handed (exact length, strictly +1 contiguous from
  `first_index`) and fails loudly before spawning anything.
  A unit test pins a gapped plan's rejection.
- NOT consumed: `batches` / `max_in_flight` (bounds
  concurrent RENDERING; there is one `Renderer`, and
  in-flight stream frames are already bounded by the C1
  queue capacity), checkpoint files (they exist to resume;
  a stream cannot), `CancelFlag` (no owner exists in
  `main.rs` today; adding signal handling is a CLI
  decision, not a session behaviour).

## 7. Contradictions found (verified against code)

- **C1 — `ffstream` cannot carry the finishing contract.**
  `ffstream::build_args` is video-only: no BT.709
  `-color_*`, no `-x264-params` VUI, no audio/AAC, no
  `+faststart`, no `-threads`, no `color_range`. Its own
  note defers finishing to "the existing finishing step",
  but for a stream there is no such step that works: the
  VUI must be written at encode time (finish.sh header;
  MP4-EVIDENCE finding F1 — `-color_*` alone landed only
  colorspace), and re-encoding a video-only `.part` would
  be a second x264 generation = a silent quality change.
  Resolution in `streamsession.rs`: `FullContractSink`
  spawns ONE ffmpeg with the adapter's full argv and
  rawvideo stdin — the shape `capture.mjs` video mode
  always used. `ffstream` stays as the video-only sink;
  the V2 seat notes that describe wiring `FfmpegSink`
  directly into `run_film` as the production stream path
  (ffstream/streammodes delivery notes) are superseded by
  this paragraph, and the lead should say so at integration.
- **C2 — Two finishing contracts disagree in details.**
  `film/finish.sh` vs the adapter (`capture_native.mjs`,
  mirroring `capture.mjs`): finish.sh has NO `-threads`,
  `-color_range`, `-shortest`, `-ar`; the adapter has all
  four; finish.sh REQUIRES the mix argument, the adapter
  falls back to a silent track; AAC is 256k in both master
  sets (an older capture.ts-era record says 192k master —
  stale). The task names the ADAPTER's contract; the
  session mirrors the adapter, argument-for-argument,
  pinned byte-for-byte in the `build_full_args` tests.
- **C3 — "PNG/RGBA bytes" (task wording).** `run_film`'s
  in-memory product is RGBA, never PNG bytes; PNG is only
  the disk serialisation. The producer interface is
  therefore index → tight RGBA. Feeding PNG bytes to a
  rawvideo sink would be a category error.
- **C4 — No Rust ffprobe validation existed.**
  ffstream's note: "the encoded file's frame count is not
  independently probed (no ffprobe here)". The repo's
  existing approach is the MP4-EVIDENCE ffprobe record;
  the session implements that probe in Rust
  (`-count_frames` / `nb_read_frames`) as its sixth,
  independent accounting check, run on the `.part` BEFORE
  rename. Consequence: stream mode requires ffprobe on
  PATH — preflighted before the first frame renders.
- **C5 — `docs/v2/STREAMING.md` is stale.** It claims there
  is "no `--output-mode` flag in the CLI" and that
  `streammodes.rs` is "not yet registered in main.rs";
  at `b093f3a` the flag exists, the module is registered,
  and main resolves + prints the mode. Harmless history,
  but nobody should wire from that doc.
- **C6 — fps is not a free parameter today.** The session
  takes `fps` explicitly, but `run_film`'s evaluator is
  hardwired to 30 (`t = n/30`). The wiring passes 30. If a
  future change wants another rate, the evaluator changes
  first — passing a different `spec.fps` alone would
  encode a 30 fps-evaluated film at a lied-about rate, and
  `validate_probe` would (correctly) pass it, since probe
  and spec would agree on the lie. Named here so it is a
  decision, not a trap.
- **C7 — `PacketPool` reuse is drain-side only.**
  `streammodes::drain_queue` releases packet buffers into
  a consumer-side pool that nothing acquires from (the
  producer's buffers come from `Readback::collect`, which
  allocates its own Vec — streamq's own known limitation).
  The session inherits this: the pool bounds retained
  buffers, it does not yet recycle them into the producer.
  No memory claim beyond "bounded" is made anywhere in
  this landing.

## 8. Failure-mode behaviour (what the user sees)

All errors are `TaleError::render` (exit 3) unless noted;
`main`'s existing `exit_on_err` prints `error: <msg>` and
exits with that code. In EVERY failure below: no final MP4
is created or overwritten, and the `.part` is removed
best-effort (a leftover, if removal itself fails, is
discarded by the next run's recovery decision).

| Failure | Detection | Behaviour |
|---|---|---|
| ffmpeg not on PATH | `run_session` preflight, before any frame | Loud error naming the remedy (`--output-mode png`). Note Auto never reaches here without ffmpeg — it resolves to PNG in main. |
| ffprobe not on PATH | Same preflight | Loud error; the session refuses to produce a file it cannot validate. |
| Stale `.part` from an interrupted run | `recovery_decision` at session start | DiscardPartial → removed; the encode restarts from frame `first_index`. Never resumed, never appended. |
| ffmpeg dies mid-stream | Consumer `write_frame` → EPIPE, or drain ends short | Drain error is returned; producer's blocked `send` unblocks (channel disconnect); sink dropped (Drop already killed/reaped the child); `.part` removed; error carries the write index + out_part. |
| ffmpeg exits non-zero at close | `FullContractSink::finish` status check | Error carries exit status, frames fed, and the last 4096 chars of stderr. |
| Producer (render) error at frame k | Producer loop | Abort flag set → consumer drains the queue, drops the sink WITHOUT finish (Drop kills ffmpeg) → the PRODUCER's error is returned verbatim (its own category/exit code preserved). |
| Producer returns wrong-length frame | `FramePacket::validate` before queueing | Loud error naming frame, actual and expected byte counts; frame never queued (a bad frame cannot desync the rawvideo stream). |
| Count mismatch (any stage) | Five-way reconciliation at close: produced = pushed = popped = drained = sink-written = expected | One error naming ALL six numbers; no rename. A sink whose `finish` under-reports is caught here even if its own finish passed (unit-tested). |
| Encoded file fails ffprobe validation | `validate_probe` on the `.part`: counted frames, dimensions, h264, fps, AAC present, duration within 2 frames + 0.30 s, Master colour all bt709 | One error naming every failed check; `.part` removed; no rename. A master whose BT.709 did not land (finding F1's shape) fails here, loudly. |
| Named mix file missing | `validate_spec`, before spawn | Loud error naming the path; NO silent fallback to the silent track (the caller named a mix — silence would be a silent product change). |
| Odd frame dimensions | `validate_spec` | Loud error; yuv420p cannot carry odd dimensions and the session never scales/crops silently. (main's `effective_size` already rounds drafts to even.) |
| Consumer thread panics | `join` error | Session aborts with a render error; `.part` removed; no promotion. |

## 9. Test evidence

CI remains the Rust gate for the real crate (the local cargo
registry holds only wgpu 29.0.1, so the wgpu-30 crate cannot
resolve offline here and no repo build was attempted, per
the seat rules). What WAS run, as a scratch check in /tmp
(repo untouched, scratch deleted afterwards): the real
`streamsession.rs` compiled together with the real std-only
modules it consumes (`streamq`, `ffstream`, `streammodes`,
`scheduler`, `scene`, `math`) against a minimal stub of
`errors.rs` (same constructors/categories/exit codes) and a
one-constant stub for `math.rs`'s single wgpu reference.
In that harness:

- `streamsession` suite: **15/15 passed**, in 7 separate
  runs (filtered, and 6× at `--test-threads=8`).
- Full harness suite (106 tests, all copied modules):
  **106/106 passed** on the clean re-run.
- The real-ffmpeg smoke RAN (not skipped — ffmpeg/ffprobe
  8.1.2 are on this VM's PATH): 6 frames of 64×64, Master
  profile, silent track → renamed MP4 present, `.part`
  gone, ffprobe counted exactly 6 frames, BT.709 fields
  validated on the real encode.
- One anomaly, reported honestly: in the FIRST full-suite
  run (which overlapped a stray background filesystem scan
  on this 1.5-CPU VM), the smoke failed once at its
  `run_session` expect; the error text was not captured.
  It did not reproduce in the clean full-suite re-run or in
  any of the 7 module-suite runs. No logic change was made
  on speculation; if CI ever shows it, the failing
  `TaleError` text is the first thing to capture.

## 10. What this landing does NOT do

- No PNG-path change of any kind (§5).
- No second mux pass and no re-encode (§7 C1).
- No resume for streams, by design (§6).
- No cancellation wiring (§6).
- No change to `ffstream`, `streamq`, `streammodes`,
  `scheduler`, or any other existing file.
