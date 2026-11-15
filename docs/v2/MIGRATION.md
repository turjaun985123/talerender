# TaleRender — Migration guides

## Part A — wgpu 0.20 → 30 (from the actual upgrade commits)

The upgrade landed **alone** (no feature changes in the same
commits), under the pixel-neutrality law, in this range
(Stage 1 item 0; user directive 2026-11-15 "must use everything
latest versions crate", targets verified on crates.io at
implementation time):

| Commit | Part | Content |
|---|---|---|
| `c246ec8` | 1 | Version bump ONLY: `wgpu 0.20 → 30.0.2`, `pollster 0.3 → 1.0.1`, `image 0.25 → 0.25.10` in `Cargo.toml`. |
| `5e1bd05` | 2 | API migration fallout, one transform per CI error cluster (list below). |
| `985ed04` | 3 | Repair of the part-2 script's multiline `VertexBufferLayout` splits (9 sites) + the shader checker learns the wgpu 30 shapes. |
| `b9098d2` | 4 | Repair of the last split literal (scenefile depth pipeline — the transform had also comma-split an inline `VertexAttribute` array). |
| `65e4ea2` | 5 | `RenderPipelineDescriptor.cache` field + `PollType::Wait` struct form. |

(`d882bbc`, the V2 master prompt commit, sits inside the range
but is docs-only.) Final dependency state, exact-pinned in
`Cargo.toml`: `wgpu = "30.0.2"`, `pollster = "1.0.1"`,
`image = "0.25.10"` — still exactly three direct dependencies.

### A.1 The API changes actually hit

From the part-2 commit message and the diff, verified against
the current tree:

- **Option-ification, broadly:** `entry_point: Some(..)`,
  `depth_write_enabled: Some(..)`, `depth_compare: Some(..)`,
  elements of `bind_group_layouts: &[Some(&layout), …]`, and
  elements of `VertexState.buffers` (`&[Option<VertexBufferLayout>]`
  — including the four `make_pbr`/`make_depth` closure signatures
  that take them).
- **`push_constant_ranges: &[]` → `immediate_size: 0`** on
  `PipelineLayoutDescriptor`.
- **`multiview: None` → `multiview_mask: None`** on pipeline
  descriptors, and `RenderPassDescriptor` gained a
  `multiview_mask` field; `RenderPassColorAttachment` gained
  `depth_slice: None`.
- **Renames:** `ImageCopyTexture` → `TexelCopyTextureInfo`,
  `ImageCopyBuffer` → `TexelCopyBufferInfo` (per the rename
  family in the commit), `ImageDataLayout` →
  `TexelCopyBufferLayout`.
- **`SamplerDescriptor.mipmap_filter`** now takes a
  `MipmapFilterMode` value instead of the old filter enum form.
- **`InstanceDescriptor` no longer implements `Default`:**
  construct with `InstanceDescriptor::new_without_display_handle()`
  (gpucore does exactly this, then sets `backends = VULKAN`).
- **`request_adapter` returns `Result`, not `Option`**
  (0.20), and the options gain `apply_limit_buckets: false` —
  set false to keep the adapter's exact limits (0.20 behaviour).
  gpucore's two-step high-performance → forced-fallback logic
  is unchanged around it.
- **`request_device` is single-argument** in the new API shape
  (the descriptor carries label/features/limits); gpucore's
  wrapper signature for callers is unchanged.
- **`PollType::Wait`:** `Maintain::Wait` is gone. Part 5 makes
  it the struct variant
  `PollType::Wait { submission_index: None, timeout: None }`,
  which is `Maintain::Wait`'s exact semantics (wait for all
  queued work, no timeout).
- **`get_mapped_range` is now fallible** — readback code must
  handle the `Result`.
- **`RenderPipelineDescriptor` gained `cache:
  Option<&PipelineCache>`** — all 23 literals in the tree set
  it to `None` (no pipeline cache in use; 0.20 behaviour).

### A.2 The checker-pattern updates

The migration broke the tree's own tooling in a characteristic
way, and the fixes are part of the guide:

- The part-2 mechanical transform comma-split one multiline
  `VertexBufferLayout` literal per file and wrapped its
  *fields* in `Some(..)`. The correct shape is a **single
  `Some(..)` wrap of the whole layout literal**. Part 3 repaired
  9 layout-level sites; part 4 repaired the last one, whose
  `attributes` array is an inline `VertexAttribute` literal the
  transform had split too. Lesson for any future scripted
  migration: Option-wrapping transforms must operate on whole
  literals, never on comma-split field lists — and field values
  were verified against the pre-migration text at `c246ec8`
  before the repair was accepted.
- `tools/ci/check_shaders.py` (the WGSL↔Rust agreement checker)
  was taught the wgpu 30 shapes: unwrap `Some(..)` around
  `bind_group_layouts` elements and accept
  `entry_point: Some(..)`. Its WGSL↔Rust contract itself is
  unchanged; the part-3 commit records it verified locally at
  21/21 PASS, 0 unresolved.

### A.3 The pixel-neutrality proof method

The method, as recorded in `V2-CONTRACTS.md` (Phase A gate) and
`STAGE1-SCOPE.md` — this is the reusable recipe for any future
dependency migration:

