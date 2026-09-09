use crate::demo::{BlendMode, DemoFrame, SpriteRole, WorldRect};
use crate::visible_set::{CELL_BYTES, VisibleSpriteSet};
use bytemuck::{Pod, Zeroable};
use serde::Serialize;
use std::mem::size_of;
use std::ops::Range;
use std::sync::Arc;
use std::time::Instant;
use winit::window::Window;

const SPRITES_PER_GPU_PAGE: usize = 64;
const MAX_SPRITE_INSTANCES: usize = 16_384;
const MAX_OVERLAY_INSTANCES: usize = 4_096;
const FONT_SCALE_PX: f32 = 2.0;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct GpuSpriteInstance {
    pos_size: [f32; 4],
    resource_data: [f32; 4],
    color: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct GpuOverlayInstance {
    pos_size: [f32; 4],
    color: [f32; 4],
    shape: [f32; 4],
}

struct GpuSpritePage {
    bind_group: wgpu::BindGroup,
    _texture: wgpu::Texture,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SpriteBatchKey {
    page: usize,
    blend: BlendMode,
}

#[derive(Debug, Clone)]
struct SpriteBatch {
    key: SpriteBatchKey,
    instances: Range<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct OverlayBatchKey {
    blend: BlendMode,
}

#[derive(Debug, Clone)]
struct OverlayBatch {
    key: OverlayBatchKey,
    instances: Range<u32>,
}

struct Pipelines {
    sprite_alpha: wgpu::RenderPipeline,
    sprite_additive: wgpu::RenderPipeline,
    overlay_alpha: wgpu::RenderPipeline,
    overlay_additive: wgpu::RenderPipeline,
}

#[derive(Debug, Clone, Serialize)]
pub struct RendererEvidence {
    pub backend: String,
    pub adapter_name: String,
    pub driver: String,
    pub driver_info: String,
    pub device_type: String,
    pub max_texture_array_layers: u32,
    pub surface_format: String,
    pub present_mode: String,
    pub available_present_modes: Vec<String>,
    pub visible_sprite_count: usize,
    pub gpu_page_count: usize,
    pub upload_bytes: usize,
    pub gpu_reserved_bytes: usize,
    pub rendered_frames: u64,
    pub first_render_cpu_ms: Option<f64>,
    pub last_render_cpu_ms: f64,
    pub max_render_cpu_ms: f64,
    pub surface_timeout: u64,
    pub surface_occluded: u64,
    pub surface_outdated: u64,
    pub surface_lost: u64,
    pub device_loss_detected: bool,
    pub gpu_storage_policy: String,
    pub sampler: String,
}

pub struct RealContentRenderer {
    surface: wgpu::Surface<'static>,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface_config: wgpu::SurfaceConfiguration,
    available_present_modes: Vec<String>,
    sprite_instance_buffer: wgpu::Buffer,
    overlay_instance_buffer: wgpu::Buffer,
    sprite_pages: Vec<GpuSpritePage>,
    overlay_bind_group: wgpu::BindGroup,
    pipelines: Pipelines,
    visible_sprite_count: usize,
    upload_bytes: usize,
    gpu_reserved_bytes: usize,
    rendered_frames: u64,
    first_render_cpu_ms: Option<f64>,
    last_render_cpu_ms: f64,
    max_render_cpu_ms: f64,
    surface_timeout: u64,
    surface_occluded: u64,
    surface_outdated: u64,
    surface_lost: u64,
}

impl RealContentRenderer {
    pub fn new(window: Arc<Window>, visible: &VisibleSpriteSet) -> Result<Self, String> {
        if visible.entries().is_empty() || visible.rgba_cells().is_empty() {
            return Err("real-content renderer requires a non-empty visible sprite set".to_owned());
        }
        if visible.rgba_cells().len() != visible.entries().len().saturating_mul(CELL_BYTES) {
            return Err("visible sprite payload/cardinality mismatch".to_owned());
        }

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
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("oteryn-real-content-device"),
            required_features: wgpu::Features::empty(),
            ..wgpu::DeviceDescriptor::default()
        }))
        .map_err(|error| format!("device request: {error}"))?;

        let caps = surface.get_capabilities(&adapter);
        let mut surface_config = surface
            .get_default_config(&adapter, 1280, 960)
            .ok_or_else(|| "surface has no default configuration".to_owned())?;
        if let Some(format) = caps
            .formats
            .iter()
            .copied()
            .find(wgpu::TextureFormat::is_srgb)
        {
            surface_config.format = format;
        }
        if caps.present_modes.contains(&wgpu::PresentMode::Immediate) {
            surface_config.present_mode = wgpu::PresentMode::Immediate;
        } else if caps.present_modes.contains(&wgpu::PresentMode::AutoNoVsync) {
            surface_config.present_mode = wgpu::PresentMode::AutoNoVsync;
        }
        let available_present_modes = caps
            .present_modes
            .iter()
            .map(|mode| format!("{mode:?}"))
            .collect::<Vec<_>>();
        surface.configure(&device, &surface_config);

        let sprite_instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("oteryn-real-content-sprite-instances"),
            size: (MAX_SPRITE_INSTANCES * size_of::<GpuSpriteInstance>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let overlay_instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("oteryn-real-content-overlay-instances"),
            size: (MAX_OVERLAY_INSTANCES * size_of::<GpuOverlayInstance>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let sprite_layout = sprite_texture_layout(&device);
        let overlay_layout = storage_layout(&device, "oteryn-real-content-overlay-layout");
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("oteryn-real-content-nearest"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..wgpu::SamplerDescriptor::default()
        });

        let page_bytes = SPRITES_PER_GPU_PAGE * CELL_BYTES;
        let gpu_page_count = visible.entries().len().div_ceil(SPRITES_PER_GPU_PAGE);
        let mut sprite_pages = Vec::with_capacity(gpu_page_count);
        for page_index in 0..gpu_page_count {
            let start = page_index.saturating_mul(page_bytes);
            let end = visible.rgba_cells().len().min(start.saturating_add(page_bytes));
            let mut bytes = vec![0_u8; page_bytes];
            bytes[..end.saturating_sub(start)].copy_from_slice(&visible.rgba_cells()[start..end]);
            sprite_pages.push(create_sprite_page(
                &device,
                &queue,
                &sprite_layout,
                &sprite_instance_buffer,
                &sampler,
                &bytes,
            ));
        }

        let overlay_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("oteryn-real-content-overlay-bind"),
            layout: &overlay_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: overlay_instance_buffer.as_entire_binding(),
            }],
        });
        let pipelines = create_pipelines(
            &device,
            surface_config.format,
            &sprite_layout,
            &overlay_layout,
        );

        Ok(Self {
            surface,
            adapter,
            device,
            queue,
            surface_config,
            available_present_modes,
            sprite_instance_buffer,
            overlay_instance_buffer,
            sprite_pages,
            overlay_bind_group,
            pipelines,
            visible_sprite_count: visible.entries().len(),
            upload_bytes: visible.rgba_cells().len(),
            gpu_reserved_bytes: gpu_page_count.saturating_mul(page_bytes),
            rendered_frames: 0,
            first_render_cpu_ms: None,
            last_render_cpu_ms: 0.0,
            max_render_cpu_ms: 0.0,
            surface_timeout: 0,
            surface_occluded: 0,
            surface_outdated: 0,
            surface_lost: 0,
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

    pub fn render(&mut self, frame: &DemoFrame) -> Result<(), String> {
        let start = Instant::now();
        if frame.camera.zoom <= 0.0 || !frame.camera.zoom.is_finite() {
            return Err("real-content camera zoom must be finite and positive".to_owned());
        }
        let width = self.surface_config.width as f32;
        let height = self.surface_config.height as f32;
        if width <= 0.0 || height <= 0.0 {
            return Ok(());
        }

        let mut sprite_instances = Vec::with_capacity(frame.sprites.len());
        let mut sprite_batches = Vec::new();
        for draw in &frame.sprites {
            if draw.role == SpriteRole::OutfitMask {
                return Err(
                    "two-layer outfit mask reached preview; qualification selection must use a one-layer real outfit until Game-owned tint values are carried into the manifest"
                        .to_owned(),
                );
            }
            if sprite_instances.len() >= MAX_SPRITE_INSTANCES {
                return Err(format!(
                    "sprite instance cap exceeded: {} >= {MAX_SPRITE_INSTANCES}",
                    sprite_instances.len()
                ));
            }
            let dense = usize::try_from(draw.dense_index)
                .map_err(|_| "dense sprite index exceeds usize".to_owned())?;
            let page = dense / SPRITES_PER_GPU_PAGE;
            let layer = dense % SPRITES_PER_GPU_PAGE;
            if page >= self.sprite_pages.len() {
                return Err(format!(
                    "dense sprite index {} resolves outside {} GPU pages",
                    draw.dense_index,
                    self.sprite_pages.len()
                ));
            }
            let lighting = light_factor(frame, draw.rect, draw.role);
            let index = u32::try_from(sprite_instances.len())
                .map_err(|_| "sprite instance index exceeds u32".to_owned())?;
            sprite_instances.push(encode_sprite_instance(
                draw.rect,
                layer,
                lighting,
                frame,
                width,
                height,
            ));
            push_sprite_batch(
                &mut sprite_batches,
                SpriteBatchKey {
                    page,
                    blend: draw.blend,
                },
                index,
            );
        }
        if !sprite_instances.is_empty() {
            self.queue.write_buffer(
                &self.sprite_instance_buffer,
                0,
                bytemuck::cast_slice(&sprite_instances),
            );
        }

        let (overlay_instances, overlay_batches) = build_overlays(frame, width, height)?;
        if overlay_instances.len() > MAX_OVERLAY_INSTANCES {
            return Err(format!(
                "overlay instance cap exceeded: {} > {MAX_OVERLAY_INSTANCES}",
                overlay_instances.len()
            ));
        }
        if !overlay_instances.is_empty() {
            self.queue.write_buffer(
                &self.overlay_instance_buffer,
                0,
                bytemuck::cast_slice(&overlay_instances),
            );
        }

        let surface_frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame)
            | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
            wgpu::CurrentSurfaceTexture::Timeout => {
                self.surface_timeout = self.surface_timeout.saturating_add(1);
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Occluded => {
                self.surface_occluded = self.surface_occluded.saturating_add(1);
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Outdated => {
                self.surface_outdated = self.surface_outdated.saturating_add(1);
                self.surface.configure(&self.device, &self.surface_config);
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Lost => {
                self.surface_lost = self.surface_lost.saturating_add(1);
                self.surface.configure(&self.device, &self.surface_config);
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Validation => {
                return Err("surface validation failure".to_owned());
            }
        };
        let view = surface_frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("oteryn-real-content-encoder"),
            });
        {
            let attachments = [Some(wgpu::RenderPassColorAttachment {
                view: &view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(clear_color(frame)),
                    store: wgpu::StoreOp::Store,
                },
            })];
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("oteryn-real-content-sprite-pass"),
                color_attachments: &attachments,
                ..wgpu::RenderPassDescriptor::default()
            });
            for batch in &sprite_batches {
                pass.set_pipeline(match batch.key.blend {
                    BlendMode::Alpha => &self.pipelines.sprite_alpha,
                    BlendMode::Additive => &self.pipelines.sprite_additive,
                });
                pass.set_bind_group(0, &self.sprite_pages[batch.key.page].bind_group, &[]);
                pass.draw(0..6, batch.instances.clone());
            }
        }
        if !overlay_batches.is_empty() {
            let attachments = [Some(wgpu::RenderPassColorAttachment {
                view: &view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })];
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("oteryn-real-content-overlay-pass"),
                color_attachments: &attachments,
                ..wgpu::RenderPassDescriptor::default()
            });
            pass.set_bind_group(0, &self.overlay_bind_group, &[]);
            for batch in &overlay_batches {
                pass.set_pipeline(match batch.key.blend {
                    BlendMode::Alpha => &self.pipelines.overlay_alpha,
                    BlendMode::Additive => &self.pipelines.overlay_additive,
                });
                pass.draw(0..6, batch.instances.clone());
            }
        }
        let _submission = self.queue.submit([encoder.finish()]);
        self.queue.present(surface_frame);

        self.rendered_frames = self.rendered_frames.saturating_add(1);
        let cpu_ms = start.elapsed().as_secs_f64() * 1000.0;
        if self.first_render_cpu_ms.is_none() {
            self.first_render_cpu_ms = Some(cpu_ms);
        }
        self.last_render_cpu_ms = cpu_ms;
        self.max_render_cpu_ms = self.max_render_cpu_ms.max(cpu_ms);
        Ok(())
    }

    pub fn evidence(&self) -> RendererEvidence {
        let info = self.adapter.get_info();
        let limits = self.adapter.limits();
        RendererEvidence {
            backend: format!("{:?}", info.backend),
            adapter_name: info.name,
            driver: info.driver,
            driver_info: info.driver_info,
            device_type: format!("{:?}", info.device_type),
            max_texture_array_layers: limits.max_texture_array_layers,
            surface_format: format!("{:?}", self.surface_config.format),
            present_mode: format!("{:?}", self.surface_config.present_mode),
            available_present_modes: self.available_present_modes.clone(),
            visible_sprite_count: self.visible_sprite_count,
            gpu_page_count: self.sprite_pages.len(),
            upload_bytes: self.upload_bytes,
            gpu_reserved_bytes: self.gpu_reserved_bytes,
            rendered_frames: self.rendered_frames,
            first_render_cpu_ms: self.first_render_cpu_ms,
            last_render_cpu_ms: self.last_render_cpu_ms,
            max_render_cpu_ms: self.max_render_cpu_ms,
            surface_timeout: self.surface_timeout,
            surface_occluded: self.surface_occluded,
            surface_outdated: self.surface_outdated,
            surface_lost: self.surface_lost,
            device_loss_detected: false,
            gpu_storage_policy: "experiment-only-paged-64-layer-texture-array; no production atlas-vs-array verdict"
                .to_owned(),
            sampler: "nearest".to_owned(),
        }
    }
}

