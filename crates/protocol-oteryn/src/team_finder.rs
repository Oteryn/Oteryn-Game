//! Leader-owned Team Finder listing commands.
use crate::charm_wire::{CyclopediaWireError as Error, read_varint, set_once};

pub const CAPABILITY_TEAM_FINDER_LEADER_V1: u32 = 200;
pub const COMMAND_TYPE_TEAM_FINDER_LEADER_UPSERT: u32 = 390;
pub const COMMAND_TYPE_TEAM_FINDER_LEADER_REMOVE: u32 = 391;
pub const MAX_REQUEST_BYTES: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TeamTarget {
    Boss(u16),
    Hunt { kind: u16, area: u16 },
    Quest(u16),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LeaderListing {
    pub min_level: u16,
    pub max_level: u16,
    pub vocation_mask: u8,
    pub slots: u16,
    pub free_slots: u16,
    pub timestamp: u32,
    pub target: TeamTarget,
}

pub fn encode_upsert(value: LeaderListing) -> Result<Vec<u8>, Error> {
    if value.min_level == 0
        || value.min_level > value.max_level
        || value.slots == 0
        || value.free_slots == 0
        || value.free_slots > value.slots
        || value.vocation_mask == 0
    {
        return Err(Error::Malformed);
    }
    let mut out = Vec::new();
    for (field, number) in [
        (1, u64::from(value.min_level)),
        (2, u64::from(value.max_level)),
        (3, u64::from(value.vocation_mask)),
        (4, u64::from(value.slots)),
        (5, u64::from(value.free_slots)),
        (6, u64::from(value.timestamp)),
    ] {
        crate::charm_wire::push_varint_field(&mut out, field, number);
    }
    match value.target {
        TeamTarget::Boss(id) => {
            crate::charm_wire::push_varint_field(&mut out, 7, 1);
            crate::charm_wire::push_varint_field(&mut out, 8, u64::from(id));
        }
        TeamTarget::Hunt { kind, area } => {
            crate::charm_wire::push_varint_field(&mut out, 7, 2);
            crate::charm_wire::push_varint_field(&mut out, 8, u64::from(kind));
            crate::charm_wire::push_varint_field(&mut out, 9, u64::from(area));
        }
        TeamTarget::Quest(id) => {
            crate::charm_wire::push_varint_field(&mut out, 7, 3);
            crate::charm_wire::push_varint_field(&mut out, 8, u64::from(id));
        }
    }
    if out.len() > MAX_REQUEST_BYTES {
        Err(Error::LimitExceeded)
    } else {
        Ok(out)
    }
}

pub fn decode_upsert(bytes: &[u8]) -> Result<LeaderListing, Error> {
    if bytes.len() > MAX_REQUEST_BYTES {
        return Err(Error::LimitExceeded);
    }
    let mut fields: [Option<u64>; 9] = [None; 9];
    let mut at = 0;
    while at < bytes.len() {
        let key = read_varint(bytes, &mut at)?;
        if key & 7 != 0 || !(8..=72).contains(&key) {
            return Err(Error::Malformed);
        }
        let index = usize::try_from(key / 8 - 1).map_err(|_| Error::Malformed)?;
        set_once(&mut fields[index], read_varint(bytes, &mut at)?)?;
    }
    let number = |i: usize| fields[i].ok_or(Error::Malformed);
    let u16n = |i| u16::try_from(number(i)?).map_err(|_| Error::LimitExceeded);
    let kind = number(6)?;
    let first = u16n(7)?;
    let target = match kind {
        1 => TeamTarget::Boss(first),
        2 => TeamTarget::Hunt {
            kind: first,
            area: u16n(8)?,
        },
        3 => TeamTarget::Quest(first),
        _ => return Err(Error::Malformed),
    };
    let value = LeaderListing {
        min_level: u16n(0)?,
        max_level: u16n(1)?,
        vocation_mask: u8::try_from(number(2)?).map_err(|_| Error::LimitExceeded)?,
        slots: u16n(3)?,
        free_slots: u16n(4)?,
        timestamp: u32::try_from(number(5)?).map_err(|_| Error::LimitExceeded)?,
        target,
    };
    encode_upsert(value)?;
    Ok(value)
}

pub fn decode_remove(bytes: &[u8]) -> Result<(), Error> {
    if bytes.is_empty() {
        Ok(())
    } else {
        Err(Error::Malformed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn leader_listing_round_trip_and_refusals() -> Result<(), Error> {
        let listing = LeaderListing {
            min_level: 8,
            max_level: 100,
            vocation_mask: 15,
            slots: 5,
            free_slots: 4,
            timestamp: 7,
            target: TeamTarget::Hunt { kind: 2, area: 9 },
        };
        assert_eq!(decode_upsert(&encode_upsert(listing)?)?, listing);
        assert!(
            encode_upsert(LeaderListing {
                min_level: 101,
                max_level: 100,
                ..listing
            })
            .is_err()
        );
        assert!(decode_remove(&[1]).is_err());
        Ok(())
    }
}
