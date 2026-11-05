use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    pub fn cyclopedia_map_panel(&self) -> oteryn_client_ui::cyclopedia_map::Panel {
        lock(&self.mailbox).properties.cyclopedia_map.clone()
    }
}
#[cfg(windows)]
pub(in crate::play) fn open_window(link: &PlayLink) {
    use crate::play::feature_0115_mail::view::native::{Form, open};
    open(
        link,
        Form {
            title: "Cyclopedia map",
            labels: &["Donation gold (1-10000000)"],
            buttons: &["Query", "Start", "Shuffle", "Find", "Donate"],
            snapshot: |mailbox| {
                let panel = &mailbox.properties.cyclopedia_map;
                (
                    panel.open,
                    panel.pending,
                    panel.fields.to_vec(),
                    oteryn_client_ui::cyclopedia_map::panel_text(panel),
                )
            },
            set_open: |mailbox, value| mailbox.properties.cyclopedia_map.open = value,
            submit: |mailbox, commands, action, values| {
                let panel = &mut mailbox.properties.cyclopedia_map;
                if panel.pending {
                    return;
                }
                for (index, value) in values.into_iter().enumerate() {
                    super::input::edit(panel, index, value);
                }
                let intent = match action {
                    0 => oteryn_client_ui::cyclopedia_map::Intent::Query,
                    1 => oteryn_client_ui::cyclopedia_map::Intent::Start,
                    2 => oteryn_client_ui::cyclopedia_map::Intent::Shuffle,
                    3 => oteryn_client_ui::cyclopedia_map::Intent::Find,
                    4 => oteryn_client_ui::cyclopedia_map::Intent::Donate,
                    _ => return,
                };
                super::input::submit(mailbox, commands, intent);
            },
        },
    );
}
