use super::*;

#[test]
fn negotiation_and_pending_gate_world_input_and_refusal_allows_retry() {
    let position = ActorPosition {
        x: -50,
        y: 200,
        floor: -3,
    };
    let mut state = WalkState::default();
    assert_eq!(request(&mut state, position), None);
    assert!(!state.pending);
    state.selected = true;
    assert_eq!(request(&mut state, position), Some(position));
    assert_eq!(request(&mut state, position), None);
    apply(
        &mut state,
        WalkResponse {
            status: CommandStatus::Rejected,
            disposition: Some(WalkDisposition::Rejected),
        },
    );
    assert!(!state.pending);
    assert_eq!(state.feedback, Some("Walk request refused"));
    assert_eq!(request(&mut state, position), Some(position));
    apply(
        &mut state,
        WalkResponse {
            status: CommandStatus::Accepted,
            disposition: Some(WalkDisposition::NoPath),
        },
    );
    assert_eq!(state.feedback, Some("No path to that tile"));
    assert_eq!(request(&mut state, position), Some(position));
    apply(
        &mut state,
        WalkResponse {
            status: CommandStatus::Accepted,
            disposition: Some(WalkDisposition::Accepted),
        },
    );
    assert_eq!(state.feedback, Some("Walk request accepted"));
    assert!(!state.pending);
}
