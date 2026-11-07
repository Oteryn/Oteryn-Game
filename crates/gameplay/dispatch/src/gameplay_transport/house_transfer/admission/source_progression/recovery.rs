//! Lost handoff acknowledgement reconstructs observations, never source writes.
use crate::domain::{CharacterId, CharacterRevision};
use crate::durability::DurabilityError;
use crate::durability::character_progression::CharacterProgressionError;
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::durability::house_scope_handoff::HouseId;
use crate::foundation::{ChannelRuntimeV1, GameSessionState};
use crate::gameplay_transport::{
    ComposedFreshAdmission, QuestSession, SessionProgressionBinding,
    character_progression_binding::ProgressionUnbound,
    connection::{AdmissionRefusal, AdmittedSession},
    item_fence_of, operator_event,
};
use std::sync::Arc;

impl ComposedFreshAdmission<'_, '_, '_> {
    /// The parent has recovered an actual committed House destination while
    /// holding its existing spell lane. The retired source is never reopened.
    /// Each durable read below is Character-owned and grants no gameplay fence.
    pub(in crate::gameplay_transport::house_transfer::admission) async fn restore_house_progression_views(
        &self,
        admitted: &AdmittedSession,
    ) -> Result<(), AdmissionRefusal> {
        use AdmissionRefusal::Unavailable;
        current_house(self, admitted).await?;
        let character = admitted.item_fence.ok_or(Unavailable)?.character_id;
        let revision = self
            .root
            .read_current_character(self.character, character)
            .await
            .map_err(|_| Unavailable)?
            .revision;
        let binding = Box::pin(read_progression_view(self, character, revision)).await?;
        // These owners already fail their optional projections closed when a
        // bounded load fails. No reset, obligation or Atelier initializer runs.
        let copy = self
            .root
            .read_character_quest_state(self.character, character)
            .await
            .ok();
        let allocation = match &self.wheel_ruleset {
            Some(ruleset) => self
                .root
                .read_character_wheel(self.character, character, Arc::clone(ruleset))
                .await
                .ok(),
            None => None,
        };
        let atelier = self
            .root
            .read_character_atelier(self.character, character)
            .await
            .ok()
            .flatten();
        let account = admitted.controller.ok_or(Unavailable)?.account_id;
        let promoted = self
            .root
            .read_character_promotion(self.character, character)
            .await
            .ok();
        let promotion = if let Some(promoted) = promoted {
            let premium = match self.premium_coordinator {
                Some(coordinator) => coordinator.premium_gate(account).await.premium_current(),
                None => false,
            };
            Some((promoted, premium))
        } else {
            None
        };
        current_house(self, admitted).await?;
        if self
            .root
            .read_current_character(self.character, character)
            .await
            .map_err(|_| Unavailable)?
            .revision
            != revision
        {
            return Err(Unavailable);
        }
        let runtime = self.runtime.lock().await;
        if !house_matches(self, &runtime, admitted) {
            return Err(AdmissionRefusal::Rejected);
        }
        // All are data-only leaf maps. Acquire every lock before changing any.
        let mut progression = self
            .states
            .progression_sessions
            .lock()
            .map_err(|_| Unavailable)?;
        let mut quests = self.states.quest_sessions.lock().map_err(|_| Unavailable)?;
        let mut wheels = self.states.wheel_sessions.lock().map_err(|_| Unavailable)?;
        let mut ateliers = self
            .states
            .atelier_sessions
            .lock()
            .map_err(|_| Unavailable)?;
        let mut premium = self
            .states
            .premium_sessions
            .lock()
            .map_err(|_| Unavailable)?;
        let session = admitted.game_session_id;
        let old = quests.get(&session);
        let version = old.map_or(Some(1), |entry| entry.version.checked_add(1));
        let deferred = binding.is_none() || old.is_some_and(|entry| entry.experience_deferred);
        let retry_at = old.and_then(|entry| entry.retry_at);
        if let Some((promoted, current_premium)) = promotion
            && !premium.restore_promotion_snapshot(account, session, promoted, current_premium)
        {
            return Err(Unavailable);
        }
        match binding {
            Some(binding) => {
                progression.insert(session, binding);
            }
            None => {
                progression.remove(&session);
            }
        }
        quests.insert(
            session,
            QuestSession {
                // Exhaustion keeps quest actions closed without reusing a version.
                copy: version.and(copy),
                version: version.unwrap_or(u64::MAX),
                experience_deferred: deferred,
                // A surviving clock is preserved and the cadence defers House.
                // Cold recovery creates no new House obligation-write timer.
                retry_at,
            },
        );
        wheels.insert(session, allocation);
        ateliers.insert(session, atelier);
        Ok(())
    }
}

