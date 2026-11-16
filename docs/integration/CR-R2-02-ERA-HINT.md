# R2-02 — era-hint exit-101 extension — NOTE

Seat: R2-02, Chrome-Replacement REMEDIATION wave, Batch 2.
Charter: extend `era_hint.mjs` classification for the REMEDIATION-CONTRACTS
§1.4 failure taxonomy + SKILL.md §0 symptom-list draft (§7, Package 4).
All deliverables are patch candidates in this directory only; nothing in
the shipped skill tree or the talerender repo was edited, nothing committed.

## Files delivered

| File | What |
|---|---|
| `era_hint.mjs` | Deliverable A — full replacement candidate for `template/tools/native_modes/era_hint.mjs`, with the Deliverable B test additions integrated into its `runSelfTest()` |
| `era_hint.mjs.patch` | Deliverable A (diff form) — unified diff, `a/`/`b/` labels rooted at the skill tree (`template/tools/native_modes/era_hint.mjs`); verified to apply with `patch -p1` (see Evidence) |
| `era_hint.self-test-additions.mjs` | Deliverable B — the 11 new test cases as a standalone runnable file in the existing self-test's format (imports `./era_hint.mjs`, the patched candidate beside it) |
| `SKILL-section0.patch` | Deliverable C — unified diff against the skill's `SKILL.md`, rooted at the skill tree; adds the §0 symptom list including the panic class |
| `NOTE.md` | This file |

## Guard verification (performed live, not from records)

Command (local binary, CPU/CLI only):

```
~/workspace/talerender/target/release/talerender --film-telemetry \
  ~/faceless-shorts/the-lighthouse-cats-last-shift/native/film
```

Result: **exit 3**, stdout empty, stderr verbatim:

```
film telemetry failed: film data is not staged by the legacy driver (missing anchors: bookseller9, coconuts6, commander8, decades6, flew9, found7, loyal11, message3, never2, only8, order10, refused7, surrender2, surrendered10, thought4, trick4, years0); bridge it with driver: scene-sequence (see REMEDIATION-CONTRACTS §1)
```

