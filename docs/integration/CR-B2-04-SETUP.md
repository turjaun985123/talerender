# CR-B2-04 — Native setup (Chrome-replacement wave, Batch 2, Seat 04)

Mission row: contract §1 row 10 (`scripts/setup.sh` Chrome check →
REPLACE-NATIVE). Deliverables staged, NEW files only, nothing
committed, nothing edited in place:

- `~/workspace/chrome-replacement/batch2/B2-04/setup.sh` — the
  replacement setup script (self-contained; see §4 for the
  helpers decision).
- `~/workspace/chrome-replacement/batch2/B2-04/helpers/` —
  `fetch_talerender.sh`, `vulkan_probe.sh`, `era_check.sh`
  (standalone forms of the three native checks).
- `~/workspace/chrome-replacement/batch2/B2-04/telemetry-run.log` —
  the full era-check run log from this VM (§3).

All four scripts pass `bash -n`. No system packages were installed
and no browser was downloaded during this seat, per the seat laws.

## 1. Audit of the current setup (step by step)

Source audited: `~/workspace/custom-skills/tall-tale-shorts/scripts/setup.sh`
(the whole file; it calls only `scripts/voice.py --download`, `npm`,
`pip`/`uv`, and system tools — verified by reading it end to end),
plus the consumers that justify each step.

