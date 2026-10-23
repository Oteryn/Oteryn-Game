//! Quest admission/refresh orchestration; existing durability owners serialize every write.

use super::{
    ComposedFreshAdmission, FreshAdmissionStore, GameSessionId, QUEST_OBLIGATION_RETRY, domain,
    operator_event,
};

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
    }

    /// The gameplay fence of `session` from current durable reads; its expected revision is
    /// replaced by the revision slot's cursor at commit. `Ok(None)` when the session is proven
    /// terminal; `Err` when a read failed, which proves nothing.
    pub(super) async fn current_quest_fence(
        &self,
        session: GameSessionId,
    ) -> Result<Option<crate::durability::character_progression::CurrentCharacterGameplayFence>, ()>
    {
        let store = FreshAdmissionStore::from_root(self.root.clone());
        let (current, _) = store.current_session_at(session).await.map_err(|_| ())?;
        if current.session_state() == crate::foundation::GameSessionState::Terminal {
            return Ok(None);
        }
        let character_id =
            domain::CharacterId::from_bytes(*current.commit().character_id().as_bytes())
                .map_err(|_| ())?;
        let record = self
            .root
            .read_current_character(self.character, character_id)
            .await
            .map_err(|_| ())?;
        Ok(Some(
            crate::durability::character_progression::CurrentCharacterGameplayFence {
                character_id,
                game_session_id: session,
                connection_generation: current.current_connection_generation(),
                character_lease_generation: current.current_character_lease().generation(),
                runtime_scope: current.current_runtime_scope(),
                scope_ownership_generation: current.current_scope_generation(),
                expected_character_revision: record.revision,
            },
        ))
    }
}

#[cfg(test)]
#[path = "quest_admission_pause_tests.rs"]
pub(super) mod test_pause;
