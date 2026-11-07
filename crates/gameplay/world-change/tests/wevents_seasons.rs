use oteryn_gameplay_world_change::timed::{EventKind, EventState, Input, MINUTE, Phase};

#[test]
fn wevents_spring_hatch_requires_native_spawn_success_and_personal_wait() -> Result<(), &'static str>
{
    let mut s = EventState::for_kind(EventKind::SpringIntoLife, 0, 10_000_000)?;
    assert!(s.advance(EventKind::SpringIntoLife, 1_000, Input::BeginObject(1), 0)?);
    assert!(!s.advance(EventKind::SpringIntoLife, 2_000, Input::BeginObject(1), 0)?);
    assert!(!s.object_ready(1, 1_000 + 60 * MINUTE - 1));
    assert!(s.object_ready(1, 1_000 + 60 * MINUTE));
    assert_eq!(s.phase, Phase::Active);
    s.note_spawn(1, true)?;
    s.advance(
        EventKind::SpringIntoLife,
        1_000 + 60 * MINUTE,
        Input::Tick,
        0,
    )?;
    assert_eq!(s.phase, Phase::Victory);
    Ok(())
}

#[test]
fn wevents_devovorga_synchronizes_incarnations_resets_failed_attempts_and_wins()
-> Result<(), &'static str> {
    let day = 24 * 60 * MINUTE;
    let mut s = EventState::for_kind(EventKind::RiseOfDevovorga, 0, 7 * day)?;
    s.advance(EventKind::RiseOfDevovorga, day, Input::Tick, 0)?;
    assert_eq!(s.lifecycle.stage, 1);
    assert!(s.advance(EventKind::RiseOfDevovorga, day + 1, Input::Defeat(0), 0)?);
    let generation = s.lifecycle.generation;
    s.advance(
        EventKind::RiseOfDevovorga,
        day + 1 + 5 * MINUTE,
        Input::Tick,
        0,
    )?;
    assert_eq!(s.lifecycle.mask, 0);
    assert!(s.lifecycle.generation > generation);
    for member in 0..5 {
        assert!(s.advance(
            EventKind::RiseOfDevovorga,
            day + 2 + 5 * MINUTE,
            Input::Defeat(member),
            0
        )?);
    }
    assert_eq!(s.lifecycle.stage, 2);
    assert!(!s.advance(
        EventKind::RiseOfDevovorga,
        day + 3 + 5 * MINUTE,
        Input::Defeat(0),
        0
    )?);
    assert!(s.advance(
        EventKind::RiseOfDevovorga,
        day + 4 + 5 * MINUTE,
        Input::Defeat(5),
        0
    )?);
    assert_eq!(s.phase, Phase::Victory);
    assert!(s.reward_admitted(1, false));
    Ok(())
}

#[test]
fn wevents_hourly_robbery_never_fires_at_reset_or_rerolls_a_consumed_slot()
-> Result<(), &'static str> {
    let mut s = EventState::for_kind(EventKind::BankRobbery, 0, 24 * 60 * MINUTE)?;
    assert_eq!(s.phase, Phase::Dormant);
    assert!(!s.advance(EventKind::BankRobbery, 59 * MINUTE, Input::Tick, 0)?);
    assert!(!s.advance(EventKind::BankRobbery, 60 * MINUTE, Input::Tick, u64::MAX)?);
    assert!(!s.advance(EventKind::BankRobbery, 60 * MINUTE, Input::Tick, 0)?);
    assert!(s.advance(EventKind::BankRobbery, 120 * MINUTE, Input::Tick, 0)?);
    assert_eq!(s.phase, Phase::Active);
    assert!(!s.advance(EventKind::BankRobbery, 180 * MINUTE, Input::Tick, 0)?);
    Ok(())
}

#[test]
fn wevents_fire_requires_water_and_celebration_continues_after_victory() -> Result<(), &'static str>
{
    let hour = 60 * MINUTE;
    let mut fire = EventState::for_kind(EventKind::InsectoidInvasion, 0, 24 * hour)?;
    assert!(!fire.advance(EventKind::InsectoidInvasion, 1, Input::Extinguish(1), 0)?);
    assert!(fire.advance(EventKind::InsectoidInvasion, 2, Input::Fill(0), 0)?);
    assert!(fire.advance(EventKind::InsectoidInvasion, 3, Input::Extinguish(1), 0)?);
    assert!(!fire.advance(EventKind::InsectoidInvasion, 4, Input::Extinguish(2), 0)?);
    let mut kings = EventState::for_kind(EventKind::Kingsday, 0, 24 * hour)?;
    kings.advance(EventKind::Kingsday, 1, Input::Defeat(0), 0)?;
    assert!(kings.advance(EventKind::Kingsday, hour, Input::Tick, 0)?);
    assert_eq!(kings.wave, 1);
    Ok(())
}

#[test]
fn wevents_hatch_ack_is_versioned_and_rearms_without_erasing_reward_history()
-> Result<(), &'static str> {
    let mut state = EventState::for_kind(EventKind::SpringIntoLife, 0, 10_000_000)?;
    state.advance(EventKind::SpringIntoLife, 100, Input::BeginObject(1), 0)?;
    let version = state.lifecycle.objects[&1].revision();
    assert!(
        !state
            .lifecycle
            .complete_hatch(1, version, 100, 60 * MINUTE)?
    );
    assert!(
        state
            .lifecycle
            .complete_hatch(1, version, 100 + 60 * MINUTE, 60 * MINUTE)?
    );
    assert!(
        !state
            .lifecycle
            .complete_hatch(1, version, 101 + 60 * MINUTE, 60 * MINUTE)?
    );
    assert_eq!(state.lifecycle.last_success_ms[&1], 100 + 60 * MINUTE);
    assert!(!state.object_ready(1, 100 + 60 * MINUTE));
    assert!(state.object_ready(1, 100 + 120 * MINUTE));
    state.lifecycle.validate()?;
    Ok(())
}

#[test]
fn wevents_kingsday_royal_greeting_is_participation_without_arena_combat()
-> Result<(), &'static str> {
    let hour = 60 * MINUTE;
    let mut celebration = EventState::for_kind(EventKind::Kingsday, 0, 24 * hour)?;
    assert!(!celebration.reward_admitted(0, false));
    assert!(celebration.advance(EventKind::Kingsday, 1, Input::Attend, 0)?);
    assert!(celebration.reward_admitted(1, false));
    assert!(!celebration.reward_admitted(1, true));
    assert!(celebration.advance(EventKind::Kingsday, hour, Input::Tick, 0)?);
    assert_eq!(celebration.wave, 1);
    assert_eq!(celebration.phase, Phase::Victory);
    Ok(())
}
