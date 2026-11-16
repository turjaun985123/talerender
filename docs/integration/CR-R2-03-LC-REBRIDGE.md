# R2-03 — Lighthouse Cat re-bridge (remediation Batch 2, critical path)

Seat: R2-03. Date: 2026-11-16. Binary: `~/workspace/talerender/target/release/talerender`
@ talerender HEAD `67fd9ef`. Live project NEVER edited (verified at close:
live `native/film.json` sha256 still `3f83fcb2d738…`, no `driver` field;
talerender tree clean). All work in this staging dir.

## VERDICT (read this first)

**The re-bridge itself is complete and correct: generic telemetry reaches
`FILM TELEMETRY: CHECK OK` (exit 0) on it** — all 157 samples, overflow
none, continuity clean, caption audit 46 chunks / min fit 1.00 / max
rendered width 809 px. **Validator: `RESULT: OK (0 flagged)`, no STALE.**

**One defect OUTSIDE the bridge blocks the canonical invocation:**
`src/filmtelemetry.rs` passes a descriptive label
(`"segment s03 scene at lt=0.09"`, filmtelemetry.rs:1042) — not the scene
path — to `scenefile::load_root`, which derives `scene_dir` from the
label's parent (scenefile.rs:1104). File textures therefore resolve
against the **process CWD**, not the scene directory, and the canonical
run (CWD = home) fails closed at s03. The film driver (filmseq.rs:582)
passes the real scene path and resolves correctly. **Lead fix (one line,
src core — not a seat edit):** in `filmtelemetry.rs` pass
`seg.scene_path.display().to_string()` as the `load_root` label, exactly
as filmseq does (or give `load_root` an explicit scene-dir parameter).
With that single variable neutralised (diagnostic run, CWD =
`<native>/scenes/film`), the identical bridge + binary produce CHECK OK.
Evidence for every clause is quoted verbatim in §6.

This defect is latent for ANY scene-sequence film using `file:` hold
plates — the repo's own Onoda scenes declare plates exactly this way
(`scenes/film/s07.scene.json`: `{"id": "coconut", "file":
"../../film/assets/coconut.png", …}`) — so it will bite the acceptance
proof regardless of this film until fixed in filmtelemetry.

## 1. What was wrong with the old bridge (all confirmed first-hand)

1. No `"driver"` field → legacy guard (exit 3, 17 anchors) / not generic.
2. Invented F2 data: `calendarYears ["1893","1904","1915","1924"]`,
   `calendarDark`, `sign` + 6 invented PNGs — the film has no calendar
   or sign in any scene.
3. All 43 caption `fit: 1.0` assumed (R1-10 F4).
4. 3 WIDTH-failing caption chunks (R1-10): idx 0 / 3 / 40.
5. **Hold plates undeclared in scene `textures[]`** (R1-02 F1): s03
   `lens-cloth`, s08 `oil-can` — `scenefile::plate_of` fails closed.
6. **NEW finding (this seat): transform op `rotateY`** (TS camelCase) in
   s06 + s10 fishing-boat groups — loader vocabulary is `rotate_y`
   (scenefile.rs `compose_transform`; scene-format.md:212). Value 0.4
   (radians) is correct; only the spelling is wrong. Masked until now
   because telemetry died at s03 first.
7. `BRIDGE-STATUS.txt` stale ("UNBRIDGED") although a bridge existed.

## 2. Plate audit (charter §3) — EVERY segment × referenced plates

Method: recursive walk of all 11 scene files (nodes + children + state),
every `holdL`/`holdR` `.plate` extracted; cross-checked with
`grep -rn '"plate"'` over the scenes dir (only 2 hits exist in the whole
film) and against `film/assets/plates.json` + PNG presence. No §15
clips/prefabs exist in these scenes (no `clips` keys), so the authored
state is the only hold source — the audit is exhaustive, not sampled.

