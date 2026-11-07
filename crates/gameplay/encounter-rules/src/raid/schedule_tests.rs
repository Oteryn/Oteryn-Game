use super::definition_tests::definition;
use super::*;
use oteryn_gameplay_content::content::ProductionKey;
use oteryn_protocol_oteryn::WorldId;

type Result = std::result::Result<(), Box<dyn std::error::Error>>;
fn id(seed: u8) -> [u8; 16] {
    let mut id = [seed; 16];
    id[6] = 0x70;
    id[8] = 0x80;
    id
}
fn plan(
    s: &RaidSchedule,
    d: &RaidDefinition,
    action: RaidAction,
) -> std::result::Result<RaidSchedule, Box<dyn std::error::Error>> {
    Ok(s.plan(
        WorldId::decode(&id(1))?,
        &ProductionKey::new("raid:badger")?,
        d,
        action,
    )?
    .after()
    .clone())
}
fn check(now_ms: i64) -> RaidAction {
    RaidAction::Check {
        now_ms,
        eligible: true,
        calendar: None,
        firing_id: id(2),
        bundle_digest: [3; 32],
    }
}

pub(super) fn persistent_once_per_epoch() -> Result {
    let d = definition(true)?;
    let s = plan(&RaidSchedule::initial(1, 0)?, &d, check(60_000))?;
    assert_eq!(
        s.latest_firing.as_ref().map(|f| f.state),
        Some(FiringState::Running)
    );
    let blocked = plan(&s, &d, check(120_000))?;
    assert_eq!(blocked.check_ordinal, 2);
    assert_eq!(blocked.next_firing_ordinal, 2);
    let done = plan(&blocked, &d, RaidAction::Close { now_ms: 120_000 })?;
    let blocked = plan(&done, &d, check(180_000))?;
    assert_eq!(blocked.next_firing_ordinal, 2);
    let reset = plan(
        &blocked,
        &d,
        RaidAction::Reset {
            reset_epoch: 2,
            now_ms: 180_000,
        },
    )?;
    let fired = plan(&reset, &d, check(240_000))?;
    assert_eq!(
        fired
            .latest_firing
            .as_ref()
            .map(|f| (f.ordinal, f.reset_epoch)),
        Some((2, 2))
    );
    assert_eq!(fired.check_ordinal, 4);
    Ok(())
}

pub(super) fn minimum_gap_survives_reset() -> Result {
    let mut d = definition(false)?;
    d.policy = RaidPolicy::Random {
        mean_interval_s: 60,
        min_gap_s: 180,
    };
    let fired = plan(&RaidSchedule::initial(1, 0)?, &d, check(60_000))?;
    let reset = plan(
        &fired,
        &d,
        RaidAction::Reset {
            reset_epoch: 2,
            now_ms: 60_000,
        },
    )?;
    assert_eq!(
        reset.latest_firing.as_ref().map(|f| f.state),
        Some(FiringState::Cancelled)
    );
    let blocked = plan(&reset, &d, check(239_999))?;
    assert_eq!(blocked.next_firing_ordinal, 2);
    let fired = plan(&blocked, &d, check(299_999))?;
    assert_eq!(fired.next_firing_ordinal, 3);
    Ok(())
}

#[test]
fn raid_due_checks_are_once_and_downtime_is_not_caught_up() -> Result {
    let d = definition(false)?;
    let initial = RaidSchedule::initial(1, 0)?;
    assert_eq!(plan(&initial, &d, check(59_999))?, initial);
    let s = plan(&initial, &d, check(10_000_000))?;
    assert_eq!(s.check_ordinal, 1);
    assert_eq!(s.next_check_at_ms, 10_060_000);
    assert_eq!(plan(&s, &d, check(10_000_000))?, s);
    assert_eq!(plan(&s, &d, RaidAction::Close { now_ms: 10_002_999 })?, s);
    assert_eq!(
        plan(&s, &d, RaidAction::Close { now_ms: 10_003_000 })?
            .latest_firing
            .map(|f| f.state),
        Some(FiringState::Done)
    );
    let retired = plan(&s, &d, RaidAction::Retire)?;
    assert_eq!(plan(&retired, &d, check(11_000_000))?, retired);
    Ok(())
}

