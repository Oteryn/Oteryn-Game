//! Crystal 201189f8 fluids.lua:36-48 and 151-184.
use super::*;
use crate::foundation::{MovementLocalPosition, MovementPositionSnapshot};
use crate::map::{boot::BundleMap, overlay::TilePos};
use oteryn_protocol_oteryn::world_object::WorldObjectTarget;

const BLOOD: u8 = 5;
const GRAVESTONE_AID: u16 = 2023;
const DESTINATION: MovementLocalPosition = MovementLocalPosition {
    x: 32791,
    y: 32332,
    floor: 10,
};

pub(super) fn target_matches(map: &BundleMap, placement_key: u64, fluid: u8) -> bool {
    fluid == BLOOD
        && map
            .facts
            .quest_callback_matches(placement_key, None, Some(GRAVESTONE_AID), None)
}

pub(super) struct GroupTransfer {
    members: Vec<(ExactActorRef, GameSessionId, MovementPositionSnapshot)>,
    destination: MovementLocalPosition,
    due_us: u64,
    optional_pvp: Option<bool>,
}

impl GroupTransfer {
    pub(super) fn project(
        self,
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
        for ((actor, session, before), after) in self.members.into_iter().zip(snapshots) {
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
                super::super::paced_movement::underwater_drown::underwater(map, before.position()),
                super::super::paced_movement::underwater_drown::underwater(map, after.position()),
            );
        }
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn prepare_goshnar_transfer(
        &self,
        permit: &crate::durability::spell_owner_commit::SpellLanePermit,
        actor: ExactActorRef,
        session: GameSessionId,
        target: &WorldObjectTarget,
    ) -> Option<GroupTransfer> {
        let optional_pvp = self.special_movement_policy(permit, actor, session).await;
        let mut runtime = self.runtime.lock().await;
        let at = super::super::bundle_map_position(&mut runtime, self.movement_cells(), actor)?;
        let facts = at.map.facts.pinned();
        let source = super::super::world_map::MapViewSource {
            overlay: &at.map.overlay,
            facts: &facts,
            content_generation: at.content_generation,
            reset_epoch: self.reset_epoch,
        };
        let resolved =
            super::super::world_map::resolve_map_target(&source, target, |_| None::<()>).ok()?;
        let TilePos { x, y, floor } = resolved.pos;
        if !target_matches(at.map, resolved.placement_key, BLOOD)
            || !fluid_map_target_in_range(
                at.at.x,
                at.at.y,
                at.at.floor,
                i32::from(x),
                i32::from(y),
                i16::from(floor),
            )
            || runtime.bound_attacker_lease(actor, session).is_err()
        {
            return None;
        }
        let gathering = MovementLocalPosition {
            x: i32::from(x),
            y: i32::from(y) + 1,
            floor: -i16::from(floor),
        };
        at.map.overlay.base().tile(x, y.checked_add(1)?, floor)?;
        let census = runtime.positioned_melee_census().ok()?;
        let mut occupants = census
            .into_iter()
            .filter(|(_, position, _)| position.position() == gathering)
            .peekable();
        let members = if occupants.peek().is_none() {
            vec![(actor, session, runtime.read_actor_position(actor).ok()?)]
        } else {
            occupants
                .filter_map(|(actor, position, session)| {
                    session.map(|member| (actor, member, position))
                })
                .collect()
        };
        Some(GroupTransfer {
            members,
            destination: DESTINATION,
            due_us: self.owner_now().get(),
            optional_pvp,
        })
    }
}
