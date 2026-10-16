//! Inactive HOUSE-WIRE-1 body codec, from accepted HOUSE_OWN0 §11 and HOUSE_RUNTIME0 §5.
//!
//! Field tags and flat byte bounds are a protocol-review proposal in `house_v1.proto`.
//! D1170 allocates leaf numbers only; no capability or dispatch is offered. The envelope owns
//! CommandRef and the current session. These bodies contain requests, never authority.

use std::collections::BTreeSet;

pub use crate::charm_wire::CyclopediaWireError as HouseWireError;
use crate::charm_wire::{
    push_message_field, push_nonzero_varint_field, push_varint_field, read_bytes, read_varint,
    set_once,
};
use crate::world_spatial::{ActorPosition, decode_position, encode_position};
use crate::{CharacterId, WorldId};

type Result<T> = std::result::Result<T, HouseWireError>;
/// Native migration0025/HouseId key bound, not a new registered resource row.
pub const MAX_HOUSE_KEY_BYTES: usize = 512;
/// HOUSEOWN0-RL-12, per list.
pub const MAX_HOUSE_ACL_ENTRIES: usize = 200;
/// CP D1170: leaf assignments; registry/capability/dispatch activation remains A2-owned.
pub const CAPABILITY_HOUSE_V1: u32 = 27;
pub const COMMAND_TYPE_HOUSE_INTENT: u32 = 27;
pub const COMMAND_TYPE_HOUSE_QUERY: u32 = 28;
pub const HOUSE_DOMAIN: u32 = 23;
/// Largest native Door action3626 + explicit World/512-byte key target536 +6 spare.
pub const MAX_HOUSE_INTENT_BYTES: usize = 4168;
const MAX_HOUSE_ACTION_BYTES: usize = 3632;

