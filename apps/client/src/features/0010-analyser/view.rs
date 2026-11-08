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
            labels: &[
                "XP still needed for your level goal (blank disables)",
                "Item number from loot or supply list",
                "Unit item value (gold)",
                "Session export file path",
            ],
            buttons: &[
                "Overview",
                "Hunting",
                "Experience",
                "Loot",
                "Supplies",
                "Damage",
                "Healing",
                "Impact",
                "Input",
                "Hunting session",
                "Party hunt",
                "Loot tracker",
                "Kill tracker",
                "Reset session",
                "Apply level goal",
                "Apply item price",
                "Resolve item info",
                "Export (replace file)",
                "Refresh party hunt",
                "Party supply price",
            ],
            snapshot: |mailbox| {
                let panel = &mailbox.analyser;
                (
                    panel.open,
                    false,
                    vec![
                        panel.goal_input.clone(),
                        panel.price_input[0].clone(),
                        panel.price_input[1].clone(),
                        panel.export_path.clone(),
                    ],
                    oteryn_client_ui::analyser::panel_text(panel),
                )
            },
            set_open: |mailbox, value| mailbox.analyser.open = value,

            submit: super::input::submit,
        },
    );
}
