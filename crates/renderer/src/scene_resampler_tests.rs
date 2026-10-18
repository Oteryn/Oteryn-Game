//! Physical DX12 readback: nearest, smooth and retro must affect actual pixels.
use super::*;
use crate::{
    AtlasImage, SpriteBatch, SurfaceSize, TileBatch, TileCoord, TileView, scene_gpu::SceneGpu,
};

fn capture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    mode: SceneSampling,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let format = wgpu::TextureFormat::Rgba8UnormSrgb;
    let pixels = (0..8)
        .flat_map(|i| if i % 4 < 2 { [0, 0, 0, 255] } else { [255; 4] })
        .collect();
    let atlas = AtlasImage::new(2, 2, 1, pixels)?;
    let view = TileView::new(TileCoord::new(0, 0), 2, 2, 1)?;
    let tiles = TileBatch::from_cells(&view, &atlas, &[0, 1])?;
    let sprites = SpriteBatch::new();
    let scene = SceneGpu::new(device, queue, format, &atlas);
    scene.upload(queue, SurfaceSize::new(4, 2), &tiles, &sprites);
    let source_size = mode
        .source_size([4, 2], [6.0, 3.0], 4096)?
        .ok_or("resampling source")?;
    let resampler = SceneResampler::new(device, format, source_size, mode);
    let output = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("resampling-test-output"),
        size: wgpu::Extent3d {
            width: 6,
            height: 3,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let output_view = output.create_view(&Default::default());
    let mut encoder = device.create_command_encoder(&Default::default());
    for (target, source_pass) in [(&resampler.source, true), (&output_view, false)] {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        });
        if source_pass {
            scene.draw(&mut pass, &tiles, &sprites);
        } else {
            resampler.draw(&mut pass);
        }
    }
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("resampling-test-readback"),
        size: 256 * 3,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    encoder.copy_texture_to_buffer(
        output.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(256),
                rows_per_image: None,
            },
        },
        output.size(),
    );
    let submission = queue.submit([encoder.finish()]);
    let (sender, receiver) = std::sync::mpsc::sync_channel(1);
    buffer.map_async(wgpu::MapMode::Read, .., move |result| {
        let _ = sender.send(result);
    });
    device.poll(wgpu::PollType::Wait {
        submission_index: Some(submission),
        timeout: Some(std::time::Duration::from_secs(10)),
    })?;
    receiver.recv_timeout(std::time::Duration::from_secs(1))??;
    let mapped = buffer.get_mapped_range(..);
    let pixels = crate::unpack_capture_rows(&mapped, 6, 3, 256, crate::CapturedPixelOrder::Rgba)?;
    drop(mapped);
    buffer.unmap();
    Ok(pixels)
}

#[test]
fn dx12_readback_distinguishes_smooth_sampling_from_nearest_and_retro()
-> Result<(), Box<dyn std::error::Error>> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::DX12,
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });
    let adapter = pollster::block_on(instance.request_adapter(&Default::default()))?;
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default()))?;
    let nearest = capture(&device, &queue, SceneSampling::None)?;
    let smooth = capture(&device, &queue, SceneSampling::Antialiasing)?;
    let retro = capture(&device, &queue, SceneSampling::SmoothRetro)?;
    assert_ne!(nearest, smooth, "AA must change real rendered pixels");
    assert_ne!(smooth, retro, "Retro must use its integral intermediate");
    assert_eq!(nearest, retro, "this aligned edge remains sharp in Retro");
    for pixels in [&nearest, &smooth, &retro] {
        for row in pixels.chunks_exact(24) {
            assert_eq!(&row[0..4], &[0, 0, 0, 255]);
            assert_eq!(&row[20..24], &[255; 4]);
        }
    }
    // Resize/mode recreation must never leave an old sampler or image active.
    assert_eq!(capture(&device, &queue, SceneSampling::None)?, nearest);
    Ok(())
}
