use oteryn_gameplay_world_change::{
    owner::SeasonalWindow,
    timed::{EventKind, EventState, Phase},
};

#[test]
fn wevents_chyllfroest_includes_may_first_and_closes_on_may_second() -> Result<(), &'static str> {
    let (start, end) = EventKind::Chyllfroest
        .window()
        .ok_or("missing Chyllfroest window")?;
    let window = SeasonalWindow { start, end };
    assert!(!window.contains(3, 31));
    assert!(window.contains(4, 1));
    assert!(window.contains(5, 1));
    assert!(!window.contains(5, 2));
    Ok(())
}

#[test]
fn wevents_rewards_require_victory_participation_and_unclaimed_admission() {
    let mut state = EventState::new(0, 100);
    assert!(!state.reward_admitted(1, false));
    state.phase = Phase::Victory;
    assert!(!state.reward_admitted(0, false));
    assert!(state.reward_admitted(1, false));
    assert!(!state.reward_admitted(1, true));
    state.phase = Phase::Failure;
    assert!(!state.reward_admitted(1, false));
}