fn create_sprite_page(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
    instances: &wgpu::Buffer,
    sampler: &wgpu::Sampler,
    bytes: &[u8],
) -> GpuSpritePage {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("oteryn-real-content-sprite-page"),
        size: wgpu::Extent3d {
            width: 64,
            height: 64,
            depth_or_array_layers: SPRITES_PER_GPU_PAGE as u32,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        bytes,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(64 * 4),
            rows_per_image: Some(64),
        },
        wgpu::Extent3d {
            width: 64,
            height: 64,
            depth_or_array_layers: SPRITES_PER_GPU_PAGE as u32,
        },
    );
    let view = texture.create_view(&wgpu::TextureViewDescriptor {
        dimension: Some(wgpu::TextureViewDimension::D2Array),
        ..wgpu::TextureViewDescriptor::default()
    });
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("oteryn-real-content-sprite-page-bind"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: instances.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(&view),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::Sampler(sampler),
            },
        ],
    });
    GpuSpritePage {
        bind_group,
        _texture: texture,
    }
}

fn encode_sprite_instance(
    rect: WorldRect,
    layer: usize,
    lighting: f32,
    frame: &DemoFrame,
    width: f32,
    height: f32,
) -> GpuSpriteInstance {
    let left = (rect.x_units - frame.camera.center_x_units) * frame.camera.zoom + width * 0.5;
    let top = (rect.y_units - frame.camera.center_y_units) * frame.camera.zoom + height * 0.5;
    let draw_width = rect.width_units * frame.camera.zoom;
    let draw_height = rect.height_units * frame.camera.zoom;
    let center_x = ((left + draw_width * 0.5) / width) * 2.0 - 1.0;
    let center_y = 1.0 - ((top + draw_height * 0.5) / height) * 2.0;
    GpuSpriteInstance {
        pos_size: [
            center_x,
            center_y,
            draw_width / width * 2.0,
            draw_height / height * 2.0,
        ],
        resource_data: [layer as f32, 0.0, 0.0, 0.0],
        color: [lighting, lighting, lighting, 1.0],
    }
}

