//! P01 source-qualified native charm forwarding; existing owners retain every non-charm path.
use super::*;
use crate::foundation::owner_timer::SemanticTimeMicros;
use crate::player_lethal::*;

pub(in crate::gameplay_transport) struct CharmSourceVitals<'a> {
    pub states: &'a mut ChannelSpellStates,
    pub attack: &'a mut ChannelAttackStates,
    pub catalogue: &'a dyn CharmCatalogueRead,
    pub root: GameplayDecisionRoot,
}

impl PlayerLethalVitals for CharmSourceVitals<'_> {
    fn dodge_source_attack(
        &self,
        runtime: &ChannelRuntimeV1,
        source: ExactActorRef,
        target: ExactActorRef,
        session: GameSessionId,
        occurrence: &str,
    ) -> bool {
        if self
            .states
            .source_player_health(runtime, target, session)
            .is_none()
            || self.states.has_pending_spell_commit(target, session)
            || runtime.assert_actor_spell_unreserved(target).is_err()
        {
            return false;
        }
        let Ok(creature) = runtime.companion_snapshot(source) else {
            return false;
        };
        if creature.state.master.is_some() || creature.state.policy.is_familiar {
            return false;
        }
        let digest = Sha256::new()
            .chain_update(b"oteryn.charm.source.v1")
            .chain_update(occurrence.as_bytes())
            .finalize();
        let mut identity = [0; 16];
        identity.copy_from_slice(&digest[..16]);
        evaluate(
            self.states,
            runtime,
            self.catalogue,
            target,
            session,
            &creature.state.policy.definition_key,
            &self.root,
            DecisionOccurrenceId::from_bytes(identity),
            CharmHookEvent::IncomingCreatureAttack,
        )
        .iter()
        .any(|outcome| matches!(effect(outcome), Some(CharmEffectOutcome::DodgeAttack)))
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "preserve the existing native owner ABI"
    )]
    fn reflect_equipment_damage(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        source: ExactActorRef,
        target: ExactActorRef,
        session: GameSessionId,
        element: Option<&str>,
        magnitude: u32,
        stream: &[u8],
        now: crate::foundation::owner_timer::SemanticTimeMicros,
    ) -> bool {
        PlayerLethalVitals::reflect_equipment_damage(
            self.states,
            runtime,
            source,
            target,
            session,
            element,
            magnitude,
            stream,
            now,
        )
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "preserve the existing native owner ABI"
    )]
    fn apply_source_appearance_batch(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        fence: &crate::foundation::ScopeRuntimeFence,
        stamp: crate::foundation::RuntimeWorkStamp,
        source: ExactActorRef,
        definitions: &[crate::foundation::ConditionDefinition],
        targets: &[SourceAppearanceTarget<'_>],
    ) -> Option<Vec<SourceAppearanceReceipt>> {
        PlayerLethalVitals::apply_source_appearance_batch(
            self.states,
            runtime,
            fence,
            stamp,
            source,
            definitions,
            targets,
        )
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "preserve the existing native owner ABI"
    )]
    fn apply_source_appearance_batch_current(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        stamp: crate::foundation::RuntimeWorkStamp,
        source: ExactActorRef,
        definitions: &[crate::foundation::ConditionDefinition],
        targets: &[SourceAppearanceTarget<'_>],
    ) -> Option<Vec<SourceAppearanceReceipt>> {
        PlayerLethalVitals::apply_source_appearance_batch_current(
            self.states,
            runtime,
            stamp,
            source,
            definitions,
            targets,
        )
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "preserve the existing native owner ABI"
    )]
    fn source_player_health(
        &self,
        runtime: &ChannelRuntimeV1,
        target: ExactActorRef,
        session: GameSessionId,
    ) -> Option<(u32, u32, u64)> {
        PlayerLethalVitals::source_player_health(self.states, runtime, target, session)
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "preserve the existing native owner ABI"
    )]
    fn apply_source_player_heal(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        target: ExactActorRef,
        session: GameSessionId,
        magnitude: u32,
        occurrence: &str,
        now: crate::foundation::owner_timer::SemanticTimeMicros,
    ) -> Option<PlayerHealReceipt> {
        PlayerLethalVitals::apply_source_player_heal(
            self.states,
            runtime,
            target,
            session,
            magnitude,
            occurrence,
            now,
        )
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "preserve the existing native owner ABI"
    )]
    fn remove_attack_invisibility(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        target: ExactActorRef,
        session: GameSessionId,
        source: ExactActorRef,
        now: u64,
        fence: &crate::foundation::ScopeRuntimeFence,
        stamp: crate::foundation::RuntimeWorkStamp,
    ) -> bool {
        PlayerLethalVitals::remove_attack_invisibility(
            self.states,
            runtime,
            target,
            session,
            source,
            now,
            fence,
            stamp,
        )
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "preserve the existing native owner ABI"
    )]
    fn apply_attack_mana_drain(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        target: ExactActorRef,
        session: GameSessionId,
        magnitude: u32,
        occurrence: &str,
        now: crate::foundation::owner_timer::SemanticTimeMicros,
    ) -> Option<PlayerManaDrainReceipt> {
        PlayerLethalVitals::apply_attack_mana_drain(
            self.states,
            runtime,
            target,
            session,
            magnitude,
            occurrence,
            now,
        )
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "preserve the existing native owner ABI"
    )]
    fn apply_attack_conditions(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        target: ExactActorRef,
        session: GameSessionId,
        occurrence: &str,
        source: ExactActorRef,
        definitions: &[crate::foundation::ConditionDefinition],
        facts: &crate::foundation::ApplicationFacts<'_>,
        immunities: &[crate::foundation::ConditionType],
        fence: &crate::foundation::ScopeRuntimeFence,
        stamp: crate::foundation::RuntimeWorkStamp,
    ) -> bool {
        PlayerLethalVitals::apply_attack_conditions(
            self.states,
            runtime,
            target,
            session,
            occurrence,
            source,
            definitions,
            facts,
            immunities,
            fence,
            stamp,
        )
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "preserve the existing native owner ABI"
    )]
    fn apply_composite_attack_damage(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        target: ExactActorRef,
        session: GameSessionId,
        magnitude: u32,
        occurrence: &str,
        source: ExactActorRef,
        definitions: &[crate::foundation::ConditionDefinition],
        facts: &crate::foundation::ApplicationFacts<'_>,
        immunities: &[crate::foundation::ConditionType],
        fence: &crate::foundation::ScopeRuntimeFence,
        stamp: crate::foundation::RuntimeWorkStamp,
    ) -> Option<PlayerDamageReceipt> {
        let before_hit = self.states.source_player_health(runtime, target, session);
        let was_dead = self.states.is_dead(target);
        let receipt = PlayerLethalVitals::apply_composite_attack_damage(
            self.states,
            runtime,
            target,
            session,
            magnitude,
            occurrence,
            source,
            definitions,
            facts,
            immunities,
            fence,
            stamp,
        )?;
        if magnitude > 0 && before_hit != self.states.source_player_health(runtime, target, session)
        {
            incoming_conditions(
                self.states,
                runtime,
                self.catalogue,
                &self.root,
                source,
                target,
                session,
                magnitude,
                receipt.vitals_revision,
                occurrence,
                facts.now,
                false,
            );
        }
        if !was_dead && let Some(death) = receipt.death {
            self.attack.capture_death_charm(
                runtime,
                self.states,
                self.catalogue,
                target,
                session,
                source,
                death.occurrence_bytes(),
                &self.root,
            );
        }
        Some(receipt)
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "preserve the existing native owner ABI"
    )]
    fn apply_typed_attack_damage(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        target: ExactActorRef,
        session: GameSessionId,
        magnitude: u32,
        kind: PlayerHitKind,
        occurrence: &str,
        now: crate::foundation::owner_timer::SemanticTimeMicros,
    ) -> Option<PlayerDamageReceipt> {
        PlayerLethalVitals::apply_typed_attack_damage(
            self.states,
            runtime,
            target,
            session,
            magnitude,
            kind,
            occurrence,
            now,
        )
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "preserve the existing native owner ABI"
    )]
    fn apply_typed_composite_attack_damage(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        target: ExactActorRef,
        session: GameSessionId,
        magnitude: u32,
        kind: PlayerHitKind,
        occurrence: &str,
        source: ExactActorRef,
        definitions: &[crate::foundation::ConditionDefinition],
        facts: &crate::foundation::ApplicationFacts<'_>,
        immunities: &[crate::foundation::ConditionType],
        fence: &crate::foundation::ScopeRuntimeFence,
        stamp: crate::foundation::RuntimeWorkStamp,
    ) -> Option<PlayerDamageReceipt> {
        let before_hit = self.states.source_player_health(runtime, target, session);
        let was_dead = self.states.is_dead(target);
        let receipt = PlayerLethalVitals::apply_typed_composite_attack_damage(
            self.states,
            runtime,
            target,
            session,
            magnitude,
            kind,
            occurrence,
            source,
            definitions,
            facts,
            immunities,
            fence,
            stamp,
        )?;
        if magnitude > 0 && before_hit != self.states.source_player_health(runtime, target, session)
        {
            incoming_conditions(
                self.states,
                runtime,
                self.catalogue,
                &self.root,
                source,
                target,
                session,
                magnitude,
                receipt.vitals_revision,
                occurrence,
                facts.now,
                false,
            );
        }
        if !was_dead && let Some(death) = receipt.death {
            self.attack.capture_death_charm(
                runtime,
                self.states,
                self.catalogue,
                target,
                session,
                source,
                death.occurrence_bytes(),
                &self.root,
            );
        }
        Some(receipt)
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "preserve the existing native owner ABI"
    )]
    fn apply_typed_composite_attack_damage_current(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        target: ExactActorRef,
        session: GameSessionId,
        magnitude: u32,
        kind: PlayerHitKind,
        occurrence: &str,
        source: ExactActorRef,
        definitions: &[crate::foundation::ConditionDefinition],
        facts: &crate::foundation::ApplicationFacts<'_>,
        immunities: &[crate::foundation::ConditionType],
        stamp: crate::foundation::RuntimeWorkStamp,
    ) -> Option<PlayerDamageReceipt> {
        let before_hit = self.states.source_player_health(runtime, target, session);
        let was_dead = self.states.is_dead(target);
        let receipt = PlayerLethalVitals::apply_typed_composite_attack_damage_current(
            self.states,
            runtime,
            target,
            session,
            magnitude,
            kind,
            occurrence,
            source,
            definitions,
            facts,
            immunities,
            stamp,
        )?;
        if magnitude > 0 && before_hit != self.states.source_player_health(runtime, target, session)
        {
            incoming_conditions(
                self.states,
                runtime,
                self.catalogue,
                &self.root,
                source,
                target,
                session,
                magnitude,
                receipt.vitals_revision,
                occurrence,
                facts.now,
                false,
            );
        }
        if !was_dead && let Some(death) = receipt.death {
            self.attack.capture_death_charm(
                runtime,
                self.states,
                self.catalogue,
                target,
                session,
                source,
                death.occurrence_bytes(),
                &self.root,
            );
        }
        Some(receipt)
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "preserve the existing native owner ABI"
    )]
    fn apply_attack_conditions_current(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        target: ExactActorRef,
        session: GameSessionId,
        occurrence: &str,
        source: ExactActorRef,
        definitions: &[crate::foundation::ConditionDefinition],
        facts: &crate::foundation::ApplicationFacts<'_>,
        immunities: &[crate::foundation::ConditionType],
        stamp: crate::foundation::RuntimeWorkStamp,
    ) -> bool {
        PlayerLethalVitals::apply_attack_conditions_current(
            self.states,
            runtime,
            target,
            session,
            occurrence,
            source,
            definitions,
            facts,
            immunities,
            stamp,
        )
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "preserve the existing native owner ABI"
    )]
    fn apply_composite_attack_damage_current(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        target: ExactActorRef,
        session: GameSessionId,
        magnitude: u32,
        occurrence: &str,
        source: ExactActorRef,
        definitions: &[crate::foundation::ConditionDefinition],
        facts: &crate::foundation::ApplicationFacts<'_>,
        immunities: &[crate::foundation::ConditionType],
        stamp: crate::foundation::RuntimeWorkStamp,
    ) -> Option<PlayerDamageReceipt> {
        let before_hit = self.states.source_player_health(runtime, target, session);
        let was_dead = self.states.is_dead(target);
        let receipt = PlayerLethalVitals::apply_composite_attack_damage_current(
            self.states,
            runtime,
            target,
            session,
            magnitude,
            occurrence,
            source,
            definitions,
            facts,
            immunities,
            stamp,
        )?;
        if magnitude > 0 && before_hit != self.states.source_player_health(runtime, target, session)
        {
            incoming_conditions(
                self.states,
                runtime,
                self.catalogue,
                &self.root,
                source,
                target,
                session,
                magnitude,
                receipt.vitals_revision,
                occurrence,
                facts.now,
                false,
            );
        }
        if !was_dead && let Some(death) = receipt.death {
            self.attack.capture_death_charm(
                runtime,
                self.states,
                self.catalogue,
                target,
                session,
                source,
                death.occurrence_bytes(),
                &self.root,
            );
        }
        Some(receipt)
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "preserve the existing native owner ABI"
    )]
    fn remove_attack_invisibility_current(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        target: ExactActorRef,
        session: GameSessionId,
        source: ExactActorRef,
        now: u64,
        stamp: crate::foundation::RuntimeWorkStamp,
    ) -> bool {
        PlayerLethalVitals::remove_attack_invisibility_current(
            self.states,
            runtime,
            target,
            session,
            source,
            now,
            stamp,
        )
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "preserve the existing native owner ABI"
    )]
    fn apply_attack_damage(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        target: ExactActorRef,
        session: GameSessionId,
        magnitude: u32,
        occurrence: &str,
        now: crate::foundation::owner_timer::SemanticTimeMicros,
    ) -> Option<PlayerDamageReceipt> {
        PlayerLethalVitals::apply_attack_damage(
            self.states,
            runtime,
            target,
            session,
            magnitude,
            occurrence,
            now,
        )
    }
    #[allow(
        clippy::too_many_arguments,
        reason = "preserve the source-qualified native owner ABI"
    )]
    fn apply_source_mana_drain(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        source: ExactActorRef,
        target: ExactActorRef,
        session: GameSessionId,
        magnitude: u32,
        occurrence: &str,
        now: SemanticTimeMicros,
    ) -> Option<PlayerManaDrainReceipt> {
        self.states.apply_charm_mana_drain(
            runtime,
            self.catalogue,
            &self.root,
            source,
            target,
            session,
            magnitude,
            occurrence,
            now,
        )
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "retain source identity through the existing native hit ABI"
    )]
    fn apply_source_typed_attack_damage(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        source: ExactActorRef,
        target: ExactActorRef,
        session: GameSessionId,
        magnitude: u32,
        kind: PlayerHitKind,
        occurrence: &str,
        now: SemanticTimeMicros,
    ) -> Option<PlayerDamageReceipt> {
        let before_hit = self.states.source_player_health(runtime, target, session);
        let was_dead = self.states.is_dead(target);
        let receipt = PlayerLethalVitals::apply_typed_attack_damage(
            self.states,
            runtime,
            target,
            session,
            magnitude,
            kind,
            occurrence,
            now,
        )?;
        if magnitude > 0 && before_hit != self.states.source_player_health(runtime, target, session)
        {
            incoming_conditions(
                self.states,
                runtime,
                self.catalogue,
                &self.root,
                source,
                target,
                session,
                magnitude,
                receipt.vitals_revision,
                occurrence,
                now.get(),
                false,
            );
        }
        if !was_dead && let Some(death) = receipt.death {
            self.attack.capture_death_charm(
                runtime,
                self.states,
                self.catalogue,
                target,
                session,
                source,
                death.occurrence_bytes(),
                &self.root,
            );
        }
        Some(receipt)
    }
}
