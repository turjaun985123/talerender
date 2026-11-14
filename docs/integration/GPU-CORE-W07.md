# GPU-CORE-W07 — shared GPU core: survey, divergences, migration plan

Wave 2, seat W07 (audit finding B). Deliverables: this survey +
`src/gpucore.rs` (new module; the lead registers it with a one-line
`mod gpucore;` in main.rs and migrates call sites — this seat edited
no existing file). All line numbers are against the tree as surveyed
(HEAD after d7bebdd, the professional-CLI commit).

## 0. Headline: the audit is half stale — and the live half is worse than it says

**Adapter request is already unified.** Every render entry point
calls `crate::request_adapter` (main.rs:281–302); there is exactly
one adapter implementation in the tree. What is actually duplicated,
five times over, is everything around it:

- the **instance descriptor** — byte-identical at 5 sites;
- the **device request** — 5 sites, **three divergences** (one
  downlevel-limits outlier, one wrong panic message, four labels);
- the **offscreen target helper** — two *identical* `make_target`
  functions (dof.rs:135, scenefile.rs:1550), one fixed-usage variant
  (composer.rs:72 `make_rt`), and inline creations everywhere else;
- the **readback** — 7 sites, two spellings of the padding
  arithmetic, two strip loops, two PNG save spellings, three
  map-error message variants.

`gpucore.rs` therefore keeps `request_adapter`'s exact semantics as
its centrepiece and unifies the rest around it.

## 1. GPU-init sites

| Site | Instance | Adapter | Device label | Limits | Panic msg |
|---|---|---|---|---|---|
| main.rs `run()` — instance 442, device 454–463 | VULKAN + `..Default::default()` | `request_adapter` (local) | `"talerender device"` | **`Limits::downlevel_defaults()`** ⚠ | `"request_device failed"` |
| ab1.rs `run_ab1` — 586, device 606–619 | identical | `crate::request_adapter` | `"talerender device"` | `Limits::default()` (standard) | `"request_device failed"` |
| dof.rs `run_dof` — 157, device 168–177 | identical | `crate::request_adapter` | `"talerender-dof-device"` | `Limits::default()` | **`"request device"`** ⚠ |
| scenefile.rs `make_device` — 1134–1159, called 1246 + 1595 | identical | `crate::request_adapter` | parameter: `"talerender scene-file device"` / `"talerender scene-file composer device"` | `Limits::default()` | `"request_device failed"` |
| film.rs `Renderer::new` — 2355, device 2366–2375 | identical | `crate::request_adapter` | `"talerender film device"` | `Limits::default()` | `"request_device failed"` |

All five also print an adapter-info line between adapter and device
request, in three formats — main/ab1/scenefile:
`adapter: {name} | backend: {:?} | device type: {:?} | driver: {} {}`;
dof: a shorter line with a "software rasteriser" rider; film:
`renderer: {name} | …`. **The film line is a contract**: USER-PC.md
§2 verifies a PC run by checking that the `renderer:` line names
RADV. The prints stay at the call sites; gpucore prints nothing.

### The adapter helper itself (main.rs:281–302) — the strongest behaviour, kept verbatim

```rust
pub(crate) async fn request_adapter(instance: &wgpu::Instance) -> wgpu::Adapter {
    if let Some(adapter) = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
        })
        .await
    {
        return adapter;
    }
    instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: None,
            force_fallback_adapter: true,
        })
        .await
        .expect("no wgpu adapter found — a Vulkan driver (hardware or lavapipe) is required")
}
```

### The limits divergence is documented at the ab1 site (606–619)

```rust
required_features: wgpu::Features::empty(),
// NOT downlevel_defaults: those cap textures at 2048,
// but the real character canvas is 2560x2816 (res 2).
// Standard WebGPU limits (8192) are what the scene
// needs; lavapipe admits them.
required_limits: wgpu::Limits::default(),
```

scenefile.rs's `make_device` carries the same warning in one line.
main.rs::run (the M1 smoke scene) is the only downlevel site; its
textures are tiny, so it works — the divergence is accidental in
origin but **changing it is a behaviour change**, so gpucore takes
limits as a parameter and the main.rs migration passes
`downlevel_defaults()` explicitly, preserving today's behaviour
bit-for-bit. New code gets `request_device_default` (standard).

## 2. Readback sites

| Site | Padding arithmetic | Strip | Map-error strings | Save | Encoder |
|---|---|---|---|---|---|
| main.rs 767–853 | `math::padded_row_bytes(w)` | extend-into-Vec | `"map channel closed"` / `"buffer map failed"` | `RgbaImage::from_raw` + `.save()` | the frame encoder |
| ab1.rs 1555–1590 | same | same | same | same | the frame encoder |
| scenefile.rs direct 1450–1538 | same | same | same | same (1535) | the frame encoder |
| film.rs 3291–3325 | same | same | same | returns pixels; writer at 3488 uses `from_raw` + custom panic `"cannot save {path}: {e}"` | the per-frame encoder |
| scenefile.rs composer 2198–2240 | same | copy-into-preallocated Vec | same | **`image::save_buffer`** (2240, `"write PNG"`) | the frame encoder |
| dof.rs chart 524–555 | **inline `div_ceil`** | copy-into-preallocated Vec | **`"map channel"` / `"map readback"`** | **`image::save_buffer`** (555) | the frame encoder |
| dof.rs `dump_packed_depth` 568–645 | **inline, bpp 4 or 8** | none — samples the mapped rows in place | `"map channel"` / `"map depth readback"` | — (diagnostic print) | **creates its own** |

