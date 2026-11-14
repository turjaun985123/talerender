# EXPORTER-ACCEPT-W77 — the exporter acceptance harness: contract + local-run report

**Seat:** Wave 2 Batch 8, W77. **Deliverables (new files only,
nothing committed):** `tools/format/exporter_acceptance.py` +
synthetic fixtures under `tools/format/fixtures/exporter_acceptance/`.
The harness is the repo-side gate EXPORTER-W47 §4 designed: any
exporter output — or any hand-authored scene file claiming
equivalence with a Chrome shot — must pass it. The exporter itself
remains skill-side/future; this harness is what it will be judged by.

---

## 1. Usage contract

```
exporter_acceptance.py --samples DIR --bin PATH
    [--band 45.0] [--film-dir DIR] [--timeout 300] [--renders DIR]
exporter_acceptance.py --scene FILE --ref PNG --bin PATH [same options]
exporter_acceptance.py --self-test
```

**Sample layout.** A samples directory holds pairs:
`foo.scene.json` + `foo.ref.png` (the Chrome reference frame for
that scene). The render is made at the reference's own pixel size
(`--width/--height` are taken from the reference), so a size
mismatch is a finding, not a defaults artefact.

**In-place mode.** `--scene/--ref` tests one scene where it lives.
This mode exists because repo scene files resolve their textures
relative to their own location — `scenes/ab1.scene.json` reaches
`../docs/ab/onoda-s01-t150/assets/caption.png` — so copying a repo
scene into a samples directory breaks its data (proven locally,
§3.2). Exporter output is self-contained by design and uses
`--samples`; repo scenes use `--scene/--ref`.

**What runs, in order** (mirroring W47 §4 steps 4–7): each sample
is rendered via the binary's `--scene-file` mode and scored; the
first sample that renders is rendered a second time for the
**determinism spot-check** (§4.7 — the pair must be byte-identical);
if `--film-dir` is given, `--film-telemetry` runs over it and the
gate reads the verdict line (§4.6 — `FILM TELEMETRY: CHECK OK`,
exit 0). Data gates (§4.3 validators) are not duplicated here —
they already run in tools-ci before any render is attempted.

**Classifications.**

| class | meaning |
|---|---|
| `PASS` | PSNR ≥ band (default **45.0 dB**, W47 §4.5). Infinite PSNR (identical RGB) is PASS. The §4.5 rider (≤ 0.01 % of pixels with max-channel Δ > 10) is printed per sample and annotated when exceeded; the class follows the PSNR bar. |
| `BELOW-BAND` | A real render was produced and scored under the band, or could not be scored against the reference's geometry. **A quality verdict.** |
| `ENV-BLOCKED` | The environment prevented a verdict: binary failure (no adapter, panic, crash), no output, timeout, or the **black-frame signature** (render RGB ~uniform black — mean ≤ 1.0 and ≥ 99.9 % pure-black pixels — against a reference whose RGB mean > 10). **Never a quality verdict**: a software-renderer wall says nothing about the scene or the exporter. A genuinely dark pair (reference also near-black) is not the wall and falls through to PSNR. |
| `MISSING-REFERENCE` | No `foo.ref.png` beside the scene file. The gate cannot run for that sample. |

**Exit codes** mirror the telemetry convention (EXIT-CODES-W02),
with one stable verdict line:

- **0** — `EXPORTER ACCEPTANCE: PASS`: every sample PASS,
  determinism byte-identical, telemetry CHECK OK when requested.
- **1** — `EXPORTER ACCEPTANCE: FAIL (...)`: every sample received
  a quality verdict and at least one failed (BELOW-BAND,
  determinism mismatch, or telemetry CHECK FAILED).
- **3** — `EXPORTER ACCEPTANCE: COULD NOT RUN (...)`: any sample
  MISSING-REFERENCE or ENV-BLOCKED, the binary or samples
  directory unusable, or telemetry could not run. Precedence is
  deliberate: an incomplete gate is reported as not-run even when
  a completed leg also failed — a partial FAIL must not
  masquerade as a full verdict.

