//! Milestone 3 gate scene: the DoF chart (compatibility contract §1).
//!
//! A controlled chart — textured panels at known depths, fixed camera,
//! focus 10 m, production bokeh defaults (aperture 0.0015, maxblur 0.006)
//! — rendered through the native transcription of three.js r170's
//! BokehPass so the blur-vs-depth curve can be measured against the real
//! engine (docs/ab/dof-chart/).
//!
//! Pass chain (mirrors the production composer chain for the parts the
//! chart's look does not neutralise):
//!   1. scene colour -> Rgba16Float, MSAA 4x, resolved
//!      (engine.ts: EffectComposer over a HalfFloatType RT, samples = 4)
//!   2. depth prepass -> packed-depth target, cleared to white;
//!      MeshDepthMaterial + RGBADepthPacking equivalent
//!      (shader_depth.wgsl), single-sample, NearestFilter on read.
//!      Storage is Rgba8Unorm (lossless for a byte packing on every
//!      driver) rather than BokehPass's HalfFloat — a deliberate,
//!      measured deviation documented in docs/ab/dof-chart/gaps.md;
//!      TALERENDER_DOF_DEPTH_FORMAT=halffloat restores the exact
//!      BokehPass target configuration.
//!   3. bokeh composite (shader_bokeh.wgsl) -> Rgba16Float
//!   4. output stage (shader_output.wgsl): NeutralToneMapping at
//!      exposure 1 + sRGB OETF — the OutputPass steps. Bloom (strength 0)
//!      and GRADE (vignette/grain/warm 0, sat 1) are exact identities for
//!      this look, so they are not reproduced.
//!
//! The chart spec constants below are shared verbatim with the reference
//! project's src/shots.ts; both sides derive geometry and texture from
//! them, never by eyeballing.

use crate::math;

pub const FOCUS: f32 = 10.0;
pub const APERTURE: f32 = 0.0015;
pub const MAXBLUR: f32 = 0.006;
pub const NEAR: f32 = 0.05;
pub const FAR: f32 = 800.0;
pub const FOV_Y_DEG: f32 = 30.0;
/// (ndc_x, ndc_y, depth_m), ndc_y up. The 2 m panel overlaps the bottom
/// of the 10 m focal panel on purpose (foreground-bleed test case).
pub const PANELS: [(f32, f32, f32); 10] = [
    (-0.62, 0.66, 30.0),
    (0.0, 0.66, 14.0),
    (0.62, 0.66, 11.0),
    (-0.62, 0.0, 12.0),
    (0.0, 0.0, 10.0),
    (0.62, 0.0, 8.0),
    (-0.62, -0.66, 7.0),
    (0.0, -0.66, 5.0),
    (0.62, -0.66, 3.5),
    (0.0, -0.30, 2.0),
];
pub const PANEL_NDC_W: f32 = 0.56;
pub const PANEL_NDC_H: f32 = 0.36;
pub const BLEED_NDC_H: f32 = 0.30;
pub const TEX_SIZE: u32 = 512;

const SHADER_TEX: &str = include_str!("shader_tex.wgsl");
const SHADER_DEPTH: &str = include_str!("shader_depth.wgsl");
const SHADER_BOKEH: &str = include_str!("shader_bokeh.wgsl");
const SHADER_OUTPUT: &str = include_str!("shader_output.wgsl");

struct Uniforms {
    mvp: math::Mat4,
    model: math::Mat4,
}

// Byte packing follows ab1.rs's house pattern (the crate carries no
// bytemuck dependency): native-endian f32/u32 streams, column-major
// matrices.
fn mat_bytes(m: &math::Mat4) -> [u8; 64] {
    let mut out = [0u8; 64];
    let mut k = 0;
    for col in 0..4 {
        for row in 0..4 {
            out[k..k + 4].copy_from_slice(&m[col][row].to_ne_bytes());
            k += 4;
        }
    }
    out
}

