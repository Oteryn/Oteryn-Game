//! Mounted inside play's Store view to keep mailbox access within its owner.
use crate::combat_input::PlayCommand;
use crate::play::{Outbox, PlayLink, mailbox::lock};
use oteryn_client_runtime::raw_feature::{FeatureCommand, GatedCommandResult};
impl Outbox {
    pub(crate) fn premium_game_initialize(&self, selected: bool) {
        lock(&self.0).premium_game = crate::premium::game::Panel {
            selected,
            ..Default::default()
        };
    }
    pub(crate) fn premium_game_result(
        &self,
        command: &FeatureCommand,
        result: &GatedCommandResult,
    ) {
        lock(&self.0).premium_game.apply(command, result);
    }
}
impl PlayLink {
    pub fn premium_overview_text(&self) -> String {
        lock(&self.mailbox).premium_game.view.status_text()
    }
    pub fn premium_overview_refresh(&self) {
        let mut mailbox = lock(&self.mailbox);
        if let Some(command) = mailbox.premium_game.request()
            && self.commands.send(PlayCommand::Feature(command)).is_err()
        {
            mailbox.premium_game.fail("The session has ended.");
        }
    }
    pub fn premium_overview_open(&self) {
        self.premium_overview_refresh();
        lock(&self.mailbox).premium_game.open = true;
    }
    pub fn premium_overview_input(
        &self,
        events: &[oteryn_input_actions::NormalizedInputEvent],
    ) -> bool {
        use oteryn_input_actions::{ButtonState, Modifier, Modifiers, NormalizedInputEvent};
        let chord = Modifiers::one(Modifier::Control)
            .with(Modifier::Alt)
            .with(Modifier::Shift);
        if events.iter().any(|event|matches!(event,NormalizedInputEvent::Key {code,state:ButtonState::Pressed,modifiers,repeat:false} if code.get()==19&&*modifiers==chord)) {self.premium_overview_open();true}else{false}
    }
}

pub(in crate::play) fn premium_form(_link: &PlayLink) -> crate::play::feature_0115_mail::view::form::Form {
    use crate::play::feature_0115_mail::view::form::Form;
    Form {
        title: "Oteryn Premium overview",
        labels: &[],
        buttons: &["Refresh Premium status"],
        snapshot: |mailbox| {
            let p = &mailbox.premium_game;
            (p.open, p.pending, Vec::new(), p.view.status_text())
        },
        set_open: |mailbox, open| mailbox.premium_game.open = open,
        submit: |mailbox, commands, _, _| {
            if let Some(command) = mailbox.premium_game.request()
                && commands
                    .send(crate::combat_input::PlayCommand::Feature(command))
                    .is_err()
            {
                mailbox.premium_game.fail("The session has ended.");
            }
        },
    }
}
