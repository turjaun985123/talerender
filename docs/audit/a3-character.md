# A3 — Character & 2D Canvas Inventory (TaleRender Milestone 0 Audit)

**Scope (read-only):** `template/src/core/toon.ts` (803 lines), `looks.ts` (66), `cast.ts` (87),
`captions.ts` (87), `board.ts` (64) in the shipped Tall Tale Shorts skill
(`~/workspace/custom-skills/tall-tale-shorts/`). Line numbers below are from those files as
shipped. Two facts from neighbouring files are cited where the five files delegate to them and
are marked *(lib.ts / main.ts)*.

**Headline:** the entire character system is ONE procedural Canvas2D painter — `drawToonLayer`
(toon.ts:190) — with no image assets, no SVG, no sprite sheets. A character is re-rasterised from
vector path commands whenever its state changes, uploaded as a `THREE.CanvasTexture`, and shown
as a camera-facing textured plane. Captions are drawn with Canvas2D text directly into the
1080×1920 frame canvas — they are NOT part of the toon canvas. **42 distinct Canvas 2D APIs**
are used (inventory in §1).

---

## 1. CanvasRenderingContext2D API inventory

### 1.0 How often anything draws at all (frequency model)

Per-frame frequency is governed by a repaint cache in `Toon#set` (toon.ts:619–640):

- The state object is serialised to a key — `JSON.stringify` with functions replaced by name
  and **numbers rounded to 2 decimals** (toon.ts:619). If the key is unchanged, the canvas is
  NOT repainted and the texture is NOT re-uploaded.
- If the key changed: one full repaint — `setTransform` reset, `clearRect` over the whole
  canvas (toon.ts:625), `scale(res,res)` + `translate(STAGE.OX, STAGE.OY)` (toon.ts:630), one
  complete `drawToon` pass, then `tex.needsUpdate = true` (toon.ts:640) → **full-canvas texture
  re-upload** (2560×2816 px at default `res = 2`).

So per toon per frame the frequency of every API below is either **0** (state unchanged) or
**one full drawToon pass**. A full pass executes on the order of 40–70 `beginPath`, 30–60
`fill`/`stroke`, and 5–15 text calls, varying with look/mood/hat/pattern/props (source call-site
counts are given per API; executed counts vary by branch). Captions are the exception: they draw
**every frame** into the frame canvas (§3). The board (board.ts) is a QA/style-sheet tool, drawn
once per sheet, never per production frame.

### 1.1 Path construction methods

| API | First / representative file:line | What it draws here | Freq | Native equivalent | Class |
|---|---|---|---|---|---|
| `beginPath()` | toon.ts:201 (190 call sites in toon.ts) | Starts every limb, feature, prop sub-path | per repaint | CPU rasterizer path builder (or GPU path tessellation) | APPROXIMATE¹ |
| `moveTo()` | toon.ts:202 (66 sites) | Path starts: legs, body bean, mouths, hats | per repaint | same | APPROXIMATE¹ |
| `lineTo()` | toon.ts:202 (69 sites) | Leg segments (sit pose), tie/scarf/medal polygons, dizzy spiral (eye, toon.ts:356) | per repaint | same | APPROXIMATE¹ |
| `quadraticCurveTo()` | toon.ts:210 (45 sites) | **The limb curve**: legs and arms are single quadratic strokes hip/shoulder→hand/foot; brows, mouths | per repaint | same (curve flattening tolerance differs per rasterizer) | APPROXIMATE¹ |
| `bezierCurveTo()` | toon.ts:215 (9 sites on ctx, 8 on Path2D) | Body bean outline (toon.ts:215–219), bowtie, mustache, feather | per repaint | same | APPROXIMATE¹ |
| `arc()` | toon.ts:280 (55 sites) | Pupils, buttons, hat crowns, flower petals, most props; also misused as full ellipse with end-angle `7` (> 2π) in dozens of places, e.g. toon.ts:313 | per repaint | same | APPROXIMATE¹ |
| `ellipse()` | toon.ts:203 (33 sites) | **Feet** (20×10), **head** via Path2D (toon.ts:283: radii `rp × rp*0.97`), pupils (13×17), mouths, blush, shade/highlight lobes | per repaint | same (ellipse→arc tessellation differs) | APPROXIMATE¹ |
| `rect()` | toon.ts:287 (9 sites) | Head-shade mask rect; parcel/prop boxes | per repaint | same | APPROXIMATE¹ |
| `roundRect()` | toon.ts:309 (29 sites) | Overalls bib, apron, boater/tophat crowns, angry teeth, ~15 props. Uses array radii form `[16,16,4,4]` (toon.ts:449) | per repaint | same (corner tessellation differs); not in older canvas impls | APPROXIMATE¹ |
| `closePath()` | toon.ts:219 (25 sites) | Body bean, tie, laugh mouth, umbrella canopy | per repaint | same | APPROXIMATE¹ |
| `Path2D` (constructor + `moveTo`/`bezierCurveTo`/`ellipse`/`closePath`) | toon.ts:213 (body), 283 (head), 333 (bowtie) | The three shapes that are filled, clipped AND stroked: body bean, head disc, bowtie | per repaint | Native path object retained across fill/clip/stroke | APPROXIMATE¹ |