fn uniform_bytes(u: &Uniforms) -> Vec<u8> {
    let mut out = Vec::with_capacity(128);
    out.extend_from_slice(&mat_bytes(&u.mvp));
    out.extend_from_slice(&mat_bytes(&u.model));
    out
}

fn pack_f32(data: &[f32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len() * 4);
    for v in data {
        out.extend_from_slice(&v.to_ne_bytes());
    }
    out
}

fn pack_u32(data: &[u32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len() * 4);
    for v in data {
        out.extend_from_slice(&v.to_ne_bytes());
    }
    out
}

/// All panels in one vertex/index set (flat pos xyz + uv floats),
/// camera at the origin looking down -Z, so model = view = identity
/// and mvp = proj.
fn panel_geometry(aspect: f32) -> (Vec<f32>, Vec<u32>) {
    let tan_h = (FOV_Y_DEG * 0.5).to_radians().tan();
    let mut verts: Vec<f32> = Vec::new();
    let mut idx: Vec<u32> = Vec::new();
    for &(nx, ny, d) in PANELS.iter() {
        let hh = d * tan_h;
        let hw = hh * aspect;
        let h_ndc = if d == 2.0 { BLEED_NDC_H } else { PANEL_NDC_H };
        let w = PANEL_NDC_W * hw;
        let h = h_ndc * hh;
        let cx = nx * hw;
        let cy = ny * hh;
        let z = -d;
        let base = (verts.len() / 5) as u32;
        // BL, BR, TR, TL — uv.y picks the pattern row the same way
        // three's flipY CanvasTexture + PlaneGeometry uvs do: panel top
        // samples pattern w ~ 0 (buffer row 0).
        verts.extend_from_slice(&[cx - w / 2.0, cy - h / 2.0, z, 0.0, 1.0]);
        verts.extend_from_slice(&[cx + w / 2.0, cy - h / 2.0, z, 1.0, 1.0]);
        verts.extend_from_slice(&[cx + w / 2.0, cy + h / 2.0, z, 1.0, 0.0]);
        verts.extend_from_slice(&[cx - w / 2.0, cy + h / 2.0, z, 0.0, 0.0]);
        idx.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    (verts, idx)
}

pub async fn run_dof(width: u32, height: u32, out: &str) -> crate::errors::TaleResult<()> {
    let instance = crate::gpucore::new_instance();
    let adapter = crate::gpucore::request_adapter(&instance).await;
    let info = adapter.get_info();
    println!(
        "adapter: {} ({:?}, driver {}) — software rasteriser, no performance claim",
        info.name, info.backend, info.driver
    );
    let (device, queue) =
        crate::gpucore::request_device_default(&adapter, "talerender-dof-device").await;

    let aspect = width as f32 / height as f32;
    // Packed-depth storage format. The pack is a BYTE packing, so a
    // byte target stores it losslessly and identically on every
    // driver. BokehPass itself uses a HalfFloat target, whose
    // fp32->fp16 store conversion is driver-dependent: on this VM's
    // llvmpipe it truncates, costing up to 1 fp16 ulp on the top byte
    // (~0.9 m of viewZ error at the 10 m focal plane — measured in
    // docs/ab/dof-chart/). The native default is therefore
    // Rgba8Unorm; TALERENDER_DOF_DEPTH_FORMAT=halffloat reproduces
    // BokehPass's exact target configuration for comparison. The
    // packing/unpacking ARITHMETIC is identical either way.
    let depth_format = if std::env::var("TALERENDER_DOF_DEPTH_FORMAT").is_ok_and(|v| v == "halffloat") {
        wgpu::TextureFormat::Rgba16Float
    } else {
        wgpu::TextureFormat::Rgba8Unorm
    };
    let proj = math::perspective(FOV_Y_DEG.to_radians(), aspect, NEAR, FAR);
    let uniforms = Uniforms { mvp: proj, model: math::identity() };
    let uniform_buf = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("dof-uniforms"),
        size: 128,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    queue.write_buffer(&uniform_buf, 0, &uniform_bytes(&uniforms));

    // ---- chart texture (sRGB, single level, linear sampling) ----
    let tex_data = crate::scene::chart_texture_rgba(TEX_SIZE);
    let chart_tex = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("dof-chart-texture"),
        size: wgpu::Extent3d { width: TEX_SIZE, height: TEX_SIZE, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        wgpu::ImageCopyTexture { texture: &chart_tex, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
        &tex_data,
        wgpu::ImageDataLayout { offset: 0, bytes_per_row: Some(TEX_SIZE * 4), rows_per_image: Some(TEX_SIZE) },
        wgpu::Extent3d { width: TEX_SIZE, height: TEX_SIZE, depth_or_array_layers: 1 },
    );
    let chart_view = chart_tex.create_view(&wgpu::TextureViewDescriptor::default());
    let linear_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("dof-linear-sampler"),
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::FilterMode::Nearest,
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        ..Default::default()
    });
    let nearest_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("dof-nearest-sampler"),
        mag_filter: wgpu::FilterMode::Nearest,
        min_filter: wgpu::FilterMode::Nearest,
        mipmap_filter: wgpu::FilterMode::Nearest,
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        ..Default::default()
    });

    // ---- geometry ----
    let (verts, idx) = panel_geometry(aspect);
    let vb = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("dof-panels-vb"),
        size: (verts.len() * 4) as u64,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    queue.write_buffer(&vb, 0, &pack_f32(&verts));
    let ib = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("dof-panels-ib"),
        size: (idx.len() * 4) as u64,
        usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    queue.write_buffer(&ib, 0, &pack_u32(&idx));
    let index_count = idx.len() as u32;
    let vlayout = wgpu::VertexBufferLayout {
        array_stride: 20,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &[
            wgpu::VertexAttribute { offset: 0, shader_location: 0, format: wgpu::VertexFormat::Float32x3 },
            wgpu::VertexAttribute { offset: 12, shader_location: 1, format: wgpu::VertexFormat::Float32x2 },
        ],
    };

    // ---- targets ----
    // Scene colour: HalfFloat + MSAA 4x (the composer's render target).
    let scene_msaa = crate::gpucore::make_target(&device, width, height, wgpu::TextureFormat::Rgba16Float, 4, wgpu::TextureUsages::RENDER_ATTACHMENT, "dof-scene-msaa");
    let scene_resolved = crate::gpucore::make_target(&device, width, height, wgpu::TextureFormat::Rgba16Float, 1, wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING, "dof-scene-resolved");
    let scene_depth = crate::gpucore::make_target(&device, width, height, wgpu::TextureFormat::Depth32Float, 4, wgpu::TextureUsages::RENDER_ATTACHMENT, "dof-scene-depth");
    // Packed-depth prepass target: HalfFloat, single-sample, cleared to
    // white (BokehPass renders its depth target with a white clear).
    let packed_depth = crate::gpucore::make_target(&device, width, height, depth_format, 1, wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_SRC, "dof-packed-depth");
    let prepass_depth = crate::gpucore::make_target(&device, width, height, wgpu::TextureFormat::Depth32Float, 1, wgpu::TextureUsages::RENDER_ATTACHMENT, "dof-prepass-depth");
    let bokeh_out = crate::gpucore::make_target(&device, width, height, wgpu::TextureFormat::Rgba16Float, 1, wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING, "dof-bokeh-out");
    let final_tex = crate::gpucore::make_target(&device, width, height, wgpu::TextureFormat::Rgba8Unorm, 1, wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC, "dof-final");

    // ---- pipelines ----
    let tex_module = device.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some("dof-shader-tex"), source: wgpu::ShaderSource::Wgsl(SHADER_TEX.into()) });
    let tex_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("dof-tex-bgl"),
        entries: &[
            wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::VERTEX, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None }, count: None },
            wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: true }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }, count: None },
            wgpu::BindGroupLayoutEntry { binding: 2, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering), count: None },
        ],
    });
    let tex_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("dof-tex-bg"),
        layout: &tex_bgl,
        entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: uniform_buf.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&chart_view) },
            wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&linear_sampler) },
        ],
    });
    let scene_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("dof-scene-pipeline"),
        layout: Some(&device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: Some("dof-scene-layout"), bind_group_layouts: &[&tex_bgl], push_constant_ranges: &[] })),
        vertex: wgpu::VertexState { module: &tex_module, entry_point: "vs_main", buffers: &[vlayout.clone()], compilation_options: Default::default() },
        fragment: Some(wgpu::FragmentState {
            module: &tex_module,
            entry_point: "fs_main",
            targets: &[Some(wgpu::ColorTargetState { format: wgpu::TextureFormat::Rgba16Float, blend: Some(wgpu::BlendState::REPLACE), write_mask: wgpu::ColorWrites::ALL })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState { cull_mode: None, ..Default::default() },
        depth_stencil: Some(wgpu::DepthStencilState { format: wgpu::TextureFormat::Depth32Float, depth_write_enabled: true, depth_compare: wgpu::CompareFunction::Less, stencil: Default::default(), bias: Default::default() }),
        multisample: wgpu::MultisampleState { count: 4, ..Default::default() },
        multiview: None,
    });

    let depth_module = device.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some("dof-shader-depth"), source: wgpu::ShaderSource::Wgsl(SHADER_DEPTH.into()) });
    let depth_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("dof-depth-bgl"),
        entries: &[wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::VERTEX, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None }, count: None }],
    });
    let depth_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("dof-depth-bg"),
        layout: &depth_bgl,
        entries: &[wgpu::BindGroupEntry { binding: 0, resource: uniform_buf.as_entire_binding() }],
    });
    let depth_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("dof-depth-pipeline"),
        layout: Some(&device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: Some("dof-depth-layout"), bind_group_layouts: &[&depth_bgl], push_constant_ranges: &[] })),
        vertex: wgpu::VertexState { module: &depth_module, entry_point: "vs_main", buffers: &[vlayout], compilation_options: Default::default() },
        fragment: Some(wgpu::FragmentState {
            module: &depth_module,
            entry_point: "fs_main",
            targets: &[Some(wgpu::ColorTargetState { format: depth_format, blend: Some(wgpu::BlendState::REPLACE), write_mask: wgpu::ColorWrites::ALL })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState { cull_mode: None, ..Default::default() },
        depth_stencil: Some(wgpu::DepthStencilState { format: wgpu::TextureFormat::Depth32Float, depth_write_enabled: true, depth_compare: wgpu::CompareFunction::Less, stencil: Default::default(), bias: Default::default() }),
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
    });

    // WGSL BokehUniforms layout: six f32 fields + two padding slots.
    // Debug knobs for the gate investigation (not part of the chart
    // spec): TALERENDER_DOF_APERTURE overrides the aperture uniform —
    // 0 makes the gather an exact identity, isolating the pre-bokeh
    // scene pass; TALERENDER_DOF_DUMP_DEPTH=1 reads the packed-depth
    // target back and prints the recovered viewZ per panel centre.
    let aperture = std::env::var("TALERENDER_DOF_APERTURE")
        .ok()
        .and_then(|v| v.parse::<f32>().ok())
        .unwrap_or(APERTURE);
    let dump_depth = std::env::var("TALERENDER_DOF_DUMP_DEPTH").is_ok_and(|v| v == "1");
    let bokeh_uniform_data: [f32; 8] = [FOCUS, aspect, aperture, MAXBLUR, NEAR, FAR, 0.0, 0.0];
    let bokeh_uniform_buf = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("dof-bokeh-uniforms"),
        size: 32,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    queue.write_buffer(&bokeh_uniform_buf, 0, &pack_f32(&bokeh_uniform_data));
    let bokeh_module = device.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some("dof-shader-bokeh"), source: wgpu::ShaderSource::Wgsl(SHADER_BOKEH.into()) });
    let bokeh_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("dof-bokeh-bgl"),
        entries: &[
            wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None }, count: None },
            wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: true }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }, count: None },
            wgpu::BindGroupLayoutEntry { binding: 2, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering), count: None },
            wgpu::BindGroupLayoutEntry { binding: 3, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: true }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }, count: None },
            wgpu::BindGroupLayoutEntry { binding: 4, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering), count: None },
        ],
    });
    let scene_view = scene_resolved.create_view(&wgpu::TextureViewDescriptor::default());
    let packed_view = packed_depth.create_view(&wgpu::TextureViewDescriptor::default());
    let bokeh_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("dof-bokeh-bg"),
        layout: &bokeh_bgl,
        entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: bokeh_uniform_buf.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&scene_view) },
            wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&linear_sampler) },
            wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::TextureView(&packed_view) },
            wgpu::BindGroupEntry { binding: 4, resource: wgpu::BindingResource::Sampler(&nearest_sampler) },
        ],
    });
    let bokeh_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("dof-bokeh-pipeline"),
        layout: Some(&device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: Some("dof-bokeh-layout"), bind_group_layouts: &[&bokeh_bgl], push_constant_ranges: &[] })),
        vertex: wgpu::VertexState { module: &bokeh_module, entry_point: "vs_main", buffers: &[], compilation_options: Default::default() },
        fragment: Some(wgpu::FragmentState {
            module: &bokeh_module,
            entry_point: "fs_main",
            targets: &[Some(wgpu::ColorTargetState { format: wgpu::TextureFormat::Rgba16Float, blend: Some(wgpu::BlendState::REPLACE), write_mask: wgpu::ColorWrites::ALL })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
    });

    let output_module = device.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some("dof-shader-output"), source: wgpu::ShaderSource::Wgsl(SHADER_OUTPUT.into()) });
    let output_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("dof-output-bgl"),
        entries: &[
            wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: true }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }, count: None },
            wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering), count: None },
        ],
    });
    let bokeh_view = bokeh_out.create_view(&wgpu::TextureViewDescriptor::default());
    let output_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("dof-output-bg"),
        layout: &output_bgl,
        entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&bokeh_view) },
            wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&linear_sampler) },
        ],
    });
    let output_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("dof-output-pipeline"),
        layout: Some(&device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: Some("dof-output-layout"), bind_group_layouts: &[&output_bgl], push_constant_ranges: &[] })),
        vertex: wgpu::VertexState { module: &output_module, entry_point: "vs_main", buffers: &[], compilation_options: Default::default() },
        fragment: Some(wgpu::FragmentState {
            module: &output_module,
            entry_point: "fs_main",
            targets: &[Some(wgpu::ColorTargetState { format: wgpu::TextureFormat::Rgba8Unorm, blend: Some(wgpu::BlendState::REPLACE), write_mask: wgpu::ColorWrites::ALL })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
    });

    // ---- render ----
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("dof-encoder") });
    {
        let msaa_view = scene_msaa.create_view(&wgpu::TextureViewDescriptor::default());
        let sdepth_view = scene_depth.create_view(&wgpu::TextureViewDescriptor::default());
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("dof-scene-pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &msaa_view,
                resolve_target: Some(&scene_view),
                ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 }), store: wgpu::StoreOp::Store },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &sdepth_view,
                depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(1.0), store: wgpu::StoreOp::Store }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        pass.set_pipeline(&scene_pipeline);
        pass.set_bind_group(0, &tex_bg, &[]);
        pass.set_vertex_buffer(0, vb.slice(..));
        pass.set_index_buffer(ib.slice(..), wgpu::IndexFormat::Uint32);
        pass.draw_indexed(0..index_count, 0, 0..1);
    }
    {
        let packed_v = packed_depth.create_view(&wgpu::TextureViewDescriptor::default());
        let pdepth_view = prepass_depth.create_view(&wgpu::TextureViewDescriptor::default());
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("dof-depth-prepass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &packed_v,
                resolve_target: None,
                // BokehPass clears its depth target to WHITE before the
                // override render; background pixels then unpack to ~far.
                ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color { r: 1.0, g: 1.0, b: 1.0, a: 1.0 }), store: wgpu::StoreOp::Store },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &pdepth_view,
                depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(1.0), store: wgpu::StoreOp::Store }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        pass.set_pipeline(&depth_pipeline);
        pass.set_bind_group(0, &depth_bg, &[]);
        pass.set_vertex_buffer(0, vb.slice(..));
        pass.set_index_buffer(ib.slice(..), wgpu::IndexFormat::Uint32);
        pass.draw_indexed(0..index_count, 0, 0..1);
    }
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("dof-bokeh-pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &bokeh_view,
                resolve_target: None,
                ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT), store: wgpu::StoreOp::Store },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        pass.set_pipeline(&bokeh_pipeline);
        pass.set_bind_group(0, &bokeh_bg, &[]);
        pass.draw(0..3, 0..1);
    }
    {
        let final_view = final_tex.create_view(&wgpu::TextureViewDescriptor::default());
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("dof-output-pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &final_view,
                resolve_target: None,
                ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT), store: wgpu::StoreOp::Store },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        pass.set_pipeline(&output_pipeline);
        pass.set_bind_group(0, &output_bg, &[]);
        pass.draw(0..3, 0..1);
    }

    let rb = crate::gpucore::Readback::record(&device, &mut encoder, &final_tex, width, height, "dof-readback");
    queue.submit(Some(encoder.finish()));
    let rgba = rb.collect(&device);
    image::save_buffer(out, &rgba, width, height, image::ColorType::Rgba8)
        .map_err(|e| crate::errors::TaleError::image_io(out, e))?;

    if dump_depth {
        dump_packed_depth(&device, &queue, &packed_depth, width, height, depth_format);
    }

    println!(
        "wrote {out} ({width}x{height}) — scene dof-chart: panels at 2..30 m, focus {FOCUS} m, aperture {APERTURE}, maxblur {MAXBLUR}; chain: HalfFloat MSAA-4x scene -> packed-depth prepass -> transcribed r170 bokeh -> Neutral+sRGB output"
    );
    Ok(())
}

