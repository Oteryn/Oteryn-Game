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
    link.use_shared_frontend();
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
            modifiers: Modifiers::one(Modifier::Control)
                .with(Modifier::Alt)
                .with(Modifier::Shift),
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
