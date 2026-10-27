//! App composition of pinned assets, deterministic projection and the renderer atlas.
//!
//! A bounded square of the actual join floor around its position is decoded once from the
//! B3 region files, every item is resolved through the pinned 15.30 appearances, and the cells
//! they need are copied into one static atlas. Walking only rebuilds quads over it. Full map
//! streaming is milestone 2.
//!
//! The data comes from a local asset root laid out like the repository (`OTERYN_ASSET_DIR`);
//! the installer ships no map or sprite files. Without it, [`World::builtin`] draws only the
//! builtin cells.

use crate::input::StepDir;
use oteryn_client_assets::window::{ResolvedWindow, within_draw_cap};
use oteryn_client_domain::world::CELL_PX;
use oteryn_client_simulation::world::{project_cell, project_tile};
use oteryn_renderer::world::{BUILTIN_CELLS, WorldAtlas, builtin_rgba};
use oteryn_renderer::{AtlasImage, BatchError};
use std::collections::HashMap;
use std::path::Path;

/// Names the local asset root.
pub const ASSET_DIR_ENV: &str = "OTERYN_ASSET_DIR";
/// Bounded world-loading values supplied by the asset layer.
pub use oteryn_client_assets::window::{MAX_TILE_DRAWS, PLAYER_LOOK_TYPE, RADIUS};
/// Simulation-owned margin used when composing a scene.
pub use oteryn_client_simulation::world::SCAN_MARGIN;
/// Renderer-owned builtin atlas cells.
pub use oteryn_renderer::world::{BLACK_CELL, MARKER_CELL, TARGET_CELL};

/// Outfit directions in pattern order.
const FACINGS: [StepDir; 4] = [StepDir::North, StepDir::East, StepDir::South, StepDir::West];

/// Domain values used to compose the map and its scene.
pub use oteryn_client_domain::world::{Draw, MapTile, START, START_FLOOR};

#[derive(Debug, Clone)]
pub struct World {
    center: (i32, i32, i16),
    atlas: AtlasImage,
    tiles: HashMap<(i32, i32), MapTile>,
    /// The own player's cells per facing, in [`FACINGS`] order.
    player: [Vec<Draw>; 4],
}

impl World {
    /// Only the builtin cells: no map, and the player drawn as the marker glyph.
    pub fn builtin() -> Result<Self, BatchError> {
        let atlas = AtlasImage::new(CELL_PX, BUILTIN_CELLS as u32, 1, builtin_rgba())?;
        Ok(Self {
            center: (START.0, START.1, i16::from(START_FLOOR)),
            atlas,
            tiles: HashMap::new(),
            player: FACINGS.map(|_| {
                vec![Draw {
                    cell: MARKER_CELL,
                    offset: [0, 0],
                }]
            }),
        })
    }

    /// Loads from `OTERYN_ASSET_DIR`; any failure falls back to [`World::builtin`] with the reason.
    pub fn from_env(center: (i32, i32, i16)) -> (Result<Self, BatchError>, Option<String>) {
        let loaded = std::env::var_os(ASSET_DIR_ENV)
            .ok_or_else(|| format!("{ASSET_DIR_ENV} is not set"))
            .and_then(|root| Self::load_around(Path::new(&root), center));
        match loaded {
            Ok(world) => (Ok(world), None),
            Err(reason) => (Self::builtin(), Some(reason)),
        }
    }

    /// Decodes the start area from an asset root laid out like the repository.
    pub fn load(root: &Path) -> Result<Self, String> {
        Self::load_around(root, (START.0, START.1, i16::from(START_FLOOR)))
    }

    pub const fn center(&self) -> (i32, i32, i16) {
        self.center
    }

    /// Decode the real join position's bounded map window, including its floor.
    pub fn load_around(root: &Path, center: (i32, i32, i16)) -> Result<Self, String> {
        let ResolvedWindow {
            entries,
            player,
            mut sheets,
        } = ResolvedWindow::load_around(root, center)?;

        let atlas = WorldAtlas::prepare(
            entries
                .iter()
                .flat_map(|(_, items)| items.iter().flat_map(|(_, _, cells)| cells))
                .chain(player.iter().flatten()),
            |sprite, x, y| {
                sheets
                    .cell_rgba(sprite, x, y)
                    .map_err(|error| error.to_string())
            },
        )?;
        let slot = |cell: &oteryn_client_assets::DrawCell| atlas.slot(cell);
        let tiles = entries
            .iter()
            .map(|(position, items)| (*position, project_tile(items, slot)))
            .collect();
        let player: [Vec<Draw>; 4] = player.map(|cells| {
            cells
                .iter()
                .filter_map(|cell| project_cell(cell, 0, slot))
                .collect()
        });
        for cells in &player {
            within_draw_cap(cells.len()).map_err(|error| format!("outfit: {error}"))?;
        }
        let atlas = atlas.into_image();
        Ok(Self {
            center,
            atlas,
            tiles,
            player,
        })
    }

    #[must_use]
    pub const fn atlas(&self) -> &AtlasImage {
        &self.atlas
    }

    /// The map tile at an absolute map position, if it lies in the loaded area.
    #[must_use]
    pub fn tile(&self, x: i32, y: i32) -> Option<&MapTile> {
        self.tiles.get(&(x, y))
    }

    #[must_use]
    pub fn tile_count(&self) -> usize {
        self.tiles.len()
    }

    /// The own player's cells facing `facing`, standing on a tile.
    #[must_use]
    pub fn player(&self, facing: StepDir) -> &[Draw] {
        let index = FACINGS.iter().position(|f| *f == facing).unwrap_or(2);
        &self.player[index]
    }
}

#[cfg(test)]
impl World {
    /// The builtin world with the given map tiles.
    pub(crate) fn builtin_with(
        tiles: impl IntoIterator<Item = ((i32, i32), MapTile)>,
    ) -> Result<Self, BatchError> {
        let mut world = Self::builtin()?;
        world.tiles.extend(tiles);
        Ok(world)
    }
}

#[cfg(test)]
#[path = "map/tests.rs"]
mod tests;

#[cfg(test)]
#[path = "features/0031-tutorial-notices/tests/world.rs"]
mod tutorial_notice_tests;
