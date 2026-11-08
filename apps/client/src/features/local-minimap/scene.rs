//! Render the current server window as a compact overlay over the world batch.
use crate::scene::Scene;
use oteryn_client_ui::minimap::{COLUMNS, ROWS, TILE_PX, position, view};
use oteryn_renderer::world::{BLACK_CELL, MARKER_CELL, TARGET_CELL};
use oteryn_renderer::{BatchError, TileCoord, TileView};

impl Scene {
    pub(crate) fn set_minimap(&mut self, enabled: bool) -> Result<(), BatchError> {
        self.minimap = enabled;
        self.rebuild_sprites()
    }

    pub(crate) fn draw_minimap(&mut self) -> Result<(), BatchError> {
        let view = view()?;
        let remembered: std::collections::BTreeMap<_, _> = self
            .minimap_memory
            .tiles
            .iter()
            .filter(|(_, _, floor, _)| *floor == self.minimap_floor)
            .map(|&(x, y, _, speed)| ((x, y), speed))
            .collect();
        for y in 0..ROWS as i32 {
            for x in 0..COLUMNS as i32 {
                let cell = TileCoord::new(x, y);
                self.sprites
                    .push(&view, self.world.atlas(), cell, BLACK_CELL)?;
                let Some(tile) = position(self.own, cell) else {
                    continue;
                };
                if let Some(anchor) = self.anchor
                    && let (Some(x), Some(y)) =
                        (tile.x.checked_add(anchor.x), tile.y.checked_add(anchor.y))
                    && let Some(terrain) = self.world.tile(x, y)
                    && let Some(draw) = terrain.under.first().or(terrain.over.first())
                {
                    self.sprites
                        .push(&view, self.world.atlas(), cell, draw.cell)?;
                } else if let Some(speed) = remembered.get(&(tile.x, tile.y))
                    && !self.minimap_origin.is_some_and(|origin| {
                        oteryn_session::world_map::in_window(
                            origin,
                            oteryn_session::ActorPosition {
                                x: tile.x,
                                y: tile.y,
                                floor: self.minimap_floor,
                            },
                        )
                    })
                {
                    memory_pixel(
                        &mut self.sprites,
                        &view,
                        self.world.atlas(),
                        cell,
                        if *speed == 0 { [2, 2, 2] } else { [2, 4, 2] },
                    )?;
                }
                if self.minimap_memory.marks.iter().any(|mark| {
                    mark.floor == self.minimap_floor && mark.x == tile.x && mark.y == tile.y
                }) {
                    memory_pixel(
                        &mut self.sprites,
                        &view,
                        self.world.atlas(),
                        cell,
                        [5, 4, 0],
                    )?;
                }
                if self
                    .visible
                    .iter()
                    .any(|entity| entity.tile == tile && entity.entity.is_some())
                {
                    self.sprites
                        .push(&view, self.world.atlas(), cell, MARKER_CELL)?;
                }
            }
        }
        // The atlas outline is two source pixels wide: at four screen pixels it
        // disappears under nearest sampling. A 12px frame keeps a visible 1px edge.
        let marker = TileView::new(TileCoord::new(0, 0), 12, COLUMNS, ROWS)?;
        self.sprites.push_offset(
            &marker,
            self.world.atlas(),
            TileCoord::new(0, 0),
            TARGET_CELL,
            [
                (COLUMNS / 2 * TILE_PX) as i32 - 4,
                (ROWS / 2 * TILE_PX) as i32 - 4,
            ],
            12,
        )?;
        Ok(())
    }
}

// Original minimap colours use the builtin palette; no asset bytes enter transfer files.
fn memory_pixel(
    batch: &mut oteryn_renderer::SpriteBatch,
    view: &TileView,
    atlas: &oteryn_renderer::AtlasImage,
    cell: TileCoord,
    [r, g, b]: [u32; 3],
) -> Result<(), BatchError> {
    let palette = u32::from(oteryn_renderer::world::PALETTE_CELL);
    let columns = atlas.width() / atlas.cell_px();
    let index = r * 36 + g * 6 + b;
    let x = palette % columns * atlas.cell_px() + index % atlas.cell_px();
    let y = palette / columns * atlas.cell_px() + index / atlas.cell_px();
    let u = (x as f32 + 0.5) / atlas.width() as f32;
    let v = (y as f32 + 0.5) / atlas.height() as f32;
    batch.push_quad(oteryn_renderer::QuadInstance {
        position: view.tile_to_screen(cell),
        size: [view.tile_px() as f32; 2],
        uv: [u, v, u, v],
    })
}
