use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    pub fn pvp_expert_panel(&self) -> oteryn_client_ui::pvp_expert::Panel {
        lock(&self.mailbox).properties.pvp_expert.clone()
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
            title: "PvP expert",
            labels: &[
                "Fight: 1 offensive, 2 balanced, 3 defensive",
                "Chase: 1 stand, 2 chase",
                "Secure: 0 off, 1 on",
                "Expert: 0 dove, 1 white hand, 2 yellow hand, 3 red fist",
            ],
            buttons: &["Apply"],
            snapshot: |mailbox| {
                let panel = &mailbox.properties.pvp_expert;
                (
                    panel.open,
                    panel.pending,
                    panel.fields.to_vec(),
                    oteryn_client_ui::pvp_expert::panel_text(panel),
                )
            },
            set_open: |mailbox, value| mailbox.properties.pvp_expert.open = value,
            submit: |mailbox, commands, action, values| {
                let panel = &mut mailbox.properties.pvp_expert;
                if panel.pending {
                    return;
                }
                for (index, value) in values.into_iter().enumerate() {
                    super::input::edit(panel, index, value);
                }
                let intent = match action {
                    0 => oteryn_client_ui::pvp_expert::Intent::Apply,
                    _ => return,
                };
                super::input::submit(mailbox, commands, intent);
            },
        },
    );
}
