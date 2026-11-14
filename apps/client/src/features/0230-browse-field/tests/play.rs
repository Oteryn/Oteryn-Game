use crate::play::play_channel;
#[test]
fn browse_field_panel_opens_and_sends() {
    let (link, mut rx, outbox) = play_channel();
    outbox.browse_field_initialize(true);
    link.browse_field_request(vec![]);
    assert!(matches!(
        rx.try_recv(),
        Ok(crate::combat_input::PlayCommand::ServerMechanic(_))
    ));
}

#[test]
fn browse_field_native_moves_use_session_item_commands() {
    use oteryn_session::{ItemHandle, ItemMoveDestination};
    let (link, mut rx, outbox) = play_channel();
    outbox.browse_field_initialize(true);
    let Some(source) = ItemHandle::new(7) else {
        return;
    };
    link.browse_field_move(source, ItemMoveDestination::MainBackpack);
    assert!(
        matches!(rx.try_recv(), Ok(crate::combat_input::PlayCommand::BrowseFieldMove(intent)) if intent.source == source)
    );
}

#[test]
fn browse_field_up_keeps_the_ground_container_view_id() -> Result<(), Box<dyn std::error::Error>> {
    let (link, mut rx, outbox) = play_channel();
    outbox.browse_field_initialize(true);
    link.browse_field_up(3);
    let command = rx.try_recv()?;
    let crate::combat_input::PlayCommand::ServerMechanic(command) = command else {
        return Err("missing container command".into());
    };
    assert_eq!((command.capability, command.command_type), (14, 21));
    assert_eq!(
        oteryn_session::container_tree::decode_container_view_intent(&command.payload)
            .map_err(|e| format!("{e:?}"))?,
        oteryn_session::container_tree::ContainerViewIntent::Up { view_id: 3 }
    );
    Ok(())
}

#[test]
fn browse_field_selects_the_first_visible_server_window() {
    let (link, _, outbox) = play_channel();
    outbox.browse_field_initialize(true);
    outbox.push(vec![oteryn_session::SessionEvent::ContainerViews(
        oteryn_session::AppliedDelta {
            server_sequence: 41,
            base_revision: 1,
            new_revision: 2,
            value: oteryn_session::container_tree::ContainerViews {
                views: vec![oteryn_session::container_tree::ContainerView {
                    view_id: 3,
                    container_handle: oteryn_session::ItemHandle::MIN,
                    parent_handle: None,
                    capacity: 30,
                    entries: vec![],
                    browse: Some(oteryn_session::browse_field::BrowseFieldPage {
                        position: oteryn_session::item_view::WorldTilePosition {
                            x: 100,
                            y: 200,
                            floor: 0,
                        },
                        index: 0,
                        total: 0,
                    }),
                }],
            },
        },
    )]);
    assert_eq!(link.browse_field_panel().view_id, 3);
}

#[cfg(all(not(windows), not(feature = "bevy-shell")))]
#[path = "../native.rs"]
mod native;
#[cfg(all(not(windows), feature = "bevy-shell"))]
use crate::play::feature_0230_browse_field::view::bevy_tests::native;

#[cfg(not(windows))]
#[test]
fn browse_field_native_controls_and_results() {
    use oteryn_client_runtime::cui3::browse_field::{
        BrowseFieldIntent, decode_browse_field_intent,
    };
    use oteryn_client_runtime::cui3::item_view::WorldTilePosition;
    use oteryn_client_runtime::server_mechanic::{CommandStatus, ServerMechanicResponse};
    use oteryn_client_ui::browse_field::{BrowseFieldPanel, apply_result};
    let mut p = BrowseFieldPanel {
        selected: true,
        ..Default::default()
    };
    assert!(native::submit(&mut p, false, ["bad", "200", "-7", "0", "0"]).is_none());
    assert!(!p.pending);
    let command = native::submit(&mut p, false, ["100", "200", "-7", "0", "0"]);
    assert!(
        matches!(command, Some(c) if c.capability == 230 && c.command_type == 440 && decode_browse_field_intent(&c.payload) == Ok(BrowseFieldIntent::Open { position: WorldTilePosition { x: 100, y: 200, floor: -7 } }))
    );
    assert!(!native::enabled(&p));
    assert!(native::result_text(&p).contains("Waiting"));
    assert!(native::submit(&mut p, true, ["", "", "", "1", "30"]).is_none());
    apply_result(
        &mut p,
        &ServerMechanicResponse {
            command_id: 1,
            status: CommandStatus::Accepted,
            server_sequence: 1,
            payload: vec![8, 1],
        },
    );
    assert!(native::enabled(&p));
    assert!(native::result_text(&p).contains("Opened"));
    let command = native::submit(&mut p, true, ["", "", "", "1", "30"]);
    assert!(
        matches!(command, Some(c) if decode_browse_field_intent(&c.payload) == Ok(BrowseFieldIntent::Seek { view_id: 1, index: 30 }))
    );
    apply_result(
        &mut p,
        &ServerMechanicResponse {
            command_id: 2,
            status: CommandStatus::Rejected,
            server_sequence: 2,
            payload: vec![],
        },
    );
    assert!(native::enabled(&p));
    assert!(native::result_text(&p).contains("refused"));
    p.selected = false;
    assert!(!native::enabled(&p));
}

