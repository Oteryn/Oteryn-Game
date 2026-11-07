use oteryn_dev_client::{GuildChatLine, GuildRoomState};

pub(super) fn render_line(line: &GuildChatLine) -> String {
    let rank = match line.rank {
        1 => " Leader",
        2 => " Vice Leader",
        _ => "",
    };
    format!("[Guild{rank}] {}: {}", line.speaker_name, line.text)
}

pub(super) fn render_room(out: &mut String, state: &GuildRoomState) {
    match state {
        GuildRoomState::Absent => {}
        GuildRoomState::Unavailable => out.push_str("guild unavailable\n"),
        GuildRoomState::Member { motd, .. } => {
            out.push_str("guild: ");
            out.push_str(motd);
            out.push('\n');
        }
    }
}

#[cfg(test)]
#[path = "guild_chat_tests.rs"]
mod tests;
