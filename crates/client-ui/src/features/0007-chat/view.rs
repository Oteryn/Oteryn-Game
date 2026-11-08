use super::ChatPanel;
use oteryn_client_runtime::chat::ChatLine;
use std::fmt::Write;
#[must_use]
pub fn panel_text(panel: &ChatPanel) -> String {
    let mut text = format!(
        "Chat — {} | Input {:?}\n",
        panel.console.tab.label(),
        panel.console.mode
    );
    text.push_str(&crate::chat_guild_room::panel_text(panel));
    for line in &panel.lines {
        if !panel.console.accepts(line) {
            continue;
        }
        if let ChatLine::Guild(line) = line
            && !matches!(&panel.guild, oteryn_client_runtime::chat_guild_room::GuildRoomState::Member { guild_id, .. } if *guild_id == line.guild_id)
        {
            continue;
        }
        let _ = writeln!(text, "{}", super::console::line_text(line));
    }
    if let Some(feedback) = &panel.feedback {
        let _ = writeln!(text, "{feedback}");
    }
    text
}
