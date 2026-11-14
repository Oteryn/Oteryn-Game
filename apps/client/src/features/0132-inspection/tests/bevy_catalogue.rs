use crate::play::{mailbox::lock, play_channel};

fn reply(outbox: &crate::play::Outbox) -> Result<(), String> {
    use oteryn_client_runtime::server_mechanic::{CommandStatus, ServerMechanicResponse};
    use oteryn_client_ui::inspection::{InspectedItem, Reply, encode_reply};
    let payload = encode_reply(&Reply {
        status: 1,
        requests: vec![],
        catalogue: None,
        items: vec![InspectedItem {
            slot: 0,
            definition_ref: 7,
            count: 1,
            tier: 0,
            name: "Silver sword".into(),
            attributes: vec![
                ("Category".into(), "Weapons".into()),
                ("Catalogue categories".into(), "Armor|Weapons".into()),
                ("Drop sources".into(), "Race 3: Grove watcher".into()),
            ],
        }],
    })
    .map_err(|error| format!("server fixture: {error:?}"))?;
    outbox.discovery_result(
        132,
        &ServerMechanicResponse {
            command_id: 1,
            server_sequence: 1,
            status: CommandStatus::Accepted,
            payload,
        },
    );
    Ok(())
}
#[test]
fn item_catalogue_window_shows_owner_data_only() -> Result<(), String> {
    let (link, mut receiver, outbox) = play_channel();
    link.use_shared_frontend();
    lock(&link.mailbox).discovery.inspection.selected = true;
    assert!(link.item_catalogue_open_bound());
    assert!(receiver.try_recv().is_ok());
    assert!(!link.item_catalogue_snapshot().text.contains("Silver sword"));
    reply(&outbox)?;
    let snapshot = link.item_catalogue_snapshot();
    assert_eq!(snapshot.text, link.item_catalogue_text());
    assert_eq!(snapshot.panel, link.inspection_panel());
    assert!(snapshot.text.contains("Silver sword"));
    assert!(snapshot.text.contains("Grove watcher"));
    assert_eq!(snapshot.panel.categories(), ["Armor", "Weapons"]);
    Ok(())
}
#[test]
fn item_catalogue_actions_are_bound_and_confirmed() -> Result<(), String> {
    let (link, mut receiver, outbox) = play_channel();
    link.use_shared_frontend();
    lock(&link.mailbox).discovery.inspection.selected = true;
    assert!(link.item_catalogue_open_bound());
    assert!(receiver.try_recv().is_ok());
    let pending = link.item_catalogue_snapshot();
    assert!(!link.item_catalogue_dispatch_bound(&pending, pending.fields(), Some(0)));
    assert!(link.item_catalogue_dispatch_bound(&pending, pending.fields(), None));
    assert!(link.inspection_panel().pending);
    assert!(!link.inspection_panel().catalogue_open);
    assert!(link.item_catalogue_open_bound());
    assert!(receiver.try_recv().is_err());
    assert!(link.inspection_panel().pending);
    reply(&outbox)?;
    assert!(!link.inspection_panel().pending);
    assert!(link.item_catalogue_text().contains("Silver sword"));
    Ok(())
}
#[test]
fn item_catalogue_rejected_when_session_replaced_or_target_changed() {
    let (link, mut receiver, _) = play_channel();
    link.use_shared_frontend();
    {
        let mut m = lock(&link.mailbox);
        m.discovery.inspection.selected = true;
        m.discovery.inspection.catalogue_open = true;
    }
    let old = link.item_catalogue_snapshot();
    let (other, _other_receiver, _) = play_channel();
    other.use_shared_frontend();
    lock(&other.mailbox).discovery.inspection = old.panel.clone();
    assert!(!other.item_catalogue_dispatch_bound(&old, old.fields(), None));
    assert!(!other.item_catalogue_dispatch_bound(&old, old.fields(), Some(0)));
    lock(&link.mailbox)
        .discovery
        .inspection
        .catalogue_definition = "7".into();
    let changed = link.inspection_panel();
    assert!(!link.item_catalogue_dispatch_bound(&old, old.fields(), Some(0)));
    assert_eq!(link.inspection_panel(), changed);
    assert!(receiver.try_recv().is_err());
}
#[test]
fn item_catalogue_window_registered() {
    assert!(
        include_str!("../../../bevy_shell/game_ui/mod.rs").contains("item_catalogue::install(app)")
    );
    assert!(
        include_str!("../../../bevy_shell/controls.rs").contains("item_catalogue_input(routed)")
    );
    assert!(include_str!("../../../bevy_shell/ui/toolbar.rs").contains("Control::ItemCatalogue"));
    assert!(include_str!("../../../bevy_shell/ui.rs").contains("Catalogue (Ctrl+Alt+Shift+I)"));
}

