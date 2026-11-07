//! Basic Guild commands and private own-Guild queries (GUILD-WIRE-1).
//! Selectors are untrusted UUIDv7 data, never identity or membership authority.
pub use crate::charm_wire::CyclopediaWireError as GuildWireError;
use crate::charm_wire::{
    WireResult, push_message_field, push_varint_field, read_bytes, read_varint, set_once,
};

pub const CAPABILITY_GUILD_V1: u32 = 40;
pub const COMMAND_TYPE_GUILD_INTENT: u32 = 40;
pub const COMMAND_TYPE_GUILD_QUERY: u32 = 41;
pub const MAX_GUILD_REQUEST_BYTES: usize = 128;
pub const MAX_GUILD_RESULT_BYTES: usize = 65_536;
pub const MAX_GUILD_PAGE_ENTRIES: usize = 100;
pub const MAX_GUILD_ENTRY_BYTES: usize = 512;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GuildIntent {
    Found {
        name: String,
    },
    Invite {
        guild: [u8; 16],
        name: String,
    },
    Join {
        guild: [u8; 16],
    },
    Leave {
        guild: [u8; 16],
    },
    Kick {
        guild: [u8; 16],
        character: [u8; 16],
    },
    SetRank {
        guild: [u8; 16],
        character: [u8; 16],
        level: u8,
        expected_revision: u64,
    },
    RevokeInvite {
        guild: [u8; 16],
        character: [u8; 16],
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuildRosterCursor {
    pub guild: [u8; 16],
    pub level: u8,
    pub character: [u8; 16],
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuildQuery {
    Own,
    Roster {
        guild: [u8; 16],
        after: Option<GuildRosterCursor>,
    },
    Invitations,
}

/// Codes 1..=13 correspond to native GuildResult::code()+1; 14 is NAME_INVALID.
/// Foundation REJECTED has an empty payload and never masquerades as a Guild refusal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuildIntentResult {
    pub code: u8,
    pub guild: [u8; 16],
    pub revision: Option<u64>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuildSummary {
    pub guild: [u8; 16],
    pub name: String,
    pub state: u8,
    pub revision: u64,
    pub own_rank: u8,
    pub title: String,
    pub message: String,
    pub ranks: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuildMember {
    pub character: [u8; 16],
    pub name: String,
    pub level: u8,
    pub title: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuildInvitation {
    pub guild: [u8; 16],
    pub name: String,
    pub expires_at_unix_ms: u64,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GuildQueryResult {
    Own(Option<GuildSummary>),
    Roster {
        guild: [u8; 16],
        revision: u64,
        members: Vec<GuildMember>,
        next: Option<GuildRosterCursor>,
    },
    Invitations(Vec<GuildInvitation>),
}

mod codec;
pub use codec::{
    decode_guild_intent, decode_guild_intent_result, decode_guild_query, decode_guild_query_result,
    encode_guild_intent, encode_guild_intent_result, encode_guild_query, encode_guild_query_result,
};
#[cfg(test)]
#[path = "guild_tests.rs"]
mod tests;
