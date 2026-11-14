use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    pub fn timed_item_panel(&self) -> oteryn_client_ui::timed_item::Panel {
        lock(&self.mailbox).properties.timed_item.clone()
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
            title: "Timed items",
            labels: &[
                "Item handle",
                "Expected revision",
                "Aldo entity UUID (repair only)",
                "Aldo generation (repair only)",
            ],
            buttons: &["Inspect", "Use", "Repair"],
            snapshot: |mailbox| {
                let panel = &mailbox.properties.timed_item;
                (
                    panel.open,
                    panel.pending,
                    panel.fields.to_vec(),
                    oteryn_client_ui::timed_item::panel_text(panel),
                )
            },
            set_open: |mailbox, value| mailbox.properties.timed_item.open = value,
            submit: |mailbox, commands, action, values| {
                let panel = &mut mailbox.properties.timed_item;
                if panel.pending {
                    return;
                }
                for (index, value) in values.into_iter().enumerate() {
                    super::input::edit(panel, index, value);
                }
                let intent = match action {
                    0 => oteryn_client_ui::timed_item::Intent::Inspect,
                    1 => oteryn_client_ui::timed_item::Intent::Use,
                    2 => oteryn_client_ui::timed_item::Intent::Repair,
                    _ => return,
                };
                super::input::submit(mailbox, commands, intent);
            },
        },
    );
}