/// Client request identity only; transport verifies World and native HouseId grammar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HouseTarget {
    pub world_id: WorldId,
    pub house_key: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HouseIntent {
    pub house: HouseTarget,
    pub action: HouseAction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HouseAclRole {
    Subowner = 1,
    Guest = 2,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HouseAction {
    Bid {
        max_gold: u64,
    },
    /// Requested Unix milliseconds. The owning writer evaluates1..30 days under its DB clock.
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

fn malformed<T>() -> Result<T> {
    Err(HouseWireError::Malformed)
}

fn validate_ids(ids: &[CharacterId]) -> Result<()> {
    if ids.len() > MAX_HOUSE_ACL_ENTRIES {
        return Err(HouseWireError::LimitExceeded);
    }
    let mut seen = BTreeSet::new();
    for id in ids {
        if !seen.insert(*id.as_bytes()) {
            return malformed();
        }
    }
    Ok(())
}

fn validate(value: &HouseAction) -> Result<()> {
    match value {
        HouseAction::MoveOut { date_unix_ms } if *date_unix_ms <= 0 => return malformed(),
        HouseAction::AclSet { character_ids, .. } => validate_ids(character_ids)?,
        HouseAction::DoorSet {
            position,
            character_ids,
            ..
        } => {
            // Exactly the domain of native world_map::tile_pos; no sign mapping.
            if u16::try_from(position.x).is_err()
                || u16::try_from(position.y).is_err()
                || !(-15..=0).contains(&position.floor)
            {
                return malformed();
            }
            validate_ids(character_ids)?;
        }
        _ => {}
    }
    Ok(())
}

/// Encode a complete body only after validating its complete typed input.
fn encode_house_action(value: &HouseAction) -> Result<Vec<u8>> {
    validate(value)?;
    let mut body = Vec::new();
    let field = match value {
        HouseAction::Bid { max_gold } => {
            push_nonzero_varint_field(&mut body, 2, *max_gold);
            1
        }
        HouseAction::MoveOut { date_unix_ms } => {
            push_varint_field(&mut body, 1, *date_unix_ms as u64);
            2
        }
        HouseAction::CancelMoveOut => 3,
        HouseAction::AclSet {
            role,
            character_ids,
            expected_revision,
        } => {
            push_varint_field(&mut body, 1, *role as u64);
            encode_list(&mut body, character_ids, *expected_revision);
            4
        }
        HouseAction::DoorSet {
            position,
            character_ids,
            expected_revision,
        } => {
            push_message_field(&mut body, 1, &encode_position(position));
            encode_list(&mut body, character_ids, *expected_revision);
            5
        }
        HouseAction::Kick { character_id } => {
            push_message_field(&mut body, 1, character_id.as_bytes());
            6
        }
        HouseAction::Leave => 7,
    };
    let mut output = Vec::new();
    push_message_field(&mut output, field, &body);
    if output.len() > MAX_HOUSE_ACTION_BYTES {
        return Err(HouseWireError::LimitExceeded);
    }
    Ok(output)
}

fn encode_list(body: &mut Vec<u8>, ids: &[CharacterId], revision: u64) {
    for id in ids {
        push_message_field(body, 2, id.as_bytes());
    }
    push_nonzero_varint_field(body, 3, revision);
}

/// Unknown fields, duplicate singular fields, empty/multiple oneofs and invalid identifiers
/// refuse. Unpacked repeated UUID bytes are bounded before allocating another entry.
fn decode_house_action(input: &[u8]) -> Result<HouseAction> {
    if input.len() > MAX_HOUSE_ACTION_BYTES {
        return Err(HouseWireError::LimitExceeded);
    }
    let mut cursor = 0;
    let key = read_varint(input, &mut cursor)?;
    let body = read_bytes(input, &mut cursor)?;
    if cursor != input.len() {
        return malformed();
    }
    let value = match key {
        10 => decode_bid(body)?,
        18 => HouseAction::MoveOut {
            date_unix_ms: i64::try_from(decode_scalar(body)?)
                .map_err(|_| HouseWireError::Malformed)?,
        },
        26 if body.is_empty() => HouseAction::CancelMoveOut,
        34 => decode_acl(body)?,
        42 => decode_door(body)?,
        50 => {
            let mut c = 0;
            if read_varint(body, &mut c)? != 10 {
                return malformed();
            }
            let character_id = decode_id(read_bytes(body, &mut c)?)?;
            if c != body.len() {
                return malformed();
            }
            HouseAction::Kick { character_id }
        }
        58 if body.is_empty() => HouseAction::Leave,
        _ => return malformed(),
    };
    validate(&value)?;
    Ok(value)
}

fn decode_id(bytes: &[u8]) -> Result<CharacterId> {
    CharacterId::decode(bytes).map_err(|_| HouseWireError::Malformed)
}
fn decode_scalar(body: &[u8]) -> Result<u64> {
    let mut cursor = 0;
    if read_varint(body, &mut cursor)? != 8 {
        return malformed();
    }
    let value = read_varint(body, &mut cursor)?;
    if cursor != body.len() {
        return malformed();
    }
    Ok(value)
}
fn decode_bid(body: &[u8]) -> Result<HouseAction> {
    let mut gold = None;
    let mut cursor = 0;
    while cursor < body.len() {
        match read_varint(body, &mut cursor)? {
            16 => set_once(&mut gold, read_varint(body, &mut cursor)?)?,
            _ => return malformed(),
        }
    }
    Ok(HouseAction::Bid {
        max_gold: gold.unwrap_or(0),
    })
}
fn read_id_list(body: &[u8], cursor: &mut usize, ids: &mut Vec<CharacterId>) -> Result<()> {
    if ids.len() == MAX_HOUSE_ACL_ENTRIES {
        return Err(HouseWireError::LimitExceeded);
    }
    ids.push(decode_id(read_bytes(body, cursor)?)?);
    Ok(())
}
fn decode_acl(body: &[u8]) -> Result<HouseAction> {
    let (mut role, mut revision) = (None, None);
    let mut ids = Vec::new();
    let mut cursor = 0;
    while cursor < body.len() {
        match read_varint(body, &mut cursor)? {
            8 => set_once(
                &mut role,
                match read_varint(body, &mut cursor)? {
                    1 => HouseAclRole::Subowner,
                    2 => HouseAclRole::Guest,
                    _ => return malformed(),
                },
            )?,
            18 => read_id_list(body, &mut cursor, &mut ids)?,
            24 => set_once(&mut revision, read_varint(body, &mut cursor)?)?,
            _ => return malformed(),
        }
    }
    Ok(HouseAction::AclSet {
        role: role.ok_or(HouseWireError::Malformed)?,
        character_ids: ids,
        expected_revision: revision.unwrap_or(0),
    })
}
fn decode_door(body: &[u8]) -> Result<HouseAction> {
    let (mut position, mut revision) = (None, None);
    let mut ids = Vec::new();
    let mut cursor = 0;
    while cursor < body.len() {
        match read_varint(body, &mut cursor)? {
            10 => set_once(
                &mut position,
                decode_position(read_bytes(body, &mut cursor)?)
                    .map_err(|_| HouseWireError::Malformed)?,
            )?,
            18 => read_id_list(body, &mut cursor, &mut ids)?,
            24 => set_once(&mut revision, read_varint(body, &mut cursor)?)?,
            _ => return malformed(),
        }
    }
    Ok(HouseAction::DoorSet {
        position: position.ok_or(HouseWireError::Malformed)?,
        character_ids: ids,
        expected_revision: revision.unwrap_or(0),
    })
}

/// Complete D1170 intent. Every action has exactly one explicit World-qualified target.
/// The target is request data and never attests current admission or placement.
pub fn encode_house_intent(value: &HouseIntent) -> Result<Vec<u8>> {
    validate_target(&value.house)?;
    let mut output = encode_house_action(&value.action)?;
    let mut target = Vec::new();
    push_message_field(&mut target, 1, value.house.world_id.as_bytes());
    push_message_field(&mut target, 2, value.house.house_key.as_bytes());
    push_message_field(&mut output, 8, &target);
    if output.len() > MAX_HOUSE_INTENT_BYTES {
        return Err(HouseWireError::LimitExceeded);
    }
    Ok(output)
}
fn validate_target(target: &HouseTarget) -> Result<()> {
    if target.house_key.len() > MAX_HOUSE_KEY_BYTES {
        return Err(HouseWireError::LimitExceeded);
    }
    if target.house_key.is_empty() {
        return malformed();
    }
    Ok(())
}
fn decode_target(input: &[u8]) -> Result<HouseTarget> {
    let (mut world, mut key) = (None, None);
    let mut cursor = 0;
    while cursor < input.len() {
        match read_varint(input, &mut cursor)? {
            10 => set_once(
                &mut world,
                WorldId::decode(read_bytes(input, &mut cursor)?)
                    .map_err(|_| HouseWireError::Malformed)?,
            )?,
            18 => {
                let bytes = read_bytes(input, &mut cursor)?;
                if bytes.len() > MAX_HOUSE_KEY_BYTES {
                    return Err(HouseWireError::LimitExceeded);
                }
                set_once(
                    &mut key,
                    std::str::from_utf8(bytes)
                        .map_err(|_| HouseWireError::Malformed)?
                        .to_owned(),
                )?;
            }
            _ => return malformed(),
        }
    }
    let target = HouseTarget {
        world_id: world.ok_or(HouseWireError::Malformed)?,
        house_key: key.ok_or(HouseWireError::Malformed)?,
    };
    validate_target(&target)?;
    Ok(target)
}
pub fn decode_house_intent(input: &[u8]) -> Result<HouseIntent> {
    if input.len() > MAX_HOUSE_INTENT_BYTES {
        return Err(HouseWireError::LimitExceeded);
    }
    let (mut house, mut action) = (None, None);
    let mut cursor = 0;
    while cursor < input.len() {
        let start = cursor;
        let key = read_varint(input, &mut cursor)?;
        let body = read_bytes(input, &mut cursor)?;
        match key {
            66 => set_once(&mut house, decode_target(body)?)?,
            10 | 18 | 26 | 34 | 42 | 50 | 58 => {
                set_once(&mut action, decode_house_action(&input[start..cursor])?)?
            }
            _ => return malformed(),
        }
    }
    Ok(HouseIntent {
        house: house.ok_or(HouseWireError::Malformed)?,
        action: action.ok_or(HouseWireError::Malformed)?,
    })
}

#[cfg(test)]
#[path = "house_tests.rs"]
mod tests;
