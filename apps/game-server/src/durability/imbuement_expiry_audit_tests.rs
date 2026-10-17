//! Single native library registration; the path-loaded PG durability root owns no copy.

use crate::durability::imbuement_expiry_audit::{
    ImbuementExpiryEventIdentity, OneItemImbuementExpiryV1, OneItemImbuementSlotV1,
    check_imbuement_expiry, decode_imbuement_expiry_envelope, decode_imbuement_expiry_payload,
    encode_imbuement_expiry_event,
};
use crate::durability::item_mint_audit::{
    self as mint, AuditError, OneItemStateV1, OneItemTypedDefinitionRevisionV1, Type2EventTuple,
};
use prost::Message;
use sha2::{Digest, Sha256};

type TestResult = Result<(), String>;

fn id(seed: u8) -> [u8; 16] {
    [
        seed, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, seed,
    ]
}

fn shape() -> OneItemImbuementExpiryV1 {
    OneItemImbuementExpiryV1 {
        item: Some(OneItemStateV1 {
            item_instance_id: id(1).to_vec(),
            world_id: id(2).to_vec(),
            quantity: 1,
            lifecycle: mint::ITEM_LIFECYCLE_LIVE,
            key_number: 0,
            definition: Some(OneItemTypedDefinitionRevisionV1 {
                family: "Item".into(),
                production_key: "fixture:expiry-wear".into(),
                revision_ref: "item-r1".into(),
            }),
        }),
        row_before: Some(OneItemImbuementSlotV1 {
            slot: 0,
            imbuement_key: "fixture:imbuement".into(),
            content_revision: "apply-r1".into(),
            category: "fixture:category".into(),
            tier: 1,
            remaining_ms: 0,
            applying_occurrence: id(3).to_vec(),
            applying_transaction_id: id(4).to_vec(),
        }),
        character_id: id(5).to_vec(),
        world_id: id(2).to_vec(),
        channel_id: id(6).to_vec(),
        scope_ownership_generation: 1,
        expiry_occurrence: id(7).to_vec(),
        character_revision: 1,
        equipment_revision: 1,
        equipment_slot: 1,
    }
}

fn identity() -> ImbuementExpiryEventIdentity<'static> {
    ImbuementExpiryEventIdentity {
        event_id: id(8),
        transaction_id: id(9),
        occurred_at_unix_ms: 1_700_000_000_000,
        server_build_id: "expiry-test",
    }
}

fn debug(error: AuditError) -> String {
    format!("{error:?}")
}

#[test]
fn expiry_round_trip_keeps_complete_retained_binding_and_both_native_tuples() -> TestResult {
    for tuple in [Type2EventTuple::V1, Type2EventTuple::V2] {
        for slot in 0..=2 {
            for tier in 1..=3 {
                let mut expected = shape();
                let row = expected.row_before.as_mut().ok_or("fixture retained row")?;
                row.slot = slot;
                row.tier = tier;
                let wire = encode_imbuement_expiry_event(identity(), expected.clone(), tuple)
                    .map_err(debug)?;
                let (envelope, decoded) = decode_imbuement_expiry_envelope(&wire).map_err(debug)?;
                assert_eq!(expected, decoded);
                assert_eq!(envelope.event_schema_revision, tuple.schema_revision());
                assert_eq!(envelope.retention_profile_id, tuple.retention_profile_id());
                // Interpretation revision 1, proposed operation tag 14 (length-delimited key 114).
                assert_eq!(&envelope.payload[..3], &[8, 1, 114]);
                assert!(envelope.game_session_id.is_none() && envelope.command_id.is_none());
            }
        }
    }
    Ok(())
}

#[test]
fn expiry_refuses_nonzero_time_bad_slot_tier_identity_and_missing_binding() -> TestResult {
    for mutation in 0..19 {
        let mut invalid = shape();
        let row = invalid.row_before.as_mut().ok_or("fixture retained row")?;
        match mutation {
            0 => row.remaining_ms = 1,
            1 => row.slot = 3,
            2 => row.tier = 0,
            3 => row.tier = 4,
            4 => row.imbuement_key.clear(),
            5 => row.content_revision = " ".into(),
            6 => row.category = "bad\nkey".into(),
            7 => row.applying_occurrence[6] = 0x60,
            8 => row.applying_transaction_id[8] = 0,
            9 => invalid.row_before = None,
            10 => invalid.item.as_mut().ok_or("fixture item")?.quantity = 0,
            11 => invalid.item.as_mut().ok_or("fixture item")?.lifecycle = 2,
            12 => invalid.item.as_mut().ok_or("fixture item")?.world_id = id(10).to_vec(),
            13 => invalid.character_revision = 0,
            14 => invalid.equipment_revision = 0,
            15 => invalid.scope_ownership_generation = 0,
            16 => invalid.equipment_slot = 9,
            17 => invalid.expiry_occurrence.clear(),
            _ => invalid.expiry_occurrence = row.applying_occurrence.clone(),
        }
        assert_eq!(
            check_imbuement_expiry(&invalid),
            Err(AuditError::InvalidInput),
            "mutation {mutation}"
        );
    }
    Ok(())
}