**17 anchors named** — matches the charter's expectation and the
`REQUIRED: [&str; 17]` set in talerender `src/film.rs`
(`legacy_anchor_guard`, §1.4). Source chain confirmed in code:
`run_film_telemetry` → `legacy_anchor_guard(&data)?` → `main.rs` prints
`film telemetry failed: {e}` and exits 3. The project's manifest
(`native/film.json`, "The Lighthouse Cat's Last Shift") carries **no
`driver` field**, so §1.3 routes it to the legacy driver — the guard
firing is the designed behaviour for that bridge state, which is exactly
why its remedy is re-bridging (R2-03's job), not a binary change.

## Taxonomy — before (shipped `era_hint.mjs`, read end-to-end)

`classifyNoVerdict()` returns `{ stale, dataRefused, how }`; a verdict
line present returns `null`. `eraHintMessage()` branches in this order:

| # | Class (before) | Positive evidence | Remedy text |
|---|---|---|---|
| 1 | data-refused (`dataRefused && !stale`) | stderr/stdout contains `film telemetry failed:` | re-bridge: `scripts/retrofit.py` or `tools/bridge/build_bridge.py`; "Provisioning a different binary does not fix refused data" |
| 2 | stale (`stale`) | exit 2, or `unknown argument: --film-telemetry` on either stream | provision current binary from CI, `export TALERENDER_BIN=…`, repo-checkout-slot warning |
| 3 | generic (everything else) | none — fallback | "Exit 3 means the film data could not be read…; a crash or signal means the binary itself failed"; stale offered as "one other named cause" with a currency-check one-liner, **plus the full provisioning remedy** |

The self-test embedded in the shipped file covers exactly these:
stale (6 checks), generic exit-3 (3), data-refused (3), signal (1),
verdict-present-null (1), `hasVerdictLine` (1) = **15 checks**.

## Taxonomy — after (patch candidate)

`classifyNoVerdict()` additionally returns `{ legacyGuard, panicked }`,
both positive-evidence only. Branch order in `eraHintMessage()`:

| # | Class (after) | Positive evidence | Remedy text |
|---|---|---|---|
| 1 | **legacy-guard refusal** (NEW) | combined output contains BOTH `film data is not staged by the legacy driver` AND `driver: scene-sequence` (the guard's verbatim two halves), and no stale evidence | re-bridge so the manifest selects the generic driver: rebuild with `tools/bridge/build_bridge.py` (emits `"driver": "scene-sequence"` per §1.5), verify `film.json` carries the field; explicitly NOT provisioning |
| 2 | **current-binary panic — engine defect, not an era problem** (NEW, named class exported as `PANIC_CLASS`) | exit **101** (Rust panic exit code, exported as `PANIC_EXIT`), and no stale evidence | **report the defect**: preserve the full output incl. the `thread 'main' panicked at <file>:<line>` line, record command + film dir + binary path, file against `turjaun985123/talerender`; explicitly "Do NOT re-provision… do NOT re-bridge… do NOT fall back…". The message never calls `provisioningRemedy()` — no `gh run download`, no `export TALERENDER_BIN=` |
| 3 | data-refused | unchanged | unchanged |
| 4 | stale | unchanged | unchanged |
| 5 | generic | unchanged (101 and guard cases no longer reach it) | unchanged |

Design decisions worth the lead's eye:

- **Precedence:** the guard is tested before generic data-refusal
  because the guard line *contains* `DATA_REFUSED_MARKER` (`main.rs`
  prefixes it) — without the reorder it can never be distinguished.
- **Current-era qualifier:** both new branches require `!stale`,
  mirroring the existing `dataRefused && !stale` pattern. A hypothetical
  run with stale evidence (exit 2 / unknown-argument text) classifies
  stale even at exit 101 — the panic class is, per the charter, for a
  *current-era* binary. This is pinned by a self-test case.
- **Both guard markers required:** a partial quote of the guard phrase
  falls back to generic data-refusal (still a re-bridge remedy), never
  to a false legacy-guard claim. Pinned by a self-test case.
- Fail-closed semantics untouched: the module still only changes the
  TEXT of the adapter's `fail(...)`, never the verdict (per the shipped
  file's own wiring note).

## Test evidence (all run locally with Node, 2026-11-16)

| Run | Result |
|---|---|
| Shipped self-test: `node …/template/tools/native_modes/era_hint.mjs --self-test` | **PASS, 15/15 checks, exit 0** (baseline, unmodified tree) |
| Patched candidate: `node era_hint.mjs --self-test` (this dir) | **PASS, 26/26 checks (15 original + 11 new), exit 0** |
| Standalone additions vs patched candidate: `node era_hint.self-test-additions.mjs` | **PASS, 11/11, exit 0** |
| Standalone additions vs SHIPPED module (before-state control, run in `/tmp`) | **FAIL, 10 of 11 failing, exit 1** — expected: the shipped classifier has no `legacyGuard`/`panicked` fields and neither new message. The 1 passing case is "panic + stale evidence stays stale", which the shipped code also satisfies (it falls into the stale branch) |
| Patched candidate fed the REAL captured guard stderr (exit 3, verbatim from the live probe above) | classifies `{ stale:false, dataRefused:true, legacyGuard:true, panicked:false, how:"exit 3" }` and prints the legacy-guard message with the scene-sequence remedy |
| Patch application check (scratch copy in `/tmp`, `patch -p1` dry-run + apply for both `.patch` files, then self-test on the applied tree) | both patches apply cleanly; applied `era_hint.mjs --self-test` **PASS** |

No results fabricated: every line above is from a command run in this
session; the guard fixture in the tests is the verbatim live stderr.

## Discrepancies — where current behaviour contradicts the contracts

1. **Guard case misclassified today (contracts §1.4/§1.5, §7).** The
   shipped classifier puts the §1.4 guard output in the generic
   data-refused bucket (it matches `film telemetry failed:`). Its
   message explains the refusal with the wrong classic case ("missing
   F2 fields, flat plates.json") and its remedy never names
   `"driver": "scene-sequence"` — the one remedy the guard itself
   prescribes and §1.5 builds into `build_bridge.py`. An operator
   following the shipped hint re-bridges without the driver field and
   hits the same guard again. Verified by running the shipped module
   on the real guard output in this session.
2. **Exit 101 gets the era remedy today (contracts §7).** Probed
   directly: shipped `eraHintMessage({status:101, stderr:"thread 'main'
   panicked at src/film.rs:3726: no entry found for key"})` falls into
   the generic bucket, whose text offers era mismatch as a named cause
   and prints the full CI provisioning remedy (`gh run download`,
   `export TALERENDER_BIN=`). For a current binary that is exactly the
   remedy the charter forbids suggesting: post-§1.4, unstageable data
   must produce the exit-3 guard, so a remaining 101 is an engine
   defect. The patch names that class and replaces the remedy with
   report-the-defect.
3. **SKILL.md §0 names only the stale symptom.** §0's era-pairing
   prose describes classification as "exit 2, or the binary's own
   `unknown argument` text" — it does not mention the data-refusal
   class that already exists in code (CR-B3-04), the legacy-guard
   case, or the panic case §7 requires adding. There is also no
   symptom *table* in §0 today (the charter's "symptom table" is
   prose); Deliverable C therefore adds a symptom-list table rather
   than editing an existing one.
4. **Scope limit the lead should note (not fixed by this seat):**
   `era_hint.mjs` is wired only into `capture_native.mjs`'s
   `runTelemetry()` no-verdict branch. A panic on the `--film` render
   path (e.g. during `frames`/`video` after the gate, or a direct
   binary run) never reaches `era_hint` — the adapter reports it only
   as `talerender exited 101 rendering frames …`. Classifying that
   surface needs adapter wiring beyond this charter. Likewise
   `scripts/helpers/era_check.sh` mirrors the OLD taxonomy (no-verdict
   + non-stale → one generic FAIL line); it is a setup helper, out of
   this seat's file scope, flagged here for whoever owns it.
