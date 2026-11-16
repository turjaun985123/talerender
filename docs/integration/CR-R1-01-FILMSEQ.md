# R1-01 — filmseq driver: wiring note for the lead

Deliverable: `filmseq.rs` (this directory) → lead moves it to
`src/filmseq.rs`, adds the `mod` declaration, applies the ONE
visibility change below, and inserts the §1 fork. Nothing else
in the tree needs to change for this package. Verified against
code at HEAD `aa3de63` (L0 landed: `scenefile::load_root` at
scenefile.rs:862, `scenefile::render_loaded_pixels` at
scenefile.rs:1303 — both `pub(crate)`, exactly as contracted).

## 1. Wiring spec

### 1a. Module declaration (main.rs)

Add after `mod film;` (main.rs:58 at aa3de63):

```rust
mod filmseq;
```

### 1b. The §1 fork — insertion point in `run_film` (film.rs:3726)

Insert at the TOP of `run_film`'s body, BEFORE the existing
`load_film_data` / `load_assets` calls (film.rs:3733–3734) —
the contract's load-bearing requirement is that driver
selection happens before any film data is loaded, so a
scene-sequence film never touches the legacy loaders' staging
assumptions (and a legacy film's data is never parsed by the
generic path):

```rust
pub(crate) async fn run_film(width: u32, height: u32, film_dir: &str, out_dir: &str, from: u32, to: u32, knobs: crate::Knobs, adapter_policy: &str, output_mode: crate::streammodes::OutputMode, queue_capacity: usize) -> crate::errors::TaleResult<()> {
    // REMEDIATION-CONTRACTS §1 fork: discover the manifest and
    // route BEFORE any film data is loaded. No manifest, or a
    // manifest without driver == "scene-sequence", falls
    // through to the legacy staging below, byte-unchanged —
    // the Onoda manifests carry no driver field, so the census
    // gate is protected by construction.
    if let Some(manifest) = crate::filmseq::discover_manifest(Path::new(film_dir))
        .map_err(|e| crate::errors::TaleError::data_load(film_dir, e))?
    {
        if manifest.driver.as_deref() == Some("scene-sequence") {
            return crate::filmseq::run_film_seq(
                width, height, &manifest, Path::new(film_dir), out_dir,
                from, to, knobs, adapter_policy, output_mode, queue_capacity,
            )
            .await;
        }
    }
    // …existing body unchanged from `let tag = …` down.
```

Notes on the fork:

- `discover_manifest` returns `Ok(None)` when no manifest
  exists at any of the three candidates — that is the normal
  legacy case, NOT an error. It returns `Err` only when a
  manifest was FOUND but is broken (unreadable / unparseable /
  malformed / non-tiling / a segment scene resolves nowhere):
  era pairing fails closed, per §1, and the fork propagates it
  as a `data_load` error naming the film dir.
- The same discovery call is what R1-02's telemetry fork needs
  in `run_film_telemetry` (film.rs:3992); `discover_manifest`
  is `pub(crate)` for that reuse. This seat does not wire
  telemetry.
- §1.4's legacy-entry guard (named error instead of the
  `no entry found for key` panic when legacy staging meets
  non-Onoda anchors) is NOT in this seat's deliverable — it
  edits `run_film` / `run_film_telemetry` entry code, which is
  lead territory. Flagging it here so it is not lost: it is
  still owed by the wave.

### 1c. The ONE visibility change required

| Item | Location at aa3de63 | Current | Needed |
|---|---|---|---|
| `overlay_caption` | film.rs:3688 | `fn` (module-private) | `pub(crate) fn` |

That is the only visibility change. Everything else consumed
was already `pub(crate)` (table in §2).

## 2. Cross-module consumption — every item, with current visibility

