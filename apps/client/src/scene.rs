//! Placeholder tile scene built from the production-owned placeholder atlas (ADR-0020, N2).
//!
//! It only exercises the renderer primitives with fixed data. It carries no gameplay state and
//! does not change the fail-closed entry: N3 (click to tile) and N6 (entity rendering) replace
//! the fixed cells and sprites with real input and `WORLD_SPATIAL` data.

use oteryn_placeholder_assets::{PlaceholderCell, placeholder_atlas};
use oteryn_renderer::{AtlasImage, BatchError, SpriteBatch, TileBatch, TileCoord, TileView};

/// Atlas cells are 16 px; the scene draws them at 3x.
pub const SCENE_TILE_PX: u32 = 48;
pub const SCENE_COLUMNS: u32 = 15;
pub const SCENE_ROWS: u32 = 11;

#[derive(Debug, Clone)]
pub struct PlaceholderScene {
    atlas: AtlasImage,
    view: TileView,
    tiles: TileBatch,
    sprites: SpriteBatch,
}

impl PlaceholderScene {
    /// Builds the scene: a stone border, a dirt road, a pond, and three sprites around tile (0, 0).
    pub fn new() -> Result<Self, BatchError> {
        let source = placeholder_atlas();
        let atlas = AtlasImage::new(source.cell_px, source.columns, source.rows, source.rgba)?;
        let origin = TileCoord::new(-7, -5);
        let view = TileView::new(origin, SCENE_TILE_PX, SCENE_COLUMNS, SCENE_ROWS)?;
        let mut cells = Vec::new();
        for row in 0..SCENE_ROWS {
            for column in 0..SCENE_COLUMNS {
                cells.push(terrain_cell(column, row).index());
            }
        }
        let tiles = TileBatch::from_cells(&view, &atlas, &cells)?;
        let mut sprites = SpriteBatch::new();
        for (tile, cell) in [
            (TileCoord::new(0, 0), PlaceholderCell::Player),
            (TileCoord::new(3, -1), PlaceholderCell::Creature),
            (TileCoord::new(-3, 2), PlaceholderCell::OtherPlayer),
        ] {
            sprites.push(&view, &atlas, tile, cell.index())?;
        }
        Ok(Self {
            atlas,
            view,
            tiles,
            sprites,
        })
    }

    #[must_use]
    pub const fn atlas(&self) -> &AtlasImage {
        &self.atlas
    }

    #[must_use]
    pub const fn view(&self) -> &TileView {
        &self.view
    }

    #[must_use]
    pub const fn tiles(&self) -> &TileBatch {
        &self.tiles
    }

    #[must_use]
    pub const fn sprites(&self) -> &SpriteBatch {
        &self.sprites
    }
}

fn terrain_cell(column: u32, row: u32) -> PlaceholderCell {
    if column == 0 || row == 0 || column == SCENE_COLUMNS - 1 || row == SCENE_ROWS - 1 {
        PlaceholderCell::Stone
    } else if row == SCENE_ROWS / 2 + 2 {
        PlaceholderCell::Dirt
    } else if (10..13).contains(&column) && (2..5).contains(&row) {
        PlaceholderCell::Water
    } else {
        PlaceholderCell::Grass
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scene_builds_one_quad_per_tile_and_one_per_sprite() -> Result<(), BatchError> {
        let scene = PlaceholderScene::new()?;
        assert_eq!(scene.tiles().len(), (SCENE_COLUMNS * SCENE_ROWS) as usize);
        assert_eq!(scene.tiles().vertex_count(), 165 * 6);
        assert_eq!(scene.sprites().len(), 3);
        assert_eq!(scene.sprites().vertex_count(), 18);
        Ok(())
    }

    #[test]
    fn scene_view_fits_the_default_window_and_maps_clicks_to_tiles() -> Result<(), BatchError> {
        let scene = PlaceholderScene::new()?;
        assert_eq!(scene.view().viewport_px(), (720, 528));
        // The player sprite sits on tile (0, 0), and a click on it resolves to that tile.
        let [x, y] = scene.view().tile_to_screen(TileCoord::new(0, 0));
        assert_eq!(scene.sprites().instances()[0].position, [x, y]);
        assert_eq!(
            scene.view().screen_to_tile(x + 1.0, y + 1.0),
            Some(TileCoord::new(0, 0))
        );
        assert_eq!(scene.view().screen_to_tile(720.0, 0.0), None);
        Ok(())
    }

    #[test]
    fn every_terrain_and_sprite_cell_is_inside_the_atlas() -> Result<(), BatchError> {
        let scene = PlaceholderScene::new()?;
        assert_eq!(
            scene.atlas().cell_count() as usize,
            PlaceholderCell::ALL.len()
        );
        for cell in PlaceholderCell::ALL {
            scene.atlas().uv_rect(cell.index())?;
        }
        Ok(())
    }
}
