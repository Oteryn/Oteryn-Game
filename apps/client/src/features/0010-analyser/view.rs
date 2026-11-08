use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    pub fn analyser_panel(&self) -> oteryn_client_ui::analyser::Panel {
        lock(&self.mailbox).analyser.clone()
    }
}
#[cfg(windows)]
pub(in crate::play) fn open_window(link: &PlayLink) {
    use crate::play::feature_0115_mail::view::native::{Form, open};
    open(
        link,
        Form {
            title: "Analysers",
            labels: &["XP still needed for your level goal (blank disables)"],
            buttons: &[
                "Overview",
                "XP",
                "Loot",
                "Supply",
                "Impact",
                "Input",
                "Hunting session",
                "Party hunt",
                "Loot tracker",
                "Kill tracker",
                "Reset session",
                "Apply level goal",
            ],
            snapshot: |mailbox| {
                let panel = &mailbox.analyser;
                (
                    panel.open,
                    false,
                    vec![panel.goal_input.clone()],
                    oteryn_client_ui::analyser::panel_text(panel),
                )
            },
            set_open: |mailbox, value| mailbox.analyser.open = value,

            submit: |mailbox, _, action, values| {
                if let Some(tool) = oteryn_client_ui::analyser::Tool::ALL.get(action) {
                    oteryn_client_ui::analyser::select_tool(&mut mailbox.analyser, tool.key());
                } else if action == 10 {
                    oteryn_client_ui::analyser::protocol::reset(&mut mailbox.analyser);
                } else if action == 11
                    && let Some(value) = values.first()
                {
                    oteryn_client_ui::analyser::set_xp_goal(&mut mailbox.analyser, value);
                }
            },
        },
    );
}