| Item | Where | Visibility at aa3de63 | Change? |
|---|---|---|---|
| `film::load_film_data` | film.rs:74 | `pub(crate)` | none |
| `film::load_assets` | film.rs:2237 | `pub(crate)` | none |
| `film::FilmData` (+ `pub` fields `captions`, `duration`, …) | film.rs:44 | `pub(crate)` struct | none |
| `film::Caption` (+ `pub s/e/text/fit/strip`) | film.rs:62 | `pub(crate)` struct | none |
| `film::Assets` | film.rs:2223 | `pub(crate)` struct | none |
| `film::overlay_caption` | film.rs:3688 | **private** | **`pub(crate)` — lead** |
| `film::lerp` / `film::seg` / `film::eob` (caption rule) | film.rs:191/194/207 | `pub(crate)` | none |
| `film::eval_frame`, `film::look_registry` (tests only) | film.rs:2178/557 | `pub(crate)` | none |
| `scenefile::load_root` | scenefile.rs:862 | `pub(crate)` (L0) | none |
| `scenefile::render_loaded_pixels` | scenefile.rs:1303 | `pub(crate)` (L0) | none |
| `scenefile::Loaded` (opaque, passed by `&`) | scenefile.rs:122 | `pub(crate)` struct, private fields | none — driver never inspects it |
| `scene::parse_json`, `scene::Value` (+ `get`/`as_arr`/`as_str`/`as_f64`) | scene.rs:1575/1539 | `pub(crate)` | none |
| `schedwire::resolve_range` | schedwire.rs:102 | `pub(crate)` | none |
| `schedwire::fs_frame_iter`, `schedwire::Step` (+ `FrameIter::done/skipped/total/note_rendered/progress_numerator`) | schedwire.rs:270/115 | `pub(crate)` | none |
| `outwire::frame_path` | outwire.rs:114 | `pub(crate)` | none |
| `runlog::RunClock`, `runlog::frame_timing_enabled` | runlog.rs | `pub` items in a crate-private module | none |
| `errors::TaleError::{data_load, image_io, render}`, `errors::TaleResult` | errors.rs:115–141 | `pub` | none |
| `streammodes::OutputMode` | streammodes.rs:182 | `pub(crate)` | none |
| `Knobs` (+ `is_draft`, `tag`) | main.rs:231 | `pub(crate)` | none |
| `image::RgbaImage` | Cargo.toml:11 (`image = "0.25.10"`) | existing dependency, same use as film.rs:3791 | none |

Deliberately NOT consumed:

- `eval15::obj_set` (eval15.rs:85) would set the scene root's
  `time`, but it is module-private (`fn`, not `pub(crate)`).
  Rather than ask for a second visibility change, filmseq
  carries a 10-line local `set_root_time` with identical
  semantics (replace-in-place else append). If the lead prefers
  one shared helper, making `obj_set` `pub(crate)` and deleting
  the local copy is a two-line follow-up.
- `film::Renderer` / `PaintCache` — legacy-path only; the
  generic path renders through scenefile (see §4, deviation 1).
- `streamsession::*` — not consumed; see §3.

## 3. Stream decision: NAMED ERROR (fail loudly), not wired

