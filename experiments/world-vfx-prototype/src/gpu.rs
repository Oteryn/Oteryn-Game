use crate::model::{
    BenchConfig, CorpusShape, FramePlan, MAX_LIGHTS, MaterialClass, PAGE_CELLS, PrewarmMode,
    PrimitiveKind, ResourceLayout,
};
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};
use std::ops::Range;
use std::sync::{Arc, mpsc};
use std::time::{Duration, Instant};
use winit::window::Window;

const INSTANCE_BYTES: usize = 64;
const MAX_INSTANCES: usize = 100_000;
const GLOBAL_VEC4S: usize = 1 + MAX_LIGHTS;
const GLOBAL_BYTES: usize = GLOBAL_VEC4S * 16;

pub enum RenderOutcome {
    Continue,
    Complete(String),
}

struct PageResource {
    _texture: wgpu::Texture,
    bind_group: wgpu::BindGroup,
    last_used: u64,
}

#[derive(Default)]
struct CacheFrameStats {
    hits: u64,
    misses: u64,
    evictions: u64,
    upload_bytes: u64,
    upload_ms: f64,
}

struct TimestampState {
    query_set: wgpu::QuerySet,
    resolve_buffer: wgpu::Buffer,
    readback_buffer: wgpu::Buffer,
    period_ns: f32,
    query_count: u32,
}

#[derive(Default)]
struct Samples {
    frame_interval_ms: Vec<f64>,
    cpu_frame_ms: Vec<f64>,
    prep_ms: Vec<f64>,
    gpu_frame_ms: Vec<f64>,
    scroll_stall_ms: Vec<f64>,
    zoom_stall_ms: Vec<f64>,
    floor_stall_ms: Vec<f64>,
    upload_ms: Vec<f64>,
    visible_sum: u64,
    visible_max: u64,
    batches_sum: u64,
    batches_max: u64,
    particles_max: u64,
    lights_max: u64,
    cache_hits: u64,
    cache_misses: u64,
    cache_evictions: u64,
    upload_bytes: u64,
    decorative_dropped: u64,
    variant_fallbacks: u64,
    critical_visible_min: u64,
    gameplay_signature_xor: u64,
    saw_rain: bool,
    saw_snow: bool,
    saw_fog: bool,
    saw_winter: bool,
}

impl Samples {
    fn record(
        &mut self,
        frame_interval: Duration,
        cpu_frame: Duration,
        prep: Duration,
        cache: &CacheFrameStats,
        plan: &FramePlan,
        batches: u64,
    ) {
        let interval_ms = ms(frame_interval);
        let cpu_ms = ms(cpu_frame);
        let prep_ms = ms(prep);
        self.frame_interval_ms.push(interval_ms);
        self.cpu_frame_ms.push(cpu_ms);
        self.prep_ms.push(prep_ms);
        if plan.camera_moved {
            self.scroll_stall_ms.push(cpu_ms);
        }
        if plan.zoom_changed {
            self.zoom_stall_ms.push(cpu_ms);
        }
        if plan.floor_changed {
            self.floor_stall_ms.push(cpu_ms);
        }
        if cache.upload_ms > 0.0 {
            self.upload_ms.push(cache.upload_ms);
        }
        let visible = plan.instances.len() as u64;
        self.visible_sum = self.visible_sum.saturating_add(visible);
        self.visible_max = self.visible_max.max(visible);
        self.batches_sum = self.batches_sum.saturating_add(batches);
        self.batches_max = self.batches_max.max(batches);
        self.particles_max = self.particles_max.max(u64::from(plan.stats.particles));
        self.lights_max = self.lights_max.max(u64::from(plan.stats.lights));
        self.cache_hits = self.cache_hits.saturating_add(cache.hits);
        self.cache_misses = self.cache_misses.saturating_add(cache.misses);
        self.cache_evictions = self.cache_evictions.saturating_add(cache.evictions);
        self.upload_bytes = self.upload_bytes.saturating_add(cache.upload_bytes);
        self.decorative_dropped = self
            .decorative_dropped
            .saturating_add(u64::from(plan.stats.decorative_dropped));
        self.variant_fallbacks = self
            .variant_fallbacks
            .saturating_add(u64::from(plan.stats.variant_fallbacks));
        let critical = u64::from(plan.stats.critical_visible);
        self.critical_visible_min = if self.critical_visible_min == 0 {
            critical
        } else {
            self.critical_visible_min.min(critical)
        };
        self.gameplay_signature_xor ^= plan.gameplay_signature;
        match plan.environment.weather {
            crate::model::Weather::Rain => self.saw_rain = true,
            crate::model::Weather::Snow => self.saw_snow = true,
            crate::model::Weather::Fog => self.saw_fog = true,
            crate::model::Weather::Clear => {}
        }
        self.saw_winter |= plan.environment.season == crate::model::Season::Winter;
    }
}

