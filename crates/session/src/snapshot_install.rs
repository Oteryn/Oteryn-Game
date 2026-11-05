//! Staging and installing validated replacement domain state.
use super::*;
use snapshot_state::SnapshotState;

impl SnapshotState {
    pub(super) fn capture<S>(session: &Session<S>) -> Self {
        Self {
            world_spatial_observation: Some(session.world_spatial),
            entities: session.entities.clone(),
            world_object_overlay: Some(session.world_object_overlay.clone()),
            actor_vitals: session.actor_vitals,
            combat_state: session.combat_state,
            combat_revision: session.combat_revision,
            spatial_revision: session.spatial_revision,
            overlay_revision: session.overlay_revision,
            vitals_revision: session.vitals_revision,
            chat: session.chat.clone(),
            chat_revision: session.chat_revision,
            inventory: session.inventory.clone(),
            container_views: session.container_views.clone(),
            open_container: session.open_container.clone(),
            inventory_revision: session.inventory_revision,
            container_revision: session.container_revision,
            gated_snapshots: session.gated_snapshots.clone(),
            gated_revisions: session.gated_revisions.clone(),
            appearance: session.appearance.clone(),
            map_header: session.map_header,
            events: Vec::new(),
        }
    }
    pub(super) fn install<S>(self, session: &mut Session<S>) {
        if let Some(value) = self.world_spatial_observation {
            session.world_spatial = value;
        }
        session.entities = self.entities;
        if let Some(value) = self.world_object_overlay {
            session.world_object_overlay = value;
        }
        session.actor_vitals = self.actor_vitals;
        session.combat_state = self.combat_state;
        session.combat_revision = self.combat_revision;
        session.spatial_revision = self.spatial_revision;
        session.overlay_revision = self.overlay_revision;
        session.vitals_revision = self.vitals_revision;
        session.chat = self.chat;
        session.chat_revision = self.chat_revision;
        session.inventory = self.inventory;
        session.container_views = self.container_views;
        session.open_container = self.open_container;
        session.inventory_revision = self.inventory_revision;
        session.container_revision = self.container_revision;
        session.gated_snapshots = self.gated_snapshots;
        session.gated_revisions = self.gated_revisions;
        session.appearance = self.appearance;
        session.map_header = self.map_header;
    }
}
