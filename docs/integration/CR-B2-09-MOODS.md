# CR-B2-09 — GAP G2: `puzzled` + `deadpan` moods ported to the native painter

Seat B2-09 (Chrome-replacement Batch 2). Gap source: CR-B1-03 §5 G2 —
TS `MOODS` has 19 moods, `src/toonfull.rs` `Mood` had 17; `puzzled`
and `deadpan` silently fell back to neutral in every native render
(castsheet/previews included). This seat ports both, faithfully and
additively.

**Deliverables** (all under `~/workspace/chrome-replacement/batch2/B2-09/`):

| File | Role |
|---|---|
| `toonfull.rs.patch` | PRIMARY patch — the port itself (against repo HEAD `172052d`) |
| `film.rs.patch` | REQUIRED companion — `mood_name()` is an exhaustive match over `Mood`; without these two arms the crate does not compile after the primary patch |
| `charanim.rs.patch` | Mirror patch — `Expr` is documented as a 1:1 mirror of `Mood` ("they cannot drift silently"); extends it to 19 and updates its pinning test |
| `charstate.rs.patch` | Mirror patch — `MoodKey` is documented as a same-order mirror of `Mood` (discriminant parity); appends the two variants at the end, so all existing discriminants are unchanged |
| `harness/` | Standalone proof harness (see §4) |
| `results.txt`, `results-fidelity.txt`, `results-chunks.txt` | Raw harness output |

Nothing was committed and no repo file was edited in place: the port
was developed on copies in `B2-09/work/` and the patches were verified
to apply cleanly to pristine HEAD files and to reproduce the harness
after-copies byte-for-byte (`patch -p1` + `cmp`). Landing = apply
`toonfull.rs.patch` + `film.rs.patch` (the two mirror patches in the
same commit or a follow-up; neither is compile-required).

## 1. TS source of truth (quoted)

Skill: `~/workspace/custom-skills/tall-tale-shorts/template/src/core/toon.ts`.

Mood union + list (toon.ts:58, :66): 19 moods ending
`… | 'relieved' | 'puzzled' | 'deadpan'`.

**Brows** — base table (drawFace, toon.ts:696-698):

```ts
puzzled: [6, 4], deadpan: [2, 0]
```

Per-side override (drawFace brow loop, toon.ts:704) — puzzled only;
deadpan has none:

```ts
if (m === 'puzzled') { if (s < 0) { lift = 16; tilt = -2; } else { lift = -2; tilt = 8; } }   // one brow up, one brow down — the whole read
```

Brow stroke (same for all moods): `y = ey - 44 - lift`;
`moveTo(fx + s*(ex-26), y + tilt*0.5)`, `quadraticCurveTo(fx + s*ex, y - 7, fx + s*(ex+26), y - tilt*0.5)`, lineWidth 12.
Corroborated by `core/face.ts:587-588` BROW_TABLE: puzzled
`{ lift: 6, tilt: 4, note: 'fully asymmetric in the renderer: brows at 16/−2 and −2/8 per side' }`,
deadpan `{ lift: 2, tilt: 0 }`.

**Eyes** — `eye()` (toon.ts:679-684). Neither mood has an arc/spiral/
shock/awe branch; both reach the pupil:

```ts
g.ellipse(px, py, 13, 17 * (1 - bl * 0.6) * (m === 'suspicious' && side < 0 ? 0.55 : 1) * (m === 'deadpan' ? 0.8 : 1), 0, 0, 7); g.fill();
if (m === 'smug' || m === 'tired' || m === 'sly' || m === 'proud' || m === 'determined' || m === 'deadpan') {           // heavy lid over the top half
  const lid = m === 'tired' ? 6 : m === 'deadpan' ? 8 : 0;
  g.fillStyle = L.head; g.fillRect(x - 24, y - 28, 48, 26 + lid);
  g.lineWidth = 7; g.beginPath(); g.moveTo(x - 20, y - 2 + lid); g.lineTo(x + 20, y - 2 + lid); g.stroke();
}
```

So: **puzzled eye = plain pupil** (13 × 17·(1−blink·0.6), no lid, no
sparkle). **Deadpan eye = pupil with ry × 0.8, plus heavy lid with
lid offset 8** (rect from y−28 to y−2+8 = y+6; lid line at y+6).

**Mouths** (drawFace, lineWidth 8, `my = ey + 62`):

