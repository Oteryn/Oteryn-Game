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
            let visible = owner.settings.preferences.panel_visible(id)
                || id == "vip"
                    && owner
                        .play
                        .as_ref()
                        .is_some_and(|play| play.link.vip_is_open());
            node.display = if visible {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unavailable_item_appearance_builds_one_visible_source_label()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut app = App::new();
        app.insert_resource(Skins {
            buttons: source::ButtonImages {
                up: default(),
                down: default(),
            },
            checked: default(),
            unchecked: default(),
            frame: default(),
            inset: default(),
            font: default(),
        })
        .add_systems(Update, missing_item_appearances);
        let icon = app
            .world_mut()
            .spawn((
                ItemIcon(oteryn_session::ItemEntry {
                    handle: 7.try_into()?,
                    item_definition_ref: std::num::NonZeroU32::MIN,
                    count: 3.try_into()?,
                    sub_type: 0,
                }),
                ImageNode::default(),
            ))
            .id();
        app.update();
        let icon = app.world().entity(icon);
        assert_eq!(
            icon.get::<ImageNode>().ok_or("icon image")?.color,
            Color::NONE
        );
        assert_eq!(icon.get::<Children>().ok_or("label child")?.len(), 1);
        assert_eq!(
            icon.get::<Name>().ok_or("appearance diagnostic")?.as_str(),
            "Appearance unavailable, 3 items"
        );
        let mut labels = app.world_mut().query::<&Text>();
        assert_eq!(labels.single(app.world())?.0, "?");
        Ok(())
    }
}
