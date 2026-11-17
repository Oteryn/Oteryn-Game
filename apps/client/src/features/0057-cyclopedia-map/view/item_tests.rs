#[test]
fn item_catalogue_loads_from_shared_host_with_pending_and_disconnect_gates()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::{combat_input::PlayCommand, play::play_channel};
    let (link, mut receiver, _) = play_channel();
    let mut mailbox = crate::play::mailbox::Mailbox::default();
    mailbox.discovery.inspection.selected = true;
    mailbox.discovery.inspection.catalogue_definition = "7".into();
    super::actions::load_item_definition(&mut mailbox, &link.commands);
    assert!(
        matches!(receiver.try_recv()?, PlayCommand::ServerMechanic(command) if command.capability == 132)
    );
    super::actions::load_item_definition(&mut mailbox, &link.commands);
    assert!(receiver.try_recv().is_err());
    mailbox.discovery.inspection.pending = false;
    drop(receiver);
    super::actions::load_item_definition(&mut mailbox, &link.commands);
    assert!(!mailbox.discovery.inspection.pending);
    assert_eq!(
        mailbox.discovery.inspection.feedback,
        "The session has ended"
    );
    Ok(())
}

#[test]
fn item_browser_round_trip_uses_paged_public_catalogue_protocol()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::{
        combat_input::PlayCommand,
        play::{mailbox::lock, play_channel},
    };
    use oteryn_client_runtime::server_mechanic::{CommandStatus, ServerMechanicResponse};
    use oteryn_client_ui::inspection::{self as wire, catalogue};
    let (link, mut receiver, outbox) = play_channel();
    {
        let mut mailbox = lock(&link.mailbox);
        mailbox.discovery.inspection.selected = true;
        mailbox.discovery.inspection.search = "silver".into();
        mailbox.discovery.inspection.category = "sword".into();
        super::actions::browse_items(&mut mailbox, &link.commands, false);
    }
    let PlayCommand::ServerMechanic(command) = receiver.try_recv()? else {
        return Err("catalogue command".into());
    };
    assert_eq!(
        wire::decode_request(&command.payload).map_err(|_| "request")?,
        wire::Request::CataloguePage(
            catalogue::PageQuery::new(0, "silver", "sword").map_err(|_| "query")?
        )
    );
    let reply = wire::Reply {
        status: 1,
        requests: vec![],
        items: vec![wire::InspectedItem {
            slot: 0,
            definition_ref: 10,
            count: 1,
            tier: 0,
            name: "Silver sword".into(),
            attributes: vec![
                ("Weapon Type".into(), "sword".into()),
                ("Attack".into(), "25".into()),
            ],
        }],
        catalogue: Some(catalogue::Page {
            next: Some(10),
            categories: vec!["sword".into()],
        }),
    };
    outbox.discovery_result(
        command.capability,
        &ServerMechanicResponse {
            command_id: 1,
            server_sequence: 1,
            status: CommandStatus::Accepted,
            payload: wire::encode_reply(&reply).map_err(|_| "reply")?,
        },
    );
    let mut mailbox = lock(&link.mailbox);
    assert!(super::actions::navigate_section(
        &mut mailbox,
        1,
        &[String::new(), "silver".into(), "sword".into()]
    ));
    assert!(super::section_text(&mailbox).contains("Attack: 25"));
    super::actions::browse_items(&mut mailbox, &link.commands, true);
    assert!(
        matches!(receiver.try_recv()?, PlayCommand::ServerMechanic(command)
        if wire::decode_request(&command.payload).map_err(|_| "request")? == wire::Request::CataloguePage(catalogue::PageQuery::new(10, "silver", "sword").map_err(|_| "query")?))
    );
    Ok(())
}

#[test]
fn catalogue_market_source_only_enqueues_read_only_offers_for_the_selected_item()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::{combat_input::PlayCommand, play::play_channel};
    use oteryn_client_ui::inspection::InspectedItem;
    let (link, mut receiver, _) = play_channel();
    let mut mailbox = crate::play::mailbox::Mailbox::default();
    mailbox.market.selected = true;
    mailbox.discovery.inspection.catalogue_definition = "40000".into();
    mailbox.discovery.inspection.catalogue.insert(
        40000,
        InspectedItem {
            slot: 0,
            definition_ref: 40000,
            count: 1,
            tier: 0,
            name: "Sword".into(),
            attributes: vec![],
        },
    );
    super::actions::item_market(&mut mailbox, &link.commands, false);
    let PlayCommand::Market(command) = receiver.try_recv()? else {
        return Err("market request".into());
    };
    assert_eq!(
        (
            command.0.action,
            command.0.definition,
            command.0.amount,
            command.0.price
        ),
        (2, 40000, 0, 0)
    );
    assert_eq!(command.0.offer, [0; 16]);
    mailbox.discovery.inspection.catalogue_definition = "999".into();
    super::actions::item_market(&mut mailbox, &link.commands, false);
    assert!(receiver.try_recv().is_err());
    Ok(())
}

