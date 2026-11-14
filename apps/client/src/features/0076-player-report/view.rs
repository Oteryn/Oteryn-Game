use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    pub fn player_report_panel(&self) -> oteryn_client_ui::player_report::Panel {
        lock(&self.mailbox).properties.player_report.clone()
    }
}
#[cfg(windows)]
pub(in crate::play) fn open_window(link: &PlayLink) {
    if link.uses_shared_frontend() {
        return;
    }
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
            buttons: &[
                "Bug",
                "Review violation",
                "Send violation",
                "Name",
                "Statement",
                "Bot",
                "Previous reason",
                "Next reason",
            ],
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
                    0 => super::input::submit(
                        mailbox,
                        commands,
                        oteryn_client_ui::player_report::Intent::Bug,
                    ),
                    1 | 2 => violation::activate(mailbox, commands, action == 1),
                    3..=5 => {
                        let panel = &mut mailbox.properties.player_report;
                        panel.fields[1] = (action - 3).to_string();
                        panel.violation_reviewed = None;
                        panel.feedback = tr!(
                            "Choose the operator's reason code, describe the evidence, then review."
                        )
                        .into();
                    }
                    6 | 7 => mailbox.properties.player_report.cycle_reason(action == 7),
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

#[cfg(all(test, feature = "bevy-shell"))]
#[path = "tests/bevy_bound.rs"]
mod bevy_bound;

/// Exact owner observation; the private weak identity cannot bind to another session.
#[cfg(feature = "bevy-shell")]
#[derive(Clone)]
pub struct PlayerReportSnapshot {
    pub panel: oteryn_client_ui::player_report::Panel,
    session: std::sync::Weak<std::sync::Mutex<crate::play::mailbox::Mailbox>>,
}
#[cfg(feature = "bevy-shell")]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PlayerReportControl {
    Bug,
    ReviewViolation,
    SendViolation,
    ReportType(u8),
    Reason(bool),
    Close,
}
#[cfg(feature = "bevy-shell")]
impl PlayerReportSnapshot {
    pub fn same_session(&self, other: &Self) -> bool {
        self.session.ptr_eq(&other.session)
    }
}
#[cfg(feature = "bevy-shell")]
impl PlayLink {
    pub fn player_report_snapshot(&self) -> PlayerReportSnapshot {
        PlayerReportSnapshot {
            panel: self.player_report_panel(),
            session: std::sync::Arc::downgrade(&self.mailbox),
        }
    }
    /// Validate and act under the same owner lock used by the native presenter.
    pub fn player_report_dispatch_bound(
        &self,
        expected: &PlayerReportSnapshot,
        fields: [String; 5],
        action: PlayerReportControl,
    ) -> bool {
        if !expected
            .session
            .ptr_eq(&std::sync::Arc::downgrade(&self.mailbox))
        {
            return false;
        }
        let mut mailbox = lock(&self.mailbox);
        let panel = &mailbox.properties.player_report;
        if *panel != expected.panel
            || !panel.open
            || self.commands.is_closed()
            || mailbox.inventory_shutdown
            || mailbox.server_modal.is_active()
        {
            return false;
        }
        if action == PlayerReportControl::Close {
            mailbox.properties.player_report.open = false;
            return true;
        }
        if !panel.selected
            || panel.pending
            || fields
                .iter()
                .any(|field| field.len() > 7996 || field.contains('\0'))
            || matches!(action, PlayerReportControl::ReportType(kind) if kind > 2)
        {
            return false;
        }
        for (index, value) in fields.into_iter().enumerate() {
            super::input::edit(&mut mailbox.properties.player_report, index, value);
        }
        match action {
            PlayerReportControl::Bug => super::input::submit(
                &mut mailbox,
                &self.commands,
                oteryn_client_ui::player_report::Intent::Bug,
            ),
            PlayerReportControl::ReviewViolation | PlayerReportControl::SendViolation => {
                violation::activate(
                    &mut mailbox,
                    &self.commands,
                    action == PlayerReportControl::ReviewViolation,
                );
            }
            PlayerReportControl::ReportType(kind) => {
                let panel = &mut mailbox.properties.player_report;
                panel.fields[1] = kind.to_string();
                panel.violation_reviewed = None;
                panel.feedback =
                    tr!("Choose the operator's reason code, describe the evidence, then review.")
                        .into();
            }
            PlayerReportControl::Reason(next) => {
                mailbox.properties.player_report.cycle_reason(next)
            }
            PlayerReportControl::Close => return false,
        }
        true
    }
}

#[cfg(all(test, feature = "bevy-shell"))]
impl crate::play::Outbox {
    pub(crate) fn bevy_player_report_selected(&self) {
        lock(&self.0).properties.player_report.selected = true;
    }
}
