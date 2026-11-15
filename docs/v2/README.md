# TaleRender V2 — documentation (`docs/v2/`)

**Seat:** TaleRender V2, Batch 2, Seat 10 (WP14 — documentation +
migration guide; master prompt §18 + §23, `docs/integration/V2-CONTRACTS.md`).

**Basis of every claim:** the working tree as surveyed by this seat at
HEAD `712a6aa` ("V2 B1 fixes"). Line counts, module registrations,
call-site wiring, and file existence were checked against the actual
files under `src/`, `tools/`, and `docs/integration/` — not against
delivery notes alone. Where a delivery note and the tree disagree, this
documentation follows the tree and says so.

**The one fact that shapes this whole set:** at this HEAD, all ten
Batch 1 V2 modules exist and are registered in `src/main.rs`
(`mod assetstream; mod config; mod dirtyrect; mod ffstream; mod
mempressure; mod observability; mod projectfile; mod scheduler; mod
streamq;` plus the pre-V2 modules), and they carry in-module unit
tests — but **none of the Batch 1 V2 modules has a production call
site yet**. A tree-wide search for `config::`, `mempressure::`,
`scheduler::`, `observability::`, `assetstream::`, `projectfile::`,
`dirtyrect::`, and `ffstream::` outside their own files returns no
production caller (`streamq::` is mentioned only inside `ffstream.rs`'s
delivery note). They are compiled policy/logic modules awaiting the
lead's wiring, exactly as their own delivery notes state. Nothing in
this doc set describes them as wired.

## Delivery note — what exists, what is contract-only

### Exists in the tree at `712a6aa` (verified by this seat)

- The v1.0.1 production pipeline: built-in scenes, scene-format files,
  the Onoda film driver (PNG sequence), film telemetry gate, composer
  chain, VRAM ledger + `gpu-mem:` line (`f9429a8`).
- The wgpu 0.20 → 30.0.2 / pollster 1.0.1 / image 0.25.10 upgrade
  (`c246ec8..65e4ea2`), `Cargo.toml` pinning exactly those three
  direct dependencies.
- Batch 1 V2 modules (registered, unit-tested, **not wired**):
  `src/dirtyrect.rs`, `src/mempressure.rs`, `src/streamq.rs`,
  `src/ffstream.rs`, `src/projectfile.rs`, `src/scheduler.rs`,
  `src/assetstream.rs`, `src/observability.rs`, `src/config.rs`,
  and `tools/perf/lab/bench_lab.py`.
- `docs/integration/V2-ERROR-REFERENCE.md` (Batch 1 Seat 9, WP12).

### Batch 2 — landing in this wave (re-checked at delivery)

At this seat's first survey every Batch 2 file was absent. On the
final re-check, immediately before delivery, sibling Batch 2 seats
had landed most of their files **as untracked seat files** — present
in the tree, but **not yet registered in `main.rs`** (the `mod` list
is unchanged) and not yet lead-integrated. Sections describing them
say "landing in this wave" with that integration caveat:

| Batch 2 item | Assigned path | Status at final check |
|---|---|---|
| WP1.1 LRU spec | `docs/integration/V2-WP1-LRU-SPEC.md` | EXISTS (682 lines, untracked) — spec only; `film.rs` eviction is still FIFO |
| WP1.4 lifetimes | `src/lifetimes.rs` | EXISTS (612 lines, untracked, not registered) |
| WP2 telemetry reporting (C2) | `src/memreport.rs` | EXISTS (712 lines, untracked, not registered) — **and it renders `crate::gpucore::MemSnapshot`, a type that does NOT exist in `gpucore.rs` yet** (lead-owned, C2), so it cannot compile until the lead lands that half |
| WP3 adapter policy + device loss (C7) | `src/adapter.rs` | **ABSENT — CONTRACT only** |
| WP4 streaming modes (C3) | `src/streammodes.rs` | EXISTS (764 lines, untracked, not registered) |
| WP5 TypeScript exporter (C4) | `tools/export/export_project.mjs` (+ `export_project.selftest.mjs`) | EXISTS (357 lines, untracked) |
| WP9 production integration | `tools/integration/validate_native_project.py`, `docs/integration/V2-WP9-INTEGRATION.md` | EXISTS (862 / 477 lines, untracked) |
| WP10 character-state efficiency | `src/charstate.rs` | **ABSENT — CONTRACT only** |
| WP11 output/encoding manager | `src/outputmgr.rs` | EXISTS (933 lines, untracked, not registered) |
| WP6 reporting engine | `tools/perf/lab/report.py` | EXISTS (670 lines, untracked) |

Only `src/adapter.rs` (WP3) and `src/charstate.rs` (WP10) remain
pure contract. Everything in the "exists" rows is a seat delivery
awaiting the lead's integration landing — none of it changes the
running binary yet.

## Index — per-file status

| File | Contents | Status (§23 vocabulary) |
|---|---|---|
| `README.md` (this file) | Index + delivery note | IMPLEMENTED (this delivery) |
| `ARCHITECTURE.md` | Module map of all 31 `src/*.rs`, data flow, V2 wiring state | IMPLEMENTED — describes the tree at `712a6aa` |
| `MEMORY.md` | Paint budget, buffer pool, atlas, VRAM ledger, mempressure, dirty regions, lifetimes — as implemented | IMPLEMENTED |
| `GPU.md` | Adapter selection/reporting, `gpu-mem:` grammar, telemetry, device loss (contract + current state) | IMPLEMENTED; device-loss section is CONTRACT-only (`adapter.rs` absent); memreport section updated at final check |
| `STREAMING.md` | Three output modes, queue/sink design, `.part` discipline, finishing contract | IMPLEMENTED; Stream/Auto modes are module-level, not wired |
| `CLI.md` | The v1.0.1 CLI exactly as `src/main.rs`'s `HELP` states it | IMPLEMENTED |
| `CONFIG.md` | `src/config.rs` precedence system + the env vars that actually take effect today | IMPLEMENTED; config module not yet consulted by `main.rs` |
| `PERF-LAB.md` | `bench_lab.py` + `report.py` usage, `talerender-bench/1` record schema, lavapipe labelling law, PENDING-PC | IMPLEMENTED (`report.py` landed during writing — see delivery note) |
| `MIGRATION.md` | wgpu 0.20 → 30 guide from commits `c246ec8..65e4ea2` + v1.0.1 → V2 integrator notes | IMPLEMENTED |

**Not verified by this seat:** build/test status of the current HEAD
(CI is the only build, per standing law; this seat ran no build and
no git commands beyond read-only log inspection by the parent survey —
treat any "tests pass" statement in these docs as a citation of
`V2-CONTRACTS.md` / `STAGE1-SCOPE.md`, not a fresh verification).
Hardware (RADV) behaviour is **not verified** anywhere in this set:
every rendering number in the repo's record is CI lavapipe.
