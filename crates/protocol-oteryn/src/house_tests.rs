use super::*;
fn id(n: u8) -> [u8; 16] {
    let mut b = [n; 16];
    b[6] = 0x70;
    b[8] = 0x80;
    b
}
fn target_body() -> Vec<u8> {
    let mut b = Vec::new();
    bytes(&mut b, 1, &id(1));
    bytes(&mut b, 2, b"oteryn:content.house.test");
    b
}
fn intent(tag: u64, nested: &[u8]) -> Vec<u8> {
    let mut b = Vec::new();
    bytes(&mut b, tag, nested);
    bytes(&mut b, 8, &target_body());
    b
}
fn acl(count: usize) -> Vec<u8> {
    let mut b = Vec::new();
    scalar(&mut b, 1, 1);
    scalar(&mut b, 2, 0);
    for n in 0..count {
        bytes(&mut b, 3, &id(n as u8));
    }
    b
}
#[test]
fn house_intent_strict_bounds_oneof_and_list() {
    assert!(matches!(
        decode_house_intent(&intent(4, &acl(200))),
        Ok(HouseIntent {
            action: HouseAction::Acl { revision: 0, .. },
            ..
        })
    ));
    assert_eq!(
        decode_house_intent(&intent(4, &acl(201))),
        Err(Error::LimitExceeded)
    );
    assert!(decode_house_intent(&vec![0; MAX_INTENT_BYTES + 1]).is_err());
    let mut b = intent(4, &acl(1));
    bytes(&mut b, 3, &[]);
    assert!(decode_house_intent(&b).is_err());
    let mut b = acl(1);
    bytes(&mut b, 3, &id(0));
    assert!(decode_house_intent(&intent(4, &b)).is_err());
    for b in [
        vec![],
        vec![0x08],
        vec![0x80; 10],
        intent(3, &[0]),
        intent(9, &[]),
    ] {
        assert!(decode_house_intent(&b).is_err());
    }
    let mut b = intent(3, &[]);
    bytes(&mut b, 8, &target_body());
    assert!(decode_house_intent(&b).is_err());
    let mut bad = Vec::new();
    bytes(&mut bad, 1, &[0; 16]);
    bytes(&mut bad, 2, b"h");
    let mut b = Vec::new();
    bytes(&mut b, 3, &[]);
    bytes(&mut b, 8, &bad);
    assert!(decode_house_intent(&b).is_err());
}
#[test]
fn house_door_native_coordinates_and_empty_clear() {
    for floor in [0, 29] {
        let mut p = Vec::new();
        scalar(&mut p, 1, 65535);
        scalar(&mut p, 2, 0);
        scalar(&mut p, 3, floor);
        let mut d = Vec::new();
        bytes(&mut d, 1, &p);
        scalar(&mut d, 2, 0);
        assert!(decode_house_intent(&intent(5, &d)).is_ok());
    }
    for (x, floor) in [(65536, 0), (0, 2), (0, 30), (0, 31)] {
        let mut p = Vec::new();
        scalar(&mut p, 1, x);
        scalar(&mut p, 2, 0);
        scalar(&mut p, 3, floor);
        let mut d = Vec::new();
        bytes(&mut d, 1, &p);
        scalar(&mut d, 2, 0);
        assert!(decode_house_intent(&intent(5, &d)).is_err());
    }
}
#[test]
fn house_query_strict_cursor_and_town() {
    let mut b = Vec::new();
    bytes(&mut b, 1, &id(1));
    bytes(&mut b, 2, b"oteryn:content.area.city.thais");
    bytes(&mut b, 3, &[]);
    assert!(matches!(
        decode_house_query(&b),
        Ok(HouseQuery { cursor: None, .. })
    ));
    bytes(&mut b, 3, &[]);
    assert!(decode_house_query(&b).is_err());
    assert!(decode_house_query(&vec![0; MAX_QUERY_BYTES + 1]).is_err());
    for town in [
        b"".as_slice(),
        b"thais",
        b"oteryn:content.area.city.",
        b"\xff",
    ] {
        let mut b = Vec::new();
        bytes(&mut b, 1, &id(1));
        bytes(&mut b, 2, town);
        assert!(decode_house_query(&b).is_err());
    }
}
#[test]
fn house_results_page_limit_and_typed_refusal() {
    assert_eq!(
        encode_house_result(HouseDisposition::Conflict, 0),
        vec![8, 3, 16, 0]
    );
    let row = HouseSummary {
        house: "h".into(),
        state: 3,
        rent: 0,
        revision: 1,
        acl_revision: 1,
        owned: true,
        move_out_ms: None,
        price: 0,
        ends_ms: None,
        own_maximum: None,
    };
    assert!(encode_house_page(&vec![row.clone(); PAGE_SIZE], None).is_ok());
    assert!(encode_house_page(&vec![row; PAGE_SIZE + 1], None).is_err());
}
