//! The tile scene: real map sprites from a [`World`] around the own actor, overlay markers,
//! the own player's outfit and the target highlight (CLIENT-VIS-1).
//!
//! View coordinates are the session's; `anchor` is added to reach map coordinates, so a session
//! whose positions are not map positions still draws the start area around its actor.

use crate::input::{StepDir, TargetKind, Targetable, pick_target};
use crate::map::{BLACK_CELL, Draw, MARKER_CELL, MAX_TILE_DRAWS, SCAN_MARGIN, World};
use oteryn_client_simulation::scene::{scene_origin, visit_scene_entities};
use oteryn_renderer::{
    AtlasImage, BatchError, MAX_BATCH_QUADS, SpriteBatch, TileBatch, TileCoord, TileView,
};
use std::sync::Arc;

/// Compatibility exports for the deterministic viewport policy.
pub use oteryn_client_simulation::scene::{SCENE_COLUMNS, SCENE_ROWS, SCENE_TILE_PX};

// A full view plus the minimap (background, terrain, creature and user marker per cell, plus own marker)
// fits one sprite batch.
// One floor (map cells, a marker per tile, the player and the target) fits one sprite batch.
// The renderer also bounds the aggregate draws from the visible floors.
const _: () = assert!(
    ((SCENE_COLUMNS + SCAN_MARGIN as u32) * (SCENE_ROWS + SCAN_MARGIN as u32)) as usize
        * (MAX_TILE_DRAWS + 1)
        + MAX_TILE_DRAWS
        + (31 * 25 * 4 + 1)
        < MAX_BATCH_QUADS
);