#[test]
fn expiry_bounds_are_checked_before_decode_and_closed_wire_rejects_substitution() -> TestResult {
    let mut maximal = shape();
    let row = maximal.row_before.as_mut().ok_or("fixture retained row")?;
    row.slot = 2;
    row.tier = 3;
    row.imbuement_key = "k".repeat(512);
    row.content_revision = "r".repeat(512);
    row.category = "c".repeat(512);
    let item = maximal.item.as_mut().ok_or("fixture item")?;
    item.quantity = u32::MAX;
    let definition = item.definition.as_mut().ok_or("fixture definition")?;
    definition.family = "f".repeat(128);
    definition.production_key = "p".repeat(512);
    definition.revision_ref = "r".repeat(512);
    maximal.character_revision = u64::MAX;
    maximal.equipment_revision = u64::MAX;
    maximal.scope_ownership_generation = u64::MAX;
    maximal.equipment_slot = 10;
    let build = "b".repeat(128);
    let mut identity = identity();
    identity.server_build_id = &build;
    identity.occurred_at_unix_ms = i64::MAX - mint::AUDIT_RETENTION_P90D_MS;
    let wire = encode_imbuement_expiry_event(identity, maximal.clone(), Type2EventTuple::V2)
        .map_err(debug)?;
    let (mut envelope, _) = decode_imbuement_expiry_envelope(&wire).map_err(debug)?;
    assert!(envelope.payload.len() <= mint::RL07_PAYLOAD_BYTES_MAX);
    assert!(wire.len() <= mint::RL07_ENVELOPE_BYTES_MAX);
    eprintln!(
        "IMBUE-EXPIRY maximum payload={} envelope={}",
        envelope.payload.len(),
        wire.len()
    );
    maximal
        .row_before
        .as_mut()
        .ok_or("fixture retained row")?
        .category
        .push('x');
    assert_eq!(
        check_imbuement_expiry(&maximal),
        Err(AuditError::InvalidInput)
    );
    assert_eq!(
        decode_imbuement_expiry_payload(&vec![0; mint::RL07_PAYLOAD_BYTES_MAX + 1]),
        Err(AuditError::CapacityExceeded)
    );
    assert_eq!(
        decode_imbuement_expiry_envelope(&vec![0; mint::RL07_ENVELOPE_BYTES_MAX + 1]),
        Err(AuditError::CapacityExceeded)
    );
    // Timed expiry, HOUSE, TRADE (the old private Imbue tag) and Forge are foreign arms.
    for operation_tag in [7_u8, 11, 12, 13] {
        envelope.payload[2] = (operation_tag << 3) | 2;
        envelope.payload_sha256 = Sha256::digest(&envelope.payload).to_vec();
        assert!(
            decode_imbuement_expiry_envelope(&envelope.encode_to_vec()).is_err(),
            "foreign operation tag {operation_tag}"
        );
    }
    let mut unknown = wire.clone();
    unknown.extend([0xf8, 0x07, 0x01]);
    assert!(decode_imbuement_expiry_envelope(&unknown).is_err());
    let (mut caller, _) = decode_imbuement_expiry_envelope(&wire).map_err(debug)?;
    caller.game_session_id = Some(id(10).to_vec());
    assert!(decode_imbuement_expiry_envelope(&caller.encode_to_vec()).is_err());
    for invalid_time in [
        i64::MIN,
        -1,
        0,
        i64::MAX - mint::AUDIT_RETENTION_P90D_MS + 1,
    ] {
        let mut invalid_identity = identity;
        invalid_identity.occurred_at_unix_ms = invalid_time;
        assert_eq!(
            encode_imbuement_expiry_event(invalid_identity, shape(), Type2EventTuple::V2),
            Err(AuditError::InvalidInput)
        );
    }
    let mut reused_identity = identity;
    reused_identity.event_id = reused_identity.transaction_id;
    assert_eq!(
        encode_imbuement_expiry_event(reused_identity, shape(), Type2EventTuple::V2),
        Err(AuditError::InvalidInput)
    );
    Ok(())
}
