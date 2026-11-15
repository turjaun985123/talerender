# CR-B1-09 — WP7 / WP8 / WP11 production callers: integration spec

Chrome-Replacement Batch 1, Seat 9. Deliverables (new files
only; nothing existing was edited, nothing committed):

| File | Wire |
|---|---|
| `src/schedwire.rs` | WP7 `scheduler` → `film::run_film`'s loop |
| `src/assetwire.rs` | WP8 `assetstream` dedup → `scenefile` texture loads |
| `src/outwire.rs` | WP11 `outputmgr` → `film` output naming |
| this file | per-module call sites, parity arguments, scope-outs |

All line numbers are at HEAD `b093f3a` and were verified
against the code, not the V2 docs (whose claims this seat
re-checked: WP7 "planning only", WP8 "no production caller",
WP11 "not wired / byte-identical by construction" are all
accurate — but two details in the WP11 delivery note are
not safe to follow literally; see §3.3).

**Verification performed by this seat** (CI remains the
Rust gate): a scratch harness in `/tmp` (`rustc --edition
2021 --test`, real `scheduler.rs` / `outputmgr.rs` /
`assetstream.rs` included by path, a signature-faithful
`scene::Value` stub) compiled all three wire modules and
ran their tests: **schedwire 8/8, assetwire 7/7, outwire
8/8 passed**; the pre-existing `scheduler` (12/12) and
`outputmgr` (14/14) tests also passed in the same harness.
`assetstream`'s six `from_project_json` tests fail ONLY in
the harness, because the stub's `parse_json` is a stub —
in the real crate `scene::parse_json` is the real parser
and those tests are unaffected by this seat's files (no
wire module calls `from_project_json`). No `Cargo.toml`
edit, no `target/` left in the repo (`git status` shows
only the four new files).

**Global wiring (all three):** three `mod` lines in
`src/main.rs` — `mod assetwire;` after `mod assetstream;`
(line 21), `mod outwire;` after `mod outputmgr;` (line
38), `mod schedwire;` after `mod scheduler;` (line 43).
No other lead change is needed for the modules to compile
and their tests to run.

---

## 1. WP7 — schedwire → `run_film`

### 1.1 The current loop, as verified (src/film.rs)

- Range resolution, film.rs:3657, before the loop:
  `let to = if to == 0 { frames_total } else { to.min(frames_total) };`
  with `frames_total = (data.duration * 30.0).ceil() as u32`
  (3656). `from` is never clamped.
- Loop, film.rs:3673–3710: `for n in from..to` — per frame,
  build `format!("{out_dir}/frame_{n:05}.png")`, skip if
  `Path::new(&path).exists()` (`skipped += 1; continue`),
  else render, `img.save(&path)?`, `done += 1`, optional
  env-gated `frame-time:` line, and a progress `println!`
  iff `done % 25 == 0`, whose timing suffix takes
  `(done + skipped)` and `(to - from)`.
- Semantics that are load-bearing: (a) ascending half-open
  order, reversed range = zero iterations; (b) the exists
  test is **lazy** — per frame, at that frame's turn, not a
  pre-scan; (c) `done` increments only after a successful
  save, `skipped` on the skip itself, progress cadence is
  a function of `done` alone.

### 1.2 Exact call sites for the lead

1. `src/film.rs:3657` — replace the `let to = ...` line with:
   ```rust
   let (from, to) = crate::schedwire::resolve_range(from, to, frames_total);
   ```
2. `src/film.rs:3659` — `create_dir_all(out_dir)` stays,
   unchanged, before the loop.
3. `src/film.rs:3673–3710` — replace the `for` loop with the
   iterator form; the render body between the skip test and
   the save is **unchanged**:
   ```rust
   let mut frames = crate::schedwire::fs_frame_iter(from, to, out_dir);
   while let Some(step) = frames.next() {
       let n = step.index();
       if let crate::schedwire::Step::Skip(_) = step { continue; }
       let path = crate::outwire::frame_path(out_dir, n);
       let frame_start = std::time::Instant::now();
       let t = n as f64 / 30.0;
       // ... existing body: eval_frame / resolve_chars /
       // render / overlays / fade / caption, unchanged ...
       img.save(&path).map_err(|e| crate::errors::TaleError::image_io(&path, e))?;
       let progress_due = frames.note_rendered();   // was: done += 1
       if timing {
           println!("frame-time: frame={n} ms={:.2}",
               frame_start.elapsed().as_secs_f64() * 1000.0);
       }
       if progress_due {                            // was: if done % 25 == 0
           println!("  frame {n} (t = {t:.2}s) — {} rendered, {} skipped · {}{tag}",
               frames.done(), frames.skipped(),
               clock.timing_suffix(frames.progress_numerator(),
                                   frames.total() as u64));
       }
   }
   ```
   Ordering note: the current code increments `done`
   BEFORE the `frame-time:` line and tests `% 25` after it;
   `note_rendered()` is placed exactly at the old
   `done += 1` so both lines see identical counter values.
