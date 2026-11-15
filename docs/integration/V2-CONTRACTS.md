# TALERENDER V2 — ARCHITECTURE CONTRACTS (Phase B)

Authority: `V2-MASTER-PROMPT.md` (user launch order
2026-11-15 12:26 +06). Phase A gate PASSED: the wgpu 30
upgrade is landed and pixel-neutral (CI `65e4ea2` build +
195/195 tests + smoke green; probe `43688575920`
telemetry CHECK OK, byte probes 40/40 vs `d101770`;
golden verify 17 OK). No seat spawns before this document
is committed.

## §1 Ownership law (master prompt §19, binding)

- The LEAD owns every existing file: all of `src/*.rs`,
  `Cargo.toml`, `.github/workflows/*`, existing `tools/**`
  and existing docs. Only the lead commits, always with
  explicit paths.
- SEATS write NEW files only, at the exact paths their
  charter assigns. A seat never edits an existing file,
  never runs git, never commits. No two seats share a file.
- A seat's module is integrated by the lead: the lead adds
  the `mod` line to `src/main.rs`, wires call sites per the
  seat's DELIVERY NOTE, and lands it under the minimal
  gates (§4). If a seat's code needs a change in an
  existing file beyond wiring, the note specifies it
  precisely (file, anchor, replacement) and the lead
  applies it as part of that seat's landing.
- Zero new dependencies (master prompt §2): the crate has
  exactly three direct deps (wgpu 30.0.2, pollster 1.0.1,
  image 0.25.10). Seats use std + the three deps only.
  Bounded queues are `std::sync::mpsc::sync_channel`;
  JSON is the in-tree `scene::parse_json` / `scene::Value`.

## §2 Tree map (verified at `65e4ea2`, 24,420 lines)

Lead-owned core: `film.rs` (film renderer + PaintCache),
`scene.rs` (scene model, evaluation, JSON), `scenefile.rs`
(scene-file loader/runners), `telemetry.rs`, `composer.rs`
(post chain), `gpucore.rs` (instance/device/readback +
VRAM ledger), `charanim.rs`, `anim.rs`, `locomotion.rs`,
`cameramoves.rs`, `toonfull.rs`, `rendergraph.rs`,
`dof.rs`, `ab1.rs`, `main.rs` (CLI + entry points),
`errors.rs` (TaleError/TaleResult), `runlog.rs`,
`paintbudget.rs`, `bufferpool.rs`, `atlas.rs`, `math.rs`,
`battery.rs`. Shaders: `src/*.wgsl` (lead-owned;
check_shaders.py enforces WGSL↔Rust agreement).

## §3 Shared contracts

### C1 — Memory-pressure controller (WP1.3 owns; WP1/WP4/WP7 consume)
New module `src/mempressure.rs`:
```rust
pub(crate) enum MemoryProfile { LowMemory, Balanced, HighPerformance }
pub(crate) enum PressureLevel { Normal, Elevated, Critical }
pub(crate) struct MemPressure { /* profile, ceilings */ }
impl MemPressure {
    pub(crate) fn from_env_and_arg(arg: Option<&str>) -> MemPressure;
    pub(crate) fn profile(&self) -> MemoryProfile;
    pub(crate) fn paint_budget_bytes(&self) -> u64;   // LowMemory = the derived 2 GiB ceiling; never above it on LowMemory
    pub(crate) fn frame_queue_capacity(&self) -> usize; // LowMemory: 2; Balanced: 4; HighPerformance: 8
    pub(crate) fn level(&self, accounted_bytes: u64, peak_rss_mb: Option<f64>) -> PressureLevel;
    pub(crate) fn preload_enabled(&self) -> bool;    // false on LowMemory
}
```
Profile selection: `--mem-profile` CLI > `TALERENDER_MEM_PROFILE`
env > default `Balanced`, EXCEPT an unset profile on a
machine reporting ≤ 8 GiB total RAM defaults to LowMemory
(detect via /proc/meminfo; failure → Balanced). Profiles
NEVER change output quality, resolution, fps or encoding
— they bound caches, queues and preload only.

### C2 — Telemetry reporting interface (WP2 owns; WP6 consumes)
The lead adds to `gpucore.rs`:
```rust
pub(crate) struct MemSnapshot {
    pub accounted_bytes: u64, pub peak_accounted_bytes: u64,
    pub per_class: [(u64, u64); 7],   // (current, peak) for classes A..G
    pub textures: u64, pub buffers: u64,
    pub created_textures: u64, pub created_buffers: u64,
    pub destroyed_textures: u64, pub destroyed_buffers: u64,
    pub readback_bytes: u64,
    pub sysfs_gtt_used: Option<u64>, pub sysfs_vram_used: Option<u64>,
}
pub(crate) fn mem_snapshot() -> MemSnapshot;
```
(creation/destruction counters are added to the ledger by
the lead in the same landing.) WP2's `src/memreport.rs`
renders a snapshot + adapter identity as (a) stable
`mem-report:` key=value lines and (b) a Markdown table.
The three honesty classes of master prompt §6 stay
labelled: TRACKED (ledger), DRIVER (sysfs, may be
unavailable), ESTIMATE (anything derived — named as such).
GPU timestamps: capability-detected via
`adapter.features()` (`TIMESTAMP_QUERY`); if absent the
report says `gpu-timestamps: unavailable` — never faked.

