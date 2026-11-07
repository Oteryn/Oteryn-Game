//! Closed Forge audit proof. Full snapshots stay in the immutable operation row;
//! their hashes bind a bounded event even when cores occupy many single-unit stacks.
use super::*;
use crate::durability::item_mint_audit::{
    self as mint, AuditError, CausationRefV1, CausationV1, EnvelopeCommandRefV1, EventEnvelopeV1,
    OneItemOperationV1, OneItemTransactionV1, TransactionEventRefV1, TransactionResourceUsage,
    Type2EventTuple,
};
use prost::Message;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

// 85 is the pinned largest transfer-core cost; each core may occupy its own stack.
// Two operands, existing 20 fee inputs and two fee-change outputs bound this shape.
pub(crate) const FORGE_INPUTS_MAX: u64 = 85 + 2;
pub(crate) const PROOF_BYTES_MAX: usize = 4_096;
pub(crate) const FEE_PAYLOAD_BYTES_MAX: usize =
    crate::durability::item_fee_burn_audit::FEE_RL07_PAYLOAD_BYTES_MAX + PROOF_BYTES_MAX;
pub(crate) const FEE_ENVELOPE_BYTES_MAX: usize =
    crate::durability::item_fee_burn_audit::FEE_RL07_ENVELOPE_BYTES_MAX + PROOF_BYTES_MAX;

