use crate::config::{
    AssetProof, BenchConfig, CorpusCensus, Family, RESOURCE_SPRITES_PER_PAGE, ResourceMode,
};
use crate::metrics::{
    AdapterEvidence, AssetEvidence, CacheCounters, CorpusEvidence, FrameSample, Percentiles,
    ReliabilityEvidence, RunResult, Samples,
};
use crate::model::{
    AnimationEvaluationEvidence, BlendMode, GeometryClass, RenderPrimitive, RenderSnapshot,
    SourceClass, TextureRef,
};
use bytemuck::{Pod, Zeroable};
use std::collections::BTreeSet;
use std::mem::size_of;
use std::ops::Range;
use std::sync::{Arc, mpsc};
use std::time::{Duration, Instant};
use winit::window::Window;

const SPRITES_PER_PAGE: u32 = RESOURCE_SPRITES_PER_PAGE;
const ATLAS_COLUMNS: u32 = 8;
const MAX_INSTANCES: usize = 200_000;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct GpuInstance {
    pos_size: [f32; 4],
    resource: [f32; 4],
    color: [f32; 4],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum PageKind {
    Atlas,
    Array,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct PageKey {
    kind: PageKind,
    page: u32,
}

struct CachedPage {
    key: PageKey,
    last_used_frame: u64,
    bind_group: wgpu::BindGroup,
    _texture: wgpu::Texture,
    bytes: u64,
}

struct PageCache {
    pages: Vec<CachedPage>,
    capacity: usize,
    counters: CacheCounters,
}

struct CacheLayouts<'a> {
    atlas: &'a wgpu::BindGroupLayout,
    array: &'a wgpu::BindGroupLayout,
    sampler: &'a wgpu::Sampler,
    instances: &'a wgpu::Buffer,
}

impl PageCache {
    fn new(capacity: usize) -> Self {
        Self {
            pages: Vec::with_capacity(capacity),
            capacity,
            counters: CacheCounters::default(),
        }
    }

    fn ensure_page(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        layouts: CacheLayouts<'_>,
        density: u32,
        key: PageKey,
        frame_number: u64,
    ) -> Result<(usize, Duration), String> {
        if let Some(index) = self.pages.iter().position(|page| page.key == key) {
            self.counters.hits = self.counters.hits.saturating_add(1);
            self.pages[index].last_used_frame = frame_number;
            return Ok((index, Duration::ZERO));
        }

        self.counters.misses = self.counters.misses.saturating_add(1);
        let slot = if self.pages.len() < self.capacity {
            self.pages.len()
        } else {
            self.pages
                .iter()
                .enumerate()
                .filter(|(_, page)| page.last_used_frame != frame_number)
                .min_by_key(|(_, page)| page.last_used_frame)
                .map(|(index, _)| index)
                .ok_or_else(|| {
                    "cache capacity exhausted by pages active in current frame".to_owned()
                })?
        };

        let upload_start = Instant::now();
        let new_page = create_cached_page(device, queue, layouts, density, key, frame_number)?;
        let upload_cpu = upload_start.elapsed();

        self.counters.uploads = self.counters.uploads.saturating_add(1);
        self.counters.upload_bytes = self.counters.upload_bytes.saturating_add(new_page.bytes);
        if slot < self.pages.len() {
            self.counters.evictions = self.counters.evictions.saturating_add(1);
            self.pages[slot] = new_page;
        } else {
            self.pages.push(new_page);
        }
        Ok((slot, upload_cpu))
    }

    fn overflow_fallback(&mut self) {
        self.counters.overflow_fallbacks = self.counters.overflow_fallbacks.saturating_add(1);
    }

    fn estimated_gpu_bytes(&self) -> u64 {
        self.pages.iter().map(|page| page.bytes).sum()
    }
}

fn create_cached_page(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layouts: CacheLayouts<'_>,
    density: u32,
    key: PageKey,
    frame_number: u64,
) -> Result<CachedPage, String> {
    let cell = density.saturating_mul(2);
    match key.kind {
        PageKind::Atlas => {
            let width = cell.saturating_mul(ATLAS_COLUMNS);
            let height = width;
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("oteryn-world-vfx-atlas-page"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
            let bytes = generate_atlas_page(width, height, cell, key.page);
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                &bytes,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(width * 4),
                    rows_per_image: Some(height),
                },
                wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
            );
            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            let bind_group = texture_bind_group(
                device,
                layouts.atlas,
                layouts.instances,
                &view,
                layouts.sampler,
                "oteryn-world-vfx-atlas-bind",
            );
            Ok(CachedPage {
                key,
                last_used_frame: frame_number,
                bind_group,
                _texture: texture,
                bytes: u64::from(width) * u64::from(height) * 4,
            })
        }
        PageKind::Array => {
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("oteryn-world-vfx-array-page"),
                size: wgpu::Extent3d {
                    width: cell,
                    height: cell,
                    depth_or_array_layers: SPRITES_PER_PAGE,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
            let bytes = generate_array_page(cell, key.page);
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                &bytes,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(cell * 4),
                    rows_per_image: Some(cell),
                },
                wgpu::Extent3d {
                    width: cell,
                    height: cell,
                    depth_or_array_layers: SPRITES_PER_PAGE,
                },
            );
            let view = texture.create_view(&wgpu::TextureViewDescriptor {
                dimension: Some(wgpu::TextureViewDimension::D2Array),
                ..wgpu::TextureViewDescriptor::default()
            });
            let bind_group = texture_bind_group(
                device,
                layouts.array,
                layouts.instances,
                &view,
                layouts.sampler,
                "oteryn-world-vfx-array-bind",
            );
            Ok(CachedPage {
                key,
                last_used_frame: frame_number,
                bind_group,
                _texture: texture,
                bytes: u64::from(cell) * u64::from(cell) * u64::from(SPRITES_PER_PAGE) * 4,
            })
        }
    }
}

