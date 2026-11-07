use oteryn_gameplay_world_change::timed::{EventKind, EventState, HOUR, Input, Phase};

#[test]
fn wevents_devovorga_requires_mutations_second_incarnations_and_cocoon() -> Result<(), &'static str>
{
    let kind = EventKind::RiseOfDevovorga;
    let mut state = EventState::for_kind(kind, 0, 7 * 24 * HOUR)?;
    let at = 24 * HOUR;
    state.advance(kind, at, Input::Tick, 0)?;
    for member in 0..5 {
        assert!(state.advance(kind, at, Input::Defeat(member), 0)?);
    }
    assert_eq!(state.lifecycle.stage, 2);
    assert!(!state.advance(kind, at, Input::Defeat(5), 0)?);
    for member in [6, 7] {
        assert!(state.advance(kind, at, Input::Defeat(member), 0)?);
    }
    assert_eq!(state.lifecycle.stage, 3);
    for member in 0..5 {
        assert!(state.advance(kind, at, Input::Defeat(member), 0)?);
    }
    assert_eq!(state.lifecycle.stage, 4);
    assert!(state.advance(kind, at, Input::Defeat(8), 0)?);
    assert_eq!(state.lifecycle.stage, 5);
    assert!(state.advance(kind, at, Input::Defeat(5), 0)?);
    assert_eq!(state.phase, Phase::Victory);
    Ok(())
}
