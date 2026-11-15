# CR-B1-10 — Adapter mode-matrix implementation + P3 metadata sidecar

Chrome-Replacement Wave, Batch 1, Seat 10. Contracts:
`CHROME-REPLACEMENT-CONTRACTS.md` §1 (rows 5/6/7 + the adapter's existing
rows 1–4). All claims below verified against
`~/workspace/custom-skills/tall-tale-shorts/template/tools/capture_native.mjs`
and `template/tools/CAPTURE-NATIVE.md` as they stand on 2026-11-15.

**Delivered (NEW files only — no existing file was edited, nothing
committed):**

| File | What |
|---|---|
| `template/tools/native_modes/render_meta.mjs` (skill tree) | WP9 P3 sidecar: `writeRenderMeta(ctx, out)` / `buildRenderMeta(ctx, out)` / `sidecarPathFor(out)` + `--self-test` |
| `template/tools/native_modes/mode_dispatch.mjs` (skill tree) | Proposed dispatch for `castsheet` / `cues` / `preview`: `dispatchChangedMode(opts[, deps])`, `isChangedMode(mode)`, `CHANGED_MODES`, `cuesFreshness(projectRoot)` + `--self-test` |
| `template/tools/native_modes/era_hint.mjs` (skill tree) | Era-pairing message for the no-verdict gate failure: `eraHintMessage({...})`, `classifyNoVerdict({...})`, `hasVerdictLine(stdout)`, `provisioningRemedy(bin)` + `--self-test` |
| `docs/integration/CR-B1-10-ADAPTER.md` (this file, repo tree) | Integration note for the lead |

Node builtins only in all three modules. Self-tests (temp fixtures only,
no binary executed, no project touched):

- `render_meta.mjs --self-test` — **PASS, 13/13 checks, exit 0**
- `era_hint.mjs --self-test` — **PASS, 12/12 checks, exit 0**
- `mode_dispatch.mjs --self-test` — **PASS, 19/19 checks, exit 0**

Run as:
`node template/tools/native_modes/<module>.mjs --self-test` from the
skill tree (each also exits 2 with a usage line when run bare).

---

## 1. Dispatch table — before / after

### Before (verified in `capture_native.mjs` today)

`main()` checks a `REFUSALS` table first; each refusal goes through
`fail()` → `main().catch` prints `capture-native: <msg>` → **exit 1**.
Unknown modes print the modes line → **exit 2**. Binary discovery +
manifest/film-data validation run before any surviving mode.

| Mode | Behaviour today | Exit |
|---|---|---|
| `check` | Native: `talerender --film-telemetry`; prints `CHECK OK` / `CHECK FAILED` | 0 / 1 |
| `frames` | Native: gate first, then renders + copies `t<time>.png` | 0 / 1 |
| `sheet` | Native (partial): tiles native frames; no gate; prints the "golden-sheet procedure stays Chrome" note | 0 / 1 (2 on bad step/cols) |
| `video` | Native: gate first, renders, finishes (x264 CRF 18 slow master / CRF 23 draft, `.part` + rename). Provenance is stdout only | 0 / 1 |
| `cues` | **REFUSED.** Text: cues are extracted from the project page (`OP.cues()` reads `src/shots.ts`); "no native equivalent … already baked into `data/mix.wav`". Fallback printed: `Run: node tools/capture.mjs cues` | 1 |
| `castsheet` | **REFUSED.** Text: previews staged through `castsheet.html` / CAST page in the browser; "no native castsheet mode and no native preview census". Fallback: `Run: node tools/capture.mjs castsheet` | 1 |
| `preview` | **REFUSED.** Text: serves the live project page; "the native backend has no page to serve". Fallback: `Run: node tools/capture.mjs preview` | 1 |
| *(unknown)* | Prints `modes (native): check · frames · sheet (partial) · video — Chrome-only, refused here: cues · castsheet · preview` | 2 |
| — (gate) | `frames`/`video` no-verdict case fails closed with: "film telemetry produced no verdict line (binary \<how\>) — treated as a FAILED gate. Exit 3 means the film data could not be read …; exit 2 usually means the binary predates --film-telemetry — use a current CI artifact." (see §4 for the gap) | 1 |

### After (with this seat's modules wired as in §2 — proposed, not yet in the adapter)

