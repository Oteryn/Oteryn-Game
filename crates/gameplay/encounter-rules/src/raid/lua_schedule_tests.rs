use super::*;

fn sugar() -> LuaRaidPolicy {
    LuaRaidPolicy {
        initial_chance: Some(0.10),
        target_chance_per_day: 0.05,
        max_chance_per_check: 1.0,
        min_gap_ms: 0,
        allowed_weekdays: vec![5, 6, 7],
        min_active_players: 1,
        max_checks_per_day: None,
    }
}

fn check(now_ms: i64) -> LuaRaidCheck {
    LuaRaidCheck {
        now_ms,
        day: 10,
        weekday: 5,
        server_save_at_ms: now_ms + 86_400_000,
        active_players: 1,
        running: false,
        roll: 100,
    }
}

#[test]
fn raid_lua_sugar_percent_boundary_does_not_increase_when_target_is_lower() -> Result<(), RaidError>
{
    let policy = sugar();
    policy.validate()?;
    for failed in [0, 1, 1_380, 100_000] {
        assert!(policy.wins(failed, 100));
        assert!(!policy.wins(failed, 101));
    }
    let adaptive = LuaRaidPolicy {
        initial_chance: None,
        target_chance_per_day: 1.38,
        max_chance_per_check: 1.0,
        ..policy
    };
    assert!(adaptive.wins(0, 1));
    assert!(!adaptive.wins(0, 2));
    assert!(adaptive.wins(1, 1));
    assert!(!adaptive.wins(1, 2));
    assert!(adaptive.wins(2, 2));
    assert!(!adaptive.wins(2, 3));
    assert!(adaptive.wins(1_380, 1_000));
    assert!(!adaptive.wins(1_380, 1_001));
    Ok(())
}

#[test]
fn raid_lua_failed_roll_precedes_weekday_and_deferred_win_bypasses_later_draws()
-> Result<(), RaidError> {
    let policy = sugar();
    let state = LuaRaidState::initial(0, 10)?;
    let missed = state.plan(
        &policy,
        LuaRaidCheck {
            weekday: 4,
            roll: 101,
            ..check(60_000)
        },
    )?;
    assert!(!missed.start);
    assert_eq!(missed.state.checks_today, 1);
    assert_eq!(missed.state.failed_attempts, 1);
    assert!(!missed.state.trigger_when_possible);
    let pending = missed.state.plan(
        &policy,
        LuaRaidCheck {
            weekday: 4,
            ..check(120_000)
        },
    )?;
    assert!(!pending.start);
    assert!(pending.state.trigger_when_possible);
    assert_eq!(pending.state.failed_attempts, 1);
    assert_eq!(pending.state.checks_today, 2);
    let constrained = LuaRaidPolicy {
        min_gap_ms: u64::MAX,
        max_checks_per_day: Some(2),
        ..policy
    };
    let inactive = pending.state.plan(
        &constrained,
        LuaRaidCheck {
            roll: 100_000,
            active_players: 0,
            ..check(180_000)
        },
    )?;
    assert!(!inactive.start);
    assert!(inactive.state.trigger_when_possible);
    let started = inactive.state.plan(
        &constrained,
        LuaRaidCheck {
            roll: 100_000,
            day: 11,
            ..check(240_999)
        },
    )?;
    assert!(started.start);
    assert!(!started.state.trigger_when_possible);
    assert_eq!(started.state.failed_attempts, 0);
    assert_eq!(started.state.checks_today, 2);
    assert_eq!(started.state.check_day, 11);
    assert_eq!(started.state.last_occurrence_ms, Some(240_000));
    Ok(())
}

#[test]
fn raid_lua_gap_running_and_check_budget_do_not_consume_random_attempts() -> Result<(), RaidError> {
    let policy = LuaRaidPolicy {
        min_gap_ms: 120_000,
        max_checks_per_day: Some(1),
        ..sugar()
    };
    let state = LuaRaidState::initial(0, 10)?;
    let early = state.plan(&policy, check(60_000))?;
    assert!(!early.start);
    assert_eq!(early.state.checks_today, 0);
    let started = early.state.plan(&policy, check(120_000))?;
    assert!(started.start);
    let running = started.state.plan(
        &policy,
        LuaRaidCheck {
            running: true,
            ..check(180_000)
        },
    )?;
    assert!(!running.start);
    assert_eq!(running.state.checks_today, 1);
    let next_day = running.state.plan(
        &policy,
        LuaRaidCheck {
            day: 11,
            ..check(86_520_000)
        },
    )?;
    assert!(!next_day.start);
    assert_eq!(next_day.state.checks_today, 1);
    assert_eq!(next_day.state.failed_attempts, 0);
    assert_eq!(next_day.state.check_day, 11);
    Ok(())
}

