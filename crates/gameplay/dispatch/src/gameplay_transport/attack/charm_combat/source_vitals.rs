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
    fn source_hit_gate(
        &self,
        runtime: &ChannelRuntimeV1,
        target: ExactActorRef,
        session: GameSessionId,
        magnitude: u32,
        kind: PlayerHitKind,
        now: SemanticTimeMicros,
    ) -> Option<bool> {
        let state = self.states.get(runtime, target, session)?;
        let (_, _, gate) = crate::spell::actor_conditions::stage_typed_hit_with_gate(
            runtime,
            target,
            state,
            magnitude,
            kind,
            now.get(),
        )
        .ok()?;
        Some(gate > 0)
    }

    #[allow(clippy::too_many_arguments)]
    fn apply_source_incoming_hit(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        source: ExactActorRef,
        target: ExactActorRef,
        session: GameSessionId,
        magnitude: u32,
        base_damage: u32,
        charm_occurrence: Option<DecisionOccurrenceId>,
        kind: PlayerHitKind,
        occurrence: &str,
        now: SemanticTimeMicros,
        conditions: Option<SourceHitConditions<'_>>,
        equipment: Option<SourceHitEquipment<'_>>,
    ) -> Option<PlayerDamageReceipt> {
        let replay = self
            .states
            .source_hit_is_replay(target, session, occurrence);
        let gate_magnitude = if conditions.is_some() {
            self.states.source_incoming_magnitude(
                runtime,
                source,
                target,
                session,
                magnitude,
                now.get(),
            )
        } else {
            magnitude
        };
        let native_bite = equipment
            .as_ref()
            .is_some_and(|equipment| equipment.native_bite);
        let commit = |states: &mut ChannelSpellStates, runtime: &ChannelRuntimeV1| {
            if let Some(c) = conditions {
                states.source_composite_attack_damage(
                    runtime,
                    target,
                    session,
                    magnitude,
                    kind,
                    occurrence,
                    source,
                    c.definitions,
                    c.facts,
                    c.immunities,
                    c.fence,
                    c.stamp,
                )
            } else if native_bite {
                // Native bites still hit the held player during disconnect grace. Their
                // own bite ledger retains replay; the generic source-spell refusal differs.
                let position = runtime.read_actor_position(target).ok()?.position();
                let hit = states.apply_creature_damage_kind_from(
                    runtime,
                    target,
                    session,
                    magnitude,
                    kind,
                    now,
                    super::super::super::analyser::source(runtime, source),
                    crate::gameplay_transport::actor_spell::native_public_death_cause(
                        runtime, source,
                    ),
                )?;
                Some(PlayerDamageReceipt {
                    applied: hit.damage.applied,
                    health_after: hit.damage.health_after,
                    vitals_revision: hit.vitals_revision,
                    death: hit.death.map(|death| {
                        PlayerLethalReceipt::from_native_commit(
                            runtime,
                            target,
                            session,
                            hit.vitals_revision,
                            death,
                            position,
                        )
                    }),
                })
            } else {
                let receipt = states.apply_typed_attack_damage_from(
                    runtime,
                    target,
                    session,
                    magnitude,
                    kind,
                    occurrence,
                    now,
                    super::super::super::analyser::source(runtime, source),
                )?;
                if !replay && receipt.death.is_some() {
                    states.capture_native_death_cause(runtime, target, session, source);
                }
                Some(receipt)
            }
        };
        if replay {
            return commit(self.states, runtime);
        }
        if self.states.is_dead(target)
            || self.states.has_pending_spell_commit(target, session)
            || runtime.assert_actor_spell_unreserved(target).is_err()
        {
            return None;
        }
        let before = self.states.get(runtime, target, session)?;
        let (_, _, gate) = crate::spell::actor_conditions::stage_typed_hit_with_gate(
            runtime,
            target,
            before,
            gate_magnitude,
            kind,
            now.get(),
        )
        .ok()?;
        let revision = before.revision().checked_add(1)?;
        let was_dead = self.states.is_dead(target);
        let plan = (gate > 0)
            .then(|| {
                prepare_incoming(
                    self.states,
                    runtime,
                    self.catalogue,
                    &self.root,
                    source,
                    target,
                    session,
                    base_damage,
                    occurrence,
                    charm_occurrence,
                )
            })
            .flatten();
        let mut parry = plan.as_ref().map_or(0, |(_, _, amount)| *amount);
        let snapshot = runtime.companion_snapshot(source).ok();
        let source_health = snapshot.as_ref().map_or(0, |snapshot| snapshot.health);
        let parry_first = equipment
            .as_ref()
            .is_some_and(|equipment| equipment.native_bite);
        let equipment_stream = equipment.as_ref().map(|equipment| equipment.stream);
        if parry_first
            && snapshot.as_ref().is_some_and(|snapshot| {
                self.states.elite_bounty(
                    runtime,
                    target,
                    session,
                    &snapshot.state.policy.definition_key,
                )
            })
        {
            parry /= 2;
        }
        let equipment = if let Some(equipment) =
            equipment.filter(|equipment| equipment.element.is_some())
            && source_health > 0
        {
            let state = self.states.get(runtime, target, session)?;
            let offence = state.equipped_offence();
            let from = runtime.read_actor_position(source).ok()?.position();
            let to = runtime.read_actor_position(target).ok()?.position();
            let snapshot = snapshot.as_ref()?;
            let amount = offence.reflected(
                equipment.magnitude,
                equipment.element == Some("physical"),
                from.x.abs_diff(to.x).max(from.y.abs_diff(to.y)),
                u64::try_from(snapshot.maximum_health).ok()?,
            );
            if self.states.elite_bounty(
                runtime,
                target,
                session,
                &snapshot.state.policy.definition_key,
            ) {
                amount / 2
            } else {
                amount
            }
        } else {
            0
        };
        let equipment = if parry_first && source_health <= i64::from(parry) {
            0
        } else {
            equipment
        };
        let parry = if !parry_first && source_health <= i64::from(equipment) {
            0
        } else {
            parry
        };
        let source_alive_after_parry =
            source_health > i64::from(parry) + if parry_first { 0 } else { i64::from(equipment) };
        let amount = parry.checked_add(equipment)?;
        let principal = if amount > 0 {
            let lease = runtime.bound_attacker_lease(target, session).ok()?;
            Some(crate::combat::CapturedRewardPrincipal {
                character: lease.character_id(),
                lease_generation: lease.generation(),
                actor: target,
                session,
            })
        } else {
            None
        };
        if amount > 0
            && self.states.equipment_reflection_kills.len() >= runtime.actor_slot_capacity()
        {
            return None;
        }
        self.states
            .equipment_reflection_kills
            .try_reserve(usize::from(amount > 0))
            .ok()?;
        let minor = if let Some((identity, race, _)) = &plan {
            evaluate_incoming_category(
                self.states,
                runtime,
                self.catalogue,
                &self.root,
                target,
                session,
                race,
                *identity,
                base_damage,
                oteryn_gameplay_progression::charm_effects::CharmCategory::Minor,
                source_alive_after_parry,
            )
        } else {
            Vec::new()
        };
        let mut reflected = None;
        let receipt = if amount > 0 {
            let mut stream = b"charm-parry-source:".to_vec();
            stream.extend_from_slice(session.as_bytes());
            let sequence = if plan.is_some() {
                revision
            } else {
                stream.extend_from_slice(b":equipment:");
                stream.extend_from_slice(equipment_stream.unwrap_or_default());
                now.get()
            };
            runtime
                .commit_source_damage_with_vitals(
                    source,
                    target,
                    session,
                    &stream,
                    sequence,
                    amount,
                    now.get() / 1000,
                    |runtime, result| {
                        let receipt = commit(self.states, runtime)?;
                        reflected = Some(result);
                        Some(receipt)
                    },
                )
                .ok()??
        } else {
            commit(self.states, runtime)?
        };
        if let Some(result) = reflected.filter(|result| result.applied) {
            let removed = u64::try_from(result.health_before.saturating_sub(result.health_after))
                .unwrap_or(0);
            let parry_removed = if parry_first {
                removed
            } else {
                removed.saturating_sub(u64::from(equipment))
            }
            .min(u64::from(parry));
            if parry_removed > 0 {
                self.states.analyser.impact_amount(
                    target,
                    session,
                    parry_removed,
                    Some(super::super::super::analyser::charm_element(
                        CharmElement::Physical,
                    )),
                );
            }
            if removed > parry_removed {
                self.states.analyser.impact_amount(
                    target,
                    session,
                    removed - parry_removed,
                    Some(oteryn_protocol_oteryn::damage_element::DamageElement::Untyped),
                );
            }
            if result.health_after == 0
                && let Some(principal) = principal
            {
                self.states
                    .equipment_reflection_kills
                    .push((source, principal, now.get()));
            }
        }
        if let Some((identity, _, _)) = plan {
            apply_incoming_minor(
                self.states,
                runtime,
                &self.root,
                source,
                target,
                session,
                identity,
                now.get(),
                &minor,
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
        if self.states.is_dead(target)
            || self.states.has_pending_spell_commit(target, session)
            || runtime.assert_actor_spell_unreserved(target).is_err()
            || !runtime
                .player_control_facts(target, session)
                .is_ok_and(|facts| facts.control_loss.is_none())
            || !self
                .states
                .get(runtime, target, session)
                .is_some_and(|state| state.owned_conditions().accepts_time(now.get()))
        {
            return false;
        }

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
        self.apply_source_incoming_hit(
            runtime,
            source,
            target,
            session,
            magnitude,
            magnitude,
            None,
            PlayerHitKind::Mitigated,
            occurrence,
            SemanticTimeMicros::from_micros(facts.now),
            Some(SourceHitConditions {
                definitions,
                facts,
                immunities,
                fence: Some(fence),
                stamp,
            }),
            None,
        )
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
        self.apply_source_incoming_hit(
            runtime,
            source,
            target,
            session,
            magnitude,
            magnitude,
            None,
            kind,
            occurrence,
            SemanticTimeMicros::from_micros(facts.now),
            Some(SourceHitConditions {
                definitions,
                facts,
                immunities,
                fence: Some(fence),
                stamp,
            }),
            None,
        )
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
        self.apply_source_incoming_hit(
            runtime,
            source,
            target,
            session,
            magnitude,
            magnitude,
            None,
            kind,
            occurrence,
            SemanticTimeMicros::from_micros(facts.now),
            Some(SourceHitConditions {
                definitions,
                facts,
                immunities,
                fence: None,
                stamp,
            }),
            None,
        )
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
        self.apply_source_incoming_hit(
            runtime,
            source,
            target,
            session,
            magnitude,
            magnitude,
            None,
            PlayerHitKind::Mitigated,
            occurrence,
            SemanticTimeMicros::from_micros(facts.now),
            Some(SourceHitConditions {
                definitions,
                facts,
                immunities,
                fence: None,
                stamp,
            }),
            None,
        )
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
        self.apply_source_incoming_hit(
            runtime, source, target, session, magnitude, magnitude, None, kind, occurrence, now,
            None, None,
        )
    }
}
