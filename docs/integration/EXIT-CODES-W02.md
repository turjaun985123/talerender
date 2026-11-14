# EXIT-CODES-W02 — film-telemetry exit codes + adapter dual convention

**Seat:** TaleRender Wave 2, W02 (telemetry exit codes + adapter compatibility).
**Date:** 2026-11-14. **Status:** adapter side DONE + proven in the skill
work copy (`~/workspace/talerender-skill-wiring/`); binary side SPECCED
below for the lead/integrator — this seat did not touch `src/`.

## 1. Problem

`talerender --film-telemetry <dir>` prints its verdict as a stdout line
— `FILM TELEMETRY: CHECK OK` / `FILM TELEMETRY: CHECK FAILED (…)` — but
the process **always exits 0**: pass, fail, and even the error path
(unreadable film data prints `film telemetry failed: …` on stderr and
still exits 0). The external audit (Phase 1) demands proper exit codes.
The skill adapter already gates correctly by parsing the line, so the
fix must be **additive**: the line stays printed verbatim and stays
authoritative; exit codes learn to agree with it.

## 2. The contract (binary, `--film-telemetry` only)

| Exit | Meaning |
|---|---|
| 0 | `FILM TELEMETRY: CHECK OK` |
| 1 | `FILM TELEMETRY: CHECK FAILED (…)` — a QA verdict, not a crash |
| 2 | CLI error — unchanged (`cli_error`: unknown flag, bad/missing value; happens before telemetry runs) |
| 3 | Telemetry **could not run** (unreadable/invalid film dir) — NEW |

**Line discipline (unchanged, and load-bearing):**

- The verdict line texts are byte-identical to today and remain the
  authoritative record of the verdict, counts included.
- On the could-not-run path there is **no verdict line on stdout**,
  ever. This already holds structurally: `run_film_telemetry`
  accumulates its report in a `Vec<String>` and prints only at the end,
  so an early `Err` prints nothing to stdout. The
  `film telemetry failed: {e}` message stays on **stderr**, verbatim
  format, now followed by exit 3 instead of a silent 0.
- Scope: this contract covers `--film-telemetry` only. The M6
  `--telemetry` modes keep their existing `telemetry::run` exit
  behaviour; all render modes are untouched.

