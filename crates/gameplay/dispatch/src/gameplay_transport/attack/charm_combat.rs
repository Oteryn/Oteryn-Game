//! Charm consumers at the existing attack, leech and source damage seams.
use super::*;
use sha2::{Digest, Sha256};
pub(in crate::gameplay_transport) mod source_vitals;
use crate::gameplay_transport::actor_spell::ChannelSpellStates;
use oteryn_gameplay_progression::charm_effects::{
    CharmCatalogueRead, CharmDamageKind, CharmEffectOutcome, CharmElement, CharmHookEvent,
    CharmOutcome, CharmResult,
};

#[derive(Debug)]
pub(super) struct DeathCharmSnapshot {
    pub(super) actor: ExactActorRef,
    pub(super) session: GameSessionId,
    stamp: crate::foundation::RuntimeWorkStamp,
    loss: crate::durability::character_death::promotion::DeathCharmLoss,
}

impl ChannelAttackStates {
    #[allow(
        clippy::too_many_arguments,
        reason = "the exact lethal source, actor/session and death occurrence are independently retained"
    )]
    pub(in crate::gameplay_transport) fn capture_death_charm(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        states: &ChannelSpellStates,
        catalogue: &dyn CharmCatalogueRead,
        actor: ExactActorRef,
        session: GameSessionId,
        source: ExactActorRef,
        death: [u8; 16],
        root: &GameplayDecisionRoot,
    ) {
        if self
            .death_charms
            .iter()
            .any(|entry| entry.actor == actor && entry.session == session)
            || self.death_charms.len() >= runtime.actor_slot_capacity()
        {
            return;
        }
        // The already admitted hit still lands if Parry killed its source in this
        // owner turn. The exact incarnation's retained policy remains its race proof.
        let Ok(snapshot) = runtime.companion_snapshot_including_dead(source) else {
            return;
        };
        if snapshot.state.master.is_some() || snapshot.state.policy.is_familiar {
            return;
        }
        let race = &snapshot.state.policy.definition_key;
        let outcome = evaluate(
            states,
            runtime,
            catalogue,
            actor,
            session,
            race,
            root,
            DecisionOccurrenceId::from_bytes(death),
            CharmHookEvent::CharacterDeath,
        )
        .into_iter()
        .find_map(|outcome| match effect(&outcome) {
            Some(CharmEffectOutcome::DeathLossReduction { percent }) => {
                u8::try_from(percent.hundredths() / 100)
                    .ok()
                    .map(|percent| (outcome.stage, percent))
            }
            _ => None,
        });
        let Some((stage, percent)) = outcome else {
            return;
        };
        let Ok(stamp) = runtime.issue_owner_work() else {
            return;
        };
        if self.death_charms.try_reserve(1).is_err() {
            return;
        }
        self.death_charms.push(DeathCharmSnapshot {
            actor,
            session,
            stamp,
            loss: crate::durability::character_death::promotion::DeathCharmLoss {
                death_occurrence: death,
                source_actor: source.placement_identity(),
                race_key: race.clone(),
                stage,
                percent,
            },
        });
    }

    pub(in crate::gameplay_transport) fn death_charm(
        &self,
        runtime: &ChannelRuntimeV1,
        session: GameSessionId,
        death: [u8; 16],
    ) -> Option<crate::durability::character_death::promotion::DeathCharmLoss> {
        let entry = self
            .death_charms
            .iter()
            .find(|entry| entry.session == session && entry.loss.death_occurrence == death)?;
        runtime
            .player_control_facts(entry.actor, session)
            .ok()
            .filter(|facts| facts.control_loss.is_none())?;
        runtime
            .owner_fence()
            .ok()
            .filter(|fence| fence.accepts_stamp(entry.stamp))?;
        Some(entry.loss.clone())
    }

    pub(in crate::gameplay_transport) fn clear_death_charm(
        &mut self,
        session: GameSessionId,
        death: [u8; 16],
    ) {
        self.death_charms
            .retain(|entry| entry.session != session || entry.loss.death_occurrence != death);
    }
}

#[allow(clippy::too_many_arguments)]
pub(in crate::gameplay_transport) fn evaluate(
    states: &ChannelSpellStates,
    runtime: &ChannelRuntimeV1,
    catalogue: &dyn CharmCatalogueRead,
    actor: ExactActorRef,
    session: GameSessionId,
    race: &str,
    root: &GameplayDecisionRoot,
    occurrence: DecisionOccurrenceId,
    event: CharmHookEvent,
) -> Vec<CharmOutcome> {
    let Ok(lease) = runtime.bound_attacker_lease(actor, session) else {
        return Vec::new();
    };
    let Some(state) = states.charm_state(runtime, actor, session) else {
        return Vec::new();
    };
    run_charm_hook(
        catalogue,
        state,
        *lease.character_id().as_bytes(),
        race,
        root,
        occurrence,
        event,
    )
    .unwrap_or_default()
}

