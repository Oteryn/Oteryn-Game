//! Registered DUR-03 reward-claim MINT durable-audit encoding: event type 2
//! (`oteryn.events.v1.OneItemTransactionV1`, operation `reward_claim_mint`,
//! oneof tag 4) in the normative ANL-01 `EventEnvelope`, for the CHEST-1
//! shape of the reward chest decisions (§5.1): one fresh live item placed in a
//! new direct entry of the equipped main backpack for a `once` RewardClaim.
//!
//! There is no Ground and no creature-death source. The cause is the actual
//! player USE CommandRef and the claimed RewardClaim definition. Every
//! registered bound is checked before encode allocation and oversize input is
//! rejected, never truncated.

use super::item_mint_audit::{
    self as mint, AuditError, CausationRefV1, CausationV1, EnvelopeCommandRefV1, EventEnvelopeV1,
    ITEM_LIFECYCLE_LIVE, OneItemOperationV1, OneItemStateV1, OneItemTransactionV1,
    OneItemTypedDefinitionRevisionV1, RL07_ENVELOPE_BYTES_MAX, RL07_PAYLOAD_BYTES_MAX,
    TransactionEventRefV1, TransactionResourceUsage, check_content_key, check_definition,
    check_technical_text, check_uuid_v7, encode_bounded,
};
use super::item_transfer_audit::{
    GAMEITEM01_STACK_QUANTITY_MAX, OneItemCommandRefV1, OneItemInventoryV1,
};
use prost::Message;
use sha2::{Digest, Sha256};

/// Typed cause of a reward-claim MINT (D40).
pub const REWARD_CLAIM_MINT_TYPED_CAUSE: &str = "reward_claim_mint";

/// Reward-claim MINT cause. Same field numbers as `OneItemTransferCauseV1`
/// for the shared fields; field 10 is the claimed RewardClaim definition.
#[derive(Clone, PartialEq, Eq, Message)]
pub struct OneItemRewardClaimCauseV1 {
    #[prost(string, tag = "1")]
    pub typed_cause: String,
    #[prost(string, tag = "6")]
    pub content_revision: String,
    #[prost(string, tag = "7")]
    pub ruleset_revision: String,
    #[prost(string, tag = "8")]
    pub sim_revision: String,
    #[prost(message, optional, tag = "9")]
    pub command_ref: Option<OneItemCommandRefV1>,
    #[prost(message, optional, tag = "10")]
    pub reward_claim: Option<OneItemTypedDefinitionRevisionV1>,
}

#[derive(Clone, PartialEq, Eq, Message)]
pub struct OneItemRewardClaimMintV1 {
    #[prost(message, optional, tag = "1")]
    pub after: Option<OneItemStateV1>,
    #[prost(message, optional, tag = "2")]
    pub destination: Option<OneItemInventoryV1>,
    #[prost(message, optional, tag = "3")]
    pub source: Option<OneItemRewardClaimCauseV1>,
    #[prost(bool, tag = "4")]
    pub before_semantically_absent: bool,
}

/// One fresh ItemInstance, one new backpack entry line, one participant
/// whose work is the participant, its placement and the RewardClaim record,
/// one event.
pub const REWARD_CLAIM_MINT_USAGE: TransactionResourceUsage = TransactionResourceUsage {
    touched_item_instances: 1,
    location_custody_lines: 1,
    value_lines: 0,
    transform_lines: 0,
    container_expansion: 0,
    participants: 1,
    effect_work_units: 3,
    events: 1,
};

