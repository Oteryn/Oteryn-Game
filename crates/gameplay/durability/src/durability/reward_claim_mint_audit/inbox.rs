//! Additive Inbox destination of the existing `reward_claim_mint` operation.
//! This proposal preserves operation tag 4 and every CHEST field/byte. Its
//! source-specific count and byte rows must be registered in the protected slot
//! before production activation; common one-item limits are not widened.

use super::{OneItemRewardClaimMintV1, REWARD_CLAIM_MINT_TYPED_CAUSE};
use crate::durability::item_mint_audit::{
    AuditError, ITEM_LIFECYCLE_LIVE, OneItemStateV1, TransactionResourceUsage, check_content_key,
    check_definition, check_key_number, check_technical_text, check_uuid_v7,
};
use crate::durability::item_transfer_audit::GAMEITEM01_STACK_QUANTITY_MAX;
use prost::Message;

/// DAILY-REWARD-0 section 4.1 source output bound; the proposed Inbox resource
/// rows below do not impose a capacity on the CharacterInbox itself.
pub const REWARD_INBOX_ITEMS_MAX: usize = 20;
/// Proposed DUR03-RL-07-REWARD-INBOX-PAYLOAD-BYTES. At most twenty bounded
/// states plus one source/header fit below this conservative fixed ceiling.
pub const REWARD_INBOX_PAYLOAD_BYTES_MAX: usize = 32_768;
/// Proposed DUR03-RL-07-REWARD-INBOX-ENVELOPE-BYTES; includes the bounded
/// normative ANL-01 envelope as well as the payload.
pub const REWARD_INBOX_ENVELOPE_BYTES_MAX: usize = 34_816;

/// An actual permanent Inbox position, in the source's immutable output order.
#[derive(Clone, PartialEq, Eq, Message)]
pub struct RewardClaimInboxMintOutputV1 {
    #[prost(message, optional, tag = "1")]
    pub after: Option<OneItemStateV1>,
    #[prost(uint64, tag = "2")]
    pub ordinal: u64,
    #[prost(uint32, tag = "3")]
    pub source_line: u32,
}

/// Field 5 of `OneItemRewardClaimMintV1`, not another operation or Cause.
#[derive(Clone, PartialEq, Eq, Message)]
pub struct RewardClaimInboxMintV1 {
    #[prost(bytes = "vec", tag = "1")]
    pub character_id: Vec<u8>,
    #[prost(bytes = "vec", tag = "2")]
    pub world_id: Vec<u8>,
    #[prost(uint64, tag = "3")]
    pub expected_session_generation: u64,
    #[prost(bytes = "vec", tag = "4")]
    pub source_binding: Vec<u8>,
    #[prost(message, repeated, tag = "5")]
    pub outputs: Vec<RewardClaimInboxMintOutputV1>,
    #[prost(bytes = "vec", tag = "6")]
    pub plan_binding: Vec<u8>,
    #[prost(bytes = "vec", tag = "7")]
    pub mint_intent_sha256: Vec<u8>,
    #[prost(uint64, tag = "8")]
    pub expected_character_lease_generation: u64,
    /// Optional closed House authority; absent on the existing Channel path.
    #[prost(string, optional, tag = "9")]
    pub house_key: Option<String>,
    #[prost(uint64, optional, tag = "10")]
    pub house_scope_ownership_generation: Option<u64>,
    /// 1 live House session, 2 current House recall with historical CommandRef provenance.
    #[prost(uint32, optional, tag = "11")]
    pub house_authority_kind: Option<u32>,
}

/// The bridge's own resource use: one participant and placement per fresh item,
/// one reward receipt and one aggregate event. The source owner must separately
/// account for its other effects when composing the complete claim transaction.
pub fn reward_inbox_usage(count: usize) -> Result<TransactionResourceUsage, AuditError> {
    if count == 0 {
        return Err(AuditError::InvalidInput);
    }
    if count > REWARD_INBOX_ITEMS_MAX {
        return Err(AuditError::CapacityExceeded);
    }
    let count = count as u64;
    Ok(TransactionResourceUsage {
        touched_item_instances: count,
        location_custody_lines: count,
        value_lines: 0,
        transform_lines: 0,
        container_expansion: 0,
        participants: count,
        effect_work_units: 2 * count + 1,
        events: 1,
    })
}

pub(super) fn check_inbox(value: &OneItemRewardClaimMintV1) -> Result<(), AuditError> {
    let inbox = value.inbox.as_ref().ok_or(AuditError::InvalidInput)?;
    let source = value.source.as_ref().ok_or(AuditError::InvalidInput)?;
    let command = source
        .command_ref
        .as_ref()
        .ok_or(AuditError::InvalidInput)?;
    reward_inbox_usage(inbox.outputs.len())?;
    check_uuid_v7(&inbox.character_id)?;
    check_uuid_v7(&inbox.world_id)?;
    check_uuid_v7(&command.game_session_id)?;
    check_technical_text(&source.typed_cause)?;
    check_content_key(&source.content_revision)?;
    check_content_key(&source.ruleset_revision)?;
    check_content_key(&source.sim_revision)?;
    check_definition(source.reward_claim.as_ref())?;
    match (&inbox.house_key, inbox.house_scope_ownership_generation, inbox.house_authority_kind) {
        (None, None, None) => {}
        (Some(house), Some(generation), Some(1 | 2)) if generation > 0 => {
            let world = crate::foundation::WorldId::decode(&inbox.world_id)
                .map_err(|_| AuditError::InvalidInput)?;
            if crate::durability::house_scope_handoff::HouseId::new(world, house).is_none() {
                return Err(AuditError::InvalidInput);
            }
            if inbox.house_authority_kind == Some(2)
                && (inbox.outputs.len() != 1 || inbox.outputs[0].after.as_ref().is_none_or(|item|
                    item.quantity != 1 || item.key_number != 0 || item.definition != source.reward_claim || item.definition.as_ref().is_none_or(|definition|
                        definition.family != "Item" || definition.production_key != crate::durability::house_hireling::HIRELING_LAMP_KEY))) {
                return Err(AuditError::InvalidInput);
            }
        }
        _ => return Err(AuditError::InvalidInput),
    }
    if value.after.is_some()
        || value.destination.is_some()
        || !value.before_semantically_absent
        || source.boss_reward.is_some()
        || source.typed_cause != REWARD_CLAIM_MINT_TYPED_CAUSE
        || command.command_id == 0
        || inbox.expected_session_generation == 0
        || inbox.expected_character_lease_generation == 0
        || inbox.source_binding.len() != 32
        || inbox.plan_binding.len() != 32
        || inbox.mint_intent_sha256.len() != 32
    {
        return Err(AuditError::InvalidInput);
    }
    for (index, output) in inbox.outputs.iter().enumerate() {
        let after = output.after.as_ref().ok_or(AuditError::InvalidInput)?;
        check_uuid_v7(&after.item_instance_id)?;
        check_uuid_v7(&after.world_id)?;
        check_definition(after.definition.as_ref())?;
        check_key_number(after)?;
        if after.world_id != inbox.world_id
            || after.lifecycle != ITEM_LIFECYCLE_LIVE
            || after.quantity == 0
            || after.quantity > GAMEITEM01_STACK_QUANTITY_MAX
            || output.ordinal == 0
            || (index > 0 && inbox.outputs[index - 1].ordinal >= output.ordinal)
            || inbox.outputs[..index].iter().any(|prior| {
                prior
                    .after
                    .as_ref()
                    .is_some_and(|state| state.item_instance_id == after.item_instance_id)
            })
        {
            return Err(AuditError::InvalidInput);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