| Scene | Character holds (state) | Plate referenced | In scene `textures[]` (OLD) | In `plates.json` + PNG in assets | Verdict OLD | Fix in NEW bridge |
|---|---|---|---|---|---|---|
| s01 | — none | — | — (sky, sand) | — | OK | none |
| s02 | — none | — | — (sky, sand) | — | OK | none |
| s03 | Biscuit `holdR` | `lens-cloth` (x0 −90, y0 −90) | **NO** (wall, floor only) | YES (`lens-cloth`, `lens-cloth.png`) | **FAIL — load_root error** | declared: `{"id":"lens-cloth","file":"../../film/assets/lens-cloth.png"}` |
| s04 | — none | — | — (sky, sand) | — | OK | none |
| s05 | — none | — | — (sky, sand) | — | OK | none |
| s06 | — none | — | — (sky, sand) | — | OK (but see §1.6: `rotateY`) | op respelled |
| s07 | — none | — | — (wall, floor) | — | OK | none |
| s08 | Biscuit `holdR` | `oil-can` (x0 −100, y0 −110) | **NO** (sky, sand only) | YES (`oil-can`, `oil-can.png`) | **FAIL — load_root error** | declared: `{"id":"oil-can","file":"../../film/assets/oil-can.png"}` |
| s09 | — none | — | — (sky, sand) | — | OK | none |
| s10 | — none (Biscuit + Fisher) | — | — (sky, sand) | — | OK (but `rotateY`) | op respelled |
| s11 | — none | — | — (sky, sand) | — | OK | none |