```ts
else if (m === 'tired' || m === 'dizzy' || m === 'deadpan') { g.beginPath(); g.moveTo(fx - 18, my + 6); g.lineTo(fx + 18, my + 6); g.stroke(); }
else if (m === 'puzzled') { g.beginPath(); g.moveTo(fx - 16, my + 5); g.quadraticCurveTo(fx, my + 1, fx + 16, my - 1); g.stroke(); }
```

No other drawFace/eye behaviour keys on either mood (grep-verified
over toon.ts: no sweat/tears auto-fire, no blush, no talk/shock/laugh
special-casing beyond the shared `t > 0.05` open-mouth rule, which the
native painter already applies mood-independently).

## 2. Native before (quoted, repo HEAD 172052d)

`src/toonfull.rs`: `Mood` enum has 17 variants (Neutral…Relieved);
`Mood::from_str` maps unknown strings to `Mood::Neutral` — so
`"puzzled"`/`"deadpan"` painted neutral faces under their own labels.
`Mood::brow()` has 17 arms; `heavy_lid()` =
`Smug | Tired | Sly | Proud | Determined`; the lid geometry inlines
`if m == Mood::Tired { 6.0 } else { 0.0 }` at four points; the pupil
ry is `17.0 * (1.0 - st.blink * 0.6) * slit * RES` with
`slit = suspicious-left ? 0.55 : 1.0`; the mouth match has a
`Tired | Dizzy` flat-line arm and no puzzled arm (unreachable `_`
arm = neutral smile).

## 3. The port (toonfull.rs.patch, 11 hunks)

Following the existing code patterns exactly; variants appended
**after `Relieved`** (TS MOODS order), so existing discriminants —
and film.rs `state_key`'s `{:?}` strings for the 17 — are unchanged:

1. Header comment 17 → 19 moods.
2. Enum: `Puzzled, Deadpan` appended.
3. `from_str`: `"puzzled" => Mood::Puzzled`, `"deadpan" => Mood::Deadpan`.
4. `brow()`: `Puzzled => (6.0, 4.0)`, `Deadpan => (2.0, 0.0)`.
5. `heavy_lid()`: `| Mood::Deadpan` added; new `lid_offset()` helper
   (`Tired => 6.0, Deadpan => 8.0, _ => 0.0`) — the TS `lid` ternary,
   factored once instead of inlined four times.
6. Brow loop: puzzled per-side override after the suspicious one,
   verbatim values (s<0 → lift 16, tilt −2; s>0 → lift −2, tilt 8).
7. Pupil: `let deadpan = if m == Mood::Deadpan { 0.8 } else { 1.0 };`
   multiplied into ry beside the existing `slit` factor — the TS
   factor order (suspicious factor × deadpan factor; the two never
   co-fire, so order is moot).
8. Lid geometry: the four `if m == Mood::Tired { 6.0 } else { 0.0 }`
   become `m.lid_offset()` — value-identical for every existing mood
   (Tired 6.0; Smug/Sly/Proud/Determined 0.0).
9. Mouth: `Mood::Tired | Mood::Dizzy | Mood::Deadpan` flat line
   (fx−18, my+6)-(fx+18, my+6), lw 8 — TS groups them identically.
10. Mouth: new `Mood::Puzzled` arm,
    `stroke_quad(hpt(fx-16, my+5), hpt(fx, my+1), hpt(fx+16, my-1), lw(8))`
    — `stroke_quad` is the painter's quadraticCurveTo.

`film.rs.patch`: `mood_name()` gains `Puzzled => "puzzled"`,
`Deadpan => "deadpan"` (exhaustive-match completeness).
`charanim.rs.patch`: `Expr` gains both variants everywhere the 17
live (enum, `ALL` 17→19, `from_str`, `as_str`, `brow`,
`heavy_lid` incl. Deadpan) + its vocabulary test updated (19 names,
new brow/lid pins). `charstate.rs.patch`: `MoodKey::Puzzled/Deadpan`
appended (order/discriminants of the 17 preserved).

