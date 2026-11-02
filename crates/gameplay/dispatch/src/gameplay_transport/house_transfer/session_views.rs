//! Existing session projection owners follow a genuine committed House handoff.
//! These are data and account pull registrations, never session/lease grants.
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::durability::house_scope_handoff::HouseId;
use crate::foundation::{GameSessionId, GameSessionState, RuntimeScopeRefV1};
use crate::gameplay_transport::{
    ComposedFreshAdmission, QUEST_OBLIGATION_RETRY,
    connection::{AdmissionRefusal, AdmittedSession},
    item_fence_of,
};

impl ComposedFreshAdmission<'_, '_, '_> {
    /// Run before COMMIT while the parent holds the current runtime lock.
    /// Proficiency's native checkpoint creator takes that same lock, so an
    /// unresolved original fence cannot appear between this gate and handoff.
    pub(super) fn house_session_views_ready(
        &self,
        source: GameSessionId,
        destination: GameSessionId,
    ) -> bool {
        source != destination
            && self.states.proficiencies.lock().is_ok_and(|states| {
                !states.contains_key(&destination)
                    && !states
                        .get(&source)
                        .is_some_and(|state| state.checkpoint.is_some())
            })
            && self.states.chat.house_transfer_ready(source, destination)
            && self
                .states
                .npc_conversations
                .house_transfer_ready(source, destination)
    }

    /// The native writer and carrier have already committed this exact edge.
    /// The source descriptor is historical input, not renewed authority.
    pub(super) async fn carry_house_session_views(
        &self,
        source: &AdmittedSession,
        destination: &AdmittedSession,
    ) -> Result<(), AdmissionRefusal> {
        let before = source.item_fence.ok_or(AdmissionRefusal::Unavailable)?;
        let after = destination
            .item_fence
            .ok_or(AdmissionRefusal::Unavailable)?;
        let source_controller = source.controller.ok_or(AdmissionRefusal::Unavailable)?;
        let destination_controller = destination
            .controller
            .ok_or(AdmissionRefusal::Unavailable)?;
        let channel = RuntimeScopeRefV1::channel(self.world_id, self.channel_id);
        if source.runtime_actor != destination.runtime_actor
            || source.world_id != destination.world_id
            || source.channel_id != destination.channel_id
            || before.game_session_id != source.game_session_id
            || before.character_id != after.character_id
            || before.character_lease_generation.checked_add(1)
                != Some(after.character_lease_generation)
            || source_controller.account_id != destination_controller.account_id
            || source_controller.transport != destination_controller.transport
            || !matches!(
                (
                    before.runtime_scope == channel,
                    after.runtime_scope == channel
                ),
                (true, false) | (false, true)
            )
        {
            return Err(AdmissionRefusal::Rejected);
        }
        self.carry_recovered_house_session_views(source.game_session_id, destination)
            .await
    }

    /// The caller proves this source ID from the immutable committed receipt.
    /// It deliberately does not construct an AdmittedSession for a terminal source.
    pub(super) async fn carry_recovered_house_session_views(
        &self,
        source: GameSessionId,
        destination: &AdmittedSession,
    ) -> Result<(), AdmissionRefusal> {
        if source == destination.game_session_id {
            return Err(AdmissionRefusal::Rejected);
        }
        let in_house = current_destination(self, destination).await?;
        self.move_house_source_progression(source, destination.game_session_id)?;
        let account = destination
            .controller
            .ok_or(AdmissionRefusal::Unavailable)?
            .account_id;
        {
            let mut registrations = self
                .states
                .premium_sessions
                .lock()
                .map_err(|_| AdmissionRefusal::Unavailable)?;
            let registered = registrations.account_of(source) == Some(account)
                || registrations.account_of(destination.game_session_id) == Some(account);
            if !registrations.transfer_session(account, source, destination.game_session_id) {
                return Err(AdmissionRefusal::Rejected);
            }
            if !registered {
                // A cold recovery uses the original account puller, under the
                // same registration lock as ordinary admission and release.
                self.premium.admit(account);
            }
        }
        self.states
            .resting_areas
            .transfer_session(
                source,
                destination.game_session_id,
                destination
                    .runtime_actor
                    .ok_or(AdmissionRefusal::Unavailable)?,
            )
            .map_err(|_| AdmissionRefusal::Unavailable)?;
        let character = *destination
            .item_fence
            .ok_or(AdmissionRefusal::Unavailable)?
            .character_id
            .as_bytes();
        if !self
            .states
            .chat
            .transfer_house_session(source, destination.game_session_id, character)
            || !self
                .states
                .npc_conversations
                .transfer_house_session(source, destination.game_session_id)
        {
            return Err(AdmissionRefusal::Unavailable);
        }
        if !in_house {
            self.rearm_channel_quest_observations(destination.game_session_id)?;
        }
        Ok(())
    }

    /// A surviving deadline is kept exactly. A cold read-only House recovery
    /// has no process clock to restore; only the next genuine Channel session
    /// may schedule the existing obligation owner for its still-pending data.
    fn rearm_channel_quest_observations(
        &self,
        session: GameSessionId,
    ) -> Result<(), AdmissionRefusal> {
        let bound = self
            .states
            .progression_sessions
            .lock()
            .map_err(|_| AdmissionRefusal::Unavailable)?
            .contains_key(&session);
        let mut quests = self
            .states
            .quest_sessions
            .lock()
            .map_err(|_| AdmissionRefusal::Unavailable)?;
        if let Some(entry) = quests.get_mut(&session)
            && entry.retry_at.is_none()
            && entry.copy.as_ref().is_none_or(|copy| {
                copy.pending_obligations().next().is_some()
                    || (bound && !copy.xp_obligations().is_empty())
            })
        {
            entry.retry_at = Some(std::time::Instant::now() + QUEST_OBLIGATION_RETRY);
        }
        Ok(())
    }
}

