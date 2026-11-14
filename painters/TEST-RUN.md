# Painter integration pack — combined test run (seat W39)

**Verdict: 52 passed, 0 failed** — all 41 painter tests from the
seven staging modules, plus the 11 pre-existing tests of the
repo's own `scene.rs` / `math.rs` they were compiled against.
No failures, so there is no failure output to quote.

## Method (the W23 technique)

Standalone `rustc` (rustup stable, rustc 1.99.1), **not** cargo,
no repo files modified. A scratch crate in `/tmp/w39/` was
assembled as:

```
src/lib.rs        pub mod math; pub mod scene; pub mod wallpaper;
                  pub mod felt; pub mod parcel; pub mod picture;
                  pub mod sign; pub mod calendar;
src/math.rs       verbatim copy of src/math.rs
src/scene.rs      verbatim copy of src/scene.rs, with
                  painters/planks.rs appended (planks is written
                  as a paste-in fragment for scene.rs — it uses
                  that module's Painter/Rgba/hex_rgb/rnd
                  unqualified, so this is its native form; its
                  tests land as scene::planks_tests)
src/wallpaper.rs  verbatim copy of painters/wallpaper.rs
src/felt.rs       verbatim copy of painters/felt.rs
src/parcel.rs     verbatim copy of painters/parcel.rs
src/picture.rs    verbatim copy of painters/picture.rs
src/sign.rs       verbatim copy of painters/sign.rs
src/calendar.rs   verbatim copy of painters/calendar.rs
```

Build & run:

```
rustc --edition 2021 --crate-type rlib --crate-name wgpu wgpu_stub.rs -o libwgpu_stub.rlib
rustc --edition 2021 --test src/lib.rs --extern wgpu=libwgpu_stub.rlib -o harness_test
./harness_test        # → test result: ok. 52 passed; 0 failed (79.6 s)
```

Compilation produced only dead-code warnings (expected: in the
scratch crate nothing but the tests consumes most of scene.rs).

### The one harness shim — named, not silent

The real `src/math.rs` references
`wgpu::COPY_BYTES_PER_ROW_ALIGNMENT` in `padded_row_bytes()`
(math.rs:185); the full crate gets it from its wgpu dependency.
The harness supplies a stub rlib containing only that constant
at its documented value (256). This is a harness-side stub for
an unrelated external dependency of `math.rs` — **no painter
module needed any shim**: every painter module compiled exactly
as staged, in its intended integration form, against the real
`scene.rs` `Painter`. (`math::tests::padded_rows_align_to_256`
exercises the stubbed constant and passes.)

This also supersedes two "tests not run" caveats in the seats'
own docs, which were written under the CI-only rule:
[picture-sign.md](picture-sign.md) said picture/sign's tests
verify on first CI run — they have now run, here, and pass
(including the `sun_disc_area_band` estimate, inside its
1250..1600 band unmodified); [wallpaper.md](wallpaper.md) and
[calendar.md](calendar.md) reported their own standalone runs,
which this combined run reproduces.

## Results by module

| module (harness path) | tests | passed | failed |
|---|---|---|---|
| planks (`scene::planks_tests`, pasted into scene.rs) | 7 | 7 | 0 |
| wallpaper (`wallpaper::tests`) | 6 | 6 | 0 |
| felt (`felt::tests`) | 5 | 5 | 0 |
| parcel (`parcel::tests`) | 5 | 5 | 0 |
| picture (`picture::tests`) | 4 | 4 | 0 |
| sign (`sign::tests`) | 7 | 7 | 0 |
| calendar (`calendar::tests`) | 7 | 7 | 0 |
| **painter subtotal** | **41** | **41** | **0** |
| baseline: repo `math::tests` | 9 | 9 | 0 |
| baseline: repo `scene::json_tests` | 2 | 2 | 0 |
| **total** | **52** | **52** | **0** |

