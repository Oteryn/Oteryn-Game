//! Native admitted query flow for Party effects.
use crate::combat_input::PlayCommand;
use crate::play::{Outbox, PlayLink, mailbox::lock};
use oteryn_client_runtime::raw_feature::FeatureCommand;
use oteryn_input_actions::NormalizedInputEvent;
impl PlayLink {
    pub fn party_effects_request(&self) {
        let mut mailbox = lock(&self.mailbox);
        if let Some(payload) = oteryn_client_ui::party_effects::request(&mut mailbox.party_effects)
            && self
                .commands
                .send(PlayCommand::Feature(FeatureCommand {
                    capability: 280,
                    kind: 496,
                    payload,
                }))
                .is_err()
        {
            mailbox.party_effects.pending = false;
            mailbox.party_effects.error = Some(tr!("Session ended"));
        }
    }
    pub fn party_effects_input(&self, events: &[NormalizedInputEvent]) -> bool {
        if !oteryn_client_ui::party_effects::open_key(events) {
            return false;
        }
        self.party_effects_request();
        lock(&self.mailbox).party_effects.open = true;
        true
    }
    pub fn party_effects_panel(&self) -> oteryn_client_ui::party_effects::PartyEffectsPanel {
        lock(&self.mailbox).party_effects.clone()
    }
}
impl Outbox {
    pub(in crate::play) fn party_effects_initialize(&self, selected: bool) {
        lock(&self.0).party_effects.selected = selected;
    }
    pub(in crate::play) fn party_effects_result(
        &self,
        result: &oteryn_session::GatedCommandResult,
    ) {
        let accepted = matches!(
            result.status,
            oteryn_session::CommandStatus::Accepted
                | oteryn_session::CommandStatus::DuplicateReplay
        );
        lock(&self.0)
            .party_effects
            .finish(accepted, &result.payload);
    }
}

/// The status window of the shared frontend: the server's text and one Refresh.
pub(in crate::play) fn form(_link: &PlayLink) -> crate::play::feature_0115_mail::view::form::Form {
    crate::play::feature_0115_mail::view::form::Form {
        title: tr!("Party effects — Ctrl+Alt+Shift+P"),
        labels: &[],
        buttons: &["Refresh"],
        snapshot: |mailbox| {
            let panel = &mailbox.party_effects;
            (
                panel.open && !mailbox.quest_shutdown,
                panel.pending,
                Vec::new(),
                oteryn_client_ui::party_effects::panel_text(panel),
            )
        },
        set_open: |mailbox, value| mailbox.party_effects.open = value,
        submit: |mailbox, commands, _, _| {
            if let Some(payload) = oteryn_client_ui::party_effects::request(&mut mailbox.party_effects)
                && commands
                    .send(PlayCommand::Feature(FeatureCommand {
                        capability: 280,
                        kind: 496,
                        payload,
                    }))
                    .is_err()
            {
                mailbox.party_effects.pending = false;
                mailbox.party_effects.error = Some(tr!("Session ended"));
            }
        },
    }
}
