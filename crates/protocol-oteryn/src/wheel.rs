//! Wheel of Destiny allocation wire contract.
use crate::charm_wire::{CyclopediaWireError as Error, push_varint_field, read_varint, set_once};

pub const CAPABILITY_WHEEL_ALLOCATION_V1: u32 = 148;
pub const COMMAND_TYPE_WHEEL_ALLOCATION: u32 = 161;
pub const WHEEL_SLOTS: usize = 36;
pub const WHEEL_VESSELS: usize = 4;
pub const MAX_REQUEST_BYTES: usize = 256;
pub const MAX_REPLY_BYTES: usize = 256;

/// Green, red, purple, blue. Gem identities are opaque 128-bit owner-issued values.
pub type WheelVessels = [Option<[u8; 16]>; WHEEL_VESSELS];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WheelAllocationRequest {
    /// 0 reads, 1 replaces the allocation, 2 resets it at a temple.
    pub action: u8,
    pub occurrence: [u8; 16],
    pub expected_revision: u64,
    pub points: [u16; WHEEL_SLOTS],
    /// Absent preserves vessels; present atomically replaces all four with the allocation.
    pub vessels: Option<WheelVessels>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WheelAllocationReply {
    pub refused: bool,
    pub revision: u64,
    pub points: [u16; WHEEL_SLOTS],
    /// Returned only for a request using the vessel extension, preserving legacy replies.
    pub vessels: Option<WheelVessels>,
}

pub fn encode_wheel_request(r: &WheelAllocationRequest) -> Result<Vec<u8>, Error> {
    if r.action > 2
        || (r.action == 0
            && (r.expected_revision != 0
                || r.occurrence != [0; 16]
                || r.points != [0; WHEEL_SLOTS]
                || r.vessels.is_some()))
        || (r.action == 2 && r.points != [0; WHEEL_SLOTS])
    {
        return Err(Error::Malformed);
    }
    let mut out = Vec::new();
    push_varint_field(&mut out, 1, u64::from(r.action));
    if r.action != 0 {
        crate::charm_wire::push_message_field(&mut out, 2, &r.occurrence);
        push_varint_field(&mut out, 3, r.expected_revision);
        for point in r.points {
            push_varint_field(&mut out, 4, u64::from(point));
        }
        encode_vessels(&mut out, 5, r.vessels.as_ref())?;
    }
    if out.len() > MAX_REQUEST_BYTES {
        return Err(Error::LimitExceeded);
    }
    Ok(out)
}

pub fn decode_wheel_request(bytes: &[u8]) -> Result<WheelAllocationRequest, Error> {
    if bytes.len() > MAX_REQUEST_BYTES {
        return Err(Error::LimitExceeded);
    }
    let (mut action, mut occurrence, mut revision) = (None, None, None);
    let mut points = Vec::new();
    let mut vessels = Vec::new();
    let mut at = 0;
    while at < bytes.len() {
        match read_varint(bytes, &mut at)? {
            8 => set_once(
                &mut action,
                u8::try_from(read_varint(bytes, &mut at)?).map_err(|_| Error::Malformed)?,
            )?,
            18 => {
                let value = crate::charm_wire::read_bytes(bytes, &mut at)?;
                if value.len() != 16 || occurrence.is_some() {
                    return Err(Error::Malformed);
                }
                let mut id = [0; 16];
                id.copy_from_slice(value);
                occurrence = Some(id);
            }
            24 => set_once(&mut revision, read_varint(bytes, &mut at)?)?,
            32 => points
                .push(u16::try_from(read_varint(bytes, &mut at)?).map_err(|_| Error::Malformed)?),
            42 => {
                if vessels.len() == WHEEL_VESSELS {
                    return Err(Error::Malformed);
                }
                let value = crate::charm_wire::read_bytes(bytes, &mut at)?;
                let gem = if value.is_empty() {
                    None
                } else {
                    Some(value.try_into().map_err(|_| Error::Malformed)?)
                };
                vessels.push(gem);
            }
            _ => return Err(Error::Malformed),
        }
    }
    let action = action.ok_or(Error::Malformed)?;
    let r = if action == 0 {
        if occurrence.is_some() || revision.is_some() || !points.is_empty() || !vessels.is_empty() {
            return Err(Error::Malformed);
        }
        WheelAllocationRequest {
            action,
            occurrence: [0; 16],
            expected_revision: 0,
            points: [0; WHEEL_SLOTS],
            vessels: None,
        }
    } else {
        if points.len() != WHEEL_SLOTS {
            return Err(Error::Malformed);
        }
        WheelAllocationRequest {
            action,
            occurrence: occurrence.ok_or(Error::Malformed)?,
            expected_revision: revision.ok_or(Error::Malformed)?,
            points: points.try_into().map_err(|_| Error::Malformed)?,
            vessels: if vessels.is_empty() {
                None
            } else {
                Some(vessels.try_into().map_err(|_| Error::Malformed)?)
            },
        }
    };
    encode_wheel_request(&r)?;
    Ok(r)
}

pub fn encode_wheel_reply(r: &WheelAllocationReply) -> Result<Vec<u8>, Error> {
    let mut out = Vec::new();
    push_varint_field(&mut out, 1, u64::from(r.refused));
    push_varint_field(&mut out, 2, r.revision);
    for point in r.points {
        push_varint_field(&mut out, 3, u64::from(point));
    }
    encode_vessels(&mut out, 4, r.vessels.as_ref())?;
    if out.len() > MAX_REPLY_BYTES {
        Err(Error::LimitExceeded)
    } else {
        Ok(out)
    }
}

/// Decode the bounded server projection, preserving an absent legacy vessel extension.
pub fn decode_wheel_reply(bytes: &[u8]) -> Result<WheelAllocationReply, Error> {
    if bytes.len() > MAX_REPLY_BYTES {
        return Err(Error::LimitExceeded);
    }
    let (mut refused, mut revision) = (None, None);
    let mut points = Vec::with_capacity(WHEEL_SLOTS);
    let mut vessels = Vec::with_capacity(WHEEL_VESSELS);
    let mut at = 0;
    while at < bytes.len() {
        match read_varint(bytes, &mut at)? {
            8 => set_once(
                &mut refused,
                match read_varint(bytes, &mut at)? {
                    0 => false,
                    1 => true,
                    _ => return Err(Error::Malformed),
                },
            )?,
            16 => set_once(&mut revision, read_varint(bytes, &mut at)?)?,
            24 => {
                if points.len() == WHEEL_SLOTS {
                    return Err(Error::Malformed);
                }
                points.push(
                    u16::try_from(read_varint(bytes, &mut at)?).map_err(|_| Error::Malformed)?,
                );
            }
            34 => {
                if vessels.len() == WHEEL_VESSELS {
                    return Err(Error::Malformed);
                }
                let value = crate::charm_wire::read_bytes(bytes, &mut at)?;
                vessels.push(if value.is_empty() {
                    None
                } else {
                    Some(value.try_into().map_err(|_| Error::Malformed)?)
                });
            }
            _ => return Err(Error::Malformed),
        }
    }
    let reply = WheelAllocationReply {
        refused: refused.ok_or(Error::Malformed)?,
        revision: revision.ok_or(Error::Malformed)?,
        points: points.try_into().map_err(|_| Error::Malformed)?,
        vessels: if vessels.is_empty() {
            None
        } else {
            Some(vessels.try_into().map_err(|_| Error::Malformed)?)
        },
    };
    // The common encoder also rejects duplicate nonempty gem identities.
    encode_wheel_reply(&reply)?;
    Ok(reply)
}

fn encode_vessels(
    out: &mut Vec<u8>,
    field: u64,
    vessels: Option<&WheelVessels>,
) -> Result<(), Error> {
    if let Some(vessels) = vessels {
        for (index, gem) in vessels.iter().enumerate() {
            if gem.is_some() && vessels[..index].contains(gem) {
                return Err(Error::Malformed);
            }
            crate::charm_wire::push_message_field(
                out,
                field,
                gem.as_ref().map_or(&[][..], |id| id.as_slice()),
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    const OCCURRENCE: [u8; 16] = [7, 7, 7, 7, 7, 7, 0x70, 7, 0x80, 7, 7, 7, 7, 7, 7, 7];

    #[test]
    fn wheel_reply_round_trips_legacy_clear_and_full_vessels() -> Result<(), Error> {
        let legacy = WheelAllocationReply {
            refused: false,
            revision: 0,
            points: [0; WHEEL_SLOTS],
            vessels: None,
        };
        let mut golden = vec![8, 0, 16, 0];
        for _ in 0..WHEEL_SLOTS {
            golden.extend_from_slice(&[24, 0]);
        }
        assert_eq!(encode_wheel_reply(&legacy)?, golden);
        assert_eq!(decode_wheel_reply(&golden)?, legacy);
        for vessels in [
            Some([None; WHEEL_VESSELS]),
            Some([Some([0; 16]), Some([1; 16]), Some([2; 16]), Some([3; 16])]),
        ] {
            let reply = WheelAllocationReply {
                refused: true,
                revision: u64::MAX,
                points: [u16::MAX; WHEEL_SLOTS],
                vessels,
            };
            let bytes = encode_wheel_reply(&reply)?;
            assert!(bytes.len() <= MAX_REPLY_BYTES);
            assert_eq!(decode_wheel_reply(&bytes)?, reply);
        }
        Ok(())
    }

    #[test]
    fn wheel_reply_refuses_invalid_scalars_cardinality_tags_and_truncation() -> Result<(), Error> {
        let reply = WheelAllocationReply {
            refused: false,
            revision: 7,
            points: [0; WHEEL_SLOTS],
            vessels: None,
        };
        let valid = encode_wheel_reply(&reply)?;
        assert_eq!(
            decode_wheel_reply(&vec![0; MAX_REPLY_BYTES + 1]),
            Err(Error::LimitExceeded)
        );
        for bytes in [
            vec![],
            vec![8, 128],
            vec![8, 0, 16, 128],
            valid[2..].to_vec(),
            {
                let mut bytes = valid.clone();
                bytes.drain(2..4);
                bytes
            },
            {
                let mut bytes = valid.clone();
                bytes[1] = 2;
                bytes
            },
            {
                let mut bytes = valid.clone();
                bytes.truncate(bytes.len() - 2);
                bytes
            },
        ] {
            assert!(decode_wheel_reply(&bytes).is_err());
        }
        for suffix in [
            &[8, 0][..],
            &[16, 7],
            &[24, 0],
            &[40, 0],
            &[26, 0],
            &[24, 128],
            &[34],
        ] {
            let mut bytes = valid.clone();
            bytes.extend_from_slice(suffix);
            assert!(decode_wheel_reply(&bytes).is_err());
        }
        let mut overflow = vec![8, 0, 16, 0];
        push_varint_field(&mut overflow, 3, u64::from(u16::MAX) + 1);
        for _ in 1..WHEEL_SLOTS {
            push_varint_field(&mut overflow, 3, 0);
        }
        assert!(decode_wheel_reply(&overflow).is_err());
        Ok(())
    }

    #[test]
    fn wheel_reply_refuses_partial_malformed_and_duplicate_vessels() -> Result<(), Error> {
        let legacy = encode_wheel_reply(&WheelAllocationReply {
            refused: false,
            revision: 1,
            points: [0; WHEEL_SLOTS],
            vessels: None,
        })?;
        for count in [1, 2, 3, 5] {
            let mut bytes = legacy.clone();
            for _ in 0..count {
                crate::charm_wire::push_message_field(&mut bytes, 4, &[]);
            }
            assert!(decode_wheel_reply(&bytes).is_err());
        }
        for length in [1, 15, 17] {
            let mut bytes = legacy.clone();
            crate::charm_wire::push_message_field(&mut bytes, 4, &vec![1; length]);
            for _ in 1..WHEEL_VESSELS {
                crate::charm_wire::push_message_field(&mut bytes, 4, &[]);
            }
            assert!(decode_wheel_reply(&bytes).is_err());
        }
        let mut bytes = legacy;
        for id in [&[1_u8; 16][..], &[], &[1_u8; 16], &[]] {
            crate::charm_wire::push_message_field(&mut bytes, 4, id);
        }
        assert!(decode_wheel_reply(&bytes).is_err());
        Ok(())
    }
    #[test]
    fn allocation_and_reset_round_trip_and_malformed_shapes_refuse() -> Result<(), Error> {
        let mut occurrence = [7; 16];
        occurrence[6] = 0x70;
        occurrence[8] = 0x80;
        let mut points = [0; WHEEL_SLOTS];
        points[15] = 50;
        let request = WheelAllocationRequest {
            action: 1,
            occurrence,
            expected_revision: 3,
            points,
            vessels: None,
        };
        assert_eq!(
            decode_wheel_request(&encode_wheel_request(&request)?)?,
            request
        );
        let reset = WheelAllocationRequest {
            action: 2,
            occurrence,
            expected_revision: 4,
            points: [0; WHEEL_SLOTS],
            vessels: None,
        };
        assert_eq!(decode_wheel_request(&encode_wheel_request(&reset)?)?, reset);
        let mut invalid = reset;
        invalid.points[0] = 1;
        assert!(encode_wheel_request(&invalid).is_err());
        assert!(decode_wheel_request(&[8, 1]).is_err());
        assert!(decode_wheel_request(&[8, 0, 24, 1]).is_err());
        assert!(decode_wheel_request(&[8, 0, 32, 0]).is_err());
        Ok(())
    }

    #[test]
    fn wheel_vessel_extension_round_trips_without_changing_legacy_payloads() -> Result<(), Error> {
        let request = WheelAllocationRequest {
            action: 1,
            occurrence: OCCURRENCE,
            expected_revision: 3,
            points: [0; WHEEL_SLOTS],
            vessels: None,
        };
        let legacy = encode_wheel_request(&request)?;
        assert_eq!(
            encode_wheel_request(&decode_wheel_request(&legacy)?)?,
            legacy
        );
        let mut extended = legacy;
        // Gem IDs are opaque 128-bit identities, including SHA-derived initial gems.
        for gem in [&[1_u8; 16][..], &[][..], &[2_u8; 16][..], &[][..]] {
            crate::charm_wire::push_message_field(&mut extended, 5, gem);
        }
        let decoded = decode_wheel_request(&extended)?;
        assert_eq!(encode_wheel_request(&decoded)?, extended);
        assert_eq!(
            decoded.vessels,
            Some([Some([1; 16]), None, Some([2; 16]), None])
        );
        let clear = WheelAllocationRequest {
            vessels: Some([None; WHEEL_VESSELS]),
            ..request
        };
        assert_eq!(decode_wheel_request(&encode_wheel_request(&clear)?)?, clear);
        Ok(())
    }

    #[test]
    fn wheel_vessels_reject_partial_duplicate_malformed_and_query_fields() -> Result<(), Error> {
        let request = WheelAllocationRequest {
            action: 1,
            occurrence: OCCURRENCE,
            expected_revision: 3,
            points: [0; WHEEL_SLOTS],
            vessels: None,
        };
        let legacy = encode_wheel_request(&request)?;
        for count in [1, 2, 3, 5] {
            let mut malformed = legacy.clone();
            for _ in 0..count {
                crate::charm_wire::push_message_field(&mut malformed, 5, &[]);
            }
            assert!(decode_wheel_request(&malformed).is_err());
        }
        for length in [1, 15, 17] {
            let mut malformed = legacy.clone();
            crate::charm_wire::push_message_field(&mut malformed, 5, &vec![1; length]);
            for _ in 1..WHEEL_VESSELS {
                crate::charm_wire::push_message_field(&mut malformed, 5, &[]);
            }
            assert!(decode_wheel_request(&malformed).is_err());
        }
        let duplicate = WheelAllocationRequest {
            vessels: Some([Some([1; 16]), None, Some([1; 16]), None]),
            ..request
        };
        assert!(encode_wheel_request(&duplicate).is_err());
        let mut duplicate = legacy;
        for value in [&[1_u8; 16][..], &[][..], &[1_u8; 16][..], &[][..]] {
            crate::charm_wire::push_message_field(&mut duplicate, 5, value);
        }
        assert!(decode_wheel_request(&duplicate).is_err());
        let mut query = vec![8, 0];
        for _ in 0..WHEEL_VESSELS {
            crate::charm_wire::push_message_field(&mut query, 5, &[]);
        }
        assert!(decode_wheel_request(&query).is_err());
        Ok(())
    }

    #[test]
    fn vessel_extension_stays_within_the_existing_wire_limits() -> Result<(), Error> {
        let vessels = Some([Some([1; 16]), Some([2; 16]), Some([3; 16]), Some([4; 16])]);
        let request = WheelAllocationRequest {
            action: 1,
            occurrence: OCCURRENCE,
            expected_revision: u64::MAX,
            points: [u16::MAX; WHEEL_SLOTS],
            vessels,
        };
        let bytes = encode_wheel_request(&request)?;
        assert!(bytes.len() <= MAX_REQUEST_BYTES);
        assert_eq!(decode_wheel_request(&bytes)?, request);
        let reply = WheelAllocationReply {
            refused: false,
            revision: u64::MAX,
            points: request.points,
            vessels,
        };
        assert!(encode_wheel_reply(&reply)?.len() <= MAX_REPLY_BYTES);
        let legacy = encode_wheel_reply(&WheelAllocationReply {
            vessels: None,
            ..reply
        })?;
        let mut expected = Vec::new();
        push_varint_field(&mut expected, 1, 0);
        push_varint_field(&mut expected, 2, u64::MAX);
        for _ in 0..WHEEL_SLOTS {
            push_varint_field(&mut expected, 3, u64::from(u16::MAX));
        }
        assert_eq!(legacy, expected);
        Ok(())
    }
}