| Mode | Behaviour after | Exit |
|---|---|---|
| `check` / `frames` / `sheet` | **Unchanged** (sheet's golden-clause retirement is the sheet seat's / Batch 2's text, not this seat's) | unchanged |
| `video` | Unchanged, **plus**: after the `.part` → final rename succeeds (and only then), `writeRenderMeta(...)` writes `<out>.render.json` beside the output (§3) | unchanged |
| `castsheet` | **Native.** `mode_dispatch` dynamically imports the sibling `native_modes/castsheet_mode.mjs` (Batch 1 castsheet seat) and calls `castsheetMode({ projectRoot, outdir, kind, talerenderBin, log })` — `outdir` = `rest[0]` resolved against the project root (default `review/cast`), `kind` = `rest[1]` or `undefined`. Missing module → loud exit-1 failure naming `tools/native_modes/castsheet_mode.mjs`; loaded-but-wrong-export and thrown errors likewise exit 1 with the cause. **No Chrome suggestion anywhere in this path** (asserted by self-test) | 0 on success, 1 on any failure |
| `cues` | **Routing, fail-closed.** Prints the resolution: cues are authored data emitted by `tools/cues_native.py` (Batch 1 cues seat) from bridge/scene data, consumed by `tools/mix.py` / `audio_recipes.py build_cues`. Reports the state of `data/cues.json`: missing / **newer than the scene data** (newest input + both timestamps named; scene inputs = `native/film.json`, `native/scenes/film/*.scene.json`, `native/film/onoda.json`, `src/shots.ts`) / older (= stale, regenerate) / unverifiable. Then prints the exact command — `python3 tools/cues_native.py` — and exits non-zero in **every** case, including the fresh-file case: the adapter did not produce the file in this run and never silently passes a mode it did not run | 1 always |
| `preview` | **Retired.** Prints that the mode retired with Chrome (it only ever served the project page; never a production step) and names the native review deliverables with exact commands: `sheet review/all.jpg`, `frames review/frames "1.2,8.4"`, `video renders/out.mp4` | 1 (refusal discipline — the mode is known, it is retired; not usage-error 2) |
| *(unknown)* | **Unchanged** — `dispatchChangedMode` returns `null` for anything outside the three modes, so the adapter's exit-2 usage path stands. Only the printed modes line needs its text updated (§5, item D2) | 2 |

Exit-code discipline is unchanged throughout: 0 success, 1 capture
failed / refused / named prerequisite missing, 2 usage/selection —
exactly the `CAPTURE-NATIVE.md` table.

---

## 2. Where each module plugs in (`capture_native.mjs`, lead's edit)

**P-A — imports** (top of file, beside the existing `node:` imports):

```js
import { dispatchChangedMode, isChangedMode } from './native_modes/mode_dispatch.mjs';
import { writeRenderMeta } from './native_modes/render_meta.mjs';
import { eraHintMessage } from './native_modes/era_hint.mjs';
```

(The adapter lives in `template/tools/`; the modules live in
`template/tools/native_modes/`, hence `./native_modes/…`.)

**P-B — changed-mode dispatch** in `main()`, replacing the `REFUSALS`
table + its `if (REFUSALS[mode])` block, and placed **before** binary
discovery (none of the three modes needs the gate; `cues`/`preview`
need no binary at all). The lead may instead discover the binary first
and pass it — `castsheet` forwards whatever it is given:

```js
if (isChangedMode(mode)) {
  process.exitCode = await dispatchChangedMode({
    mode, rest, projectRoot: ROOT,
    talerenderBin: undefined, // or the discovered `bin` if discovery moves first
  });
  return;
}
```

Delete the `REFUSALS` constant once this lands — its texts are
superseded (the old `cues` claim "already baked into data/mix.wav" was
about the finished mix, not the `data/cues.json` that `mix.py` consumes;
the new routing text states the emitter route instead).

**P-C — P3 sidecar** in `videoMode`, immediately after
`fs.renameSync(part, out)` and the two existing `console.log` lines
(never before the rename: a failed encode must not leave a sidecar
claiming a finished render):

