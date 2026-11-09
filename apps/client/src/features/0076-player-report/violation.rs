//! A preview is bound to the exact encoded report, including any optional translation.
use crate::combat_input::PlayCommand;
use crate::play::mailbox::Mailbox;
use oteryn_client_ui::player_report::Intent;
use tokio::sync::mpsc::UnboundedSender;

pub(in crate::play) fn activate(
    mailbox: &mut Mailbox,
    commands: &UnboundedSender<PlayCommand>,
    review: bool,
) {
    let panel = &mut mailbox.properties.player_report;
    if !panel.selected || panel.pending {
        return;
    }
    let mut draft = panel.clone();
    if !draft.fields[3].trim().is_empty() {
        draft.fields[4].push_str(tr!("\nReporter translation (unverified): "));
        draft.fields[4].push_str(&panel.fields[3]);
    }
    let prepared = match super::super::protocol::command(&draft, Intent::Violation) {
        Ok(command) => command,
        Err(message) => {
            panel.feedback = message;
            return;
        }
    };
    if review {
        panel.violation_preview = lfmt!(
            "Review before sending\r\nCharacter: {}\r\nType: {} (0 name, 1 statement, 2 bot)\r\n{}\r\n{}",
            draft.fields[2],
            draft.fields[1],
            draft.reason_help(),
            draft.fields[4]
        );
        panel.violation_reviewed = Some(prepared.payload);
        panel.feedback = tr!(
            "Confirm the exact report above. Editing any submitted value requires review again."
        )
        .into();
    } else {
        if panel.violation_reviewed.as_ref() != Some(&prepared.payload) {
            panel.feedback = tr!("Review this violation before sending").into();
            return;
        }
        panel.violation_reviewed = None;
        if oteryn_client_ui::player_report::request(panel, prepared.kind)
            && commands.send(PlayCommand::Feature(prepared)).is_err()
        {
            panel.failed(tr!("The session has ended"));
        }
    }
}

impl crate::play::PlayLink {
    /// Review or confirm a violation through the same path as the native form.
    pub fn violation_review(&self, review: bool) {
        activate(
            &mut crate::play::mailbox::lock(&self.mailbox),
            &self.commands,
            review,
        );
    }
}
