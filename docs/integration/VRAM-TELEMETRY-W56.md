# VRAM-TELEMETRY-W56 — GPU-memory telemetry design (PERF §9 item 5)

Wave 2, Batch 6, seat W56. Surveyed at HEAD `aa92cee`. **Design
only** — no `src/` file was touched; implementation is lead work
against §6 below. Companion prototype: `tools/perf/vram_report.py`
+ fixture `tools/perf/fixtures/vram/film-run-tail.log` (self-test
transcript in §7).

## §1 The blind spot, as recorded

`docs/runbook/PERF.md` §9 item 5, verbatim:

> **Named blind spot — GPU memory.** wgpu exposes no VRAM figure
> here, and on the 5650G the iGPU shares system RAM, so GPU-side
> pressure is invisible in both the binary's output and (partly)
> host RSS. External observation (`radeontop`, if installed) is
> the only route; recorded as a blind spot, not a request — but
> any future adapter-info extension that surfaces it should be
> printed next to the `renderer:` line.

§9 items 1–4 are discharged (runlog W09, paint budget W06,
telemetry exit codes W02/lead `aa10b13`). Item 5 is the last open
one. It cannot be "closed" in the sense of a true device-memory
reading — §5 lists what stays unknowable — but the blind spot has
three layers, and two of them are closable today:

1. **The renderer's own allocations** — exactly knowable, from
   inside the process, with byte formulas (§2.2, §4). Today
   nobody adds them up.
2. **OS device counters on the user's PC** (amdgpu sysfs) —
   knowable on RADV hardware, absent on CI (§2.4).
3. **Driver-internal truth** — not knowable from wgpu 0.20 or
   from this process (§5). The design says so instead of
   papering over it.

Why it matters, concretely: the target PC has ~6.7 GiB usable
RAM *shared* with the Vega 7 iGPU (`paintbudget.rs` header,
PERF §7's ~4.6 GB cache worst case was already a genuine OOM
risk). Host RSS is measured (runlog); host paint-cache bytes are
measured (paintbudget); device bytes are the missing third term
of the same physical budget.

## §2 What can be known where

### §2.1 The wgpu 0.20 adapter surface — no memory figure exists

What the code already uses, verifiably at HEAD:

