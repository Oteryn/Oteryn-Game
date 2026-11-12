use crate::chat::ChatPanel;
use oteryn_client_runtime::chat_guild_room::GuildRoomState;
use std::fmt::Write;
#[must_use]
pub fn panel_text(panel: &ChatPanel) -> String {
    let mut text = String::new();
    if let GuildRoomState::Member { motd, .. } = &panel.guild {
        let _ = lwriteln!(text, "Guild — {motd}", motd = motd);
    }
    text
}
