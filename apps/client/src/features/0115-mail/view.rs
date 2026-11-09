use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    pub fn mail_panel(&self) -> oteryn_client_ui::mail::Panel {
        lock(&self.mailbox).properties.mail.clone()
    }
}
#[cfg(windows)]
pub(in crate::play) fn open_window(link: &PlayLink) {
    use crate::play::feature_0115_mail::view::native::{Form, open};
    open(
        link,
        Form {
            title: "Writing and mail",
            labels: &[
                "Selected item",
                "Selected mailbox (for posting)",
                "Text (include recipient address for mail)",
            ],
            buttons: &["Write", "Post"],
            snapshot: |mailbox| {
                let panel = &mailbox.properties.mail;
                (
                    panel.open,
                    panel.pending,
                    panel.fields.to_vec(),
                    oteryn_client_ui::mail::panel_text(panel),
                )
            },
            set_open: |mailbox, value| mailbox.properties.mail.open = value,
            submit: |mailbox, commands, action, values| {
                let panel = &mut mailbox.properties.mail;
                if panel.pending {
                    return;
                }
                for (index, value) in values.into_iter().enumerate() {
                    super::input::edit(panel, index, value);
                }
                let intent = match action {
                    0 => oteryn_client_ui::mail::Intent::Write,
                    1 => oteryn_client_ui::mail::Intent::Post,
                    _ => return,
                };
                super::input::submit(mailbox, commands, intent);
            },
        },
    );
}

#[cfg(windows)]
#[path = "native.rs"]
pub(in crate::play) mod native;
