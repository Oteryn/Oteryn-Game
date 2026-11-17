use crate::{
    combat_input::PlayCommand,
    play::{mailbox::lock, play_channel},
};
use oteryn_client_runtime::{
    discovery::inspection::{self as wire, InspectedItem, Reply, catalogue::Query},
    server_mechanic::{CommandStatus, ServerMechanicResponse},
};

#[test]
fn item_catalogue_search_drops_and_market_use_production_queues()
-> Result<(), Box<dyn std::error::Error>> {
    let (link, mut receiver, outbox) = play_channel();
    {
        let mut mailbox = lock(&link.mailbox);
        mailbox.discovery.inspection.selected = true;
        mailbox.market.selected = true;
    }
    link.item_catalogue_filter("silver", "");
    use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
    assert!(
        link.item_catalogue_input(&[NormalizedInputEvent::Key {
            code: KeyCode::new(12).map_err(|e| format!("{e:?}"))?,
            state: ButtonState::Pressed,
            modifiers: Modifiers::one(Modifier::Control).with(Modifier::Alt),
            repeat: false,
        }])
    );
    let PlayCommand::ServerMechanic(command) = receiver.try_recv()? else {
        return Err("missing item browse query".into());
    };
    let wire::Request::CatalogueQuery(query @ Query::Items { .. }) =
        wire::decode_request(&command.payload).map_err(|e| format!("{e:?}"))?
    else {
        return Err("wrong item query".into());
    };
    assert_eq!(
        wire::catalogue::search(query).map_err(|e| format!("{e:?}"))?,
        "silver"
    );
    let item = InspectedItem {
        slot: 0,
        definition_ref: 7,
        count: 1,
        tier: 0,
        name: "Silver sword".into(),
        attributes: vec![
            ("Category".into(), "Weapons".into()),
            ("Catalogue categories".into(), "Armor|Weapons".into()),
            ("Catalogue next".into(), "7".into()),
        ],
    };
    outbox.discovery_result(
        132,
        &ServerMechanicResponse {
            command_id: 1,
            server_sequence: 1,
            status: CommandStatus::Accepted,
            payload: wire::encode_reply(&Reply {
                catalogue: None,
                status: 1,
                items: vec![item.clone()],
                requests: vec![],
            })
            .map_err(|e| format!("{e:?}"))?,
        },
    );
    let panel = link.discovery_panel().inspection;
    assert!(oteryn_client_ui::inspection::catalogue_text(&panel).contains("Silver sword"));
    assert_eq!(panel.catalogue_next, Some(7));
    assert_eq!(panel.categories(), vec!["Armor", "Weapons"]);
    link.item_catalogue_request(6);
    let PlayCommand::ServerMechanic(details) = receiver.try_recv()? else {
        return Err("missing item details query".into());
    };
    assert!(matches!(
        wire::decode_request(&details.payload).map_err(|e| format!("{e:?}"))?,
        wire::Request::Catalogue(7)
    ));
    let mut detailed = item.clone();
    detailed.attributes = vec![("Attack".into(), "25".into())];
    outbox.discovery_result(
        132,
        &ServerMechanicResponse {
            command_id: 2,
            server_sequence: 2,
            status: CommandStatus::Accepted,
            payload: wire::encode_reply(&Reply {
                catalogue: None,
                status: 1,
                items: vec![detailed],
                requests: vec![],
            })
            .map_err(|e| format!("{e:?}"))?,
        },
    );
    assert!(link.item_catalogue_text().contains("Attack: 25"));
    assert!(
        link.discovery_panel().inspection.catalogue[&7]
            .attributes
            .iter()
            .any(|(key, value)| key == "Category" && value == "Weapons")
    );
    link.item_catalogue_request(7);
    let PlayCommand::ServerMechanic(command) = receiver.try_recv()? else {
        return Err("missing drop sources query".into());
    };
    assert!(matches!(
        wire::decode_request(&command.payload).map_err(|e| format!("{e:?}"))?,
        wire::Request::CatalogueQuery(Query::Drops {
            reference: 7,
            after: 0
        })
    ));
    let mut dropped = item;
    dropped.attributes = vec![
        ("Category".into(), "Weapons".into()),
        ("Drop sources".into(), "Race 3: Grove watcher".into()),
    ];
    outbox.discovery_result(
        132,
        &ServerMechanicResponse {
            command_id: 2,
            server_sequence: 2,
            status: CommandStatus::Accepted,
            payload: wire::encode_reply(&Reply {
                catalogue: None,
                status: 1,
                items: vec![dropped],
                requests: vec![],
            })
            .map_err(|e| format!("{e:?}"))?,
        },
    );
    assert!(
        oteryn_client_ui::inspection::catalogue_text(&link.discovery_panel().inspection)
            .contains("Grove watcher")
    );
    link.item_catalogue_request(9);
    let PlayCommand::Market(command) = receiver.try_recv()? else {
        return Err("missing linked market query".into());
    };
    assert_eq!((command.0.action, command.0.definition), (2, 7));
    assert_eq!(command.0.amount, 0);
    use oteryn_session::activity_scope::{
        ActivityScope, ActivityScopeSnapshot, encode_activity_scope,
    };
    let mut id = [1; 16];
    id[6] = 0x70;
    id[8] = 0x80;
    let mut successor = id;
    successor[15] = 2;
    let payload = encode_activity_scope(&ActivityScopeSnapshot {
        handoff: id,
        source_session: id,
        session: successor,
        scope: ActivityScope::Instance {
            world: id,
            instance: id,
        },
        scope_generation: 1,
        connection_generation: 1,
        lease_generation: 2,
        selection: vec![117, 132],
        requires_ack: false,
    })
    .map_err(|e| format!("{e:?}"))?;
    outbox.push(vec![oteryn_session::SessionEvent::GatedSnapshotReplaced(
        oteryn_session::AppliedDelta {
            server_sequence: 3,
            base_revision: 0,
            new_revision: 1,
            value: oteryn_session::GatedSnapshot {
                domain_id: 99,
                snapshot_type: 1,
                revision: 1,
                payload,
            },
        },
    )]);
    assert!(link.discovery_panel().inspection.catalogue.is_empty());
    link.item_catalogue_request(9);
    assert!(receiver.try_recv().is_err());
    outbox.logged_out();
    assert!(link.discovery_panel().inspection.catalogue.is_empty());
    link.item_catalogue_request(0);
    assert!(receiver.try_recv().is_err());
    assert!(link.discovery_panel().inspection.catalogue.is_empty());
    Ok(())
}