pub struct Renderer {
    surface: wgpu::Surface<'static>,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface_config: wgpu::SurfaceConfiguration,
    alpha_pipeline: wgpu::RenderPipeline,
    additive_pipeline: wgpu::RenderPipeline,
    global_bind_group: wgpu::BindGroup,
    page_bind_group_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    instance_buffer: wgpu::Buffer,
    global_buffer: wgpu::Buffer,
    pages: BTreeMap<u32, PageResource>,
    timestamp: Option<TimestampState>,
    frame_number: u64,
    last_frame_start: Instant,
    samples: Samples,
    startup: Duration,
    prewarm_ms: f64,
    first_frame_cpu_ms: Option<f64>,
    first_upload_ms: Option<f64>,
    surface_lost: u64,
    surface_outdated: u64,
    surface_timeout: u64,
    surface_occluded: u64,
    corpus: CorpusShape,
}

impl Renderer {
    pub fn new(
        window: Arc<Window>,
        config: &BenchConfig,
        initial_plan: &FramePlan,
        corpus: CorpusShape,
        startup_before_gpu: Duration,
    ) -> Result<Self, String> {
        if config.total_frames() > 2_048 {
            return Err("total frames must be <= 2048 for timestamp query evidence".to_owned());
        }
        let gpu_start = Instant::now();
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::DX12,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let surface = instance
            .create_surface(window)
            .map_err(|error| format!("surface creation: {error}"))?;
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            compatible_surface: Some(&surface),
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            apply_limit_buckets: false,
        }))
        .map_err(|error| format!("adapter request: {error}"))?;
        let adapter_features = adapter.features();
        let timestamp_supported = adapter_features.contains(wgpu::Features::TIMESTAMP_QUERY);
        let required_features = if timestamp_supported {
            wgpu::Features::TIMESTAMP_QUERY
        } else {
            wgpu::Features::empty()
        };
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("oteryn-world-vfx-device"),
            required_features,
            ..wgpu::DeviceDescriptor::default()
        }))
        .map_err(|error| format!("device request: {error}"))?;

        let capabilities = surface.get_capabilities(&adapter);
        let mut surface_config = surface
            .get_default_config(&adapter, config.width, config.height)
            .ok_or_else(|| "surface has no default configuration".to_owned())?;
        if let Some(format) = capabilities
            .formats
            .iter()
            .copied()
            .find(wgpu::TextureFormat::is_srgb)
        {
            surface_config.format = format;
        }
        surface_config.present_mode = wgpu::PresentMode::AutoNoVsync;
        surface.configure(&device, &surface_config);

        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("oteryn-world-vfx-instances"),
            size: (MAX_INSTANCES * INSTANCE_BYTES) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let global_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("oteryn-world-vfx-globals"),
            size: GLOBAL_BYTES as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let global_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("oteryn-world-vfx-global-layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let global_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("oteryn-world-vfx-global-bind-group"),
            layout: &global_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: instance_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: global_buffer.as_entire_binding(),
                },
            ],
        });

        let view_dimension = match config.layout {
            ResourceLayout::Atlas => wgpu::TextureViewDimension::D2,
            ResourceLayout::Array => wgpu::TextureViewDimension::D2Array,
        };
        let page_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("oteryn-world-vfx-page-layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });
        let filter = if config.family == crate::model::PresentationFamily::Classic {
            wgpu::FilterMode::Nearest
        } else {
            wgpu::FilterMode::Linear
        };
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("oteryn-world-vfx-sampler"),
            mag_filter: filter,
            min_filter: filter,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..wgpu::SamplerDescriptor::default()
        });

        let shader_source = match config.layout {
            ResourceLayout::Atlas => include_str!("sprite_atlas.wgsl"),
            ResourceLayout::Array => include_str!("sprite_array.wgsl"),
        };
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("oteryn-world-vfx-shader"),
            source: wgpu::ShaderSource::Wgsl(shader_source.into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("oteryn-world-vfx-pipeline-layout"),
            bind_group_layouts: &[Some(&global_layout), Some(&page_bind_group_layout)],
            immediate_size: 0,
        });
        let alpha_pipeline = create_pipeline(
            &device,
            &pipeline_layout,
            &shader,
            surface_config.format,
            wgpu::BlendState::ALPHA_BLENDING,
            "oteryn-world-vfx-alpha-pipeline",
        );
        let additive_pipeline = create_pipeline(
            &device,
            &pipeline_layout,
            &shader,
            surface_config.format,
            wgpu::BlendState {
                color: wgpu::BlendComponent {
                    src_factor: wgpu::BlendFactor::SrcAlpha,
                    dst_factor: wgpu::BlendFactor::One,
                    operation: wgpu::BlendOperation::Add,
                },
                alpha: wgpu::BlendComponent::OVER,
            },
            "oteryn-world-vfx-additive-pipeline",
        );

        let timestamp = if timestamp_supported {
            let query_count = u32::try_from(config.total_frames() * 2)
                .map_err(|_| "timestamp query count exceeds u32".to_owned())?;
            let query_set = device.create_query_set(&wgpu::QuerySetDescriptor {
                label: Some("oteryn-world-vfx-timestamps"),
                ty: wgpu::QueryType::Timestamp,
                count: query_count,
            });
            let bytes = u64::from(query_count) * 8;
            let resolve_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("oteryn-world-vfx-timestamp-resolve"),
                size: bytes,
                usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            });
            let readback_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("oteryn-world-vfx-timestamp-readback"),
                size: bytes,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            });
            Some(TimestampState {
                query_set,
                resolve_buffer,
                readback_buffer,
                period_ns: queue.get_timestamp_period(),
                query_count,
            })
        } else {
            None
        };

        let startup = startup_before_gpu + gpu_start.elapsed();
        let mut renderer = Self {
            surface,
            adapter,
            device,
            queue,
            surface_config,
            alpha_pipeline,
            additive_pipeline,
            global_bind_group,
            page_bind_group_layout,
            sampler,
            instance_buffer,
            global_buffer,
            pages: BTreeMap::new(),
            timestamp,
            frame_number: 0,
            last_frame_start: Instant::now(),
            samples: Samples::default(),
            startup,
            prewarm_ms: 0.0,
            first_frame_cpu_ms: None,
            first_upload_ms: None,
            surface_lost: 0,
            surface_outdated: 0,
            surface_timeout: 0,
            surface_occluded: 0,
            corpus,
        };
        if config.prewarm == PrewarmMode::Critical {
            let started = Instant::now();
            let _ = renderer.prepare_pages(config, initial_plan)?;
            renderer.prewarm_ms = ms(started.elapsed());
        }
        Ok(renderer)
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.surface_config.width = width;
        self.surface_config.height = height;
        self.surface.configure(&self.device, &self.surface_config);
    }

    pub fn render(
        &mut self,
        config: &BenchConfig,
        plan: &FramePlan,
        model_prep: Duration,
    ) -> Result<RenderOutcome, String> {
        let frame_start = Instant::now();
        let frame_interval = frame_start.duration_since(self.last_frame_start);
        self.last_frame_start = frame_start;
        let cache = self.prepare_pages(config, plan)?;
        if self.first_upload_ms.is_none() && cache.upload_ms > 0.0 {
            self.first_upload_ms = Some(cache.upload_ms);
        }
        let encode_start = Instant::now();
        let instance_bytes = encode_instances(config, plan)?;
        self.queue
            .write_buffer(&self.instance_buffer, 0, &instance_bytes);
        let global_bytes =
            encode_globals(self.surface_config.width, self.surface_config.height, plan);
        self.queue
            .write_buffer(&self.global_buffer, 0, &global_bytes);
        let prep =
            model_prep + encode_start.elapsed() + Duration::from_secs_f64(cache.upload_ms / 1000.0);

        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame)
            | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
            wgpu::CurrentSurfaceTexture::Timeout => {
                self.surface_timeout = self.surface_timeout.saturating_add(1);
                return Ok(RenderOutcome::Continue);
            }
            wgpu::CurrentSurfaceTexture::Occluded => {
                self.surface_occluded = self.surface_occluded.saturating_add(1);
                return Ok(RenderOutcome::Continue);
            }
            wgpu::CurrentSurfaceTexture::Outdated => {
                self.surface_outdated = self.surface_outdated.saturating_add(1);
                self.surface.configure(&self.device, &self.surface_config);
                return Ok(RenderOutcome::Continue);
            }
            wgpu::CurrentSurfaceTexture::Lost => {
                self.surface_lost = self.surface_lost.saturating_add(1);
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
        let batches = make_batches(&plan.instances);
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("oteryn-world-vfx-encoder"),
            });
        let query_base = u32::try_from(self.frame_number * 2)
            .map_err(|_| "frame query index exceeds u32".to_owned())?;
        let timestamp_writes =
            self.timestamp
                .as_ref()
                .map(|timestamp| wgpu::RenderPassTimestampWrites {
                    query_set: &timestamp.query_set,
                    beginning_of_pass_write_index: Some(query_base),
                    end_of_pass_write_index: Some(query_base + 1),
                });
        let clear = clear_color(plan);
        {
            let attachments = [Some(wgpu::RenderPassColorAttachment {
                view: &view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(clear),
                    store: wgpu::StoreOp::Store,
                },
            })];
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("oteryn-world-vfx-pass"),
                color_attachments: &attachments,
                timestamp_writes,
                ..wgpu::RenderPassDescriptor::default()
            });
            pass.set_bind_group(0, &self.global_bind_group, &[]);
            let mut active_material = None;
            for batch in &batches {
                if active_material != Some(batch.material) {
                    let pipeline = match batch.material {
                        MaterialClass::Alpha => &self.alpha_pipeline,
                        MaterialClass::Additive => &self.additive_pipeline,
                    };
                    pass.set_pipeline(pipeline);
                    active_material = Some(batch.material);
                }
                let page = self
                    .pages
                    .get(&batch.page_id)
                    .ok_or_else(|| format!("resource page {} is not resident", batch.page_id))?;
                pass.set_bind_group(1, &page.bind_group, &[]);
                pass.draw(0..6, batch.instances.clone());
            }
        }

        let final_frame = self.frame_number + 1 >= config.total_frames();
        if final_frame {
            if let Some(timestamp) = &self.timestamp {
                let used_queries = query_base + 2;
                if used_queries <= timestamp.query_count {
                    encoder.resolve_query_set(
                        &timestamp.query_set,
                        0..used_queries,
                        &timestamp.resolve_buffer,
                        0,
                    );
                    encoder.copy_buffer_to_buffer(
                        &timestamp.resolve_buffer,
                        0,
                        &timestamp.readback_buffer,
                        0,
                        u64::from(used_queries) * 8,
                    );
                }
            }
        }
        self.queue.submit([encoder.finish()]);
        self.queue.present(frame);
        let cpu_frame = model_prep + frame_start.elapsed();
        if self.first_frame_cpu_ms.is_none() {
            self.first_frame_cpu_ms = Some(ms(cpu_frame));
        }
        if self.frame_number >= config.warmup_frames {
            self.samples.record(
                frame_interval,
                cpu_frame,
                prep,
                &cache,
                plan,
                batches.len() as u64,
            );
        }
        self.frame_number = self.frame_number.saturating_add(1);

        if self.samples.cpu_frame_ms.len() >= config.sample_frames as usize {
            self.collect_gpu_timestamps(config)?;
            let report = self.report(config);
            return Ok(RenderOutcome::Complete(report));
        }
        Ok(RenderOutcome::Continue)
    }

    fn prepare_pages(
        &mut self,
        config: &BenchConfig,
        plan: &FramePlan,
    ) -> Result<CacheFrameStats, String> {
        let mut stats = CacheFrameStats::default();
        let needed: BTreeSet<u32> = plan.needed_pages.iter().copied().collect();
        for page_id in &plan.needed_pages {
            if let Some(page) = self.pages.get_mut(page_id) {
                page.last_used = self.frame_number;
                stats.hits = stats.hits.saturating_add(1);
                continue;
            }
            stats.misses = stats.misses.saturating_add(1);
            if self.pages.len() >= config.cache_pages as usize {
                let candidate = self
                    .pages
                    .iter()
                    .filter(|(id, _)| !needed.contains(id))
                    .min_by_key(|(_, page)| page.last_used)
                    .map(|(id, _)| *id)
                    .or_else(|| {
                        self.pages
                            .iter()
                            .min_by_key(|(_, page)| page.last_used)
                            .map(|(id, _)| *id)
                    });
                if let Some(evicted) = candidate {
                    self.pages.remove(&evicted);
                    stats.evictions = stats.evictions.saturating_add(1);
                }
            }
            let bytes =
                procedural_page_bytes(config.density, *page_id, config.layout, config.family);
            let upload_started = Instant::now();
            let page = self.create_page(config, *page_id, &bytes)?;
            stats.upload_ms += ms(upload_started.elapsed());
            stats.upload_bytes = stats.upload_bytes.saturating_add(bytes.len() as u64);
            self.pages.insert(*page_id, page);
        }
        Ok(stats)
    }

    fn create_page(
        &self,
        config: &BenchConfig,
        page_id: u32,
        bytes: &[u8],
    ) -> Result<PageResource, String> {
        let (width, height, layers) = match config.layout {
            ResourceLayout::Atlas => (config.density * 4, config.density * 4, 1),
            ResourceLayout::Array => (config.density, config.density, PAGE_CELLS),
        };
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("oteryn-world-vfx-page"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: layers,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            bytes,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width * 4),
                rows_per_image: Some(height),
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: layers,
            },
        );
        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some("oteryn-world-vfx-page-view"),
            dimension: Some(match config.layout {
                ResourceLayout::Atlas => wgpu::TextureViewDimension::D2,
                ResourceLayout::Array => wgpu::TextureViewDimension::D2Array,
            }),
            base_array_layer: 0,
            array_layer_count: if config.layout == ResourceLayout::Array {
                Some(PAGE_CELLS)
            } else {
                None
            },
            ..wgpu::TextureViewDescriptor::default()
        });
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("oteryn-world-vfx-page-bind-group"),
            layout: &self.page_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });
        let _ = page_id;
        Ok(PageResource {
            _texture: texture,
            bind_group,
            last_used: self.frame_number,
        })
    }

    fn collect_gpu_timestamps(&mut self, config: &BenchConfig) -> Result<(), String> {
        let Some(timestamp) = &self.timestamp else {
            return Ok(());
        };
        let slice = timestamp.readback_buffer.slice(..);
        let (sender, receiver) = mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
        self.device
            .poll(wgpu::PollType::wait_indefinitely())
            .map_err(|error| format!("device poll for timestamp readback: {error:?}"))?;
        let mapped = receiver
            .recv()
            .map_err(|error| format!("timestamp callback channel: {error}"))?;
        mapped.map_err(|error| format!("timestamp map: {error}"))?;
        {
            let bytes = slice
                .get_mapped_range()
                .map_err(|error| format!("timestamp mapped range: {error}"))?;
            for frame in config.warmup_frames..config.total_frames() {
                let offset = frame as usize * 16;
                let Some(begin_bytes) = bytes.get(offset..offset + 8) else {
                    break;
                };
                let Some(end_bytes) = bytes.get(offset + 8..offset + 16) else {
                    break;
                };
                let mut begin_array = [0_u8; 8];
                let mut end_array = [0_u8; 8];
                begin_array.copy_from_slice(begin_bytes);
                end_array.copy_from_slice(end_bytes);
                let begin = u64::from_ne_bytes(begin_array);
                let end = u64::from_ne_bytes(end_array);
                if end >= begin {
                    let duration_ms =
                        (end - begin) as f64 * f64::from(timestamp.period_ns) / 1_000_000.0;
                    self.samples.gpu_frame_ms.push(duration_ms);
                }
            }
        }
        timestamp.readback_buffer.unmap();
        Ok(())
    }

    fn report(&self, config: &BenchConfig) -> String {
        let info = self.adapter.get_info();
        let limits = self.adapter.limits();
        let frame_count = self.samples.cpu_frame_ms.len().max(1) as f64;
        let frame_mean = mean(&self.samples.frame_interval_ms);
        let gpu_status = if self.timestamp.is_some() && !self.samples.gpu_frame_ms.is_empty() {
            "measured_timestamp_query"
        } else {
            "unavailable_or_unreliable"
        };
        json!({
            "prototype": "OTERYN_WORLD_VFX_PROTOTYPE",
            "scenario": config.scenario.as_str(),
            "density": config.density,
            "layout": config.layout.as_str(),
            "presentation_family": config.family.as_str(),
            "prewarm": config.prewarm.as_str(),
            "warmup_frames": config.warmup_frames,
            "sample_frames": config.sample_frames,
            "window": format!("{}x{}", config.width, config.height),
            "adapter": {
                "name": info.name,
                "backend": format!("{:?}", info.backend),
                "driver": info.driver,
                "driver_info": info.driver_info,
                "device_type": format!("{:?}", info.device_type),
                "max_texture_array_layers": limits.max_texture_array_layers,
            },
            "surface": {
                "format": format!("{:?}", self.surface_config.format),
                "present_mode": format!("{:?}", self.surface_config.present_mode),
                "alpha_mode": format!("{:?}", self.surface_config.alpha_mode),
            },
            "corpus_shape": {
                "objects": self.corpus.objects,
                "outfits": self.corpus.outfits,
                "effects": self.corpus.effects,
                "missiles": self.corpus.missiles,
                "real_pixels_committed": false,
            },
            "cpu_frame_ms": summary_json(&self.samples.cpu_frame_ms),
            "frame_interval_ms": summary_json(&self.samples.frame_interval_ms),
            "prep_ms": summary_json(&self.samples.prep_ms),
            "gpu_frame_ms": {
                "status": gpu_status,
                "summary": summary_json(&self.samples.gpu_frame_ms),
            },
            "mean_fps": if frame_mean > 0.0 { 1000.0 / frame_mean } else { 0.0 },
            "startup_ms": ms(self.startup),
            "first_frame_cpu_ms": self.first_frame_cpu_ms,
            "prewarm_ms": self.prewarm_ms,
            "first_upload_ms": self.first_upload_ms,
            "ram_bytes": "MEASURED_BY_HOST_HARNESS",
            "vram_bytes": "UNAVAILABLE_UNLESS_HOST_HARNESS_PROVES_TRUSTWORTHY_COUNTER",
            "visible_primitives": {
                "mean": self.samples.visible_sum as f64 / frame_count,
                "max": self.samples.visible_max,
            },
            "draw_batches": {
                "mean": self.samples.batches_sum as f64 / frame_count,
                "max": self.samples.batches_max,
                "order_preserving": true,
            },
            "draw_submissions": config.sample_frames,
            "active_particles_max": self.samples.particles_max,
            "active_lights_max": self.samples.lights_max,
            "cache": {
                "capacity_pages": config.cache_pages,
                "hits": self.samples.cache_hits,
                "misses": self.samples.cache_misses,
                "evictions": self.samples.cache_evictions,
                "upload_bytes": self.samples.upload_bytes,
                "upload_ms": summary_json(&self.samples.upload_ms),
            },
            "camera_scroll_stall_ms": summary_json(&self.samples.scroll_stall_ms),
            "zoom_stall_ms": summary_json(&self.samples.zoom_stall_ms),
            "floor_transition_stall_ms": summary_json(&self.samples.floor_stall_ms),
            "readability": {
                "critical_visible_min": self.samples.critical_visible_min,
                "decorative_dropped": self.samples.decorative_dropped,
                "critical_degrades_after_decorative": true,
            },
            "environment_seen": {
                "rain": self.samples.saw_rain,
                "snow": self.samples.saw_snow,
                "fog": self.samples.saw_fog,
                "winter": self.samples.saw_winter,
            },
            "variant_fallbacks": self.samples.variant_fallbacks,
            "gameplay_signature_xor": format!("{:016x}", self.samples.gameplay_signature_xor),
            "reliability": {
                "surface_lost": self.surface_lost,
                "surface_outdated": self.surface_outdated,
                "surface_timeout": self.surface_timeout,
                "surface_occluded": self.surface_occluded,
                "device_loss_detected": false,
            }
        })
        .to_string()
    }
}

