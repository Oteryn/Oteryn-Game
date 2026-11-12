use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    pub fn training_panel(&self) -> oteryn_client_ui::training::Panel {
        lock(&self.mailbox).properties.training.clone()
    }
}
#[cfg(windows)]
pub(in crate::play) fn open_window(link: &PlayLink) {
    use crate::play::feature_0115_mail::view::native::{Form, open};
    open(
        link,
        Form {
            title: "Training",
            labels: &[
                "Offline statue handle",
                "Exercise weapon handle",
                "Exercise dummy handle",
            ],
            buttons: &["Offline", "Start", "Status", "Stop"],
            snapshot: |mailbox| {
                let panel = &mailbox.properties.training;
                (
                    panel.open,
                    panel.pending,
                    panel.fields.to_vec(),
                    oteryn_client_ui::training::panel_text(panel),
                )
            },
            set_open: |mailbox, value| mailbox.properties.training.open = value,
            submit: |mailbox, commands, action, values| {
                let panel = &mut mailbox.properties.training;
                if panel.pending {
                    return;
                }
                for (index, value) in values.into_iter().enumerate() {
                    super::input::edit(panel, index, value);
                }
                let intent = match action {
                    0 => oteryn_client_ui::training::Intent::Offline,
                    1 => oteryn_client_ui::training::Intent::Start,
                    2 => oteryn_client_ui::training::Intent::Status,
                    3 => oteryn_client_ui::training::Intent::Stop,
                    _ => return,
                };
                super::input::submit(mailbox, commands, intent);
            },
        },
    );
}
