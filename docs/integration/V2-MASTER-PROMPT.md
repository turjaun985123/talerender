# MASTER EXECUTION PROMPT — TALERENDER V2 (FIXED, 2026-11-15)

Complete Native Rendering Engine | 20-Agent Parallel Engineering Mission

This is the corrected mission document. It supersedes the draft V2 prompt and the
earlier 10-work-package roadmap. Corrections are marked **[FIX]** where they change
the draft's meaning; everything else preserves the user's strategy as given.

---

## 0. CURRENT STATE — READ FIRST (source of truth)

The draft prompt described older repository archives. The active working trees are
ahead of them. Verified against the live repositories on 2026-11-15:

**Already released / landed — DO NOT rebuild, DO NOT redo:**
- TaleRender **v1.0.1 is released** (GitHub release, CI-built binary, six gates proven
  on the bump SHA). v1.0.0 remains untouched.
- User decisions of 2026-11-15 are EXECUTED: the 73-chunk caption split stands;
  the s28 golden pin is RETIRED; lip-sync Variant A is landed (generator-side
  closed-exit step keys); locomotion is registered AND wired at all seven film
  walk sites (858 frames changed, all inside the gait footprint, 0/923 control
  frames touched).
- The native backend is **adopted into the shipped Tall Tale Shorts skill (v6.0)
  as an opt-in backend** (`template/tools/capture_native.mjs` + selftest hook).
  Chrome is the default renderer. This decision is final for V2.
- Memory infrastructure exists: byte-denominated paint budget (2 GiB derived
  ceiling, `src/paintbudget.rs`), LRU buffer pool (`src/bufferpool.rs`), texture
  atlas module (`src/atlas.rs`), GPU core module (`src/gpucore.rs`), RunLog.
- Golden pins and A/B evidence images are **LOCAL-ONLY** (user standing rule:
  rendered photos/frames NEVER go to GitHub; they are delivered to the user as
  zips). Pin hashes/metrics live in docs as text. `make_golden.py` has an explicit
  LOCAL-ONLY mode. CI cannot verify image pins; pin verification is a local act.
- **Stage 1 is already in flight** (user order 2026-11-15 11:55 +06) and is
  ABSORBED by this mission as its first landings — do not duplicate it:
  - Scope doc landed: `docs/integration/STAGE1-SCOPE.md` (commits 239f4f0, d5a79d8).
  - **WP2's core (VRAM telemetry) is LANDED**: commit f9429a8 (gpucore ledger +
    gpu-mem line, per design pack VRAM-IMPL-W84; formula 220,930,688 B steady /
    229,286,528 B peak).
  - **Dependency upgrade part 1 is LANDED**: commit c246ec8 (see §2).
  - Remaining Stage 1 items — paint-cache LRU (→ V2 WP1.1), adapter reporting +
    device-loss (→ V2 WP3), benchmark automation tooling (→ V2 WP6) — continue
    as V2's first work packages under the Stage 1 lead until the V2 lead takes
    integration over. One integrator owns the tree at a time; handover is explicit.

**Staged designs that already exist — start from them, do not reinvent:**
- §15 evaluator step 2 (loader consumption of clips/prefabs; the s12 code-side
  retirement rides it, byte-probe gated) → the foundation of V2 WP5.
- Split programmes (22 serial pure-move commits, precondition discharged) —
  NOT part of V2 unless the V2 lead schedules them explicitly.
- PC session pack (`docs/integration/PC-SESSION-W98.md`) — the RADV/hardware
  leg. It belongs to the VALIDATION wave, not this implementation wave.

**Known open items V2 does not close (validation wave / user):**
- No RADV hardware evidence exists. Every native number is CI lavapipe.
  V2 must never present lavapipe numbers as hardware results.
- The film-s03 golden pin is stale for a non-gait reason (pre-dates blink
  adoption). Recorded pin debt; V2 does not silently re-pin it.
- Colour QA: the user reported the native Onoda film looks pale/faded
  (2026-11-14); global saturation measured equal-or-higher than Chrome.
  V2 WP5/WP10 work must not "fix" this by changing production defaults.

---

## 1. THE MISSION

You are a coordinated team of 20 senior software engineers, graphics programmers,
Rust engineers, GPU specialists, rendering architects, video pipeline engineers
and integration specialists.

Your mission is to take the existing TaleRender and Tall Tale Shorts repositories
and implement the complete next-generation TaleRender V2: a professional,
browser-independent, low-memory, GPU-accelerated rendering engine capable of
replacing Chrome Headless in Tall Tale Shorts.

