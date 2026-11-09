use crate::play::{
    PlayLink,
    mailbox::{Mailbox, lock},
};
impl PlayLink {
    pub fn cast_chat_input(&self, events: &[oteryn_input_actions::NormalizedInputEvent]) -> bool {
        if !oteryn_client_ui::cast_chat::open_key(events) {
            return false;
        }
        #[cfg(windows)]
        super::view::open_window(self);
        #[cfg(not(windows))]
        {
            lock(&self.mailbox).cast_chat.open = true;
        }
        true
    }
    pub fn cast_chat_request(&self, text: &str, subject: &str, action: u8) {
        submit(
            &mut lock(&self.mailbox),
            &self.commands,
            text,
            subject,
            action,
        );
    }
}
pub(in crate::play) fn submit(
    mailbox: &mut Mailbox,
    commands: &tokio::sync::mpsc::UnboundedSender<crate::combat_input::PlayCommand>,
    text: &str,
    subject: &str,
    action: u8,
) {
    let mut key = [0; 16];
    if action != 0 && oteryn_platform_client::native_login::secure_random(&mut key).is_err() {
        mailbox.cast_chat.feedback = tr!("Cast action identifier unavailable").into();
        return;
    }
    let result = if action == 0 {
        oteryn_client_ui::cast_chat::protocol::command(
            &mailbox.cast_chat,
            &mailbox.cast_control,
            text,
            subject,
            action,
            key,
        )
    } else {
        oteryn_client_ui::cast_control::moderate(
            &mailbox.cast_chat,
            &mailbox.cast_control,
            subject,
            action,
            key,
        )
    };
    match result {
        Ok(command) => {
            mailbox.cast_chat.pending = commands
                .send(crate::combat_input::PlayCommand::ServerMechanic(command))
                .is_ok();
            if !mailbox.cast_chat.pending {
                mailbox.cast_chat.feedback = tr!("The session has ended").into();
            }
        }
        Err(error) => mailbox.cast_chat.feedback = error.into(),
    }
}
