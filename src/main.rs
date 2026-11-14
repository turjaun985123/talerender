//! TaleRender — native Rust + wgpu renderer for Tall Tale Shorts.
//!
//! Milestone 1, step 2: a real (if small) 3D scene rendered headless —
//! a perspective camera (36 deg, the Tall Tale house FOV), a lit cube, a
//! ground plane, and a textured character card that yaw-billboards
//! toward the camera the way the existing engine's `face()` does.
//! Depth-tested, offscreen, PNG out. No browser, no window, no display
//! server. Shadows, real materials and the composer chain are later
//! milestones — this file claims none of them.
//!
//! The adapter actually used is printed: on GPU-less machines wgpu
//! falls back to a software Vulkan driver (lavapipe/llvmpipe) and the
//! log says so. Nothing here claims GPU acceleration it cannot prove.
//!
//! Usage: talerender [--width 1080] [--height 1920] [--out frame.png]
//!        [--scene m1|ab1|dof-chart] [--caption <caption-strip.png>]
//!        [--scene-file <scene.json>]
//!        [--scene stroke-battery] [--out <dir>]  (M4 battery cards)
//!        [--telemetry ab1|ab1-file|ab1-variant] [--captions <metrics.json>]
//!        [--cutouts-out <cutouts.json>]  (M6 QA telemetry, contract §3)

mod ab1;
mod battery;
mod composer;
mod dof;
mod math;
mod scene;
mod scenefile;
mod telemetry;
mod toonfull;

use math::Mat4;
use std::borrow::Cow;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut width: u32 = 1080;
    let mut height: u32 = 1920;
    let mut out = String::from("frame.png");
    let mut scene = String::from("m1");
    let mut caption = String::from("docs/ab/onoda-s01-t150/assets/caption.png");
    let mut scene_file: Option<String> = None;
    // M5 ablation control for --scene ab1: how much of the composer
    // chain runs (lighting | bloom | grade). Default is the full
    // chain, matching production.
    let mut post_stage = String::from("grade");
    let mut telemetry_mode: Option<String> = None;
    let mut captions_path: Option<String> = None;
    let mut cutouts_out: Option<String> = None;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--width" => {
                i += 1;
                width = args.get(i).and_then(|v| v.parse().ok()).expect("bad --width");
            }
            "--height" => {
                i += 1;
                height = args.get(i).and_then(|v| v.parse().ok()).expect("bad --height");
            }
            "--out" => {
                i += 1;
                out = args.get(i).expect("missing value for --out").clone();
            }
            "--scene" => {
                i += 1;
                scene = args.get(i).expect("missing value for --scene").clone();
            }
            "--scene-file" => {
                i += 1;
                scene_file = Some(args.get(i).expect("missing value for --scene-file").clone());
            }
            "--caption" => {
                i += 1;
                caption = args.get(i).expect("missing value for --caption").clone();
            }
            "--post-stage" => {
                i += 1;
                post_stage = args.get(i).expect("missing value for --post-stage").clone();
            }
            "--telemetry" => {
                i += 1;
                telemetry_mode = Some(args.get(i).expect("missing value for --telemetry").clone());
            }
            "--captions" => {
                i += 1;
                captions_path = Some(args.get(i).expect("missing value for --captions").clone());
            }
            "--cutouts-out" => {
                i += 1;
                cutouts_out = Some(args.get(i).expect("missing value for --cutouts-out").clone());
            }
            other => {
                eprintln!("unknown argument: {other}");
                std::process::exit(2);
            }
        }
        i += 1;
    }
    if let Some(mode) = &telemetry_mode {
        // M6 QA telemetry (contract §3): pure CPU — the painter
        // canvas + scene-graph projection; no GPU device is created.
        std::process::exit(telemetry::run(
            mode,
            scene_file.as_deref(),
            captions_path.as_deref(),
            cutouts_out.as_deref(),
        ));
    }
    if let Some(path) = &scene_file {
        // A scene file wins over --scene: the two are alternative
        // scene sources, and the file is the more specific request.
        pollster::block_on(scenefile::run_scene_file(width, height, &out, path));
        return;
    }
    match scene.as_str() {
        "m1" => pollster::block_on(run(width, height, &out)),
        "ab1" => {
            let stage = match post_stage.as_str() {
                "lighting" => 1,
                "bloom" => 2,
                "grade" => 3,
                other => {
                    eprintln!("unknown --post-stage: {other} (expected lighting, bloom or grade)");
                    std::process::exit(2);
                }
            };
            pollster::block_on(ab1::run_ab1(width, height, &out, &caption, stage))
        }
        "dof-chart" => pollster::block_on(dof::run_dof(width, height, &out)),
        "stroke-battery" => battery::run_battery(&out),
        other => {
            eprintln!("unknown scene: {other} (expected m1, ab1, dof-chart or stroke-battery)");
            std::process::exit(2);
        }
    }
}

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

