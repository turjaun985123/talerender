# GPU-FIRST TALERENDER — PHASE 0 / SEAT INV-B: CHARACTER-PAINTER PRIMITIVE INVENTORY

**Scope.** Every paint-operation call site in `paint_toon_full`
(`src/toonfull.rs:699–1691`, HEAD `bc12a5c2bbb33d93416f25eaebbe8fec8c4eb678`,
verified by `git rev-parse HEAD` at inventory time), plus the M4 subset
painter `paint_toon` (`src/scene.rs:1034–1335`) that the stroke battery
exercises. Code at HEAD is the only truth; all line numbers below are
HEAD line numbers. Structural inventory only — the deep parity math
rating is INV-A's seat; §9 gives this seat's structural first impression.

**Reading conventions.**

- *Call site* = one textual invocation in the painter body
  (`paint_toon_full`, lines 699+). Loop/branch multiplicity is stated
  separately as *executions per paint* — a site inside the leg loop
  executes ×2, a site inside the stripes `while` executes ×8, etc.
- *Device px*: the painter renders to a `Painter` of
  2560×2816 device px (`(1280*2, 1408*2)`, toonfull.rs:706). Geometry is
  authored in stage units (512×1024 stage, feet at y=1000) and mapped
  ×`RES = 2.0` (toonfull.rs:33) into device px, so a stage-space size
  `s` is ≈`2s` device px. Ellipse arguments in the code are already
  device-px radii (`r * RES`); stroke widths go through `lw(w) = w * RES`
  (toonfull.rs:710). Sizes below are device-px bounding boxes at the
  default state (no squash/tilt/lean, adult, `rp ≈ 138` stage).