Not touched, deliberately: `scene.rs` `ToonMood` (the separate
5-mood §28 subset — not the full painter, out of G2's scope);
`scenefile.rs` (parses mood via `Mood::from_str` — gains both moods
automatically); film `state_key` (uses `{:?}` — new variants key
 distinctly with no code change).

## 4. Proof (harness, geometry + pixel level, no GPU)

`B2-09/harness/` is a dependency-free cargo bin that compiles the
**real repo `scene.rs`** (production CPU Painter, verbatim) plus
verbatim copies of toonfull.rs BEFORE (HEAD) and AFTER (patched) as
two modules in one binary, and paints through the real
`paint_toon_full`. Local toolchain: cargo/rustc 1.99.1
(`~/.cargo/bin`), release build. (First full run was killed by a VM
restart at ~11 min; the harness was then chunked per mood with
per-case output persisted to disk, and completed in chunks.)

**(a) Additivity — 170/170 cases BYTE-IDENTICAL, 0 mismatches.**
Matrix: the 17 original moods × 7 states (default; posed
look/tilt/turn/pitch/squash/lean/bob; blink 0.8 + talk 0.7; walk;
run + sweat; jump + tears; sit) × plain look (all 7 states) + full
look (vest/cap/tie/glasses/mustache; 3 states) = 170 cases, each
comparing the complete 2560×2816 RGBA canvas bytes and both hand
positions BEFORE vs AFTER. Per-case FNV-1a hashes (all equal in
pairs) in `results.txt` / `results-chunks.txt`; every chunk exited
rc=0 with 0 failures.

**(b) Fidelity — 21/21 checks PASS** (`results-fidelity.txt`).
Every TS value compared, measured on the AFTER painter's actual
pixels at the default state (fx=256, ex=46, ey=182, my=244 stage):

| TS value | Measured |
|---|---|
| puzzled brow s<0 quad (lift 16, tilt −2) centroid | pred (210.00, 119.76) vs meas (209.67, 119.50) |
| puzzled brow s>0 quad (lift −2, tilt 8) centroid | pred (302.00, 137.76) vs meas (301.44, 137.55) |
| puzzled brow asymmetry (lift diff 18) | centroid diff **18.09** |
| puzzled mouth quad (−16,+5)(0,+1)(+16,−1) centroid | pred (256.00, 245.68) vs meas (255.76, 245.66) |
| puzzled mouth slope / full pupil (ry 17, no lid) | ink probes at (fx+14, my−1), (fx+46, ey+15), (fx+46, ey−12) all as TS |
| deadpan brow (2, 0) both sides centroid | pred y 133.76 vs meas 133.52 (both sides) |
| deadpan pupil ry = 17×0.8 = 13.6 | ink at ey+13, no ink at ey+15 (neutral: ink) — brackets ry ∈ (13, 15) |
| deadpan lid rect over pupil top | no ink at (fx+46, ey−12); head colour there (neutral: ink) |
| deadpan lid offset 8 → line centre ey+6 (tired: 6 → ey+4) | line centres **187.75** vs TS 188; tired **185.75** vs 186; diff **2.00** |
| deadpan mouth flat line (fx±18, my+6) | ink at centre/±16 on my+6; no ink at (fx, my+11) where neutral's sagging smile has ink |
| distinctness | full-canvas hashes: puzzled `5cd1ffee…`, deadpan `3b5ba793…`, neutral `fe72e37b…`, tired `380decc4…` — all distinct |

Also proven: BEFORE `from_str("puzzled")`/`from_str("deadpan")` =
Neutral (the bug); AFTER they resolve to the new variants with
Debug names `Puzzled`/`Deadpan`.

**(c) Mirror patches compile-tested standalone:** charanim (its own
doc's method, `rustc --test`): **12/12 tests pass**, including the
updated 19-name vocabulary test. charstate in a check crate with
the real `dirtyrect.rs`: **38/38 tests pass**. `film_after.rs`:
zero parse errors standalone (only expected unresolved-import /
missing-`include_str!` errors outside the crate). All four patches
apply cleanly to pristine HEAD files and reproduce the tested
after-copies byte-for-byte.

## 5. Residual gaps / notes for the lead

- **No residual TS behaviour was found inexpressible** in the
  native painter: every puzzled/deadpan primitive (quad brow/mouth
  strokes, ellipse pupil, polygon lid rect, polyline lid/mouth) is
  one the painter already has. Nothing was approximated.
- Pre-existing divergences NOT in this seat's scope and unchanged
  by the port: TS v6 `browLift`/`browTilt` state channels and the
  TS `dizzy` spiral eye have no native counterpart for *any* mood.
- After landing, the skill-side generator flag should be cleared:
  `template/tools/castsheet_native.py` hardcodes
  `nativeMoodFallback: "neutral"` for these two moods (B1-03) —
  after this port lands it can emit both mood previews natively
  (41 → 43 of 77 catalog items fully native; G1 held-props remains
  the larger blocker).
- Render-level proof (byte-probe / film census / CI) still rides
  the lead's landing gate, per the wave's division of labour; the
  harness proves the painter's output bytes for the 17 old moods
  are unchanged and the two new faces match TS geometry on the
  production CPU rasteriser, which is the same code the GPU path
  uploads as character cards.
