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
    so a chunked/interrupted run loses nothing; chunking never
    changes a frame (frame(t) is pure — proven by re-rendering a
    chunk boundary twice and byte-comparing).

talerender --film-telemetry <film-dir>
    The M6 audit semantics over the whole film: 0.25 s sampling,
    per-character cutouts from the real painted canvases,
    overflow / edge / continuity / composition derivations, and
    the strict caption audit. Prints CHECK OK or itemises every
    failure. No GPU needed.

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

## Proof (this seat)

TBD-PROOF: telemetry verdict; per-shot A/B PSNR table headline;
MP4 ffprobe + loudness; determinism byte-compare; format v2
proof render. Renderer of record for every image on this VM:
**llvmpipe/lavapipe (software Vulkan)** — labelled as such;
no performance claims are made from it.

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
  keeps fog disabled bit-exactly, and the fog path itself is
  validated by the film's S06 (fog set) A/B frames, not by a
  separate gate.
- **Held props ignore their staging angle argument** — verified
  in the source: every prop the film holds is drawn unrotated;
  the plates are therefore composited by translation only.
- **BK floor texture rotation** (π/2 map rotation in three.js)
  is emulated with a swapped repeat; shelf book spacing uses the
  transcribed hash layout with a 0.005 gap the digest did not
  record.
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
3. `talerender --film-telemetry film` — expect CHECK OK.
4. `film/finish.sh frames <project>/data/mix.wav onoda-native.mp4`.
5. Verify: `ffprobe onoda-native.mp4` (1080×1920, 30 fps, x264,
   AAC, ~60.46 s) and loudness ≈ −14.0 LUFS (the mix is the
   project's own master).
