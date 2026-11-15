#[cfg(feature = "bevy-shell")]
#[derive(Debug, Clone)]
pub(crate) struct LookMessage {
    pub receipt: u64,
    pub text: String,
}

#[cfg(feature = "bevy-shell")]
impl crate::play::PlayLink {
    pub(crate) fn look_message(&self) -> Option<LookMessage> {
        crate::play::mailbox::lock(&self.mailbox)
            .screen_look
            .clone()
    }
}

#[cfg(feature = "bevy-shell")]
impl crate::play::PlayView {
    pub(crate) fn screen_message_controls(&self) -> (bool, bool, u64) {
        let center =
            crate::play::feature_local_game_notifications::state::lock(&self.game_notifications.0);
        (
            center.banners_enabled,
            center.popups_enabled,
            self.game_notifications
                .0
                .screen_dismissals
                .load(std::sync::atomic::Ordering::Relaxed),
        )
    }
}

#[cfg(feature = "bevy-shell")]
pub(in crate::play) fn record_look(
    mailbox: &mut crate::play::mailbox::Mailbox,
    action: oteryn_client_runtime::world_interaction::Action,
    text: &str,
) {
    if action != oteryn_client_runtime::world_interaction::Action::Look {
        return;
    }
    let receipt = mailbox
        .screen_look
        .as_ref()
        .map_or(1, |held| held.receipt.saturating_add(1));
    mailbox.screen_look = Some(LookMessage {
        receipt,
        text: text.into(),
    });
    let received = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    mailbox
        .chat
        .console
        .notices
        .push(oteryn_client_ui::chat::NoticeClass::Status, text, received);
}

#[cfg(all(test, feature = "bevy-shell"))]
mod tests;