- *Overlap* = does this site's paint land on pixels already painted by
  an earlier site in the same paint (painter's-algorithm stack), i.e.
  is its output blend-order-sensitive.
- All primitives resolve, inside `scene::Painter`, to per-pixel
  signed-distance coverage + `blend_cov`/`blend`
  (scene.rs:506–830): `fill_ellipse` (569), `stroke_ellipse` (584 —
  internally a 144-segment `stroke_polyline`), `stroke_polyline` (601 —
  two-pass union-distance capsule stroke), `stroke_quad` (697 —
  `flatten_quad(…, 24)` then `stroke_polyline`), `fill_polygon` (704 —
  even-odd, SDF coverage), `polygon_mask` (742) / `ellipse_mask` (763)
  (boolean, centre-sampled, **no AA**), `fill_ellipse_masked` (780).
  There is no other raster path in the character painter.

---

## 1. Count reconciliation (mechanical, grep at HEAD)

Task-supplied counts verified — they are exact for the five headline
classes once substring collisions are separated:

| Class (grep token) | Whole-file `grep -c` | In `paint_toon_full` (≥699) | Reconciliation |
|---|---|---|---|
| `stroke_polyline` | **35** | **35** | All 35 are `p.stroke_polyline` call sites in the painter; none in helpers above 699. ✅ matches task (35) |
| `fill_polygon` | 31 | 27 pure `p.fill_polygon` | 31 = 27 pure + 3 `fill_polygon_masked(` call sites (821, 840, 859) + 1 definition (229). ✅ task's 27 = pure sites |
| `fill_ellipse` | 30 | 26 pure `p.fill_ellipse` | 30 = 26 pure + 4 `p.fill_ellipse_masked` (872, 874, 1182, 1204). ✅ task's 26 = pure sites |
| `stroke_ellipse` | **17** | **17** | ✅ matches task (17) |
| `stroke_quad` | **12** | **12** | ✅ matches task (12) |
| `fill_polygon_masked` (free fn) | 4 lines | **3 call sites** (821, 840, 859) | +1 = definition at 229 |
| `fill_ellipse_masked` | 4 | **4 call sites** (872, 874, 1182, 1204) | — |
| `polygon_mask` | 7 lines whole-file | **2 direct sites** (801, 1202) | Other lines: def-free uses in helper 230 (`fill_polygon_masked` internal), plus scene.rs is a different file; in toonfull.rs: 230, 801, 1202 are the `p.polygon_mask` calls (230 is inside the helper, executed per `fill_polygon_masked` call) |
| `ellipse_mask` | — | **1 direct site** (1180) | Inside the `open_mouth` closure; executed per closure invocation (≤3 call sites, §5) |
| `composite_plate` | 2 lines | **1 call site** (965) | +1 = definition at 284; site is in the arm loop → ≤2 executions/paint |
| `p.blend_cov` direct | — | **1 site** (1014, head shade loop) | +1 helper-internal site at 346 (`composite_plate` bilinear) |
| `p.blend(` direct | — | **1 site** (1211, laugh teeth loop) | +1 helper-internal site at 243 (`fill_polygon_masked`) |
| `canvas_anchor` | 9 lines | 1 call (716) + definition 404 | Geometry-side only — §6 |

**Closure caveat.** The `open_mouth` closure (toonfull.rs:1175–1185)
contains 3 primitive sites (`fill_ellipse` 1177, `stroke_ellipse` 1178,
conditionally `ellipse_mask` 1180 + `fill_ellipse_masked` 1182) and is
invoked from 3 sites (1186 Shock, 1217 talk, 1263 Awe; detailed in §4). The counts
above count the closure body **once**, as written.

**Indirect-dispatch caveat.** Every `stroke_ellipse` site is also a
`stroke_polyline` execution at the `Painter` level (scene.rs:598), and
every `stroke_quad` site likewise (scene.rs:699). A GPU backend that
implements `stroke_polyline` + `fill_ellipse` coverage faithfully
inherits both classes; they are not independent rasterisers.

**Total textual primitive sites in `paint_toon_full`:**
35 + 27 + 26 + 17 + 12 + 3 + 4 + 2 + 1 + 1 + 2 (direct blends) = **130**,
plus the 3 closure-body sites counted once above are already inside the
26/17/4/1 figures. Maximum executions per single paint ≈ **215**
(all looks/extras on, stripes pattern, flower hat, both plates held —
see per-site multiplicities in §3–§5).

---

## 2. Global gating state (what can gate any site)

- **Look** (`FullLook`, toonfull.rs:147–174): `body`, `head`, `shade?`,
  `pattern ∈ {plain, vest, stripes, apron}`, `pat_col?`,
  `hat ∈ {none, cap, bowler, boater, beanie, visor, tophat, bun, flower,
  bow, mailcap}`, `hat_col?`, `band?`,
  `neck ∈ {none, tie, scarf, collar}`, `neck_col?`, `glasses`,
  `mustache` (+`mustache_col?`), `kid`, `blush`, `freckles`, `pencil`,
  `monocle`, `hair_col?` (bow locks only).
- **State** (`FullState`, 176–197): `mood` (19 values, §5.1),
  `look: [f32; 2]` (gaze), `blink`, `talk`, `sweat`, `tears`,
  `arm_l/arm_r: (f32, f32)` FK angles, `legs ∈ {Stand, Walk, Run, Sit,
  Jump}`, `phase`, `lean`, `bob`, `squash`, `head_tilt`, `head_turn`,
  `cam_pitch`, `sit_drop?`, `hold_l/hold_r: Option<Plate>`.
- **Derived geometry every site shares:** `by` (sit/kid/bob/jump
  vertical shift, 730–733), bean `(bx=256, top=330+by, bot=600+0.98by,
  bw=92)`, head `(hx=256, hy=178+0.99by, r=138)`, `rp = r·(1+0.04·cam_pitch)`,
  maps `px` (lean about feet + anchor + offset, 720) and `hpt`
  (squash about head centre → tilt about neck pivot → `px`, 743),
  `head_rot = lean + tilt` (752). Body sites use `px`; head/face/hat
  sites use `hpt`. This split is the single most important structural
  fact for a GPU port: two transform stacks, one shared lean.

---

## 3. Master inventory — `paint_toon_full` by section

Sizes are device-px bboxes at default state. "Stack" = overlaps
previously painted pixels (Y = always, C = conditionally/partially,
N = paints on empty canvas in the normal case).

### 3.1 Legs (toonfull.rs:754–786) — gate: `st.legs`, `st.phase`, `sit_drop`

| Line | Class | Paints | Gate | Exec | Size (dev px) | Stack |
|---|---|---|---|---|---|---|
| 755 | stroke_polyline | Sit leg: 3-pt polyline hip→knee→foot, lw 15 | `legs==Sit` (per leg) | ×2 | ~90×450, width 30 | N (first paint; bean later covers hip joint) |
| 761 | fill_ellipse | Sit foot: ellipse r 20×10 at foot + (side·10, −4) | `legs==Sit` | ×2 | 80×40 | C (touches leg end) |
| 782 | stroke_polyline | Non-sit leg: `flatten_quad(hip, knee, foot, 20)`, lw 15. Knee/foot from branch: Walk (amp .55, lift ≤34), Run (amp 1.0, lift ≤70), Jump (fixed tucked), Stand (fixed) | `legs!=Sit`; sub-branch on `legs` + `phase` | ×2 | ~110×860, width 30 | N |
| 784 | fill_ellipse | Non-sit foot, same geometry as 761 | `legs!=Sit` | ×2 | 80×40 | C |

### 3.2 Body bean + patterns + shade (788–880)

| Line | Class | Paints | Gate | Exec | Size | Stack |
|---|---|---|---|---|---|---|
| 800 | fill_polygon | Bean body fill (`look.body`): 4 flattened cubics, 109 pts | always | ×1 | ~370×540 | C (over leg tops) |
| 801 | polygon_mask | Bean clip mask (full-canvas bool) — feeds 821/840/859/872/874 | always | ×1 | canvas | — (mask, no paint) |
| 821 | fill_polygon_masked | Stripes: vertical rect 14 wide, full bean height, colour `pat_col ‖ shade(body,0.7)` | `pattern=="stripes"` | ×8 (`sx` −130…<130 step 36) | each ~28×580, clipped to bean | Y (over bean fill) |
| 840 | fill_polygon_masked | Apron body: rounded rect 140×(bot−top), r18, `pat_col ‖ #F4EFE4` | `pattern=="apron"` | ×1 | ~280×540, clipped | Y |
| 848 | stroke_polyline | Apron pocket outline: rounded rect 80×60 r8, lw 4, `shade(pat,0.8)` — **unmasked** (probe-verified inside bean, comment 834–838) | `pattern=="apron"` | ×1 | ~160×120 | Y |
| 859 | fill_polygon_masked | Vest panel: quad from (bx±140, top−10) to centre slit | `pattern=="vest"` | ×2 (per side) | each ~260×580, clipped | Y |
| 863 | fill_ellipse | Vest buttons: r6 discs at (bx+26, top+150/195/240), `#E9C46A` — **unmasked** | `pattern=="vest"` | ×3 | 24×24 each | Y (over panel/bean) |
| 872 | fill_ellipse_masked | Body shade: ellipse r 70×260 at (bx+bw+40, mid+20), alpha .85, `shade ‖ shade(body,0.8)`, clipped to bean | always | ×1 | 280×1040, clipped | Y |
| 874 | fill_ellipse_masked | Body highlight: ellipse r 16×60 rot 0.2+th at (bx−50, top+90), white alpha .25, clipped | always | ×1 | 64×240, clipped | Y |
| 878 | stroke_polyline | Bean outline ring (bean pts + closing pt), lw 9, ink | always | ×1 | ~370×540 ring, width 18 | Y (over fill/shade edges) |

### 3.3 Neckwear (882–947) — gate: `look.neck`, `neck_col`

| Line | Class | Paints | Gate | Exec | Size | Stack |
|---|---|---|---|---|---|---|
| 892 | fill_polygon | Tie blade: 5-pt polygon (bx±14/±20, top+10…+170) | `neck=="tie"` | ×1 | ~80×320 | Y (over bean top) |
| 895 | stroke_polyline | Tie outline ring, lw 6 | tie | ×1 | as above | Y |
| 905 | fill_polygon | Scarf tail: quad (bx+6…+58, top+20…+138) | `neck=="scarf"` | ×1 | ~105×235 | Y |
| 908 | stroke_polyline | Scarf tail ring, lw 6 | scarf | ×1 | as above | Y |
| 916 | fill_polygon | Scarf wrap band: 3 chained quads (flattened, 47 pts) around neck | scarf | ×1 | ~280×145 | Y |
| 919 | stroke_polyline | Scarf band ring, lw 6 | scarf | ×1 | as above | Y |
| 921 | stroke_quad | Scarf shade line across band, lw 4, `shade(c,0.75)` | scarf | ×1 | ~200×60 arc | Y |
| 940 | fill_polygon | Collar triangle per side: (bx, top+18), (bx±56, top+2), (bx±40, top+46); white — `neck_col` **ignored** (comment 934–936) | `neck=="collar"` | ×2 | each ~110×90 | Y |
| 943 | stroke_polyline | Collar triangle ring, lw 6 | collar | ×2 | as above | Y |

### 3.4 Arms + held plates + hands (949–973)

| Line | Class | Paints | Gate | Exec | Size | Stack |
|---|---|---|---|---|---|---|
| 963 | stroke_polyline | Arm: `flatten_quad(shoulder, elbow, hand, 20)` FK (upper 118, fore 112 stage; angles `arm_l/arm_r`), lw 14 | always (per arm) | ×2 | up to ~460 long, width 28 | C (shoulder over bean edge; rest mostly empty canvas) |
| 965 | composite_plate | Held-prop plate composited at hand point (see §6.2) | `hold_l`/`hold_r` is `Some` (per arm) | ≤×2 | plate-dependent (prop plates, stage units ×2) | Y (over arm; hand 968 then paints over plate edge) |
| 968 | fill_ellipse | Hand disc r 15×17 ink at FK hand point | always (per arm) | ×2 | 60×68 | Y (over arm end / plate) |

### 3.5 Head base (975–1019)

| Line | Class | Paints | Gate | Exec | Size | Stack |
|---|---|---|---|---|---|---|
| 987 | fill_ellipse | Bun disc r46 at (hx+10+turn·18, hy−rp−8), `hat_col ‖ #6B4A3A` — drawn **before** head disc (head-part, not a hat branch) | `hat=="bun"` | ×1 | ~184×184 (squash-scaled radii) | N→C (head disc then overlaps its lower edge) |
| 988 | stroke_ellipse | Bun outline, lw 9 | bun | ×1 | as above | Y |
| 992 | fill_ellipse | **Head disc**: centre (hx,hy), rx `rp·(1−0.3·sq)`, ry `rp·0.97·(1+sq)`, rot `head_rot`, `look.head` | always | ×1 | ~552×535 | C (over bun bottom, bean top, neckwear top) |
| 1014 | blend_cov (direct, in loop) | Head even-odd shade: pixels inside head ellipse AND outside offset circle (centre hpt(hx+52+turn·30, hy+44), radii (rp·(1−0.3sq), rp·(1+sq))), colour `shade(head,0.9)` alpha via coverage product `cov_head · unshaded` | always | per-pixel loop over ~552×552 bbox | ~552×535 crescent | Y (over head fill) |
| 1019 | stroke_ellipse | Head outline, lw 10 | always | ×1 | ~552×535 ring, width 20 | Y |


---

## 4. The face in full detail (toonfull.rs:1021–1345)

All face sites use the `hpt` map (squash→tilt→lean) and `head_rot`.
Face origin: `fx = hx + turn·30` (1033), eye spacing
`ex = 46·(1 − 0.18·|turn|)` (1024), eye line
`ey = hy + 4 + cam_pitch·8` (1025), mouth line `my = ey + 62` (1171).
Per-side loops run over `s ∈ {−1, +1}`.

### 4.1 Brows (1028–1056) — all 19 moods

| Line | Class | Paints | Gate | Exec | Size | Stack |
|---|---|---|---|---|---|---|
| 1048 | stroke_quad | Brow arc per side, lw 12; `(lift, tilt)` from `Mood::brow()` table (toonfull.rs:106–126): Neutral (4,0), Smug (−8,10), Happy (8,0), Laugh (10,0), Shock (24,0), Worried (10,−16), Angry (−10,22), Sad (8,−18), Sly (−6,12), Proud (6,4), Tired (−2,0), Dizzy (14,0), Determined (−4,16), Embarrassed (12,−10), Awe (26,0), Suspicious (−2,6), Relieved (14,−4), Puzzled (6,4), Deadpan (2,0). Per-side overrides: Smug/Sly right brow → (16,−4); Suspicious left → (18,−6); Puzzled left (16,−2) / right (−2,8) | always, every mood | ×2 | each ~104×~30 arc, width 24 | Y (over head fill/shade) |

### 4.2 Eyes (1058–1127) — dispatch order matters

Branch order per eye: (1) arc family → (2) Shock/Awe → (3) blink →
(4) default pupil (+lid, +sparkle). **Blink is unreachable for the
arc and Shock/Awe families** — a structural quirk a port must keep.

| Line | Class | Paints | Gate | Exec | Size | Stack |
|---|---|---|---|---|---|---|
| 1067 | stroke_polyline | Closed-eye arc: 25 pts, angles 1.15π…1.85π, r20 about (x, ey+10), lw 10 | mood ∈ {Happy, Laugh, Relieved} | ×2 | ~80×45 | Y |
| 1072 | fill_ellipse | Eye white r 26×30 | mood ∈ {Shock, Awe} | ×2 | 104×120 | Y |
| 1073 | stroke_ellipse | Eye-white outline, lw 6 | Shock/Awe | ×2 | 104×120 ring | Y |
| 1076 | fill_ellipse | Pupil r7 (Shock) / r9 (Awe), offset by gaze `look·8` | Shock/Awe | ×2 | 28–36 Ø | Y (over white) |
| 1079 | fill_ellipse | Pupil sparkle r3 white at pupil −(3,3) | Awe only | ×2 | 12 Ø | Y |
| 1084 | stroke_quad | Blink lid arc, lw 8 | default family AND `blink > 0.6` | ×2 | ~56×20 | Y |
| 1096 | fill_ellipse | Default pupil r 13 × 17·(1−0.6·blink)·slit·deadpan, offset gaze `look·(9,8)`; slit 0.55 = Suspicious left eye only; deadpan factor 0.8 = Deadpan | default family, `blink ≤ 0.6` | ×2 | 52×~68 | Y |
| 1112 | fill_polygon | Heavy-lid rect (head colour): (x±24, ey−28)…(ey−2+lid_offset); lid_offset 6 Tired, 8 Deadpan, else 0 (`lid_offset()`, 130–136) | `mood.heavy_lid()` = {Smug, Tired, Sly, Proud, Determined, Deadpan} | ×2 | 96×~52–68 | Y (over pupil top) |
| 1113 | stroke_polyline | Lid line (x−20…x+20 at ey−2+lid_offset), lw 7 | heavy-lid moods | ×2 | 80×14 | Y |
| 1124 | fill_ellipse | Pupil sparkle r3.5 white at pupil −(3,5) | mood ∈ {Sad, Worried} (default family) | ×2 | 14 Ø | Y |

### 4.3 Eye extras (1129–1150)

| Line | Class | Paints | Gate | Exec | Size | Stack |
|---|---|---|---|---|---|---|
| 1130 | stroke_ellipse | Glasses rim r30 per eye, lw 5 | `look.glasses` | ×2 | 120 Ø ring | Y (over eye stack) |
| 1132 | stroke_polyline | Glasses bridge (fx−16…fx+16 at ey−4), lw 5 | glasses | ×1 | 64×10 | Y |
| 1141 | stroke_ellipse | Monocle rim r30, gold `#C9A13B`, lw 5, at figure's **right** eye only (fx+ex, ey) | `look.monocle` | ×1 | 120 Ø ring | Y |
| 1142 | stroke_quad | Monocle chain: quad via (fx+ex+40, ey+90) to (fx+ex+10, ey+140), lw 5 gold | monocle | ×1 | ~85×280 arc | Y/C |

### 4.4 Cheeks (1152–1170)

| Line | Class | Paints | Gate | Exec | Size | Stack |
|---|---|---|---|---|---|---|
| 1157 | fill_ellipse | Blush r 22×12 at (fx±78, ey+40), rgba(240,120,130,0.35) | `look.blush` OR mood ∈ {Laugh, Proud, Embarrassed} | ×2 | 88×48 each | Y |
| 1168 | fill_ellipse | Freckle dots r3.5 `#C98A64` at (fx+s·(74+dx), ey+36+dy), (dx,dy) ∈ {(0,0),(12,8),(−10,10)} — dx inside the side multiplication, as authored | `look.freckles` | ×6 | 14 Ø each | Y |

### 4.5 Mouths (1171–1280)

**`open_mouth` closure (1175–1185)** — one body, three invocations:

| Line | Class | Paints | Gate (inside closure) |
|---|---|---|---|
| 1177 | fill_ellipse | Mouth disc r (w, max(h,4)) at (fx, my+0.3h), `#3A1A2A` | always in closure |
| 1178 | stroke_ellipse | Mouth outline, lw 8 | always in closure |
| 1180 | ellipse_mask | Mouth-ellipse clip mask (full-canvas bool) | `tongue && h > 12` |
| 1182 | fill_ellipse_masked | Tongue r (0.7w, 0.6h) at (fx, my+1.1h), `#E2697A`, clipped by 1180 | `tongue && h > 12` |

Closure invocations: **1186** Shock — (w16, h 20+6·talk, tongue);
**1217** talk branch — any mood with `talk > 0.05`, (w 14+10·talk,
h 6+18·talk, tongue); **1263** Awe — (w11, h13, **no tongue** — and
h=13 would pass the h>12 test, so the `tongue:false` flag is the only
thing suppressing the Awe tongue).

**Laugh mouth (1188–1216) — the quirk, in full.** Dispatch:
`mood==Laugh` is checked *before* the talk branch, so a laughing,
talking character still gets this mouth.

| Line | Class | Paints | Gate | Size | Stack |
|---|---|---|---|---|---|
| 1201 | fill_polygon | Laugh mouth shape: 3 chained quads — top edge (fx±40, my−6), sides down to (fx, my+52) — filled `#3A1A2A` | Laugh | ~160×115 | Y |
| 1202 | polygon_mask | Mouth-shape clip (full-canvas bool) — feeds 1204 + 1211 | Laugh | canvas | — |
| 1204 | fill_ellipse_masked | Tongue r 24×18 at (fx, my+50), `#E2697A`, clipped to mouth | Laugh | 96×72, clipped | Y |
| 1211 | blend (direct, in loop) | Teeth: white fill over the masked bbox strip between hpt(fx−40, my−10) and hpt(fx+40, my+2) — a masked rect fill open-coded as a per-pixel `p.blend` loop (no primitive) | Laugh | ~160×24 strip, clipped | Y |
| 1215 | stroke_ellipse | **THE QUIRK:** the outline stroke lands on the **tongue ellipse** (r 24×18 at the tongue centre, lw 8), *not* on the mouth shape — a faithful port of the source Canvas2D path-state bug (the final `stroke()` runs after the tongue's `beginPath()`; cf. the M4 twin's comment, scene.rs:1263–1269). The laugh mouth outline is **never stroked** | Laugh | 96×72 ring | Y |

**Closed-mouth mood match (1219–1279)** — reached only when
mood ∉ {Shock, Laugh} and `talk ≤ 0.05`:

| Line | Class | Mood(s) | Shape |
|---|---|---|---|
| 1221 | stroke_quad | Happy, Proud | Smile arc (fx±28, my)→(fx, my+26), lw 8 |
| 1224 | stroke_quad | Smug, Sly | Asymmetric smirk (fx−22,my+6)→(fx+4,my+16)→(fx+24,my−4), lw 8 |
| 1234 | stroke_polyline | Worried | Flattened cubic wobble (fx−26,my+8)…(fx+26,my+2), lw 8 |
| 1237 | stroke_quad | Angry | Frown arc (fx±30, my+12)→(fx, my−8), lw 8 |
| 1242 | fill_polygon | Angry | Teeth: rounded rect 48×14 r6 at (fx−24, my−2), white |
| 1245 | stroke_polyline | Angry | Teeth outline ring, lw 5 |
| 1248 | stroke_quad | Sad | Frown arc (fx±24, my+12)→(fx, my−4), lw 8 |
| 1251 | stroke_polyline | Tired, Dizzy, Deadpan | Straight line (fx±18, my+6), lw 8 |
| 1254 | stroke_polyline | Determined | Near-flat line (fx−24,my+5)→(fx+24,my+1), lw 8 |
| 1260 | stroke_polyline | Embarrassed | Two chained quads, small wobble ±17 wide, lw 8 |
| 1266 | stroke_quad | Suspicious | Slanted line-arc (fx−22,my)→(fx,my+2)→(fx+24,my+9), lw 8 |
| 1269 | stroke_quad | Puzzled | Small quad (fx±16, my+5/my−1), lw 8 |
| 1272 | stroke_quad | Relieved | Soft smile (fx±20, my)→(fx, my+18), lw 8 |
| 1274 | fill_ellipse | Relieved | Open dot r 5×3 `#3A1A2A` at (fx, my+16) — under the smile arc |
| 1277 | stroke_quad | default: Neutral (+ any unlisted) | Smile arc (fx±20, my+2)→(fx, my+14), lw 8 |

### 4.6 Face extras: mustache, pencil, sweat, tears (1282–1345)

| Line | Class | Paints | Gate | Exec | Size | Stack |
|---|---|---|---|---|---|---|
| 1295 | fill_polygon | Mustache: 4 chained cubics, symmetric wings to fx±64, `mustache_col ‖ #3A2A22` | `look.mustache` | ×1 | ~256×60 | Y (over mouth top) |
| 1298 | stroke_polyline | Mustache outline ring, lw 4 | mustache | ×1 | as above | Y |
| 1320 | fill_polygon | Pencil body: rect 16×120 `#F2C230`, rotated −0.5 rad about anchor (fx+132, hy−50) in stage space, then `hpt` | `look.pencil` | ×1 | ~32×240 (rotated) | Y/C (beside head) |
| 1321 | fill_polygon | Pencil eraser: rect 16×14 `#E58A9A`, same rotation | pencil | ×1 | ~32×28 | Y |
| 1324 | stroke_polyline | Pencil outline ring (rect 16×130), lw 4 | pencil | ×1 | as body | Y |
| 1334 | fill_polygon | **Sweat drop**: two chained quads forming a teardrop — tip (sx, sy−26), belly (sx, sy+12), half-width 18 — at (sx, sy) = (fx+104, hy−70), fill `#8FD3F4` | `st.sweat` OR mood==Worried | ×1 | ~72×76 | Y/C (head edge / empty canvas) |
| 1337 | stroke_polyline | Sweat drop outline ring, lw 4 | sweat/Worried | ×1 | as above | Y |
| 1342 | fill_ellipse | **Tear drops**: r 7×14 `#8FD3F4` at (fx±(ex+6), ey+40) — plain ellipses, no outline, no mask | `st.tears` OR mood==Sad | ×2 | 28×56 each | Y (over cheek/head) |

---

## 5. Hats (1347–1691) — gate: `look.hat` (centre `hatx = hx + turn·18`, `htop = hy − rp·0.97`)

All hat sites use `hpt`. `bun` is **not** here — it paints in the head
section (§3.5, 987–988) because the source draws it before the head disc.

| Hat | Sites (line: class — part) |
|---|---|
| cap | 1363 fill_polygon — crown (upper half-ellipse r rp·0.98×62, rot −0.05+head_rot, as 49-pt polygon); 1366 stroke_polyline — crown ring **closed** (chord stroked), lw 9; 1378 fill_polygon — brim (lower half-ellipse r rp·0.78×22, shade 0.75); 1379 stroke_polyline — brim, **open** (no closing pt — chord not stroked); 1383 fill_ellipse — button r12 (shade 1.15, rot 0.0 — canvas-absolute, not head_rot); 1384 stroke_ellipse — button, lw 9 |
| bowler | 1392 fill_ellipse — brim r rp·1.02×22; 1393 stroke_ellipse — brim, lw 9; 1404 fill_polygon — dome (upper half-ellipse r rp·0.66×80 as polygon); 1407 stroke_polyline — dome ring, lw 9; 1418 fill_polygon — band rect (w rp·0.64, h 24, `band ‖ shade(c,0.6)`); 1421 stroke_polyline — band ring, lw 9 |
| boater | 1434 fill_ellipse — brim r rp·1.3×26; 1435 stroke_ellipse — brim; 1441 fill_polygon — crown rounded rect (rp·1.32×76, radii [16,16,4,4]); 1444 stroke_polyline — crown ring; 1453 fill_polygon — band rect (`band ‖ #3C6FB6`); 1456 stroke_polyline — band ring; **1460 stroke_polyline — straw hatch**: verticals leaning +6, lw 3, shade(c,0.82), **unclipped**, in a `while` loop `hoff` −0.6rp…<0.6rp step 16 → ~11 executions |
| beanie | 1486 fill_polygon — dome (upper half-ellipse r rp·0.95×96 as polygon); 1489 stroke_polyline — dome ring; 1496 fill_polygon — brim rounded rect (rp·1.96×40, r18, shade 0.8); 1499 stroke_polyline — brim ring; 1503 fill_ellipse — pom r22 (`band ‖ #F4EFE4`); 1504 stroke_ellipse — pom |
| visor | 1529 fill_polygon — translucent sheet (two chained quads, rgba(70,190,120,0.85)); 1532 stroke_polyline — sheet ring; 1546 fill_polygon — cap band over sheet top (same construction, smaller); 1549 stroke_polyline — band ring |
| tophat | 1560 fill_ellipse — brim r rp×20; 1561 stroke_ellipse — brim; 1567 fill_polygon — crown rounded rect (rp·1.2×164, r8); 1570 stroke_polyline — crown ring; 1579 fill_polygon — band rect (`band ‖ #B5372E`); 1582 stroke_polyline — band ring |
| bow | 1610 fill_polygon — bow loop per side (2 chained cubics to ±66 wide); 1613 stroke_polyline — loop ring, lw 6 (×2); 1617 fill_ellipse — centre r15; 1618 stroke_ellipse — centre, lw 6; 1624 fill_ellipse — side lock r 22×50 rot head_rot∓0.2, `hair_col ‖ #6B4A3A` (×2); 1625 stroke_ellipse — lock, lw 6 (×2) |
| mailcap | 1641 fill_polygon — crown rounded rect (rp·1.44×70, radii [20,20,4,4]); 1644 stroke_polyline — crown ring; 1655 fill_polygon — brim: INK half-ellipse r rp·0.82×18 rot 0.04+head_rot as 49-pt polygon, **fill only, no stroke** (as authored); 1659 fill_ellipse — badge r13 `#E2B33C`; 1660 stroke_ellipse — badge, lw 4 |
| flower | 1672 fill_ellipse — petal r14 at radius 18 around (hatx+rp·0.62, htop+40), ×6 loop; 1673 stroke_ellipse — petal, lw 4 (×6); 1677 fill_ellipse — centre r11 `#F6C945`; 1678 stroke_ellipse — centre, lw 4 |

Hat stack note: every hat paints **after** the full face, over the
head disc + head outline (+ bun region); brims overlap the brow band.
All hat strokes lw 9 (bow 6, flower/mailcap-badge 4) at RES 2 → 18/12/8
device px wide.

---

## 6. canvas_anchor and composite_plate

### 6.1 `canvas_anchor` (definition toonfull.rs:404–697; called once, 716)

`canvas_anchor(look, st) -> (anx, any)` computes a stage-space
translation that `paint_toon_full` folds into its stage→canvas map:
`px` applies lean about the feet (256,1000), **then** `+(anx, any)`,
then `+(384, 256)`, then ×RES (720–727); `composite_plate` receives
the same `(anx, any)` (965) and applies it in its forward/inverse
maps (284–356). It exists because the cut-out canvas is finite:
high-|lean| states rotate the head past a side edge and high seats
(`sit_drop`) lift it past the top; the anchor rebuilds the painted
envelope from the painter's own geometry (bean cubics, head ellipse,
hat/neckwear extremes, arm FK, every leg branch), runs it through the
painter's transform stack, and returns the smallest whole-figure
translation bringing the envelope inside the canvas, inset by 4 stage
units (405, 658–696).

