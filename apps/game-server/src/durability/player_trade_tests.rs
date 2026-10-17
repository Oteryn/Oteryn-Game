//! Closed native event grammar and pure first-fence continuity policy.
//! These tests do not manufacture a native issuer, session, node or Root proof.

#![allow(clippy::unwrap_used)]

use super::*;
use crate::foundation::{
    ChannelId, ConnectionGeneration, GameSessionId, ScopeOwnershipGeneration, WorldId,
};

fn id(seed: u8) -> [u8; 16] {
    let mut value = [0; 16];
    value[6] = 0x70;
    value[8] = 0x80;
    value[15] = seed;
    value
}
fn command(seed: u8, number: u64) -> super::super::item_transfer_audit::OneItemCommandRefV1 {
    super::super::item_transfer_audit::OneItemCommandRefV1 {
        game_session_id: id(seed).to_vec(),
        command_id: number,
    }
}
fn sample(maximum: bool) -> OneItemPlayerTradeV1 {
    let wide = if maximum { u64::MAX } else { 4 };
    let first = command(41, wide);
    let trigger = command(42, wide);
    let participants = [(1, 11, first.clone()), (2, 12, trigger.clone())]
        .map(
            |(character, account, command)| OneItemPlayerTradeParticipantV1 {
                character_id: id(character).to_vec(),
                account_id: id(account).to_vec(),
                command: Some(command),
                accepted_connection_generation: wide,
                current_connection_generation: wide,
                character_lease_generation: wide,
            },
        )
        .to_vec();
    let lines = [(21, 1, 2, 31, 32, 71), (22, 2, 1, 32, 31, 72)]
        .map(
            |(item, source, destination, source_parent, destination_parent, old_tx)| {
                let state = OneItemStateV1 {
                    item_instance_id: id(item).to_vec(),
                    world_id: id(51).to_vec(),
                    definition: Some(OneItemTypedDefinitionRevisionV1 {
                        family: if maximum {
                            "f".repeat(128)
                        } else {
                            "Item".into()
                        },
                        production_key: if maximum {
                            "k".repeat(512)
                        } else {
                            "trade:test".into()
                        },
                        revision_ref: if maximum {
                            "r".repeat(512)
                        } else {
                            "r1".into()
                        },
                    }),
                    quantity: 1,
                    lifecycle: ITEM_LIFECYCLE_LIVE,
                    key_number: if maximum { 65535 } else { 0 },
                };
                OneItemPlayerTradeLineV1 {
                    before: Some(state.clone()),
                    after: Some(state),
                    state_revision_before: wide - 1,
                    state_revision_after: wide,
                    source: Some(OneItemPlayerTradeLocationV1 {
                        character_id: id(source).to_vec(),
                        parent_item_instance_id: id(source_parent).to_vec(),
                        placement_ordinal: wide - 1,
                        placed_transaction_id: id(old_tx).to_vec(),
                    }),
                    destination: Some(OneItemPlayerTradeLocationV1 {
                        character_id: id(destination).to_vec(),
                        parent_item_instance_id: id(destination_parent).to_vec(),
                        placement_ordinal: wide,
                        placed_transaction_id: id(80).to_vec(),
                    }),
                    unchanged_timed_state: maximum.then(|| OneItemPlayerTradeTimedV1 {
                        row: Some(super::super::item_timed_state_audit::OneItemTimedRowV1 {
                            charges: Some(65535),
                            remaining_ms: Some(604800000),
                            revision: u64::MAX,
                        }),
                        deadline_at: Some(i64::MAX),
                    }),
                }
            },
        )
        .to_vec();
    OneItemPlayerTradeV1 {
        typed_cause: PLAYER_TRADE_CAUSE.into(),
        trade_id: id(61).to_vec(),
        occurrence_id: id(62).to_vec(),
        first_accept: Some(first),
        trigger: Some(trigger),
        world_id: id(51).to_vec(),
        channel_id: id(52).to_vec(),
        runtime_scope_ownership_generation: wide,
        content_digest: vec![7; 32],
        participants,
        lines,
    }
}
fn identity() -> PlayerTradeEventIdentity {
    PlayerTradeEventIdentity {
        transaction_id: id(80),
        event_id: id(81),
        occurred_at_unix_ms: 1_700_000_000_000,
    }
}

