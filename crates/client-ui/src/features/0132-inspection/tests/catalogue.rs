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
    let query = catalogue::Query::new(0, "silver", "sword").map_err(|_| "query")?;
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
        Request::CataloguePage(catalogue::Query::new(20, "silver", "sword").map_err(|_| "query")?)
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
