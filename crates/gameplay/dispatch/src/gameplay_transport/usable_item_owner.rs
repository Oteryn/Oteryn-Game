//! Retain a fireworks health successor across cancellation or an unknown COMMIT.
use super::*;
use crate::durability::item_use::novelty_appearance::{ActionReceipt, ActionRequest};
use crate::foundation::CommandRef;
use oteryn_gameplay_items::usable_actions::Action;
use oteryn_protocol_oteryn::world_object::UseDisposition;
#[derive(Debug)]
pub(super) struct PendingAction {
    pub(super) actor: ExactActorRef,
    pub(super) request: ActionRequest,
    item_id: u32,
    before_revision: u64,
    after: Option<PlayerSpellState>,
    death: Option<PlayerDeath>,
    target: Option<ExactActorRef>,
}
impl ChannelSpellStates {
    pub(in crate::gameplay_transport) fn has_pending_usable_action(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> bool {
        self.pending_c4
            .iter()
            .any(|p| p.actor == actor && p.request.command.game_session_id() == session)
    }
    pub(in crate::gameplay_transport) fn c4_request(
        &self,
        actor: ExactActorRef,
        command: CommandRef,
        intent: &[u8],
    ) -> Option<ActionRequest> {
        self.pending_c4
            .iter()
            .find(|p| {
                p.actor == actor && p.request.command == command && p.request.intent == intent
            })
            .map(|p| p.request.clone())
    }
    pub(in crate::gameplay_transport) fn release_c4(
        &mut self,
        actor: ExactActorRef,
        command: CommandRef,
    ) {
        self.pending_c4
            .retain(|p| p.actor != actor || p.request.command != command);
    }
    pub(in crate::gameplay_transport) fn prepare_c4(
        &mut self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        request: &ActionRequest,
        item_id: u32,
        now: u64,
        target: Option<ExactActorRef>,
    ) -> Result<(), UseDisposition> {
        if let Some(p) = self
            .pending_c4
            .iter()
            .find(|p| p.actor == actor && p.request.command == request.command)
        {
            return if p.request.source == request.source && p.request.intent == request.intent {
                Ok(())
            } else {
                Err(UseDisposition::Rejected)
            };
        }
        if self.is_dead(actor)
            || !self.spell_idle(runtime, actor, request.command.game_session_id())
        {
            return Err(UseDisposition::Rejected);
        }
        // oteryn-choice: share ITEMUSE0-RL-01 rather than add a separate novelty limiter.
        self.item_uses.admit_usable(
            runtime,
            actor,
            request.command.game_session_id(),
            request.command.command_id().get(),
            now / 1000,
        )?;
        let current = self
            .get(runtime, actor, request.command.game_session_id())
            .ok_or(UseDisposition::StaleState)?;
        let before_revision = current.revision();
        let after = if item_id == 6576 {
            Some(
                crate::spell::actor_conditions::stage_item_health_loss(current, 10, now)
                    .map_err(|_| UseDisposition::NotPossible)?
                    .0,
            )
        } else {
            None
        };
        let death = if after.as_ref().is_some_and(|p| p.health == 0) {
            self.deaths
                .try_reserve(1)
                .map_err(|_| UseDisposition::NotPossible)?;
            Some(PlayerDeath {
                occurrence: (self.mint_death)().ok_or(UseDisposition::NotPossible)?,
                cell: runtime
                    .read_actor_position(actor)
                    .map_err(|_| UseDisposition::StaleState)?
                    .position(),
            })
        } else {
            None
        };
        self.pending_c4
            .try_reserve(1)
            .map_err(|_| UseDisposition::NotPossible)?;
        self.pending_c4.push(PendingAction {
            actor,
            request: request.clone(),
            item_id,
            before_revision,
            after,
            death,
            target,
        });
        Ok(())
    }
    pub(in crate::gameplay_transport) fn finish_c4(
        &mut self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        request: &ActionRequest,
        receipt: &ActionReceipt,
    ) -> Result<Option<(Action, Option<ExactActorRef>)>, ()> {
        let Some(index) = self
            .pending_c4
            .iter()
            .position(|p| p.actor == actor && p.request.command == request.command)
        else {
            return Ok(None);
        };
        if receipt.disposition != UseDisposition::Committed {
            self.pending_c4.remove(index);
            return Ok(None);
        }
        let p = &self.pending_c4[index];
        let action = receipt.action.as_ref().ok_or(())?;
        if runtime.owner_fence().is_err()
            || p.request.intent != request.intent
            || p.item_id != action.item_id
            || self
                .get(runtime, actor, request.command.game_session_id())
                .is_none_or(|s| p.after.is_some() && s.revision() != p.before_revision)
        {
            return Err(());
        }
        let p = self.pending_c4.remove(index);
        if let Some(after) = p.after
            && !self.commit(runtime, actor, request.command.game_session_id(), after)
        {
            return Err(());
        }
        if let Some(death) = p.death {
            self.deaths
                .push((actor, request.command.game_session_id(), death));
        }
        self.owner_wake.notify_one();
        Ok(Some((action.clone(), p.target)))
    }
}
