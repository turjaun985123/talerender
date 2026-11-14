# Structured errors — wiring note (wave-2 seat W05)

**Deliverable:** `src/errors.rs` — `TaleError` + `TaleResult<T>`, ready
to wire in. Nothing else was touched; nothing is committed.

**Verification:** the module compiles standalone (rustc 1.99.1,
edition 2021) and its **10 unit tests pass** (Display text for every
variant, the full exit-code mapping, the `From<image::ImageError>`
conversion, String/io source ergonomics). The scratch check used a
stand-in for the `image` crate exposing exactly the API surface the
module touches (`ImageError: Display + Error + From<io::Error>` —
all true of image 0.25). First compile against the real crate
happens in CI the moment `mod errors;` lands — run CI before
starting the call-site migration to confirm.

## 1. Wiring (one line)

In `src/main.rs`, insert among the module declarations (currently
lines 18–27), alphabetically between `mod dof;` and `mod math;`:

```rust
mod errors;
```

That is the only wiring the module needs. Call sites then use
`crate::errors::{TaleError, TaleResult}`. No new dependencies
(std + the existing `image` crate only).

Top-level handling is the lead's call, but the intended shape is:
`main`'s mode entry points return `TaleResult<()>`, and `main`
prints `error: {e}` to stderr and exits with `e.exit_code()`.
CLI parse errors keep going through the existing `cli_error`
(exit 2) — they are already professional; do not reroute them.

**Do not migrate the telemetry verdict path.**
`film::run_film_telemetry` returning its verdict as a printed line
with exit 0 is a *contract* (the skill adapter keys on the line).
Exit code 1 in the audit's table belongs to that verdict alone;
no `TaleError` variant maps to it, by design.

## 2. The type, briefly

| Variant | Carries | Exit |
|---|---|---|
| `GpuInit { context, source }` | adapter/instance request site + cause | 3 |
| `DeviceRequest { context, source }` | `request_device` site + cause | 3 |
| `BufferMap { context, source }` | readback site + cause | 3 |
| `Render { context, source }` | pass/submission site + cause | 3 |
| `ImageIo { path: Option, source }` | frame/asset path + cause | 4 |
| `DataLoad { path: Option, source }` | scene/film file + cause | 2 |

Constructors (`gpu_init`, `device_request`, `buffer_map`, `render`,
`image_io`, `image_io_msg`, `data_load`, `data_load_msg`) take
`impl Display` for the source, so the loaders' existing `String`
errors and wgpu's error types slot in directly.

**No blanket `From<std::io::Error>` — deliberate.** An io failure
*reading* a scene file is `DataLoad` (exit 2); one *writing* a
frame is `ImageIo` (exit 4). A blanket conversion would silently
misclassify one of them. `From<image::ImageError>` exists because
image failures are unambiguously `ImageIo`.

## 3. Panic-site inventory (grep of `src/`, 2026-11-14)

Method: `grep -n -E '\.expect\(|panic!\(|\.unwrap\(\)' src/*.rs` —
**112 line-sites**: **27 runtime-migrate**, **57 acceptable-invariant**,
**28 test code**. (The staging copies under `painters/` mirror
`scene.rs` painters for the test harness and are not inventoried;
production is `src/`.) File:line below is exact at HEAD `d7bebdd`.

### 3a. CLI — already handled (0 sites)

`main.rs` argument parsing no longer uses `expect` at all (fixed in
`d7bebdd`: `cli_error`, exit 2, help hint). Nothing to do.

### 3b. Runtime-migrate — 27 sites, in migration order

**Step 1 — film loading + image save first** (user-visible failures:
missing/corrupt project data, unwritable output):

| Site | Today | Target |
|---|---|---|
| `film.rs:3456` | `load_film_data(dir).unwrap_or_else(\|e\| panic!("film data: {e}"))` | `DataLoad` (film dir) — `load_film_data` already returns `Result<_, String>`; becomes `TaleResult` |
| `film.rs:3457` | `load_assets(...).unwrap_or_else(\|e\| panic!("film assets: {e}"))` | `DataLoad` — same |
| `film.rs:3462` | `create_dir_all(out_dir).expect("cannot create frames dir")` | `ImageIo` (frames dir) |
| `film.rs:3489` | `img.save(&path).unwrap_or_else(\|e\| panic!(...))` | `ImageIo` (frame path) |
| `scenefile.rs:1233` | `load(path).unwrap_or_else(\|e\| panic!("{e}"))` | `DataLoad` (scene path) — `load` already returns `Result<_, String>` |
| `scenefile.rs:1536` | `img.save(out).expect("failed to save PNG")` | `ImageIo` |
| `scenefile.rs:2240` | `image::save_buffer(...).expect("write PNG")` | `ImageIo` |
| `main.rs:852` | `img.save(out).expect("failed to save PNG")` | `ImageIo` |
| `ab1.rs:1590` | `img.save(out).expect("failed to save PNG")` | `ImageIo` |
| `ab1.rs:689` | caption strip load `panic!("cannot load caption strip {caption_path}: {e}")` | `DataLoad` (caption asset path) |
| `dof.rs:555` | `image::save_buffer(...).expect("write PNG")` | `ImageIo` |
| `battery.rs:34` | `img.save(path).expect("failed to save PNG")` | `ImageIo` (QA tool — do last in this step) |
| `battery.rs:84` | `create_dir_all(out_dir).expect("cannot create out dir")` | `ImageIo` |