#[test]
fn catalogue_continuations_reject_changed_inputs() -> Result<(), Box<dyn std::error::Error>> {
    let (link, mut receiver, _) = play_channel();
    {
        let mut mailbox = lock(&link.mailbox);
        let panel = &mut mailbox.discovery.inspection;
        panel.selected = true;
        panel.catalogue_next = Some(50);
        let query = wire::catalogue::items(0, 0, "old").map_err(|e| format!("{e:?}"))?;
        panel.last_request = Some(wire::Request::CatalogueQuery(query));
        panel.catalogue_items_query = Some(query);
        panel.search = "new".into();
    }
    link.item_catalogue_request(1);
    assert!(
        receiver.try_recv().is_err(),
        "changed search reused old cursor"
    );
    {
        let mut mailbox = lock(&link.mailbox);
        let panel = &mut mailbox.discovery.inspection;
        panel.catalogue_query_categories = vec!["Weapons".into()];
    }
    link.item_catalogue_filter("old", "Weapons");
    link.item_catalogue_request(1);
    assert!(
        receiver.try_recv().is_err(),
        "changed category reused old cursor"
    );
    link.item_catalogue_filter("old", "");
    link.item_catalogue_request(1);
    let PlayCommand::ServerMechanic(command) = receiver.try_recv()? else {
        return Err("missing valid item continuation".into());
    };
    assert!(matches!(
        wire::decode_request(&command.payload).map_err(|e| format!("{e:?}"))?,
        wire::Request::CatalogueQuery(Query::Items { after: 50, .. })
    ));
    {
        let mut mailbox = lock(&link.mailbox);
        let panel = &mut mailbox.discovery.inspection;
        panel.pending = false;
        panel.last_request = Some(wire::Request::CatalogueQuery(Query::Drops {
            reference: 7,
            after: 0,
        }));
        panel.catalogue_drop_next = Some(10);
        panel.catalogue_definition = "8".into();
    }
    link.item_catalogue_request(8);
    assert!(
        receiver.try_recv().is_err(),
        "changed item reused drop cursor"
    );
    lock(&link.mailbox)
        .discovery
        .inspection
        .catalogue_definition = "7".into();
    link.item_catalogue_request(8);
    let PlayCommand::ServerMechanic(command) = receiver.try_recv()? else {
        return Err("missing valid drop continuation".into());
    };
    assert!(matches!(
        wire::decode_request(&command.payload).map_err(|e| format!("{e:?}"))?,
        wire::Request::CatalogueQuery(Query::Drops {
            reference: 7,
            after: 10
        })
    ));
    Ok(())
}

#[test]
fn failed_catalogue_queries_cannot_reuse_previous_item_cursors()
-> Result<(), Box<dyn std::error::Error>> {
    let (link, mut receiver, outbox) = play_channel();
    {
        let mut mailbox = lock(&link.mailbox);
        let panel = &mut mailbox.discovery.inspection;
        panel.selected = true;
        panel.catalogue_next = Some(50);
        let query = wire::catalogue::items(0, 0, "old").map_err(|e| format!("{e:?}"))?;
        panel.last_request = Some(wire::Request::CatalogueQuery(query));
        panel.catalogue_items_query = Some(query);
        panel.search = "new".into();
    }
    link.item_catalogue_request(0);
    assert!(receiver.try_recv().is_ok());
    outbox.discovery_result(
        132,
        &ServerMechanicResponse {
            command_id: 1,
            server_sequence: 1,
            status: CommandStatus::Rejected,
            payload: vec![],
        },
    );
    link.item_catalogue_request(1);
    assert!(
        receiver.try_recv().is_err(),
        "failed fresh query borrowed old item cursor"
    );
    Ok(())
}

