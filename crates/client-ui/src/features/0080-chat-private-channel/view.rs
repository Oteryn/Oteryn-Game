use super::ChatPanel;
use oteryn_client_runtime::chat_private_channel::ChatLine;
use std::fmt::Write;

#[must_use]
pub fn panel_text(panel: &ChatPanel) -> String {
    let mut text = String::from(tr!(
        "Private channels — Create/Open own, or select an invitation and Join\nInvites and exclusions apply to your own channel.\n"
    ));
    if let Some(list) = &panel.list {
        if let Some(id) = list.own_channel {
            let _ = lwriteln!(text, "Own channel: {id}", id = id);
        }
        for invite in &list.invited {
            let _ = lwriteln!(
                text,
                "Invitation: {} ({})",
                invite.owner_name,
                invite.channel_id
            );
        }
    }
    if let Some(id) = panel.channel {
        let _ = lwriteln!(text, "Selected channel: {id}", id = id);
        let start = panel.history_start.get(&id).copied().unwrap_or(0).max(
            panel
                .lines
                .len()
                .saturating_sub(usize::from(panel.presentation.limit)),
        );
        for line in panel.lines.iter().skip(start) {
            if !panel.communication.visible(line) {
                continue;
            }
            match line {
                ChatLine::PrivateChannel(line) if line.channel_id == id => {
                    if !panel.presentation.visible[1] {
                        continue;
                    }
                    let _ = writeln!(text, "{}: {}", line.speaker_name, line.text);
                }
                ChatLine::Dropped if panel.presentation.dropped => {
                    text.push_str(tr!("[Some chat lines were dropped]\n"))
                }
                _ => {}
            }
        }
    }
    if let Some(feedback) = &panel.feedback
        && panel.presentation.feedback
    {
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
        tr!("Close own channel")
    } else {
        tr!("Leave")
    }
}
