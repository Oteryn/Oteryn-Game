use oteryn_gameplay_world_change::timed::{HOUR, IntervalClock};

#[test]
fn wevents_mid_epoch_draw_is_once_per_slot_and_inside_window() {
    let mut clock = IntervalClock::default();
    assert_eq!(clock.check(100, 4 * HOUR, 99, HOUR, 10, 0), Ok(false));
    assert_eq!(
        clock.check(100, 4 * HOUR, 100, HOUR, 10, u64::MAX),
        Ok(false)
    );
    assert_eq!(clock.check(100, 4 * HOUR, 101, HOUR, 10, 0), Ok(false));
    assert_eq!(
        clock.check(100, 4 * HOUR, HOUR + 100, HOUR, 10, 0),
        Ok(true)
    );
    assert_eq!(
        clock.check(100, 4 * HOUR, HOUR + 100, HOUR, 10, 0),
        Ok(false)
    );
    let encoded = serde_json::to_vec(&clock).unwrap_or_default();
    let mut restored: IntervalClock = serde_json::from_slice(&encoded).unwrap_or_default();
    assert_eq!(
        restored.check(100, 4 * HOUR, HOUR + 100, HOUR, 10, 0),
        Ok(false)
    );
    assert_eq!(
        restored.check(100, 4 * HOUR, 3 * HOUR + 100, HOUR, 0, 0),
        Ok(false)
    );
    assert_eq!(
        restored.check(100, 4 * HOUR, 4 * HOUR, HOUR, 1000, 0),
        Ok(false)
    );
    assert!(
        restored
            .check(100, 4 * HOUR, 3 * HOUR + 100, 0, 10, 0)
            .is_err()
    );
}
