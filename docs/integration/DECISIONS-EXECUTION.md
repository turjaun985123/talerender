# Post-Wave-2 decisions execution — record

Authority: the user's ruling of **2026-11-15 07:21 +06** — "Approve
all six recommendations" (DECISIONS-W94.md). This record covers the
four decisions executed in this repo, in the ordered sequence
2 → 4 → 3 → 5. Decision 1 (captions: keep 73) required no action and
none was taken; the telemetry gate stayed green throughout.
Decision 6 (skill adoption, opt-in) is executed separately against
the skill tree, not here.

Standing-rule change mid-execution (user order, 2026-11-15):
rendered photos are never pushed to GitHub again — they go to the
user as zips. Executed in `3e2820a` (77 docs images removed,
121 MB, delivered as `talerender-photos-ab.zip` /
`talerender-photos-golden.zip`). Consequences for this execution:

- Golden-pin and A/B evidence IMAGES live locally only; hashes,
  metrics and verdicts are committed as TEXT (this file and the
  notes it cites).
- `make_golden.py` verify gained an explicit LOCAL-ONLY status for
  pinned-but-absent images (commit `06d2081`): on CI (no images) it
  prints 17 LOCAL-ONLY + a note and exits 0; STALE still fails.
  **CI can no longer verify image pins** — pin verification is a
  local/PC act against the restored (gitignored) images; CI gates
  on the text pins, film telemetry and the test suites. The
  visual-regression runner's references are local-only for the same
  reason, so a CI VR dispatch can no longer fully verify pins.
- Locomotion A/B contact sheets were produced locally and are held
  in `~/workspace/talerender-evidence/` (outside git) for delivery
  to the user as a zip; only their hashes/metrics are recorded here.
- The v1.0.1 golden gate was run LOCALLY against the restored
  images, output recorded in §5.

## 1. Decision 2 — s28 pin RETIRED (`b18b47c`)