fn texture_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    instances: &wgpu::Buffer,
    view: &wgpu::TextureView,
    sampler: &wgpu::Sampler,
    label: &'static str,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some(label),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: instances.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(view),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::Sampler(sampler),
            },
        ],
    })
}

fn generate_atlas_page(width: u32, height: u32, cell: u32, page: u32) -> Vec<u8> {
    let mut bytes = vec![0_u8; width as usize * height as usize * 4];
    for local in 0..SPRITES_PER_PAGE {
        let sprite_id = page.saturating_mul(SPRITES_PER_PAGE).saturating_add(local);
        let ox = (local % ATLAS_COLUMNS) * cell;
        let oy = (local / ATLAS_COLUMNS) * cell;
        paint_cell(&mut bytes, width, ox, oy, cell, cell, sprite_id);
    }
    bytes
}

fn generate_array_page(cell: u32, page: u32) -> Vec<u8> {
    let layer_bytes = cell as usize * cell as usize * 4;
    let mut bytes = vec![0_u8; layer_bytes * SPRITES_PER_PAGE as usize];
    for layer in 0..SPRITES_PER_PAGE {
        let sprite_id = page.saturating_mul(SPRITES_PER_PAGE).saturating_add(layer);
        let start = layer as usize * layer_bytes;
        paint_cell(
            &mut bytes[start..start + layer_bytes],
            cell,
            0,
            0,
            cell,
            cell,
            sprite_id,
        );
    }
    bytes
}

fn paint_cell(
    bytes: &mut [u8],
    row_width: u32,
    origin_x: u32,
    origin_y: u32,
    width: u32,
    height: u32,
    sprite_id: u32,
) {
    let base = pseudo_color(sprite_id);
    for y in 0..height {
        for x in 0..width {
            let gx = origin_x + x;
            let gy = origin_y + y;
            let offset = ((gy * row_width + gx) * 4) as usize;
            let edge = x < 2 || y < 2 || x + 2 >= width || y + 2 >= height;
            let checker = ((x / 8) + (y / 8) + sprite_id) & 1 == 0;
            bytes[offset] = if edge {
                235
            } else if checker {
                base[0]
            } else {
                base[0] / 2
            };
            bytes[offset + 1] = if edge {
                235
            } else if checker {
                base[1]
            } else {
                base[1] / 2
            };
            bytes[offset + 2] = if edge {
                235
            } else if checker {
                base[2]
            } else {
                base[2] / 2
            };
            bytes[offset + 3] = 255;
        }
    }
}

