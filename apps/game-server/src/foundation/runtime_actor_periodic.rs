//! Actual condition-store ticks use an owner-input origin, never a player command.
//! The world owner supplies independently qualified standing/PZ facts in this
//! uninterrupted turn. No detached tick or caller damage number is accepted.
type CurrentPlayerPeriodicSource<'a> =
    dyn FnMut(&ChannelRuntimeV1, ExactActorRef, u64) -> Result<Option<u32>, CarrierError> + 'a;

use super::runtime_actor_companion::{CompanionSnapshot, CompanionState};
use super::*;
use crate::foundation::RuntimeWorkStamp;
use crate::foundation::condition::{ConditionTick, TickFacts, TickKind};

#[derive(Debug)]
pub(crate) struct PreparedCreaturePeriodicTurn {
    before: CompanionSnapshot,
    original_slot: Slot,
    after: CompanionState,
    health_after: i64,
    stamp: RuntimeWorkStamp,
    ticks: Vec<ConditionTick<ExactActorRef>>,
    next_ordinal: u64,
    lethal_record: Option<OwnerCommitRecord>,
}
#[derive(Debug)]
pub(crate) struct CreaturePeriodicReceipt {
    pub(crate) actor: ExactActorRef,
    pub(crate) health_before: i64,
    pub(crate) health_after: i64,
    pub(crate) ticks: Vec<ConditionTick<ExactActorRef>>,
}
impl ChannelRuntimeV1 {
    /// Stages only a source-qualified physical actor's existing condition store.
    /// `facts` belong to the actual world/tile owner, not the attacker or client.
    pub(crate) fn stage_creature_periodic_turn(
        &mut self,
        actor: ExactActorRef,
        now_us: u64,
        facts: TickFacts,
    ) -> Result<PreparedCreaturePeriodicTurn, CarrierError> {
        self.stage_creature_periodic_turn_inner(actor, now_us, facts, None)
    }
    /// Source snapshots/PlayerSpellState reads are acquired and committed in
    /// one locked Channel-owner call. This is not a detached damage capability.
    pub(crate) fn apply_creature_periodic_turn_with_sources(
        &mut self,
        actor: ExactActorRef,
        now_us: u64,
        facts: TickFacts,
        player_source: &mut CurrentPlayerPeriodicSource<'_>,
    ) -> Result<CreaturePeriodicReceipt, CarrierError> {
        let prepared =
            self.stage_creature_periodic_turn_inner(actor, now_us, facts, Some(player_source))?;
        self.commit_creature_periodic_turn(prepared)
    }

