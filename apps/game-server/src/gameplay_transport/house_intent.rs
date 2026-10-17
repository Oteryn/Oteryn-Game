//! HOUSE_INTENT preparation and native character-ACL writer adapter. No dispatch is installed.
//!
//! Every body supplies an explicit World-qualified target. Its World must equal the current
//! actor World; native HouseId validates the key. The request selects no owner/session/lease. Inputs are
//! data, not evidence of current admission. CP's handler must resolve actual current session/
//! actor/house/property/ACL under owning native locks before calling any mutation writer.

use crate::durability::DurabilityRoot;
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::durability::house_acl::{AclError, AclReceipt, AclSet, rules};
use crate::durability::house_scope_handoff::HouseId;
use crate::durability::item_transfer::CurrentCharacterItemFence;
use crate::durability::runtime_scope_assignment::NodeIncarnationProof;
use crate::foundation::{CharacterId, CommandRef, WorldId};
use oteryn_protocol_oteryn::house::{
    HouseAclRole, HouseAction, HouseWireError, decode_house_intent,
};
use oteryn_protocol_oteryn::world_spatial::ActorPosition;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreparedHouseAction {
    Bid {
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
    house: HouseId,
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
    pub const fn house(&self) -> &HouseId {
        &self.house
    }
    pub const fn action(&self) -> &PreparedHouseAction {
        &self.action
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HouseIntentPreparationError {
    Wire(HouseWireError),
    InvalidHouseKey,
    WorldMismatch,
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
    let request = decode_house_intent(payload).map_err(HouseIntentPreparationError::Wire)?;
    if request.house.world_id != world {
        return Err(HouseIntentPreparationError::WorldMismatch);
    }
    let house = HouseId::new(world, &request.house.house_key)
        .ok_or(HouseIntentPreparationError::InvalidHouseKey)?;
    let action = match request.action {
        HouseAction::Bid { max_gold } => PreparedHouseAction::Bid { max_gold },
        HouseAction::MoveOut { date_unix_ms } => PreparedHouseAction::MoveOut { date_unix_ms },
        HouseAction::CancelMoveOut => PreparedHouseAction::CancelMoveOut,
        HouseAction::AclSet {
            role,
            character_ids,
            expected_revision,
        } => PreparedHouseAction::AclSet {
            role,
            character_ids,
            expected_revision,
        },
        HouseAction::DoorSet {
            position,
            character_ids,
            expected_revision,
        } => PreparedHouseAction::DoorSet {
            position,
            character_ids,
            expected_revision,
        },
        HouseAction::Kick { character_id } => PreparedHouseAction::Kick { character_id },
        HouseAction::Leave => PreparedHouseAction::Leave,
    };
    Ok(PreparedHouseIntent {
        command,
        actor,
        world,
        house,
        action,
    })
}

/// Immutable envelope/fence binding failures, not current ACL or session decisions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HouseAclRequestError {
    ActorMismatch,
    SessionMismatch,
    WorldMismatch,
    NotAclAction,
    InvalidDoor,
    InvalidCharacterIdentity,
}

#[derive(Debug)]
#[allow(
    dead_code,
    reason = "native adapter outcomes are consumed by the later CP ingress slot"
)]
pub(crate) enum HouseAclHandlerError {
    Request(HouseAclRequestError),
    Writer(AclError),
}

