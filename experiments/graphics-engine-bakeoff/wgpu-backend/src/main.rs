use oteryn_graphics_bakeoff_shared::{
    ATLAS_COLUMNS, ATLAS_ROWS, BenchConfig, BenchSamples, RenderQuad, RenderSnapshot,
    animation_frame, build_snapshot, result_json, synthetic_atlas_rgba8,
};
use std::process::ExitCode;
use std::sync::Arc;
use std::time::{Duration, Instant};
use wgpu::util::DeviceExt;
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowAttributes, WindowId};

const INSTANCE_BYTES: usize = 32;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("wgpu-bakeoff-error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let config = BenchConfig::from_env_args()?;
    let snapshot = build_snapshot(&config);
    let event_loop = EventLoop::new().map_err(|error| format!("event loop: {error}"))?;
    let mut application = Application::new(config, snapshot);
    event_loop
        .run_app(&mut application)
        .map_err(|error| format!("event loop run: {error}"))?;
    application.fatal_error.map_or(Ok(()), Err)
}

struct Application {
    config: BenchConfig,
    snapshot: RenderSnapshot,
    process_start: Instant,
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    fatal_error: Option<String>,
}

impl Application {
    fn new(config: BenchConfig, snapshot: RenderSnapshot) -> Self {
        Self {
            config,
            snapshot,
            process_start: Instant::now(),
            window: None,
            renderer: None,
            fatal_error: None,
        }
    }

    fn fail(&mut self, event_loop: &ActiveEventLoop, error: impl Into<String>) {
        if self.fatal_error.is_none() {
            self.fatal_error = Some(error.into());
        }
        event_loop.exit();
    }
}

impl ApplicationHandler for Application {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.renderer.is_some() {
            return;
        }
        let attributes = WindowAttributes::default()
            .with_title("Oteryn graphics bake-off — custom wgpu")
            .with_inner_size(LogicalSize::new(
                f64::from(self.config.width),
                f64::from(self.config.height),
            ));
        let window = match event_loop.create_window(attributes) {
            Ok(window) => Arc::new(window),
            Err(error) => {
                self.fail(event_loop, format!("window creation: {error}"));
                return;
            }
        };
        match Renderer::new(
            Arc::clone(&window),
            &self.config,
            &self.snapshot,
            self.process_start.elapsed(),
        ) {
            Ok(renderer) => {
                window.request_redraw();
                self.window = Some(window);
                self.renderer = Some(renderer);
            }
            Err(error) => self.fail(event_loop, error),
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(renderer) = &mut self.renderer {
                    renderer.resize(size.width, size.height);
                }
            }
            WindowEvent::RedrawRequested => {
                let Some(renderer) = &mut self.renderer else {
                    return;
                };
                match renderer.render(&self.config, &self.snapshot) {
                    Ok(RenderOutcome::Continue) => {}
                    Ok(RenderOutcome::Complete(result)) => {
                        println!("{result}");
                        event_loop.exit();
                    }
                    Err(error) => self.fail(event_loop, error),
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

enum RenderOutcome {
    Continue,
    Complete(String),
}

struct Renderer {
    surface: wgpu::Surface<'static>,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface_config: wgpu::SurfaceConfiguration,
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    instance_buffer: wgpu::Buffer,
    static_instance_bytes: usize,
    instance_count: u32,
    frame_number: u64,
    samples: BenchSamples,
    startup: Duration,
}

impl Renderer {
    fn new(
        window: Arc<Window>,
        config: &BenchConfig,
        snapshot: &RenderSnapshot,
        startup_before_gpu: Duration,
    ) -> Result<Self, String> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::DX12,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let surface = instance
            .create_surface(window)
            .map_err(|error| format!("surface creation: {error}"))?;
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            compatible_surface: Some(&surface),
            ..wgpu::RequestAdapterOptions::default()
        }))
        .map_err(|error| format!("adapter request: {error}"))?;
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("oteryn-graphics-bakeoff-device"),
            ..wgpu::DeviceDescriptor::default()
        }))
        .map_err(|error| format!("device request: {error}"))?;

        let mut surface_config = surface
            .get_default_config(&adapter, config.width, config.height)
            .ok_or_else(|| "surface has no default configuration".to_owned())?;
        surface_config.present_mode = wgpu::PresentMode::AutoNoVsync;
        surface.configure(&device, &surface_config);

        let initial_bytes = encode_all_instances(snapshot, config, 0);
        let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("oteryn-bakeoff-instances"),
            contents: &initial_bytes,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        let atlas_width = config.sprite_px * ATLAS_COLUMNS;
        let atlas_height = config.sprite_px * ATLAS_ROWS;
        let atlas_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("oteryn-bakeoff-atlas"),
            size: wgpu::Extent3d {
                width: atlas_width,
                height: atlas_height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let atlas_bytes = synthetic_atlas_rgba8(config.sprite_px);
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &atlas_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &atlas_bytes,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(atlas_width * 4),
                rows_per_image: Some(atlas_height),
            },
            wgpu::Extent3d {
                width: atlas_width,
                height: atlas_height,
                depth_or_array_layers: 1,
            },
        );
        let atlas_view = atlas_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("oteryn-bakeoff-nearest"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..wgpu::SamplerDescriptor::default()
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("oteryn-bakeoff-bind-layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
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
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("oteryn-bakeoff-bind-group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: instance_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&atlas_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("oteryn-bakeoff-sprite-shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("sprite.wgsl").into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("oteryn-bakeoff-pipeline-layout"),
            bind_group_layouts: &[&bind_group_layout],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("oteryn-bakeoff-pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vertex_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fragment_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });

        let total_instances = snapshot.static_quads.len() + snapshot.animated_quads.len();
        let instance_count = u32::try_from(total_instances)
            .map_err(|_| "instance count exceeds u32".to_owned())?;
        Ok(Self {
            surface,
            adapter,
            device,
            queue,
            surface_config,
            pipeline,
            bind_group,
            instance_buffer,
            static_instance_bytes: snapshot.static_quads.len() * INSTANCE_BYTES,
            instance_count,
            frame_number: 0,
            samples: BenchSamples::default(),
            startup: startup_before_gpu.elapsed() + startup_before_gpu,
        })
    }

    fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.surface_config.width = width;
        self.surface_config.height = height;
        self.surface.configure(&self.device, &self.surface_config);
    }

    fn render(
        &mut self,
        config: &BenchConfig,
        snapshot: &RenderSnapshot,
    ) -> Result<RenderOutcome, String> {
        let frame_start = Instant::now();
        let prep_start = Instant::now();
        let animated_bytes = encode_animated_instances(snapshot, config, self.frame_number);
        let prep = prep_start.elapsed();
        self.queue.write_buffer(
            &self.instance_buffer,
            self.static_instance_bytes as u64,
            &animated_bytes,
        );

        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame)
            | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                return Ok(RenderOutcome::Continue);
            }
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                self.surface.configure(&self.device, &self.surface_config);
                return Ok(RenderOutcome::Continue);
            }
            wgpu::CurrentSurfaceTexture::Validation => {
                return Err("surface validation failure".to_owned());
            }
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("oteryn-bakeoff-encoder"),
            });
        {
            let color_attachments = [Some(wgpu::RenderPassColorAttachment {
                view: &view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })];
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("oteryn-bakeoff-pass"),
                color_attachments: &color_attachments,
                ..wgpu::RenderPassDescriptor::default()
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.bind_group, &[]);
            pass.draw(0..6, 0..self.instance_count);
        }
        let _submission = self.queue.submit([encoder.finish()]);
        self.queue.present(frame);

        let elapsed = frame_start.elapsed();
        if self.frame_number >= config.warmup_frames {
            self.samples.record(elapsed, prep);
        }
        self.frame_number += 1;
        if self.samples.len() >= config.sample_frames as usize {
            let info = self.adapter.get_info();
            let adapter = format!("{}|{:?}|{}", info.name, info.backend, info.driver);
            return Ok(RenderOutcome::Complete(result_json(
                "custom-wgpu-30.0.0",
                config,
                &self.samples,
                self.startup,
                &adapter,
                Some(1),
            )));
        }
        Ok(RenderOutcome::Continue)
    }
}