This is a CODE IMPLEMENTATION MISSION.

The user has explicitly chosen this development strategy:

FIRST: Implement all V2 features, write the code, integrate all modules and
produce the complete build.

SECOND: After the implementation wave is complete, perform comprehensive testing,
benchmarking, debugging, visual comparisons and optimization in a separate wave.

Do not turn this assignment into another audit-only, planning-only,
benchmarking-only or testing-heavy wave.

The final deliverable must be working integrated source code, not merely
architecture documents or recommendations.

**[FIX] Minimal per-landing gates are NOT deferred** (see §21): this repository's
standing law is that every landing keeps CI green, the existing test suite passing,
film telemetry at CHECK OK, and the film's determinism byte-probes intact. Those
are the definition of "integrated" here, not testing-wave activities. What IS
deferred is campaigns: benchmark campaigns, hardware validation, visual-regression
campaigns, stress testing, fault injection.

---

## 2. DEPENDENCIES — LATEST VERSIONS ONLY **[FIX]**

User directive (2026-11-15): *"must use everything latest versions crate"* —
verified against crates.io on 2026-11-15:

| Crate    | Was   | Latest (verified) | Status |
|----------|-------|-------------------|--------|
| wgpu     | 0.20  | **30.0.2**        | part 1 landed (c246ec8) |
| pollster | 0.3   | **1.0.1**         | part 1 landed (c246ec8) |
| image    | 0.25  | **0.25.10**       | part 1 landed (c246ec8) |

Rules:
- TaleRender has exactly **three direct dependencies** (zero-new-deps law).
  Any addition needs a written justification in the delivery ledger, approved
  by the chief architect. Transitive versions resolve to latest compatible.
- At implementation time, **re-check crates.io** for every crate you touch.
  If a newer version than the table exists, use it and record the change.
- Pin exact versions in `Cargo.toml` (e.g. `wgpu = "30.0.2"`, not `"30"`), and
  record the full resolved dependency tree in the handover ledger.
- CI uses the latest stable Rust toolchain.
- The wgpu 0.20 → 30 migration lands ALONE (no feature changes in the same
  commits) under a **pixel-neutrality proof**: byte probes over the standard
  film ranges + telemetry CHECK OK + local golden verify against the
  pre-upgrade head (d101770). If any frame byte differs, STOP, quantify the
  diff (frames, PSNR, pipeline stage) and report it as a user decision.
  Never re-pin goldens over an unexplained diff.

---

## 3. EXISTING CAPABILITIES — PRESERVE THEM

TaleRender: Rust + wgpu native renderer; Vulkan architecture; native character
painting; 3D environment rendering; scene format v2; cinematic cameras; depth
of field; PBR; PMREM environment sampling; shadows; bloom; color grading; fog
and atmosphere; film rendering; PNG frame generation; FFmpeg finishing; native
film telemetry; deterministic frame evaluation; QA infrastructure; memory-budget
infrastructure; LRU buffer pool; atlas module; gpucore; RunLog; professional CLI
(-h/--help, -V/--version, panic-free errors, --res/--aa/--shadow knobs with
[DRAFT] tagging); telemetry exit codes (0 OK / 1 FAILED / 3 could-not-run) and
in-binary telemetry tables.

Tall Tale Shorts: native backend as opt-in; Chrome default; character
locomotion; blinking; lip-sync Variant A; character definitions and pose
systems; shot and scene definitions; camera systems; environment systems;
captions and audio; production QA.

Do not rebuild these capabilities. Extend them through compatible interfaces.

---

## 4. PRIMARY ENGINEERING TARGET

A low-memory Linux production machine:
- AMD Ryzen 5 PRO 5650G, AMD Radeon Vega 7 integrated GPU, RADV/Vulkan.
- **Approximately 6.7 GiB usable system RAM** (the user's actual machine —
  low-memory is the PRIMARY target, not an edge case).

High-end computers are secondary targets. Every major subsystem must respect
constrained system memory and shared GPU/system memory.

Design principles: bounded memory; GPU-first rendering; deterministic
animation; minimal unnecessary copying; efficient texture management; resource
reuse; progressive asset loading; bounded frame processing; browser-independent
execution; **no silent quality reduction, ever** (no reduced resolution/FPS/CRF
for speed — standing user ruling).

Do not assume a dedicated GPU with abundant VRAM exists.

---

## 5. WORK PACKAGE 1 — ADVANCED MEMORY MANAGEMENT (CRITICAL)

