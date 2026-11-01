#![allow(clippy::expect_used, clippy::unwrap_used)]

use super::*;
use crate::durability::item_mint_audit::{
    self as common, OneItemTypedDefinitionRevisionV1, Type2EventTuple,
};
use crate::durability::item_transfer_audit::OneItemCommandRefV1;
use crate::durability::reward_claim_mint_audit::{
    OneItemRewardClaimCauseV1, RewardClaimMintEventIdentity, decode_reward_claim_mint_envelope,
    decode_reward_claim_mint_payload, encode_reward_claim_mint_event,
};

fn id(tag: u8) -> Vec<u8> {
    let mut bytes = vec![0; 16];
    bytes[6] = 0x70;
    bytes[8] = 0x80;
    bytes[15] = tag;
    bytes
}

fn definition() -> OneItemTypedDefinitionRevisionV1 {
    OneItemTypedDefinitionRevisionV1 {
        family: "Item".into(),
        production_key: "fixture:reward".into(),
        revision_ref: "r1".into(),
    }
}

fn batch(count: usize) -> OneItemRewardClaimMintV1 {
    OneItemRewardClaimMintV1 {
        after: None,
        destination: None,
        before_semantically_absent: true,
        source: Some(OneItemRewardClaimCauseV1 {
            boss_reward: None,
            typed_cause: REWARD_CLAIM_MINT_TYPED_CAUSE.into(),
            content_revision: "content-r1".into(),
            ruleset_revision: "rules-r1".into(),
            sim_revision: "sim-r1".into(),
            command_ref: Some(OneItemCommandRefV1 {
                game_session_id: id(3),
                command_id: u64::MAX,
            }),
            reward_claim: Some(definition()),
        }),
        inbox: Some(RewardClaimInboxMintV1 {
            character_id: id(1),
            world_id: id(2),
            expected_session_generation: 9,
            source_binding: vec![7; 32],
            plan_binding: vec![8; 32],
            mint_intent_sha256: vec![9; 32],
            expected_character_lease_generation: 11,
            house_key: None,
            house_scope_ownership_generation: None,
            house_authority_kind: None,
            outputs: (0..count)
                .map(|index| RewardClaimInboxMintOutputV1 {
                    after: Some(OneItemStateV1 {
                        item_instance_id: id(index as u8 + 20),
                        world_id: id(2),
                        definition: Some(definition()),
                        quantity: 1,
                        lifecycle: ITEM_LIFECYCLE_LIVE,
                        key_number: 0,
                    }),
                    ordinal: index as u64 + 1,
                    source_line: index as u32,
                })
                .collect(),
        }),
    }
}

fn identity() -> RewardClaimMintEventIdentity<'static> {
    RewardClaimMintEventIdentity {
        event_id: id(5).try_into().unwrap(),
        transaction_id: id(6).try_into().unwrap(),
        occurred_at_unix_ms: 1_700_000_000_000,
        channel_id: id(4).try_into().unwrap(),
        server_build_id: "oteryn-game-server/test",
    }
}

#[test]
fn inbox_batch_reuses_tag4_and_round_trips_each_admitted_tuple() {
    for count in [1, 2, REWARD_INBOX_ITEMS_MAX] {
        for tuple in [Type2EventTuple::V1, Type2EventTuple::V2] {
            let expected = batch(count);
            let wire = encode_reward_claim_mint_event(identity(), expected.clone(), tuple).unwrap();
            let (envelope, actual) = decode_reward_claim_mint_envelope(&wire).unwrap();
            assert_eq!(actual, expected);
            assert_eq!(envelope.event_schema_revision, tuple.schema_revision());
            assert_eq!(envelope.retention_profile_id, tuple.retention_profile_id());
            assert_eq!(envelope.connection_generation, Some(9));
            assert_eq!(envelope.command_id, Some(u64::MAX));
            assert_eq!(envelope.transaction_event.as_ref().unwrap().count, 1);
            let transaction =
                common::OneItemTransactionV1::decode(envelope.payload.as_slice()).unwrap();
            assert!(matches!(
                transaction.operation,
                Some(common::OneItemOperationV1::RewardClaimMint(_))
            ));
            assert_eq!(
                encode_reward_claim_mint_event(identity(), expected, tuple).unwrap(),
                wire
            );
            assert!(common::decode_envelope(&wire).is_err());
        }
    }
}