```js
const { sidecarPath } = writeRenderMeta({
  backend: 'native',
  binary: { path: ctx.bin, via: ctx.via, version: ctx.binaryVersion },
  manifest: { path: ctx.manifestPath },
  mode: 'video',
  frameRange: { from: n0, count: n, fps: FPS },
  encode: { draft, preset: draft ? 'veryfast' : 'slow', crf: draft ? 23 : 18, audioBitrate: draft ? '192k' : '256k' },
  gate: { verdict: 'CHECK OK' }, // videoMode only reaches here past gateOrStop()
  resolution: { width: ctx.manifest.W, height: ctx.manifest.H },
  durationS: ctx.manifest.durationS, hasMix,
}, out);
console.log(`render metadata: ${sidecarPath}`);
```

Two small context widenings this needs (lead's call, both trivial):
`ctx` currently carries `{ bin, manifest, filmDir }` — add `via` and
`manifestPath` (both already in scope in `main()`), and capture
`ctx.binaryVersion` once after discovery. Verified: the binary
supports it — `src/main.rs` handles `-V` / `--version` ("Print the
version and exit", main.rs:160, 321) — but the adapter never calls it
today, so the lead's wiring is
`spawnSync(bin, ['--version'], { encoding: 'utf8' }).stdout?.trim()`
with a failure tolerated as `null` (the sidecar records null, never a
guess). `render_meta` computes the manifest sha256 itself from
`manifest.path` when no hash is supplied.

**P-D — era hint** in `runTelemetry()`: replace the final no-verdict
`fail(...)` with

```js
fail(eraHintMessage({ status, signal: r.signal, stdout: r.stdout, stderr: r.stderr, bin, via }));
```

`runTelemetry(bin, filmDir)` needs `via` added to its parameters (or
read from a module-scope discovery result). Semantics are identical —
still `fail()`, still exit 1, still fail-closed; only the text grows
(§4). The OK/FAILED line-vs-exit-code disagreement branches are
untouched.

---

## 3. P3 sidecar format (`render_meta.mjs`)

`<out>.render.json` = the full output filename + `.render.json`
(e.g. `renders/out.mp4.render.json`), per WP9 P3's spelling. Written
via temp sibling + rename, mirroring the adapter's `.part` discipline.

```json
{
  "format": "talerender-render-meta", "version": 1,
  "generatedAt": "<ISO>", "backend": "native", "mode": "video",
  "output": "out.mp4",
  "binary":   { "path": "...", "via": "TALERENDER_BIN", "version": "<--version output verbatim>" },
  "manifest": { "path": "...", "sha256": "<hex>", "title": null, "totalFrames": null },
  "frameRange": { "from": 0, "to": 1811, "count": 1811, "fps": 30 },
  "resolution": { "width": 1080, "height": 1920 },
  "durationS": 60.37,
  "encode":   { "draft": false, "preset": "slow", "crf": 18, "audioBitrate": "256k" },
  "audio":    { "source": "data/mix.wav" },
  "gate":     { "verdict": "CHECK OK", "recognised": true }
}
```

Honesty rules (self-tested): a field the caller did not supply is
`null`, never invented; the `--version` output is recorded verbatim
when supplied and `null` when not (the module never runs the binary);
the manifest sha256 is the one measured exception — computed from the
file on disk when a path is given without a hash, and on read failure
it is `null` with `manifest.sha256Error` naming why. The context
accepts the nested shape above and flat aliases (`bin`, `via`,
`gateVerdict`, `n0`/`n`, `draft`, …) so the lead can pass the adapter's
existing variables without reshaping. `frameRange.to` is exclusive,
matching the adapter's `[n0, n0+n)` ranges.

---

## 4. Era-pairing error surface — verified gap + fix

**What the adapter prints today** (final branch of `runTelemetry()`):

> film telemetry produced no verdict line (binary \<how\>) — treated as
> a FAILED gate. Exit 3 means the film data could not be read (the
> reason is on stderr, above); exit 2 usually means the binary predates
> --film-telemetry — use a current CI artifact.

**Gap:** it names a cause but not the provisioning remedy the contract
requires — no `TALERENDER_BIN`, no CI download commands, no warning
that the repo-checkout discovery slot is the usual stale source
(`CAPTURE-NATIVE.md` Prerequisite 1 documents all three; the error
text names none). So `era_hint.mjs` is delivered.

**What it prints instead.** Classification is positive-evidence only:
`stale` ⇔ exit 2 or the binary's own `unknown argument:
--film-telemetry` on either stream. For a stale run:

> film telemetry produced no verdict line (binary exit 2) — treated as
> a FAILED gate.
> Symptom: era mismatch — the talerender binary (via …) predates the
> film gate: it does not recognise --film-telemetry (…). The binary
> and the project's bridge data must come from compatible repo states;
> a stale binary renders nothing rather than rendering wrong frames.
> Remedy: Provision a current binary from a green CI run of
> turjaun985123/talerender and point TALERENDER_BIN at it:
> `gh run list …` / `gh run download <run-id> … -n talerender-binary
> -D ~/talerender-bin` / `chmod +x …` / `export
> TALERENDER_BIN=~/talerender-bin/talerender`. Do not rely on the
> repo-checkout discovery slot (…). Then re-run the same command.
> There is no flag to render past a failed gate.

For a non-stale no-verdict run (exit 3, crash, signal) the message
keeps today's data-cause explanation, offers the era mismatch as a
named possibility **with the one-line currency check** (the
`grep -q 'unknown argument: --film-telemetry'` probe from
`CAPTURE-NATIVE.md`), and appends the same `TALERENDER_BIN` remedy —
offered, not asserted. A run that *did* produce a verdict line is not
this module's case: both entry points return `null`.

---

## 5. `CAPTURE-NATIVE.md` text changes required (lead's edit — none made here)

- **D1 — Modes table:** replace the row
  `` `cues` · `castsheet` · `preview` | — | **refuse**: Chrome-only, exit 1, fallback instructions printed ``
  with three rows: `castsheet` (native, delegates to
  `tools/native_modes/castsheet_mode.mjs`; outputs per that module's
  contract), `cues` (routing only: prints the `tools/cues_native.py`
  route + `data/cues.json` freshness, exit 1 always), `preview`
  (retired with Chrome; native review deliverables are sheet /
  frames / video).
- **D2 — Header + selection section:** the adapter header comment
  ("cues · castsheet · preview — Chrome-only: they REFUSE here") and
  the modes line printed for unknown modes must be updated to match
  D1; the "adapter never falls back to Chrome silently" wording
  should become "there is no Chrome fallback for these modes" where
  it referred to the three.
- **D3 — New sidecar paragraph** under *video — finishing parity*:
  a successful video render also writes `<out>.render.json`
  (`talerender-render-meta` v1: backend, binary path/via/`--version`,
  manifest path + sha256, frame range, encode settings, gate verdict)
  via `tools/native_modes/render_meta.mjs`; a failed encode writes
  none.
- **D4 — Era-pairing subsection:** append that the no-verdict gate
  failure now prints the symptom + the `TALERENDER_BIN` provisioning
  remedy inline (via `tools/native_modes/era_hint.mjs`), so the
  remedy no longer lives only in Prerequisite 1.
- **D5 — Exit-codes table:** no code changes; add a clarifying note
  that `cues` (routing) and `preview` (retired) exit 1 under the
  existing "a mode refused" clause.
- **D6 — Prerequisites / film-data section:** no change from this
  seat. (The hardcoded Onoda asset list is P4 — the film-data
  validator seat's file, `native_modes/filmdata_validate.mjs`, not
  this seat's.)

## 6. Dependencies on sibling Batch 1 deliveries

- `castsheet_mode.mjs` (castsheet seat): `mode_dispatch` is written
  against the contracted export
  `castsheetMode({ projectRoot, outdir, kind, talerenderBin, log })`.
  Until it lands, `castsheet` fails loudly by design (self-tested
  with an injected missing-module error). If the landed signature
  differs, only `castsheetDispatch` in `mode_dispatch.mjs` changes.
- `tools/cues_native.py` (cues seat): the routing text assumes it is
  run as `python3 tools/cues_native.py` from the project folder and
  writes `data/cues.json`. If the landed tool needs arguments, the
  single constant `CUES_COMMAND` in `mode_dispatch.mjs` is the one
  place to change (plus D1's wording).
- Nothing in this seat's modules imports or assumes
  `filmdata_validate.mjs`.

## 7. Not done / not claimed

- `capture_native.mjs` itself is **not** edited — §2 is the lead's
  integration diff. Until it lands, the shipped adapter still refuses
  all three modes with the old Chrome-fallback text.
- No live adapter run was made (no binary executed); verification is
  the three module self-tests above plus code reading.
- All proof is local/static; nothing here speaks to RADV vs lavapipe.