/// Debug: read the packed-depth target back and print the recovered
/// viewZ at each panel centre (the number the bokeh shader sees).
fn dump_packed_depth(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    tex: &wgpu::Texture,
    width: u32,
    height: u32,
    format: wgpu::TextureFormat,
) {
    let bytes_per_px: usize = if format == wgpu::TextureFormat::Rgba8Unorm { 4 } else { 8 };
    fn f16_to_f32(h: u16) -> f32 {
        let sign = ((h >> 15) & 1) as u32;
        let exp = ((h >> 10) & 0x1f) as i32;
        let frac = (h & 0x3ff) as u32;
        let bits = if exp == 0 {
            if frac == 0 {
                sign << 31
            } else {
                // subnormal: normalise
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
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("dof-depth-copy") });
    let rb = crate::gpucore::Readback::record_bpp(
        device,
        &mut encoder,
        tex,
        width,
        height,
        bytes_per_px as u32,
        "dof-depth-readback",
    );
    queue.submit(Some(encoder.finish()));
    let (mapped, padded_u32) = rb.collect_padded(device);
    let padded = padded_u32 as usize;
    println!("packed-depth dump (panel centres):");
    for &(nx, ny, d) in PANELS.iter() {
        let px = (((nx + 1.0) / 2.0) * width as f32) as usize;
        let py = ((1.0 - (ny + 1.0) / 2.0) * height as f32) as usize;
        let off = py * padded + px * bytes_per_px;
        let mut ch = [0.0f32; 4];
        for (i, c) in ch.iter_mut().enumerate() {
            *c = if bytes_per_px == 4 {
                mapped[off + i] as f32 / 255.0
            } else {
                f16_to_f32(u16::from_le_bytes([mapped[off + i * 2], mapped[off + i * 2 + 1]]))
            };
        }
        let view_z = depth_to_view_z(unpack_rgba_depth(ch), NEAR, FAR);
        println!("  panel d={d:>5} m @({px},{py}): packed={ch:?} -> viewZ {view_z:.4} (expect {})", -d);
    }
}

// ---- CPU mirrors of the packing arithmetic (transcription tests) ----

fn modf_pos(x: f32) -> (f32, f32) {
    let w = x.floor();
    (x - w, w)
}

/// packDepthToRGBA, f32 mirror of shader_depth.wgsl.
#[cfg(test)]
fn pack_depth_rgba(v: f32) -> [f32; 4] {
    if v <= 0.0 {
        return [0.0; 4];
    }
    if v >= 1.0 {
        return [1.0; 4];
    }
    let (af, mut vuf) = modf_pos(v * 16777216.0);
    let (bf, vuf2) = modf_pos(vuf * (1.0 / 256.0));
    vuf = vuf2;
    let (gf, vuf3) = modf_pos(vuf * (1.0 / 256.0));
    vuf = vuf3;
    [vuf * (1.0 / 255.0), gf * (256.0 / 255.0), bf * (256.0 / 255.0), af]
}

/// unpackRGBAToDepth, f32 mirror of shader_bokeh.wgsl.
fn unpack_rgba_depth(v: [f32; 4]) -> f32 {
    let downscale = 255.0f32 / 256.0;
    v[0] * (downscale / 1.0) + v[1] * (downscale / 256.0) + v[2] * (downscale / 65536.0) + v[3] * (1.0 / 16777216.0)
}

/// perspectiveDepthToViewZ.
fn depth_to_view_z(depth: f32, near: f32, far: f32) -> f32 {
    (near * far) / ((far - near) * depth - far)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frag_z_at(distance: f32) -> f32 {
        let proj = math::perspective(FOV_Y_DEG.to_radians(), 1080.0 / 1920.0, NEAR, FAR);
        let clip = math::mul_vec4(&proj, [0.0, 0.0, -distance, 1.0]);
        // math::perspective is the wgpu-style projection: clip z/w is
        // already in [0, 1]. three's fragCoordZ is 0.5*ndc+0.5 of the GL
        // [-1, 1] value — algebraically the same number (both equal
        // far*(d-near) / ((far-near)*d)).
        clip[2] / clip[3]
    }

    #[test]
    fn pack_unpack_round_trip_recovers_chart_depths() {
        for &d in &[2.0f32, 3.5, 5.0, 7.0, 8.0, 10.0, 11.0, 12.0, 14.0, 30.0] {
            let packed = pack_depth_rgba(frag_z_at(d));
            let view_z = depth_to_view_z(unpack_rgba_depth(packed), NEAR, FAR);
            let err = (view_z + d).abs();
            assert!(err < 0.05, "depth {d} m recovered as {} m (err {err})", -view_z);
        }
        // Endpoints of the packing contract.
        assert_eq!(pack_depth_rgba(0.0), [0.0; 4]);
        assert_eq!(pack_depth_rgba(1.0), [1.0; 4]);
        // Background clear (1,1,1,1) unpacks to ~far, not to an error.
        let bg = depth_to_view_z(unpack_rgba_depth([1.0; 4]), NEAR, FAR);
        assert!(bg < -799.0, "white clear should read as far plane, got {bg}");
    }

    #[test]
    fn dofblur_curve_matches_shader_formula() {
        // factor = focus + viewZ; dofblur = clamp(factor * aperture, +-maxblur)
        let cases: [(f32, f32); 10] = [
            (2.0, 0.006),
            (3.5, 0.006),
            (5.0, 0.006),
            (7.0, 0.0045),
            (8.0, 0.003),
            (10.0, 0.0),
            (11.0, -0.0015),
            (12.0, -0.003),
            (14.0, -0.006),
            (30.0, -0.006),
        ];
        for (d, expect) in cases {
            let view_z = depth_to_view_z(unpack_rgba_depth(pack_depth_rgba(frag_z_at(d))), NEAR, FAR);
            let blur = ((FOCUS + view_z) * APERTURE).clamp(-MAXBLUR, MAXBLUR);
            assert!((blur - expect).abs() < 0.0004, "depth {d}: dofblur {blur}, expected ~{expect}");
        }
    }
}