4. Completion line (film.rs:3709): substitute
   `done` → `frames.done()`, `skipped` → `frames.skipped()`;
   the local `(mut done, mut skipped)` declaration at
   film.rs:3665 is deleted. The `clock` / `cache` lines are
   untouched.

### 1.3 Parity argument

- **Sequence**: `FrameIter` yields `Skip`/`Render` by the
  same predicate (exists at that index), in the same
  ascending order, over the same resolved range.
  In-file tests compare it against a line-for-line
  transcription of film.rs:3673–3678 over representative
  ranges: empty, reversed (5..3), single, all-cached,
  none-cached, sparse cache with out-of-range entries,
  mid-film start (1810..1811), and the full Onoda range
  (0..1811). All equal.
- **Plan equivalence**: for a static exists set,
  `scheduler::plan(range, &existing)` equals the iterator's
  Render subsequence, and `batches(plan, k)` flattens to
  the plan for every `k` including the 0→1 clamp (tests).
  **Range bridging**: steps(0..3) ++ steps(3..7) ==
  steps(0..7) (test) — a `--to b` run resumed with
  `--from b` is indistinguishable from one full run.
- **Resume**: production uses `fs_frame_iter`, whose exists
  test runs inside `next()` — the same lazy moment as the
  loop. A test creates frame 2's file mid-iteration and
  observes `Skip(2)`; a pre-scan plan would have rendered
  it. This is why the iterator, not `plan()`, is the
  production shape; `plan`/`batches` are exposed for
  reporting and future consumers.
- **Counters/progress**: tests pin the cadence (due after
  saves 25 and 50 of 60; 3 interleaved skips do not shift
  it, but do enter the numerator: first due at
  `(done, numerator) = (25, 28)`).
- **Pixels/stdout**: the iterator decides only WHICH frames
  render, in the loop's own order; frame bytes are a pure
  function of index (existing contract), and every printed
  line keeps its current text, values and order.

### 1.4 Scope-outs (adoption WOULD change behaviour)

- **Scheduler checkpoint as resume source — DO NOT WIRE.**
  `plan_resume`/`load_checkpoint` substitute a text file
  for PNG existence. Divergences: a checkpointed index
  whose PNG was deleted would never re-render (the loop
  renders it); a present PNG with no checkpoint line
  would be re-rendered and overwritten (the loop skips
  it); `append_checkpoint` adds a new mid-loop I/O failure
  surface the loop does not have. PNG existence remains
  the sole resume source. Revisit only as a separately
  approved semantics change.
- **Parallel batch rendering — DO NOT WIRE.**
  `planned_batches` with `max_in_flight > 1` is exposed
  for planning only. The current loop is strictly
  sequential (one `Renderer`, one `PaintCache`); rendering
  batches concurrently is a renderer change, not a caller
  wire. Sequential batch-by-batch consumption in order is
  identical (flatten property, §1.3) if the lead ever
  wants the shape.
- **`SchedEvent` translation — not needed.** The scheduler
  prints nothing; the loop's existing lines (§1.2) already
  carry the same information. Mapping events onto them
  would be a second vocabulary for the same lines.

---

## 2. WP8 — assetwire → `scenefile` texture loads

### 2.1 The actual call sites, as verified

- `src/scene.rs` performs **no file I/O** (grep-verified:
  no `image::open` / `fs::read` / `File::open`). It is not
  an asset-load site; claims to the contrary are wrong.
- The scene loader's only file-asset read is
  `scenefile::resolve_texture` (src/scenefile.rs:409),
  file branch at 410–418: non-string `"file"` →
  `"{ctx}: 'file' must be a string"`; otherwise
  `image::open(scene_dir.join(path))` → error
  `"{ctx}: cannot load texture file {full}: {e}"`, then
  `.to_rgba8()`. All other branches are procedural
  generators (no I/O).