**Why it is NOT a paint primitive:**

- It touches no pixels: no `Painter` argument, no blend, no coverage —
  its output is two floats consumed by the coordinate maps.
- It is a **pure function of (look, state)** (doc comment 376–403):
  deterministic, exactly (0,0) for every state that already fits (so
  previously clean states paint byte-identically), and continuous in
  the state (an animated fall slides its anchor instead of popping).
- It is geometry-side bookkeeping *about* the paint, in the same
  category as `px`/`hpt` themselves: it changes **where** every
  primitive lands, never **how** any primitive rasterises. A GPU port
  implements it once, on the host, as a uniform/offset in the
  stage→canvas transform — it adds no GPU primitive class.
- Its envelope model deliberately **excludes held plates** (comment
  395–397: plate reach is bounded by the prop contract; plates ride
  the hands, which are modelled) — one more sign it is a bounds
  solver, not a painter.

### 6.2 `composite_plate` (definition 284–356; call site 965)

The one non-`Painter`-primitive paint operation. Per held arm with a
`Plate` (pre-rendered RGBA at RES 2 in stage units): forward-map the
plate rect's corners through lean + anchor to get the canvas bbox,
then per canvas pixel inverse-map (undo offset/anchor/lean) to
plate-local coordinates, bilinear-sample the plate (premultiplied
handling per comment 320–321), and `p.blend_cov(x, y, sampled_rgb,
sampled_alpha)` (346) when alpha > 0. Order: arm stroke (963) →
plate (965) → hand disc (968), so the hand overpaints the plate's
grip edge. Plates do **not** rotate with any per-prop angle at
composite time beyond the figure lean — the film's props bake their
fixed angles into the plate artwork (module docs, 20–25).

