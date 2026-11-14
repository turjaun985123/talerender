# Errors migration — step 2+ patch pack (wave-2 seat W35)

**Deliverable:** this pack only. Nothing else touched, nothing
committed. It re-inventories the two step-2 scopes from
ERRORS-W05.md §3b at current HEAD and turns them into an ordered,
gated patch sequence for the lead.

**State at survey time:** HEAD `df89ad6` ("gpucore migration site
1/5 — dof.rs"). Step 1 landed in `ba88596`. Line numbers below are
exact at `df89ad6`; because the gpucore migration is in flight
(sites 2–5 pending), **re-grep before applying each patch** — every
site is keyed by containing function + exact expression, not by
line alone.

Method: `grep -n -E '\.expect\(|panic!\(|\.unwrap\(\)' src/*.rs`
(148 line-sites tree-wide now, up from W05's 112 — the growth is
the new wave modules' test code, not new runtime panics), then
manual classification of the two scopes.

---

## 1. What changed since the W05 inventory

W05's steps 2+3 listed **13 lines** (6 GPU-init, 7 readback). At
HEAD there are **12 lines / 18 `expect` calls**, because the
gpucore refactor (finding B) partially landed first — exactly the
collapse W05 predicted:

| Delta | Detail |
|---|---|
| dof.rs: **zero** local sites | `new_instance`/`request_adapter`/`request_device_default` via gpucore (dof.rs:136–144); both readbacks via `gpucore::Readback` (dof.rs:489–492, 541–552). Its 3 W05 lines (device 178, readbacks 545/623) now live inside gpucore. |
| scenefile.rs device: delegated | `make_device` (scenefile.rs:1134) is a 3-line wrapper over `gpucore::new_instance` + `request_adapter` + `request_device_default` (1135–1144). Its W05 device line (1158) is gone; its two readbacks remain local. |
| ⚠️ **two live copies** of the adapter helper | `main.rs`'s `pub(crate) request_adapter` (~main.rs:312, expect at :334) AND `gpucore::request_adapter` (gpucore.rs:~57, expect at :80) both exist and both are called. gpucore migration sites for ab1/main/film haven't landed, so main's copy still serves ab1 + film + main. The errors migration must cover **both** copies (or land after the gpucore migration deletes main's). |
| Entry points already fallible | Every containing function below already returns `TaleResult<()>` — `main::run`, `ab1::run_ab1`, `scenefile::run_scene_file`/`run_direct`/`run_composer`, `dof::run_dof`, `film::run_film` — and `main::exit_on_err` (main.rs:152) already prints `error: {e}` and exits `e.exit_code()`. Step 1's plumbing means most of step 2 is local `?` conversions, not signature work. The two exceptions are film's `Renderer::new` / `Renderer::render` (§3, B5/A6). |
| Instance creation | No panic sites. `wgpu::Instance::new` is an infallible constructor at all five sites (inline in main/ab1/film; `gpucore::new_instance` for dof/scenefile). Nothing to migrate. |

**Failure-behaviour delta the whole step produces:** today every
site below panics → stderr panic message, process exit **101**.
After migration: `error: GPU initialisation failed (…): …` (etc.)
on stderr via `exit_on_err`, exit **3**. Success paths must stay
byte-identical — that is what each patch's gate proves.

---

## 2. Scope (a) — GPU init: adapter + device (6 lines)

| # | Site (HEAD `df89ad6`) | Exact expression | Variant → exit | Reachability |
|---|---|---|---|---|
| A1 | `main.rs:334`, in `pub(crate) async fn request_adapter` | `.expect("no wgpu adapter found — a Vulkan driver (hardware or lavapipe) is required")` on the force-fallback `request_adapter` result | `GpuInit` → 3 | **CLI-reachable — the canonical new-machine failure.** Called by main.rs:481 (`run`), ab1.rs:590 (`run_ab1`), film.rs:2379 (`Renderer::new`). Fires whenever no Vulkan adapter answers either request (no driver/ICD). See §5: this message is doc-quoted. |
| A2 | `gpucore.rs:80`, in `pub(crate) async fn request_adapter` | same expect, verbatim same message | `GpuInit` → 3 | **CLI-reachable**, same condition. Called by dof.rs:137 (`run_dof`) and scenefile.rs:1136 (`make_device`). Duplicate of A1 — migrate both, keep the message text identical in both. |
| A3 | `main.rs:498`, in `async fn run` (M1 path, main.rs:476) | `.expect("request_device failed")` — device requested with `Limits::downlevel_defaults()`, label `"talerender device"` | `DeviceRequest` → 3 | **CLI-reachable**: adapter answered but device creation refused (limits/features unsupported, device lost at init). `run` already returns `TaleResult<()>` → local `.map_err(…)?` conversion, context `"request_device (main run)"`. |
| A4 | `gpucore.rs:103`, in `pub(crate) async fn request_device` | `.expect("request_device failed")` | `DeviceRequest` → 3 | **CLI-reachable** via `request_device_default` ← dof.rs:144 and scenefile.rs:1144 (`make_device`, labels supplied per caller). One migration here fixes both consumers; `make_device`'s signature becomes `TaleResult<(Device, Queue)>`, its two callers (scenefile.rs:1232, :1561) already sit in `TaleResult` functions. |
| A5 | `ab1.rs:619`, in `pub async fn run_ab1` (ab1.rs:585) | `.expect("request_device failed")` — `Limits::default()` (the 2048-cap comment at the site is load-bearing; do not "harmonise" limits during this patch) | `DeviceRequest` → 3 | **CLI-reachable**, same condition as A3. `run_ab1` already returns `TaleResult<()>` → local conversion. |
| A6 | `film.rs:2395`, in `Renderer::new` (film.rs:2374) | `.expect("request_device failed")` — label `"talerender film device"`, `Limits::default()` | `DeviceRequest` → 3 | **CLI-reachable**, same condition. ⚠️ Signature ripple: `Renderer::new` returns bare `Renderer` today. It has exactly **one** caller — `run_film` (film.rs:3481), already `TaleResult` — so `new` → `TaleResult<Renderer>` is a two-line ripple. Do it in the same patch as B5 (same function family, one gate). |

No scope-(a) site is an internal invariant: adapter absence and
device-request refusal are both environmental conditions a user
on a new machine genuinely hits (PERF.md's failure table exists
precisely because A1 fires in the field).

## 3. Scope (b) — buffer-map readbacks (6 lines, 12 `expect` calls)

Every site is the same two-layer idiom:
`rx.recv().expect("map channel closed").expect("buffer map failed")`
— layer 1 = the map callback never delivered (channel closed:
device lost / poll failure), layer 2 = the map itself returned an
error. Both layers map to `BufferMap` (exit 3); the source text
keeps them distinguishable (`"map channel closed"` vs the wgpu
error's own Display). All are **CLI-reachable** on device loss or
driver failure mid-run; none is an invariant — a lost device is a
runtime condition, not a programming error.

| # | Site | Containing function (signature today) | Notes |
|---|---|---|---|
| B1 | `main.rs:875` | `run` → `TaleResult<()>` ✔ | Local conversion only. |
| B2 | `ab1.rs:1578` | `run_ab1` → `TaleResult<()>` ✔ | Local conversion only. |
| B3 | `scenefile.rs:1510` | `run_direct` → `TaleResult<()>` ✔ | Local conversion only. |
| B4 | `scenefile.rs:2196` | `run_composer` → `TaleResult<()>` ✔ | Local conversion only. (scenefile.rs:2236+ unwraps are `#[cfg(test)]` — out of scope, leave.) |
| B5 | `film.rs:3333` | `Renderer::render` (film.rs:2843) → returns bare `Vec<u8>` | ⚠️ The only real ripple in scope (b). One call site: the frame loop in `run_film` (film.rs:3499). Recommend `render` → `TaleResult<Vec<u8>>` + `?` at the call site. Payoff beyond tidiness: today a device loss at frame *N* of 1,811 panics (exit 101) mid-run; migrated, it exits 3 with context and the existing `--from` resume contract picks up cleanly. Do NOT instead stash the error in the Renderer — the direct `Result` is simpler and the call graph is one deep. |
| B6 | `gpucore.rs:295`, in `Readback::collect_padded` | `collect` / `collect_padded` → return bare `Vec<u8>` / `(Vec<u8>, u32)` | Serves both dof readbacks today: chart frame (dof.rs:489–492 via `collect`) and packed-depth dump (dof.rs:541–552 via `collect_padded`, inside `dump_packed_depth`, dof.rs:507 — which returns `()` today and is called as a statement from `run_dof`; it becomes `TaleResult<()>` + `?` at that one call). Also the landing spot for every future gpucore readback migration (ab1/scenefile/film local readbacks B2–B5 collapse into B6 when their gpucore site lands — migrate the locals now anyway; the variants are the contract). |

### Adjacent sites in the same functions — explicitly NOT step 2

Re-confirmed at HEAD, leave as panics (W05 §3c reasoning still
holds):

- **`RgbaImage::from_raw(...).expect("pixel buffer size mismatch")`** —
  main.rs:886, ab1.rs:1589, scenefile.rs:1521, film.rs:3509. The
  pixel Vec is built row-by-row immediately above each site at
  exactly `width × height × 4`; a mismatch is a construction bug,
  not a runtime condition. (The `.save(...)` right after each one
  already migrated in step 1 — visible as the `map_err(...)?` at
  main.rs:887 / ab1.rs:1590 / scenefile.rs:1522 / film.rs:3510.)
- **Bind-group `Option` unwraps** — ab1.rs:1369, 1419;
  film.rs:3152, 3159, 3192, 3199, 3206, 3240, 3249, 3257, 3269.
  Populated during pipeline construction before any pass records;
  `None` is unreachable unless construction code changes.
- **`film.rs:2141–2142`** — `order.pop_front().expect("cache order")`
  + the `map.remove(&old).expect("cache order")` the paint-budget
  wiring added beside it. Queue and map are mutated in lockstep;
  invariant.
- **`scenefile.rs:1560`** — `scene.bokeh.as_ref().expect("composer
  path requires post.bokeh")`: `run_composer` is only entered from
  the branch that matched `post.bokeh` present. Invariant.
- **`gpucore.rs:331–332`** — `save_png`'s `from_raw` expect
  (invariant, same reasoning) and `img.save(path).expect("failed
  to save PNG")`. The save is **step-1 class** (`ImageIo`, exit 4),
  not step 2 — flagged here because it sits in gpucore: it has
  **no callers yet** (grep: no `gpucore::save_png` call sites
  outside gpucore.rs; same for `readback_rgba`). Recommend the
  lead fold its conversion into whichever patch first calls it,
  rather than a step-2 patch touching it speculatively.
- **`battery.rs`** — QA harness; its expects (incl. the two
  step-1-class saves W05 listed) are out of step-2 scope entirely.

---

## 4. What step 2 must NOT touch

1. **The telemetry verdict path.** Both halves:
   - `--film-telemetry`: main.rs's dispatch (at HEAD, main.rs
     ~262–275) — `Ok(true)` → proceed/exit 0, `Ok(false)` → exit
     1, `Err(e)` → `film telemetry failed: {e}` on stderr + exit 3
     — and `film::run_film_telemetry` returning
     `Result<bool, String>`. This is the EXIT-CODES-W02 contract:
     exit 1 is the QA *verdict*, exit 3 is its could-not-run code,
     and **no `TaleError` variant maps to 1, by design**
     (errors.rs module header states it outright). Do not convert
     `run_film_telemetry`'s `Err(String)` into a `TaleError`:
     its exit-3 arrives by a different route, and touching it
     risks the verdict-line discipline (line byte-identical,
     authoritative, never printed on the could-not-run path).
   - The M6 `--telemetry` modes keep `telemetry::run`'s own exit
     behaviour.
   - Structural safety note: both telemetry paths are **pure
     CPU** — `telemetry.rs` contains no wgpu/device code at all
     (grep-verified at HEAD), and main.rs's own comment at the
     dispatch says no GPU device is created. Nothing in step 2's
     scopes is shared with them. The only way step 2 could touch
     the verdict path is a deliberate "unify the error types"
     refactor — don't.
2. **`scene.rs:31` `hex_rgb` — W05's "last" scheduling is
   CONFIRMED, with one refinement.** Evidence at HEAD: the
   function (scene.rs:30–33) still has its infallible signature
   and the `.expect("bad hex colour")` at :31; grep counts **16
   call sites** — 13 inside scene.rs itself (:53, :69, :484,
   :1068, :1397, :1411, :1874, :2397, :2535, :2537, :2548, :2553,
   :2555), plus scenefile.rs:445, toonfull.rs:189, film.rs:488.
   The majority pass string literals (`"#EFF4EA"`, `"#B9CCE6"`,
   `"#C8935A"`, …) — compile-time constants whose parse cannot
   fail unless the source is edited, i.e. invariants in effect.
   Making `hex_rgb` fallible would ripple a `?` (or a silent
   fallback colour — worse) through 16 painter call sites to
   handle a case that, for literals, cannot occur. That is
   exactly W05's reasoning and it holds.
   - *Refinement (a split, not a reschedule):* a minority of
     calls are data-driven — scenefile.rs:445 feeds authored
     scene-JSON (`str_field(spec, "color", ctx)`) through it, and
     scene.rs:53/:69 (`hex_lin`/`shade_rgb`) forward caller
     strings. A malformed hex in a user's scene file therefore
     panics mid-render today instead of failing at load. The
     right fix for that path is **load-time validation in
     scenefile's loader** (which already returns
     `Result<_, String>` and can reject a bad colour as
     `DataLoad`, exit 2, before any GPU work starts) — a small,
     separate patch that leaves `hex_rgb` itself and its 13
     internal callers untouched. Recommend: keep `hex_rgb` last
     on the migration list as W05 ordered; optionally land the
     loader-validation patch with step 2, since it is loader-side
     and zero-ripple.
