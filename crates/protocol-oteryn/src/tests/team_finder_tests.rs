use super::*;

const LISTING: LeaderListing = LeaderListing {
    min_level: 8,
    max_level: 100,
    vocation_mask: 15,
    slots: 5,
    free_slots: 4,
    party_members_join: true,
    timestamp: 7,
    target: TeamTarget::Hunt { kind: 2, area: 9 },
};

fn raw(fields: &[(u64, u64)]) -> Vec<u8> {
    let mut out = Vec::new();
    for &(field, value) in fields {
        push_varint_field(&mut out, field, value);
    }
    out
}

fn with(field: u64, value: u64) -> Vec<u8> {
    let mut fields = vec![
        (1, 8),
        (2, 100),
        (3, 15),
        (4, 5),
        (5, 4),
        (6, 1),
        (7, 7),
        (8, 2),
        (9, 2),
        (10, 9),
    ];
    for entry in &mut fields {
        if entry.0 == field {
            entry.1 = value;
        }
    }
    raw(&fields)
}

#[test]
fn every_target_kind_round_trips() -> Result<(), Error> {
    for target in [
        TeamTarget::Boss(1),
        TeamTarget::Hunt { kind: 2, area: 9 },
        TeamTarget::Quest(3),
    ] {
        let listing = LeaderListing {
            target,
            free_slots: 0,
            party_members_join: false,
            ..LISTING
        };
        assert_eq!(decode_upsert(&encode_upsert(listing)?)?, listing);
    }
    assert_eq!(decode_upsert(&with(0, 0))?, LISTING);
    Ok(())
}

#[test]
fn zero_minimum_level_is_refused() {
    assert_eq!(decode_upsert(&with(1, 0)), Err(Error::Malformed));
}

#[test]
fn minimum_above_maximum_level_is_refused() {
    assert_eq!(decode_upsert(&with(1, 101)), Err(Error::Malformed));
}

#[test]
fn empty_vocation_mask_is_refused() {
    assert_eq!(decode_upsert(&with(3, 0)), Err(Error::Malformed));
}

#[test]
fn vocation_mask_above_a_byte_is_refused() {
    assert_eq!(decode_upsert(&with(3, 256)), Err(Error::LimitExceeded));
}

#[test]
fn zero_team_slots_are_refused() {
    assert_eq!(decode_upsert(&with(4, 0)), Err(Error::Malformed));
}

#[test]
fn more_free_than_team_slots_are_refused() {
    assert_eq!(decode_upsert(&with(5, 6)), Err(Error::Malformed));
}

#[test]
fn party_flag_other_than_zero_or_one_is_refused() {
    assert_eq!(decode_upsert(&with(6, 2)), Err(Error::Malformed));
}

#[test]
fn level_above_u16_is_refused() {
    assert_eq!(decode_upsert(&with(2, 65_536)), Err(Error::LimitExceeded));
}

#[test]
fn unknown_team_type_is_refused() {
    assert_eq!(decode_upsert(&with(8, 4)), Err(Error::Malformed));
}

#[test]
fn hunt_without_area_is_refused() {
    let mut bytes = with(0, 0);
    bytes.truncate(bytes.len() - 2);
    assert_eq!(decode_upsert(&bytes), Err(Error::Malformed));
}

#[test]
fn boss_with_an_area_is_refused() {
    assert_eq!(decode_upsert(&with(8, 1)), Err(Error::Malformed));
}

#[test]
fn missing_field_is_refused() {
    let mut fields = vec![(1, 8), (2, 100), (3, 15), (4, 5), (5, 4), (6, 1), (8, 3)];
    fields.push((9, 1));
    assert_eq!(decode_upsert(&raw(&fields)), Err(Error::Malformed));
}

#[test]
fn repeated_field_is_refused() {
    let mut bytes = with(0, 0);
    push_varint_field(&mut bytes, 1, 8);
    assert_eq!(decode_upsert(&bytes), Err(Error::Malformed));
}

#[test]
fn unknown_field_is_refused() {
    let mut bytes = with(0, 0);
    push_varint_field(&mut bytes, 11, 1);
    assert_eq!(decode_upsert(&bytes), Err(Error::Malformed));
}

#[test]
fn oversized_request_is_refused() {
    assert_eq!(
        decode_upsert(&[0; MAX_REQUEST_BYTES + 1]),
        Err(Error::LimitExceeded)
    );
}

#[test]
fn remove_with_a_payload_is_refused() {
    assert_eq!(decode_remove(&[]), Ok(()));
    assert_eq!(decode_remove(&[8]), Err(Error::Malformed));
}
