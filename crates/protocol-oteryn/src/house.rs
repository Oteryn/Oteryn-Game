//! Inactive HOUSE-WIRE-1 body codec, from accepted HOUSE_OWN0 §11 and HOUSE_RUNTIME0 §5.
//!
//! Field tags and flat byte bounds are a protocol-review proposal in `house_v1.proto`.
//! No capability/opcode/domain numbers are allocated or offered here. The envelope owns
//! CommandRef and the current session. These bodies contain requests, never authority.

use std::collections::BTreeSet;

use crate::CharacterId;
pub use crate::charm_wire::CyclopediaWireError as HouseWireError;
use crate::charm_wire::{
    push_message_field, push_nonzero_varint_field, push_varint_field, read_bytes, read_varint,
    set_once,
};
use crate::world_spatial::{ActorPosition, decode_position, encode_position};

type Result<T> = std::result::Result<T, HouseWireError>;
/// Native migration0025/HouseId key bound, not a new registered resource row.
pub const MAX_HOUSE_KEY_BYTES: usize = 512;
/// HOUSEOWN0-RL-12, per list.
pub const MAX_HOUSE_ACL_ENTRIES: usize = 200;
/// Proposed flat bound:200 UUID fields(3600), native position message(12), revision(11), envelope(3), with6 spare bytes.
pub const MAX_HOUSE_INTENT_BYTES: usize = 3632;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HouseAclRole {
    Subowner = 1,
    Guest = 2,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HouseIntent {
    Bid {
        house_key: String,
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

fn validate(value: &HouseIntent) -> Result<()> {
    match value {
        HouseIntent::Bid { house_key, .. } => {
            if house_key.len() > MAX_HOUSE_KEY_BYTES {
                return Err(HouseWireError::LimitExceeded);
            }
            // Grammar is resolved by native HouseId at the transport boundary.
            if house_key.is_empty() {
                return malformed();
            }
        }
        HouseIntent::MoveOut { date_unix_ms } if *date_unix_ms <= 0 => return malformed(),
        HouseIntent::AclSet { character_ids, .. } => validate_ids(character_ids)?,
        HouseIntent::DoorSet {
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
pub fn encode_house_intent(value: &HouseIntent) -> Result<Vec<u8>> {
    validate(value)?;
    let mut body = Vec::new();
    let field = match value {
        HouseIntent::Bid {
            house_key,
            max_gold,
        } => {
            push_message_field(&mut body, 1, house_key.as_bytes());
            push_nonzero_varint_field(&mut body, 2, *max_gold);
            1
        }
        HouseIntent::MoveOut { date_unix_ms } => {
            push_varint_field(&mut body, 1, *date_unix_ms as u64);
            2
        }
        HouseIntent::CancelMoveOut => 3,
        HouseIntent::AclSet {
            role,
            character_ids,
            expected_revision,
        } => {
            push_varint_field(&mut body, 1, *role as u64);
            encode_list(&mut body, character_ids, *expected_revision);
            4
        }
        HouseIntent::DoorSet {
            position,
            character_ids,
            expected_revision,
        } => {
            push_message_field(&mut body, 1, &encode_position(position));
            encode_list(&mut body, character_ids, *expected_revision);
            5
        }
        HouseIntent::Kick { character_id } => {
            push_message_field(&mut body, 1, character_id.as_bytes());
            6
        }
        HouseIntent::Leave => 7,
    };
    let mut output = Vec::new();
    push_message_field(&mut output, field, &body);
    if output.len() > MAX_HOUSE_INTENT_BYTES {
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
pub fn decode_house_intent(input: &[u8]) -> Result<HouseIntent> {
    if input.len() > MAX_HOUSE_INTENT_BYTES {
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
        18 => HouseIntent::MoveOut {
            date_unix_ms: i64::try_from(decode_scalar(body)?)
                .map_err(|_| HouseWireError::Malformed)?,
        },
        26 if body.is_empty() => HouseIntent::CancelMoveOut,
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
            HouseIntent::Kick { character_id }
        }
        58 if body.is_empty() => HouseIntent::Leave,
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
fn decode_bid(body: &[u8]) -> Result<HouseIntent> {
    let (mut key, mut gold) = (None, None);
    let mut cursor = 0;
    while cursor < body.len() {
        match read_varint(body, &mut cursor)? {
            10 => {
                let bytes = read_bytes(body, &mut cursor)?;
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
            16 => set_once(&mut gold, read_varint(body, &mut cursor)?)?,
            _ => return malformed(),
        }
    }
    Ok(HouseIntent::Bid {
        house_key: key.ok_or(HouseWireError::Malformed)?,
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
fn decode_acl(body: &[u8]) -> Result<HouseIntent> {
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
    Ok(HouseIntent::AclSet {
        role: role.ok_or(HouseWireError::Malformed)?,
        character_ids: ids,
        expected_revision: revision.unwrap_or(0),
    })
}
fn decode_door(body: &[u8]) -> Result<HouseIntent> {
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
    Ok(HouseIntent::DoorSet {
        position: position.ok_or(HouseWireError::Malformed)?,
        character_ids: ids,
        expected_revision: revision.unwrap_or(0),
    })
}

#[cfg(test)]
#[path = "house_tests.rs"]
mod tests;
