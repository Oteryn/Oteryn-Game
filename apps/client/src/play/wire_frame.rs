//! The frame of domain-1 positions on the wire versus the native map frame the client draws in.
//!
//! Domain 17 map positions are always native (negative floors on bundle worlds). Domain-1
//! positions are native on pinned activity bundles and raw (the runtime's positive floors) on
//! ordinary bundles. The frame is derived from those two server facts, never from a floor sign:
//! when the latest wire actor position and the latest map origin share x and y, equal floors
//! mean `Native` and opposite floors mean `Raw`; anything else keeps the previous frame.
//! Fixture worlds have no map, so they stay `Native` (identity).
use super::mailbox::Mailbox;
use oteryn_session::{ActorPosition, SessionEvent, WorldSpatialEntity};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum WireFrame {
    #[default]
    Native,
    Raw,
}

impl WireFrame {
    const fn flip(position: ActorPosition) -> ActorPosition {
        ActorPosition {
            floor: position.floor.saturating_neg(),
            ..position
        }
    }

    /// A domain-1 wire position in the native map frame.
    pub(crate) const fn to_native(self, position: ActorPosition) -> ActorPosition {
        match self {
            Self::Native => position,
            Self::Raw => Self::flip(position),
        }
    }

    /// A native position as the server expects it on an inbound command.
    pub(crate) const fn to_wire(self, position: ActorPosition) -> ActorPosition {
        self.to_native(position)
    }

    /// The same for a bare floor.
    pub(crate) const fn floor_to_native(self, floor: i16) -> i16 {
        self.to_native(ActorPosition { x: 0, y: 0, floor }).floor
    }

    /// The frame proven by the latest wire actor position and map origin; `previous` when they
    /// do not decide it (other tile, floor 0, or a transient mismatch during a step).
    pub(crate) const fn derive(previous: Self, wire: ActorPosition, origin: ActorPosition) -> Self {
        if wire.x != origin.x || wire.y != origin.y || wire.floor == 0 {
            previous
        } else if wire.floor == origin.floor {
            Self::Native
        } else if wire.floor == origin.floor.saturating_neg() {
            Self::Raw
        } else {
            previous
        }
    }
}

/// The frame with the two facts it is derived from, one per play session.
#[derive(Debug, Default, Clone, Copy)]
pub(super) struct WireTracker {
    frame: WireFrame,
    wire: Option<ActorPosition>,
    origin: Option<ActorPosition>,
}

impl WireTracker {
    /// Records the latest raw domain-1 actor position.
    pub(super) fn observe_wire(&mut self, position: ActorPosition) {
        self.wire = Some(position);
        self.refresh();
    }

    /// Records the latest domain-17 map origin.
    pub(super) fn observe_origin(&mut self, origin: ActorPosition) {
        self.origin = Some(origin);
        self.refresh();
    }

    fn refresh(&mut self) {
        if let (Some(wire), Some(origin)) = (self.wire, self.origin) {
            self.frame = WireFrame::derive(self.frame, wire, origin);
        }
    }
}

fn entity_native(frame: WireFrame, entity: &mut WorldSpatialEntity) {
    entity.position = frame.to_native(entity.position);
}

impl Mailbox {
    pub(super) const fn wire_frame(&self) -> WireFrame {
        self.wire.frame
    }

    /// The position of a native-frame target on an inbound command.
    pub(super) const fn wire_position(&self, native: ActorPosition) -> ActorPosition {
        self.wire.frame.to_wire(native)
    }

    pub(super) fn native_entities(&self, wire: &[WorldSpatialEntity]) -> Vec<WorldSpatialEntity> {
        let mut entities = wire.to_vec();
        entities
            .iter_mut()
            .for_each(|entity| entity_native(self.wire.frame, entity));
        entities
    }

    pub(super) fn native_appearances(
        &self,
        wire: &[oteryn_session::VisibleAppearance],
    ) -> Vec<oteryn_session::VisibleAppearance> {
        let mut entries = wire.to_vec();
        entries
            .iter_mut()
            .for_each(|entry| entity_native(self.wire.frame, &mut entry.entity));
        entries
    }

    /// An activity scope change starts a new frame: the destination bundle decides it afresh.
    pub(super) fn reset_wire_frame(&mut self) {
        self.wire = WireTracker::default();
        self.entities_wire.clear();
    }

    /// Learns the frame from `events`, then converts every domain-1 position in them to native,
    /// exactly once. When the frame changed, positions already held in wire form are redone.
    pub(super) fn convert_events(&mut self, events: &mut [SessionEvent]) {
        let before = self.wire.frame;
        for event in events.iter_mut() {
            if let SessionEvent::ActivityScope(_) = event {
                self.reset_wire_frame();
            }
            if let SessionEvent::WorldSpatial(delta) = event {
                self.wire.observe_wire(delta.value.actor_position);
            }
            if let SessionEvent::WorldSpatialEntities(delta) = event {
                self.wire.observe_wire(delta.value.actor_position);
            }
            if let SessionEvent::WorldMap(delta) = event {
                self.wire.observe_origin(delta.value.header.origin);
            }
            if let SessionEvent::WorldMapSnapshot(snapshot) = event {
                self.wire.observe_origin(snapshot.value.header.origin);
            }
        }
        let frame = self.wire.frame;
        for event in events.iter_mut() {
            if let SessionEvent::WorldSpatial(delta) = event {
                delta.value.actor_position = frame.to_native(delta.value.actor_position);
            }
            if let SessionEvent::WorldSpatialEntities(delta) = event {
                delta.value.actor_position = frame.to_native(delta.value.actor_position);
                delta
                    .value
                    .enter
                    .iter_mut()
                    .chain(delta.value.update.iter_mut())
                    .for_each(|entity| entity_native(frame, entity));
            }
        }
        if frame != before {
            if !self.entities_wire.is_empty() {
                self.pushed.entities = Some(self.native_entities(&self.entities_wire));
            }
            if !self.appearance_state.is_empty() {
                self.pushed.appearances = Some(self.native_appearances(&self.appearance_state));
            }
        }
    }
}

#[cfg(test)]
#[path = "wire_frame_tests.rs"]
mod tests;
