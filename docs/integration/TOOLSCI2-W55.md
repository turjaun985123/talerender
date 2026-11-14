# TOOLSCI2-W55 — tools-ci wiring pack 2: strictness ratchet + native sheet suite

Seat W55, Wave 2 Batch 6. Deliverables, **new files only, nothing
committed, no existing file edited** (`.github/workflows/tools-ci.yml`
is lead-owned; the YAML below is paste-ready text):

- `tools/format/unknown_fields_baseline.json` — the ratchet
  baseline: all 15 unknown-key paths from STRICTNESS-W42.md §6,
  per path, with occurrence counts, grouped by that note's
  F3-a / F3-b / F3-c classes (174 + 19 + 3 = 196 occurrences).
- this note — the two paste-ready steps, the ratchet's exact
  comparison logic, the trigger-path audit, and the clean-tree
  verification transcripts.

**Headline: the clean-clone verification exposed NO committed-tree
gap.** Both steps were simulated end-to-end against
`git archive HEAD` (HEAD `aa92cee`) — committed files only — and
every assertion passed, including both negative controls. The two
earlier waves of "passes locally, fails on the runner" (absolute
manifest paths at `630d82e`; git-ignored fixture PNGs at
`617ea4e`) do not recur here: the strictness corpus reproduces
W42's numbers exactly from the archive, and all 90 sheet fixture
frames are present in the committed tree (the `865c471`
`.gitignore` negations cover them).

## 1. How the lead lands this pack

1. Commit `tools/format/unknown_fields_baseline.json` (this pack)
   — the ratchet step reads it from the checkout.
2. Append the three steps in §2 to `.github/workflows/tools-ci.yml`,
   at the end of the `steps:` list (after the WGSL step). The
   steps are independent of the existing ones and of each other;
   placement is free. (SHEET-W44 suggested its step sit after the
   film validator — equivalent, since no step consumes another's
   output.)
3. No trigger-path changes (§4). No other file changes.

## 2. Paste-ready YAML

In the file's existing style: explanatory comment block, then the
step, appended at the end of `steps:` with the file's indentation
(6 spaces for `- name:`, 8 for `run:`).

### Step A1 — strictness fixtures (the checker must bite)

```yaml
      # W42's strictness checker proves itself before it is
      # trusted with the corpus: the clean fixture exercises the
      # full loader surface and must pass with ZERO tolerated
      # occurrences (the known sets are the loader's, not the
      # corpus's habits); the allowlisted fixture passes on its
      # citations; and every planted unknown must FAIL with its
      # exact path named — a checker that stops biting turns
      # this red (same negative-control discipline as the
      # light-rig and asset fixture steps above).
      - name: Unknown-field strictness (fixtures — clean passes, planted fails named)
        run: |
          python3 tools/format/check_unknown_fields.py tools/format/fixtures/unknown_fields/fx-clean.scene.json
          python3 tools/format/check_unknown_fields.py tools/format/fixtures/unknown_fields/fx-allowlisted.scene.json
          if python3 tools/format/check_unknown_fields.py tools/format/fixtures/unknown_fields/fx-unknown.scene.json > "$RUNNER_TEMP/uf-unknown.log" 2>&1; then
            echo "::error::fx-unknown unexpectedly PASSED"; exit 1
          fi
          cat "$RUNNER_TEMP/uf-unknown.log"
          grep -F "first at $.bogus" "$RUNNER_TEMP/uf-unknown.log"
          grep -F "first at $.nodes[0].bogosity" "$RUNNER_TEMP/uf-unknown.log"
          grep -F "first at $.nodes[0].material.shininess" "$RUNNER_TEMP/uf-unknown.log"
          if python3 tools/format/check_unknown_fields.py tools/format/fixtures/unknown_fields/fx-film-data.json > "$RUNNER_TEMP/uf-film-data.log" 2>&1; then
            echo "::error::fx-film-data unexpectedly PASSED"; exit 1
          fi
          cat "$RUNNER_TEMP/uf-film-data.log"
          grep -F "first at $.studio" "$RUNNER_TEMP/uf-film-data.log"
          grep -F "first at $.captions[0].bogus" "$RUNNER_TEMP/uf-film-data.log"
          if python3 tools/format/check_unknown_fields.py tools/format/fixtures/unknown_fields/fx-manifest.json > "$RUNNER_TEMP/uf-manifest.log" 2>&1; then
            echo "::error::fx-manifest unexpectedly PASSED"; exit 1
          fi
          cat "$RUNNER_TEMP/uf-manifest.log"
          grep -F "first at $.segments[0].bogus" "$RUNNER_TEMP/uf-manifest.log"
```