// The pure evaluator still identifies historically unavailable systems. Only these consumers
// discharge that marker locally; all other FailedClosed outcomes stay unapplied.
pub(in crate::gameplay_transport) fn effect(outcome: &CharmOutcome) -> Option<CharmEffectOutcome> {
    match outcome.result {
        CharmResult::Applied(effect)
        | CharmResult::FailedClosed {
            evaluated: effect, ..
        } => Some(effect),
        _ => None,
    }
}

/// W22b: the loot owner's Gut Charm bonus on `race`, in hundredths of a percent; 0 without one.
/// Gut has no trigger roll, so the death's occurrence only keys the evaluation.
#[allow(clippy::too_many_arguments)]
pub(in crate::gameplay_transport) fn gut_bonus_hundredths(
    states: &ChannelSpellStates,
    runtime: &ChannelRuntimeV1,
    catalogue: &dyn CharmCatalogueRead,
    actor: ExactActorRef,
    session: GameSessionId,
    race: &str,
    root: &GameplayDecisionRoot,
    occurrence: DecisionOccurrenceId,
) -> u32 {
    evaluate(
        states,
        runtime,
        catalogue,
        actor,
        session,
        race,
        root,
        occurrence,
        CharmHookEvent::CreatureLoot,
    )
    .iter()
    .find_map(|outcome| match effect(outcome) {
        Some(CharmEffectOutcome::CreatureProductBonus { percent }) => Some(percent.hundredths()),
        _ => None,
    })
    .unwrap_or(0)
}

pub(super) fn bonuses(
    offence: &mut oteryn_gameplay_combat_rules::resolution::equipped_offence::EquippedOffence,
    outcomes: &[CharmOutcome],
) {
    for outcome in outcomes {
        let (slot, percent, requires_existing_skill) = match effect(outcome) {
            Some(CharmEffectOutcome::CriticalHitChanceBonus { percent }) => {
                (&mut offence.critical_chance, percent, false)
            }
            Some(CharmEffectOutcome::CriticalExtraDamageBonus { percent }) => {
                (&mut offence.critical_damage, percent, false)
            }
            Some(CharmEffectOutcome::LifeLeechBonus { percent }) => {
                (&mut offence.life_leech, percent, true)
            }
            Some(CharmEffectOutcome::ManaLeechBonus { percent }) => {
                (&mut offence.mana_leech, percent, true)
            }
            _ => continue,
        };
        // CipSoft news 8421: critical chance and extra damage exist intrinsically
        // (5% / 10%) even when their additional equipment fields are zero.
        // Leech has no intrinsic amount, so its existing-skill rule is unchanged.
        if requires_existing_skill && *slot == 0 {
            continue;
        }
        *slot = slot.saturating_add(u16::try_from(percent.hundredths()).unwrap_or(u16::MAX));
    }
}

/// The committed hit's passive contributions. Secondary auto targets receive
/// no ordinary equipment Leech, but retain the accepted D186 charm exceptions.
pub(in crate::gameplay_transport) fn committed_hit_bonuses(
    offence: &mut oteryn_gameplay_combat_rules::resolution::equipped_offence::EquippedOffence,
    outcomes: &[CharmOutcome],
    off_target: bool,
) {
    let ordinary = (offence.life_leech, offence.mana_leech);
    // Eligibility uses the original installed skill. Zeroing it first would
    // also suppress the two D186 secondary-target charm exceptions.
    bonuses(offence, outcomes);
    if off_target {
        offence.life_leech = offence.life_leech.saturating_sub(ordinary.0);
        offence.mana_leech = offence.mana_leech.saturating_sub(ordinary.1);
    }
}

