//! Synthetic proposed facts only; these are not real raid content or current authority fixtures.
#![allow(
    clippy::expect_used,
    reason = "Synthetic pure-state fixture construction"
)]
use super::calendar::{CalendarSelector, FixedCalendarRule, ResetCalendarFacts, Weekday};
use super::*;

pub(super) fn id(seed: u8) -> [u8; 16] {
    [
        seed, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, seed,
    ]
}
pub(super) fn channel(seed: u8) -> ChannelId {
    ChannelId::decode(&id(seed)).expect("native channel")
}
pub(super) fn generation(value: u64) -> ScopeOwnershipGeneration {
    ScopeOwnershipGeneration::new(value).expect("native generation")
}
pub(super) fn key() -> RaidKey {
    RaidKey {
        world: WorldId::decode(&id(1)).expect("native world"),
        raid: "oteryn:raid.proposed".into(),
    }
}
pub(super) fn timing(persistent: bool) -> Timing {
    Timing {
        policy: Policy::Random {
            mean_interval_s: 60,
            min_gap_s: 0,
        },
        last_wave_delay_ms: 10_000,
        despawn_after_ms: (!persistent).then_some(20_000),
    }
}
pub(super) fn initial(persistent: bool) -> Change {
    Snapshot::initialize(key(), 1, [2; 32], timing(persistent), 0).expect("proposed initialization")
}
pub(super) fn facts(now: i64) -> CheckFacts {
    CheckFacts {
        now_ms: now,
        proposed_firing_id: id(10),
        proposed_targets: vec![channel(2), channel(3)],
        fixed_slot_ms: None,
    }
}
pub(super) fn fired(persistent: bool) -> Snapshot {
    initial(persistent)
        .after
        .check(&facts(60_000))
        .expect("check")
        .expect("due")
        .after
}

#[test]
fn deterministic_tuple_includes_world_key_and_ordinal() {
    let k = key();
    let d = draw(&k, 17).expect("draw");
    assert_eq!(draw(&k, 17), Ok(d));
    let mut changed = k.clone();
    changed.world = WorldId::decode(&id(2)).expect("world");
    assert_ne!(draw(&changed, 17), Ok(d));
    changed = k.clone();
    changed.raid = "oteryn:raid.other".into();
    assert_ne!(draw(&changed, 17), Ok(d));
    assert_ne!(draw(&k, 18), Ok(d));
    let state = initial(false).after;
    assert_eq!(
        state
            .check(&facts(60_000))
            .expect("check")
            .expect("due")
            .after,
        state
            .check(&facts(60_000))
            .expect("retry")
            .expect("due")
            .after
    );
}

#[test]
fn due_check_advances_once_and_never_catches_up() {
    let state = initial(false).after;
    assert!(state.check(&facts(59_999)).expect("early").is_none());
    let next = state
        .check(&facts(9_000_000))
        .expect("late")
        .expect("due")
        .after;
    assert_eq!(next.check_ordinal(), 1);
    assert_eq!(next.next_check_at_ms(), 9_060_000);
    assert!(next.check(&facts(9_000_000)).expect("same now").is_none());
    let resumed = next.resume_from(10_000_000).expect("resume").after;
    assert_eq!(resumed.next_check_at_ms(), 10_060_000);
    assert_eq!(resumed.check_ordinal(), 1);
    assert_eq!(
        resumed.schedule.last_fired_at_ms,
        next.schedule.last_fired_at_ms
    );
}

#[test]
fn running_guard_advances_check_but_cannot_duplicate_firing() {
    let state = fired(false);
    let next = state
        .check(&facts(120_000))
        .expect("check")
        .expect("due")
        .after;
    assert_eq!(next.latest, state.latest);
    assert_eq!(next.schedule.next_firing_ordinal, 1);
    assert_eq!(next.check_ordinal(), 2);
}

