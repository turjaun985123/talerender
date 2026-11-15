# CR-B3-10 — FLIP PACKAGE + CURRENT-ERA BINARY PROVISIONING

Seat B3-10 (Chrome-replacement wave, Batch 3). **Nothing landed, nothing
committed, nothing overwritten.** Two prepared inputs for the lead's
Batch 3 default-flip commit (separate, LAST):

- **Part A** — a locally built current-era `talerender` binary from the
  tasked HEAD, staged with checksums + era-check logs.
- **Part B** — the staged flip package under
  `~/workspace/chrome-replacement/batch3/B3-10/flip/`.

All seat files: `~/workspace/chrome-replacement/batch3/B3-10/`
(`talerender-63cc816`, `flip/`, `logs/`, `scratch-project/`,
`scratch-broken/`).

## 1. Part A — the binary

| | |
|---|---|
| Built from | talerender HEAD `63cc816907457be6e2907b618e15003455e69367` (the tasked head: eval15, streamsession, s12 retirement, puzzled/deadpan `caf18c0`, validator fix `0774a31`) |
| Command | `cargo build --release` (rustc 1.99.1, `CARGO_BUILD_JOBS=2`, `CARGO_INCREMENTAL=0`), detached via `nohup`, log `logs/build.log` |
| Result | **`Finished release profile [optimized] target(s) in 39m 41s`** — 135 crates, warnings only (251, pre-existing), no errors |
| Staged at | `~/workspace/chrome-replacement/batch3/B3-10/talerender-63cc816` (23,427,936 B) |
| sha256 | **`07fcf89abacca13160be03aea1734876fa0fa3444a1ca1a31bd8a6215f56c19d`** (`logs/checksums.txt`) |
| `--version` | prints `talerender 1.0.1` — the crate version string was never bumped, so **the version string does not distinguish eras**; provenance is the HEAD sha + checksum above |

First attempt failed in seconds: `cargo build --release --locked`
refused because the repo ships no `Cargo.lock` ("cannot create the lock
file … because --locked was passed"). Restarted without `--locked`;
the build generated an untracked `Cargo.lock` in the repo tree (build
artifact, left in place, disclosed here). The local build does not
replace CI as the code gate — CI for these heads is already green; this
build exists to provision an era-current binary locally.

**NOT touched:** `~/talerender-bin/talerender` — still the v1.0.1-era
provisioned binary, sha256
`b331f6704ebc8836057aa0e1b0f97421cb638c41ce9757abf0da230846903128`
(the v1.0.1 release asset), verified unchanged after all runs. The
lead swaps it at the flip (FLIP-CHECKLIST step 1). The repo-checkout
discovery slot `~/workspace/talerender/talerender` was likewise not
overwritten (the build output lives in `target/release/`).

**Race note (read before swapping).** The build ran 18:11–18:52 UTC.
During it, sibling Batch 3 seats landed four commits on this repo —
`de84b32` (B3-09, no src), **`fbfa0ca` (B3-06: 15 look features;
src/film.rs, src/scenefile.rs, src/toonfull.rs; tree files updated
18:47:09)**, `80a7dad` (B3-02, no src), `148dcab` (B3-02, CI yaml) —
moving HEAD to `148dcab`. The talerender crate's rustc started
≈18:44:30 (observed etime), i.e. its parse phase read the `63cc816`
sources before the 18:47:09 file updates. **Proof:** a fingerprint
rebuild probe after the fact recompiled the crate from the current
tree and produced a *different* sha256
(`e800e09f0721beaa82c2c2c9557f6380a6a1b7de4bc156ee360f8f0ea720a146`,
log tail in `logs/build.log`). So the staged binary is exactly the
tasked `63cc816` build — and it therefore does **not** contain
CR-B3-06's look features. If the flip should carry the newest tree,
rebuild at flip time (deps are cached; the probe recompile took
1m 16s) or take the CI artifact — the era checks below were run
against the staged `63cc816` binary and would need a re-run against
any replacement.

## 2. Era checks (against the staged binary; logs in `logs/`)

1. **Film telemetry — PASS.** `./talerender-63cc816 --film-telemetry
   ~/workspace/talerender/film` → final line
   **`FILM TELEMETRY: CHECK OK`** (`logs/telemetry.log`; ~13 min
   wall, partly contended). The nohup wrapper did not capture the
   process exit code; per the adapter contract (EXIT-CODES-W02) the
   verdict *line* is authoritative, and it is in the log.
2. **Adapter check-mode smoke — PASS.** Scratch project
   (`scratch-project/`: the `_selftest-native` bridged project —
   "The Soldier Who Didn't Know The War Was Over", 60.36 s, 1,811
   frames, 14 segments — with `tools/` refreshed from the shipped
   skill template), run as
   `TALERENDER_BIN=<staged binary> node tools/capture_native.mjs check`
   → **exit 0, `CHECK OK`** (`logs/smoke-new-binary-check.log`;
   ~13.6 min). Header confirms the binary was taken via
   `TALERENDER_BIN`; the full parity report (shots, captions,
   composition: 31 findings, caption audit 73 chunks) printed.
3. **Era-hint path — exercised.**
   - `node tools/native_modes/era_hint.mjs --self-test` →
     **PASS, 13/13** (`logs/era-hint-selftest.log`): the stale fixture
     yields the full hint (era-mismatch symptom, `TALERENDER_BIN`
     remedy, CI provisioning commands, repo-checkout-slot warning);
     classification is positive-evidence only.
   - Binary-level probes (`logs/era-probes.log`): with the correct
     argument order (`--film-telemetry <DIR> --tables`, as
     check_mode spawns it), the stale v1.0.1 binary does **not**
     reject `--tables` — it proceeds to read film data (exit 3 on a
     missing dir, same as the new binary). Its stale surface in
     check mode is therefore a verdict *without tables*, surfacing
     only after the full audit — not the era-hint branch.
   - Honest negatives, recorded not smoothed: (a) a check-mode run
     against the stale binary was started and **killed by my own
     120 s timeout** (`logs/smoke-stale-binary-check.log` — header
     only; the audit was still running; exit 124 is the timeout, not
     the binary); (b) a frames-mode gate run against a
     scene-corrupted scratch copy (`scratch-broken/`) was likewise
     **killed at my 300 s timeout** — telemetry evaluates staging
     from film data and does not fail fast on a corrupt scene file,
     so the no-verdict era-hint branch was not reached end-to-end
     within the timeout (an earlier empty-`onoda.json` variant was
     caught even earlier, by the adapter's own film-data validator,
     exit 1 with the full missing list — fail-closed, but again not
     the hint branch). The hint branch itself is proven by the
     module self-test in (3) and by its wiring in
     `capture_native.mjs` `runTelemetry()`.

## 3. Part B — the flip package (`flip/`)

Paths mirror the skill tree rooted at
`~/workspace/custom-skills/tall-tale-shorts/`:

1. **`flip/SKILL.md`** — the shipped SKILL.md (Batch 2 integrated)
   with exactly two lines changed (verified by diff): front-matter
   `version: 6.0` → `7.0` and the `**Version 6.0**` → `**Version
   7.0**` line that carries the `CHANGELOG.md` pointer. The
   historical "(Versions up to 6.0 captured through headless
   Chrome…)" prose is untouched.
