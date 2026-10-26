use super::*;

fn cell(sprite_id: u32, cell_x: u32) -> DrawCell {
    DrawCell {
        sprite_id,
        cell_x,
        cell_y: 0,
        offset_x: 0,
        offset_y: 0,
    }
}

#[test]
fn atlas_slots_deduplicate_and_read_in_sprite_order() -> Result<(), String> {
    let cells = [cell(9, 1), cell(2, 0), cell(9, 0), cell(9, 1)];
    let mut reads = Vec::new();
    let atlas = WorldAtlas::prepare(&cells, |sprite, x, y| {
        reads.push((sprite, x, y));
        Ok(vec![sprite as u8; (CELL_PX * CELL_PX * 4) as usize])
    })?;
    assert_eq!(reads, [(2, 0, 0), (9, 0, 0), (9, 1, 0)]);
    assert_eq!(atlas.slot(&cells[0]), Some(5));
    assert_eq!(atlas.slot(&cells[1]), Some(3));
    assert_eq!(atlas.slot(&cells[2]), Some(4));
    assert_eq!(atlas.slot(&cells[3]), Some(5));
    assert_eq!(atlas.slot(&cell(10, 0)), None);
    let image = atlas.into_image();
    assert_eq!(image.cell_count(), 6);
    // Each source cell is copied into its assigned horizontal slot, line by line.
    for (slot, value) in [(3, 2), (4, 9), (5, 9)] {
        for y in 0..CELL_PX as usize {
            let start = (y * image.width() as usize + slot * CELL_PX as usize) * 4;
            assert!(
                image.rgba()[start..start + CELL_PX as usize * 4]
                    .iter()
                    .all(|byte| *byte == value)
            );
        }
    }
    Ok(())
}

#[test]
fn atlas_capacity_leaves_excess_slots_black_without_reading_them() -> Result<(), String> {
    let cells: Vec<_> = (0..MAX_CELLS as u32)
        .map(|sprite| cell(sprite, 0))
        .collect();
    let mut reads = 0;
    let atlas = WorldAtlas::prepare(&cells, |_, _, _| {
        reads += 1;
        Ok(vec![0; (CELL_PX * CELL_PX * 4) as usize])
    })?;
    assert_eq!(reads, MAX_CELLS - BUILTIN_CELLS);
    assert_eq!(atlas.slot(&cells[reads - 1]), Some((MAX_CELLS - 1) as u16));
    for cell in &cells[reads..] {
        assert_eq!(atlas.slot(cell), Some(BLACK_CELL));
    }
    assert_eq!(atlas.into_image().cell_count() as usize, MAX_CELLS);
    Ok(())
}

#[test]
fn atlas_decode_error_keeps_the_sprite_context() {
    let result = WorldAtlas::prepare(&[cell(7, 0)], |_, _, _| Err("decode failed".into()));
    assert_eq!(result.err().as_deref(), Some("sprite 7: decode failed"));
}

#[test]
fn scene_batch_preserves_offsets_order_and_last_target() -> Result<(), BatchError> {
    let view = TileView::new(TileCoord::new(0, 0), 48, 2, 2)?;
    let atlas = AtlasImage::new(CELL_PX, BUILTIN_CELLS as u32, 1, builtin_rgba())?;
    let batch = sprite_batch(
        &view,
        &atlas,
        |emit| {
            emit(
                (1, 1),
                Draw {
                    cell: MARKER_CELL,
                    offset: [-16, -8],
                },
            )?;
            // An offset can bring a tile beyond the view back into the batch.
            emit(
                (2, 1),
                Draw {
                    cell: BLACK_CELL,
                    offset: [-16, 0],
                },
            )?;
            emit(
                (3, 1),
                Draw {
                    cell: BLACK_CELL,
                    offset: [0, 0],
                },
            )
        },
        Some(TileCoord::new(1, 1)),
    )?;
    let quads = batch.instances();
    assert_eq!(quads.len(), 3);
    assert_eq!(quads[0].position, [24.0, 36.0]);
    assert_eq!(quads[0].uv, atlas.uv_rect(MARKER_CELL)?);
    assert_eq!(quads[1].position, [72.0, 48.0]);
    assert_eq!(quads[2].position, [48.0, 48.0]);
    assert_eq!(quads[2].uv, atlas.uv_rect(TARGET_CELL)?);
    Ok(())
}

#[test]
fn scene_batch_propagates_visitor_and_invalid_cell_errors() -> Result<(), BatchError> {
    let view = TileView::new(TileCoord::new(0, 0), 48, 2, 2)?;
    let atlas = AtlasImage::new(CELL_PX, BUILTIN_CELLS as u32, 1, builtin_rgba())?;
    let result = sprite_batch(&view, &atlas, |_| Err(BatchError::InvalidView), None);
    assert_eq!(result.err(), Some(BatchError::InvalidView));
    let result = sprite_batch(
        &view,
        &atlas,
        |emit| {
            emit(
                (0, 0),
                Draw {
                    cell: 3,
                    offset: [0, 0],
                },
            )
        },
        None,
    );
    assert_eq!(result.err(), Some(BatchError::CellOutOfRange { cell: 3 }));
    Ok(())
}