fn pseudo_color(value: u32) -> [u8; 3] {
    [
        ((value.wrapping_mul(47).wrapping_add(53) % 191) + 48) as u8,
        ((value.wrapping_mul(83).wrapping_add(31) % 191) + 48) as u8,
        ((value.wrapping_mul(29).wrapping_add(97) % 191) + 48) as u8,
    ]
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BatchTarget {
    Solid,
    Fx,
    Fallback,
    Atlas(usize),
    Array(usize),
}

#[derive(Debug, Clone)]
struct Batch {
    target: BatchTarget,
    blend: BlendMode,
    instances: Range<u32>,
}

struct PreparedFrame {
    batches: Vec<Batch>,
    instance_count: usize,
    active_resource_pages: usize,
    upload_cpu: Duration,
}

struct TimestampState {
    query_set: wgpu::QuerySet,
    resolve_buffer: wgpu::Buffer,
    readback_buffer: wgpu::Buffer,
    period_ns: f32,
    query_count: u32,
}

struct Pipelines {
    atlas_alpha: wgpu::RenderPipeline,
    atlas_additive: wgpu::RenderPipeline,
    array_alpha: wgpu::RenderPipeline,
    array_additive: wgpu::RenderPipeline,
    solid_alpha: wgpu::RenderPipeline,
    solid_additive: wgpu::RenderPipeline,
}

pub struct Renderer {
    surface: wgpu::Surface<'static>,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface_config: wgpu::SurfaceConfiguration,
    available_present_modes: Vec<String>,
    instance_buffer: wgpu::Buffer,
    sampler: wgpu::Sampler,
    atlas_layout: wgpu::BindGroupLayout,
    array_layout: wgpu::BindGroupLayout,
    solid_bind_group: wgpu::BindGroup,
    fx_bind_group: wgpu::BindGroup,
    fallback_bind_group: wgpu::BindGroup,
    _fx_texture: wgpu::Texture,
    _fallback_texture: wgpu::Texture,
    pipelines: Pipelines,
    cache: PageCache,
    cache_sample_baseline: Option<CacheCounters>,
    samples: Samples,
    startup: Duration,
    pipeline_prewarm: Duration,
    frame_number: u64,
    last_frame_start: Instant,
    asset: Option<AssetEvidence>,
    timestamp: Option<TimestampState>,
    last_camera: Option<crate::model::CameraState>,
    surface_timeout: u64,
    surface_occluded: u64,
    surface_outdated: u64,
    surface_lost: u64,
    first_frame_ms: Option<f64>,
    animation_evaluation: AnimationEvaluationEvidence,
}

impl Renderer {
    pub fn new(
        window: Arc<Window>,
        config: &BenchConfig,
        animation_evaluation: AnimationEvaluationEvidence,
        startup_before_gpu: Duration,
        asset: Option<&AssetProof>,
    ) -> Result<Self, String> {
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
            ..wgpu::RequestAdapterOptions::default()
        }))
        .map_err(|error| format!("adapter request: {error}"))?;
        let timestamp_supported = adapter.features().contains(wgpu::Features::TIMESTAMP_QUERY);
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

        let caps = surface.get_capabilities(&adapter);
        let mut surface_config = surface
            .get_default_config(&adapter, config.width, config.height)
            .ok_or_else(|| "surface has no default configuration".to_owned())?;
        if let Some(format) = caps
            .formats
            .iter()
            .copied()
            .find(wgpu::TextureFormat::is_srgb)
        {
            surface_config.format = format;
        }
        let available_present_modes = caps
            .present_modes
            .iter()
            .map(|mode| format!("{mode:?}"))
            .collect::<Vec<_>>();
        if caps.present_modes.contains(&wgpu::PresentMode::Immediate) {
            surface_config.present_mode = wgpu::PresentMode::Immediate;
        } else if caps.present_modes.contains(&wgpu::PresentMode::AutoNoVsync) {
            surface_config.present_mode = wgpu::PresentMode::AutoNoVsync;
        }
        surface.configure(&device, &surface_config);

        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("oteryn-world-vfx-instances"),
            size: (MAX_INSTANCES * size_of::<GpuInstance>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let solid_layout = storage_layout(&device, "oteryn-world-vfx-solid-layout");
        let atlas_layout = texture_layout(
            &device,
            wgpu::TextureViewDimension::D2,
            "oteryn-world-vfx-atlas-layout",
        );
        let array_layout = texture_layout(
            &device,
            wgpu::TextureViewDimension::D2Array,
            "oteryn-world-vfx-array-layout",
        );
        let sampler_label = match config.family() {
            Family::Classic => "oteryn-world-vfx-nearest",
            Family::Enhanced | Family::Hd => "oteryn-world-vfx-linear",
        };
        let filter = match config.family() {
            Family::Classic => wgpu::FilterMode::Nearest,
            Family::Enhanced | Family::Hd => wgpu::FilterMode::Linear,
        };
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some(sampler_label),
            mag_filter: filter,
            min_filter: filter,
            mipmap_filter: match config.family() {
                Family::Classic => wgpu::MipmapFilterMode::Nearest,
                Family::Enhanced | Family::Hd => wgpu::MipmapFilterMode::Linear,
            },
            ..wgpu::SamplerDescriptor::default()
        });

        let solid_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("oteryn-world-vfx-solid-bind"),
            layout: &solid_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: instance_buffer.as_entire_binding(),
            }],
        });

        let (fx_texture, fx_view) = create_fx_texture(&device, &queue);
        let fx_bind_group = texture_bind_group(
            &device,
            &atlas_layout,
            &instance_buffer,
            &fx_view,
            &sampler,
            "oteryn-world-vfx-fx-bind",
        );

        let cell = config.density.saturating_mul(2);
        let fallback_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("oteryn-world-vfx-fallback"),
            size: wgpu::Extent3d {
                width: cell * ATLAS_COLUMNS,
                height: cell * ATLAS_COLUMNS,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let fallback_bytes = generate_fallback_page(cell);
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &fallback_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &fallback_bytes,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(cell * ATLAS_COLUMNS * 4),
                rows_per_image: Some(cell * ATLAS_COLUMNS),
            },
            wgpu::Extent3d {
                width: cell * ATLAS_COLUMNS,
                height: cell * ATLAS_COLUMNS,
                depth_or_array_layers: 1,
            },
        );
        let fallback_view = fallback_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let fallback_bind_group = texture_bind_group(
            &device,
            &atlas_layout,
            &instance_buffer,
            &fallback_view,
            &sampler,
            "oteryn-world-vfx-fallback-bind",
        );

        let pipeline_start = Instant::now();
        let pipelines = create_pipelines(
            &device,
            surface_config.format,
            &solid_layout,
            &atlas_layout,
            &array_layout,
        );
        let pipeline_prewarm = pipeline_start.elapsed();
        let query_count = u32::try_from(config.total_frames().saturating_mul(2))
            .map_err(|_| "timestamp query count exceeds u32".to_owned())?;
        let timestamp = if timestamp_supported && query_count > 0 {
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

        Ok(Self {
            surface,
            adapter,
            device,
            queue,
            surface_config,
            available_present_modes,
            instance_buffer,
            sampler,
            atlas_layout,
            array_layout,
            solid_bind_group,
            fx_bind_group,
            fallback_bind_group,
            _fx_texture: fx_texture,
            _fallback_texture: fallback_texture,
            pipelines,
            cache: PageCache::new(config.cache_slots()),
            cache_sample_baseline: None,
            samples: Samples::default(),
            startup,
            pipeline_prewarm,
            frame_number: 0,
            last_frame_start: Instant::now(),
            asset: asset.map(asset_evidence),
            timestamp,
            last_camera: None,
            surface_timeout: 0,
            surface_occluded: 0,
            surface_outdated: 0,
            surface_lost: 0,
            first_frame_ms: None,
            animation_evaluation,
        })
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
        census: &CorpusCensus,
        snapshot: &RenderSnapshot,
    ) -> Result<Option<RunResult>, String> {
        let frame_start = Instant::now();
        let frame_interval = frame_start.duration_since(self.last_frame_start);
        self.last_frame_start = frame_start;
        if self.first_frame_ms.is_none() {
            self.first_frame_ms = Some(frame_interval.as_secs_f64() * 1000.0);
        }

        if self.frame_number == config.warmup_frames {
            self.cache_sample_baseline = Some(self.cache.counters.clone());
        }

        if snapshot.camera.zoom <= 0.0 || snapshot.camera.pixels_per_tile <= 0.0 {
            return Err("invalid camera presentation scale".to_owned());
        }

        let prep_start = Instant::now();
        let prepared = self.prepare_frame(config, snapshot)?;
        let prep = prep_start.elapsed();

        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame)
            | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
            wgpu::CurrentSurfaceTexture::Timeout => {
                self.surface_timeout = self.surface_timeout.saturating_add(1);
                return Ok(None);
            }
            wgpu::CurrentSurfaceTexture::Occluded => {
                self.surface_occluded = self.surface_occluded.saturating_add(1);
                return Ok(None);
            }
            wgpu::CurrentSurfaceTexture::Outdated => {
                self.surface_outdated = self.surface_outdated.saturating_add(1);
                self.surface.configure(&self.device, &self.surface_config);
                return Ok(None);
            }
            wgpu::CurrentSurfaceTexture::Lost => {
                self.surface_lost = self.surface_lost.saturating_add(1);
                self.surface.configure(&self.device, &self.surface_config);
                return Ok(None);
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
                label: Some("oteryn-world-vfx-encoder"),
            });
        let query_base = u32::try_from(self.frame_number.saturating_mul(2))
            .map_err(|_| "timestamp query index exceeds u32".to_owned())?;
        let timestamp_writes =
            self.timestamp
                .as_ref()
                .map(|timestamp| wgpu::RenderPassTimestampWrites {
                    query_set: &timestamp.query_set,
                    beginning_of_pass_write_index: Some(query_base),
                    end_of_pass_write_index: Some(query_base + 1),
                });
        {
            let clear = clear_color(snapshot);
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
            for batch in &prepared.batches {
                let pipeline = self.pipeline(batch.target, batch.blend);
                pass.set_pipeline(pipeline);
                match batch.target {
                    BatchTarget::Solid => {
                        pass.set_bind_group(0, &self.solid_bind_group, &[]);
                    }
                    BatchTarget::Fx => {
                        pass.set_bind_group(0, &self.fx_bind_group, &[]);
                    }
                    BatchTarget::Fallback => {
                        pass.set_bind_group(0, &self.fallback_bind_group, &[]);
                    }
                    BatchTarget::Atlas(index) | BatchTarget::Array(index) => {
                        pass.set_bind_group(0, &self.cache.pages[index].bind_group, &[]);
                    }
                }
                pass.draw(0..6, batch.instances.clone());
            }
        }
        let final_frame = self.frame_number + 1 >= config.total_frames();
        if final_frame && let Some(timestamp) = &self.timestamp {
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
        let _submission = self.queue.submit([encoder.finish()]);
        self.queue.present(frame);

        let camera_moved = self.last_camera.is_some_and(|previous| {
            previous.world_x != snapshot.camera.world_x
                || previous.world_y != snapshot.camera.world_y
        });
        let zoom_changed = self
            .last_camera
            .is_some_and(|previous| previous.zoom != snapshot.camera.zoom);
        let floor_changed = self
            .last_camera
            .is_some_and(|previous| previous.floor != snapshot.camera.floor);
        self.last_camera = Some(snapshot.camera);

        if self.frame_number >= config.warmup_frames {
            self.samples.record(
                FrameSample {
                    frame: frame_interval,
                    prep,
                    batches: prepared.batches.len(),
                    primitives: prepared.instance_count,
                    upload_cpu: prepared.upload_cpu,
                    active_resource_pages: prepared.active_resource_pages,
                    camera_moved,
                    zoom_changed,
                    floor_changed,
                    gameplay_signature: snapshot.gameplay_signature,
                },
                &snapshot.stats,
            );
        }
        self.frame_number = self.frame_number.saturating_add(1);

        if self.samples.len() >= config.sample_frames as usize {
            let gpu_frame_ms = self.collect_gpu_timestamps(config)?;
            return Ok(Some(self.result(config, census, gpu_frame_ms)));
        }
        Ok(None)
    }

    fn prepare_frame(
        &mut self,
        config: &BenchConfig,
        snapshot: &RenderSnapshot,
    ) -> Result<PreparedFrame, String> {
        let width = self.surface_config.width as f32;
        let height = self.surface_config.height as f32;
        let mut gpu_instances = Vec::with_capacity(snapshot.primitives.len());
        let mut batches = Vec::<Batch>::new();
        let mut upload_cpu = Duration::ZERO;
        let mut active_resource_pages = BTreeSet::new();

        for primitive in snapshot
            .primitives
            .iter()
            .filter(|primitive| primitive.intersects_viewport(width, height))
        {
            if gpu_instances.len() >= MAX_INSTANCES {
                return Err(format!(
                    "visible instance bound exceeded: {} >= {MAX_INSTANCES}",
                    gpu_instances.len()
                ));
            }

            if primitive.stable_id != primitive.sort_key.stable_id {
                return Err("presentation identity/order mismatch".to_owned());
            }
            if primitive.critical && primitive.source == SourceClass::Environment {
                return Err("environment VFX cannot claim critical gameplay priority".to_owned());
            }
            if let Some(key) = desired_page_key(config, primitive) {
                active_resource_pages.insert(key);
            }
            let (target, resource, upload) = self.resolve_resource(config, primitive)?;
            upload_cpu += upload;
            let index = gpu_instances.len() as u32;
            gpu_instances.push(encode_instance(primitive, resource, width, height));
            push_batch(&mut batches, target, primitive.blend, index);
        }

        let upload_start = Instant::now();
        if !gpu_instances.is_empty() {
            self.queue.write_buffer(
                &self.instance_buffer,
                0,
                bytemuck::cast_slice(&gpu_instances),
            );
        }
        upload_cpu += upload_start.elapsed();

        Ok(PreparedFrame {
            batches,
            instance_count: gpu_instances.len(),
            active_resource_pages: active_resource_pages.len(),
            upload_cpu,
        })
    }

    fn resolve_resource(
        &mut self,
        config: &BenchConfig,
        primitive: &RenderPrimitive,
    ) -> Result<(BatchTarget, [f32; 4], Duration), String> {
        match primitive.texture {
            TextureRef::Solid => Ok((BatchTarget::Solid, [0.0; 4], Duration::ZERO)),
            TextureRef::Fx { cell } => {
                let cell = u32::from(cell) % 4;
                let x = cell % 2;
                let y = cell / 2;
                Ok((
                    BatchTarget::Fx,
                    [x as f32 * 0.5, y as f32 * 0.5, 0.5, 0.5],
                    Duration::ZERO,
                ))
            }
            TextureRef::World {
                sprite_id,
                geometry,
                missing_variant,
            } => {
                if missing_variant {
                    return Ok((
                        BatchTarget::Fallback,
                        fallback_uv(geometry, config.density),
                        Duration::ZERO,
                    ));
                }
                let kind = resource_kind(config.resource_mode, geometry);
                let page = sprite_id / SPRITES_PER_PAGE;
                let cell = sprite_id % SPRITES_PER_PAGE;
                let key = PageKey { kind, page };
                let layouts = CacheLayouts {
                    atlas: &self.atlas_layout,
                    array: &self.array_layout,
                    sampler: &self.sampler,
                    instances: &self.instance_buffer,
                };
                match self.cache.ensure_page(
                    &self.device,
                    &self.queue,
                    layouts,
                    config.density,
                    key,
                    self.frame_number,
                ) {
                    Ok((index, upload)) => match kind {
                        PageKind::Atlas => Ok((
                            BatchTarget::Atlas(index),
                            atlas_uv(cell, geometry, config.density),
                            upload,
                        )),
                        PageKind::Array => Ok((
                            BatchTarget::Array(index),
                            [
                                cell as f32,
                                geometry.width_ratio(),
                                geometry.height_ratio(),
                                0.0,
                            ],
                            upload,
                        )),
                    },
                    Err(_) => {
                        self.cache.overflow_fallback();
                        Ok((
                            BatchTarget::Fallback,
                            fallback_uv(geometry, config.density),
                            Duration::ZERO,
                        ))
                    }
                }
            }
        }
    }

    fn pipeline(&self, target: BatchTarget, blend: BlendMode) -> &wgpu::RenderPipeline {
        match (target, blend) {
            (BatchTarget::Solid, BlendMode::Alpha) => &self.pipelines.solid_alpha,
            (BatchTarget::Solid, BlendMode::Additive) => &self.pipelines.solid_additive,
            (BatchTarget::Array(_), BlendMode::Alpha) => &self.pipelines.array_alpha,
            (BatchTarget::Array(_), BlendMode::Additive) => &self.pipelines.array_additive,
            (_, BlendMode::Alpha) => &self.pipelines.atlas_alpha,
            (_, BlendMode::Additive) => &self.pipelines.atlas_additive,
        }
    }

    fn collect_gpu_timestamps(&self, config: &BenchConfig) -> Result<Option<Percentiles>, String> {
        let Some(timestamp) = &self.timestamp else {
            return Ok(None);
        };
        let slice = timestamp.readback_buffer.slice(..);
        let (sender, receiver) = mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
        self.device
            .poll(wgpu::PollType::wait_indefinitely())
            .map_err(|error| format!("device poll for timestamp readback: {error:?}"))?;
        receiver
            .recv()
            .map_err(|error| format!("timestamp callback channel: {error}"))?
            .map_err(|error| format!("timestamp map: {error}"))?;
        let bytes = slice
            .get_mapped_range()
            .map_err(|error| format!("timestamp mapped range: {error}"))?;
        let mut values = Vec::with_capacity(config.sample_frames as usize);
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
                values.push((end - begin) as f64 * f64::from(timestamp.period_ns) / 1_000_000.0);
            }
        }
        drop(bytes);
        timestamp.readback_buffer.unmap();
        if values.len() != config.sample_frames as usize {
            return Ok(None);
        }
        Ok(Some(percentiles(&values)))
    }

    fn result(
        &self,
        config: &BenchConfig,
        census: &CorpusCensus,
        gpu_frame_ms: Option<Percentiles>,
    ) -> RunResult {
        let frame = self.samples.frame();
        let cache = self.cache.counters.delta_from(
            self.cache_sample_baseline
                .as_ref()
                .unwrap_or(&CacheCounters::default()),
        );
        let info = self.adapter.get_info();
        let limits = self.adapter.limits();
        let gpu_timestamp_reliable = gpu_frame_ms.is_some();
        let sampler = match config.family() {
            Family::Classic => "nearest",
            Family::Enhanced | Family::Hd => "linear",
        };
        RunResult {
            schema: "oteryn-world-vfx-prototype-result-v1",
            backend: "custom-wgpu-30.0.0-dx12",
            workload: if config.atlas_slice_path.is_some() {
                "real_atlas_fullworld_slice"
            } else {
                "procedural_corpus_shape"
            }
            .to_owned(),
            scenario: config.scenario.as_str().to_owned(),
            resource_mode: config.resource_mode.as_str().to_owned(),
            presentation_family: config.family().as_str().to_owned(),
            density: config.density,
            width: self.surface_config.width,
            height: self.surface_config.height,
            warmup_frames: config.warmup_frames,
            sample_frames: config.sample_frames,
            seed: config.seed,
            startup_ms: self.startup.as_secs_f64() * 1000.0,
            first_frame_ms: self.first_frame_ms.unwrap_or(0.0),
            surface_format: format!("{:?}", self.surface_config.format),
            present_mode: format!("{:?}", self.surface_config.present_mode),
            available_present_modes: self.available_present_modes.clone(),
            sampler: sampler.to_owned(),
            surface_is_srgb: self.surface_config.format.is_srgb(),
            adapter: AdapterEvidence {
                name: info.name,
                driver: info.driver,
                driver_info: info.driver_info,
                backend: format!("{:?}", info.backend),
                device_type: format!("{:?}", info.device_type),
                max_texture_array_layers: limits.max_texture_array_layers,
            },
            frame_ms: frame,
            gpu_frame_ms,
            prep_ms: self.samples.prep(),
            order_preserving_batches: self.samples.batches(),
            visible_primitives: self.samples.primitives(),
            upload_submit_cpu_ms: self.samples.upload_cpu(),
            active_resource_pages: self.samples.active_resource_pages(),
            camera_scroll_stall_ms: self.samples.camera_scroll(),
            zoom_change_stall_ms: self.samples.zoom_change(),
            floor_transition_stall_ms: self.samples.floor_transition(),
            gameplay_signature_xor: format!("{:016x}", self.samples.gameplay_signature_xor()),
            mean_fps: if frame.mean > 0.0 {
                1000.0 / frame.mean
            } else {
                0.0
            },
            cache,
            cache_resident_pages: self.cache.pages.len(),
            cache_capacity_pages: self.cache.capacity,
            cache_estimated_gpu_bytes: self.cache.estimated_gpu_bytes(),
            max_semantics: self.samples.max_semantics.clone(),
            corpus: corpus_evidence(census),
            asset: self.asset.clone(),
            gpu_timestamp_reliable,
            vram_measurement_reliable: false,
            reliability: ReliabilityEvidence {
                surface_timeout: self.surface_timeout,
                surface_occluded: self.surface_occluded,
                surface_outdated: self.surface_outdated,
                surface_lost: self.surface_lost,
                device_loss_detected: false,
            },
            pipeline_prewarm_ms: self.pipeline_prewarm.as_secs_f64() * 1000.0,
            animation_evaluation: self.animation_evaluation.clone(),
        }
    }

    pub fn pipeline_prewarm(&self) -> Duration {
        self.pipeline_prewarm
    }
}

