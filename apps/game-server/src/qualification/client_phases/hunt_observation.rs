//! A kill needs target removal, earned XP and a fresh, unambiguous corpse.
use oteryn_protocol_oteryn::world_spatial::ActorPosition;
use oteryn_protocol_oteryn::world_spatial_entities::{
    EntityKind, EntityRef, WorldSpatialEntitiesDelta, WorldSpatialEntity,
};
use std::collections::BTreeSet;

#[derive(Debug)]
pub(super) struct HuntObservation {
    target: EntityRef,
    pub(super) last_position: ActorPosition,
    pub(super) removed: bool,
    excluded: BTreeSet<EntityRef>,
}
impl HuntObservation {
    pub(super) fn new(target: WorldSpatialEntity, excluded: BTreeSet<EntityRef>) -> Self {
        Self {
            target: target.entity,
            last_position: target.position,
            removed: false,
            excluded,
        }
    }
    pub(super) fn observe(&mut self, delta: &WorldSpatialEntitiesDelta) {
        if let Some(live) = delta
            .update
            .iter()
            .chain(&delta.enter)
            .find(|entry| entry.entity == self.target)
        {
            self.last_position = live.position;
            self.removed = false;
        }
        if delta.leave.contains(&self.target) {
            self.removed = true;
        }
        if !self.removed {
            self.excluded.extend(
                delta
                    .enter
                    .iter()
                    .filter(|entry| entry.kind == EntityKind::Corpse)
                    .map(|entry| entry.entity),
            );
        }
    }
    pub(super) fn corpse<'a>(
        &self,
        entities: impl Iterator<Item = &'a WorldSpatialEntity>,
        earned_xp: bool,
    ) -> Option<WorldSpatialEntity> {
        if !self.removed || !earned_xp {
            return None;
        }
        let mut nearest = None;
        let mut distance = u32::MAX;
        let mut ambiguous = false;
        for corpse in entities.filter(|entry| {
            entry.kind == EntityKind::Corpse
                && !self.excluded.contains(&entry.entity)
                && entry.position.floor == self.last_position.floor
        }) {
            let next = self
                .last_position
                .x
                .abs_diff(corpse.position.x)
                .max(self.last_position.y.abs_diff(corpse.position.y));
            if next > 1 {
                continue;
            }
            if next < distance {
                nearest = Some(*corpse);
                distance = next;
                ambiguous = false;
            } else if next == distance {
                ambiguous = true;
            }
        }
        if ambiguous { None } else { nearest }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oteryn_protocol_oteryn::{
        world_spatial::StepDirection, world_spatial_entities::EntityDetail,
    };
    fn entity(id: u8, kind: EntityKind, x: i32) -> WorldSpatialEntity {
        let mut identity = [id; 16];
        identity[6] = 0x70;
        identity[8] = 0x80;
        WorldSpatialEntity {
            entity: EntityRef {
                identity,
                generation: u64::from(kind == EntityKind::Creature),
            },
            display_name: None,
            kind,
            position: ActorPosition { x, y: 0, floor: 8 },
            detail: if kind == EntityKind::Creature {
                EntityDetail::Actor {
                    direction: StepDirection::North,
                    appearance_ref: 1,
                    health_percent: 100,
                }
            } else {
                EntityDetail::Object {
                    item_definition_ref: 1,
                    quantity: 1,
                    item_handle: None,
                }
            },
        }
    }
    fn frame(enter: Vec<WorldSpatialEntity>, leave: Vec<EntityRef>) -> WorldSpatialEntitiesDelta {
        WorldSpatialEntitiesDelta {
            content_generation: [1; 32],
            actor_position: ActorPosition {
                x: 0,
                y: 0,
                floor: 8,
            },
            enter,
            update: vec![],
            leave,
        }
    }
    #[test]
    fn qualification_nbroute_rat_corpse_requires_xp_removal_freshness_and_unique_nearest() {
        let rat = entity(1, EntityKind::Creature, 1);
        let old = entity(2, EntityKind::Corpse, 1);
        let early = entity(3, EntityKind::Corpse, 1);
        let fresh = entity(4, EntityKind::Corpse, 2);
        let tie = entity(5, EntityKind::Corpse, 0);
        let mut observed = HuntObservation::new(rat, BTreeSet::from([old.entity]));
        observed.observe(&frame(vec![early], vec![]));
        assert!(observed.corpse([&fresh].into_iter(), true).is_none());
        observed.observe(&frame(vec![fresh], vec![rat.entity]));
        assert!(observed.corpse([&fresh].into_iter(), false).is_none());
        assert_eq!(
            observed.corpse([&old, &early, &fresh].into_iter(), true),
            Some(fresh)
        );
        assert!(observed.corpse([&fresh, &tie].into_iter(), true).is_none());
    }
}