Pre-change finding (reported before changing anything): s28 **did**
appear in the golden pack — `tools/golden/make_golden.py` ITEMS
(id `s28-onoda-t150`) and a pin in
`tools/golden/golden-hashes.json`. Retirement therefore removed the
golden item as well as the VR register entry, in the same commit
(the register's same-commit law): make_golden ITEMS entry, the
golden-hashes pin, the visual-regression workflow's `--render
s28-onoda-t150=…` line, the vr_triage REGISTER entry +
EXPECTED_IDS, all four vr_triage self-test fixtures rebuilt on the
post-retirement steady state, disposition recorded in
`docs/golden/NATIVE-GOLDEN.md`. The artefact
`docs/ab/onoda-s01-t150/native.png` was NOT deleted by this commit
(it was later removed from git by the no-photos commit `3e2820a`,
like all docs images, and is held locally). The ab1 pin untouched.

Verification at the landing head:

- Golden verify: **17 OK / 0 STALE / 0 MISSING** (was 18 OK — the
  delta is exactly this retirement), exit 0.
- `vr_triage.py --self-test`: PASS 4/4.
- VR dispatch run `43662989226` at `b18b47c`: **SUCCESS**,
  `VISUAL REGRESSION: PASS (3 pins, 0 drift)`; counts 3 MATCH ·
  14 NOT-RENDERED of 17; artifact triage: 3 OK · 14 KNOWN-DEBT ·
  0 NEW-SIGNAL, AS EXPECTED, exit 0.
- CI + Tools CI at `b18b47c`: SUCCESS / SUCCESS.

## 2. Decision 4 — lipsync Variant A (`bf1b27c`)

`tools/lipsync/build_visemes.py` patched exactly per
LIPSYNC-BIND-W86 §1.1: constant `CLOSED_EXIT_MAX = 0.05` (the
painter's open threshold, `src/toonfull.rs`) with cite; in
`_keys_from_segments`, a key carries `ease:"step"` iff its
predecessor in the final collapsed list has value ≤ 0.05; same rule
for the trailing duration key. (Variant A is the builder-side
closed-exit rule — NOT W59's literal silence-only wording, which
W86 disproved by measurement.) Baseline proven first: the unpatched
builder reproduced the committed demo byte-identically.

Regenerated and committed together: the demo track
(`tools/lipsync/fixtures/onoda-lipsync.demo.scene.json`, built from
the Onoda project's `words.json`, target narrator) and the expected
fixture (regenerated via the self-check's own build path).
**Exactly 77 of 551 keys changed, all ease-only (linear→step)** —
W86's predicted count. Self-check PASS; `check_clips` on the demo
PASS 5/5. CI + Tools CI at `bf1b27c`: SUCCESS / SUCCESS.

The track is unbound in the current film (zero frames change) —
confirmed: film telemetry with the `bf1b27c` CI binary prints
**FILM TELEMETRY: CHECK OK, exit 0**. (The run's output files were
later lost when this VM was replaced mid-execution; the sentinel
`exit=0` and the verdict line were read and reported before the
loss. A duplicate telemetry process from the first attempt was
found running concurrently and killed; the surviving run's verdict
is the one recorded.)

## 3. Decision 3 — locomotion adoption (D1–D4)

### 3a. Step 0 — registration (`b6d1fd0`)

One line: `mod locomotion;` in `src/main.rs` (between `mod math;`
and `mod paintbudget;`), per LOCOMOTION-STEP0-W87. Zero
`locomotion::` call sites outside the module at that head
(grep-verified). CI + Tools CI at `b6d1fd0`: SUCCESS / SUCCESS.
**CI test count rose 180 → 189 passed** (the 9 gait tests joining
`cargo test`; read from the CI log of run `43663855619`).
Telemetry at the step-0 head (CI binary of `b6d1fd0`):
**FILM TELEMETRY: CHECK OK, exit 0** (output preserved in the
evidence folder). Zero-pixel proof: the 40 standing-range probe
frames (195-204, 795-814, 1195-1204) rendered by CI at `bf1b27c`
(run `43665150684`) and at the step-0-era head `06d2081` (run
`43664830922`; `src/` identical to `b6d1fd0`) are
**byte-identical — 40/40 on the probe ranges, and in fact 0 of 1,811 frames differ across the whole film (bf1 render run `43665150684` vs step-0-era render run `43664830922`)** — see §3b note on method.

### 3b. Wiring (`56348ac`)

The seven film walk sites adopt the gait module per
LOCOMOTION-AB-W57 §5 step 2 + §6 census and LOCOMOTION-W24 §4,
via a new `gait_eval` helper in `src/film.rs` (GaitParams::walk(1.0)
with per-site step_length / bob_amp / arm_base; phase from
`phase_for_distance`, presented via `painter_phase`; bob = the
module's support-drop waveform):

- **D1** full replacement INCLUDING standing frames — phase+bob
  written on every gait-staged frame at all seven sites (s02
  group, s06 mk_walk, s07, s08 Suzuki, s08 Onoda, s10 Tani, s11
  walk-in); no gait-gating.
- **D2** companions swing from their (8,6)/(8,6) resting base
  (s02 pair via a shared eval, s06 mk_walk), not the module
  default.
- **D3** arm swing written only on advancing frames (driver
  strictly inside its window; s02 additionally requires
  legs == Walk — census frames 184-353; the k > 0.97 slide tail
  keeps staged arms). Census-free arms only: s07 arm_l (arm_r is
  the coconut beat), s08 Suzuki arm_r (arm_l holds the paper
  map), s10 arm_l (arm_r is the book beat/hold).
- **D4** s11 writes phase/bob only — the D15 authored walk-in
  arms (16,-6)/(10,5) untouched.
- Legs lines unchanged everywhere. `walk_phase` / `walk_bob`
  retained as the phase-law reference the locomotion tests cite,
  with a retention note.

CI + Tools CI at `56348ac`: SUCCESS / SUCCESS (189 tests).
Telemetry at `56348ac` (CI binary): ****FILM TELEMETRY: CHECK OK, exit 0****.

A/B (CI renders: baseline = step-0-era head `06d2081`, run
`43664830922`; post-wiring = `56348ac`, run `43665874388`; both
full 1,811-frame renders on CI lavapipe):

- **858 frames changed, ALL inside the gait footprint (858 of 888 footprint frames; per-range s02 214/214, s06 128/128, s07 101/115, s08 184/184, s10 159/159, s11 72/88); control ranges **0 of 923 changed**; standing probe ranges **40/40 changed** (the expected signature). PSNR sample (10 footprint frames): min 21.43 dB, mean 39.61 dB, all finite. Full note: `docs/ab/locomotion-adoption.md`**
- Changed-frame census vs W57's expectation (471 walking /
  888 bob-changed / 448 swing): **pixel-measured 858 changed frames vs the 888 staging-level bob-differing frames — the 30-frame delta is exactly the pre-driver rest frames of s07 (912-925) and s11 (1520-1535), where wk = 0 and the module rest bob coincides with the staged bob at phase 0, so they render identically. Walking (471) / swing (448) are staging-level subsets of the same changed set, not separately pixel-countable; containment (nothing changed outside the bob footprint) holds exactly**
- Determinism spot-check: **range 918-967 re-rendered at `56348ac` (run `43676113546`): **50/50 frames byte-identical** to the full render**
- Contact sheets (W57 §5.6 ranges) produced locally, held in
  `~/workspace/talerender-evidence/` for zip delivery to the user
  (no-photos rule); sheet hashes: **recorded in `docs/ab/locomotion-adoption.md` (s02 29 samples, s06 18, s07 16, s08 25, s10 11, s11 10)**

Golden impact at `56348ac` (local verify, restored images):
**local verify with restored images after the wiring: the six gait-segment mids (film-s02/s06/s07/s08/s10/s11) re-pinned to the adopted render in the SEPARATE commit `0d5f86b` (D4), each justified by the A/B above; verify after the act: 17 OK / 0 STALE. Disclosed in the same act: film-s02's pin was already blink-era stale pre-wiring (absorbed by the re-pin); film-s03's pin is blink-era stale but NOT gait-changed — deliberately NOT re-pinned, recorded as pre-existing pin debt in `docs/golden/NATIVE-GOLDEN.md`**

Method note: local film renders on this VM segfault under
SwiftShader while a telemetry run holds the machine (observed
2026-11-15; the same ranges render fine on CI), so all byte probes
were taken from CI renders, as the packs prescribe. The earlier
local probe attempt is not used as evidence.

## 4. LOCAL-ONLY adaptation (`06d2081`)

See the standing-rule section above. Additionally:
`docs/golden/NATIVE-GOLDEN.md` gained the "Where the images live"
section, and its s28 artefact line was reworked (the frame is held
locally, no longer in-repo) — the s28 retirement itself is text
(register/pins) and unchanged. Verified both modes locally:
no-images → 17 LOCAL-ONLY, exit 0, explicit note; images restored
→ 17 OK / 0 STALE. CI + Tools CI at `06d2081`: SUCCESS / SUCCESS
(Tools CI's golden step now prints the LOCAL-ONLY note on the
runner).

## 5. Decision 5 — v1.0.1 release

Bump commit **`967fa37`**: `Cargo.toml` version 1.0.0 → 1.0.1 +
the `README.md:65` current-version line, per RELEASE-101B-W89 §3.3
(the pack's one "decision" literal; all other 1.0.0 literals are
historical/minimum-version and were left untouched).

Six gates re-proven ON THE BUMP SHA:

| Gate | Evidence | Verdict |
|---|---|---|
| CI | run 43676800502 | SUCCESS |
| Tools CI | run 43677393319 | SUCCESS — dispatched manually: tools-ci.yml is path-filtered and the bump touches only Cargo.toml/README.md, so no push-triggered run exists for this SHA by design |
| Knobs gate | run 43677367966, exact line `KNOBS GATE: PASS` | SUCCESS + exact line present in the log |
| Determinism dispatch | run 43677370242 | SUCCESS |
| Golden verify (LOCAL, restored images — no-photos rule) | 17 OK / 0 STALE / 0 MISSING / 0 LOCAL-ONLY | PASS (local act, per the adjusted gate) |
| Film telemetry (bump-SHA CI binary) | `FILM TELEMETRY: CHECK OK`, exit 0 | PASS |

Binary verified before upload: `--version` prints exactly
`talerender 1.0.1`; help names `--res` / `--aa` / `--shadow` /
`--draft`; bad-flag smoke = clean error, exit 2, no panic.
Release: **https://github.com/turjaun985123/talerender/releases/tag/v1.0.1** — published (not draft, not
prerelease), target = bump SHA, sole asset `talerender`,
SHA-256 **b331f6704ebc8836057aa0e1b0f97421cb638c41ce9757abf0da230846903128**. The v1.0.0 release is untouched; its
proof MP4 was NOT attached (standing prohibition).

## 6. Disclosed incidents during execution

- A foreign commit (`3e2820a`, the no-photos removal, user order
  via the main chat) landed on origin/main mid-execution, on top of
  step 0. The wiring edits (uncommitted) were unaffected; the
  sequence was adapted per §4 without force-push.
- This VM was replaced twice mid-execution (uptime resets): /tmp was
  wiped, losing downloaded CI binaries and the lipsync telemetry
  run's output files (its verdict had already been read and is
  recorded in §2). All persistent state (repo, restored images,
  evidence folder) survived; affected runs were re-executed.
- Local SwiftShader film renders segfaulted under CPU/memory
  contention with a concurrent telemetry run; byte-probe evidence
  was taken from CI renders instead (§3b method note).