fn light_factor(frame: &DemoFrame, rect: WorldRect, role: SpriteRole) -> f32 {
    let center_x = rect.x_units + rect.width_units * 0.5;
    let center_y = rect.y_units + rect.height_units * 0.5;
    let dx = center_x - frame.lighting.point.center_x_units;
    let dy = center_y - frame.lighting.point.center_y_units;
    let distance = (dx * dx + dy * dy).sqrt();
    let radial = if frame.lighting.point.radius_units > 0.0 {
        (1.0 - distance / frame.lighting.point.radius_units).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let emissive = if matches!(role, SpriteRole::Effect | SpriteRole::Missile) {
        0.20
    } else {
        0.0
    };
    (frame.lighting.ambient + radial * frame.lighting.point.intensity + emissive).clamp(0.18, 1.0)
}

fn build_overlays(
    frame: &DemoFrame,
    width: f32,
    height: f32,
) -> Result<(Vec<GpuOverlayInstance>, Vec<OverlayBatch>), String> {
    let mut instances = Vec::new();
    let mut batches = Vec::new();

    let telegraph_center = world_to_screen(
        frame.telegraph.center_x_units,
        frame.telegraph.center_y_units,
        frame,
        width,
        height,
    );
    let telegraph_diameter = frame.telegraph.radius_units * 2.0 * frame.camera.zoom;
    push_overlay(
        &mut instances,
        &mut batches,
        OverlayBatchKey {
            blend: BlendMode::Additive,
        },
        overlay_rect_px(
            telegraph_center.0 - telegraph_diameter * 0.5,
            telegraph_center.1 - telegraph_diameter * 0.5,
            telegraph_diameter,
            telegraph_diameter,
            [1.0, 0.24, 0.05, 0.20 + frame.telegraph.pulse * 0.42],
            [1.0, 0.0, 0.0, 0.0],
            width,
            height,
        ),
    )?;

    let anchor = world_to_screen(
        frame.actor_overlay.anchor_x_units,
        frame.actor_overlay.anchor_y_units,
        frame,
        width,
        height,
    );
    let text_width = text_pixel_width(&frame.actor_overlay.name, FONT_SCALE_PX);
    let text_left = anchor.0 - text_width * 0.5;
    let text_top = anchor.1 - 43.0;
    push_overlay(
        &mut instances,
        &mut batches,
        OverlayBatchKey {
            blend: BlendMode::Alpha,
        },
        overlay_rect_px(
            text_left - 4.0,
            text_top - 3.0,
            text_width + 8.0,
            19.0,
            [0.0, 0.0, 0.0, 0.62],
            [0.0; 4],
            width,
            height,
        ),
    )?;
    push_text(
        &mut instances,
        &mut batches,
        text_left,
        text_top,
        &frame.actor_overlay.name,
        [0.96, 0.98, 1.0, 1.0],
        width,
        height,
    )?;

    let hp_width = 76.0;
    let hp_left = anchor.0 - hp_width * 0.5;
    let hp_top = anchor.1 - 21.0;
    push_overlay(
        &mut instances,
        &mut batches,
        OverlayBatchKey {
            blend: BlendMode::Alpha,
        },
        overlay_rect_px(
            hp_left,
            hp_top,
            hp_width,
            7.0,
            [0.03, 0.03, 0.03, 0.88],
            [0.0; 4],
            width,
            height,
        ),
    )?;
    let hp_ratio = frame.actor_overlay.hp_ratio.clamp(0.0, 1.0);
    push_overlay(
        &mut instances,
        &mut batches,
        OverlayBatchKey {
            blend: BlendMode::Alpha,
        },
        overlay_rect_px(
            hp_left + 1.0,
            hp_top + 1.0,
            (hp_width - 2.0) * hp_ratio,
            5.0,
            [0.16, 0.82, 0.24, 0.96],
            [0.0; 4],
            width,
            height,
        ),
    )?;

    Ok((instances, batches))
}

fn push_text(
    instances: &mut Vec<GpuOverlayInstance>,
    batches: &mut Vec<OverlayBatch>,
    left: f32,
    top: f32,
    text: &str,
    color: [f32; 4],
    width: f32,
    height: f32,
) -> Result<(), String> {
    let key = OverlayBatchKey {
        blend: BlendMode::Alpha,
    };
    let mut cursor_x = left;
    for character in text.chars() {
        let rows = glyph_rows(character);
        for (row_index, row) in rows.into_iter().enumerate() {
            for column in 0..5_u8 {
                let bit = 1_u8 << (4 - column);
                if row & bit == 0 {
                    continue;
                }
                let rect = overlay_rect_px(
                    cursor_x + f32::from(column) * FONT_SCALE_PX,
                    top + row_index as f32 * FONT_SCALE_PX,
                    FONT_SCALE_PX,
                    FONT_SCALE_PX,
                    color,
                    [0.0; 4],
                    width,
                    height,
                );
                push_overlay(instances, batches, key, rect)?;
            }
        }
        cursor_x += 6.0 * FONT_SCALE_PX;
    }
    Ok(())
}

fn text_pixel_width(text: &str, scale: f32) -> f32 {
    text.chars().count() as f32 * 6.0 * scale
}

fn glyph_rows(character: char) -> [u8; 7] {
    match character {
        '1' => [0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110],
        '2' => [0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111],
        '3' => [0b11110, 0b00001, 0b00001, 0b01110, 0b00001, 0b00001, 0b11110],
        '5' => [0b11111, 0b10000, 0b10000, 0b11110, 0b00001, 0b00001, 0b11110],
        '.' => [0, 0, 0, 0, 0, 0b00110, 0b00110],
        'A' => [0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001],
        'C' => [0b01111, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b01111],
        'E' => [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111],
        'L' => [0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111],
        'N' => [0b10001, 0b11001, 0b10101, 0b10101, 0b10011, 0b10001, 0b10001],
        'O' => [0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
        'R' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001],
        'T' => [0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100],
        ' ' => [0; 7],
        _ => [0b11111, 0b10001, 0b00110, 0b00100, 0b00110, 0b10001, 0b11111],
    }
}

fn overlay_rect_px(
    left: f32,
    top: f32,
    rect_width: f32,
    rect_height: f32,
    color: [f32; 4],
    shape: [f32; 4],
    width: f32,
    height: f32,
) -> GpuOverlayInstance {
    let center_x = ((left + rect_width * 0.5) / width) * 2.0 - 1.0;
    let center_y = 1.0 - ((top + rect_height * 0.5) / height) * 2.0;
    GpuOverlayInstance {
        pos_size: [
            center_x,
            center_y,
            rect_width / width * 2.0,
            rect_height / height * 2.0,
        ],
        color,
        shape,
    }
}

fn world_to_screen(
    x_units: f32,
    y_units: f32,
    frame: &DemoFrame,
    width: f32,
    height: f32,
) -> (f32, f32) {
    (
        (x_units - frame.camera.center_x_units) * frame.camera.zoom + width * 0.5,
        (y_units - frame.camera.center_y_units) * frame.camera.zoom + height * 0.5,
    )
}

fn push_sprite_batch(batches: &mut Vec<SpriteBatch>, key: SpriteBatchKey, instance: u32) {
    if let Some(last) = batches.last_mut()
        && last.key == key
        && last.instances.end == instance
    {
        last.instances.end = instance.saturating_add(1);
        return;
    }
    batches.push(SpriteBatch {
        key,
        instances: instance..instance.saturating_add(1),
    });
}

fn push_overlay(
    instances: &mut Vec<GpuOverlayInstance>,
    batches: &mut Vec<OverlayBatch>,
    key: OverlayBatchKey,
    instance: GpuOverlayInstance,
) -> Result<(), String> {
    if instances.len() >= MAX_OVERLAY_INSTANCES {
        return Err("overlay instance cap exceeded while building overlay".to_owned());
    }
    let index = u32::try_from(instances.len())
        .map_err(|_| "overlay instance index exceeds u32".to_owned())?;
    instances.push(instance);
    if let Some(last) = batches.last_mut()
        && last.key == key
        && last.instances.end == index
    {
        last.instances.end = index.saturating_add(1);
    } else {
        batches.push(OverlayBatch {
            key,
            instances: index..index.saturating_add(1),
        });
    }
    Ok(())
}

fn clear_color(frame: &DemoFrame) -> wgpu::Color {
    let ambient = f64::from(frame.lighting.ambient.clamp(0.0, 1.0));
    wgpu::Color {
        r: 0.025 * ambient,
        g: 0.036 * ambient,
        b: 0.048 * ambient,
        a: 1.0,
    }
}

fn sprite_texture_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("oteryn-real-content-sprite-layout"),
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
                    view_dimension: wgpu::TextureViewDimension::D2Array,
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

fn create_pipelines(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    sprite_layout: &wgpu::BindGroupLayout,
    overlay_layout: &wgpu::BindGroupLayout,
) -> Pipelines {
    let sprite_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("oteryn-real-content-sprite-shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("real_sprite.wgsl").into()),
    });
    let overlay_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("oteryn-real-content-overlay-shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("overlay.wgsl").into()),
    });
    Pipelines {
        sprite_alpha: pipeline(
            device,
            format,
            sprite_layout,
            &sprite_shader,
            wgpu::BlendState::ALPHA_BLENDING,
            "oteryn-real-content-sprite-alpha",
        ),
        sprite_additive: pipeline(
            device,
            format,
            sprite_layout,
            &sprite_shader,
            additive_blend(),
            "oteryn-real-content-sprite-additive",
        ),
        overlay_alpha: pipeline(
            device,
            format,
            overlay_layout,
            &overlay_shader,
            wgpu::BlendState::ALPHA_BLENDING,
            "oteryn-real-content-overlay-alpha",
        ),
        overlay_additive: pipeline(
            device,
            format,
            overlay_layout,
            &overlay_shader,
            additive_blend(),
            "oteryn-real-content-overlay-additive",
        ),
    }
}

