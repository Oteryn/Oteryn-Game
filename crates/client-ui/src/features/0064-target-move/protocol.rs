use super::{CommandStatus, WalkDisposition, WalkResponse, WalkState};

pub fn apply(state: &mut WalkState, response: WalkResponse) {
    state.pending = false;
    state.response = Some(response);
    state.feedback = Some(
        if !matches!(
            response.status,
            CommandStatus::Accepted | CommandStatus::DuplicateReplay
        ) {
            "Walk request refused"
        } else {
            match response.disposition {
                Some(WalkDisposition::Accepted) => "Walk request accepted",
                Some(WalkDisposition::NoPath) => "No path to that tile",
                _ => "Walk request refused",
            }
        },
    );
}
