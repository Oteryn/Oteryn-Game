use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    pub fn appearance_panel(&self) -> oteryn_client_ui::appearance::Panel {
        lock(&self.mailbox).properties.appearance.clone()
    }
}
#[cfg(windows)]
pub(in crate::play) fn open_window(link: &PlayLink) {
    use self::native::{Form, open};
    open(
        link,
        Form {
            title: "Appearance",
            labels: &[
                "Outfit key (see catalogue)",
                "Addon mask",
                "Mount key (empty for none)",
                "Colours: head,body,legs,feet",
                "Content revision",
            ],
            buttons: &[
                "Apply",
                "Previous outfit",
                "Next outfit",
                "Head colour",
                "Body colour",
                "Legs colour",
                "Feet colour",
                "Addon 1",
                "Addon 2",
            ],
            snapshot: |mailbox| {
                let panel = &mailbox.properties.appearance;
                (
                    panel.open,
                    panel.pending,
                    panel.fields.to_vec(),
                    oteryn_client_ui::appearance::panel_text(panel),
                )
            },
            set_open: |mailbox, value| mailbox.properties.appearance.open = value,
            submit: |mailbox, commands, action, values| {
                let panel = &mut mailbox.properties.appearance;
                if panel.pending {
                    return;
                }
                for (index, value) in values.into_iter().enumerate() {
                    super::input::edit(panel, index, value);
                }
                if matches!(action, 1 | 2) {
                    panel.cycle_outfit(action == 2);
                    return;
                }
                if matches!(action, 3..=6) {
                    panel.cycle_colour(action - 3);
                    return;
                }
                if matches!(action, 7 | 8) {
                    panel.toggle_addon(if action == 7 { 1 } else { 2 });
                    return;
                }
                let intent = match action {
                    0 => oteryn_client_ui::appearance::Intent::Apply,
                    _ => return,
                };
                super::input::submit(mailbox, commands, intent);
            },
        },
    );
}

#[cfg(windows)]
#[path = "native.rs"]
mod native;
#[cfg(any(windows, test))]
#[path = "preview.rs"]
pub(in crate::play) mod preview;
