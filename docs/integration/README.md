# Production integration — staging to native film

How a Tall Tale project becomes a complete native film in
TaleRender, proven end-to-end on **"THE SOLDIER WHO DIDN'T KNOW
THE WAR WAS OVER"** (the Onoda project, 60.357 s, 12 shots /
14 segments, 1,811 frames at 1080×1920 @ 30 fps).

Decision recorded first: contract §5 is **RESOLVED (B)** — native
frames stay **clean** (no deliberate q0.93 JPEG generation on
native masters). Native output paths carry clean PNG frames into
finishing; the sheet/review JPEGs (0.8/0.92) remain preview-grade
only, exactly as in the original pipeline.

## The bridge

```
project staging (build/src: shots.ts / world.ts / cast.ts)
        │  read once, formulas transcribed under the f32 data law
        ├─► film/onoda.json        (film/gen_film_data.py)
        │     timing, word anchors, caption chunks — generated
        │     from the project's lines.json / words.json /
        │     script.json by the same rules as timing.ts /
        │     captions.ts (chunking, fit, line starts, cuts)
        ├─► film/assets/           (film/gen_assets.py)
        │     held-prop plates, caption strips, calendar plates,
        │     the bookshop sign — rasterised once with PIL from
        │     the staging's own drawing recipes
        └─► src/film.rs            (the evaluator)
              shot functions s01..s12 mirroring shots.ts shot by
              shot; set builders for jungle / shore / bookshop;
              the look registry; motion formulas ported from
              motion.ts (poseTrack, camK, blinkDirector, settle,
              walk phase/bob) — frame(t) is a pure function
```

Plus scene format v2 (below) for the declarative side: the same
S01 staging expressed as data in `scenes/character-v2.scene.json`.

## Driver CLI

```
talerender --film <film-dir> --out <frames-dir> [--from N] [--to M]
    Render frames [N, M) of the film (frame n is t = n/30 s).
    Resume: frames already present in the output dir are skipped,
    so a chunked/interrupted run loses nothing. frame(t) is pure
    and chunking does not change any frame — proven by
    re-rendering overlap ranges in independent runs and
    byte-comparing (probes 195–204, 795–814, 1195–1204 all
    byte-identical at the current head). One historical caveat,
    fixed: the character paint cache originally keyed states at
    2-dp rounding, which made five frames at one boundary depend
    on invocation history (72.34 dB, ~1-code differences); keys
    are now exact f32 bit patterns (commit cabcc30) and the
    failing probe passes 10/10.

talerender --film-telemetry <film-dir>
    The M6 audit semantics over the whole film: 0.25 s sampling,
    per-character cutouts from the real painted canvases,
    overflow / edge / continuity / composition derivations, and
    the strict caption audit. Prints
    `FILM TELEMETRY: CHECK OK` or itemises every failure.
    No GPU needed.

film/finish.sh FRAMES_DIR MIX_WAV OUT_MP4
    The pipeline's finishing settings: x264 slow CRF 18, BT.709
    tags, the project's existing data/mix.wav muxed as AAC,
    faststart, .part discipline.
```

## Format v2 (docs/scene-format.md §14)

Version field accepts 1.0 and 2.0; v1 files carrying v2 fields
are rejected. Additions: `time`, camera `keys` (smootherstep
interpolation), a `lights` block (key/rim/hemisphere/point rigs
with shadow flags and three.js HemisphereLight/PointLight field
conventions, defaults matching `lightRig()`), and a `character`
node (full drawToon look + state) replacing the M2-era code-ref
nodes for character and rifle.

## Proof (this seat) — film at f021991; frames rendered in CI (lavapipe, Mesa 23.2)

Renderer of record for every native image in this proof:
**lavapipe (software Vulkan) on GitHub's ubuntu-22.04 runners** —
labelled as such; no performance claims are made from it.

- **Inventory outcome:** 14/14 segments (12 shots; S05 and S08
  split) native-rendered end to end; nothing blocked, no
  substitutions. 1,811 frames at 1080×1920 @ 30 fps, clean PNG
  (decision (B): no deliberate JPEG generation on the masters).
- **Telemetry:** `FILM TELEMETRY: CHECK FAILED (0 overflow
  groups, 0 continuity violations, 7 caption failures)` — the
  geometry audit is clean across all 242 samples (independently
  reproduced by wave seat W85 with the same verdict line and
  identical failure list). The 7 caption-width failures are
  pre-existing authoring-bar violations in the project's own
  chunks, verified present in the Chrome original (worst chunk
  "and Onoda surrendered": Chrome renders it at 953 px ink,
  fit-capped from a ~1050 px natural width that the native PIL
  measurement reproduces within ~1%). They are reproduced
  faithfully, not introduced natively; the lead's ruling is
  ACCEPT WITH WAIVER — re-chunking would desync the native film
  from the original it must match.
- **A/B vs the project's Chrome review frames** (60 pairs, one
  per second): **mean 23.02 dB, min 19.36**. Per-shot means:
  S01 25.03, S02 22.83, S03 21.00, S04 23.15, S05a 23.24,
  S05b 19.36 (n=1), S06 25.46, S07 22.38, S08 23.42, S08b 22.04,
  S09 21.89, S10 21.82, S11 21.91, S12 24.49. Full table +
  side-by-sides: film/ab-out4/. The residual classes are the
  declared ones: the Chrome side's q-JPEG floor (the one
  PNG-vs-PNG controlled pair, ab1 t=1.50, measures 23.96 dB),
  defocused-foliage bokeh, and IBL-sensitive surfaces. Fix
  history of the headline: 21.44 (first full render) → 21.84
  (WAVE-RECORD §5 staging fixes) → 22.43 (sky-dome shader +
  atmosphere re-dressing) → 23.02 (paint-cache exact keys,
  S06 fog dome, golden-state inheritance into S08–S09).
