use super::ChatPanel;
use oteryn_client_runtime::chat_private_channel::ChatLine;
use std::fmt::Write;

#[must_use]
pub fn panel_text(panel: &ChatPanel) -> String {
    let mut text = String::from(
        "Private channels — Create/Open own, or select an invitation and Join\nInvites and exclusions apply to your own channel.\n",
    );
    if let Some(list) = &panel.list {
        if let Some(id) = list.own_channel {
            let _ = writeln!(text, "Own channel: {id}");
        }
        for invite in &list.invited {
            let _ = writeln!(
                text,
                "Invitation: {} ({})",
                invite.owner_name, invite.channel_id
            );
        }
    }
    if let Some(id) = panel.channel {
        let _ = writeln!(text, "Selected channel: {id}");
        let start = panel.history_start.get(&id).copied().unwrap_or(0);
        for line in panel.lines.iter().skip(start) {
            if !panel.communication.visible(line) {
                continue;
            }
            match line {
                ChatLine::PrivateChannel(line) if line.channel_id == id => {
                    let _ = writeln!(text, "{}: {}", line.speaker_name, line.text);
                }
                ChatLine::Dropped => text.push_str("[Some chat lines were dropped]\n"),
                _ => {}
            }
        }
    }
    if let Some(feedback) = &panel.feedback {
        let _ = writeln!(text, "{feedback}");
    }
    text
}

#[must_use]
pub fn leave_label(panel: &ChatPanel) -> &'static str {
    if panel.channel.is_some_and(|id| {
        panel
            .list
            .as_ref()
            .is_some_and(|list| list.own_channel == Some(id))
    }) {
        "Close own channel"
    } else {
        "Leave"
    }
}