**Step 2 — GPU init / device request** (the "no adapter" and
"device lost" failures a user on a new machine actually hits):

| Site | Today | Target |
|---|---|---|
| `main.rs:299` | `request_adapter(...).expect("no wgpu adapter found — …")` (in `request_adapter` helper; the Option is the failure) | `GpuInit` |
| `main.rs:463` | `.expect("request_device failed")` | `DeviceRequest` |
| `ab1.rs:619` | `.expect("request_device failed")` | `DeviceRequest` |
| `dof.rs:178` | `.expect("request device")` | `DeviceRequest` |
| `scenefile.rs:1158` | `.expect("request_device failed")` (in `make_device`) | `DeviceRequest` |
| `film.rs:2375` | `.expect("request_device failed")` | `DeviceRequest` |

Note the duplicated device-setup code across these files (audit
finding B): if the shared GPU core lands first, steps 2–3 collapse
into it. Migrating per-file first is also fine — the variants are
the contract either way.

**Step 3 — buffer-map readbacks.** Each site is the two-layer
`rx.recv().expect("map channel closed").expect("buffer map failed")`;
both layers become `BufferMap` (source text distinguishes them):

`main.rs:840` · `ab1.rs:1578` · `dof.rs:545` · `dof.rs:623` ·
`scenefile.rs:1524` · `scenefile.rs:2230` · `film.rs:3313`

**Step 4 — authored-data parsing:**

| Site | Today | Target |
|---|---|---|
| `scene.rs:31` | `hex_rgb()`: `from_str_radix(...).expect("bad hex colour")` | `DataLoad` (no path — `data_load_msg`). Ripple: `hex_rgb` returns `(u8, u8, u8)` today and has many painter callers; add a fallible sibling or change the signature — lead's call, hence last. |

### 3c. Acceptable-invariant — 57 sites (leave as panics)

A panic here means a programming error was caught, not a runtime
condition mishandled:

- **Bind-group `Option` unwraps (11)** — `ab1.rs:1369, 1419`;
  `film.rs:3132, 3139, 3172, 3179, 3186, 3220, 3229, 3237, 3249`.
  The fields are populated during pipeline construction, before
  any pass records them; `None` is unreachable unless the
  construction code changes.
- **`RgbaImage::from_raw` expects (5)** — `main.rs:851`,
  `ab1.rs:1589`, `scenefile.rs:1535`, `film.rs:3488`,
  `battery.rs:33`. The buffer is sized `width × height × 4` by
  construction immediately above each site.
- **`film.rs:2125`** — `order.pop_front().expect("cache order")`:
  the eviction queue is pushed in lockstep with cache insertion;
  non-empty whenever eviction runs.
- **`scenefile.rs:1594`** — `scene.bokeh.as_ref().expect("composer
  path requires post.bokeh")`: `run_composer` is only entered from
  the branch that already matched `post.bokeh` present.
- **`telemetry.rs:874`** — `comp_shots.last_mut().unwrap()`
  immediately after `push` in the same branch.
- **Battery spec parsing (38)** — `battery.rs` lines 20, 24, 29,
  45, 50, 55, 61, 63, 79–82, 89, 92, 93, 97, 98, 101, 102, 105,
  106, 109, 110, 113, 114, 117, 118, 119, 126, 128, 134, 136, 137,
  138, 144–147. Every one parses the compile-time-embedded `SPEC`
  constant of a QA tool; a failure is a build defect the battery's
  own CI run exposes, never user input.

### 3d. Test code — 28 sites (leave)

`scene.rs:1772–1773`; `scenefile.rs:2268–2435` (26 sites). Panics
are the correct failure mechanism inside `#[cfg(test)]`.

## 4. Migration order, summarised

1. Add `mod errors;` → CI (proves the real-crate compile + tests).
2. Step 1 (film loading + image save) — highest user visibility,
   smallest ripple: two loaders already return `Result<_, String>`.
3. Step 2 (GPU init/device), then Step 3 (buffer maps) — or fold
   both into the shared GPU core if finding B's refactor goes first.
4. Step 4 (`hex_rgb`) last — widest caller ripple.
5. After each step, CI: the suite must stay green and the film
   telemetry verdict line must remain byte-identical in behaviour.

*Seat W05 — module + inventory only. No other file modified,
nothing committed.*