| # | Current step (evidence) | Disposition | Why |
|---|---|---|---|
| 1 | PATH prepend, `SKILL`/`FACELESS_HOME`/`VENV` resolution | KEEP (verbatim shape) | Backend-agnostic plumbing. New script adds a `TALL_TALE_SKILL` override for staged testing. |
| 2 | Node.js ≥ 22 check (hard fail) | KEEP | Still the adapter runtime. Verified from code: `capture_native.mjs` + `native_modes/*.mjs` import ONLY `node:` builtins and relative files — Node is needed, npm capture deps are not. |
| 3 | Bun detection (warning only) | KEEP (warning only, reworded) | Bun was never the renderer; it builds edited project sources (`bun run build` in `new_video.sh`). Native capture itself needs no Bun. |
| 4 | Shared node_modules hub in `$FACELESS_HOME` + `npm install` from skill-root `package.json` (`typescript`, `@types/three` only) | KEEP | Its consumer is selftest's typecheck gate + authoring typecheck, never Chrome. **Puppeteer: NONE exists** — grep-verified: neither `package.json` (skill root nor template) contains puppeteer/playwright; `capture.mjs` drives Chrome over raw CDP/WebSocket with zero npm packages. There is no puppeteer step to remove. |
| 5 | TypeScript-via-hub hard check (v5 P-08) | KEEP | `selftest.sh` exits 1 without `$SKILL/node_modules/.bin/tsc`; setup must not print "All set" over a broken gate. |
| 6 | ffmpeg + ffprobe check (hard fail) | KEEP | Native finishing is the same step (x264 CRF 18 slow / draft CRF 23, BT.709, AAC, `.part` discipline — `capture_native.mjs` video mode), sheet tiling is ffmpeg `drawtext`, loudness is ffmpeg. |
| 7 | **Chrome discovery loop + hard requirement** (the `CH` loop over Chrome/Chromium/Edge/Brave paths + `$CHROME`) | **REMOVE → REPLACE** | The only Chrome step in setup. Replaced by §2's three native steps (binary, Vulkan probe, era gate). The new `setup.sh` contains no browser path, binary name as a *target*, or `CHROME=` handling at all — the word survives only in header comments stating the policy (6 comment lines; Batch 3's grep audit should classify them as policy text, not code). |
| 8 | Python 3.10+ detection (`python3.12…python`, `py -3`) | KEEP | Unchanged; consumers below never used Chrome. |
| 9 | Free-space pre-check (≥ 1.5 GB, v5 P-08) | KEEP | Model downloads still dominate; the release binary is ~20 MB and does not change the budget. |
| 10 | venv creation (`uv venv` or `python -m venv`) in `$FH/venv` | KEEP | Unchanged. |
| 11 | `pip install numpy scipy kokoro-onnx faster-whisper` + import check | KEEP | Consumers verified from imports: `voice.py` (numpy, scipy.io, kokoro-onnx models), `words.py` (faster-whisper), `template/tools/mix.py` (numpy, scipy.signal/io), `template/tools/cues_native.py` (stdlib + the same env via `scripts/py`). Cues/mix tooling is why the charter's "python deps for cues/mix" stays exactly this set — no addition needed, none removed. |
| 12 | Kokoro model fetch via `voice.py --download` (~340 MB, hash-pinned inside `voice.py`) | KEEP | Voice pipeline is NOT-CHROME (contract §1 row 20). |
| 13 | Whisper `small.en` fetch (~0.5 GB) | KEEP | Same — NOT-CHROME. |
| 14 | Fonts | **NOT A SETUP STEP — now an explicit verify** | The old script never downloaded fonts (SKILL.md's "fonts" in the setup summary is doc drift). Fonts ship in `template/fonts/` (Inter-Black/ExtraBold .otf, Caladea-Bold .ttf + OFL texts — listed on disk) and are copied per-project by `new_video.sh`; `native_modes/sheet_mode.mjs findFont()` searches project `fonts/` sans-first, then DejaVu Sans Bold. New setup verifies the two bundled faces exist and fails as "tree damaged" otherwise — a check, not a download. |
| 15 | `chmod +x scripts/*.sh scripts/py`; "All set" / exit-1 discipline | KEEP (+ one change) | New script adds an "All set (with warnings…)" variant: a WARNING means a verdict was *not obtained* (e.g. software Vulkan, fixture-less era probe) and the final line now says so instead of letting warnings scroll past a bare "All set". |

Nothing else is called by the old setup: no Chrome *download*
exists in it (it only *required* a pre-installed Chrome — the
bundled-Chrome path in `capture.mjs`'s discovery is a capture-time
concern owned by other Batch 2 seats, not setup).

## 2. The new setup's native steps

In order (after node/bun/hub/ffmpeg/fonts above):

1. **TaleRender binary — present or fetched.** Discovery mirrors
   the adapter's `findBinary` order: explicit `TALERENDER_BIN`
   (a bad path is a hard error, never second-guessed) → the setup
   install dir `~/talerender-bin` → repo checkout (accepted only
   with a WARNING naming its stale-build history) → `PATH`.
   If none is found, setup fetches the **pinned release asset**:
   `TALERENDER_VERSION` (default `v1.0.1`, the current release —
   the lead re-pins by changing one variable plus the digest),
   downloaded to a `.part`, **sha256-verified** against the pinned
   digest, `chmod +x`, atomically renamed. Pinned digest for
   v1.0.1: `b331f6704ebc8836057aa0e1b0f97421cb638c41ce9757abf0da230846903128`
   — verified two independent ways on 2026-11-15: the release's
   published asset digest (`gh release view v1.0.1 --json assets`;
   asset `talerender`, 19,589,440 bytes) and `sha256sum` of the
   provisioned binary `~/talerender-bin/talerender` (identical).
   For any unpinned version there is deliberately **no default
   digest**: setup refuses to fetch until `TALERENDER_SHA256` is
   set — an unverified binary is never installed silently. The
   CI-artifact route (`gh run download … -n talerender-binary`)
   is printed as the fallback text on download failure, not
   attempted automatically (it is not version-pinned).
   Setup ends the step by printing the
   `export TALERENDER_BIN=…` line for the user's shell profile.
2. **Vulkan probe — probed, not assumed** (`helpers/vulkan_probe.sh`
   is the standalone form). Layers: ICD-manifest inventory in the
   standard loader dirs (+ `VK_ICD_FILENAMES`/`VK_DRIVER_FILES`),
   classified RADV/ANV/NVIDIA/lavapipe; `vulkaninfo --summary`
   when present (advisory only — its absence is a warning, never
   a verdict); then the **authoritative binary smoke**: one
   `--scene-file` frame to a temp PNG, classified from positive
   evidence — adapter line naming hardware → HARDWARE pass;
   lavapipe/llvmpipe → SOFTWARE pass, explicitly labelled
   correctness-only; stderr containing `no wgpu adapter found`
   → **NO-ADAPTER**, its own hard failure with the
   `mesa-vulkan-drivers` remedy; timeout / other non-zero /
   exit-0-without-a-valid-PNG → OTHER-FAILURE. "No adapter" is
   never lumped in with other failures.
3. **Era-pairing gate** (`helpers/era_check.sh` standalone) — §3.
4. Python env + models, exactly as audited (steps 8–13 above).

**Idempotency:** every step is check-then-act — hub migration and
symlink only when absent, npm/pip installs only when the import /
tsc checks fail, model fetches delegated to `voice.py --download`
(which hash-verifies and skips good files) and the Whisper loader
(cache-resident), binary fetch skipped when the installed bytes
already match the pin (proven live, §3), probes are read-only.
Re-running on a provisioned machine performs checks and downloads
nothing. **No system packages are installed by the script** —
missing system pieces (ffmpeg, Vulkan drivers, vulkan-tools) are
named with their install command, matching the old script's
discipline. **No CUDA anywhere**: the target is RADV/Vulkan on
the user's Linux PC (Ryzen 5 PRO 5650G / Vega 7); the script says
so in its header.

## 3. Era-pairing check — design and REAL run output

**Design.** Setup runs `talerender --film-telemetry <fixture>`
where the fixture is the known-good repo film data
(`~/workspace/talerender/film`, containing `onoda.json` + `assets/`;
overridable via `TALERENDER_FILM_FIXTURE`). Telemetry is CPU-only,
so the gate runs on any machine. The verdict **line** is
authoritative and the exit code is cross-checked, exactly as the
adapter's gate (`capture_native.mjs runTelemetry`) and
`native_modes/era_hint.mjs` specify:

- `FILM TELEMETRY: CHECK OK` + exit 0 → PASS.
- `CHECK FAILED` line (exit 0 or 1) → hard FAIL (a verdict against
  known-good data = binary/data mismatch).
- No verdict line → stale iff exit 2 or the output contains
  `unknown argument: --film-telemetry` (positive evidence only) →
  hard FAIL whose text is the era-hint remedy: provision the
  release binary, `export TALERENDER_BIN=~/talerender-bin/talerender`,
  plus the warning not to rely on the repo-checkout slot.
- Fixture absent on the machine → era-probe fallback: a telemetry
  call on a nonexistent dir must NOT answer `unknown argument`
  (if it does → STALE, hard fail); a data error proves current
  era, and setup proceeds with a WARNING that the full CHECK OK
  gate was not run — it never claims a verdict it didn't see.

**Real runs on this VM (2026-11-15, provisioned binary
`~/talerender-bin/talerender`, `talerender 1.0.1`, sha256 as §2):**

(a) Full fixture gate — `~/talerender-bin/talerender --film-telemetry ~/workspace/talerender/film`.
First attempt was killed at sample 140/241 by a VM runtime
restart (no verdict; the partial log is the exec record). The
re-run completed in ~21 min wall on this 1.5-shared-CPU VM.
Verdict tail (full log staged at
`~/workspace/chrome-replacement/batch2/B2-04/telemetry-run.log`):

```
film-telemetry: sample 240 (t=59.75)
paint-cache: entries=74 bytes=2133852160 hits=96 misses=386 evictions=312 rerenders=0 peak_bytes=2162688000 budget_bytes=2147483648
FILM TELEMETRY — THE SOLDIER WHO DIDN'T KNOW THE WAR WAS OVER (native, 0.25 s sampling)
CANVAS OVERFLOW: none
CONTINUITY: clean (5 named characters tracked)
COMPOSITION: 31 findings (caption-zone 0, safe-area 8, thirds 3, framing 2, edge-touch 18)
CAPTION AUDIT: 73 chunks, 0/12 lines authored, min fit 1.00, max rendered width 844 px
FILM TELEMETRY: CHECK OK
EXIT:0
```

(The EDGE TOUCH / SAFE AREA / FRAMING lines between OVERFLOW and
CONTINUITY are warnings inside a passing gate — the same findings
the Chrome check reported as warnings; the gate verdict is the
CHECK OK line.) **Result: PASS — the provisioned v1.0.1 binary is
era-paired with the repo film data.**

(b) The staged helper itself, run end-to-end against the same
fixture (`bash helpers/era_check.sh ~/talerender-bin/talerender ~/workspace/talerender/film`,
~11 min wall on the warmer cache) — final lines verbatim (full
log staged at `~/workspace/chrome-replacement/batch2/B2-04/era-helper-run.log`):

```
FILM TELEMETRY: CHECK OK
era-check: PASS — FILM TELEMETRY: CHECK OK (fixture /home/hatch/workspace/talerender/film, exit 0). Binary and film-data era are paired.
HELPER_EXIT:0
```

**Result: the staged era-check helper passes end-to-end, exit 0.**

(c) No-fixture era probe (`TALERENDER_FILM_FIXTURE=/nonexistent-fixture`,
helper): `era-check: SKIP-FIXTURE — binary answers --film-telemetry
with a data error (current era)…`, exit 4. Correct: current era
proven, no verdict claimed.

(d) Vulkan probe on this VM (`helpers/vulkan_probe.sh`): this
(replacement) VM currently has **no Vulkan ICD at all** — no
`/usr/share/vulkan/icd.d`, no `/dev/dri`, `vulkaninfo` absent —
and the binary smoke answered exactly
`error: GPU initialisation failed (request_adapter (gpucore)): no wgpu adapter found — a Vulkan driver (hardware or lavapipe) is required`,
exit 3. The probe classified it **NO-ADAPTER (exit 3)**, distinct
from other failures, with the driver remedy. This also corrects
the contract §0 note for the *current* VM instance (it claims a
lavapipe ICD; this instance has none — VM replacements change
the software stack). Consequence: on THIS VM the new setup.sh
would correctly fail its vulkan step; on the user's RADV PC the
same probe is the RADV confirmation. The fetch helper's
idempotent path was also run live: `fetch: … already matches
v1.0.1 (sha256 verified) — nothing to do`, exit 0. The release
*download* path was designed and digest-verified but **not**
executed (the binary was already provisioned; the laws for this
seat did not require a redundant 20 MB download, and the .part →
verify → rename logic is the same code path as the proven
idempotent check up to the network call).

## 4. Decisions the lead should know

- **setup.sh is self-contained** (all three native checks inlined)
  so it works wherever it is dropped into `scripts/`; the
  `helpers/` files are the same logic as standalone diagnostics
  (independently runnable, separately exit-coded). If the lead
  prefers DRY, setup.sh can be rewired to source
  `helpers/` — but then the helpers must be installed alongside
  it (suggested home: `scripts/` next to setup.sh).
- **Version pin location:** `TALERENDER_VERSION` + the `case`
  digest table at the top of the binary step in both `setup.sh`
  and `helpers/fetch_talerender.sh` — two places, deliberately
  identical; a version bump must touch both (or the DRY rewire
  above removes the duplication).
- The staged `setup.sh` resolves the skill tree relative to
  itself, with a fallback to the known live path when staged
  outside the tree, so it can be smoke-tested from the seat dir.
  Once installed at `scripts/setup.sh` the primary resolution is
  the correct one.

## 5. Unresolvable in this wave (named)

1. **RADV/hardware Vulkan proof** — impossible on this VM (no
   ICD, no `/dev/dri`); the probe's HARDWARE branch is untested
   live. It rides the user's PC session (contract §0 already
   carries this; PC-SESSION-W98). The SOFTWARE (lavapipe) branch
   is likewise untested on this instance for the same reason —
   only NO-ADAPTER, OTHER paths' signatures, and the telemetry
   gate were exercised here.
2. **Release-download path not executed** (see §3d) — designed,
   digest-pinned, idempotent path proven; the network fetch
   itself should be exercised once at integration on a machine
   without `~/talerender-bin` (or with it renamed aside).
3. **Full `setup.sh` end-to-end run not performed** — by seat law
   (no system installs; the model-download steps would re-fetch
   ~0.9 GB against an already-provisioned machine). Its native
   steps were each proven individually (§3); the kept steps are
   byte-near-identical to the shipping script's. First full run
   belongs to Batch 3's fresh-scaffold selftest.
4. **SKILL.md's setup summary line** ("venv…, node deps, fonts,
   Chrome check") is stale in two places (fonts, Chrome) — doc
   rewrite is another Batch 2 seat's file; flagged here, not
   edited (ownership law).
5. `scripts/selftest.sh` still hard-requires the hub tsc and the
   Chrome capture flow — its native rewrite is contract row 11,
   a sibling seat; this setup keeps provisioning everything the
   *current* selftest needs so nothing regresses before that lands.
