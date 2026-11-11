//! Personal NPC map marks use the native minimap batch, independently of actors and targets.
use super::Scene;
use oteryn_renderer::{BatchError, QuadInstance};
impl Scene {
    pub(super) fn draw_personal_map_marks(
        &mut self,
        map: &crate::play::feature_local_minimap::state::Minimap,
    ) -> Result<(), BatchError> {
        let Some(anchor) = self.anchor else {
            return Ok(());
        };
        let center = oteryn_renderer::TileCoord::new(map.x, map.y);
        let atlas = self.world.atlas();
        // Sample a single emissive red palette pixel. Do not change builtin atlas layout.
        let cell = u32::from(oteryn_renderer::world::PALETTE_CELL);
        let index = 5 * 36; // red=255, green=blue=0 in the existing six-step palette.
        let columns = atlas.width() / atlas.cell_px();
        let x = (cell % columns) * atlas.cell_px() + index % atlas.cell_px();
        let y = (cell / columns) * atlas.cell_px() + index / atlas.cell_px();
        let u = (x as f32 + 0.5) / atlas.width() as f32;
        let v = (y as f32 + 0.5) / atlas.height() as f32;
        for cell in oteryn_client_ui::personal_map_marks::project(
            &map.quest_marks,
            center,
            anchor,
            map.floor,
            map.zoom,
        ) {
            let x = cell.x as f32 * 4.0;
            let y = cell.y as f32 * 4.0;
            for (dx, dy) in [(0., 0.), (2., 0.), (1., 1.), (0., 2.), (2., 2.)] {
                self.sprites.push_quad(QuadInstance {
                    position: [x + dx, y + dy],
                    size: [1., 1.],
                    uv: [u, v, u, v],
                })?;
            }
        }
        let lines = oteryn_client_ui::personal_map_marks::caption(
            &map.quest_marks,
            center,
            anchor,
            map.floor,
        );
        if !lines.is_empty() {
            let black = self
                .world
                .atlas()
                .uv_rect(oteryn_renderer::world::BLACK_CELL)?;
            self.sprites.push_quad(QuadInstance {
                position: [0., 102.],
                size: [124., (lines.len() * 10 + 4) as f32],
                uv: black,
            })?;
        }
        for (row, line) in lines.iter().enumerate() {
            for (column, character) in line.chars().enumerate() {
                let rgba = oteryn_renderer::name_text::glyph_rgba(character);
                for dy in 0..8 {
                    for dx in 0..6 {
                        let sx = (dx * 32 + 16) / 6;
                        let sy = (dy * 32 + 16) / 8;
                        if rgba[(sy * 32 + sx) * 4] != 0 {
                            self.sprites.push_quad(QuadInstance {
                                position: [(column * 6 + dx) as f32, (104 + row * 10 + dy) as f32],
                                size: [1., 1.],
                                uv: [u, v, u, v],
                            })?;
                        }
                    }
                }
            }
        }
        Ok(())
    }
}
