//! Read-only status durations. The projection never advances a condition or its RNG.
use super::*;

impl<S> ConditionInstance<S> {
    /// Finite DOT uses its remaining tick schedule; -1 is reserved for a permanent instance.
    /// The schedule is projected without future refreshes or standing-field retention.
    pub fn status_remaining_ms(&self, now: u64) -> Option<i64> {
        if self.started_at > now {
            return Some(0);
        }
        if let Some(end) = self.ends_at {
            return i64::try_from(end.saturating_sub(now).div_ceil(MICROS_PER_MS)).ok();
        }
        let span_ms = match self.definition.values {
            ConditionValues::DamageSchedule { .. } => {
                self.scheduled_damage.as_ref()?.remaining_span_ms()?
            }
            ConditionValues::DamageSequence { steps, len, .. } => {
                let current = steps.get(usize::from(self.sequence_step))?;
                let mut span = u64::from(
                    current
                        .repetitions
                        .checked_sub(self.sequence_repetition)?
                        .checked_sub(1)?,
                )
                .checked_mul(u64::from(current.interval_ms))?;
                for step in &steps[usize::from(self.sequence_step) + 1..usize::from(len)] {
                    span = span.checked_add(
                        u64::from(step.repetitions).checked_mul(u64::from(step.interval_ms))?,
                    )?;
                }
                span
            }
            ConditionValues::DamageOverTime {
                per_tick,
                interval_ms,
                ..
            }
            | ConditionValues::Agony {
                per_tick,
                interval_ms,
                ..
            } => u64::from(
                self.remaining_total
                    .checked_div(per_tick)?
                    .checked_add(u32::from(!self.remaining_total.is_multiple_of(per_tick)))?
                    .checked_sub(1)?,
            )
            .checked_mul(u64::from(interval_ms))?,
            ConditionValues::AgonyRamp { interval_ms, .. } => {
                u64::from(self.sequence_repetition.checked_sub(1)?)
                    .checked_mul(u64::from(interval_ms))?
            }
            _ => return Some(-1),
        };
        let due = [
            self.next_tick_at
                .map(|at| i128::from(at) - i128::from(self.tick_debt_us)),
            self.immediate_due
                .map(|at| i128::from(at) - i128::from(self.immediate_debt_us)),
        ]
        .into_iter()
        .flatten()
        .min()?;
        // Preserve negative logical deadlines when a restored process clock cannot
        // represent the overdue tick. Clamp only after adding the remaining schedule.
        let end = due + i128::from(span_ms) * i128::from(MICROS_PER_MS);
        let remaining_us = (end - i128::from(now)).max(1);
        // Retained overdue ticks are still active until their bounded owner turn drains them.
        i64::try_from((remaining_us + i128::from(MICROS_PER_MS - 1)) / i128::from(MICROS_PER_MS))
            .ok()
    }
}
impl<S: Clone> ConditionStore<S> {
    pub fn visible_statuses(&self, now: u64) -> impl Iterator<Item = (ConditionType, i64)> + '_ {
        self.instances.iter().filter_map(move |instance| {
            let kind = instance.definition.condition_type();
            if self.suppressed.contains(&kind) {
                return None;
            }
            let remaining = instance.status_remaining_ms(now)?;
            (remaining != 0).then_some((kind, remaining))
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn condition_status_reports_finite_damage_ticks_and_timed_expiry() -> Result<(), String> {
        let root = GameplayDecisionRoot::from_bytes([1; 32]);
        let facts = ApplicationFacts {
            now: 0,
            base_speed: 220,
            mana_shield_capacity: 0,
            target_reentry_protected: false,
            source_reentry_protected: false,
            target_is_player: true,
            decision_root: &root,
            occurrence: DecisionOccurrenceId::from_bytes([2; 16]),
        };
        let mut store = ConditionStore::<String>::new();
        let dot = ConditionDefinition::new(
            "status.poison",
            1,
            ConditionValues::DamageOverTime {
                element: DotElement::Poison,
                total_min: 30,
                total_max: 30,
                per_tick: 10,
                interval_ms: 2_000,
                delayed: true,
            },
        )
        .ok_or("dot")?;
        store
            .apply(&dot, None, ConditionSourceKind::Creature, &[], &facts)
            .map_err(|e| format!("{e:?}"))?;
        let instance = store.instances().first().ok_or("instance")?;
        assert_eq!(instance.status_remaining_ms(1_000_000), Some(5_000));
        assert_eq!(
            instance.remaining_total(),
            30,
            "projection must not consume damage"
        );
        let timed = ConditionDefinition::new(
            "status.rooted",
            1,
            ConditionValues::TimedStatus {
                kind: StatusKind::Rooted,
                duration_ms: 10_000,
            },
        )
        .ok_or("timed")?;
        store
            .apply(&timed, None, ConditionSourceKind::Creature, &[], &facts)
            .map_err(|e| format!("{e:?}"))?;
        let instance = store
            .get(ConflictKey::Status(StatusKind::Rooted))
            .ok_or("rooted")?;
        assert_eq!(instance.status_remaining_ms(1_000_000), Some(9_000));
        assert_eq!(instance.status_remaining_ms(10_000_000), Some(0));
        Ok(())
    }
    fn stored(definition: &ConditionDefinition) -> Result<ConditionStore<String>, String> {
        let root = GameplayDecisionRoot::from_bytes([1; 32]);
        let facts = ApplicationFacts {
            now: 0,
            base_speed: 220,
            mana_shield_capacity: 0,
            target_reentry_protected: false,
            source_reentry_protected: false,
            target_is_player: true,
            decision_root: &root,
            occurrence: DecisionOccurrenceId::from_bytes([2; 16]),
        };
        let mut store = ConditionStore::new();
        store
            .apply(definition, None, ConditionSourceKind::Creature, &[], &facts)
            .map_err(|e| format!("{e:?}"))?;
        Ok(store)
    }
    fn remaining(store: &ConditionStore<String>, now: u64) -> Option<i64> {
        store.instances().first()?.status_remaining_ms(now)
    }
    #[test]
    fn condition_status_schedule_and_sequence_preserve_segment_intervals_after_ticks()
    -> Result<(), String> {
        let schedule = ConditionDefinition::new_damage_schedule(
            "status.schedule",
            1,
            DotElement::Poison,
            DamageSchedule::Fixed {
                delayed: true,
                segments: vec![
                    DamageSegment {
                        count: 2,
                        amount: 10,
                        interval_ms: 1_000,
                    },
                    DamageSegment {
                        count: 2,
                        amount: 5,
                        interval_ms: 3_000,
                    },
                ],
            },
        )
        .ok_or("schedule")?;
        let mut steps = [DotSequenceStep::default(); MAX_DOT_SEQUENCE_STEPS];
        steps[0] = DotSequenceStep {
            repetitions: 2,
            amount: 10,
            interval_ms: 1_000,
        };
        steps[1] = DotSequenceStep {
            repetitions: 2,
            amount: 5,
            interval_ms: 3_000,
        };
        let sequence = ConditionDefinition::new(
            "status.sequence",
            1,
            ConditionValues::DamageSequence {
                element: DotElement::Poison,
                steps,
                len: 2,
                delayed: true,
            },
        )
        .ok_or("sequence")?;
        for definition in [&schedule, &sequence] {
            let mut store = stored(definition)?;
            assert_eq!(remaining(&store, 500_000), Some(7_500));
            assert_eq!(store.take_due(1_000_000, TickFacts::default()).len(), 1);
            assert_eq!(remaining(&store, 1_000_000), Some(7_000));
            assert_eq!(store.take_due(2_000_000, TickFacts::default()).len(), 1);
            assert_eq!(remaining(&store, 2_000_000), Some(6_000));
            assert_eq!(store.take_due(5_000_000, TickFacts::default()).len(), 1);
            assert_eq!(remaining(&store, 5_000_000), Some(3_000));
            assert_eq!(store.take_due(8_000_000, TickFacts::default()).len(), 1);
            assert!(store.visible_statuses(8_000_000).next().is_none());
        }
        Ok(())
    }
    #[test]
    fn condition_status_agony_remaining_counter_overdue_permanent_and_suppression()
    -> Result<(), String> {
        for values in [
            ConditionValues::AgonyRamp {
                start: 3,
                repetitions: 3,
                interval_ms: 2_000,
                delayed: false,
            },
            ConditionValues::Agony {
                total_min: 30,
                total_max: 30,
                per_tick: 10,
                interval_ms: 2_000,
                delayed: false,
            },
        ] {
            let definition = ConditionDefinition::new("status.agony", 1, values).ok_or("agony")?;
            let mut store = stored(&definition)?;
            assert_eq!(remaining(&store, 0), Some(4_000));
            assert_eq!(store.take_due(0, TickFacts::default()).len(), 1);
            assert_eq!(remaining(&store, 0), Some(4_000));
            assert_eq!(store.take_due(2_000_000, TickFacts::default()).len(), 1);
            assert_eq!(remaining(&store, 2_000_000), Some(2_000));
            assert_eq!(
                remaining(&store, 9_000_000),
                Some(1),
                "due backlog remains active"
            );
            let pending = store.instances()[0].remaining_total();
            let dealt = store
                .take_due(9_000_000, TickFacts::default())
                .iter()
                .map(|tick| match tick.kind {
                    TickKind::Agony {
                        amount,
                        refused: false,
                    } => amount,
                    _ => 0,
                })
                .sum::<u32>();
            assert_eq!(
                dealt, pending,
                "the bounded owner spends exactly the retained damage"
            );
            assert!(store.visible_statuses(9_000_000).next().is_none());
        }
        let permanent =
            ConditionDefinition::new_permanent_outfit("status.outfit", 1, 128).ok_or("outfit")?;
        let mut store = stored(&permanent)?;
        assert_eq!(remaining(&store, 100_000_000), Some(-1));
        assert_eq!(store.visible_statuses(100_000_000).count(), 1);
        assert_eq!(
            store.set_suppressed(std::collections::BTreeSet::from([ConditionType::Outfit])),
            1
        );
        assert!(store.visible_statuses(100_000_000).next().is_none());
        Ok(())
    }
    #[test]
    fn condition_status_restored_overdue_damage_keeps_its_logical_deadline() -> Result<(), String> {
        let definition = ConditionDefinition::new(
            "status.overdue",
            1,
            ConditionValues::DamageOverTime {
                element: DotElement::Poison,
                total_min: 30,
                total_max: 30,
                per_tick: 10,
                interval_ms: 2_000,
                delayed: true,
            },
        )
        .ok_or("dot")?;
        let store = stored(&definition)?;
        assert_eq!(remaining(&store, 9_000_000), Some(1));
        let saved = store
            .persistent_snapshot(9_000_000)
            .map_err(|e| format!("{e:?}"))?;
        let restored =
            ConditionStore::from_persistent_snapshot(&saved, 0).map_err(|e| format!("{e:?}"))?;
        assert_eq!(
            remaining(&restored, 0),
            Some(1),
            "rebasing preserves overdue schedule debt"
        );
        assert_eq!(
            restored.instances()[0].remaining_total(),
            30,
            "display never spends queued damage"
        );
        Ok(())
    }
}