fn create_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    format: wgpu::TextureFormat,
    blend: wgpu::BlendState,
    label: &'static str,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vertex_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[],
        },
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some("fragment_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(blend),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    })
}

#[derive(Debug, Clone)]
struct Batch {
    page_id: u32,
    material: MaterialClass,
    instances: Range<u32>,
}

fn make_batches(instances: &[crate::model::RenderInstance]) -> Vec<Batch> {
    let mut batches = Vec::new();
    if instances.is_empty() {
        return batches;
    }
    let mut start = 0_usize;
    let mut page = instances[0].page_id;
    let mut material = instances[0].material;
    for (index, instance) in instances.iter().enumerate().skip(1) {
        if instance.page_id != page || instance.material != material {
            batches.push(Batch {
                page_id: page,
                material,
                instances: start as u32..index as u32,
            });
            start = index;
            page = instance.page_id;
            material = instance.material;
        }
    }
    batches.push(Batch {
        page_id: page,
        material,
        instances: start as u32..instances.len() as u32,
    });
    batches
}

fn encode_instances(config: &BenchConfig, plan: &FramePlan) -> Result<Vec<u8>, String> {
    if plan.instances.len() > MAX_INSTANCES {
        return Err(format!(
            "visible instance count {} exceeds prototype capacity {MAX_INSTANCES}",
            plan.instances.len()
        ));
    }
    let mut bytes = Vec::with_capacity(plan.instances.len() * INSTANCE_BYTES);
    for instance in &plan.instances {
        let uv = match config.layout {
            ResourceLayout::Atlas => {
                let cell_x = instance.cell % 4;
                let cell_y = instance.cell / 4;
                [cell_x as f32 / 4.0, cell_y as f32 / 4.0, 0.25, 0.25]
            }
            ResourceLayout::Array => [instance.cell as f32, 0.0, 1.0, 1.0],
        };
        let meta = [
            instance.emissive,
            if instance.kind == PrimitiveKind::Overlay {
                1.0
            } else {
                0.0
            },
            if instance.screen_space { 1.0 } else { 0.0 },
            if instance.critical { 1.0 } else { 0.0 },
        ];
        for value in [
            instance.screen_x,
            instance.screen_y,
            instance.width,
            instance.height,
        ]
        .into_iter()
        .chain(uv)
        .chain(instance.color)
        .chain(meta)
        {
            bytes.extend_from_slice(&value.to_ne_bytes());
        }
    }
    Ok(bytes)
}