- Its only production caller is the texture loop in
  `scenefile::load` (1010–1021): per spec, in order —
  duplicate-id check (`duplicate texture id '{id}'`)
  BEFORE resolution, then `resolve_texture(...)?`; the
  first failure aborts the load. `resolve_texture` and
  `load` are both private (`fn`), which shapes the wire
  (§2.2). Textures appended by `walk_nodes` (painted
  cards, blob shadows, 688–691) are in-memory, not loads.

### 2.2 Exact call sites for the lead

1. `scenefile::load`, just before the texture loop
   (~line 1008): `let mut wire = crate::assetwire::AssetWire::new();`
   — one instance per `load()` (the current code has no
   cross-load cache; neither does the wire).
2. Pass `&mut wire` into `resolve_texture` (signature
   gains one parameter; its one caller is the loop above).
3. Replace the file branch (410–418) with:
   ```rust
   if let Some(r) = wire.resolve_file(spec, scene_dir, ctx, |full| {
       image::open(full)
           .map_err(|e| crate::assetwire::AssetWire::load_error(ctx, full, &e))
           .and_then(|img| {
               let img = img.to_rgba8();
               let (w, h) = img.dimensions();
               Ok((img.into_raw(), w, h))
           })
   }) {
       return r;
   }
   // fall through to the existing generator dispatch, unchanged
   ```
   The closure IS the current decode expression, moved
   verbatim; `load_error` reproduces
   `err(ctx, &format!("cannot load texture file {}: {e}", full.display()))`
   byte-for-byte (`err` at scenefile.rs:151 is
   `"{ctx}: {msg}"`) and is pinned by test.
4. Optional, advisory only: after a successful load the
   lead MAY log `AssetWire::dedup_pairs(&scene_dir, &specs)`
   — and must not fail the load on its `Err` (§2.3).

### 2.3 Parity argument + scope-outs

Parity, by construction and test (assetwire, 7 tests):

- **Same point, same order, first-failure**: `resolve_file`
  runs per spec at the exact place the file branch ran;
  nothing is pre-scanned. The duplicate-id check still
  precedes resolution. A test drives two specs naming the
  same missing file: each fails at its own turn, opener
  invoked each time, identical error strings.
- **Failures never cached** (above) and **no stale hits**:
  a hit requires a fresh `metadata` stat at request time
  showing the same path, still a regular file, with the
  SAME size as at decode. Deletion → fall-through to a
  real open and its real error (tested); size change →
  real open returning the new bytes (tested). This is
  WP8's own path+size rule applied to decoded textures.
- **Generator specs**: `resolve_file` returns `None`; the
  dispatch below is untouched (tested: opener never runs).
- **Duplicate file, static FS**: opened exactly once;
  both texture ids receive byte-identical decoded data
  (tested). Texture ORDER and indices are unchanged —
  the cache is inside resolution, the loop still pushes
  one entry per spec.

Scope-outs / residuals:

- **`Manifest::from_value` as a load-path gate — DO NOT
  WIRE.** It rejects `..` path components (C4 rule); the
  loader's `scene_dir.join(path)` accepts them and a scene
  referencing `../shared/tex.png` loads today. Evidence:
  assetwire test `manifest_rejects_dotdot_paths_the_loader_accepts_scope_out_evidence`
  pins both sides (manifest errs; the front-end loads the
  file). It would also move duplicate-id failure ahead of
  earlier file errors if built from all specs up front.
  `assetwire::manifest`/`dedup_pairs` exist for advisory
  reporting only.
- **`Manifest::diagnostics()` as pre-flight — DO NOT
  WIRE.** It lists EVERY missing asset in WP8's own string
  format; the current behaviour is the first failure, in
  scenefile's ctx format, at that spec's turn. Different
  string, different timing, different failure surface.
- **Residual divergence (accepted WP8 limit, not a
  defect of the wire)**: a file rewritten with different
  bytes but the SAME size between two specs of one load
  is served from cache where today it would be
  re-decoded. No in-process write can interleave (all
  textures resolve before any rendering); an external
  writer racing the load is the only trigger. This is
  assetstream's documented path+size limit, verbatim.
- **`film::load_png` / `load_assets` (film.rs:2173ff) —
  out of scope, not wired.** Plates / caption strips /
  calendar / sign are a separate loader with its own
  error strings (`cannot load {path}: {e}`). The same
  cache pattern would fit (plates can repeat a file under
  two names), but the charter names the scene loader;
  recommend a follow-up seat if the lead wants it.

