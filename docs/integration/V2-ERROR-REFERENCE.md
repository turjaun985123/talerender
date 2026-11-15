# V2 ERROR REFERENCE — `TaleError` categories, exit codes, recovery + observability event vocabulary

**Seat:** TaleRender V2, Batch 1 Seat 9 (WP12 — structured errors +
observability, contract C6 in `V2-CONTRACTS.md`; master prompt §16).
**Source of truth:** `src/errors.rs` as it stands in the tree. Every
variant, constructor, category string, exit code, and Display string
below was read from that file. Where any other document disagrees
with `errors.rs`, `errors.rs` wins and the disagreement is noted in
§6. Companion code: `src/observability.rs` (§5 of this document).

---

## 1. The error type at a glance

`errors::TaleError` is THE error type (contract C6). The alias
`errors::TaleResult<T> = Result<T, TaleError>` is the migration
target signature replacing the loaders' `Result<T, String>`.

Shape of every failure, as implemented today:

- **What operation failed** — the `context` string (context-carrying
  variants) or the `path` (file-carrying variants).
- **Where** — `path: Option<PathBuf>` on `ImageIo` / `DataLoad`;
  `None` when no single file is attached (the `*_msg` constructors).
- **Underlying cause** — `source: String`, preserved **verbatim**
  (constructors take `impl Display` for the source, so `String`
  causes, `std::io::Error`, and wgpu's `Display` error types all
  slot in without conversion boilerplate). `source_message()`
  returns it unchanged.
- **Category** — `category() -> &'static str` (kebab-case, §2).
- **Process exit code** — `exit_code() -> i32` (§3).

`Display` prints one line; main.rs's top-level handler
(`exit_on_err`) prints it as `error: {e}` on **stderr** and exits
with `e.exit_code()`. `std::error::Error::source()` returns `None`
by design — the cause is text inside the variant, shown by Display,
not a chained error object.

Deliberate non-conversions (from the `errors.rs` header — these are
laws, not omissions):

- **No blanket `From<std::io::Error>`.** An io failure *reading* a
  scene file is `DataLoad` (exit 2, invalid input); one *writing* a
  frame is `ImageIo` (exit 4, output failure). A blanket conversion
  would silently pick one and misclassify the exit code. Call sites
  say which: `TaleError::data_load(path, e)` /
  `TaleError::image_io(path, e)`.
- **`From<image::ImageError>` IS provided** — image encode/decode
  failures are unambiguously `ImageIo` (with `path: None`; call
  sites that know the path should prefer
  `TaleError::image_io(path, e)`).

---

## 2. Category table — every variant in `errors.rs`

| Category (`category()`) | Variant | Constructors | Carries | Exit |
|---|---|---|---|---|
| `gpu-init` | `GpuInit { context, source }` | `gpu_init(context, source)` | request site + cause text | 3 |
| `device-request` | `DeviceRequest { context, source }` | `device_request(context, source)` | `request_device` site + cause text | 3 |
| `buffer-map` | `BufferMap { context, source }` | `buffer_map(context, source)` | readback site + cause text | 3 |
| `image-io` | `ImageIo { path: Option<PathBuf>, source }` | `image_io(path, source)`, `image_io_msg(source)`, `From<image::ImageError>` (→ `path: None`) | frame/asset path (optional) + cause text | 4 |
| `data-load` | `DataLoad { path: Option<PathBuf>, source }` | `data_load(path, source)`, `data_load_msg(source)` | scene/film file (optional) + cause text | 2 |
| `render` | `Render { context, source }` | `render(context, source)` | pass/submission site + cause text | 3 |

Constructor argument types (from `errors.rs`): `context` is
`impl Into<String>`; `path` is `impl Into<PathBuf>`; `source` is
`impl fmt::Display` in every constructor.

### 2.1 `gpu-init` — exit 3

- **When raised:** no usable Vulkan adapter / wgpu instance setup
  failed — e.g. `request_adapter` found nothing. The canonical
  new-machine failure.
- **Display:** `GPU initialisation failed ({context}): {source}`
- **Recovery:** this is environmental, not a data problem. Verify a
  Vulkan driver is installed and visible (a hardware driver, or the
  lavapipe software driver as fallback). The source text for the
  adapter-absence case is doc-quoted in the runbooks and must stay
  verbatim: `no wgpu adapter found — a Vulkan driver (hardware or
  lavapipe) is required`. Fix the driver/ICD setup and re-run; no
  input files need to change.

### 2.2 `device-request` — exit 3

- **When raised:** the adapter was found but `request_device`
  failed (limits/features refused, device lost at initialisation).
- **Display:** `GPU device request failed ({context}): {source}`
- **Recovery:** environmental. Check the source text for the
  refused limit/feature; try a different adapter (including the
  software adapter) or an updated driver, then re-run. The failing
  site is named in `context` (e.g. `make_device (scene-file)`).

### 2.3 `buffer-map` — exit 3

- **When raised:** a GPU buffer could not be mapped for readback —
  the map channel closed, or the map itself errored. In practice
  this is device loss or driver failure mid-run, including mid-film
  (a lost device at frame *N* surfaces here with the frame named in
  `context`, e.g. `readback (film frame 1200)`).
