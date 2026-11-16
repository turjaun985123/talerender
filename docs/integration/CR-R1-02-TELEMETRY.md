# CR-R1-02 — Generic telemetry (`src/filmtelemetry.rs`)

Seat R1-02, remediation Batch 1. Deliverable: `filmtelemetry.rs`
in this directory (new file only; nothing edited, nothing
committed). Lead moves it to `talerender/src/filmtelemetry.rs`,
adds the `mod` line, and wires the §1 fork (§7 below).

Verified against code at talerender HEAD `aa3de63` (L0 landed:
`scenefile::load_root` + `render_loaded_pixels` present) and
against the consumer at
`~/workspace/custom-skills/tall-tale-shorts/template/tools/native_modes/check_mode.mjs`
(read in full for this note).

Entry point, exactly as chartered:

```rust
pub(crate) fn run_telemetry_seq(
    manifest: &crate::filmseq::FilmManifest,
    data: &crate::film::FilmData,
    tables: bool,
) -> Result<bool, String>
```

`Ok(true)` = CHECK OK, `Ok(false)` = CHECK FAILED, `Err` =
telemetry could not run — `film::run_film_telemetry`'s exact
return contract, so main.rs's exit-code mapping (0 / 1 / 3)
applies unchanged.

---

## 1. The consumer contract — the three regexes, verbatim

Copied character-for-character from `check_mode.mjs`
(lines 102–104 at the time of reading):

```js
const SHOT_ROW = /^(\S+)\s+\d+-\d+\s+(\d+\.\d+)-(\d+\.\d+)\s+(\d+\.\d+)\s{2,}(\S*)\s+(\S+)\s+(.*)$/;
const CAP_ROW = /^(\d+)\s+(\d+)\s+(\S+)\s+(\d+\.\d+)-(\d+\.\d+)\s+(\d+\.\d+)\s+(\d+)\s+(\d+)\s{2}(\S*)\s+"(.*)"$/;
const CAST_ROW = /^(\S+)\s+\d+\s+\d+\s+\d+\s{2}(\S.*)$/;
```

How `check_mode.mjs` consumes the output (its `runCheck`):

- It spawns `talerender --film-telemetry <filmDir> --tables`.
- The verdict is the first line starting `FILM TELEMETRY: CHECK`;
  no such line → exit 3 territory (stale binary / refused data).
- Audit lines = every line **before** the verdict except lines
  starting `FILM TELEMETRY —`, lines starting `paint-cache:`, and
  blank lines. They pass through byte-for-byte except the
  `CAPTION AUDIT:` summary (authored count corrected from
  script.json when present, via
  `/^CAPTION AUDIT: (\d+) chunks, \d+\/\d+ lines authored, min fit (\S+), max rendered width (\S+) px$/`),
  the `CAPTIONS: estimate-grade` line (same correction), and
  `CAPTION (WIDTH|FIT) FAIL:` → `WARN:` downgrade when
  `CAP_CHECK` is not `strict`.
- Tables are parsed from lines **after** the verdict:
  `reformatTables` tracks the current section by
  `startsWith('SHOT TABLE' | 'CAPTION TABLE' | 'CAST LIST')` and
  applies the regexes above. Group use: SHOT_ROW → id=1, at=2,
  end=3, dur=4, set=5, atmo=6; CAP_ROW → s=4, e=5, text=10;
  CAST_ROW → id=1, and any cast-section line containing `beats:`
  is excluded from the cast id list.
- The FAILED verdict's counts are parsed back out with
  `/CHECK FAILED \((\d+) overflow groups, (\d+) continuity violations, (\d+) caption failures\)/`;
  caption failures only fail the check under `CAP_CHECK=strict`.

