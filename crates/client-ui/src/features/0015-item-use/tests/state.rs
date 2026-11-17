use crate::world_interaction::menu::State;
#[test]
fn source_removal_clears_targeting() {
    let mut state = State::default();
    state.begin_targeting(std::num::NonZeroU64::MIN);
    super::source_current(&mut state, true);
    assert!(state.targeting.is_some());
    super::source_current(&mut state, false);
    assert!(state.targeting.is_none());
    assert!(
        state
            .feedback(std::time::Instant::now())
            .is_some_and(|text| text.contains("disappeared"))
    );
}