---

## 7. Text: does the character painter rasterise glyphs?

**No. Neither painter draws any text itself.** Established from code:

- `paint_toon_full` (toonfull.rs) contains **no** text/font/glyph
  call of any kind — the only text-adjacent line in the file is the
  module doc (line 22): held props arrive as pre-rendered plates
  "rendered by film/gen_assets.py from the same prop geometry —
  **including the Inter text, which enters as data**." Any lettering
  on a held paper/leaflet/book is pixels inside the `Plate`, arriving
  through `composite_plate` (§6.2) — the painter sees only RGBA.
- `paint_toon` (scene.rs) likewise has no text path.
- `paint_parcel` (scene.rs:2550–2576) — the texture painter the task
  names — draws **no glyphs**: its `label=true` branch is two plain
  `fill_rect`s (label paper `#FFFDF4` 90×70 at (150,150), stamp
  `#D8343F` 20×24 at (214,156), 2571–2575). The "label" is a blank
  paper rectangle with a stamp block; the bookshop stages both
  parcels with `label=false` anyway (comment 2547–2548).
- The engine-wide rule is stated at the one place text *would*
  appear: `paint_sign`'s doc (scene.rs:2790–2797) — "this engine has
  **NO Rust text rasteriser** — every text-bearing texture enters as
  a PIL asset rendered from the project's own fonts"; `sign_with_text`
  (2901) merely composites a pre-rendered PIL glyph strip over the
  natively painted board, and without a strip the board carries no
  text (test at 2955).

