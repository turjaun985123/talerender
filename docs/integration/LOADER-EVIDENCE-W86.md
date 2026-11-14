# LOADER-EVIDENCE-W86 — fx-textures + fog chart with the current CI binary

Seat W86, 2026-11-14. Scope: first-hand loader-lane evidence for the
two items W71/W74 left evidence-thin — (a) `fx-textures` loading
through the completed texture registry, (b) the fog chart on the
native side after the lead's fog wiring (`69952ab`, verified by the
lead in `5b0889d`). Nothing committed by this seat; `src/` untouched.
Scratch (frames, CI downloads, A/B pair) in `~/workspace/.w86-scratch/`.

## 0. Binary under test

Provisioned exactly per `CAPTURE-NATIVE.md` (work copy): the
`talerender-binary` artifact of a green CI run at head ≥ `69952ab`.

- CI run **43575835793**, head **`5b0889d`** (success) — `69952ab`
  plus the spec-only addendum, no code change.
- Installed at `~/talerender-bin/talerender`; 19,548,480 bytes;
  sha256 `7e487d73af642cbbf41a6f84876eaacd0b33dc841c0a68ff9643bee70533fcbd`.
- Currency probes: `--film-telemetry` recognised (no
  `unknown argument`); `--scene-file /nonexistent.json` reaches the
  loader and panics on the file read — the flag dispatches, it is not
  an unknown argument.
- Local renderer: llvmpipe (LLVM 20.1.2, 256 bits), Vulkan, device
  type Cpu, Mesa 25.2.8-0ubuntu0.24.04.4. CI frames below are native
  CI lavapipe (render.yml dispatch runs at head `69952ab`).

## 1. fx-textures — PASS, and byte-identical to CI

`talerender --scene-file scenes/fixtures/fx-textures.scene.json`
(direct path; the fixture has no `post` block), exit 0:

- Scene line: "2 (format vfx-textures — registry census: one quad per
  generator the film data names, direct path, **14 draws, 14
  textures**)" — eager resolution exercised every arm with no
  `unknown texture generator` error.
- Output PNG: 1080×1920, 62,924 bytes, sha256
  `6ef2e39b3d2cc3589e6655435bff52f8e5ff638ffc0aedf603f250500577304d`.
- Content sanity: 790 unique colours; channel means
  (59.26, 54.62, 52.13), channel stdev (59.26, 43.79, 32.32) —
  a rich, non-blank census frame, as designed.
