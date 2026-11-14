# RUNLOG-W09 — progress elapsed + run telemetry (PERF §9 remainder)

**Seat:** Wave 2 W09. **Delivered:** `src/runlog.rs` (new file,
uncommitted — the lead wires and lands it). **Not touched:**
every other `src/` file, both runbooks, W06's cache work.

## 1. What the module is

A tiny std-only timing/process-telemetry helper. Division of
labour, fixed by contract with W06: **W06 counts** paint-cache
events (hits / misses / evictions / bytes); **runlog owns time
and process memory** — a monotonic `RunClock`, the progress /
summary line formats, and the peak-RSS self-report. Neither
side formats the other's numbers; the completion line composes
both (§2c). Everything is log-only — no return value feeds a
pixel, a verdict, or a gate, so the determinism law is
untouched (elapsed times differ between runs; frames must not).

API (all `pub`, module is `crate::runlog` once declared):

| Item | Purpose |
|---|---|
| `RunClock::new()` / `elapsed()` / `elapsed_secs()` | monotonic run clock (one `Instant`) |
| `elapsed_part()` | `elapsed 03:12` |
| `eta_part(done, total)` | `eta 02:24`, or `eta --:--` before the first unit |
| `timing_suffix(done, total)` | `elapsed 03:12 · eta 02:24` — the progress-line segment |
| `elapsed_line(done, total)` | standalone line: `frame 1051/1811 (58.0%) elapsed 03:12 eta 02:24` |
| `summary_suffix()` | `elapsed 03:12 · peak RSS 512.3 MB` — the end-of-run segment |
| `peak_rss_mb()` | VmHWM from `/proc/self/status` in MiB; `None` off-Linux / on failure |
| `parse_vmhwm_mb(text)` | the parser, exposed for tests |
| `clock_str(secs)` / `percent(done, total)` / `eta_secs(elapsed, done, total)` | pure arithmetic (the tested layer) |
| `frame_timing_enabled()` | the `TALERENDER_FRAME_TIMING` switch for §9.1's optional per-frame line |

Format decisions, pinned by tests: clocks are `MM:SS` with
**total** minutes (grow past 59: `97:25`) so the shape never
changes mid-log; seconds round to nearest; the no-rate ETA and
bad inputs share one sentinel, `--:--`; percent is one decimal;
a zero total is defined as 0.0%.

**ETA basis (film):** work units = *frames visited*
(rendered + skipped) against the requested range length.
Skips are ~free, so a resume run's early ETA is optimistic and
converges as renders dominate; on a fresh run skipped = 0 and
the basis is exact. This is documented on `eta_part` itself.

## 2. Wiring points (lead — three small diffs + one declaration)

### (a) Declare the module — `src/main.rs`, mod block (lines 18–27)

```rust
mod runlog;
```

Land this in the same commit as (b)/(c): an unwired private
module of `pub` items draws dead-code warnings in a binary
crate (harmless, but noisy).

### (b) Progress line — `src/film.rs`, `run_film`

Construct the clock with the loop counters (currently line
3468), i.e. *after* data load and renderer creation, so
elapsed measures the render loop (load + GPU setup excluded —
say so if that ever matters):

```rust
let (mut done, mut skipped) = (0u32, 0u32);
let clock = crate::runlog::RunClock::new();
```

Progress print (currently lines 3491–3493) — append the
suffix; the existing head is byte-identical:

```rust
if done % 25 == 0 {
    println!(
        "  frame {n} (t = {t:.2}s) — {done} rendered, {skipped} skipped · {}",
        clock.timing_suffix((done + skipped) as u64, (to - from) as u64)
    );
}
```

Cadence unchanged (every 25 *rendered* frames). Resulting
shape, on PERF §2's own worked example (suffix values
illustrative):

```
  frame 1051 (t = 35.03s) — 25 rendered, 0 skipped · elapsed 00:07 · eta 00:18
```

`elapsed_line` exists for any future mode that wants the
bare form; the film driver should not switch to it — PERF.md
quotes the current head in §2 and in the §9 failure table
("The last progress line names the last completed 25-frame
boundary" stays true either way, but the extension keeps
every quoted prefix matching).

### (c) Completion line — `src/film.rs` (currently line 3495)

Append runlog's summary, then W06's cache segment **last**
(contract order: counts → elapsed → peak RSS → cache):

```rust
println!(
    "film render complete: {done} rendered, {skipped} skipped (resume), range {from}..{to} · {} · {}",
    clock.summary_suffix(),
    cache.stats_line() // ← W06's segment, their symbol — this is its slot
);
```

W06's exact symbol name is theirs to fix; the slot is the
final ` · {…}` segment of this one line, in both film and
telemetry mode (their §9.2 ask covers both). Call
`summary_suffix()` here and nowhere later: VmHWM is a
process-wide high-water mark read at call time.

### (d) Telemetry mode — a caution, not a wiring point

