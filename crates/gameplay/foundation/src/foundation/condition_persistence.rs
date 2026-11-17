//! Crystal condition.cpp:356-370,1680-1694: finite DEFAULT/COMBAT conditions and
//! the unspent DOT list survive logout; attacker authority does not survive login.
use super::*;

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct SavedConditions {
    schema: u8,
    conditions: Vec<ConditionInstance<String>>,
}

impl ConditionStore<String> {
    /// Freeze online timers as remaining microseconds, without advancing damage or RNG.
    /// Ephemeral suppression/immunity and permanent appearances are rebuilt by their owners.
    pub fn persistent_snapshot(&self, now: u64) -> Result<serde_json::Value, ConditionRefusal> {
        if !self.accepts_time(now) {
            return Err(ConditionRefusal::ArithmeticBounds);
        }
        let mut conditions = Vec::new();
        for instance in &self.instances {
            if instance.definition.permanent_outfit
                || instance.ends_at.is_some_and(|end| end <= now)
            {
                continue;
            }
            let mut saved = instance.clone();
            saved.elapsed_before_start = instance.age_at(now);
            saved.started_at = 0;
            saved.tick_debt_us = instance
                .tick_debt_us
                .saturating_add(instance.next_tick_at.map_or(0, |at| now.saturating_sub(at)));
            saved.immediate_debt_us = instance.immediate_debt_us.saturating_add(
                instance
                    .immediate_due
                    .map_or(0, |at| now.saturating_sub(at)),
            );
            saved.ends_at = instance.ends_at.map(|at| at.saturating_sub(now));
            saved.next_tick_at = instance.next_tick_at.map(|at| at.saturating_sub(now));
            saved.immediate_due = instance.immediate_due.map(|at| at.saturating_sub(now));
            // Crystal does not serialize ConditionDamage::owner. Retain descriptive
            // provenance, but a prior session can never authorize damage attribution.
            saved.provenance.source_kind = ConditionSourceKind::Restored;
            saved.provenance.source = None;
            saved.provenance.lineage = None;
            // Logout deliberately drops historical attacker display as well as
            // authority. Old schema1 snapshots omit this optional field unchanged.
            saved.provenance.historical_source = None;
            conditions.push(saved);
        }
        serde_json::to_value(SavedConditions {
            schema: 1,
            conditions,
        })
        .map_err(|_| ConditionRefusal::ArithmeticBounds)
    }