#[test]
fn persistent_guard_survives_done_and_only_new_reset_allows_next_firing() {
    let state = fired(true);
    let closed = state.close(70_000).expect("close").expect("deadline").after;
    let blocked = closed
        .check(&facts(120_000))
        .expect("check")
        .expect("due")
        .after;
    assert_eq!(blocked.schedule.next_firing_ordinal, 1);
    let reset = blocked
        .reset(2, [3; 32], timing(true), 130_000)
        .expect("reset")
        .after;
    let mut input = facts(190_000);
    input.proposed_firing_id = id(11);
    let fired = reset.check(&input).expect("check").expect("due").after;
    assert_eq!(fired.schedule.next_firing_ordinal, 2);
    assert_eq!(fired.latest.expect("firing").reset_epoch, 2);
}

#[test]
fn minimum_gap_counts_durable_last_fired_even_after_restart() {
    let mut config = timing(false);
    config.policy = Policy::Random {
        mean_interval_s: 60,
        min_gap_s: 300,
    };
    let state = Snapshot::initialize(key(), 1, [2; 32], config, 0)
        .expect("initialize")
        .after;
    let first = state
        .check(&facts(60_000))
        .expect("check")
        .expect("due")
        .after;
    let closed = first.close(90_000).expect("close").expect("deadline").after;
    let resumed = closed.resume_from(180_000).expect("resume").after;
    let blocked = resumed
        .check(&facts(240_000))
        .expect("check")
        .expect("due")
        .after;
    assert_eq!(blocked.schedule.next_firing_ordinal, 1);
    let mut input = facts(360_000);
    input.proposed_firing_id = id(11);
    assert_eq!(
        blocked
            .check(&input)
            .expect("check")
            .expect("due")
            .after
            .schedule
            .next_firing_ordinal,
        2
    );
}

#[test]
fn fixed_slot_is_explicit_once_only_and_does_not_guess_calendar() {
    let mut config = timing(false);
    config.policy = Policy::Fixed;
    let state = Snapshot::initialize(key(), 1, [2; 32], config, 0)
        .expect("initialize")
        .after;
    assert_eq!(
        state.check(&facts(60_000)).err(),
        Some(RaidError::InvalidInput)
    );
    let mut input = facts(60_000);
    input.fixed_slot_ms = Some(60_001);
    let early = state.check(&input).expect("check").expect("due").after;
    assert!(early.latest.is_none());
    input.now_ms = 120_000;
    input.fixed_slot_ms = Some(120_000);
    let first = early.check(&input).expect("check").expect("due").after;
    let closed = first
        .close(150_000)
        .expect("close")
        .expect("deadline")
        .after;
    input.now_ms = 180_000;
    assert_eq!(
        closed
            .check(&input)
            .expect("repeat slot")
            .expect("due")
            .after
            .schedule
            .next_firing_ordinal,
        1
    );
}

#[test]
fn fixed_missed_slot_is_not_caught_up_after_restart() {
    let mut config = timing(false);
    config.policy = Policy::Fixed;
    let state = Snapshot::initialize(key(), 1, [2; 32], config, 0)
        .expect("initialize")
        .after;
    let resumed = state.resume_from(300_000).expect("restart").after;
    let mut input = facts(360_000);
    input.fixed_slot_ms = Some(60_000);
    let skipped = resumed.check(&input).expect("check").expect("due").after;
    assert!(skipped.latest.is_none());
    assert_eq!(skipped.check_ordinal(), 1);
}

#[test]
fn random_no_hit_still_advances_and_large_mean_has_no_overflow() {
    let mut config = timing(false);
    config.policy = Policy::Random {
        mean_interval_s: u64::MAX,
        min_gap_s: u64::MAX,
    };
    let state = Snapshot::initialize(key(), 1, [2; 32], config, 0)
        .expect("initialize")
        .after;
    assert!(draw(&key(), 0).expect("draw") > 60);
    let next = state
        .check(&facts(60_000))
        .expect("check")
        .expect("due")
        .after;
    assert!(next.latest.is_none());
    assert_eq!(next.check_ordinal(), 1);
}