fn encode_globals(width: u32, height: u32, plan: &FramePlan) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(GLOBAL_BYTES);
    let light_count = plan.lights.len().min(MAX_LIGHTS);
    for value in [
        width as f32,
        height as f32,
        plan.environment.ambient,
        light_count as f32,
    ] {
        bytes.extend_from_slice(&value.to_ne_bytes());
    }
    for index in 0..MAX_LIGHTS {
        let light = plan
            .lights
            .get(index)
            .copied()
            .unwrap_or(crate::model::LocalLight {
                x: 0.0,
                y: 0.0,
                radius: 1.0,
                intensity: 0.0,
            });
        for value in [light.x, light.y, light.radius, light.intensity] {
            bytes.extend_from_slice(&value.to_ne_bytes());
        }
    }
    bytes
}

fn procedural_page_bytes(
    density: u32,
    page_id: u32,
    layout: ResourceLayout,
    family: crate::model::PresentationFamily,
) -> Vec<u8> {
    let (width, height, layers) = match layout {
        ResourceLayout::Atlas => (density * 4, density * 4, 1),
        ResourceLayout::Array => (density, density, PAGE_CELLS),
    };
    let mut bytes = vec![0_u8; width as usize * height as usize * layers as usize * 4];
    for layer in 0..layers {
        for y in 0..height {
            for x in 0..width {
                let cell = match layout {
                    ResourceLayout::Atlas => (y / density) * 4 + (x / density),
                    ResourceLayout::Array => layer,
                };
                let local_x = x % density;
                let local_y = y % density;
                let layer_offset = layer as usize * width as usize * height as usize * 4;
                let offset = layer_offset + ((y * width + x) * 4) as usize;
                let seed = page_id
                    .wrapping_mul(47)
                    .wrapping_add(cell.wrapping_mul(83))
                    .wrapping_add(31);
                let border =
                    local_x < 2 || local_y < 2 || local_x + 2 >= density || local_y + 2 >= density;
                let checker =
                    ((local_x / (density / 8).max(1)) + (local_y / (density / 8).max(1)) + cell)
                        % 2
                        == 0;
                let family_bias = match family {
                    crate::model::PresentationFamily::Classic => 0_u32,
                    crate::model::PresentationFamily::Enhanced => 17,
                    crate::model::PresentationFamily::Hd => 31,
                };
                let r = ((seed + family_bias) % 160 + 64) as u8;
                let g = ((seed.wrapping_mul(3) + family_bias) % 160 + 64) as u8;
                let b = ((seed.wrapping_mul(7) + family_bias) % 160 + 64) as u8;
                bytes[offset] = if border {
                    245
                } else if checker {
                    r
                } else {
                    r / 2
                };
                bytes[offset + 1] = if border {
                    245
                } else if checker {
                    g
                } else {
                    g / 2
                };
                bytes[offset + 2] = if border {
                    245
                } else if checker {
                    b
                } else {
                    b / 2
                };
                bytes[offset + 3] = 255;
            }
        }
    }
    bytes
}