**Precise statement for the GPU plan:** no painter surface in
`toonfull.rs` or `scene.rs` rasterises glyphs. All text reaches the
frame as pre-rendered pixels — held-prop plates (composited, §6.2),
PIL glyph strips (composited, scene.rs:2901), or caption/calendar
plate assets — so a GPU character painter needs **no font/text
pipeline at all**; text is entirely a composite-plate problem.

---

## 8. scene.rs `paint_toon` — the M4 subset (scene.rs:1034–1335)

`paint_toon(spec: &ToonSpec)` is the five-mood stand subset the §28
scene / stroke battery uses: `ToonMood ∈ {Neutral, Determined, Angry,
Laugh, Shock}` (992–998), `ToonSpec` = body/head/shade?/cap?/arms/
lean/head_turn/cam_pitch (1000–1013) — **no** patterns, neckwear,
legs-state (stand only), hats other than cap, extras, blink/talk,
sweat/tears, squash/tilt (lean only, via its own `px` map, 1046–1053),
no anchor, no plates. Canvas identical: 2560×2816, RES 2.

Textual sites in range (grep, reconciled as in §1 — `p.fill_ellipse`
grep counts 13 lines including the 4 masked):

| Class | Sites | Lines (scene.rs) | Battery-relevant notes |
|---|---|---|---|
| stroke_polyline | 9 | 1063 legs (×2), 1092 bean ring — **unclosed** `&bean` (toonfull closes it explicitly, 876–879), 1106 arms (in `arm` closure ×2), 1183 laugh/shock eye arc (×2), 1198 determined lid line (×2), 1215 determined mouth, 1231 angry teeth ring, 1306 cap crown ring, 1321 cap brim (open) | |
| fill_polygon | 5 | 1081 bean, 1228 angry teeth, 1246 laugh mouth, 1300 cap crown, 1320 cap brim | |
| fill_ellipse (pure) | 9 | 1065 feet (×2), 1108 hands (×2), 1118 head, 1187 shock eye white (×2), 1189 shock pupil (×2) — **at eye centre, no gaze offset** (toonfull offsets by look·8, 1075), 1193 default pupil (×2), 1208 laugh blush (×2), 1276 shock mouth, 1327 cap button | |
| fill_ellipse_masked | 4 | 1089 bean shade, 1091 bean highlight, 1250 laugh tongue, 1280 shock tongue | Same masks as toonfull |
| stroke_ellipse | 5 | 1143 head, 1188 shock eye (×2), 1270 laugh **tongue** (the quirk, §4.5 — with its explanatory comment at 1263–1269), 1277 shock mouth, 1328 cap button | |
| stroke_quad | 3 | 1161 brows (×2), 1218 neutral mouth, 1221 angry mouth | |
| polygon_mask | 2 | 1082 bean, 1247 laugh mouth | |
| ellipse_mask | 1 | 1278 shock mouth (for tongue) | |
| fill_rect | 1 | 1197 determined heavy-lid rect — **axis-aligned device-space rect** via `fill_rect` (toonfull paints the equivalent as a head-colour `fill_polygon` through `hpt`, 1112; M4 has no tilt/squash so they coincide at lean 0) | Only `fill_rect` site in either painter's character path |
| blend_cov (direct) | 1 | 1138 head shade loop — differs from toonfull: unshaded term is a **circle distance** `(dist(px,cc) − rp·res + 0.5)` (1136), where toonfull uses the offset **ellipse** SDF (1004–1012) | Parity-relevant divergence between the two painters |
| blend (direct) | 1 | 1260 laugh teeth strip (same open-coded masked loop as toonfull 1211) | |

