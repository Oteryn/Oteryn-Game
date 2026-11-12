use super::*;
impl BattlePanel {
    pub fn replace_entities(&mut self, entities: &[WorldSpatialEntity], own_floor: Option<i16>) {
        self.rows = entities
            .iter()
            .filter(|e| {
                Some(e.position.floor) == own_floor
                    && matches!(e.kind, EntityKind::Player | EntityKind::Creature)
            })
            .copied()
            .collect();
        self.rows.sort_by_key(|e| e.entity);
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
