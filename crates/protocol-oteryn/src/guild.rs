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
    /// The member's current Channel of this World (SOCIAL_CHANNEL_V1, capability 513);
    /// None when the member is offline or the capability was not selected.
    pub channel: Option<[u8; 16]>,
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

pub const CAPABILITY_GUILD_TEXT_V1: u32 = 638;
pub const COMMAND_TYPE_GUILD_TEXT_INTENT: u32 = 6411;
pub const MAX_GUILD_TEXT_REQUEST_BYTES: usize = 26 + 19 * 129 + 21;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuildTextIntent {
    pub guild: [u8; 16],
    pub expected_revision: u64,
    pub edit: GuildTextEdit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GuildTextEdit {
    Message(String),
    Title {
        character: [u8; 16],
        title: String,
    },
    /// The ordered layout keeps the native request stable across retries.
    RankName {
        level: u8,
        names: Vec<String>,
    },
}

pub fn encode_guild_text_intent(intent: &GuildTextIntent) -> Result<Vec<u8>, GuildWireError> {
    use GuildTextEdit::{Message, RankName, Title};
    let tag = match &intent.edit {
        Message(_) => 1,
        Title { .. } => 2,
        RankName { .. } => 3,
    };
    let mut bytes = vec![tag];
    bytes.extend_from_slice(&intent.guild);
    bytes.extend_from_slice(&intent.expected_revision.to_be_bytes());
    match &intent.edit {
        Message(message) => {
            guild_edit_text(message.as_bytes(), 255)?;
            bytes.extend_from_slice(message.as_bytes());
        }
        Title { character, title } => {
            guild_edit_text(title.as_bytes(), 15)?;
            bytes.extend_from_slice(character);
            bytes.extend_from_slice(title.as_bytes());
        }
        RankName { level, names } => {
            if names.len() > 20 {
                return Err(GuildWireError::LimitExceeded);
            }
            bytes.push(*level);
            for name in names {
                guild_edit_text(name.as_bytes(), 128)?;
                bytes.push(name.len() as u8);
                bytes.extend_from_slice(name.as_bytes());
            }
        }
    }
    decode_guild_text_intent(&bytes)?;
    Ok(bytes)
}

pub fn decode_guild_text_intent(bytes: &[u8]) -> Result<GuildTextIntent, GuildWireError> {
    use GuildWireError::{LimitExceeded, Malformed};
    if bytes.len() > MAX_GUILD_TEXT_REQUEST_BYTES {
        return Err(LimitExceeded);
    }
    let (&tag, bytes) = bytes.split_first().ok_or(Malformed)?;
    let (guild, bytes) = bytes.split_first_chunk::<16>().ok_or(Malformed)?;
    let guild = crate::decode_uuid_v7(guild).map_err(|_| Malformed)?;
    let (revision, body) = bytes.split_first_chunk::<8>().ok_or(Malformed)?;
    let expected_revision = u64::from_be_bytes(*revision);
    if expected_revision == 0 {
        return Err(Malformed);
    }
    let edit = match tag {
        1 => {
            let message = guild_edit_text(body, 255)?;
            if message.trim().is_empty() || message.chars().any(char::is_control) {
                return Err(Malformed);
            }
            GuildTextEdit::Message(message.to_owned())
        }
        2 => {
            let (target, title) = body.split_first_chunk::<16>().ok_or(Malformed)?;
            GuildTextEdit::Title {
                character: crate::decode_uuid_v7(target).map_err(|_| Malformed)?,
                title: guild_edit_text(title, 15)?.to_owned(),
            }
        }
        3 => {
            let (&level, mut names_bytes) = body.split_first().ok_or(Malformed)?;
            if !(1..=3).contains(&level) {
                return Err(Malformed);
            }
            let mut names = Vec::new();
            while let Some((&size, rest)) = names_bytes.split_first() {
                if size == 0 {
                    return Err(Malformed);
                }
                if names.len() == 20 {
                    return Err(LimitExceeded);
                }
                let (name, rest) = rest.split_at_checked(usize::from(size)).ok_or(Malformed)?;
                let max = if names.len() + 1 == usize::from(level) {
                    20
                } else {
                    128
                };
                names.push(guild_edit_text(name, max)?.to_owned());
                names_bytes = rest;
            }
            if names.len() < 3 {
                return Err(Malformed);
            }
            GuildTextEdit::RankName { level, names }
        }
        _ => return Err(Malformed),
    };
    Ok(GuildTextIntent {
        guild,
        expected_revision,
        edit,
    })
}

fn guild_edit_text(bytes: &[u8], max: usize) -> Result<&str, GuildWireError> {
    if bytes.len() > max {
        return Err(GuildWireError::LimitExceeded);
    }
    let text = std::str::from_utf8(bytes).map_err(|_| GuildWireError::Malformed)?;
    if text.contains('\0') {
        return Err(GuildWireError::Malformed);
    }
    Ok(text)
}

mod codec;
pub use codec::{
    decode_guild_intent, decode_guild_intent_result, decode_guild_query, decode_guild_query_result,
    encode_guild_intent, encode_guild_intent_result, encode_guild_query, encode_guild_query_result,
};
#[cfg(test)]
#[path = "guild_tests.rs"]
mod tests;

#[cfg(test)]
mod guild_text_tests {
    use super::*;

    #[test]
    fn guild_text_intent_round_trips_and_checks_limits() -> Result<(), GuildWireError> {
        let mut id = [1; 16];
        id[6] = 0x70;
        id[8] = 0x80;
        for edit in [
            GuildTextEdit::Message("a".repeat(255)),
            GuildTextEdit::Title {
                character: id,
                title: String::new(),
            },
            GuildTextEdit::RankName {
                level: 3,
                names: vec!["Leader".into(), "Vice Leader".into(), "a".repeat(20)],
            },
        ] {
            let intent = GuildTextIntent {
                guild: id,
                expected_revision: u64::MAX,
                edit,
            };
            let encoded = encode_guild_text_intent(&intent)?;
            assert_eq!(decode_guild_text_intent(&encoded)?, intent);
            let mut invalid = encoded;
            invalid[17..25].fill(0);
            assert_eq!(
                decode_guild_text_intent(&invalid),
                Err(GuildWireError::Malformed)
            );
        }
        for edit in [
            GuildTextEdit::Message("a".repeat(256)),
            GuildTextEdit::Title {
                character: id,
                title: "é".repeat(8),
            },
            GuildTextEdit::RankName {
                level: 3,
                names: vec!["Leader".into(), "Vice Leader".into(), "a".repeat(21)],
            },
        ] {
            assert_eq!(
                encode_guild_text_intent(&GuildTextIntent {
                    guild: id,
                    expected_revision: 1,
                    edit
                }),
                Err(GuildWireError::LimitExceeded)
            );
        }
        Ok(())
    }
}
