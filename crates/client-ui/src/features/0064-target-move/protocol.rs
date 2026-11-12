use super::{CommandStatus, WalkDisposition, WalkResponse, WalkState};

pub fn apply(state: &mut WalkState, response: WalkResponse) {
    state.pending = false;
    state.response = Some(response);
    state.feedback = Some(
        if !matches!(
            response.status,
            CommandStatus::Accepted | CommandStatus::DuplicateReplay
        ) {
            tr!("Walk request refused")
        } else {
            match response.disposition {
                Some(WalkDisposition::Accepted) => tr!("Walk request accepted"),
                Some(WalkDisposition::NoPath) => tr!("No path to that tile"),
                _ => tr!("Walk request refused"),
            }
        },
    );
}