- **Display:** `GPU buffer mapping failed ({context}): {source}`
- **Recovery:** re-run once the device/driver is healthy. For film
  renders the run's existing resume contract applies — completed
  frames on disk are not re-rendered, so a resumed run picks up
  from the failure point rather than from zero. If it recurs at the
  same frame, treat the source text + frame context as the bug
  report payload.

### 2.4 `render` — exit 3

- **When raised:** a render pass failed *after* setup succeeded —
  pipeline, submission, or scene-evaluation errors that are neither
  device loss nor buffer mapping. (V2 contract C3 also assigns this
  constructor to the FFmpeg streaming sink: a non-zero ffmpeg exit
  or a frame-count mismatch at close is a `TaleError::render` with
  the captured stderr tail attached as the source.)
- **Display:** `render failed ({context}): {source}`
- **Recovery:** read `context` for the failing pass/site and the
  source verbatim — it is the diagnostic. Scene-evaluation causes
  are fixed in the scene/film data (note: a *load-time* parse
  failure of that same data is `data-load`, exit 2, not `render`);
  pipeline/submission causes are engine/driver issues — capture the
  full `error:` line when reporting.

### 2.5 `image-io` — exit 4

- **When raised:** an image could not be encoded, decoded, or
  written — PNG frame saves, caption/asset image loads via the
  `image` crate, output-directory creation.
- **Display:** with a path — `image output failed for {path}: {source}`;
  without — `image operation failed: {source}`.
- **Recovery:** output/filesystem side. Check the named path:
  output directory exists and is writable, disk is not full, and
  (for decode failures) the asset file is a valid image of a
  supported format. Fix the filesystem/asset condition and re-run;
  the scene data itself loaded fine if the run reached this point.

### 2.6 `data-load` — exit 2

- **When raised:** scene/film data could not be read or parsed —
  scene JSON, `film.json` + manifests, caption/asset files, and
  authored data values (e.g. a malformed hex colour, via
  `data_load_msg` when no single file is attached).
- **Display:** with a path — `failed to load {path}: {source}`;
  without — `data load failed: {source}`.
- **Recovery:** input side — the run never started rendering.
  Check the named file exists and is readable, its JSON parses,
  and authored values are well-formed; the source text names the
  parse problem verbatim. Correct the input and re-run. Do not
  confuse with exit-2 *CLI* errors (§3): those come from
  `main.rs`'s `cli_error`, print a `Try 'talerender --help'`
  hint, and are not `TaleError`s.

---

## 3. Exit codes — the full table

From `exit_code()` in `errors.rs`, plus the two codes no variant
maps to:

