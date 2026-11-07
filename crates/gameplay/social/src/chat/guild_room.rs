//! Guild destination on the existing authenticated World relay; never a public room.
use super::relay::{Cursor, RelayDestination, RelayError};
use crate::domain::guild::{GuildId, GuildRank};
use oteryn_protocol_oteryn::chat::{ChatLine, guild_room::GuildChatLine};

pub(super) const KIND_GUILD: u8 = 3;

pub(super) fn encode_destination(out: &mut Vec<u8>, guild: GuildId, rank: GuildRank) {
    out.extend_from_slice(guild.as_bytes());
    out.push(rank.level());
}

pub(super) fn decode_destination(cursor: &mut Cursor<'_>) -> Result<RelayDestination, RelayError> {
    let guild_id = GuildId::from_bytes(cursor.array()?).map_err(|_| RelayError::BadPayload)?;
    let rank = GuildRank::new(cursor.take(1)?[0]).map_err(|_| RelayError::BadPayload)?;
    Ok(RelayDestination::Guild { guild_id, rank })
}

pub fn guild_line(guild: GuildId, rank: GuildRank, name: &str, text: &super::ChatText) -> ChatLine {
    ChatLine::Guild(GuildChatLine {
        guild_id: *guild.as_bytes(),
        rank: rank.level(),
        speaker_name: name.to_owned(),
        text: text.as_str().to_owned(),
    })
}

#[cfg(test)]
#[path = "guild_room_tests.rs"]
mod tests;
