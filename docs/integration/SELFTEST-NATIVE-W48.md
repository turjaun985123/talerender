# SELFTEST-NATIVE-W48 — frame-hash comparison tool + the native selftest leg spec

Seat W48, TaleRender Wave 2 Batch 5 (gap-fix batch), 2026-11-14.
Scope: gap **G19** / requirement **R10** support — *"selftest.sh
must pass with the native path selected, including a frame-hash
comparison against the Chrome path for one project."* This seat
delivers the comparison tool and the leg spec. It does **not**
touch the skill: everything about the shipped skill's
`scripts/selftest.sh` is outside this repo, and every claim about
it below is either cited to the skill-wiring work copy
(`~/workspace/talerender-skill-wiring/`, read read-only) or marked
**TO-VERIFY**.

Deliverables (new files only, nothing committed):

- `tools/ci/frame_hash_compare.py` — the tool.
- `tools/ci/fixtures/frame_hash/` — synthetic fixture dirs.
- This note — verification record + the leg spec.

---

## 1. The tool

`frame_hash_compare.py REF_DIR CAND_DIR [--pattern GLOB]
[--strict | --band DB]`

Compares two directories of same-named frames (default glob
`frame_*.png`; the comparison set is the union of matching
names), hashes each pair with sha256, and classifies every frame
in the **visual-regression dialect** (`tools/ci/visual_regression.py`,
W16), so the selftest speaks the same language as the W16/W37
tooling:

| class | meaning | fails? |
|---|---|---|
| `MATCH` | sha256 identical | no |
| `WITHIN-BAND` | bytes differ, PSNR ≥ band (band mode only; includes pixel-identical re-encodes, PSNR inf) | no — a signal to look; wants an explanation in the run's notes |
| `DRIFT` | band mode: PSNR < band, **or PSNR unavailable for a differing pair** (fails closed — an undecodable difference cannot be proven within band) | yes |
| `DIFFERS` | strict mode: bytes differ (PSNR reported as diagnostic where decodable) | yes |
| `MISSING-IN-CANDIDATE` / `MISSING-IN-REFERENCE` | name exists on one side only (the MISSING-RENDER / REFERENCE-MISSING role: a frame that did not happen must not read as a pass) | yes, both modes |

Modes: `--strict` (default — any difference fails; the
same-renderer regime, where the post-cabcc30 norm is
byte-identity) and `--band DB` (the cross-path regime; §3.3
explains why the cross-path leg **cannot** be strict). Choosing
a band is a policy act stated on the command line; band mode has
no silent default. The W16 band is **80 dB**.

PSNR is **imported, not duplicated**: the tool loads the sibling
`byte_probes.py` by path and uses its stdlib PNG decoder + house
PSNR (`10*log10(255²/MSE)`, MSE over all decoded channels,
float64) — the same import pattern `visual_regression.py` uses
for `tools/ab/measure_ab.py`, so the definitions cannot drift
apart. Decoder subset: bit depth 8, colour types 0/2/4/6,
non-interlaced. Outside it → "PSNR unavailable" with the reason.

Exit codes (house convention; the exit code agrees with the
verdict line, as in `visual_regression.py`): **0** `FRAME HASH:
PASS`, **1** `FRAME HASH: FAIL`, **2** input error/refusal — one
`ERROR:` line on stderr, **no verdict line** (a side containing
no matching frames at all is refused as no-evidence, the same
discipline as `byte_probes.py`).

Relationship to neighbours: `byte_probes.py` remains the
renderer-change gate over film frame-number ranges on one
renderer; this tool compares whole directories of *named*
frames (the review-frames shape) across two production paths.

## 2. Verification (all first-hand, this VM, CPython stdlib only)

Fixtures are 64×64 RGBA PNGs from deterministic pixel formulas
(gradient / 16-px blocks / diagonal), written with a filter-0
encoder. `native-identical` = byte copies of `chrome/`.
`native-withinband` = `frame_00001.png` with exactly **one
sample +1** (pixel 0,0 red). `native-drift` = `frame_00001.png`
with the right half inverted. `native-missing` = `frame_00002`
absent, extra `frame_00003` present. (At fixture size one wrong
sample is 90.28 dB; at full 1080×1920 a single wrong *pixel*
sits at ~116 dB per W16's math — the fixture value is a
size effect, not a band argument.)

`python3 tools/ci/frame_hash_compare.py --self-test` — verbatim:

```
self-test A: chrome vs native-identical, strict -> expect PASS, exit 0
frame hash compare: reference=/home/hatch/workspace/talerender/tools/ci/fixtures/frame_hash/chrome candidate=/home/hatch/workspace/talerender/tools/ci/fixtures/frame_hash/native-identical pattern=frame_*.png mode=strict
  frame_00000.png: MATCH (sha256 5e178cf675a03082…)
  frame_00001.png: MATCH (sha256 052bb3ce0534181a…)
  frame_00002.png: MATCH (sha256 292f6a473c4deabf…)
FRAME HASH: PASS (3 MATCH; 3 frame name(s), mode strict)
self-test: identical dirs exit 0: ok
self-test: identical dirs all MATCH: ok
self-test B1: chrome vs native-withinband, strict -> expect FAIL (DIFFERS), exit 1
frame hash compare: reference=/home/hatch/workspace/talerender/tools/ci/fixtures/frame_hash/chrome candidate=/home/hatch/workspace/talerender/tools/ci/fixtures/frame_hash/native-withinband pattern=frame_*.png mode=strict
  frame_00000.png: MATCH (sha256 5e178cf675a03082…)
  frame_00001.png: DIFFERS (PSNR 90.28 dB; sha256 reference 052bb3ce0534181a… / candidate 304ed885aac2c246…)
  frame_00002.png: MATCH (sha256 292f6a473c4deabf…)
FRAME HASH: FAIL (2 MATCH, 1 DIFFERS; 3 frame name(s), mode strict)
self-test: withinband strict exit 1: ok
self-test: planted frame named DIFFERS: ok
self-test B2: chrome vs native-withinband, band 80 -> expect PASS (WITHIN-BAND), exit 0
frame hash compare: reference=/home/hatch/workspace/talerender/tools/ci/fixtures/frame_hash/chrome candidate=/home/hatch/workspace/talerender/tools/ci/fixtures/frame_hash/native-withinband pattern=frame_*.png mode=band 80 dB
  frame_00000.png: MATCH (sha256 5e178cf675a03082…)
  frame_00001.png: WITHIN-BAND (PSNR 90.28 dB >= 80 dB; sha256 reference 052bb3ce0534181a… / candidate 304ed885aac2c246…)
  frame_00002.png: MATCH (sha256 292f6a473c4deabf…)
FRAME HASH: PASS (2 MATCH, 1 WITHIN-BAND; 3 frame name(s), mode band 80 dB)
self-test: withinband band-mode exit 0: ok
self-test: planted frame classified WITHIN-BAND: ok
self-test C: chrome vs native-drift, band 80 -> expect FAIL (DRIFT), exit 1
frame hash compare: reference=/home/hatch/workspace/talerender/tools/ci/fixtures/frame_hash/chrome candidate=/home/hatch/workspace/talerender/tools/ci/fixtures/frame_hash/native-drift pattern=frame_*.png mode=band 80 dB
  frame_00000.png: MATCH (sha256 5e178cf675a03082…)
  frame_00001.png: DRIFT (PSNR 10.53 dB < 80 dB; sha256 reference 052bb3ce0534181a… / candidate 032c4126f491161c…)
  frame_00002.png: MATCH (sha256 292f6a473c4deabf…)
FRAME HASH: FAIL (2 MATCH, 1 DRIFT; 3 frame name(s), mode band 80 dB)
self-test: drift band-mode exit 1: ok
self-test: planted frame classified DRIFT: ok
self-test D: chrome vs native-missing, strict -> expect FAIL, both directions named, exit 1
frame hash compare: reference=/home/hatch/workspace/talerender/tools/ci/fixtures/frame_hash/chrome candidate=/home/hatch/workspace/talerender/tools/ci/fixtures/frame_hash/native-missing pattern=frame_*.png mode=strict
  frame_00000.png: MATCH (sha256 5e178cf675a03082…)
  frame_00001.png: MATCH (sha256 052bb3ce0534181a…)
  frame_00002.png: MISSING-IN-CANDIDATE (present in reference only)
  frame_00003.png: MISSING-IN-REFERENCE (present in candidate only)
FRAME HASH: FAIL (2 MATCH, 1 MISSING-IN-CANDIDATE, 1 MISSING-IN-REFERENCE; 4 frame name(s), mode strict)
self-test: missing-fixture exit 1: ok
self-test: absent frame named MISSING-IN-CANDIDATE: ok
self-test: extra frame named MISSING-IN-REFERENCE: ok
SELF-TEST: PASS
```

(exit 0.) Input-error discipline, verbatim stderr:

