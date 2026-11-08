use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::imbue::ImbuePanel;

impl PlayLink {
    #[must_use]
    pub fn imbue_panel(&self) -> ImbuePanel {
        lock(&self.mailbox).imbue.clone()
    }

    pub fn open_imbue(&self) {
        self.imbue_request(oteryn_client_ui::imbue::Intent::Refresh);
        #[cfg(windows)]
        open_window(self);
        #[cfg(not(windows))]
        {
            lock(&self.mailbox).imbue.open = true;
        }
    }
}

#[cfg(windows)]
#[path = "native.rs"]
mod native;
#[cfg(windows)]
pub(in crate::play) use native::open_window;

impl PlayLink {
    pub fn open_imbue_tracker(&self) {
        if lock(&self.mailbox).imbue.tracker_open {
            return;
        }
        #[cfg(windows)]
        {
            use crate::play::feature_0115_mail::view::native::{Form, open};
            open(
                self,
                Form {
                    title: "Imbuement time tracker — Ctrl+Alt+Shift+I",
                    labels: &["Maximum minutes (0 = all)"],
                    buttons: &["Apply time filter", "Refresh equipment"],
                    snapshot: |m| {
                        (
                            m.imbue.tracker_open,
                            m.imbue.equipment.pending.is_some(),
                            vec![m.imbue.tracker_minutes.clone()],
                            oteryn_client_ui::imbue::tracker_text(&m.imbue),
                        )
                    },
                    set_open: |m, value| m.imbue.tracker_open = value,
                    submit: |m, commands, action, values| {
                        if let Some(value) = values.first() {
                            m.imbue.tracker_minutes.clone_from(value);
                        }
                        if action == 1 {
                            super::input::tracker::enqueue(m, commands, true);
                        }
                    },
                },
            );
        }
        #[cfg(not(windows))]
        {
            lock(&self.mailbox).imbue.tracker_open = true;
        }
        self.imbue_tracker_refresh();
        #[cfg(windows)]
        super::input::tracker::poll(
            std::sync::Arc::downgrade(&self.mailbox),
            self.commands.clone(),
        );
    }
}