One runtime note: `scene::planks_tests::
deterministic_and_base_sensitive` took ~60 s of the 79.6 s
suite (it paints the 512×512 canvas three times through the
CPU painter's per-pixel coverage path). Slow, not a failure;
worth knowing before these run in CI.

### Test inventory

- **calendar** (7): alpha_law_boundaries, card_pixel_colours,
  dark_shading_midflip, flip_sy_floor, off_screen_renders_nothing,
  text_widths_fit_the_card, years_at_task_times
- **felt** (5): deterministic, fully_opaque,
  pinned_base_pixel_count, shade_is_the_three_linear_multiply,
  size_and_dims
- **parcel** (5): deterministic, label_false_draws_no_label_colours,
  label_true_counts, pinned_tape_pixel_count, size_and_dims
- **picture** (4): deterministic, pinned_pixels, size_and_opacity,
  sun_disc_area_band
- **planks** (7): deterministic_and_base_sensitive,
  knot_centres_are_exact, mean_colour_matches_algorithm,
  output_size_and_opaque, row_fills_are_exact,
  seam_band_is_exact, shade_truth_table
- **sign** (7): board_pixels, canvas_size_rule, deterministic,
  edge_shade_is_the_linear_source_value,
  pil_measurement_means_no_shrink, strip_composites_over_board,
  text_anchor_matches_fill_text
- **wallpaper** (6): alpha_dot_over_base_composites,
  bookshop_wall_colours, ceiling_variant_and_linear_shade,
  deterministic, size_and_fully_opaque, source_defaults

## What this run does and does not prove

It proves the seven modules compile together, in their intended
integration forms, against the repo's real `scene.rs`/`math.rs`,
and that every unit test the seats wrote passes — including the
cross-module witnesses for the §8 shade question in
[INDEX.md](INDEX.md) (planks' f64 `planks_shade`, felt's f64
`shade_three`, wallpaper's f32 `shade_hex` and sign's
`sign_edge_rgba` all pin the same corpus values in the same
binary). It does **not** prove the `resolve_texture` arms (they
exist only as the INDEX snippets until the lead pastes them),
the wgpu render path, or anything about `src/film.rs`'s local
duplicate painters — those remain CI/integration territory.

---

## Addendum — 2026-11-14, seat W82: staging felt reconcile

**The staging felt shade test now matches production again.**
`felt::tests::shade_is_the_three_linear_multiply` in
`painters/felt.rs` pinned `shade_three("#1F5A45", 0.85)` with
`assert_ne!` against `scene::shade_rgb` — correct when
`shade_rgb` was the old sRGB byte multiply. The shade root
fix `c00897d` replaced `shade_rgb` with the same THREE.Color
linear multiply, so the two implementations now agree on the
corpus witness (28, 83, 64), and the lead re-pinned the
production test in `src/scene.rs` (`d8fbe83`) to assert that
agreement. The staging copy kept the retired `assert_ne!`
(W70's red team caught the resulting 51/52; W80 confirmed
production reads 45/45 while staging read 51/52). W82
reconciled the staging test: `assert_ne!` → `assert_eq!`,
production's post-fix comment mirrored, plus a reconcile
note. The pinned values never diverged — (0x1F, 0x5A, 0x45)
at k = 1.0 and (28, 83, 64) at k = 0.85 in both copies,
before and after; the only remaining textual difference is
the `crate::scene::` path prefix, required because the
staging module compiles standalone.

**Re-runs (standalone rustc 1.99.1, same recipe as above,
scratch in `/tmp/w82-staging` and `/tmp/w82-integrated`):**

| form | assembly | result |
|---|---|---|
| staging | staging `painters/*.rs` + `src/scene.rs` from `c74cf0c^` (planks appended) + repo `src/math.rs` | **52 passed, 0 failed** |
| integrated | HEAD `src/scene.rs` + `src/math.rs` verbatim | **45 passed, 0 failed** (unchanged) |

**Standing rule:** staging mirrors production. When the two
disagree, production wins and the staging copy is reconciled
to it — never the reverse. A staging/production divergence
is a reconcile task (like this one), not evidence about
which implementation is right; that question is settled in
production and its CI.