#[test]
fn player_trade_native_event_round_trip_and_measured_maximum_fit_existing_tuple() {
    for maximum in [false, true] {
        let value = sample(maximum);
        let bytes =
            encode_player_trade_event(identity(), value.clone(), audit::Type2EventTuple::V1)
                .unwrap();
        let (envelope, decoded) = decode_player_trade_envelope(&bytes).unwrap();
        assert_eq!(decoded, value);
        assert_eq!(
            envelope.command_id,
            Some(if maximum { u64::MAX } else { 4 })
        );
        assert!(envelope.payload.len() <= audit::RL07_PAYLOAD_BYTES_MAX);
        assert!(bytes.len() <= audit::RL07_ENVELOPE_BYTES_MAX);
        if maximum {
            eprintln!(
                "TRADE-TYPE2-MAX payload_bytes={} envelope_bytes={} items={} locations={} participants={} effects={}",
                envelope.payload.len(),
                bytes.len(),
                PLAYER_TRADE_TOUCHED_ITEMS,
                PLAYER_TRADE_LOCATION_LINES,
                PLAYER_TRADE_PARTICIPANTS,
                PLAYER_TRADE_EFFECT_WORK_UNITS
            );
        }
    }
}

#[test]
fn player_trade_event_preserves_zero_remaining_timed_row_and_v2_tuple() {
    let mut value = sample(true);
    let timed = value.lines[0].unchanged_timed_state.as_mut().unwrap();
    timed.row.as_mut().unwrap().remaining_ms = Some(0);
    timed.deadline_at = None;
    let bytes =
        encode_player_trade_event(identity(), value.clone(), audit::Type2EventTuple::V2).unwrap();
    let (envelope, decoded) = decode_player_trade_envelope(&bytes).unwrap();
    assert_eq!(decoded, value);
    assert_eq!(
        envelope.event_schema_revision,
        audit::Type2EventTuple::V2.schema_revision()
    );
    assert_eq!(
        envelope.retention_profile_id,
        audit::Type2EventTuple::V2.retention_profile_id()
    );
    let mut invalid = value;
    invalid.lines[0]
        .unchanged_timed_state
        .as_mut()
        .unwrap()
        .row
        .as_mut()
        .unwrap()
        .charges = Some(0);
    assert!(matches!(
        check_player_trade(&invalid),
        Err(AuditError::InvalidInput)
    ));
}

#[test]
fn player_trade_event_refuses_independent_state_identity_and_conservation_changes() {
    let valid = sample(false);
    check_player_trade(&valid).unwrap();
    let mut changes = Vec::new();
    let mut x = valid.clone();
    x.lines[0].after.as_mut().unwrap().quantity = 2;
    changes.push(x);
    let mut x = valid.clone();
    x.lines[0].state_revision_after += 1;
    changes.push(x);
    let mut x = valid.clone();
    x.lines[0].destination.as_mut().unwrap().character_id = id(3).to_vec();
    changes.push(x);
    let mut x = valid.clone();
    x.lines[0]
        .destination
        .as_mut()
        .unwrap()
        .parent_item_instance_id = id(33).to_vec();
    changes.push(x);
    let mut x = valid.clone();
    x.lines[0].before.as_mut().unwrap().world_id = id(53).to_vec();
    x.lines[0].after = x.lines[0].before.clone();
    changes.push(x);
    let mut x = valid.clone();
    x.participants[1].account_id = x.participants[0].account_id.clone();
    changes.push(x);
    let mut x = valid.clone();
    x.lines.swap(0, 1);
    changes.push(x);
    let mut x = valid.clone();
    x.participants.swap(0, 1);
    changes.push(x);
    let mut x = valid.clone();
    x.content_digest.clear();
    changes.push(x);
    let mut x = valid.clone();
    x.typed_cause = "npc_trade".into();
    changes.push(x);
    for (index, x) in changes.into_iter().enumerate() {
        assert!(
            matches!(check_player_trade(&x), Err(AuditError::InvalidInput)),
            "independent mutation {index}"
        );
    }
}

