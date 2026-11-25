use crate::chat::ChatPanel;
use oteryn_client_runtime::chat_guild_room::GuildRoomState;
use std::fmt::Write;

pub fn show_motd_notice(panel: &mut ChatPanel) {
    panel.guild_motd_notice = match &panel.guild {
        GuildRoomState::Member { motd, .. }
            if super::available(panel)
                && !motd.is_empty()
                && panel
                    .console
                    .channels
                    .contains(&crate::chat::ChannelTab::Guild) =>
        {
            Some(motd.clone())
        }
        _ => None,
    };
}

#[must_use]
pub fn motd_notice_text(panel: &ChatPanel) -> Option<String> {
    if !super::available(panel) {
        return None;
    }
    panel
        .guild_motd_notice
        .as_ref()
        .map(|motd| lfmt!("Message of the Day: {motd}", motd = motd))
}

#[must_use]
pub fn panel_text(panel: &ChatPanel) -> String {
    let mut text = String::new();
    if let GuildRoomState::Member { motd, .. } = &panel.guild {
        let _ = lwriteln!(text, "Guild — {motd}", motd = motd);
    }
    text
}