### C3 — Streaming (WP4 owns; WP11 consumes)
New modules `src/streamq.rs`, `src/ffstream.rs`:
```rust
pub(crate) struct FramePacket { pub index: u32, pub width: u32, pub height: u32, pub rgba: Vec<u8> }
pub(crate) fn frame_queue(capacity: usize)
    -> (std::sync::mpsc::SyncSender<FramePacket>, std::sync::mpsc::Receiver<FramePacket>);
```
Queue capacity comes from C1 (`frame_queue_capacity`) —
bounded always; the producer blocks on a full queue
(backpressure), it never drops or accumulates.
`ffstream.rs` owns the FFmpeg child: spawn with rawvideo
`rgba` stdin (`-f rawvideo -pix_fmt rgba -s WxH -r FPS -i -`),
production encode x264 CRF 18 (preset slow) to
`<out>.part.mp4`, then the existing finishing contract
(mix.wav mux + faststart rename) — the seat codes against
`docs/runbook/FINISH.md` if present, else the finish.sh
behaviour recorded in WAVE2-RECORD (lead verifies at
integration). Stderr captured to a string; non-zero exit
or a frame-count mismatch at close is a `TaleError::render`
with the stderr tail attached. Modes (Batch 2,
`src/streammodes.rs`): `OutputMode { Png, Stream, Auto }`;
Auto = Stream when ffmpeg is on PATH, else Png, and the
chosen mode is PRINTED — never silent.

### C4 — Evaluator interchange schema (WP5 owns)
New module `src/projectfile.rs` + exporter
`tools/export/export_project.mjs`. The interchange
document is JSON, parsed with `scene::parse_json`:
```json
{ "format": "talerender-project/1",
  "meta": { "title": "...", "fps": 30, "width": 1080, "height": 1920 },
  "assets": [ { "id": "...", "path": "relative/path", "kind": "image|audio|data" } ],
  "characters": [ ... ], "scenes": [ ... ], "shots": [ ... ],
  "captions": [ ... ], "audio": [ ... ] }
```
Rules: unknown `format` or a major version ≠ 1 → a hard
compatibility error naming both versions; missing asset
file → a diagnostic listing EVERY missing asset (never a
silent skip); paths are relative and must not escape the
project root (`..` rejected); section shapes validate
against the field lists the seat derives from
`scene.rs`/`scenefile.rs` — the seat's charter pack
(FORMAT15-DRAFT-W31 + FORMAT-EXT-CLIPS-W15) is the design
basis, and the §15 step-2 loader consumption is the
foundation, not a competing plan. The s12 code retirement
rides WP5 ONLY as its pack (S12-ADOPT-W83) specifies,
byte-probe gated on frames 1666–1810, lead-applied.

### C5 — Render scheduler (WP7 owns)
New module `src/scheduler.rs`:
```rust
pub(crate) struct FrameRange { pub start: u32, pub end: u32 } // end exclusive
pub(crate) struct SchedOpts { pub max_in_flight: usize, pub checkpoint_path: Option<PathBuf> }
pub(crate) enum SchedEvent { FrameDone(u32), RangeDone, Cancelled(u32) }
pub(crate) fn plan(range: FrameRange, opts: &SchedOpts) -> Vec<u32>; // resume-aware ordering
```
Determinism law: output ORDER and bytes never depend on
completion order; `plan` is a pure function of (range,
checkpoint contents). Checkpoint file = text, one
completed index per line; resume skips completed indices.
Cancellation is cooperative (an `AtomicBool` the caller
owns). Concurrency defaults come from C1 (LowMemory → 1
in flight). The scheduler schedules; it does not render —
the film loop stays the only renderer in this wave.

### C6 — Errors and observability (WP12 owns)
`errors::TaleError` remains THE error type; every new
module returns `errors::TaleResult<T>`. New variants are
added by the lead only, keeping `category()`/`exit_code()`
total. WP12's `src/observability.rs` provides:
```rust
pub(crate) fn event(name: &str, fields: &[(&str, String)]); // prints: event=<name> k=v k=v
pub(crate) fn startup_line(mode: &str);
```
Event names are dotted lowercase (`render.start`,
`adapter.selected`, `stream.open`, `mem.pressure`,
`output.finalized`). Values never contain spaces
(underscores substituted). WP12 also authors
`docs/integration/V2-ERROR-REFERENCE.md` from the actual
`errors.rs` (every category, exit code, recovery note).