Do **not** append anything to the `FILM TELEMETRY: CHECK …`
verdict line (film.rs ~3676–3679): the skill adapter and
PERF §4/§7 grep that line, and the verdict-law record treats
the line as the verdict. If telemetry-mode timing is wanted,
print runlog segments on a *separate* line after the verdict,
or omit them there. (Peak RSS for telemetry runs is already
captured by §4's `/usr/bin/time -v` wrapping; the film-mode
completion line is where the self-report pays off.)

### (e) Optional per-frame timing (§9.1, second half)

`runlog::frame_timing_enabled()` is the switch
(`TALERENDER_FRAME_TIMING` ≠ `0`). If the lead wants the
per-frame millisecond line, the loop reads the switch once
before the frame loop and prints inside it; the env name and
the switch now live in exactly one place. Not wired by this
seat; the line format is the lead's call.

## 3. What becomes true in the docs once wired

For the docs editors — quoted exactly; **this seat did not
edit the runbooks.**

**PERF.md §9** (header: "These are requests, recorded
here — the workarounds in §2–§4 … are protocol scaffolding,
not solutions"):

- Item 1's request — *"progress lines carry elapsed seconds"*
  — becomes true for film mode at wiring (b). §9.1 can be
  marked discharged, first half; the optional per-frame line
  stays open until (e) is wired.
- Item 3's request — *"the process reports its own peak RSS
  on exit (stderr is fine)"* — becomes true at wiring (c),
  on **stdout** as part of the completion line (better than
  the request's stderr allowance: it lands in the same log
  the protocol already archives). §9.3 can be marked
  discharged for film mode.
- **PERF.md §2, line ~136** — the worked example
  ``(`  frame 1051 (t = 35.03s) — 25 rendered, 0 skipped`, …)``
  stays *interpretable* verbatim: real logs keep that exact
  head and gain ` · elapsed … · eta …`. Editors may extend
  the quoted example with a suffix, but nothing in §2's
  arithmetic depends on it.
- **PERF.md §4, line ~174** — *"Measured with
  `/usr/bin/time -v` (`Maximum resident set size`)."* After
  wiring, the completion line is a second source for the
  same number; editors should note the unit conversion
  (`/usr/bin/time` tables are KiB; runlog prints MiB with
  one decimal — KiB ÷ 1024) so §4's tables stay comparable,
  and keep `/usr/bin/time` as the cross-check, not the only
  route.

**USER-PC.md** — no line becomes false and none needs
changing for runlog: it quotes no progress-line format and
no RSS method. Its §2 adapter checks and the PERF failure
table's progress-line references remain accurate under the
extension. (Its one `--help`-era statement was already
corrected in Wave 1's CLI commit, independently of this
seat.)

## 4. §9 checklist disposition (W95 §3 numbering)

| # | Ask (W95 §3) | Disposition |
|---|---|---|
| 1 | Elapsed seconds on progress lines | **Discharged by W09** (module + wiring b/c). Optional per-frame ms line: switch provided (`frame_timing_enabled`), emission still open — lead wiring, §2e. Per-frame cost *distribution* / slowest frame remain unmeasurable until that lands; the progress-line ETA + elapsed are the run-level signal. |
| 2 | Paint-cache statistics | **W06's**, in flight this batch. Runlog's only part is the completion-line slot (§2c). |
| 3 | Peak-RSS self-report | **Discharged by W09** (module + wiring c), Linux. Off-Linux the summary prints `peak RSS unavailable` — an explicit marker, never a fabricated number. Caveat recorded on `peak_rss_mb`: VmHWM is process-wide and monotonic; in this binary each process is one run, so the semantics match the ask. |
| 4 | Film-telemetry exit codes | **Still open — not this seat's.** It changes `main.rs` control flow and the adapter's contract (the adapter currently greps the verdict line); whoever lands it must keep the verdict line byte-stable. Runlog neither needs nor affects it. |
| 5 | GPU memory figure (named blind spot) | **Still open, by PERF's own framing** ("recorded as a blind spot, not a request"). wgpu surfaces no VRAM figure here; the `renderer:` line is lead-owned. Runlog changes nothing. |

## 5. Verification (this seat, first-hand)

- Standalone harness (the wave's no-cargo route):
  `rustc --edition 2021 --test src/runlog.rs` →
  **12 passed; 0 failed**. Tests pin: `MM:SS` formatting
  incl. the >59-minutes shape, second-rounding incl. the
  minute carry, the `--:--` sentinel (negative/NaN/∞),
  percent incl. the 1051/1811 → 58.0 decomposition of the
  brief's example, ETA edges (no-rate → None, complete →
  0.0, half-done → equal remainder), the end-to-end
  `elapsed_line` format at a fresh clock, suffix composition,
  summary shape (never a fabricated `0.0 MB`), the VmHWM
  parser against kernel-format sample text + three malformed
  variants, the live `peak_rss_mb()` on this Linux host, and
  the env switch.
- `rustc --edition 2021 --crate-type lib src/runlog.rs` —
  clean build, no warnings.
- The module is inert until the lead declares it: nothing
  else references it, no existing file was modified, and
  `git status` shows only the two new files from this seat.
  Real CI verification rides on the lead's wiring commit,
  per the wave's CI-only build rule.