#[test]
fn raid_lua_server_save_uses_seconds_and_strict_one_hour_boundary() -> Result<(), RaidError> {
    let policy = sugar();
    let state = LuaRaidState::initial(0, 10)?;
    let at_boundary = LuaRaidCheck {
        server_save_at_ms: 3_660_000,
        ..check(60_999)
    };
    assert!(state.plan(&policy, at_boundary)?.start);
    let after_boundary = state.plan(
        &policy,
        LuaRaidCheck {
            now_ms: 61_000,
            ..at_boundary
        },
    )?;
    assert!(!after_boundary.start);
    assert_eq!(after_boundary.state.checks_today, 0);
    assert_eq!(after_boundary.state.check_ordinal, 1);
    let deferred = LuaRaidState {
        trigger_when_possible: true,
        ..state
    };
    assert!(
        !deferred
            .plan(
                &policy,
                LuaRaidCheck {
                    now_ms: 61_000,
                    ..at_boundary
                }
            )?
            .start
    );
    assert!(
        !deferred
            .plan(
                &policy,
                LuaRaidCheck {
                    running: true,
                    ..at_boundary
                }
            )?
            .start
    );
    Ok(())
}

#[test]
fn raid_lua_due_checks_advance_once_and_do_not_burst_after_clock_jump() -> Result<(), RaidError> {
    let state = LuaRaidState::initial(0, 10)?;
    let policy = sugar();
    assert_eq!(state.plan(&policy, check(59_999))?.state, state);
    let late = state.plan(&policy, check(86_400_000))?;
    assert!(late.start);
    assert_eq!(late.state.check_ordinal, 1);
    assert_eq!(late.state.checks_today, 1);
    assert_eq!(late.state.next_check_at_ms, 86_460_000);
    let duplicate = late.state.plan(&policy, check(86_400_000))?;
    assert!(!duplicate.start);
    assert_eq!(duplicate.state, late.state);
    Ok(())
}

#[test]
fn raid_lua_durable_state_roundtrip_preserves_deferred_start()
-> Result<(), Box<dyn std::error::Error>> {
    let pending = LuaRaidState::initial(0, 10)?.plan(
        &sugar(),
        LuaRaidCheck {
            weekday: 4,
            ..check(60_000)
        },
    )?;
    let restored: LuaRaidState = serde_json::from_str(&serde_json::to_string(&pending.state)?)?;
    restored.validate()?;
    assert_eq!(restored, pending.state);
    assert!(
        restored
            .plan(
                &sugar(),
                LuaRaidCheck {
                    roll: 100_000,
                    ..check(120_000)
                }
            )?
            .start
    );
    Ok(())
}

#[test]
fn raid_lua_rejects_invalid_evidence_before_any_transition() -> Result<(), RaidError> {
    let state = LuaRaidState::initial(0, 10)?;
    let policy = sugar();
    assert_eq!(
        state.plan(
            &policy,
            LuaRaidCheck {
                roll: 0,
                ..check(60_000)
            }
        ),
        Err(RaidError::InvalidState)
    );
    assert_eq!(
        state.plan(
            &policy,
            LuaRaidCheck {
                roll: 100_001,
                ..check(60_000)
            }
        ),
        Err(RaidError::InvalidState)
    );
    assert_eq!(
        state.plan(
            &policy,
            LuaRaidCheck {
                weekday: 0,
                ..check(60_000)
            }
        ),
        Err(RaidError::InvalidCalendar)
    );
    assert_eq!(
        state.plan(
            &policy,
            LuaRaidCheck {
                day: 9,
                ..check(60_000)
            }
        ),
        Err(RaidError::InvalidCalendar)
    );
    assert_eq!(
        state.plan(
            &policy,
            LuaRaidCheck {
                server_save_at_ms: 1,
                ..check(60_000)
            }
        ),
        Err(RaidError::InvalidTime)
    );
    assert_eq!(
        LuaRaidPolicy {
            initial_chance: Some(f64::NAN),
            ..policy
        }
        .validate(),
        Err(RaidError::InvalidDefinition)
    );
    assert_eq!(
        LuaRaidState {
            last_occurrence_ms: Some(1),
            ..state
        }
        .validate(),
        Err(RaidError::InvalidState)
    );
    Ok(())
}