**Caller caveat (intended consequence):** shell/CI callers running the
telemetry call under `set -e` will now stop on a FAILED verdict (exit 1)
where they previously continued. That is the audit's point. Callers that
read the verdict line (the adapter; the wave's telemetry evidence runs)
are unaffected — the line is unchanged.

## 3. The exact binary change (for the lead)

### 3a. `src/film.rs` — return the verdict

`run_film_telemetry` (line 3569) currently ends (line 3688) with
`Ok(())` after printing the report; the pass/fail fact is already
computed at line 3674 as `bad` (= overflow + violations + caption
failures). Two edits, nothing else in the function changes:

```rust
// signature, line 3569
pub(crate) fn run_film_telemetry(dir: &Path) -> Result<bool, String>
// Ok(true) = CHECK OK, Ok(false) = CHECK FAILED, Err = could not run.

// final return, line 3688
Ok(bad == 0)
```

There is exactly one `Ok` return in the function (verified); every early
return is already `Err(String)`, which is precisely the could-not-run
set. The printed report and verdict line are untouched.

### 3b. `src/main.rs` — map the result onto exit codes

Replace the `--film-telemetry` branch (currently: `Ok(()) => {}`,
`Err(e) => eprintln!(...)`, then `return` → exit 0) with:

```rust
if let Some(dir) = &film_telemetry_dir {
    // Native telemetry gate over the whole film (M6 semantics,
    // 0.25 s sampling) — no GPU needed. The verdict line printed by
    // run_film_telemetry stays authoritative; the exit code agrees
    // with it (EXIT-CODES-W02): 0 OK, 1 FAILED, 3 could not run.
    match film::run_film_telemetry(std::path::Path::new(dir)) {
        Ok(true) => {}
        Ok(false) => std::process::exit(1),
        Err(e) => {
            eprintln!("film telemetry failed: {e}");
            std::process::exit(3);
        }
    }
    return;
}
```

### 3c. `src/main.rs` HELP text — one paragraph goes stale

The `--help` EXIT CODES section currently says
*(--film-telemetry always exits 0: its verdict is the printed line, not
the exit code.)* — replace with the new truth in the same landing,
e.g.:

```
EXIT CODES:
    0   Success.
    1   --film-telemetry QA verdict: CHECK FAILED (the printed
        line carries the counts and stays authoritative).
    2   Command-line error (unknown flag, bad or missing value).
    3   --film-telemetry could not run (film data unreadable;
        the reason is printed on stderr, no verdict line).
```

### 3d. Follow-up outside this seat's write scope (flagged, not done)

`docs/runbook/PERF.md` (§1) still states the old law ("the binary exits
0 on pass, fail, and error alike") in its telemetry guidance. Once 3a/3b
land, that sentence and the W85-era "the line is the verdict, binary
exits 0 either way" phrasing in the integration docs need a dated
correction: **the line is still the verdict; the exit code now agrees
with it.** The adapter side (below) already speaks the dual truth.

## 4. Adapter side (DONE — work copy)

`template/tools/capture_native.mjs`, `runTelemetry()` now cross-checks
the verdict line against the exit code and accepts **both binary
generations** — required because era pairing means a project may be
captured with the v1.0.0 binary (old convention) or a post-W02 binary
(new convention), and the adapter must gate correctly on either:

| Verdict line | Exit | Adapter behaviour |
|---|---|---|
| CHECK FAILED | 0 (old) or 1 (new) | gate FAILS (return false → capture stops, adapter exit 1) |
| CHECK OK | 0 (both generations) | gate passes |
| CHECK OK | ≠ 0 | **fail closed** — adapter error: line and exit code disagree, never trusted |
| CHECK FAILED | ∉ {0, 1} (2, 3, signal) | **fail closed** — adapter error: a verdict run ends 0 or 1; anything else is a broken/mismatched binary |
| *(no verdict line)* | any (incl. 3 could-not-run, 2 stale binary, signal) | **fail closed** — adapter error naming the exit; exit 3 is called out as "film data could not be read", exit 2 as "binary predates --film-telemetry" |

Unchanged: the binary's full stdout/stderr is echoed before deciding;
FAILED is still tested before OK (W54/F1); `check` still prints its own
`CHECK OK`/`CHECK FAILED` terminal line; `frames`/`video` still stop at
the gate with no flag to render past it; the Chrome-only refusals
(`cues`/`castsheet`/`preview`) are untouched; the adapter's own exit
codes (0/1/2) are untouched.

Docs updated to the dual-convention truth:
`template/tools/CAPTURE-NATIVE.md` (the "Verdict parsing" blockquote) and
`docs/NATIVE-BACKEND.md` (failure-behaviour bullet, dated note).

## 5. Stub proof (DONE — recorded 2026-11-14)

Suite: `tools/test_adapter_exit_codes.sh` in the work copy
(self-contained; builds a minimal bridged-project fixture + stub
binaries in a temp dir, runs the **real** adapter via `check`/`frames`,
asserts exit codes, terminal lines, and a byte-identical fixture tree).
Re-run: `bash ~/workspace/talerender-skill-wiring/tools/test_adapter_exit_codes.sh`

**Result: 10 passed, 0 failed.**

| Case | Stub behaviour | Adapter result |
|---|---|---|
| old-ok | OK line, exit 0 | rc 0, `CHECK OK` ✓ |
| old-fail | FAILED line, exit 0 | rc 1, `CHECK FAILED` ✓ |
| new-ok | OK line, exit 0 | rc 0, `CHECK OK` ✓ |
| new-fail | FAILED line, exit 1 | rc 1, `CHECK FAILED` ✓ |
| disagree | OK line, exit 1 | rc 1, **no** `CHECK OK` line, fail-closed disagreement message ✓ |
| error3 (bonus) | no line, stderr reason, exit 3 | rc 1, failure message names exit 3 ✓ |
| frames + old-fail | FAILED line, exit 0 | rc 1, gate-stop message, **zero artefacts** (no `renders/`, no `review/`) ✓ |
| frames + disagree | OK line, exit 1 | rc 1, **zero artefacts** ✓ |
| cues refusal | any stub | rc 1, Chrome-only refusal intact ✓ |
| fixture tree | after all runs | identical to before (no stray files) ✓ |

The five required stubs gate correctly; the disagreement stub — the
case the dual convention exists for — fails closed with zero artefacts.

## 6. What this seat did NOT do

- No change to the repo's `src/` (binary change is §3, the lead's).
- No change to the shipped skill at `~/workspace/custom-skills/`.
- No commits. Files written: this spec; work-copy
  `template/tools/capture_native.mjs`,
  `template/tools/CAPTURE-NATIVE.md`, `docs/NATIVE-BACKEND.md`,
  `tools/test_adapter_exit_codes.sh`.
