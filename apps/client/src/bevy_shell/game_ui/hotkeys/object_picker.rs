//! Select only objects whose current inventory ancestry proves they are carried.
use super::*;
pub(super) fn carried(link: &crate::play::PlayLink) -> Vec<ItemEntry> {
    link.carried_shortcut_items()
}

pub(super) fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skins: Res<Skins>,
    editors: Query<&Editor>,
    mut roots: Query<(Entity, &mut Node), With<Objects>>,
    mut previous: Local<Option<Vec<ItemEntry>>>,
) {
    let picking = owner.preferences_draft().is_some() && editors.iter().any(|e| e.picking);
    let items = picking.then(|| {
        owner
            .play
            .as_ref()
            .map_or_else(Vec::new, |play| carried(&play.link))
    });
    for (root, mut node) in &mut roots {
        node.display = if picking {
            Display::Flex
        } else {
            Display::None
        };
        if items == *previous {
            continue;
        }
        commands.entity(root).despawn_children();
        if let Some(items) = &items {
            if items.is_empty() {
                label(&mut commands, root, &skins, "No carried objects available");
            }
            for item in items {
                let card = button(
                    &mut commands,
                    root,
                    &skins,
                    &format!(
                        "{} : {} × {}",
                        item.item_definition_ref, item.sub_type, item.count
                    ),
                    Control::Object(*item),
                );
                let icon = commands
                    .spawn((
                        super::super::ItemIcon(*item),
                        ImageNode {
                            color: Color::NONE,
                            ..default()
                        },
                        Node {
                            width: px(32.),
                            height: px(32.),
                            ..default()
                        },
                        Pickable::IGNORE,
                    ))
                    .id();
                commands.entity(card).add_child(icon);
            }
        }
    }
    *previous = items;
}
