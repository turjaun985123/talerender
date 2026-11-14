# LIGHTS-W26 — light-rig validator (Wave 2, seat W26)

**Deliverables (new files only; nothing existing edited, nothing committed):**

- `tools/format/check_lights.py` — the validator (house style of
  `tools/film/validate_film.py` / `tools/format/check_clips.py`:
  numbered checks, per-check `[n name] PASS/FAIL — headline` lines,
  `OVERALL: PASS/FAIL (n/6 checks passed[, w warning(s)])`,
  exit 0 pass / 1 fail / 2 input error with a one-line stderr ERROR).
- `scenes/fixtures/lights/` — 9 fixtures: one valid, one per failure
  class, one warnings-only.
- This note.

Usage: `python3 tools/format/check_lights.py FILE [FILE ...]` or
`--film DIR` (every manifest segment scene; segment paths are
repo-root-relative and resolved against DIR's repo root). The preset
vocabulary comes from `scenes/atmospheres.json` (override with
`--atmospheres`).

## The checks, with their rule citations

| # | Check | Rule |
|---|-------|------|
| 1 | Files parse; every number under `lights` / `post.grade` is finite (NaN / Infinity literals FAIL by path) | JSON edge case: Python's `json` accepts `NaN`/`Infinity`; the format never intends them |
| 2 | `post.grade` keys ⊆ the loader's set; unknown keys FAIL **by name** | §14.5 addendum: canonical keys **sat / warm / exposure / time**; "`saturation` is NOT a key" — the Wave-1 silent-ignore bug, now mechanical (the message even suggests `sat` for that exact key) |
| 3 | `post.grade.time` == 0 when present (absent = loader default 0, fine) | **R3** (§14.5): GRADE grain time pinned 0 for the whole film — the Wave-1 drift in 7 files |
| 4 | `atmosphere` / `shot.atmosphere` must be a string naming a preset (`dawn dusk fog golden night noon storm`); an inline rig object FAILs; a `filmLooks` key FAILs by name | **R5** (§14.5): files name PRESETS; the driver resolves set × preset; `filmLooks` is a derived validation index, "not the scene-file vocabulary" |
| 5 | Light rig: closed vocabulary per block (unknown keys FAIL by name — the loader reads only the §14.2 fields), intensities / `envIntensity` / `distance` / `decay` / `normalBias` ≥ 0, `shadowBox` / `far` > 0, `bias` finite-only (its default is negative), positions vec3-finite | §14.2 + `src/scenefile.rs` light parsing (KeyCfg/RimCfg/HemiCfg/PointCfg field reads) |
| 6 | Fire point basis — **warnings only** | **R1** (§14.5): the fire light records its rendered basis **4.6**; the constructor's 5.0 "is never rendered" |

### Two encoding decisions (where the brief's shorthand needed the source)

1. **Grade vocabulary.** The brief's set {sat, warm, exposure, time}
   is the ruling's *canonical* four, but `src/scenefile.rs`'s
   `GradeCfg` also consumes `vignette` and `grain` — every real scene
   carries them. Failing them would false-fail all 14 film scenes, so
   the gate's known set is the loader's six; the failure class is
   unchanged: any key the loader does not read is silently ignored,
   and that is what FAILs by name.
2. **Colour vs multiplier.** In this format a light colour is a
   **hex string** (`KeyCfg.color: String`, linearised by `hex_lin`),
   and the multiplier role belongs to the separate `intensity` field.
   So: hex strings are format-checked (#RGB / #RRGGBB / #RRGGBBAA);
   an *array* colour can only be raw components, and there the
   [0, 1] channel law applies — a channel outside [0, 1] FAILs. No
   range limit is placed on intensities beyond ≥ 0 (4.6, 2.6, … are
   multipliers, not channels).

### Warning-class rationale (check 6)

5.0-basis suspicion can never be proof, because a scene file may
legitimately record an *evaluated instant* of the fire channel —
S06's fire point carries `0.6333622932434082` with the fire dying and
no channel marker. So identification is tiered by confidence:

- **Tier A (explicit):** the point carries `channel.kind ==
  "fireLevel"` → the block declares itself the fire basis; any
  intensity ≠ 4.6 exactly warns (5.0 named as the constructor value).
- **Tier B (heuristic):** no marker, but colour `#FF9A3C` + distance
  7 + decay 1.6 + position above a declared pit (`shot.fire.pit` /
  `fire.pit`) → only intensity exactly 5.0 warns. Any other value is
  silent (the S06 case).

Warnings never change the exit code.

## Investigation: the one sweep finding (resolved as a tool bug, not data)

The first `--film` run FAILed check 4 on **s06**:

```
scenes/film/s06.scene.json: atmospherePreset is an inline rig object — R5: ...
```

Investigation (no scene edited): `atmospherePreset` is **not a
naming field**. In the film corpus it is a *record* of resolved
preset values (sky colours, cloudTint, tint), documented as such in
`scenes/film/ASSUMPTIONS.md` (lines 558, 670–673), sitting alongside
s06's proper declaration `atmosphere: "fog"` — which passes. No
`src/` code reads the field. Only s06 carries it. The check was
corrected to constrain the actual naming fields (`atmosphere`,
`shot.atmosphere`); the record field is deliberately unchecked, and
the docstring says why. This is the one place the tool's first cut
disagreed with the corpus — the corpus was right.

## Runs (verbatim outcomes)

Fixtures, run individually (exit codes observed per run):

| Fixture | Result |
|---|---|
| `fx-valid.scene.json` | `OVERALL: PASS (6/6 checks passed)`, exit 0 |
| `fx-grade-unknown-key.scene.json` | check 2 FAIL — `post.grade key 'saturation' is not read by the loader and is silently ignored (the canonical key is 'sat')`, exit 1 |
| `fx-grade-time.scene.json` | check 3 FAIL — `post.grade.time is 0.35 — R3 pins the GRADE grain time to 0`, exit 1 |
| `fx-atmosphere-inline.scene.json` | check 4 FAIL — `atmosphere is an inline rig object`, exit 1 |
| `fx-atmosphere-filmlook.scene.json` | check 4 FAIL ×2 — `atmosphere` and `shot.atmosphere` name filmLooks key `'jg-noon'`, exit 1 |
| `fx-light-negative.scene.json` | check 5 FAIL — `lights.key.intensity is -1.5 — must be >= 0`, exit 1 |
| `fx-light-nan.scene.json` | check 1 FAIL — `lights.rim.intensity is nan — not a finite number`, exit 1 |
| `fx-light-colour-range.scene.json` | check 5 FAIL — `lights.points[0].color[0] is 1.4 — outside [0, 1]`, exit 1 |
| `fx-fire-basis-warn.scene.json` | checks PASS, 1 tier-A warning (5.0 ≠ 4.6 on a `fireLevel` point), `OVERALL: PASS (6/6 checks passed, 1 warning(s))`, exit 0 |

Tier-B synthetics (temp copies of fx-valid, marker removed): at the
declared pit with intensity 5.0 → 1 tier-B warning, exit 0; with the
S06-style evaluated value 0.6333622932434082 → no warning, exit 0.

Negative controls: nonexistent file → `ERROR: check_lights.py: scene
file: cannot read ...`, exit 2; malformed JSON → one-line ERROR,
exit 2.

### The 14 real film scenes — verbatim

```
$ python3 tools/format/check_lights.py --film scenes/film
check_lights.py — 14 file(s)

[1 parse] PASS — files parse; numbers under lights / post.grade finite

[2 grade keys] PASS — post.grade keys subset of the loader's set (unknown named)

[3 grade time] PASS — R3: post.grade.time == 0

[4 atmosphere] PASS — R5: atmosphere declarations name presets, never inline rigs or filmLooks keys

[5 light rig] PASS — §14.2 light rig: closed vocabulary, sane ranges, hex colours

[6 fire basis] PASS — R1: fire point basis 4.6, not the constructor's 5.0 (warnings)

OVERALL: PASS (6/6 checks passed)
exit=0
```

Clean, as expected post-Wave-1: no `saturation` keys, every
`grade.time` is 0, every channel-marked fire point sits at exactly
4.6, and S06's evaluated fire value is correctly unflagged. No scene
was edited by this seat.

## tools-ci hookup shape (for the lead)

Two steps in `.github/workflows/tools-ci.yml`, mirroring the
existing "Film data validator (7/7)" step (plain `run:` steps; the
tool needs only python3 + the repo — no project tree, no
`--no-source`-style caveat):

```yaml
      - name: Light-rig validator (film scenes)
        run: python3 tools/format/check_lights.py --film scenes/film

      - name: Light-rig validator (fixtures — valid + warnings-only pass)
        run: |
          python3 tools/format/check_lights.py \
            scenes/fixtures/lights/fx-valid.scene.json \
            scenes/fixtures/lights/fx-fire-basis-warn.scene.json

      - name: Light-rig validator (negative controls — must FAIL by class)
        run: |
          for f in fx-grade-unknown-key fx-grade-time fx-atmosphere-inline \
                   fx-atmosphere-filmlook fx-light-negative fx-light-nan \
                   fx-light-colour-range; do
            if python3 tools/format/check_lights.py \
                 scenes/fixtures/lights/$f.scene.json; then
              echo "::error::$f unexpectedly PASSED"; exit 1
            fi
          done
```

The atmospheres vocabulary resolves from the repo root relative to
the tool's own path, so the steps work from any CWD.