/// Feed passive Low Blow / Savage Blow into the real numerical spell owner.
/// No chance is rolled here: the qualified AoE owner retains its common roll.
#[allow(
    clippy::too_many_arguments,
    reason = "current actor, source catalogue and spell provenance stay explicit"
)]
pub(in crate::gameplay_transport) fn select_spell_critical(
    states: &ChannelSpellStates,
    runtime: &ChannelRuntimeV1,
    catalogue: &dyn CharmCatalogueRead,
    actor: ExactActorRef,
    command: CommandRef,
    magnitude: &mut crate::spell::magnitude_owner::PreparedMagnitudeOwner,
) -> Result<(), crate::spell::combat_batch::Error> {
    use crate::spell::combat_batch::Error;
    let offence = super::equipped_offence::attacker_offence(
        states,
        runtime,
        actor,
        command.game_session_id(),
    );
    let root = GameplayDecisionRoot::from_bytes(runtime.content_pin().server_artifact_digest());
    let hash = Sha256::new()
        .chain_update(b"oteryn:spell-passive-charm:v1")
        .chain_update(actor.placement_identity())
        .chain_update(command.command_id().get().to_le_bytes())
        .finalize();
    let mut bytes = [0; 16];
    bytes.copy_from_slice(&hash[..16]);
    let occurrence = DecisionOccurrenceId::from_bytes(bytes);
    let mut selected = Vec::new();
    for (target, race) in magnitude.charm_targets() {
        let outcomes = evaluate(
            states,
            runtime,
            catalogue,
            actor,
            command.game_session_id(),
            race,
            &root,
            occurrence,
            CharmHookEvent::AttackDamageCalculation {
                source: oteryn_gameplay_progression::charm_effects::CharmHitSource::CharacterAttack,
            },
        );
        let mut augmented = offence.clone();
        bonuses(&mut augmented, &outcomes);
        let chance = augmented
            .critical_chance
            .saturating_sub(offence.critical_chance);
        let extra = augmented
            .critical_damage
            .saturating_sub(offence.critical_damage);
        if chance != 0 || extra != 0 {
            selected
                .try_reserve(1)
                .map_err(|_| Error::AllocationFailed)?;
            selected.push((target, chance, extra));
        }
    }
    magnitude.select_charm_critical(selected)
}

pub(super) fn damage_amount(
    runtime: &ChannelRuntimeV1,
    target: ExactActorRef,
    damage: CharmDamageKind,
    amount: u64,
    armor_draw: u64,
) -> Option<u32> {
    let snapshot = runtime.companion_snapshot(target).ok()?;
    let mut amount = u32::try_from(amount).unwrap_or(u32::MAX);
    if damage.reduced_by_armor {
        amount = oteryn_gameplay_combat_rules::resolution::player_armor_mitigated(
            AttackConstants::checked_in()?,
            amount,
            snapshot.state.policy.armor?,
            armor_draw,
        );
    }
    if !damage.ignores_resistances {
        let element = match damage.element {
            CharmElement::Physical => "physical",
            CharmElement::Fire => "fire",
            CharmElement::Earth => "earth",
            CharmElement::Energy => "energy",
            CharmElement::Ice => "ice",
            CharmElement::Holy => "holy",
            CharmElement::Death => "death",
        };
        amount = u32::try_from(
            oteryn_gameplay_combat_rules::resolution::creature_damage_response(
                &snapshot.state.policy,
                i64::from(amount),
                element,
            )?,
        )
        .ok()?;
    }
    Some(amount)
}

/// Cripple/Numb use the existing native actor condition owner and its immunity/CAS rules.
/// Canary 47dfd51f iobestiary.cpp:105-109,149-153; condition.cpp:2542-2553.
#[allow(
    clippy::too_many_arguments,
    reason = "native source, target, session and deterministic occurrence remain explicit"
)]
pub(super) fn paralyse_creature(
    runtime: &mut ChannelRuntimeV1,
    source: ExactActorRef,
    session: GameSessionId,
    target: ExactActorRef,
    duration_ms: u32,
    root: &GameplayDecisionRoot,
    occurrence: DecisionOccurrenceId,
    now: u64,
) -> bool {
    use crate::foundation::{
        ActorConditionTransition, ApplicationFacts, ConditionDefinition, ConditionSource,
        ConditionSourceKind, ConditionValues, SpeedRange,
    };
    let Ok(snapshot) = runtime.companion_snapshot(target) else {
        return false;
    };
    if snapshot.health <= 0 || snapshot.state.master.is_some() || snapshot.state.policy.is_familiar
    {
        return false;
    }
    let Ok(base_speed) = u16::try_from(snapshot.state.policy.base_speed) else {
        return false;
    };
    let Some(definition) = ConditionDefinition::new(
        "oteryn:charm.paralysis",
        1,
        ConditionValues::Speed {
            paralysis: true,
            range: SpeedRange {
                a_min: -1000,
                a_max: -1000,
                b_min: 0,
                b_max: 0,
            },
            duration_ms,
        },
    ) else {
        return false;
    };
    let facts = ApplicationFacts {
        now,
        base_speed,
        mana_shield_capacity: 0,
        target_reentry_protected: false,
        source_reentry_protected: false,
        target_is_player: false,
        decision_root: root,
        occurrence,
    };
    let time = oteryn_simulation_determinism::SemanticTimeMicros::from_micros(now);
    let Ok(plan) = runtime.prepare_actor_condition(
        target,
        None,
        ActorConditionTransition::Apply {
            definition: &definition,
            source: ConditionSource {
                actor: source,
                session: Some(session),
                kind: ConditionSourceKind::Player,
            },
            immunities: &snapshot.state.policy.condition_immunities,
            facts,
        },
        time,
    ) else {
        return false;
    };
    matches!(runtime.commit_actor_condition(&plan, time), Ok(true))
}