**Total: 36 textual sites** (≈52 executions/paint). Battery coverage
mapping: the battery exercises every class the full painter uses
**except** `fill_polygon_masked` (vest/stripes/apron — M4 has no
patterns), `composite_plate` (no plates), and `canvas_anchor`
(geometry-side, §6). Everything else — including both direct-blend
loops, both mask types, the laugh quirk, and the masked tongues — has
an M4 twin. Two known M4↔full divergences to keep on the parity list:
head-shade unshaded term (circle vs ellipse SDF, above) and the shock
pupil placement (centred vs gaze-offset).

---

## 9. Per-class roll-up

Executions assume a fully dressed paint (pattern + neckwear + hat +
extras + both plates); "gating breadth" counts distinct look/mood/
state values that can trigger the class at all.

| Class | Textual sites (full / M4) | Executions per paint (typical → max) | Gating breadth | Max overlap depth contributed | Parity-risk first impression (structural) |
|---|---|---|---|---|---|
| stroke_polyline | 35 / 9 | ~14 → ~48 (boater hatch ×~11, flower-adjacent rings, per-side loops) | Universal: every mood (mouth/lid variants), every legs state, 3 patterns/neck pieces, 9 hats | Ring strokes sit on top of their fills everywhere | **High.** The CPU stroke is a union-distance capsule with flattened-arc cap fans (scene.rs:601–695) — joins/caps are the subtlest geometry in the painter; the battery exists precisely for this class |
| stroke_quad | 12 / 3 | 4 → 6 (brows ×2 always; +blink or mouth) | Brows: all 19 moods; mouths: 8 mood branches + blink + monocle + scarf | Over brows/eyes/mouth — always on the head stack | Medium-high: it *is* stroke_polyline over a 24-segment flattening (scene.rs:697–700), so risk inherits the stroke class, plus flattening-count fidelity |
| stroke_ellipse | 17 / 5 | 2 → ~22 (flower petals ×6 pairs) | Head always; Shock/Awe, glasses, monocle, 8 hats | Outlines over fills (head, eyes, hats) | Medium-high: implemented as a 144-segment stroke_polyline (scene.rs:584–598) — a GPU ellipse-SDF stroke will differ at joins/curvature unless it reproduces the flattened path or proves equivalence |
| fill_polygon | 27 / 5 | ~6 → ~15 | Bean always; patterns, neckwear (3), angry teeth, mustache, sweat, 7 hats | Base layer for bean/neck/hat stacks | Medium: even-odd fill with SDF coverage (scene.rs:704–741) is conceptually simple, but half-ellipse-as-polygon hats (49-pt arcs) and chained-cubic mustache/laugh shapes punish any tessellation shortcut |
| fill_polygon_masked | 3 / 0 | 0 → 11 (stripes ×8) | 3 of 4 patterns | Pattern layer inside bean stack | Medium: two boolean centre-sampled masks ANDed per pixel (toonfull.rs:229–248) + plain `blend` — the *mask edge* is binary (no AA) while the polygon edge is also centre-tested; reproducing the exact aliased clip edge is the risk, not the fill |
| fill_ellipse | 26 / 9 | ~10 → ~30 (freckles ×6, petals ×6, tears ×2, buttons ×3) | Universal (head, pupils, feet, hands); all mood families; 6 hats | Everywhere — pupils on whites, hands on plates, pom/badge on crowns | **Low-medium.** Single analytic shape, SDF coverage via `cov_area` (scene.rs:569–583); the cleanest class to port — watch the approximate ellipse SDF (gradient-normalised, scene.rs:~824) at high eccentricity (tear 7×14, shade ellipses) |
| fill_ellipse_masked | 4 / 4 | 2 → 4 | Bean always (shade+highlight); open/laugh mouths | Shade/highlight inside bean; tongue inside mouth | Medium: same binary-mask caveat as fill_polygon_masked, but the underlying ellipse edge *is* AA'd against the mask test — mask/AA interaction at the bean and mouth rims is the parity surface |
| polygon_mask / ellipse_mask | 2+1 / 2+1 (+1 helper-internal per fill_polygon_masked) | 1 → ~14 mask builds/paint (stripes: helper rebuilds a mask per rect) | Bean (always), laugh mouth, open mouth | — (no paint) | **Cost risk, not pixel risk** (see §10): each mask is a full-canvas `Vec<bool>` = 2560×2816 ≈ 7.2 MB allocation + a bbox point-in-polygon sweep, zero-filled over the whole canvas even for a 96×72 tongue |
| composite_plate | 1 / 0 | 0 → 2 | `hold_l`/`hold_r` only | Plate between arm and hand | Medium: per-pixel inverse-lean + bilinear sample + `blend_cov` (284–356) — a texture sampling problem, GPU-natural, but the exact inverse map (anchor included) and bilinear rounding (`as u8` truncation at 350) must match |
| direct blend / blend_cov loops | 2 / 2 (+2 helper-internal) | 2 loops always-ish (head shade every paint; laugh teeth when Laugh) | Head shade: universal. Teeth: Laugh only | Head shade is layer 2 of the head stack; teeth layer 4 of the laugh mouth | Medium-high: both are open-coded per-pixel loops with non-standard coverage math (coverage *product* for head shade, 1010–1015; raw opaque `blend` under a binary mask for teeth, 1206–1213) — neither is a primitive call, so a class-by-class GPU port must special-case both |

**Deepest blend stacks (one pixel region, worst case):**

- **Head/eye region ≈ 6 layers:** bun fill+stroke (behind) → head
  fill → head shade (blend_cov) → pupil/eye-white → heavy-lid rect →
  glasses/monocle rim — plus head outline and hat brim crossing the
  top of the same region.
- **Laugh mouth ≈ 5 layers:** head fill → shade → mouth fill →
  tongue (masked) → teeth (blend) — with the tongue's stroke ring on
  top (the quirk, §4.5).
- **Bean torso ≈ 6 layers:** leg/arm strokes (under/over at edges) →
  bean fill → pattern (masked) → shade (masked) → highlight (masked)
  → neckwear or vest buttons → bean outline at the rim.
