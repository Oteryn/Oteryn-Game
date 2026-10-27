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

    /// Current dedication, conviction and revelation effects for stat/spell/combat owners. Like stages, this is recomputed from
    /// the session allocation on every read, so admission and a replaced allocation cannot
    /// retain an old projection.
    pub fn wheel_stat_bonuses(
        &self,
        session: GameSessionId,
        vocation: &str,
        level: u32,
        promoted: bool,
        premium: bool,
    ) -> Option<oteryn_gameplay_progression::wheel_effects::WheelStatBonuses> {
        let allocation = self
            .states
            .wheel_sessions
            .lock()
            .ok()?
            .get(&session)?
            .clone()?;
        project_wheel_runtime_effects(&allocation, vocation, level, promoted, premium)
    }
}

fn project_wheel_runtime_effects(
    allocation: &crate::durability::character_wheel::WheelAllocation,
    vocation: &str,
    level: u32,
    promoted: bool,
    premium: bool,
) -> Option<oteryn_gameplay_progression::wheel_effects::WheelRuntimeEffects> {
    let data = oteryn_gameplay_progression::wheel_gem_data::WheelGemData::embedded().ok()?;
    oteryn_gameplay_progression::wheel_effects::derive_runtime_effects(
        data.wheel(),
        allocation,
        vocation,
        level,
        promoted,
        premium,
    )
}

#[cfg(test)]
#[allow(clippy::expect_used, reason = "qualified embedded dispatch fixture")]
mod tests {
    use super::*;

    #[test]
    fn dispatch_projects_full_slice_conviction_and_revelation_effects() {
        let data = oteryn_gameplay_progression::wheel_gem_data::WheelGemData::embedded()
            .expect("qualified catalogue");
        let mut points = [0; oteryn_gameplay_progression::wheel::WHEEL_SLOTS];
        for (index, slot) in data.wheel()["topology"]
            .as_array()
            .expect("topology")
            .iter()
            .enumerate()
        {
            if slot["domain"] == "green" {
                points[index] = u16::try_from(slot["capacity"].as_u64().expect("capacity"))
                    .expect("bounded capacity");
            }
        }
        let allocation = crate::durability::character_wheel::WheelAllocation {
            wheel_ruleset_revision: Some(data.revision().to_owned()),
            wheel_revision: 1,
            slots: crate::durability::character_wheel::WheelSlots::new(points)
                .expect("bounded slots"),
            current: true,
        };
        let effects = project_wheel_runtime_effects(&allocation, "elite_knight", 2_000, true, true)
            .expect("dispatch projection");
        assert_eq!(effects.revelation_stages.get("gift_of_life"), Some(&3));
        assert!(effects.life_leech_hundredths_percent > 0);
        assert!(!effects.sockets.resonance_stages.is_empty());
    }
}
