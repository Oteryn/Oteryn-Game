//! Shared in-game regions. Modules project the retained client owners into these anchors.
mod actions;
mod chat;
mod feature_dialogs;
mod hud;
mod inventory;
mod journal;
pub(super) mod minimap;
pub(super) mod modal;
pub(super) mod options;
pub(super) mod viewport;
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
    let battle = region(&mut commands);
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
        .add_children(&[battle, skills, vip, analyzers]);
    commands
        .entity(inventory)
        .add_children(&[toolbar, minimap, equipment, containers]);
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
pub(super) fn project(owner: NonSend<HostOwner>, mut roots: Query<&mut Node, With<GameRoot>>) {
    let display = if owner.play.is_some() {
        Display::Flex
    } else {
        Display::None
    };
    for mut root in &mut roots {
        if root.display != display {
            root.display = display;
        }
    }
}

pub(super) fn install(app: &mut App) {
    modal::install(app);
    options::install(app);
    journal::install(app);
    chat::install(app);
    actions::install(app);
    feature_dialogs::install(app);
    hud::install(app);
    inventory::install(app);
    minimap::install(app);
    app.add_systems(
        Update,
        (missing_item_appearances, project_regions.after(project)),
    );
}

#[cfg(test)]
mod native_tests;

/// Current item entries carry no verified appearance binding. Never treat a definition
/// index as a sprite id; the server-owned appearance capability will replace this fallback.
fn missing_item_appearances(
    mut commands: Commands,
    skins: Res<Skins>,
    mut icons: Query<(Entity, &ItemIcon, &mut ImageNode), Added<ItemIcon>>,
) {
    for (entity, item, mut image) in &mut icons {
        image.color = Color::NONE;
        let label = commands
            .spawn((
                source::text("?", source::font(skins.font.clone(), false)),
                Node {
                    position_type: PositionType::Absolute,
                    left: px(10.),
                    top: px(7.),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        commands
            .entity(entity)
            .insert(Name::new(format!(
                "Appearance unavailable, {} items",
                item.0.count
            )))
            .add_child(label);
    }
}

fn project_regions(
    owner: NonSend<HostOwner>,
    regions: Res<GameRegions>,
    mut nodes: Query<&mut Node>,
) {
    for (entity, id) in [
        (regions.battle, "battle"),
        (regions.chat, "chat"),
        (regions.equipment, "inventory"),
        (regions.containers, "inventory"),
        (regions.minimap, "minimap"),
        (regions.vip, "vip"),
    ] {
        if let Ok(mut node) = nodes.get_mut(entity) {
            node.display = if owner.settings.preferences.panel_visible(id) {
                Display::Flex
            } else {
                Display::None
            };
        }
    }
    if let Ok(mut node) = nodes.get_mut(regions.vitals) {
        node.display = if owner.settings.preferences.status_panel_visible {
            Display::Flex
        } else {
            Display::None
        };
    }
}

pub(super) fn open_quests(owner: &mut HostOwner) {
    journal::open_quests(owner);
}
pub(super) fn open_spells(owner: &mut HostOwner) {
    journal::open_spells(owner);
}