fn pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    layout: &wgpu::BindGroupLayout,
    shader: &wgpu::ShaderModule,
    blend: wgpu::BlendState,
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
                blend: Some(blend),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    })
}

fn additive_blend() -> wgpu::BlendState {
    wgpu::BlendState {
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::demo::{DemoCamera, LightingState, PointLight};

    fn frame() -> DemoFrame {
        DemoFrame {
            elapsed_ms: 0,
            camera: DemoCamera {
                center_x_units: 640.0,
                center_y_units: 480.0,
                zoom: 1.25,
            },
            sprites: Vec::new(),
            actor_overlay: crate::demo::ActorOverlay {
                name: "15.32 REAL CONTENT".to_owned(),
                anchor_x_units: 640.0,
                anchor_y_units: 480.0,
                hp_ratio: 0.73,
            },
            telegraph: crate::demo::TelegraphOverlay {
                center_x_units: 640.0,
                center_y_units: 480.0,
                radius_units: 32.0,
                pulse: 0.5,
            },
            lighting: LightingState {
                ambient: 0.5,
                point: PointLight {
                    center_x_units: 640.0,
                    center_y_units: 480.0,
                    radius_units: 128.0,
                    intensity: 0.8,
                },
            },
            outfit_phase: 0,
            effect_phase: 0,
            missile_phase: 0,
            actor_direction: crate::demo::DemoDirection::South,
        }
    }

    #[test]
    fn screen_space_overlay_anchor_is_zoom_aware() {
        let frame = frame();
        let center = world_to_screen(640.0, 480.0, &frame, 1280.0, 960.0);
        assert!((center.0 - 640.0).abs() < f32::EPSILON);
        assert!((center.1 - 480.0).abs() < f32::EPSILON);
        let east = world_to_screen(672.0, 480.0, &frame, 1280.0, 960.0);
        assert!((east.0 - 680.0).abs() < f32::EPSILON);
    }

    #[test]
    fn name_overlay_fits_bounded_instance_budget() -> Result<(), String> {
        let frame = frame();
        let (instances, _batches) = build_overlays(&frame, 1280.0, 960.0)?;
        assert!(!instances.is_empty());
        assert!(instances.len() < MAX_OVERLAY_INSTANCES);
        Ok(())
    }
}
