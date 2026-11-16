use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::cyclopedia_tracker::CyclopediaTrackerPanel;
impl PlayLink {
    #[must_use]
    pub fn cyclopedia_tracker_panel(&self) -> CyclopediaTrackerPanel {
        lock(&self.mailbox).cyclopedia_tracker.clone()
    }
}

/// Bestiary and Bosstiary tracking for the shared frontend.
pub(in crate::play) fn form(_link: &PlayLink) -> crate::play::feature_0115_mail::view::form::Form {
    use oteryn_client_ui::cyclopedia_tracker::{Intent, panel_text};
    crate::play::feature_0115_mail::view::form::Form {
        title: tr!("Cyclopedia tracker"),
        labels: &["Creature key", "Bosstiary: 1 yes, 0 no"],
        buttons: &["Add", "Remove", "Refresh"],
        snapshot: |mailbox| {
            let panel = &mailbox.cyclopedia_tracker;
            let key = if panel.creature_key.is_empty() {
                "oteryn:creature.".to_owned()
            } else {
                panel.creature_key.clone()
            };
            (
                panel.open,
                panel.pending,
                vec![key, if panel.boss { "1" } else { "0" }.to_owned()],
                panel_text(panel),
            )
        },
        set_open: |mailbox, value| mailbox.cyclopedia_tracker.open = value,
        submit: |mailbox, commands, action, values| {
            use crate::play::feature_0115_mail::view::form::field;
            let intent = match action {
                0 => Intent::Add,
                1 => Intent::Remove,
                2 => Intent::Read,
                _ => return,
            };
            mailbox.cyclopedia_tracker.creature_key = field(&values, 0).to_owned();
            mailbox.cyclopedia_tracker.boss = field(&values, 1) == "1";
            super::input::enqueue(mailbox, commands, intent);
        },
    }
}
