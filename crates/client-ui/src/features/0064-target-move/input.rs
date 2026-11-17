use super::{ActorPosition, WalkState};

pub fn request(state: &mut WalkState, position: ActorPosition) -> Option<ActorPosition> {
    if !state.selected {
        state.feedback = Some(tr!("Server walking unavailable"));
        return None;
    }
    if state.pending {
        return None;
    }
    state.pending = true;
    state.feedback = Some(tr!("Walk request pending"));
    Some(position)
}
