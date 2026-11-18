//! Crystal 201189f8 fluids.lua:36-48 and 151-184.
use super::super::group_transfer::GroupTransfer;
use super::*;
use crate::foundation::MovementLocalPosition;
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
        Some(GroupTransfer::new(
            members,
            DESTINATION,
            self.owner_now().get(),
            optional_pvp,
        ))
    }
}