Declaration pattern = the repo's own (Onoda `coconut`, quoted in the
verdict). Post-fix proof: generic telemetry prints **no** `GENERIC
TELEMETRY NOTE … held-plate` line and `resolve_plate` finds both files
(unresolved count 0); `--scene-file` loads of s03/s08 pass texture
resolution (fail only later, at this VM's missing wgpu adapter).

Other scene-data audit (same pass): texture generators used = `solid`,
`sky-gradient` (both in the loader registry); geometry kinds incl.
`flat-quad` (fixed by the Batch-1 lead colour_geo landing); the ONLY
loader-vocabulary violation in all 11 scenes was `rotateY` ×2.

## 3. Caption re-chunk (R1-10 idx 0 / 3 / 40)

Measured with PIL/FreeType `getlength()` @ 88 px Inter-Black (the R1-10
measurer; font md5 `c26b4e77dc103fd3a1e3b426549296ed`). Splits at word
boundaries in `script.json` `caps` for lines 1 and 11 (1-based):

| Old idx | Old text | rawW / ink | Split into (new idx) | rawW each |
|---|---|---|---|---|
| 0 | `For thirty-one years,` | 917.9 / 922 | `For thirty-one` (0) + `years,` (1) | 616.4 / 284.0 |
| 3 | `the lighthouse burning.` | 1039.7 / 1049 | `the lighthouse` (4) + `burning.` (5) | 639.9 / 382.3 |
| 40 | `Biscuit finally slept,` | 882.6 / 887 | `Biscuit finally` (42) + `slept,` (43) | 609.1 / 256.0 |

Result: 43 → **46 chunks**; every chunk's measured fit = 1.0 (max rawW
now 798.5, `polished the great`); audit max **rendered (ink) width
809 px ≤ 850** (was 1049). New strips for the 6 split halves were
rendered with the `film/gen_assets.py` recipe (1080×300, 88 px, shadow
(543,159) α115, stroke 8 #0B0B10) — **verified byte-identical** against
an existing strip (`cap_001.png` "a cat named": sha256 `ec9b0e57…`
reproduced exactly) before use; the other 40 strips are the original
PNG bytes under their renumbered names. The re-chunked authoring source
ships as `script.json.rechunked` in this dir — **the lead must apply
the same two `caps` edits to the live project's script.json at swap
time**, or a future re-bridge reverts the splits.

## 4. Procedure (exact, reproducible by the lead)

```bash
STAGE=~/workspace/chrome-replacement/remediation/batch2/R2-03
LIVE=~/faceless-shorts/the-lighthouse-cats-last-shift

# (a) scratch project: builder/validator inputs copied from live;
#     script.json caps split per §3 (deliverable: script.json.rechunked)
mkdir -p $STAGE/work/project/{data,fonts,src,tools} $STAGE/work/scenes-fixed $STAGE/work/assets-src
cp $LIVE/script.json $STAGE/work/project/            # then apply the §3 caps splits
cp $LIVE/data/{lines.json,words.json,mix.wav} $STAGE/work/project/data/
cp $LIVE/fonts/Inter-Black.otf $STAGE/work/project/fonts/
cp $LIVE/src/shots.ts $STAGE/work/project/src/; cp $LIVE/tools/capture_native.mjs $STAGE/work/project/tools/

# (b) fixed scenes: copy live scenes; in s03 append textures[] entry
#     {"id":"lens-cloth","file":"../../film/assets/lens-cloth.png"};
#     in s08 the same for "oil-can"; in s06+s10 rename transform key
#     "rotateY" -> "rotate_y" (value 0.4 unchanged)
cp $LIVE/native/scenes/film/*.scene.json $STAGE/work/scenes-fixed/   # then the 4 edits above

# (c) stripped prior: ONLY anchors from the old film data (so the
#     builder carries anchors but cannot carry calendar/sign):
#     {"anchors": {Biscuit0, polished2, never4, dark5, thing6, time7, harbour9, slept10}}
#     -> $STAGE/work/prior-anchors-only.json

# (d) assets-src: for each of the 46 new chunks (builder's own
#     build_captions over the scratch project) copy the old strip PNG
#     when (text, s) matches an old chunk, else render with the
#     gen_assets.py recipe (byte-verified); copy lens-cloth.png +
#     oil-can.png from the old bridge assets.

# (e) build (shipped tooling, post-Batch-1 interface)
python3 ~/workspace/talerender/tools/bridge/build_bridge.py \
  --project $STAGE/work/project --scenes $STAGE/work/scenes-fixed \
  --assets-dir $STAGE/work/assets-src \
  --film-data $STAGE/work/prior-anchors-only.json \
  --out $STAGE/native --no-telemetry
# -> "built 62 files", "caption fit: MEASURED for 46 chunk(s)",
#    "verify (a) P4 adapter semantics: PASS", exit 0

# (f) hand steps (tool gaps, §7): restore manifest segment "set"
#     (BCH/RM, verbatim from the old manifest); write corrected
#     BRIDGE-STATUS.txt; regenerate MANIFEST-SHA over all files
#     (builder format: "<sha256>  <relpath>", sorted).

# (g) proofs
cp -r $STAGE/native $STAGE/work/project/native
python3 ~/workspace/talerender/tools/integration/validate_native_project.py $STAGE/work/project
~/workspace/talerender/target/release/talerender --film-telemetry $STAGE/native/film --tables   # canonical
cd $STAGE/native/scenes/film && ~/workspace/talerender/target/release/talerender \
  --film-telemetry $STAGE/native/film --tables                                                  # diagnostic
for f in $STAGE/native/scenes/film/*.scene.json; do
  ~/workspace/talerender/target/release/talerender --scene-file $f --out /tmp/probe.png; done   # load probes
```

## 5. Diff summary — old bridge vs new bridge

- **Files:** old-only: `calendar-1893/1904/1915/1924.png`,
  `calendar-dark.png`, `sign.png` (invented F2 assets — GONE),
  `README.md`, `film.json.example` (scaffold files, not bridge output).
  New-only: `cap_043/044/045.png` (46 strips vs 43), `BRIDGE-STATUS.txt`
  (corrected; old one said UNBRIDGED). Byte-identical: plate PNGs,
  `plates.json` (builder regenerated it byte-same), 7 of 11 scenes.
- **`film.json`:** + `"driver": "scene-sequence"`; + `audio` block
  (builder, from the project's real mix.wav); `captionChunks` shift —
  s01 4→6 chunks (0–5), s11 4→5 (41–45), all other segments same count,
  indices shifted; `bridge.captionFit = {mode: "measured", fontMd5:
  c26b4e77…, chunksRevisedVsFilmData: 0}` added; `bridge.skeletonOwes`
  now `[]` (no skeleton file survives to source the old 8 owes items —
  the R5 events gap is still named in `eventsGap`); `project` /
  `bridge.sourceProject` / `sourceScenes` record THIS staging build's
  paths (true provenance; lead may rewrite at swap + regen SHA).
- **`film/onoda.json`:** keys now exactly `anchors, captions, duration,
  fps, frames, lines` — `calendarYears` / `calendarDark` / `sign`
  REMOVED; anchors preserved verbatim (8); captions 43→46 per §3 with
  MEASURED fits (all 1.0 post-split; the old idx-3 chunk that truly
  measured 0.9233 no longer exists as a chunk).
- **Scenes (JSON deep-diff, only changes):** s03 `textures` 2→3
  (lens-cloth decl); s08 `textures` 2→3 (oil-can decl); s06 + s10
  `nodes[4].transform[1]`: `rotateY: 0.4` → `rotate_y: 0.4`. Nothing else.
- **Strips:** 40/46 byte-identical reuses (renumbered from the first
  split onward — a same-name file comparison shows all-differ, which is
  the renumbering, not new pixels); 6 newly rendered (§3).

## 6. Proof outputs (verbatim)

### 6a. Validator — final bridge, assembled project (exit 0)

```
validate-native: project /home/hatch/workspace/chrome-replacement/remediation/batch2/R2-03/work/project
  info: title "The Lighthouse Cat's Last Shift" - 30 fps - 1172 frames - 39.049s - 1080x1920 - 11 segments/shots - 11 lines - 46 caption strips referenced
RESULT: OK (0 flagged)
```

### 6b. Canonical telemetry — final bridge, CWD=/home/hatch (exit 3)

stderr (stdout empty):

```
film-telemetry: sample 20 (t=4.75)
film telemetry failed: filmtelemetry: segment s03 at lt=0.09: scene.textures[2]: cannot load texture file ../../film/assets/lens-cloth.png: No such file or directory (os error 2)
```

Attribution (code-read, not inferred): `filmtelemetry.rs:1042` builds
`label = format!("segment {} scene at lt={lt:.2}", seg.id)` and calls
`scenefile::load_root(root.clone(), &label)`; `scenefile.rs` load_root
does `let scene_dir = Path::new(path).parent()` on that label → `""` →
`AssetWire` joins the file path against CWD. The driver
(`filmseq.rs:582-583`) passes `seg.scene_path.display()` and is correct.
filmtelemetry's OWN walk uses the real `scene_dirs` (filmtelemetry.rs:
1006-1019), which is why its plate resolution succeeds once the gate is
passed. **This is the next named failure the charter asked me to report;
the bridge cannot fix it** (no single relative path satisfies both
bases, and an absolute path would break the CI proof's checkout path).

### 6c. Diagnostic telemetry — same binary, same bridge, CWD=`<native>/scenes/film` (exit 0)

Full stdout (stderr: progress lines only, last `sample 140 (t=34.75)`):

```
FILM TELEMETRY — The Lighthouse Cat's Last Shift (native, 0.25 s sampling)
CANVAS OVERFLOW: none
EDGE TOUCH (warning): s02 Biscuit crosses top (first at t=4.75s, 5 sampled frames)
EDGE TOUCH (warning): s02 Biscuit crosses bottom (first at t=7.75s, 6 sampled frames)
EDGE TOUCH (warning): s03 Biscuit crosses bottom (first at t=11.75s, 5 sampled frames)
EDGE TOUCH (warning): s04 Biscuit crosses right (first at t=15.25s, 1 sampled frames)
EDGE TOUCH (warning): s06 Biscuit crosses top (first at t=19.75s, 12 sampled frames)
EDGE TOUCH (warning): s06 Biscuit crosses left (first at t=19.75s, 15 sampled frames)
EDGE TOUCH (warning): s07 Biscuit crosses bottom (first at t=24.50s, 14 sampled frames)
EDGE TOUCH (warning): s08 Biscuit crosses top (first at t=28.00s, 2 sampled frames)
EDGE TOUCH (warning): s08 Biscuit crosses bottom (first at t=30.25s, 4 sampled frames)
EDGE TOUCH (warning): s10 Biscuit crosses top (first at t=32.75s, 6 sampled frames)
EDGE TOUCH (warning): s10 Biscuit crosses left (first at t=32.75s, 6 sampled frames)
EDGE TOUCH (warning): s10 Fisher crosses right (first at t=32.75s, 6 sampled frames)
CONTINUITY: clean (2 named characters tracked)
CAPTION ZONE (warning): s01 Biscuit face in caption zone (first at t=0.00s, 9 sampled frames)
FRAMING (warning): s01 Biscuit subject under 15% of frame height (7% of frame height) (first at t=0.00s, 12 sampled frames)
CAPTION ZONE (warning): s02 Biscuit face in caption zone (first at t=7.25s, 2 sampled frames)
SAFE AREA (warning): s02 Biscuit face centre in bottom UI band (first at t=7.75s, 6 sampled frames)
CAPTION ZONE (warning): s07 Biscuit face in caption zone (first at t=25.75s, 9 sampled frames)
CAPTION ZONE (warning): s08 Biscuit face in caption zone (first at t=30.00s, 1 sampled frames)
SAFE AREA (warning): s08 Biscuit face centre in bottom UI band (first at t=30.25s, 4 sampled frames)
FRAMING (warning): s10 Biscuit subject under 15% of frame height (11% of frame height) (first at t=32.75s, 6 sampled frames)
SAFE AREA (warning): s10 Fisher face centre under right rail (first at t=32.75s, 6 sampled frames)
FRAMING (warning): s11 Biscuit subject under 15% of frame height (14% of frame height) (first at t=36.00s, 13 sampled frames)
CAPTION ZONE (warning): s11 Biscuit face in caption zone (first at t=37.00s, 9 sampled frames)
COMPOSITION (advisory): s05 subject dead-centre for the whole shot — check the --guides sheet
COMPOSITION: 24 findings (caption-zone 5, safe-area 3, thirds 1, framing 3, edge-touch 12)
CAPTION AUDIT: 46 chunks, 0/11 lines authored, min fit 1.00, max rendered width 809 px
CAPTIONS: estimate-grade (auto-chunked) on lines 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11 — author caps in script.json for a master
[26 CAPTION TIME (advisory) lines — pre-existing display-rate advisories across many chunks, incl. un-split ones; advisory only (ruling R5), never counted; full text in work/telemetry-diag2-out.txt]
FILM TELEMETRY: CHECK OK
```

Tables after the verdict (shot/caption/cast) are in
`work/telemetry-diag2-out.txt` verbatim; highlights: shot table carries
the restored `set` values (BCH/RM) and both characters; caption table
max rend 809 (idx 13 `polished the great`); cast list Biscuit 11 shots,
Fisher 1 (s10). Warnings above are the audit's standing advisory
classes — none enters the verdict (contracts §3 / R1-02 §6).

### 6d. Scene load probes (driver-path loader, all 11 scenes)

Every scene: load succeeds; the only error is environmental, AFTER load:

```
s01..s11.scene.json: error: GPU initialisation failed (request_adapter (gpucore)): no wgpu adapter found — a Vulkan driver (hardware or lavapipe) is required   (exit 3, identical for all 11)
```

## 7. Tool gaps named (per charter)

1. **filmtelemetry `load_root` label (BLOCKER, lead/src):** §6b. One-line
   fix named there. Until it lands, the LC acceptance leg (b) cannot
   pass its `check` leg through the adapter (adapter spawns telemetry
   with inherited CWD — `check_mode.mjs:124`, no `cwd` option).
2. **build_bridge.py `set`:** segment `set` comes only from
   `--skeleton`; no skeleton artefact survives for LC (`/tmp/b301`
   gone), so the builder emitted no `set`. Restored by hand from the
   old manifest (filmseq defaults missing set to `""`, so this is
   fidelity, not function).
3. **build_bridge.py cannot re-chunk or render strips:** re-chunking is
   an authoring edit (script `caps`) and new strips need the
   `gen_assets.py` recipe — both done by hand here (§3/§4d);
   `gen_caption_strips.py` was NOT used for strips (its recipe renders
   at settled fit scale with stroke 15 — a different artefact from the
   film path's scale-1 / stroke-8 strips the binary composites).

## 8. Deliverables in this dir

- `native/` — the complete replacement bridge (63 files incl.
  BRIDGE-STATUS.txt + MANIFEST-SHA; film.json + film/ + scenes/).
- `script.json.rechunked` — live-project authoring patch source (§3).
- `NOTE.md` — this file. `work/` — scratch project, fixed scenes,
  assets-src, priors, and all raw outputs (`telemetry-*.txt`,
  `diff-summary.txt`, `new-captions.json`).
