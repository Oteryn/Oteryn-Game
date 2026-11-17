use super::*;
use oteryn_client_runtime::server_mechanic::{CommandStatus, ServerMechanicResponse};
#[test]
fn public_catalogue_paging_uses_wire_filters_and_keeps_confirmed_page_on_refusal()
-> Result<(), Box<dyn std::error::Error>> {
    let mut panel = Panel {
        selected: true,
        search: "silver".into(),
        category: "sword".into(),
        ..Default::default()
    };
    let command = request_page(&mut panel, false).ok_or("first query")?;
    let query = catalogue::PageQuery::new(0, "silver", "sword").map_err(|_| "query")?;
    assert_eq!(
        decode_request(&command.payload).map_err(|_| "request")?,
        Request::CataloguePage(query)
    );
    let reply = Reply {
        status: 1,
        requests: vec![],
        items: vec![InspectedItem {
            slot: 0,
            definition_ref: 11,
            count: 1,
            tier: 0,
            name: "Silver sword".into(),
            attributes: vec![("Weapon Type".into(), "sword".into())],
        }],
        catalogue: Some(catalogue::Page {
            next: Some(20),
            categories: vec!["sword".into(), "rune".into()],
        }),
    };
    apply_result(
        &mut panel,
        &ServerMechanicResponse {
            command_id: 1,
            server_sequence: 1,
            status: CommandStatus::Accepted,
            payload: encode_reply(&reply).map_err(|_| "reply")?,
        },
    );
    assert!(panel.categories().contains(&"rune"));
    let command = request_page(&mut panel, true).ok_or("next query")?;
    assert_eq!(
        decode_request(&command.payload).map_err(|_| "request")?,
        Request::CataloguePage(
            catalogue::PageQuery::new(20, "silver", "sword").map_err(|_| "query")?
        )
    );
    apply_result(
        &mut panel,
        &ServerMechanicResponse {
            command_id: 2,
            server_sequence: 2,
            status: CommandStatus::Rejected,
            payload: vec![],
        },
    );
    assert!(panel.catalogue.contains_key(&11));
    assert_eq!(
        panel.catalogue_page.as_ref().and_then(|page| page.next),
        Some(20)
    );
    panel.search = "different".into();
    assert!(request_page(&mut panel, true).is_none());
    Ok(())
}

