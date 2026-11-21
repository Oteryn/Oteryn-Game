//! Select only objects whose current inventory ancestry proves they are carried.
use super::*;
pub(super) fn carried(link: &crate::play::PlayLink) -> Vec<ItemEntry> {
    let Some(inventory) = link.inventory_panel().inventory else {
        return Vec::new();
    };
    let mut items: Vec<_> = inventory
        .main_backpack
        .into_iter()
        .chain(inventory.entries)
        .chain(inventory.equipment.into_iter().map(|e| e.item))
        .collect();
    let mut handles: std::collections::BTreeSet<_> = items.iter().map(|item| item.handle).collect();
    if let Some(views) = link.container_panel().views {
        let mut admitted = std::collections::BTreeSet::new();
        for _ in 0..views.views.len() {
            let before = admitted.len();
            for view in &views.views {
                if view.browse.is_none()
                    && !admitted.contains(&view.view_id)
                    && (handles.contains(&view.container_handle)
                        || view
                            .parent_handle
                            .is_some_and(|parent| handles.contains(&parent)))
                {
                    admitted.insert(view.view_id);
                    handles.insert(view.container_handle);
                    handles.extend(view.entries.iter().map(|item| item.handle));
                    items.extend_from_slice(&view.entries);
                }
            }
            if before == admitted.len() {
                break;
            }
        }
    }
    items.sort_by_key(|item| item.handle);
    items.dedup_by_key(|item| item.handle);
    items
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
                button(
                    &mut commands,
                    root,
                    &skins,
                    &format!(
                        "{} : {} × {}",
                        item.item_definition_ref, item.sub_type, item.count
                    ),
                    Control::Object(*item),
                );
            }
        }
    }
    *previous = items;
}