#[test]
fn item_acquisition_sources_round_trip_to_matching_creatures_and_market_offers()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::{
        combat_input::PlayCommand,
        play::{mailbox::lock, play_channel},
    };
    use oteryn_client_runtime::{
        market::{self, market as quotes},
        server_mechanic::{CommandStatus, ServerMechanicResponse},
    };
    use oteryn_client_ui::inspection::{self as wire, catalogue};
    let (link, mut receiver, outbox) = play_channel();
    {
        let mut mailbox = lock(&link.mailbox);
        mailbox.discovery.inspection.selected = true;
        mailbox.discovery.inspection.catalogue_definition = "40000".into();
        mailbox.discovery.inspection.catalogue.insert(
            40000,
            wire::InspectedItem {
                slot: 0,
                definition_ref: 40000,
                count: 1,
                tier: 0,
                name: "Silver sword".into(),
                attributes: vec![],
            },
        );
        mailbox
            .properties
            .cyclopedia_map
            .navigate(oteryn_client_ui::cyclopedia_map::Section::Items);
        mailbox.market.selected = true;
        super::actions::item_sources(&mut mailbox, &link.commands, false);
    }
    let PlayCommand::ServerMechanic(command) = receiver.try_recv()? else {
        return Err("source command".into());
    };
    assert_eq!(
        wire::decode_request(&command.payload).map_err(|_| "request")?,
        wire::Request::CatalogueSources {
            definition: 40000,
            after: 0
        }
    );
    let reply = wire::Reply {
        status: 1,
        requests: vec![],
        items: vec![wire::InspectedItem {
            slot: 0,
            definition_ref: 40000,
            count: 1,
            tier: 0,
            name: "Silver sword".into(),
            attributes: vec![("Dropped by".into(), "Dragon".into())],
        }],
        catalogue: Some(catalogue::Page {
            next: Some(256),
            categories: vec![],
        }),
    };
    outbox.discovery_result(
        command.capability,
        &ServerMechanicResponse {
            command_id: 1,
            server_sequence: 1,
            status: CommandStatus::Accepted,
            payload: wire::encode_reply(&reply).map_err(|_| "reply")?,
        },
    );
    {
        let mut mailbox = lock(&link.mailbox);
        assert!(super::section_text(&mailbox).contains("Dropped by: Dragon"));
        super::actions::item_market(&mut mailbox, &link.commands, false);
    }
    let PlayCommand::Market(command) = receiver.try_recv()? else {
        return Err("market command".into());
    };
    assert_eq!((command.0.action, command.0.definition), (2, 40000));
    let mut offer = [1; 16];
    offer[6] = 0x71;
    offer[8] = 0x81;
    let reply = quotes::MarketReply {
        action: 2,
        code: 1,
        offer: [0; 16],
        rows: vec![
            quotes::MarketRow {
                offer,
                definition: 40000,
                side: 2,
                amount: 3,
                price: 12,
                state: 1,
                anonymous: false,
            },
            quotes::MarketRow {
                offer: {
                    let mut other = offer;
                    other[15] = 2;
                    other
                },
                definition: 39999,
                side: 2,
                amount: 1,
                price: 999,
                state: 1,
                anonymous: false,
            },
        ],
    };
    let bytes = quotes::encode_reply(&reply).map_err(|_| "market reply")?;
    outbox.market_result(&market::MarketResponse {
        status: market::CommandStatus::Accepted,
        reply: Some(quotes::decode_reply(&bytes).map_err(|_| "market reply")?),
    });
    let mailbox = lock(&link.mailbox);
    let text = super::section_text(&mailbox);
    assert!(text.contains("price 12"));
    assert!(!text.contains("39999"));
    drop(mailbox);
    {
        let mut mailbox = lock(&link.mailbox);
        super::actions::item_market(&mut mailbox, &link.commands, false);
    }
    assert!(matches!(receiver.try_recv()?, PlayCommand::Market(_)));
    outbox.market_result(&market::MarketResponse {
        status: market::CommandStatus::Rejected,
        reply: None,
    });
    let mailbox = lock(&link.mailbox);
    assert!(!super::section_text(&mailbox).contains("price 12"));
    assert_eq!(
        mailbox.discovery.inspection.catalogue_market_requested,
        None
    );
    Ok(())
}

#[test]
fn catalogue_scope_change_invalidates_definition_and_source_cursors()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::play::play_channel;
    use oteryn_client_ui::inspection::{Request, catalogue};
    let (link, mut receiver, _) = play_channel();
    let mut mailbox = crate::play::mailbox::Mailbox::default();
    let panel = &mut mailbox.discovery.inspection;
    panel.selected = true;
    panel.catalogue_definition = "40000".into();
    panel.pending = true;
    panel.catalogue.insert(
        40000,
        oteryn_client_ui::inspection::InspectedItem {
            slot: 0,
            definition_ref: 40000,
            count: 1,
            tier: 0,
            name: "Sword".into(),
            attributes: vec![],
        },
    );
    panel.catalogue_query = Some(catalogue::PageQuery::new(0, "", "").map_err(|_| "query")?);
    panel.catalogue_page = Some(catalogue::Page {
        next: Some(256),
        categories: vec![],
    });
    panel.catalogue_source_definition = 40000;
    panel.catalogue_source_next = Some(10);
    panel.catalogue_sources.push("Dragon".into());
    panel.catalogue_market_requested = Some(40000);
    panel.last_request = Some(Request::CatalogueSources {
        definition: 40000,
        after: 0,
    });
    mailbox.cancel_activity_requests();
    let panel = &mailbox.discovery.inspection;
    assert!(panel.catalogue_definition.is_empty());
    assert!(panel.catalogue.is_empty());
    assert!(!panel.pending);
    assert_eq!(panel.catalogue_query, None);
    assert_eq!(panel.catalogue_page, None);
    assert_eq!(panel.catalogue_source_next, None);
    assert_eq!(panel.catalogue_market_requested, None);
    assert!(panel.catalogue_sources.is_empty());
    assert_eq!(panel.last_request, None);
    assert!(!mailbox.discovery.inspection.select_item(false));
    super::actions::item_sources(&mut mailbox, &link.commands, true);
    assert!(receiver.try_recv().is_err());
    Ok(())
}
