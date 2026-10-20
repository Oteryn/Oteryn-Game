//! Single lib.rs test mount for the new LIMIT_RAISE audit/value regressions.

use crate::domain::forge_dust::{
    DUST_LIMIT_UPGRADES_MAX, DustLimit, ForgeDustBalance, ForgeDustError,
};
use crate::durability::character_forge_dust_audit::*;
use crate::durability::item_mint_audit::{
    self as mint, AuditError, OneItemOperationV1, OneItemTransactionV1, Type2EventTuple,
};
use prost::Message;

fn id(n: u8) -> Vec<u8> {
    let mut value = vec![n; 16];
    value[6] = 0x70;
    value[8] = 0x80;
    value
}
fn value(stage: u16) -> OneItemForgeDustLimitV1 {
    let cost = 25 + u32::from(stage);
    OneItemForgeDustLimitV1 {
        character_id: id(1),
        world_id: id(2),
        channel_id: id(3),
        occurrence_id: id(4),
        entry_id: id(5),
        previous_entry_id: Some(id(6)),
        before: Some(OneForgeDustBalanceV1 {
            balance: cost,
            dust_limit: 100 + 20 * u32::from(stage),
        }),
        after: Some(OneForgeDustBalanceV1 {
            balance: 0,
            dust_limit: 120 + 20 * u32::from(stage),
        }),
        dust_burned: cost,
        content_digest: vec![7; 32],
        ruleset_revision: DUST_LIMIT_RULESET_REVISION.into(),
        simulation_profile_revision: 1,
        game_session_id: id(8),
        command_id: u64::MAX,
        connection_generation: u64::MAX,
        character_lease_generation: u64::MAX,
        scope_ownership_generation: u64::MAX,
        character_revision: u64::MAX,
    }
}
fn identity() -> mint::MintEventIdentity<'static> {
    let mut event = [11; 16];
    event[6] = 0x70;
    event[8] = 0x80;
    let mut transaction = [12; 16];
    transaction[6] = 0x70;
    transaction[8] = 0x80;
    mint::MintEventIdentity {
        event_id: event,
        transaction_id: transaction,
        occurred_at_unix_ms: 1_800_000_000_000,
        server_build_id: "forge-limit-test",
    }
}

#[test]
fn all_225_limit_transitions_round_trip_both_registered_type2_tuples() -> Result<(), AuditError> {
    for stage in 0..225 {
        for tuple in [Type2EventTuple::V1, Type2EventTuple::V2] {
            let expected = value(stage);
            let wire = encode_forge_dust_limit_event(identity(), expected.clone(), tuple)?;
            let (envelope, decoded) = decode_forge_dust_limit_envelope(&wire)?;
            assert_eq!(decoded, expected);
            assert_eq!(envelope.event_schema_revision, tuple.schema_revision());
            assert_eq!(envelope.retention_profile_id, tuple.retention_profile_id());
            assert!(envelope.payload.len() <= mint::RL07_PAYLOAD_BYTES_MAX);
            assert!(wire.len() <= mint::RL07_ENVELOPE_BYTES_MAX);
        }
    }
    assert_eq!(
        check_forge_dust_limit(&value(225)),
        Err(AuditError::InvalidInput)
    );
    Ok(())
}

#[test]
fn closed_limit_shape_rejects_arithmetic_identity_binding_and_missing_fields() {
    for mutation in 0..23 {
        let mut changed = value(0);
        match mutation {
            0 => changed.dust_burned += 1,
            1 => {
                changed.after = Some(OneForgeDustBalanceV1 {
                    balance: 1,
                    dust_limit: 120,
                })
            }
            2 => {
                changed.after = Some(OneForgeDustBalanceV1 {
                    balance: 0,
                    dust_limit: 121,
                })
            }
            3 => {
                changed.before = Some(OneForgeDustBalanceV1 {
                    balance: 24,
                    dust_limit: 100,
                })
            }
            4 => changed.before = None,
            5 => changed.after = None,
            6 => changed.previous_entry_id = None,
            7 => changed.previous_entry_id = Some(changed.entry_id.clone()),
            8 => {
                changed.content_digest.pop();
            }
            9 => changed.content_digest = vec![0; 32],
            10 => changed.ruleset_revision = "unknown".into(),
            11 => changed.simulation_profile_revision = 0,
            12 => changed.command_id = 0,
            13 => changed.connection_generation = 0,
            14 => changed.character_lease_generation = 0,
            15 => changed.scope_ownership_generation = 0,
            16 => changed.character_revision = 0,
            17 => changed.occurrence_id[6] = 0x40,
            18 => changed.character_id[8] = 0xc0,
            19 => changed.world_id = changed.channel_id.clone(),
            20 => changed.entry_id = vec![0; 16],
            21 => changed.game_session_id = vec![0; 15],
            22 => {
                changed.before = Some(OneForgeDustBalanceV1 {
                    balance: 25,
                    dust_limit: 225,
                })
            }
            _ => unreachable!(),
        }
        assert_eq!(
            check_forge_dust_limit(&changed),
            Err(AuditError::InvalidInput),
            "mutation {mutation}"
        );
    }
}