```
$ frame_hash_compare.py /nonexistent <dir>
ERROR: reference dir /nonexistent does not exist or is not a directory     (exit 2)
$ frame_hash_compare.py <a> <b> --band -3
ERROR: --band must be positive (got -3)                                    (exit 2)
```

Undecodable differing pair in band mode fails closed as DRIFT
(synthetic JPEG pair in /tmp, verbatim):

```
frame hash compare: reference=/tmp/fh-a candidate=/tmp/fh-b pattern=frame_*.jpg mode=band 80 dB
  frame_00000.jpg: DRIFT (PSNR unavailable (not a PNG file) — cannot prove within band, fails closed; sha256 reference 5116f61646f774ff… / candidate f300683535632cd5…)
FRAME HASH: FAIL (1 DRIFT; 1 frame name(s), mode band 80 dB)               (exit 1)
```

## 3. The leg spec — a native frame-hash leg for the skill's selftest

### 3.1 The law: extend, never bypass (G19)

The gap analysis's G19 is explicit: the renderer swap must
**extend** the skill's selftest, not bypass it — the selftest is
the only thing that guarantees a fresh project works. So the
leg specified here is **additive**: every Chrome-path leg of
`selftest.sh` runs unchanged and first; the native hash leg is
one more leg inside the native smoke, selected only when the
operator selects the native backend. An operator who never
selects native is never failed — or slowed — by it (the work
copy's gate discipline, `tools/SELFTEST-NATIVE.md` in
`~/workspace/talerender-skill-wiring/`).

### 3.2 Place in the sequence

The work copy's `tools/SELFTEST-NATIVE.md` ("Proposed invocation
point in `scripts/selftest.sh`") proposes invoking the native
smoke **once, immediately before the final `SELFTEST PASSED`
verdict block**, so it runs only after the whole Chrome pipeline
has already passed, guarded by the same `RENDERER=native` flag
the script itself checks; `docs/ADOPTION-REVIEW-W59.md` §3
states the same placement ("invoked from `scripts/selftest.sh`"
after the Chrome gates, with the exit treatment in §3.4 below).
The frame-hash leg belongs **inside that native smoke**, after
its telemetry legs: it needs frames from *both* paths of the
same project, and the Chrome side of the pair only exists once
the Chrome pipeline has run. Placing it mid-script next to the
Chrome `check` gate is wrong for the same reason the work copy
gives for the smoke itself: that gate runs inside the throwaway
project directory, while the native legs are project- and
provisioning-dependent.

### 3.3 Era pairing — binary + bridge data from compatible repo states

The rule, in the work copy's own terms
(`docs/ADOPTION-DECISION-W94.md` §2): *"binary + bridge data
must come from compatible repo states (the era-pairing rule —
a stale pairing fails closed at the gate: safe, but blocked)"*;
and `docs/SELFTEST-W79.md`: *"Binary and script must be from
the same era (era-pairing rule…) ; a stale pairing fails closed
as ENV-BLOCKED…, which is safe but blocked."*

For this leg, concretely:

- The binary comes from the stable path named by
  `TALERENDER_BIN`, provisioned from a **green CI run's**
  `talerender-binary` artifact — **never** the repo-checkout
  discovery fallback: `docs/ADOPTION-REVIEW-W59.md` §3 records
  that the fallback "silently selected a stale build in every
  local wave run," and its fix is part of the adoption gate.