#[test]
fn raid_lua_activity_is_strict_and_native_roll_is_reproducible()
-> Result<(), Box<dyn std::error::Error>> {
    assert!(lua_raid_player_is_active(299_999));
    assert!(!lua_raid_player_is_active(300_000));
    assert!(!lua_raid_player_is_active(u64::MAX));
    let mut id = [1; 16];
    id[6] = 0x71;
    id[8] = 0x81;
    let world = WorldId::decode(&id).map_err(|error| format!("world: {error:?}"))?;
    let key = ProductionKey::new("raid:candia.sugarmommy")?;
    let mut observed = std::collections::BTreeSet::new();
    for ordinal in 0..64 {
        let draw = lua_raid_roll(world, &key, ordinal)?;
        assert!((1..=100_000).contains(&draw));
        assert_eq!(draw, lua_raid_roll(world, &key, ordinal)?);
        observed.insert(draw);
    }
    assert!(observed.len() > 1);
    Ok(())
}

#[test]
fn raid_lua_resolved_schedule_has_no_second_roll_and_allows_a_later_same_epoch_run()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::raid::{RaidAction, RaidDefinition, RaidPolicy, RaidSchedule, Wave, WaveEntry};
    let mut id = [1; 16];
    id[6] = 0x71;
    id[8] = 0x81;
    let world = WorldId::decode(&id).map_err(|error| format!("world: {error:?}"))?;
    let key = ProductionKey::new("raid:candia.sugarmommy")?;
    let definition = RaidDefinition {
        policy: RaidPolicy::SourceScheduled,
        despawn_after_ms: None,
        announcements: vec![],
        waves: vec![Wave {
            delay_ms: 0,
            entries: vec![WaveEntry {
                creature: ProductionKey::new("creature:sugar-mommy")?,
                count: 1,
                area_anchor: ProductionKey::new("area:candia.sugarmommy")?,
            }],
        }],
    };
    let action = RaidAction::ResolvedCheck {
        now_ms: 60_000,
        selected: true,
        firing_id: id,
        bundle_digest: [1; 32],
    };
    let initial = RaidSchedule::initial(1, 0)?;
    let started = initial.plan(world, &key, &definition, action)?;
    assert_eq!(started.after().next_firing_ordinal, 2);
    assert_eq!(started.after().check_ordinal, 1);
    assert_eq!(started.after().last_fired_at_ms, Some(60_000));
    assert_eq!(
        started
            .after()
            .plan(world, &key, &definition, action)?
            .after(),
        started.after()
    );
    let blocked = started.after().plan(
        world,
        &key,
        &definition,
        RaidAction::ResolvedCheck {
            now_ms: 120_000,
            selected: true,
            firing_id: id,
            bundle_digest: [1; 32],
        },
    )?;
    assert_eq!(blocked.after().next_firing_ordinal, 2);
    assert_eq!(blocked.after().check_ordinal, 2);
    let closed = blocked.after().plan(
        world,
        &key,
        &definition,
        RaidAction::Close { now_ms: 120_000 },
    )?;
    id[15] = 2;
    let repeated = closed.after().plan(
        world,
        &key,
        &definition,
        RaidAction::ResolvedCheck {
            now_ms: 180_000,
            selected: true,
            firing_id: id,
            bundle_digest: [1; 32],
        },
    )?;
    assert_eq!(repeated.after().reset_epoch, 1);
    assert_eq!(repeated.after().next_firing_ordinal, 3);
    assert_eq!(repeated.after().last_fired_at_ms, Some(180_000));
    let retired = initial.plan(world, &key, &definition, RaidAction::Retire)?;
    assert_eq!(
        retired
            .after()
            .plan(world, &key, &definition, action)?
            .after(),
        retired.after()
    );
    let native = RaidDefinition {
        policy: RaidPolicy::Random {
            mean_interval_s: 600,
            min_gap_s: 0,
        },
        ..definition
    };
    assert_eq!(
        initial.plan(world, &key, &native, action),
        Err(RaidError::InvalidDefinition)
    );
    Ok(())
}
