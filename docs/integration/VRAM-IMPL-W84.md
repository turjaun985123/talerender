# VRAM-IMPL-W84 — VRAM telemetry: implementation staging pack

Wave 2, Batch 9, seat W84. Census taken at HEAD `021ca01`
(working tree; `src/ab1.rs` was in the lead's lane with
uncommitted changes when this seat started — ab1 line numbers
are the tree as read, and may shift by the lead's in-flight
edit; every other file was clean). **Staging only** — no `src/`
file was touched, nothing committed. The design of record is
Batch 6's `VRAM-TELEMETRY-W56.md` (§§ cited as W56 §n); this
pack re-derives its census and formulas against today's code,
prices the knobs era (F4 phases 1–3, `ea98458`/`9ad2177`/
`76b922c`), and hands the lead an insertion-ready patch shape.

Headline deltas vs W56's survey at `aa92cee`:

1. **The knobs are real now.** `Knobs` lives at
   `src/main.rs:173` (`FULL = { res: 1.0, aa: 4, shadow: 2048 }`,
   `samples()` at :186 maps aa 0 → 1). Every frame target's
   byte size is a function of `(effective W, effective H,
   samples, shadow)` — §2 gives the per-site expressions, §3
   the closed forms. At `Knobs::FULL` the W56 totals reproduce
   **exactly** (re-derived in §3, not copied).
2. **Bloom is per-call, and was at W56's HEAD too.** The chain
   (11 textures) is allocated inside `PostChain::bloom`
   (`src/composer.rs:493,504,505`) on every call — film calls
   it per frame (`src/film.rs:3342`) — and dropped at call end.
   W56's table counts class C in the steady total anyway. The
   ledger therefore needs one explicit convention to reproduce
   W56's acceptance numbers; §4 stages it as decision D2.
3. **ab1 never migrated.** Four of ab1's frame targets are
   still inline `create_texture` (`src/ab1.rs:816,835,849,
   862`); film's twins went through `make_target`. The census
   (§2) counts 12 direct `create_texture` escape-hatch sites
   outside the chokepoints; each needs W56 §6.2's explicit
   ledger note.
4. **Telemetry cannot print the line** — it is a CPU-only mode
   (no device is ever created; `src/main.rs:413` region), so
   its ledger would be all zeros, and the parser grammar
   (`tools/perf/vram_report.py`, `GPU_MEM_RE`) admits
   `unavailable` only for the sysfs fields. Staged as
   decision D1 (§4): omit the line in telemetry, deviating
   from W56 §3's telemetry placement, for the lead's ruling.

## §1 Census method and chokepoint counts

Method: `grep` census over `src/**/*.rs` for `make_target`,
`make_rt`, `create_texture`, `create_buffer`, `Readback::`,
re-run at this HEAD, each site then read in place for its size
expression. The format universe is still closed — and the
counts are byte-for-byte W56's: `Rgba16Float` 36 sites
(8 B/px), `Rgba8Unorm` 24 (4), `Depth32Float` 23 (4),
`Rgba8UnormSrgb` 14 (4). The knobs parameterised existing
sites; they added no format and no new texture site class.

| Chokepoint | Definition | Call sites at this HEAD |
|---|---|---|
| `gpucore::make_target` / `make_target_viewed` | `src/gpucore.rs:141` / `:164` (viewed delegates to plain) | **24** — film 5, ab1 1, scenefile 9, dof 7, main 2 |
| `composer::make_rt` | `src/composer.rs:72` (sample_count fixed 1) | **13 lines** — film 5, ab1 5, composer-internal bloom 3 (→ 11 textures per bloom call) |
| `gpucore::Readback::record` / `record_bpp` | `src/gpucore.rs:238` / `:252` (staging buffer created in `record_bpp`, ~:262) | **7** — film 1, ab1 1, scenefile 2, dof 2 (one `record_bpp`), main 1 |
| Upload lanes | per-lane, §2 | film 2 sites (one standing, one per-frame), ab1 1, scenefile 1 helper |
| Direct `create_texture` (escape hatch) | — | **12** — film 2, ab1 5, scenefile 3, dof 1, main 1 |
| Direct `create_buffer` | — | film 3 (all per-frame), ab1 5, dof 4, main 3; scenefile and composer route every buffer through one helper each (`scenefile.rs:1126`, `composer.rs:61`) — one insertion point per file |