2. **`flip/CHANGELOG.md`** — B2-07's prepared v7.0 entry
   (`~/workspace/chrome-replacement/batch2/B2-07/CHANGELOG-7.0-entry.md`,
   staging preamble stripped) prepended above the v6.0 entry. Its
   five Batch 3 evidence slots are left as
   **`[TO FILL FROM CR-B3-*]` markers** for the lead: proof-film
   record, Onoda re-run verdict, failure-mode verdicts, flip-commit
   sha, GitHub sync result.
3. **`flip/template/tools/castsheet_native.py`** (+
   `castsheet_native.py.patch`) — the fallback-clearing edit: the
   `if mid in ("puzzled", "deadpan"): it["nativeMoodFallback"] =
   "neutral"` block is removed and the stale MOODS comment (17
   variants, gap open) is replaced with the closed-gap note +
   era gate. **Era gate: valid only against a binary ≥ `caf18c0`** —
   satisfied by the Part A binary (`63cc816`); against an older
   binary the two moods would render neutral *silently*. Functional
   proof: the staged generator emits 77 catalog items with **zero**
   `nativeMoodFallback` flags; the shipped generator flags exactly
   `puzzled` + `deadpan` (both runs logged in the seat session;
   manifests under `/tmp/cs-test`, `/tmp/cs-old` at run time).
4. **`flip/FLIP-CHECKLIST.md`** — the lead's ordered flip procedure:
   gates first (Batch 3 proof passed; Part A binary staged +
   era-checked; castsheet era gate), then binary swap → fill the
   CHANGELOG slots → land the three files + the one-line README
   stamp (per B2-07's rationale checklist; README deliberately not
   in this package) → re-verify flipped tree (`LIBRARY OK`,
   479 items; adapter check smoke) → the flip commit, separate and
   LAST → standing sync (verify `gh api user` prints
   `turjaun985123` first, then
   `~/workspace/tall-tale-upgrade/sync-skill-to-github.sh`, which
   self-aborts on account mismatch) → previews exclusion intact
   (mirror `.gitignore` still blocks `library/previews/` +
   `style-sheet.png`; remote tree grep → 0) → close the record,
   with the lavapipe/RADV caveat stated. Every step carries its
   verification.

## 4. Laws compliance

- No commits made (talerender HEAD movement during the seat is
  sibling seats' work, §1 race note). This note is a new untracked
  file, as tasked.
- Provisioned binary and shipped skill verified untouched after all
  runs (sha256 + `version: 6.0` + fallback flag still present in the
  shipped tree).
- No fabricated results: every PASS above names its log; the two
  timeouts are reported as timeouts, with the distinction between
  the timeout's exit code and the binary's behaviour stated.