- `adapter.get_info()` — printed at five sites:
  `src/ab1.rs:590`, `src/dof.rs:140`, `src/film.rs:2388`,
  `src/main.rs:469`, `src/scenefile.rs:1141`, in the shape
  `adapter: {name} | backend: {:?} | device type: {:?} | driver:
  {driver} {driver_info}` (film's variant is headed `renderer:`).
  `AdapterInfo` carries identity strings and a device-type enum —
  no byte counts of any kind.
- Device limits are requested at device creation
  (`gpucore::request_device` / `request_device_default`,
  `src/gpucore.rs:96,122`; main's deliberate downlevel limits).
  `wgpu::Limits` fields are *capability ceilings* (max texture
  size, max buffer size, …) — a ceiling is not a usage figure,
  and no `Limits` field reports memory.

Beyond that, this note claims nothing about the 0.20 API from
memory: the vendored crate source is not on this VM (checked:
no `wgpu-0.20*` under `~/.cargo/registry/src`). Two candidate
surfaces are therefore registered **TO-VERIFY** (§8): the exact
`Limits` field inventory worth printing, and whether
`Instance::generate_report()` exists in 0.20 and what its report
contains (object counts would still not be bytes). The design
does not depend on either: **the ledger in §2.2 is the source of
record; adapter data is decoration.**

### §2.2 The renderer's own ledger — the exact source

Every device allocation in `src/` flows through a small set of
chokepoints, which is what makes an exact ledger feasible:

| Chokepoint | Sites at HEAD |
|---|---|
| `gpucore::make_target` / `make_target_viewed` (`src/gpucore.rs:141,164`) | 24 call sites outside gpucore |
| `composer::make_rt` (`src/composer.rs:72`) | film/ab1 chains + shadow depth + prepass |
| `gpucore::Readback` staging buffer (`src/gpucore.rs:226`) | every texture→CPU collect |
| Upload lanes: film data closure (`src/film.rs:2421`), `Renderer::upload_dyn` (`src/film.rs:2776`), scenefile uploads (`src/scenefile.rs:1106,1416`), PMREM (`src/composer.rs:105`) | one texture per image asset / dynamic card |
| Direct `create_texture` / `create_buffer` | film 5, ab1 10, scenefile 4, dof 5, main 4, composer 3 texture/buffer calls (grep census at HEAD) — each is a ledger registration site or an escape-hatch call (§6) |

The format universe is closed — only four texture formats appear
in the whole tree (grep census at HEAD): `Rgba16Float` (36 sites,
8 B/px), `Rgba8Unorm` (24, 4 B/px), `Depth32Float` (23, 4 B/px),
`Rgba8UnormSrgb` (14, 4 B/px). So every texture's byte size is
`width × height × layers × bytes_per_pixel(format) ×
sample_count`, with no format left unpriced.

### §2.3 The two existing partial ledgers — and a wiring truth

- **PaintBudget** (`src/paintbudget.rs`, wired into the film
  PaintCache at `86f0c81`) prints
  `paint-cache: entries={} bytes={} hits={} misses={} evictions={}
  rerenders={} peak_bytes={} budget_bytes={}` (format pinned by
  its `stats_line_format_is_stable` test). **Classification
  warning: these are HOST bytes.** `entry_bytes()` is
  `pixels.len()` of a CPU `Vec<u8>` (2560×2816 RGBA8 canvas =
  28,835,840 B, `CANVAS_BYTES`). The device-side cost of a painted
  canvas exists separately, when the canvas is uploaded as a
  texture (class E, §4). Any report that files paint-cache bytes
  as GPU memory is wrong; any report that ignores them on a
  shared-memory iGPU is incomplete. §3's line and the prototype
  both keep the two in separate, labelled columns.
- **BufferPool** (`src/bufferpool.rs`) has a `stats_line()` in
  the same style —
  `buffer-pool: retained_bytes={} free={} hits={} misses={}
  evictions={} peak_retained_bytes={} budget_bytes={}` —
  **but at HEAD it has no production caller**: `BufferPool` is
  referenced only inside `src/bufferpool.rs` itself (grep,
  HEAD `aa92cee`). The gpucore::Readback adoption that would wire
  it is still an open lead item. The design and the parser both
  treat the line as *future* until that adoption lands; the
  fixture includes one, labelled synthetic, so the parser is
  ready.

### §2.4 OS counters on Linux RADV — PC-only, TO-VERIFY

On amdgpu, the kernel exposes per-device byte counters in sysfs.
Candidate paths (all **TO-VERIFY on the user's PC**, §8 — this VM
has no GPU: `ls /sys/class/drm/` here shows no `cardN` device,
verified during this seat):

- `/sys/class/drm/cardN/device/mem_info_vram_total` /
  `mem_info_vram_used`
- `/sys/class/drm/cardN/device/mem_info_gtt_total` /
  `mem_info_gtt_used`
- `/sys/class/drm/cardN/device/mem_info_vis_vram_used`
- per-process: `/proc/self/fdinfo/<drm-fd>` `drm-memory-vram:` /
  `drm-memory-gtt:` lines (kernel fdinfo reporting; exact key
  spellings TO-VERIFY)

Semantics that must travel with the numbers: the `mem_info_*`
counters are **device-wide** (all processes), and on the 5650G
the "VRAM" is a carve-out of the same system RAM — GTT used is
usually the larger and more honest pressure term there; neither
counter is this process's share. `radeontop` (PERF §9.5's named
route) reads the same sources and stays the external cross-check.
On CI / lavapipe none of these paths exist: the line must print
`unavailable`, never 0 — the same discipline as runlog's
`peak RSS unavailable` (never a fabricated number).

## §3 The proposed stats line

One line, house style (same family as `paint-cache:` /
`buffer-pool:`: leading label, space-separated `key=value`,
integers in raw bytes, field order is the contract):

```
gpu-mem: accounted_bytes=<u64> peak_accounted_bytes=<u64> textures=<u64> buffers=<u64> readback_bytes=<u64> sysfs_gtt_used_bytes=<u64|unavailable> sysfs_vram_used_bytes=<u64|unavailable>
```

Field semantics:

- `accounted_bytes` — device bytes live at end of run, summed by
  the §6 ledger over classes A–E + G (§4). A **lower bound** on
  true device footprint (§5), and exactly reproducible from the
  §4 formulas — that reproducibility is its acceptance test.
- `peak_accounted_bytes` — the ledger's high-water mark (the
  number that matters for OOM headroom; mirrors
  `peak_bytes`/`peak_retained_bytes` precedent).