### C7 — Adapter policy and device loss (WP3 owns)
New module `src/adapter.rs`:
```rust
pub(crate) enum AdapterPolicy { Auto, HardwareOnly, SoftwareOk, Named(String) }
pub(crate) struct AdapterReport { pub name: String, pub backend: String,
    pub device_type: String, pub driver: String, pub is_software: bool }
pub(crate) fn policy_from(arg: Option<&str>, env: Option<&str>) -> AdapterPolicy;
pub(crate) fn describe(info: &wgpu::AdapterInfo) -> AdapterReport;
pub(crate) fn report_line(r: &AdapterReport) -> String; // the canonical `adapter:` line
```
`is_software` is `device_type == Cpu` — a software adapter
is always labelled software in every report (master
prompt §7). Device loss (wgpu 30 surface): the lead wires
`device.set_device_lost_closure` at device creation in
gpucore to a handler that prints
`event=device.lost reason=<...>` and exits with code 3
(the GpuInit class) — detection + diagnosed failure; true
recreate-and-resume is implemented ONLY if the seat's
survey shows wgpu 30 supports it without fabricating
state, else the limitation is documented in the seat's
note and in STAGE1-SCOPE §6. dof.rs's conditional
software-note fix (Stage 1 item 3 residue) is applied by
the lead in WP3's landing.

### C8 — Configuration (WP13 owns)
New module `src/config.rs`: one `Config` struct resolved
with precedence **CLI arg > env (`TALERENDER_*`) > config
file (`talerender.toml`, hand-parsed key=value — no new
deps) > default**. Fields: mem_profile, adapter_policy,
output_mode, stream_fps override (none by default),
perf_record_dir. Every field's resolution is unit-tested
in-module. The lead wires `main.rs` to consult `Config`
for the new flags without changing any existing flag's
behaviour or the v1.0.1 CLI contract.

## §4 Minimal gates per landing (master prompt §1/§21)

Every integration landing keeps: CI build SUCCESS (a
conclusion of failure caused SOLELY by the artifact-quota
upload step is reported as such, with the step list as
evidence — it does not block), the full test suite green,
FILM TELEMETRY: CHECK OK (probe workflow), and the
determinism byte-probes 40/40 vs the §3 baseline
(`~/workspace/talerender-evidence/hashes-post.txt` ranges
195–204, 795–814, 1195–1204). Landings that cannot touch
the render path (pure tools/docs) still get CI + Tools CI
verdicts. Campaigns (benchmarks, stress, fault injection,
visual regression) are DEFERRED to the validation wave.

## §5 Batch plan (actual counts reported at each close)

**Batch 1 — 10 seats** (independent new modules):
1. WP1.2 dirty-region uploads → `src/dirtyrect.rs`
2. WP1.3 memory-pressure controller → `src/mempressure.rs` (C1)
3. WP4 streaming core → `src/streamq.rs` (C3 queue half)
4. WP4 FFmpeg integration → `src/ffstream.rs` (C3 sink half)
5. WP5 evaluator generalisation → `src/projectfile.rs` (C4)
6. WP6 perf-lab infrastructure → `tools/perf/lab/bench_lab.py` + record schema doc in-file
7. WP7 render scheduler → `src/scheduler.rs` (C5)
8. WP8 asset streaming → `src/assetstream.rs`
9. WP12 structured errors/observability → `src/observability.rs` + `docs/integration/V2-ERROR-REFERENCE.md` (C6)
10. WP13 CLI + configuration → `src/config.rs` (C8)

**Batch 2 — 10 seats** (integrate/complete):
1. WP1.1 LRU + WP1.4 lifetimes → patch spec `docs/integration/V2-WP1-LRU-SPEC.md` (lead applies to film.rs) + `src/lifetimes.rs`
2. WP2 telemetry reporting completion → `src/memreport.rs` (C2)
3. WP3 adapter policies + device-loss → `src/adapter.rs` (C7) + device-loss survey note in its delivery
4. WP4 streaming modes + recovery → `src/streammodes.rs` (C3)
5. WP5 TypeScript export pipeline → `tools/export/export_project.mjs` (C4)
6. WP9 production integration → `tools/integration/validate_native_project.py` + `docs/integration/V2-WP9-INTEGRATION.md`
7. WP10 character rendering efficiency → `src/charstate.rs`
8. WP11 output/encoding completion → `src/outputmgr.rs`
9. WP6 reporting engine → `tools/perf/lab/report.py` (consumes seat B1-6's schema)
10. WP14 documentation + migration guide → `docs/v2/` (architecture, CLI, config, streaming, migration)

Seat delivery format: the new file(s) in the tree
(untracked), plus a DELIVERY NOTE appended as a comment
block at the top of the main file: files created, the
contract points implemented, the wiring the lead must do
(mod line + call sites, exact anchors), tests included,
known limitations. Seats write unit tests in-module
(`#[cfg(test)]`) for every pure function — CI runs them
at integration.

## §6 Explicitly out of scope this wave

§15 evaluator work beyond C4/WP5's charter; the split
programmes; the film-s03 pin (debt stands, no re-pin);
skill-tree edits (WP9 is repo-side; skill changes are
proposals in its doc); RADV/hardware numbers (PC session
belongs to the validation wave); making native the
default (master prompt §24.16 — Chrome stays default).
