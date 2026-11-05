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
