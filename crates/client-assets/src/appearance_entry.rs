/// The cells of one pattern of `appearance`, at `phase`, over `layers`.
fn entry(
    appearance: &Appearance,
    sheets: &SpriteSheets,
    (pattern_x, pattern_y, pattern_z): (u32, u32, u32),
    layers: std::ops::Range<u32>,
    phase: u32,
) -> Result<ResolvedEntry, AssetError> {
    let mut cells = Vec::new();
    for layer in layers {
        let index = (((phase * appearance.pattern_depth + pattern_z) * appearance.pattern_height
            + pattern_y)
            * appearance.pattern_width
            + pattern_x)
            * appearance.layers
            + layer;
        let sprite_id = appearance
            .sprite_ids
            .get(index as usize)
            .copied()
            .ok_or(AssetError::UnknownSprite { sprite_id: 0 })?;
        if sprite_id == 0 {
            continue;
        }
        let (cells_x, cells_y) = sheets.layout(sprite_id)?.cells();
        for cell_y in 0..cells_y {
            for cell_x in 0..cells_x {
                cells.push(DrawCell {
                    sprite_id,
                    cell_x,
                    cell_y,
                    offset_x: (cell_x as i32 + 1 - cells_x as i32) * CELL_PX as i32
                        - appearance.displacement_x as i32,
                    offset_y: (cell_y as i32 + 1 - cells_y as i32) * CELL_PX as i32
                        - appearance.displacement_y as i32,
                });
            }
        }
    }
    if cells.len() > MAX_ENTRY_CELLS {
        return Err(AssetError::TooManyCells { cells: cells.len() });
    }
    Ok(ResolvedEntry {
        cells,
        elevation: appearance.elevation,
        ground: appearance.ground,
        ground_border: appearance.ground_border,
        on_bottom: appearance.on_bottom,
        on_top: appearance.on_top,
    })
}
