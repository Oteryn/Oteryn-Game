//! Atomic presentation binding; all wire requests remain with the existing owners.
use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::cyclopedia_map::{Intent, Panel, atlas_action};
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MapControl {
    Close,
    Request(Intent),
    Atlas(usize),
    Geometry(usize),
    Cell(usize),
    Donation(String),
    Floor(bool),
    Pois,
    Analytics(usize),
}
impl MapControl {
    pub fn needs_confirmation(&self) -> bool {
        matches!(
            self,
            Self::Request(Intent::Donate | Intent::Shuffle | Intent::Start)
        )
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MapSnapshot {
    pub panel: Panel,
    pub text: String,
    pub analytics: String,
}
fn snapshot(mailbox: &crate::play::mailbox::Mailbox) -> MapSnapshot {
    let panel = mailbox.properties.cyclopedia_map.clone();
    MapSnapshot {
        text: oteryn_client_ui::cyclopedia_map::panel_text(&panel),
        panel,
        analytics: format!(
            "Selected tool: {}\r\n{}",
            mailbox.analyser.tool.title(),
            oteryn_client_ui::analyser::panel_text(&mailbox.analyser)
        ),
    }
}
impl PlayLink {
    pub fn cyclopedia_map_snapshot(&self) -> MapSnapshot {
        snapshot(&lock(&self.mailbox))
    }
    pub fn cyclopedia_map_toggle_bound(&self) -> bool {
        let mut mailbox = lock(&self.mailbox);
        if self.commands.is_closed()
            || mailbox.world_input_blocked()
            || mailbox.server_modal.is_active()
        {
            return false;
        }
        let panel = &mut mailbox.properties.cyclopedia_map;
        panel.open = !panel.open;
        if !panel.open {
            panel.geometry.open = false;
            panel.atlas_open = false;
        }
        panel.geometry.revision = panel.geometry.revision.wrapping_add(1);
        true
    }
    pub fn cyclopedia_map_dispatch_bound(
        &self,
        expected: &MapSnapshot,
        control: MapControl,
        confirmed: bool,
    ) -> bool {
        let mut mailbox = lock(&self.mailbox);
        if self.commands.is_closed()
            || mailbox.world_input_blocked()
            || mailbox.server_modal.is_active()
            || !std::sync::Arc::ptr_eq(
                &expected.panel.geometry.lifetime,
                &mailbox.properties.cyclopedia_map.geometry.lifetime,
            )
            || snapshot(&mailbox) != *expected
            || !expected.panel.open
            || (control.needs_confirmation() && !confirmed)
        {
            return false;
        }
        let panel = &mut mailbox.properties.cyclopedia_map;
        if control == MapControl::Close {
            panel.open = false;
            panel.geometry.open = false;
            panel.atlas_open = false;
            panel.geometry.revision = panel.geometry.revision.wrapping_add(1);
            return true;
        }
        if !panel.selected || panel.pending || panel.speed_pending {
            return false;
        }
        match control {
            MapControl::Close => return false,
            MapControl::Request(intent) => {
                if super::super::protocol::command(panel, intent).is_err() {
                    return false;
                }
                super::super::input::submit(&mut mailbox, &self.commands, intent);
            }
            MapControl::Atlas(action) => {
                if action > 15 {
                    return false;
                }
                if atlas_action(panel, action) {
                    super::super::input::submit(&mut mailbox, &self.commands, Intent::Atlas);
                }
            }
            MapControl::Geometry(action) => {
                if action > 13 || action == 11 {
                    return false;
                }
                panel.geometry.open = true;
                super::super::input::atlas::submit(&mut mailbox, &self.commands, action);
            }
            MapControl::Cell(cell) => {
                if panel.atlas.as_ref().is_none_or(|a| cell >= a.cells.len()) {
                    return false;
                }
                panel.atlas_cell = cell;
            }
            MapControl::Donation(value) => {
                if value.len() > 64 || value.contains('\0') {
                    return false;
                }
                super::super::input::edit(panel, 0, value);
            }
            MapControl::Floor(previous) => panel.select_floor(previous),
            MapControl::Pois => panel.hide_pois = !panel.hide_pois,
            MapControl::Analytics(index) => {
                let Some(tool) = oteryn_client_ui::analyser::Tool::CYCLOPEDIA.get(index) else {
                    return false;
                };
                mailbox.analyser.tool = *tool;
            }
        }
        mailbox.properties.cyclopedia_map.geometry.revision = mailbox
            .properties
            .cyclopedia_map
            .geometry
            .revision
            .wrapping_add(1);
        true
    }
}

#[cfg(test)]
impl crate::play::Outbox {
    pub(crate) fn bevy_cyclopedia_map_select(&self) {
        lock(&self.0).properties.cyclopedia_map.selected = true;
    }
    pub(crate) fn bevy_cyclopedia_map_reply(
        &self,
        command: &oteryn_client_runtime::raw_feature::FeatureCommand,
        reply: &oteryn_client_runtime::raw_feature::GatedCommandResult,
    ) {
        self.properties_result(command, reply);
    }
}
