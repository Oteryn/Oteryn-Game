use super::{ActorPosition, MAX_WALK_RETRIES, WALK_STALL, WalkState};

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
    state.goal = Some(position);
    state.retries = 0;
    state.blocked = false;
    state.last_progress = None;
    Some(position)
}

/// Arrival ends the walk; a new own position restarts the stall window.
pub fn progress(state: &mut WalkState, own: ActorPosition, now: std::time::Instant) {
    if state.goal == Some(own) {
        cancel(state);
    } else if state.last_position != Some(own) || state.last_progress.is_none() {
        state.last_progress = Some(now);
    }
    state.last_position = Some(own);
}

/// "There is no way." during an accepted walk: a step was blocked.
pub fn no_way(state: &mut WalkState) {
    if state.goal.is_some() && !state.pending {
        state.blocked = true;
    }
}

/// User input ends the walk and its re-requests.
pub fn cancel(state: &mut WalkState) {
    state.goal = None;
    state.blocked = false;
}

/// Like Tibia Global, a blocked step or a dropped walk asks the server again for a route to the
/// same destination, at most `MAX_WALK_RETRIES` times; the server's answer decides what follows.
pub fn retry(state: &mut WalkState, now: std::time::Instant) -> Option<ActorPosition> {
    let goal = state.goal?;
    if !state.selected || state.pending {
        return None;
    }
    let stalled = state
        .response
        .is_some_and(|response| response.disposition == Some(super::WalkDisposition::Accepted))
        && state
            .last_progress
            .is_some_and(|last| now.saturating_duration_since(last) >= WALK_STALL);
    if !state.blocked && !stalled {
        return None;
    }
    if state.retries >= MAX_WALK_RETRIES {
        cancel(state);
        return None;
    }
    state.retries += 1;
    state.blocked = false;
    state.pending = true;
    state.last_progress = None;
    state.feedback = Some(tr!("Walk request pending"));
    Some(goal)
}
