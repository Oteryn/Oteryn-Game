use crate::input::{TargetKind, Targetable};
use crate::map::World;
use crate::play::{PlayView, Pushed};
use crate::scene::Scene;
use oteryn_renderer::{BatchError, TileCoord};
use oteryn_session::{EntityKind, JoinSnapshot};
use std::sync::Arc;

impl PlayView {
    pub fn from_join(snapshot: &JoinSnapshot) -> Result<Self, BatchError> {
        let position = snapshot.world_spatial.actor_position;
        Self::new(
            TileCoord::new(position.x, position.y),
            position.floor,
            &snapshot.world_object_overlay,
        )
    }

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
        next.ground_items = super::super::feature_0004_item_view_move::state::map_targets(map);
        // Incremental windows do not replace same-floor spatial observations.
        if next.floor != map.header.origin.floor {
            next.place(map.header.origin);
        }
        next.map_floor = Some(map.header.origin.floor);
        next.anchor = TileCoord::new(0, 0);
        next.local.set_world(Arc::new(world));
        next.live_map = true;
        super::super::feature_local_minimap::state::lock(&next.minimap).observe(map);
        next.remember_minimap(map);
        next.rebuild()?;
        *self = next;
        Ok(())
    }

    pub(in crate::play) fn rebuild(&mut self) -> Result<(), BatchError> {
        let markers = self.drawn_markers().collect::<Vec<_>>();
        let target = self.scene.target();
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
                item: super::super::feature_0004_item_view_move::state::entity_item(entity),
                entity: matches!(entity.kind, EntityKind::Player | EntityKind::Creature)
                    .then_some(entity.entity),
            })
            .chain(
                self.ground_items
                    .iter()
                    .filter(|target| target.item.is_some_and(|item| item.floor() == self.floor))
                    .copied(),
            )
            .collect::<Vec<_>>();
        self.scene.recenter_with(
            self.local.world(),
            (self.map_floor == Some(self.floor)).then_some(self.anchor),
            self.own,
            self.facing,
            &markers,
            &entities,
        )?;
        self.apply_preference_scale()?;
        self.scene.target_frame = self.preferences().game_window.target_frame;
        self.scene.set_name_visibility(self.preferences().hud_names);
        self.scene.set_name_options(
            self.preferences().hud_marks_other,
            self.preferences()
                .hud_npc_names
                .unwrap_or(self.preferences().hud_names[2]),
        );
        self.scene
            .set_presentations(&self.appearances, self.floor)?;
        self.scene.retain_target(target)?;
        self.light_scene()?;
        self.weather_scene();
        self.project_minimap_memory();
        self.scene.set_minimap(
            self.live_map && self.preferences().panel_visible("minimap"),
            self.minimap.clone(),
            self.floor,
        )?;
        self.project_presentation(std::time::Instant::now())
    }

    /// Applies the latest live map window, own position, visible entities and overlay entries.
    pub fn apply_pushed(&mut self, pushed: &Pushed) -> Result<(), BatchError> {
        if pushed.scope_changed {
            self.party_icons.clear();
            self.party_icons_refresh = None;
            self.minimap
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .quest_marks = Default::default();
            self.inputs.clear();
            self.cursor_spell = None;
            self.in_flight = false;
            self.scene.clear_target()?;
            self.ground_items.clear();
            self.entities.clear();
            self.appearances.clear();
            self.markers.clear();
            self.visuals.clear();
            self.sounds.clear();
            self.combat_line = None;
            self.target_walk_feedback = None;
        }
        if let Some(scope) = &pushed.activity_scope {
            self.apply_activity_scope(scope)?;
        }
        if let Some(identity) = pushed.own_identity {
            self.set_own_identity(identity);
        }
        if let Some(channel) = pushed.served_channel {
            oteryn_client_ui::channel_list::protocol::apply(&mut self.served_channel, channel);
        }
        if let Some(marks) = &pushed.personal_map_marks
            && !self
                .minimap
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .replace_quest_marks(marks.clone())
        {
            return Err(BatchError::InvalidView);
        }
        self.world_clock = pushed.world_clock.or(self.world_clock);
        self.world_weather = pushed.world_weather.or(self.world_weather);
        self.apply_server_messages(&pushed.server_messages);
        if let Some(map) = &pushed.map {
            self.set_live_map(map)?;
        }
        if let Some(position) = pushed.own {
            self.place(position);
        }
        if let Some(entries) = &pushed.appearances {
            self.appearances.clone_from(entries);
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
        self.receive_presentation(&pushed.visuals, std::time::Instant::now());
        if pushed.own_hit_flash {
            self.hit_flash = Some(std::time::Instant::now());
        }
        self.rebuild()
    }
}