#[derive(Debug, Clone)]
pub struct Scene {
    minimap: bool,
    minimap_state: Option<crate::play::feature_local_minimap::state::SharedMinimap>,
    pub(crate) minimap_memory: crate::play::MinimapMemory,
    pub(crate) minimap_floor: i16,
    pub(crate) minimap_origin: Option<oteryn_session::ActorPosition>,
    world: Arc<World>,
    lit_atlas: Option<AtlasImage>,
    /// `None` draws no map: the session is on a floor the loaded map does not cover.
    anchor: Option<TileCoord>,
    own: TileCoord,
    facing: StepDir,
    view: TileView,
    tiles: TileBatch,
    sprites: SpriteBatch,
    visible: Vec<Targetable>,
    target: Option<Targetable>,
    presentation_atlas: Option<Arc<AtlasImage>>,
    cache: appearance_cache::PresentationCache,
    own_identity: Option<[u8; 16]>,
    own_draws: Option<Vec<Draw>>,
    actor_draws: Vec<(TileCoord, Vec<Draw>)>,
    names: Vec<(TileCoord, Vec<u16>)>,
    name_visibility: [bool; 3],
    name_marks: bool,
    npc_names: bool,
    label_scale: u16,
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
                item: None,
            })
            .chain(entities.iter().copied())
            .chain([Targetable {
                tile: own,
                kind: TargetKind::Entity,
                glyph: false,
                entity: None,
                item: None,
            }])
            .collect();
        let mut scene = Self {
            minimap: false,
            minimap_state: None,
            minimap_floor: -7,
            minimap_memory: Default::default(),
            minimap_origin: None,
            world,
            lit_atlas: None,
            anchor,
            own,
            facing,
            view,
            tiles,
            sprites: SpriteBatch::new(),
            visible,
            target: None,
            presentation_atlas: None,
            cache: appearance_cache::PresentationCache::default(),
            own_identity: None,
            own_draws: None,
            actor_draws: Vec::new(),
            names: Vec::new(),
            name_visibility: [true; 3],
            name_marks: true,
            npc_names: true,
            label_scale: 100,
        };
        scene.rebuild_sprites()?;
        Ok(scene)
    }

    pub fn set_weather(&mut self, color: [u8; 3], strength: u8) {
        self.lit_atlas = Some(oteryn_renderer::weather_atlas(
            self.atlas(),
            color,
            strength,
        ));
    }
    pub fn set_tile_px(&mut self, tile_px: u32) -> Result<(), BatchError> {
        if self.view.tile_px() == tile_px {
            return Ok(());
        }
        self.view = TileView::new(self.view.origin(), tile_px, SCENE_COLUMNS, SCENE_ROWS)?;
        let cells = vec![BLACK_CELL; (SCENE_COLUMNS * SCENE_ROWS) as usize];
        self.tiles = TileBatch::from_cells(&self.view, self.world.atlas(), &cells)?;
        self.rebuild_sprites()
    }

    pub fn set_ambient(&mut self, ambient: [u8; 3]) -> Result<(), BatchError> {
        self.sprites.lighting.ambient = ambient;
        self.sprites.lighting.palette = self
            .unlit_atlas()
            .uv_rect(oteryn_renderer::world::PALETTE_CELL)
            .ok();
        Ok(())
    }

    pub(crate) fn set_lights(&mut self, sources: Vec<oteryn_renderer::lighting::PointLight>) {
        self.sprites.lighting.sources = sources;
    }

    /// Rebuilds base sprites before appending this frame's transient visual quads.
    pub(crate) fn presentation_base(
        &mut self,
    ) -> Result<(&TileView, &AtlasImage, &mut SpriteBatch), BatchError> {
        self.rebuild_sprites()?;
        self.sprites.lighting.emissive_from = Some(self.sprites.len());
        let atlas = self
            .lit_atlas
            .as_ref()
            .or(self.presentation_atlas.as_deref())
            .unwrap_or_else(|| self.world.atlas());
        Ok((&self.view, atlas, &mut self.sprites))
    }

    /// Current target, if any.
    #[must_use]
    pub const fn target(&self) -> Option<Targetable> {
        self.target
    }

    pub(crate) fn target_at(&self, tile: TileCoord) -> Option<Targetable> {
        // The own-actor highlight has no command identity; it must not hide an item below it.
        let on_tile = || {
            self.visible.iter().rev().filter(|target| {
                target.tile == tile && (target.entity.is_some() || target.item.is_some())
            })
        };
        on_tile()
            .find(|target| target.kind == TargetKind::Entity)
            .or_else(|| on_tile().find(|target| target.kind == TargetKind::Object))
            .copied()
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

    /// Keeps a selection only while that exact visible identity still exists.
    pub(crate) fn retain_target(&mut self, target: Option<Targetable>) -> Result<(), BatchError> {
        self.target = target.filter(|target| self.visible.contains(target));
        self.rebuild_sprites()
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
        let (atlas, origin) = (self.atlas(), self.view.origin());
        let mut sprites = oteryn_renderer::world::sprite_batch(
            &self.view,
            atlas,
            |emit| {
                for floor in self.world.draw_floors() {
                    let own_floor = floor == self.world.center().2;
                    visit_scene_entities(
                        (origin.x, origin.y),
                        self.anchor.map(|anchor| (anchor.x, anchor.y)),
                        (
                            (self.own.x, self.own.y),
                            if own_floor {
                                self.own_draws
                                    .as_deref()
                                    .unwrap_or_else(|| self.world.player(self.facing))
                            } else {
                                &[]
                            },
                        ),
                        |x, y| self.world.floor_tile(floor, x, y),
                        |x, y| {
                            let tile = TileCoord::new(x, y);
                            if !own_floor {
                                return Vec::new();
                            }
                            let has_actor = self.actor_draws.iter().any(|(at, _)| *at == tile);
                            // Object markers remain below actors. Entity fallback glyphs are replaced.
                            let marker = self.visible.iter().any(|v| {
                                v.tile == tile
                                    && ((v.kind == TargetKind::Object
                                        && (v.glyph || v.item.is_none()))
                                        || (v.glyph && !has_actor))
                            });
                            marker
                                .then_some(Draw {
                                    cell: MARKER_CELL,
                                    offset: [0, 0],
                                })
                                .into_iter()
                                .chain(
                                    self.actor_draws
                                        .iter()
                                        .filter(|(at, _)| *at == tile)
                                        .flat_map(|(_, draws)| draws.iter().copied()),
                                )
                                .collect()
                        },
                        &mut *emit,
                    )?;
                }
                Ok(())
            },
            self.target.map(|target| target.tile),
        )?;
        for (tile, cells) in &self.names {
            let [x, y] = self.view.tile_to_screen(*tile);
            let scale = f32::from(self.label_scale) / 100.0;
            let left =
                (x + self.view.tile_px() as f32 / 2.0 - cells.len() as f32 * 4.0 * scale).max(0.0);
            for (index, cell) in cells.iter().enumerate() {
                oteryn_renderer::name_text::label_quad_scaled(
                    &mut sprites,
                    atlas,
                    *cell,
                    [
                        left + index as f32 * 8.0 * scale,
                        (y - 14.0 * scale).max(0.0),
                    ],
                    self.label_scale,
                )?;
            }
        }
        sprites.lighting.clone_from(&self.sprites.lighting);
        sprites.lighting.opacity.clear();
        self.sprites = sprites;
        if self.minimap {
            self.draw_minimap()?;
        }
        Ok(())
    }

    #[must_use]
    pub const fn facing(&self) -> StepDir {
        self.facing
    }

    fn unlit_atlas(&self) -> &AtlasImage {
        self.presentation_atlas
            .as_deref()
            .unwrap_or_else(|| self.world.atlas())
    }

    #[must_use]
    pub fn atlas(&self) -> &AtlasImage {
        self.lit_atlas
            .as_ref()
            .unwrap_or_else(|| self.unlit_atlas())
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

#[path = "features/0006-world-spatial-entities/scene.rs"]
mod live_entities;
#[path = "features/local-minimap/scene.rs"]
mod minimap;
#[path = "features/local-reference-preferences/scene_hud.rs"]
mod reference_hud;

#[path = "features/0006-world-spatial-entities/cache.rs"]
mod appearance_cache;

#[path = "features/0023-presentation/scene.rs"]
mod visual_scene;