Decision: `run_film_seq` returns
`TaleError::render("filmseq stream mode", …)` naming the film,
the manifest, and the remedy, as its FIRST statement — before
any data load, scene parse, or directory creation, so a stream
request can never half-start. Contracts §2 explicitly permits
this ("support it via the same FullContractSink contract … or
fail loudly naming it"); the evidence for choosing refusal:

1. **Producer contract mismatch.** The legacy stream branch
   works because its producer is a synchronous closure:
   `streamsession::FrameProducer` is implemented for any
   `FnMut(u32) -> TaleResult<Vec<u8>>` (streamsession.rs:470–484),
   and `run_session_inner` calls `producer.produce(index)` in a
   plain loop on the calling thread (the consumer/encoder runs
   on a spawned thread; the producer does not). The legacy
   producer satisfies this because `film::Renderer::render` is
   a synchronous method.
2. **The generic renderer is async.**
   `scenefile::render_loaded_pixels` is an `async fn` that
   builds a wgpu device per call (`make_device(...).await`,
   scenefile.rs:1192). Driving it from the synchronous producer
   would require a nested `pollster::block_on` inside the outer
   `pollster::block_on(film::run_film …)` in main.rs:537 —
   unproven in this codebase, and a deadlock/panic there would
   surface inside the encoder session, the worst place for a
   first failure. No CI leg can currently exercise it either
   (all proof is lavapipe frame renders).
3. **No consumer needs it.** Contracts §2 itself records: "the
   adapter's video mode is disk-frames + finish; stream is
   exercised only by --film direct runs." The acceptance gate
   (§5b) runs the adapter — PNG frames + finish — so refusing
   stream blocks no acceptance leg.
4. `StreamSpec.fps` is `u32` while the manifest's fps is data
   (f64 in filmseq's model); the legacy branch hardcodes 30
   with a comment tying it to run_film's own rate. A generic
   stream wiring would have to define the fractional-fps rule
   too — more unproven surface for a mode nobody calls.

`queue_capacity` is therefore accepted (signature parity with
`run_film`) and unused on this driver; the error message says
so and reports the value it was given. An `OutputMode::Auto`
reaching the driver has already been resolved by main's mode
resolution (main.rs, the `resolve_mode` call before dispatch),
exactly as in `run_film` — anything that is not `Stream` takes
the PNG path, mirroring the legacy branch structure.

## 4. The caption rule (exact extraction)

From `eval_frame` (film.rs:2217–2222 at aa3de63), quoted
verbatim — this is the code `caption_at` extracts:

```rust
if let Some((ci, c)) = data.captions.iter().enumerate().find(|(_, c)| t >= c.s && t < c.e) {
    let lt = t - c.s;
    let sc = if lt < 0.1 { lerp(0.86, 1.0, eob(seg(lt, 0.0, 0.1))) } else { 1.0 };
    f.caption = Some(ci);
    f.caption_scale = (sc * c.fit) as f32;
}
```

So `sc` is the pop-in scale: `lerp(0.86, 1.0, eob(seg(lt, 0.0, 0.1)))`
for the caption's first 0.1 s (`eob` = easeOutBack, which
overshoots above 1.0 mid-way — at `lt = 0.05`, `sc ≈ 1.01228`),
and exactly `1.0` from `lt = 0.1` on. The overlay scale is
`caption_scale = sc * fit`, `fit` being the caption's own
declared value, cast to f32 at the same point. `caption_at`
returns `(ci, caption_scale)`; selection is the FIRST caption
with `s <= t < e` (half-open: a `t` exactly at `e` belongs to
the next caption or to none). The unit tests pin this three
ways: hard numbers at lt = 0 / 0.05 / 0.1 computed independently
of the crate's easing functions, boundary behaviour at the
first/last captions, and a full-film agreement sweep against
`eval_frame` itself (0.25 s cadence + every caption-boundary
neighbourhood — index AND scale must match at every sample).

## 5. Contract deviations and design choices (all deliberate)

1. **Completion line tails.** `run_film`'s completion line ends
   with the paint-cache stats segment, followed by the
   Renderer's GPU mem-report block and dyn-pool line. The
   generic path has no `PaintCache` and no persistent
   `Renderer` (scenefile builds device + pipelines per
   `render_loaded_pixels` call — the L0 contract's stated v1
   shape), so those tails have no honest values to print.
   filmseq's completion line mirrors run_film's shape up to the
   clock summary suffix and stops there. Banner, progress, and
   frame-timing lines are shape-identical to run_film's,
   including the draft `{tag}` suffix. (Per contracts §2, the
   adapter and bench tooling parse none of these lines.)
2. **Banner fps is the manifest's.** §2's banner template is
   `@<fps>fps` from the manifest; for both real films fps = 30,
   so the printed shape matches the legacy banner exactly
   (`@30fps`). Integral rates print as integers; a fractional
   rate would print as authored rather than being rounded in a
   log line. Legacy prints its hardcoded 30 — same string for
   every film that exists today.
3. **`adapter_policy` is not consumed.** The scene path's
   device construction (`scenefile::make_device`,
   scenefile.rs:1192) takes no policy argument — adapter
   selection there is `gpucore::request_adapter` with no policy
   input. The parameter is kept for signature parity with
   `run_film` and explicitly discarded (`let _ = adapter_policy;`)
   with a comment, rather than silently ignored. If the scene
   path later gains policy support, this is the seam.
4. **Tiling is validated at discovery.** §2 states the manifest
   frame ranges tile `[0, totalFrames)`; filmseq enforces it
   (first segment starts at 0, each start == previous end, last
   end == totalFrames, `endS > startS`, `endFrame > startFrame`,
   `captionChunks.last >= first`) and names the offending
   segment on violation. Both real manifests pass (verified
   against the files on disk). Effect: a malformed manifest
   fails at the fork with a readable error instead of the
   driver guessing mid-render. The per-frame lookup still
   fails closed (named `data_load` error) if a frame in the
   resolved range has no owner — unreachable given the tiling
   validation + `resolve_range` clamping, kept as a guard.
5. **Field types.** `fps: f64` (data-faithful; `t = n/fps` is
   f64 in both drivers, and `StreamSpec`'s u32 problem is one
   more reason stream is refused, §3.4); `total_frames: u32`;
   `Segment.lines: Vec<usize>`; `caption_first/last:
   Option<usize>` — `captionChunks` is optional in filmseq's
   parser (absent → `None, None`) because nothing in the driver
   consumes it (it is carried for R1-02's telemetry shot table);
   both real manifests declare it for every segment.
   `FilmManifest` also carries `manifest_path` beyond the
   contracted field list — it is the source of every
   manifest-error message and costs nothing.
6. **Paths are kept lexical, never canonicalised.** Discovery
   candidates are `film_dir.join("../film.json")` etc., exactly
   as §1 spells them, and resolved scene paths keep the
   `D/../…` spelling. This mirrors the tree's existing law
   (outwire's note: `Path::join` spelling is load-bearing in
   run_film's paths). Tests assert the lexical spellings.
7. **No calendar overlay, no fade** — by contract §2, stated
   here for completeness: the generic path applies ONLY the
   caption overlay post-render. Scene-carried fades/grades are
   the scene path's own (the real scene files carry
   `shot.fades` data).
8. **Inherited loader dependency (not a deviation — a
   sequencing fact).** The generic driver calls the SHARED
   `load_film_data` / `load_assets`. Today's loaders demand
   `calendarYears` / `calendarDark` / `sign` (film.rs:126/151
   region) and the corresponding PNG files. The CURRENT
   Lighthouse Cat bridge satisfies them (it carries the
   invented F3 keys + files, verified on disk), so the generic
   path loads LC as-is. The RE-bridged LC (R2-03, after R1-03's
   decoupling) will only load once R1-03 lands — contracts §4
   is a separate package, and the acceptance run must sequence
   R1-03 before the LC proof. filmseq neither invents nor
   forgives those keys itself.

## 6. Tests (in filmseq.rs, `#[cfg(test)]`)

CPU-only; no GPU, no rendering.

- `discover_repo_layout_manifest` — D = `film/` (repo layout):
  candidates 1–2 absent, candidate 3 hits; title/fps/duration/
  totalFrames/driver-None/14 segments asserted; scene
  resolution lands on the SECOND base (`D/..`, the repo root);
  every segment's scene file exists.
- `discover_lighthouse_cat_manifest` — D =
  `<native>/film` at
  `/home/hatch/faceless-shorts/the-lighthouse-cats-last-shift/native/`
  (project layout): candidate 1 hits; 11 segments; resolution
  lands on the FIRST base (the manifest's directory).
  **Skips with an eprintln when the project is absent** — it
  lives outside the repo, so CI cannot contain it; the repo
  tests never skip.
- `discover_no_manifest_is_none`, `discover_order_first_candidate_wins`
  (synthetic temp-dir tree, all three candidates present →
  §1 order honoured as each is removed),
  `discover_unresolved_scene_is_named_error`,
  `discover_non_tiling_manifest_is_named_error`,
  `discover_malformed_manifest_is_error_not_none`.
- `segment_at_tiles_repo_film` / `segment_at_tiles_lighthouse_cat` —
  every frame in `[0, totalFrames)` is owned, boundaries land
  on the right segment ids (0/177/178/1810 for Onoda;
  0/137/138/1171 for LC), `segment_at(total_frames) == None`.
- `caption_at_onoda_matches_eval_frame_rule` — hard numbers
  (0.86 at lt=0; 1.01227765 at lt=0.05; fit at lt≥0.1),
  half-open boundaries.
- `caption_at_onoda_agrees_with_eval_frame_everywhere` — the
  agreement sweep described in §4.
- `caption_at_lighthouse_cat` — 43 captions; same rule; every
  caption reachable at its midpoint. (Skips with the project.)
- `set_root_time_writes_f32_nearest` — the stored f64 is the
  widened f32 (`0.1f32 as f64`, not f64's 0.1); a missing
  `time` key is appended.

Assumption the lead should know: cargo runs unit tests with
the crate root as CWD, so the repo-layout tests use relative
paths (`film`, `scenes/film/film.json` via discovery) — the
same assumption the existing docs' invocations make.

## 7. Expected integration fixes / what I could NOT verify

Per this seat's rules there was NO local full-repo build (CI
is the gate), and filmseq's crate dependencies mean the file
cannot be compiled standalone against the real crate here.
To reduce the risk anyway, the file was type-checked against
a stub harness in /tmp whose module signatures were copied
from the real crate at aa3de63 (stub only — not a repo build,
not evidence about the real tree). [STUB RESULT — see the
seat's report to the lead; this note is written to be updated
with it before landing if the lead wants it recorded.]

Expected first-CI nits, in rough likelihood order:

1. The `pub(crate)` change to `overlay_caption` (§1c) — if
   missed, filmseq fails with E0603 (private function). This
   is THE required lead edit besides `mod` + fork.
2. Formatting: the file is written in the tree's rustfmt style
   (4-space, trailing commas, width ≈100) but was not run
   through rustfmt (not installed for this toolchain on the
   VM); `cargo fmt --check` in CI may reflow a few long
   `format!` lines. Cosmetic only.
3. The agreement-sweep test calls `eval_frame` ~750 times on
   CPU. Telemetry already samples it at the same cadence over
   the same data, so cost should be comparable to existing
   tests; if CI time objects, gate that one test, not the
   hard-number tests.
4. `#[allow(clippy::too_many_arguments)]` is on
   `run_film_seq` (11 parameters, mirroring run_film's 10 —
   run_film itself carries no allow at aa3de63, so if the
   tree's clippy config doesn't flag run_film it won't flag
   this either; the allow is belt-and-braces and can be
   dropped if it warns as unused).
5. Path-spelling assertions in tests are lexical
   (`film/../scenes/film/…`). They hold on any OS where
   `PathBuf::join` keeps the `..` component (all supported
   targets), but if a future change canonicalises discovery
   paths, those two assertions — not the driver — need
   updating.

Not verified by this seat (honest list): any actual render
through the driver (no GPU on this VM; the acceptance proof
is CI's, contracts §5b); stream behaviour (refused by design,
§3); behaviour with a manifest whose fps ≠ 30 (no such film
exists yet); the re-bridged LC project (R2-03 hasn't run —
the current bridge was verified instead).
