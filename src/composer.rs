//! Composer pass machinery (Milestone 5): the post chain the A1 audit
//! documented for the production engine — RenderPass -> BokehPass ->
//! UnrealBloomPass -> OutputPass -> GRADE — as reusable wgpu passes.
//! Bokeh itself lives with its gate (src/dof.rs, src/shader_bokeh.wgsl);
//! this module carries the stages M5 adds:
//!
//!   * the PMREM environment texture capture (RoomEnvironment,
//!     engine.ts:32-34), stored verbatim as float16 in
//!     assets/room-env-pmrem.f16 and sampled by shader_pbr.wgsl's
//!     textureCubeUV transcription;
//!   * UnrealBloomPass: bright (luminosity high-pass) -> 5 mip levels
//!     of separable Gaussian blur -> composite -> additive blend
//!     (three/addons/postprocessing/UnrealBloomPass.js, MIT);
//!   * OutputPass with a uniform exposure (shader_output_exp.wgsl);
//!   * the engine's custom GRADE shader (engine.ts:18-27).
//!
//! Every pass renders a single fullscreen triangle; all intermediate
//! targets are HalfFloat, matching the composer's render targets.

use std::borrow::Cow;

pub(crate) struct BloomParams {
    pub threshold: f32,
    pub strength: f32,
    pub radius: f32,
}

pub(crate) struct GradeParams {
    pub vignette: f32,
    pub grain: f32,
    pub warm: f32,
    pub sat: f32,
    pub time: f32,
}

pub(crate) const BLOOM_KERNEL_RADII: [usize; 5] = [3, 5, 7, 9, 11];
pub(crate) const BLOOM_FACTORS: [f32; 5] = [1.0, 0.8, 0.6, 0.4, 0.2];

/// UnrealBloomPass mip sizes: start at (round(w/2), round(h/2)) and
/// halve with Math.round at each level.
pub(crate) fn bloom_mip_sizes(width: u32, height: u32) -> [(u32, u32); 5] {
    let mut out = [(0u32, 0u32); 5];
    let mut x = (width as f32 / 2.0).round();
    let mut y = (height as f32 / 2.0).round();
    for i in 0..5 {
        out[i] = (x as u32, y as u32);
        x = (x / 2.0).round();
        y = (y / 2.0).round();
    }
    out
}

/// Device bytes of ONE bloom chain at frame size (width, height)
/// — the VRAM ledger's class C (VRAM-IMPL-W84 §3.2): bright at
/// mip 0 plus horiz and vert at every mip, all Rgba16Float
/// (8 B/px): `8 × (px₀ + 2·Σᵢ pxᵢ)` over `bloom_mip_sizes`.
/// At 1080×1920: 8 × (518,400 + 2×690,600) = 15,196,800.
pub(crate) fn bloom_chain_bytes(width: u32, height: u32) -> u64 {
    let sizes = bloom_mip_sizes(width, height);
    let px0 = sizes[0].0 as u64 * sizes[0].1 as u64;
    let sum: u64 = sizes.iter().map(|&(w, h)| w as u64 * h as u64).sum();
    8 * (px0 + 2 * sum)
}

fn pack_f32(data: &[f32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len() * 4);
    for v in data {
        out.extend_from_slice(&v.to_ne_bytes());
    }
    out
}

fn make_buffer(device: &wgpu::Device, queue: &wgpu::Queue, label: &str, bytes: &[u8], usage: wgpu::BufferUsages) -> wgpu::Buffer {
    let buf = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size: bytes.len() as u64,
        usage,
        mapped_at_creation: false,
    });
    // VRAM ledger (W84 §5.2): every PostChain pass buffer flows
    // through here — one insertion covers them all. Lifetime is
    // the caller's MemScope (PostChain methods open one per call).
    crate::gpucore::mem_alloc_buffer(crate::gpucore::MemClass::G, bytes.len() as u64);
    queue.write_buffer(&buf, 0, bytes);
    buf
}

