//! Shared in-game regions. Modules project the retained client owners into these anchors.
pub(super) use achievement_notices::open as open_achievement_notices;
mod achievement_notices;
pub(super) mod achievements;
mod actions;
pub(super) mod appearance;
pub(super) mod arena;
pub(super) mod boss_difficulty;
mod boss_reward_chest;
pub(super) mod bosstiary;
pub(super) mod cast_chat;
pub(super) mod cast_control;
pub(super) use boss_reward_chest::open as open_boss_reward_chest;
pub(super) mod animus_mastery;
mod bestiary;
pub(super) mod browse_field;
mod chat;
pub(super) mod condition_details;
pub(super) mod cyclopedia_character;
mod daily_rewards;
mod exiva_settings;
pub(super) use exiva_settings::open_exiva_settings;
mod coin_market;
mod container_sorting;
pub(super) mod cyclopedia_map;
pub(super) mod cyclopedia_trackers;
mod depot;
mod exaltation_forge;
pub(super) mod expert_pvp_modes;
mod feature_dialogs;
mod furniture_rotation;
pub(super) mod furniture_wrapping;
pub(super) mod guild;
pub(super) mod hireling_management;
pub(super) mod hireling_services;
pub(super) use furniture_rotation::toggle as toggle_furniture_rotation;
mod guild_wars;
pub(super) mod guildhall_leadership;
pub(super) mod highscores_character_info;
pub(super) mod house_browsing;
pub(super) mod house_move_out;
pub(super) mod house_transfer;
mod house_podium;
pub(super) use house_podium::open as open_house_podium;
mod hud;
mod imbue;
mod inventory;
pub(super) use inventory::ItemPointerCapture;
#[cfg(test)]
pub(super) use inventory::initialize_item_pointer_test_state;
mod item_details;
mod journal;
mod layout;
mod mail;
pub(super) use layout::setup;
mod market;
pub(super) mod minimap;
pub(super) mod modal;
mod npc_shop;
pub(super) mod options;
pub(super) mod party;
pub(super) mod player_report_report;
mod prey;
pub(super) use prey::open as open_prey;
mod projection;
pub(super) use projection::project;
mod quick_loot;
pub(super) mod raid_schedule;
pub(super) mod skills_config;
pub(super) mod staff_tools;
pub(super) mod status_details;
pub(super) mod support_evidence;
mod task_board;
mod team_finder;
pub(super) use team_finder::open as open_team_finder;
pub(super) mod store;
mod trade;
pub(super) mod training;
pub(super) mod viewport;
pub(super) mod weapon_proficiency;
mod wheel;
pub(super) mod world_event_window;
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

pub(super) fn install(app: &mut App) {
    achievement_notices::install(app);
    achievements::install(app);
    actions::install(app);
    animus_mastery::install(app);
    appearance::install(app);
    arena::install(app);
    bestiary::install(app);
    boss_difficulty::install(app);
    boss_reward_chest::install(app);
    bosstiary::install(app);
    browse_field::install(app);
    cast_chat::install(app);
    cast_control::install(app);
    condition_details::install(app);
    cyclopedia_character::install(app);
    cyclopedia_map::install(app);
    cyclopedia_trackers::install(app);
    daily_rewards::install(app);
    exaltation_forge::install(app);
    container_sorting::install(app);
    chat::install(app);
    coin_market::install(app);
    depot::install(app);
    exiva_settings::install(app);
    expert_pvp_modes::install(app);
    feature_dialogs::install(app);
    furniture_rotation::install(app);
    furniture_wrapping::install(app);
    guild::install(app);
    guild_wars::install(app);
    guildhall_leadership::install(app);
    highscores_character_info::install(app);
    hireling_management::install(app);
    hireling_services::install(app);
    house_browsing::install(app);
    house_move_out::install(app);
    house_podium::install(app);
    house_transfer::install(app);
    hud::install(app);
    imbue::install(app);
    inventory::install(app);
    item_details::install(app);
    journal::install(app);
    mail::install(app);
    market::install(app);
    minimap::install(app);
    modal::install(app);
    npc_shop::install(app);
    options::install(app);
    party::install(app);
    player_report_report::install(app);
    prey::install(app);
    quick_loot::install(app);
    raid_schedule::install(app);
    skills_config::install(app);
    staff_tools::install(app);
    status_details::install(app);
    store::install(app);
    support_evidence::install(app);
    task_board::install(app);
    team_finder::install(app);
    trade::install(app);
    training::install(app);
    weapon_proficiency::install(app);
    wheel::install(app);
    world_event_window::install(app);
    app.add_systems(
        Update,
        (
            projection::missing_item_appearances,
            projection::project_regions.after(project),
        ),
    );
}

#[cfg(test)]
mod native_tests;

pub(super) fn open_container_sorting(owner: &mut HostOwner) {
    container_sorting::open(owner);
}

pub(super) fn open_quests(owner: &mut HostOwner) {
    journal::open_quests(owner);
}
pub(super) fn open_spells(owner: &mut HostOwner) {
    journal::open_spells(owner);
}

pub(super) fn open_guild_wars(owner: &HostOwner) {
    guild_wars::open(owner);
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
