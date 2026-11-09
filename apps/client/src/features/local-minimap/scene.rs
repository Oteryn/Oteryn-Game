//! Persistent exploration under the current server window; only the current floor can walk.
use crate::play::feature_local_minimap::state::SharedMinimap;
use crate::scene::Scene;
use oteryn_client_ui::minimap::{COLUMNS, ROWS, TILE_PX, view};
use oteryn_renderer::world::{BLACK_CELL, MARKER_CELL, PALETTE_CELL, TARGET_CELL};
use oteryn_renderer::{BatchError, QuadInstance, TileCoord, TileView};

impl Scene {
    pub(crate) fn set_minimap(
        &mut self,
        enabled: bool,
        state: SharedMinimap,
        floor: i16,
    ) -> Result<(), BatchError> {
        self.minimap = enabled;
        state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .follow(self.own.x, self.own.y, floor);
        self.minimap_state = Some(state);
        self.minimap_floor = floor;
        self.rebuild_sprites()
    }
    pub(crate) fn draw_minimap(&mut self) -> Result<(), BatchError> {
        let view = view()?;
        let Some(shared) = self.minimap_state.clone() else {
            return Ok(());
        };
        let map = shared
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let palette = self.world.atlas().uv_rect(PALETTE_CELL)?;
        let color_uv = |index: u8| {
            let px = self.world.atlas().cell_px();
            let u = palette[0]
                + ((u32::from(index) % px) as f32 + 0.5) / self.world.atlas().width() as f32;
            let v = palette[1]
                + ((u32::from(index) / px) as f32 + 0.5) / self.world.atlas().height() as f32;
            [u, v, u, v]
        };
        for y in 0..ROWS as i32 {
            for x in 0..COLUMNS as i32 {
                let cell = TileCoord::new(x, y);
                self.sprites
                    .push(&view, self.world.atlas(), cell, BLACK_CELL)?;
                let Some((tx, ty, floor)) = map.position(x, y) else {
                    continue;
                };
                let tile = TileCoord::new(tx, ty);
                let mut live = false;
                if floor == self.minimap_floor
                    && map.zoom == 1
                    && let Some(anchor) = self.anchor
                    && let (Some(x), Some(y)) =
                        (tile.x.checked_add(anchor.x), tile.y.checked_add(anchor.y))
                    && let Some(terrain) = self.world.tile(x, y)
                    && let Some(draw) = terrain.under.first().or(terrain.over.first())
                {
                    self.sprites
                        .push(&view, self.world.atlas(), cell, draw.cell)?;
                    live = true;
                }
                if !live && let Some(&color) = map.cells.get(&(floor, tx, ty)) {
                    self.sprites.push_quad(QuadInstance {
                        position: view.tile_to_screen(cell),
                        size: [TILE_PX as f32; 2],
                        uv: color_uv(color),
                    })?;
                }
                if floor == self.minimap_floor
                    && map.zoom == 1
                    && self
                        .visible
                        .iter()
                        .any(|entity| entity.tile == tile && entity.entity.is_some())
                {
                    self.sprites
                        .push(&view, self.world.atlas(), cell, MARKER_CELL)?;
                }
                if map.quest_markers.iter().any(|m| {
                    m.floor == floor
                        && (0..map.zoom).contains(&m.x.saturating_sub(tx))
                        && (0..map.zoom).contains(&m.y.saturating_sub(ty))
                }) {
                    let at = view.tile_to_screen(cell);
                    // Cross icon, distinct from a user marker's solid cell.
                    for (offset, size) in [([0.0, 1.0], [4.0, 2.0]), ([1.0, 0.0], [2.0, 4.0])] {
                        self.sprites.push_quad(QuadInstance {
                            position: [at[0] + offset[0], at[1] + offset[1]],
                            size,
                            uv: color_uv(180),
                        })?;
                    }
                }
                if let Some(marker) = map.markers.iter().find(|m| {
                    m.floor == floor
                        && (0..map.zoom).contains(&m.x.saturating_sub(tx))
                        && (0..map.zoom).contains(&m.y.saturating_sub(ty))
                }) {
                    let color = [180, 5, 30, 210][usize::from(marker.color)];
                    self.sprites.push_quad(QuadInstance {
                        position: view.tile_to_screen(cell),
                        size: [TILE_PX as f32; 2],
                        uv: color_uv(color),
                    })?;
                }
            }
        }
        if map.floor == self.minimap_floor {
            let cell = TileCoord::new(
                self.own.x.saturating_sub(map.x).div_euclid(map.zoom) + 15,
                self.own.y.saturating_sub(map.y).div_euclid(map.zoom) + 12,
            );
            if view.contains(cell) {
                let marker = TileView::new(TileCoord::new(0, 0), 12, COLUMNS, ROWS)?;
                self.sprites.push_offset(
                    &marker,
                    self.world.atlas(),
                    TileCoord::new(0, 0),
                    TARGET_CELL,
                    [cell.x * TILE_PX as i32 - 4, cell.y * TILE_PX as i32 - 4],
                    12,
                )?;
            }
        }
        Ok(())
    }
}
