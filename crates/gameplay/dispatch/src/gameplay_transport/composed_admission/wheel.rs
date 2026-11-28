use super::*;

impl ComposedFreshAdmission<'_, '_, '_> {
    /// Installs the current Wheel maxima in the live vitals owner. Re-reading current actor
    /// eligibility makes allocation, Premium and level/promotion transitions converge here.
    pub(in crate::gameplay_transport) async fn refresh_wheel_vitals(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> bool {
        self.refresh_wheel_vitals_for_transport(actor, session, None)
            .await
    }

    pub(super) async fn refresh_wheel_vitals_for_transport(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        generation: Option<u64>,
    ) -> bool {
        self.refresh_wheel_vitals_with_context(actor, session, None, generation)
            .await
    }

    pub(in crate::gameplay_transport) async fn refresh_wheel_vitals_with_budget(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        budget: Option<&AdmissionRetryBudget<'_>>,
    ) -> bool {
        self.refresh_wheel_vitals_with_context(actor, session, budget, None)
            .await
    }

    async fn refresh_wheel_vitals_with_context(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        budget: Option<&AdmissionRetryBudget<'_>>,
        generation: Option<u64>,
    ) -> bool {
        // Without a current allocation there are no Wheel effects to qualify. In particular,
        // observing vitals must not take the root's sole holder away from death reconciliation.
        // Still apply zero bonuses below so a removed/stale allocation clears old maxima.
        let current_allocation = self
            .states
            .wheel_sessions
            .lock()
            .ok()
            .is_some_and(|sessions| {
                sessions
                    .get(&session)
                    .and_then(Option::as_ref)
                    .is_some_and(|allocation| allocation.current)
            });
        // Allocation checks this session's Premium owner too. The spell state's Premium
        // cache can still be unset at entry or stale until the next spell-access refresh.
        let access_ready = if current_allocation && self.premium_coordinator.is_some() {
            // Install the same owner-qualified promotion and conservative Premium deadline
            // that spell casts use. A bool-only gate cannot authorize later combat reads.
            !matches!(
                &*self.refresh_and_apply_spell_access_for_transport(actor, session, budget, generation).await,
                crate::gameplay_transport::spell_premium_coordinator::RegisteredSpellAccessOwner::Unavailable { .. }
            )
        } else {
            true
        };
        if !access_ready {
            return false;
        }
        let runtime = self.runtime.lock().await;
        let mut states = self.states.spell_states.lock().await;
        if generation.is_some_and(|generation| {
            !states.current_cooldown_transport(actor, session, generation)
        }) {
            return false;
        }
        let Some(state) = states.get(&runtime, actor, session) else {
            return false;
        };
        let facts = state.character_facts();
        let vocation = match facts.vocation {
            crate::spell::Vocation::None => "none",
            crate::spell::Vocation::Druid => "druid",
            crate::spell::Vocation::ElderDruid => "elder_druid",
            crate::spell::Vocation::Sorcerer => "sorcerer",
            crate::spell::Vocation::MasterSorcerer => "master_sorcerer",
            crate::spell::Vocation::Knight => "knight",
            crate::spell::Vocation::EliteKnight => "elite_knight",
            crate::spell::Vocation::Paladin => "paladin",
            crate::spell::Vocation::RoyalPaladin => "royal_paladin",
            crate::spell::Vocation::Monk => "monk",
            crate::spell::Vocation::ExaltedMonk => "exalted_monk",
        };
        let now = self.owner_now().get();
        let eligible = current_allocation
            && self.premium_coordinator.is_some()
            && state.wheel_effects_at(now).is_some();
        let bonuses = self
            .wheel_stat_bonuses(session, vocation, facts.level, eligible, eligible)
            .unwrap_or_default();
        states
            .get_mut(&runtime, actor, session)
            .is_some_and(|state| {
                if state
                    .apply_wheel_maximum_bonus(bonuses.max_health, bonuses.max_mana)
                    .is_err()
                {
                    return false;
                }
                state.install_wheel_effects_at(bonuses, now);
                true
            })
    }

    /// WHEEL-0 §4 "Wheel reset" and "Load": the admission Wheel reset in the Character's
    /// revision slot, then the session's allocation. A failed load fails the session's Wheel
    /// closed (every stage 0), never the login.
    pub(in crate::gameplay_transport) async fn admit_wheel_session(&self, session: GameSessionId) {
        let fence = self.current_quest_fence(session).await;
        let retry = self.wheel_ruleset.is_some() && !matches!(&fence, Ok(None));
        let loaded = match (fence, &self.wheel_ruleset) {
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
                .map(|allocation| (fence, allocation))
            }
            _ => None,
        };
        // A Character without admitted progression has no Wheel allocation. Keep the
        // documented fail-closed result local to Wheel instead of immediately starting the
        // progression-backed Atelier owner with the same missing state.
        let Some((fence, allocation)) = loaded else {
            if retry {
                self.schedule_quest_refresh(session);
            }
            if let Ok(mut sessions) = self.states.atelier_sessions.lock() {
                sessions.insert(session, None);
            }
            if let Ok(mut sessions) = self.states.wheel_sessions.lock() {
                sessions.insert(session, None);
            }
            return;
        };
        if self.premium_coordinator.is_some() {
            let _ = self
                .execute_gem_atelier(
                    session,
                    super::super::atelier::GemBuildCommand::Atelier(
                        crate::durability::character_atelier::AtelierRequest::initialize(
                            fence.character_id,
                        ),
                    ),
                )
                .await;
        }
        let atelier = self
            .root
            .read_character_atelier(self.character, fence.character_id)
            .await
            .ok()
            .flatten();
        if let Ok(mut sessions) = self.states.atelier_sessions.lock() {
            sessions.insert(session, atelier);
        }
        // A busy optional Atelier read must not discard the admitted Wheel allocation.
        if let Ok(mut sessions) = self.states.wheel_sessions.lock() {
            sessions.insert(session, Some(allocation));
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
        let mut effects =
            project_wheel_runtime_effects(&allocation, vocation, level, promoted, premium)?;
        if allocation.current
            && level > 50
            && promoted
            && premium
            && let Some(state) = self
                .states
                .atelier_sessions
                .lock()
                .ok()?
                .get(&session)
                .cloned()
                .flatten()
        {
            let data =
                oteryn_gameplay_progression::wheel_gem_data::WheelGemData::embedded().ok()?;
            effects.contribute_gems(&state, &data, &allocation)?;
        }
        Some(effects)
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
        assert!(effects.sockets.gem_vessel_nodes > 0);
    }
}
