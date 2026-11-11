//! Successful key openings exposed without introducing another door-state owner.
use super::{DoorBook, DoorUse};

/// An opening produced only by a successful native key-door transition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeyDoorOpening {
    placement_key: u64,
    key_number: u16,
    revision: u64,
}

impl KeyDoorOpening {
    /// Native bundle placement key; no coordinate conversion is implied.
    pub const fn placement_key(self) -> u64 {
        self.placement_key
    }
    /// The key action number the native door actually accepted.
    pub const fn key_number(self) -> u16 {
        self.key_number
    }
    /// The new revision of this successful opening.
    pub const fn revision(self) -> u64 {
        self.revision
    }
}

impl DoorBook {
    /// Apply the existing key rules and expose an opening observation.
    /// Refusals, stale attempts and locking never produce an observation.
    pub fn use_key_observed(
        &self,
        key: u64,
        expected_revision: u64,
        action: u16,
    ) -> (Option<DoorUse>, Option<KeyDoorOpening>) {
        let outcome = self.use_key(key, expected_revision, action);
        let opening = match outcome {
            Some(DoorUse::Toggled {
                open: true,
                revision,
            }) => Some(KeyDoorOpening {
                placement_key: key,
                key_number: action,
                revision,
            }),
            _ => None,
        };
        (outcome, opening)
    }
}