    fn stage_creature_periodic_turn_inner(
        &mut self,
        actor: ExactActorRef,
        now_us: u64,
        facts: TickFacts,
        mut player_source: Option<&mut CurrentPlayerPeriodicSource<'_>>,
    ) -> Result<PreparedCreaturePeriodicTurn, CarrierError> {
        if self.actor_spell_reserved(actor) {
            return Err(CarrierError::PlanConflict);
        }
        let before = self.companion_snapshot(actor)?;
        if self
            .companion_policy(&before.state.policy.definition_key)?
            .as_ref()
            != before.state.policy.as_ref()
        {
            return Err(CarrierError::PlanConflict);
        }
        if self
            .carrier
            .bone_shared_groups
            .iter()
            .any(|g| g.phylactery == actor || g.cages.contains(&actor))
        {
            return Err(CarrierError::PositionContextMismatch);
        }
        if !before.state.conditions.accepts_time(now_us) {
            return Err(CarrierError::PlanConflict);
        }
        let stamp = self.issue_owner_work()?;
        let mut after = before.state.clone();
        let ticks = after.conditions.take_due(now_us, facts);
        let mut total = 0i64;
        let mut had_damage = false;
        let mut health_after = before.health;
        for tick in &ticks {
            match tick.kind {
                TickKind::Damage {
                    amount,
                    refused: false,
                    ..
                } => {
                    let mut healing = 0;
                    let mut dealt_percent = 100;
                    let mut attributed = tick.clone();
                    if let Some(source) = tick.provenance.source {
                        if let Some(lineage) = &tick.provenance.lineage
                            && (lineage.source_content_digest
                                != self.content_pin().server_artifact_digest()
                                || lineage.source_actor_placement != source.placement_identity()
                                || lineage.source_scope_generation
                                    != source.scope_generation().get())
                        {
                            return Err(CarrierError::PlanConflict);
                        }
                        let source_percent = match self.companion_snapshot_including_dead(source) {
                            Ok(snapshot) if snapshot.health <= 0 => None,
                            Ok(snapshot) => {
                                if self
                                    .companion_policy(&snapshot.state.policy.definition_key)?
                                    .as_ref()
                                    != snapshot.state.policy.as_ref()
                                {
                                    return Err(CarrierError::PlanConflict);
                                }
                                self.assert_actor_spell_unreserved(source)?;
                                let position = self.read_actor_position(source)?;
                                if position.context() != self.pinned_movement_context() {
                                    return Err(CarrierError::PositionContextMismatch);
                                }
                                use crate::foundation::condition::{ConditionValues, ConflictKey};
                                let source_store = self
                                    .actor_conditions_at(source, None, now_us)
                                    .map_err(|_| CarrierError::PlanConflict)?;
                                let native = snapshot
                                    .state
                                    .conditions
                                    .active_at(ConflictKey::Attributes, now_us);
                                let imported =
                                    source_store.active_at(ConflictKey::Attributes, now_us);
                                let chosen = match (native, imported) {
                                    (Some(a), Some(b))
                                        if a.definition() == b.definition()
                                            && a.provenance() == b.provenance() =>
                                    {
                                        Some(a)
                                    }
                                    (Some(_), Some(_)) => return Err(CarrierError::PlanConflict),
                                    (a, b) => a.or(b),
                                };
                                Some(match chosen.map(|a| a.definition().values()) {
                                    Some(ConditionValues::Attributes {
                                        damage_dealt_percent,
                                        ..
                                    }) => damage_dealt_percent,
                                    _ => 100,
                                })
                            }
                            Err(_) => {
                                if let Some(ref mut lookup) = player_source {
                                    lookup(self, source, now_us)?
                                } else if !self.borrow_exact_actor_lookup().contains(source) {
                                    None
                                } else {
                                    return Err(CarrierError::PlanConflict);
                                }
                            }
                        };
                        if let Some(percent) = source_percent {
                            let TickKind::Damage { element, .. } = tick.kind else {
                                return Err(CarrierError::PlanConflict);
                            };
                            if before.state.policy.flags.attackable {
                                healing = before.state.policy.damage_healing(
                                    periodic_element_key(element),
                                    i64::from(amount),
                                )?;
                            }
                            dealt_percent = percent;
                        }
                        attributed.provenance.source = None;
                        attributed.provenance.lineage = None;
                    } else if tick.provenance.lineage.is_some() {
                        return Err(CarrierError::PlanConflict);
                    }
                    let adjusted = (u64::from(amount) * u64::from(dealt_percent)) / 100;
                    let adjusted =
                        u32::try_from(adjusted).map_err(|_| CarrierError::DamageOverflow)?;
                    use crate::foundation::condition::{ConditionValues, ConflictKey};
                    let imported = self
                        .actor_conditions_at(actor, None, now_us)
                        .map_err(|_| CarrierError::PlanConflict)?;
                    let a = before
                        .state
                        .conditions
                        .active_at(ConflictKey::Attributes, now_us);
                    let b = imported.active_at(ConflictKey::Attributes, now_us);
                    let chosen = match (a, b) {
                        (Some(a), Some(b))
                            if a.definition() == b.definition()
                                && a.provenance() == b.provenance() =>
                        {
                            Some(a)
                        }
                        (Some(_), Some(_)) => return Err(CarrierError::PlanConflict),
                        (a, b) => a.or(b),
                    };
                    let received = match chosen.map(|a| a.definition().values()) {
                        Some(ConditionValues::Attributes {
                            incoming_reduction_percent,
                            ..
                        }) => 100u32
                            .checked_sub(incoming_reduction_percent)
                            .ok_or(CarrierError::PlanConflict)?,
                        _ => 100,
                    };
                    let damage = finish_condition_tick_with_received(
                        &before,
                        &attributed,
                        adjusted,
                        now_us,
                        Some(received),
                    )?;
                    had_damage |= damage > 0;
                    if damage > 0 {
                        after
                            .conditions
                            .remove_type(crate::foundation::condition::ConditionType::Invisible);
                    }
                    // Source combatBlockHit heals before residual damage; each
                    // real tick caps independently, and lethal never resurrects.
                    health_after = health_after
                        .saturating_add(healing)
                        .min(before.state.policy.maximum_health);
                    health_after = health_after
                        .checked_sub(damage)
                        .ok_or(CarrierError::DamageOverflow)?
                        .max(0);
                    total = before
                        .health
                        .checked_sub(health_after)
                        .ok_or(CarrierError::DamageOverflow)?;
                    if health_after == 0 {
                        after.conditions.clear_on_death();
                        break;
                    }
                }
                TickKind::Damage { refused: true, .. }
                | TickKind::Regeneration {
                    suppressed: true, ..
                }
                | TickKind::SpellRegeneration {
                    suppressed: true, ..
                } => {}
                // Creature regeneration needs its actual source vitals recipe,
                // and is not inferred from the player's regeneration rules.
                _ => return Err(CarrierError::PlanConflict),
            }
        }
        let index = self.carrier.validate_ref(&self.continuity, actor.0)?;
        let Slot::CreatureOccupied { committed, .. } = &self.carrier.slots[index] else {
            return Err(CarrierError::NotCreature);
        };
        let original_slot = self.carrier.slots[index].clone();
        let ordinal = committed.next_ordinal;
        let next_ordinal = if had_damage {
            ordinal
                .checked_add(1)
                .ok_or(CarrierError::CapacityArithmeticOverflow)?
        } else {
            ordinal
        };
        let lethal_record = if before.health > 0 && health_after == 0 {
            // The retained lethal source is the real store-issued owner input.
            // Nonlethal replay is fenced by complete store+HP comparison; no
            // unsequenced command receipt is consumed for every DOT pulse.
            if committed.entries.len() >= COMBAT01_DAMAGE_RECEIPTS_PER_CREATURE_GENERATION_MAX {
                return Err(CarrierError::DamageReceiptCapacityExceeded);
            }
            let occurrence = format!(
                "owner-tick/{}/{}/{}/{}",
                actor.actor_local_id(),
                actor.actor_local_generation(),
                stamp.generation().get(),
                stamp.ordinal().get()
            );
            let mut binding = occurrence.into_bytes();
            binding.push(0);
            binding.extend_from_slice(&actor.placement_identity());
            binding.extend_from_slice(&now_us.to_be_bytes());
            for tick in &ticks {
                binding.extend_from_slice(&tick.sequence.to_be_bytes());
                binding.extend_from_slice(&tick.due.to_be_bytes());
            }
            Some(OwnerCommitRecord {
                binding: copy_bounded_binding(&binding)?,
                damage: total,
                result: OwnerDamageResult {
                    applied: true,
                    health_before: before.health,
                    health_after,
                },
                ordinal,
                origin: None,
            })
        } else {
            None
        };
        Ok(PreparedCreaturePeriodicTurn {
            before,
            original_slot,
            after,
            health_after,
            stamp,
            ticks,
            next_ordinal,
            lethal_record,
        })
    }
    /// One real slot mutation installs consumed store state and physical HP.
    pub(crate) fn commit_creature_periodic_turn(
        &mut self,
        prepared: PreparedCreaturePeriodicTurn,
    ) -> Result<CreaturePeriodicReceipt, CarrierError> {
        if self.actor_spell_reserved(prepared.before.actor)
            || !self.owner_fence()?.accepts_stamp(prepared.stamp)
            || self.companion_snapshot(prepared.before.actor)? != prepared.before
        {
            return Err(CarrierError::PlanConflict);
        }
        if let Some(master) = prepared.before.state.master {
            self.carrier
                .player_slot_index(&self.continuity, master.actor.0, master.session)?;
        }
        let index = self
            .carrier
            .validate_ref(&self.continuity, prepared.before.actor.0)?;
        if self.carrier.slots.get(index) != Some(&prepared.original_slot) {
            return Err(CarrierError::PlanConflict);
        }
        let Slot::CreatureOccupied {
            health,
            companion,
            committed,
            ..
        } = &mut self.carrier.slots[index]
        else {
            return Err(CarrierError::NotCreature);
        };
        if let Some(record) = &prepared.lethal_record {
            if record.ordinal != committed.next_ordinal
                || committed.entries.len() >= COMBAT01_DAMAGE_RECEIPTS_PER_CREATURE_GENERATION_MAX
            {
                return Err(CarrierError::PlanConflict);
            }
            committed
                .entries
                .try_reserve(1)
                .map_err(|_| CarrierError::CapacityArithmeticOverflow)?;
        }
        // Box allocation precedes HP/store/ordinal installation.
        let next = Box::new(prepared.after);
        if let Some(record) = prepared.lethal_record {
            committed.entries.push(record);
        }
        committed.next_ordinal = prepared.next_ordinal;
        *health = prepared.health_after;
        if prepared.health_after == 0 {
            committed.conditions.die();
        }
        *companion = Some(next);
        Ok(CreaturePeriodicReceipt {
            actor: prepared.before.actor,
            health_before: prepared.before.health,
            health_after: prepared.health_after,
            ticks: prepared.ticks,
        })
    }
}