- **Held hand ≈ 4 layers:** arm stroke → plate → hand disc → (bean
  outline if the hand crosses the torso).

**Structural summary for INV-A:** two transform stacks (`px` body /
`hpt` head), one raster model (SDF coverage + source-over blend),
two binary-mask clip paths, two open-coded blend loops, one texture
composite. No text, no gradients, no shadows, no per-pixel state
beyond the masks. The porting surface is small; the parity surface
concentrates in (a) stroke joins/caps, (b) binary-mask edges,
(c) the two open-coded loops.

---

## 10. Per-frame sampled poses — what changes (context update, coordinator-verified at bc12a5c)

**The profile change.** Native projects are now authored with
per-frame sampled character poses in scene JSON: filmseq clones the
root per frame and sets `time`; the loader rejects clips
(scenefile.rs ~574). Unique character states per film rise by roughly
an order of magnitude, and the continuously varying channels — lean,
bob, arm angles, phase, look (gaze) — mean **nearly every paint is a
fresh full paint**: there is no longer a meaningful population of
repeated (look, state) pairs for a paint cache to hit, and no class
gets to be "the rare one" on a per-film basis. The per-class picture
below is structural; measured frequencies are the FREQ seat's brief.

### 10.1 Relative weight by class under sampling

| Class | Weight change under per-frame sampling |
|---|---|
| stroke_polyline (limbs, bean ring, arm strokes) | **Full rate, every frame.** Legs/arms are driven by the continuously varying channels (phase, arm angles, lean) — these sites can never be cached or skipped, and their geometry changes every frame even when the character "holds still" (bob/lean drift). Already the largest class by sites (35); sampling makes it the largest by executions too |
| stroke_quad (brows, mouths, blink) | Full rate: brows and mouth re-derive from mood/talk/blink every frame; gaze-driven pupil offsets ride along. No change in *which* frames trigger it — it was already per-paint — but no paint is ever a repeat |
| fill_ellipse (head, pupils, feet, hands) | Full rate, same reasoning: head disc, pupils (gaze moves continuously), hands (arm FK) repaint every frame |
| fill_polygon (bean, mustache, sweat, teeth…) | Full rate for the bean (bob/lean move it every frame); gated pieces follow their look gates as below |
| fill_polygon_masked / fill_ellipse_masked (patterns, bean shade/highlight, tongues) | **Whenever the sampled character wears the pattern** — sampling does not change the gate (pattern is a look property, constant across a character's frames), so a striped character pays the masked-pattern cost on *every one* of its ~10× frames instead of on a handful of cached poses. Bean shade/highlight masked ellipses are ungated: full rate, every character, every frame |
| stroke_ellipse (head outline, eye whites, hat parts) | Full rate for head/eyes; hat-gated sites follow the hat (below) |
| Hats / neckwear / extras classes (all classes, §5, §3.3, §4.3–4.6) | **Whenever the sampled character wears them** — gates are look properties, so per-film cost is (worn?) × (frames on screen). Sampling multiplies the second factor ~10×; it does not widen the first. A capped character's 6 cap sites now cost 6 × every frame, not 6 × distinct poses |
| composite_plate | Whenever a sampled character holds a prop — and the plate's *placement* varies every frame (hand FK + lean + anchor are continuous), so even a byte-identical plate is a fresh composite per frame; no transform caching survives |
| direct blend loops (head shade, laugh teeth) | Head shade: full rate, every frame, ~552×552 bbox sweep each time. Laugh teeth: per Laugh frame |

Net structural effect: **the gated/ungated distinction stops mattering
for optimisation priority.** Under the old pose-sparse profile one
could deprioritise a class that "only" fires for one hat; under
sampling, every class a film's cast wears runs at frame rate for the
whole film. The only classes that stay rare are those gated on
things no sampled character in the film wears at all.

### 10.2 Per-paint fixed overheads — the costs sampling exposes

These are costs paid **per paint regardless of how little geometry
changed**, so their share of total cost grows linearly with paint
count — under sampling they are ~10× more significant than the
inventory's per-class pixel work alone suggests:

1. **Mask allocations (highest concern).** `polygon_mask` /
   `ellipse_mask` (scene.rs:742–795) each allocate a **full-canvas**
   `Vec<bool>` — 2560×2816 ≈ **7.2 MB, zero-filled** — and sweep a
   point-in-polygon / ellipse test over the shape bbox. Per paint,
   the full painter builds: bean mask (801, always) + one mask per
   `fill_polygon_masked` execution via the helper's internal
   `polygon_mask` (230) — stripes alone = **8 mask builds** — +
   laugh-mouth mask (1202) or open-mouth ellipse mask (1180) when
   those mouths fire. Worst case ≈ 11 full-canvas mask allocations
   per paint (~79 MB of allocation churn) for a striped, laughing
   character. The masks' *content* is mostly a small bbox; the
   allocation and zero-fill are canvas-sized. A GPU (or optimised
   CPU-fallback) design should treat mask lifetime/extent as a
   first-class decision — bbox-sized masks or stencil-style clip
   would remove most of this; per §9 the binary edge semantics must
   be preserved either way.
2. **Head-shade loop bbox sweep (1014).** A per-pixel loop over a
   ~552×552 device-px bbox with two ellipse-SDF evaluations per
   pixel, every paint, ungated — pure fixed overhead under sampling.
3. **composite_plate bbox rasterisation (284–356).** Per held plate
   per paint: full bbox sweep with per-pixel inverse transform +
   bilinear sample, even for frames where the hand moved by a
   fraction of a pixel. No incremental path exists.
4. **Bean construction + mask sweep (788–801).** The 109-point bean
   path is rebuilt and its polygon mask swept over a ~370×540 bbox
   every paint; cheap relative to (1) but likewise uncacheable once
   bob/lean vary continuously.
5. **canvas_anchor envelope build (404–697).** Geometry-side (§6.1)
   and comparatively small (hundreds of envelope points), but it too
   runs per paint and cannot be keyed off a repeated state under
   sampling.

**Implication, stated structurally:** under the sampled profile the
win from GPU offload is *larger* than a per-pixel-cost model suggests,
because the CPU painter's per-paint fixed overheads (especially mask
allocation) scale with paint count and cannot be amortised by any
paint cache. Conversely, a GPU design that re-introduces a
full-canvas mask allocation (or a full-canvas readback) per paint
would recreate the dominant overhead in a new place — masks/clips
should live on-device, bbox-scoped, for the duration of one paint.

## 11. TS-side channels the Rust painter lacks — disposition (coordinator addendum, 2026-11-17)

Source: `~/workspace/tall-tale-v7-build/RUST-PARITY-SPECS.md` (v7
reconciliation, closed 2026-11-16; evidence findings rec-08 / rec-09 /
rec-09x in the same tree), verified against the shipped binary
(sha256 `24439786…`, HEAD `bc12a5c`) by a 2,152-cell FACE_TABLE replay:
2,314,476 relation pairs, EQ violated 362× (all mood `dizzy`), NEQ
violated 6,271×.

**Governing rule for this programme:** GPU-first's parity reference is
the **Rust CPU painter at bc12a5c**, not the TypeScript painter. Every
item below is therefore dispositioned as **pre-existing Rust-parity
debt that GPU-first does NOT absorb**: the GPU executor replicates the
Rust painter's behaviour (including its gaps), because implementing
TS-only behaviour on the GPU side alone would make GPU output differ
from the CPU reference and break the parity law and the Onoda census.
Each item names what happens *when* a separate Rust port lands — in
every case the GPU side inherits through existing primitive classes or
the host-side transform mechanism, with no new GPU primitive required.

