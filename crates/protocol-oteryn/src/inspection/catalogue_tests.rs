use super::*;
#[test]
fn public_catalogue_query_and_page_metadata_are_bounded_and_round_trip() -> Result<()> {
    let query = catalogue::Query::new(7, "silver", "sword")?;
    let request = Request::CataloguePage(query);
    assert_eq!(decode_request(&encode_request(request)?)?, request);
    assert!(catalogue::Query::new(0, &"x".repeat(49), "").is_err());
    let reply = Reply {
        status: 1,
        items: vec![],
        requests: vec![],
        catalogue: Some(catalogue::Page {
            next: Some(256),
            categories: vec!["sword".into(), "armor".into()],
        }),
    };
    assert_eq!(decode_reply(&encode_reply(&reply)?)?, reply);
    let mut invalid = reply;
    if let Some(page) = &mut invalid.catalogue {
        page.next = Some(0);
    }
    assert!(encode_reply(&invalid).is_err());
    assert!(decode_request(&[50, 2, 8, 0, 50, 2, 8, 0]).is_err());
    Ok(())
}

#[test]
fn catalogue_sources_use_their_own_cursor_without_exposing_instance_state() -> Result<()> {
    let request = Request::CatalogueSources {
        definition: 40_000,
        after: 7,
    };
    assert_eq!(decode_request(&encode_request(request)?)?, request);
    assert!(
        encode_request(Request::CatalogueSources {
            definition: 0,
            after: 0
        })
        .is_err()
    );
    let reply = Reply {
        status: 1,
        requests: vec![],
        items: vec![InspectedItem {
            slot: 0,
            definition_ref: 40_000,
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
    assert_eq!(decode_reply(&encode_reply(&reply)?)?, reply);
    let mut invalid = reply;
    invalid.items[0].tier = 1;
    assert!(encode_reply(&invalid).is_err());
    Ok(())
}