    /// Admission calls once, after independently proving the fresh actor/session/lease.
    /// Rebase onto its new process clock; offline time consumes no condition ticks.
    pub fn from_persistent_snapshot(
        value: &serde_json::Value,
        now: u64,
    ) -> Result<Self, ConditionRefusal> {
        let saved: SavedConditions = serde_json::from_value(value.clone())
            .map_err(|_| ConditionRefusal::ArithmeticBounds)?;
        if saved.schema != 1 || saved.conditions.len() > COND0_RL_01_INSTANCES_PER_ACTOR {
            return Err(ConditionRefusal::ArithmeticBounds);
        }
        let mut result = Self::new();
        result.pass_at = now;
        for mut instance in saved.conditions {
            let definition = &instance.definition;
            let valid = match (&definition.values, &definition.damage_schedule) {
                (ConditionValues::DamageSchedule { .. }, Some(schedule)) => {
                    schedule.validate() && valid_atom(&definition.key) && definition.revision > 0
                }
                (_, None) => ConditionDefinition::new(
                    &definition.key,
                    definition.revision,
                    definition.values,
                )
                .is_some(),
                _ => false,
            };
            let damage = matches!(
                definition.condition_type(),
                ConditionType::DamageOverTime(_) | ConditionType::Agony
            );
            let scheduled = matches!(definition.values, ConditionValues::DamageSchedule { .. });
            let key = shared_mutation_slot(definition.condition_type().conflict_key());
            if !valid
                || definition.permanent_outfit
                || instance.started_at != 0
                || damage != instance.ends_at.is_none()
                || (damage && (instance.remaining_total == 0 || instance.next_tick_at.is_none()))
                || scheduled != instance.scheduled_damage.is_some()
                || instance.ends_at == Some(0)
                || (instance.tick_debt_us > 0 && instance.next_tick_at != Some(0))
                || (instance.immediate_debt_us > 0 && instance.immediate_due != Some(0))
                || instance.regeneration_phase.is_some_and(|phase| {
                    !matches!(definition.values, ConditionValues::SpellRegeneration { .. })
                        || instance.next_tick_at.is_none()
                        || phase.health_last_age_us > instance.elapsed_before_start
                        || phase.mana_last_age_us > instance.elapsed_before_start
                })
                || !instance.provenance.is_unattributed()
                || instance.provenance.definition_key != definition.key
                || instance.provenance.definition_revision != definition.revision
                || result.instances.iter().any(|i| {
                    shared_mutation_slot(i.definition.condition_type().conflict_key()) == key
                })
                || instance
                    .scheduled_damage
                    .as_ref()
                    .is_some_and(|c| !c.valid() || c.remaining != instance.remaining_total)
            {
                return Err(ConditionRefusal::ArithmeticBounds);
            }
            if let ConditionValues::DamageSequence { steps, len, .. } = definition.values
                && (instance.sequence_step >= len
                    || instance.sequence_repetition
                        >= steps[usize::from(instance.sequence_step)].repetitions)
            {
                return Err(ConditionRefusal::ArithmeticBounds);
            }
            let rebase = |at: u64| {
                now.checked_add(at)
                    .ok_or(ConditionRefusal::ArithmeticBounds)
            };
            instance.started_at = now;
            instance.ends_at = instance.ends_at.map(rebase).transpose()?;
            instance.next_tick_at = instance.next_tick_at.map(rebase).transpose()?;
            instance.immediate_due = instance.immediate_due.map(rebase).transpose()?;
            result.next_sequence = result.next_sequence.max(
                instance
                    .sequence
                    .checked_add(1)
                    .ok_or(ConditionRefusal::ArithmeticBounds)?,
            );
            result.instances.push(instance);
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn c08_persistent_conditions_rebase_ticks_light_and_regeneration_phase()
    -> Result<(), ConditionRefusal> {
        let root = GameplayDecisionRoot::from_bytes([7; 32]);
        let at = 10_000_000;
        let facts = ApplicationFacts {
            now: at,
            base_speed: 220,
            mana_shield_capacity: 500,
            target_reentry_protected: false,
            source_reentry_protected: false,
            target_is_player: true,
            decision_root: &root,
            occurrence: DecisionOccurrenceId::from_bytes([7; 16]),
        };
        let mut store = ConditionStore::<String>::new();
        let definitions = [
            ConditionValues::DamageOverTime {
                element: DotElement::Poison,
                total_min: 100,
                total_max: 100,
                per_tick: 5,
                interval_ms: 5_000,
                delayed: true,
            },
            ConditionValues::SpellRegeneration {
                duration_ms: 60_000,
                sub_id: 0,
                health_gain: 3,
                health_interval_ms: 2_000,
                mana_gain: 2,
                mana_interval_ms: 3_000,
            },
            ConditionValues::SpellLight {
                level: 6,
                color: 181,
                duration_ms: 60_000,
            },
            ConditionValues::Speed {
                paralysis: false,
                range: SpeedRange {
                    a_min: 1000,
                    a_max: 1000,
                    b_min: 100_000,
                    b_max: 100_000,
                },
                duration_ms: 60_000,
            },
        ];
        for (i, values) in definitions.into_iter().enumerate() {
            let definition = ConditionDefinition::new(&format!("test.c08.{i}"), 1, values)
                .ok_or(ConditionRefusal::ArithmeticBounds)?;
            store.apply(
                &definition,
                Some("old-session-attacker".into()),
                ConditionSourceKind::Creature,
                &[],
                &facts,
            )?;
        }
        let saved_at = at + 13_000_000;
        for pass in 0..4 {
            store.take_due(saved_at - 4 + pass, TickFacts::default());
        }
        let value = store.persistent_snapshot(saved_at)?;
        // New process semantic clock starts close to zero, even after a long logout.
        let mut restored = ConditionStore::from_persistent_snapshot(&value, 100)?;
        assert_eq!(restored.light_at(100), store.light_at(saved_at));
        assert_eq!(restored.speed_delta_at(100), store.speed_delta_at(saved_at));
        assert_eq!(restored.instances().len(), 4);
        assert!(
            restored
                .instances()
                .iter()
                .all(|i| i.provenance().source.is_none())
        );
        assert!(
            restored
                .take_due(100 + 999_999, TickFacts::default())
                .is_empty()
        );
        let ticks = restored.take_due(100 + 1_000_000, TickFacts::default());
        assert_eq!(ticks.len(), 1);
        assert_eq!(
            ticks[0].kind,
            TickKind::SpellRegeneration {
                health_gain: 3,
                mana_gain: 0,
                suppressed: false
            }
        );
        let ticks = restored.take_due(100 + 2_000_000, TickFacts::default());
        assert_eq!(ticks.len(), 2);
        assert!(ticks.iter().any(|t| matches!(
            t.kind,
            TickKind::Damage {
                element: DotElement::Poison,
                amount: 5,
                ..
            }
        )));
        assert!(ticks.iter().any(|t| t.kind
            == TickKind::SpellRegeneration {
                health_gain: 0,
                mana_gain: 2,
                suppressed: false
            }));
        Ok(())
    }

    #[test]
    fn c08_persistent_conditions_keep_overdue_order_and_compressed_damage_cursor()
    -> Result<(), ConditionRefusal> {
        let root = GameplayDecisionRoot::from_bytes([8; 32]);
        let facts = ApplicationFacts {
            now: 1_000_000,
            base_speed: 220,
            mana_shield_capacity: 0,
            target_reentry_protected: false,
            source_reentry_protected: false,
            target_is_player: true,
            decision_root: &root,
            occurrence: DecisionOccurrenceId::from_bytes([8; 16]),
        };
        let mut original = ConditionStore::<String>::new();
        let definition = ConditionDefinition::new_damage_schedule(
            "test.c08.cursor",
            1,
            DotElement::Poison,
            DamageSchedule::Fixed {
                delayed: true,
                segments: vec![
                    DamageSegment {
                        count: 2,
                        amount: 7,
                        interval_ms: 5_000,
                    },
                    DamageSegment {
                        count: 3,
                        amount: 4,
                        interval_ms: 2_000,
                    },
                ],
            },
        )
        .ok_or(ConditionRefusal::ArithmeticBounds)?;
        original.apply(
            &definition,
            None,
            ConditionSourceKind::Creature,
            &[],
            &facts,
        )?;
        let definition = ConditionDefinition::new(
            "test.c08.regen",
            1,
            ConditionValues::SpellRegeneration {
                duration_ms: 60_000,
                sub_id: 0,
                health_gain: 3,
                health_interval_ms: 2_000,
                mana_gain: 2,
                mana_interval_ms: 3_000,
            },
        )
        .ok_or(ConditionRefusal::ArithmeticBounds)?;
        original.apply(&definition, None, ConditionSourceKind::SelfUse, &[], &facts)?;
        let saved_at = 14_000_000;
        let value = original.persistent_snapshot(saved_at)?;
        let mut restored = ConditionStore::from_persistent_snapshot(&value, 0)?;
        for pass in 0..4_u64 {
            let old_ticks = original.take_due(saved_at + pass, TickFacts::default());
            let new_ticks = restored.take_due(pass, TickFacts::default());
            assert_eq!(
                old_ticks.iter().map(|t| &t.kind).collect::<Vec<_>>(),
                new_ticks.iter().map(|t| &t.kind).collect::<Vec<_>>()
            );
        }
        let poison = original
            .get(ConflictKey::Element(DotElement::Poison))
            .ok_or(ConditionRefusal::ArithmeticBounds)?;
        let loaded = restored
            .get(ConflictKey::Element(DotElement::Poison))
            .ok_or(ConditionRefusal::ArithmeticBounds)?;
        assert_eq!(loaded.remaining_total(), poison.remaining_total());
        Ok(())
    }

    #[test]
    fn c08_persistent_conditions_reject_corrupt_schema_and_instance() -> Result<(), ConditionRefusal>
    {
        let empty = ConditionStore::<String>::new().persistent_snapshot(0)?;
        assert!(ConditionStore::from_persistent_snapshot(&empty, u64::MAX).is_ok());
        for bad in [
            serde_json::json!({"schema":2,"conditions":[]}),
            serde_json::json!({"schema":1,"conditions":[{}]}),
        ] {
            assert!(ConditionStore::from_persistent_snapshot(&bad, 0).is_err());
        }
        Ok(())
    }
}