- `textures` / `buffers` — live object counts at end of run
  (leak smoke-detector: a film run whose texture count grows per
  frame is visible here before it is an OOM).
- `readback_bytes` — the largest single in-flight readback
  staging allocation during the run (class F; transient, already
  inside the peak, broken out because it is the spikiest term).
- `sysfs_*_used_bytes` — §2.4 counters sampled at end of run, or
  the literal `unavailable`.

**Placement** (one line per run, in every GPU mode):

- *Film render:* its own line immediately **after** the
  `film render complete: …` line. Not appended to that line —
  its composition is pinned by runlog's contract (counts, then
  time+RSS suffix, then cache stats last; `src/film.rs:3446`).
- *Film telemetry:* after the existing `cache.stats_line()` print
  (`src/film.rs:3590`).
- *Scene-file, ab1, dof, main single-still modes:* end of run,
  after rendering completes. These modes print the `adapter:`
  line at start and have no end-of-run summary today; the
  gpu-mem line stands alone, as the paint-cache line does in
  telemetry.

**On PERF §9.5's placement sentence** ("printed next to the
`renderer:` line"): that sentence governs *adapter-info
extensions*. The accounted total cannot exist at adapter time —
no allocation has happened yet — so this design splits the
difference honestly: anything adapter-side (e.g. limits, if ever
printed) goes next to `adapter:`/`renderer:` per §9.5; the
accounted *usage* line goes end-of-run, next to the other usage
lines (runlog suffix, paint-cache). The lead should fold this
paragraph into PERF §9.5 when implementing, so the runbook stops
pointing at a placement the physics forbids.

## §4 The accounting table (implementation contract)

Byte formulas per allocation-site class. `bpp`: Rgba16Float = 8,
Rgba8Unorm = Rgba8UnormSrgb = Depth32Float = 4. Texture bytes =
`w × h × layers × bpp × sample_count` throughout.

