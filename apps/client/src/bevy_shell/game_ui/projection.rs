//! Projection of retained region visibility and unavailable item appearances.
use super::*;

pub(in crate::bevy_shell) fn project(
    owner: NonSend<HostOwner>,
    mut roots: Query<&mut Node, With<GameRoot>>,
) {
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

/// Current item entries carry no verified appearance binding. Never treat a definition
/// index as a sprite id; the server-owned appearance capability will replace this fallback.
pub(super) fn missing_item_appearances(
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

pub(super) fn project_regions(
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
