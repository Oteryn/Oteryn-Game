use crate::input::{TargetKind, Targetable};
use crate::map::World;
use crate::play::{PlayView, Pushed};
use crate::scene::Scene;
use oteryn_renderer::{BatchError, TileCoord};
use oteryn_session::EntityKind;
use std::sync::Arc;

impl PlayView {
    #[must_use]
    pub const fn scene(&self) -> &Scene {
        &self.scene
    }

    /// Draws `world` with the current own position on the start tile and the current floor on
    /// the decoded map floor.
    pub fn set_world(&mut self, world: Arc<World>) -> Result<(), BatchError> {
        if self.live_map {
            return Ok(());
        }
        self.map_floor = Some(world.center().2);
        self.local.set_world(world);
        self.anchor = TileCoord::new(0, 0);
        self.rebuild()
    }

    /// Installs a complete server window; local placement files never supply its geometry.
    pub fn set_live_map(
        &mut self,
        map: &oteryn_session::world_map::WorldMapViewSnapshot,
    ) -> Result<(), BatchError> {
        let world = World::from_live_map(map)?;
        let mut next = self.clone();
        next.place(map.header.origin);
        next.map_floor = Some(map.header.origin.floor);
        next.anchor = TileCoord::new(0, 0);
        next.local.set_world(Arc::new(world));
        next.live_map = true;
        next.rebuild()?;
        *self = next;
        Ok(())
    }

    pub(in crate::play) fn rebuild(&mut self) -> Result<(), BatchError> {
        let markers = self.drawn_markers().collect::<Vec<_>>();
        let entities = self
            .entities
            .iter()
            .filter(|entity| entity.position.floor == self.floor)
            .map(|entity| Targetable {
                tile: TileCoord::new(entity.position.x, entity.position.y),
                // Only a creature may be attacked; NPCs and the rest are drawn and inspected.
                kind: match entity.kind {
                    EntityKind::Corpse | EntityKind::GroundItem => TargetKind::Object,
                    _ => TargetKind::Entity,
                },
                glyph: true,
                entity: matches!(entity.kind, EntityKind::Creature).then_some(entity.entity),
            })
            .collect::<Vec<_>>();
        self.scene = Scene::centered_with(
            self.local.world(),
            (self.map_floor == Some(self.floor)).then_some(self.anchor),
            self.own,
            self.facing,
            &markers,
            &entities,
        )?;
        Ok(())
    }

    /// Applies the latest live map window, own position, visible entities and overlay entries.
    pub fn apply_pushed(&mut self, pushed: &Pushed) -> Result<(), BatchError> {
        self.apply_server_messages(&pushed.server_messages);
        if let Some(map) = &pushed.map {
            self.set_live_map(map)?;
        }
        if let Some(position) = pushed.own {
            self.place(position);
        }
        if let Some(entities) = &pushed.entities {
            self.entities.clone_from(entities);
        }
        if pushed.markers_reset {
            self.markers.clear();
        }
        for entry in &pushed.markers {
            self.set_marker(entry);
        }
        self.rebuild()
    }
}
