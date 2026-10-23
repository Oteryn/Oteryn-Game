//! Read only players already selected by the existing interest/visibility owner.
use super::*;
impl SessionVisibility {
    pub(in crate::gameplay_transport) fn appearance_players(&self) -> Vec<WorldSpatialEntity> {
        self.entities
            .values()
            .filter(|e| e.kind == EntityKind::Player)
            .copied()
            .collect()
    }
}
