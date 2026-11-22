use super::*;
impl BattlePanel {
    pub fn replace_entities(&mut self, entities: &[WorldSpatialEntity], own_floor: Option<i16>) {
        let mut visible: std::collections::BTreeMap<_, _> = entities
            .iter()
            .filter(|e| {
                Some(e.position.floor) == own_floor
                    && matches!(
                        e.kind,
                        EntityKind::Player | EntityKind::Creature | EntityKind::Npc
                    )
            })
            .map(|row| (row.entity, *row))
            .collect();
        self.visible_actors.retain_mut(|row| {
            if let Some(updated) = visible.remove(&row.entity) {
                *row = updated;
                true
            } else {
                false
            }
        });
        self.visible_actors.extend(
            entities
                .iter()
                .filter_map(|row| visible.remove(&row.entity)),
        );
        self.rows = self
            .visible_actors
            .iter()
            .copied()
            .filter(|row| row.kind != EntityKind::Npc)
            .collect();
    }
}

/// The registered read-only metadata command path for battle filters.
pub fn metadata_request(
    state: &mut super::PlayerMetadataState,
    visible: &[oteryn_client_runtime::combat::EntityRef],
    now: std::time::Instant,
) -> Option<oteryn_client_runtime::server_mechanic::ServerMechanicCommand> {
    super::metadata::request(state, visible, now)
}
