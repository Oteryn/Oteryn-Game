use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    pub fn house_podium_panel(&self) -> oteryn_client_ui::house_podium::Panel {
        lock(&self.mailbox).properties.house_podium.clone()
    }
}
#[cfg(windows)]
pub(in crate::play) fn open_window(link: &PlayLink) {
    use crate::play::feature_0115_mail::view::native::{Form, open};
    open(
        link,
        Form {
            title: "House podium",
            labels: &[
                "Podium item UUID",
                "Display: 1 outfit, 2 mount, 3 item",
                "Direction: 0 north, 1 east, 2 south, 3 west",
                "Display content key",
            ],
            buttons: &["Refresh", "Projection", "Set"],
            snapshot: |mailbox| {
                let panel = &mailbox.properties.house_podium;
                (
                    panel.open,
                    panel.pending,
                    panel.fields.to_vec(),
                    oteryn_client_ui::house_podium::panel_text(panel),
                )
            },
            set_open: |mailbox, value| mailbox.properties.house_podium.open = value,
            submit: |mailbox, commands, action, values| {
                let panel = &mut mailbox.properties.house_podium;
                if panel.pending {
                    return;
                }
                for (index, value) in values.into_iter().enumerate() {
                    super::input::edit(panel, index, value);
                }
                let intent = match action {
                    0 => oteryn_client_ui::house_podium::Intent::Refresh,
                    1 => oteryn_client_ui::house_podium::Intent::Projection,
                    2 => oteryn_client_ui::house_podium::Intent::Set,
                    _ => return,
                };
                super::input::submit(mailbox, commands, intent);
            },
        },
    );
}
