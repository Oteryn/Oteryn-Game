//! Shared in-game regions. Modules project the retained client owners into these anchors.
mod actions;
pub(super) mod cast_chat;
pub(super) mod cast_control;
pub(super) mod boss_difficulty;
mod boss_reward_chest;
pub(super) use boss_reward_chest::open as open_boss_reward_chest;
pub(super) mod animus_mastery;
pub(super) mod browse_field;
mod bestiary;
mod chat;
mod exiva_settings;
pub(super) use exiva_settings::open_exiva_settings;
pub(super) mod cyclopedia_trackers;
pub(super) mod cyclopedia_map;
mod exaltation_forge;
mod container_sorting;
mod coin_market;
mod depot;
mod feature_dialogs;
pub(super) mod hireling_services;
mod furniture_rotation;
pub(super) use furniture_rotation::toggle as toggle_furniture_rotation;
pub(super) mod house_move_out;
pub(super) mod house_transfer;
pub(super) mod house_browsing;
pub(super) mod guildhall_leadership;
mod guild_wars;
pub(super) mod highscores_character_info;
mod hud;
mod imbue;
mod inventory;
pub(super) use inventory::ItemPointerCapture;
#[cfg(test)]
pub(super) use inventory::initialize_item_pointer_test_state;
mod item_details;
mod journal;
mod market;
pub(super) mod minimap;
pub(super) mod modal;
mod npc_shop;
pub(super) mod options;
pub(super) mod party;
pub(super) mod player_report_report;
mod projection;
pub(super) use projection::project;
mod quick_loot;
pub(super) mod staff_tools;
mod team_finder;
pub(super) use team_finder::open as open_team_finder;
pub(super) mod store;
mod trade;
pub(super) mod training;
pub(super) mod viewport;
pub(super) mod world_event_window;
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

pub(super) fn setup(mut commands: Commands, skins: Res<Skins>) {
    let root = commands
        .spawn((
            GameRoot,
            bevy::input_focus::tab_navigation::TabGroup::new(0),
            Node {
                position_type: PositionType::Absolute,
                width: percent(100.),
                height: percent(100.),
                flex_direction: FlexDirection::Column,
                display: Display::None,
                ..default()
            },
            GlobalZIndex(0),
        ))
        .id();
    let vitals = commands
        .spawn(Node {
            width: percent(100.),
            min_height: px(28.),
            flex_shrink: 0.,
            ..default()
        })
        .id();
    let middle = commands
        .spawn(Node {
            width: percent(100.),
            flex_grow: 1.,
            min_height: px(0.),
            ..default()
        })
        .id();
    let main = commands
        .spawn(Node {
            flex_grow: 1.,
            min_width: px(0.),
            min_height: px(0.),
            flex_direction: FlexDirection::Column,
            ..default()
        })
        .id();
    let world = commands
        .spawn((
            WorldRegion,
            Node {
                width: percent(100.),
                min_height: px(0.),
                flex_grow: 1.,
                overflow: Overflow::clip(),
                ..default()
            },
        ))
        .id();
    let bars = commands
        .spawn((
            Node {
                width: percent(100.),
                min_height: px(36.),
                max_height: px(156.),
                flex_shrink: 0.,
                overflow: Overflow::clip(),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            BackgroundColor(Color::srgb(0.12, 0.12, 0.12)),
        ))
        .id();
    let chat = commands
        .spawn((
            source::frame(skins.inset.clone(), [1.; 4]),
            Node {
                width: percent(100.),
                height: px(150.),
                min_height: px(90.),
                max_height: percent(30.),
                flex_shrink: 0.,
                padding: UiRect::all(px(2.)),
                flex_direction: FlexDirection::Column,
                overflow: Overflow::clip(),
                ..default()
            },
        ))
        .id();
    let panels = commands
        .spawn((
            Node {
                width: px(176.),
                min_width: px(176.),
                height: percent(100.),
                flex_shrink: 0.,
                flex_direction: FlexDirection::Column,
                overflow: Overflow::scroll_y(),
                ..default()
            },
            bevy::ui_widgets::ScrollArea,
            BackgroundColor(Color::srgb(0.18, 0.18, 0.18)),
        ))
        .id();
    let inventory = commands
        .spawn((
            Node {
                width: px(176.),
                min_width: px(176.),
                height: percent(100.),
                flex_shrink: 0.,
                flex_direction: FlexDirection::Column,
                overflow: Overflow::scroll_y(),
                ..default()
            },
            bevy::ui_widgets::ScrollArea,
            BackgroundColor(Color::srgb(0.18, 0.18, 0.18)),
        ))
        .id();
    let toolbar = commands
        .spawn((
            ToolbarRegion,
            Node {
                width: percent(100.),
                min_height: px(24.),
                flex_shrink: 0.,
                flex_direction: FlexDirection::Column,
                ..default()
            },
        ))
        .id();
    let minimap = region(&mut commands);
    let equipment = region(&mut commands);
    let containers = region(&mut commands);
    let trade_dock = region(&mut commands);
    commands.entity(trade_dock).insert(trade::TradeDock);
    let battle = region(&mut commands);
    let party_dock = region(&mut commands);
    commands.entity(party_dock).insert(party::PartyDock);
    let skills = region(&mut commands);
    let vip = region(&mut commands);
    let analyzers = region(&mut commands);
    commands.entity(root).add_children(&[vitals, middle]);
    commands
        .entity(middle)
        .add_children(&[main, panels, inventory]);
    commands.entity(main).add_children(&[world, bars, chat]);
    commands
        .entity(panels)
        .add_children(&[battle, party_dock, skills, vip, analyzers]);
    commands
        .entity(inventory)
        .add_children(&[toolbar, minimap, equipment, trade_dock, containers]);
    commands.insert_resource(GameRegions {
        vitals,
        minimap,
        equipment,
        containers,
        battle,
        skills,
        vip,
        analyzers,
        chat,
        bars,
        world,
    });
}
fn region(commands: &mut Commands) -> Entity {
    commands
        .spawn(Node {
            width: percent(100.),
            flex_shrink: 0.,
            flex_direction: FlexDirection::Column,
            ..default()
        })
        .id()
}

pub(super) fn install(app: &mut App) {
    cyclopedia_map::install(app);
    cyclopedia_trackers::install(app);
    cast_control::install(app);
    cast_chat::install(app);
    actions::install(app);
    exiva_settings::install(app);
    boss_difficulty::install(app);
    boss_reward_chest::install(app);
    animus_mastery::install(app);
    browse_field::install(app);
    bestiary::install(app);
    exaltation_forge::install(app);
    container_sorting::install(app);
    chat::install(app);
    coin_market::install(app);
    depot::install(app);
    feature_dialogs::install(app);
    hireling_services::install(app);
    furniture_rotation::install(app);
    house_move_out::install(app);
    house_transfer::install(app);
    house_browsing::install(app);
    guildhall_leadership::install(app);
    guild_wars::install(app);
    highscores_character_info::install(app);
    hud::install(app);
    imbue::install(app);
    inventory::install(app);
    item_details::install(app);
    journal::install(app);
    market::install(app);
    minimap::install(app);
    modal::install(app);
    npc_shop::install(app);
    options::install(app);
    party::install(app);
    player_report_report::install(app);
    quick_loot::install(app);
    staff_tools::install(app);
    team_finder::install(app);
    store::install(app);
    trade::install(app);
    world_event_window::install(app);
    training::install(app);
    wheel::install(app);
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