- The leg's output header must record the pairing evidence:
  `talerender --version`, the CI run the binary was provisioned
  from (or its sha256), and the bridge-data provenance (the repo
  state the project's film data was packaged from). A pairing
  whose eras cannot be shown compatible is **not a frame
  verdict** — it is classed ENV-BLOCKED (§3.4) before any frame
  is compared.
- The stale-binary symptom is known and must be recognised, not
  misread as a renderer failure (`tools/selftest_native.sh`
  header, work copy): a binary older than the telemetry CLI
  answers `unknown argument: --telemetry`, exit 2 — reported
  **ENV-BLOCKED (stale provisioning), never FAIL**. The same
  treatment applies to this leg: an era-mismatched pair blocks
  the leg; it does not fail the backend.

### 3.4 Which project, which times, which mode

- **Project:** exactly one — R10's acceptance is a *one-project*
  comparison. The natural subject is the selftest's own
  throwaway project (the one its Chrome legs already build), so
  the leg adds no new fixture project. **TO-VERIFY:** which
  project the shipped `scripts/selftest.sh` builds.
- **Times:** the same named times the project's review-frames
  step already samples, rendered by both paths to **same-named
  PNG files** in two directories. **TO-VERIFY:** the shipped
  selftest's / capture's exact frame-time list.
- **Format prerequisite (hard):** PSNR needs PNG on both sides.
  The Chrome review path writes JPEG frames, and a differing
  JPEG pair is undecodable by the stdlib decoder → DRIFT by the
  fail-closed rule (§2). The leg therefore requires the Chrome
  side to emit PNG frames at the chosen times. **TO-VERIFY:**
  whether the shipped Chrome capture can emit PNG at named times
  (its `frames` mode is JPEG per the skill's docs); if it
  cannot, this prerequisite is a small skill-side addition, and
  until it lands the leg is ENV-BLOCKED, not faked with JPEGs.
- **Mode: `--band 80`, never `--strict`, for the cross-path
  leg.** Chrome masters carry one q0.93 JPEG generation and
  native frames are clean (decision (B), compatibility-contract
  §5 — restated in the work copy's `docs/NATIVE-BACKEND.md`):
  byte-identical hashes across the two paths are impossible
  **by design**, so a strict cross-path gate would fail forever
  and teach operators to ignore it. The band is the W16 band
  (80 dB — above the 72.34 dB worst same-renderer defect on
  record, far below single-pixel noise at full frame size).
  `--strict` remains the right mode for same-path comparisons
  (native re-render vs native reference), which is the
  byte-probe regime, not this leg.

### 3.5 Verdicts: what blocks, what warns

Mapping this tool's outcomes onto the native smoke's class
system (work copy `tools/SELFTEST-NATIVE.md` exit table and
`docs/ADOPTION-REVIEW-W59.md` §3: *SKIP / ENV-BLOCKED do not
fail the selftest; FAIL does*):

| leg outcome | selftest class | effect |
|---|---|---|
| `FRAME HASH: PASS`, all rows MATCH | PASS | proceed |
| `FRAME HASH: PASS` with WITHIN-BAND rows | PASS, **warn** | proceed; each WITHIN-BAND frame wants an explanation in the run's notes (W16 discipline) |
| `FRAME HASH: FAIL` (DRIFT / missing frames) | **FAIL** | fails the selftest (native smoke exit 1) |
| tool exit 2 — a side rendered nothing / wrong dir | **FAIL of the leg's inputs**, reported as a broken run, never a soft pass | fails the selftest once the leg was actually selected and provisioned; a provisioning absence is the next row instead |
| binary missing / stale era / era pairing unprovable / Chrome-side PNG prerequisite unmet | **ENV-BLOCKED** (exit 3) | printed, selftest continues — but it is **not** a pass: the leg has produced no verdict (SELFTEST-W79's honesty rule: an ENV-BLOCKED refresh "confirms the state, it does not advance it") |
| `RENDERER` ≠ `native`, or `TALERENDER_BIN` unset | **SKIP** (exit 0) | one SKIP line; Chrome flow unaffected |

### 3.6 TO-VERIFY register (shipped skill, outside this repo)

Nothing in this section was invented; each item is open because
the shipped skill's files are not this repo's to read as spec:

1. The shipped `scripts/selftest.sh`'s actual leg sequence and
   final verdict block (the work copy's invocation point is a
   *proposal*; SELFTEST-W79 records it was not yet wired —
   "no reference to `selftest_native` exists in
   `scripts/selftest.sh` or `SKILL.md`" at its writing).
2. Which project the shipped selftest builds, and its
   frame-time list (§3.4).
3. Whether the shipped Chrome capture can emit PNG frames at
   named times (§3.4's hard prerequisite).
4. Whether the shipped selftest propagates a native leg's exit
   code with the SKIP / ENV-BLOCKED / FAIL treatment of §3.5
   (proposed in the work copy; adoption is a separate, pending
   user decision).

## 4. Limits

- Hash comparison proves *agreement between two paths at the
  sampled times*, not correctness: two identically wrong frames
  MATCH. Correctness stays with the telemetry gate and the A/B
  evidence (the byte-probe pack states the same law for its
  regime).
- The band is a policy parameter. This seat ships the tool and
  the reasoning for 80 dB (W16's); if the lead or the user sets
  a different band for the leg, that is a commit-message-level
  decision, not a run-time improvisation.
- No RADV or Chrome execution happened here: the fixtures are
  synthetic and the leg is a spec. The first real run of the leg
  is an adoption-time event on a machine with both paths
  provisioned, under the era-pairing rule.
