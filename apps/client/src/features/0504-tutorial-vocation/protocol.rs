use crate::{
    combat_input::PlayCommand,
    play::{Outbox, PlayLink, mailbox::lock},
};
use oteryn_client_runtime::raw_feature::GatedCommandResult;
use oteryn_client_ui::tutorial_vocation::{VocationPanel, apply_result, request};
impl Outbox {
    pub(in crate::play) fn tutorial_vocation_initialize(&self, selected: bool) {
        lock(&self.0).tutorial_vocation = VocationPanel {
            selected,
            ..Default::default()
        };
    }
    pub(in crate::play) fn tutorial_vocation_result(&self, result: &GatedCommandResult) {
        apply_result(&mut lock(&self.0).tutorial_vocation, result);
    }
}
impl PlayLink {
    pub fn tutorial_vocation_panel(&self) -> VocationPanel {
        lock(&self.mailbox).tutorial_vocation.clone()
    }
    pub fn tutorial_vocation_request(&self, choice: Option<u8>) {
        let mut mailbox = lock(&self.mailbox);
        if let Some(command) = request(&mut mailbox.tutorial_vocation, choice)
            && self.commands.send(PlayCommand::Feature(command)).is_err()
        {
            mailbox.tutorial_vocation.pending = false;
            mailbox.tutorial_vocation.observation = None;
        }
    }
}

#[cfg(all(test, bevy_shell))]
impl Outbox {
    pub(crate) fn bevy_tutorial_vocation_initialize(&self) {
        self.tutorial_vocation_initialize(true);
    }
    pub(crate) fn bevy_tutorial_vocation_result(&self, result: &GatedCommandResult) {
        self.tutorial_vocation_result(result);
    }
}
