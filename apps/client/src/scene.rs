//! The tile scene: real map sprites from a [`World`] around the own actor, overlay markers,
//! the own player's outfit and the target highlight (CLIENT-VIS-1).
//!
//! View coordinates are the session's; `anchor` is added to reach map coordinates, so a session
//! whose positions are not map positions still draws the start area around its actor.

use crate::input::{StepDir, TargetKind, Targetable, pick_target};
use crate::world::{BLACK_CELL, Draw, MARKER_CELL, MAX_TILE_DRAWS, SCAN_MARGIN, World};
use oteryn_client_simulation::scene::{scene_origin, visit_scene};
use oteryn_renderer::{
    AtlasImage, BatchError, MAX_BATCH_QUADS, SpriteBatch, TileBatch, TileCoord, TileView,
};
use std::sync::Arc;

/// Compatibility exports for the deterministic viewport policy.
pub use oteryn_client_simulation::scene::{SCENE_COLUMNS, SCENE_ROWS, SCENE_TILE_PX};

// A full view (map cells, a marker per tile, the player and the target) fits one sprite batch.
const _: () = assert!(
    ((SCENE_COLUMNS + SCAN_MARGIN as u32) * (SCENE_ROWS + SCAN_MARGIN as u32)) as usize
        * (MAX_TILE_DRAWS + 1)
        + MAX_TILE_DRAWS
        < MAX_BATCH_QUADS
);

#[derive(Debug, Clone)]
pub struct Scene {
    world: Arc<World>,
    /// `None` draws no map: the session is on a floor the loaded map does not cover.
    anchor: Option<TileCoord>,
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
        anchor: Option<TileCoord>,
        own: TileCoord,
        facing: StepDir,
        markers: &[TileCoord],
    ) -> Result<Self, BatchError> {
        Self::centered_with(world, anchor, own, facing, markers, &[])
    }

    /// [`Self::centered_on`] with the other visible entities the server reports, each drawn as a
    /// glyph on its tile and selectable by a click.
    pub fn centered_with(
        world: Arc<World>,
        anchor: Option<TileCoord>,
        own: TileCoord,
        facing: StepDir,
        markers: &[TileCoord],
        entities: &[Targetable],
    ) -> Result<Self, BatchError> {
        let (x, y) = scene_origin((own.x, own.y));
        let origin = TileCoord::new(x, y);
        let view = TileView::new(origin, SCENE_TILE_PX, SCENE_COLUMNS, SCENE_ROWS)?;
        let cells = vec![BLACK_CELL; (SCENE_COLUMNS * SCENE_ROWS) as usize];
        let tiles = TileBatch::from_cells(&view, world.atlas(), &cells)?;
        let visible = markers
            .iter()
            .map(|&tile| Targetable {
                tile,
                kind: TargetKind::Object,
                glyph: false,
                entity: None,
            })
            .chain(entities.iter().copied())
            .chain([Targetable {
                tile: own,
                kind: TargetKind::Entity,
                glyph: false,
                entity: None,
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

    /// Turns the player to `facing`, keeping the target.
    pub fn set_facing(&mut self, facing: StepDir) -> Result<(), BatchError> {
        self.facing = facing;
        self.rebuild_sprites()
    }

    pub fn clear_target(&mut self) -> Result<(), BatchError> {
        self.target = None;
        self.rebuild_sprites()
    }

    /// Row by row, [`SCAN_MARGIN`] tiles past the view on the right and bottom (the world drops
    /// map cells reaching further up and left): each tile's map items, its markers and the
    /// player, then what goes over them.
    /// The target highlight is drawn last.
    fn rebuild_sprites(&mut self) -> Result<(), BatchError> {
        let (atlas, origin) = (self.world.atlas(), self.view.origin());
        let sprites = oteryn_renderer::world::sprite_batch(
            &self.view,
            atlas,
            |emit| {
                visit_scene(
                    (origin.x, origin.y),
                    self.anchor.map(|anchor| (anchor.x, anchor.y)),
                    ((self.own.x, self.own.y), self.world.player(self.facing)),
                    |x, y| self.world.tile(x, y),
                    |x, y| {
                        // One glyph per tile however many markers and entities stand on it.
                        self.visible
                            .iter()
                            .any(|v| {
                                v.tile == TileCoord::new(x, y)
                                    && (v.kind == TargetKind::Object || v.glyph)
                            })
                            .then_some(Draw {
                                cell: MARKER_CELL,
                                offset: [0, 0],
                            })
                    },
                    emit,
                )
            },
            self.target.map(|target| target.tile),
        )?;
        self.sprites = sprites;
        Ok(())
    }

    #[must_use]
    pub const fn facing(&self) -> StepDir {
        self.facing
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
mod tests;
