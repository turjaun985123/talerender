# STAGE1-SCOPE — TaleRender 2.0, Stage 1: scope against the current tree

Authority: user order 2026-11-15 11:55 +06 — "Scope Stage 1 and
start the unblocked work", where Stage 1 is Stage 1 of the
external TaleRender 2.0 roadmap the user approved scoping
("Stabilize the native engine": actual Radeon/Vulkan GPU
testing, byte-based memory budget, cache and resource
profiling, benchmark automation, GPU device recovery).

Method: every claim below was verified against the tree at
HEAD `d101770` (post-Wave-2, post-decisions-execution) on
2026-11-15 — code read in place, not the roadmap's snapshot
and not the wave record. Where the roadmap (written against
older zip snapshots) disagrees with the tree, the tree wins
and the disagreement is recorded in §5.

## §1 The roadmap's Stage 1, mapped to the current tree

| Roadmap Stage-1 item | State at `d101770` (verified) | Disposition |
|---|---|---|
| Actual Radeon/Vulkan GPU testing | No RADV number exists anywhere; every native figure is CI lavapipe. The session pack exists (`PC-SESSION-W98.md`, runbook F1-fixed). | **PC-BLOCKED** — §4. Not executable from this VM; not faked, not filled. |
| Byte-based memory budget | **LANDED.** `src/paintbudget.rs`: byte budget with a 2 GiB derived ceiling (`BUDGET_CEILING_BYTES`), env override, entry cap as secondary guard; `src/bufferpool.rs`: LRU GPU buffer pool (module + tests; **no production caller yet** — verified by grep, unchanged since W84). `src/atlas.rs`: packing half landed, GPU wiring not done (per-card upload stands). | Remainder = paint-cache eviction policy → **item 2**. Atlas wiring and buffer-pool adoption are NOT Stage 1 (they change upload paths; they ride their own gates later). |
| Cache and resource profiling | Paint stats print (`paint-cache:` line: entries/bytes/hits/misses/evictions/rerenders/peak/budget) and RunLog prints elapsed + peak RSS. **GPU-side: nothing** — no VRAM ledger exists; the implementation pack is staged (`VRAM-IMPL-W84.md`, formula re-derived byte-exact: steady 220,930,688 B / peak 229,286,528 B at FULL defaults, film lane, excl. data-driven uploads + buffers). | **Item 1** (VRAM telemetry per W84, incl. its rulings D1/D2). |
| Benchmark automation | `bench.yml` exists: two lavapipe legs (cap 160 / cap 24), `time -v`, logs-only artifact, `tools/perf/bench_report.py` renders PERF-shaped tables. Gaps: **no frame-time percentiles** (runlog's `TALERENDER_FRAME_TIMING` switch exists but has **no caller in the film loop** — verified by grep: the per-frame timing lines it documents were never wired); **no output hashes** (frame sizes only); no committed software-leg record (results live in expiring artifacts). | **Item 4** (wire per-frame timing, add hashes + percentile record, commit the record as text). |
| GPU device recovery | Adapter identity **is** printed at all five render entry points (film `renderer:` line; ab1/main/scenefile canonical `adapter:` line) — but dof's line hardcodes "— software rasteriser, no performance claim", which would **mislabel hardware as software on RADV**. Device loss: **no handling exists** — no lost-device callback, no detection, no diagnosed failure path. | **Item 3** (truthful adapter reporting everywhere + device-loss detection/diagnosis within the wgpu 0.20 surface; recovery only if the API honestly supports it). |

WP remainders folded in: WP1 (memory) → items 1–2; WP2 (GPU
engine) → item 3 + the PC leg; WP10 (performance laboratory,
Stage-1 slice) → item 4. WP3 (FFmpeg streaming), WP4's
evaluator depth (§15 step 2), WP5–WP9 are **not** Stage 1 and
are not touched.

## §2 Phase B — the implementation items, in landing order

Each item lands separately (commit + push + CI verdicts
checked at its head). A red gate stops that item's chain;
byte-probe failure on a zero-pixel item stops it outright.

**Restructure (user directive 2026-11-15 12:15 +06,
corrected 12:16):** "must use everything latest versions
crate" — a dependency upgrade is inserted as **item 0**,
landing before items 2–4 and alone. Item 1 had already
landed (`f9429a8`) when the directive arrived; its probe
evidence doubles as the baseline-equivalence proof the
upgrade's pixel gate needs (see item 0).

### Item 0 — Dependency upgrade: everything to latest

Directive: all dependencies to their latest versions.
Verified on crates.io at implementation time (2026-11-15):

| Dependency | Current | Target (verified latest) |
|---|---|---|
| wgpu | 0.20 | **30.0.2** (published 2026-11-12; MSRV rust 1.87) |
| pollster | 0.3 | **1.0.1** |
| image | 0.25 | **0.25.10** (MSRV rust 1.88) |
| Rust toolchain | `dtolnay/rust-toolchain@stable` in every workflow (no pin, no rust-toolchain file) | unchanged — floating latest stable IS the policy; the upgrade build's rustc version is recorded from its CI log |

Transitives resolve fresh (the repo carries no Cargo.lock,
by standing policy), so the whole tree lands at latest
compatible under the new direct specs.

**Lands alone.** The upgrade's commit range contains only
the version specs and the API-migration fallout the new
versions force (call-shape renames, descriptor fields,
poll/copy type renames). No feature work, no behaviour
changes smuggled in; item 1's code is touched only where
the wgpu 30 API compels it.

**Pixel-neutrality gate (hard).** Byte probes over the
standard ranges + film telemetry CHECK OK + local golden
verify, against the pre-upgrade render — the local baseline
of §3 (src-identical to `d101770`; item 1's probe run at
`f9429a8` establishes that the current head renders
byte-identically to it before the upgrade starts). wgpu
upgrades can shift output subtly (naga translation, output
stage). **If ANY frame byte differs, the upgrade chain
stops**: the diff is quantified (frame count, PSNR, the
pipeline stage responsible) and reported as a user
decision. No waving through, no re-pinning over it.

**If latest cannot pass:** if wgpu 30 cannot build or pass
the CI lavapipe leg for a genuine upstream reason, the
blocker is documented with evidence and the newest version
that DOES pass is landed, with the gap reported — never a
silent stay on 0.20.

**Design impact on items 1–4.** (1) VRAM ledger: its
`tex_bytes` prices formats via
`TextureFormat::block_copy_size` — survival/shape in wgpu
30 is confirmed at the upgrade build; if the API moved,
the migration adapts that one call (forced fallout). The
formula's semantics (byte sizes per format) do not change.
(2) Paint-cache LRU: no wgpu surface — unaffected. (3)
Adapter reporting + device loss: now targets the **wgpu
30** APIs (AdapterInfo shape, the lost-device surface) —
the 0.20-era survey in the original item-3 text is moot.
(4) Benchmark automation: unchanged in design; its first
record will be a post-upgrade record.

**Edition 2024: DEFERRED, deliberately.** The directive
targets crate versions. An edition migration is orthogonal,
tree-wide churn (reserved keywords, lifetime-capture
rules) that would confound this item's pixel-neutrality
isolation — a differing frame byte could no longer be
attributed to the dependency change alone. The toolchain
supports edition 2024 (stable ≥ 1.85), so this is a
sequencing choice, not a blocker: edition 2024 gets its
own gated change after item 0 lands.

### Item 1 — VRAM telemetry (per `VRAM-IMPL-W84.md`)

The gpucore ledger (W84 D3), per-caller registration at the
pack's §5.2 insertion points, the pinned `gpu-mem:` line at
the pack's §5.3 print points, the sysfs probe (§5.4), the
replay unit test (§6.2), and the tools-ci grep-completeness
check (§6.4). Rulings honoured: **D1 — film telemetry prints
no gpu-mem line** (telemetry is CPU-only; a zeroed ledger
line would be a fabricated-looking measurement, and the
parser grammar forecloses the honest spelling); **D2 —
bloom is per-call**: one chain is registered once, as a
standing working set, at `PostChain`'s first `bloom()` call
for a given size, with the convention comment at the site.

Acceptance: replay test asserts the class subtotals and the
220,930,688 / 229,286,528 figures; a CI film render's tail
carries a parseable `gpu-mem:` line with `sysfs_*` =
`unavailable` and `readback_bytes` = 8,355,840;
`vram_report.py --self-test` stays PASS (16 checks); film
telemetry still `CHECK OK`, exit 0; **zero frame-byte
changes** (byte probes vs the baseline in §3).

### Item 2 — Paint-cache eviction: FIFO → byte-aware LRU

`PaintCache` (film.rs) keeps the byte budget as the binding
constraint and the entry cap as the secondary guard
(`PaintBudget::should_evict` already encodes both); the
eviction order changes from insertion order to
least-recently-used: a hit refreshes the key's recency. The
stats surface gains a hit-rate field (appended to the
`paint-cache:` line, after the parser-pinned prefix).

**Hard law — zero pixel change.** Painting is a pure
function of the exact state key (film.rs `state_key`, f32
bit patterns), so eviction order can only change how often
`paint_toon_full` re-runs, never its output. Proof: byte
probes over the standard ranges + telemetry CHECK OK +
local golden verify 17 OK (LOCAL-ONLY mode, reported as
such — pin images are local by the no-photos rule). If any
probed frame differs, the item stops and reports; the
painter's output is sacred.

### Item 3 — Adapter reporting + device loss (WP2 remainder)

- dof's adapter line stops claiming "software rasteriser"
  unconditionally: it reports the same fields as the other
  lanes (name / backend / device type / driver), with the
  software note printed **only when the adapter's device
  type is actually Cpu** — CI lavapipe output stays
  byte-identical to today; RADV output becomes truthful.
- Device loss: survey the wgpu **30** surface as built by
  CI (item 0 moved the target; the 0.20-era survey is moot).
  If a lost-device callback exists, wire detection to a
  clean, diagnosed failure (named exit + message identifying
  the loss) at every render entry point. True
  recreate-and-resume recovery is implemented **only** if
  the API supports it without fabricating state; otherwise
  the limitation is documented here (§6 follow-ups) and in
  code comments. No fabricated capabilities.

Acceptance: CI + Tools CI green; the still lanes' adapter
lines unchanged on lavapipe except dof's (identical there
too — Cpu branch); byte probes unaffected (log-only change,
probes still run as the guard).

