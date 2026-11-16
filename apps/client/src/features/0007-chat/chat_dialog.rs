//! Local chat dialog message pump on its owning thread.

/// Whether this Send invocation queued a message and may clear the native composer.
pub(in crate::play) fn submit_composer(
    mailbox: &mut crate::play::mailbox::Mailbox,
    commands: &tokio::sync::mpsc::UnboundedSender<crate::combat_input::PlayCommand>,
    mode: oteryn_client_ui::chat::Mode,
    recipient: &str,
) -> bool {
    let was_pending = mailbox.chat.pending;
    crate::play::feature_0007_chat::input::enqueue(mailbox, commands, mode, recipient);
    !was_pending && mailbox.chat.pending
}

/// Keep utility shortcuts and composer submission on the same message-pump route.
pub(in crate::play) fn route_keyboard(
    utility_consumed: bool,
    composer_enter: bool,
    actions: &mut Vec<usize>,
) -> bool {
    if utility_consumed {
        return true;
    }
    if composer_enter {
        if actions.len() < 16 {
            actions.push(105); // SEND uses the existing button handler below the message pump.
        }
        return true;
    }
    false
}

#[cfg(test)]
#[path = "tests/native_composer.rs"]
mod tests;
