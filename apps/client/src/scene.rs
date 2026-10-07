//! The tile scene: real map sprites from a [`World`] around the own actor, overlay markers,
//! the own player's outfit and the target highlight (CLIENT-VIS-1).
//!
//! View coordinates are the session's; `anchor` is added to reach map coordinates, so a session
//! whose positions are not map positions still draws the start area around its actor.

use crate::input::{StepDir, TargetKind, Targetable, pick_target};
use crate::world::{BLACK_CELL, Draw, MARKER_CELL, TARGET_CELL, World};
use oteryn_client_assets::CELL_PX;
use oteryn_renderer::{AtlasImage, BatchError, SpriteBatch, TileBatch, TileCoord, TileView};
use std::sync::Arc;

/// Sprite cells are 32 px; the scene draws them at 1.5x.
pub const SCENE_TILE_PX: u32 = 48;
pub const SCENE_COLUMNS: u32 = 15;
pub const SCENE_ROWS: u32 = 11;

#[derive(Debug, Clone)]
pub struct Scene {
    world: Arc<World>,
    anchor: TileCoord,
    own: TileCoord,
    facing: StepDir,
    view: TileView,
    tiles: TileBatch,
    sprites: SpriteBatch,
    visible: Vec<Targetable>,
    target: Option<Targetable>,
}

impl Scene {
    /// The view centred on `own`, the player facing `facing`, with one marker per overlay
    /// object tile.
    pub fn centered_on(
        world: Arc<World>,
        anchor: TileCoord,
        own: TileCoord,
        facing: StepDir,
        markers: &[TileCoord],
    ) -> Result<Self, BatchError> {
        let origin = TileCoord::new(
            own.x
                .saturating_sub(i32::try_from(SCENE_COLUMNS / 2).unwrap_or(0)),
            own.y
                .saturating_sub(i32::try_from(SCENE_ROWS / 2).unwrap_or(0)),
        );
        let view = TileView::new(origin, SCENE_TILE_PX, SCENE_COLUMNS, SCENE_ROWS)?;
        let cells = vec![BLACK_CELL; (SCENE_COLUMNS * SCENE_ROWS) as usize];
        let tiles = TileBatch::from_cells(&view, world.atlas(), &cells)?;
        let visible = markers
            .iter()
            .map(|&tile| Targetable {
                tile,
                kind: TargetKind::Object,
            })
            .chain([Targetable {
                tile: own,
                kind: TargetKind::Entity,
            }])
            .collect();
        let mut scene = Self {
            world,
            anchor,
            own,
            facing,
            view,
            tiles,
            sprites: SpriteBatch::new(),
            visible,
            target: None,
        };
        scene.rebuild_sprites()?;
        Ok(scene)
    }

    /// Current target, if any.
    #[must_use]
    pub const fn target(&self) -> Option<Targetable> {
        self.target
    }

    /// Selects what is drawn on `tile` (entity before object). Returns the new target; a tile
    /// with nothing on it leaves the target unchanged and returns `None`.
    pub fn select_tile(&mut self, tile: TileCoord) -> Result<Option<Targetable>, BatchError> {
        let Some(picked) = pick_target(tile, &self.visible) else {
            return Ok(None);
        };
        self.target = Some(picked);
        self.rebuild_sprites()?;
        Ok(Some(picked))
    }

    pub fn clear_target(&mut self) -> Result<(), BatchError> {
        self.target = None;
        self.rebuild_sprites()
    }

    /// Row by row, two tiles past the view on the right and bottom (a displaced 64 px sprite
    /// reaches back up and left by up to two tiles): each tile's map items, its markers and the player, then what goes over them.
    /// The target highlight is drawn last.
    fn rebuild_sprites(&mut self) -> Result<(), BatchError> {
        let mut sprites = SpriteBatch::new();
        let (atlas, origin) = (self.world.atlas(), self.view.origin());
        let push = |sprites: &mut SpriteBatch, tile, draw: &Draw, lift: i32| {
            let offset = [draw.offset[0] - lift, draw.offset[1] - lift];
            sprites.push_offset(&self.view, atlas, tile, draw.cell, offset, CELL_PX)
        };
        for row in 0..SCENE_ROWS as i32 + 2 {
            for column in 0..SCENE_COLUMNS as i32 + 2 {
                // Past the edge of the coordinate space there is nothing to draw.
                let (Some(x), Some(y)) = (origin.x.checked_add(column), origin.y.checked_add(row))
                else {
                    continue;
                };
                let tile = TileCoord::new(x, y);
                let map = x
                    .checked_add(self.anchor.x)
                    .zip(y.checked_add(self.anchor.y))
                    .and_then(|(x, y)| self.world.tile(x, y));
                for draw in map.iter().flat_map(|map| &map.under) {
                    push(&mut sprites, tile, draw, 0)?;
                }
                let lift = map.map_or(0, |map| map.elevation);
                for marker in self.visible.iter().filter(|v| v.tile == tile) {
                    if marker.kind == TargetKind::Object {
                        let glyph = Draw {
                            cell: MARKER_CELL,
                            offset: [0, 0],
                        };
                        push(&mut sprites, tile, &glyph, lift)?;
                    }
                }
                if tile == self.own {
                    for draw in self.world.player(self.facing) {
                        push(&mut sprites, tile, draw, lift)?;
                    }
                }
                for draw in map.iter().flat_map(|map| &map.over) {
                    push(&mut sprites, tile, draw, 0)?;
                }
            }
        }
        if let Some(target) = self.target {
            sprites.push(&self.view, atlas, target.tile, TARGET_CELL)?;
        }
        self.sprites = sprites;
        Ok(())
    }