/// Convert request values to the single native ACL owner's input without assigning a role,
/// resolving a house from the actor's position, or changing the requested revision.
/// The supplied fence is only expected binding; the writer independently checks its liveness.
fn native_acl_request(
    prepared: &PreparedHouseIntent,
    fence: &CurrentCharacterItemFence,
) -> Result<AclSet, HouseAclRequestError> {
    if prepared.actor.as_bytes() != fence.character_id.as_bytes() {
        return Err(HouseAclRequestError::ActorMismatch);
    }
    if prepared.command.game_session_id() != fence.game_session_id {
        return Err(HouseAclRequestError::SessionMismatch);
    }
    // Use the native scope's World without converting a House/Instance to Channel.
    // Fresh-scope admission and historical replay remain decisions of the native writer.
    if prepared.world != fence.runtime_scope.world_id() {
        return Err(HouseAclRequestError::WorldMismatch);
    }
    let (list, character_ids, expected_revision) = match &prepared.action {
        PreparedHouseAction::AclSet {
            role,
            character_ids,
            expected_revision,
        } => (
            match role {
                HouseAclRole::Subowner => rules::List::Subowners,
                HouseAclRole::Guest => rules::List::Guests,
            },
            character_ids,
            *expected_revision,
        ),
        PreparedHouseAction::DoorSet {
            position,
            character_ids,
            expected_revision,
        } => (
            rules::List::Door(
                super::world_map::tile_pos(*position).ok_or(HouseAclRequestError::InvalidDoor)?,
            ),
            character_ids,
            *expected_revision,
        ),
        _ => return Err(HouseAclRequestError::NotAclAction),
    };
    let entries = character_ids
        .iter()
        .map(|id| {
            crate::domain::CharacterId::from_bytes(*id.as_bytes())
                .map(rules::Entry::Character)
                .map_err(|_| HouseAclRequestError::InvalidCharacterIdentity)
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(AclSet {
        house: prepared.house.clone(),
        command: prepared.command,
        list,
        expected_revision,
        entries,
    })
}

/// Execute only ACL/door replacement through HOUSE-ACL-1's real fenced transaction owner.
/// The ingress owner must first gate HOUSE_V1 and supply its retained command/fence. This
/// adapter creates no success ACK, session authority, role cache, transaction, or SQL writer.
/// It preserves the owner's committed/replayed receipt and every typed or unknown outcome.
/// Guild entries and House-scope composition await their assigned native/wire owners.
#[allow(
    dead_code,
    reason = "native ACL adapter awaits the CP-owned HOUSE_V1 ingress slot"
)]
pub(crate) async fn handle_house_acl_intent(
    root: &DurabilityRoot,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    node: &NodeIncarnationProof,
    fence: CurrentCharacterItemFence,
    prepared: &PreparedHouseIntent,
) -> Result<AclReceipt, HouseAclHandlerError> {
    let request = native_acl_request(prepared, &fence).map_err(HouseAclHandlerError::Request)?;
    root.replace_house_acl(authority, node, fence, request)
        .await
        .map_err(HouseAclHandlerError::Writer)
}

/// Inactive request data only. ProductionKey is the native FirstProductionArea.key type;
/// its city prefix comes from the House catalogue contract. This does not prove that a
/// town exists in the active bundle, or authorize a catalogue/owner/ACL read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedHouseQuery {
    command: CommandRef,
    actor: CharacterId,
    world: WorldId,
    catalogue_town: crate::content::ProductionKey,
    cursor: Option<Vec<u8>>,
}
impl PreparedHouseQuery {
    pub const fn command(&self) -> CommandRef {
        self.command
    }
    pub const fn actor(&self) -> CharacterId {
        self.actor
    }
    pub const fn world(&self) -> WorldId {
        self.world
    }
    pub const fn catalogue_town(&self) -> &crate::content::ProductionKey {
        &self.catalogue_town
    }
    pub fn cursor(&self) -> Option<&[u8]> {
        self.cursor.as_deref()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HouseQueryPreparationError {
    Wire(HouseWireError),
    WorldMismatch,
    InvalidCatalogueTown,
}

pub fn prepare_house_query(
    payload: &[u8],
    command: CommandRef,
    actor: CharacterId,
    world: WorldId,
) -> Result<PreparedHouseQuery, HouseQueryPreparationError> {
    let query = oteryn_protocol_oteryn::house::decode_house_query(payload)
        .map_err(HouseQueryPreparationError::Wire)?;
    if query.world_id != world {
        return Err(HouseQueryPreparationError::WorldMismatch);
    }
    let prefix = "oteryn:content.area.city.";
    if !query
        .catalogue_town
        .strip_prefix(prefix)
        .is_some_and(|slug| !slug.is_empty())
    {
        return Err(HouseQueryPreparationError::InvalidCatalogueTown);
    }
    let catalogue_town = crate::content::ProductionKey::new(&query.catalogue_town)
        .map_err(|_| HouseQueryPreparationError::InvalidCatalogueTown)?;
    Ok(PreparedHouseQuery {
        command,
        actor,
        world,
        catalogue_town,
        cursor: query.cursor.filter(|cursor| !cursor.is_empty()),
    })
}

#[cfg(test)]
#[path = "house_intent_tests.rs"]
mod tests;
