//! Hand off the castle rite's authorized roster to the atomic movement owner.
use super::*;
use oteryn_gameplay_quests::quest::blood_brothers_castle as rite;

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn step_blood_brothers_castle(
        &self,
        permit: &SpellLanePermit,
        actor: ExactActorRef,
        session: GameSessionId,
        entered: MovementPositionSnapshot,
    ) -> Option<MovementPositionSnapshot> {
        let entered_tile = native(entered.position())?;
        let trigger = Position {
            x: entered_tile.x,
            y: entered_tile.y,
            floor: entered_tile.floor,
        };
        if !rite::SYMBOLS.contains(&trigger) {
            return None;
        }
        let map = self.bundle_map()?;
        let mut members = Vec::with_capacity(4);
        {
            let runtime = self.runtime.lock().await;
            let census = runtime.positioned_melee_census().ok()?;
            for symbol in rite::SYMBOLS {
                let mut occupants = census
                    .iter()
                    .filter(|(_, position, _)| position.position() == local(symbol));
                let &(occupant, snapshot, Some(member)) = occupants.next()? else {
                    return None;
                };
                if occupants.next().is_some() {
                    return None;
                }
                members.push((occupant, snapshot, member));
            }
        }
        let initiator = members
            .iter()
            .position(|member| *member == (actor, entered, session))?;
        let mut players = [None; 4];
        let mut leases = Vec::with_capacity(4);
        for (index, &(_, _, member)) in members.iter().enumerate() {
            let fence = self.current_quest_fence(member).await.ok()??;
            let state = self
                .root
                .read_character_quest_state(self.character, fence.character_id)
                .await
                .ok()?;
            players[index] = Some(rite::Participant {
                mission: state
                    .tracks()
                    .get(rite::MISSION)
                    .copied()
                    .unwrap_or_default(),
                entrance: state
                    .tracks()
                    .get(rite::ENTRANCE)
                    .copied()
                    .unwrap_or_default(),
                has_charged_crystal: false,
            });
            leases.push((fence.character_id, fence.character_lease_generation));
        }
        let admissions = rite::plan(trigger, players)?;
        let optional_pvp = self.special_movement_policy(permit, actor, session).await;
        let mut runtime = self.runtime.lock().await;
        let mut states = self.states.spell_states.try_lock().ok()?;
        let closed = crate::map::floor::BundleDoors(&map.facts);
        let mut proofs = Vec::with_capacity(4);
        for ((&(occupant, snapshot, member), admission), (character, generation)) in
            members.iter().zip(admissions).zip(leases)
        {
            if states.is_dead(occupant)
                || !states.spell_idle(&runtime, occupant, member)
                || !self.states.player_trades.physical(member, None)
                || self.party_member_has_foreign_pending(member, session)
                || !runtime
                    .bound_attacker_lease(occupant, member)
                    .is_ok_and(|lease| {
                        lease.character_id().as_bytes() == character.as_bytes()
                            && lease.generation() == generation
                    })
            {
                return None;
            }
            let proof = crate::map::floor::prepare_quest_callback_teleport(
                &runtime,
                &self.movement_cells,
                occupant,
                member,
                snapshot,
                local(admission.destination),
                super::super::pvp_live::pz_blocked(&states, occupant, member, self.owner_now()),
                &closed,
            )
            .ok()?;
            proofs.push(proof);
        }
        let arrived = runtime
            .commit_source_steps_and_project(proofs, || true)
            .ok()?;
        for (&(occupant, snapshot, member), destination) in members.iter().zip(arrived.iter()) {
            let before = snapshot.position();
            let after = destination.position();
            let now = self.owner_now().get();
            states.stage_special_movement_step(
                &runtime,
                map,
                occupant,
                member,
                now,
                Some(before),
                after,
                optional_pvp,
            );
            states.stage_underwater_step(
                &runtime,
                occupant,
                member,
                now,
                underwater(map, before),
                underwater(map, after),
            );
            if let Some(at) = native(after) {
                map.facts.emit_movement_effect(at, 11);
            }
        }
        arrived.get(initiator).copied()
    }
}