/// Committed native source damage; caller suppresses receipt replay before entering.
#[allow(clippy::too_many_arguments)]
pub(in crate::gameplay_transport) fn incoming_conditions(
    states: &mut ChannelSpellStates,
    runtime: &mut ChannelRuntimeV1,
    catalogue: &dyn CharmCatalogueRead,
    root: &GameplayDecisionRoot,
    source: ExactActorRef,
    target: ExactActorRef,
    session: GameSessionId,
    magnitude: u32,
    source_revision: u64,
    occurrence_text: &str,
    now: u64,
    mana: bool,
) {
    let Ok(creature) = runtime.companion_snapshot(source) else {
        return;
    };
    if magnitude == 0 || creature.state.master.is_some() || creature.state.policy.is_familiar {
        return;
    }
    let digest = Sha256::new()
        .chain_update(b"oteryn.charm.source.v1")
        .chain_update(occurrence_text.as_bytes())
        .finalize();
    let mut bytes = [0; 16];
    bytes.copy_from_slice(&digest[..16]);
    let occurrence = DecisionOccurrenceId::from_bytes(bytes);
    let event = if mana {
        CharmHookEvent::IncomingManaDrain {
            mana_drained: u64::from(magnitude),
        }
    } else {
        CharmHookEvent::IncomingCreatureHit {
            base_damage: u64::from(magnitude),
        }
    };
    let outcomes = evaluate(
        states,
        runtime,
        catalogue,
        target,
        session,
        &creature.state.policy.definition_key,
        root,
        occurrence,
        event,
    );
    for outcome in &outcomes {
        match effect(outcome) {
            Some(CharmEffectOutcome::ReflectDamage { damage, amount }) if !mana => {
                let Ok(draw) = oteryn_simulation_determinism::deterministic_decision_u64(
                    root,
                    occurrence,
                    "charm.parry-armor",
                    0,
                ) else {
                    continue;
                };
                let Some(amount) = damage_amount(runtime, source, damage, amount, draw)
                    .filter(|amount| *amount > 0)
                else {
                    continue;
                };
                let Ok(lease) = runtime.bound_attacker_lease(target, session) else {
                    continue;
                };
                // One bounded stream per source/reflector/session. The native HP successor
                // revision is strictly increasing, independently fencing this exact receipt.
                let mut stream = b"charm-parry-source:".to_vec();
                stream.extend_from_slice(session.as_bytes());
                if let Ok(result) = runtime.commit_source_damage(
                    source,
                    target,
                    session,
                    &stream,
                    source_revision,
                    amount,
                    now / 1000,
                ) && result.applied
                {
                    states.analyser.impact_amount(
                        target,
                        session,
                        u64::try_from(result.health_before.saturating_sub(result.health_after))
                            .unwrap_or(0),
                        Some(super::super::analyser::charm_element(damage.element)),
                    );
                    if result.health_after == 0 {
                        states.equipment_reflection_kills.push((
                            source,
                            crate::combat::CapturedRewardPrincipal {
                                character: lease.character_id(),
                                lease_generation: lease.generation(),
                                actor: target,
                                session,
                            },
                            now,
                        ));
                    }
                }
            }
            Some(CharmEffectOutcome::ParalyseCreature { duration_ms }) => {
                paralyse_creature(
                    runtime,
                    target,
                    session,
                    source,
                    duration_ms,
                    root,
                    occurrence,
                    now,
                );
            }
            Some(
                effect @ (CharmEffectOutcome::Haste { .. }
                | CharmEffectOutcome::CleanseOneNegativeCondition),
            ) => {
                states.apply_player_charm_condition(
                    runtime, target, session, effect, root, occurrence, now,
                );
            }
            _ => {}
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::durability::charm_state::{CharacterCharmState, CombatCharmState};
    use crate::gameplay_transport::actor_spell::tests::runtime_with_player;

    #[test]
    fn charm_bonuses_augment_intrinsic_critical_but_require_existing_leech() {
        use oteryn_gameplay_combat_rules::resolution::equipped_offence::EquippedOffence;
        use oteryn_gameplay_progression::charm_effects::{
            CharmHook, CharmMissingSystem, CharmPercent,
        };
        let percent = CharmPercent::from_hundredths(400).expect("percent");
        let outcomes: Vec<_> = [
            CharmEffectOutcome::CriticalHitChanceBonus { percent },
            CharmEffectOutcome::CriticalExtraDamageBonus { percent },
            CharmEffectOutcome::LifeLeechBonus { percent },
            CharmEffectOutcome::ManaLeechBonus { percent },
        ]
        .into_iter()
        .map(|evaluated| CharmOutcome {
            charm_key: "fixture:charm".into(),
            stage: 1,
            hook: CharmHook::AttackHit,
            result: CharmResult::FailedClosed {
                reason: CharmMissingSystem::CriticalHit,
                evaluated,
            },
        })
        .collect();
        let mut empty = EquippedOffence::default();
        bonuses(&mut empty, &outcomes);
        assert_eq!(
            (
                empty.critical_chance,
                empty.critical_damage,
                empty.life_leech,
                empty.mana_leech
            ),
            (400, 400, 0, 0),
            "CipSoft 8421: both critical bonuses augment innate skills without Strike; no free leech"
        );
        assert_eq!(empty.player_critical(), (900, 1400));
        let mut equipped = EquippedOffence::from_sums(100, 5000, 100, 200);
        bonuses(&mut equipped, &outcomes);
        assert_eq!(
            (
                equipped.critical_chance,
                equipped.critical_damage,
                equipped.life_leech,
                equipped.mana_leech
            ),
            (500, 5400, 500, 600)
        );
    }

    #[test]
    fn charm_evaluation_error_suppresses_only_the_effects() {
        let (mut runtime, actor, session) = runtime_with_player(26);
        let character =
            crate::foundation::CharacterId::decode(session.as_bytes()).expect("character");
        runtime
            .bind_attacker_lease(
                actor,
                session,
                crate::foundation::CharacterLease::new(character, 1).expect("lease"),
            )
            .expect("bind");
        let mut states = ChannelSpellStates::default();
        let catalogue =
            crate::content::charm_source::CanonicalCharmCatalogue::embedded().expect("catalogue");
        let root = GameplayDecisionRoot::from_bytes([1; 32]);
        let occurrence = DecisionOccurrenceId::from_bytes([2; 16]);
        assert!(
            evaluate(
                &states,
                &runtime,
                &catalogue,
                actor,
                session,
                "oteryn:creature.rat",
                &root,
                occurrence,
                CharmHookEvent::IncomingCreatureAttack
            )
            .is_empty()
        );
        let wrong_character =
            crate::domain::CharacterId::from_bytes(*runtime_with_player(27).2.as_bytes())
                .expect("other character");
        assert!(states.install_charm_state(
            &runtime,
            actor,
            session,
            CombatCharmState::new(wrong_character, CharacterCharmState::default())
        ));
        assert!(
            evaluate(
                &states,
                &runtime,
                &catalogue,
                actor,
                session,
                "oteryn:creature.rat",
                &root,
                occurrence,
                CharmHookEvent::IncomingCreatureAttack
            )
            .is_empty()
        );
        assert!(runtime.player_control_facts(actor, session).is_ok());
    }

    #[test]
    fn charm_snapshot_is_session_scoped_and_prunes_a_logged_out_generation() {
        let (mut runtime, actor, session) = runtime_with_player(24);
        let character =
            crate::domain::CharacterId::from_bytes(*session.as_bytes()).expect("character");
        let mut states = ChannelSpellStates::default();
        assert!(states.install_charm_state(
            &runtime,
            actor,
            session,
            CombatCharmState::new(character, CharacterCharmState::default())
        ));
        let other = runtime_with_player(25).2;
        assert!(states.charm_state(&runtime, actor, other).is_none());
        runtime
            .remove_terminal_session(session, actor)
            .expect("terminal session");
        assert!(states.charm_state(&runtime, actor, session).is_none());
        let reservation = runtime
            .reserve_fresh_session(other)
            .expect("reserve replacement");
        let replacement = runtime
            .commit_fresh_session(reservation)
            .expect("replacement");
        assert!(states.install_charm_state(
            &runtime,
            replacement,
            other,
            CombatCharmState::new(character, CharacterCharmState::default())
        ));
        assert_eq!(states.charm_states.len(), 1);
        assert!(states.charm_state(&runtime, replacement, session).is_none());
    }
}