fn encode_all_instances(
    snapshot: &RenderSnapshot,
    config: &BenchConfig,
    frame_number: u64,
) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(
        (snapshot.static_quads.len() + snapshot.animated_quads.len()) * INSTANCE_BYTES,
    );
    for quad in &snapshot.static_quads {
        encode_instance(&mut bytes, *quad, quad.frame, config);
    }
    for quad in &snapshot.animated_quads {
        encode_instance(
            &mut bytes,
            *quad,
            animation_frame(quad.phase, frame_number),
            config,
        );
    }
    bytes
}

fn encode_animated_instances(
    snapshot: &RenderSnapshot,
    config: &BenchConfig,
    frame_number: u64,
) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(snapshot.animated_quads.len() * INSTANCE_BYTES);
    for quad in &snapshot.animated_quads {
        encode_instance(
            &mut bytes,
            *quad,
            animation_frame(quad.phase, frame_number),
            config,
        );
    }
    bytes
}

fn encode_instance(bytes: &mut Vec<u8>, quad: RenderQuad, frame: u32, config: &BenchConfig) {
    let position_x = quad.x * 2.0 / config.width as f32;
    let position_y = quad.y * 2.0 / config.height as f32;
    let size_x = quad.logical_size * 2.0 / config.width as f32;
    let size_y = quad.logical_size * 2.0 / config.height as f32;
    let frame_x = frame % ATLAS_COLUMNS;
    let frame_y = frame / ATLAS_COLUMNS;
    let uv_width = 1.0 / ATLAS_COLUMNS as f32;
    let uv_height = 1.0 / ATLAS_ROWS as f32;
    for value in [
        position_x,
        position_y,
        size_x,
        size_y,
        frame_x as f32 * uv_width,
        frame_y as f32 * uv_height,
        uv_width,
        uv_height,
    ] {
        bytes.extend_from_slice(&value.to_ne_bytes());
    }
}
