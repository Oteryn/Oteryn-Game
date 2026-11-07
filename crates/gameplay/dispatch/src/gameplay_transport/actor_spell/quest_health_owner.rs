//! Retain a quest's source health successor until its durable command is classified.
use super::*;
use crate::durability::item_transfer::CurrentCharacterItemFence;
use crate::durability::quest_state::{QuestCause, QuestTransitionRequest};

#[derive(Debug)]
pub(super) struct PendingQuestHealth {
    pub actor: ExactActorRef,
    pub session: GameSessionId,
    fence: CurrentCharacterItemFence,
    request: QuestTransitionRequest,
    amount: u32,
    before_revision: u64,
    after: PlayerSpellState,
    death: Option<PlayerDeath>,
}

impl ChannelSpellStates {
    pub(in crate::gameplay_transport) fn prepare_quest_health(
        &mut self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        fence: CurrentCharacterItemFence,
        request: &QuestTransitionRequest,
        amount: u32,
        now_us: u64,
    ) -> Result<(), ()> {
        let QuestCause::Command(command) = request.cause else {
            return Err(());
        };
        let session = command.game_session_id();
        if fence.game_session_id != session || amount == 0 {
            return Err(());
        }
        if let Some(memo) = self
            .pending_quest_health
            .iter()
            .find(|memo| memo.actor == actor && memo.request.cause == request.cause)
        {
            return if memo.fence == fence && memo.request == *request && memo.amount == amount {
                Ok(())
            } else {
                Err(())
            };
        }
        if self.is_dead(actor) || !self.spell_idle(runtime, actor, session) {
            return Err(());
        }
        let current = self.get(runtime, actor, session).ok_or(())?;
        let before_revision = current.revision();
        let after = crate::spell::actor_conditions::stage_item_health_loss(current, amount, now_us)
            .map_err(|_| ())?
            .0;
        let death = if after.health == 0 {
            self.deaths.try_reserve(1).map_err(|_| ())?;
            Some(PlayerDeath {
                occurrence: (self.mint_death)().ok_or(())?,
                cell: runtime
                    .read_actor_position(actor)
                    .map_err(|_| ())?
                    .position(),
            })
        } else {
            None
        };
        self.pending_quest_health.try_reserve(1).map_err(|_| ())?;
        self.pending_quest_health.push(PendingQuestHealth {
            actor,
            session,
            fence,
            request: request.clone(),
            amount,
            before_revision,
            after,
            death,
        });
        Ok(())
    }

    pub(in crate::gameplay_transport) fn settle_quest_health(
        &mut self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        request: &QuestTransitionRequest,
        committed: bool,
    ) -> Result<(), ()> {
        let Some(index) = self
            .pending_quest_health
            .iter()
            .position(|memo| memo.actor == actor && memo.request.cause == request.cause)
        else {
            // Classified replays without original owner work cannot apply damage again.
            return Ok(());
        };
        let memo = &self.pending_quest_health[index];
        if memo.request != *request {
            return Err(());
        }
        if !committed {
            self.pending_quest_health.remove(index);
            return Ok(());
        }
        if runtime.owner_fence().is_err()
            || runtime.binding().runtime_scope() != memo.fence.runtime_scope
            || runtime.binding().scope_generation() != memo.fence.scope_ownership_generation
            || !runtime
                .bound_attacker_lease(actor, memo.session)
                .is_ok_and(|lease| {
                    lease.character_id().as_bytes() == memo.fence.character_id.as_bytes()
                        && lease.generation() == memo.fence.character_lease_generation
                })
            || self
                .get(runtime, actor, memo.session)
                .is_none_or(|current| current.revision() != memo.before_revision)
            || self.is_dead(actor)
        {
            return Err(());
        }
        let memo = self.pending_quest_health.remove(index);
        assert!(self.commit(runtime, actor, memo.session, memo.after));
        if let Some(death) = memo.death {
            self.deaths.push((actor, memo.session, death));
        }
        self.owner_wake.notify_one();
        Ok(())
    }
}