Dimensions vocabulary used throughout §2: `(W, H)` are the
**effective** dimensions the lane was invoked with — for film,
`main.rs:413` computes `(ew, eh) = effective_size(width,
height, knobs.res)` (`main.rs:246`: res == 1.0 returns the
base exactly; else `round(v·res)`, floor 2, even-masked) and
`film::run_film(ew, eh, …)` builds `Renderer::new` at that
size, so every `width`/`height` inside film.rs **is** the
effective size. `s = knobs.samples()` (default 4),
`S = knobs.shadow` (default 2048), `p` = packed-depth bytes
per pixel (4 for the default `Rgba8Unorm`; 8 when
`TALERENDER_DOF_DEPTH_FORMAT=halffloat`, selected at
`film.rs:2623` and its ab1/scenefile/dof twins).

## §2 Allocation-site census, per lane

Class letters are W56 §4's (A frame targets, B shadow pair,
C bloom chain, D PMREM, E uploads, F readback, G buffers;
H host paint cache — excluded; I BufferPool — future).

### §2.1 Film lane (`src/film.rs`, `Renderer::new` unless noted)

Standing textures — all created once, held in the `Renderer`
(`tex_keep`), live at the completion print:

| Site | What | Size expression | Class |
|---|---|---|---|
| :2510 (upload closure, loop at :2530) | every painted/file texture in `textures_px` | Σ per texture `w×h×4` — **data-driven, no closed form**; the ledger counts live at the closure | E |
| :2535 | `composer::load_room_env` (PMREM) | `768×1024×8` = 6,291,456 (constant; asset-asserted at `composer.rs:104`) | D |
| :2617 | key shadow map (`make_target`, Rgba8Unorm, 1 sample) | `S×S×4` | B |
| :2620 | shadow depth (`make_rt`, Depth32Float) | `S×S×4` | B |
| :2628 | packed depth (`make_target`, `depth_format`) | `W×H×p` | A |
| :2631 | prepass depth (`make_rt`, Depth32Float) | `W×H×4` | A |
| :2634 | hdr msaa (`make_target`, Rgba16Float, `samples`) | `W×H×8×s` | A |
| :2637 | hdr resolved (`make_rt`, Rgba16Float) | `W×H×8` | A |
| :2640 | hdr depth (`make_target`, Depth32Float, `samples`) | `W×H×4×s` | A |
| :2643 | bokeh out (`make_rt`, Rgba16Float) | `W×H×8` | A |
| :2646 | output srgb (`make_rt`, Rgba8Unorm) | `W×H×4` | A |
| :2649 | final (`make_target`, Rgba8Unorm) | `W×H×4` | A |

Film-lane completeness note: film.rs contains exactly **two**
`create_texture` sites (:2510, :2867). Every texture the film
lane ever creates flows through one of them — the census for
this lane is complete by construction, not by sampling.

Per-frame transients — all inside `Renderer::render` (:2886),
dropped at return:

| Site | What | Size expression | Class |
|---|---|---|---|
| :3086 (`upload_dyn`, def :2866) | one card texture per `dyn_tex` entry, per frame | Σ `w×h×4` (Rgba8UnormSrgb) — alive through the frame's readback | E-transient |
| :2952 (`make_ub` closure) | frame uniforms | `frame_bytes.len()` = 448 (debug-asserted at :2950) | G-transient |
| :2963 / :2970 (`make_vb` closure) | vertex / index buffer per draw | `verts.len()×4` / `idx.len()×4` | G-transient |
| via :3342 → `composer::bloom` | bloom chain + its uniform buffers | chain per §3; buffers `bytes.len()` (16 B typical) per `make_buffer` call (`composer.rs:496,511,523,547,575`) | C + G-transient |
| :3364 | `Readback::record` of `final_tex` | `padded_row_bytes(W,4)×H` (`gpucore.rs:192`: unpadded rounded up to 256) | F (peak-only) |

Standing buffers in the film lane: **none** — film.rs has no
`create_buffer` outside `render()`. Class G for film is
entirely transient.

### §2.2 ab1 lane (`src/ab1.rs`, single still)

