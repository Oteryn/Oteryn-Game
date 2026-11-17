//! Native UI warning geometry; these are not Item/effect asset identities.
use super::*;
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GroundWarning {
    pub tile: TileCoord,
    pub color: [u8; 3],
    pub seed: bool,
}
impl Scene {
    pub(crate) fn set_drome_hazards(
        &mut self,
        hazards: Vec<GroundWarning>,
    ) -> Result<(), BatchError> {
        if self.drome_hazards == hazards {
            return Ok(());
        }
        self.drome_hazards = hazards;
        self.rebuild_sprites()
    }
    pub(super) fn draw_drome_hazards(
        &self,
        batch: &mut SpriteBatch,
        atlas: &AtlasImage,
    ) -> Result<(), BatchError> {
        for warning in &self.drome_hazards {
            if !self.view.contains(warning.tile) {
                continue;
            }
            let [x, y] = self.view.tile_to_screen(warning.tile);
            let size = self.view.tile_px() as f32;
            let [r, g, b] = warning.color.map(|component| u32::from(component / 51));
            let index = r * 36 + g * 6 + b;
            let cell = u32::from(oteryn_renderer::world::PALETTE_CELL);
            let columns = atlas.width() / atlas.cell_px();
            let px = (cell % columns) * atlas.cell_px() + index % atlas.cell_px();
            let py = (cell / columns) * atlas.cell_px() + index / atlas.cell_px();
            let u = (px as f32 + 0.5) / atlas.width() as f32;
            let v = (py as f32 + 0.5) / atlas.height() as f32;
            let rectangles = if warning.seed {
                vec![([x + size * 0.35, y + size * 0.35], [size * 0.3, size * 0.3])]
            } else {
                vec![
                    ([x, y], [size, 2.0]),
                    ([x, y + size - 2.0], [size, 2.0]),
                    ([x, y], [2.0, size]),
                    ([x + size - 2.0, y], [2.0, size]),
                ]
            };
            for (position, size) in rectangles {
                batch.push_quad(oteryn_renderer::QuadInstance {
                    position,
                    size,
                    uv: [u, v, u, v],
                })?;
            }
        }
        Ok(())
    }
}