#[test]
fn failed_catalogue_queries_cannot_reuse_previous_drop_cursors()
-> Result<(), Box<dyn std::error::Error>> {
    let (link, mut receiver, outbox) = play_channel();
    {
        let mut mailbox = lock(&link.mailbox);
        let panel = &mut mailbox.discovery.inspection;
        panel.selected = true;
        panel.catalogue_drop_next = Some(10);
        panel.last_request = Some(wire::Request::CatalogueQuery(Query::Drops {
            reference: 7,
            after: 0,
        }));
        panel.catalogue_definition = "8".into();
    }
    link.item_catalogue_request(7);
    assert!(receiver.try_recv().is_ok());
    outbox.discovery_result(
        132,
        &ServerMechanicResponse {
            command_id: 1,
            server_sequence: 1,
            status: CommandStatus::Rejected,
            payload: vec![],
        },
    );
    link.item_catalogue_request(8);
    assert!(
        receiver.try_recv().is_err(),
        "failed fresh query borrowed old drop cursor"
    );
    Ok(())
}

#[test]
fn catalogue_next_page_survives_item_details_and_drop_source_requests() -> Result<(), Box<dyn std::error::Error>> {
    for action in [6, 7] {
        let (link, mut commands, outbox) = play_channel();
        {
            let mut mailbox = lock(&link.mailbox);
            let panel = &mut mailbox.discovery.inspection;
            panel.selected = true;
            panel.catalogue_next = Some(7);
            panel.catalogue_definition = "7".into();
            let query = wire::catalogue::items(0, 0, "silver").map_err(|e| format!("{e:?}"))?;
            panel.catalogue_items_query = Some(query);
            panel.last_request = Some(wire::Request::CatalogueQuery(query));
            panel.search = "silver".into();
        }
        link.item_catalogue_request(action);
        let PlayCommand::ServerMechanic(_) = commands.try_recv()? else { return Err("detail command".into()) };
        outbox.discovery_result(132, &ServerMechanicResponse {
            command_id: 1, server_sequence: 1, status: CommandStatus::Accepted,
            payload: wire::encode_reply(&Reply { status: 1, requests: vec![], catalogue: None, items: vec![InspectedItem { slot: 0, definition_ref: 7, count: 1, tier: 0, name: "Silver sword".into(), attributes: vec![("Drop sources".into(), "Rat".into())] }] }).map_err(|e| format!("{e:?}"))?,
        });
        link.item_catalogue_request(1);
        let PlayCommand::ServerMechanic(command) = commands.try_recv()? else { return Err("next page".into()) };
        assert!(matches!(wire::decode_request(&command.payload), Ok(wire::Request::CatalogueQuery(Query::Items { after: 7, category: 0, .. }))));
    }
    Ok(())
}

#[test]
fn catalogue_keeps_search_draft_while_paging_with_trimmed_query()
-> Result<(), Box<dyn std::error::Error>> {
    let (link, mut commands, outbox) = play_channel();
    lock(&link.mailbox).discovery.inspection.selected = true;
    let search = "sword ";
    link.item_catalogue_filter(search, "");
    link.item_catalogue_request(0);
    let PlayCommand::ServerMechanic(command) = commands.try_recv()? else {
        return Err("browse command".into());
    };
    let wire::Request::CatalogueQuery(query) =
        wire::decode_request(&command.payload).map_err(|error| format!("{error:?}"))?
    else {
        return Err("browse query".into());
    };
    assert_eq!(
        wire::catalogue::search(query).map_err(|error| format!("{error:?}"))?,
        "sword"
    );
    outbox.discovery_result(
        132,
        &ServerMechanicResponse {
            command_id: 1,
            server_sequence: 1,
            status: CommandStatus::Accepted,
            payload: wire::encode_reply(&Reply {
                status: 1,
                requests: vec![],
                catalogue: None,
                items: vec![InspectedItem {
                    slot: 0,
                    definition_ref: 7,
                    count: 1,
                    tier: 0,
                    name: "Sword".into(),
                    attributes: vec![("Catalogue next".into(), "7".into())],
                }],
            })
            .map_err(|error| format!("{error:?}"))?,
        },
    );
    let panel = link.inspection_panel();
    assert_eq!(panel.catalogue_next, Some(7));
    // The Bevy Next control restarts the browse if the live draft differs from this field.
    assert_eq!(panel.search, search);
    link.item_catalogue_filter(search, "");
    link.item_catalogue_request(1);
    let PlayCommand::ServerMechanic(command) = commands.try_recv()? else {
        return Err("next page command".into());
    };
    assert!(matches!(
        wire::decode_request(&command.payload),
        Ok(wire::Request::CatalogueQuery(Query::Items { after: 7, .. }))
    ));
    Ok(())
}