#[test]
fn target_set_is_frozen_missing_rows_never_finish_early() {
    let state = fired(false);
    assert!(state.close(60_001).expect("not terminal").is_none());
    assert_eq!(
        state.claim(id(10), channel(4), generation(1)).err(),
        Some(RaidError::NotTargeted)
    );
    assert_eq!(
        state.claim(id(11), channel(2), generation(1)).err(),
        Some(RaidError::NotRunning)
    );
    let claimed = state
        .claim(id(10), channel(2), generation(5))
        .expect("claim")
        .expect("new")
        .after;
    assert!(
        claimed
            .claim(id(10), channel(2), generation(5))
            .expect("retry")
            .is_none()
    );
    let done = claimed
        .record_done(id(10), channel(2), generation(5))
        .expect("done metadata")
        .expect("new")
        .after;
    assert!(done.close(60_001).expect("still missing target").is_none());
    let closed = done.close(90_000).expect("close").expect("deadline").after;
    assert_eq!(
        closed.latest.as_ref().expect("firing").targets[&channel(3)],
        Some(Run::Lost(None))
    );
    assert_eq!(
        closed.claim(id(10), channel(3), generation(5)).err(),
        Some(RaidError::NotRunning)
    );
}

#[test]
fn newer_generation_records_lost_without_reclaim_or_action() {
    let state = fired(false)
        .claim(id(10), channel(2), generation(5))
        .expect("claim")
        .expect("new")
        .after;
    assert_eq!(
        state.claim(id(10), channel(2), generation(4)).err(),
        Some(RaidError::StaleGeneration)
    );
    let lost = state
        .claim(id(10), channel(2), generation(6))
        .expect("recover")
        .expect("changed")
        .after;
    assert_eq!(
        lost.latest.as_ref().expect("firing").targets[&channel(2)],
        Some(Run::Lost(Some(generation(5))))
    );
    assert!(
        lost.claim(id(10), channel(2), generation(7))
            .expect("no reclaim")
            .is_none()
    );
    assert_eq!(
        lost.record_done(id(10), channel(2), generation(6)).err(),
        Some(RaidError::StaleGeneration)
    );
}

#[test]
fn all_terminal_targets_close_early_but_empty_targets_wait_deadline() {
    let mut state = fired(false);
    for c in [channel(2), channel(3)] {
        state = state
            .claim(id(10), c, generation(1))
            .expect("claim")
            .expect("new")
            .after;
        state = state
            .record_done(id(10), c, generation(1))
            .expect("done metadata")
            .expect("new")
            .after;
    }
    assert_eq!(
        state
            .close(60_001)
            .expect("close")
            .expect("terminal")
            .after
            .latest
            .expect("firing")
            .state,
        FiringState::Done
    );
    let mut input = facts(60_000);
    input.proposed_targets.clear();
    let empty = initial(false)
        .after
        .check(&input)
        .expect("check")
        .expect("due")
        .after;
    assert!(empty.close(60_001).expect("empty").is_none());
    assert!(empty.close(90_000).expect("deadline").is_some());
}

#[test]
fn reset_cancels_old_firing_retirement_keeps_row_and_prevents_checks() {
    let state = fired(false);
    let reset = state
        .reset(2, [3; 32], timing(false), 61_000)
        .expect("reset")
        .after;
    assert_eq!(
        reset.latest.expect("old firing").state,
        FiringState::Cancelled
    );
    assert_eq!(
        state.reset(1, [3; 32], timing(false), 61_000).err(),
        Some(RaidError::InvalidInput)
    );
    let retired = state.retire().expect("retire").expect("new").after;
    assert!(retired.check(&facts(120_000)).expect("retired").is_none());
    assert!(retired.latest.is_some());
    assert!(retired.retire().expect("idempotent").is_none());
}

