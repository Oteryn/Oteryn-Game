use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    pub fn raid_schedule_panel(&self) -> oteryn_client_ui::raid_schedule::Panel {
        lock(&self.mailbox).properties.raid_schedule.clone()
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
            title: "Raid schedule",
            labels: &[],
            buttons: &["Refresh"],
            snapshot: |mailbox| {
                let panel = &mailbox.properties.raid_schedule;
                (
                    panel.open,
                    panel.pending,
                    panel.fields.to_vec(),
                    oteryn_client_ui::raid_schedule::panel_text(panel),
                )
            },
            set_open: |mailbox, value| mailbox.properties.raid_schedule.open = value,
            submit: |mailbox, commands, action, values| {
                let panel = &mut mailbox.properties.raid_schedule;
                if panel.pending {
                    return;
                }
                for (index, value) in values.into_iter().enumerate() {
                    super::input::edit(panel, index, value);
                }
                let intent = match action {
                    0 => oteryn_client_ui::raid_schedule::Intent::Query,
                    _ => return,
                };
                super::input::submit(mailbox, commands, intent);
            },
        },
    );
}