The byte-denominated paint budget and its 2 GiB derived ceiling already exist.
Do not rebuild the global budget. **[Stage 1 item 2 folds in here.]**

5.1 **Paint-cache eviction.** Replace FIFO eviction with byte-aware LRU
(recency on hit); byte budget is the binding constraint, entry cap a secondary
guard. Frequently reused character states stay cached. Cache hit/miss telemetry
and eviction-reason tracking surfaced via RunLog/telemetry. Safe invalidation;
no stale texture references. **HARD LAW: zero pixel change** — proven by byte
probes + telemetry CHECK OK + local golden verify. If eviction order changes
any frame byte, stop and report.

5.2 **Dirty-region texture uploads.** Track changed regions; compute and merge
dirty rectangles; partial uploads with full-upload fallback; preserve exact
existing character appearance; deterministic; handle multiple animated
characters; avoid unnecessary GPU synchronization.

5.3 **Memory-pressure controller.** A central layer coordinating paint cache,
buffer pool, texture atlas, render targets, scene assets, frame queues and
temporary resources, with configurable profiles:
- LOW_MEMORY: 6–8 GB system RAM target (the primary machine), conservative
  preloading, aggressive reclamation of unused resources, strict transient
  limits, bounded frame processing.
- BALANCED: moderate caching and preloading.
- HIGH_PERFORMANCE: larger allowances where beneficial.
All profiles remain memory-bounded. Never silently reduce output quality.

5.4 **Resource lifetime management.** Explicit ownership; safe disposal; reuse
of compatible GPU allocations; unused-texture reclamation; temporary-allocation
tracking; lifetime diagnostics.

## 6. WORK PACKAGE 2 — VRAM AND GPU TELEMETRY (CRITICAL)

**Core implementation is LANDED (f9429a8)** — the gpucore allocation ledger and
the gpu-mem telemetry line, per VRAM-IMPL-W84. V2 completes the remainder:
- A stable machine-readable reporting interface for the telemetry (the ledger
  data, adapter identity, backend, tracked current/peak allocations, creation/
  destruction counters, allocation history).
- GPU timestamp measurements where the adapter supports them (capability-
  detected, never faked).
- Honest separation, always: (1) tracked renderer allocations, (2) driver-
  reported memory when available, (3) estimates — labelled as estimates.
  For integrated GPUs, state the shared-memory limitation. Never report
  allocator estimates as physical VRAM usage.

## 7. WORK PACKAGE 3 — ADVANCED GPU RENDERING ENGINE (HIGH)

Extend the existing wgpu architecture on the NEW wgpu 30 APIs. Do not replace
the framework. **[Stage 1 item 3 folds in here.]**
- Adapter management: hardware adapter discovery; vendor/device/backend
  identification; capability inspection; adapter preference policies;
  integrated-vs-dedicated detection where the API exposes it.
- **Adapter reporting: the selected adapter and backend appear in EVERY render
  report. Software rendering is never reported as hardware rendering.**
- Device reliability: device-loss detection and reporting; recovery strategy
  where wgpu 30 supports it; safe shutdown; resource recreation strategy;
  where true recovery is impossible, a clear fatal error (diagnosed, correct
  exit code) — documented honestly as a limitation.
- GPU resource optimization: pipeline reuse; shader compilation cache;
  bind-group reuse; texture reuse; buffer/render-target pooling integration;
  reduced unnecessary readbacks; efficient command submission.
- Fallback: RADV, Intel, NVIDIA and other Vulkan adapters; software Vulkan
  when available — always labelled.

## 8. WORK PACKAGE 4 — DIRECT FFMPEG STREAMING (VERY HIGH)

Genuinely new. Build an OPTIONAL streaming rendering mode:
Film Timeline → deterministic frame evaluator → native GPU renderer →
readback buffer pool → bounded frame queue → FFmpeg rawvideo stdin → MP4.
- Sequential deterministic frame generation; configurable batching; controlled
  readback; reusable staging buffers.
- Bounded queues with backpressure; configurable capacity; **no unlimited
  buffering, no full-film RAM accumulation** (critical on the 6.7 GiB target).
- FFmpeg: rawvideo stdin; configurable pixel format/resolution/fps; audio
  integration and A/V sync (the existing `data/mix.wav` contract); encoder
  configuration; process lifecycle management; stderr capture; exit-code
  handling.
- Reliability: temporary `.part` output discipline (existing project law);
  safe finalization; interrupted-render recovery; frame-count accounting;
  final MP4 validation (frame count, duration, loudness path unchanged).
