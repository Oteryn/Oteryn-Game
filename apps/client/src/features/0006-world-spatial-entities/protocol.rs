use crate::play::Pushed;
use oteryn_session::{AppliedDelta, WorldSpatialEntitiesDelta, WorldSpatialObservation};

pub(in crate::play) fn spatial(pushed: &mut Pushed, delta: AppliedDelta<WorldSpatialObservation>) {
    pushed.own = Some(delta.value.actor_position);
}

pub(in crate::play) fn entities(
    pushed: &mut Pushed,
    delta: AppliedDelta<WorldSpatialEntitiesDelta>,
) {
    pushed.own = Some(delta.value.actor_position);
}

impl crate::play::Outbox {
    pub(in crate::play) fn appearances(&self, entries: Vec<oteryn_session::VisibleAppearance>) {
        let mut mailbox = crate::play::mailbox::lock(&self.0);
        if mailbox.appearance_state != entries {
            mailbox.appearance_state.clone_from(&entries);
            mailbox.pushed.appearances = Some(entries);
        }
    }
}
