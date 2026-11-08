use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    pub fn character_info_panel(&self) -> oteryn_client_ui::character_info::Panel {
        lock(&self.mailbox).properties.character_info.clone()
    }
}
#[cfg(windows)]
pub(in crate::play) fn open_window(link: &PlayLink) {
    use crate::play::feature_0115_mail::view::native::{Form, open};
    open(
        link,
        Form {
            title: "Character information",
            labels: &[
                "Page offset",
                "Page size (1-50)",
                "Visible character UUID",
                "Title ID (0 clears selection)",
            ],
            buttons: &["Self", "Visible", "Look", "Unlock", "Select"],
            snapshot: |mailbox| {
                let panel = &mailbox.properties.character_info;
                (
                    panel.open,
                    panel.pending,
                    panel.fields.to_vec(),
                    oteryn_client_ui::character_info::panel_text(panel),
                )
            },
            set_open: |mailbox, value| mailbox.properties.character_info.open = value,
            submit: |mailbox, commands, action, values| {
                let panel = &mut mailbox.properties.character_info;
                if panel.pending {
                    return;
                }
                for (index, value) in values.into_iter().enumerate() {
                    super::input::edit(panel, index, value);
                }
                let intent = match action {
                    0 => oteryn_client_ui::character_info::Intent::Query,
                    1 => oteryn_client_ui::character_info::Intent::Visible,
                    2 => oteryn_client_ui::character_info::Intent::Look,
                    3 => oteryn_client_ui::character_info::Intent::Unlock,
                    4 => oteryn_client_ui::character_info::Intent::Select,
                    _ => return,
                };
                super::input::submit(mailbox, commands, intent);
            },
        },
    );
}

#[cfg(windows)]
#[path = "view/titles.rs"]
mod titles;
#[cfg(windows)]
pub(in crate::play) use titles::open_titles;