async fn current_destination(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
) -> Result<bool, AdmissionRefusal> {
    let fence = admitted.item_fence.ok_or(AdmissionRefusal::Unavailable)?;
    let actor = admitted
        .runtime_actor
        .ok_or(AdmissionRefusal::Unavailable)?;
    let controller = admitted.controller.ok_or(AdmissionRefusal::Unavailable)?;
    let (current, _) = FreshAdmissionStore::from_root(owner.root.clone())
        .current_session_at(admitted.game_session_id)
        .await
        .map_err(|_| AdmissionRefusal::Unavailable)?;
    if admitted.world_id != owner.world_id
        || admitted.channel_id != owner.channel_id
        || fence.game_session_id != admitted.game_session_id
        || current.session_state() != GameSessionState::Active
        || current.current_transport() != Some(controller.transport)
        || item_fence_of(current) != Some(fence)
    {
        return Err(AdmissionRefusal::Rejected);
    }
    let runtime = owner.runtime.lock().await;
    match fence.runtime_scope {
        RuntimeScopeRefV1::Channel { .. } => {
            let lease = runtime
                .bound_attacker_lease(actor, admitted.game_session_id)
                .map_err(|_| AdmissionRefusal::Rejected)?;
            if fence.runtime_scope != RuntimeScopeRefV1::channel(owner.world_id, owner.channel_id)
                || fence.scope_ownership_generation != runtime.binding().scope_generation()
                || lease.character_id().as_bytes() != fence.character_id.as_bytes()
                || lease.generation() != fence.character_lease_generation
                || runtime.read_actor_position(actor).is_err()
            {
                return Err(AdmissionRefusal::Rejected);
            }
            Ok(false)
        }
        RuntimeScopeRefV1::Instance { .. } => {
            let house = runtime
                .house_actor_state(actor, admitted.game_session_id)
                .map_err(|_| AdmissionRefusal::Rejected)?;
            if house.scope != fence.runtime_scope
                || house.generation != fence.scope_ownership_generation
                || house.lease.character_id().as_bytes() != fence.character_id.as_bytes()
                || house.lease.generation() != fence.character_lease_generation
                || HouseId::new(owner.world_id, &house.house).and_then(|id| id.runtime_scope().ok())
                    != Some(house.scope)
            {
                return Err(AdmissionRefusal::Rejected);
            }
            Ok(true)
        }
    }
}
