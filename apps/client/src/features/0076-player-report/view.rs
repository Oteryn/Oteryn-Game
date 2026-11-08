use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    pub fn player_report_panel(&self) -> oteryn_client_ui::player_report::Panel {
        lock(&self.mailbox).properties.player_report.clone()
    }
}
#[cfg(windows)]
pub(in crate::play) fn open_window(link: &PlayLink) {
    use crate::play::feature_0115_mail::view::native::{Form, open};
    open(
        link,
        Form {
            title: "Player reports",
            labels: &[
                "Category (bug) or reason (violation)",
                "Report type: 0 name, 1 statement, 2 bot",
                "Reported character (violation)",
                "Map bug position / optional violation translation",
                "Comment",
            ],
            buttons: &["Bug", "Review violation", "Send violation", "Name", "Statement", "Bot"],
            snapshot: |mailbox| {
                let panel = &mailbox.properties.player_report;
                (
                    panel.open,
                    panel.pending,
                    panel.fields.to_vec(),
                    oteryn_client_ui::player_report::panel_text(panel),
                )
            },
            set_open: |mailbox, value| mailbox.properties.player_report.open = value,
            submit: |mailbox, commands, action, values| {
                let panel = &mut mailbox.properties.player_report;
                if panel.pending {
                    return;
                }
                for (index, value) in values.into_iter().enumerate() {
                    super::input::edit(panel, index, value);
                }
                match action {
                    0 => super::input::submit(mailbox, commands, oteryn_client_ui::player_report::Intent::Bug),
                    1 | 2 => violation::activate(mailbox, commands, action == 1),
                    3..=5 => {
                        let panel = &mut mailbox.properties.player_report;
                        panel.fields[1] = (action - 3).to_string();
                        panel.violation_reviewed = None;
                        panel.feedback = "Choose the operator's reason code, describe the evidence, then review.".into();
                    }
                    _ => {}
                }
            },
        },
    );
}

#[path = "bug.rs"]
pub(in crate::play) mod bug;

#[path = "violation.rs"]
pub(in crate::play) mod violation;