pub(crate) fn make_rt(
    device: &wgpu::Device,
    width: u32,
    height: u32,
    format: wgpu::TextureFormat,
    label: &str,
) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    })
}

// ---------------------------------------------------------------------
// The captured PMREM environment (assets/room-env-pmrem.f16): the real
// PMREMGenerator.fromScene(new RoomEnvironment(), 0.04) output read
// back from the vendored three r170 in headless Chrome
// (docs/ab/composer/project/env-capture.html). 768x1024 RGBA float16,
// WebGL readback row order (row 0 = v 0, matching three's sampling).
// ---------------------------------------------------------------------

pub(crate) const ROOM_ENV_W: u32 = 768;
pub(crate) const ROOM_ENV_H: u32 = 1024;

pub(crate) fn load_room_env(device: &wgpu::Device, queue: &wgpu::Queue) -> wgpu::Texture {
    let bytes = include_bytes!("../assets/room-env-pmrem.f16");
    assert_eq!(bytes.len(), (ROOM_ENV_W * ROOM_ENV_H * 8) as usize, "room env asset size drifted");
    let tex = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("room environment PMREM (captured)"),
        size: wgpu::Extent3d { width: ROOM_ENV_W, height: ROOM_ENV_H, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba16Float,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    // VRAM ledger (W84 §5.2): class D, the PMREM environment.
    crate::gpucore::mem_alloc_texture(
        crate::gpucore::MemClass::D,
        crate::gpucore::tex_bytes(ROOM_ENV_W, ROOM_ENV_H, wgpu::TextureFormat::Rgba16Float, 1),
    );
    queue.write_texture(
        wgpu::TexelCopyTextureInfo { texture: &tex, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
        bytes,
        wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(ROOM_ENV_W * 8), rows_per_image: Some(ROOM_ENV_H) },
        wgpu::Extent3d { width: ROOM_ENV_W, height: ROOM_ENV_H, depth_or_array_layers: 1 },
    );
    tex
}

/// IEEE 754 half -> f32 (for capture statistics and tests).
pub(crate) fn half_to_f32(h: u16) -> f32 {
    let sign = ((h >> 15) & 1) as u32;
    let exp = ((h >> 10) & 0x1f) as i32;
    let frac = (h & 0x3ff) as u32;
    let bits = if exp == 0 {
        if frac == 0 {
            sign << 31
        } else {
            // Subnormal half: normalise.
            let mut e = -14i32;
            let mut f = frac;
            while f & 0x400 == 0 {
                f <<= 1;
                e -= 1;
            }
            (sign << 31) | (((e + 127) as u32) << 23) | ((f & 0x3ff) << 13)
        }
    } else if exp == 31 {
        (sign << 31) | (0xff << 23) | (frac << 13)
    } else {
        (sign << 31) | (((exp - 15 + 127) as u32) << 23) | (frac << 13)
    };
    f32::from_bits(bits)
}

/// (max, mean of green) over the captured PMREM texels — logged by the
/// renderers as provenance that the shipped asset is the captured one
/// (capture log: max 56.3125, meanG 0.67713).
pub(crate) fn room_env_stats() -> (f32, f32) {
    let bytes = include_bytes!("../assets/room-env-pmrem.f16");
    let mut max = 0.0f32;
    let mut sum = 0.0f64;
    let n = (ROOM_ENV_W * ROOM_ENV_H) as usize;
    for i in 0..n {
        for c in 0..3 {
            let o = (i * 4 + c) * 2;
            let v = half_to_f32(u16::from_le_bytes([bytes[o], bytes[o + 1]]));
            if v > max {
                max = v;
            }
            if c == 1 {
                sum += v as f64;
            }
        }
    }
    (max, (sum / n as f64) as f32)
}

// ---------------------------------------------------------------------
// Separable blur shader generation. UnrealBloomPass builds one
// ShaderMaterial per (mip, direction) with KERNEL_RADIUS as a define
// and the Gaussian coefficients as a uniform array; the WGSL bakes
// both into the module text, coefficients evaluated in f64 with the
// addon's formula (0.39894 * exp(-0.5 * i^2 / r^2) / r).
// ---------------------------------------------------------------------

fn blur_wgsl(horizontal: bool, radius: usize) -> String {
    let coeffs: Vec<String> = (0..radius)
        .map(|i| {
            let v = 0.39894f64 * (-0.5 * (i as f64) * (i as f64) / ((radius * radius) as f64)).exp() / (radius as f64);
            format!("{v}")
        })
        .collect();
    let dir = if horizontal { "vec2f(1.0, 0.0)" } else { "vec2f(0.0, 1.0)" };
    format!(
        r#"// Generated: three.js r170 UnrealBloomPass separable blur,
// KERNEL_RADIUS = {radius}, direction = {dir} (MIT — see
// src/composer.rs for provenance). diffuseSum is rgb-only and the
// output alpha is 1.0, exactly like the GLSL.
struct U {{
    inv_size: vec4f, // xy: 1 / target size
}};
@group(0) @binding(0) var<uniform> u: U;
@group(0) @binding(1) var tex: texture_2d<f32>;
@group(0) @binding(2) var smp: sampler;
struct Vout {{
    @builtin(position) clip: vec4f,
    @location(0) uv: vec2f,
}};
@vertex
fn vs_main(@builtin(vertex_index) i: u32) -> Vout {{
    var o: Vout;
    let x = f32((i << 1u) & 2u);
    let y = f32(i & 2u);
    o.uv = vec2f(x, y);
    o.clip = vec4f(x * 2.0 - 1.0, 1.0 - y * 2.0, 0.0, 1.0);
    return o;
}}
var<private> KERNEL: array<f32, {radius}> = array<f32, {radius}>({coeffs});
@fragment
fn fs_main(in: Vout) -> @location(0) vec4f {{
    var weight_sum = KERNEL[0];
    var diffuse_sum = textureSample(tex, smp, in.uv).rgb * weight_sum;
    for (var i = 1; i < {radius}; i++) {{
        let x = f32(i);
        let w = KERNEL[i];
        let uv_offset = {dir} * u.inv_size.xy * x;
        let s1 = textureSample(tex, smp, in.uv + uv_offset).rgb;
        let s2 = textureSample(tex, smp, in.uv - uv_offset).rgb;
        diffuse_sum += (s1 + s2) * w;
        weight_sum += 2.0 * w;
    }}
    return vec4f(diffuse_sum / weight_sum, 1.0);
}}
"#,
        radius = radius,
        dir = dir,
        coeffs = coeffs.join(", ")
    )
}

const COPY_WGSL: &str = r#"
// CopyShader (three/addons, MIT): gl_FragColor = opacity * texel with
// opacity 1.0 — a passthrough; the pass's AdditiveBlending state does
// the work (UnrealBloomPass's final blend over the read buffer).
@group(0) @binding(0) var tex: texture_2d<f32>;
@group(0) @binding(1) var smp: sampler;
struct Vout {
    @builtin(position) clip: vec4f,
    @location(0) uv: vec2f,
};
@vertex
fn vs_main(@builtin(vertex_index) i: u32) -> Vout {
    var o: Vout;
    let x = f32((i << 1u) & 2u);
    let y = f32(i & 2u);
    o.uv = vec2f(x, y);
    o.clip = vec4f(x * 2.0 - 1.0, 1.0 - y * 2.0, 0.0, 1.0);
    return o;
}
@fragment
fn fs_main(in: Vout) -> @location(0) vec4f {
    return textureSample(tex, smp, in.uv);
}
"#;

// ---------------------------------------------------------------------
// The pass set. Pipelines are created once per device; per-call bind
// groups and targets follow the codebase's still-renderer pattern.
// ---------------------------------------------------------------------

fn tex_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        },
        count: None,
    }
}

