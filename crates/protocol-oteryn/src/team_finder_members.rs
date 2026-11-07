//! Team Finder browse and member decisions (S14b/S14c).
//!
//! Crystal 201189f `protocolgame.cpp:2823-2970,3060-3157` supplies the fields and
//! status values. Targets and cursors are connection-scoped opaque handles;
//! CharacterId, AccountId, WorldId and session authority never cross this wire.
pub use crate::charm_wire::CyclopediaWireError as TeamFinderWireError;
use crate::charm_wire::{push_varint_field, read_varint, set_once};
use crate::team_finder::LeaderListing;

pub const CAPABILITY_TEAM_FINDER_MEMBERS_V1: u32 = 232;
pub const COMMAND_TYPE_TEAM_FINDER_QUERY: u32 = 444;
pub const COMMAND_TYPE_TEAM_FINDER_MEMBER_ACTION: u32 = 445;
pub const COMMAND_TYPE_TEAM_FINDER_LEADER_ACTION: u32 = 446;
// Existing Party/Guild transport budgets, not additional gameplay limits.
pub const MAX_REQUEST_BYTES: usize = crate::party::MAX_PARTY_INTENT_BYTES;
pub const MAX_RESULT_BYTES: usize = crate::guild::MAX_GUILD_RESULT_BYTES;
pub const MAX_PAGE_ENTRIES: usize = crate::guild::MAX_GUILD_PAGE_ENTRIES;
pub const MAX_NAME_BYTES: usize = crate::party::MAX_PARTY_NAME_BYTES;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MemberStatus {
    None = 0,
    Pending = 1,
    Invited = 2,
    Accepted = 3,
    Denied = 4,
}

impl TryFrom<u8> for MemberStatus {
    type Error = TeamFinderWireError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::None),
            1 => Ok(Self::Pending),
            2 => Ok(Self::Invited),
            3 => Ok(Self::Accepted),
            4 => Ok(Self::Denied),
            _ => Err(TeamFinderWireError::Malformed),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TeamFinderQuery {
    Browse { after: Option<u64> },
    Applicants { after: Option<u64> },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemberAction {
    Request { leader: u64 },
    Cancel { leader: u64 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeaderAction {
    Invite { member: u64 },
    Accept { member: u64 },
    Deny { member: u64 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrowseListing {
    pub leader_handle: u64,
    pub name: String,
    /// `free_slots` is the authoring baseline, not the effective free capacity.
    pub listing: LeaderListing,
    /// Global occupancy; effective free slots are `slots.saturating_sub(occupied_slots)`.
    pub occupied_slots: u16,
    pub status: MemberStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Applicant {
    pub handle: u64,
    pub name: String,
    pub level: u16,
    pub vocation: u8,
    pub status: MemberStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TeamFinderResult {
    Browse {
        listings: Vec<BrowseListing>,
        next: Option<u64>,
    },
    Applicants {
        /// Absent when the leader has no active advertisement.
        listing: Option<LeaderListing>,
        /// Counted over the whole advertisement, independently of this page.
        occupied_slots: u16,
        members: Vec<Applicant>,
        next: Option<u64>,
    },
}

fn encode_request(kind: u64, handle: Option<u64>) -> Result<Vec<u8>, TeamFinderWireError> {
    if handle == Some(0) {
        return Err(TeamFinderWireError::Malformed);
    }
    let mut out = Vec::new();
    push_varint_field(&mut out, 1, kind);
    if let Some(handle) = handle {
        push_varint_field(&mut out, 2, handle);
    }
    Ok(out)
}

fn decode_request(input: &[u8]) -> Result<(u64, Option<u64>), TeamFinderWireError> {
    if input.len() > MAX_REQUEST_BYTES {
        return Err(TeamFinderWireError::LimitExceeded);
    }
    let (mut kind, mut handle) = (None, None);
    let mut cursor = 0;
    while cursor < input.len() {
        match read_varint(input, &mut cursor)? {
            8 => set_once(&mut kind, read_varint(input, &mut cursor)?)?,
            16 => set_once(&mut handle, read_varint(input, &mut cursor)?)?,
            _ => return Err(TeamFinderWireError::Malformed),
        }
    }
    if handle == Some(0) {
        return Err(TeamFinderWireError::Malformed);
    }
    Ok((kind.ok_or(TeamFinderWireError::Malformed)?, handle))
}

pub fn encode_query(value: TeamFinderQuery) -> Result<Vec<u8>, TeamFinderWireError> {
    match value {
        TeamFinderQuery::Browse { after } => encode_request(1, after),
        TeamFinderQuery::Applicants { after } => encode_request(2, after),
    }
}

pub fn decode_query(input: &[u8]) -> Result<TeamFinderQuery, TeamFinderWireError> {
    match decode_request(input)? {
        (1, after) => Ok(TeamFinderQuery::Browse { after }),
        (2, after) => Ok(TeamFinderQuery::Applicants { after }),
        _ => Err(TeamFinderWireError::Malformed),
    }
}

pub fn encode_member_action(value: MemberAction) -> Result<Vec<u8>, TeamFinderWireError> {
    match value {
        MemberAction::Request { leader } => encode_request(1, Some(leader)),
        MemberAction::Cancel { leader } => encode_request(2, Some(leader)),
    }
}

pub fn decode_member_action(input: &[u8]) -> Result<MemberAction, TeamFinderWireError> {
    match decode_request(input)? {
        (1, Some(leader)) => Ok(MemberAction::Request { leader }),
        (2, Some(leader)) => Ok(MemberAction::Cancel { leader }),
        _ => Err(TeamFinderWireError::Malformed),
    }
}

pub fn encode_leader_action(value: LeaderAction) -> Result<Vec<u8>, TeamFinderWireError> {
    // Preserve the donor's decision status bytes, protocolgame.cpp:3094-3105.
    match value {
        LeaderAction::Invite { member } => encode_request(2, Some(member)),
        LeaderAction::Accept { member } => encode_request(3, Some(member)),
        LeaderAction::Deny { member } => encode_request(4, Some(member)),
    }
}

pub fn decode_leader_action(input: &[u8]) -> Result<LeaderAction, TeamFinderWireError> {
    match decode_request(input)? {
        (2, Some(member)) => Ok(LeaderAction::Invite { member }),
        (3, Some(member)) => Ok(LeaderAction::Accept { member }),
        (4, Some(member)) => Ok(LeaderAction::Deny { member }),
        _ => Err(TeamFinderWireError::Malformed),
    }
}

#[path = "team_finder_members/result.rs"]
mod result;
pub use result::{decode_result, encode_result};

#[cfg(test)]
#[path = "tests/team_finder_members_tests.rs"]
mod tests;