3. **The CLI parse path.** `cli_error` (main.rs:128) already
   exits 2 with a help hint since `d7bebdd`. Do not reroute it
   through `TaleError` — exit 2 is shared between the two by the
   audit's table, but the mechanisms are separate on purpose
   (W05 §1).
4. **Device limits and labels at the device sites.** A3 uses
   downlevel limits, the others standard limits; labels differ
   per site. These divergences are deliberate (see the canvas-size
   comment at ab1.rs:612–615 and gpucore.rs's header). The errors
   migration changes failure handling only — harmonising limits
   belongs to the gpucore migration, gated on its own pins.

---

## 5. Doc-quoted panic message — verbatim preservation required

**The A1/A2 message is quoted in the runbooks:**

- `docs/runbook/PERF.md:295` (failure-mode table, "No adapter"
  row): ``panic: `no wgpu adapter found — a Vulkan driver
  (hardware or lavapipe) is required` `` — with user-facing
  remediation steps built around it.
- `docs/runbook/USER-PC.md:153` quotes the same string as the
  diagnostic a user on their own PC is told to recognise.
- `docs/integration/GPU-CORE-W07.md:168` already flags the text
  as load-bearing for the gpucore move.
- `src/errors.rs`'s own unit test `display_gpu_init` pins the
  string as a `GpuInit` source — the intended post-migration
  shape already exists in the test suite.

**Preservation rule for patches P5/P6 (§6):** the `TaleError`
source text must be exactly
`no wgpu adapter found — a Vulkan driver (hardware or lavapipe) is required`
(em-dash included), so the post-migration stderr line
`error: GPU initialisation failed (request_adapter …): no wgpu adapter found — …`
still contains the quoted string verbatim and PERF.md's
recognition instructions keep working. What does *not* survive —
and should not be faked — is the `panic:` framing and exit 101;
PERF.md:295's row label and USER-PC.md:153 need a dated doc
correction when P5/P6 lands (lead/doc-sweep item, noted here so
it isn't lost).

No other step-2 message is doc-quoted: `"request_device failed"`
and `"buffer map failed"` appear only in code and in the W05/W07
wave notes.

---

## 6. Ordered patch sequence (smallest risk first)

Each patch: one concern, CI after each. "Gate" = the existing job
that must be green + the evidence that proves the success path
didn't move. All render evidence is via **render.yml dispatches**
(the dev VM has no GPU; CI lavapipe is the evidence of record,
per gpucore.rs's header law).

| Patch | Change | Gate / evidence |
|---|---|---|
| **P1** | B6 only: `gpucore::Readback::collect`/`collect_padded` → `TaleResult<…>` with `BufferMap` (context names the readback label); `dof::dump_packed_depth` → `TaleResult<()>`, `?` at its `run_dof` call. Smallest blast radius: one consumer file, already fallible. | ci.yml green (build + `cargo test`); render.yml **dof-chart** dispatch — chart numbers unchanged (M3 step-edge widths, PSNR vs pin) and the packed-depth panel dump values identical to the pre-patch run. |
| **P2** | B3 + B4: scenefile's two local readbacks → `BufferMap` via `?` in `run_direct`/`run_composer`. | ci.yml green; render.yml **scene-file** dispatch — output PNG byte-identical to the pre-patch dispatch (fx-textures byte-identity pin covers the composer path). |
| **P3** | A5 + B2: ab1's inline device request + readback → `DeviceRequest`/`BufferMap` in `run_ab1`. | ci.yml green; render.yml **ab1** dispatch — the ab1 pair still measures **23.96 dB** (the M5 shade-fix pin); no byte claim (composer path), PSNR is the evidence. |
| **P4** | A3 + B1: main `run`'s inline device request + readback → `DeviceRequest`/`BufferMap`. | ci.yml green — its built-in M1 smoke-frame step renders the same frame (artifact compare against the previous green run); cargo test green. |
| **P5** | A4 + A2: `gpucore::request_device`/`request_device_default` → `TaleResult<(Device, Queue)>`; `gpucore::request_adapter` → `TaleResult<Adapter>` (§5 message rule); `scenefile::make_device` → `TaleResult<…>`, `?` at scenefile.rs:1232/:1561; dof.rs:137/:144 updated. | ci.yml green; render.yml **dof-chart AND scene-file** dispatches re-run — both pins from P1/P2 must reproduce exactly (this patch re-touches both consumers). |
| **P6** | A1: main.rs's `request_adapter` → `TaleResult<Adapter>` (§5 message rule); `?` at main.rs:481, ab1.rs:590 (film's call site waits for P7). **Sequencing choice for the lead:** if the gpucore migration reaches main/ab1 first and deletes this helper, P6 is moot — skip it and let A1 die with the duplicate. If errors lands first, migrate in place; the later gpucore deletion then removes an already-migrated copy. Either order is safe; doing both is the only waste. | ci.yml green; render.yml **ab1** dispatch (23.96 dB pin again — ab1 is this helper's consumer). Plus the one **failure-path** check CI can't do: run the CI binary in a job/container with no Vulkan ICD → expect stderr `error: GPU initialisation failed …` and exit **3**, not a panic/101. (No-adapter is the one behaviour this step exists to change; prove it once, manually, at this patch.) |
| **P7** | A6 + B5 together: `Renderer::new` → `TaleResult<Renderer>` (device → `DeviceRequest`; its adapter call `?`s once P6 has landed) and `Renderer::render` → `TaleResult<Vec<u8>>` (readback → `BufferMap`); `?` at run_film's two call sites (film.rs:3481, :3499). Largest care, not largest size: the per-frame hot path. | ci.yml green; render.yml **film** dispatch over the standard chunk — frames **byte-identical** to the pre-patch dispatch; **determinism.yml** dispatch — probes byte-identical (the 1195–1204 / 795–814 windows); `--film-telemetry` re-run prints `FILM TELEMETRY: CHECK OK` with exit 0 (proves the verdict path untouched by proximity). |

After P7, steps 2+3 from W05 are complete: every GPU-init and
readback failure in the five entry-point files and gpucore exits
3 with context; the only runtime panic sites left in these files
are the §3 invariants, the step-1-class `gpucore::save_png` save
(flagged, awaiting its first caller), battery.rs (QA tool), and
`hex_rgb` (§4.2, scheduled last, optionally preceded by the
loader-validation patch).

## 7. Summary counts

| Scope | W05 (at `d7bebdd`) | Now (at `df89ad6`) | After step 2 |
|---|---|---|---|
| (a) GPU init lines | 6 | 6 (A1–A6; dof/scenefile device collapsed into gpucore A4, adapter duplicated as A1+A2) | 0 panicking |
| (b) Readback lines | 7 | 6 (B1–B6; dof's two collapsed into gpucore B6) | 0 panicking |
| Failure exit code | 101 (panic) | 101 (panic) | **3** via `exit_on_err`, success paths byte-identical per patch gates |

*Seat W35 — inventory + patch pack only. No code modified,
nothing committed.*