Same chain as film, built inline. Standing: upload closure
(:698, class E, Σ `w×h×4`), PMREM (:727, D, 6,291,456),
shadow map **inline** (:816, B, `S×S×4`), shadow depth (:827,
B, `S×S×4`), packed depth **inline** (:835, A, `W×H×p`),
prepass (:846, A, `W×H×4`), hdr msaa **inline** (:849, A,
`W×H×8×s` — `samples` from `knobs.samples()` at :591,
`shadow_size` at :592), hdr resolved (:860, A, `W×H×8`),
hdr depth **inline** (:862, A, `W×H×4×s`), bokeh (:873, A,
`W×H×8`), output srgb (:875, A, `W×H×4`), final via
`make_target_viewed` (:877, A, `W×H×4`). Bloom at :1499
(class C, same convention). Readback :1551 (F,
`padded_row_bytes(W,4)×H`). Buffers (G, note `descriptor.size`
at each site): frame uniforms :672 (`frame_bytes.len()`),
:1022, vertex/index :1078/:1085, :1095.

### §2.3 Scenefile lanes (`src/scenefile.rs`, two paths)

Shared: `upload_texture` helper (:1137, E, `w×h×4`
Rgba8UnormSrgb, per scene texture); **every** buffer flows
through `make_buffer` (:1126, G, `bytes.len()` — one
insertion point covers the file).

Direct path: frame target **inline** (:1452, A, `W×H×4`
Rgba8UnormSrgb), depth **inline** (:1463, A, `W×H×4`),
readback :1516 (F).

Composer path: scene msaa (:1618, A, `W×H×8×s` — `samples`
from `knobs.samples()` at :1564), scene resolved (:1619, A,
`W×H×8`), scene depth (:1620, A, `W×H×4×s`), packed depth
(:1621, A, `W×H×p`), prepass depth (:1622, A, `W×H×4`),
bokeh out (:1623, A, `W×H×8`), final (:1624, A, `W×H×4`),
fogged (:1632, A, `W×H×8`, **conditional** — fog only),
grade-src (:2000, A, `W×H×4`, **conditional** — grade
present), `PostChain` built only when bloom/grade is asked
(:1995), bloom at :2122 (C), readback :2172 (F).
**No shadow pair and no PMREM in either scenefile path** —
the knobs' shadow term is inert here (it still tags the run
as draft via `is_draft`, `main.rs:194`).

### §2.4 dof lane (`src/dof.rs`, chart still)

Chart texture **inline** (:180, E, `512×512×4` = 1,048,576 —
`TEX_SIZE` at :56, constant). Targets (:246–254, all
`make_target`): scene msaa `W×H×8×s` (`samples` at :141),
scene resolved `W×H×8`, scene depth `W×H×4×s`, packed depth
`W×H×p`, prepass depth `W×H×4`, bokeh out `W×H×8`, final
`W×H×4` — class A throughout. Readbacks: :498 (F,
`padded_row_bytes(W,4)×H`) and `record_bpp` :550 (F,
`padded_row_bytes(W,bpp)×H` with bpp = p — the fp16 depth
dump is 8 B/px under halffloat). Buffers (G): uniforms :170
(128 B), vertex/index :220/:227, bokeh uniforms :329. No
PostChain, no bloom, no shadow pair in this lane.

### §2.5 main m1 lane (`src/main.rs`, single still)

Character texture **inline** (:840, E, `256×256×4` = 262,144,
constant), frame target via `make_target_viewed` (:900, A,
`W×H×4` Rgba8UnormSrgb, **1 sample — m1 does not consume
`knobs.samples()`**), depth via `make_target_viewed` (:909,
A, `W×H×4`), readback :959 (F), buffers :647, :807, :814
(G, `descriptor.size` at site).

### §2.6 Shared internals

- `composer::make_rt` body (`composer.rs:79`) — the
  `create_texture` inside it is the chokepoint itself.
- `composer::load_room_env` (`composer.rs:102`, texture at
  :105) — class D wherever called (film :2535, ab1 :727;
  **not** called by scenefile or dof).
- `composer::make_buffer` (`composer.rs:61`) — every
  PostChain pass buffer (bloom/output/grade uniforms) flows
  through it; one insertion point, class G-transient.
- `gpucore::Readback::record_bpp` buffer creation
  (`gpucore.rs` ~:262) — class F staging; the ledger note
  belongs in `record_bpp` so both `record` and `record_bpp`
  callers are covered by one insertion.
- Bind groups, bind-group layouts, pipelines, shader
  modules, samplers: created in quantity, **unpriced** —
  W56 §5 stands (no wgpu 0.20 surface prices them); they are
  not ledger classes and the census does not pretend
  otherwise.


## §3 The formula, re-derived at this HEAD

