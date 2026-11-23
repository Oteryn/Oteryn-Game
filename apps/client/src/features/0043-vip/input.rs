#[path = "account.rs"]
pub(in crate::play) mod account;
use crate::combat_input::PlayCommand;
use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::vip::{Intent, request};
use oteryn_input_actions::{ButtonState, Modifier, NormalizedInputEvent};

impl PlayLink {
    #[cfg(bevy_shell)]
    pub(crate) fn vip_private_channel_available(&self) -> bool {
        let mailbox = lock(&self.mailbox);
        owns_private_channel(&mailbox.chat_private)
    }

    #[cfg(bevy_shell)]
    pub(crate) fn vip_private_recipient(&self, name: &str, invite: bool) {
        use oteryn_client_ui::chat_private_channel::Intent as PrivateIntent;
        let mut mailbox = lock(&self.mailbox);
        if !owns_private_channel(&mailbox.chat_private) {
            return;
        }
        let draft = std::mem::replace(&mut mailbox.chat_private.name, name.to_owned());
        let intent = if invite {
            PrivateIntent::Invite
        } else {
            PrivateIntent::Exclude
        };
        crate::play::feature_0080_chat_private_channel::input::enqueue(
            &mut mailbox,
            &self.commands,
            intent,
        );
        mailbox.chat_private.name = draft;
    }

    pub fn vip_request(&self, intent: Intent) {
        let mut mailbox = lock(&self.mailbox);
        if let Some(command) = request(&mut mailbox.vip, intent)
            && self.commands.send(PlayCommand::Vip(command)).is_err()
        {
            mailbox.vip.pending = false;
            mailbox.vip.feedback = Some(tr!("The session has ended").into());
        }
    }
    pub fn vip_input(&self, events: &[NormalizedInputEvent]) -> bool {
        if events.iter().any(|event| matches!(event,NormalizedInputEvent::Key {code,state:ButtonState::Pressed,modifiers,repeat:false} if *code==oteryn_input_actions::KeyCode::KEY_F && modifiers.contains(Modifier::Control) && modifiers.contains(Modifier::Alt) && !modifiers.contains(Modifier::Shift) && !modifiers.contains(Modifier::Super))) {
            self.account_friends_open();return true;
        }
        let open = events.iter().any(|event| matches!(event, NormalizedInputEvent::Key { code, state: ButtonState::Pressed, modifiers, repeat: false } if code.get() == 25 && modifiers.contains(Modifier::Control) && !modifiers.contains(Modifier::Alt) && !modifiers.contains(Modifier::Shift) && !modifiers.contains(Modifier::Super)));
        if !open {
            return false;
        }
        self.vip_request(Intent::Query(0));
        lock(&self.mailbox).vip.open = true;
        true
    }
}

#[cfg(bevy_shell)]
fn owns_private_channel(panel: &oteryn_client_ui::chat_private_channel::ChatPanel) -> bool {
    panel.selected
        && !panel.pending
        && panel
            .list
            .as_ref()
            .is_some_and(|list| list.own_channel.is_some())
}
