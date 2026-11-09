//! Native admitted query flow for Battle Instinct.
use crate::combat_input::PlayCommand;
use crate::play::{Outbox, PlayLink, mailbox::lock};
use oteryn_client_runtime::raw_feature::FeatureCommand;
use oteryn_input_actions::NormalizedInputEvent;
impl PlayLink {
    pub fn battle_instinct_status_request(&self) {
        let mut mailbox = lock(&self.mailbox);
        if let Some(payload) =
            oteryn_client_ui::battle_instinct_status::request(&mut mailbox.battle_instinct_status)
            && self
                .commands
                .send(PlayCommand::Feature(FeatureCommand {
                    capability: 278,
                    kind: 494,
                    payload,
                }))
                .is_err()
        {
            mailbox.battle_instinct_status.pending = false;
            mailbox.battle_instinct_status.error = Some(tr!("Session ended"));
        }
    }
    pub fn battle_instinct_status_input(&self, events: &[NormalizedInputEvent]) -> bool {
        if !oteryn_client_ui::battle_instinct_status::open_key(events) {
            return false;
        }
        self.battle_instinct_status_request();
        #[cfg(windows)]
        crate::play::status_window::open_window(
            self,
            tr!("Battle Instinct — Ctrl+Alt+Shift+B"),
            |mailbox| &mut mailbox.battle_instinct_status,
        );
        #[cfg(not(windows))]
        {
            lock(&self.mailbox).battle_instinct_status.open = true;
        }
        true
    }
    pub fn battle_instinct_status_panel(
        &self,
    ) -> oteryn_client_ui::battle_instinct_status::BattleInstinctStatusPanel {
        lock(&self.mailbox).battle_instinct_status.clone()
    }
}
impl Outbox {
    pub(in crate::play) fn battle_instinct_status_initialize(&self, selected: bool) {
        lock(&self.0).battle_instinct_status.selected = selected;
    }
    pub(in crate::play) fn battle_instinct_status_result(
        &self,
        result: &oteryn_session::GatedCommandResult,
    ) {
        let accepted = matches!(
            result.status,
            oteryn_session::CommandStatus::Accepted
                | oteryn_session::CommandStatus::DuplicateReplay
        );
        lock(&self.0)
            .battle_instinct_status
            .finish(accepted, &result.payload);
    }
}