Derived here from §2's size expressions and the closed
format universe (Rgba16Float 8 B/px; Rgba8Unorm /
Rgba8UnormSrgb / Depth32Float 4 B/px; texture bytes =
`w×h×bpp×samples`, layers = 1 everywhere — no site in the
tree creates a layered texture).

### §3.1 Film lane at `Knobs::FULL`, 1080×1920, default depth format

`W×H = 1080×1920 = 2,073,600 px`; `s = 4`; `S = 2048`;
`p = 4`.

| Term | Arithmetic | Bytes |
|---|---|---|
| hdr msaa | 2,073,600 × 8 × 4 | 66,355,200 |
| hdr depth | 2,073,600 × 4 × 4 | 33,177,600 |
| hdr resolved | 2,073,600 × 8 | 16,588,800 |
| packed depth | 2,073,600 × 4 | 8,294,400 |
| prepass depth | 2,073,600 × 4 | 8,294,400 |
| bokeh out | 2,073,600 × 8 | 16,588,800 |
| output srgb | 2,073,600 × 4 | 8,294,400 |
| final | 2,073,600 × 4 | 8,294,400 |
| **A subtotal** | | **165,888,000** |
| shadow pair (B) | 2 × (2048×2048×4) | 33,554,432 |
| bloom (C, §3.2) | | 15,196,800 |
| PMREM (D) | 768×1024×8 | 6,291,456 |
| **Steady total (A+B+C+D)** | | **220,930,688** |
| + one frame readback (F) | padded row 4352 × 1920 | 8,355,840 |
| **Peak** | | **229,286,528** |

The readback row: `padded_row_bytes(1080, 4)` — unpadded
4320, rounded up to the 256-byte alignment
(`gpucore.rs:192`) = 4352; `4352×1920 = 8,355,840`, agreeing
with `bufferpool.rs:52`'s pinned `FRAME_READBACK_BYTES`.
**220,930,688 and 229,286,528 reproduce** — computed from
today's code, they land on W56 §4's figures to the byte,
which is the cross-check that the knobs refactor changed
parameterisation, not the default allocation set.

Closed form for class A (film and ab1 chains):
`A = W·H·(12s + p + 28)` — terms `8s` (msaa) + `4s` (hdr
depth) + `8` (resolved) + `p` (packed) + `4` (prepass) +
`8` (bokeh) + `4` (srgb) + `4` (final). At defaults:
`2,073,600 × 80 = 165,888,000`. ✓ Scenefile's composer
chain drops the srgb term: `W·H·(12s + p + 24)`, plus
`8·W·H` if fogged, `4·W·H` if grade-src exists.

### §3.2 Bloom chain arithmetic

`bloom_mip_sizes(W,H)` (`composer.rs:41`): five sizes,
halving with round-half-up from `(W/2, H/2)`. At
1080×1920: (540,960), (270,480), (135,240), (68,120),
(34,60) = 518,400 + 129,600 + 32,400 + 8,160 + 2,040 =
690,600 px. Per `bloom()` call: bright at mip 0 (518,400 px)
+ horiz and vert at **every** mip (2 × 690,600 px), all
Rgba16Float: `C = 8 × (518,400 + 2×690,600) = 8 ×
1,899,600 = 15,196,800`. ✓ (11 textures.) General form:
`C(W,H) = 8 × (px₀ + 2·Σᵢ pxᵢ)` over `bloom_mip_sizes`.

### §3.3 Knob-general film formula (what the ledger must compute)

```
(W,H) = effective_size(base_w, base_h, res)     main.rs:246
s     = if aa == 0 { 1 } else { aa }            main.rs:186
A     = W·H·(12s + p + 28)
B     = 8·S²                                    (S = knobs.shadow)
C     = 8·(px₀ + 2·Σ pxᵢ),  px = bloom_mip_sizes(W,H)
D     = 6,291,456
F     = padded_row_bytes(W,4)·H                 (frame readback)
steady = A + B + C + D;   peak = steady + F     (unit-replay form, §6)
```

Spot values the implementer can check by hand: `--draft`
(res 0.5, aa 0, shadow 512) at base 1080×1920 gives
(W,H) = (540,960), s = 1, A = 518,400×44 = 22,809,600,
B = 2,097,152, C: mips (270,480), (135,240), (68,120),
(34,60), (17,30) = 172,710 px, so C = 8×(129,600 +
2×172,710) = 3,800,160, steady = 34,998,368. Halffloat packed depth at FULL
(p = 8): A = 2,073,600×84 = 174,182,400, steady =
229,225,088, peak = 237,580,928.

