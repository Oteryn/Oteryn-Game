//! Deadline-qualified relocation for source callbacks that gather players onto one tile.
use super::{ComposedFreshAdmission, actor_spell::ChannelSpellStates};
use crate::foundation::{
    ChannelRuntimeV1, ExactActorRef, GameSessionId, MovementLocalPosition, MovementPositionSnapshot,
};

pub(super) struct GroupTransfer {
    members: Vec<(ExactActorRef, GameSessionId, MovementPositionSnapshot)>,
    destination: MovementLocalPosition,
    due_us: u64,
    optional_pvp: Option<bool>,
}

impl GroupTransfer {
    pub(super) fn new(
        members: Vec<(ExactActorRef, GameSessionId, MovementPositionSnapshot)>,
        destination: MovementLocalPosition,
        due_us: u64,
        optional_pvp: Option<bool>,
    ) -> Self {
        Self {
            members,
            destination,
            due_us,
            optional_pvp,
        }
    }

    /// Revalidate every captured movement snapshot and publish the stack together
    /// once the caller's owner clock reaches the requested deadline.
    pub(super) fn project(
        &self,
        owner: &ComposedFreshAdmission<'_, '_, '_>,
        runtime: &mut ChannelRuntimeV1,
        states: &mut ChannelSpellStates,
    ) {
        let now = owner.owner_now().get();
        if now < self.due_us || self.members.is_empty() {
            return;
        }
        let Some(map) = owner.bundle_map() else {
            return;
        };
        let closed = crate::map::floor::BundleDoors(&map.facts);
        let proofs = self
            .members
            .iter()
            .map(|(actor, session, before)| {
                crate::map::floor::prepare_quest_encounter_ejection(
                    runtime,
                    owner.movement_cells(),
                    *actor,
                    *session,
                    *before,
                    self.destination,
                    &closed,
                )
            })
            .collect::<Result<Vec<_>, _>>();
        let Ok(proofs) = proofs else { return };
        let Ok(snapshots) = runtime.commit_quest_relocation(proofs, || true) else {
            return;
        };
        for ((actor, session, before), after) in self.members.iter().copied().zip(snapshots) {
            states.stage_special_movement_step(
                runtime,
                map,
                actor,
                session,
                now,
                Some(before.position()),
                after.position(),
                self.optional_pvp,
            );
            states.stage_underwater_step(
                runtime,
                actor,
                session,
                now,
                super::paced_movement::underwater_drown::underwater(map, before.position()),
                super::paced_movement::underwater_drown::underwater(map, after.position()),
            );
        }
    }
}