#[test]
fn raid_fixed_calendar_mismatch_missed_slots_and_once_only() -> Result {
    for selector in [CalendarSelector::Weekday(1), CalendarSelector::MonthDay(10)] {
        let mut d = definition(false)?;
        d.policy = RaidPolicy::Fixed {
            selector,
            offset_after_reset_ms: 60_000,
        };
        let initial = RaidSchedule::initial(1, 0)?;
        let action = |now_ms, weekday, month_day, eligible| RaidAction::Check {
            now_ms,
            eligible,
            calendar: Some(CalendarDay {
                reset_at_ms: 0,
                weekday,
                month_day,
            }),
            firing_id: id(2),
            bundle_digest: [3; 32],
        };
        let mismatch = plan(&initial, &d, action(60_000, 2, 11, true))?;
        assert_eq!(mismatch.check_ordinal, 1);
        assert_eq!(mismatch.latest_firing, None);
        let missed = plan(&initial, &d, action(120_000, 1, 10, true))?;
        assert_eq!(missed.latest_firing, None);
        let fired = plan(&initial, &d, action(119_999, 1, 10, true))?;
        assert!(fired.latest_firing.is_some());
        let closed = plan(&fired, &d, RaidAction::Close { now_ms: 179_999 })?;
        assert_eq!(
            plan(&closed, &d, action(179_999, 1, 10, true))?.next_firing_ordinal,
            2
        );
        let blocked = plan(&initial, &d, action(60_000, 1, 10, false))?;
        assert_eq!(blocked.last_fixed_slot_ms, Some(60_000));
        assert_eq!(
            plan(&blocked, &d, action(120_000, 1, 10, true))?.latest_firing,
            None
        );
        assert!(plan(&initial, &d, check(60_000)).is_err());
    }
    Ok(())
}

#[test]
fn raid_random_draw_is_framed_reproducible_and_bounded() -> Result {
    let key = ProductionKey::new("raid:badger")?;
    let world = WorldId::decode(&id(1))?;
    for interval in [1, 60, 61, 30_000, u32::MAX] {
        let draws = (0..1_000)
            .map(|n| super::schedule::random_selected(world, &key, n, interval))
            .collect::<std::result::Result<Vec<_>, _>>()?;
        assert_eq!(
            draws,
            (0..1_000)
                .map(|n| super::schedule::random_selected(world, &key, n, interval))
                .collect::<std::result::Result<Vec<_>, _>>()?
        );
        if interval <= 60 {
            assert!(draws.iter().all(|won| *won));
        }
    }
    let draws = |world, key: &ProductionKey| {
        (0..100)
            .map(|n| super::schedule::random_selected(world, key, n, 120))
            .collect::<std::result::Result<Vec<_>, _>>()
    };
    assert_ne!(draws(world, &key)?, draws(WorldId::decode(&id(3))?, &key)?);
    assert_ne!(
        draws(world, &key)?,
        draws(world, &ProductionKey::new("raid:badgers")?)?
    );
    // Pinned vector detects accidental changes to framing/ordinal/native profile.
    assert_eq!(
        &draws(world, &key)?[..16],
        &[
            true, true, false, false, false, true, false, false, true, false, false, true, true,
            true, false, true
        ]
    );
    Ok(())
}

#[test]
fn raid_arithmetic_and_corruption_fail_closed() -> Result {
    let d = definition(false)?;
    let mut s = RaidSchedule::initial(1, 0)?;
    assert!(RaidSchedule::initial(0, 0).is_err());
    assert!(RaidSchedule::initial(1, i64::MAX).is_err());
    assert!(plan(&s, &d, check(i64::MAX)).is_err());
    assert!(plan(&s, &d, check(-1)).is_err());
    s.check_ordinal = u64::MAX;
    assert!(plan(&s, &d, check(60_000)).is_err());
    s.check_ordinal = 0;
    s.revision = u64::MAX;
    assert!(plan(&s, &d, check(60_000)).is_err());
    s.revision = 1;
    s.next_firing_ordinal = 2;
    assert_eq!(s.validate(), Err(RaidError::InvalidState));
    Ok(())
}