### Step A2 — the ratchet over the real corpus

```yaml
      # Report-only + ratchet, per STRICTNESS-W42 section 7
      # Option A: the corpus legitimately carries 196 unknown-key
      # occurrences in 15 paths today (the F3-a/b/c register —
      # dead generator provenance, uncited note fields, bookshop
      # residue), so the checker itself can never go green here
      # and a hard gate would be red forever. Instead the
      # baseline (tools/format/unknown_fields_baseline.json,
      # transcribed from STRICTNESS-W42.md section 6) pins the
      # register, and this step fails only on GROWTH: a path not
      # in the baseline, or a baselined path whose occurrence
      # count increased. Shrinkage passes and prints the lowered
      # register in the baseline's own shape, so a data fix
      # lowers the baseline in the same PR. The checker's exit 2
      # (input error — e.g. a corpus file deleted or unreadable)
      # fails the step outright: a missing file is never read as
      # shrinkage. The corpus is W42's 27-file command verbatim,
      # globs unexpanded in the YAML so new scene files join the
      # corpus automatically (fixtures stay excluded — the globs
      # are single-level, scenes/fixtures/ is not matched).
      # Tolerated occurrences are reported by the checker but are
      # NOT gated: growth in the cited dialect is legal.
      - name: Unknown-field ratchet (real corpus vs baseline — growth fails)
        run: |
          LOG="$RUNNER_TEMP/unknown-fields.log"
          set +e
          python3 tools/format/check_unknown_fields.py \
            scenes/*.scene.json scenes/film/*.scene.json \
            scenes/film/film.json scenes/film/s07-calendar.json \
            scenes/sets/*.scene.json scenes/atmospheres.json \
            film/onoda.json film/assets/plates.json > "$LOG" 2>&1
          RC=$?
          set -e
          cat "$LOG"
          if [ "$RC" -eq 2 ]; then
            echo "::error::check_unknown_fields input error (exit 2) — a corpus file is missing or unreadable; the ratchet never treats that as shrinkage"
            exit 1
          fi
          python3 - "$LOG" <<'EOF'
          import json, re, sys

          log_path = sys.argv[1]
          text = open(log_path, encoding="utf-8").read()
          line_re = re.compile(
              r"^    (\S+): (\S+) \u2014 unknown key '([^']+)' in (\S+) object "
              r"\((\d+) occurrence\(s\); first at ", re.M)
          run = {}
          for f, pattern, key, ctx, n in line_re.findall(text):
              run[(f, pattern, key, ctx)] = int(n)

          base = json.load(open("tools/format/unknown_fields_baseline.json",
                                encoding="utf-8"))
          bmap = {}
          for p in base["paths"]:
              bmap[(p["file"], p["pattern"], p["key"], p["context"])] = \
                  p["occurrences"]

          failures = []
          for k in sorted(run):
              if k not in bmap:
                  failures.append(
                      "NEW unknown-key path (not in baseline): %s %s key '%s' "
                      "(%s) x%d" % (k[0], k[1], k[2], k[3], run[k]))
              elif run[k] > bmap[k]:
                  failures.append(
                      "GREW past baseline: %s %s key '%s' (%s) %d -> %d"
                      % (k[0], k[1], k[2], k[3], bmap[k], run[k]))
          total = sum(run.values())
          if total > base["totalOccurrences"]:
              failures.append("TOTAL unknown occurrences grew: %d -> %d"
                              % (base["totalOccurrences"], total))

          if failures:
              for f in failures:
                  print("::error::" + f)
              print("RATCHET: FAIL - %d growth finding(s); baseline is "
                    "tools/format/unknown_fields_baseline.json "
                    "(STRICTNESS-W42.md section 6)" % len(failures))
              sys.exit(1)

          shrunk = []
          for k in sorted(bmap):
              if k not in run:
                  shrunk.append("RESOLVED: %s %s key '%s' (%s) was x%d, now "
                                "absent" % (k[0], k[1], k[2], k[3], bmap[k]))
              elif run[k] < bmap[k]:
                  shrunk.append("DECREASED: %s %s key '%s' (%s) %d -> %d"
                                % (k[0], k[1], k[2], k[3], bmap[k], run[k]))
          for s in shrunk:
              print(s)
          print("RATCHET: PASS - %d unknown-key occurrence(s) in %d distinct "
                "path(s), baseline %d in %d; no growth"
                % (total, len(run), base["totalOccurrences"], len(bmap)))
          if shrunk:
              print("The register shrank - lower the baseline in this PR by "
                    "saving the register below over "
                    "tools/format/unknown_fields_baseline.json:")
              by_key = {}
              for p in base["paths"]:
                  by_key[(p["file"], p["pattern"], p["key"],
                          p["context"])] = p
              new = dict(base)
              new["paths"] = []
              for k in sorted(run):
                  entry = dict(by_key[k])
                  entry["occurrences"] = run[k]
                  new["paths"].append(entry)
              new["totalOccurrences"] = total
              new["distinctPaths"] = len(run)
              print(json.dumps(new, indent=2, sort_keys=True))
          EOF
```