#[test]
fn player_trade_event_only_first_accept_may_advance_connection_generation() {
    let mut value = sample(false);
    value.participants[0].current_connection_generation += 1;
    check_player_trade(&value).unwrap();
    let bytes =
        encode_player_trade_event(identity(), value.clone(), audit::Type2EventTuple::V1).unwrap();
    assert_eq!(decode_player_trade_envelope(&bytes).unwrap().1, value);
    let mut changed = value.clone();
    changed.participants[1].current_connection_generation += 1;
    assert!(matches!(
        check_player_trade(&changed),
        Err(AuditError::InvalidInput)
    ));
    let mut changed = value;
    changed.participants[0].current_connection_generation = 3;
    assert!(matches!(
        check_player_trade(&changed),
        Err(AuditError::InvalidInput)
    ));
}

#[test]
fn player_trade_decoder_refuses_noncanonical_oversize_and_wrong_envelope_binding() {
    let bytes =
        encode_player_trade_event(identity(), sample(false), audit::Type2EventTuple::V1).unwrap();
    let (mut envelope, _) = decode_player_trade_envelope(&bytes).unwrap();
    let mut unknown = envelope.payload.clone();
    unknown.extend_from_slice(&[0x98, 0x06, 0x01]);
    assert!(matches!(
        decode_player_trade_payload(&unknown),
        Err(AuditError::InvalidInput)
    ));
    let mut duplicate = envelope.payload.clone();
    duplicate.extend_from_slice(&[0x08, 0x01]);
    assert!(matches!(
        decode_player_trade_payload(&duplicate),
        Err(AuditError::InvalidInput)
    ));
    assert!(matches!(
        decode_player_trade_payload(&vec![0; audit::RL07_PAYLOAD_BYTES_MAX + 1]),
        Err(AuditError::CapacityExceeded)
    ));
    envelope.command_id = Some(5);
    assert!(matches!(
        decode_player_trade_envelope(&envelope.encode_to_vec()),
        Err(AuditError::InvalidInput)
    ));
    let (mut envelope, _) = decode_player_trade_envelope(&bytes).unwrap();
    envelope.transaction_event.as_mut().unwrap().transaction_id = id(82).to_vec();
    assert!(matches!(
        decode_player_trade_envelope(&envelope.encode_to_vec()),
        Err(AuditError::InvalidInput)
    ));
}

#[test]
fn first_accept_continuity_pins_every_component_except_forward_connection() {
    let expected = CurrentCharacterItemFence {
        character_id: CharacterId::from_bytes(id(1)).unwrap(),
        game_session_id: GameSessionId::decode(&id(41)).unwrap(),
        connection_generation: ConnectionGeneration::new(2).unwrap(),
        character_lease_generation: 3,
        runtime_scope: RuntimeScopeRefV1::channel(
            WorldId::decode(&id(51)).unwrap(),
            ChannelId::decode(&id(52)).unwrap(),
        ),
        scope_ownership_generation: ScopeOwnershipGeneration::new(4).unwrap(),
    };
    assert!(first_fence_continues(expected, expected));
    let mut resumed = expected;
    resumed.connection_generation = ConnectionGeneration::new(u64::MAX).unwrap();
    assert!(first_fence_continues(expected, resumed));
    for invariant in 0..6 {
        let mut changed = resumed;
        match invariant {
            0 => changed.character_id = CharacterId::from_bytes(id(2)).unwrap(),
            1 => changed.game_session_id = GameSessionId::decode(&id(42)).unwrap(),
            2 => changed.character_lease_generation = 4,
            3 => {
                changed.runtime_scope = RuntimeScopeRefV1::channel(
                    WorldId::decode(&id(53)).unwrap(),
                    ChannelId::decode(&id(52)).unwrap(),
                )
            }
            4 => changed.scope_ownership_generation = ScopeOwnershipGeneration::new(5).unwrap(),
            _ => changed.connection_generation = ConnectionGeneration::new(1).unwrap(),
        }
        assert!(
            !first_fence_continues(expected, changed),
            "invariant {invariant}"
        );
    }
}