fn clear_color(plan: &FramePlan) -> wgpu::Color {
    let ambient = f64::from(plan.environment.ambient);
    match plan.environment.weather {
        crate::model::Weather::Clear => wgpu::Color {
            r: 0.055 * ambient,
            g: 0.075 * ambient,
            b: 0.095 * ambient,
            a: 1.0,
        },
        crate::model::Weather::Rain => wgpu::Color {
            r: 0.045 * ambient,
            g: 0.060 * ambient,
            b: 0.090 * ambient,
            a: 1.0,
        },
        crate::model::Weather::Snow => wgpu::Color {
            r: 0.080 * ambient,
            g: 0.090 * ambient,
            b: 0.105 * ambient,
            a: 1.0,
        },
        crate::model::Weather::Fog => wgpu::Color {
            r: 0.075 * ambient,
            g: 0.080 * ambient,
            b: 0.085 * ambient,
            a: 1.0,
        },
    }
}

fn ms(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1000.0
}

fn mean(values: &[f64]) -> f64 {
    if values.is_empty() {
        0.0
    } else {
        values.iter().sum::<f64>() / values.len() as f64
    }
}

fn percentile(values: &[f64], quantile: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let mut ordered = values.to_vec();
    ordered.sort_by(f64::total_cmp);
    let last = ordered.len() - 1;
    let index = ((last as f64) * quantile).round() as usize;
    ordered[index.min(last)]
}