#[test]
fn catalogue_sources_keep_separate_item_and_creature_cursors()
-> Result<(), Box<dyn std::error::Error>> {
    let mut panel = Panel {
        selected: true,
        catalogue_definition: "40000".into(),
        catalogue_query: Some(catalogue::PageQuery::new(0, "", "").map_err(|_| "query")?),
        catalogue_page: Some(catalogue::Page {
            next: Some(40000),
            categories: vec![],
        }),
        ..Default::default()
    };
    panel.catalogue.insert(
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
    let request = request_sources(&mut panel, false).ok_or("sources")?;
    assert_eq!(
        decode_request(&request.payload).map_err(|_| "request")?,
        Request::CatalogueSources {
            definition: 40000,
            after: 0
        }
    );
    let reply = Reply {
        status: 1,
        requests: vec![],
        items: vec![InspectedItem {
            slot: 0,
            definition_ref: 40000,
            count: 1,
            tier: 0,
            name: "Sword".into(),
            attributes: vec![("Dropped by".into(), "Dragon".into())],
        }],
        catalogue: Some(catalogue::Page {
            next: Some(256),
            categories: vec![],
        }),
    };
    apply_result(
        &mut panel,
        &ServerMechanicResponse {
            command_id: 1,
            server_sequence: 1,
            status: CommandStatus::Accepted,
            payload: encode_reply(&reply).map_err(|_| "reply")?,
        },
    );
    assert_eq!(panel.catalogue_sources, ["Dragon"]);
    assert_eq!(panel.catalogue_source_next, Some(256));
    let page = request_page(&mut panel, true).ok_or("catalogue next")?;
    assert_eq!(
        decode_request(&page.payload).map_err(|_| "request")?,
        Request::CataloguePage(catalogue::PageQuery::new(40000, "", "").map_err(|_| "query")?)
    );
    apply_result(
        &mut panel,
        &ServerMechanicResponse {
            command_id: 2,
            server_sequence: 2,
            status: CommandStatus::Rejected,
            payload: vec![],
        },
    );
    let source = request_sources(&mut panel, true).ok_or("sources next")?;
    assert_eq!(
        decode_request(&source.payload).map_err(|_| "request")?,
        Request::CatalogueSources {
            definition: 40000,
            after: 256
        }
    );
    let mut wrong = reply;
    wrong.items[0].definition_ref = 39999;
    if let Some(page) = &mut wrong.catalogue {
        page.next = Some(512);
    }
    apply_result(
        &mut panel,
        &ServerMechanicResponse {
            command_id: 3,
            server_sequence: 3,
            status: CommandStatus::Accepted,
            payload: encode_reply(&wrong).map_err(|_| "reply")?,
        },
    );
    assert_eq!(panel.catalogue_sources, ["Dragon"]);
    assert_eq!(panel.catalogue_source_next, Some(256));
    request_page(&mut panel, false).ok_or("restart catalogue")?;
    let invalid_page = Reply {
        status: 1,
        requests: vec![],
        items: vec![InspectedItem {
            slot: 0,
            definition_ref: 40000,
            count: 1,
            tier: 0,
            name: "Sword".into(),
            attributes: vec![],
        }],
        catalogue: Some(catalogue::Page {
            next: Some(256),
            categories: vec![],
        }),
    };
    apply_result(
        &mut panel,
        &ServerMechanicResponse {
            command_id: 4,
            server_sequence: 4,
            status: CommandStatus::Accepted,
            payload: encode_reply(&invalid_page).map_err(|_| "reply")?,
        },
    );
    assert_eq!(
        panel.catalogue_page.as_ref().and_then(|page| page.next),
        Some(40000)
    );
    Ok(())
}

#[test]
fn catalogue_item_selection_clears_sources_when_the_selected_template_changes() {
    let mut panel = Panel {
        selected: true,
        ..Default::default()
    };
    for id in [10, 20] {
        panel.catalogue.insert(
            id,
            InspectedItem {
                slot: 0,
                definition_ref: id,
                count: 1,
                tier: 0,
                name: format!("Item {id}"),
                attributes: vec![],
            },
        );
    }
    assert!(panel.select_item(false));
    assert_eq!(panel.catalogue_definition, "10");
    panel.catalogue_source_definition = 10;
    panel.catalogue_sources.push("Dragon".into());
    assert!(panel.select_item(false));
    assert_eq!(panel.catalogue_definition, "20");
    assert!(panel.catalogue_sources.is_empty());
    assert_eq!(panel.catalogue_source_definition, 0);
}

#[test]
fn catalogue_encodings_switch_without_reusing_the_other_cursor()
-> Result<(), Box<dyn std::error::Error>> {
    let mut panel = Panel {
        selected: true,
        ..Default::default()
    };
    let page = catalogue::PageQuery::new(0, "", "").map_err(|_| "page query")?;
    panel.catalogue_query = Some(page);
    panel.catalogue_page = Some(catalogue::Page {
        next: Some(100),
        categories: vec![],
    });
    panel.catalogue_sources.push("Dragon".into());
    let compact = catalogue::items(0, 0, "").map_err(|_| "compact query")?;
    request(&mut panel, Request::CatalogueQuery(compact)).ok_or("compact request")?;
    let mut reply = Reply {
        status: 1,
        requests: vec![],
        catalogue: None,
        items: vec![InspectedItem {
            slot: 0,
            definition_ref: 10,
            count: 1,
            tier: 0,
            name: "Sword".into(),
            attributes: vec![
                ("Catalogue next".into(), "10".into()),
                ("Catalogue categories".into(), "Weapon".into()),
            ],
        }],
    };
    apply_result(
        &mut panel,
        &ServerMechanicResponse {
            command_id: 1,
            server_sequence: 1,
            status: CommandStatus::Accepted,
            payload: encode_reply(&reply).map_err(|_| "compact reply")?,
        },
    );
    assert_eq!(panel.catalogue_next, Some(10));
    assert!(panel.catalogue_query.is_none());
    assert!(panel.catalogue_page.is_none());
    assert!(panel.catalogue_sources.is_empty());
    assert!(request_page(&mut panel, true).is_none());
    request_page(&mut panel, false).ok_or("page request")?;
    reply.items[0].attributes.clear();
    reply.catalogue = Some(catalogue::Page {
        next: Some(20),
        categories: vec!["sword".into()],
    });
    apply_result(
        &mut panel,
        &ServerMechanicResponse {
            command_id: 2,
            server_sequence: 2,
            status: CommandStatus::Accepted,
            payload: encode_reply(&reply).map_err(|_| "page reply")?,
        },
    );
    assert!(panel.catalogue_next.is_none());
    assert!(panel.catalogue_query_categories.is_empty());
    assert_eq!(panel.catalogue_query, Some(page));
    assert_eq!(
        panel.catalogue_page.as_ref().and_then(|page| page.next),
        Some(20)
    );
    Ok(())
}
