//! SUPPLY-STASH-0 §7: one economy-ledger event per successful conversion.
use super::super::item_mint_audit::{
    self as envelope, CausationRefV1, CausationV1, EnvelopeCommandRefV1, EventEnvelopeV1,
    TransactionEventRefV1,
};
use super::*;
use prost::Message;
use sha2::{Digest, Sha256};

const PAYLOAD_BYTES_MAX:usize=34_000;
const ENVELOPE_BYTES_MAX:usize=36_000;

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
    #[prost(string, optional, tag = "6")]
    house_key: Option<String>,
    #[prost(uint64, optional, tag = "7")]
    house_scope_ownership_generation: Option<u64>,
}

fn check_house_scope(operation:&StashOperationV1)->Result<(),DurabilityError> {
    match (operation.house_key.as_deref(),operation.house_scope_ownership_generation) {
        (None,None)=>Ok(()),
        (Some(house),Some(generation)) if generation>0 && operation.value.as_ref()
            .is_some_and(|value|super::super::house_hireling::audit::valid(&value.world,house))=>Ok(()),
        _=>Err(DurabilityError::InvalidStoredState),
    }
}

fn decode_operation(payload:&[u8])->Result<StashOperationV1,DurabilityError> {
    if payload.len()>PAYLOAD_BYTES_MAX {return Err(DurabilityError::InvalidStoredState)}
    let operation=StashOperationV1::decode(payload).map_err(|_|DurabilityError::InvalidStoredState)?;
    if operation.encode_to_vec()!=payload {return Err(DurabilityError::InvalidStoredState)}
    check_house_scope(&operation)?;Ok(operation)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn encode(
    fence: CurrentCharacterItemFence,
    channel: crate::foundation::ChannelId,
    request: &StashRequest,
    plan: &Plan,
    transaction: &str,
    event: &str,
    occurred: i64,
    house_key:Option<&str>,
) -> Result<Vec<u8>, DurabilityError> {
    let world = fence.runtime_scope.world_id();
    match (fence.runtime_scope,house_key) {
        (crate::foundation::RuntimeScopeRefV1::Channel{..},None)=>{},
        (crate::foundation::RuntimeScopeRefV1::Instance{instance_id,..},Some(house))
            if super::super::house_scope_handoff::HouseId::new(world,house).is_some_and(|id|id.instance_id()==instance_id)=>{},
        _=>return Err(DurabilityError::InvalidStoredState),
    }
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
        house_key:house_key.map(str::to_owned),
        house_scope_ownership_generation:house_key.map(|_|fence.scope_ownership_generation.get()),
    };
    encode_operation(
        fence,
        request.command,
        channel,
        operation,
        transaction,
        event,
        occurred,
        1,
    )
}

/// Schema2 adds the Task Board sink while the legacy stow/withdraw writer keeps
/// schema1 byte-for-byte. This debit creates no replacement ItemInstances.
#[allow(clippy::too_many_arguments)]
pub(crate) fn encode_task_board_consumption(
    fence: CurrentCharacterItemFence,
    command: CommandRef,
    intent: &[u8],
    definition: &TypedDefinitionRef,
    entry: &str,
    amount: u32,
    before: i64,
    transaction: &str,
    event: &str,
    occurred: i64,
) -> Result<Vec<u8>, DurabilityError> {
    let (world, _) = scope_of(&fence).map_err(|_| DurabilityError::InvalidStoredState)?;
    let after = before
        .checked_sub(i64::from(amount))
        .filter(|value| *value >= 0)
        .ok_or(DurabilityError::InvalidStoredState)?;
    if amount == 0 || intent.len() > oteryn_protocol_oteryn::task_board::full::MAX_REQUEST_BYTES {
        return Err(DurabilityError::InvalidStoredState);
    }
    let operation = StashOperationV1 {
        interpretation_revision: 2,
        conversion_kind: 3,
        intent: intent.to_vec(),
        items: Vec::new(),
        value: Some(StashValueLineV1 {
            entry: uuid_text(entry)?.to_vec(),
            asset: format!(
                "stash:{}@{}",
                definition.production_key, definition.revision_ref
            ),
            character: fence.character_id.as_bytes().to_vec(),
            world: world.as_bytes().to_vec(),
            kind: 3,
            class: 2,
            amount,
            quantity_before: u64::try_from(before)
                .map_err(|_| DurabilityError::InvalidStoredState)?,
            quantity_after: u64::try_from(after)
                .map_err(|_| DurabilityError::InvalidStoredState)?,
        }),
        house_key: None,
        house_scope_ownership_generation: None,
    };
    let (_, channel) = scope_of(&fence).map_err(|_| DurabilityError::InvalidStoredState)?;
    encode_operation(fence, command, channel, operation, transaction, event, occurred, 2)
}

#[allow(clippy::too_many_arguments)]
fn encode_operation(
    fence: CurrentCharacterItemFence,
    command: CommandRef,
    channel: crate::foundation::ChannelId,
    operation: StashOperationV1,
    transaction: &str,
    event: &str,
    occurred: i64,
    schema_revision: u32,
) -> Result<Vec<u8>, DurabilityError> {
    let world = fence.runtime_scope.world_id();
    check_house_scope(&operation)?;
    let payload = envelope::encode_bounded(&operation, PAYLOAD_BYTES_MAX)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let value = EventEnvelopeV1 {
        envelope_revision: 1,
        event_id: uuid_text(event)?.to_vec(),
        event_type_id: 9,
        event_schema_revision: schema_revision,
        durability_class: envelope::DURABLE_AUDIT,
        privacy_class: envelope::RESTRICTED_PLAYER_LINKED,
        retention_profile_id: "ECONOMY_LEDGER_RETENTION_V1".into(),
        occurred_at_unix_ms: occurred,
        world_id: Some(world.as_bytes().to_vec()),
        channel_id: Some(channel.as_bytes().to_vec()),
        game_session_id: Some(fence.game_session_id.as_bytes().to_vec()),
        connection_generation: Some(fence.connection_generation.get()),
        command_id: Some(command.command_id().get()),
        transaction_event: Some(TransactionEventRefV1 {
            transaction_id: uuid_text(transaction)?.to_vec(),
            ordinal: 1,
            count: 1,
        }),
        causation: Some(CausationRefV1 {
            cause: Some(CausationV1::Command(EnvelopeCommandRefV1 {
                game_session_id: fence.game_session_id.as_bytes().to_vec(),
                command_id: command.command_id().get(),
            })),
        }),
        server_build_id: super::super::character_authority::SERVER_BUILD_ID.into(),
        payload_sha256: Sha256::digest(&payload).to_vec(),
        payload,
        ..Default::default()
    };
    let wire = envelope::encode_bounded(&value, ENVELOPE_BYTES_MAX)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let decoded = EventEnvelopeV1::decode(wire.as_slice())
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    if decoded != value
        || decode_operation(value.payload.as_slice())? != operation
    {
        return Err(DurabilityError::InvalidStoredState);
    }
    Ok(wire)
}

#[cfg(test)]
#[path="audit/tests.rs"]
mod tests;
