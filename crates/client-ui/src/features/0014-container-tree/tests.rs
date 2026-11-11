use super::*;
use oteryn_client_runtime::container_tree::{
    ContainerView, ContainerViewIntent, ContainerViews, ItemEntry, ItemHandle,
};
use oteryn_client_runtime::inventory::{ItemMoveDestination, ItemMoveOutcome, MoveResponse};
use std::num::{NonZeroU32, NonZeroU64};

fn handle(value: u64) -> ItemHandle {
    NonZeroU64::new(value).unwrap_or(NonZeroU64::MIN)
}
fn entry(value: u64) -> ItemEntry {
    ItemEntry {
        handle: handle(value),
        item_definition_ref: NonZeroU32::MIN,
        count: NonZeroU32::MIN,
        sub_type: 0,
    }
}
fn view(view_id: u8, container: u64, entries: &[u64]) -> ContainerView {
    ContainerView {
        view_id,
        container_handle: handle(container),
        parent_handle: None,
        capacity: 20,
        entries: entries.iter().map(|v| entry(*v)).collect(),
        browse: None,
    }
}
fn panel() -> ContainerPanel {
    let mut panel = ContainerPanel::default();
    panel.apply(Some(ContainerViews {
        views: vec![view(0, 100, &[1, 2, 3]), view(1, 200, &[])],
    }));
    panel
}
fn moved() -> Option<MoveResponse> {
    Some(MoveResponse {
        outcome: ItemMoveOutcome::Moved,
        detail: None,
    })
}

#[test]
fn t038_managed_containers_menu_switches_views_selects_rows_and_opens_the_selection() {
    let mut panel = panel();
    assert_eq!(panel.current().map(|v| v.view_id), Some(0));
    assert_eq!(panel.rows().len(), 3);
    assert!(panel.activate(1));
    assert!(panel.rows().is_empty());
    assert!(!panel.activate(9));
    assert!(panel.activate(0));
    assert!(!panel.select(handle(77)));
    assert!(panel.select(handle(2)));
    assert_eq!(
        panel.open_selected(),
        Some(ContainerViewIntent::Open {
            handle: handle(2),
            replace_view: Some(0)
        })
    );
    assert!(panel.pending);
    assert!(panel.open_selected().is_none());
    assert!(panel_text_of(&panel).contains("Waiting for server"));
}

fn panel_text_of(panel: &ContainerPanel) -> String {
    view::panel_text(panel)
}

#[test]
fn t038_move_all_contents_snapshots_every_direct_entry_into_the_destination_in_order() {
    let mut panel = panel();
    let requests = panel.move_all(1).unwrap_or_default();
    assert_eq!(requests.len(), 3);
    for (request, source) in requests.iter().zip([1, 2, 3]) {
        assert_eq!(request.source, handle(source));
        assert_eq!(
            request.destination,
            ItemMoveDestination::Container(handle(200))
        );
        assert_eq!(request.count, None);
    }
    assert!(panel.bulk.pending);
    assert!(panel.move_all(1).is_none());
    assert!(panel.open_selected().is_none());
    assert!(panel.bulk_result(moved()));
    assert!(panel.bulk_result(moved()));
    assert!(!panel.bulk_result(moved()));
    assert_eq!((panel.bulk.completed, panel.bulk.total), (3, 3));
    assert!(view::panel_text(&panel).contains("3 / 3 moved"));
}

#[test]
fn t038_move_all_refuses_self_unknown_empty_and_stops_at_the_first_refusal() {
    let mut panel = panel();
    assert!(panel.move_all(0).is_none());
    assert!(panel.move_all(9).is_none());
    assert!(panel.activate(1));
    assert!(panel.move_all(0).is_none());
    assert!(panel.activate(0));
    assert!(panel.move_all(1).is_some());
    assert!(panel.bulk_result(moved()));
    assert!(!panel.bulk_result(Some(MoveResponse {
        outcome: ItemMoveOutcome::NoRoom,
        detail: None,
    })));
    assert_eq!((panel.bulk.completed, panel.bulk.total), (1, 3));
    assert!(!panel.bulk.pending);
    assert!(panel.bulk.feedback.is_some());
}
