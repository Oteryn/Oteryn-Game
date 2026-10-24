//! Basic PARTY-1 server wire. Durable identities never cross this boundary.
pub use crate::charm_wire::CyclopediaWireError as PartyWireError;
use crate::charm_wire::{push_message_field, push_varint_field, read_bytes, read_varint, set_once};
use crate::world_spatial_entities::EntityRef;
type Result<T> = std::result::Result<T, PartyWireError>;
pub const CAPABILITY_PARTY_V1: u32 = 42;
pub const COMMAND_TYPE_PARTY_INTENT: u32 = 45;
pub const COMMAND_TYPE_PARTY_QUERY: u32 = 46;
pub const MAX_PARTY_INTENT_BYTES: usize = 48;
pub const MAX_PARTY_RESULT_BYTES: usize = 8192;
pub const MAX_PARTY_MEMBERS: usize = 50;
pub const MAX_PARTY_INVITATIONS: usize = 20;
pub const MAX_PARTY_NAME_BYTES: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartyIntent {
    Invite(EntityRef),
    Accept(u64),
    Leave,
    Kick(u64),
    PassLeadership(u64),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartyDisposition {
    Ok = 1,
    NotLeader = 2,
    AlreadyInParty = 3,
    NotInvited = 4,
    NotVisible = 5,
    LogoutBlocked = 6,
    Unavailable = 7,
    Rejected = 8,
    NotMember = 9,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartyEntry {
    pub handle: u64,
    pub name: String,
    pub leader: bool,
    /// Invitation expiry on the database clock, seconds since Unix epoch; zero for members.
    pub expires_at: u64,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartyResult {
    pub disposition: PartyDisposition,
    pub shared_xp_enabled: bool,
    pub members: Vec<PartyEntry>,
    pub invitations: Vec<PartyEntry>,
}
impl PartyResult {
    pub const fn refused(disposition: PartyDisposition) -> Self {
        Self {
            disposition,
            shared_xp_enabled: false,
            members: Vec::new(),
            invitations: Vec::new(),
        }
    }
}
fn bad<T>() -> Result<T> {
    Err(PartyWireError::Malformed)
}
fn bounded(input: &[u8], limit: usize) -> Result<()> {
    if input.len() > limit {
        return Err(PartyWireError::LimitExceeded);
    }
    Ok(())
}
/// Canonical request encoding is also the server's receipt binding input.
pub fn encode_party_intent(intent: PartyIntent) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    let (kind, handle) = match intent {
        PartyIntent::Invite(entity) => {
            if entity.generation == 0 {
                return bad();
            }
            push_message_field(&mut out, 2, &entity.identity);
            push_varint_field(&mut out, 3, entity.generation);
            (1, None)
        }
        PartyIntent::Accept(h) => (2, Some(h)),
        PartyIntent::Leave => (3, None),
        PartyIntent::Kick(h) => (4, Some(h)),
        PartyIntent::PassLeadership(h) => (5, Some(h)),
    };
    push_varint_field(&mut out, 1, kind);
    if let Some(handle) = handle {
        if handle == 0 {
            return bad();
        }
        push_varint_field(&mut out, 4, handle);
    }
    Ok(out)
}
pub fn decode_party_intent(input: &[u8]) -> Result<PartyIntent> {
    bounded(input, MAX_PARTY_INTENT_BYTES)?;
    let (mut kind, mut identity, mut generation, mut handle) = (None, None, None, None);
    let mut cursor = 0;
    while cursor < input.len() {
        match read_varint(input, &mut cursor)? {
            8 => set_once(&mut kind, read_varint(input, &mut cursor)?)?,
            18 => set_once(
                &mut identity,
                read_bytes(input, &mut cursor)?
                    .try_into()
                    .map_err(|_| PartyWireError::Malformed)?,
            )?,
            24 => set_once(&mut generation, read_varint(input, &mut cursor)?)?,
            32 => set_once(&mut handle, read_varint(input, &mut cursor)?)?,
            _ => return bad(),
        }
    }
    match (kind, identity, generation, handle) {
        (Some(1), Some(identity), Some(generation), None) if generation != 0 => {
            Ok(PartyIntent::Invite(EntityRef {
                identity,
                generation,
            }))
        }
        (Some(2), None, None, Some(h)) if h != 0 => Ok(PartyIntent::Accept(h)),
        (Some(3), None, None, None) => Ok(PartyIntent::Leave),
        (Some(4), None, None, Some(h)) if h != 0 => Ok(PartyIntent::Kick(h)),
        (Some(5), None, None, Some(h)) if h != 0 => Ok(PartyIntent::PassLeadership(h)),
        _ => bad(),
    }
}
pub fn encode_party_result(result: &PartyResult) -> Result<Vec<u8>> {
    if result.members.len() > MAX_PARTY_MEMBERS || result.invitations.len() > MAX_PARTY_INVITATIONS
    {
        return Err(PartyWireError::LimitExceeded);
    }
    let mut out = Vec::new();
    push_varint_field(&mut out, 1, result.disposition as u64);
    push_varint_field(&mut out, 2, u64::from(result.shared_xp_enabled));
    let mut handles = std::collections::BTreeSet::new();
    for (field, entries) in [(3, &result.members), (4, &result.invitations)] {
        for entry in entries {
            if entry.handle == 0
                || !handles.insert(entry.handle)
                || entry.name.is_empty()
                || entry.name.len() > MAX_PARTY_NAME_BYTES
                || (field == 3 && entry.expires_at != 0)
                || (field == 4 && (entry.expires_at == 0 || entry.leader))
            {
                return bad();
            }
            let mut inner = Vec::new();
            push_varint_field(&mut inner, 1, entry.handle);
            push_message_field(&mut inner, 2, entry.name.as_bytes());
            push_varint_field(&mut inner, 3, u64::from(entry.leader));
            push_varint_field(&mut inner, 4, entry.expires_at);
            push_message_field(&mut out, field, &inner);
        }
    }
    bounded(&out, MAX_PARTY_RESULT_BYTES)?;
    Ok(out)
}
/// Strict result decoder, shared by wire consumers and the dispatch qualification.
pub fn decode_party_result(input: &[u8]) -> Result<PartyResult> {
    bounded(input, MAX_PARTY_RESULT_BYTES)?;
    let (mut disposition, mut shared) = (None, None);
    let (mut members, mut invitations) = (Vec::new(), Vec::new());
    let mut cursor = 0;
    while cursor < input.len() {
        match read_varint(input, &mut cursor)? {
            8 => set_once(
                &mut disposition,
                match read_varint(input, &mut cursor)? {
                    1 => PartyDisposition::Ok,
                    2 => PartyDisposition::NotLeader,
                    3 => PartyDisposition::AlreadyInParty,
                    4 => PartyDisposition::NotInvited,
                    5 => PartyDisposition::NotVisible,
                    6 => PartyDisposition::LogoutBlocked,
                    7 => PartyDisposition::Unavailable,
                    8 => PartyDisposition::Rejected,
                    9 => PartyDisposition::NotMember,
                    _ => return bad(),
                },
            )?,
            16 => set_once(&mut shared, boolean(read_varint(input, &mut cursor)?)?)?,
            tag @ (26 | 34) => {
                let entries = if tag == 26 {
                    &mut members
                } else {
                    &mut invitations
                };
                let limit = if tag == 26 {
                    MAX_PARTY_MEMBERS
                } else {
                    MAX_PARTY_INVITATIONS
                };
                if entries.len() >= limit {
                    return Err(PartyWireError::LimitExceeded);
                }
                entries.push(decode_entry(read_bytes(input, &mut cursor)?)?);
            }
            _ => return bad(),
        }
    }
    let result = PartyResult {
        disposition: disposition.ok_or(PartyWireError::Malformed)?,
        shared_xp_enabled: shared.ok_or(PartyWireError::Malformed)?,
        members,
        invitations,
    };
    encode_party_result(&result)?;
    Ok(result)
}
fn boolean(n: u64) -> Result<bool> {
    match n {
        0 => Ok(false),
        1 => Ok(true),
        _ => bad(),
    }
}
fn decode_entry(input: &[u8]) -> Result<PartyEntry> {
    bounded(input, 104)?;
    let (mut handle, mut name, mut leader, mut expires) = (None, None, None, None);
    let mut cursor = 0;
    while cursor < input.len() {
        match read_varint(input, &mut cursor)? {
            8 => set_once(&mut handle, read_varint(input, &mut cursor)?)?,
            18 => set_once(
                &mut name,
                std::str::from_utf8(read_bytes(input, &mut cursor)?)
                    .map_err(|_| PartyWireError::Malformed)?
                    .to_owned(),
            )?,
            24 => set_once(&mut leader, boolean(read_varint(input, &mut cursor)?)?)?,
            32 => set_once(&mut expires, read_varint(input, &mut cursor)?)?,
            _ => return bad(),
        }
    }
    Ok(PartyEntry {
        handle: handle.ok_or(PartyWireError::Malformed)?,
        name: name.ok_or(PartyWireError::Malformed)?,
        leader: leader.ok_or(PartyWireError::Malformed)?,
        expires_at: expires.ok_or(PartyWireError::Malformed)?,
    })
}
#[cfg(test)]
#[path = "party_tests.rs"]
mod tests;
