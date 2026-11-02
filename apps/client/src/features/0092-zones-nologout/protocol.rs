use crate::play::Pushed;
use oteryn_session::{AppliedDelta, ServerMessage};

pub(in crate::play) fn message(pushed: &mut Pushed, delta: AppliedDelta<ServerMessage>) {
    pushed.server_messages.push(delta);
}