The canonical block (ab1.rs:1562–1590; main/film/scenefile-direct
are character-identical apart from variable names):

```rust
encoder.copy_texture_to_buffer(
    wgpu::ImageCopyTexture { texture: &final_tex, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
    wgpu::ImageCopyBuffer {
        buffer: &readback,
        layout: wgpu::ImageDataLayout { offset: 0, bytes_per_row: Some(padded), rows_per_image: Some(height) },
    },
    wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
);
queue.submit(Some(encoder.finish()));
let slice = readback.slice(..);
let (tx, rx) = std::sync::mpsc::channel();
slice.map_async(wgpu::MapMode::Read, move |result| { let _ = tx.send(result); });
device.poll(wgpu::Maintain::Wait);
rx.recv().expect("map channel closed").expect("buffer map failed");
```

dof.rs's divergent padding spelling (524–525) — arithmetically the
same function as `math::padded_row_bytes` (math.rs:184–186):

```rust
let unpadded = (width * 4) as usize;
let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize;
let padded = unpadded.div_ceil(align) * align;
```

## 3. Target-creation sites

- `dof.rs:135` and `scenefile.rs:1550` — **the same function twice**:
  `make_target(device, w, h, format, samples, usage, label) -> Texture`,
  identical descriptor body.
- `composer.rs:72` `make_rt` — same descriptor with usage frozen to
  `RENDER_ATTACHMENT | TEXTURE_BINDING` and samples = 1. Already
  shared (film.rs calls it); it stays — gpucore's `make_target` is
  the general form it can later delegate to.
- Inline creations: main.rs frame target (744–753, `Rgba8UnormSrgb`,
  `RENDER_ATTACHMENT | COPY_SRC`) + Depth32Float depth; ab1.rs
  final_tex (882–892, `Rgba8Unorm`); film.rs final_tex (2579–2589,
  `Rgba8Unorm`) + MSAA-4 Depth32Float hdr depth (2561–2572);
  scenefile.rs composer chain via its make_target (1647–1661:
  Rgba16Float MSAA-4 scene, packed-depth, bokeh, `Rgba8Unorm`
  final, fog target).

## 4. Load-bearing vs accidental

**Load-bearing — gpucore preserves each, by parameter or verbatim:**

1. The two-step adapter fallback. Step 2 (`force_fallback_adapter`)
   is the only reason anything renders on GPU-less machines (CI
   lavapipe, this VM); step 1's HighPerformance is what selects
   RADV on the user's PC.
2. Standard limits at ab1/dof/scenefile/film — the 2560×2816
   character canvas exceeds the 2048 downlevel texture cap.
3. **One-submit batching**: the copy rides the frame's own encoder
   at every image site. film.rs submits once per frame; splitting
   the copy into a second submit is a hot-path change, not a
   refactor. (gpucore's `Readback` is two-phase for this reason.)
4. Per-path target formats: direct paths render into
   `Rgba8UnormSrgb`; composer paths finish into `Rgba8Unorm` and
   work in `Rgba16Float`. A unified helper that hardcoded either
   would change pixels — format/samples/usage are parameters.
5. The adapter/renderer print lines (runbook contracts, §1).
6. The `"no wgpu adapter found — …"` panic text — PERF.md's
   failure table quotes it verbatim as the "No adapter" signature.
   `"request_device failed"` is likewise the documented "Device
   failure" signature.

**Accidental — safe to unify:**

1. The instance descriptor, duplicated verbatim ×5.
2. Device label strings (cosmetic; kept as a parameter so old
   logs remain interpretable).
3. dof.rs's `expect("request device")` — the one panic message
   that *disagrees* with the documented signature. Migration
   changes dof's device-failure panic to the canonical
   `"request_device failed"`; this is a deliberate, flagged
   message change, aligned with PERF.md.
4. main.rs's downlevel limits (M1-era; harmless for the smoke
   scene). Preserved explicitly at migration, not silently
   "fixed" (§1).
5. The two padding-arithmetic spellings — provably identical
   (gpucore's `padded_row_bytes(w, 4)` is unit-tested against
   `math::padded_row_bytes(w)` over 12 widths).
6. The two strip loops — byte-identical output by construction.
7. The three map-error message variants and the two PNG save
   spellings (same image-crate PNG encoder underneath; the one
   byte-sensitive consumer — fx-textures — is pinned, §5 step 2).

## 5.
...[truncated 5822 chars]