fn desired_page_key(config: &BenchConfig, primitive: &RenderPrimitive) -> Option<PageKey> {
    match primitive.texture {
        TextureRef::World {
            sprite_id,
            geometry,
            missing_variant: false,
        } => Some(PageKey {
            kind: resource_kind(config.resource_mode, geometry),
            page: sprite_id / SPRITES_PER_PAGE,
        }),
        _ => None,
    }
}

fn resource_kind(mode: ResourceMode, geometry: GeometryClass) -> PageKind {
    match mode {
        ResourceMode::Atlas => PageKind::Atlas,
        ResourceMode::Array => PageKind::Array,
        ResourceMode::Hybrid => {
            if geometry.is_square() {
                PageKind::Array
            } else {
                PageKind::Atlas
            }
        }
    }
}

fn atlas_uv(cell: u32, geometry: GeometryClass, density: u32) -> [f32; 4] {
    let cell_x = cell % ATLAS_COLUMNS;
    let cell_y = cell / ATLAS_COLUMNS;
    let page_px = density.saturating_mul(2).saturating_mul(ATLAS_COLUMNS);
    let inset = 0.5 / page_px.max(1) as f32;
    let base_u = cell_x as f32 / ATLAS_COLUMNS as f32 + inset;
    let base_v = cell_y as f32 / ATLAS_COLUMNS as f32 + inset;
    let du = geometry.width_ratio() / ATLAS_COLUMNS as f32 - inset * 2.0;
    let dv = geometry.height_ratio() / ATLAS_COLUMNS as f32 - inset * 2.0;
    [base_u, base_v, du.max(inset), dv.max(inset)]
}

