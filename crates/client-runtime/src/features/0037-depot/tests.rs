use super::*;
use std::num::{NonZeroU32, NonZeroU64};
fn item(n: u64) -> ItemEntry {
    ItemEntry {
        handle: NonZeroU64::new(n).unwrap_or(NonZeroU64::MIN),
        item_definition_ref: NonZeroU32::MIN,
        count: NonZeroU32::MIN,
        sub_type: 0,
    }
}
fn view(
    id: u8,
    container: u64,
    parent: Option<u64>,
    entries: &[u64],
) -> container_tree::ContainerView {
    container_tree::ContainerView {
        view_id: id,
        container_handle: item(container).handle,
        parent_handle: parent.map(|n| item(n).handle),
        capacity: 20,
        entries: entries.iter().map(|n| item(*n)).collect(),
        browse: None,
    }
}
#[test]
fn depot_sources_follow_carried_ancestry_and_exclude_corpse_and_browse_views() {
    let inventory = CharacterInventory {
        main_backpack: Some(item(1)),
        entries: vec![item(2)],
        ..Default::default()
    };
    let mut browse = view(5, 1, None, &[99]);
    browse.browse = Some(oteryn_session::browse_field::BrowseFieldPage {
        position: oteryn_session::item_view::WorldTilePosition {
            x: 0,
            y: 0,
            floor: 0,
        },
        index: 0,
        total: 1,
    });
    // Child precedes parent to exercise the ancestry closure; corpse and its nested bag
    // must remain excluded even though both have server-issued handles.
    let views = container_tree::ContainerViews {
        views: vec![
            view(0, 3, Some(1), &[4]),
            view(1, 1, None, &[3]),
            view(2, 2, None, &[5]),
            view(3, 80, None, &[81]),
            view(4, 81, Some(80), &[82]),
            browse,
        ],
    };
    assert_eq!(
        carried_entries(&inventory, &views),
        vec![item(4), item(3), item(5)]
    );
    assert!(carried_entries(&CharacterInventory::default(), &views).is_empty());
    // A replaced parent view still identifies a child whose parent is the carried backpack.
    assert_eq!(
        carried_entries(
            &inventory,
            &container_tree::ContainerViews {
                views: vec![view(0, 3, Some(1), &[4])]
            }
        ),
        vec![item(4)]
    );
}