| # | Channel (spec) | Rust status | Layer | GPU-first disposition |
|---|---|---|---|---|
| 1 | Wing appendage painter — declared segment counts + far-side wing (Spec 1) | **Absent** — no field/channel/prim; probe: wing keys byte-identical ignored | Painter | **Debt, not absorbed.** When ported: wing panels are `fill_ellipse` + ink stroke + shade lines — all existing prim classes; `wingSegs`/`wingA` join `state_key` element-wise (Spec §3b law). No GPU design change needed now. |
| 2 | Ambient blink layer — identity seed, 0.5–2.3 s cadence, trapezoid pulse, 2-frame lead, swap-masking (Spec 2) | **Partial** — a seeded blink exists in the film driver only (`charanim::blink_amount`, splitmix seed, 2.6–4.4 s gaps, 0.24 s bump); no lead, no swap-mask, no scene-path opt-in | Evaluator / staging | **Out of painter scope.** Blink reaches the painter as an existing scalar state input (already key-covered); the lid/pupil painting it drives uses existing eye prims. The delta port is evaluator work for the Rust-parity programme, invisible to the GPU executor either way. |
| 3a | `bodyPitch` + `bodyPitchCounter` — hip-pivot rotation of the upper assembly (Spec 3) | **Absent** — scene keys silently ignored (probe byte-identical) | Painter transform seam | **Debt, not absorbed — with a design note.** Pitch is a host-side point transform (rotate about `(bx, hipY)`), the same mechanism INTEG already mandates for `canvas_anchor` (host-computed transform uniform; WGSL never re-derives). If the Rust port lands, the GPU executor takes pitch through that uniform path; the per-pixel prims are unchanged. Two new `FullState` fields must join `state_key` explicitly at port time. |
| 3b | F-1 repaint-key threshold semantics (Spec 3b) | **Present** — `state_key` is exact f32 bit patterns; thresholds match (blink > 0.6, talk > 0.05) | Cache key | **No action.** Confirms the programme's standing law: any future `FullState` field (wing, pitch, sweat widening…) is added to `state_key` explicitly or the F-1 defect class is recreated. |
| 4 | Shortest-arc blending of the gait `phase` channel (Spec 4) | **Absent** — `eval15::sample_scalar_keys` and `anim::AnimValue for f32` lerp the long way across 0/2π (probe: midpoint renders as phase π, not 0) | Evaluator / interpolation | **Out of painter scope.** Phase reaches the painter as an evaluated scalar; how it was interpolated is invisible to both painters. Port belongs to the evaluator layer (`sample_scalar_keys` phase special-case); GPU executor unaffected. |
| 5a | `browLift` / `browTilt` post-table brow offsets (Spec 5a) — **largest family**: S3's 304 cells collapse to 19 Rust classes; 2,280 within-group NEQ pairs | **Absent** — brow block replicates row + per-side overrides, never adds the offsets | Painter (face) | **Debt, not absorbed.** When ported: offsets only move the existing brow stroke's control points (`lift += browLift`, `tilt += browTilt·s`, after per-side overrides) — the GPU stroke prims take the moved geometry unchanged. No clamp/quantisation may be introduced (the fixture's 0.5 steps distinguish). |
| 5b | Fox ears + `earA`/`earAL`/`earAR` (Spec 5b) — S9: 6 cells → 1 Rust class | **Absent** at every layer | Painter (head, behind disc) | **Debt, not absorbed.** When ported: two triangles per side = `fill_polygon` + ink stroke (outer) and `fill_polygon` (inner) — existing prims; `canvas_anchor` must add the ear-triangle points at port time (host-side, per INTEG's anchor rule). |
| 5c | Hair `hair` / `hairSegs` / `hairCol` (Spec 5c) — S10: TS 4 classes → Rust 1 | **Absent** — `hair_col` exists but feeds only the bow hat's side locks | Painter (head, behind disc) | **Debt, not absorbed.** When ported: strands are two-pass quadratic strokes (widths 17 ink / 11 colour) — the existing stroke machinery incl. CapFan flattening; `hairSegs` joins `state_key` element-wise. TS anchor does NOT envelope hair — mirror that, invent nothing. |
| 5d | `flip` whole-figure mirror (Spec 5d) — S12: TS 16 classes → Rust 8 (the halving is exactly flip) | **Absent** — point maps compose lean + anchor only | Painter transform | **Debt, not absorbed — with a binding design note for GPUX/INTEG:** flip is a stage-space reflection (`x → 512 − x`) composed **after** the anchor translation and **before** lean, mirrored identically in `canvas_anchor`. If the Rust port lands, the GPU executor's point-transform composition must place flip at exactly that point in the order; today neither painter emits it, so current GPU work is unaffected. |
| 5e | Sweat gradation — numeric alpha + `worried` forcing (Spec 5e) — S11-sweat: TS 16 classes → Rust 7 | **Reduced** — `FullState.sweat` is `bool`; painter fills at alpha 1 unconditionally; magnitude unrepresentable | Painter + state representation + `state_key` | **Debt, not absorbed — one GPU design constraint recorded:** the GPU sweat/drop prim must carry its blend alpha as a per-prim parameter (the executor's blend already parameterises alpha), so the eventual widening (`Option<f32>` + presence bit in `state_key`, `alpha = min(1, k)`, `k = sweat ?? 1` under forcing moods) needs **no GPU redesign**. GPU-first does NOT widen `state_key` itself — that changes production cache behaviour and belongs to the Rust-parity port. The STATES battery's sweat states exercise today's bool semantics, which is the correct reference for this programme. (Tears are NOT part of this gap: both painters read tears by truthiness; S11-tears passes exactly.) |
| 5f | Dizzy eye semantics (Spec 5f) — **DECISION ITEM D-5F, suspended** | **Divergent** — TS paints blink/gaze-inert spiral glyphs; Rust has no spiral glyph and paints dizzy as a pupil mood. All 362 EQ violations are dizzy | Painter (eye) | **Explicitly NOT dispositioned by GPU-first.** The 8 dizzy TS classes / 362 EQ pairs are suspended from Spec 5's acceptance bar pending a user-visible decision (spiral vs pupil). GPU eye prims replicate **Rust** behaviour (the programme's reference). If D-5F resolves to Option A, the spiral is a polyline stroke — an existing prim class — and lands via the Rust port first, GPU second, never GPU-first-only. |
| 5g | Deadpan micro-merges (Spec 5g) — blink 0.599 ≡ 0.6; gaze [−1,−1] ≡ [0,−0.99] | **Divergent at raster resolution only** — Rust formulas identical to TS; merges are sub-pixel (Δheight ≈ 0.008 stage units) | Rasterizer resolution | **Acceptance data for this programme's parity tables:** these are the known borderline family. If the same pairs surface in GPU-vs-CPU comparisons as EDGE/FRINGE-only diffs, they are to be reported as measured bounds of this family — not chased as GPU defects, and not silently absorbed either (§5g's own rule: an evidenced sub-pixel limit is a documented limitation). |

**Net effect on this programme:** zero new GPU primitives, zero scope
change, zero change to Batch 1–3 gates. The five painter-layer debts
(wing, bodyPitch, brow offsets, ears, hair, flip) all reduce to
existing prim classes or the host-side transform path when their Rust
ports land; the two evaluator items (blink layer, phase blending)
never touch the executor; sweat gradation imposes the single recorded
constraint (alpha as a per-prim parameter); dizzy stays suspended under
D-5F. The Batch 3 programme record must carry this table forward as
the named Rust-parity debt list so the debts are never mistaken for
GPU-first shortfalls — or silently fixed on the GPU side alone.