Two accounting subtleties the formula already encodes:

- **Allocated, not used.** At aa = 0 the hdr msaa texture
  is still allocated (film.rs:2634 is unconditional; only
  the attachment view switches, film.rs:3272–3273), so its
  `W·H·8×1` counts. The ledger follows allocation, always —
  it hooks creation sites, never render passes.
- **`--res` scales the readback too** (F is a function of
  the effective W), and at res = 1 `effective_size` returns
  the base **exactly** (no float path), so the default
  figures are integer-exact, not approximate.

## §4 Design rulings this staging needs from the lead

**D1 — telemetry prints no gpu-mem line.** W56 §3 placed
the line in film telemetry after `cache.stats_line()`
(film.rs:3703). But telemetry never creates a device
(CPU-only audit; the mode early-returns before any GPU
dispatch, main.rs:413 comment), so the ledger there is
structurally zero — and `accounted_bytes=0` in a log is a
fabricated-looking measurement of exactly the kind the
house discipline forbids (runlog prints `peak RSS
unavailable`, sysfs prints `unavailable`, never 0). The
parser forecloses the honest spelling anyway:
`GPU_MEM_RE` admits `unavailable` only for the two sysfs
fields. **Recommendation: omit the line in telemetry**;
render modes only. The verdict line remains telemetry's
whole contract.

**D2 — bloom registration is a retained-registration
convention.** The chain's 11 textures are created and
dropped on every `bloom()` call (film: 1,811 calls), yet
W56's acceptance total counts one chain in the steady
figure and `peak = steady + F` — which only composes if
the chain is counted as a standing working set (during any
frame's post pass exactly one chain is live, and it is
live in *every* frame). Two implementations, identical
numbers: (a) **register once** — `PostChain` computes the
chain bytes from `bloom_mip_sizes(w,h)` at its first
`bloom()` call for a given (w,h) and registers them as
standing class C (plus a debug assertion / re-register if
(w,h) ever changes mid-instance; in today's code it never
does); or (b) **hoist** — move the 11 allocations into
`PostChain` fields, created lazily at first bloom, making
plain creation-site accounting literally true. (a) is
staged below (zero allocation-motion); (b) is the cleaner
end-state if the lead prefers no convention. Either way
the registration site must carry the comment: *one chain
counted; a future pipelined overlap of two chains
under-counts peak by one chain (15,196,800 at defaults).*

**D3 — ledger home: a section in `src/gpucore.rs`.**
W56 §6.1 allowed gpucore or a W06/W09/W19-pattern sibling
module. Staged in gpucore because its two biggest clients
(`make_target`, `Readback`) live there and registration
inside them then needs no cross-module path; the sibling
(`src/gpumem.rs` + one mod-list line at main.rs:18–36)
remains a five-minute alternative if the lead prefers the
module-per-stats precedent (paintbudget/runlog/bufferpool
are all siblings).

## §5 Patch shape

### §5.1 The ledger (new section in `src/gpucore.rs`)

Process-global, because no context object threads through
`Renderer`, `PostChain`, and the four still lanes — and one
mode runs per process, so a global cannot mix runs.
Interior-mutable via `std::sync::Mutex` (const-constructible;
no new deps). Log-only: it reads descriptors and returns
numbers; it is never on a pixel path (W56 §6.1's
determinism argument).

```rust
pub(crate) enum MemClass { A, B, C, D, E, F, G }  // W56 §4 letters

pub(crate) struct GpuMem { /* current, peak, textures,
    buffers, readback_max, per-class [u64; 7] */ }

static GPU_MEM: Mutex<GpuMem> = Mutex::new(GpuMem::new());

pub(crate) fn mem_alloc_texture(class: MemClass, bytes: u64);
pub(crate) fn mem_free_texture(class: MemClass, bytes: u64);
pub(crate) fn mem_alloc_buffer(class: MemClass, bytes: u64);
pub(crate) fn mem_free_buffer(class: MemClass, bytes: u64);
/// Peak-side only (W56 §6.1): raises peak by `bytes` over
/// current, records the largest single readback; never
/// enters `current` (the staging buffer is freed inside
/// collect, before any end-of-run print).
pub(crate) fn mem_note_readback(bytes: u64);
/// Snapshot/restore guard for per-frame transients
/// (film render() scope, PostChain pass scope): restores
/// current + counts on drop; peak is NOT restored
/// (it is a watermark).
pub(crate) struct MemScope { /* snapshot */ }
pub(crate) fn mem_scope() -> MemScope;

pub(crate) fn mem_stats_line() -> String;  // §5.3 format
#[cfg(test)] pub(crate) fn mem_class_subtotals() -> [u64; 7];
#[cfg(test)] pub(crate) fn mem_reset();
```

