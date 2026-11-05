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
