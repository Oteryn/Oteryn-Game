//! Shared in-game region layout.
use super::{GameRegions, GameRoot, ToolbarRegion, WorldRegion, party, trade};
use crate::bevy_shell::ui::Skins;
use bevy::prelude::*;
use oteryn_client_ui_bevy as source;

pub(in crate::bevy_shell) fn setup(mut commands: Commands, skins: Res<Skins>) {
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