¹ APPROXIMATE as a group: Chrome rasterises with Skia. Any replacement rasterizer (tiny-skia,
custom GPU tessellation) will match flat interior fills exactly but will differ in anti-aliasing
at edges and in curve flattening. No single call in this group requires a source change.

### 1.2 Painting methods

| API | file:line | What it draws | Freq | Native equivalent | Class |
|---|---|---|---|---|---|
| `fill()` — incl. `fill(Path2D)` and `fill('evenodd')` | toon.ts:224 `fill(body)`; **toon.ts:287 `fill('evenodd')`** (137 sites) | All solid shapes; the even-odd call paints the head's shade crescent (arc + covering rect, clipped to head) | per repaint | CPU/GPU rasterizer fill, nonzero + evenodd rules | APPROXIMATE |
| `stroke()` — incl. `stroke(Path2D)` | toon.ts:202 (137 sites) | **The ink outline system**: every silhouette and feature is a stroke (widths §5.1) | per repaint | Rasterizer stroke with round caps/joins | APPROXIMATE |
| `clip()` — incl. `clip(Path2D)` | toon.ts:237 `clip(body)`, 286 `clip(head)`, 337 `clip(bt)`, 411/414 current-path (5 sites) | Confines body pattern + shade to the bean; head shading to the head disc; tongue to mouth; bowtie dots to bowtie | per repaint | Rasterizer clip (mask or stencil) | APPROXIMATE |
| `fillRect()` | toon.ts:241 (39 sites) | Stripes pattern, hat bands, highlights, book/ticket text rules, prop parts | per repaint | Rasterizer rect fill | EXACT² |
| `strokeRect()` | toon.ts:310 (15 sites) | Overalls stitching (dashed), hat bands, prop outlines | per repaint | Rasterizer rect stroke | APPROXIMATE |
| `setLineDash()` | toon.ts:310 `[8,7]`, 522 `[8,6]` (4 sites) | Overall stitch lines; map route line. Always reset with `setLineDash([])` | per repaint, overalls/map only | Rasterizer dashed stroke | APPROXIMATE |
| `createRadialGradient()` + `CanvasGradient.addColorStop()` | toon.ts:538–539 | The blob-shadow texture: 128×128 canvas, stops `rgba(30,20,40,0.75)` → `0.25` @0.6 → `0` @1 | once per process (cached singleton, toon.ts:536–540) | CPU gradient raster, one-off | APPROXIMATE (one-off, low risk) |
| `getImageData()` | toon.ts:169 | Overflow audit only: reads 4 border strips, counts alpha > 32 px. Never draws | per capture check, not per frame | CPU readback of raster buffer — trivial if the rasterizer is CPU-side; a GPU path must read back strips | EXACT (if CPU raster) / APPROXIMATE (GPU readback) |
| `drawImage()` | board.ts:38 | Style-sheet only: composites the 1080×1920 offscreen frame canvas into the 1920×1080 board | per sheet, QA tool only | Blit/scale | APPROXIMATE (QA tool, not production frames) |

² EXACT for axis-aligned solid rects fully inside the canvas — pixel-identical in any sane
rasterizer. Rects under rotation (via transform) revert to APPROXIMATE.

### 1.3 State, transform and text methods

