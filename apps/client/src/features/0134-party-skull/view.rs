use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    pub fn party_skull_panel(&self) -> oteryn_client_ui::party_skull::Panel {
        lock(&self.mailbox).properties.party_skull.clone()
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
            title: "Party shields and skulls",
            labels: &[],
            buttons: &["Party icons", "Unjustified points"],
            snapshot: |mailbox| {
                let panel = &mailbox.properties.party_skull;
                (
                    panel.open,
                    panel.pending,
                    panel.fields.to_vec(),
                    oteryn_client_ui::party_skull::panel_text(panel),
                )
            },
            set_open: |mailbox, value| mailbox.properties.party_skull.open = value,
            submit: |mailbox, commands, action, values| {
                let panel = &mut mailbox.properties.party_skull;
                if panel.pending {
                    return;
                }
                for (index, value) in values.into_iter().enumerate() {
                    super::input::edit(panel, index, value);
                }
                let intent = match action {
                    0 => oteryn_client_ui::party_skull::Intent::Query,
                    1 => oteryn_client_ui::party_skull::Intent::PvpStatus,
                    _ => return,
                };
                super::input::submit(mailbox, commands, intent);
            },
        },
    );
}

#[path = "pvp_status.rs"]
pub(in crate::play) mod pvp_status;
