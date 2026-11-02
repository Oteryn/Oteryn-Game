//! A current House may read retained conditions for presentation. Channel writes stay fenced.
use super::*;

impl ChannelRuntimeV1 {
    pub fn house_actor_conditions_at(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        now: u64,
    ) -> Result<&ConditionStore<ExactActorRef>, ConditionOwnerError> {
        self.house_actor_state(actor, session)
            .map_err(ConditionOwnerError::Actor)?;
        let index = usize::try_from(actor.actor_local_id())
            .ok()
            .and_then(|value| value.checked_sub(1))
            .ok_or(ConditionOwnerError::Actor(
                CarrierError::InvalidActorIdentity,
            ))?;
        let current = self.condition_state(index);
        if now < current.at || !current.store.accepts_time(now) {
            return Err(ConditionOwnerError::TimeMismatch);
        }
        Ok(&current.store)
    }
}