#[derive(Clone, PartialEq, Eq, Message)]
pub struct OneItemForgeV1 {
    #[prost(bytes = "vec", tag = "1")]
    pub proof: Vec<u8>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Proof {
    pub occurrence: Vec<u8>,
    pub character: Vec<u8>,
    pub world: Vec<u8>,
    pub operation: u8,
    pub content: Vec<u8>,
    pub ruleset: String,
    pub simulation_revision: u32,
    pub rolls: [u16; 3],
    pub reply: Value,
    pub inputs_hash: Vec<u8>,
    pub outputs_hash: Vec<u8>,
    pub stash_hash: Vec<u8>,
    pub inputs: u64,
    pub outputs: u64,
    pub retired: u64,
    pub tier_changes: u64,
    pub dust_spent: u32,
    pub gold_spent: u64,
    pub stash_spent: bool,
}

fn closed_object(value: &Value, keys: &[&str]) -> bool {
    value.as_object().is_some_and(|object| {
        object.len() == keys.len() && keys.iter().all(|key| object.contains_key(*key))
    })
}

fn valid_reply(proof: &Proof) -> bool {
    let reply = &proof.reply;
    let result = &reply["result"];
    let pair = proof.operation <= 4;
    let handle = |name: &str| {
        if pair {
            result[name].as_u64().is_some_and(|value| value > 0)
        } else {
            result[name].is_null()
        }
    };
    let bounded = |name: &str, max| result[name].as_u64().is_some_and(|value| value <= max);
    let operation = match proof.operation {
        1 => "fusion",
        2 => "convergence_fusion",
        3 => "transfer",
        4 => "convergence_transfer",
        5 => "dust_to_slivers",
        6 => "slivers_to_cores",
        _ => return false,
    };
    closed_object(
        reply,
        &[
            "operation",
            "succeeded",
            "applied",
            "reason",
            "bonus",
            "result",
        ],
    ) && closed_object(
        result,
        &[
            "first_item",
            "first_tier",
            "second_item",
            "second_tier",
            "dust",
            "dust_limit",
            "gold_spent",
            "cores_spent",
            "slivers_spent",
            "slivers_created",
            "cores_created",
        ],
    ) && reply["operation"].as_str() == Some(operation)
        && reply["succeeded"].is_boolean()
        && reply["applied"] == true
        && reply["reason"].is_null()
        && reply["bonus"].as_u64().is_some_and(|bonus| bonus <= 8)
        && handle("first_item")
        && handle("second_item")
        && (!pair || result["first_item"] != result["second_item"])
        && if pair {
            bounded("first_tier", 10)
                && (result["second_tier"].is_null() || bounded("second_tier", 10))
        } else {
            result["first_tier"].is_null() && result["second_tier"].is_null()
        }
        && result["dust_limit"].as_u64().is_some_and(|limit| {
            (100..=4600).contains(&limit)
                && (limit - 100).is_multiple_of(20)
                && result["dust"].as_u64().is_some_and(|dust| dust <= limit)
        })
        && result["gold_spent"].as_u64() == Some(proof.gold_spent)
        && bounded("cores_spent", 85)
        && bounded("slivers_spent", 50)
        && bounded("slivers_created", 3)
        && bounded("cores_created", 1)
}

pub(crate) fn decode(proof: &[u8]) -> std::result::Result<Proof, AuditError> {
    if proof.len() > PROOF_BYTES_MAX {
        return Err(AuditError::CapacityExceeded);
    }
    let value: Proof = serde_json::from_slice(proof).map_err(|_| AuditError::InvalidInput)?;
    check_uuid_v7(&value.occurrence)?;
    check_uuid_v7(&value.character)?;
    check_uuid_v7(&value.world)?;
    if !(1..=6).contains(&value.operation)
        || value.content.len() != 32
        || value.inputs_hash.len() != 32
        || value.outputs_hash.len() != 32
        || value.stash_hash.len() != 32
        || value.ruleset != storage::RULESET
        || value.simulation_revision != 1
        || value.rolls[0] >= 100
        || value.rolls[1] >= 100
        || value.rolls[2] > 10_000
        || value.inputs > FORGE_INPUTS_MAX
        || value.outputs > 1
        || value.retired > value.inputs
        || value.tier_changes > 2
        || !valid_reply(&value)
        || value.gold_spent > 300_000_000_000
        || !matches!(value.dust_spent, 0 | 60 | 100 | 130 | 160)
    {
        return Err(AuditError::InvalidInput);
    }
    Ok(value)
}

pub(crate) fn check(proof: &OneItemForgeV1) -> std::result::Result<(), AuditError> {
    decode(&proof.proof).map(|_| ())
}

fn decode_envelope(wire: &[u8]) -> std::result::Result<EventEnvelopeV1, AuditError> {
    let envelope = mint::decode_common_envelope(wire)?;
    let payload = OneItemTransactionV1::decode(envelope.payload.as_slice())
        .map_err(|_| AuditError::InvalidInput)?;
    if payload.encode_to_vec() != envelope.payload
        || payload.interpretation_revision != mint::INTERPRETATION_REVISION
    {
        return Err(AuditError::InvalidInput);
    }
    let Some(OneItemOperationV1::Forge(operation)) = payload.operation else {
        return Err(AuditError::InvalidInput);
    };
    let proof = decode(&operation.proof)?;
    let Some(CausationV1::Command(command)) = envelope
        .causation
        .as_ref()
        .and_then(|cause| cause.cause.as_ref())
    else {
        return Err(AuditError::InvalidInput);
    };
    if envelope.world_id.as_ref() != Some(&proof.world)
        || envelope.game_session_id.as_ref() != Some(&command.game_session_id)
        || envelope.command_id != Some(command.command_id)
        || envelope.connection_generation.is_none()
    {
        return Err(AuditError::InvalidInput);
    }
    Ok(envelope)
}

pub(crate) fn usage(proof: &OneItemForgeV1) -> TransactionResourceUsage {
    let value = decode(&proof.proof).ok();
    let inputs = value.as_ref().map_or(FORGE_INPUTS_MAX + 1, |p| p.inputs);
    let outputs = value.as_ref().map_or(2, |p| p.outputs);
    let retired = value.as_ref().map_or(0, |p| p.retired);
    let values = value.as_ref().map_or(0, |p| {
        u64::from(p.dust_spent > 0) + u64::from(p.stash_spent)
    });
    TransactionResourceUsage {
        touched_item_instances: inputs + outputs,
        location_custody_lines: retired + outputs,
        value_lines: values,
        transform_lines: value.as_ref().map_or(0, |p| p.tier_changes),
        container_expansion: 0,
        participants: inputs + outputs + values,
        effect_work_units: 3 * inputs + 2 * outputs + 2 * values,
        events: 0,
    }
}

pub(crate) fn check_combined_usage(
    usage: &TransactionResourceUsage,
) -> std::result::Result<(), AuditError> {
    // Existing fee BURN maximum, plus the bounded Forge operation above.
    if usage.touched_item_instances > FORGE_INPUTS_MAX + 22
        || usage.location_custody_lines > FORGE_INPUTS_MAX + 22
        || usage.value_lines > 3
        || usage.transform_lines > 2
        || usage.container_expansion != 0
        || usage.participants > FORGE_INPUTS_MAX + 22 + 3
        || usage.effect_work_units > 3 * FORGE_INPUTS_MAX + 64 + 6
        || usage.events > 1
    {
        Err(AuditError::CapacityExceeded)
    } else {
        Ok(())
    }
}

pub(super) fn encode(
    fence: CurrentCharacterItemFence,
    command: CommandRef,
    reserved: &storage::Reservation,
    proof: Vec<u8>,
    tuple: Type2EventTuple,
) -> NativeResult<Vec<u8>> {
    let (world, channel) = scope_of(&fence).map_err(|_| DurabilityError::InvalidStoredState)?;
    let operation = OneItemForgeV1 { proof };
    check(&operation).map_err(|_| DurabilityError::InvalidStoredState)?;
    let mut used = usage(&operation);
    used.events = 1;
    check_combined_usage(&used).map_err(|_| DurabilityError::InvalidStoredState)?;
    let payload = mint::encode_bounded(
        &OneItemTransactionV1 {
            interpretation_revision: mint::INTERPRETATION_REVISION,
            operation: Some(OneItemOperationV1::Forge(operation)),
        },
        mint::RL07_PAYLOAD_BYTES_MAX,
    )
    .map_err(|_| DurabilityError::InvalidStoredState)?;
    let envelope = EventEnvelopeV1 {
        envelope_revision: mint::ENVELOPE_REVISION,
        event_id: reserved.event.to_vec(),
        event_type_id: mint::EVENT_TYPE_ID,
        event_schema_revision: tuple.schema_revision(),
        durability_class: mint::DURABLE_AUDIT,
        privacy_class: mint::RESTRICTED_PLAYER_LINKED,
        retention_profile_id: tuple.retention_profile_id().into(),
        occurred_at_unix_ms: reserved.occurred_at,
        world_id: Some(world.as_bytes().to_vec()),
        channel_id: Some(channel.as_bytes().to_vec()),
        game_session_id: Some(fence.game_session_id.as_bytes().to_vec()),
        connection_generation: Some(fence.connection_generation.get()),
        command_id: Some(command.command_id().get()),
        transaction_event: Some(TransactionEventRefV1 {
            transaction_id: reserved.transaction.to_vec(),
            ordinal: 1,
            count: 1,
        }),
        causation: Some(CausationRefV1 {
            cause: Some(CausationV1::Command(EnvelopeCommandRefV1 {
                game_session_id: fence.game_session_id.as_bytes().to_vec(),
                command_id: command.command_id().get(),
            })),
        }),
        server_build_id: SERVER_BUILD_ID.into(),
        payload_sha256: Sha256::digest(&payload).to_vec(),
        payload,
        ..EventEnvelopeV1::default()
    };
    let wire = mint::encode_bounded(&envelope, mint::RL07_ENVELOPE_BYTES_MAX)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    if decode_envelope(&wire).map_err(|_| DurabilityError::InvalidStoredState)? != envelope {
        return Err(DurabilityError::InvalidStoredState);
    }
    Ok(wire)
}
