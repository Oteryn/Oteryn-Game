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
        #[cfg(windows)]
        super::premium_native::open(self);
        #[cfg(not(windows))]
        {
            lock(&self.mailbox).premium_game.open = true;
        }
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
