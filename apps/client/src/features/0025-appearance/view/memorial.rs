//! Source NPC dialogue shortcuts; every economic confirmation is an explicit player reply.
use crate::{
    combat_input::PlayCommand,
    play::{
        PlayLink,
        mailbox::{Mailbox, lock},
    },
};
use oteryn_client_ui::appearance::MemorialAction;
use tokio::sync::mpsc::UnboundedSender;
pub(in crate::play) fn submit(
    mailbox: &mut Mailbox,
    commands: &UnboundedSender<PlayCommand>,
    action: MemorialAction,
) {
    let panel = &mut mailbox.properties.appearance.memorial;
    panel.navigate(action);
    let Some(text) = panel.phrase(action) else {
        return;
    };
    let Some(target) = mailbox.dialogue.target else {
        return;
    };
    if let Some(intent) =
        oteryn_client_ui::npc_dialogue::request(&mut mailbox.dialogue, target, text)
        && commands.send(PlayCommand::NpcTalk(intent)).is_err()
    {
        mailbox.dialogue = Default::default();
    }
}
impl PlayLink {
    pub fn memorial_action(&self, action: MemorialAction) {
        submit(&mut lock(&self.mailbox), &self.commands, action);
    }
    /// Opens the Outfit Memorial for the shared frontend, from the outfit window's button.
    pub fn memorial_open(&self) {
        lock(&self.mailbox).properties.appearance.memorial.open = true;
    }
}

pub(in crate::play) fn form(_link: &PlayLink) -> crate::play::feature_0115_mail::view::form::Form {
    use crate::play::feature_0115_mail::view::form::Form;
    Form {
        title: "Outfit Memorial",
        labels: &[],
        buttons: &[
            "Golden outfits",
            "Token exchange",
            "Previous",
            "Next",
            "Female / male",
            "Preview addons",
            "Ask selected NPC",
            "Yes (current quote)",
            "No",
            "Armour",
            "Boots",
            "Helmet",
        ],
        snapshot: |mailbox| {
            let panel = &mailbox.properties.appearance.memorial;
            let text = format!(
                "{}\r\n\r\nCurrent server conversation:\r\n{}",
                panel.text(),
                mailbox.dialogue.text()
            );
            (panel.open, mailbox.dialogue.pending, Vec::new(), text)
        },
        set_open: |mailbox, value| mailbox.properties.appearance.memorial.open = value,
        submit: |mailbox, commands, index, _| {
            let actions = [
                MemorialAction::Golden,
                MemorialAction::Tokens,
                MemorialAction::Previous,
                MemorialAction::Next,
                MemorialAction::Gender,
                MemorialAction::Addons,
                MemorialAction::Ask,
                MemorialAction::Yes,
                MemorialAction::No,
                MemorialAction::Armour,
                MemorialAction::Boots,
                MemorialAction::Helmet,
            ];
            if let Some(action) = actions.get(index) {
                submit(mailbox, commands, *action);
            }
        },
    }
}
#[cfg(test)]
#[path = "../tests/memorial.rs"]
mod tests;
