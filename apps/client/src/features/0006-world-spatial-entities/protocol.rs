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
    pub(in crate::play) fn appearance_palette(&self, palette: Option<&[[u8; 3]]>) {
        let palette = palette
            .filter(|values| values.len() <= 256)
            .unwrap_or_default();
        let mut mailbox = crate::play::mailbox::lock(&self.0);
        if mailbox.appearance_palette != palette {
            mailbox.appearance_palette = palette.to_vec();
            mailbox.pushed.appearance_palette = Some(palette.to_vec());
        }
    }

    pub(in crate::play) fn appearances(&self, entries: Vec<oteryn_session::VisibleAppearance>) {
        let mut mailbox = crate::play::mailbox::lock(&self.0);
        if mailbox.appearance_state != entries {
            mailbox.appearance_state.clone_from(&entries);
            mailbox.pushed.appearances = Some(entries);
        }
    }
}