- Three modes, explicitly configured and reported: (1) PNG sequence (existing,
  preserved), (2) direct streaming, (3) automatic with reported fallback.
  Never silently switch output quality or encoding parameters. Final encode
  remains x264 CRF 18 (production law).

## 9. WORK PACKAGE 5 — UNIVERSAL TALL TALE EVALUATOR (HIGH)

The existing adapter is real but proven only through the Onoda bridge.
Generalize it. **Start from the staged §15 evaluator step-2 design** (loader
consumption of clips/prefabs) — it is the foundation, not a competing plan.
- Support: project metadata/version; scenes; shots; character registries,
  poses, moods; animation, locomotion, facial and lip-sync tracks; camera
  tracks; environments; props and attachments; captions; word timing; audio
  cues; transitions; lighting; materials; atmosphere; QA telemetry.
- Versioned project interchange contract: the TypeScript authoring pipeline
  produces native-compatible project data with schema validation, version
  compatibility checks, asset manifests, relative paths, missing-asset
  reporting, and clear compatibility errors.
- Do NOT attempt to translate arbitrary JavaScript closures into Rust.
  Explicit supported authoring APIs and serialization contracts only.
- No hardcoded Onoda-specific assumptions in the evaluator; project-specific
  data is configuration; project-specific adapters stay isolated.
- The s12 code-side retirement rides this work, byte-probe gated
  (frames 1666–1810) as its pack specifies.

## 10. WORK PACKAGE 6 — RENDER PERFORMANCE LABORATORY (HIGH)

Build the tooling and reporting infrastructure. **Do NOT run the hardware
benchmark campaign in this wave; do NOT fabricate or prepopulate results.**
The existing PERF tables stay blank until genuine measurements exist.
**[Stage 1 item 4 folds in here.]**
- Four comparison configurations: (A) Chrome GPU, (B) Chrome software,
  (C) TaleRender GPU, (D) TaleRender software.
- Metrics: process/per-core CPU; current + peak RSS; allocation history where
  available; cache and frame-queue memory; adapter identity/backend; GPU
  allocation estimates and driver-reported memory where available; GPU frame
  timings where supported; total render time; mean/P50/P95/P99 frame times;
  FPS; dropped/failed frames; texture upload volume; cache hit rate; output
  resolution/fps/frame count/file size/encoding settings/hashes; visual
  comparison metadata.
- Automation: reusable scenarios; machine-readable results; Markdown report
  generation; historical storage; configuration comparison; hardware metadata;
  provenance; missing-data handling; a CLI to run benchmarks later.
- Software-leg (lavapipe) runs MAY be executed to prove the tooling works and
  must be labelled lavapipe. GPU rows remain PENDING-PC.

## 11. WORK PACKAGE 7 — ADVANCED RENDER SCHEDULER

Frame scheduling; configurable concurrency; bounded task queues; CPU/GPU
workload coordination; job priorities; cancellation; progress reporting;
checkpoint metadata; resume support; graceful shutdown; error propagation;
deterministic output ordering. Designed for low-memory machines: no unlimited
parallel frame rendering, no unnecessary simultaneous GPU allocations, long
films without holding all frames in memory. Determinism is never compromised
by parallel state.

## 12. WORK PACKAGE 8 — SCENE AND ASSET STREAMING

Progressive asset loading; asset manifests; texture deduplication; scene asset
reuse; resource residency tracking; safe unloading; environment texture
management; large-texture handling; asset dependency resolution; missing-asset
diagnostics. Where beneficial: mipmap management, resolution-aware loading,
LOD selection, instanced repeated geometry, frustum culling. None of it may
alter the default visual appearance.

## 13. WORK PACKAGE 9 — TALL TALE PRODUCTION INTEGRATION

The native backend is adopted as opt-in. Preserve that decision. Improve:
backend configuration; native project export; project validation; renderer
invocation; error handling; output management; progress reporting; FFmpeg
integration; QA integration; render metadata. **Chrome remains the default
during this wave. Do not change the default backend. Do not remove Chrome
compatibility.** Leave a clean future migration path for a separate acceptance
phase to make native the default.

## 14. WORK PACKAGE 10 — CHARACTER RENDERING AND ANIMATION COMPATIBILITY

