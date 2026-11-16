use crate::combat_input::PlayCommand;
use crate::play::{
    PlayLink,
    mailbox::{Mailbox, lock},
};
use oteryn_client_runtime::raw_feature::FeatureCommand;
use oteryn_client_ui::player_report::{Intent, Panel};
use tokio::sync::mpsc::UnboundedSender;
const CATEGORIES: &[&str] = &[
    "Map location",
    "Text or spelling",
    "Client behavior",
    "Other issue",
];
fn command(panel: &Panel) -> Result<FeatureCommand, String> {
    if panel.bug.fields[1].trim().is_empty() {
        return Err(tr!("Describe what happened and what you expected").into());
    }
    let mut draft = panel.clone();
    let mut comment = panel.bug.fields[1].clone();
    if panel.bug.fields[2].trim() == "YES" {
        comment.push_str(&lfmt!(
            "\nBuild details: {}; OS: {}; architecture: {}",
            crate::CLIENT_BUILD,
            std::env::consts::OS,
            std::env::consts::ARCH
        ));
    }
    draft.fields = [
        panel.bug.category.to_string(),
        String::new(),
        String::new(),
        panel.bug.fields[0].clone(),
        comment,
    ];
    super::super::protocol::command(&draft, Intent::Bug)
}
pub(in crate::play) fn activate(
    mailbox: &mut Mailbox,
    commands: &UnboundedSender<PlayCommand>,
    action: usize,
) {
    let panel = &mut mailbox.properties.player_report;
    if !panel.selected {
        panel.feedback = tr!("Reports are unavailable in this session").into();
        return;
    }
    if panel.pending {
        return;
    }
    if action < CATEGORIES.len() {
        panel.bug.category = action as u8;
        panel.bug.reviewed = None;
        let guidance = [
            tr!(
                "Coordinates: enter x,y,floor and explain the blocked tile, missing object or incorrect destination."
            ),
            tr!(
                "Quote the original text, where it appears, its language and the suggested correction."
            ),
            tr!(
                "Describe reproducible steps, operating system and the observed client behavior. Optional diagnostics include only build, OS and architecture, never credentials or logs."
            ),
            tr!("Describe the steps, observed behavior and expected result."),
        ][action];
        panel.bug.preview = lfmt!(
            "Category: {}. {guidance} Map reports need x,y,floor; other categories omit the position. Build details are attached only if you enter YES.",
            oteryn_client_ui::localization::tr(CATEGORIES[action]),
            guidance = guidance
        );
        return;
    }
    let prepared = match command(panel) {
        Ok(command) => command,
        Err(message) => {
            panel.feedback = message;
            return;
        }
    };
    if action == 4 {
        match oteryn_client_runtime::player_report::decode_player_report_request(&prepared.payload)
        {
            Ok(request) => {
                panel.bug.preview = lfmt!(
                    "Review: {}\r\nPosition: {},{},{}\r\n{}\r\nSend queues this report for support.",
                    oteryn_client_ui::localization::tr(CATEGORIES[usize::from(panel.bug.category)]),
                    request.map_x,
                    request.map_y,
                    request.map_floor,
                    request.comment
                );
                panel.bug.reviewed = Some(prepared.payload);
                panel.feedback = tr!("Review the exact report above before sending").into();
            }
            Err(_) => panel.feedback = tr!("Cannot prepare report preview").into(),
        }
    } else if action == 5 {
        if panel.bug.reviewed.as_ref() != Some(&prepared.payload) {
            panel.feedback =
                tr!("Review this draft before sending; edits require a new review").into();
            return;
        }
        panel.bug.reviewed = None;
        if oteryn_client_ui::player_report::request(panel, prepared.kind)
            && commands.send(PlayCommand::Feature(prepared)).is_err()
        {
            panel.failed(tr!("The session has ended"));
        }
    }
}
impl PlayLink {
    /// Update only the guided bug draft; the existing flow still validates and sends it.
    pub fn bug_report_edit(&self, field: usize, value: String) -> bool {
        let mut mailbox = lock(&self.mailbox);
        let panel = &mut mailbox.properties.player_report;
        if panel.pending || !panel.bug.open {
            return false;
        }
        if value.len() > 7996 || value.contains('\0') {
            panel.bug.reviewed = None;
            panel.feedback = tr!("Invalid report fields").into();
            return false;
        }
        let Some(slot) = panel.bug.fields.get_mut(field) else {
            return false;
        };
        if *slot != value {
            *slot = value;
            panel.bug.reviewed = None;
            panel.feedback.clear();
            panel.summary.clear();
        }
        true
    }
    pub fn bug_report_close(&self) {
        lock(&self.mailbox).properties.player_report.bug.open = false;
    }
    pub fn bug_report_open(&self) {
        lock(&self.mailbox).properties.player_report.bug.open = true;
    }
    pub fn bug_report_action(&self, action: usize) {
        activate(&mut lock(&self.mailbox), &self.commands, action);
    }
}

#[cfg(test)]
#[path = "tests/e2e_n04.rs"]
mod e2e_n04;

#[cfg(all(test, bevy_shell))]
impl crate::play::Outbox {
    pub(crate) fn bevy_bug_report_result(
        &self,
        command: &oteryn_client_runtime::raw_feature::FeatureCommand,
        result: &oteryn_client_runtime::raw_feature::GatedCommandResult,
    ) {
        self.properties_result(command, result);
    }
    pub(crate) fn bevy_bug_report_selected(&self) {
        lock(&self.0).properties.player_report.selected = true;
    }
}