### Item 4 — Benchmark automation (WP10, Stage-1 slice)

- Wire runlog's per-frame timing switch into the film
  render loop (log-only, env-gated, default off — the
  switch and its documented line were built in Wave 2 and
  never connected).
- Extend `bench.yml` + `bench_report.py`: per-leg frame
  hashes (sha256, text), frame-time p50/p95/p99 from the
  per-frame lines, peak RSS, total render time, cache
  stats, renderer identity — composed into a bench record.
- The record is committed as text in
  `docs/runbook/BENCH-SOFTWARE-RECORD.md` (linked from
  PERF.md), headed with the lavapipe honesty label. GPU
  rows remain **PENDING-PC** — never filled from lavapipe
  numbers.
- Run the extended bench once; report the numbers
  lavapipe-labelled.

Acceptance: bench dispatch SUCCESS with the record
produced; default (timing-off) film output byte-identical
(byte probes); artifacts stay logs/text only (quota law).

## §3 Evidence baseline

Byte probes compare against the CI film render preserved
locally at `~/workspace/talerender-evidence/renders/post/
frames` (1,811 frames, CI run `43665874388` at `56348ac`).
`git diff 56348ac..d101770 -- src/` is empty (only the
Cargo.toml version literal moved), so that render is a
valid baseline for HEAD. Probe ranges are the standing
ones (byte_probes.py defaults): 195–204, 795–814,
1195–1204 (40 frames). Candidate probe renders are
dispatched per item via render.yml in those ranges only;
their artifacts are downloaded, probed, and deleted
(quota law — no full-film artifacts).