Preserve all implemented animation capabilities. Do not rebuild locomotion,
blinking or lip-sync Variant A. Focus on integration and rendering efficiency:
efficient character state caching; character atlas integration; partial texture
updates (with WP1.2); animation state evaluation; deterministic facial
animation; pose interpolation; prop attachment compatibility; character
visibility management; multiple-character rendering efficiency. Characters
remain 2D characters inside dimensional 3D environments — the project's
non-negotiable identity. Never generic 3D humanoids.

## 15. WORK PACKAGE 11 — RENDER OUTPUT AND ENCODING SYSTEM

PNG output compatibility; direct MP4 streaming integration (WP4); configurable
intermediate output; output directory management; safe temporary files; render
metadata; frame numbering; audio synchronization; encoder configuration;
finalization reporting. Preserve the current production finishing workflow
(x264 CRF 18, faststart, `.part` discipline, mix.wav muxed unchanged).
Do not unnecessarily replace FFmpeg.

## 16. WORK PACKAGE 12 — ERROR HANDLING AND OBSERVABILITY

Unified structured errors: category, code, human-readable message, project and
scene, frame number, resource, recovery possibility, suggested corrective
action. Build on the existing TaleResult migration (Wave 2 landed 2 steps;
complete it where V2 touches code). Structured logs for renderer startup,
adapter initialization, asset loading, scene evaluation, frame production,
memory pressure, GPU operations, encoder operations, output finalization.
Never hide errors behind generic success messages.

## 17. WORK PACKAGE 13 — PROFESSIONAL CLI AND CONFIGURATION

Extend the existing professional CLI without breaking existing commands
(the v1.0.1 CLI contract: -h/--help, -V/--version, panic-free errors, knob
flags with [DRAFT] tagging). Add clear commands for: streaming rendering;
memory profile selection; adapter selection; performance report generation;
telemetry output; project validation; asset inspection; render recovery.
Configuration via CLI arguments, config files, and environment overrides with
documented precedence. Useful --help for every command.

## 18. WORK PACKAGE 14 — DOCUMENTATION AND DEVELOPER EXPERIENCE

Update: README; architecture; memory architecture; GPU architecture; adapter
compatibility; streaming architecture; CLI documentation; project interchange
schema; configuration reference; error reference; performance laboratory
guide; Tall Tale integration guide; migration guide. Documentation describes
the ACTUAL implemented code. Never document hypothetical functionality as
complete.

---

## 19. TEAM ORGANIZATION — 20 AGENTS **[FIX: honest execution shape]**

The 20 roles from the draft stand (chief architect; repository/compatibility;
paint-cache; dirty-region; memory-pressure; GPU adapter; GPU reliability; GPU
telemetry; texture/atlas; rendering performance; FFmpeg streaming; frame
scheduling; evaluator architect; TypeScript export; production integration;
benchmark infrastructure; performance reporting; error handling; documentation;
integration/release).

**[FIX] Execution reality, stated plainly:** the parallel ceiling is 10 agents
per batch. The team therefore runs as **one chief-architect/lead + seats in two
batches of 10**, exactly as Waves 1–2 ran (100 seats each, zero rejected
landings). Report ACTUAL agent counts at every batch close — never the ordered
number as if it were the run number.

Ownership law (proven in Waves 1–2): the lead owns the existing core files
(`src/*.rs`, `Cargo.toml`, workflows, format/integration docs) and integrates;
seats write NEW files/modules in their owned areas and never commit; the lead
commits centrally with explicit paths. No two agents modify the same file.
Interfaces and data contracts are defined by the lead BEFORE parallel
implementation. No duplicate modules. No new dependencies without ledger
justification (see §2).

## 20. STANDING LAWS (override any conflicting instruction in any pack)

1. **CI is the only build.** No local cargo builds on the VM (no rustc in PATH;
   verification is GitHub Actions). Report EVERY CI verdict the moment it is
   checked, pass or fail. A cancelled run (concurrency supersession) is not a
   failure — say which it is.
2. **Verify claims against code and run output.** This project's record has
   contained proven errors (a phase reported landed that did not exist).
   Never relay a doc/commit claim as fact without checking.
3. **No rendered photos/frames/evidence images to GitHub, ever.** Images are
   delivered to the user as zips in chat. Docs carry hashes/metrics as text.
4. **Artifact quota discipline:** GitHub artifact storage quota was exhausted
   once by ~4.2 GB frame artifacts. CI artifacts stay small (binaries, logs,
   hashes). Never upload full frame sets.
5. **Determinism is sacred.** frame(t) evaluation stays deterministic; the
   Onoda film's byte-probes and telemetry CHECK OK gate every landing that
   touches the render path.
