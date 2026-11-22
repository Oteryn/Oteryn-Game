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
        crate::play::mailbox::lock(&self.0).appearances(entries);
    }
}

impl crate::play::mailbox::Mailbox {
    pub(in crate::play) fn appearances(&mut self, entries: Vec<oteryn_session::VisibleAppearance>) {
        if self.appearance_state != entries {
            self.appearance_state.clone_from(&entries);
            let native = self.native_appearances(&entries);
            self.pushed.appearances = Some(native);
        }
    }
}
