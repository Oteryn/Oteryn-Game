//! HOUSE-TRANSFER-1: bounded scope-changing command and post-commit acknowledgement.
use crate::charm_wire::CyclopediaWireError as Error;
use crate::charm_wire::{
    push_message_field as bytes, push_varint_field as scalar, read_bytes, read_varint, set_once,
};

pub const CAPABILITY_HOUSE_TRANSFER_V1: u32 = 86;
pub const COMMAND_TYPE_HOUSE_TRANSFER: u32 = 102;
pub const STATE_DOMAIN_HOUSE_SCOPE: u32 = 51;
pub const MAX_INTENT_BYTES: usize = 540;
pub const MAX_ACK_BYTES: usize = 2048;

/// The client names no destination generation, Channel or selected capability.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HouseTransferIntent {
    pub exit: bool,
    pub house: String,
}

/// Immutable committed result; current authority must be checked separately on recovery.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HouseTransferAck {
    pub transfer: [u8; 16],
    pub session: [u8; 16],
    pub world: [u8; 16],
    /// Recorded origin; the destination is this Channel exactly when `house` is absent.
    pub channel: [u8; 16],
    pub house: Option<String>,
    pub scope_generation: u64,
    pub connection_generation: u64,
    pub lease_generation: u64,
    pub selection: Vec<u32>,
    pub position: [i32; 3],
}

fn house(value: &[u8]) -> Result<String, Error> {
    let text = std::str::from_utf8(value).map_err(|_| Error::Malformed)?;
    let suffix = text
        .strip_prefix("oteryn:content.house.")
        .filter(|s| !s.is_empty())
        .ok_or(Error::Malformed)?;
    if value.len() > 512
        || !suffix
            .bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'_')
    {
        return Err(Error::Malformed);
    }
    Ok(text.to_owned())
}

pub fn decode_house_transfer_intent(body: &[u8]) -> Result<HouseTransferIntent, Error> {
    if body.len() > MAX_INTENT_BYTES {
        return Err(Error::LimitExceeded);
    }
    let (mut cursor, mut direction, mut key) = (0, None, None);
    while cursor < body.len() {
        match read_varint(body, &mut cursor)? {
            8 => set_once(&mut direction, read_varint(body, &mut cursor)?)?,
            18 => set_once(&mut key, house(read_bytes(body, &mut cursor)?)?)?,
            _ => return Err(Error::Malformed),
        }
    }
    let exit = match direction {
        Some(1) => false,
        Some(2) => true,
        _ => return Err(Error::Malformed),
    };
    Ok(HouseTransferIntent {
        exit,
        house: key.ok_or(Error::Malformed)?,
    })
}

/// Empty is a refusal, never a grant. Every successful encoding includes the full selection.
pub fn encode_house_transfer_ack(ack: &HouseTransferAck) -> Result<Vec<u8>, Error> {
    for id in [ack.transfer, ack.session, ack.world, ack.channel] {
        if id[6] >> 4 != 7 || id[8] & 0xc0 != 0x80 {
            return Err(Error::Malformed);
        }
    }
    if ack.scope_generation == 0
        || ack.connection_generation == 0
        || ack.lease_generation == 0
        || ack.selection.len() > crate::MAX_CAPABILITY_COUNT
        || ack.selection.windows(2).any(|v| v[0] >= v[1])
        || ack
            .selection
            .binary_search(&CAPABILITY_HOUSE_TRANSFER_V1)
            .is_err()
    {
        return Err(Error::Malformed);
    }
    let mut out = Vec::new();
    for (tag, id) in [
        (1, ack.transfer),
        (2, ack.session),
        (3, ack.world),
        (4, ack.channel),
    ] {
        bytes(&mut out, tag, &id);
    }
    if let Some(key) = &ack.house {
        house(key.as_bytes())?;
        bytes(&mut out, 5, key.as_bytes());
    }
    scalar(&mut out, 6, ack.scope_generation);
    scalar(&mut out, 7, ack.connection_generation);
    scalar(&mut out, 8, ack.lease_generation);
    for id in &ack.selection {
        scalar(&mut out, 9, u64::from(*id));
    }
    for (tag, coordinate) in [
        (10, ack.position[0]),
        (11, ack.position[1]),
        (12, ack.position[2]),
    ] {
        let encoded = (i64::from(coordinate) << 1) ^ (i64::from(coordinate) >> 63);
        scalar(&mut out, tag, encoded as u64);
    }
    if out.len() > MAX_ACK_BYTES {
        return Err(Error::LimitExceeded);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn house_transfer_intent_refuses_unknown_duplicate_and_oversized_input() {
        let mut body = vec![8, 1];
        bytes(&mut body, 2, b"oteryn:content.house.test");
        assert_eq!(
            decode_house_transfer_intent(&body).map(|v| v.exit),
            Ok(false)
        );
        let mut duplicate = body.clone();
        duplicate.extend([8, 2]);
        assert!(decode_house_transfer_intent(&duplicate).is_err());
        body[1] = 3;
        assert!(decode_house_transfer_intent(&body).is_err());
        assert!(decode_house_transfer_intent(&vec![0; MAX_INTENT_BYTES + 1]).is_err());
        assert!(decode_house_transfer_intent(&[]).is_err());
    }
}