fn smp_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
        count: None,
    }
}

fn ubo_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

fn fs_pipeline(
    device: &wgpu::Device,
    label: &str,
    module: &wgpu::ShaderModule,
    bgl: &wgpu::BindGroupLayout,
    format: wgpu::TextureFormat,
    blend: Option<wgpu::BlendState>,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor { cache: None,
        label: Some(label),
        layout: Some(&device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some(label),
            bind_group_layouts: &[Some(bgl)],
            immediate_size: 0,
        })),
        vertex: wgpu::VertexState { module, entry_point: Some("vs_main"), buffers: &[], compilation_options: Default::default() },
        fragment: Some(wgpu::FragmentState {
            module,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState { format, blend, write_mask: wgpu::ColorWrites::ALL })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
    })
}

fn draw_fs(
    encoder: &mut wgpu::CommandEncoder,
    label: &str,
    pipeline: &wgpu::RenderPipeline,
    bg: &wgpu::BindGroup,
    target: &wgpu::TextureView,
    load: wgpu::LoadOp<wgpu::Color>,
) {
    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some(label),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment { depth_slice: None,
            view: target,
            resolve_target: None,
            ops: wgpu::Operations { load, store: wgpu::StoreOp::Store },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
    pass.set_pipeline(pipeline);
    pass.set_bind_group(0, bg, &[]);
    pass.draw(0..3, 0..1);
}

pub(crate) struct PostChain {
    sampler: wgpu::Sampler,
    bright_pipeline: wgpu::RenderPipeline,
    bright_bgl: wgpu::BindGroupLayout,
    blur_pipelines: [[wgpu::RenderPipeline; 2]; 5], // [mip][0=H, 1=V]
    blur_bgl: wgpu::BindGroupLayout,
    composite_pipeline: wgpu::RenderPipeline,
    composite_bgl: wgpu::BindGroupLayout,
    add_pipeline: wgpu::RenderPipeline,
    add_bgl: wgpu::BindGroupLayout,
    output_pipeline: wgpu::RenderPipeline,
    output_bgl: wgpu::BindGroupLayout,
    grade_pipeline: wgpu::RenderPipeline,
    grade_bgl: wgpu::BindGroupLayout,
    /// VRAM ledger (W84 ruling D2): the frame size whose bloom
    /// chain is currently registered as standing class C, if
    /// any. See the registration in `bloom`.
    bloom_mem: std::cell::Cell<Option<(u32, u32)>>,
}

impl PostChain {
    pub(crate) fn new(device: &wgpu::Device) -> PostChain {
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("post linear sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            ..Default::default()
        });
        let module = |label: &str, src: Cow<'_, str>| {
            device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(label),
                source: wgpu::ShaderSource::Wgsl(src),
            })
        };
        let bgl = |label: &str, entries: &[wgpu::BindGroupLayoutEntry]| {
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor { label: Some(label), entries })
        };
        let half = wgpu::TextureFormat::Rgba16Float;
        let byte = wgpu::TextureFormat::Rgba8Unorm;

        // Bright (luminosity high-pass).
        let bright_bgl = bgl("bloom bright layout", &[ubo_entry(0), tex_entry(1), smp_entry(2)]);
        let bright_module = module("bloom bright shader", Cow::Borrowed(include_str!("shader_bloom_bright.wgsl")));
        let bright_pipeline = fs_pipeline(device, "bloom bright pipeline", &bright_module, &bright_bgl, half, Some(wgpu::BlendState::REPLACE));

        // Separable blurs (generated modules per mip/direction).
        let blur_bgl = bgl("bloom blur layout", &[ubo_entry(0), tex_entry(1), smp_entry(2)]);
        let blur_pipelines: [[wgpu::RenderPipeline; 2]; 5] = std::array::from_fn(|i| {
            std::array::from_fn(|d| {
                let m = module("bloom blur shader", Cow::Owned(blur_wgsl(d == 0, BLOOM_KERNEL_RADII[i])));
                fs_pipeline(device, "bloom blur pipeline", &m, &blur_bgl, half, Some(wgpu::BlendState::REPLACE))
            })
        });

        // Composite.
        let composite_bgl = bgl(
            "bloom composite layout",
            &[ubo_entry(0), tex_entry(1), tex_entry(2), tex_entry(3), tex_entry(4), tex_entry(5), smp_entry(6)],
        );
        let composite_module = module("bloom composite shader", Cow::Borrowed(include_str!("shader_bloom_composite.wgsl")));
        let composite_pipeline = fs_pipeline(device, "bloom composite pipeline", &composite_module, &composite_bgl, half, Some(wgpu::BlendState::REPLACE));

        // Additive blend (CopyShader + AdditiveBlending: src rgb scaled
        // by src alpha, dst kept — WebGLState's AdditiveBlending).
        let add_bgl = bgl("bloom add layout", &[tex_entry(0), smp_entry(1)]);
        let add_module = module("bloom copy shader", Cow::Borrowed(COPY_WGSL));
        let add_blend = wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::SrcAlpha,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::SrcAlpha,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Add,
            },
        };
        let add_pipeline = fs_pipeline(device, "bloom add pipeline", &add_module, &add_bgl, half, Some(add_blend));

        // OutputPass (exposure as uniform).
        let output_bgl = bgl("output layout", &[tex_entry(0), smp_entry(1), ubo_entry(2)]);
        let output_module = module("output shader", Cow::Borrowed(include_str!("shader_output_exp.wgsl")));
        let output_pipeline = fs_pipeline(device, "output pipeline", &output_module, &output_bgl, byte, Some(wgpu::BlendState::REPLACE));

        // GRADE.
        let grade_bgl = bgl("grade layout", &[ubo_entry(0), tex_entry(1), smp_entry(2)]);
        let grade_module = module("grade shader", Cow::Borrowed(include_str!("shader_grade.wgsl")));
        let grade_pipeline = fs_pipeline(device, "grade pipeline", &grade_module, &grade_bgl, byte, Some(wgpu::BlendState::REPLACE));

        PostChain {
            sampler,
            bright_pipeline,
            bright_bgl,
            blur_pipelines,
            blur_bgl,
            composite_pipeline,
            composite_bgl,
            add_pipeline,
            add_bgl,
            output_pipeline,
            output_bgl,
            grade_pipeline,
            grade_bgl,
            bloom_mem: std::cell::Cell::new(None),
        }
    }

    fn tex_bg(&self, device: &wgpu::Device, layout: &wgpu::BindGroupLayout, ub: Option<&wgpu::Buffer>, views: &[&wgpu::TextureView]) -> wgpu::BindGroup {
        let mut entries: Vec<wgpu::BindGroupEntry> = Vec::new();
        let mut binding = 0u32;
        if let Some(ub) = ub {
            entries.push(wgpu::BindGroupEntry { binding, resource: ub.as_entire_binding() });
            binding += 1;
        }
        for v in views {
            entries.push(wgpu::BindGroupEntry { binding, resource: wgpu::BindingResource::TextureView(v) });
            binding += 1;
        }
        entries.push(wgpu::BindGroupEntry { binding, resource: wgpu::BindingResource::Sampler(&self.sampler) });
        device.create_bind_group(&wgpu::BindGroupDescriptor { label: Some("post bind group"), layout, entries: &entries })
    }

    /// UnrealBloomPass, end to end: bright pass on `input`, the
    /// 5-level blur cascade, the composite, and the additive blend of
    /// the composite over `target` (the read buffer — loaded, not
    /// cleared). `width`/`height` are the FULL frame size; mip sizes
    /// derive from it exactly like the addon.
    pub(crate) fn bloom(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        input: &wgpu::TextureView,
        target: &wgpu::TextureView,
        width: u32,
        height: u32,
        params: &BloomParams,
    ) {
        // This call's uniform buffers are transients: the scope
        // rolls their class-G registration back at return (the
        // buffers drop with the call). The chain textures below
        // are NOT registered per call — see the class-C note.
        let _mem_scope = crate::gpucore::mem_scope();
        // VRAM ledger, class C (VRAM-IMPL-W84 ruling D2 —
        // retained registration): the chain's 11 textures are
        // created and dropped on EVERY bloom() call, but during
        // any frame's post pass exactly one chain is live, and
        // one is live in every frame — so the chain is counted
        // ONCE, as a standing working set, at this instance's
        // first bloom for a given frame size (re-registered if
        // the size ever changes mid-instance; in today's code
        // it never does). A future pipelined overlap of two
        // chains would under-count peak by one chain
        // (15,196,800 at 1080×1920) — this comment is part of
        // the convention, not decoration.
        match self.bloom_mem.get() {
            Some((w, h)) if (w, h) == (width, height) => {}
            Some((w, h)) => {
                crate::gpucore::mem_free_texture(crate::gpucore::MemClass::C, bloom_chain_bytes(w, h));
                crate::gpucore::mem_alloc_texture(crate::gpucore::MemClass::C, bloom_chain_bytes(width, height));
                self.bloom_mem.set(Some((width, height)));
            }
            None => {
                crate::gpucore::mem_alloc_texture(crate::gpucore::MemClass::C, bloom_chain_bytes(width, height));
                self.bloom_mem.set(Some((width, height)));
            }
        }
        let sizes = bloom_mip_sizes(width, height);
        let bright_tex = make_rt(device, sizes[0].0, sizes[0].1, wgpu::TextureFormat::Rgba16Float, "bloom bright");
        let bright_view = bright_tex.create_view(&wgpu::TextureViewDescriptor::default());
        {
            let ub = make_buffer(device, queue, "bloom bright uniforms", &pack_f32(&[params.threshold, 0.0, 0.0, 0.0]), wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST);
            let bg = self.tex_bg(device, &self.bright_bgl, Some(&ub), &[input]);
            draw_fs(encoder, "bloom bright pass", &self.bright_pipeline, &bg, &bright_view, wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT));
        }
        // Blur cascade: input = bright, then V[i-1].
        let mut horiz: Vec<wgpu::Texture> = Vec::new();
        let mut vert: Vec<wgpu::Texture> = Vec::new();
        for i in 0..5 {
            horiz.push(make_rt(device, sizes[i].0, sizes[i].1, wgpu::TextureFormat::Rgba16Float, "bloom horiz"));
            vert.push(make_rt(device, sizes[i].0, sizes[i].1, wgpu::TextureFormat::Rgba16Float, "bloom vert"));
        }
        let mut input_view = bright_tex.create_view(&wgpu::TextureViewDescriptor::default());
        for i in 0..5 {
            let (w, h) = sizes[i];
            let inv = [1.0 / w as f32, 1.0 / h as f32, 0.0, 0.0];
            let ub = make_buffer(device, queue, "bloom blur uniforms", &pack_f32(&inv), wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST);
            let hv = horiz[i].create_view(&wgpu::TextureViewDescriptor::default());
            let vv = vert[i].create_view(&wgpu::TextureViewDescriptor::default());
            let bg_h = self.tex_bg(device, &self.blur_bgl, Some(&ub), &[&input_view]);
            draw_fs(encoder, "bloom blur H", &self.blur_pipelines[i][0], &bg_h, &hv, wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT));
            let bg_v = self.tex_bg(device, &self.blur_bgl, Some(&ub), &[&hv]);
            draw_fs(encoder, "bloom blur V", &self.blur_pipelines[i][1], &bg_v, &vv, wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT));
            input_view = vv;
        }
        // Composite into horiz[0].
        let comp_view = horiz[0].create_view(&wgpu::TextureViewDescriptor::default());
        {
            let ub = make_buffer(device, queue, "bloom composite uniforms", &pack_f32(&[params.strength, params.radius, 0.0, 0.0]), wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST);
            let vviews: Vec<wgpu::TextureView> = vert.iter().map(|t| t.create_view(&wgpu::TextureViewDescriptor::default())).collect();
            let refs: Vec<&wgpu::TextureView> = vviews.iter().collect();
            let bg = self.tex_bg(device, &self.composite_bgl, Some(&ub), &refs);
            draw_fs(encoder, "bloom composite pass", &self.composite_pipeline, &bg, &comp_view, wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT));
        }
        // Additive blend over the read buffer.
        {
            let bg = self.tex_bg(device, &self.add_bgl, None, &[&comp_view]);
            draw_fs(encoder, "bloom add pass", &self.add_pipeline, &bg, target, wgpu::LoadOp::Load);
        }
    }

    /// OutputPass: NeutralToneMapping(exposure) + sRGB OETF,
    /// HalfFloat in, Rgba8Unorm (stored sRGB values) out.
    pub(crate) fn output(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        input: &wgpu::TextureView,
        target: &wgpu::TextureView,
        exposure: f32,
    ) {
        // VRAM ledger: this call's uniform buffer is a
        // transient (see bloom's scope note).
        let _mem_scope = crate::gpucore::mem_scope();
        let ub = make_buffer(device, queue, "output uniforms", &pack_f32(&[exposure, 0.0, 0.0, 0.0]), wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST);
        // The output layout orders its bindings tex(0), sampler(1),
        // uniform(2) — matching shader_output_exp.wgsl — so this bind
        // group is built explicitly rather than through tex_bg.
        let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("output bind group"),
            layout: &self.output_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(input) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&self.sampler) },
                wgpu::BindGroupEntry { binding: 2, resource: ub.as_entire_binding() },
            ],
        });
        draw_fs(encoder, "output pass", &self.output_pipeline, &bg, target, wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT));
    }

    /// GRADE: sRGB-space vignette/grain/warmth/saturation,
    /// Rgba8Unorm in and out (no colour conversion either side).
    pub(crate) fn grade(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        input: &wgpu::TextureView,
        target: &wgpu::TextureView,
        params: &GradeParams,
    ) {
        // VRAM ledger: this call's uniform buffer is a
        // transient (see bloom's scope note).
        let _mem_scope = crate::gpucore::mem_scope();
        let data: [f32; 8] = [params.vignette, params.grain, params.warm, params.sat, params.time, 0.0, 0.0, 0.0];
        let ub = make_buffer(device, queue, "grade uniforms", &pack_f32(&data), wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST);
        let bg = self.tex_bg(device, &self.grade_bgl, Some(&ub), &[input]);
        draw_fs(encoder, "grade pass", &self.grade_pipeline, &bg, target, wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bloom_sizes_match_the_addon() {
        // UnrealBloomPass at resolution 1080x1920: round(1080/2)=540,
        // halving with Math.round: 540, 270, 135, 68, 34 (67.5 -> 68).
        assert_eq!(
            bloom_mip_sizes(1080, 1920),
            [(540, 960), (270, 480), (135, 240), (68, 120), (34, 60)]
        );
    }

    #[test]
    fn bloom_chain_bytes_matches_w84_arithmetic() {
        // W84 §3.2: mips sum to 690,600 px at 1080x1920;
        // C = 8 x (518,400 + 2 x 690,600) = 15,196,800.
        assert_eq!(bloom_chain_bytes(1080, 1920), 15_196_800);
        // The --draft size from W84 §3.3: mips (270,480),
        // (135,240), (68,120), (34,60), (17,30) = 172,710 px;
        // C = 8 x (129,600 + 2 x 172,710) = 3,800,160.
        assert_eq!(bloom_chain_bytes(540, 960), 3_800_160);
    }

    #[test]
    fn half_decode_known_values() {
        assert_eq!(half_to_f32(0x3c00), 1.0);
        assert_eq!(half_to_f32(0x0000), 0.0);
        assert_eq!(half_to_f32(0xbc00), -1.0);
        assert_eq!(half_to_f32(0x3555), 0.33325195);
        assert!(half_to_f32(0x7c00).is_infinite());
    }

    #[test]
    fn room_env_asset_matches_its_capture_log() {
        // docs/ab/composer: maxValue 56.3125, meanG 0.67713.
        let (max, mean_g) = room_env_stats();
        assert!((max - 56.3125).abs() < 0.01, "max {max}");
        assert!((mean_g - 0.67713).abs() < 0.002, "meanG {mean_g}");
    }
}
