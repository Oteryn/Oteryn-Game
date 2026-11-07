use super::*;

#[test]
fn agony_ramp_spends_each_donor_tick_and_expires() -> Result<(), ConditionRefusal> {
    let definition = ConditionDefinition::new(
        "condition.agony",
        1,
        ConditionValues::AgonyRamp {
            start: 260,
            repetitions: 260,
            interval_ms: 5_000,
            delayed: false,
        },
    )
    .ok_or(ConditionRefusal::ArithmeticBounds)?;
    let root = GameplayDecisionRoot::from_bytes([7; 32]);
    let mut store = ConditionStore::<u32>::new();
    store.apply(
        &definition,
        Some(1),
        ConditionSourceKind::Field,
        &[],
        &ApplicationFacts {
            now: 0,
            base_speed: 220,
            mana_shield_capacity: 500,
            target_reentry_protected: false,
            source_reentry_protected: false,
            target_is_player: true,
            decision_root: &root,
            occurrence: DecisionOccurrenceId::from_bytes([1; 16]),
        },
    )?;
    let mut remaining = (1..=260).sum::<u32>();
    assert_eq!(store.instances()[0].remaining_total(), remaining);
    for tick in 0..260_u32 {
        let amount = 260 - tick;
        let ticks = store.take_due(u64::from(tick) * 5_000_000, TickFacts::default());
        assert_eq!(ticks.len(), 1);
        assert_eq!(
            ticks[0].kind,
            TickKind::Agony {
                amount,
                refused: false
            }
        );
        remaining -= amount;
        if remaining > 0 {
            assert_eq!(store.instances()[0].remaining_total(), remaining);
        }
    }
    assert!(store.instances().is_empty());
    Ok(())
}
