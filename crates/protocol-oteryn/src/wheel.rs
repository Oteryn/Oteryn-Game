//! Wheel of Destiny allocation wire contract.
use crate::charm_wire::{CyclopediaWireError as Error, push_varint_field, read_varint, set_once};

pub const CAPABILITY_WHEEL_ALLOCATION_V1: u32 = 148;
pub const COMMAND_TYPE_WHEEL_ALLOCATION: u32 = 161;
pub const WHEEL_SLOTS: usize = 36;
pub const MAX_REQUEST_BYTES: usize = 256;
pub const MAX_REPLY_BYTES: usize = 256;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WheelAllocationRequest {
    /// 0 reads, 1 replaces the allocation, 2 resets it at a temple.
    pub action: u8,
    pub occurrence: [u8; 16],
    pub expected_revision: u64,
    pub points: [u16; WHEEL_SLOTS],
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WheelAllocationReply {
    pub refused: bool,
    pub revision: u64,
    pub points: [u16; WHEEL_SLOTS],
}

pub fn encode_wheel_request(r: &WheelAllocationRequest) -> Result<Vec<u8>, Error> {
    if r.action > 2
        || (r.action == 0
            && (r.expected_revision != 0
                || r.occurrence != [0; 16]
                || r.points != [0; WHEEL_SLOTS]))
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
            _ => return Err(Error::Malformed),
        }
    }
    let action = action.ok_or(Error::Malformed)?;
    let r = if action == 0 {
        if occurrence.is_some() || revision.is_some() || !points.is_empty() {
            return Err(Error::Malformed);
        }
        WheelAllocationRequest {
            action,
            occurrence: [0; 16],
            expected_revision: 0,
            points: [0; WHEEL_SLOTS],
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
    if out.len() > MAX_REPLY_BYTES {
        Err(Error::LimitExceeded)
    } else {
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
}
