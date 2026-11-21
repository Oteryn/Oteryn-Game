use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    pub fn house_transfer_panel(&self) -> oteryn_client_ui::house_transfer::Panel {
        lock(&self.mailbox).properties.house_transfer.clone()
    }
}

pub(in crate::play) fn form(_link: &PlayLink) -> crate::play::feature_0115_mail::view::form::Form {
    use crate::play::feature_0115_mail::view::form::Form;
    Form {
        title: "House transfer",
        labels: &["House key"],
        buttons: &["Enter", "Exit"],
        snapshot: |mailbox| {
            let panel = &mailbox.properties.house_transfer;
            (
                panel.open,
                panel.pending,
                panel.fields.to_vec(),
                oteryn_client_ui::house_transfer::panel_text(panel),
            )
        },
        set_open: |mailbox, value| mailbox.properties.house_transfer.open = value,
        submit: |mailbox, commands, action, values| {
            let panel = &mut mailbox.properties.house_transfer;
            if panel.pending {
                return;
            }
            for (index, value) in values.into_iter().enumerate() {
                super::input::edit(panel, index, value);
            }
            let intent = match action {
                0 => oteryn_client_ui::house_transfer::Intent::Enter,
                1 => oteryn_client_ui::house_transfer::Intent::Exit,
                _ => return,
            };
            super::input::submit(mailbox, commands, intent);
        },
    }
}
