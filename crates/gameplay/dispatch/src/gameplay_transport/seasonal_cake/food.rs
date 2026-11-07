//! Closed source Cake food successor; a terrain receipt decides publication.
use super::super::ComposedFreshAdmission;
use super::super::actor_spell::ChannelSpellStates;
use super::super::connection::UseCommand;
use super::super::item_ref_admission::NativeItemWeights;
use crate::durability::character_condition_snapshot::CharacterConditionSnapshot;
use crate::durability::item_craft::{MapHarvestFeedback, MapHarvestRequest, MapHarvestResult};
use crate::durability::item_transfer::CurrentCharacterItemFence;
use crate::durability::seasonal_cake::CakeFoodContext;
use crate::durability::spell_owner_commit::SpellLanePermit;
use crate::foundation::{ChannelRuntimeV1, ExactActorRef, GameSessionId};
use crate::spell::cast::PlayerSpellState;
use oteryn_protocol_oteryn::world_object::UseDisposition;

#[derive(Debug, Clone)]
pub(in crate::gameplay_transport) struct PendingCakeFood {
    pub(in crate::gameplay_transport) actor: ExactActorRef,
    pub(in crate::gameplay_transport) session: GameSessionId,
    fence: CurrentCharacterItemFence,
    request: MapHarvestRequest,
    food: CakeFoodContext,
    before: PlayerSpellState,
    after: PlayerSpellState,
}
fn current(
    runtime: &ChannelRuntimeV1,
    actor: ExactActorRef,
    session: GameSessionId,
    fence: CurrentCharacterItemFence,
) -> bool {
    let binding = runtime.binding();
    fence.runtime_scope == binding.runtime_scope()
        && fence.scope_ownership_generation == binding.scope_generation()
        && runtime.player_control_facts(actor, session).is_ok()
        && runtime
            .bound_attacker_lease(actor, session)
            .is_ok_and(|lease| {
                lease.character_id().as_bytes() == fence.character_id.as_bytes()
                    && lease.generation() == fence.character_lease_generation
            })
}
fn install(
    states: &mut ChannelSpellStates,
    runtime: &ChannelRuntimeV1,
    pending: &PendingCakeFood,
    committed: bool,
) -> bool {
    let state = states.get(runtime, pending.actor, pending.session);
    if committed {
        if state != Some(&pending.after)
            && (state != Some(&pending.before)
                || !states.commit(
                    runtime,
                    pending.actor,
                    pending.session,
                    pending.after.clone(),
                ))
        {
            return false;
        }
    } else if state != Some(&pending.before) {
        return false;
    }
    states
        .pending_cake_food
        .retain(|row| row.actor != pending.actor || row.session != pending.session);
    true
}
impl ChannelSpellStates {
    // Same source-only stage used by the real native owner and its lifecycle test.
    pub(in crate::gameplay_transport) fn prepare_cake_food_successor(
        &mut self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        (command, fence): (UseCommand, CurrentCharacterItemFence),
        request: &MapHarvestRequest,
        native: &crate::content::native_gameplay::NativeGameplayState,
        now_us: u64,
    ) -> Result<Option<PendingCakeFood>, ()> {
        if !current(runtime, actor, command.game_session_id, fence)
            || !runtime
                .player_control_facts(actor, command.game_session_id)
                .is_ok_and(|facts| facts.control_loss.is_none())
            || !self.spell_idle(runtime, actor, command.game_session_id)
            || self.has_pending_spell_commit(actor, command.game_session_id)
            || self.is_dead(actor)
            || native.source_digest() != runtime.content_pin().server_artifact_digest()
        {
            return Err(());
        }
        let operand = request.operand.as_ref().ok_or(())?;
        if !matches!(operand.mutation.before, 12085 | 12086) {
            return Err(());
        }
        let at = runtime.read_actor_position(actor).map_err(|_| ())?;
        let position = at.position();
        let expected = crate::durability::map_item_mint_audit::map_item_spatial_position(
            u16::try_from(position.x).map_err(|_| ())?,
            u16::try_from(position.y).map_err(|_| ())?,
            i8::try_from(-position.floor).map_err(|_| ())?,
        );
        if expected != operand.actor_position {
            return Err(());
        }
        let before = self
            .get(runtime, actor, command.game_session_id)
            .ok_or(())?
            .clone();
        let now_ms = now_us / 1000;
        let Some(after) = before.after_food(240, now_ms) else {
            return Ok(None);
        };
        let snapshot = |state: &PlayerSpellState| -> Result<CharacterConditionSnapshot, ()> {
            Ok(CharacterConditionSnapshot {
                lease_generation: fence.character_lease_generation,
                content_digest: native.source_digest(),
                conditions: state
                    .owned_conditions()
                    .persistent_snapshot(now_ms.saturating_mul(1000))
                    .map_err(|_| ())?,
            })
        };
        let food = CakeFoodContext {
            actor_identity: actor.placement_identity(),
            movement_revision: at.revision(),
            state_revision: before.revision(),
            before: snapshot(&before)?,
            after: snapshot(&after)?,
        };
        if self.pending_cake_food.len() >= self.actors.len() {
            return Err(());
        }
        self.pending_cake_food.try_reserve(1).map_err(|_| ())?;
        let pending = PendingCakeFood {
            actor,
            session: command.game_session_id,
            fence,
            request: request.clone(),
            food: food.clone(),
            before,
            after,
        };
        self.pending_cake_food.push(pending.clone());
        Ok(Some(pending))
    }
}
impl ComposedFreshAdmission<'_, '_, '_> {
    /// Called by source map USE, cadence and actor-end persistence before any snapshot overwrite.
    pub(in crate::gameplay_transport) async fn resolve_pending_cake_food(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> Result<(), ()> {
        let runtime = self.runtime.lock().await;
        let mut states = self.states.spell_states.lock().await;
        let Some(pending) = states
            .pending_cake_food
            .iter()
            .find(|row| row.actor == actor && row.session == session)
            .cloned()
        else {
            return Ok(());
        };
        if !current(&runtime, actor, session, pending.fence) {
            return Err(());
        }
        let committed = self
            .root
            .read_cake_food_receipt(
                self.character,
                pending.fence,
                pending.request.clone(),
                pending.food.clone(),
            )
            .await
            .map_err(|_| ())?;
        if !install(&mut states, &runtime, &pending, committed) {
            return Err(());
        }
        if committed {
            self.states.chat.cake_bite_speech(&runtime, actor, session);
        }
        Ok(())
    }

    pub(in crate::gameplay_transport) async fn resolve_cake_food_for_cadence(&self) {
        let pending: Vec<_> = self
            .states
            .spell_states
            .lock()
            .await
            .pending_cake_food
            .iter()
            .map(|row| (row.actor, row.session))
            .collect();
        for (actor, session) in pending {
            let _ = self.resolve_pending_cake_food(actor, session).await;
        }
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "exact native actor, current resource and physical source share one owner turn"
    )]
    pub(in crate::gameplay_transport) async fn harvest_cake_food_owned(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        fence: CurrentCharacterItemFence,
        request: MapHarvestRequest,
        native: &crate::content::native_gameplay::NativeGameplayState,
        permit: &SpellLanePermit,
    ) -> Result<Option<MapHarvestResult>, ()> {
        let runtime = self.runtime.lock().await;
        let mut states = self.states.spell_states.lock().await;
        let Some(pending) = states.prepare_cake_food_successor(
            &runtime,
            actor,
            (command, fence),
            &request,
            native,
            self.owner_now().get(),
        )?
        else {
            return Ok(Some(MapHarvestResult {
                disposition: UseDisposition::Full,
                fresh: false,
                feedback: None,
            }));
        };
        let food = pending.food.clone();
        // Native guards retain the complete successor through COMMIT and its serialized receipt probe.
        let result = self
            .root
            .harvest_cake_food(
                permit,
                self.character,
                self.holder,
                fence,
                request.clone(),
                native,
                &NativeItemWeights(native),
                food.clone(),
            )
            .await;
        let committed = self
            .root
            .read_cake_food_receipt(self.character, fence, request, food)
            .await
            .map_err(|_| ())?;
        if !install(&mut states, &runtime, &pending, committed) {
            return Err(());
        }
        if committed {
            // On an uncertain acknowledgement, the exact food+terrain receipt supplies the answer.
            Ok(Some(MapHarvestResult {
                disposition: UseDisposition::Committed,
                fresh: true,
                feedback: Some(MapHarvestFeedback::CakeConsumed),
            }))
        } else {
            result.map_err(|_| ())
        }
    }
}
