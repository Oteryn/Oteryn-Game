//! Read actors already selected by the existing interest/visibility owner.
use super::*;
impl SessionVisibility {
    pub(in crate::gameplay_transport) fn appearance_actors(
        &self,
        forge: bool,
    ) -> Vec<WorldSpatialEntity> {
        self.entities
            .values()
            .filter(|e| {
                e.kind == EntityKind::Player
                    || e.kind == EntityKind::Npc
                    || (forge && e.kind == EntityKind::Creature)
            })
            .copied()
            .collect()
    }
}