## §4 PC-blocked leg (rides PC-SESSION-W98; not this execution)

- RADV adapter proof + determinism RADV leg (contract E7).
- GPU benchmark rows (the four-way matrix's hardware legs)
  and PERF.md's PC tables.
- sysfs VRAM cross-check (item 1's numeric path — on this
  VM and CI, `/sys/class/drm` has no `cardN`, so only the
  `unavailable` path is testable here; W56 §8 V1/V2 stay
  open until the visit).
- The film-s03 pin debt can be settled in the same visit
  or a small follow-on; it is not part of Stage 1.

## §5 Corrections to the roadmap's stale claims (for the record)

Verified against the tree, 2026-11-15: (a) the native
backend **is** adopted into the shipped skill (opt-in,
2026-11-15 ruling) — the roadmap's "not yet adopted" is
pre-decision; (b) the paint cache is **not** entry-count
only — a byte budget with a 2 GiB ceiling has been the
binding constraint since Wave 2; the "~4.6 GB worst case"
is the pre-budget figure; (c) an LRU **buffer pool** and a
texture **atlas** module exist (the pool unwired, the atlas
unwired into the upload path — neither is the finished
feature the roadmap asks for, and neither is Stage 1);
(d) locomotion, blink adoption and lip-sync Variant A —
listed in the roadmap's WP6 future — landed 2026-11-15;
(e) v1.0.1 is released (the roadmap's snapshot predates
it). None of this shrinks Stage 1's real remainder: the
RADV proof, the VRAM ledger, the eviction policy, truthful
adapter reporting, device-loss handling, and benchmark
automation — items 1–4 plus the PC leg above.

## §6 Follow-ups and open limitations (to be completed as items land)

- (Item 3 lands here: the device-loss verdict — what the
  wgpu 0.20 surface supports, what was implemented, what
  remains impossible without an API change.)
