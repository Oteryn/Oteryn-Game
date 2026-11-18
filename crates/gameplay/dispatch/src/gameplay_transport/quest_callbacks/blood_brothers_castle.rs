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
        let mut fences = Vec::with_capacity(4);
        for &(occupant, snapshot, member) in &members {
            let fence = self.current_quest_fence(member).await.ok()??;
            let command = step_command(occupant, member, snapshot.revision(), self.reset_epoch)?;
            fences.push((fence, command));
        }
        let fences: [(CurrentCharacterGameplayFence, CommandRef); 4] = fences.try_into().ok()?;
        let mut order = [0, 1, 2, 3];
        order.sort_by_key(|&index| fences[index].0.character_id);
        if order
            .windows(2)
            .any(|p| fences[p[0]].0.character_id == fences[p[1]].0.character_id)
        {
            return None;
        }
        let mut slots = Vec::with_capacity(4);
        for index in order {
            slots.push(
                self.revision_sequencer
                    .acquire(fences[index].0.character_id)
                    .await,
            );
        }
        let optional_pvp = self.special_movement_policy(permit, actor, session).await;
        let closed = crate::map::floor::BundleDoors(&map.facts);
        let proofs = {
            let runtime = self.runtime.lock().await;
            let states = self.states.spell_states.try_lock().ok()?;
            let mut proofs = Vec::with_capacity(4);
            for ((&(occupant, snapshot, member), destination), (fence, _)) in
                members.iter().zip(rite::DESTINATIONS).zip(fences)
            {
                if states.is_dead(occupant)
                    || !states.spell_idle(&runtime, occupant, member)
                    || !self.states.player_trades.physical(member, None)
                    || self.party_member_has_foreign_pending(member, session)
                    || !runtime
                        .bound_attacker_lease(occupant, member)
                        .is_ok_and(|lease| {
                            lease.character_id().as_bytes() == fence.character_id.as_bytes()
                                && lease.generation() == fence.character_lease_generation
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
                    local(destination),
                    super::super::pvp_live::pz_blocked(&states, occupant, member, self.owner_now()),
                    &closed,
                )
                .ok()?;
                proofs.push(proof);
            }
            proofs
        };
        let durable = self
            .root
            .commit_blood_brothers_castle(
                self.character,
                self.holder,
                permit,
                trigger,
                fences,
                std::sync::Arc::clone(self.quest_catalogue.as_ref()?),
            )
            .await;
        for slot in &mut slots {
            slot.observe_endgame_progression();
        }
        if !durable.ok()? {
            return None;
        }
        let mut runtime = self.runtime.lock().await;
        let mut states = self.states.spell_states.try_lock().ok()?;
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