#[test]
fn malformed_duplicate_and_overflow_facts_fail_without_state_mutation() {
    let state = initial(false).after;
    let original = state.clone();
    let mut input = facts(60_000);
    input.proposed_targets.push(channel(2));
    assert_eq!(state.check(&input).err(), Some(RaidError::InvalidInput));
    input = facts(60_000);
    input.proposed_firing_id = [0; 16];
    assert_eq!(state.check(&input).err(), Some(RaidError::InvalidInput));
    input = facts(i64::MAX);
    assert_eq!(state.check(&input).err(), Some(RaidError::Overflow));
    assert_eq!(state, original);
    let mut invalid = state.clone();
    invalid.schedule.check_ordinal = u64::MAX;
    assert_eq!(
        invalid.check(&facts(60_000)).err(),
        Some(RaidError::Overflow)
    );
    invalid = state;
    invalid.schedule.revision = u64::MAX;
    assert_eq!(invalid.resume_from(1).err(), Some(RaidError::Overflow));
    assert_eq!(
        Snapshot::initialize(key(), 1, [0; 32], timing(false), 0).err(),
        Some(RaidError::InvalidInput)
    );
}

fn fixed_initial(persistent: bool) -> Snapshot {
    let mut config = timing(persistent);
    config.policy = Policy::Fixed;
    Snapshot::initialize(key(), 1, [2; 32], config, 0)
        .expect("fixed initialization")
        .after
}

fn reset_calendar() -> ResetCalendarFacts {
    ResetCalendarFacts {
        reset_at_ms: 0,
        weekday: Weekday::Monday,
        day_of_month: 1,
    }
}

fn weekday_rule() -> FixedCalendarRule {
    FixedCalendarRule {
        selector: CalendarSelector::Weekday(Weekday::Monday),
        offset_after_reset_ms: 60_000,
    }
}

#[test]
fn calendar_weekday_uses_the_existing_fixed_transition_and_replays_identically() {
    for weekday in [
        Weekday::Monday,
        Weekday::Tuesday,
        Weekday::Wednesday,
        Weekday::Thursday,
        Weekday::Friday,
        Weekday::Saturday,
        Weekday::Sunday,
    ] {
        let state = fixed_initial(false);
        let rule = FixedCalendarRule {
            selector: CalendarSelector::Weekday(weekday),
            ..weekday_rule()
        };
        let reset = ResetCalendarFacts {
            weekday,
            ..reset_calendar()
        };
        let input = facts(60_000);
        let converted = state
            .check_fixed_calendar(&input, &rule, &reset)
            .expect("calendar check")
            .expect("due");
        let raw = CheckFacts {
            fixed_slot_ms: Some(60_000),
            ..input.clone()
        };
        assert_eq!(
            converted.after,
            state.check(&raw).expect("raw check").expect("due").after
        );
        assert_eq!(converted.after.schedule.last_fixed_slot_ms, Some(60_000));
        assert_eq!(
            converted.after,
            state
                .check_fixed_calendar(&input, &rule, &reset)
                .expect("retry")
                .expect("due")
                .after
        );
        assert!(
            converted
                .after
                .check_fixed_calendar(&input, &rule, &reset)
                .expect("committed replay")
                .is_none()
        );
    }
}

#[test]
fn calendar_mismatch_consumes_only_due_check_without_a_synthetic_slot() {
    let state = fixed_initial(false);
    let rule = weekday_rule();
    let reset = ResetCalendarFacts {
        weekday: Weekday::Tuesday,
        ..reset_calendar()
    };
    assert!(
        state
            .check_fixed_calendar(&facts(59_999), &rule, &reset)
            .expect("not due")
            .is_none()
    );
    let next = state
        .check_fixed_calendar(&facts(60_000), &rule, &reset)
        .expect("no slot")
        .expect("due")
        .after;
    assert_eq!(next.check_ordinal(), 1);
    assert_eq!(next.next_check_at_ms(), 120_000);
    assert_eq!(next.latest, state.latest);
    assert_eq!(
        next.schedule.last_fixed_slot_ms,
        state.schedule.last_fixed_slot_ms
    );
    assert_eq!(
        next.schedule.last_fired_at_ms,
        state.schedule.last_fired_at_ms
    );
    assert_eq!(next.schedule.next_firing_ordinal, 0);
    let fired = state
        .check_fixed_calendar(&facts(60_000), &rule, &reset_calendar())
        .expect("matching day")
        .expect("due")
        .after;
    let absent = fired
        .check_fixed_calendar(&facts(120_000), &rule, &reset)
        .expect("no slot with latest firing")
        .expect("due")
        .after;
    assert_eq!(absent.latest, fired.latest);
    assert_eq!(absent.schedule.last_fixed_slot_ms, Some(60_000));
}

