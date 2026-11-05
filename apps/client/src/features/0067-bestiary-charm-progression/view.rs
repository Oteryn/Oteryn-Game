use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    pub fn charm_progression_panel(&self) -> oteryn_client_ui::charm_progression::Panel {
        lock(&self.mailbox).properties.charm_progression.clone()
    }
}
#[cfg(windows)]
pub(in crate::play) fn open_window(link: &PlayLink) {
    use crate::play::feature_0115_mail::view::native::{Form, open};
    open(
        link,
        Form {
            title: "Bestiary and Charms",
            labels: &["Charm index", "Bestiary race index"],
            buttons: &["Query", "Unlock", "Assign", "Remove", "ResetAll"],
            snapshot: |mailbox| {
                let panel = &mailbox.properties.charm_progression;
                (
                    panel.open,
                    panel.pending,
                    panel.fields.to_vec(),
                    oteryn_client_ui::charm_progression::panel_text(panel),
                )
            },
            set_open: |mailbox, value| mailbox.properties.charm_progression.open = value,
            submit: |mailbox, commands, action, values| {
                let panel = &mut mailbox.properties.charm_progression;
                if panel.pending {
                    return;
                }
                for (index, value) in values.into_iter().enumerate() {
                    super::input::edit(panel, index, value);
                }
                let intent = match action {
                    0 => oteryn_client_ui::charm_progression::Intent::Query,
                    1 => oteryn_client_ui::charm_progression::Intent::Unlock,
                    2 => oteryn_client_ui::charm_progression::Intent::Assign,
                    3 => oteryn_client_ui::charm_progression::Intent::Remove,
                    4 => oteryn_client_ui::charm_progression::Intent::ResetAll,
                    _ => return,
                };
                super::input::submit(mailbox, commands, intent);
            },
        },
    );
}