| API | file:line | Use | Freq | Native equivalent | Class |
|---|---|---|---|---|---|
| `save()` / `restore()` | toon.ts:193/243 (24/25 sites) | Bracket flip, lean, clip regions, head tilt/squash sandwiches, prop draws, caption draw | per repaint, nested ~4 deep | State stack in rasterizer | EXACT (semantics) |
| `translate()` | toon.ts:195–197 (22 sites) | Flip pivot, lean pivot at feet (256,1000), head pivot, prop placement | per repaint | Transform stack | EXACT (math); raster result APPROXIMATE |
| `rotate()` | toon.ts:197 (11 sites) | Whole-body lean (degrees→rad), head tilt, prop angles | per repaint | same | EXACT (math) |
| `scale()` | toon.ts:196 (6 sites) | Mirror flip `scale(-1,1)`; head squash `scale(1-0.3·squash, 1+squash)` (toon.ts:281); caption fit + pop-in (captions.ts:77) | per repaint / per frame (captions) | same | EXACT (math) |
| `setTransform()` | toon.ts:625, 633; board.ts:36 (3 sites) | Repaint reset; head-canvas reset; board offscreen reset | per repaint | same | EXACT |
| `clearRect()` | toon.ts:625, 633; board.ts:36 (3 sites) | Full-canvas clear before each repaint (transparent background — the plane's alpha IS the character silhouette) | per repaint | Transparent clear | EXACT |
| `fillText()` | captions.ts:79, 81; toon.ts:314 (13 sites) | Captions (white fill + shadow pass); in-world text: stamp values `5¢`/`2¢`, book title, `TICKET`, sign text, map label; board titles/labels | captions: 2×/frame; toon: per repaint when text-bearing look/prop | Font stack (see §3) | APPROXIMATE |
| `strokeText()` | captions.ts:80 (1 site) | Caption outline: lineWidth 15, `#0B0B10`, round join | 1×/frame when caption active | Font stack + stroke raster | APPROXIMATE |
| `measureText()` | captions.ts:21 (1 site) | Caption fit: `fit = min(1, CAP.maxW / width)`; result cached per string in a Map (captions.ts:17) | once per distinct caption string | Font stack metrics | APPROXIMATE — see §5.2 |

### 1.4 Context properties (all set; one also read)

| Property | file:line (representative) | Values used | Native equivalent | Class |
|---|---|---|---|---|
| `fillStyle` | toon.ts:203 (167 sets) | Hex colours, `rgba(...)` strings, one CanvasGradient (toon.ts:539). **Also READ** at toon.ts:310 (`shadeOf(g.fillStyle as string, …)`) | Colour state | EXACT |
| `strokeStyle` | toon.ts:200 (77 sets) | Mostly `INK = '#1F1C33'` (toon.ts:44) | Colour state | EXACT |
| `lineWidth` | toon.ts:200 (87 sets) | 15 legs, 14 arms, 9 body/head outline (`LW`, toon.ts:45), 10 head outline, 12 brows, 4–8 features, 15 caption stroke | Stroke state | EXACT (value); rendered weight APPROXIMATE |
| `lineCap` | toon.ts:194 (1 set) | `'round'`, set once per drawToon pass | Stroke state | APPROXIMATE (rendering) |
| `lineJoin` | toon.ts:194; captions.ts:77 (2 sets) | `'round'` everywhere, incl. caption outline | Stroke state | APPROXIMATE (rendering) |
| `globalAlpha` | toon.ts:239 (9 sets) | Body shade 0.85, highlight 0.25, check pattern 0.55/0.5, sweat fade | Alpha state (premultiplied blend) | APPROXIMATE |
| `font` | captions.ts:16; toon.ts:314; board.ts:13 (13 sets) | All Inter: `900 ${size}px Inter` family strings (sizes 13–88 px) + board `800 …px Inter` | Font stack | APPROXIMATE |
| `textAlign` | captions.ts:77 (11 sets) | `'center'` (captions, props), `'left'`/`'right'` (board) | Font stack | APPROXIMATE |
| `textBaseline` | captions.ts:77 (3 sets) | `'middle'` (captions, sign prop); `'alphabetic'` (board) | Font stack | APPROXIMATE |
| `letterSpacing` | captions.ts:64, 74; board.ts:14–19 (9 sets) | `'0px'` for captions (an explicit assumption baked into the measure cache, captions.ts:17); `'2px'`/`'3px'`/`'6px'` on the board only | Font stack — **not supported by all canvas impls**; a native stack must apply tracking in shaping | APPROXIMATE / SOURCE-CHANGE risk if the native text API lacks tracking (board only; production captions use `'0px'`) |

### 1.5 Canvas element APIs (not context methods)

| API | file:line | Use |
|---|---|---|
| `document.createElement('canvas')` | toon.ts:537 (blob), 565 (toon), 597 (head); board.ts:7 (offscreen) | All canvases are offscreen DOM canvases; there is no on-screen canvas for characters |
| `canvas.getContext('2d')` | toon.ts:538, 566, 598; board.ts:8, 13; captions via lib `ctx` | One context per canvas, held for the canvas's life |
| `canvas.width` / `canvas.height` (set + read) | toon.ts:565, 625; scan reads at toon.ts:165–175 | Sized once at construction; read by clear + overflow scan |
| `THREE.CanvasTexture` | toon.ts:539, 568, 599 | §2 |

**Count: 42 distinct Canvas 2D APIs** = 29 context methods (§1.1–1.3: beginPath, moveTo,
lineTo, quadraticCurveTo, bezierCurveTo, arc, ellipse, rect, roundRect, closePath, fill, stroke,
clip, fillRect, strokeRect, setLineDash, createRadialGradient, getImageData, drawImage, save,
restore, translate, rotate, scale, setTransform, clearRect, fillText, strokeText, measureText)
+ 10 context properties (§1.4) + `Path2D` + `CanvasGradient.addColorStop` +
`HTMLCanvasElement.getContext`. `cast.ts` uses **zero** Canvas APIs (pure registry logic);
`looks.ts` uses **zero** (data + a deterministic picker, §4.3).

---

## 2. How the character canvas becomes a 3D object

All numbers quoted from toon.ts.

**Stage and canvas geometry (toon.ts:135):**
```ts
export const STAGE = { W: 512, H: 1024, CW: 1280, CH: 1408, OX: 384, OY: 256 };
```
- `drawToon` draws in a **512×1024 logical stage**, feet at stage y = 1000 (toon.ts:1–19 header).
- The actual canvas is **1280×1408 logical** (stage anchored at offset OX 384, OY 256; margins:
  384 each side, 256 above head, 128 below feet — headroom for raised arms, tophat crowns at
  stage y ≈ −96, held props).
- Physical pixels: canvas is `STAGE.CW × res` × `STAGE.CH × res` (toon.ts:565) with
  **default `res = 2` → 2560×2816 px** per character. Head-plane canvas (multiPlane): 1024×1024
  logical → 2048×2048 px (toon.ts:597).

**Texture upload path (toon.ts:568, 640):**
- `new THREE.CanvasTexture(canvas)`; `colorSpace = THREE.SRGBColorSpace`; `anisotropy = 4`.
  No other texture parameters are set in code (three.js defaults apply: flipY, mipmaps on).
- Upload is manual and total: `tex.needsUpdate = true` after every repaint → the **entire**
  2560×2816 canvas re-uploads; there is no dirty-region or sub-rect upload.
- The same texture feeds the shadow pass: `mesh.customDepthMaterial = MeshDepthMaterial({
  depthPacking: RGBADepthPacking, map: tex, alphaTest: 0.5 })` (toon.ts:583), while the colour
  material is `MeshBasicMaterial({ map: tex, transparent: true, alphaTest: 0.04,
  side: DoubleSide, toneMapped: false })` (toon.ts:581). Note the two different alphaTest
  thresholds — colour silhouette (0.04) is slightly larger than shadow silhouette (0.5).

**Plane geometry (toon.ts:574–580):**
- `H = height / 0.96` where `height` defaults to **1.75 m** → H ≈ 1.8229 (head top sits ~4%
  below stage top).
- `planeW = H × 1280/1024 = 1.25·H`, `planeH = H × 1408/1024 = 1.375·H` → default plane ≈
  **2.279 m × 2.506 m**, at a uniform scale of H/1024 m per logical unit on both axes.
- `PlaneGeometry(planeW, planeH)` then `geo.translate(0, H × 552/1024, 0)` so canvas row 1256
  (the feet) sits at world y = 0.
- Facing: `face()` (toon.ts:~660) yaws the pivot with `atan2` toward the camera (billboard
  around Y only by default); opt-in `perspective` adds a clamped pitch
  (`rotation.x = clamp(pitchAng × 0.45, −0.38, 0.38)`).

**Multi-plane setup (opt-in, toon.ts:590–608):** a second canvas/plane carries ONLY the head
(`drawToonLayer(..., 'headonly')` on the head canvas, `'nohead'` on the main canvas). Head
plane is 1024·s × 1024·s (s = H/1024), its pivot at `(0, 648·s, 0.055)` — **0.055 m in front**
of the body plane — `renderOrder = mesh.renderOrder + 1`, own CanvasTexture (sRGB, anisotropy
4) and own depth material (alphaTest 0.5). Stage→head-canvas mapping: `(sx + 256, sy + 160)`.

**Blob shadow (toon.ts:536–540, 584–586):** separate 128×128 radial-gradient texture on a flat
ground plane `(H/2)×0.9` wide × `(H/2)×0.34`, `rotation.x = −π/2`, y = 0.01, opacity 0.55,
depthWrite off; it shrinks/fades with height above `groundY` in `syncBlob()`.

**attach3D note:** real 3D props can be attached to the drawn hands (toon.ts:~735) — hand
positions returned by drawToon (stage coords) are converted to world via the mesh matrix, so the
2D hand position and the 3D prop meet. Any native port must preserve drawToon's returned
`Hands` coordinates exactly, not just the pixels.

---

## 3. Text rendering path

### 3.1 Fonts

- **One family: Inter**, weights 900 and 800 (700 Caladea appears only in the style-sheet entry,
  *(sheet.ts)*). Font strings used: captions `900 88px Inter`; in-toon `900 13px Inter` (stamp
  values, toon.ts:~314), `900 20px Inter` (map), `900 24px`/`900 30px Inter` (ticket), `900 30px
  Inter` (book title), `900 44px Inter` (sign prop); board `900 44px`, `800 22px`, `900 30px`,
  `800 ${size}px` with a parameterisable family defaulting to `'Inter'` (board.ts:12).
- **Loading:** entry points await the Font Loading API before any drawing —
  `document.fonts.load('900 100px Inter')` and `('800 100px Inter')` *(main.ts:42)*;
  captions.ts:75–76 states the contract: "fonts are loaded before any captionText call in both
  entry points". There is no fallback font stack anywhere — every font string names Inter alone.
- **Measuring:** `ctx.measureText(text).width` under `font = FONT; letterSpacing = '0px'`
  (captions.ts:21, 64, 74), cached per string (`fitCache`, captions.ts:17). This single measurer
  is shared by the renderer and the capture-check caption audit (`captionMetrics`,
  captions.ts:63–67) — the audit deliberately never re-measures with a second engine.

### 3.2 Caption constants and layout (captions.ts:15–16)

```ts
export const CAP = { y: 1340, size: 88, maxW: 960 };
const FONT = `900 ${CAP.size}px Inter`;
```
- Frame is 1080×1920 *(lib.ts:3: W = 1080, H = 1920)*; captions are centred at **x = 540**,
  **y = CAP.y = 1340** (captions.ts:85), `textAlign 'center'`, `textBaseline 'middle'`.
- Fit law: `fit = min(1, 960 / measuredWidth)` — text wider than 960 px at 88 px is scaled
  down uniformly (via the draw transform, captions.ts:77), never wrapped, never re-sized per
  glyph.
- Chunking: script lines split into chunks (authored via `"caps": "a | b | c"` or auto-chunked
  1 word per chunk with short-word merging ≤ 11 chars, captions.ts:24–31); chunks are anchored
  to word timings, shown from 0.03 s before their first word until the next chunk (captions.ts:
  38–55).

### 3.3 Caption drawing code path (captions.ts:73–86)

`drawCaption(t)` (every frame) finds the active chunk and calls `captionText(text, 540, CAP.y,
scale)` where scale is a **pop-in ramp 0.86 → 1 over the first 0.1 s** (ease-out, captions.ts:85).
`captionText` then, under `save()`:
1. `translate(x, y)`, `scale(sc × fit, sc × fit)`;
2. shadow pass: `fillStyle 'rgba(0,0,0,0.45)'`, `fillText(text, 3, 9)`;
3. outline pass: `lineWidth 15`, `strokeStyle '#0B0B10'`, `lineJoin 'round'`, `strokeText(text, 0, 0)`;
4. face pass: `fillStyle '#FFFFFF'`, `fillText(text, 0, 0)`;
5. `restore()`.

Three text rasterisations per caption per frame, all glyph-shaped by the browser's text engine.

---

## 4. Character drawing architecture

### 4.1 Draw order (one `drawToonLayer` 'full' pass, toon.ts:190–275)

State setup first: `save()`, `lineCap/lineJoin = 'round'` (toon.ts:194), optional mirror
(`translate(512,0); scale(−1,1)` for `flip`), then the lean sandwich — rotate about the feet
pivot (256, 1000) by `lean` degrees (toon.ts:197). Then, back to front:

1. **Legs** (toon.ts:~199–211) — behind the body. Two stroked paths, `lineWidth 15`, INK;
   geometry branches on leg pose (stand / walk / run / jump / sit); each ends in a filled foot
   ellipse (20×10 at foot point + (side×10, −4)).
2. **Body bean** (toon.ts:213–224) — a `Path2D` of 4 beziers; filled with the body colour, then
   `save(); clip(body)` for the pattern (`drawPattern`: check/stripes/overalls/vest/stamps/
   apron, toon.ts:296), a shade ellipse at `globalAlpha 0.85` and a highlight ellipse at `0.25`
   on the right/left of the torso; `restore()`; the bean is then stroked with `lineWidth LW = 9`.
3. **Neckwear** (toon.ts:329) — bowtie (its own Path2D, optionally clipped dot pattern), tie,
   scarf, collar — drawn over the body's top edge.
4. **Arms** (toon.ts:~246–259) — in front of the body. Each arm is ONE quadratic stroke
   (`lineWidth 14`) shoulder → elbow(control) → hand, lengths upper 118 / fore 112 units, angles
   from state. If a held-prop callback is set it is invoked **between the arm stroke and the
   hand** (`save(); hold(g, hx, hy, angle, side); restore()`), then the hand is a filled ellipse
   15×17. Hand coords are recorded into the returned `Hands` map.
5. **Head** (`drawHeadPart`, toon.ts:277) — skipped in 'nohead' layer mode. Inside a tilt
   sandwich (rotate about neck pivot (256, pivotY)) and a squash sandwich
   (`scale(1 − 0.3·squash, 1 + squash)` about the head centre): bun (if worn) → head `Path2D`
   ellipse (r × 0.97r) filled with head colour → clipped shade crescent (`arc` + `rect`,
   `fill('evenodd')`, shade at 0.9× head colour) → head stroked at `lineWidth LW + 1 = 10` →
   `drawFace` → `drawHat` (hat drawn last, over the head's top, shifted by `headTurn × 18` for
   parallax lag).
6. **`drawFace`** (toon.ts:384): brows (per-mood `[lift, tilt]` table, `lineWidth 12`) → eyes
   (`eye()`, toon.ts:351: arcs for happy/laugh, spiral polyline for dizzy, white ellipse +
   pupil for shock/awe, blink-squashed pupil ellipse 13×17, lid rects for smug/tired/sly/
   proud/determined/deadpan) → glasses/monocle → blush/freckles → mouth (19 mood branches:
   open-mouth ellipse with clipped tongue, laugh shape with teeth rect, or a single stroked
   curve) → mustache (bezier blob) → pencil-behind-ear → sweat drop / tears (state- or
   mood-triggered).

Multi-plane mode splits steps 1–4 ('nohead') and step 5 ('headonly') across two canvases/planes
(§2). `looks.ts` contributes **no drawing code**: it is a preset table (`kid` ⇒ height 1.35,
`elder` ⇒ height 1.68 + a stoop pose, etc.) that only feeds existing `kid`/`height`/pose
mechanisms, plus `crowdLook`. `cast.ts` is a name → look registry with an identity signature
(`lookSignature`, cast.ts:26 — identity fields: body, pattern, patCol, hat, hatCol, band, neck,
neckCol, head, kid, sorted extras) that the capture check uses to prove the same character
renders identically in every set.

### 4.2 What state drives a frame (`ToonState`, toon.ts:88–101; defaults toon.ts:192)

| Field | Drives |
|---|---|
| `mood` (19 values) | Brow table, eye branch, mouth branch, lid/blush/tear triggers |
| `look: [dx, dy]` (−1..1) | Pupil offset (×9 / ×8 units) |
| `blink` 0..1 | Pupil ellipse height `17 × (1 − 0.6·blink)`; > 0.6 → closed-eye stroke |
| `talk` 0..1 | Open-mouth width/height (`14 + 10t`, `6 + 18t`) when > 0.05 |
| `sweat`, `tears` | Sweat drop (alpha = min(1, sweat)), tear ellipses |
| `armL` / `armR`: [shoulder°, elbow°] | Two-angle arm chain (default [8, 6]) |
| `legs`: stand/walk/run/sit/jump + `phase` (rad) | Leg branch; walk/run use `sin(phase)` / `max(0, −cos(phase))` |
| `lean` (deg) | Whole-body rotation about feet |
| `bob` (units) | Vertical body drop offset |
| `squash` | Head scale sandwich (and ONLY the head — body is unaffected) |
| `holdL` / `holdR` (PropFn callbacks) | Held-prop drawing at the hand — **code, not data**: 36 entries in `PROPS` (toon.ts:461+) plus project-defined ones |
| `flip` | Horizontal mirror of the whole stage |
| `headTilt` (deg), `headTurn` (−1..1), `camPitch` (−1..1) | Head pivot rotation; feature shift `turn × 30`, hat shift `turn × 18`, eye spacing `46 × (1 − 0.18·|turn|)`; head radius `r × (1 + 0.04·camPitch)`, features slide `camPitch × 8` |
| `sitOn` (m) / `sitDrop` (units) | Sit leg branch + body drop (sitDrop default 220.2) |
| Arbitrary extra keys | Ignored by the painter; they exist only to bust the repaint cache key |

The `look` (identity) side: body/head colours, `shade`, `pattern` + `patCol`/`patCol2`, `hat`
(11 kinds) + `hatCol`/`band`, `neck` (5 kinds) + `neckCol`, `dots`, `stamps`, `extra[]`
(mustache, glasses, freckles, blush, pencil, monocle), `kid` (adds 190 units of body drop —
short legs), colour overrides (`mustacheCol`, `hairCol`).

### 4.3 Determinism properties

- **No `Math.random` anywhere in the five files** (grep-verified). `drawToon` is a pure
  function of (look, state): all motion is closed-form trig of state values (`sin(phase)`,
  `cos(phase)`); there is no time source, no accumulated state, no physics integration inside
  the painter.
- The one pseudo-random source in scope is `crowdLook` (looks.ts:44), built on lib's
  `rnd(i) = frac(sin(i × 127.1 + 311.7) × 43758.5453)` *(lib.ts:25)* — a pure sin-hash of the
  member index, so crowd member *i* always gets the same look.
- Repaint quantisation: state numbers are rounded to 2 decimals in the cache key (toon.ts:619),
  so sub-0.005 state wiggles reuse the previous raster — frame output is a deterministic step
  function of state.
- The only environmental nondeterminism is **font availability**: if Inter is not loaded when a
  caption/toon text first measures, widths differ. This is controlled procedurally (entry points
  await `document.fonts.load`, §3.1), not by the painter.
- Cross-platform caveat for a port: `Math.sin/cos` and the sin-hash are IEEE-754 double ops in
  JS; a Rust port using f64 libm may differ in the last ulp on some platforms — irrelevant for
  pixels, but the crowd sin-hash and cache-key rounding should be ported in f64, not f32.

---

## 5. Identity risks — the canvas behaviours whose mismatch most damages the characters

### 5.1 #1 — Ink stroke rasterisation: round joins/caps at fixed widths (toon.ts:194, 200–202)

Every recognisable feature of every character is a **stroked path**, not a filled shape: limbs
are single quadratic strokes (legs 15, arms 14 units wide), the body/head outlines are 9–10,
brows 12. `lineCap` and `lineJoin` are both `'round'`, set once per pass (toon.ts:194), and the
whole style is "thick ink outline" (toon.ts:1). A replacement rasterizer that flattens
quadratics/beziers differently, joins segments differently, or anti-aliases stroke edges with a
different coverage curve changes the perceived line weight and corner softness of **every limb,
face and silhouette in every frame**. This is the single highest-leverage mismatch: it cannot be
fixed per-character, and small differences read instantly as "a different drawing style".

### 5.2 #2 — Text metrics: `measureText` under Inter 900 (captions.ts:16–21)

Caption size is not a constant — it is `fit = min(1, 960 / width)` where `width` comes from the
browser's shaping of Inter at 88 px with `letterSpacing '0px'`. A native font stack with
different advance widths (different Inter build, different shaper, hinting on/off) changes every
fit value: captions render larger/smaller, and the capture check's caption audit — which shares
this exact measurer — would silently pass/fail against different numbers. The same engine shapes
all in-world text (signs, book covers, the `5¢` stamp glyphs). Identity risk is highest for
captions because the fit law amplifies metric differences into scale differences.

### 5.3 #3 — Ellipse geometry and AA: the head itself (toon.ts:283)

The head — the most recognised shape in the system — is `Path2D.ellipse(hx, hy, rp, rp × 0.97)`,
and pupils (13×17), feet (20×10), mouths, blush and torso shading are all ellipses. Browsers
convert ellipses to arc segments with their own tessellation; a port that approximates ellipses
with a different segment count or AA coverage subtly reshapes the head's 0.97 squash and the
pupil ellipses that carry gaze direction. Errors here don't look like rendering noise — they
look like a slightly different character.

### 5.4 #4 — Clip + even-odd head shading (toon.ts:286–287)

The head's modelling (the shade crescent that makes the flat disc read as a face) is produced by
a specific trick: clip to the head path, then fill an arc-plus-rect path with the `'evenodd'`
rule. Fill-rule handling, clip mask resolution and their interaction are exactly where
rasterizers differ (mask AA at the clip boundary softens or hardens the crescent edge). The same
clip mechanism confines the body pattern and shade (toon.ts:237) — mismatch shows as pattern
bleeding past the bean outline or a shade edge cutting across the torso. The laugh mouth's
clipped tongue/teeth (toon.ts:414) and bowtie dot clip (toon.ts:337) share the risk.

### 5.5 #5 — Canvas→texture alpha and colour path (toon.ts:565–583)

The character's silhouette in 3D IS the canvas alpha, thresholded twice: `alphaTest 0.04` for
colour, `0.5` for the shadow depth material. Canvas2D in the browser produces **premultiplied**
RGBA; if a native rasterizer's buffer is un-premultiplied (or vice versa) at upload, every edge
pixel gets a light/dark fringe, the effective silhouette grows or shrinks by a pixel, and the
cast shadow's cut-out no longer matches the visible character. Combined with the sRGB colour
space tag and full-canvas mipmapped upload at 2560×2816 (anisotropy 4), this path decides
whether distant characters shimmer or halo. Invisible in a still of one character at rest;
damaging across a whole film.

*Runner-up:* `globalAlpha` premultiplied blending for the body shade (0.85) and highlight (0.25)
ellipses (toon.ts:239–241) — the torso's modelling depends on exact blend arithmetic against the
pattern beneath.

---

## 6. Porting summary for the architect

- The painter is **fully procedural and stateless** — an ideal CPU-rasterizer target. No asset
  pipeline is needed for characters; the port is the rasterizer + the path/text semantics above.
- `holdL`/`holdR` are **function values inside state** (toon.ts:97) and `PROPS` is a table of 36
  drawing closures (toon.ts:461+). Any IR/serialisation boundary (compiler milestone) hits this:
  held props cannot cross a data-only IR as-is — they need to become named prop IDs with
  parameters (the closures are parameterised by at most `(title|text|n|open|col)`).
- Captions never touch the toon canvas; they draw into the 1080×1920 frame canvas every frame
  (§3.3) — in a native engine they belong to the 2D compositing layer over the rendered frame,
  and their fit law makes the font stack a **production-critical** dependency, not a UI detail.
- board.ts is QA tooling (style sheets) but uses the same kit (`drawToon`, `captionText` via the
  shared lib `ctx`), so a port that keeps the painters keeps the style sheets for free.