| Code | Meaning | Produced by |
|---|---|---|
| 0 | Success | (no error) |
| 1 | QA validation failed | The film-telemetry gate's **verdict only** — `FILM TELEMETRY: CHECK FAILED`. **Never a `TaleError`.** The printed verdict line stays authoritative and the exit code agrees with it (EXIT-CODES-W02). Do not route the telemetry verdict through `TaleError`. |
| 2 | Invalid arguments or input | `DataLoad` (§2.6). CLI parse errors also exit 2, via `main.rs`'s separate `cli_error` path (unknown flag, bad/missing value) — same code, separate mechanism, by design. |
| 3 | GPU / rendering failure | `GpuInit`, `DeviceRequest`, `BufferMap`, `Render`. **Also:** `--film-telemetry` could-not-run (unreadable film dir) exits 3 — but by a different route (main.rs maps `run_film_telemetry`'s `Err(String)` to 3 directly); it is **not** a `TaleError` and prints no verdict line. Device loss handled by the lead-wired gpucore closure (contract C7) also exits 3 (the `gpu-init` class). |
| 4 | Output / filesystem failure | `ImageIo` (§2.5) |
| 101 | *(not in the table)* | A Rust panic. Runtime conditions should never surface as 101 — a 101 in the field means an unmigrated panic site or a genuine invariant violation (programming error), per the ERRORS-W05/W35 inventories. |

---

## 4. Using the type (for V2 module authors)

- Return `errors::TaleResult<T>` from every new module (C6). New
  variants are added by the lead only, keeping `category()` /
  `exit_code()` total.
- Pick the variant by **which side of the run failed**, not by the
  Rust type of the underlying error: reading input → `data_load`;
  writing output/images → `image_io`; adapter → `gpu_init`;
  device → `device_request`; readback → `buffer_map`; everything
  else render-time → `render`.
- Put the operation + site in `context` (the house style names the
  site, e.g. `request_adapter (scene path)`, `readback (film frame
  1200)` — frame numbers live in `context` by convention; see §6.1).
- Pass the underlying error directly as `source` (it is
  `impl Display`); do not summarise or reword it — downstream docs
  and runbooks quote some source strings verbatim (§2.1).

---

## 5. Observability event vocabulary (`src/observability.rs`)

One line per event, on **stdout**, printed by
`observability::event(name, fields)`:

```
event=<name> k=v k=v
```

Format laws (contract C6 + the module):

- Exactly one physical line per event; fields in the caller's
  order; an empty field set prints just `event=<name>`.
- Event names are dotted lowercase.
- Values never contain spaces: spaces, `\n`, `\r`, `\t` in any
  value (and, defensively, in keys and the name) are replaced
  with `_` by `format_event`, the pure builder `event()` prints.
- Events are log-only companions to the existing human-readable
  lines — they replace nothing, and no event feeds a pixel, a
  verdict, or a gate. Errors are not events: failures print
  `error: {e}` on stderr and exit per §3.

### 5.1 Events with typed helpers

| Event | Helper | Fields (in order) | Emitted when |
|---|---|---|---|
| `render.start` | `startup_line(mode)` | `mode` | Renderer startup, once the run's mode is known. |
| `adapter.selected` | `adapter_line(name, backend, device_type, driver, is_software)` | `name`, `backend`, `device_type`, `driver`, `software` | Adapter initialisation/selection. `software` is `true`/`false` and is **always present** — a software adapter is always labelled software (master prompt §7, contract C7); `is_software` is `device_type == Cpu`. |
| `frame.done` | `frame_done_line(index, total)` | `index`, `total` | A frame completes production. |
| `mem.pressure` | `pressure_line(level, accounted)` | `level`, `accounted` | A memory-pressure observation (contract C1 levels); `accounted` is the VRAM ledger's accounted byte count (contract C2). |
| `output.finalized` | `finalized_line(frames, out)` | `frames`, `out` | Output finalization completes (after mux + rename); `out` is the final output path. |

Example lines (exact, as the tests in `observability.rs` pin them):

```
event=render.start mode=film
event=adapter.selected name=llvmpipe backend=Vulkan device_type=Cpu driver=llvmpipe software=true
event=frame.done index=1051 total=1811
event=mem.pressure level=elevated accounted=1500000000
event=output.finalized frames=1811 out=out/film.mp4
```

### 5.2 Events named by the contracts, emitted via `event()` directly

| Event | Named by | Fields | Notes |
|---|---|---|---|
| `stream.open` | C6 (example vocabulary) | caller's choice (e.g. `width`, `height`, `fps`) | No typed helper in this seat; WP4's streaming landing calls `event("stream.open", …)` directly. |
| `device.lost` | C7 | `reason` | Printed by the lead-wired `device.set_device_lost_closure` handler in gpucore as `event=device.lost reason=<...>`; the process then exits with code 3 (the `gpu-init` class, §3). Detection + diagnosed failure only — no recreate-and-resume is claimed. |

Master prompt §16's remaining structured-log areas — asset
loading, scene evaluation, GPU operations, encoder operations —
have no dedicated event names in C6 or in this seat's module;
emitters use `event()` with a dotted-lowercase name until the
lead standardises more vocabulary.

---

## 6. Disagreements and gaps — `errors.rs` wins

### 6.1 Master prompt §16 vs `errors.rs` (the substantive gap)

§16 asks for unified structured errors carrying *category, code,
human-readable message, project and scene, frame number, resource,
recovery possibility, suggested corrective action*. The type as it
stands provides exactly: **category** (`category()`), **code**
(`exit_code()`), and a **human-readable message** (Display =
context/path + verbatim source). There are **no dedicated fields**
for project, scene, frame number, resource, recovery possibility,
or suggested action in any variant. Frame/site information exists
only by convention inside the `context` string, and recovery
guidance exists only as prose (this document, §2). This reference
describes the code as it is; closing the §16 gap would need new
fields/variants, which C6 reserves to the lead.

### 6.2 Exit 3 has three producers, only one of them `TaleError`

`errors.rs`'s header table says exit 3 is "GPU/render failure",
which is true of its variants — but §3's full picture (from
EXIT-CODES-W02 and contract C7) is that `--film-telemetry`
could-not-run and the device-lost closure also exit 3 without any
`TaleError` being constructed. Readers scripting on exit codes
must not assume `3 ⇒ a category in §2`. The current `--help` text
in `main.rs` already states this bundled truth; the narrower
replacement text proposed in EXIT-CODES-W02 §3c was superseded by
that broader wording. Codes agree everywhere; only the prose
differed.

### 6.3 Cosmetic ordering

ERRORS-W05 §2 lists the variants `… Render, ImageIo, DataLoad`,
while `errors.rs` declares them `GpuInit, DeviceRequest,
BufferMap, ImageIo, DataLoad, Render`. Declaration order carries
no semantics (category and exit code are per-variant methods);
the table in §2 above follows the source order.

### 6.4 No disagreement found on the load-bearing facts

Category strings, the exit-code mapping (2/3/4, with 1 reserved
for the telemetry verdict), the no-blanket-`From<io::Error>` rule,
and the `From<image::ImageError> → ImageIo` conversion agree
across `errors.rs`, ERRORS-W05, ERRORS-STEP2-W35, EXIT-CODES-W02,
the `main.rs` `--help` EXIT CODES section, and contract C6.

---

*Seat 9 (WP12) — two new files only (`src/observability.rs`,
this reference). No existing file modified, nothing committed.
The lead wires `mod observability;` and any call sites per the
DELIVERY NOTE at the top of `observability.rs`.*