Notes on the comparator (the precise comparison logic):

- A run finding is keyed by the 4-tuple **(file, path pattern,
  key, context)** parsed from the checker's finding lines — the
  same aggregation the checker itself prints (`[]` folds array
  indices; counts are occurrence counts). The baseline stores
  exactly these 4-tuples.
- **FAIL** iff any of: (a) a run 4-tuple absent from the baseline
  (NEW path); (b) a run count greater than its baselined count
  (GREW); (c) the run total greater than `totalOccurrences`
  (implied by (a)+(b); kept as a belt so a parser drift that
  somehow dropped lines cannot pass silently against a higher
  total — and conversely if the checker ever prints *fewer*
  finding lines than its OVERALL claims, the parsed total drops
  and the step reports shrinkage, which a human reviewing the
  lowered-baseline paste would see; the OVERALL line is `cat`ed
  into the log above the verdict for exactly this cross-check).
- **PASS** otherwise. Each baselined path that vanished prints
  `RESOLVED`, each that decreased prints `DECREASED`, and if any
  shrinkage occurred the step prints the complete new register as
  JSON in the baseline file's shape (same keys), ready to save
  over `tools/format/unknown_fields_baseline.json` in the same
  PR. That paste is the only sanctioned way to lower the
  baseline. Raising a count, or adding a path, is a deliberate
  act: edit the baseline AND update the STRICTNESS-W42 register
  citation in the same PR, or the next reader cannot tell growth
  from dialect.
