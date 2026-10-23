//! Additive native type-2 operation tag 14, after ForgeDustLimit 13 on main.
//! The canonical JSON binding carries both complete immutable before observations,
//! both accepts/fences and the full Node/Content binding. It is never live authority.
use super::super::item_mint_audit::{
    self as mint, AuditError, CausationRefV1, CausationV1, EnvelopeCommandRefV1, EventEnvelopeV1,
    OneItemOperationV1, OneItemTransactionV1, TransactionEventRefV1, Type2EventTuple,
};
use super::super::item_transfer_audit::OneItemContainerEntryV1;
use super::*;
use prost::Message;
use sha2::{Digest, Sha256};

/// Fixed PLAYER-TRADE0 §6 effect shape; inventory reads are separately bounded by BAGS0.
pub const PLAYER_TRADE_RESOURCES: mint::TransactionResourceUsage = mint::TransactionResourceUsage {
    touched_item_instances: 2,
    location_custody_lines: 4,
    value_lines: 0,
    transform_lines: 0,
    container_expansion: 0,
    participants: 2,
    effect_work_units: 6,
    events: 1,
};

#[derive(Clone, PartialEq, Eq, Message)]
pub struct OneItemPlayerTradeV1 {
    #[prost(bytes = "vec", tag = "1")]
    pub request_binding: Vec<u8>,
    /// Two new entries, in source-line order, each in the opposite participant's backpack.
    #[prost(message, repeated, tag = "2")]
    pub destinations: Vec<OneItemContainerEntryV1>,
}

pub(super) fn encode(
    request: &PlayerTradeRequest,
    intent: &[u8],
    destinations: [(u64, u32); 2],
    tuple: Type2EventTuple,
) -> Result<Vec<u8>, AuditError> {
    let locations = request
        .participants
        .iter()
        .enumerate()
        .map(|(index, _)| {
            Ok(OneItemContainerEntryV1 {
                parent_item_instance_id: snapshot::uuid(
                    &request.participants[1 - index].leaf.entry,
                    "parent_item_instance_id",
                )
                .map_err(|_| AuditError::InvalidInput)?
                .to_vec(),
                placement_ordinal: destinations[index].0,
            })
        })
        .collect::<Result<Vec<_>, AuditError>>()?;
    let payload = mint::encode_bounded(
        &OneItemTransactionV1 {
            interpretation_revision: mint::INTERPRETATION_REVISION,
            operation: Some(OneItemOperationV1::PlayerTrade(OneItemPlayerTradeV1 {
                request_binding: intent.to_vec(),
                destinations: locations,
            })),
        },
        mint::RL07_PAYLOAD_BYTES_MAX,
    )?;
    let fence = &request.participants[1].accepted;
    let command = request.participants[1].command;
    let (world, channel) = scope_of(fence).map_err(|_| AuditError::InvalidInput)?;
    let envelope = EventEnvelopeV1 {
        envelope_revision: mint::ENVELOPE_REVISION,
        event_id: request.event_id.to_vec(),
        event_type_id: mint::EVENT_TYPE_ID,
        event_schema_revision: tuple.schema_revision(),
        durability_class: mint::DURABLE_AUDIT,
        privacy_class: mint::RESTRICTED_PLAYER_LINKED,
        retention_profile_id: tuple.retention_profile_id().into(),
        occurred_at_unix_ms: request.occurred_at_unix_ms,
        world_id: Some(world.as_bytes().to_vec()),
        channel_id: Some(channel.as_bytes().to_vec()),
        game_session_id: Some(command.game_session_id().as_bytes().to_vec()),
        connection_generation: Some(fence.connection_generation.get()),
        command_id: Some(command.command_id().get()),
        transaction_event: Some(TransactionEventRefV1 {
            transaction_id: request.transaction_id.to_vec(),
            ordinal: 1,
            count: 1,
        }),
        causation: Some(CausationRefV1 {
            cause: Some(CausationV1::Command(EnvelopeCommandRefV1 {
                game_session_id: command.game_session_id().as_bytes().to_vec(),
                command_id: command.command_id().get(),
            })),
        }),
        server_build_id: super::super::character_authority::SERVER_BUILD_ID.into(),
        payload_sha256: Sha256::digest(&payload).to_vec(),
        payload,
        ..EventEnvelopeV1::default()
    };
    let wire = mint::encode_bounded(&envelope, mint::RL07_ENVELOPE_BYTES_MAX)?;
    decode(&wire)?;
    Ok(wire)
}

