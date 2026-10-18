//! BED0 §5.2 persisted data. These values are not current house/bed authority.

use crate::content::LogicalCell;
use crate::domain::CharacterId;
use crate::house_scope::HouseId;
use std::num::NonZeroU64;

/// The stored head coordinate in the native World-global house identity.
/// This does not prove that the active map still contains a valid bed pair.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredBedKey {
    pub house: HouseId,
    pub head: LogicalCell,
}

// Persisted variant uses the existing native sex representation, not new truth.
pub use crate::domain::appearance::Sex as OccupiedVariant;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FreedCause {
    OwnerWake,
    OwnerChange,
    Disposition,
    BedRemoved,
}

impl FreedCause {
    #[must_use]
    pub fn from_stored(value: &str) -> Option<Self> {
        match value {
            "owner_wake" => Some(Self::OwnerWake),
            "owner_change" => Some(Self::OwnerChange),
            "disposition" => Some(Self::Disposition),
            "bed_removed" => Some(Self::BedRemoved),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FreedSleeper {
    /// Unix milliseconds, the native build receipt timestamp representation.
    pub at: i64,
    pub cause: FreedCause,
}

/// Raw persistence snapshot; neither a logout marker nor an award/settlement grant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredSleeper {
    pub bed: StoredBedKey,
    pub character: CharacterId,
    pub lease_generation: NonZeroU64,
    pub variant: OccupiedVariant,
    /// Exact logout marker committed_at in native Unix milliseconds.
    pub slept_at: i64,
    pub freed: Option<FreedSleeper>,
    pub occupancy_revision: NonZeroU64,
}

/// One raw storage snapshot for an exact bed key, including its House-wide
/// occupancy revision. This is not proof that the key names an active bed,
/// that the House exists, or that a character may use or wake it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredBedOccupancy {
    pub bed: StoredBedKey,
    /// None means no revision row was observed. It does not establish that the
    /// House or bed is new; callers still need their actual native owners.
    pub occupancy_revision: Option<NonZeroU64>,
    /// Only an open sleeper can occupy this key. Freed rows remain available
    /// to their character's settlement and are not returned as occupants.
    pub sleeper: Option<StoredSleeper>,
}

#[cfg(test)]
#[path = "sleeper_tests.rs"]
mod tests;