Semantics pinned: `peak = max(peak, current)` after every
alloc; `mem_note_readback(b)` does `peak = max(peak,
current + b)` and `readback_max = max(readback_max, b)`
without touching `current`. `snapshot()` for the line:
`accounted_bytes = current` (classes A–E + G standing;
class F never enters current), `textures`/`buffers` =
live counts.

Format-stability unit test in the house pattern (both
existing stats modules pin theirs): `mem_stats_line`
output matches the §5.3 grammar byte-for-byte on a
synthetic ledger state, plus the §6 replay test.

### §5.2 Per-site insertions

Insertion points — chokepoint bodies where one insertion
covers every caller; per-caller lines where the class or
size expression differs by caller:

| Insertion point | Insert | Covers |
|---|---|---|
| `gpucore::make_target` body (:141) | `mem_alloc_texture(class?, w×h×bpp(format)×samples)` — **but the class differs per caller** (A vs B at film :2617). Stage: `make_target` takes no class today; add a `mem_alloc_texture(MemClass::A, …)` default **and** have the two B call sites (film :2617 is make_target; ab1's shadow is inline) pass via a thin `make_target_classed` wrapper — or simpler: make_target does not self-register; see the per-caller rows below. **Staged decision: per-caller registration** (one line after each call, size expression repeated from the call) — 24 lines, zero signature churn, and the grep acceptance (§6.4) can see every one. | — |
| `composer::make_rt` callers | per-caller line, same pattern (13 lines) | film/ab1 chains |
| `composer::make_buffer` body (:61) | `mem_alloc_buffer(G, bytes.len())` — lifetime handled by the caller's `MemScope` (PostChain methods take one scope per call) | all PostChain pass buffers |
| `gpucore::Readback::record_bpp` body (~:262) | `mem_note_readback((padded_row * height) as u64)` | all 7 readback sites |
| `composer::load_room_env` body (:102) | `mem_alloc_texture(D, 768×1024×8)` (use `ROOM_ENV_W/H` consts, :99–100) | film + ab1 PMREM |
| `PostChain::bloom` head (composer.rs:~492) | D2: first-call registration `mem_alloc_texture(C, 8×(px₀ + 2·Σpxᵢ))` from `bloom_mip_sizes(width, height)`; one `mem_scope()` for the method's buffers | bloom chain |

Per-caller registration lines (class, size expression —
each is the call's own arguments run through
`w×h×bpp×samples`; a `gpucore::tex_bytes(w, h, format,
samples) -> u64` helper keeps them one-liners):

- **film:** :2617 B `S×S×4` via `tex_bytes(shadow_size,
  shadow_size, Rgba8Unorm, 1)`; :2620 B (make_rt) same
  with Depth32Float; :2628 A `tex_bytes(W,H,depth_format,1)`;
  :2631 A Depth32Float; :2634 A `tex_bytes(W,H,Rgba16Float,
  samples)`; :2637 A Rgba16Float ×1; :2640 A Depth32Float
  ×samples; :2643 A; :2646 A Rgba8Unorm; :2649 A Rgba8Unorm;
  upload closure :2510 E `w×h×4` (the closure's own w/h —
  format is a closure parameter but every caller's data is
  4 B/px, bytes_per_row `w*4` at :2520); `render()` head:
  one `mem_scope()` (covers :3086 E-transient uploads —
  which still `mem_alloc_texture(E, w×h×4)` per call —
  and :2952/:2963/:2970 `mem_alloc_buffer(G, size)`).
- **ab1:** :698 E (closure); :816 B inline; :827 B;
  :835 A; :846 A; :849 A ×samples; :860 A; :862 A
  ×samples; :873 A; :875 A; :877 A; buffers :672, :1022,
  :1078, :1085, :1095 G with each descriptor's `size`
  expression duplicated into the note.
- **scenefile:** `upload_texture` body (:1137) E
  `w×h×4` (covers all its callers); direct :1452 A,
  :1463 A; composer :1618 A ×samples, :1619 A, :1620 A
  ×samples, :1621 A `depth_format`, :1622 A, :1623 A,
  :1624 A, :1632 A (inside its fog conditional), :2000 A
  (inside the grade `map`). Buffers: free — `make_buffer`
  (:1126) gets the same body insertion as composer's
  (G, `bytes.len()`); scenefile stills are single-shot, so
  no scope is needed (buffers stand until the print).
- **dof:** :180 E (512×512×4); :246–254 A ×7 (msaa/depth
  at ×samples); buffers :170, :220, :227, :329 G.
- **main:** :840 E (256×256×4); :900 A; :909 A; buffers
  :647, :807, :814 G.

Escape-hatch discipline (W56 §6.2): every `create_texture`
/ `create_buffer` in `src/` is then either inside a
chokepoint body or within a few lines of a `mem_alloc_*`
line — the 12 inline sites above (:2510 film counted as a
lane site, ab1 ×5, scenefile ×3, dof ×1, main ×1, film
:2867) each carry their note at the site, not in a helper
they don't share.

### §5.3 Print points and the line

The line (W56 §3, grammar already pinned by the parser —
do not change field order or spelling):

```
gpu-mem: accounted_bytes=<u64> peak_accounted_bytes=<u64> textures=<u64> buffers=<u64> readback_bytes=<u64> sysfs_gtt_used_bytes=<u64|unavailable> sysfs_vram_used_bytes=<u64|unavailable>
```

| Mode | Print immediately after | Site |
|---|---|---|
| film render | the completion `println!` block | `film.rs:3558–3561` → new line after it (Renderer still alive there; standing set counted) |
| ab1 | `wrote {out}` println | `ab1.rs:1557` |
| scenefile direct | `wrote {out}` println | `scenefile.rs:1522` |
| scenefile composer | `wrote {out}` println | `scenefile.rs:2177` |
| dof | `wrote` println | `dof.rs:508` |
| main m1 | `wrote {out}` println | `main.rs:965` |
| film telemetry | **none** (decision D1) | — |

One line per run, its own line — never appended to the
completion line (whose composition is pinned by runlog's
contract: counts, time+RSS suffix, cache stats last).

### §5.4 Sysfs probe

At print time, Linux only, in gpucore next to the ledger:
iterate `card0..card8` under `/sys/class/drm/` (or
`read_dir` and take the first `cardN` whose
`device/mem_info_gtt_used` exists); read
`device/mem_info_gtt_used` and `device/mem_info_vram_used`,
trim + parse `u64`. Any failure at any step → that field
prints the literal `unavailable`. Never errors, never
blocks, never fails the run — the `.ok()`-chain discipline
of `runlog::peak_rss_mib` (`src/runlog.rs:167–176`) is the
house template.

**Verified on this VM (this seat):** `ls /sys/class/drm/`
shows only `version` — no `cardN` at all. The probe's
unavailable path is therefore the only locally (and CI-)
testable path, and it must be exercised by a unit test
that points the probe at an empty temp dir. The numeric
path is PC-only and stays TO-VERIFY (W56 §8 V1/V2) until
the W36 visit: counters are **device-wide** (all
processes), and on the 5650G the VRAM figure is a carve-out
of the same system RAM — neither is this process's share.

## §6 Acceptance

1. **Parser fixture (already green, must stay green).**
   `python3 tools/perf/vram_report.py --self-test` →
   `SELF-TEST: PASS (16 checks)` (re-run by this seat at
   this HEAD: PASS). Its fixture
   (`tools/perf/fixtures/vram/film-run-tail.log`) carries
   the expected line verbatim:
   `gpu-mem: accounted_bytes=220930688 peak_accounted_bytes=229286528 textures=41 buffers=18 readback_bytes=8355840 sysfs_gtt_used_bytes=unavailable sysfs_vram_used_bytes=unavailable`.
   The fixture is synthetic (its header says so); 41/18 are
   its composed counts, **not** predictions — a real film
   run's counts are data-driven (class E) and its
   `accounted_bytes` will exceed 220,930,688 by exactly
   E + G. Do not "fix" the fixture to match a real run.
2. **Ledger replay test (the exact gate).** A Rust unit
   test replaying the film lane's standing registrations
   at (1080, 1920, s = 4, S = 2048, p = 4) through the
   ledger API asserts: class subtotals A = 165,888,000,
   B = 33,554,432, C = 15,196,800, D = 6,291,456;
   `accounted` (A–D) = **220,930,688**; after
   `mem_note_readback(8_355_840)`: peak =
   **229,286,528**, `readback_bytes` = 8,355,840. Plus
   the halffloat variant (p = 8): A = 174,182,400,
   steady 229,225,088. This test needs no GPU — the
   ledger is pure arithmetic — so it runs in the normal
   CI `cargo test` leg.
3. **CI dispatch (render.yml, film mode, lavapipe).**
   The run's tail must contain a gpu-mem line such that:
   `vram_report.py` parses the log (exit 0);
   `sysfs_*` both `unavailable`; `readback_bytes` =
   8,355,840 exactly (every frame readback at default
   size is forced to that figure); `accounted_bytes` ≥
   220,930,688 with the excess equal to the ledger's own
   E + G subtotals (read via the test accessor in a
   debug build, or accepted from the unit replay + the
   line's internal consistency: `peak − accounted` ≥
   `readback_bytes`, equality iff no transient outlived
   the standing set at print time).
4. **Grep completeness (W56 §6.2).** A tools-ci check in
   the W29 shader-checker pattern: every `create_texture`
   / `create_buffer` occurrence in `src/` is inside
   gpucore/composer chokepoint bodies or within N lines
   of a `mem_alloc_*` / `mem_note_readback` line; the 12
   escape-hatch sites of §1 are the allowlist's test.
5. **PC (W36 visit, unchanged from W56 §6.5):** sysfs
   fields numeric on RADV, cross-checked against
   `radeontop`; V1/V2 of W56 §8 discharged there or kept
   open honestly.

## §7 Honesty traps — restated for the implementer

1. **Paint-cache bytes are HOST memory, never device.**
   `paintbudget.rs`'s `bytes`/`peak_bytes` are `Vec<u8>`
   canvases on the CPU (28,835,840 B each, 2560×2816
   RGBA). They already print on the completion line's
   `paint-cache:` segment. The ledger must never ingest
   them; a painted canvas's device cost exists only when
   it is uploaded (class E, at the upload site). On the
   shared-memory 5650G both pressures are real — and they
   are still two numbers, in two labelled lines, never
   summed by the binary (the labelled REFERENCE SUM in
   `vram_report.py` is the only sanctioned addition, and
   it is a report-tool eyeball, not a measurement).
2. **BufferPool bytes are not in the ledger.** At this
   HEAD `src/bufferpool.rs` still has **no production
   caller** (grep: zero references outside its own file —
   re-verified by this seat). Its stats line in the
   fixture is parser-readiness only. If the
   gpucore::Readback adoption later wires the pool, its
   retained buffers are class F allocations owned by the
   pool: register them **once**, at pool retention (W56
   class I) — never again per `collect`, or every frame's
   readback double-counts.
3. **accounted_bytes is a lower bound, always.** Driver
   tiling/swizzle padding beyond the 256-byte row rule,
   alignment rounding, compression metadata,
   fragmentation, pipeline and shader-module memory, bind
   groups and samplers: inside the driver, invisible to
   wgpu 0.20, unpriced here (W56 §5 stands verbatim). The
   MSAA terms charge full per-sample storage; a
   tile-based driver may hold less physically. Accounted
   is the contractual number, residency is not claimed.
4. **Knob state travels with the number.** A gpu-mem
   figure from a `--draft` run is a draft-sized figure
   (§3.3); the completion line beside it carries the
   `[DRAFT … — NOT FOR DELIVERY]` tag, and any quoting of
   the line without that context is a category error of
   the same kind as quoting lavapipe timings as hardware
   performance.
5. **The bloom convention is a convention** (D2): one
   chain counted standing. It is exact for today's call
   pattern (one chain live per frame, never two) and
   silently wrong by one chain if post passes are ever
   pipelined. The comment at the registration site is
   part of the implementation, not decoration.
6. **`unavailable`, never 0** — sysfs fields, and (per
   D1) no line at all where no GPU work happened.

## §8 What this seat did not do

No `src/` edits, no PERF.md edit (the §9.5 placement
fold-in remains the lead's, per W56 §3), no wiring, no PC
numbers — every figure above is re-derived arithmetic from
the code's own size expressions, a pinned constant quoted
from source, or the verified-empty sysfs of this VM.
Nothing committed.