/// Complete closed CHEST-1 shape and every registered per-field bound.
pub fn check_reward_claim_mint(value: &OneItemRewardClaimMintV1) -> Result<(), AuditError> {
    let after = value.after.as_ref().ok_or(AuditError::InvalidInput)?;
    let destination = value.destination.as_ref().ok_or(AuditError::InvalidInput)?;
    let source = value.source.as_ref().ok_or(AuditError::InvalidInput)?;
    let command = source
        .command_ref
        .as_ref()
        .ok_or(AuditError::InvalidInput)?;
    let entry = destination
        .container_entry
        .as_ref()
        .ok_or(AuditError::InvalidInput)?;
    check_uuid_v7(&after.item_instance_id)?;
    check_uuid_v7(&after.world_id)?;
    check_definition(after.definition.as_ref())?;
    check_uuid_v7(&destination.character_id)?;
    check_uuid_v7(&destination.expected_game_session_id)?;
    check_uuid_v7(&entry.parent_item_instance_id)?;
    check_technical_text(&source.typed_cause)?;
    check_content_key(&source.content_revision)?;
    check_content_key(&source.ruleset_revision)?;
    check_content_key(&source.sim_revision)?;
    check_uuid_v7(&command.game_session_id)?;
    check_definition(source.reward_claim.as_ref())?;
    if !value.before_semantically_absent
        || after.lifecycle != ITEM_LIFECYCLE_LIVE
        || after.quantity == 0
        || after.quantity > GAMEITEM01_STACK_QUANTITY_MAX
        || source.typed_cause != REWARD_CLAIM_MINT_TYPED_CAUSE
        || destination.equipment_container_slot
        || entry.placement_ordinal == 0
        || entry.parent_item_instance_id == after.item_instance_id
        || command.command_id == 0
        || command.game_session_id != destination.expected_game_session_id
        || destination.expected_session_generation == 0
    {
        return Err(AuditError::InvalidInput);
    }
    REWARD_CLAIM_MINT_USAGE.check()
}

pub fn decode_reward_claim_mint_payload(
    wire: &[u8],
) -> Result<OneItemRewardClaimMintV1, AuditError> {
    if wire.len() > RL07_PAYLOAD_BYTES_MAX {
        return Err(AuditError::CapacityExceeded);
    }
    let value = OneItemTransactionV1::decode(wire).map_err(|_| AuditError::InvalidInput)?;
    if value.encode_to_vec() != wire
        || value.interpretation_revision != mint::INTERPRETATION_REVISION
    {
        return Err(AuditError::InvalidInput);
    }
    let Some(OneItemOperationV1::RewardClaimMint(minted)) = value.operation else {
        return Err(AuditError::InvalidInput);
    };
    check_reward_claim_mint(&minted)?;
    Ok(minted)
}

/// Envelope gate: the common RL-07 gate, then the World of the minted item,
/// a Channel, and the command, session and causation of the payload cause.
pub fn decode_reward_claim_mint_envelope(
    wire: &[u8],
) -> Result<(EventEnvelopeV1, OneItemRewardClaimMintV1), AuditError> {
    let value = mint::decode_common_envelope(wire)?;
    let minted = decode_reward_claim_mint_payload(&value.payload)?;
    let after = minted.after.as_ref().ok_or(AuditError::InvalidInput)?;
    let destination = minted
        .destination
        .as_ref()
        .ok_or(AuditError::InvalidInput)?;
    let command = minted
        .source
        .as_ref()
        .and_then(|source| source.command_ref.as_ref())
        .ok_or(AuditError::InvalidInput)?;
    let channel_id = value
        .channel_id
        .as_deref()
        .ok_or(AuditError::InvalidInput)?;
    check_uuid_v7(channel_id)?;
    let envelope_command = CausationRefV1 {
        cause: Some(CausationV1::Command(EnvelopeCommandRefV1 {
            game_session_id: command.game_session_id.clone(),
            command_id: command.command_id,
        })),
    };
    if value.world_id.as_deref() != Some(after.world_id.as_slice())
        || value.game_session_id.as_deref() != Some(command.game_session_id.as_slice())
        || value.command_id != Some(command.command_id)
        || value.connection_generation != Some(destination.expected_session_generation)
        || value.causation.as_ref() != Some(&envelope_command)
    {
        return Err(AuditError::InvalidInput);
    }
    Ok((value, minted))
}

/// Envelope identity fixed before the first commit attempt, with the fenced
/// Channel (the payload carries no Ground).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RewardClaimMintEventIdentity<'a> {
    pub event_id: [u8; 16],
    pub transaction_id: [u8; 16],
    pub occurred_at_unix_ms: i64,
    pub channel_id: [u8; 16],
    pub server_build_id: &'a str,
}