- **Cross-check:** the CI render-dispatch frame for the same fixture
  (run **43575103004**, head `69952ab`) has the *identical* sha256.
  Local llvmpipe and CI lavapipe agree bit-for-bit on this render.
  The same held for `fx-fog` (local sha256 `4f12acc9…` = CI run
  43575109536's frame).

Verdict: the acceptance W71 §4 specified is satisfied in substance —
the fixture loads through the complete registry (13/13 arms + the
`file` texture) and renders end-to-end, proven locally and matched
byte-for-byte against the CI dispatch at the fog-wiring head.

## 2. Fog chart — the gate passes in CI (confirmed first-hand); this VM's llvmpipe renders every v2 composer scene black

### 2.1 Local runs (current binary, this VM): both frames 100% black

`scenes/fog-chart.scene.json` and `scenes/fog-chart-nofog.scene.json`
both exit 0 via the composer path — and both output PNGs are a single
colour, (0, 0, 0) over all 2,073,600 px, **byte-identical to each
other** (sha256 `8d3c7c0b80c960f9d1f736efcf047c739f7805530314e05c7003ca08b761ed2e`).
Sampler on the local fog frame: every centre [0, 0, 0], worst basic
|Δ| = 214 codes. That is not a fog verdict — the no-fog control is
identically black, so fog is not the variable. Localisation, same
binary, same VM, one variable changed at a time:

| scene | format / path | local result |
|---|---|---|
| fx-textures | v2-fixture / direct | content, byte-identical to CI |
| fx-fog (fog block present) | v2 / direct | content (5 colours), byte-identical to CI |
| dof-chart | v1 / composer | content (241 colours) |
| fog-chart + fog-chart-nofog | v2 / composer | **100% black** |
| character-v2 (no fog, no lighting block) | v2 / composer | **100% black** (same sha256) |

The failing class is **format-v2 scenes on the composer path under
this VM's llvmpipe (Mesa 25.2.8)** — the same family as the known
local llvmpipe PBR segfault: this VM's software Vulkan is not a
renderer of record for the heavy paths. CI lavapipe and the user's
PC (RADV) are. No local fog conclusion can be drawn from this VM,
in either direction.

### 2.2 CI frames (the lead's dispatches at `69952ab`): PASS, ≤1 code everywhere

Independent re-measurement of the lead's own CI artifacts with
`tools/fog-chart/chrome/sample_centres.py` against
`tools/fog-chart/expected.json`:

- **Fog frame** — run 43575069908, sha256 `aff4fb0f…`: every panel
  centre within **≤1 code** of `expectedSrgb`. Basic panels: worst
  |Δ| = 1 (basic-3m R −1; all other basic channels exact), incl.
  basic-25m exactly (195, 201, 187), the pure fog colour. Standard
  panels: **Δ = 0 at all five depths**. The ±2 gate passes with
  margin, on the basic (pure-fog) panels and the standard panels
  alike — the standard centres are gated against the analytic
  column, per W74 §3; the ×π caveat concerns native-vs-Chrome
  comparison only (below).
- **No-fog control** — run 43575075993, sha256 `5a4d3062…`: every
  centre within ≤1 code of `expectedSrgbNoFog` (basic-25m B +1; the
  rest exact), and it diverges from the fog column exactly as the
  smoothstep law predicts (e.g. basic-15m 155 codes from the fog
  expectation, 0 from its own). The two CI frames differ — fog is
  demonstrably applied, and the control regression is clean.
- The classic-ramp discriminator is settled by the same numbers: a
  linear-ramp build would miss the basic panels by 26/12/8 codes at
  6/10/15 m; the measured misses are 0.

### 2.3 PSNR leg (supplementary, caveated — not a gate failure)

CI fog frame vs the Chrome reference
(`tools/fog-chart/chrome/project/review/full/t1.00.jpg`,
Chrome SwiftShader, JPEG), `measure_ab.py --pair`: **23.565 dB**,
mean |Δ| 5.69, 19.30% px > 10, max |Δ| 113 — below the ≥45 dB line
in W74 §3, for the reason that section itself states: the Chrome
rig's standard panels carry the house/π light factor
(VERIFY.md:333–356), so the standard half of the chart differs by
reference construction, and the JPEG floor caps the rest. On the
gated basic half, agreement is centre-exact in substance: Chrome's
basic centres sit ≤1 code from the analytic column (W66) and the
native CI basic centres sit ≤1 code from the same column (§2.2).
A ≥45 dB full-chart number requires the ×π-amended Chrome
re-capture — a reference-side task, still outstanding; it is not a
native defect and does not touch the fog verdict.

## 3. Comparison with the lead's CI verdict — AGREEMENT

The lead's `5b0889d` records "fog chart ≤1 code at every panel".
Re-sampling the lead's own CI frames first-hand reproduces that
exactly: **≤1 code at every panel** for the fog frame against
`expectedSrgb`, and ≤1 for the control against `expectedSrgbNoFog`.
No divergence on substance. The one divergence is environmental:
the same binary on this VM's llvmpipe renders every format-v2
composer scene black (§2.1), so the gate cannot be re-run locally
on this VM — it stands on CI lavapipe evidence, independently
re-measured here, and on the fx-textures/fx-fog byte-identities
that tie the local binary to the CI one on every path this VM can
execute.

## 4. Verdicts

- **fx-textures: PASS** — loads through the complete registry and
  renders with the current binary, locally and in CI, byte-identical
  frames.
- **Fog chart (native): PASS in CI** — fog frame ≤1 code at every
  panel vs the analytic gate (bar ±2), control ≤1 vs its column,
  smoothstep law confirmed, ramp excluded. Local re-run on this VM:
  **not executable** (v2-composer renders black under llvmpipe
  Mesa 25.2.8 — environment limitation, precisely bounded in §2.1).
- **Lead's ≤1-code claim: CONFIRMED** first-hand from the CI
  artifacts.
