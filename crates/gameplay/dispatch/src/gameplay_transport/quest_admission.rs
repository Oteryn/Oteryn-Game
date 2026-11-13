//! Quest admission/refresh orchestration; existing durability owners serialize every write.

use super::{ComposedFreshAdmission, GameSessionId, QUEST_OBLIGATION_RETRY, operator_event};

/// One admitted session's quest state (QUEST-STATE-0 §5.4, §7).
#[derive(Debug)]
pub struct QuestSession {
    /// `None`: the load failed or exceeded a bound; quest actions fail closed.
    pub copy: Option<crate::durability::quest_state::QuestStateCopy>,
    /// Whether this session already reported an absent progression binding.
    pub experience_deferred: bool,
    /// Monotonic copy revision, consumed by the journal connection.
    pub version: u64,
    /// When the pending obligations may be requested again after a failed attempt.
    pub retry_at: Option<std::time::Instant>,
}

impl ComposedFreshAdmission<'_, '_, '_> {
    /// Reload the quest copy of `session` from the store and request its pending obligations
    /// again. A failed load or an attempt without an outcome is tried again after
    /// [`QUEST_OBLIGATION_RETRY`] on the owner cadence. `insert` is false for a refresh of a
    /// session that may have ended meanwhile: it then updates only a session still present.
    pub(super) async fn refresh_quest_session(&self, session: GameSessionId, insert: bool) {
        let mut deferred = false;
        let fence = self.current_quest_fence(session).await;
        #[cfg(test)]
        test_pause::after_fence().await;
        let (copy, retry) = match fence {
            Ok(Some(fence)) => {
                let mut admission = crate::durability::quest_state::admit_character_quest_state(
                    &self.revision_sequencer,
                    self.root,
                    self.character,
                    self.holder,
                    fence,
                    self.quest_catalogue.as_ref(),
                )
                .await;
                let mut retry = admission.retry || admission.copy.is_none();
                if let Some(copy) = admission.copy.as_mut() {
                    if let Some(binding) = self.progression_binding(session) {
                        let attempted_experience = !copy.xp_obligations().is_empty();
                        retry |= Box::pin(
                            crate::durability::quest_state::request_pending_quest_experience(
                                &self.revision_sequencer,
                                self.root,
                                self.character,
                                self.holder,
                                fence,
                                &binding.policy,
                                copy,
                            ),
                        )
                        .await;
                        if attempted_experience {
                            // The XP owner may have committed before an acknowledgement
                            // was lost. LEVELUP-LIVE-1 reads committed facts and retries;
                            // the quest path never invents live levels or vital maxima.
                            self.states.level_refresh.mark([fence.character_id]);
                        }
                    } else {
                        deferred = true;
                    }
                }
                (Some(admission.copy), retry)
            }
            // A proven terminal session: no quest action and nothing to retry.
            Ok(None) => (Some(None), false),
            // A failed read proves nothing: keep the current copy and try again.
            Err(()) => (None, true),
        };
        let retry_at = retry.then(|| std::time::Instant::now() + QUEST_OBLIGATION_RETRY);
        let mut log_deferred = false;
        if let Ok(mut sessions) = self.states.quest_sessions.lock()
            && (insert || sessions.contains_key(&session))
        {
            let entry = sessions.entry(session).or_insert(QuestSession {
                copy: None,
                retry_at: None,
                experience_deferred: false,
                version: 0,
            });
            if let Some(copy) = copy {
                // Exhaustion fails this session closed; never reuse an observed version.
                entry.copy = entry.version.checked_add(1).and_then(|version| {
                    entry.version = version;
                    copy
                });
            }
            entry.retry_at = retry_at;
            if deferred && !entry.experience_deferred {
                entry.experience_deferred = true;
                log_deferred = true;
            }
        }
        if log_deferred {
            operator_event("quest_experience_deferred reason=no_progression_binding");
        }
        self.refresh_grizzly_kill_snapshot(session).await;
        let _ = self.bounty_talisman_for_session(session).await;
        self.refresh_native_objective_snapshot(session).await;
        let disguise_retry = self.refresh_quest_disguise(session).await;
        if (self.refresh_quest_guardians(session).await || disguise_retry)
            && let Ok(mut sessions) = self.states.quest_sessions.lock()
            && let Some(entry) = sessions.get_mut(&session)
        {
            let retry = std::time::Instant::now() + std::time::Duration::from_secs(5);
            entry.retry_at = Some(entry.retry_at.map_or(retry, |existing| existing.min(retry)));
        }
    }

    /// [`Self::current_quest_fence`] for an admission step: a failed read (such as a busy
    /// holder) is retried under `budget` (NODEBOOT-1).
    pub(super) async fn current_quest_fence_within(
        &self,
        budget: &super::AdmissionRetryBudget<'_>,
        session: GameSessionId,
    ) -> Result<Option<crate::durability::character_progression::CurrentCharacterGameplayFence>, ()>
    {
        budget
            .retry_unavailable(async || {
                self.current_quest_fence(session)
                    .await
                    .map_err(|()| super::AdmissionRefusal::Unavailable)
            })
            .await
            .map_err(|_| ())
    }

    /// The gameplay fence of `session` from current durable reads; its expected revision is
    /// replaced by the revision slot's cursor at commit. `Ok(None)` when the session is proven
    /// terminal; `Err` when a read failed, which proves nothing.
    pub(super) async fn current_quest_fence(
        &self,
        session: GameSessionId,
    ) -> Result<Option<crate::durability::character_progression::CurrentCharacterGameplayFence>, ()>
    {
        self.current_quest_fence_durable(session)
            .await
            .map_err(|_| ())
    }

    /// Preserve the acquisition error for command callers that retry only a busy holder.
    pub(super) async fn current_quest_fence_durable(
        &self,
        session: GameSessionId,
    ) -> Result<
        Option<crate::durability::character_progression::CurrentCharacterGameplayFence>,
        crate::durability::DurabilityError,
    > {
        use crate::durability::{DurabilityError, character_authority::CharacterAuthorityError};
        self.root
            .read_current_gameplay_fence(self.character, session)
            .await
            .map_err(|error| match error {
                CharacterAuthorityError::Unavailable(error) => error,
                CharacterAuthorityError::Rejected
                | CharacterAuthorityError::Conflict
                | CharacterAuthorityError::NameUnavailable => DurabilityError::CommitRejected,
            })
    }
}

#[cfg(test)]
#[path = "quest_admission_pause_tests.rs"]
pub(super) mod test_pause;