#[test]
fn item_catalogue_every_action_matches_native_owner_dispatch() {
    use oteryn_client_ui::inspection::InspectedItem;
    for action in 0..10 {
        let (link, mut receiver, _) = play_channel();
        link.use_shared_frontend();
        let (native, mut native_receiver, _) = play_channel();
        native.use_shared_frontend();
        for session in [&link, &native] {
            let mut m = lock(&session.mailbox);
            m.market.selected = true;
            let p = &mut m.discovery.inspection;
            p.selected = true;
            p.catalogue_open = true;
            p.catalogue_next = Some(9);
            p.catalogue_drop_next = Some(2);
            p.catalogue_query_categories = vec!["Weapons".into(), "Armor".into()];
            p.catalogue_definition = "7".into();
            for id in [7, 8] {
                p.catalogue.insert(
                    id,
                    InspectedItem {
                        slot: 0,
                        definition_ref: id,
                        count: 1,
                        tier: 0,
                        name: format!("Server item {id}"),
                        attributes: vec![("Category".into(), "Weapons".into())],
                    },
                );
            }
        }
        let snapshot = link.item_catalogue_snapshot();
        assert!(
            link.item_catalogue_dispatch_bound(&snapshot, snapshot.fields(), Some(action)),
            "action {action}"
        );
        native.item_catalogue_request(action);
        assert_eq!(
            receiver.try_recv(),
            native_receiver.try_recv(),
            "action {action}"
        );
        assert_eq!(
            link.inspection_panel(),
            native.inspection_panel(),
            "action {action}"
        );
        // Read-only catalogue actions cannot invent returned item data.
        assert_eq!(link.inspection_panel().catalogue, snapshot.panel.catalogue);
        if action == 9 {
            let pending_market = link.item_catalogue_snapshot();
            assert!(!link.item_catalogue_dispatch_bound(
                &pending_market,
                pending_market.fields(),
                Some(9)
            ));
            assert_eq!(link.item_catalogue_snapshot(), pending_market);
        }
        if link.inspection_panel().pending || action == 9 {
            assert!(!link.item_catalogue_dispatch_bound(
                &snapshot,
                snapshot.fields(),
                Some(action)
            ));
            assert!(receiver.try_recv().is_err());
        }
    }
}

#[test]
fn item_catalogue_close_precedes_draft_capability_and_input_validation() {
    let (link, _receiver, _) = play_channel();
    link.use_shared_frontend();
    {
        let mut m = lock(&link.mailbox);
        m.discovery.inspection.catalogue_open = true;
        m.discovery.inspection.pending = true;
    }
    let expected = link.item_catalogue_snapshot();
    {
        let mut m = lock(&link.mailbox);
        m.discovery.inspection.selected = false;
        m.death_reset_commands = true;
    }
    assert!(link.item_catalogue_dispatch_bound(
        &expected,
        ["\0".into(), "x".repeat(1000), "invalid".into()],
        None
    ));
    assert!(link.inspection_panel().pending);
    assert!(!link.inspection_panel().catalogue_open);
}

#[test]
fn item_catalogue_modal_blocked_closed_and_ended_sessions_do_not_submit() {
    let (link, mut receiver, _) = play_channel();
    link.use_shared_frontend();
    {
        let mut m = lock(&link.mailbox);
        m.discovery.inspection.selected = true;
        m.discovery.inspection.catalogue_open = true;
    }
    let expected = link.item_catalogue_snapshot();
    let modal = link.bevy_server_modal_fixture();
    assert!(!link.item_catalogue_dispatch_bound(&expected, expected.fields(), Some(0)));
    assert_eq!(link.item_catalogue_snapshot(), expected);
    assert!(link.item_catalogue_dispatch_bound(&expected, expected.fields(), None));
    drop(modal);
    let closed = link.item_catalogue_snapshot();
    assert!(!link.item_catalogue_dispatch_bound(&closed, closed.fields(), Some(0)));
    assert_eq!(link.item_catalogue_snapshot(), closed);
    assert!(receiver.try_recv().is_err());
    lock(&link.mailbox).discovery.inspection.catalogue_open = true;
    lock(&link.mailbox).death_reset_commands = true;
    let blocked = link.item_catalogue_snapshot();
    assert!(!link.item_catalogue_dispatch_bound(&blocked, blocked.fields(), Some(0)));
    assert_eq!(link.item_catalogue_snapshot(), blocked);
    drop(receiver);
    assert!(!link.item_catalogue_dispatch_bound(&blocked, blocked.fields(), None));
    assert_eq!(link.item_catalogue_snapshot(), blocked);
}
