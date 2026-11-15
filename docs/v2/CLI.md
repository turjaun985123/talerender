# TaleRender — CLI reference (v1.0.1, as it stands)

Source of truth: `const HELP` in `src/main.rs` (the file's own
header requires the code and the help text to be kept in step).
Everything below is that text's content, reformatted — no flag
is documented here that `main.rs` does not parse. Version:
`talerender 1.0.1` (`Cargo.toml` version, printed by
`-V`/`--version` as `talerender 1.0.1`).

## Usage

```
talerender [OPTIONS]
talerender --scene-file <scene.json> [--out frame.png]
talerender --film <DIR> --out <FRAMES_DIR> [--from N] [--to N]
talerender --film-telemetry <DIR>
talerender --telemetry <MODE> [--scene-file <scene.json>]
```

## Modes (choose one; default is a built-in scene render)

| Flag | Meaning |
|---|---|
| `--scene <NAME>` | Built-in scene: `m1` \| `ab1` \| `dof-chart` \| `stroke-battery` (default `m1`). `ab1` = the §28 Onoda reference scene; `dof-chart` = the M3 Bokeh chart; `stroke-battery` writes the M4 battery cards into `--out` (a directory). |
| `--scene-file <PATH>` | Render a scene-format JSON file (v1/v2; `docs/scene-format.md`). Wins over `--scene` when both are given. |
| `--film <DIR>` | Render a whole film: DIR holds the film data (`film.json` — the generic entry; `onoda.json` accepted for this film) + an `assets/` dir; segment shots are staged by the driver in code (`--film` never reads `scenes/film/`). Frames are written to `--out` as `frame_%05d.png`; existing frames are skipped (resume). |
| `--film-telemetry <DIR>` | Native telemetry gate over a film directory (CPU only, no GPU). The verdict is the printed line `FILM TELEMETRY: CHECK OK` / `CHECK FAILED (...)` — read the line. |
| `--tables` | With `--film-telemetry`: also print the shot, caption and cast tables after the verdict line. Default output is unchanged. |
| `--telemetry <MODE>` | M6 QA telemetry (CPU only): `ab1` \| `ab1-file` \| `ab1-variant`. Pair `ab1-file` with `--scene-file`. |

Dispatch order in `main()` (after `-h`/`-V`, which win anywhere
on the command line): `--telemetry` → `--film-telemetry` →
`--film` → `--scene-file` → `--scene`.

## Options

| Flag | Meaning |
|---|---|
| `--width <PX>` | Frame width (default 1080). |
| `--height <PX>` | Frame height (default 1920). |
| `--out <PATH>` | Output PNG file — or, for `--film` and `stroke-battery`, the output directory (default `frame.png`). |
| `--caption <PATH>` | Caption strip PNG composited by `--scene ab1` (default: the §28 strip under `docs/ab/` — a repo-relative path, so run from the repo root or pass `--caption` explicitly). |
| `--post-stage <STAGE>` | Composer ablation for `--scene ab1`: `lighting` \| `bloom` \| `grade` (default `grade` = the full chain). |
| `--captions <PATH>` | Caption metrics JSON (for `--telemetry`). |
| `--cutouts-out <PATH>` | Write cutouts() JSON (with `--telemetry`). |
| `--from <N>` | First film frame to render, inclusive (default 0). |
| `--to <N>` | Film frame end, exclusive; 0 = through the end (default 0). |
| `--res <SCALE>` | Render scale for drafts: `0 < SCALE <= 1`, applied to `--width`/`--height` (default 1 = full size). The whole frame scales, captions included. Draft only. |
| `--aa <0\|2\|4\|8>` | MSAA samples for the HDR scene pass (default 4 = full quality; 0 = single-sample). Draft only below 4. |
| `--shadow <SIZE>` | Key shadow map size: 512 \| 1024 \| 2048 \| 4096 (default 2048 = full quality). |
| `--draft` | Draft preset: `--res 0.5 --aa 0 --shadow 512`. Any of the three flags given after `--draft` overrides it. Draft runs label every output line `DRAFT` — never use for delivery. (The in-band tag is `[DRAFT res=… aa=… shadow=… — NOT FOR DELIVERY]`, from `Knobs::tag`.) |
| `-h`, `--help` | Print the help text and exit. |
| `-V`, `--version` | Print the version and exit. |

Bad invocations print `error: …` plus a help hint and exit 2 —
no panics on bad input (`cli_error` in `main.rs`). Structured
runtime failures print `error: {e}` and exit with the error's
own code (`exit_on_err` → `TaleError::exit_code()`).

## Environment (the variables HELP itself documents)

| Variable | Meaning |
|---|---|
| `TALERENDER_PAINT_CACHE <N>` | Cap the paint cache at N entries (default 160). Lower it on low-RAM machines; telemetry runs often use 24. |
| `TALERENDER_PAINT_BUDGET_MB <N>` | Cap the paint cache at N MiB of painted canvases (default: derived — the entry cap's bytes, at most 2048 MiB). 0 disables the byte budget (entry cap alone). Eviction under either cap costs time, never pixels. |

Other `TALERENDER_*` variables that exist in code but are *not*
in HELP are listed in `CONFIG.md` §3 — do not treat HELP as an
exhaustive env list.

## Exit codes (as HELP states them)

| Code | Meaning |
|---|---|
| 0 | Success. |
| 1 | `--film-telemetry` QA verdict: CHECK FAILED (the printed line carries the counts and stays authoritative). |
| 2 | Command-line error (unknown flag, bad or missing value); also film or scene data that fails to load. |
| 3 | `--film-telemetry` could not run (film data unreadable; the reason is printed on stderr, no verdict line); also GPU initialisation or render failure in any mode. |
| 4 | Image I/O failure (reading or writing a PNG). |

These agree with `errors.rs`: `DataLoad` → 2;
`GpuInit`/`DeviceRequest`/`BufferMap`/`Render` → 3;
`ImageIo` → 4. Full error reference:
`docs/integration/V2-ERROR-REFERENCE.md`.

## What is NOT in the CLI yet (V2 honesty note)

The V2 contracts propose new flags — `--mem-profile`,
`--adapter-policy`, `--output-mode`, `--perf-record-dir`,
`--config` (contract C8 / `config.rs`'s delivery note). **None
of them is parsed by `main.rs` at this HEAD, none appears in
HELP, and passing one today is an unknown-flag error (exit 2).**
`src/config.rs` exists to resolve their values once the lead
wires the parse arms; until then the v1.0.1 CLI above is the
complete CLI. Likewise there is no streaming flag, no scheduler
flag, and no project-file flag — `projectfile.rs`,
`scheduler.rs`, `streamq.rs` and `ffstream.rs` have no CLI
surface yet.