---

## 3. WP11 — outwire → film output naming

### 3.1 Every existing case, enumerated from the code

| # | Case | Source | Owner after wire |
|---|---|---|---|
| 1 | Frame PNG path `"{out_dir}/frame_{n:05}.png"` — the ONLY inline output construction in `run_film`; used for the exists test and `img.save` (error path surfaces the string) | film.rs:3674 | `outwire::frame_path` |
| 2 | Mux input pattern `frame_%05d.png` (same canonical name, shell spelling) | film/finish.sh:25 | pinned by `outwire::frame_name` / `parse_frame_name`; script unchanged |
| 3 | In-flight MP4 `"${OUT}.part.mp4"` (renamed to user-supplied `$OUT`) | film/finish.sh:22 | `outwire::part_mp4_path` pins the spelling; script stays owner (outputmgr has no mp4 namer) |
| — | `format!("calendar-{y}.png")` (film.rs:2241) | an INPUT asset path under `film/assets/` — NOT an output; excluded | — |
| — | `--out` paths (film dir, scene PNG) | user-supplied in main.rs, never constructed | — |

### 3.2 Exact call sites for the lead

1. film.rs:3674 (or the §1.2 loop swap, which already
   does it): `let path = crate::outwire::frame_path(out_dir, n);`
2. Nothing else in the default wire. `finalize()` may be
   called by a future validate mode; `existing_frames()`
   only after `create_dir_all(out_dir)` (see §3.3).

### 3.3 Parity argument + scope-outs

- **Byte parity**: `frame_path` is
  `format!("{out_dir}/{}", FrameNamer::frame_name(n))`.
  Tests compare it against the literal film.rs:3674
  expression for indices {0, 1, 7, 42, 999, 1810, 1811,
  99999, 100000, 123456, 9999999, u32::MAX} × out_dirs
  {"frames", "frames/", "/tmp/x", "a/b/c", "", "."} —
  all byte-identical, including natural widening past
  five digits.
- **The join trap — outputmgr's suggested rewrite is
  scoped OUT.** Its delivery note proposes
  `Path::new(out_dir).join(FrameNamer::frame_name(n))`.
  Not byte-identical: `out_dir = "frames/"` gives inline
  `frames//frame_00001.png` vs join `frames/frame_00001.png`;
  `out_dir = ""` gives inline `/frame_00001.png` vs join
  `frame_00001.png`. Same file on Linux, different STRING
  — and the string is observable in
  `TaleError::image_io(&path, e)` on save failure. A test
  pins the divergence so it cannot be "simplified" away.
  `frame_path` keeps the `format!` shape.
- **`RenderMetadata::write_to` — DO NOT WIRE by default.**
  It ADDS `render-metadata.txt` to a directory that today
  contains only frames: a content change (the file shows
  up in `OutputScan` stray lists and downstream
  listings). `outwire::metadata(...)` provides the tested
  field mapping (`frames_rendered` ← `done`,
  `frames_skipped_existing` ← `skipped`) so the lead can
  adopt it as a deliberate, announced addition — it is
  not a parity wire.
- **`existing_frames()` (scan-based resume set) — call
  only after `create_dir_all`.** `OutputScan::scan`
  errors on a missing directory where the loop's
  `exists()` returns false. `run_film` creates the
  directory at film.rs:3659 before the loop, so a
  post-creation scan agrees pointwise with the loop's
  tests (tested, including the non-canonical `frame_1.png`
  stray, which both ignore). The production resume path
  (§1) needs no scan at all.
- **`finalize()` printing — not in the default wire.**
  The report itself is pure and non-mutating, but
  PRINTING its rows would add stdout lines the current
  run does not emit. Keep it for a validate/finish mode.

---

## 4. What the lead should expect at integration

- CI build+test green with the three `mod` lines; the
  wire modules add 23 tests (8 + 7 + 8) to the suite.
- `FILM TELEMETRY: CHECK OK` and the byte probes are
  unaffected by construction: no wire touches a rendered
  pixel, a staging value, or a printed line in the
  default integrations above (§1.2, §2.2, §3.2).
- The three scope-out clusters — scheduler checkpoint,
  parallel batches, manifest/diagnostics gating,
  metadata-file writing — are the complete list of
  adoption paths this seat found that WOULD change
  behaviour. Each is available as a separate, announced
  change if the user/lead wants it; none is smuggled into
  the parity wires.