- Corpus membership is the step's glob set, evaluated by the
  shell at run time: adding a scene file under `scenes/`,
  `scenes/film/` or `scenes/sets/` adds it to the corpus (a new
  file's unknown keys fail as NEW paths — the intended bite);
  deleting or renaming a corpus *file* breaks a glob or an
  explicit path, the checker exits 2, and the step fails before
  any comparison. Renaming a file while keeping its keys moves
  its paths (old RESOLVED + new NEW → FAIL): rename the baseline
  entries in the same PR.

### Step B — native sheet suite

```yaml
      # W44's native contact-sheet tool verifies itself: the
      # suite tiles the 90 synthetic fixture frames (all committed
      # — tools/film/fixtures/sheet/frames/, covered by the
      # .gitignore fixture negations) and checks dimensions via
      # ffprobe plus tile order/identity via an ffmpeg rawvideo
      # decode; its negative cases are inside the suite (a sampled
      # frame removed must fail BY NAME with no sheet left; ffmpeg
      # absent must exit 2). Exit 0 from run_tests.py IS the
      # 10/10 verdict. Needs ffmpeg + ffprobe on the runner (the
      # ubuntu-22.04 hosted image ships both; if a future image
      # drops them this step fails loudly inside case A/D, never
      # silently). ~9 s on the dev VM.
      - name: Native contact sheet suite (10/10)
        run: python3 tools/film/fixtures/sheet/run_tests.py
```

## 3. Trigger paths — no additions needed

Both `push` and `pull_request` already list `tools/**`,
`scenes/**`, `src/**` and `film/**`. Everything the new steps
read is inside those trees:

| step | reads | covered by |
|---|---|---|
| A1 fixtures | `tools/format/check_unknown_fields.py`, `tools/format/fixtures/unknown_fields/*` | `tools/**` |
| A2 ratchet | the checker + `tools/format/unknown_fields_baseline.json`; corpus under `scenes/**`, `scenes/film/film.json`, `film/onoda.json`, `film/assets/plates.json` | `tools/**`, `scenes/**`, `film/**` |
| B sheet | `tools/film/make_sheet.py`, `tools/film/fixtures/sheet/**` | `tools/**` |

The checker also *transcribes* the loaders in `src/scenefile.rs`
/ `src/film.rs`; `src/**` is already a trigger, so a loader change
that adds a field re-runs the ratchet (and fails as NEW/GREW
until the checker's sets and, if the corpus uses it, the
baseline catch up — the intended friction).

## 4. Verification — clean tree only, all first-hand

Method (the tools-ci incident's lesson, applied literally): the
working tree was never used. `git archive HEAD` (HEAD `aa92cee`)
was extracted to a scratch dir; the ONLY file added was this
pack's `unknown_fields_baseline.json`, i.e. the simulation runs
the tree exactly as it will exist once the lead lands the pack.
Mutation controls ran in subtree copies of that extraction
(scenes/ + film/ + tools/ — the extracted repo is 164 MB, most
of it `docs/` golden frames the controls don't touch).
Environment: CPython 3.12.3 (the workflow pins 3.12), ffmpeg /
ffprobe 8.1.2 on this VM. Two simulation concessions, neither
load-bearing: `$RUNNER_TEMP` was a plain /tmp path locally, and
the comparator ran as a file whose content is byte-identical to
the §2 heredoc (its argument is the log path in both forms).

### A1 — fixtures (assertion-relevant lines, verbatim)

```
$ python3 tools/format/check_unknown_fields.py tools/format/fixtures/unknown_fields/fx-clean.scene.json
OVERALL: PASS (5/5 checks passed, 0 unknown-key occurrence(s) in 0 distinct path(s), 0 tolerated occurrence(s))   rc=0
$ python3 tools/format/check_unknown_fields.py tools/format/fixtures/unknown_fields/fx-allowlisted.scene.json
OVERALL: PASS (5/5 checks passed, 0 unknown-key occurrence(s) in 0 distinct path(s), 9 tolerated occurrence(s))   rc=0
$ # fx-unknown.scene.json — fails as expected; every plant named:
    .../fx-unknown.scene.json: $ — unknown key 'bogus' in scene-root object (1 occurrence(s); first at $.bogus)
    .../fx-unknown.scene.json: $.nodes[] — unknown key 'bogosity' in node object (1 occurrence(s); first at $.nodes[0].bogosity)
    .../fx-unknown.scene.json: $.nodes[].material — unknown key 'shininess' in material object (1 occurrence(s); first at $.nodes[0].material.shininess)
$ # fx-film-data.json — fails as expected:
    .../fx-film-data.json: $ — unknown key 'studio' in film-root object (1 occurrence(s); first at $.studio)
    .../fx-film-data.json: $.captions[] — unknown key 'bogus' in film-caption object (1 occurrence(s); first at $.captions[0].bogus)
$ # fx-manifest.json — fails as expected:
    .../fx-manifest.json: $.segments[] — unknown key 'bogus' in manifest-segment object (1 occurrence(s); first at $.segments[0].bogus)
STEP-A-FIXTURES: ALL ASSERTIONS PASSED
```

### A2 — corpus + ratchet at baseline (must pass, exactly equal)

```
$ python3 tools/format/check_unknown_fields.py \
    scenes/*.scene.json scenes/film/*.scene.json \
    scenes/film/film.json scenes/film/s07-calendar.json \
    scenes/sets/*.scene.json scenes/atmospheres.json \
    film/onoda.json film/assets/plates.json
check_unknown_fields.py — 27 file(s)
OVERALL: FAIL (3/5 checks passed, 196 unknown-key occurrence(s) in 15 distinct path(s), 7043 tolerated occurrence(s))   checker rc=1
$ python3 <comparator> unknown-fields.log
RATCHET: PASS - 196 unknown-key occurrence(s) in 15 distinct path(s), baseline 196 in 15; no growth   rc=0
```

The committed tree reproduces STRICTNESS-W42.md §6 to the
occurrence: 27 files, 196 / 15 / 7043. The checker's own verdict
is FAIL (the register is real); the ratchet's is PASS (no
growth) — that difference is the design.

### A2 — negative controls (must fail, both growth shapes)

Planted in the subtree copy: a brand-new root key
`bogusNewKey: 123` in `scenes/film/s01.scene.json`, and one
extra texture `note` in `scenes/film/s10.scene.json` (an
existing baselined path, 2 → 3):

```
OVERALL: FAIL (3/5 checks passed, 198 unknown-key occurrence(s) in 16 distinct path(s), 7043 tolerated occurrence(s))
::error::NEW unknown-key path (not in baseline): scenes/film/s01.scene.json $ key 'bogusNewKey' (scene-root) x1
::error::GREW past baseline: scenes/film/s10.scene.json $.textures[] key 'note' (texture) 2 -> 3
::error::TOTAL unknown occurrences grew: 196 -> 198
RATCHET: FAIL - 3 growth finding(s); baseline is tools/format/unknown_fields_baseline.json (STRICTNESS-W42.md section 6)   rc=1
```

### A2 — shrinkage control (must pass and print the lowered register)

Plants reverted; the dead F3-a key `fps` deleted from
`film/onoda.json`'s root:

```
OVERALL: FAIL (3/5 checks passed, 195 unknown-key occurrence(s) in 14 distinct path(s), 7043 tolerated occurrence(s))
RESOLVED: film/onoda.json $ key 'fps' (film-root) was x1, now absent
RATCHET: PASS - 195 unknown-key occurrence(s) in 14 distinct path(s), baseline 196 in 15; no growth   rc=0
(the full lowered register follows in the step output; its totals:)
  "distinctPaths": 14,
  "totalOccurrences": 195
```

### A2 — exit-2 control (a missing corpus file is not shrinkage)

`film/assets/plates.json` deleted from the copy; the step's own
shell block run verbatim:

```
ERROR: check_unknown_fields.py: input file: cannot read film/assets/plates.json ([Errno 2] No such file or directory: 'film/assets/plates.json')   checker rc=2
::error::check_unknown_fields input error (exit 2) - a corpus file is missing or unreadable
STEP RESULT: FAIL (as designed)
```

### B — sheet suite in the clean tree (verbatim)

```
$ python3 tools/film/fixtures/sheet/run_tests.py
PASS  A exit 0  (rc=0)
PASS  A dims 286x96  (got (286, 96))
PASS  A tiles are frames 15/30/45/60/75 in order  (centres R=[14, 29, 43, 59, 73])
PASS  A success line
PASS  B defaults exit 0, dims 1976x384  (rc=0, dims=(1976, 384))
PASS  C missing frame exit 1  (rc=1)
PASS  C frame named
PASS  C no sheet left
PASS  D no-ffmpeg exit 2 + ERROR line  (rc=2)
PASS  E window frames 30/45 only, dims 112x96  (rc=0, dims=(112, 96))
OVERALL: PASS (10/10 checks)   suite rc=0   (~9 s)
```

### B — standalone negative control (the committed tool, not the suite)

A copy of the committed fixture frames with `frame_00030.png`
removed, sampled at step 1 (which samples frames 30/60) via
`make_sheet.py` directly:

```
sheet: /tmp/w55-sheet-neg — 89 frames present (0..89), duration 3.00 s
MISSING frame_00030.png (sample at t=1.00 s)
SHEET: FAILED (1 sampled frame(s) missing — no sheet written)   tool rc=1
ls: cannot access '/tmp/w55-sheet-neg.jpg': No such file or directory
```

## 5. Residual notes for the lead

- The ratchet's honesty rests on the checker's finding-line
  format; if `check_unknown_fields.py` ever changes that format,
  the comparator parses zero findings and reports total
  shrinkage (a PASS with a suspicious register dump). The §2
  comment block names the cross-check (the OVERALL line is in
  the same log, one screen above the verdict). A format change
  to the checker should update this comparator in the same PR.
- Step B's only environmental assumption is ffmpeg + ffprobe on
  PATH. The hosted ubuntu-22.04 image ships both; the suite was
  verified under this VM's ffmpeg 8.1.2, and its tolerances
  (JPEG ±28 on centre pixels) are the same ones SHEET-W44
  recorded. If the runner image ever drops ffmpeg, case A fails
  and case D's expectation inverts — loud either way.
- Nothing in this pack touches the loaders, the checker, or the
  sheet tool. F3's data dispositions (F3-a generator fix, F3-b
  citations, F3-c decision) remain the fixes phase's work; when
  they land, this ratchet is what proves the register only ever
  shrinks.
