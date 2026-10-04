//! GPU resources that draw the tile and sprite batches (ADR-0020 section 4, child N2).
//!
//! One instanced-quad pipeline draws both batches: tiles first, then sprites on top. The vertex
//! stage builds the six corners of each quad from the vertex index, so there is no vertex or
//! index buffer beyond the per-instance data built in `batch.rs`.

use crate::{
    AtlasImage, MAX_BATCH_QUADS, QUAD_INSTANCE_BYTES, SpriteBatch, SurfaceSize, TileBatch,
    VERTICES_PER_QUAD, instance_bytes,
};

const SHADER: &str = r"
struct Screen {
    size: vec2<f32>,
    padding: vec2<f32>,
}

@group(0) @binding(0) var<uniform> screen: Screen;
@group(0) @binding(1) var atlas: texture_2d<f32>;
@group(0) @binding(2) var atlas_sampler: sampler;

struct Instance {
    @location(0) position: vec2<f32>,
    @location(1) size: vec2<f32>,
    @location(2) uv: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) index: u32, instance: Instance) -> VertexOutput {
    var corners = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(1.0, 1.0),
    );
    let corner = corners[index];
    let pixel = instance.position + corner * instance.size;
    var output: VertexOutput;
    output.clip = vec4<f32>(
        pixel.x / screen.size.x * 2.0 - 1.0,
        1.0 - pixel.y / screen.size.y * 2.0,
        0.0,
        1.0,
    );
    output.uv = mix(instance.uv.xy, instance.uv.zw, corner);
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(atlas, atlas_sampler, input.uv);
}
";

const INSTANCE_ATTRIBUTES: [wgpu::VertexAttribute; 3] =
    wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2, 2 => Float32x4];

const UNIFORM_BYTES: u64 = 16;
const INSTANCE_BUFFER_BYTES: u64 = (2 * MAX_BATCH_QUADS * QUAD_INSTANCE_BYTES) as u64;

pub(crate) struct SceneGpu {
    format: wgpu::TextureFormat,
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    uniform: wgpu::Buffer,
    instances: wgpu::Buffer,
    atlas_texture: wgpu::Texture,
}

impl SceneGpu {
    pub(crate) fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        atlas: &AtlasImage,
    ) -> Self {
        let extent = wgpu::Extent3d {
            width: atlas.width(),
            height: atlas.height(),
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("oteryn-renderer-atlas"),
            size: extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            // Authored colours are sRGB; the surface encodes on write, so decode on read.
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            texture.as_image_copy(),
            atlas.rgba(),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(atlas.width() * 4),
                rows_per_image: Some(atlas.height()),
            },
            extent,
        );
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("oteryn-renderer-atlas-sampler"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..wgpu::SamplerDescriptor::default()
        });
        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("oteryn-renderer-screen"),
            size: UNIFORM_BYTES,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let instances = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("oteryn-renderer-instances"),
            size: INSTANCE_BUFFER_BYTES,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("oteryn-renderer-scene-layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(UNIFORM_BYTES),
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
            label: Some("oteryn-renderer-scene-bind-group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("oteryn-renderer-scene-pipeline-layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("oteryn-renderer-scene-shader"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("oteryn-renderer-scene-pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: QUAD_INSTANCE_BYTES as u64,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &INSTANCE_ATTRIBUTES,
                })],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        Self {
            format,
            pipeline,
            bind_group,
            uniform,
            instances,
            atlas_texture: texture,
        }
    }

    /// Writes one RGBA cell into the atlas texture at pixel origin `(x, y)`. The sprite page
    /// calls this for each cell that `SpriteFrame::uploads` lists before the frame draws.
    #[allow(dead_code, reason = "wired by the sprite page owner in windows.rs")]
    pub(crate) fn write_cell(
        &self,
        queue: &wgpu::Queue,
        (x, y): (u32, u32),
        cell_px: u32,
        rgba: &[u8],
    ) {
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                origin: wgpu::Origin3d { x, y, z: 0 },
                ..self.atlas_texture.as_image_copy()
            },
            rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(cell_px * 4),
                rows_per_image: Some(cell_px),
            },
            wgpu::Extent3d {
                width: cell_px,
                height: cell_px,
                depth_or_array_layers: 1,
            },
        );
    }

    pub(crate) const fn format(&self) -> wgpu::TextureFormat {
        self.format
    }

    /// Uploads the screen size and both batches. Batches are bounded by `MAX_BATCH_QUADS`, so
    /// they always fit the instance buffer.
    pub(crate) fn upload(
        &self,
        queue: &wgpu::Queue,
        size: SurfaceSize,
        tiles: &TileBatch,
        sprites: &SpriteBatch,
    ) {
        let mut screen = [0_u8; UNIFORM_BYTES as usize];
        for (chunk, value) in
            screen
                .chunks_exact_mut(4)
                .zip([size.width() as f32, size.height() as f32, 0.0, 0.0])
        {
            chunk.copy_from_slice(&value.to_ne_bytes());
        }
        queue.write_buffer(&self.uniform, 0, &screen);
        let tile_bytes = instance_bytes(tiles.instances());
        let sprite_bytes = instance_bytes(sprites.instances());
        if !tile_bytes.is_empty() {
            queue.write_buffer(&self.instances, 0, &tile_bytes);
        }
        if !sprite_bytes.is_empty() {
            queue.write_buffer(&self.instances, tile_bytes.len() as u64, &sprite_bytes);
        }
    }

    /// Records one instanced draw: the tile instances first, then the sprite instances on top.
    /// Call after `upload` with the same batches.
    pub(crate) fn draw(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        tiles: &TileBatch,
        sprites: &SpriteBatch,
    ) {
        // Bounded by `MAX_BATCH_QUADS` each, so both counts fit in `u32`.
        let tile_count = tiles.len() as u32;
        let sprite_count = sprites.len() as u32;
        if tile_count == 0 && sprite_count == 0 {
            return;
        }
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.set_vertex_buffer(0, self.instances.slice(..));
        pass.draw(0..VERTICES_PER_QUAD, 0..tile_count + sprite_count);
    }
}
