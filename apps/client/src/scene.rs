//! The tile scene: real map sprites from a [`World`] around the own actor, overlay markers,
//! the own player's outfit and the target highlight (CLIENT-VIS-1).
//!
//! View coordinates are the session's; `anchor` is added to reach map coordinates, so a session
//! whose positions are not map positions still draws the start area around its actor.

use crate::input::{StepDir, TargetKind, Targetable, pick_target};
use crate::map::{BLACK_CELL, Draw, MARKER_CELL, MAX_TILE_DRAWS, SCAN_MARGIN, World};
use oteryn_client_simulation::scene::{
    scene_origin, visit_scene_entities_scrolled as visit_scene_entities,
};
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

#[derive(Clone, Debug)]
struct ActorDraw {
    floor: i16,
    tile: TileCoord,
    draws: Vec<Draw>,
}

#[derive(Debug, Clone)]
pub struct Scene {
    actor_clocks: actor_motion::ActorClocks,
    minimap: bool,
    #[cfg(feature = "bevy-shell")]
    minimap_quads: std::ops::Range<usize>,
    /// Fixed sprite ranges: the own actor and screen overlays.
    pub(crate) player_quads: Vec<std::ops::Range<usize>>,
    /// Retained corner gauges and circles omitted by the Bevy HUD.
    #[cfg(feature = "bevy-shell")]
    pub(crate) native_hud_quads: std::ops::Range<usize>,
    floor_separator: u8,
    #[cfg(feature = "bevy-shell")]
    floor_quads: Vec<(std::ops::Range<usize>, i16)>,
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
    pub(crate) scroll: [f32; 2],
    view: TileView,
    tiles: TileBatch,
    sprites: SpriteBatch,
    visible: Vec<Targetable>,
    pub(crate) target_frame: bool,
    target: Option<Targetable>,
    presentation_atlas: Option<Arc<AtlasImage>>,
    cache: appearance_cache::PresentationCache,
    own_identity: Option<[u8; 16]>,
    own_draws: Option<Vec<Draw>>,
    actor_draws: Vec<ActorDraw>,
    pub(crate) floor_visuals: Vec<(i16, Vec<oteryn_renderer::QuadInstance>, u8)>,
    names: Vec<(TileCoord, Vec<u16>)>,
    name_entities: Vec<oteryn_session::EntityRef>,
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
            #[cfg(feature = "bevy-shell")]
            minimap_quads: 0..0,
            player_quads: Vec::new(),
            #[cfg(feature = "bevy-shell")]
            native_hud_quads: 0..0,
            floor_separator: 0,
            #[cfg(feature = "bevy-shell")]
            floor_quads: Vec::new(),
            minimap_state: None,
            minimap_floor: -7,
            minimap_memory: Default::default(),
            minimap_origin: None,
            world,
            lit_atlas: None,
            anchor,
            own,
            facing,
            scroll: [0.; 2],
            view,
            tiles,
            sprites: SpriteBatch::new(),
            visible,
            target_frame: true,
            target: None,
            presentation_atlas: None,
            cache: appearance_cache::PresentationCache::default(),
            own_identity: None,
            own_draws: None,
            actor_draws: Vec::new(),
            actor_clocks: actor_motion::ActorClocks::default(),
            floor_visuals: Vec::new(),
            names: Vec::new(),
            name_entities: Vec::new(),
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

    pub(crate) fn item_target_at(&self, tile: TileCoord) -> Option<crate::play::GroundItem> {
        self.visible
            .iter()
            .rev()
            .filter(|target| target.tile == tile)
            .find_map(|target| target.item)
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

    /// Painter order over the scrolled viewport and guard tiles; the target is drawn last.
    fn rebuild_sprites(&mut self) -> Result<(), BatchError> {
        let (mut sprites, dimmed, floor_quads, player_quads) = self.world_sprites()?;
        #[cfg(feature = "bevy-shell")]
        {
            self.floor_quads = floor_quads;
        }
        #[cfg(not(feature = "bevy-shell"))]
        let _ = floor_quads;
        let atlas = self.atlas();
        let mut player_quads: Vec<_> = player_quads.into_iter().collect();
        for (index, (tile, cells)) in self.names.iter().enumerate() {
            let [dx, dy] = self
                .name_entities
                .get(index)
                .map_or([0.; 2], |entity| self.actor_offset(*entity));
            let [x, y] = self.view.tile_to_screen(*tile);
            let (x, y) = (x + dx, y + dy);
            let name_start = sprites.len();
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
            if *tile == self.own {
                player_quads.push(name_start..sprites.len());
            }
        }
        self.player_quads = player_quads;
        let opacity = std::mem::take(&mut sprites.lighting.opacity);
        sprites.lighting.clone_from(&self.sprites.lighting);
        sprites.lighting.opacity = opacity;
        sprites.lighting.dimmed = dimmed;
        self.sprites = sprites;
        #[cfg(feature = "bevy-shell")]
        let start = self.sprites.instances().len();
        if self.minimap {
            self.draw_minimap()?;
        }
        #[cfg(feature = "bevy-shell")]
        {
            self.minimap_quads = start..self.sprites.instances().len();
            self.native_hud_quads = 0..0;
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

    /// Sprite-batch quad ranges that stay put while the camera scrolls with a walk.
    #[must_use]
    pub fn player_quads(&self) -> &[std::ops::Range<usize>] {
        &self.player_quads
    }

    /// Exact overlay partition for frontends which place the minimap in a sidebar.
    #[cfg(feature = "bevy-shell")]
    pub(crate) fn minimap_quad_range(&self) -> std::ops::Range<usize> {
        self.minimap_quads.clone()
    }

    #[cfg(feature = "bevy-shell")]
    pub(crate) fn health_circle_center(&self) -> [f32; 2] {
        let [x, y] = self.view.tile_to_screen(self.own);
        let half = self.view.tile_px() as f32 / 2.;
        [x + half, y + half]
    }

    #[must_use]
    pub const fn sprites(&self) -> &SpriteBatch {
        &self.sprites
    }
}

#[cfg(test)]
mod tests;

#[path = "features/0502-actor-motion/scene.rs"]
mod actor_motion;

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

#[path = "features/0019-world-light/scene.rs"]
pub(crate) mod floor_lighting;
