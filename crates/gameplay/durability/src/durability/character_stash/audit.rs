//! SUPPLY-STASH-0 §7: one economy-ledger event per successful conversion.
use super::super::item_mint_audit::{
    self as envelope, CausationRefV1, CausationV1, EnvelopeCommandRefV1, EventEnvelopeV1,
    TransactionEventRefV1,
};
use super::*;
use prost::Message;
use sha2::{Digest, Sha256};

#[derive(Clone, PartialEq, Message)]
struct StashItemLineV1 {
    #[prost(bytes = "vec", tag = "1")]
    item: Vec<u8>,
    #[prost(string, tag = "2")]
    definition_key: String,
    #[prost(string, tag = "3")]
    definition_revision: String,
    #[prost(uint32, tag = "4")]
    quantity: u32,
    #[prost(uint32, tag = "5")]
    direction: u32,
    #[prost(uint32, tag = "6")]
    class: u32,
}
#[derive(Clone, PartialEq, Message)]
struct StashValueLineV1 {
    #[prost(bytes = "vec", tag = "1")]
    entry: Vec<u8>,
    #[prost(string, tag = "2")]
    asset: String,
    #[prost(bytes = "vec", tag = "3")]
    character: Vec<u8>,
    #[prost(bytes = "vec", tag = "4")]
    world: Vec<u8>,
    #[prost(uint32, tag = "5")]
    kind: u32,
    #[prost(uint32, tag = "6")]
    class: u32,
    #[prost(uint32, tag = "7")]
    amount: u32,
    #[prost(uint64, tag = "8")]
    quantity_before: u64,
    #[prost(uint64, tag = "9")]
    quantity_after: u64,
}
#[derive(Clone, PartialEq, Message)]
struct StashOperationV1 {
    #[prost(uint32, tag = "1")]
    interpretation_revision: u32,
    #[prost(uint32, tag = "2")]
    conversion_kind: u32,
    #[prost(bytes = "vec", tag = "3")]
    intent: Vec<u8>,
    #[prost(message, repeated, tag = "4")]
    items: Vec<StashItemLineV1>,
    #[prost(message, optional, tag = "5")]
    value: Option<StashValueLineV1>,
}

#[allow(clippy::too_many_arguments)]
pub(super) fn encode(
    fence: CurrentCharacterItemFence,
    request: &StashRequest,
    plan: &Plan,
    transaction: &str,
    event: &str,
    occurred: i64,
) -> Result<Vec<u8>, DurabilityError> {
    let (world, channel) = scope_of(&fence).map_err(|_| DurabilityError::InvalidStoredState)?;
    let stow = matches!(request.intent.action, StashAction::Stow(_));
    let items = plan
        .lines
        .iter()
        .map(|line| {
            Ok(StashItemLineV1 {
                item: uuid_text(
                    line["item"]
                        .as_str()
                        .ok_or(DurabilityError::InvalidStoredState)?,
                )?
                .to_vec(),
                definition_key: plan.definition.production_key.clone(),
                definition_revision: plan.definition.revision_ref.clone(),
                quantity: u32::try_from(
                    line["count"]
                        .as_u64()
                        .ok_or(DurabilityError::InvalidStoredState)?,
                )
                .map_err(|_| DurabilityError::InvalidStoredState)?,
                direction: if stow { 1 } else { 2 },
                class: 1,
            })
        })
        .collect::<Result<Vec<_>, DurabilityError>>()?;
    let value = StashValueLineV1 {
        entry: uuid_text(&plan.entry)?.to_vec(),
        asset: format!(
            "stash:{}@{}",
            plan.definition.production_key, plan.definition.revision_ref
        ),
        character: fence.character_id.as_bytes().to_vec(),
        world: world.as_bytes().to_vec(),
        kind: if stow { 1 } else { 2 },
        class: 1,
        amount: plan.amount,
        quantity_before: plan.before as u64,
        quantity_after: (plan.before
            + if stow {
                i64::from(plan.amount)
            } else {
                -i64::from(plan.amount)
            }) as u64,
    };
    let operation = StashOperationV1 {
        interpretation_revision: 1,
        conversion_kind: if stow { 1 } else { 2 },
        intent: encode_stash_intent(request.intent)
            .map_err(|_| DurabilityError::InvalidStoredState)?,
        items,
        value: Some(value),
    };
    let payload = envelope::encode_bounded(&operation, 34_000)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let value = EventEnvelopeV1 {
        envelope_revision: 1,
        event_id: uuid_text(event)?.to_vec(),
        event_type_id: 9,
        event_schema_revision: 1,
        durability_class: envelope::DURABLE_AUDIT,
        privacy_class: envelope::RESTRICTED_PLAYER_LINKED,
        retention_profile_id: "ECONOMY_LEDGER_RETENTION_V1".into(),
        occurred_at_unix_ms: occurred,
        world_id: Some(world.as_bytes().to_vec()),
        channel_id: Some(channel.as_bytes().to_vec()),
        game_session_id: Some(fence.game_session_id.as_bytes().to_vec()),
        connection_generation: Some(fence.connection_generation.get()),
        command_id: Some(request.command.command_id().get()),
        transaction_event: Some(TransactionEventRefV1 {
            transaction_id: uuid_text(transaction)?.to_vec(),
            ordinal: 1,
            count: 1,
        }),
        causation: Some(CausationRefV1 {
            cause: Some(CausationV1::Command(EnvelopeCommandRefV1 {
                game_session_id: fence.game_session_id.as_bytes().to_vec(),
                command_id: request.command.command_id().get(),
            })),
        }),
        server_build_id: super::super::character_authority::SERVER_BUILD_ID.into(),
        payload_sha256: Sha256::digest(&payload).to_vec(),
        payload,
        ..Default::default()
    };
    let wire = envelope::encode_bounded(&value, 36_000)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let decoded = EventEnvelopeV1::decode(wire.as_slice())
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    if decoded != value
        || StashOperationV1::decode(value.payload.as_slice())
            .map_err(|_| DurabilityError::InvalidStoredState)?
            != operation
    {
        return Err(DurabilityError::InvalidStoredState);
    }
    Ok(wire)
}