1. Land the upgrade **alone**, in its own commit range, against
   the pre-upgrade head as baseline (`d101770`).
2. **Byte probes:** render the standard film ranges and compare
   frame bytes against the baseline hash file
   (`~/workspace/talerender-evidence/hashes-post.txt`, ranges
   195–204, 795–814, 1195–1204 — 40 frames). Required result:
   **40/40 identical**.
3. **Telemetry:** the probe workflow must print
   `FILM TELEMETRY: CHECK OK`.
4. **Tests + smoke:** full suite green (recorded: 195/195) and
   the CI smoke render green.
5. **Local golden verify:** golden pins verify against the
   pre-upgrade head (recorded: 17 OK). Pins are local-only
   (rendered images never go to GitHub).
6. **Stop rule:** if any frame byte differs, STOP, quantify the
   diff (frames, PSNR, pipeline stage) and report it as a user
   decision. Never re-pin goldens over an unexplained diff.

Recorded outcome for this migration: Phase A gate **PASSED** —
CI at `65e4ea2` green (build + 195/195 tests + smoke), probe
run `43688575920` telemetry CHECK OK, byte probes 40/40 vs
`d101770`, golden verify 17 OK. (These are citations of the
repo's own records; this seat did not re-run them —
**not verified** independently.)

## Part B — v1.0.1 → V2: what changed for integrators

v1.0.1 is the released binary (`talerender 1.0.1`, six gates
proven on the bump SHA `967fa37`). V2 at HEAD `712a6aa` is a
source-tree state, **not a new release**: `Cargo.toml` still
says `1.0.1`, and no V2 version bump exists in the tree.

### B.1 Unchanged (safe to rely on)

- **The CLI is byte-for-byte the v1.0.1 CLI.** `const HELP`,
  every flag, every default, every exit code — see `CLI.md`.
  No existing flag changed behaviour.
- **All production render paths are unchanged in behaviour:**
  PNG film rendering with resume, scene files, built-in scenes,
  telemetry gates, the composer chain, `film/finish.sh`.
- **Determinism gates stand:** the same byte-probe ranges and
  `FILM TELEMETRY: CHECK OK` gate every landing (§4 of
  V2-CONTRACTS).
- **The native backend's place in Tall Tale Shorts is
  unchanged:** opt-in in the shipped skill, Chrome default.

### B.2 Changed under the hood

- **Dependencies:** wgpu 0.20 → 30.0.2, pollster 0.3 → 1.0.1,
  image 0.25 → 0.25.10 (Part A). Anyone embedding or patching
  TaleRender code must use the wgpu 30 API shapes listed in
  A.1 — wgpu 0.20-era snippets (bare `entry_point`, `multiview`,
  `ImageCopy*`, `Maintain::Wait`) will not compile.
- **VRAM ledger is live:** render runs now print the `gpu-mem:`
  line (grammar in `GPU.md`); log parsers may rely on its field
  order/spelling (pinned by `tools/perf/vram_report.py`).
  Film-telemetry runs deliberately print no such line.

### B.3 Added but inert (integrators: do not build on these yet)

- Nine Batch 1 modules are compiled into the binary and carry
  unit tests, but have **no production call sites and no CLI
  surface**: `dirtyrect`, `mempressure`, `scheduler`, `streamq`,
  `ffstream`, `projectfile`, `assetstream`, `observability`,
  `config`. Their public-in-crate APIs and delivery-note wiring
  specs are stable references for the lead's integration, but a
  downstream consumer cannot invoke any of them through the
  binary today: `--mem-profile`, `--adapter-policy`,
  `--output-mode`, `--perf-record-dir`, `--config` are unknown
  flags (exit 2); `talerender.toml` is never read; streaming
  output cannot be selected.
- `tools/perf/lab/bench_lab.py` **is** usable today as a
  standalone tool (it drives the binary through its existing
  `--film` CLI) — see `PERF-LAB.md`.

### B.4 Batch 2 state at delivery (re-checked)

Landing in this wave (files exist as untracked seat files,
**not yet registered in `main.rs` or lead-integrated** — so
still inert for integrators): `src/lifetimes.rs` (WP1.4) +
`docs/integration/V2-WP1-LRU-SPEC.md` (WP1.1 spec; the
paint-cache FIFO → LRU change itself is lead-applied and has
NOT happened), `src/memreport.rs` (C2 rendering half — blocked
on the lead-owned `gpucore::MemSnapshot`, which does not exist
yet; GPU timestamps remain unimplemented), `src/streammodes.rs`
(C3 modes + recovery), `tools/export/export_project.mjs` (C4
exporter), `tools/integration/validate_native_project.py` +
`docs/integration/V2-WP9-INTEGRATION.md` (WP9),
`src/outputmgr.rs` (WP11: canonical frame naming, output-dir
scan/resume-gap analysis, `render-meta:` metadata written
atomically via `.part` + rename), `tools/perf/lab/report.py`
(WP6 reporting).

Still CONTRACT only (files absent at final check):
`src/adapter.rs` (C7 policies + device-loss handler) and
`src/charstate.rs` (WP10). For those two, the §23 status is
INCOMPLETE — code absent, reason: their Batch 2 seats had not
landed files when this set was delivered.