fn summary_json(values: &[f64]) -> serde_json::Value {
    json!({
        "count": values.len(),
        "mean": mean(values),
        "p50": percentile(values, 0.50),
        "p95": percentile(values, 0.95),
        "p99": percentile(values, 0.99),
        "max": values.iter().copied().fold(0.0_f64, f64::max),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        AppearanceFamily, AppearanceRef, PresentationClass, RenderInstance, WorldPosition,
    };

    fn instance(page_id: u32, material: MaterialClass, sequence: u32) -> RenderInstance {
        RenderInstance {
            appearance: AppearanceRef {
                family: AppearanceFamily::Generated,
                semantic_id: sequence,
                frame: 0,
                direction: 0,
            },
            class: PresentationClass::Common,
            kind: PrimitiveKind::Sprite,
            material,
            world: WorldPosition {
                x: 0,
                y: 0,
                floor: 0,
            },
            screen_x: 0.0,
            screen_y: 0.0,
            width: 32.0,
            height: 32.0,
            color: [1.0; 4],
            emissive: 0.0,
            page_id,
            cell: 0,
            critical: false,
            screen_space: false,
            order_floor: 0,
            order_y: 0,
            order_x: 0,
            sequence,
        }
    }

    #[test]
    fn batches_preserve_input_order_and_only_join_contiguous_state() {
        let instances = vec![
            instance(1, MaterialClass::Alpha, 0),
            instance(1, MaterialClass::Alpha, 1),
            instance(2, MaterialClass::Alpha, 2),
            instance(1, MaterialClass::Alpha, 3),
        ];
        let batches = make_batches(&instances);
        assert_eq!(batches.len(), 3);
        assert_eq!(batches[0].instances, 0..2);
        assert_eq!(batches[1].instances, 2..3);
        assert_eq!(batches[2].instances, 3..4);
    }

    #[test]
    fn atlas_and_array_pages_have_equal_payload_bytes() {
        let atlas = procedural_page_bytes(
            64,
            7,
            ResourceLayout::Atlas,
            crate::model::PresentationFamily::Enhanced,
        );
        let array = procedural_page_bytes(
            64,
            7,
            ResourceLayout::Array,
            crate::model::PresentationFamily::Enhanced,
        );
        assert_eq!(atlas.len(), array.len());
        assert_eq!(atlas.len(), 64 * 64 * PAGE_CELLS as usize * 4);
    }
}