#[test]
fn house_inbox_audit_distinguishes_live_fence_from_offline_provenance() {
    let legacy = batch(1);
    assert!(legacy.inbox.as_ref().unwrap().house_key.is_none());
    for kind in [1, 2] {
        let mut house = legacy.clone();
        let inbox = house.inbox.as_mut().unwrap();
        inbox.house_key = Some("oteryn:content.house.fixture".into());
        inbox.house_scope_ownership_generation = Some(19);
        inbox.house_authority_kind = Some(kind);
        if kind == 2 {
            inbox.outputs[0].after.as_mut().unwrap().definition.as_mut().unwrap().production_key =
                crate::durability::house_hireling::HIRELING_LAMP_KEY.into();
            house.source.as_mut().unwrap().reward_claim.as_mut().unwrap().production_key =
                crate::durability::house_hireling::HIRELING_LAMP_KEY.into();
        }
        let bytes = encode_reward_claim_mint_event(identity(), house.clone(), Type2EventTuple::V2).unwrap();
        let (_, decoded) = decode_reward_claim_mint_envelope(&bytes).unwrap();
        assert_eq!(decoded, house);
        for changed in 0..5 {
            let mut invalid = house.clone();
            let scope = invalid.inbox.as_mut().unwrap();
            match changed {
                0 => scope.house_key = None,
                1 => scope.house_scope_ownership_generation = Some(0),
                2 => scope.house_authority_kind = Some(3),
                3 => scope.house_key = Some("oteryn:activity.instance.fixture".into()),
                _ => scope.house_scope_ownership_generation = None,
            }
            assert_eq!(check_inbox(&invalid), Err(AuditError::InvalidInput));
        }
        if kind == 2 {
            let mut invalid = house;
            invalid.inbox.as_mut().unwrap().outputs[0].after.as_mut().unwrap().quantity = 2;
            assert_eq!(check_inbox(&invalid), Err(AuditError::InvalidInput));
        }
    }
}

#[test]
fn inbox_rejects_empty_oversize_mixed_duplicate_and_foreign_outputs() {
    assert_eq!(check_inbox(&batch(0)), Err(AuditError::InvalidInput));
    assert_eq!(check_inbox(&batch(21)), Err(AuditError::CapacityExceeded));
    type Mutation = Box<dyn Fn(&mut OneItemRewardClaimMintV1)>;
    for mutate in [
        Box::new(|value: &mut OneItemRewardClaimMintV1| {
            value.after = value.inbox.as_ref().unwrap().outputs[0].after.clone()
        }) as Mutation,
        Box::new(|value| value.before_semantically_absent = false),
        Box::new(|value| value.source.as_mut().unwrap().typed_cause = "daily_reward".into()),
        Box::new(|value| {
            value
                .source
                .as_mut()
                .unwrap()
                .command_ref
                .as_mut()
                .unwrap()
                .command_id = 0
        }),
        Box::new(|value| value.inbox.as_mut().unwrap().expected_session_generation = 0),
        Box::new(|value| {
            value
                .inbox
                .as_mut()
                .unwrap()
                .expected_character_lease_generation = 0
        }),
        Box::new(|value| {
            value
                .inbox
                .as_mut()
                .unwrap()
                .source_binding
                .pop()
                .map(|_| ())
                .unwrap()
        }),
        Box::new(|value| value.inbox.as_mut().unwrap().plan_binding.push(0)),
        Box::new(|value| value.inbox.as_mut().unwrap().mint_intent_sha256.clear()),
        Box::new(|value| value.inbox.as_mut().unwrap().outputs[0].ordinal = 0),
        Box::new(|value| value.inbox.as_mut().unwrap().outputs[1].ordinal = 1),
        Box::new(|value| {
            value.inbox.as_mut().unwrap().outputs[1]
                .after
                .as_mut()
                .unwrap()
                .item_instance_id = id(20)
        }),
        Box::new(|value| {
            value.inbox.as_mut().unwrap().outputs[1]
                .after
                .as_mut()
                .unwrap()
                .world_id = id(99)
        }),
        Box::new(|value| {
            value.inbox.as_mut().unwrap().outputs[1]
                .after
                .as_mut()
                .unwrap()
                .quantity = 0
        }),
        Box::new(|value| {
            value.inbox.as_mut().unwrap().outputs[1]
                .after
                .as_mut()
                .unwrap()
                .quantity = 101
        }),
        Box::new(|value| {
            value.inbox.as_mut().unwrap().outputs[1]
                .after
                .as_mut()
                .unwrap()
                .lifecycle = 2
        }),
    ] {
        let mut changed = batch(2);
        mutate(&mut changed);
        assert!(check_inbox(&changed).is_err());
    }
}

#[test]
fn inbox_envelope_binds_world_generation_session_and_full_command() {
    let bytes = encode_reward_claim_mint_event(identity(), batch(2), Type2EventTuple::V1).unwrap();
    for index in 0..4 {
        let mut event = common::EventEnvelopeV1::decode(bytes.as_slice()).unwrap();
        match index {
            0 => event.world_id = Some(id(99)),
            1 => event.connection_generation = Some(10),
            2 => event.game_session_id = Some(id(99)),
            _ => event.command_id = Some(u64::MAX - 1),
        }
        assert!(decode_reward_claim_mint_envelope(&event.encode_to_vec()).is_err());
    }
}