6. **No silent quality reduction; no fabricated results; no placeholder
   success.** If a capability is unavailable, implement detection + honest
   fallback and label it.
7. **The user's brief and rulings outrank this prompt** where they conflict
   (opt-in backend, Chrome default, low-memory primary target, latest crates).

## 21. IMPLEMENTATION-FIRST EXECUTION POLICY

Phase A — Short source audit: consume STAGE1-SCOPE.md and the staged packs;
confirm boundaries. Do not spend the wave auditing.
Phase B — Architecture contracts: interfaces, ownership, data contracts.
Phase C — Full implementation: real Rust, TypeScript, schemas, CLI and
integration code. No plans-as-deliverables, no pseudocode.
Phase D — Integration: wire everything into the existing repositories; resolve
interface mismatches.
Phase E — Build completion: the complete V2 tree, with the §1 minimal gates
green at every landing (CI success, existing tests passing, telemetry CHECK OK,
determinism probes intact).
Phase F — Handover: freeze and deliver the handover report (§23).

DEFERRED TO THE VALIDATION WAVE (do not spend this wave on them): full
performance benchmarking; Vega 7 hardware validation; full Chrome-vs-native
comparisons; extensive visual-regression campaigns; large-scale stress
testing; broad memory-profiling campaigns; full device-loss fault injection;
optimization driven by uncollected performance data. Do not invent results.
Do not claim untested functionality is production-verified.

## 22. CODE QUALITY REQUIREMENTS

Modular architecture; clear ownership; consistent naming; explicit resource
lifetimes; predictable error handling; bounded memory behaviour; minimal
unnecessary allocations; safe concurrency; deterministic frame evaluation;
backward-compatible interfaces where possible; documented public APIs;
maintainable code. **No fake implementations, no placeholder success responses,
no empty modules presented as finished, no hardcoded benchmark results, no fake
GPU utilization, no fabricated VRAM readings.** If a feature depends on
unavailable hardware capabilities, implement detection and fallback without
claiming hardware validation.

## 23. FINAL DELIVERABLES AND HANDOVER

Source: complete TaleRender V2 implementation; updated Tall Tale integration;
memory improvements; GPU telemetry; device reliability; direct FFmpeg
streaming; universal evaluator; performance laboratory; render scheduler;
asset streaming; CLI improvements.
Docs: V2 architecture; subsystem docs; interchange contract; streaming docs;
configuration reference; integration guide; migration guide; known limitations.
Build: complete updated source tree; release build where the environment
permits; configuration files; CLI reference; dependency inventory (exact
versions, §2).

Handover report, per work package, using EXACTLY these statuses:
- **IMPLEMENTED** — code exists and is integrated.
- **BUILD VERIFIED** — the relevant build completed successfully.
- **NOT VALIDATED** — comprehensive testing intentionally deferred.
- **INCOMPLETE** — precisely what remains, and why.
Plus: implemented features; modified files; new files; new dependencies;
architecture decisions; integration status; build status; features awaiting
validation; hardware-dependent limitations; known unfinished items.
Never confuse these statuses.

## 24. DEFINITION OF SUCCESS

TaleRender V2 is implementation-complete when: (1) all agreed work packages
have actual code implementations; (2) memory improvements are integrated;
(3) GPU telemetry is implemented; (4) device-loss handling and adapter
reporting are implemented; (5) direct FFmpeg streaming is integrated;
(6) the universal evaluator supports a generalized project contract;
(7) the performance laboratory is implemented; (8) the render scheduler
supports bounded processing; (9) Tall Tale integration remains compatible;
(10) existing character animation features are preserved; (11) the native
backend remains opt-in; (12) Chrome compatibility remains intact; (13) all
modules are integrated into the source tree; (14) documentation reflects the
actual code; (15) a complete V2 build is delivered.

**[FIX]** (16) **The native backend does NOT become the default renderer in
this mission** — Chrome remains the default (the draft's §24.15 said the
opposite of its own §13; §13 governs). (17) V2 is NOT claimed hardware-
validated until the validation wave runs the PC session.

## 25. FINAL INSTRUCTION

Build first. Integrate everything. Complete the code. Think like engineers
building a real native rendering product: aggressive about implementation,
disciplined about architecture, conservative about modifying proven
functionality. The outcome is a unified, extensible, low-memory, GPU-first
rendering engine that can eventually replace Chrome Headless entirely in
Tall Tale Shorts — proven in the next wave, on the user's hardware.