fn periodic_element_key(element: crate::foundation::condition::DotElement) -> &'static str {
    use crate::foundation::condition::DotElement;
    match element {
        DotElement::Poison => "earth",
        DotElement::Fire => "fire",
        DotElement::Energy => "energy",
        DotElement::Bleeding => "physical",
        DotElement::Drown => "drowning",
        DotElement::Freezing => "ice",
        DotElement::Dazzled => "holy",
        DotElement::Cursed => "death",
    }
}
/// ORIGIN_CONDITION has neither armor/defense nor critical/fatal/pierce draws.
/// Attributed ticks additionally require the real live source owner; absence
/// of that qualification cannot silently turn a known caster into no attacker.
fn finish_unattributed_condition_tick(
    target: &CompanionSnapshot,
    tick: &ConditionTick<ExactActorRef>,
    amount: u32,
    now_us: u64,
) -> Result<i64, CarrierError> {
    finish_condition_tick_with_received(target, tick, amount, now_us, None)
}
fn finish_condition_tick_with_received(
    target: &CompanionSnapshot,
    tick: &ConditionTick<ExactActorRef>,
    amount: u32,
    now_us: u64,
    received_override: Option<u32>,
) -> Result<i64, CarrierError> {
    use crate::foundation::condition::{ConditionValues, ConflictKey};
    if tick.provenance.source.is_some() || tick.provenance.lineage.is_some() {
        return Err(CarrierError::PlanConflict);
    }
    let TickKind::Damage { element, .. } = tick.kind else {
        return Err(CarrierError::PlanConflict);
    };
    let key = periodic_element_key(element);
    let policy = &target.state.policy;
    if !policy.flags.attackable || policy.damage_immunities.iter().any(|v| v == key) {
        return Ok(0);
    }
    let mut value = i64::from(amount);
    let mut received_percent = 100;
    if let Some(attributes) = target
        .state
        .conditions
        .active_at(ConflictKey::Attributes, now_us)
        && let ConditionValues::Attributes {
            incoming_reduction_percent,
            ..
        } = attributes.definition().values()
    {
        received_percent = 100 - incoming_reduction_percent;
        value = (value as f64 * f64::from(received_percent) / 100.0).trunc() as i64;
    }
    if let Some(current) = received_override {
        if current > 100 {
            return Err(CarrierError::PlanConflict);
        }
        received_percent = current;
        value = (i64::from(amount) as f64 * f64::from(current) / 100.0).trunc() as i64;
    }
    let mitigation = policy.mitigation.ok_or(CarrierError::PlanConflict)?;
    if mitigation.denominator == 0 || mitigation.numerator < 0 {
        return Err(CarrierError::PlanConflict);
    }
    let mitigation =
        ((mitigation.numerator as f64 / mitigation.denominator as f64) as f32).min(30.0);
    // Source int32 * float happens before promotion by the double /100 literal.
    value = (value as f64 - f64::from(value as f32 * mitigation) / 100.0).trunc() as i64;
    if let Some(resistance) = policy.resistances.iter().find(|v| v.damage_type == key) {
        if resistance.percent.denominator == 0 {
            return Err(CarrierError::PlanConflict);
        }
        let sensitivity =
            100.0 - resistance.percent.numerator as f64 / resistance.percent.denominator as f64;
        let result = (value as f64 * sensitivity / 100.0).round();
        if !result.is_finite() || result >= i64::MAX as f64 {
            return Err(CarrierError::DamageOverflow);
        }
        value = (result as i64).max(0);
    }
    // Canary combatChangeHealth applies the received buff again after
    // combatBlockHit; ORIGIN_CONDITION is retained through both calls.
    value = (value as f64 * f64::from(received_percent) / 100.0).trunc() as i64;
    Ok(value.max(0))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
    use super::super::runtime_actor_companion::{CreatureExactRatio, CreatureFlags};
    use super::*;
    use crate::foundation::condition::{
        ApplicationFacts, ConditionDefinition, ConditionSourceKind, ConditionValues, DotElement,
    };
    use crate::foundation::{CompiledCreaturePolicies, CompiledCreaturePolicy};
    use oteryn_simulation_determinism::{DecisionOccurrenceId, GameplayDecisionRoot};
    fn id(n: u8) -> [u8; 16] {
        [n, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, n]
    }
    fn policy(name: &str) -> CompiledCreaturePolicy {
        CompiledCreaturePolicy {
            definition_key: format!("creature:{name}"),
            definition_revision: "test-1".into(),
            display_name: name.into(),
            maximum_health: 812,
            base_speed: 220,
            outfit_look_type: 991,
            object_look_type: None,
            summonable: true,
            convinceable: true,
            mana_cost: Some(250),
            is_familiar: false,
            condition_immunities: Vec::new(),
            preferred_distance: Some(1),
            reward_boss: Some(false),
            armor: Some(10),
            mitigation: Some(CreatureExactRatio {
                numerator: 0,
                denominator: 1,
            }),
            resistances: vec![],
            damage_immunities: vec![],
            healing_from_damage: vec![],
            flags: CreatureFlags {
                attackable: true,
                illusionable: false,
                health_hidden: false,
            },
        }
    }
    fn owner() -> (ChannelRuntimeV1, ExactActorRef, GameSessionId) {
        let world = WorldId::decode(&id(1)).unwrap();
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            world,
            ChannelId::decode(&id(2)).unwrap(),
            NodeId::decode(&id(3)).unwrap(),
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            4,
            ChannelContentPin::test(world),
        )
        .unwrap();
        let session = GameSessionId::decode(&id(4)).unwrap();
        let reserved = runtime.reserve_fresh_session(session).unwrap();
        let actor = runtime.commit_fresh_session(reserved).unwrap();
        runtime.initialize_first_entry_position(actor).unwrap();
        runtime
            .install_companion_policies(
                CompiledCreaturePolicies::from_active_artifact([1; 32], vec![policy("rat")])
                    .unwrap(),
            )
            .unwrap();
        (runtime, actor, session)
    }

    fn install_dot(
        runtime: &mut ChannelRuntimeV1,
        creature: ExactActorRef,
        amount: u32,
        repetitions: u32,
    ) {
        let before = runtime.companion_snapshot(creature).unwrap();
        let mut after = before.state.clone();
        let definition = ConditionDefinition::new(
            "test.source.dot",
            1,
            ConditionValues::DamageOverTime {
                element: DotElement::Fire,
                total_min: amount * repetitions,
                total_max: amount * repetitions,
                per_tick: amount,
                interval_ms: 1000,
                delayed: true,
            },
        )
        .unwrap();
        let root = GameplayDecisionRoot::from_bytes([7; 32]);
        after
            .conditions
            .apply(
                &definition,
                None,
                ConditionSourceKind::Field,
                &[],
                &ApplicationFacts {
                    now: 0,
                    base_speed: 220,
                    mana_shield_capacity: 0,
                    target_reentry_protected: false,
                    source_reentry_protected: false,
                    target_is_player: false,
                    decision_root: &root,
                    occurrence: DecisionOccurrenceId::from_bytes([5; 16]),
                },
            )
            .unwrap();
        runtime.compare_companion_state(&before, after).unwrap();
    }
    #[test]
    fn physical_dot_more_than_sixteen_pulses_uses_actual_store_and_refuses_stale_tick_seal() {
        let (mut runtime, owner, session) = owner();
        let creature = runtime
            .create_companion(
                owner,
                session,
                "rat",
                MovementLocalPosition {
                    x: 1,
                    y: 0,
                    floor: 0,
                },
                None,
                None,
                0,
            )
            .unwrap();
        install_dot(&mut runtime, creature, 1, 20);
        let first = runtime
            .stage_creature_periodic_turn(creature, 1_000_000, TickFacts::default())
            .unwrap();
        let stale = runtime
            .stage_creature_periodic_turn(creature, 1_000_000, TickFacts::default())
            .unwrap();
        let result = runtime.commit_creature_periodic_turn(first).unwrap();
        assert_eq!((result.health_before, result.health_after), (812, 811));
        assert_eq!(result.ticks.len(), 1);
        assert!(runtime.commit_creature_periodic_turn(stale).is_err());
        for second in 2..=20 {
            let prepared = runtime
                .stage_creature_periodic_turn(creature, second * 1_000_000, TickFacts::default())
                .unwrap();
            runtime.commit_creature_periodic_turn(prepared).unwrap();
        }
        let current = runtime.companion_snapshot(creature).unwrap();
        assert_eq!(current.health, 792);
        assert!(current.state.conditions.instances().is_empty());
    }
    #[test]
    fn actual_periodic_lethal_tick_retains_genuine_corpse_receipt_without_wire_command() {
        let (mut runtime, owner, session) = owner();
        let creature = runtime
            .create_companion(
                owner,
                session,
                "rat",
                MovementLocalPosition {
                    x: 1,
                    y: 0,
                    floor: 0,
                },
                None,
                None,
                0,
            )
            .unwrap();
        install_dot(&mut runtime, creature, 1000, 1);
        let prepared = runtime
            .stage_creature_periodic_turn(creature, 1_000_000, TickFacts::default())
            .unwrap();
        let receipt = runtime.commit_creature_periodic_turn(prepared).unwrap();
        assert_eq!(receipt.health_after, 0);
        assert!(
            runtime
                .borrow_combat_death()
                .committed_lethal_receipt(creature)
                .is_ok()
        );
        assert!(
            runtime
                .stage_creature_periodic_turn(creature, 2_000_000, TickFacts::default())
                .is_err()
        );
    }
    #[test]
    fn periodic_source_policy_finisher_uses_mitigation_resistance_and_refuses_unknown() {
        let (mut runtime, owner, session) = owner();
        let creature = runtime
            .create_companion(
                owner,
                session,
                "rat",
                MovementLocalPosition {
                    x: 1,
                    y: 0,
                    floor: 0,
                },
                None,
                None,
                0,
            )
            .unwrap();
        install_dot(&mut runtime, creature, 100, 1);
        let mut target = runtime.companion_snapshot(creature).unwrap();
        let tick = target
            .state
            .conditions
            .take_due(1_000_000, TickFacts::default())
            .remove(0);
        let mut policy = (*target.state.policy).clone();
        policy.mitigation = Some(CreatureExactRatio {
            numerator: 10,
            denominator: 1,
        });
        policy.resistances = vec![super::super::runtime_actor_companion::CreatureResistance {
            damage_type: "fire".into(),
            percent: CreatureExactRatio {
                numerator: 50,
                denominator: 1,
            },
        }];
        target.state.policy = std::sync::Arc::new(policy.clone());
        assert_eq!(
            finish_unattributed_condition_tick(&target, &tick, 100, 1_000_000).unwrap(),
            45
        );
        policy.mitigation = None;
        target.state.policy = std::sync::Arc::new(policy.clone());
        assert!(finish_unattributed_condition_tick(&target, &tick, 100, 1_000_000).is_err());
        policy.healing_from_damage =
            vec![super::super::runtime_actor_companion::CreatureResistance {
                damage_type: "fire".into(),
                percent: CreatureExactRatio {
                    numerator: 300,
                    denominator: 1,
                },
            }];
        policy.mitigation = Some(CreatureExactRatio {
            numerator: 10,
            denominator: 1,
        });
        target.state.policy = std::sync::Arc::new(policy.clone());
        assert_eq!(
            finish_unattributed_condition_tick(&target, &tick, 100, 1_000_000).unwrap(),
            45,
            "actual source-free condition damage ignores healingMap without attacker"
        );
        policy.damage_immunities.push("fire".into());
        target.state.policy = std::sync::Arc::new(policy);
        assert_eq!(
            finish_unattributed_condition_tick(&target, &tick, 100, 1_000_000).unwrap(),
            0
        );
        let mut known_source = tick;
        known_source.provenance.source = Some(owner);
        assert!(
            finish_unattributed_condition_tick(&target, &known_source, 100, 1_000_000).is_err()
        );
    }
}
