//! Fresh House admission initializes Character data under its genuine Channel source.
//! No physical Channel position or House write authority is manufactured here.
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::foundation::{GameSessionId, GameSessionState, RuntimeScopeRefV1};
use crate::gameplay_transport::spell_character_facts::{CastFactsLoad, load_character_cast_facts};
use crate::gameplay_transport::{
    AdmissionRetryBudget, ComposedFreshAdmission, RECONCILE_ATTEMPTS, RECONCILE_BACKOFF,
    SessionProgressionBinding, character_progression_binding,
    connection::{AdmissionRefusal, AdmittedSession},
    item_fence_of, operator_event,
};

#[path = "source_progression/recovery.rs"]
mod recovery;

impl ComposedFreshAdmission<'_, '_, '_> {
    /// The same progression owner as ordinary admission, before any other source
    /// participant advances the root. first_entry carries no authority here.
    pub(super) async fn bind_house_source_progression(
        &self,
        source: &AdmittedSession,
        budget: &AdmissionRetryBudget<'_>,
    ) -> Result<(), AdmissionRefusal> {
        budget.retry_unavailable(async || {
            current_source(self, source).await?;
            let Some(content) = self.active_generation.and_then(|active| active.progression()) else {
                return Ok(());
            };
            let binding = Box::pin(character_progression_binding::bind_character_progression(
                character_progression_binding::ProgressionDurable {
                    root: self.root, character: self.character, holder: self.holder,
                },
                |profile, ruleset, revision| content.policy_for(profile, ruleset, revision).ok(),
                || self.current_quest_fence(source.game_session_id),
                character_progression_binding::ProgressionRounds {
                    attempts: RECONCILE_ATTEMPTS, backoff: RECONCILE_BACKOFF,
                },
            )).await;
            match binding {
                character_progression_binding::ProgressionBinding::Bound(policy) => {
                    let binding = std::sync::Arc::new(SessionProgressionBinding::from_content(*policy));
                    self.states.progression_sessions.lock().map_err(|_| AdmissionRefusal::Unavailable)?
                        .insert(source.game_session_id, binding);
                    Ok(())
                }
                character_progression_binding::ProgressionBinding::Unbound(unbound) => {
                    operator_event(&format!("progression_unbound reason={}", unbound.reason()));
                    if unbound == character_progression_binding::ProgressionUnbound::InitializationUnavailable {
                        Err(AdmissionRefusal::Unavailable)
                    } else {
                        // Preserve ordinary admission's explicit unbound policy.
                        Ok(())
                    }
                }
            }
        }).await
    }

    /// Run after the other genuine source participants. Quest XP/Wheel may
    /// change the root, so spell repair loads fresh facts after those owners.
    pub(super) async fn finish_house_source_quests(
        &self,
        source: &AdmittedSession,
        budget: &AdmissionRetryBudget<'_>,
    ) -> Result<(), AdmissionRefusal> {
        budget
            .retry_unavailable(async || current_source(self, source).await)
            .await?;
        self.admit_quest_session(source).await;
        let character = source
            .item_fence
            .ok_or(AdmissionRefusal::Unavailable)?
            .character_id;
        let loaded = budget
            .retry_unavailable(async || {
                current_source(self, source).await?;
                match load_character_cast_facts(self.root, self.character, character).await {
                    CastFactsLoad::Ready {
                        facts,
                        character_revision,
                    } => Ok(Some((facts, character_revision))),
                    CastFactsLoad::NoVocation { .. } => Ok(None),
                    CastFactsLoad::Unavailable => Err(AdmissionRefusal::Unavailable),
                }
            })
            .await?;
        let Some((facts, revision)) = loaded else {
            return Ok(());
        };
        budget
            .retry_unavailable(async || {
                current_source(self, source).await?;
                if self
                    .reconcile_automatic_spells(
                        character,
                        source.game_session_id,
                        revision,
                        facts.level,
                        facts.vocation,
                    )
                    .await
                {
                    Ok(())
                } else {
                    Err(AdmissionRefusal::Unavailable)
                }
            })
            .await
    }

    /// Called only after the parent proves the real committed destination ack.
    /// These maps contain projection data, never session, lease or write grants.
    /// Lock all owners before moving anything; an unknown lock leaves them intact.
    pub(in crate::gameplay_transport::house_transfer) fn move_house_source_progression(
        &self,
        source: GameSessionId,
        destination: GameSessionId,
    ) -> Result<(), AdmissionRefusal> {
        if source == destination {
            return Err(AdmissionRefusal::Rejected);
        }
        let mut progression = self
            .states
            .progression_sessions
            .lock()
            .map_err(|_| AdmissionRefusal::Unavailable)?;
        let mut quests = self
            .states
            .quest_sessions
            .lock()
            .map_err(|_| AdmissionRefusal::Unavailable)?;
        let mut wheel = self
            .states
            .wheel_sessions
            .lock()
            .map_err(|_| AdmissionRefusal::Unavailable)?;
        let mut atelier = self
            .states
            .atelier_sessions
            .lock()
            .map_err(|_| AdmissionRefusal::Unavailable)?;
        let mut proficiencies = self
            .states
            .proficiencies
            .lock()
            .map_err(|_| AdmissionRefusal::Unavailable)?;
        // An already moved source is idempotent. Never overwrite another
        // destination's independently installed data with an old source copy.
        if (progression.contains_key(&source) && progression.contains_key(&destination))
            || (quests.contains_key(&source) && quests.contains_key(&destination))
            || (wheel.contains_key(&source) && wheel.contains_key(&destination))
            || (atelier.contains_key(&source) && atelier.contains_key(&destination))
            || (proficiencies.contains_key(&source) && proficiencies.contains_key(&destination))
            || proficiencies
                .get(&source)
                .is_some_and(|state| state.checkpoint.is_some())
        {
            return Err(AdmissionRefusal::Unavailable);
        }
        if let Some(data) = progression.remove(&source) {
            progression.insert(destination, data);
        }
        if let Some(data) = quests.remove(&source) {
            quests.insert(destination, data);
        }
        if let Some(data) = wheel.remove(&source) {
            wheel.insert(destination, data);
        }
        if let Some(data) = atelier.remove(&source) {
            atelier.insert(destination, data);
        }
        if let Some(data) = proficiencies.remove(&source) {
            proficiencies.insert(destination, data);
        }
        Ok(())
    }

    /// The recovery caller proves the immutable handoff source and has already
    /// installed the current destination observations. Retire only data left by
    /// a surviving process; the old command namespace has its own owner.
    pub(super) fn cleanup_house_source_progression(
        &self,
        source: GameSessionId,
        destination: GameSessionId,
    ) -> Result<(), AdmissionRefusal> {
        if source == destination {
            return Err(AdmissionRefusal::Rejected);
        }
        let mut progression = self
            .states
            .progression_sessions
            .lock()
            .map_err(|_| AdmissionRefusal::Unavailable)?;
        let mut quests = self
            .states
            .quest_sessions
            .lock()
            .map_err(|_| AdmissionRefusal::Unavailable)?;
        let mut wheel = self
            .states
            .wheel_sessions
            .lock()
            .map_err(|_| AdmissionRefusal::Unavailable)?;
        let mut atelier = self
            .states
            .atelier_sessions
            .lock()
            .map_err(|_| AdmissionRefusal::Unavailable)?;
        let mut proficiencies = self
            .states
            .proficiencies
            .lock()
            .map_err(|_| AdmissionRefusal::Unavailable)?;
        if proficiencies
            .get(&source)
            .is_some_and(|state| state.checkpoint.is_some())
        {
            return Err(AdmissionRefusal::Unavailable);
        }
        progression.remove(&source);
        quests.remove(&source);
        wheel.remove(&source);
        atelier.remove(&source);
        proficiencies.remove(&source);
        Ok(())
    }
}

/// This is an independently current pre-handoff session, not a forged
/// AdmittedSession.first_entry status. Every native writer repeats its own fence.
async fn current_source(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    source: &AdmittedSession,
) -> Result<(), AdmissionRefusal> {
    let fence = source.item_fence.ok_or(AdmissionRefusal::Unavailable)?;
    let controller = source.controller.ok_or(AdmissionRefusal::Unavailable)?;
    if source.world_id != owner.world_id
        || source.channel_id != owner.channel_id
        || fence.game_session_id != source.game_session_id
        || fence.runtime_scope != RuntimeScopeRefV1::channel(owner.world_id, owner.channel_id)
    {
        return Err(AdmissionRefusal::Rejected);
    }
    let (current, _) = FreshAdmissionStore::from_root(owner.root.clone())
        .current_session_at(source.game_session_id)
        .await
        .map_err(|_| AdmissionRefusal::Unavailable)?;
    if current.session_state() != GameSessionState::Active
        || current.current_transport() != Some(controller.transport)
        || item_fence_of(current) != Some(fence)
    {
        return Err(AdmissionRefusal::Rejected);
    }
    Ok(())
}
