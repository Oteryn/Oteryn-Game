//! Shared in-game regions. Modules project the retained client owners into these anchors.
mod actions;
mod analyser;
mod bestiary;
mod bosstiary;
mod bug_report;
mod chat;
mod cyclopedia;
mod cyclopedia_items;
mod cyclopedia_map;
#[cfg(test)]
#[path = "cyclopedia/tests.rs"]
mod cyclopedia_tests;
mod cyclopedia_tracker;
pub(super) use cyclopedia_tracker::toggle as toggle_cyclopedia_tracker;
#[path = "hud/art.rs"]
pub(in crate::bevy_shell) mod art;
mod depot;
mod feature_dialogs;
mod feature_forms;
mod forge;
pub(super) mod highscores;
mod hotkeys;
mod hud;
pub(in crate::bevy_shell) mod hud_panes;
mod imbuement_tracker;
mod imbuing;
mod install;
pub(super) use install::install;
mod inventory;
pub(super) use inventory::ItemPointerCapture;
#[cfg(test)]
pub(super) use inventory::initialize_item_pointer_test_state;
pub(super) mod item_details;
mod journal;
pub(super) mod layout;
mod market;
pub(super) mod minimap;
pub(super) mod modal;
mod npc_shop;
pub(super) mod options;
mod outfit;
pub(super) mod party;
mod prey;
mod proficiency;
mod projection;
pub(super) use projection::project;
mod quick_loot;
mod reward_wall;
pub(super) mod rule_violation;
pub(super) mod satellite_map;
mod stash;
mod store;
mod store_inbox;
mod task_board;
mod task_tracker;
mod team_finder;
mod trade;
mod tutorial;
pub(super) mod unjustified_points;
pub(super) mod viewport;
mod wheel;
use super::{owner::HostOwner, ui::Skins};
use bevy::prelude::*;
use oteryn_client_ui_bevy as source;

#[derive(Resource)]
pub(super) struct GameRegions {
    pub vitals: Entity,
    pub minimap: Entity,
    pub equipment: Entity,
    pub containers: Entity,
    pub battle: Entity,
    pub skills: Entity,
    pub vip: Entity,
    pub analyzers: Entity,
    pub chat: Entity,
    pub bars: Entity,
    pub world: Entity,
}
/// The centre-column region over the game window that holds the Tibia status bar.
#[derive(Resource, Clone, Copy)]
pub(super) struct StatusBar(pub Entity);
#[derive(Component)]
pub(super) struct GameModal;
#[derive(Component, Clone, Copy)]
pub(super) struct ItemDragSource {
    pub generation: u64,
    pub item: oteryn_session::ItemEntry,
}
#[derive(Component)]
pub(super) struct ItemIcon(pub oteryn_session::ItemEntry);
#[derive(Component)]
pub(super) struct GameRoot;
#[derive(Component)]
pub(super) struct WorldRegion;
#[derive(Component)]
pub(in crate::bevy_shell) struct ToolbarRegion;

/// The sidebar columns and panes the widget chrome docks into.
#[derive(Resource, Clone, Copy)]
pub(super) struct Sidebar {
    /// The optional left sidebar and the optional second right sidebar (hidden by default).
    pub left: Entity,
    pub extra: Entity,
    /// The dockable widget pane of each column, top to bottom, ending in a fill pane.
    pub left_widgets: Entity,
    pub extra_widgets: Entity,
    pub right_widgets: Entity,
    /// The content of the analytics selector widget.
    pub analytics: Entity,
}
/// Tibia Global sidebar width; the sidebar cannot be resized.
pub(super) const SIDEBAR_WIDTH: f32 = 176.;
/// Tibia Global panel pane margins (PanelPane.qml): horizontal 6, vertical 4.
pub(super) const PANEL_MARGIN: [f32; 2] = [6., 4.];

mod regions;
pub(super) use regions::setup;

#[cfg(test)]
mod native_tests;

pub(super) fn open_quests(owner: &mut HostOwner) {
    journal::open_quests(owner);
}
pub(super) fn open_spells(owner: &mut HostOwner) {
    journal::open_spells(owner);
}

pub(super) fn open_market(owner: &mut HostOwner) {
    market::open(owner);
}
pub(super) fn open_party(owner: &mut HostOwner) {
    if !owner.modal_active()
        && let Some(play) = &owner.play
    {
        play.link.open_party();
    }
}
