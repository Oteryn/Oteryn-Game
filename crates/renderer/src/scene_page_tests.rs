//! Physical DX12 proof that page uploads and later instance batches affect actual pixels.
use super::*;
use crate::{
    AtlasPage, CellKey, PAGE_CELL_PX, PAGE_CELLS_PER_ROW, PlacedCell, SpritePageUpload, TileCoord,
    TileView,
};

fn read_pixel(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    scene: &SceneGpu,
    frame: &SpriteFrame,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let output = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("map-page-readback"),
        size: wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let target = output.create_view(&Default::default());
    let mut encoder = device.create_command_encoder(&Default::default());
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &target,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        });
        scene.draw_page(&mut pass, frame);
    }
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("map-page-pixel"),
        size: 256,
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
    let pixel = mapped[..4].to_vec();
    drop(mapped);
    buffer.unmap();
    Ok(pixel)
}

#[test]
fn dx12_page_readback_keeps_later_batches_cell_overwrites_and_recreated_textures()
-> Result<(), Box<dyn std::error::Error>> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::DX12,
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });
    let adapter = pollster::block_on(instance.request_adapter(&Default::default()))?;
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default()))?;
    let (width, height) = AtlasPage::texture_size();
    let mut atlas = AtlasImage::new(
        PAGE_CELL_PX,
        PAGE_CELLS_PER_ROW,
        PAGE_CELLS_PER_ROW + 1,
        vec![0; (width * height * 4) as usize],
    )?;
    let pixels = |color: [u8; 4]| color.repeat((PAGE_CELL_PX * PAGE_CELL_PX) as usize);
    atlas.apply_page_uploads(&[
        SpritePageUpload {
            slot: 0,
            rgba: pixels([255, 0, 0, 255]),
        },
        SpritePageUpload {
            slot: 1,
            rgba: pixels([0, 255, 0, 255]),
        },
    ])?;
    let view = TileView::new(TileCoord::new(0, 0), 1, 1, 1)?;
    let mut page = AtlasPage::new();
    let mut frame = SpriteFrame::new(16);
    frame.begin(&mut page);
    let cell = |id| PlacedCell {
        key: CellKey {
            sprite_id: id,
            cell_x: 0,
            cell_y: 0,
        },
        offset_x: 0,
        offset_y: 0,
    };
    for _ in 0..MAX_BATCH_QUADS {
        frame.push_entry(&mut page, &view, TileCoord::new(0, 0), &[cell(1)]);
    }
    frame.push_entry(&mut page, &view, TileCoord::new(0, 0), &[cell(2)]);
    assert_eq!(frame.quads().batches().len(), 2);
    let mut scene = SceneGpu::new(&device, &queue, wgpu::TextureFormat::Rgba8Unorm, &atlas);
    scene.upload_page(&device, &queue, SurfaceSize::new(1, 1), &frame)?;
    assert_eq!(
        read_pixel(&device, &queue, &scene, &frame)?,
        [0, 255, 0, 255]
    );
    let update = SpritePageUpload {
        slot: 1,
        rgba: pixels([0, 0, 255, 255]),
    };
    scene.write_cell(
        &queue,
        AtlasPage::slot_origin(update.slot),
        PAGE_CELL_PX,
        &update.rgba,
    );
    atlas.apply_page_uploads(&[update])?;
    assert_eq!(
        read_pixel(&device, &queue, &scene, &frame)?,
        [0, 0, 255, 255]
    );
    let mut recreated = SceneGpu::new(&device, &queue, wgpu::TextureFormat::Rgba8Unorm, &atlas);
    recreated.upload_page(&device, &queue, SurfaceSize::new(1, 1), &frame)?;
    assert_eq!(
        read_pixel(&device, &queue, &recreated, &frame)?,
        [0, 0, 255, 255]
    );
    Ok(())
}