/// The exact progression owner's read conditions, without its revision-1
/// initializer. An absent/stale/context-mismatched row binds no policy.
async fn read_progression_view(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    character: CharacterId,
    revision: CharacterRevision,
) -> Result<Option<Arc<SessionProgressionBinding>>, AdmissionRefusal> {
    let Some(content) = owner
        .active_generation
        .and_then(|active| active.progression())
    else {
        return Ok(None);
    };
    let interpretation = owner
        .root
        .read_character_root_interpretation(owner.character, character)
        .await
        .map_err(|_| AdmissionRefusal::Unavailable)?;
    let Some(interpretation) = interpretation else {
        log_unbound(ProgressionUnbound::ContextMismatch);
        return Ok(None);
    };
    let [profile, ruleset, revision_name, _] = interpretation.revisions();
    let Some(policy) = content
        .policy_for(profile, ruleset, revision_name)
        .ok()
        .map(Box::new)
    else {
        log_unbound(ProgressionUnbound::ContextMismatch);
        return Ok(None);
    };
    let state = match owner
        .root
        .read_character_progression(owner.character, character)
        .await
    {
        Ok(Some(state)) => state,
        Ok(None)
        | Err(CharacterProgressionError::Unavailable(DurabilityError::InvalidStoredState)) => {
            log_unbound(ProgressionUnbound::Uninitialized);
            return Ok(None);
        }
        Err(CharacterProgressionError::ProgressionContextMismatch) => {
            log_unbound(ProgressionUnbound::ContextMismatch);
            return Ok(None);
        }
        Err(_) => return Err(AdmissionRefusal::Unavailable),
    };
    if state.context != policy.context
        || state.policy_revision != policy.policy_revision
        || state.reward_revision != policy.reward_revision
    {
        log_unbound(ProgressionUnbound::ContextMismatch);
        return Ok(None);
    }
    if state.character_revision != revision {
        log_unbound(ProgressionUnbound::StaleRevision);
        return Ok(None);
    }
    Ok(Some(Arc::new(SessionProgressionBinding::from_content(
        *policy,
    ))))
}

fn log_unbound(reason: ProgressionUnbound) {
    operator_event(&format!("progression_unbound reason={}", reason.reason()));
}

async fn current_house(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
) -> Result<(), AdmissionRefusal> {
    let fence = admitted.item_fence.ok_or(AdmissionRefusal::Unavailable)?;
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
        || !house_matches(owner, &*owner.runtime.lock().await, admitted)
    {
        return Err(AdmissionRefusal::Rejected);
    }
    Ok(())
}

fn house_matches(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    runtime: &ChannelRuntimeV1,
    admitted: &AdmittedSession,
) -> bool {
    let (Some(actor), Some(fence)) = (admitted.runtime_actor, admitted.item_fence) else {
        return false;
    };
    let Ok(house) = runtime.house_actor_state(actor, admitted.game_session_id) else {
        return false;
    };
    house.scope == fence.runtime_scope
        && house.generation == fence.scope_ownership_generation
        && house.lease.character_id().as_bytes() == fence.character_id.as_bytes()
        && house.lease.generation() == fence.character_lease_generation
        && HouseId::new(owner.world_id, &house.house).and_then(|id| id.runtime_scope().ok())
            == Some(house.scope)
}