    #[must_use]
    pub fn atlas(&self) -> &AtlasImage {
        self.world.atlas()
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

#[cfg(test)]
mod tests {
    use super::*;

    fn scene() -> Result<Scene, BatchError> {
        let world = Arc::new(World::builtin()?);
        let markers = [TileCoord::new(3, -1), TileCoord::new(-2, -2)];
        Scene::centered_on(
            world,
            TileCoord::new(0, 0),
            TileCoord::new(0, 0),
            StepDir::South,
            &markers,
        )
    }

    #[test]
    fn scene_builds_one_quad_per_tile_and_one_per_sprite() -> Result<(), BatchError> {
        let scene = scene()?;
        assert_eq!(scene.tiles().len(), (SCENE_COLUMNS * SCENE_ROWS) as usize);
        assert_eq!(scene.tiles().vertex_count(), 165 * 6);
        assert_eq!(scene.sprites().len(), 3);
        assert_eq!(scene.sprites().vertex_count(), 18);
        Ok(())
    }

    #[test]
    fn scene_view_fits_the_default_window_and_maps_clicks_to_tiles() -> Result<(), BatchError> {
        let scene = scene()?;
        assert_eq!(scene.view().viewport_px(), (720, 528));
        // The player sprite sits on tile (0, 0), drawn last, and a click on it resolves to it.
        let [x, y] = scene.view().tile_to_screen(TileCoord::new(0, 0));
        assert_eq!(scene.sprites().instances()[2].position, [x, y]);
        assert_eq!(
            scene.view().screen_to_tile(x + 1.0, y + 1.0),
            Some(TileCoord::new(0, 0))
        );
        assert_eq!(scene.view().screen_to_tile(720.0, 0.0), None);
        Ok(())
    }

    #[test]
    fn clicking_an_entity_or_an_object_targets_it_and_draws_a_highlight() -> Result<(), BatchError>
    {
        let mut scene = scene()?;
        assert_eq!(scene.target(), None);
        for (tile, kind) in [
            (TileCoord::new(0, 0), TargetKind::Entity),
            (TileCoord::new(-2, -2), TargetKind::Object),
        ] {
            // Click position -> tile -> selection -> highlight on the batch.
            let [x, y] = scene.view().tile_to_screen(tile);
            let clicked =
                crate::input::click_tile(scene.view(), f64::from(x) + 5.0, f64::from(y) + 5.0);
            assert_eq!(clicked, Some(tile));
            let picked = scene.select_tile(tile)?;
            assert_eq!(picked.map(|p| p.kind), Some(kind));
            assert_eq!(scene.target(), picked);
            assert_eq!(scene.sprites().len(), 4);
            assert_eq!(scene.sprites().instances()[3].position, [x, y]);
        }
        // An empty tile keeps the target; clearing removes the highlight.
        assert_eq!(scene.select_tile(TileCoord::new(5, 3))?, None);
        assert_eq!(scene.sprites().len(), 4);
        scene.clear_target()?;
        assert_eq!(scene.sprites().len(), 3);
        Ok(())
    }

    #[test]
    fn a_displaced_sprite_two_tiles_past_the_view_is_drawn() -> Result<(), BatchError> {
        use crate::world::MapTile;
        // Column 16 and row 12 of the view, displaced back into its last column and row.
        let reach = |x, y| {
            let tile = MapTile {
                under: vec![Draw {
                    cell: MARKER_CELL,
                    offset: [-40, -40],
                }],
                ..MapTile::default()
            };
            ((x, y), tile)
        };
        let world = Arc::new(World::builtin_with([reach(9, 0), reach(0, 7)])?);
        let scene = Scene::centered_on(
            world,
            TileCoord::new(0, 0),
            TileCoord::new(0, 0),
            StepDir::South,
            &[],
        )?;
        assert_eq!(scene.sprites().len(), 3);
        Ok(())
    }

    #[test]
    fn a_view_at_the_edge_of_the_coordinate_space_builds() -> Result<(), BatchError> {
        let world = Arc::new(World::builtin()?);
        let own = TileCoord::new(i32::MAX - 8, i32::MAX - 6);
        let scene = Scene::centered_on(world, TileCoord::new(0, 0), own, StepDir::South, &[])?;
        assert_eq!(scene.sprites().len(), 1);
        let world = Arc::new(World::builtin()?);
        let scene = Scene::centered_on(
            world,
            TileCoord::new(i32::MAX, i32::MAX),
            own,
            StepDir::South,
            &[],
        )?;
        assert_eq!(scene.sprites().len(), 1);
        Ok(())
    }

    #[test]
    fn the_start_area_draws_real_sprites_around_the_player() -> Result<(), String> {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let world = Arc::new(World::load(&root)?);
        let start = TileCoord::new(crate::world::START.0, crate::world::START.1);
        let scene = Scene::centered_on(world, TileCoord::new(0, 0), start, StepDir::South, &[])
            .map_err(|error| error.to_string())?;
        // Far more than a quad per tile, all inside the atlas and the batch cap.
        assert!(scene.sprites().len() > (SCENE_COLUMNS * SCENE_ROWS) as usize);
        for quad in scene.sprites().instances() {
            assert!(quad.uv.iter().all(|uv| (0.0..=1.0).contains(uv)));
        }
        // The same area through an anchored session position.
        let anchored = Scene::centered_on(
            Arc::clone(&scene.world),
            TileCoord::new(start.x - 1, start.y + 1),
            TileCoord::new(1, -1),
            StepDir::South,
            &[],
        )
        .map_err(|error| error.to_string())?;
        assert_eq!(anchored.sprites().len(), scene.sprites().len());
        Ok(())
    }
}
