#![allow(clippy::expect_used)]
use super::timed_item::{
    TIMEDITEM0_RL_01_CHARGES_MAX, TIMEDITEM0_RL_02_REMAINING_MS_MAX, TimedItemLane, TimedValues,
};
use oteryn_test_support::property::for_each_case;

#[test]
fn property_timed_item_charges_stop_at_zero() {
    for_each_case("item charges", |rng, _| {
        let initial = 1 + rng.below(128) as u32;
        let mut lane = TimedItemLane::live_from_row(
            1,
            TimedValues::new(Some(initial), None).expect("bounded charges"),
            0,
        );
        for spent in 0..initial {
            assert!(lane.spend_charge());
            assert_eq!(lane.live_values().charges(), Some(initial - spent - 1));
        }
        let exhausted = lane.clone();
        for _ in 0..3 {
            assert!(!lane.spend_charge());
        }
        lane.advance(u64::MAX);
        assert_eq!(lane, exhausted);
        Ok(())
    });
}

#[test]
fn property_timed_item_duration_clamps_and_elapsed_partition_is_equivalent() {
    for_each_case("item duration", |rng, _| {
        let remaining = rng.below(TIMEDITEM0_RL_02_REMAINING_MS_MAX + 1);
        let values = TimedValues::new(Some(TIMEDITEM0_RL_01_CHARGES_MAX), Some(remaining))
            .expect("bounded values");
        let random = rng.u64();
        let partial = rng.below(remaining + 1);
        let first = *rng.pick(&[
            0,
            1,
            partial,
            remaining,
            remaining.saturating_add(1),
            random,
        ]);
        let second = *rng.pick(&[0, 1, partial, remaining / 2, u64::MAX]);
        let mut split = TimedItemLane::live_from_row(1, values, 0);
        let mut combined = split.clone();
        split.advance(first);
        split.advance(second);
        combined.advance(first.saturating_add(second));
        assert_eq!(split.live_values(), combined.live_values());
        assert_eq!(
            split.live_values().remaining_ms(),
            Some(remaining.saturating_sub(first).saturating_sub(second))
        );
        assert_eq!(
            split.live_values().charges(),
            Some(TIMEDITEM0_RL_01_CHARGES_MAX)
        );
        assert!(TimedValues::new(Some(0), None).is_err());
        assert!(TimedValues::new(Some(TIMEDITEM0_RL_01_CHARGES_MAX + 1), None).is_err());
        assert!(TimedValues::new(None, Some(TIMEDITEM0_RL_02_REMAINING_MS_MAX + 1)).is_err());
        Ok(())
    });
}
