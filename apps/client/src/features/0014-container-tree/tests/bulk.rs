#[test]
fn managed_container_moves_all_through_live_handles_and_stops_on_refusal()
-> Result<(), Box<dyn std::error::Error>> {
    use oteryn_session::container_tree::{ContainerView, ContainerViews};
    use oteryn_session::{ItemEntry, ItemMoveDestination, ItemMoveOutcome};
    let item = |handle: u64| -> Result<ItemEntry, Box<dyn std::error::Error>> {
        Ok(ItemEntry {
            handle: handle.try_into()?,
            item_definition_ref: 42.try_into()?,
            count: 7.try_into()?,
            sub_type: 0,
        })
    };
    let (link, mut commands, outbox) = play_channel();
    outbox.container_initialize(Some(ContainerViews {
        views: vec![
            ContainerView {
                view_id: 0,
                container_handle: 10.try_into()?,
                parent_handle: None,
                capacity: 20,
                entries: vec![item(11)?, item(12)?],
                browse: None,
            },
            ContainerView {
                view_id: 1,
                container_handle: 20.try_into()?,
                parent_handle: None,
                capacity: 20,
                entries: vec![],
                browse: None,
            },
        ],
    }));
    assert!(link.container_activate(0));
    assert!(!link.container_move_all(0));
    assert!(commands.try_recv().is_err());
    assert!(link.container_move_all(1));
    let crate::combat_input::PlayCommand::MoveContents(requests) = commands.try_recv()? else {
        return Err("move contents command".into());
    };
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[0].source, item(11)?.handle);
    assert_eq!(requests[1].source, item(12)?.handle);
    assert_eq!(
        requests[0].destination,
        ItemMoveDestination::Container(20.try_into()?)
    );
    assert_eq!(requests[0].count, None);
    assert!(!link.container_move_all(1));
    assert!(
        outbox.container_bulk_result(Some(oteryn_client_runtime::inventory::MoveResponse {
            outcome: ItemMoveOutcome::Moved,
            detail: None
        }))
    );
    assert!(
        !outbox.container_bulk_result(Some(oteryn_client_runtime::inventory::MoveResponse {
            outcome: ItemMoveOutcome::NoRoom,
            detail: None
        }))
    );
    let panel = link.container_panel();
    assert!(!panel.bulk.pending);
    assert_eq!(panel.bulk.completed, 1);
    assert_eq!(panel.bulk.total, 2);
    assert!(oteryn_client_ui::container_tree::view::panel_text(&panel).contains("1 / 2 moved"));
    assert!(oteryn_client_ui::container_tree::view::panel_text(&panel).contains("room"));
    outbox.container_end();
    assert!(!link.container_move_all(1));
    Ok(())
}

#[test]
fn managed_container_menu_opens_the_selected_live_inventory_container()
-> Result<(), Box<dyn std::error::Error>> {
    let (link, mut commands, outbox) = play_channel();
    outbox.container_initialize(Some(Default::default()));
    let backpack = oteryn_session::ItemEntry {
        handle: 10.try_into()?,
        item_definition_ref: 42.try_into()?,
        count: 1.try_into()?,
        sub_type: 0,
    };
    outbox.inventory_initialize(
        Some(oteryn_session::CharacterInventory {
            main_backpack: Some(backpack),
            ..Default::default()
        }),
        false,
    );
    assert!(link.inventory_select(backpack.handle));
    assert!(!link.discovery_action(98, 9, &[], 0));
    assert!(link.discovery_action(98, 7, &[], 0));
    assert!(link.discovery_action(98, 9, &[], 0));
    assert_eq!(
        commands.try_recv()?,
        crate::combat_input::PlayCommand::ContainerView(
            oteryn_session::container_tree::ContainerViewIntent::Open {
                handle: backpack.handle,
                replace_view: None
            }
        )
    );
    assert!(commands.try_recv().is_err());
    Ok(())
}
