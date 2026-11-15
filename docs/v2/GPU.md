# TaleRender V2 — GPU architecture (as implemented)

## 1. Adapter selection — `gpucore::request_adapter` (WIRED)

All five render entry points share the one canonical helper in
`src/gpucore.rs` (there is no per-mode copy left):

1. `new_instance()`: a Vulkan-only `wgpu::Instance`
   (`Backends::VULKAN`; wgpu 30 construction via
   `InstanceDescriptor::new_without_display_handle()` — see
   `MIGRATION.md`).
2. First request: `PowerPreference::HighPerformance`,
   `force_fallback_adapter: false`,
   `apply_limit_buckets: false` (keeps the adapter's exact
   limits, the wgpu 0.20 behaviour).
3. If that errors, retry with default power preference and
   `force_fallback_adapter: true` (the software adapter).
4. If no adapter answers at all: `TaleError::gpu_init`
   (category `gpu-init`, exit 3) with the established message
   "no wgpu adapter found — a Vulkan driver (hardware or
   lavapipe) is required".

Devices come from `gpucore::request_device` /
`request_device_default` (standard `wgpu::Limits::default()` —
downlevel limits cap textures at 2048 and cannot hold the
2560×2816 character canvas); failures are `DeviceRequest`
(exit 3). There is **no adapter policy selection in the binary
today**: no `--adapter-policy` flag is parsed, and
`config.rs`'s `adapter_policy` value is not consulted (see
`CONFIG.md`). The C7 policy type below is CONTRACT.

## 2. Adapter reporting — what actually prints

Every render run prints the adapter it actually used, in the
house format (one line per entry point, same shape in
`main.rs`, `ab1.rs`, `scenefile.rs`; `film.rs` spells the first
field `renderer:`):

```
adapter: <name> | backend: <Backend> | device type: <DeviceType> | driver: <driver> <driver_info>
```

(`film.rs`: `renderer: <name> | backend: … | device type: … |
driver: …`.) A software fallback therefore shows up as a CPU
device type with a lavapipe/llvmpipe name — it is never
relabelled as hardware. Two honest variants exist:

- `dof.rs` prints `adapter: <name> (<backend>, driver <driver>)
  — software rasteriser, no performance claim` for its chart run.
- `battery.rs` (pure CPU) prints a `renderer: TaleRender CPU
  painter … — no GPU involved` line.

The V2 structured companion exists but is **not wired**:
`observability::adapter_line(...)` would emit
`event=adapter.selected name=.. backend=.. device_type=..
driver=.. software=true|false` with the `software` field always
present (`is_software` = `device_type == Cpu`). No call site
emits it yet.

**CONTRACT (C7, `src/adapter.rs` — file ABSENT):**
`AdapterPolicy { Auto, HardwareOnly, SoftwareOk, Named(String) }`,
`policy_from(arg, env)`, `describe(&AdapterInfo) ->
AdapterReport`, and `report_line()` producing a canonical
`adapter:` line. The string forms `config.rs` validates today
are `auto | hardware-only | software-ok | named:<name>`.
None of this is in the tree; §1 above is the whole implemented
behaviour.

## 3. The `gpu-mem:` line — grammar (WIRED)

Printed at end of run by `gpucore::mem_stats_line()` in render
modes (film telemetry prints **no** line: it is CPU-only, no
device is ever created, and a structurally-zero ledger would be
a fabricated-looking measurement — ruling D1 in the code).

```
gpu-mem: accounted_bytes=<u64> peak_accounted_bytes=<u64> textures=<u64> buffers=<u64> readback_bytes=<u64> sysfs_gtt_used_bytes=<u64|unavailable> sysfs_vram_used_bytes=<u64|unavailable>
```

Pinned example (from the module's own test, the `f9429a8`
staging): `gpu-mem: accounted_bytes=220930688
peak_accounted_bytes=229286528 textures=41 buffers=18
readback_bytes=8355840 sysfs_gtt_used_bytes=unavailable
sysfs_vram_used_bytes=unavailable`. Field order and spelling
are parsed by `tools/perf/vram_report.py` — do not change them.

The three honesty classes (master prompt §6), as implemented:

- **TRACKED** — `accounted_bytes` / `peak_accounted_bytes` /
  `textures` / `buffers` / `readback_bytes`: the gpucore ledger
  (classes A–G; see `MEMORY.md` §4). A lower bound: descriptor
  arithmetic only.