| Class | Sites (HEAD) | Formula |
|---|---|---|
| **A. Frame targets** | `make_target`/`make_rt` in film `Renderer::new` + PostChain (film.rs ~2536–2560), ab1 targets (ab1.rs:842–867), scenefile depth (scenefile.rs:1427), dof chart (dof.rs:174) | Σ per target `w×h×bpp×samples` |
| **B. Shadow pair** | film.rs:2528 (`make_target` 2048² Rgba8) + film.rs:2531 (`make_rt` 2048² Depth32Float); ab1.rs:809 inline + depth twin | `2048×2048×4` × 2 per lane that builds shadows |
| **C. Bloom chain** | composer.rs:493 (bright), 504/505 (horiz+vert per mip) | Σ over `bloom_mip_sizes(w,h)` (5 mips, halving with round): bright at mip 0 + horiz + vert at every mip, all Rgba16Float (8 B/px) |
| **D. PMREM room env** | composer.rs:105 `load_room_env`, `include_bytes!` asset | constant: `768 × 1024 × 8` = **6,291,456 B** |
| **E. Uploaded image textures** | film.rs:2421 closure, film.rs:2776 `upload_dyn`, scenefile.rs:1106/1416, texture generators' outputs | per texture `w×h×4` (Rgba8 family); count is **data-driven** (plates, calendar years, sign, caption strips, cards) — the ledger counts live textures; no closed-form total exists |
| **F. Readback staging** | gpucore.rs `Readback::record`/`record_bpp` | per in-flight readback `padded_row_bytes(w, bpp) × h`; pinned constants (bufferpool.rs:52,56): frame RGBA **8,355,840 B** (4352×1920), fp16 depth **16,711,680 B** (8704×1920) |
| **G. Buffers** | create_buffer sites: film.rs:2861/2872/2879, ab1.rs:665/1015/1071/1078/1088, scenefile.rs:1095, dof.rs:164/214/221/323, main.rs:510/670/677 | Σ `descriptor.size` per buffer (uniforms, vertex/index, shadow misc) — individually small, counted exactly, never estimated |
| **H. Paint cache — HOST, excluded** | paintbudget.rs | host `Vec<u8>` bytes; reported by the existing paint-cache line; **never** added into `accounted_bytes` |
| **I. BufferPool retained — future** | bufferpool.rs (unwired at HEAD) | when the Readback adoption lands, pool buffers are class F allocations owned by the pool; the ledger must count them **once** (pool registration, not per-collect) |

### Worked example — film lane at 1080×1920 (acceptance numbers)

`W = 1080 × 1920 = 2,073,600 px`. Steady-state classes only
(A+B+C+D); E is data-driven and F transient, both fielded
separately in the line.

| Term | Arithmetic | Bytes |
|---|---|---|
| hdr msaa (Rgba16Float ×4 samples) | 2,073,600 × 8 × 4 | 66,355,200 |
| hdr depth (Depth32Float ×4) | 2,073,600 × 4 × 4 | 33,177,600 |
| hdr resolved (Rgba16Float) | 2,073,600 × 8 | 16,588,800 |
| packed depth (Rgba8Unorm default; ×2 if `TALERENDER_DOF_DEPTH_FORMAT=halffloat`) | 2,073,600 × 4 | 8,294,400 |
| prepass depth (Depth32Float) | 2,073,600 × 4 | 8,294,400 |
| bokeh out (Rgba16Float) | 2,073,600 × 8 | 16,588,800 |
| output srgb (Rgba8Unorm) | 2,073,600 × 4 | 8,294,400 |
| final (Rgba8Unorm) | 2,073,600 × 4 | 8,294,400 |
| **A subtotal** | | **165,888,000** |
| shadow colour + shadow depth (B) | 2 × (2048×2048×4) | 33,554,432 |
| bloom (C): mips (540,960),(270,480),(135,240),(68,120),(34,60) = 690,600 px; horiz+vert = 2 × 690,600 × 8; bright = 518,400 × 8 | 11,049,600 + 4,147,200 | 15,196,800 |
| PMREM (D) | 768×1024×8 | 6,291,456 |
| **Steady-state accounted total** | | **220,930,688 B ≈ 210.7 MiB** |
| + one frame readback in flight (F) | 8,355,840 | **peak 229,286,528 B ≈ 218.7 MiB** |

These are **formula results, not measurements** — no run at HEAD
prints them. Their role: when the lead implements the ledger, a
film run's `accounted_bytes` must reproduce 220,930,688 for the
steady-state classes (given the default depth format) before
anyone trusts the line; a mismatch is a missed registration site,
and the table says where to look. MSAA caveat: the formula
charges full per-sample storage; tile-based drivers may hold
less physically — accounted is the contractual number, physical
residency is §5.

