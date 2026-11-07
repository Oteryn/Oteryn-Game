//! Leader-owned Team Finder listing commands (S14a).
//!
//! Fields follow Crystal 201189f8 `ProtocolGame::createLeaderTeamFinder`
//! (src/server/network/protocol/protocolgame.cpp:2979-3010): minimum and maximum level
//! (u16), vocation mask (u8), team and free slots (u16), the party flag (u8 == 1), the
//! client timestamp (u32), the team type (u8) and its target: boss (1, u16), hunt (2, two
//! u16) or quest (3, u16).
use crate::charm_wire::{CyclopediaWireError as Error, push_varint_field, read_varint, set_once};

pub const CAPABILITY_TEAM_FINDER_LEADER_V1: u32 = 200;
pub const COMMAND_TYPE_TEAM_FINDER_LEADER_UPSERT: u32 = 390;
pub const COMMAND_TYPE_TEAM_FINDER_LEADER_REMOVE: u32 = 391;
pub const MAX_REQUEST_BYTES: usize = 64;
const FIELDS: usize = 10;

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
    pub party_members_join: bool,
    pub timestamp: u32,
    pub target: TeamTarget,
}

/// Crystal stores whatever the client sends; these are the listing's own invariants.
fn valid(value: &LeaderListing) -> bool {
    value.min_level != 0
        && value.min_level <= value.max_level
        && value.vocation_mask != 0
        && value.slots != 0
        && value.free_slots <= value.slots
}

pub fn encode_upsert(value: LeaderListing) -> Result<Vec<u8>, Error> {
    if !valid(&value) {
        return Err(Error::Malformed);
    }
    let (kind, first, area) = match value.target {
        TeamTarget::Boss(id) => (1, id, None),
        TeamTarget::Hunt { kind, area } => (2, kind, Some(area)),
        TeamTarget::Quest(id) => (3, id, None),
    };
    let mut out = Vec::new();
    for (field, number) in [
        (1, u64::from(value.min_level)),
        (2, u64::from(value.max_level)),
        (3, u64::from(value.vocation_mask)),
        (4, u64::from(value.slots)),
        (5, u64::from(value.free_slots)),
        (6, u64::from(value.party_members_join)),
        (7, u64::from(value.timestamp)),
        (8, kind),
        (9, u64::from(first)),
    ] {
        push_varint_field(&mut out, field, number);
    }
    if let Some(area) = area {
        push_varint_field(&mut out, 10, u64::from(area));
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
    let mut fields: [Option<u64>; FIELDS] = [None; FIELDS];
    let mut at = 0;
    while at < bytes.len() {
        let key = read_varint(bytes, &mut at)?;
        let index = usize::try_from(key / 8).map_err(|_| Error::Malformed)?;
        if key & 7 != 0 || !(1..=FIELDS).contains(&index) {
            return Err(Error::Malformed);
        }
        set_once(&mut fields[index - 1], read_varint(bytes, &mut at)?)?;
    }
    let number = |i: usize| fields[i - 1].ok_or(Error::Malformed);
    let u16n = |i| u16::try_from(number(i)?).map_err(|_| Error::LimitExceeded);
    let first = u16n(9)?;
    let target = match (number(8)?, fields[9]) {
        (1, None) => TeamTarget::Boss(first),
        (2, Some(_)) => TeamTarget::Hunt {
            kind: first,
            area: u16n(10)?,
        },
        (3, None) => TeamTarget::Quest(first),
        _ => return Err(Error::Malformed),
    };
    let value = LeaderListing {
        min_level: u16n(1)?,
        max_level: u16n(2)?,
        vocation_mask: u8::try_from(number(3)?).map_err(|_| Error::LimitExceeded)?,
        slots: u16n(4)?,
        free_slots: u16n(5)?,
        party_members_join: match number(6)? {
            0 => false,
            1 => true,
            _ => return Err(Error::Malformed),
        },
        timestamp: u32::try_from(number(7)?).map_err(|_| Error::LimitExceeded)?,
        target,
    };
    if valid(&value) {
        Ok(value)
    } else {
        Err(Error::Malformed)
    }
}

pub fn decode_remove(bytes: &[u8]) -> Result<(), Error> {
    if bytes.is_empty() {
        Ok(())
    } else {
        Err(Error::Malformed)
    }
}

#[cfg(test)]
#[path = "tests/team_finder_tests.rs"]
mod tests;