#[test]
fn calendar_month_day_matches_supplied_day_and_offset_can_cross_midnight() {
    for day in [1, 28, 29, 30, 31] {
        let rule = FixedCalendarRule {
            selector: CalendarSelector::DayOfMonth(day),
            offset_after_reset_ms: 86_460_000,
        };
        let reset = ResetCalendarFacts {
            day_of_month: day,
            ..reset_calendar()
        };
        let state = fixed_initial(false);
        let early = state
            .check_fixed_calendar(&facts(60_000), &rule, &reset)
            .expect("future slot")
            .expect("due")
            .after;
        assert!(early.latest.is_none());
        let fired = early
            .check_fixed_calendar(&facts(86_460_000), &rule, &reset)
            .expect("offset beyond a day")
            .expect("due")
            .after;
        assert_eq!(fired.schedule.last_fixed_slot_ms, Some(86_460_000));
        let mismatch = ResetCalendarFacts {
            day_of_month: if day == 1 { 2 } else { 1 },
            ..reset
        };
        let absent = state
            .check_fixed_calendar(&facts(86_460_000), &rule, &mismatch)
            .expect("other day")
            .expect("due")
            .after;
        assert!(absent.latest.is_none());
    }
    let zero = FixedCalendarRule {
        offset_after_reset_ms: 0,
        ..weekday_rule()
    };
    let reset = ResetCalendarFacts {
        reset_at_ms: 60_000,
        ..reset_calendar()
    };
    assert!(
        fixed_initial(false)
            .check_fixed_calendar(&facts(60_000), &zero, &reset)
            .expect("zero offset")
            .expect("due")
            .after
            .latest
            .is_some()
    );
}

#[test]
fn calendar_keeps_running_persistent_and_same_slot_guards() {
    for persistent in [false, true] {
        let rule = weekday_rule();
        let reset = reset_calendar();
        let state = fixed_initial(persistent);
        let fired = state
            .check_fixed_calendar(&facts(60_000), &rule, &reset)
            .expect("fire")
            .expect("due")
            .after;
        let later_reset_facts = ResetCalendarFacts {
            reset_at_ms: 60_000,
            ..reset
        };
        let running = fired
            .check_fixed_calendar(&facts(120_000), &rule, &later_reset_facts)
            .expect("running guard")
            .expect("due")
            .after;
        assert_eq!(running.latest, fired.latest);
        let closed = fired.close(90_000).expect("close").expect("deadline").after;
        let repeated = closed
            .check_fixed_calendar(&facts(120_000), &rule, &reset)
            .expect("same slot")
            .expect("due")
            .after;
        assert_eq!(repeated.schedule.next_firing_ordinal, 1);
        let mut input = facts(120_000);
        input.proposed_firing_id = id(11);
        let next = closed
            .check_fixed_calendar(&input, &rule, &later_reset_facts)
            .expect("new slot")
            .expect("due")
            .after;
        assert_eq!(
            next.schedule.next_firing_ordinal,
            if persistent { 1 } else { 2 }
        );
        assert_eq!(
            next.schedule.last_fixed_slot_ms,
            Some(if persistent { 60_000 } else { 120_000 })
        );
    }
}

