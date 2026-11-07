use crate::play::{Pushed, mailbox::MAX_PUSHED_MARKERS};
use oteryn_session::{AppliedDelta, WorldObjectOverlayEntry};

pub(in crate::play) fn overlay(pushed: &mut Pushed, delta: AppliedDelta<WorldObjectOverlayEntry>) {
    pushed.set_marker(delta.value);
}

impl Pushed {
    pub(in crate::play) fn set_marker(&mut self, entry: WorldObjectOverlayEntry) {
        if let Some(held) = self
            .markers
            .iter_mut()
            .find(|held| held.placement == entry.placement)
        {
            *held = entry;
        } else if self.markers.len() < MAX_PUSHED_MARKERS {
            self.markers.push(entry);
        }
    }
}

/// A replacement is a reset, not a series of upserts over the previous placement set.
pub(in crate::play) fn overlay_snapshot(
    pushed: &mut Pushed,
    snapshot: AppliedDelta<Vec<WorldObjectOverlayEntry>>,
) {
    pushed.markers_reset = true;
    pushed.markers.clear();
    for entry in snapshot.value {
        pushed.set_marker(entry);
    }
}
