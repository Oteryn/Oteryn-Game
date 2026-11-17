use super::state::GuildRoomState;
use crate::chat::ChatPanel;
use oteryn_client_runtime::chat::ChatIntent;
#[must_use]
pub fn available(panel: &ChatPanel) -> bool {
    panel.selected && panel.guild_selected && matches!(panel.guild, GuildRoomState::Member { .. })
}
pub fn request(panel: &mut ChatPanel) -> Option<ChatIntent> {
    if !available(panel) || panel.pending {
        panel.feedback = Some(tr!("Guild chat unavailable or waiting for the server").into());
        return None;
    }
    if panel.text.trim().is_empty() || panel.text.len() > 1_020 || panel.text.contains('\0') {
        panel.feedback = Some(tr!("Enter a message of at most 1,020 bytes").into());
        return None;
    }
    panel.pending = true;
    panel.feedback = None;
    Some(ChatIntent::Guild {
        text: panel.text.clone(),
    })
}

/// Replace the chat snapshot projection; negotiated Guild Own responses in play
/// override this join-time membership after guild queries and actions.
pub fn replace_state(panel: &mut ChatPanel, selected: bool, guild: GuildRoomState) {
    panel.guild_selected = selected;
    panel.guild = guild;
}
