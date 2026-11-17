use super::WalkState;

pub fn feedback(state: &WalkState) -> Option<&'static str> {
    state.feedback
}
