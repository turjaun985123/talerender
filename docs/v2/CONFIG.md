# TaleRender V2 — Configuration reference

Two different things must not be conflated:

1. **`src/config.rs`** (V2, WP13, contract C8) — a resolution
   module that exists, is registered and unit-tested, but is
   **not consulted by `main.rs` yet**. No flag, env var, or file
   described in §1–§2 changes the binary's behaviour today.
2. **The env vars that actually take effect today** (§3) —
   read directly by the modules that use them.

## 1. The C8 precedence system — `src/config.rs`

One `Config` struct, resolved per field with precedence:

```
CLI arg  >  env (TALERENDER_*)  >  config file (talerender.toml)  >  default
```

- The config file is hand-parsed `key = value` text (no new
  dependencies); the default path is `./talerender.toml`
  (`DEFAULT_CONFIG_FILE`), overridable by the (future) `--config`
  path. A missing/unreadable file is not an error — resolution
  proceeds with the remaining layers.
- Resolution API: `resolve(cli, env, file_text)` takes the
  environment as an injected closure so tests never touch the
  process env; `resolve_with_report` additionally returns the
  warnings for ignored invalid values;
  `resolve_from_process` / `resolve_from_process_with_report`
  wire `std::env` + the file on disk.
- The module stores **strings**, not consumer enums — config is
  the interchange layer and creates no cross-module dependency.
  It never parses argv itself and never exits the process; the
  v1.0.1 CLI contract (`-h`/`-V`, panic-free errors, `[DRAFT]`
  knob tagging) is untouched by its presence.

## 2. Fields, layers, validation

| Field (`Config`) | CLI override (`CliOverrides`) | Env var | File key | Default | Valid values |
|---|---|---|---|---|---|
| `mem_profile: String` | `mem_profile` | `TALERENDER_MEM_PROFILE` | `mem_profile` | `balanced` | `low-memory` \| `balanced` \| `high-performance` |
| `adapter_policy: String` | `adapter_policy` | `TALERENDER_ADAPTER_POLICY` | `adapter_policy` | `auto` | `auto` \| `hardware-only` \| `software-ok` \| `named:<name>` (name non-empty) |
| `output_mode: String` | `output_mode` | `TALERENDER_OUTPUT_MODE` | `output_mode` | `png` | `png` \| `stream` \| `auto` |
| `perf_record_dir: Option<PathBuf>` | `perf_record_dir` | `TALERENDER_PERF_RECORD_DIR` | `perf_record_dir` | `None` | any non-empty path |

- Values are case-sensitive and whitespace-trimmed per layer.
- **Fall-through semantics (deliberate divergence):** an invalid
  value at *any* layer is ignored **with a warning**, and
  resolution falls through to the next layer — it does *not*
  `cli_error`/exit 2 the way `--res`/`--aa`/`--shadow` do. Only
  a *missing* value for a future CLI flag would error, via the
  existing `value()` helper. This is why the lead wiring must
  pass raw strings into `resolve()`.
- Intended consumers (none wired yet): `mem_profile` →
  `mempressure` (C1; same spelling set), `adapter_policy` →
  the C7 adapter module (CONTRACT, absent), `output_mode` →
  the C3 stream-modes module (CONTRACT, absent),
  `perf_record_dir` → the WP6 perf lab.
- **Divergence from the contract text:** V2-CONTRACTS §3 C8
  also names a "stream_fps override (none by default)" field.
  The implemented `Config` has **no such field** — the seat's
  charter field set (four fields above) is what was built.
  (A `TALERENDER_STREAM_FPS` string does appear in the tree's
  env-name census, inside `config.rs`'s tests/comments surface;
  no resolved field carries it.) Treat C8's stream-fps line as
  unimplemented.

Example `talerender.toml` (shape only — inert until wiring):

```toml
mem_profile = low-memory
adapter_policy = auto
output_mode = png
# perf_record_dir = bench-out/lab
```

## 3. Environment variables that take effect TODAY

Read directly by their owning modules, independent of
`config.rs`:

| Variable | Owner | Effect |
|---|---|---|
| `TALERENDER_PAINT_CACHE` | `film.rs::PaintCache::new` | Paint-cache entry cap (default 160). Also the `--paint-cache` mechanism `bench_lab.py` sets per run. |
| `TALERENDER_PAINT_BUDGET_MB` | `paintbudget::PaintBudget::from_env` | Paint-cache byte budget in MiB; unset/unparseable → derived default (≤ 2048 MiB); `0` disables byte enforcement. |
| `TALERENDER_BUFFER_POOL_MB` | `bufferpool::BufferPool::from_env` | Readback pool retained budget (default 64 MiB). Inert in practice: the pool has no production caller (`MEMORY.md` §2). |
| `TALERENDER_FRAME_TIMING` | `runlog::frame_timing_enabled` | Enables per-frame timing emission when set to anything but `0`. At this HEAD the switch exists but its emission into the film loop is still lead wiring (see `PERF-LAB.md`). |
| `TALERENDER_DOF_APERTURE`, `TALERENDER_DOF_DEPTH_FORMAT`, `TALERENDER_DOF_DUMP_DEPTH` | `dof.rs` | DoF chart diagnostics knobs (gate tooling, not production rendering). |
| `TALERENDER_TELEMETRY_DEBUG` | `telemetry.rs` | Extra telemetry debug output for the M6 gate tooling. |
| `TALERENDER_MEM_PROFILE` | `mempressure::MemPressure::from_env_and_arg` | Recognised by the C1 module itself (arg > env > machine default) — but since no render path constructs a `MemPressure` yet, setting it changes nothing in the binary today. It is *also* `config.rs`'s env layer for the same field; the two agree on spelling and values by design. |

## 4. Precedence in the binary as it actually stands

Until the lead wires C8, the effective configuration story is:

- Frame/quality knobs come only from argv (`--res`, `--aa`,
  `--shadow`, `--draft`; defaults = full quality, and any
  deviation tags the run `[DRAFT …]`).
- Memory behaviour comes only from the §3 env vars above.
- There is no config-file support in the running binary:
  a `talerender.toml` in the working directory is ignored
  (not an error — simply never read).
