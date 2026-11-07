use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    pub fn container_sort_panel(&self) -> oteryn_client_ui::container_sort::Panel {
        lock(&self.mailbox).properties.container_sort.clone()
    }
}
#[cfg(windows)]
pub(in crate::play) fn open_window(link: &PlayLink) {
    use crate::play::feature_0115_mail::view::native::{Form, open};
    open(
        link,
        Form {
            title: "Container sorting",
            labels: &[
                "Open container view (0-15)",
                "Sort key: 1 name, 2 weight, 3 expiry, 4 stack",
                "Descending: 0 no, 1 yes",
                "Recursive: 0 no, 1 yes",
            ],
            buttons: &["Sort"],
            snapshot: |mailbox| {
                let panel = &mailbox.properties.container_sort;
                (
                    panel.open,
                    panel.pending,
                    panel.fields.to_vec(),
                    oteryn_client_ui::container_sort::panel_text(panel),
                )
            },
            set_open: |mailbox, value| mailbox.properties.container_sort.open = value,
            submit: |mailbox, commands, action, values| {
                let panel = &mut mailbox.properties.container_sort;
                if panel.pending {
                    return;
                }
                for (index, value) in values.into_iter().enumerate() {
                    super::input::edit(panel, index, value);
                }
                let intent = match action {
                    0 => oteryn_client_ui::container_sort::Intent::Sort,
                    _ => return,
                };
                super::input::submit(mailbox, commands, intent);
            },
        },
    );
}
