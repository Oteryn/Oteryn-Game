use super::{ActorPosition, WalkState};

pub fn request(state: &mut WalkState, position: ActorPosition) -> Option<ActorPosition> {
    if !state.selected {
        state.feedback = Some("Server walking unavailable");
        return None;
    }
    if state.pending {
        return None;
    }
    state.pending = true;
    state.feedback = Some("Walk request pending");
    Some(position)
}