fn fallback_uv(geometry: GeometryClass, density: u32) -> [f32; 4] {
    atlas_uv(0, geometry, density)
}

fn encode_instance(
    primitive: &RenderPrimitive,
    resource: [f32; 4],
    width: f32,
    height: f32,
) -> GpuInstance {
    let [x, y, w, h] = primitive.rect_px;
    let center_x = ((x + w * 0.5) / width) * 2.0 - 1.0;
    let center_y = 1.0 - ((y + h * 0.5) / height) * 2.0;
    GpuInstance {
        pos_size: [center_x, center_y, w / width * 2.0, h / height * 2.0],
        resource,
        color: primitive.color,
    }
}

fn push_batch(batches: &mut Vec<Batch>, target: BatchTarget, blend: BlendMode, instance: u32) {
    if let Some(last) = batches.last_mut()
        && last.target == target
        && last.blend == blend
        && last.instances.end == instance
    {
        last.instances.end = instance.saturating_add(1);
        return;
    }
    batches.push(Batch {
        target,
        blend,
        instances: instance..instance.saturating_add(1),
    });
}

fn clear_color(snapshot: &RenderSnapshot) -> wgpu::Color {
    let night = f64::from(snapshot.environment.night_factor.clamp(0.0, 1.0));
    let winter = f64::from(snapshot.environment.winter_factor.clamp(0.0, 1.0));
    wgpu::Color {
        r: 0.055 * (1.0 - night * 0.45) + winter * 0.012,
        g: 0.070 * (1.0 - night * 0.50) + winter * 0.018,
        b: 0.090 * (1.0 - night * 0.35) + winter * 0.030,
        a: 1.0,
    }
}

