//! Settle a qualified passage and its exact recoverable arrival together.
use super::{ComposedFreshAdmission, paced_movement::underwater_drown::underwater};
use crate::durability::{
    character_last_position::LastVitals, item_transfer::CurrentCharacterItemFence,
    spell_owner_commit::SpellLanePermit, world_event::changes::variants::PassageStep,
};
use crate::foundation::{
    ExactActorRef, GameSessionId, MovementLocalPosition, MovementPositionSnapshot,
};
use sha2::{Digest, Sha256};

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn daily_variant_step(
        &self,
        permit: &SpellLanePermit,
        actor: ExactActorRef,
        session: GameSessionId,
        origin: MovementPositionSnapshot,
        entered: MovementPositionSnapshot,
    ) -> MovementPositionSnapshot {
        let Some(map) = self.bundle_map() else {
            return entered;
        };
        let p = entered.position();
        let (Ok(x), Ok(y), Ok(z)) = (
            u16::try_from(p.x),
            u16::try_from(p.y),
            i8::try_from(p.floor),
        ) else {
            return entered;
        };
        let position = (x, y, z);
        if map.overlay.base().layer_epoch() != Some(self.reset_epoch)
            || !map.facts.daily_variant_passage(position)
        {
            return entered;
        }
        let Some(fence) = self
            .current_quest_fence(session)
            .await
            .ok()
            .flatten()
            .map(|f| f.item_fence())
        else {
            return entered;
        };
        let Some(command) = super::quest_callbacks::step_command(
            actor,
            session,
            entered.revision(),
            self.reset_epoch,
        ) else {
            return entered;
        };
        let day = self
            .root
            .world_changes(self.world_id, self.reset_epoch)
            .await
            .ok()
            .flatten();
        let destination = day
            .as_ref()
            .and_then(|day| day.variants.passage(position))
            .map(|(_, portal)| portal.destination);
        let optional_pvp = self.special_movement_policy(permit, actor, session).await;
        let mut runtime = self.runtime.lock().await;
        let Ok(mut states) = self.states.spell_states.try_lock() else {
            return entered;
        };
        if !current(&runtime, actor, entered, fence)
            || states.is_dead(actor)
            || !states.spell_idle(&runtime, actor, session)
        {
            return entered;
        }
        let now = self.owner_now();
        let blocked = super::pvp_live::pz_blocked(&states, actor, session, now);
        let closed = crate::map::floor::BundleDoors(map.facts.doors());
        let mut arrival = None;
        if let Some((to_x, to_y, to_z)) = destination {
            let target = MovementLocalPosition {
                x: i32::from(to_x),
                y: i32::from(to_y),
                floor: i16::from(to_z),
            };
            if let Ok(proof) = crate::map::floor::prepare_quest_callback_teleport(
                &runtime,
                &self.movement_cells,
                actor,
                session,
                entered,
                target,
                blocked,
                &closed,
            ) {
                let mut saved = self.placed_record(&runtime, target);
                if let Some(player) = states.get(&runtime, actor, session) {
                    saved.record.vitals = player
                        .saved_vitals()
                        .and_then(|(health, mana)| LastVitals::new(health, mana));
                    saved.record.rest = Some(player.bed_rest_snapshot(now.get()));
                }
                if let Ok(staged) = runtime.stage_source_step(proof) {
                    let step = PassageStep {
                        epoch: self.reset_epoch,
                        position,
                        command,
                        binding: Sha256::new()
                            .chain_update(b"oteryn.daily-variant-step.v2")
                            .chain_update(actor.placement_identity())
                            .chain_update(entered.revision().to_be_bytes())
                            .chain_update(staged.snapshot().revision().to_be_bytes())
                            .chain_update(x.to_be_bytes())
                            .chain_update(y.to_be_bytes())
                            .chain_update(z.to_be_bytes())
                            .chain_update(to_x.to_be_bytes())
                            .chain_update(to_y.to_be_bytes())
                            .chain_update(to_z.to_be_bytes())
                            .finalize()
                            .into(),
                        arrival: saved,
                    };
                    // The exclusive runtime turn reserves the landing through
                    // settlement. Retry the same receipt after lost SQL acknowledgement.
                    let settle = || {
                        self.root
                            .daily_variant_passage(self.character, self.holder, fence, step)
                    };
                    let result = match settle().await {
                        Err(_) => settle().await,
                        known => known,
                    };
                    match result {
                        Ok(Some(cell)) if Some(cell) == destination => {
                            arrival = Some(staged.publish())
                        }
                        Err(_) => return entered,
                        _ => {}
                    }
                }
            }
        }
        let snapshot = if let Some(snapshot) = arrival {
            snapshot
        } else {
            let Ok(proof) = crate::map::floor::prepare_quest_callback_teleport(
                &runtime,
                &self.movement_cells,
                actor,
                session,
                entered,
                origin.position(),
                blocked,
                &closed,
            ) else {
                return entered;
            };
            let Ok(snapshot) = runtime.commit_source_step(proof) else {
                return entered;
            };
            snapshot
        };
        states.stage_special_movement_step(
            &runtime,
            map,
            actor,
            session,
            now.get(),
            Some(entered.position()),
            snapshot.position(),
            optional_pvp,
        );
        states.stage_underwater_step(
            &runtime,
            actor,
            session,
            now.get(),
            underwater(map, entered.position()),
            underwater(map, snapshot.position()),
        );
        snapshot
    }
}

fn current(
    runtime: &crate::foundation::ChannelRuntimeV1,
    actor: ExactActorRef,
    position: MovementPositionSnapshot,
    fence: CurrentCharacterItemFence,
) -> bool {
    runtime.owner_fence().is_ok()
        && runtime.read_actor_position(actor) == Ok(position)
        && runtime.binding().runtime_scope() == fence.runtime_scope
        && runtime.binding().scope_generation() == fence.scope_ownership_generation
        && runtime
            .bound_attacker_lease(actor, fence.game_session_id)
            .is_ok_and(|lease| {
                lease.character_id().as_bytes() == fence.character_id.as_bytes()
                    && lease.generation() == fence.character_lease_generation
            })
}