- **DRIVER** — `sysfs_gtt_used_bytes` / `sysfs_vram_used_bytes`:
  read from `/sys/class/drm/cardN/device/mem_info_{gtt,vram}_used`
  (first card with a GTT file wins). These are *device-wide, all
  processes*, and on the 5650G the VRAM figure is a carve-out of
  the same system RAM — neither is this process's share. Any
  failure yields the literal `unavailable`, never `0`.
- **ESTIMATE** — nothing in the line is an estimate, and nothing
  derived is presented as physical VRAM. Integrated-GPU
  shared-memory means "VRAM usage" for this process is not a
  well-defined number on the target machine; the docs and the
  ledger say so instead of inventing one.

## 4. Telemetry + memreport

Implemented today:

- The `gpu-mem:` line (§3) and the `paint-cache:` line
  (`MEMORY.md` §1) are the machine-readable memory outputs.
- `tools/perf/vram_report.py` parses `gpu-mem:` lines from run
  logs; `tools/perf/bench_report.py` builds the PERF tables.
- Film/QA telemetry (`telemetry.rs`, `--film-telemetry`,
  `--telemetry`) is a separate, CPU-only semantic gate
  (`FILM TELEMETRY: CHECK OK` / `CHECK FAILED`) — it measures
  staging semantics, not GPU memory, and creates no device.

**Landing in this wave (C2):** `src/memreport.rs` (712 lines)
landed during this wave as an untracked seat file. It renders a
snapshot + adapter identity two ways — `render_lines` (stable
`mem-report:` key=value lines) and `render_markdown` (a table) —
both pure functions, with the honesty classes explicit via an
`honesty=` field / `Source` column: TRACKED (ledger figures),
DRIVER (adapter identity + sysfs counters, missing sysfs =
literal `unavailable`, never 0), ESTIMATE (its only derived
figure restates the tracked total as
`estimated_vram_lower_bound_bytes`, named as a lower bound).
**Integration caveat, honestly:** the module renders
`crate::gpucore::MemSnapshot` — and that type (with
`mem_snapshot()` and the creation/destruction counters C2
specifies) **does not exist in `gpucore.rs` yet**; it is the
lead-owned half of C2. The file is also not registered in
`main.rs`. Until the lead lands both, `memreport` cannot
compile and nothing in the binary emits a `mem-report:` line.
The ledger in the tree today tracks current bytes, per-class
counts of live textures/buffers, the peak watermark, and the
largest single readback — no creation/destruction counters.
GPU timestamps are likewise **not implemented**: no code in
`src/` queries `TIMESTAMP_QUERY` (verified by search). The C2
contract requires capability detection via `adapter.features()`
and the literal report `gpu-timestamps: unavailable` when
absent — until the C2 landing completes, there is no timestamps
field anywhere, faked or otherwise.

## 5. Device loss — AS IMPLEMENTED: not implemented

This is the section where the honest answer is a gap.

- **In the tree today:** no `set_device_lost_closure` (or any
  device-lost callback) is registered anywhere in `src/`
  (verified by search; the only `device.lost` string in the
  tree is a formatting test inside `observability.rs`). If the
  device is lost mid-run, behaviour is whatever wgpu 30's
  default handling produces — **not verified** by this seat,
  and no TaleRender-level diagnosed exit is guaranteed.
- **CONTRACT (C7):** the lead wires
  `device.set_device_lost_closure` at device creation in
  gpucore to a handler that prints
  `event=device.lost reason=<...>` and exits with code 3 (the
  `GpuInit` class). That is detection + diagnosed failure.
  Recreate-and-resume is to be implemented only if the seat's
  survey shows wgpu 30 supports it without fabricating state;
  otherwise the limitation is documented. At writing time
  neither the handler nor `src/adapter.rs` exists, and no
  device-loss survey note from the Batch 2 seat is in the tree.
- Related residue named in C7: the dof.rs conditional
  software-note fix (Stage 1 item 3) is applied by the lead in
  WP3's landing — also not yet in the tree beyond the existing
  line quoted in §2.

**Status (§23): device loss = INCOMPLETE** — contract written
(V2-CONTRACTS §3 C7), implementation absent. Do not cite
TaleRender as having device-loss handling until `adapter.rs`
and the gpucore closure land.