fn mat_bytes(m: &Mat4) -> [u8; 64] {
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

struct Uniforms {
    mvp: Mat4,
    model: Mat4,
}

fn uniform_bytes(u: &Uniforms) -> Vec<u8> {
    let mut out = Vec::with_capacity(128);
    out.extend_from_slice(&mat_bytes(&u.mvp));
    out.extend_from_slice(&mat_bytes(&u.model));
    out
}

/// Vertex for the colour pipeline: position, normal, colour.
fn push_vert(v: &mut Vec<f32>, pos: [f32; 3], nrm: [f32; 3], col: [f32; 3]) {
    v.extend_from_slice(&pos);
    v.extend_from_slice(&nrm);
    v.extend_from_slice(&col);
}

/// A cube of half-extent `h` centred on the origin, 24 verts / 36 indices,
/// per-face colours. Windings verified so each triangle's right-hand-rule
/// normal matches its face normal (pipeline culls back faces).
fn cube(h: f32) -> (Vec<f32>, Vec<u32>) {
    // (normal, colour, four corners CCW seen from outside)
    let faces: [([f32; 3], [f32; 3], [[f32; 3]; 4]); 6] = [
        ([0.0, 0.0, 1.0], [0.85, 0.33, 0.30], [[-h, -h, h], [h, -h, h], [h, h, h], [-h, h, h]]), // +Z front
        ([0.0, 0.0, -1.0], [0.30, 0.45, 0.85], [[h, -h, -h], [-h, -h, -h], [-h, h, -h], [h, h, -h]]), // -Z back
        ([1.0, 0.0, 0.0], [0.40, 0.75, 0.45], [[h, -h, h], [h, -h, -h], [h, h, -h], [h, h, h]]), // +X right
        ([-1.0, 0.0, 0.0], [0.65, 0.45, 0.80], [[-h, -h, -h], [-h, -h, h], [-h, h, h], [-h, h, -h]]), // -X left
        ([0.0, 1.0, 0.0], [0.95, 0.80, 0.35], [[-h, h, h], [h, h, h], [h, h, -h], [-h, h, -h]]), // +Y top
        ([0.0, -1.0, 0.0], [0.25, 0.25, 0.28], [[-h, -h, -h], [h, -h, -h], [h, -h, h], [-h, -h, h]]), // -Y bottom
    ];
    let mut verts = Vec::new();
    let mut idx = Vec::new();
    for (nrm, col, corners) in faces {
        let base = (verts.len() / 9) as u32;
        for c in corners {
            push_vert(&mut verts, c, nrm, col);
        }
        idx.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    (verts, idx)
}

/// Ground plane at y = 0 spanning [-s, s] in x/z, one colour.
fn ground(s: f32, col: [f32; 3]) -> (Vec<f32>, Vec<u32>) {
    let n = [0.0, 1.0, 0.0];
    let corners = [[-s, 0.0, s], [s, 0.0, s], [s, 0.0, -s], [-s, 0.0, -s]];
    let mut verts = Vec::new();
    for c in corners {
        push_vert(&mut verts, c, n, col);
    }
    (verts, vec![0, 1, 2, 0, 2, 3])
}

/// The character-card texture, painted procedurally on the CPU (the M1
/// stand-in for a CanvasTexture): cream card, ink border, eyes, smile.
fn character_texture(size: u32) -> Vec<u8> {
    let mut px = vec![0u8; (size * size * 4) as usize];
    let s = size as i32;
    let set = |px: &mut Vec<u8>, x: i32, y: i32, rgba: [u8; 4]| {
        if x >= 0 && y >= 0 && x < s && y < s {
            let k = ((y * s + x) * 4) as usize;
            px[k..k + 4].copy_from_slice(&rgba);
        }
    };
    let disc = |cx: i32, cy: i32, r: i32| -> Vec<(i32, i32)> {
        let mut out = Vec::new();
        for y in (cy - r)..=(cy + r) {
            for x in (cx - r)..=(cx + r) {
                if (x - cx) * (x - cx) + (y - cy) * (y - cy) <= r * r {
                    out.push((x, y));
                }
            }
        }
        out
    };
    let ink = [35, 35, 40, 255];
    for y in 0..s {
        for x in 0..s {
            set(&mut px, x, y, [245, 238, 220, 255]);
            if x < 8 || y < 8 || x >= s - 8 || y >= s - 8 {
                set(&mut px, x, y, ink);
            }
        }
    }
    for (x, y) in disc(88, 100, 15) {
        set(&mut px, x, y, ink);
    }
    for (x, y) in disc(168, 100, 15) {
        set(&mut px, x, y, ink);
    }
    for (x, y) in disc(70, 142, 13) {
        set(&mut px, x, y, [232, 178, 178, 255]);
    }
    for (x, y) in disc(186, 142, 13) {
        set(&mut px, x, y, [232, 178, 178, 255]);
    }
    // Smile: lower arc of a ring centred (128, 148), radius ~36, width ~7.
    for y in 0..s {
        for x in 0..s {
            let d = ((x - 128) as f32).powi(2) + ((y - 148) as f32).powi(2);
            let r = d.sqrt();
            if (32.0..=39.0).contains(&r) && y > 148 {
                set(&mut px, x, y, ink);
            }
        }
    }
    px
}

async fn run(width: u32, height: u32, out: &str) {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::VULKAN,
        ..Default::default()
    });
    let adapter = request_adapter(&instance).await;
    let info = adapter.get_info();
    println!(
        "adapter: {} | backend: {:?} | device type: {:?} | driver: {} {}",
        info.name, info.backend, info.device_type, info.driver, info.driver_info
    );

    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: Some("talerender device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_defaults(),
            },
            None,
        )
        .await
        .expect("request_device failed");

    // ---- camera & transforms -------------------------------------------
    let aspect = width as f32 / height as f32;
    let proj = math::perspective(36f32.to_radians(), aspect, 0.1, 100.0);
    // Portrait framing check: at 36 deg vertical FOV and aspect 0.5625,
    // the horizontal half-angle only fits ~0.183 x distance of half-width,
    // so the staging (cube left edge ~ -1.9, card right edge ~ +2.2 in
    // world units) needs ~11 m of camera distance. The first CI frame
    // (eye z 6.8) clipped both subjects — measured, not guessed.
    let eye = [3.4, 2.4, 11.0];
    let view = math::look_at(eye, [0.2, 0.9, 0.0], [0.0, 1.0, 0.0]);
    let vp = math::mul(&proj, &view);

    let cube_model = math::mul(
        &math::translate(-1.0, 0.75, 0.0),
        &math::rotate_y(0.6),
    );
    let char_pos = [1.6, 0.0, 0.2];
    let yaw = (eye[0] - char_pos[0]).atan2(eye[2] - char_pos[2]);
    let char_model = math::mul(&math::translate(char_pos[0], char_pos[1], char_pos[2]), &math::rotate_y(yaw));

    let make_uniform = |model: &Mat4| -> wgpu::Buffer {
        let u = Uniforms {
            mvp: math::mul(&vp, model),
            model: *model,
        };
        device
            .create_buffer(&wgpu::BufferDescriptor {
                label: Some("uniforms"),
                size: 128,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
            .also_write(&queue, &u)
    };

    // ---- colour pipeline ------------------------------------------------
    let shader3d = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("geometry shader"),
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("shader3d.wgsl"))),
    });
    let bgl3d = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("uniform layout"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    });
    let layout3d = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("colour pipeline layout"),
        bind_group_layouts: &[&bgl3d],
        push_constant_ranges: &[],
    });
    let colour_attrs = [
        wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x3, offset: 0, shader_location: 0 },
        wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x3, offset: 12, shader_location: 1 },
        wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x3, offset: 24, shader_location: 2 },
    ];
    let pipeline3d = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("colour pipeline"),
        layout: Some(&layout3d),
        vertex: wgpu::VertexState {
            module: &shader3d,
            entry_point: "vs_main",
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: 36,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &colour_attrs,
            }],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader3d,
            entry_point: "fs_main",
            targets: &[Some(wgpu::ColorTargetState {
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState {
            cull_mode: Some(wgpu::Face::Back),
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Less,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
    });

    // ---- textured pipeline ----------------------------------------------
    let shader_tex = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("textured shader"),
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("shader_tex.wgsl"))),
    });
    let bgl_tex = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("textured layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
    });
    let layout_tex = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("textured pipeline layout"),
        bind_group_layouts: &[&bgl_tex],
        push_constant_ranges: &[],
    });
    let tex_attrs = [
        wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x3, offset: 0, shader_location: 0 },
        wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x2, offset: 12, shader_location: 1 },
    ];
    let pipeline_tex = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("textured pipeline"),
        layout: Some(&layout_tex),
        vertex: wgpu::VertexState {
            module: &shader_tex,
            entry_point: "vs_main",
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: 20,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &tex_attrs,
            }],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader_tex,
            entry_point: "fs_main",
            targets: &[Some(wgpu::ColorTargetState {
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState {
            cull_mode: Some(wgpu::Face::Back),
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Less,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
    });

    // ---- geometry buffers -----------------------------------------------
    let make_vb = |verts: &[f32], idx: &[u32]| -> (wgpu::Buffer, wgpu::Buffer) {
        let vb = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("vertex buffer"),
            size: (verts.len() * 4) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&vb, 0, &pack_f32(verts));
        let ib = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("index buffer"),
            size: (idx.len() * 4) as u64,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&ib, 0, &pack_u32(idx));
        (vb, ib)
    };
    let (cube_verts, cube_idx) = cube(0.75);
    let (cube_vb, cube_ib) = make_vb(&cube_verts, &cube_idx);
    let (gnd_verts, gnd_idx) = ground(9.0, [0.22, 0.38, 0.22]);
    let (gnd_vb, gnd_ib) = make_vb(&gnd_verts, &gnd_idx);

    // Character quad: local XY plane, 1.2 wide x 1.8 tall, base at origin.
    let quad_verts: [f32; 20] = [
        -0.6, 0.0, 0.0, 0.0, 1.0, // BL
        0.6, 0.0, 0.0, 1.0, 1.0, // BR
        0.6, 1.8, 0.0, 1.0, 0.0, // TR
        -0.6, 1.8, 0.0, 0.0, 0.0, // TL
    ];
    let quad_idx: [u32; 6] = [0, 1, 2, 0, 2, 3];
    let (quad_vb, quad_ib) = make_vb(&quad_verts, &quad_idx);

    // ---- character texture ----------------------------------------------
    let tex_size = 256u32;
    let char_tex = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("character texture"),
        size: wgpu::Extent3d { width: tex_size, height: tex_size, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        wgpu::ImageCopyTexture {
            texture: &char_tex,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &character_texture(tex_size),
        wgpu::ImageDataLayout {
            offset: 0,
            bytes_per_row: Some(tex_size * 4),
            rows_per_image: Some(tex_size),
        },
        wgpu::Extent3d { width: tex_size, height: tex_size, depth_or_array_layers: 1 },
    );
    let char_view = char_tex.create_view(&wgpu::TextureViewDescriptor::default());
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("linear sampler"),
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        ..Default::default()
    });

    // ---- bind groups ------------------------------------------------------
    let bind3d = |buf: &wgpu::Buffer| {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("colour bind group"),
            layout: &bgl3d,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buf.as_entire_binding(),
            }],
        })
    };
    let ground_uniform = make_uniform(&math::identity());
    let cube_uniform = make_uniform(&cube_model);
    let char_uniform = make_uniform(&char_model);
    let ground_bg = bind3d(&ground_uniform);
    let cube_bg = bind3d(&cube_uniform);
    let char_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("character bind group"),
        layout: &bgl_tex,
        entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: char_uniform.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&char_view) },
            wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&sampler) },
        ],
    });

    // ---- frame target + depth --------------------------------------------
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("frame target"),
        size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view_tex = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let depth = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("depth target"),
        size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let depth_view = depth.create_view(&wgpu::TextureViewDescriptor::default());

    let padded = math::padded_row_bytes(width);
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("readback buffer"),
        size: (padded * height) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("frame encoder"),
    });
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("scene pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view_tex,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.10, g: 0.16, b: 0.30, a: 1.0 }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        pass.set_pipeline(&pipeline3d);
        pass.set_bind_group(0, &ground_bg, &[]);
        pass.set_vertex_buffer(0, gnd_vb.slice(..));
        pass.set_index_buffer(gnd_ib.slice(..), wgpu::IndexFormat::Uint32);
        pass.draw_indexed(0..gnd_idx.len() as u32, 0, 0..1);
        pass.set_bind_group(0, &cube_bg, &[]);
        pass.set_vertex_buffer(0, cube_vb.slice(..));
        pass.set_index_buffer(cube_ib.slice(..), wgpu::IndexFormat::Uint32);
        pass.draw_indexed(0..cube_idx.len() as u32, 0, 0..1);
        pass.set_pipeline(&pipeline_tex);
        pass.set_bind_group(0, &char_bg, &[]);
        pass.set_vertex_buffer(0, quad_vb.slice(..));
        pass.set_index_buffer(quad_ib.slice(..), wgpu::IndexFormat::Uint32);
        pass.draw_indexed(0..quad_idx.len() as u32, 0, 0..1);
    }
    encoder.copy_texture_to_buffer(
        wgpu::ImageCopyTexture {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::ImageCopyBuffer {
            buffer: &readback,
            layout: wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(padded),
                rows_per_image: Some(height),
            },
        },
        wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
    );
    queue.submit(Some(encoder.finish()));

    let slice = readback.slice(..);
    let (tx, rx) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |result| {
        let _ = tx.send(result);
    });
    device.poll(wgpu::Maintain::Wait);
    rx.recv().expect("map channel closed").expect("buffer map failed");
    let data = slice.get_mapped_range();
    let row = (width * 4) as usize;
    let stride = padded as usize;
    let mut pixels = Vec::with_capacity(row * height as usize);
    for y in 0..height as usize {
        pixels.extend_from_slice(&data[y * stride..y * stride + row]);
    }
    drop(data);
    readback.unmap();

    let img = image::RgbaImage::from_raw(width, height, pixels).expect("pixel buffer size mismatch");
    img.save(out).expect("failed to save PNG");
    println!("wrote {out} ({width}x{height})");
}

// Small helper so `make_uniform` above reads linearly: creates the buffer
// and immediately queues the initial contents.
trait AlsoWrite {
    fn also_write(self, queue: &wgpu::Queue, u: &Uniforms) -> wgpu::Buffer;
}

impl AlsoWrite for wgpu::Buffer {
    fn also_write(self, queue: &wgpu::Queue, u: &Uniforms) -> wgpu::Buffer {
        queue.write_buffer(&self, 0, &uniform_bytes(u));
        self
    }
}