**Empirical verification (seat, this session):** the module's
exact Rust format strings were replicated and run against the
three regexes above (Python `re`, same syntax for these
patterns). Results: SHOT_ROW matches s01/s11 rows and the
empty-set row, groups in the consumer's order
`(s01, 0.00, 4.59, 4.59, BCH, dusk, …)`; CAP_ROW matches clean
and `WIDTH+FIT` rows, group 4/5 = window start/end, group 10 =
text (including text containing embedded quotes — greedy `.*`
to the last quote, same as legacy); CAST_ROW matches the cast
row and does **not** match the `beats:` line (also excluded by
the consumer's own `beats:` check); none of the three column
header lines matches its section's regex; the CAPTION AUDIT
summary and FAILED-verdict shapes match the consumer's two
secondary regexes.

## 2. Line-by-line conformance map

Every line the module prints, in order, and where it comes from.
Legacy source = `film::run_film_telemetry` + `telemetry.rs`
unless noted.

### Before the verdict (stdout)

| # | Line shape | Emitted by | Consumer treatment |
|---|---|---|---|
| 1 | `FILM TELEMETRY — {manifest.title} (native, 0.25 s sampling)` | `run_telemetry_seq` header; legacy prints the Onoda title as an uppercase literal — generic prints the manifest title **as authored** (finding F4) | Excluded from audit lines (prefix rule); only the `FILM TELEMETRY — ` prefix is contractual |
| 2 | `CANVAS OVERFLOW: {id} ({desc}) {side}={n} … — pose {pose}` per group, else `CANVAS OVERFLOW: none` | `telemetry::Audit::print_audit(false, …)` — the shared engine, fed generic cutouts | Audit line, pass-through |
| 3 | `EDGE TOUCH (warning): …` per entry, else `EDGE TOUCH: none` | same | Audit line, pass-through |
| 4 | `CONTINUITY VIOLATION: {id} in {seg} — changed: {fields}`, else `CONTINUITY: clean ({n} named characters tracked)` | `Audit::print_continuity` | Audit line, pass-through |
| 5 | `CAPTION ZONE (warning): …`, `SAFE AREA (warning): …`, `FRAMING (warning): …`, `COMPOSITION (advisory): …`, closing `COMPOSITION: {t} findings (…)` | `Audit::print_composition` | Audit lines, pass-through; advisory, never in verdict (legacy law) |
| 6 | `CAPTION AUDIT: {N} chunks, {A}/{B} lines authored, min fit {X}, max rendered width {Y} px` | `telemetry::print_caption_audit(&caps, true, …)` — the shared function, strict exactly as film.rs calls it | Matched by the consumer's correction regex; A/B recomputed from script.json when present. Generic A/B comes out `0/{lines}` because film captions are constructed `auto: true` — **identical to the legacy film path**, which is why the consumer correction exists |
| 7 | `CAPTIONS: estimate-grade (auto-chunked) on lines {…} — author caps in script.json for a master` (when any auto lines) | same function | Corrected / dropped by the consumer from script.json, as legacy |
| 8 | `CAPTION WIDTH FAIL: line {n} chunk "{text}" renders {w} px (> 850) at t={s}s — …` / `CAPTION FIT FAIL: … fit {f} (< 0.68; raw width {rw} px) …` | same function (strict tag) | Downgraded to WARN by the consumer unless `CAP_CHECK=strict`; counted in the verdict's caption failures by the binary either way — legacy parity |
| 9 | `CAPTION TIME (advisory): line {n} chunk "{text}" shows {d} s for {w} words ({p} s/word < 0.33) at t={s}s — advisory only (ruling R5); not counted in the verdict` | **Generic-only addition** (`caption_time_advisories`); see §4 | Audit line, pass-through; no consumer pattern matches it, and it never enters any count |
| 10 | `GENERIC TELEMETRY NOTE: {n} held-plate sample(s) painted without plate pixels …` — only when n > 0 | `run_telemetry_seq`; see §3 defence | Audit line, pass-through |
| 11 | `FILM TELEMETRY: CHECK OK` or `FILM TELEMETRY: CHECK FAILED ({o} overflow groups, {v} continuity violations, {c} caption failures)` | `verdict_line()` — wording copied from film.rs's format string | The verdict: consumer's anchor + count regex |

Deliberately **absent**: the legacy `paint-cache:` line
(`println!("{}", cache.stats_line())` before the report). The
generic path paints uncached (scenefile parity — `walk_nodes`
paints every character per load), so there is no cache to
report; the consumer only *filters* such lines, and its absence
changes nothing it parses (finding F3). Progress lines
(`film-telemetry: sample {n} (t={t:.2})` every 20 samples) go to
**stderr**, mirroring the legacy loop, and are never parsed.

### After the verdict (only with `tables = true`)

Printed from a separate line vec after the verdict lines, so
the default output is a strict prefix of the `--tables`
output — the legacy invariant (film.rs W85 comment).

| Section | Lines | Source of values |
|---|---|---|
| (blank line) | `""` | legacy shape |
| `SHOT TABLE — {n} shots, {total_frames} frames @ {fps} fps, {duration:.2} s` | header; legacy hardcodes the 30 fps / duration-derived frame count — generic uses the manifest's own `fps` and `totalFrames` | manifest |
| `{:<6}{:<13}{:<16}{:>6}  {:<5}{:<11}characters` | column header (legacy format verbatim; does not match SHOT_ROW) | — |
| `{id:<6}{start_frame-(end_frame-1):<13}{start_s:.2}-{end_s:.2}:<16}{dur:>6.2}  {set:<5}{atmosphere:<11}{characters}` | one per segment. Frame range printed inclusive (`end_frame - 1`) because the manifest's `endFrame` is exclusive (LC s01: 0..138 → `0-137`) while the legacy ranges are inclusive. Characters = this run's sampled presence for the segment (`{id}` or `{id} x{max_simul}`), `(none)` when empty | manifest segments + `PresenceRow`s |
| `CAPTION TABLE — {n} chunks (audit bars: rendered > 850 px = WIDTH fail, fit < 0.68 = FIT fail; strict)` | header (legacy text verbatim) | captions |
| `{:<5}{:<6}{:<6}{:<17}{:>5}{:>6}{:>7}  {:<10}text` | column header (legacy verbatim; does not match CAP_ROW) | — |
| `{idx:<5}{line+1:<6}{owner:<6}{s:.2}-{e:.2}:<17}{fit:>5.2}{rendered:>6.0}{raw:>7.0}  {flag:<10}"{text}"` | one per caption; owner = `telemetry::shot_at` over segment starts (the legacy attribution rule); flag from the audit's own literals (`> 850.0`, `< 0.68`) | FilmData captions + measured widths (§4) |
| `note: chunk {i} ({owner}) straddles the cut — its window runs {x:.2} s into the next shot; the driver draws it across the cut (window rule)` | for captions crossing a segment boundary (legacy note, wording's film.rs citation generalised) | captions × segments |
| `CAST LIST — {n} identities (frame counts are sampled presence: 0.25 s audit samples x {fps/4} frames per sample)` | header; at 30 fps the parenthetical is the legacy `x 7.5` text's value | presence |
| `{:<9}{:>6}{:>8}{:>7}  shot list / authored blink beats` | column header (legacy verbatim; does not match CAST_ROW) | — |
| `{id:<9}{shots:>6}{frames:>8}{beats:>7}  {shot list}` | one per identity, first-appearance order (legacy sort). `frames` = sampled count × fps/4, rounded — at 30 fps identical to the legacy `(samples*15+1)/2`. `beats` = **0**: scene data carries no authored blink-beat bindings (the legacy table's bindings are transcribed from Onoda staging code — contracts §0/W85 D7); an honest 0, not an invented binding | presence |
| `{:<9}{:>6}{:>8}{:>7}  beats: -` | second line per identity, legacy shape with the legacy empty-list dash; excluded from the cast ids by the consumer's `beats:` rule (and does not match CAST_ROW) | — |

## 3. Overflow / continuity — design and defence

**The question:** the legacy gate builds each sample's
`Cutout`s from `eval_frame` + `resolve_chars`: the *real painted
canvas* of each staged character (PaintCache → `paint_toon_full`)
scanned by `scan_canvas_overflow(.., 2.0)`, the card's *real
model matrix* projected by `project_box`, and the registry
look's signature. The generic path has no staging and no
registry. What is the honest analogue?

**What the loaded scene supports:** `scenefile::Loaded` is
opaque — every field (`camera`, `draws`, `textures`, …) is
private to `scenefile`, and `Draw`/`Camera` are private types.
There is **no** accessor from which a second module could read
character rectangles out of a `Loaded`. An exact
"cutouts-from-Loaded" analogue therefore **does not exist** at
HEAD `aa3de63`, and this note says so plainly rather than
pretending otherwise.

**The analogue built instead** — same inputs the loader itself
consumes, same computations downstream:

1. Per sample (0.25 s, legacy loop shape), take the owning
   segment (manifest time ranges), clone its parsed scene root,
   set `time := lt` (f32 law, contracts §2).
2. **Validate** through `scenefile::load_root(root, label)` —
   the loader must accept the scene at that `lt`; any error is a
   named `Err` for the whole run (fail closed, era pairing).
   This is not decorative: it is what makes the raw-root reading
   below trustworthy, because every field the walk reads has
   just been type-checked by the loader's own req/num posture.
3. Fold with `eval15::process_scene(root, &[])` — the identical
   fold `load_root` performs first — so §15 clip-evaluated
   character state at `lt` is what gets measured, not the
   un-evaluated file state.
4. Walk the folded root's nodes (groups recursed, transforms
   composed by a line-for-line mirror of
   `scenefile::compose_transform` over `crate::math`). For each
   `character` node, construct `FullLook`/`FullState` with a
   field-for-field mirror of `walk_nodes`' character branch —
   same defaults (`body #E8734A`, `head #F7F7FA`, arms `(8,6)`,
   `sitOn × 1024/h_t`, …) — and paint with the **same painter**
   `paint_toon_full`. Overflow = `scan_canvas_overflow(px, w, h,
   2.0)` on that canvas: the identical function, canvas size
   (2560×2816) and strip law as the legacy gate.
5. Box: faced model built as the loader builds it
   (`parent · T(pos) · RotY(yaw) · RotX(pitch)`, f32, yaw/pitch
   from the loader's own formulas against the camera evaluated
   at `lt` — camera keys evaluated by a mirror of
   `eval_camera_keys`, smootherstep + `linear` ease), converted
   by `telemetry::model_from_f32` and projected by
   `telemetry::project_box`. One deliberate difference: the
   projection height is the node's declared `height` (default
   1.75). The legacy call passes the literal `1.75` — Onoda's
   height transcribed; `plane_bbox(height)` is defined from the
   toon's own height, and the scene quad itself is built from
   the node's height, so the generic form is the same rule with
   the film-specific literal replaced by the character's data.
   For any 1.75 character the two are bit-identical.
6. Continuity: identity = the character node's authored `name`
   (the cast name in scene data; LC: `Biscuit` in all 11
   segments, `Fisher` in s10). Signature =
   `telemetry::look_signature` over a `telemetry::Look` built
   from the **authored** look (extras whole — cast.ts's
   `lookSignature` reads the authored look, while the painter
   drops extras it does not paint; the signature's job is
   identity, so it follows cast.ts). `desc` =
   `telemetry::describe`, pose = `telemetry::pose_summary` over
   the mirrored state (mood/legs names copied from film.rs's
   private `mood_name`/`legs_name`). All fed to the **shared**
   `telemetry::Audit` — same dedupe keys, fig/face fractions,
   edge/composition derivations, and verdict expression as the
   legacy gate.

**Why this still catches the same defect classes:** canvas
overflow is a property of the painted canvas, and the canvas
here is produced by the same painter from the same look/state
values the loader itself derives — a pose that paints past the
canvas edge in the render paints past it here. Edge-touch /
caption-zone / framing are properties of the projected plane,
projected by the same function from the same faced model the
renderer draws. Continuity is a property of the authored look
per identity across time, read from the same authored data the
renderer consumes. The one measured quantity that can differ
from a full render is held-plate ink (next paragraph) — and
plates are anchored at the hand, interior to the canvas, so
the affected defect class is a plate itself crossing the
canvas border, which the NOTE line discloses per run.

**Held plates (the disclosed gap):** the audit has no GPU and
does not run scenefile's texture painters, so a plate is
resolved only when its scene texture is a `file` PNG beside
the scene (read with the `image` crate, `x0/y0` from the hold
spec — `scenefile`'s `plate_of` semantics). A generator-declared
plate is painted **without** its pixels and counted; the count
prints as the GENERIC TELEMETRY NOTE line (§2 #10) before the
verdict. An *undeclared* plate id never reaches the walk:
`load_root` fails closed on it first (step 2).

**Unnamed characters:** a character node without a `name`
cannot be bound across segments; it is tracked as
`{seg_id}#char{n}` — present for overflow/presence in its own
segment, never falsely continuous and never falsely clean
across segments. LC has none (all characters named).

**Cost honesty:** `load_root` per sample re-derives textures
and repaints characters per sample (the legacy gate repaints
through its cache instead). For LC: 157 samples × 11 scenes'
worth of loads ≈ the legacy gate's own order of CPU. It is a
gate, not a render; no streaming or persistent-renderer
machinery is borrowed for it.

## 4. Caption audit — estimator, floors, advisory

**Width estimator (named, as chartered):** the legacy film
audit's estimator is **the ink bounding box of the film's own
rendered caption strip PNGs** — film.rs `run_film_telemetry`
caption block: for each caption, scan `assets.strips[i]` for
alpha > 32, `rendered_w = x1 − x0 + 1`, `raw_w = rendered_w /
fit`. It is *not* a font measurer (a native text measurer is
the R1-10 / contracts §7 concern, feeding the bridge's `fit`
values). The generic module reuses the identical estimator and
arithmetic (`strip_ink_width` is the same loop), loading the
strips directly from `<film_dir>/assets/<strip>` instead of
via `film::load_assets`. The narrower perimeter is deliberate:
post-decoupling (contracts §4) a film may declare no calendar
years / dark plate / sign, and `load_assets` at HEAD still
demands those files — the caption audit must not fail a
decoupled film for assets it never declared. A missing strip
file is a named `Err` (fail closed), as in the legacy path.

**CaptionInfo construction:** `line` by film.rs's own rule
(last line with `start <= s + 0.05`); `auto = true` for every
caption, exactly as the legacy film path constructs them (film
data carries no authored flag; the consumer corrects authored
counts from script.json — §2 #6).

**Floors:** `print_caption_audit(&caps, true, …)` — strict,
the legacy film call — enforces rendered width ≤ 850 px and
fit ≥ 0.68 as FAILs counted into the verdict. The module adds
no second implementation of these floors anywhere except the
caption table's flag column, which duplicates the literals
deliberately (the legacy tables' own W85 §1.2 freeze posture).

**Display-time advisory:** the legacy *Rust* audit prints no
per-chunk time lines; the floor exists as ruling R5 in
`telemetry::print_caption_audit`'s doc comment ("the ≥0.33
s/word time floor is NOT enforced — the width floor wins").
The generic module surfaces that same floor as explicit
`CAPTION TIME (advisory):` lines (words = whitespace split of
the chunk text; fires when `(e − s) / words < 0.33`). Advisory
lines are never counted — the verdict expression is untouched
(§6). This is a generic-path addition, disclosed here, not a
legacy behaviour claimed as copied.

## 5. Cross-module items consumed, with visibility at `aa3de63`

| Item | Visibility at HEAD | Note |
|---|---|---|
| `crate::filmseq::FilmManifest` + segment type + `scene_path(&seg)` | **not yet in tree** (R1-01 is a parallel seat; `src/filmseq.rs` absent at `aa3de63`) | Consumed exactly per the R1-01 API in the seat charter; assumptions listed in §8 for lead confirmation |
| `crate::film::FilmData` (fields `captions`, `lines`), `crate::film::Caption` (`s/e/text/fit/strip`) | `pub(crate)` struct, `pub` fields | `duration` deliberately **not** used: the manifest is the timing authority on the generic path (contracts §3) |
| `crate::scenefile::load_root(root, label)` | `pub(crate)` (L0) | Validation gate per sample |
| `crate::eval15::process_scene(root, &[])` | `pub` | The loader's own §15 fold |
| `crate::scene::Value`, `crate::scene::parse_json` | `pub(crate)` | Scene roots, manifest-shaped test parsing |
| `crate::telemetry::{Audit, Cutout, CaptionInfo, ShotInfo, PresenceRow, Look, ProjCamera, print_caption_audit, scan_canvas_overflow, project_box, model_from_f32, look_signature, describe, pose_summary, PoseState, shot_at, note_presence}` | all `pub(crate)` | The shared audit engine — no audit logic is reimplemented |
| `crate::toonfull::{paint_toon_full, FullLook, FullState, Legs, Mood, Plate}` + `Mood::from_str` | `pub(crate)` | The same painter the renderer and legacy gate use |
| `crate::math::{Mat4, identity, mul, translate, rotate_x, rotate_y, euler_xyz, scale}` | `pub` | Transform composition mirror |
| `image` crate (`image::open`) | existing dependency (film.rs `load_png` uses it) | Strip + plate PNG reads |

**Copied instead of consumed (private at HEAD, named here so
the lead can hoist them to a shared home later if desired):**
film.rs `mood_name` / `legs_name` (verbatim match arms);
film.rs strip ink-bbox loop (verbatim arithmetic, §4);
scenefile `compose_transform` law, `eval_camera_keys` law, and
the character branch's look/state defaults (mirrored, each
cited in a code comment). `film::tel_look`'s `Box::leak`
pattern is replaced by a per-run `Interner` (leaks each
distinct authored string once per run, not once per sample).

**Nothing needs a visibility change for this module to land**,
beyond R1-01's own `filmseq` landing and the `mod` line.

## 6. Verdict and failure posture

- Verdict counts = `audit.overflow_len() +
  audit.violations_len() + cap_fails` — the legacy expression,
  with composition findings advisory-only (inside `Audit`,
  unchanged) and caption-time advisories excluded by
  construction.
- `Err` (exit 3 via main.rs) on: empty segment list; a
  segment's declared caption range outside the film's caption
  list (`check_caption_ranges` — a corrupt bridge named, not
  misreported); an unreadable/unparseable segment scene; a
  scene `load_root` rejects at any sampled `lt`; a missing
  caption strip. No sample is ever silently skipped and no
  audit is ever printed as `incomplete` — the generic path
  either audits every sample or does not run.

## 7. Lead wiring spec — the §1 fork in `run_film_telemetry`

`film::run_film_telemetry(dir, tables)` (film.rs; contracts
cite :3992) gains the manifest fork **before** any film data
is loaded, mirroring the `run_film` fork R1-01 specifies:

```rust
pub(crate) fn run_film_telemetry(dir: &Path, tables: bool) -> Result<bool, String> {
    // §1 fork: manifest discovery BEFORE film data loads.
    // Candidates, in order: dir/../film.json, dir/film.json,
    // dir/../scenes/film/film.json  (filmseq::find_manifest,
    // R1-01). Parse if found.
    if let Some(manifest) = crate::filmseq::find_manifest(dir)? {
        if manifest.driver.as_deref() == Some("scene-sequence") {
            let data = load_film_data(dir)?;   // captions/lines only, on this path
            return crate::filmtelemetry::run_telemetry_seq(&manifest, &data, tables);
        }
    }
    // Legacy path: unchanged staging below, PLUS the §1.4
    // legacy-entry guard (anchor-set check -> named error,
    // never the `no entry found for key` panic).
    ...
}
```

Plus, in `main.rs`: `mod filmtelemetry;` beside `mod film;`
(alphabetical neighbourhood of the existing mod block,
main.rs:52–59). No changes to main.rs's dispatch: the
`--film-telemetry` arm already maps `Ok(true)/Ok(false)/Err`
to exit 0/1/3 for whichever path the fork took.

Manifest field expectations on this path: `manifest.film_dir`
must be the film-data dir (the `--film-telemetry` argument —
`<native>/film` in project layout), since strips load from
`film_dir/assets/`; `manifest.manifest_dir` is carried for
R1-01's driver and unused here.

## 8. Assumptions the lead must confirm against R1-01 as landed

The module compiles against the charter's R1-01 API. If the
landed `filmseq` differs on any of these, the adaptation is
mechanical and confined to the spots named:

1. Segment type name (test constructor only — module code
   never names it; it iterates `manifest.segments` and reads
   fields through the bindings).
2. Segment fields: `id: String`, `scene: String`,
   `lines: Vec<usize>`, `start_s/end_s: f64`,
   `start_frame/end_frame: u32`,
   `caption_first/caption_last: usize`, **`set: String`,
   `atmosphere: String`** (from the manifest JSON's `set` /
   `atmosphere` keys — present in both real manifests; consumed
   in `shot_table_lines` and the test constructor only).
3. Manifest fields: `title: String`, `fps: f64`,
   `duration_s: f64`, `total_frames: u32`,
   `driver: Option<String>`, `manifest_dir: PathBuf`,
   `film_dir: PathBuf`.
4. `scene_path(&seg)` returns `PathBuf` (used as
   `let path = manifest.scene_path(seg);`). If it returns
   `Result<PathBuf, String>`, add `?` at that one call site.
5. `fps` as `f64`: used in `fmt_fps` and the cast-list
   frames-per-sample product. If R1-01 lands `fps: u32`, both
   spots take `manifest.fps as f64`.

## 9. Findings (named, per contracts §3's honesty rule)

- **F1 — the current LC bridge does not pass `load_root` on
  two segments.** Evidence (seat, this session, from the files
  on disk): `s03.scene.json` declares textures `{wall, floor}`
  but its Biscuit node holds `holdR: {plate: "lens-cloth"}`;
  `s08.scene.json` declares `{sand, sky}` but holds
  `oil-can`. `scenefile`'s `plate_of` errors on an undeclared
  plate id, so `load_root` fails on s03/s08 at any `lt`, and
  the generic telemetry therefore returns a named `Err` on the
  LC project **as currently bridged**. This is a bridge-data
  defect (the bridge's own BRIDGE-STATUS lists caption-strips
  among its owed items), squarely in R2-03's re-bridge scope;
  the telemetry's fail-closed posture is the contract's, and
  this note names it instead of working around it (e.g. by
  skipping validation, which was considered and rejected —
  §3 step 2 is load-bearing).
- **F2 — authored look key `ears` is invisible to the
  signature.** LC's Biscuit carries `look.ears: "fox"`.
  `telemetry::Look` / cast.ts's signature have 11 fixed keys;
  `ears` is none of them, and the painter ignores it too. A
  change to `ears` alone would not trip continuity — the same
  limitation the legacy gate has for any look field outside
  the 11 keys. Named, not fixed: widening the signature is a
  contract change for the lead, not a seat decision.
- **F3 — no `paint-cache:` line** on the generic path (§2).
  Consumer-safe (filter-only), disclosed.
- **F4 — header title as authored.** Legacy prints its title
  literal in uppercase; generic prints `manifest.title`
  verbatim (`The Lighthouse Cat's Last Shift`). The consumer
  only requires the `FILM TELEMETRY — ` prefix.
- **F5 — segment ids are used verbatim** (`s01`, lowercase, as
  the bridge authors them) in the shot table, audit
  attribution, and presence — where legacy prints its
  hardcoded `S01` forms. The consumer treats ids as opaque
  `\S+`; internal consistency is what the tables need.
- **F6 — `CAPTION TIME (advisory)` lines are generic-only**
  (§4). They pass through the consumer as audit lines and
  never enter counts.

## 10. Tests (in-module, `#[cfg(test)]`)

No local full-repo build was run (CI gates; no toolchain on
this VM). The tests are written to the crate's existing test
posture and are expected to run at lead integration:

1. `shot_table_from_real_lc_manifest` — builds a
   `FilmManifest` from the real LC `film.json`
   (`~/faceless-shorts/the-lighthouse-cats-last-shift/native/film.json`,
   11 segments / 1172 frames / 43 captions) via a test-side
   constructor against the R1-01 fields, and asserts the shot
   table's header and first/last rows carry s01/s11's own
   frames, times, set and atmosphere; also asserts
   `check_caption_ranges` passes at 43 captions and fails
   closed at 10. **Skips (does not fail) when the project is
   absent**, as it is in CI — the seat verified the assertions
   against the file on disk by construction and by the §1
   regex replication; the lead should run this one test on a
   machine with the project present before the acceptance run.
2. `caption_audit_floors_on_synthetic_captions` — synthetic
   `FilmData` (3 captions, 2 lines) with measured widths fed
   through `caption_infos_from_widths`: one WIDTH failure
   (900 px), one FIT failure (0.50), strict
   `print_caption_audit` returns exactly 2, the summary line is
   byte-exact (`CAPTION AUDIT: 3 chunks, 0/2 lines authored,
   min fit 0.50, max rendered width 900 px`), and the
   display-time advisory fires on a 10-words-in-2.0 s variant
   without touching the failure count.
3. `verdict_line_formatting` — CHECK OK and both FAILED
   shapes, byte-exact against the consumer's count regex.
4. `cutout_from_synthetic_scene_root` — a minimal v2 scene
   (one named, kid, collared character, camera without keys)
   folded and walked: one cutout, authored id, signature
   carrying the authored body colour and `kid`, overflow and
   box present, pose string from the mirrored state, zero
   unresolved plates.