fn storage_layout(device: &wgpu::Device, label: &'static str) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some(label),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only: true },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    })
}

fn texture_layout(
    device: &wgpu::Device,
    dimension: wgpu::TextureViewDimension,
    label: &'static str,
) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some(label),
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
                    view_dimension: dimension,
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
    })
}

fn create_pipelines(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    solid_layout: &wgpu::BindGroupLayout,
    atlas_layout: &wgpu::BindGroupLayout,
    array_layout: &wgpu::BindGroupLayout,
) -> Pipelines {
    let atlas_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("oteryn-world-vfx-atlas-shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("atlas.wgsl").into()),
    });
    let array_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("oteryn-world-vfx-array-shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("array.wgsl").into()),
    });
    let solid_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("oteryn-world-vfx-solid-shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("solid.wgsl").into()),
    });
    Pipelines {
        atlas_alpha: pipeline(
            device,
            format,
            atlas_layout,
            &atlas_shader,
            BlendMode::Alpha,
            "oteryn-world-vfx-atlas-alpha",
        ),
        atlas_additive: pipeline(
            device,
            format,
            atlas_layout,
            &atlas_shader,
            BlendMode::Additive,
            "oteryn-world-vfx-atlas-additive",
        ),
        array_alpha: pipeline(
            device,
            format,
            array_layout,
            &array_shader,
            BlendMode::Alpha,
            "oteryn-world-vfx-array-alpha",
        ),
        array_additive: pipeline(
            device,
            format,
            array_layout,
            &array_shader,
            BlendMode::Additive,
            "oteryn-world-vfx-array-additive",
        ),
        solid_alpha: pipeline(
            device,
            format,
            solid_layout,
            &solid_shader,
            BlendMode::Alpha,
            "oteryn-world-vfx-solid-alpha",
        ),
        solid_additive: pipeline(
            device,
            format,
            solid_layout,
            &solid_shader,
            BlendMode::Additive,
            "oteryn-world-vfx-solid-additive",
        ),
    }
}

