# CR-B2-10 — G1 prop plates + G3/G4 look-completeness (Batch 2, Seat 10)

Charter: close CR-B1-03's gaps G1 (36 held-prop PropFn recipes
unported → 36/77 castsheet items plate-blocked) and assess G3/G4
(look features the native painter/loader drops) to PORT designs
or named EXCEPTIONs. Ownership law honoured: NEW files only,
nothing edited in place, nothing committed. All deliverables in
`~/workspace/chrome-replacement/batch2/B2-10/` plus this note.
Plates/evidence are local only (256 KB total), never git.

## A. Prop plate generator — G1 CLOSED at recipe level

`B2-10/prop_plates.py` (PIL + numpy) ports all 36 `PROPS`
recipes from `template/src/core/toon.ts` to transparent RGBA
plates in the exact format the native castsheet path consumes
(toonfull.rs `Plate`; CR-B1-03 §2; film/gen_assets.py precedent):

- stage units at RES 2; plate top-left = (x0, y0) in the hand
  frame; each PropFn's internal translate/rotate baked in (every
  recipe ignores its `angle` argument or rotates by a fixed
  authored amount — verified per recipe against the source);
- castsheet factory defaults: book('RULES', 0) closed, cash(1),
  paper('NEWS'), sign('SALE'), map('');
- canvas semantics transcribed op-by-op: INK #1F1C33, centred
  strokes (PIL outlines grow inward, so outline shapes are drawn
  expanded by lw/2 — the ring lands on path ± lw/2), round
  lineCaps (drawToon sets `lineCap='round'`), source-over alpha,
  per-corner roundRect radii (cup [4,4,14,14] via an
  arc-flattened path), dashed route by arc length (map),
  non-zero-winding fill (medal's pentagram star), text in the
  skill's own Inter-Black.otf at authored sizes with canvas
  baseline semantics.

**Audit (recipe-level, numbers — this VM has no Vulkan ICD, so
rendered A/B rides PC/CI by charter)**: per prop, hand-derived
alpha-bbox expectations from the TS geometry (incl. stroke
half-widths), colour probes inside authored fills, authored-hole
transparency probes (shovel D-handle, umbrella under-canopy),
and ink-pixel counts for the four text defaults. Full machine
record: `B2-10/audit.json`; table: `B2-10/classification.md`.

**Result: 36/36 PORTED-PROVEN, 0 PORTED-UNVERIFIED, 0
NOT-PORTED.** Generation is deterministic (two runs → identical
sha256 for all 36 PNGs). First audit run was 28/36: two failures
were generator bugs (round lineCaps not emulated — mirror handle
6 units short; PIL pieslice end>360° paints a near-full disc —
ball), both fixed in the generator; six were errors in my own
hand-derived expectations, each re-derived from the recipe with
the plate already matching the corrected arithmetic (log in
classification.md). No expectation was loosened to fit output.

**Integration smoke (no render)**: a /tmp copy of
`castsheet_native.py` with the generated origins merged reports
36 held items, **0 plate-missing**, 36 origins "proven"; the
held-sign scene carries `holdR {plate, x0: -105, y0: -255}`.

**Lead wiring (only remaining G1 act)**: (1) copy `B2-10/plates/
*.png` into the castsheet work dir's `plates/` (or point
`--plates-dir` at them); (2) merge `B2-10/PLATE_ORIGINS_B2_10.py`
into `castsheet_native.py`'s `PLATE_ORIGINS` (replacing the
paper/book film entries — the new paper origin (−82, −170.5) is
the 'NEWS' castsheet plate, NOT the film's 'MAP' plate (−200,
−270); the film assets are untouched and separate); (3) the
closing rendered proof — native castsheet kind `held` A/B
against Chrome's `held-<id>.jpg` set — rides the PC/CI session
like every other render gate this wave. Out of scope by
contract: factory variants beyond castsheet defaults (open
book, cash(n), other texts) — the recipe functions are
parameterisable by a future seat if wanted.

## B. G3/G4 look-completeness — 27 PORT designs, 0 unconditional EXCEPTIONs

Full assessment: `B2-10/look_completeness.md` — per feature:
TS spec (geometry/colours/draw site), native work at paint-op
level (all within existing `scene.rs` Painter ops), effort,
risk, verdict. Headlines:

- **G4 extras** freckles / blush / pencil / monocle: all S.
  Blush is a one-line condition extension (the mood half already
  paints); monocle/freckles sit beside the existing glasses code;
  pencil needs 8 rotated corners, no new op.
- **G4 fields** patCol2 / dots / stamps-count / hairCol: S each,
  land with their consumers (check / bowtie / stamps / hair+bow).
- **G4 appendages** ears / tail / wing / hair / scarf-streamer:
  all M, PORT. Tail + scarf share one chain painter. **⚠ Any
  new FullState field MUST enter `film.rs::state_key` +
  charstate in the same change** — the sweat/tears defect class
  (fixed at `0dec530`) is the standing trap, named per row.
  Also flagged: look `scarf` (streamer bool) vs neck `'scarf'`
  (wrap, already painted) name collision.
- **G3 hats ×8** (boater, beanie, visor, tophat, bun, flower,
  bow, mailcap): PORT, all S (bow S–M); cap/bowler blocks are
  the template; bun's site is drawHeadPart, not drawHat.
- **G3 necks**: bowtie (+dots, masked dots via existing
  polygon_mask machinery) M; collar S.
- **G3 patterns**: check (axis-aligned lines as masked rects —
  no new stroke-clip op needed) M; stripes S; overalls M (one
  new sub-op: dashed rounded-rect stroke; the dash-walk
  algorithm is proven in this seat's plate generator); apron S.
- **stamps pattern**: the ONLY feature needing text inside the
  painter ('5¢'/'2¢' tile labels), which the native
  architecture excludes by design. PORT design exists — bake
  the 10 fixed spot tiles as plates (same machinery as Part A,
  rotation baked per spot) + a masked variant of
  composite_plate — effort L, risk medium. **If Batch 3
  descopes exactly one feature, stamps is the EXCEPTION
  candidate, on that evidence** (sole text-rasterisation
  collision); the exception must be named and must not render
  stamps-less bodies under a stamps look.
- **Census closure**: recommend a Batch 3 unit test enumerating
  the TS HATS/NECKS/PatternKind/EXTRAS unions against painter
  branches, so future values fail loudly (B1-03's gap was
  precisely that values dropped silently).

Suggested Batch 3 sequencing + gates (golden/telemetry
byte-neutrality with fields absent, one positive probe scene
per feature via the B1-03 cast mode) are in the assessment's
summary. castsheet_native.py's `NATIVE_LOOK_DROPPED` list
shrinks in step as ports land (skill-side lead wiring).

## Files

| File | What |
|---|---|
| `B2-10/prop_plates.py` | generator + audit (single file, rerunnable) |
| `B2-10/plates/*.png` | 36 plates, local only |
| `B2-10/classification.md` | per-prop verdicts, origins, sizes, caveats, audit-correction log |
| `B2-10/audit.json` | machine audit record (every check, expected vs measured) |
| `B2-10/plates-manifest.json` / `plate_origins.json` / `PLATE_ORIGINS_B2_10.py` | lead wiring data |
| `B2-10/look_completeness.md` | G3/G4 per-feature assessment (27 rows) |
| this note | summary + wiring |

Not done / not claimed: no native frame was rendered (no
Vulkan ICD on this VM — stated, not worked around); no skill or
repo file was edited; nothing committed or uploaded.
