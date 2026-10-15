//! HOUSE-RUNTIME-1a house-side transition facts (accepted packet §2.1).
//!
//! This is preparation for the three SECURITY DEFINER writers, not a writer or an
//! authority token. Validation proves structural custody/provenance relationships only.
//! The future commit must independently resolve current session, house scope, access
//! revisions and tile membership under the accepted locks. No production caller is wired.

use super::house_scope_handoff::{HOUSE_TILE_MAX_BYTES, HouseId};
use super::item_mint_audit::check_uuid_v7;
use crate::domain::ItemInstanceId;
use crate::foundation::CharacterId;

/// HOUSEOWN0-RL-14, enforced by the future SQL writer under the house lock.
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
pub struct HouseInteriorLocation {
    pub house: HouseId,
    pub spatial_position: Vec<u8>,
    pub stack_ordinal: u64,
    pub scope_ownership_generation: u64,
    pub placement_transaction_id: [u8; 16],
}

impl HouseInteriorLocation {
    pub fn validate(&self) -> Result<(), HouseInteriorError> {
        if self.spatial_position.is_empty()
            || self.spatial_position.len() > HOUSE_TILE_MAX_BYTES
            || self.stack_ordinal == 0
            || self.scope_ownership_generation == 0
            || check_uuid_v7(&self.placement_transaction_id).is_err()
        {
            return Err(HouseInteriorError::InvalidLocation);
        }
        Ok(())
    }
}

/// Migration 0025 provenance: it is never current access authority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HousingReclaimProvenance {
    pub house: HouseId,
    pub item_instance_id: ItemInstanceId,
    pub reclaim_subject: CharacterId,
    pub placement_transaction_id: [u8; 16],
    pub revision: u64,
}

/// The house portion of each accepted shape. Character-side source/destination,
/// transaction identities and command/session fences still belong to the writer/event.
/// The enum prevents missing or duplicate house endpoints without inventing a wire layout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HouseInteriorEffect {
    Place {
        destination: HouseInteriorLocation,
        provenance: HousingReclaimProvenance,
    },
    Take {
        source: HouseInteriorLocation,
        provenance: HousingReclaimProvenance,
    },
    TileMove {
        source: HouseInteriorLocation,
        destination: HouseInteriorLocation,
        provenance_before: HousingReclaimProvenance,
        provenance_after: HousingReclaimProvenance,
    },
}

impl HouseInteriorEffect {
    /// Checks immutable event relationships. It neither authorizes nor mutates anything.
    pub fn validate(
        &self,
        item: ItemInstanceId,
        actor: CharacterId,
    ) -> Result<(), HouseInteriorError> {
        let check = |location: &HouseInteriorLocation, provenance: &HousingReclaimProvenance| {
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
    pub fn check_policy(
        self,
        actor: CharacterId,
        effect: &HouseInteriorEffect,
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
