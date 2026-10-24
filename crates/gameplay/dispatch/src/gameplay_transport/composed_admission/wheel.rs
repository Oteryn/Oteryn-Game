use super::*;

impl ComposedFreshAdmission<'_, '_, '_> {
    /// WHEEL-0 §4 "Wheel reset" and "Load": the admission Wheel reset in the Character's
    /// revision slot, then the session's allocation. A failed load fails the session's Wheel
    /// closed (every stage 0), never the login.
    pub(in crate::gameplay_transport) async fn admit_wheel_session(&self, session: GameSessionId) {
        let allocation = match (self.current_quest_fence(session).await, &self.wheel_ruleset) {
            (Ok(Some(fence)), Some(ruleset)) => {
                crate::durability::character_wheel::admit_character_wheel(
                    &self.revision_sequencer,
                    self.root,
                    self.character,
                    self.holder,
                    fence,
                    ruleset,
                )
                .await
            }
            _ => None,
        };
        if let Ok(mut sessions) = self.states.wheel_sessions.lock() {
            sessions.insert(session, allocation);
        }
    }

    /// The Wheel stages of `session` for one cast snapshot (WHEEL0-EL-1): derived from the cached
    /// allocation and the eligibility facts read at this use. All 0 without an allocation.
    #[allow(
        dead_code,
        reason = "SPELL-WHEEL-GATE-1 passes these stages into the cast snapshot"
    )]
    pub fn wheel_stages(
        &self,
        session: GameSessionId,
        vocation: &str,
        level: u32,
        promoted: bool,
    ) -> crate::durability::character_wheel::WheelStages {
        let Some(ruleset) = &self.wheel_ruleset else {
            return crate::durability::character_wheel::WheelStages::default();
        };
        self.states
            .wheel_sessions
            .lock()
            .ok()
            .and_then(|sessions| sessions.get(&session).cloned().flatten())
            .map(|allocation| {
                crate::durability::character_wheel::WheelStages::derive(
                    ruleset,
                    &allocation,
                    vocation,
                    level,
                    promoted,
                )
            })
            .unwrap_or_default()
    }
}