- **Determinism:** overlap ranges re-rendered in independent
  CI runs and byte-compared: v1 range 195–204, 10/10 identical;
  v4 probes 1195–1204 (10/10) and 795–814 (20/20) identical —
  including frames 1195–1199, which diverged at 72.34 dB in
  two earlier fleets before the paint-cache key fix (see the
  resume note in the Driver section).
- **MP4:** `onoda-native.mp4` from the v4 frames via
  film/finish.sh: ffprobe shows h264 1080×1920 @ 30 fps,
  color_space/color_transfer/color_primaries all **bt709**
  (the x264 VUI params added after the wave's MP4 evidence
  found primaries/transfer landing as "unknown"), AAC audio,
  duration 60.457 s, 29,689,829 bytes, moov atom before mdat
  (faststart). Measured loudness of the mux: **−14.00 LUFS
  integrated, −1.58 dBTP** — the project's own mix, unchanged,
  matching its −14.0 LUFS master.
- **Format v2 proof:** scenes/character-v2.scene.json renders
  through the loader + composer (8 dark pixels — card
  transparency correct); the analytic fog chart passes ≤1 sRGB
  code at every panel (bar ±2); fx-textures / fx-fog fixtures
  render through the live loader; the loader's full texture
  registry (13 generators), post.grade.exposure and the fog
  block are all consumed (Batch-8 acceptance census).

## Honest remaining limits

- **Caption text measurement enters as data.** Caption chunk
  widths (and therefore `fit`) were measured with PIL at asset
  generation, not Skia. PIL metrics differ slightly from the
  browser's; only an over-wide chunk's fit factor can differ.
  The film's caption strips are the PIL-rendered assets
  themselves, so what is measured is what is shown.
- **Calendar + captions + film fade are CPU-composited** after
  readback (they are 2D canvas overlays in the staging too).
  The calendar is composited with the exact inverse-map affine
  the staging's drawImage transform implies.
- **Fog** is implemented in the PBR/basic shaders as an append
  (three.js Fog smoothstep over view depth); the ab1 proof path
  keeps fog disabled bit-exactly. The film path's fog is
  validated by the film's S06 (fog set) A/B frames; the LOADER
  path (scenefile) also consumes the v2 `fog` block — a pre-bokeh
  pass over the packed-depth prepass — and is validated by an
  analytic fog chart (all 10 panels within 1 sRGB code of the
  expected smoothstep values, bar ±2). The loader path treats
  cleared-depth (sky dome) pixels as unfogged; the film path
  keeps the source's per-draw fog flags (the dome is fog:false
  in the source and here).
- **The sky dome is the source's dome SHADER sampled into a
  texture** (per-row: `mix(hor, top, pow(clamp(y·1.6,0,1), 0.7))`,
  flat ground colour below the horizon; y = cos(π(1−v)) on the
  UV sphere). The shader is constant across each row, so the
  sampling is exact up to row quantisation (512 rows).
  Atmosphere re-dressing (env.ts applyAtmosphere step 2) is
  modelled as dome/cloud variants per shot — INCLUDING its
  statefulness: noon passes `sky: false` and restores neither
  the dome, the cloud tint, nor the key-light position, so
  S08/S08b/S09 inherit S07's golden dome, cloud tint and key
  position in the Chrome original and here.
- **Held props ignore their staging angle argument** — verified
  in the source: every prop the film holds is drawn unrotated;
  the plates are therefore composited by translation only.
- **BK floor texture rotation** (π/2 map rotation in three.js)
  is emulated with a swapped repeat. (The BK shelf unit —
  uprights, row boards, book advance bw+0.012, gap predicate —
  is transcribed source-exactly; an earlier digest's gap value
  was superseded by the WAVE-RECORD D11 fix.)
- **RADV fp16 unverified** (carried from M3): the bokeh depth
  path defaults to Rgba8Unorm storage; `TALERENDER_DOF_DEPTH_FORMAT=halffloat`
  restores HalfFloat where a driver is proven.
- The film evaluator (src/film.rs) duplicates ab1's pipeline
  setup by design (repo precedent) so M3–M6 proof paths carry
  zero regression risk from the film work.

## Runbook — the user's PC (RADV)

1. Download the `talerender-binary` artifact from the latest
   green CI run of turjaun985123/talerender (or build with the
   same workflow). The binary is identical; only the Vulkan
   driver changes: RADV on the Vega 7 replaces lavapipe.
2. `talerender --film film --out frames/` (chunk with
   --from/--to if desired; resume is automatic).
3. `talerender --film-telemetry film` — for THIS film expect
   `FILM TELEMETRY: CHECK FAILED (0 overflow groups, 0 continuity
   violations, 7 caption failures)`: the geometry audit is clean
   and the 7 caption failures are pre-existing authoring-bar
   violations in the project's own chunks, present identically
   in the Chrome original (see Proof).
4. `film/finish.sh frames <project>/data/mix.wav onoda-native.mp4`.
5. Verify: `ffprobe onoda-native.mp4` (1080×1920, 30 fps, x264,
   AAC, ~60.46 s) and loudness ≈ −14.0 LUFS (the mix is the
   project's own master).
