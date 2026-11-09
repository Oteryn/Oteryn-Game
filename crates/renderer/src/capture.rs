//! Explicit readback of the client's rendered surface, never the desktop.
use crate::RendererError;

#[derive(Debug)]
pub struct CapturedFrame {
    pub width: u32,
    pub height: u32,
    pub bgra: Vec<u8>,
}

pub(crate) struct Readback {
    buffer: wgpu::Buffer,
    width: u32,
    height: u32,
    stride: u32,
    rgba: bool,
}
impl Readback {
    pub(crate) fn encode(
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        texture: &wgpu::Texture,
    ) -> Result<Self, RendererError> {
        let width = texture.width();
        let height = texture.height();
        if width == 0 || height == 0 || u64::from(width) * u64::from(height) > 32 * 1024 * 1024 {
            return Err(RendererError::Validation);
        }
        let rgba = match texture.format() {
            wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb => false,
            wgpu::TextureFormat::Rgba8Unorm | wgpu::TextureFormat::Rgba8UnormSrgb => true,
            _ => return Err(RendererError::Validation),
        };
        if !texture.usage().contains(wgpu::TextureUsages::COPY_SRC) {
            return Err(RendererError::Validation);
        }
        let stride = (width * 4).div_ceil(256) * 256;
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("oteryn-explicit-screenshot"),
            size: u64::from(stride) * u64::from(height),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
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
            texture.size(),
        );
        Ok(Self {
            buffer,
            width,
            height,
            stride,
            rgba,
        })
    }
    pub(crate) fn finish(self, device: &wgpu::Device) -> Result<CapturedFrame, RendererError> {
        let (sender, receiver) = std::sync::mpsc::sync_channel(1);
        self.buffer
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                let _ = sender.send(result);
            });
        device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(std::time::Duration::from_secs(2)),
            })
            .map_err(|_| RendererError::Validation)?;
        receiver
            .recv_timeout(std::time::Duration::from_secs(2))
            .map_err(|_| RendererError::Validation)?
            .map_err(|_| RendererError::Validation)?;
        let mapped = self
            .buffer
            .slice(..)
            .get_mapped_range()
            .map_err(|_| RendererError::Validation)?;
        let mut bgra = Vec::with_capacity(self.width as usize * self.height as usize * 4);
        for row in mapped.chunks_exact(self.stride as usize) {
            bgra.extend_from_slice(&row[..self.width as usize * 4]);
        }
        drop(mapped);
        self.buffer.unmap();
        for pixel in bgra.chunks_exact_mut(4) {
            if self.rgba {
                pixel.swap(0, 2);
            }
            pixel[3] = 255;
        }
        Ok(CapturedFrame {
            width: self.width,
            height: self.height,
            bgra,
        })
    }
}
