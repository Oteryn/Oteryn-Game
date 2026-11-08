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
                "Map bug position: x,y,floor",
                "Comment",
            ],
            buttons: &["Bug", "Violation"],
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
                let intent = match action {
                    0 => oteryn_client_ui::player_report::Intent::Bug,
                    1 => oteryn_client_ui::player_report::Intent::Violation,
                    _ => return,
                };
                super::input::submit(mailbox, commands, intent);
            },
        },
    );
}

#[path = "bug.rs"]
pub(in crate::play) mod bug;