fn pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    layout: &wgpu::BindGroupLayout,
    shader: &wgpu::ShaderModule,
    blend: BlendMode,
    label: &'static str,
) -> wgpu::RenderPipeline {
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some(label),
        bind_group_layouts: &[Some(layout)],
        immediate_size: 0,
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(&pipeline_layout),
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
                blend: Some(blend_state(blend)),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    })
}

fn blend_state(mode: BlendMode) -> wgpu::BlendState {
    match mode {
        BlendMode::Alpha => wgpu::BlendState::ALPHA_BLENDING,
        BlendMode::Additive => wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::SrcAlpha,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                operation: wgpu::BlendOperation::Add,
            },
        },
    }
}

fn create_fx_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> (wgpu::Texture, wgpu::TextureView) {
    const CELL: u32 = 64;
    const WIDTH: u32 = CELL * 2;
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("oteryn-world-vfx-fx-atlas"),
        size: wgpu::Extent3d {
            width: WIDTH,
            height: WIDTH,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    let bytes = generate_fx_bytes(CELL);
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &bytes,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(WIDTH * 4),
            rows_per_image: Some(WIDTH),
        },
        wgpu::Extent3d {
            width: WIDTH,
            height: WIDTH,
            depth_or_array_layers: 1,
        },
    );
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    (texture, view)
}

