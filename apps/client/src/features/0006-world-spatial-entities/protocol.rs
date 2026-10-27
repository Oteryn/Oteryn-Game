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
