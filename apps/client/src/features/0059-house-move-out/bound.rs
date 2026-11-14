//! Bevy presentation bindings; all capability-59 commands use the retained owner.
use crate::{
    combat_input::PlayCommand,
    play::{PlayLink, mailbox::lock},
};
use oteryn_client_ui::house_move_out::{Intent, Panel};
use tokio::sync::mpsc::UnboundedSender;

/// A snapshot belongs to the actual session command channel, even if panels look identical.
#[derive(Clone)]
pub(crate) struct Snapshot {
    pub(crate) panel: Panel,
    commands: UnboundedSender<PlayCommand>,
}
impl PartialEq for Snapshot {
    fn eq(&self, other: &Self) -> bool {
        self.commands.same_channel(&other.commands) && self.panel == other.panel
    }
}
pub(crate) enum Action {
    Close,
    Request {
        intent: Intent,
        fields: [String; 2],
        confirmed: bool,
    },
}
impl PlayLink {
    pub(crate) fn house_move_out_snapshot(&self) -> Snapshot {
        Snapshot {
            panel: self.house_move_out_panel(),
            commands: self.commands.clone(),
        }
    }
    pub(crate) fn house_move_out_toggle_bound(&self) -> bool {
        let mut mailbox = lock(&self.mailbox);
        if self.commands.is_closed()
            || mailbox.world_input_blocked()
            || mailbox.server_modal.is_active()
        {
            return false;
        }
        mailbox.properties.house_move_out.open = !mailbox.properties.house_move_out.open;
        true
    }
    /// Validate, prepare and enqueue under the same lock. Rejection never changes the owner.
    pub(crate) fn house_move_out_dispatch_bound(
        &self,
        expected: &Snapshot,
        action: Action,
    ) -> bool {
        let mut mailbox = lock(&self.mailbox);
        if !self.commands.same_channel(&expected.commands)
            || self.commands.is_closed()
            || mailbox.world_input_blocked()
            || mailbox.server_modal.is_active()
            || mailbox.properties.house_move_out != expected.panel
            || !expected.panel.open
        {
            return false;
        }
        let Action::Request {
            intent,
            fields,
            confirmed,
        } = action
        else {
            mailbox.properties.house_move_out.open = false;
            return true;
        };
        if !expected.panel.selected
            || expected.panel.pending
            || (intent != Intent::Query && !confirmed)
            || fields
                .iter()
                .any(|field| field.len() > 512 || field.contains('\0'))
        {
            return false;
        }
        let mut prepared = expected.panel.clone();
        prepared.fields = fields;
        let Ok(command) = super::super::protocol::command(&prepared, intent) else {
            return false;
        };
        if !oteryn_client_ui::house_move_out::request(&mut prepared, command.kind)
            || self.commands.send(PlayCommand::Feature(command)).is_err()
        {
            return false;
        }
        mailbox.properties.house_move_out = prepared;
        true
    }
}

#[cfg(test)]
impl crate::play::Outbox {
    pub(crate) fn bevy_house_move_out_panel(&self, panel: Panel) {
        lock(&self.0).properties.house_move_out = panel;
    }
    pub(crate) fn bevy_house_move_out_reply(&self, payload: &[u8]) {
        let mut mailbox = lock(&self.0);
        let panel = &mut mailbox.properties.house_move_out;
        let kind = panel.pending_kind;
        super::super::protocol::apply(
            panel,
            &oteryn_client_runtime::raw_feature::FeatureCommand {
                capability: 59,
                kind,
                payload: Vec::new(),
            },
            &oteryn_client_runtime::raw_feature::GatedCommandResult {
                command_id: 1,
                server_sequence: 1,
                status: oteryn_session::CommandStatus::Accepted,
                payload: payload.to_vec(),
            },
        );
    }
}

#[cfg(test)]
#[path = "tests/bound.rs"]
mod tests;
