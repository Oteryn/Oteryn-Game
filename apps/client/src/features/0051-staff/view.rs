use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    pub fn staff_panel(&self) -> super::state::Panel {
        lock(&self.mailbox).properties.staff.clone()
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
            title: "Staff authorized actions",
            labels: &[
                "Operator",
                "Authorized action (exact signed text)",
                "Authorization signature (hex)",
            ],
            buttons: &["Submit action", "Store signed receipt"],
            snapshot: |mailbox| {
                let panel = &mailbox.properties.staff;
                (
                    panel.open,
                    panel.pending,
                    panel.fields.to_vec(),
                    lfmt!(
                        "First authorize self-inspection to activate your staff role.\nInspect, ban/unban, teleport/to player, bring, notify/broadcast, ghost/invisible, mute/unmute, kick.\n{}",
                        oteryn_client_ui::staff::panel_text(&panel.summary, &panel.feedback)
                    ),
                )
            },
            set_open: |mailbox, value| mailbox.properties.staff.open = value,
            submit: |mailbox, commands, action, values| {
                if mailbox.properties.staff.pending {
                    return;
                }
                for (index, value) in values.into_iter().enumerate() {
                    super::input::edit(&mut mailbox.properties.staff, index, value);
                }
                match action {
                    0 => super::input::submit(mailbox, commands, false),
                    1 => super::input::submit(mailbox, commands, true),
                    _ => {}
                }
            },
        },
    );
}

#[cfg(feature = "bevy-shell")]
#[path = "bound.rs"]
mod bound;
#[cfg(feature = "bevy-shell")]
pub use bound::StaffToolsSnapshot;