#[test]
fn unknown_or_duplicate_operation_and_extra_value_field_are_not_admitted() -> Result<(), AuditError>
{
    let transaction = OneItemTransactionV1 {
        interpretation_revision: mint::INTERPRETATION_REVISION,
        operation: Some(OneItemOperationV1::ForgeDustLimit(value(0))),
    };
    let wire = transaction.encode_to_vec();
    let mut unknown = wire.clone();
    unknown.extend_from_slice(&[0x72, 0]); // unknown arm 14
    assert!(decode_forge_dust_limit_payload(&unknown).is_err());
    let mut duplicate = wire.clone();
    duplicate.extend_from_slice(&wire[2..]);
    assert!(decode_forge_dust_limit_payload(&duplicate).is_err());
    let mut operation = value(0).encode_to_vec();
    operation.extend_from_slice(&[0x98, 0x01, 0x01]); // undeclared field 19; no extra effect slot
    let mut extra = vec![0x08, 1, 0x6a]; // interpretation 1, proposed arm 13
    prost::encoding::encode_varint(operation.len() as u64, &mut extra);
    extra.extend_from_slice(&operation);
    assert!(decode_forge_dust_limit_payload(&extra).is_err());
    assert_eq!(decode_forge_dust_limit_payload(&wire)?, value(0));
    assert_eq!(
        decode_forge_dust_limit_payload(&vec![0; mint::RL07_PAYLOAD_BYTES_MAX + 1]),
        Err(AuditError::CapacityExceeded)
    );
    Ok(())
}

#[test]
fn envelope_rejects_unbound_context_noncanonical_bytes_and_oversize() -> Result<(), AuditError> {
    let wire = encode_forge_dust_limit_event(identity(), value(224), Type2EventTuple::V1)?;
    let (original, _) = decode_forge_dust_limit_envelope(&wire)?;
    for mutation in 0..8 {
        let mut changed = original.clone();
        match mutation {
            0 => changed.command_id = Some(1),
            1 => changed.connection_generation = Some(1),
            2 => changed.game_session_id = Some(id(9)),
            3 => changed.world_id = Some(id(9)),
            4 => changed.privacy_class = 4, // Registered SECURITY_SENSITIVE privacy discriminant.
            5 => changed.server_build_id = "bad\nidentity".into(),
            6 => changed.event_id = identity().transaction_id.to_vec(),
            7 => changed.operation_id = Some(id(9)),
            _ => unreachable!(),
        }
        assert!(
            decode_forge_dust_limit_envelope(&changed.encode_to_vec()).is_err(),
            "mutation {mutation}"
        );
    }
    let mut extra = wire.clone();
    extra.extend_from_slice(&[0xf0, 0x01, 1]);
    assert!(decode_forge_dust_limit_envelope(&extra).is_err());
    assert_eq!(
        decode_forge_dust_limit_envelope(&vec![0; mint::RL07_ENVELOPE_BYTES_MAX + 1]),
        Err(AuditError::CapacityExceeded)
    );
    eprintln!(
        "FORGE-LIMIT-AUDIT: last stage payload={} envelope={}",
        original.payload.len(),
        wire.len()
    );
    Ok(())
}

#[test]
fn every_paid_stage_debits_exactly_once_and_preserves_gain_spend() -> Result<(), ForgeDustError> {
    for n in 0..DUST_LIMIT_UPGRADES_MAX {
        let limit = DustLimit::from_upgrade_count(n)?;
        let cost_u16 = 25 + n;
        let cost = u32::from(cost_u16);
        let before = ForgeDustBalance::new(cost_u16, limit)?;
        let raised = before.raise_limit()?;
        assert_eq!(raised.before, before);
        assert_eq!(raised.amount, cost);
        assert_eq!(raised.after.balance(), 0);
        assert_eq!(raised.after.limit().get(), 100 + 20 * (n + 1));
        assert_eq!(raised.after.limit().upgrade_count(), n + 1);
        let gained = raised.after.gain(u32::MAX)?;
        assert_eq!(gained.after.limit(), raised.after.limit());
        assert_eq!(gained.lost, u32::MAX - u32::from(gained.after.balance()));
        assert_eq!(gained.after.spend(1)?.after.limit(), raised.after.limit());
        let short = ForgeDustBalance::new(cost_u16 - 1, limit)?;
        assert_eq!(short.raise_limit(), Err(ForgeDustError::InsufficientDust));
    }
    let full = ForgeDustBalance::new(4_600, DustLimit::from_upgrade_count(225)?)?;
    assert_eq!(full.raise_limit(), Err(ForgeDustError::MaximumLimitReached));
    assert_eq!(
        DustLimit::from_upgrade_count(226),
        Err(ForgeDustError::InvalidLimit)
    );
    assert_eq!(
        DustLimit::from_upgrade_count(u16::MAX),
        Err(ForgeDustError::InvalidLimit)
    );
    Ok(())
}
