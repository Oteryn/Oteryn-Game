use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    pub fn appearance_panel(&self) -> oteryn_client_ui::appearance::Panel {
        lock(&self.mailbox).properties.appearance.clone()
    }
}
#[cfg(windows)]
pub(in crate::play) fn open_window(link: &PlayLink) {
    if link.uses_shared_frontend() {
        return;
    }
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
                "Previous mount",
                "Next mount",
                "Outfit Memorial",
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
                if action == 11 {
                    // Open from the appearance UI; the worker is started after releasing the mailbox.
                    mailbox.properties.appearance.memorial.requested = true;
                    return;
                }
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
                if matches!(action, 9 | 10) {
                    panel.cycle_mount(action == 10);
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
    preview_window::open(link);
    memorial::watch(link);
}

#[cfg(any(windows, test, feature = "bevy-shell"))]
#[path = "view/preview.rs"]
pub(in crate::play) mod gallery_preview;

#[cfg(windows)]
#[path = "view/preview_window.rs"]
mod preview_window;

#[path = "view/memorial.rs"]
pub(in crate::play) mod memorial;

#[cfg(windows)]
#[path = "native.rs"]
mod native;
#[cfg(any(windows, test))]
#[path = "preview.rs"]
pub(in crate::play) mod preview;

#[cfg(all(test, feature = "bevy-shell"))]
#[path = "tests/bevy_window.rs"]
pub(crate) mod bevy_window_tests;

#[cfg(feature = "bevy-shell")]
#[path = "view/bound.rs"]
mod bound;
#[cfg(feature = "bevy-shell")]
pub use bound::{AppearanceControl, AppearancePreview, AppearanceSnapshot};

#[cfg(all(test, feature = "bevy-shell"))]
impl crate::play::Outbox {
    pub(crate) fn bevy_appearance_end(&self) {
        self.end(oteryn_platform_client::native_login::PublicClass::SessionUnavailable);
    }
    pub(crate) fn bevy_appearance_reply(
        &self,
        command: &oteryn_client_runtime::raw_feature::FeatureCommand,
    ) {
        use oteryn_client_runtime::{
            appearance as wire,
            raw_feature::{CommandStatus, GatedCommandResult},
        };
        let result = GatedCommandResult {
            command_id: 1,
            server_sequence: 1,
            status: CommandStatus::Accepted,
            payload: wire::ChangeResult::Committed(8)
                .encode()
                .unwrap_or_default(),
        };
        super::protocol::apply(&mut lock(&self.0).properties.appearance, command, &result);
    }
}
