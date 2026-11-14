#[cfg(windows)]
#[path = "catalogue_window.rs"]
mod catalogue_window;
use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    pub fn charm_progression_panel(&self) -> oteryn_client_ui::charm_progression::Panel {
        lock(&self.mailbox).properties.charm_progression.clone()
    }
}
#[cfg(windows)]
#[path = "selector_window.rs"]
mod selector_window;
#[cfg(windows)]
pub(in crate::play) fn open_window(link: &PlayLink) {
    if link.uses_shared_frontend() {
        return;
    }
    catalogue_window::open(link);
    selector_window::open(link);
}

#[cfg(windows)]
pub(in crate::play) fn open_classic_window(link: &PlayLink) {
    if link.uses_shared_frontend() {
        return;
    }
    use crate::play::feature_0115_mail::view::native::{Form, open};
    open(
        link,
        Form {
            title: "Bestiary and Charms",
            labels: &[
                "Charm name",
                "Creature name (browse the creature catalogue)",
            ],
            buttons: &[
                "Query",
                "Unlock",
                "Assign",
                "Remove",
                "ResetAll",
                "All races",
                "In progress",
                "Complete",
                "Previous race",
                "Next race",
                "Previous charm",
                "Next charm",
                "Previous eligible race",
                "Next eligible race",
                "Browse creature catalogue",
                "Next catalogue page",
                "Creature details",
                "Next loot page",
                "Previous creature category",
                "Next creature category",
                "Previous catalogue creature",
                "Next catalogue creature",
                "Load charm names",
                "More charm names",
            ],
            snapshot: |mailbox| {
                let panel = &mailbox.properties.charm_progression;
                (
                    panel.classic_open,
                    panel.pending,
                    oteryn_client_ui::charm_progression::charm_browser::named_fields(panel),
                    format!(
                        "{}\r\n{}",
                        oteryn_client_ui::charm_progression::bestiary_text(panel),
                        oteryn_client_ui::charm_progression::charms_text(panel)
                    ),
                )
            },
            set_open: |mailbox, value| mailbox.properties.charm_progression.classic_open = value,
            submit: |mailbox, commands, action, values| {
                if matches!(action, 22 | 23) {
                    super::input::charm_browser::submit(mailbox, commands, action == 23);
                    return;
                }
                if action >= 14 {
                    for (index, value) in values.into_iter().enumerate() {
                        super::input::edit(&mut mailbox.properties.charm_progression, index, value);
                    }
                    super::input::catalogue::submit(mailbox, commands, action - 14);
                    return;
                }
                let panel = &mut mailbox.properties.charm_progression;
                if panel.pending {
                    return;
                }
                if action >= 5 {
                    use oteryn_client_ui::charm_progression::RaceFilter;
                    match action {
                        5 => panel.race_filter = RaceFilter::All,
                        6 => panel.race_filter = RaceFilter::InProgress,
                        7 => panel.race_filter = RaceFilter::Complete,
                        8 | 9 => panel.select_race(action == 8),
                        10 | 11 => panel.select_charm(action == 10),
                        12 | 13 => panel.select_assignment_race(action == 12),
                        _ => {}
                    }
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

#[cfg(feature = "bevy-shell")]
#[path = "bound.rs"]
mod bound;
#[cfg(all(test, feature = "bevy-shell"))]
#[path = "tests/bound.rs"]
mod bound_tests;
#[cfg(feature = "bevy-shell")]
pub(crate) use bound::{BestiaryBinding, BestiaryControl};
#[cfg(all(test, feature = "bevy-shell"))]
impl crate::play::Outbox {
    pub(crate) fn bevy_bestiary_selected(&self) {
        lock(&self.0).properties.charm_progression.selected = true;
    }
    pub(crate) fn bevy_bestiary_reply(
        &self,
        command: &oteryn_client_runtime::raw_feature::FeatureCommand,
        payload: Vec<u8>,
    ) {
        self.properties_result(
            command,
            &oteryn_client_runtime::raw_feature::GatedCommandResult {
                command_id: 1,
                server_sequence: 1,
                status: oteryn_client_runtime::raw_feature::CommandStatus::Accepted,
                payload,
            },
        );
    }
    pub(crate) fn bevy_bestiary_end(&self) {
        self.end(oteryn_platform_client::native_login::PublicClass::SessionUnavailable);
    }
}