#[cfg(not(windows))]
#[test]
fn browse_field_native_item_controls_preserve_move_and_up_routes()
-> Result<(), Box<dyn std::error::Error>> {
    use oteryn_session::{ItemEntry, ItemHandle, ItemMoveDestination};
    let (link, _, outbox) = play_channel();
    outbox.browse_field_initialize(true);
    let position = oteryn_session::item_view::WorldTilePosition {
        x: 100,
        y: 200,
        floor: 0,
    };
    let item = ItemEntry {
        handle: ItemHandle::MIN,
        item_definition_ref: 42.try_into()?,
        count: 1.try_into()?,
        sub_type: 0,
    };
    let mut panel = link.browse_field_panel();
    panel
        .views
        .views
        .push(oteryn_session::container_tree::ContainerView {
            view_id: 3,
            container_handle: 50.try_into()?,
            parent_handle: None,
            capacity: 30,
            entries: vec![item],
            browse: Some(oteryn_session::browse_field::BrowseFieldPage {
                position,
                index: 0,
                total: 1,
            }),
        });
    panel.view_id = 3;
    panel.inventory.entries.push(ItemEntry {
        handle: 2.try_into()?,
        ..item
    });
    let command = native::item_command(&mut panel, 105, "0", "0");
    assert!(
        matches!(command,Some(crate::combat_input::PlayCommand::BrowseFieldMove(intent)) if intent.source==item.handle && intent.destination==ItemMoveDestination::MainBackpack)
    );
    assert!(panel.pending);
    panel.pending = false;
    let command = native::item_command(&mut panel, 106, "0", "0");
    assert!(
        matches!(command,Some(crate::combat_input::PlayCommand::BrowseFieldMove(intent)) if intent.source.get()==2 && intent.destination==ItemMoveDestination::Ground(position))
    );
    panel.pending = false;
    let command = native::item_command(&mut panel, 108, "invalid", "invalid");
    let Some(crate::combat_input::PlayCommand::ServerMechanic(command)) = command else {
        return Err("missing Up command".into());
    };
    assert_eq!((command.capability, command.command_type), (14, 21));
    assert_eq!(
        oteryn_session::container_tree::decode_container_view_intent(&command.payload),
        Ok(oteryn_session::container_tree::ContainerViewIntent::Up { view_id: 3 })
    );
    panel.pending = false;
    let command = native::item_command(&mut panel, 109, "0", "0");
    assert!(
        matches!(command, Some(crate::combat_input::PlayCommand::WorldContext(command)) if command.target == oteryn_client_runtime::world_interaction::Target::Item(item.handle) && command.action == oteryn_client_runtime::world_interaction::Action::Use)
    );
    assert!(!panel.pending);
    assert!(native::item_command(&mut panel, 105, "invalid", "0").is_none());
    assert!(!panel.pending);
    Ok(())
}

#[test]
fn browse_panel_accepts_negotiated_house_views_and_keeps_legacy_bound()
-> Result<(), Box<dyn std::error::Error>> {
    use oteryn_session::container_tree::{
        ContainerView, ContainerViews, encode_container_views_with_house_capacity,
    };
    let views = ContainerViews {
        views: vec![ContainerView {
            view_id: 0,
            container_handle: 1_u64.try_into()?,
            parent_handle: None,
            capacity: 25,
            entries: (2_u64..=26)
                .map(|id| {
                    Ok(oteryn_session::ItemEntry {
                        handle: id.try_into()?,
                        item_definition_ref: 2854_u32.try_into()?,
                        count: 1_u32.try_into()?,
                        sub_type: 0,
                    })
                })
                .collect::<Result<_, Box<dyn std::error::Error>>>()?,
            browse: None,
        }],
    };
    let bytes = encode_container_views_with_house_capacity(&views, true, true)
        .map_err(|e| format!("{e:?}"))?;
    for selected in [false, true] {
        let (link, _, outbox) = play_channel();
        outbox.container_initialize_with_house_capacity(None, selected);
        outbox.browse_field_initialize(true);
        outbox.browse_field_views(&bytes);
        let panel = link.browse_field_panel();
        if selected {
            assert_eq!(panel.views, views);
            assert!(panel.feedback.is_none());
        } else {
            assert!(panel.views.views.is_empty());
            assert_eq!(panel.feedback.as_deref(), Some("Invalid container view"));
        }
    }
    Ok(())
}