/// Build the exact immutable event bytes for one reward-claim MINT. The
/// returned envelope is re-decoded through the registered gates first.
pub fn encode_reward_claim_mint_event(
    identity: RewardClaimMintEventIdentity<'_>,
    minted: OneItemRewardClaimMintV1,
) -> Result<Vec<u8>, AuditError> {
    check_uuid_v7(&identity.event_id)?;
    check_uuid_v7(&identity.transaction_id)?;
    check_uuid_v7(&identity.channel_id)?;
    check_technical_text(identity.server_build_id)?;
    if identity.occurred_at_unix_ms <= 0 {
        return Err(AuditError::InvalidInput);
    }
    check_reward_claim_mint(&minted)?;
    let world_id = minted
        .after
        .as_ref()
        .map(|after| after.world_id.clone())
        .ok_or(AuditError::InvalidInput)?;
    let command = minted
        .source
        .as_ref()
        .and_then(|source| source.command_ref.clone())
        .ok_or(AuditError::InvalidInput)?;
    let connection_generation = minted
        .destination
        .as_ref()
        .map(|destination| destination.expected_session_generation)
        .ok_or(AuditError::InvalidInput)?;
    let payload = encode_bounded(
        &OneItemTransactionV1 {
            interpretation_revision: mint::INTERPRETATION_REVISION,
            operation: Some(OneItemOperationV1::RewardClaimMint(minted)),
        },
        RL07_PAYLOAD_BYTES_MAX,
    )?;
    let envelope = EventEnvelopeV1 {
        envelope_revision: mint::ENVELOPE_REVISION,
        event_id: identity.event_id.to_vec(),
        event_type_id: mint::EVENT_TYPE_ID,
        event_schema_revision: mint::EVENT_SCHEMA_REVISION,
        durability_class: mint::DURABLE_AUDIT,
        privacy_class: mint::RESTRICTED_PLAYER_LINKED,
        retention_profile_id: mint::RETENTION_PROFILE_ID.into(),
        occurred_at_unix_ms: identity.occurred_at_unix_ms,
        world_id: Some(world_id),
        channel_id: Some(identity.channel_id.to_vec()),
        game_session_id: Some(command.game_session_id.clone()),
        connection_generation: Some(connection_generation),
        command_id: Some(command.command_id),
        transaction_event: Some(TransactionEventRefV1 {
            transaction_id: identity.transaction_id.to_vec(),
            ordinal: 1,
            count: 1,
        }),
        causation: Some(CausationRefV1 {
            cause: Some(CausationV1::Command(EnvelopeCommandRefV1 {
                game_session_id: command.game_session_id,
                command_id: command.command_id,
            })),
        }),
        server_build_id: identity.server_build_id.into(),
        payload_sha256: Sha256::digest(&payload).to_vec(),
        payload,
        ..EventEnvelopeV1::default()
    };
    let wire = encode_bounded(&envelope, RL07_ENVELOPE_BYTES_MAX)?;
    let (decoded, _) = decode_reward_claim_mint_envelope(&wire)?;
    if decoded != envelope {
        return Err(AuditError::InvalidInput);
    }
    Ok(wire)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]
    use super::super::item_mint_audit::{
        RL07_CONTENT_KEY_BYTES_MAX, decode_envelope, decode_payload,
    };
    use super::super::item_transfer_audit::{
        OneItemContainerEntryV1, decode_transfer_envelope, decode_transfer_payload,
    };
    use super::*;

    fn uuid(tag: u8) -> Vec<u8> {
        let mut bytes = vec![0; 16];
        bytes[5] = 1;
        bytes[6] = 0x70;
        bytes[8] = 0x80;
        bytes[15] = tag;
        bytes
    }

    fn fixed(tag: u8) -> [u8; 16] {
        uuid(tag).try_into().unwrap()
    }

    fn definition(family: &str, key: &str) -> OneItemTypedDefinitionRevisionV1 {
        OneItemTypedDefinitionRevisionV1 {
            family: family.into(),
            production_key: key.into(),
            revision_ref: "definition-r1".into(),
        }
    }

    fn minted() -> OneItemRewardClaimMintV1 {
        OneItemRewardClaimMintV1 {
            after: Some(OneItemStateV1 {
                item_instance_id: uuid(9),
                world_id: uuid(1),
                definition: Some(definition("Item", "fixture:chest.coin")),
                quantity: 30,
                lifecycle: ITEM_LIFECYCLE_LIVE,
            }),
            destination: Some(OneItemInventoryV1 {
                character_id: uuid(41),
                expected_session_generation: 1,
                expected_game_session_id: uuid(50),
                expected_character_lease_generation: 1,
                container_entry: Some(OneItemContainerEntryV1 {
                    parent_item_instance_id: uuid(20),
                    placement_ordinal: 3,
                }),
                equipment_container_slot: false,
            }),
            source: Some(OneItemRewardClaimCauseV1 {
                typed_cause: REWARD_CLAIM_MINT_TYPED_CAUSE.into(),
                content_revision: "content-1".into(),
                ruleset_revision: "ruleset-1".into(),
                sim_revision: "sim-1".into(),
                command_ref: Some(OneItemCommandRefV1 {
                    game_session_id: uuid(50),
                    command_id: 7,
                }),
                reward_claim: Some(definition("RewardClaim", "fixture:chest.claim")),
            }),
            before_semantically_absent: true,
        }
    }

    fn identity() -> RewardClaimMintEventIdentity<'static> {
        RewardClaimMintEventIdentity {
            event_id: fixed(30),
            transaction_id: fixed(31),
            occurred_at_unix_ms: 1_700_000_000_000,
            channel_id: fixed(2),
            server_build_id: "test-build",
        }
    }

    #[test]
    fn encodes_and_round_trips_through_the_registered_gates() {
        let wire = encode_reward_claim_mint_event(identity(), minted()).unwrap();
        let (envelope, decoded) = decode_reward_claim_mint_envelope(&wire).unwrap();
        assert_eq!(decoded, minted());
        assert_eq!(envelope.channel_id, Some(uuid(2)));
        assert_eq!(envelope.world_id, Some(uuid(1)));
        assert_eq!(envelope.command_id, Some(7));
        // The MINT and TRANSFER gates reject the reward-claim operation.
        assert!(decode_payload(&envelope.payload).is_err());
        assert!(decode_transfer_payload(&envelope.payload).is_err());
        assert!(decode_envelope(&wire).is_err());
        assert!(decode_transfer_envelope(&wire).is_err());
    }

    #[test]
    fn rejects_every_open_shape() {
        type Mutation = Box<dyn Fn(&mut OneItemRewardClaimMintV1)>;
        let cases: Vec<(&str, Mutation)> = vec![
            (
                "prior item",
                Box::new(|v| v.before_semantically_absent = false),
            ),
            (
                "zero quantity",
                Box::new(|v| v.after.as_mut().unwrap().quantity = 0),
            ),
            (
                "stack above 100",
                Box::new(|v| v.after.as_mut().unwrap().quantity = 101),
            ),
            (
                "retired",
                Box::new(|v| v.after.as_mut().unwrap().lifecycle = 2),
            ),
            (
                "other cause",
                Box::new(|v| v.source.as_mut().unwrap().typed_cause = "loot_mint".into()),
            ),
            (
                "container slot",
                Box::new(|v| v.destination.as_mut().unwrap().equipment_container_slot = true),
            ),
            (
                "no entry",
                Box::new(|v| v.destination.as_mut().unwrap().container_entry = None),
            ),
            (
                "zero ordinal",
                Box::new(|v| {
                    v.destination
                        .as_mut()
                        .unwrap()
                        .container_entry
                        .as_mut()
                        .unwrap()
                        .placement_ordinal = 0;
                }),
            ),
            (
                "item is its own parent",
                Box::new(|v| {
                    v.destination
                        .as_mut()
                        .unwrap()
                        .container_entry
                        .as_mut()
                        .unwrap()
                        .parent_item_instance_id = uuid(9);
                }),
            ),
            (
                "zero command",
                Box::new(|v| {
                    v.source
                        .as_mut()
                        .unwrap()
                        .command_ref
                        .as_mut()
                        .unwrap()
                        .command_id = 0;
                }),
            ),
            (
                "command of another session",
                Box::new(|v| {
                    v.source
                        .as_mut()
                        .unwrap()
                        .command_ref
                        .as_mut()
                        .unwrap()
                        .game_session_id = uuid(51);
                }),
            ),
            (
                "zero connection generation",
                Box::new(|v| v.destination.as_mut().unwrap().expected_session_generation = 0),
            ),
            (
                "no claim",
                Box::new(|v| v.source.as_mut().unwrap().reward_claim = None),
            ),
            (
                "oversize claim key",
                Box::new(|v| {
                    v.source
                        .as_mut()
                        .unwrap()
                        .reward_claim
                        .as_mut()
                        .unwrap()
                        .production_key = "k".repeat(RL07_CONTENT_KEY_BYTES_MAX + 1);
                }),
            ),
        ];
        for (label, mutate) in cases {
            let mut value = minted();
            mutate(&mut value);
            assert!(check_reward_claim_mint(&value).is_err(), "{label}");
            assert!(
                encode_reward_claim_mint_event(identity(), value).is_err(),
                "{label}"
            );
        }
        assert!(check_reward_claim_mint(&minted()).is_ok());
    }

    #[test]
    fn usage_fits_the_registered_rows() {
        assert!(REWARD_CLAIM_MINT_USAGE.check().is_ok());
    }
}