fn generate_fx_bytes(cell: u32) -> Vec<u8> {
    let width = cell * 2;
    let mut bytes = vec![0_u8; width as usize * width as usize * 4];
    for y in 0..width {
        for x in 0..width {
            let cell_x = x / cell;
            let cell_y = y / cell;
            let local_x = (x % cell) as f32 / cell as f32 * 2.0 - 1.0;
            let local_y = (y % cell) as f32 / cell as f32 * 2.0 - 1.0;
            let radius = (local_x * local_x + local_y * local_y).sqrt();
            let shape = cell_y * 2 + cell_x;
            let alpha = match shape {
                0 => (1.0 - radius).clamp(0.0, 1.0),
                1 => (1.0 - ((radius - 0.68).abs() * 7.0)).clamp(0.0, 1.0),
                2 => ((1.0 - radius) * (0.55 + local_y * 0.25)).clamp(0.0, 1.0),
                _ => (1.0 - (local_x.abs() * 1.5 + local_y.abs() * 0.45)).clamp(0.0, 1.0),
            };
            let offset = ((y * width + x) * 4) as usize;
            bytes[offset] = 255;
            bytes[offset + 1] = 255;
            bytes[offset + 2] = 255;
            bytes[offset + 3] = (alpha * 255.0) as u8;
        }
    }
    bytes
}

fn generate_fallback_page(cell: u32) -> Vec<u8> {
    let width = cell * ATLAS_COLUMNS;
    let mut bytes = vec![0_u8; width as usize * width as usize * 4];
    for y in 0..cell {
        for x in 0..cell {
            let offset = ((y * width + x) * 4) as usize;
            let checker = ((x / 8) + (y / 8)) & 1 == 0;
            bytes[offset] = 255;
            bytes[offset + 1] = if checker { 0 } else { 64 };
            bytes[offset + 2] = 220;
            bytes[offset + 3] = 255;
        }
    }
    bytes
}

fn corpus_evidence(census: &CorpusCensus) -> CorpusEvidence {
    CorpusEvidence {
        object_appearances: census.census.object.appearances,
        outfit_appearances: census.census.outfit.appearances,
        effect_appearances: census.census.effect.appearances,
        missile_appearances: census.census.missile.appearances,
        animated_object_appearances: census.census.object.animated_appearances,
        animated_outfit_appearances: census.census.outfit.animated_appearances,
        animated_effect_appearances: census.census.effect.animated_appearances,
        animated_missile_appearances: census.census.missile.animated_appearances,
        unique_world_sprite_ids: census
            .census
            .object
            .unique_sprite_ids
            .saturating_add(census.census.outfit.unique_sprite_ids)
            .saturating_add(census.census.effect.unique_sprite_ids)
            .saturating_add(census.census.missile.unique_sprite_ids),
        source_label: census.source.label.clone(),
        source_zip_sha256: census.source.zip_sha256.clone(),
        source_catalog_sha256: census.source.catalog_sha256.clone(),
        source_appearance_sha256: census.source.appearance_sha256.clone(),
    }
}

fn asset_evidence(proof: &AssetProof) -> AssetEvidence {
    AssetEvidence {
        path: proof.path.display().to_string(),
        bytes: proof.bytes,
        sha256: proof.sha256.clone(),
        expected_sha256: proof.expected_sha256.clone(),
        matches_expected: proof.matches_expected,
    }
}

fn percentiles(values: &[f64]) -> Percentiles {
    if values.is_empty() {
        return Percentiles::default();
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let pick = |q: f64| {
        let last = sorted.len() - 1;
        let index = ((last as f64) * q).round() as usize;
        sorted[index.min(last)]
    };
    Percentiles {
        mean: sorted.iter().sum::<f64>() / sorted.len() as f64,
        p50: pick(0.50),
        p95: pick(0.95),
        p99: pick(0.99),
        max: *sorted.last().unwrap_or(&0.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hybrid_keeps_rectangular_cells_in_atlas() {
        assert_eq!(
            resource_kind(ResourceMode::Hybrid, GeometryClass::Cell32x64),
            PageKind::Atlas
        );
        assert_eq!(
            resource_kind(ResourceMode::Hybrid, GeometryClass::Cell64x64),
            PageKind::Array
        );
    }

    #[test]
    fn batching_never_reorders_instances() {
        let mut batches = Vec::new();
        push_batch(&mut batches, BatchTarget::Solid, BlendMode::Alpha, 0);
        push_batch(&mut batches, BatchTarget::Solid, BlendMode::Alpha, 1);
        push_batch(&mut batches, BatchTarget::Fx, BlendMode::Alpha, 2);
        push_batch(&mut batches, BatchTarget::Solid, BlendMode::Alpha, 3);
        assert_eq!(batches.len(), 3);
        assert_eq!(batches[0].instances, 0..2);
        assert_eq!(batches[1].instances, 2..3);
        assert_eq!(batches[2].instances, 3..4);
    }

    #[test]
    fn additive_blend_is_source_alpha_weighted() {
        let blend = blend_state(BlendMode::Additive);
        assert_eq!(blend.color.src_factor, wgpu::BlendFactor::SrcAlpha);
        assert_eq!(blend.color.dst_factor, wgpu::BlendFactor::One);
    }
}
