//! App composition of pinned assets, deterministic projection and the renderer atlas.
//!
//! Live play resolves server-supplied map tiles through pinned appearances and sprite sheets.
//! Offline browsing loads a bounded window from local B3 regions. Both paths build a bounded
//! atlas and use the same deterministic stack projection. `OTERYN_ASSET_DIR` names the local
//! asset root; missing sprites use builtin glyphs.

use crate::input::StepDir;
use oteryn_client_assets::window::{ResolvedWindow, within_draw_cap};
use oteryn_client_simulation::world::{project_cell, project_tile};
use oteryn_renderer::world::{BUILTIN_CELLS, WorldAtlas, atlas_from_cells, builtin_rgba};
use oteryn_renderer::{AtlasImage, BatchError};
use std::collections::{BTreeMap, HashMap};
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
    effects: oteryn_client_assets::EffectCatalog,
    center: (i32, i32, i16),
    atlas: AtlasImage,
    tiles: HashMap<(i32, i32), MapTile>,
    /// Other floors keep independent stacks at coordinates projected onto the own-floor plane.
    layers: BTreeMap<i16, HashMap<(i32, i32), MapTile>>,
    /// The own player's cells per facing, in [`FACINGS`] order.
    player: [Vec<Draw>; 4],
}

impl World {
    #[must_use]
    pub fn with_effect_catalog(mut self, effects: oteryn_client_assets::EffectCatalog) -> Self {
        self.effects = effects;
        self
    }
    #[must_use]
    pub const fn effects(&self) -> &oteryn_client_assets::EffectCatalog {
        &self.effects
    }

    /// Only the builtin cells: no map, and the player drawn as the marker glyph.
    pub fn builtin() -> Result<Self, BatchError> {
        let atlas = atlas_from_cells(builtin_rgba(), BUILTIN_CELLS)?;
        Ok(Self {
            effects: Default::default(),
            center: (START.0, START.1, i16::from(START_FLOOR)),
            atlas,
            layers: BTreeMap::new(),
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
            .ok_or_else(|| lfmt!("{ASSET_DIR_ENV} is not set", ASSET_DIR_ENV = ASSET_DIR_ENV))
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
            effects,
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
            effects: Default::default(),
            center,
            atlas,
            tiles,
            layers: BTreeMap::new(),
            player,
        }
        .with_effect_catalog(effects))
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
        self.tiles.len() + self.layers.values().map(HashMap::len).sum::<usize>()
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

#[path = "features/local-world-view/world.rs"]
mod live_map;