#[test]
fn inbox_maximum_fields_fit_its_own_byte_and_resource_bounds() {
    let mut value = batch(REWARD_INBOX_ITEMS_MAX);
    let maximum = OneItemTypedDefinitionRevisionV1 {
        family: "f".repeat(128),
        production_key: "k".repeat(512),
        revision_ref: "r".repeat(512),
    };
    let source = value.source.as_mut().unwrap();
    source.reward_claim = Some(maximum.clone());
    source.content_revision = "c".repeat(512);
    source.ruleset_revision = "r".repeat(512);
    source.sim_revision = "s".repeat(512);
    value.inbox.as_mut().unwrap().expected_session_generation = u64::MAX;
    value
        .inbox
        .as_mut()
        .unwrap()
        .expected_character_lease_generation = u64::MAX;
    for (index, output) in value.inbox.as_mut().unwrap().outputs.iter_mut().enumerate() {
        output.ordinal = u64::MAX - 19 + index as u64;
        output.source_line = u32::MAX;
        let after = output.after.as_mut().unwrap();
        after.definition = Some(maximum.clone());
        after.key_number = u32::from(u16::MAX);
    }
    let house = format!("oteryn:content.house.{}",
        "a".repeat(512 - "oteryn:content.house.".len()));
    let mut live_house = value.clone();
    let live_scope = live_house.inbox.as_mut().unwrap();
    live_scope.house_key = Some(house);
    live_scope.house_scope_ownership_generation = Some(u64::MAX);
    live_scope.house_authority_kind = Some(1);
    let mut recall = live_house.clone();
    let lamp = OneItemTypedDefinitionRevisionV1 { family: "Item".into(),
        production_key: crate::durability::house_hireling::HIRELING_LAMP_KEY.into(),
        revision_ref: "r".repeat(512) };
    recall.source.as_mut().unwrap().reward_claim = Some(lamp.clone());
    let recall_scope = recall.inbox.as_mut().unwrap();
    recall_scope.house_authority_kind = Some(2);
    recall_scope.outputs.truncate(1);
    recall_scope.outputs[0].ordinal = u64::MAX;
    let after = recall_scope.outputs[0].after.as_mut().unwrap();
    after.definition = Some(lamp);
    after.key_number = 0;
    let build = "b".repeat(128);
    for (scope, value) in [("Channel", value), ("House", live_house), ("Recall", recall)] {
        let output_count = value.inbox.as_ref().unwrap().outputs.len();
        let payload_len = common::OneItemTransactionV1 {
            interpretation_revision: common::INTERPRETATION_REVISION,
            operation: Some(common::OneItemOperationV1::RewardClaimMint(value.clone())),
        }.encoded_len();
        for tuple in [Type2EventTuple::V1, Type2EventTuple::V2] {
            let event_identity = RewardClaimMintEventIdentity { server_build_id: &build,
                occurred_at_unix_ms: i64::MAX - common::AUDIT_RETENTION_P90D_MS, ..identity() };
            let wire = encode_reward_claim_mint_event(event_identity, value.clone(), tuple).unwrap();
            if output_count == 20 { assert!(wire.len() > common::RL07_ENVELOPE_BYTES_MAX); }
            assert!(wire.len() <= REWARD_INBOX_ENVELOPE_BYTES_MAX);
            let (envelope, decoded) = decode_reward_claim_mint_envelope(&wire).unwrap();
            assert_eq!(decoded, value);
            assert_eq!(payload_len, envelope.payload.len());
            assert_eq!(envelope.encoded_len(), wire.len());
            assert!(payload_len <= REWARD_INBOX_PAYLOAD_BYTES_MAX);
            eprintln!("reward Inbox native prost cap measurement: scope={scope} outputs={output_count} schema={} payload_bytes={payload_len} envelope_bytes={} payload_cap={} envelope_cap={}",
                tuple.schema_revision(), wire.len(), REWARD_INBOX_PAYLOAD_BYTES_MAX, REWARD_INBOX_ENVELOPE_BYTES_MAX);
        }
    }
    assert_eq!(reward_inbox_usage(20).unwrap().effect_work_units, 41);
    assert_eq!(reward_inbox_usage(21), Err(AuditError::CapacityExceeded));
    assert_eq!(
        decode_reward_claim_mint_payload(&vec![0xff; REWARD_INBOX_PAYLOAD_BYTES_MAX + 1]),
        Err(AuditError::CapacityExceeded)
    );
    assert_eq!(
        decode_reward_claim_mint_envelope(&vec![0xff; REWARD_INBOX_ENVELOPE_BYTES_MAX + 1]),
        Err(AuditError::CapacityExceeded)
    );
}
