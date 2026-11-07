//! Pure house interior endpoint, provenance and access rules (HOUSE-RUNTIME-1a).

use super::house_scope_handoff::{HOUSE_TILE_MAX_BYTES, HouseId};
use oteryn_gameplay_domain::domain::{CharacterId, ItemInstanceId};

/// HOUSEOWN0-RL-14, enforced by the native SQL shapes under the house lock.
pub const HOUSE_INTERIOR_ITEMS_MAX: u64 = 2_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HouseInteriorError {
    InvalidLocation,
    InvalidProvenance,
    CrossHouseMove,
    NoAccess,
}

/// House endpoint audit facts, not proof of tile membership or current scope authority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HouseInteriorLocation<H = HouseId> {
    pub house: H,
    pub spatial_position: Vec<u8>,
    pub stack_ordinal: u64,
    pub scope_ownership_generation: u64,
    pub placement_transaction_id: [u8; 16],
}

impl<H> HouseInteriorLocation<H> {
    pub fn validate(&self) -> Result<(), HouseInteriorError> {
        if self.spatial_position.is_empty()
            || self.spatial_position.len() > HOUSE_TILE_MAX_BYTES
            || self.stack_ordinal == 0
            || self.scope_ownership_generation == 0
            || ItemInstanceId::from_bytes(self.placement_transaction_id).is_err()
        {
            return Err(HouseInteriorError::InvalidLocation);
        }
        Ok(())
    }
}

/// Migration 0025 provenance: it is never current access authority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HousingReclaimProvenance<H = HouseId, C = CharacterId> {
    pub house: H,
    pub item_instance_id: ItemInstanceId,
    pub reclaim_subject: C,
    pub placement_transaction_id: [u8; 16],
    pub revision: u64,
}

/// The house portion of each accepted shape. Character-side source/destination,
/// transaction identities and command/session fences still belong to the writer/event.
/// The enum prevents missing or duplicate house endpoints without inventing a wire layout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HouseInteriorEffect<H = HouseId, C = CharacterId> {
    Place {
        destination: HouseInteriorLocation<H>,
        provenance: HousingReclaimProvenance<H, C>,
    },
    Take {
        source: HouseInteriorLocation<H>,
        provenance: HousingReclaimProvenance<H, C>,
    },
    TileMove {
        source: HouseInteriorLocation<H>,
        destination: HouseInteriorLocation<H>,
        provenance_before: HousingReclaimProvenance<H, C>,
        provenance_after: HousingReclaimProvenance<H, C>,
    },
}

impl<H: PartialEq, C: PartialEq> HouseInteriorEffect<H, C> {
    /// Checks immutable event relationships. It neither authorizes nor mutates anything.
    pub fn validate(&self, item: ItemInstanceId, actor: C) -> Result<(), HouseInteriorError> {
        let check = |location: &HouseInteriorLocation<H>,
                     provenance: &HousingReclaimProvenance<H, C>| {
            location.validate()?;
            if provenance.house != location.house
                || provenance.item_instance_id != item
                || provenance.revision == 0
                || provenance.placement_transaction_id != location.placement_transaction_id
            {
                return Err(HouseInteriorError::InvalidProvenance);
            }
            Ok(())
        };
        match self {
            Self::Place {
                destination,
                provenance,
            } => {
                check(destination, provenance)?;
                if provenance.reclaim_subject != actor {
                    return Err(HouseInteriorError::InvalidProvenance);
                }
            }
            Self::Take { source, provenance } => check(source, provenance)?,
            Self::TileMove {
                source,
                destination,
                provenance_before,
                provenance_after,
            } => {
                check(source, provenance_before)?;
                check(destination, provenance_after)?;
                if source.house != destination.house {
                    return Err(HouseInteriorError::CrossHouseMove);
                }
                if source.scope_ownership_generation != destination.scope_ownership_generation
                    || source.placement_transaction_id == destination.placement_transaction_id
                    || provenance_before.reclaim_subject != provenance_after.reclaim_subject
                    || provenance_before.revision.checked_add(1) != Some(provenance_after.revision)
                {
                    return Err(HouseInteriorError::InvalidProvenance);
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HouseItemRole {
    Owner,
    Subowner,
    Guest,
}

impl HouseItemRole {
    /// Exact roles returned by game_house_access; unknown roles fail closed.
    pub fn from_access_role(role: &str) -> Option<Self> {
        match role {
            "OWNER" => Some(Self::Owner),
            "SUBOWNER" => Some(Self::Subowner),
            "GUEST" => Some(Self::Guest),
            _ => None,
        }
    }

    /// HOUSE-RUNTIME-0 §5.2 policy only. The SQL commit must resolve this role
    /// afresh under locks and compare every authorization revision.
    pub fn check_policy<H, C: PartialEq>(
        self,
        actor: C,
        effect: &HouseInteriorEffect<H, C>,
    ) -> Result<(), HouseInteriorError> {
        if let HouseInteriorEffect::Take { provenance, .. } = effect
            && self == Self::Guest
            && provenance.reclaim_subject != actor
        {
            return Err(HouseInteriorError::NoAccess);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HouseCharacterSource {
    Inventory { parent: [u8; 16], ordinal: u64 },
    ContainerSlot,
    Equipment { slot: u8 },
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HouseItemAction {
    Place {
        source: HouseCharacterSource,
        position: Vec<u8>,
    },
    Take {
        placement: [u8; 16],
        provenance_revision: u64,
    },
    TileMove {
        placement: [u8; 16],
        provenance_revision: u64,
        position: Vec<u8>,
    },
}
impl HouseItemAction {
    pub fn shape(&self) -> i16 {
        match self {
            Self::Place { .. } => 5,
            Self::Take { .. } => 6,
            Self::TileMove { .. } => 7,
        }
    }
    pub fn source_kind(&self) -> i16 {
        match self {
            Self::Place {
                source: HouseCharacterSource::Inventory { .. },
                ..
            } => 1,
            Self::Place {
                source: HouseCharacterSource::ContainerSlot,
                ..
            } => 2,
            Self::Place {
                source: HouseCharacterSource::Equipment { .. },
                ..
            } => 3,
            _ => 4,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HouseItemReceipt {
    pub transaction_id: [u8; 16],
    pub event_id: [u8; 16],
    pub occurred_at_unix_ms: i64,
    pub envelope_sha256: [u8; 32],
    /// Present only while the independently expirable audit outbox retains it.
    pub envelope: Option<Vec<u8>>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HouseItemOutcome {
    Committed(HouseItemReceipt),
    AlreadyCommitted(HouseItemReceipt),
}

#[cfg(test)]
#[path = "house_interior_tests.rs"]
mod tests;
