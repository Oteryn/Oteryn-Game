use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    pub fn guildhall_panel(&self) -> oteryn_client_ui::guildhall::Panel {
        lock(&self.mailbox).properties.guildhall.clone()
    }
}
#[cfg(windows)]
pub(in crate::play) fn open_window(link: &PlayLink) {
    use crate::play::feature_0115_mail::view::native::{Form, open};
    open(
        link,
        Form {
            title: "Guildhall leadership",
            labels: &[
                "Guild UUID",
                "Successor character UUID",
                "Expected guild revision",
            ],
            buttons: &["Transfer"],
            snapshot: |mailbox| {
                let panel = &mailbox.properties.guildhall;
                (
                    panel.open,
                    panel.pending,
                    panel.fields.to_vec(),
                    oteryn_client_ui::guildhall::panel_text(panel),
                )
            },
            set_open: |mailbox, value| mailbox.properties.guildhall.open = value,
            submit: |mailbox, commands, action, values| {
                let panel = &mut mailbox.properties.guildhall;
                if panel.pending {
                    return;
                }
                for (index, value) in values.into_iter().enumerate() {
                    super::input::edit(panel, index, value);
                }
                let intent = match action {
                    0 => oteryn_client_ui::guildhall::Intent::Transfer,
                    _ => return,
                };
                super::input::submit(mailbox, commands, intent);
            },
        },
    );
}
