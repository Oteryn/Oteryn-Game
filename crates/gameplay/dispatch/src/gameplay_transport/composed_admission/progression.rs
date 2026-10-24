use super::*;

impl ComposedFreshAdmission<'_, '_, '_> {
    /// The progression binding of `session`; `None` for an unbound or unknown session. The lock
    /// is released before the caller awaits.
    pub fn progression_binding(
        &self,
        session: GameSessionId,
    ) -> Option<std::sync::Arc<SessionProgressionBinding>> {
        session_progression_binding(&self.states.progression_sessions, session)
    }

    /// ARCH-PROGRESSION-SOURCE-0 §1.5: the progression step of a fresh admission whose first
    /// entry is positioned. A World pin without the section binds nothing; otherwise the session
    /// is bound or logs why not once, and a step whose rounds end undecided leaves the actor
    /// not input-eligible.
    pub(in crate::gameplay_transport) async fn admit_progression_session(
        &self,
        admitted: &mut AdmittedSession,
    ) {
        let Some(content) = self
            .active_generation
            .and_then(|active| active.progression())
        else {
            return;
        };
        let session = admitted.game_session_id;
        // The step holds the request, whose policy carries the whole threshold table by value,
        // across its rounds: on the heap, so the admission future does not grow by it.
        let binding = Box::pin(character_progression_binding::bind_character_progression(
            character_progression_binding::ProgressionDurable {
                root: self.root,
                character: self.character,
                holder: self.holder,
            },
            |profile, ruleset, revision| content.policy_for(profile, ruleset, revision).ok(),
            || self.current_quest_fence(session),
            character_progression_binding::ProgressionRounds {
                attempts: RECONCILE_ATTEMPTS,
                backoff: RECONCILE_BACKOFF,
            },
        ))
        .await;
        match binding {
            character_progression_binding::ProgressionBinding::Bound(policy) => {
                let binding = std::sync::Arc::new(SessionProgressionBinding::from_content(*policy));
                if let Ok(mut sessions) = self.states.progression_sessions.lock() {
                    sessions.insert(session, binding);
                }
            }
            character_progression_binding::ProgressionBinding::Unbound(unbound) => {
                operator_event(&format!("progression_unbound reason={}", unbound.reason()));
                if unbound
                    == character_progression_binding::ProgressionUnbound::InitializationUnavailable
                {
                    admitted.first_entry = FirstEntryOutcome::RefusedUnavailable;
                }
            }
        }
    }
}