Context the table gives for free: the renderer's whole steady
device footprint (~211 MiB) is an order of magnitude below the
host paint cache's 2 GiB budget ceiling — on the target PC the
memory fight is host-side first, which is exactly the kind of
conclusion PERF §9.5's blind spot has been hiding.

## §5 What stays unknowable (stated plainly)

- **Driver-internal overhead.** Tiling/swizzle padding beyond
  the 256-byte row rule, alignment rounding, compression
  metadata, fragmentation: inside RADV/lavapipe, invisible to
  wgpu and to this process. `accounted_bytes` is a **lower
  bound**, always.
- **Pipeline and shader-module memory.** Compiled pipelines live
  in the driver; no wgpu 0.20 surface prices them.
- **Per-process device share.** Sysfs `mem_info_*` is
  device-wide; fdinfo (§2.4) may close this on the PC —
  TO-VERIFY — but from CI it is simply absent.
- **Shared-memory pressure as a single number.** On the 5650G,
  host RSS, GTT, and the VRAM carve-out all draw on the same
  ~6.7 GiB, and whether a device allocation also appears inside
  the allocating process's RSS depends on driver mapping. Any
  tool that prints one summed "total memory" figure is
  fabricating precision — the prototype (§7) prints components
  plus a labelled reference sum, and the design asks the lead to
  keep it that way.
- **Lavapipe runs have no device counters at all** — every CI
  number for this feature is the ledger's accounted figure or
  nothing.

## §6 Implementation sketch (for the lead)

1. `GfxLedger` in `gpucore.rs` (or a sibling module in the
   W06/W09/W19 pattern): `register_texture(bytes) ->
   LedgerToken`, `register_buffer(bytes)`, deregistration on
   token drop, current + peak watermarks, `stats_line()` in the
   §3 format. Log-only, no pixel path — the same determinism
   argument as runlog's module header.
2. Registration at the chokepoints of §2.2: `make_target`,
   `make_target_viewed`, `composer::make_rt`, `Readback::record`
   (peak-side only — readback is transient; feed
   `readback_bytes` + peak, not the end-of-run total),
   the upload lanes. Direct `create_*` sites register through an
   explicit `ledger.note_texture(bytes)` escape hatch so no site
   is silently skipped — and a grep-level check (every
   `create_texture`/`create_buffer` in `src/` is either inside
   gpucore/composer or within N lines of a registration) belongs
   in the acceptance, mirroring how W29's shader checker treats
   unresolved references.
3. Sysfs sampling at end of run (Linux only): first `cardN` with
   a `device/mem_info_gtt_used` file; read or `unavailable`.
   Never block, never fail the run over a counter.
4. Print sites per §3. Format-stability unit test in the house
   pattern (both existing stats modules pin theirs; this one
   must too).
5. Acceptance: film run reproduces §4's 220,930,688 steady-state
   figure (default depth format) and peak 229,286,528 with one
   readback in flight; PC run (W36 visit) verifies the sysfs
   fields against `radeontop` side by side.

## §7 Prototype — `tools/perf/vram_report.py`

Stdlib-only parser for the four line formats (runlog suffix,
paint-cache, buffer-pool, and the proposed gpu-mem), printing the
accounted report: device components, host components (with the
paint-cache-inside-RSS warning), OS counters, and the labelled
reference sum. Exit 0/1/2 per house convention. The fixture
(`tools/perf/fixtures/vram/film-run-tail.log`) is **synthetic** —
composed to the pinned formats, its gpu-mem figures set to §4's
formula totals so doc and fixture cross-check; it is not a
captured run, and the file says so in its header.

Self-test, run at HEAD `aa92cee` (verbatim):

