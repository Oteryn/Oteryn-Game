//! Inactive HOUSE_INTENT preparation boundary. No dispatch or writer is installed.
//!
//! Resolves the body house key using native HouseId and the actor's supplied World; the client
//! cannot nominate another owner, World, session, lease or controller in this body. Inputs are
//! data, not evidence of current admission. CP's handler must resolve actual current session/
//! actor/house/property/ACL under owning native locks before calling any mutation writer.

use crate::durability::house_scope_handoff::HouseId;
use crate::foundation::{CharacterId, CommandRef, WorldId};
use oteryn_protocol_oteryn::house::{
    HouseAclRole, HouseIntent, HouseWireError, decode_house_intent,
};
use oteryn_protocol_oteryn::world_spatial::ActorPosition;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreparedHouseAction {
    Bid {
        house: HouseId,
        max_gold: u64,
    },
    MoveOut {
        date_unix_ms: i64,
    },
    CancelMoveOut,
    AclSet {
        role: HouseAclRole,
        character_ids: Vec<CharacterId>,
        expected_revision: u64,
    },
    DoorSet {
        position: ActorPosition,
        character_ids: Vec<CharacterId>,
        expected_revision: u64,
    },
    Kick {
        character_id: CharacterId,
    },
    Leave,
}

/// Immutable request binding; no field attests current authority or an executed outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedHouseIntent {
    command: CommandRef,
    actor: CharacterId,
    world: WorldId,
    action: PreparedHouseAction,
}
impl PreparedHouseIntent {
    pub const fn command(&self) -> CommandRef {
        self.command
    }
    pub const fn actor(&self) -> CharacterId {
        self.actor
    }
    pub const fn world(&self) -> WorldId {
        self.world
    }
    pub const fn action(&self) -> &PreparedHouseAction {
        &self.action
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HouseIntentPreparationError {
    Wire(HouseWireError),
    InvalidHouseKey,
}

/// Prepare syntactic request data. It neither chooses a current actor house nor checks Premium,
/// role, guild membership, funds, dates, target presence, kick rights or ownership. The exact
/// command/session binding must go unchanged into the owning writer's current fence and replay
/// checks. Never return OK or advance a revision merely because this function returned data.
pub fn prepare_house_intent(
    payload: &[u8],
    command: CommandRef,
    actor: CharacterId,
    world: WorldId,
) -> Result<PreparedHouseIntent, HouseIntentPreparationError> {
    let action = match decode_house_intent(payload).map_err(HouseIntentPreparationError::Wire)? {
        HouseIntent::Bid {
            house_key,
            max_gold,
        } => PreparedHouseAction::Bid {
            house: HouseId::new(world, &house_key)
                .ok_or(HouseIntentPreparationError::InvalidHouseKey)?,
            max_gold,
        },
        HouseIntent::MoveOut { date_unix_ms } => PreparedHouseAction::MoveOut { date_unix_ms },
        HouseIntent::CancelMoveOut => PreparedHouseAction::CancelMoveOut,
        HouseIntent::AclSet {
            role,
            character_ids,
            expected_revision,
        } => PreparedHouseAction::AclSet {
            role,
            character_ids,
            expected_revision,
        },
        HouseIntent::DoorSet {
            position,
            character_ids,
            expected_revision,
        } => PreparedHouseAction::DoorSet {
            position,
            character_ids,
            expected_revision,
        },
        HouseIntent::Kick { character_id } => PreparedHouseAction::Kick { character_id },
        HouseIntent::Leave => PreparedHouseAction::Leave,
    };
    Ok(PreparedHouseIntent {
        command,
        actor,
        world,
        action,
    })
}

#[cfg(test)]
#[path = "house_intent_tests.rs"]
mod tests;
