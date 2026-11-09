//! Offscreen game capture uses the live scene pipeline and atlas, without native UI windows.
use super::*;

impl<T> WindowsRenderer<T>
where
    T: Clone + Into<wgpu::SurfaceTarget<'static>>,
{
    pub fn capture_batches(
        &mut self,
        tiles: &TileBatch,
        sprites: &SpriteBatch,
    ) -> Result<(u32, u32, Vec<u8>), RendererError> {
        self.prepare_scene(Some(FrameBatches { tiles, sprites }))?;
        let config = self
            .configuration
            .as_ref()
            .ok_or(RendererError::Validation)?;
        let (width, height, format) = (config.width, config.height, config.format);
        if width == 0 || height == 0 || width > 4096 || height > 4096 {
            return Err(RendererError::Validation);
        }
        let bgra = matches!(
            format,
            wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb
        );
        if !bgra
            && !matches!(
                format,
                wgpu::TextureFormat::Rgba8Unorm | wgpu::TextureFormat::Rgba8UnormSrgb
            )
        {
            return Err(RendererError::Validation);
        }
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("screenshot"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let stride = (width * 4).div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT)
            * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("screenshot-readback"),
            size: u64::from(stride) * u64::from(height),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let attachments = [Some(wgpu::RenderPassColorAttachment {
                view: &view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.035,
                        g: 0.055,
                        b: 0.090,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })];
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("screenshot-scene"),
                color_attachments: &attachments,
                ..Default::default()
            });
            self.scene
                .as_ref()
                .ok_or(RendererError::Validation)?
                .draw(&mut pass, tiles, sprites);
        }
        encoder.copy_texture_to_buffer(
            texture.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(stride),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        self.queue.submit([encoder.finish()]);
        let (send, receive) = std::sync::mpsc::sync_channel(1);
        buffer
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                let _ = send.send(result);
            });
        self.device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(std::time::Duration::from_secs(5)),
            })
            .map_err(|_| RendererError::BackendFatal)?;
        receive
            .recv_timeout(std::time::Duration::from_secs(5))
            .map_err(|_| RendererError::BackendFatal)?
            .map_err(|_| RendererError::BackendFatal)?;
        let mapped = buffer
            .slice(..)
            .get_mapped_range()
            .map_err(|_| RendererError::BackendFatal)?;
        let mut rgba = Vec::with_capacity(width as usize * height as usize * 4);
        for row in mapped.chunks_exact(stride as usize) {
            rgba.extend_from_slice(&row[..width as usize * 4]);
        }
        drop(mapped);
        buffer.unmap();
        if bgra {
            for pixel in rgba.chunks_exact_mut(4) {
                pixel.swap(0, 2);
            }
        }
        Ok((width, height, rgba))
    }
}