```
$ python3 tools/perf/vram_report.py --self-test
  [PASS] gpu-mem line parsed
  [PASS] paint-cache line parsed
  [PASS] buffer-pool line parsed
  [PASS] peak RSS parsed
  [PASS] accounted == §4 formula total
  [PASS] peak == accounted + one frame readback
  [PASS] textures counted
  [PASS] readback == pinned FRAME_READBACK_BYTES
  [PASS] sysfs fields parse as unavailable (not 0)
  [PASS] paint peak == 74 x CANVAS_BYTES
  [PASS] paint peak under budget
  [PASS] pool peak == one readback buffer
  [PASS] pool budget == 64 MiB default
  [PASS] empty log parses to no components
  [PASS] report labels the reference sum as not-a-measurement
  [PASS] report states the double-count caveat
SELF-TEST: PASS (16 checks)

TaleRender accounted-memory report

DEVICE (GPU-side, renderer's own ledger):
  accounted (end of run) :   220,930,688 B (210.7 MiB)  [41 textures, 18 buffers]
  accounted (peak)       :   229,286,528 B (218.7 MiB)
  of which readback      :     8,355,840 B (8.0 MiB)  (transient staging, in the peak)
  buffer-pool peak       :     8,355,840 B (8.0 MiB)  (budget 67,108,864 B; class of the readback bytes above)

HOST (CPU-side):
  peak RSS (runlog)      :        2314.6 MB (as printed, MB = MiB)
  paint-cache peak       : 2,133,852,160 B (2035.0 MiB)  (budget 2,147,483,648 B)
    ^ host Vec<u8> canvases — already inside peak RSS;
      listed for the split, never added on top of it.

OS COUNTERS (device-wide, all processes — not this run alone):
  unavailable in this log (no amdgpu sysfs on the run machine,
  e.g. CI lavapipe — see the design note §2.4; never read as 0)

REFERENCE SUM (not a measurement): peak accounted device 229,286,528 B + peak RSS 2314.6 MB = 2,656,320,537 B (2533.3 MiB).
  On the shared-memory 5650G some device allocations may already
  sit inside RSS, so this sum can double-count; driver-internal
  overhead is in neither component, so it can also under-count.
  Use it only as a pressure eyeball against the PC's ~6.7 GiB.
```

Negative controls (verbatim): a log with no stats lines exits 1
with `ERROR: no recognised stats lines in log …` on stderr; a
missing file exits 2 with `ERROR: no such log file: …`.

## §8 TO-VERIFY register

| # | Claim | Verify how |
|---|---|---|
| V1 | `/sys/class/drm/cardN/device/mem_info_{vram,gtt}_{total,used}` + `mem_info_vis_vram_used` exist and read as raw bytes on the user's PC (RADV, kernel 7.0) | PC visit (W36 runbook): `cat` each path; add to the visit checklist |
| V2 | `/proc/self/fdinfo/` drm-memory keys + spelling on the PC's kernel | PC visit: open the renderer's drm fd during a run, `grep drm-memory` |
| V3 | wgpu 0.20 `Limits` field inventory (which ceilings, if any, are worth printing beside `adapter:`) | `grep -n "pub " ~/.cargo/registry/src/*/wgpu-0.20*/src/limits.rs` on a machine with the vendored crate, or in CI |
| V4 | wgpu 0.20 `Instance::generate_report()` existence + report contents | same vendored-crate grep, `instance.rs` |
| V5 | Bloom horiz/vert target count is exactly 2 × 5 mips + bright (§4 class C assumes composer.rs:493–505's loop spans all 5 mips) | lead confirms at ledger implementation; the formula, not the worked total, is the contract |
| V6 | Exactly one readback in flight on current call paths (peak = steady + 1 × class F) | lead confirms at implementation; if pipelining ever overlaps two, `readback_bytes` field semantics (largest single) still hold |

## §9 What this seat did not do

No `src/` edits, no PERF.md edit (the §9.5 placement fold-in is
§3's note to the lead), no wiring of the gpu-mem line into any
mode, no PC numbers of any kind — every figure above is a
formula, a pinned constant quoted from source, or in the §8
register. Nothing committed.
