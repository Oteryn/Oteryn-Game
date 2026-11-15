//! Deterministic viewport scanning and painter order, without renderer or input types.

use crate::world::SCAN_MARGIN;
use oteryn_client_domain::world::{Draw, MapTile};

/// Sprite cells are 32 px; the scene draws them at 1.5x.
pub const SCENE_TILE_PX: u32 = 48;
pub const SCENE_COLUMNS: u32 = 15;
pub const SCENE_ROWS: u32 = 11;

/// Top-left session coordinate of the fixed view centred on the own actor.
pub fn scene_origin(own: (i32, i32)) -> (i32, i32) {
    (
        own.0
            .saturating_sub(i32::try_from(SCENE_COLUMNS / 2).unwrap_or(0)),
        own.1
            .saturating_sub(i32::try_from(SCENE_ROWS / 2).unwrap_or(0)),
    )
}

/// Visit map cells, a glyph per occupied tile, the player, then over-layer cells.
/// Renderer preparation supplies the glyph and consumes the ordered draws; the target
/// highlight is drawn separately after the entire scene.
pub fn visit_scene<'a, E>(
    origin: (i32, i32),
    anchor: Option<(i32, i32)>,
    player: ((i32, i32), &[Draw]),
    tile_at: impl Fn(i32, i32) -> Option<&'a MapTile>,
    glyph_at: impl Fn(i32, i32) -> Option<Draw>,
    emit: impl FnMut((i32, i32), Draw) -> Result<(), E>,
) -> Result<(), E> {
    visit_scene_entities(
        origin,
        anchor,
        player,
        tile_at,
        |x, y| glyph_at(x, y).into_iter().collect(),
        emit,
    )
}

/// Scene traversal with all live actors on a tile, preserving map over-layers.
pub fn visit_scene_entities<'a, E>(
    origin: (i32, i32),
    anchor: Option<(i32, i32)>,
    player: ((i32, i32), &[Draw]),
    tile_at: impl Fn(i32, i32) -> Option<&'a MapTile>,
    glyph_at: impl Fn(i32, i32) -> Vec<Draw>,
    emit: impl FnMut((i32, i32), Draw) -> Result<(), E>,
) -> Result<(), E> {
    visit_scene_entities_scrolled(origin, anchor, player, tile_at, glyph_at, [0.; 2], emit)
}

/// Traverse the union of the resting and scrolled viewport, with one guard tile.
/// `scroll` is measured in tiles, with positive values shifting artwork right/down.
pub fn visit_scene_entities_scrolled<'a, E>(
    origin: (i32, i32),
    anchor: Option<(i32, i32)>,
    player: ((i32, i32), &[Draw]),
    tile_at: impl Fn(i32, i32) -> Option<&'a MapTile>,
    glyph_at: impl Fn(i32, i32) -> Vec<Draw>,
    scroll: [f32; 2],
    mut emit: impl FnMut((i32, i32), Draw) -> Result<(), E>,
) -> Result<(), E> {
    let start = scroll.map(|v| (-(v.max(0.).ceil() as i32)).saturating_sub(1));
    let end = scroll.map(|v| {
        (v.min(0.).floor() as i32)
            .saturating_neg()
            .saturating_add(1)
    });
    let rows = start[1]..(SCENE_ROWS as i32 + SCAN_MARGIN).saturating_add(end[1]);
    let columns = start[0]..(SCENE_COLUMNS as i32 + SCAN_MARGIN).saturating_add(end[0]);
    for diagonal in rows.start + columns.start
        ..(rows.end - 1)
            .saturating_add(columns.end - 1)
            .saturating_add(1)
    {
        for column in columns.start.max(diagonal - (rows.end - 1))
            ..=(columns.end - 1).min(diagonal - rows.start)
        {
            let row = diagonal - column;
            // Past the edge of the coordinate space there is nothing to draw.
            let (Some(x), Some(y)) = (origin.0.checked_add(column), origin.1.checked_add(row))
            else {
                continue;
            };
            let tile = (x, y);
            let map = anchor
                .and_then(|anchor| tile_at(x.wrapping_add(anchor.0), y.wrapping_add(anchor.1)));
            for draw in map.iter().flat_map(|map| &map.under) {
                emit(tile, *draw)?;
            }
            let lift = map.map_or(0, |map| map.elevation);
            let lifted = |draw: Draw| Draw {
                offset: [draw.offset[0] - lift, draw.offset[1] - lift],
                ..draw
            };
            for glyph in glyph_at(x, y) {
                emit(tile, lifted(glyph))?;
            }
            if tile == player.0 {
                for draw in player.1 {
                    emit(tile, lifted(*draw))?;
                }
            }
            for draw in map.iter().flat_map(|map| &map.over) {
                emit(tile, *draw)?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
