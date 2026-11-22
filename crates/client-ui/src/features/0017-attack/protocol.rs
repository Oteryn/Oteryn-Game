use super::*;
impl BattlePanel {
    pub fn replace_entities(&mut self, entities: &[WorldSpatialEntity], own_floor: Option<i16>) {
        let mut visible: std::collections::BTreeMap<_, _> = entities
            .iter()
            .filter(|e| {
                Some(e.position.floor) == own_floor
                    && matches!(e.kind, EntityKind::Player | EntityKind::Creature)
            })
            .map(|row| (row.entity, *row))
            .collect();
        self.rows.retain_mut(|row| {
            if let Some(updated) = visible.remove(&row.entity) {
                *row = updated;
                true
            } else {
                false
            }
        });
        self.rows.extend(
            entities
                .iter()
                .filter_map(|row| visible.remove(&row.entity)),
        );
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