**PSNR — the house math, with one declared plumbing deviation.**
The score comes from `tools/ci/byte_probes.py`'s `psnr_of_pair`,
**imported, not re-derived**. But byte_probes refuses mixed-channel
pairs outright, and Chrome references decode to 3 channels against
the native render's 4 — so unmodified, it cannot score a single
real exporter pair. The harness therefore normalises both sides to
RGB before calling it (dropping the candidate's alpha after
checking opacity; a non-opaque candidate is flagged in its detail
line). This is the repo's own established native-vs-Chrome
convention — `film/ab_compare.py`, the tooling behind every film
A/B table, is RGB MSE — and it is verified, not asserted: RGB-only
scoring of `docs/ab/onoda-s01-t150`'s recorded pair reproduces
`metrics.json`'s **20.152 dB exactly** (§3.4 cross-checks the full
chain end to end).

**Named bias (W47 §4.1).** Chrome references carry one JPEG
generation (decision (B) — native frames stay clean); that bias
counts against the native render in every PSNR the harness prints.

---

## 2. Self-test

`--self-test` runs three synthetic cases (fixtures under
`tools/format/fixtures/exporter_acceptance/`) against a stub
binary that "renders" by copying a planted frame — the harness's
classification logic exercised end to end with no GPU:

| case | planted truth | harness verdict | exit |
|---|---|---|---|
| `case_identical` | stub frame = reference bytes | PASS, PSNR inf, determinism byte-identical | 0 ✓ |
| `case_shifted` | stub frame = reference shifted 1 px (18.2 dB RGB) | BELOW-BAND, FAIL line, rider exceeded (1.51 % px>10) | 1 ✓ |
| `case_missing_ref` | scene with no `.ref.png` | MISSING-REFERENCE, COULD NOT RUN line | 3 ✓ |

**Result: SELF-TEST: PASS (3/3 cases)**, re-run after every
harness change made during this seat.

---

## 3. Local run — the honest report

Binary under test: `~/talerender-bin/talerender` —
`talerender 1.0.0`, sha256 `bb1f595145f36f49841ee2566e9c7ba08617bf9dee78570b7c7048771dd6f306`
(the professional-CLI build; it predates the Wave 2 landings, which
is fine for a harness shakedown and stated so no number below is
read as current-HEAD renderer quality).

Sample: `scenes/ab1.scene.json` vs its Chrome reference
`docs/ab/onoda-s01-t150/original.png` (1080×1920), plus
`--film-dir film` for the telemetry leg.

### 3.1 No Vulkan ICD on this VM → ENV-BLOCKED, as classified

This VM has **no Vulkan driver ICD at all** (no lavapipe files
anywhere on disk; the loader exists but has nothing to load — the
replacement-VM state recorded earlier in the wave). Plain run:

```
  ab1    ENV-BLOCKED  binary exit 101: thread 'main' panicked at
         src/main.rs:309:10: no wgpu adapter found — a Vulkan
         driver (hardware or lavapipe) is required
```

The harness quoted the panic's location *and* message (a panic
splits them across two stderr lines; the evidence picker joins
them — a harness fix made during this seat after the first local
run's evidence line named a line number and hid the cause).

### 3.2 The copied-scene trap (why `--scene/--ref` exists)

Copying `ab1.scene.json` into a scratch samples dir and rendering
the copy panics differently:

```
thread 'main' panicked at src/scenefile.rs:1233:48:
scene.textures[5]: cannot load texture file
/home/hatch/workspace/.w77-scratch/samples/../docs/ab/onoda-s01-t150/assets/caption.png:
No such file or directory (os error 2)
```

The scene's texture paths are relative to its own directory.
Classified ENV-BLOCKED (a sample-setup failure, no quality
verdict) — and the reason `--scene/--ref` in-place mode was added.

### 3.3 With SwiftShader: a real verdict

Chrome's bundled SwiftShader ICD
(`~/.faceless-shorts/chrome/opt/google/chrome/vk_swiftshader_icd.json`,
via `VK_ICD_FILENAMES`) gives the binary a software Vulkan adapter:

```
adapter: SwiftShader Device (Subzero) | backend: Vulkan |
device type: Cpu | driver: SwiftShader driver
scene: file — 1 (format vab1 — Onoda S01 @ t=1.50 (scene-format
re-expression), direct path, 334 draws, 6 textures)
```

The ab1 scene file takes the **direct path**, not the composer —
so the lavapipe/llvmpipe black-frame wall does not apply to this
sample, and the harness's final run produced real verdicts:

```
exporter acceptance — 1 sample(s), band 45.0 dB, binary /home/hatch/talerender-bin/talerender
  ab1    BELOW-BAND  PSNR 20.30 dB  79.5413% px>10 (W47 §4.5 rider exceeded)
  determinism: PASS  ab1: two renders byte-identical (1003077 bytes)
  telemetry: COULD-NOT-RUN  exit -9: film-telemetry: sample 20 (t=4.75)
EXPORTER ACCEPTANCE: COULD NOT RUN (telemetry could not run; 0 pass, 1 below-band)
harness exit=3
```

### 3.4 Reading the 20.30 dB — calibration, not an exporter failure

**There is no exporter yet, and this number is not one failing.**
`ab1.scene.json` is the §28 *hand transcription* of a Chrome shot,
whose recorded score is **20.152 dB** (lavapipe native vs the same
Chrome original, `docs/ab/onoda-s01-t150/metrics.json`). The
harness, on a different software adapter (SwiftShader) with a
different-vintage binary, reproduces that anchor at **20.30 dB** —
and an independent hand computation of the same pair through the
harness's own functions agrees to the printed digit (20.30 dB;
79.541 % px>10 vs the recorded 79.904 %; candidate alpha uniformly
opaque, min 255). W47 §4.5 itself sets the 45 dB bar for *exporter*
fidelity — same staging emitted for both engines — and records
this anchor's ~20 dB as a pre-format transcription artefact. The
harness measuring the known anchor correctly, on both its sample
and determinism legs, is the strongest local evidence available
that the scoring chain is calibrated. BELOW-BAND is the honest
class for this pair under the exporter bar.

### 3.5 Telemetry leg: could not complete on this VM

The `--film-telemetry film` leg did not finish locally: two
attempts hit the harness's 300 s per-invocation cap under
multi-agent load, and the final attempt (cap raised to 1800 s)
was **OOM-killed at audit sample 20 (t=4.75)** after ~24 minutes
with the VM's 6 GiB shared across the wave's seats (a second
telemetry instance from another seat's dry run was resident
throughout). The harness reported `COULD-NOT-RUN` with the kill
as evidence — the class working as designed. The telemetry
verdict for this data on a working environment is already of
record (CHECK OK, exit 0, CI). No local telemetry claim is made.

---

## 4. The CI dispatch shape that produces real verdicts

On the pinned `ubuntu-22.04` runners the lavapipe ICD exists and
the composer path renders correctly (CI frames have been the
wave's evidence of record throughout) — so the full gate,
including v2/composer samples that this VM env-blocks, runs there:

```yaml
# workflow_dispatch — exporter-acceptance
steps:
  - uses: actions/checkout@v4
  - # build or download the CI release binary, as in render.yml
  - name: Stage samples
    run: |
      # exporter output dir + Chrome reference frames, laid out
      # as <name>.scene.json + <name>.ref.png pairs
      ls samples/
  - name: Acceptance
    env:
      VK_ICD_FILENAMES: /usr/share/vulkan/icd.d/lvp_icd.x86_64.json
    run: |
      python3 tools/format/exporter_acceptance.py \
        --samples samples/ \
        --bin ./target/release/talerender \
        --film-dir exported-film/ \
        --timeout 1800 \
        --renders acceptance-renders/
  - # gate on the step's exit code; archive the transcript and
    # acceptance-renders/ as the run's evidence
```

Per W47 §4.4 the adapter class is part of the claim: CI lavapipe
verdicts prove correctness; a RADV claim requires the same
invocation on the user's PC, and numbers from the two are never
mixed in one table.

---

## 5. Deviations from the seat brief, declared

1. **RGB normalisation before `psnr_of_pair`** (§1) — the brief's
   import instruction stands for the math; the normalisation is
   plumbing the brief did not foresee, forced by byte_probes'
   mixed-channel refusal and justified by the exact reproduction
   of the recorded 20.152 dB anchor under the repo's RGB
   convention.
2. **`--scene/--ref` in-place mode added** (§3.2) — repo scene
   files are not copyable into a samples directory; without this
   mode the harness could not test the repo's own scenes at all.
3. **Exit-3 precedence over exit-1** when a gate is incomplete
   (§1) — a partial FAIL is not allowed to read as a full verdict.
