use super::*;

#[test]
fn resource_negotiation_requires_atelier_and_is_bundle_only() {
    use crate::capability_tables::{
        BUNDLE_WORLD_OFFERS_V1, ITEM_VIEW_OFFERS_V1, PRODUCTION_OFFERS_V1,
    };

    assert_eq!(crate::validate_capability_closure(&[148]), Ok(()));
    assert_eq!(crate::validate_capability_closure(&[148, 272]), Ok(()));
    assert_eq!(
        crate::validate_capability_closure(&[272]),
        Err(crate::FoundationProtocolError::InvalidCapabilitySet)
    );
    assert_eq!(
        BUNDLE_WORLD_OFFERS_V1
            .iter()
            .find(|(id, _)| *id == 272)
            .map(|(_, requires)| *requires),
        Some(&[148][..])
    );
    for offers in [PRODUCTION_OFFERS_V1, ITEM_VIEW_OFFERS_V1] {
        assert!(!offers.iter().any(|(id, _)| *id == 272));
    }
}

#[test]
fn query_is_empty_and_cannot_supply_resource_facts() {
    assert_eq!(encode_query(), Vec::<u8>::new());
    assert_eq!(decode_query(&[]), Ok(()));
    for bytes in [
        vec![0],
        vec![8, 1],
        encode_reply(&ResourceBalances::default()),
    ] {
        assert_eq!(decode_query(&bytes), Err(Error::Malformed));
    }
    assert_eq!(MAX_REQUEST_BYTES, 0);
}

#[test]
fn all_seven_balances_have_distinct_required_canonical_fields() {
    let value = ResourceBalances {
        bank_gold: 1,
        carried_gold: 2,
        lesser_gems: 3,
        regular_gems: 4,
        greater_gems: 5,
        basic_fragments: 6,
        supreme_fragments: 7,
    };
    let bytes = vec![8, 1, 16, 2, 24, 3, 32, 4, 40, 5, 48, 6, 56, 7];
    assert_eq!(encode_reply(&value), bytes);
    assert_eq!(decode_reply(&bytes), Ok(value));
    let zero = vec![8, 0, 16, 0, 24, 0, 32, 0, 40, 0, 48, 0, 56, 0];
    assert_eq!(encode_reply(&ResourceBalances::default()), zero);
    assert_eq!(decode_reply(&zero), Ok(ResourceBalances::default()));
    for field in 0..7 {
        let mut missing = bytes.clone();
        drop(missing.drain(field * 2..field * 2 + 2));
        assert_eq!(decode_reply(&missing), Err(Error::Malformed));
        let mut duplicate = bytes.clone();
        duplicate.extend_from_slice(&bytes[field * 2..field * 2 + 2]);
        assert_eq!(decode_reply(&duplicate), Err(Error::Malformed));
    }
}

#[test]
fn resource_reply_refuses_unknown_wire_types_noncanonical_values_and_truncation() {
    let canonical = encode_reply(&ResourceBalances::default());
    for prefix in [vec![0, 0], vec![64, 0], vec![10, 0], vec![9, 0]] {
        let mut bytes = prefix;
        bytes.extend_from_slice(&canonical);
        assert_eq!(decode_reply(&bytes), Err(Error::Malformed));
    }
    let mut overlong_tag = canonical.clone();
    drop(overlong_tag.splice(0..1, [0x88, 0]));
    assert_eq!(decode_reply(&overlong_tag), Err(Error::Malformed));
    let mut overlong_value = canonical.clone();
    drop(overlong_value.splice(1..2, [0x80, 0]));
    assert_eq!(decode_reply(&overlong_value), Err(Error::Malformed));
    let mut out_of_order = canonical.clone();
    out_of_order.swap(0, 2);
    assert_eq!(decode_reply(&out_of_order), Err(Error::Malformed));
    for length in 0..canonical.len() {
        assert_eq!(decode_reply(&canonical[..length]), Err(Error::Malformed));
    }
}

#[test]
fn the_registered_reply_bound_is_exact_for_seven_u64_values() {
    let maximum = ResourceBalances {
        bank_gold: u64::MAX,
        carried_gold: u64::MAX,
        lesser_gems: u64::MAX,
        regular_gems: u64::MAX,
        greater_gems: u64::MAX,
        basic_fragments: u64::MAX,
        supreme_fragments: u64::MAX,
    };
    let mut bytes = encode_reply(&maximum);
    assert_eq!(MAX_REPLY_BYTES, 77);
    assert_eq!(bytes.len(), MAX_REPLY_BYTES);
    assert_eq!(decode_reply(&bytes), Ok(maximum));
    bytes.push(0);
    assert_eq!(decode_reply(&bytes), Err(Error::LimitExceeded));
    assert_eq!(
        decode_reply(&[0xff; MAX_REPLY_BYTES]),
        Err(Error::Malformed)
    );
}

#[test]
fn old_atelier_v1_reply_bytes_and_strict_field_set_are_unchanged() {
    let old = super::super::AtelierReply {
        refused: false,
        revision: 3,
        gems: Vec::new(),
        grades: Vec::new(),
        quality_quotes: Vec::new(),
        grade_quotes: Vec::new(),
    };
    let bytes = vec![8, 0, 16, 3];
    assert_eq!(super::super::encode_reply(&old), Ok(bytes.clone()));
    assert_eq!(super::super::decode_reply(&bytes), Ok(old));
    let mut forbidden_extension = bytes;
    forbidden_extension.extend_from_slice(&[56, 1]);
    assert_eq!(
        super::super::decode_reply(&forbidden_extension),
        Err(Error::Malformed)
    );
    assert_eq!(decode_reply(&[8, 0, 16, 3]), Err(Error::Malformed));
    assert_eq!(super::super::COMMAND_TYPE_ATELIER_QUERY, 308);
    assert_eq!(super::super::COMMAND_TYPE_ATELIER_SET_LOCK, 309);
    assert_eq!(super::super::COMMAND_TYPE_ATELIER_MUTATE, 424);
}