#[test]
fn calendar_restart_skips_missed_slot_without_reinitializing_check_state() {
    let rule = weekday_rule();
    let reset = reset_calendar();
    let state = fixed_initial(false)
        .resume_from(300_000)
        .expect("restart")
        .after;
    let missed = state
        .check_fixed_calendar(&facts(360_000), &rule, &reset)
        .expect("missed slot")
        .expect("due")
        .after;
    assert!(missed.latest.is_none());
    assert_eq!(missed.check_ordinal(), 1);
    assert_eq!(missed.next_check_at_ms(), 420_000);
    let next_reset = ResetCalendarFacts {
        reset_at_ms: 360_000,
        ..reset
    };
    let next = missed
        .check_fixed_calendar(&facts(420_000), &rule, &next_reset)
        .expect("next supplied slot")
        .expect("due")
        .after;
    assert_eq!(next.schedule.last_fixed_slot_ms, Some(420_000));
    assert_eq!(next.check_ordinal(), 2);
}

#[test]
fn calendar_rejects_conflicting_policy_or_raw_slot_without_mutating_state() {
    let rule = weekday_rule();
    let reset = reset_calendar();
    let random = initial(false).after;
    let before = random.clone();
    assert_eq!(
        random
            .check_fixed_calendar(&facts(60_000), &rule, &reset)
            .err(),
        Some(RaidError::InvalidInput)
    );
    assert_eq!(random, before);
    let state = fixed_initial(false);
    let before = state.clone();
    let input = CheckFacts {
        fixed_slot_ms: Some(60_000),
        ..facts(60_000)
    };
    assert_eq!(
        state.check_fixed_calendar(&input, &rule, &reset).err(),
        Some(RaidError::InvalidInput)
    );
    assert_eq!(
        state.check(&facts(60_000)).err(),
        Some(RaidError::InvalidInput)
    );
    assert_eq!(state, before);
}

#[test]
fn calendar_invalid_intrinsic_bounds_and_arithmetic_are_refused_without_state_change() {
    let state = fixed_initial(false);
    let before = state.clone();
    let rule = weekday_rule();
    let reset = reset_calendar();
    for day in [0, 32, u8::MAX] {
        let bad_rule = FixedCalendarRule {
            selector: CalendarSelector::DayOfMonth(day),
            ..rule
        };
        assert_eq!(
            state
                .check_fixed_calendar(&facts(60_000), &bad_rule, &reset)
                .err(),
            Some(RaidError::InvalidInput)
        );
        let bad_reset = ResetCalendarFacts {
            day_of_month: day,
            ..reset
        };
        assert_eq!(
            state
                .check_fixed_calendar(&facts(60_000), &rule, &bad_reset)
                .err(),
            Some(RaidError::InvalidInput)
        );
    }
    assert_eq!(
        state
            .check_fixed_calendar(
                &facts(60_000),
                &FixedCalendarRule {
                    offset_after_reset_ms: -1,
                    ..rule
                },
                &reset
            )
            .err(),
        Some(RaidError::InvalidInput)
    );
    assert_eq!(
        state
            .check_fixed_calendar(
                &facts(60_000),
                &rule,
                &ResetCalendarFacts {
                    reset_at_ms: -1,
                    ..reset
                }
            )
            .err(),
        Some(RaidError::InvalidInput)
    );
    assert_eq!(
        state
            .check_fixed_calendar(
                &facts(60_000),
                &rule,
                &ResetCalendarFacts {
                    reset_at_ms: i64::MAX,
                    ..reset
                }
            )
            .err(),
        Some(RaidError::Overflow)
    );
    // No offset cap was accepted: the largest representable sum is valid proposed data.
    let largest = FixedCalendarRule {
        offset_after_reset_ms: i64::MAX,
        ..rule
    };
    assert!(
        state
            .check_fixed_calendar(&facts(60_000), &largest, &reset)
            .expect("representable future slot")
            .expect("due")
            .after
            .latest
            .is_none()
    );
    assert_eq!(state, before);
}