pub(super) fn decode(wire: &[u8]) -> Result<(EventEnvelopeV1, OneItemPlayerTradeV1), AuditError> {
    let envelope = mint::decode_common_envelope(wire)?;
    let payload = OneItemTransactionV1::decode(envelope.payload.as_slice())
        .map_err(|_| AuditError::InvalidInput)?;
    if payload.interpretation_revision != mint::INTERPRETATION_REVISION
        || payload.encode_to_vec() != envelope.payload
    {
        return Err(AuditError::InvalidInput);
    }
    let Some(OneItemOperationV1::PlayerTrade(value)) = payload.operation else {
        return Err(AuditError::InvalidInput);
    };
    let binding: Value =
        serde_json::from_slice(&value.request_binding).map_err(|_| AuditError::InvalidInput)?;
    if serde_json::to_vec(&binding).map_err(|_| AuditError::InvalidInput)? != value.request_binding
        || binding["version"] != 1
        || value.destinations.len() as u64 != PLAYER_TRADE_RESOURCES.touched_item_instances
    {
        return Err(AuditError::InvalidInput);
    }
    for key in ["occurrence", "event", "transaction", "node"] {
        let id: Vec<u8> =
            serde_json::from_value(binding[key].clone()).map_err(|_| AuditError::InvalidInput)?;
        mint::check_uuid_v7(&id)?;
    }
    for key in ["content_revision", "ruleset_revision", "sim_revision"] {
        mint::check_technical_text(binding[key].as_str().ok_or(AuditError::InvalidInput)?)?;
    }
    for key in ["content", "weights"] {
        let digest: Vec<u8> =
            serde_json::from_value(binding[key].clone()).map_err(|_| AuditError::InvalidInput)?;
        if digest.len() != 32 || digest.iter().all(|b| *b == 0) {
            return Err(AuditError::InvalidInput);
        }
    }
    if binding["node_revision"].as_u64().is_none_or(|v| v == 0) {
        return Err(AuditError::InvalidInput);
    }
    let parties = binding["parties"]
        .as_array()
        .filter(|p| p.len() as u64 == PLAYER_TRADE_RESOURCES.participants)
        .ok_or(AuditError::InvalidInput)?;
    let bytes = |v: &Value| -> Result<Vec<u8>, AuditError> {
        serde_json::from_value(v.clone()).map_err(|_| AuditError::InvalidInput)
    };
    for index in [0, 1] {
        let party = &parties[index];
        let other = &parties[1 - index];
        let leaf: TradeLeaf =
            serde_json::from_value(party["leaf"].clone()).map_err(|_| AuditError::InvalidInput)?;
        for key in ["character", "session", "world", "channel"] {
            mint::check_uuid_v7(&bytes(&party["fence"][key])?)?;
        }
        for key in ["connection", "lease", "scope_generation"] {
            if party["fence"][key].as_u64().is_none_or(|v| v == 0) {
                return Err(AuditError::InvalidInput);
            }
        }
        if party["command"].as_u64().is_none_or(|v| v == 0) {
            return Err(AuditError::InvalidInput);
        }
        let definition = snapshot::definition(&leaf.item).map_err(|_| AuditError::InvalidInput)?;
        mint::check_definition(Some(&mint::OneItemTypedDefinitionRevisionV1 {
            family: definition.family,
            production_key: definition.production_key,
            revision_ref: definition.revision_ref,
        }))?;
        if let Some(key) = &leaf.key
            && (!(1..=65535).contains(
                &snapshot::number(key, "key_number").map_err(|_| AuditError::InvalidInput)?,
            ) || snapshot::uuid(key, "item_instance_id")
                .map_err(|_| AuditError::InvalidInput)?
                != snapshot::uuid(&leaf.item, "item_instance_id")
                    .map_err(|_| AuditError::InvalidInput)?
                || key["minted_transaction_id"] != leaf.item["minted_transaction_id"])
        {
            return Err(AuditError::InvalidInput);
        }
        let source = snapshot::uuid(&leaf.entry, "parent_item_instance_id")
            .map_err(|_| AuditError::InvalidInput)?;
        let item =
            snapshot::uuid(&leaf.item, "item_instance_id").map_err(|_| AuditError::InvalidInput)?;
        mint::check_uuid_v7(&item)?;
        mint::check_uuid_v7(&source)?;
        if snapshot::number(&leaf.item, "quantity").map_err(|_| AuditError::InvalidInput)?
            > u64::from(oteryn_gameplay_economy::player_trade::LEAF_QUANTITY_MAX)
            || leaf.item["lifecycle"] != 1
            || snapshot::number(&leaf.item, "quantity").map_err(|_| AuditError::InvalidInput)? == 0
            || party["fence"]["character"] == other["fence"]["character"]
            || party["fence"]["session"] == other["fence"]["session"]
            || party["fence"]["world"] != other["fence"]["world"]
            || party["fence"]["channel"] != other["fence"]["channel"]
            || value.destinations[index].placement_ordinal == 0
            || value.destinations[index].placement_ordinal
                <= snapshot::number(&other["leaf"]["entry"], "placement_ordinal")
                    .map_err(|_| AuditError::InvalidInput)?
            || snapshot::uuid(&leaf.entry, "item_instance_id")
                .map_err(|_| AuditError::InvalidInput)?
                != item
            || snapshot::uuid(&leaf.item, "world_id")
                .map_err(|_| AuditError::InvalidInput)?
                .as_slice()
                != bytes(&party["fence"]["world"])?
            || snapshot::uuid(&leaf.entry, "character_id")
                .map_err(|_| AuditError::InvalidInput)?
                .as_slice()
                != bytes(&party["fence"]["character"])?
        {
            return Err(AuditError::InvalidInput);
        }
        let other_leaf: TradeLeaf =
            serde_json::from_value(other["leaf"].clone()).map_err(|_| AuditError::InvalidInput)?;
        if value.destinations[index].parent_item_instance_id
            != snapshot::uuid(&other_leaf.entry, "parent_item_instance_id")
                .map_err(|_| AuditError::InvalidInput)?
            || item
                == snapshot::uuid(&other_leaf.item, "item_instance_id")
                    .map_err(|_| AuditError::InvalidInput)?
        {
            return Err(AuditError::InvalidInput);
        }
    }
    let pair_session = bytes(&binding["pair"]["session"])?;
    mint::check_uuid_v7(&pair_session)?;
    let pair_command = binding["pair"]["command"]
        .as_u64()
        .filter(|n| *n > 0)
        .ok_or(AuditError::InvalidInput)?;
    if !parties.iter().any(|p| {
        bytes(&p["fence"]["session"]).is_ok_and(|s| s == pair_session)
            && p["command"].as_u64().is_some_and(|n| n > pair_command)
    }) {
        return Err(AuditError::InvalidInput);
    }
    let trigger = &parties[1];
    let transaction = envelope
        .transaction_event
        .as_ref()
        .ok_or(AuditError::InvalidInput)?;
    if bytes(&binding["event"])? != envelope.event_id
        || bytes(&binding["transaction"])? != transaction.transaction_id
        || bytes(&trigger["fence"]["world"])?
            != envelope.world_id.clone().ok_or(AuditError::InvalidInput)?
        || bytes(&trigger["fence"]["channel"])?
            != envelope
                .channel_id
                .clone()
                .ok_or(AuditError::InvalidInput)?
        || bytes(&trigger["fence"]["session"])?
            != envelope
                .game_session_id
                .clone()
                .ok_or(AuditError::InvalidInput)?
        || trigger["command"].as_u64() != envelope.command_id
        || trigger["fence"]["connection"].as_u64() != envelope.connection_generation
        || binding["time"].as_i64() != Some(envelope.occurred_at_unix_ms)
        || transaction.ordinal != 1
        || transaction.count != 1
    {
        return Err(AuditError::InvalidInput);
    }
    if envelope.causation.as_ref().and_then(|c| c.cause.as_ref())
        != Some(&CausationV1::Command(EnvelopeCommandRefV1 {
            game_session_id: bytes(&trigger["fence"]["session"])?,
            command_id: trigger["command"]
                .as_u64()
                .ok_or(AuditError::InvalidInput)?,
        }))
    {
        return Err(AuditError::InvalidInput);
    }
    Ok((envelope, value))
}

#[cfg(test)]
#[path = "player_trade_audit_tests.rs"]
mod player_trade_audit_tests;
