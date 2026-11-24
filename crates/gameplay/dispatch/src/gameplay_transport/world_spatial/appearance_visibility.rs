//! Read actors already selected by the existing interest/visibility owner.
use super::*;
impl SessionVisibility {
    /// A current runtime update may change riding and forecasts on already
    /// visible players without changing their position or appearance binding.
    pub(in crate::gameplay_transport) fn is_mount_motion_only(
        &self,
        next: &Self,
        delta: &WorldSpatialEntitiesDelta,
    ) -> bool {
        self.published == Some((delta.content_generation, delta.actor_position))
            && self.lights == next.lights
            && delta.enter.is_empty()
            && delta.leave.is_empty()
            && !delta.update.is_empty()
            && delta.update.iter().all(|current| {
                self.entities
                    .get(&current.entity.identity)
                    .is_some_and(|prior| {
                        current.kind == EntityKind::Player
                            && current
                                .motion
                                .zip(prior.motion)
                                .is_some_and(|(after, before)| {
                                    after.mount_look_type != before.mount_look_type
                                        && after.translation_revision == before.translation_revision
                                })
                            && *prior
                                == WorldSpatialEntity {
                                    motion: prior.motion,
                                    ..*current
                                }
                    })
            })
    }

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
